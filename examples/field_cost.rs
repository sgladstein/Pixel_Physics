//! What the coarse field actually costs per frame, and what decides it.
//!
//! # The finding this exists to record
//!
//! `scale_probe` measured a fresh world costing 30-50 ms/frame in `field::
//! step` for its first ~4500 frames and then dropping to near zero, and that
//! was written up -- twice -- as a **load transient**: generated terrain
//! starting far from field equilibrium and relaxing. The planned fix was to
//! seed the field at load.
//!
//! It is not a transient and generation has nothing to do with it. **It is
//! the wind.** `weather::at(seed, frame)` is a pure function of the frame, and
//! at seed 1 it opens on a gale: wind 0.963, falling below `GUST_THRESHOLD`
//! (0.45) at frame **3704**. `weather::gust` fires a +-34 pressure dipole
//! every 26 frames for the whole of that stretch, and the field goes quiet at
//! 4501 -- about 800 frames after the last gust, which is how long one takes
//! to disperse. Three ablations pinned it:
//!
//! - **Terrain is at rest by frame 7** (`mode=ca`: zero awake chunks). The CA
//!   is not driving anything, so "generated terrain is far from equilibrium"
//!   was never a live hypothesis; nothing was moving to be far from it.
//! - **Per-channel attribution** (`FIELD_DRIFT=N`) puts every unsettled tile
//!   on pressure, with peak frame-to-frame swings of 10.7, 13.5 and **14.7 at
//!   frame 2400** -- larger than at frame 1200. A decaying transient does not
//!   get louder. Repeated injection does.
//! - **Starting the same world at a calm frame** costs what a settled world
//!   costs. That is the comparison `weather=` runs below.
//!
//! So the cost is not paid once at load. It is paid **whenever it is windy**,
//! for as long as the wind blows, forever -- and it is the dominant per-frame
//! cost in the engine at 4x.
//!
//! # What it measures
//!
//! One generated world per size, re-run at several starting frames. Weather
//! is a pure function of `(seed, frame)`, so moving `world.frame` moves the
//! weather and changes nothing else -- a paired comparison in the sense
//! `CLAUDE.md` asks for, cancelling terrain, seed and machine.
//!
//! ```text
//! cargo run --release --example field_cost -- size=2048x640
//! cargo run --release --example field_cost -- size=8192x2560 weather=0,6000 frames=300
//! FIELD_DRIFT=200 cargo run --release --example field_cost -- frames=1000
//! ```

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{field, parallel, structural, weather};
use pixel_physics::worldgen::{self, Spec, WorldgenPresets};
use std::time::Instant;

/// `weather::gust`'s own threshold, duplicated here because it is private and
/// this probe only classifies frames by it -- it never has to agree exactly,
/// and a wrong copy would show up as a bucket that does not separate.
const GUST_THRESHOLD: f32 = 0.45;

/// A cost bucket: how many frames landed in it and what they cost.
#[derive(Default, Clone, Copy)]
struct Bucket {
    frames: usize,
    total: f64,
    worst: f64,
}

impl Bucket {
    fn add(&mut self, ms: f64) {
        self.frames += 1;
        self.total += ms;
        self.worst = self.worst.max(ms);
    }
    fn mean(&self) -> f64 {
        if self.frames == 0 { 0.0 } else { self.total / self.frames as f64 }
    }
}

fn main() {
    // 2048x640 was the shipped size when this default was chosen; the world
    // has since grown to 8192x2560, so this default is now a quarter-linear
    // world (a sixteenth the area) rather than the size that ships. Left as
    // is rather than bumped to match -- `size=8192x2560` (see the module
    // doc's usage examples) is how to measure at the shipped size; this
    // stays the cheap default for a quick run.
    let mut size = (2048i32, 640i32);
    // Two full day/night cycles by default. **A shorter run is not a
    // measurement of this quantity**: the sky is a designed oscillator on a
    // 3600-frame period and the wind is another on a slower one, so any
    // window shorter than a cycle samples them at an arbitrary phase. Three
    // 600-frame windows on the same world, differing only in where they
    // started, measured 0.00, 4.98 and 7.04 ms/frame -- all three "the
    // settled field cost", none of them it.
    let mut frames = 2 * field::DAY_NIGHT_PERIOD_FRAMES as usize;
    let mut warm = 1500usize;
    let mut preset = String::new();
    let mut seed = 1u64;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "size" => {
                let (w, h) = v.split_once('x').expect("size=WxH");
                size = (w.parse().expect("width"), h.parse().expect("height"));
            }
            "frames" => frames = v.parse().expect("frames=N"),
            "warm" => warm = v.parse().expect("warm=N"),
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

    let dump_dir: Option<String> = std::env::var("FIELD_DUMP").ok();
    let hash_every: u64 =
        std::env::var("FIELD_HASH").ok().and_then(|v| v.parse().ok()).unwrap_or(0);

    let (w, h) = size;
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    worldgen::generate_only(&mut world, Spec::Generated { params: &params, seed });
    structural::compute_world_distances(&mut world);

    // Terrain reaches rest by frame 7 (measured by ablation); what this
    // warm-up is really for is the light and moisture channels, which start
    // at zero on a fresh world and have to fill it once.
    for _ in 0..warm {
        parallel::step(&mut world);
        field::step(&mut world);
    }

    println!("preset {name}, seed {seed}, {w}x{h}, {warm} warm-up frames, {frames} measured\n");

    // Four buckets, because there are exactly two things that wake the field
    // on a world where nothing is moving, and they are independent.
    let mut buckets = [[Bucket::default(); 2]; 2]; // [sky stepped][gusting]
    let mut sweep = Bucket::default();
    for _ in 0..frames {
        let frame = world.frame;
        let sky_stepped =
            field::sky_light_amplitude(frame) != field::sky_light_amplitude(frame.saturating_sub(1));
        let gusting = weather::at(seed, frame).wind.abs() >= GUST_THRESHOLD;

        let t = Instant::now();
        parallel::step(&mut world);
        sweep.add(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        field::step(&mut world);
        buckets[usize::from(sky_stepped)][usize::from(gusting)].add(t.elapsed().as_secs_f64() * 1000.0);
        if hash_every > 0 && world.frame.is_multiple_of(hash_every) {
            println!("  field hash @{:>6}: {:016x}", world.frame, field::field_hash(&world));
        }
        // `FIELD_DUMP=<dir>` writes the raw channels at the same cadence, for
        // the changes whose claim is bounded divergence rather than identity.
        if let Some(dir) = &dump_dir {
            if world.frame.is_multiple_of(hash_every.max(1)) {
                let v = field::field_channels(&world);
                let mut bytes = Vec::with_capacity(v.len() * 4);
                for f in &v {
                    bytes.extend_from_slice(&f.to_le_bytes());
                }
                std::fs::write(format!("{dir}/f{}.bin", world.frame), bytes).expect("dump");
            }
        }
    }

    // `FIELD_HASH=1` turns the run into a correctness check instead of a
    // timing one: the field's full state, printed at fixed frames, so a fast
    // path can be held to producing the *same field*. Compare two builds line
    // by line -- a divergence names the frame it started at.
    if std::env::var("FIELD_HASH").is_ok() {
        println!("  field hash @{:>6}: {:016x}", world.frame, field::field_hash(&world));
    }

    println!("{:>12} {:>9} {:>8} {:>9} {:>10} {:>10}", "sky", "wind", "frames", "share", "field mean", "field max");
    println!("{}", "-".repeat(64));
    let mut total = 0.0;
    for (sky, by_wind) in buckets.iter().enumerate() {
        for (wind, b) in by_wind.iter().enumerate() {
            if b.frames == 0 {
                continue;
            }
            total += b.total;
            println!(
                "{:>12} {:>9} {:>8} {:>8.1}% {:>9.2}ms {:>9.2}ms",
                if sky == 1 { "stepping" } else { "flat" },
                if wind == 1 { "gusting" } else { "calm" },
                b.frames,
                100.0 * b.frames as f64 / frames as f64,
                b.mean(),
                b.worst,
            );
        }
    }
    println!("{}", "-".repeat(64));
    let worst = buckets.iter().flatten().fold(0.0f64, |m, b| m.max(b.worst));
    println!(
        "{:>12} {:>9} {:>8} {:>8.1}% {:>9.2}ms {:>9.2}ms",
        "amortised",
        "",
        frames,
        100.0,
        total / frames as f64,
        worst,
    );
    println!(
        "\nsweep {:.3} ms mean / {:.2} ms worst, {} awake chunks at the end.\n\
         The amortised row is the honest per-frame figure; the bucket rows say\n\
         which of the two oscillators to spend effort on.",
        sweep.mean(),
        sweep.worst,
        world.active_chunk_count()
    );
}
