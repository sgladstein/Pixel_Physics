//! SCRATCH (teacher-training step 1, not for main): run one genome on a set of training-style worlds with the copier's
//! log on, and write each world's score.
//!
//! cargo run --release --example copylog -- <soil|fetch|trail> <genome> <run_seed> <n> <out.csv> [threads]
//!
//! The genome is loaded **through the suite** (soil 8 rows, fetch, trail -- the dig-8 runs' arenas), as training would
//! load it, so no arena's bans are baked in; each arena applies its own when it runs. `PP_GENOME_OUT=<file>` saves the
//! genome as loaded (the copier's starting point). `PP_COPY_LOG=<file>` turns the log on; `PP_TEACH_LABEL=1` adds the
//! teacher's state labels (soil only). Worlds are `train::training_layouts(run_seed, 0, n)`: pick run seeds that no
//! other set uses (the 96 evaluation worlds are 9001 and 9002).
use pixel_physics::sim::creature;
use pixel_physics::sim::creature::mind::scratch_log;
use pixel_physics::train::challenges::{Scarce, Soil, Suite, Trail};
use pixel_physics::train::{self, Challenge};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

fn soil() -> Soil {
    Soil { ants: 1, frames: 12000, step: 4, shaft: 8, budget: Some(2400.0), foot: true, step_near: 0.0 }
}
fn fetch() -> Scarce {
    Scarce {
        ants: 1,
        frames: 6000,
        drip: 5,
        every: 1000,
        game_walk: false,
        brake: Some(creature::FoodBrake::FORAGE),
        dist: (10, 30),
        steps: true,
        fed: true,
        dig: false,
    }
}
fn trail() -> Trail {
    Trail { ants: 6, frames: 6000, clones: false }
}

fn main() {
    std::env::set_var("PIXEL_PHYSICS_MUTATION", "off");
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (arena, genome, run_seed, n, out) = (a[0].as_str(), a[1].as_str(), a[2].parse::<u64>().unwrap(), a[3].parse::<usize>().unwrap(), a[4].as_str());
    let threads = a.get(5).map_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()), |t| t.parse().unwrap());
    let suite = Suite::new(vec![(Box::new(soil()), false), (Box::new(fetch()), true), (Box::new(trail()), true)], 0.8);
    let g = train::start_mind(&suite, genome);
    if let Ok(p) = std::env::var("PP_GENOME_OUT") {
        train::save_genome(std::path::Path::new(&p), &g).unwrap();
    }
    let ch: Box<dyn Challenge> = match arena {
        "soil" => Box::new(soil()),
        "fetch" => Box::new(fetch()),
        "trail" => Box::new(trail()),
        _ => panic!("arena: soil|fetch|trail"),
    };
    let lays = train::training_layouts(run_seed, 0, n);
    println!(
        "copylog: {arena}, {genome}, run seed {run_seed}, {n} worlds, {threads} threads, copy log {}, labels {}",
        scratch_log::copy_on(),
        scratch_log::label_on()
    );
    let next = AtomicUsize::new(0);
    let res = Mutex::new(Vec::new());
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| loop {
                let k = next.fetch_add(1, Ordering::Relaxed);
                let Some(&lay) = lays.get(k) else { break };
                scratch_log::WORLD.with(|w| w.set(k as u64));
                let r = ch.run(lay, &[g.as_slice()], &mut |_, _| {});
                let full = if arena == "soil" { r[0].parts[24] } else { f64::NAN };
                res.lock().unwrap().push((k, lay.seed, r[0].fit, full));
            });
        }
    });
    scratch_log::copy_flush();
    let mut v = res.into_inner().unwrap();
    v.sort_by_key(|r| r.0);
    let rows: Vec<String> = v.iter().map(|(k, s, f, l)| format!("{k},{s},{f},{l}")).collect();
    std::fs::write(out, format!("world,seed,score,full_loads\n{}\n", rows.join("\n"))).unwrap();
    let mean = v.iter().map(|r| r.2).sum::<f64>() / v.len() as f64;
    println!("wrote {out}: mean score {mean:.3}");
    println!("{}", scratch_log::teach_report());
}
