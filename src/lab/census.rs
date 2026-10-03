//! **The late-game census, as a library function.**
//!
//! Built for `Reports/evolution-lab-late-game-design-2026-09-12.md` brief 0
//! and moved here from `examples/latecensus.rs` so the headless harness and
//! the lab's own chronicle read the same numbers off the same code -- the
//! whole point of a CENSUS section in a chronicle is that it is *this*
//! table, not a second guess at it that can drift.
//!
//! What a session run to hundreds of ants cannot answer from the run log
//! alone (`Lab::write_chronicle`'s own doc): how many ants, how many living
//! plants, the seed bank, the larder split by kind in joules, the nest's
//! footprint (roofed chambers, an open pit, packed spoil, the mound above
//! the original surface) and the dead zone around a nest where nothing
//! grows. `census()` reads all of it off one pass of the grid; everything
//! else here packages a sample for a chronicle row.
//!
//! `examples/latecensus.rs` is this module's own harness -- it calls
//! straight through, so its own doc comment (the columns, the selftest, the
//! command lines) is still the reference for what each field means.

use crate::sim::cell::OrganismId;
use crate::lab::scenario::{Placement, Scenario};
use crate::lab::scene::LabBox;
use crate::sim::creature::{diet_yield, EAT_YIELD_THRESHOLD};
use crate::sim::material::{self, MaterialId, MaterialKind};
use crate::sim::organism::{self, DeathCause, TRAIT_GUT_BIAS};
use crate::sim::world::World;

/// Half-width of the nest band, in columns, for the dead-zone ratio. The
/// played bed founds in the open ground 210..310 and a colony's home range
/// on it is a few tens of columns either side of the door, so 64 covers the
/// mound and the traffic without reaching the far stands.
pub const BAND: i32 = 64;

/// How far above the original surface ground still counts as the bed's own
/// (a mound, a heap of spoil) rather than the box's lid or a lamp.
pub const MOUND_REACH: i32 = 48;

/// The material ids the census buckets by, resolved once.
pub struct Ids {
    leaf: Vec<MaterialId>,
    fruit: Vec<MaterialId>,
    litter: Vec<MaterialId>,
    seed: Vec<MaterialId>,
    corpse: Vec<MaterialId>,
    flower: Vec<MaterialId>,
    ground: Vec<MaterialId>,
    /// Public: `examples/latecensus.rs`'s `selftest` places a packed-soil
    /// cell by hand to carve its known mound, and needs the id directly.
    pub packed: Option<MaterialId>,
    /// **Every worked ground, for the `packed_above`/`packed_below` columns.**
    /// `packedsoil` is the wall an ant cut in place and `spoil` is the pellet
    /// it hauled out (`assets/materials/spoil.ron`, §Z18); both are tamped and
    /// both belong in a count of worked ground, so this is a set and `packed`
    /// above stays the single lining id the selftest plants by hand. Without
    /// it, splitting the pellet out of `packedsoil` would have silently taken
    /// `packed_above` -- the mound column every lane reads -- to near zero.
    packed_any: Vec<MaterialId>,
}

impl Ids {
    pub fn resolve(world: &World) -> Self {
        let ids = |names: &[&str]| names.iter().filter_map(|n| world.materials.id_of(n)).collect::<Vec<_>>();
        Ids {
            leaf: ids(&["leaf", "grassblade", "moss"]),
            fruit: ids(&["fruit", "windfall"]),
            litter: ids(&["litter", "deadleaf"]),
            seed: ids(&["seed", "pip"]),
            corpse: ids(&["corpse"]),
            flower: ids(&["flower"]),
            ground: ids(&["soil", "packedsoil", "spoil", "nest"]),
            packed: world.materials.id_of("packedsoil"),
            packed_any: ids(&["packedsoil", "spoil"]),
        }
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Sample {
    pub ants: usize,
    pub plants: usize,
    pub seed_bank: usize,
    pub edible: usize,
    pub worth: f64,
    pub leaf_j: f64,
    pub fruit_j: f64,
    pub litter_j: f64,
    pub seed_j: f64,
    pub corpse_j: f64,
    pub flower_j: f64,
    pub other_j: f64,
    pub standing_flowers: usize,
    /// Void below the original surface with ground somewhere above it in
    /// the same column -- a chamber or a gallery.
    ///
    /// **This counts cells that are materially EMPTY, and an ant standing
    /// in a gallery is not empty** -- so on its own it is not the size of
    /// the nest. See [`Sample::roofed_bodies`] and [`Sample::room_total`].
    pub roofed: usize,
    /// Void below the original surface open to the sky -- a pit.
    pub pit: usize,
    /// **Roofed room that is occupied rather than empty** -- creature cells
    /// standing below the original surface with ground above them.
    ///
    /// Measured 2026-09-19 in `examples/digbox`: roofed 157 + open 165 +
    /// **bodies 692** = 1,014, against **941** cells hauled above the
    /// original surface. Digging only *moves* material
    /// (`spoil_dumped/digs` = 0.986), so net void below and material above
    /// are two sides of one conservation law and must account for each
    /// other. On [`Sample::roofed`] alone that identity fails by **619
    /// cells**; with the bodies counted it closes to 8%.
    ///
    /// Three results in the nest line were filed as nulls having been
    /// scored on the unoccupied count alone, which undercounts a dense
    /// colony's nest by about threefold. `CLAUDE.md`'s metric trap --
    /// *what a player calls a nest is roofed void* -- is right about the
    /// roof and silent about the occupant.
    ///
    /// **Split from [`Sample::pit_bodies`] deliberately.** `digbox`'s own
    /// census carries one undivided `bodies` total, which cannot tell an
    /// ant standing in an open pit from one standing in a gallery; this
    /// module already separates roofed from open and would otherwise
    /// inherit that conflation.
    pub roofed_bodies: usize,
    /// As [`Sample::roofed_bodies`], for void open to the sky.
    pub pit_bodies: usize,
    pub packed_below: usize,
    pub packed_above: usize,
    /// Soil, packed soil or nest cells standing above the original surface.
    pub mound: usize,
    /// Rows above the original surface of the highest such cell.
    pub mound_high: i32,
    /// Columns within `BAND` of a nest holding no plant cell at all, and the
    /// band's width; the same outside it.
    pub bare_in_band: usize,
    pub band_cols: usize,
    pub bare_outside: usize,
    pub outside_cols: usize,
    pub plant_cells_in_band: usize,
    pub plant_cells_outside: usize,
    /// **Columns inside `BAND` whose surface stands above the original
    /// ground** -- the anthill itself, as a footprint.
    pub mound_cols: usize,
    /// ...of those, the ones carrying no plant cell in the four rows above
    /// that surface.
    ///
    /// **The per-column `bare_in_band` above cannot answer "is the anthill
    /// green", and this is why it gets its own pair.** A column is "not bare"
    /// to that metric if a plant stands anywhere in it, floor to lid -- so a
    /// seedling at the foot of a mound marks the whole column vegetated while
    /// the cemented slope above it is bare rock. `CLAUDE.md`'s *ask what your
    /// number counts*, landing on the question the room-per-ant build is
    /// judged by.
    pub mound_bare: usize,
    /// **Corpse cells standing on the bed.** A stock, not a rate: the death
    /// counters say how many animals went and this says how much of them is
    /// still lying there uneaten, which is the half that decides whether age
    /// deaths *feed* the colony or merely remove mouths from it.
    ///
    /// Counted before the `diet_yield` gate that fills `corpse_j`, not inside
    /// it: a corpse burnt to charcoal or half digested can fall under the
    /// edible threshold and still be lying on the ground.
    pub corpses: usize,
}

impl Sample {
    /// **The size of the nest: room whether or not somebody is standing in
    /// it.** This, not [`Sample::roofed`], is what a question about how much
    /// a colony has dug should read.
    ///
    /// `roofed + pit + roofed_bodies + pit_bodies`. The conservation
    /// identity it satisfies is the check that found the defect: digging
    /// moves material rather than destroying it, so this total should
    /// account for the material standing above the original surface. Read
    /// against `roofed` alone the identity fails by hundreds of cells on any
    /// colony dense enough to fill its own galleries.
    ///
    /// [`Sample::roofed_room`] is the same quantity for the roofed half
    /// alone, which is the one a player would call a nest -- *a hole open to
    /// the sky is not a room*.
    pub fn room_total(&self) -> usize {
        self.roofed + self.pit + self.roofed_bodies + self.pit_bodies
    }

    /// Roofed room, occupied or not -- `roofed + roofed_bodies`.
    pub fn roofed_room(&self) -> usize {
        self.roofed + self.roofed_bodies
    }
}

fn is_waiting_seed(world: &World, id: OrganismId, state: &organism::OrganismState) -> bool {
    // **A seed riding in an ant's crop is still bank, and owns no cell at
    // all while it rides** -- round 29, Brief 1. `plant::take_seed_passenger`
    // lifts the seed's one cell out of the world and keeps the organism live
    // (`World::carried_seed_organisms`), so the one-cell test below says no
    // and the passenger would otherwise be counted as a *plant*. One per
    // carrying ant, and wrong in the direction that flatters the seed-cargo
    // build, which is why it is closed here rather than noted.
    //
    // **This survived a file move**: the census body moved out of
    // `examples/latecensus.rs` into this module on the same day the rule was
    // added, and the version that moved was the one without it. Taking the
    // moved side wholesale would have put every passenger back in the plant
    // column without a single test going red.
    if world.is_carried_seed(id) {
        return true;
    }
    state.cells.len() == 1
        && state
            .cells
            .keys()
            .next()
            .map(|(x, y)| organism::cell_type(world.get(*x, *y).aux()) == Some(organism::CellType::Seed))
            .unwrap_or(false)
}

/// **The rows and columns the census walks -- the world's, not the spec's.**
///
/// `LabBox` describes the bed that *will* be built on the next REBUILD, and
/// `params::write_bed` moves it the instant a row on the parameters page is
/// nudged; `raising_the_width_and_rebuilding_gives_a_wider_world` asserts
/// that the running world is deliberately left alone until then. So a spec
/// and the world it is censusing can be different shapes for as long as the
/// player leaves the page without pressing the button, which is most of a
/// played session. Falls back to the spec only for a world with no bounds
/// at all, which cannot be walked either way.
fn extents(world: &World, spec: &LabBox) -> (std::ops::RangeInclusive<i32>, std::ops::RangeInclusive<i32>) {
    match world.bounds() {
        Some(b) => (b.min_x..=b.max_x, b.min_y..=b.max_y),
        None => (0..=(spec.width - 1), 0..=(spec.height - 1)),
    }
}

/// **The original top-of-ground row for one column**, from the world's own
/// frozen datum ([`World::room_surface_at`]), with `LabBox::ground_y` as the
/// fallback for a world that has never been stepped.
///
/// **This is the repair of the five dead columns.** Read off the spec, the
/// datum is whatever `ground_y` currently says; in the owner's 560,000-frame
/// playtest he raised the box height during setup, `ground_y` rode the
/// height (`params::write_bed`'s `"height"` arm scales it deliberately) and
/// never rebuilt, so the census was measuring a surface **96 rows below the
/// one the world had** -- down in the stone base. Everything scoped to
/// "below the original surface" then reads a region that holds no void and
/// no worked soil: `roofed` **0**, `pit` **0**, `pack<` **0** across all 56
/// samples of a session with 356,688 digs, and `mnd` pinned at exactly
/// `MOUND_REACH` because the whole 48-row window above the false datum was
/// solid bed. `pack^` did not read zero and that was not it working -- with
/// the datum in the stone it was counting the *deep gallery lining* as
/// mound.
///
/// Per column rather than one number for the bed, because that is what the
/// world stores and what `World::step_nest_room` already reads: two rules
/// for one word is what `freeze_room_datum`'s own doc records going wrong
/// the last time.
fn surface_of(world: &World, spec: &LabBox, x: i32) -> i32 {
    world.room_surface_at(x).unwrap_or(spec.ground_y)
}

/// One pass of the grid: the stand, the larder by kind, the nest's
/// footprint and the dead-zone ratio. See this module's own doc and
/// `examples/latecensus.rs`'s for what each field answers and
/// `control=selftest` for the carved-cavity positive control.
pub fn census(world: &World, spec: &LabBox, gut: f32, nest_cols: &[i32], ids: &Ids) -> Sample {
    let mut s = Sample { mound_high: 0, ..Sample::default() };
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_some() {
            s.ants += 1;
        } else if is_waiting_seed(world, id, state) {
            s.seed_bank += 1;
        } else {
            s.plants += 1;
        }
    }
    let (xs, ys) = extents(world, spec);
    let width = (xs.end() - xs.start() + 1) as usize;
    let mut has_plant = vec![false; width];
    let mut plant_cells = vec![0usize; width];
    for x in xs.clone() {
        let col = (x - xs.start()) as usize;
        let surface = surface_of(world, spec, x);
        let mut covered = false;
        for y in ys.clone() {
            let cell = world.get(x, y);
            let kind = world.materials.kind(cell.material);
            let is_ground = cell.material != material::EMPTY
                && matches!(kind, MaterialKind::Powder | MaterialKind::Solid)
                && cell.organism_id() == 0;
            if kind == MaterialKind::Plant || (cell.organism_id() != 0 && world.organism(cell.organism_id()).is_some_and(|st| world.species.get(st.species).creature.is_none())) {
                has_plant[col] = true;
                plant_cells[col] += 1;
            }
            // **Cover is ground near the surface, not the lid.** The lab box
            // is sealed, so row 0 is a wall in every column and a plain
            // "anything solid above" read the whole bed as roofed (selftest:
            // a shaft open to the sky counted as a chamber). A mound is a few
            // rows; the lid is 160 up.
            if is_ground && y >= surface - MOUND_REACH {
                covered = true;
                if y < surface && ids.ground.contains(&cell.material) {
                    s.mound += 1;
                    s.mound_high = s.mound_high.max(surface - y);
                }
                if ids.packed_any.contains(&cell.material) {
                    if y < surface {
                        s.packed_above += 1;
                    } else {
                        s.packed_below += 1;
                    }
                }
            } else if y >= surface && world.is_empty(x, y) {
                if covered {
                    s.roofed += 1;
                } else {
                    s.pit += 1;
                }
            // **A gallery with an ant in it is still a gallery.** The arm
            // above counts materially EMPTY cells only, so every occupied
            // cell of the nest fell through this chain and was counted as
            // nothing -- see `Sample::roofed_bodies` for the conservation
            // failure that found it.
            //
            // **The material test, not `organism_id() != 0`.** A plant cell
            // carries an organism id too, which is why the plant branch
            // above pairs the id with `creature.is_none()`. Matching on
            // `MaterialKind::Creature` also leaves a corpse out, correctly:
            // a corpse is a `Powder`, it is ground and larder, not room.
            // `creature::is_animal_cell` states the same rule and is
            // private to that module, so this inlines it rather than
            // widening its visibility for one caller.
            } else if y >= surface && kind == MaterialKind::Creature {
                if covered {
                    s.roofed_bodies += 1;
                } else {
                    s.pit_bodies += 1;
                }
            }
            if ids.flower.contains(&cell.material) {
                s.standing_flowers += 1;
            }
            // **Before the `diet_yield` gate below, deliberately** -- see the
            // field's own doc. A corpse that has burnt or been half eaten can
            // fall under `EAT_YIELD_THRESHOLD` and still be a body lying on
            // the bed, which is what this counts.
            if ids.corpse.contains(&cell.material) {
                s.corpses += 1;
            }
            let yielded = diet_yield(world, cell, gut);
            if yielded <= EAT_YIELD_THRESHOLD {
                continue;
            }
            if world.organism(cell.organism_id()).is_some_and(|st| world.species.get(st.species).creature.is_some()) {
                continue;
            }
            s.edible += 1;
            s.worth += yielded as f64;
            let m = cell.material;
            let j = yielded as f64;
            if ids.leaf.contains(&m) {
                s.leaf_j += j;
            } else if ids.fruit.contains(&m) {
                s.fruit_j += j;
            } else if ids.litter.contains(&m) {
                s.litter_j += j;
            } else if ids.seed.contains(&m) {
                s.seed_j += j;
            } else if ids.corpse.contains(&m) {
                s.corpse_j += j;
            } else if ids.flower.contains(&m) {
                s.flower_j += j;
            } else {
                s.other_j += j;
            }
        }
    }
    // **Bare on the mound's own surface**, walked as its own pass because it
    // asks a different question from the column loop below: not "does this
    // column hold a plant" but "is the ground you can see on the anthill
    // growing anything".
    for x in xs.clone() {
        // **Inside the band only, which is what makes this the anthill's
        // surface rather than the bed's.** Measured 2026-09-12 on seed 1 at
        // 20,000 frames, unbanded: **141 mound columns** with only 19 packed
        // cells above the surface -- litter rotting to soil high on a drift,
        // which the late-game report already records as why `mound_high`
        // reads 24-40 rows on the *unfed* bed. Half the bed is not an
        // anthill.
        if nest_cols.iter().map(|c| (c - x).abs()).min().unwrap_or(i32::MAX) > BAND {
            continue;
        }
        // **The same window *and* the same material set the `mound` column
        // above uses**, and both halves were paid for: a first pass looked
        // for the topmost `Powder`/`Solid` below the lid and read **9 mound
        // columns in a bare box**. The window alone did not fix it -- the
        // grow lamps hang within `MOUND_REACH` of the surface, and a lamp is
        // not an anthill. A mound is made of the bed's own ground, which is
        // what `Ids::ground` is.
        let surface = surface_of(world, spec, x);
        let Some(top) = ((surface - MOUND_REACH).max(*ys.start())..=*ys.end()).find(|&y| {
            let cell = world.get(x, y);
            ids.ground.contains(&cell.material) && cell.organism_id() == 0
        }) else {
            continue;
        };
        if top >= surface {
            continue;
        }
        s.mound_cols += 1;
        // Four rows, not one: a seedling rooted in the slope stands above the
        // cell it is rooted in, and a metric that only looked at the surface
        // cell itself would call every planted mound bare.
        let green = ((top - 4).max(0)..top).any(|y| {
            let cell = world.get(x, y);
            world.materials.kind(cell.material) == MaterialKind::Plant
                || (cell.organism_id() != 0 && world.organism(cell.organism_id()).is_some_and(|st| world.species.get(st.species).creature.is_none()))
        });
        if !green {
            s.mound_bare += 1;
        }
    }
    for x in xs.clone() {
        let col = (x - xs.start()) as usize;
        let d = nest_cols.iter().map(|c| (c - x).abs()).min().unwrap_or(i32::MAX);
        let bare = !has_plant[col];
        if d <= BAND {
            s.band_cols += 1;
            s.plant_cells_in_band += plant_cells[col];
            if bare {
                s.bare_in_band += 1;
            }
        } else {
            s.outside_cols += 1;
            s.plant_cells_outside += plant_cells[col];
            if bare {
                s.bare_outside += 1;
            }
        }
    }
    s
}

/// **The mean gut bias over every living animal** -- what `census` prices
/// the larder at (`diet_yield` needs a gut to price a cell against). 0.0 on a
/// bed with no animal alive, as before.
///
/// **It was the first live animal's gut, and that stopped being one number.**
/// The doc used to say *every ant so far seen on the bed shares one species'
/// trait, so any live ant answers it* -- true until heredity let the gut
/// drift. The 10-03 playtest (`chronicle-herb_ant-s1.txt`, spec F23) had its
/// `worth(J)`/`leafJ` columns **triple at frame 220,000 from a change of
/// ant, not of food**: the first slot in `live_organism_ids` died, the next
/// one carried a drifted gut, and the whole bed was repriced. A mean moves
/// only as fast as the population does, so a step in those columns is now a
/// step in the larder. The gut used is printed on each row's addendum line
/// (`row_addendum`) so a reader can still tell the two apart.
///
/// Every animal species, not only the colony's, the way the first-found rule
/// it replaces was: a bed with predators averages them in. Name kept, so
/// `examples/latecensus.rs` and `examples/chronicle.rs` move to the mean with
/// no edit (their `worth` columns change on any bed whose guts have drifted;
/// `examples/labforage.rs` keeps a private first-found copy and does not).
pub fn ant_gut_bias(world: &World) -> f32 {
    let (mut sum, mut n) = (0.0f64, 0u32);
    for s in world.live_organism_ids().iter().filter_map(|id| world.organism(*id)) {
        if world.species.get(s.species).creature.is_some() {
            sum += f64::from(s.traits[TRAIT_GUT_BIAS]);
            n += 1;
        }
    }
    if n == 0 {
        0.0
    } else {
        (sum / f64::from(n)) as f32
    }
}

/// **Kills of a colony-species animal by its own colony**, read off
/// `World::group_deaths` -- the `killed_by` entries whose attacker species and
/// colony equal the victim group's own. A pure read of what the bite already
/// tallies, not a second counter (`creature.rs`'s test helper
/// `own_colony_kills` is the same sum over every species).
///
/// Spec D16/E19: the 10-03 findings had to read *"Ant 4 lost 14 of 33 to
/// killed by Ant 4"* off the LEGENDS prose, and the row's `killd` cannot say
/// whether a colony is at war or eating itself.
pub fn own_colony_kills(world: &World, species: &str) -> u64 {
    world
        .group_deaths
        .iter()
        .filter(|g| world.species.get(g.species).name == species)
        .map(|g| g.killed_by.iter().filter(|(sp, col, _)| *sp == g.species && *col == g.colony).map(|(_, _, n)| *n).sum::<u64>())
        .sum()
}

/// The colony species' deaths by cause, colonies rolled up.
/// Returns `(starved, killed, oldage, other)`.
///
/// **Old age gets a column of its own rather than being folded into
/// `other`**, which is what it would be if this simply counted variants. The
/// question the lifespan build asks is whether a colony stops at a size
/// instead of eating the bed, and "it settled" and "it ran out of food" are
/// the *same* population line -- only the split between this column and
/// `starved` tells them apart.
pub fn colony_deaths(world: &World, species: &str) -> (u64, u64, u64, u64) {
    let (mut starved, mut killed, mut oldage, mut other) = (0, 0, 0, 0);
    for g in &world.group_deaths {
        if world.species.get(g.species).name != species {
            continue;
        }
        for (i, n) in g.by_cause.iter().enumerate() {
            if i == DeathCause::Starved.index() || i == DeathCause::StarvedInFlight.index() {
                starved += n;
            } else if i == DeathCause::Killed.index() {
                killed += n;
            } else if i == DeathCause::OldAge.index() {
                oldage += n;
            } else {
                other += n;
            }
        }
    }
    (starved, killed, oldage, other)
}

/// Columns any ant colony is founded at over this bed's life: **every nest
/// the world actually holds** (`World::nest_sites`), plus the ones the bed
/// was built with (`LabBox::colony_columns`) and whatever a scenario's own
/// placements or timeline add later.
///
/// **`World::nest_sites` is the half that was missing, and it is the half
/// that matters on a played bed.** The other two sources are both *specs* --
/// they say what was asked for at build time. A colony the player founds
/// with the colony key mints a nest patch and a site
/// (`creature::paint_nest_patch` -> `World::register_nest_site`) and touches
/// neither. The owner's 560,000-frame playtest is exactly that session: its
/// chronicle header reads `FOUNDERS 0  COLONIES 0` and it ran five
/// hand-placed long-ant colonies, so `nest_cols` came back empty, every
/// column fell outside the band, and the band read **`0/0` in all 56
/// samples** -- the dead-zone ratio the owner's own complaint ("they dig
/// large chambers ... which creates an area where plants don't grow") is
/// measured by, dividing by an empty set.
///
/// The spec halves are kept rather than replaced: a scenario timeline can
/// name a colony that has not landed yet, and a site is only minted when
/// the patch is painted.
///
/// A nest does not move once founded, but one can be founded at any moment
/// of a played session, so **recompute this per sample** rather than
/// resolving it once before the run.
pub fn nest_columns(world: &World, spec: &LabBox, scenario: Option<&Scenario>) -> Vec<i32> {
    let mut v = spec.colony_columns();
    v.extend(world.nest_sites.iter().map(|n| n.x));
    if let Some(s) = scenario {
        v.extend(s.placements.iter().chain(s.timeline.iter().map(|e| &e.what)).filter_map(|p| match p {
            Placement::Colony { x, .. } => Some(*x),
            _ => None,
        }));
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// **How the box was keeping up, sampled beside the census.** `None` on
/// `ChronicleRow::perf` when no [`crate::lab::time::TimeControl`] is
/// available at all -- `examples/chronicle.rs`'s own harness drives
/// `sim::frame::step` directly with no dial, so "achieved against
/// requested" simply does not apply there. Every field here is a read of
/// state `TimeControl` already tracks for its own on-screen readout
/// (`ticks_per_frame`/`requested_ticks_per_frame`/`multiple`/`display_hz`/
/// `owed_ticks`/`draws_skipped`); nothing here is new measurement, and none
/// of it costs more than the handful of field reads it looks like --
/// confirmed against `World::active_chunk_count`/`active_site_count`
/// (`ChronicleRow`'s own `awake_chunks`/`active_sites`, sampled
/// unconditionally) before adding either: both are already O(chunks)/O(1),
/// the same cost the debug overlay pays every drawn frame.
///
/// **Round 31's own reason this exists**: `ChronicleRow`'s other fields are
/// all world *content* -- ants, plants, joules, mound geometry -- and none
/// of them can say whether a session was ever slow. The owner's next round
/// is a perf deep-dive on a played box past a thousand ants, and this is
/// the load a chronicle needs to carry for that to be answerable from a
/// log rather than reproduced from a guess.
///
/// **Read `speed_multiple` beside anything else here before trusting it.**
/// The same achieved `ticks_per_frame` means "keeping up" at `1X` and
/// "badly behind" near the top of the speed ladder -- many simulated ticks
/// run inside one drawn frame at a high multiplier, so the rate alone does
/// not say which. This is the same caution `autosave_cost`'s own doc gives
/// for a single call landing inside one displayed frame.
#[derive(Default, Clone, Copy, Debug)]
pub struct PerfSample {
    /// Ticks actually shown per displayed frame -- `TimeControl::
    /// ticks_per_frame`, what the screen is actually doing.
    pub ticks_per_frame: u32,
    /// What the dial's multiplier would put on screen if the box could meet
    /// it -- `TimeControl::requested_ticks_per_frame`. Diverges from
    /// `ticks_per_frame` exactly when the box cannot keep up; the gap
    /// between the two, not either number alone, is "is it keeping up".
    pub requested_ticks_per_frame: u32,
    /// The speed dial's multiplier at the moment of the sample --
    /// `TimeControl::multiple` (`0` while paused). See this struct's own
    /// doc: required context for every other field here, not optional.
    pub speed_multiple: u32,
    /// Displayed frames per second -- `TimeControl::display_hz`.
    pub display_hz: u32,
    /// Whole simulated ticks currently owed and not yet run --
    /// `TimeControl::owed_ticks`. Separates a frame that merely ran a
    /// little short from a box that has fallen behind and is not catching
    /// up, which `ticks_per_frame` alone cannot: a debt that keeps growing
    /// says the box has given up on the dial, not just missed one frame.
    pub debt_ticks: u32,
    /// How many displayed frames have been skipped (ticked but not drawn,
    /// to buy ticks at a high dial) over the whole run so far --
    /// `TimeControl::draws_skipped`. The sim-bound/render-bound fork: a
    /// session with a high, climbing skip count spent its time simulating
    /// rather than painting, which is the first question any perf work
    /// needs answered and today cannot be, from a log alone.
    pub draws_skipped: u64,
}

/// **One CENSUS row**: a `Sample` plus the colony-turnover numbers that live
/// on `World` rather than in the grid -- everything `examples/latecensus.rs`
/// prints for one sampled frame, bundled so the lab and the harness call one
/// function and cannot drift on which counters go with which frame.
#[derive(Default, Clone, Copy, Debug)]
pub struct ChronicleRow {
    pub frame: u64,
    /// Unix seconds at the moment this row was taken. Alongside `frame`
    /// because a frame count alone cannot be turned into real played
    /// minutes -- the speed dial and how often the player paused both sit
    /// between the two, and a perf reader's first question about a session
    /// is how long it actually ran.
    pub wall_clock_secs: u64,
    pub sample: Sample,
    pub births: u64,
    pub deaths: u64,
    pub starved: u64,
    pub killed: u64,
    pub other_deaths: u64,
    pub eats: u64,
    pub digs: u64,
    pub deliveries: u64,
    /// Chunks that will be swept next step -- `World::active_chunk_count`,
    /// the headline number for whether sleeping is working. Always sampled
    /// (unlike `perf`, this needs no `TimeControl` -- it is a plain read off
    /// `world`), because it is the missing link between "there are 1,000
    /// ants" and "are they costing anything": a sleeping ant is free, and
    /// the census alone cannot tell which this session had.
    pub awake_chunks: usize,
    /// Pending active sites -- `World::active_site_count`, the scheduler's
    /// own headline number for whether its cost is proportional to
    /// "interesting cells" rather than world size. Read beside
    /// `awake_chunks` for the same reason.
    pub active_sites: usize,
    /// How the box was keeping up at the moment of the sample. `None` for a
    /// harness with no dial at all -- see [`PerfSample`]'s own doc.
    pub perf: Option<PerfSample>,
    /// The gut bias the larder columns were priced at -- the `gut` argument
    /// `take_chronicle_row` was handed, which is `ant_gut_bias`'s population
    /// mean in both callers. Printed on the addendum line.
    pub gut: f32,
    /// Colony-species deaths of old age -- `colony_deaths`'s third value.
    /// **`other_deaths` still includes these**, so the `othr` column reads the
    /// same as in every chronicle landed before this field; `oldag` in the
    /// counters group is the split.
    pub oldage_deaths: u64,
    /// `own_colony_kills` for the colony species: how many of `killed` were
    /// killed by their own colony.
    pub kills_own_colony: u64,
    /// **The dig funnel and the nest, cumulative, straight off
    /// `CreatureStats`** (spec D16/D18): `dig_rolls` (the urge fired),
    /// `digs_aimed_down` (the roll turned downward), `digs_down_refused` (no
    /// way down: all three cells under the animal uncuttable),
    /// `digs_refused_roof` (the cut lay in a nest's roof), and
    /// `at_nest_ticks` (creature ticks taken standing at a nest). `digs` alone
    /// cannot split *why not more digging* into "no roll", "vetoed" and "no
    /// jaw"; these and `digs` beside them can. See each field's own doc on
    /// `CreatureStats` for exactly what it counts.
    pub dig_rolls: u64,
    pub digs_aimed_down: u64,
    pub digs_down_refused: u64,
    pub digs_refused_roof: u64,
    pub at_nest_ticks: u64,
    /// **The seed-rides-home loop, cumulative, off `World`** (spec F24):
    /// passengers loaded (`seeds_carried`), set down on wet ground
    /// (`pips_set_on_soil`) or left on dry nest ground (`pips_set_on_nest`),
    /// and lost with nowhere to go (`seeds_lost_no_room`). The loop the owner
    /// watched in the 10-03 session and the file could not confirm.
    pub seeds_carried: u64,
    pub pips_set_on_soil: u64,
    pub pips_set_on_nest: u64,
    pub seeds_lost_no_room: u64,
    /// **Where the tick actually went, since the previous row** -- the eight
    /// phases `sim::frame::step` orders, drained from its stopwatch. `None`
    /// when the clock is off: **on by default in the lab's own binary**
    /// (`frame::phase_clock_default_on`, unless `PIXEL_PHYSICS_PHASE_CLOCK=0`)
    /// and **off by default everywhere else** -- every harness, including
    /// `examples/chronicle.rs`, unless `PIXEL_PHYSICS_PHASE_CLOCK=1`. This
    /// doc used to call `=1` "the default", which it never was until the lab
    /// made it so.
    ///
    /// This column group is the answer to the one question the owner's own
    /// session log could not ask. `Reports/evolution-lab-playtest-2026-09-13.md`
    /// §1 records `awake chunks` going 12 -> 52 as ants went 39 -> 2,473 and
    /// has no split at all, so "is the field a large part of this" stayed an
    /// estimate across two machines
    /// (`Reports/ant-sim-research-review-2026-09-19.md` §8.4). It lands *in
    /// this row* rather than in a log line of its own precisely so the split
    /// and `awake_chunks` are read off the same sample: the mechanism that
    /// makes the field track ant count is the chunks the ants wake, and the
    /// two numbers only mean anything together.
    ///
    /// A **window**, not a session mean -- see `frame::take_phase_times`.
    pub phases: Option<crate::sim::frame::PhaseTimes>,
}

/// Take one `ChronicleRow` off `world` right now, at `world.frame`.
///
/// `time` is `None` for a headless harness with no dial (`examples/
/// chronicle.rs`'s own bare `World` loop) and `Some(&lab.time)` for a real
/// `Lab` (`Lab::tick`) -- see [`PerfSample`]'s own doc for why the perf
/// columns cannot be filled in without one.
pub fn take_chronicle_row(
    world: &World,
    spec: &LabBox,
    gut: f32,
    nest_cols: &[i32],
    ids: &Ids,
    colony_species: &str,
    time: Option<&crate::lab::time::TimeControl>,
) -> ChronicleRow {
    let sample = census(world, spec, gut, nest_cols, ids);
    let st = world.creature_stats;
    // The chronicle's row has no old-age column of its own yet, so an age
    // death lands in `other_deaths` here rather than being dropped.
    let (starved, killed, oldage, other) = colony_deaths(world, colony_species);
    // `othr` keeps old age in it, as it always has; `oldage_deaths` below is
    // the split, printed in its own column at the end of the line.
    let other_deaths = other + oldage;
    let wall_clock_secs =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let perf = time.map(|t| PerfSample {
        ticks_per_frame: t.ticks_per_frame(),
        requested_ticks_per_frame: t.requested_ticks_per_frame(),
        speed_multiple: t.multiple(),
        display_hz: t.display_hz(),
        debt_ticks: t.owed_ticks(),
        draws_skipped: t.draws_skipped(),
    });
    ChronicleRow {
        frame: world.frame,
        wall_clock_secs,
        sample,
        births: st.births,
        deaths: st.deaths,
        starved,
        killed,
        other_deaths,
        eats: st.eats,
        digs: st.digs,
        deliveries: st.deliveries,
        awake_chunks: world.active_chunk_count(),
        active_sites: world.active_site_count(),
        perf,
        gut,
        oldage_deaths: oldage,
        kills_own_colony: own_colony_kills(world, colony_species),
        dig_rolls: st.dig_rolls,
        digs_aimed_down: st.digs_aimed_down,
        digs_down_refused: st.digs_down_refused,
        digs_refused_roof: st.digs_refused_roof,
        at_nest_ticks: st.at_nest_ticks,
        seeds_carried: world.seeds_carried,
        pips_set_on_soil: world.pips_set_on_soil,
        pips_set_on_nest: world.pips_set_on_nest,
        seeds_lost_no_room: world.seeds_lost_no_room,
        // **Drained here, unconditionally, and that is safe because the
        // accumulator is empty when the clock is off.** Draining rather than
        // reading is what makes each row the window since the previous one;
        // a session mean over a hundred thousand ticks could not show the
        // field's share moving as a colony grows, which is the whole point.
        // `ticks == 0` means the clock never ran, and the printer says `--`
        // rather than `0.000` for it -- the same distinction the perf columns
        // above already make, and for the same reason.
        phases: crate::sim::frame::phase_clock_on().then(crate::sim::frame::take_phase_times),
    }
}

/// The column-header line, in the exact order and widths
/// `examples/latecensus.rs` has always printed -- shared so a chronicle row
/// and a `latecensus` row are the same text or nothing here has done its job.
///
/// **The final `| wall awake sites | ach/f req/f x dispHz debt skip` group is
/// round 31's own addition and `examples/latecensus.rs` does not carry it**
/// -- that harness has no `TimeControl` at all (it drives `sim::frame::step`
/// directly), so the perf columns have nothing to read there, and that file
/// is out of this round's scope besides. The two headers were already not
/// byte-identical before this (`latecensus.rs` carries `oldag`/`crpss`
/// `header_line` does not); this widens the gap rather than opening it.
pub fn header_line() -> String {
    format!(
        "{:>7} {:>5} {:>5} {:>5} {:>6} {:>9} | {:>8} {:>7} {:>7} {:>6} {:>6} {:>7} {:>7} {:>5} | {:>5} {:>5} {:>5} {:>5} {:>4} {:>6} {:>6} {:>7} | {:>6} {:>5} {:>6} {:>6} {:>4} | {:>4}/{:<3} {:>4}/{:<3} {:>6} {:>6} | {:>10} {:>6} {:>6} | {:>5} {:>5} {:>4} {:>6} {:>5} {:>6}",
        "frame", "ants", "plnts", "bank", "edible", "worth(J)",
        "leafJ", "fruitJ", "littrJ", "seedJ", "crpsJ", "flowrJ", "otherJ", "flwrs",
        "born", "died", "strvd", "killd", "othr", "eats", "digs", "delivs",
        "roofed", "pit", "pack<", "pack^", "mnd", "bare", "band", "bare", "out", "pcIn", "pcOut",
        "wall", "awake", "sites",
        "ach/f", "req/f", "x", "dispHz", "debt", "skip"
    ) + &counter_header_group()
        + &phase_header_group()
}

/// **The counters group, appended after the perf group and before the
/// stopwatch's** (spec D16, D18, E19, F24): the dig funnel, the nest, the
/// seed loop, and the two death splits the row used to fold away.
///
/// Its own function and its own `|`-group for [`phase_header_group`]'s
/// reason: appended, it cannot move a column `examples/latecensus.rs` shares,
/// and that file is untouched. Before the stopwatch rather than after it so
/// the stopwatch stays the line's last group -- the one a reader lops off, and
/// the one `no_time_control_gives_dashes_not_zeros` used to read as "last".
///
/// | column | field |
/// |---|---|
/// | `dRoll` | `dig_rolls` |
/// | `dDown` | `digs_aimed_down` |
/// | `dDnX` | `digs_down_refused` |
/// | `dRfX` | `digs_refused_roof` |
/// | `atNest` | `at_nest_ticks` |
/// | `sCarr` | `seeds_carried` |
/// | `pipS` | `pips_set_on_soil` |
/// | `pipN` | `pips_set_on_nest` |
/// | `sLost` | `seeds_lost_no_room` |
/// | `ownK` | `kills_own_colony` (of `killd`) |
/// | `oldag` | `oldage_deaths` (of `othr`) |
///
/// Every one cumulative over the run, like `born`/`eats`/`digs`: a window is
/// the difference of two rows.
fn counter_header_group() -> String {
    format!(
        " | {:>7} {:>6} {:>5} {:>5} {:>8} {:>5} {:>5} {:>5} {:>5} {:>5} {:>5}",
        "dRoll", "dDown", "dDnX", "dRfX", "atNest", "sCarr", "pipS", "pipN", "sLost", "ownK", "oldag"
    )
}

/// The counters group's data, matching [`counter_header_group`].
fn counter_row_group(row: &ChronicleRow) -> String {
    format!(
        " | {:>7} {:>6} {:>5} {:>5} {:>8} {:>5} {:>5} {:>5} {:>5} {:>5} {:>5}",
        row.dig_rolls,
        row.digs_aimed_down,
        row.digs_down_refused,
        row.digs_refused_roof,
        row.at_nest_ticks,
        row.seeds_carried,
        row.pips_set_on_soil,
        row.pips_set_on_nest,
        row.seeds_lost_no_room,
        row.kills_own_colony,
        row.oldage_deaths
    )
}

/// The stopwatch's own column group, appended to [`header_line`].
///
/// **A separate function, not eight more arguments to that `format!`.** That
/// call is already thirty-nine positional arguments wide and its own doc says
/// the widths are load-bearing against `examples/latecensus.rs`; appending a
/// group cannot disturb a column that harness shares, and eight more
/// positions in the same call could. The group is also the part a reader is
/// most likely to want to lop off, and this is where the `PHASE_NAMES` table
/// is read rather than re-typed -- `ca_sweep` renamed there renames it here.
fn phase_header_group() -> String {
    let mut out = String::from(" | ticks");
    for name in crate::sim::frame::PHASE_NAMES {
        // Truncated to the width the numbers below need. `{:>8}` and three
        // decimals of a millisecond: a phase under a microsecond reads 0.000
        // and one at 20 ms still fits.
        out.push_str(&format!(" {:>8}", &name[..name.len().min(8)]));
    }
    out
}

/// One data row, in the same columns `header_line` names.
///
/// **The last six perf columns read `--` when `row.perf` is `None`** (no
/// `TimeControl` was available when the row was taken) rather than `0`,
/// which would read as "the box achieved zero ticks" -- a real and alarming
/// finding this is not. See [`PerfSample`]'s own doc for what each of the
/// six means and why `x` (the speed multiple) has to be read beside the
/// other five, never alone.
pub fn row_line(row: &ChronicleRow) -> String {
    let s = &row.sample;
    let dash = || "--".to_string();
    let (achf, reqf, mult, disp, debt, skip) = match &row.perf {
        Some(p) => (
            p.ticks_per_frame.to_string(),
            p.requested_ticks_per_frame.to_string(),
            p.speed_multiple.to_string(),
            p.display_hz.to_string(),
            p.debt_ticks.to_string(),
            p.draws_skipped.to_string(),
        ),
        None => (dash(), dash(), dash(), dash(), dash(), dash()),
    };
    format!(
        "{:>7} {:>5} {:>5} {:>5} {:>6} {:>9.0} | {:>8.0} {:>7.0} {:>7.0} {:>6.0} {:>6.0} {:>7.0} {:>7.0} {:>5} | {:>5} {:>5} {:>5} {:>5} {:>4} {:>6} {:>6} {:>7} | {:>6} {:>5} {:>6} {:>6} {:>4} | {:>4}/{:<3} {:>4}/{:<3} {:>6} {:>6} | {:>10} {:>6} {:>6} | {:>5} {:>5} {:>4} {:>6} {:>5} {:>6}",
        row.frame, s.ants, s.plants, s.seed_bank, s.edible, s.worth,
        s.leaf_j, s.fruit_j, s.litter_j, s.seed_j, s.corpse_j, s.flower_j, s.other_j, s.standing_flowers,
        row.births, row.deaths, row.starved, row.killed, row.other_deaths, row.eats, row.digs, row.deliveries,
        s.roofed, s.pit, s.packed_below, s.packed_above, s.mound_high,
        s.bare_in_band, s.band_cols, s.bare_outside, s.outside_cols, s.plant_cells_in_band, s.plant_cells_outside,
        row.wall_clock_secs, row.awake_chunks, row.active_sites,
        achf, reqf, mult, disp, debt, skip
    ) + &counter_row_group(row)
        + &phase_row_group(row.phases.as_ref())
}

/// The stopwatch's data columns, matching [`phase_header_group`].
///
/// **Per tick, not per row**, because a row covers `CHRONICLE_CENSUS_EVERY`
/// ticks by default (10,000) and a total over that is not a number anyone can
/// compare to anything. `ticks` is printed beside them so the division is
/// visible and a short window is not mistaken for a cheap one.
///
/// `--` for every column when the clock was off, **and also when it was on
/// but no tick has run since the last row** -- both mean "this row has no
/// measurement", and printing 0.000 for either would read as a phase that
/// costs nothing, which is a finding rather than an absence.
fn phase_row_group(phases: Option<&crate::sim::frame::PhaseTimes>) -> String {
    match phases.filter(|p| p.ticks > 0) {
        None => {
            let mut out = String::from(" |    --");
            for _ in crate::sim::frame::PHASE_NAMES {
                out.push_str(&format!(" {:>8}", "--"));
            }
            out
        }
        Some(p) => {
            let mut out = format!(" | {:>5}", p.ticks);
            for i in 0..crate::sim::frame::PHASE_NAMES.len() {
                out.push_str(&format!(" {:>8.3}", p.mean_ms(i)));
            }
            out
        }
    }
}

/// **What the owner watches, spelled out**: the nest band's bare-ground ratio
/// against the same ratio outside it, and the mound (packed cells standing
/// above the original surface, with its height) -- the two numbers
/// `header_line`/`row_line`'s dense columns bury in `bare/band` and
/// `pack^`/`mnd` and the owner's own complaint names directly ("they dig
/// large chambers ... which creates an area where plants don't grow").
pub fn row_addendum(row: &ChronicleRow) -> String {
    let s = &row.sample;
    let pct = |n: usize, d: usize| if d == 0 { 0.0 } else { 100.0 * n as f64 / d as f64 };
    format!(
        "        bare ground: nest band {:.0}% ({}/{}) vs outside {:.0}% ({}/{}) | mound (packed above surface): {} cells, {} rows high",
        pct(s.bare_in_band, s.band_cols), s.bare_in_band, s.band_cols,
        pct(s.bare_outside, s.outside_cols), s.bare_outside, s.outside_cols,
        s.packed_above, s.mound_high
    ) + &gut_addendum(row)
        + &phase_addendum(row)
}

/// **The gut the larder was priced at**, appended to the addendum's first
/// line -- spec F23. `worth(J)` and the per-kind joule columns are a price at
/// one gut, and since `ant_gut_bias` became a population mean that gut drifts
/// with heredity; printed, a step in the columns can be checked against a
/// step in the gut before anyone reads it as a change in the food.
fn gut_addendum(row: &ChronicleRow) -> String {
    format!(" | larder priced at gut {:+.3} (mean over living animals)", row.gut)
}

/// **The phase split as shares, in words, ranked** -- a second line under the
/// addendum, only when the stopwatch ran.
///
/// The column group in `row_line` is the machine-readable form, comparable
/// column for column with `examples/lab_cost.rs phases=1`. This is the form
/// the question is actually asked in. "Is the field a large part of the cost"
/// is a question about a *share*, and `CLAUDE.md`'s own rule for the
/// neighbouring case applies -- a worst-frame figure is worthless unless an
/// aggregate pins it -- so the tick total is printed beside the shares and the
/// two biggest phases are named rather than left to be found among eight
/// columns of milliseconds.
///
/// `awake` is repeated here on purpose. The field's cost tracks ant count
/// through the chunks the ants wake (`field::creature_wake_skip`), so a share
/// that moved and an awake count that did not means the cause is somewhere
/// else, and reading the two off different lines is how that gets missed.
fn phase_addendum(row: &ChronicleRow) -> String {
    let Some(p) = row.phases.filter(|p| p.ticks > 0) else { return String::new() };
    let total = p.total_ms();
    if total <= 0.0 {
        return String::new();
    }
    let mut ranked: Vec<(usize, f64)> =
        (0..crate::sim::frame::PHASE_NAMES.len()).map(|i| (i, p.ms[i])).collect();
    // `sort_by` on the share, descending, with the phase index as the
    // tie-break -- never a bare `sort_unstable_by`, so two phases at exactly
    // 0.000 ms always print in `PHASE_NAMES` order rather than in whatever
    // order the sort happened to leave them. `CLAUDE.md` keeps an entry on
    // tie-order for exactly this shape.
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("no NaN in a duration").then(a.0.cmp(&b.0)));
    let share = |i: usize| 100.0 * p.ms[i] / total;
    let names = crate::sim::frame::PHASE_NAMES;
    format!(
        "\n        tick {:.3} ms over {} tick(s), {} awake chunk(s) | field {:.0}% ({:.3} ms) | active_sites {:.0}% ({:.3} ms) -- every creature decision and every plant tick is in there, nowhere else | dearest: {} {:.0}%, {} {:.0}%",
        total / p.ticks as f64,
        p.ticks,
        row.awake_chunks,
        share(6),
        p.mean_ms(6),
        share(4),
        p.mean_ms(4),
        names[ranked[0].0],
        share(ranked[0].0),
        names[ranked[1].0],
        share(ranked[1].0),
    )
}

/// **The whole CENSUS section**, oldest row first: a header, then a row plus
/// its addendum per sample -- `Lab::write_chronicle`'s and
/// `examples/chronicle.rs`'s shared text, so a chronicle from a real session
/// and the headless export can never disagree about what a CENSUS section
/// looks like.
pub fn chronicle_section(rows: &[ChronicleRow]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    out.push_str("CENSUS\n");
    if rows.is_empty() {
        out.push_str("NO CENSUS SAMPLE HAS RUN YET.\n");
        return out;
    }
    let _ = writeln!(out, "{}", header_line());
    for row in rows {
        let _ = writeln!(out, "{}", row_line(row));
        let _ = writeln!(out, "{}", row_addendum(row));
    }
    out
}

/// **The `.census.csv` sidecar's text** (spec H30): one header line and one
/// line per row, every `ChronicleRow` field -- every `Sample` field, every
/// counter, the gut, the six perf fields and the eight phases (as mean ms per
/// tick, plus `phase_ticks`). Written beside the chronicle by
/// `Lab::write_chronicle`, same stem, same moment.
///
/// Why a sidecar at all: the 10-03 analysis needed `parse_chronicle.py` to
/// get the CENSUS table back out of fixed-width text, and fixed widths
/// truncate. This is the same rows with nothing lost to a column.
///
/// **The `Sample` columns are read off its `Debug` output, not listed by
/// hand.** `Sample` is a flat struct of numbers, so its derived `Debug` is
/// `Sample { name: value, .. }` and splits cleanly; a field added to it (the
/// nest lane's room columns are on their way) reaches this file with no edit
/// here, which a hand list would silently miss. Empty cells where the text
/// prints `--`.
pub fn census_csv(rows: &[ChronicleRow]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let sample_names: Vec<String> = debug_fields(&format!("{:?}", Sample::default())).into_iter().map(|(k, _)| k).collect();
    let mut head: Vec<String> = ["frame", "wall_clock_secs"].iter().map(|s| s.to_string()).collect();
    head.extend(sample_names.iter().cloned());
    head.extend(
        [
            "births", "deaths", "starved", "killed", "other_deaths", "oldage_deaths", "kills_own_colony", "eats", "digs", "deliveries",
            "dig_rolls", "digs_aimed_down", "digs_down_refused", "digs_refused_roof", "at_nest_ticks", "seeds_carried",
            "pips_set_on_soil", "pips_set_on_nest", "seeds_lost_no_room", "gut", "awake_chunks", "active_sites",
            "ticks_per_frame", "requested_ticks_per_frame", "speed_multiple", "display_hz", "debt_ticks", "draws_skipped",
            "phase_ticks",
        ]
        .iter()
        .map(|s| s.to_string()),
    );
    head.extend(crate::sim::frame::PHASE_NAMES.iter().map(|n| format!("{n}_ms")));
    let _ = writeln!(out, "{}", head.join(","));
    for row in rows {
        let mut v: Vec<String> = vec![row.frame.to_string(), row.wall_clock_secs.to_string()];
        v.extend(debug_fields(&format!("{:?}", row.sample)).into_iter().map(|(_, val)| val));
        v.extend(
            [
                row.births, row.deaths, row.starved, row.killed, row.other_deaths, row.oldage_deaths, row.kills_own_colony, row.eats,
                row.digs, row.deliveries, row.dig_rolls, row.digs_aimed_down, row.digs_down_refused, row.digs_refused_roof,
                row.at_nest_ticks, row.seeds_carried, row.pips_set_on_soil, row.pips_set_on_nest, row.seeds_lost_no_room,
            ]
            .iter()
            .map(|n| n.to_string()),
        );
        v.push(format!("{:.4}", row.gut));
        v.push(row.awake_chunks.to_string());
        v.push(row.active_sites.to_string());
        match &row.perf {
            Some(p) => v.extend(
                [
                    u64::from(p.ticks_per_frame),
                    u64::from(p.requested_ticks_per_frame),
                    u64::from(p.speed_multiple),
                    u64::from(p.display_hz),
                    u64::from(p.debt_ticks),
                    p.draws_skipped,
                ]
                .iter()
                .map(|n| n.to_string()),
            ),
            None => v.extend(std::iter::repeat_n(String::new(), 6)),
        }
        match row.phases.filter(|p| p.ticks > 0) {
            Some(p) => {
                v.push(p.ticks.to_string());
                v.extend((0..crate::sim::frame::PHASE_NAMES.len()).map(|i| format!("{:.4}", p.mean_ms(i))));
            }
            None => v.extend(std::iter::repeat_n(String::new(), 1 + crate::sim::frame::PHASE_NAMES.len())),
        }
        let _ = writeln!(out, "{}", v.join(","));
    }
    out
}

/// `Name { a: 1, b: 2.5 }` -> `[("a", "1"), ("b", "2.5")]`. Only for a flat
/// struct of numbers ([`census_csv`]'s `Sample`): a nested struct, a string
/// or a collection would split wrongly, which `census_csv_has_every_column`
/// checks by counting.
fn debug_fields(debug: &str) -> Vec<(String, String)> {
    let inner = debug.split_once('{').map(|(_, r)| r).unwrap_or("").trim_end().trim_end_matches('}');
    inner
        .split(',')
        .filter_map(|kv| kv.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::scene::LabBox;
    use crate::lab::time::TimeControl;
    use crate::sim::cell::Cell;

    /// A world just built, minimal but real -- enough to call
    /// `take_chronicle_row` without a scenario or a colony.
    fn tiny_world() -> World {
        LabBox { founders: 0, colonies: 0, width: 128, height: 96, ..LabBox::default() }.build()
    }

    /// **`ant_gut_bias` is the mean over living animals, not the first one
    /// found.** A founded colony with its guts set by hand to alternate two
    /// values: the first-found rule would return whichever the first slot
    /// held, the mean is the hand-computed average of all of them.
    /// Sensitivity: the two values differ, so the first-found rule this
    /// replaced fails the `assert` (put it back to watch it go red), and a
    /// colony of one would not have been a test at all -- hence the count
    /// check.
    #[test]
    fn gut_bias_is_the_population_mean() {
        let spec = LabBox { founders: 0, colonies: 0, ..LabBox::default() };
        let mut world = spec.build();
        // Through the `COLONY` tool's own call, which puts live animals down
        // at once -- the lab's own verb test founds one exactly this way.
        let placed = world.found_colony_of(spec.width / 2, spec.ground_y, "ant", 8);
        assert!(placed > 0, "found_colony_of placed nobody");
        let animals: Vec<_> = world
            .live_organism_ids()
            .into_iter()
            .filter(|id| world.organism(*id).is_some_and(|s| world.species.get(s.species).creature.is_some()))
            .collect();
        assert!(animals.len() >= 3, "the colony founded {} animals; the mean needs several", animals.len());
        let mut sum = 0.0f64;
        for (i, id) in animals.iter().enumerate() {
            let g = if i % 2 == 0 { 0.8f32 } else { -0.4f32 };
            world.organism_mut(*id).expect("live").traits[TRAIT_GUT_BIAS] = g;
            sum += f64::from(g);
        }
        let want = (sum / animals.len() as f64) as f32;
        let got = ant_gut_bias(&world);
        assert!((got - want).abs() < 1e-5, "gut {got} is not the mean {want} -- first slot alone would read 0.8");
        assert!((got - 0.8).abs() > 0.05, "the test cannot tell the mean from the first slot");
        assert_eq!(ant_gut_bias(&tiny_world()), 0.0, "no animal alive must price at 0.0 as before");
    }

    /// **`own_colony_kills` counts only kills by the victim's own colony.**
    /// Two `killed_by` entries on one ant group -- 5 by its own colony, 7 by
    /// another -- and one entry on a group of a different species name: only
    /// the 5 is the answer. Provable red by dropping either half of the
    /// species+colony match.
    #[test]
    fn own_colony_kills_reads_only_the_victims_own_colony() {
        let mut world = tiny_world();
        let ant = world.species.id_of("ant").expect("the lab ships an ant");
        let mut g = crate::sim::world::GroupDeaths { species: ant, colony: 3, by_cause: [0; organism::DEATH_CAUSES], killed_by: vec![(ant, 3, 5), (ant, 4, 7)] };
        world.group_deaths.push(g.clone());
        assert_eq!(own_colony_kills(&world, "ant"), 5);
        assert_eq!(own_colony_kills(&world, "no_such_species"), 0);
        g.colony = 4; // colony 4's own kill of colony 4 -- now (ant, 4, 7) is own
        world.group_deaths.push(g);
        assert_eq!(own_colony_kills(&world, "ant"), 12);
    }

    /// **Every census column reaches the `.census.csv` sidecar, and the
    /// header and each row have the same number of cells.** The `Sample`
    /// half is read off its `Debug`, so this counts it against `Sample`'s
    /// own `Debug` and names a few fields that must be there -- a split that
    /// went wrong (a nested struct, a comma in a value) shows as a count
    /// mismatch. Also checks the counters group and the addendum's gut reach
    /// the text.
    #[test]
    fn census_csv_has_every_column() {
        let world = tiny_world();
        let ids = Ids::resolve(&world);
        let mut row = take_chronicle_row(&world, &LabBox::default(), 0.25, &[], &ids, "ant", None);
        row.dig_rolls = 4321;
        row.kills_own_colony = 17;
        let csv = census_csv(&[row, row]);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3, "a header and two rows");
        let head: Vec<&str> = lines[0].split(',').collect();
        for l in &lines[1..] {
            assert_eq!(l.split(',').count(), head.len(), "row and header disagree on width: {l}");
        }
        let n_sample = format!("{:?}", Sample::default()).matches(": ").count();
        assert_eq!(debug_fields(&format!("{:?}", Sample::default())).len(), n_sample);
        for name in ["frame", "ants", "roofed_bodies", "corpses", "dig_rolls", "kills_own_colony", "gut", "draws_skipped", "ca_sweep_ms"] {
            assert!(head.contains(&name), "the sidecar has no {name} column: {}", lines[0]);
        }
        let at = head.iter().position(|h| *h == "dig_rolls").expect("dig_rolls column");
        assert_eq!(lines[1].split(',').nth(at), Some("4321"));
        assert!(row_line(&row).contains("4321"), "the counters group did not reach the text row");
        assert!(row_addendum(&row).contains("gut +0.250"), "the addendum does not name the gut: {}", row_addendum(&row));
    }

    /// **`time: None` produces `perf: None`, and `row_line` prints `--` for
    /// every perf column rather than `0`.** `examples/chronicle.rs`'s own
    /// case -- no `Lab`, no dial. Provable red by having `row_line` fall
    /// back to `p.unwrap_or_default()` instead of the dash: a reader would
    /// then see a real session's `0` (a genuinely stalled box) and this
    /// harness's "not sampled" as the identical string.
    #[test]
    fn no_time_control_gives_dashes_not_zeros() {
        let world = tiny_world();
        let ids = Ids::resolve(&world);
        let row = take_chronicle_row(&world, &LabBox::default(), 0.0, &[], &ids, "ant", None);
        assert!(row.perf.is_none(), "no TimeControl was given; perf must be None");
        // The `|`-group headed `ach/f req/f x dispHz debt skip` is exactly
        // the six perf columns -- checked in isolation so a real, non-zero
        // wall-clock timestamp or chunk count elsewhere in the line cannot
        // hide a `0` that should have been a dash.
        //
        // **Found by its header, not as "the last group".** This used to
        // take `rsplit('|').next()`, which stopped being the perf group the
        // day the stopwatch's group was appended after it -- and the
        // stopwatch's group prints `--` throughout whenever the clock is off
        // (every test process), so the assertion kept passing while reading
        // the wrong eight columns. Caught 2026-10-03 adding the counters
        // group; put the fault back (`p.unwrap_or_default()`) and this now
        // goes red, which the old form did not.
        let line = row_line(&row);
        let perf_at = header_line().split('|').position(|g| g.contains("ach/f")).expect("the header names its perf group");
        let perf_columns = line.split('|').nth(perf_at).expect("row_line has as many groups as header_line");
        assert_eq!(perf_columns.split_whitespace().count(), 6, "the perf group is six columns, got {perf_columns:?}");
        assert!(
            perf_columns.split_whitespace().all(|field| field == "--"),
            "expected every perf column to read '--' with no TimeControl, got: {perf_columns:?}"
        );
    }

    /// **`time: Some(..)` fills every perf column from `TimeControl`'s own
    /// reads, at the exact values it reports** -- the one-function
    /// guarantee `take_chronicle_row`'s own doc makes (a session's file and
    /// a headless run can never implement this differently, because there
    /// is only the one implementation). Provable red by hand-computing any
    /// one of the five checked values differently from the `TimeControl`
    /// call it is supposed to mirror.
    #[test]
    fn a_real_time_control_fills_every_perf_column() {
        let world = tiny_world();
        let ids = Ids::resolve(&world);
        let mut t = TimeControl::new();
        t.set_preset(2); // a real, non-default multiplier and display rate
        let row = take_chronicle_row(&world, &LabBox::default(), 0.0, &[], &ids, "ant", Some(&t));
        let perf = row.perf.expect("a real TimeControl was given; perf must be Some");
        assert_eq!(perf.speed_multiple, t.multiple());
        assert_eq!(perf.display_hz, t.display_hz());
        assert_eq!(perf.ticks_per_frame, t.ticks_per_frame());
        assert_eq!(perf.requested_ticks_per_frame, t.requested_ticks_per_frame());
        assert_eq!(perf.debt_ticks, t.owed_ticks());
        assert_eq!(perf.draws_skipped, t.draws_skipped());
        assert!(row_line(&row).contains(&t.multiple().to_string()), "the speed multiple did not reach the printed row");
    }

    /// **`awake_chunks`/`active_sites` are sampled regardless of `time`** --
    /// unlike the perf columns, these come straight off `world` and have no
    /// reason to be `None` for a headless harness. A fresh, empty world has
    /// nothing awake and nothing active, which doubles as the positive
    /// control for `World::active_chunk_count`/`active_site_count` reading
    /// zero on a box with nothing in it.
    #[test]
    fn chunk_and_site_counts_need_no_time_control() {
        let world = tiny_world();
        let ids = Ids::resolve(&world);
        let row = take_chronicle_row(&world, &LabBox::default(), 0.0, &[], &ids, "ant", None);
        assert_eq!(row.awake_chunks, world.active_chunk_count());
        assert_eq!(row.active_sites, world.active_site_count());
    }

    // ---------------------------------------------------------------- the
    // five columns the 560,000-frame playtest found dead, each against a
    // hand-placed feature of known size and then against its removal
    // ------------------------------------------------------------------
    //
    // **Both halves, deliberately.** `CLAUDE.md`: *before you cite a guard's
    // green as evidence, put the fault it is named for back and watch it go
    // red* -- and a column that reads 0 everywhere passes any test that only
    // checks it does not crash. Every case below carves a feature, asserts
    // the exact count, then takes the feature away and asserts the column
    // returns to zero. A column that reports the chamber and goes on
    // reporting it once the chamber is gone is not repaired, it is
    // differently broken.

    /// A bare bed with no founders and no colony, its room datum frozen the
    /// way `World::begin_step` freezes it on the first simulated frame.
    /// 256 wide so the whole box is inside one nest's band.
    fn bare_bed() -> (World, LabBox, Ids) {
        let spec = LabBox { founders: 0, colonies: 0, width: 256, height: 320, ..LabBox::default() };
        let mut world = spec.build();
        world.freeze_room_datum();
        let ids = Ids::resolve(&world);
        (world, spec, ids)
    }

    /// `census` at the bed's own centre column as the only nest.
    fn at(world: &World, spec: &LabBox, ids: &Ids) -> Sample {
        census(world, spec, 0.0, &[spec.width / 2], ids)
    }

    /// Put back exactly the soil a carve removed, so the negative half of
    /// each control restores the bed rather than approximating it.
    fn fill(world: &mut World, x: i32, y: i32) {
        let soil = world.materials.id_of("soil").expect("soil is compiled in");
        world.set(x, y, Cell::new(soil, 0).with_aux(crate::sim::material::SOIL_FIELD_CAPACITY));
    }

    /// **A gallery with an ant standing in it is still a gallery.**
    ///
    /// The defect this module shipped until 2026-09-19: `roofed` counts
    /// materially EMPTY cells, so every occupied cell of a nest was counted
    /// as nothing and a dense colony's workings read about a third of their
    /// true size. Measured in `examples/digbox`: roofed 157 + open 165 +
    /// **bodies 692** = 1,014 against 941 cells hauled above the surface --
    /// a conservation identity that fails by **619 cells** on `roofed`
    /// alone.
    ///
    /// **Both halves, and the first half is the one that matters.** Ten
    /// tests in this module stayed green straight through the defect,
    /// because `bare_bed` founds no colony and the one case that does place
    /// ants puts them *above* the surface datum and asserts only band
    /// columns. So a guard that merely carves a chamber cannot see this at
    /// all; the ant has to be standing in it. Provable red by reverting
    /// `census`'s `MaterialKind::Creature` arm -- `room_total` then drops by
    /// exactly the cells the body occupies.
    ///
    /// The cell is placed by hand rather than by founding a colony: this is
    /// a test of the *counting rule*, and where a founded ant happens to
    /// walk is not something a census guard should depend on.
    #[test]
    fn an_ant_standing_in_a_chamber_does_not_shrink_the_chamber() {
        let (mut world, spec, ids) = bare_bed();
        let ant = world.materials.id_of("ant").expect("the ant material is compiled in");
        assert_eq!(world.materials.kind(ant), MaterialKind::Creature, "this guard is about creature-kind cells");

        let (cx, cy) = (spec.width / 2, spec.ground_y + 10);
        for dy in 0..3 {
            for dx in 0..3 {
                world.set(cx + dx, cy + dy, Cell::EMPTY);
            }
        }
        let empty = at(&world, &spec, &ids);
        assert_eq!(empty.roofed, 9, "a 3x3 chamber ten rows under the surface is nine cells of room");
        assert_eq!(empty.roofed_bodies, 0, "nobody is standing in it yet");
        assert_eq!(empty.room_total(), 9, "an empty chamber is nine cells of room");

        // One ant, two cells -- `ant.ron` authors `body: Chain(2)`, so
        // asserting against a hardcoded 8 would be wrong for the shipped
        // body and would silently stop testing anything if the body widened.
        world.set(cx, cy, Cell::new(ant, 0));
        world.set(cx + 1, cy, Cell::new(ant, 0));

        let occupied = at(&world, &spec, &ids);
        assert_eq!(occupied.roofed, 7, "two of the nine cells are now occupied, so the EMPTY count falls -- this is the number that used to be read alone");
        assert_eq!(occupied.roofed_bodies, 2, "and the two occupied cells are counted as the room they are");
        assert_eq!(
            occupied.room_total(),
            empty.room_total(),
            "the chamber did not get smaller because somebody walked into it -- room_total must be invariant under occupancy"
        );

        // The negative half: take the ants out and the split returns.
        world.set(cx, cy, Cell::EMPTY);
        world.set(cx + 1, cy, Cell::EMPTY);
        let vacated = at(&world, &spec, &ids);
        assert_eq!(vacated.roofed_bodies, 0, "the bodies column is still reporting ants that have gone");
        assert_eq!(vacated.roofed, 9, "and the empty count came back");
    }

    /// **An ant standing in an open pit is not in a chamber**, which is the
    /// split `digbox`'s own undivided `bodies` total cannot make.
    ///
    /// `CLAUDE.md`'s standing metric trap -- *a hole open to the sky is not
    /// a room* -- applies to the occupant exactly as it applies to the void,
    /// and a pair that only ever moved together would be one column shipped
    /// twice.
    #[test]
    fn an_ant_in_a_pit_is_counted_as_pit_and_not_as_chamber() {
        let (mut world, spec, ids) = bare_bed();
        let ant = world.materials.id_of("ant").expect("the ant material is compiled in");
        let cx = spec.width / 2;
        // A shaft open to the sky: every cell from the surface down.
        for dy in 0..5 {
            world.set(cx, spec.ground_y + dy, Cell::EMPTY);
        }
        world.set(cx, spec.ground_y + 3, Cell::new(ant, 0));

        let s = at(&world, &spec, &ids);
        assert_eq!(s.pit_bodies, 1, "the ant is in a hole open to the sky");
        assert_eq!(s.roofed_bodies, 0, "and that hole is not a chamber");
        assert_eq!(s.room_total(), 5, "five cells of shaft, one of them occupied");

        // Roof the mouth and the same ant becomes a chamber occupant --
        // the two columns must move in opposite directions, not together.
        fill(&mut world, cx, spec.ground_y);
        let roofed = at(&world, &spec, &ids);
        assert_eq!(roofed.pit_bodies, 0, "the shaft is covered now");
        assert_eq!(roofed.roofed_bodies, 1, "so the ant is standing in a chamber");
    }

    /// **`roofed`: a chamber under intact soil is counted, and filling it in
    /// takes the count back to zero.**
    ///
    /// The column `CLAUDE.md`'s metric-trap section names as *the* one to
    /// read for excavation -- *what a player calls a nest is roofed void* --
    /// and the one that read 0 in all 56 samples of a session with 356,688
    /// digs in it.
    #[test]
    fn a_roofed_chamber_is_counted_and_filling_it_in_uncounts_it() {
        let (mut world, spec, ids) = bare_bed();
        assert_eq!(at(&world, &spec, &ids).roofed, 0, "nobody has dug this bed yet");
        let (cx, cy) = (spec.width / 2, spec.ground_y + 10);
        for dy in 0..3 {
            for dx in 0..3 {
                world.set(cx + dx, cy + dy, Cell::EMPTY);
            }
        }
        assert_eq!(at(&world, &spec, &ids).roofed, 9, "a 3x3 chamber ten rows under the surface is nine cells of room");
        for dy in 0..3 {
            for dx in 0..3 {
                fill(&mut world, cx + dx, cy + dy);
            }
        }
        assert_eq!(at(&world, &spec, &ids).roofed, 0, "the chamber was filled in and the column is still reporting it");
    }

    /// **`pit` and `roofed` separate a hole open to the sky from a room**,
    /// which is the standing metric trap this repo has already paid for once
    /// (`CLAUDE.md`: *a hole open to the sky is not a room*). Roofing the
    /// shaft's mouth must move its cells from one column to the other
    /// without changing the total -- a pair that only ever moved together
    /// would be one column shipped twice.
    #[test]
    fn a_shaft_open_to_the_sky_is_a_pit_until_something_is_put_over_it() {
        let (mut world, spec, ids) = bare_bed();
        let cx = spec.width / 2;
        for dy in 0..5 {
            world.set(cx, spec.ground_y + dy, Cell::EMPTY);
        }
        let open = at(&world, &spec, &ids);
        assert_eq!((open.pit, open.roofed), (5, 0), "a five-deep shaft cut from the surface is five cells of pit and no room");
        // Roof it: one cell of soil laid over the mouth, nothing else moved.
        fill(&mut world, cx, spec.ground_y);
        let roofed = at(&world, &spec, &ids);
        assert_eq!((roofed.pit, roofed.roofed), (0, 4), "roofing the mouth turns the shaft below it into room");
        // ...and take the roof off again. Both directions, so neither column
        // can be a constant.
        world.set(cx, spec.ground_y, Cell::EMPTY);
        let reopened = at(&world, &spec, &ids);
        assert_eq!((reopened.pit, reopened.roofed), (5, 0), "the roof came off and the shaft is a pit again");
        for dy in 0..5 {
            fill(&mut world, cx, spec.ground_y + dy);
        }
        let filled = at(&world, &spec, &ids);
        assert_eq!((filled.pit, filled.roofed), (0, 0), "the shaft was filled in and the columns are still reporting it");
    }

    /// **`pack<` counts worked soil below the original surface and `pack^`
    /// counts it above** -- the gallery lining against the mound, which is
    /// the whole reason they are two columns. `pack^` is the one structural
    /// column the playtest found alive, so it is the control here: the
    /// lining must not reach it.
    #[test]
    fn worked_soil_is_counted_on_the_side_of_the_surface_it_is_on() {
        let (mut world, spec, ids) = bare_bed();
        let packed = ids.packed.expect("packedsoil is registered");
        let cx = spec.width / 2;
        let bare = at(&world, &spec, &ids);
        assert_eq!((bare.packed_below, bare.packed_above), (0, 0), "nothing has been worked in this bed");
        // Two cells of lining twenty rows down, and one of spoil standing on
        // the surface.
        world.set(cx, spec.ground_y + 20, Cell::new(packed, 0));
        world.set(cx + 1, spec.ground_y + 20, Cell::new(packed, 0));
        world.set(cx + 4, spec.ground_y - 1, Cell::new(packed, 0));
        let worked = at(&world, &spec, &ids);
        assert_eq!((worked.packed_below, worked.packed_above), (2, 1), "two lining cells below the surface and one of spoil above it");
        // Take the lining out and leave the spoil: `pack<` must fall to zero
        // while `pack^` does not move. A single scrape that cleared both
        // would pass a test that only looked at the one it was named for.
        fill(&mut world, cx, spec.ground_y + 20);
        fill(&mut world, cx + 1, spec.ground_y + 20);
        let scraped = at(&world, &spec, &ids);
        assert_eq!((scraped.packed_below, scraped.packed_above), (0, 1), "the lining is gone and the spoil on the surface is not");
        world.set(cx + 4, spec.ground_y - 1, Cell::EMPTY);
        let cleared = at(&world, &spec, &ids);
        assert_eq!((cleared.packed_below, cleared.packed_above), (0, 0), "the spoil was cleared and the column is still reporting it");
    }

    /// **`mnd` is the height of the highest ground standing above the
    /// original surface, and a flat bed has none** -- the column that read
    /// exactly `MOUND_REACH` at zero mound cells for 560,000 frames, which
    /// is a number that cannot move.
    #[test]
    fn a_mound_of_known_height_reads_that_height_and_a_flat_bed_reads_zero() {
        let (mut world, spec, ids) = bare_bed();
        assert_eq!(at(&world, &spec, &ids).mound_high, 0, "flat ground stands zero rows above itself");
        let packed = ids.packed.expect("packedsoil is registered");
        let cx = spec.width / 2;
        for dy in 1..=3 {
            world.set(cx, spec.ground_y - dy, Cell::new(packed, 0));
        }
        let heaped = at(&world, &spec, &ids);
        assert_eq!((heaped.mound, heaped.mound_high), (3, 3), "a three-cell heap on the surface is three cells, three rows high");
        // One more course, so the column is measured moving rather than
        // merely being non-zero once.
        world.set(cx, spec.ground_y - 4, Cell::new(packed, 0));
        assert_eq!(at(&world, &spec, &ids).mound_high, 4, "another course on the heap is another row of height");
        for dy in 1..=4 {
            world.set(cx, spec.ground_y - dy, Cell::EMPTY);
        }
        let levelled = at(&world, &spec, &ids);
        assert_eq!((levelled.mound, levelled.mound_high), (0, 0), "the heap was levelled and the column is still reporting it");
    }

    /// **The nest band exists because the world holds a nest, not because
    /// the bed spec asked for one.**
    ///
    /// The playtest's own shape: `FOUNDERS 0  COLONIES 0` in the header and
    /// five colonies founded by hand, so every spec-derived source of nest
    /// columns was empty and the band read `0/0` in all 56 samples. Founded
    /// through `World::found_colony_of`, which is the same call the colony
    /// key makes.
    #[test]
    fn a_colony_founded_by_hand_gives_the_band_columns_to_measure() {
        let (mut world, spec, _ids) = bare_bed();
        assert_eq!(spec.colonies, 0, "this bed is built with no colony, which is the case under test");
        let empty = nest_columns(&world, &spec, None);
        assert!(empty.is_empty(), "no nest has been founded yet: {empty:?}");
        let ids = Ids::resolve(&world);
        let before = census(&world, &spec, 0.0, &empty, &ids);
        assert_eq!((before.band_cols, before.bare_in_band), (0, 0), "no nest, no band");
        assert_eq!(before.outside_cols, spec.width as usize, "with no band every column is outside it");

        let cx = spec.width / 2;
        let founded = world.found_colony_of(cx, spec.ground_y - 2, &spec.colony_species, 8);
        assert!(founded > 0, "the hand-founded colony placed no ants, so this control cannot say anything");
        let cols = nest_columns(&world, &spec, None);
        assert!(cols.contains(&cx), "founding a colony at {cx} must put {cx} in the nest columns, got {cols:?}");
        let after = census(&world, &spec, 0.0, &cols, &ids);
        assert!(after.band_cols > 0, "the band is still empty with a colony standing in the bed");
        assert_eq!(
            after.band_cols + after.outside_cols,
            spec.width as usize,
            "every column is either in the band or outside it, exactly once"
        );
    }

    /// **The whole defect, end to end: a spec that has drifted from the
    /// world it is censusing.**
    ///
    /// This is what the owner's session did. `params::write_bed` moves the
    /// spec the moment a bed row is nudged and the world is only reshaped on
    /// REBUILD -- `raising_the_width_and_rebuilding_gives_a_wider_world`
    /// asserts that separation deliberately. He raised the box height during
    /// setup, `ground_y` rode the height, and the census then measured a
    /// surface 96 rows below the one the world had: down in the stone base,
    /// where there is no void to find and no worked soil to count.
    ///
    /// Provable red by hand: put `spec.ground_y` back in place of
    /// `surface_of` and every assertion below reads 0 (or, for `mnd`,
    /// exactly `MOUND_REACH`), which is the playtest log's five constants.
    #[test]
    fn a_spec_whose_ground_has_drifted_from_the_world_still_censuses_the_world() {
        let (mut world, built, ids) = bare_bed();
        let (cx, cy) = (built.width / 2, built.ground_y + 10);
        for dy in 0..3 {
            for dx in 0..3 {
                world.set(cx + dx, cy + dy, Cell::EMPTY);
            }
        }
        for dy in 0..5 {
            world.set(cx + 20, built.ground_y + dy, Cell::EMPTY);
        }
        let packed = ids.packed.expect("packedsoil is registered");
        world.set(cx + 1, built.ground_y + 20, Cell::new(packed, 0));
        world.set(cx + 30, built.ground_y - 1, Cell::new(packed, 0));
        let truth = at(&world, &built, &ids);
        assert_eq!(
            (truth.roofed, truth.pit, truth.packed_below, truth.packed_above, truth.mound_high),
            (9, 5, 1, 1, 1),
            "the control itself is wrong: the carved bed does not read what was carved into it"
        );

        // Now the drift, exactly as `params::write_bed`'s `"height"` arm
        // produces it: height 320 -> 512 scales `ground_y` 160 -> 256, and
        // nobody pressed REBUILD.
        let mut drifted = built.clone();
        assert!(crate::lab::params::write_bed(&mut drifted, "height", 512.0));
        assert_eq!(
            (drifted.height, drifted.ground_y),
            (512, 256),
            "this control depends on `ground_y` riding the height; if that stopped, the drift under test is a different one"
        );
        assert_ne!(
            drifted.ground_y,
            world.room_surface_at(cx).expect("the datum was frozen"),
            "the spec and the world must actually disagree, or this test is measuring nothing"
        );
        let s = census(&world, &drifted, 0.0, &[cx], &ids);
        assert_eq!(
            (s.roofed, s.pit, s.packed_below, s.packed_above, s.mound_high),
            (9, 5, 1, 1, 1),
            "the census followed the spec instead of the world"
        );
        // ...and the columns walked are the world's too, not the spec's.
        assert_eq!(
            s.band_cols + s.outside_cols,
            built.width as usize,
            "the census walked {} columns for a {}-wide world",
            s.band_cols + s.outside_cols,
            built.width
        );
    }

    /// **The fallback, stated rather than assumed**: a world that has never
    /// been stepped has no frozen datum, and the census falls back to
    /// `LabBox::ground_y`. Every production path steps the world
    /// (`World::begin_step` freezes it on the first tick), so this covers
    /// the harnesses that drive `census` directly.
    #[test]
    fn an_unstepped_world_falls_back_to_the_specs_own_ground() {
        let spec = LabBox { founders: 0, colonies: 0, width: 256, height: 320, ..LabBox::default() };
        let mut world = spec.build();
        assert_eq!(world.room_surface_at(spec.width / 2), None, "nothing has frozen this world's datum yet");
        let ids = Ids::resolve(&world);
        let (cx, cy) = (spec.width / 2, spec.ground_y + 10);
        for dy in 0..3 {
            for dx in 0..3 {
                world.set(cx + dx, cy + dy, Cell::EMPTY);
            }
        }
        assert_eq!(at(&world, &spec, &ids).roofed, 9, "the spec's own ground is the fallback and it agrees with this bed");
    }
}
