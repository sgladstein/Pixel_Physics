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
use super::liquid::{self, LiquidBody};
use super::material::{self, MaterialId, MaterialKind, MaterialRegistry};
use super::organism::{self, OrganismState, SpeciesId, SpeciesRegistry};
use super::rng::Rng;
use super::scheduler::{self, ActiveSite};
use super::surface::CellSurface;

/// Bits of `Cell::organism_id` given to the slot index (the rest, high 4
/// bits, are generation). 4095 concurrently-live organisms — generous for
/// anything this engine plays at real-time rates, and `push_organism`'s
/// own debug assertion catches the day that stops being true rather than
/// silently wrapping into a valid-looking but wrong id.
const ORGANISM_INDEX_BITS: u32 = 12;
const ORGANISM_INDEX_MASK: u16 = (1 << ORGANISM_INDEX_BITS) - 1;
/// 4 bits: a slot wraps back to generation 0 after 16 reuses, at which
/// point a sufficiently stale reference from exactly 16 reuses ago could
/// in principle alias a live organism again. Accepted rather than
/// widening `Cell` a third time this session for a failure mode that
/// needs a bug (a cell holding an `organism_id` no live cell should still
/// reference) compounded with exactly the wrong reuse count to manifest —
/// the generational check still catches every *other* staleness case,
/// which is the actual, common failure mode it exists for.
const GENERATION_MASK: u8 = 0b1111;

fn encode_organism_id(slot_index: u16, generation: u8) -> u16 {
    debug_assert!(slot_index != 0 && slot_index <= ORGANISM_INDEX_MASK, "organism slot index out of range: {slot_index}");
    ((generation as u16 & GENERATION_MASK as u16) << ORGANISM_INDEX_BITS) | slot_index
}

/// `(slot_index, generation)` — `slot_index == 0` means "no organism",
/// matching `organism_id`'s own zero-is-empty convention.
fn decode_organism_id(organism_id: u16) -> (u16, u8) {
    let slot_index = organism_id & ORGANISM_INDEX_MASK;
    let generation = ((organism_id >> ORGANISM_INDEX_BITS) as u8) & GENERATION_MASK;
    (slot_index, generation)
}

struct OrganismSlot {
    generation: u8,
    /// `None` when this slot is on the free list — kept rather than
    /// removing the slot entirely, since `organisms` is addressed by
    /// stable index and shrinking it would renumber every slot after it.
    state: Option<OrganismState>,
}

/// Identifies a promoted `liquid::LiquidBody` (`Reports/liquid-heightfield-
/// design.md` §3c/§9a). Never stored on a `Cell` — unlike `organism_id`,
/// which has to round-trip through a cell's own bits, a liquid body's cell
/// has no body-local coordinate to remember (its position *is* its column
/// index, recoverable from `x` alone), so this only ever lives in
/// `World::body_index`. Carries a generation for the identical reason
/// `organism_id` does: a stale id held after its slot is freed and reused
/// must resolve to `None`, not to a different, unrelated body.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct BodyId {
    index: u32,
    generation: u32,
}

struct BodySlot {
    generation: u32,
    /// `None` when this slot is on the free list — same reasoning as
    /// `OrganismSlot::state`.
    state: Option<LiquidBody>,
}

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
    /// M8: coherent pieces of broken structure currently in flight
    /// (`rigid::ChunkBody`). A plain `Vec`, stepped in index order, because
    /// insertion order is the only tiebreak that stays identical run to run
    /// — the same determinism requirement that moved `active_sites` off a
    /// `HashMap`. Distinct from `bodies` below, which is the liquid
    /// heightfield's own unrelated arena.
    pub chunk_bodies: Vec<crate::sim::rigid::ChunkBody>,
    /// M9: the summoned character, if any — off-grid like `chunk_bodies`
    /// and stepped in the same serial phase (`player::step`). On `World`
    /// rather than `App` so the renderer (which takes `&World`) can draw
    /// it and so the sim step stays a pure function of (world, input).
    pub player: Option<crate::sim::player::Player>,
    /// M16: growing plant tips (and M17/M18's structural checks and
    /// creature ticks), due soonest at the top -- a min-heap keyed on
    /// `ActiveSite::next_frame`, see `scheduler::step`'s own doc for why
    /// this replaced a `HashMap<ChunkCoord, Vec<ActiveSite>>` (issue #7:
    /// nothing ever looked sites up *by* chunk, only iterated the whole
    /// thing every frame regardless, and a `HashMap`'s randomized iteration
    /// order was the engine's one documented source of non-determinism).
    active_sites: BinaryHeap<Reverse<ActiveSite>>,
    /// Positions with an `ActiveKind::StructuralCheck` currently somewhere
    /// in `active_sites` — a dedup index the heap itself can't answer
    /// cheaply (a `BinaryHeap` has no membership test). Exists because
    /// `structural::schedule_structural_check_around` fans out to five
    /// positions (the disturbed cell plus its four neighbours) per call,
    /// and disturbance sites routinely overlap — an explosion clearing a
    /// filled circle calls it once per cleared cell, so a radius-20
    /// explosion (~1,256 cells) can raise up to ~6,280 raw requests for a
    /// handful of genuinely distinct positions near the boundary. Without
    /// this, every one of those lands in the heap and gets processed the
    /// same frame (`next_frame` is always "now" for a structural check),
    /// spiking that frame's cost in proportion to explosion size — exactly
    /// what the active-site scheduler's whole design exists to avoid.
    /// `StructuralCheck` carries no state beyond position, so `(x, y)`
    /// alone is an unambiguous key. Kept in lockstep with `active_sites`:
    /// inserted in `structural::schedule_structural_check` only when not
    /// already present (skipping the push entirely when it is), removed
    /// in `scheduler::step` the instant a `StructuralCheck` site is popped
    /// — before `structural::tick` runs, so a check that legitimately
    /// re-schedules itself or a neighbour while running is a fresh
    /// request, not a stale one being silently dropped.
    pending_structural_checks: std::collections::HashSet<(i32, i32)>,
    /// Backing storage for promoted `liquid::LiquidBody` bodies (`Reports/
    /// liquid-heightfield-design.md` §9a) — the `World::organisms` /
    /// `OrganismSlot` generational-slot pattern, reused rather than
    /// reinvented (a `BodyId` is not a `Cell` field, so unlike `organism_id`
    /// there is no bit budget forcing index/generation packing, but the
    /// same staleness hazard — a freed slot's id still held somewhere —
    /// exists identically).
    bodies: Vec<BodySlot>,
    free_body_slots: Vec<u32>,
    /// "Which bodies touch this chunk" (`Reports/liquid-heightfield-
    /// design.md` §3c) — a body's cells have no back-pointer to their own
    /// `BodyId`, so resolving a disturbed position to the body that owns it
    /// goes through here: one hash lookup to the handful of candidates
    /// touching that chunk, then a linear scan checking each candidate's
    /// own recorded column range. Bodies are few (tens, not thousands) and
    /// a chunk overlaps at most a handful, so this stays cheap without
    /// needing a denser index. A `Vec`, not `SmallVec` — the crate has no
    /// existing `smallvec` dependency and a body touching more than a
    /// couple of chunks is rare enough not to justify adding one.
    body_index: HashMap<ChunkCoord, Vec<BodyId>>,
    /// M18: per-creature state (currently just its energy budget) — see
    /// `creature::CreatureState`. Never shrinks, mirroring `trees` above;
    /// indexed by `u16`, not `u32` like `trees`, because `Cell::aux` (also
    /// a `u16`) stores this same index directly per its own documented
    /// meaning for `MaterialKind::Creature` — unlike a tree's growth stage,
    /// "which creature owns this cell" has to round-trip through the cell.
    creatures: Vec<CreatureState>,
    /// Species data for organism-owned cells — see `organism.rs`. Loaded
    /// with the compiled-in set by default, same as `materials`; `App::new`
    /// overlays the assets directory the same way it does for materials.
    pub species: SpeciesRegistry,
    /// Backing storage for `organism_id`-owned organisms (the generational
    /// allocator issue #8 called for — see `Reports/organism-substrate-
    /// design.md` §6). `Cell::organism_id` encodes a 1-based slot index in
    /// its low 12 bits and a generation in its high 4 — see `encode_
    /// organism_id`/`decode_organism_id` below. A freed slot's index is
    /// pushed to `free_organism_slots` and its generation bumped on reuse,
    /// so a stale `organism_id` still held by some cell (a bug, not a
    /// normal case) resolves to `None` via `organism`/`organism_mut`
    /// rather than silently reading a *different*, unrelated organism's
    /// state once the slot is recycled.
    organisms: Vec<OrganismSlot>,
    free_organism_slots: Vec<u16>,
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
    /// §11: every chunk that changed state (checked on *both* sides of
    /// `end_sweep`, see `end_step`'s own comment) during *any* tick since a
    /// renderer last consumed this set via `take_touched_chunks`. Exists
    /// because a chunk's own `is_settled()` is a snapshot of one instant
    /// only — `main.rs`'s own frame loop can run `App::update` up to
    /// `MAX_TICKS_PER_FRAME` times before the next `App::draw`, and a
    /// chunk that goes active then settles again *within* that window
    /// would read as settled at draw time despite having visibly moved in
    /// between, leaving stale pixels behind. Accumulating across every
    /// `end_step` since the last render, rather than reading one snapshot
    /// at render time, is what closes that gap — found by a debug harness
    /// that (deliberately, to stress exactly this) called `App::update`
    /// 300 times before ever drawing again, and caught a settled pile of
    /// sand still rendering at its original mid-air position. A second,
    /// narrower gap (one write to an *already-settled* chunk missing by
    /// exactly one `end_step`, since a write only arms `pending_dirty` and
    /// the settled-before check alone can't see that promotion happening
    /// in the very call that's checking it) was caught by an independent
    /// review and closed the same way, checking both before and after.
    touched_chunks: std::collections::HashSet<ChunkCoord>,
    /// Cells the load walks in `load.rs` may still visit this frame,
    /// refilled to `load::MAX_LOAD_CELLS_PER_FRAME` by `scheduler::step`.
    ///
    /// Lives on `World` rather than being threaded through the scheduler
    /// because a structural check is dispatched one site at a time and the
    /// budget has to survive between them. Spent down rather than counted
    /// up, so a walk can hand `&mut` straight to it and stop when it hits
    /// zero without knowing what the ceiling was.
    pub load_budget: u32,
    /// Cumulative count of structural failures, by kind
    /// (`load::FailureMode`). Debug instrumentation, and deliberately not
    /// optional: a coherent falling slab and a scatter of loose grains are
    /// indistinguishable in a contact sheet, and so are the two failure
    /// modes -- one of which is "this was overloaded" and the other "this
    /// was never held". Read by `examples/filmstrip.rs` beside the image.
    pub structural_failures: FailureCounts,
    /// Whether rock with nowhere to go cracks in place instead of
    /// displacing. See `structural::crush_in_place`; `true` is the shipped
    /// behaviour.
    ///
    /// A switch and not a constant because this is a *look* question, and
    /// this project's answer to a look question is a runtime selector
    /// rather than an argument -- five grain modes behind one key settled
    /// in minutes what no amount of still images had. It is also the
    /// control that isolates the mechanism: a sweep only varies its knob,
    /// and anything that rode along with the change is in every data
    /// point, so being able to run the *same binary* with the rule off is
    /// what makes a before/after here a measurement rather than a memory.
    pub crush_confined: bool,
    /// Whether a roof is relieved by the arch that forms over the opening
    /// beneath it. See `load::arch_relief`; `true` is the shipped
    /// behaviour.
    ///
    /// A switch for the same reason `crush_confined` is one: it is the
    /// control that isolates the mechanism. A sweep only varies its knob,
    /// and anything that rode along with a change is in every data point --
    /// which has already read here as "the approach is wrong" when it was
    /// not. Being able to run the *same binary* both ways is what makes a
    /// before/after a measurement rather than a memory of an older build.
    pub arch_relief: bool,
    /// Whether a column is judged at the worst of its section rather than
    /// on its own single load path. See `load::evaluate_within`; `true` is
    /// the shipped behaviour.
    ///
    /// A switch for the same reason the two above are: this one redraws the
    /// stress field of every solid structure in the world, and "is the
    /// one-pixel line gone" is a question best answered by one binary run
    /// twice rather than by two builds an hour apart.
    pub section_share: bool,
    /// How far from something that was actually disturbed a structural
    /// failure is allowed to happen, in cells, and for how long. See
    /// `ChainMode`; `i32::MAX` is the shipped behaviour (no limit).
    ///
    /// # Why this is a policy and not a deletion
    ///
    /// The owner asked how simple "no chaining at all" would be. The
    /// obvious lever -- the `schedule_solid_neighbours` calls a failure
    /// makes -- was measured and is **inert**: switching it off produced
    /// bit-identical output on the big-strike scene. What actually
    /// propagates a collapse is the distance-relaxation wavefront, and
    /// that *is* the structural model: remove it and distances never
    /// update. So the only place to stand is at the far end, refusing a
    /// failure that is too far from anything that happened.
    ///
    /// It is a slider rather than a switch on purpose, because the owner
    /// has stated two opposed wants: "they chain too far and too much",
    /// and "collapse must be obvious and delayed, so the player can get
    /// supports in first" -- which is a description of chaining. One
    /// radius spans both, and which one is right is a question for the
    /// hand rather than for argument.
    pub chain_reach: i32,
    /// How long a disturbance keeps licensing failures near it, in frames.
    /// Generous by default: a cave-in that arrives a few seconds after you
    /// undermine something is the mechanic, not a bug.
    pub chain_window: u64,
    /// Where the world was last disturbed, and when. A small ring: only
    /// the most recent handful matter, since older ones fall outside
    /// `chain_window` anyway.
    pub disturbances: std::collections::VecDeque<(i32, i32, u64)>,

    /// Per-frame caches for the load walks (`load::Cache`).
    /// Cleared by `scheduler::step` each frame and again by
    /// `structural::tick` the instant a break mutates the grid, since both
    /// invalidate the support forest it summarises.
    pub load_cache: crate::sim::load::Cache,
    /// **This world's identity**, mixed into anything that should differ
    /// between worlds but be stable within one.
    ///
    /// Set by `worldgen::generate` from the spec's own seed; left at
    /// `DEFAULT_WORLD_SEED` for a hand-built world (every test, every
    /// harness scene), which is what keeps those reproducible without any
    /// of them having to think about it.
    ///
    /// Its first consumer is `plant::seed_genotype`. An individual plant's
    /// genotype is drawn from *this* plus the coordinate it germinated at,
    /// rather than from its `organism_id` — ids are assigned in planting
    /// order, so an id-keyed genotype makes a tree's character a property
    /// of the world's event history: plant one extra sapling anywhere
    /// earlier and every later plant in the world redraws. Position keying
    /// is stable under that, stable under save/load by construction (a save
    /// that restores the grid restores the genotypes), and still gives
    /// "same world, same trees", which `PLAN.md`'s determinism requirement
    /// wants.
    pub seed: u64,
}

/// The seed a world has when nothing has given it one. Arbitrary, fixed,
/// and deliberately not zero — a zero seed mixed into a hash tends to make
/// the first few draws correlate with the position alone.
pub const DEFAULT_WORLD_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// How many structural failures of each kind have fired, and how much
/// material each took. See `World::structural_failures`.
#[derive(Clone, Copy, Debug, Default)]
pub struct FailureCounts {
    pub overloaded: u32,
    pub overloaded_cells: u32,
    pub unsupported: u32,
    pub unsupported_cells: u32,
    /// Furthest a failure has ever been found from the cell whose check
    /// found it, in cells.
    ///
    /// Instrumentation for a decision, not a metric anyone needs at
    /// runtime. `Reports/prior-art-destruction.md` flags
    /// `ROOTWARD_CHECK_STEPS = 128` as having 7 Days to Die's exact bug
    /// shape -- a blow bringing down rock a hundred cells away, frames
    /// later, which players experienced as bases collapsing for no visible
    /// reason. The proposed fix (bound the walk by distance from what
    /// actually changed) contradicts that constant's own doc comment,
    /// which records that 16 was too small and left `scene=ligament`'s neck
    /// standing at a stress ratio of 1.87. So the question is empirical:
    /// how far do failures *actually* land from their trigger here?
    pub max_chain_reach: u32,
    /// The largest single failing region, in cells.
    ///
    /// The mean (`overloaded_cells / overloaded`) is not enough on its own:
    /// one 200-cell break averaged with fifty 1-cell ones reads as a
    /// respectable 5, and 1-cell failures are exactly the shape that
    /// produces dust, because `rigid::fracture` declines anything below
    /// `MIN_FRACTURE_CELLS` and falls through to per-cell conversion. So
    /// the pair -- mean and max -- is what says whether pieces or grit came
    /// out, and neither half says it alone.
    pub largest_failure: u32,
    /// Failures whose whole region was **confined** — not one cell of it
    /// touching air — and the cells they took.
    ///
    /// Instrumentation for a decision, and deliberately measured before
    /// the mechanism it is for. The owner's framing of what still looks
    /// wrong: *"it is stone in the middle of a mountain falling in on
    /// itself... in solid rock you should just have cracks that propagate
    /// and maybe break rock into small pieces that for the most part stay
    /// where they are."* Rock deep in a massif cannot displace, because
    /// there is nowhere for it to move.
    ///
    /// Whether that is worth building depends on whether it *happens*, and
    /// that is a count, not a picture: a collapse at a cliff edge and one
    /// eighty cells inside a mountain are the same grey rubble at the zoom
    /// a contact sheet is read at. So this asks the question first. If it
    /// stays at zero on every scene, the mechanism has nothing to fix.
    pub confined: u32,
    pub confined_cells: u32,
    /// The deepest any confined failure was buried, in cells from the
    /// nearest air. Separates "one row under a surface that is itself
    /// coming apart" from "the middle of a mountain", which is the only
    /// one of the two that is the reported artifact.
    pub deepest_confined: u32,
    /// Cells a crush actually wrote a fissure into.
    ///
    /// The "did it fire at all" counter, and it earned its place: a crush
    /// whose crack pattern was rewritten twice produced *bit-identical*
    /// images and counters both times, which reads exactly like "the
    /// mechanism is dead" and is indistinguishable, in a contact sheet,
    /// from cracks too fine to see at that zoom. An image cannot say
    /// whether the thing you built is what produced it.
    pub crushed_cells: u32,
}

impl FailureCounts {
    pub fn record_reach(&mut self, reach: u32) {
        self.max_chain_reach = self.max_chain_reach.max(reach);
    }

    pub fn record_confined(&mut self, cells: usize, depth: u32) {
        self.confined += 1;
        self.confined_cells += cells as u32;
        self.deepest_confined = self.deepest_confined.max(depth);
    }

    pub fn record(&mut self, mode: crate::sim::load::FailureMode, cells: usize) {
        self.largest_failure = self.largest_failure.max(cells as u32);
        match mode {
            crate::sim::load::FailureMode::Overloaded => {
                self.overloaded += 1;
                self.overloaded_cells += cells as u32;
            }
            crate::sim::load::FailureMode::Unsupported => {
                self.unsupported += 1;
                self.unsupported_cells += cells as u32;
            }
        }
    }
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
            chunk_bodies: Vec::new(),
            player: None,
            active_sites: BinaryHeap::new(),
            pending_structural_checks: std::collections::HashSet::new(),
            bodies: Vec::new(),
            free_body_slots: Vec::new(),
            body_index: HashMap::new(),
            creatures: Vec::new(),
            species: SpeciesRegistry::builtin(),
            organisms: Vec::new(),
            free_organism_slots: Vec::new(),
            fields_settled: false,
            touched_chunks: std::collections::HashSet::new(),
            load_budget: crate::sim::load::MAX_LOAD_CELLS_PER_FRAME,
            crush_confined: true,
            arch_relief: true,
            section_share: true,
            chain_reach: i32::MAX,
            chain_window: crate::sim::structural::CHAIN_WINDOW_FRAMES,
            disturbances: std::collections::VecDeque::new(),
            load_cache: crate::sim::load::Cache::default(),
            structural_failures: FailureCounts::default(),
            seed: DEFAULT_WORLD_SEED,
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
        // Mature organism cells are no longer on that schedule at all --
        // their upkeep runs here, once per organism. See
        // `plant::step_organisms`.
        super::plant::step_organisms(self);
    }

    /// Every live organism's encoded id.
    ///
    /// Collected rather than iterated in place because the caller needs
    /// `&mut World` to run each organism's pass.
    pub(crate) fn live_organism_ids(&self) -> Vec<u16> {
        self.organisms
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.state.is_some())
            .map(|(i, slot)| encode_organism_id((i + 1) as u16, slot.generation))
            .collect()
    }

    /// Advance every promoted liquid body by one frame — its own serial
    /// phase, after the CA sweep and before active sites (`app.rs`'s own
    /// comment on the call site has the frame-order reasoning; design doc
    /// §8a has why it must be serial rather than inside the parallel
    /// sweep). Since design doc §11 step 3: runs each live body's own
    /// `LiquidBody::step` (the persistent-flux pipe solver).
    ///
    /// Collects every live `BodyId` first, then takes/steps/restores one at
    /// a time — same take-then-restore reasoning as `absorb_liquid`
    /// (`LiquidBody::step` needs `&mut World` and `&mut LiquidBody`
    /// simultaneously), just over every body rather than one. Collecting
    /// the id list up front rather than iterating `self.bodies` directly
    /// means a body demoted mid-loop (a disturbance a solver's own
    /// rasterization triggers, say) doesn't invalidate the iteration —
    /// the next id in the list simply resolves to `None` and is skipped.
    pub fn step_liquid_bodies(&mut self) {
        let ids: Vec<BodyId> = self
            .bodies
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| slot.state.is_some().then_some(BodyId { index: index as u32, generation: slot.generation }))
            .collect();
        for id in ids {
            let Some(slot) = self.bodies.get_mut(id.index as usize) else { continue };
            if slot.generation != id.generation {
                continue;
            }
            let Some(mut body) = slot.state.take() else { continue };
            // Skipping `register_body_chunks` while a body stays asleep
            // avoids rebuilding a `HashSet` over its whole footprint every
            // frame for no reason (design doc §8c: "a sleeping body costs
            // nothing per frame"). But `try_extend` runs even while
            // asleep (so a sleeping body can still reclaim a neighbour),
            // and a successful claim can grow the footprint into a chunk
            // never touched before. Gating registration on the *pre-step*
            // sleep state alone (`was_asleep`) missed that case: a body
            // asleep going in that wakes via `try_extend` this frame
            // skipped registration entirely, silently desyncing
            // disturbance/demotion handling in the newly claimed chunk
            // (found by independent review). Register whenever the body
            // wasn't asleep on both sides of `step` — skip only the
            // steady-state case where it was asleep before and is still
            // asleep after, since nothing but `try_extend` can change a
            // sleeping body's footprint, and a no-op `try_extend` leaves
            // `asleep` untouched.
            let was_asleep = body.asleep;
            body.step(self);
            if !(was_asleep && body.asleep) {
                self.register_body_chunks(id, &body);
            }
            let stranded = body.columns() < 2;
            if let Some(slot) = self.bodies.get_mut(id.index as usize) {
                slot.state = Some(body);
            }
            // A body that has shed itself down to a single column hands the
            // rest back rather than sitting on it.
            //
            // `LiquidBody::step` bails at `columns() < 2` -- there are no
            // interfaces left to move flux across, so the solver has nothing
            // to do -- and before edge shedding could fire on an uncontained
            // body (`edge_with_room`) that was unreachable in practice. Now
            // it is the *normal* end state of a body spreading onto open
            // floor, and without this the leftovers strand: measured on a
            // 100-column basin, a promoted body walked itself down to one
            // column still holding 40,000 fill, forty cells of water stacked
            // in a single column that nothing would ever move again, and the
            // basin never levelled at all.
            if stranded {
                self.demote_body(id);
            }
        }
    }

    /// Queue a site to be checked by the scheduler once it's due. Used by
    /// `plant::plant_moss_seed`/`plant_tree` and by growth itself scheduling
    /// its own continuation.
    ///
    /// **The one canonical insertion point for `ActiveKind::StructuralCheck`
    /// dedup** (`pending_structural_checks`'s own doc). An independent
    /// review of the first version of this dedup found it only covered
    /// `structural::schedule_structural_check`'s own callers — `fire.rs`'s
    /// burnout fan-out builds `ActiveSite`s by hand and calls
    /// `CellSurface::schedule_active_site` directly, which for the serial
    /// path *is* `World::schedule_active_site` and for the parallel path
    /// reaches it anyway via `ChunkView`'s `pending_active_sites` queue and
    /// `parallel::run_pass`'s replay (`for site in outcome.pending_active_
    /// sites { world.schedule_active_site(site); }`) — so putting the
    /// check here, at the one point every external `StructuralCheck`
    /// insertion actually passes through regardless of caller, closes that
    /// gap for good rather than chasing each new call site individually.
    /// `structural::schedule_structural_check` no longer duplicates this
    /// check itself. Only `scheduler::step`'s own `produced_this_frame`
    /// loop (a tick rescheduling itself or a neighbour) is a genuinely
    /// separate insertion point — `world.active_sites` has already been
    /// taken out of `self` by the time that loop runs, so it can't route
    /// through here, and carries the identical check inline instead.
    pub fn schedule_active_site(&mut self, site: ActiveSite) {
        if matches!(site.kind, scheduler::ActiveKind::StructuralCheck) {
            if self.structural_check_pending(site.x, site.y) {
                return;
            }
            self.mark_structural_check_pending(site.x, site.y);
        }
        self.active_sites.push(Reverse(site));
    }

    /// Total pending active sites. The headline number for whether the
    /// scheduler's cost is actually proportional to "interesting cells"
    /// rather than world size — see the debug overlay.
    pub fn active_site_count(&self) -> usize {
        self.active_sites.len()
    }

    /// How many of `organism_id`'s cells currently read as `cell_type` —
    /// `Behavior::Grow`'s `max_active_tips` cap (`Reports/tree-rewrite-
    /// design.md` §5, the restoration of the old `MAX_TIPS_PER_TREE`/
    /// `MAX_ROOTS_PER_TREE` caps).
    ///
    /// **Counts the organism's own cell list, and used to scan the
    /// schedule heap — the difference is a bug the handoff kept a tripwire
    /// for.** `open-bugs-handoff.md` §3: a site being dispatched is not on
    /// the heap, so a heap scan undercounts by whatever is in flight, and
    /// the cap under-enforces. Measured as unreachable when written (tip
    /// retirement held one live tip per lineage, so a cap of 14 was never
    /// approached) and left unfixed with a tripwire test — which fired the
    /// session multiplicative crowding let crowded tips live: 19
    /// simultaneous tips against the cap of 14, exactly as the handoff
    /// predicted something eventually would.
    ///
    /// The fix is the one that handoff also predicted: Decision 2's cell
    /// list is maintained at the single `World::set` seam under both
    /// drivers (there are tests for the parallel paths), so a count over
    /// it sees every tip no matter what the scheduler is doing, including
    /// cells destroyed by fire mid-frame — the grid stays the single
    /// source of truth for the type, the list only says where to look.
    /// Cost is one `get` per organism cell per gate check, a few thousand
    /// reads per frame on a grown stand; if that ever shows up in a
    /// profile the next step is caching the type in the sidecar, which
    /// buys speed at the price of a second copy of the truth.
    pub fn organism_active_tip_count(&self, organism_id: u16, cell_type: super::organism::CellType) -> usize {
        let Some(state) = self.organism(organism_id) else {
            return 0;
        };
        state
            .cells
            .keys()
            .filter(|&&(x, y)| {
                let cell = self.get(x, y);
                cell.organism_id() == organism_id && super::organism::cell_type(cell.aux()) == Some(cell_type)
            })
            .count()
    }

    // --- crate-internal seams used only by `scheduler::step` and
    // `plant.rs` -----------------------------------------------------------

    /// Pop the next active site if it's due by `due` (`next_frame <= due`),
    /// or `None` if the heap is empty or its minimum isn't due yet (nothing
    /// after it, in a min-heap ordered by `next_frame`, can be due either).
    /// Clears the popped site's `pending_structural_checks` entry first when
    /// it's a `StructuralCheck` — before the caller dispatches it to `tick`,
    /// so a check that legitimately reschedules itself or a neighbour while
    /// running is a fresh request, not a stale one being silently dropped.
    ///
    /// Deliberately pops one at a time rather than taking the whole heap out
    /// (`scheduler::step`'s previous shape, via a since-removed `take_active_
    /// sites`/`set_active_sites` pair): taking the heap out left `self.
    /// active_sites` field genuinely empty for the whole dispatch loop, so
    /// any `schedule_active_site` call made *from inside* a dispatched tick
    /// (a growth behaviour scheduling a structural check around a new cell,
    /// say) silently wrote into that empty field and was then discarded when
    /// the real heap was written back over it at the end. Popping in place
    /// keeps `self.active_sites` live and correctly populated (holding every
    /// not-yet-dispatched-this-frame site) for the entire duration of every
    /// tick, so `schedule_active_site` — and anything that reads the heap,
    /// like `organism_active_tip_count` — works correctly no matter where in
    /// the call stack it's invoked from.
    pub(crate) fn pop_due_active_site(&mut self, due: u64) -> Option<ActiveSite> {
        let &Reverse(site) = self.active_sites.peek()?;
        if site.next_frame > due {
            return None;
        }
        self.active_sites.pop();
        if let scheduler::ActiveKind::StructuralCheck = site.kind {
            self.clear_structural_check_pending(site.x, site.y);
        }
        Some(site)
    }

    /// See `pending_structural_checks`'s own doc. `true` means a check for
    /// this exact position is already somewhere in the heap; the caller
    /// should skip scheduling a duplicate.
    pub(crate) fn structural_check_pending(&self, x: i32, y: i32) -> bool {
        self.pending_structural_checks.contains(&(x, y))
    }

    pub(crate) fn mark_structural_check_pending(&mut self, x: i32, y: i32) {
        self.pending_structural_checks.insert((x, y));
    }

    pub(crate) fn clear_structural_check_pending(&mut self, x: i32, y: i32) {
        self.pending_structural_checks.remove(&(x, y));
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

    /// Allocate a new organism. Checks `free_organism_slots` first (bumping
    /// the reused slot's generation) before ever growing `organisms` —
    /// nothing populates that list yet in this pass (no `free_organism`
    /// exists yet either; see the comment a few methods down for why), so
    /// this always takes the growth branch today, but the reuse path is
    /// real, correct code, not a stub — issue #8's actual fix, ready the
    /// moment a future caller needs it. Returns the encoded `organism_id`
    /// to stamp onto `Cell::organism_id`.
    pub(crate) fn push_organism(&mut self, species: SpeciesId) -> u16 {
        let state = OrganismState {
            species,
            cells: std::collections::HashMap::new(),
            root_cells: 0,
            shoot_cells: 0,
            collar_y: None,
            // The species mean until something germinates and draws — see
            // `OrganismState::genotype_draws`.
            genotype_draws: [0.0; organism::GENOTYPE_TRAITS],
        };
        if let Some(slot_index) = self.free_organism_slots.pop() {
            let slot = &mut self.organisms[(slot_index - 1) as usize];
            // Wraps at 16 generations (4 bits) rather than growing further
            // -- see `encode_organism_id`'s own doc for why this bound was
            // accepted rather than widening `Cell` a third time.
            slot.generation = (slot.generation + 1) & GENERATION_MASK;
            slot.state = Some(state);
            encode_organism_id(slot_index, slot.generation)
        } else {
            debug_assert!(
                self.organisms.len() < ORGANISM_INDEX_MASK as usize,
                "organism index would overflow the 12 bits Cell::organism_id reserves for it"
            );
            self.organisms.push(OrganismSlot { generation: 0, state: Some(state) });
            encode_organism_id(self.organisms.len() as u16, 0)
        }
    }

    /// `None` for `organism_id == 0` (no organism) or a stale id whose slot
    /// has since been reused by a different organism — the generation
    /// mismatch this whole scheme exists to catch, not a panic.
    pub(crate) fn organism(&self, organism_id: u16) -> Option<&OrganismState> {
        let (slot_index, generation) = decode_organism_id(organism_id);
        if slot_index == 0 {
            return None;
        }
        let slot = self.organisms.get((slot_index - 1) as usize)?;
        if slot.generation != generation {
            return None;
        }
        slot.state.as_ref()
    }

    /// Mutable counterpart to `organism`, same generational check.
    ///
    /// Added for `set`'s cell-list bookkeeping (`Reports/plant-substrate-v2-
    /// design.md` Decision 2, step 1). The generation test is what makes a
    /// stale `organism_id` still held by some cell resolve to `None` rather
    /// than silently editing an unrelated organism that has since been
    /// allocated the same slot.
    pub(crate) fn organism_mut(&mut self, organism_id: u16) -> Option<&mut OrganismState> {
        let (slot_index, generation) = decode_organism_id(organism_id);
        if slot_index == 0 {
            return None;
        }
        let slot = self.organisms.get_mut((slot_index - 1) as usize)?;
        if slot.generation != generation {
            return None;
        }
        slot.state.as_mut()
    }

    // `free_organism` (return a slot to `free_organism_slots`, the other
    // half of issue #8's actual fix) is not here yet, deliberately: no
    // species retrofitted so far needs it. Moss's `Divide` never
    // touches `OrganismState` after creation (its resource scalar lives
    // entirely in `Cell::aux`, not here), and detecting "this organism has
    // no cells left" cheaply needs a real anchor/tip list to search from
    // (`reachable_from_anchors`) or a live cell count — both real,
    // deliberately deferred work for the tree retrofit (which already
    // needs exactly this, generalizing `reclaim_if_tree_is_fully_dead`).
    // Adding either method now with no caller would be dead code by this
    // crate's own standard, not a head start; `decode_organism_id`'s
    // generation check is already fully exercised by `organism`'s own
    // tests above, so the one thing actually worth verifying now — that a
    // stale id can never silently alias a reused slot — has real coverage
    // regardless of when `free_organism` itself lands.

    // --- Liquid heightfield bodies (`Reports/liquid-heightfield-
    // design.md`, step 1 of §11's build order: the ownership substrate and
    // the promote/demote round trip, no solver yet) -----------------------

    /// Allocate a new promoted body's slot, from `free_body_slots` first
    /// (bumping the reused slot's generation) before ever growing `bodies`
    /// — the identical reuse-before-growth ordering `push_organism` above
    /// already established, for the identical reason.
    fn push_body(&mut self, body: LiquidBody) -> BodyId {
        if let Some(index) = self.free_body_slots.pop() {
            let slot = &mut self.bodies[index as usize];
            slot.generation = slot.generation.wrapping_add(1);
            slot.state = Some(body);
            BodyId { index, generation: slot.generation }
        } else {
            self.bodies.push(BodySlot { generation: 0, state: Some(body) });
            BodyId { index: (self.bodies.len() - 1) as u32, generation: 0 }
        }
    }

    /// `None` for a stale id whose slot has since been reused by a
    /// different body — the generation mismatch this whole scheme exists
    /// to catch, mirroring `organism`'s own doc above.
    pub(crate) fn body(&self, id: BodyId) -> Option<&LiquidBody> {
        let slot = self.bodies.get(id.index as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        slot.state.as_ref()
    }

    fn free_body(&mut self, id: BodyId) {
        if let Some(slot) = self.bodies.get_mut(id.index as usize) {
            if slot.generation == id.generation && slot.state.is_some() {
                slot.state = None;
                self.free_body_slots.push(id.index);
            }
        }
    }

    /// Attempt to promote the liquid body at `(x, y)` — `liquid::
    /// label_body` plus design doc §3b's validation, both already inside
    /// that one call. `None` if `(x, y)` isn't `Liquid`, the component
    /// fails validation, or it exceeds `liquid::MAX_BODY_CELLS`.
    ///
    /// Marks every claimed cell `FLAG_MANAGED` — the body's own columns
    /// (`LiquidBody::managed_positions`) and its bed/walls (`LiquidBody::
    /// container_positions`) — and moves no mass: `h[]` is read from cells
    /// that already existed (`liquid::label_body`'s own fill sum), not
    /// computed by moving anything, which is what makes promotion mass-free
    /// (design doc §2a/§9a's `promote` contract, §10's conservation table).
    pub fn promote_liquid_body(&mut self, x: i32, y: i32) -> Option<BodyId> {
        let scan = liquid::label_body(self, x, y)?;
        let flux = vec![0i32; scan.fill.len().saturating_sub(1)];
        let body = LiquidBody {
            material: scan.material,
            x0: scan.x0,
            top_y: scan.top_y,
            bed_y: scan.bed_y,
            h: scan.fill,
            flux,
            // Not quiescent by construction -- a fresh promotion always
            // gets at least one real solver pass before it could possibly
            // qualify, rather than assuming a just-promoted body happens
            // to already be flat (design doc §4a: quiescence is a
            // structural non-requirement for promotion, so nothing here
            // guarantees it).
            asleep: false,
            extend_cooldown_until: 0,
        };

        let managed: Vec<(i32, i32)> = body.managed_positions().collect();
        let container = body.container_positions();
        let touched_chunks: std::collections::HashSet<ChunkCoord> =
            managed.iter().chain(container.iter()).map(|&(px, py)| ChunkCoord::containing(px, py)).collect();

        let id = self.push_body(body);
        for coord in touched_chunks {
            self.body_index.entry(coord).or_default().push(id);
        }
        for (px, py) in managed.into_iter().chain(container) {
            let cell = self.get(px, py);
            self.set_owned(px, py, cell.with_managed(true));
        }
        Some(id)
    }

    /// Look up and demote whichever body owns `(x, y)`, if any — the write
    /// seam's own call (`set`'s doc), and also usable directly (tests,
    /// M10's chunk-unload path per design doc §8c). A no-op if nothing at
    /// `(x, y)` is currently managed.
    pub(crate) fn demote_body_at(&mut self, x: i32, y: i32) {
        if let Some(id) = self.find_body_at(x, y) {
            self.demote_body(id);
        }
    }

    fn find_body_at(&self, x: i32, y: i32) -> Option<BodyId> {
        let coord = ChunkCoord::containing(x, y);
        let candidates = self.body_index.get(&coord)?;
        candidates.iter().copied().find(|&id| self.body(id).is_some_and(|b| b.owns(x, y)))
    }

    /// Demote a body: clear `FLAG_MANAGED` on every cell it owns (its own
    /// columns and its container cells), remove it from `body_index`, and
    /// free its slot. No mass moves — the cells are already exactly the
    /// body's own state (design doc §2a/§5b), so demotion is this cheap and
    /// this exact by construction, not by care taken here.
    pub(crate) fn demote_body(&mut self, id: BodyId) {
        let Some(body) = self.body(id) else { return };
        let positions: Vec<(i32, i32)> = body.managed_positions().chain(body.container_positions()).collect();
        let touched_chunks: std::collections::HashSet<ChunkCoord> = positions.iter().map(|&(px, py)| ChunkCoord::containing(px, py)).collect();

        for coord in touched_chunks {
            if let Some(list) = self.body_index.get_mut(&coord) {
                list.retain(|&candidate| candidate != id);
                if list.is_empty() {
                    self.body_index.remove(&coord);
                }
            }
        }
        self.free_body(id);

        for (px, py) in positions {
            let cell = self.get(px, py);
            if cell.managed() {
                self.set_owned(px, py, cell.with_managed(false));
            }
        }
    }

    /// Absorb `fill` units into whichever body owns `(x, y)` — design doc
    /// §6b/§8b, `CellSurface::absorb_liquid`'s own doc has the caller-side
    /// contract. Silently does nothing if `(x, y)` doesn't resolve to a
    /// live body: the only caller (`update::transfer_liquid_vertical`)
    /// already checked `managed()` before deciding to absorb, so this
    /// should never actually miss in practice, but the check exists to be
    /// looked up rather than assumed, the same defensive shape `body`/
    /// `demote_body_at` already use for a stale or nonexistent `BodyId`.
    ///
    /// Takes the `LiquidBody` out of its slot for the duration — mirroring
    /// `scheduler::step`'s own take-then-restore shape for the identical
    /// reason: `rasterize_column` needs `&mut World` (to read/write cells,
    /// draw a shade) at the same time as `&mut LiquidBody`, which can't
    /// both be live simultaneously while the body is still borrowed *from*
    /// `self.bodies`.
    pub(crate) fn absorb_liquid(&mut self, x: i32, y: i32, fill: u32) {
        let Some(id) = self.find_body_at(x, y) else { return };
        let Some(slot) = self.bodies.get_mut(id.index as usize) else { return };
        if slot.generation != id.generation {
            return;
        }
        let Some(mut body) = slot.state.take() else { return };

        // Clamped into the body's own columns rather than skipped when it
        // falls outside them, because **the caller has already spent the
        // mass**. `update::transfer_liquid_vertical` writes `Cell::EMPTY`
        // over the source and credits the whole amount here in the same
        // call, precisely so a debit can never be separated from its credit
        // -- and the bounds check this replaces was doing exactly that
        // separating, silently destroying the fill.
        //
        // Reachable because `owns` is broader than `h`: it deliberately
        // covers a body's container cells, its bed and its walls, which sit
        // at `x0 - 1` and `x0 + columns()`. `find_body_at` resolving one of
        // those means the water landed on the body's *edge*, so crediting
        // the edge column is not a fudge to conserve mass -- it is where the
        // water went. Found by review.
        if body.h.is_empty() {
            self.register_body_chunks(id, &body);
            self.bodies[id.index as usize].state = Some(body);
            return;
        }
        let i = (x - body.x0).clamp(0, body.columns() as i32 - 1) as usize;
        body.h[i] += fill;
        // New mass to redistribute -- a sleeping body (design doc
        // §7d/§8c) must wake to actually do that, or it would sit
        // asleep with a pile absorption just dropped on it forever.
        body.asleep = false;
        body.rasterize_column(self, i);

        self.register_body_chunks(id, &body);
        self.bodies[id.index as usize].state = Some(body);
    }

    /// Register every chunk `body`'s current full footprint touches in
    /// `body_index`, without duplicating an already-present entry. Called
    /// after anything that can move a body's footprint — `absorb_liquid`'s
    /// growth and `LiquidBody::step`'s own solver-driven growth alike.
    ///
    /// Found by independent review, twice: `promote_liquid_body` only ever
    /// registers a body's *initial* footprint, but both `rasterize_column`'s
    /// growth (Step 2) and the solver's own redistribution (Step 3) can
    /// claim cells in a chunk that was never touched at promotion time (a
    /// tall enough column crosses a `CHUNK_SIZE` boundary). `find_body_at`
    /// — the one path both `absorb_liquid` and the write-seam's `demote_
    /// body_at` use to resolve `(x, y) → BodyId` — hard-fails on a chunk
    /// with no `body_index` entry at all, rather than falling back to a
    /// scan. Left unregistered: further absorption into the new chunk
    /// silently loses mass, and a disturbance there silently fails to
    /// demote. The first fix (in `absorb_liquid` alone) missed the second
    /// call path entirely — factored out here specifically so a third
    /// future caller can't reintroduce the same gap a third time.
    ///
    /// Recomputed from the body's current full footprint rather than
    /// tracked incrementally — same "not cached, bounded and rare" trade
    /// `container_positions` already makes. Does not remove entries for
    /// chunks a *shrinking* column no longer touches; a stale entry is
    /// harmless (`find_body_at`'s `body.owns(x, y)` check simply fails for
    /// it), just a wasted candidate check, not a correctness gap — noted in
    /// `PLAN.md` rather than fixed here.
    fn register_body_chunks(&mut self, id: BodyId, body: &LiquidBody) {
        let touched: std::collections::HashSet<ChunkCoord> =
            body.managed_positions().chain(body.container_positions()).map(|(px, py)| ChunkCoord::containing(px, py)).collect();
        for coord in touched {
            let list = self.body_index.entry(coord).or_default();
            if !list.contains(&id) {
                list.push(id);
            }
        }
    }

    /// Total live promoted bodies — for tests and the debug overlay, not
    /// consulted by any correctness-bearing path.
    #[cfg(test)]
    pub(crate) fn body_count(&self) -> usize {
        self.bodies.iter().filter(|slot| slot.state.is_some()).count()
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

    /// Raise moisture in a filled circle, capped at `1.0` — the mirror of
    /// [`Self::deplete_moisture`], and what rain writes.
    ///
    /// Capped rather than accumulating without limit: the channel is a
    /// saturation fraction, and a cell that has been rained on for an hour is
    /// wet, not a thousand times wet. Without the cap a long storm would
    /// leave ground that takes just as long to dry out afterwards, which
    /// reads as the rain having broken something.
    pub fn add_moisture(&mut self, cx: i32, cy: i32, radius: i32, amount: f32) {
        self.paint_field(cx, cy, radius, |c| c.moisture = (c.moisture + amount).min(1.0));
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
                    // Un-settle the tile as well as the world. The global flag
                    // above is what gets `field::step` past its early-out;
                    // this is what puts *this* tile in the awake set once it
                    // is there, instead of the solve falling back to every
                    // resident chunk.
                    tile.set_settled(false);
                }
            }
        }
    }

    // --- crate-internal seams used only by `field::step` -------------------

    pub(crate) fn fields_settled(&self) -> bool {
        self.fields_settled
    }

    /// Set the moisture floor covering the field block containing `(x, y)`.
    ///
    /// For `worldgen` only: this is how the saturated zone below the water
    /// table is laid down. Saturated *ground* cannot be liquid cells — a cell
    /// holds one material and there is no porosity — so the aquifer is a
    /// property of the field rather than of the grid, and this is the seam
    /// that writes it. See `field::FieldTile::moisture_floor`.
    ///
    /// Silently skips positions whose chunk is not resident, exactly as CA
    /// writes outside a loaded chunk are not materialised.
    pub(crate) fn set_field_moisture_floor(&mut self, x: i32, y: i32, floor: f32) {
        let (fx, fy) = field::field_coord_of(x, y);
        let (tile_coord, lx, ly) = field::tile_and_local(fx, fy);
        if let Some(tile) = self.fields.get_mut(&tile_coord) {
            tile.set_moisture_floor_local(lx, ly, floor);
            // A write from outside the solve, so the solve has to run at
            // least once more even if it had converged -- the same reason
            // `paint_field` clears this.
            tile.set_settled(false);
            self.fields_settled = false;
        }
    }

    /// The moisture floor at a world position, for tests and the inspector.
    pub fn field_moisture_floor(&self, x: i32, y: i32) -> f32 {
        let (fx, fy) = field::field_coord_of(x, y);
        let (tile_coord, lx, ly) = field::tile_and_local(fx, fy);
        self.fields.get(&tile_coord).map_or(0.0, |t| t.moisture_floor_local(lx, ly))
    }

    pub(crate) fn set_fields_settled(&mut self, settled: bool) {
        self.fields_settled = settled;
    }

    /// How many field tiles are still unconverged.
    ///
    /// The quantity `field::step` itself branches on, exposed because it is
    /// the only honest way to ask "has this disturbance gone away". Measuring
    /// the same question through summed pressure fails: the field's own
    /// background relaxation is an order of magnitude larger than a gust, so
    /// a disturbance that never disperses and one that disperses slowly are
    /// indistinguishable by pressure and perfectly distinct by this.
    /// See `weather::tests::a_gust_disperses`, which exists because of it.
    /// Test-only: nothing in the engine branches on the *count*, only on
    /// `fields_settled()`, and exposing it more widely would invite someone
    /// to build a per-frame decision on a full scan of the tile map.
    #[cfg(test)]
    pub(crate) fn unsettled_field_tiles(&self) -> usize {
        self.fields.values().filter(|t| !t.settled()).count()
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
            tile.set_settled(false);
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
            tile.set_settled(false);
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
        let old = self.write_cell(x, y, cell);
        // Disturbance detection at the one write seam every caller already
        // goes through (`Reports/liquid-heightfield-design.md` §5a): if the
        // cell just overwritten was `FLAG_MANAGED` — owned by a promoted
        // liquid body — demote its owner. Catches the brush, the eraser,
        // `explosion::trigger`, `fire.rs`'s neighbour ignition, and ordinary
        // CA movement without enumerating any of them by name — the same
        // "an enumeration that has to stay complete is the failure mode
        // this project keeps rediscovering" lesson `schedule_active_site`'s
        // own doc already states for the identical shape of problem.
        // `set_owned` is the one sanctioned bypass, for the body's own
        // rasterizer. Checked *after* the write, against `write_cell`'s own
        // returned old value, rather than a separate `self.get(x, y)`
        // before it -- reading first would mean two chunk-map lookups per
        // write instead of one, a real cost in the hottest function in the
        // engine (measured: ~1.7x the serial stress-scene worst frame
        // before this was folded into a single lookup).
        if old.managed() {
            self.demote_body_at(x, y);
        }
        // Organism cell bookkeeping, at the same seam and for the same
        // reason `managed()` above is checked here rather than at every
        // caller: `Reports/plant-substrate-v2-design.md`'s Decision 2 lists
        // a dozen creation and removal sites to hook (germinate, both of
        // Grow's children, the leaf spawn, Divide's child, thicken's write,
        // both planters, structural::break_free, fire's burnout, brush
        // erase), and warns that step 2a "is where the real bugs are".
        //
        // It does not need to be a list. Every one of those paths writes
        // through here, so hooking the write itself is complete by
        // construction -- which is this function's own recorded lesson,
        // stated a few lines above: "an enumeration that has to stay
        // complete is the failure mode this project keeps rediscovering."
        //
        // Guarded so the overwhelmingly common case -- neither cell belongs
        // to an organism -- costs one branch on a value already in hand.
        // This is the hottest function in the engine and the reason the
        // `managed()` check above reuses `write_cell`'s returned old value
        // rather than reading the cell a second time.
        //
        // The entry carries the cell's scalars (`OrganismCell`), so the
        // `was == now` fast path is doing real work beyond saving a lookup:
        // it is what makes an ordinary in-place rewrite -- a tip retiring
        // to `MatureBody`, `Photosynthesize` restamping the same cell --
        // *keep* its carbon instead of resetting it. A cell only gets a
        // fresh, zeroed `OrganismCell` when it genuinely changes hands,
        // which is what `a freshly divided cell should start at 0 resource,
        // not inherit any` asserts.
        self.reindex_organism_cell(x, y, old.organism_id(), cell.organism_id());
    }

    /// Move `(x, y)` from organism `was`'s cell list to organism `now`'s.
    ///
    /// Factored out of `set` because **`set` is not the only write seam.**
    /// `parallel::ChunkView::set` writes a same-chunk cell straight into its
    /// own `Chunk`, deliberately never touching `World::set` (see that
    /// function for why), so it queues the membership change and replays it
    /// through here after the pass — exactly the shape its `demotions`
    /// queue already uses for the same reason.
    ///
    /// **This gap was real and it was silent.** Decision 2 step 2a hooked
    /// `World::set` and recorded that doing so was "complete by
    /// construction", which is true of every *caller* but not of the
    /// parallel sweep, which does not call it. A falling seed moving inside
    /// one chunk therefore vanished from its own organism's cell list while
    /// staying in the grid. It went unnoticed because the list was
    /// deliberately behaviour-free at the time, and because the test
    /// guarding it runs `update::step` — the *serial* driver — so it could
    /// not observe `ChunkView` at all. `CLAUDE.md` says to test both
    /// drivers; this is what it costs not to.
    ///
    /// **A cell that moves gets a fresh, zeroed `OrganismCell`, and that is
    /// a real limitation rather than an oversight.** This seam sees a
    /// remove at one position and an insert at another; nothing tells it
    /// the two are the same cell relocating, so the scalars cannot ride
    /// along the way they used to when they lived in `Cell::aux` and
    /// travelled with the cell. It is correct today because a `Seed` is the
    /// only organism cell that moves (`relocated_seed`'s own doc: "every
    /// other organism cell is immovable") and a seed carries no carbon —
    /// `Germinate` has no resource gate. **The moment a carbon-carrying
    /// cell can move, this needs a move-aware seam**, not a second
    /// remove/insert pair.
    pub(crate) fn reindex_organism_cell(&mut self, x: i32, y: i32, was: u16, now: u16) {
        if was == now {
            return;
        }
        if was != 0 {
            if let Some(state) = self.organism_mut(was) {
                state.cells.remove(&(x, y));
            }
        }
        if now != 0 {
            if let Some(state) = self.organism_mut(now) {
                state.cells.insert((x, y), organism::OrganismCell::default());
            }
        }
    }

    /// The sidecar scalars for the organism-owned cell at `(x, y)`, or
    /// `None` if nothing there belongs to an organism.
    ///
    /// The read half of `Reports/plant-substrate-v2-design.md` Decision 2:
    /// callers that used to `unpack_aux(cell.aux()).1` come through here
    /// instead. Two lookups (organism slot, then position) where the old
    /// form was a shift and a mask -- which is why `transport` resolves its
    /// topology once per tick rather than calling this in its inner loop.
    pub fn organism_cell(&self, x: i32, y: i32) -> Option<&organism::OrganismCell> {
        let id = self.get(x, y).organism_id();
        self.organism(id)?.cells.get(&(x, y))
    }

    /// Mutable counterpart to `organism_cell`.
    ///
    /// Returns `None` rather than inserting when the cell is not registered:
    /// registration is `set`'s job and happens at the write that creates the
    /// cell, so a `None` here means the caller is writing a scalar to a cell
    /// that does not exist yet -- which was a silent no-op under the packed
    /// layout and should stay one rather than manufacturing an entry the
    /// grid scan would then flag as a phantom.
    pub fn organism_cell_mut(&mut self, x: i32, y: i32) -> Option<&mut organism::OrganismCell> {
        let id = self.get(x, y).organism_id();
        self.organism_mut(id)?.cells.get_mut(&(x, y))
    }

    /// Carbon at `(x, y)`, or `0.0` where there is no organism cell —
    /// the reading callers of the old packed field expect, since an
    /// unregistered or inert cell held a zeroed scalar field.
    pub fn carbon_at(&self, x: i32, y: i32) -> f32 {
        self.organism_cell(x, y).map_or(0.0, |c| c.carbon)
    }

    /// Canopy density at `(x, y)`, or `0.0` where there is no organism
    /// cell — "nothing has grown near here yet", which is the correct
    /// reading and not a sentinel to special-case.
    pub fn canopy_density_at(&self, x: i32, y: i32) -> f32 {
        self.organism_cell(x, y).map_or(0.0, |c| c.canopy_density)
    }

    /// The body's own sanctioned rasterizer write — bypasses `set`'s
    /// disturbance check, since this *is* the body moving its own cell, not
    /// something disturbing it. No production caller yet: step 1 of
    /// `Reports/liquid-heightfield-design.md`'s build order gives a
    /// promoted body no solver, so it never rasterizes anything after
    /// promotion — this exists now so the seam is already in place (and
    /// `promote_liquid_body`/`demote_body` already route their own flag
    /// writes through it) before a later step's solver needs it for real.
    pub(crate) fn set_owned(&mut self, x: i32, y: i32, cell: Cell) {
        self.write_cell(x, y, cell);
    }

    /// Writes the cell and returns whatever was there immediately before —
    /// `Cell::OUT_OF_BOUNDS` (never itself `managed()`, so `set`'s own check
    /// is still correct either way) if the write was dropped for being out
    /// of bounds. One chunk-map lookup total, shared between the write and
    /// the read `set`'s disturbance check needs — see that method's own
    /// comment for why this matters.
    fn write_cell(&mut self, x: i32, y: i32, cell: Cell) -> Cell {
        if !self.in_bounds(x, y) {
            return Cell::OUT_OF_BOUNDS;
        }
        let coord = ChunkCoord::containing(x, y);
        let reach = self.materials.get(cell.material).sweep_reach();
        let is_liquid = self.materials.kind(cell.material) == MaterialKind::Liquid;
        let chunk = self.chunks.entry(coord).or_insert_with(|| Chunk::new(coord));
        let old = chunk.get_world(x, y);
        chunk.set_world(x, y, cell, reach, is_liquid);
        self.touch_neighbours(x, y, coord);
        old
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

    /// Clear a cell's undercut flag once the sweep has visited it. Quiet for
    /// the same reason `clear_moved` above is.
    pub fn clear_undercut(&mut self, x: i32, y: i32) {
        if !self.in_bounds(x, y) {
            return;
        }
        if let Some(chunk) = self.chunks.get_mut(&ChunkCoord::containing(x, y)) {
            let cell = chunk.get_world(x, y).with_undercut(false);
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
    ///
    /// Delegates to `CellSurface::move_cell`'s default rather than
    /// duplicating it, so this and the generic sweep path (`update.rs`) can
    /// never silently diverge — `<Self as CellSurface>::move_cell` since an
    /// inherent method of the same name would otherwise shadow the trait
    /// one at the call site.
    pub fn move_cell(&mut self, fx: i32, fy: i32, tx: i32, ty: i32, revisited: bool) {
        <Self as CellSurface>::move_cell(self, fx, fy, tx, ty, revisited);
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
                // A raw material check, not `cell.is_empty()` -- this debug
                // brush's own question is "is there material here to
                // ignite," not "is this position available to use," so a
                // promoted liquid body's materially-empty-but-`FLAG_
                // MANAGED` container cell should be skipped the same way
                // any other empty cell is, not treated as occupied and
                // igniting nothing while incidentally demoting a nearby
                // body it merely brushed past.
                if cell.material == material::EMPTY || cell.is_burning() {
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
        self.paint_capsule_as(a, b, radius, material, density)
    }

    /// As `paint_capsule`, but places material as part of the **background
    /// mass** when `attached` (see `Cell::attached`).
    ///
    /// The brush lays down foreground by default, and that is the right
    /// default: material a player stacks has to hold itself up, which is
    /// what makes building a real constraint. But terrain is not built that
    /// way — a cave wall is braced by rock out of plane — so authoring
    /// terrain needs to say so, or every hand-made cavern behaves like a
    /// free-standing structure and collapses.
    ///
    /// Kept as a separate entry point rather than a parameter on
    /// `paint_capsule` so the dozens of existing callers, none of which want
    /// this, stay untouched.
    pub fn paint_capsule_as(
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
        let mut touched_structure = false;

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
                // A full random byte, not `below(shades)`. The low bits
                // still choose the palette entry exactly as before
                // (`cell_colour` takes `shade % palette.len()`), and the
                // high bits are otherwise unused -- which makes them the
                // one piece of per-cell entropy that survives a move, and
                // therefore the only thing `render::GrainMode::Cell` can
                // key grain on so the texture travels with the material.
                let shade = (self.rng.below(shades) + shades * self.rng.below(256 / shades.max(1))) as u8;
                // Background rock has to *join* background rock.
                //
                // `Cell::attached` means "backed by mass the slice cannot
                // show". A floating island of it is a claim the model has
                // no way to check and every way to be ruined by: attached
                // rock carries a twelvefold capacity bonus, so a detached
                // blob of it is very nearly indestructible terrain hanging
                // in mid-air. "Paint indestructible terrain anywhere,
                // unlimited" is the right tool for authoring a test scene
                // and the wrong one for a game about building things that
                // can fall down.
                //
                // So the brush extends the massif rather than conjuring
                // it: a cell becomes background only if it touches
                // background, bedrock, or the world edge (which reads as
                // bedrock via `Cell::OUT_OF_BOUNDS`). Anything else lands
                // as ordinary foreground and has to hold itself up. Terrain
                // grows from terrain, which is the same statement
                // `attached` was always making.
                //
                // Cheap now that C1 exists: material keyed into terrain
                // gets the bonus at its joint anyway, so the case this
                // used to be needed for is already served.
                // **Everything placed is intact.**
                //
                // `Cell::attached` used to mean "part of the background
                // massif", authorable only through a separate brush mode.
                // It now means *undamaged*, which is what a construction is
                // until something happens to it. Reported from play: "I
                // don't want my constructions to just immediately fall down
                // or to have to work at all to make sure they are
                // structurally stable, but I do want it to break
                // realistically."
                //
                // A **multiplier, never an exemption**, and that is the
                // whole design (`Reports/building-rethink.md` §3a). Intact
                // rock is still evaluated and can still fail; it carries
                // `attached_span_bonus` while it does. Exempting it instead
                // would make one chip level a castle -- a structure
                // standing only by exemption has no answer the moment
                // anything asks, so the cascade reaches everything. With a
                // multiplier the ring behind a wound is judged against a
                // real capacity, so a chunky wall holds and an
                // over-reaching span does not, and a collapse stops where
                // the structure is genuinely sound.
                //
                // Damage revokes it, and every destructive verb already
                // does: `structural::detach_exposed_neighbours` for digging
                // and blasts, `detach_around_crack` for every crack a blow
                // scores, `rigid::strike` over its chip zone. That is why
                // this is not the "everything the player builds is
                // indestructible" failure that killed four earlier support
                // models -- in each of those, nothing ever revoked it.
                self.set(x, y, Cell::new(material, shade).with_attached(material != material::EMPTY));
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
                if !placed_structural && !erased_structural {
                    continue;
                }
                self.schedule_structural_check_around(x, y);
                // Cutting rock costs its neighbours their backing, which is
                // what lets mining produce anything at all -- see
                // `structural::detach_exposed_neighbours`. Erasing only:
                // *placing* material must not strip the attachment of the
                // terrain it was placed against.
                if erased_structural {
                    super::structural::detach_exposed_neighbours(self, x, y);
                }
                touched_structure = true;
            }
        }
        // One converged pass over what the stroke touched, rather than
        // letting a reactive wavefront climb through it a cell per five
        // frames. See `structural::relax_region` for why a stroke needs
        // this and generated terrain never did. Margin covers the cells
        // just outside the brush whose own distance the new material
        // changes, and `DETACH_DEPTH`'s loosened band on an erase.
        if touched_structure {
            const MARGIN: i32 = 4;
            let region = Rect::new(
                a.0.min(b.0) - r - MARGIN,
                a.1.min(b.1) - r - MARGIN,
                a.0.max(b.0) + r + MARGIN,
                a.1.max(b.1) + r + MARGIN,
            );
            super::structural::relax_region(self, region);
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
        let touched = &mut self.touched_chunks;
        for chunk in self.chunks.values_mut() {
            let was_settled = chunk.is_settled();
            chunk.end_sweep();
            let settled_now = chunk.is_settled();
            // Checked on *both* sides of `end_sweep`, not just before it —
            // an independent review found that checking only `!was_settled`
            // misses a chunk that was already fully settled (`dirty` and
            // `pending_dirty` both `None`) and then received exactly one
            // out-of-sweep write since the previous `end_step` (organism
            // growth via `step_active_sites`, an explosion, a structural
            // collapse, a landing free particle, a hot-reload's `wake_all`).
            // Such a write only ever sets `pending_dirty` -- `was_settled`,
            // read *before* `end_sweep` promotes it, still sees the old
            // `dirty == None` and reports settled, so `!was_settled` alone
            // stays false for this exact tick even though `end_sweep` right
            // above just made the chunk genuinely active. Confirmed via a
            // temporary test forcing exactly this sequence (one write, one
            // `end_step`) before this fix, then reverted once fixed.
            // `!was_settled` alone still matters too -- it is what catches
            // the opposite transition, a chunk that *was* active and
            // settles again this very tick (finished falling, burned out),
            // which `settled_now` alone would miss since by definition it's
            // `true` right when the render actually needs to see the change.
            if !was_settled || !settled_now {
                touched.insert(chunk.coord);
            }
            if !was_settled && settled_now {
                chunk.recompute_reach(|cell| materials.get(cell.material).sweep_reach());
                chunk.recompute_has_liquid(|cell| materials.kind(cell.material) == MaterialKind::Liquid);
            }
        }
    }

    /// Every chunk touched by any tick since the last call to this method
    /// — see `touched_chunks`'s own doc. `Renderer::draw` (via `App::draw`)
    /// is the one real caller, once per frame; draining rather than only
    /// reading is what makes "since the last call" true without the caller
    /// needing to remember anything itself.
    pub fn take_touched_chunks(&mut self) -> std::collections::HashSet<ChunkCoord> {
        std::mem::take(&mut self.touched_chunks)
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
    fn clear_undercut(&mut self, x: i32, y: i32) {
        World::clear_undercut(self, x, y)
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
    fn field_wind_at(&self, x: i32, y: i32) -> (f32, f32) {
        let f = World::field_at(self, x, y);
        (f.vx, f.vy)
    }

    #[inline]
    fn frame(&self) -> u64 {
        self.frame
    }

    #[inline]
    fn schedule_active_site(&mut self, site: ActiveSite) {
        World::schedule_active_site(self, site)
    }

    #[inline]
    fn absorb_liquid(&mut self, x: i32, y: i32, fill: u32) {
        World::absorb_liquid(self, x, y, fill)
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

    // --- §11: touched_chunks --------------------------------------------

    #[test]
    fn a_single_out_of_sweep_write_to_a_settled_chunk_is_touched_after_one_end_step() {
        // An independent review of §11's render optimization found this
        // exact gap: `World::set` only ever arms `pending_dirty`, and the
        // old `end_step` checked settledness *before* `end_sweep` promoted
        // it -- so a chunk that was fully settled (both `dirty` and
        // `pending_dirty` already `None`), then received exactly one write
        // from outside the sweep (organism growth, an explosion, a
        // structural collapse, a landing particle, none of which are
        // gated on the cursor being over the window the way painting is),
        // would not appear in `take_touched_chunks` until a *second*
        // `end_step` -- one whole tick later than the write that actually
        // changed its pixels. Confirmed via revert to fail against the
        // pre-fix `!was_settled`-only check.
        let mut w = test_world();
        w.end_step();
        w.end_step(); // fully settled
        w.take_touched_chunks(); // drain the initial construction-time batch
        assert!(w.take_touched_chunks().is_empty(), "test setup should start with nothing touched once drained");

        w.set(10, 10, Cell::new(material::STONE, 0));
        w.end_step();

        let touched = w.take_touched_chunks();
        assert!(!touched.is_empty(), "a single write to a settled chunk must be visible after exactly one end_step, not two");
    }

    #[test]
    fn a_chunk_that_finishes_settling_this_very_tick_is_still_touched() {
        // The opposite transition, already covered before this exact test
        // was added but worth pinning down explicitly: a chunk that *was*
        // active and settles again on this tick (nothing new pending)
        // must still be reported touched -- `settled_now` alone would miss
        // it, since by the time anyone checks, the chunk already reads
        // settled.
        let mut w = test_world();
        w.set(10, 10, Cell::new(material::STONE, 0));
        w.end_step(); // promotes pending_dirty -> dirty; not yet settled
        w.take_touched_chunks();

        w.end_step(); // nothing new written; this call settles it
        let touched = w.take_touched_chunks();
        assert!(!touched.is_empty(), "a chunk settling on this exact tick must still be reported touched");
    }

    #[test]
    fn organism_ids_round_trip_and_encode_a_nonzero_generation() {
        let mut w = test_world();
        let species = SpeciesId(0);
        let id = w.push_organism(species);
        assert_ne!(id, 0, "0 is reserved for \"no organism\"");
        assert_eq!(w.organism(id).unwrap().species, species);
    }

    #[test]
    fn organism_id_zero_is_always_none() {
        let w = test_world();
        assert!(w.organism(0).is_none());
    }

    #[test]
    fn two_organisms_get_distinct_ids_and_each_resolves_to_its_own_species() {
        // Exercises `decode_organism_id`'s generation check indirectly:
        // two freshly-allocated organisms both start at generation 0 (same
        // decoded generation), so this only passes if they also land on
        // different slot indices -- the actual `organism_id` values must
        // differ, and each must resolve back to the species it was created
        // with, not the other's.
        let mut w = test_world();
        let species_a = SpeciesId(0);
        let species_b = SpeciesId(1);
        let a = w.push_organism(species_a);
        let b = w.push_organism(species_b);
        assert_ne!(a, b, "two live organisms must not share an id");
        assert_eq!(w.organism(a).unwrap().species, species_a);
        assert_eq!(w.organism(b).unwrap().species, species_b);
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
    fn a_chunk_woken_from_beyond_its_own_reach_counts_as_settled() {
        // The other half of the test above. Marking that neighbour dirty and
        // then giving it no sweep region is the *right* answer to "how much
        // of it should be re-examined" (issue #3) — but it used to be the
        // wrong answer to "is it awake": the chunk reported active forever
        // while never actually being swept, so the world could not sleep and
        // the overlay's awake count was inflated by chunks with provably
        // nothing to do. Measured at 3 such chunks under the parallel driver
        // on the seam-cliff scene (`update.rs`'s `seam_cliffs`).
        //
        // Fixed by defining `Chunk::is_settled` in terms of `sweep_region`
        // rather than `dirty` — see its own doc for why clamping the dirty
        // mark into the chunk's bounds instead was tried and reverted.
        let mut w = World::new(Rect::new(0, 0, 255, 127));
        w.end_step();
        w.end_step();
        assert_eq!(w.active_chunk_count(), 0);

        w.set(32, 32, Cell::new(material::SAND, 0));
        w.end_step();

        assert!(w.sweep_region(ChunkCoord::new(1, 0)).is_none());
        assert!(
            w.chunk(ChunkCoord::new(1, 0)).unwrap().is_settled(),
            "a chunk with no sweep region is not awake -- it has provably nothing to do"
        );
        // Only the chunk that actually holds the write is active.
        assert_eq!(w.active_chunk_count(), 1);
        assert!(
            w.chunks().all(|c| c.is_settled() || c.sweep_region().is_some()),
            "a chunk counted awake must have something to sweep"
        );
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
