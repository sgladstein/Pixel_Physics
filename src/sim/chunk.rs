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

/// The furthest a movement rule may look sideways from the cell it is deciding
/// about — a powder's roll, a liquid's dispersion and its search for somewhere
/// to fall are all capped at this.
///
/// Sweep regions are widened by it, because a cell has to be re-examined
/// whenever anything it can *see* changes, not just its immediate neighbours.
/// Every rule that reads further than this must either be capped or the region
/// widened to match, or material goes stale exactly the way it did when the
/// region was widened by a single cell.
///
/// It directly multiplies the work an isolated change causes — a dirty cell
/// drags a band `2 * MAX_REACH + 1` wide into the sweep — but it also sets how
/// far a liquid can see when levelling, and water that cannot see across its
/// own surface settles into a wedge instead of a flat top. At 32 the band is
/// wider than a chunk, so any change sweeps that chunk's full width; the
/// vertical banding, where most of the saving comes from, is unaffected.
pub const MAX_REACH: i32 = 32;

/// Address of a chunk in the chunk grid. Signed, because the world extends in
/// every direction once streaming arrives in M10.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
}

impl ChunkCoord {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
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

    #[inline]
    pub fn set_world(&mut self, x: i32, y: i32, cell: Cell) {
        self.cells[local_index(x, y)] = cell;
        self.mark_dirty(x, y);
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
    /// whenever anything it can see has moved — `MAX_REACH` sideways, since
    /// powders roll and liquids flow along a row, and one cell vertically,
    /// which is as far as any rule looks up or down.
    pub fn sweep_region(&self) -> Option<Rect> {
        self.dirty?
            .expanded_xy(MAX_REACH, 1)
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

        chunk.set_world(10, 10, Cell::new(material::SAND, 0));
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
        chunk.set_world(0, 0, Cell::new(material::SAND, 0));
        chunk.end_sweep();
        let region = chunk.sweep_region().unwrap();
        assert_eq!(region.min_x, 0);
        assert_eq!(region.min_y, 0);
    }

    #[test]
    fn a_settled_chunk_has_no_sweep_region() {
        let mut chunk = Chunk::new(ChunkCoord::new(0, 0));
        chunk.end_sweep();
        assert!(chunk.sweep_region().is_none());
    }
}
