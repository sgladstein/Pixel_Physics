//! SCRATCH (not for main): children of one genome by train::mutate, with an optional
//! no-prune mask of the parent's links above |w| > tau (research lane Q1, arm A').
//! mutants <start> <n> <seed> <out> pm sigma prune grow [tau]
use pixel_physics::train::challenges::Soil;
use pixel_physics::train::{self, Rng};
use std::path::Path;
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (start, n, seed, out): (&str, usize, u64, &str) = (&a[0], a[1].parse().unwrap(), a[2].parse().unwrap(), &a[3]);
    let soil = Soil { ants: 1, frames: 12000, step: 4, shaft: 8, budget: Some(2400.0), foot: true, step_near: 0.0 };
    let g = train::start_mind(&soil, start);
    let slots = train::trainable_for(&soil, g.len());
    let num = |i: usize| a[i].parse::<f32>().unwrap();
    let (pm, sigma, prune, grow) = (num(4), num(5), num(6), num(7));
    let tau: Option<f32> = a.get(8).map(|s| s.parse().unwrap());
    let keep: Vec<bool> = match tau { Some(t) => g.iter().map(|w| w.abs() > t).collect(), None => vec![] };
    println!("mutants: pm {pm} sigma {sigma} prune {prune} grow {grow} tau {tau:?} kept {}", keep.iter().filter(|&&k| k).count());
    let mut rng = Rng(seed.wrapping_mul(0xA24B_AED4_963E_E407) ^ 0x7EA1);
    std::fs::create_dir_all(out).unwrap();
    for k in 0..n {
        let c = train::mutate(&g, &slots, pm, sigma, prune, grow, &keep, &mut rng);
        train::save_genome(&Path::new(out).join(format!("m{k:02}.txt")), &c).unwrap();
    }
}
