//! Fixed-size tiles of the world, and the coordinate maths that maps global
//! world coordinates onto them.
//!
//! 64x64 is a deliberate compromise: small enough that a dirty rectangle
//! usefully narrows the work, large enough that per-chunk dispatch overhead
//! stays negligible once M5 spreads chunks across threads.

use super::cell::Cell;
use super::rng::Rng;

pub const CHUNK_SIZE: i32 = 64;
pub const CHUNK_AREA: usize = (CHUNK_SIZE * CHUNK_SIZE) as usize;

/// The furthest any movement rule, for any material, may ever look sideways
/// from the cell it is deciding about. Every rule that reads sideways caps
/// itself at this independently — a powder's roll (`Material::roll_reach_at`,
/// itself clamped when derived from `friction_angle`), a liquid's levelling
/// search (`HORIZONTAL_TRANSFER_REACH`, 8) — so this is a hard outer bound
/// on all of them, not a value any single rule normally reaches. **Gas is
/// the exception**: `flow_sideways`'s free-surface branch (`update.rs`'s
/// `SURFACE_SEARCH`, itself defined as `= MAX_REACH`) means a gas cell's
/// true reach is `dispersion + MAX_REACH`, clamped straight back down to
/// exactly `MAX_REACH` — a gas cell's own worst case genuinely does reach
/// this bound, not just approach it (see `Material::sweep_reach`'s `Gas`
/// arm, which had this wrong in this section's first draft — a real bug an
/// independent review caught, not merely a hypothetical one this comment is
/// warning against).
///
/// Two things this bound feeds, kept in sync deliberately:
///
/// - `parallel.rs`'s cross-chunk write-safety proof, which needs an exact
///   value (`CHUNK_SIZE / 2`) to reason about how far a single write can
///   ever land, and stays keyed on this constant regardless of what any
///   individual material actually uses.
/// - `Material::sweep_reach`'s own cap — the per-chunk value
///   `Chunk::sweep_region` actually widens by (issue #3) is smaller than
///   this for `Powder`/`Liquid`-only chunks, since it tracks only what a
///   chunk's *resident* materials need rather than the theoretical worst
///   case across every material in the registry — but a chunk with any
///   resident `Gas` cell gets no narrowing at all, correctly, since nothing
///   smaller than `MAX_REACH` would be safe for one.
pub const MAX_REACH: i32 = 32;

/// Address of a chunk in the chunk grid. Signed, because the world extends in
/// every direction once streaming arrives in M10.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    /// Reserved for M10's worldgen redesign (`Reports/worldgen-design.md`
    /// §0, issue #11): a generic slice identifier for which 2D vertical
    /// slice through the 3D coarse worldgen layer this chunk belongs to.
    /// Always `0` today — the play world is one slice, and every
    /// `ChunkCoord` is built through `new`/`containing` below, both of
    /// which hardcode it, so nothing outside this file needed to change to
    /// add this field. Deliberately not named `z`: a straight slice wants
    /// a z-coordinate, but a route following a drainage network (the
    /// currently open, still-undecided option in the worldgen report)
    /// wants a route id instead, and a bare `slice` covers either without
    /// committing to which. Reserved now rather than later because
    /// `ChunkCoord` is the `HashMap` key for both `World::chunks` and
    /// `World::fields` and will reach the save format once M10 lands —
    /// adding a field to an already-shipped save format is a migration and
    /// a compatibility break; adding one now, always zero, costs nothing.
    pub slice: u32,
}

impl ChunkCoord {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y, slice: 0 }
    }

    /// The chunk containing a world position.
    ///
    /// `div_euclid` rather than `/` is load-bearing: truncating division maps
    /// both -1 and 0 to chunk 0, which folds the row just below the origin onto
    /// the wrong chunk. Euclidean division floors toward negative infinity.
    #[inline]
    pub fn containing(x: i32, y: i32) -> Self {
        Self {
            x: x.div_euclid(CHUNK_SIZE),
            y: y.div_euclid(CHUNK_SIZE),
            slice: 0,
        }
    }

    /// World coordinate of this chunk's top-left cell.
    #[inline]
    pub fn origin(self) -> (i32, i32) {
        (self.x * CHUNK_SIZE, self.y * CHUNK_SIZE)
    }

    /// The world-space region this chunk covers, inclusive on both ends.
    pub fn bounds(self) -> Rect {
        let (ox, oy) = self.origin();
        Rect::new(ox, oy, ox + CHUNK_SIZE - 1, oy + CHUNK_SIZE - 1)
    }
}

/// Index of a world position within its chunk's cell array.
///
/// Uses `rem_euclid` for the same reason `containing` uses `div_euclid`:
/// `-1 % 64` is -1, which would index out of bounds.
#[inline]
pub fn local_index(x: i32, y: i32) -> usize {
    let lx = x.rem_euclid(CHUNK_SIZE);
    let ly = y.rem_euclid(CHUNK_SIZE);
    (ly * CHUNK_SIZE + lx) as usize
}

/// An inclusive rectangle in world coordinates.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rect {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl Rect {
    pub fn new(min_x: i32, min_y: i32, max_x: i32, max_y: i32) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    pub fn point(x: i32, y: i32) -> Self {
        Self::new(x, y, x, y)
    }

    pub fn width(self) -> i32 {
        self.max_x - self.min_x + 1
    }

    pub fn height(self) -> i32 {
        self.max_y - self.min_y + 1
    }

    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }

    /// Grow to cover a point.
    pub fn include(&mut self, x: i32, y: i32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    /// Grow by `n` cells on every side.
    pub fn expanded(self, n: i32) -> Self {
        self.expanded_xy(n, n)
    }

    /// Grow by `dx` horizontally and `dy` vertically.
    ///
    /// The two differ because movement rules are not symmetric: they read one
    /// row up or down but scan many cells sideways, so the sweep region has to
    /// stretch much further horizontally than vertically.
    pub fn expanded_xy(self, dx: i32, dy: i32) -> Self {
        Self::new(
            self.min_x - dx,
            self.min_y - dy,
            self.max_x + dx,
            self.max_y + dy,
        )
    }

    /// The smallest rect containing both.
    pub fn union(self, other: Rect) -> Self {
        Self::new(
            self.min_x.min(other.min_x),
            self.min_y.min(other.min_y),
            self.max_x.max(other.max_x),
            self.max_y.max(other.max_y),
        )
    }

    /// The overlapping region, or `None` when the two are disjoint.
    pub fn intersection(self, other: Rect) -> Option<Rect> {
        let r = Self::new(
            self.min_x.max(other.min_x),
            self.min_y.max(other.min_y),
            self.max_x.min(other.max_x),
            self.max_y.min(other.max_y),
        );
        if r.min_x > r.max_x || r.min_y > r.max_y {
            None
        } else {
            Some(r)
        }
    }
}

#[derive(Clone)]
pub struct Chunk {
    pub coord: ChunkCoord,
    /// Boxed: 64*64*4 = 16 KB is far too large to move around by value.
    cells: Box<[Cell]>,
    /// Region that changed during the previous sweep — the region the next
    /// sweep must examine. M4 uses this to skip settled chunks entirely.
    dirty: Option<Rect>,
    /// Region changed so far during the sweep currently in flight. Kept separate
    /// from `dirty` so that writes made *while* sweeping do not extend the
    /// region being swept, which would let material fall further than one cell
    /// per frame. Promoted to `dirty` by `end_sweep`.
    pending_dirty: Option<Rect>,
    /// This chunk's own RNG stream (M5). The parallel checkerboard sweep
    /// gives each active chunk exclusive ownership of itself for a pass, so
    /// movement tie-breaks and fire's ignition/reaction rolls draw from here
    /// rather than a single generator shared across threads — no
    /// synchronization needed, and none of this engine's randomness was ever
    /// required to be reproducible (see the plan's determinism decision), so
    /// a per-chunk stream costs nothing behaviourally that a shared one
    /// bought. `World` keeps its own separate `Rng` for everything outside
    /// the sweep — painting, explosions, particle bursts.
    rng: Rng,
    /// How far sideways `sweep_region` widens a dirty rectangle by (issue
    /// #3) — the furthest any material currently resident in this chunk can
    /// reach, per `Material::sweep_reach`. Grows
    /// immediately on every write via `set_world` (cheap, and never unsafe
    /// to widen early — a too-large reach only costs a few extra stale
    /// cells re-examined). Only shrinks via `recompute_reach`, a full scan
    /// that is deliberately *not* run on every write: `World::end_step`
    /// calls it exactly when a chunk transitions from active to settled,
    /// the one point a smaller value is both cheap to discover (nothing is
    /// mid-sweep) and safe to adopt (nothing needs the wider, possibly
    /// stale value until this chunk wakes again, at which point growth
    /// via `set_world` takes back over). Starts at 1, the same floor every
    /// material's own reach is clamped to — even an all-`Empty`/`Solid`
    /// chunk still needs its one immediate neighbour re-examined when
    /// something adjacent changes.
    reach: i32,
    /// **The same dirty marks, one x-span per row instead of one rect for
    /// the whole chunk** — and the reason the sweep is a third of the size it
    /// used to be.
    ///
    /// `dirty` is a *bounding box*, so two writes at opposite corners of a
    /// chunk dirty everything between them. Measured in the evolution lab
    /// 2026-09-01 (`Reports/evolution-lab-frame-cost-2026-09-01.md`): a
    /// settled bed with eight plants in it changes **447 cells a tick** and
    /// sweeps **45,442** for them, a ratio of 102 to 1, because the plants'
    /// roots keep soil moisture moving in scattered single cells all over the
    /// root zone and every one of them widens a box.
    ///
    /// These spans are the union of the marks' own neighbourhoods rather than
    /// their bounding box, so they are **strictly a subset of `dirty` and
    /// still a superset of every cell the sweep can act on** — the invariant
    /// is unchanged, only its shape. **Off by default** —
    /// `PIXEL_PHYSICS_SWEEP=rows` turns them on; see [`row_spans_enabled`]
    /// for the measurement and for why a strictly-tighter region is still a
    /// behaviour change here.
    ///
    /// Indexed by **local row + 1**, so index 0 is the row above the chunk
    /// and index `CHUNK_SIZE + 1` the row below: a mark one row outside still
    /// constrains the chunk's own edge row, exactly as `expanded_xy(_, 1)`
    /// does for the box. Values are **world** x, unclipped — `reach` is not
    /// known when a mark is made and can grow afterwards, so clipping happens
    /// where the box's does, in `sweep_plan`. A span with `min > max` is
    /// empty, which is how a row says "nothing here".
    dirty_rows: [(i16, i16); SPAN_ROWS],
    pending_rows: [(i16, i16); SPAN_ROWS],
    /// **A second dirty channel, for soil moisture only** — and the reason
    /// a bed of plants no longer keeps the whole box awake.
    ///
    /// Moisture transport is not a *movement*. A wetness change needs its
    /// own cell and the four it exchanges with reconsidered, and nothing
    /// else — not the `reach`-wide movement neighbourhood, and not a chunk
    /// held awake for the CA sweep to walk and for `field::step` to solve
    /// against. Measured before this existed
    /// (`Reports/evolution-lab-frame-cost-2026-09-01.md`): **410 of the 447
    /// cells that change per tick in a settled lab bed are soil wetness**,
    /// every one of them marked its chunk dirty on the ordinary channel, and
    /// between the sweep and the field that was 63% of the tick.
    ///
    /// So moisture writes go through [`World::set_soil_moisture`], which
    /// writes quietly, marks the chunk for the *renderer* (wet soil is a
    /// different colour) and marks it here. `World::step_soil_water` is the
    /// only reader. Seeded from the ordinary channel in [`Self::end_sweep`],
    /// so every other writer in the engine — rain, a root drinking, a
    /// painted bucket, a landing particle — reaches it for free and with no
    /// hot-path cost.
    ///
    /// Single-buffered rather than promoted like `dirty`, because
    /// `take_moist_plan` snapshots and clears in one move at the top of the
    /// pass — the same two-phase `parallel::step` uses on `chunks_to_sweep`,
    /// and for the same reason: a write made during the pass must land in
    /// the *next* pass's set, not grow the one being walked.
    pending_moist_rows: [(i16, i16); SPAN_ROWS],
    /// Whether this chunk currently holds any `Liquid`-kind cell.
    ///
    /// Tracked exactly like `reach` above and for the same reasons: grown
    /// for free on every write (`set_world`), and only ever *shrunk* by a
    /// full scan (`recompute_has_liquid`) at the one moment that is both
    /// cheap and safe, when the chunk settles. A stale `true` costs one
    /// pointless chunk redraw; a stale `false` would drop an animation
    /// frame, which is why it is never cleared except by that scan.
    ///
    /// Read only by `render.rs`, to redraw just the chunks holding liquid on
    /// the frames an animated `GrainMode` steps — the whole screen was being
    /// redrawn, which measured at ~10 ms on a settled world.
    has_liquid: bool,
}

/// Rows a chunk keeps a dirty span for: its own, plus one either side. See
/// [`Chunk::dirty_rows`].
const SPAN_ROWS: usize = CHUNK_SIZE as usize + 2;

/// An empty span. `min > max`, so every consumer's `min <= max` test rejects
/// it without a second flag.
const NO_SPAN: (i16, i16) = (i16::MAX, i16::MIN);

/// Spans store **world** x in an `i16`, so a world wider than this would
/// silently wrap them. 32,767 cells is 64x the lab bed and 4x the outdoor
/// world's 8,192; the assertion is here so M10's streaming does not discover
/// it as a wrong-cells bug.
const SPAN_MAX_WORLD: i32 = i16::MAX as i32;

/// Every row of a chunk marked dirty end to end — what `new` and `wake` mean
/// by "the whole chunk".
fn full_rows(coord: ChunkCoord) -> [(i16, i16); SPAN_ROWS] {
    let b = coord.bounds();
    let mut rows = [NO_SPAN; SPAN_ROWS];
    // Only the chunk's own rows: the two outriggers exist to carry marks made
    // *outside* the chunk, and waking a chunk makes none.
    for r in rows.iter_mut().skip(1).take(CHUNK_SIZE as usize) {
        *r = (b.min_x as i16, b.max_x as i16);
    }
    rows
}

/// Whether the sweep uses the per-row spans or the old bounding box.
/// `PIXEL_PHYSICS_SWEEP=rows` turns the spans on; anything else is the
/// bounding box the engine has always used.
///
/// **An A/B inside one binary**, the shape `CLAUDE.md` asks for whenever two
/// arms have to be compared on a box that is not quiet: `crumb_rule`'s own
/// reasoning, and the reason the `relax_region` night ended in a measurement
/// rather than an argument.
///
/// # Why this is off by default, which is the part to read before turning it on
///
/// **These spans change the world, and not only through the RNG. That was
/// measured 2026-09-05 and it is the opposite of what this comment used to
/// say**, so read this paragraph before reasoning from anything downstream of
/// it.
///
/// The original argument was: the sweep's draws are consumed per *visited*
/// cell, so narrowing the region shifts the per-chunk `Rng` stream, and the
/// unlock is therefore to seed the draw from position and frame instead. That
/// was built (`rng::sweep`, `surface::VisitRng`,
/// `PIXEL_PHYSICS_RNG=positional`) and it settles the question the other way:
///
/// - **The premise is false.** With the draw keyed on position and frame, so
///   that visit order cannot reach it at all, box and rows **still diverge** —
///   identical through frame 4,329 and first differing at **frame 4,330** on
///   the standard lab bed, bisected to the frame, every arm's hash
///   reproducing exactly across three reps. So the spans are *not* a superset
///   of every cell the rules can act on, the RNG was one coupling among at
///   least two, and the second one is unidentified. Filed as **§E2** in
///   `Reports/open-bugs-handoff.md` with the reproduction; the leading
///   unmeasured hypothesis is chunk wakefulness feeding `field::step`'s
///   `active_chunk_count()` gate.
/// - **And the economics are gone anyway.** The positional draw costs
///   **+0.149 ms/tick**, about the whole of what the spans save, so the two
///   together measured 2.631 -> 2.636 ms with overlapping ranges. The "worth
///   up to 3x of this phase" this comment used to quote came from `labperf`'s
///   `est_` columns, which are ~90% soil moisture and — since moisture got its
///   own dirty channel — do not wake the sweep at all, so they fail that
///   instrument's own stated control.
///
/// One factual correction while it is in view, because three documents
/// inherited it from here: `update_liquid` and `update_powder` do **not** each
/// "open with `surface.rng().flip()` on every visit". `update_powder` begins
/// at `update.rs:701` and its flip is at `:966` behind five returns;
/// `update_liquid`'s is at `:1352`, after a straight-down `try_move` that
/// returns at `:1349`. Neither runs at all for `Empty`, `Solid`, `Plant` or
/// `Creature`. The draws are still consumed in visit order, which is why the
/// stream shift was real; there are just far fewer of them than the old
/// wording implies.
///
/// Measured 2026-09-01 in the evolution lab, paired, three runs a side: the
/// CA sweep **3.67 -> 2.67 ms** and the whole tick **6.51 -> 5.49 ms**. Both
/// numbers are stale as a *frame* argument — re-measured 2026-09-05 the same
/// spans are 1.19x on the phase and **1.05x on the tick**, because everything
/// around them got cheaper. Which guards go red also reshuffles between
/// builds: it was `frame_step_matches_the_sequence_app_update_ran_before_
/// extraction` plus the determinacy guard, and is now that hash plus
/// `a_spread_leaf_cluster_is_longer_than_a_blob`.
///
/// **So this stays off, and the reason is no longer "it costs a stream
/// shift" — it is that narrowing the region is a behaviour change nobody has
/// explained.** Full account in
/// `Reports/sweep-positional-rng-2026-09-05.md`, which supersedes
/// `Reports/evolution-lab-frame-cost-2026-09-01.md` §5 on every point above.
///
/// Read once per process through a `OnceLock` and consulted once per chunk
/// per pass, never per cell.
fn row_spans_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("PIXEL_PHYSICS_SWEEP").as_deref() == Ok("rows"))
}

/// One chunk's sweep, row by row.
///
/// Carries the bounding box **as well as** the spans, deliberately: the box
/// is what `parallel.rs`'s write-disjointness proof is stated against and
/// what every existing caller of `sweep_region` reads, and it must stay the
/// same rect it always was. The spans only ever narrow what is walked
/// *inside* it.
#[derive(Clone, Copy)]
pub struct SweepPlan {
    /// Exactly `Chunk::sweep_region`'s rect.
    pub bounds: Rect,
    /// Per row of `bounds`, the inclusive world-x span to visit. Indexed by
    /// `y - bounds.min_y`.
    rows: [(i16, i16); SPAN_ROWS],
}

impl SweepPlan {
    /// The x span to sweep on world row `y`, or `None` when that row has
    /// nothing to do. Rows outside `bounds` always answer `None`.
    #[inline]
    pub fn row(&self, y: i32) -> Option<(i32, i32)> {
        if y < self.bounds.min_y || y > self.bounds.max_y {
            return None;
        }
        let i = (y - self.bounds.min_y) as usize;
        let (min, max) = self.rows[i];
        if min > max {
            return None;
        }
        Some((min as i32, max as i32))
    }
}

impl Chunk {
    pub fn new(coord: ChunkCoord) -> Self {
        Self {
            coord,
            cells: vec![Cell::EMPTY; CHUNK_AREA].into_boxed_slice(),
            // A new chunk is considered fully dirty so its contents get one
            // sweep; generated terrain may need to settle immediately.
            dirty: Some(coord.bounds()),
            pending_dirty: None,
            // A new chunk is fully dirty on both representations, or the
            // spans would narrow away the one sweep generated terrain needs.
            dirty_rows: full_rows(coord),
            pending_rows: [NO_SPAN; SPAN_ROWS],
            // Fully moist-dirty for the same reason it is fully dirty:
            // generated terrain arrives with whatever wetness worldgen gave
            // it and needs one pass to find its own equilibrium.
            pending_moist_rows: full_rows(coord),
            rng: Rng::new(seed_from_coord(coord)),
            reach: 1,
            has_liquid: false,
        }
    }

    #[inline]
    pub fn rng_mut(&mut self) -> &mut Rng {
        &mut self.rng
    }

    #[inline]
    pub fn get_world(&self, x: i32, y: i32) -> Cell {
        self.cells[local_index(x, y)]
    }

    /// `reach` is the new cell's material's own `Material::sweep_reach`
    /// (the caller's job to look up, since `Chunk` has no `MaterialRegistry`
    /// access by design — only ever an opaque `MaterialId`). Only ever grows
    /// `self.reach`; see the field's own doc for why shrinking is handled
    /// separately, in `recompute_reach`.
    #[inline]
    pub fn set_world(&mut self, x: i32, y: i32, cell: Cell, reach: i32, is_liquid: bool) {
        self.cells[local_index(x, y)] = cell;
        self.mark_dirty(x, y);
        self.reach = self.reach.max(reach);
        self.has_liquid |= is_liquid;
    }

    /// See the field's own doc. Read by `render.rs`.
    #[inline]
    pub fn has_liquid(&self) -> bool {
        self.has_liquid
    }

    /// The shrinking counterpart to `set_world`'s growth, run at the same
    /// active-to-settled moment `recompute_reach` is and for the same
    /// reason -- a chunk whose last liquid cell drained away has no way to
    /// discover that from writes alone.
    pub fn recompute_has_liquid(&mut self, is_liquid: impl Fn(Cell) -> bool) {
        self.has_liquid = self.cells.iter().any(|&cell| is_liquid(cell));
    }

    /// Write without marking the chunk dirty.
    ///
    /// Exists solely for parity bookkeeping. The sweep has to stamp every
    /// mobile cell it visits — including ones that did not move — but if that
    /// stamp dirtied the chunk, every chunk holding a grain of sand would stay
    /// awake forever and sleeping would never save anything.
    #[inline]
    pub fn set_world_quiet(&mut self, x: i32, y: i32, cell: Cell) {
        self.cells[local_index(x, y)] = cell;
    }

    /// Record that a cell changed, so the next sweep looks at it and its
    /// neighbours. Cheap enough to call on every write.
    #[inline]
    pub fn mark_dirty(&mut self, x: i32, y: i32) {
        match &mut self.pending_dirty {
            Some(r) => r.include(x, y),
            None => self.pending_dirty = Some(Rect::point(x, y)),
        }
        // The same mark, kept per row. A mark more than one row outside the
        // chunk cannot reach it — the box expands by exactly one row — so it
        // is dropped here rather than clamped, which is the whole of the
        // narrowing in the vertical direction.
        let ly = y - self.coord.bounds().min_y;
        if !(-1..=CHUNK_SIZE).contains(&ly) {
            return;
        }
        let span = &mut self.pending_rows[(ly + 1) as usize];
        let x = x.clamp(-SPAN_MAX_WORLD, SPAN_MAX_WORLD) as i16;
        span.0 = span.0.min(x);
        span.1 = span.1.max(x);
    }

    /// The furthest any material resident in this chunk can move sideways in
    /// one tick, capped at `MAX_REACH`. Exposed because it is the multiplier
    /// on [`Self::sweep_region`]'s horizontal expansion, and therefore half
    /// of the answer to "why is this chunk sweeping 1,600 cells for twenty
    /// writes" — a question no instrument could ask from outside before.
    #[inline]
    pub fn reach(&self) -> i32 {
        self.reach
    }

    /// The region this sweep should examine, clipped to the chunk.
    ///
    /// Widened around what changed, because a cell must be reconsidered
    /// whenever anything it can see has moved — `self.reach` sideways (issue
    /// #3: the furthest any resident material can actually reach, capped at
    /// `MAX_REACH`, rather than unconditionally `MAX_REACH` itself), since
    /// powders roll and liquids flow along a row, and one cell vertically,
    /// which is as far as any rule looks up or down.
    pub fn sweep_region(&self) -> Option<Rect> {
        self.dirty?
            .expanded_xy(self.reach, 1)
            .intersection(self.coord.bounds())
    }

    /// The same region as [`Self::sweep_region`], with the per-row spans that
    /// narrow it.
    ///
    /// **The bounding rect is unchanged and is still the answer to "could
    /// this chunk write anywhere outside itself"** — every safety argument in
    /// `parallel.rs` rests on that rect and none of them is weakened here.
    /// What the spans change is only which cells inside it are walked.
    ///
    /// Each row's span is the union of the marks on that row and the two
    /// beside it, expanded by `reach` sideways and clipped to the chunk. That
    /// is exactly `expanded_xy(reach, 1)` applied per mark instead of to
    /// their bounding box — the same rule, not a looser one — so a cell is
    /// visited iff something within its own reach changed, which is the
    /// contract `sweep_region`'s doc states.
    pub fn sweep_plan(&self) -> Option<SweepPlan> {
        let bounds = self.sweep_region()?;
        let mut rows = [NO_SPAN; SPAN_ROWS];
        if !row_spans_enabled() {
            // The old behaviour, exactly: every row of the box, end to end.
            for r in rows.iter_mut().take((bounds.max_y - bounds.min_y + 1) as usize) {
                *r = (bounds.min_x as i16, bounds.max_x as i16);
            }
            return Some(SweepPlan { bounds, rows });
        }
        let reach = self.reach;
        let chunk = self.coord.bounds();
        for y in bounds.min_y..=bounds.max_y {
            let ly = y - chunk.min_y;
            // The marks on this row and the two beside it — the vertical
            // half of the box's `expanded_xy(_, 1)`.
            let mut min = i32::MAX;
            let mut max = i32::MIN;
            for d in -1..=1 {
                let i = ly + d + 1;
                if !(0..SPAN_ROWS as i32).contains(&i) {
                    continue;
                }
                let (lo, hi) = self.dirty_rows[i as usize];
                if lo <= hi {
                    min = min.min(lo as i32);
                    max = max.max(hi as i32);
                }
            }
            if min > max {
                continue;
            }
            let lo = (min - reach).max(bounds.min_x);
            let hi = (max + reach).min(bounds.max_x);
            if lo <= hi {
                rows[(y - bounds.min_y) as usize] = (lo as i16, hi as i16);
            }
        }
        Some(SweepPlan { bounds, rows })
    }

    /// True when this chunk has nothing to sweep and can be skipped.
    ///
    /// Deliberately defined as "`sweep_region` has nothing to give me", not
    /// the narrower "`dirty` is empty" it used to be. Those differ in one
    /// real case: `World::touch_neighbours` wakes a neighbour by marking it
    /// dirty at the position of the *write*, which by construction lies
    /// outside that neighbour, and `sweep_region` then expands by the
    /// chunk's own `reach` and clips to its own bounds — so a chunk whose
    /// reach is too short to drag that point back inside gets an empty
    /// region. Under the old definition such a chunk counted as awake
    /// forever while never actually being swept: a chunk that could not
    /// possibly have anything to do, keeping the world from ever sleeping
    /// and re-marked by its neighbour's ongoing activity every frame.
    /// Measured on the seam-cliff scene (`update.rs`'s `seam_cliffs`): 3
    /// such chunks under the parallel driver at frame 400.
    ///
    /// The alternative — clamping `(x, y)` into the chunk's own bounds
    /// inside `mark_dirty` — was tried and reverted. It works, but it throws
    /// away the issue #3 optimization that
    /// `neighbour_waking_stops_at_the_neighbours_own_reach` (`world.rs`)
    /// documents as deliberate: an empty neighbour chunk should not pay for
    /// a wide, pointless sweep just because something moved far away next
    /// door. Answering the question from `sweep_region` keeps that and fixes
    /// this, because the two questions were the same question all along.
    ///
    /// Self-healing rather than merely masked: the stale `dirty` rect that
    /// produced the empty region is replaced by the next `end_sweep`, so
    /// nothing accumulates. And a chunk cannot go from settled to awake
    /// without something writing into it, which grows `reach` and marks it
    /// dirty in-bounds through `set_world` — so this can never report
    /// settled for a chunk that genuinely has work.
    pub fn is_settled(&self) -> bool {
        self.sweep_region().is_none()
    }

    /// Promote writes made during this sweep into the region for the next one.
    pub fn end_sweep(&mut self) {
        // **Every ordinary write seeds the moisture channel**, and this one
        // line is the whole of that plumbing. A moisture pass has to
        // reconsider a cell whenever anything near it changed — rain landing,
        // a root drinking, a grain falling into a puddle — and every one of
        // those goes through `mark_dirty`. Unioning here rather than in
        // `mark_dirty` keeps the hottest write path untouched, and cannot
        // feed back on itself: `World::set_soil_moisture` is quiet on the
        // ordinary channel, so a moisture write never appears in
        // `pending_rows`.
        for (moist, dirty) in self.pending_moist_rows.iter_mut().zip(self.pending_rows.iter()) {
            if dirty.0 <= dirty.1 {
                moist.0 = moist.0.min(dirty.0);
                moist.1 = moist.1.max(dirty.1);
            }
        }
        self.dirty = self.pending_dirty.take();
        self.dirty_rows = std::mem::replace(&mut self.pending_rows, [NO_SPAN; SPAN_ROWS]);
    }

    /// Mark a cell for the next soil-moisture pass. See
    /// [`Self::pending_moist_rows`].
    #[inline]
    pub fn mark_moist_dirty(&mut self, x: i32, y: i32) {
        let ly = y - self.coord.bounds().min_y;
        if !(-1..=CHUNK_SIZE).contains(&ly) {
            return;
        }
        let span = &mut self.pending_moist_rows[(ly + 1) as usize];
        let x = x.clamp(-SPAN_MAX_WORLD, SPAN_MAX_WORLD) as i16;
        span.0 = span.0.min(x);
        span.1 = span.1.max(x);
    }

    /// **What the next soil-moisture pass must walk, taken and cleared in
    /// one move.**
    ///
    /// The horizontal expansion is **1**, not `reach`, and that is the whole
    /// reason this channel is cheap: `update_soil_water` reads and writes
    /// its own cell, the four it shares a face with, and the one below —
    /// so a write at `p` can only change the outcome for cells one step from
    /// `p`. The `reach` the movement sweep expands by (14 in a lab bed, up to
    /// `MAX_REACH`) is about what can *flow into* a cell, and no amount of
    /// water flows sideways through soil in one tick.
    ///
    /// Clearing here rather than promoting at the end is deliberate: writes
    /// made *during* the pass must land in the next pass's set rather than
    /// growing the one being walked, which is the same two-phase
    /// `parallel::step` applies to `chunks_to_sweep`.
    pub fn take_moist_plan(&mut self) -> Option<SweepPlan> {
        let marks = std::mem::replace(&mut self.pending_moist_rows, [NO_SPAN; SPAN_ROWS]);
        let chunk = self.coord.bounds();
        let mut rows = [NO_SPAN; SPAN_ROWS];
        let (mut min_y, mut max_y) = (i32::MAX, i32::MIN);
        let (mut min_x, mut max_x) = (i32::MAX, i32::MIN);
        for ly in 0..CHUNK_SIZE {
            let (mut lo, mut hi) = (i32::MAX, i32::MIN);
            for d in -1..=1 {
                let i = ly + d + 1;
                if !(0..SPAN_ROWS as i32).contains(&i) {
                    continue;
                }
                let (a, b) = marks[i as usize];
                if a <= b {
                    lo = lo.min(a as i32);
                    hi = hi.max(b as i32);
                }
            }
            if lo > hi {
                continue;
            }
            let lo = (lo - 1).max(chunk.min_x);
            let hi = (hi + 1).min(chunk.max_x);
            if lo > hi {
                continue;
            }
            let y = chunk.min_y + ly;
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            min_x = min_x.min(lo);
            max_x = max_x.max(hi);
            rows[ly as usize] = (lo as i16, hi as i16);
        }
        if min_y > max_y {
            return None;
        }
        // Re-index the spans against `bounds.min_y`, which is what
        // `SweepPlan::row` looks them up by.
        let mut shifted = [NO_SPAN; SPAN_ROWS];
        for (i, slot) in shifted.iter_mut().enumerate() {
            let ly = min_y - chunk.min_y + i as i32;
            if (0..CHUNK_SIZE).contains(&ly) {
                *slot = rows[ly as usize];
            }
        }
        Some(SweepPlan { bounds: Rect::new(min_x, min_y, max_x, max_y), rows: shifted })
    }

    /// Recompute `reach` from scratch by scanning every resident cell —
    /// the only way it can *shrink*, since `set_world` only ever grows it.
    /// `reach_of` is supplied by the caller (`World::end_step`, the one
    /// place with `MaterialRegistry` access) rather than looked up here,
    /// keeping `Chunk` itself free of any dependency on `material.rs` beyond
    /// the opaque `MaterialId` already inside every `Cell`.
    pub fn recompute_reach(&mut self, reach_of: impl Fn(Cell) -> i32) {
        // `self.cells` always holds exactly `CHUNK_AREA` elements (`Chunk::
        // new` fills it up front and nothing ever shrinks it), so `.max()`
        // over a non-empty iterator can never actually be `None` -- the
        // `.max(1)` alone is what enforces the floor.
        self.reach = self.cells.iter().map(|&cell| reach_of(cell)).max().expect("cells is never empty").max(1);
    }

    /// Force the whole chunk to be examined on the next sweep.
    pub fn wake(&mut self) {
        self.dirty = Some(self.coord.bounds());
        self.dirty_rows = full_rows(self.coord);
        self.pending_moist_rows = full_rows(self.coord);
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }
}

/// Deterministic-in-value but not required to be so — only used so different
/// chunks don't share an RNG stream. Casting through `u32` before widening to
/// `u64` gives a stable bit pattern for negative coordinates without relying
/// on `as u64`'s sign-extension behaviour being what a reader expects.
fn seed_from_coord(coord: ChunkCoord) -> u64 {
    let x = (coord.x as u32) as u64;
    let y = (coord.y as u32) as u64;
    x.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ y.wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::material;

    #[test]
    fn slice_defaults_to_zero_through_every_constructor() {
        // Issue #11: reserved for M10's worldgen redesign, always zero
        // today. Both constructors are checked directly (not inferred from
        // the containing/new equality test below, which would pass even if
        // both happened to agree on some other stray value).
        assert_eq!(ChunkCoord::new(3, -7).slice, 0);
        assert_eq!(ChunkCoord::containing(200, -5).slice, 0);
    }

    #[test]
    fn chunk_coords_floor_toward_negative_infinity() {
        assert_eq!(ChunkCoord::containing(0, 0), ChunkCoord::new(0, 0));
        assert_eq!(ChunkCoord::containing(63, 63), ChunkCoord::new(0, 0));
        assert_eq!(ChunkCoord::containing(64, 64), ChunkCoord::new(1, 1));
        // The case truncating division gets wrong.
        assert_eq!(ChunkCoord::containing(-1, -1), ChunkCoord::new(-1, -1));
        assert_eq!(ChunkCoord::containing(-64, -64), ChunkCoord::new(-1, -1));
        assert_eq!(ChunkCoord::containing(-65, -65), ChunkCoord::new(-2, -2));
    }

    #[test]
    fn local_index_stays_in_range_across_the_origin() {
        for x in -200..200 {
            for y in -200..200 {
                assert!(local_index(x, y) < CHUNK_AREA, "({x}, {y}) escaped");
            }
        }
    }

    #[test]
    fn neighbouring_cells_across_a_boundary_land_in_different_chunks() {
        assert_ne!(ChunkCoord::containing(-1, 0), ChunkCoord::containing(0, 0));
        // ...but at the same row within their respective chunks.
        assert_eq!(local_index(-1, 5), local_index(63, 5));
    }

    #[test]
    fn chunk_bounds_cover_exactly_the_chunk() {
        let c = ChunkCoord::new(-2, 3);
        let b = c.bounds();
        assert_eq!(b.width(), CHUNK_SIZE);
        assert_eq!(b.height(), CHUNK_SIZE);
        assert_eq!((b.min_x, b.min_y), c.origin());
        assert!(b.contains(b.min_x, b.min_y));
        assert!(b.contains(b.max_x, b.max_y));
        assert!(!b.contains(b.max_x + 1, b.max_y));
    }

    #[test]
    fn rect_intersection_detects_disjoint() {
        let a = Rect::new(0, 0, 10, 10);
        assert_eq!(a.intersection(Rect::new(5, 5, 20, 20)), Some(Rect::new(5, 5, 10, 10)));
        assert_eq!(a.intersection(Rect::new(11, 0, 20, 10)), None);
    }

    #[test]
    fn writes_dirty_the_chunk_only_after_the_sweep_ends() {
        let coord = ChunkCoord::new(0, 0);
        let mut chunk = Chunk::new(coord);
        chunk.end_sweep(); // clear the initial full-chunk dirty region
        assert!(chunk.is_settled());

        chunk.set_world(10, 10, Cell::new(material::SAND, 0), 1, false);
        // The write must not extend the sweep currently in flight...
        assert!(chunk.is_settled());
        // ...but must be picked up by the next one.
        chunk.end_sweep();
        assert!(!chunk.is_settled());
        let region = chunk.sweep_region().unwrap();
        assert!(region.contains(10, 10));
        // Expanded by one so the cell below is reconsidered too.
        assert!(region.contains(10, 11));
    }

    #[test]
    fn sweep_region_is_clipped_to_the_chunk() {
        let coord = ChunkCoord::new(0, 0);
        let mut chunk = Chunk::new(coord);
        chunk.end_sweep();
        // A write in the corner would expand past the chunk edge.
        chunk.set_world(0, 0, Cell::new(material::SAND, 0), 1, false);
        chunk.end_sweep();
        let region = chunk.sweep_region().unwrap();
        assert_eq!(region.min_x, 0);
        assert_eq!(region.min_y, 0);
    }

    #[test]
    fn a_chunks_tracked_reach_starts_at_one_and_only_grows_from_writes() {
        // Issue #3: a chunk holding only short-reach material (or nothing at
        // all) must not pay for `MAX_REACH`-wide sweep regions. A fresh
        // chunk's reach is the floor, 1, not the flat `MAX_REACH` every
        // chunk used before this change. Positions kept well clear of the
        // chunk edge throughout so `sweep_region`'s own clip-to-bounds never
        // masks what's actually being tested.
        let coord = ChunkCoord::new(0, 0);
        let mut chunk = Chunk::new(coord);
        chunk.end_sweep();
        chunk.set_world(30, 30, Cell::new(material::SAND, 0), 1, false);
        chunk.end_sweep();
        let region = chunk.sweep_region().unwrap();
        assert_eq!(region.min_x, 30 - 1);
        assert_eq!(region.max_x, 30 + 1);

        // A write carrying a larger reach (e.g. a wide-dispersion gas)
        // widens the chunk's *tracked* reach -- a property of the chunk,
        // not of any one dirty rect (`dirty` itself reflects only the most
        // recently completed sweep interval; `end_sweep` replaces it rather
        // than accumulating across calls, so this region is centred on the
        // second write alone, not the union of both).
        chunk.set_world(40, 40, Cell::new(material::SMOKE, 0), 6, false);
        chunk.end_sweep();
        let region = chunk.sweep_region().unwrap();
        assert_eq!(region.min_x, 40 - 6);
        assert_eq!(region.max_x, 40 + 6);

        // The widened reach persists for a later write even with a smaller
        // reach of its own -- proving it is `set_world`'s `max`, not a
        // per-write value that would reset.
        chunk.set_world(30, 30, Cell::new(material::SAND, 0), 1, false);
        chunk.end_sweep();
        let region = chunk.sweep_region().unwrap();
        assert_eq!(region.min_x, 30 - 6);
        assert_eq!(region.max_x, 30 + 6);
    }

    #[test]
    fn recompute_reach_shrinks_once_the_wide_reach_material_is_gone() {
        // The one place a chunk's tracked reach is allowed to shrink back
        // down -- growth via `set_world` alone can never discover that a
        // wide-reach material has since been fully removed. Position kept
        // well clear of the chunk edge so the expansion below isn't clipped.
        let coord = ChunkCoord::new(0, 0);
        let mut chunk = Chunk::new(coord);
        chunk.end_sweep();
        chunk.set_world(30, 30, Cell::new(material::SMOKE, 0), 6, false);
        chunk.end_sweep();
        let widened = chunk.sweep_region().unwrap();
        assert_eq!(widened.max_x - widened.min_x, 12);

        chunk.set_world(30, 30, Cell::EMPTY, 0, false);
        chunk.end_sweep();
        // Still wide -- nothing has recomputed it yet.
        let still_wide = chunk.sweep_region().unwrap();
        assert_eq!(still_wide.max_x - still_wide.min_x, 12);

        chunk.recompute_reach(|_| 0);
        chunk.set_world(30, 30, Cell::EMPTY, 0, false); // re-dirty so sweep_region is Some again
        chunk.end_sweep();
        let region = chunk.sweep_region().unwrap();
        assert_eq!(region.max_x - region.min_x, 2); // back to the floor of 1
    }

    #[test]
    fn a_settled_chunk_has_no_sweep_region() {
        let mut chunk = Chunk::new(ChunkCoord::new(0, 0));
        chunk.end_sweep();
        assert!(chunk.sweep_region().is_none());
    }
}
