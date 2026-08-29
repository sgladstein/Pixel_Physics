//! Audit instrument for `Reports/worldgen-architecture-ceilings-2026-08-29.md`.
//!
//! Three questions the existing probes cannot answer, all about what the
//! architecture can *express* rather than what one world came out like:
//!
//! * `mode=step`    — the largest single-column skyline step the generator
//!   produces, and which pass produced it. The "sharp vertical faces"
//!   half of the owner's 2026-08-22 verdict was answered once with "the
//!   largest step worldgen produces anywhere is 5 rows"; that number was
//!   measured against the terrain, and `residuals` writes vertical-sided
//!   towers *after* it. This re-measures the finished world and ablates.
//! * `mode=region`  — where region boundaries actually land, as a fraction
//!   of world width, over a seed sweep. The centres are evenly spaced with
//!   bounded jitter (`region.rs`), so the claim under test is that a world
//!   of `n` regions puts its boundaries at `i/n` +- a bounded amount.
//! * `mode=strata`  — how many distinct rock hardnesses a whole world has.
//!   `HardnessField::at` keys the draw on the band index alone, so this is
//!   a count of bands, not of places.
//!
//! Read-only: builds worlds, writes nothing, changes no default.

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material;
use pixel_physics::sim::world::World;
use pixel_physics::worldgen::{self, region::RegionMap, Spec, WorldgenPresets};

const W: i32 = 8192;
const H: i32 = 2560;

fn skyline(world: &World, w: i32, h: i32) -> Vec<i32> {
    (0..w)
        .map(|x| (0..h).find(|&y| world.get(x, y).material != material::EMPTY).unwrap_or(h))
        .collect()
}

/// Largest and 99.9th-percentile adjacent-column step in a skyline.
fn steps(tops: &[i32]) -> (i32, i32) {
    let mut d: Vec<i32> = tops.windows(2).map(|p| (p[1] - p[0]).abs()).collect();
    d.sort_unstable();
    let p999 = d[((d.len() as f32 - 1.0) * 0.999) as usize];
    (*d.last().unwrap(), p999)
}

fn main() {
    let mut mode = "step".to_string();
    let mut seeds: u64 = 6;
    let mut only = String::new();
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "mode" => mode = v.to_string(),
            "seeds" => seeds = v.parse().expect("seeds=N"),
            "preset" => only = v.to_string(),
            _ => panic!("unknown argument {arg:?}"),
        }
    }
    let (presets, err) = WorldgenPresets::load();
    if let Some(e) = err {
        eprintln!("preset load: {e}");
    }
    let names: Vec<String> = presets
        .presets
        .keys()
        .filter(|&n| only.is_empty() || *n == only)
        .filter(|&n| n != "flat")
        .cloned()
        .collect();
    println!("wg_ceilings mode={mode} seeds={seeds} world={W}x{H} presets={names:?}");

    match mode.as_str() {
        // ---- the skyline-step question -------------------------------
        "step" => {
            // Which passes to switch off, to attribute the step. `None` is
            // the full build.
            // `""` skips nothing: the FULL arm goes through the *same*
            // `generate_ablated` path as the others, so the two arms differ
            // only by the pass under test. `generate` would also run the
            // structural pass, which is a second difference even though it
            // writes no material -- CLAUDE.md's "an A/B whose arms differed
            // in two things".
            let ablate: [&str; 4] = ["", "residuals", "brows", "boulders"];
            println!("\n  largest single-column skyline step, cells (p99.9 in brackets)");
            for name in &names {
                let p = presets.get(name).expect("preset");
                for a in ablate {
                    let mut worst = 0;
                    let mut worst_p999 = 0;
                    for s in 0..seeds {
                        let mut world = World::new(Rect::new(0, 0, W - 1, H - 1));
                        worldgen::generate_ablated(
                            &mut world,
                            Spec::Generated { params: p, seed: s },
                            a,
                        );
                        let (mx, p999) = steps(&skyline(&world, W, H));
                        worst = worst.max(mx);
                        worst_p999 = worst_p999.max(p999);
                    }
                    // One tagged line per cell: the passes print during
                    // generation, so a row built with `print!` interleaves.
                    println!(
                        "STEP {name:<10} {:<14} max {worst:>4}  p99.9 {worst_p999:>4}",
                        if a.is_empty() { "FULL" } else { a }
                    );
                }
            }
        }
        // ---- where region boundaries land ----------------------------
        "region" => {
            println!("\n  boundary positions as a fraction of world width");
            println!("  a boundary is the midpoint between two adjacent region centres");
            for name in &names {
                let p = presets.get(name).expect("preset");
                let mut by_count: std::collections::BTreeMap<usize, Vec<f32>> =
                    Default::default();
                for s in 0..seeds {
                    let map = RegionMap::new(s, p, W);
                    // Recover the centres by scanning for the flat cores:
                    // sample() holds each end region, so the boundaries are
                    // where the blended character is exactly halfway. Cheaper
                    // and exact: read the transition midpoints off `len()`
                    // by locating each maximal run of constant character.
                    let mut runs: Vec<(i32, i32)> = Vec::new();
                    let (mut s0, mut prev) = (0, map.sample(0));
                    for x in 1..W {
                        let c = map.sample(x);
                        if c != prev {
                            if x - s0 > 1 {
                                runs.push((s0, x - 1));
                            }
                            s0 = x;
                            prev = c;
                        }
                    }
                    runs.push((s0, W - 1));
                    // Cores are the runs longer than one column: the flat
                    // parts either side of each transition band.
                    let cores: Vec<(i32, i32)> =
                        runs.into_iter().filter(|r| r.1 - r.0 >= 8).collect();
                    let n = cores.len();
                    let bounds: Vec<f32> = cores
                        .windows(2)
                        .map(|w| ((w[0].1 + w[1].0) as f32 * 0.5) / W as f32)
                        .collect();
                    by_count.entry(n).or_default().extend(bounds);
                }
                println!("\n  {name}:");
                for (n, mut b) in by_count {
                    b.sort_by(f32::total_cmp);
                    // Expected fractions for n regions: i/n.
                    let expect: Vec<String> =
                        (1..n).map(|i| format!("{:.3}", i as f32 / n as f32)).collect();
                    let got: Vec<String> = b.iter().map(|v| format!("{v:.3}")).collect();
                    println!("    {n} regions -> expected i/n {expect:?}");
                    println!("               observed        {got:?}");
                }
            }
        }
        // ---- how many rock hardnesses a world has --------------------
        "strata" => {
            println!("\n  distinct strata bands in one world (hardness is drawn per BAND index)");
            println!("  band index = ((datum - e) + strata_tilt*x + strata_fold*fbm(x)) / strata_thickness");
            for name in &names {
                let p = presets.get(name).expect("preset");
                // Band index range over the whole world: the elevation span
                // is the world height; the lateral term adds tilt*W plus the
                // fold's own amplitude, both in cells.
                let lateral = p.strata_tilt * W as f32 + 2.0 * p.strata_fold;
                let bands = (H as f32 + lateral) / p.strata_thickness.max(3.0);
                println!(
                    "    {name:<10} thickness {:>5.1}  tilt {:>5.3} (={:>6.0} cells over the world)  fold +-{:>4.1}  => {:>5.0} bands",
                    p.strata_thickness, p.strata_tilt, p.strata_tilt * W as f32, p.strata_fold, bands
                );
            }
        }
        other => panic!("unknown mode {other:?}"),
    }
}
