//! **M0 of the plant community plan** (`Reports/plant-community-plan-2026-10-20.md`
//! §8.1): the baselines every later step is compared against, and the
//! feasibility checks the ladder rests on. One binary, one mode per item.
//!
//! ```text
//! # M0.4-M0.6: one community bed, one seed, censused every `sample` frames
//! cargo run --release --example plant_m0 -- mode=community mix=roles4 seed=1 frames=450000 sample=25000 ants=0
//! # the crippled-species control: shrub's photosynthesis x0.1 at sowing
//! cargo run --release --example plant_m0 -- mode=community mix=roles4 seed=1 cripple=shrub
//! # M0.6(a)'s control: two plants two columns apart against one alone
//! cargo run --release --example plant_m0 -- mode=boxed
//! # M1 A1: two copies of one bed in lockstep, one draw taken from the
//! # copy's shared world stream at frame `at` and nothing else changed
//! cargo run --release --example plant_m0 -- mode=nudge mix=roles4 loadfail=0 at=1000 frames=60000 sample=2000
//! ```
//!
//! **Every unknown argument is refused** (`Reports/instruments.md`: an ignored
//! argument once produced eight byte-identical logs).
//!
//! What a community row counts, per species: `plants` (alive with a shoot;
//! a waiting seed is the bank, counted apart), `leaf` (cells of the species'
//! own leaf material, so grass counts its blades), `est` (plants that first
//! reached 20 shoot cells since the previous row). Per bed: `water` (mean
//! soil water, 0-1 of saturation, by 10-row band below the sown surface),
//! `seed`/`litter` (cells lying above the soil line, and the deepest column),
//! `gaps` (runs of 3+ columns with no plant shoot, per 100 columns),
//! `tips` (shoot growing tips; how many have no free neighbour, split by
//! whether another plant's cells are among those boxing it), `light` (at the
//! soil surface, columns under plant cover against open ones).

use pixel_physics::lab::rain::{Rain, Spells};
use pixel_physics::lab::scenario::{Event, Placement, Scenario};
use pixel_physics::lab::scene::{Ground, LabBox};
use pixel_physics::sim::cell::OrganismId;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::organism::{self, CellType, ParamId};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;
use pixel_physics::train_plant::genome;
use std::collections::{BTreeMap, HashMap};

const KNOWN: &[&str] = &["mode", "mix", "seed", "frames", "sample", "ants", "cripple", "rain", "width", "species", "widths", "lengths", "shots", "out", "loadfail", "at", "drain", "spells", "grow", "blocks", "block", "ground", "below", "axis", "switch"];

fn args() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for a in std::env::args().skip(1) {
        let Some((k, v)) = a.split_once('=') else {
            eprintln!("plant_m0: `{a}` is not key=value");
            std::process::exit(2);
        };
        if !KNOWN.contains(&k) {
            eprintln!("plant_m0: unknown argument `{k}` (known: {})", KNOWN.join(" "));
            std::process::exit(2);
        }
        m.insert(k.to_string(), v.to_string());
    }
    m
}

fn get<T: std::str::FromStr>(m: &BTreeMap<String, String>, k: &str, default: T) -> T {
    m.get(k).map_or(default, |v| v.parse().unwrap_or_else(|_| panic!("{k}={v} does not parse")))
}

/// The two beds of record. **mix7** is the mixed planted bed the grass
/// lifespan was measured on (`grass.ron`); **roles4** is the plan's four
/// roles (§4.2): canopy perennial, shade-tolerant understory, annual
/// colonist, clonal ground cover, starting from the shipped species.
fn mix(name: &str) -> Vec<&'static str> {
    match name {
        "mix7" => vec!["herb", "scrambler", "shrub", "creeper", "grass", "conifer", "tree"],
        "roles4" => vec!["shrub", "creeper", "herb", "grass"],
        other => panic!("mix={other} is not mix7|roles4"),
    }
}

/// Columns for `n` plants spread over the bed, **keeping the band 210..310
/// bare in every arm**, as the played bed does so a colony can seat
/// (`played_bed.ron`); the same layout with and without ants, so the two
/// arms differ only by the colony.
fn columns(n: usize, width: i32) -> Vec<i32> {
    let spans = [(10, 205), (315, width - 10)];
    let total: i32 = spans.iter().map(|(a, b)| b - a).sum();
    (0..n)
        .map(|i| {
            let mut t = (total as f32 * (i as f32 + 0.5) / n as f32) as i32;
            for (a, b) in spans {
                if t < b - a {
                    return a + t;
                }
                t -= b - a;
            }
            spans[1].1 - 1
        })
        .collect()
}

/// A shipped species with both photosynthesis rates x`k`, under its own name,
/// as RON: registered into one bed only, through `LabBox::species_ron`.
fn crippled(name: &str, k: f32) -> String {
    let path = format!("{}/assets/species/{name}.ron", env!("CARGO_MANIFEST_DIR"));
    let mut def = pixel_physics::train_plant::load(std::path::Path::new(&path)).expect("the species loads");
    for cell in [CellType::GrowingTip, CellType::Leaf] {
        let addr = genome::Addr::Param { cell, param: ParamId::PhotosynthesizeRate, tier: 0 };
        if let Some(v) = genome::read(&def, addr) {
            genome::write(&mut def, addr, v * k);
        }
    }
    pixel_physics::sim::species_export::to_ron(&def).expect("exports")
}

fn rain(m: &BTreeMap<String, String>) -> Rain {
    match m.get("rain").map(String::as_str) {
        None | Some("light") => Rain::Light,
        Some("off") => Rain::Off,
        Some("steady") => Rain::Steady,
        Some("heavy") => Rain::Heavy,
        Some(v) => panic!("rain={v} is not off|light|steady|heavy"),
    }
}

/// The community bed `community` and `nudge` both run: built, heredity off,
/// load failure as `loadfail=` says.
fn community_bed(m: &BTreeMap<String, String>) -> (World, LabBox, Scenario, Vec<&'static str>, String, usize) {
    let mix_name: String = get(m, "mix", "roles4".to_string());
    let species = mix(&mix_name);
    let seed: u64 = get(m, "seed", 1);
    let frames: u64 = get(m, "frames", 450_000);
    let ants = get(m, "ants", 0u8) != 0;
    let width: i32 = get(m, "width", 512);
    let cripple = m.get("cripple").cloned();
    let per = if mix_name == "mix7" { 2 } else { 3 };
    let cols = columns(species.len() * per, width);
    let placements: Vec<Placement> =
        cols.iter().enumerate().map(|(i, &x)| Placement::Plant { species: species[i % species.len()].to_string(), x }).collect();
    let bed = LabBox {
        width,
        height: 320,
        soil_depth: 80,
        ground_y: 160,
        founders: 0,
        colonies: 0,
        predators: 0,
        species: species[0].to_string(),
        lamp_spacing: 33,
        seed,
        rain: rain(m),
        // The bed's own default (on since 2026-10-20) unless `drain=0`.
        drain: get(m, "drain", u8::from(LabBox::default().drain)) != 0,
        spells: m.get("spells").map(|v| {
            let (wet, dry) = v.split_once(',').expect("spells=WET,DRY in frames");
            Spells { wet: wet.parse().expect("wet frames"), dry: dry.parse().expect("dry frames") }
        }),
        species_ron: cripple.as_deref().map(|c| crippled(c, 0.1)),
        ground: match m.get("ground").map(String::as_str) {
            None | Some("uniform") => Ground::Uniform,
            Some("patchy") => Ground::Patchy,
            Some("sandy") => Ground::Sandy,
            Some("loam") => Ground::Loam,
            Some("rich") => Ground::Rich,
            Some(v) => panic!("ground={v} is not uniform|patchy|sandy|loam|rich"),
        },
        ..LabBox::default()
    };
    let scenario = Scenario {
        name: format!("m0-{mix_name}"),
        title: "M0".into(),
        question: "M0 community baseline".into(),
        bed: bed.clone(),
        settings: vec![],
        placements,
        timeline: if ants {
            vec![Event { at: 6_000, every: 0, until: 0, what: Placement::Colony { species: "ant".into(), x: 256, count: 52 } }]
        } else {
            vec![]
        },
        horizon: frames,
    };
    let (mut w, _, placed) = scenario.build();
    // In-bed heredity off, as on the bed of record: a census of today's
    // species, not of whatever they mutate into over 450,000 frames.
    w.fate_mutation_chance = 0.0;
    w.param_mutation_chance = 0.0;
    // `loadfail=0` is the owner's own setting (`PLANT_LOAD_FAILURE=false` in
    // the 2026-10-09 playtest, `examples/replay.rs`): a plant with no support
    // path is not felled. On by default, as the lab ships.
    w.plant_load_failure = get(m, "loadfail", 1u8) != 0;
    (w, bed, scenario, species, mix_name, placed.plants)
}

fn community(m: &BTreeMap<String, String>) {
    let started = std::time::Instant::now();
    let (mut w, bed, scenario, species, mix_name, placed) = community_bed(m);
    let (seed, frames, sample) = (bed.seed, get(m, "frames", 450_000u64), get(m, "sample", 25_000u64));
    let ants = get(m, "ants", 0u8) != 0;
    let cripple = m.get("cripple").cloned();
    println!(
        "plant_m0 community: mix={mix_name} {:?} seed={seed} frames={frames} sample={sample} ants={ants} rain={:?} cripple={} plants_placed={} load_failure={}",
        species,
        bed.rain,
        cripple.as_deref().unwrap_or("none"),
        placed,
        w.plant_load_failure
    );
    let (mut particles, mut blasts, tuning) = (ParticleSystem::new(), Blasts::new(), player::Tuning::default());
    let mut established: HashMap<(OrganismId, u64), u64> = HashMap::new();
    // `shots=F,F,..` draws the bed through the shipped renderer at those
    // frames (rows 100 above the soil to 20 below, 2 px a cell) into `out=`.
    let shots: Vec<u64> = m.get("shots").map_or(vec![], |v| v.split(',').map(|x| x.parse().expect("a frame")).collect());
    let out: String = get(m, "out", "plant_m0".to_string());
    let mut renderer = pixel_physics::render::Renderer::new();
    for f in 1..=frames {
        pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bed, Some(&scenario));
        if f % sample == 0 || f == frames {
            let row = census(&w, &bed, &species, &mut established, f, sample);
            println!("{row}");
        }
        if shots.contains(&f) {
            let (fw, fh) = (bed.width as u32, bed.height as u32);
            let mut frame = vec![0u8; (fw * fh * 4) as usize];
            let touched = w.take_touched_chunks();
            renderer.draw(&w, &particles, &touched, &mut frame, (fw, fh), true);
            let below: usize = get(m, "below", 20usize);
            let (top, rows, sc) = ((bed.ground_y - 100).max(0) as usize, 100 + below, 2usize);
            let (tw, th) = (fw as usize * sc, rows * sc);
            let mut img = vec![0u8; tw * th * 4];
            for y in 0..th {
                for x in 0..tw {
                    let src = ((top + y / sc) * fw as usize + x / sc) * 4;
                    img[(y * tw + x) * 4..(y * tw + x) * 4 + 4].copy_from_slice(&frame[src..src + 4]);
                }
            }
            let path = format!("{out}_f{f}.png");
            image::save_buffer(&path, &img, tw as u32, th as u32, image::ColorType::Rgba8).expect("write png");
            println!("plant_m0: wrote {path}");
        }
    }
    println!("plant_m0 done: {:.1} s", started.elapsed().as_secs_f64());
}

fn census(w: &World, bed: &LabBox, species: &[&str], est: &mut HashMap<(OrganismId, u64), u64>, frame: u64, sample: u64) -> String {
    let ground = bed.ground_y;
    let id_of = |n: &str| w.materials.id_of(n);
    let soil: Vec<_> = ["soil", "packedsoil"].iter().filter_map(|n| id_of(n)).collect();
    let seeds: Vec<_> = ["seed", "buriedseed", "reedseed"].iter().filter_map(|n| id_of(n)).collect();
    let litter: Vec<_> = ["litter", "deadleaf"].iter().filter_map(|n| id_of(n)).collect();
    let empty = pixel_physics::sim::material::EMPTY;
    // Per species: plants alive, waiting seeds, leaf cells, newly established.
    let mut per: BTreeMap<&str, [u64; 4]> = species.iter().map(|s| (*s, [0u64; 4])).collect();
    let mut covered = vec![false; bed.width as usize];
    // Per species: (soil water summed over root-adjacent soil cells, cells,
    // deepest root summed over plants, plants).
    let mut roots: BTreeMap<&str, (f64, u64, u64, u64)> = BTreeMap::new();
    let (mut tips, mut boxed_own, mut boxed_other, mut contact, mut up_blocked) = (0u64, 0u64, 0u64, 0u64, 0u64);
    for id in w.live_organism_ids() {
        let Some(st) = w.organism(id) else { continue };
        let def = w.species.get(st.species);
        let name = def.name.as_str();
        let Some(row) = per.get_mut(name) else { continue };
        // **A waiting seed is the bank, not a plant.** Its one cell counts as
        // a shoot cell, so `shoot_cells == 0` missed it and the first census
        // counted ~2,800 grass seeds lying on the bed as 2,890 "plants".
        // A plant is an organism with any cell that is not a seed.
        let germinated = st.cells.keys().any(|&(x, y)| organism::cell_type(w.get(x, y).aux()) != Some(CellType::Seed));
        if st.shoot_cells == 0 || !germinated {
            row[1] += 1;
            continue;
        }
        row[0] += 1;
        let leaf = id_of(&def.leaf_material);
        // **Water where this plant's roots are, and how deep they go**: a
        // shallow and a deep rooter must read differently through a dry
        // spell, or the spells give roots nothing to answer.
        let mut deepest = 0i32;
        for &(x, y) in st.cells.keys() {
            if y < ground {
                continue;
            }
            deepest = deepest.max(y - ground);
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let n = w.get(x + dx, y + dy);
                if soil.contains(&n.material) {
                    let r = roots.entry(name).or_insert((0.0, 0, 0, 0));
                    r.0 += n.aux() as f64 / pixel_physics::sim::material::SOIL_SATURATED as f64;
                    r.1 += 1;
                }
            }
        }
        let r = roots.entry(name).or_insert((0.0, 0, 0, 0));
        r.2 += deepest as u64;
        r.3 += 1;
        for &(x, y) in st.cells.keys() {
            let c = w.get(x, y);
            if Some(c.material) == leaf {
                row[2] += 1;
            }
            if y < ground && (0..bed.width).contains(&x) {
                covered[x as usize] = true;
                if organism::cell_type(c.aux()) == Some(CellType::GrowingTip) {
                    tips += 1;
                    let mut free = false;
                    let mut other = false;
                    let (mut up_full, mut up_other) = (true, false);
                    for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                        let n = w.get(x + dx, y + dy);
                        let theirs = n.organism_id() != 0 && n.organism_id() != id;
                        if n.material == empty {
                            free = true;
                            if dy < 0 {
                                up_full = false;
                            }
                        } else if theirs {
                            other = true;
                            if dy < 0 {
                                up_other = true;
                            }
                        }
                    }
                    // **Contact, not only boxing.** A tip is almost never
                    // walled in on all eight sides -- it steers away first
                    // (crown shyness) -- so "no free neighbour" read zero
                    // even for herbs sown two columns apart. What the slice
                    // does is put another plant *beside* a tip, and close its
                    // way *up*: those are the counts that can fire.
                    if other {
                        contact += 1;
                    }
                    if up_full && up_other {
                        up_blocked += 1;
                    }
                    if !free {
                        if other {
                            boxed_other += 1;
                        } else {
                            boxed_own += 1;
                        }
                    }
                }
            }
        }
        if st.shoot_cells >= 20 {
            if let std::collections::hash_map::Entry::Vacant(e) = est.entry((id, st.born_frame)) {
                e.insert(frame);
                row[3] += 1;
            }
        }
    }
    // Soil water by 10-row band below the sown surface.
    let mut water = Vec::new();
    for band in 0..(bed.soil_depth / 10) {
        let (mut sum, mut n) = (0.0f64, 0u64);
        for y in ground + band * 10..ground + band * 10 + 10 {
            for x in 0..bed.width {
                let c = w.get(x, y);
                if soil.contains(&c.material) {
                    sum += c.aux() as f64 / pixel_physics::sim::material::SOIL_SATURATED as f64;
                    n += 1;
                }
            }
        }
        water.push(if n > 0 { sum / n as f64 } else { -1.0 });
    }
    // **Is there a moisture signal across the bed?** The spread, over
    // columns, of each column's mean soil water in the top band: zero on a
    // uniform bed, which is what a plant has nothing to answer
    // (plan §3.4).
    let top: Vec<f64> = (0..bed.width)
        .filter_map(|x| {
            let (mut sum, mut n) = (0.0f64, 0u64);
            for y in ground..ground + 10 {
                let c = w.get(x, y);
                if soil.contains(&c.material) {
                    sum += c.aux() as f64 / pixel_physics::sim::material::SOIL_SATURATED as f64;
                    n += 1;
                }
            }
            (n > 0).then(|| sum / n as f64)
        })
        .collect();
    let top_mean = top.iter().sum::<f64>() / top.len().max(1) as f64;
    let top_sd = (top.iter().map(|v| (v - top_mean).powi(2)).sum::<f64>() / top.len().max(1) as f64).sqrt();
    // The same spread at every 10-row band, and of the nutrient share a root
    // can use in the top band (`World::soil_nutrient_fraction`): step E's
    // signal census. Columns with no soil in a band are left out.
    let sd = |v: &[f64]| {
        let mu = v.iter().sum::<f64>() / v.len().max(1) as f64;
        (v.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / v.len().max(1) as f64).sqrt()
    };
    let band_sd: Vec<f64> = (0..(bed.soil_depth / 10))
        .map(|band| {
            let cols: Vec<f64> = (0..bed.width)
                .filter_map(|x| {
                    let (mut sum, mut n) = (0.0f64, 0u64);
                    for y in ground + band * 10..ground + band * 10 + 10 {
                        let c = w.get(x, y);
                        if soil.contains(&c.material) {
                            sum += c.aux() as f64 / pixel_physics::sim::material::SOIL_SATURATED as f64;
                            n += 1;
                        }
                    }
                    (n > 0).then(|| sum / n as f64)
                })
                .collect();
            sd(&cols)
        })
        .collect();
    let nut: Vec<f64> = (0..bed.width)
        .filter_map(|x| {
            let (mut sum, mut n) = (0.0f64, 0u64);
            for y in ground..ground + 10 {
                if soil.contains(&w.get(x, y).material) {
                    sum += w.soil_nutrient_fraction(x, y) as f64;
                    n += 1;
                }
            }
            (n > 0).then(|| sum / n as f64)
        })
        .collect();
    let (nut_mean, nut_sd) = (nut.iter().sum::<f64>() / nut.len().max(1) as f64, sd(&nut));
    // Seed and litter lying above the soil line, and the deepest column.
    let (mut seed_cells, mut seed_max, mut litter_cells, mut litter_max, mut seed_live) = (0u64, 0u64, 0u64, 0u64, 0u64);
    for x in 0..bed.width {
        let (mut sc, mut lc) = (0u64, 0u64);
        for y in 0..ground {
            let c = w.get(x, y);
            if c.organism_id() == 0 || seeds.contains(&c.material) {
                if seeds.contains(&c.material) {
                    sc += 1;
                    // A seed cell that still carries an organism is a waiting
                    // seed (the bank); one that does not is spent seed
                    // material lying on the bed.
                    if c.organism_id() != 0 {
                        seed_live += 1;
                    }
                } else if litter.contains(&c.material) {
                    lc += 1;
                }
            }
        }
        seed_cells += sc;
        litter_cells += lc;
        seed_max = seed_max.max(sc);
        litter_max = litter_max.max(lc);
    }
    // Standing water above the soil line, and the deepest column of it. The
    // rain mister was calibrated over 120,000 frames (`lab/rain.rs`); by
    // 450,000 the default bed is under a lake (M0 pictures, 2026-10-20).
    let water_id = id_of("water");
    let (mut standing, mut standing_max) = (0u64, 0u64);
    for x in 0..bed.width {
        let mut col = 0u64;
        for y in 0..ground {
            if Some(w.get(x, y).material) == water_id {
                col += 1;
            }
        }
        standing += col;
        standing_max = standing_max.max(col);
    }
    // **What standing water is standing on**: the cell under each column's
    // lowest water cell, by material, and for soil whether it is full.
    // Water over unsaturated soil should soak in, so a lake over dry
    // ground is lying on something else.
    let mut under: BTreeMap<String, u64> = BTreeMap::new();
    for x in 0..bed.width {
        for y in 0..ground + 2 {
            if Some(w.get(x, y).material) == water_id && Some(w.get(x, y + 1).material) != water_id {
                let below = w.get(x, y + 1);
                let mut name = w.materials.get(below.material).name.clone();
                if soil.contains(&below.material) {
                    name.push_str(if below.aux() >= pixel_physics::sim::material::SOIL_SATURATED { "(full)" } else { "(room)" });
                } else if below.organism_id() != 0 {
                    name.push_str("(plant)");
                }
                *under.entry(name).or_insert(0) += 1;
            }
        }
    }
    let mut under: Vec<(String, u64)> = under.into_iter().collect();
    under.sort_by_key(|e| std::cmp::Reverse(e.1));
    let under = under.iter().take(5).map(|(k, v)| format!("{k}:{v}")).collect::<Vec<_>>().join(",");
    // Gaps: runs of 3+ columns with no plant shoot, per 100 columns.
    let (mut gaps, mut run) = (0u64, 0u64);
    for &c in &covered[8..(bed.width - 8) as usize] {
        if c {
            if run >= 3 {
                gaps += 1;
            }
            run = 0;
        } else {
            run += 1;
        }
    }
    if run >= 3 {
        gaps += 1;
    }
    let gaps_per_100 = gaps as f64 * 100.0 / (bed.width - 16) as f64;
    // Light at the soil surface as a fraction of the same column's light
    // 100 rows up, so a column's distance from a lamp cancels; under a canopy
    // (some plant cell 6+ rows above the soil) against open columns (no
    // plant cell above the soil at all). A grass blade at the soil line is
    // neither: the first version counted it as cover and read the same light
    // under "cover" as in the open.
    let mut canopy = vec![false; bed.width as usize];
    for id in w.live_organism_ids() {
        if let Some(st) = w.organism(id) {
            for &(x, y) in st.cells.keys() {
                if y <= ground - 6 && (0..bed.width).contains(&x) {
                    canopy[x as usize] = true;
                }
            }
        }
    }
    let (mut lc, mut nc, mut lo, mut no) = (0.0f64, 0u64, 0.0f64, 0u64);
    for x in 8..(bed.width - 8) {
        let top = w.field_at_bilinear(x as f32, (ground - 100) as f32).light as f64;
        if top <= 0.0 {
            continue;
        }
        let l = w.field_at_bilinear(x as f32, (ground - 2) as f32).light as f64 / top;
        if canopy[x as usize] {
            lc += l;
            nc += 1;
        } else if !covered[x as usize] {
            lo += l;
            no += 1;
        }
    }
    let mut s = format!("f={frame}");
    for (name, r) in &per {
        let (wet, n, depth, plants) = roots.get(name).copied().unwrap_or((0.0, 0, 0, 0));
        s.push_str(&format!(
            " | {name} plants={} bank={} leaf={} est={} rootwet={:.3} rootdepth={:.1}",
            r[0],
            r[1],
            r[2],
            r[3],
            if n > 0 { wet / n as f64 } else { -1.0 },
            if plants > 0 { depth as f64 / plants as f64 } else { -1.0 }
        ));
    }
    s.push_str(&format!(
        " | water={} topsd={top_sd:.3} bandsd={} nut={nut_mean:.3} nutsd={nut_sd:.3} | standing={standing} standingmax={standing_max} rained={} drained={} vapour={:.0} spell={} on={under} | seed={seed_cells} seedlive={seed_live} seedmax={seed_max} litter={litter_cells} littermax={litter_max} | gaps100={gaps_per_100:.2} | tips={tips} contact_other={contact} up_blocked_other={up_blocked} boxed_own={boxed_own} boxed_other={boxed_other} | light_cov={:.3} n={nc} light_open={:.3} n={no} | window={sample}",
        water.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>().join(","),
        band_sd.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>().join(","),
        w.rain_cells,
        w.drained_water / pixel_physics::sim::material::LIQUID_FULL as u64,
        w.atmospheric_bank,
        bed.spells.map_or("none", |sp| if sp.wet_at(frame) { "wet" } else { "dry" }),
        if nc > 0 { lc / nc as f64 } else { -1.0 },
        if no > 0 { lo / no as f64 } else { -1.0 },
    ));
    s
}

/// M0.6(a)'s control: one plant alone must read zero tips boxed by another
/// plant; two sown two columns apart must read some.
fn boxed(m: &BTreeMap<String, String>) {
    let sp: String = get(m, "species", "shrub".to_string());
    let frames: u64 = get(m, "frames", 8_000);
    // Read before the first seeds land: a lone herb has 25 plants by frame
    // 10,000, and its seedlings are other organisms, so a late sample counts
    // kin as neighbours (measured on the first version of this control).
    let every: u64 = get(m, "sample", 2_000);
    // Centred on a lamp: a lone shrub between two lamps stalls at a seedling
    // (8 leaf cells and no growing tip at frames 10,000 and 20,000), and a
    // plant with no tips reads zero "boxed by another" for the wrong reason.
    let probe = LabBox { width: 256, founders: 0, colonies: 0, predators: 0, species: sp.clone(), seed: 3, ..LabBox::default() };
    let lamp = *probe.lamp_columns().0.iter().min_by_key(|&&c| (c - 128).abs()).expect("a lamp");
    for (label, cols) in [
        ("alone", vec![lamp]),
        ("two columns apart", vec![lamp - 1, lamp + 1]),
        ("touching rows", (-3..=3).map(|k| lamp + 2 * k).collect::<Vec<i32>>()),
    ] {
        let bed = probe.clone();
        let scenario = Scenario {
            name: "m0-boxed".into(),
            title: "M0".into(),
            question: "boxed control".into(),
            bed: bed.clone(),
            settings: vec![],
            placements: cols.iter().map(|&x| Placement::Plant { species: sp.clone(), x }).collect(),
            timeline: vec![],
            horizon: frames,
        };
        let (mut w, _, _) = scenario.build();
        let (mut particles, mut blasts, tuning) = (ParticleSystem::new(), Blasts::new(), player::Tuning::default());
        let mut est = HashMap::new();
        let mut last = String::new();
        for f in 1..=frames {
            pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bed, Some(&scenario));
            if f % every == 0 {
                last = census(&w, &bed, &[sp.as_str()], &mut est, f, every);
                let t = last.split(" | ").find(|p| p.starts_with("tips=")).unwrap_or("");
                let t = t.split(" boxed_own").next().unwrap_or(t);
                let plants = last.split(" | ").find(|p| p.starts_with(sp.as_str())).unwrap_or("");
                println!("  {label:<18} f={f:>6} {t} | {plants}");
            }
        }
        let _ = last;
    }
}


/// The founder's form, read off its own cells: shoot cells, height above the
/// soil, shoot width, leaf share of the shoot, root cells and root depth.
fn form(w: &World, id: OrganismId, ground: i32) -> Option<[f32; 6]> {
    let st = w.organism(id)?;
    let def = w.species.get(st.species);
    let leaf = w.materials.id_of(&def.leaf_material);
    let (mut shoot, mut leafc, mut root) = (0u32, 0u32, 0u32);
    let (mut top, mut deep, mut lo, mut hi) = (ground, ground, i32::MAX, i32::MIN);
    for &(x, y) in st.cells.keys() {
        if y < ground {
            shoot += 1;
            top = top.min(y);
            lo = lo.min(x);
            hi = hi.max(x);
            if Some(w.get(x, y).material) == leaf {
                leafc += 1;
            }
        } else {
            root += 1;
            deep = deep.max(y);
        }
    }
    if shoot == 0 {
        return None;
    }
    Some([shoot as f32, (ground - top) as f32, (hi - lo + 1) as f32, leafc as f32 / shoot as f32, root as f32, (deep - ground) as f32])
}

const FORM: [&str; 6] = ["shoot", "height", "width", "leaf_share", "root", "root_depth"];

fn mean_sd(v: &[f32]) -> (f32, f32) {
    let n = v.len().max(1) as f32;
    let m = v.iter().sum::<f32>() / n;
    let var = v.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / (n - 1.0).max(1.0);
    (m, var.sqrt())
}

/// **M0.2, the surroundings.** One founder in the trainer's lab bed, in four
/// arms: under a lamp or half way between two, with the mister on Light or
/// off, over `seeds` world seeds each; its form at `frames`. For each trait,
/// the shift between arms divided by the spread inside an arm (which here
/// includes the soil's texture, so it is the noise a trainer would face).
fn plastic(m: &BTreeMap<String, String>) {
    use pixel_physics::train_plant::arena::{self, BedSpec, Layout, Offset};
    let sp: String = get(m, "species", "herb".to_string());
    let frames: u64 = get(m, "frames", 20_000);
    let n: u64 = get(m, "seed", 6);
    let spec = BedSpec { frames, ..BedSpec::default() };
    // `axis=ground` (M1 step E): the same founder under the lamp, on a bed
    // that is all sandy, all loam or all rich (`Ground`), with the spells
    // the training world runs. The question is whether the ground moves a
    // plant more than its own dice do.
    if get(m, "axis", "light".to_string()) == "ground" {
        // The fourth arm is the control: loam again on different dice, so
        // "loam B - loam" is what a shift reads when the ground is the same.
        let arms = [("sandy", Ground::Sandy, 0u64), ("loam", Ground::Loam, 0), ("rich", Ground::Rich, 0), ("loam B", Ground::Loam, 1_000)];
        println!("plant_m0 plastic axis=ground: species={sp} frames={frames} seeds={n} (founder alone under a lamp, in-bed heredity off, drain on, spells 20k/40k)");
        let mut table: Vec<Vec<[f32; 6]>> = Vec::new();
        for (label, ground, dice) in arms {
            let mut rows = Vec::new();
            for s in 0..n {
                let mut bed = arena::bed_for(None, &sp, Layout { seed: 5_000_000 + dice + s, offset: Offset::Under }, &spec);
                bed.ground = ground;
                bed.rain = Rain::Steady;
                bed.spells = Some(Spells { wet: 20_000, dry: 40_000 });
                let (mut w, id) = arena::build(&bed, &spec);
                let (mut particles, mut blasts, tuning) = (ParticleSystem::new(), Blasts::new(), player::Tuning::default());
                for _ in 0..frames {
                    pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bed, None);
                }
                if let Some(f) = form(&w, id, bed.ground_y) {
                    rows.push(f);
                }
            }
            print!("  {label:<6} alive {}/{n} |", rows.len());
            for (i, name) in FORM.iter().enumerate() {
                let (mu, sd) = mean_sd(&rows.iter().map(|r| r[i]).collect::<Vec<_>>());
                print!(" {name} {mu:.2}+-{sd:.2}");
            }
            println!();
            table.push(rows);
        }
        for (axis, a, b) in [("rich - sandy", 2usize, 0usize), ("loam - sandy", 1, 0), ("rich - loam", 2, 1), ("loam B - loam", 3, 1)] {
            print!("  {axis:<14}");
            for (i, name) in FORM.iter().enumerate() {
                let (ma, sa) = mean_sd(&table[a].iter().map(|r| r[i]).collect::<Vec<_>>());
                let (mb, sb) = mean_sd(&table[b].iter().map(|r| r[i]).collect::<Vec<_>>());
                let pooled = ((sa * sa + sb * sb) / 2.0).sqrt().max(1e-6);
                print!(" {name} {:+.2}", (ma - mb) / pooled);
            }
            println!("   (shift in units of the spread inside an arm)");
        }
        return;
    }
    let arms = [("bright, wet", Offset::Under, Rain::Light), ("dim, wet", Offset::Between, Rain::Light), ("bright, dry", Offset::Under, Rain::Off), ("dim, dry", Offset::Between, Rain::Off)];
    println!("plant_m0 plastic: species={sp} frames={frames} seeds={n} (founder alone, trainer's lab bed, in-bed heredity off)");
    let mut table: Vec<Vec<[f32; 6]>> = Vec::new();
    for (label, off, r) in arms {
        let mut rows = Vec::new();
        let mut dead = 0;
        for s in 0..n {
            let mut bed = arena::bed_for(None, &sp, Layout { seed: 5_000_000 + s, offset: off }, &spec);
            bed.rain = r;
            let (mut w, id) = arena::build(&bed, &spec);
            let (mut particles, mut blasts, tuning) = (ParticleSystem::new(), Blasts::new(), player::Tuning::default());
            for _ in 0..frames {
                pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bed, None);
            }
            match form(&w, id, bed.ground_y) {
                Some(f) => rows.push(f),
                None => dead += 1,
            }
        }
        print!("  {label:<12} alive {}/{n} |", rows.len());
        for (i, name) in FORM.iter().enumerate() {
            let (mu, sd) = mean_sd(&rows.iter().map(|r| r[i]).collect::<Vec<_>>());
            print!(" {name} {mu:.2}+-{sd:.2}");
        }
        println!("{}", if dead > 0 { format!("  ({dead} dead or never grew)") } else { String::new() });
        table.push(rows);
    }
    // Shift / pooled spread, for light (bright vs dim, both waters) and water (wet vs dry, both lights).
    for (axis, a, b) in [("light: dim - bright", [1usize, 3], [0usize, 2]), ("water: dry - wet", [2, 3], [0, 1])] {
        print!("  {axis:<20}");
        for (i, name) in FORM.iter().enumerate() {
            let pick = |idx: [usize; 2]| idx.iter().flat_map(|&k| table[k].iter().map(move |r| r[i])).collect::<Vec<f32>>();
            let (ma, sa) = mean_sd(&pick(a));
            let (mb, sb) = mean_sd(&pick(b));
            let pooled = ((sa * sa + sb * sb) / 2.0).sqrt().max(1e-6);
            print!(" {name} {:+.2}", (ma - mb) / pooled);
        }
        println!("   (shift in units of the spread inside an arm)");
    }
}

/// **M0.7, what an arena costs**: one herb founder, one thread, by bed size.
fn cost(m: &BTreeMap<String, String>) {
    let widths: Vec<i32> = m.get("widths").map_or(vec![24, 48, 96, 192], |v| v.split(',').map(|x| x.parse().expect("a width")).collect());
    let lengths: Vec<u64> = m.get("lengths").map_or(vec![3_000, 10_000], |v| v.split(',').map(|x| x.parse().expect("a length")).collect());
    println!("plant_m0 cost: one herb founder, single thread (run with RAYON_NUM_THREADS=1)");
    for (h, ground, soil) in [(320, 160, 80), (120, 70, 40)] {
        for &wd in &widths {
            for &fr in &lengths {
                let bed = LabBox { width: wd, height: h, ground_y: ground, soil_depth: soil, founders: 1, colonies: 0, predators: 0, species: "herb".into(), seed: 7, ..LabBox::default() };
                let built = std::panic::catch_unwind(|| bed.build());
                let Ok(mut w) = built else {
                    println!("  {wd:>4} x {h:<3} : refused by LabBox::build");
                    continue;
                };
                let (mut particles, mut blasts, tuning) = (ParticleSystem::new(), Blasts::new(), player::Tuning::default());
                let t = std::time::Instant::now();
                for _ in 0..fr {
                    pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bed, None);
                }
                let secs = t.elapsed().as_secs_f64();
                let plants = w.live_organism_ids().len();
                println!("  {wd:>4} x {h:<3} {fr:>6} frames: {secs:>7.2} s  ({:.2} us per cell-frame, {plants} organisms at the end)", secs * 1e6 / (wd as f64 * h as f64 * fr as f64));
            }
        }
    }
}

/// **M0.8, copying a grown bed.** Two copies of one grown world, each stepped
/// 5,000 frames from fresh particles, must end hash-equal; then a species
/// registered into a copy and sown there must germinate.
fn copy(_m: &BTreeMap<String, String>) {
    let bed = LabBox { width: 96, founders: 2, colonies: 0, predators: 0, species: "herb".into(), seed: 11, ..LabBox::default() };
    let mut w = bed.build();
    let (mut particles, mut blasts, tuning) = (ParticleSystem::new(), Blasts::new(), player::Tuning::default());
    for _ in 0..20_000 {
        pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bed, None);
    }
    let hash = |w: &World| {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for y in 0..bed.height {
            for x in 0..bed.width {
                let c = w.get(x, y);
                for v in [c.material.0 as u64, c.shade as u64, c.aux() as u64, c.organism_id() as u64] {
                    h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
        }
        h
    };
    let run = |mut c: World| {
        let (mut p, mut b) = (ParticleSystem::new(), Blasts::new());
        for _ in 0..5_000 {
            pixel_physics::lab::tick_world(&mut c, &mut p, &mut b, &tuning, &bed, None);
        }
        c
    };
    let (a, b) = (run(w.clone()), run(w.clone()));
    println!("plant_m0 copy: grown 20,000 frames, two copies stepped 5,000 more: hashes {:016x} {:016x} -> {}", hash(&a), hash(&b), if hash(&a) == hash(&b) { "EQUAL" } else { "DIFFERENT" });
    println!("  (the original, carried on with its own particles: {:016x})", hash(&run(w.clone())));
    // A renamed species registered into a copy, and sown into it.
    let path = format!("{}/assets/species/grass.ron", env!("CARGO_MANIFEST_DIR"));
    let mut def = pixel_physics::train_plant::load(std::path::Path::new(&path)).expect("grass loads");
    def.name = "grass_copytest".into();
    let text = pixel_physics::sim::species_export::to_ron(&def).expect("exports");
    let mut c = w.clone();
    let sid = c.species.register_ron(&text).expect("registers into the copy");
    let placed = pixel_physics::lab::scenario::apply_placements(&mut c, &[Placement::Plant { species: "grass_copytest".into(), x: 20 }], &bed);
    let c = run(c);
    let grown: Vec<u32> = c.live_organism_ids().into_iter().filter_map(|id| c.organism(id)).filter(|s| s.species == sid).map(|s| s.shoot_cells).collect();
    println!("  new species registered into the copy and sown: placed {} plant(s); after 5,000 frames {} live, shoot cells {:?}", placed.plants, grown.len(), grown);
}

/// **Does one draw from the shared world stream reshuffle the bed?** Two
/// copies of one community bed, stepped in lockstep. At frame `at` the copy
/// takes one `World::rng` draw and throws it away: no cell, field or
/// organism is touched, so any difference after that is the stream's
/// coupling and nothing else. Each row: cells that differ (material,
/// organism, or water), the columns they fall in, and plants per species in
/// each copy. The first frame at which any cell differs is printed once.
///
/// **Control:** without the nudge (`at=0`) the copies must stay identical
/// for the whole run, or the engine is not reproducible and the row means
/// nothing.
fn nudge(m: &BTreeMap<String, String>) {
    let (mut a, bed, scenario, species, mix_name, _) = community_bed(m);
    let mut b = a.clone();
    let (frames, sample, at) = (get(m, "frames", 60_000u64), get(m, "sample", 2_000u64), get(m, "at", 1_000u64));
    println!("plant_m0 nudge: mix={mix_name} seed={} frames={frames} sample={sample} at={at} load_failure={}", bed.seed, a.plant_load_failure);
    let mut pa = (ParticleSystem::new(), Blasts::new());
    let mut pb = (ParticleSystem::new(), Blasts::new());
    let tuning = player::Tuning::default();
    let mut first: Option<u64> = None;
    // Germinated plants per species, by `census`'s own test.
    let plants = |w: &World| -> Vec<usize> {
        let mut n = vec![0usize; species.len()];
        for id in w.live_organism_ids() {
            let Some(st) = w.organism(id) else { continue };
            let Some(i) = species.iter().position(|s| *s == w.species.get(st.species).name) else { continue };
            if st.shoot_cells > 0 && st.cells.keys().any(|&(x, y)| organism::cell_type(w.get(x, y).aux()) != Some(CellType::Seed)) {
                n[i] += 1;
            }
        }
        n
    };
    for f in 1..=frames {
        if f == at {
            let _ = b.rng.next_u64();
        }
        pixel_physics::lab::tick_world(&mut a, &mut pa.0, &mut pa.1, &tuning, &bed, Some(&scenario));
        pixel_physics::lab::tick_world(&mut b, &mut pb.0, &mut pb.1, &tuning, &bed, Some(&scenario));
        let check = first.is_none() || f % sample == 0 || f == frames;
        if !check {
            continue;
        }
        let (mut cells, mut cols) = (0u32, 0u32);
        for x in 0..bed.width {
            let mut col = false;
            for y in 0..bed.height {
                let (ca, cb) = (a.get(x, y), b.get(x, y));
                if ca.material != cb.material || ca.organism_id() != cb.organism_id() || ca.aux() != cb.aux() {
                    cells += 1;
                    col = true;
                }
            }
            cols += col as u32;
        }
        if cells > 0 && first.is_none() {
            first = Some(f);
            println!("first difference at f={f}: {cells} cells in {cols} columns");
        }
        if f % sample == 0 || f == frames {
            println!("f={f} differ cells={cells} cols={cols} | plants a={:?} b={:?}", plants(&a), plants(&b));
        }
    }
    println!("plant_m0 done: first difference {}", first.map_or("never".to_string(), |f| f.to_string()));
}

/// **What the floor drain costs a frame** (M1 step 2). One bed grown `grow`
/// frames with the arm's own watering, then copied: the copy with the
/// drain off and the copy with it on are stepped in alternating `block`-frame
/// blocks, `blocks` times each, so machine load lands on both alike. Prints
/// microseconds per tick and the soil-water phase's own work counter
/// (`World::soil_water_stats.visited`, cells walked per tick), which does not
/// move with load. Run with `RAYON_NUM_THREADS` pinned.
fn drain_cost(m: &BTreeMap<String, String>) {
    let (mut w, bed, scenario, _, mix_name, _) = community_bed(m);
    let (grow, blocks, block) = (get(m, "grow", 60_000u64), get(m, "blocks", 10u32), get(m, "block", 2_000u64));
    let tuning = player::Tuning::default();
    let (mut p, mut b) = (ParticleSystem::new(), Blasts::new());
    for _ in 0..grow {
        pixel_physics::lab::tick_world(&mut w, &mut p, &mut b, &tuning, &bed, Some(&scenario));
    }
    // `switch=texture` prices step E's two soil rules instead: the same
    // grown bed, one copy with `World::soil_texture` and
    // `soil_drains_around` turned off.
    // `switch=around` prices `soil_drains_around` alone, texture left on.
    let switch: String = get(m, "switch", "drain".to_string());
    let around = switch == "around";
    let texture = switch == "texture" || around;
    let arms = if texture {
        [bed.clone(), bed.clone()]
    } else {
        [LabBox { drain: false, ..bed.clone() }, LabBox { drain: true, ..bed.clone() }]
    };
    let mut off = w.clone();
    if texture {
        off.soil_texture = around;
        off.soil_drains_around = false;
    }
    let mut worlds = [off, w];
    let mut state = [(ParticleSystem::new(), Blasts::new()), (ParticleSystem::new(), Blasts::new())];
    let (mut micros, mut visited, mut changed) = ([0f64; 2], [0u64; 2], [0u64; 2]);
    for _ in 0..blocks {
        for i in 0..2 {
            let t = std::time::Instant::now();
            for _ in 0..block {
                pixel_physics::lab::tick_world(&mut worlds[i], &mut state[i].0, &mut state[i].1, &tuning, &arms[i], Some(&scenario));
                visited[i] += worlds[i].soil_water_stats.visited;
                changed[i] += worlds[i].soil_water_stats.changed;
            }
            micros[i] += t.elapsed().as_secs_f64() * 1e6;
        }
    }
    let ticks = (blocks as u64 * block) as f64;
    println!("plant_m0 draincost: mix={mix_name} seed={} grow={grow} blocks={blocks}x{block} rain={:?} spells={:?}", bed.seed, bed.rain, bed.spells);
    let names = if around {
        ["around off", "around on"]
    } else if texture {
        ["texture off", "texture on"]
    } else {
        ["drain off", "drain on"]
    };
    for (i, name) in names.iter().enumerate() {
        println!(
            "{name:>9}: {:.0} us/tick  soil-water cells walked/tick {:.0}  changed/tick {:.0}  drained {} cells",
            micros[i] / ticks,
            visited[i] as f64 / ticks,
            changed[i] as f64 / ticks,
            worlds[i].drained_water / pixel_physics::sim::material::LIQUID_FULL as u64
        );
    }
}

fn main() {
    let m = args();
    match m.get("mode").map(String::as_str).unwrap_or("community") {
        "community" => community(&m),
        "boxed" => boxed(&m),
        "plastic" => plastic(&m),
        "cost" => cost(&m),
        "copy" => copy(&m),
        "nudge" => nudge(&m),
        "draincost" => drain_cost(&m),
        other => {
            eprintln!("plant_m0: mode={other} is not community|boxed|plastic|cost|copy|nudge|draincost");
            std::process::exit(2);
        }
    }
}
