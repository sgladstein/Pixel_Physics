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

use super::cell::{Cell, AMBIENT_TEMPERATURE};
use super::material::{self, MaterialKind};
use super::scheduler::{ActiveKind, ActiveSite};
use super::world::World;

const NEIGHBOURS_4: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

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

/// Per-creature state too large for `Cell::aux`'s `u16` — mirrors
/// `plant::TreeState`'s reasoning. Never shrinks; entries are indexed by
/// `ActiveKind::Creature` and never reassigned. Indexed by `u16`, not `u32`
/// like `TreeState`, because `Cell::aux` (also a `u16`) stores this same
/// index directly, per its own documented meaning for
/// `MaterialKind::Creature` — unlike a tree's growth stage, "which creature
/// owns this cell" has to round-trip through the cell itself, so the index
/// space is bounded by what `aux` can hold.
pub struct CreatureState {
    energy: f32,
}

/// Plant a worm at `(x, y)` if it's empty and the `worm` material is
/// loaded. Returns the site to schedule, or nothing if either precondition
/// failed.
pub fn plant_worm_seed(world: &mut World, x: i32, y: i32) -> Option<ActiveSite> {
    let worm_id = world.materials.id_of("worm")?;
    if !world.is_empty(x, y) {
        return None;
    }
    let shades = world.materials.get(worm_id).palette.len().max(1) as u32;
    let shade = world.rng.below(shades) as u8;
    let creature_id = world.push_creature(CreatureState { energy: WORM_START_ENERGY });
    world.set(x, y, Cell::new(worm_id, shade).with_aux(creature_id));
    Some(ActiveSite { x, y, kind: ActiveKind::Creature { creature: creature_id }, next_frame: world.frame + WORM_TICK_INTERVAL })
}

/// Dispatch a due `ActiveKind::Creature` site to `worm_tick`. `scheduler::step`
/// never routes any other `ActiveKind` here.
pub fn tick(world: &mut World, site: &ActiveSite) -> Vec<ActiveSite> {
    let ActiveKind::Creature { creature } = site.kind else {
        unreachable!("scheduler::step only routes ActiveKind::Creature to creature::tick");
    };
    worm_tick(world, site.x, site.y, creature)
}

fn worm_tick(world: &mut World, x: i32, y: i32, creature_id: u16) -> Vec<ActiveSite> {
    let Some(worm_id) = world.materials.id_of("worm") else {
        return Vec::new();
    };
    let cell = world.get(x, y);
    // The cell may have burned into a corpse (fire.rs, uniformly for every
    // material kind), been erased by the brush, or been cleared by an
    // explosion by the time this tick is due -- nothing left to move.
    if cell.material != worm_id {
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
    if cell.is_burning() {
        return vec![ActiveSite { x, y, kind: ActiveKind::Creature { creature: creature_id }, next_frame: world.frame + WORM_TICK_INTERVAL }];
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
        return apply_energy_delta(world, x, y, creature_id, -WORM_IDLE_COST);
    }

    let (tx, ty, cost) = if fleeing {
        // Move toward whichever reachable neighbour reads coolest --
        // descending the field's temperature gradient is the entire
        // mechanism, same as the real AFD neuron comparing against a
        // remembered set-point.
        // Bilinear, not block-nearest (architecture §6a) — every candidate
        // here is only 1 cell from `(x, y)`, well inside the same
        // `FIELD_SCALE`-sided field block `field_at` would read, which
        // otherwise makes every candidate compare temperature-equal and
        // `min_by` silently degenerate into "always pick the first
        // neighbour" instead of real gradient descent.
        *candidates
            .iter()
            .min_by(|a, b| {
                let ta = world.field_at_bilinear(a.0 as f32, a.1 as f32).temperature;
                let tb = world.field_at_bilinear(b.0 as f32, b.1 as f32).temperature;
                ta.partial_cmp(&tb).unwrap()
            })
            .expect("candidates is non-empty here")
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
        pool[world.rng.below(pool.len() as u32) as usize]
    };

    let target = world.get(tx, ty);
    let ate = world.materials.kind(target.material) == MaterialKind::Powder;

    // Move: the worm cell (carrying its own creature id in aux) takes the
    // target position; whatever was there is left behind at the worm's old
    // position -- an earthworm's actual ingest-ahead, cast-behind burrowing
    // behaviour (see the research file), simplified to "the material passes
    // through" rather than modelling digested organic content separately.
    let moved_worm = Cell::new(worm_id, cell.shade).with_aux(creature_id).with_temperature(cell.temperature());
    world.set(x, y, target);
    world.set(tx, ty, moved_worm);

    let delta = if ate { WORM_ENERGY_FROM_EATING - cost } else { -cost };
    apply_energy_delta(world, tx, ty, creature_id, delta)
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
fn apply_energy_delta(world: &mut World, x: i32, y: i32, creature_id: u16, delta: f32) -> Vec<ActiveSite> {
    let energy = {
        let state = world.creature_mut(creature_id);
        state.energy += delta;
        state.energy
    };
    if energy <= 0.0 {
        die(world, x, y);
        return Vec::new();
    }
    vec![ActiveSite { x, y, kind: ActiveKind::Creature { creature: creature_id }, next_frame: world.frame + WORM_TICK_INTERVAL }]
}

fn die(world: &mut World, x: i32, y: i32) {
    let Some(corpse_id) = world.materials.id_of("corpse") else {
        return;
    };
    let shades = world.materials.get(corpse_id).palette.len().max(1) as u32;
    let shade = world.rng.below(shades) as u8;
    let temp = world.get(x, y).temperature();
    world.set(x, y, Cell::new(corpse_id, shade).with_temperature(temp));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::field;
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
        // worm was actually created and did something (moved and/or
        // starved into a corpse), so this test can't pass vacuously the
        // way an earlier version of this suite did when `plant_worm` was
        // accidentally called on already-occupied ground (see README.md's
        // M18 status section).
        let worm_id = w.materials.id_of("worm").unwrap();
        let corpse_id = w.materials.id_of("corpse").unwrap();
        let existed = (81..150).any(|x| (90..110).any(|y| { let m = w.get(x, y).material; m == worm_id || m == corpse_id }));
        assert!(existed, "no worm or corpse found anywhere -- was one ever actually created?");
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
        // Heat the east end of the corridor sharply -- the worm starts
        // inside the hot zone, so it should flee west from tick one rather
        // than foraging randomly first. Radius is in world cells but
        // `add_heat` paints in field-cell units (`FIELD_SCALE` = 8 world
        // cells each), so this needs to be generously larger than the
        // 35-cell distance from the heat centre to the worm's start (100)
        // to actually reach it, not just look large in world units.
        w.add_heat(135, y, 48, 400.0);
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
        let mut w = test_world();
        let y = 100;
        for x in 60..120 {
            w.set(x, y - 1, Cell::new(material::STONE, 0));
            w.set(x, y + 1, Cell::new(material::STONE, 0));
        }
        // Field cell (11, *) spans world x 88..=95; radius 4 (< FIELD_SCALE
        // = 8) paints only the one field cell containing (88, 100). Field
        // cell (12, *) -- world x 96..=103, immediately east -- stays
        // ambient.
        w.add_heat(88, y, 4, 400.0);
        w.plant_worm(93, y); // 5 cells into the painted cell, 2 short of its eastern edge
        run(&mut w, 300);

        let worm_id = w.materials.id_of("worm").unwrap();
        let fx = (60..120)
            .find(|&x| w.get(x, y).material == worm_id)
            .expect("worm should still be alive and somewhere in the corridor");
        assert!(fx > 93, "worm at x={fx} did not move toward the cooler cell immediately to its east");
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
    }
}
