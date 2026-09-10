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
use pixel_physics::sim::frame;
use pixel_physics::sim::material::MaterialId;
use pixel_physics::sim::organism::TRAIT_GUT_BIAS;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::pheromone::Channel;
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
fn census(world: &World, spec: &LabBox, gut: f32, visited: &[bool], nest_cols: &[i32], windfall_id: Option<MaterialId>) -> Sample {
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

/// Every column any ant head has stood in, ever. Cumulative — the mask is
/// never cleared, so `unvisited` is a claim about the whole run rather than
/// about this frame.
fn mark_visited(world: &World, visited: &mut [bool], width: i32) {
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_none() {
            continue;
        }
        for &(x, _) in &state.chain {
            if (0..width).contains(&x) {
                visited[x as usize] = true;
            }
        }
    }
}

fn main() {
    let control: String = arg("control").unwrap_or_else(|| "run".to_string());
    let frames: u64 = arg("frames").unwrap_or(300_000);
    let sample_every: u64 = arg("sample").unwrap_or(900);
    let handout: u64 = arg("handout").unwrap_or(0);
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
        "labforage: frames={frames} sample={sample_every} founders={} of {} colonies={} walls={} soil={} seed={} handout={handout} ants_at={ants_at}{}",
        spec.founders, spec.species, spec.colonies, spec.compartments, spec.soil_depth, spec.seed,
        scenario.as_ref().map(|s| format!(" scenario={} ({})", s.name, s.question)).unwrap_or_default()
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
    let windfall_id = world.materials.id_of("windfall");
    let mut handed_out = 0u64;
    let mut first: Option<Sample> = None;
    let mut last = Sample::default();
    let mut peak_edible = 0usize;

    println!(
        "{:>7} {:>5} {:>6} {:>7} {:>10} {:>6} {:>6} {:>6} {:>9} | {:>5} {:>5} {:>5} {:>5} | {:>4} {:>5} {:>5} {:>6} | {:>4} {:>4} {:>4} | {:>5} {:>8}",
        "frame", "ants", "plnts", "edible", "worth(J)", "floor", "low", "aloft", "unvisited",
        "d<16", "d<48", "d<128", "far", "high", "eats", "born", "died",
        "brdr", "gen", "bgen", "fvis", "necJ"
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
            println!("  ants_at {ants_at}: founded {ants_placed} ants at {nest_cols:?}, founder gut_bias {gut}\n");
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
                println!("  frame {f}: {} animal(s) arrived on the timeline, founder gut_bias {gut}\n", arrived.animals);
            }
        }
        mark_visited(&world, &mut visited, spec.width);
        if f % sample_every == 0 {
            let s = census(&world, &spec, gut, &visited, &nest_cols, windfall_id);
            peak_edible = peak_edible.max(s.edible);
            if first.is_none() {
                first = Some(s);
            }
            last = s;
            let st = world.creature_stats;
            // One line per sample and every column on it, so the whole run is
            // one greppable block rather than a shape that has to be reread.
            println!(
                "{f:>7} {:>5} {:>6} {:>7} {:>10.0} {:>6} {:>6} {:>6} {:>9} | {:>5} {:>5} {:>5} {:>5} | {:>4} {:>5} {:>5} {:>6} | {:>4} {:>4} {:>4} | {:>5} {:>8.0} | wfall={}",
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
                world.flower_visits, world.nectar_paid, s.windfall
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
    println!(
        "SUMMARY seed={} founders={} colonies={} frames={frames} handout={handout} cols={cols} plants={} windfall={} fruit_dropped={} edible={} unvisited={} floor={} aloft={} \
         peak_edible={peak_edible} eats={} born={} died={} alive={} intake={:.0} burn={:.0} shares={} shared_j={:.0} moves={} deliveries={} nest_visits={} \
         regime={} breeders={} gen={} bgen={} windfall_bitten={} seeds_spilled={} plants_from_pip={} pips_rotted={} pips_eaten={} \
         windfall_bitten_ownerless={} lookup={} visits={} flower_visits={} nectar_paid={:.0} nectar_j_per_1000f={:.2} organs_built={}",
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
        world.organs_built
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

    let base = census(&world, &spec, 0.0, &visited_none, &nest_cols, Some(wid));
    println!("labforage selftest: empty bed reads edible {} (must be 0)", base.edible);
    assert_eq!(base.edible, 0, "an unplanted bed is not food; the census is counting something it should not");
    assert_eq!(base.windfall, 0, "an unplanted bed has no fallen fruit either; the raw windfall count is counting something it should not");

    // One cell on the floor beside the nest, one 40 rows up and 200 columns
    // away. The two differ in every band the run's finding turns on.
    let near = (nest + 2, spec.ground_y - 1);
    let far_x = (nest + 200).min(spec.width - 6);
    let far = (far_x, spec.ground_y - 40);
    world.set(near.0, near.1, Cell::new(wid, 0));
    world.set(far.0, far.1, Cell::new(wid, 0));

    let s = census(&world, &spec, 0.0, &visited_none, &nest_cols, Some(wid));
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

    // ...and the mask has to be able to go the other way, or `unvisited`
    // would be a constant wearing a measurement's clothes.
    visited_all[..].fill(true);
    let s2 = census(&world, &spec, 0.0, &visited_all, &nest_cols, Some(wid));
    println!("  same bed with every column marked visited: unvisited {} (must be 0)", s2.unvisited);
    assert_eq!(s2.unvisited, 0, "the visited mask does not reach the census");
    assert_eq!(s2.edible, 2, "the mask must not change what is counted as food");

    // A gut that cannot digest plants must stop seeing them -- the predicate
    // is the mouth's, so this is the check that the census asks the mouth.
    let s3 = census(&world, &spec, 1.0, &visited_none, &nest_cols, Some(wid));
    println!("  same bed read at a pure-flesh gut (bias +1.0): edible {} (must be 0)", s3.edible);
    assert_eq!(s3.edible, 0, "a carnivore's census must not count plants; the gut is not reaching diet_yield");
    assert_eq!(s3.windfall, 2, "the raw windfall count must NOT depend on the gut -- unlike edible, it is a material census");

    println!("labforage selftest: PASS -- every band moves for a case whose answer is known");
}
