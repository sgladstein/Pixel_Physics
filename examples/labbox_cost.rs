//! **What a sealed lab box costs per frame, and which of its world systems
//! is paying for it** — the feasibility number for the evolution-lab concept
//! (`Reports/evolution-lab-feasibility-2026-08-30.md`).
//!
//! `scale_probe phases=1` answers "what does a *generated world* of size N
//! cost", and its answer at every size measured so far is *the field*, at
//! 69-86% of the frame. That is the number this harness exists to
//! interrogate, because the lab-box concept removes the two things that
//! drive the field — **an open sky and outdoor weather** — and no measurement
//! in this repo tells you what is left when you do.
//!
//! Four arms over one bed, each a single switch off the one before it, so a
//! difference is attributable:
//!
//! | arm | what it changes |
//! |---|---|
//! | `live` | nothing — the shipped outdoor world in a box |
//! | `calm` | `set_weather_pin(Clear)` — no wind, no rain |
//! | `lab` | `calm` plus `set_sky_hold(noon)` — a grow light, not a sun |
//! | `floor` | `lab` with the field step not called at all — the control that
//!   says how much of the frame is *not* the field, and therefore what any
//!   field work could ever buy |
//!
//! **`floor` is a control, not a proposal.** Skipping the field entirely
//! changes behaviour (light stops being delivered), so its plant census is
//! expected to differ and is printed for exactly that reason — see the
//! stand columns.
//!
//! **Read the counters beside the timings, never the timings alone.** A
//! frame that got cheaper because the field stopped being asked anything
//! and one that got cheaper because it converged are indistinguishable in a
//! timing (`CLAUDE.md`: *a cost that vanishes may be work that vanished*).
//! `solved/frame` says whether the field still ran, `awake` says whether the
//! CA still had anything to sweep, and `cells`/`orgs`/`seeds` say whether
//! the stand this is all supposedly for still grew. An arm that is fast and
//! has a dead stand has not made the game cheaper, it has made a different
//! game.
//!
//! ```text
//! cargo run --release --example labbox_cost
//! cargo run --release --example labbox_cost -- frames=6000 width=1024 soil=120 trees=16
//! cargo run --release --example labbox_cost -- arms=live,lab species=herb
//! ```

mod common;

use common::PlantScene;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::particle::{self, ParticleSystem};
use pixel_physics::sim::weather::Pin;
use pixel_physics::sim::{parallel, rigid};
use std::time::Instant;

const PHASES: [&str; 6] =
    ["ca_sweep", "chunk_bodies", "active_sites", "particles", "field", "pheromones"];

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses"))
    })
}

struct Arm {
    name: &'static str,
    calm: bool,
    sky_hold: bool,
    field: bool,
}

const ARMS: [Arm; 4] = [
    Arm { name: "live", calm: false, sky_hold: false, field: true },
    Arm { name: "calm", calm: true, sky_hold: false, field: true },
    Arm { name: "lab", calm: true, sky_hold: true, field: true },
    Arm { name: "floor", calm: true, sky_hold: true, field: false },
];

fn main() {
    let frames: u64 = arg("frames").unwrap_or(6_000);
    let warm: u64 = arg("warm").unwrap_or(300);
    let width: i32 = arg("width").unwrap_or(1024);
    let soil: i32 = arg("soil").unwrap_or(120);
    let height: i32 = arg("height").unwrap_or(320);
    let trees: usize = arg("trees").unwrap_or(16);
    let species: String = arg("species").unwrap_or_else(|| "herb".to_string());
    let worldseed: u64 = arg("worldseed").unwrap_or(1);
    let want: String = arg("arms").unwrap_or_else(|| "live,calm,lab,floor".to_string());
    // **`daymin=N` lengthens the day without removing it** -- the shipped
    // `Clock::day_minutes`, which divides the sky clock so the sun's
    // amplitude steps N times less often. This is *not* the `lab` arm's
    // grow light: the cycle still runs, still reaches the night floor, and
    // still looks like a day. It is the "fake the sun a little" lever for a
    // world that must keep its day/night, and the reason it is here is that
    // `field.rs`'s own comment claims a longer day is "proportionally
    // cheaper" and nothing had measured it.
    let daymin: u32 = arg("daymin").unwrap_or(1);

    // Echoes its own parameters -- `instruments.md`'s standing rule, and the
    // reason a 3.5-hour study once produced eight identical logs.
    println!(
        "labbox_cost: frames={frames} warm={warm} width={width}x{height} soil={soil} \
         trees={trees} species={species} worldseed={worldseed} daymin={daymin} arms={want}"
    );

    println!(
        "\n{:>6}  {:>9}  {:>9}  {:>9}  {:>8}  {:>8}  {:>7}  {:>6}  {:>6}  {:>6}",
        "arm", "mean", "p50", "worst", "solved/f", "awake/f", "x live", "cells", "orgs", "seeds"
    );

    let mut baseline: Option<f64> = None;
    let mut rows: Vec<(&str, Vec<f64>, [f64; PHASES.len()])> = Vec::new();

    for arm in ARMS.iter().filter(|a| want.split(',').any(|w| w == a.name)) {
        let mut scene = PlantScene { species: species.clone(), ..PlantScene::default() };
        scene.width = width;
        scene.height = height;
        scene.soil_depth = soil;
        scene.trees = trees;
        scene.seed = Some(worldseed);
        let mut world = scene.build();

        if daymin > 1 {
            let f = world.frame;
            world.clock.set_rates(f, |c| c.day_minutes = daymin);
        }
        if arm.calm {
            world.set_weather_pin(Pin::Clear);
        }
        if arm.sky_hold {
            // Noon, held. `sky_hold` takes the frame the sky is pinned to;
            // a quarter of a day into the cycle is the sun at its highest,
            // which is what a grow light is: constant, and bright.
            // **Which frame is noon is measured, not assumed.** The hump
            // is a cosine over half the period and the phase is
            // `sun_elevation`'s, not this file's, so pick the frame with
            // the highest amplitude rather than guessing a quarter or a
            // half. Guessing it wrong pins a grow light at midnight, which
            // reads in the census as "constant light shrinks the stand".
            let noon = (0..pixel_physics::sim::field::DAY_NIGHT_PERIOD_FRAMES)
                .max_by(|a, b| {
                    pixel_physics::sim::field::sky_light_amplitude(*a)
                        .total_cmp(&pixel_physics::sim::field::sky_light_amplitude(*b))
                })
                .expect("the day has frames in it");
            if arm.name == "lab" {
                println!(
                    "  grow light held at frame {noon} of {}, amplitude {:.3} \
                     (the cycle this replaces runs {:.3}..{:.3})",
                    pixel_physics::sim::field::DAY_NIGHT_PERIOD_FRAMES,
                    pixel_physics::sim::field::sky_light_amplitude(noon),
                    (0..pixel_physics::sim::field::DAY_NIGHT_PERIOD_FRAMES)
                        .map(pixel_physics::sim::field::sky_light_amplitude)
                        .fold(f32::INFINITY, f32::min),
                    pixel_physics::sim::field::sky_light_amplitude(noon),
                );
            }
            world.set_sky_hold(Some(noon));
        }

        let mut particles = ParticleSystem::default();
        let mut blasts = Blasts::default();
        let mut totals = Vec::with_capacity(frames as usize);
        let mut phase_sums = [0.0f64; PHASES.len()];
        let mut solved = 0u64;
        let mut awake = 0u64;
        let mut counted = 0u64;

        for f in 0..(warm + frames) {
            let record = f >= warm;
            if record {
                world.field_stats.tiles_solved = 0;
            }
            let mut marks = [Instant::now(); PHASES.len() + 1];
            parallel::step(&mut world);
            marks[1] = Instant::now();
            rigid::step_chunk_bodies(&mut world);
            marks[2] = Instant::now();
            world.step_active_sites();
            marks[3] = Instant::now();
            blasts.step(&mut world, &mut particles);
            particle::throw_splashes(&mut world, &mut particles);
            particles.step(&mut world);
            marks[4] = Instant::now();
            if arm.field {
                world.step_fields();
            }
            marks[5] = Instant::now();
            world.step_pheromones();
            marks[6] = Instant::now();

            if record {
                let mut total = 0.0;
                for i in 0..PHASES.len() {
                    let ms = marks[i + 1].duration_since(marks[i]).as_secs_f64() * 1000.0;
                    phase_sums[i] += ms;
                    total += ms;
                }
                totals.push(total);
                solved += world.field_stats.tiles_solved;
                awake += world.active_chunk_count() as u64;
                counted += 1;
            }
        }

        totals.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
        let mean = totals.iter().sum::<f64>() / totals.len() as f64;
        // The stand census: is this arm still growing the thing the game is
        // about, or has it merely stopped doing work?
        let orgs = world.live_organism_count();
        let ids = world.live_organism_ids();
        let cells: usize = ids.iter().filter_map(|id| world.organism(*id)).map(|s| s.cells.len()).sum();
        let seeds: u32 = ids.iter().filter_map(|id| world.organism(*id)).map(|s| s.seeds_set).sum();
        if baseline.is_none() {
            baseline = Some(mean);
        }
        println!(
            "{:>6}  {:>7.3}ms  {:>7.3}ms  {:>7.3}ms  {:>8.1}  {:>8.1}  {:>6.1}x  {:>6}  {:>6}  {:>6}",
            arm.name,
            mean,
            totals[totals.len() / 2],
            totals[totals.len() - 1],
            solved as f64 / counted as f64,
            awake as f64 / counted as f64,
            // Speed-up over 60 Hz real time: how many simulated seconds
            // pass per wall-clock second.
            1000.0 / mean / 60.0,
            cells,
            orgs,
            seeds,
        );
        rows.push((arm.name, totals, phase_sums.map(|s| s / counted as f64)));
    }

    println!("\nper-phase mean, ms");
    print!("{:>6}", "arm");
    for p in PHASES {
        print!("  {p:>13}");
    }
    println!();
    for (name, _, phases) in &rows {
        print!("{name:>6}");
        for ms in phases {
            print!("  {ms:>13.3}");
        }
        println!();
    }
}
