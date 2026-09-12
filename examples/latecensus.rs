//! **The late-game census** -- what the stand, the larder, the colony and the
//! nest's footprint each do on the played bed over a session and beyond.
//!
//! Built for `Reports/evolution-lab-late-game-design-2026-09-12.md` brief 0.
//! The owner's report is about the *late* game -- "when colonies got up to
//! hundreds of creatures ... they decimate all the plants, the food
//! disappears and then the colony dies", and "they dig large chambers
//! underground and piles of dirt/chambers above the nest, which creates an
//! area where plants don't grow". `labforage` censuses the larder and
//! `labnest` the nest, and neither runs the played bed to 500,000 frames
//! with both halves on one row. This does, and only that: one line per
//! stop, every column on it, so a boom, a crash and a dead zone can each be
//! dated to a frame.
//!
//! Three things on a row nothing else prints:
//!
//! * **the larder by kind** -- edible joules split into leaf, fruit (fruit +
//!   windfall), litter, seed (seed + pip), corpse and flower, at the
//!   founders' own gut through `creature::diet_yield`, so "what could a
//!   colony on fruit and carrion alone live on" is read off the bed rather
//!   than argued;
//! * **the nest's footprint** -- `roofed` void below the original surface
//!   (chambers), `pit` (void open to the sky), `packed` cells above and
//!   below the surface (tunnel lining and tamped spoil), `mound` (soil,
//!   packed soil or nest material standing above the original surface) and
//!   its highest row;
//! * **the dead zone** -- columns within `BAND` of a nest with no plant cell
//!   at all, against the same count outside the band, so "plants don't grow
//!   above the nest" is a ratio and not an impression.
//!
//! ```text
//! cargo run --release --example latecensus -- scenario=played_bed frames=500000 sample=20000 seed=1
//! cargo run --release --example latecensus -- scenario=played_bed frames=500000 sample=20000 seed=1 no_colony=1
//! cargo run --release --example latecensus -- control=selftest
//! ```
//!
//! `control=selftest` carves a known roofed cavity, a known open pit and a
//! known mound into a bare box and asserts each column reports exactly it --
//! `aloft` and `unvisited` were the two silently-always-zero columns in
//! `labforage`, and `roofed`/`pit`/`mound` are this file's equivalents.
//!
//! **The census itself moved to `pixel_physics::lab::census`** (PR
//! following `Reports/evolution-lab-late-game-design-2026-09-12.md` brief
//! 0), so the lab's own CENSUS section in its chronicle export
//! (`Lab::write_chronicle`) and this harness read off one function rather
//! than two copies that can drift. This file is now that module's own
//! harness and positive control -- everything below is command-line
//! plumbing and the loop; `Ids`, `Sample`, `census`, `ant_gut_bias` and
//! `colony_deaths` are `pixel_physics::lab::census`'s.

use pixel_physics::lab::census::{self, Ids};
use pixel_physics::lab::scenario::{Placement, Scenario};
use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::creature;
use pixel_physics::sim::organism;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

/// Half-width of the nest band, in columns -- `census::BAND`'s own value,
/// named here too since the module header above still quotes it.
const BAND: i32 = census::BAND;

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

fn selftest() {
    let spec = LabBox { colonies: 0, founders: 0, ..LabBox::default() };
    let mut world = spec.build();
    let ids = Ids::resolve(&world);
    let base = census::census(&world, &spec, 0.0, &[spec.width / 2], &ids);
    println!("latecensus selftest: bare box reads roofed {} pit {} mound {} edible {} (all must be 0)", base.roofed, base.pit, base.mound, base.edible);
    assert_eq!((base.roofed, base.pit, base.mound, base.edible), (0, 0, 0, 0), "a bare box must read nothing");
    assert_eq!(base.bare_in_band, base.band_cols, "a bare box has no plant in any column");
    // A 3x3 chamber ten rows under the surface, roofed by the soil above it.
    let (cx, cy) = (spec.width / 2, spec.ground_y + 10);
    for dy in 0..3 {
        for dx in 0..3 {
            world.set(cx + dx, cy + dy, Cell::EMPTY);
        }
    }
    // A five-deep shaft from the surface, open to the sky, twenty columns away.
    for dy in 0..5 {
        world.set(cx + 20, spec.ground_y + dy, Cell::EMPTY);
    }
    // A two-high heap of packed soil on the surface, thirty columns away.
    let packed = ids.packed.expect("packedsoil is registered");
    world.set(cx + 30, spec.ground_y - 1, Cell::new(packed, 0));
    world.set(cx + 30, spec.ground_y - 2, Cell::new(packed, 0));
    let s = census::census(&world, &spec, 0.0, &[cx], &ids);
    println!(
        "  carved: roofed {} (must be 9) pit {} (must be 5) mound {} (must be 2) high {} (must be 2) packed_above {} (must be 2)",
        s.roofed, s.pit, s.mound, s.mound_high, s.packed_above
    );
    assert_eq!(s.roofed, 9, "a 3x3 chamber under intact soil is nine roofed cells");
    assert_eq!(s.pit, 5, "a five-deep shaft open to the sky is five pit cells");
    assert_eq!((s.mound, s.mound_high, s.packed_above), (2, 2, 2), "two packed cells on the surface are a two-high mound");
    println!("latecensus selftest: PASS -- every footprint column moves for a case whose answer is known");
}

fn main() {
    let control: String = arg("control").unwrap_or_else(|| "run".to_string());
    if control == "selftest" {
        selftest();
        return;
    }
    let frames: u64 = arg("frames").unwrap_or(500_000);
    // **`no_colony=1` -- the paired unfed control**, `labforage`'s own shape:
    // the scenario's timeline still founds the colony (nothing on the
    // scenario side can suppress it), and this clears every creature's cells
    // the frame they land, so the bed's own production -- litter shed, seeds
    // borne, fruit dropped -- is read with nothing eating it.
    let no_colony: bool = arg::<u32>("no_colony").unwrap_or(0) != 0;
    let sample_every: u64 = arg("sample").unwrap_or(20_000);
    let scenario_name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let mut scenario = Scenario::load(&scenario_name).unwrap_or_else(|e| {
        eprintln!("scenario {scenario_name}: {e}");
        std::process::exit(2);
    });
    if let Some(sd) = arg::<u64>("seed") {
        scenario.bed.seed = sd;
    }
    let spec = scenario.bed.clone();
    println!(
        "latecensus: scenario={} seed={} frames={frames} sample={sample_every} band={BAND} threads={}",
        scenario.name,
        spec.seed,
        std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "default".into())
    );
    let (mut world, planted, placed) = scenario.build();
    // **`drift=` -- the ant's `CreatureDef::scent_drift`**, added 2026-09-12 to
    // attribute the played-bed baseline shift to a commit and then to a
    // channel. It defaults to whatever the species file authors, so the
    // shipped arm of this harness is unchanged and only an explicit `drift=`
    // moves anything. Deliberately weaker than patching the constant: the
    // shipped arm has to stay the shipped arm or the pair is not a pair.
    if let Some(v) = arg::<f32>("drift") {
        if let Some(id) = world.species.id_of("ant") {
            let mut def = world.species.get(id).creature.as_ref().expect("creature").clone();
            def.scent_drift = v;
            world.species.set_creature(id, def);
        }
    }
    println!(
        "latecensus: ant scent_drift = {:?}",
        world.species.id_of("ant").and_then(|id| world.species.get(id).creature.as_ref().map(|d| d.scent_drift))
    );
    // **`lifespan=<frames>` -- the sweep knob for brief 2**, written through
    // to the colony species' `CreatureDef::life_half_life` right after the
    // bed is built and before a single frame runs, so every animal the
    // timeline founds at frame 6,000 already has it. `0` is the shipped
    // pre-2026-09-12 behaviour (immortal) and is the paired control every
    // arm of the sweep is read against.
    //
    // Echoed on its own line whether or not it was passed, `plant_probe`'s
    // rule: a log that does not name its lifespan was written by a binary
    // that never had one, and eight byte-identical logs are what that looks
    // like from the outside. Written *after* `drift=` above for the same
    // reason the SUMMARY line appends rather than interleaves: every lane adds
    // a knob here, and one order for all of them is what keeps the diffs small.
    if let Some(v) = arg::<u32>("lifespan") {
        if let Some(id) = world.species.id_of(&spec.colony_species) {
            if let Some(mut def) = world.species.get(id).creature.clone() {
                def.life_half_life = v;
                world.species.set_creature(id, def);
            }
        }
    }
    println!(
        "  {} life_half_life = {} frames (0 = immortal)",
        spec.colony_species,
        world.species.id_of(&spec.colony_species).and_then(|id| world.species.get(id).creature.as_ref().map(|d| d.life_half_life)).unwrap_or(0)
    );
    println!(
        "  bed: {} of {} founders planted; scenario placed {} cells, {} plants, {} animals",
        planted.planted, planted.asked, placed.cells, placed.plants, placed.animals
    );
    let nest_cols: Vec<i32> = {
        let mut v: Vec<i32> = scenario
            .placements
            .iter()
            .chain(scenario.timeline.iter().map(|e| &e.what))
            .filter_map(|p| match p {
                Placement::Colony { x, .. } => Some(*x),
                _ => None,
            })
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    println!("  nests at {nest_cols:?}; band = +-{BAND} columns of a nest\n");
    let ids = Ids::resolve(&world);
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    let mut gut = 0.0f32;
    println!(
        "{:>7} {:>5} {:>5} {:>5} {:>6} {:>9} | {:>8} {:>7} {:>7} {:>6} {:>6} {:>7} {:>7} {:>5} | {:>5} {:>5} {:>5} {:>5} {:>5} {:>4} {:>5} {:>6} {:>6} {:>7} | {:>6} {:>5} {:>6} {:>6} {:>4} | {:>4}/{:<3} {:>4}/{:<3} {:>6} {:>6}",
        "frame", "ants", "plnts", "bank", "edible", "worth(J)",
        "leafJ", "fruitJ", "littrJ", "seedJ", "crpsJ", "flowrJ", "otherJ", "flwrs",
        "born", "died", "strvd", "oldag", "killd", "othr", "crpss", "eats", "digs", "delivs",
        "roofed", "pit", "pack<", "pack^", "mnd", "bare", "band", "bare", "out", "pcIn", "pcOut"
    );
    println!("        (then, cumulative production: shed=leaves shed, borne=seeds borne, germ=germinations, fdrop=fruit dropped)");
    for f in 0..=frames {
        let arrived = pixel_physics::lab::scenario::tick_timeline(&scenario, &mut world, &spec);
        if arrived.animals > 0 {
            gut = census::ant_gut_bias(&world);
            println!("  frame {f}: {} animal(s) arrived on the timeline, founder gut_bias {gut}", arrived.animals);
            if no_colony {
                let cleared = strip_colony(&mut world);
                println!("  frame {f}: no_colony=1, cleared {cleared} colony animal(s) right back off the bed");
            }
        }
        if f % sample_every == 0 {
            let s = census::census(&world, &spec, gut, &nest_cols, &ids);
            let st = world.creature_stats;
            let (starved, killed, oldage, other) = census::colony_deaths(&world, &spec.colony_species);
            println!(
                "{f:>7} {:>5} {:>5} {:>5} {:>6} {:>9.0} | {:>8.0} {:>7.0} {:>7.0} {:>6.0} {:>6.0} {:>7.0} {:>7.0} {:>5} | {:>5} {:>5} {:>5} {:>5} {:>5} {:>4} {:>5} {:>6} {:>6} {:>7} | {:>6} {:>5} {:>6} {:>6} {:>4} | {:>4}/{:<3} {:>4}/{:<3} {:>6} {:>6}",
                s.ants, s.plants, s.seed_bank, s.edible, s.worth,
                s.leaf_j, s.fruit_j, s.litter_j, s.seed_j, s.corpse_j, s.flower_j, s.other_j, s.standing_flowers,
                st.births, st.deaths, starved, oldage, killed, other, s.corpses, st.eats, st.digs, st.deliveries,
                s.roofed, s.pit, s.packed_below, s.packed_above, s.mound_high,
                s.bare_in_band, s.band_cols, s.bare_outside, s.outside_cols, s.plant_cells_in_band, s.plant_cells_outside
            );
            // **Production, cumulative, so a rate is a difference of two
            // rows.** The standing larder above is stock, which is
            // production times residence, and only the rate says what a
            // colony living on the bed's surplus could sustain.
            println!(
                "        shed={} borne={} germ={} fdrop={}",
                world.shed_shade as u64 + world.shed_drought as u64 + world.shed_stranded as u64,
                world.seeds_borne,
                world.germinations,
                world.fruit_dropped
            );
            // **The two numbers that separate the two candidate channels for
            // the 2026-09-12 baseline shift**, measured beside the census
            // rather than inferred from it.
            //
            // `shares` is the trophallaxis channel: `creature::neediest_kin`
            // gates its recipient on `is_living_kin`, which reads the scent
            // predicate, so a drifting colony can only ever feed a shrinking
            // set of its own. `strangers` is the far side of that same
            // predicate read directly -- the share of ORDERED pairs of living
            // ants that are not mutually family, which is what "the colony
            // has become strangers" means as a number rather than as a story.
            // Read the two together: the shift turned out to be the first
            // with the second flat, and the `killd` column zero throughout.
            //
            // **Strided to at most 200 animals, deterministically.** The pair
            // statistic is O(n^2) and this bed reaches 3,000 ants; a stride
            // bounds it and keeps two identical worlds reporting identical
            // numbers, which a random sample would not.
            {
                let ids: Vec<u16> = world
                    .live_organism_ids()
                    .into_iter()
                    .filter(|id| world.organism(*id).is_some_and(|st| world.species.get(st.species).creature.is_some()))
                    .collect();
                let stride = (ids.len() / 200).max(1);
                let sample: Vec<[f32; organism::CREATURE_TRAITS]> =
                    ids.iter().step_by(stride).filter_map(|id| world.organism(*id).map(|st| st.traits)).collect();
                let (mut pairs, mut strangers) = (0u64, 0u64);
                for (i, a) in sample.iter().enumerate() {
                    for (j, b) in sample.iter().enumerate() {
                        if i == j {
                            continue;
                        }
                        pairs += 1;
                        if !creature::scent_accepts(a, b) {
                            strangers += 1;
                        }
                    }
                }
                let mut mean = [0.0f32; 3];
                for t in &sample {
                    for (m, v) in mean.iter_mut().zip(creature::scent_of(t).iter()) {
                        *m += *v;
                    }
                }
                let n = sample.len().max(1) as f32;
                for m in &mut mean {
                    *m /= n;
                }
                let spread: f32 =
                    sample.iter().map(|t| creature::scent_distance_sq(&creature::scent_of(t), &mean).sqrt()).sum::<f32>() / n;
                println!(
                    "        shares={} shared_j={:.0} | sampled={} strangers={:.2}% scent_spread={:.4} (tolerance radius 1.0)",
                    world.creature_stats.shares,
                    world.creature_stats.shared_j,
                    sample.len(),
                    if pairs > 0 { 100.0 * strangers as f64 / pairs as f64 } else { 0.0 },
                    spread
                );
            }
        }
        if f < frames {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        }
    }
    let st = world.creature_stats;
    let final_census = census::census(&world, &spec, gut, &nest_cols, &ids);
    // **`starved`, `killed` and `oldage` stay three fields, never a pooled
    // `died`.** Three mortality channels now move independently on this bed --
    // hunger, the seed-cargo build's `Killed` channel, and age -- and a colony
    // that settled and a colony that ran out of food are the same population
    // line. Only the split tells them apart. Read once here rather than three
    // times inside the argument list, which walked `group_deaths` per field.
    let (starved, killed, oldage, _other) = census::colony_deaths(&world, &spec.colony_species);
    println!(
        // Round 29's fields are appended after main's, format string and
        // argument list in the same order -- every lane adds to this line, so
        // the house rule is append, never interleave.
        "\nSUMMARY scenario={} seed={} frames={frames} born={} died={} eats={} digs={} spoil_dumped={} deliveries={} nectar_paid={:.0} \
         bare_seeds_spared={} bare_seeds_carried={} seeds_carried={} seeds_delivered={} pips_released_by_digestion={} plants_from_pip={} \
         seed_bank={} plants={} ants={} leaf_kj={:.1} litter_kj={:.1} seed_kj={:.1} \
         lifespan={} oldage={} starved={} killed={}",
        scenario.name,
        spec.seed,
        st.births,
        st.deaths,
        st.eats,
        st.digs,
        st.spoil_dumped,
        st.deliveries,
        world.nectar_paid,
        // **Round 29, Brief 1.** `bare_seeds_spared` is the plant side's "it
        // fired" -- a bare-seed bite rolled `seed_gut_survival` and won --
        // and `bare_seeds_carried` the effect counter from the far side of
        // the call: that survivor became a `Crop::passenger` instead of
        // standing where it was bitten. `seeds_carried` beside them is the
        // pre-existing union of the bare and the in-fruit routes, kept so
        // the two can be differenced. `plants_from_pip` is the end of the
        // loop the whole brief is about: a seed an ant carried, set down
        // where the meal ended, that came up as a plant.
        world.bare_seeds_spared,
        world.bare_seeds_carried,
        world.seeds_carried,
        world.seeds_delivered,
        world.pips_released_by_digestion,
        world.plants_from_pip,
        // The final row's standing state, repeated on the SUMMARY line so a
        // sweep can read one line per run rather than parse the table. The
        // leaf larder is here beside the bank because the owner's live-play
        // report is that the colony strips the stand *as well as* the bank,
        // so a build judged on the bank alone would be judged on half of it.
        final_census.seed_bank,
        final_census.plants,
        final_census.ants,
        final_census.leaf_j / 1000.0,
        final_census.litter_j / 1000.0,
        final_census.seed_j / 1000.0,
        // Brief 2's own four, appended last.
        world.species.id_of(&spec.colony_species).and_then(|id| world.species.get(id).creature.as_ref().map(|d| d.life_half_life)).unwrap_or(0),
        oldage,
        starved,
        killed
    );
}
