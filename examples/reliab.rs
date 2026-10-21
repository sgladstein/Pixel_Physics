//! SCRATCH (investigation part 4, not for main): every genome solo on two independent sets of training-style worlds.
//! cargo run --release --example reliab -- <shaft> <frames> <out.csv> <genome files...>
use pixel_physics::train::challenges::Soil;
use pixel_physics::train::{self, Challenge};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

fn main() {
    std::env::set_var("PIXEL_PHYSICS_MUTATION", "off");
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (shaft, frames, out, files): (i32, u64, &str, &[String]) = (a[0].parse().unwrap(), a[1].parse().unwrap(), &a[2], &a[3..]);
    let budget = 1200.0 * frames as f32 / 6000.0;
    let soil = Soil { ants: 1, frames, step: 4, shaft, budget: Some(budget), foot: true, step_near: 0.0 };
    let gs: Vec<Vec<f32>> = files.iter().map(|f| train::start_mind(&soil, f)).collect();
    // Two independent 48-world sets, drawn the way training draws a generation's (mirrored pairs).
    let sets = [train::training_layouts(9001, 0, 48), train::training_layouts(9002, 0, 48)];
    let n = gs.len();
    let jobs: Vec<(usize, usize, usize)> = (0..2).flat_map(|s| (0..48).flat_map(move |l| (0..n).map(move |g| (s, l, g)))).collect();
    let next = AtomicUsize::new(0);
    let res = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    println!("reliab: {} genomes x 2 sets x 48 worlds, shaft {shaft}, frames {frames}, budget {budget}, threads {threads}", gs.len());
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| loop {
                let j = next.fetch_add(1, Ordering::Relaxed);
                let Some(&(s, l, g)) = jobs.get(j) else { break };
                let r = soil.run(sets[s][l], &[gs[g].as_slice()], &mut |_, _| {});
                res.lock().unwrap().push(format!("{},{},{},{},{},{}", files[g], s, l, sets[s][l].k, r[0].parts[0], r[0].parts[24]));
            });
        }
    });
    let mut v = res.into_inner().unwrap();
    v.sort();
    std::fs::write(out, format!("file,set,world,k,score,full_loads\n{}\n", v.join("\n"))).unwrap();
    // SCRATCH (teacher-training ceiling test): how often the teacher acted.
    println!("{}", pixel_physics::sim::creature::mind::scratch_log::teach_report());
    println!("wrote {out}");
}
