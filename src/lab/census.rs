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
    pub roofed: usize,
    /// Void below the original surface open to the sky -- a pit.
    pub pit: usize,
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

/// The gut bias of the first ant `census` would find -- founder or
/// timeline-delivered, `live_organism_ids`'s own order. `diet_yield` needs a
/// gut to price a cell against, and every ant so far seen on the bed shares
/// one species' trait, so any live ant answers it.
pub fn ant_gut_bias(world: &World) -> f32 {
    world
        .live_organism_ids()
        .iter()
        .filter_map(|id| world.organism(*id))
        .find(|s| world.species.get(s.species).creature.is_some())
        .map(|s| s.traits[TRAIT_GUT_BIAS])
        .unwrap_or(0.0)
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
    )
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
    )
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
        // The last `|`-group is exactly the six perf columns (`header_line`'s
        // own layout: `... | wall awake sites | ach/f req/f x dispHz debt
        // skip`) -- checked in isolation so a real, non-zero wall-clock
        // timestamp or chunk count earlier in the line cannot hide a `0`
        // that should have been a dash.
        let line = row_line(&row);
        let perf_columns = line.rsplit('|').next().expect("row_line always has at least one `|`");
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
