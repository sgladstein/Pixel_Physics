//! **M0 of the plant community plan** (`Reports/plant-community-plan-2026-10-20.md`
//! §8.1): the baselines every later step is compared against, and the
//! feasibility checks the ladder rests on. One binary, one mode per item.
//!
//! ```text
//! # M0.4-M0.6: one community bed, one seed, censused every `sample` frames
//! cargo run --release --example plant_m0 -- mode=community mix=roles4 seed=1 frames=450000 sample=25000 ants=0
//! # the crippled-species control: shrub's photosynthesis x0.1 at sowing
//! cargo run --release --example plant_m0 -- mode=community mix=roles4 seed=1 cripple=shrub
//! # trained species files standing in for shipped roles, under their names
//! cargo run --release --example plant_m0 -- mode=community mix=roles seed=1 swap=shrub:best.ron,tree:t.ron
//! # M0.6(a)'s control: two plants two columns apart against one alone
//! cargo run --release --example plant_m0 -- mode=boxed
//! # M1 A1: two copies of one bed in lockstep, one draw taken from the
//! # copy's shared world stream at frame `at` and nothing else changed
//! cargo run --release --example plant_m0 -- mode=nudge mix=roles4 loadfail=0 at=1000 frames=60000 sample=2000
//! # rungs 5-6's arena: a stand of the other roles grown once, copied per
//! # arm, sown with the role's seed (`none` = the resident-only control)
//! cargo run --release --example plant_m0 -- mode=stand role=tree mix=stand-tree width=192 arms=none,shipped,t.ron
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
use pixel_physics::sim::cell::{Cell, OrganismId};
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::organism::{self, CellType, ParamId};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;
use pixel_physics::train_plant::genome;
use std::collections::{BTreeMap, HashMap};

const KNOWN: &[&str] = &["scald", "endow", "waits", "reserves", "settle", "oldage", "mode", "mix", "seed", "frames", "sample", "ants", "cripple", "rain", "width", "species", "widths", "lengths", "shots", "out", "loadfail", "at", "drain", "spells", "grow", "blocks", "block", "ground", "below", "axis", "switch", "heredity", "pile", "life", "budreserve", "shade", "genes", "dimlamp", "dimspacing", "trace", "pics", "level", "roots", "rotground", "altpal", "rotsoil", "twater", "arm", "traceall", "vary", "devkey", "base", "whys", "swap", "role", "arms", "sow", "window", "gap", "gapat", "gaps", "rootcensus", "beds", "round", "rounds", "pick", "keep", "pmut", "fmut", "garden", "gframes", "save", "load", "pool", "bed"];

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

/// The beds of record. **mix7** is the mixed planted bed the grass
/// lifespan was measured on (`grass.ron`); **roles4** is the plan's four
/// roles (§4.2): canopy perennial, shade-tolerant understory, annual
/// colonist, clonal ground cover, starting from the shipped species.
fn mix(name: &str) -> Vec<&'static str> {
    match name {
        "mix7" => vec!["herb", "scrambler", "shrub", "creeper", "grass", "conifer", "tree"],
        "roles4" => vec!["shrub", "creeper", "herb", "grass"],
        // **The four roles as the review remapped them** (plan §12, M4):
        // canopy perennial (tree), shade-tolerant understory (shrub), annual
        // colonist (herb), clonal ground cover (creeper). Grass is outside
        // the trained four. `roles4` above is M0's mix, from before the
        // remap, and every M1 community run to 2026-10-21 used it.
        "roles" => vec!["tree", "shrub", "herb", "creeper"],
        // **A resident stand for rungs 5-6** (`mode=stand`): the roles
        // without the one being sown into it.
        "stand-tree" => vec!["shrub", "herb", "creeper"],
        "stand-shrub" => vec!["tree", "herb", "creeper"],
        "stand-herb" => vec!["tree", "shrub", "creeper"],
        "stand-creeper" => vec!["tree", "shrub", "herb"],
        // **The pooled roles `played_bed` sows** (`bed=played`): one tree,
        // three shrubs, four herbs; its scrambler and grass are not pooled.
        "played" => vec!["tree", "shrub", "herb"],
        other => panic!("mix={other} is not mix7|roles4|roles|stand-ROLE|played"),
    }
}

/// Columns for `n` plants spread over the bed, **keeping the band 210..310
/// bare in every arm**, as the played bed does so a colony can seat
/// (`played_bed.ron`); the same layout with and without ants, so the two
/// arms differ only by the colony.
fn columns(n: usize, width: i32) -> Vec<i32> {
    // A bed too narrow for the band (rungs 5-6's stand) is spread whole.
    let spans: Vec<(i32, i32)> = if width >= 330 { vec![(10, 205), (315, width - 10)] } else { vec![(10, width - 10)] };
    let total: i32 = spans.iter().map(|(a, b)| b - a).sum();
    (0..n)
        .map(|i| {
            let mut t = (total as f32 * (i as f32 + 0.5) / n as f32) as i32;
            for &(a, b) in &spans {
                if t < b - a {
                    return a + t;
                }
                t -= b - a;
            }
            spans[spans.len() - 1].1 - 1
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
        // `pool=1`: founders from the lab's bed-selected plant pool
        // (`LabBox::plant_pool`, off by default since 2026-10-25).
        plant_pool: get(m, "pool", u8::from(pixel_physics::sim::plant_pool::pool_switch())) != 0,
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
    // `swap=shrub:FILE.ron,tree:FILE.ron`: a trained species file stands in
    // for a shipped role under the shipped name, so the mix, the census and
    // the pictures all read it as that role -- the community bed as the
    // judge of a trainer's champions (plant-community plan §6.2, rung 7).
    // Registered after `build`, which has planted only seeds: a founder
    // reads its species at germination (`seed_genotype`) and every tick
    // after, and the seed material is the role's either way.
    if let Some(spec) = m.get("swap") {
        for item in spec.split(',') {
            let (name, file) = item.split_once(':').expect("swap=NAME:FILE.ron,...");
            let mut def = pixel_physics::train_plant::load(std::path::Path::new(file)).expect("the swapped species loads");
            assert!(w.species.id_of(name).is_some(), "swap: no species {name} in the bed");
            def.name = name.to_string();
            let ron = pixel_physics::sim::species_export::to_ron(&def).expect("exports");
            w.species.register_ron(&ron).expect("the swapped species registers");
            println!("plant_m0 swap: {name} <- {file}");
        }
    }
    // In-bed heredity off, as on the bed of record: a census of today's
    // species, not of whatever they mutate into over 450,000 frames.
    w.fate_mutation_chance = 0.0;
    w.param_mutation_chance = 0.0;
    // **And the other two, which M0 left on** (review M7, plan §12): the
    // per-birth genotype jitter and founder variants. Without these a
    // "census of today's species" was a census of a drifting population.
    // `heredity=on` restores M0's bed for comparison.
    if get(m, "heredity", "off".to_string()) != "on" {
        w.mutation_sigma = 0.0;
        w.founder_variant_chance = 0.0;
    }
    // `loadfail=0` is the owner's own setting (`PLANT_LOAD_FAILURE=false` in
    // the 2026-10-09 playtest, `examples/replay.rs`): a plant with no support
    // path is not felled. On by default, as the lab ships.
    w.plant_load_failure = get(m, "loadfail", 1u8) != 0;
    // `pile=D` (A6): a waiting seed on a heap D deep leaves the slice.
    w.seed_pile_depth = get(m, "pile", w.seed_pile_depth);
    println!("plant_m0 seed_pile_depth={}", w.seed_pile_depth);
    // `budreserve=1` (A3): a plant with no tip breaks a bud on stored carbon.
    w.bud_break_reserve = get(m, "budreserve", u8::from(w.bud_break_reserve)) != 0;
    // `life=shrub:45000,creeper:40000`: a species' median lifespan in
    // plant-time frames, set on this bed only -- the arm the owner's
    // lifespan ruling is measured on before any species file changes.
    if let Some(spec) = m.get("life") {
        for item in spec.split(',') {
            let (name, v) = item.split_once(':').expect("life=NAME:FRAMES,...");
            let id = w.species.id_of(name).unwrap_or_else(|| panic!("life: no species {name}"));
            w.species.get_mut(id).life_half_life = v.parse().expect("life frames");
        }
    }
    apply_shade(m, &mut w);
    // `level=1`: `World::soil_capillary_levels` (soil water levels sideways
    // when saturated). `roots=1`: `World::soil_through_roots` (soil water
    // passes through a root to the soil beyond). The standing-water arms.
    w.soil_capillary_levels = get(m, "level", u8::from(w.soil_capillary_levels)) != 0;
    w.soil_through_roots = get(m, "roots", u8::from(w.soil_through_roots)) != 0;
    // `rotground=1`: `World::decay_reads_ground` (litter under a plant reads
    // the ground's wetness).
    w.decay_reads_ground = get(m, "rotground", u8::from(w.decay_reads_ground)) != 0;
    // `rotsoil=0`: the closed soil budget (`LabBox::rot_makes_soil` off) --
    // rot leaves no soil and a dead root in a mat repays its own.
    if get(m, "rotsoil", 1u8) == 0 {
        w.rot_soil_yield = 0.0;
        w.root_mats_repay = true;
    }
    // `twater=1`: `World::texture_water` (a plant reads soil water on the
    // texture's own scale; acts only with `ground=patchy` or a textured bed).
    w.texture_water = get(m, "twater", u8::from(w.texture_water)) != 0;
    println!(
        "plant_m0 soil_capillary_levels={} soil_through_roots={} decay_reads_ground={} rot_soil_yield={} root_mats_repay={} soil_texture={} texture_water={}",
        w.soil_capillary_levels, w.soil_through_roots, w.decay_reads_ground, w.rot_soil_yield, w.root_mats_repay, w.soil_texture, w.texture_water
    );
    println!(
        "plant_m0 bud_break_reserve={} lifespans: {}",
        w.bud_break_reserve,
        species.iter().map(|n| format!("{n}={}", w.species.id_of(n).map_or(0.0, |id| w.species.get(id).life_half_life))).collect::<Vec<_>>().join(" ")
    );
    println!(
        "plant_m0 heredity: fate_mutation={} param_mutation={} mutation_sigma={} founder_variants={}",
        w.fate_mutation_chance, w.param_mutation_chance, w.mutation_sigma, w.founder_variant_chance
    );
    (w, bed, scenario, species, mix_name, placed.plants)
}

/// `shade=1` turns on `World::shade_plasticity` (A5), and
/// `genes=SPECIES:avoid|tolerate:VALUE,...` sets a species' shade genes on
/// this bed only -- the A/B the shade genes are measured on before any
/// species file changes. `scald=1` turns on `World::shade_scald` (A5's
/// bright-light price). Prints what it set.
fn apply_shade(m: &BTreeMap<String, String>, w: &mut World) {
    w.shade_plasticity = get(m, "shade", u8::from(w.shade_plasticity)) != 0;
    w.shade_scald = get(m, "scald", u8::from(w.shade_scald)) != 0;
    w.seedling_waits = get(m, "waits", u8::from(w.seedling_waits)) != 0;
    // `reserves=1`: `World::seedling_reserves`.
    w.seedling_reserves = get(m, "reserves", u8::from(w.seedling_reserves)) != 0;
    // `settle=1`: `World::seed_settles`.
    w.seed_settles = get(m, "settle", u8::from(w.seed_settles)) != 0;
    // `oldage=1`: `World::old_age_plant_time`.
    w.old_age_plant_time = get(m, "oldage", u8::from(w.old_age_plant_time)) != 0;
    if let Some(spec) = m.get("genes") {
        for item in spec.split(',') {
            let parts: Vec<&str> = item.split(':').collect();
            let [name, gene, value] = parts[..] else { panic!("genes=SPECIES:avoid|tolerate|germ:VALUE,...") };
            let id = w.species.id_of(name).unwrap_or_else(|| panic!("genes: no species {name}"));
            // `germ`: the seed's `Germinate.light_threshold` (absolute
            // light, open 2.40), the first of the three walls a seed meets
            // under a canopy -- a gene, so selection can move it.
            let (cell_type, param) = match gene {
                "avoid" => (CellType::GrowingTip, ParamId::ShadeAvoid),
                "tolerate" => (CellType::GrowingTip, ParamId::ShadeTolerate),
                "germ" => (CellType::Seed, ParamId::GerminateLight),
                other => panic!("genes: {other} is not avoid|tolerate|germ"),
            };
            assert!(w.species.set_param(id, cell_type, param, 0, value.parse().expect("gene value")), "genes: {name} has no {gene} gene");
        }
    }
    println!("plant_m0 shade_plasticity={} shade_scald={} open_light={:.2} genes={}", w.shade_plasticity, w.shade_scald, w.open_light, m.get("genes").map_or("none", String::as_str));
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
    let gaps = get(m, "gaps", 0u64);
    let mut renderer = pixel_physics::render::Renderer::new();
    // Soil-moisture writes a tick, per sample window: the churn
    // `World::soil_capillary_levels` was held off for (`examples/waterstand`).
    let (mut sw_changed, mut sw_ticks) = (0u64, 0u64);
    // `rootcensus=1`'s history: every 1,000 frames, each plant's root cells,
    // drinking roots, shoot cells and how many of its non-root cells touch
    // soil or water (the stem cells `Absorb` also drinks through). Printed
    // as `ROOTLOG` rows at each shot for the plants with no drinking root.
    let rootlog_on = get(m, "rootcensus", 0u8) != 0;
    // (frame, root cells, drinking roots, shoot cells, stem cells on the ground)
    type RootRow = (u64, u32, u32, u32, u32);
    let mut rootlog: HashMap<OrganismId, Vec<RootRow>> = HashMap::new();
    let mut root_seen: HashMap<OrganismId, Vec<(i32, i32)>> = HashMap::new();
    let mut root_loss: BTreeMap<(String, bool), u64> = BTreeMap::new();
    for f in 1..=frames {
        pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bed, Some(&scenario));
        // What a small plant's root cell became when the plant lost it --
        // the "how does a plant end up rootless" half of the probe. Every
        // 200 frames, plants with 1-12 root cells; a vanished root's cell is
        // tallied by what now stands there (another plant, litter, soil...).
        if rootlog_on && f % 200 == 0 {
            for id in w.live_organism_ids() {
                let Some(st) = w.organism(id) else { continue };
                let now: Vec<(i32, i32)> = st
                    .cells
                    .keys()
                    .copied()
                    .filter(|&(x, y)| {
                        let c = w.get(x, y);
                        w.materials.get(c.material).reinforces_powder || organism::cell_type(c.aux()) == Some(CellType::RootTip)
                    })
                    .collect();
                if let Some(prev) = root_seen.get(&id) {
                    for &(x, y) in prev {
                        if !now.contains(&(x, y)) {
                            let c = w.get(x, y);
                            let what = if c.organism_id() == id {
                                "own-shoot(retargeted)".to_string()
                            } else if c.organism_id() != 0 {
                                format!("other-plant:{}", w.materials.get(c.material).name)
                            } else {
                                w.materials.get(c.material).name.to_string()
                            };
                            let lost_all = now.is_empty();
                            *root_loss.entry((what, lost_all)).or_default() += 1;
                        }
                    }
                }
                if !now.is_empty() && now.len() <= 12 {
                    root_seen.insert(id, now);
                } else {
                    root_seen.remove(&id);
                }
            }
        }
        if rootlog_on && f % 1000 == 0 {
            for id in w.live_organism_ids() {
                let Some(st) = w.organism(id) else { continue };
                let mut wet_stem = 0u32;
                for &(x, y) in st.cells.keys() {
                    let c = w.get(x, y);
                    if w.materials.get(c.material).reinforces_powder || organism::cell_type(c.aux()) == Some(CellType::RootTip) {
                        continue;
                    }
                    if [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| {
                        let n = w.get(x + dx, y + dy);
                        n.organism_id() == 0 && (w.materials.get(n.material).water_capacity > 0 || w.materials.get(n.material).kind == pixel_physics::sim::material::MaterialKind::Liquid)
                    }) {
                        wet_stem += 1;
                    }
                }
                rootlog.entry(id).or_default().push((f, st.root_cells, st.contact_root_cells, st.shoot_cells, wet_stem));
            }
        }
        sw_changed += w.soil_water_stats.changed;
        sw_ticks += 1;
        if f % sample == 0 || f == frames {
            let row = census(&w, &bed, &species, &mut established, f, sample);
            println!("{row} | sw={:.1}", sw_changed as f64 / sw_ticks.max(1) as f64);
            (sw_changed, sw_ticks) = (0, 0);
        }
        // `gaps=N`: canopy gaps, every N frames -- the floor's light per
        // column (two rows above the ground, as a digit 0-9 for 0-9% of open
        // light and `+` above), and every tree: `TREE id span x0-x1 cells
        // senescent`. Built 2026-10-22 for the owner's "gaps from tree
        // death": how wide a dead tree's gap is, and how fast the crowns
        // around it close it again.
        if gaps > 0 && f % gaps == 0 {
            let row: String = (0..bed.width)
                .map(|x| {
                    let pct = pixel_physics::sim::plant::ambient_light_above(&w, x, bed.ground_y - 2) / w.open_light * 100.0;
                    if pct >= 10.0 { '+' } else { char::from(b'0' + pct as u8) }
                })
                .collect();
            println!("GAPS f={f} {row}");
            let tree = w.species.id_of("tree");
            for id in w.live_organism_ids() {
                let Some(st) = w.organism(id) else { continue };
                if Some(st.species) != tree || st.shoot_cells < 20 {
                    continue;
                }
                let (x0, x1) = st.cells.keys().fold((i32::MAX, i32::MIN), |(a, b), &(x, _)| (a.min(x), b.max(x)));
                println!(
                    "TREE f={f} id={id} span={x0}-{x1} cells={} senescent={} age wall={} ticks={} plant={}",
                    st.cells.len(),
                    st.senescent,
                    w.frame - st.born_frame,
                    st.age_ticks as u64 * pixel_physics::sim::plant::ORGANISM_TICK_INTERVAL,
                    st.plant_frames
                );
            }
        }
        if shots.contains(&f) {
            println!("{}", screen_mix(&w, &bed, f));
            println!("{}", water_why(&w, &bed, f));
            let below: usize = get(m, "below", 20usize);
            let path = format!("{out}_f{f}.png");
            shot(&mut w, &particles, &mut renderer, &bed, below, &path);
            // `rootcensus=1`: where every plant's roots are, one row a plant,
            // and the same crop redrawn by owner (`owner_shot`) -- the probe
            // behind "plants that seem to have no roots but survive".
            if get(m, "rootcensus", 0u8) != 0 {
                root_census(&w, &bed, below, f);
                println!("ROOTLOSS f={f} (became, plant left rootless) {:?} roots_required={} adventitious={} dieback_kept={}", root_loss, w.roots_required, w.adventitious_roots, w.dieback_roots_kept);
                for id in w.live_organism_ids() {
                    let Some(st) = w.organism(id) else { continue };
                    if st.contact_root_cells != 0 || st.shoot_cells < 5 {
                        continue;
                    }
                    let h = rootlog.get(&id).map_or(&[][..], |v| v.as_slice());
                    let max_roots = h.iter().map(|e| e.1).max().unwrap_or(0);
                    let last_rooted = h.iter().rev().find(|e| e.2 > 0).map_or(0, |e| e.0);
                    let first = h.first().map_or(0, |e| e.0);
                    let tail: Vec<String> = h.iter().rev().step_by(5).take(12).map(|e| format!("{}:{}/{}/{}/{}", e.0 / 1000, e.1, e.2, e.3, e.4)).collect();
                    println!("ROOTLOG f={f} id={id} first={first} max_roots={max_roots} last_drinking_root={last_rooted} recent(kframe:roots/contact/shoot/wetstem)={}", tail.join(" "));
                }
                owner_shot(&w, &bed, below, &format!("{out}_f{f}_owners.png"));
            }
            // `altpal=1`: the same world, the same frame, drawn again with
            // brighter foliage and seeds and litter toned toward the soil --
            // a colour A/B with nothing else different (`alt_palette`).
            // `altpal=2` also draws a third version with roots toned toward
            // the soil too (`_alt2`), to tell which tan reads as "seeds".
            let alts: u8 = get(m, "altpal", 0u8);
            for level in 1..=alts.min(2) {
                let saved = alt_palette(&mut w, level >= 2);
                let path = format!("{out}_f{f}_alt{}.png", if level >= 2 { "2" } else { "" });
                shot(&mut w, &particles, &mut renderer, &bed, below, &path);
                for (id, pal) in saved {
                    w.materials.get_mut(id).palette = pal;
                }
            }
        }
    }
    println!("plant_m0 done: {:.1} s", started.elapsed().as_secs_f64());
}

/// One `ROOTS` row a living plant: where its shoot meets the ground, how
/// much root it has and where, what the shot's `below` rows can show of it,
/// and its water and nutrient terms. `hidden` is root tissue deeper than the
/// picture; `visible` is root tissue inside it.
fn root_census(w: &World, bed: &LabBox, below: usize, frame: u64) {
    let ground = bed.ground_y;
    let floor = ground + below as i32;
    for id in w.live_organism_ids() {
        let Some(st) = w.organism(id) else { continue };
        if !st.cells.keys().any(|&(x, y)| organism::cell_type(w.get(x, y).aux()) != Some(CellType::Seed)) {
            continue;
        }
        let name = w.species.get(st.species).name.clone();
        let (mut roots, mut visible, mut hidden, mut leaf, mut shoot) = (0u32, 0u32, 0u32, 0u32, 0u32);
        let (mut deepest, mut root_x0, mut root_x1) = (i32::MIN, i32::MAX, i32::MIN);
        let (mut lowest_shoot, mut lowest_shoot_x) = (i32::MIN, 0i32);
        let (mut shoot_x0, mut shoot_x1) = (i32::MAX, i32::MIN);
        let mut roots_above = 0u32;
        // Soil faces the roots touch (what nutrient is read over), and the
        // plant's foliage: leaves and growing tips, as `organism_upkeep`
        // counts them -- so a row can say how much root a leaf has behind it.
        let (mut nfaces, mut foliage) = (0u32, 0u32);
        for &(x, y) in st.cells.keys() {
            let c = w.get(x, y);
            let ty = organism::cell_type(c.aux());
            if matches!(ty, Some(CellType::Leaf) | Some(CellType::GrowingTip)) {
                foliage += 1;
            }
            if w.materials.get(c.material).reinforces_powder || ty == Some(CellType::RootTip) {
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let n = w.get(x + dx, y + dy);
                    let m = w.materials.get(n.material);
                    if m.kind == pixel_physics::sim::material::MaterialKind::Powder && m.water_capacity > 0 {
                        nfaces += 1;
                    }
                }
            }
        }
        for &(x, y) in st.cells.keys() {
            let c = w.get(x, y);
            let ty = organism::cell_type(c.aux());
            let root = w.materials.get(c.material).reinforces_powder || ty == Some(CellType::RootTip);
            if root {
                roots += 1;
                deepest = deepest.max(y);
                root_x0 = root_x0.min(x);
                root_x1 = root_x1.max(x);
                if y < ground {
                    roots_above += 1;
                } else if y < floor {
                    visible += 1;
                } else {
                    hidden += 1;
                }
            } else {
                shoot += 1;
                if ty == Some(CellType::Leaf) {
                    leaf += 1;
                }
                if y > lowest_shoot {
                    (lowest_shoot, lowest_shoot_x) = (y, x);
                }
                shoot_x0 = shoot_x0.min(x);
                shoot_x1 = shoot_x1.max(x);
            }
        }
        println!(
            "ROOTS f={frame} id={id} sp={name} shoot={shoot} leaf={leaf} shootx={shoot_x0}..{shoot_x1} base=({lowest_shoot_x},{}) roots={roots} visible={visible} hidden={hidden} above={roots_above} rootx={} deepest={} contact={} ws={:.3} desic={:.3} rzw={:.3} nut={:.3} stock={:.2} uptake={:.4} demand={:.4} nfaces={nfaces} foliage={foliage}",
            lowest_shoot - ground,
            if roots > 0 { format!("{root_x0}..{root_x1}") } else { "-".into() },
            if roots > 0 { (deepest - ground).to_string() } else { "-".into() },
            st.contact_root_cells,
            st.water_status,
            st.water_desiccation,
            st.root_zone_water,
            st.nutrient_status,
            st.water,
            st.water_uptake,
            st.water_demand,
        );
    }
}

/// The `shot` crop redrawn by owner, a full replace on fixed colours (no
/// blend into the cell's own palette): each plant one hue from its id, its
/// root tissue at half brightness, soil dark grey, water dark blue, the rest
/// black. A root and the shoot it feeds share a hue, so a shoot with no
/// root in the picture is a hue with no dim twin below the line.
fn owner_shot(w: &World, bed: &LabBox, below: usize, path: &str) {
    let (top, rows, sc) = ((bed.ground_y - 100).max(0), 100 + below as i32, 2usize);
    let (tw, th) = (bed.width as usize * sc, rows as usize * sc);
    let mut img = vec![0u8; tw * th * 4];
    for cy in 0..rows {
        for cx in 0..bed.width {
            let (x, y) = (cx, top + cy);
            let c = w.get(x, y);
            let mat = w.materials.get(c.material);
            let id = c.organism_id();
            let rgb: [u8; 3] = if id != 0 && w.organism(id).is_some_and(|st| st.cells.contains_key(&(x, y))) {
                let h = (id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40;
                let hue = (h % 360) as f32;
                let (r, g, b) = hsv(hue, 0.85, 1.0);
                let root = mat.reinforces_powder || organism::cell_type(c.aux()) == Some(CellType::RootTip);
                let k = if root { 0.5 } else { 1.0 };
                [(r * k * 255.0) as u8, (g * k * 255.0) as u8, (b * k * 255.0) as u8]
            } else if c.material == pixel_physics::sim::material::EMPTY {
                [0, 0, 0]
            } else if mat.kind == pixel_physics::sim::material::MaterialKind::Liquid {
                [20, 30, 90]
            } else if mat.water_capacity > 0 {
                [45, 45, 45]
            } else {
                [25, 25, 25]
            };
            for py in 0..sc {
                for px in 0..sc {
                    let o = ((cy as usize * sc + py) * tw + cx as usize * sc + px) * 4;
                    img[o..o + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
                }
            }
        }
    }
    image::save_buffer(path, &img, tw as u32, th as u32, image::ColorType::Rgba8).expect("write png");
    println!("plant_m0: wrote {path}");
}

fn hsv(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (r + m, g + m, b + m)
}

/// Draw the bed through the shipped renderer and save rows 100 above the
/// soil to `below` under it, 2 px a cell.
fn shot(w: &mut World, particles: &ParticleSystem, renderer: &mut pixel_physics::render::Renderer, bed: &LabBox, below: usize, path: &str) {
    let (fw, fh) = (bed.width as u32, bed.height as u32);
    let mut frame = vec![0u8; (fw * fh * 4) as usize];
    let touched = w.take_touched_chunks();
    renderer.draw(w, particles, &touched, &mut frame, (fw, fh), true);
    let (top, rows, sc) = ((bed.ground_y - 100).max(0) as usize, 100 + below, 2usize);
    let (tw, th) = (fw as usize * sc, rows * sc);
    let mut img = vec![0u8; tw * th * 4];
    for y in 0..th {
        for x in 0..tw {
            let src = ((top + y / sc) * fw as usize + x / sc) * 4;
            img[(y * tw + x) * 4..(y * tw + x) * 4 + 4].copy_from_slice(&frame[src..src + 4]);
        }
    }
    image::save_buffer(path, &img, tw as u32, th as u32, image::ColorType::Rgba8).expect("write png");
    println!("plant_m0: wrote {path}");
}

/// The colour arm of the "does the bed look like plants" A/B, render only:
/// foliage brighter and a little more saturated (HSV value x1.45, saturation
/// x1.15), and loose seed and litter pulled 55% toward the soil's own brown so
/// they read as ground rather than as a scatter of grains. Returns the
/// palettes it replaced.
fn alt_palette(w: &mut World, roots: bool) -> Vec<(pixel_physics::sim::material::MaterialId, Vec<[u8; 4]>)> {
    fn hsv_lift(c: [u8; 4]) -> [u8; 4] {
        let (r, g, b) = (c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let v = max;
        let s = if max > 0.0 { (max - min) / max } else { 0.0 };
        let h = if max == min {
            0.0
        } else if max == r {
            ((g - b) / (max - min)).rem_euclid(6.0)
        } else if max == g {
            (b - r) / (max - min) + 2.0
        } else {
            (r - g) / (max - min) + 4.0
        };
        let (v, s) = ((v * 1.45).min(1.0), (s * 1.15).min(1.0));
        let c2 = v * s;
        let x = c2 * (1.0 - ((h % 2.0) - 1.0).abs());
        let (r1, g1, b1) = match h as i32 {
            0 => (c2, x, 0.0),
            1 => (x, c2, 0.0),
            2 => (0.0, c2, x),
            3 => (0.0, x, c2),
            4 => (x, 0.0, c2),
            _ => (c2, 0.0, x),
        };
        let mm = v - c2;
        [((r1 + mm) * 255.0) as u8, ((g1 + mm) * 255.0) as u8, ((b1 + mm) * 255.0) as u8, c[3]]
    }
    let soil = [68.0f32, 50.0, 37.0];
    let toward_soil = |c: [u8; 4]| -> [u8; 4] {
        let k = 0.55;
        [(c[0] as f32 * (1.0 - k) + soil[0] * k) as u8, (c[1] as f32 * (1.0 - k) + soil[1] * k) as u8, (c[2] as f32 * (1.0 - k) + soil[2] * k) as u8, c[3]]
    };
    let mut saved = Vec::new();
    let mut names = vec![("leaf", true), ("grassblade", true), ("seed", false), ("buriedseed", false), ("litter", false), ("pip", false), ("windfall", false)];
    if roots {
        names.extend([("rootwood", false), ("grassroot", false), ("reedroot", false)]);
    }
    for (name, lift) in names {
        let Some(id) = w.materials.id_of(name) else { continue };
        let old = w.materials.get(id).palette.clone();
        let new: Vec<[u8; 4]> = old.iter().map(|&c| if lift { hsv_lift(c) } else { toward_soil(c) }).collect();
        w.materials.get_mut(id).palette = new;
        saved.push((id, old));
    }
    saved
}

/// **What the picture is made of**: every cell in the window a shot draws
/// (100 rows above the bed's original soil line, 20 below), tallied by
/// material -- organism cells by what they are (`leaf`, `wood`, `root`,
/// ...), soil above the original line as `heaped`, and empty or water below
/// it as `hole`/`water-in-ground`. Percentages of the non-empty cells above
/// the line, then the top 20 rows of ground. Paired with each shot, because
/// "it looks like a pile of water and seeds" is a claim about these shares.
fn screen_mix(w: &World, bed: &LabBox, frame: u64) -> String {
    let mut above: BTreeMap<String, u64> = BTreeMap::new();
    let mut below: BTreeMap<String, u64> = BTreeMap::new();
    for y in (bed.ground_y - 100).max(0)..(bed.ground_y + 20) {
        for x in 0..bed.width {
            let c = w.get(x, y);
            let mat = w.materials.get(c.material);
            let ground = y >= bed.ground_y;
            let label = if c.material == pixel_physics::sim::material::EMPTY {
                if ground { "hole".to_string() } else { continue }
            } else if c.organism_id() != 0 && w.organism(c.organism_id()).is_some_and(|st| st.cells.contains_key(&(x, y))) {
                match organism::cell_type(c.aux()) {
                    Some(CellType::Leaf) => "leaf".to_string(),
                    Some(CellType::RootTip) => "root-tip".to_string(),
                    Some(CellType::Seed) => "seed(live)".to_string(),
                    Some(CellType::Flower) | Some(CellType::Fruit) => "flower/fruit".to_string(),
                    Some(t) => format!("{}:{t:?}", mat.name),
                    None => format!("{}(organism)", mat.name),
                }
            } else if mat.kind == pixel_physics::sim::material::MaterialKind::Liquid {
                if ground { "water-in-ground".to_string() } else { "water".to_string() }
            } else if !ground && mat.water_capacity > 0 {
                "heaped-soil".to_string()
            } else {
                mat.name.to_string()
            };
            *(if ground { &mut below } else { &mut above }).entry(label).or_default() += 1;
        }
    }
    let fmt = |t: &BTreeMap<String, u64>| {
        let n: u64 = t.values().sum::<u64>().max(1);
        let mut v: Vec<(&String, &u64)> = t.iter().collect();
        v.sort_by_key(|e| std::cmp::Reverse(*e.1));
        format!("n={n} ") + &v.iter().map(|(k, c)| format!("{k}:{:.1}%", **c as f64 * 100.0 / n as f64)).collect::<Vec<_>>().join(" ")
    };
    // Where heaped ground comes from: every soil-family cell in the bed,
    // against the rots so far (`decay.rs`; 5% of a rotted litter, seed or
    // corpse cell comes back as soil).
    let soil_cells: u64 = (0..bed.height).map(|y| (0..bed.width).filter(|&x| w.materials.get(w.get(x, y).material).water_capacity > 0).count() as u64).sum();
    format!(
        "SCREEN f={frame} soilcells={soil_cells} rotted={} above[{}] || ground-top20[{}]",
        w.decayed_damp as u64 + w.decayed_dry as u64,
        fmt(&above),
        fmt(&below)
    )
}

/// **Why is the standing water standing?** Every water cell above the
/// soil line by what it rests on, and for those on plant matter,
/// `update::drip_through_organism`'s downward scan replayed (as
/// `examples/waterstand` does it, at `update::DRIP_REACH`) to name what refused the drop.
/// `examples/waterstand`'s census, on this harness's beds.
fn water_why(w: &World, bed: &LabBox, frame: u64) -> String {
    use pixel_physics::sim::material::{self, MaterialKind};
    use pixel_physics::sim::update;
    let Some(water) = w.materials.id_of("water") else { return String::new() };
    let mut t: BTreeMap<String, u64> = BTreeMap::new();
    let plantish = |c: pixel_physics::sim::cell::Cell| c.organism_id() != 0 || w.materials.kind(c.material) == MaterialKind::Plant;
    for y in 0..bed.ground_y {
        for x in 0..bed.width {
            let c = w.get(x, y);
            if c.material != water {
                continue;
            }
            let below = w.get(x, y + 1);
            let key = if below.material == material::EMPTY {
                "falling".to_string()
            } else if below.material == water {
                "on-water".to_string()
            } else if plantish(below) {
                let mut why = "on-plant:too-deep".to_string();
                // `update::DRIP_REACH` (32, `MAX_REACH`), mirrored.
                for probe in (y + 1)..(y + 33) {
                    let here = w.get(x, probe);
                    if here.material == material::EMPTY {
                        why = "on-plant:can-drip".to_string();
                        break;
                    }
                    if plantish(here) {
                        continue;
                    }
                    if here.material == water {
                        let room = (material::LIQUID_FULL + material::LIQUID_MAX_COMPRESS).saturating_sub(update::liquid_fill(here));
                        why = if room > 0 && !here.managed() { "on-plant:can-drip" } else { "on-plant:held-by-full-water" }.to_string();
                        break;
                    }
                    let cap = w.materials.get(here.material).water_capacity;
                    why = if cap == 0 {
                        format!("on-plant:held-by-{}", w.materials.get(here.material).name)
                    } else if update::soil_moisture(here) >= cap {
                        // What stops that soil draining: straight below,
                        // and the two diagonals `soil_drains_around` tries.
                        let held = |c: pixel_physics::sim::cell::Cell| {
                            let m = w.materials.get(c.material);
                            if c.material == material::EMPTY {
                                "air"
                            } else if plantish(c) {
                                "plant"
                            } else if m.water_capacity > 0 {
                                if update::soil_moisture(c) >= m.water_capacity { "fullsoil" } else { "soil" }
                            } else {
                                "other"
                            }
                        };
                        *t.entry(format!("  satsoil-under:{}/{}/{}", held(w.get(x - 1, probe + 1)), held(w.get(x, probe + 1)), held(w.get(x + 1, probe + 1)))).or_default() += 1;
                        "on-plant:held-by-saturated-soil".to_string()
                    } else {
                        "on-plant:can-drip".to_string()
                    };
                    break;
                }
                why
            } else if w.materials.get(below.material).water_capacity > 0 {
                if update::soil_moisture(below) >= w.materials.get(below.material).water_capacity { "on-saturated-soil" } else { "on-soil-with-room" }.to_string()
            } else {
                format!("on-{}", w.materials.get(below.material).name)
            };
            *t.entry(key).or_default() += 1;
        }
    }
    let n: u64 = t.values().sum();
    let mut v: Vec<(String, u64)> = t.into_iter().collect();
    v.sort_by_key(|e| std::cmp::Reverse(e.1));
    format!("WATER f={frame} standing={n} {}", v.iter().map(|(k, c)| format!("{k}:{c}")).collect::<Vec<_>>().join(" "))
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
    // **What a tip touches, split** (plan §8.1 step 4): a waiting seed, a
    // plant of its own kind, or a plant of another kind. `contact` above is
    // all three; a pile of seed reads as crowding in it.
    let (mut touch_seed, mut touch_kin, mut touch_foreign) = (0u64, 0u64, 0u64);
    // Per species: noon-equivalent light summed over its own leaf cells,
    // and the count -- the light its leaves actually get.
    let mut leaf_light: BTreeMap<&str, (f64, u64)> = BTreeMap::new();
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
                let l = leaf_light.entry(name).or_insert((0.0, 0));
                // The day divided out only where the sky lights the world:
                // the lab is lit by lamps with no daily cycle, and dividing
                // by the sky's night amplitude read every leaf as full sun.
                let raw = w.field_at(x, y).light;
                let lit = if w.sky_lighting() { pixel_physics::sim::field::noon_equivalent_light(raw, w.frame) } else { raw };
                l.0 += lit as f64 / pixel_physics::sim::field::MAX_LIGHT as f64;
                l.1 += 1;
            }
            if y < ground && (0..bed.width).contains(&x) {
                covered[x as usize] = true;
                if organism::cell_type(c.aux()) == Some(CellType::GrowingTip) {
                    tips += 1;
                    let mut free = false;
                    let mut other = false;
                    let (mut t_seed, mut t_kin, mut t_foreign) = (false, false, false);
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
                            match w.organism(n.organism_id()) {
                                Some(o) if o.dormant_seed || organism::cell_type(n.aux()) == Some(CellType::Seed) => t_seed = true,
                                Some(o) if o.species == st.species => t_kin = true,
                                _ => t_foreign = true,
                            }
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
                    touch_seed += t_seed as u64;
                    touch_kin += t_kin as u64;
                    touch_foreign += t_foreign as u64;
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
        let (ll, ln) = leaf_light.get(name).copied().unwrap_or((0.0, 0));
        s.push_str(&format!(
            " | {name} plants={} bank={} leaf={} est={} rootwet={:.3} rootdepth={:.1} leaflight={:.3}",
            r[0],
            r[1],
            r[2],
            r[3],
            if n > 0 { wet / n as f64 } else { -1.0 },
            if plants > 0 { depth as f64 / plants as f64 } else { -1.0 },
            if ln > 0 { ll / ln as f64 } else { -1.0 }
        ));
    }
    s.push_str(&format!(
        " | water={} topsd={top_sd:.3} bandsd={} nut={nut_mean:.3} nutsd={nut_sd:.3} | standing={standing} standingmax={standing_max} rained={} drained={} vapour={:.0} spell={} on={under} | seed={seed_cells} seedlive={seed_live} seedmax={seed_max} spread={} litter={litter_cells} littermax={litter_max} | gaps100={gaps_per_100:.2} | tips={tips} contact_other={contact} touch_seed={touch_seed} touch_kin={touch_kin} touch_foreign={touch_foreign} up_blocked_other={up_blocked} boxed_own={boxed_own} boxed_other={boxed_other} | light_cov={:.3} n={nc} light_open={:.3} n={no} | window={sample}",
        water.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>().join(","),
        band_sd.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>().join(","),
        w.rain_cells,
        w.drained_water / pixel_physics::sim::material::LIQUID_FULL as u64,
        w.atmospheric_bank,
        bed.spells.map_or("none", |sp| if sp.wet_at(frame) { "wet" } else { "dry" }),
        w.seeds_spread,
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
/// Render the whole bed once and save a `w x h` cell window at `(x0, y0)`,
/// each cell `scale` pixels square.
#[allow(clippy::too_many_arguments)]
fn crop_shot(w: &World, particles: &ParticleSystem, bw: i32, bh: i32, x0: i32, y0: i32, cw: i32, ch: i32, scale: usize, path: &str) {
    let (fw, fh) = (bw as u32, bh as u32);
    let mut frame = vec![0u8; (fw * fh * 4) as usize];
    let mut renderer = pixel_physics::render::Renderer::new();
    renderer.draw(w, particles, &Default::default(), &mut frame, (fw, fh), true);
    let (tw, th) = (cw as usize * scale, ch as usize * scale);
    let mut img = vec![0u8; tw * th * 4];
    for y in 0..th {
        for x in 0..tw {
            let (sx, sy) = (x0 + (x / scale) as i32, y0 + (y / scale) as i32);
            if (0..bw).contains(&sx) && (0..bh).contains(&sy) {
                let src = (sy as usize * fw as usize + sx as usize) * 4;
                img[(y * tw + x) * 4..(y * tw + x) * 4 + 4].copy_from_slice(&frame[src..src + 4]);
            }
        }
    }
    image::save_buffer(path, &img, tw as u32, th as u32, image::ColorType::Rgba8).expect("write png");
}

/// Scale every grow lamp's `beam` by `factor`, and rewrite each fixture so
/// its field block is rescanned with the new value (`rebuild_blocked` reads
/// `beam` only from blocks written since their last scan).
fn dim_lamps(w: &mut World, factor: f32, width: i32, height: i32) {
    let Some(lamp) = w.materials.id_of("growlamp") else {
        return;
    };
    if factor == 1.0 {
        return;
    }
    w.materials.get_mut(lamp).beam *= factor;
    for y in 0..height {
        for x in 0..width {
            let c = w.get(x, y);
            if c.material == lamp {
                w.set(x, y, Cell::EMPTY);
                w.set(x, y, c);
            }
        }
    }
}

/// One line of a founder's ledger (`plastic axis=shade trace=N`).
fn trace_founder(w: &World, id: OrganismId, label: &str) {
    let Some(st) = w.organism(id) else {
        println!("    TRACE {label} dead");
        return;
    };
    // A founder still one cell is a seed that has not sprouted: print what
    // `Behavior::Germinate` reads -- light at the seed, and what is below it.
    if st.cells.len() == 1 {
        let &(x, y) = st.cells.keys().next().expect("one cell");
        let (here, below) = (w.get(x, y), w.get(x, y + 1));
        println!(
            "    TRACE {label} seed at ({x},{y}) {} type {:?} light {:.2} | below {} holds {} available {:.2} owned {}",
            w.materials.get(here.material).name,
            organism::cell_type(here.aux()),
            pixel_physics::sim::plant::ambient_light_above(w, x, y),
            w.materials.get(below.material).name,
            w.materials.get(below.material).water_capacity,
            pixel_physics::sim::update::plant_available_fraction(below),
            below.organism_id() != 0
        );
    }
    let (mut leaves, mut tips, mut buds, mut carbon, mut light) = (0, 0, 0, 0.0f32, 0.0f32);
    for &(x, y) in st.cells.keys() {
        carbon += w.carbon_at(x, y);
        match organism::cell_type(w.get(x, y).aux()) {
            Some(CellType::Leaf) => {
                leaves += 1;
                light += pixel_physics::sim::plant::ambient_light_above(w, x, y) / pixel_physics::sim::field::MAX_LIGHT;
            }
            Some(CellType::GrowingTip) => tips += 1,
            Some(CellType::DormantBud) => buds += 1,
            _ => {}
        }
    }
    println!(
        "    TRACE {label} cells {} leaves {leaves} light {:.2} tips {tips} buds {buds} roots {} | income {:.2} bill {:.2} unpaid {:.2} starving {} | carbon {carbon:.1} endow {:.2} water {:.2} nut {:.2} senescent {}",
        st.cells.len(),
        if leaves > 0 { light / leaves as f32 } else { 0.0 },
        st.root_cells,
        st.income,
        st.maintenance,
        st.maintenance_unpaid,
        st.starving_ticks,
        st.endowment,
        st.water_status,
        st.nutrient_status,
        st.senescent
    );
    // `PIXEL_PHYSICS_TRACE_CELLS=1`: a small plant cell by cell -- what each
    // cell is and the carbon it holds, which is where a seedling's stake
    // went when its tip reads too poor to grow.
    if std::env::var("PIXEL_PHYSICS_TRACE_CELLS").is_ok() && st.cells.len() <= 32 {
        let mut cells: Vec<_> = st.cells.keys().copied().collect();
        cells.sort_by_key(|&(x, y)| (y, x));
        let row: Vec<String> = cells
            .iter()
            .map(|&(x, y)| format!("{:?}@{},{}={:.2}", organism::cell_type(w.get(x, y).aux()).unwrap_or(CellType::Seed), x, y, w.carbon_at(x, y)))
            .collect();
        println!("    CELLS {label} {}", row.join(" "));
    }
}

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
    // `axis=shade` (M1 A5): the same founder under the same lamp, bright
    // and dimmed (`dimlamp=`, the lamp's `beam` scaled), with the shade
    // genes ignored and read. The light at its own leaves is printed, the
    // control that "dim" is dim -- M0's between-lamps arm was not.
    //
    // **Dimmed, not moved between lamps, and the first version did the
    // latter.** With lamps 66 apart and the founder midway, the bed read
    // 0.26 of full at the planting site and every founder died -- with or
    // without the genes. A per-plant trace (`trace=`) showed why: the
    // seedling's tip read light **0.00** from its first tick, because
    // between beams the only light is `field::step_diffusion`'s sideways
    // bleed, and that skips any block holding a `Plant` or `Solid` cell
    // (`rebuild_blocked`). Empty air beside the plant reads 0.26; the plant
    // reads nothing. The probe measured the air, the plant lived in the
    // block. At the played spacing (33) the beams overlap and a founder
    // between them reads 0.45, so the game is not dark; a gap beyond the
    // beams is. Light through a canopy arrives from above, so a dimmer lamp
    // is the honest dim.
    if get(m, "axis", "light".to_string()) == "shade" {
        println!("plant_m0 plastic axis=shade: species={sp} frames={frames} seeds={n} genes={}", m.get("genes").map_or("none", String::as_str));
        let mut table: Vec<Vec<[f32; 6]>> = Vec::new();
        // `dimlamp=`: the dim arm's lamps at this fraction of their beam.
        // `dimspacing=N` instead puts the dim founder midway between full
        // lamps N apart -- kept as the reproduction of
        // `open-bugs-handoff.md` §Z39 (66 is dark to a plant), not as a dim.
        let dim: f32 = get(m, "dimlamp", 0.4);
        let gap: Option<i32> = m.get("dimspacing").and_then(|v| v.parse().ok());
        let (dim_off, dim_spacing, dim_lamp) = match gap {
            Some(g) => (Offset::Between, g, 1.0),
            None => (Offset::Under, 33, dim),
        };
        for (label, off, spacing, plastic, lamp) in [
            ("bright, off", Offset::Under, 33, false, 1.0),
            ("dim, off", dim_off, dim_spacing, false, dim_lamp),
            ("bright, on", Offset::Under, 33, true, 1.0),
            ("dim, on", dim_off, dim_spacing, true, dim_lamp),
        ] {
            // `arm=bright-off` runs that arm alone (labels with ", " -> "-").
            if m.get("arm").is_some_and(|a| *a != label.replace(", ", "-")) {
                continue;
            }
            let (mut rows, mut lights) = (Vec::new(), Vec::new());
            // Wide enough for the dim column (half of 132 from the nearest
            // lamp) to exist, and the same in both arms.
            let wide = BedSpec { width: 288, ..spec };
            for s in 0..n {
                // `vary=` splits the spread (plan §3.2's decomposition):
                // `world` (default) varies the world seed with the founder
                // fixed -- under the lab's `DevelopmentalKey::World` the
                // plant's own draws do not read the world seed, so this is
                // the environment's dice; `column` fixes the world and moves
                // the founder one column per seed -- the plant's dice (its
                // draws are keyed on position) in one environment; `none`
                // repeats one plant, the determinism control.
                let vary: String = get(m, "vary", "world".to_string());
                // `base=`: the world seed the fixed arms use (and the first
                // of the varied one).
                let base: u64 = get(m, "base", 5_000_000u64);
                let world_seed = if vary == "world" { base + s } else { base };
                let mut bed = arena::bed_for(None, &sp, Layout { seed: world_seed, offset: off }, &wide);
                bed.lamp_spacing = spacing;
                let shift = if vary == "column" { s as i32 } else { 0 };
                bed.founder_at = Some(vec![arena::founder_column(&bed, off) + shift]);
                let (mut w, id) = arena::build(&bed, &wide);
                dim_lamps(&mut w, lamp, bed.width, bed.height);
                // **The light probe runs on a copy.** It stepped the light
                // field 30 times on seed 0's own world, so seed 0 grew a
                // different plant from the same inputs on any later seed
                // (measured: one world repeated 16 times read 206 cells on
                // seed 0 and 513 on the other 15). Every earlier run of this
                // arm carries that on seed 0, the same in each paired arm.
                if s == 0 {
                    let fx = arena::founder_column(&bed, off);
                    let mut probe = w.clone();
                    for _ in 0..30 {
                        pixel_physics::sim::field::step(&mut probe);
                    }
                    println!("    {label}: light at the planting site (3 rows up) {:.2} of full", pixel_physics::sim::plant::ambient_light_above(&probe, fx, bed.ground_y - 3) / pixel_physics::sim::field::MAX_LIGHT);
                }
                // `devkey=N`: `DevelopmentalKey::Plant { coarseness: N }`
                // instead of the lab's `World` key (A4's question).
                if let Some(c) = m.get("devkey") {
                    w.developmental_key = pixel_physics::sim::organism::DevelopmentalKey::Plant { coarseness: c.parse().expect("devkey=N") };
                }
                let mut args = m.clone();
                args.insert("shade".into(), u8::from(plastic).to_string());
                apply_shade(&args, &mut w);
                // `endow=X`: the founder seed carries X carbon, as a seed its
                // parent bred would (`Reproduce.seed_cost`), instead of a
                // planted seed's 0. Under a canopy the first step is paid
                // from this or not at all.
                if let Some(e) = m.get("endow") {
                    pixel_physics::sim::plant::endow_seed(&mut w, id, e.parse().expect("endow=X"));
                }
                let (mut particles, mut blasts, tuning) = (ParticleSystem::new(), Blasts::new(), player::Tuning::default());
                // `trace=N`: the first seed's founder every N frames -- its
                // organs, its earnings against its bill, and its stores.
                // Why a plant dies in the shade is a question about one
                // plant's ledger over its life, not about the census.
                let trace: u64 = get(m, "trace", 0);
                // `whys=1`: under each trace row, why the founder's shoot
                // tips did or did not grow since the last one (`GrowWhy`,
                // from `World::grow_log`) -- a stalled seedling's tip that
                // never retires is counted, so the census says what holds it.
                let whys = get(m, "whys", 0u8) != 0;
                let traced = |s: u64| trace > 0 && (s == 0 || get(m, "traceall", 0u8) != 0);
                if whys && traced(s) {
                    w.grow_log = Some(Vec::new());
                }
                for f in 0..frames {
                    pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bed, None);
                    // `traceall=1`: every seed's founder, not only seed 0 --
                    // the per-individual trace the stall lottery needs.
                    if traced(s) && f % trace == 0 {
                        trace_founder(&w, id, &format!("{label} s={s} f={f}"));
                        if let Some(log) = w.grow_log.as_mut() {
                            let mut by: BTreeMap<String, u32> = BTreeMap::new();
                            for r in log.iter().filter(|r| r.organism == id && !r.root) {
                                *by.entry(format!("{:?}", r.why)).or_default() += 1;
                            }
                            // `PIXEL_PHYSICS_TRACE_CELLS=1`: every shoot-tip visit,
                            // with the carbon it held against the price of a cell.
                            if std::env::var("PIXEL_PHYSICS_TRACE_CELLS").is_ok() {
                                for r in log.iter().filter(|r| r.organism == id && !r.root) {
                                    println!("    VISIT f={} ({},{}) order {} carbon {:.3} cost {:.3} why {:?} retired {}", r.frame, r.x, r.y, r.order, r.carbon, r.cost, r.why, r.retired);
                                }
                            }
                            log.clear();
                            println!("    WHYS {label} s={s} f={f} {by:?}");
                        }
                    }
                }
                // `pics=N`: the founder's patch for the first N seeds,
                // `{out}_{arm}_s{seed}.png`, ground line 3/4 of the way down.
                if s < get(m, "pics", 0u64) {
                    let out: String = get(m, "out", "plant_m0".to_string());
                    let fx = arena::founder_column(&bed, off);
                    let path = format!("{out}_{}_s{s}.png", label.replace(", ", "-"));
                    crop_shot(&w, &particles, bed.width, bed.height, fx - 40, bed.ground_y - 66, 80, 88, 3, &path);
                }
                if s == 0 {
                    let causes: Vec<String> = w
                        .deaths_by_cause
                        .iter()
                        .enumerate()
                        .filter(|(_, n)| **n > 0)
                        .map(|(i, n)| format!("{}:{n}", organism::DEATH_CAUSE_LIST[i].label()))
                        .collect();
                    println!("    {label}: founder {} at the end; deaths in the bed by cause {:?}", if w.organism(id).is_some() { "alive" } else { "dead" }, causes);
                }
                // One line per seed, so the arms can be compared paired
                // within seed rather than by mean of survivors: a shoot's
                // spread inside an arm (SD ~250 on 600) swamps a 10% shift.
                println!(
                    "    SEED {label} s={s} shoot {} sprouts {} shed shade {} scald {} waits {} reserves {}",
                    form(&w, id, bed.ground_y).map_or("dead".to_string(), |f| format!("{:.0}", f[0])),
                    w.collar_sprouts,
                    w.shed_shade,
                    w.shed_scald,
                    w.seedling_waits_held,
                    w.seedling_reserves_drawn
                );
                if let Some(f) = form(&w, id, bed.ground_y) {
                    rows.push(f);
                    if let Some(st) = w.organism(id) {
                        let ls: Vec<f32> = st.cells.keys().filter(|&&(x, y)| organism::cell_type(w.get(x, y).aux()) == Some(CellType::Leaf)).map(|&(x, y)| pixel_physics::sim::plant::ambient_light_above(&w, x, y) / pixel_physics::sim::field::MAX_LIGHT).collect();
                        if !ls.is_empty() {
                            lights.push(ls.iter().sum::<f32>() / ls.len() as f32);
                        }
                    }
                }
            }
            let (lm, _) = mean_sd(&lights);
            print!("  {label:<12} alive {}/{n} leaf light {lm:.2} |", rows.len());
            for (i, name) in FORM.iter().enumerate() {
                let (mu, sd) = mean_sd(&rows.iter().map(|r| r[i]).collect::<Vec<_>>());
                print!(" {name} {mu:.2}+-{sd:.2}");
            }
            println!();
            table.push(rows);
        }
        // Only when every arm ran: under `arm=` the table holds a subset,
        // and its positions no longer name the arms below.
        for (axis, a, b) in [("dim - bright, genes ignored", 1usize, 0usize), ("dim - bright, genes read", 3, 2)] {
            if table.len() < 4 {
                continue;
            }
            print!("  {axis:<28}");
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
                w.bud_break_reserve = get(m, "budreserve", 0u8) != 0;
                // `twater=1`: `World::texture_water`, the A/B of a plant
                // reading each texture's water on its own scale.
                w.texture_water = get(m, "twater", u8::from(w.texture_water)) != 0;
                let (mut particles, mut blasts, tuning) = (ParticleSystem::new(), Blasts::new(), player::Tuning::default());
                // Mean root-zone water the founder reads (`root_zone_water`,
                // through `plant::available_water`), sampled every 1,000
                // frames: what the switch changes first, before any form.
                let (mut wet, mut wet_n) = (0.0f32, 0u32);
                for f in 0..frames {
                    pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bed, None);
                    if f % 1_000 == 999 {
                        if let Some(st) = w.organism(id).filter(|st| st.contact_root_cells > 0) {
                            wet += st.root_zone_water;
                            wet_n += 1;
                        }
                    }
                }
                println!("    SEED {label} s={s} texture_water={} rootwet {:.3} shoot {}", w.texture_water, wet / wet_n.max(1) as f32, form(&w, id, bed.ground_y).map_or("dead".to_string(), |f| format!("{:.0}", f[0])));
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

/// **Rungs 5-6's arena, as a probe** (plan §6.2): a resident stand of the
/// other roles (`mix=stand-ROLE`, `swap=` for trained residents) grown
/// `grow` frames once, then copied once per arm. Each copy is sown with `sow`
/// seeds of `role`, spread evenly, and run `window` frames more; `none` sows
/// nothing and is the resident-only control (review B1), `shipped` sows the
/// shipped species, and a path sows that species file under the role's
/// name. Arms run on their own threads from the one grown world.
///
/// Each row, per species: `plants` (germinated), `bank` (waiting seeds),
/// `leaf` cells, and `shoot` cells over the whole species. The question it
/// was built for: does a candidate that floods the long bed with its own
/// seedlings (the seed-trained tree, 2026-10-22) hold the residents below
/// where the shipped role holds them, inside a window a trainer can pay for?
fn stand(m: &BTreeMap<String, String>) {
    let started = std::time::Instant::now();
    let role: String = get(m, "role", "tree".to_string());
    let (mut w, bed, scenario, residents, mix_name, placed) = community_bed(m);
    assert!(!residents.contains(&role.as_str()), "role={role} is also a resident of mix={mix_name}");
    let (grow, window, sample, sow, gap) = (get(m, "grow", 60_000u64), get(m, "window", 80_000u64), get(m, "sample", 10_000u64), get(m, "sow", 3usize), get(m, "gap", 0i32));
    let gapat = get(m, "gapat", 0u64);
    let arms: Vec<String> = get(m, "arms", "none,shipped".to_string()).split(',').map(str::to_string).collect();
    println!(
        "plant_m0 stand: role={role} mix={mix_name} {residents:?} seed={} width={} grow={grow} window={window} sample={sample} sow={sow} gap={gap} gapat={gapat} arms={} plants_placed={placed}",
        bed.seed,
        bed.width,
        arms.join(",")
    );
    let tuning = player::Tuning::default();
    let (mut particles, mut blasts) = (ParticleSystem::new(), Blasts::new());
    for _ in 0..grow {
        pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bed, Some(&scenario));
    }
    let mut names: Vec<&str> = residents.clone();
    names.push(role.as_str());
    let row = |w: &World| -> String {
        let mut per: BTreeMap<&str, [u64; 4]> = names.iter().map(|s| (*s, [0u64; 4])).collect();
        for id in w.live_organism_ids() {
            let Some(st) = w.organism(id) else { continue };
            let def = w.species.get(st.species);
            let Some(r) = per.get_mut(def.name.as_str()) else { continue };
            let germinated = st.cells.keys().any(|&(x, y)| organism::cell_type(w.get(x, y).aux()) != Some(CellType::Seed));
            if st.shoot_cells == 0 || !germinated {
                r[1] += 1;
                continue;
            }
            r[0] += 1;
            r[3] += st.shoot_cells as u64;
            let leaf = w.materials.id_of(&def.leaf_material);
            r[2] += st.cells.keys().filter(|&&(x, y)| Some(w.get(x, y).material) == leaf).count() as u64;
        }
        names.iter().map(|n| { let r = per[n]; format!("{n} {}/{}/{}/{}", r[0], r[1], r[2], r[3]) }).collect::<Vec<_>>().join(" | ")
    };
    println!("grown f={grow} | {}  (plants/bank/leaf/shoot)", row(&w));
    // The floor's light: every column, two rows above the ground, as a share
    // of open light -- what a seedling there gets (`PIXEL_PHYSICS_LEAF_DEPTH`
    // is the lever this was added for, 2026-10-22).
    let floor = |w: &World| -> String {
        let mut v: Vec<f32> =
            (0..bed.width).map(|x| pixel_physics::sim::plant::ambient_light_above(w, x, bed.ground_y - 2) / w.open_light).collect();
        v.sort_by(f32::total_cmp);
        let q = |p: f32| v[((v.len() - 1) as f32 * p) as usize] * 100.0;
        format!("floor light % of open: p10 {:.1} p25 {:.1} p50 {:.1} p75 {:.1} p90 {:.1}", q(0.1), q(0.25), q(0.5), q(0.75), q(0.9))
    };
    println!("grown f={grow} | {}", floor(&w));
    let results: Vec<(String, Vec<String>)> = std::thread::scope(|sc| {
        let handles: Vec<_> = arms
            .iter()
            .map(|arm| {
                let (mut c, bed, scenario, role, row, floor) = (w.clone(), &bed, &scenario, role.as_str(), &row, &floor);
                sc.spawn(move || {
                    let mut out = Vec::new();
                    if arm != "none" && arm != "shipped" {
                        let mut def = pixel_physics::train_plant::load(std::path::Path::new(arm)).expect("the arm's species loads");
                        def.name = role.to_string();
                        let ron = pixel_physics::sim::species_export::to_ron(&def).expect("exports");
                        c.species.register_ron(&ron).expect("the arm's species registers");
                    }
                    // `gap=G`: a disturbance gap G columns wide cut at each
                    // sowing column in every arm, `none` included -- every
                    // plant cell above the ground in it erased, roots left to
                    // resprout. Measured without it (2026-10-22, 4 seeds, a
                    // 60,000-frame stand of shrub, herb and creeper 192
                    // wide): no tree seed sown onto the stand ever
                    // germinated, shipped or trained, so every tree arm
                    // ended cell-identical to the shipped one and the rung
                    // could not tell a flooding tree from any other.
                    //
                    // `gapat=F`: cut it F frames into the window instead, over
                    // seed sown under the closed stand -- whether a seedling
                    // bank waiting on the floor is released when the canopy
                    // opens (advance regeneration), which is what
                    // `World::seedling_reserves` is for.
                    // A late cut spares the sown role: a fallen crown opens
                    // the floor, it does not clear the seedlings waiting on
                    // it (first measured with them cut too, 2026-10-22, and
                    // the bank was the thing erased).
                    let cut = |c: &mut World, spare: bool| {
                        let role_id = c.species.id_of(role);
                        for i in 0..sow {
                            let x0 = (bed.width as f32 * (i as f32 + 0.5) / sow as f32) as i32 - gap / 2;
                            for x in x0.max(0)..(x0 + gap).min(bed.width) {
                                for y in 0..bed.ground_y {
                                    let id = c.get(x, y).organism_id();
                                    if id != 0 && !(spare && c.organism(id).map(|o| o.species) == role_id) {
                                        c.set(x, y, Cell::EMPTY);
                                    }
                                }
                            }
                        }
                    };
                    if gap > 0 && gapat == 0 {
                        cut(&mut c, false);
                    }
                    let mut sown: Vec<OrganismId> = Vec::new();
                    if arm != "none" {
                        let at: Vec<Placement> =
                            (0..sow).map(|i| Placement::Plant { species: role.to_string(), x: (bed.width as f32 * (i as f32 + 0.5) / sow as f32) as i32 }).collect();
                        let before: std::collections::HashSet<_> = c.live_organism_ids().into_iter().collect();
                        let n = pixel_physics::lab::scenario::apply_placements(&mut c, &at, bed);
                        // `endow=X`: each sown seed carries X carbon, as a
                        // seed its parent bred would (`Reproduce.seed_cost`,
                        // 0.25 for shrub and creeper), instead of a planted
                        // seed's 0 -- so the rung sows what natural
                        // recruitment drops. Default 0, the old behaviour.
                        let stake = get(m, "endow", 0.0f32);
                        sown = c.live_organism_ids().into_iter().filter(|id| !before.contains(id)).collect();
                        if stake > 0.0 {
                            for &id in &sown {
                                pixel_physics::sim::plant::endow_seed(&mut c, id, stake);
                            }
                        }
                        out.push(format!("sown {} of {sow} endow {stake}", n.plants));
                    }
                    // `trace=N`: every sown plant every N frames (printed as it
                    // runs, so give it one arm), and how each one died.
                    let trace = get(m, "trace", 0u64);
                    if trace > 0 {
                        c.death_log = Some(Vec::new());
                    }
                    let (mut p, mut b) = (ParticleSystem::new(), Blasts::new());
                    for f in 1..=window {
                        pixel_physics::lab::tick_world(&mut c, &mut p, &mut b, &tuning, bed, Some(scenario));
                        if gap > 0 && f == gapat {
                            cut(&mut c, true);
                            out.push(format!("f={f} gap cut | {}", row(&c)));
                        }
                        if trace > 0 && f % trace == 0 {
                            for (i, &id) in sown.iter().enumerate() {
                                if c.organism(id).is_some() {
                                    trace_founder(&c, id, &format!("f={f} sown{i}"));
                                }
                            }
                        }
                        if f % sample == 0 || f == window {
                            out.push(format!("f={f} | {}", row(&c)));
                        }
                    }
                    out.push(format!("end | {}", floor(&c)));
                    out.push(format!(
                        "SWITCHES reserves_drawn {} settled {} heaved {} germinations {}",
                        c.seedling_reserves_drawn, c.seeds_settled, c.seeds_heaved, c.germinations
                    ));
                    for d in c.death_log.iter().flatten().filter(|d| sown.contains(&d.id)) {
                        out.push(format!(
                            "DEATH sown{} at f={} age {} cause {:?} declared {} dormant_seed {} at {:?}",
                            sown.iter().position(|&id| id == d.id).unwrap_or(0),
                            d.frame,
                            d.frame - d.born_frame,
                            d.cause,
                            d.declared,
                            d.dormant_seed,
                            d.at
                        ));
                    }
                    out
                })
            })
            .collect();
        arms.iter().cloned().zip(handles.into_iter().map(|h| h.join().expect("an arm's thread"))).collect()
    });
    for (arm, rows) in results {
        for r in rows {
            println!("arm={arm} {r}");
        }
    }
    println!("plant_m0 done: {:.1} s", started.elapsed().as_secs_f64());
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

/// One plant's heritable state, as `World::organism_genotype` and the fate
/// table hold it: what `mode=bedsel` carries from a selected bed into the
/// founders of the next round's fresh beds.
#[derive(Clone, Copy)]
struct PlantGenome {
    draws: [f32; organism::GENOTYPE_TRAITS],
    alleles: [u8; organism::DISCRETE_LOCI],
    params: organism::ParamGenome,
    lineage: u64,
    fates: organism::FateGenome,
}

/// A [`PlantGenome`] in a form `ron` can write and read back: the pool
/// `mode=bedsel save=` keeps, and `load=` judges again without re-running
/// the rounds -- the product of a selection run, not only its log.
#[derive(serde::Serialize, serde::Deserialize)]
struct SavedGenome {
    role: String,
    draws: Vec<f32>,
    alleles: Vec<u8>,
    params: Vec<(CellType, ParamId, u8, f32)>,
    fates: Vec<(CellType, Vec<organism::Fate>)>,
    lineage: u64,
}

fn save_pool(path: &str, pool: &BTreeMap<String, Vec<PlantGenome>>) {
    let list: Vec<SavedGenome> = pool
        .iter()
        .flat_map(|(role, gs)| {
            gs.iter().map(move |g| SavedGenome {
                role: role.clone(),
                draws: g.draws.to_vec(),
                alleles: g.alleles.to_vec(),
                params: g.params.overrides().iter().map(|o| (o.cell_type, o.param, o.tier, o.value)).collect(),
                fates: g.fates.to_table(),
                lineage: g.lineage,
            })
        })
        .collect();
    let text = ron::ser::to_string_pretty(&list, ron::ser::PrettyConfig::default()).expect("the pool serialises");
    std::fs::write(path, text).expect("the pool file writes");
    println!("plant_m0 bedsel: saved {} genomes to {path}", list.len());
}

fn load_pool(path: &str) -> BTreeMap<String, Vec<PlantGenome>> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("load={path}: {e}"));
    let list: Vec<SavedGenome> = ron::from_str(&text).unwrap_or_else(|e| panic!("load={path}: {e}"));
    let mut pool: BTreeMap<String, Vec<PlantGenome>> = BTreeMap::new();
    for g in list {
        let mut params = organism::ParamGenome::default();
        for (c, p, t, v) in g.params {
            assert!(params.set(c, p, t, v), "load: a param override did not fit");
        }
        pool.entry(g.role).or_default().push(PlantGenome {
            draws: g.draws.try_into().expect("load: draws length"),
            alleles: g.alleles.try_into().expect("load: alleles length"),
            params,
            lineage: g.lineage,
            fates: organism::FateGenome::from_table(&g.fates),
        });
    }
    println!("plant_m0 bedsel: loaded {} genomes from {path}", pool.values().map(Vec::len).sum::<usize>());
    pool
}

/// What one bed of `mode=bedsel` ends a round with: leaf cells per role (in
/// the mix's order), the genomes it offers the next round per role, and how
/// many founders it was given from the pool.
struct BedEnd {
    leaf: Vec<u64>,
    harvest: BTreeMap<String, Vec<PlantGenome>>,
    transplanted: usize,
    /// `bed=played` only: the colony's larder over the round -- mean edible
    /// food at each 25,000-frame sample (`lab::census`), the ants at the
    /// end, and the colony's starved deaths.
    food: u64,
    ants: usize,
    starved: u64,
}

/// One bed of `mode=bedsel`: the community bed at `seed`, its founders
/// given genomes from `pool` (role -> genomes, cycled; a role with none
/// keeps its species file), run `frames` with the heredity `heredity` sets.
fn bedsel_bed(m: &BTreeMap<String, String>, seed: u64, frames: u64, pool: &BTreeMap<String, Vec<PlantGenome>>, heredity: bool, salt: usize, shot_path: Option<String>) -> BedEnd {
    let mut mm = m.clone();
    mm.insert("seed".into(), seed.to_string());
    // The trainer's beds found from species files plus the pool it is
    // given -- never from the engine's shipped pool, or round 0 and the
    // judge's "shipped" arm would both be someone else's selection.
    mm.insert("pool".into(), "0".into());
    mm.insert("heredity".into(), if heredity { "on" } else { "off" }.into());
    // **`bed=played`: the lab's own bed with its colony** (2026-10-25). The
    // four-role pool bred without ants fed the colony a quarter less food
    // in the game (`Reports/plant-community-2026-10-20/m2/bedsel/`), so this
    // breeds where the plants will live -- `played_bed` as the game builds
    // it, ants landing at frame 6,000 -- stepped by `lab::tick_world`, the
    // game's own tick with the timeline and the mister.
    let played = m.get("bed").map(String::as_str) == Some("played");
    let (mut w, bed, scenario, species) = if played {
        let mut sc = Scenario::load("played_bed").expect("played_bed loads");
        sc.bed.seed = seed;
        sc.bed.plant_pool = false;
        let (mut w, _, _) = sc.build();
        // `community_bed`'s heredity dials, for the same reasons.
        w.fate_mutation_chance = 0.0;
        w.param_mutation_chance = 0.0;
        if !heredity {
            w.mutation_sigma = 0.0;
            w.founder_variant_chance = 0.0;
        }
        (w, sc.bed.clone(), sc, mix("played"))
    } else {
        let (w, bed, scenario, species, _, _) = community_bed(&mm);
        (w, bed, scenario, species)
    };
    if heredity {
        // `community_bed` zeroes the two coarse dials for a census of
        // today's species; selection needs a lineage to be able to leave
        // its species file, so the rule-level dial goes back to the game's
        // and the number-level one to `pmut=`.
        w.fate_mutation_chance = get(m, "fmut", pixel_physics::sim::plant::fate_mutation_chance_seed());
        w.param_mutation_chance = get(m, "pmut", 0.1f32);
    }
    let mut transplanted = 0;
    let mut next: BTreeMap<String, usize> = BTreeMap::new();
    for id in w.live_organism_ids() {
        let Some(st) = w.organism(id) else { continue };
        let name = w.species.get(st.species).name.clone();
        let Some(gs) = pool.get(&name).filter(|g| !g.is_empty()) else { continue };
        let k = next.entry(name).or_insert(salt);
        let g = gs[*k % gs.len()];
        *k += 1;
        w.set_organism_genotype(id, g.draws, g.alleles, g.params, g.lineage);
        w.set_organism_fates(id, g.fates);
        transplanted += 1;
    }
    let (mut particles, mut blasts, tuning) = (ParticleSystem::new(), Blasts::new(), player::Tuning::default());
    let ids = pixel_physics::lab::census::Ids::resolve(&w);
    let (mut food_sum, mut food_n) = (0u64, 0u64);
    for f in 1..=frames {
        pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bed, Some(&scenario));
        if played && f % 25_000 == 0 {
            let nest = pixel_physics::lab::census::nest_columns(&w, &bed, Some(&scenario));
            food_sum += pixel_physics::lab::census::census(&w, &bed, 0.0, &nest, &ids).edible as u64;
            food_n += 1;
        }
    }
    let food = food_sum / food_n.max(1);
    let ants = w.live_creature_count();
    let starved = if played { pixel_physics::lab::census::colony_deaths(&w, &bed.colony_species).0 } else { 0 };
    if let Some(path) = shot_path {
        shot(&mut w, &particles, &mut pixel_physics::render::Renderer::new(), &bed, get(m, "below", 20usize), &path);
    }
    let leaf_mat = w.materials.id_of("leaf");
    let keep: usize = get(m, "keep", 8);
    let mut leaf = vec![0u64; species.len()];
    let mut by_role: Vec<Vec<(u64, u32, OrganismId)>> = vec![vec![]; species.len()];
    for id in w.live_organism_ids() {
        let Some(st) = w.organism(id) else { continue };
        let name = &w.species.get(st.species).name;
        let Some(i) = species.iter().position(|s| s == name) else { continue };
        let l = st.cells.keys().filter(|&&(x, y)| Some(w.get(x, y).material) == leaf_mat).count() as u64;
        leaf[i] += l;
        by_role[i].push((l, st.shoot_cells, id));
    }
    let mut harvest = BTreeMap::new();
    for (i, mut list) in by_role.into_iter().enumerate() {
        // The leafiest plants first, then the biggest, then the waiting
        // seed bank -- a role whose adults all died still offers its seeds.
        list.sort_by_key(|&(l, sh, id)| (std::cmp::Reverse(l), std::cmp::Reverse(sh), id));
        let gs: Vec<PlantGenome> = list
            .iter()
            .take(keep)
            .filter_map(|&(_, _, id)| {
                let (draws, alleles, params, lineage) = w.organism_genotype(id)?;
                Some(PlantGenome { draws, alleles, params, lineage, fates: w.organism(id)?.fates })
            })
            .collect();
        harvest.insert(species[i].to_string(), gs);
    }
    BedEnd { leaf, harvest, transplanted, food, ants, starved }
}

/// A bed's rank in `mode=bedsel`: roles holding 100+ leaf first, then the
/// weakest role's leaf -- a bed is only as mixed as its scarcest member, so
/// one huge tree cannot buy the rank (the ant line's summed colony score
/// was won by not digging; `Reports/` scoring review, 2026-10-22).
fn bedsel_rank(leaf: &[u64]) -> (usize, u64) {
    (leaf.iter().filter(|&&l| l >= 100).count(), leaf.iter().copied().min().unwrap_or(0))
}

/// **`mode=bedsel` -- plants trained the way the ants are: whole beds
/// selected** (owner, 2026-10-24, from the ant line's colony selection).
/// Each round runs `beds` fresh community beds for `round` frames with
/// in-bed heredity on (`pmut=` param mutation, `fmut=` fate mutation, the
/// genotype jitter); each bed's founders are given genomes from its parent
/// pool. At the round's end every bed offers its `keep` leafiest plants per
/// role, the beds are ranked (`bedsel_rank`), and the top half's offerings
/// become every bed's pool for the next round. Fresh beds every round, so
/// what is selected is the plants, not a lucky bed's soil.
///
/// `pick=random` ranks the beds by a seeded shuffle instead: the control
/// for "selection did it" against "heredity and time did it".
///
/// **The judge** is a common garden after the last round: `garden` fresh
/// beds per arm (seeds 9001..), heredity off, `gframes` frames, shipped
/// species files against founders drawn from the final pool, paired by
/// seed. Selection that only rearranged luck shows no gain there.
fn bedsel(m: &BTreeMap<String, String>) {
    let started = std::time::Instant::now();
    let beds: usize = get(m, "beds", 8);
    let round: u64 = get(m, "round", 100_000);
    let rounds: usize = get(m, "rounds", 6);
    let pick: String = get(m, "pick", "types".to_string());
    let base: u64 = get(m, "seed", 1);
    let garden: usize = get(m, "garden", 8);
    let gframes: u64 = get(m, "gframes", 300_000);
    assert!(pick == "types" || pick == "random", "pick={pick} is not types|random");
    let played = m.get("bed").map(String::as_str) == Some("played");
    if played {
        pixel_physics::sim::creature::mutation_off_for_measuring();
    }
    let species = if played { mix("played") } else { mix(&get(m, "mix", "roles4".to_string())) };
    println!(
        "plant_m0 bedsel: bed={} beds={beds} round={round} rounds={rounds} pick={pick} seed={base} keep={} pmut={} fmut={} garden={garden} gframes={gframes} roles={species:?}",
        if played { "played" } else { "community" },
        get(m, "keep", 8usize),
        get(m, "pmut", 0.1f32),
        get(m, "fmut", pixel_physics::sim::plant::fate_mutation_chance_seed())
    );
    // `out=PATH`: each garden bed's last frame is drawn to PATH_ARM_sSEED.png.
    let out: Option<String> = m.get("out").cloned();
    let run_all = |seeds: &[u64], frames: u64, pool: &BTreeMap<String, Vec<PlantGenome>>, heredity: bool, arm: Option<&str>| -> Vec<BedEnd> {
        std::thread::scope(|sc| {
            let hs: Vec<_> = seeds
                .iter()
                .enumerate()
                .map(|(i, &sd)| {
                    let path = arm.zip(out.as_ref()).map(|(a, o)| format!("{o}_{a}_s{sd}.png"));
                    sc.spawn(move || bedsel_bed(m, sd, frames, pool, heredity, i * 7, path))
                })
                .collect();
            hs.into_iter().map(|h| h.join().expect("a bed panicked")).collect()
        })
    };
    // `load=FILE` starts from a saved pool (with `rounds=0`, only the
    // garden runs); `save=FILE` writes the final pool.
    let mut pool: BTreeMap<String, Vec<PlantGenome>> = m.get("load").map_or_else(BTreeMap::new, |p| load_pool(p));
    for r in 0..rounds {
        let seeds: Vec<u64> = (0..beds).map(|i| base * 1000 + (r * beds + i) as u64).collect();
        let ends = run_all(&seeds, round, &pool, true, None);
        let mut order: Vec<usize> = (0..beds).collect();
        if pick == "types" {
            // `bed=played`: kinds first, then the colony's larder -- never
            // one summed score, so food cannot buy back a lost kind.
            if played {
                order.sort_by_key(|&i| (std::cmp::Reverse((bedsel_rank(&ends[i].leaf).0, ends[i].food)), i));
            } else {
                order.sort_by_key(|&i| (std::cmp::Reverse(bedsel_rank(&ends[i].leaf)), i));
            }
        } else {
            let mut rng = pixel_physics::sim::rng::stream(base, r as u64, 0x5e1, 0);
            for i in (1..order.len()).rev() {
                order.swap(i, rng.below(i as u32 + 1) as usize);
            }
        }
        let parents: Vec<usize> = order[..beds.div_ceil(2)].to_vec();
        for (i, e) in ends.iter().enumerate() {
            let (t, lo) = bedsel_rank(&e.leaf);
            println!(
                "BED r={r} bed={i} seed={} types={t} weakest={lo} food={} ants={} starved={} leaf={} given={} parent={}",
                seeds[i],
                e.food,
                e.ants,
                e.starved,
                species.iter().zip(&e.leaf).map(|(n, l)| format!("{n}:{l}")).collect::<Vec<_>>().join(","),
                e.transplanted,
                parents.contains(&i)
            );
        }
        let mean_types = ends.iter().map(|e| bedsel_rank(&e.leaf).0).sum::<usize>() as f64 / beds as f64;
        let mean_food = ends.iter().map(|e| e.food).sum::<u64>() / beds as u64;
        // A role this bed does not sow (the creeper, under `bed=played`)
        // is carried over untouched, so the pool keeps all four.
        let mut next: BTreeMap<String, Vec<PlantGenome>> =
            pool.iter().filter(|(role, _)| !species.contains(&role.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
        for &p in &parents {
            for (role, gs) in &ends[p].harvest {
                next.entry(role.clone()).or_default().extend(gs.iter().copied());
            }
        }
        println!(
            "ROUND r={r} mean_types={mean_types:.2} mean_food={mean_food} pool={} elapsed={:.0}s",
            next.iter().map(|(k, v)| format!("{k}:{}", v.len())).collect::<Vec<_>>().join(","),
            started.elapsed().as_secs_f64()
        );
        pool = next;
    }
    if let Some(path) = m.get("save") {
        save_pool(path, &pool);
    }
    // The judge: paired fresh beds, heredity off, shipped vs pool.
    let gseeds: Vec<u64> = (0..garden).map(|i| 9001 + i as u64).collect();
    let empty = BTreeMap::new();
    for (arm, p) in [("shipped", &empty), ("pool", &pool)] {
        let ends = run_all(&gseeds, gframes, p, false, Some(arm));
        for (i, e) in ends.iter().enumerate() {
            let (t, lo) = bedsel_rank(&e.leaf);
            println!(
                "GARDEN arm={arm} seed={} types={t} weakest={lo} leaf={} given={}",
                gseeds[i],
                species.iter().zip(&e.leaf).map(|(n, l)| format!("{n}:{l}")).collect::<Vec<_>>().join(","),
                e.transplanted
            );
        }
    }
    println!("plant_m0 done: {:.1} s", started.elapsed().as_secs_f64());
}

fn main() {
    let m = args();
    match m.get("mode").map(String::as_str).unwrap_or("community") {
        "community" => community(&m),
        "boxed" => boxed(&m),
        "plastic" => plastic(&m),
        "cost" => cost(&m),
        "copy" => copy(&m),
        "stand" => stand(&m),
        "nudge" => nudge(&m),
        "draincost" => drain_cost(&m),
        "bedsel" => bedsel(&m),
        other => {
            eprintln!("plant_m0: mode={other} is not community|boxed|plastic|cost|copy|stand|nudge|draincost|bedsel");
            std::process::exit(2);
        }
    }
}
