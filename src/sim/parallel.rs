//! M5: the multithreaded CA sweep.
//!
//! `rayon` over a checkerboard sweep — one pass per (chunk row, `cx` parity),
//! taken bottom row first (see `step` for why it is not the plan's original
//! `(cx % 2, cy % 2)` four-group form) — but the concurrency-safety design here
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
//! - Vertical co-activity cannot arise at all: a pass holds exactly one
//!   chunk row (`pass_key`), so a passive chunk's top and bottom neighbours
//!   are never active together, and the top/bottom case above reduces to a
//!   single writer. Each passive chunk this pass touches therefore receives
//!   writes from at most one horizontally opposite pair, whose footprints are
//!   disjoint by construction. This is strictly *narrower* than the four-group
//!   form it replaced — fewer chunks are co-active, never more — so it cannot
//!   invalidate any argument that held there.
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
//! order does not affect *safety* — no single move ever crosses two chunk
//! boundaries at once, since every reach is bounded by `MAX_REACH`, itself
//! bounded by one `CHUNK_SIZE`. It does affect behaviour, which is the whole
//! reason chunk rows are ordered rather than parity-split: see `step`.

use std::collections::HashMap;

use rayon::prelude::*;

use super::cell::Cell;
use super::chunk::{Chunk, ChunkCoord, Rect, CHUNK_SIZE, MAX_REACH};
use super::field::{self, FieldTile, FIELD_SCALE};
use super::material::{MaterialKind, MaterialRegistry};
use super::rng::Rng;
use super::scheduler::ActiveSite;
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
    // Weather before the sweep, so rain landing this frame is material the
    // sweep then moves -- rather than a drop that sits for a frame before
    // anything notices it. Both drivers, deliberately: `CLAUDE.md`'s "two
    // drivers, and the app runs the parallel one".
    super::weather::step(world);
    let rightward = world.frame.is_multiple_of(2);

    // Snapshotted once, up front — see the module doc's note on why this
    // must not be recomputed mid-frame.
    let active = world.chunks_to_sweep();

    // **Chunk rows bottom to top, and only then a checkerboard on `cx`.**
    //
    // The obvious four-group form — `(cx % 2, cy % 2)`, run in index order —
    // silently breaks the engine's other ordering rule. Rows sweep bottom to
    // top so a falling column descends as a unit, and that requires the chunk
    // row *below* to be swept before the one above. A parity split on `cy`
    // cannot deliver it: `cy` and `cy + 1` always land in different groups,
    // so whichever fixed order the four passes run in, half of all horizontal
    // seams get the upper chunk first. A cell dropped across such a seam is
    // then moved again by the receiving chunk in the same frame — two cells
    // in one frame, which thins a falling body into a dark one-row line lying
    // exactly on the seam. Reported from live play, on exactly the half of
    // the seams this predicts.
    //
    // Marking those cells `revisited` instead was tried twice and reverted
    // both times (`e816477`): it does clear the line, and it replaces it with
    // a throttle at the same seam, because every crossing cell now waits a
    // frame. Measured at a summed row-fill deficit of 2236 asking about every
    // cross-chunk move and 1948 asking only about downward ones, against 988
    // for this ordering. The artifact moves; it does not go away. Ordering
    // the sweep correctly costs no cell anything.
    //
    // Safety is unchanged, and rests on the same argument the module doc
    // makes: every chunk in a pass now shares a `cy` *and* a `cx` parity, so
    // any two are at least two apart horizontally, and `MAX_REACH` being
    // exactly `CHUNK_SIZE / 2` keeps their write footprints — into each
    // other, into a shared horizontal neighbour, or into the rows above and
    // below — disjoint by construction. Chunk rows are processed strictly one
    // at a time, so a worker's ±1-row writes never land in a row anyone else
    // is currently sweeping.
    // Grouped through `pass_key` rather than by open-coding the same
    // predicate here, so the driver and the safety tests that check the
    // co-activity invariant cannot drift apart. Ordered chunk row descending
    // (lower rows are larger `cy`, and they must go first), then parity.
    let mut passes: Vec<(i32, i32)> = active.iter().map(|&c| pass_key(c)).collect();
    passes.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    passes.dedup();

    for key in passes {
        let pass: Vec<ChunkCoord> = active.iter().copied().filter(|&c| pass_key(c) == key).collect();
        run_pass(world, &pass, rightward);
    }

    world.end_step();
}

/// Which pass a chunk is swept in, as an orderable key: chunk row first,
/// then `cx` parity. Two chunks run **concurrently exactly when this
/// matches**, which is the property every safety argument in this module is
/// stated against.
///
/// Was `(cx % 2, cy % 2)` collapsed into one of four groups. That form put
/// vertically adjacent chunk rows in different groups and therefore in a
/// fixed order, which broke the bottom-to-top row invariant at half of all
/// horizontal seams -- see `step`.
#[inline]
fn pass_key(coord: ChunkCoord) -> (i32, i32) {
    (coord.y, coord.x.rem_euclid(2))
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
    // The `None` arm below is now unreachable from `chunks_to_sweep`:
    // `Chunk::is_settled` answers from `sweep_region` itself, so a chunk
    // whose dirty mark cannot expand back into its own bounds reports
    // settled and never gets here. It used to be very reachable — a
    // neighbour's out-of-bounds dirty mark plus a short `reach` produced
    // exactly this, and such chunks then sat awake-but-never-swept forever.
    //
    // Kept anyway, deliberately. `sweep_region` returning `Option` is a
    // total contract this function must honour regardless of which caller
    // selected the coordinate, and the cost of honouring it is one branch
    // per active chunk per pass. What must *not* happen is `filter_map`ing
    // the case away: only what ends up in `owned` gets put back below, so
    // discarding a chunk here would drop it from `world.chunks` forever.
    // `a_chunk_touched_only_by_a_neighbours_dirty_mark_is_never_lost`
    // covers that.
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
    // Liquid-body disturbance positions accumulated across every outcome
    // (both a same-chunk write's own `demotions` queue and a remote
    // write's disturbance, detected below) but deliberately *not* resolved
    // until every chunk from this pass is back in `world.chunks` — see the
    // loop's trailing comment.
    let mut pending_demotions: Vec<(i32, i32)> = Vec::new();
    // Liquid absorbed into a body this pass, same deferred-resolution
    // reasoning: `World::absorb_liquid`'s own rasterization
    // (`LiquidBody::rasterize_column`) can write into a chunk the growing
    // column just crossed into, which might not be reinserted yet.
    let mut pending_absorptions: Vec<(i32, i32, u32)> = Vec::new();

    for outcome in outcomes {
        world.put_chunk(outcome.coord, outcome.chunk);
        world.put_field(outcome.coord, outcome.field);
        for ((x, y), cell) in outcome.remote_writes {
            // `world.set_owned`, not `world.set` — `set`'s own built-in
            // disturbance check (`Reports/liquid-heightfield-design.md`
            // §5a) would resolve and demote *immediately*, which is unsafe
            // here: this write's target chunk is safely resident (a
            // remote write can only ever land in a passive chunk this
            // pass — `MAX_REACH` bounds it below a full chunk width, so it
            // can never reach a second same-pass *active* chunk), but the
            // body that owns the disturbed cell can still have *other*
            // columns in an active chunk from this same pass that hasn't
            // been reinserted yet. Detected here instead, deferred to
            // `pending_demotions` below.
            let old = world.get(x, y);
            if old.managed() {
                pending_demotions.push((x, y));
            }
            world.set_owned(x, y, cell);
            // `set_owned` goes straight to `write_cell`, so it bypasses
            // `World::set`'s organism bookkeeping exactly as the same-chunk
            // path does -- the remote half of the same seam, and it has to
            // be replayed here for the same reason. Missing this left a
            // seed whose *last* fall step crossed a chunk boundary out of
            // its own organism's cell list, which is a one-row-in-64 window
            // and produced a sterile single cell rather than a tree. See
            // `World::reindex_organism_cell`.
            world.reindex_organism_cell(x, y, old.organism_id(), cell.organism_id());
        }
        for (coord, x, y) in outcome.dirty_touches {
            world.mark_dirty_at(coord, x, y);
        }
        for (tile_coord, lx, ly, amount) in outcome.field_writes {
            world.add_heat_local(tile_coord, lx, ly, amount);
        }
        for (tile_coord, lx, ly, amount) in outcome.light_writes {
            world.add_light_local(tile_coord, lx, ly, amount);
        }
        // See `ChunkView::field_touched`'s own doc: a same-chunk heat push
        // has no `&mut World` to clear `fields_settled` (issue #4) on the
        // spot, so it's replayed here instead, the same as every other
        // queued write above.
        if outcome.field_touched {
            world.set_fields_settled(false);
        }
        for site in outcome.pending_active_sites {
            world.schedule_active_site(site);
        }
        for (x, y, was, now) in outcome.organism_moves {
            world.reindex_organism_cell(x, y, was, now);
        }
        world.phase_changes.merge(outcome.phase_counts);
        pending_demotions.extend(outcome.demotions);
        pending_absorptions.extend(outcome.absorptions);
    }

    // Absorptions before demotions, both only once every chunk from this
    // pass is resident again (see `pending_demotions`'s/`pending_
    // absorptions`'s own doc above). The order between the two matters:
    // an absorption's debit (the source cell emptying) already happened
    // synchronously during the sweep, so if the same body were demoted
    // *before* its pending credit resolved, `absorb_liquid` would find no
    // live body and silently drop the fill -- a real mass loss. Resolving
    // absorption first instead means that credit either lands on a still-
    // live body, or -- if something else also disturbed this body this
    // same pass -- gets written into ordinary cells by `rasterize_column`
    // and then correctly folded into the CA grid by the demotion that
    // follows, never lost either way. `demote_body_at` is safe to call
    // more than once for the same body (a no-op once it's already gone),
    // so no dedup is needed even though several disturbed positions
    // commonly share one owner.
    for (x, y, fill) in pending_absorptions {
        world.absorb_liquid(x, y, fill);
    }
    for (x, y) in pending_demotions {
        world.demote_body_at(x, y);
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
    light_writes: Vec<(ChunkCoord, i32, i32, f32)>,
    field_touched: bool,
    pending_active_sites: Vec<ActiveSite>,
    /// Positions where a same-chunk write overwrote a `FLAG_MANAGED` cell
    /// (`Reports/liquid-heightfield-design.md` §5a) — see `ChunkView::set`'s
    /// own comment for why this is a genuinely separate queue from the
    /// others here, not something `World::set`'s own disturbance check
    /// already covers.
    demotions: Vec<(i32, i32)>,
    /// Liquid absorbed into a promoted body (`Reports/liquid-heightfield-
    /// design.md` §6b/§8b) — `(x, y)` is the managed cell the fill was
    /// absorbed *into*, not the source (already emptied via the ordinary
    /// same-pass `set`/`remote_writes` path). Only `World` owns `bodies`,
    /// so this is queued and resolved in `run_pass`, same reasoning as
    /// `demotions` — deferred until every chunk from the pass is resident
    /// again, since `World::absorb_liquid`'s own rasterization can write
    /// into a chunk the growing column just crossed into.
    absorptions: Vec<(i32, i32, u32)>,
    /// Organism cell-list membership changes from *same-chunk* writes —
    /// `(x, y, was_organism_id, now_organism_id)`.
    ///
    /// `World::set` maintains `OrganismState::cells` itself, but **neither
    /// half of this function's write path goes through it**: a same-chunk
    /// write lands in the worker's own `Chunk`, and a remote write is
    /// replayed by `run_pass` through `World::set_owned`, which calls
    /// `write_cell` directly. So both halves have to replay the
    /// bookkeeping — this queue covers the same-chunk half, and `run_pass`
    /// covers the remote one at the point of replay.
    ///
    /// An earlier revision of this comment claimed remote writes were
    /// replayed through `World::set` and fixed only the same-chunk half.
    /// They are not, and the surviving half of the bug had a one-row-in-64
    /// window: a seed whose *last* fall step crossed a chunk boundary
    /// vanished from its cell list and grew into a single sterile cell.
    /// Caught by independent review, not by the suite. See
    /// `World::reindex_organism_cell`.
    organism_moves: Vec<(i32, i32, u16, u16)>,
    /// See `ChunkView::phase_counts`'s own doc.
    phase_counts: crate::sim::fire::PhaseCounts,
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
    /// Same shape as `field_writes`, one channel over — queued cross-chunk
    /// light writes from `add_light`, replayed via `World::add_light_local`.
    light_writes: Vec<(ChunkCoord, i32, i32, f32)>,
    /// Set whenever `add_heat`/`add_light`'s same-chunk branch writes
    /// directly into `self.field`, below — a worker has no `&mut World` to
    /// clear `fields_settled` (issue #4) on the spot, unlike `World::
    /// add_heat`/`add_light`'s serial path, so this is queued and applied
    /// once after the pass, mirroring `field_writes`'s own reason for
    /// existing. Deliberately one flag for both channels, not two — this
    /// only ever needs to answer "did *anything* change in my own field
    /// tile," never which channel. Found by independent review: without
    /// this, a same-chunk heat push (the common case — `FIELD_SCALE`
    /// divides `CHUNK_SIZE` evenly, so `tick_burn`'s radius-1 call almost
    /// always lands here) left `fields_settled` unchanged, currently masked
    /// only by the coincidence that a burning cell's own `tick_burn` also
    /// writes its cell every frame it burns, independently keeping the
    /// chunk (and therefore `active_chunk_count()`) awake regardless.
    field_touched: bool,
    /// Sites queued via `schedule_active_site` (architecture §5f, ash
    /// decay) — only `World` owns the active-site heap, so a worker has
    /// nowhere to put a newly-scheduled site except here, replayed once
    /// after the pass. Unlike `field_writes`/`light_writes`, no same-
    /// chunk-vs-remote split is needed: the heap isn't chunk-scoped at all,
    /// so every queued site is handled identically regardless of where it
    /// sits.
    pending_active_sites: Vec<ActiveSite>,
    /// See `ChunkOutcome::demotions`'s own doc.
    demotions: Vec<(i32, i32)>,
    /// See `ChunkOutcome::absorptions`'s own doc.
    absorptions: Vec<(i32, i32, u32)>,
    /// See `ChunkOutcome::organism_moves`'s own doc.
    organism_moves: Vec<(i32, i32, u16, u16)>,
    /// This worker's private `fire::PhaseCounts` tally, merged into
    /// `World::phase_changes` by `run_pass` — only `World` owns the
    /// cumulative counters, same reasoning as `pending_active_sites`.
    phase_counts: crate::sim::fire::PhaseCounts,
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
            light_writes: Vec::new(),
            field_touched: false,
            pending_active_sites: Vec::new(),
            demotions: Vec::new(),
            absorptions: Vec::new(),
            organism_moves: Vec::new(),
            phase_counts: crate::sim::fire::PhaseCounts::default(),
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
            light_writes: self.light_writes,
            field_touched: self.field_touched,
            pending_active_sites: self.pending_active_sites,
            demotions: self.demotions,
            absorptions: self.absorptions,
            organism_moves: self.organism_moves,
            phase_counts: self.phase_counts,
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
            // Liquid-body disturbance detection (`Reports/liquid-
            // heightfield-design.md` §5a): a *remote* write's disturbance
            // is caught for free when it's replayed through the ordinary
            // `World::set` after the pass (see `run_pass`), since that
            // function carries the identical check. A *same-chunk* write
            // like this one never goes through `World::set` at all — it
            // writes `self.chunk` directly — so it needs its own check,
            // queued here and resolved once every outcome from this pass
            // is back in `world.chunks` (`run_pass`'s own comment on why
            // that ordering matters: a body can span two same-parity active
            // chunks, and resolving mid-loop could look up a body whose
            // other columns live in a chunk not yet reinserted).
            let old = self.chunk.get_world(x, y);
            if old.managed() {
                self.demotions.push((x, y));
            }
            // Same reason the disturbance check above is queued here: this
            // write never passes through `World::set`, so the organism
            // cell-list bookkeeping that lives there has to be replayed.
            if old.organism_id() != cell.organism_id() {
                self.organism_moves.push((x, y, old.organism_id(), cell.organism_id()));
            }
            let reach = self.world.materials.get(cell.material).sweep_reach();
            let is_liquid = self.world.materials.kind(cell.material) == MaterialKind::Liquid;
            self.chunk.set_world(x, y, cell, reach, is_liquid);
        } else {
            // Reach for this chunk's own tracked value is handled when this
            // write is replayed through the ordinary `World::set` after the
            // pass (see `run_pass`) — this worker has no `&mut` to the
            // remote chunk to update it directly. That replay is also what
            // catches this write's own disturbance check, above.
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

    fn clear_undercut(&mut self, x: i32, y: i32) {
        // Same contract as `clear_moved` above: only ever the position being
        // visited, which is always inside this worker's own chunk.
        debug_assert!(self.owns(x, y), "clear_undercut called outside the chunk being swept");
        let cell = self.chunk.get_world(x, y).with_undercut(false);
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
                    self.field_touched = true;
                } else {
                    self.field_writes.push((tile_coord, lx, ly, amount));
                }
            }
        }
    }

    fn add_light(&mut self, x: i32, y: i32, radius: i32, amount: f32) {
        // Same field-cell-granular reasoning as `add_heat` above.
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
                    cell.light += amount;
                    self.field.set_local(lx, ly, cell);
                    self.field_touched = true;
                } else {
                    self.light_writes.push((tile_coord, lx, ly, amount));
                }
            }
        }
    }

    fn field_moisture_at(&self, x: i32, y: i32) -> f32 {
        let (fx, fy) = field::field_coord_of(x, y);
        let (tile_coord, lx, ly) = field::tile_and_local(fx, fy);
        debug_assert_eq!(
            tile_coord, self.coord,
            "field_moisture_at called with a position outside this worker's own chunk"
        );
        self.field.get_local(lx, ly).moisture
    }

    fn field_wind_at(&self, x: i32, y: i32) -> (f32, f32) {
        let (fx, fy) = field::field_coord_of(x, y);
        let (tile_coord, lx, ly) = field::tile_and_local(fx, fy);
        debug_assert_eq!(
            tile_coord, self.coord,
            "field_wind_at called with a position outside this worker's own chunk"
        );
        let f = self.field.get_local(lx, ly);
        (f.vx, f.vy)
    }

    fn frame(&self) -> u64 {
        self.world.frame
    }

    fn schedule_active_site(&mut self, site: ActiveSite) {
        self.pending_active_sites.push(site);
    }

    fn absorb_liquid(&mut self, x: i32, y: i32, fill: u32) {
        self.absorptions.push((x, y, fill));
    }

    fn count_phase_event(&mut self, event: crate::sim::fire::PhaseEvent) {
        // Tallied privately and merged by `run_pass` — only `World` owns
        // `phase_changes`, the same reasoning as `pending_active_sites`.
        self.phase_counts.record(event);
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
    fn a_pass_holds_one_chunk_row_and_one_cx_parity() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for x in -5..5 {
            for y in -5..5 {
                let k = pass_key(ChunkCoord::new(x, y));
                assert_eq!(k, (y, x.rem_euclid(2)));
                seen.insert(k);
            }
        }
        // Ten rows, two parities each: every pass in the neighbourhood is
        // reachable, and no two rows are ever merged into one pass -- which
        // is the property that keeps chunk rows strictly ordered.
        assert_eq!(seen.len(), 20, "not every pass was reachable");
    }

    #[test]
    fn concurrent_chunks_are_never_within_reach_of_each_other() {
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
                let group = pass_key(a);
                for ocx in -6..6 {
                    for ocy in -6..6 {
                        let b = ChunkCoord::new(ocx, ocy);
                        if a == b || pass_key(b) != group {
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
        // The specific geometry an independent review originally flagged as
        // argued-but-not-directly-tested: chunk(0,0) and chunk(2,0) are both
        // group (0,0) (same-parity, same pass), and both are within reach
        // of chunk(1,0) sitting between them (passive this pass). This
        // isolates that exact sandwiching case at the cell level, rather
        // than trusting that the broader multi-chunk stress tests exercise
        // it incidentally.
        //
        // Rewritten for the compressible-volume liquid model: the original
        // version routed water through `flow_sideways`'s long-range search
        // to a specific pit several cells away, which `Liquid` no longer
        // does at all -- transfer now only ever reaches an immediate (1-
        // cell) neighbour, so the two source cells sit directly adjacent to
        // chunk(1,0)'s near edge on each side. A floor blocks straight-down
        // and both diagonal falls, forcing the only available move to be
        // the horizontal transfer this test exists to exercise.
        let mut w = World::new(Rect::new(-64, 0, 255, 63));
        for x in -64..200 {
            w.set(x, 11, SimCell::new(material::STONE, 0));
        }
        w.set(63, 10, SimCell::new(material::WATER, 0)); // last column of chunk(0,0)
        w.set(128, 10, SimCell::new(material::WATER, 0)); // first column of chunk(2,0)
        let before = liquid_volume(&w, material::WATER);

        for _ in 0..5 {
            step(&mut w);
        }

        assert_eq!(liquid_volume(&w, material::WATER), before, "water was created, destroyed, or one write clobbered the other");
        assert_eq!(w.get(64, 10).material, material::WATER, "chunk(0,0)'s transfer into the shared chunk's near edge did not land");
        assert_eq!(w.get(127, 10).material, material::WATER, "chunk(2,0)'s transfer into the shared chunk's near edge did not land");
        // Both source cells should have given up *some* fill (a partial
        // transfer, not a full vacate -- this model moves fill gradually,
        // capped by `flow_rate`, not the whole cell at once).
        assert!(update::liquid_fill(w.get(63, 10)) < material::LIQUID_FULL, "source cell in chunk(0,0) never gave up any fill");
        assert!(update::liquid_fill(w.get(128, 10)) < material::LIQUID_FULL, "source cell in chunk(2,0) never gave up any fill");
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
        // Sand is a whole-cell-move material, so its cell count really is
        // conserved directly. Water's conserved quantity is fill volume, not
        // cell count -- see `liquid_volume`'s own doc.
        let sand_before = count(&w, material::SAND);
        let water_before = liquid_volume(&w, material::WATER);

        for _ in 0..2000 {
            step(&mut w);
        }

        assert_eq!(count(&w, material::SAND), sand_before, "sand was created, destroyed, or duplicated");
        assert_eq!(liquid_volume(&w, material::WATER), water_before, "water was created or destroyed");
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
    fn a_same_chunk_heat_push_during_the_parallel_sweep_wakes_the_settled_field() {
        // Regression: an independent review of issue #4 (field sleeping)
        // found that ChunkView::add_heat's same-chunk branch (the common
        // case -- FIELD_SCALE divides CHUNK_SIZE evenly, so tick_burn's
        // radius-1 call almost always lands here) wrote directly into the
        // worker's own field tile without clearing `fields_settled`,
        // currently masked only by the coincidence that a burning cell's
        // own tick_burn also writes its cell every frame it burns,
        // independently keeping the chunk (and therefore
        // active_chunk_count()) awake regardless. This isolates the flag
        // itself, checked immediately after one parallel sweep -- before
        // anything else (a later field::step call, say) could have cleared
        // it for an unrelated reason.
        let mut w = wide_world();
        w.end_step(); // promote the fresh world's initial full-dirty state so the field can actually reach "settled"
        // More than one step: the sky light source (architecture §2) takes
        // several frames of diffusion to reach a fixed point through the
        // world's full chunk depth -- see `field::an_impulse_wakes_an_
        // already_settled_field`'s own version of this reasoning.
        for _ in 0..500 {
            field::step(&mut w);
        }
        assert!(w.fields_settled(), "test setup should have started with a converged field");

        let mut burning = SimCell::new(material::OIL, 0);
        burning.ignite(9999);
        w.set(0, 0, burning);
        // `Chunk::mark_dirty` (which `set` triggers) only ever sets
        // `pending_dirty` -- promoting it to the `dirty` state the sweep
        // actually reads happens in `end_step`, which the *next* full
        // frame's own `step` call performs only *after* that frame's sweep
        // already ran against the old state. Promote it explicitly here,
        // the same as a real previous frame completing normally would
        // have, so the very next `step` call below actually visits (0, 0).
        w.end_step();

        step(&mut w); // one parallel CA sweep -- fire::update visits (0,0), tick_burn pushes heat via ChunkView::add_heat

        assert!(!w.fields_settled(), "a same-chunk heat push during the parallel sweep should have woken the settled field");
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

    /// The actual conserved quantity for a `Liquid` material under the
    /// compressible-volume model -- see `update::liquid_fill`'s own doc.
    /// `count` alone is the wrong invariant for one: a single full cell can
    /// split its fill across two cells (one still `Liquid`, one newly
    /// created from `Empty`), which correctly changes the *cell count*
    /// without creating or destroying any material at all.
    fn liquid_volume(w: &World, id: material::MaterialId) -> u64 {
        let b = w.bounds().unwrap();
        let mut total = 0u64;
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let cell = w.get(x, y);
                if cell.material == id {
                    total += crate::sim::update::liquid_fill(cell) as u64;
                }
            }
        }
        total
    }

    #[test]
    fn a_landing_column_does_not_balloon_in_cell_count() {
        // The regression this session actually needs guarded, after a
        // reordering fix (horizontal transfer tried before vertical, to
        // fix a *different*, still-open problem -- tall columns stalling
        // with a visible step at chunk boundaries, `Reports/liquid-
        // simulation-research-r2.md` §5's "wide bodies level in O(width²)")
        // was tried and reverted. Live report: three tall water columns
        // dropped onto short platforms would balloon out to nearly 5x
        // their cell count within a couple hundred frames before slowly
        // re-collapsing, while total fill stayed exactly conserved the
        // whole time (confirmed every single frame, never drifted) --
        // water is incompressible, so "same mass, far more cells" is
        // physically nonsensical even though it isn't a conservation bug.
        // Root cause: the old vertical-first order has a load-bearing side
        // effect that isn't obvious from reading it alone -- a deep,
        // blocked, full cell's vertical attempt only ever has
        // `LIQUID_MAX_COMPRESS` (1%) of genuine room, but that tiny
        // transfer still succeeds and returns early, which incidentally
        // throttles the cell out of horizontal transfer almost every
        // frame. Trying horizontal first (even only for cells with open
        // space directly above them, also tried) removes that throttle
        // broadly enough that a column's whole body leaks sideways within
        // the same few frames its base lands.
        let stone = material::STONE;
        let floor_y = 300;
        let mut w = World::new(Rect::new(0, 0, 511, floor_y + 10));
        for x in 0..512 {
            w.set(x, floor_y, SimCell::new(stone, 0));
        }
        for &(cx0, cx1) in &[(80, 110), (220, 250), (400, 430)] {
            for y in (floor_y - 260)..floor_y {
                for x in cx0..cx1 {
                    w.set(x, y, SimCell::new(material::WATER, 0));
                }
            }
        }
        fn water_cell_count(w: &World) -> usize {
            let b = w.bounds().unwrap();
            let mut n = 0;
            for y in b.min_y..=b.max_y {
                for x in b.min_x..=b.max_x {
                    if w.get(x, y).material == material::WATER {
                        n += 1;
                    }
                }
            }
            n
        }

        let start_cells = water_cell_count(&w);
        let start_fill = liquid_volume(&w, material::WATER);

        let mut max_cells = start_cells;
        for _ in 0..300 {
            step(&mut w);
            max_cells = max_cells.max(water_cell_count(&w));
            assert_eq!(
                liquid_volume(&w, material::WATER),
                start_fill,
                "total fill must stay exactly conserved even mid-collapse"
            );
        }
        assert!(
            max_cells < start_cells * 3 / 2,
            "cell count ballooned to {max_cells} against a start of {start_cells} \
             (an incompressible liquid shouldn't spread across several times \
             its own cell count just because a column landed)"
        );
    }

    /// Falling water must not tear a dark line along a horizontal chunk seam.
    ///
    /// Reported from live play: two thin dark horizontal lines lying exactly
    /// on chunk gridlines, in a large body of water that was still falling
    /// and spreading. They could be painted over and never appeared in
    /// settled water, which is the tell that this is material *in motion*
    /// rather than corruption.
    ///
    /// Cause: `try_move` marks a downward move `revisited = false` because
    /// rows sweep bottom to top and the sweep will not come back. True
    /// within one chunk, false at a horizontal seam whose lower chunk runs
    /// in a later pass — `step` runs its four groups in index order, so an
    /// even-`cy` chunk row always sweeps before the odd row beneath it, and
    /// a cell dropped across that seam is moved on *again* by the receiving
    /// chunk in the same frame. Two cells in one frame thins the body
    /// exactly at the seam. Fixed by `CellSurface::swept_after_me`.
    ///
    /// Measured as a **fill** deficit, not an occupancy one: the rows are
    /// not empty, they are uniformly low on fill, which `render.rs` draws as
    /// a dark line. An occupancy metric finds nothing here — the same trap
    /// that cost three reproductions in `update.rs`'s `seam_terracing`.
    ///
    /// The serial driver is the control: its chunk order is already
    /// bottom-up, so it never had this and must keep not having it.
    /// Fixed by ordering the sweep's passes bottom chunk row first, rather
    /// than by penalising the crossing cell -- see `step`.
    ///
    /// Two earlier attempts marked the crossing cell `revisited` so the
    /// receiving chunk would not move it twice. Both cleared these rows and
    /// both were reverted (`e816477`), because they replaced the tear with a
    /// throttle at the same seam: every crossing cell now waited a frame.
    /// Measured as a summed row-fill deficit of 2236 asking about every
    /// cross-chunk move and 1948 asking only about downward ones, against
    /// 988 for correct ordering -- see `liquid_acceptance`'s banding bar in
    /// `update.rs`, which is what caught the second attempt before it
    /// shipped.
    #[test]
    fn falling_water_does_not_tear_a_line_along_a_horizontal_chunk_seam() {
        const W: i32 = 512;
        const H: i32 = 320;
        const FLOOR: i32 = H - 8;

        let scene = || {
            let mut w = World::new(Rect::new(0, 0, W - 1, H - 1));
            for x in 0..W {
                for y in FLOOR..H {
                    w.set(x, y, SimCell::new(material::STONE, 0));
                }
            }
            for x in 20..492 {
                for y in 40..FLOOR {
                    w.set(x, y, SimCell::new(material::WATER, 0));
                }
            }
            w
        };

        // Mean fill per row across the body, on the `LIQUID_FULL` scale.
        let row_fill = |w: &World, y: i32| {
            let total: u32 = (20..492)
                .map(|x| {
                    let c = w.get(x, y);
                    if c.material == material::WATER {
                        crate::sim::update::liquid_fill(c) as u32
                    } else {
                        0
                    }
                })
                .sum();
            (total / 472) as i32
        };
        // Rows markedly darker than both neighbours, while inside the body.
        let dark_seam_rows = |w: &World| {
            (41..FLOOR - 1)
                .filter(|&y| {
                    let neighbours = (row_fill(w, y - 1) + row_fill(w, y + 1)) / 2;
                    neighbours > 300 && row_fill(w, y) < neighbours * 3 / 4 && y.rem_euclid(CHUNK_SIZE) == 0
                })
                .count()
        };

        let mut parallel_world = scene();
        let mut serial_world = scene();
        let (mut parallel_worst, mut serial_worst) = (0, 0);
        for frame in 1..=600 {
            step(&mut parallel_world);
            update::step(&mut serial_world);
            if frame % 20 == 0 {
                parallel_worst = parallel_worst.max(dark_seam_rows(&parallel_world));
                serial_worst = serial_worst.max(dark_seam_rows(&serial_world));
            }
        }

        assert_eq!(serial_worst, 0, "the serial driver should never tear a seam row -- its chunk order is bottom-up");
        assert_eq!(
            parallel_worst, 0,
            "falling water is thinning out exactly on horizontal chunk seams under the parallel sweep \
             (before the fix: 2 such rows, short by 427/1000 and 333/1000 of full fill)"
        );
    }
}
