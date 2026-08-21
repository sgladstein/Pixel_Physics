//! What a bigger world actually costs, measured rather than extrapolated.
//!
//! The owner has asked for a 4x-linear world (8192x2560, sixteen times the
//! cells). Every number available for that decision was an *extrapolation*
//! from the shipped 2048x640 size — generation scales with area, so 542 ms
//! becomes ~8.7 s; the field solve was 7.2 ms at 2048x1280 and touches every
//! field cell in the world every frame, so ~57 ms. Extrapolations across a
//! 16x range are exactly the kind of number this repo keeps being wrong
//! about, and the whole of the scale plan's first phase is sized off them.
//!
//! So: build the thing and time it. Reports, per size:
//!
//! - **generation**, split into the two halves that matter — the pass table
//!   (`generate_only`) and `structural::compute_world_distances`, which a
//!   review put at 45% of the total and which allocates a second full-grid
//!   mirror.
//! - **peak RSS**, read from `/proc/self/status: VmHWM`, so the transient
//!   generation spike is visible and not just the steady grid.
//! - **frame cost**, split into the CA sweep (`parallel::step`, the driver
//!   the app runs) and `field::step`, separately while the world is still
//!   settling and once it has gone quiet. The settled number is the one that
//!   decides whether a big world is playable, because a world that costs
//!   57 ms/frame with nothing happening is not a world.
//!
//! ```text
//! cargo run --release --example scale_probe                  # 1x, 2x, 3x, 4x
//! cargo run --release --example scale_probe -- scales=1,4
//! cargo run --release --example scale_probe -- size=8192x2560 frames=120
//! ```

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{field, parallel, structural};
use pixel_physics::worldgen::{self, Spec, WorldgenPresets};
use std::time::Instant;

/// The shipped size every multiplier is taken against.
const BASE_W: i32 = 2048;
const BASE_H: i32 = 640;

/// Peak resident set size in MiB, from the kernel rather than from a guess.
///
/// `VmHWM` is the high-water mark, which is the point: the generation spike
/// (`compute_world_distances` holds a second `Vec<Cell>` the size of the
/// whole grid, plus a `Vec<u16>` of distances) is invisible to any
/// steady-state accounting and is what decides whether a size fits.
fn peak_rss_mib() -> f64 {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else { return f64::NAN };
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            if let Some(kb) = rest.split_whitespace().next().and_then(|t| t.parse::<f64>().ok()) {
                return kb / 1024.0;
            }
        }
    }
    f64::NAN
}

fn main() {
    let mut scales: Vec<i32> = vec![1, 2, 3, 4];
    let mut explicit: Option<(i32, i32)> = None;
    let mut frames = 240usize;
    let mut preset = String::new();
    let mut seed = 1u64;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "scales" => scales = v.split(',').map(|t| t.parse().expect("scales=1,2,4")).collect(),
            "size" => {
                let (w, h) = v.split_once('x').expect("size=WxH");
                explicit = Some((w.parse().expect("width"), h.parse().expect("height")));
            }
            "frames" => frames = v.parse().expect("frames=N"),
            "preset" => preset = v.to_string(),
            "seed" => seed = v.parse().expect("seed=N"),
            _ => eprintln!("ignoring unknown argument {arg}"),
        }
    }

    let (presets, err) = WorldgenPresets::load();
    if let Some(e) = err {
        eprintln!("preset load: {e}");
    }
    let name = if preset.is_empty() { presets.default_name() } else { preset.clone() };
    let params = presets.get(&name).unwrap_or_else(|| panic!("unknown preset {name:?}")).clone();

    let sizes: Vec<(i32, i32)> = match explicit {
        Some(wh) => vec![wh],
        None => scales.iter().map(|&s| (BASE_W * s, BASE_H * s)).collect(),
    };

    println!("preset {name}, seed {seed}, {frames} frames per size\n");
    println!(
        "{:>11} {:>10} {:>9} {:>10} {:>9} {:>9} {:>9} {:>9}   settled mean/worst, and awake chunks",
        "size", "cells", "place", "structural", "gen", "peakRSS", "sweep!", "field!"
    );
    println!("{}", "-".repeat(140));

    for (w, h) in sizes {
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));

        let t = Instant::now();
        worldgen::generate_only(&mut world, Spec::Generated { params: &params, seed });
        let place_ms = t.elapsed().as_secs_f64() * 1000.0;

        let t = Instant::now();
        structural::compute_world_distances(&mut world);
        let structural_ms = t.elapsed().as_secs_f64() * 1000.0;

        // Worst frame, not mean: a mean hides the frame that drops the
        // player's input, and the worst is what `ascii` already gates on.
        // Sweep and field are timed apart because they scale differently --
        // the sweep is proportional to *awake* chunks and the field's sky
        // walk is proportional to the whole world.
        let (mut worst_sweep, mut worst_field) = (0.0f64, 0.0f64);
        for _ in 0..frames {
            let t = Instant::now();
            parallel::step(&mut world);
            worst_sweep = worst_sweep.max(t.elapsed().as_secs_f64() * 1000.0);
            let t = Instant::now();
            field::step(&mut world);
            worst_field = worst_field.max(t.elapsed().as_secs_f64() * 1000.0);
        }

        // Then the state the optimisation exists for: nothing moving. A
        // settled world is where the dirty-rect skip and the field's own
        // early-out do their work, and where a whole-world-per-frame pass
        // has nowhere to hide.
        //
        // **Split, and reported next to the awake-chunk count**, because the
        // first version of this probe reported 11 ms "settled" at the shipped
        // size against a recorded 0.008 ms, and a number that disagrees with
        // the record by three orders of magnitude is a question about the
        // probe first. Without the awake count there is no way to tell a slow
        // settled world from a world that never settled.
        let (mut worst_ss, mut worst_sf) = (0.0f64, 0.0f64);
        let (mut sum_ss, mut sum_sf) = (0.0f64, 0.0f64);
        for _ in 0..60 {
            let t = Instant::now();
            parallel::step(&mut world);
            let d = t.elapsed().as_secs_f64() * 1000.0;
            worst_ss = worst_ss.max(d);
            sum_ss += d;
            let t = Instant::now();
            field::step(&mut world);
            let d = t.elapsed().as_secs_f64() * 1000.0;
            worst_sf = worst_sf.max(d);
            sum_sf += d;
        }
        let awake = world.active_chunk_count();

        println!(
            "{:>11} {:>10} {:>8.0}ms {:>9.0}ms {:>8.0}ms {:>7.0}MiB {:>7.2}ms {:>7.2}ms | \
             sweep {:>6.2}/{:<6.2} field {:>6.2}/{:<6.2} awake {}",
            format!("{w}x{h}"),
            w as i64 * h as i64,
            place_ms,
            structural_ms,
            place_ms + structural_ms,
            peak_rss_mib(),
            worst_sweep,
            worst_field,
            sum_ss / 60.0,
            worst_ss,
            sum_sf / 60.0,
            worst_sf,
            awake,
        );
    }

    println!(
        "\nplace = the pass table; structural = compute_world_distances (allocates a second\n\
         full grid); sweep/field = worst single frame over the run; settled = worst frame\n\
         of both together once the world has gone quiet. peakRSS is cumulative across sizes\n\
         in one process, so read it from the largest row or run one size at a time."
    );
}
