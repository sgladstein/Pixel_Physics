//! **Is the colony starving because the bed's food is gone, or because it
//! never gets to it?** — the separator `open-bugs-handoff.md` §Z6 asks for.
//!
//! §Z6 censused nine 300,000-frame runs and found every shipped bed's colony
//! dead of starvation with the stand still standing, and said in as many
//! words that its own census could not tell the two apart:
//!
//! > Whether the colony *overgrazes* (eats the stand faster than it regrows
//! > while it lives) or *cannot reach* what is there ... are different bugs
//! > with different fixes, and this census cannot tell them apart.
//!
//! The three instruments it named cannot either, and the reason is the
//! scene rather than the counters: `forage_probe` builds a hand-made bank
//! with a food pile at a fixed gap, `stamp_probe` runs outdoor terrain, and
//! `labnest`'s columns are about the *nest* — it reports `roofed`, `packed`
//! and `buried` and has no food census at all. Nothing measures the shipped
//! bed's larder.
//!
//! # What separates them, and why it is a *cumulative* mask
//!
//! A standing count of food cannot answer this. Food is standing in *both*
//! worlds — an overgrazed bed still holds wood, and an unreachable larder is
//! by definition untouched — so `edible` on its own is the same number for
//! opposite findings. What differs is **whether the colony was ever there**:
//!
//! * **Overgrazing** — the ants walk the bed, eat what they find, and the
//!   standing food falls below the unfed control. `visited` covers the bed;
//!   `edible` is far under `colonies=0`.
//! * **Cannot reach** — the standing food is at or near the unfed control's
//!   level, and it is standing in columns no ant ever entered, or at heights
//!   no ant ever climbed to.
//!
//! So the column that decides it is `unvisited` — edible cells standing in a
//! column the colony has *never* occupied over the whole run — beside
//! `eaten` and beside the same bed at `colonies=0`.
//!
//! **The mask is over columns, not cells, and that is deliberate.** An ant
//! reaches into its head's 8-neighbourhood from the surface it walks on, and
//! the surface drifts as litter piles; a cell-exact mask would score a leaf
//! one row above a track as unreached, which is a statement about the
//! bookkeeping and not about the animal. A column mask is the *generous*
//! reading — it credits the colony with everything in any column it ever
//! walked — so a large `unvisited` is a finding that had to survive the
//! reading most likely to erase it.
//!
//! # Controls, in the binary
//!
//! * `colonies=0` — the unfed stand. `eaten` must be 0 and `edible` is the
//!   ceiling every fed run is read against. This is the specificity half.
//! * `handout=N` — a food cell dropped on the colony's doorstep every N
//!   frames, `windfall_probe`'s positive control. Supply without travel: if
//!   the colony lives with it and dies without it, the failure is delivery.
//! * `control=selftest` — the sensitivity half, and the one this file would
//!   be untrustworthy without. It plants a known count of known food at a
//!   known height and a known distance and asserts every band reports it,
//!   because a census reading zero because nothing is there and one reading
//!   zero because it is blind are the same line of output.
//!
//! ```text
//! cargo run --release --example labforage -- frames=300000 seed=1
//! cargo run --release --example labforage -- frames=300000 seed=1 colonies=0
//! cargo run --release --example labforage -- frames=300000 founders=256 colonies=3
//! cargo run --release --example labforage -- control=selftest
//! ```

use pixel_physics::lab::scenario::{Placement, Scenario};
use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::brain;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::creature::{diet_yield, EAT_YIELD_THRESHOLD};
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::field;
use pixel_physics::sim::frame;
use pixel_physics::sim::material::MaterialId;
use pixel_physics::sim::organism::TRAIT_GUT_BIAS;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::pheromone::Channel;
use pixel_physics::sim::plant;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// `hidden=<Input>:<unit>:<weight>[,...]` -- see the call site, which is
/// where the reason it must run before founding lives.
fn hidden_rider() -> Vec<(brain::BrainInput, usize, f32)> {
    let Some(spec) = arg::<String>("hidden") else { return Vec::new() };
    spec.split(',')
        .map(|entry| {
            let bits: Vec<&str> = entry.split(':').collect();
            assert_eq!(bits.len(), 3, "hidden entry {entry:?} wants Input:unit:weight, e.g. hidden=PheroBAlong:2:6.0");
            let input = brain::INPUTS
                .iter()
                .copied()
                .find(|i| brain::INPUT_NAMES[*i as usize].eq_ignore_ascii_case(bits[0]))
                .unwrap_or_else(|| panic!("unknown input {:?}; known: {:?}", bits[0], brain::INPUT_NAMES));
            let unit: usize = bits[1].parse().unwrap_or_else(|_| panic!("hidden unit {:?} does not parse", bits[1]));
            assert!(unit < brain::BRAIN_HIDDEN, "hidden unit {unit} is past BRAIN_HIDDEN ({})", brain::BRAIN_HIDDEN);
            let w: f32 = bits[2].parse().unwrap_or_else(|_| panic!("hidden weight {:?} does not parse", bits[2]));
            (input, unit, w)
        })
        .collect()
}

/// `wire=<Input>:<Output>:<weight>[,...]` -- the input-to-**output** half of
/// the rider above, `labstats`' and `creature_arena`'s own syntax, added here
/// for P2's control arm.
///
/// **It exists because the alternative is the `include_str!` trap.** P2 needs
/// the shipped hopper raced at `(Bias, Impulse, 2.0)` against the flitter,
/// and `hopper.ron` ships 0.5. A species *file* copy carrying the other
/// weight is not "the same binary two arms": species files are
/// `include_str!`-embedded, so the arm is a different build, and this repo
/// has three bit-identical sweeps on record from exactly that. `instruments.
/// md` names this case by name -- *"The hopper's entire reason to exist is a
/// single instinct row, `(Bias, Impulse, 2.0)`, and there was no way to move
/// it without editing `hopper.ron` and rebuilding between arms"* -- and the
/// fix it records is this knob, which `labstats` and `creature_arena`
/// already carry. This is the third harness, deliberately with the same
/// spelling so the three race one set of numbers rather than three
/// transcriptions.
fn wire_rider() -> Vec<(brain::BrainInput, brain::BrainOutput, f32)> {
    let Some(spec) = arg::<String>("wire") else { return Vec::new() };
    spec.split(',')
        .map(|entry| {
            let bits: Vec<&str> = entry.split(':').collect();
            assert_eq!(bits.len(), 3, "wire entry {entry:?} wants Input:Output:weight, e.g. wire=Bias:Impulse:2.0");
            let input = brain::INPUTS
                .iter()
                .copied()
                .find(|i| brain::INPUT_NAMES[*i as usize].eq_ignore_ascii_case(bits[0]))
                .unwrap_or_else(|| panic!("unknown input {:?}; known: {:?}", bits[0], brain::INPUT_NAMES));
            let output = brain::OUTPUTS
                .iter()
                .copied()
                .find(|o| brain::OUTPUT_NAMES[*o as usize].eq_ignore_ascii_case(bits[1]))
                .unwrap_or_else(|| panic!("unknown output {:?}; known: {:?}", bits[1], brain::OUTPUT_NAMES));
            let w: f32 = bits[2].parse().unwrap_or_else(|_| panic!("wire weight {:?} does not parse", bits[2]));
            (input, output, w)
        })
        .collect()
}

/// The gut bias off a live founder, never off the species table -- the run
/// has to be measuring the gut it says it is. `0.0` (neutral) before any
/// ant exists to read one off, which only happens between the bed being
/// built and `ants_at` arriving when `ants_at > 0`.
fn ant_gut_bias(world: &World) -> f32 {
    world
        .live_organism_ids()
        .iter()
        .filter_map(|id| world.organism(*id))
        .find(|s| world.species.get(s.species).creature.is_some())
        .map(|s| s.traits[TRAIT_GUT_BIAS])
        .unwrap_or(0.0)
}

/// How far above the soil surface a cell still counts as **on the floor**.
///
/// Same value and same reasoning as `windfall_probe`'s: an ant is a two-cell
/// chain standing on the ground reaching into its head's 8-neighbourhood, so
/// a couple of rows above the surface is food it takes without climbing, and
/// litter drifts the walking surface upward over a run. Generous on purpose —
/// the finding this harness exists to make is about food out of reach, and a
/// generous floor band makes that finding harder to get rather than easier.
const FLOOR_BAND: i32 = 3;

/// Above this many rows off the soil, food is up a stem rather than on the
/// ground. Between the two bands is what an ant reaches by climbing a little.
const LOW_BAND: i32 = 16;

/// Distance bands from the nearest nest column, in cells. `larder_probe`'s
/// shape: a quantity present in the world and a quantity concentrated where
/// the animals are are different findings, and only a banded census separates
/// them.
const DIST_BANDS: [i32; 4] = [16, 48, 128, i32::MAX];

#[derive(Default, Clone, Copy)]
struct Sample {
    /// Standing cells this gut would eat, by the mouth's own predicate.
    edible: usize,
    /// ...of those, within `FLOOR_BAND` rows of the soil surface.
    floor: usize,
    /// ...between `FLOOR_BAND` and `LOW_BAND` — reachable by a short climb.
    low: usize,
    /// ...above `LOW_BAND` — up a stem.
    aloft: usize,
    /// ...standing in a column no ant has occupied at any point in the run.
    unvisited: usize,
    /// Face value of every edible cell standing, in joules, at this gut.
    worth: f64,
    /// Edible cells by distance band from the nearest nest column.
    by_dist: [usize; DIST_BANDS.len()],
    ants: usize,
    /// Deepest row above the soil any ant head is standing at.
    ant_high: i32,
    /// **Living animals that have produced at least one child** -- the
    /// breeders, in the sense `creature::try_bud`'s suppression uses.
    ///
    /// This is the "did it fire at all" counter for a breeding regime, and
    /// nothing else in the run can stand in for it. Under `queen` it must
    /// sit at one per colony; under `individual` it climbs with the
    /// population. A regime that reads as working from the birth count
    /// alone, with this flat at zero, never suppressed anything -- the
    /// collapse that was read as "chunks are working" while the body count
    /// was zero for the whole run.
    breeders: usize,
    /// Deepest generation any animal has reached, ever -- sterile workers
    /// included.
    gen: u16,
    /// Deepest generation of an animal that has ITSELF reproduced: the
    /// chain a genome actually travels.
    ///
    /// Printed beside `gen` rather than instead of it, because the two
    /// diverging is the whole tell. Under queen-only breeding `gen` counts
    /// workers that are genetic dead ends and reads one step deeper than
    /// the line; if `gen` climbs while this sits still, the arm is
    /// manufacturing dead ends and its generation count is answering a
    /// different question from the one asked.
    bgen: u16,
    /// **Live plant organisms** -- `world.live_organism_ids()` filtered to
    /// species with no `creature` component, the plant-side twin of `ants`.
    /// M2's brief (`Reports/lanes/evolution-lab-ecology-measure-2.md`) asks
    /// "does the thicket grow at all", and a stand that never establishes
    /// answers that before any fruit or bite count does.
    plants: usize,
    /// **Standing `windfall` cells, raw** -- counted off `cell.material`
    /// *before* the `diet_yield`/`EAT_YIELD_THRESHOLD` gate `edible` is
    /// filtered through, so this is a physical count of fallen fruit on the
    /// ground regardless of whether the founders' own gut would eat it. The
    /// loop that fills `edible` already visits every cell; this rides along
    /// rather than re-sweeping the grid.
    windfall: usize,
    /// **Standing `flower`/`fruit` cells, raw** -- the rebloom brief's own
    /// "the effect" pair beside `World::flowers_rebloomed`'s "it fired"
    /// (`Reports/evolution-lab-pollinator-design-2026-09-10.md`'s rebloom
    /// extension). A physical material census, same shape as `windfall`
    /// above and for the same reason: `edible` is gated on the founders'
    /// gut and would go to zero at a pure-carrion bias while the bed is
    /// visibly still in flower.
    standing_flowers: usize,
    standing_fruit: usize,
}

/// **Round 28's garden-loop instrument** — the fruit → animal → nest →
/// seedling brief's hypothesis (b): does windfall land somewhere the
/// colony's own foot traffic never reaches? `Reports/lanes/evolution-lab-
/// garden-loop.md`.
///
/// Accumulated across every stop and never cleared, the same convention
/// `visited` uses — so `windfall_heat`/`ant_heat` are a whole-run exposure,
/// not one frame's snapshot, and the two can be read against each other
/// column by column without a second grid sweep: `census`'s own loop
/// already visits every cell once per stop, so the windfall side rides
/// along with it instead of duplicating it (`mark_visited` gets the ant
/// side the same way, since it already walks every ant's chain once a
/// frame). One `Garden` per run, not per stop.
struct Garden {
    /// Per-column: how many stops found a standing `windfall` cell there.
    windfall_heat: Vec<u32>,
    /// Per-column: how many frames found any part of an ant's body there
    /// -- `mark_visited`'s own boolean turned into a count over the same
    /// loop, so this is exposure-time, not a distinct census.
    ant_heat: Vec<u32>,
    /// Standing windfall by height band, same thresholds `Sample::floor`/
    /// `low`/`aloft` use, but un-gated by diet -- a windfall census, not
    /// an edibility one.
    windfall_floor: u64,
    windfall_low: u64,
    windfall_aloft: u64,
    /// Standing windfall by distance from the nearest nest column, same
    /// bands `Sample::by_dist` uses.
    windfall_by_dist: [u64; DIST_BANDS.len()],
    /// **A light-based "under canopy" rather than the played bed's four
    /// scrambler columns hardcoded** — `plant::ambient_light_above` below
    /// half of open-sky noon-equivalent (`field::MAX_LIGHT`), so the same
    /// instrument reads any bed rather than only this one. Counts of
    /// stop-samples, not of distinct fruit (a fruit standing through ten
    /// stops is counted ten times, same as `windfall_heat`).
    windfall_shaded: u64,
    windfall_open: u64,
}

impl Garden {
    fn new(width: usize) -> Self {
        Garden {
            windfall_heat: vec![0; width],
            ant_heat: vec![0; width],
            windfall_floor: 0,
            windfall_low: 0,
            windfall_aloft: 0,
            windfall_by_dist: [0; DIST_BANDS.len()],
            windfall_shaded: 0,
            windfall_open: 0,
        }
    }
}

/// **What is standing here that this gut would eat, and where.**
///
/// Priced through `creature::diet_yield` and gated on
/// `EAT_YIELD_THRESHOLD` — the mouth's own predicate, called rather than
/// re-derived, because a readout that decides for itself what counts as food
/// is the standing house failure (`food_value`'s own doc records the eat verb
/// and the overlay disagreeing about what a cell is worth).
///
/// Swept over the grid rather than over the organism registry, unlike
/// `windfall_probe`'s: loose litter, corpses, fallen leaves and spoil are not
/// organism-owned and are exactly the food a walking ant meets. The sweep is
/// 512x320 per sample and the default interval is 900 frames.
///
/// Eight material-id/behaviour parameters is over clippy's bare threshold;
/// bundling the three raw material censuses (`windfall`/`flower`/`fruit`)
/// into a struct for this one caller would cost a name at every call site for
/// no reader this function has, so the lint is silenced rather than the
/// signature contorted -- the same call `src/worldgen/cave.rs` and
/// `src/sky.rs` already make for functions with several independent
/// per-call parameters.
#[allow(clippy::too_many_arguments)]
fn census(
    world: &World,
    spec: &LabBox,
    gut: f32,
    visited: &[bool],
    nest_cols: &[i32],
    windfall_id: Option<MaterialId>,
    flower_id: Option<MaterialId>,
    fruit_id: Option<MaterialId>,
    garden: &mut Garden,
) -> Sample {
    let mut s = Sample {
        ant_high: i32::MIN,
        gen: world.deepest_animal_generation,
        bgen: world.deepest_breeder_generation,
        ..Sample::default()
    };
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_some() {
            s.ants += 1;
            if state.children > 0 {
                s.breeders += 1;
            }
            if let Some(&(_, hy)) = state.chain.first() {
                s.ant_high = s.ant_high.max(spec.ground_y - hy);
            }
        } else {
            s.plants += 1;
        }
    }
    if s.ant_high == i32::MIN {
        s.ant_high = 0;
    }
    for y in 0..spec.height {
        for x in 0..spec.width {
            let cell = world.get(x, y);
            if windfall_id.is_some_and(|wid| cell.material == wid) {
                s.windfall += 1;
                // **Round 28's garden-loop instrument, riding this same
                // sweep rather than a second one.** `Reports/lanes/
                // evolution-lab-garden-loop.md` hypothesis (b): where does
                // fruit actually land, against where the colony actually
                // walks (`garden.ant_heat`, filled by `mark_visited`)?
                garden.windfall_heat[x.clamp(0, spec.width - 1) as usize] += 1;
                let wf_above = spec.ground_y - y;
                if wf_above <= FLOOR_BAND {
                    garden.windfall_floor += 1;
                } else if wf_above <= LOW_BAND {
                    garden.windfall_low += 1;
                } else {
                    garden.windfall_aloft += 1;
                }
                let wf_d = nest_cols.iter().map(|c| (c - x).abs()).min().unwrap_or(i32::MAX);
                for (b, &edge) in DIST_BANDS.iter().enumerate() {
                    if wf_d <= edge {
                        garden.windfall_by_dist[b] += 1;
                        break;
                    }
                }
                if plant::ambient_light_above(world, x, y) < field::MAX_LIGHT / 2.0 {
                    garden.windfall_shaded += 1;
                } else {
                    garden.windfall_open += 1;
                }
            }
            if flower_id.is_some_and(|fid| cell.material == fid) {
                s.standing_flowers += 1;
            }
            if fruit_id.is_some_and(|fid| cell.material == fid) {
                s.standing_fruit += 1;
            }
            let yielded = diet_yield(world, cell, gut);
            if yielded <= EAT_YIELD_THRESHOLD {
                continue;
            }
            // A living ant is not larder. The colony does not eat its own
            // (`is_living_kin`), and counting standing ants as food would put
            // the answer inside the question — a bed whose colony is dying
            // would read as a bed getting richer.
            if world.organism(cell.organism_id()).is_some_and(|st| world.species.get(st.species).creature.is_some()) {
                continue;
            }
            s.edible += 1;
            s.worth += yielded as f64;
            let above = spec.ground_y - y;
            if above <= FLOOR_BAND {
                s.floor += 1;
            } else if above <= LOW_BAND {
                s.low += 1;
            } else {
                s.aloft += 1;
            }
            if !visited[x.clamp(0, spec.width - 1) as usize] {
                s.unvisited += 1;
            }
            let d = nest_cols.iter().map(|c| (c - x).abs()).min().unwrap_or(i32::MAX);
            for (b, &edge) in DIST_BANDS.iter().enumerate() {
                if d <= edge {
                    s.by_dist[b] += 1;
                    break;
                }
            }
        }
    }
    s
}

/// **`no_colony=1` — the colony-removed control the rebloom brief's own
/// paired reading needs**, without touching a scenario file. A scenario's
/// `Colony`/`Colonies` timeline entries are unconditional
/// (`lab::scenario::apply_colony` reads nothing off `LabBox::colonies`), so
/// there is no flag on the scenario side to suppress arrival with; this
/// clears every creature organism's cells the instant they land instead,
/// which is public-API-only (no `pub(crate)` reached) and leaves the bed
/// itself, including whatever the colony would have eaten, untouched.
///
/// Cells rather than the organism slot: `World::free_organism` is
/// `pub(crate)` and not reachable from an example, but an organism whose
/// cell list has gone empty is reclaimed by the very next `step_organisms`
/// pass on its own (`push_organism`'s own doc on the reclaim rule) -- so
/// clearing the grid is both sufficient and the only public lever.
fn strip_colony(world: &mut World) -> usize {
    let mut cleared = 0usize;
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_none() {
            continue;
        }
        let positions: Vec<(i32, i32)> = state.cells.keys().copied().collect();
        for (x, y) in positions {
            world.set(x, y, Cell::EMPTY);
        }
        cleared += 1;
    }
    cleared
}

/// Every column any ant head has stood in, ever. Cumulative — the mask is
/// never cleared, so `unvisited` is a claim about the whole run rather than
/// about this frame.
///
/// **`heat` rides the same loop** (round 28's garden-loop instrument,
/// `Garden::ant_heat`) rather than a second walk of `live_organism_ids`:
/// every column any part of any ant's body occupies this frame gets one
/// count, so a column's total is exposure-time in frames, comparable
/// column-by-column against `census`'s `windfall_heat` at the stop
/// cadence. Called every frame (not gated by `sample_every`), so this is a
/// *finer* census than "sampled every N frames" asks for, not a coarser
/// one.
fn mark_visited(world: &World, visited: &mut [bool], heat: &mut [u32], width: i32) {
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_none() {
            continue;
        }
        for &(x, _) in &state.chain {
            if (0..width).contains(&x) {
                visited[x as usize] = true;
                heat[x as usize] += 1;
            }
        }
    }
}

fn main() {
    let control: String = arg("control").unwrap_or_else(|| "run".to_string());
    let frames: u64 = arg("frames").unwrap_or(300_000);
    let sample_every: u64 = arg("sample").unwrap_or(900);
    let handout: u64 = arg("handout").unwrap_or(0);
    // **`no_colony=1` -- the colony-removed control, on a scenario or off
    // one alike.** See `strip_colony`'s own doc for why this is a
    // post-arrival sweep rather than a flag on the founding call: a
    // scenario's own timeline places its colony unconditionally.
    let no_colony: bool = arg::<u32>("no_colony").unwrap_or(0) != 0;
    // **Found the colony at frame `ants_at` instead of at frame 0** --
    // `labshot.rs`'s own knob and the same owner framing, 2026-09-09: a bed
    // grown first and stocked later is how the game is actually played. 0
    // (the default) keeps this file's existing behaviour byte-for-byte --
    // founding happens before the loop, exactly as it always has.
    let ants_at: u64 = arg("ants_at").unwrap_or(0);
    // **`scenario=<name>` builds the whole bed from a saved scenario**, the
    // way `labshot` already does, and for the reason the played bed forced:
    // the flags below spread ONE species evenly, and the owner's bed is a
    // mix laid out by column. `lab::scenario` already places arbitrary
    // species at arbitrary columns and already founds colonies on a
    // timeline, so a species mix wanted a data file rather than another
    // knob here. A bad name refuses at load rather than running the default
    // bed under the wrong label -- the "an unknown argument is silently
    // ignored" shape `CLAUDE.md` names.
    let scenario: Option<Scenario> = arg::<String>("scenario").map(|n| {
        let mut sc = Scenario::load(&n).unwrap_or_else(|e| {
            eprintln!("scenario {n}: {e}");
            std::process::exit(2);
        });
        // **`seed=` overrides the scenario's own bed seed, and it has to be
        // done HERE, on the scenario, not on the `spec` below.**
        //
        // `Scenario::build` reads `self.bed`, so a seed applied only to the
        // local `spec` reaches the census and nothing else: the world is
        // built at the file's pinned seed every time. Caught by three seeds
        // returning a **byte-identical** sample row -- `CLAUDE.md`'s
        // "identical output across a change that must have moved something",
        // and it was one command away from turning an eighteen-run sweep
        // into three runs reported six times.
        if let Some(sd) = arg::<u64>("seed") {
            sc.bed.seed = sd;
        }
        // **`creature=<species>` overrides who a scenario's `Colony`/
        // `Colonies` placements found, the same shape `seed=` above is and
        // for the same reason: a scenario's `.ron` bakes one species into
        // its timeline (`played_bed.ron`'s `Colony(species: "ant", ...)`),
        // so racing a second species on the owner's own played bed needs an
        // override here rather than a second copy of the file --
        // `Reports/evolution-lab-pollinator-design-2026-09-10.md` P1's
        // three-arm measurement is exactly this: the played bed, once with
        // its own ant and once with a bloom-wired species, same seed, same
        // everything else.
        if let Some(species) = arg::<String>("creature") {
            sc.bed.colony_species = species.clone();
            for p in sc.placements.iter_mut().chain(sc.timeline.iter_mut().map(|e| &mut e.what)) {
                match p {
                    Placement::Colony { species: s, .. } | Placement::Colonies { species: s, .. } => *s = species.clone(),
                    _ => {}
                }
            }
        }
        sc
    });
    let spec = match &scenario {
        // The scenario's own bed, so `ground_y`, `width` and the soil the
        // census reads are the ones its placements were authored against --
        // **except the seed, which `seed=` still overrides.**
        //
        // Without that override a scenario pins its own `bed.seed` and every
        // run of a six-seed sweep is the same world six times. It announces
        // itself as three seeds reporting an identical founder count, which
        // is `CLAUDE.md`'s "identical outputs across settings mean the knob
        // was never connected" -- caught here by exactly that tell, one
        // command before an eighteen-run sweep would have been six copies of
        // three runs.
        Some(s) => s.bed.clone(),
        None => LabBox {
        width: arg("width").unwrap_or(512),
        height: arg("height").unwrap_or(320),
        // `labstats`' default, so a run here and a run there are the same bed.
        soil_depth: arg("soil").unwrap_or(80),
        founders: arg("founders").unwrap_or(8),
        colonies: arg("colonies").unwrap_or(1),
        compartments: arg("walls").unwrap_or(1),
        seed: arg("seed").unwrap_or(1),
        // `plant=<species>`: every figure this harness has produced was on
        // the default eight herbs; the owner's played bed is whatever the
        // PLANT chip offers, and a canopy is a different larder from a herb.
            species: arg::<String>("plant").unwrap_or_else(|| LabBox::default().species),
            ..LabBox::default()
        },
    };
    if control == "selftest" {
        return selftest(spec);
    }
    // Echo the parameters. A knob nobody can see the value of is a knob
    // nobody can tell is disconnected -- `plant_probe`'s 3.5-hour lesson.
    println!(
        "labforage: frames={frames} sample={sample_every} founders={} of {} colonies={} walls={} soil={} seed={} colony={} handout={handout} ants_at={ants_at} no_colony={no_colony}{}",
        spec.founders, spec.species, spec.colonies, spec.compartments, spec.soil_depth, spec.seed, spec.colony_species,
        scenario.as_ref().map(|s| format!(" scenario={} ({})", s.name, s.question)).unwrap_or_default()
    );
    // **Round 29 B1: the float's three switches, echoed with the rest.** A
    // knob nobody can see the value of is a knob nobody can tell is
    // disconnected -- `plant_probe`'s 3.5-hour lesson, and `flight_speed` in
    // particular ships as a *selector* (0.25/0.5/1.0) precisely because
    // which of them reads as a bee is a judge-by-eye question, so a log that
    // does not name the active one cannot be read at all. Taken from
    // `creature::flight_speed()` rather than from the env directly, so the
    // line reports what the simulation resolved and not what was typed.
    println!(
        "labforage: flight -- fly={} flight_speed={} land_afloat={}",
        if std::env::var("FLY").as_deref() == Ok("0") { "off (ballistic hop, main's arm)" } else { "on" },
        pixel_physics::sim::creature::flight_speed(),
        if std::env::var("LAND_AFLOAT").as_deref() == Ok("0") { "off (bug Z9 put back)" } else { "on" }
    );

    // Built bare and founded afterwards, for `windfall_probe`'s reason: a
    // species-level write after the founders are standing reaches nobody,
    // because `place_creature` copies the traits at placement. At the
    // default `ants_at=0` the founding happens right here, same as always;
    // at `ants_at>0` it is deferred to that frame, in the loop below.
    let (mut world, planted) = match &scenario {
        Some(s) => {
            if ants_at > 0 {
                println!("  ants_at={ants_at} ignored -- a scenario's own timeline decides colony placement");
            }
            let (w, p, sp) = s.build();
            println!(
                "  scenario {}: {} cells, {} plants, {} animals, {} settings applied",
                s.name, sp.cells, sp.plants, sp.animals, sp.settings
            );
            (w, p)
        }
        None => {
            let bare = LabBox { colonies: 0, ..spec.clone() };
            bare.build_counted()
        }
    };
    // **`hidden=<Input>:<unit>:<weight>[,...]` -- input-to-hidden weights set
    // on the colony species before a single ant is placed**, so a brain
    // change can be raced on the *played* bed without editing a `.ron` and
    // rebuilding between arms (the `include_str!` trap, which has produced
    // three bit-identical "sweeps" in this repo).
    //
    // **Before founding, and that is the whole reason it sits here rather
    // than beside the other knobs**: `place_creature` copies the genome at
    // placement, so the same write after the founders are standing reaches
    // nobody -- the identical trap the comment above this block records for
    // traits. `creature_arena` carries the same rider with the same syntax
    // and `trailfollow.rs` prints the string, so the three harnesses race
    // one set of numbers rather than three transcriptions of it.
    // **`bdecay=` / `adecay=` -- how fast a trail plane forgets**, against
    // `pheromone::DECAY_RHO`'s shipped 0.03 for both. The arm §Z7 needs:
    // once the ant can read channel B, the colony converges on patches it has
    // already eaten, and a trail that outlives its patch is that failure
    // exactly. Faster decay is the lever a real colony uses against it.
    for (key, channel) in [("adecay", Channel::A), ("bdecay", Channel::B)] {
        if let Some(rho) = arg::<f32>(key) {
            world.pheromones.set_channel_rho(channel, rho);
            println!("  {key}= {rho} (shipped {})", pixel_physics::sim::pheromone::DECAY_RHO);
        }
    }
    // **Same block, same reason, same refusal.** See `wire_rider`'s own doc:
    // before founding, because `place_creature` copies the genome at
    // placement -- and it asserts that the write actually moved a slot,
    // because an arm that matched nothing is the control wearing a label,
    // which reads as a clean null rather than as a broken run.
    let wires = wire_rider();
    if !wires.is_empty() {
        let sid = world.species.id_of(&spec.colony_species).expect("the colony species is compiled in");
        let mut genome = world.species.get(sid).genome.clone();
        let mut moved = 0;
        for &(input, output, w) in &wires {
            let i = brain::io_slot(input, output);
            if genome[i] != w {
                genome[i] = w;
                moved += 1;
            }
        }
        assert!(moved > 0, "wire= matched no slot the species did not already carry; this arm is the control wearing a label");
        world.species.set_genome(sid, genome);
        println!(
            "  wire= set {moved} of {} input->output weights on {}: {}",
            wires.len(),
            spec.colony_species,
            wires
                .iter()
                .map(|&(i, o, w)| format!("{}:{}:{w}", brain::INPUT_NAMES[i as usize], brain::OUTPUT_NAMES[o as usize]))
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    let hidden = hidden_rider();
    if !hidden.is_empty() {
        let sid = world.species.id_of(&spec.colony_species).expect("the colony species is compiled in");
        let mut genome = world.species.get(sid).genome.clone();
        let mut moved = 0;
        for &(input, unit, w) in &hidden {
            let i = brain::ih_slot(input, unit);
            if genome[i] != w {
                genome[i] = w;
                moved += 1;
            }
        }
        assert!(moved > 0, "hidden= matched no slot the species did not already carry; this arm is the control wearing a label");
        world.species.set_genome(sid, genome);
        println!(
            "  hidden= set {moved} of {} input->hidden weights on {}: {}",
            hidden.len(),
            spec.colony_species,
            hidden.iter().map(|&(i, u, w)| format!("{}:{u}:{w}", brain::INPUT_NAMES[i as usize])).collect::<Vec<_>>().join(",")
        );
    }
    // **Where the nests are, which the distance bands are measured from.**
    // A scenario sets `colonies: 0` and founds on its timeline instead, so
    // `colony_columns()` is empty for one and the whole `d<16 / d<48 /
    // d<128 / far` split would silently collapse into `far` -- a census
    // that still prints four columns and means none of them. Read the
    // scenario's own `Colony` entries instead, from placements and timeline
    // alike, since either may carry them.
    let nest_cols: Vec<i32> = match &scenario {
        Some(s) => {
            let mut v: Vec<i32> = s
                .placements
                .iter()
                .chain(s.timeline.iter().map(|e| &e.what))
                .filter_map(|p| match p {
                    Placement::Colony { x, .. } => Some(*x),
                    _ => None,
                })
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        }
        None => spec.colony_columns(),
    };
    let mut ants_placed = 0usize;
    let mut gut = 0.0f32;
    if scenario.is_none() && ants_at == 0 {
        for &x in &nest_cols {
            ants_placed += world.found_colony(x, spec.ground_y - 2);
        }
        gut = ant_gut_bias(&world);
        if no_colony {
            let cleared = strip_colony(&mut world);
            println!("  no_colony=1: cleared {cleared} colony/colonies right back off the bed");
            ants_placed = 0;
        }
    }
    println!(
        "  bed: {} of {} founders planted, {}",
        planted.planted,
        planted.asked,
        match (&scenario, ants_at) {
            (Some(s), _) => format!("colonies arrive on {}'s timeline, nests at {nest_cols:?}", s.name),
            (None, 0) => format!("{ants_placed} ants in {} colony/colonies at {nest_cols:?}, founder gut_bias {gut}", spec.colonies),
            (None, _) => format!("ants founded later at frame {ants_at}, {} colony/colonies staged at {nest_cols:?}", spec.colonies),
        }
    );
    if scenario.is_none() && ants_at > frames {
        // The same "an unknown argument is silently ignored" shape
        // `CLAUDE.md` names, with a frame number standing in for the flag:
        // a founding frame past the run's own length would otherwise never
        // fire and the run would read as a colony that starved instantly.
        println!("  WARNING: ants_at={ants_at} is past frames={frames} -- the colony is never founded");
    }
    println!(
        "  bands: floor <= {FLOOR_BAND} rows above soil, low <= {LOW_BAND}, aloft above that | distance bands {DIST_BANDS:?}\n"
    );

    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    let mut visited = vec![false; spec.width as usize];
    // Round 28's garden-loop instrument -- see `Garden`'s own doc.
    let mut garden = Garden::new(spec.width as usize);
    let windfall_id = world.materials.id_of("windfall");
    let flower_id = world.materials.id_of("flower");
    let fruit_id = world.materials.id_of("fruit");
    let mut handed_out = 0u64;
    let mut first: Option<Sample> = None;
    let mut last = Sample::default();
    // See `trace=`'s own block in the loop for what these four do. The scan
    // is over the whole bed once per traced frame, which is why it is off
    // unless asked for and why `traceevery` defaults to a round number
    // rather than to 1.
    let trace_species: Option<String> = arg::<String>("trace");
    let trace_every: u64 = arg("traceevery").unwrap_or(50).max(1);
    let trace_from: u64 = arg("tracefrom").unwrap_or(0);
    let trace_to: u64 = arg("traceto").unwrap_or(u64::MAX);
    // See the census in the loop below for why this is sampled at 10 frames.
    let mut head_max: std::collections::BTreeMap<String, i32> = std::collections::BTreeMap::new();
    let mut peak_edible = 0usize;

    println!(
        "{:>7} {:>5} {:>6} {:>7} {:>10} {:>6} {:>6} {:>6} {:>9} | {:>5} {:>5} {:>5} {:>5} | {:>4} {:>5} {:>5} {:>6} | {:>4} {:>4} {:>4} | {:>5} {:>8} {:>5}",
        "frame", "ants", "plnts", "edible", "worth(J)", "floor", "low", "aloft", "unvisited",
        "d<16", "d<48", "d<128", "far", "high", "eats", "born", "died",
        "brdr", "gen", "bgen", "fvis", "necJ", "bseen"
    );
    for f in 0..=frames {
        // **Founding, deferred to here when `ants_at > 0`.** Checked before
        // `mark_visited`/`census` below so the frame it lands on already
        // sees the colony rather than the empty bed it replaced.
        if scenario.is_none() && ants_at > 0 && f == ants_at {
            for &x in &nest_cols {
                ants_placed += world.found_colony(x, spec.ground_y - 2);
            }
            gut = ant_gut_bias(&world);
            if no_colony {
                let cleared = strip_colony(&mut world);
                println!("  ants_at {ants_at}: no_colony=1, cleared {cleared} colony/colonies right back off the bed\n");
                ants_placed = 0;
            } else {
                println!("  ants_at {ants_at}: founded {ants_placed} ants at {nest_cols:?}, founder gut_bias {gut}\n");
            }
        }
        // **The scenario's timeline, before the census on the same frame**,
        // so the frame a colony lands on already reports it rather than the
        // empty bed it replaced -- the same ordering `ants_at` above needs
        // and for the same reason.
        if let Some(sc) = &scenario {
            let arrived = pixel_physics::lab::scenario::tick_timeline(sc, &mut world, &spec);
            if arrived.animals > 0 {
                ants_placed += arrived.animals;
                // **Read the founders' gut the frame they arrive, not
                // before.** `ant_gut_bias` over an empty bed is 0.0, which
                // is a real gut value (a pure-carrion ant), so a gut left
                // at its initialiser is indistinguishable from a measured
                // one -- a null wearing a measurement.
                gut = ant_gut_bias(&world);
                if no_colony {
                    // **The colony-removed control, on a scenario's own
                    // timeline.** `apply_colony`/`apply_colonies` read
                    // nothing off `LabBox::colonies`, so the timeline places
                    // its colony regardless -- this clears it the instant it
                    // lands rather than suppressing the placement, which
                    // needs no edit to the scenario file at all.
                    let cleared = strip_colony(&mut world);
                    println!("  frame {f}: no_colony=1, cleared {cleared} colony/colonies right back off the bed\n");
                    ants_placed -= arrived.animals;
                } else {
                    println!("  frame {f}: {} animal(s) arrived on the timeline, founder gut_bias {gut}\n", arrived.animals);
                }
            }
        }
        // **`trace=<species> [traceevery=N] [tracefrom=F] [traceto=T]` --
        // follow ONE animal and print what it is doing.** Round 28's own
        // question, and it is not one a counter can answer: the flitter's
        // `flower_visits` reads 3-20 over 120,000 frames with the eye firing
        // constantly, and "turns toward the flower and overshoots", "lands
        // beside it and does not bite", "never gets within a cell of one"
        // and "sits on a leaf" are four different repairs that produce the
        // same small number.
        //
        // The first living animal of the species by organism id, the same
        // rule `labgif`'s `follow=` picks its subject by, so a trace and a
        // card of the same run are of the same animal. One line per
        // `traceevery` frames: where its head is, how high, whether it is
        // in the air, where the nearest flower is and whether that flower
        // is actually paying, whether it is close enough to drink, and the
        // running visit count. **`visits` is the effect column** -- the
        // other five say what the animal did and only that one says whether
        // it fed.
        if let Some(sp_name) = &trace_species {
            if f >= trace_from && f <= trace_to && f % trace_every == 0 {
                if let Some(sid) = world.species.id_of(sp_name) {
                    let subject = world
                        .live_organism_ids()
                        .into_iter()
                        .find(|id| world.organism(*id).is_some_and(|st| st.species == sid));
                    if let Some(id) = subject {
                        if let Some(st) = world.organism(id) {
                            let (hx, hy) = st.chain.first().copied().unwrap_or((0, 0));
                            let aloft = st.flight.is_some();
                            let energy = st.energy;
                            // Nearest flower cell in the whole bed, by
                            // Chebyshev -- the same distance the 8-ring the
                            // mouth uses is a radius of, so "d 1" reads as
                            // "could drink right now" with no arithmetic.
                            let mut best: Option<(i32, i32, i32, bool)> = None;
                            for y in 0..spec.height {
                                for x in 0..spec.width {
                                    let c = world.get(x, y);
                                    if c.organism_id() == 0
                                        || pixel_physics::sim::organism::cell_type(c.aux())
                                            != Some(pixel_physics::sim::organism::CellType::Flower)
                                    {
                                        continue;
                                    }
                                    let d = (x - hx).abs().max((y - hy).abs());
                                    if best.is_none_or(|(bd, _, _, _)| d < bd) {
                                        best = Some((d, x, y, pixel_physics::sim::plant::nectar_available(&world, x, y)));
                                    }
                                }
                            }
                            let visits = world.flower_visits_by_species.get(&sid.0).copied().unwrap_or(0);
                            match best {
                                Some((d, fx, fy, wet)) => println!(
                                    "  trace {sp_name} f{f}: head ({hx},{hy}) {} rows up, {} | nearest flower ({fx},{fy}) d {d} {} | adjacent {} | energy {energy:.0} | visits {visits}",
                                    spec.ground_y - hy,
                                    if aloft { "IN THE AIR" } else { "standing" },
                                    if wet { "PAYING" } else { "dry" },
                                    if d <= 1 { "YES" } else { "no" }
                                ),
                                None => println!(
                                    "  trace {sp_name} f{f}: head ({hx},{hy}) {} rows up, {} | no flower standing in the bed | energy {energy:.0} | visits {visits}",
                                    spec.ground_y - hy,
                                    if aloft { "IN THE AIR" } else { "standing" }
                                ),
                            }
                        }
                    } else {
                        println!("  trace {sp_name} f{f}: no living {sp_name} left");
                    }
                }
            }
        }
        mark_visited(&world, &mut visited, &mut garden.ant_heat, spec.width);
        // **How high any animal of each species has ever got, in rows above
        // the soil surface** -- P2's reach census, and the one number that
        // separates the two readings of a zero `flower_visits`. The flower
        // this bed's animals live on stands ~22 rows up its own stem, and
        // `dead-ends.md`'s hopper entry is built on exactly this figure
        // (*"the wired seed 3 colony's highest head reached 21 rows above
        // the soil"*), so it is stated in the same units on purpose: a
        // successor that reads 30 here and still never feeds has a
        // different problem from one that reads 13.
        //
        // **Every 10 frames, not every sample.** A hop lasts tens of frames
        // and `sample_every` is tens of thousands, so a per-sample reading
        // would photograph whatever happened to be in the air at six
        // instants -- the max of a sparse sample of a transient, which is
        // not a maximum at all. Ten frames is inside the shortest arc and
        // walks a few dozen live organisms; the cost does not show against
        // the sweep.
        if f % 10 == 0 {
            for id in world.live_organism_ids() {
                let Some(state) = world.organism(id) else { continue };
                if world.species.get(state.species).creature.is_none() {
                    continue;
                }
                let Some(&(_, hy)) = state.chain.first() else { continue };
                let rows = spec.ground_y - hy;
                if rows > 0 {
                    let e = head_max.entry(world.species.get(state.species).name.clone()).or_insert(0);
                    *e = (*e).max(rows);
                }
            }
        }
        if f % sample_every == 0 {
            let s = census(&world, &spec, gut, &visited, &nest_cols, windfall_id, flower_id, fruit_id, &mut garden);
            peak_edible = peak_edible.max(s.edible);
            if first.is_none() {
                first = Some(s);
            }
            last = s;
            let st = world.creature_stats;
            // One line per sample and every column on it, so the whole run is
            // one greppable block rather than a shape that has to be reread.
            println!(
                "{f:>7} {:>5} {:>6} {:>7} {:>10.0} {:>6} {:>6} {:>6} {:>9} | {:>5} {:>5} {:>5} {:>5} | {:>4} {:>5} {:>5} {:>6} | {:>4} {:>4} {:>4} | {:>5} {:>8.0} {:>5} | wfall={} flwr={} frt={} rblm={} rblk={}",
                s.ants, s.plants, s.edible, s.worth, s.floor, s.low, s.aloft, s.unvisited,
                s.by_dist[0], s.by_dist[1], s.by_dist[2], s.by_dist[3],
                s.ant_high, st.eats, st.births, st.deaths,
                s.breeders, s.gen, s.bgen,
                // **B1' (nectar, in two currencies)** --
                // `Reports/evolution-lab-pollinator-design-2026-09-10.md`
                // §3.1. `fvis` is the sensitivity counter (every reach of an
                // owned flower, paid or not); `necJ` is the cumulative
                // joules actually paid, the effect half -- their pairing is
                // the positive control the design's own brief names: at
                // `nectar_refill: 0.0`, `necJ` must stay flat at 0 across
                // the whole run while `fvis` keeps climbing. Divide `necJ`
                // by `f / 1000.0` for "joules paid per 1,000 frames" at any
                // sampled frame.
                //
                // **P1's own pair, beside it**: `bseen` is `bloom_seen`, the
                // bloom sense's own "did it fire at all" -- an eyed animal
                // that read `BloomNear > 0` this tick, cumulative over the
                // whole run so far. Read it against `fvis`: a bloom sense
                // that fires while `fvis` stays flat found flowers and did
                // not reach one; a `fvis` that climbs with `bseen` at 0 is a
                // bite that arrived by chance, not by sight.
                world.flower_visits, world.nectar_paid, world.creature_stats.bloom_seen, s.windfall,
                // **The rebloom extension, 2026-09-10** (this PR). `flwr`/
                // `frt` are the standing counts -- the effect a picture would
                // show, read at every stop rather than only at the run's end.
                // `rblm` is `World::flowers_rebloomed`, cumulative -- the "did
                // it fire at all" counter, since a bed that merely takes
                // longer to run dry would read identically on `flwr` alone
                // until this run is long enough to tell the two apart.
                // `rblk` is the reproductive-account refusal
                // (`organ_ripening_blocked`), the same counter the rest of
                // the organ pipeline uses -- a rebloom that keeps the flower
                // count up by exploding this instead is not the fix.
                s.standing_flowers, s.standing_fruit, world.flowers_rebloomed, world.organ_ripening_blocked
            );
        }
        if handout > 0 && f > 0 && f % handout == 0 {
            if let Some(wid) = windfall_id {
                let x = nest_cols[(handed_out as usize) % nest_cols.len().max(1)];
                for dy in 1..=6 {
                    let y = spec.ground_y - dy;
                    if world.get(x, y).material == pixel_physics::sim::material::EMPTY {
                        world.set(x, y, Cell::new(wid, 0));
                        handed_out += 1;
                        break;
                    }
                }
            }
        }
        if f < frames {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        }
    }

    let st = world.creature_stats;
    let l = &world.energy_ledger;
    let cols = visited.iter().filter(|v| **v).count();
    let burn = l.metabolized + l.moved + l.synapse_tax;
    println!("\n  the colony's reach over {frames} frames:");
    println!(
        "    columns ever walked   {cols} of {} ({:.0}% of the bed's width)",
        spec.width,
        100.0 * cols as f64 / spec.width as f64
    );
    println!("    highest ant head      {} rows above the soil at the last sample", last.ant_high);
    println!(
        "    edible standing now   {} cells ({:.0} J), peak over the run {peak_edible}",
        last.edible, last.worth
    );
    println!(
        "    ...of which unvisited {} cells ({:.0}% of what is standing) -- food in a column no ant ever entered",
        last.unvisited,
        if last.edible > 0 { 100.0 * last.unvisited as f64 / last.edible as f64 } else { 0.0 }
    );
    println!("    ...by height          floor {} low {} aloft {}", last.floor, last.low, last.aloft);
    // **M2's own two questions, both live counts rather than cumulative.**
    // `plants` is the stand itself (did the thicket establish); `windfall`
    // is standing fallen fruit on the ground right now, unfiltered by
    // whether the founders' gut would eat it -- the physical quantity a
    // bite needs present, not the diet-priced `edible` figure above.
    println!(
        "  standing now: plants {} | windfall (fallen fruit) {} cells, cumulative fruit_dropped {}",
        last.plants, last.windfall, world.fruit_dropped
    );
    println!(
        "\n  what the colony took: eats {} pickups {} | harvested plant {:.0} J corpse {:.0} J against burn {:.0} J",
        st.eats, st.pickups, l.harvested_plant, l.harvested_corpse, burn
    );
    println!("  animals: born {} died {} alive {} | handouts placed {handed_out}", st.births, st.deaths, last.ants);
    // **`deliveries` is the round trip, and it is the number a trail is
    // *for*.** `cols` says how far the colony ranged and `eats` says what it
    // put in its own mouth; only this says a laden ant got back to the nest,
    // which is what central-place foraging means and what the homing half of
    // the pheromone circuit exists to produce. It counts only for a species
    // that authors a `nest` -- `creature.rs`'s `adjacent_nest` returns
    // `false` for ever without one, so a run on `ancestor` reads 0 here by
    // construction rather than by failure.
    println!("  round trips: deliveries {} nest visits {}", st.deliveries, st.nest_visits);
    // **A2 -- the seed rides home.** `Reports/evolution-lab-ecology-design-
    // 2026-09-10.md` §2.6/§2.7, Brief A2. `nest_cols` is already computed
    // above (scenario `Colony` entries or `LabBox::colony_columns`) for the
    // food-distance bands, so the headline reuses it rather than asking the
    // engine to have an opinion about where a nest is -- see
    // `World::pip_germination_x`'s own doc for why that split is
    // deliberate.
    const NEAR_NEST_REACH: i32 = 32;
    let plants_from_pip_near_nest =
        world.pip_germination_x.iter().filter(|&&x| nest_cols.iter().any(|&c| (c - x).abs() <= NEAR_NEST_REACH)).count();
    // **Round 29's seed-where-eaten lane** -- `Reports/lanes/evolution-lab-
    // seed-where-eaten.md`, the owner's 2026-09-11 rule: "where should the
    // seed drop when a creature picks up food -- it should drop where it
    // is eaten, not immediately." This is the distribution the rule is
    // actually asking about: for each pip digestion released
    // (`World::pip_digestion_release_x`), the eating animal's column
    // distance from the nearest nest, bucketed into the same `DIST_BANDS`
    // every other distance reading in this file already uses rather than
    // a bespoke scale -- a colony that finishes its food before it gets
    // home should read differently here than one that always carries a
    // meal all the way in first.
    let mut digestion_release_by_dist = [0u64; DIST_BANDS.len()];
    for &x in &world.pip_digestion_release_x {
        let d = nest_cols.iter().map(|c| (c - x).abs()).min().unwrap_or(i32::MAX);
        for (b, &edge) in DIST_BANDS.iter().enumerate() {
            if d <= edge {
                digestion_release_by_dist[b] += 1;
                break;
            }
        }
    }
    println!(
        "  A2, eaten -- where the eater stood when digestion released the seed, by distance from the nearest nest {DIST_BANDS:?}: {digestion_release_by_dist:?} (n={})",
        world.pip_digestion_release_x.len()
    );
    // Median, not mean: `seed_transit_frames` is exactly the kind of
    // long-tailed sample (one lucky seed dropped a step from the bite, one
    // unlucky one carried the length of the bed) a mean would let the tail
    // dominate. `CLAUDE.md`'s own worked cases are about the same shape.
    let seed_transit_median = {
        let mut v = world.seed_transit_frames.clone();
        v.sort_unstable();
        v.get(v.len() / 2).copied()
    };
    println!(
        "  A2 -- the seed rides home: seeds_carried {} seeds_delivered {} | plants_from_pip {} of which within {NEAR_NEST_REACH} cols of a nest {plants_from_pip_near_nest} | \
         median carry {} frames over {} completed deliveries (herb.seed_half_life is 14,000; this becomes a live knob only if the median nears four figures)",
        world.seeds_carried,
        world.seeds_delivered,
        world.plants_from_pip,
        seed_transit_median.map_or_else(|| "n/a".to_string(), |m| m.to_string()),
        world.seed_transit_frames.len()
    );
    // **P2's own roll-ups (the flitter, Brief P2).** Three per-species
    // lines, because every one of them reads a single number on a bed that
    // holds two animals and the whole question is which animal it belongs
    // to. `CLAUDE.md`'s effect-counter rule is why they come in pairs here:
    // `impulses` says the verb fired and `impulses_refused` says how much of
    // that firing was into thin air (a launch called while already airborne
    // is refused, `creature::launch`), so the number that means anything is
    // the difference, printed as `real_launches` rather than left to be
    // subtracted by eye.
    let mut alive_by: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    for g in world.live_creature_groups() {
        *alive_by.entry(world.species.get(g.species).name.clone()).or_insert(0) += g.alive;
    }
    let fmt_alive = alive_by.iter().map(|(k, v)| format!("{k}:{v}")).collect::<Vec<_>>().join(",");
    let fmt_visits = world
        .flower_visits_by_species
        .iter()
        .map(|(sp, n)| format!("{}:{n}", world.species.get(pixel_physics::sim::organism::SpeciesId(*sp)).name))
        .collect::<Vec<_>>()
        .join(",");
    // **The effect half, and it points the other way.** A visit is an animal
    // drinking and the flower surviving; a bite is the flower coming off. The
    // pair is the design's own §2.2 instrument, and a nectar-only species must
    // read 0 here while an ordinary one still moves -- which is the positive
    // control that says this is a fact about the mouth and not a blind row.
    let fmt_bitten = world
        .flowers_bitten_by_species
        .iter()
        .map(|(sp, n)| format!("{}:{n}", world.species.get(pixel_physics::sim::organism::SpeciesId(*sp)).name))
        .collect::<Vec<_>>()
        .join(",");
    // The death-cause histogram, per species -- the cost fork's own
    // deliverable ("report the death-cause histogram and stop") and the only
    // place `starved aloft` can be told from ordinary starvation for ONE of
    // two animals in a bed. `World::group_deaths` is already split by
    // `(species, colony)`; this rolls the colonies up.
    let mut deaths_by: std::collections::BTreeMap<String, [u64; pixel_physics::sim::organism::DEATH_CAUSES]> =
        std::collections::BTreeMap::new();
    for g in &world.group_deaths {
        let row = deaths_by
            .entry(world.species.get(g.species).name.clone())
            .or_insert([0; pixel_physics::sim::organism::DEATH_CAUSES]);
        for (i, n) in g.by_cause.iter().enumerate() {
            row[i] += n;
        }
    }
    let fmt_deaths = deaths_by
        .iter()
        .map(|(name, row)| {
            let causes = pixel_physics::sim::organism::DEATH_CAUSE_LIST
                .iter()
                .enumerate()
                .filter(|(i, _)| row[*i] > 0)
                .map(|(i, c)| format!("{}:{}", c.label().replace(' ', "_"), row[i]))
                .collect::<Vec<_>>()
                .join("/");
            format!("{name}[{}]", if causes.is_empty() { "none".to_string() } else { causes })
        })
        .collect::<Vec<_>>()
        .join(",");
    let starved_aloft = world.deaths_by_cause[pixel_physics::sim::organism::DeathCause::StarvedInFlight.index()];

    // **Round 28 -- the garden-loop instrument.** `Reports/lanes/
    // evolution-lab-garden-loop.md`. Two questions the A2 line above
    // cannot answer: given a pip *was* delivered (or stood at the bite
    // site), does its drop cell actually meet the species' own Germinate
    // thresholds (hypothesis a), and does windfall land somewhere the
    // colony's own foot traffic reaches at all (hypothesis b)?
    let pip_checks_n = world.pip_checks.len();
    let pip_checks_delivered = world.pip_checks.iter().filter(|c| c.delivered).count();
    let pip_checks_resting = world.pip_checks.iter().filter(|c| c.resting).count();
    let pip_checks_light_ok = world.pip_checks.iter().filter(|c| c.light >= c.light_threshold).count();
    let pip_checks_water_ok = world.pip_checks.iter().filter(|c| c.soil_water >= c.soil_water_threshold).count();
    let pip_checks_ready = world
        .pip_checks
        .iter()
        .filter(|c| c.resting && c.light >= c.light_threshold && c.soil_water >= c.soil_water_threshold)
        .count();
    println!(
        "\n  round 28 -- garden loop, hypothesis (a): first Germinate check per pip \
         ({pip_checks_n} pips checked, {pip_checks_delivered} rode home/A2, {} stood at the bite/A1):",
        pip_checks_n - pip_checks_delivered
    );
    println!(
        "    resting {pip_checks_resting}/{pip_checks_n} | light>=threshold {pip_checks_light_ok}/{pip_checks_n} | \
         soil_water>=threshold {pip_checks_water_ok}/{pip_checks_n} | all three (would germinate now) {pip_checks_ready}/{pip_checks_n}"
    );
    for c in &world.pip_checks {
        println!(
            "      frame {:>7} x={:>4} y={:>4} {} resting={:<5} light {:>5.2}/{:<5.2} ({}) water {:>4.2}/{:<4.2} ({}) overburden={}",
            c.frame,
            c.x,
            c.y,
            if c.delivered { "A2" } else { "A1" },
            c.resting,
            c.light,
            c.light_threshold,
            if c.light >= c.light_threshold { "OK " } else { "LOW" },
            c.soil_water,
            c.soil_water_threshold,
            if c.soil_water >= c.soil_water_threshold { "OK " } else { "DRY" },
            c.overburden
        );
    }
    let pip_rot_x = &world.pip_rot_x;
    let pip_eaten_x = &world.pip_eaten_x;
    println!(
        "  round 28 -- pip exits: seeds_spilled {} = seeds_carried {} (rode home) + {} (stood at the bite) | \
         plants_from_pip {} + pips_rotted {} (at {pip_rot_x:?}) + pips_eaten {} (at {pip_eaten_x:?}) should not exceed seeds_spilled, the rest still standing | \
         dig_diverted_seed {} (garden-fix: the dig verb routed around a live seed instead of clearing it)",
        world.seeds_spilled,
        world.seeds_carried,
        world.seeds_spilled.saturating_sub(world.seeds_carried),
        world.plants_from_pip,
        world.pips_rotted,
        world.pips_eaten,
        world.dig_diverted_seed
    );

    // Hypothesis (b): where fruit lands against where the colony walks.
    let total_wf_heat: u64 = garden.windfall_heat.iter().map(|&v| u64::from(v)).sum();
    let wf_dead_zone: u64 =
        garden.windfall_heat.iter().zip(garden.ant_heat.iter()).filter(|&(_, &ah)| ah == 0).map(|(&wh, _)| u64::from(wh)).sum();
    let wf_dead_zone_pct = if total_wf_heat > 0 { 100.0 * wf_dead_zone as f64 / total_wf_heat as f64 } else { 0.0 };
    println!(
        "\n  round 28 -- garden loop, hypothesis (b): windfall landing vs the colony's own foot traffic:"
    );
    println!(
        "    standing-windfall height bands: floor {} low {} aloft {} | distance bands {:?} | \
         under canopy (<{:.1} noon-equiv light) {} of {} column-stops",
        garden.windfall_floor,
        garden.windfall_low,
        garden.windfall_aloft,
        garden.windfall_by_dist,
        field::MAX_LIGHT / 2.0,
        garden.windfall_shaded,
        garden.windfall_shaded + garden.windfall_open
    );
    println!(
        "    windfall column-stops {total_wf_heat}, of which {wf_dead_zone} ({wf_dead_zone_pct:.0}%) sit in a column \
         the colony's own heat map never touched all run (ant_heat==0 there)"
    );
    const HEAT_BUCKET: usize = 32;
    let nbuckets = (spec.width as usize).div_ceil(HEAT_BUCKET);
    let mut wf_buckets = vec![0u64; nbuckets];
    let mut ant_buckets = vec![0u64; nbuckets];
    for (i, (&wh, &ah)) in garden.windfall_heat.iter().zip(garden.ant_heat.iter()).enumerate() {
        wf_buckets[i / HEAT_BUCKET] += u64::from(wh);
        ant_buckets[i / HEAT_BUCKET] += u64::from(ah);
    }
    println!("    by {HEAT_BUCKET}-column band, col_start: windfall column-stops / ant body-frames (nest at {nest_cols:?}):");
    for b in 0..nbuckets {
        println!("      {:>4}: {:>7} / {:>10}", b * HEAT_BUCKET, wf_buckets[b], ant_buckets[b]);
    }

    println!(
        "SUMMARY seed={} founders={} colonies={} frames={frames} handout={handout} cols={cols} plants={} windfall={} fruit_dropped={} edible={} unvisited={} floor={} aloft={} \
         peak_edible={peak_edible} eats={} born={} died={} alive={} intake={:.0} burn={:.0} shares={} shared_j={:.0} moves={} deliveries={} nest_visits={} \
         regime={} breeders={} gen={} bgen={} windfall_bitten={} seeds_spilled={} plants_from_pip={} pips_rotted={} pips_eaten={} \
         windfall_bitten_ownerless={} seeds_carried={} seeds_delivered={} plants_from_pip_near_nest={} seed_transit_median={} lookup={} visits={} \
         flower_visits={} nectar_paid={:.0} nectar_j_per_1000f={:.2} organs_built={} bloom_seen={} \
         standing_flowers={} standing_fruit={} flowers_rebloomed={} organ_ripening_blocked={} organ_ripening_paid={} \
         pip_checks={pip_checks_n} pip_checks_delivered={pip_checks_delivered} pip_checks_resting={pip_checks_resting} \
         pip_checks_light_ok={pip_checks_light_ok} pip_checks_water_ok={pip_checks_water_ok} pip_checks_ready={pip_checks_ready} \
         windfall_col_stops={total_wf_heat} windfall_dead_zone_stops={wf_dead_zone} windfall_dead_zone_pct={wf_dead_zone_pct:.0} \
         windfall_floor={} windfall_low={} windfall_aloft={} windfall_shaded={} windfall_open={} dig_diverted_seed={} \
         launch_attempts={} real_launches={} impulses_refused={} refused_pct={:.0} starved_aloft={} flight_frames={} \
         flower_visits_by={} flowers_bitten_by={} alive_by={} deaths_by={} head_max_rows={} \
         pips_set_on_soil={} pips_set_on_nest={} \
         fly_ticks={} fly_frames={} fly_turns={} fly_j={:.1} landed_afloat={} \
         moves_per_launch={:.2} frames_per_launch={:.0} fly_share={:.0} flight_speed={} \
         pips_released_by_digestion={} fruit_dropped_with_seed={} digestion_release_by_dist={digestion_release_by_dist:?} \
         nest_blends={} share_blends={} nest_sites={} nest_gap_max={:.4}",
        spec.seed, spec.founders, spec.colonies, last.plants, last.windfall, world.fruit_dropped, last.edible, last.unvisited, last.floor, last.aloft,
        st.eats, st.births, st.deaths, last.ants, l.harvested_plant + l.harvested_corpse, burn, st.shares, st.shared_j, st.moves,
        st.deliveries, st.nest_visits,
        std::env::var("PIXEL_PHYSICS_BREEDING").unwrap_or_else(|_| "individual".to_string()),
        last.breeders, world.deepest_animal_generation, world.deepest_breeder_generation,
        // **M2's own counter** -- every bite that reached an owned windfall
        // cell and was about to roll for survival, counted *before* the
        // roll in `plant::seed_survives_bite`. The true bite rate, rather
        // than an estimate backed out of `seeds_spilled / seed_gut_survival`
        // (which is silent at `seed_gut_survival: 0.0`). See
        // `World::windfall_bitten`.
        world.windfall_bitten,
        // **A1's counters, both halves** -- "it fired" (`seeds_spilled`) and
        // the three exits that sum to it (`plants_from_pip`, `pips_rotted`,
        // `pips_eaten`), plus whatever is still standing as a `pip`.
        // `Reports/evolution-lab-ecology-design-2026-09-10.md` §2.6.
        world.seeds_spilled, world.plants_from_pip, world.pips_rotted, world.pips_eaten,
        // **The measure lane's finding, 2026-09-10** -- a bite met a
        // `windfall` with no organism owning it, so nothing above could
        // roll. High against a low `seeds_spilled`: read this before
        // retuning `seed_gut_survival`. See `World::
        // windfall_bitten_ownerless`.
        world.windfall_bitten_ownerless,
        // **A2's own counters** -- "it fired" (`seeds_carried`), "it worked"
        // (`seeds_delivered`), the headline (`plants_from_pip_near_nest`),
        // and the transit-cost check, all computed just above.
        // `Reports/evolution-lab-ecology-design-2026-09-10.md` §2.6/§2.7.
        world.seeds_carried,
        world.seeds_delivered,
        plants_from_pip_near_nest,
        seed_transit_median.map_or_else(|| "n/a".to_string(), |m| m.to_string()),
        // **`visits` is the whole point of the breeder-index change, and
        // `lookup` says which arm produced it.** Organisms the breeding rule
        // had to look at over the run: the full-slot scan walks every
        // organism in the world, the per-colony index walks that colony's
        // breeders. Both arms count the same way, so the pair is a ratio
        // rather than two numbers.
        //
        // Printed beside `born`/`alive` deliberately: the index changes only
        // HOW an answer is computed, so at one seed and one regime every
        // other field on this line must be identical between the arms. If
        // `born` moves, the index changed behaviour and is wrong -- a
        // whole-run equivalence check that no unit test can match.
        if std::env::var("PIXEL_PHYSICS_BREEDER_INDEX").as_deref() == Ok("scan") { "scan" } else { "index" },
        st.breeder_scan_visits,
        // **B1' (nectar, in two currencies)**
        // (`Reports/evolution-lab-pollinator-design-2026-09-10.md` §3.1,
        // Brief B1'). `flower_visits` is the sensitivity counter, `nectar_
        // paid` the effect (raw joules the plant side handed out, not the
        // gut-filtered credit an animal actually banked -- see `World::
        // nectar_paid`'s own doc for why the two differ). `nectar_j_per_
        // 1000f` is the design's own "joules paid per 1,000 frames" read at
        // this run's own length; re-read it from the per-sample table
        // above for the frame-40,000 figure the brief asks for on a longer
        // run. `organs_built` is B1's own guard: nectar must not starve
        // fruit, so this must not fall against a `nectar_yield: 0` ablation
        // of the same seed.
        world.flower_visits, world.nectar_paid,
        if frames > 0 { world.nectar_paid / (frames as f64 / 1000.0) } else { 0.0 },
        world.organs_built,
        // **P1 (the bloom sense)** -- the "it fired" half beside B1's
        // "it worked" half above: an eyed animal reading `BloomNear > 0`,
        // cumulative over the run. See `World::CreatureStats::bloom_seen`.
        world.creature_stats.bloom_seen,
        // **The rebloom extension, 2026-09-10** (this PR,
        // `Reports/evolution-lab-pollinator-design-2026-09-10.md`'s rebloom
        // brief). `standing_flowers`/`standing_fruit` are the last sample's
        // raw material census -- the effect a picture of the bed would show.
        // `flowers_rebloomed` is `World::flowers_rebloomed`, the "did it
        // fire at all" counter: a bed that merely ran the ordinary
        // once-per-axis route slower would still move `standing_flowers`,
        // and only this says the axes are actually being reused.
        // `organ_ripening_blocked`/`organ_ripening_paid` are the whole
        // organ pipeline's shared refusal/success pair (fruit-set,
        // seed-drop and rebloom all count against the same two lines) --
        // read against the pre-rebloom baseline for whether keeping the
        // flower count up also exploded the refusal rate.
        last.standing_flowers, last.standing_fruit, world.flowers_rebloomed,
        world.organ_ripening_blocked, world.organ_ripening_paid,
        // Round 28's garden-loop instrument, appended rather than woven in
        // -- `labforage`'s SUMMARY line is contested by every lane
        // (`Reports/evolution-lab-round-27-2026-09-10.md`'s own environment
        // note): keep main's fields and append the branch's.
        garden.windfall_floor, garden.windfall_low, garden.windfall_aloft, garden.windfall_shaded, garden.windfall_open,
        // Round 28 garden-fix: the dig verb's own "it fired" counter for
        // routing around a live seed instead of clearing it as spoil --
        // see `World::dig_diverted_seed`'s own doc.
        world.dig_diverted_seed,
        // **P2 (the flitter)** -- the hop's own pair and the two
        // per-species splits.
        //
        // **`CreatureStats::impulses` is already the real launches**, not
        // the attempts: `creature::launch` increments it only on the branch
        // that puts a body in the air and increments `impulses_refused` on
        // the branch that returns false. So the design report's `launches`
        // column is the SUM of the two, and it is printed here as
        // `launch_attempts` rather than left to be reconstructed -- a first
        // draft of this line printed `impulses - refused` as the real
        // launches and read **0 real launches** on an arm whose animals were
        // visibly hopping, the counter-means-what-you-assumed failure
        // `CLAUDE.md` opens its measurement section with.
        //
        // `refused_pct` is printed beside the raw counts because the raw
        // counts are not comparable between arms of different population:
        // an arm with twenty times the animals asks for the verb twenty
        // times as often. The design's own prediction is about the share
        // (*"at 2.0, 60% of every launch is one"*). `starved_aloft` is the
        // verb's own bill (`DeathCause::StarvedInFlight`).
        world.creature_stats.impulses + world.creature_stats.impulses_refused,
        world.creature_stats.impulses,
        world.creature_stats.impulses_refused,
        {
            let asked = world.creature_stats.impulses + world.creature_stats.impulses_refused;
            if asked > 0 { 100.0 * world.creature_stats.impulses_refused as f64 / asked as f64 } else { 0.0 }
        },
        starved_aloft,
        world.creature_stats.flight_frames,
        if fmt_visits.is_empty() { "none".to_string() } else { fmt_visits },
        if fmt_bitten.is_empty() { "none".to_string() } else { fmt_bitten },
        if fmt_alive.is_empty() { "none".to_string() } else { fmt_alive },
        if fmt_deaths.is_empty() { "none".to_string() } else { fmt_deaths },
        // **Rows above the soil, the highest any animal of that species
        // reached at any sampled frame.** Read it against 22, the height of
        // the flower: a `flower_visits` of zero beside a `head_max_rows`
        // short of that is a REACH failure and nothing else, and beside one
        // well past it is a failure of the mouth, the menu or the refill --
        // opposite fixes, and the count alone cannot tell them apart.
        {
            let v = head_max.iter().map(|(k, r)| format!("{k}:{r}")).collect::<Vec<_>>().join(",");
            if v.is_empty() { "none".to_string() } else { v }
        },
        // Round 28's garden-midden build (this lane, appended per the same
        // "keep main's fields, append the branch's" convention the line
        // above already follows): where a delivered pip's final ground
        // stood, read the same two-part test `Behavior::Germinate` runs.
        // `World::pips_set_on_soil`/`pips_set_on_nest`'s own docs.
        world.pips_set_on_soil, world.pips_set_on_nest,
        // **Round 29 B1's own counters** (`Reports/evolution-lab-flight-
        // design-2026-09-11.md` §7), appended rather than woven in, per the
        // same "keep main's fields, append the branch's" convention the
        // block above already follows.
        //
        // **The pair, in the order `CLAUDE.md` asks them to be read.**
        // `fly_ticks` is "it was asked at all" -- brain evaluations made
        // aloft, which is **0 on `main` by construction**, since an airborne
        // animal there did not read the world or evaluate its brain.
        // `fly_frames` is the effect from the far side of the call: airborne
        // frames on which `BrainOutput::Fly` was actually holding the body
        // up. High ticks against zero frames is a wiring problem; zero ticks
        // is no species having priced flight, or nothing having left the
        // ground, and `real_launches` above says which.
        world.creature_stats.fly_ticks,
        world.creature_stats.fly_frames,
        // **`Turn`'s own effect counter, and it exists because of R4.** On
        // the ground both outer candidates lose at every `Turn` value on
        // level footing, so "the weight is authored and the animal is
        // steering" is a false inference this engine has already made once.
        // This counts octant rotations actually applied to a velocity.
        world.creature_stats.fly_turns,
        world.creature_stats.fly_energy,
        // **§Z9's exit firing**: bodies put down because they were
        // weightless and not flying -- standing on water rather than
        // hanging over it. Read beside `deaths_by`'s `STARVED ALOFT` share,
        // which is the number the bug is about; `LAND_AFLOAT=0` puts the
        // defect back and this goes to 0.
        world.creature_stats.landed_afloat,
        // **Walking steps per real launch -- the design's own headline, and
        // it reads 0.60 on `main`.** At that rate the animal turns about
        // once per two hops and each hop carries it ~27 uncontrolled cells,
        // so a flower nine cells away is a 3x overshoot rather than a near
        // miss.
        if world.creature_stats.impulses > 0 { st.moves as f64 / world.creature_stats.impulses as f64 } else { 0.0 },
        // **Frames per launch, read against the 22-frame ballistic arc** for
        // a `Chain(2)` body at launch speed 2.0. Anything well above 22 is
        // airborne time that is not an arc -- §Z9 at the scale the register
        // does not carry -- and `landed_afloat` beside it says whether the
        // fix is what closed it.
        if world.creature_stats.impulses > 0 { world.creature_stats.flight_frames as f64 / world.creature_stats.impulses as f64 } else { 0.0 },
        // The share of airborne frames the verb was paying for, in percent.
        if world.creature_stats.flight_frames > 0 {
            100.0 * world.creature_stats.fly_frames as f64 / world.creature_stats.flight_frames as f64
        } else {
            0.0
        },
        if std::env::var("FLY").as_deref() == Ok("0") { "off".to_string() } else { pixel_physics::sim::creature::flight_speed().to_string() },
        // **Round 29's seed-where-eaten lane's own counters, appended per
        // the same "keep main's fields, append the branch's" convention
        // every block above follows.** `pips_released_by_digestion` is
        // this build's own eaten exit's "it fired"; `fruit_dropped_with_
        // seed` is the uneaten exit's; `seeds_delivered` above (shared
        // with A2) is the union of both plus the pre-existing drop path.
        // The histogram computed just above is the where-eaten
        // distribution the owner's rule is about.
        world.pips_released_by_digestion,
        world.fruit_dropped_with_seed,
        // **Appended at the end, keeping `main`'s fields first and in place**
        // -- this line is contested by every lane and a reordering breaks
        // everyone's parser. The pair is `CLAUDE.md`'s "it fired" counter and
        // its far side: `nest_blends` says cohesion ran, `nest_gap_max` says
        // what it produced.
        world.creature_stats.nest_blends,
        world.creature_stats.share_blends,
        world.nest_sites.len(),
        world.nest_scent_gaps().iter().map(|(_, _, d)| *d).fold(0.0f32, f32::max),
    );
}

/// **The sensitivity half.** A census reading zero because there is nothing
/// there and one reading zero because it is blind print the same line, and
/// `CLAUDE.md`'s standing remedy is to construct the case whose answer is
/// known and watch the instrument report it.
///
/// Plants windfall (960 J face, well clear of the threshold at any gut near
/// neutral) at a known height and a known distance from the nest and asserts
/// every band moves: the total, the height band, the distance band and the
/// visited mask. Each assert is a band that could otherwise be silently
/// always-zero -- which is exactly what `aloft` and `unvisited` would look
/// like on a bed where the finding is real.
fn selftest(spec: LabBox) {
    let bare = LabBox { colonies: 0, founders: 0, ..spec.clone() };
    let (mut world, _) = bare.build_counted();
    let nest_cols = spec.colony_columns();
    let nest = nest_cols[0];
    let wid = world.materials.id_of("windfall").expect("windfall material");
    let visited_none = vec![false; spec.width as usize];
    let mut visited_all = vec![true; spec.width as usize];

    let mut garden0 = Garden::new(spec.width as usize);
    let base = census(&world, &spec, 0.0, &visited_none, &nest_cols, Some(wid), None, None, &mut garden0);
    println!("labforage selftest: empty bed reads edible {} (must be 0)", base.edible);
    assert_eq!(base.edible, 0, "an unplanted bed is not food; the census is counting something it should not");
    assert_eq!(base.windfall, 0, "an unplanted bed has no fallen fruit either; the raw windfall count is counting something it should not");
    // Round 28's garden-loop instrument, same shape: nothing planted means
    // nothing in any of `Garden`'s windfall bands either.
    assert_eq!(garden0.windfall_heat.iter().sum::<u32>(), 0, "an unplanted bed must not heat any column");
    assert_eq!(garden0.windfall_floor + garden0.windfall_low + garden0.windfall_aloft, 0, "an unplanted bed has no windfall to band by height");

    // One cell on the floor beside the nest, one 40 rows up and 200 columns
    // away. The two differ in every band the run's finding turns on.
    let near = (nest + 2, spec.ground_y - 1);
    let far_x = (nest + 200).min(spec.width - 6);
    let far = (far_x, spec.ground_y - 40);
    world.set(near.0, near.1, Cell::new(wid, 0));
    world.set(far.0, far.1, Cell::new(wid, 0));

    let mut garden1 = Garden::new(spec.width as usize);
    let s = census(&world, &spec, 0.0, &visited_none, &nest_cols, Some(wid), None, None, &mut garden1);
    println!(
        "  planted 2 cells (one at the nest on the floor, one {} columns out and 40 rows up): \
         edible {} floor {} low {} aloft {} unvisited {} by_dist {:?} worth {:.0} J windfall {}",
        far.0 - nest, s.edible, s.floor, s.low, s.aloft, s.unvisited, s.by_dist, s.worth, s.windfall
    );
    assert_eq!(s.edible, 2, "the census must see both planted cells");
    assert_eq!(s.floor, 1, "the floor band must see the cell on the floor and only it");
    assert_eq!(s.aloft, 1, "the aloft band must see the cell 40 rows up -- otherwise `aloft 0` means nothing");
    assert_eq!(s.unvisited, 2, "with no column visited, every cell must read unvisited");
    assert!(s.by_dist[0] >= 1, "the near band must see the cell 2 columns from the nest");
    assert!(s.by_dist[2] >= 1 || s.by_dist[3] >= 1, "the far cell must land in a far distance band, not the near one");
    assert!(s.worth > 0.0, "food priced at zero is not food; diet_yield is not reaching the census");
    assert_eq!(s.windfall, 2, "the raw windfall count must see both planted cells regardless of the gut -- it is a material census, not a diet_yield one");
    // **Round 28's garden-loop instrument, the same positive control
    // applied to `Garden`.** Two known cells at known columns, heights and
    // distances must move every one of its bands, or a zero on the played
    // bed would be indistinguishable from blind.
    println!(
        "  garden: windfall_heat[near]={} windfall_heat[far]={} floor {} low {} aloft {} by_dist {:?} shaded {} open {}",
        garden1.windfall_heat[near.0 as usize], garden1.windfall_heat[far.0 as usize],
        garden1.windfall_floor, garden1.windfall_low, garden1.windfall_aloft, garden1.windfall_by_dist,
        garden1.windfall_shaded, garden1.windfall_open
    );
    assert_eq!(garden1.windfall_heat[near.0 as usize], 1, "the near cell's own column must heat by exactly one stop's worth");
    assert_eq!(garden1.windfall_heat[far.0 as usize], 1, "the far cell's own column must heat too, independently of the near one");
    assert_eq!(garden1.windfall_floor, 1, "garden's height band must agree with Sample's: the floor cell is the only floor windfall");
    assert_eq!(garden1.windfall_aloft, 1, "garden's height band must agree with Sample's: the far cell is the only aloft windfall");
    assert_eq!(garden1.windfall_low, 0, "neither planted cell sits in the low band");
    assert!(garden1.windfall_by_dist[0] >= 1, "garden's distance band must agree with Sample's: the near cell is within d<16");
    assert!(
        garden1.windfall_by_dist[2] >= 1 || garden1.windfall_by_dist[3] >= 1,
        "garden's distance band must agree with Sample's: the far cell lands in a far band"
    );
    assert_eq!(
        garden1.windfall_shaded + garden1.windfall_open,
        2,
        "every standing windfall cell must land in exactly one of shaded/open -- a light read that silently drops a cell would show up as a sum under 2"
    );

    // ...and the mask has to be able to go the other way, or `unvisited`
    // would be a constant wearing a measurement's clothes.
    visited_all[..].fill(true);
    let mut garden2 = Garden::new(spec.width as usize);
    let s2 = census(&world, &spec, 0.0, &visited_all, &nest_cols, Some(wid), None, None, &mut garden2);
    println!("  same bed with every column marked visited: unvisited {} (must be 0)", s2.unvisited);
    assert_eq!(s2.unvisited, 0, "the visited mask does not reach the census");
    assert_eq!(s2.edible, 2, "the mask must not change what is counted as food");

    // A gut that cannot digest plants must stop seeing them -- the predicate
    // is the mouth's, so this is the check that the census asks the mouth.
    let mut garden3 = Garden::new(spec.width as usize);
    let s3 = census(&world, &spec, 1.0, &visited_none, &nest_cols, Some(wid), None, None, &mut garden3);
    println!("  same bed read at a pure-flesh gut (bias +1.0): edible {} (must be 0)", s3.edible);
    assert_eq!(s3.edible, 0, "a carnivore's census must not count plants; the gut is not reaching diet_yield");
    assert_eq!(s3.windfall, 2, "the raw windfall count must NOT depend on the gut -- unlike edible, it is a material census");
    assert_eq!(
        garden3.windfall_heat.iter().sum::<u32>(),
        2,
        "garden's windfall census must NOT depend on the gut either, same reasoning as s3.windfall"
    );

    println!("labforage selftest: PASS -- every band moves for a case whose answer is known");
}
