//! Guards for `evaporation.rs`, kept in their own file because the scenes
//! they need are long and shared.
//!
//! **What these are written to catch is the mechanic *stopping*, not the
//! mechanic firing.** Four bugs in a row in this area passed guards that only
//! ever asked whether something happened once (`Reports/weather-handoff.md`,
//! "the single lesson worth carrying forward"), and the reverted first
//! version of evaporation was one of them: it ran, it was measurable, and it
//! silently stopped the moment the water settled — which is the only state it
//! exists for.
//!
//! Three conventions throughout, each of which has already cost time here:
//!
//! * **Volume, never cell count.** A `Liquid` cell holds continuous fill, so
//!   six full cells spreading into a thin film across thirty-eight is
//!   constant volume and a 6 -> 228 *increase* by cell count. The handoff
//!   records reading exactly that and taking it for evaporation
//!   manufacturing water.
//! * **Matched depth in every paired scene.** Only surface cells evaporate,
//!   so a shallow body has a larger fraction of itself at the surface than a
//!   deep one, and a puddle-vs-lake test with mismatched depths would pass on
//!   geometry alone — a size measurement smuggled in through the back door,
//!   which is the one thing this design is built not to contain. Both bodies
//!   are `DEPTH` rows deep in every scene below, so surface-to-volume is
//!   identical and the only thing left that can differ is how much shelter
//!   each body makes for itself.
//! * **One body per world.** The first version of the paired scene put the
//!   puddle and the lake in the same world 200 cells apart, and the lake
//!   humidified the puddle: the air over the puddle read 1.82 against 1.45
//!   for the same puddle alone, a third of the way to the lake's own 2.31,
//!   and it slowed the puddle by the same margin. Weather is a pure function
//!   of `(seed, frame)`, so two worlds built from the same seed and stepped
//!   the same number of frames get *identical* weather — the pairing survives
//!   the separation, and the cross-talk does not.

use super::*;
use crate::sim::chunk::Rect;
use crate::sim::field;
use crate::sim::material;
use crate::sim::parallel;
use crate::sim::scheduler;
use crate::sim::update;

const FLOOR_Y: i32 = 160;
const DEPTH: i32 = 4;
const BASIN_X: i32 = 300;
const PUDDLE_W: i32 = 6;
const LAKE_W: i32 = 240;
const SEED: u64 = 12345;

/// A world with a stone floor, a stone lid, and one walled basin `width`
/// cells across holding water `DEPTH` rows deep.
///
/// **The lid on the topmost row is what makes the scene rain-proof**, and it
/// is deterministic rather than a lucky seed: `weather::surface_under_sky`
/// walks from the top, so every column answers `min_y`, and both the
/// water-cell and the snow-cell spawns require `surface_y > bounds.min_y`.
/// Rain still soaks the lid — stone, `water_capacity == 0`, so that stops on
/// the first cell — and still writes humidity 160 rows above the water, where
/// the moisture channel's own decay leaves nothing that reaches anything
/// measured here. Needed because both drivers run `weather::step`, so any
/// test that steps the world at all is subject to rain unless it says
/// otherwise, and a run long enough to watch a puddle dry is far longer than
/// any dry window a seed offers.
///
/// **Walled** rather than open for the reason `decay.rs`'s own damp-moss
/// scene is walled: an unwalled puddle spreads away across the floor before
/// the field ever registers it as a source, and the test then measures
/// spreading rather than the thing it is named for.
fn scene(width: i32) -> World {
    let mut w = World::new(Rect::new(0, 0, 639, 199));
    w.seed = SEED;
    let b = w.bounds().unwrap();
    for x in b.min_x..=b.max_x {
        w.set(x, b.min_y, Cell::new(material::STONE, 0));
        for y in FLOOR_Y..=b.max_y {
            w.set(x, y, Cell::new(material::STONE, 0));
        }
    }
    for y in (FLOOR_Y - DEPTH)..FLOOR_Y {
        w.set(BASIN_X - 1, y, Cell::new(material::STONE, 0));
        w.set(BASIN_X + width, y, Cell::new(material::STONE, 0));
        for x in BASIN_X..(BASIN_X + width) {
            w.set(x, y, Cell::new(material::WATER, 0));
        }
    }
    w
}

/// Summed `liquid_fill` over the basin — the *volume* of water in it, not how
/// many cells hold some. See the module doc.
fn volume(w: &World, width: i32) -> u64 {
    let b = w.bounds().unwrap();
    let mut sum = 0u64;
    for y in b.min_y..=b.max_y {
        for x in (BASIN_X - 1)..=(BASIN_X + width) {
            let c = w.get(x, y);
            if w.materials.kind(c.material) == MaterialKind::Liquid {
                sum += update::liquid_fill(c) as u64;
            }
        }
    }
    sum
}

/// A full frame: CA sweep, then active sites, then the field — the same order
/// and the same phases `App::update` runs, and the parallel driver
/// specifically, since that is the one the app uses.
fn run(w: &mut World, frames: usize) {
    for _ in 0..frames {
        parallel::step(w);
        w.step_active_sites();
        w.step_fields();
    }
}

/// A frame with **no CA sweep at all** — only the scheduler and the field, so
/// the world's cells are touched by nothing except the active-site list
/// itself.
fn run_without_the_sweep(w: &mut World, frames: usize) {
    for _ in 0..frames {
        w.begin_step();
        field::step(w);
        scheduler::step(w);
        w.end_step();
    }
}

fn pending_evaporation_sites(w: &World) -> usize {
    w.active_sites_for_test().iter().filter(|s| matches!(s.kind, ActiveKind::Evaporate { .. })).count()
}

/// Fraction of the basin's water that is gone after `frames` frames.
fn fraction_lost(width: i32, frames: usize) -> f32 {
    let mut w = scene(width);
    let before = volume(&w, width);
    run(&mut w, frames);
    1.0 - volume(&w, width) as f32 / before as f32
}

#[test]
fn a_puddle_dries_up_and_a_lake_does_not() {
    // The headline paired comparison. Same seed, so the same weather; same
    // frame count; same depth, so the same surface-to-volume ratio. The two
    // bodies differ only in width, and nothing in the mechanic reads a width.
    let puddle = fraction_lost(PUDDLE_W, 11_000);
    let lake = fraction_lost(LAKE_W, 11_000);
    println!("11000 frames: puddle lost {:.1}%, lake lost {:.1}%", puddle * 100.0, lake * 100.0);

    assert!(puddle > 0.80, "the puddle should have all but gone, lost {:.1}%", puddle * 100.0);
    assert!(lake < 0.05, "the lake should be essentially untouched, lost {:.1}%", lake * 100.0);
}

#[test]
fn a_lake_survives_the_gale_that_dries_the_puddle() {
    // **The interaction that nearly sank this feature, kept as a guard
    // because nothing else here can see it.**
    //
    // `weather::gust` fires every 26 frames for as long as the wind channel
    // stays above its threshold, which on seed 12345 is from frame 11460
    // onward for the rest of that epoch. Sustained gusts advect the humid
    // layer off the top of a lake — traced at 2.31 -> 0.23 within ten frames
    // of the crossing, and 2.31 -> 0.42 with the lake down to 39% of its
    // volume on the full 2048x640 world. Under humidity alone the mechanic
    // could not tell a lake from a puddle in a gale, because a gale mixes the
    // atmosphere and the air over both is equally dry, so every lake in the
    // world went.
    //
    // This runs *through* that crossing and well past it. It fails outright
    // if `evaporation::shelter` is removed, or if it is ever made a function
    // of the advected humidity channel.
    let lake = fraction_lost(LAKE_W, 18_000);
    println!("18000 frames (spanning the gale from 11460): lake lost {:.1}%", lake * 100.0);
    assert!(lake < 0.20, "the gale took the lake apart: lost {:.1}%", lake * 100.0);
}

#[test]
fn evaporation_keeps_going_after_the_ca_sweep_stops_entirely() {
    // **The guard the reverted version could not pass, and the reason this
    // file exists.** That version evaporated from the sweep's own `Liquid`
    // arm, so it worked exactly as long as the water was still moving and
    // then stopped without any sign that it had — which read as a working
    // mechanic in every test written for it, and produced a lake losing 7%
    // against a puddle's 1.7%, because a lake stays awake settling for
    // longer.
    //
    // So: settle everything with the sweep running, then take the sweep away
    // completely and check the puddle carries on drying anyway. Nothing but
    // the active-site list is touching the world in the second window.
    let mut w = scene(PUDDLE_W);
    run(&mut w, 1_500);
    let settled = volume(&w, PUDDLE_W);
    run_without_the_sweep(&mut w, 12_000);
    let lost = 1.0 - volume(&w, PUDDLE_W) as f32 / settled as f32;
    println!("sweepless: puddle lost {:.1}%", lost * 100.0);
    assert!(
        lost > 0.80,
        "evaporation stopped when the sweep did — the puddle lost only {:.1}% across twelve thousand sweepless frames",
        lost * 100.0
    );
}

#[test]
fn a_sealed_pool_stops_checking_itself_but_an_open_lake_does_not() {
    // The two halves of "and then it stops", which are different questions
    // and go wrong in opposite directions.
    //
    // A pool with rock laid straight onto it is *structurally* finished: it
    // must retire off the schedule and stay retired, or every buried body of
    // water in a world pays a check forever.
    //
    // A lake surface is not. It evaporates at exactly zero and must still
    // reschedule, because how wet the air is and how sheltered a surface is
    // are *values* that change — with the weather, and with anyone digging a
    // channel out of the lake — not structures that do not. A version that
    // treated a zero rate as staleness would look identical in every other
    // test here and would leave a lake permanently unable to resume.
    let mut sealed = scene(PUDDLE_W);
    for x in BASIN_X..(BASIN_X + PUDDLE_W) {
        sealed.set(x, FLOOR_Y - DEPTH - 1, Cell::new(material::STONE, 0));
        // Scheduled by hand as well as by the sweep: a site can legitimately
        // exist for a covered cell, because the cover can land after the site
        // was made, and that is exactly the case `stale_ticks` bounds.
        sealed.schedule_active_site(ActiveSite {
            x,
            y: FLOOR_Y - DEPTH,
            kind: ActiveKind::Evaporate { stale_ticks: 0 },
            next_frame: sealed.frame + CHECK_INTERVAL,
        });
    }
    let before = volume(&sealed, PUDDLE_W);
    run(&mut sealed, 4_000);
    assert_eq!(volume(&sealed, PUDDLE_W), before, "a sealed pool should not have lost a drop");
    assert_eq!(
        pending_evaporation_sites(&sealed),
        0,
        "a sealed pool should have retired every one of its evaporation sites"
    );

    let mut lake = scene(LAKE_W);
    run(&mut lake, 4_000);
    let sites = pending_evaporation_sites(&lake);
    assert!(
        sites > LAKE_W as usize / 2,
        "a sheltered lake surface must stay on the schedule so it can resume when conditions change — only {sites} of {LAKE_W} sites left"
    );
}

#[test]
fn the_serial_driver_schedules_evaporation_too() {
    // `CLAUDE.md`: two drivers, and the app runs the parallel one — so test
    // both. The hook lives in `update.rs`'s shared dispatch, which both
    // drivers run, but the route a scheduled site takes back to the world
    // differs (`ChunkView`'s `pending_active_sites` replay for the parallel
    // sweep, a direct call for the serial one) and that is worth a guard.
    let mut w = scene(PUDDLE_W);
    let before = volume(&w, PUDDLE_W);
    for _ in 0..8_000 {
        update::step(&mut w);
        w.step_active_sites();
        w.step_fields();
    }
    let lost = 1.0 - volume(&w, PUDDLE_W) as f32 / before as f32;
    println!("serial driver: puddle lost {:.1}%", lost * 100.0);
    assert!(lost > 0.50, "the serial driver never got the puddle evaporating: lost {:.1}%", lost * 100.0);
}

#[test]
fn a_settled_body_is_scheduled_once_per_surface_cell_however_long_it_took_to_settle() {
    // The dedup `World::pending_evaporation` provides, checked as the
    // property it exists for rather than as an implementation detail: the
    // number of pending sites tracks the number of *surface cells*, not how
    // many frames the body spent awake. Without it, a body's evaporation rate
    // would scale with its settling time — and bigger bodies settle for
    // longer, which is exactly how the reverted version ended up drying a
    // lake faster than a puddle.
    let mut w = scene(PUDDLE_W);
    run(&mut w, 400);
    let sites = pending_evaporation_sites(&w);
    assert!(
        (1..=PUDDLE_W as usize).contains(&sites),
        "expected at most one site per surface cell ({PUDDLE_W}), found {sites}"
    );
}

#[test]
#[ignore = "probe, not a guard"]
fn probe_drying_curve() {
    // What the constants actually produce, and the first thing to re-run
    // after touching any of them. Prints both bodies side by side through the
    // gale that starts at frame 11460 on this seed.
    let mut p = scene(PUDDLE_W);
    let mut l = scene(LAKE_W);
    let (p0, l0) = (volume(&p, PUDDLE_W), volume(&l, LAKE_W));
    println!("frame   puddle%  h_p    lake%   h_l    wind");
    for _ in 0..20 {
        run(&mut p, 1_000);
        run(&mut l, 1_000);
        let ys = FLOOR_Y - DEPTH - field::FIELD_SCALE;
        println!(
            "{:6}  {:6.1}  {:5.2}  {:6.1}  {:5.2}  {:5.2}",
            p.frame,
            100.0 * (1.0 - volume(&p, PUDDLE_W) as f32 / p0 as f32),
            p.field_at(BASIN_X + PUDDLE_W / 2, ys).moisture,
            100.0 * (1.0 - volume(&l, LAKE_W) as f32 / l0 as f32),
            l.field_at(BASIN_X + LAKE_W / 2, ys).moisture,
            crate::sim::weather::at(SEED, p.frame).wind,
        );
    }
}

#[test]
#[ignore = "probe, not a guard"]
fn probe_humidity_against_width() {
    // What `HUMID_STOP` is set from. Re-run after any change to the moisture
    // channel's constants or to `field::step_diffusion`.
    println!("width  block_above  shelter_would_be");
    for &width in &[2i32, 4, 8, 16, 32, 64, 128, 240] {
        let mut w = scene(width);
        run(&mut w, 1_500);
        let ys = FLOOR_Y - DEPTH;
        println!("{width:5}  {:11.3}", w.field_at(BASIN_X + width / 2, ys - field::FIELD_SCALE).moisture);
    }
}

#[test]
#[ignore = "probe, not a guard"]
fn probe_awake_chunks_and_frame_cost() {
    // What evaporation costs a *settled* world, which is the state the
    // dirty-rect skip and per-tile field sleeping exist for and therefore
    // the only state worth measuring the cost in. A wide body losing water
    // at its shoreline re-levels itself, and re-levelling wakes chunks.
    for &(label, width) in &[("puddle", PUDDLE_W), ("lake", LAKE_W)] {
        let mut w = scene(width);
        run(&mut w, 2_000);
        let t = std::time::Instant::now();
        let mut awake_total = 0usize;
        let mut worst = 0.0f64;
        for _ in 0..2_000 {
            let f = std::time::Instant::now();
            run(&mut w, 1);
            worst = worst.max(f.elapsed().as_secs_f64() * 1000.0);
            awake_total += w.active_chunk_count();
        }
        println!(
            "{label:7} width {width:3}: mean awake chunks {:.2}, worst frame {worst:.3} ms, mean {:.4} ms",
            awake_total as f64 / 2000.0,
            t.elapsed().as_secs_f64() * 1000.0 / 2000.0
        );
    }
}

#[test]
#[ignore = "probe, not a guard"]
fn probe_a_puddle_poured_on_an_open_shelf() {
    // `filmstrip scene=tree`'s geometry: a stone shelf spanning the whole
    // world and a blob of water dropped on it, with nothing to pool against.
    // The question the contact sheet could not answer by eye is whether the
    // water dried or merely spread into a film too thin to see.
    let mut w = World::new(Rect::new(0, 0, 511, 319));
    w.seed = SEED;
    for x in 0..512 {
        for y in 40..46 {
            w.set(x, y, Cell::new(material::STONE, 0));
        }
    }
    w.paint_circle(150, 36, 7, material::WATER);
    let total = |w: &World| -> (u64, usize) {
        let mut fill = 0u64;
        let mut cells = 0usize;
        for y in 0..40 {
            for x in 0..512 {
                let c = w.get(x, y);
                if w.materials.kind(c.material) == MaterialKind::Liquid {
                    fill += update::liquid_fill(c) as u64;
                    cells += 1;
                }
            }
        }
        (fill, cells)
    };
    println!("frame   volume   cells  humidity");
    for i in 0..8 {
        run(&mut w, if i == 0 { 100 } else { 2_000 });
        let (fill, cells) = total(&w);
        println!("{:6}  {fill:7}  {cells:5}  {:.3}", w.frame, w.field_at(150, 39 - field::FIELD_SCALE).moisture);
    }
}

#[test]
#[ignore = "probe, not a guard"]
fn probe_stress_scene_split() {
    // `ascii.rs`'s "full screen + field step every frame (parallel)" scene,
    // with the two phases timed apart, to say which one the regression is in.
    let mut w = World::new(Rect::new(0, 0, 511, 319));
    for x in 0..512 {
        w.set(x, 319, Cell::new(material::STONE, 0));
    }
    for y in 20..160 {
        for x in 0..512 {
            let m = if y < 90 { material::SAND } else { material::WATER };
            w.set(x, y, Cell::new(m, 0));
        }
    }
    w.add_pressure_impulse(256, 100, 20, 150.0);
    let (mut worst_ca, mut worst_field, mut worst_total) = (0.0f64, 0.0f64, 0.0f64);
    let (mut sum_ca, mut sum_field) = (0.0f64, 0.0f64);
    for _ in 0..400 {
        let t0 = std::time::Instant::now();
        parallel::step(&mut w);
        let ca = t0.elapsed().as_secs_f64() * 1000.0;
        let t1 = std::time::Instant::now();
        w.step_fields();
        let fl = t1.elapsed().as_secs_f64() * 1000.0;
        worst_ca = worst_ca.max(ca);
        worst_field = worst_field.max(fl);
        worst_total = worst_total.max(ca + fl);
        sum_ca += ca;
        sum_field += fl;
    }
    println!(
        "worst CA {worst_ca:.2} ms, worst field {worst_field:.2} ms, worst total {worst_total:.2} ms;          mean CA {:.3}, mean field {:.3}; sites {}",
        sum_ca / 400.0,
        sum_field / 400.0,
        w.active_site_count()
    );
}
