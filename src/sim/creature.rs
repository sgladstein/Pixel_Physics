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
use super::field;
use super::material::{self, MaterialKind};
use super::organism::{pack_cell_type, CellType, CreatureDef};
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

/// How strongly a worm is allowed to pick the *wrong* neighbour.
///
/// The additive term `k` in `choose_weighted`'s `(k + s)²`. At `0.1`, a
/// candidate scoring 0 against one scoring 1 is chosen `0.01 / 1.22`, about
/// **0.8% of ticks** — enough that a worm never gets deterministically
/// wedged, small enough that fleeing a fire still looks purposeful.
pub const CHOICE_EXPLORATION_K: f32 = 0.1;

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
    let organism = world.push_organism(worm_species);
    let shades = world.materials.get(worm_id).palette.len().max(1) as u32;
    let shade = rng::stream(world.seed, organism as u64, world.frame, RNG_SLOT_SHADE).below(shades) as u8;
    world.set(x, y, Cell::new(worm_id, shade).with_organism_id(organism).with_aux(pack_cell_type(CellType::Head)));
    if let Some(state) = world.organism_mut(organism) {
        state.energy = WORM_START_ENERGY;
        // A worm is a chain of one. Ants are 2-3; nothing here assumes the
        // length is 1 except `worm_tick`'s own 4-neighbour candidate set.
        state.chain = vec![(x, y)];
    }
    Some(ActiveSite { x, y, kind: ActiveKind::Creature { organism }, next_frame: world.frame + WORM_TICK_INTERVAL })
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
    let Some(state) = world.organism(organism) else {
        return Vec::new();
    };
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
        return vec![ActiveSite { x, y, kind: ActiveKind::Creature { organism }, next_frame: world.frame + WORM_TICK_INTERVAL }];
    }

    // C. elegans-style thermotaxis: read the local ambient field, and if
    // it's dangerously hot, every subsequent choice is about descending the
    // gradient rather than foraging.
    let fleeing = world.field_at(x, y).temperature - AMBIENT_TEMPERATURE as f32 > WORM_HEAT_THRESHOLD_ABOVE_AMBIENT;

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
        let temps: Vec<f32> = candidates.iter().map(|c| world.field_at_bilinear(c.0 as f32, c.1 as f32).temperature).collect();
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
    vec![ActiveSite { x, y, kind: ActiveKind::Creature { organism }, next_frame: world.frame + WORM_TICK_INTERVAL }]
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

/// Radius of the crowding scan, in cells. Small on purpose: this is a
/// *contact-range* read of the grid, which decision D5 explicitly permits
/// (`Reports/creature-direction.md`) — the hard line is at colony scale,
/// where the field is the mechanism, and it is not crossed by an ant
/// noticing the ants it is standing among.
const CROWDING_RADIUS: i32 = 2;
/// How strongly a candidate with a foothold is preferred over one into
/// thin air. Added to the candidate's score, not multiplied into it — see
/// `step_chain` for the measurement that decided which.
///
/// A bonus rather than a veto: an ant must still be able to walk off a
/// ledge, and forbidding it outright would be the "gate whether something
/// happens" mistake that `CLAUDE.md` warns a size cap must never make.
const FOOTING_BONUS: f32 = 0.6;
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
    let genome = world.species.get(species_id).genome.clone();

    // Every cell of the chain has to be available *before* anything is
    // allocated or written, or a half-placed body leaks a slot and leaves
    // orphan cells (the same reason `plant_worm_seed` checks first).
    let mut positions = Vec::with_capacity(def.body_cells as usize);
    for i in 0..def.body_cells as i32 {
        let p = (x - i, y);
        if !world.is_empty(p.0, p.1) {
            return None;
        }
        positions.push(p);
    }

    let organism = world.push_organism(species_id);
    let shades = world.materials.get(material_id).palette.len().max(1) as u32;
    for (i, &(px, py)) in positions.iter().enumerate() {
        let shade = rng::stream(world.seed, organism as u64, i as u64, RNG_SLOT_SHADE).below(shades) as u8;
        let cell_type = if i == 0 { CellType::Head } else { CellType::Segment };
        world.set(px, py, Cell::new(material_id, shade).with_organism_id(organism).with_aux(pack_cell_type(cell_type)));
    }
    if let Some(state) = world.organism_mut(organism) {
        state.energy = def.start_energy;
        state.chain = positions;
        state.heading = 0; // east
        state.genome = genome;
        // Starts *at* the nest as far as scent goes: an ant that has just
        // hatched has, by construction, just been at home.
        state.since_nest = 0;
    }
    world.creature_stats.spawned += 1;
    world.energy_ledger.granted += def.start_energy as f64;
    Some(ActiveSite { x, y, kind: ActiveKind::Creature { organism }, next_frame: world.frame + def.tick_interval })
}

impl World {
    /// Place an ant at `(x, y)` — the debug/scene entry point, mirroring
    /// `plant_worm`.
    pub fn plant_ant(&mut self, x: i32, y: i32) {
        if let Some(site) = plant_creature_seed(self, x, y, "ant") {
            self.schedule_active_site(site);
        }
    }
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
        release_if_bodyless(world, organism);
        return Vec::new();
    }
    if cell.is_burning() {
        // Same deferral the worm makes, for the same reason: let fire.rs
        // finish deciding this creature's fate first.
        return vec![ActiveSite { x, y, kind: ActiveKind::Creature { organism }, next_frame: world.frame + def.tick_interval }];
    }

    let heading = world.organism(organism).map_or(0, |s| s.heading);
    let inputs = sense(world, x, y, organism, heading, def);
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
    let mut spent = def.idle_cost + def.synapse_cost * active_synapses as f32;
    world.energy_ledger.metabolized += def.idle_cost as f64;
    world.energy_ledger.synapse_tax += (def.synapse_cost * active_synapses as f32) as f64;

    // --- the four verbs, before moving: an ant that is going to pick
    // --- something up should do it from where it can reach it.
    act(world, x, y, organism, def, &outputs, &mut draw);

    // --- move -----------------------------------------------------------
    let p_move = outputs[brain::BrainOutput::Move as usize].clamp(0.0, 1.0);
    let mut moved = false;
    if draw.unit_f32() < p_move {
        moved = step_chain(world, organism, heading, &outputs, def, &mut draw, material_id);
        if moved {
            spent += def.move_cost;
            world.energy_ledger.moved += def.move_cost as f64;
        }
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
    apply_creature_energy(world, hx, hy, organism, -spent, def)
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
    let inputs = sense(world, x, y, organism, state.heading, def);
    let mut brain_state = state.brain_state;
    let (outputs, active) = brain::eval_brain(&state.genome, &inputs, &mut brain_state);
    (inputs, outputs, active)
}

/// The 14 brain inputs. Slot indices are `brain::BrainInput`'s and are a
/// permanent public contract — see that enum.
fn sense(world: &World, x: i32, y: i32, organism: u16, heading: u8, def: &CreatureDef) -> [f32; brain::BRAIN_INPUTS] {
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

    let moisture_at = |px: i32, py: i32| world.field_at_bilinear(px as f32, py as f32).moisture / WORM_MOISTURE_SATURATION;
    inputs[I::MoistureFront as usize] = moisture_at(fx, fy);
    inputs[I::MoistureLateral as usize] = moisture_at(rx, ry) - moisture_at(lx, ly);

    let here = world.field_at_bilinear(x as f32, y as f32);
    // Divided through by the day/night oscillator, per CLAUDE.md: a
    // threshold sampled at an arbitrary phase of a designed oscillator is a
    // different threshold every hour, and the light channel swings 20:1.
    inputs[I::LightHere as usize] = (field::noon_equivalent_light(here.light, world.frame) / field::MAX_LIGHT).clamp(0.0, 1.0);
    inputs[I::TempAboveAmb as usize] = ((here.temperature - AMBIENT_TEMPERATURE as f32) / TEMP_INPUT_SCALE).clamp(-1.0, 1.0);

    inputs[I::FoodAdjacent as usize] = if adjacent_food(world, x, y, def).is_some() { 1.0 } else { 0.0 };
    inputs[I::AtNest as usize] = if adjacent_nest(world, x, y, def) { 1.0 } else { 0.0 };

    if let Some(state) = world.organism(organism) {
        inputs[I::Energy as usize] = (state.energy / def.start_energy.max(1.0)).clamp(0.0, 1.0);
        inputs[I::Carrying as usize] = if state.carrying.is_some() { 1.0 } else { 0.0 };
    }

    let mut crowd = 0;
    for dy in -CROWDING_RADIUS..=CROWDING_RADIUS {
        for dx in -CROWDING_RADIUS..=CROWDING_RADIUS {
            if dx == 0 && dy == 0 {
                continue;
            }
            if world.materials.kind(world.get(x + dx, y + dy).material) == MaterialKind::Creature {
                crowd += 1;
            }
        }
    }
    inputs[I::Crowding as usize] = (crowd as f32 / CROWDING_SCALE).min(1.0);

    inputs
}

/// The first food cell in the head's 8-neighbourhood, if any.
fn adjacent_food(world: &World, x: i32, y: i32, def: &CreatureDef) -> Option<(i32, i32, material::MaterialId)> {
    NEIGHBOURS_8.iter().find_map(|&(dx, dy)| {
        let m = world.get(x + dx, y + dy).material;
        // Resolved by name per tick rather than cached on the species: a
        // handful of hash lookups per creature per tick (~50 a frame at 100
        // ants) against the alternative, which is a resolved-id cache that
        // has to be invalidated on every F5 material reload. Cheap enough
        // that the simpler mechanism wins.
        def.food.iter().any(|name| world.materials.id_of(name) == Some(m)).then_some((x + dx, y + dy, m))
    })
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
    let drop_urge = outputs[O::Drop as usize].clamp(0.0, 1.0);

    // --- eat / pick up --------------------------------------------------
    if carrying.is_none() {
        if let Some((fxx, fyy, food)) = adjacent_food(world, x, y, def) {
            if draw.unit_f32() < dig_urge {
                let hungry = world.organism(organism).is_some_and(|s| s.energy < def.start_energy * def.hunger_fraction);
                world.set(fxx, fyy, Cell::EMPTY);
                if hungry {
                    if let Some(state) = world.organism_mut(organism) {
                        state.energy += def.eat_energy;
                    }
                    world.energy_ledger.eaten += def.eat_energy as f64;
                    world.creature_stats.eats += 1;
                } else {
                    // Full: carry it home instead of eating it. This is the
                    // whole reason a colony accumulates stores rather than
                    // every ant simply feeding itself.
                    if let Some(state) = world.organism_mut(organism) {
                        state.carrying = Some(food);
                    }
                    world.creature_stats.pickups += 1;
                }
                return;
            }
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
                world.set(dx, dy, Cell::new(held, 0));
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
    material_id: material::MaterialId,
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
    let base = [turn.max(0.0), 0.15, (-turn).max(0.0)];
    let mut scores = [0.0f32; 3];
    let mut passable = [false; 3];
    for (i, &d) in dirs.iter().enumerate() {
        let (dx, dy) = DIRS[d as usize];
        let (tx, ty) = (hx + dx, hy + dy);
        // Raw emptiness, plus "my own tail", which a chain may legitimately
        // step into because it vacates on the same tick.
        passable[i] = world.is_empty(tx, ty) || chain.contains(&(tx, ty));
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
            base[i] + if head_has_foothold(world, (tx, ty)) { FOOTING_BONUS } else { 0.0 }
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
    let footing_ahead = passable.iter().zip(&scores).any(|(&p, &s)| p && s > FOOTING_BONUS * 0.5);
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
        let viable: Vec<u8> = (0..8u8)
            .filter(|&d| {
                let (dx, dy) = DIRS[d as usize];
                let (tx, ty) = (hx + dx, hy + dy);
                (world.is_empty(tx, ty) || chain.contains(&(tx, ty))) && head_has_foothold(world, (tx, ty))
            })
            .collect();
        if let Some(state) = world.organism_mut(organism) {
            state.heading = if viable.is_empty() { draw.below(8) as u8 } else { viable[draw.below(viable.len() as u32) as usize] };
        }
        world.creature_stats.moves_blocked += 1;
        return false;
    }

    // Zero out anything without footing now that at least one candidate
    // has some: the discount was never the mechanism, the choice is.
    for (i, s) in scores.iter_mut().enumerate() {
        if !passable[i] || *s <= FOOTING_BONUS * 0.5 {
            *s = 0.0;
        }
    }
    let pick = choose_weighted(&scores, CHOICE_EXPLORATION_K, draw.unit_f32());
    let pick = if scores[pick] > 0.0 { pick } else { scores.iter().position(|&s| s > 0.0).expect("footing_ahead guarantees one") };
    let new_heading = dirs[pick];
    let (dx, dy) = DIRS[new_heading as usize];
    let (tx, ty) = (hx + dx, hy + dy);

    let mut next: Vec<(i32, i32)> = Vec::with_capacity(chain.len());
    next.push((tx, ty));
    next.extend(chain.iter().take(chain.len() - 1).copied());
    relocate_chain(world, organism, &chain, &next);
    if let Some(state) = world.organism_mut(organism) {
        state.heading = new_heading;
    }
    world.creature_stats.moves += 1;

    // Touching the nest resets the scent clock, which is what makes channel
    // A a gradient rather than a uniform smear.
    if adjacent_nest(world, tx, ty, def) {
        if let Some(state) = world.organism_mut(organism) {
            // Only count it as a visit if the creature had actually been
            // away: an ant loitering on the nest would otherwise register
            // one every tick and the counter would say nothing.
            if state.since_nest > 0 {
                world.creature_stats.nest_visits += 1;
            }
            let state = world.organism_mut(organism).expect("live");
            state.since_nest = 0;
        }
    }
    let _ = material_id;
    true
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
fn head_has_foothold(world: &World, (x, y): (i32, i32)) -> bool {
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
        world.in_bounds(nx, ny)
            && matches!(world.materials.kind(world.get(nx, ny).material), MaterialKind::Solid | MaterialKind::Powder | MaterialKind::Plant)
    })
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
    vec![ActiveSite { x, y, kind: ActiveKind::Creature { organism }, next_frame: world.frame + def.tick_interval }]
}

/// Every cell of the chain becomes `corpse`, and the slot comes back.
///
/// **A dead ant is matter** — food for the next one, fuel for a fire — and
/// that costs no code at all because the material system already does it.
/// It is also what closes the colony's loop: the energy a forager spent
/// getting somewhere it could not return from is not deleted, it is left
/// lying there as something edible.
fn creature_dies(world: &mut World, organism: u16) {
    let chain = world.organism(organism).map(|s| s.chain.clone()).unwrap_or_default();
    let held = world.organism(organism).and_then(|s| s.carrying);
    let leftover = world.organism(organism).map_or(0.0, |s| s.energy.max(0.0));
    world.energy_ledger.died_holding += leftover as f64;
    if let Some(corpse_id) = world.materials.id_of("corpse") {
        let shades = world.materials.get(corpse_id).palette.len().max(1) as u32;
        for (i, &(cx, cy)) in chain.iter().enumerate() {
            let shade = rng::stream(world.seed, organism as u64, i as u64, RNG_SLOT_DEATH).below(shades) as u8;
            let temp = world.get(cx, cy).temperature();
            world.set(cx, cy, Cell::new(corpse_id, shade).with_temperature(temp));
        }
    }
    // Whatever it was carrying falls where it fell. Losing it would be a
    // silent material sink, and the census is about to care.
    if let (Some(held), Some(&(cx, cy))) = (held, chain.last()) {
        if world.is_empty(cx, cy) {
            world.set(cx, cy, Cell::new(held, 0));
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
        let heir = w.push_organism(heir);
        assert_eq!(doomed & 0x0FFF, heir & 0x0FFF, "the test needs the heir to inherit the slot");
        if let Some(state) = w.organism_mut(heir) {
            state.energy = 999.0;
        }

        w.schedule_active_site(ActiveSite { x: 100, y: 100, kind: ActiveKind::Creature { organism: doomed }, next_frame: w.frame + 1 });
        run(&mut w, 10); // must not panic

        assert_eq!(w.organism(heir).expect("the heir is live").energy, 999.0, "a stale creature site must not have spent the energy of the organism that inherited its slot");
        assert!(w.active_site_count() <= 1, "the stale site should have dropped itself rather than rescheduling");
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
}
