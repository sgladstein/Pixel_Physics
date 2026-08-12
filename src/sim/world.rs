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

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use super::cell::Cell;
use super::chunk::{Chunk, ChunkCoord, Rect, CHUNK_SIZE, MAX_REACH};
use super::creature::CreatureState;
use super::field::{self, FieldCell, FieldTile, FIELD_SCALE};
use super::material::{self, MaterialId, MaterialRegistry};
use super::plant::TreeState;
use super::rng::Rng;
use super::scheduler::{self, ActiveSite};
use super::surface::CellSurface;

pub struct World {
    chunks: HashMap<ChunkCoord, Chunk>,
    /// One tile per chunk, same lifetime — see the module doc on `field` for
    /// why tying them together avoids a second loading/unloading system.
    fields: HashMap<ChunkCoord, FieldTile>,
    /// `Some` for the fixed-size world of M2; M10 sets this to `None` to mean
    /// unbounded, at which point reads outside loaded chunks trigger generation
    /// instead of returning the out-of-bounds sentinel.
    bounds: Option<Rect>,
    pub frame: u64,
    pub materials: MaterialRegistry,
    pub rng: Rng,
    /// M16: growing plant tips (and M17/M18's structural checks and
    /// creature ticks), due soonest at the top -- a min-heap keyed on
    /// `ActiveSite::next_frame`, see `scheduler::step`'s own doc for why
    /// this replaced a `HashMap<ChunkCoord, Vec<ActiveSite>>` (issue #7:
    /// nothing ever looked sites up *by* chunk, only iterated the whole
    /// thing every frame regardless, and a `HashMap`'s randomized iteration
    /// order was the engine's one documented source of non-determinism).
    active_sites: BinaryHeap<Reverse<ActiveSite>>,
    /// M16: per-tree state (attractor points, growth tips, root tips) too
    /// large to fit in a `Cell` — see `plant::TreeState`. Never shrinks;
    /// entries are indexed by `ActiveKind::TreeTip`/`RootTip` and never
    /// reassigned, the same stability guarantee `MaterialId`s get.
    trees: Vec<TreeState>,
    /// M18: per-creature state (currently just its energy budget) — see
    /// `creature::CreatureState`. Never shrinks, mirroring `trees` above;
    /// indexed by `u16`, not `u32` like `trees`, because `Cell::aux` (also
    /// a `u16`) stores this same index directly per its own documented
    /// meaning for `MaterialKind::Creature` — unlike a tree's growth stage,
    /// "which creature owns this cell" has to round-trip through the cell.
    creatures: Vec<CreatureState>,
    /// M13/issue #4: whether the field grid has already converged to a
    /// fixed point (every cell within `field::step`'s settle epsilon of its
    /// previous value). `field::step` skips its whole five-pass solve when
    /// this is `true` *and* nothing is moving on the CA grid — see that
    /// function's own doc for why checking both, not just this flag alone,
    /// is what keeps a shockwave crossing the world safe: CA activity
    /// (which includes the very act of painting a new wall, since any
    /// `World::set` dirties its own chunk) forces at least one more full
    /// pass, which is what lets an occupancy change actually get noticed
    /// rather than needing separate tracking for it. Starts `false` so a
    /// freshly created world's field gets at least one real solve.
    fields_settled: bool,
}

impl World {
    pub fn new(bounds: Rect) -> Self {
        let mut world = Self {
            chunks: HashMap::new(),
            fields: HashMap::new(),
            bounds: Some(bounds),
            frame: 0,
            materials: MaterialRegistry::builtin(),
            rng: Rng::default(),
            active_sites: BinaryHeap::new(),
            trees: Vec::new(),
            creatures: Vec::new(),
            fields_settled: false,
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
                self.fields.entry(coord).or_insert_with(FieldTile::new);
            }
        }
    }

    /// Advance the coarse field grid by one step. Its own frame phase,
    /// deliberately separate from the CA sweep — see `field::step`.
    pub fn step_fields(&mut self) {
        field::step(self);
    }

    /// Advance the M16 active-site schedule by one step. Its own frame
    /// phase too, after the CA sweep and before particles — see
    /// `scheduler::step` for why growth reads/writes go through the
    /// ordinary `World::get`/`set` rather than needing any of M5's
    /// parallel-sweep machinery.
    pub fn step_active_sites(&mut self) {
        scheduler::step(self);
    }

    /// Queue a site to be checked by the scheduler once it's due. Used by
    /// `plant::plant_moss_seed`/`plant_tree` and by growth itself scheduling
    /// its own continuation.
    pub fn schedule_active_site(&mut self, site: ActiveSite) {
        self.active_sites.push(Reverse(site));
    }

    /// Total pending active sites. The headline number for whether the
    /// scheduler's cost is actually proportional to "interesting cells"
    /// rather than world size — see the debug overlay.
    pub fn active_site_count(&self) -> usize {
        self.active_sites.len()
    }

    // --- crate-internal seams used only by `scheduler::step` and
    // `plant.rs` -----------------------------------------------------------

    pub(crate) fn take_active_sites(&mut self) -> BinaryHeap<Reverse<ActiveSite>> {
        std::mem::take(&mut self.active_sites)
    }

    pub(crate) fn set_active_sites(&mut self, sites: BinaryHeap<Reverse<ActiveSite>>) {
        self.active_sites = sites;
    }

    /// Store a new tree's state and return its stable id.
    pub(crate) fn push_tree(&mut self, tree: TreeState) -> u32 {
        self.trees.push(tree);
        (self.trees.len() - 1) as u32
    }

    pub(crate) fn tree(&self, id: u32) -> &TreeState {
        &self.trees[id as usize]
    }

    pub(crate) fn tree_mut(&mut self, id: u32) -> &mut TreeState {
        &mut self.trees[id as usize]
    }

    /// Store a new creature's state and return its stable id.
    pub(crate) fn push_creature(&mut self, creature: CreatureState) -> u16 {
        debug_assert!(self.creatures.len() < u16::MAX as usize, "creature index would overflow u16 -- Cell::aux can't address more than 65535 live creature slots");
        self.creatures.push(creature);
        (self.creatures.len() - 1) as u16
    }

    pub(crate) fn creature_mut(&mut self, id: u16) -> &mut CreatureState {
        &mut self.creatures[id as usize]
    }

    /// Field conditions at a world-cell position — any position inside the
    /// same `FIELD_SCALE`-sided block reads the same cell.
    pub fn field_at(&self, world_x: i32, world_y: i32) -> FieldCell {
        field::sample(&self.fields, self.bounds, world_x, world_y)
    }

    /// Bilinear-interpolated field read at a fractional world position —
    /// architecture report §6a, "the resolution problem." Unlike `field_at`,
    /// two positions inside the same `FIELD_SCALE`-sided block don't
    /// necessarily read identically, which is what a gradient-follower with
    /// a short sensor offset needs: a worm's own four ±1-cell neighbours
    /// land in the same coarse block ~7 times in 8, degenerating a
    /// block-nearest `min_by` into "always pick the first candidate" rather
    /// than real thermotaxis. The fallback `sample_bilinear` substitutes for
    /// a blocked interpolation corner is this same position's own
    /// block-nearest reading — the gradient-follower equivalent of
    /// advection's "the destination cell's own pre-advection value."
    pub fn field_at_bilinear(&self, fx: f32, fy: f32) -> FieldCell {
        let fallback = self.field_at(fx.floor() as i32, fy.floor() as i32);
        field::sample_bilinear(&self.fields, self.bounds, fx, fy, fallback)
    }

    /// Whether the field cell covering this position is blocked by CA-solid
    /// material — the field's own occupancy map, recomputed every field step
    /// from a full 8x8 (or whatever `FIELD_SCALE` is) scan. Distinct from
    /// checking a single CA cell directly: a wall not aligned to a
    /// `FIELD_SCALE` boundary can block a field cell without any specific
    /// sampled world position inside it reading as solid.
    pub fn field_is_blocked(&self, world_x: i32, world_y: i32) -> bool {
        field::is_blocked(&self.fields, self.bounds, world_x, world_y)
    }

    /// Raise pressure in a filled circle. A synthetic disturbance for testing
    /// the field solver on its own, and the mechanism M15 explosions use.
    pub fn add_pressure_impulse(&mut self, cx: i32, cy: i32, radius: i32, amount: f32) {
        self.paint_field(cx, cy, radius, |c| c.pressure += amount);
    }

    /// Raise temperature in a filled circle. A synthetic heat source for
    /// testing diffusion on its own, ahead of M14 giving fire a real reason
    /// to call this.
    pub fn add_heat(&mut self, cx: i32, cy: i32, radius: i32, amount: f32) {
        self.paint_field(cx, cy, radius, |c| c.temperature += amount);
    }

    /// Raise light in a filled circle. A synthetic source for testing the
    /// diffusion/decay approximation before anything in the world emits light
    /// on its own (M14's fire will be the first real emitter).
    pub fn add_light(&mut self, cx: i32, cy: i32, radius: i32, amount: f32) {
        self.paint_field(cx, cy, radius, |c| c.light += amount);
    }

    /// Lower moisture in a filled circle, floored at zero — architecture
    /// §5g, a root's own write to the channel it reads. `apply_moisture_
    /// sources` will re-force this back up next step if the drained cell
    /// still contains a `Liquid` CA cell (a body of water big enough that
    /// one root's sip is noise against it), so this only actually matters —
    /// which is the point — where a root has drained the *local* water
    /// faster than the source can replenish it, e.g. a small puddle a root
    /// is draining cell by cell. That's the resource-competition signal a
    /// neighbouring root's own `moisture_pull` read is meant to notice.
    pub fn deplete_moisture(&mut self, cx: i32, cy: i32, radius: i32, amount: f32) {
        self.paint_field(cx, cy, radius, |c| c.moisture = (c.moisture - amount).max(0.0));
    }

    /// Apply `f` to every field cell within `radius` *world cells* of
    /// `(cx, cy)`.
    ///
    /// Works entirely in field-cell space rather than stepping world
    /// coordinates by `FIELD_SCALE`: starting that walk from an arbitrary,
    /// non-field-aligned point and testing world-space distance against the
    /// radius can skip the very field cell the caller meant to hit — a radius
    /// smaller than `FIELD_SCALE` has no world-space sample point that both
    /// lands on a field-cell boundary and falls inside a small circle. Testing
    /// distance in field-cell units instead guarantees the containing field
    /// cell is always included, and the "+1" slack keeps the footprint an
    /// approximate disc rather than a diamond. Field-level physics does not
    /// resolve anything finer than one field cell, so this is exactly as
    /// precise as the abstraction supports — exact circle-vs-rectangle overlap
    /// math would be precision spent on a value nothing downstream can use.
    fn paint_field(&mut self, cx: i32, cy: i32, radius: i32, f: impl Fn(&mut FieldCell)) {
        // A disturbance from outside `field::step`'s own solve -- must wake
        // it even if the field had already converged, or the write below
        // would sit unprocessed forever the next time `field::step` sees
        // zero CA activity and skips its pass entirely (see issue #4).
        self.fields_settled = false;
        let (fcx, fcy) = field::field_coord_of(cx, cy);
        let field_radius = radius / FIELD_SCALE;
        let r2 = field_radius * field_radius + 1;

        for dfy in -field_radius..=field_radius {
            for dfx in -field_radius..=field_radius {
                if dfx * dfx + dfy * dfy > r2 {
                    continue;
                }
                let (fx, fy) = (fcx + dfx, fcy + dfy);
                let (tile_coord, lx, ly) = field::tile_and_local(fx, fy);
                // A field cell exists only where its owning chunk is
                // resident, mirroring how CA writes outside a loaded chunk
                // are simply not materialised.
                if let Some(tile) = self.fields.get_mut(&tile_coord) {
                    let mut cell = tile.get_local(lx, ly);
                    f(&mut cell);
                    tile.set_local(lx, ly, cell);
                }
            }
        }
    }

    // --- crate-internal seams used only by `field::step` -------------------

    pub(crate) fn fields_settled(&self) -> bool {
        self.fields_settled
    }

    pub(crate) fn set_fields_settled(&mut self, settled: bool) {
        self.fields_settled = settled;
    }

    pub(crate) fn fields_ref(&self) -> &HashMap<ChunkCoord, FieldTile> {
        &self.fields
    }

    pub(crate) fn replace_fields(&mut self, new_fields: HashMap<ChunkCoord, FieldTile>) {
        self.fields = new_fields;
    }

    // --- crate-internal seams used only by `parallel::step` (M5) -----------
    //
    // A rayon worker needs exclusive `&mut Chunk`/`&mut FieldTile` access to
    // its own chunk while the rest of `World` stays shared and read-only.
    // Pulling the chunk and its field tile out of their maps into a plain
    // `Vec` element is what makes that safe without `unsafe`: a `Vec`'s
    // elements don't alias each other the way two `&mut` borrows into the
    // same `HashMap` would. See `parallel.rs` for the full picture.

    pub(crate) fn take_chunk(&mut self, coord: ChunkCoord) -> Option<Chunk> {
        self.chunks.remove(&coord)
    }

    pub(crate) fn put_chunk(&mut self, coord: ChunkCoord, chunk: Chunk) {
        self.chunks.insert(coord, chunk);
    }

    pub(crate) fn take_field(&mut self, coord: ChunkCoord) -> Option<FieldTile> {
        self.fields.remove(&coord)
    }

    pub(crate) fn put_field(&mut self, coord: ChunkCoord, field: FieldTile) {
        self.fields.insert(coord, field);
    }

    /// Replay a `ChunkView`'s queued neighbour-wake from `set` on a chunk
    /// that has since been reinserted. Mirrors `touch_neighbours`'s own
    /// existence check: a non-resident chunk has nothing to simulate and is
    /// silently skipped rather than created.
    pub(crate) fn mark_dirty_at(&mut self, coord: ChunkCoord, x: i32, y: i32) {
        if let Some(chunk) = self.chunks.get_mut(&coord) {
            chunk.mark_dirty(x, y);
        }
    }

    /// Replay a `ChunkView`'s queued field write on a tile that has since
    /// been reinserted. Field-cell granular (not `add_heat`'s whole-circle
    /// call) so replaying several queued cells from one pass never
    /// double-applies to any cell a worker already wrote to directly.
    pub(crate) fn add_heat_local(&mut self, tile_coord: ChunkCoord, lx: i32, ly: i32, amount: f32) {
        if let Some(tile) = self.fields.get_mut(&tile_coord) {
            let mut cell = tile.get_local(lx, ly);
            cell.temperature += amount;
            tile.set_local(lx, ly, cell);
            // Same reasoning as `paint_field` -- a burning cell's per-frame
            // heat push (`fire::tick_burn`) must be able to wake an
            // already-converged field, or it would sit unprocessed the next
            // time `field::step` sees zero CA activity and skips its pass.
            self.fields_settled = false;
        }
    }

    /// Replay a `ChunkView`'s queued cross-chunk light write -- mirrors
    /// `add_heat_local` exactly, one channel over.
    pub(crate) fn add_light_local(&mut self, tile_coord: ChunkCoord, lx: i32, ly: i32, amount: f32) {
        if let Some(tile) = self.fields.get_mut(&tile_coord) {
            let mut cell = tile.get_local(lx, ly);
            cell.light += amount;
            tile.set_local(lx, ly, cell);
            self.fields_settled = false;
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
        let reach = self.materials.get(cell.material).sweep_reach();
        self.chunks
            .entry(coord)
            .or_insert_with(|| Chunk::new(coord))
            .set_world(x, y, cell, reach);
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
        // see it — `MAX_REACH` sideways, one row up or down. This guard is a
        // no-op at today's constants: `MAX_REACH` (32) is exactly
        // `CHUNK_SIZE / 2` (64), so `MAX_REACH..CHUNK_SIZE - MAX_REACH` is
        // `32..32`, the empty range, and `contains` is always `false` —
        // every column in the chunk is within reach of some neighbour, so
        // there is no interior left to skip. Kept (rather than deleted) as
        // documentation of that fact.
        //
        // Deliberately still keyed on the flat `MAX_REACH`, not the
        // per-chunk tracked reach issue #3 added to `Chunk::sweep_region`
        // (`chunk.rs`). Those are different questions: this decides which
        // chunks get *woken* (a conservative "might this matter" check, safe
        // to over-wake), while `sweep_region`'s widening decides how much of
        // an already-awake chunk gets *re-examined* (where over-widening is
        // the actual cost issue #3 exists to cut). `parallel.rs`'s
        // cross-chunk write-safety proof is pinned to this same flat
        // `MAX_REACH` too, via `queue_touch_neighbours`'s identical guard —
        // narrowing this one would need re-deriving that proof from an
        // equality to an inequality, which issue #3's actual fix does not
        // require: sweep_region only ever *shrinks* relative to before, so
        // it cannot invalidate a proof about how far a write can land.
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

    /// Force-ignite every non-empty cell in a filled circle. A debug/testing
    /// tool for triggering fire without waiting on a spontaneous ignition
    /// source — M15 explosions will have their own, more physical way to
    /// start fires; this exists so M14's fire mechanics can be exercised and
    /// watched in the live app before that lands.
    ///
    /// Ignoring `material.flammability` entirely and always using a fallback
    /// duration when `burn_duration` is unset (0) is deliberate for a debug
    /// tool: it should light *anything*, including a material nobody has
    /// tuned combustion numbers for yet, rather than silently doing nothing
    /// and leaving whoever pressed the key wondering if it's broken.
    pub fn ignite_circle(&mut self, cx: i32, cy: i32, radius: i32) {
        const FALLBACK_DURATION: u16 = 180;
        let r2 = radius * radius;
        for y in (cy - radius)..=(cy + radius) {
            for x in (cx - radius)..=(cx + radius) {
                let (dx, dy) = (x - cx, y - cy);
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let mut cell = self.get(x, y);
                if cell.is_empty() || cell.is_burning() {
                    continue;
                }
                let duration = self.materials.get(cell.material).burn_duration;
                cell.ignite(if duration > 0 { duration } else { FALLBACK_DURATION });
                self.set(x, y, cell);
            }
        }
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
                // does not silently delete stone. `Plant` gets the same
                // protection as `Solid` -- a grown tree is exactly as
                // deliberately placed as a stone wall, and without this the
                // brush could erase it cell by cell same as any loose powder.
                let existing_material = self.get(x, y).material;
                if material != material::EMPTY
                    && existing_material != material::EMPTY
                    && matches!(
                        self.materials.kind(existing_material),
                        material::MaterialKind::Solid | material::MaterialKind::Plant
                    )
                {
                    continue;
                }
                let shade = self.rng.below(shades) as u8;
                self.set(x, y, Cell::new(material, shade));
                // M17: either side of this write might be a `Solid`/`Plant`
                // (architecture item 9) that just gained or lost a neighbour
                // it was relying on -- placing new stone, or erasing existing
                // stone (or a tree trunk) out from under something else.
                // Schedule reactively rather than at every paint stroke
                // unconditionally, so a stroke over already-empty ground (the
                // overwhelmingly common case while painting powders/liquids)
                // costs nothing extra.
                let placed_structural = matches!(self.materials.kind(material), material::MaterialKind::Solid | material::MaterialKind::Plant);
                let erased_structural = material == material::EMPTY
                    && matches!(self.materials.kind(existing_material), material::MaterialKind::Solid | material::MaterialKind::Plant);
                if placed_structural || erased_structural {
                    self.schedule_structural_check_around(x, y);
                }
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
        // Recomputing reach is a full scan of the chunk's cells, so it only
        // runs at the one point that is both cheap and safe: exactly when a
        // chunk transitions from active to settled this step (issue #3). A
        // chunk that stays active keeps whatever `set_world` has grown its
        // reach to; a chunk that was already settled has nothing that could
        // have changed since the last recompute, so re-scanning it every
        // frame would burn cycles on a world that is otherwise supposed to
        // cost near-zero once everything sleeps.
        let materials = &self.materials;
        for chunk in self.chunks.values_mut() {
            let was_settled = chunk.is_settled();
            chunk.end_sweep();
            if !was_settled && chunk.is_settled() {
                chunk.recompute_reach(|cell| materials.get(cell.material).sweep_reach());
            }
        }
    }
}

/// Thin delegation to `World`'s own methods, unchanged behaviour — the serial
/// path every test and every non-sweep caller (painting, explosions,
/// particles) already uses. See `surface.rs` for why this exists as a trait
/// at all, and `parallel.rs`'s `ChunkView` for the other implementer.
impl CellSurface for World {
    #[inline]
    fn get(&self, x: i32, y: i32) -> Cell {
        World::get(self, x, y)
    }

    #[inline]
    fn set(&mut self, x: i32, y: i32, cell: Cell) {
        World::set(self, x, y, cell)
    }

    #[inline]
    fn in_bounds(&self, x: i32, y: i32) -> bool {
        World::in_bounds(self, x, y)
    }

    #[inline]
    fn clear_moved(&mut self, x: i32, y: i32) {
        World::clear_moved(self, x, y)
    }

    #[inline]
    fn materials(&self) -> &MaterialRegistry {
        &self.materials
    }

    #[inline]
    fn rng(&mut self) -> &mut Rng {
        &mut self.rng
    }

    #[inline]
    fn add_heat(&mut self, x: i32, y: i32, radius: i32, amount: f32) {
        World::add_heat(self, x, y, radius, amount)
    }

    #[inline]
    fn add_light(&mut self, x: i32, y: i32, radius: i32, amount: f32) {
        World::add_light(self, x, y, radius, amount)
    }

    #[inline]
    fn field_moisture_at(&self, x: i32, y: i32) -> f32 {
        World::field_at(self, x, y).moisture
    }

    #[inline]
    fn frame(&self) -> u64 {
        self.frame
    }

    #[inline]
    fn schedule_active_site(&mut self, site: ActiveSite) {
        World::schedule_active_site(self, site)
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
    fn neighbour_waking_stops_at_the_neighbours_own_reach() {
        // Waking has to cover everything that can see the write, and nothing
        // beyond — waking the whole world on every write would be correct but
        // would defeat sleeping entirely.
        //
        // Before issue #3, every chunk widened its sweep region by the same
        // flat `MAX_REACH` regardless of what it actually held, so any write
        // within `MAX_REACH` of a neighbour always produced a real sweep
        // region there. Now a chunk's widening is its own tracked reach
        // (`Chunk::sweep_region`), so `touch_neighbours` still conservatively
        // marks a distant neighbour dirty (unchanged — see its own doc), but
        // an otherwise-empty neighbour's own reach floors at 1, and a write
        // 32 cells from its edge is far further than anything with reach 1
        // could ever see. That is the fix, not a regression: an empty
        // neighbour chunk no longer pays for a wide, pointless sweep just
        // because something moved far away in a chunk next door.
        // `a_write_at_a_chunk_edge_wakes_the_neighbour` above covers the
        // genuinely-adjacent case, where waking still works correctly.
        let mut w = World::new(Rect::new(0, 0, 255, 127));
        w.end_step();
        w.end_step();
        assert_eq!(w.active_chunk_count(), 0);

        w.set(32, 32, Cell::new(material::SAND, 0));
        w.end_step();

        // Its own chunk always gets a real sweep region...
        assert!(w.sweep_region(ChunkCoord::new(0, 0)).is_some());
        // ...but chunk (1,0), 32 cells from the write and holding nothing
        // but empty cells (reach 1), does not — even though it is within
        // `touch_neighbours`'s conservative `MAX_REACH` wake radius and gets
        // marked dirty, its own small reach can't expand back into its own
        // bounds from a point that far outside them.
        assert!(w.sweep_region(ChunkCoord::new(1, 0)).is_none());
        // Far beyond even the conservative wake radius in both axes.
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
