//! **Does moving a grow light move the light — one cell at a time?**
//!
//! The evolution lab's fixtures light the bed under them (`Material::beam`),
//! and the owner's ask is that adjusting growth by moving them be *fun*. That
//! is a claim about **granularity** before it is a claim about anything else:
//! light lives on the coarse field, one value per `FIELD_SCALE`-wide block, so
//! a mechanism that emits per block would only respond when a fixture crossed
//! a block boundary. Dragged, that reads as nothing happening for seven cells
//! and then a jump — the exact shape of a control a player calls broken.
//!
//! So this sweeps one fixture's column **one cell at a time across two
//! blocks** and reports what the bench under it actually does. Two modes,
//! because the light and the stand are different questions and only the
//! second one is the mechanic:
//!
//! - `mode=light` (default) — an empty box, one fixture, the field settled at
//!   every stop. Prints the bench's light **centroid**, which is the quantity
//!   that answers the question: a block-quantised emitter's centroid sits
//!   still for `FIELD_SCALE` cells and then steps, a continuous one tracks the
//!   fixture. Peak and total say whether the pool changed *shape* while it
//!   moved, which is the artifact an averaged emitter could introduce and a
//!   centroid alone would not show.
//! - `mode=stand` — the shipped bed, founders and all, grown to `frames` at
//!   each stop, with the census beside the light. `CLAUDE.md`: a light
//!   measurement without the stand census beside it can report a dead lab as
//!   a bright one.
//!
//! - `mode=cost` — the frame, timed, in **alternating paired runs of two arms
//!   off one binary**: `roof` is the lab as it was (sunlit, its fixtures
//!   replaced by stone, exactly the `lamps=0` control that measured
//!   byte-identical) and `lamps` is the lab as it ships. Alternating because
//!   `CLAUDE.md` records a 25-50% "regression" that was entirely the machine
//!   slowing between two runs an hour apart, and paired because the two arms
//!   must differ only by the switch. Census beside every timing, for the
//!   reason `labbox_cost`'s own `floor` arm exists: 0.007 ms and no seeds is
//!   a dead lab, not a fast one.
//!
//! ```text
//! cargo run --release --example lamp_probe
//! cargo run --release --example lamp_probe -- span=32 step=1
//! cargo run --release --example lamp_probe -- mode=stand span=64 step=8 frames=3600
//! cargo run --release --example lamp_probe -- mode=cost frames=3600 reps=3
//! ```
//!
//! **Read the centroid against `FIELD_SCALE`, which it prints.** The whole
//! finding is a comparison between the two, and at 16 the mechanic is twice
//! as coarse — see `Reports/lab-lamps-light-the-bed-2026-08-30.md` §4.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::field;
use pixel_physics::sim::frame;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// The light along the bench, as a profile and as the three numbers that say
/// whether it moved.
///
/// **Centroid over the pool, not over the bed.** A bed-wide centroid is
/// dominated by whatever the far walls are doing and barely moves when the
/// pool does; weighting only cells above a floor of the peak measures the
/// pool itself. The floor is a fraction rather than an absolute so it does
/// not have to be re-derived when the fixture's `beam` is retuned.
fn bench(world: &World, width: i32, y: i32) -> (f32, f32, f32) {
    let mut peak = 0.0f32;
    for x in 0..width {
        peak = peak.max(world.field_at(x, y).light);
    }
    let floor = peak * 0.25;
    let (mut mass, mut moment, mut total) = (0.0f64, 0.0f64, 0.0f64);
    for x in 0..width {
        let v = world.field_at(x, y).light;
        total += f64::from(v);
        if v >= floor {
            mass += f64::from(v);
            moment += f64::from(v) * f64::from(x);
        }
    }
    let centroid = if mass > 0.0 { (moment / mass) as f32 } else { f32::NAN };
    (centroid, peak, total as f32)
}

fn settle(world: &mut World, steps: u32) {
    for _ in 0..steps {
        world.step_fields();
    }
}

/// One timed run of the lab, with the census that says whether it was alive.
///
/// Returns `(mean ms, worst ms, cells, orgs, seeds, bench light)`.
///
/// **The light is in the tuple rather than left to a second harness**, because
/// the two arms differ in *where the light comes from* and a cost table with
/// no light in it cannot say whether the cheaper arm was simply darker.
fn timed_run(spec: &LabBox, sunlit: bool, frames: u64) -> (f64, f64, usize, usize, u32, f32) {
    let mut world = spec.build();
    if sunlit {
        // The lab as it was: daylight through four rows of stone, and the
        // fixtures inert. Two switches, and they are the same switch --
        // "where does the crop's light come from".
        world.set_sky_lighting(true);
        for cx in spec.lamps_in(&world) {
            spec.remove_lamp(&mut world, cx);
        }
    }
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    let (mut total, mut worst) = (0.0f64, 0.0f64);
    for _ in 0..frames {
        let t = std::time::Instant::now();
        frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        total += ms;
        worst = worst.max(ms);
    }
    let ids = world.live_organism_ids();
    let cells: usize = ids.iter().filter_map(|id| world.organism(*id)).map(|s| s.cells.len()).sum();
    let seeds: u32 = ids.iter().filter_map(|id| world.organism(*id)).map(|s| s.seeds_set).sum();
    let cols = spec.founder_columns();
    let lit = cols.iter().map(|&x| world.field_at(x, spec.ground_y - 2).light).sum::<f32>()
        / cols.len().max(1) as f32
        / field::MAX_LIGHT;
    (total / frames as f64, worst, cells, ids.len(), seeds, lit)
}

fn main() {
    let mode: String = arg("mode").unwrap_or_else(|| "light".to_string());
    let span: i32 = arg("span").unwrap_or(2 * field::FIELD_SCALE);
    let step: i32 = arg("step").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(3600);
    let width: i32 = arg("width").unwrap_or(512);
    // Echoes its own parameters, including the constant the finding is a
    // comparison against — `CLAUDE.md`'s megastudy gotcha, where eight
    // byte-identical logs were eight runs of a binary that never had the knob.
    println!(
        "lamp_probe: mode={mode} span={span} step={step} frames={frames} width={width} \
         FIELD_SCALE={} MAX_LIGHT={}",
        field::FIELD_SCALE,
        field::MAX_LIGHT
    );

    if mode == "cost" {
        let reps: u32 = arg("reps").unwrap_or(3);
        // **`founders=0 colonies=0` is the control that separates the two
        // costs, and it is the only reading that prices the *machinery*.**
        // With the bed planted, the lamp arm grows a much larger stand, and a
        // larger stand costs about 0.7 us per plant cell per tick
        // (`labbox_cost` §2a) -- so the planted delta is mostly the biosphere
        // the change bought, not the light model. An empty box has the same
        // fixtures, the same descent and nothing living to confound it.
        let spec = LabBox {
            width,
            founders: arg("founders").unwrap_or(LabBox::default().founders),
            colonies: arg("colonies").unwrap_or(LabBox::default().colonies),
            ..LabBox::default()
        };
        println!(
            "  {:>4}  {:>6}  {:>9}  {:>9}  {:>7} {:>5} {:>5}  {:>5}",
            "rep", "arm", "mean ms", "worst ms", "cells", "orgs", "seeds", "light"
        );
        let mut means: Vec<(f64, f64)> = Vec::new();
        for rep in 0..reps {
            // Alternating within the rep, not two blocks of runs: a machine
            // that slows partway through biases two blocks and cancels out of
            // a pair.
            let a = timed_run(&spec, true, frames);
            let b = timed_run(&spec, false, frames);
            for (name, r) in [("roof", a), ("lamps", b)] {
                println!(
                    "  {rep:>4}  {name:>6}  {:>9.3}  {:>9.3}  {:>7} {:>5} {:>5}  {:>5.3}",
                    r.0, r.1, r.2, r.3, r.4, r.5
                );
            }
            means.push((a.0, b.0));
        }
        let roof: f64 = means.iter().map(|m| m.0).sum::<f64>() / means.len() as f64;
        let lamps: f64 = means.iter().map(|m| m.1).sum::<f64>() / means.len() as f64;
        let wins = means.iter().filter(|m| m.1 < m.0).count();
        println!(
            "  mean of {reps}: roof {roof:.3} ms, lamps {lamps:.3} ms -- {:+.3} ms ({:+.1}%), \
             lamps cheaper in {wins} of {reps}",
            lamps - roof,
            (lamps - roof) / roof * 100.0
        );
        return;
    }

    // One fixture in an otherwise empty box for the light sweep: a second
    // fixture's pool overlaps the first's and the centroid then measures the
    // pair, which moves half as far as the lamp does and reads as damping.
    let spec = if mode == "stand" {
        LabBox { width, ..LabBox::default() }
    } else {
        LabBox { width, founders: 0, colonies: 0, lamp_spacing: width * 2, ..LabBox::default() }
    };
    let bench_y = spec.ground_y - 2;

    let probe = spec.build();
    let home = *spec.lamps_in(&probe).first().expect("the box has a fixture");
    println!("  fixture at {home}, bench row {bench_y}, lamps at {:?}", spec.lamps_in(&probe));
    println!(
        "  {:>6}  {:>9}  {:>9}  {:>7}  {:>9}{}",
        "offset", "centroid", "d(centroid)", "peak", "total", if mode == "stand" { "   cells  orgs seeds" } else { "" }
    );

    let mut previous: Option<f32> = None;
    let mut offset = 0;
    while offset <= span {
        // Rebuilt rather than dragged along, so every stop is one move from
        // the same state — a chain of moves would accumulate any asymmetry in
        // `paint_lamp` and report it as drift in the mechanic.
        let mut world = spec.build();
        if offset != 0 && !spec.move_lamp(&mut world, home, home + offset) {
            println!("  {offset:>6}  REFUSED");
            offset += step;
            continue;
        }
        let mut particles = ParticleSystem::new();
        let mut blasts = Blasts::new();
        let tuning = player::Tuning::default();
        let census = if mode == "stand" {
            for _ in 0..frames {
                frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
            }
            let ids = world.live_organism_ids();
            let cells: usize = ids.iter().filter_map(|id| world.organism(*id)).map(|s| s.cells.len()).sum();
            let seeds: u32 = ids.iter().filter_map(|id| world.organism(*id)).map(|s| s.seeds_set).sum();
            format!("  {cells:>7} {:>5} {seeds:>5}", ids.len())
        } else {
            settle(&mut world, 240);
            String::new()
        };
        let (centroid, peak, total) = bench(&world, spec.width, bench_y);
        let delta = previous.map_or(f32::NAN, |p| centroid - p);
        previous = Some(centroid);
        println!(
            "  {offset:>6}  {centroid:>9.3}  {delta:>11.3}  {:>7.3}  {total:>9.1}{census}",
            peak / field::MAX_LIGHT
        );
        offset += step;
    }
}
