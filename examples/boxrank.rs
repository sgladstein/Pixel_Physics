//! Scratch (not for main): does scoring several different brains in one dig box
//! rank them the way scoring each alone does?
//!
//! cargo run --release --example boxrank -- <out.csv> <genome files...>
//!
//! For every genome: alone on the 12 sealed worlds (A), alone on the 12
//! hold-out worlds (B), and in 4-ant boxes of different genomes on the sealed
//! worlds with boxmates reshuffled per world, twice with different shuffles
//! (C1, C2). Arena: soil step=4 shaft=4 foot=1 budget=1200, as the scoring
//! command in the brain-lane handoff. One CSV row per genome: mean score and
//! mean full loads over the 12 worlds in each condition.
use pixel_physics::train::challenges::Soil;
use pixel_physics::train::{self, Challenge, Layout, Rng, Score};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

fn soil(ants: usize) -> Soil {
    Soil { ants, frames: 6000, step: 4, shaft: 4, budget: Some(1200.0), foot: true }
}

/// Every job is (layout, member genomes); returns, per job, the per-member scores.
fn run_jobs(ch: &Soil, gs: &[Vec<f32>], layouts: &[Layout], jobs: &[(usize, Vec<usize>)], threads: usize) -> Vec<Vec<Score>> {
    let next = AtomicUsize::new(0);
    let out = Mutex::new(vec![Vec::new(); jobs.len()]);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let j = next.fetch_add(1, Ordering::Relaxed);
                let Some((l, members)) = jobs.get(j) else { break };
                let g: Vec<&[f32]> = members.iter().map(|&m| gs[m].as_slice()).collect();
                let sc = ch.run(layouts[*l], &g, &mut |_, _| {});
                out.lock().unwrap()[j] = sc;
            });
        }
    });
    out.into_inner().unwrap()
}

fn main() {
    std::env::set_var("PIXEL_PHYSICS_MUTATION", "off");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (out, files) = args.split_first().expect("out.csv genome...");
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let solo = soil(1);
    let gs: Vec<Vec<f32>> = files.iter().map(|f| train::start_mind(&solo, f)).collect();
    let n = gs.len();
    assert!(n % 4 == 0, "a multiple of 4 genomes, so no box needs filler");
    let sealed = train::sealed_layouts(12);
    let hold = train::holdout_layouts(12);
    println!("boxrank: {n} genomes, threads={threads}, soil step=4 shaft=4 foot=1 budget=1200 frames=6000");
    // [genome] -> (sum score, sum full loads) per condition
    let mut table = vec![[(0.0f64, 0.0f64); 4]; n];
    let t = std::time::Instant::now();
    for (c, layouts) in [(0usize, &sealed), (1, &hold)] {
        let jobs: Vec<(usize, Vec<usize>)> = (0..layouts.len()).flat_map(|l| (0..n).map(move |g| (l, vec![g]))).collect();
        let res = run_jobs(&solo, &gs, layouts, &jobs, threads);
        for ((_, m), r) in jobs.iter().zip(&res) {
            table[m[0]][c].0 += r[0].parts[0];
            table[m[0]][c].1 += r[0].parts[24];
        }
        println!("solo {} done, {:.0} s", if c == 0 { "sealed" } else { "hold-out" }, t.elapsed().as_secs_f64());
    }
    let four = soil(4);
    let mut box_totals = Vec::new();
    for (c, seed) in [(2usize, 11u64), (3, 22)] {
        let mut rng = Rng(seed);
        let mut jobs = Vec::new();
        for l in 0..sealed.len() {
            let mut order: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                order.swap(i, rng.below(i + 1));
            }
            jobs.extend(order.chunks(4).map(|ch| (l, ch.to_vec())));
        }
        let res = run_jobs(&four, &gs, &sealed, &jobs, threads);
        for ((_, m), r) in jobs.iter().zip(&res) {
            assert_eq!(r.len(), 4, "per-ant scores");
            for (&g, s) in m.iter().zip(r) {
                table[g][c].0 += s.parts[0];
                table[g][c].1 += s.parts[1];
            }
            box_totals.push((r[0].parts[2], r[0].parts[3], r.iter().map(|s| s.parts[0]).sum::<f64>()));
        }
        println!("mixed boxes shuffle {seed} done, {:.0} s", t.elapsed().as_secs_f64());
    }
    // Positive control on the per-ant split: per-ant scores summed vs the box score (differs only by the box refill penalty).
    let (bs, bf, ps) = box_totals.iter().fold((0.0, 0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2));
    println!("box check: mean box score {:.2}, mean sum of per-ant scores {:.2}, mean box full loads {:.2}", bs / box_totals.len() as f64, ps / box_totals.len() as f64, bf / box_totals.len() as f64);
    let mut csv = String::from("file,A_score,A_full,B_score,B_full,C1_score,C1_full,C2_score,C2_full\n");
    for (f, row) in files.iter().zip(&table) {
        csv.push_str(f);
        for (s, fl) in row {
            csv.push_str(&format!(",{:.3},{:.3}", s / 12.0, fl / 12.0));
        }
        csv.push('\n');
    }
    std::fs::write(out, csv).expect("write csv");
    println!("wrote {out}, {:.0} s total", t.elapsed().as_secs_f64());
}
