//! **What the lab's garden does over a long run, and how much of the colony
//! it feeds, and where.** One stop line every `sample=` frames on a lab
//! scenario (default `played_bed`), then a closing table.
//!
//! Built 2026-10-03 for the owner's question *"is the garden balanced, or does
//! it shape the colony in ways that hide ant problems?"* Every other lab
//! harness answers a piece of it from the ant's side (`labforage`: food
//! standing and how high; `colonybooks`: joules by material, but on the
//! harness bed rather than the played one). This one puts the garden and the
//! colony's plate on the same line, at the same stop, on the bed the owner
//! plants:
//!
//! * **The garden**: live plants by species, plant-owned cells by material,
//!   and the two the nest lane asked about -- plant cells standing **in the
//!   nest band above the soil** (on the landing) and **below it** (roots in
//!   the shaft).
//! * **The plate**: joules eaten since the last stop by source material, read
//!   off `ColonyBooks::diet()` -- the same call that credits the animal, so it
//!   cannot disagree with the verb -- and the share eaten off corpses.
//! * **Where**: every time an ant's crop goes from empty to full (a pickup,
//!   seen at the 10-frame tick, so a few cells of walking error at most), the
//!   distance from the nest column and the material. And the face value
//!   digested at the nest against all of it (`CreatureStats::digested_*`).
//! * **The diet gene**: median and range of `TRAIT_GUT_BIAS` over live ants,
//!   because the engine review saw it drift toward plants on 3 of 3 seeds.
//!
//! `colony=0` skips the scenario's timeline, so the colony never lands: the
//! garden alone, paired on the same seed, is what "the colony eats the
//! garden" is measured against.
//!
//! ```text
//! cargo run --release --example labgarden -- seed=1 frames=300000
//! cargo run --release --example labgarden -- seed=1 frames=300000 colony=0
//! ```
use std::collections::BTreeMap;

use pixel_physics::lab::scenario::{Event, Placement, Scenario};
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::organism::TRAIT_GUT_BIAS;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}="))
            .map(|v| v.parse().ok().expect("parses"))
    })
}

/// Distance bands from the nest column, the same edges `labforage` prints.
const DIST_BANDS: [i32; 3] = [16, 48, 128];
/// Half-width of the nest band the landing/shaft counts read.
const NEST_HALF: i32 = 16;

fn band(d: i32) -> usize {
    DIST_BANDS.iter().position(|&b| d < b).unwrap_or(DIST_BANDS.len())
}

fn is_creature(world: &World, sp: pixel_physics::sim::organism::SpeciesId) -> bool {
    world.species.get(sp).creature.is_some()
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(300_000);
    let sample: u64 = arg("sample").unwrap_or(6_000);
    let name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let with_colony = arg::<u32>("colony").unwrap_or(1) != 0;
    let mut sc = Scenario::load(&name).unwrap_or_else(|e| {
        eprintln!("scenario {name}: {e}");
        std::process::exit(2);
    });
    // Seed on the scenario, not a local copy: `Scenario::build` reads
    // `self.bed` (labforage's three-identical-seeds lesson).
    if let Some(sd) = arg::<u64>("seed") {
        sc.bed.seed = sd;
    }
    // **`add=` -- the bed the owner actually plays** (2026-10-03: *"The
    // actual game starts with no creatures or plants and the player gets to
    // choose which creatures and plants get added and can add more over
    // time."*). Strips the scenario's own plants and replaces them with
    // `species:x@frame` entries, comma-separated; `colony_at=` moves the
    // colony's landing frame (default: the scenario's own). So
    // `add=grass:150@30000 colony_at=1` is a colony on an empty bed with one
    // grass planted 106 columns away once the nest exists.
    if let Some(list) = arg::<String>("add") {
        sc.placements.retain(|p| !matches!(p, Placement::Plant { .. }));
        sc.timeline.retain(|e| !matches!(e.what, Placement::Plant { .. }));
        for entry in list.split(',').filter(|e| !e.is_empty()) {
            let (what, at) = entry.split_once('@').unwrap_or((entry, "1"));
            let (species, x) = what.split_once(':').expect("add= wants species:x@frame");
            let at: u64 = at.parse().expect("add= frame parses");
            let x: i32 = x.parse().expect("add= x parses");
            let p = Placement::Plant {
                species: species.to_string(),
                x,
            };
            if at <= 1 {
                sc.placements.push(p);
            } else {
                sc.timeline.push(Event {
                    at,
                    every: 0,
                    until: 0,
                    what: p,
                });
            }
        }
    }
    if let Some(at) = arg::<u64>("colony_at") {
        for e in sc.timeline.iter_mut() {
            if matches!(e.what, Placement::Colony { .. }) {
                e.at = at.max(1);
            }
        }
    }
    // `colony=0` drops only the colony from the timeline, so plants a
    // player adds later still arrive.
    if !with_colony {
        sc.timeline
            .retain(|e| !matches!(e.what, Placement::Colony { .. } | Placement::Colonies { .. }));
    }
    let spec = sc.bed.clone();
    let nest_x: i32 = arg("nest").unwrap_or_else(|| {
        sc.timeline
            .iter()
            .find_map(|e| match &e.what {
                Placement::Colony { x, .. } => Some(*x),
                _ => None,
            })
            .unwrap_or(spec.width / 2)
    });
    let ground_y = spec.ground_y;
    println!(
        "labgarden: scenario={name} add={} seed={} frames={frames} sample={sample} colony={} nest_x={nest_x} ground_y={ground_y} nest band +-{NEST_HALF} | distance bands {DIST_BANDS:?}",
        arg::<String>("add").unwrap_or_else(|| "scenario's own".into()),
        spec.seed,
        if with_colony { "on" } else { "OFF (garden alone)" }
    );
    let (mut world, _, _) = sc.build();
    let rain = spec.rain;
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();

    // Pickups: crop None -> Some, by (distance band, material name).
    let mut had_crop: BTreeMap<u32, bool> = BTreeMap::new();
    let mut pickups_stop: BTreeMap<String, [u64; 4]> = BTreeMap::new();
    let mut pickups_all: BTreeMap<String, [u64; 4]> = BTreeMap::new();
    // Ant-frames spent with a crop aboard (i.e. digesting), by distance band.
    let mut digest_frames = [0u64; 4];
    let mut last_diet: BTreeMap<String, f64> = BTreeMap::new();
    let mut total_intake: BTreeMap<String, f64> = BTreeMap::new();
    let mut ants_peak = 0usize;

    for f in 0..=frames {
        pixel_physics::lab::scenario::tick_timeline(&sc, &mut world, &spec);
        if f % 10 == 0 {
            for id in world.live_organism_ids() {
                let Some(s) = world.organism(id) else { continue };
                if !is_creature(&world, s.species) {
                    continue;
                }
                let Some(&(hx, _)) = s.chain.first() else { continue };
                let b = band((hx - nest_x).abs());
                let now = s.crop.is_some();
                if let Some(c) = &s.crop {
                    digest_frames[b] += 10;
                    if !had_crop.get(&id).copied().unwrap_or(false) {
                        let m = world.materials.get(c.material).name.clone();
                        pickups_stop.entry(m.clone()).or_default()[b] += 1;
                        pickups_all.entry(m).or_default()[b] += 1;
                    }
                }
                had_crop.insert(id, now);
            }
        }
        if f % sample == 0 {
            stop(
                &world,
                f,
                nest_x,
                ground_y,
                (spec.width, spec.height),
                &mut last_diet,
                &mut total_intake,
                &mut pickups_stop,
                &mut ants_peak,
            );
        }
        if f < frames {
            frame::step(
                &mut world,
                &mut particles,
                &mut blasts,
                player::PlayerInput::default(),
                &tuning,
            );
            pixel_physics::lab::rain::tick(&mut world, &spec, rain);
        }
    }

    let total: f64 = total_intake.values().sum();
    println!("\n  colony intake over the run, by source material (J, share):");
    let mut rows: Vec<_> = total_intake.iter().collect();
    rows.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap().then(a.0.cmp(b.0)));
    for (m, j) in &rows {
        println!("    {m:>12} {:>12.0} {:>5.1}%", j, 100.0 * **j / total.max(1.0));
    }
    println!("  pickups over the run, by material and distance from the nest [<16 <48 <128 far]:");
    for (m, b) in &pickups_all {
        println!("    {m:>12} {b:?}");
    }
    let df: u64 = digest_frames.iter().sum();
    println!(
        "  ant-frames digesting by distance [<16 <48 <128 far]: {digest_frames:?} ({:.0}% within 16 of the nest)",
        100.0 * digest_frames[0] as f64 / df.max(1) as f64
    );
    let st = world.creature_stats;
    println!(
        "  digested at nest (AtNest input) {:.0} of {:.0} face J ({:.1}%)",
        st.digested_at_nest_face,
        st.digested_face,
        100.0 * st.digested_at_nest_face / st.digested_face.max(1.0)
    );
    let corpse: f64 = ["corpse", "ant", "brood"]
        .iter()
        .filter_map(|m| total_intake.get(*m))
        .sum();
    let near: u64 = pickups_all.values().map(|b| b[0]).sum();
    let all: u64 = pickups_all.values().map(|b| b.iter().sum::<u64>()).sum();
    println!(
        "GARDEN_SUMMARY seed={} colony={} frames={frames} intake={total:.0} corpse_pct={:.1} pickups={all} pickups_near_pct={:.1} digest_near_pct={:.1} ants_peak={ants_peak}",
        spec.seed,
        with_colony as u8,
        100.0 * corpse / total.max(1.0),
        100.0 * near as f64 / all.max(1) as f64,
        100.0 * digest_frames[0] as f64 / df.max(1) as f64,
    );
}

#[allow(clippy::too_many_arguments)]
fn stop(
    world: &World,
    f: u64,
    nest_x: i32,
    ground_y: i32,
    (width, height): (i32, i32),
    last_diet: &mut BTreeMap<String, f64>,
    total_intake: &mut BTreeMap<String, f64>,
    pickups_stop: &mut BTreeMap<String, [u64; 4]>,
    ants_peak: &mut usize,
) {
    // Plants by species, ants, gut genes.
    let mut plants: BTreeMap<String, usize> = BTreeMap::new();
    let mut ants = 0usize;
    let mut ants_home = 0usize;
    let mut ants_under = 0usize;
    let mut guts: Vec<f32> = Vec::new();
    for id in world.live_organism_ids() {
        let Some(s) = world.organism(id) else { continue };
        let def = world.species.get(s.species);
        if def.creature.is_some() {
            ants += 1;
            guts.push(s.traits[TRAIT_GUT_BIAS]);
            if let Some(&(hx, hy)) = s.chain.first() {
                if (hx - nest_x).abs() < NEST_HALF {
                    ants_home += 1;
                }
                if hy > ground_y {
                    ants_under += 1;
                }
            }
        } else {
            *plants.entry(def.name.clone()).or_default() += 1;
        }
    }
    *ants_peak = (*ants_peak).max(ants);
    guts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let gut = |q: f32| {
        guts.get(((guts.len() as f32 - 1.0) * q).round() as usize)
            .copied()
            .unwrap_or(f32::NAN)
    };

    // Plant-owned cells by material; and the nest band, above and below the
    // soil line.
    let mut cells: BTreeMap<String, usize> = BTreeMap::new();
    let (mut band_above, mut band_below) = (0usize, 0usize);
    let mut total_plant = 0usize;
    let mut loose: BTreeMap<String, usize> = BTreeMap::new();
    let mut col_has = vec![false; width as usize];
    let mut loose_nest: BTreeMap<String, usize> = BTreeMap::new();
    for y in 0..height {
        for x in 0..width {
            let c = world.get(x, y);
            let mname = &world.materials.get(c.material).name;
            // Loose plant food lying about -- leaf litter, shed leaves,
            // crumbs, ungerminated seed (organism-owned, so counted before
            // the organism filter) -- the colony's actual staple, and what
            // the nest lane found filling the door.
            if matches!(
                mname.as_str(),
                "litter" | "deadleaf" | "crumbs" | "seed" | "pip" | "windfall"
            ) {
                *loose.entry(mname.clone()).or_default() += 1;
                if (x - nest_x).abs() < NEST_HALF {
                    *loose_nest.entry(mname.clone()).or_default() += 1;
                }
            }
            let oid = c.organism_id();
            if oid == 0 {
                continue;
            }
            let Some(s) = world.organism(oid) else { continue };
            if is_creature(world, s.species) || s.brood.is_some() {
                continue;
            }
            total_plant += 1;
            col_has[x as usize] = true;
            *cells.entry(world.materials.get(c.material).name.clone()).or_default() += 1;
            if (x - nest_x).abs() < NEST_HALF {
                if y < ground_y {
                    band_above += 1;
                } else {
                    band_below += 1;
                }
            }
        }
    }

    // How far the garden has spread: columns holding any plant, and the
    // nearest plant column to the nest.
    let cols = col_has.iter().filter(|b| **b).count();
    let nearest = col_has
        .iter()
        .enumerate()
        .filter(|(_, b)| **b)
        .map(|(x, _)| (x as i32 - nest_x).abs())
        .min()
        .map_or("none".to_string(), |d| d.to_string());
    // Intake since the last stop, by material.
    let mut diet_now: BTreeMap<String, f64> = BTreeMap::new();
    for books in world.all_colony_books() {
        for (m, j) in books.diet() {
            *diet_now.entry(world.materials.get(m).name.clone()).or_default() += j;
        }
    }
    let mut delta: Vec<(String, f64)> = diet_now
        .iter()
        .map(|(m, j)| (m.clone(), j - last_diet.get(m).copied().unwrap_or(0.0)))
        .filter(|(_, d)| *d > 0.5)
        .collect();
    delta.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
    for (m, d) in &delta {
        *total_intake.entry(m.clone()).or_default() += d;
    }
    *last_diet = diet_now;

    let fmt_map = |m: &BTreeMap<String, usize>| m.iter().map(|(k, v)| format!("{k}:{v}")).collect::<Vec<_>>().join(",");
    let picks = pickups_stop
        .iter()
        .map(|(k, b)| format!("{k}:{}/{}/{}/{}", b[0], b[1], b[2], b[3]))
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "STOP f={f} ants={ants} home={ants_home} under={ants_under} gut_p10/50/90={:.2}/{:.2}/{:.2} plants[{}] plant_cells={total_plant} plant_cols={cols} nearest_to_nest={nearest} nest_band_above={band_above} nest_band_below={band_below} loose[{}] loose_nest[{}] cells[{}] eaten_J[{}] pickups_near/48/128/far[{picks}]",
        gut(0.1),
        gut(0.5),
        gut(0.9),
        fmt_map(&plants),
        fmt_map(&loose),
        fmt_map(&loose_nest),
        fmt_map(&cells),
        delta.iter().map(|(m, d)| format!("{m}:{d:.0}")).collect::<Vec<_>>().join(","),
    );
    pickups_stop.clear();
}
