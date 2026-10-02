//! **Brood: eggs, larvae that must be fed, and pupae** -- the ant's life
//! cycle in place of budding a whole adult on the spot
//! (`Reports/ant-breeding-plan-2026-09-29.md` B1 and B3, built together on
//! the owner's 2026-10-02 ruling; the review that reordered them is
//! `/mnt/project-files/breeding/plan-review-2026-10-02.md`).
//!
//! **Why.** Before this, any ant that had eaten enough split off a full-grown
//! copy of itself wherever it stood, paying the whole 1,040 J at once. On the
//! food box nearly all income became births (seed 3: ~6.1 MJ taken, ~5.5 MJ
//! into births), so no food was ever put by, and a colony that outgrew its
//! income starved all at once. Here laying is cheap (`BroodDef::egg_cost`),
//! and growing an ant is what costs: a larva has to be fed the rest of the
//! adult's price by nestmates and by food lying beside it. In a famine the
//! larvae go hungry and die first, so the colony shrinks from its brood
//! before it starves its workers -- graded, not a cliff.
//!
//! **What a brood organism is.** An organism of the laying species with an
//! empty `chain`, one cell of the brood material (a `Powder` that does not
//! roll and drops through bodies, as `crumbs` does) whose `aux` packs
//! `CellType::Seed`, exactly as a plant's seed does. That encoding closes
//! most of the verbs that could mistake one before any code here runs:
//!
//! - the jaw refuses `is_live_seed`, so no digger cuts an egg;
//! - the founding cut and footing refuse any organism-owned cell;
//! - `is_living_kin_id` reads the owner, so a nestmate's mouth skips it as
//!   food and counts it into `KinNeed`;
//! - `World::set` keeps its cell list current under both drivers, so it is
//!   found wherever it fell;
//! - its material is not the species' body material, so `sense_ahead` and
//!   every body-material test pass it by.
//!
//! The brood material carries no food value, so **a stranger cannot eat a
//! living egg** in this step: predation on brood is future work, and making
//! it inedible keeps the ledger closed without pricing an egg at its bank.
//! A starved larva's corpse is ordinary `corpse` and is eaten like any.
//!
//! **The census.** `World::live_organism_ids`, `live_creature_groups` and
//! `live_creature_count` leave brood out, so the hundreds of sites that count
//! animals through them do not count an egg as an ant. `World::live_brood_ids`
//! is the brood's own list, and `CreatureStats` carries its counters.
//!
//! **The economy.** No step creates energy, and each has a ledger row:
//!
//! | event | who pays | ledger |
//! |---|---|---|
//! | lay | the layer pays `egg_cost` into the egg's bank (food in reach covers a shortfall, as a bud's does) | live to live |
//! | a larva is shared to | a nestmate above `start_energy`, mouth to mouth | live to live |
//! | a larva eats | food beside it, at `diet_yield` | the harvest accounts |
//! | a larva waits | its bank, `larva_upkeep` a frame | `Metabolized` |
//! | hatch | the brood's bank pays the body's stamp; the rest is the adult's first bank | `StoredInMeat` |
//! | a larva starves | what it held becomes a corpse cell | `StoredInMeat`, the rounding `Dissipated` |
//! | the cell is destroyed | the bank goes with it | `Dissipated` |
//!
//! **Counted at hatching, not at laying**: `CreatureStats::births`, the
//! `Born` log line and the line's population, so "animals born" keeps
//! meaning adults that appeared. Laying counts the parent's `children`
//! (so breeder suppression reads it at once), `life.offspring` and
//! `eggs_laid`. A brood organism that never hatches is freed through
//! `World::free_brood`, which books no death.

use super::cell::{Cell, OrganismId};
use super::creature;
use super::organism::{self, pack_cell_type, BroodDef, BroodStage, CellType, CreatureDef, SpeciesId, CREATURE_TRAITS};
use super::scheduler::{ActiveKind, ActiveSite};
use super::world::{Account, World};

/// **Is brood on, absent a per-world override?** `PIXEL_PHYSICS_BROOD=on`
/// turns it on; anything else, or unset, leaves it off. Off is today's
/// budding, exactly: `brood_of` returns `None` before anything is read.
fn brood_env() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| matches!(std::env::var("PIXEL_PHYSICS_BROOD").as_deref(), Ok("on")))
}

/// The species' brood block, when it has one and brood is on for this world
/// (`World::brood`, else `PIXEL_PHYSICS_BROOD`). `None` means bud a whole
/// adult as before.
pub fn brood_of(world: &World, def: &CreatureDef) -> Option<BroodDef> {
    let block = def.brood.as_ref()?;
    world.brood.unwrap_or_else(brood_env).then(|| {
        let mut b = block.clone();
        if let Some(v) = lay_at_env() {
            b.lay_at = v;
        }
        if let Some(v) = egg_cost_env() {
            b.egg_cost = v;
        }
        b
    })
}

/// `PIXEL_PHYSICS_EGG_COST=<J>`: override the brood block's `egg_cost`, for
/// a sweep. At the adult's whole price (1,040 for the shipped ant) a larva
/// needs no feeding at all, which is budding with a delay -- the control.
fn egg_cost_env() -> Option<f32> {
    static V: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    *V.get_or_init(|| std::env::var("PIXEL_PHYSICS_EGG_COST").ok().and_then(|v| v.parse().ok()))
}

/// `PIXEL_PHYSICS_LAY_AT=<J>`: override the brood block's `lay_at`, for a
/// sweep. Unset, the species file's value.
fn lay_at_env() -> Option<f32> {
    static V: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    *V.get_or_init(|| std::env::var("PIXEL_PHYSICS_LAY_AT").ok().and_then(|v| v.parse().ok()))
}

/// Frames between a larva's ticks: how often it pays upkeep and takes a bite
/// of food beside it. One dispatch per larva per this many frames is the
/// whole cost of brood to the frame; an egg or a pupa is one dispatch per
/// stage.
pub const LARVA_TICK: u64 = 60;
/// Frames between attempts to hatch a pupa whose adult body does not fit.
pub const HATCH_RETRY: u64 = 60;
/// How far from its own cell, in rings, a hatchling's head may be placed.
pub const HATCH_REACH: i32 = 3;

/// Everything an egg inherits, read off the parent by `try_bud` exactly as a
/// bud's `Origin::Bud` is, and already mutated where a bud's is (the body's
/// fates on the parent's handle).
pub struct Egg {
    pub species: SpeciesId,
    pub genome: Vec<f32>,
    pub traits: [f32; CREATURE_TRAITS],
    pub generation: u16,
    pub lineage: u32,
    pub colony: u32,
    pub made: f32,
    pub fates: organism::FateGenome,
}

/// **Lay one egg beside the head**, in the first empty neighbour in `DIRS`
/// order -- one cell fits where a whole body in a line rarely does, which is
/// what makes laying at the nest possible at all (`BUD_SITE=nest` took births
/// to zero with a whole adult, `ant-scenes-2026-09-23.md` §9-10).
///
/// Returns the egg's first site, or `None` when no neighbour is empty (the
/// caller counts that as a refused birth, as it does for a bud). The parent
/// pays `egg_cost`; the bar it had to clear is the same as a bud's, so a
/// laying ant keeps a reserve of roughly `reproduce_at - egg_cost`.
pub(super) fn lay_egg(world: &mut World, parent: OrganismId, head: (i32, i32), def: &CreatureDef, brood: &BroodDef, egg: Egg) -> Option<ActiveSite> {
    let material = world.materials.id_of(&brood.material)?;
    let (hx, hy) = head;
    let (ex, ey) = creature::DIRS.iter().map(|&(dx, dy)| (hx + dx, hy + dy)).find(|&(x, y)| world.is_empty(x, y))?;
    let child = world.push_organism(egg.species)?;
    // **Fixed at laying**: the adult this egg becomes costs what a bud of
    // this parent would have -- the authored body's stamp plus the grant the
    // layer's own `TRAIT_BIRTH_GRANT` names.
    let target = creature::birth_cost_of(def, creature::birth_grant(def, &egg.traits));
    let frame = world.frame;
    if let Some(st) = world.organism_mut(child) {
        st.energy = brood.egg_cost;
        st.genome = egg.genome;
        st.traits = egg.traits;
        st.fates = egg.fates;
        st.made = egg.made;
        st.lineage = egg.lineage;
        st.colony = egg.colony;
        st.generation = egg.generation;
        st.inherited = true;
        st.brood = Some(organism::Brood { stage: BroodStage::Egg, since: frame, target, parent, last_tick: frame });
    }
    world.set(ex, ey, Cell::new(material, BroodStage::Egg as u8).with_organism_id(child).with_aux(pack_cell_type(CellType::Seed)));
    // **The parent pays what it has; food within reach pays the rest** -- the
    // bud's rule (`place_creature`'s `Origin::Bud` arm), at the egg's price.
    let colony = world.colony_of(parent);
    let shortfall = (brood.egg_cost + 1.0 - world.organism(parent).map_or(0.0, |s| s.energy)).max(0.0);
    if shortfall > 0.0 {
        let gut = creature::gut_of(world, parent, def);
        let cells: Vec<(f32, i32, i32)> = creature::provisions_in_reach(world, hx, hy, gut).collect();
        creature::eat_toward_birth(world, parent, colony, cells, shortfall);
    }
    if let Some(p) = world.organism_mut(parent) {
        p.energy -= brood.egg_cost;
        p.life.offspring += 1;
        p.seeds_set = p.seeds_set.saturating_add(1);
    }
    if world.organism(parent).is_some_and(|s| s.energy < 0.5) {
        world.creature_stats.births_overdrawn += 1;
    }
    world.creature_stats.eggs_laid += 1;
    Some(ActiveSite { x: ex, y: ey, kind: ActiveKind::Creature { organism: child }, next_frame: world.creature_due(brood.egg_frames) })
}

/// Where this brood organism's cell is now: its scheduled site, or wherever
/// it fell since (it is a powder, so it can drop through air and bodies).
fn brood_cell(world: &World, organism: OrganismId, material: super::material::MaterialId, at: (i32, i32)) -> Option<(i32, i32)> {
    let c = world.get(at.0, at.1);
    if c.organism_id() == organism && c.material == material {
        return Some(at);
    }
    let state = world.organism(organism)?;
    state.cells.keys().copied().filter(|&(x, y)| world.get(x, y).material == material).min_by_key(|&(x, y)| (y, x))
}

/// **One tick of a brood organism**, dispatched from `creature::tick` before
/// anything reads a body: an egg becomes a larva, a larva pays upkeep, eats
/// what lies beside it and pupates once fed to its target, a pupa hatches.
pub fn brood_tick(world: &mut World, site: &ActiveSite) -> Vec<ActiveSite> {
    let ActiveKind::Creature { organism } = site.kind else {
        return Vec::new();
    };
    let Some(state) = world.organism(organism) else {
        return Vec::new();
    };
    let Some(b) = state.brood else {
        return Vec::new();
    };
    let species = state.species;
    let Some(def) = world.species.get(species).creature.clone() else {
        return Vec::new();
    };
    // A world that switched brood off mid-life still runs what is already
    // laid to its end, so nothing is stranded half-paid.
    let Some(block) = def.brood.clone() else {
        return Vec::new();
    };
    let Some(material) = world.materials.id_of(&block.material) else {
        return Vec::new();
    };
    let Some((x, y)) = brood_cell(world, organism, material, (site.x, site.y)) else {
        // **The cell is gone** -- burned, blasted, erased, buried by a write.
        // What the brood held goes with it.
        let bank = world.organism(organism).map_or(0.0, |s| s.energy);
        let colony = world.colony_of(organism);
        world.book(colony, Account::Dissipated, bank as f64);
        world.creature_stats.brood_lost += 1;
        world.free_brood(organism);
        return Vec::new();
    };
    let frame = world.frame;
    let colony = world.colony_of(organism);
    let at = |next: u64| vec![ActiveSite { x, y, kind: ActiveKind::Creature { organism }, next_frame: next }];
    match b.stage {
        BroodStage::Egg => {
            if frame < b.since + block.egg_frames {
                return at(b.since + block.egg_frames);
            }
            set_stage(world, organism, (x, y), material, BroodStage::Larva, frame);
            world.creature_stats.larvae += 1;
            at(world.creature_due(LARVA_TICK))
        }
        BroodStage::Larva => {
            // **Upkeep for the frames since the last tick**, so a late
            // dispatch neither skips nor double-charges.
            let elapsed = frame.saturating_sub(b.last_tick) as f32;
            let upkeep = block.larva_upkeep * elapsed;
            if let Some(st) = world.organism_mut(organism) {
                st.energy -= upkeep;
                if let Some(bb) = st.brood.as_mut() {
                    bb.last_tick = frame;
                }
            }
            world.book(colony, Account::Metabolized, upkeep as f64);
            world.creature_stats.brood_upkeep_j += upkeep as f64;
            let bank = world.organism(organism).map_or(0.0, |s| s.energy);
            if bank <= 0.0 {
                larva_starves(world, organism, (x, y), bank, colony);
                return Vec::new();
            }
            // **One bite of food beside it**, by the species' own mouth rule
            // (`provisions_in_reach`: same diet filter, same price, kin
            // skipped) -- a larva is a second mouth, so it takes the same
            // filter the adult's does.
            if bank < b.target {
                let gut = creature::gut_of(world, organism, &def);
                let cells: Vec<(f32, i32, i32)> = creature::provisions_in_reach(world, x, y, gut).take(1).collect();
                if !cells.is_empty() {
                    creature::eat_toward_birth(world, organism, colony, cells, b.target - bank);
                    let after = world.organism(organism).map_or(bank, |s| s.energy);
                    world.creature_stats.brood_ate_j += (after - bank) as f64;
                }
            }
            if world.organism(organism).is_some_and(|s| s.energy >= b.target) {
                set_stage(world, organism, (x, y), material, BroodStage::Pupa, frame);
                world.creature_stats.pupae += 1;
                return at(world.creature_due(block.pupa_frames));
            }
            at(world.creature_due(LARVA_TICK))
        }
        BroodStage::Pupa => {
            if frame < b.since + block.pupa_frames {
                return at(b.since + block.pupa_frames);
            }
            match hatch(world, organism, (x, y), species, &def) {
                Some(adult) => vec![adult],
                None => {
                    world.creature_stats.hatches_denied += 1;
                    at(world.creature_due(HATCH_RETRY))
                }
            }
        }
    }
}

/// Move a brood organism to `stage`, and its cell to that stage's shade.
fn set_stage(world: &mut World, organism: OrganismId, (x, y): (i32, i32), material: super::material::MaterialId, stage: BroodStage, frame: u64) {
    if let Some(st) = world.organism_mut(organism) {
        if let Some(b) = st.brood.as_mut() {
            b.stage = stage;
            b.since = frame;
            b.last_tick = frame;
        }
    }
    let cell = world.get(x, y);
    world.set(x, y, Cell::new(material, stage as u8).with_organism_id(organism).with_aux(cell.aux()));
}

/// **A larva that ran out**: what it held, if anything, becomes a corpse cell
/// on its own cell, so a nestmate can eat back what the colony spent on it.
fn larva_starves(world: &mut World, organism: OrganismId, (x, y): (i32, i32), bank: f32, colony: u32) {
    let leftover = bank.max(0.0);
    // Below zero is the last tick's charge overrunning what was there, booked
    // as an adult's overrun is.
    world.book(colony, Account::Overdrawn, (leftover - bank) as f64);
    let worth = leftover.round().clamp(0.0, u16::MAX as f32);
    match world.materials.id_of("corpse").filter(|_| worth >= 1.0) {
        Some(corpse) => {
            world.set(x, y, Cell::new(corpse, 0).with_aux(worth as u16));
            world.book(colony, Account::StoredInMeat, worth as f64);
            world.book(colony, Account::Dissipated, (leftover - worth).max(0.0) as f64);
            world.creature_stats.brood_corpse_j += worth as f64;
        }
        None => {
            world.set(x, y, Cell::EMPTY);
            world.book(colony, Account::Dissipated, leftover as f64);
        }
    }
    world.creature_stats.larvae_starved += 1;
    world.free_brood(organism);
}

/// **A pupa becomes an adult on or beside its own cell**, if the body fits
/// within `HATCH_REACH`. The cell is cleared first so the head can take it,
/// and put back if the body cannot be laid anywhere. Returns the adult's
/// first site.
fn hatch(world: &mut World, organism: OrganismId, (x, y): (i32, i32), species: SpeciesId, def: &CreatureDef) -> Option<ActiveSite> {
    let state = world.organism(organism)?;
    let b = state.brood?;
    let (genome, traits, generation, lineage, colony, made, fates, bank) = (state.genome.clone(), state.traits, state.generation, state.lineage, state.colony, state.made, state.fates, state.energy);
    let cell = world.get(x, y);
    world.set(x, y, Cell::EMPTY);
    // **On its own cell first, then the nearest open cell around it**, ring
    // by ring out to `HATCH_REACH`. Measured on the food box with the own
    // cell alone (main 44f14af, 4 seeds, 144,000 frames): 23,752-31,201
    // refused attempts, and 10-22 pupae standing at the end holding 24-37 kJ
    // -- 25-35 ants' worth the colony had paid for and could not use,
    // because a pupa in a pile has brood and nestmates on every side and a
    // five-cell body needs a line of open cells. A real callow walks out of
    // the pile; a short reach stands in for that walk.
    let mut placed = None;
    'rings: for r in 0..=HATCH_REACH {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                let (hx, hy) = (x + dx, y + dy);
                if r > 0 && !world.is_empty(hx, hy) {
                    continue;
                }
                for facing_west in [false, true] {
                    placed = creature::place_hatchling(world, hx, hy, species, def, facing_west, b.parent, genome.clone(), traits, generation, lineage, colony, made, fates, bank);
                    if placed.is_some() {
                        break 'rings;
                    }
                }
            }
        }
    }
    let Some(site) = placed else {
        world.set(x, y, cell);
        return None;
    };
    world.free_brood(organism);
    // **The line's own history**, at the moment the adult exists -- see
    // `try_bud`'s call of the same two notes.
    if lineage != 0 {
        if let ActiveKind::Creature { organism: adult } = site.kind {
            let born_frame = world.organism(adult).map_or(0, |s| s.born_frame);
            world.note_line_generation(lineage, generation, adult, born_frame, species);
            world.note_line_record(lineage, &traits, adult, born_frame, species, generation);
        }
    }
    Some(site)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::material;
    use crate::sim::chunk::Rect;

    /// A stone floor at row 101 and one ant standing on it at (100, 100),
    /// rich enough to breed, with brood on for this world.
    fn bed(brood: bool) -> (World, OrganismId, CreatureDef) {
        let mut w = World::new(Rect::new(0, 0, 199, 199));
        for x in 60..140 {
            w.set(x, 101, Cell::new(material::STONE, 0));
        }
        w.brood = Some(brood);
        w.plant_ant(100, 100);
        let ant = w.get(100, 100).organism_id();
        assert_ne!(ant, 0, "test setup: the ant was not placed");
        w.organism_mut(ant).expect("live").energy = 1_500.0;
        let species = w.organism(ant).expect("live").species;
        let def = w.species.get(species).creature.clone().expect("ant is a creature");
        assert!(def.brood.is_some(), "test setup: ant.ron authors no brood block");
        (w, ant, def)
    }

    fn gap(w: &World) -> f64 {
        w.live_creature_energy() - w.energy_ledger.expected_live_total()
    }

    fn the_egg(w: &World) -> OrganismId {
        let ids = w.live_brood_ids();
        assert_eq!(ids.len(), 1, "expected exactly one brood organism, found {}", ids.len());
        ids[0]
    }

    /// **The whole life of one egg**: laid for `egg_cost`, a larva at
    /// `egg_frames`, fed by food beside it to its target, a pupa, and an
    /// adult on its own cell after `pupa_frames` -- with the live energy
    /// identity closed at every step, the census never counting the brood
    /// as an animal, and "born" counted once, at hatching.
    #[test]
    fn an_egg_is_laid_cheaply_fed_by_food_beside_it_and_hatches_with_the_books_closed() {
        let (mut w, ant, def) = bed(true);
        let block = def.brood.clone().expect("brood");
        let g0 = gap(&w);
        let animals = w.live_creature_count();
        let site = creature::try_bud(&mut w, ant, &def, 0.0).expect("a rich ant lays");
        assert_eq!(w.creature_stats.eggs_laid, 1);
        assert_eq!(w.creature_stats.births, 0, "an egg is not a birth; births count adults that appeared");
        assert!((w.organism(ant).expect("parent lives").energy - (1_500.0 - block.egg_cost)).abs() < 1e-3, "the layer pays egg_cost, not an adult's price");
        let egg = the_egg(&w);
        assert!(!w.live_organism_ids().contains(&egg), "an egg leaked into the animal census");
        assert_eq!(w.live_creature_count(), animals, "an egg was counted as an animal");
        assert_eq!(w.live_creature_groups().iter().map(|g| g.alive as usize).sum::<usize>(), animals);
        assert!((gap(&w) - g0).abs() < 1e-3, "laying moved the live identity by {}", gap(&w) - g0);
        let (ex, ey) = (site.x, site.y);
        assert_eq!(w.get(ex, ey).organism_id(), egg);

        // Too early: still an egg, rescheduled for when it is due.
        let sites = brood_tick(&mut w, &site);
        assert_eq!(w.organism(egg).and_then(|s| s.brood).map(|b| b.stage), Some(BroodStage::Egg));
        assert_eq!(sites[0].next_frame, block.egg_frames);

        w.frame = block.egg_frames;
        let sites = brood_tick(&mut w, &sites[0]);
        assert_eq!(w.organism(egg).and_then(|s| s.brood).map(|b| b.stage), Some(BroodStage::Larva));
        assert_eq!(w.get(ex, ey).shade, BroodStage::Larva as u8, "the cell's shade is its stage");

        // Food beside the larva: fruit is worth 960, so two bites fill it.
        let fruit = w.materials.id_of("fruit").expect("fruit");
        let mut site = sites[0];
        for _ in 0..6 {
            for (dx, dy) in [(0, -1), (1, -1), (-1, -1)] {
                if w.is_empty(ex + dx, ey + dy) {
                    w.set(ex + dx, ey + dy, Cell::new(fruit, 0));
                }
            }
            w.frame += LARVA_TICK;
            let next = brood_tick(&mut w, &site);
            site = next[0];
            if w.organism(egg).and_then(|s| s.brood).is_some_and(|b| b.stage == BroodStage::Pupa) {
                break;
            }
        }
        assert_eq!(w.organism(egg).and_then(|s| s.brood).map(|b| b.stage), Some(BroodStage::Pupa), "a larva with food beside it was not fed to its target");
        assert!(w.creature_stats.brood_ate_j > 0.0);
        assert!((gap(&w) - g0).abs() < 1e-2, "feeding moved the live identity by {}", gap(&w) - g0);

        // Hatch: an adult on the brood's own cell, the brood freed.
        let bank = w.organism(egg).expect("pupa").energy;
        w.frame += block.pupa_frames;
        let sites = brood_tick(&mut w, &site);
        assert_eq!(w.creature_stats.births, 1, "the hatching is the birth");
        assert!(w.organism(egg).is_none(), "the pupa's slot was not released");
        assert!(w.live_brood_ids().is_empty());
        let adult = w.get(ex, ey).organism_id();
        assert_ne!(adult, 0, "no adult stands on the brood's cell");
        assert_eq!(sites.len(), 1);
        assert_eq!(w.live_creature_count(), animals + 1);
        let stamp = def.body_energy * w.organism(adult).expect("adult").chain.len() as f32;
        assert!((w.organism(adult).expect("adult").energy - (bank - stamp)).abs() < 1e-2, "the adult starts with the bank less the body's stamp");
        assert!((gap(&w) - g0).abs() < 1e-2, "hatching moved the live identity by {}", gap(&w) - g0);
        assert_eq!(w.deaths_by_cause.iter().sum::<u64>(), 0, "a brood organism's release booked a death");
    }

    /// **An unfed larva starves without booking a death**: the brood columns
    /// move, the adult books do not, and the identity still closes.
    #[test]
    fn an_unfed_larva_starves_and_books_no_death() {
        let (mut w, ant, def) = bed(true);
        let block = def.brood.clone().expect("brood");
        let g0 = gap(&w);
        let site = creature::try_bud(&mut w, ant, &def, 0.0).expect("lays");
        let egg = the_egg(&w);
        w.frame = block.egg_frames;
        let sites = brood_tick(&mut w, &site);
        let deaths_before = w.deaths_by_cause.iter().sum::<u64>();
        let graves = w.graveyard.len();
        // Far past the bank's worth of upkeep, in one late tick.
        w.frame += (2.0 * block.egg_cost / block.larva_upkeep) as u64;
        let sites = brood_tick(&mut w, &sites[0]);
        assert!(sites.is_empty());
        assert!(w.organism(egg).is_none(), "the starved larva's slot was not released");
        assert_eq!(w.creature_stats.larvae_starved, 1);
        assert_eq!(w.deaths_by_cause.iter().sum::<u64>(), deaths_before, "a larva's starvation was booked as an animal's death");
        assert_eq!(w.graveyard.len(), graves, "a larva got a grave");
        assert!((gap(&w) - g0).abs() < 1e-2, "starvation moved the live identity by {}", gap(&w) - g0);
    }

    /// **A larva starved part-way leaves a corpse worth what it held**, which
    /// a nestmate can eat back.
    #[test]
    fn a_larva_that_starves_holding_energy_leaves_a_corpse_worth_it() {
        let (mut w, ant, def) = bed(true);
        let block = def.brood.clone().expect("brood");
        let site = creature::try_bud(&mut w, ant, &def, 0.0).expect("lays");
        let egg = the_egg(&w);
        w.frame = block.egg_frames;
        let sites = brood_tick(&mut w, &site);
        let (x, y) = (sites[0].x, sites[0].y);
        // Handed 300 J to die with, as a larva would be after a nurse
        // stopped coming: it leaves a corpse worth exactly that.
        let colony = w.colony_of(egg);
        larva_starves(&mut w, egg, (x, y), 300.0, colony);
        let corpse = w.materials.id_of("corpse").expect("corpse");
        assert_eq!(w.get(x, y).material, corpse);
        assert_eq!(w.get(x, y).aux(), 300);
        assert_eq!(w.creature_stats.brood_corpse_j, 300.0);
    }

    /// **Switched off, a species with a brood block buds a whole adult**,
    /// exactly as before brood existed.
    #[test]
    fn with_brood_off_the_ant_buds_a_whole_adult() {
        let (mut w, ant, def) = bed(false);
        let site = creature::try_bud(&mut w, ant, &def, 0.0).expect("a rich ant buds");
        assert_eq!(w.creature_stats.births, 1);
        assert_eq!(w.creature_stats.eggs_laid, 0);
        assert!(w.live_brood_ids().is_empty());
        let ActiveKind::Creature { organism: child } = site.kind else { panic!("not a creature site") };
        assert!(!w.organism(child).expect("child").chain.is_empty(), "the bud has no body");
    }

    /// **A larva reads as needy against its own target; an egg and a pupa are
    /// not fed.**
    #[test]
    fn a_larva_is_needy_against_its_target_and_an_egg_is_not_fed() {
        let (mut w, ant, def) = bed(true);
        let block = def.brood.clone().expect("brood");
        let site = creature::try_bud(&mut w, ant, &def, 0.0).expect("lays");
        let egg = the_egg(&w);
        assert_eq!(creature::kin_deficit(&w, egg, def.start_energy), None, "an egg is not fed");
        w.frame = block.egg_frames;
        brood_tick(&mut w, &site);
        let target = w.organism(egg).and_then(|s| s.brood).expect("larva").target;
        let need = creature::kin_deficit(&w, egg, def.start_energy).expect("a larva is fed");
        assert!((need - (1.0 - block.egg_cost / target)).abs() < 1e-4, "a larva's need reads against its target: {need}");
        // An adult reads exactly as it always did.
        let adult = creature::kin_deficit(&w, ant, def.start_energy).expect("adult");
        let energy = w.organism(ant).expect("adult").energy;
        assert!((adult - (1.0 - energy / def.start_energy).clamp(0.0, 1.0)).abs() < 1e-6);
    }

    /// **A destroyed brood cell is lost, its bank dissipated**, and no death
    /// is booked.
    #[test]
    fn a_destroyed_egg_is_lost_with_the_books_closed() {
        let (mut w, ant, def) = bed(true);
        let g0 = gap(&w);
        let site = creature::try_bud(&mut w, ant, &def, 0.0).expect("lays");
        let egg = the_egg(&w);
        w.set(site.x, site.y, Cell::EMPTY);
        let sites = brood_tick(&mut w, &site);
        assert!(sites.is_empty());
        assert!(w.organism(egg).is_none());
        assert_eq!(w.creature_stats.brood_lost, 1);
        assert_eq!(w.deaths_by_cause.iter().sum::<u64>(), 0);
        assert!((gap(&w) - g0).abs() < 1e-2, "a lost egg moved the live identity by {}", gap(&w) - g0);
    }
}
