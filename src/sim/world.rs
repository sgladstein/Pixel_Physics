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
use super::decay;
use super::field::{self, FieldCell, FieldTile, FIELD_SCALE};
use super::liquid::{self, LiquidBody};
use super::material::{self, MaterialId, MaterialKind, MaterialRegistry};
use super::organism::{self, OrganismState, SpeciesId, SpeciesRegistry};
use super::pheromone::{Channel, Pheromones};
use super::rng::Rng;
use super::scheduler::{self, ActiveSite};
use super::surface::CellSurface;

/// Bits of `Cell::organism_id` given to the slot index (the rest, high 4
/// bits, are generation). 4095 concurrently-live organisms — generous for
/// anything this engine plays at real-time rates.
///
/// **The bound is enforced in release, not by a debug assertion.** It used
/// to be the latter, and `encode_organism_id` below does not mask, so a
/// 4,096th slot index set bit 12 — the generation's low bit — and the new
/// organism silently *became* an existing live one. `push_organism` now
/// refuses the birth and counts it (`World::organisms_refused`) instead.
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

/// Per-verb creature counters. Printed beside every scene.
///
/// `trips_completed` is the one that proves the *loop* rather than its
/// parts: a colony can move, deposit, dig and drop convincingly while never
/// once completing nest -> food -> nest with cargo.
#[derive(Default, Clone, Copy, Debug)]
pub struct CreatureStats {
    pub spawned: u64,
    pub moves: u64,
    pub moves_blocked: u64,
    /// Heading re-rolls — the tumble half of run-and-tumble. High is not a
    /// fault: it is what a creature does while it is looking for a
    /// gradient, and the ratio against `moves` is the readout on whether
    /// the colony is searching or commuting.
    pub tumbles: u64,
    pub falls: u64,
    pub eats: u64,
    pub pickups: u64,
    pub digs: u64,
    pub drops: u64,
    /// Drops that happened at the nest — food actually delivered home.
    /// **The number that proves the loop rather than its parts.**
    pub deliveries: u64,
    /// **Not a trip counter, and not a sessility guard — read
    /// `forage_trips` for either.** It increments on any move made while
    /// nest-adjacent, guarded on `OrganismState::since_nest > 0`; but
    /// `since_nest` is bumped unconditionally every tick, so that guard is
    /// false exactly once in a creature's life and this counts *loitering*.
    /// One ant, a nest and no food scores `moves 648, nest_visits 389`.
    ///
    /// Kept because the ratio against `moves` is still a real readout — an
    /// immobile colony drives it toward 1 and a ranging one toward 0 — and
    /// because deleting it would silently change every scene that prints
    /// it. What it is not is "arrivals at the nest after having been away",
    /// which is what it used to claim and what `forage_trips` now measures.
    pub nest_visits: u64,
    /// **Round trips: excursions that got at least `FORAGE_TRIP_MIN` cells
    /// from home and came back.** The thing `nest_visits` was believed to
    /// be counting and never was.
    ///
    /// Booked at the moment of nest contact, off
    /// `OrganismState::forage_max` — a spatial depth that re-anchors on
    /// every contact, so neither loitering nor `tick_interval` can inflate
    /// it. See that field for why the obvious `since_nest` fix is unsound.
    pub forage_trips: u64,
    /// Summed depth of the trips in `forage_trips`, in cells. `/
    /// forage_trips` is the mean foraging range — **the quantity that was
    /// missing, and the reason no foraging change could be judged.**
    pub forage_depth_sum: u64,
    /// Deepest single excursion any creature has made, in cells. Bounds the
    /// mean: a colony with a good mean and a small max is commuting a fixed
    /// route, which is a different animal from one that ranges.
    pub forage_depth_max: u64,
    /// **The excursion-depth profile, and the reason this metric does not
    /// live or die on one threshold.** Bucket `i` counts excursions that
    /// reached at least `FORAGE_REACH_BUCKETS[i]` cells from home — a
    /// cumulative distribution, so it is monotonically non-increasing and
    /// bucket 0 is every excursion there was.
    ///
    /// A single count needs a bar, and a bar set from an aspiration is how
    /// this project gets numbers that cannot fail for the right reason. The
    /// profile needs none: an immobile colony is a spike in bucket 0 that
    /// vanishes by bucket 2, and a ranging one carries weight out to the
    /// distance of whatever it is ranging *to*. That shape is the readout,
    /// and it is what `forage_trips`'s bar was then set from.
    ///
    /// It also separates two colonies a mean cannot: a hundred short hops
    /// and ten real trips average the same as a hundred medium ones.
    pub forage_reach: [u64; 8],
    pub deaths: u64,
    /// Creatures that lost a body cell and survived it.
    pub injuries: u64,
}

/// Where every joule went. See `World::energy_ledger`.
///
/// Two stocks, not one. **Live** is the energy inside creatures; **meat**
/// is the energy standing in the world as corpse cells (and in whatever a
/// carrier is holding). Every account below is a monotone counter, and each
/// one is labelled as a *source* (value created out of nothing), a *sink*
/// (value destroyed) or a *transfer* (value moved between the two stocks).
///
/// ```text
/// live = granted + harvested_plant + harvested_corpse + overdrawn
///        - metabolized - moved - synapse_tax - stored_in_meat - dissipated
///
/// meat = stamped + stored_in_meat - harvested_corpse - meat_lost
/// ```
///
/// # What this can and cannot catch, because the difference has been
/// # misread once
///
/// It catches **charges that do not land**: a cost debited from the ledger
/// but never taken off a creature, or vice versa. That is a real class of
/// bug and the first identity finds it immediately.
///
/// It **cannot be a conservation law**, and reading a balanced census as
/// "energy is conserved" is a mistake. `granted`, `stamped` and
/// `harvested_plant` are genuinely free — the sun is the largest source in
/// the world by far and lives entirely outside these numbers, and nothing
/// books photosynthesis yet.
///
/// **What changed at S3, and why it matters more than the tidiness.** The
/// old ledger had one `eaten` account and it was a free term *defined as
/// whatever happened*: `eat_energy` was a constant of the **eater**, so
/// when a beetle bit an ant, `eaten` grew by the beetle's number, the
/// victim's remainder was written off, both were booked, the identity held,
/// and 300 joules were conjured (§13l). `harvested_corpse` is not free: it
/// is matched, joule for joule, by meat that was booked into `stamped` when
/// the animal was built or into `stored_in_meat` when it died. That is what
/// makes the second identity above worth asserting, and it is the property
/// evolution actually needs (P-20): **no lineage may extract unbounded
/// energy from a cycle it controls.**
///
/// `meat_lost` **is** an account here as of 2026-08-23. It was not, and the
/// meat identity was an upper bound rather than an equality because nothing
/// hooked the seam a corpse is destroyed through. Three of the four paths
/// that entry named are now booked — fire burning a corpse to ash, an
/// explosion consuming one, the brush erasing one — and the fourth (decay)
/// does not exist: `corpse.ron` declares no `decays_into`, so a corpse does
/// not rot, and the day it does is the day that hook is needed.
/// `creature::tests::the_standing_meat_never_exceeds_what_was_put_into_it`
/// still asserts a `<=`, because meat riding a `Particle` between a throw
/// and a landing is standing nowhere; what closes exactly is
/// `a_destroyed_corpse_is_booked_rather_than_forgotten`.
#[derive(Default, Clone, Copy, Debug)]
pub struct EnergyLedger {
    /// **Source.** Metabolic energy created at spawn — a creature's
    /// `start_energy`, the pool it can actually spend.
    pub granted: f64,
    /// **Source.** Structural energy created at spawn: `body_energy` for
    /// every cell of the body. The animal can never spend this; it exists
    /// so that a *starved* creature, dead at exactly 0, still leaves food
    /// behind. Booked separately from `granted` because it is never part of
    /// the live stock — it goes straight into meat when the animal dies.
    pub stamped: f64,
    /// **Source.** Eating something whose worth comes from its material:
    /// leaf, moss, seed, a live animal's flesh. Free until plants book
    /// photosynthesis into the same accounts.
    pub harvested_plant: f64,
    /// **Transfer**, out of the meat stock and into the live one. Eating a
    /// cell that carries its own worth in `Cell::aux`.
    pub harvested_corpse: f64,
    pub metabolized: f64,
    pub moved: f64,
    pub synapse_tax: f64,
    /// **Transfer**, out of the live stock and into meat: what a creature
    /// still had in the bank when it died, written into its corpse cells
    /// alongside the stamp. Was `died_holding`, and the rename is the
    /// point — it is not destroyed, it is what makes a fresh kill better
    /// eating than carrion.
    pub stored_in_meat: f64,
    /// **Sink.** Leftover energy with nowhere to go: a creature died in a
    /// world with no `corpse` material compiled in, so there was nothing to
    /// write the worth into. Should read 0 in every real scene; it is here
    /// so that the case does not silently unbalance the live identity.
    pub dissipated: f64,
    /// **A correction, and the only term that adds back.** Charges that
    /// landed on a creature which could not pay them: an animal dies *at*
    /// zero in the accounting but arrives there having been debited past
    /// it, so its last tick of metabolism came out of an empty bank.
    ///
    /// Found by `the_standing_meat_never_exceeds_what_was_put_into_it` the
    /// first time the live identity was ever asserted rather than printed
    /// — 2.16 joules across twelve ants over 40,000 frames, which is small,
    /// real, and exactly the kind of free term that becomes an attractor
    /// the moment something can select on it. Booked rather than clamped
    /// away at the charge sites, because a creature that overshoots zero
    /// by a tick is honest behaviour and pretending it did not is how a
    /// counterweight constant gets born.
    pub overdrawn: f64,
    /// **Sink.** Meat destroyed rather than eaten: the worth stamped into a
    /// `worth_in_aux` cell that something removed from the world without an
    /// animal getting it. Fire burning a corpse to ash, an explosion
    /// consuming one, the brush erasing one.
    ///
    /// Booked at the few call sites that destroy such a cell, **never in
    /// `World::set`** — that is per-cell hot-path work on the sweep's
    /// busiest seam, and `CLAUDE.md`'s "guard hot-path work at the call site
    /// that already has the data" applies exactly: each of those sites has
    /// already read the outgoing cell for its own reasons, so the hook costs
    /// a `Vec` index on a branch that was taken anyway.
    ///
    /// **This is what turns `max_standing_meat` from a hope into a bound.**
    /// Before it existed, meat could quietly go missing and every guard
    /// passed: the meat identity was a `<=`, and `creature_biomass` is
    /// asserted monotone non-increasing, which a loss also satisfies.
    ///
    /// **The other seam is still open and is deliberately not this one.**
    /// Living flesh carries a stamp that nothing pays for, because its sink
    /// — a parent paying stamps for a child — does not exist until S6
    /// reproduction (`Reports/creature-evolution-plan.md` §2.3, "One seam
    /// left open"; `creature.rs`'s `food_value` and `ant.ron`'s
    /// `food_energy == body_energy` equality are the load-bearing pieces).
    /// Close that one *with* S6, not here: without the sink there is nothing
    /// for the booking to balance against. The two are recorded together on
    /// purpose so whoever finds one finds the other.
    pub meat_lost: f64,
}

impl EnergyLedger {
    /// What the live population's total energy should equal.
    pub fn expected_live_total(&self) -> f64 {
        self.granted + self.harvested_plant + self.harvested_corpse
            - self.metabolized
            - self.moved
            - self.synapse_tax
            - self.stored_in_meat
            - self.dissipated
            + self.overdrawn
    }

    /// The most meat that can be standing in the world.
    ///
    /// **Was an upper bound and is now a real one**, because `meat_lost`
    /// books the destruction paths that used to make it a hope: every joule
    /// of meat was either stamped in, eaten out, or destroyed, and all three
    /// are now accounts. Two things keep it a `<=` rather than an `==`, and
    /// they arrived on different branches, so neither's author saw the
    /// other's: meat in flight — a corpse cell riding a `Particle` between
    /// the throw and the landing is standing nowhere — and S5's digestive
    /// loss, where a mismatched gut removes a full cell of meat from the
    /// world while `harvested_corpse` is credited only the filtered yield
    /// (`creature.rs`'s `diet_yield`). Both are one-directional, so the
    /// bound stays sound; do not tighten it to an equality without a sink
    /// for each.
    pub fn max_standing_meat(&self) -> f64 {
        self.stamped + self.stored_in_meat - self.harvested_corpse - self.meat_lost
    }

    /// The worth of a cell about to be destroyed, or `None` if destroying it
    /// loses no meat.
    ///
    /// The shared predicate behind every `meat_lost` booking, so the four
    /// call sites cannot drift apart on what counts. Deliberately mirrors
    /// `creature::food_value`'s gate — `worth_in_aux` **and** a non-zero
    /// stamp — because the quantity being destroyed has to be the same
    /// quantity an animal would have eaten. An unstamped corpse (`aux == 0`,
    /// which `fire.rs`'s burnout writes deliberately) is worth the material
    /// fallback to an eater but books nothing here: it never had a stamp to
    /// lose, and charging the fallback would invent a sink out of a
    /// material default.
    pub fn meat_worth_of(materials: &MaterialRegistry, cell: Cell) -> Option<f64> {
        (materials.get(cell.material).worth_in_aux && cell.aux() != 0).then(|| cell.aux() as f64)
    }
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
    /// **World time**, and deliberately not the same thing as `frame`.
    ///
    /// `frame` is the physics clock: one tick, one CA sweep, and the
    /// definition of how fast anything falls. `clock` is how fast the world
    /// *ages* — the length of a day and the pace of growth — and both of its
    /// knobs default to the historical behaviour exactly. See
    /// `sim::clock`'s module doc for why the two are separable at all.
    pub clock: crate::sim::clock::Clock,
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
    /// Where water crosses the plane of the world — see `spring.rs`'s
    /// module doc for the design and the off-plane-flux decision it
    /// implements. Plain `Vec`s in insertion order, the `chunk_bodies`
    /// determinism reasoning; registered via `add_spring`/`add_drain`
    /// (worldgen's placement pass later, harnesses and `viewshot` today).
    pub springs: Vec<crate::sim::spring::Spring>,
    pub drains: Vec<(i32, i32)>,
    /// Spring/drain flow accounting — the counter that gets printed next
    /// to every waterfall image.
    pub spring_ledger: crate::sim::spring::SpringLedger,
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
    /// The same dedup index, for `ActiveKind::Evaporate`, and needed for a
    /// reason that is about correctness rather than cost.
    ///
    /// The CA sweep asks for an evaporation site every frame that a settling
    /// liquid cell fails to move with air above it, so the number of raw
    /// requests a body produces is proportional to **how long it stays awake
    /// settling**. That is exactly the quantity that made the reverted,
    /// sweep-driven version of this mechanic take a lake apart faster than a
    /// puddle (`Reports/weather-handoff.md` §1): a big body settles for
    /// longer, so it would accumulate proportionally more duplicate sites
    /// and evaporate proportionally faster, re-importing the size dependence
    /// the whole design exists to avoid. Kept separate from the structural
    /// set rather than merged into one keyed on kind, so the structural
    /// path's behaviour is untouched.
    ///
    /// Kept in lockstep with `active_sites` the same way: marked in
    /// `schedule_active_site` only when not already present, cleared in
    /// `pop_due_active_site` before the tick runs, so a site that
    /// reschedules itself is a fresh request rather than a dropped one.
    pending_evaporation: std::collections::HashSet<(i32, i32)>,
    /// The same dedup index again, for `ActiveKind::Dissipate`, and
    /// load-bearing for the same reason as `pending_evaporation` directly
    /// above rather than merely for cost: the CA sweep asks for a
    /// dissipation site on **every frame** a gas cell fails to move, so
    /// without this the number of rolls a trapped cell gets each second
    /// would be proportional to how long its chunk happened to stay awake
    /// after it settled — smoke in a busy crater would fade faster than the
    /// same smoke in a quiet one, which is a size/activity dependence
    /// nothing about the mechanic wants.
    ///
    /// A third set rather than one keyed on kind, matching the choice made
    /// for `pending_evaporation`: the existing paths' behaviour stays
    /// untouched.
    pending_dissipation: std::collections::HashSet<(i32, i32)>,
    /// The topmost row of *ground* in each column, indexed from
    /// `bounds.min_x`, recorded once and never revised. `i32::MAX` for a
    /// column that held no ground at all; empty until `freeze_sky_surface`
    /// has run.
    ///
    /// **This is the definition of "underground", and it deliberately does
    /// not follow the world.** Everything below a column's entry is inside
    /// the ground as far as anything looking at the world is concerned, and
    /// digging, building, collapsing and growing all leave it exactly where
    /// it was. That is the point: a tunnel you dig stays a tunnel however
    /// wide you make it, and a plank you lay across a gap does not turn the
    /// air under it into a cave.
    ///
    /// **Every attempt to infer this from the shape of the world has been
    /// wrong in a new way**, which is what makes storing it worth the
    /// kilobytes (see below for how many, at the size that ships now).
    /// Measured on the version this replaced, which took the
    /// topmost non-empty cell and then patched up anything with higher
    /// ground within six columns either side: one floating cell put twenty
    /// rows of cave under it, a plank of *any* width from one cell to fifty
    /// did the same, and a dug shaft flipped from tunnel to open daylight
    /// between twelve and thirteen cells wide — which is exactly the
    /// dimension a player widens. Geometry cannot tell "I dug this" from "I
    /// built this" from "this is a hill", and no reach or threshold makes it
    /// able to.
    ///
    /// A `Vec` over the world's width, which is 32 KB at the shipped 8192
    /// wide (was 8 KB at 2048 before the world grew). M10's streaming will
    /// want this keyed per chunk column instead, alongside everything else
    /// that is currently sized to a resident world.
    sky_surface: Vec<i32>,
    /// Which positions were **inside the ground when the world was made** —
    /// one bit per cell, row-major from `bounds.min_x`/`bounds.min_y`,
    /// packed 64 to a `u64`. Empty until `freeze_underground_map` has run.
    ///
    /// **This is `sky_surface`'s answer asked per cell instead of per
    /// column, and that is the whole of the difference.** The column form
    /// asks *"is there anything solid above me in this column"*, which
    /// cannot tell a cave roof from a cliff brow: the open air outside an
    /// overhanging lip has rock above it in its column, so it drew as the
    /// inside of a cave, and so did the air under any solid object standing
    /// in the sky at genesis. Reported from play as dark bands under
    /// overhangs and under objects; measured at 156–408 cells per 2048x640
    /// world across seeds 1–6, in patches of twenty to fifty cells sitting
    /// on the skyline (`examples/underground_probe.rs`,
    /// `Reports/dark-bands-diagnosis.md`).
    ///
    /// Seeded by a flood fill from the top of the world through everything
    /// that is not `Solid` or `Powder` — the exact complement of
    /// `freeze_sky_surface`'s own predicate, so the two cannot hold
    /// different opinions about what ground is. A cell is underground if it
    /// was ground, or if it was air the sky could not reach.
    ///
    /// **It stores more history; it does not infer.** That distinction is
    /// the one `Reports/dead-ends.md` §977 makes after four inference rules
    /// failed in four different ways — *"revisit only by storing more
    /// history, never by inferring"* — and every property the column form
    /// was chosen for survives, because those all follow from the cells
    /// having been rock: a shaft you dig stays a tunnel at any width, with
    /// no threshold anywhere; a roof you build still leaves outdoors under
    /// it; a grain left in the air casts nothing.
    ///
    /// The change is **one-directional by construction** — a cell can only
    /// go from underground to outdoors, never the reverse — because both
    /// ways of being marked here (it was solid; it was air with something
    /// solid above it) put the column's topmost solid at or above the cell,
    /// which is exactly what the column rule tested. Asserted by
    /// `the_per_cell_map_never_turns_open_sky_into_cave`.
    ///
    /// 164 KB at 2048x640, against `sky_surface`'s 8 KB, and 5.8 ms to
    /// build once against worldgen's own ~325 ms. M10's streaming will want
    /// it per chunk alongside everything else sized to a resident world.
    underground: Vec<u64>,
    /// **The top of the ground, asked so that a cliff brow is not mistaken
    /// for it.** One entry per column from `bounds.min_x`, `i32::MAX` for a
    /// column that holds no ground at all — same shape and same convention
    /// as `sky_surface`, and it exists because that array answers a subtly
    /// different question than the terrain shading needs.
    ///
    /// `sky_surface` is the *topmost* ground in the column, which is right
    /// for "is there anything above me" and wrong for "how deep am I". A
    /// brow is ground, so it sets that entry, and every cell in its column
    /// down to bedrock is then graded as if buried under the brow's height.
    /// Measured, that drew a **straight vertical tone seam one to ten
    /// columns wide running from the surface to the bottom of the world** —
    /// 2,990 cells at x 332..337 on seed 5, a single 494-cell column on seed
    /// 7 (`examples/underground_probe.rs`, `Reports/dark-bands-diagnosis.md`).
    ///
    /// So this walks each column **up from the bottom** and stops at the
    /// first cell the sky can reach, recording the last row before it. That
    /// skips a brow, because open air sits under one; it does *not* skip a
    /// cave, because cave air is not outdoors and the walk carries straight
    /// through; and a notch is untouched, so `light_datum`'s opening still
    /// has the same job to do.
    ///
    /// Frozen once, after `underground`, which it reads.
    ground_datum: Vec<i32>,
    /// The same dedup idea as `pending_structural_checks`, for
    /// `ActiveKind::Decay`, and it exists for a different reason worth
    /// stating: decay sites are scheduled by `World::end_step`'s settle
    /// scan, which fires **every time a chunk goes from awake to settled**.
    /// A litter drift that is disturbed and re-settles ten times would
    /// otherwise stack ten sites on each of its cells, and since each site
    /// independently rolls `DECAY_CHANCE_*`, the effective decay rate would
    /// become a function of how often the ground was walked on. That is not
    /// a performance problem, it is a correctness one -- the rate has to be
    /// a property of the material, not of the chunk's history.
    ///
    /// `Decay` carries no state beyond position, so `(x, y)` is an
    /// unambiguous key, exactly as for a structural check.
    pending_decay_sites: std::collections::HashSet<(i32, i32)>,
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
    // M18's `creatures: Vec<CreatureState>` is **gone**, not moved. A
    // creature is an organism now (`Reports/creature-direction.md` §3a), so
    // its state lives in `organisms` below with everything else's. The
    // parallel vector had no generations, no reclamation and a `u16`
    // overflow guarded only by a `debug_assert`; `push_creature` and
    // `creature_mut` went with it.
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
    /// The two stigmergy planes (`Reports/creature-direction.md` §5) — 2
    /// channels x 2 buffers x 1 byte, CA resolution rather than
    /// `FIELD_SCALE`, for the measured reason in `pheromone.rs`'s module
    /// doc. That was ~640 KB at the 512x320 world this was sized against;
    /// at the shipped 8192x2560 it is ~84 MB, allocated eagerly in
    /// `Pheromones::new` whether or not a creature exists anywhere in the
    /// world — a real, standing cost, not a hypothetical one.
    ///
    /// A `World` field rather than something creatures own, for the same
    /// reason `fields` is: the signal outlives whoever deposited it, which
    /// is the entire point of stigmergy.
    pub pheromones: Pheromones,
    /// The "did it fire" counters for the field solve — see [`FieldStats`],
    /// which explains why the obvious alternative (counting unsettled tiles)
    /// cannot answer the question they exist for.
    pub field_stats: field::FieldStats,
    /// The "did it fire" counters for creature behaviour — `FailureCounts`
    /// in shape and in purpose.
    ///
    /// An image shows *what and where*; it cannot show whether the
    /// mechanism you built is what produced it. A colony of ants milling
    /// plausibly and a colony genuinely foraging look identical at the zoom
    /// a contact sheet is read at, and `trips_completed` is the number that
    /// tells them apart (`CLAUDE.md`, and the collapse that rendered
    /// convincingly for a whole run while its body count said the feature
    /// had never once executed).
    pub creature_stats: CreatureStats,
    /// **Evolution is a fuzzer for your conservation laws.** Every surveyed
    /// sim that evolved anything eventually evolved an exploit of an
    /// energy-accounting bug — Karl Sims' creatures harvesting integration
    /// error is the canonical case. So the census is built *before*
    /// mutation is switched on (stage 4), because afterwards every anomaly
    /// is ambiguous between "bug" and "adaptation".
    ///
    /// `f64` because these accumulate over long runs and an `f32` total
    /// stops being able to represent a single `idle_cost` addition once it
    /// passes about 16 million.
    pub energy_ledger: EnergyLedger,
    organisms: Vec<OrganismSlot>,
    free_organism_slots: Vec<u16>,
    /// **Cumulative organism births and deaths — the lineage turnover
    /// readout the plant plan of record's Phase 0d asks for and nothing
    /// printed.**
    ///
    /// `Reports/plant-evolution-design.md` §5: "the count of
    /// inherited-genome establishments per run is the plant equivalent of
    /// births-per-generation, and if it reads ~0 at 30k frames, every
    /// evolution claim at that horizon is about founders". A standing
    /// count cannot answer that — `organism_slot_usage` reports how many
    /// slots are live *now*, and slot reuse makes a flat live count
    /// consistent with both a frozen stand and a fast cycle. Only a
    /// cumulative pair separates them.
    ///
    /// Always-on rather than `#[cfg(test)]`, on the same reasoning
    /// `organism_generation_wraps` above already records: a `u64` add on
    /// the allocation path is free beside the `HashMap` that call just
    /// built, and a counter that exists only in test builds cannot say
    /// anything about a long run, which is the only place the number gets
    /// interesting.
    organisms_born: u64,
    organisms_died: u64,
    /// **Germinations refused because every organism slot was live** — the
    /// other half of making the 4,095 ceiling a real check rather than a
    /// `debug_assert` (see `push_organism`).
    ///
    /// A counter rather than a panic because refusing one birth is a
    /// recoverable, in-world outcome — a seed that finds no room is a seed
    /// that does not sprout — while a panic would take the session down for
    /// a condition a dense world can legitimately reach. But a refusal that
    /// nobody counts is indistinguishable from a world where nothing
    /// happened to breed this frame, which is the *other* half of `§F4`'s
    /// severity: the corruption was silent and so would the fix be.
    ///
    /// Always-on, on the same reasoning as `organisms_born` above: this
    /// number only gets interesting in a long release run, which is exactly
    /// where a `#[cfg(test)]` counter cannot see.
    organisms_refused: u64,
    /// How many times a reused slot's 4-bit generation has wrapped back to
    /// zero — see `push_organism`, which is the only writer.
    ///
    /// A wrap is the single case the generational check cannot catch: a
    /// reference stale by exactly 16 reuses reads as live again. That was
    /// accepted (`encode_organism_id`'s doc) on the grounds that it needs a
    /// bug compounded with exactly the wrong reuse count — but "accepted"
    /// should mean "known quantity", not "unobservable". With creatures
    /// allocating on their own schedule this is the number that says
    /// whether the assumption still holds at play rates.
    pub organism_generation_wraps: u32,
    /// **Seeds that waited for water and then germinated** — the counter
    /// for the dormancy mechanic, because a picture cannot show it and no
    /// existing readout separates the cases.
    ///
    /// `plant_probe` prints "seeds or seedlings", which lumps a seed
    /// patiently waiting on dry ground together with a seedling starving
    /// after germinating on it — the exact two states this mechanic exists
    /// to tell apart, and the failure it was built to end. A stand can look
    /// identical either way.
    ///
    /// Counts only germinations that were *deferred at least once*: a seed
    /// that lands on damp ground and sprouts immediately is the old
    /// behaviour and is not evidence of anything.
    pub seeds_germinated_after_waiting: u32,

    /// Decay events, split by which side of `DECAY_MOISTURE_THRESHOLD` the
    /// field humidity was on when the roll was made.
    ///
    /// Split rather than totalled, because the question these were added for
    /// is not "how much rotted" but **"which rate is the world running on"**.
    /// The two chances differ 25x (`decay::DECAY_CHANCE_DAMP` 0.05 against
    /// `DECAY_CHANCE_DRY` 0.002), so a total conflates a little damp ground
    /// with a lot of dry ground and cannot tell them apart -- which is the
    /// distinction the worldgen soil baseline moved, and the reason anyone is
    /// looking. A picture cannot show it either: rotted litter and unrotted
    /// litter are the same few pixels at contact-sheet zoom.
    pub decayed_damp: u32,
    /// Counterpart to `decayed_damp`; see it for why these are separate.
    pub decayed_dry: u32,
    /// Leaves shed by the graded shade pressure (`tree.ron`'s
    /// `shade_death`), the upstream half of §O's decay count. Split by
    /// *cause* for the same reason the decay counters are split by rate:
    /// the abscission retune has three levers -- shade, drought, and the
    /// stranded-spray reclaim that rides on both -- and the decay total
    /// cannot say which one manufactured the litter it rotted.
    pub shed_shade: u32,
    /// Counterpart to `shed_shade`: the drought pressure (`drought_death`).
    pub shed_drought: u32,
    /// Leaves reclaimed by `shed_stranded_leaves` after either pressure
    /// fired -- consequential fall, not a lever of its own.
    pub shed_stranded: u32,
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
    /// Cumulative temperature-triggered transition counts (boiled,
    /// condensed, froze, melted, reacted). Same "did it fire at all"
    /// instrumentation as `structural_failures`, for the same reason: a
    /// steam plume and painted smoke are indistinguishable in a contact
    /// sheet. Read by `examples/filmstrip.rs` beside the image.
    pub phase_changes: crate::sim::fire::PhaseCounts,
    /// How often evaporation found the air above a surface already
    /// saturated, split by surface kind -- see `evaporation::DrynessCounts`.
    pub dryness_counts: crate::sim::evaporation::DrynessCounts,
    /// Holds the sky at one state instead of reading `weather::at(seed,
    /// frame)`. `None` in play, and in every test that is not *about*
    /// holding it still.
    ///
    /// **Exists because a live sky silently invalidated a placement
    /// claim.** `tests/worldgen.rs`'s `generated_terrain_is_already_at_rest`
    /// asserts that generation emits a world which does not slump. It
    /// already switches off the live processes it knew about — plants, moss,
    /// `spring_flow`, each with a comment saying a growing thing is "a live
    /// process, not a placement defect". Weather arrived afterwards and got
    /// no such treatment, so on any seed whose sky is busy the test was
    /// asserting that terrain holds still *while snow falls on it*. Seed 3
    /// precipitates from frame 0 (`Snow`, intensity 0.36, 1,786 wet frames
    /// in 12,000) and is the seed both at-rest tests failed on
    /// (`open-bugs-handoff.md` §M); seeds 1, 2 and 5 never precipitate at
    /// all in that window, and passed.
    ///
    /// `Weather::CLEAR`'s own doc already called itself "the one every 'does
    /// this stay settled' test asserts against". This is what lets a test
    /// actually do that, on the seed it was given rather than on a seed
    /// picked for having a quiet sky — which would be tuning the sweep to
    /// the answer.
    pub weather_override: Option<crate::sim::weather::Weather>,
    /// **The atmosphere's water, in liquid-water cell-equivalents** — the
    /// credit half of the outer water cycle, and the one number that closes
    /// it.
    ///
    /// The inner cycle (boil, condense, freeze, melt) is conserved per cell
    /// already: `fire::transform` carries a cell's fill across every
    /// transition and hands it back. The *outer* cycle was not conserved at
    /// all. `evaporation::tick` deleted water and credited nothing;
    /// `weather::step` spawned water cells out of the sky. A world's total
    /// water was whatever the difference between those two rates happened to
    /// be, and neither of them knew the other existed.
    ///
    /// This is where the one goes and where the other comes from. Scaled so
    /// **1.0 is one full water cell** (`material::LIQUID_FULL`'s 0..1000
    /// scale divided out), which is the same unit `weather`'s
    /// `water_equivalents` census reports, so `water_equivalents(world) +
    /// atmospheric_bank` is a constant a test can actually assert on.
    ///
    /// `f64` for the reason `energy_ledger` is one: it accumulates over long
    /// runs, and an `f32` total stops being able to represent a single cell's
    /// worth of credit once it passes about sixteen million.
    ///
    /// **One bank for the whole world, not a per-column or per-region one.**
    /// The atmosphere mixes — that is `evaporation::shelter`'s whole story
    /// about why a gale makes the air over a puddle and the air over a lake
    /// read identically — so a bank that tracked *where* the water went would
    /// be modelling a thing the field channel already models badly on
    /// purpose, at the cost of an extra grid. What this is for is the
    /// conservation law, and a conservation law is global.
    ///
    /// Written only by `credit_atmosphere` and `spend_atmosphere`; `pub` so
    /// a test can drain it and a harness can print it.
    pub atmospheric_bank: f64,
    /// Where a denser cell displaced near-full liquid at a free surface
    /// this frame — **candidate** splash sites, not splashes. See
    /// `CellSurface::report_splash` for why the sweep only reports them,
    /// and `particle::throw_splashes` for the one place that acts on them.
    ///
    /// Cleared at the top of every step, so a frame nobody drained is
    /// discarded rather than growing, and bounded at `MAX_SPLASH_SITES` so
    /// a blob landing in a lake cannot make the list the expensive part of
    /// the frame.
    pub splash_sites: Vec<(i32, i32, f32)>,
    /// Cumulative count of splash droplets actually thrown -- the "did it
    /// fire at all" counter for the effect, and a different number from
    /// `splash_sites.len()`, which is only how many candidates the sweep
    /// reported this frame. A droplet in flight is one pixel and a contact
    /// sheet cannot tell one from a stray grain of water, so the count is
    /// what says the mechanism ran. Bumped by `particle::throw_splashes`.
    pub splashes_thrown: u32,
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
    /// `ChainMode`. `i32::MAX` is the default -- `CHAIN_MODES[0]`
    /// (`SPREAD`), no limit. `TIGHT` was built as the default on the
    /// owner's request and measured back out; `ChainMode`'s own doc
    /// carries the table and the reason.
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
    /// Where the world was last disturbed, when, and **how big the wound
    /// was**. A small ring: only the most recent handful matter, since
    /// older ones fall outside `chain_window` anyway. See
    /// `structural::Disturbance` for why the extent is not optional.
    pub disturbances: std::collections::VecDeque<crate::sim::structural::Disturbance>,
    /// Failures already judged and part-way through coming down, one slice
    /// per `structural::STRUCTURAL_TICK_INTERVAL` frames. See
    /// `structural::StagedFracture` and `advance_staged_fractures`; empty
    /// in every frame where nothing is mid-collapse, which is nearly all of
    /// them.
    pub staged_fractures: std::collections::VecDeque<crate::sim::structural::StagedFracture>,

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
    /// **This is not a containment measure and never was**, whatever it has
    /// been labelled: it is Manhattan `|failure.at - (x, y)|`, the distance
    /// from the *checked cell* to the *failing ancestor*, bounded by
    /// construction to `load::ROOTWARD_CHECK_STEPS` hops. It cannot see how
    /// far the consequence landed from what the player hit, and measured on
    /// a rolling-world blast it reads **1 cell** while real failures are
    /// landing everywhere. `max_damage_reach` below is the number that
    /// answers that question; this one answers the empirical question its
    /// own paragraph below poses and nothing else.
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
    /// **The containment measure**: the greatest Chebyshev distance from
    /// the nearest *live* disturbance to any cell a consequence actually
    /// destroyed, over the whole run.
    ///
    /// Reported in the units `World::chain_reach` is written in (see
    /// `World::distance_to_live_disturbance`, its measurement twin), so it
    /// can be read straight against the `F9` setting: at LOCAL this
    /// standing at 200 says damage travelled four times past the licence,
    /// and no other counter here can say that at all.
    ///
    /// **Damage only.** Recorded where something stops being what it was --
    /// the region handed to the fracturer, each paced slice,
    /// `break_free` on the organism path, and the cells a crush actually
    /// fissures. Deliberately *not* recorded for `rigid::settle`'s landing
    /// scheduling or for a check that was scheduled and refused: those are
    /// **work**, not damage, and folding them in would make the number
    /// unreadable -- a scheduled check that changed nothing is exactly what
    /// a contained world is full of.
    ///
    /// Nothing is recorded when there is no live disturbance at all, rather
    /// than `0`: see `World::distance_to_live_disturbance` for why zero is
    /// the wrong answer to "nothing was disturbed".
    pub max_damage_reach: u32,
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
    /// Failing regions that `rigid::fracture_failing_region` **declined**,
    /// because they were smaller than `MIN_FRACTURE_CELLS`, and the cells
    /// they took. Those fall through to per-cell `break_free` -- which is
    /// powder.
    ///
    /// # Why this is its own counter and not read off the mean
    ///
    /// `largest_failure`'s note above says the mean and the max together
    /// say whether pieces or grit came out. They do not, quite, and an
    /// independent review found the gap: a mean of 4.1 against a threshold
    /// of 6 *suggests* the typical event is dust and cannot show it, because
    /// a mean is equally consistent with a handful of big chunks beside a
    /// swarm of singles. That is the same shape as the metric traps in
    /// `CLAUDE.md`, one level up.
    ///
    /// This counts the thing itself. Read `crumbled_cells` against
    /// `overloaded_cells + unsupported_cells`: it is the fraction of failed
    /// material that never got the chance to become a chunk.
    pub crumbled: u32,
    pub crumbled_cells: u32,
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
    /// Slices `structural::advance_staged_fractures` took off a failure
    /// that was too big to fracture in one tick, and the cells they took.
    ///
    /// The "did it fire at all" counter for R3a, and it is exactly the
    /// question a contact sheet cannot answer: a collapse that arrives in
    /// one frame and one that arrives in five bites look identical in a
    /// grid of stills, and the difference between them is the whole
    /// change. `overloaded`/`unsupported` above cannot say it either --
    /// they count the *failure*, which is recorded once, whole, before any
    /// of this.
    pub staged_slices: u32,
    pub staged_cells: u32,
    /// Cells lifted out of the grid into a tumbling `ChunkBody`, and how
    /// many bodies they became.
    ///
    /// **The only counter here that measures a displacement rather than a
    /// judgement.** Every field above it is recorded at `structural.rs`'s
    /// `record` call, which runs before the free-face test, the boundary
    /// erosion, the slicing and the fracture -- so all of them can be large
    /// on a run where nothing whatsoever moved. That is not hypothetical:
    /// it is the exact shape of the owner's *"no pieces move, ever"*
    /// against a harness reporting hundreds of unsupported failures.
    ///
    /// Recorded inside `rigid::promote`, at the line that actually pushes
    /// the body, rather than at any call site -- `fracture_with_impulse`,
    /// `calve_collar` and `fracture_shell` all reach it through the same
    /// door, and so does anything added later.
    ///
    /// Cells that left a fracture as part of a promoted `ChunkBody`, and
    /// cells that left it as rubble.
    ///
    /// **The mass, because the event counts were answering a different
    /// question than the one being asked.** Reported from play against a
    /// collapse whose region sizes and body count had both improved by a
    /// large factor: *"they don't look like chunks when they fall, they are
    /// still mostly dust when they sink."* He was right and every counter
    /// said otherwise, because `size_buckets` measures how big the *region*
    /// was and "peak chunk bodies" counts how many *events* there were, and
    /// a player watches neither — he watches how much of what is falling is
    /// in pieces big enough to see.
    ///
    /// A region of 83 cells that fractures into eleven 4-cell fragments is
    /// a large region, several bodies' worth of events, and entirely dust
    /// on screen. Only the ratio of these two tells them apart.
    pub promoted_bodies: u32,
    pub promoted_cells: u32,
    /// Cells converted in place to `breaks_into` rubble -- the other half
    /// of a fracture's output, and the one the ethos calls grit.
    ///
    /// Paired with `promoted_cells` deliberately: the ratio between them is
    /// the block-size distribution the owner's *"a few blocks, more
    /// cobbles, a lot of grit"* is about, and either number alone cannot
    /// show it.
    ///
    /// Two code paths write the identical conversion and both are counted:
    /// `rigid::shatter_to_rubble`, which takes the fragments that came out
    /// below `MIN_BODY_CELLS`, and `structural::break_free`, which is the
    /// fallback when a region was too small to fracture at all. Neither
    /// counts a cell whose material has no `breaks_into` -- both decline
    /// and leave it standing, and counting a decline would make grit look
    /// like it happened.
    ///
    /// **`break_free`'s organism path is deliberately *not* counted.** The
    /// third caller is `structural.rs`'s plant-support check turning a
    /// limb that lost its anchor into deadwood: the same conversion, an
    /// entirely different event, and one that fires on its own schedule
    /// all through any world with vegetation in it. Folding it in here
    /// would put tree death into the number that is supposed to say how
    /// rock came apart, and swamp it on exactly the generated worlds this
    /// counter exists to judge.
    pub shattered_cells: u32,
    /// Failing-region sizes, bucketed — `SIZE_BUCKETS` names the edges.
    ///
    /// **The mean and the max together still hide the shape, and the shape
    /// is the whole question.** `largest_failure` says a 12-cell region
    /// happened once; the mean says 1.7; neither says whether the
    /// distribution has a body between them or is 570 single cells with one
    /// outlier. The answer decides whether the fragment ladder can help at
    /// all: `rigid::MIN_FRACTURE_CELLS` declines below 6, and 6-7 cells can
    /// produce no fragment reaching `MIN_BODY_CELLS`, so a distribution that
    /// lives entirely under 8 cannot produce a chunk however the rungs are
    /// tuned. The owner's report is the other end of the same fact:
    /// *"better with chunks instead of pile of dust"*.
    ///
    /// Bucket edges are the two floors and powers of two around them, so
    /// the boundary that matters is a boundary in the readout too.
    pub size_buckets: [u32; SIZE_BUCKETS.len()],
    /// The size of every body promoted, bucketed by doubling: `<8`
    /// (impossible -- `MIN_BODY_CELLS` is 8, so it stays 0 and is the
    /// sanity check on the bucketing), `8-15`, `16-31`, `32-63`, `64-127`,
    /// `128-255`, `256+`.
    ///
    /// **`promoted_bodies` and `promoted_cells` cannot answer the question
    /// this does.** Their ratio is a *mean*, and a run of forty 30-cell
    /// blocks and a run of one 200-cell slab plus thirty-nine 25-cell ones
    /// have nearly the same mean and are the two outcomes the ethos is
    /// about: *"a few blocks, more cobbles, a lot of grit"* is a
    /// **distribution**, and its absence -- everything one size, or the
    /// all-or-nothing split -- reads as fake on sight. Reported from play
    /// as *"could the pattern of cracks be more heterogeneous, so the
    /// chunks that break off are different sizes"*.
    ///
    /// Seven `u32`s on a struct that is copied per tile, which is the
    /// cheapest thing that can answer a distribution question at all.
    pub promoted_sizes: [u32; 7],
    /// Quarter turns a falling body asked for, and how many the fit probe
    /// refused.
    ///
    /// **These exist because the probe they measure was dead for the life
    /// of the mechanism and nothing could tell.** `rigid::rotation_fits`
    /// used to compare every cell against its own position, so it answered
    /// "clear" unconditionally and every body rotated through whatever was
    /// beside it (`Reports/open-bugs-handoff.md` bug K). A probe that always
    /// says yes and a probe that works produce the same tumbling on a
    /// contact sheet at the zoom one is read at; only the refusal count
    /// separates them, which is `CLAUDE.md`'s "did it fire at all needs a
    /// counter, not a picture" in its purest form.
    ///
    /// `refused` at exactly zero over a scene with walls in it is the tell
    /// that the probe has gone vacuous again — a *ratio* is a tuning
    /// question, but a zero is a wiring one.
    pub rotations_asked: u32,
    pub rotations_refused: u32,
    /// Organism cells the plant-support check broke free — a limb that lost
    /// its anchor becoming deadwood.
    ///
    /// **The "did it fire at all" counter for felling**, and it exists
    /// because none of the fields above can answer it. `shattered_cells`
    /// deliberately excludes this conversion (see its own doc, and
    /// `structural::break_free`), `unsupported` is only recorded on the
    /// inert path, and a crown that came down and a crown that was never
    /// asked are the same handful of brown pixels on a contact sheet. A
    /// coherent-looking collapse with a body count of zero has fooled this
    /// project once already (`CLAUDE.md`), and a severed tree that quietly
    /// kept growing has now fooled it a second time: measured on
    /// `scene=fell cut=248,186,16,6`, 83 cells of trunk removed and living
    /// tissue going *up* from 2,823 to 2,911 over the next 210 frames,
    /// with every counter in this struct reading zero.
    ///
    /// Kept out of `shattered_cells` for that field's own stated reason,
    /// and reported beside it rather than folded in: a tree shedding
    /// deadwood on its own schedule and a tree being chopped down are the
    /// same conversion and different events.
    pub severed_organism_cells: u32,
    /// Of `severed_organism_cells`, how many left the grid as **pieces** --
    /// cells of a promoted `ChunkBody` rather than grains converted in
    /// place.
    ///
    /// **The acceptance number for felling, and the pair is the metric, not
    /// either half.** `severed_organism_cells` alone says a tree came down;
    /// it says nothing about *what came down*, and the two outcomes it
    /// cannot tell apart are the whole complaint: a crown that arrives as
    /// logs and a crown that arrives as two and a half thousand grains of
    /// deadwood are the same number here and are the difference between
    /// "the tree fell" and `design-philosophy.md` §0a's uniform dissolve to
    /// powder. Measured before this package: 2,648 severed, 45 as pieces --
    /// **1.7%**.
    ///
    /// Kept apart from `promoted_cells` for that field's own stated reason
    /// read the other way round: `promoted_cells` is world-wide and
    /// cumulative over rock and tissue alike, so a felling scene's share
    /// cannot be recovered from it once anything else in the world has
    /// broken. Both are recorded; only this one answers "how much of the
    /// *tree* survived as something you can see move".
    pub severed_organism_pieces: u32,
    /// Cells of a settling `ChunkBody` that found **nowhere to go** and were
    /// dropped -- neither written back into the grid nor displaced.
    ///
    /// `rigid::settle` searches (empty cell, then a liquid it outweighs,
    /// then a ring search, then an adjacent liquid) and a cell that fails
    /// every arm is destroyed silently. `Reports/open-bugs-handoff.md` §1c
    /// carries the standing measurement, around 10% of a body's cells; this
    /// makes it a per-run number instead of a remembered one.
    ///
    /// **Instrumentation ahead of the fix, deliberately.** It matters much
    /// more now than it did: before organism tissue could be promoted at
    /// all, a body landed on terrain, and now a felled crown's pieces land
    /// in a large pile of the same crown's own grit -- which is exactly the
    /// configuration where `nearest_free`'s rings come back empty. A share
    /// of a fall that simply vanishes reads on screen as a fall that ate
    /// itself, and no other counter here can distinguish it from grit.
    pub settle_lost_cells: u32,
    /// Cells `rigid::settle` wrote back as a **piece tier**
    /// (`MaterialDef::severs_into`) — a promoted limb arriving as `log`.
    ///
    /// The delivery end of `severed_organism_pieces`, which is the
    /// promotion end. Both are needed: a census of `log` standing in the
    /// world measures what has *survived* decay and fire since, and cannot
    /// distinguish a fall that never delivered its pieces from one that
    /// delivered them and then lost them.
    pub settled_tissue_cells: u32,
}

/// Inclusive lower bounds of `FailureCounts::size_buckets`. 6 is
/// `rigid::MIN_FRACTURE_CELLS` and 8 is `MIN_BODY_CELLS`; a region below the
/// first cannot fracture at all, and one below the second cannot yield a
/// promoted body.
pub const SIZE_BUCKETS: [u32; 7] = [1, 2, 3, 6, 8, 16, 64];

impl FailureCounts {
    pub fn record_reach(&mut self, reach: u32) {
        self.max_chain_reach = self.max_chain_reach.max(reach);
    }

    /// See `max_damage_reach`. Callers pass a distance already resolved
    /// against a *live* disturbance (`World::distance_to_live_disturbance`
    /// returned `Some`), so there is no sentinel to filter here.
    pub fn record_damage_reach(&mut self, reach: i32) {
        self.max_damage_reach = self.max_damage_reach.max(reach.max(0) as u32);
    }

    /// See `severed_organism_cells`. One call per cell the organism path
    /// actually converted -- a declined conversion (no `breaks_into`) must
    /// not be counted, for the same reason `break_free` reports it.
    pub fn record_severed_organism(&mut self, cells: u32) {
        self.severed_organism_cells = self.severed_organism_cells.saturating_add(cells);
    }

    /// See `severed_organism_pieces`. Always a subset of the cells passed to
    /// `record_severed_organism` for the same event, and recorded beside it
    /// rather than derived: the grit half is everything that is not this,
    /// and computing it from two independently accumulated world totals
    /// would drift the moment anything else in the world breaks.
    pub fn record_severed_pieces(&mut self, cells: u32) {
        self.severed_organism_pieces = self.severed_organism_pieces.saturating_add(cells);
    }

    /// See `settle_lost_cells`. One call per cell `rigid::settle` could not
    /// place anywhere.
    pub fn record_settle_loss(&mut self, cells: u32) {
        self.settle_lost_cells = self.settle_lost_cells.saturating_add(cells);
    }

    /// See `settled_tissue_cells`.
    pub fn record_settled_tissue(&mut self, cells: u32) {
        self.settled_tissue_cells = self.settled_tissue_cells.saturating_add(cells);
    }

    pub fn record_confined(&mut self, cells: usize, depth: u32) {
        self.confined += 1;
        self.confined_cells += cells as u32;
        self.deepest_confined = self.deepest_confined.max(depth);
    }

    pub fn record_staged(&mut self, cells: usize) {
        self.staged_slices += 1;
        self.staged_cells += cells as u32;
    }

    /// One quarter turn offered to the fit probe, and whether it fitted.
    /// See `rotations_asked`.
    pub fn record_rotation(&mut self, fits: bool) {
        self.rotations_asked = self.rotations_asked.saturating_add(1);
        if !fits {
            self.rotations_refused = self.rotations_refused.saturating_add(1);
        }
    }

    /// One body, `cells` cells, actually lifted off the grid. See
    /// `promoted_bodies`.
    ///
    /// `saturating_add` rather than the `+=` its neighbours use, and that
    /// is not a style slip: the fields above it count discrete failure
    /// events and this one counts *cells*, over runs that go to 5,000
    /// frames and nine charges. A wrap here would read as a collapse in
    /// the one counter the night's work is judged by.
    pub fn record_promoted(&mut self, cells: usize) {
        self.promoted_bodies = self.promoted_bodies.saturating_add(1);
        self.promoted_cells = self.promoted_cells.saturating_add(cells as u32);
        // Bucket by doubling from 8, the smallest a body can be. `ilog2`
        // rather than a loop, and clamped at the top so a 400-cell body
        // (`MAX_BODY_CELLS`) lands in the last bucket rather than past it.
        let bucket = if cells < 8 { 0 } else { (cells.ilog2() as usize - 2).min(6) };
        self.promoted_sizes[bucket] = self.promoted_sizes[bucket].saturating_add(1);
    }

    /// `cells` cells converted where they stood. See `shattered_cells` for
    /// which paths count and which one deliberately does not.
    pub fn record_shattered(&mut self, cells: usize) {
        self.shattered_cells = self.shattered_cells.saturating_add(cells as u32);
    }

    pub fn record(&mut self, mode: crate::sim::load::FailureMode, cells: usize) {
        self.largest_failure = self.largest_failure.max(cells as u32);
        let bucket = SIZE_BUCKETS.iter().rposition(|&edge| cells as u32 >= edge).unwrap_or(0);
        self.size_buckets[bucket] += 1;
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
    /// Take the leash off `chain_reach`, restoring the pre-playtest
    /// behaviour in which the load model alone decides what fails.
    ///
    /// # Why the load-model tests want this and the game does not
    ///
    /// `chain_reach` is a **policy layered over** the load model: the model
    /// works out that a cell cannot carry what is on it, and the policy
    /// then decides whether the consequence is allowed to happen here. The
    /// default is `TIGHT` because a playtest picked it, and TIGHT means
    /// "near something that reported itself disturbed" -- which a unit test
    /// that hand-places geometry and calls `schedule_structural_check`
    /// directly never does, because no verb was involved.
    ///
    /// Left at the default those tests do not exercise a stricter model,
    /// they exercise *nothing*: an empty disturbance ring refuses every
    /// failure, so a beam that should snap simply stands and the assertion
    /// fails for a reason that has nothing to do with what it is named
    /// for. `CLAUDE.md`'s "a superseded mechanism's tests keep passing
    /// while testing nothing" trap, run in reverse.
    ///
    /// So the model's own tests say so out loud here, and the policy keeps
    /// its own paired test (`structural::tests::
    /// a_reach_limit_keeps_damage_near_what_was_disturbed`), which sets a
    /// reach explicitly and runs both sides. **Nothing outside `#[cfg(test)]`
    /// should call this** -- a harness that wants the shipped behaviour
    /// (`filmstrip`, `ascii`, `acceptance.sh`) must take the default, or it
    /// stops measuring what the player sees.
    pub fn without_chain_limit(mut self) -> Self {
        self.chain_reach = i32::MAX;
        self
    }

    pub fn new(bounds: Rect) -> Self {
        let mut world = Self {
            chunks: HashMap::new(),
            fields: HashMap::new(),
            bounds: Some(bounds),
            frame: 0,
            clock: crate::sim::clock::Clock::default(),
            materials: MaterialRegistry::builtin(),
            rng: Rng::default(),
            chunk_bodies: Vec::new(),
            player: None,
            springs: Vec::new(),
            drains: Vec::new(),
            spring_ledger: crate::sim::spring::SpringLedger::default(),
            active_sites: BinaryHeap::new(),
            pending_structural_checks: std::collections::HashSet::new(),
            pending_evaporation: std::collections::HashSet::new(),
            pending_dissipation: std::collections::HashSet::new(),
            sky_surface: Vec::new(),
            underground: Vec::new(),
            ground_datum: Vec::new(),
            pending_decay_sites: std::collections::HashSet::new(),
            bodies: Vec::new(),
            free_body_slots: Vec::new(),
            body_index: HashMap::new(),
            species: SpeciesRegistry::builtin(),
            pheromones: Pheromones::new(bounds),
            field_stats: field::FieldStats::default(),
            creature_stats: CreatureStats::default(),
            energy_ledger: EnergyLedger::default(),
            organisms: Vec::new(),
            free_organism_slots: Vec::new(),
            organisms_born: 0,
            organisms_died: 0,
            organisms_refused: 0,
            organism_generation_wraps: 0,
            seeds_germinated_after_waiting: 0,
            decayed_damp: 0,
            decayed_dry: 0,
            shed_shade: 0,
            shed_drought: 0,
            shed_stranded: 0,
            fields_settled: false,
            touched_chunks: std::collections::HashSet::new(),
            load_budget: crate::sim::load::MAX_LOAD_CELLS_PER_FRAME,
            crush_confined: true,
            arch_relief: true,
            section_share: true,
            // Mirrors CHAIN_MODES[0], whichever that is. Kept in sync by
            // `the_default_chain_reach_is_the_first_chain_mode`, so moving
            // the default is one edit in one place. Currently SPREAD, so
            // this is `i32::MAX` -- the literal it replaced.
            chain_reach: crate::sim::structural::CHAIN_MODES[0].reach,
            chain_window: crate::sim::structural::CHAIN_WINDOW_FRAMES,
            disturbances: std::collections::VecDeque::new(),
            staged_fractures: std::collections::VecDeque::new(),
            load_cache: crate::sim::load::Cache::default(),
            structural_failures: FailureCounts::default(),
            phase_changes: crate::sim::fire::PhaseCounts::default(),
            // **A fresh world's early storms run on an endowment.** The sky
            // starts holding exactly one full-supply storm's reserve, so
            // frame 0 of a brand-new world rains exactly as hard as it did
            // before this existed — every scene and every guard written
            // against the old behaviour still sees it — and only a world
            // that has spent more than it has evaporated back starts to
            // thin out. Seeding it at zero instead would mean no world ever
            // saw rain until something had dried up first, which is not a
            // water cycle, it is a drought with a cycle bolted on.
            atmospheric_bank: crate::sim::weather::STORM_RESERVE,
            dryness_counts: crate::sim::evaporation::DrynessCounts::default(),
            weather_override: None,
            splash_sites: Vec::new(),
            splashes_thrown: 0,
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

    /// Advance both pheromone planes. Its own frame phase, and callers
    /// call it **every** frame like `step_fields` — the
    /// `PHEROMONE_INTERVAL` gate lives inside, so no caller has to know
    /// the interval exists.
    pub fn step_pheromones(&mut self) {
        let interval = self.clock.creature_interval(crate::sim::pheromone::PHEROMONE_INTERVAL);
        self.pheromones.step(self.frame, interval);
    }

    /// Add to a pheromone channel at `(x, y)`. Out-of-world deposits are
    /// dropped silently.
    pub fn deposit_pheromone(&mut self, channel: Channel, x: i32, y: i32, amount: u8) {
        self.pheromones.deposit(channel, x, y, amount);
    }

    /// Read a pheromone channel at `(x, y)`. Nearest-cell — the plane is
    /// already at CA resolution. Out of world reads 0.
    pub fn pheromone_at(&self, channel: Channel, x: i32, y: i32) -> u8 {
        self.pheromones.sample(channel, x, y)
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

    /// How many organism slots are currently allocated.
    ///
    /// The "did it fire" counter for anything that creates or destroys an
    /// organism — a harness can print it beside a picture, which is the one
    /// thing a picture cannot show: a worm cell whose organism has leaked
    /// and one whose organism is live draw identically (`CLAUDE.md`). It is
    /// also the direct readout on `free_organism` doing its job, since a
    /// missing release shows up here as a count that only ever climbs.
    pub fn live_organism_count(&self) -> usize {
        self.organisms.iter().filter(|slot| slot.state.is_some()).count()
    }

    /// Total energy held by every live organism — the left-hand side of
    /// `EnergyLedger`'s invariant.
    pub fn live_creature_energy(&self) -> f64 {
        self.organisms.iter().filter_map(|slot| slot.state.as_ref()).map(|state| state.energy as f64).sum()
    }

    /// Read-only view of one organism's whole-plant state, for probes.
    ///
    /// **A plain alias for `organism`, and only still here because it has
    /// callers.** It was added on the plant line to get around `organism`
    /// being `pub(crate)`, which an example crate could not see; the
    /// creature line made `organism` itself `pub` for its own reasons, so
    /// the workaround outlived the problem and the two met at the merge.
    /// Kept rather than removed because ten call sites across `plant.rs`
    /// and `examples/plant_probe.rs` read it and renaming them is churn,
    /// not reconciliation — but prefer `organism` in new code, and fold
    /// this away whenever those sites are next touched anyway.
    pub fn organism_state(&self, organism_id: u16) -> Option<&organism::OrganismState> {
        self.organism(organism_id)
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
        // See `pending_evaporation`'s own doc — this one is load-bearing for
        // the *rate*, not only for the frame cost.
        //
        // `insert` returns false when the position was already present, so
        // each of these both tests and marks in one go -- the structural arm
        // above needs two calls only because its index is also read
        // elsewhere. Two arms rather than one keyed on kind, because the
        // two sets are independent and the structural path's behaviour must
        // stay untouched.
        if matches!(site.kind, scheduler::ActiveKind::Evaporate { .. })
            && !self.pending_evaporation.insert((site.x, site.y))
        {
            return;
        }
        // See `pending_dissipation`'s own doc — load-bearing for the rate,
        // exactly as the evaporation one above is.
        if matches!(site.kind, scheduler::ActiveKind::Dissipate) && !self.pending_dissipation.insert((site.x, site.y)) {
            return;
        }
        // The same guard again for decay, which `origin/main` added
        // independently against its own site kind. Two kinds, two sets.
        if matches!(site.kind, scheduler::ActiveKind::Decay) && !self.pending_decay_sites.insert((site.x, site.y)) {
            return;
        }
        self.active_sites.push(Reverse(site));
    }

    /// Every pending active site, for tests that need to ask *what kind* is
    /// scheduled rather than only how many — `evaporation.rs`'s guards, which
    /// turn on the difference between a site retiring and a site staying on
    /// the list at a zero rate. Order is the heap's internal one and carries
    /// no meaning; every caller filters.
    #[cfg(test)]
    pub(crate) fn active_sites_for_test(&self) -> Vec<ActiveSite> {
        self.active_sites.iter().map(|&Reverse(site)| site).collect()
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
        if let scheduler::ActiveKind::Evaporate { .. } = site.kind {
            self.pending_evaporation.remove(&(site.x, site.y));
        }
        if let scheduler::ActiveKind::Dissipate = site.kind {
            self.pending_dissipation.remove(&(site.x, site.y));
        }
        if let scheduler::ActiveKind::Decay = site.kind {
            self.pending_decay_sites.remove(&(site.x, site.y));
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

    /// Allocate a new organism. Checks `free_organism_slots` first (bumping
    /// the reused slot's generation) before ever growing `organisms` —
    /// issue #8's actual fix, and live now that `free_organism` below
    /// populates that list.
    ///
    /// **The generation bump lives here and only here.** Freeing does not
    /// bump; reuse does. Two bumps per life-cycle would spend the 4 bits at
    /// double rate for no extra staleness detection, since nothing can hold
    /// a reference to a slot between the free and the reuse that the free
    /// alone would have invalidated.
    ///
    /// Returns the encoded `organism_id` to stamp onto `Cell::organism_id`,
    /// or **`None` when the 4,095 slots are all live** — see
    /// `organisms_refused`. Every caller has a refusal path already (they
    /// all check the target cell is free first and return early when it is
    /// not); the `Option` is what makes the compiler insist they use it,
    /// which is the whole reason the signature changed rather than a
    /// sentinel being returned. A sentinel `0` would stamp an *ownerless*
    /// organism cell onto the grid at the ceiling — softer than corrupting
    /// an identity, still a leak of exactly the kind this allocator exists
    /// to end.
    pub(crate) fn push_organism(&mut self, species: SpeciesId) -> Option<u16> {
        // **The ceiling is a real check now, not a `debug_assert`.**
        //
        // `Cell::organism_id` gives 12 bits to the slot index, so there are
        // 4,095 of them, and `encode_organism_id` does not mask: a 4,096th
        // slot index would set bit 12, which is the *generation*'s low bit.
        // In a release build that is silent — the new organism reads as a
        // different, live organism, and every cell that already pointed at
        // that identity now points at this one. `Reports/open-bugs-
        // handoff.md` §F4 names it "silent organism identity corruption in
        // release"; `Reports/population-dynamics-research.md` 9g asks for
        // exactly this fix in exactly these words ("Add a release-mode
        // check, not a `debug_assert`").
        //
        // **The failure mode is refusal, and refusal is counted.** A
        // germination that cannot get a slot simply does not happen, which
        // is a bounded, visible loss of one seed; nothing on the grid is
        // written, so nothing is left half-allocated. `organisms_refused`
        // is what stops that being invisible — a world quietly refusing
        // every birth and a world where nothing is breeding look identical
        // in every other readout.
        //
        // Checked before `organisms_born` is incremented so the born count
        // stays "organisms that exist", not "attempts".
        if self.free_organism_slots.is_empty() && self.organisms.len() >= ORGANISM_INDEX_MASK as usize {
            self.organisms_refused += 1;
            return None;
        }
        self.organisms_born += 1;
        let state = OrganismState {
            water: 0.0,
            water_status: 1.0,
            water_uptake: 0.0,
            water_demand: 0.0,
            water_uptake_acc: 0.0,
            water_desiccation: 0.0,
            endowment: 0.0,
            species,
            cells: std::collections::HashMap::new(),
            root_cells: 0,
            contact_root_cells: 0,
            shoot_cells: 0,
            anchor_cells: 0,
            anchor_moment: 0.0,
            crown_moment: 0.0,
            // **1.0, not 0.0, and the direction is deliberate.** A plant
            // that has not had an upkeep pass yet reads as *well anchored*,
            // so the first thing a germinating seedling does is not divert
            // its whole budget into roots on the strength of a number
            // nothing has computed. Same bias as `OrganismCell::support`
            // defaulting to "anchored": a rule with a consequence must
            // default to the answer that defers.
            anchor_status: 1.0,
            slenderness: 0.0,
            income: 0.0,
            maintenance_basis: 0.0,
            maintenance: 0.0,
            maintenance_unpaid: 0.0,
            starved_cells: 0,
            collar_y: None,
            // The species mean until something germinates and draws — see
            // `OrganismState::genotype_draws`.
            genotype_draws: [0.0; organism::GENOTYPE_TRAITS],
            // Creature fields: a plant is a chainless, headingless organism
            // with no energy budget of its own, and stays at these.
            //
            // `traits` is the *neutral* vector rather than any species'
            // authored one, because `push_organism` does not know whether
            // it is allocating a plant or a creature. `plant_creature_seed`
            // overwrites it from `CreatureDef::traits` one line after this
            // returns; a creature that reached the world without going
            // through that seam would eat as a generalist rather than
            // silently as a carnivore, which is the failure direction to
            // prefer.
            traits: [0.0; organism::CREATURE_TRAITS],
            chain: Vec::new(),
            heading: 0,
            energy: 0.0,
            carrying: None,
            since_nest: 0,
            forage_anchor: (0, 0),
            forage_max: 0,
            brain_state: [0.0; organism::BRAIN_HIDDEN_FOR_STATE],
            genome: Vec::new(),
            shoot_top_y: None,
            sympodial_forks: 0,
            plagiotropic_steps: 0,
            foliage_band: 0,
            bark_band: 0,
            inherited: false,
            generation: 0,
            seeds_set: 0,
            alleles: [0; organism::DISCRETE_LOCI],
            deferred_germination: false,
            senescent: false,
            rigid_steps: 0,
            lateral_departures: 0,
            departure_angle_sum: 0.0,
        };
        if let Some(slot_index) = self.free_organism_slots.pop() {
            let slot = &mut self.organisms[(slot_index - 1) as usize];
            // Wraps at 16 generations (4 bits) rather than growing further
            // -- see `encode_organism_id`'s own doc for why this bound was
            // accepted rather than widening `Cell` a third time.
            slot.generation = (slot.generation + 1) & GENERATION_MASK;
            // P-8: the wrap is the one moment a stale id can alias a live
            // organism again, so count it rather than leaving it a
            // theoretical footnote -- "how often does this actually
            // happen" should be a number the engine can answer. Always-on
            // rather than the design report's debug-only suggestion: a u32
            // add on the allocation path is free next to the HashMap the
            // same function just built, and a counter that only exists in
            // debug builds cannot tell you anything about a long release
            // session, which is the only place the count gets interesting.
            if slot.generation == 0 {
                self.organism_generation_wraps += 1;
            }
            slot.state = Some(state);
            Some(encode_organism_id(slot_index, slot.generation))
        } else {
            // The guard at the top of this function is what makes the index
            // below in range; this assertion is the second pair of eyes on
            // it, and it is the only thing left that a `debug_assert` is
            // the right tool for -- an internal invariant, not a runtime
            // condition the world can reach.
            debug_assert!(
                self.organisms.len() < ORGANISM_INDEX_MASK as usize,
                "organism index would overflow the 12 bits Cell::organism_id reserves for it"
            );
            self.organisms.push(OrganismSlot { generation: 0, state: Some(state) });
            Some(encode_organism_id(self.organisms.len() as u16, 0))
        }
    }

    /// `None` for `organism_id == 0` (no organism) or a stale id whose slot
    /// has since been reused by a different organism — the generation
    /// mismatch this whole scheme exists to catch, not a panic.
    pub fn organism(&self, organism_id: u16) -> Option<&OrganismState> {
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

    /// Return an organism's slot to the free list — the other half of
    /// issue #8's fix, and `Reports/organism-substrate-design.md` §6's
    /// generational allocator finally closed at both ends.
    ///
    /// **Why it exists now, having deliberately not existed before.** The
    /// note this replaces recorded that no retrofitted species needed it:
    /// moss's `Divide` never touches `OrganismState` after creation, and
    /// trees are planted by hand, so `organisms` growing forever was a
    /// bounded leak nobody could reach. Creatures end that. A colony that
    /// lays eggs allocates on its own schedule, and the 12-bit slot index
    /// caps concurrent organisms at 4,095 — one long session of a laying
    /// queen exhausts it (`Reports/creature-direction.md` §2b).
    ///
    /// Generation-checked exactly like `organism`/`organism_mut`: a stale
    /// id, or one already freed, is a **silent no-op** — never a panic, and
    /// never a second push of the same index onto the free list, which
    /// would hand the same slot to two live organisms. `state.is_none()`
    /// is that double-free guard, and it is load-bearing rather than
    /// defensive: the natural creature call site ("my cell list went
    /// empty, release me") can genuinely fire twice, once from the death
    /// path and once from the next scheduled tick finding nothing left.
    ///
    /// **The generation is not bumped here**, despite the design report's
    /// §2b wording. `push_organism`'s reuse branch already bumps on
    /// *reuse*, so bumping here as well would advance it twice per
    /// life-cycle and burn the 4-bit space at double rate. One bump per
    /// reuse, in exactly one place.
    ///
    /// **`plant::step_organisms` is the second caller**, releasing plant
    /// organisms whose cell list has gone empty — the one liveness
    /// definition that cannot orphan a standing cell, since a cell still
    /// referring to the organism is exactly what makes the list non-empty.
    pub(crate) fn free_organism(&mut self, organism_id: u16) {
        let (slot_index, generation) = decode_organism_id(organism_id);
        if slot_index == 0 {
            return;
        }
        let Some(slot) = self.organisms.get_mut((slot_index - 1) as usize) else {
            return;
        };
        if slot.generation != generation || slot.state.is_none() {
            return;
        }
        slot.state = None;
        self.free_organism_slots.push(slot_index);
        // Counted here rather than at either call site: this is the one
        // function that decides a release really happened (both callers can
        // fire twice for one death, and the guards above are what stop the
        // second one).
        self.organisms_died += 1;
    }

    /// Organisms ever allocated, and ever released — see
    /// `World::organisms_born`. The pair a turnover rate is computed from;
    /// their difference is the live count `organism_slot_usage` reports.
    pub fn organism_turnover(&self) -> (u64, u64) {
        (self.organisms_born, self.organisms_died)
    }

    /// How many organism slots are currently allocated, and how many of
    /// those are live — the high-water reading the 4,095 ceiling is judged
    /// against.
    ///
    /// The live half is `live_organism_count` rather than a second copy of
    /// the same filter: the two accessors arrived independently on the two
    /// merged lines, and one of them counting differently from the other
    /// later is exactly the kind of drift nobody would think to check.
    pub fn organism_slot_usage(&self) -> (usize, usize) {
        (self.organisms.len(), self.live_organism_count())
    }

    /// **The high-water mark of concurrently-live organisms**, and it is
    /// the *same number* as `organism_slot_usage`'s first element rather
    /// than a second tally — which is worth stating, because it looks like
    /// it should need one.
    ///
    /// `push_organism` pops `free_organism_slots` before it ever grows
    /// `organisms`, so the vector lengthens only on a birth that found no
    /// free slot — i.e. only when the live count is about to exceed every
    /// value it has ever held. `organisms.len()` is therefore exactly
    /// max-over-time of the live count, for free, with no per-frame
    /// bookkeeping. The `ceiling` it is judged against is the 12-bit slot
    /// index's own bound.
    pub fn organism_slot_high_water(&self) -> (usize, usize) {
        (self.organisms.len(), ORGANISM_INDEX_MASK as usize)
    }

    /// Births refused at the slot ceiling — see `organisms_refused`. Zero
    /// on every world that has not reached 4,095 live organisms, which is
    /// every world measured to date; a non-zero reading means the ceiling
    /// is now a live design constraint and not a footnote.
    pub fn organisms_refused(&self) -> u64 {
        self.organisms_refused
    }

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

    /// Put `fill` units of water into the sky's bank — the credit half of
    /// the outer cycle. `fill` is on `material::LIQUID_FULL`'s 0..1000
    /// scale, which is what every liquid write in the engine already speaks;
    /// the division to cell-equivalents happens here so no caller has to
    /// remember it.
    ///
    /// **Three callers, all of them water by construction**:
    /// `evaporation::tick` for a drying puddle, `evaporation::tick_soil` for
    /// a drying soil surface, and `fire::try_phase_change` for steam that
    /// condenses under open sky. `evaporates` is set on exactly one material
    /// and `condenses_into_sky` on exactly one; soil moisture is on
    /// `SOIL_SATURATED`'s scale, which infiltration already exchanges 1:1
    /// with a liquid fill. If any of those is ever set on something that is
    /// not water, this needs the density ratio the melt path already carries
    /// (`fire::melt_fill`) — a cell-equivalent is a *water* cell-equivalent,
    /// and a lighter liquid's fill is not worth the same water.
    #[inline]
    pub(crate) fn credit_atmosphere(&mut self, fill: u16) {
        self.atmospheric_bank += fill as f64 / crate::sim::material::LIQUID_FULL as f64;
    }

    /// Take `cells` cell-equivalents out of the bank for something the sky is
    /// about to create, or refuse and change nothing.
    ///
    /// **All-or-nothing, and floored at zero**, which is what keeps this an
    /// accounting identity rather than an approximation: a caller that gets
    /// `true` has already been charged and must create the cell, and a
    /// caller that gets `false` must not. There is no partial spend, because
    /// there is no such thing as three-tenths of a spawned water cell.
    ///
    /// The gross throttle is `storm_supply` below — by the time a spawn asks
    /// here the storm has already been thinned to what the bank can afford,
    /// so a refusal is the rounding at the very bottom of the barrel rather
    /// than the mechanism. Both exist: the supply factor is what the storm
    /// *looks* like, this is what it may actually *spend*.
    #[inline]
    pub(crate) fn spend_atmosphere(&mut self, cells: f64) -> bool {
        if self.atmospheric_bank < cells {
            return false;
        }
        self.atmospheric_bank -= cells;
        true
    }

    /// How much of a full-strength storm the sky can currently pay for,
    /// `0.0..=1.0`.
    ///
    /// **The same factor the simulation throttles the storm by and the
    /// renderer thins the drawn rain by**, which is the whole reason it is a
    /// method here rather than a local in `weather::step`. Falling
    /// precipitation is drawn straight from `weather::at(seed, frame)` and
    /// is not simulated at all (`weather::step`'s own doc: it is simulated
    /// where it lands, not where it falls), so a bankrupt sky with the gate
    /// on the landing side alone would *draw* a downpour that deposits
    /// nothing — visibly, for as long as the front lasts. One factor, read
    /// in both places, is what keeps the drawn storm and the landing storm
    /// the same storm.
    #[inline]
    pub fn storm_supply(&self) -> f32 {
        crate::sim::weather::supply(self.atmospheric_bank)
    }

    /// The sky this frame: `weather::at(seed, weather_frame)` unless
    /// [`Self::weather_override`] is holding it.
    ///
    /// **One resolution point, read by both the simulation and the
    /// renderer**, for the same reason `storm_supply` above is a method
    /// rather than a local: the storm that is drawn and the storm that lands
    /// have to be the same storm, and an override honoured by only one of
    /// them would draw snow that never settles on anything.
    ///
    /// **Read on the *weather* clock**, which is independent of the day's
    /// (`sim::clock::Clock::weather_slowdown`) and identical to `frame` at
    /// the default. This method and the override arrived on separate
    /// branches and merged into one body without conflicting -- the clock
    /// half is the half a textual merge drops, because the override is the
    /// line that moved.
    #[inline]
    pub fn weather(&self) -> crate::sim::weather::Weather {
        self.weather_override.unwrap_or_else(|| crate::sim::weather::at(self.seed, self.weather_frame()))
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

    /// See `field::ground_wetness_at` -- how wet the matter at and just
    /// below `(x, y)` is, `0..=1`. `fire::try_ignite`'s moisture gate.
    pub fn ground_wetness_at(&self, world_x: i32, world_y: i32) -> f32 {
        field::ground_wetness_at(&self.fields, self.bounds, world_x, world_y)
    }

    /// How strongly the field block at `(x, y)` sources moisture, `0..=1`.
    /// `ChunkView`'s half of `ground_wetness_at` above -- it has to
    /// assemble the two samples itself, because during a parallel pass one
    /// of them may live in its own detached tile rather than in `self`.
    pub(crate) fn moisture_source_at(&self, world_x: i32, world_y: i32) -> f32 {
        field::moisture_source_at(&self.fields, self.bounds, world_x, world_y)
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

    /// How strongly the field block covering this position sources moisture,
    /// `0.0..=1.0`. See `field::moisture_source_at` — the point of reading
    /// this rather than `field_at(..).moisture` is that it is rebuilt from
    /// the CA grid every frame and never advected, so wind cannot move it.
    pub(crate) fn field_moisture_source_at(&self, world_x: i32, world_y: i32) -> f32 {
        field::moisture_source_at(&self.fields, self.bounds, world_x, world_y)
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
                    // An impulse can land in open air with no CA cell
                    // changing anywhere near it -- `weather::gust` does
                    // exactly that -- so nothing else would tell the field
                    // that this tile's momentum channels are live again.
                    // Without this the solver's zero-momentum fast path
                    // would skip the very pass that is meant to disperse
                    // the impulse, and the gust would sit there.
                    tile.disturb_momentum();
                }
            }
        }
    }

    // --- crate-internal seams used only by `field::step` -------------------

    /// Whether every field tile reached its channel epsilons on the last
    /// solve — the field's own "nothing is changing any more".
    ///
    /// `pub` rather than `pub(crate)` so a harness can wait for a world to go
    /// genuinely quiet instead of stepping a fixed number of frames and
    /// hoping. `examples/scale_probe.rs` needs exactly that: the field takes
    /// thousands of frames to converge after generation, and a settled-cost
    /// figure taken before then is measuring the transient. Read-only, so it
    /// widens nothing.
    pub fn fields_settled(&self) -> bool {
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
    /// Diagnostic-only: nothing in the engine branches on the *count*, only
    /// on `fields_settled()`, and nothing may start to — this is a full scan
    /// of the tile map and a per-frame decision built on it would be the
    /// exact cost class the field's own sleeping exists to avoid. Public
    /// (rather than the `#[cfg(test)]` it started as) for one consumer: the
    /// river-cost harness in `examples/ascii.rs` prints it at coarse
    /// intervals, because "how many field tiles does a held disturbance
    /// keep awake" is the number that gates the 2026-08 world review's
    /// rivers track (`Reports/world-review-2026-08.md` §4).
    pub fn unsettled_field_tiles(&self) -> usize {
        self.fields.values().filter(|t| !t.settled()).count()
    }

    /// The same count, for the headless harnesses (`examples/ascii.rs`), which
    /// live in another crate and so cannot see the `#[cfg(test)]` form above.
    ///
    /// The warning on that one applies here with knobs on: this is a full scan
    /// of the tile map and **nothing in the engine may branch on it**. It
    /// exists because "did the field actually stay asleep for a whole day"
    /// cannot be answered by a timing (a scan that costs 0.02 ms and one that
    /// costs 0.00 ms are the same number through a `Duration` at this scale)
    /// and must not be answered by a picture — `CLAUDE.md`: "did it fire at
    /// all" needs a counter.
    #[doc(hidden)]
    pub fn awake_field_tiles(&self) -> usize {
        self.fields.values().filter(|t| !t.settled()).count()
    }

    pub(crate) fn fields_ref(&self) -> &HashMap<ChunkCoord, FieldTile> {
        &self.fields
    }

    /// Overwrite just the given tiles, leaving every other tile in place —
    /// `field::step`'s subset merge: solved tiles land, sleeping tiles are
    /// never cloned or touched. See the `next`-building comment in
    /// `field::step` for the design and the revert it supersedes.
    pub(crate) fn merge_fields(&mut self, solved: HashMap<ChunkCoord, FieldTile>) {
        self.fields.extend(solved);
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
    /// Fill a vertical run in one column, resolving the chunk and the
    /// material's sweep properties once per chunk-row instead of once per
    /// cell.
    ///
    /// **A worldgen seam, and it exists because generation writes the whole
    /// world through `set`.** `stone_massif` alone writes 19.7 M cells at
    /// 8192x2560 and measured **4302 ms** doing it -- 69% of the entire pass
    /// table -- at ~219 ns a cell. Almost none of that is the write: it is
    /// `set` paying, per cell, a `ChunkCoord::containing`, a `HashMap` entry
    /// lookup, two `materials` lookups (`sweep_reach` and `kind`), and
    /// `touch_neighbours`, for a run of cells that share a column, a chunk
    /// and a material.
    ///
    /// Every one of those is loop-invariant over a run. This hoists them:
    /// the chunk is resolved once per 64-row segment, the material once for
    /// the whole run.
    ///
    /// **It is not a bypass of the write seam's bookkeeping.** `set`'s
    /// `managed()` demotion and organism reindexing are still done, per
    /// cell, against the old value -- see `set` for why hooking the write
    /// rather than enumerating callers is load-bearing. What is skipped is
    /// only the repeated *lookup* of things that cannot change within a run.
    /// `every_cell_of_a_filled_run_matches_set` pins the equivalence, and
    /// `scale_probe`'s `WORLD_HASH=1` mode checks it end to end on a
    /// generated world.
    ///
    /// All cells in the run must share `material`; debug builds assert it,
    /// since a caller that varied it would silently get the first cell's
    /// sweep reach applied to the rest.
    pub fn fill_run(&mut self, x: i32, y0: i32, y1: i32, material: MaterialId, mut make: impl FnMut(i32) -> Cell) -> usize {
        let Some(bounds) = self.bounds else { return 0 };
        if x < bounds.min_x || x > bounds.max_x {
            return 0;
        }
        let (lo, hi) = (y0.max(bounds.min_y), y1.min(bounds.max_y));
        if lo > hi {
            return 0;
        }
        let reach = self.materials.get(material).sweep_reach();
        let is_liquid = self.materials.kind(material) == MaterialKind::Liquid;
        let mut written = 0;
        let mut y = lo;
        while y <= hi {
            let coord = ChunkCoord::containing(x, y);
            // The last row of this chunk, or the end of the run.
            let seg_end = hi.min(coord.origin().1 + CHUNK_SIZE - 1);
            let chunk = self.chunks.entry(coord).or_insert_with(|| Chunk::new(coord));
            let mut pending: Vec<(i32, Cell, Cell)> = Vec::new();
            for cy in y..=seg_end {
                let cell = make(cy);
                debug_assert_eq!(cell.material, material, "fill_run cells must share a material");
                let old = chunk.get_world(x, cy);
                chunk.set_world(x, cy, cell, reach, is_liquid);
                written += 1;
                if old.managed() || old.organism_id() != 0 || cell.organism_id() != 0 {
                    pending.push((cy, old, cell));
                }
            }
            // Deferred out of the borrow above, and rare: on generated
            // terrain nothing being overwritten is managed or organism-owned,
            // so this is normally empty.
            for (cy, old, cell) in pending {
                if old.managed() {
                    self.demote_body_at(x, cy);
                }
                self.reindex_organism_cell(x, cy, old.organism_id(), cell.organism_id());
            }
            for cy in y..=seg_end {
                self.touch_neighbours(x, cy, coord);
            }
            y = seg_end + 1;
        }
        written
    }

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

    /// The **water stock** of the organism owning this cell, and its
    /// stomatal term — see `OrganismState::water` for why the balance is
    /// held per organism rather than per cell.
    pub fn water_at(&self, x: i32, y: i32) -> (f32, f32) {
        let id = self.get(x, y).organism_id();
        self.organism(id).map_or((0.0, 1.0), |s| (s.water, s.water_status))
    }

    /// The open-stomata shortfall of the organism owning this cell — what
    /// drought shedding reads. Deliberately not `water_status`: see
    /// `OrganismState::water_desiccation` for why prudence must not read
    /// as thirst.
    pub fn desiccation_at(&self, x: i32, y: i32) -> f32 {
        let id = self.get(x, y).organism_id();
        self.organism(id).map_or(0.0, |s| s.water_desiccation)
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
        // Two numbers, not one, since the palettes grew families.
        //
        // `entries` is the modulus `render::cell_colour` applies; `base` is
        // how many of them a *random* pick may use. They were the same
        // number until worldgen started baking a region's rock family into
        // the shade (`worldgen::passes::palette_family`), and collapsing
        // them again paints stone as confetti of grey, sandstone and
        // bleached cap-rock -- the brush must only ever lay down the first
        // family.
        let entries = self.materials.get(material).palette.len().max(1) as u32;
        let base = self.materials.get(material).base_shades.max(1) as u32;
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
                let existing = self.get(x, y);
                let existing_material = existing.material;
                // **The brush is a destruction path like any other.** Erasing
                // a stamped corpse takes real meat out of the world, and
                // before `meat_lost` existed it did so silently -- so a
                // player tidying up a battlefield could make
                // `max_standing_meat` a lie. Free here: the outgoing cell was
                // already read on the line above for the solid-terrain guard.
                //
                // Only on an erase. Painting over a corpse is blocked for
                // `Solid`/`Plant` and permitted otherwise, and that permitted
                // case destroys meat too -- but it is a *replacement*, and
                // charging it here would double-book against whatever the
                // replacement is worth. Erasing is the unambiguous half.
                if material == material::EMPTY {
                    if let Some(worth) = EnergyLedger::meat_worth_of(&self.materials, existing) {
                        self.energy_ledger.meat_lost += worth;
                    }
                }
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
                //
                // The entry is drawn from `base` and the stride is
                // `entries`: `shade % entries` then lands inside the first
                // family for every draw, which keeps this identical to what
                // it always did for the single-family materials and stops
                // the multi-family ones being painted at random.
                let shade = (self.rng.below(base) + entries * self.rng.below(256 / entries.max(1))) as u8;
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
                // And it is a *disturbance*, not just a reason to re-check:
                // laying stone under an overhang or cutting it out from
                // under one is the player doing something to the world,
                // and `World::chain_reach` only licenses failures near
                // something that happened. Invisible while the default was
                // no limit; the moment `TIGHT` became the default, a brush
                // that scheduled checks but recorded nothing meant erasing
                // a support did precisely nothing.
                // Extent 0: this loop writes one cell, so the wound
                // *is* this cell. A capsule stroke coalesces into a few
                // records rather than one per cell.
                self.record_disturbance(x, y, 0);
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

    /// Register a spring weeping across `span` columns. Refuses past the
    /// flow budget (`spring::MAX_TOTAL_SPAN`, summed over every spring) or
    /// a single span past `spring::MAX_SPAN` — the budget is enforced
    /// here, at registration, loudly, rather than by silently skipping
    /// springs at step time (a cap bounds work; it must not quietly gate
    /// whether a registered thing happens).
    pub fn add_spring(&mut self, x: i32, y: i32, span: i32) -> bool {
        if !(1..=crate::sim::spring::MAX_SPAN).contains(&span) {
            return false;
        }
        let flowing: i32 = self.springs.iter().map(|s| s.span).sum();
        if flowing + span > crate::sim::spring::MAX_TOTAL_SPAN {
            return false;
        }
        self.springs.push(crate::sim::spring::Spring { x, y, span });
        true
    }

    /// Register a drain cell — the spring's inverse; no budget, since a
    /// drain only ever removes work.
    pub fn add_drain(&mut self, x: i32, y: i32) {
        self.drains.push((x, y));
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
        // Last frame's candidates, dropped rather than carried. A caller
        // that owns a `ParticleSystem` drains them right after its step
        // (`App::update`); one that does not -- `examples/ascii.rs`, the
        // unit tests -- simply never sees them, which is the behaviour that
        // makes the effect optional rather than load-bearing.
        self.splash_sites.clear();
        // Idempotent, and the first simulated frame is the right moment:
        // the world has been generated (or hand-built) by now, and nothing
        // has had a chance to dig into it or build on top of it yet, since
        // both of those are things that happen while it runs. A regenerate
        // makes a whole new `World` (`App::reset`), so this cannot go stale.
        self.freeze_sky_surface();
        self.freeze_underground_map();
        self.freeze_ground_datum();
        self.frame = self.frame.wrapping_add(1);
        // No world-time bookkeeping here on purpose. The phase clocks are
        // *derived* from `frame` (`clock::Clock::sky_frame`), not advanced
        // beside it -- an earlier version incremented a counter from this
        // very line and thereby ignored the 27 places that assign
        // `World::frame` directly to select a time of day or a weather
        // window. See that function's own doc; the invariant it restores is
        // what makes "every knob defaults to today's behaviour" true.
    }

    /// **The clock every day/night reader takes, in place of [`World::frame`].**
    ///
    /// `field::sun_elevation` and everything downstream of it — the painted
    /// sky, the light channel, the temperature swing, the weather epoch —
    /// are pure functions of a frame number modulo
    /// `field::DAY_NIGHT_PERIOD_FRAMES`. Feeding them a slower clock is what
    /// lengthens a day; the period itself must not move (`sim::clock`'s
    /// module doc has the full reasoning, and it is a field-sleeping
    /// argument, not a preference).
    ///
    /// Identical to `frame` at the default `day_minutes: 1`.
    pub fn sky_frame(&self) -> u64 {
        self.clock.sky_frame(self.frame)
    }

    /// [`World::sky_frame`] as of the previous real frame — the comparison
    /// point for "has the sky moved", which is not `sky_frame() - 1` under a
    /// slowed clock. See `clock::Clock::prev_sky_frame`.
    pub fn prev_sky_frame(&self) -> u64 {
        self.clock.prev_sky_frame(self.frame)
    }

    /// The weather's own clock — independent of [`World::sky_frame`]. See
    /// `clock::Clock::weather_slowdown`.
    pub fn weather_frame(&self) -> u64 {
        self.clock.weather_frame(self.frame)
    }

    /// A creature-schedule interval scaled by `clock.creature_slowdown`, as an
    /// absolute frame to be due on — the creature twin of
    /// [`World::organism_due`].
    pub fn creature_due(&self, base_interval: u64) -> u64 {
        self.frame + self.clock.creature_interval(base_interval)
    }

    /// The lightning lit at `frame`, if any, with this world's own two
    /// clocks supplied — `weather::strike` needs both, and its doc says why.
    /// `frame` is a *real* frame, so a caller wanting last frame's still-lit
    /// flash passes `world.frame.wrapping_sub(1)` exactly as it always did.
    pub fn lightning_at(&self, frame: u64) -> Option<crate::sim::weather::Strike> {
        crate::sim::weather::strike(self.seed, frame, |f| self.clock.weather_frame(f), self.bounds())
    }

    /// An organism-schedule interval scaled by `clock.growth_slowdown`, as
    /// an absolute frame to be due on. Every `world.frame + INTERVAL` in the
    /// plant subsystem goes through here, which is what makes growth pace a
    /// single knob rather than a sweep of constants.
    pub fn organism_due(&self, base_interval: u64) -> u64 {
        self.frame + self.clock.organism_interval(base_interval)
    }

    /// Record where the ground starts in each column, once. See
    /// `sky_surface`.
    ///
    /// **Ground means `Solid` or `Powder`, and each exclusion is load-bearing.**
    ///
    /// `Plant` and `Creature` are things standing *in* the world rather than
    /// part of it. Worldgen plants trees before the first frame runs, so a
    /// canopy is present at freeze time, and counting it would bake the very
    /// bug this replaced into the one place nothing can later correct: the
    /// air under every generated tree dark for the life of the world.
    ///
    /// `Liquid` and `Gas` are excluded because **a water surface is not a
    /// ground surface, and water levels move.** Counting the top of a lake
    /// would fix the sky at the waterline as it stood on frame one, so
    /// anything that lowered it afterwards — draining it into a shaft,
    /// evaporation, a dam breaking — would leave a band of false cave
    /// hanging in the open air above the new level, creeping down as the
    /// lake fell. Seen in a render: mining into a lake drained it and left a
    /// dimmed strip along the old waterline. Taking the rock beneath as the
    /// surface makes the whole water column read as outdoors, which it is,
    /// and it costs nothing because a liquid cell is not empty and draws as
    /// itself either way.
    pub fn freeze_sky_surface(&mut self) {
        let Some(b) = self.bounds else { return };
        if !self.sky_surface.is_empty() {
            return;
        }
        let width = b.width() as usize;
        self.sky_surface = (0..width as i32)
            .map(|i| {
                let x = b.min_x + i;
                (b.min_y..=b.max_y)
                    .find(|&y| {
                        matches!(self.materials.kind(self.get(x, y).material), MaterialKind::Solid | MaterialKind::Powder)
                    })
                    .unwrap_or(i32::MAX)
            })
            .collect();
    }

    /// The frozen ground surface, one entry per column indexed from
    /// `bounds.min_x`, or empty if it has not been frozen yet — which only
    /// happens for a world nothing has ever stepped.
    pub fn sky_surface(&self) -> &[i32] {
        &self.sky_surface
    }

    /// Record which positions were inside the ground when the world was
    /// made, once. See `underground`.
    ///
    /// A flood fill from the top row through everything that is not `Solid`
    /// or `Powder`, marking everything it *fails* to reach. Deliberately the
    /// same predicate as `freeze_sky_surface`, so the per-cell and
    /// per-column answers cannot disagree about what counts as ground —
    /// water and gas conduct (a lake is outdoors, and its level moves), rock
    /// and soil block.
    ///
    /// **4-connected, not 8.** Air passes through a shared face; two rocks
    /// touching at a corner are not a way out. The same distinction
    /// `diffuse_resource` makes, for the same reason.
    ///
    /// Iterative with an explicit stack rather than recursion: the open air
    /// over a 2048x640 world is a single region of ~400,000 cells and a
    /// recursive fill would blow the stack on the first world that generated
    /// a wide sky.
    pub fn freeze_underground_map(&mut self) {
        let Some(b) = self.bounds else { return };
        if !self.underground.is_empty() {
            return;
        }
        let (w, h) = (b.width() as usize, b.height() as usize);
        let idx = |x: i32, y: i32| (y - b.min_y) as usize * w + (x - b.min_x) as usize;
        let blocks = |world: &Self, x: i32, y: i32| {
            matches!(world.materials.kind(world.get(x, y).material), MaterialKind::Solid | MaterialKind::Powder)
        };

        let mut open = vec![false; w * h];
        let mut stack: Vec<(i32, i32)> = Vec::new();
        for x in b.min_x..=b.max_x {
            if !blocks(self, x, b.min_y) {
                open[idx(x, b.min_y)] = true;
                stack.push((x, b.min_y));
            }
        }
        while let Some((x, y)) = stack.pop() {
            for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                if nx < b.min_x || nx > b.max_x || ny < b.min_y || ny > b.max_y {
                    continue;
                }
                if open[idx(nx, ny)] || blocks(self, nx, ny) {
                    continue;
                }
                open[idx(nx, ny)] = true;
                stack.push((nx, ny));
            }
        }

        self.underground = vec![0u64; (w * h).div_ceil(64)];
        for (i, reached) in open.iter().enumerate() {
            if !reached {
                self.underground[i >> 6] |= 1 << (i & 63);
            }
        }
    }

    /// The frozen per-cell underground map, or empty if it has not been
    /// built — which only happens on a world nothing has ever stepped.
    pub fn underground_map(&self) -> &[u64] {
        &self.underground
    }

    /// Record the top of the ground in each column, once. See
    /// `ground_datum`.
    ///
    /// Walks up from the bottom of the world and stops at the first cell the
    /// sky can reach, so the answer is the top of the **lowest** run of cells
    /// the sky cannot reach rather than the topmost ground of any kind.
    ///
    /// Reads `underground`, so it has to run after `freeze_underground_map`
    /// and does — `begin_step` calls them in order. A column that is
    /// outdoors right down to the world floor holds no ground at all and
    /// takes `i32::MAX`, which is `sky_surface`'s convention for the same
    /// thing.
    pub fn freeze_ground_datum(&mut self) {
        let Some(b) = self.bounds else { return };
        if !self.ground_datum.is_empty() || self.underground.is_empty() {
            return;
        }
        self.ground_datum = (b.min_x..=b.max_x)
            .map(|x| {
                let mut top = i32::MAX;
                for y in (b.min_y..=b.max_y).rev() {
                    if !self.was_underground(x, y) {
                        break;
                    }
                    top = y;
                }
                top
            })
            .collect();
    }

    /// The frozen top-of-ground datum, or empty if it has not been built.
    pub fn ground_datum(&self) -> &[i32] {
        &self.ground_datum
    }

    /// Whether `(x, y)` was inside the ground when the world was made.
    ///
    /// `false` (and therefore "outdoors") for a position outside the world
    /// or on a world that has never been stepped, matching `is_outdoors`'s
    /// own conservative behaviour there.
    pub fn was_underground(&self, x: i32, y: i32) -> bool {
        let Some(b) = self.bounds else { return false };
        if self.underground.is_empty() || x < b.min_x || x > b.max_x || y < b.min_y || y > b.max_y {
            return false;
        }
        let i = (y - b.min_y) as usize * b.width() as usize + (x - b.min_x) as usize;
        self.underground[i >> 6] & (1 << (i & 63)) != 0
    }

    /// Whether `(x, y)` is open to the sky rather than inside the ground.
    ///
    /// The stored definition of "outdoors", asked as a predicate so callers
    /// do not each have to remember how it is indexed.
    ///
    /// **Reads the per-cell map, and falls back to the per-column
    /// `sky_surface` only when there is no map.** The fallback is not a
    /// second opinion — it is the older, coarser form of the same stored
    /// answer, and it is reachable only on a world nothing has ever stepped,
    /// which is a handful of tests: `begin_step` builds both before the
    /// first sweep, so no CA rule can observe the gap. The two agree
    /// everywhere except air that the sky could reach at genesis while
    /// something solid stood above it in the same column — a cliff brow, or
    /// an object hanging in the air — which is precisely the case the map
    /// exists to get right.
    ///
    /// `false` before either has been recorded, which is the conservative
    /// direction: it keeps whatever the indoor behaviour is.
    pub fn is_outdoors(&self, x: i32, y: i32) -> bool {
        let Some(b) = self.bounds else { return false };
        // **Both out-of-range answers are carried over from the column
        // form deliberately**, because `fire.rs`'s condensation asks this
        // and must not change behaviour merely because the storage did. A
        // column outside the world answered `false` (`sky_surface.get`
        // returned `None`); a row outside it answered `y < ground`, so above
        // the world was outdoors and below it was not. The map has no bit
        // for either, so both are restated here rather than left to fall out
        // of the indexing.
        if x < b.min_x || x > b.max_x {
            return false;
        }
        if y < b.min_y {
            return true;
        }
        if y > b.max_y {
            return false;
        }
        if !self.underground.is_empty() {
            return !self.was_underground(x, y);
        }
        let Some(&ground) = self.sky_surface.get((x - b.min_x) as usize) else {
            return false;
        };
        y < ground
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
        // **Where decayable matter gets its decay site**, collected here and
        // scheduled after the loop (the loop holds `self.chunks` mutably, so
        // it cannot call `schedule_active_site`).
        //
        // A decay site is a bare coordinate and nothing makes it follow its
        // cell -- `move_cell` touches no scheduler state -- so scheduling one
        // when the cell is *created* strands it the moment the cell falls,
        // which for shed litter is every time. Scheduling on **settle**
        // instead is not a workaround for that; it is what the rule actually
        // means. Weathering happens to matter that has come to rest, so the
        // awake->settled transition is exactly the event, and a cell that
        // moves afterwards simply gets a fresh site when it settles again.
        // Bounded (one chunk), rare (chunks settle once and stay settled),
        // and free of any hot-path cost. See `Reports/open-bugs-handoff.md`
        // §0 for the four candidates this was chosen over.
        let mut settled_decayables: Vec<(i32, i32)> = Vec::new();
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
                // Rides the scan `recompute_reach` is already doing, on the
                // same transition and for the same reason it was chosen:
                // this is the one point that is both cheap and safe.
                let bounds = chunk.coord.bounds();
                for y in bounds.min_y..=bounds.max_y {
                    for x in bounds.min_x..=bounds.max_x {
                        if materials.get(chunk.get_world(x, y).material).decays_into.is_some() {
                            settled_decayables.push((x, y));
                        }
                    }
                }
            }
        }
        // Deduped inside `schedule_active_site`, which is what stops a drift
        // that settles repeatedly stacking sites and turning the decay rate
        // into a function of how often the ground was disturbed.
        for (x, y) in settled_decayables {
            self.schedule_active_site(ActiveSite { x, y, kind: scheduler::ActiveKind::Decay, next_frame: self.organism_due(decay::DECAY_TICK_INTERVAL) });
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
    fn ground_wetness_at(&self, x: i32, y: i32) -> f32 {
        World::ground_wetness_at(self, x, y)
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

    fn organism_due(&self, base_interval: u64) -> u64 {
        World::organism_due(self, base_interval)
    }

    #[inline]
    fn schedule_active_site(&mut self, site: ActiveSite) {
        World::schedule_active_site(self, site)
    }

    #[inline]
    fn record_disturbance(&mut self, x: i32, y: i32, extent: i32) {
        World::record_disturbance(self, x, y, extent)
    }

    #[inline]
    fn absorb_liquid(&mut self, x: i32, y: i32, fill: u32) {
        World::absorb_liquid(self, x, y, fill)
    }

    #[inline]
    fn report_splash(&mut self, x: i32, y: i32, strength: f32) {
        if self.splash_sites.len() < MAX_SPLASH_SITES {
            self.splash_sites.push((x, y, strength));
        }
    }

    #[inline]
    fn book_meat_lost(&mut self, worth: f64) {
        self.energy_ledger.meat_lost += worth;
    }

    fn count_phase_event(&mut self, event: crate::sim::fire::PhaseEvent) {
        self.phase_changes.record(event);
    }

    #[inline]
    fn is_outdoors(&self, x: i32, y: i32) -> bool {
        World::is_outdoors(self, x, y)
    }

    #[inline]
    fn credit_atmosphere(&mut self, fill: u16) {
        World::credit_atmosphere(self, fill);
    }
}

/// How many splash candidates one frame may record. A cap on *work*, not a
/// gate on whether splashing happens (`CLAUDE.md`) -- every site past this
/// is one more droplet in a frame that already has plenty, and the sweep
/// visits them in a fixed order, so dropping the tail is a look decision
/// rather than a correctness one. Sized well above what a sand blob
/// entering a pool produces so it is a backstop, not the usual path.
pub(crate) const MAX_SPLASH_SITES: usize = 256;

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

    // --- meat_lost: the destruction seam ---------------------------------

    /// A world with a corpse slab in it, every cell stamped `worth`.
    fn meat_world(worth: u16) -> (World, MaterialId, Rect) {
        let mut w = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            w.set(x, 127, Cell::new(material::STONE, 0));
        }
        let corpse = w.materials.id_of("corpse").expect("corpse material");
        let slab = Rect::new(50, 100, 77, 107);
        for y in slab.min_y..=slab.max_y {
            for x in slab.min_x..=slab.max_x {
                w.set(x, y, Cell::new(corpse, 0).with_aux(worth));
            }
        }
        (w, corpse, Rect::new(0, 0, 127, 127))
    }

    /// **Meat that leaves the world is booked, not forgotten.**
    ///
    /// The guard for `EnergyLedger::meat_lost`. Before it, a corpse burnt,
    /// blasted or erased simply stopped existing and nothing recorded it —
    /// which is what made `max_standing_meat` an upper bound rather than a
    /// bound, and it could not be caught by any existing guard: the meat
    /// identity was asserted as `<=`, and `creature_biomass` is asserted
    /// monotone non-increasing, so a *loss* satisfied both.
    ///
    /// Asserted as **conservation across each arm** — meat standing
    /// afterwards plus meat booked lost equals meat standing before — rather
    /// than against the ledger's source terms, because this scene places its
    /// corpses by hand rather than through `creature_dies`, so `stamped` is
    /// legitimately 0 here and an identity built on it would be measuring
    /// the wrong thing.
    ///
    /// **The control arm is the point of the test as much as the other
    /// three.** A hook that fires when nothing is being destroyed is exactly
    /// as wrong as one that never fires, and it is the failure a
    /// conservation assertion alone cannot see.
    #[test]
    fn a_destroyed_corpse_is_booked_rather_than_forgotten() {
        use crate::sim::creature::standing_meat;
        const WORTH: u16 = 1_020;

        // --- control: nothing destroys anything ---------------------------
        {
            let (mut w, _corpse, all) = meat_world(WORTH);
            let before = standing_meat(&w, all);
            for _ in 0..200 {
                crate::sim::update::step(&mut w);
            }
            assert_eq!(
                w.energy_ledger.meat_lost, 0.0,
                "meat was booked lost in a world where nothing burns, blasts or erases"
            );
            assert!(
                (standing_meat(&w, all) - before).abs() < 1.0,
                "the control arm lost meat on its own ({before} -> {})",
                standing_meat(&w, all)
            );
        }

        // --- fire, on both drivers ---------------------------------------
        // `corpse.ron` is flammable and burns into ash, so a body in a
        // grassfire takes its stamp out of the world. Run under both drivers
        // because this is the one destruction path that happens *inside* the
        // CA sweep: the serial one books straight into the ledger and the
        // parallel one tallies per chunk and merges in `run_pass`, and a
        // merge that was dropped would show up here and nowhere else.
        for parallel in [false, true] {
            let (mut w, _corpse, all) = meat_world(WORTH);
            let before = standing_meat(&w, all);
            w.ignite_circle(63, 103, 20);
            for _ in 0..2_000 {
                if parallel {
                    crate::sim::parallel::step(&mut w);
                } else {
                    crate::sim::update::step(&mut w);
                }
            }
            let after = standing_meat(&w, all);
            let lost = w.energy_ledger.meat_lost;
            let driver = if parallel { "parallel" } else { "serial" };
            assert!(lost > 0.0, "the {driver} driver burnt corpses and booked nothing ({before} -> {after})");
            assert!(
                (before - after - lost).abs() < 1.0,
                "{driver}: {before} of meat became {after} standing and {lost} booked lost — \
                 {} unaccounted for",
                before - after - lost
            );
        }

        // --- the brush ----------------------------------------------------
        {
            let (mut w, _corpse, all) = meat_world(WORTH);
            let before = standing_meat(&w, all);
            w.paint_circle(63, 103, 10, material::EMPTY);
            let after = standing_meat(&w, all);
            let lost = w.energy_ledger.meat_lost;
            assert!(lost > 0.0, "the brush erased corpses and booked nothing ({before} -> {after})");
            assert!(
                (before - after - lost).abs() < 1.0,
                "brush: {before} of meat became {after} standing and {lost} booked lost — {} unaccounted for",
                before - after - lost
            );
        }

        // --- an explosion -------------------------------------------------
        // The arm with two halves: a cell the blast *consumes* is meat
        // destroyed, and a cell it *throws* is meat in flight, whose stamp
        // rides `Particle::aux` (bug Z2, fixed alongside this). Booking the
        // throw would put `max_standing_meat` below the truth, so the
        // identity here has to include what is still in the air — which is
        // also the sharpest way to state that the two fixes are halves of
        // one thing.
        {
            let (mut w, corpse, all) = meat_world(WORTH);
            let before = standing_meat(&w, all);
            let mut ps = crate::sim::particle::ParticleSystem::new();
            crate::sim::explosion::trigger(&mut w, &mut ps, 63, 103, 20, 180.0);
            let in_flight: f64 = ps
                .iter()
                .filter(|p| p.material == corpse && p.aux != 0)
                .map(|p| p.aux as f64)
                .sum();
            let after = standing_meat(&w, all);
            let lost = w.energy_ledger.meat_lost;
            assert!(lost > 0.0, "the blast consumed corpses and booked nothing ({before} -> {after})");
            assert!(in_flight > 0.0, "test setup: the blast threw no stamped corpse, so the in-flight half is untested");
            assert!(
                (before - after - lost - in_flight).abs() < 1.0,
                "blast: {before} of meat became {after} standing, {lost} booked lost and {in_flight} in flight — \
                 {} unaccounted for",
                before - after - lost - in_flight
            );

            // And once it lands, the in-flight half is standing again and
            // nothing further is booked.
            let lost_before_landing = w.energy_ledger.meat_lost;
            for _ in 0..600 {
                if ps.is_empty() {
                    break;
                }
                ps.step(&mut w);
            }
            assert!(ps.is_empty(), "{} particles never landed", ps.len());
            assert_eq!(
                w.energy_ledger.meat_lost, lost_before_landing,
                "landing a particle booked meat as lost — a throw is not a destruction"
            );
            let landed = standing_meat(&w, all);
            assert!(
                (before - landed - lost).abs() < 1.0,
                "after landing: {before} of meat became {landed} standing and {lost} booked lost — {} unaccounted for",
                before - landed - lost
            );
        }
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
        let id = w.push_organism(species).expect("an organism slot is free");
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
        let a = w.push_organism(species_a).expect("an organism slot is free");
        let b = w.push_organism(species_b).expect("an organism slot is free");
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
