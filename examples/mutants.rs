//! SCRATCH (teacher-training measurement 1): children of one genome, made by the trainer's own mutation
//! rule at the dig-8 runs' settings (pm 0.03, sigma 0.3, prune 0.3, grow 0.1; no crossover), for the paired-world test.
//! cargo run --release --example mutants -- <start genome> <n> <seed> <out dir> [pm sigma prune grow]
use pixel_physics::train::challenges::Soil;
use pixel_physics::train::{self, Rng};
use std::path::Path;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (start, n, seed, out): (&str, usize, u64, &str) = (&a[0], a[1].parse().unwrap(), a[2].parse().unwrap(), &a[3]);
    let soil = Soil { ants: 1, frames: 12000, step: 4, shaft: 8, budget: Some(2400.0), foot: true, step_near: 0.0, nest: false };
    let g = train::start_mind(&soil, start);
    let slots = train::trainable_for(&soil, g.len());
    let mut rng = Rng(seed.wrapping_mul(0xA24B_AED4_963E_E407) ^ 0x7EA1);
    let num = |i: usize, d: f32| a.get(i).map_or(d, |s| s.parse().unwrap());
    let (pm, sigma, prune, grow) = (num(4, 0.03), num(5, 0.3), num(6, 0.3), num(7, 0.1));
    println!("mutants: pm {pm} sigma {sigma} prune {prune} grow {grow}");
    std::fs::create_dir_all(out).unwrap();
    let live = |g: &[f32]| g.iter().filter(|w| **w != 0.0).count();
    for k in 0..n {
        let c = train::mutate(&g, &slots, pm, sigma, prune, grow, &vec![false; g.len()], &mut rng);
        let changed = g.iter().zip(&c).filter(|(a, b)| a != b).count();
        let p = Path::new(out).join(format!("m{k:02}.txt"));
        train::save_genome(&p, &c).unwrap();
        println!("{} changed {changed} live {} (parent {})", p.display(), live(&c), live(&g));
    }
}
