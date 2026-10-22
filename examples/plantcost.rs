//! **What a bed of plants costs per frame, and whether a speed change left it
//! the same bed** -- the plant scenes of the shared performance benchmark
//! (2026-10-21: four performance threads, one benchmark, `scripts/perfbench.sh`
//! runs the set). Two scenes, for the two places plants are paid for:
//!
//! - **`scene=community`** -- many plants: `plant_m0 mode=community`'s bed of
//!   record (512 wide, `mix=roles4`, 12 founders, mister `Light`, drain on, no
//!   colony, in-bed heredity off), rebuilt here so the benchmark does not
//!   depend on that harness's arguments. Pre-run to `at=` (default 60,000:
//!   about 125 plants, most of them grass, measured seed 1), then `reps`
//!   windows of `window` frames, round-robin and the minimum quoted
//!   (`antcost`'s rule: contention can only make a window slower).
//! - **`scene=rung`** -- what the plant trainer pays for one individual:
//!   `train_plant::arena`'s rung 2 (`grow`, 48 wide, 10,000 frames, under a
//!   lamp, bright station), `species=` (default herb), `layouts=` world seeds
//!   from 3,000,000. Each bed is timed whole, since a trainer pays for the
//!   whole bed; `at=`/`window=` are refused here.
//!
//! **Every run ends in three hashes**, and a speed change ships only if all
//! three match before and after: `cells` (material, shade, aux, organism id
//! over the box), `field` (`field::field_hash`: light, moisture, the rest),
//! and `orgs` (per live organism, in id order: species, water, water status,
//! endowment and every cell's position, carbon and support). The third is
//! there because a plant's carbon and water decide its growth a few hundred
//! frames before any cell moves, so a 1,200-frame window can end on equal
//! cells over unequal plants.
//!
//! `rain=off` turns the mister off (both scenes), for the question the
//! plant-community thread raised: the mister is ~94% of a rung bed's cost.
//! `cg=1` brackets each timed window with callgrind's client requests, as
//! `needscost` does:
//!
//! ```text
//! RAYON_NUM_THREADS=1 cargo run --release --example plantcost -- scene=community at=60000 window=1200 reps=3
//! RAYON_NUM_THREADS=1 cargo run --release --example plantcost -- scene=rung layouts=4
//! RAYON_NUM_THREADS=1 valgrind --tool=callgrind --instr-atstart=no \
//!     target/release/examples/plantcost scene=community at=60000 window=1200 reps=1 cg=1
//! ```
//!
//! Every unknown argument is refused.

use pixel_physics::lab::rain::Rain;
use pixel_physics::lab::scenario::{Placement, Scenario};
use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::field;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;
use pixel_physics::train_plant::arena::{self, Rung, Station};
use std::collections::BTreeMap;
use std::time::Instant;

const KNOWN: &[&str] = &["churn", "scene", "seed", "at", "window", "reps", "rain", "mix", "species", "layouts", "cg"];

fn args() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for a in std::env::args().skip(1) {
        let Some((k, v)) = a.split_once('=') else {
            eprintln!("plantcost: `{a}` is not key=value");
            std::process::exit(2);
        };
        if !KNOWN.contains(&k) {
            eprintln!("plantcost: unknown argument `{k}` (known: {})", KNOWN.join(" "));
            std::process::exit(2);
        }
        m.insert(k.to_string(), v.to_string());
    }
    m
}

fn get<T: std::str::FromStr>(m: &BTreeMap<String, String>, k: &str, default: T) -> T {
    m.get(k).map_or(default, |v| v.parse().unwrap_or_else(|_| panic!("{k}={v} does not parse")))
}

/// A callgrind client request, inline: `needscost`'s, copied (it explains
/// the sequence). A no-op natively.
#[cfg(target_arch = "x86_64")]
fn callgrind(request: u64, arg1: u64) {
    let args: [u64; 6] = [request, arg1, 0, 0, 0, 0];
    let mut result: u64 = 0;
    // SAFETY: as in `needscost`: every register is left as found; `args` is
    // read through `rax` and, under valgrind, `rdx` is written.
    unsafe {
        std::arch::asm!(
            "rol rdi, 3",
            "rol rdi, 13",
            "rol rdi, 61",
            "rol rdi, 51",
            "xchg rbx, rbx",
            inout("rdx") result,
            in("rax") args.as_ptr(),
            options(nostack),
        );
    }
    let _ = result;
}

#[cfg(not(target_arch = "x86_64"))]
fn callgrind(_request: u64, _arg1: u64) {}

const CG_ZERO_STATS: u64 = 0x4354_0001;
const CG_DUMP_STATS_AT: u64 = 0x4354_0003;
const CG_START_INSTRUMENTATION: u64 = 0x4354_0004;
const CG_STOP_INSTRUMENTATION: u64 = 0x4354_0005;

fn cg_start(on: bool) {
    if on {
        callgrind(CG_ZERO_STATS, 0);
        callgrind(CG_START_INSTRUMENTATION, 0);
    }
}

fn cg_stop(on: bool, label: &str) {
    if on {
        let name = std::ffi::CString::new(label).expect("no nul");
        callgrind(CG_DUMP_STATS_AT, name.as_ptr() as u64);
        callgrind(CG_STOP_INSTRUMENTATION, 0);
    }
}

fn fnv(h: u64, v: u64) -> u64 {
    (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
}

const FNV0: u64 = 0xcbf2_9ce4_8422_2325;

/// Every cell of the box: material, shade, aux, organism id.
fn cells_hash(w: &World, width: i32, height: i32) -> u64 {
    let mut h = FNV0;
    for y in 0..height {
        for x in 0..width {
            let c = w.get(x, y);
            for v in [c.material.0 as u64, c.shade as u64, c.aux() as u64, c.organism_id() as u64] {
                h = fnv(h, v);
            }
        }
    }
    h
}

/// Every live organism's economy and cells, in id order, cells sorted.
fn orgs_hash(w: &World) -> u64 {
    let mut h = FNV0;
    let mut ids = w.live_organism_ids();
    ids.sort_unstable();
    for id in ids {
        let Some(s) = w.organism(id) else { continue };
        h = fnv(h, id as u64);
        h = fnv(h, s.species.0 as u64);
        for f in [s.water, s.water_status, s.endowment] {
            h = fnv(h, f.to_bits() as u64);
        }
        let mut cells: Vec<_> = s.cells.iter().map(|(p, c)| (*p, c.carbon.to_bits(), c.support)).collect();
        cells.sort_unstable_by_key(|(p, _, _)| (p.1, p.0));
        h = fnv(h, cells.len() as u64);
        for ((x, y), carbon, support) in cells {
            h = fnv(h, x as u64);
            h = fnv(h, y as u64);
            h = fnv(h, carbon as u64);
            h = fnv(h, support as u64);
        }
    }
    h
}

fn hashes(w: &World, width: i32, height: i32) -> String {
    format!("cells={:016x} field={:016x} orgs={:016x}", cells_hash(w, width, height), field::field_hash(w), orgs_hash(w))
}

fn rain(m: &BTreeMap<String, String>) -> Rain {
    match m.get("rain").map(String::as_str) {
        None | Some("light") => Rain::Light,
        Some("off") => Rain::Off,
        Some("steady") => Rain::Steady,
        Some(v) => panic!("rain={v} is not off|light|steady"),
    }
}

/// `plant_m0`'s mixes, the two it runs community beds on.
fn mix(name: &str) -> Vec<&'static str> {
    match name {
        "roles4" => vec!["shrub", "creeper", "herb", "grass"],
        "roles" => vec!["tree", "shrub", "herb", "creeper"],
        other => panic!("mix={other} is not roles4|roles"),
    }
}

/// `plant_m0::columns`, copied: `n` plants over the bed, keeping 210..310
/// bare as the played bed does.
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

/// `plant_m0`'s community bed at its defaults (`heredity=off`, load failure
/// on, drain on), with `rain=` and `mix=`.
fn community_world(m: &BTreeMap<String, String>) -> (World, LabBox, Scenario, usize) {
    let species = mix(&get(m, "mix", "roles4".to_string()));
    let per = 3;
    let width = 512;
    let placements: Vec<Placement> = columns(species.len() * per, width)
        .iter()
        .enumerate()
        .map(|(i, &x)| Placement::Plant { species: species[i % species.len()].to_string(), x })
        .collect();
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
        seed: get(m, "seed", 1),
        rain: rain(m),
        ..LabBox::default()
    };
    let scenario = Scenario {
        name: "plantcost-community".into(),
        title: "plantcost".into(),
        question: "plant bed cost".into(),
        bed: bed.clone(),
        settings: vec![],
        placements,
        timeline: vec![],
        horizon: u64::MAX,
    };
    let (mut w, _, placed) = scenario.build();
    w.fate_mutation_chance = 0.0;
    w.param_mutation_chance = 0.0;
    w.mutation_sigma = 0.0;
    w.founder_variant_chance = 0.0;
    (w, bed, scenario, placed.plants)
}

fn plants(w: &World) -> (usize, usize) {
    let ids = w.live_organism_ids();
    let cells = ids.iter().filter_map(|&id| w.organism(id)).map(|s| s.cells.len()).sum();
    (ids.len(), cells)
}

fn community(m: &BTreeMap<String, String>, cg: bool) {
    let at: u64 = get(m, "at", 60_000);
    let window: u64 = get(m, "window", 1_200);
    let reps: usize = get(m, "reps", 3);
    let (mut w, bed, scenario, placed) = community_world(m);
    println!(
        "plantcost community: mix={} seed={} rain={:?} placed={placed} at={at} window={window} reps={reps}",
        get(m, "mix", "roles4".to_string()),
        bed.seed,
        bed.rain
    );
    let (mut particles, mut blasts, tuning) = (ParticleSystem::new(), Blasts::new(), player::Tuning::default());
    let t0 = Instant::now();
    for _ in 0..at {
        pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bed, Some(&scenario));
    }
    let (orgs, cells) = plants(&w);
    println!("  pre-run to frame {at}: {:.1} s, {orgs} organisms, {cells} organism cells", t0.elapsed().as_secs_f64());
    let mut ms = Vec::new();
    for rep in 0..reps {
        cg_start(cg);
        let t = Instant::now();
        for _ in 0..window {
            pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bed, Some(&scenario));
        }
        let dt = t.elapsed().as_secs_f64();
        cg_stop(cg, &format!("community-rep{rep}-f{}", w.frame));
        let v = dt * 1000.0 / window as f64;
        ms.push(v);
        let (orgs, cells) = plants(&w);
        println!("  rep {rep}: {v:.3} ms/frame, {orgs} organisms, {cells} organism cells");
    }
    ms.sort_by(f64::total_cmp);
    println!("RESULT scene=community min_ms_per_frame={:.3} median={:.3} frame={} {}", ms[0], ms[ms.len() / 2], w.frame, hashes(&w, bed.width, bed.height));
}

fn rung(m: &BTreeMap<String, String>, cg: bool) {
    for k in ["at", "window", "reps"] {
        assert!(!m.contains_key(k), "scene=rung times each whole bed; {k}= does not apply");
    }
    let species: String = get(m, "species", "herb".to_string());
    let n: usize = get(m, "layouts", 4);
    let station = Station { rain: rain(m), ..Station::BRIGHT };
    let spec = Rung::Grow.spec(station);
    println!("plantcost rung: species={species} rung=grow width={} frames={} station={} rain={:?} layouts={n}", spec.width, spec.frames, station.name, station.rain);
    let (mut total, mut h) = (0.0, FNV0);
    for (i, layout) in Rung::Grow.layouts(3_000_000, n).into_iter().enumerate() {
        let bed = arena::bed_for(None, &species, layout, &spec);
        let (mut w, _) = arena::build(&bed, &spec);
        let (mut particles, mut blasts, tuning) = (ParticleSystem::new(), Blasts::new(), player::Tuning::default());
        cg_start(cg);
        let t = Instant::now();
        let mut sw = [0u64; 3];
        let churn = get(m, "churn", 0u8) == 1;
        let soil_id = w.materials.id_of("soil");
        let grid = |w: &World| -> Vec<u16> {
            let mut v = Vec::with_capacity((bed.width * bed.height) as usize);
            for y in 0..bed.height {
                for x in 0..bed.width {
                    let c = w.get(x, y);
                    v.push(if Some(c.material) == soil_id { pixel_physics::sim::update::soil_moisture(c) } else { u16::MAX });
                }
            }
            v
        };
        // `churn=1`: |change| per changed soil cell per tick, bucketed, and
        // over each 1,000-frame block how far a cell that changed ended from
        // where it started (oscillation against flow).
        let mut hist = [0u64; 7];
        let (mut prev, mut block0) = (Vec::new(), Vec::new());
        let mut changes_in_block: Vec<u32> = Vec::new();
        let mut net_hist = [0u64; 5];
        for f in 0..spec.frames {
            if churn && f >= 5_000 && prev.is_empty() {
                prev = grid(&w);
                block0 = prev.clone();
                changes_in_block = vec![0; prev.len()];
            }
            pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bed, None);
            if churn && !prev.is_empty() {
                let now = grid(&w);
                for (i, (&a, &b)) in prev.iter().zip(now.iter()).enumerate() {
                    if a == u16::MAX || b == u16::MAX || a == b {
                        continue;
                    }
                    let d = (a as i32 - b as i32).unsigned_abs();
                    let k = match d { 1 => 0, 2 => 1, 3..=5 => 2, 6..=10 => 3, 11..=50 => 4, 51..=200 => 5, _ => 6 };
                    hist[k] += 1;
                    changes_in_block[i] += 1;
                }
                if (f + 1) % 1_000 == 0 {
                    for (i, &n) in changes_in_block.iter().enumerate() {
                        if n < 10 || block0[i] == u16::MAX || now[i] == u16::MAX {
                            continue;
                        }
                        let d = (block0[i] as i32 - now[i] as i32).unsigned_abs();
                        let k = match d { 0..=5 => 0, 6..=20 => 1, 21..=50 => 2, 51..=200 => 3, _ => 4 };
                        net_hist[k] += 1;
                    }
                    block0 = now.clone();
                    changes_in_block.iter_mut().for_each(|n| *n = 0);
                }
                prev = now;
            }
            let s = w.soil_water_stats;
            sw[0] += s.visited;
            sw[1] += s.soil;
            sw[2] += s.changed;
        }
        let dt = t.elapsed().as_secs_f64();
        cg_stop(cg, &format!("rung-{i}"));
        let per = |v: u64| v as f64 / spec.frames as f64;
        if churn {
            let tot: u64 = hist.iter().sum();
            println!("    churn |delta| per change: 1 {:.1}% 2 {:.1}% 3-5 {:.1}% 6-10 {:.1}% 11-50 {:.1}% 51-200 {:.1}% >200 {:.1}% (n {tot})", hist[0] as f64 * 100.0 / tot as f64, hist[1] as f64 * 100.0 / tot as f64, hist[2] as f64 * 100.0 / tot as f64, hist[3] as f64 * 100.0 / tot as f64, hist[4] as f64 * 100.0 / tot as f64, hist[5] as f64 * 100.0 / tot as f64, hist[6] as f64 * 100.0 / tot as f64);
            let nt: u64 = net_hist.iter().sum();
            println!("    cells changing 10+ times in a 1,000-frame block, net move over the block: 0-5 {} 6-20 {} 21-50 {} 51-200 {} >200 {} (of {nt})", net_hist[0], net_hist[1], net_hist[2], net_hist[3], net_hist[4]);
        }
        println!("    soil water a tick: visited {:.0} soil {:.0} changed {:.0}", per(sw[0]), per(sw[1]), per(sw[2]));
        total += dt;
        let (orgs, cells) = plants(&w);
        let hs = hashes(&w, bed.width, bed.height);
        for part in hs.split(' ') {
            let v = u64::from_str_radix(part.split_once('=').expect("k=v").1, 16).expect("hex");
            h = fnv(h, v);
        }
        println!("  layout {i} seed {}: {:.3} s, {:.3} ms/frame, {orgs} organisms, {cells} organism cells, {hs}", layout.seed, dt, dt * 1000.0 / spec.frames as f64);
    }
    println!("RESULT scene=rung species={species} s_per_bed={:.3} ms_per_frame={:.3} beds={n} hash={h:016x}", total / n as f64, total * 1000.0 / (n as f64 * spec.frames as f64));
}

fn main() {
    let m = args();
    let cg = get(&m, "cg", 0u8) == 1;
    let threads = std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "unset".to_string());
    println!("plantcost RAYON_NUM_THREADS={threads} cg={}", u8::from(cg));
    match m.get("scene").map(String::as_str).unwrap_or("community") {
        "community" => community(&m, cg),
        "rung" => rung(&m, cg),
        other => {
            eprintln!("plantcost: scene={other} is not community|rung");
            std::process::exit(2);
        }
    }
}
