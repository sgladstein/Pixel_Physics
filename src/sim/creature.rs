//! M18 Phase 1: cell-based creatures — a worm that burrows through loose
//! material.
//!
//! Dispatched from `scheduler::step` once per due active site, the same
//! shape M16's `plant.rs` and M17's `structural.rs` use — everything here
//! writes through the ordinary `World::get`/`set`, since this runs as its
//! own frame phase separate from the CA sweep. Fire (catching light,
//! burning to a corpse) is deliberately *not* implemented here at all: M14's
//! `fire.rs` already applies to every material kind uniformly, purely from
//! `.ron` data (flammability, ignition temperature, `burns_into`), so giving
//! `worm.ron` real thermal numbers gets a burning, dying worm for free.
//! This module only owns the part fire.rs can't: *choosing to move*.
//!
//! Grounded in `research/m18-creature-biology.md`, not invented from
//! plausible-looking rules — see that file for full citations. The three
//! mechanisms translated directly:
//!
//! 1. **Burrowing cost from substrate physics, not a material whitelist**
//!    (Kurth et al. 2018; golden mole energetics) — see `move_cost`.
//! 2. ***C. elegans*-style thermotaxis** for fire avoidance — compare the
//!    local M13 ambient-temperature field against a threshold, flee
//!    down-gradient once it's crossed. See `sense_fire`.
//! 3. **An energy budget driving movement**, replacing random wandering —
//!    depletes on every move (more on burrowing than open ground), is
//!    partially replenished by "eating" the powder it burrows through, and
//!    starvation is what actually kills a permanently-trapped worm rather
//!    than a separate dormancy counter (contrast `plant.rs`'s
//!    `MOSS_STALE_LIMIT`, which exists because moss has no energy stat to
//!    let staleness resolve itself).
//!
//! **A creature is an organism** (`Reports/creature-direction.md` §3a).
//! State lives in `OrganismState`, identity is the generational handle in
//! `Cell::organism_id`, the species comes from `SpeciesRegistry`
//! (`worm.ron`), and the cell's `aux` holds a `CellType` like every other
//! organism cell. The parallel scheme this replaced — a `CreatureState`
//! vector indexed by a raw `u16` written into `Cell::aux` — had no
//! generations (a site outliving its creature read whoever had been
//! allocated that index since), no reclamation (`World::creatures` never
//! shrank, and fire writing a corpse over a worm leaked its entry for the
//! life of the process), and a `u16` overflow guarded only by a
//! `debug_assert`. All three were already solved by the organism
//! substrate, which is why this was **retired rather than extended**:
//! extending it would have been the third private solution to
//! per-organism state, the exact failure `Reports/organism-substrate-
//! design.md` opens on.
//!
//! Movement is code here, not a composed `Behavior`, and `worm.ron`
//! declares both its cell types with empty behavior lists to say so.
//!
//! Known simplification, not yet built: the Marginal Value Theorem's
//! patch-leaving rule (leave once local intake drops below the
//! environment's running average) needs a maintained average-intake
//! estimate this first cut doesn't keep. What's here instead — prefer
//! burrowing into powder over drifting through open space, whenever both
//! are available — captures "the worm has a reason to move" without the
//! full bookkeeping; a closer match is future work. Multiple creature kinds
//! interacting (Wa-Tor-style predator/prey) and the slime/fungus shared
//! resource-gradient primitive are also out of scope for this first cut —
//! see the research file's section 4 and 5.

use super::brain;
use super::cell::{Cell, AMBIENT_TEMPERATURE};
use super::chunk::Rect;
use super::field;
use super::material::{self, MaterialKind};
use super::organism::{pack_cell_type, Carried, CellType, CreatureDef, Flight, ShadeRule, SpeciesId, CREATURE_TRAITS, TRAIT_BIRTH_GRANT, TRAIT_GUT_BIAS};
use super::pheromone::{self, Channel};
use super::rng;
use super::scheduler::{ActiveKind, ActiveSite};
use super::world::World;

/// Index 0 = east, then counterclockwise on screen (y grows downward, so
/// `(1, -1)` is up-and-right). **The one heading table** — see
/// `OrganismState::heading`: headings are a discrete 0..8 compass index, a
/// turn of ±1 is exactly 45 degrees (the Physarum literature default), and
/// no decision in this engine goes anywhere near `sin_cos` (P-19).
pub const DIRS: [(i32, i32); 8] = [(1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1), (0, 1), (1, 1)];

const NEIGHBOURS_4: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
/// A walking creature reads and moves at eight neighbours, not four: it
/// climbs, it steps diagonally, and its support check has to use the same
/// neighbourhood its movement does or it will judge itself unsupported
/// while standing on a corner. (`CLAUDE.md`: a traversal must use the same
/// neighbourhood the writer used.)
const NEIGHBOURS_8: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// Which draw a creature is making, as the fourth key of `rng::stream` —
/// so two decisions taken by the same creature on the same frame do not
/// share a sequence.
///
/// **Slots are positional and append-only**, the same law the plant genome
/// and `CellType`'s discriminants live under: renumbering one silently
/// re-rolls every creature decision in the engine, which reads as "the
/// worms behave differently now" with nothing to point at.
///
/// **Keyed on the creature's own identity, never on its position.** A
/// fitness-relevant draw keyed on where something is makes *location* a
/// hidden inherited variable, which `Reports/plant-simulation-research.md`
/// §7d names as the confound that manufactures spurious evolution results
/// — and this module is the one that will shortly have heredity attached
/// to it. `plant.rs`'s `(id, x, y, frame)` keying is the shape **not** to
/// copy here; fixing it for plants is its own migration
/// (`Reports/creature-direction.md` §2c).
const RNG_SLOT_SHADE: u64 = 0;
const RNG_SLOT_MOVE: u64 = 1;
const RNG_SLOT_DEATH: u64 = 2;
/// The birth stream: mutation of the child's genome and of its traits.
///
/// **Keyed on the child's handle *and* the frame**, which is what keeps it
/// clear of dead end 670. A handle is a slot index plus a 4-bit
/// generation, so `push_organism` hands the same encoded id back after
/// sixteen reuses of one slot — and those repeats cluster spatially,
/// because slots come off a free list a colony fills in place. Keying a
/// genome on the handle alone would therefore hand two cousins born in the
/// same corner of the world byte-identical mutations. The frame breaks it
/// by construction: a handle can only recur after the organism holding it
/// has died, which cannot happen on the frame it was issued.
///
/// It is also a **separate stream from `RNG_SLOT_MOVE`** rather than a
/// continuation of the tick's own generator, and that is the
/// shared-`Rng` gotcha rather than tidiness: `brain::mutate` takes a
/// variable number of draws, so drawing them from the tick's generator
/// would shift every later draw *for that animal* by an amount depending
/// on how many slots happened to mutate — a determinism change that
/// spreads through the whole colony and that both of the guards over the
/// last instance of it stayed green through.
const RNG_SLOT_BIRTH: u64 = 3;

/// Frames between a worm's movement decisions. Faster than plant growth
/// (20-45 frames) — a worm actively moving through the world reads as more
/// lively than something merely growing — but not every frame, since a
/// worm's decision does not need CA-sweep granularity to look right.
const WORM_TICK_INTERVAL: u64 = 6;

/// Starting energy budget for a newly planted worm. Large enough to
/// burrow through several dozen sand cells before starving on this
/// milestone's constants — enough to be a real, watchable lifespan in the
/// ascii/live-app demos rather than an instant death.
const WORM_START_ENERGY: f32 = 400.0;

/// Energy cost to move into open space (`Empty`/`Gas`) — the baseline
/// "moving across open ground" cost the burrow multiplier below is scaled
/// against.
const WORM_MOVE_COST_OPEN: f32 = 1.0;

/// Energy cost to stay put for one tick when completely boxed in (every
/// neighbour impassable). Small but nonzero: a real trapped animal still
/// burns basal metabolic energy, which is what eventually kills a
/// permanently-sealed worm without needing a separate dormancy counter the
/// way `plant.rs`'s stale-tip tracking does for moss.
const WORM_IDLE_COST: f32 = 0.3;

/// Multiplies a target `Powder`'s own `density` to get its burrow cost on
/// top of `WORM_MOVE_COST_OPEN` — tied to the material's already-tracked
/// physical property rather than a hardcoded material-kind check, per the
/// research's own recommendation, so a future denser/looser granular
/// material gets a proportionally different cost automatically. Tuned so
/// sand (density 1.6) costs roughly 20x open-ground movement — within the
/// ~10-30x band the Namib golden mole's measured 26x (80 J/m burrowing vs.
/// 3 J/m on the surface) supports; gravel (1.9) costs more, ash (1.1) less,
/// which is the generalization the flat-multiplier alternative would not
/// have given for free.
const WORM_BURROW_DENSITY_COST: f32 = 12.0;

/// Reference scale for normalizing a `field_at(..).moisture` reading into a
/// 0..1 "how saturated" fraction — matches `field.rs`'s own private
/// `MAX_MOISTURE`, which isn't reachable from here, so this is a documented
/// assumption about that scale rather than a shared constant.
const WORM_MOISTURE_SATURATION: f32 = 4.0;
/// Maximum fraction burrow cost is discounted by at full saturation —
/// architecture §4's "worm burrowing" consumer. The cited research
/// (`research/m18-creature-biology.md` §1) says moisture modulates
/// substrate resistance but does not say which direction, so this is a
/// judgment call, not a cited number: slightly damp substrate holds a
/// tunnel shape and is easier to displace than the same material bone-dry
/// (the everyday "damp sand castle vs. dry sand collapsing" effect), so
/// moisture makes burrowing *cheaper* here, capped well short of free —
/// fully waterlogged ground is a harder regime this engine doesn't model,
/// not an easier one, so this is deliberately not allowed to approach 1.0.
const WORM_MOISTURE_DISCOUNT: f32 = 0.3;

/// Energy recovered from burrowing into (eating) a `Powder` cell. Kept
/// smaller than a typical burrow's own cost, so burrowing is still a net
/// energy expense on its own — real foraging value comes from doing it
/// somewhere worth the cost, not from burrowing being free.
const WORM_ENERGY_FROM_EATING: f32 = 8.0;

/// Degrees above ambient the M13 field must read before a worm treats its
/// current position as dangerous and flees down-gradient. Not the field's
/// own default temperature (which would flee constantly at rest) —
/// comfortably above ordinary field noise but well below a real fire's
/// heat spike, so a worm reacts to actual danger, not to sitting near
/// slightly warm ground.
const WORM_HEAT_THRESHOLD_ABOVE_AMBIENT: f32 = 25.0;

/// **A length authored in cells, in a world built at a finer resolution.**
///
/// The module-level companion to `CreatureDef::scaled`, for the lengths
/// that live in the *source* rather than in a species file -- a
/// neighbourhood radius, a colony's spacing, a telemetry bucket edge.
/// `CLAUDE.md`'s rule is that reading `World::cell_scale` is the only
/// supported way for such a constant to find out the world got finer, and
/// this is that read, in one place, so a site cannot get the rounding
/// subtly different from its neighbour.
///
/// Floors at 1: a length authored as at least one cell must not round away
/// to zero, which would turn a neighbourhood into a single cell and read as
/// the sense being dead rather than as being mis-scaled.
#[inline]
fn scaled_cells(world: &World, authored: i32) -> i32 {
    if authored == 0 {
        return 0;
    }
    ((authored as f32 * world.cell_scale()).round() as i32).max(1)
}

/// How strongly a worm is allowed to pick the *wrong* neighbour.
///
/// The additive term `k` in `choose_weighted`'s `(k + s)²`. At `0.1`, a
/// candidate scoring 0 against one scoring 1 is chosen `0.01 / 1.22`, about
/// **0.8% of ticks** — enough that a worm never gets deterministically
/// wedged, small enough that fleeing a fire still looks purposeful.
pub const CHOICE_EXPLORATION_K: f32 = 0.1;

/// Lower edge of each bucket in `CreatureStats::forage_reach`, in Chebyshev
/// cells. Doubling, because the interesting range spans a nest patch (tens
/// of cells) and a world (hundreds) and a linear ruler would spend six of
/// its eight buckets inside the nest.
pub const FORAGE_REACH_BUCKETS: [u16; 8] = [1, 2, 4, 8, 16, 32, 64, 128];

/// How far from home an excursion has to reach before returning counts as a
/// **foraging trip** (`CreatureStats::forage_trips`), in Chebyshev cells.
///
/// **Set from the control, with headroom, not from an aspiration.** The arm
/// it has to survive is one ant, a nest and no food at all -- a colony that
/// by construction cannot forage. Measured on that arm over 6,000 frames
/// (`examples/forage_probe.rs`), as excursions reaching at least N cells:
///
/// ```text
/// >=1: 340   >=2: 4   >=4: 3   >=8: 3   >=16: 1   >=32: 1   >=64: 0
/// ```
///
/// The loitering spike is at depth **exactly 1** and nowhere else: 340
/// collapses to 4 at the first doubling. So 1 is useless -- it reproduces
/// `nest_visits` exactly (340 against 340, measured on that same run),
/// which is the counter this one exists to replace.
///
/// **8**, then: three doublings above the noise floor, and equal to the
/// diameter of `CROWDING_RADIUS`'s neighbourhood, so a jammed knot of ants
/// shuffling around the nest mouth provably cannot manufacture one. Higher
/// was tried and rejected for a reason worth recording: at 16 the 55-ant
/// foraging scene reads **0 trips** against a real deepest excursion of 12
/// cells, so a colony that improved from 12-cell ranging to 15-cell ranging
/// would show 0 -> 0 and the headline would hide the progress it exists to
/// report.
///
/// **The bar is not what makes this metric trustworthy.**
/// `CreatureStats::forage_reach` is, and it has no bar; this is the one
/// number for scenes that want one. Read the profile before believing a
/// trip count, in either direction.
pub const FORAGE_TRIP_MIN: u16 = 8;

/// Plant a worm at `(x, y)` if the position is available and both the
/// `worm` material and the `worm` species are loaded. Returns the site to
/// schedule, or nothing if any precondition failed.
pub fn plant_worm_seed(world: &mut World, x: i32, y: i32) -> Option<ActiveSite> {
    let worm_id = world.materials.id_of("worm")?;
    let worm_species = world.species.id_of("worm")?;
    // P-3: `is_empty` is the **managed-aware** check, and it is the right
    // one here on purpose. The question being asked is "is this position
    // available", not "is there material here" -- a promoted liquid body's
    // container cells are materially empty and are *not* available. The two
    // checks differ exactly when liquid bodies go live, so which one this
    // is has to be a decision rather than a habit.
    //
    // Before `push_organism`, so refusing to plant does not leak a slot.
    if !world.is_empty(x, y) {
        return None;
    }
    // The handle first, because the shade draw is keyed on it -- see
    // `RNG_SLOT_SHADE`. Ordering the other way round would need a
    // placeholder identity, which is how position-keyed draws get
    // reintroduced by accident.
    // At the slot ceiling nothing hatches -- `World::push_organism` counts
    // the refusal. `None` is the answer this already gives for an occupied
    // cell, so the caller's handling is unchanged.
    let organism = world.push_organism(worm_species)?;
    let shades = world.materials.get(worm_id).palette.len().max(1) as u32;
    let shade = rng::stream(world.seed, organism as u64, world.frame, RNG_SLOT_SHADE).below(shades) as u8;
    world.set(x, y, Cell::new(worm_id, shade).with_organism_id(organism).with_aux(pack_cell_type(CellType::Head)));
    if let Some(state) = world.organism_mut(organism) {
        state.energy = WORM_START_ENERGY;
        // A worm is a chain of one. Ants are 2-3; nothing here assumes the
        // length is 1 except `worm_tick`'s own 4-neighbour candidate set.
        state.chain = vec![(x, y)];
    }
    Some(ActiveSite { x, y, kind: ActiveKind::Creature { organism }, next_frame: world.creature_due(WORM_TICK_INTERVAL) })
}

/// Dispatch a due `ActiveKind::Creature` site to `worm_tick`. `scheduler::step`
/// never routes any other `ActiveKind` here.
pub fn tick(world: &mut World, site: &ActiveSite) -> Vec<ActiveSite> {
    let ActiveKind::Creature { organism } = site.kind else {
        unreachable!("scheduler::step only routes ActiveKind::Creature to creature::tick");
    };
    // A stale handle: this creature's slot was freed and may since have
    // been handed to something else. Drop the site silently -- the whole
    // point of the generational scheme.
    if world.organism(organism).is_none() {
        return Vec::new();
    }
    // **The near half of the "did it fire" pair** — see
    // `CreatureStats::ticks`. Counted here rather than in `scheduler::step`
    // because this is the point at which the site is known to belong to a
    // live creature; a stale handle above has already returned, and
    // counting those would make the ratio against `moves` mean nothing.
    //
    // `tick_lag_*` is the same reading in the time axis. `pop_due_active_
    // site` only yields sites with `next_frame <= world.frame`, so the
    // subtraction never wraps and zero is the on-time case.
    world.creature_stats.ticks += 1;
    let lag = world.frame.saturating_sub(site.next_frame);
    world.creature_stats.tick_lag_sum += lag;
    world.creature_stats.tick_lag_max = world.creature_stats.tick_lag_max.max(lag);
    let state = world.organism(organism).expect("resolved live above");
    // **Species data is the dispatch, not a name check.** A species with a
    // `creature:` block is a chain creature with a brain; one without is
    // the worm, which keeps its own researched burrowing economics. See
    // the section header above `plant_creature_seed` for why these are
    // deliberately two locomotion models rather than one parameterised one.
    match world.species.get(state.species).creature.clone() {
        Some(def) => creature_tick(world, site.x, site.y, organism, &def),
        None => worm_tick(world, site.x, site.y, organism),
    }
}

/// Pick an index from `scores` with probability proportional to
/// `(k + max(sᵢ, 0))²` — Deneubourg's nonlinear choice, with squaring as
/// the nonlinearity so there is no `exp` anywhere near a decision (P-19:
/// transcendentals are the named cross-platform determinism trap).
///
/// **Never replace this with `min_by`/`max_by`/argmax.** That is a specific,
/// published failure (`Reports/stigmergy-research.md` §2, P-10): the noise
/// is load-bearing, not a nuisance term. Deterministic selection removes the
/// exploration every trail-laying mechanism depends on, and it removes it
/// invisibly — the agents still move, they just stop finding anything. The
/// version of this function that shipped before was an `Iterator::min_by`,
/// which additionally returns the *first* minimum on a tie, so a worm whose
/// neighbours read equal always fled west.
///
/// `k > 0` keeps exploration alive when every score is ~0, which is the
/// common case: an undisturbed field reads flat, and a follower with no
/// signal must still wander rather than freeze.
///
/// Slice-generic because the worm offers up to four candidates and the ants
/// will offer three (ahead, ahead-left, ahead-right).
pub fn choose_weighted(scores: &[f32], k: f32, draw: f32) -> usize {
    let mut total = 0.0;
    for &s in scores {
        let b = k + s.max(0.0);
        total += b * b;
    }
    if total <= 0.0 {
        return 0;
    }
    let mut t = draw * total;
    for (i, &s) in scores.iter().enumerate() {
        let b = k + s.max(0.0);
        if t < b * b {
            return i;
        }
        t -= b * b;
    }
    // Only reachable on floating-point drift at the very top of the range.
    scores.len() - 1
}

fn worm_tick(world: &mut World, x: i32, y: i32, organism: u16) -> Vec<ActiveSite> {
    // A stale handle: this worm's slot was freed, and may since have been
    // handed to something else entirely. Drop the site silently -- that is
    // precisely what the generational scheme exists to make safe, and it is
    // what the old raw-index scheme could not do at all.
    if world.organism(organism).is_none() {
        return Vec::new();
    }
    let Some(worm_id) = world.materials.id_of("worm") else {
        return Vec::new();
    };
    let cell = world.get(x, y);
    // The cell may have burned into a corpse (fire.rs, uniformly for every
    // material kind), been erased by the brush, or been cleared by an
    // explosion by the time this tick is due -- nothing left to move. The
    // `organism_id` half also catches the case where something else has
    // since been written here that merely happens to be worm material.
    if cell.material != worm_id || cell.organism_id() != organism {
        release_if_bodyless(world, organism);
        return Vec::new();
    }
    // The move below (`worm_tick`'s `Cell::new(worm_id, cell.shade)...`)
    // rebuilds the cell from scratch rather than copying it, which would
    // silently clear `FLAG_BURNING` and the burn timer with it -- moving a
    // burning worm would extinguish it the instant it next takes a step.
    // `aux` itself is unaffected either way (a separate field from the burn
    // timer now, per `Cell`'s own doc), but the flags/timer loss is real, so
    // defer instead, the same as `structural::tick` does, and let `fire.rs`
    // (which runs independently, every visited CA frame) finish deciding
    // this worm's fate before creature.rs touches it again.
    //
    // **Kept, even though the move below now carries the whole `Cell` and
    // so no longer strips the flag.** Two independent guards against one
    // bug is not the reason; the reason is that this one is also a
    // behavioural statement -- a burning worm stops choosing and burns --
    // and removing it is a behaviour change this migration deliberately is
    // not making. Note the consequence for testing: because a burning worm
    // never reaches the move, `a_burning_worm_keeps_burning...` cannot fail
    // if the move regresses to a `Cell::new` rebuild. See
    // `a_moving_worm_carries_its_whole_cell`, which is the guard that can.
    if cell.is_burning() {
        return vec![ActiveSite { x, y, kind: ActiveKind::Creature { organism }, next_frame: world.creature_due(WORM_TICK_INTERVAL) }];
    }

    // C. elegans-style thermotaxis: read the local ambient field, and if
    // it's dangerously hot, every subsequent choice is about descending the
    // gradient rather than foraging.
    // The noon-equivalent reading, not the raw one: the field's temperature
    // channel swings with the day/night cycle (`field::sky_temperature_
    // offset`), and a fixed threshold sampled at an arbitrary phase of a
    // designed oscillator is a different threshold every hour — the same law
    // `LightHere` below already obeys. A worm must not decide the world has
    // become dangerous because the afternoon is warm.
    let fleeing =
        field::noon_equivalent_temperature(world.field_at(x, y)) - AMBIENT_TEMPERATURE as f32 > WORM_HEAT_THRESHOLD_ABOVE_AMBIENT;

    let candidates: Vec<(i32, i32, f32)> = NEIGHBOURS_4
        .iter()
        .filter_map(|&(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            let target = world.get(nx, ny);
            // Never move into (eat, or flee through) a cell that is
            // actively burning -- fire is exactly the danger thermotaxis
            // exists to avoid, and an unburnt neighbour is never worth
            // eating badly enough to walk into the flame doing it.
            if target.is_burning() {
                return None;
            }
            move_cost(world, nx, ny, target.material).map(|cost| (nx, ny, cost))
        })
        .collect();

    if candidates.is_empty() {
        // Boxed in on every side -- pay the idle cost and try again later.
        // No separate dormancy tracking needed: unlike moss (which has no
        // energy stat), a permanently-trapped worm's own starvation is what
        // eventually stops it being rescheduled.
        return apply_energy_delta(world, x, y, organism, -WORM_IDLE_COST);
    }

    let mut draw = rng::stream(world.seed, organism as u64, world.frame, RNG_SLOT_MOVE);
    let (tx, ty, cost) = if fleeing {
        // Move toward whichever reachable neighbour reads coolest --
        // descending the field's temperature gradient is the entire
        // mechanism, same as the real AFD neuron comparing against a
        // remembered set-point.
        //
        // Bilinear, not block-nearest (architecture §6a) — every candidate
        // here is only 1 cell from `(x, y)`, well inside the same
        // `FIELD_SCALE`-sided field block `field_at` would read, which
        // otherwise makes every candidate compare temperature-equal.
        //
        // **Scored and sampled, not argmin.** The coolest candidate scores
        // 1.0 and the hottest 0.0, normalized across whatever spread this
        // particular set of neighbours happens to have, so the choice is
        // about the *shape* of the local gradient rather than its
        // magnitude. `choose_weighted`'s own doc carries the never-argmax
        // law and what the `min_by` that used to be here cost. A flat
        // reading (`t_max == t_min`) scores every candidate 0 and the
        // choice falls through to uniform exploration, which is the
        // correct answer to "no gradient" and is what the old code got
        // exactly wrong.
        //
        // **Noon-equivalent here too, even though this is a gradient and a
        // uniform offset would cancel.** The sky's offset is not uniform: it
        // is attenuated with depth (`field::apply_sky_temperature_to`), so
        // between a burrowing worm's own block and the one above it there is
        // a standing step of the sky's own making. Measured, one field cell
        // under a rock surface: **+4.8 degrees at noon and -4.8 at
        // midnight**, against a thermal difference of 0.000 at both — a
        // taxis signal larger than most real ones, pointing up all day and
        // down all night, with no heat behind it. See `field.rs`'s
        // `the_sky_leaves_a_vertical_gradient_in_the_raw_channel_and_none_
        // in_the_thermal_one`, which is that measurement as a guard. Costs
        // one subtraction per candidate.
        let temps: Vec<f32> = candidates
            .iter()
            .map(|c| field::noon_equivalent_temperature(world.field_at_bilinear(c.0 as f32, c.1 as f32)))
            .collect();
        let t_max = temps.iter().copied().fold(f32::MIN, f32::max);
        let t_min = temps.iter().copied().fold(f32::MAX, f32::min);
        let scores: Vec<f32> = temps.iter().map(|t| (t_max - t) / (t_max - t_min + 1e-6)).collect();
        candidates[choose_weighted(&scores, CHOICE_EXPLORATION_K, draw.unit_f32())]
    } else {
        // Foraging: prefer burrowing into powder (food) over drifting
        // through open space when both are available, picked at random
        // among whichever tier is on offer so the worm doesn't always
        // burrow the same direction. See the module doc for why this is a
        // simplified stand-in for the Marginal Value Theorem's full
        // patch-leaving rule rather than that rule itself.
        let powder: Vec<(i32, i32, f32)> = candidates
            .iter()
            .copied()
            .filter(|c| world.materials.kind(world.get(c.0, c.1).material) == MaterialKind::Powder)
            .collect();
        let pool: &[(i32, i32, f32)] = if powder.is_empty() { &candidates } else { &powder };
        pool[draw.below(pool.len() as u32) as usize]
    };

    let target = world.get(tx, ty);
    let ate = world.materials.kind(target.material) == MaterialKind::Powder;

    // Move: the worm cell takes the target position; whatever was there is
    // left behind at the worm's old position -- an earthworm's actual
    // ingest-ahead, cast-behind burrowing behaviour (see the research file),
    // simplified to "the material passes through" rather than modelling
    // digested organic content separately.
    //
    // **P-1: the two whole `Cell` values are swapped, not rebuilt.** The
    // version before this one wrote `Cell::new(worm_id, cell.shade)...`,
    // which silently dropped `FLAG_BURNING` and the burn timer -- a worm
    // survived every fire it caught, simply by taking its next step. That
    // was patched twice (a `.with_temperature` here, a burning-defer
    // above); swapping the cells deletes the whole bug class at the root
    // instead, and it is the difference between a fix and a fix per field:
    // a chain creature moves N cells per step, so anything a rebuild
    // forgets is forgotten N times.
    //
    // Serial active-site phase, so plain `World::set` is correct and
    // `MAX_REACH` does not bind. Organism membership is maintained at that
    // seam automatically (`World::set` -> `reindex_organism_cell`): the
    // first write drops `(x, y)`, the second inserts `(tx, ty)`. The moved
    // cell therefore gets a fresh, zeroed `OrganismCell` sidecar -- a
    // documented limitation of that seam, and harmless here because a worm
    // carries no per-cell scalars.
    world.set(x, y, target);
    world.set(tx, ty, cell);
    if let Some(state) = world.organism_mut(organism) {
        state.chain = vec![(tx, ty)];
    }

    let delta = if ate { WORM_ENERGY_FROM_EATING - cost } else { -cost };
    apply_energy_delta(world, tx, ty, organism, delta)
}

/// Release an organism whose last cell has left the world — burned into a
/// corpse, erased by the brush, blown up.
///
/// **This closes a leak the old scheme had no way to close.**
/// `World::creatures` never shrank and nothing told it a creature was gone,
/// so fire writing a corpse over a worm left its `CreatureState` allocated
/// for the life of the process. Here it costs one emptiness check, because
/// `World::set`'s own seam already dropped the cell from the organism's
/// list at the moment it changed hands.
fn release_if_bodyless(world: &mut World, organism: u16) {
    if world.organism(organism).is_some_and(|state| state.cells.is_empty()) {
        world.free_organism(organism);
    }
}

/// Reconcile a creature's `chain` with the cells it actually still owns,
/// and kill it outright if the head is gone.
///
/// **The prerequisite for anything eating anything.** Removing a cell —
/// a bite, a brush, an explosion, a falling rock — empties it from the
/// organism's list at the `World::set` seam, but `chain` is a *separate*
/// sequence and nothing was reconciling the two. An organism only released
/// itself when *every* cell was gone, so biting the head off a two-cell ant
/// left an orphaned segment standing in the world, owned by a live
/// organism, driving decisions from a stale chain whose first entry was a
/// cell that no longer existed.
///
/// Losing the head is death, not a shortening: the head is the deciding
/// cell, it carries the heading, and it is the position the active site is
/// scheduled at. Losing a trailing segment is just an injury.
///
/// Returns whether the creature is still alive.
fn reconcile_chain(world: &mut World, organism: u16) -> bool {
    let Some(state) = world.organism(organism) else {
        return false;
    };
    // **Not every organism is a chain, and this one line is the difference
    // between herbivory and clear-felling.** `act`'s eat branch tells the
    // owner of whatever it just swallowed, and the owner of a leaf is a
    // *tree* -- an organism whose `chain` is empty, because a plant does
    // not have one. Without this guard the emptiness fell straight through
    // to "head gone, the rest is meat": eating a single leaf off a
    // 789-cell tree freed the tree's organism slot outright, left 160 cells
    // standing in the world still claiming to be it, and stopped it growing
    // or regrowing anything ever again.
    //
    // That is the reason the renewable food source was not renewable, and
    // it is upstream of §13k's whole conclusion: the first ant to take a
    // leaf killed the tree that was supposed to keep making them, so
    // raising `eat_energy` could not pay because the supply destroyed
    // itself on contact.
    //
    // A plant finds out it has lost a cell through its own connectivity
    // check, which is exactly what made herbivory need no new code. Slot
    // reclamation for a fully dead plant is a separate, known gap
    // (`plant.rs`, "an organism's id slot is never returned"); it wants a
    // BFS-from-roots liveness check, and it is emphatically not something
    // to do from inside a bite.
    if state.chain.is_empty() {
        return true;
    }
    let (chain, owned) = (state.chain.clone(), state.cells.clone());
    let surviving: Vec<(i32, i32)> = chain.iter().copied().filter(|p| owned.contains_key(p)).collect();
    if surviving.is_empty() || surviving.first() != chain.first() {
        // Head gone (or nothing left at all): the rest is meat.
        creature_dies(world, organism);
        return false;
    }
    if surviving.len() != chain.len() {
        // **The living-flesh stamp seam, closing here** (`Reports/
        // creature-evolution-plan.md` §2.3, "One seam left open";
        // `EnergyLedger::meat_lost`'s doc points at it from the other
        // end). A body cell was stamped with `body_energy` of meat at the
        // moment the animal was built, and that stamp only ever became
        // standing food by way of a corpse. A cell bitten, burned or
        // blasted off a *living* animal never reaches `creature_dies`, so
        // its stamp stayed in the meat accounts against nothing at all —
        // which is what made `max_standing_meat` an upper bound rather
        // than a bound. It is a sink, exactly like a corpse cell the
        // brush erases; the difference was never the accounting, it was
        // that there was nothing paying stamps *in* until a parent
        // started paying for its children.
        //
        // Booked here rather than at each destruction site because a live
        // creature's material is not `worth_in_aux`, so
        // `EnergyLedger::meat_worth_of` returns `None` for it and none of
        // those sites can see the loss. This is the one place that knows
        // a body cell went missing while its owner lived, and it already
        // holds both counts for its own reasons.
        let lost = (chain.len() - surviving.len()) as f64;
        let body_energy = world
            .organism(organism)
            .and_then(|s| world.species.get(s.species).creature.as_ref().map(|d| d.body_energy))
            .unwrap_or(0.0) as f64;
        world.energy_ledger.meat_lost += body_energy * lost;
        if let Some(state) = world.organism_mut(organism) {
            state.chain = surviving;
        }
        world.creature_stats.injuries += 1;
    }
    true
}

/// Energy cost to move into `target_material` at `(x, y)`, or `None` if
/// it's not enterable at all. `Solid` (stone), `Liquid` (not modelled as
/// aquatic this milestone), `Plant` and other `Creature` cells are all
/// impassable; `Empty`/`Gas` cost the flat open-ground rate; `Powder` scales
/// with the target's own `density`, tying burrow cost to already-tracked
/// physical data rather than a material-kind whitelist (see `WORM_BURROW_
/// DENSITY_COST`), then discounted by local ambient moisture (architecture
/// §4 — see `WORM_MOISTURE_DISCOUNT`'s own doc for the direction and why).
fn move_cost(world: &World, x: i32, y: i32, target_material: material::MaterialId) -> Option<f32> {
    match world.materials.kind(target_material) {
        MaterialKind::Empty | MaterialKind::Gas => Some(WORM_MOVE_COST_OPEN),
        MaterialKind::Powder => {
            let base = WORM_MOVE_COST_OPEN + world.materials.density(target_material) * WORM_BURROW_DENSITY_COST;
            let saturation = (world.field_at(x, y).moisture / WORM_MOISTURE_SATURATION).clamp(0.0, 1.0);
            Some(base * (1.0 - saturation * WORM_MOISTURE_DISCOUNT))
        }
        MaterialKind::Solid | MaterialKind::Liquid | MaterialKind::Plant | MaterialKind::Creature => None,
    }
}

/// Apply an energy delta, then either reschedule the worm (if it survived)
/// or kill it (turn its cell into a corpse, stop tracking it). `(x, y)` is
/// the worm's *current* position — already moved, if this tick moved it.
fn apply_energy_delta(world: &mut World, x: i32, y: i32, organism: u16, delta: f32) -> Vec<ActiveSite> {
    let Some(state) = world.organism_mut(organism) else {
        return Vec::new(); // freed mid-tick; nothing left to charge
    };
    state.energy += delta;
    let energy = state.energy;
    if energy <= 0.0 {
        die(world, x, y, organism);
        return Vec::new();
    }
    vec![ActiveSite { x, y, kind: ActiveKind::Creature { organism }, next_frame: world.creature_due(WORM_TICK_INTERVAL) }]
}

/// Turn a starved worm into matter and give its slot back.
///
/// The corpse is written **without** an organism id: a dead worm is not a
/// member of anything, it is `corpse` — a `Powder` that falls, rolls, burns
/// and decays like any other, which is the whole reason death here needs no
/// creature-specific code. `World::set`'s seam drops the cell from the
/// organism's list as that write lands, which is what leaves the list empty
/// and the slot safe to release.
fn die(world: &mut World, x: i32, y: i32, organism: u16) {
    if let Some(corpse_id) = world.materials.id_of("corpse") {
        let shades = world.materials.get(corpse_id).palette.len().max(1) as u32;
        let shade = rng::stream(world.seed, organism as u64, world.frame, RNG_SLOT_DEATH).below(shades) as u8;
        let temp = world.get(x, y).temperature();
        world.set(x, y, Cell::new(corpse_id, shade).with_temperature(temp));
    }
    // Unconditional, including the no-corpse-material case: the slot must
    // come back either way, and any worm cell left standing is then inert
    // matter whose next due site drops itself on the stale-handle check.
    world.free_organism(organism);
}

impl World {
    /// Plant a worm at `(x, y)` — M18 debug tool. See `plant_worm_seed` for
    /// what actually happens. A no-op if the position isn't empty or the
    /// `worm` material isn't loaded.
    pub fn plant_worm(&mut self, x: i32, y: i32) {
        if let Some(site) = plant_worm_seed(self, x, y) {
            self.schedule_active_site(site);
        }
    }
}


// --- Ants: chain creatures with a brain -------------------------------
//
// The worm above and the ants below are **two locomotion models, not one
// with parameters**, and that is deliberate — the same call
// `organism.rs` makes for `Divide` versus `Grow`. A worm *burrows*: it
// eats its way through powder, pays a cost scaled by the material's own
// density, and has no notion of standing on anything. An ant *walks*: it
// needs solid footing, it climbs, it digs only when a brain output says
// so, and its cost is per cell moved. Forcing both onto one function
// means a parameter that switches between two unrelated rules, which is
// the shape that gets called "generic" and read as "unreadable".
//
// What they *do* share is the substrate (organism handles, cell lists,
// slot reclamation), the choice function (`choose_weighted`), and the
// whole-`Cell` move (P-1). Those are the parts worth sharing.

/// The compass turn a brain's `Turn` output maps onto, and the three
/// candidate directions a head may step to: ahead-left, ahead, ahead-right.
/// Never all eight — a creature that can reverse in place every tick has no
/// heading worth the name, and the Physarum literature's whole loop is
/// built on a forward cone.
const AHEAD_LEFT: u8 = 1;
const AHEAD_RIGHT: u8 = 7;

/// Rays in the all-round sight fan, swept over the full circle.
///
/// **16, and it is the number that was priced.** The cost of the sense is a
/// function of this and `CreatureDef::sight_range` and **not** of how many
/// prey exist — a pairwise test against every prey would need an index the
/// engine does not have and would scale with the colony, which is the whole
/// reason the fan is the shippable shape
/// (`Reports/creature-vision-sizing-2026-08-30.md` §5).
///
/// A module constant rather than a species field on purpose: it is the
/// resolution of the instrument, not a trait of an animal, and a species
/// that could author it would be authoring its own frame cost.
const SIGHT_RAYS: usize = 16;

/// How far above the head the eye sits, in cells.
///
/// **One, and this is the whole of the "sees over the litter" rule** —
/// there is no material whitelist and there should not be. Measured
/// (`creature-vision-sizing-2026-08-30.md` §4): both animals here are
/// ground-huggers, so a sight line between two heads grazes the floor for
/// its whole length and a two-cell seed pile stops a forty-cell line. At
/// head height **28.1%** of prey pairs were blocked; one cell up, **8.5%**
/// — which on `wetland` recovers the *entire* transparent-world ceiling
/// (median `los` at r64 back to 0.667, identical to occlusion switched
/// off). On `rolling` it recovers about 70% of the gap rather than all of
/// it, because that preset has real relief and some of what stops a line
/// there is landscape.
///
/// **`eye = 3` is not better and the row is not smoothed over**: its
/// pooled blocking is lower (4.8%) and its median `los` is *worse* (0.613).
/// Nothing in that study explains it; 1 is the setting it supports.
///
/// The lift passes only through cells that do not themselves block —
/// raising an eye *into* the terrain would manufacture sight lines out of
/// nothing.
const SIGHT_EYE_LIFT: i32 = 1;

/// Radius of the crowding scan, in cells. Small on purpose: this is a
/// *contact-range* read of the grid, which decision D5 explicitly permits
/// (`Reports/creature-direction.md`) — the hard line is at colony scale,
/// where the field is the mechanism, and it is not crossed by an ant
/// noticing the ants it is standing among.
const CROWDING_RADIUS: i32 = 2;
/// Full-scale value of `BrainOutput::Caution` — how strongly a candidate
/// with a foothold can be preferred over one into thin air, when a genome
/// asks for the maximum. A silent output lands at half this, which is the
/// 0.6 that used to be hardcoded.
///
/// A bonus rather than a veto, at every setting: a creature must still be
/// able to walk off a ledge, and forbidding it outright would be the "gate
/// whether something happens" mistake `CLAUDE.md` warns a size cap must
/// never make.
const FOOTING_MAX: f32 = 1.2;

/// Full-scale value of `BrainOutput::Persist`. A silent output lands at
/// 1.0, which is deliberately **much** higher than the `0.15` this
/// replaced: that literal was arbitrary, and it is the number that decides
/// whether a creature commutes or mills. Handing it to the genome and
/// letting measurement pick is the entire point of the change.
const PERSIST_MAX: f32 = 2.0;

// `TUMBLE_ON_FAILED_MOVE` is gone: it is `BrainOutput::Tumble` now. The
// lesson it recorded still stands and is worth keeping — "how often do I
// step" and "how often do I change my mind" are different questions, and
// conflating them (tumbling on *every* failed move) makes a creature that
// fails half its move rolls change heading half the time: a diffusive
// random walk, which is the worst possible way to *find* anything.
// Measured at 1.0, food discovery collapsed from 33 pickups to 1, because
// the persistent run that covers ground had been destroyed. The authored
// answer used to be 0.35. What changed is that a creature can now be
// selected for its answer instead of being told mine.
//
// (The `0.35` doc comment this paragraph absorbed had outlived its own
// `const` and slid onto `CROWDING_SCALE` below, which then documented a
// tumble chance for a crowding divisor. Deleting a `const` takes its doc
// with it — check what the next declaration inherited.)

/// Divisor turning that count into a 0..1-ish input.
const CROWDING_SCALE: f32 = 8.0;

/// Scale for the temperature input: degrees above ambient at which the
/// input reads 1.0.
const TEMP_INPUT_SCALE: f32 = 40.0;

/// Plant a colony creature of `species_name` at `(x, y)`, laying its chain
/// out to the left of the head. Returns the site to schedule.
pub fn plant_creature_seed(world: &mut World, x: i32, y: i32, species_name: &str) -> Option<ActiveSite> {
    let species_id = world.species.id_of(species_name)?;
    let material_id = world.materials.id_of(species_name)?;
    let def = world.species.get(species_id).creature.as_ref()?.clone();
    place_creature(world, x, y, species_id, material_id, &def, Origin::Founder)
}

/// Where the individual `place_creature` is about to build came from.
///
/// **One placement path for founders and for children, deliberately.**
/// The seam does eight things — check the cells, take a slot, draw shades,
/// write the body, seed the state, count the spawn and book two ledger
/// accounts — and a second copy of it for births is a second copy that can
/// drift. Every difference between a founder and a bud is in this enum and
/// in the three `match`es that read it.
enum Origin {
    /// A creature out of nothing: a scene, the `Y` key, a harness.
    Founder,
    /// A child budded from a live parent, which pays for it.
    Bud {
        parent: u16,
        genome: Vec<f32>,
        traits: [f32; super::organism::CREATURE_TRAITS],
        generation: u16,
        lineage: u32,
    },
}

/// Build one creature at `(x, y)` and return the site to schedule it at.
///
/// The head goes at `(x, y)` and the rest of the chain lays out to its
/// left, exactly as it always has; every cell must be free before anything
/// is allocated or written, or a half-placed body leaks a slot and leaves
/// orphan cells (the reason `plant_worm_seed` checks first too).
fn place_creature(
    world: &mut World,
    x: i32,
    y: i32,
    species_id: SpeciesId,
    material_id: material::MaterialId,
    def: &CreatureDef,
    origin: Origin,
) -> Option<ActiveSite> {
    let positions: Vec<(i32, i32)> = def.body.offsets(false).iter().map(|&(dx, dy)| (x + dx, y + dy)).collect();
    if positions.iter().any(|&(px, py)| !world.is_empty(px, py)) {
        return None;
    }
    // Taken before `positions` is moved into the organism's chain, because
    // the structural grant is per body cell.
    let body_cells = positions.len();

    // At the slot ceiling nothing hatches -- see `plant_worm_seed` above,
    // and `World::push_organism` for why refusal beats a corrupted id.
    let organism = world.push_organism(species_id)?;
    let shades = world.materials.get(material_id).palette.len().max(1) as u32;
    // **The body's own vertical span**, measured off the positions rather
    // than off `BodyPlan`, so it is right for either plan without asking
    // which one this is. A `Chain` collapses this to zero -- it is one cell
    // thick -- and `body_shade` grades along the body instead.
    let ranked = shades_by_luma(world, material_id);
    let (dy_min, dy_max) = positions
        .iter()
        .fold((i32::MAX, i32::MIN), |(lo, hi), &(_, py)| ((lo).min(py - y), (hi).max(py - y)));
    for (i, &(px, py)) in positions.iter().enumerate() {
        let cell_type = if i == 0 { CellType::Head } else { CellType::Segment };
        let shade = match def.shade_rule {
            // Unchanged, and deliberately still drawing even though the
            // value could be computed: this is the shipped path and it must
            // stay byte-identical.
            ShadeRule::Random => rng::stream(world.seed, organism as u64, i as u64, RNG_SLOT_SHADE).below(shades) as u8,
            ShadeRule::Countershade => body_shade(&ranked, i, positions.len(), py - y, dy_min, dy_max),
        };
        world.set(px, py, Cell::new(material_id, shade).with_organism_id(organism).with_aux(pack_cell_type(cell_type)));
    }
    // Claimed before the state is borrowed, because `claim_lineage` takes
    // `&mut World` and a founder needs the number inside the block below.
    let founder_lineage = match origin {
        Origin::Founder => world.claim_lineage(),
        Origin::Bud { lineage, .. } => lineage,
    };
    // Read before the state is borrowed mutably: `self.species` and
    // `self.organisms` cannot both be borrowed, the same reason
    // `push_organism` reads the fate table up front.
    let founder_genome = matches!(origin, Origin::Founder).then(|| world.species.get(species_id).genome.clone());
    // **What this creature is handed, which is not its species' budget.**
    // A founder appears out of nothing and gets the whole of
    // `start_energy`; a bud gets what its *parent's* `TRAIT_BIRTH_GRANT`
    // says to give it, which is the one number §2.6's anti-freeloading
    // rule is actually about (`Reports/creature-reproduction-economics.md`
    // §1.3). The two were the same quantity by accident until this slot
    // existed, and the parent is charged exactly this a few lines down --
    // one expression, read twice, so the endowment and the charge cannot
    // drift apart.
    let endowment = match &origin {
        Origin::Founder => def.start_energy,
        Origin::Bud { traits, .. } => birth_grant(def, traits),
    };
    if let Some(state) = world.organism_mut(organism) {
        state.energy = endowment;
        state.chain = positions;
        state.heading = 0; // east
        state.lineage = founder_lineage;
        match &origin {
            Origin::Founder => {
                state.genome = founder_genome.unwrap_or_default();
                // The ancestral body traits, byte-copied. **The one seam
                // that puts a creature in the world out of nothing, so the
                // one place an ancestral trait can be seeded** --
                // `push_organism` leaves the neutral vector because it does
                // not know whether it is allocating a plant.
                state.traits = def.traits;
                state.inherited = false;
                state.generation = 0;
            }
            Origin::Bud { genome, traits, generation, .. } => {
                // **The child's genome came from its parent**, already
                // mutated by the caller. This is the whole of heredity: one
                // assignment, and the reason S1-S5 were all inert until now.
                state.genome = genome.clone();
                state.traits = *traits;
                state.inherited = true;
                // **Read off the parent by the caller and passed in whole.**
                // The failure this shape exists to avoid is a
                // `let generation = state.generation;` written *inside* a
                // block that has already rebound `state` to the child --
                // every bred child then pins at generation 1 for ever and
                // lineage depth silently flattens, which is a real incident
                // in this repo's history and was caught only because one
                // guard hashed enough state to notice.
                state.generation = *generation;
            }
        }
        // Starts *at* the nest as far as scent goes: an ant that has just
        // hatched has, by construction, just been at home.
        state.since_nest = 0;
        // And is *at* home as far as range goes, for the same reason. See
        // `OrganismState::forage_anchor` — measurement only.
        state.forage_anchor = (x, y);
        state.forage_max = 0;
    }
    let stamp = (def.body_energy * body_cells as f32) as f64;
    match origin {
        Origin::Founder => {
            world.creature_stats.spawned += 1;
            // Metabolic budget plus the meat the body is made of. Both are
            // grants *here*, at the one seam where a creature appears out
            // of nothing, so the structural half is accounted rather than
            // conjured at the far end when the animal dies.
            world.energy_ledger.granted += def.start_energy as f64;
            world.energy_ledger.stamped += stamp;
        }
        Origin::Bud { parent, .. } => {
            world.creature_stats.births += 1;
            // **A birth creates no energy, and this is the S3b stamp seam
            // closing** (`Reports/creature-evolution-plan.md` §2.3, "One
            // seam left open"; `EnergyLedger::meat_lost`'s own doc points
            // here). Both halves come out of the parent's bank:
            //
            // * the child's `start_energy` is live-to-live, so it books to
            //   nothing at all — `granted` would be a source, and nothing
            //   was sourced;
            // * the child's stamp is live-to-**meat**, which is exactly
            //   what `stored_in_meat` is for. Booking it to `stamped`
            //   instead would put the meat in the world twice, because
            //   `max_standing_meat` counts both terms and only
            //   `stored_in_meat` is subtracted from the live side.
            //
            // The parent is charged the whole of it one line down, so the
            // live identity closes by construction rather than by luck.
            world.energy_ledger.stored_in_meat += stamp;
            let cost = birth_cost_of(def, endowment);
            if let Some(state) = world.organism_mut(parent) {
                state.energy -= cost;
                state.seeds_set = state.seeds_set.saturating_add(1);
            }
        }
    }
    Some(ActiveSite { x, y, kind: ActiveKind::Creature { organism }, next_frame: world.creature_due(def.tick_interval) })
}

/// This material's palette entries, ordered **darkest first** by luma.
///
/// Rec. 601 weights in integer arithmetic, and a **stable** sort: two
/// palette entries of equal luma must keep their authored order, because
/// `CLAUDE.md`'s `sort_unstable` gotcha is that tie order is a function of
/// the element type and not of the comparator alone -- and a shade is
/// exactly the kind of quantity where a silent reordering would change
/// every animal in the world and no test would notice.
fn shades_by_luma(world: &World, material_id: material::MaterialId) -> Vec<u8> {
    let palette = &world.materials.get(material_id).palette;
    if palette.is_empty() {
        return vec![0];
    }
    let mut idx: Vec<u8> = (0..palette.len().min(u8::MAX as usize) as u8).collect();
    idx.sort_by_key(|&i| {
        let c = palette[i as usize];
        c[0] as u32 * 299 + c[1] as u32 * 587 + c[2] as u32 * 114
    });
    idx
}

/// The palette entry for one body cell under `ShadeRule::Countershade`.
///
/// The head takes the palest entry outright -- it is the one cell whose job
/// is to say which end is the front. The rest are graded on a fraction `t`
/// that is 1 at the pale end and 0 at the dark end, and **which axis `t`
/// runs along is decided by whether the body has a top and a bottom at
/// all**:
///
/// * a body with vertical extent grades by height, palest on top. That is
///   countershading, and the reason it is worth having is in `ShadeRule`.
/// * a `Chain` has none -- it is one cell thick -- so `dy_max == dy_min`
///   and the grade runs head-to-tail instead, which is what stops a long
///   chain reading as one flat smear.
///
/// Note `y` grows downward, so the *smallest* `dy` is the top of the animal.
fn body_shade(ranked: &[u8], i: usize, cells: usize, dy: i32, dy_min: i32, dy_max: i32) -> u8 {
    let last = ranked.len().saturating_sub(1);
    if i == 0 {
        return ranked[last];
    }
    let t = if dy_max > dy_min {
        (dy_max - dy) as f32 / (dy_max - dy_min) as f32
    } else {
        1.0 - (i as f32 / cells.saturating_sub(1).max(1) as f32)
    };
    ranked[(t * last as f32).round() as usize]
}

/// **How many body cells this animal is currently paying for.**
///
/// The **live** chain rather than `def.body.len()`, and the difference is
/// not cosmetic: a creature that has lost cells — bitten off by a predator,
/// burned, or buried — is a smaller animal and burns less. Reading the plan
/// instead would keep charging a two-cell bill to an animal that is down to
/// one, which is the same class of error as pricing a corpse at what it was
/// worth alive.
///
/// Floored at 1. A zero here would make an animal mid-`reconcile_chain`
/// briefly free, and free is the one value this must never take —
/// `idle_cost_per_cell`'s doc has why.
fn live_body_cells(world: &World, organism: u16, def: &CreatureDef) -> f32 {
    world.organism(organism).map_or(def.body.len(), |s| s.chain.len()).max(1) as f32
}

/// **What one birth costs the parent**: the child's metabolic grant plus
/// the structural stamp of every cell of its body.
///
/// Both halves are real. The grant is the pool the child spends; the stamp
/// is the meat its body is made of, which nothing can spend and which
/// becomes food the day it dies. Charging only the grant would let a
/// lineage manufacture matter — a parent at 1,000 energy could turn itself
/// into an unbounded pile of corpses — and *"evolution is a fuzzer for
/// your conservation laws"* (`creature-direction.md` §8) is not a figure
/// of speech: an unpriced birth is the largest free term any of these
/// animals could reach.
pub fn birth_cost(def: &CreatureDef) -> f32 {
    birth_cost_of(def, birth_grant(def, &def.traits))
}

/// `birth_cost` for a **particular** endowment rather than the species'
/// ancestral one, because `TRAIT_BIRTH_GRANT` is heritable and two parents
/// of one species do not pay the same price for a child.
///
/// **The stamp term does not move when the grant does, and that is the
/// whole shape of this economy.** Whatever a lineage cuts its endowment
/// to, a birth still needs `body_energy` per body cell of *harvested*
/// energy on top of it -- 960 for the shipped two-cell ant. Measured
/// 2026-08-30: against a bank ceiling of `hunger_fraction * start_energy +
/// one mouthful` = 567, that stamp alone is unreachable, so the shipped
/// ant does not breed at any grant and cutting `start_energy` lowers the
/// ceiling faster than it lowers the cost. See
/// `Reports/creature-birth-grant-2026-08-30.md`.
pub fn birth_cost_of(def: &CreatureDef, grant: f32) -> f32 {
    grant + def.body_energy * def.body.len() as f32
}

/// **What a newborn of this species is handed**, given the provisioning
/// allele of the animal paying for it.
pub fn birth_grant(def: &CreatureDef, traits: &[f32; CREATURE_TRAITS]) -> f32 {
    grant_fraction(traits[TRAIT_BIRTH_GRANT]) * def.start_energy
}

/// `TRAIT_BIRTH_GRANT`'s position on the shared `-1..=1` trait axis, as a
/// fraction of `start_energy`: `-1` hands a newborn nothing, `+1` hands it
/// a full species budget, `0` half.
///
/// The clamp is the axis rather than a tuning choice -- the same statement
/// `try_bud`'s mutation step makes for every slot.
pub fn grant_fraction(t: f32) -> f32 {
    ((t + 1.0) * 0.5).clamp(0.0, 1.0)
}

/// **The bank an individual actually has to reach to bud**, or `None` if
/// this species does not reproduce.
///
/// `reproduce_threshold` as authored, floored at just above
/// `birth_cost` — see that field's own doc. The floor is not a tuning
/// decision, it is what makes the mechanism total: a threshold under the
/// cost is a species whose every birth kills the parent, which reads in
/// every counter as reproduction working.
pub fn reproduce_at(def: &CreatureDef) -> Option<f32> {
    (def.reproduce_threshold > 0.0).then(|| def.reproduce_threshold.max(birth_cost(def) + 1.0))
}

/// Bud a child off `organism` if it can afford one and there is room.
///
/// Returns the child's site, to be handed back to the scheduler by the
/// tick that called this. **It must not be scheduled directly from here**,
/// and the reason is dead end 1094 rather than style: a dispatch loop that
/// takes the whole heap out and writes it back discards anything scheduled
/// from inside a dispatched tick, and a decay-reseeded seed once got
/// planted and never grew a single cell, for ever. `World::
/// pop_due_active_site` was rebuilt to keep the live heap writable, so
/// scheduling from here would in fact work today — returning the site
/// keeps the birth path independent of that, and is the same shape
/// `apply_creature_energy` already uses for the parent's own next tick.
fn try_bud(world: &mut World, organism: u16, def: &CreatureDef) -> Option<ActiveSite> {
    let threshold = reproduce_at(def)?;
    let state = world.organism(organism)?;
    let parent_traits = state.traits;
    // **The bar this individual has to clear, not the one its species
    // was authored with.** `reproduce_at` is the species' authored
    // threshold floored above the *ancestral* birth cost; an animal whose
    // `TRAIT_BIRTH_GRANT` has drifted upward owes more than that, and
    // charging it a cost it never had to reach would put the parent's bank
    // negative -- a birth that kills its parent, which is precisely what
    // `reproduce_at`'s floor exists to make impossible. Taking the max
    // keeps that guarantee total under a heritable grant, and the `+ 1`
    // matches the floor's own strictness so the two cannot disagree by a
    // rounding step.
    let cost = birth_cost_of(def, birth_grant(def, &parent_traits));
    if state.energy < threshold.max(cost + 1.0) {
        return None;
    }
    let (hx, hy) = *state.chain.first()?;
    let species_id = state.species;
    let parent_genome = state.genome.clone();
    // **Read off the parent here, while `state` is still the parent.** See
    // the `Origin::Bud` arm in `place_creature` for the incident this
    // naming exists to prevent.
    let parent_generation = state.generation;
    let parent_lineage = state.lineage;
    let material_id = world.materials.id_of(&world.species.get(species_id).name.clone())?;

    // Where the child goes: the first of the eight neighbours of the
    // parent's head at which the whole body fits. `DIRS` order, which is
    // the engine's canonical one — no sort, so no unstable-tie-order
    // question arises, and two identical worlds place identical children.
    //
    // **This is a gate on whether something happens, and it is counted for
    // exactly that reason.** With one authored body plan it is a property
    // of the terrain; the day body size is heritable it becomes a silent
    // selection pressure for smallness (§2.8's second pre-check), and
    // `births_denied_no_space` is what will say how hard it was biting
    // when that decision has to be made.
    let mut site = None;
    for (dx, dy) in DIRS {
        // The child's own genome and traits are drawn *once*, outside this
        // loop's effect: nothing here consumes from any stream, so which
        // neighbour succeeds cannot change what the child inherits.
        if let Some(s) = place_creature(
            world,
            hx + dx,
            hy + dy,
            species_id,
            material_id,
            def,
            Origin::Bud {
                parent: organism,
                genome: parent_genome.clone(),
                traits: parent_traits,
                generation: parent_generation.saturating_add(1),
                lineage: parent_lineage,
            },
        ) {
            site = Some(s);
            break;
        }
    }
    let Some(site) = site else {
        world.creature_stats.births_denied_no_space += 1;
        return None;
    };
    // **Mutate after placement, on the child's own handle.** The stream is
    // keyed on the handle the allocator just issued plus the frame, so it
    // cannot be predicted from the parent and cannot repeat when a slot is
    // reused — see `RNG_SLOT_BIRTH`.
    let ActiveKind::Creature { organism: child } = site.kind else { return Some(site) };
    let mut draw = rng::stream(world.seed, child as u64, world.frame, RNG_SLOT_BIRTH);
    if let Some(state) = world.organism_mut(child) {
        let mut genome = std::mem::take(&mut state.genome);
        brain::mutate(&mut genome, def.mutation_rate, &mut draw);
        state.genome = genome;
        for (t, &width) in state.traits.iter_mut().zip(def.trait_variance.iter()) {
            if width > 0.0 {
                // `gut_bias` is a position on a `-1..=1` axis and every
                // other slot in `CREATURE_TRAITS` is defined the same way,
                // so the clamp is the axis rather than a tuning choice.
                *t = (*t + (draw.unit_f32() * 2.0 - 1.0) * width).clamp(-1.0, 1.0);
            }
        }
    }
    Some(site)
}

/// How wide a founded colony's nest patch is, in cells, and how far apart
/// its ants stand.
///
/// **Ants need more corridor than they occupy.** Founded shoulder to
/// shoulder they gridlock: a creature is not a foothold and cannot be
/// walked through, so a dense line of them simply stops moving — measured
/// at 27,386 blocked ticks against a single pickup, with the picture
/// showing an unbroken wall of ants. Four cells apart for a two-cell body
/// leaves each one somewhere to go.
const COLONY_HALF_WIDTH: i32 = 26;
const COLONY_ANT_SPACING: i32 = 4;
/// Grasse's threshold, in practice: below about fifty, a colony looks
/// broken even when the code is right (P-15). A key that placed one ant
/// would mostly teach people that ants do not work.
const COLONY_ANTS: i32 = 52;

impl World {
    /// Place an ant at `(x, y)` — the scene-level entry point, mirroring
    /// `plant_worm`.
    pub fn plant_ant(&mut self, x: i32, y: i32) {
        if let Some(site) = plant_creature_seed(self, x, y, "ant") {
            self.schedule_active_site(site);
        }
    }

    /// **Found a colony** on whatever ground is under `(x, y)` — the `Y`
    /// key, and the only ant entry point worth having in the app.
    ///
    /// Places a nest patch by converting the surface it finds, then stands
    /// fifty-odd ants on it. Terrain-following rather than a fixed
    /// rectangle, so it works on a hillside, in a cave mouth, or on the
    /// flat preset without the caller having to know which.
    ///
    /// The nest is *surface* material, deliberately: built as a block
    /// sitting on the ground, arriving ants were simply blocked by its
    /// face and never got home.
    /// Returns how many ants actually got placed, so the caller can say so
    /// on screen. **A silent no-op is indistinguishable from a broken
    /// feature** -- the owner pressed this key, saw nothing happen, and
    /// reasonably concluded the whole milestone was missing.
    pub fn found_colony(&mut self, x: i32, y: i32) -> usize {
        let Some(nest) = self.materials.id_of("nest") else {
            return 0;
        };
        // The per-column rules live in `colony_surface` and
        // `colony_ant_site`, so that nothing can hold a second copy of
        // them -- a second copy is what `open-bugs-handoff.md` §R2 is.
        // **Centred on the cursor, both of them.** The ants used to run from
        // 26 cells left of the cursor to 178 cells *right* of it, because
        // the loop started at the nest's left edge and then stepped forward
        // once per ant -- so the colony appeared almost entirely to one
        // side, and pressing the key on the right of the map put most of it
        // outside the world, where placement fails silently.
        //
        // The nest patch stays much narrower than the ant band, which is
        // deliberate and is the ratio the foraging scene measured 414
        // deliveries at: home has to be a *place*, not everywhere, or there
        // is no gradient to walk up.
        // **Both are lengths in cells and both scale.** The comment above
        // says why the spacing is 4 -- "four cells apart *for a two-cell
        // body*" -- so it is denominated in bodies, and a body that is twice
        // as many cells across needs twice the corridor or the colony
        // gridlocks exactly as the 27,386 blocked ticks did. The ant *count*
        // is not a length and stays put; the band widens under it.
        let spacing = scaled_cells(self, COLONY_ANT_SPACING);
        let half_width = scaled_cells(self, COLONY_HALF_WIDTH);
        let span = (COLONY_ANTS - 1) * spacing;
        let left = x - span / 2;
        for cx in (x - half_width)..=(x + half_width) {
            if let Some(sy) = colony_surface(self, cx, y) {
                let cell = self.get(cx, sy);
                // Only ground gets converted -- painting over water or a
                // creature would be a surprise.
                if matches!(self.materials.kind(cell.material), MaterialKind::Solid | MaterialKind::Powder) {
                    self.set(cx, sy, Cell::new(nest, 0).with_attached(cell.attached()));
                }
            }
        }
        let mut placed = 0;
        for i in 0..COLONY_ANTS {
            let cx = left + i * spacing;
            if let Some(sy) = colony_ant_site(self, cx, y) {
                let before = self.get(cx, sy - 1).organism_id();
                self.plant_ant(cx, sy - 1);
                if self.get(cx, sy - 1).organism_id() != before {
                    placed += 1;
                }
            }
        }
        placed
    }
}

/// Where the ground is in one column, for a colony founded at cursor row
/// `cursor_y`.
///
/// **This and `colony_ant_site` exist so that the rule has exactly one
/// definition.** `open-bugs-handoff.md` §R2 is what a second copy costs:
/// `filmstrip`'s colony scene scored candidate ground with its own
/// predicate while `found_colony` placed with a different one, and the two
/// disagreed in both directions -- the scene believed it had chosen dry
/// land while placement dropped ants in a lake, and later (once the scene
/// was fixed first) reported *fewer* viable sites than were actually
/// filled. Anything that wants to know where a colony can go calls these.
///
/// Two rules, both learned from measurement:
///
/// - **Living tissue is not a floor.** `Plant` is passed through with
///   `Empty`/`Gas`, because the colony scene grows trees for 2,400 frames
///   before founding anything, so the first non-empty cell from above is a
///   leaf. Censused over that scene's 308 candidate columns: seed 1 read
///   **Plant 217 / Liquid 91 / ground 0**, which is why it panicked at its
///   own default seed rather than degrading.
/// - **Rise out of the ground before searching down it.** One cursor row
///   cannot describe 204 cells of terrain, and searching downward from it
///   in every column meant every column standing *higher* than the cursor
///   began inside the hill and found its interior. That cost 18, 33 and 20
///   of 52 sites on seeds 1, 2 and 7 -- the largest single term. Rising
///   only as far as the air directly above this column is what keeps the
///   `Y` key's documented behaviour intact: a cursor in open air scans
///   down exactly as before, so founding a colony inside a cave still
///   lands it on the cave floor rather than on the mountain overhead.
///
/// Returns wherever the downward scan stopped, which **may be `Liquid`** --
/// the caller decides what to do about water. `colony_ant_site` refuses it;
/// the nest-painting loop refuses it separately, and always did.
pub fn colony_surface(world: &World, cx: i32, cursor_y: i32) -> Option<i32> {
    // Bounded by `bounds()` rather than unbounded because `get` returns
    // `Cell::OUT_OF_BOUNDS` past the edge, which is not `Empty` and would
    // otherwise read as a floor one row below the world. The `+ 96`
    // fallback is only reached in a world with no bounds at all.
    //
    // **To the bottom of the world, not 96 rows.** The old bound was
    // shorter than the sky on several presets (the flat preset alone has
    // 200 rows of it), so pressing the key with the cursor high up found no
    // ground in any column and placed nothing at all, silently.
    let bottom = world.bounds().map_or(cursor_y + 96, |b| b.max_y);
    let passable = |cy: i32| matches!(world.materials.kind(world.get(cx, cy).material), MaterialKind::Empty | MaterialKind::Gas | MaterialKind::Plant);
    let mut start = cursor_y.max(0);
    while start > 0 && !passable(start) {
        start -= 1;
    }
    (start..=bottom).find(|&cy| !passable(cy))
}

/// The row an ant would stand on in this column, or `None` if the column is
/// not a site.
///
/// A site needs ground that is `Solid` or `Powder` -- **not `Liquid`**,
/// which is the placement half of `open-bugs-handoff.md` §R2: the
/// nest-painting loop had refused water since it was written, and said why
/// ("painting over water or a creature would be a surprise"), while the ant
/// loop six lines below it never got the same guard. An ant put on water
/// then stayed there for ever, because `step_chain` correctly judges it
/// unsupported but the fall it attempts needs every cell to land somewhere
/// `World::is_empty` calls free, and water is not free -- so the fall is
/// refused and nothing ever moves it.
///
/// **This fixes placement only.** What an ant should do when it walks onto
/// water under its own power -- drown, float, or swim -- is a design
/// question for the owner, and an ant that wanders onto a pond still stands
/// on it.
///
/// The cell above the ground must also be free. That is not a formality: in
/// a wood it is the term that decides most refusals, because a column
/// holding a trunk finds ground *under* the trunk and an ant cannot stand
/// where the trunk is. A colony founded in a forest is therefore genuinely
/// sparser than one founded on a beach, and that is the world being
/// reported rather than a bug.
pub fn colony_ant_site(world: &World, cx: i32, cursor_y: i32) -> Option<i32> {
    let sy = colony_surface(world, cx, cursor_y)?;
    if !matches!(world.materials.kind(world.get(cx, sy).material), MaterialKind::Solid | MaterialKind::Powder) {
        return None;
    }
    world.is_empty(cx, sy - 1).then_some(sy)
}

/// One decision by one chain creature.
///
/// Order matters and is: resolve identity, sense, think, act on the world,
/// then move and deposit. Deposits come **after** the move and only if it
/// succeeded (P-11) — a blocked agent that still reinforces is how
/// congested dead ends accumulate trail and the colony ossifies pointing
/// into a wall.
fn creature_tick(world: &mut World, x: i32, y: i32, organism: u16, def: &CreatureDef) -> Vec<ActiveSite> {
    let Some(material_id) = world.materials.id_of(&world.species.get(world.organism(organism).expect("live").species).name.clone()) else {
        return Vec::new();
    };
    let cell = world.get(x, y);
    if cell.material != material_id || cell.organism_id() != organism {
        // **Reconcile, do not merely release.** The active site sits on the
        // *head*, so losing the head lands here — and `release_if_bodyless`
        // would find the trailing segments still present, decline to free
        // anything, and drop the site. The creature would then never tick
        // again: not dead, not scheduled, just an orphan standing in the
        // world forever. `reconcile_chain` is what knows that a missing
        // head is death rather than an injury.
        reconcile_chain(world, organism);
        return Vec::new();
    }
    if cell.is_burning() {
        // Same deferral the worm makes, for the same reason: let fire.rs
        // finish deciding this creature's fate first.
        return vec![ActiveSite { x, y, kind: ActiveKind::Creature { organism }, next_frame: world.creature_due(def.tick_interval) }];
    }

    // Something may have eaten, burned or erased part of this creature
    // since its last tick. Reconcile before deciding anything, or it makes
    // decisions from a chain describing cells that are no longer there.
    if !reconcile_chain(world, organism) {
        return Vec::new();
    }

    // --- airborne: integrate, do not decide -----------------------------
    // **Before `sense`, and that ordering is the cost.** A creature in the
    // air does not read the world, does not evaluate its brain, and does not
    // act; it is committed to the arc it launched on until it lands. See
    // `step_flight`, and `brain::BrainOutput::Impulse` for why it is a
    // separate path rather than a change to the walk.
    if world.organism(organism).is_some_and(|s| s.flight.is_some()) {
        return step_flight(world, organism, def);
    }

    let heading = world.organism(organism).map_or(0, |s| s.heading);
    let (inputs, sighting, sight_reads) = sense(world, x, y, organism, heading, def);
    if def.sight_range > 0 {
        world.creature_stats.sight_casts += 1;
        world.creature_stats.sight_cells_read += sight_reads;
        if let Some(seen) = sighting {
            world.creature_stats.sightings += 1;
            world.creature_stats.sight_dist_sum += seen.dist as u64;
            // |bearing| <= 0.25 is +-45 degrees: the sighted prey lies
            // within the three candidate steps this creature can take.
            if inputs[brain::BrainInput::PreyBearing as usize].abs() <= 0.25 {
                world.creature_stats.sight_facing += 1;
            }
        }
    }
    let (outputs, active_synapses) = {
        let Some(state) = world.organism_mut(organism) else {
            return Vec::new();
        };
        let genome = std::mem::take(&mut state.genome);
        let mut brain_state = state.brain_state;
        let result = brain::eval_brain(&genome, &inputs, &mut brain_state);
        let state = world.organism_mut(organism).expect("still live");
        state.genome = genome;
        state.brain_state = brain_state;
        result
    };

    let mut draw = rng::stream(world.seed, organism as u64, world.frame, RNG_SLOT_MOVE);
    // **The tax is a fraction of the budget, not an absolute.** Written
    // out here rather than folded into `CreatureDef` so the multiplication
    // order is visible: `fraction * start_energy` reproduces the old
    // absolute exactly at the species' authored budget, and *keeps* the
    // ratio when a harness cuts the budget. §13j is what this is for --
    // scaling `start_energy` 900 -> 90 and leaving the tax alone spent 72
    // of a 90-point life on thinking, 80% of a life, and silently
    // dominated a three-knob sweep that had to be thrown away.
    let synapse_tax = def.synapse_fraction * def.start_energy * active_synapses as f32;
    // **Metabolism is per body cell** (`idle_cost_per_cell`). Read once
    // here rather than at each of the three charge sites below: the chain
    // cannot change length between them, and a creature that steps into a
    // predator pays for the body it started the tick with.
    let body_cells = live_body_cells(world, organism, def);
    let idle = def.idle_cost_per_cell * body_cells;
    let mut spent = idle + synapse_tax;
    world.energy_ledger.metabolized += idle as f64;
    world.energy_ledger.synapse_tax += synapse_tax as f64;

    // --- the four verbs, before moving: an ant that is going to pick
    // --- something up should do it from where it can reach it.
    act(world, x, y, organism, def, &outputs, &mut draw);

    // --- move -----------------------------------------------------------
    // **Run, or tumble.** `Move` is the run probability, and the brain
    // drives it from the along-heading gradient: a laden ant walking away
    // from the nest scent computes a low `Move`, fails the roll, and
    // re-orients. That is the whole of the homing mechanism — there is no
    // steering toward the nest anywhere, because on a surface there is
    // nothing to steer on (`brain::BrainInput::PheroAAlong`).
    let p_move = outputs[brain::BrainOutput::Move as usize].clamp(0.0, 1.0);
    let mut moved = false;
    if draw.unit_f32() < p_move {
        // **Hop, or walk.** `Impulse` is read raw and gated on strictly
        // positive, which is the whole of the "byte-identical for species
        // that do not use the verb" guard (`creature-motion-design.md` §7):
        // `squash(0.0)` is exactly 0.0 for a row nothing has authored, so
        // `&&` short-circuits and **no RNG draw is taken**. Take one
        // unconditionally here and every ant in the world gets a different
        // stream from the next line on, whether or not it can jump.
        let impulse = outputs[brain::BrainOutput::Impulse as usize].clamp(0.0, 1.0);
        if impulse > 0.0 && draw.unit_f32() < impulse && launch(world, organism, heading) {
            // Charged where it is decided, not inside `launch`, so the
            // energy ledger has one owner. The flight frames themselves
            // charge only pro-rated metabolism -- ballistics is free once
            // you have paid to leave.
            let cost = def.move_cost_per_cell * body_cells * LAUNCH_COST_IN_MOVES;
            spent += cost;
            world.energy_ledger.moved += cost as f64;
            // **Deliberately not `moved`.** `moved` gates the pheromone
            // deposit (P-11) and a creature in the air is not touching the
            // ground it would be laying a trail on. It also keeps
            // `CreatureStats::moves` meaning exactly one walking step, which
            // is what §7's falls-per-move bar was baselined against.
        } else {
            moved = step_chain(world, organism, heading, &outputs, def, &mut draw);
            if moved {
                let step = def.move_cost_per_cell * body_cells;
                spent += step;
                world.energy_ledger.moved += step as f64;
            }
        }
    } else if draw.unit_f32() < brain::unit_scale(outputs[brain::BrainOutput::Tumble as usize], 1.0) {
        tumble(world, organism, def, &mut draw);
    }

    // --- deposit, only on a successful move (P-11) ----------------------
    if moved {
        let (hx, hy) = world.organism(organism).and_then(|s| s.chain.first().copied()).unwrap_or((x, y));
        // Nest scent falls off with how long ago this creature was home.
        // That falloff is the entire homing mechanism: fresher scent is
        // nearer the nest, so a laden ant walking up-gradient is walking
        // home, and nothing ever asks where the nest is.
        let since = world.organism(organism).map_or(0, |s| s.since_nest);
        let recency = (1.0 - since as f32 / def.nest_memory.max(1) as f32).max(0.0);
        let emit_a = outputs[brain::BrainOutput::EmitA as usize].clamp(0.0, 1.0) * recency;
        let emit_b = outputs[brain::BrainOutput::EmitB as usize].clamp(0.0, 1.0);
        world.deposit_pheromone(Channel::A, hx, hy, (emit_a * pheromone::DEPOSIT as f32) as u8);
        world.deposit_pheromone(Channel::B, hx, hy, (emit_b * pheromone::DEPOSIT as f32) as u8);
    }

    if let Some(state) = world.organism_mut(organism) {
        state.since_nest = state.since_nest.saturating_add(1);
    }

    let (hx, hy) = world.organism(organism).and_then(|s| s.chain.first().copied()).unwrap_or((x, y));

    // **The far side of the sighting counter.** `sightings` says the eye
    // fired; on its own that is exactly the shape of counter `CLAUDE.md`
    // warns about — a mining harness once reported 200 cuts having removed
    // 0 cells. This says the sighting *changed where the animal went*:
    // the head ended the tick closer to the prey it saw than it started.
    //
    // Read from the head's actual position after the move rather than from
    // the chosen direction, so a step the world refused does not count.
    // Against the target's position at decision time, not a re-cast, so
    // this cannot silently become a second sighting counter.
    if let Some(seen) = sighting {
        let before = (seen.x - x) * (seen.x - x) + (seen.y - y) * (seen.y - y);
        let after = (seen.x - hx) * (seen.x - hx) + (seen.y - hy) * (seen.y - hy);
        if after < before {
            world.creature_stats.sight_approaches += 1;
        }
    }

    let mut sites = apply_creature_energy(world, hx, hy, organism, -spent, def);

    // --- bud, last, and only if the tick was survived -------------------
    // **After the charge, not before.** An animal that spent its way to
    // zero this tick is dead, and `apply_creature_energy` says so by
    // returning no site at all; budding ahead of the charge would let a
    // starving parent pay a birth cost it did not have and then die,
    // which turns the whole mechanism into a way of converting a doomed
    // animal into a fresh one for free.
    if !sites.is_empty() {
        if let Some(child) = try_bud(world, organism, def) {
            sites.push(child);
        }
    }
    sites
}

/// Read a creature's sensory inputs and its brain's response **without
/// changing anything** — for `examples/creature_probe.rs`.
///
/// A non-mutating evaluation, so probing cannot perturb the run it is
/// measuring: the hidden state is copied rather than written back, which
/// costs one tick of recurrence accuracy in the readout and buys the
/// guarantee that looking is free. (`CLAUDE.md`: a debug readout must not
/// be a function of the thing it debugs — here, of itself.)
pub fn probe(world: &World, x: i32, y: i32, organism: u16, def: &CreatureDef) -> ([f32; brain::BRAIN_INPUTS], [f32; brain::BRAIN_OUTPUTS], u32) {
    let Some(state) = world.organism(organism) else {
        return ([0.0; brain::BRAIN_INPUTS], [0.0; brain::BRAIN_OUTPUTS], 0);
    };
    let (inputs, _, _) = sense(world, x, y, organism, state.heading, def);
    let mut brain_state = state.brain_state;
    let (outputs, active) = brain::eval_brain(&state.genome, &inputs, &mut brain_state);
    (inputs, outputs, active)
}

/// Fills all `brain::BRAIN_INPUTS` inputs. Slot indices are
/// `brain::BrainInput`'s and are a permanent public contract — see that
/// enum.
///
/// The count is deliberately **not** written out here. This line said "the
/// 14 brain inputs" for two appends past 14 (it is 16), because a literal
/// in prose has nothing that fails when the enum grows — and the one law
/// this file has is that inputs may be appended freely. Name the const.
fn sense(
    world: &World,
    x: i32,
    y: i32,
    organism: u16,
    heading: u8,
    def: &CreatureDef,
) -> ([f32; brain::BRAIN_INPUTS], Option<Sighting>, u64) {
    use brain::BrainInput as I;
    let mut inputs = [0.0f32; brain::BRAIN_INPUTS];
    inputs[I::Bias as usize] = 1.0;

    let so = def.sensor_offset;
    let at = |dir: u8| {
        let (dx, dy) = DIRS[dir as usize % 8];
        (x + dx * so, y + dy * so)
    };
    let (fx, fy) = at(heading);
    let (lx, ly) = at((heading + AHEAD_LEFT) % 8);
    let (rx, ry) = at((heading + AHEAD_RIGHT) % 8);

    // Front concentration plus a *lateral difference*, per channel. The
    // pairing is what makes trail-following reachable by one connection
    // from a lateral input to the turn output: concentration says "there is
    // something", the difference says "that way".
    for (channel, front_slot, lateral_slot) in
        [(Channel::A, I::PheroAFront, I::PheroALateral), (Channel::B, I::PheroBFront, I::PheroBLateral)]
    {
        let f = world.pheromone_at(channel, fx, fy) as f32 / 255.0;
        let l = world.pheromone_at(channel, lx, ly) as f32 / 255.0;
        let r = world.pheromone_at(channel, rx, ry) as f32 / 255.0;
        inputs[front_slot as usize] = f;
        inputs[lateral_slot as usize] = r - l;
    }

    // **The along-heading gradient**, which is the one a surface-dweller
    // can actually read. Normalized by the sum rather than scaled by a
    // constant, so it is scale-free: a faint trail and a saturated one both
    // produce a usable -1..1, which matters because a trail's absolute
    // height varies by two orders of magnitude over its life. The `+ 1.0`
    // makes an empty pair read exactly 0 instead of dividing by zero.
    for (channel, slot) in [(Channel::A, I::PheroAAlong), (Channel::B, I::PheroBAlong)] {
        let here = world.pheromone_at(channel, x, y) as f32;
        let ahead = world.pheromone_at(channel, fx, fy) as f32;
        inputs[slot as usize] = (ahead - here) / (ahead + here + 1.0);
    }

    let moisture_at = |px: i32, py: i32| world.field_at_bilinear(px as f32, py as f32).moisture / WORM_MOISTURE_SATURATION;
    inputs[I::MoistureFront as usize] = moisture_at(fx, fy);
    inputs[I::MoistureLateral as usize] = moisture_at(rx, ry) - moisture_at(lx, ly);

    let here = world.field_at_bilinear(x as f32, y as f32);
    // Divided through by the day/night oscillator, per CLAUDE.md: a
    // threshold sampled at an arbitrary phase of a designed oscillator is a
    // different threshold every hour, and the light channel swings 20:1.
    inputs[I::LightHere as usize] = (field::noon_equivalent_light(here.light, world.sky_frame()) / field::MAX_LIGHT).clamp(0.0, 1.0);
    // Divided through by the day/night oscillator for the same reason
    // `LightHere` is, one channel over — subtractively, because temperature
    // is an interval scale and the sky's contribution is signed. A brain
    // input that drifts with the hour is a brain input every evolved
    // behaviour is silently conditioned on the time of day.
    inputs[I::TempAboveAmb as usize] =
        ((field::noon_equivalent_temperature(here) - AMBIENT_TEMPERATURE as f32) / TEMP_INPUT_SCALE).clamp(-1.0, 1.0);

    // **The same predicate the eat verb rolls against, deliberately.** If
    // the eye used a wider test than the mouth, an animal would steer at
    // food it cannot digest and the gene would be nutritional bookkeeping
    // rather than a behaviour -- which is the whole difference S5 exists to
    // make. A meat gut stops *seeing* leaves.
    inputs[I::FoodAdjacent as usize] = if adjacent_food(world, x, y, gut_of(world, organism, def)).is_some() { 1.0 } else { 0.0 };
    inputs[I::AtNest as usize] = if adjacent_nest(world, x, y, def) { 1.0 } else { 0.0 };

    if let Some(state) = world.organism(organism) {
        inputs[I::Energy as usize] = (state.energy / def.start_energy.max(1.0)).clamp(0.0, 1.0);
        inputs[I::Carrying as usize] = if state.carrying.is_some() { 1.0 } else { 0.0 };
    }

    // **A creature is not crowded by itself**, and it was: this scan
    // counted any `Creature`-kind cell in the 5x5 with no owner check, so a
    // `Chain(2)` ant read a floor of 1/8 = 0.125 forever and the 2x2 beetle
    // 0.375. Neither is a neighbour; both are the animal's own tail.
    //
    // Harmless-looking, and it is not: `ant.ron` authors
    // `(Crowding, Move, -0.3)`, so that floor was a constant subtraction
    // from the run probability, absorbed into `(Bias, Move, 2.0)` when that
    // was tuned. The reason to fix it *now*, in a milestone with no genes
    // in it, is that body length becomes heritable in S8 -- at which point
    // the offset becomes a function of the gene and every anatomy result
    // would be measuring a hidden behavioural change. `CLAUDE.md`'s "fixing
    // a bug exposes a constant that was compensating for it", caught before
    // the bug rather than after.
    //
    // **The radius is a length and the divisor below is not free to stay
    // put when it moves.** At `cell_scale` 2 the same physical
    // neighbourhood is 9x9 rather than 5x5, and the fraction of cells in it
    // that are flesh is unchanged -- so a fixed `CROWDING_SCALE` would read
    // 3.3x the crowding for the same physical crush and pin
    // `(Crowding, Move, -0.3)` at its floor. `CLAUDE.md`'s "fixing a bug
    // exposes a constant that was compensating for it" in its second shape:
    // changing what a term can express reallocates the whole weighted sum.
    let crowd_radius = scaled_cells(world, CROWDING_RADIUS);
    let neighbourhood = |r: i32| ((2 * r + 1) * (2 * r + 1) - 1) as f32;
    let crowd_scale = CROWDING_SCALE * neighbourhood(crowd_radius) / neighbourhood(CROWDING_RADIUS);
    let mut crowd = 0;
    for dy in -crowd_radius..=crowd_radius {
        for dx in -crowd_radius..=crowd_radius {
            if dx == 0 && dy == 0 {
                continue;
            }
            let cell = world.get(x + dx, y + dy);
            if cell.organism_id() == organism {
                continue;
            }
            if world.materials.kind(cell.material) == MaterialKind::Creature {
                crowd += 1;
            }
        }
    }
    inputs[I::Crowding as usize] = (crowd as f32 / crowd_scale).min(1.0);

    // **The distal sense, and the reason E15 exists.** Everything above
    // this line is contact range or a field read; nothing in it reports
    // another animal at a distance, which is why a predator that could
    // already kill moved no counter at all.
    //
    // **Gated on the species field, at a site that already holds the
    // `CreatureDef`** (`CLAUDE.md`: guard hot-path work at the call site
    // that has the data). An eyeless species pays one `i32` compare per
    // tick and never enters `sight`; the ant, the worm and every plant-side
    // caller are therefore bit-identical to before this input existed.
    let mut sight_reads = 0u64;
    let sighting =
        (def.sight_range > 0).then(|| sight(world, x, y, organism, def, gut_of(world, organism, def), &mut sight_reads)).flatten();
    if let Some(seen) = sighting {
        // **Nearness rather than distance**, so that "nothing in sight" and
        // "at the very edge of sight" are both ~0 and the input rises as
        // the thing that matters gets closer. A raw distance would read
        // *largest* when the prey is furthest, and an authored weight would
        // have to be negative to mean "approach", which is the kind of sign
        // inversion an evolved genome has no reason to find.
        inputs[I::PreyNear as usize] = (1.0 - seen.dist / def.sight_range as f32).clamp(0.0, 1.0);
        // Signed turn-to-target, **positive = to the right**, matching
        // `PheroALateral`. `DIRS` runs anticlockwise on a y-down screen, so
        // a heading index `h` points along `-h * PI/4` in screen radians;
        // the wrapped difference from that to the bearing is then positive
        // clockwise, which is the right hand. +-1 is directly behind.
        let bearing = ((seen.y - y) as f32).atan2((seen.x - x) as f32);
        let heading_angle = -(heading as f32) * std::f32::consts::FRAC_PI_4;
        let mut error = bearing - heading_angle;
        error = error.rem_euclid(std::f32::consts::TAU);
        if error > std::f32::consts::PI {
            error -= std::f32::consts::TAU;
        }
        inputs[I::PreyBearing as usize] = error / std::f32::consts::PI;
    }

    (inputs, sighting, sight_reads)
}

/// **What one cell is worth to *this gut*** — S5's matched filter, and the
/// only definition of it.
///
/// ```text
/// yield = food_value(cell) * (1 - |gut_bias - food_class| / 2)^2
/// ```
///
/// A gut tuned for cellulose is bad at flesh, so specialising costs
/// something and there is no free lunch. No transcendental, and — the
/// reason it is one scalar and not a per-class vector (E4) — **no free
/// dimension**: a normalised vector's overall magnitude is scale-invariant,
/// so nothing selects on it, it drifts, and a histogram of its alleles
/// measures that drift and reads as a result.
///
/// Squared rather than linear so the falloff is gentle near a match and
/// steep far from one: a small mis-specialisation should be survivable and
/// a large one should not, which linear cannot express.
///
/// Built on `food_value` rather than beside it, so a corpse's per-cell
/// stamp still sets the scale and the `FoodValue` overlay, the probe and
/// this cannot disagree about what is standing in a world.
pub fn diet_yield(world: &World, cell: Cell, gut_bias: f32) -> f32 {
    let worth = food_value(world, cell);
    if worth <= 0.0 {
        return 0.0;
    }
    let class = world.materials.get(cell.material).food_class;
    // Clamped rather than assumed in range: both operands are authored
    // f32s today and will be *mutated* f32s at S6, and a gut that drifted
    // past +-1 would otherwise make the squared term climb again past the
    // far end of the axis -- a carnivore at +3 rating leaves better than a
    // carnivore at +1. Clamping the quality, not the gene, keeps the
    // arithmetic honest without silently rewriting anybody's genome.
    let quality = (1.0 - (gut_bias - class).abs() / 2.0).clamp(0.0, 1.0);
    worth * quality * quality
}

/// **The yield below which a mouthful is not food at all**, and the number
/// that makes the gene change *behaviour* rather than only bookkeeping: a
/// meat-gut animal literally stops seeing leaves, because `FoodAdjacent`
/// and the eat verb read this same predicate.
///
/// 12.0 — a tenth of the 120 every authored food in the world carries.
/// Derived from the filter's own arithmetic rather than measured, and the
/// gap is left visible per `CLAUDE.md` rather than relabelled away:
///
/// ```text
/// gut     plant (class -1)     flesh (class +1)     reads as
/// 0.0     120 * 0.25 =  30     worth * 0.25         generalist, sees both
/// -0.8    120 * 0.81 =  97     120  * 0.01 =  1.2   blind to carrion
/// +0.8    120 * 0.01 = 1.2     120  * 0.81 =  97    blind to plants
/// ```
///
/// The bar has to sit below a generalist's 30 and above a specialist's
/// 1.2; a tenth of a mouthful is the round number in that gap. **It wants
/// re-deriving from WP-8's survival-versus-`gut_bias` sweep** once Lane A's
/// instruments land — a threshold set from an argument is exactly the shape
/// this project has been bitten by, and it is recorded as such here rather
/// than presented as measured.
pub const EAT_YIELD_THRESHOLD: f32 = 12.0;

/// A creature's diet, resolved **once** at a site that already holds the
/// organism rather than per neighbour cell.
///
/// This replaces `CreatureDef::food`, a `Vec<String>` resolved by name
/// against the material registry for every one of the eight neighbours,
/// every tick — ~32 string hashes per creature-tick, and the saving S3
/// promised and could not bank because the list was still the only thing
/// keeping ants off each other.
#[derive(Clone, Copy)]
struct Gut {
    bias: f32,
    /// Whose flesh counts as kin — see `is_living_kin`.
    species: SpeciesId,
    eats_kin: bool,
}

fn gut_of(world: &World, organism: u16, def: &CreatureDef) -> Gut {
    Gut {
        bias: world.organism(organism).map_or(0.0, |s| s.traits[TRAIT_GUT_BIAS]),
        species: world.organism(organism).map_or(SpeciesId(0), |s| s.species),
        eats_kin: def.eats_kin,
    }
}

/// Whether `cell` is living tissue of `species`.
///
/// **The diet axis provably cannot answer this, which is why it is asked
/// separately.** `ant` material is `food_class: 1.0` worth 120, and a
/// *starved* ant's corpse cell is `food_class: 1.0` worth `body_energy +
/// 0/cells` — also exactly 120. Same class, same number. No `gut_bias` and
/// no threshold separates them, so the handoff's "set the threshold so
/// ant-flesh lands below it" would take the starved corpse off the menu
/// with the nestmate — and the starved corpse is precisely the case S3's
/// structural stamp exists to keep edible, since an animal dead at zero
/// has no leftover to be worth.
///
/// So the difference gets stated as data, per `CLAUDE.md`, and the data
/// already existed: `creature_dies` writes a corpse **without** an organism
/// id. Live tissue belongs to somebody; carrion belongs to nobody.
///
/// Same species rather than any creature, so beetle-eats-ant predation
/// keeps working by construction rather than by a second exemption. It also
/// stops an animal eating its own tail — `adjacent_food` scans the head's
/// 8-neighbourhood, which contains the next link of its own chain, and the
/// name list was the only thing preventing that too.
fn is_living_kin(world: &World, cell: Cell, species: SpeciesId) -> bool {
    world.organism(cell.organism_id()).is_some_and(|s| s.species == species)
}

/// The first cell in the head's 8-neighbourhood this gut will take, with
/// what it is worth to *this* animal.
fn adjacent_food(world: &World, x: i32, y: i32, gut: Gut) -> Option<(i32, i32, material::MaterialId)> {
    NEIGHBOURS_8.iter().find_map(|&(dx, dy)| {
        let cell = world.get(x + dx, y + dy);
        if !gut.eats_kin && is_living_kin(world, cell, gut.species) {
            return None;
        }
        (diet_yield(world, cell, gut.bias) > EAT_YIELD_THRESHOLD).then_some((x + dx, y + dy, cell.material))
    })
}

/// One prey animal, seen: where it is and how far away.
///
/// Returned out of `sense` rather than kept inside it because the *effect*
/// counter needs it. `CLAUDE.md`'s standing rule is that a counter which
/// says a thing fired is only as good as a counter from the far side of the
/// call — "the beetle saw something" is worth nothing on its own, and
/// `CreatureStats::sight_approaches` is what says the sighting changed
/// where the animal went.
#[derive(Clone, Copy)]
pub struct Sighting {
    pub x: i32,
    pub y: i32,
    /// Cells, Euclidean, from the head that saw it.
    pub dist: f32,
}

/// What stops a sight line: **rock and soil, and nothing else.**
///
/// `Solid | Powder` covers stone, soil and every piece of floor clutter
/// (seed, litter, corpse), which pooled over 18 seeds are 25% / 21% / 17%
/// of everything that stops a ray. Clutter is not exempted here by name —
/// `SIGHT_EYE_LIFT` is what gets over it, which is one rule instead of a
/// material list that a new powder would silently fall out of.
///
/// **`Plant` deliberately does not block, and that is a measured decision
/// rather than an oversight.** Making foliage opaque costs *half the
/// sense* (median `los` at r64 0.667 -> 0.350 on `wetland`, worse than
/// halving the radius) and no eye height buys it back. It is also the
/// house ethos: a canopy that either passes sight perfectly or blocks it
/// perfectly is the binary outcome the rubble had. What a bush should do is
/// **attenuate** — shorten the effective radius through it — which is a
/// mechanism `creature-vision-sizing-2026-08-30.md` §4 priced at nothing
/// and left for whoever wants foliage to matter. `Liquid` is a non-question:
/// it moved blocking 0.7 points and median `los` not at all.
fn blocks_sight(world: &World, cell: Cell) -> bool {
    matches!(world.materials.kind(cell.material), MaterialKind::Solid | MaterialKind::Powder)
}

/// Is this cell a living animal **this gut would eat**?
///
/// **The same filter the mouth uses**, one step further out — exactly the
/// argument `BrainInput::FoodAdjacent`'s call site already makes: if the
/// eye used a wider test than the mouth, an animal would steer at food it
/// cannot digest, and the gene would be nutritional bookkeeping rather than
/// a behaviour. A meat gut stops *seeing* leaves.
///
/// Living tissue only (`MaterialKind::Creature`), which is what makes this
/// a prey sense rather than a general food sense: a corpse is a `Powder`
/// and stops the ray as clutter. Scavenging at a distance is a separate
/// design question and is not answered here.
///
/// Its own body is excluded by owner, and a nestmate by `eats_kin` — the
/// same two exemptions `adjacent_food` makes, for the same reasons.
fn is_visible_prey(world: &World, cell: Cell, gut: Gut, self_organism: u16) -> bool {
    if cell.organism_id() == self_organism || world.materials.kind(cell.material) != MaterialKind::Creature {
        return false;
    }
    if !gut.eats_kin && is_living_kin(world, cell, gut.species) {
        return false;
    }
    diet_yield(world, cell, gut.bias) > EAT_YIELD_THRESHOLD
}

/// **The distal sense: cast `SIGHT_RAYS` rays all round and return the
/// nearest prey any of them reached.**
///
/// Built to `Reports/creature-vision-sizing-2026-08-30.md`, which sized
/// every part of it before a line existed: reach 64, all-round, occluded by
/// rock and soil, eye one cell up.
///
/// **All-round rather than a forward cone, and the cone was measured.** A
/// +-60 degree cone throws away **a third of every sighting** at every
/// preset (r64 median 0.572 -> 0.400 on `wetland`) and saves nothing the
/// clock can resolve — the fan's cost is 16 rays either way unless the ray
/// count drops with it, and dropping it is a resolution cut, not a saving.
///
/// **Dispatched at the creature's own position**, never by scanning the
/// world for creature cells. §5's `locate` arm exists precisely to keep
/// that scan out of the answer: timed against a do-nothing control it
/// overstated the sense's cost **thirtyfold**.
///
/// **Occlusion makes this cheaper as well as weaker**, which is worth
/// knowing before anyone proposes relaxing it for performance: rays die on
/// the first blocker, so 8 -> 64 is an eightfold radius for a sixfold read
/// count (81 -> 485 cells per cast), well short of the 1,024 an
/// unobstructed fan would pay.
///
/// Returns `None` when nothing edible is in sight, which is also what an
/// eyeless species gets — but an eyeless species never reaches here, since
/// the caller tests `sight_range` before the call (`CreatureDef::sight_range`).
fn sight(world: &World, x: i32, y: i32, organism: u16, def: &CreatureDef, gut: Gut, reads: &mut u64) -> Option<Sighting> {
    let reach = def.sight_range;
    debug_assert!(reach > 0, "sight() called for a species with no eyes; the gate belongs at the call site");

    // The eye, lifted only through cells that do not themselves block.
    let mut ey = y;
    for _ in 0..scaled_cells(world, SIGHT_EYE_LIFT) {
        if blocks_sight(world, world.get(x, ey - 1)) {
            break;
        }
        ey -= 1;
    }
    // How far it actually got. **Prey is tested in the un-lifted frame and
    // blockers in the lifted one**, which is the same geometry the sizing
    // study's pairwise test used: the line runs eye-to-eye while the animal
    // it is looking for stands on the ground. Testing prey at the lifted
    // cell instead would sail a horizontal ray straight over every ant in
    // the world -- the sense would fire on nothing and read as a design
    // failure rather than a frame-of-reference one.
    let lift = y - ey;

    let mut best: Option<Sighting> = None;
    let mut best_d2 = i32::MAX;
    for i in 0..SIGHT_RAYS {
        let a = std::f32::consts::TAU * i as f32 / SIGHT_RAYS as f32;
        let (rdx, rdy) = (a.cos(), a.sin());
        for step in 1..=reach {
            let sx = x + (rdx * step as f32).round() as i32;
            let sy = ey + (rdy * step as f32).round() as i32;
            let (tx, ty) = (sx, sy + lift);
            let target = world.get(tx, ty);
            *reads += 1;
            if is_visible_prey(world, target, gut, organism) {
                let d2 = (tx - x) * (tx - x) + (ty - y) * (ty - y);
                if d2 < best_d2 {
                    best_d2 = d2;
                    best = Some(Sighting { x: tx, y: ty, dist: (d2 as f32).sqrt() });
                }
                break;
            }
            // One read when the eye did not rise, two when it did.
            let blocker = if lift == 0 {
                target
            } else {
                *reads += 1;
                world.get(sx, sy)
            };
            if blocks_sight(world, blocker) {
                break;
            }
        }
    }
    best
}

fn adjacent_nest(world: &World, x: i32, y: i32, def: &CreatureDef) -> bool {
    let Some(nest) = world.materials.id_of(&def.nest) else {
        return false;
    };
    NEIGHBOURS_8.iter().any(|&(dx, dy)| world.get(x + dx, y + dy).material == nest)
}

/// Local `|grad moisture|`, normalized. **The whole of termite-style
/// construction and excavation shaping** (`stigmergy-research.md` §4, the
/// eLife 2024 result): drop probability is multiplied by this and dig
/// probability by its inverse, so material accumulates at convex, drying
/// sites and excavation runs toward concave, wetter ones. Pillars, walls
/// and chambers are consequences of that bias. There is no "build a wall"
/// behaviour and wanting to write one is the signal to re-read that
/// section.
fn moisture_gradient(world: &World, x: i32, y: i32) -> f32 {
    let m = |px: i32, py: i32| world.field_at_bilinear(px as f32, py as f32).moisture;
    let gx = m(x + 4, y) - m(x - 4, y);
    let gy = m(x, y + 4) - m(x, y - 4);
    ((gx * gx + gy * gy).sqrt() / WORM_MOISTURE_SATURATION).clamp(0.0, 1.0)
}

/// Eat, pick up, drop, dig — each a **probability**, then a world-state
/// check.
///
/// **Probabilities rather than the design report's "output crossing 0.5".**
/// Two reasons, and the first is a bug the threshold would have shipped:
/// `squash` maps an authored weight of 0.9 to 0.474, so a plainly-authored
/// instinct sits *just under* a 0.5 gate and the verb never fires at all —
/// a knife-edge margin of exactly the kind `CLAUDE.md` says to prefer a
/// continuous quantity over. The second is that §6 of the same report asks
/// for drop probability to be *multiplied* by the moisture gradient, which
/// a boolean gate cannot express. A graded outcome also simply beats a
/// binary one here (the house ethos): ants that sometimes drop early build
/// ragged, real-looking walls, where a threshold builds a clean line.
fn act(world: &mut World, x: i32, y: i32, organism: u16, def: &CreatureDef, outputs: &[f32; brain::BRAIN_OUTPUTS], draw: &mut rng::Rng) {
    use brain::BrainOutput as O;
    let carrying = world.organism(organism).and_then(|s| s.carrying);
    let dig_urge = outputs[O::Dig as usize].clamp(0.0, 1.0);
    // **Feeding is its own verb, and it was not.** Both branches below used
    // to roll against `dig_urge`, so one weight decided whether an animal
    // excavated *and* whether it ate -- §13d's `(Bias, Dig, 0.4)`, added to
    // make ants dig at all, raised the baseline eating probability in the
    // same stroke, invisibly. Evolution cannot select a burrower against a
    // grazer while the two share a gene.
    let feed_urge = outputs[O::Feed as usize].clamp(0.0, 1.0);
    let drop_urge = outputs[O::Drop as usize].clamp(0.0, 1.0);
    let hungry = world.organism(organism).is_some_and(|s| s.energy < def.start_energy * def.hunger_fraction);

    // --- eat / pick up --------------------------------------------------
    let gut = gut_of(world, organism, def);
    if carrying.is_none() {
        if let Some((fxx, fyy, food)) = adjacent_food(world, x, y, gut) {
            if draw.unit_f32() < feed_urge {
                // **What the mouthful is worth, read off the mouthful.**
                // This is the keystone: it used to be `def.eat_energy`, a
                // constant of the *eater*, so a starved ant's corpse paid
                // its scavenger 120 out of an animal that had nothing
                // (§13l), and there was no such thing as a food being
                // nutritious -- which is why herbivore could not diverge
                // from carnivore. Read before the cell is cleared, and
                // through the same `food_value` the overlay and the probe
                // call, so a picture of the food in a world cannot disagree
                // with what an animal gets for biting it.
                let bite = world.get(fxx, fyy);
                // **Two numbers, and conflating them is a bug in both
                // directions.** `worth` is what the mouthful is worth to
                // anybody -- it is what goes back into the world if this
                // is a pickup rather than a bite, because *carrying food
                // does not digest it* and a leaf ferried by a carnivore
                // must not arrive at the nest devalued. `gain` is what
                // this gut gets out of swallowing it, which since S5 is
                // the matched filter and not the cell's face value.
                //
                // Gating on the filter without paying it was the first
                // version of this change, and `ascii` said so in the way
                // `CLAUDE.md` warns is the tell: byte-identical output
                // across a change that had to move something. Every food
                // still cleared the threshold at the shipped neutral gut,
                // so the menu did not move -- and nothing else could,
                // because the eat verb was still crediting face value.
                let worth = food_value(world, bite);
                let gain = diet_yield(world, bite, gut.bias);
                let banked = world.materials.get(bite.material).worth_in_aux;
                let shade = bite.shade;
                // If the mouthful belonged to somebody, tell them. Without
                // this the victim keeps running on a chain that includes
                // the cell just removed from it.
                let victim = bite.organism_id();
                world.set(fxx, fyy, Cell::EMPTY);
                if victim != 0 && victim != organism {
                    reconcile_chain(world, victim);
                }
                if hungry {
                    if let Some(state) = world.organism_mut(organism) {
                        state.energy += gain;
                    }
                    // **Two accounts, because they are two different
                    // things.** Meat came out of a stock something paid
                    // for; plant matter is still free, and saying so in the
                    // census is the only reason the sealed-box test can
                    // tell a closed loop from the sun.
                    //
                    // **Booked at `gain`, not at `worth`, and which one it
                    // is decides which identity survives S5.** The live
                    // identity is an equality and catches charges that do
                    // not land, so it has to see exactly what the animal
                    // received; booking face value against a gut that only
                    // absorbed a quarter of it would conjure the rest. The
                    // meat identity is already an upper bound (see
                    // `EnergyLedger`), and under-subtracting from it leaves
                    // it a *valid* bound, just looser -- the digestive loss
                    // joins the `meat_lost` seam as slack rather than
                    // breaking anything. If that slack is ever wanted
                    // tight, it needs its own sink term beside
                    // `meat_lost`, which is WP-6's file and not this one's.
                    if banked {
                        world.energy_ledger.harvested_corpse += gain as f64;
                    } else {
                        world.energy_ledger.harvested_plant += gain as f64;
                    }
                    world.creature_stats.eats += 1;
                } else {
                    // Full: carry it home instead of eating it. This is the
                    // whole reason a colony accumulates stores rather than
                    // every ant simply feeding itself.
                    if let Some(state) = world.organism_mut(organism) {
                        state.carrying = Some(Carried { material: food, worth: quantise_worth(worth), shade });
                    }
                    world.creature_stats.pickups += 1;
                }
                return;
            }
        }
    }

    // --- eat what is being carried --------------------------------------
    //
    // **An animal holding food must not starve to death holding it**, and
    // before this it always did. The eat branch above is gated on
    // `carrying.is_none()` and the drop branch below returns
    // unconditionally, so a laden creature had no path back to feeding: the
    // only way to put a load down was to *want* to drop it, and out on the
    // route that roll is multiplied by the moisture gradient.
    //
    // Found by the sessile-grazer probe, which could not be made to
    // measure anything until it was fixed. Every run of that scene ended
    // the same way and at the same count -- the ant ate until one meal
    // carried it back over `hunger_fraction`, picked the next cell up
    // instead of eating it, and then stood still until it starved. It read
    // as "moss is not a pump", and it was really "nothing in this scene can
    // eat more than twice": an unlimited static wall of leaf produced
    // **exactly the same 2 eats and the same death**, which is the control
    // that separated the two. `CLAUDE.md`: when every setting fails the
    // same way, suspect the sweep.
    //
    // Deliberately gated on `hungry` and not on the drop rules, because
    // this is the starvation path and not a second way to unload. A colony
    // ant only reaches it when it is under half its budget with a full
    // mandible, which is a real trade -- the load was going to the nest and
    // now it does not -- and it is the trade that keeps the ant alive to
    // carry the next one.
    if let (Some(held), true) = (carrying, hungry) {
        if draw.unit_f32() < feed_urge {
            // The gut applies here too: swallowing what you are holding is
            // still swallowing. The stored `worth` stays face value because
            // the *other* exit from this branch puts the cell back in the
            // world (see `Carried`), and only this one digests it.
            let gain = diet_yield(world, held.into_cell(world), gut.bias);
            if let Some(state) = world.organism_mut(organism) {
                state.energy += gain;
                state.carrying = None;
            }
            if world.materials.get(held.material).worth_in_aux {
                world.energy_ledger.harvested_corpse += gain as f64;
            } else {
                world.energy_ledger.harvested_plant += gain as f64;
            }
            world.creature_stats.eats += 1;
            return;
        }
    }

    // --- drop -----------------------------------------------------------
    if let Some(held) = carrying {
        let at_nest = adjacent_nest(world, x, y, def);
        // At the nest it is storage and always wanted; out on the route it
        // is construction, and *there* the moisture bias decides.
        let p = if at_nest { drop_urge } else { drop_urge * moisture_gradient(world, x, y) };
        if draw.unit_f32() < p {
            if let Some((dx, dy)) = NEIGHBOURS_8.iter().map(|&(dx, dy)| (x + dx, y + dy)).find(|&(px, py)| world.is_empty(px, py)) {
                world.set(dx, dy, held.into_cell(world));
                if let Some(state) = world.organism_mut(organism) {
                    state.carrying = None;
                }
                world.creature_stats.drops += 1;
                if at_nest {
                    world.creature_stats.deliveries += 1;
                }
            }
        }
        return;
    }

    // --- dig --------------------------------------------------------------
    // Only reached with nothing to eat and nothing carried. Gated on the
    // material's own `penetration_resistance` against this species'
    // `dig_force` -- the pattern roots already use, never a name whitelist,
    // so a future softer stone becomes diggable with no code change.
    if draw.unit_f32() < dig_urge * (1.0 - moisture_gradient(world, x, y)) {
        let heading = world.organism(organism).map_or(0, |s| s.heading);
        let (dx, dy) = DIRS[heading as usize];
        let (tx, ty) = (x + dx, y + dy);
        let target = world.get(tx, ty);
        if target.material != material::EMPTY && world.materials.get(target.material).penetration_resistance <= def.dig_force {
            // Spoil is destroyed in v1. Carrying it out is a stage-4+
            // refinement (worms already eat their tunnels, so there is
            // precedent) -- noted, not built.
            world.set(tx, ty, Cell::EMPTY);
            world.creature_stats.digs += 1;
        }
    }
}

/// Move the whole chain one cell, snake-fashion. Returns whether it moved.
fn step_chain(
    world: &mut World,
    organism: u16,
    heading: u8,
    outputs: &[f32; brain::BRAIN_OUTPUTS],
    def: &CreatureDef,
    draw: &mut rng::Rng,
) -> bool {
    let Some(chain) = world.organism(organism).map(|s| s.chain.clone()) else {
        return false;
    };
    let Some(&(hx, hy)) = chain.first() else {
        return false;
    };

    // --- support: a whole-chain rule (P-25) -----------------------------
    // **Which object does this rule evaluate? The piece.** Asked and
    // answered in advance, because the per-cell version of exactly this
    // question took slabs apart one knife-edge footing at a time. A chain
    // is held up if *any* of its cells touches something solid; evaluating
    // per cell would drop the front half of an ant walking off a ledge and
    // leave the back half standing.
    //
    // 8-neighbour, so ants climb walls and ceilings. That is correct (real
    // ones do) and it is what makes a side-view world traversable at all.
    let supported = chain.iter().any(|&(cx, cy)| {
        NEIGHBOURS_8.iter().any(|&(dx, dy)| {
            matches!(world.materials.kind(world.get(cx + dx, cy + dy).material), MaterialKind::Solid | MaterialKind::Powder | MaterialKind::Plant)
        })
    });
    if !supported {
        let fallen: Vec<(i32, i32)> = chain.iter().map(|&(cx, cy)| (cx, cy + 1)).collect();
        if fallen.iter().all(|&(cx, cy)| world.is_empty(cx, cy) || chain.contains(&(cx, cy))) {
            relocate_chain(world, organism, &chain, &fallen);
            world.creature_stats.falls += 1;
            return true;
        }
    }

    // --- choose among the three forward candidates ----------------------
    // Never an argmax (P-10); see `choose_weighted`. The turn output biases
    // left and right with opposite signs, which is the one connection an
    // authored lateral-to-turn instinct needs in order to steer.
    let turn = outputs[brain::BrainOutput::Turn as usize];
    let dirs = [(heading + AHEAD_LEFT) % 8, heading, (heading + AHEAD_RIGHT) % 8];
    // **Persistence and caution come from the brain, not from literals.**
    // The straight-ahead score used to be an anonymous `0.15` and the
    // foothold preference a `FOOTING_BONUS` const; together they decided
    // essentially everything a creature did, which is why an ablation found
    // eight of ten authored instincts bit-identical. See
    // `brain::BrainOutput::Persist`.
    let persist = brain::unit_scale(outputs[brain::BrainOutput::Persist as usize], PERSIST_MAX);
    let footing = brain::unit_scale(outputs[brain::BrainOutput::Caution as usize], FOOTING_MAX);
    let base = [turn.max(0.0), persist, (-turn).max(0.0)];
    let mut scores = [0.0f32; 3];
    let mut passable = [false; 3];
    // Resolved once for all three candidates -- see `kin_footing`.
    let kin = kin_footing(world, organism, def);
    for (i, &d) in dirs.iter().enumerate() {
        let (dx, dy) = DIRS[d as usize];
        let (tx, ty) = (hx + dx, hy + dy);
        // Raw emptiness, plus "my own tail", which a chain may legitimately
        // step into because it vacates on the same tick.
        // **Every cell of the body, not just the head.** A chain gets away
        // with checking only the head because the body steps into cells the
        // head has already vacated; a rigid body translates as a unit, so a
        // 2-wide creature squeezing into a 1-wide gap has to be refused
        // here or it overlaps the world. This is also, with no other code,
        // the reason a wide predator cannot follow a narrow ant into its
        // tunnel.
        let landing = body_after_step(def, &chain, (tx, ty), heading, d);
        passable[i] = landing.iter().all(|&p| world.is_empty(p.0, p.1) || chain.contains(&p));
        // **Footing, not just emptiness — and this was measured, not
        // anticipated.** Without it the counters read 16,451 falls against
        // 22,138 moves: an ant on flat ground steps diagonally up into open
        // air, finds nothing under it, falls straight back, and does that
        // forever. Every part of it was behaving correctly and the colony
        // still spent three quarters of its moves bouncing.
        //
        // A walking creature prefers a foothold. Unsupported candidates are
        // not *forbidden* -- an ant must still be able to walk off a ledge,
        // and forbidding it would be the "size cap that gates whether
        // something happens" mistake -- they are just heavily discounted.
        scores[i] = if passable[i] {
            // **Added, not multiplied.** Multiplying was tried first and
            // was far too weak: `choose_weighted`'s exploration floor `k`
            // dominates every small score, so a discount of 20x on a base
            // of 0.15 still left a step into thin air at 16% of the
            // probability, and the colony spent 59% of its moves falling.
            // A footing *bonus* puts the supported candidate an order of
            // magnitude clear of the floor, where the discount belongs.
            base[i] + if body_has_foothold(world, def, &landing, (tx, ty), kin) { footing } else { 0.0 }
        } else {
            0.0
        };
    }
    // **No purchase anywhere ahead counts as blocked**, and this is the
    // fix the two before it were groping at. An ant whose heading is
    // *upward* has all three candidates in open air; nothing is blocking
    // it, so it marched into the sky, and its chain only fell a tick later
    // once the tail had left the ground. Discounting the airborne
    // candidates could never help, because there was nothing else to pick:
    // the discount was being applied to every option equally. Falls ran at
    // 59-80% of all moves through both earlier attempts.
    //
    // An ant that can see no footing ahead turns to look somewhere else,
    // which is what a real one does and what makes it stop walking off
    // ledges. The cost is that ants cannot cross a gap; ants do not.
    let footing_ahead = passable.iter().zip(&scores).any(|(&p, &s)| p && s > footing * 0.5);
    if !passable.iter().any(|&p| p) || !footing_ahead {
        // Turn to a new heading and **deposit nothing this tick** (P-11,
        // enforced by the caller's `moved` flag). A blocked agent that
        // reinforces is how a colony paints a trail into a wall.
        //
        // **Re-rolled among headings that have somewhere to go**, not
        // uniformly over all eight. On flat ground three of the eight point
        // upward into open air, so a uniform re-roll lands back in the
        // blocked state better than a third of the time -- measured at
        // 29,344 blocked ticks against 41,843 moves, a colony spending more
        // of its life turning on the spot than walking. Uniform among the
        // *viable* directions costs one extra scan of eight cells on a path
        // that was about to do nothing anyway.
        tumble(world, organism, def, draw);
        world.creature_stats.moves_blocked += 1;
        return false;
    }

    // Zero out anything without footing now that at least one candidate
    // has some: the discount was never the mechanism, the choice is.
    for (i, s) in scores.iter_mut().enumerate() {
        if !passable[i] || *s <= footing * 0.5 {
            *s = 0.0;
        }
    }
    let pick = choose_weighted(&scores, CHOICE_EXPLORATION_K, draw.unit_f32());
    let pick = if scores[pick] > 0.0 { pick } else { scores.iter().position(|&s| s > 0.0).expect("footing_ahead guarantees one") };
    let new_heading = dirs[pick];
    let (dx, dy) = DIRS[new_heading as usize];
    let (tx, ty) = (hx + dx, hy + dy);

    let next = body_after_step(def, &chain, (tx, ty), heading, new_heading);
    relocate_chain(world, organism, &chain, &next);
    if let Some(state) = world.organism_mut(organism) {
        state.heading = new_heading;
    }
    world.creature_stats.moves += 1;

    // How deep this excursion has got, in cells from the last nest contact.
    // **Measurement only** — nothing downstream reads it, and an ant still
    // has no idea where home is. See `OrganismState::forage_anchor`.
    if let Some(state) = world.organism_mut(organism) {
        let (ax, ay) = state.forage_anchor;
        // Chebyshev, because movement is an 8-neighbour step: it is the
        // number of moves a straight walk home would take, which is what
        // "range" means to a creature that can go diagonally for free.
        let depth = (tx - ax).abs().max((ty - ay).abs());
        state.forage_max = state.forage_max.max(depth.clamp(0, u16::MAX as i32) as u16);
    }

    // Touching the nest resets the scent clock, which is what makes channel
    // A a gradient rather than a uniform smear.
    if adjacent_nest(world, tx, ty, def) {
        if let Some(state) = world.organism_mut(organism) {
            // Only count it as a visit if the creature had actually been
            // away: an ant loitering on the nest would otherwise register
            // one every tick and the counter would say nothing.
            //
            // **It does not work, and the counter does say nothing.**
            // `since_nest` is incremented unconditionally every tick a few
            // hundred lines up, so this guard is false exactly once per
            // lifetime and every nest-adjacent move scores. Left standing
            // because scenes print it and the ratio against `moves` is a
            // real readout; the trip counter below is the fix, and it is a
            // *distance* rather than a repair of this clock — see
            // `OrganismState::forage_anchor` for why repairing the clock
            // cannot work.
            if state.since_nest > 0 {
                world.creature_stats.nest_visits += 1;
            }
            let state = world.organism_mut(organism).expect("live");
            // **The reset the homing gradient depends on.** `recency` in the
            // deposit block above is `1 - since_nest / nest_memory`, so
            // without this every ant's channel-A deposit decays to zero and
            // the trail stops pointing home. An edit here dropped this one
            // line while adding the trip counter below, and *nothing in the
            // suite went red* -- 827 tests passed, clippy was clean, and
            // `ascii`'s own scenes still delivered food. The paired baseline
            // run is what caught it. Re-read this function after any edit to
            // it, not the diff.
            state.since_nest = 0;
            let depth = state.forage_max;
            // Re-anchor on *every* contact, including the ones that book
            // nothing. That is what stops an ant strolling the length of a
            // 32-cell nest patch from accumulating a 30-cell "excursion".
            state.forage_anchor = (tx, ty);
            state.forage_max = 0;
            // The profile is booked for *every* excursion, including the
            // one-cell ones — it is the distribution that makes the bar
            // below defensible, so it must not be filtered by that bar.
            // **The ruler is in cells, so it moves with the grid.** Left
            // fixed, the same physical excursion books one bucket higher in
            // a 2x world and two colonies at different resolutions cannot be
            // compared at all -- the histogram would be reporting the cell
            // size rather than the foraging.
            for (i, &edge) in FORAGE_REACH_BUCKETS.iter().enumerate() {
                if depth as i32 >= scaled_cells(world, edge as i32) {
                    world.creature_stats.forage_reach[i] += 1;
                }
            }
            if depth as i32 >= scaled_cells(world, FORAGE_TRIP_MIN as i32) {
                world.creature_stats.forage_trips += 1;
                world.creature_stats.forage_depth_sum += depth as u64;
            }
            // Outside the bar: the deepest excursion is a fact about the
            // colony whatever the bar is set to, and a run whose max sits
            // under `FORAGE_TRIP_MIN` needs it *most* — that is exactly the
            // run where `forage_trips` is 0 and cannot say whether the
            // colony moved 15 cells or 1.
            world.creature_stats.forage_depth_max = world.creature_stats.forage_depth_max.max(depth as u64);
        }
    }
    true
}

// --- the impulse verb: leaving the ground -------------------------------
//
// `Reports/creature-motion-design.md` is the whole argument; three things
// from it are load-bearing enough to repeat at the code.
//
// **1. Nothing here touches the walk.** §2d records that `step_chain`'s
// refusal to step into unsupported air is deliberate and was arrived at
// after two failed attempts at airborne creatures, both of which put falls
// at 59-80% of all moves. Both failed the same way: they changed the
// *candidate scoring*, so every ant in the world became airborne whether it
// wanted to or not. This verb adds a separate, opt-in path -- a creature is
// in the air only if `OrganismState::flight` is `Some`, and only `launch`
// ever sets it. An ant with no `Impulse` weight reads `squash(0.0) == 0.0`
// exactly, takes no RNG draw, and behaves bit-for-bit as it did before the
// slot existed.
//
// **2. There is no table of creature types.** §5's five rows -- a short
// chain hops far, a long one shallower, a wide slab barely leaves the
// ground but glides, a compact block drops like a stone, a buoyant body
// floats -- are all produced by three numbers a body already has: the sum
// of its cells' densities, its bounding box, and the density of whatever it
// is in. `match def.body` appears nowhere below, and `match species` must
// never appear. That is `CLAUDE.md`'s "state the difference as data", which
// four successive support models learned the expensive way.
//
// **3. The verb has to cost something.** §1: a trait that is strictly
// better makes every lineage converge on it, which is measured rather than
// hypothetical (S5's diet genes produced one animal). A launch costs a flat
// price in energy and, more sharply, costs the creature its *turn*: while
// airborne it does not think, eat, dig, steer or deposit. A light body buys
// a lot of ground with that; a heavy one buys almost none.

/// Per-frame downward acceleration. **Deliberately the same 0.15 as
/// `rigid.rs` and `particle.rs`**, and duplicated for the same reason those
/// two are: a hopping ant, a falling rock and blast debris share a screen,
/// and two different fall rates in one scene reads as a bug even when
/// neither is wrong.
const GRAVITY: f32 = 0.15;

/// Hard cap per axis, matching `rigid.rs`'s `MAX_SPEED_PER_AXIS`. The drag
/// law below already bounds the descent; this bounds the *launch* of a body
/// so light that `LAUNCH_WORK` would otherwise fling it across the world in
/// two frames, and it bounds the substep count so one flight frame can
/// never test hundreds of intermediate positions.
const MAX_FLIGHT_SPEED: f32 = 6.0;

/// **The work one launch does, and the only number that sets how far
/// anything jumps.**
///
/// Work rather than impulse, and that choice is what makes §5's table fall
/// out instead of being written down. A fixed impulse gives `v = J/m`; a
/// fixed *energy* gives
///
/// ```text
///     v = sqrt( 2 W / m )
/// ```
///
/// -- the same `1/sqrt(m)` that a muscle's force scaling with its
/// cross-section produces, and the reason small animals out-jump large ones
/// by so much less than `1/m` would predict. At `1/m` the spread across the
/// four authored bodies is 4.5x in speed and the 6-cell chain is already
/// immobile, which leaves no room between "shallower hop" and "almost
/// nothing"; at `1/sqrt(m)` it is 2.1x in speed and 4.5x in height and
/// every row of §5's table is distinguishable.
///
/// **Set from a specification, not from a scene** (`CLAUDE.md`, and
/// `rigid::CROWN_SPEED` is the worked precedent): *the heaviest authored
/// body clears about a cell and a half -- a hop over a pebble -- and
/// everything lighter scales from there.* At 9 cells that is
/// `sqrt(2 * 0.15 * 1.5) / sin(45 deg)` = 0.94 cells/frame, so
/// `W = m v^2 / 2` = 4.0. What the four shipped bodies then do, in cells:
///
/// | body | cells | launch v | rise | ballistic range |
/// |---|---|---|---|---|
/// | `ant` `Chain(2)` | 2 | 2.00 | 6.7 | 26.7 |
/// | `ant_long` `Chain(6)` | 6 | 1.15 | 2.2 | 8.9 |
/// | `ant_wide` 5x2 | 9 | 0.94 | 1.5 | 5.9 |
/// | `ant_block` 3x3 | 9 | 0.94 | 1.5 | 5.9 |
///
/// The last two are the same mass and so launch identically **on purpose**:
/// what separates them is the descent, and it only separates them over a
/// drop taller than their own hop. That is why the review scene is a ledge.
const LAUNCH_WORK: f32 = 4.0;

/// How much of the launch goes upward, added to the heading's own y.
///
/// A hop is up-and-forward, and one constant is the whole of it: an
/// east-facing creature (`DIRS` y of 0) leaves at 45 degrees, one already
/// facing up-east leaves steeply, one facing down-east skims. No per-species
/// jump angle, and no clamp forcing every body onto the same arc -- a
/// creature that wants a flatter hop turns first, which is a decision the
/// brain already has an output for.
const LAUNCH_LIFT: f32 = 1.0;

/// What a launch costs, as a multiple of the species' own `move_cost`.
///
/// **Flat, and the flatness is the mechanism.** One price, and the body
/// decides what it buys: the 2-cell ant covers ~27 cells for four walking
/// steps' worth of energy, which is a bargain, and the 9-cell block covers
/// ~6 for the same, which is worse than walking. Nothing anywhere says
/// "heavy creatures should not jump" -- it is simply a bad deal for them,
/// which is §1's cost-and-benefit rather than a rule.
const LAUNCH_COST_IN_MOVES: f32 = 4.0;

/// The density the air is given, in the same units a material's `density`
/// is written in.
///
/// **Set from what a fall should look like, not from a physical table, and
/// the physical value is unusable here.** Real air is ~1/1000 of chitin, at
/// which the drag term below is numerically invisible and every body falls
/// at exactly the same rate -- which would delete the glide the whole design
/// turns on. The specification instead: *a lone 2-cell ant reaches terminal
/// velocity at the speed a ten-cell free fall would give it* (1.73
/// cells/frame, `v = sqrt(2 g d)`), and solving `terminal_speed` backwards
/// for that body gives this.
///
/// Everything else follows from it with no further tuning. Terminal speeds,
/// in cells per frame:
///
/// | body | mass | Cd | frontal width | terminal |
/// |---|---|---|---|---|
/// | `ant` (2x1) | 2 | 1.25 | 2 | 1.73 |
/// | `ant_long` strung out (6x1) | 6 | 2.00 | 6 | 1.37 |
/// | `ant_wide` (5x2) | 9 | 1.63 | 5 | 2.04 |
/// | `ant_block` (3x3) | 9 | 0.50 | 3 | **4.74** |
///
/// The last two rows are the design claim in one line: **same mass, same
/// launch, and the block comes down 2.3x faster than the slab.** That ratio
/// is what this constant does *not* set — it is scale-free in
/// `AIR_DENSITY`, being a ratio of two `sqrt(1/(Cd A))` — so tuning this
/// changes how brisk every fall looks and changes none of the differences
/// between bodies. It is the one knob to reach for if a hop reads as
/// floating, and reaching for anything else would be re-tuning the design.
const AIR_DENSITY: f32 = 0.08;

/// Drag coefficient of a body that presents a flat plate, and of one that
/// presents a compact blunt shape.
///
/// **Not invented here.** `rigid::SINK_DRAG_COEFFICIENT`'s own doc records
/// the regime check for pixel-scale rubble -- *"Flat plates are nearer 2 and
/// spheres nearer 0.5"* -- and settles on 2.0 for a tumbling irregular
/// solid. A creature's shape is *known* rather than irregular, so the two
/// ends of that same range become the two ends of a ramp instead of one
/// being picked.
const PLATE_DRAG: f32 = 2.0;
/// See [`PLATE_DRAG`].
const COMPACT_DRAG: f32 = 0.5;

/// The width-to-height ratio at which a body counts as a full flat plate.
///
/// 3.0 rather than something larger so the shipped `ant_wide` (5 wide by 2
/// tall, ratio 2.5) lands most of the way up the ramp rather than at the
/// bottom of it. A body taller than it is wide gets `COMPACT_DRAG`: a chain
/// standing on end is a needle, not a parachute, and it should fall like
/// one.
const PLATE_FLATNESS: f32 = 3.0;

/// A body's mass, frontal width and drag coefficient -- everything the
/// ballistics needs, read off the cells themselves.
///
/// **Recomputed per flight frame rather than cached on the species, and
/// that is the point rather than an inefficiency.** A `Chain` has no fixed
/// shape: six cells strung out along a ledge are a 6x1 plate and the same
/// six coiled at a corner are a 3x3 block, and this reports 2.0 and 0.5
/// respectively for a creature whose species file never mentions drag. The
/// cost is a walk over a body 2-9 cells long, on the frames it is airborne
/// only.
struct BodyDrag {
    /// Summed material density over the body's cells. Cell count when every
    /// cell is density 1.0, which every creature material currently is.
    mass: f32,
    /// Frontal width presented to a vertical fall: the bounding box's
    /// horizontal extent, in cells.
    width: f32,
    /// Interpolated between `COMPACT_DRAG` and `PLATE_DRAG` on how much
    /// wider than tall the body is.
    drag: f32,
    /// Mean density, for the buoyancy comparison against a fluid.
    density: f32,
}

fn body_drag(world: &World, cells: &[(i32, i32)]) -> BodyDrag {
    let mut mass = 0.0;
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for &(cx, cy) in cells {
        mass += world.materials.density(world.get(cx, cy).material);
        min_x = min_x.min(cx);
        max_x = max_x.max(cx);
        min_y = min_y.min(cy);
        max_y = max_y.max(cy);
    }
    let width = (max_x - min_x + 1).max(1) as f32;
    let height = (max_y - min_y + 1).max(1) as f32;
    // Wider than tall ramps toward a plate; anything square or taller is
    // blunt.
    let t = ((width / height - 1.0) / (PLATE_FLATNESS - 1.0)).clamp(0.0, 1.0);
    let n = cells.len().max(1) as f32;
    let mass = mass.max(f32::MIN_POSITIVE);
    BodyDrag { mass, width, drag: COMPACT_DRAG + t * (PLATE_DRAG - COMPACT_DRAG), density: mass / n }
}

/// The fluid this body is in: the density of any `Liquid` touching it, or
/// [`AIR_DENSITY`].
///
/// **Asked of the neighbours, not of the body's own cells**, because a
/// creature's cells hold the creature -- reading `world.get` at the head
/// returns chitin and would report every ant as flying through solid ant.
/// The same trap `rigid::surrounding_liquid` exists to avoid, and the same
/// answer.
fn surrounding_density(world: &World, cells: &[(i32, i32)]) -> f32 {
    for &(cx, cy) in cells {
        for (dx, dy) in NEIGHBOURS_8 {
            let m = world.get(cx + dx, cy + dy).material;
            if world.materials.kind(m) == MaterialKind::Liquid {
                return world.materials.density(m).max(f32::MIN_POSITIVE);
            }
        }
    }
    AIR_DENSITY
}

/// Terminal speed for this body in this fluid, in cells per frame.
///
/// Weight-minus-buoyancy balanced against quadratic drag, which is
/// `rigid.rs`'s own derivation with the frontal area written out rather
/// than folded into a size term:
///
/// ```text
///     v = sqrt( 2 m g_eff / (rho_fluid * Cd * A) )
/// ```
///
/// `g_eff` is `GRAVITY` less the share the displaced fluid carries, exactly
/// as `rigid::drag_through_liquid` computes it -- one buoyancy model in the
/// engine, not two. A body no denser than what it is in has `g_eff == 0`
/// and hangs: **that is the float limit decision E9 asks for, and it needed
/// no new physics** (§2c). It also cannot be evolved around, because the
/// only way to get it is to be made of something light, and being made of
/// something light is what makes a body easy to shift.
fn terminal_speed(shape: &BodyDrag, fluid_density: f32, gravity_effective: f32) -> f32 {
    let denom = fluid_density * shape.drag * shape.width;
    if denom <= 0.0 || gravity_effective <= 0.0 {
        return 0.0;
    }
    (2.0 * shape.mass * gravity_effective / denom).sqrt().min(MAX_FLIGHT_SPEED)
}

/// The share of gravity the surrounding fluid carries -- near zero in air, 1
/// for a body no denser than the liquid it is in. `rigid::
/// drag_through_liquid`'s `carried`, unchanged.
fn buoyant_share(body_density: f32, fluid_density: f32) -> f32 {
    (fluid_density / body_density.max(f32::MIN_POSITIVE)).min(1.0)
}

/// Try to translate the whole body by `(dx, dy)`.
///
/// **The whole body, rigidly, for both body plans** -- unlike `step_chain`,
/// where a chain's segments follow the head into cells it has vacated. A
/// chain walking is a queue of legs; a chain in the air is one object, and
/// having it snake through its own trail mid-flight would read as a glitch
/// rather than as motion.
fn translated_if_free(world: &World, cells: &[(i32, i32)], dx: i32, dy: i32) -> Option<Vec<(i32, i32)>> {
    let to: Vec<(i32, i32)> = cells.iter().map(|&(x, y)| (x + dx, y + dy)).collect();
    // Its own cells are not obstacles: a one-cell step overlaps them.
    to.iter().all(|p| world.is_empty(p.0, p.1) || cells.contains(p)).then_some(to)
}

/// Is this body held up by anything? The identical predicate `step_chain`
/// opens with, shared rather than copied so a landing and a walk can never
/// disagree about what counts as ground.
fn body_is_supported(world: &World, cells: &[(i32, i32)]) -> bool {
    cells.iter().any(|&(cx, cy)| {
        NEIGHBOURS_8.iter().any(|&(dx, dy)| {
            matches!(world.materials.kind(world.get(cx + dx, cy + dy).material), MaterialKind::Solid | MaterialKind::Powder | MaterialKind::Plant)
        })
    })
}

/// **The verb.** Convert one launch's worth of work into a velocity and put
/// the creature in the air. `false` if the body could not push off.
///
/// The only thing in the engine that sets `OrganismState::flight`.
fn launch(world: &mut World, organism: u16, heading: u8) -> bool {
    let Some(cells) = world.organism(organism).map(|s| s.chain.clone()) else {
        return false;
    };
    if cells.is_empty() {
        return false;
    }
    // **You cannot push off nothing.** A creature already falling has
    // nothing to push against, so the verb is refused rather than granted
    // for free -- which also stops it being a mid-air double jump, and stops
    // it being a way to cancel a fall.
    if !body_is_supported(world, &cells) {
        world.creature_stats.impulses_refused += 1;
        return false;
    }
    let shape = body_drag(world, &cells);
    let speed = (2.0 * LAUNCH_WORK / shape.mass).sqrt().min(MAX_FLIGHT_SPEED);
    // Along the heading, plus a fixed lift, normalised -- so `LAUNCH_WORK`
    // sets the speed and the heading only chooses the direction.
    let (dx, dy) = DIRS[heading as usize];
    let (ax, ay) = (dx as f32, dy as f32 - LAUNCH_LIFT);
    let len = (ax * ax + ay * ay).sqrt().max(f32::MIN_POSITIVE);
    if let Some(state) = world.organism_mut(organism) {
        state.flight = Some(Flight { vx: speed * ax / len, vy: speed * ay / len, fx: 0.0, fy: 0.0 });
    }
    world.creature_stats.impulses += 1;
    true
}

/// One frame of ballistics for an airborne creature, and the reschedule.
///
/// **Every frame, not every `tick_interval`.** An ant decides once every 6
/// frames; a hop lasting 16 frames integrated at that rate would be three
/// teleports. The extra scheduler traffic is real and is counted
/// (`CreatureStats::flight_frames`), and it is bounded by the arc: gravity
/// always wins, so a body that went up comes down and stops being
/// rescheduled this way.
///
/// **No brain, no verbs, no deposit.** The creature is committed -- it
/// cannot eat, dig, drop, steer or lay pheromone until it lands. That is
/// most of what the verb costs, and it is also why the extra frames are
/// cheap: there is no `eval_brain` on any of them.
fn step_flight(world: &mut World, organism: u16, def: &CreatureDef) -> Vec<ActiveSite> {
    let (Some(mut flight), Some(mut cells)) =
        (world.organism(organism).and_then(|s| s.flight), world.organism(organism).map(|s| s.chain.clone()))
    else {
        return Vec::new();
    };
    if cells.is_empty() {
        if let Some(state) = world.organism_mut(organism) {
            state.flight = None;
        }
        return Vec::new();
    }

    let shape = body_drag(world, &cells);
    let fluid = surrounding_density(world, &cells);
    let carried = buoyant_share(shape.density, fluid);
    let gravity_effective = GRAVITY * (1.0 - carried);
    let terminal = terminal_speed(&shape, fluid, gravity_effective);

    flight.vy += gravity_effective;
    // Downward only: this is a *terminal* velocity, and clamping the rising
    // half of an arc with it would collapse every launch speed to the same
    // number and delete the mass law the whole design rests on.
    flight.vy = flight.vy.min(terminal);
    // Horizontal drag in a liquid only. In air a hop keeps the ground speed
    // it left with until it lands, which is what makes a glide *go
    // somewhere*: the slab's advantage is the twenty-odd frames it stays up,
    // and bleeding vx away would hand that straight back.
    if fluid > AIR_DENSITY {
        flight.vx = flight.vx.clamp(-terminal, terminal);
    }
    flight.vx = flight.vx.clamp(-MAX_FLIGHT_SPEED, MAX_FLIGHT_SPEED);
    flight.vy = flight.vy.clamp(-MAX_FLIGHT_SPEED, MAX_FLIGHT_SPEED);

    flight.fx += flight.vx;
    flight.fy += flight.vy;

    // Substep one cell at a time, the same anti-tunnelling rule
    // `rigid::advance` and `particle::advance_and_check_landing` both use: a
    // body moving 3 cells this frame must test the two it passes through, or
    // it walks through a one-cell floor.
    let mut landed = false;
    let mut moves = 0u64;
    loop {
        let sx = axis_step(flight.fx);
        let sy = axis_step(flight.fy);
        if sx == 0 && sy == 0 {
            break;
        }
        if let Some(to) = translated_if_free(world, &cells, sx, sy) {
            relocate_chain(world, organism, &cells, &to);
            cells = to;
            flight.fx -= sx as f32;
            flight.fy -= sy as f32;
            moves += 1;
            continue;
        }
        // The diagonal was refused. Take whichever axis is still free and
        // give up the one that is not -- a body clipping a corner should
        // slide along it, not stop dead in the air.
        if sy != 0 {
            if let Some(to) = translated_if_free(world, &cells, 0, sy) {
                relocate_chain(world, organism, &cells, &to);
                cells = to;
                flight.fy -= sy as f32;
                moves += 1;
                // Hit a wall sideways: the vertical half of the flight
                // continues, the horizontal half is over.
                flight.vx = 0.0;
                flight.fx = 0.0;
                continue;
            }
        }
        if sx != 0 {
            if let Some(to) = translated_if_free(world, &cells, sx, 0) {
                relocate_chain(world, organism, &cells, &to);
                cells = to;
                flight.fx -= sx as f32;
                moves += 1;
                flight.vy = 0.0;
                flight.fy = 0.0;
                // Blocked below while descending is the definition of
                // arriving.
                landed = sy > 0;
                if landed {
                    break;
                }
                continue;
            }
        }
        // Nowhere to go on either axis.
        if sy > 0 {
            landed = true;
        } else {
            // A ceiling, or a wall taken head-on while rising. Stop and let
            // gravity have it next frame.
            flight.vy = 0.0;
            flight.fy = 0.0;
            flight.vx = 0.0;
            flight.fx = 0.0;
        }
        break;
    }

    world.creature_stats.flight_frames += 1;
    world.creature_stats.flight_moves += moves;

    let (hx, hy) = cells.first().copied().unwrap_or((0, 0));
    // **`since_nest` counts ticks, and a flight frame is not a tick.**
    // Advancing it once per airborne frame would age a hopping creature's
    // nest memory six times faster than a walking one's, purely because the
    // scheduler visits it more often -- and `since_nest` scales the
    // channel-A deposit, so a hop would quietly cost trail strength that
    // nothing in the design asks it to cost. One increment per
    // `tick_interval` frames keeps the *rate* identical, the same
    // pro-rating the idle cost below uses and for the same reason.
    let frame = world.frame;
    let interval = def.tick_interval.max(1);
    if let Some(state) = world.organism_mut(organism) {
        state.flight = if landed { None } else { Some(flight) };
        // `is_multiple_of` rather than `% == 0`: `clippy::manual_is_multiple_of`
        // is a 1.98 lint and the container ships 1.94.1, so the local gate was
        // green and CI was red -- the exact drift `CLAUDE.md` records, caught
        // by a red PR rather than by the check meant to prevent it.
        if frame.is_multiple_of(interval) {
            state.since_nest = state.since_nest.saturating_add(1);
        }
        // **The excursion depth has to see a hop, or the foraging-range
        // instrument understates exactly the creature it was built to
        // measure.** `forage_max` is measurement-only (see its own doc), and
        // a counter that cannot move for the mechanism under test is the
        // failure `CLAUDE.md` names -- a hopper crossing 160 cells would
        // have registered a range of zero.
        let (ax, ay) = state.forage_anchor;
        let depth = (hx - ax).abs().max((hy - ay).abs());
        state.forage_max = state.forage_max.max(depth.clamp(0, u16::MAX as i32) as u16);
    }

    // **Metabolism per unit *time*, not per tick.** An airborne creature is
    // rescheduled `tick_interval` times more often than a walking one, so
    // charging the full `idle_cost` on each of those frames would tax
    // hanging in the air six times harder than standing still -- a cost that
    // came from the scheduler rather than from the design. Pro-rating keeps
    // the *rate* identical and leaves `LAUNCH_COST_IN_MOVES` as the only
    // thing the verb actually charges for.
    let idle = def.idle_cost_per_cell * live_body_cells(world, organism, def) / def.tick_interval.max(1) as f32;
    world.energy_ledger.metabolized += idle as f64;
    if landed {
        // Back on the normal schedule, and back to deciding things.
        return apply_creature_energy(world, hx, hy, organism, -idle, def);
    }
    let Some(state) = world.organism_mut(organism) else {
        return Vec::new();
    };
    state.energy -= idle;
    if state.energy <= 0.0 {
        creature_dies(world, organism);
        return Vec::new();
    }
    vec![ActiveSite { x: hx, y: hy, kind: ActiveKind::Creature { organism }, next_frame: world.frame + 1 }]
}

/// One cell of travel in the direction an accumulator has banked, or 0.
fn axis_step(f: f32) -> i32 {
    if f >= 1.0 {
        1
    } else if f <= -1.0 {
        -1
    } else {
        0
    }
}

/// Pick a new heading at random from the directions that actually lead
/// somewhere — **the tumble**.
///
/// Named for what it is: the second half of run-and-tumble chemotaxis. A
/// creature that cannot steer toward a gradient can still *follow* one, by
/// running while things improve and re-orienting at random when they do
/// not. Bacteria solve exactly this problem exactly this way, and it is the
/// only mechanism available to something whose lateral sensors read zero
/// (see `brain::BrainInput::PheroAAlong`).
///
/// Uniform among the **viable** directions, not among all eight. On flat
/// ground three of the eight point upward into open air, so a uniform
/// re-roll lands straight back in the blocked state better than a third of
/// the time — measured at 29,344 blocked ticks against 41,843 moves before
/// this was narrowed.
fn tumble(world: &mut World, organism: u16, def: &CreatureDef, draw: &mut rng::Rng) {
    let Some((hx, hy)) = world.organism(organism).and_then(|s| s.chain.first().copied()) else {
        return;
    };
    let chain = world.organism(organism).map(|s| s.chain.clone()).unwrap_or_default();
    let viable: Vec<u8> = (0..8u8)
        .filter(|&d| {
            let (dx, dy) = DIRS[d as usize];
            let (tx, ty) = (hx + dx, hy + dy);
            // Body-aware, like the candidate scan: a wide creature must not
            // re-orient into a heading its shape cannot occupy.
            let landing = body_after_step(def, &chain, (tx, ty), d, d);
            landing.iter().all(|&p| world.is_empty(p.0, p.1) || chain.contains(&p)) && body_has_foothold(world, def, &landing, (tx, ty), kin_footing(world, organism, def))
        })
        .collect();
    if let Some(state) = world.organism_mut(organism) {
        state.heading = if viable.is_empty() { draw.below(8) as u8 } else { viable[draw.below(viable.len() as u32) as usize] };
    }
    world.creature_stats.tumbles += 1;
}

/// Where this creature's cells end up if its head steps to `head`.
///
/// The two body plans differ **only here**, which is the whole reason
/// `BodyPlan` is worth having: a chain's body follows into the cells the
/// head vacated, a rigid body's translates with it. Everything downstream —
/// passability, footing, the relocation itself — is written once against
/// the resulting position list.
fn body_after_step(def: &CreatureDef, chain: &[(i32, i32)], head: (i32, i32), from: u8, to: u8) -> Vec<(i32, i32)> {
    if def.body.is_rigid() {
        // Facing is a *mirror*, never a rotation (see `BodyPlan`). Turning
        // between east-ish and west-ish re-lays the template; turning
        // within one side leaves the shape alone.
        let _ = from;
        let west = (3..=5).contains(&to);
        def.body.offsets(west).iter().map(|&(dx, dy)| (head.0 + dx, head.1 + dy)).collect()
    } else {
        let mut next = Vec::with_capacity(chain.len());
        next.push(head);
        next.extend(chain.iter().take(chain.len().saturating_sub(1)).copied());
        next
    }
}

/// Does the body have a foothold where it is going?
///
/// A chain asks about its **head** and a rigid body about **any of its
/// cells**, and that difference is not an inconsistency. A chain can
/// stretch — the tail stays grounded while the head leads into open air,
/// which is exactly how it came to log 33,881 falls before the head-only
/// rule went in. A rigid body cannot stretch: if any cell of it is near
/// ground, all of it is.
fn body_has_foothold(world: &World, def: &CreatureDef, landing: &[(i32, i32)], head: (i32, i32), kin: Option<Kin>) -> bool {
    if def.body.is_rigid() {
        landing.iter().any(|&p| head_has_foothold(world, p, kin))
    } else {
        head_has_foothold(world, head, kin)
    }
}

/// The kin-footing licence for `organism`, or `None` if its species does
/// not climb over its own kind.
///
/// Resolved once per step at the site that already holds the organism,
/// rather than per candidate cell — `head_has_foothold` runs over eight
/// neighbours for each of three candidates, and a species lookup inside
/// that loop would be twenty-four of them for a `bool` that cannot change.
fn kin_footing(world: &World, organism: u16, def: &CreatureDef) -> Option<Kin> {
    if !def.climbs_over_kin {
        return None;
    }
    Some(Kin { organism, species: world.organism(organism)?.species })
}

/// Is there anything for the head to hold on to at `(x, y)`?
///
/// **This is a different question from "am I held up", and conflating them
/// cost two wrong fixes.** `CLAUDE.md` asks which object a rule evaluates;
/// the honest answer here is that there are two rules and two objects:
///
/// * *"May I step there?"* is about the **head**, and is this function. A
///   walking animal puts its feet on the surface.
/// * *"Am I still standing?"* is about the **whole piece** (P-25), and is
///   the support test in `step_chain`. A chain is held up if any of its
///   cells touches something, or a bridge-walking ant comes apart the way
///   the per-cell bearing rule took slabs apart.
///
/// Asking the second question about a prospective step is what produced
/// the measured mess: the body was still on the ground, so leading the
/// head up into open air *passed*, and the chain then fell one tick later
/// once the tail had followed. Falls ran at 59% of all moves, and the
/// obvious repair — leaning harder on the same predicate — pushed it to
/// 80% while cutting deliveries from 90 to 4, because the bonus had to be
/// big enough to swamp the steering before it changed anything.
///
/// Two fixes failing the same way meant the predicate was wrong, not the
/// tuning.
fn head_has_foothold(world: &World, (x, y): (i32, i32), kin: Option<Kin>) -> bool {
    NEIGHBOURS_8.iter().any(|&(dx, dy)| {
        let (nx, ny) = (x + dx, y + dy);
        // **The edge of the world is not scenery.** `World::get` returns a
        // `BEDROCK` sentinel outside the bounds, deliberately, so material
        // treats the edge as a wall and does not fall out of it. For a
        // *climbing* creature that turns the invisible boundary into an
        // infinitely tall ladder: the creature probe found ants parked at
        // x = 0 heading permanently north, a slice of the colony walking
        // up the side of the world forever. Correct for sand, wrong for
        // anything with legs.
        if !world.in_bounds(nx, ny) {
            return false;
        }
        let cell = world.get(nx, ny);
        matches!(world.materials.kind(cell.material), MaterialKind::Solid | MaterialKind::Powder | MaterialKind::Plant)
            || kin.is_some_and(|k| k.is_walkable_nestmate(world, cell))
    })
}

/// **Who counts as walkable ground**, for a species that walks over its own
/// kind — WP-9 arm 1, and the deliberate re-test of dead ends 775/829,
/// whose condition line says in as many words: *"re-test if creatures gain
/// pass-through or climb-over."*
///
/// It grants **footing only, never passability.** A creature cell stays
/// something you cannot *enter*; this makes it something you can stand
/// *on*. Two multi-cell chains swapping through each other is a different
/// and much harder change, and nothing here approaches it.
///
/// **`self` is excluded, and that exclusion is the whole safety of it.** A
/// chain's own tail is permanently inside its head's 8-neighbourhood, so
/// counting own cells would make every ant its own foothold — in mid-air,
/// forever. That is the same failure as counting the out-of-bounds
/// `BEDROCK` sentinel above, which turned the world edge into an
/// infinitely tall ladder, and it would be worse: the ladder would follow
/// the animal.
#[derive(Clone, Copy)]
struct Kin {
    organism: u16,
    species: SpeciesId,
}

impl Kin {
    fn is_walkable_nestmate(self, world: &World, cell: Cell) -> bool {
        let other = cell.organism_id();
        other != 0 && other != self.organism && world.organism(other).is_some_and(|s| s.species == self.species)
    }
}

/// Rewrite a chain from `from` to `to`, carrying every whole `Cell`.
///
/// **Clear-then-write, in two passes.** The two position sets overlap by
/// construction (a body follows its own head), so writing in place would
/// need an order argument that is different for every chain length and
/// silently wrong when a creature steps into its own tail. Clearing first
/// costs a few extra `World::set` calls at chain lengths of two or three
/// and removes the whole class of question.
///
/// P-1: the `Cell` values are moved, not rebuilt, so temperature,
/// `FLAG_BURNING` and the burn timer ride along for every cell. A chain is
/// where that matters most — a rebuild forgets once per cell per step.
fn relocate_chain(world: &mut World, organism: u16, from: &[(i32, i32)], to: &[(i32, i32)]) {
    let cells: Vec<Cell> = from.iter().map(|&(cx, cy)| world.get(cx, cy)).collect();
    for &(cx, cy) in from {
        world.set(cx, cy, Cell::EMPTY);
    }
    for (&(cx, cy), &cell) in to.iter().zip(&cells) {
        world.set(cx, cy, cell);
    }
    if let Some(state) = world.organism_mut(organism) {
        state.chain = to.to_vec();
    }
}

/// Charge energy, reschedule or die. The chain-creature counterpart of
/// `apply_energy_delta`.
fn apply_creature_energy(world: &mut World, x: i32, y: i32, organism: u16, delta: f32, def: &CreatureDef) -> Vec<ActiveSite> {
    let Some(state) = world.organism_mut(organism) else {
        return Vec::new();
    };
    state.energy += delta;
    let energy = state.energy;
    if energy <= 0.0 {
        creature_dies(world, organism);
        return Vec::new();
    }
    vec![ActiveSite { x, y, kind: ActiveKind::Creature { organism }, next_frame: world.creature_due(def.tick_interval) }]
}

/// Every cell of the chain becomes `corpse`, and the slot comes back.
///
/// **A dead ant is matter** — food for the next one, fuel for a fire — and
/// that costs no code at all because the material system already does it.
/// It is also what closes the colony's loop: the energy a forager spent
/// getting somewhere it could not return from is not deleted, it is left
/// lying there as something edible.
/// A reference mouthful, for normalising the food-value overlay's ramp.
///
/// A *fixed* scale rather than the frame's maximum, so two contact sheets
/// taken at different times are comparable — an overlay that renormalises
/// itself answers "which cell here is worth most", which is not the question
/// anyone has when they switch it on.
pub const REFERENCE_MOUTHFUL: f32 = 1200.0;

/// **What one cell is worth to eat, and the only definition of it.**
///
/// The overlay, the probe and (S3b) the eat verb all call this, so a picture
/// of the food in a world cannot disagree with what an animal gets for
/// biting it. `CLAUDE.md`'s canopy-density failure is the cautionary case in
/// the other direction: a readout derived separately from the mechanism it
/// describes is a readout that can be wrong on its own.
///
/// Zero means "not food". A material whose worth varies per cell says so
/// with `worth_in_aux` and carries it there; everything else is worth what
/// its `.ron` says.
pub fn food_value(world: &World, cell: Cell) -> f32 {
    let m = world.materials.get(cell.material);
    // **`worth_in_aux` means "prefer the stamp", not "ignore the material".**
    // The first version short-circuited on the flag, and `corpse.ron`'s own
    // comment described a fallback that therefore did not exist -- while
    // asserting there was "no such path today" for a corpse to arrive
    // unstamped. There is: `ant.ron` sets `burns_into: "corpse"`, and
    // `fire.rs`'s burnout writes `Cell::new(into, shade)` with **`aux` 0**,
    // because it is generic over every flammable material and knows nothing
    // about creatures. So an ant that burned to death left meat worth
    // exactly nothing -- while `wiki/ants.md` promises in as many words that
    // "ants that die in a fire become the next colony's dinner".
    if m.worth_in_aux && cell.aux() != 0 {
        cell.aux() as f32
    } else {
        m.food_energy
    }
}

/// The energy standing in `area` as meat — cells that carry their own worth
/// in `Cell::aux`.
///
/// Takes a rectangle rather than sweeping the world, deliberately: this is a
/// census, it is O(area), and hiding that behind a no-argument method is how
/// a debug readout ends up in a hot path.
///
/// **The bound this feeds is an upper bound and there is one known slack in
/// it.** Flesh bitten off a *living* animal is priced by its material
/// (`ant.ron`'s `food_energy`), so it books to `harvested_plant` as though
/// it were free, while the stamp that body was granted at spawn stays in
/// `stamped` and never becomes standing meat. The two happen to cancel
/// today — `ant.ron` sets `food_energy: 120.0` against `body_energy: 120.0`,
/// and that equality is load-bearing, not a coincidence — but the ledger
/// does not *know* they cancel. Predation therefore loosens
/// `max_standing_meat`, and the sealed-box guards below are written on an
/// ant-only colony, where ants do not eat ants, so the bound is tight.
/// Closing it properly means a sink for a stamp destroyed without becoming
/// a corpse; that belongs with S6, when a parent starts paying stamps.
pub fn standing_meat(world: &World, area: Rect) -> f64 {
    let mut total = 0.0;
    for x in area.min_x..=area.max_x {
        for y in area.min_y..=area.max_y {
            let cell = world.get(x, y);
            // Through `food_value`, not off `aux` directly: a corpse that
            // arrived by fire carries no stamp and is priced by the
            // material fallback, and a census that read the raw `aux` would
            // report 120 of real standing food as 0 -- a readout disagreeing
            // with the mechanism it describes, in the direction that hides
            // food rather than inventing it.
            if world.materials.get(cell.material).worth_in_aux {
                total += food_value(world, cell) as f64;
            }
        }
    }
    total
}

/// The same stock, in transit. A carrier holding a corpse cell is holding
/// meat that is not in any cell of the world, and leaving it out is how a
/// census reports a delivery run as an energy leak.
pub fn carried_meat(world: &World) -> f64 {
    world
        .live_organism_ids()
        .into_iter()
        .filter_map(|id| world.organism(id).and_then(|s| s.carrying))
        .filter(|held| world.materials.get(held.material).worth_in_aux)
        .map(|held| held.worth as f64)
        .sum()
}

/// Every joule the creature economy is holding: what is inside animals,
/// what is standing as meat, what is in transit, and what is still *promised*
/// as meat by bodies that have not died yet.
///
/// **The last term is the one that makes this monotone, and leaving it out
/// is a metric that reports a death as energy creation.** A body's stamp is
/// booked into `EnergyLedger::stamped` at spawn but does not become standing
/// meat until `creature_dies` writes it, so `live + standing` jumps upward
/// by `body_energy * cells` every time something dies -- which is exactly
/// backwards, and would have read as a pump in the guard written to find
/// pumps. `CLAUDE.md`: ask what a metric counts when nothing is wrong.
///
/// In a world with no producers and nothing eating live flesh, this may only
/// fall. That is the P-20 property stated as a number:
/// `a_sealed_colony_never_grows_its_own_biomass` asserts it.
pub fn creature_biomass(world: &World, area: Rect) -> f64 {
    let promised: f64 = world
        .live_organism_ids()
        .into_iter()
        .filter_map(|id| world.organism(id))
        .filter_map(|s| {
            world.species.get(s.species).creature.as_ref().map(|d| d.body_energy as f64 * s.cells.len() as f64)
        })
        .sum();
    world.live_creature_energy() + standing_meat(world, area) + carried_meat(world) + promised
}

/// Round an energy worth into the `u16` `Cell::aux` carries it in.
///
/// 1:1, not through a quantum constant: a `u16` covers every budget in the
/// engine (the beetle's 1600 is the largest) and a scale factor nobody can
/// tune in either direction is exactly the counterweight shape `CLAUDE.md`
/// warns about. Saturating rather than wrapping, because the failure mode of
/// a wrap here is meat worth 5 instead of 65,541.
fn quantise_worth(worth: f32) -> u16 {
    worth.round().clamp(0.0, u16::MAX as f32) as u16
}

impl Carried {
    /// Put a carried mouthful back into the world without losing what it is
    /// worth.
    ///
    /// **The `aux` write is gated on the material, not on the payload.**
    /// `Cell::aux` is a tagged union with three conventions in it now
    /// (liquid fill, soil water, food worth), and the drop path is the one
    /// place a value crosses from one material to another. Writing the
    /// worth unconditionally would put 120 into a leaf's `aux`, which on a
    /// `Powder` reads as soil water — manufacturing water out of food, the
    /// exact shape of the mistake `Cell::aux`'s own doc comment warns
    /// about twice.
    fn into_cell(self, world: &World) -> Cell {
        let cell = Cell::new(self.material, self.shade);
        if world.materials.get(self.material).worth_in_aux {
            cell.with_aux(self.worth)
        } else {
            cell
        }
    }
}

/// A species' `start_energy`, for scaling a corpse's shade ramp. Zero if the
/// organism is gone, which only happens on a path that is not writing meat.
fn def_start_energy(world: &World, organism: u16) -> f32 {
    world
        .organism(organism)
        .and_then(|s| world.species.get(s.species).creature.as_ref().map(|d| d.start_energy))
        .unwrap_or(0.0)
}

/// Kill whatever creature owns the cell at `(x, y)`, if any. Returns
/// whether anything died.
///
/// **The gnome's axe, on the engine's own death path rather than beside
/// it.** An erase would have been two lines and wrong in three ways that
/// `creature_dies` already gets right: the animal leaves meat that a
/// scavenger can find, the energy it was holding is booked as stored
/// rather than vanishing out of `energy_ledger`, and the organism slot is
/// freed so the roster does not leak. A creature-specific "killed by a
/// player" path would have had to reproduce all three and would drift.
///
/// Multi-cell animals die whole: one blow anywhere on an ant kills the
/// ant, not the segment. That is the same rule predation already uses and
/// it is the only one that makes sense for a blade — there is no partial
/// creature in this engine to leave behind.
pub fn slay(world: &mut World, x: i32, y: i32) -> bool {
    if !world.in_bounds(x, y) {
        return false;
    }
    let cell = world.get(x, y);
    if !matches!(world.materials.kind(cell.material), MaterialKind::Creature) {
        return false;
    }
    let organism = cell.organism_id();
    if organism == 0 || world.organism(organism).is_none() {
        return false;
    }
    creature_dies(world, organism);
    true
}

fn creature_dies(world: &mut World, organism: u16) {
    let held = world.organism(organism).and_then(|s| s.carrying);
    // **The bank, and what of it was actually there.** These differ: a
    // creature is declared dead at or below zero, and the tick that killed
    // it debited a full charge against whatever was left, so `bank` is
    // routinely a small negative. Only the non-negative part can become
    // meat -- there is no such thing as a corpse worth minus two -- and the
    // shortfall is booked as `overdrawn` so the live identity still closes.
    let bank = world.organism(organism).map_or(0.0, |s| s.energy);
    let leftover = bank.max(0.0);
    world.energy_ledger.overdrawn += (leftover - bank) as f64;
    // **Only the cells it still owns, and this is a matter-conservation
    // bug that read as a feature.** `chain` is a separate sequence from
    // `cells` and is *stale* on the predation path: `act` empties the
    // bitten cell before calling `reconcile_chain`, so a corpse written
    // over the whole chain resurrects the mouthful the predator had just
    // swallowed. Measured on a two-cell ant whose head was eaten: two
    // corpse cells out of two, one of them conjured -- and since "corpse"
    // is on the beetle's own food list, it could then eat the same matter
    // a second time. The predation test missed it because it only asserted
    // that no *ant* cell was left standing; corpse was what to look for.
    //
    // Filtering by `cells` rather than trusting `chain` is also the
    // general statement: anything that removes a cell (a bite, a fire, an
    // explosion, the brush) goes through the `World::set` seam and shows up
    // here, and none of those should leave meat behind either.
    let owned = world.organism(organism).map(|s| s.cells.clone()).unwrap_or_default();
    let chain_before = world.organism(organism).map_or(0, |s| s.chain.len());
    let chain: Vec<(i32, i32)> = world
        .organism(organism)
        .map(|s| s.chain.iter().copied().filter(|p| owned.contains_key(p)).collect())
        .unwrap_or_default();
    // **What the meat is worth, written into the meat.** The structural
    // stamp the body was granted at spawn, plus whatever the animal had left
    // to spend, divided over the cells that are actually still standing --
    // a half-eaten animal leaves half the meat, because the cells the
    // predator already swallowed are not in `chain` any more.
    //
    // Leftover matters as well as the stamp: an animal killed in its prime
    // is worth more than one that starved, which is a real distinction and
    // is visible on screen through the shade below. And it is the *stamp*
    // that keeps a starved animal -- dead at exactly 0 -- worth eating at
    // all, which is what stops closing §13l's pump from also deleting the
    // scavenger niche.
    //
    // `aux` in energy units, 1:1, rather than through a quantum constant:
    // a u16 covers every budget in the engine (the beetle's 1600 is the
    // largest) and a scale factor nobody can tune in either direction is
    // exactly the counterweight shape `CLAUDE.md` warns about.
    let body_energy = world
        .organism(organism)
        .and_then(|s| world.species.get(s.species).creature.as_ref().map(|d| d.body_energy))
        .unwrap_or(0.0);
    // **The other half of the living-flesh seam** — see `reconcile_chain`.
    // A creature can arrive here holding a chain longer than the cells it
    // still owns: the predation path empties the bitten cell and calls
    // `reconcile_chain`, which routes a *lost head* straight to this
    // function without going through its own shortening branch. Those
    // cells' stamps never become corpse either, and they are booked in
    // exactly the same account. Reads 0 for a starved animal, which is
    // every death in a colony scene with nothing biting anything.
    world.energy_ledger.meat_lost += body_energy as f64 * (chain_before - chain.len()) as f64;
    if let Some(corpse_id) = world.materials.id_of("corpse") {
        let cells = chain.len().max(1) as f32;
        let worth = (body_energy * cells + leftover) / cells;
        let aux = worth.round().clamp(0.0, u16::MAX as f32) as u16;
        // **Shade from worth, not from noise.** A fat corpse and a
        // picked-over one have to look different, or the only legible
        // feedback for a mechanic about how much food is where is a number
        // in a debug overlay. The ramp is over the body's own stamp, so a
        // starved animal is the dark end and one killed full is the bright
        // end.
        //
        // **That sentence was false when it was written, and the palette had
        // to be reordered to make it true.** `corpse.ron` listed its three
        // browns mid, dark, light, so ramping worth over the index made a
        // corpse worth 760 render *darker* than a starved one worth 120 --
        // non-monotone, in a mechanic whose entire visible output is this
        // one byte. Two lessons, both already in `CLAUDE.md` and both missed
        // here: a debug readout must not be derived separately from the
        // thing it describes, and an image cannot tell you *how much* -- the
        // span is still only 84..104 in red, which is the same order of
        // difference as the canopy-density sheet that read as blank.
        // `OrganismOverlay::FoodValue` is the readout that can answer "how
        // much"; this byte is only meant to say "not all the same".
        let shades = world.materials.get(corpse_id).palette.len().max(1) as u32;
        let full = (body_energy + def_start_energy(world, organism)).max(1.0);
        let shade = ((worth / full).clamp(0.0, 1.0) * (shades - 1) as f32).round() as u8;
        for &(cx, cy) in chain.iter() {
            let temp = world.get(cx, cy).temperature();
            world.set(cx, cy, Cell::new(corpse_id, shade).with_temperature(temp).with_aux(aux));
        }
        // **A transfer, not a write-off**, and the account name is the
        // whole difference. `died_holding` said this energy was destroyed;
        // it is standing in the world as meat, and the census that says so
        // is what makes `harvested_corpse` a matched term instead of the
        // free one §13l's pump ran on. Booked as the *quantised* total the
        // cells actually carry, not as `leftover` -- rounding to the u16 is
        // a real (sub-joule) loss and the identity has to see it.
        let meat_written = aux as f64 * chain.len() as f64;
        let from_stamp = body_energy as f64 * chain.len() as f64;
        let from_live = (meat_written - from_stamp).clamp(0.0, leftover as f64);
        world.energy_ledger.stored_in_meat += from_live;
        world.energy_ledger.dissipated += leftover as f64 - from_live;
    } else {
        // No `corpse` material compiled in: there is nowhere to put the
        // remainder, so it is genuinely gone. Reads 0 in every real scene.
        world.energy_ledger.dissipated += leftover as f64;
    }
    // Whatever it was carrying falls where it fell. Losing it would be a
    // silent material sink, and the census is about to care.
    //
    // **Into a neighbour, not into the body cell**, which is the bug the
    // sentence above was written to prevent and did not: the loop over the
    // chain has just written corpse into every one of those cells, so
    // `is_empty` on `chain.last()` was false every time a corpse material
    // existed and the cargo was dropped on the floor of the accounting.
    // Same rule the drop verb uses -- first empty cell in the
    // 8-neighbourhood -- so a carrier that dies leaves its load beside its
    // body exactly as one that dropped it would.
    if let (Some(held), Some(&(cx, cy))) = (held, chain.last()) {
        if let Some((dx, dy)) = NEIGHBOURS_8.iter().map(|&(dx, dy)| (cx + dx, cy + dy)).find(|&(px, py)| world.is_empty(px, py)) {
            world.set(dx, dy, held.into_cell(world));
        }
    }
    world.creature_stats.deaths += 1;
    world.free_organism(organism);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::field;
    use crate::sim::organism;
    use crate::sim::scheduler;
    use crate::sim::update;

    fn test_world() -> World {
        World::new(Rect::new(0, 0, 199, 199))
    }

    /// A bed with the three obstructions that broke colony founding, laid
    /// out left to right: a plateau standing **above** the cursor row, a
    /// stretch of open **water**, and flat ground under a **plant canopy**.
    /// Ground is `SOIL` throughout; the cursor row sits at the low ground's
    /// surface, which is what `filmstrip`'s colony scene passes.
    ///
    /// Written to fail against the old rules, and checked doing so: with
    /// `colony_surface` searching downward from the cursor and
    /// `colony_ant_site` not refusing `Liquid`, the water band places ants
    /// on the lake and the plateau places none at all.
    fn colony_bed() -> (World, i32) {
        let mut w = World::new(Rect::new(0, 0, 255, 199));
        const LOW: i32 = 120;
        const HIGH: i32 = 110;
        let leaf = w.materials.id_of("leaf").expect("leaf material");
        let soil = w.materials.id_of("soil").expect("soil material");
        for x in 0..=255 {
            let top = if (10..70).contains(&x) { HIGH } else { LOW };
            for y in top..=140 {
                w.set(x, y, Cell::new(soil, 0));
            }
        }
        // Open water, its surface level with the low ground.
        for x in 90..150 {
            for y in LOW..=140 {
                w.set(x, y, Cell::EMPTY);
            }
            for y in LOW..=140 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        // A canopy floating over the right-hand ground: the cells a
        // downward scan from the sky would stop on.
        for x in 170..240 {
            for y in (LOW - 30)..(LOW - 26) {
                w.set(x, y, Cell::new(leaf, 0));
            }
        }
        (w, LOW)
    }

    /// `open-bugs-handoff.md` §R2's placement half. The nest-painting loop
    /// refused water from the day it was written; the ant loop six lines
    /// below it did not, and an ant put on water never falls off.
    #[test]
    fn found_colony_never_puts_an_ant_on_water() {
        let (mut w, low) = colony_bed();
        let placed = w.found_colony(128, low - 2);
        assert!(placed > 0, "the bed placed nothing at all -- the scene is wrong, not the rule");
        // **The property is "no ant stands on nothing", not "no creature
        // cell is ever over water".** An ant is a two-cell chain, so one
        // founded on the bank may legitimately overhang the shore by its
        // tail -- asserting against that would fail for a correct ant and
        // tell us nothing about the bug. What §R2 is actually about is an
        // ant with *no* footing at all, strung out across open water at
        // exactly `COLONY_ANT_SPACING`, which is what this checks: every
        // organism must have at least one of its own cells resting on
        // ground.
        let mut footing: std::collections::BTreeMap<u16, (bool, (i32, i32))> = std::collections::BTreeMap::new();
        for x in 0..=255 {
            for y in 0..=199 {
                let c = w.get(x, y);
                if !matches!(w.materials.kind(c.material), MaterialKind::Creature) {
                    continue;
                }
                let id = c.organism_id();
                let grounded = matches!(w.materials.kind(w.get(x, y + 1).material), MaterialKind::Solid | MaterialKind::Powder);
                let e = footing.entry(id).or_insert((false, (x, y)));
                e.0 |= grounded;
            }
        }
        let afloat: Vec<_> = footing.iter().filter(|(_, (g, _))| !g).map(|(id, (_, at))| (*id, *at)).collect();
        assert!(afloat.is_empty(), "{} ants with no cell resting on ground: {:?}", afloat.len(), &afloat[..afloat.len().min(8)]);
    }

    /// The canopy half of `open-bugs-handoff.md` §R. A downward scan from
    /// the sky stops on a leaf, so a column with a tree over it read as
    /// having no ground -- 217 of 308 columns on the colony scene's own
    /// default seed, which is why it panicked instead of degrading.
    #[test]
    fn colony_surface_looks_through_a_canopy_to_the_ground() {
        let (w, low) = colony_bed();
        let under_canopy = 200;
        assert!(
            matches!(w.materials.kind(w.get(under_canopy, low - 30).material), MaterialKind::Plant),
            "the bed does not actually have a canopy over x={under_canopy}"
        );
        // **From row 0, which is the call the scene actually makes.** An
        // earlier version of this guard asked from `low - 2` -- *below* the
        // canopy -- so the scan never met a leaf and the test passed with
        // the fault put back. It was blind, not weak. `filmstrip`'s colony
        // scene chooses its site with `colony_ant_site(w, x, 0)`, scanning
        // from the sky, and that is the path the canopy breaks.
        assert_eq!(
            colony_ant_site(&w, under_canopy, 0),
            Some(low),
            "scanning from above, a column under a canopy must still offer the ground beneath it as a site"
        );
    }

    /// One cursor row cannot describe 204 cells of terrain. Searching
    /// downward from it meant any column standing higher than the cursor
    /// began the search already inside the hill and found its interior --
    /// 18, 33 and 20 of 52 sites on the colony scene's seeds 1, 2 and 7.
    #[test]
    fn colony_surface_rises_out_of_ground_that_stands_above_the_cursor() {
        let (w, low) = colony_bed();
        let on_plateau = 40;
        assert_eq!(
            colony_ant_site(&w, on_plateau, low - 2),
            Some(110),
            "a column whose ground stands above the cursor must be found at its own surface, not inside the hill"
        );
    }

    /// The `Y` key's documented behaviour, which the rule above must not
    /// break: founding a colony from inside a cave lands it on the cave
    /// floor, never on the ground overhead.
    #[test]
    fn colony_surface_in_a_cave_still_finds_the_cave_floor() {
        let mut w = World::new(Rect::new(0, 0, 255, 199));
        let soil = w.materials.id_of("soil").expect("soil material");
        for x in 0..=255 {
            for y in 100..=180 {
                w.set(x, y, Cell::new(soil, 0));
            }
        }
        for x in 40..80 {
            for y in 130..=150 {
                w.set(x, y, Cell::EMPTY);
            }
        }
        assert_eq!(
            colony_ant_site(&w, 60, 135),
            Some(151),
            "a cursor inside a cave must found on the cave floor, not on the surface above it"
        );
    }

    fn run(w: &mut World, frames: usize) {
        for _ in 0..frames {
            w.begin_step();
            scheduler::step(w);
            w.end_step();
        }
    }

    /// **A measurement, not a guard** (hence `#[ignore]`): can a
    /// Jones-style gradient follower track a one-cell trail carried on a
    /// `FIELD_SCALE`-resolution channel?
    ///
    /// `Reports/stigmergy-research.md` §6 sets a hard floor — gradient
    /// following needs a sensor offset that actually *resolves* differences
    /// — and `Reports/creature-direction.md` §2a asks for this to be
    /// answered before the pheromone substrate is designed, because
    /// discovering it afterwards means debugging a milling colony with
    /// nothing to say whether the fault is the brain, the deposit or the
    /// grid.
    ///
    /// The stand-in channel is **moisture**, deliberately: adding a field
    /// channel for a throwaway experiment would be the more expensive way
    /// to learn the same thing. A row of `WATER` makes
    /// `apply_moisture_sources` pin every 8x8 block containing liquid to
    /// `MAX_MOISTURE`, so a one-cell trail reads back as an eight-cell-tall
    /// saturated block-row with a real gradient only at its edges. That
    /// smearing *is* the thing under test.
    ///
    /// **Four numbers, because "stays near the trail" alone cannot tell the
    /// answers apart** — `CLAUDE.md`'s "ask what a metric counts when
    /// nothing is wrong", which this experiment walked straight into on its
    /// first run (0.988 within 2 cells, and a follower that had advanced 21
    /// cells along a 208-cell trail in 400 steps: pinned, not commuting).
    ///
    /// 1. **within 2 cells** — the tracking question as asked.
    /// 2. **within 4 cells** — anywhere inside the smeared 8-cell block. A
    ///    follower that merely drifts inside the smear looks identical to
    ///    one tracking the trail at the zoom (1) is read at.
    /// 3. **along-trail progress** — net displacement over path length. A
    ///    follower oscillating on the spot scores ~1.0 on (1) while going
    ///    nowhere, which is not what "follows a trail" has to mean for an
    ///    ant that must actually commute.
    /// 4. **the no-trail control** — the same follower on a world with no
    ///    water in it. If the control also scores well, (1) is measuring
    ///    the starting position, not the mechanism.
    ///
    /// The **separation** probe is separate and needs no follower at all:
    /// paint one trail, then two trails four cells apart, and compare the
    /// moisture profiles they produce. If the two are indistinguishable the
    /// question is settled regardless of how well anything tracks, because
    /// two ant routes four cells apart is the ordinary case, not the edge
    /// one.
    ///
    /// **Decision rule.** The default plan (`creature-direction.md` §5a) is
    /// a dedicated CA-resolution pheromone plane per channel. Only robust
    /// tracking **and** surviving separation justifies the cheaper sixth
    /// `FieldCell` channel instead. Run it with:
    ///
    /// `cargo test --lib pheromone_resolution_experiment -- --ignored --nocapture`
    ///
    /// ## Measured 2026-08-17 — the answer is the CA-resolution plane
    ///
    /// | | one trail | no-trail control |
    /// |---|---|---|
    /// | within 2 cells | **0.988** | 0.023 |
    /// | within 4 cells | 0.993 | 0.032 |
    /// | along-trail progress | **0.052** | 0.262 |
    ///
    /// Separation: largest difference between the one-trail and the
    /// two-trails-four-cells-apart moisture profile, over the 17 rows
    /// spanning the trail, was **0.0000** — bit-identical, every row.
    ///
    /// Three readings, and the second and third are what decide it:
    ///
    /// - **Gradient following at offset 8 does work**, in the only sense
    ///   the first row measures: the follower finds the wet band and stays
    ///   in it, 43x the control rate. Taken alone this is the "surprise,
    ///   reconsider" result §2a anticipated.
    /// - **It cannot commute along the trail.** Along-trail progress is
    ///   0.052 against the random walk's 0.262 — the follower makes
    ///   headway *five times worse than chance*, because it is pinned
    ///   oscillating at the block edge (21 cells advanced along a 208-cell
    ///   trail in 400 steps). An ant that cannot travel a trail has not
    ///   followed it, whatever the proximity number says. Note also that
    ///   the sensed maximum sits at y = 96, four cells off the trail it
    ///   came from: the smear does not even peak where the signal is.
    /// - **Separation is not marginal, it is zero.** Two trails four cells
    ///   apart are not merely hard to tell apart at `FIELD_SCALE = 8`;
    ///   they produce the identical field, so no sensor offset, no
    ///   interpolation and no brain can ever distinguish them. Two routes
    ///   four cells apart is the ordinary case for a colony, not an edge
    ///   one, and path *selection* between competing routes is the entire
    ///   mechanism (`stigmergy-research.md` §2).
    ///
    /// So the default stands, on stronger evidence than "the literature
    /// says it may not work": Stage 2 builds dedicated per-channel planes
    /// at CA resolution. The sixth `FieldCell` channel is refuted, not
    /// merely unattractive.
    #[test]
    #[ignore = "a measurement, not a guard -- prints numbers, asserts nothing"]
    fn pheromone_resolution_experiment_offset8_tracking() {
        /// Index 0 = east, then counterclockwise on screen (y grows
        /// downward, so `(1, -1)` is up-and-right). The same table
        /// `creature-direction.md` §4a fixes for headings, kept local
        /// because nothing outside this experiment uses it yet.
        const DIRS: [(i32, i32); 8] = [(1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1), (0, 1), (1, 1)];
        /// Sensor offset, in world cells. 8 = exactly `FIELD_SCALE`, the
        /// smallest offset that can straddle two field blocks at all.
        const SO: i32 = 8;
        const TRAIL_Y: i32 = 100;
        const STEPS: usize = 400;

        /// Paint `rows` as one-cell water trails, then settle the field.
        /// Field only -- never the CA sweep, or the water falls and the
        /// trail being measured stops existing (`CLAUDE.md`: a scene that
        /// contradicts the code looks like a bug in the code).
        fn trail_world(rows: &[i32]) -> World {
            let mut w = World::new(Rect::new(0, 0, 255, 199));
            for &y in rows {
                for x in 24..232 {
                    w.set(x, y, Cell::new(material::WATER, 0));
                }
            }
            for _ in 0..60 {
                field::step(&mut w);
            }
            w
        }

        /// One Jones follower run. Returns
        /// `(within 2, within 4, net |dx| / path length)`.
        fn follow(w: &World, label: &str) -> (f32, f32, f32) {
            let (mut px, mut py) = (40.0f32, 108.0f32);
            let start_x = px;
            let mut heading: u8 = 0;
            let (mut near_trail, mut in_band, mut path) = (0usize, 0usize, 0.0f32);
            for step in 0..STEPS {
                let sense = |h: u8| {
                    let (dx, dy) = DIRS[h as usize % 8];
                    w.field_at_bilinear(px + (dx * SO) as f32, py + (dy * SO) as f32).moisture
                };
                let f = sense(heading);
                let l = sense((heading + 1) % 8);
                let r = sense((heading + 7) % 8);

                // Jones: steer toward the strongest of the three; on a tie,
                // choose randomly among the tied options rather than
                // falling through to a fixed preference. Deterministic
                // tie-breaking is the named regression here (P-10) and it
                // would also make this measurement a lie: inside the
                // saturated block every sample is identical, so a fixed
                // rule would report whatever that rule happens to do rather
                // than what a follower does.
                let best = f.max(l).max(r);
                let mut tied: [u8; 3] = [0; 3];
                let mut n = 0;
                for (i, v) in [(0u8, f), (1, l), (2, r)] {
                    if (v - best).abs() < 1e-6 {
                        tied[n] = i;
                        n += 1;
                    }
                }
                let pick = tied[crate::sim::rng::stream(w.seed, 0, step as u64, 0).below(n as u32) as usize];
                heading = match pick {
                    1 => (heading + 1) % 8,
                    2 => (heading + 7) % 8,
                    _ => heading,
                };

                let (dx, dy) = DIRS[heading as usize];
                px = (px + dx as f32).clamp(1.0, 254.0);
                py = (py + dy as f32).clamp(1.0, 198.0);
                path += 1.0;

                let off = (py - TRAIL_Y as f32).abs();
                if off <= 2.0 {
                    near_trail += 1;
                }
                if off <= 4.0 {
                    in_band += 1;
                }
            }
            let (near, band) = (near_trail as f32 / STEPS as f32, in_band as f32 / STEPS as f32);
            let progress = (px - start_x).abs() / path;
            println!("  {label:<22} within2 {near:.3}  within4 {band:.3}  along-trail {progress:.3}  ended ({px:.0}, {py:.0})");
            (near, band, progress)
        }

        println!("pheromone resolution experiment (moisture stand-in, FIELD_SCALE = 8, SO = {SO}, trail at y = {TRAIL_Y}):");
        let tracked = follow(&trail_world(&[TRAIL_Y]), "one trail");
        let control = follow(&trail_world(&[]), "no-trail control");

        // Separation: does a second trail four cells away change anything a
        // sensor can read? Sampled straight off the field, no follower
        // involved -- if the profiles match, no brain and no offset can
        // tell the two routes apart.
        let one = trail_world(&[TRAIL_Y]);
        let two = trail_world(&[TRAIL_Y - 4, TRAIL_Y]);
        println!("  separation probe (column x = 128, moisture by row):");
        let mut worst_gap = 0.0f32;
        for y in (TRAIL_Y - 8)..=(TRAIL_Y + 8) {
            let (a, b) = (one.field_at_bilinear(128.0, y as f32).moisture, two.field_at_bilinear(128.0, y as f32).moisture);
            worst_gap = worst_gap.max((a - b).abs());
            println!("    y={y:>3}  one trail {a:.3}   two trails 4 apart {b:.3}");
        }
        println!("  largest difference one-trail vs two-trails-4-apart: {worst_gap:.4}");
        println!("  tracking {:.3} vs control {:.3}; along-trail progress {:.3}", tracked.0, control.0, tracked.2);
    }

    #[test]
    fn damp_sand_is_cheaper_to_burrow_through_than_dry_sand() {
        // Architecture §4's worm-burrowing consumer. `move_cost` is a plain
        // function of position and material, so this checks it directly
        // rather than through a full burrowing run -- cleaner than fighting
        // the forage tier's own rng for a measurable difference.
        //
        // One water cell in the corner of an otherwise all-sand field block
        // is enough: `apply_moisture_sources` forces the *whole* block
        // containing a `Liquid` cell to `MAX_MOISTURE` the moment `field::
        // step` runs (see its own doc on why it doesn't gate on distance),
        // so a single step -- no diffusion wait needed -- makes every sand
        // cell sharing that block read as saturated.
        let mut dry = test_world();
        for x in 96..104 {
            for y in 96..104 {
                dry.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        field::step(&mut dry);

        let mut damp = test_world();
        for x in 96..104 {
            for y in 96..104 {
                damp.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        damp.set(96, 96, Cell::new(material::WATER, 0)); // corner of the same field block as the probe below
        field::step(&mut damp);

        let probe = (100, 100); // same field block as (96, 96): FIELD_SCALE = 8, block spans 96..=103
        let dry_cost = move_cost(&dry, probe.0, probe.1, material::SAND).expect("sand should always be enterable");
        let damp_cost = move_cost(&damp, probe.0, probe.1, material::SAND).expect("sand should always be enterable");
        assert!(
            damp_cost < dry_cost,
            "damp sand should be cheaper to burrow through than dry sand: dry={dry_cost}, damp={damp_cost}"
        );
    }

    #[test]
    fn a_worm_burrows_through_sand_but_never_enters_stone() {
        let mut w = test_world();
        // Sand to the east, a stone wall to the west -- the worm should be
        // able to move into (eat through) the sand over time, and must
        // never appear inside the stone column regardless of how long it runs.
        for y in 90..110 {
            w.set(80, y, Cell::new(material::STONE, 0));
        }
        for y in 90..110 {
            for x in 81..150 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        w.set(100, 100, Cell::EMPTY); // clear the seed cell -- plant_worm no-ops on occupied ground
        w.plant_worm(100, 100);
        run(&mut w, 400);

        for y in 90..110 {
            assert_eq!(w.get(80, y).material, material::STONE, "the worm entered/displaced the stone wall");
        }
        // A positive check, not just the negative one above: confirms a
        // worm was actually created and did something, so this test can't
        // pass vacuously the way an earlier version of this suite did when
        // `plant_worm` was accidentally called on already-occupied ground
        // (see README.md's M18 status section).
        //
        // **"Something exists" was still vacuous, and the break-check
        // caught it.** Stubbing `worm_tick` to return immediately left this
        // test green: the worm never moved, never starved, and sat exactly
        // where it was planted, satisfying both the stone assertion and an
        // existence check. Requiring that it *left the seed cell* is what
        // makes deleting the mechanism fail here (P-24). Note the sand
        // census would not do it — the move swaps two whole cells, so the
        // sand the worm burrows through is displaced behind it rather than
        // consumed, and the world's sand count is conserved exactly.
        let worm_id = w.materials.id_of("worm").unwrap();
        let corpse_id = w.materials.id_of("corpse").unwrap();
        let is_creature = |x: i32, y: i32| {
            let m = w.get(x, y).material;
            m == worm_id || m == corpse_id
        };
        assert!((81..150).any(|x| (90..110).any(|y| is_creature(x, y))), "no worm or corpse found anywhere -- was one ever actually created?");
        assert!(!is_creature(100, 100), "the worm never left the cell it was planted in -- it did not burrow at all");
    }

    #[test]
    fn a_worm_moves_from_its_starting_position_over_time() {
        let mut w = test_world();
        for y in 90..110 {
            for x in 90..150 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        w.set(95, 95, Cell::EMPTY); // clear the seed cell -- plant_worm no-ops on occupied ground
        w.plant_worm(95, 95);
        run(&mut w, 100);

        let worm_id = w.materials.id_of("worm").unwrap();
        assert_ne!(w.get(95, 95).material, worm_id, "the worm never left its starting cell");
        let found = (85..160).any(|x| (85..115).any(|y| w.get(x, y).material == worm_id));
        assert!(found, "no worm cell found anywhere after running");
    }

    #[test]
    fn the_worm_heat_threshold_does_not_move_with_the_time_of_day() {
        // The guard for `worm_tick`'s flee decision, written at the level of
        // the quantity it branches on rather than of the worm's path —
        // **because an end-to-end noon-vs-midnight behaviour comparison is
        // structurally impossible here**, and that is worth recording rather
        // than rediscovering. `rng::stream` is keyed on `world.frame`, so two
        // runs at different phases of the day draw different random numbers
        // from their first tick, and every trajectory difference is that
        // rather than the sky. Only a phase-invariant *input* can be
        // compared, so that is what this compares.
        //
        // The scene is a field reading deliberately parked just under the
        // threshold: converted it stays under at both phases (no flee, all
        // day), raw it crosses at noon and not at midnight, which is a worm
        // that panics every afternoon at a temperature it ignores at night.
        let reading = |frame: u64| -> (f32, f32) {
            let mut w = test_world();
            w.frame = frame;
            w.add_heat(100, 100, 8, WORM_HEAT_THRESHOLD_ABOVE_AMBIENT - 3.0);
            field::step(&mut w);
            let cell = w.field_at(100, 100);
            (cell.temperature - AMBIENT_TEMPERATURE as f32, field::noon_equivalent_temperature(cell) - AMBIENT_TEMPERATURE as f32)
        };
        let (raw_noon, converted_noon) = reading(0);
        let (raw_midnight, converted_midnight) = reading(field::DAY_NIGHT_PERIOD_FRAMES / 2);
        assert!(
            (converted_noon - converted_midnight).abs() < 0.01,
            "the quantity the flee threshold is compared against moved with the hour: noon {converted_noon}, midnight {converted_midnight}"
        );
        assert!(
            converted_noon < WORM_HEAT_THRESHOLD_ABOVE_AMBIENT && converted_midnight < WORM_HEAT_THRESHOLD_ABOVE_AMBIENT,
            "test setup should sit under the threshold at both phases"
        );
        // And the guard can fail: the raw channel — what this decision read
        // before the sky wrote temperature — does cross at noon.
        assert!(
            raw_noon > WORM_HEAT_THRESHOLD_ABOVE_AMBIENT && raw_midnight < WORM_HEAT_THRESHOLD_ABOVE_AMBIENT,
            "the raw reading is supposed to be phase-dependent here, or this test is not testing anything: \
             noon {raw_noon}, midnight {raw_midnight}"
        );
    }

    #[test]
    fn a_worm_flees_a_hot_field_reading() {
        let mut w = test_world();
        // A single-row corridor (walled top and bottom, so movement is
        // purely east/west) with sand only near the heat source (where the
        // worm starts) and open ground further west -- once the worm clears
        // the sand pocket, continuing to flee is cheap (open-ground cost,
        // not the burrow multiplier), so a real escape doesn't also have to
        // survive an unbroken burrow the whole way, which would be an
        // artifact of the test's energy budget rather than of the fleeing
        // mechanism itself.
        let y = 100;
        for x in 40..140 {
            w.set(x, y - 1, Cell::new(material::STONE, 0));
            w.set(x, y + 1, Cell::new(material::STONE, 0));
        }
        for x in 90..140 {
            w.set(x, y, Cell::new(material::SAND, 0));
        }
        w.set(100, y, Cell::EMPTY); // clear the seed cell -- plant_worm no-ops on occupied ground
        w.plant_worm(100, y);
        // **A ramp, one field cell at a time — because the disc this
        // replaced contained no gradient at all.** The previous version
        // called `add_heat(135, y, 48, 400.0)` and its own comment reasoned
        // that a large radius was needed "to actually reach" the worm. It
        // did reach it, and `paint_field` paints a *flat* value inside the
        // radius, so the entire corridor read 400 degrees uniformly. Every
        // neighbour compared equal, and the worm "fled" west only because
        // `Iterator::min_by` returns the first minimum on a tie and
        // `NEIGHBOURS_4` lists west first — the exact degeneracy the test
        // below this one exists to catch. It passed for a whole milestone
        // while testing nothing, and it failed the moment the choice became
        // a real weighted sample: with no gradient, the correct behaviour is
        // to explore, and an exploring worm starves in a sand corridor.
        //
        // `radius: 0` paints exactly one field cell (`field_radius = radius
        // / FIELD_SCALE` = 0, and the loop still runs once), so stepping by
        // FIELD_SCALE gives one authored value per field cell and a
        // genuinely monotone west-cooling profile across the whole corridor.
        for x in (40..=136).step_by(field::FIELD_SCALE as usize) {
            w.add_heat(x, y, 0, (x - 40) as f32 * 4.0);
        }
        run(&mut w, 300);

        let worm_id = w.materials.id_of("worm").unwrap();
        let fx = (40..140)
            .find(|&x| w.get(x, y).material == worm_id)
            .expect("worm should still be alive and somewhere in the corridor");
        assert!(fx < 100, "worm at x={fx} did not move away from the heat source at x=135");
    }

    #[test]
    fn a_worm_flees_east_even_though_west_is_checked_first() {
        // Architecture §6a regression. `NEIGHBOURS_4` checks west before
        // east, and `Iterator::min_by` returns the *first* minimum on a tie
        // -- so a block-nearest thermotaxis read, which reads the same
        // value for both of a worm's ±1-cell neighbours whenever they share
        // a coarse `FIELD_SCALE`-wide field block (the common case, since
        // FIELD_SCALE = 8), would silently degenerate into "always flee
        // west" regardless of where the heat actually is. The test above
        // puts the heat to the east, where "always flee west" happens to
        // also be the correct answer -- both a working gradient read and
        // the degenerate bug pick the same direction there, so it can't
        // tell them apart. This one is built so the two disagree.
        //
        // `add_heat`'s own paint is flat within its radius (no falloff), so
        // the only place a real temperature *gradient* exists at all is at
        // the edge of the painted disc -- deep inside it, every field cell
        // (and therefore every ±1-cell neighbour) reads identically no
        // matter how it's sampled. A tiny radius (smaller than
        // `FIELD_SCALE`) paints exactly one field cell, so the worm can be
        // placed a couple of world-cells in from that cell's *eastern* edge
        // -- close enough to the ambient cell next door for
        // `field_at_bilinear` to read a real, position-dependent blend
        // toward it (block-nearest can't, since both neighbours floor to
        // the same cell), and far enough from that edge that west and east
        // both still floor to the painted cell, keeping the old
        // "same-block, so tied, so always-west" bug fully in play if this
        // change were ever reverted.
        // **Swept over eight world seeds, and that is the strengthening this
        // test needed to survive the choice becoming probabilistic.** Under
        // the old `min_by`, the block-nearest bug produced a *deterministic*
        // always-west, so one run separated it from a working gradient read
        // cleanly. Under a weighted sample, the same bug produces a tie —
        // and a tie is a coin flip, so a single run agrees with the correct
        // answer half the time and the test degrades into a 50% flake that
        // catches nothing (P-24: a guard must be able to fail for the
        // *replacement*). Eight independent worms make the sabotage's
        // signature — a fair coin — separable from a real gradient's, at
        // p ≈ 3.5% for 7-of-8 by chance.
        let mut went_east = 0;
        for seed in 0..8u64 {
            let mut w = test_world();
            w.seed = 0xC0FFEE + seed;
            let y = 100;
            for x in 60..120 {
                w.set(x, y - 1, Cell::new(material::STONE, 0));
                w.set(x, y + 1, Cell::new(material::STONE, 0));
            }
            // Field cell (11, *) spans world x 88..=95; radius 4 (<
            // FIELD_SCALE = 8) paints only the one field cell containing
            // (88, 100). Field cell (12, *) -- world x 96..=103,
            // immediately east -- stays ambient.
            w.add_heat(88, y, 4, 400.0);
            w.plant_worm(93, y); // 5 cells into the painted cell, 2 short of its eastern edge
            run(&mut w, 300);

            let worm_id = w.materials.id_of("worm").unwrap();
            let fx = (60..120)
                .find(|&x| w.get(x, y).material == worm_id)
                .expect("worm should still be alive and somewhere in the corridor");
            if fx > 93 {
                went_east += 1;
            }
        }
        assert!(went_east >= 7, "only {went_east}/8 worms moved toward the cooler cell to their east -- a block-nearest read makes both neighbours tie, which is a coin flip, not a gradient");
    }

    #[test]
    fn a_worm_with_no_way_to_move_eventually_starves_and_leaves_a_corpse() {
        let mut w = test_world();
        // Sealed in solid stone on all four sides -- no candidate move ever
        // exists, so only the idle-energy-cost path can run.
        w.set(100, 99, Cell::new(material::STONE, 0));
        w.set(100, 101, Cell::new(material::STONE, 0));
        w.set(99, 100, Cell::new(material::STONE, 0));
        w.set(101, 100, Cell::new(material::STONE, 0));
        w.plant_worm(100, 100);

        // WORM_START_ENERGY / WORM_IDLE_COST ticks to starve, at
        // WORM_TICK_INTERVAL frames per tick, plus slack.
        let ticks_to_starve = (WORM_START_ENERGY / WORM_IDLE_COST).ceil() as usize + 5;
        run(&mut w, ticks_to_starve * WORM_TICK_INTERVAL as usize);

        let corpse_id = w.materials.id_of("corpse").unwrap();
        assert_eq!(w.get(100, 100).material, corpse_id, "a permanently trapped worm should have starved into a corpse");
        assert_eq!(w.active_site_count(), 0, "a dead worm should not still be scheduled");
        // The half the old scheme could not do at all: the state comes back.
        // `World::creatures` never shrank, so a dead worm's entry stayed
        // allocated for the life of the process.
        assert!(w.live_organism_ids().is_empty(), "a dead worm's organism slot should have been returned to the free list");
    }

    #[test]
    fn a_burning_worm_keeps_burning_even_when_its_movement_tick_comes_due() {
        // Regression: an independent review caught that a moving worm's
        // cell was always rebuilt from scratch (`Cell::new(worm_id, ...)`),
        // which silently cleared `FLAG_BURNING` and the burn timer the
        // instant a burning worm's next scheduled move came due. Since
        // `WORM_TICK_INTERVAL` (6) is much shorter than worm.ron's
        // `burn_duration` (60), a burning worm normally gets several
        // movement decisions during any single burn -- this fired in the
        // ordinary case, not just an edge case, and the worm simply
        // survived every fire it caught by moving away from its own burning
        // flag. Interleaves the CA sweep (drives `fire.rs`) with the
        // scheduler (drives `creature::tick`) exactly the way the live
        // app's own frame order does (see `App::update`) -- unlike the
        // fire test below (CA sweep only) or this module's own `run()`
        // helper (scheduler only), neither of which alone can exercise
        // this interaction.
        // Open ground, not sand -- deliberately, so movement is always the
        // cheap open-ground cost and the worm's ~400-energy budget cannot
        // run out within this test's frame budget by itself. A dense-sand
        // version of this test would be a false negative either way: a
        // burrowing worm starves to death within ~200 frames from movement
        // cost alone (see this module's other tests), so a corpse would
        // appear regardless of whether the fire bug this guards against was
        // present -- open ground is what isolates "did fire kill it" from
        // "did hunger kill it."
        let mut w = test_world();
        for dx in -20..=20 {
            w.set(100 + dx, 101, Cell::new(material::STONE, 0));
        }
        w.plant_worm(100, 100);
        w.ignite_circle(100, 100, 0);

        for _ in 0..200 {
            update::step(&mut w); // begins/ends the frame; also runs fire.rs
            scheduler::step(&mut w); // same frame's already-advanced counter, matching App::update's order
        }

        let corpse_id = w.materials.id_of("corpse").unwrap();
        let found_corpse = (60..140).any(|x| (60..101).any(|y| w.get(x, y).material == corpse_id));
        assert!(found_corpse, "a worm that moved mid-burn should still have burned out into a corpse, not survived by losing its burning flag");
    }

    #[test]
    fn a_worm_catches_fire_and_burns_into_a_corpse() {
        // Proves the module doc's central claim: fire.rs already applies to
        // every material kind uniformly from `.ron` data alone, so a
        // creature catching fire and dying needs zero creature-specific
        // code -- only `worm.ron`'s own flammability numbers. Uses the CA
        // sweep (`update::step`), not the scheduler, since that's what
        // actually calls `fire::update`.
        let mut w = test_world();
        // A wide floor below -- `corpse` is `kind: Powder` (ordinary
        // destructible loose matter, see corpse.ron), so once it exists it
        // falls/slides/*rolls* under gravity like anything else (see
        // `update::roll_along_slope`). A floor only directly underneath it
        // blocks the straight-down and diagonal falls but not a multi-cell
        // roll onto adjacent open ground followed by a fall from there --
        // observed directly via a throwaway diagnostic print when this test
        // first failed with a 3-wide floor. Wide enough here to comfortably
        // exceed any powder's roll reach.
        for dx in -20..=20 {
            w.set(100 + dx, 101, Cell::new(material::STONE, 0));
        }
        w.plant_worm(100, 100);
        w.ignite_circle(100, 100, 0);
        assert!(w.get(100, 100).is_burning(), "ignite_circle should have set the worm alight");

        for _ in 0..200 {
            update::step(&mut w);
        }

        let corpse_id = w.materials.id_of("corpse").unwrap();
        assert_eq!(w.get(100, 100).material, corpse_id, "a burned-out worm should become a corpse, via worm.ron's own burns_into");
        assert!(!w.get(100, 100).is_burning());

        // Fire takes the cell without telling creature.rs anything. The slot
        // still has to come back -- via the emptiness check on the *next*
        // due tick, since the corpse write went through `World::set`'s seam
        // and emptied the organism's cell list on its way past. A few more
        // frames than the burnout itself, because that tick has to arrive.
        for _ in 0..(WORM_TICK_INTERVAL as usize * 3) {
            w.begin_step();
            scheduler::step(&mut w);
            w.end_step();
        }
        assert!(w.live_organism_ids().is_empty(), "a worm consumed by fire should have released its organism slot -- the leak the old CreatureState vector could not close");
        assert_eq!(w.active_site_count(), 0, "and its site should have dropped itself");
    }

    #[test]
    fn a_settled_world_with_a_worm_still_sleeps_between_movement_ticks() {
        // The same separation-of-concerns property M16 guards for plant
        // growth: a worm moving on its own schedule must not force the CA
        // sweep to keep re-examining a settled world between its ticks.
        let mut w = test_world();
        for y in 90..110 {
            for x in 60..140 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        w.set(100, 100, Cell::EMPTY); // clear the seed cell -- plant_worm no-ops on occupied ground
        w.plant_worm(100, 100);
        run(&mut w, 40); // let the initial placement settle
        w.end_step();
        assert_eq!(w.active_chunk_count(), 0, "a settled world with a resting worm should have no awake CA chunks");
    }

    #[test]
    fn planting_a_worm_on_occupied_ground_is_a_no_op() {
        let mut w = test_world();
        w.set(50, 50, Cell::new(material::STONE, 0));
        w.plant_worm(50, 50);
        assert_eq!(w.get(50, 50).material, material::STONE);
        assert_eq!(w.active_site_count(), 0);
        // And it must not have leaked a slot on the way to refusing. The
        // emptiness check is deliberately *before* `push_organism` for
        // exactly this: a brush dragged across stone would otherwise burn
        // through the 4,095-slot ceiling without ever creating a worm.
        assert!(w.live_organism_ids().is_empty(), "a refused planting must not leave an organism allocated");
    }

    // --- what joining the organism substrate had to preserve, and what it
    // --- newly makes true ---------------------------------------------------

    #[test]
    fn a_moving_worm_carries_its_whole_cell() {
        // **The P-1 guard that can actually fail.** Its sibling,
        // `a_burning_worm_keeps_burning...`, cannot: the burning-defer above
        // means a burning worm never reaches the move at all, so
        // reintroducing a `Cell::new(worm_id, shade)` rebuild leaves that
        // test green (verified, not assumed -- see the commit). This one
        // watches a property the rebuild destroys on an *ordinary* move.
        //
        // Temperature is the observable: it is carried on the `Cell`, it is
        // not derivable from the material, and the old rebuild needed an
        // explicit `.with_temperature(...)` to preserve it -- so deleting
        // that patch, or swapping the whole-cell move back for a rebuild,
        // both fail here. Which is the point: a chain creature moves N cells
        // per step, and anything a rebuild forgets is forgotten N times.
        let mut w = test_world();
        for y in 99..=101 {
            for x in 95..110 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        w.set(100, 100, Cell::EMPTY);
        w.plant_worm(100, 100);
        let marked = w.get(100, 100).with_temperature(137);
        w.set(100, 100, marked);
        assert_eq!(w.get(100, 100).temperature(), 137, "the marker should be on the cell before the run");

        let worm_id = w.materials.id_of("worm").unwrap();
        run(&mut w, WORM_TICK_INTERVAL as usize * 3);

        let (fx, fy) = (95..110)
            .flat_map(|x| (99..=101).map(move |y| (x, y)))
            .find(|&(x, y)| w.get(x, y).material == worm_id)
            .expect("the worm should still be alive");
        assert_ne!((fx, fy), (100, 100), "the worm should have moved, or this proves nothing");
        assert_eq!(w.get(fx, fy).temperature(), 137, "a moved worm must arrive with its own temperature -- the move swaps whole Cells, it does not rebuild one");
        assert_eq!(organism::cell_type(w.get(fx, fy).aux()), Some(CellType::Head), "and with its CellType, which now lives in the same aux the rebuild used to overwrite");
    }

    #[test]
    fn a_worm_stays_a_head_cell_through_the_organism_upkeep_pass() {
        // Creatures joining the organism substrate means `plant::step_
        // organisms` now iterates a worm every ORGANISM_TICK_INTERVAL
        // frames, running transport, allocation, support accumulation, bud
        // break and upkeep over it. All of those are gated on behaviors or
        // cell types a worm does not have, so all of them should be no-ops
        // -- **proven, not assumed**, because the failure mode is loud
        // (CLAUDE.md: a structural check scheduled mid-organism amputates
        // it) and it would present as "creature.rs is broken".
        let mut w = test_world();
        for x in 40..160 {
            w.set(x, 101, Cell::new(material::STONE, 0));
        }
        w.plant_worm(100, 100);
        let organism = w.get(100, 100).organism_id();
        assert_ne!(organism, 0, "the worm should own its cell");

        // Long enough for many organism ticks, and for the CA sweep to have
        // had every chance to interact with the cell.
        for _ in 0..600 {
            update::step(&mut w);
            scheduler::step(&mut w);
        }

        let worm_id = w.materials.id_of("worm").unwrap();
        let (fx, fy) = (40..160)
            .flat_map(|x| (60..101).map(move |y| (x, y)))
            .find(|&(x, y)| w.get(x, y).material == worm_id)
            .expect("the worm should have survived 600 frames on open ground");
        assert_eq!(organism::cell_type(w.get(fx, fy).aux()), Some(CellType::Head), "the upkeep pass must not have retired, thickened or converted a creature cell");
        assert_eq!(w.get(fx, fy).organism_id(), organism, "and must not have re-owned it");
        assert_eq!(w.organism(organism).expect("still live").cells.len(), 1, "a one-cell chain should own exactly one cell after all that");
    }

    #[test]
    fn a_stale_creature_site_after_slot_reuse_drops_silently() {
        // The generational property, at the creature dispatch. The old
        // scheme stored a raw index with no generation, so a site outliving
        // its creature indexed straight into whoever had been allocated that
        // slot since -- and `creature_mut` would have handed it out happily.
        use crate::sim::scheduler::{ActiveKind, ActiveSite};

        let mut w = test_world();
        for x in 40..160 {
            w.set(x, 101, Cell::new(material::STONE, 0));
        }
        w.plant_worm(100, 100);
        let doomed = w.get(100, 100).organism_id();
        w.set(100, 100, Cell::EMPTY); // erased by the brush, say
        w.free_organism(doomed);

        let heir = w.species.id_of("worm").expect("worm species");
        let heir = w.push_organism(heir).expect("an organism slot is free");
        assert_eq!(doomed & 0x0FFF, heir & 0x0FFF, "the test needs the heir to inherit the slot");
        if let Some(state) = w.organism_mut(heir) {
            state.energy = 999.0;
        }

        w.schedule_active_site(ActiveSite { x: 100, y: 100, kind: ActiveKind::Creature { organism: doomed }, next_frame: w.frame + 1 });
        run(&mut w, 10); // must not panic

        assert_eq!(w.organism(heir).expect("the heir is live").energy, 999.0, "a stale creature site must not have spent the energy of the organism that inherited its slot");
        assert!(w.active_site_count() <= 1, "the stale site should have dropped itself rather than rescheduling");
    }

    // --- partial body loss --------------------------------------------------

    /// Build a two-cell ant on a stone floor and hand back its handle.
    fn ant_on_a_floor(w: &mut World, x: i32) -> u16 {
        for cx in 0..200 {
            w.set(cx, 101, Cell::new(material::STONE, 0));
        }
        w.plant_ant(x, 100);
        w.get(x, 100).organism_id()
    }

    /// One hungry ant, a bank of soil to its right and a corpse to its
    /// left, driven by a genome that authors exactly one of the two verbs.
    ///
    /// **A test rather than an ablation arm, because the ablation scene
    /// cannot show this.** `ant_ablation` reports `eats 0.0 digs 0.0` in
    /// every arm including `authored`: its floor is stone, so there is
    /// nothing diggable, and at `start_energy` 900 no ant falls below the
    /// 0.5 `hunger_fraction` inside 6,000 frames, so nothing is ever eaten
    /// rather than carried. Both counters are structurally zero there, and
    /// a decoupling claim measured against them would have been two zeroes
    /// agreeing with each other.
    fn verbs_scene(feed: f32, dig: f32) -> (u64, u64) {
        let mut w = test_world();
        let soil = w.materials.id_of("soil").expect("soil");
        // **A leaf, and it used to be a corpse.** This test is about the
        // Feed/Dig split and the food is incidental to it -- but S5 made
        // the shipped ant a detritivore that cannot digest carrion at all,
        // so the corpse arm stopped feeding and the failure read as "Feed
        // no longer eats". Swapped for a food this species actually eats,
        // rather than overriding the gut here: a wiring test should not
        // have to know about diet, and it does not now.
        let food = w.materials.id_of("leaf").expect("leaf");
        for cx in 90..130 {
            w.set(cx, 101, Cell::new(material::STONE, 0).with_attached(true));
        }
        // Geometry fitted to what the verbs actually read, which is not the
        // same for the two of them: `act` digs strictly the cell in the
        // heading direction (east, at spawn), while `adjacent_food` scans
        // the head's whole 8-neighbourhood. So the soil goes *ahead* of the
        // head and the corpse diagonally above it, where it can be eaten
        // but can never be the dig target -- otherwise the dig arm would
        // excavate the corpse and book it as a dig.
        w.set(101, 100, Cell::new(soil, 0).with_attached(true));
        w.set(101, 99, Cell::new(food, 0).with_attached(true));

        let species = w.species.id_of("ant").expect("ant species");
        let def = w.species.get(species).creature.as_ref().expect("creature").clone();
        w.species.set_genome(
            species,
            brain::genome_from_wiring(
                &[
                    brain::Instinct(brain::BrainInput::Bias, brain::BrainOutput::Move, 2.0),
                    brain::Instinct(brain::BrainInput::Bias, brain::BrainOutput::Feed, feed),
                    brain::Instinct(brain::BrainInput::Bias, brain::BrainOutput::Dig, dig),
                ],
                &def.hidden_wiring,
                &def.hidden_outputs,
                &def.recurrence,
            ),
        );

        w.plant_ant(100, 100);
        let ant = w.get(100, 100).organism_id();
        // **Assert the animal exists.** `plant_creature_seed` refuses to
        // place a chain whose cells are not all empty and returns quietly,
        // so a scene that puts anything on the body's own cells produces an
        // empty world and two counters that read zero for a reason that has
        // nothing to do with the gene under test. The first draft of this
        // scene did exactly that -- the corpse sat where the tail goes.
        assert_ne!(ant, 0, "the ant was not placed; the scene does not contain the situation this test is about");
        // Hungry, or it carries its food home instead of eating it and the
        // counter under test never moves for a reason that is not the gene.
        if let Some(state) = w.organism_mut(ant) {
            state.energy = def.start_energy * def.hunger_fraction * 0.5;
        }
        run(&mut w, 400);
        (w.creature_stats.eats, w.creature_stats.digs)
    }

    #[test]
    fn feeding_and_digging_are_separate_genes() {
        // **The claim the Feed/Dig split exists to make.** Before it, one
        // weight gated both verbs, so this pair could only move together --
        // §13d's `(Bias, Dig, 0.4)`, added because ants never dug, silently
        // raised the baseline *eating* probability, and there was no gene
        // that could tell a burrower from a grazer.
        let (eats_only, digs_when_feeding) = verbs_scene(2.0, 0.0);
        let (eats_when_digging, digs_only) = verbs_scene(0.0, 2.0);

        assert!(eats_only > 0, "a Feed weight with no Dig weight must still let the animal eat");
        assert_eq!(digs_when_feeding, 0, "Feed must not dig: that is the coupling this split removed");
        assert!(digs_only > 0, "a Dig weight with no Feed weight must still let the animal excavate");
        assert_eq!(eats_when_digging, 0, "Dig must no longer feed the animal -- one weight moving both is the bug");
    }

    // --- the sight sense (E15) ------------------------------------------

    /// A bare stone floor with a beetle and an ant standing on it, both
    /// facing east. Returns the world, the beetle's handle and its
    /// `CreatureDef`, so a caller can override `sight_range` before
    /// probing.
    ///
    /// **The floor is `STONE` and nothing else is on it**, so a sighting
    /// that fails in a derived scene fails because of what that scene added.
    fn sight_bed(beetle_x: i32, ant_x: i32) -> (World, u16, CreatureDef) {
        let mut w = test_world();
        for cx in 0..200 {
            w.set(cx, 101, Cell::new(material::STONE, 0));
        }
        let beetle = plant_creature_seed(&mut w, beetle_x, 100, "beetle").map(|_| w.get(beetle_x, 100).organism_id()).unwrap_or(0);
        assert_ne!(beetle, 0, "the beetle was not placed; the scene does not contain the situation this test is about");
        w.plant_ant(ant_x, 100);
        assert_ne!(w.get(ant_x, 100).organism_id(), 0, "the ant was not placed; there is nothing to see");
        let def = w.species.get(w.organism(beetle).expect("live").species).creature.as_ref().expect("beetle is a creature").clone();
        (w, beetle, def)
    }

    /// `(PreyNear, PreyBearing)` as the beetle at `(x, 100)` reads them.
    fn prey_inputs(w: &World, beetle: u16, x: i32, def: &CreatureDef) -> (f32, f32) {
        let (inputs, _, _) = probe(w, x, 100, beetle, def);
        (inputs[brain::BrainInput::PreyNear as usize], inputs[brain::BrainInput::PreyBearing as usize])
    }

    #[test]
    fn a_beetle_sees_an_ant_across_a_bare_floor() {
        // **The positive control, and it comes first deliberately.** Every
        // other test here is a claim that something stops the sense; none
        // of them means anything until the sense is shown to fire at all,
        // and a `los`-only readout cannot tell a reach failure from an
        // occlusion failure (`creature-vision-sizing-2026-08-30.md` §2).
        let (w, beetle, def) = sight_bed(60, 90);
        let (near, bearing) = prey_inputs(&w, beetle, 60, &def);
        assert!(near > 0.0, "a beetle on a bare floor 30 cells from an ant must see it");
        // 30 cells at reach 64: 1 - 30/64 = 0.53, give or take the ant's
        // chain laying out to the left of its head.
        assert!((0.45..0.65).contains(&near), "nearness should scale with distance, read {near}");
        assert!(bearing.abs() < 0.05, "prey dead ahead of an east-facing beetle is a bearing of ~0, read {bearing}");
    }

    #[test]
    fn a_wall_stops_the_sight_line() {
        // The occlusion test is not stuck off — the same ants at the same
        // distance behind a full-height slab.
        let (mut w, beetle, def) = sight_bed(60, 90);
        for cy in 80..101 {
            w.set(75, cy, Cell::new(material::STONE, 0));
        }
        let (near, _) = prey_inputs(&w, beetle, 60, &def);
        assert_eq!(near, 0.0, "a full-height stone wall between them must stop every ray");
    }

    #[test]
    fn floor_litter_does_not_blind_a_beetle_but_a_two_cell_pile_does() {
        // **The whole of the eye-height rule, and both halves of it.** Both
        // animals are ground-huggers, so a sight line between two heads
        // grazes the floor for its length and a low pile stops a long line:
        // measured at 28.1% of prey pairs blocked at head height against
        // 8.5% one cell up. `SIGHT_EYE_LIFT` is what gets over the first
        // pile; the second reaches the lifted eye and must still stop it,
        // or the "fix" is really "occlusion is off".
        let (mut w, beetle, def) = sight_bed(60, 90);
        let soil = w.materials.id_of("soil").expect("soil");
        w.set(75, 100, Cell::new(soil, 0));
        assert!(prey_inputs(&w, beetle, 60, &def).0 > 0.0, "one cell of floor clutter must not blind a beetle: the eye sits above it");

        w.set(75, 99, Cell::new(soil, 0));
        assert_eq!(prey_inputs(&w, beetle, 60, &def).0, 0.0, "clutter that reaches the eye must still stop the ray, or nothing is occluding anything");
    }

    #[test]
    fn prey_past_the_eye_is_not_seen() {
        // The radius test is a radius test. 100 cells against a reach of
        // 64, on the same bare floor that reads a sighting at 30.
        let (w, beetle, def) = sight_bed(20, 120);
        assert_eq!(prey_inputs(&w, beetle, 20, &def).0, 0.0, "an ant 100 cells away is outside a 64-cell eye");
    }

    #[test]
    fn a_species_with_no_sight_range_sees_nothing_and_costs_nothing() {
        // **The opt-in, which is the guard that keeps this sense off every
        // other species in the world.** `sight_range` defaults to 0 and the
        // gate is at the call site, so an eyeless animal must read exactly
        // the zero it read before this input existed — and the counters
        // must stay at zero too, or the sense is running and merely quiet.
        let (mut w, beetle, mut def) = sight_bed(60, 90);
        assert!(prey_inputs(&w, beetle, 60, &def).0 > 0.0, "the eyed control must see it, or this test cannot fail for the right reason");

        def.sight_range = 0;
        let (near, bearing) = prey_inputs(&w, beetle, 60, &def);
        assert_eq!((near, bearing), (0.0, 0.0), "a species with no eyes must read the same zero it read before this input existed");

        let species = w.organism(beetle).expect("live").species;
        w.species.set_creature(species, def);
        for _ in 0..200 {
            w.step_active_sites();
        }
        assert_eq!(
            (w.creature_stats.sight_casts, w.creature_stats.sightings),
            (0, 0),
            "an eyeless world must not cast a single ray; the gate is at the call site for exactly this reason"
        );
    }

    #[test]
    fn the_ant_has_no_eyes_and_the_shipped_ant_is_unchanged_by_this_input() {
        // Every species but the beetle authors no `sight_range`, which is
        // what makes the append lawful for them: two new slots, both zero,
        // read by nothing.
        let mut w = test_world();
        let ant = ant_on_a_floor(&mut w, 100);
        let def = w.species.get(w.organism(ant).expect("live").species).creature.as_ref().expect("ant is a creature").clone();
        assert_eq!(def.sight_range, 0, "ant.ron must not author an eye");
        let (inputs, _, _) = probe(&w, 100, 100, ant, &def);
        assert_eq!(inputs[brain::BrainInput::PreyNear as usize], 0.0);
        assert_eq!(inputs[brain::BrainInput::PreyBearing as usize], 0.0);
    }

    #[test]
    fn the_bearing_says_which_way_to_turn() {
        // **Positive is to the right**, matching `PheroALateral`, and
        // `Turn` biases *left* when positive — so a pursuit instinct is a
        // negative weight. A sign error here is a beetle that flees what it
        // can see, and it would look exactly like a sense that does not
        // work.
        let (w, beetle, def) = sight_bed(90, 60); // prey behind an east-facing beetle
        let (near, behind) = prey_inputs(&w, beetle, 90, &def);
        assert!(near > 0.0, "the beetle must see the ant behind it: the eye is all-round, not a cone");
        assert!(behind.abs() > 0.9, "prey directly behind is a hard turn, +-1, not the 0 a left-minus-right sensor would give; read {behind}");

        // **The left/right half, asked with the heading rather than with
        // the geometry.** The first draft built a ledge above the beetle
        // and the ledge blocked the very ray that would have reached the
        // ant standing on it — a scene that contradicted the code, which
        // reads exactly like the mechanism being inert (`CLAUDE.md`).
        // Turning the animal instead needs no terrain at all: facing north,
        // east is on the right hand and west on the left.
        let (mut w2, beetle2, def2) = sight_bed(100, 130);
        w2.plant_ant(70, 100);
        assert_ne!(w2.get(70, 100).organism_id(), 0, "the western ant was not placed");
        w2.organism_mut(beetle2).expect("live").heading = 2; // north
        let (near2, east) = prey_inputs(&w2, beetle2, 100, &def2);
        assert!(near2 > 0.0, "the beetle must still see an ant while facing north: the eye is all-round");
        // The eastern ant is 30 cells away and the western one 30 as well,
        // so which one is nearest is decided by the chains laying out to
        // the left of each head. Assert the *magnitude* is a right angle
        // and the sign matches whichever side won, rather than assuming.
        assert!((east.abs() - 0.5).abs() < 0.05, "prey abeam a north-facing beetle is a quarter turn, +-0.5; read {east}");
    }

    #[test]
    fn a_beetle_does_not_see_another_beetle() {
        // Its own body is excluded by owner and a nestmate by `eats_kin` —
        // the same two exemptions `adjacent_food` makes. Without them the
        // "prey" sense would be a beetle-detector and every counter it
        // moved would be beetles chasing each other.
        let mut w = test_world();
        for cx in 0..200 {
            w.set(cx, 101, Cell::new(material::STONE, 0));
        }
        let beetle = plant_creature_seed(&mut w, 60, 100, "beetle").map(|_| w.get(60, 100).organism_id()).unwrap_or(0);
        assert_ne!(beetle, 0);
        assert_ne!(plant_creature_seed(&mut w, 90, 100, "beetle").map(|_| w.get(90, 100).organism_id()).unwrap_or(0), 0, "the second beetle was not placed");
        let def = w.species.get(w.organism(beetle).expect("live").species).creature.as_ref().expect("creature").clone();
        assert_eq!(prey_inputs(&w, beetle, 60, &def).0, 0.0, "a beetle is not prey to a beetle, and neither is its own 2x2 body");
    }

    #[test]
    fn a_lone_creature_is_not_crowded_by_its_own_body() {
        // **The known-good case, which is what makes this a measurement
        // rather than a preference**: an animal alone on a floor with
        // nothing within ten cells is, by any reading of the word, not
        // crowded. It read 0.125 -- its own tail -- and a beetle read
        // 0.375, so `Crowding` was partly a body-size sensor.
        let mut w = test_world();
        let ant = ant_on_a_floor(&mut w, 100);
        let def = w.species.get(w.organism(ant).expect("live").species).creature.as_ref().expect("ant is a creature").clone();
        let (inputs, _, _) = probe(&w, 100, 100, ant, &def);
        assert_eq!(inputs[brain::BrainInput::Crowding as usize], 0.0, "a creature alone on a floor must read no crowding at all");

        // And it still senses a *neighbour*, which is the half that must
        // not be thrown out with the fix: an exclusion that read 0.000 in
        // both cases would be a dead input, not a corrected one.
        w.plant_ant(102, 100);
        let (inputs, _, _) = probe(&w, 100, 100, ant, &def);
        assert!(inputs[brain::BrainInput::Crowding as usize] > 0.0, "another animal beside it is exactly what this input is for");
    }

    #[test]
    fn losing_a_trailing_segment_is_an_injury_not_a_death() {
        let mut w = test_world();
        let ant = ant_on_a_floor(&mut w, 100);
        let tail = w.organism(ant).expect("live").chain[1];
        assert_eq!(w.organism(ant).expect("live").chain.len(), 2);

        w.set(tail.0, tail.1, Cell::EMPTY); // bitten off
        run(&mut w, 20);

        let state = w.organism(ant).expect("an ant that lost its tail should still be alive");
        assert_eq!(state.chain.len(), 1, "the chain must shrink to what the ant actually still owns");
        assert_eq!(w.creature_stats.injuries, 1);
        assert_eq!(w.creature_stats.deaths, 0);
    }

    #[test]
    fn losing_the_head_kills_the_creature_and_frees_its_slot() {
        // **The bug this exists for.** An organism only released itself
        // when *every* cell was gone, so removing the head left a live
        // organism driving decisions from a stale chain whose first entry
        // was a cell that no longer existed -- and an orphan segment
        // standing in the world forever. Predation is unbuildable until
        // this holds.
        let mut w = test_world();
        let ant = ant_on_a_floor(&mut w, 100);
        let head = w.organism(ant).expect("live").chain[0];

        w.set(head.0, head.1, Cell::EMPTY);
        run(&mut w, 20);

        assert!(w.organism(ant).is_none(), "losing the head should kill the creature");
        assert!(w.live_organism_ids().is_empty(), "and free its slot");
        let corpse = w.materials.id_of("corpse").unwrap();
        let ant_material = w.materials.id_of("ant").unwrap();
        let leftover = (90..110).flat_map(|x| (95..101).map(move |y| (x, y))).filter(|&(x, y)| w.get(x, y).material == ant_material).count();
        assert_eq!(leftover, 0, "no orphan segment may be left standing");
        let meat = (90..110).flat_map(|x| (95..102).map(move |y| (x, y))).filter(|&(x, y)| w.get(x, y).material == corpse).count();
        assert!(meat > 0, "what is left of it should be matter -- a dead creature is food");
    }

    // --- body plans ---------------------------------------------------------

    fn spawn(w: &mut World, species: &str, x: i32, y: i32) -> u16 {
        plant_creature_seed(w, x, y, species).map(|site| {
            w.schedule_active_site(site);
            w.get(x, y).organism_id()
        }).expect("the species should be placeable here")
    }

    #[test]
    fn a_rigid_body_occupies_its_whole_template() {
        let mut w = test_world();
        for cx in 0..200 {
            w.set(cx, 101, Cell::new(material::STONE, 0));
        }
        let beetle = spawn(&mut w, "beetle", 100, 100);
        let cells = &w.organism(beetle).expect("live").cells;
        assert_eq!(cells.len(), 4, "a 2x2 Rigid body is four cells, not one");
        for p in [(100, 100), (99, 100), (100, 101), (99, 101)] {
            let _ = p;
        }
        // The template is (0,0) head plus (-1,0), (0,1), (-1,1) facing east.
        for p in [(100, 100), (99, 100), (100, 99 + 2), (99, 99 + 2)] {
            let _ = p;
        }
        assert!(cells.contains_key(&(100, 100)) && cells.contains_key(&(99, 100)), "the top row of the template");
    }

    #[test]
    fn a_wide_body_cannot_enter_a_one_cell_tunnel_that_a_chain_walks_through() {
        // **The refuge, and there is no hiding code anywhere.** An ant is a
        // one-cell-wide following chain; a beetle is a 2x2 rigid block.
        // A tunnel one cell tall admits the first and refuses the second,
        // purely because a rigid body's passability check covers every cell
        // of it. This is the property `Reports/creature-direction.md` D1's
        // rejection of rigid bodies was assumed to cost us, and it is the
        // one that makes digging worth doing.
        let build = || {
            let mut w = test_world();
            // Solid rock with a one-cell-tall horizontal tunnel through it.
            for x in 60..160 {
                for y in 90..110 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            for x in 100..160 {
                w.set(x, 100, Cell::EMPTY);
            }
            // A mouth wide and tall enough for either creature to stand in.
            for x in 60..100 {
                for y in 96..101 {
                    w.set(x, y, Cell::EMPTY);
                }
            }
            w
        };
        let w = build();

        // **One creature per world.** Run together, the beetle simply ate
        // the ant -- which is the predation path working, and exactly the
        // kind of second mechanism that turns a geometric claim into an
        // unfalsifiable one. The question here is only "does this body fit",
        // so nothing else is allowed in the scene.
        let deepest = |w: &World, organism: u16| -> i32 {
            w.organism(organism).map_or(0, |s| s.cells.keys().map(|&(x, _)| x).max().unwrap_or(0))
        };

        let mut ant_world = w;
        let ant = spawn(&mut ant_world, "ant", 98, 100);
        run(&mut ant_world, 2000);
        let ant_x = deepest(&ant_world, ant);

        let mut beetle_world = build();
        let beetle = spawn(&mut beetle_world, "beetle", 90, 100);
        run(&mut beetle_world, 2000);
        let beetle_x = deepest(&beetle_world, beetle);

        assert!(ant_x >= 105, "the ant should have been able to walk into the tunnel; deepest cell x={ant_x}");
        assert!(
            beetle_x < 100,
            "a 2x2 beetle must not fit into a one-cell tunnel; deepest cell x={beetle_x}. Passability has to cover every cell of a rigid body, not just its head"
        );
    }

    #[test]
    fn a_rigid_body_translates_rather_than_following_its_head() {
        // The other half of `BodyPlan`: a chain's cells end up where the
        // head has *been*, a rigid body's keep their shape. Asserted as
        // "the set of offsets between the cells is unchanged after moving".
        let mut w = test_world();
        for cx in 0..200 {
            w.set(cx, 101, Cell::new(material::STONE, 0));
        }
        let beetle = spawn(&mut w, "beetle", 100, 100);
        let shape = |w: &World| -> Vec<(i32, i32)> {
            let cells = &w.organism(beetle).expect("live").cells;
            let minx = cells.keys().map(|&(x, _)| x).min().unwrap();
            let miny = cells.keys().map(|&(_, y)| y).min().unwrap();
            let mut v: Vec<(i32, i32)> = cells.keys().map(|&(x, y)| (x - minx, y - miny)).collect();
            v.sort();
            v
        };
        let before = shape(&w);
        run(&mut w, 400);
        assert_eq!(before, shape(&w), "a rigid body must keep its shape as it moves");
        assert_eq!(w.organism(beetle).expect("live").cells.len(), 4);
    }

    #[test]
    fn a_predator_eats_a_creature_and_needs_no_predation_code_to_do_it() {
        // **Nothing in the engine knows what "predator" means.** It used
        // to be that `food:` was a list of material names and "ant" was a
        // material; since S5 it is a carnivore `gut_bias` against `ant`
        // material's `food_class: 1.0`, and the existing eat verb still
        // does the rest -- the same verb that eats corpses and leaves.
        // Predation also survives the `eats_kin` default for free, because
        // a beetle's kin is a beetle and an ant is somebody else's flesh.
        // This was found by accident: an isolation test put a beetle and an
        // ant in one world and the ant vanished.
        //
        // The other half is `reconcile_chain`: a bite removes one cell, and
        // an organism only released itself when *every* cell was gone, so
        // before that existed a half-eaten ant left an orphan segment
        // running on a stale chain.
        // Floor across the whole world, and the ant placed **touching** the
        // beetle. A first version gave the beetle a short floor and put the
        // ant four cells away: with no pheromone instincts a beetle has no
        // way to sense food at range, so it run-and-tumbled off the end of
        // the floor and fell out of the world. Correct physics, useless
        // scene -- and a reminder that this test is about whether eating a
        // creature *works*, not about whether a predator can find one.
        // A sealed chamber, because an open floor tests the wrong thing.
        // The ant ticks every 6 frames and the beetle every 8, so on open
        // ground the ant simply walks away before the beetle's first
        // decision -- and a beetle has no pheromone instincts, so it cannot
        // sense food at range and never catches up. Whether a predator can
        // *find* prey is a separate question from whether eating one works,
        // and this test is the second.
        let mut w = test_world();
        for x in 92..112 {
            w.set(x, 101, Cell::new(material::STONE, 0));
            w.set(x, 96, Cell::new(material::STONE, 0));
        }
        for y in 96..102 {
            w.set(92, y, Cell::new(material::STONE, 0));
            w.set(111, y, Cell::new(material::STONE, 0));
        }
        let ant = spawn(&mut w, "ant", 108, 100);
        let beetle = spawn(&mut w, "beetle", 100, 100);
        assert!(w.organism(ant).is_some() && w.organism(beetle).is_some());

        run(&mut w, 1200);

        assert!(w.organism(ant).is_none(), "the beetle should have eaten the ant");
        // Eat *or* pick up: the same verb, branching on hunger. A beetle
        // that starts near full carries its prey rather than swallowing it
        // on the spot, which is the carry-versus-eat split working, not a
        // different mechanism. Asserting `eats` alone made this test
        // depend on the predator's energy budget rather than on predation.
        let consumed = w.creature_stats.eats + w.creature_stats.pickups;
        assert!(consumed > 0, "the ant should have gone through the food verb, not vanished some other way");
        assert!(w.organism(beetle).is_some(), "the beetle should still be alive");
        let ant_material = w.materials.id_of("ant").unwrap();
        let leftovers = (92..112).flat_map(|x| (96..101).map(move |y| (x, y))).filter(|&(x, y)| w.get(x, y).material == ant_material).count();
        assert_eq!(leftovers, 0, "no orphaned ant cell may be left standing -- reconcile_chain is what stops that");
    }

    /// Set one species' gut for the duration of a test.
    fn set_gut(w: &mut World, species: &str, bias: f32) {
        let id = w.species.id_of(species).expect("species");
        let mut def = w.species.get(id).creature.as_ref().expect("creature").clone();
        def.traits[TRAIT_GUT_BIAS] = bias;
        w.species.set_creature(id, def);
    }

    /// **A meat gut stops *seeing* leaves**, which is the whole difference
    /// between S5 being a behaviour and S5 being bookkeeping.
    ///
    /// Read through `probe`, so it asserts on the input the *brain* gets
    /// rather than on the eat verb's private predicate: the plan's
    /// requirement is that `BrainInput::FoodAdjacent` and the mouth read
    /// the same gene-dependent test, and an animal that steers at food it
    /// cannot digest has the gene without the behaviour.
    ///
    /// Paired -- the same ant, the same leaf, the same cell -- so the only
    /// thing that differs between the two readings is the gut. A single
    /// arm here would be a sample of one against a remembered number.
    #[test]
    fn a_meat_gut_stops_seeing_leaves() {
        let leaf_beside_an_ant = |bias: f32| -> f32 {
            let mut w = test_world();
            for x in 90..110 {
                w.set(x, 101, Cell::new(material::STONE, 0));
            }
            set_gut(&mut w, "ant", bias);
            let ant = spawn(&mut w, "ant", 100, 100);
            let leaf = w.materials.id_of("leaf").expect("leaf");
            w.set(101, 100, Cell::new(leaf, 0));
            let def = w.species.get(w.organism(ant).unwrap().species).creature.as_ref().unwrap().clone();
            let (inputs, _, _) = probe(&w, 100, 100, ant, &def);
            inputs[brain::BrainInput::FoodAdjacent as usize]
        };

        // A leaf is `food_class: -1.0` worth 120. Against a neutral gut the
        // filter reads 0.25 -> 30, over the bar of 12; against +0.9 it
        // reads 0.0025 -> 0.3, under it.
        assert_eq!(leaf_beside_an_ant(0.0), 1.0, "a generalist must see a leaf it is standing next to");
        assert_eq!(leaf_beside_an_ant(0.9), 0.0, "a meat gut must not see a leaf as food -- the gene has to change what the animal perceives");
    }

    /// **A colony does not eat itself** — asserted on the predicate, not
    /// on whether two ants happen to meet.
    ///
    /// The first version of this ran two ants in a sealed chamber for 1,200
    /// frames and asserted survival, with the cannibal arm as its control.
    /// **The control failed**, and that is why it was there: a `Chain(2)`
    /// ant spawned four cells from another walks away long before it
    /// decides anything, and an ant has no way to sense a nestmate as food
    /// at range — the identical scene error the predation test above
    /// records paying for. A survival assertion in that world passes
    /// whatever the kin rule does, which is `CLAUDE.md`'s superseded-test
    /// failure written fresh. So this asks `adjacent_food` directly, where
    /// the answer is deterministic and both arms can fail.
    ///
    /// This is the regression net for §13i, whose measured failure was that
    /// a colony eating its own sustains itself without foraging at all —
    /// and for the specific way a future fix is likely to break it, which
    /// is by reaching for a yield threshold. See `is_living_kin`: live ant
    /// flesh and a starved ant's corpse are the same class *and* the same
    /// number, so a threshold that hides one hides the other.
    #[test]
    fn an_ant_will_not_bite_a_living_nestmate_unless_its_species_says_it_may() {
        // A carnivore gut, so the *only* thing under test is the kin rule:
        // ant flesh is `food_class: 1.0` worth 120, which at +1.0 yields
        // the full 120 and is edible on yield alone. If the kin rule were
        // absent, both arms would return `Some`.
        let nestmate_seen_as_food = |eats_kin: bool| -> bool {
            let mut w = test_world();
            for x in 90..112 {
                w.set(x, 101, Cell::new(material::STONE, 0));
            }
            let id = w.species.id_of("ant").expect("ant");
            let mut def = w.species.get(id).creature.as_ref().expect("creature").clone();
            def.traits[TRAIT_GUT_BIAS] = 1.0;
            def.eats_kin = eats_kin;
            w.species.set_creature(id, def.clone());

            // `Chain(2)`, laid out to the left of the head: A takes 100 and
            // 99, B takes 102 and 101. B's tail is then A's head's east
            // neighbour, so the nestmate is genuinely in reach at frame 0
            // and no walking has to happen for the question to be asked.
            let a = spawn(&mut w, "ant", 100, 100);
            let b = spawn(&mut w, "ant", 102, 100);
            assert!(w.organism(a).is_some() && w.organism(b).is_some(), "both ants must place");
            let ant_material = w.materials.id_of("ant").expect("ant material");
            assert_eq!(w.get(101, 100).material, ant_material, "the scene must actually contain a nestmate in reach -- a mechanism looks inert when the scene lost the situation");

            adjacent_food(&w, 100, 100, gut_of(&w, a, &def)).is_some()
        };

        assert!(!nestmate_seen_as_food(false), "with eats_kin off, a living nestmate is not food");
        assert!(nestmate_seen_as_food(true), "with eats_kin on it must be -- if this fails, the arm proves nothing and the test above it is vacuous");
    }

    fn set_climbs_over_kin(w: &mut World, species: &str, on: bool) -> CreatureDef {
        let id = w.species.id_of(species).expect("species");
        let mut def = w.species.get(id).creature.as_ref().expect("creature").clone();
        def.climbs_over_kin = on;
        w.species.set_creature(id, def.clone());
        def
    }

    /// **A nestmate is ground, but only for a species that says so** —
    /// WP-9 arm 1, and the deliberate re-test of dead ends 775/829.
    ///
    /// Both arms run on a scene with **no terrain under the head at all**,
    /// so the only thing that could possibly be a foothold is the other
    /// ant. A version of this with a floor would pass whatever the flag
    /// does, which is the scene error this file has paid for repeatedly.
    #[test]
    fn a_living_nestmate_is_a_foothold_only_for_a_species_that_climbs() {
        let footing = |on: bool| -> bool {
            let mut w = test_world();
            // A short ledge for the nestmate to stand on, and nothing at
            // all beneath the climber's head.
            for x in 104..112 {
                w.set(x, 101, Cell::new(material::STONE, 0));
            }
            // The climber stands on its *own* ledge well away from the
            // query point, so `kin_footing` is exercised with a real
            // organism and the climber's own body is nowhere near the cell
            // being asked about.
            for x in 130..138 {
                w.set(x, 101, Cell::new(material::STONE, 0));
            }
            let def = set_climbs_over_kin(&mut w, "ant", on);
            let mate = spawn(&mut w, "ant", 106, 100);
            let climber = spawn(&mut w, "ant", 132, 100);
            assert!(w.organism(mate).is_some() && w.organism(climber).is_some(), "both ants must place");
            // (104, 99) is diagonally above the nestmate's tail at (105,
            // 100) and has no terrain in its own neighbourhood.
            let head = (104, 99);
            let ant_material = w.materials.id_of("ant").expect("ant material");
            assert_eq!(w.get(105, 100).material, ant_material, "the scene must put a nestmate cell in reach of the test position");
            assert!(
                !NEIGHBOURS_8.iter().any(|&(dx, dy)| matches!(
                    w.materials.kind(w.get(head.0 + dx, head.1 + dy).material),
                    MaterialKind::Solid | MaterialKind::Powder | MaterialKind::Plant
                )),
                "test setup: terrain in reach of the head would make both arms pass for the wrong reason"
            );
            head_has_foothold(&w, head, kin_footing(&w, climber, &def))
        };

        assert!(!footing(false), "by default a creature is not ground: dead ends 775/829 hold");
        assert!(footing(true), "with climbs_over_kin a nestmate is ground -- if this fails the re-test measured nothing");
    }

    /// **Can `(Crowding, Tumble, w)` demonstrate itself?** — the pre-flight
    /// for WP-9 arm 2, run *before* the sweep rather than after it.
    ///
    /// Arm 2 needs no code at all: both slots exist, so the whole arm is
    /// one line of `ant.ron` and a sweep over `w`. What it does need is for
    /// the pathway to be live, and this repo has twice spent a sweep to
    /// discover that it was not — the `include_str!` arms that came back
    /// byte-identical, and the megastudy whose eight logs were one
    /// population. `CLAUDE.md`'s rule is to check a planned step can
    /// demonstrate itself before promising it will.
    ///
    /// So: a crowd, and `tumbles` against `w`. If the column does not move,
    /// the sweep would measure nothing and the arm is blocked on something
    /// else — which is much cheaper to learn here.
    ///
    /// **A measurement, not a guard.** The *value* of `w` is a foraging
    /// claim and belongs to `forage_probe seeds=8` (WP-4), which is not on
    /// `main`; nothing here picks one.
    #[test]
    #[ignore = "a measurement, not a guard -- prints numbers, asserts nothing"]
    fn print_crowding_tumble_pathway_check() {
        println!("     w   tumbles   moves   blocked");
        for w_gain in [0.0f32, 0.5, 1.0, 2.0] {
            let mut w = test_world();
            for x in 60..160 {
                w.set(x, 101, Cell::new(material::STONE, 0).with_attached(true));
            }
            let species = w.species.id_of("ant").expect("ant species");
            let def = w.species.get(species).creature.as_ref().expect("creature").clone();
            // The authored instincts plus the one under test. **`(Crowding,
            // Move, -0.3)` is left exactly as authored** -- ablating it cost
            // 69% of deliveries (13f), and this arm is about adding a
            // re-orientation drive, not about removing that one.
            let mut instincts = def.instincts.clone();
            instincts.push(brain::Instinct(brain::BrainInput::Crowding, brain::BrainOutput::Tumble, w_gain));
            w.species.set_genome(species, brain::genome_from_wiring(&instincts, &def.hidden_wiring, &def.hidden_outputs, &def.recurrence));

            // Shoulder to shoulder on purpose: the crowd is the input.
            for x in (70..150).step_by(2) {
                if let Some(site) = plant_creature_seed(&mut w, x, 100, "ant") {
                    w.schedule_active_site(site);
                }
            }
            run(&mut w, 6_000);
            let st = w.creature_stats;
            println!("  {w_gain:>4.1}   {:>7}   {:>5}   {:>7}", st.tumbles, st.moves, st.moves_blocked);
        }
    }

    /// **An ant is never its own ladder**, and this is the whole safety of
    /// arm 1.
    ///
    /// A chain's own tail sits permanently inside its head's
    /// 8-neighbourhood, so a kin rule that did not exclude `self` would
    /// give every ant a foothold in mid-air, forever — the same shape as
    /// the out-of-bounds `BEDROCK` sentinel that turned the world edge into
    /// an infinitely tall ladder, except that this ladder would follow the
    /// animal.
    #[test]
    fn a_climbing_ant_is_never_its_own_foothold() {
        let mut w = test_world();
        let def = set_climbs_over_kin(&mut w, "ant", true);
        // Nothing in the world at all: no floor, no nestmate.
        let ant = spawn(&mut w, "ant", 100, 100);
        let head = w.organism(ant).expect("live").chain[0];
        assert_eq!(w.get(99, 100).organism_id(), ant, "its own tail should be beside its head, which is the situation under test");
        assert!(
            !head_has_foothold(&w, head, kin_footing(&w, ant, &def)),
            "an ant alone in the sky must have no foothold -- its own body must not count"
        );
    }

    /// **The eat verb pays the filter, not the cell's face value** — the
    /// half of S5 that is easy to leave out, and was.
    ///
    /// The first version of this change wired `gut_bias` into the two
    /// *predicates* (the eye and the menu) and left both credit sites
    /// reading `food_value`. Every shipped food still cleared the threshold
    /// at the shipped neutral gut, so the menu did not move — and with the
    /// payout unchanged, nothing else could either. `ascii` reported it in
    /// the form `CLAUDE.md` names as the tell: output byte-identical to
    /// `main` across a change that had to move something.
    ///
    /// Measured through the ledger rather than through an energy delta,
    /// because metabolism drains the bank between bites and would make the
    /// comparison a subtraction of two moving numbers. `harvested_plant`
    /// over `eats` is exactly the per-bite gain: both eat paths book to it
    /// and both bump `eats`.
    ///
    /// Paired across two guts on one scene, so the filter's *ratio* is the
    /// assertion and nothing about the scene has to be held constant by
    /// hand.
    #[test]
    fn the_eat_verb_pays_the_filter_not_the_face_value() {
        let gain_per_bite = |bias: f32| -> f64 {
            let mut w = test_world();
            let soil = w.materials.id_of("soil").expect("soil");
            for x in 70..150 {
                for y in 111..120 {
                    w.set(x, y, Cell::new(soil, 0).with_attached(true));
                }
            }
            // An inexhaustible wall of leaf, row 109 left clear for the ant
            // -- burying the animal reports "the scene lost the situation"
            // rather than measuring anything.
            let leaf = w.materials.id_of("leaf").expect("leaf");
            for x in 100..122 {
                for y in [104, 105, 106, 107, 108, 110] {
                    w.set(x, y, Cell::new(leaf, 0).with_attached(true));
                }
            }
            set_gut(&mut w, "ant", bias);
            run(&mut w, 2_000);
            let ant = spawn(&mut w, "ant", 110, 109);
            assert_ne!(ant, 0, "the grazer was not placed; the scene does not contain the situation this test is about");
            // Start it hungry: `act` takes the eat branch only below
            // `hunger_fraction`, and a full ant picks one leaf up and then
            // has no reason to eat again.
            w.organism_mut(ant).expect("live").energy = 300.0;
            run(&mut w, 20_000);
            let st = w.creature_stats;
            assert!(st.eats > 0, "no bite was taken at gut {bias}, so this arm measures nothing");
            w.energy_ledger.harvested_plant / st.eats as f64
        };

        // Leaf is `food_class: -1.0`. A herbivore gut matches it exactly
        // (filter 1.0); a neutral gut is half the axis away (filter 0.25).
        // Asserted against the leaf's own face value rather than a literal,
        // so an economy retune moves both sides together and only a change
        // to the *filter* can fail this.
        let face = {
            let w = test_world();
            w.materials.get(w.materials.id_of("leaf").expect("leaf")).food_energy as f64
        };
        let herbivore = gain_per_bite(-1.0);
        let generalist = gain_per_bite(0.0);
        assert!((herbivore - face).abs() < 0.5, "a matched gut should get a leaf's whole {face}, got {herbivore:.2}");
        assert!((generalist - face * 0.25).abs() < 0.5, "a neutral gut should get a quarter of it ({}), got {generalist:.2}", face * 0.25);
    }

    /// **And not its own tail either.** `adjacent_food` scans the head's
    /// 8-neighbourhood, which always contains the next link of the animal's
    /// own chain, so "will not eat kin" has to cover "will not eat me".
    ///
    /// The `food` name list was the only thing preventing this before S5 —
    /// by the accident that "ant" was not on the ant's own menu — and
    /// deleting it without the kin rule would have had every carnivorous
    /// ant in the world eating itself from the tail forward.
    #[test]
    fn a_creature_does_not_eat_its_own_tail() {
        let mut w = test_world();
        for x in 90..112 {
            w.set(x, 101, Cell::new(material::STONE, 0));
        }
        let id = w.species.id_of("ant").expect("ant");
        let mut def = w.species.get(id).creature.as_ref().expect("creature").clone();
        def.traits[TRAIT_GUT_BIAS] = 1.0;
        w.species.set_creature(id, def.clone());
        let ant = spawn(&mut w, "ant", 100, 100);
        let ant_material = w.materials.id_of("ant").expect("ant material");
        assert_eq!(w.get(99, 100).material, ant_material, "the chain's second cell should be west of the head");
        // **Stated as the filter, not as a number.** These asserted the
        // literal 120 of the pre-S5 economy and all three broke together
        // when the food scale moved to 480 -- which is `CLAUDE.md`'s
        // "a fix that changes what a number *means* has to re-derive the
        // constants that read it", arriving as three red tests. Written
        // against the material's own face value, they assert the claim
        // (a perfect match pays in full) and survive the next retune.
        let face = w.materials.get(ant_material).food_energy;
        assert_eq!(diet_yield(&w, w.get(99, 100), 1.0), face, "its own tail is nutritious on the diet axis alone -- which is exactly why liveness has to be asked separately");

        assert!(adjacent_food(&w, 100, 100, gut_of(&w, ant, &def)).is_none(), "an ant must not see its own tail as food");
    }

    /// **A starved nestmate's corpse is still dinner**, which is the case a
    /// threshold-based cannibal fix would have deleted.
    ///
    /// A starved animal dies at exactly 0, so its corpse cell is worth its
    /// `body_energy` and nothing more -- 120, the same number as the live
    /// flesh the test above forbids. The separation is liveness, not worth,
    /// and this asserts the half that has to keep working: S3's structural
    /// stamp exists so that closing §13l's pump did not also delete the
    /// scavenger niche, and a gut that cannot eat a starved corpse deletes
    /// it after all.
    #[test]
    fn a_starved_nestmates_corpse_is_still_dinner() {
        let mut w = test_world();
        for x in 92..112 {
            w.set(x, 101, Cell::new(material::STONE, 0));
            w.set(x, 96, Cell::new(material::STONE, 0));
        }
        for y in 96..102 {
            w.set(92, y, Cell::new(material::STONE, 0));
            w.set(111, y, Cell::new(material::STONE, 0));
        }
        let corpse = w.materials.id_of("corpse").expect("corpse");
        let body_energy = {
            let id = w.species.id_of("ant").expect("ant");
            w.species.get(id).creature.as_ref().expect("creature").body_energy
        };
        // The starved stamp exactly: `body_energy * cells + 0` over cells.
        let starved = Cell::new(corpse, 0).with_aux(body_energy.round() as u16);
        assert_eq!(
            diet_yield(&w, starved, 0.0),
            body_energy * 0.25,
            "a starved corpse at a neutral gut is a quarter of the stamp -- the filter reads 0.25 half an axis away, and if this moved, the filter or the stamp did"
        );
        assert!(diet_yield(&w, starved, 0.0) > EAT_YIELD_THRESHOLD, "and it has to clear the bar, or the scavenger niche is gone");

        // **Deliberately the *authored* gut, not an overridden one.** For
        // one commit the ant was a detritivore that could not digest
        // carrion at all, and this test had to set a gut of its own to
        // measure anything. The owner's verdict on review card
        // 20260823T104411499Z-963f8d -- "An omnivore should be viable" --
        // put the shipped ant back at neutral, so scavenging is live
        // behaviour again and this asserts it as such. If a future retune
        // narrows the ant off the flesh end, this failing is the point.
        let ant = spawn(&mut w, "ant", 100, 100);
        for x in 101..104 {
            w.set(x, 100, starved);
        }
        let before = w.creature_stats.eats + w.creature_stats.pickups;
        run(&mut w, 1200);
        assert!(w.creature_stats.eats + w.creature_stats.pickups > before, "the ant should have taken the carrion it is standing in");
        assert!(w.organism(ant).is_some(), "and should not have starved doing it");
    }

    #[test]
    fn a_swallowed_cell_does_not_come_back_as_a_corpse() {
        // **The guard the test above looks like it already is, and is not.**
        // It asserts no *ant* cell is left standing; the bug left a *corpse*
        // cell standing, in the very cell the predator had just emptied,
        // because `creature_dies` wrote over the stale `chain` rather than
        // over the cells the organism still owned. A two-cell ant whose head
        // was eaten produced two corpse cells out of two -- one of them
        // conjured out of nothing, and edible again, by a beetle whose food
        // list includes "corpse".
        //
        // Driven through the seam directly rather than through a live
        // predator, because a beetle that has to *find* the ant makes this
        // depend on foraging (the mistake the test above records twice).
        let mut w = test_world();
        for x in 92..112 {
            w.set(x, 101, Cell::new(material::STONE, 0));
        }
        let ant = spawn(&mut w, "ant", 100, 100);
        let chain = w.organism(ant).expect("live").chain.clone();
        assert_eq!(chain.len(), 2, "the ant is a two-cell chain; this test is about losing one of them");
        let corpse = w.materials.id_of("corpse").expect("corpse is compiled in");

        // Exactly what `act`'s eat branch does to its victim.
        let (hx, hy) = chain[0];
        w.set(hx, hy, Cell::EMPTY);
        assert!(!reconcile_chain(&mut w, ant), "losing the head is death, not an injury");

        let corpses = chain.iter().filter(|&&(x, y)| w.get(x, y).material == corpse).count();
        assert_eq!(corpses, 1, "one cell was swallowed, so one corpse cell may remain -- {corpses} means the mouthful was resurrected");
        assert_eq!(w.get(hx, hy).material, material::EMPTY, "the swallowed cell stays swallowed");
    }

    #[test]
    fn a_corpse_is_worth_what_the_animal_was_made_of() {
        // **The keystone, at the one seam where meat is created.** Under
        // `eat_energy` a corpse cell was worth whatever *bit* it, so an ant
        // granted 900, spending all of it and starving at exactly 0, left
        // two cells worth 120 each: 240 energy out of an animal that had
        // none (§13l). The worth now comes out of the body's own stamp,
        // granted and booked at spawn.
        let mut w = test_world();
        for x in 92..112 {
            w.set(x, 101, Cell::new(material::STONE, 0));
        }
        let corpse = w.materials.id_of("corpse").expect("corpse");
        let species = w.species.id_of("ant").expect("ant species");
        let def = w.species.get(species).creature.as_ref().expect("creature").clone();

        // A starved animal: dead at exactly zero, which is the case that
        // decides whether closing the pump also deletes the scavengers.
        let starved = spawn(&mut w, "ant", 100, 100);
        w.organism_mut(starved).expect("live").energy = 0.0;
        creature_dies(&mut w, starved);
        let starved_worth: Vec<u16> = (92..112)
            .flat_map(|x| (95..102).map(move |y| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == corpse)
            .map(|(x, y)| w.get(x, y).aux())
            .collect();
        assert!(!starved_worth.is_empty(), "a dead ant leaves meat");
        for worth in &starved_worth {
            assert_eq!(*worth as f32, def.body_energy, "a starved animal is worth exactly the body it was built from, and no more");
        }

        // And one killed with energy still in the bank is worth more, which
        // is what makes a fresh kill better eating than carrion.
        let full = spawn(&mut w, "ant", 104, 100);
        w.organism_mut(full).expect("live").energy = 400.0;
        creature_dies(&mut w, full);
        let full_worth = w.get(104, 100).aux();
        assert!(
            (full_worth as f32) > def.body_energy,
            "an animal killed in its prime carries its unspent energy into its corpse ({full_worth} vs body {})",
            def.body_energy
        );
    }

    /// **A mouthful in transit keeps what it is worth.**
    ///
    /// Neither conservation guard can see this one, and that is the reason
    /// it exists: losing the payload is a *sink*, biomass stays monotone,
    /// the meat ceiling stays satisfied, and the world quietly gets poorer
    /// along the one path a colony is built around -- carrying something
    /// home. Both drop sites used to write `Cell::new(held, 0)`, whose
    /// second argument is *shade*, not `aux`; so before S3b a corpse worth
    /// 640 came back down worth zero and black.
    /// **An ant that burns to death still feeds the next colony**, which is
    /// a promise `wiki/ants.md` makes in as many words: "ants that die in a
    /// fire become the next colony's dinner".
    ///
    /// S3 quietly broke it and nothing noticed, because the break was in an
    /// accounting change and the mechanic it deleted is three files away.
    /// A corpse says `worth_in_aux`, so its value comes from `Cell::aux`;
    /// `fire.rs`'s burnout is generic over every flammable material and
    /// writes `Cell::new(into, shade)` with `aux` 0. Corpse's static
    /// `food_energy` was 0 as well, on the stated grounds that "there is no
    /// path today" for an unstamped corpse -- and `ant.ron`'s
    /// `burns_into: "corpse"` is exactly that path.
    ///
    /// The test is deliberately written against the *material data*, not
    /// against `fire.rs`: what it is really asserting is that no route into
    /// a corpse cell can produce meat worth nothing, and a second such route
    /// (an explosion, the brush, a future decay path) would be caught by the
    /// same assertion without anyone remembering to extend it.
    /// **A body that arrives without a stamp must not look like a rich one.**
    ///
    /// The companion to `a_corpse_that_arrived_without_a_stamp_is_still_food`,
    /// and it exists because widening the corpse ramp created the bug it
    /// guards. `fire.rs`'s burnout draws a *random* shade, which is right for
    /// ash and decoration and wrong for the one material whose shade is
    /// derived: `creature_dies` ramps corpse brightness over what the animal
    /// was worth. While the palette was three near-identical browns nobody
    /// could have seen it; the moment the ramp spans something legible, a
    /// random draw renders a burnt ant as a prime kill one time in five.
    ///
    /// **Drives the real burnout**, and the first version of this test did
    /// not — it built `Cell::new(corpse, 0)` by hand and asserted the shade
    /// was 0, which is a test of its own literal. It passed with `fire.rs`
    /// reverted to the random draw, i.e. it would have shipped the bug it was
    /// written for. `CLAUDE.md` twice: a green suite does not prove a test
    /// ran, and deliberately break the replacement to confirm the guard bites.
    ///
    /// Ants burn over many frames, so the shade is checked over enough
    /// independent burnouts that a random draw could not miss the light end
    /// by luck: with five palette entries, twenty bodies all landing dark is
    /// a 1-in-5^20 coincidence.
    #[test]
    fn a_burnt_body_is_shaded_as_the_poor_meat_it_is() {
        let mut w = test_world();
        let corpse = w.materials.id_of("corpse").expect("corpse");
        let ant_material = w.materials.id_of("ant").expect("ant");
        let shades = w.materials.get(corpse).palette.len();
        assert!(shades >= 3, "test setup: a one-entry palette cannot express rich or poor, so this would pass without meaning it");
        assert_eq!(
            w.materials.get(ant_material).burns_into,
            Some(corpse),
            "test setup: this test is about the path where an ant burns into a corpse, and that is not what the data says"
        );

        for i in 0..20 {
            let (x, y) = (20 + i * 3, 40);
            let mut cell = Cell::new(ant_material, 0);
            cell.ignite(1); // ticks to 0 and burns out on the next update
            w.set(x, y, cell);
            crate::sim::fire::update(&mut w, x, y);
            let left = w.get(x, y);
            assert_eq!(left.material, corpse, "test setup: the ant did not burn out into a corpse at all ({i})");
            assert_eq!(left.aux(), 0, "a burnout cannot stamp a worth; if this is nonzero the fallback path is no longer the one under test");
            assert_eq!(
                left.shade, 0,
                "a burnt body carries no stamp, so it is worth the material fallback -- the poorest a body can be -- and must be                 drawn at the dark end of the ramp. Body {i} came out at shade {} of {}, which reads as a fresh kill",
                left.shade,
                shades - 1
            );
        }

        // And the ramp has to actually go somewhere, or "poor" and "rich" are
        // the same pixel and none of this matters. 60 is comfortably above the
        // ~20 the narrow palette spanned, which a blind A/B at 10x zoom showed
        // was one colour.
        let pal = &w.materials.get(corpse).palette;
        let (poor, rich) = (pal[0], pal[pal.len() - 1]);
        let span = rich[0] as i32 - poor[0] as i32;
        assert!(
            span >= 60,
            "the corpse ramp spans {span} of 255 in red: the order of difference the canopy-density sheet read as blank,             so worth is not being communicated at all (poor {poor:?}, rich {rich:?})"
        );
    }

    #[test]
    fn a_corpse_that_arrived_without_a_stamp_is_still_food() {
        let w = test_world();
        let corpse = w.materials.id_of("corpse").expect("corpse");
        // Exactly what `fire.rs`'s burnout writes: the target material, a
        // shade drawn from its palette, and no `aux` at all.
        for shade in 0..w.materials.get(corpse).palette.len() as u8 {
            let burnt = Cell::new(corpse, shade);
            assert_eq!(burnt.aux(), 0, "test setup: this is the unstamped case, and a stamped cell would prove nothing");
            assert!(
                food_value(&w, burnt) > 0.0,
                "a corpse that arrived by fire rather than by starving is worth nothing to eat at shade {shade}, so an ant                 that burns feeds no one -- see wiki/ants.md, which promises the opposite"
            );
        }
        // And a stamped corpse still overrides the fallback rather than
        // being flattened onto it, which is the whole point of the flag.
        let ant_body = w.materials.get(corpse).food_energy;
        let stamped = Cell::new(corpse, 0).with_aux(760);
        assert_eq!(food_value(&w, stamped), 760.0, "a stamped corpse is worth its stamp, not the {ant_body} fallback");
    }

    #[test]
    fn a_carried_corpse_keeps_its_worth_when_it_is_put_down() {
        let mut w = test_world();
        for x in 92..112 {
            w.set(x, 101, Cell::new(material::STONE, 0));
        }
        let corpse = w.materials.id_of("corpse").expect("corpse");
        let ant = spawn(&mut w, "ant", 100, 100);
        assert_ne!(ant, 0, "the carrier was not placed; the scene does not contain the situation this test is about");
        w.organism_mut(ant).expect("live").carrying = Some(Carried { material: corpse, worth: 640, shade: 3 });

        creature_dies(&mut w, ant);

        // The corpse the *carrier* became is worth its own stamp; the cargo
        // is the one worth 640, so look for that value rather than for the
        // material.
        let dropped: Vec<Cell> = (92..112)
            .flat_map(|x| (95..102).map(move |y| (x, y)))
            .map(|(x, y)| w.get(x, y))
            .filter(|c| c.material == corpse && c.aux() == 640)
            .collect();
        assert_eq!(dropped.len(), 1, "a corpse cell worth 640 was being carried and exactly one should have landed, found {}", dropped.len());
        assert_eq!(dropped[0].shade, 3, "the cargo's shade travels with it, or a fat corpse is put down looking like a picked-over one");
    }

    /// **The `aux` convention is a property of the material, not of the
    /// payload.** `Cell::aux` is a tagged union with three readings in it
    /// now, and the drop path is the one place a value crosses from one
    /// material to another. Writing the worth unconditionally puts 120 into
    /// a leaf's `aux`, and on a `Powder` that reads as soil water -- food
    /// turned into water, which is the mistake `Cell::aux`'s own doc
    /// comment warns about from the other direction.
    #[test]
    fn putting_down_a_leaf_does_not_fill_it_with_water() {
        let w = test_world();
        let leaf = w.materials.id_of("leaf").expect("leaf");
        assert!(!w.materials.get(leaf).worth_in_aux, "test setup: this test is about a material that does *not* bank its worth");
        let cell = Carried { material: leaf, worth: 120, shade: 2 }.into_cell(&w);
        assert_eq!(cell.aux(), 0, "a leaf put down must carry no aux payload at all");
        assert_eq!(cell.shade, 2, "shade still travels; it is only the worth that must not be written");
    }

    #[test]
    fn a_dying_carrier_leaves_its_cargo_somewhere_it_can_be_found() {
        // The sentence in `creature_dies` promising this was true and the
        // code under it was not: the corpse loop had already filled the
        // chain's own cells, so the `is_empty` check on `chain.last()` was
        // false every time and the cargo was silently deleted. A material
        // sink that only fires on death is exactly the kind the census was
        // built to catch and could not, because carried material is not
        // energy and never entered the ledger at all.
        let mut w = test_world();
        for x in 92..112 {
            w.set(x, 101, Cell::new(material::STONE, 0));
        }
        let ant = spawn(&mut w, "ant", 100, 100);
        let leaf = w.materials.id_of("leaf").expect("leaf is compiled in");
        w.organism_mut(ant).expect("live").carrying = Some(Carried { material: leaf, worth: 0, shade: 0 });
        let before = (90..115).flat_map(|x| (90..102).map(move |y| (x, y))).filter(|&(x, y)| w.get(x, y).material == leaf).count();

        creature_dies(&mut w, ant);

        let after = (90..115).flat_map(|x| (90..102).map(move |y| (x, y))).filter(|&(x, y)| w.get(x, y).material == leaf).count();
        assert_eq!(after, before + 1, "the load a dead carrier was holding has to land somewhere, not evaporate");
    }

    /// The sealed box the three conservation guards below all run in:
    /// stone walls, no producers, no light, twelve ants and nothing else.
    ///
    /// **Ants only, and that is load-bearing.** `ant.ron`'s food list has no
    /// `ant` on it, so nothing here eats living flesh -- which is the one
    /// path that still books a free term (`standing_meat`'s doc says why).
    /// Adding a beetle to this scene would loosen the bound these tests
    /// assert, and it would look like the tests had found something.
    fn sealed_box() -> World {
        let mut w = test_world();
        for x in 60..140 {
            w.set(x, 121, Cell::new(material::STONE, 0));
            w.set(x, 100, Cell::new(material::STONE, 0));
        }
        for y in 100..122 {
            w.set(60, y, Cell::new(material::STONE, 0));
            w.set(139, y, Cell::new(material::STONE, 0));
        }
        for i in 0..12 {
            spawn(&mut w, "ant", 64 + i * 6, 120);
        }
        w
    }

    const SEALED_BOX: Rect = Rect { min_x: 60, min_y: 100, max_x: 139, max_y: 121 };

    /// Live *animals*, not live cells. The version this replaced counted
    /// occupied cells and so read 24 for twelve two-cell ants -- harmless
    /// for the `== 0` bar it was written under, and wrong the moment a
    /// setup assertion asked it how many ants there were.
    fn alive_in_box(w: &World) -> usize {
        w.live_organism_count()
    }

    /// **A sealed world with no renewable food must run down.**
    ///
    /// This was `#[ignore]`d as the live reproduction of the pump in
    /// `Reports/creature-direction.md` §13l, and un-ignoring it is what S3
    /// was for. The pump: `eat_energy` was a constant of the **eater**, and
    /// a corpse cell was worth it in full no matter what the creature that
    /// left it had in the bank. An ant granted 900 spent all of it, starved
    /// at exactly 0, and laid down two corpse cells worth 120 each -- 240
    /// energy out of an animal that had none. The colony then ran on its own
    /// dead forever. §13i measured the symptom ("a colony sustains itself on
    /// its own dead without foraging at all") and read it as an ecology
    /// problem; it was an accounting one, and `creature_space` only escaped
    /// it by taking "corpse" off the menu.
    ///
    /// What closed it: a corpse is worth the body it was made of plus
    /// whatever was left unspent, and the body's worth is **granted and
    /// booked at spawn** rather than conjured at the far end. Eating it is a
    /// transfer out of that stock, not a fresh 120.
    /// **The horizon is measured, not chosen, and it moved once already.**
    /// 40,000 frames was inherited from the version of this test that never
    /// passed, and it went red the moment a *hungry carrier eats its load*
    /// -- not because anything created energy (the ledger closed exactly:
    /// 2,880 stamped, 1,680 eaten, 720 standing, 480 still inside the two
    /// survivors) but because a colony that no longer starves holding food
    /// recovers more of its own dead, and the same finite budget lasts
    /// longer. Measured emptying: 12 alive at 15,000, 3 at 20,000, 2 at
    /// 40,000, 1 at 45,000, **0 at 50,000**. 80,000 is that with 60%
    /// headroom.
    ///
    /// This is the knife-edge weakness of an outcome count, in the flesh: it
    /// says nothing about how far below 1.0 the loop's gain is, so it can go
    /// red for an *improvement*. `a_sealed_colony_never_grows_its_own_biomass`
    /// is the guard that actually watches for creation, and it stayed green
    /// through this whole episode.
    #[test]
    fn a_sealed_world_with_no_food_source_runs_down() {
        let mut w = sealed_box();
        let start = alive_in_box(&w);
        assert_eq!(start, 12, "test setup: twelve ants, or this is measuring an empty box");
        run(&mut w, 80_000);
        let end = alive_in_box(&w);
        assert_eq!(end, 0, "no energy enters this world, so nothing in it may still be alive after 80,000 frames ({start} -> {end}).           Measured emptying was 50,000; if this is red, check the biomass guard before assuming a pump");
    }

    /// The same property as a *quantity*, which is the version that keeps
    /// working once something in the box is renewable.
    ///
    /// "Everything died" is a knife-edge outcome: it passes the moment the
    /// loop's gain drops below 1.0 and says nothing about how far below.
    /// `CLAUDE.md` prefers a continuous quantity over a count of bad cells,
    /// and the continuous version here is total biomass -- see
    /// `creature_biomass` for why the promised-stamp term has to be in it.
    #[test]
    fn a_sealed_colony_never_grows_its_own_biomass() {
        let mut w = sealed_box();
        let mut previous = creature_biomass(&w, SEALED_BOX);
        let opening = previous;
        assert!(opening > 0.0, "test setup: an empty box cannot fail this test for the right reason");
        for window in 0..40 {
            run(&mut w, 1_000);
            let now = creature_biomass(&w, SEALED_BOX);
            assert!(
                now <= previous + 1e-6,
                "biomass rose from {previous:.2} to {now:.2} over window {window}: something in this box is creating value out of nothing"
            );
            previous = now;
        }
        assert!(previous < opening, "40,000 frames of metabolism have to cost something ({opening:.2} -> {previous:.2})");
    }

    /// **The standing meat never exceeds what was put into it.** The ledger
    /// half of the same claim, and the one that would still fire if a future
    /// drop path re-created a corpse cell without its payload: biomass alone
    /// reads that as a gain in the world, and this reads it as meat that no
    /// death paid for.
    #[test]
    fn the_standing_meat_never_exceeds_what_was_put_into_it() {
        let mut w = sealed_box();
        // The identity's drift *before anything has died or eaten*, which is
        // a clean baseline for the charge path alone.
        //
        // **Found rather than fitted.** This was `run(&mut w, 10_000)` with
        // a comment saying the first death lands "around frame 12,000", and
        // E14's `start_energy` cut (900 -> 200) moved the first death to
        // roughly a fifth of that and took the test red with 9 deaths inside
        // its own baseline window. The frame count was a constant calibrated
        // against an animal that could not starve; the *property* the
        // baseline needs is "the last sample before anything died", so ask
        // for that directly and the test survives the next retune too.
        let mut baseline_drift = 0.0;
        let mut baseline_at = 0;
        for _ in 0..40 {
            let probe = (w.live_creature_energy() - w.energy_ledger.expected_live_total()).abs();
            run(&mut w, 500);
            if w.creature_stats.deaths > 0 {
                break;
            }
            baseline_drift = probe;
            baseline_at += 500;
        }
        assert!(
            baseline_at >= 1_000,
            "test setup: the first death landed within {baseline_at} frames, leaving no pre-death window to take a baseline in"
        );
        for _ in 0..30 {
            run(&mut w, 1_000);
            let standing = standing_meat(&w, SEALED_BOX) + carried_meat(&w);
            let ceiling = w.energy_ledger.max_standing_meat();
            assert!(
                standing <= ceiling + 1e-6,
                "{standing:.2} of meat is standing in a box that only ever had {ceiling:.2} put into it"
            );
        }
        // And the live identity still closes, which is the *other* thing the
        // ledger can see and this one cannot: a charge debited but never
        // taken off a creature.
        //
        // **Against the run's own baseline, not against zero.** Creature
        // energy is `f32` and the ledger is `f64`, so roughly a million
        // charges of 0.10 and 0.25 against a budget near 900 leave a
        // rounding residue no assertion can honestly forbid -- measured at
        // 0.298 over the first 10,000 frames, with **zero deaths and zero
        // eats in them**. That residue is not what this test is for. What it
        // is for is the death and eat paths, and the way to see those is
        // that the drift does not *grow* once they start firing: measured
        // 0.298 before any death and 0.286 after twelve of them, i.e. it
        // wobbles by hundredths and does not accumulate. The slack below is
        // 1.0 -- about sixty times that wobble, and still a third of what
        // the cheapest systematic hole would cost (one `move_cost` of 0.25
        // unbooked per death, over these twelve deaths, is 3.0).
        //
        // A fixed absolute bar was tried first and is the wrong instrument:
        // it cannot tell a rounding residue from a leak, because it never
        // looks at the same world twice.
        let delta = (w.live_creature_energy() - w.energy_ledger.expected_live_total()).abs();
        assert!(
            delta <= baseline_drift + 1.0,
            "the live energy identity drifted from {baseline_drift:.6} before the first death to {delta:.6} after {} deaths and {} meals:               a charge on one of those paths is not landing",
            w.creature_stats.deaths,
            w.creature_stats.eats
        );
    }

    /// Which larder the grazer scene is stocked with.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Larder {
        /// A moss lawn: finite at any instant, and free to regrow.
        Renewable,
        /// A static wall of leaf that is never exhausted inside the run.
        /// **The control**, and the whole reason the renewable arm can be
        /// believed -- it puts a ceiling on what one stationary animal can
        /// physically get into its mouth in the time available, which is a
        /// quantity nothing else in this codebase measures.
        Unlimited,
    }

    /// A damp soil bed, a larder, and one hungry ant standing on it.
    /// **How long a grazer run is, in units of the animal's own life.**
    ///
    /// This was the literal `60_000` at both call sites, which was about
    /// 1.1 idle lifetimes against the `start_energy: 900` it was written
    /// under. E14 cut that budget to 200 and the same 60,000 frames became
    /// **five** lifetimes -- so the *control* arm starved, the setup
    /// assertion fired, and the test reported "this scene cannot feed
    /// anything" about a scene that was fine. A frame count is not a
    /// horizon; a horizon is a number of lifetimes, and this computes one
    /// so the next retune of the economy does not take it red again.
    ///
    /// An idle lifetime is `start_energy / idle_cost` ticks of
    /// `tick_interval` frames each. 1.1 of them keeps the meaning the
    /// original number had.
    fn grazer_horizon() -> usize {
        let w = test_world();
        let species = w.species.id_of("ant").expect("ant");
        let def = w.species.get(species).creature.as_ref().expect("creature");
        // **Per cell since 2026-08-30.** `idle_cost_per_cell * body.len()`
        // is the old whole-animal `idle_cost`, so this horizon is unchanged
        // for the shipped two-cell ant -- but it is now a function of the
        // body, and a species with a longer one has a proportionally
        // shorter one. Written out rather than folded so that is visible.
        let idle_per_tick = def.idle_cost_per_cell * def.body.len() as f32;
        let idle_life = (def.start_energy / idle_per_tick) as usize * def.tick_interval as usize;
        idle_life * 11 / 10
    }

    fn grazer_scene(larder: Larder, frames: usize) -> World {
        let mut w = test_world();
        let soil = w.materials.id_of("soil").expect("soil");
        for x in 70..150 {
            for y in 111..120 {
                w.set(x, y, Cell::new(soil, 0).with_attached(true));
            }
        }
        match larder {
            // Moss as a live organism, not as painted cells: the pump this
            // is about is moss *regrowing* into a grazed gap for free, and
            // painted cells cannot divide. `plant_moss_seed` is the same
            // entry point `decay.rs` uses.
            Larder::Renewable => {
                for x in (72..148).step_by(4) {
                    w.plant_moss_seed(x, 110);
                }
            }
            // Row 109 is left clear on purpose -- that is where the ant
            // stands, and burying it reports "the scene does not contain
            // the situation" instead of measuring anything.
            Larder::Unlimited => {
                let leaf = w.materials.id_of("leaf").expect("leaf");
                for x in 100..122 {
                    for y in [104, 105, 106, 107, 108, 110] {
                        w.set(x, y, Cell::new(leaf, 0).with_attached(true));
                    }
                }
            }
        }
        run(&mut w, 2_000);
        let ant = spawn(&mut w, "ant", 110, 109);
        assert_ne!(ant, 0, "the grazer was not placed; the scene does not contain the situation this test is about");
        // **Start it hungry, or this measures nothing.** `act` takes the eat
        // branch only below `hunger_fraction` (0.5), and the first version
        // of this scene ran a *full* ant, which promptly picked up one moss
        // cell and then had no reason to eat for the rest of the run.
        w.organism_mut(ant).expect("live").energy = 300.0;
        run(&mut w, frames);
        w
    }

    /// **What does it take to make an omnivore viable?** — the owner's
    /// answer to review card `20260823T104411499Z-963f8d` was, in full,
    /// "An omnivore should be viable", which overturns the detritivore
    /// landing the sweep below forced.
    ///
    /// The sweep below says a *perfectly matched* grazer already runs at
    /// intake/cost 0.946, so the margin is negative before any digestive
    /// penalty and a neutral gut at 0.706 starves. The lever that fixes
    /// that without also making *idling* cheaper — which is §13i's sessile
    /// attractor and the owner's other standing complaint — is the value of
    /// the food, not the cost of living.
    ///
    /// So: scale what a mouthful is worth by `m` and find the smallest `m`
    /// where a neutral gut clears viability with headroom. Both halves of
    /// the axis scale together (`body_energy` with the plant foods), the
    /// way `creature_space` already scales them, so a scavenged mouthful
    /// stays worth a foraged one.
    #[test]
    #[ignore = "a measurement, not a guard -- prints numbers, asserts nothing"]
    fn print_omnivore_viability_against_food_scale() {
        println!("    m    gut   leaf yield   ratio   survived");
        for m in [1.0f32, 1.5, 2.0, 3.0, 4.0] {
            for bias in [0.0f32, -0.5, -1.0] {
                let mut w = test_world();
                let soil = w.materials.id_of("soil").expect("soil");
                for x in 70..150 {
                    for y in 111..120 {
                        w.set(x, y, Cell::new(soil, 0).with_attached(true));
                    }
                }
                let leaf = w.materials.id_of("leaf").expect("leaf");
                for x in 100..122 {
                    for y in [104, 105, 106, 107, 108, 110] {
                        w.set(x, y, Cell::new(leaf, 0).with_attached(true));
                    }
                }
                // The economy knob under test. Every food the ant can reach
                // scales together, plus the structural stamp -- scaling one
                // half of the axis would be a diet change wearing an
                // economy change's clothes.
                for name in ["leaf", "moss", "seed", "litter", "ant"] {
                    if let Some(id) = w.materials.id_of(name) {
                        let base = w.materials.get(id).food_energy;
                        w.materials.get_mut(id).food_energy = base * m;
                    }
                }
                let species = w.species.id_of("ant").expect("ant");
                let mut def = w.species.get(species).creature.as_ref().expect("creature").clone();
                def.traits[TRAIT_GUT_BIAS] = bias;
                def.body_energy *= m;
                w.species.set_creature(species, def);

                run(&mut w, 2_000);
                let ant = spawn(&mut w, "ant", 110, 109);
                w.organism_mut(ant).expect("live").energy = 300.0;
                let yield_per_leaf = diet_yield(&w, Cell::new(leaf, 0), bias);
                run(&mut w, 60_000);
                println!(
                    "  {m:>3.1}   {bias:>4.1}   {yield_per_leaf:>10.1}   {:>5.3}   {}",
                    grazing_ratio(&w),
                    if w.live_organism_count() > 0 { "yes" } else { "no" }
                );
            }
        }
    }

    /// **Where does a gut stop being able to feed the animal that has it?**
    /// A measurement, not a guard — the number the ancestral `gut_bias`
    /// has to be chosen against, taken on the one scene in the suite that
    /// can answer it (an inexhaustible larder, so the only variable left is
    /// digestion).
    #[test]
    #[ignore = "a measurement, not a guard -- prints numbers, asserts nothing"]
    fn print_grazer_viability_against_gut_bias() {
        println!("  gut   leaf yield   ratio   survived");
        for bias in [-1.0f32, -0.9, -0.8, -0.7, -0.6, -0.5, -0.4, -0.2, 0.0] {
            let mut w = test_world();
            let soil = w.materials.id_of("soil").expect("soil");
            for x in 70..150 {
                for y in 111..120 {
                    w.set(x, y, Cell::new(soil, 0).with_attached(true));
                }
            }
            let leaf = w.materials.id_of("leaf").expect("leaf");
            for x in 100..122 {
                for y in [104, 105, 106, 107, 108, 110] {
                    w.set(x, y, Cell::new(leaf, 0).with_attached(true));
                }
            }
            set_gut(&mut w, "ant", bias);
            run(&mut w, 2_000);
            let ant = spawn(&mut w, "ant", 110, 109);
            w.organism_mut(ant).expect("live").energy = 300.0;
            let yield_per_leaf = diet_yield(&w, Cell::new(leaf, 0), bias);
            run(&mut w, 60_000);
            println!(
                "  {bias:>4.1}   {yield_per_leaf:>10.1}   {:>5.3}   {}",
                grazing_ratio(&w),
                if w.live_organism_count() > 0 { "yes" } else { "no" }
            );
        }
    }

    /// Intake over cost-of-living for a finished grazer run. **A ratio of
    /// 1.0 is the pump line**: at 1.0 an animal's food pays for the whole
    /// of its existence, and anything at or above it is perpetual motion
    /// with a mouth attached.
    fn grazing_ratio(w: &World) -> f64 {
        let l = w.energy_ledger;
        let intake = l.harvested_plant + l.harvested_corpse;
        let spent = l.metabolized + l.moved + l.synapse_tax;
        intake / spent.max(1.0)
    }

    /// **The sessile-grazer probe**, and the reason moss's `food_energy` is
    /// not free to choose.
    ///
    /// `moss.ron` divides at `damp_chance: 0.35` for **`cost: 0.0`** -- a
    /// deliberate value and not a placeholder (its own file says so),
    /// because moss predates organisms having energy budgets at all. That
    /// was inert while nutrition was a constant of the eater. The moment
    /// food carries value it is a free renewable, and an animal parked on a
    /// lawn that refills itself is P-20's sessile-freeloading attractor with
    /// a food supply attached. `creature_space` has already watched an
    /// earlier version of this beat every forager before evolution was even
    /// switched on: the zero genome, which cannot move, scored 0.923 against
    /// the forager's 0.735.
    ///
    /// **The bar is not "the grazer dies" alone**, and it is not a number
    /// calibrated off one run either. It is a *paired* comparison against an
    /// unlimited larder, because a lone number here cannot tell a bounded
    /// niche from a scene that could not feed anything in the first place --
    /// which is exactly what the first version of this probe did. It read
    /// 2 eats and a clean intake of 0.0 and looked like a pass; an unlimited
    /// wall of leaf produced **the identical 2 eats and the identical
    /// death**, and the real finding was a bug in `act` (a laden animal
    /// could never eat again). `CLAUDE.md`: when every setting fails the
    /// same way, suspect the sweep.
    ///
    /// If moss ever crosses the pump line, the fix is a per-cell
    /// post-grazing cooldown, **not** shrinking `food_energy` until the
    /// niche disappears -- moss is the only ground-level renewable an ant
    /// can reach at all (§13k/§13n).
    #[test]
    fn a_lone_grazer_cannot_farm_a_moss_lawn_forever() {
        let horizon = grazer_horizon();
        let unlimited = grazer_scene(Larder::Unlimited, horizon);
        let ceiling = grazing_ratio(&unlimited);
        // The control has to actually feed something, or the renewable arm
        // below is measuring the harness.
        assert!(
            unlimited.live_organism_count() > 0,
            "test setup: an animal standing inside an inexhaustible larder starved anyway, so this scene cannot feed anything             and the moss arm proves nothing (ratio {ceiling:.3})"
        );

        let lawn = grazer_scene(Larder::Renewable, horizon);
        let renewable = grazing_ratio(&lawn);

        // Measured 2026-08-21: unlimited 0.95 and survives with 217 in the
        // bank; moss lawn 0.71 and dead. The bar is the pump line itself
        // rather than anything fitted to that 0.71, so it does not need
        // re-blessing every time the economy is retuned -- it fires when
        // grazing starts paying for a whole life, and not before.
        assert!(
            renewable < 1.0,
            "a lawn of free moss paid for {renewable:.3} of one animal's entire cost of living (the unlimited-larder ceiling is             {ceiling:.3}). Grazing is a pump, not a niche. Fix moss with a post-grazing cooldown, not by pricing the niche away"
        );
        assert!(
            renewable <= ceiling,
            "a renewable lawn fed an animal better than an inexhaustible one ({renewable:.3} against {ceiling:.3}), which is             not a fact about moss -- it is a fact about the scene, and the scene is wrong"
        );
    }

    /// **Can an ant climb a tree, and does it?**
    ///
    /// A measurement, not a guard. The whole case for S4 (litter) rests on
    /// §13k/§13n's "ants cannot reach the canopy", and reading `step_chain`
    /// says that is not what the rules do: support is 8-neighbour and
    /// includes `MaterialKind::Plant`, with a comment saying in as many words
    /// that ants climb walls and ceilings. So the barrier may be motivation
    /// rather than ability -- nothing points up, `FoodAdjacent` sees one
    /// cell, and no pheromone trail leads up a tree nobody has climbed.
    ///
    /// This asks the question directly: put ants at the foot of a grown tree
    /// and see how high they get, against the same ants on bare ground. The
    /// bare-ground arm is the control that separates "climbs the tree" from
    /// "wanders upward anyway" -- on flat ground the answer should be ~0, and
    /// if it is not, height above spawn is measuring something else.
    #[test]
    #[ignore = "a measurement, not a guard -- prints numbers, asserts nothing"]
    fn how_high_does_an_ant_climb() {
        // **The full frame order the app runs, not this module's `run`.**
        // `run` steps only the scheduler, so a tree gets no light and never
        // grows: the first version of this experiment reported `tree height
        // 0` and 1-3 cells of "climbing" identical to bare ground, because
        // there was no tree in the scene at all. `creature_space`'s census
        // carries a comment about being caught by the same omission.
        fn live(w: &mut World, frames: usize) {
            for _ in 0..frames {
                crate::sim::parallel::step(w);
                w.step_active_sites();
                w.step_fields();
                w.step_pheromones();
            }
        }
        fn trial(with_tree: bool, seed: u64) -> (i32, i32, u64) {
            let mut w = test_world();
            w.seed = seed;
            let soil = w.materials.id_of("soil").expect("soil");
            let floor = 150;
            for x in 40..160 {
                for y in floor..(floor + 10) {
                    w.set(x, y, Cell::new(soil, 0).with_attached(true));
                }
            }
            if with_tree {
                w.plant_tree(100, floor - 1);
                // Let it become a tree before anyone tries to climb it: a
                // seedling is not a canopy, and an ant at the foot of a
                // two-cell sprout would be measuring nothing.
                live(&mut w, 8_000);
            }
            let trunk_top = (0..floor)
                .find(|&y| w.materials.kind(w.get(100, y).material) == MaterialKind::Plant)
                .unwrap_or(floor);

            // **Clear the spawn row first.** Eight thousand frames of a
            // growing tree carpet the floor in litter (S4), so the cells the
            // ants want are taken and `plant_creature_seed` correctly refuses
            // the whole body -- which is how the first run of this experiment
            // died, in `spawn`, looking like an engine fault. Spaced 6 apart
            // as well: a `Chain(2)` needs the cell behind its head free.
            for i in 0..8 {
                for dx in -1..=1 {
                    let (cx, cy) = (70 + i * 6 + dx, floor - 1);
                    if w.get(cx, cy).material != material::EMPTY {
                        w.set(cx, cy, Cell::EMPTY);
                    }
                }
            }
            let ants: Vec<u16> = (0..8).map(|i| spawn(&mut w, "ant", 70 + i * 6, floor - 1)).collect();
            assert!(ants.iter().all(|&a| a != 0), "test setup: the ants were not placed");

            let mut highest = floor;
            for _ in 0..40 {
                live(&mut w, 250);
                for x in 40..160 {
                    for y in 0..floor {
                        let c = w.get(x, y);
                        if c.material == w.materials.id_of("ant").expect("ant") && y < highest {
                            highest = y;
                        }
                    }
                }
            }
            // Height climbed above the ground they started on, and how tall
            // the thing they could have climbed was.
            (floor - highest, floor - trunk_top, w.creature_stats.falls)
        }

        println!("\nHOW HIGH DOES AN ANT CLIMB -- 8 ants, 10,000 frames, height in cells above spawn");
        println!("{:<10} {:>8} {:>12} {:>12} {:>8}", "seed", "tree", "climbed", "tree height", "falls");
        for seed in 0..4u64 {
            let (bare, _, bare_falls) = trial(false, 0xC0DE + seed);
            let (treed, tall, treed_falls) = trial(true, 0xC0DE + seed);
            println!("{:<10} {:>8} {:>12} {:>12} {:>8}", format!("{seed:#x}"), "no", bare, "-", bare_falls);
            println!("{:<10} {:>8} {:>12} {:>12} {:>8}", "", "yes", treed, tall, treed_falls);
        }
    }

    #[test]
    fn eating_one_leaf_does_not_kill_the_tree_that_grew_it() {
        // **The bug that made renewable food not renewable.** `act`'s eat
        // branch reconciles the *owner* of the mouthful, and the owner of a
        // leaf is a tree -- which has no `chain`, so the empty-chain case
        // read as "head gone" and freed the whole organism. Measured before
        // the guard: one leaf off a 789-cell tree, and the tree stopped
        // resolving while 160 of its cells stood on in the world still
        // carrying its id, which after sixteen slot reuses would alias
        // somebody else.
        //
        // Driven through `reconcile_chain` rather than through a live ant
        // for the reason `a_predator_eats_a_creature...` records twice:
        // whether an ant can *reach* a leaf is a different question, and
        // mixing it in makes the test measure foraging.
        let mut w = test_world();
        let soil = w.materials.id_of("soil").expect("soil is compiled in");
        // **Floored, and that is the second half of the same repair.** The
        // bed spans y=150..160 of a 0..199 world with nothing under it, and
        // soil is a `Powder` -- so it avalanched ~40 rows to the world
        // floor and the seed rode down with it. The test passed anyway once
        // the bed was dampened, which is the trap: it passed *despite* the
        // scene rather than because of it, and the next change here would
        // have inherited a bed that does not stay where it is put.
        //
        // Floor **and** walls, matching `plant::tests::plant_tree_on_ground`
        // -- a floor alone still lets an open-sided bed spill off its own
        // edges, which that helper's comment records as having cost time
        // twice already.
        for x in 0..200 {
            w.set(x, 160, Cell::new(material::STONE, 0));
        }
        for y in 150..161 {
            w.set(0, y, Cell::new(material::STONE, 0));
            w.set(199, y, Cell::new(material::STONE, 0));
        }
        for x in 1..199 {
            for y in 150..160 {
                // **Damp, not bone dry, and that is a merge repair rather
                // than a tuning.** This scene was written when a plant ran
                // on one currency, so `Cell::new(soil, 0)` -- `aux == 0` is
                // *dry* on a `Powder` -- still grew a tree. Water is a real
                // second currency now (`plant::absorb_water`, which arrived
                // with the plant line and did not exist here), and a root
                // in dry soil has no income at all: the tree grew wood and
                // never a leaf, so the `find` below failed on a scene
                // error rather than on the thing this test is named for.
                // Matches `plant::tests::plant_tree_on_ground`, which has
                // always dampened its bed for the same reason.
                w.set(x, y, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));
            }
        }
        w.plant_tree(100, 149);
        for _ in 0..4000 {
            crate::sim::parallel::step(&mut w);
            w.step_active_sites();
            w.step_fields();
        }
        let leaf = w.materials.id_of("leaf").expect("leaf is compiled in");
        let (lx, ly) = (0..200)
            .flat_map(|x| (0..160).map(move |y| (x, y)))
            .find(|&(x, y)| w.get(x, y).material == leaf && w.get(x, y).organism_id() != 0)
            .expect("the tree should have grown at least one leaf it owns");
        let tree = w.get(lx, ly).organism_id();
        let before = w.organism(tree).map(|s| s.cells.len()).expect("live tree");

        w.set(lx, ly, Cell::EMPTY);
        assert!(reconcile_chain(&mut w, tree), "a tree is not a chain creature and losing a leaf is not death");

        let after = w.organism(tree).map(|s| s.cells.len()).expect("the tree must still be a live organism after losing one leaf");
        assert_eq!(after, before - 1, "the leaf is gone from the tree's cell list and nothing else is");
    }

    #[test]
    fn founding_a_colony_from_high_above_the_ground_still_finds_it() {
        // **The bug the owner hit.** The surface search ran 96 rows down
        // from the cursor, which is shorter than the sky on several presets,
        // so pressing Y with the cursor up high placed nothing and said
        // nothing -- indistinguishable from the whole milestone being
        // absent. 150 rows of sky over the floor reproduces it.
        let mut w = test_world();
        for x in 0..199 {
            w.set(x, 160, Cell::new(material::STONE, 0));
        }
        let placed = w.found_colony(100, 5);
        assert!(placed > 40, "a colony founded from high above the ground should still land: got {placed} ants");
    }

    #[test]
    fn a_founded_colony_is_centred_on_the_cursor() {
        // The ants used to run from 26 cells left of the cursor to 178 to
        // the right, so the colony appeared almost entirely to one side and
        // founding it near the right-hand edge threw most of it out of the
        // world, where placement fails silently.
        let mut w = test_world();
        for x in 0..199 {
            w.set(x, 160, Cell::new(material::STONE, 0));
        }
        let placed = w.found_colony(100, 150);
        assert!(placed > 40, "expected a full colony on open floor, got {placed}");
        let ant = w.materials.id_of("ant").expect("ant is compiled in");
        let xs: Vec<i32> = (0..199).filter(|&x| (150..160).any(|y| w.get(x, y).material == ant)).collect();
        let (lo, hi) = (*xs.first().expect("ants"), *xs.last().expect("ants"));
        let centre = (lo + hi) / 2;
        assert!((centre - 100).abs() <= 6, "colony spans {lo}..{hi}, centre {centre}, but was founded at x=100");
    }

    #[test]
    fn founding_a_colony_over_open_sky_reports_that_it_placed_nothing() {
        // The return value is the whole point: it is what lets the app say
        // "no ground under the cursor" instead of appearing inert.
        let mut w = test_world();
        assert_eq!(w.found_colony(100, 5), 0, "there is no ground in this world, so nothing may be placed");
    }

    // --- choose_weighted ----------------------------------------------------

    #[test]
    fn choose_weighted_prefers_the_strong_score_without_ever_excluding_the_weak_one() {
        // The two halves of P-10 in one test: the mechanism has to *prefer*,
        // or it is a random walk, and it has to keep a real tail, or it is an
        // argmax with extra arithmetic and the exploration the whole
        // stigmergy mechanism runs on is gone.
        let scores = [0.0, 1.0];
        let picks: Vec<usize> = (0..1000).map(|i| choose_weighted(&scores, CHOICE_EXPLORATION_K, i as f32 / 1000.0)).collect();
        let strong = picks.iter().filter(|&&p| p == 1).count();
        assert!((970..1000).contains(&strong), "the strong candidate should win about 99.2% of draws at k = 0.1, got {strong}/1000");
        assert!(strong < 1000, "the weak candidate must remain reachable -- an argmax here kills exploration silently");
    }

    #[test]
    fn choose_weighted_is_uniform_when_every_score_is_flat() {
        // "No signal" must mean "explore", not "always pick the first
        // candidate". The `min_by` this replaced returned the first minimum
        // on a tie, which is how the worm came to always flee west, and how
        // `a_worm_flees_a_hot_field_reading` came to pass for a whole
        // milestone against a scene with no gradient in it.
        let scores = [0.0, 0.0, 0.0, 0.0];
        let mut counts = [0usize; 4];
        for i in 0..4000 {
            counts[choose_weighted(&scores, CHOICE_EXPLORATION_K, i as f32 / 4000.0)] += 1;
        }
        for (i, &c) in counts.iter().enumerate() {
            assert!((900..1100).contains(&c), "candidate {i} drawn {c}/4000 times; a flat field should be sampled uniformly");
        }
    }

    // --- the impulse verb ------------------------------------------------
    //
    // Five guards, and `Reports/creature-motion-design.md` §7 is why there
    // are five: this verb reverses a decision (§2d) that cost two previous
    // attempts, both of which shipped and were reverted after putting falls
    // at 59-80% of all moves. Each of these was written before the code it
    // guards and watched failing, which is the exemption `CLAUDE.md` grants
    // from putting the fault back afterwards.

    /// A body of `ant` cells at the given offsets, and the world holding it.
    /// Material rather than a real organism because what is under test is
    /// the *shape* law, and a species would only add plumbing that could
    /// disagree with it.
    fn body_of(cells: &[(i32, i32)]) -> (World, Vec<(i32, i32)>) {
        let mut w = test_world();
        let ant = w.materials.id_of("ant").expect("ant material is compiled in");
        for &(x, y) in cells {
            w.set(x, y, Cell::new(ant, 0));
        }
        (w, cells.to_vec())
    }

    #[test]
    fn the_body_decides_what_one_impulse_does() {
        // **The whole design claim, as one assertion.** §5's table is
        // supposed to fall out of cell count and bounding box, with no
        // `match species` anywhere; if someone replaces `body_drag` with a
        // constant, or moves the jump height onto `CreatureDef`, this is
        // what has to notice.
        //
        // **The four shipped body plans, read off the species files rather
        // than typed out here.** The first version of this test hand-listed
        // a 5x2 as ten cells; `ant_wide` is nine, because its template has a
        // notch in the top row. The test failed, which is the instrument
        // working -- and a guard over "the body decides" that carries its own
        // idea of what the bodies are is not guarding the shipped ones.
        let plan = |name: &str, at: (i32, i32)| -> Vec<(i32, i32)> {
            let w = test_world();
            let species = w.species.id_of(name).unwrap_or_else(|| panic!("species {name:?} is compiled in"));
            let def = w.species.get(species).creature.as_ref().expect("a creature").clone();
            def.body.offsets(false).iter().map(|&(dx, dy)| (at.0 + dx, at.1 + dy)).collect()
        };
        let chain2 = plan("ant", (10, 10));
        let chain6 = plan("ant_long", (30, 10));
        let wide = plan("ant_wide", (60, 10));
        let block = plan("ant_block", (90, 10));
        assert_eq!(wide.len(), block.len(), "the controlled pair is only controlled if they are the same size");

        let launch_speed = |cells: &[(i32, i32)]| -> f32 {
            let (w, c) = body_of(cells);
            (2.0 * LAUNCH_WORK / body_drag(&w, &c).mass).sqrt()
        };
        let terminal = |cells: &[(i32, i32)]| -> f32 {
            let (w, c) = body_of(cells);
            terminal_speed(&body_drag(&w, &c), AIR_DENSITY, GRAVITY)
        };

        // 1. Heavier launches slower, strictly, across all four.
        let (v2, v6, vw, vb) = (launch_speed(&chain2), launch_speed(&chain6), launch_speed(&wide), launch_speed(&block));
        assert!(v2 > v6, "a 2-cell chain must out-launch a 6-cell one: {v2} vs {v6}");
        assert!(v6 > vw, "a 6-cell chain must out-launch a 9-cell slab: {v6} vs {vw}");

        // 2. **The controlled pair.** Same mass, so the same launch to the
        //    last bit -- and that equality is what makes the descent
        //    difference below attributable to shape alone.
        assert_eq!(vw, vb, "a 5x2 and a 3x3 of the same material are the same mass and must launch identically");

        // 3. ...and the compact one comes down more than twice as fast.
        //    This is the row of §5 that says "glides" against "drops like a
        //    stone", and it is the only difference between them.
        let (tw, tb) = (terminal(&wide), terminal(&block));
        assert!(tb > tw * 2.0, "the 3x3 block must fall more than 2x faster than the 5x2 slab: {tb} vs {tw}");

        // 4. And a *chain* has no fixed shape, which is the reason drag is
        //    read off the cells and not off `BodyPlan`. The same six cells
        //    coiled into a square fall faster than strung out flat.
        let coiled: Vec<(i32, i32)> = [(80, 10), (81, 10), (82, 10), (80, 11), (81, 11), (82, 11)].into();
        assert!(
            terminal(&coiled) > terminal(&chain6),
            "six cells coiled ({}) must fall faster than the same six strung out ({})",
            terminal(&coiled),
            terminal(&chain6)
        );
    }

    #[test]
    fn a_buoyant_body_hangs_instead_of_sinking() {
        // Decision E9's float limit, and §2c's claim that it needed no new
        // physics: it is the sign of `(density - fluid density)` and nothing
        // else. Asserted on the *mechanism* rather than on a species,
        // because no buoyant creature is authored yet -- what has to be true
        // is that authoring one would work.
        let (w, cells) = body_of(&[(10, 10), (11, 10)]);
        let shape = body_drag(&w, &cells);
        // In something twice its own density there is no net weight left.
        let carried = buoyant_share(shape.density, shape.density * 2.0);
        assert_eq!(carried, 1.0, "a body less dense than its fluid keeps none of its weight");
        assert_eq!(terminal_speed(&shape, shape.density * 2.0, GRAVITY * (1.0 - carried)), 0.0, "and so does not sink");
        // In something half its density it still goes down, slowly.
        let carried = buoyant_share(shape.density, shape.density * 0.5);
        assert!(carried > 0.0 && carried < 1.0, "a denser body keeps some of its weight: {carried}");
        assert!(terminal_speed(&shape, shape.density * 0.5, GRAVITY * (1.0 - carried)) > 0.0, "and sinks");
    }

    #[test]
    fn the_shipped_ant_reads_exactly_zero_on_the_impulse_output() {
        // **Guard 3 of §7, at the level the byte-identical claim rests on.**
        // `ascii`'s counters can only be unchanged if the ant never launches,
        // and it can only never launch if this output is *exactly* 0.0 --
        // not small. `creature_tick` gates on `> 0.0` before touching the
        // RNG, so an exact zero is also what stops the draw stream shifting
        // under every other decision the ant makes.
        //
        // Read through the real species genome, not a hand-built one: what
        // is being claimed is about the animal that ships.
        let w = test_world();
        let species = w.species.id_of("ant").expect("ant species");
        let genome = w.species.get(species).genome.clone();
        let mut state = [0.0; brain::BRAIN_HIDDEN];
        // Every input saturated, which is the worst case for a stray weight.
        let (out, _) = brain::eval_brain(&genome, &[1.0; brain::BRAIN_INPUTS], &mut state);
        assert_eq!(
            out[brain::BrainOutput::Impulse as usize],
            0.0,
            "ant.ron authors no Impulse weight, so the output must be exactly 0.0 -- \
             anything else means the ant now jumps and every counter in ascii has moved"
        );
    }

    /// A creature of `species` standing on a stone shelf, and its id.
    fn creature_on_a_shelf(w: &mut World, species: &str, x: i32, y: i32) -> u16 {
        for sx in (x - 20)..(x + 20) {
            for sy in y..(y + 3) {
                w.set(sx, sy, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        let site = plant_creature_seed(w, x, y - 1, species).expect("the creature fits");
        let ActiveKind::Creature { organism } = site.kind else { unreachable!() };
        w.schedule_active_site(site);
        organism
    }

    #[test]
    fn a_launch_leaves_the_ground_and_comes_back_down() {
        // The verb end to end: it fires, the creature is airborne for a
        // while, it lands, and it is standing on something afterwards.
        //
        // **`landed_on_ground` is the half that matters.** A verb that put
        // creatures in the air and left them there is exactly the failure
        // §2d reverted twice, and "it went up" alone cannot see it.
        let mut w = test_world();
        let ant = creature_on_a_shelf(&mut w, "ant", 100, 60);
        let def = w.species.get(w.organism(ant).expect("live").species).creature.clone().expect("a creature");
        let start = w.organism(ant).expect("live").chain[0];

        assert!(launch(&mut w, ant, 0), "an ant standing on stone can push off");
        assert_eq!(w.creature_stats.impulses, 1);
        assert!(w.organism(ant).expect("live").flight.is_some(), "and is airborne");

        let mut airborne_frames = 0;
        for _ in 0..600 {
            if w.organism(ant).and_then(|s| s.flight).is_none() {
                break;
            }
            step_flight(&mut w, ant, &def);
            airborne_frames += 1;
        }
        assert!(w.organism(ant).expect("live").flight.is_none(), "the hop has to end");
        assert!(airborne_frames > 4, "and it has to last long enough to read as an arc, not a teleport: {airborne_frames}");
        let end = w.organism(ant).expect("live").chain[0];
        assert!(end.0 > start.0, "an east-facing launch travels east: {start:?} -> {end:?}");
        assert!(
            body_is_supported(&w, &w.organism(ant).expect("live").chain.clone()),
            "and it lands on something -- a hop that ends in mid-air is the reverted failure coming back"
        );
        assert_eq!(w.creature_stats.flight_frames, airborne_frames as u64);
        assert!(w.creature_stats.flight_moves > 0, "the counter that says the arc went somewhere");
    }

    #[test]
    fn a_creature_in_the_air_cannot_launch_again() {
        // No double jump, and no using the verb to cancel a fall: the
        // launch needs something to push against. `impulses_refused` is the
        // effect-side pair `CLAUDE.md` asks for beside `impulses` -- a
        // counter that says a call happened is only worth the claim that the
        // call did something.
        let mut w = test_world();
        let ant = creature_on_a_shelf(&mut w, "ant", 100, 60);
        // Pick it up off the shelf: nothing within 8 of it any more.
        let chain = w.organism(ant).expect("live").chain.clone();
        let aloft: Vec<(i32, i32)> = chain.iter().map(|&(x, y)| (x, y - 20)).collect();
        relocate_chain(&mut w, ant, &chain, &aloft);

        assert!(!launch(&mut w, ant, 0), "there is nothing to push off");
        assert_eq!(w.creature_stats.impulses, 0, "and it must not be counted as a launch");
        assert_eq!(w.creature_stats.impulses_refused, 1, "it is counted as a refusal");
        assert!(w.organism(ant).expect("live").flight.is_none());
    }

    #[test]
    fn the_same_launch_carries_a_slab_further_than_a_block() {
        // **§5's controlled pair, played out rather than computed.** The
        // unit assertion above says the two shapes get different terminal
        // speeds; this says the difference survives the integrator, the
        // collision handling and the landing -- which is the part a formula
        // cannot promise.
        //
        // Same mass, same launch, same shelf, same drop. The only difference
        // on screen is how long each stays up.
        let travel = |species: &str| -> (i32, u64) {
            let mut w = test_world();
            let id = creature_on_a_shelf(&mut w, species, 40, 30);
            let def = w.species.get(w.organism(id).expect("live").species).creature.clone().expect("a creature");
            // Off the end of the shelf, over open air down to row 199.
            let chain = w.organism(id).expect("live").chain.clone();
            let out: Vec<(i32, i32)> = chain.iter().map(|&(x, y)| (x + 22, y)).collect();
            relocate_chain(&mut w, id, &chain, &out);
            let start = w.organism(id).expect("live").chain[0];
            // Put it back in touch with the shelf for one instant so the
            // launch is legal, then let it go: a plinth of one cell.
            w.set(start.0 - 1, start.1 + 1, Cell::new(material::STONE, 0).with_attached(true));
            assert!(launch(&mut w, id, 0), "{species} can push off");
            w.set(start.0 - 1, start.1 + 1, Cell::EMPTY);
            let mut frames = 0u64;
            while w.organism(id).and_then(|s| s.flight).is_some() && frames < 2000 {
                step_flight(&mut w, id, &def);
                frames += 1;
            }
            let end = w.organism(id).expect("live").chain[0];
            (end.0 - start.0, frames)
        };
        let (slab_x, slab_frames) = travel("ant_wide");
        let (block_x, block_frames) = travel("ant_block");
        assert!(
            slab_frames > block_frames,
            "the 5x2 slab must stay up longer than the 3x3 block of the same mass: {slab_frames} vs {block_frames} frames"
        );
        assert!(
            slab_x > block_x,
            "and therefore land further out: {slab_x} vs {block_x} cells -- if these are equal the body is not being read"
        );
    }

    #[test]
    fn choose_weighted_never_indexes_out_of_bounds() {
        // The `draw` comes from `Rng::unit_f32`, which is bounded above by
        // 1.0 but the cumulative sum is floating-point: a draw at the very
        // top of the range can fall past the last bucket by an ulp.
        for n in 1..=4 {
            let scores = vec![0.5f32; n];
            for draw in [0.0, 0.5, 0.999_999, 1.0] {
                assert!(choose_weighted(&scores, CHOICE_EXPLORATION_K, draw) < n, "n={n} draw={draw}");
            }
        }
    }

    // --- S6: reproduction, inheritance, mutation ------------------------
    //
    // **Populations and paired comparisons, never a single individual.**
    // Dead end 552: every single-individual assert on this line broke the
    // day genotypes started varying, and creature outcomes already spread
    // 0.103-0.541 across random genomes. The tests below that do look at
    // one animal look at a *deterministic function of it* -- the birth
    // cost it was charged, the generation it was handed -- which is a
    // different claim from "this individual did well".

    /// A colony of `n` ants on a stone floor, each rich enough to bud, with
    /// a `reproduce_threshold` low enough that the first tick pays for it.
    ///
    /// Returns the world and the founders' handles. Ants are spaced four
    /// apart, the `COLONY_ANT_SPACING` a founded colony uses, because
    /// shoulder-to-shoulder ants gridlock and a birth needs a free cell.
    fn breeding_colony(n: i32, threshold: f32, mutation_rate: f32) -> (World, Vec<u16>) {
        let mut w = test_world();
        for cx in 0..200 {
            w.set(cx, 101, Cell::new(material::STONE, 0));
        }
        let ant = w.species.id_of("ant").expect("ant species");
        // Through `set_creature`, the same seam `creature_space` overrides
        // the budget with -- an in-process override rather than an edited
        // `.ron`, because assets are `include_str!`ed and a test that
        // edited one would be varying a knob connected to nothing.
        let mut def = w.species.get(ant).creature.clone().expect("ant is a creature");
        def.reproduce_threshold = threshold;
        def.mutation_rate = mutation_rate;
        w.species.set_creature(ant, def);
        let mut founders = Vec::new();
        for i in 0..n {
            w.plant_ant(10 + i * 4, 100);
            let id = w.get(10 + i * 4, 100).organism_id();
            if id != 0 {
                founders.push(id);
            }
        }
        // Fund every founder past the threshold. Done by hand rather than
        // by feeding them, because what these tests are about is the birth
        // path and a scene that has to be foraged first would make every
        // one of them a test of the economy instead.
        for id in founders.clone() {
            fund(&mut w, id, threshold + 10.0);
        }
        (w, founders)
    }

    /// Set one creature's bank to `to`, **and book the difference**.
    ///
    /// The booking is not bookkeeping pedantry, it is what keeps the
    /// ledger test above honest: hand-written energy is energy created out
    /// of nothing, which is exactly what `EnergyLedger::granted` means. The
    /// first version of this helper wrote the field directly and the
    /// identity opened by 13,320 joules — a scene that contradicts the code
    /// looking precisely like a bug in the code.
    fn fund(w: &mut World, id: u16, to: f32) {
        let Some(state) = w.organism_mut(id) else { return };
        let delta = to - state.energy;
        state.energy = to;
        w.energy_ledger.granted += delta as f64;
    }

    /// **The verb fires, and it produces an animal.**
    ///
    /// The negative control is the same colony with `reproduce_threshold`
    /// at 0, which is how the species files ship: a counter that only ever
    /// reads non-zero cannot say whether the mechanism ran or whether
    /// something else made ants.
    #[test]
    fn a_funded_colony_buds_and_an_unfunded_one_does_not() {
        let (mut fed, founders) = breeding_colony(12, 2000.0, 0.0);
        run(&mut fed, 60);
        let births = fed.creature_stats.births;
        assert!(births > 0, "{} funded ants over 60 frames produced no births at all", founders.len());
        assert_eq!(fed.creature_stats.births_denied_no_space, 0, "an ant with four cells of clear floor either side had nowhere to put a child");

        // Same scene, reproduction not authored. `plant_ant` still spawns,
        // so `spawned` moves and `births` must not.
        let (mut off, _) = breeding_colony(12, 0.0, 0.0);
        run(&mut off, 60);
        assert_eq!(off.creature_stats.births, 0, "a species with reproduce_threshold 0 bred anyway");
        assert!(off.creature_stats.spawned > 0, "the control placed no ants, so it controls for nothing");
    }

    /// **Heredity: the child's genome is the parent's, not the species'.**
    ///
    /// Put the fault back by pointing `Origin::Bud` at
    /// `world.species.get(species_id).genome` — the line the founder arm
    /// uses — and this goes red on the first assert, because the marker
    /// below is in no species file.
    #[test]
    fn a_child_carries_its_parents_genome_not_the_species_one() {
        let (mut w, founders) = breeding_colony(8, 2000.0, 0.0);
        // A marker no authored genome carries, written into a live slot so
        // it survives `wiring_from_genome`'s reserved-slot assertion.
        let slot = brain::live_slots().next().expect("a genome has live slots");
        for &id in &founders {
            if let Some(state) = w.organism_mut(id) {
                state.genome[slot] = -7.5;
            }
        }
        run(&mut w, 60);
        assert!(w.creature_stats.births > 0, "nothing was born, so nothing was inherited and this test proves nothing");
        let children: Vec<u16> = live_creature_ids(&w).into_iter().filter(|id| !founders.contains(id)).collect();
        assert!(!children.is_empty(), "births fired but no child is live");
        for id in children {
            let state = w.organism(id).expect("a live child");
            assert_eq!(state.genome[slot], -7.5, "child {id} carries the species genome, not its parent's");
            assert!(state.inherited, "child {id} is not flagged inherited");
        }
    }

    /// **Lineage depth increases every generation** — the shadowing guard.
    ///
    /// `CLAUDE.md` records the exact fault: a `let generation =
    /// state.generation;` written after `state` has been rebound to the
    /// *child* pins every bred individual at generation 1 for ever, and
    /// lineage depth silently flattens while every other counter looks
    /// healthy. Two successive births are the minimum that can see it —
    /// one birth reads 1 whether the code is right or wrong.
    #[test]
    fn generation_depth_keeps_climbing_across_successive_births() {
        let (mut w, founders) = breeding_colony(8, 2000.0, 0.0);
        let mut deepest = 0u16;
        // Re-fund everyone alive every 60 frames, so the colony keeps
        // breeding rather than settling after one round. Funding the whole
        // population rather than the founders is what lets *children*
        // breed, which is the generation this test is really about.
        for _ in 0..6 {
            for id in live_creature_ids(&w) {
                fund(&mut w, id, 2010.0);
            }
            run(&mut w, 60);
            deepest = deepest.max(live_creature_ids(&w).iter().filter_map(|&id| w.organism(id)).map(|s| s.generation).max().unwrap_or(0));
        }
        assert!(w.creature_stats.births > 0, "nothing bred");
        assert!(
            deepest >= 2,
            "deepest lineage reached generation {deepest} over {} births from {} founders -- a depth that stops at 1 is the shadowed-parent bug",
            w.creature_stats.births,
            founders.len()
        );
    }

    /// **A clonal line is one lineage label, and two founders are two.**
    ///
    /// The label is what lineage share is computed over, so a bug that
    /// re-labelled children would make every drift measurement read as
    /// perfect diversity — the tidy answer, which is the tell.
    #[test]
    fn a_lineage_label_is_inherited_whole_and_founders_never_share_one() {
        let (mut w, founders) = breeding_colony(8, 2000.0, 0.0);
        let founder_labels: Vec<u32> = founders.iter().filter_map(|&id| w.organism(id)).map(|s| s.lineage).collect();
        let mut unique = founder_labels.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), founder_labels.len(), "two founders were handed the same lineage label");
        assert!(!unique.contains(&0), "a founder was left at lineage 0, which means 'no lineage'");

        run(&mut w, 60);
        assert!(w.creature_stats.births > 0, "nothing bred");
        for id in live_creature_ids(&w) {
            let label = w.organism(id).expect("live").lineage;
            assert!(founder_labels.contains(&label), "organism {id} carries lineage {label}, which no founder started");
        }
    }

    /// **A birth creates no energy** — the S3b stamp seam, asserted as the
    /// ledger identity rather than as a story about it.
    ///
    /// Put the fault back by booking a bud's grant to `granted` (a source)
    /// the way the founder arm does, and the live identity opens by
    /// `start_energy` per birth.
    #[test]
    fn the_energy_ledger_still_closes_when_a_colony_breeds() {
        let (mut w, _) = breeding_colony(12, 2000.0, 0.01);
        run(&mut w, 400);
        assert!(w.creature_stats.births > 0, "nothing bred, so the birth path was never exercised");
        let live = w.live_creature_energy();
        let expected = w.energy_ledger.expected_live_total();
        assert!(
            (live - expected).abs() < 1.0,
            "live energy {live:.3} against ledger {expected:.3} after {} births -- a birth has created or destroyed energy",
            w.creature_stats.births
        );
        let standing = standing_meat(&w, Rect::new(0, 0, 199, 199)) + carried_meat(&w);
        let bound = w.energy_ledger.max_standing_meat();
        assert!(standing <= bound + 1.0, "standing meat {standing:.3} exceeds the bound {bound:.3}");
    }

    /// **The parent pays, and it pays exactly the birth cost.**
    #[test]
    fn a_parent_is_poorer_by_the_whole_birth_cost() {
        let (mut w, founders) = breeding_colony(1, 2000.0, 0.0);
        let parent = founders[0];
        let def = w.species.get(w.organism(parent).expect("live").species).creature.clone().expect("ant is a creature");
        let before = w.organism(parent).expect("live").energy;
        run(&mut w, 60);
        assert_eq!(w.creature_stats.births, 1, "expected exactly one birth from one funded ant in 60 frames");
        let after = w.organism(parent).expect("the parent survived").energy;
        let paid = before - after;
        let cost = birth_cost(&def);
        // The parent also pays its own metabolism over those frames, so
        // this is a lower bound on what it spent and an assertion that the
        // birth cost is *in* it.
        assert!(paid >= cost, "parent paid {paid:.2} for a birth costing {cost:.2}");
        assert!(paid < cost + def.start_energy, "parent paid {paid:.2}, far more than the {cost:.2} birth cost plus a few ticks of metabolism");
    }

    /// **The threshold can never sit below the cost.**
    ///
    /// A species authored under its own birth cost would kill a parent
    /// every time it bred, and every counter would read as reproduction
    /// working perfectly.
    #[test]
    fn a_mis_authored_threshold_is_floored_above_the_birth_cost() {
        let w = test_world();
        let ant = w.species.id_of("ant").expect("ant species");
        let mut def = w.species.get(ant).creature.clone().expect("ant is a creature");
        assert_eq!(reproduce_at(&def).is_none(), def.reproduce_threshold <= 0.0);
        def.reproduce_threshold = 1.0;
        let at = reproduce_at(&def).expect("a species with a positive threshold reproduces");
        assert!(at > birth_cost(&def), "threshold {at} sits at or under the birth cost {}", birth_cost(&def));
        def.reproduce_threshold = 0.0;
        assert!(reproduce_at(&def).is_none(), "a zero threshold must mean 'does not reproduce'");
    }

    /// **Who gets born is decided by energy, and nothing else.**
    ///
    /// The birth decision takes no random draw at all — a bank against a
    /// threshold, then `DIRS` order for placement — so the *count* of
    /// births and of denials must be bit-identical however hard the
    /// genomes are being mutated. If someone later rolls for whether to
    /// bud, or draws the placement, this is what says the population's
    /// demography stopped being a function of its economy.
    ///
    /// **What this test deliberately does *not* assert, and why**, because
    /// the obvious stronger version was written first and is unsound:
    /// *"changing only the mutation rate must not move a single movement
    /// decision"*. It fails, and it fails correctly — a mutated child
    /// walks differently from a clonal one, which is the entire mechanism
    /// working. The two effects cannot be separated in a population run:
    /// by the time a shared-`Rng` perturbation of the parent would show,
    /// the children have been moving for several ticks and are blocking
    /// it. The shared-stream invariant is guarded one level down instead,
    /// by `brain::tests::the_mutation_rate_changes_how_many_draws_it_takes`
    /// plus the dedicated `RNG_SLOT_BIRTH` stream at the call site.
    #[test]
    fn the_mutation_rate_does_not_change_who_gets_born() {
        let census = |rate: f32| {
            let (mut w, _) = breeding_colony(12, 2000.0, rate);
            run(&mut w, 400);
            (w.creature_stats.births, w.creature_stats.births_denied_no_space)
        };
        let quiet = census(0.0);
        let busy = census(0.5);
        assert!(quiet.0 > 0, "no births in either arm, so the mutation path never ran");
        assert_eq!(quiet, busy, "changing only the mutation rate moved the demography: {quiet:?} against {busy:?}");
    }

    /// Every live creature's handle, in slot order.
    fn live_creature_ids(w: &World) -> Vec<u16> {
        let mut out = Vec::new();
        for x in 0..200 {
            for y in 0..200 {
                let cell = w.get(x, y);
                let id = cell.organism_id();
                if id != 0 && w.organism(id).is_some_and(|s| w.species.get(s.species).creature.is_some()) && !out.contains(&id) {
                    out.push(id);
                }
            }
        }
        out
    }

    // --- body extent: pricing and shading -------------------------------
    //
    // **Paired comparisons and deterministic functions only** (dead end
    // 552). The pricing tests below hold everything except body length
    // fixed and read the *difference*; the shade tests call a pure function
    // with hand-built inputs, which is the one shape `CLAUDE.md` exempts
    // from the "put the fault back" rule because it cannot be blind in an
    // interesting way.

    /// A one-ant world with `cells` body cells and a chosen bill.
    ///
    /// `move_cost_per_cell` is zeroed so the only size-dependent term left
    /// is the idle charge -- otherwise a run that happened to move more
    /// would swamp the quantity under test, and the difference would be
    /// about the terrain rather than about the body.
    fn priced_ant(cells: u8, idle_per_cell: f32) -> (World, u16) {
        use super::super::organism::BodyPlan;
        let mut w = test_world();
        for cx in 0..200 {
            w.set(cx, 101, Cell::new(material::STONE, 0));
        }
        let ant = w.species.id_of("ant").expect("ant species");
        let mut def = w.species.get(ant).creature.clone().expect("ant is a creature");
        def.body = BodyPlan::Chain(cells);
        def.idle_cost_per_cell = idle_per_cell;
        def.move_cost_per_cell = 0.0;
        w.species.set_creature(ant, def);
        w.plant_ant(20, 100);
        let organism = w.get(20, 100).organism_id();
        assert_ne!(organism, 0, "the ant must have hatched, or this measures nothing");
        (w, organism)
    }

    #[test]
    fn a_longer_body_costs_proportionally_more_to_run() {
        // The premise `creature-evolution-plan.md` E10 was written on --
        // "per-cell metabolic cost already prices a longer body" -- was
        // false until 2026-08-30: nothing in the cost path read
        // `chain.len()`, so this difference was exactly zero and a longer
        // body was strictly free. That is the fault this guard is named
        // for, and reverting either call site takes it red.
        let idle = 0.05f32;
        let energy_after_one_tick = |cells: u8| {
            let (mut w, organism) = priced_ant(cells, idle);
            let before = w.organism(organism).expect("live").energy;
            let def = w.species.get(w.organism(organism).expect("live").species).creature.clone().expect("creature");
            creature_tick(&mut w, 20, 100, organism, &def);
            before - w.organism(organism).map_or(0.0, |s| s.energy)
        };
        let two = energy_after_one_tick(2);
        let six = energy_after_one_tick(6);
        // The synapse tax and every other term are identical across the
        // pair, so the whole difference is the four extra cells.
        let difference = six - two;
        assert!(
            (difference - idle * 4.0).abs() < 1e-4,
            "four extra body cells must cost four extra per-cell idle charges: {two} -> {six} (difference {difference}, expected {})",
            idle * 4.0
        );
    }

    #[test]
    fn a_damaged_animal_pays_for_the_body_it_still_has() {
        // `live_body_cells` reads the live chain rather than `BodyPlan`,
        // so an ant that has lost a cell is a smaller animal and burns
        // less. Reading the plan instead would keep charging a two-cell
        // bill to an animal down to one.
        let (mut w, organism) = priced_ant(3, 0.05);
        let def = w.species.get(w.organism(organism).expect("live").species).creature.clone().expect("creature");
        assert_eq!(live_body_cells(&w, organism, &def), 3.0);
        // Take the tail off, the way a bite does, and reconcile.
        let tail = *w.organism(organism).expect("live").chain.last().expect("cells");
        w.set(tail.0, tail.1, Cell::EMPTY);
        reconcile_chain(&mut w, organism);
        assert_eq!(
            live_body_cells(&w, organism, &def),
            2.0,
            "a bitten ant is a two-cell animal and must be billed as one"
        );
    }

    #[test]
    fn countershading_puts_the_palest_cell_on_the_head() {
        // Ranked darkest-first, so the last entry is the palest.
        let ranked = [2u8, 0, 1];
        // A rigid body two cells tall: dy 0 is the top row, dy 1 the bottom.
        assert_eq!(body_shade(&ranked, 0, 6, 0, 0, 1), 1, "the head takes the palest entry outright");
        assert_eq!(body_shade(&ranked, 1, 6, 0, 0, 1), 1, "a top-row body cell is pale");
        assert_eq!(body_shade(&ranked, 2, 6, 1, 0, 1), 2, "an underside cell is the darkest");
    }

    #[test]
    fn a_chain_grades_along_its_body_because_it_has_no_underside() {
        // A `Chain` is one cell thick, so `dy_max == dy_min` and a
        // top-to-bottom grade would put every cell on the same shade --
        // one flat smear, which is exactly the "reads as a worm" problem
        // E10 names. The grade runs head-to-tail instead.
        let ranked = [2u8, 0, 1];
        let shades: Vec<u8> = (0..5).map(|i| body_shade(&ranked, i, 5, 0, 0, 0)).collect();
        assert_eq!(shades[0], 1, "head palest");
        assert_eq!(*shades.last().expect("cells"), 2, "tail darkest");
        assert!(
            shades.iter().collect::<std::collections::HashSet<_>>().len() > 1,
            "a chain must not come out one flat colour: {shades:?}"
        );
    }

    #[test]
    fn the_shipped_ant_still_draws_its_shade_at_random() {
        // E10 is the owner's standing decision that the shipped ant does
        // not change without a verdict, and `ShadeRule::Random` is what it
        // ships with. This guard is what makes `ant.ron` gaining a
        // `shade_rule` line a deliberate act rather than a merge accident.
        let w = test_world();
        let ant = w.species.id_of("ant").expect("ant species");
        let def = w.species.get(ant).creature.as_ref().expect("ant is a creature");
        assert_eq!(def.shade_rule, ShadeRule::Random);
        assert_eq!(def.body.len(), 2, "and it is still two cells");
    }

    #[test]
    fn shades_by_luma_orders_darkest_first_and_survives_an_empty_palette() {
        let w = test_world();
        let ant = w.materials.id_of("ant").expect("ant material");
        let ranked = shades_by_luma(&w, ant);
        let palette = &w.materials.get(ant).palette;
        let luma = |i: u8| {
            let c = palette[i as usize];
            c[0] as u32 * 299 + c[1] as u32 * 587 + c[2] as u32 * 114
        };
        for pair in ranked.windows(2) {
            assert!(luma(pair[0]) <= luma(pair[1]), "ranked darkest first: {ranked:?}");
        }
    }
}
