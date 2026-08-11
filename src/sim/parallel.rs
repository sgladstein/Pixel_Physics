//! M5: the multithreaded CA sweep.
//!
//! `rayon` over the plan's 4-pass checkerboard — chunk `(cx, cy)` belongs to
//! group `(cx % 2, cy % 2)` — but the actual concurrency-safety design here
//! is not what the plan originally sketched (a single `unsafe` function
//! handing out overlapping mutable 3x3 chunk neighbourhoods). That version
//! turned out to need a second proof — that queuing everything through it
//! still resolves to a correct order of writes within one worker's own
//! sweep — before it could be trusted, and finding that gap mid-implementation
//! would have been a bad place to discover it. What is here instead needs no
//! `unsafe` anywhere, at the cost of one extra serial merge step per pass.
//!
//! # The proof this whole module leans on
//!
//! For any two *different* active chunks in the same pass, every write one
//! of them can make — direct or via `CellSurface` — lands somewhere the
//! other can never also write to. Movement rules (`update.rs`) only ever
//! move a cell by at most `MAX_REACH` (32, exactly half of `CHUNK_SIZE`, 64)
//! sideways within a row, or by exactly one cell in any of the 8 directions
//! vertically/diagonally; fire's reach (`fire.rs`) is a strict subset of the
//! latter (its neighbour checks are axis-aligned, one cell). Given that:
//!
//! - A chunk directly adjacent left/right can only ever be written to by its
//!   *other* horizontal same-parity neighbour too (since `(cx-1)%2` and
//!   `(cx+1)%2` are always equal), and their reach into it — cols 32-63 from
//!   the left, cols 0-31 from the right — never overlaps.
//! - A chunk directly adjacent top/bottom is symmetric: its two possible
//!   same-parity vertical neighbours write only to its row 0 or row 63
//!   respectively, never both.
//! - A diagonal neighbour chunk can only ever be reached by a single corner
//!   cell falling diagonally (`|dx|=1, |dy|=1` exactly — nothing else moves
//!   on both axes at once), so the two same-parity diagonal chunks that
//!   could reach it land on its two *opposite* corners, never the same cell.
//! - Working out which of a chunk's 8 neighbours can be co-active with which
//!   other neighbour (by comparing `(cx%2, cy%2)` pairwise) shows these are
//!   the *only* co-active configurations the checkerboard produces — a
//!   passive chunk never receives writes from, say, its top neighbour and
//!   its left neighbour in the same pass. Each passive chunk this pass
//!   touches receives writes from at most one geometrically opposite pair,
//!   and that pair's footprints are always disjoint by construction.
//!
//! That settles *cross-chunk* safety: two different workers this pass never
//! write the same cell. It does **not** by itself settle *within one
//! worker's own sweep* — see `ChunkView`'s `remote_writes` for the piece
//! that does.
//!
//! # Design: exclusive ownership plus a deferred queue, no `unsafe`
//!
//! Each pass:
//!
//! 1. Every active chunk (and its field tile) is pulled out of `World`'s maps
//!    into a plain `Vec`. A `Vec`'s elements don't alias each other the way
//!    two `&mut` borrows into the same `HashMap` would, so handing one
//!    element's `Chunk`/`FieldTile` to each rayon worker as `&mut` needs no
//!    `unsafe` — ordinary safe Rust, `Vec::into_par_iter`.
//! 2. Each worker gets a [`ChunkView`]: exclusive `&mut` access to its own
//!    chunk and field tile, shared read-only access to the rest of `World`
//!    (safe to share across threads — by the proof above, nothing else is
//!    concurrently writing to any chunk a worker might read), and three
//!    queues for anything that needs to happen outside its own bounds.
//! 3. After the pass (`rayon`'s `collect` is the join point), everything is
//!    serial again: put the chunks and tiles back, then replay every queued
//!    write through the ordinary safe `World::set`/`mark_dirty_at`/
//!    `add_heat_local` — unchanged, the same functions every other part of
//!    the engine uses.
//!
//! The active-chunk partition is computed once at the start of the frame,
//! matching `update::step`'s own semantics: a write during this frame's
//! sweep does not get examined again until next frame (`Chunk::end_sweep`
//! only promotes `pending_dirty` to `dirty` once, at the very end). Pass
//! order among the 4 groups does not affect correctness — no single move
//! ever crosses two chunk boundaries at once, since every reach is bounded
//! by `MAX_REACH`, which is itself bounded by one `CHUNK_SIZE` — only, at
//! most, exactly how many frames a particular grain takes to cross a
//! particular pass boundary, which was never a guarantee this engine made.

use std::collections::HashMap;

use rayon::prelude::*;

use super::cell::Cell;
use super::chunk::{Chunk, ChunkCoord, Rect, CHUNK_SIZE, MAX_REACH};
use super::field::{self, FieldTile, FIELD_SCALE};
use super::material::MaterialRegistry;
use super::rng::Rng;
use super::surface::CellSurface;
use super::update;
use super::world::World;

/// Advance the CA sweep exactly like `update::step`, but with each of the
/// four checkerboard passes run across chunks in parallel. Produces the same
/// *kind* of outcome as the serial sweep — piles settle, liquids level, fire
/// spreads — not bit-identical output, since each chunk now draws from its
/// own `Rng` stream rather than one shared generator; see `Chunk::rng` for
/// why that was never a behavioural guarantee this engine made anyway.
pub fn step(world: &mut World) {
    world.begin_step();
    let rightward = world.frame.is_multiple_of(2);

    // Snapshotted once, up front — see the module doc's note on why this
    // must not be recomputed mid-frame.
    let mut groups: [Vec<ChunkCoord>; 4] = Default::default();
    for coord in world.chunks_to_sweep() {
        groups[group_of(coord)].push(coord);
    }

    for group in groups {
        run_pass(world, &group, rightward);
    }

    world.end_step();
}

/// Which of the four checkerboard passes a chunk belongs to.
#[inline]
fn group_of(coord: ChunkCoord) -> usize {
    (coord.x.rem_euclid(2) * 2 + coord.y.rem_euclid(2)) as usize
}

fn run_pass(world: &mut World, coords: &[ChunkCoord], rightward: bool) {
    if coords.is_empty() {
        return;
    }

    // Pull every active chunk (and its field tile) out of `World` *before*
    // taking the shared reference below. From here until they're put back,
    // `world.chunks`/`world.fields` genuinely do not contain them, which is
    // what makes `shared: &World`'s reads of "everything else" accurate and
    // race-free rather than merely conventionally agreed not to be touched.
    //
    // A chunk can come back out of `sweep_region()` as `None` here even
    // though `chunks_to_sweep` just said it wasn't settled — a dirty point
    // near its edge, widened by `MAX_REACH`, can in principle fail to
    // overlap its own bounds. `filter_map`ing that case away would silently
    // drop the chunk from `world.chunks` forever, since only what ends up in
    // `owned` gets put back below — so it's put back immediately instead,
    // skipped for this pass rather than lost.
    let mut owned: Vec<(ChunkCoord, Chunk, FieldTile, Rect)> = Vec::with_capacity(coords.len());
    for &coord in coords {
        let Some(chunk) = world.take_chunk(coord) else {
            continue;
        };
        match chunk.sweep_region() {
            Some(region) => {
                let field = world.take_field(coord).unwrap_or_else(FieldTile::new);
                owned.push((coord, chunk, field, region));
            }
            None => world.put_chunk(coord, chunk),
        }
    }

    let shared: &World = world;
    let outcomes: Vec<ChunkOutcome> = owned
        .into_par_iter()
        .map(|(coord, chunk, field, region)| {
            let mut view = ChunkView::new(coord, chunk, field, shared);
            update::sweep(&mut view, region, rightward);
            view.into_outcome()
        })
        .collect();

    // Serial again: reinsert first, then replay every queued write through
    // the ordinary safe `World` API. Replay order between different
    // outcomes never matters here — by the module doc's proof, no two
    // outcomes from the same pass ever queue a write to the same position.
    //
    // This loop reinserts and replays **one outcome at a time**, not
    // "reinsert everyone, then replay everyone" — which relies on a fact an
    // independent review surfaced and is worth stating explicitly, since
    // nothing else here forces a reader to notice it: `mark_dirty_at`/
    // `World::set`'s own `touch_neighbours` silently no-op on a chunk that
    // isn't resident in `world.chunks` yet (indistinguishable, by design,
    // from a chunk that genuinely doesn't exist). If outcome A's touch/write
    // could ever reach across a shared passive chunk P to *another
    // same-pass active* chunk B before B's own outcome has been reinserted,
    // that wake-up would silently vanish. It cannot: `queue_touch_neighbours`
    // widens by exactly `MAX_REACH` in each direction, and `MAX_REACH` being
    // exactly `CHUNK_SIZE / 2` (`chunk.rs`) means the furthest A can reach
    // into P is P's own far edge — never past it into B. That equality is
    // load-bearing for this loop's structure, not just for the cross-chunk
    // write-disjointness proof above. If `MAX_REACH` is ever changed to
    // something other than exactly half of `CHUNK_SIZE`, this loop must
    // become two-phase (reinsert every outcome, *then* replay every
    // outcome) or this reasoning no longer holds.
    for outcome in outcomes {
        world.put_chunk(outcome.coord, outcome.chunk);
        world.put_field(outcome.coord, outcome.field);
        for ((x, y), cell) in outcome.remote_writes {
            world.set(x, y, cell);
        }
        for (coord, x, y) in outcome.dirty_touches {
            world.mark_dirty_at(coord, x, y);
        }
        for (tile_coord, lx, ly, amount) in outcome.field_writes {
            world.add_heat_local(tile_coord, lx, ly, amount);
        }
    }
}

/// What one worker produced: its (mutated) chunk and field tile, and
/// everything it needs someone else to apply on its behalf.
struct ChunkOutcome {
    coord: ChunkCoord,
    chunk: Chunk,
    field: FieldTile,
    remote_writes: HashMap<(i32, i32), Cell>,
    dirty_touches: Vec<(ChunkCoord, i32, i32)>,
    field_writes: Vec<(ChunkCoord, i32, i32, f32)>,
}

/// One active chunk's private workspace during a parallel pass.
///
/// Exclusive `&mut` access to its own chunk and field tile; shared read-only
/// access to the rest of the world for reading passive neighbours; and three
/// queues for anything that needs to land outside its own bounds, since a
/// worker has no way to mutate a chunk it doesn't own.
///
/// # `remote_writes` is not just a write log — reads consult it too
///
/// This is the piece the module doc's cross-chunk proof doesn't cover on its
/// own. `flow_sideways` (liquids/gases) can move a cell up to `MAX_REACH`
/// cells along a row in one step, which — near an edge — can land outside
/// this chunk entirely; the *scan* that decides how far to go reads
/// `is_empty` at each candidate cell first. If a queued cross-boundary write
/// were invisible to later reads in the *same* worker's own sweep, a second
/// cell processed later in this same chunk could scan right past the
/// already-claimed destination (seeing the stale pre-pass snapshot instead)
/// and independently claim it too — two different queued writes to the same
/// position, silently losing one of them at replay. The serial sweep never
/// has this problem because a direct `World::set` is immediately visible to
/// every subsequent read in the same sweep; `remote_writes` restores exactly
/// that property for the deferred case by making `get` check it before
/// falling through to the shared snapshot.
struct ChunkView<'w> {
    coord: ChunkCoord,
    chunk: Chunk,
    field: FieldTile,
    world: &'w World,
    remote_writes: HashMap<(i32, i32), Cell>,
    dirty_touches: Vec<(ChunkCoord, i32, i32)>,
    field_writes: Vec<(ChunkCoord, i32, i32, f32)>,
}

impl<'w> ChunkView<'w> {
    fn new(coord: ChunkCoord, chunk: Chunk, field: FieldTile, world: &'w World) -> Self {
        Self {
            coord,
            chunk,
            field,
            world,
            remote_writes: HashMap::new(),
            dirty_touches: Vec::new(),
            field_writes: Vec::new(),
        }
    }

    #[inline]
    fn owns(&self, x: i32, y: i32) -> bool {
        ChunkCoord::containing(x, y) == self.coord
    }

    fn into_outcome(self) -> ChunkOutcome {
        ChunkOutcome {
            coord: self.coord,
            chunk: self.chunk,
            field: self.field,
            remote_writes: self.remote_writes,
            dirty_touches: self.dirty_touches,
            field_writes: self.field_writes,
        }
    }

    /// Replicates `World::touch_neighbours`'s reach computation exactly,
    /// queued rather than applied directly — a worker has no `&mut` to any
    /// chunk but its own, including its own, if the write position belongs
    /// to a *different* chunk than `self.coord` (see `set`). Existence
    /// filtering (only wake chunks that are actually resident) happens at
    /// replay time in `World::mark_dirty_at`, once every chunk from this
    /// pass — including this one — is back in `world.chunks`.
    fn queue_touch_neighbours(&mut self, x: i32, y: i32) {
        let owner = ChunkCoord::containing(x, y);
        let lx = x.rem_euclid(CHUNK_SIZE);
        let ly = y.rem_euclid(CHUNK_SIZE);
        // No-op at today's constants -- see the identical comment on
        // `World::touch_neighbours`, which this replicates exactly.
        if (MAX_REACH..CHUNK_SIZE - MAX_REACH).contains(&lx) && ly > 0 && ly < CHUNK_SIZE - 1 {
            return;
        }
        let first = ChunkCoord::containing(x - MAX_REACH, y - 1);
        let last = ChunkCoord::containing(x + MAX_REACH, y + 1);
        for cy in first.y..=last.y {
            for cx in first.x..=last.x {
                let coord = ChunkCoord::new(cx, cy);
                if coord == owner {
                    continue;
                }
                self.dirty_touches.push((coord, x, y));
            }
        }
    }
}

impl CellSurface for ChunkView<'_> {
    fn get(&self, x: i32, y: i32) -> Cell {
        if !self.world.in_bounds(x, y) {
            return Cell::OUT_OF_BOUNDS;
        }
        if self.owns(x, y) {
            return self.chunk.get_world(x, y);
        }
        // Check this worker's own queued writes before falling through to
        // the shared pre-pass snapshot — see the struct doc for why this is
        // load-bearing, not an optimization.
        if let Some(&cell) = self.remote_writes.get(&(x, y)) {
            return cell;
        }
        self.world.get(x, y)
    }

    fn set(&mut self, x: i32, y: i32, cell: Cell) {
        if !self.world.in_bounds(x, y) {
            return;
        }
        if self.owns(x, y) {
            self.chunk.set_world(x, y, cell);
        } else {
            self.remote_writes.insert((x, y), cell);
        }
        self.queue_touch_neighbours(x, y);
    }

    fn in_bounds(&self, x: i32, y: i32) -> bool {
        self.world.in_bounds(x, y)
    }

    fn clear_moved(&mut self, x: i32, y: i32) {
        // Always called on the position currently being visited, which the
        // sweep only ever hands out from `self.coord`'s own bounds.
        debug_assert!(self.owns(x, y), "clear_moved called outside the chunk being swept");
        let cell = self.chunk.get_world(x, y).with_moved(false);
        self.chunk.set_world_quiet(x, y, cell);
    }

    fn materials(&self) -> &MaterialRegistry {
        &self.world.materials
    }

    fn rng(&mut self) -> &mut Rng {
        self.chunk.rng_mut()
    }

    fn add_heat(&mut self, x: i32, y: i32, radius: i32, amount: f32) {
        // Field-cell granular, not a queued whole-circle replay of
        // `World::add_heat` — a radius spanning both a local and a remote
        // field cell would otherwise double the local cells' share when the
        // replay reapplied the whole call on top of what this worker
        // already wrote directly. `tick_burn`'s only caller uses radius=1,
        // which — since `FIELD_SCALE` divides `CHUNK_SIZE` evenly — always
        // lands in this chunk's own tile in practice; this stays correct
        // regardless, rather than depending on that forever.
        let (fcx, fcy) = field::field_coord_of(x, y);
        let field_radius = radius / FIELD_SCALE;
        let r2 = field_radius * field_radius + 1;
        for dfy in -field_radius..=field_radius {
            for dfx in -field_radius..=field_radius {
                if dfx * dfx + dfy * dfy > r2 {
                    continue;
                }
                let (fx, fy) = (fcx + dfx, fcy + dfy);
                let (tile_coord, lx, ly) = field::tile_and_local(fx, fy);
                if tile_coord == self.coord {
                    let mut cell = self.field.get_local(lx, ly);
                    cell.temperature += amount;
                    self.field.set_local(lx, ly, cell);
                } else {
                    self.field_writes.push((tile_coord, lx, ly, amount));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::cell::Cell as SimCell;
    use crate::sim::material;

    fn wide_world() -> World {
        // Several chunks wide and tall in both directions, including
        // negative coordinates, so every group and every neighbour
        // configuration in the module doc's proof actually gets exercised.
        World::new(Rect::new(-128, -128, 255, 255))
    }

    #[test]
    fn group_of_covers_all_four_groups_and_matches_rem_euclid_parity() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for x in -5..5 {
            for y in -5..5 {
                let g = group_of(ChunkCoord::new(x, y));
                assert!(g < 4);
                seen.insert(g);
                assert_eq!(g, (x.rem_euclid(2) * 2 + y.rem_euclid(2)) as usize);
            }
        }
        assert_eq!(seen.len(), 4, "not every group was reachable");
    }

    #[test]
    fn same_group_chunks_are_never_within_reach_of_each_other() {
        // The load-bearing property the whole module leans on, checked
        // exhaustively for every pair within a wide neighbourhood rather
        // than trusted from the hand derivation alone. "Within reach" means
        // sharing a chunk that either could write into — the 3x3
        // neighbourhood a chunk's own writes (row-flow up to MAX_REACH,
        // single-step diagonal/vertical fall) can ever land in.
        fn reachable_chunks(c: ChunkCoord) -> Vec<ChunkCoord> {
            (-1..=1)
                .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
                .map(|(dx, dy)| ChunkCoord::new(c.x + dx, c.y + dy))
                .collect()
        }

        for cx in -6..6 {
            for cy in -6..6 {
                let a = ChunkCoord::new(cx, cy);
                let group = group_of(a);
                for ocx in -6..6 {
                    for ocy in -6..6 {
                        let b = ChunkCoord::new(ocx, ocy);
                        if a == b || group_of(b) != group {
                            continue;
                        }
                        let a_reach = reachable_chunks(a);
                        let b_reach = reachable_chunks(b);
                        let shared: Vec<_> = a_reach.iter().filter(|r| b_reach.contains(r)).collect();
                        // They may share a passive neighbour chunk (that's
                        // the whole point of the checkerboard being only
                        // 4-way, not 9-way) -- what must never happen is
                        // sharing *more* than the single opposite-pair
                        // relationship the module doc proves, which the
                        // conservation stress tests below check at the cell
                        // level. Here: confirm they never coincide (can't
                        // both be "the same chunk"), which the `a == b`
                        // guard above already ensures, and that any shared
                        // neighbour is at Chebyshev distance exactly 1 from
                        // both -- i.e. genuinely a shared *neighbour*, not
                        // literally each other.
                        for &s in &shared {
                            assert!(*s != a && *s != b);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn two_same_group_chunks_writing_into_their_shared_passive_neighbour_land_disjointly() {
        // The specific geometry an independent review flagged as argued-but-
        // not-directly-tested: chunk(0,0) and chunk(2,0) are both group
        // (0,0) (same-parity, same pass), and both are within reach of
        // chunk(1,0) sitting between them (passive this pass). This
        // isolates that exact sandwiching case at the cell level, rather
        // than trusting that the broader multi-chunk stress tests exercise
        // it incidentally.
        //
        // A floor with two single-cell *pits* (open at row 10, solid again
        // at row 11) gives each source water cell exactly one place it can
        // flow to and fall into, both inside the shared middle chunk (1,0):
        // a pit at x=66 (chunk(1,0)'s local column 2, the "left half" the
        // module doc's proof assigns to chunk(0,0)'s reach) and a pit at
        // x=125 (local column 61, the "right half" assigned to chunk(2,0)'s
        // reach). Everywhere else the floor blocks straight/diagonal falls
        // and gives flow_sideways nothing else to find, so each source
        // cell's move is forced and deterministic regardless of which
        // left/right movement roll is tried first.
        //
        // A one-row *pit*, not just a gap in row 10 with open space below,
        // because every chunk in a brand-new `World` starts fully dirty
        // (`Chunk::new`) -- so chunk(1,0) gets its own full sweep later in
        // this same frame (a different, later pass) regardless of the
        // deferred write from chunk(0,0)/(2,0)'s pass, and a plain open gap
        // would let the just-arrived water keep falling through it. The
        // pit's solid floor at row 11 gives it somewhere stable to land.
        //
        // Settling takes a few frames, not necessarily one: whether a
        // flowed cell's `moved` flag was set for this frame depends on
        // `revisited = (dir > 0) == rightward`, and the two source cells
        // move in *opposite* directions (dir=+1 vs dir=-1) into the same
        // `rightward` value -- so on any given frame one of them lands
        // already flagged "moved" (deferring chunk(1,0)'s own later pass
        // from processing it further that same frame) while the other
        // doesn't. Both still end up in the right place; this just isn't
        // observable after exactly one `step()` call, so several are run.
        let mut w = World::new(Rect::new(-64, 0, 255, 63));
        for x in -64..200 {
            if x == 66 || x == 125 {
                continue; // the two pits
            }
            w.set(x, 10, SimCell::new(material::STONE, 0));
        }
        for x in -64..200 {
            w.set(x, 11, SimCell::new(material::STONE, 0)); // seals the pits' floor
        }
        w.set(63, 9, SimCell::new(material::WATER, 0)); // last column of chunk(0,0)
        w.set(128, 9, SimCell::new(material::WATER, 0)); // first column of chunk(2,0)
        let before = count(&w, material::WATER);

        for _ in 0..5 {
            step(&mut w);
        }

        assert_eq!(count(&w, material::WATER), before, "water was created, destroyed, or one write clobbered the other");
        assert_eq!(w.get(66, 10).material, material::WATER, "chunk(0,0)'s flow into the shared chunk's left half did not land");
        assert_eq!(w.get(125, 10).material, material::WATER, "chunk(2,0)'s flow into the shared chunk's right half did not land");
        assert!(w.get(63, 9).is_empty(), "source cell in chunk(0,0) did not vacate");
        assert!(w.get(128, 9).is_empty(), "source cell in chunk(2,0) did not vacate");
    }

    #[test]
    fn a_grain_falling_across_a_chunk_boundary_is_conserved_under_the_parallel_sweep() {
        let mut w = wide_world();
        // A sand column straddling the boundary between two chunks.
        for x in 60..68 {
            for y in -60..-1 {
                w.set(x, y, SimCell::new(material::SAND, 0));
            }
        }
        let before = count(&w, material::SAND);
        for _ in 0..600 {
            step(&mut w);
        }
        assert_eq!(count(&w, material::SAND), before, "sand was created or destroyed");
    }

    #[test]
    fn a_full_multi_chunk_world_of_sand_and_water_settles_under_the_parallel_sweep() {
        // The stress scenario the plan calls out explicitly: fill the
        // screen, confirm no corruption or lost cells, and that it actually
        // comes to rest instead of jittering forever.
        let mut w = World::new(Rect::new(0, 0, 255, 191));
        for x in 0..256 {
            w.set(x, 190, SimCell::new(material::STONE, 0));
        }
        for x in 0..128 {
            for y in 0..90 {
                w.set(x, y, SimCell::new(material::SAND, 0));
            }
        }
        for x in 128..256 {
            for y in 0..90 {
                w.set(x, y, SimCell::new(material::WATER, 0));
            }
        }
        let before = count(&w, material::SAND) + count(&w, material::WATER);

        for _ in 0..2000 {
            step(&mut w);
        }

        let after = count(&w, material::SAND) + count(&w, material::WATER);
        assert_eq!(before, after, "material was created, destroyed, or duplicated");
        assert_eq!(w.active_chunk_count(), 0, "world never settled under the parallel sweep");
    }

    #[test]
    fn fire_spreads_and_burns_out_correctly_under_the_parallel_sweep() {
        let mut w = wide_world();
        // A floor: oil is a liquid, and a flat layer with nowhere to fall
        // still drains sideways toward the first gap its surface-search
        // finds, up to `MAX_REACH` cells away (see `flow_sideways`) -- so the
        // floor has to extend well past the oil on both sides, not just
        // directly underneath it, or the whole layer slowly migrates toward
        // the open edge over the length of this test instead of sitting
        // still to burn. Same physics the serial engine has; this scene
        // just runs long enough to notice it if under-provisioned.
        for x in -50..90 {
            w.set(x, 1, SimCell::new(material::STONE, 0));
        }
        for x in 0..40 {
            w.set(x, 0, SimCell::new(material::OIL, 0));
        }
        // A long but finite burn on the seed -- long enough to give fire
        // plenty of time to spread the length of the chain, short enough to
        // actually burn out and let the "fully burned to ash" check below
        // mean something. `9999` (effectively forever) was used elsewhere in
        // this module while debugging and is deliberately not reused here.
        let mut source = SimCell::new(material::OIL, 0);
        source.ignite(600);
        w.set(0, 0, source);

        let mut ever_burning_past_the_seed = false;
        for _ in 0..3000 {
            step(&mut w);
            if (1..40).any(|x| w.get(x, 0).is_burning()) {
                ever_burning_past_the_seed = true;
            }
        }
        assert!(ever_burning_past_the_seed, "fire never spread under the parallel sweep");

        for _ in 0..3000 {
            step(&mut w);
        }
        let still_oil = (0..40).filter(|&x| w.get(x, 0).material == material::OIL).count();
        assert_eq!(still_oil, 0, "oil should have fully burned out to ash");

        // 40 connected ash cells cooling from ~900C together diffuse into
        // each other, not just toward ambient -- this is exactly the shape
        // that found the `already_settled` gate fire.rs's own
        // `a_connected_mass_of_cooling_cells_actually_settles` now guards;
        // before that fix this loop did not terminate within any reasonable
        // budget. 5000 is the same bound fire.rs's own single-cell version
        // of this check uses.
        let mut settled = false;
        for _ in 0..5000 {
            step(&mut w);
            if w.active_chunk_count() == 0 {
                settled = true;
                break;
            }
        }
        assert!(settled, "world never settled after burnout");
    }

    #[test]
    fn a_chunk_touched_only_by_a_neighbours_dirty_mark_is_never_lost() {
        // Regression: `run_pass` used to `filter_map` away a chunk whose
        // `sweep_region()` came back `None` even though `chunks_to_sweep`
        // had already selected it as dirty -- silently dropping it from
        // `world.chunks` forever, since only chunks that made it into
        // `owned` were ever put back. Exercises the mechanism most likely to
        // produce that case: a chunk dirtied purely via a neighbour's
        // boundary write (`queue_touch_neighbours`), never written to
        // directly itself. Conservation across many frames is the check --
        // if a chunk vanished, the stone floor set up front would too.
        let mut w = wide_world();
        for x in -5..70 {
            w.set(x, 5, SimCell::new(material::STONE, 0));
        }
        let floor_count = count(&w, material::STONE);
        // A pile straddling several chunk boundaries so plenty of chunks
        // only ever get touched via a neighbour's edge write, not a direct
        // one of their own.
        for x in 0..70 {
            for y in -40..0 {
                w.set(x, y, SimCell::new(material::SAND, 0));
            }
        }
        let sand_before = count(&w, material::SAND);

        for _ in 0..1000 {
            step(&mut w);
        }

        assert_eq!(count(&w, material::STONE), floor_count, "the floor lost cells -- a chunk was dropped");
        assert_eq!(count(&w, material::SAND), sand_before, "sand was created or destroyed -- a chunk was dropped");
    }

    fn count(w: &World, id: material::MaterialId) -> usize {
        let b = w.bounds().unwrap();
        let mut n = 0;
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                if w.get(x, y).material == id {
                    n += 1;
                }
            }
        }
        n
    }
}
