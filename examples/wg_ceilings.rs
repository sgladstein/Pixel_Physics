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
//! * `mode=strata`  — how many distinct rock hardnesses a whole world has
//! * `mode=relief`  — **the three numbers W1 is judged on**, in one command:
//!   how far the skyline moves across one player screen, how tall the
//!   tallest thing at formation reach (15 and 30 columns) is, and what
//!   fraction of on-screen ground lies within six cells of air. Each was
//!   measured by a different lane with a different harness
//!   (`worldgen-visual-interest-2026-08-29.md` §1.1 and §2.2,
//!   `viewshot boulder=1`'s prominence table), which made a before/after
//!   three commands and three definitions; this is one arm of one binary so
//!   the two sides cannot drift apart. Prints per-preset order statistics
//!   over a seed sweep, never a single seed..
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

/// Prominence at a given reach: how far a column stands above the lower of
/// the two columns `reach` away on each side.
///
/// **The definition is copied from `viewshot.rs`'s prominence table on
/// purpose**, down to the clamping at the world edge, so a number out of this
/// harness can be read against every prominence figure already on record.
/// The reach is a scale and a single one cannot see past it — a 40-cell tor
/// twelve columns wide scores zero at reach 5, because both sample points
/// land on top of the tor.
fn prominence(tops: &[i32], reach: i32) -> Vec<i32> {
    let w = tops.len() as i32;
    (reach..w - reach)
        .map(|x| {
            let l = tops[(x - reach) as usize];
            let r = tops[(x + reach) as usize];
            (l - tops[x as usize]).min(r - tops[x as usize])
        })
        .collect()
}

/// **Local relief** at a given reach: the range of the skyline over the
/// `2*reach+1` columns centred on each one.
///
/// The companion to `prominence`, and the one to read for *"is there a rock
/// formation at this scale"*. Prominence is two-sided — it asks how far a
/// column stands above the ground on *both* sides — so it scores a spire and
/// scores **zero** on a scarp, a bench rim, a mesa edge and the whole
/// interior of a plateau. It is therefore the wrong statistic to steer by
/// when the brief is *"large rock formation (not just tall pillars)"*: a
/// change that turned every mesa in the world into a pillar would improve it.
/// Local relief counts any vertical structure at that width, whichever side
/// of it the ground is on. Both are printed; neither is enough alone.
fn local_relief(tops: &[i32], reach: i32) -> Vec<i32> {
    let w = tops.len() as i32;
    (reach..w - reach)
        .map(|x| {
            let win = &tops[(x - reach) as usize..=(x + reach) as usize];
            win.iter().max().unwrap() - win.iter().min().unwrap()
        })
        .collect()
}

/// `q`-quantile of an already-sorted slice.
fn quant(v: &[i32], q: f32) -> i32 {
    v[((v.len() as f32 - 1.0) * q) as usize]
}

/// Ground cells within `ER` cells of air, as a fraction of ground cells, over
/// a viewport-sized rect aimed at the skyline.
///
/// **The ceiling on anything that shades, outlines or textures the ground**,
/// and the definition is `terrain_shade.rs`'s so the two agree: measured off
/// true occupancy rather than off a rendered image, because deep air and
/// night rock are both near-black and an image-derived mask loses exactly the
/// places with the most boundary in them.
fn boundary_fraction(world: &World, cam: (i32, i32), vw: i32, vh: i32) -> (usize, usize, usize) {
    const ER: i32 = 6;
    // Occupancy for the viewport plus a margin of `ER`, read once. The naive
    // form asks `World::get` 169 times per ground cell and made this harness
    // twenty minutes per arm; the box test below is **separable**, so it costs
    // two 13-tap passes instead of one 169-tap one over a buffer that is
    // already in cache.
    let (bw, bh) = ((vw + 2 * ER) as usize, (vh + 2 * ER) as usize);
    let mut open = vec![false; bw * bh];
    for j in 0..bh {
        for i in 0..bw {
            let (x, y) = (cam.0 + i as i32 - ER, cam.1 + j as i32 - ER);
            open[j * bw + i] =
                x < 0 || y < 0 || x >= W || y >= H || world.get(x, y).material == material::EMPTY;
        }
    }
    // Horizontal reach, then vertical: "is any cell within the box empty" is
    // a max over a square, and a max over a square is a max over a row of
    // maxes over columns.
    let mut hor = vec![false; bw * bh];
    for j in 0..bh {
        for i in 0..bw {
            hor[j * bw + i] =
                (i.saturating_sub(ER as usize)..=(i + ER as usize).min(bw - 1)).any(|k| open[j * bw + k]);
        }
    }
    let (mut ground, mut near) = (0usize, 0usize);
    for j in 0..vh as usize {
        for i in 0..vw as usize {
            let (bi, bj) = (i + ER as usize, j + ER as usize);
            if open[bj * bw + bi] {
                continue;
            }
            ground += 1;
            if (bj - ER as usize..=bj + ER as usize).any(|k| hor[k * bw + bi]) {
                near += 1;
            }
        }
    }
    (near, ground, (vw * vh) as usize)
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
        // ---- relief: the three numbers W1 is judged on ---------------
        "relief" => {
            // A player screen, in cells. `world_look`'s unit and the unit
            // §1.1's "rise and fall over one screen" was measured in --
            // a census over the whole 8192-wide world answers a different
            // question, because a world that rises 400 cells from end to end
            // is still flat on every screen of it.
            const SCREEN_W: i32 = 512;
            const SCREEN_H: i32 = 320;
            println!("\n  three measures, over {seeds} seeds x each preset, at the shipped world size");
            println!("  screen: skyline (max-min) over each {SCREEN_W}-column window, and mean |step| per column");
            println!("  reach15/reach30: prominence at formation scale over the whole world -- p99 and max");
            println!("  near-air: ground cells within 6 of air, over {SCREEN_W}x{SCREEN_H} viewports aimed at the skyline");
            for name in &names {
                let p = presets.get(name).expect("preset");
                let mut screens: Vec<i32> = Vec::new();
                let (mut step_sum, mut step_n) = (0i64, 0i64);
                let mut r15: Vec<i32> = Vec::new();
                let mut r30: Vec<i32> = Vec::new();
                let mut l15: Vec<i32> = Vec::new();
                let mut l30: Vec<i32> = Vec::new();
                let (mut near, mut ground, mut viewcells) = (0usize, 0usize, 0usize);
                let mut passes: std::collections::BTreeMap<&'static str, usize> = Default::default();
                let mut gen_ms = 0.0f64;
                for s in 0..seeds {
                    let mut world = World::new(Rect::new(0, 0, W - 1, H - 1));
                    let started = std::time::Instant::now();
                    let report = worldgen::generate_reported(
                        &mut world,
                        Spec::Generated { params: p, seed: s },
                    );
                    gen_ms += started.elapsed().as_secs_f64() * 1000.0;
                    for (n, c) in report {
                        *passes.entry(n).or_default() += c;
                    }
                    let tops = skyline(&world, W, H);
                    for win in tops.chunks(SCREEN_W as usize) {
                        if win.len() < SCREEN_W as usize {
                            continue;
                        }
                        screens.push(win.iter().max().unwrap() - win.iter().min().unwrap());
                    }
                    for pair in tops.windows(2) {
                        step_sum += (pair[1] - pair[0]).abs() as i64;
                        step_n += 1;
                    }
                    r15.extend(prominence(&tops, 15));
                    r30.extend(prominence(&tops, 30));
                    l15.extend(local_relief(&tops, 15));
                    l30.extend(local_relief(&tops, 30));
                    // Viewports aimed at the ground, spread across the world,
                    // the same rule `world_look` uses: the camera's top edge
                    // puts the local skyline a third of the way down, so the
                    // frame is ground rather than sky.
                    for k in 0..8 {
                        let cx = (W / 8) * k + (W / 16) - SCREEN_W / 2;
                        let cx = cx.clamp(0, W - SCREEN_W);
                        let mid = tops[(cx + SCREEN_W / 2) as usize];
                        let cy = (mid - SCREEN_H / 3).clamp(0, H - SCREEN_H);
                        let (n, g, v) = boundary_fraction(&world, (cx, cy), SCREEN_W, SCREEN_H);
                        near += n;
                        ground += g;
                        viewcells += v;
                    }
                }
                screens.sort_unstable();
                r15.sort_unstable();
                r30.sort_unstable();
                l15.sort_unstable();
                l30.sort_unstable();
                println!(
                    "RELIEF {name:<10} screen med {:>4} p90 {:>4} max {:>4} | mean|step| {:>5.2} | reach15 p99 {:>4} max {:>4} | reach30 p99 {:>4} max {:>4} | near-air {:>5.1}% | gen {:>6.0} ms/world",
                    quant(&screens, 0.5),
                    quant(&screens, 0.9),
                    screens[screens.len() - 1],
                    step_sum as f64 / step_n.max(1) as f64,
                    quant(&r15, 0.99),
                    r15[r15.len() - 1],
                    quant(&r30, 0.99),
                    r30[r30.len() - 1],
                    100.0 * near as f64 / ground.max(1) as f64,
                    gen_ms / seeds as f64,
                );
                // **The denominator, printed beside the ratio.** `near-air` is
                // a share of *ground on screen*, and relief moves both terms:
                // a viewport on a mountain flank is more solid than one on a
                // plain, so the share can fall while the boundary itself has
                // grown. Reading the ratio alone said exactly that once. The
                // cells-per-screen figure is the one that answers "is there
                // more surface", and the ground share says which way the
                // denominator went.
                println!(
                    "SURFACE {name:<10} ground {:>5.1}% of viewport | near-air {:>7.0} cells per screen",
                    100.0 * ground as f64 / viewcells.max(1) as f64,
                    near as f64 / (viewcells.max(1) / (SCREEN_W * SCREEN_H) as usize).max(1) as f64,
                );
                println!(
                    "FORM   {name:<10} local relief reach15 med {:>4} p90 {:>4} p99 {:>4} | reach30 med {:>4} p90 {:>4} p99 {:>4}",
                    quant(&l15, 0.5),
                    quant(&l15, 0.9),
                    quant(&l15, 0.99),
                    quant(&l30, 0.5),
                    quant(&l30, 0.9),
                    quant(&l30, 0.99),
                );
                // The starved landform passes, in the same line of sight:
                // `brows` and `talus` key on a cliff test the terrain has
                // never cleared, so their cell counts are the cheapest
                // confirmation that new relief is real rather than a
                // statistic.
                let cells = |n: &str| passes.get(n).copied().unwrap_or(0) / seeds.max(1) as usize;
                println!(
                    "STARVED {name:<10} brows {:>8}  talus {:>8}  boulders {:>7}  residuals {:>8}  (cells/world)",
                    cells("brows"),
                    cells("talus"),
                    cells("boulders"),
                    cells("residuals"),
                );
            }
        }
        other => panic!("unknown mode {other:?}"),
    }
}
