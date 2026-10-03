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

/// **Is brood on, absent a per-world override?** On unless
/// `PIXEL_PHYSICS_BROOD=off`, since 2026-10-02 (the owner: "turn brood on by
/// default"). Off is the old budding, exactly: `brood_of` returns `None`
/// before anything is read. The measured cost, kept in view: on main
/// 5244c858 brood colonies were alive at the end of 23 of 24 long runs
/// against budding's 14 of 24, but in the lab box (`labforage played_bed`,
/// 24 seeds) births fell 566 -> 266 and the peak colony 309 -> 136.
fn brood_env() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("PIXEL_PHYSICS_BROOD").map_or(true, |v| v.trim() != "off"))
}

/// The species' brood block, when it has one and brood is on for this world
/// (`World::brood`, else `PIXEL_PHYSICS_BROOD`). `None` means bud a whole
/// adult as before.
pub fn brood_of(world: &World, def: &CreatureDef) -> Option<BroodDef> {
    world.brood.unwrap_or_else(brood_env).then(|| block_of(def)).flatten()
}

/// The species' brood block with the sweep overrides applied, whether or not
/// brood is on: what brood already laid runs by, so a world that switches
/// brood off mid-life still runs it to its end.
fn block_of(def: &CreatureDef) -> Option<BroodDef> {
    let block = def.brood.as_ref()?;
    Some({
        let mut b = block.clone();
        if let Some(v) = lay_at_env() {
            b.lay_at = v;
        }
        if let Some(v) = egg_cost_env() {
            b.egg_cost = v;
        }
        if let Some((e, p)) = stage_frames_env() {
            b.egg_frames = e;
            b.pupa_frames = p;
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

/// `PIXEL_PHYSICS_BROOD_FRAMES=<egg>,<pupa>`: override both stage
/// lengths, for the control that takes the delay away.
fn stage_frames_env() -> Option<(u64, u64)> {
    static V: std::sync::OnceLock<Option<(u64, u64)>> = std::sync::OnceLock::new();
    *V.get_or_init(|| {
        let v = std::env::var("PIXEL_PHYSICS_BROOD_FRAMES").ok()?;
        let (e, p) = v.split_once(',')?;
        Some((e.parse().ok()?, p.parse().ok()?))
    })
}

/// How far from the layer's head, in rings, an egg may be put down when
/// every neighbour is taken: `PIXEL_PHYSICS_LAY_REACH`, default 1 (the
/// eight neighbours only).
fn lay_reach() -> i32 {
    static V: std::sync::OnceLock<i32> = std::sync::OnceLock::new();
    *V.get_or_init(|| std::env::var("PIXEL_PHYSICS_LAY_REACH").ok().and_then(|v| v.parse().ok()).unwrap_or(1))
}

/// `PIXEL_PHYSICS_NURSE=off`: no feeding by touch ([`nurse`]), for the
/// control arm. Unset or anything else, on.
fn nurse_env() -> bool {
    static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *V.get_or_init(|| std::env::var("PIXEL_PHYSICS_NURSE").map_or(true, |v| v.trim() != "off"))
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
/// **How far an egg laid at the nest is carried to the brood pile**, in
/// steps through the crowd ([`pile_site`]): `PIXEL_PHYSICS_EGG_PILE`, 4 when
/// unset, `off` (or 0) for the egg beside the layer's head and "at the nest"
/// read off the layer's head alone, the rule before 2026-10-03.
pub fn egg_pile_reach() -> i32 {
    static V: std::sync::OnceLock<i32> = std::sync::OnceLock::new();
    *V.get_or_init(|| parse_egg_pile(&std::env::var("PIXEL_PHYSICS_EGG_PILE").unwrap_or_default()))
}

fn parse_egg_pile(raw: &str) -> i32 {
    match raw.trim() {
        "" | "on" => EGG_PILE_REACH,
        "off" => 0,
        v => v.parse().ok().filter(|r: &i32| *r >= 0).unwrap_or(EGG_PILE_REACH),
    }
}

/// The shipped reach of [`egg_pile_reach`]. 4 is the reach the oracle that
/// found the rule used (`laying-funnel-2026-10-03.md`): with the egg placed
/// up to four cells out and "at the nest" read four cells out, eggs laid at
/// the nest over four lab seeds went 49 -> 99 (graded suppression off in
/// both arms).
pub const EGG_PILE_REACH: i32 = 4;

/// **Where an egg laid at the nest goes: onto the brood pile**, or `None`
/// when there is nowhere at home to put one. Real ants keep their eggs in a
/// pile in a brood chamber and carry them there
/// (`Reports/ant-breeding-plan-2026-09-29.md`, the biology table, Franks &
/// Deneubourg 1997 as cited there); the egg here is put down on the pile in
/// the tick it is laid, rather than carried, which brood carry (B3b) would
/// replace.
///
/// **Why.** Traced 2026-10-03 on the lab box (`labforage played_bed`, 30k
/// frames, four seeds, main d4418bf2, `budtrace` with `creature::home_ring`):
/// with laying only at the nest, an ant that could afford an egg and stood
/// at the nest **never once** had an empty cell beside its head (0 of 308
/// samples), because the nest is a packed mound of nestmates, crumbs and
/// roots; and off the nest with home in view, the home cells nearest it were
/// all held by other ants on 58-93% of samples. Both walls are the same
/// fact -- the few cells that count as home are occupied -- so one rule
/// answers both: the egg, not the layer, has to land at home.
///
/// **The rule.** A breadth-first walk of up to `reach` steps out from the
/// head, through cells a body could pass -- empty, or another animal (the
/// crowd the egg is handed through) -- to an **empty home cell**
/// (`creature::home_at`, whatever the live home definition is). Among those,
/// one touching brood already lying there wins (the pile), then the fewest
/// steps; ties go to the walk's own `DIRS` order, so it is deterministic.
/// Runs only on the rare tick an animal could otherwise already lay.
pub(super) fn pile_site(world: &World, head: (i32, i32), def: &CreatureDef, brood: &BroodDef, reach: i32) -> Option<(i32, i32)> {
    let material = world.materials.id_of(&brood.material)?;
    let (hx, hy) = head;
    let side = 2 * reach + 1;
    let index = |x: i32, y: i32| ((y - hy + reach) * side + (x - hx + reach)) as usize;
    let mut seen = vec![false; (side * side) as usize];
    seen[index(hx, hy)] = true;
    let mut frontier = vec![(hx, hy)];
    let mut best: Option<((bool, i32), (i32, i32))> = None;
    for depth in 1..=reach {
        let mut next = Vec::new();
        for &(x, y) in &frontier {
            for (dx, dy) in creature::DIRS {
                let (nx, ny) = (x + dx, y + dy);
                if (nx - hx).abs() > reach || (ny - hy).abs() > reach || !world.in_bounds(nx, ny) || seen[index(nx, ny)] {
                    continue;
                }
                seen[index(nx, ny)] = true;
                let empty = world.is_empty(nx, ny);
                if !empty && world.materials.get(world.get(nx, ny).material).kind != super::material::MaterialKind::Creature {
                    continue;
                }
                next.push((nx, ny));
                if empty && creature::home_at(world, nx, ny, def) {
                    let on_pile = creature::DIRS.iter().any(|&(px, py)| world.get(nx + px, ny + py).material == material);
                    let key = (!on_pile, depth);
                    if best.is_none_or(|(k, _)| key < k) {
                        best = Some((key, (nx, ny)));
                    }
                }
            }
        }
        // A cell on the pile at this depth cannot be beaten further out.
        if best.is_some_and(|((off_pile, _), _)| !off_pile) {
            break;
        }
        frontier = next;
    }
    best.map(|(_, cell)| cell)
}

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
pub(super) fn lay_egg(world: &mut World, parent: OrganismId, head: (i32, i32), def: &CreatureDef, brood: &BroodDef, egg: Egg, at: Option<(i32, i32)>) -> Option<ActiveSite> {
    let material = world.materials.id_of(&brood.material)?;
    let (hx, hy) = head;
    let ring = |r: i32| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| (dx, dy))).filter(move |&(dx, dy)| dx.abs().max(dy.abs()) == r);
    // **On the brood pile when the caller found one** ([`pile_site`]), else
    // the first empty cell beside the head.
    let (ex, ey) = match at {
        Some(cell) => cell,
        None => creature::DIRS
            .iter()
            .copied()
            .chain((2..=lay_reach()).flat_map(ring))
            .map(|(dx, dy)| (hx + dx, hy + dy))
            .find(|&(x, y)| world.is_empty(x, y))?,
    };
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
    let Some(block) = block_of(&def) else {
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
            if nurse_env() {
                nurse(world, organism, (x, y), colony, &def, b.target);
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

/// **Fed by touch**: the richest grown nestmate standing on one of the
/// larva's eight neighbours hands it a quarter of what it holds above its
/// own grant, capped at what the larva still lacks -- the same quarter and
/// the same floor as a brain's `Share`, without waiting for a brain to
/// choose it. One donor a larva tick.
///
/// Why it is not left to `Share`: measured with eggs laid at 1,040 J
/// against a target near 1,060 (main 44f14af, food box, 4 seeds, 96,000
/// frames, no stage delay), 38-50 larvae stood unfinished at every stop,
/// holding ~40 kJ between them -- about forty ants -- and waiting tens of
/// thousands of frames for ~20 J each. A share is a brain decision taken
/// by an ant whose `KinNeed` happened to name the larva, and almost none
/// did: 3-11 kJ shared in over the whole run. The colony grew with budding
/// to 48,000 frames and then fell away from it (live 82-308 at 96,000
/// against budding's 302-635). Real nurses feed the brood they walk over.
///
/// It draws from an adult's bank on its way to its own next egg, so a nest
/// with brood waiting finishes them before it lays more: the regulation is
/// a side effect, not a rule.
fn nurse(world: &mut World, larva: OrganismId, (x, y): (i32, i32), colony: u32, def: &CreatureDef, target: f32) {
    let start_energy = def.start_energy;
    // **Kin by scent, as every share is** (`is_living_kin_id`), not by colony
    // id, so a species with no colony can nurse too. Not a lab fix: the lab
    // box (main 34b46b64, 24 seeds) gave identical births per seed either
    // way, because its ants do carry a colony.
    let gut = creature::gut_of(world, larva, def);
    let need = target - world.organism(larva).map_or(target, |s| s.energy);
    if need <= 0.0 {
        return;
    }
    let mut best: Option<(OrganismId, f32)> = None;
    for (dx, dy) in super::structural::NEIGHBOURS_8 {
        let c = world.get(x + dx, y + dy);
        let id = c.organism_id();
        if id == 0 {
            continue;
        }
        if id == larva || best.is_some_and(|(b, _)| b == id) {
            continue;
        }
        let Some(st) = world.organism(id) else { continue };
        if st.brood.is_some() || st.energy <= start_energy || !creature::is_living_kin_id(world, id, gut) {
            continue;
        }
        if best.is_none_or(|(_, e)| st.energy > e) {
            best = Some((id, st.energy));
        }
    }
    let Some((donor, mine)) = best else { return };
    let amount = (creature::SHARE_FRACTION * (mine - start_energy)).min(need);
    if amount <= 0.0 {
        return;
    }
    if let Some(s) = world.organism_mut(donor) {
        s.energy -= amount;
    }
    if let Some(s) = world.organism_mut(larva) {
        s.energy += amount;
    }
    world.creature_stats.brood_nursed_j += amount as f64;
    let donor_colony = world.colony_of(donor);
    world.book(donor_colony, Account::SharedOut, amount as f64);
    world.book(colony, Account::SharedIn, amount as f64);
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
    //
    // **Then the same rings again, standing on a nestmate**, budding's own
    // second pass ([`creature::bud_stack_of`]). Without it a hatchling had
    // free ground or nothing, while 85-94% of budded births on the food box
    // land on kin. Measured with the stage delay off and laying reach 3
    // (main 44f14af, 4 seeds, 96,000 frames, egg cost 1,040): born 355-597
    // against budding's 751-1,234, 0 on kin, 393 hatches refused for room.
    let mut placed = None;
    'passes: for on_kin in [false, true] {
    for r in 0..=HATCH_REACH {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                let (hx, hy) = (x + dx, y + dy);
                if r > 0 && !on_kin && !world.is_empty(hx, hy) {
                    continue;
                }
                for facing_west in [false, true] {
                    placed = creature::place_hatchling(world, hx, hy, species, def, facing_west, b.parent, genome.clone(), traits, generation, lineage, colony, made, fates, bank, on_kin);
                    if placed.is_some() {
                        if on_kin {
                            world.creature_stats.births_on_kin += 1;
                        }
                        break 'passes;
                    }
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
        // No nest on this bed, so laying anywhere (`creature::bud_at_nest`).
        w.bud_at_nest = Some(false);
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

    /// The `bed` with laying only at the nest, and nest material set into the
    /// floor at `nest_x` -- the cells above it are home.
    fn nest_bed(nest_x: i32) -> (World, OrganismId, CreatureDef, (i32, i32)) {
        let (mut w, ant, def) = bed(true);
        w.bud_at_nest = Some(true);
        let nest = w.materials.id_of(&def.nest).expect("nest material");
        w.set(nest_x, 101, Cell::new(nest, 0));
        let head = w.organism(ant).expect("live").chain[0];
        (w, ant, def, head)
    }

    /// **An ant a few steps from home lays onto it** (`pile_site`): the egg
    /// lands on an empty home cell, where the head-only read would have held
    /// the birth -- the lab's packed nest, where no ant at home ever had an
    /// empty cell beside it (2026-10-03).
    #[test]
    fn an_ant_a_few_steps_from_home_lays_its_egg_at_home() {
        let (mut w, ant, def, head) = nest_bed(104);
        assert!(!creature::home_at(&w, head.0, head.1, &def), "test setup: the ant already stands at home");
        let block = def.brood.clone().expect("brood");
        assert!(pile_site(&w, head, &def, &block, 0).is_none(), "a reach of 0 must find nothing");
        let site = creature::try_bud(&mut w, ant, &def, 0.0).expect("an ant four steps from home lays");
        assert_eq!(w.creature_stats.eggs_laid, 1);
        assert!(creature::home_at(&w, site.x, site.y, &def), "the egg landed off home at {:?}", (site.x, site.y));
        assert!((site.x - head.0).abs().max((site.y - head.1).abs()) <= EGG_PILE_REACH);
    }

    /// **Home out of reach holds the egg**, and says so in the counter the
    /// head-only rule used.
    #[test]
    fn home_out_of_reach_holds_the_egg() {
        let (mut w, ant, def, _) = nest_bed(112);
        assert!(creature::try_bud(&mut w, ant, &def, 0.0).is_none(), "laid with home eleven cells away");
        assert_eq!(w.creature_stats.eggs_laid, 0);
        assert_eq!(w.creature_stats.buds_held_for_nest, 1);
    }

    /// **The egg is handed through the crowd, never through rock**: a wall
    /// between the ant and home, with no way round inside the reach, holds it.
    #[test]
    fn the_egg_is_not_put_through_a_wall() {
        let (mut w, ant, def, head) = nest_bed(104);
        for y in 90..=100 {
            w.set(head.0 + 2, y, Cell::new(material::STONE, 0));
        }
        let block = def.brood.clone().expect("brood");
        assert!(pile_site(&w, head, &def, &block, EGG_PILE_REACH).is_none(), "the walk went through stone");
        assert!(creature::try_bud(&mut w, ant, &def, 0.0).is_none());
    }

    /// **Brood already at home draws the next egg to it** -- the pile -- even
    /// when an empty home cell is nearer.
    #[test]
    fn a_new_egg_joins_the_brood_already_lying_at_home() {
        let (mut w, _, def, head) = nest_bed(104);
        let nest = w.materials.id_of(&def.nest).expect("nest");
        w.set(head.0 + 2, 101, Cell::new(nest, 0));
        let block = def.brood.clone().expect("brood");
        let material = w.materials.id_of(&block.material).expect("brood material");
        // A brood cell at the far end of home, on the floor.
        w.set(head.0 + 5, 100, Cell::new(material, 0));
        let cell = pile_site(&w, head, &def, &block, EGG_PILE_REACH).expect("home in reach");
        assert!(creature::DIRS.iter().any(|&(dx, dy)| w.get(cell.0 + dx, cell.1 + dy).material == material), "the egg at {cell:?} is not on the pile");
    }

    /// `PIXEL_PHYSICS_EGG_PILE`'s value.
    #[test]
    fn egg_pile_parses_off_a_reach_and_garbage() {
        assert_eq!(parse_egg_pile(""), EGG_PILE_REACH);
        assert_eq!(parse_egg_pile("on"), EGG_PILE_REACH);
        assert_eq!(parse_egg_pile("off"), 0);
        assert_eq!(parse_egg_pile("6"), 6);
        assert_eq!(parse_egg_pile("-2"), EGG_PILE_REACH);
        assert_eq!(parse_egg_pile("wide"), EGG_PILE_REACH);
    }
}
