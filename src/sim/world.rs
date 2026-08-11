//! The world: a sparse map of chunks addressed by global coordinates.
//!
//! Three invariants here are load-bearing for everything that comes later, and
//! are cheap now but very expensive to retrofit:
//!
//! 1. Storage is a `HashMap<ChunkCoord, Chunk>`, never a flat array. A flat
//!    `Vec<Cell>` indexed `y * width + x` is the single decision that would
//!    force a rewrite when the streaming world arrives in M10.
//! 2. Every coordinate crossing this API is a global signed world coordinate.
//!    Screen space exists only in the renderer.
//! 3. All cell access goes through `get`/`set`. That is the seam where chunk
//!    load, generation and eviction get added later, without touching callers.

use std::collections::HashMap;

use super::cell::Cell;
use super::chunk::{Chunk, ChunkCoord, Rect, CHUNK_SIZE, MAX_REACH};
use super::material::{self, MaterialId, MaterialRegistry};
use super::rng::Rng;

pub struct World {
    chunks: HashMap<ChunkCoord, Chunk>,
    /// `Some` for the fixed-size world of M2; M10 sets this to `None` to mean
    /// unbounded, at which point reads outside loaded chunks trigger generation
    /// instead of returning the out-of-bounds sentinel.
    bounds: Option<Rect>,
    pub frame: u64,
    pub materials: MaterialRegistry,
    pub rng: Rng,
}

impl World {
    pub fn new(bounds: Rect) -> Self {
        let mut world = Self {
            chunks: HashMap::new(),
            bounds: Some(bounds),
            frame: 0,
            materials: MaterialRegistry::builtin(),
            rng: Rng::default(),
        };
        world.ensure_chunks_for(bounds);
        world
    }

    /// Create every chunk overlapping `region` up front. For the fixed world
    /// this means the whole thing exists from the start; M10 replaces this with
    /// on-demand generation around the camera.
    fn ensure_chunks_for(&mut self, region: Rect) {
        let c0 = ChunkCoord::containing(region.min_x, region.min_y);
        let c1 = ChunkCoord::containing(region.max_x, region.max_y);
        for cy in c0.y..=c1.y {
            for cx in c0.x..=c1.x {
                let coord = ChunkCoord::new(cx, cy);
                self.chunks.entry(coord).or_insert_with(|| Chunk::new(coord));
            }
        }
    }

    pub fn bounds(&self) -> Option<Rect> {
        self.bounds
    }

    #[inline]
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        match self.bounds {
            Some(b) => b.contains(x, y),
            None => true,
        }
    }

    /// Reads outside the world return a solid sentinel rather than empty space,
    /// so material treats the world edge as a wall instead of falling through it.
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Cell {
        if !self.in_bounds(x, y) {
            return Cell::OUT_OF_BOUNDS;
        }
        match self.chunks.get(&ChunkCoord::containing(x, y)) {
            Some(chunk) => chunk.get_world(x, y),
            // In bounds but not resident: empty space that has not been
            // materialised yet.
            None => Cell::EMPTY,
        }
    }

    /// Writes outside the world are silently dropped — the caller is usually a
    /// movement rule that already checked, or a brush clipped by the edge.
    pub fn set(&mut self, x: i32, y: i32, cell: Cell) {
        if !self.in_bounds(x, y) {
            return;
        }
        let coord = ChunkCoord::containing(x, y);
        self.chunks
            .entry(coord)
            .or_insert_with(|| Chunk::new(coord))
            .set_world(x, y, cell);
        self.touch_neighbours(x, y, coord);
    }

    /// Wake the chunks adjacent to a write near a chunk boundary.
    ///
    /// Without this, material freezes at chunk edges: a settled chunk never
    /// notices that the cell just across its border became free, so material
    /// that could now flow sideways or diagonally into it never re-examines the
    /// move. Marking the exact point (rather than waking the whole chunk) keeps
    /// the neighbour's next sweep narrow, since `sweep_region` clips to bounds.
    fn touch_neighbours(&mut self, x: i32, y: i32, owner: ChunkCoord) {
        let lx = x.rem_euclid(CHUNK_SIZE);
        let ly = y.rem_euclid(CHUNK_SIZE);
        // A write can only matter to another chunk if something over there can
        // see it — `MAX_REACH` sideways, one row up or down.
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
                // Only wake chunks that already exist. A non-resident chunk has
                // nothing to simulate, and will be created by the write itself
                // if material ever moves into it.
                if let Some(chunk) = self.chunks.get_mut(&coord) {
                    chunk.mark_dirty(x, y);
                }
            }
        }
    }

    /// Clear a cell's moved flag once the sweep has skipped it.
    ///
    /// Deliberately does not dirty the chunk: this is bookkeeping, not a change
    /// to the world, and waking a chunk for it would stop anything sleeping.
    pub fn clear_moved(&mut self, x: i32, y: i32) {
        if !self.in_bounds(x, y) {
            return;
        }
        if let Some(chunk) = self.chunks.get_mut(&ChunkCoord::containing(x, y)) {
            let cell = chunk.get_world(x, y).with_moved(false);
            chunk.set_world_quiet(x, y, cell);
        }
    }

    #[inline]
    pub fn is_empty(&self, x: i32, y: i32) -> bool {
        self.get(x, y).is_empty()
    }

    #[inline]
    pub fn material_at(&self, x: i32, y: i32) -> MaterialId {
        self.get(x, y).material
    }

    /// Move the cell at `(fx, fy)` to `(tx, ty)`, exchanging with whatever is
    /// already there.
    ///
    /// `revisited` says whether the sweep will reach the destination again
    /// during this same pass — true for upward moves and for sideways moves
    /// that follow the scan direction. When it does, the mover is flagged so it
    /// is skipped once and does not travel twice in a frame. Downward moves
    /// land in rows the sweep has already passed, so they must *not* be
    /// flagged: doing so would make everything fall at half speed.
    ///
    /// The displaced cell never needs flagging — it lands on the position being
    /// processed right now, which the sweep does not revisit.
    pub fn move_cell(&mut self, fx: i32, fy: i32, tx: i32, ty: i32, revisited: bool) {
        let mover = self.get(fx, fy).with_moved(revisited);
        let displaced = self.get(tx, ty).with_moved(false);
        self.set(fx, fy, displaced);
        self.set(tx, ty, mover);
    }

    /// Paint a filled circle at full density.
    pub fn paint_circle(&mut self, cx: i32, cy: i32, radius: i32, material: MaterialId) {
        self.paint_capsule((cx, cy), (cx, cy), radius, material, 1.0);
    }

    /// Paint the area swept by a circular brush travelling from `a` to `b`.
    ///
    /// Sweeping a capsule rather than stamping a circle at interpolated points
    /// means every cell is considered exactly once, however fast the cursor
    /// moved. Stamping overlapping circles would roll the density check a dozen
    /// times per cell and fill solid regardless.
    ///
    /// `density` is the chance of filling each cell. Below 1.0 a powder is
    /// emitted as scattered grains that fall as a visible stream, instead of a
    /// solid slab appearing under the cursor; holding still still fills in
    /// within a few frames because each frame rolls again.
    pub fn paint_capsule(
        &mut self,
        a: (i32, i32),
        b: (i32, i32),
        radius: i32,
        material: MaterialId,
        density: f32,
    ) {
        let shades = self.materials.get(material).palette.len().max(1) as u32;
        let r = radius.max(0);
        let r2 = (r * r) as f32;

        for y in (a.1.min(b.1) - r)..=(a.1.max(b.1) + r) {
            for x in (a.0.min(b.0) - r)..=(a.0.max(b.0) + r) {
                if !self.in_bounds(x, y) || distance_sq_to_segment(x, y, a, b) > r2 {
                    continue;
                }
                if density < 1.0 && !self.rng.chance(density) {
                    continue;
                }
                // Erasing should clear regardless of what is there; painting a
                // real material must not overwrite solid terrain, so the brush
                // does not silently delete stone.
                if material != material::EMPTY {
                    let existing = self.get(x, y).material;
                    if existing != material::EMPTY
                        && self.materials.kind(existing) == material::MaterialKind::Solid
                    {
                        continue;
                    }
                }
                let shade = self.rng.below(shades) as u8;
                self.set(x, y, Cell::new(material, shade));
            }
        }
    }

    /// Chunk coordinates that need sweeping, ordered bottom-to-top.
    ///
    /// Bottom-first matches the row order within a chunk: material must be
    /// processed from the bottom up, or a falling column resolves in a single
    /// frame and sand teleports to the floor.
    pub fn chunks_to_sweep(&self) -> Vec<ChunkCoord> {
        let mut coords: Vec<ChunkCoord> = self
            .chunks
            .values()
            .filter(|c| !c.is_settled())
            .map(|c| c.coord)
            .collect();
        coords.sort_by(|a, b| b.y.cmp(&a.y).then(a.x.cmp(&b.x)));
        coords
    }

    pub fn sweep_region(&self, coord: ChunkCoord) -> Option<Rect> {
        self.chunks.get(&coord).and_then(|c| c.sweep_region())
    }

    pub fn chunk(&self, coord: ChunkCoord) -> Option<&Chunk> {
        self.chunks.get(&coord)
    }

    pub fn chunks(&self) -> impl Iterator<Item = &Chunk> {
        self.chunks.values()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Number of chunks that will be swept next step. Drives the debug overlay
    /// and is the headline number for whether sleeping is working.
    pub fn active_chunk_count(&self) -> usize {
        self.chunks.values().filter(|c| !c.is_settled()).count()
    }

    /// Force every chunk to be examined in full on the next step.
    ///
    /// Escape hatch for cases where the dirty rectangles cannot know something
    /// changed — and the control in tests that separates "the movement rules
    /// are wrong" from "the sweep never looked".
    pub fn wake_all(&mut self) {
        for chunk in self.chunks.values_mut() {
            chunk.wake();
        }
    }

    pub fn begin_step(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    pub fn end_step(&mut self) {
        for chunk in self.chunks.values_mut() {
            chunk.end_sweep();
        }
    }
}

/// Squared distance from a cell to the segment `a`–`b`, which is what makes the
/// brush a capsule rather than a rectangle around the cursor's path.
fn distance_sq_to_segment(px: i32, py: i32, a: (i32, i32), b: (i32, i32)) -> f32 {
    let (ax, ay) = (a.0 as f32, a.1 as f32);
    let (abx, aby) = ((b.0 - a.0) as f32, (b.1 - a.1) as f32);
    let length_sq = abx * abx + aby * aby;

    // Projection of the point onto the segment, clamped to its ends. A
    // zero-length segment is a single circle, where the projection is the start.
    let t = if length_sq <= f32::EPSILON {
        0.0
    } else {
        (((px as f32 - ax) * abx + (py as f32 - ay) * aby) / length_sq).clamp(0.0, 1.0)
    };

    let dx = px as f32 - (ax + abx * t);
    let dy = py as f32 - (ay + aby * t);
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_world() -> World {
        World::new(Rect::new(0, 0, 127, 127))
    }

    #[test]
    fn reads_outside_the_world_are_solid_not_empty() {
        let w = test_world();
        assert!(!w.get(-1, 0).is_empty());
        assert_eq!(w.get(-1, 0), Cell::OUT_OF_BOUNDS);
        assert_eq!(w.get(0, 128), Cell::OUT_OF_BOUNDS);
        // ...and inside is empty.
        assert!(w.get(0, 0).is_empty());
        assert!(w.get(127, 127).is_empty());
    }

    #[test]
    fn writes_outside_the_world_are_dropped() {
        let mut w = test_world();
        w.set(-5, -5, Cell::new(material::SAND, 0));
        assert_eq!(w.get(-5, -5), Cell::OUT_OF_BOUNDS);
    }

    #[test]
    fn set_then_get_round_trips_across_chunk_boundaries() {
        let mut w = test_world();
        for (x, y) in [(0, 0), (63, 63), (64, 64), (65, 0), (127, 127)] {
            w.set(x, y, Cell::new(material::SAND, 1));
            assert_eq!(w.get(x, y).material, material::SAND, "failed at ({x}, {y})");
        }
    }

    #[test]
    fn move_cell_exchanges_materials() {
        let mut w = test_world();
        w.set(10, 10, Cell::new(material::SAND, 0));
        w.move_cell(10, 10, 10, 11, false);
        assert!(w.get(10, 10).is_empty());
        assert_eq!(w.get(10, 11).material, material::SAND);
    }

    #[test]
    fn move_cell_flags_the_mover_only_when_it_will_be_revisited() {
        // Downward moves land in already-swept rows. Flagging them would make
        // everything fall at half speed.
        let mut w = test_world();
        w.set(10, 10, Cell::new(material::SAND, 0));
        w.move_cell(10, 10, 10, 11, false);
        assert!(!w.get(10, 11).moved());

        // Upward and same-direction sideways moves will be reached again.
        let mut w = test_world();
        w.set(10, 10, Cell::new(material::SMOKE, 0));
        w.move_cell(10, 10, 10, 9, true);
        assert!(w.get(10, 9).moved());
    }

    #[test]
    fn the_displaced_cell_is_never_left_flagged() {
        // It lands on the position being processed right now, which the sweep
        // does not revisit — and a stale flag would cost it a frame.
        let mut w = test_world();
        w.set(10, 10, Cell::new(material::SAND, 0));
        w.set(10, 11, Cell::new(material::WATER, 0));
        w.move_cell(10, 10, 10, 11, true);
        assert_eq!(w.get(10, 10).material, material::WATER);
        assert!(!w.get(10, 10).moved());
    }

    #[test]
    fn clear_moved_does_not_wake_the_chunk() {
        // Clearing the flag is bookkeeping, not a change to the world. If it
        // dirtied the chunk, nothing would ever sleep.
        let mut w = test_world();
        w.set(10, 10, Cell::new(material::SAND, 0).with_moved(true));
        w.end_step();
        w.end_step();
        assert_eq!(w.active_chunk_count(), 0);

        w.clear_moved(10, 10);
        w.end_step();
        assert!(!w.get(10, 10).moved());
        assert_eq!(w.active_chunk_count(), 0, "clearing the flag woke a chunk");
    }

    #[test]
    fn a_write_at_a_chunk_edge_wakes_the_neighbour() {
        let mut w = test_world();
        w.end_step(); // settle everything after construction
        assert_eq!(w.active_chunk_count(), 0);

        // x = 63 is the last column of chunk (0,0); chunk (1,0) must notice.
        w.set(63, 10, Cell::new(material::SAND, 0));
        w.end_step();

        assert!(w.sweep_region(ChunkCoord::new(1, 0)).is_some());
        assert!(w.sweep_region(ChunkCoord::new(0, 0)).is_some());
    }

    #[test]
    fn neighbour_waking_stops_at_max_reach() {
        // Waking has to cover everything that can see the write, and nothing
        // beyond — waking the whole world on every write would be correct but
        // would defeat sleeping entirely.
        //
        // Note there is no "chunk interior" to test while `MAX_REACH` is at
        // least half a chunk: every write is then within sight of some edge,
        // and touching neighbours on all of them is correct rather than wasteful.
        let mut w = World::new(Rect::new(0, 0, 255, 127));
        w.end_step();
        w.end_step();
        assert_eq!(w.active_chunk_count(), 0);

        w.set(32, 32, Cell::new(material::SAND, 0));
        w.end_step();

        // Its own chunk, and any chunk within MAX_REACH of the write.
        assert!(w.sweep_region(ChunkCoord::new(0, 0)).is_some());
        assert_eq!(
            w.sweep_region(ChunkCoord::new(1, 0)).is_some(),
            32 + MAX_REACH >= CHUNK_SIZE,
            "chunk (1,0) should be woken exactly when the write is within reach of it"
        );
        // Far beyond reach in both axes.
        assert!(w.sweep_region(ChunkCoord::new(3, 0)).is_none());
        assert!(w.sweep_region(ChunkCoord::new(0, 1)).is_none());
    }

    #[test]
    fn chunks_are_swept_bottom_up() {
        let w = test_world();
        let order = w.chunks_to_sweep();
        // Every chunk is dirty on construction, so all four appear.
        assert_eq!(order.len(), 4);
        // Larger y is further down the screen and must come first.
        assert!(order[0].y >= order[order.len() - 1].y);
    }

    #[test]
    fn the_frame_counter_advances_every_step() {
        let mut w = test_world();
        let before = w.frame;
        w.begin_step();
        assert_eq!(w.frame, before + 1);
    }

    #[test]
    fn the_brush_does_not_erase_solid_terrain() {
        let mut w = test_world();
        w.set(20, 20, Cell::new(material::STONE, 0));
        w.paint_circle(20, 20, 3, material::SAND);
        assert_eq!(w.get(20, 20).material, material::STONE);
    }

    #[test]
    fn the_eraser_clears_solid_terrain() {
        let mut w = test_world();
        w.set(20, 20, Cell::new(material::STONE, 0));
        w.paint_circle(20, 20, 3, material::EMPTY);
        assert!(w.get(20, 20).is_empty());
    }

    #[test]
    fn the_brush_is_round_and_clipped_at_the_world_edge() {
        let mut w = test_world();
        w.paint_circle(0, 0, 4, material::SAND);
        // Inside the radius.
        assert_eq!(w.get(0, 3).material, material::SAND);
        // Outside the radius but inside the bounding box.
        assert!(w.get(3, 3).is_empty());
        // Off-world writes were dropped rather than panicking.
        assert_eq!(w.get(-1, 0), Cell::OUT_OF_BOUNDS);
    }
}
