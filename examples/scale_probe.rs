//! What a bigger world actually costs, measured rather than extrapolated.
//!
//! **Written when the owner had asked for a 4x-linear world (8192x2560,
//! sixteen times the cells) and it was still a proposal; it now ships** —
//! `app::WORLD_WIDTH`/`WORLD_HEIGHT` are 8192x2560. At the time, every
//! number available for that decision was an *extrapolation* from the then-
//! shipped 2048x640 size — generation scales with area, so 542 ms becomes
//! ~8.7 s; the field solve was 7.2 ms at 2048x1280 and touches every field
//! cell in the world every frame, so ~57 ms. Extrapolations across a 16x
//! range are exactly the kind of number this repo keeps being wrong about,
//! and the whole of the scale plan's first phase was sized off them — this
//! tool is what replaced the extrapolation with a measurement.
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
//!
//! `scales=` multiplies `BASE_W`/`BASE_H` below, which are still 2048x640 —
//! so the default `1,2,3,4` now tops out at `4` = 8192x2560, the size that
//! ships, rather than at some multiple beyond it. To measure past the
//! shipped size, `size=WxH` takes an explicit size directly and ignores
//! `scales` entirely.

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{field, parallel, structural};
use pixel_physics::worldgen::{self, Spec, WorldgenPresets};
use std::time::Instant;

/// How still counts as still, and for how long. The pressure epsilon is the
/// engine's own notion of "not changing"; `QUIET_RUN` frames of it in a row
/// is what makes the verdict robust to a field that is only stepped on some
/// frames.
const QUIET_MS: f64 = 1.0;
const QUIET_RUN: usize = 120;

/// The size every `scales=` multiplier is taken against. **Was** the
/// shipped size when this was written; the world has grown since and this
/// was deliberately left unmoved (see the module doc), so `scales=4` now
/// lands exactly on the shipped 8192x2560 rather than a size beyond it.
/// `size=WxH` is the way to probe past that without touching this.
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

        // A checksum of the finished world, so an "optimisation" can be held
        // to producing the *same world*. Tests passing is not that: the suite
        // asserts properties, and a distance field that is subtly different
        // but still plausible passes every one of them. Two builds printing
        // the same number is the claim worth making.
        if std::env::var("WORLD_HASH").is_ok() {
            let mut acc: u64 = 0xcbf2_9ce4_8422_2325;
            for y in 0..h {
                for x in 0..w {
                    let c = world.get(x, y);
                    for byte in [
                        c.material.0 as u64,
                        c.aux() as u64,
                        c.shade as u64,
                        u64::from(c.organism_id()),
                    ] {
                        acc ^= byte;
                        acc = acc.wrapping_mul(0x100_0000_01b3);
                    }
                }
            }
            println!("  world hash {w}x{h}: {acc:016x}");
        }

        // **Step until the world is genuinely quiet, and report how long that
        // took**, rather than stepping a fixed count and calling the result
        // "settled".
        //
        // The first version of this probe did the latter and got it badly
        // wrong: it ran 400 frames, reported 8.26 ms as the settled field
        // cost at the shipped size, and that number went into a report as a
        // standing defect. It was the *transient*. A generated world's field
        // takes thousands of frames to converge -- pressure decays cleanly
        // but slowly -- and 400 frames is nowhere near it. `frames` is now a
        // cap, not a target, and `to_quiet` is the headline number: it is how
        // long a player waits before the world stops costing what it costs
        // here.
        let (mut still, mut worst_sweep, mut worst_field) = (0usize, 0.0f64, 0.0f64);
        let mut to_quiet: Option<usize> = None;
        for i in 0..frames {
            let t = Instant::now();
            parallel::step(&mut world);
            worst_sweep = worst_sweep.max(t.elapsed().as_secs_f64() * 1000.0);
            let t = Instant::now();
            field::step(&mut world);
            let field_ms = t.elapsed().as_secs_f64() * 1000.0;
            worst_field = worst_field.max(field_ms);
            // **Quiet is measured by the cost itself, after two other
            // definitions of it turned out to be blind.**
            //
            // `world.fields_settled()` latched true at frame 47 while the
            // field was demonstrably still churning -- it is a verdict over
            // the tiles actually solved that frame, so an empty or lucky
            // solve set reads as "everything settled". And a lattice of
            // `field_at_bilinear` probe points reported quiet at frame 0,
            // because points spread evenly over a world mostly land in solid
            // rock where the pressure never moves at all. Both were asking a
            // question that can be answered vacuously.
            //
            // The wall time cannot be. Measured, the two states are 40x
            // apart -- 42 ms/frame while converging against 0.01 ms once
            // converged -- so a 1 ms threshold separates them at every world
            // size, and `QUIET_RUN` consecutive frames under it is robust to
            // a field that only steps on some frames.
            if field_ms < QUIET_MS {
                still += 1;
            } else {
                still = 0;
            }
            if to_quiet.is_none() && still >= QUIET_RUN {
                to_quiet = Some(i + 1 - QUIET_RUN);
            }
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
        let quiet = match to_quiet {
            Some(f) => format!("{f}"),
            None => format!(">{frames}"),
        };

        println!(
            "{:>11} {:>10} {:>8.0}ms {:>9.0}ms {:>8.0}ms {:>7.0}MiB {:>7.2}ms {:>7.2}ms | \
             sweep {:>6.2}/{:<6.2} field {:>6.2}/{:<6.2} awake {} quiet@{}",
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
            quiet,
        );
    }

    println!(
        "\nplace = the pass table; structural = compute_world_distances (allocates a second\n\
         full grid); sweep/field = worst single frame over the run; settled = worst frame\n\
         of both together once the world has gone quiet. peakRSS is cumulative across sizes\n\
         in one process, so read it from the largest row or run one size at a time."
    );
}
