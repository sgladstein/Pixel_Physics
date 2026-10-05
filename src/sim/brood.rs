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

/// **Where an egg is never put down**: `PIXEL_PHYSICS_EGG_DOOR`. The
/// laying lane's first brood-room rule (Scott, 2026-10-03: "dig an area and
/// food can all be placed together in that chamber and dig another area
/// and brood can all be placed together in that chamber").
///
/// - `off` (**the default**): anywhere at home, the rule before 2026-10-03.
/// - `door`: not in a nest's way in -- the shaft and the ground round its
///   mouth, the same cells a food drop keeps clear (`creature::in_doorway`),
///   at the layer's own `TRAIT_DOOR_CLEAR`.
/// - `cut`: not in the way in, and nowhere in the founding cut -- shaft,
///   chamber (the storeroom, where the food goes) and side room. Brood then
///   lies only in ground the colony dug itself, and with none dug in reach
///   the egg is not laid.
/// - `deep`: not in the way in, and **the founding cut only when nothing
///   the colony dug is in reach** -- the graded form of `cut`: a home cell
///   outside the cut beats any inside it, so eggs go to the chamber while
///   it is all there is and to dug ground once there is some, and
///   carrying ([`carry`]) takes brood out of the cut when it can.
///
/// **Why.** On the nest lane's test bed (`digbox`, 40 ants, larvae fed,
/// 2026-10-03) eggs laid at home filled the 26-cell founding cut: 22 eggs
/// held 16 of its cells by frame 2,000, and the colony dug 0 cells by
/// 26,000 and 13 by 40,000 against ~97 by 12,000 with brood off, while
/// 1,252 hatches were refused for lack of room. Real colonies keep the way
/// in clear and the brood in its own chambers, and those chambers grow
/// round the brood (Römer & Roces 2014, doi 10.1371/journal.pone.0097872:
/// workers relocate brood, aggregate where it lies and dig more there).
///
/// **Measured** (2026-10-03, main a707e0a1, the shipped rule against each
/// mode, paired by seed; `/mnt/project-files/laying/egg-door/` in the
/// project). Test bed (`digbox ants=40 surplus=500`, 6 seeds, 40k frames):
/// shipped dug 0 cells and grew to 178 ants with 12 brood in the 12-cell
/// shaft; `door` dug 264 (more on 6 of 6) and grew to 500 (4 of 6; two
/// seeds jam with brood filling the chamber before anything is dug); `cut`
/// dug 346 and grew to 641 (6 of 6); `deep` was identical to `door` on all
/// six, since the brood pile already sits beyond the cut once anything is
/// dug. Lab (`nestdoor played_bed`, 12 seeds, 150k, laying only at the
/// nest): births 76 shipped, `door` 88 (more on 8 of 12), `deep` 77, `cut`
/// **18** (fewer on 10) with 4 of 12 boxes alive at the end against 11 --
/// the lab colony has almost nothing dug beyond the cut, so `cut` mostly
/// means no egg. `door` was the one mode worse nowhere.
///
/// **Then ants learned to walk through brood** (`creature::PushPast`, on
/// since 2026-10-03), and the jam this was built for went away without it:
/// on the same test bed the shipped rule grew to 3,139 ants and dug 1,811
/// cells by 40k (6 seeds). `door` on top **slowed early growth on every
/// seed** -- ants at 12k 733 -> 258, births 703 -> 222, 6 of 6 -- and was
/// still behind at 40k (2,862 ants; lower on 3 of 6, digs lower on 4). In
/// the lab (4 seeds, 150k) births went 40 -> 51 median (more on 3 of 4), ants
/// at the end lower on 2, higher on 1. So all three modes ship off, kept as
/// switches for when brood has a reason not to be walked through.
///
/// Unknown values panic (a mistyped switch must not fail open).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EggDoor {
    Off,
    Door,
    Cut,
    Deep,
}

pub fn egg_door() -> EggDoor {
    static V: std::sync::OnceLock<EggDoor> = std::sync::OnceLock::new();
    *V.get_or_init(|| parse_egg_door(&std::env::var("PIXEL_PHYSICS_EGG_DOOR").unwrap_or_default()))
}

fn parse_egg_door(raw: &str) -> EggDoor {
    match raw.trim() {
        "" | "off" => EggDoor::Off,
        "door" => EggDoor::Door,
        "cut" => EggDoor::Cut,
        "deep" => EggDoor::Deep,
        other => panic!("PIXEL_PHYSICS_EGG_DOOR={other:?}: use off, door, cut or deep"),
    }
}

/// **The cells an egg laid by one ant may not take** ([`EggDoor`]): the
/// mode, and how far round the door that ant keeps clear.
#[derive(Clone, Copy, Debug)]
pub(super) struct EggBar {
    pub mode: EggDoor,
    pub clear: i32,
}

impl EggBar {
    /// The live mode, at `layer`'s own door allele.
    pub(super) fn of(world: &World, layer: OrganismId) -> Self {
        Self::with(world, layer, egg_door())
    }

    /// `mode`, at `layer`'s own door allele.
    pub(super) fn with(world: &World, layer: OrganismId, mode: EggDoor) -> Self {
        let clear = if mode == EggDoor::Off { 0 } else { creature::door_clear_cells(world, layer) };
        Self { mode, clear }
    }

    #[cfg(test)]
    pub(super) const OFF: Self = Self { mode: EggDoor::Off, clear: 0 };

    /// Whether an egg may not be put down at `(x, y)`. One scan over the
    /// world's few nest sites; `off` reads nothing.
    pub(super) fn bars(&self, world: &World, (x, y): (i32, i32)) -> bool {
        match self.mode {
            EggDoor::Off => false,
            EggDoor::Door | EggDoor::Deep => creature::in_doorway(world, (x, y), self.clear),
            EggDoor::Cut => creature::in_doorway(world, (x, y), self.clear) || in_founding_cut(world, (x, y)),
        }
    }

    /// Whether `(x, y)` is allowed but second best: under `deep`, a cell of
    /// a founding cut, taken only when nothing outside one is in reach.
    pub(super) fn shuns(&self, world: &World, (x, y): (i32, i32)) -> bool {
        self.mode == EggDoor::Deep && in_founding_cut(world, (x, y))
    }
}

fn in_founding_cut(world: &World, (x, y): (i32, i32)) -> bool {
    world.nest_sites.iter().filter_map(|s| s.shaft).any(|c| c.contains(x, y))
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
///
/// **A cell the egg bar refuses ([`EggBar`]) is walked through, never
/// chosen**: the egg is handed past the doorway to the home beyond it. A
/// cell it shuns (`deep`'s founding cut) loses to any it does not, ahead of
/// the pile.
pub(super) fn pile_site(world: &World, head: (i32, i32), def: &CreatureDef, brood: &BroodDef, reach: i32, bar: EggBar) -> Option<(i32, i32)> {
    let material = world.materials.id_of(&brood.material)?;
    let (hx, hy) = head;
    let side = 2 * reach + 1;
    let index = |x: i32, y: i32| ((y - hy + reach) * side + (x - hx + reach)) as usize;
    let mut seen = vec![false; (side * side) as usize];
    seen[index(hx, hy)] = true;
    let mut frontier = vec![(hx, hy)];
    // (shunned, off the pile, steps): the least wins.
    type Key = (bool, bool, i32);
    let mut best: Option<(Key, (i32, i32))> = None;
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
                if empty && creature::home_at(world, nx, ny, def) && !bar.bars(world, (nx, ny)) {
                    let on_pile = creature::DIRS.iter().any(|&(px, py)| world.get(nx + px, ny + py).material == material);
                    let key = (bar.shuns(world, (nx, ny)), !on_pile, depth);
                    if best.is_none_or(|(k, _)| key < k) {
                        best = Some((key, (nx, ny)));
                    }
                }
            }
        }
        // A cell on the pile, and not shunned, at this depth cannot be
        // beaten further out.
        if best.is_some_and(|((shunned, off_pile, _), _)| !shunned && !off_pile) {
            break;
        }
        frontier = next;
    }
    best.map(|(_, cell)| cell)
}

/// **How far a nestmate carries brood in one move**, in steps through the
/// crowd ([`carry`]): `PIXEL_PHYSICS_BROOD_CARRY`, `off` (0) by default;
/// `on` is [`BROOD_CARRY_REACH`]; a number is that reach. Unknown values
/// panic (a mistyped switch must not fail open).
///
/// **Off because it is close to inert** (2026-10-03, main a707e0a1): the
/// egg pile already lays eggs beside eggs, so a carrier rarely finds a
/// better place -- 0 to 19 moves in a 40k test-bed run. Carrying alone
/// left ants at 40k lower on 3 of 6 test-bed seeds and higher on none
/// (176 against 178), and with `deep` in the lab births went 77 -> 66
/// (more on 6 of 12): no gain, a hint of cost. Worth re-measuring once
/// something scatters brood that a pile rule cannot gather.
pub fn brood_carry_reach() -> i32 {
    static V: std::sync::OnceLock<i32> = std::sync::OnceLock::new();
    *V.get_or_init(|| parse_brood_carry(&std::env::var("PIXEL_PHYSICS_BROOD_CARRY").unwrap_or_default()))
}

fn parse_brood_carry(raw: &str) -> i32 {
    match raw.trim() {
        "" | "off" => 0,
        "on" => BROOD_CARRY_REACH,
        v => v.parse().ok().filter(|r: &i32| *r >= 0).unwrap_or_else(|| panic!("PIXEL_PHYSICS_BROOD_CARRY={v:?}: use off, on or a reach")),
    }
}

/// The reach `on` means: three steps, the span of an ant and a half, so one
/// move is a short carry rather than a jump across the nest.
pub const BROOD_CARRY_REACH: i32 = 3;

/// **Brood carried to brood** ([`brood_carry_reach`]): at a brood item's
/// tick, a grown nestmate touching it with free jaws moves it, if there is
/// a better place within a short carry. Returns where it lies now.
///
/// **Why.** Brood was put down where it was laid and never moved, so eggs
/// laid at home stacked in the founding cut and plugged it (the nest lane's
/// test bed, 2026-10-03), and the owner's design is a brood chamber apart
/// from the food: "dig an area and brood can all be placed together in that
/// chamber". Real workers move brood constantly, and the piles they keep
/// come from local rules, not a plan: many ant species keep their brood
/// sorted and clustered by picking an item up where few like it lie and
/// putting it down where many do (Holland & Melhuish 1999, doi
/// 10.1162/106454699568737, reviewing the brood-sorting work; Franks &
/// Sendova-Franks 1992 on *Leptothorax*). Workers relocate brood to suitable
/// sites, gather where it lies and dig more there, so chambers emerge round
/// it (Römer & Roces 2014, doi 10.1371/journal.pone.0097872).
///
/// **The rule**, a deterministic form of that pick-up/put-down:
///
/// - **The carrier** is the first grown kin nestmate on one of the item's
///   eight neighbours (`DIRS` order) that holds no pellet. No carrier, no
///   move: brood nobody tends stays where it is.
/// - **The walk**: breadth-first from the item, up to `reach` steps through
///   cells a carrier could pass -- empty, an animal, or brood (ants climb
///   over the pile).
/// - **Where it may go**: an empty cell with a floor under it (solid,
///   powder or plant, so it lies on the ground rather than dropping through
///   a body), that the egg bar ([`EggBar`], at the carrier's door gene)
///   allows, and at home if it is at home now.
/// - **Better** is, in order: out of a cell the bar refuses; onto home from
///   off it; out of a cell the bar shuns (`deep`'s founding cut), never
///   into one; next to strictly more brood than it touches now. Among better
///   cells, home, then unshunned, then the most brood wins, then the fewest
///   steps.
///
/// It always ends: every move but the three one-way kinds strictly raises the
/// number of touching brood pairs, which is bounded. One walk of at most
/// `(2 * reach + 1)^2` cells per brood tick with a carrier beside it.
pub(super) fn carry(world: &mut World, organism: OrganismId, (x, y): (i32, i32), material: super::material::MaterialId, def: &CreatureDef, reach: i32, door: EggDoor) -> (i32, i32) {
    use super::material::MaterialKind;
    if reach <= 0 {
        return (x, y);
    }
    let gut = creature::gut_of(world, organism, def);
    let carrier = creature::DIRS.iter().find_map(|&(dx, dy)| {
        let id = world.get(x + dx, y + dy).organism_id();
        if id == 0 || id == organism {
            return None;
        }
        let st = world.organism(id)?;
        (st.brood.is_none() && st.spoil.is_none() && creature::is_living_kin_id(world, id, gut)).then_some(id)
    });
    let Some(carrier) = carrier else {
        return (x, y);
    };
    // Only brood lying in its own cell is carried: one a walker is standing
    // on (`creature::PushPast` holds it out of the grid) is not there to
    // pick up, and the cell at `(x, y)` is the walker's.
    let here = world.get(x, y);
    if here.material != material || here.organism_id() != organism {
        return (x, y);
    }
    let bar = EggBar::with(world, carrier, door);
    let touching = |w: &World, (px, py): (i32, i32)| {
        creature::DIRS.iter().filter(|&&(dx, dy)| (px + dx, py + dy) != (x, y) && w.get(px + dx, py + dy).material == material).count() as i32
    };
    let floored = |w: &World, (px, py): (i32, i32)| {
        w.in_bounds(px, py + 1) && matches!(w.materials.kind(w.get(px, py + 1).material), MaterialKind::Solid | MaterialKind::Powder | MaterialKind::Plant)
    };
    let here_barred = bar.bars(world, (x, y));
    let here_shunned = bar.shuns(world, (x, y));
    let here_home = creature::home_at(world, x, y, def);
    let here_touching = touching(world, (x, y));
    let side = 2 * reach + 1;
    let index = |px: i32, py: i32| ((py - y + reach) * side + (px - x + reach)) as usize;
    let mut seen = vec![false; (side * side) as usize];
    seen[index(x, y)] = true;
    let mut frontier = vec![(x, y)];
    // (home, not shunned, brood touching, -steps): the most wins.
    type Key = (bool, bool, i32, i32);
    let mut best: Option<(Key, (i32, i32))> = None;
    for depth in 1..=reach {
        let mut next = Vec::new();
        for &(fx, fy) in &frontier {
            for (dx, dy) in creature::DIRS {
                let (nx, ny) = (fx + dx, fy + dy);
                if (nx - x).abs() > reach || (ny - y).abs() > reach || !world.in_bounds(nx, ny) || seen[index(nx, ny)] {
                    continue;
                }
                seen[index(nx, ny)] = true;
                let c = world.get(nx, ny);
                let empty = world.is_empty(nx, ny);
                if !empty && c.material != material && world.materials.kind(c.material) != MaterialKind::Creature {
                    continue;
                }
                next.push((nx, ny));
                if !empty || !floored(world, (nx, ny)) || bar.bars(world, (nx, ny)) {
                    continue;
                }
                let home = creature::home_at(world, nx, ny, def);
                let shunned = bar.shuns(world, (nx, ny));
                if (here_home && !home) || (shunned && !here_shunned) {
                    continue;
                }
                let t = touching(world, (nx, ny));
                let better = here_barred || (home && !here_home) || (home == here_home && here_shunned && !shunned) || (home == here_home && shunned == here_shunned && t > here_touching);
                let key = (home, !shunned, t, -depth);
                if better && best.is_none_or(|(k, _)| key > k) {
                    best = Some((key, (nx, ny)));
                }
            }
        }
        frontier = next;
    }
    let Some((_, (tx, ty))) = best else {
        return (x, y);
    };
    world.set(tx, ty, here);
    world.set(x, y, super::cell::Cell::EMPTY);
    world.creature_stats.brood_carried += 1;
    (tx, ty)
}

/// **How crowded a brood pile gets before nestmates spread it**
/// ([`spread`]): `PIXEL_PHYSICS_BROOD_SPREAD`, **off by default**; `on` is
/// [`BROOD_SPREAD_CROWD`]; a number of at least 3 is the crowd. Unknown
/// values panic (a mistyped switch must not fail open).
///
/// **What it is for**: eggs laid at home stack where they are put down,
/// which on the goal box is a pink column standing in the shaft (Scott,
/// 22:52 on 2026-10-03: "Brood form a tall column coming straight down the
/// tunnel. This doesn't look right"). With the rule the brood lies spread
/// over the chamber floor.
///
/// **Off because it costs colonies** (2026-10-04, main 0b3e264a, measured
/// in the commit that added it): larvae carried out of the crowd are fed
/// less -- food beside them and mouth-to-mouth food both fall -- and more of
/// them starve, so fewer colonies get through the boom. Nurses that seek
/// hungry larvae by scent ([`nurse_seek`]) did not make it up.
pub fn brood_spread() -> Option<i32> {
    static V: std::sync::OnceLock<Option<i32>> = std::sync::OnceLock::new();
    *V.get_or_init(|| parse_brood_spread(&std::env::var("PIXEL_PHYSICS_BROOD_SPREAD").unwrap_or_default()))
}

fn parse_brood_spread(raw: &str) -> Option<i32> {
    match raw.trim() {
        "on" => Some(BROOD_SPREAD_CROWD),
        "" | "off" => None,
        v => Some(v.parse().ok().filter(|c: &i32| *c >= 3).unwrap_or_else(|| panic!("PIXEL_PHYSICS_BROOD_SPREAD={v:?}: use on, off or a crowd of 3 or more"))),
    }
}

/// The crowd `on` means: an item with 6 of the 24 cells round it holding
/// brood is in a pile a quarter full or more, two deep against the floor.
pub const BROOD_SPREAD_CROWD: i32 = 6;
/// How far a crowded item may be carried, in steps through the nest.
pub const BROOD_SPREAD_REACH: i32 = 10;
/// The nearest it is put down, in steps: past the pile's own edge, so a move
/// thins the pile rather than shuffling it.
pub const BROOD_SPREAD_MIN: i32 = 5;
/// No loose food within this many cells of where it is put down.
pub const BROOD_SPREAD_FOOD: i32 = 3;

/// **A crowded brood pile is spread out** ([`brood_spread`]): at a brood
/// item's tick, if `crowd` or more of the 24 cells round it hold brood and
/// a grown nestmate touching it has free jaws, the nestmate carries it to a
/// quieter spot of the nest, away from food. Returns where it lies now.
///
/// **Why.** Eggs are laid onto the pile ([`pile_site`]) and stay where they
/// are put, so a busy colony's brood grows as one stack -- in a narrow
/// shaft, a column up its middle. Real workers do not keep brood packed:
/// they space it out by need over the floor of the brood chamber, the
/// pick-up/put-down rule that sorts and spreads it (Franks &
/// Sendova-Franks 1992 on *Leptothorax*; Holland & Melhuish 1999, doi
/// 10.1162/106454699568737), and a harvester-ant nest keeps its brood over
/// several chambers (Tschinkel 2004, doi 10.1093/jis/4.1.21).
///
/// **The rule**:
///
/// - **Crowded**: at least `crowd` of the 24 cells round the item (its
///   5x5, itself left out) hold brood.
/// - **The carrier** is the first grown kin nestmate on one of the item's
///   eight neighbours that holds no pellet, as for [`carry`].
/// - **The walk**: breadth-first from the item, up to
///   [`BROOD_SPREAD_REACH`] steps through cells a carrier could pass.
/// - **Where it may go**: at least [`BROOD_SPREAD_MIN`] steps out, an empty
///   cell with a floor under it, at home, that the egg bar allows and does
///   not shun, with at most `crowd - 2` brood round it and no loose food
///   within [`BROOD_SPREAD_FOOD`] cells.
/// - **Which**: the most brood round it (an item joins the last one put
///   down there, so a thinned pile is a pile, not a scatter), then the most
///   steps.
///
/// It always ends: every move takes an item from `crowd` or more brood
/// round it to at most `crowd - 2`, so the number of brood pairs within two
/// cells of each other falls by at least two. One walk of at most
/// `(2 * BROOD_SPREAD_REACH + 1)^2` cells per crowded brood tick with a
/// carrier beside it.
///
/// **It does not make a second room on its own** (2026-10-04, with the nest
/// lane's two digging modes, which widen only round brood or food): spread
/// brood is dug round until it joins the room it came from, and carrying it
/// farther, or only past a narrow passage, gave one room every time
/// (`Reports/dead-ends.md`).
pub(super) fn spread(world: &mut World, organism: OrganismId, (x, y): (i32, i32), material: super::material::MaterialId, def: &CreatureDef, crowd: Option<i32>, door: EggDoor) -> (i32, i32) {
    use super::material::MaterialKind;
    let Some(crowd) = crowd else {
        return (x, y);
    };
    let here = world.get(x, y);
    if here.material != material || here.organism_id() != organism {
        return (x, y);
    }
    // Brood round a cell, the item itself left out wherever it is counted
    // from.
    let round = |w: &World, (px, py): (i32, i32)| {
        let mut n = 0;
        for dy in -2..=2 {
            for dx in -2..=2 {
                if (dx, dy) != (0, 0) && (px + dx, py + dy) != (x, y) && w.get(px + dx, py + dy).material == material {
                    n += 1;
                }
            }
        }
        n
    };
    if round(world, (x, y)) < crowd {
        return (x, y);
    }
    let gut = creature::gut_of(world, organism, def);
    let carrier = creature::DIRS.iter().find_map(|&(dx, dy)| {
        let id = world.get(x + dx, y + dy).organism_id();
        if id == 0 || id == organism {
            return None;
        }
        let st = world.organism(id)?;
        (st.brood.is_none() && st.spoil.is_none() && creature::is_living_kin_id(world, id, gut)).then_some(id)
    });
    let Some(carrier) = carrier else {
        return (x, y);
    };
    let bar = EggBar::with(world, carrier, door);
    let food_near = |w: &World, (px, py): (i32, i32)| {
        (-BROOD_SPREAD_FOOD..=BROOD_SPREAD_FOOD).any(|dy| {
            (-BROOD_SPREAD_FOOD..=BROOD_SPREAD_FOOD).any(|dx| {
                let c = w.get(px + dx, py + dy);
                c.organism_id() == 0 && c.material != super::material::EMPTY && w.materials.get(c.material).food_energy > 0.0
            })
        })
    };
    let floored = |w: &World, (px, py): (i32, i32)| {
        w.in_bounds(px, py + 1) && matches!(w.materials.kind(w.get(px, py + 1).material), MaterialKind::Solid | MaterialKind::Powder | MaterialKind::Plant)
    };
    let reach = BROOD_SPREAD_REACH;
    let side = 2 * reach + 1;
    let index = |px: i32, py: i32| ((py - y + reach) * side + (px - x + reach)) as usize;
    let mut seen = vec![false; (side * side) as usize];
    seen[index(x, y)] = true;
    let mut frontier = vec![(x, y)];
    // (brood round it, steps): the most wins.
    let mut best: Option<((i32, i32), (i32, i32))> = None;
    for depth in 1..=reach {
        let mut next = Vec::new();
        for &(fx, fy) in &frontier {
            for (dx, dy) in creature::DIRS {
                let (nx, ny) = (fx + dx, fy + dy);
                if (nx - x).abs() > reach || (ny - y).abs() > reach || !world.in_bounds(nx, ny) || seen[index(nx, ny)] {
                    continue;
                }
                seen[index(nx, ny)] = true;
                let c = world.get(nx, ny);
                let empty = world.is_empty(nx, ny);
                if !empty && c.material != material && world.materials.kind(c.material) != MaterialKind::Creature {
                    continue;
                }
                next.push((nx, ny));
                if depth < BROOD_SPREAD_MIN || !empty || !floored(world, (nx, ny)) || bar.bars(world, (nx, ny)) || bar.shuns(world, (nx, ny)) || !creature::home_at(world, nx, ny, def) {
                    continue;
                }
                let n = round(world, (nx, ny));
                if n > crowd - 2 || food_near(world, (nx, ny)) {
                    continue;
                }
                let key = (n, depth);
                if best.is_none_or(|(k, _)| key > k) {
                    best = Some((key, (nx, ny)));
                }
            }
        }
        frontier = next;
    }
    let Some((_, (tx, ty))) = best else {
        return (x, y);
    };
    world.set(tx, ty, here);
    world.set(x, y, super::cell::Cell::EMPTY);
    world.creature_stats.brood_spread += 1;
    (tx, ty)
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

/// **The first empty cell beside the head an egg may take** ([`EggBar`]):
/// the eight neighbours in `DIRS` order, then rings out to
/// `PIXEL_PHYSICS_LAY_REACH`.
fn beside_head(world: &World, (hx, hy): (i32, i32), bar: EggBar) -> Option<(i32, i32)> {
    let ring = |r: i32| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| (dx, dy))).filter(move |&(dx, dy)| dx.abs().max(dy.abs()) == r);
    creature::DIRS
        .iter()
        .copied()
        .chain((2..=lay_reach()).flat_map(ring))
        .map(|(dx, dy)| (hx + dx, hy + dy))
        .find(|&(x, y)| world.is_empty(x, y) && !bar.bars(world, (x, y)))
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
    // **On the brood pile when the caller found one** ([`pile_site`]), else
    // the first empty cell beside the head that the egg bar allows: an ant
    // laying where it stands in the shaft keeps its egg, as one laying with
    // every neighbour taken does.
    let (ex, ey) = match at {
        Some(cell) => cell,
        None => beside_head(world, head, EggBar::of(world, parent))?,
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
    // **Stood on, not gone**: a body walking through brood holds its cell out
    // of the grid (`creature::PushPast`). The brood goes on growing -- a stage
    // change is written into the walker's held copy (`set_stage`) -- and only
    // the two steps that write the world around it, hatching and starving,
    // wait for the walker to step off.
    let (x, y, holder) = match brood_cell(world, organism, material, (site.x, site.y)) {
        Some((x, y)) => (x, y, None),
        None => match creature::held_brood_at(world, organism, (site.x, site.y)) {
            Some((holder, (x, y))) => {
                world.creature_stats.brood_held += 1;
                (x, y, Some(holder))
            }
            None => {
                // **The cell is gone** -- burned, blasted, erased, buried by
                // a write. What the brood held goes with it.
                let bank = world.organism(organism).map_or(0.0, |s| s.energy);
                let colony = world.colony_of(organism);
                world.book(colony, Account::Dissipated, bank as f64);
                world.creature_stats.brood_lost += 1;
                world.free_brood(organism);
                return Vec::new();
            }
        },
    };
    // **Carried to brood** ([`carry`]) before its stage runs, so the stage
    // runs where it now lies. Off, one branch.
    let (x, y) = carry(world, organism, (x, y), material, &def, brood_carry_reach(), egg_door());
    // **Spread out of a crowded pile** ([`spread`]), likewise before the
    // stage runs. Brood a walker holds is not lying in its cell, so it stays.
    let (x, y) = if holder.is_none() { spread(world, organism, (x, y), material, &def, brood_spread(), egg_door()) } else { (x, y) };
    let frame = world.frame;
    let colony = world.colony_of(organism);
    let at = |next: u64| vec![ActiveSite { x, y, kind: ActiveKind::Creature { organism }, next_frame: next }];
    match b.stage {
        BroodStage::Egg => {
            if frame < b.since + block.egg_frames {
                return at(b.since + block.egg_frames);
            }
            set_stage(world, organism, (x, y), material, BroodStage::Larva, frame, holder);
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
                // The corpse is written where the larva lies, so not while a
                // walker stands there; upkeep goes on and is booked overdrawn
                // when it does starve.
                if holder.is_some() {
                    return at(world.creature_due(LARVA_TICK));
                }
                if away_from_door(world, x) {
                    world.creature_stats.larvae_starved_away += 1;
                }
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
                    if away_from_door(world, x) {
                        world.creature_stats.brood_ate_away_j += (after - bank) as f64;
                    }
                    creature::note_feed(world, organism, (x, y), creature::FEED_ATE, 0, after - bank);
                }
            }
            if nurse_env() {
                nurse(world, organism, (x, y), colony, &def, b.target);
            }
            if world.organism(organism).is_some_and(|s| s.energy >= b.target) {
                set_stage(world, organism, (x, y), material, BroodStage::Pupa, frame, holder);
                world.creature_stats.pupae += 1;
                if away_from_door(world, x) {
                    world.creature_stats.pupae_away += 1;
                }
                return at(world.creature_due(block.pupa_frames));
            }
            at(world.creature_due(LARVA_TICK))
        }
        BroodStage::Pupa => {
            if frame < b.since + block.pupa_frames {
                return at(b.since + block.pupa_frames);
            }
            // A callow walks out from under nobody: hatching clears the cell.
            if holder.is_some() {
                return at(world.creature_due(HATCH_RETRY));
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
    let mut need = target - world.organism(larva).map_or(target, |s| s.energy);
    if need <= 0.0 {
        return;
    }
    world.creature_stats.larva_ticks_hungry += 1;
    let away = away_from_door(world, x);
    if away {
        world.creature_stats.larva_ticks_hungry_away += 1;
    }
    // **Crop first** ([`crop_feed`]): food a carrier brought home costs no
    // nestmate's bank, so it goes in before anyone's savings do.
    let mut fed = 0.0;
    if crop_nurse_of(world) != CropNurse::Off {
        let gain = crop_feed(world, larva, (x, y), colony, def, gut, need);
        need -= gain;
        fed += gain;
        if need <= 0.0 {
            note_fed_away(world, away, fed);
            return;
        }
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
    let Some((donor, mine)) = best else {
        note_fed_away(world, away, fed);
        return;
    };
    world.creature_stats.larva_ticks_nursed += 1;
    let amount = (creature::SHARE_FRACTION * (mine - start_energy)).min(need);
    if amount <= 0.0 {
        note_fed_away(world, away, fed);
        return;
    }
    note_fed_away(world, away, fed + amount);
    creature::nurse_stays(world, donor);
    if let Some(s) = world.organism_mut(donor) {
        s.energy -= amount;
    }
    if let Some(s) = world.organism_mut(larva) {
        s.energy += amount;
    }
    world.creature_stats.brood_nursed_j += amount as f64;
    creature::note_feed(world, larva, (x, y), creature::FEED_BANK, donor, amount);
    let donor_colony = world.colony_of(donor);
    world.book(donor_colony, Account::SharedOut, amount as f64);
    world.book(colony, Account::SharedIn, amount as f64);
}

/// **How far off the founding door a larva has to lie to be away from the
/// door's lane**, in columns: the lane the fed ants stand in (94-98% of
/// them, 2026-10-05).
pub const DOOR_LANE: i32 = 3;

/// Whether column `x` is more than [`DOOR_LANE`] off the founding door.
fn away_from_door(world: &World, x: i32) -> bool {
    world.nest_sites.first().is_some_and(|s| (x - s.x).abs() > DOOR_LANE)
}

/// Count a hungry larva tick away from the door's lane that a nestmate fed.
fn note_fed_away(world: &mut World, away: bool, fed: f32) {
    if away && fed > 0.0 {
        world.creature_stats.larva_ticks_fed_away += 1;
        world.creature_stats.brood_fed_away_j += f64::from(fed);
    }
}

/// **Fed from a carrier's crop** (`PIXEL_PHYSICS_CROP_NURSE`, [`nurse`]):
/// the nestmate touching the larva with the most food in its crop
/// ([`creature::crop_to_feed`]) gives it what it still lacks, out of the
/// cell it is on and no further, and the larva is credited as if it had
/// eaten that food itself. Returns the energy the larva gained.
///
/// The donor must be kin by scent and over its own stamp: a hungry carrier
/// eats its own load first, as its own digestion does at full rate below
/// its stamp. Off under `PIXEL_PHYSICS_DIGEST=lump`, where `digesting` is
/// progress not yet paid for and a cell taken from would be paid twice.
///
/// **Booked as a meal, not a share**: the food was never anyone's energy,
/// so it goes in under the harvest account of what it was, through the
/// larva's own gut -- the same `quality` and overhead digestion would have
/// charged it -- and the carrier's crop gives up face value exactly as its
/// own chewing would (`digesting` advances, a finished cell leaves). The
/// live identity is untouched; standing meat falls by face and the harvest
/// by yield, digestion's one-directional slack.
#[allow(clippy::too_many_arguments)]
fn crop_feed(world: &mut World, larva: OrganismId, (x, y): (i32, i32), colony: u32, def: &CreatureDef, gut: creature::Gut, need: f32) -> f32 {
    if super::organism::digest_is_lumpy() {
        return 0.0;
    }
    let mut best: Option<(OrganismId, super::organism::Crop)> = None;
    for (dx, dy) in super::structural::NEIGHBOURS_8 {
        let id = world.get(x + dx, y + dy).organism_id();
        if id == 0 || id == larva || best.is_some_and(|(b, _)| b == id) {
            continue;
        }
        let Some(st) = world.organism(id) else { continue };
        if st.brood.is_some() || st.energy <= def.start_energy || !creature::is_living_kin_id(world, id, gut) {
            continue;
        }
        let Some(crop) = creature::crop_to_feed(world, st) else { continue };
        if best.is_none_or(|(_, b)| crop.worth() > b.worth()) {
            best = Some((id, crop));
        }
    }
    let Some((donor, c)) = best else { return 0.0 };
    let (quality, overhead) = creature::yield_of(world, larva, def, c.material);
    let keep = quality * (1.0 - overhead);
    let left_in_cell = (c.unit - c.digesting).max(0.0);
    if keep <= 0.0 || left_in_cell <= 0.0 {
        return 0.0;
    }
    let face = (need / keep).min(left_in_cell);
    let finished = face >= left_in_cell;
    let left = if finished { c.cells - 1 } else { c.cells };
    if let Some(s) = world.organism_mut(donor) {
        s.crop = (left > 0).then_some(super::organism::Crop { cells: left, digesting: if finished { 0.0 } else { c.digesting + face }, ..c });
    }
    let gain = face * keep;
    creature::nurse_stays(world, donor);
    if let Some(s) = world.organism_mut(larva) {
        s.energy += gain;
    }
    if world.materials.get(c.material).worth_in_aux {
        world.book_meal(colony, Account::HarvestedCorpse, c.material, gain as f64);
    } else {
        world.book_meal(colony, Account::HarvestedPlant, c.material, gain as f64);
    }
    world.creature_stats.digest_overhead_energy += (face * quality * overhead) as f64;
    world.creature_stats.larva_ticks_crop_fed += 1;
    world.creature_stats.brood_crop_fed_j += gain as f64;
    creature::note_feed(world, larva, (x, y), creature::FEED_CROP, donor, gain);
    gain
}

/// **Larvae fed from the food carriers bring home**
/// (`PIXEL_PHYSICS_CROP_NURSE`): `touch` **by default** (a larva touching a
/// carrier is fed from its crop, [`crop_feed`]), `off`, or `on` (that, and a
/// carrier inside the nest is drawn up [`larva_scent`] as [`nurse_seek`]
/// draws an empty nurse). [`World::crop_nurse`] for one world;
/// [`crop_nurse_of`] reads both. Unknown values panic.
///
/// **`touch` ships on because it measured neutral, and `on` stays off
/// because it leaned worse** (2026-10-04, main 99e0be4f, 200,000 frames,
/// live ants at the end, each seed paired with today's game):
///
/// | bed | today | `touch` | `on` |
/// |---|---|---|---|
/// | food box, seeds 1-8 | 230 354 128 94 572 523 529 17 | 465 417 500 404 67 428 128 0 | 588 48 565 105 75 263 139 465 |
/// | goal bed, mister on, seeds 1-6 | 384 345 253 71 593 38 | 121 45 0 282 0 383 | 0 63 148 447 226 447 |
/// | goal bed, mister off, seeds 1-6 | 45 334 0 4 0 75 | 0 388 206 474 0 114 | -- |
///
/// `touch` was better on 10 of the 20 seeds and worse on 9 (one tie, both
/// dead), and 6 of 20 colonies ended under 50 ants in each arm; `on` was
/// better on 6 of 14 and worse on 8. The swings are the boxes' own: the goal
/// colonies that died under `touch` were fed 3-10 kJ from crops in 200,000
/// frames.
///
/// **Why so little**: carriers are seldom beside the brood. At a hungry
/// larva's tick (food box, `touch`, seeds 1-3 to 100,000 frames) the
/// nestmate touching it had an empty crop 64-83% of the time, none touched
/// it 14-34%, one held crop food it could not give (a packed lunch, or a
/// pellet in its jaws) 2-5%, and a fed carrier stood there on 0.2-0.5% of
/// ticks. Under `on` crop food was 6% of what larvae ate on the food box;
/// food dropped beside them is most of it.
///
/// **Why**: [`nurse`] gives from a nestmate's bank, and an ant's bank is
/// also what it lays from, so it moved energy between eggs and larvae
/// without adding any ([`nurse_seek`]'s note). Food in a crop is the
/// colony's store (owner, 2026-10-04: crops are the food store, no larder
/// room), and more than half of all walking on the food box was done with
/// food in it. Scott, 2026-10-04: "so the issue is that nurse workers have
/// food in the crop and therefore it doesn't work".
///
/// **Biology.** Ants pass liquid food mouth to mouth out of the crop, the
/// "social stomach", between adults and from adults to larvae (LeBoeuf et
/// al. 2016, doi 10.7554/eLife.20375); in *Camponotus* the food a forager
/// brings home spreads from its crop through the colony this way (Greenwald
/// et al. 2018, doi 10.7554/eLife.31730).
pub fn crop_nurse() -> CropNurse {
    static V: std::sync::OnceLock<CropNurse> = std::sync::OnceLock::new();
    *V.get_or_init(|| parse_crop_nurse(&std::env::var("PIXEL_PHYSICS_CROP_NURSE").unwrap_or_default()))
}

/// How larvae get carriers' food ([`crop_nurse`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CropNurse {
    Off,
    /// A carrier touching a hungry larva feeds it from its crop.
    Touch,
    /// `Touch`, and carriers inside the nest follow larva scent.
    On,
}

fn parse_crop_nurse(raw: &str) -> CropNurse {
    match raw.trim() {
        "off" => CropNurse::Off,
        "" | "touch" => CropNurse::Touch,
        "on" => CropNurse::On,
        v => panic!("PIXEL_PHYSICS_CROP_NURSE={v:?}: use on, touch or off"),
    }
}

/// The crop nursing in force in `world`: [`World::crop_nurse`] if set, else
/// [`crop_nurse`].
pub fn crop_nurse_of(world: &World) -> CropNurse {
    world.crop_nurse.unwrap_or_else(crop_nurse)
}

/// **Nurses find hungry larvae by their scent** (`PIXEL_PHYSICS_NURSE_SEEK`):
/// **off by default**; `on` (or `workers`) lets fed nest workers seek at
/// [`NURSE_SEEK_GAIN`], `all` every fed ant, a number over 0 is the workers'
/// gain; [`World::nurse_seek`] for one world. Read by
/// `creature::chooser_step`, which adds the pull of [`larva_scent`] to every
/// heading an idle nurse scores.
///
/// **Off because it moves energy between laying and larvae without adding
/// any** (2026-10-04, main 0b3e264a, measured in the commit that added it).
/// The ant it acts on -- fed, jaws and crop empty, inside the nest -- is
/// rare: on the food box, of every walking decision, 54% carry food in the
/// crop, 14% a pellet, 6% are unfed and, under `on`, 23% are foragers, so
/// the pull is felt on 0.14% of decisions (`on`) or 1.2% (`all`). Under
/// `all` the givers are the ants saving to lay, and eggs laid by 80k fell
/// on 6 of 6 paired seeds. Under `on` it is nearly a placebo, and it did
/// not keep spread brood alive: with spreading, 4 of 9 food-box and 3 of 6
/// goal-box colonies died, against 1 of 9 and 1 of 6 with neither.
///
/// **Why.** A larva is fed by food lying beside it, by a nestmate touching it
/// ([`nurse`]) and by a brain's `Share`, and every one of those needs an
/// adult to be beside it already. Nothing brought one there, so larvae were
/// fed where the crowd happened to stand, and a larva carried out of the
/// crowd went hungry: spreading crowded brood ([`spread`]) cut mouth-to-mouth
/// food into larvae from 327k to 59k J on the food box, and births fell on 3
/// of 3 seeds (2026-10-04, main 0b3e264a).
///
/// **Biology.** Fire-ant nurses taste each larva briefly and feed it at a
/// rate set by its own hunger (Cassill & Tschinkel 1995, *Anim. Behav.*
/// 50:801-813). Starved honeybee larvae give off more of a volatile,
/// E-beta-ocimene, that draws workers to their cells (He et al. 2016, doi
/// 10.1038/srep22359), and ant brood odours draw workers, though whether they
/// are true pheromones is argued (Schultner & Pulliainen 2020, doi
/// 10.1007/s00040-019-00747-3). Not every ant: *Formica* workers were not
/// drawn to starved larvae's odour (Peignier et al. 2019, doi
/// 10.3389/fevo.2019.00398). So the scent is short-range and scaled by
/// hunger.
pub fn nurse_seek() -> Option<NurseSeek> {
    static V: std::sync::OnceLock<Option<NurseSeek>> = std::sync::OnceLock::new();
    *V.get_or_init(|| parse_nurse_seek(&std::env::var("PIXEL_PHYSICS_NURSE_SEEK").unwrap_or_default()))
}

/// Who seeks, and how hard ([`nurse_seek`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NurseSeek {
    /// The pull of a full scent.
    pub gain: f32,
    /// Only nest-bound workers seek (`on`, `workers`); `all` lets every fed
    /// ant seek.
    pub workers_only: bool,
}

fn parse_nurse_seek(raw: &str) -> Option<NurseSeek> {
    let workers = |gain| Some(NurseSeek { gain, workers_only: true });
    match raw.trim() {
        "on" | "workers" => workers(NURSE_SEEK_GAIN),
        "all" => Some(NurseSeek { gain: NURSE_SEEK_GAIN, workers_only: false }),
        "" | "off" => None,
        v => workers(v.parse().ok().filter(|g: &f32| g.is_finite() && *g > 0.0).unwrap_or_else(|| panic!("PIXEL_PHYSICS_NURSE_SEEK={v:?}: use on, workers, all, off or a gain over 0"))),
    }
}

/// The nurse seeking in force in `world`: [`World::nurse_seek`] if set, else
/// [`nurse_seek`].
pub fn nurse_seek_of(world: &World) -> Option<NurseSeek> {
    world.nurse_seek.unwrap_or_else(nurse_seek)
}

/// The pull of a full scent, the home pull's own size (`HOME_GAIN` 1).
pub const NURSE_SEEK_GAIN: f32 = 1.0;
/// How far a larva can be smelt, in cells either way.
pub const NURSE_SCENT_REACH: i32 = 6;
/// The summed scent at which the pull is half its gain: one starving larva
/// two cells off.
pub const NURSE_SCENT_HALF: f32 = 0.25;

/// **Which way the hungry larvae of `colony` lie from `(hx, hy)`, and how
/// strongly they smell**: a unit direction and a strength in 0..1, `None`
/// where none is in reach.
///
/// Every larva within [`NURSE_SCENT_REACH`] cells either way adds its hunger
/// -- the share of its pupation target it still lacks -- over its distance
/// squared, along the line to it, so a near or a hungrier larva pulls
/// harder, and larvae on opposite sides cancel. The strength saturates as
/// `s / (s + NURSE_SCENT_HALF)` of the summed vector's length. A larva a
/// walker holds is out of the grid and gives no scent while it is held.
pub(super) fn larva_scent(world: &World, (hx, hy): (i32, i32), colony: u32, material: super::material::MaterialId) -> Option<(f32, f32, f32)> {
    let (mut vx, mut vy) = (0.0f32, 0.0f32);
    for dy in -NURSE_SCENT_REACH..=NURSE_SCENT_REACH {
        for dx in -NURSE_SCENT_REACH..=NURSE_SCENT_REACH {
            if (dx, dy) == (0, 0) {
                continue;
            }
            let c = world.get(hx + dx, hy + dy);
            if c.material != material {
                continue;
            }
            let Some(st) = world.organism(c.organism_id()) else { continue };
            let Some(b) = st.brood.filter(|b| b.stage == BroodStage::Larva && b.target > 0.0) else { continue };
            if st.colony != colony {
                continue;
            }
            let need = ((b.target - st.energy) / b.target).clamp(0.0, 1.0);
            let d2 = (dx * dx + dy * dy) as f32;
            vx += need * dx as f32 / d2;
            vy += need * dy as f32 / d2;
        }
    }
    let len = (vx * vx + vy * vy).sqrt();
    (len > 0.0).then(|| (vx / len, vy / len, len / (len + NURSE_SCENT_HALF)))
}

/// The brood material an animal of `def`'s species lays, if it lays any.
pub(super) fn brood_material(world: &World, def: &CreatureDef) -> Option<super::material::MaterialId> {
    block_of(def).and_then(|b| world.materials.id_of(&b.material))
}

/// Move a brood organism to `stage`, and its cell to that stage's shade.
fn set_stage(world: &mut World, organism: OrganismId, (x, y): (i32, i32), material: super::material::MaterialId, stage: BroodStage, frame: u64, holder: Option<OrganismId>) {
    if let Some(st) = world.organism_mut(organism) {
        if let Some(b) = st.brood.as_mut() {
            b.stage = stage;
            b.since = frame;
            b.last_tick = frame;
        }
    }
    // **Held by a walker: restage the copy it will put back**, since the grid
    // cell is the walker's own (`creature::PushPast`).
    if let Some(holder) = holder {
        let held = world.organism_mut(holder).and_then(|s| s.parted.iter_mut().find(|h| (h.x, h.y) == (x, y) && h.cell.organism_id() == organism));
        if let Some(h) = held {
            h.cell = Cell::new(material, stage as u8).with_organism_id(organism).with_aux(h.cell.aux());
        }
        return;
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
        let site = creature::try_bud(&mut w, ant, &def, 0.0, 0.0).expect("a rich ant lays");
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
        w.feed_log = Some(Vec::new());
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
        // The feed trace logged every meal at what was booked: each bite of
        // the food in reach from no donor, and the parent's top-ups from its
        // bank as the parent's.
        let fed = w.feed_log.take().unwrap_or_default();
        assert!(
            fed.iter().all(|r| r.larva == egg && r.at == (ex, ey)),
            "a meal was logged for the wrong larva or cell: {fed:?}"
        );
        let logged = |kind: u8| {
            fed.iter()
                .filter(|r| r.kind == kind)
                .map(|r| r.gain as f64)
                .sum::<f64>()
        };
        assert!(
            fed.iter()
                .filter(|r| r.kind == creature::FEED_ATE)
                .all(|r| r.donor == 0),
            "a bite of food in reach was logged with a donor: {fed:?}"
        );
        assert!(
            fed.iter()
                .filter(|r| r.kind == creature::FEED_BANK)
                .all(|r| r.donor == ant),
            "a bank top-up was logged from someone other than the parent: {fed:?}"
        );
        assert!(
            (logged(creature::FEED_ATE) - w.creature_stats.brood_ate_j).abs() < 1e-2,
            "logged bites {} against booked {}",
            logged(creature::FEED_ATE),
            w.creature_stats.brood_ate_j
        );
        assert!(
            (logged(creature::FEED_BANK) - w.creature_stats.brood_nursed_j).abs() < 1e-2,
            "logged top-ups {} against booked {}",
            logged(creature::FEED_BANK),
            w.creature_stats.brood_nursed_j
        );

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
        let site = creature::try_bud(&mut w, ant, &def, 0.0, 0.0).expect("lays");
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
        let site = creature::try_bud(&mut w, ant, &def, 0.0, 0.0).expect("lays");
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
        let site = creature::try_bud(&mut w, ant, &def, 0.0, 0.0).expect("a rich ant buds");
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
        let site = creature::try_bud(&mut w, ant, &def, 0.0, 0.0).expect("lays");
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
        let site = creature::try_bud(&mut w, ant, &def, 0.0, 0.0).expect("lays");
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
        assert!(pile_site(&w, head, &def, &block, 0, EggBar::OFF).is_none(), "a reach of 0 must find nothing");
        let site = creature::try_bud(&mut w, ant, &def, 0.0, 0.0).expect("an ant four steps from home lays");
        assert_eq!(w.creature_stats.eggs_laid, 1);
        assert!(creature::home_at(&w, site.x, site.y, &def), "the egg landed off home at {:?}", (site.x, site.y));
        assert!((site.x - head.0).abs().max((site.y - head.1).abs()) <= EGG_PILE_REACH);
    }

    /// **Home out of reach holds the egg**, and says so in the counter the
    /// head-only rule used.
    #[test]
    fn home_out_of_reach_holds_the_egg() {
        let (mut w, ant, def, _) = nest_bed(112);
        assert!(creature::try_bud(&mut w, ant, &def, 0.0, 0.0).is_none(), "laid with home eleven cells away");
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
        assert!(pile_site(&w, head, &def, &block, EGG_PILE_REACH, EggBar::OFF).is_none(), "the walk went through stone");
        assert!(creature::try_bud(&mut w, ant, &def, 0.0, 0.0).is_none());
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
        let cell = pile_site(&w, head, &def, &block, EGG_PILE_REACH, EggBar::OFF).expect("home in reach");
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

    /// `PIXEL_PHYSICS_EGG_DOOR`'s value.
    #[test]
    fn egg_door_parses_its_modes() {
        assert_eq!(parse_egg_door(""), EggDoor::Off, "the default is laying anywhere at home");
        assert_eq!(parse_egg_door("off"), EggDoor::Off);
        assert_eq!(parse_egg_door("door"), EggDoor::Door);
        assert_eq!(parse_egg_door(" cut "), EggDoor::Cut);
        assert_eq!(parse_egg_door("deep"), EggDoor::Deep);
        assert!(std::panic::catch_unwind(|| parse_egg_door("shaft")).is_err(), "a mistyped mode must not fail open");
    }

    /// Soil from row 40 down, a founding cut at column 60 drawn by hand --
    /// shaft 60-61 on rows 40-45, chamber 56-65 on rows 46-47 -- and, with
    /// `gallery`, a passage the colony dug east from the chamber along row 47
    /// to column 80. Home is the dug nest. Returns the world and the ant's
    /// species def (one ant stands on the surface far to the west, only so
    /// the def is the live one).
    fn cut_bed(gallery: bool) -> (World, CreatureDef, crate::sim::world::ShaftFootprint) {
        let mut w = World::new(Rect::new(0, 0, 119, 99));
        let soil = w.materials.id_of("soil").expect("soil");
        for x in 0..=119 {
            for y in 40..=99 {
                let stone = y >= 92 || x == 0 || x == 119;
                w.set(x, y, if stone { Cell::new(material::STONE, 0) } else { Cell::new(soil, 0) }.with_attached(true));
            }
        }
        w.register_nest_site(60, 39, 2);
        let cut = crate::sim::world::ShaftFootprint { x0: 60, x1: 61, top: 40, bottom: 45, mouth_bottom: 41, chamber_x0: 56, chamber_x1: 65, chamber_top: 46, chamber_bottom: 47, side: None };
        w.nest_sites[0].shaft = Some(cut);
        let mut open: Vec<(i32, i32)> = (40..=45).flat_map(|y| [(60, y), (61, y)]).chain((46..=47).flat_map(|y| (56..=65).map(move |x| (x, y)))).collect();
        if gallery {
            open.extend((66..=80).map(|x| (x, 47)));
        }
        for (x, y) in open {
            w.set(x, y, Cell::EMPTY);
        }
        w.nest_home = Some(creature::NestHome::Dug);
        w.step_nest_dug();
        w.plant_ant(10, 38);
        let ant = w.get(10, 38).organism_id();
        assert_ne!(ant, 0, "test setup: the ant was not placed");
        let species = w.organism(ant).expect("live").species;
        let def = w.species.get(species).creature.clone().expect("ant is a creature");
        (w, def, cut)
    }

    /// **An egg is handed past the doorway, never put down in it**
    /// ([`EggBar`]). Layers standing in every open cell of the cut: with
    /// `door`, no egg lands in the way in, and a layer in the chamber still
    /// lays in it (the chamber is home); with `cut`, no egg lands anywhere
    /// in the cut, and a layer near the gallery hands its egg out to it. The
    /// `off` arm is the positive control: the same layers put eggs in the
    /// door, so the scene can tell the rule from its absence. `deep` is
    /// covered by `deep_lays_in_the_cut_only_while_nothing_dug_is_in_reach`.
    #[test]
    fn an_egg_is_handed_past_the_doorway_never_put_down_in_it() {
        let (w, def, cut) = cut_bed(true);
        let block = def.brood.clone().expect("brood");
        let clear = creature::DOOR_CLEAR_CELLS as i32;
        let door = EggBar { mode: EggDoor::Door, clear };
        let whole = EggBar { mode: EggDoor::Cut, clear };
        let heads: Vec<(i32, i32)> = (40..=47).flat_map(|y| (56..=65).map(move |x| (x, y))).filter(|&(x, y)| cut.contains(x, y)).collect();
        let (mut in_door_off, mut in_chamber_door, mut to_gallery) = (0, 0, 0);
        for &head in &heads {
            let off = pile_site(&w, head, &def, &block, EGG_PILE_REACH, EggBar::OFF);
            in_door_off += usize::from(off.is_some_and(|p| creature::in_doorway(&w, p, clear)));
            let d = pile_site(&w, head, &def, &block, EGG_PILE_REACH, door);
            assert!(!d.is_some_and(|p| creature::in_doorway(&w, p, clear)), "door: a layer at {head:?} put its egg in the way in at {d:?}");
            in_chamber_door += usize::from(d.is_some_and(|(x, y)| cut.in_chamber(x, y)));
            let c = pile_site(&w, head, &def, &block, EGG_PILE_REACH, whole);
            assert!(!c.is_some_and(|(x, y)| cut.contains(x, y) || creature::in_doorway(&w, (x, y), clear)), "cut: a layer at {head:?} put its egg in the cut at {c:?}");
            to_gallery += usize::from(c.is_some_and(|(x, y)| y == 47 && x > cut.chamber_x1));
            // Beside the head, as a layer laying anywhere puts it: the same bar.
            let b = beside_head(&w, head, door);
            assert!(!b.is_some_and(|p| creature::in_doorway(&w, p, clear)), "door: beside the head at {head:?} took {b:?}");
        }
        assert!(in_door_off > 5, "control: with the bar off only {in_door_off} layers put an egg in the door -- the scene does not test the rule");
        assert!(in_chamber_door > 0, "door: no layer in the chamber could lay in it");
        assert!(to_gallery > 0, "cut: no layer handed its egg out to the gallery");
    }

    /// **With the whole cut barred and nothing dug beyond it, the egg is
    /// held**: the colony lays at home only once it has dug somewhere to lay.
    #[test]
    fn with_nothing_dug_beyond_the_cut_the_egg_is_held() {
        let (w, def, cut) = cut_bed(false);
        let block = def.brood.clone().expect("brood");
        let whole = EggBar { mode: EggDoor::Cut, clear: creature::DOOR_CLEAR_CELLS as i32 };
        let head = (cut.chamber_x1, cut.chamber_bottom);
        assert!(pile_site(&w, head, &def, &block, EGG_PILE_REACH, EggBar::OFF).is_some(), "test setup: the chamber is not home");
        assert_eq!(pile_site(&w, head, &def, &block, EGG_PILE_REACH, whole), None);
    }

    /// **`deep` lays in the cut only while nothing dug is in reach**: with
    /// no gallery the egg goes in the chamber (where `cut` holds it), out of
    /// the doorway; with the gallery dug, the same layer hands it out to the
    /// gallery even though the chamber is nearer. Every layer in the cut
    /// that `cut` sends to the gallery, `deep` sends there too.
    #[test]
    fn deep_lays_in_the_cut_only_while_nothing_dug_is_in_reach() {
        let clear = creature::DOOR_CLEAR_CELLS as i32;
        let deep = EggBar { mode: EggDoor::Deep, clear };
        let whole = EggBar { mode: EggDoor::Cut, clear };
        let (w, def, cut) = cut_bed(false);
        let block = def.brood.clone().expect("brood");
        let head = (cut.chamber_x1, cut.chamber_bottom);
        let site = pile_site(&w, head, &def, &block, EGG_PILE_REACH, deep).expect("deep: held an egg with the chamber free");
        assert!(cut.in_chamber(site.0, site.1) && !creature::in_doorway(&w, site, clear), "deep: laid at {site:?}, not the chamber floor");
        let (w, def, cut) = cut_bed(true);
        let site = pile_site(&w, head, &def, &block, EGG_PILE_REACH, deep).expect("deep: no site with the gallery dug");
        assert!(!cut.contains(site.0, site.1), "deep: laid in the cut at {site:?} with the gallery in reach");
        for head in (40..=47).flat_map(|y| (56..=65).map(move |x| (x, y))).filter(|&(x, y)| cut.contains(x, y)) {
            let c = pile_site(&w, head, &def, &block, EGG_PILE_REACH, whole);
            let d = pile_site(&w, head, &def, &block, EGG_PILE_REACH, deep);
            assert!(!d.is_some_and(|p| creature::in_doorway(&w, p, clear)), "deep: a layer at {head:?} put its egg in the way in at {d:?}");
            if c.is_some() {
                assert!(!d.is_some_and(|(x, y)| cut.contains(x, y)), "deep: a layer at {head:?} laid in the cut at {d:?}, cut found {c:?}");
            }
        }
    }

    /// `PIXEL_PHYSICS_BROOD_CARRY`'s value.
    #[test]
    fn brood_carry_parses_off_on_and_a_reach() {
        assert_eq!(parse_brood_carry(""), 0);
        assert_eq!(parse_brood_carry("off"), 0);
        assert_eq!(parse_brood_carry("on"), BROOD_CARRY_REACH);
        assert_eq!(parse_brood_carry("5"), 5);
        assert!(std::panic::catch_unwind(|| parse_brood_carry("yes")).is_err(), "a mistyped switch must not fail open");
    }

    /// Lay one egg from `parent` onto `cell`, as the brood pile would.
    fn lay_at(w: &mut World, parent: OrganismId, def: &CreatureDef, cell: (i32, i32)) -> OrganismId {
        let block = def.brood.clone().expect("brood");
        let st = w.organism(parent).expect("parent");
        let egg = Egg { species: st.species, genome: st.genome.clone(), traits: st.traits, generation: st.generation + 1, lineage: st.lineage, colony: st.colony, made: 0.0, fates: st.fates };
        let head = st.chain[0];
        let site = lay_egg(w, parent, head, def, &block, egg, Some(cell)).expect("laid");
        assert_eq!((site.x, site.y), cell);
        w.get(cell.0, cell.1).organism_id()
    }

    #[test]
    fn nurse_seek_parses_on_off_and_a_gain() {
        let workers = Some(NurseSeek { gain: NURSE_SEEK_GAIN, workers_only: true });
        assert_eq!(parse_nurse_seek(""), None);
        assert_eq!(parse_nurse_seek("on"), workers);
        assert_eq!(parse_nurse_seek("workers"), workers);
        assert_eq!(parse_nurse_seek("all"), Some(NurseSeek { gain: NURSE_SEEK_GAIN, workers_only: false }));
        assert_eq!(parse_nurse_seek("off"), None);
        assert_eq!(parse_nurse_seek("2.5"), Some(NurseSeek { gain: 2.5, workers_only: true }));
        assert!(std::panic::catch_unwind(|| parse_nurse_seek("0")).is_err(), "a gain of 0 must be spelt off");
        assert!(std::panic::catch_unwind(|| parse_nurse_seek("of")).is_err(), "a mistyped switch must not fail open");
    }

    #[test]
    fn crop_nurse_parses_on_touch_and_off() {
        assert_eq!(parse_crop_nurse(""), CropNurse::Touch, "touch is the default");
        assert_eq!(parse_crop_nurse("off"), CropNurse::Off);
        assert_eq!(parse_crop_nurse("touch"), CropNurse::Touch);
        assert_eq!(parse_crop_nurse("on"), CropNurse::On);
        assert!(std::panic::catch_unwind(|| parse_crop_nurse("of")).is_err(), "a mistyped switch must not fail open");
    }

    /// **A carrier touching a hungry larva feeds it from its crop**
    /// ([`crop_feed`]), crop first and bank second, with the live identity
    /// closed: the crop gives up face value, the larva gains the yield, and
    /// what the larva gained is exactly what the crop and the carrier's bank
    /// were booked as giving. Off, or with a carrier at its own stamp (a
    /// hungry carrier eats its own load), the crop is not touched.
    #[test]
    fn a_carrier_feeds_a_touching_larva_from_its_crop_with_the_books_closed() {
        // A 960 J fruit cell is used up whole (the larva needs more than it
        // yields); a 9,600 J one is only part-chewed, through `digesting`.
        for (mode, hungry, unit) in [(CropNurse::Off, false, 960.0), (CropNurse::Touch, false, 960.0), (CropNurse::Touch, false, 9_600.0), (CropNurse::Touch, true, 960.0)] {
            let (mut w, ant, def) = bed(true);
            w.crop_nurse = Some(mode);
            let block = def.brood.clone().expect("brood");
            let site = creature::try_bud(&mut w, ant, &def, 0.0, 0.0).expect("lays");
            let larva = the_egg(&w);
            w.frame = block.egg_frames;
            let sites = brood_tick(&mut w, &site);
            assert_eq!(w.organism(larva).and_then(|s| s.brood).map(|b| b.stage), Some(BroodStage::Larva));
            let fruit = w.materials.id_of("fruit").expect("fruit");
            let crop = crate::sim::organism::Crop { material: fruit, cells: 2, digesting: 0.0, unit, shade: 0, passenger: None };
            if hungry {
                w.organism_mut(ant).expect("live").energy = def.start_energy;
            }
            w.organism_mut(ant).expect("live").crop = Some(crop);
            let g0 = gap(&w);
            let (bank0, larva0) = (w.organism(ant).expect("live").energy, w.organism(larva).expect("larva").energy);
            w.frame += LARVA_TICK;
            // The feed trace (`World::feed_log`), on for this tick only.
            w.feed_log = Some(Vec::new());
            brood_tick(&mut w, &sites[0]);
            let fed = w.feed_log.take().unwrap_or_default();
            let held = w.organism(ant).expect("live").crop;
            let gained = w.organism(larva).expect("larva").energy - larva0;
            let from_bank = bank0 - w.organism(ant).expect("live").energy;
            assert!((gap(&w) - g0).abs() < 1e-2, "{mode:?}: feeding moved the live identity by {}", gap(&w) - g0);
            if mode == CropNurse::Off || hungry {
                assert_eq!(held, Some(crop), "{mode:?} hungry {hungry}: the crop was fed from");
                assert_eq!(w.creature_stats.brood_crop_fed_j, 0.0);
                assert!(
                    !fed.iter().any(|r| r.kind == creature::FEED_CROP),
                    "{mode:?} hungry {hungry}: the feed trace logged a crop meal that never happened"
                );
                continue;
            }
            // **The feed trace logs each meal as booked**: the crop meal and
            // the bank's top-up, from this carrier to this larva on its own
            // cell, at what the books credited. Watched red with
            // `creature::note_feed` returning before it logs.
            let logged = |kind: u8| {
                fed.iter()
                    .filter(|r| r.kind == kind)
                    .map(|r| r.gain as f64)
                    .sum::<f64>()
            };
            assert!(
                fed.iter().any(|r| r.kind == creature::FEED_CROP),
                "unit {unit}: the feed trace logged no crop meal"
            );
            assert!(
                fed.iter()
                    .all(|r| r.larva == larva && r.donor == ant && r.at == (site.x, site.y) && r.frame == w.frame),
                "unit {unit}: a logged meal names the wrong larva, donor, cell or frame: {fed:?}"
            );
            assert!(
                (logged(creature::FEED_CROP) - w.creature_stats.brood_crop_fed_j).abs() < 1e-2,
                "unit {unit}: logged crop {} against booked {}",
                logged(creature::FEED_CROP),
                w.creature_stats.brood_crop_fed_j
            );
            assert!(
                (logged(creature::FEED_BANK) - w.creature_stats.brood_nursed_j).abs() < 1e-2,
                "unit {unit}: logged bank {} against booked {}",
                logged(creature::FEED_BANK),
                w.creature_stats.brood_nursed_j
            );
            let given = crop.worth() - held.map_or(0.0, |c| c.worth());
            assert!(given > 0.0, "a fed carrier touching a hungry larva gave nothing from its crop");
            assert!(given <= crop.unit + 1e-3, "more than the cell in progress left the crop in one tick: {given}");
            let st = &w.creature_stats;
            assert!(gained > 0.0 && (gained as f64 - st.brood_crop_fed_j - st.brood_nursed_j + st.brood_upkeep_j).abs() < 1e-2, "the larva gained {gained}, booked crop {} + bank {} - upkeep {}", st.brood_crop_fed_j, st.brood_nursed_j, st.brood_upkeep_j);
            assert!((from_bank as f64 - w.creature_stats.brood_nursed_j).abs() < 1e-2, "the bank gave {from_bank}, booked {}", w.creature_stats.brood_nursed_j);
            let (quality, overhead) = creature::yield_of(&w, larva, &def, fruit);
            let keep = (quality * (1.0 - overhead)) as f64;
            assert!(keep < 1.0 && (w.creature_stats.brood_crop_fed_j - given as f64 * keep).abs() < 1e-2, "the larva was credited {} for {given} face, not the gut's {keep} of it", w.creature_stats.brood_crop_fed_j);
            assert_eq!(held.map(|c| c.cells), Some(if unit < 1_000.0 { 1 } else { 2 }), "unit {unit}: the wrong number of cells left the crop");
        }
    }

    /// **The feed trace changes nothing it logs** (`World::feed_log`): one
    /// larva fed every way this bed can feed it -- food beside it, its
    /// parent's crop and its parent's bank -- ends in the same state to the
    /// bit with the log on and off, and the log is not empty. (The colony
    /// bed of `creature`'s `the_decision_trace_changes_nothing_it_watches`
    /// raises no larva in its 9,000 frames, so it cannot carry this.)
    /// **Watched red** with `creature::note_feed` taking a joule off the
    /// larva it logs.
    #[test]
    fn the_feed_trace_changes_nothing_it_logs() {
        let run = |log: bool| {
            let (mut w, ant, def) = bed(true);
            w.crop_nurse = Some(CropNurse::Touch);
            if log {
                w.feed_log = Some(Vec::new());
            }
            let block = def.brood.clone().expect("brood");
            let site = creature::try_bud(&mut w, ant, &def, 0.0, 0.0).expect("lays");
            let larva = the_egg(&w);
            w.frame = block.egg_frames;
            let mut site = brood_tick(&mut w, &site)[0];
            let (ex, ey) = (site.x, site.y);
            let fruit = w.materials.id_of("fruit").expect("fruit");
            w.organism_mut(ant).expect("live").crop = Some(crate::sim::organism::Crop {
                material: fruit,
                cells: 2,
                digesting: 0.0,
                unit: 960.0,
                shade: 0,
                passenger: None,
            });
            for _ in 0..4 {
                for (dx, dy) in [(0, -1), (1, -1), (-1, -1)] {
                    if w.is_empty(ex + dx, ey + dy) {
                        w.set(ex + dx, ey + dy, Cell::new(fruit, 0));
                    }
                }
                w.frame += LARVA_TICK;
                site = brood_tick(&mut w, &site)[0];
            }
            let kinds: std::collections::BTreeSet<u8> =
                w.feed_log.take().unwrap_or_default().iter().map(|r| r.kind).collect();
            let st = &w.creature_stats;
            let state = (
                w.organism(larva)
                    .map(|s| (s.energy.to_bits(), s.brood.map(|b| b.stage as u8))),
                w.organism(ant)
                    .map(|s| (s.energy.to_bits(), s.crop.map(|c| (c.cells, c.digesting.to_bits())))),
                [
                    st.brood_ate_j,
                    st.brood_crop_fed_j,
                    st.brood_nursed_j,
                    st.brood_upkeep_j,
                ]
                .map(f64::to_bits),
                [(0, -1), (1, -1), (-1, -1)].map(|(dx, dy)| w.get(ex + dx, ey + dy)),
            );
            (state, kinds)
        };
        let ((off, off_kinds), (on, on_kinds)) = (run(false), run(true));
        assert!(off_kinds.is_empty(), "the untraced run logged meals: {off_kinds:?}");
        assert!(
            [creature::FEED_ATE, creature::FEED_CROP, creature::FEED_BANK]
                .iter()
                .all(|k| on_kinds.contains(k)),
            "the traced run logged only {on_kinds:?}: the scene did not feed the larva every way it is meant to"
        );
        assert_eq!(
            off, on,
            "turning the feed trace on changed the larva, its parent or the books"
        );
    }

    /// **A nest worker that feeds a larva stays one longer** ([`creature::
    /// NurseStay`]'s `stay`, through [`creature::nurse_stays`]), whether it
    /// fed from its crop ([`crop_feed`]) or its bank ([`nurse`]): its
    /// `nest_bound_until` moves out to the feeding frame plus the stay. With
    /// the stay at 0 it is left alone (the control), and a forager that feeds
    /// is not made a nest worker by it. Each arm checks the larva was fed, and
    /// fed one way only, so a feeding that never happened cannot pass for a
    /// stay withheld. Watched red with either call left out.
    #[test]
    fn under_nurse_stay_a_nest_worker_that_feeds_a_larva_stays_one_longer() {
        const STAY: u64 = 700;
        for (from_crop, stay, bound) in [
            (true, STAY, true),
            (false, STAY, true),
            (true, 0, true),
            (false, 0, true),
            (true, STAY, false),
            (false, STAY, false),
        ] {
            let (mut w, ant, def) = bed(true);
            w.crop_nurse = Some(CropNurse::Touch);
            w.nurse_stay = Some(creature::NurseStay {
                stay,
                ..creature::NurseStay::OFF
            });
            let block = def.brood.clone().expect("brood");
            let site = creature::try_bud(&mut w, ant, &def, 0.0, 0.0).expect("lays");
            let larva = the_egg(&w);
            w.frame = block.egg_frames;
            let sites = brood_tick(&mut w, &site);
            if from_crop {
                let fruit = w.materials.id_of("fruit").expect("fruit");
                // A 9,600 J cell meets the larva's whole need, so the bank is
                // never reached and the crop's own call is what is tested.
                w.organism_mut(ant).expect("live").crop = Some(crate::sim::organism::Crop {
                    material: fruit,
                    cells: 2,
                    digesting: 0.0,
                    unit: 9_600.0,
                    shade: 0,
                    passenger: None,
                });
            }
            w.frame += LARVA_TICK;
            let until0 = if bound { w.frame + 10 } else { 0 };
            w.organism_mut(ant).expect("live").nest_bound_until = until0;
            let larva0 = w.organism(larva).expect("larva").energy;
            let (crop0, bank0) = (w.creature_stats.brood_crop_fed_j, w.creature_stats.brood_nursed_j);
            brood_tick(&mut w, &sites[0]);
            let (by_crop, by_bank) = (
                w.creature_stats.brood_crop_fed_j - crop0,
                w.creature_stats.brood_nursed_j - bank0,
            );
            let (fed, other) = if from_crop {
                (by_crop, by_bank)
            } else {
                (by_bank, by_crop)
            };
            assert_eq!(
                other, 0.0,
                "test setup: crop {from_crop}: the larva was fed the other way too, so this arm tests both calls"
            );
            assert!(
                fed > 0.0 && w.organism(larva).expect("larva").energy > larva0,
                "test setup: crop {from_crop} stay {stay} bound {bound}: the ant did not feed the larva"
            );
            let want = if bound && stay > 0 { w.frame + stay } else { until0 };
            assert_eq!(
                w.organism(ant).expect("live").nest_bound_until,
                want,
                "crop {from_crop} stay {stay} bound {bound}: the feeder's stay at home is wrong"
            );
        }
    }

    /// **The scent points at a hungry larva of the ant's own colony**
    /// ([`larva_scent`]): a starving larva three cells east pulls due east; a
    /// fed one (at its target) and one of another colony give no scent; two
    /// larvae equally hungry and equally far on opposite sides cancel; and
    /// out of [`NURSE_SCENT_REACH`] nothing is smelt.
    #[test]
    fn larva_scent_points_at_a_hungry_larva_of_its_own_colony() {
        let (mut w, ant, def) = bed(true);
        let material = brood_material(&w, &def).expect("brood material");
        let colony = w.organism(ant).expect("live").colony;
        let head = w.organism(ant).expect("live").chain[0];
        let larva_at = |w: &mut World, cell: (i32, i32), fed: bool, colony: u32| {
            let id = lay_at(w, ant, &def, cell);
            let st = w.organism_mut(id).expect("laid");
            let b = st.brood.as_mut().expect("brood");
            b.stage = BroodStage::Larva;
            st.energy = if fed { b.target } else { 0.1 * b.target };
            st.colony = colony;
            id
        };
        let from = (head.0 - 10, head.1);
        assert_eq!(larva_scent(&w, from, colony, material), None, "no brood, yet a scent");
        let east = larva_at(&mut w, (from.0 + 3, from.1), false, colony);
        let (ux, uy, f) = larva_scent(&w, from, colony, material).expect("a starving larva three cells off gave no scent");
        assert!(ux > 0.99 && uy.abs() < 1e-3, "the scent points ({ux}, {uy}), not at the larva due east");
        assert!(f > 0.0 && f < 1.0, "strength {f} is not in 0..1");
        assert_eq!(larva_scent(&w, (from.0 - NURSE_SCENT_REACH + 2, from.1), colony, material), None, "a larva out of reach was smelt");
        // Fed, it is no longer smelt.
        let target = w.organism(east).and_then(|s| s.brood).expect("brood").target;
        w.organism_mut(east).expect("live").energy = target;
        assert_eq!(larva_scent(&w, from, colony, material), None, "a fed larva still gave a scent");
        // Another colony's larva is not this ant's to smell.
        larva_at(&mut w, (from.0 - 2, from.1 - 1), false, colony + 1);
        assert_eq!(larva_scent(&w, from, colony, material), None, "another colony's larva was smelt");
        // Two starving larvae of its own, mirror images: they cancel.
        w.organism_mut(east).expect("live").energy = 0.1 * target;
        larva_at(&mut w, (from.0 - 3, from.1), false, colony);
        assert_eq!(larva_scent(&w, from, colony, material), None, "two equal pulls on opposite sides did not cancel");
    }

    /// **A lone larva is carried to the pile** ([`carry`]): with a nestmate
    /// beside it and brood lying three cells off, it ends next to that
    /// brood; a second carry from there moves nothing (it is no better
    /// placed anywhere in reach); and with carrying off, or with nobody
    /// beside it, it stays where it was laid.
    #[test]
    fn a_lone_larva_is_carried_to_the_pile_and_stays_there() {
        let (mut w, ant, def) = bed(true);
        let block = def.brood.clone().expect("brood");
        let material = w.materials.id_of(&block.material).expect("brood material");
        let head = w.organism(ant).expect("live").chain[0];
        let start = (head.0 + 1, head.1);
        assert!(w.is_empty(start.0, start.1), "test setup: the cell beside the head is taken");
        let egg = lay_at(&mut w, ant, &def, start);
        // The pile: two brood cells on the floor, three and four cells on.
        for dx in [4, 5] {
            w.set(start.0 + dx - 1, start.1, Cell::new(material, 0));
        }
        let touching = |w: &World, (px, py): (i32, i32)| creature::DIRS.iter().filter(|&&(dx, dy)| w.get(px + dx, py + dy).material == material).count();
        assert_eq!(touching(&w, start), 0, "test setup: the egg already touches the pile");
        assert_eq!(carry(&mut w, egg, start, material, &def, 0, EggDoor::Off), start, "a reach of 0 moved it");
        let moved = carry(&mut w, egg, start, material, &def, BROOD_CARRY_REACH, EggDoor::Off);
        assert_ne!(moved, start, "a nestmate beside a lone egg did not carry it to the pile");
        assert_eq!(w.get(moved.0, moved.1).organism_id(), egg, "the egg's cell did not move with it");
        assert!(w.is_empty(start.0, start.1), "the egg was copied, not moved");
        assert!(touching(&w, moved) > 0, "the egg landed at {moved:?}, touching no brood");
        assert_eq!(w.creature_stats.brood_carried, 1);
        // Settled: nowhere in reach is strictly better, so it stays.
        assert_eq!(carry(&mut w, egg, moved, material, &def, BROOD_CARRY_REACH, EggDoor::Off), moved, "a settled egg was carried again");
        assert_eq!(w.creature_stats.brood_carried, 1);
        // Nobody beside it: the ant leaves, and a lone egg laid far off stays.
        let far = (start.0 - 30, start.1);
        let lone = lay_at(&mut w, ant, &def, far);
        assert_eq!(carry(&mut w, lone, far, material, &def, BROOD_CARRY_REACH, EggDoor::Off), far, "an egg with no nestmate beside it moved");
    }

    /// **Brood in the doorway is carried out of it** ([`carry`] under the
    /// egg bar): an egg at the foot of the shaft, a nestmate standing under
    /// it in the chamber, is carried onto the chamber's floor -- out of the
    /// way in, still at home; under `cut`, out of the founding cut to the
    /// gallery, and held where it is when no such home is in reach; under
    /// `deep`, onto the chamber floor at a short reach, and on to the
    /// gallery once it is in reach. With the bar off it stays, which is the
    /// positive control: nothing else in the scene would move it.
    #[test]
    fn brood_in_the_doorway_is_carried_out_of_it() {
        for (door, should_move) in [(EggDoor::Off, false), (EggDoor::Door, true), (EggDoor::Cut, true), (EggDoor::Deep, true)] {
            let (mut w, def, cut) = cut_bed(true);
            let block = def.brood.clone().expect("brood");
            let material = w.materials.id_of(&block.material).expect("brood material");
            w.plant_ant(61, 46);
            let ant = w.get(61, 46).organism_id();
            assert_ne!(ant, 0, "test setup: no ant in the chamber");
            let start = (61, 45);
            assert!(w.is_empty(start.0, start.1), "test setup: the shaft's foot is taken");
            let egg = lay_at(&mut w, ant, &def, start);
            let clear = creature::DOOR_CLEAR_CELLS as i32;
            assert!(creature::in_doorway(&w, start, clear), "test setup: the egg is not in the doorway");
            // Under `cut` the nearest home it may lie in is the gallery, five
            // steps off, so this arm carries further.
            let reach = if door == EggDoor::Cut { 6 } else { BROOD_CARRY_REACH };
            if door == EggDoor::Cut {
                assert_eq!(carry(&mut w, egg, start, material, &def, BROOD_CARRY_REACH, door), start, "cut: carried with no allowed home in reach");
            }
            let moved = carry(&mut w, egg, start, material, &def, reach, door);
            if door == EggDoor::Deep {
                // The gallery is out of a short carry, so out of the doorway
                // onto the chamber floor, the only home in reach.
                assert!(cut.in_chamber(moved.0, moved.1), "deep: carried to {moved:?}, not the chamber");
                // An egg at the chamber's east end, a nestmate over it: the
                // gallery is now in reach, and outside the cut wins.
                let (mut w, def, cut) = cut_bed(true);
                w.plant_ant(65, 46);
                let ant = w.get(65, 46).organism_id();
                assert_ne!(ant, 0, "test setup: no ant at the chamber's end");
                let at = (64, 47);
                let egg = lay_at(&mut w, ant, &def, at);
                assert!(cut.in_chamber(at.0, at.1), "test setup: the egg is not in the chamber");
                let next = carry(&mut w, egg, at, material, &def, BROOD_CARRY_REACH, door);
                assert!(!cut.contains(next.0, next.1) && next.1 == 47, "deep: from {at:?} carried to {next:?}, not out to the gallery");
                assert_eq!(carry(&mut w, egg, next, material, &def, BROOD_CARRY_REACH, door), next, "deep: carried again once out of the cut");
                // The same egg under `door` stays: the chamber is home and
                // not barred, and nothing in reach touches more brood.
                let (mut w, def, _) = cut_bed(true);
                w.plant_ant(65, 46);
                let ant = w.get(65, 46).organism_id();
                let egg = lay_at(&mut w, ant, &def, at);
                assert_eq!(carry(&mut w, egg, at, material, &def, BROOD_CARRY_REACH, EggDoor::Door), at, "door: control moved the egg");
            }
            if !should_move {
                assert_eq!(moved, start, "{door:?}: nothing should move an egg that touches no brood");
                continue;
            }
            assert!(!creature::in_doorway(&w, moved, clear), "{door:?}: carried to {moved:?}, still in the doorway");
            assert!(creature::home_at(&w, moved.0, moved.1, &def), "{door:?}: carried out of home to {moved:?}");
            if door == EggDoor::Cut {
                assert!(!cut.contains(moved.0, moved.1), "cut: carried to {moved:?}, still in the founding cut");
                assert!(moved.1 == 47 && moved.0 > cut.chamber_x1, "cut: carried to {moved:?}, not the gallery");
            }
        }
    }

    /// `PIXEL_PHYSICS_BROOD_SPREAD`'s value.
    #[test]
    fn brood_spread_parses_on_off_and_a_crowd() {
        assert_eq!(parse_brood_spread(""), None);
        assert_eq!(parse_brood_spread("on"), Some(BROOD_SPREAD_CROWD));
        assert_eq!(parse_brood_spread("off"), None);
        assert_eq!(parse_brood_spread("8"), Some(8));
        assert!(std::panic::catch_unwind(|| parse_brood_spread("2")).is_err(), "a crowd under 3 must not pass");
        assert!(std::panic::catch_unwind(|| parse_brood_spread("yes")).is_err(), "a mistyped switch must not fail open");
    }

    /// **A crowded pile is spread** ([`spread`]): an egg in the middle of a
    /// pile on the chamber floor, a nestmate touching it, is carried at least
    /// [`BROOD_SPREAD_MIN`] steps to a home cell with few brood round it, and
    /// stays there (it is no longer crowded). One brood fewer round it and
    /// it is not crowded, so it stays -- the threshold is the crowd, not
    /// something else in the scene. Off, it stays. And it is never put down
    /// within [`BROOD_SPREAD_FOOD`] cells of loose food.
    #[test]
    fn a_crowded_pile_is_spread_and_never_onto_food() {
        let scene = |food: bool| {
            let (mut w, def, cut) = cut_bed(true);
            let block = def.brood.clone().expect("brood");
            let material = w.materials.id_of(&block.material).expect("brood material");
            w.plant_ant(61, 46);
            let ant = w.get(61, 46).organism_id();
            assert_ne!(ant, 0, "test setup: no ant in the chamber");
            let start = (60, 47);
            let egg = lay_at(&mut w, ant, &def, start);
            // The pile: the chamber floor either side of the egg, two deep
            // to the west.
            for (x, y) in (56..=63).map(|x| (x, 47)).chain((56..=59).map(|x| (x, 46))) {
                if w.is_empty(x, y) {
                    w.set(x, y, Cell::new(material, 0));
                }
            }
            if food {
                let provisions = w.materials.id_of("provisions").expect("provisions");
                w.set(67, 48, Cell::new(provisions, 0));
            }
            (w, def, cut, material, egg, start)
        };
        let round = |w: &World, material, (px, py): (i32, i32)| (-2..=2).flat_map(|dy| (-2..=2).map(move |dx| (dx, dy))).filter(|&(dx, dy)| (dx, dy) != (0, 0) && w.get(px + dx, py + dy).material == material).count() as i32;
        let (mut w, def, _, material, egg, start) = scene(false);
        let crowd = round(&w, material, start);
        assert!(crowd >= 3, "test setup: only {crowd} brood round the egg");
        assert_eq!(spread(&mut w, egg, start, material, &def, None, EggDoor::Off), start, "off: moved");
        assert_eq!(spread(&mut w, egg, start, material, &def, Some(crowd + 1), EggDoor::Off), start, "one short of crowded: moved");
        let moved = spread(&mut w, egg, start, material, &def, Some(crowd), EggDoor::Off);
        assert_ne!(moved, start, "a crowded egg with a nestmate beside it was not spread");
        assert_eq!(w.get(moved.0, moved.1).organism_id(), egg, "the egg's cell did not move with it");
        assert!(w.is_empty(start.0, start.1), "the egg was copied, not moved");
        assert_eq!(w.creature_stats.brood_spread, 1);
        assert!((moved.0 - start.0).abs().max((moved.1 - start.1).abs()) >= BROOD_SPREAD_MIN, "spread only to {moved:?}, inside its own pile");
        assert!(creature::home_at(&w, moved.0, moved.1, &def), "spread out of home to {moved:?}");
        assert!(round(&w, material, moved) <= crowd - 2, "spread into another crowd at {moved:?}");
        assert_eq!(spread(&mut w, egg, moved, material, &def, Some(crowd), EggDoor::Off), moved, "spread again once out of the crowd");
        // Food near every spot that won: never put down beside it.
        let (mut w, def, _, material, egg, start) = scene(true);
        let to = spread(&mut w, egg, start, material, &def, Some(crowd), EggDoor::Off);
        assert!((to.0 - 67).abs().max((to.1 - 48).abs()) > BROOD_SPREAD_FOOD, "spread to {to:?}, beside the food at (67, 48)");
    }

    /// **Whether to lay is the brain's call** (`BrainOutput::Lay`). A rich
    /// ant whose `Lay` reads below `LAY_HOLD_BELOW` keeps its egg and its
    /// joules, and the hold is counted; the same ant just above the hold
    /// lays. And the authored ant's own `Lay` is exactly 0.0 whatever it
    /// senses, which is why generation zero lays exactly as it did before
    /// the output existed. Put the fault back by dropping the gate in
    /// `try_bud` and the first assertion goes red.
    #[test]
    fn a_brain_that_holds_its_egg_keeps_it() {
        use crate::sim::brain;
        let (mut w, ant, def) = bed(true);
        let e0 = w.organism(ant).expect("alive").energy;
        assert!(
            creature::try_bud(&mut w, ant, &def, 0.0, brain::LAY_HOLD_BELOW - 0.01).is_none(),
            "laid against a hold"
        );
        assert_eq!(w.creature_stats.lays_declined, 1);
        assert_eq!(w.creature_stats.eggs_laid, 0);
        assert_eq!(w.organism(ant).expect("alive").energy, e0, "a held egg cost the ant something");
        assert!(
            creature::try_bud(&mut w, ant, &def, 0.0, brain::LAY_HOLD_BELOW + 0.01).is_some(),
            "a Lay just above the hold did not lay"
        );
        assert_eq!((w.creature_stats.lays_declined, w.creature_stats.eggs_laid), (1, 1));

        let genome = w.organism(ant).expect("alive").genome.clone();
        assert!(
            !brain::output_row_wired(&genome, brain::BrainOutput::Lay),
            "ant.ron wires Lay, so generation zero no longer lays as it did"
        );
        for fill in [0.0f32, 1.0, -1.0, 0.5] {
            let mut hidden = [0.3f32; brain::BRAIN_HIDDEN];
            let (out, _) = brain::eval_brain(&genome, &[fill; brain::BRAIN_INPUTS], &mut hidden);
            assert_eq!(
                out[brain::BrainOutput::Lay as usize],
                0.0,
                "an unwired Lay read non-zero at inputs {fill}"
            );
        }
    }
}
