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

fn is_waiting_seed(world: &World, id: u16, state: &organism::OrganismState) -> bool {
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
    let mut has_plant = vec![false; spec.width as usize];
    let mut plant_cells = vec![0usize; spec.width as usize];
    for x in 0..spec.width {
        let mut covered = false;
        for y in 0..spec.height {
            let cell = world.get(x, y);
            let kind = world.materials.kind(cell.material);
            let is_ground = cell.material != material::EMPTY
                && matches!(kind, MaterialKind::Powder | MaterialKind::Solid)
                && cell.organism_id() == 0;
            if kind == MaterialKind::Plant || (cell.organism_id() != 0 && world.organism(cell.organism_id()).is_some_and(|st| world.species.get(st.species).creature.is_none())) {
                has_plant[x as usize] = true;
                plant_cells[x as usize] += 1;
            }
            // **Cover is ground near the surface, not the lid.** The lab box
            // is sealed, so row 0 is a wall in every column and a plain
            // "anything solid above" read the whole bed as roofed (selftest:
            // a shaft open to the sky counted as a chamber). A mound is a few
            // rows; the lid is 160 up.
            if is_ground && y >= spec.ground_y - MOUND_REACH {
                covered = true;
                if y < spec.ground_y && ids.ground.contains(&cell.material) {
                    s.mound += 1;
                    s.mound_high = s.mound_high.max(spec.ground_y - y);
                }
                if ids.packed_any.contains(&cell.material) {
                    if y < spec.ground_y {
                        s.packed_above += 1;
                    } else {
                        s.packed_below += 1;
                    }
                }
            } else if y >= spec.ground_y && world.is_empty(x, y) {
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
    for x in 0..spec.width {
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
        let Some(top) = ((spec.ground_y - MOUND_REACH).max(0)..spec.height).find(|&y| {
            let cell = world.get(x, y);
            ids.ground.contains(&cell.material) && cell.organism_id() == 0
        }) else {
            continue;
        };
        if top >= spec.ground_y {
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
    for x in 0..spec.width {
        let d = nest_cols.iter().map(|c| (c - x).abs()).min().unwrap_or(i32::MAX);
        let bare = !has_plant[x as usize];
        if d <= BAND {
            s.band_cols += 1;
            s.plant_cells_in_band += plant_cells[x as usize];
            if bare {
                s.bare_in_band += 1;
            }
        } else {
            s.outside_cols += 1;
            s.plant_cells_outside += plant_cells[x as usize];
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

/// Columns any ant colony is founded at over this bed's life: built at bed
/// construction (`LabBox::colony_columns`) plus, if a scenario is attached,
/// whatever its own placements or timeline add later (`played_bed`'s own
/// shape -- `examples/latecensus.rs` read only the second half before this
/// moved; a hand-built lab box has no scenario and reads only the first).
/// A nest does not move once founded, so this is cheap to recompute per
/// sample rather than caching it.
pub fn nest_columns(spec: &LabBox, scenario: Option<&Scenario>) -> Vec<i32> {
    let mut v = spec.colony_columns();
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
}
