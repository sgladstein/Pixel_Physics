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
            rng: Rng::new(seed_from_coord(coord)),
            reach: 1,
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
    pub fn set_world(&mut self, x: i32, y: i32, cell: Cell, reach: i32) {
        self.cells[local_index(x, y)] = cell;
        self.mark_dirty(x, y);
        self.reach = self.reach.max(reach);
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

    /// True when nothing changed last frame and the chunk can be skipped.
    pub fn is_settled(&self) -> bool {
        self.dirty.is_none()
    }

    /// Promote writes made during this sweep into the region for the next one.
    pub fn end_sweep(&mut self) {
        self.dirty = self.pending_dirty.take();
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

        chunk.set_world(10, 10, Cell::new(material::SAND, 0), 1);
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
        chunk.set_world(0, 0, Cell::new(material::SAND, 0), 1);
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
        chunk.set_world(30, 30, Cell::new(material::SAND, 0), 1);
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
        chunk.set_world(40, 40, Cell::new(material::SMOKE, 0), 6);
        chunk.end_sweep();
        let region = chunk.sweep_region().unwrap();
        assert_eq!(region.min_x, 40 - 6);
        assert_eq!(region.max_x, 40 + 6);

        // The widened reach persists for a later write even with a smaller
        // reach of its own -- proving it is `set_world`'s `max`, not a
        // per-write value that would reset.
        chunk.set_world(30, 30, Cell::new(material::SAND, 0), 1);
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
        chunk.set_world(30, 30, Cell::new(material::SMOKE, 0), 6);
        chunk.end_sweep();
        let widened = chunk.sweep_region().unwrap();
        assert_eq!(widened.max_x - widened.min_x, 12);

        chunk.set_world(30, 30, Cell::EMPTY, 0);
        chunk.end_sweep();
        // Still wide -- nothing has recomputed it yet.
        let still_wide = chunk.sweep_region().unwrap();
        assert_eq!(still_wide.max_x - still_wide.min_x, 12);

        chunk.recompute_reach(|_| 0);
        chunk.set_world(30, 30, Cell::EMPTY, 0); // re-dirty so sweep_region is Some again
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
