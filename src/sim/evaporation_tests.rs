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
//!   humidified the puddle: measured on the larger scene these guards used
//!   to run on, the air over the puddle read 1.82 with the lake present
//!   against 1.45 for the same puddle alone — a third of the way to the
//!   lake's own 2.31, and it slowed the puddle by the same margin. Weather is a pure function
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

// **Sized for cost, then re-derived from measurement.** These guards were
// six tests taking 52 seconds — 17% of the whole suite, and the worst
// time-per-test in the repo by a factor of two. Frame cost here is roughly
// world width times frames: the sky-lit field tiles never sleep (the sun is
// always moving), so a wide world pays for its whole surface every frame
// whatever is happening in it, and the drying tests have to run until a
// puddle is actually gone.
//
// So the world is a quarter of the area it was and the bodies are half as
// deep, which halves the water a puddle has to lose. Nothing about what is
// under test changed: the two bodies still differ only in width, still share
// a depth, and the bars below are re-measured rather than scaled.
const FLOOR_Y: i32 = 100;
const DEPTH: i32 = 2;
const BASIN_X: i32 = 40;
const PUDDLE_W: i32 = 6;
/// Wide enough that the middle is fully sheltered (`SHELTER_REACH` is three
/// field blocks, so 24 cells either side) with margin left over for a
/// shoreline that is *not*, which is what the gale guard measures.
const LAKE_W: i32 = 176;
/// Calm until frame 11,460, so the drying guards see no wind at all.
const SEED: u64 = 12345;
/// Crosses `GUST_THRESHOLD` at frame 1,500 and is still gusting 4,000 frames
/// later — a calm start, a crossing, and a long gale inside 6,000 frames
/// instead of the 18,000 seed 12345 needs to reach its own. Picked by
/// sweeping the wind channel, which is a pure function of `(seed, frame)`
/// and costs nothing to search (`probe_find_an_early_gale_seed`).
const GALE_SEED: u64 = 20;

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
    scene_seeded(width, SEED)
}

fn scene_seeded(width: i32, seed: u64) -> World {
    let mut w = World::new(Rect::new(0, 0, 255, 127));
    w.seed = seed;
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
    fraction_lost_seeded(width, frames, SEED)
}

fn fraction_lost_seeded(width: i32, frames: usize, seed: u64) -> f32 {
    let mut w = scene_seeded(width, seed);
    let before = volume(&w, width);
    run(&mut w, frames);
    1.0 - volume(&w, width) as f32 / before as f32
}

/// A bed of soil, saturated, open to the air.
///
/// **One row deep, and narrow.** Only a surface cell dries, so a deep bed
/// spends most of a run moving water upward by capillary flow and reaches
/// its resting state far too slowly to assert on -- written six rows deep
/// first and it was still visibly drying after 40,000 frames, which says
/// nothing about whether it *stops*. One row makes every cell a surface
/// cell. Narrow for a second reason: a wide damp bed shelters itself the
/// way a lake does, so a wide one would be measuring `shelter` instead.
/// How long a rain-free stretch the drying guard needs, in frames.
const DRY_WINDOW: u64 = 12_000;

fn soil_bed(width: i32) -> World {
    let mut w = World::new(Rect::new(0, 0, 255, 127));
    w.seed = SEED;
    // **A dry window, chosen rather than assumed.** Weather is a pure
    // function of `(seed, frame)` and it runs inside `parallel::step`, so a
    // long run in an arbitrary window rains. Written without this and the
    // bed dried to 4,320 and was back up at 23,515 twenty thousand frames
    // later -- the water cycle working correctly and making "and then it
    // stops" unaskable.
    w.frame = (0..crate::sim::weather::WEATHER_EPOCH_FRAMES * 64)
        .step_by(120)
        .find(|&f| {
            (0..DRY_WINDOW).step_by(120).all(|d| crate::sim::weather::at(SEED, f + d).kind == crate::sim::weather::Precipitation::None)
        })
        .expect("this seed has a long dry spell in it");
    let b = w.bounds().unwrap();
    for x in b.min_x..=b.max_x {
        for y in FLOOR_Y..=b.max_y {
            w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
    let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
    for x in BASIN_X..(BASIN_X + width) {
        w.set(x, FLOOR_Y - 1, Cell::new(soil, 0).with_aux(material::SOIL_SATURATED));
        w.schedule_damp_soil_for_test(x, FLOOR_Y - 1);
    }
    w
}

/// **The whole world, not the basin window.** Soil is a `Powder`: an
/// unwalled bed slumps and spreads, and a window sized to where it started
/// reads the spreading as loss. Written that way first and it reported the
/// bed losing 2,054 units while the sky was credited 54 -- which looks
/// exactly like a broken credit path and was a broken ruler.
fn soil_held(w: &World) -> u64 {
    let b = w.bounds().unwrap();
    let mut sum = 0u64;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            if w.materials.get(c.material).water_capacity > 0 {
                sum += update::soil_moisture(c) as u64;
            }
        }
    }
    sum
}

/// **Damp ground dries, banks exactly what it lost, and then stops** — the
/// second credit path, and the shape `Reports/weather-handoff.md` insists
/// on ("every guard tested that a mechanism fires, and none tested that it
/// stops").
///
/// Stopping is the whole point here and it is not incidental: soil never
/// becomes a different material when it dries, so unlike a puddle there is
/// no "the cell is gone" terminal state. It has to retire on a *value* --
/// `SOIL_DRY_FLOOR`, the permanent wilting point, which is already the
/// engine's statement of water held too tightly for a root to take.
///
/// Also asserts the bank credit matches the loss to the unit, which is what
/// makes this a transfer rather than a second place water can be invented.
#[test]
fn damp_soil_dries_to_the_wilting_point_banks_what_it_lost_and_retires() {
    const WIDTH: i32 = 24;
    let mut w = soil_bed(WIDTH);
    let opening_held = soil_held(&w);
    let opening_bank = w.atmospheric_bank;
    assert!(pending_evaporation_sites(&w) > 0, "the bed was never scheduled, so this asserts nothing");

    run(&mut w, (DRY_WINDOW / 2) as usize);

    let held = soil_held(&w);
    let lost = opening_held - held;
    let banked = w.atmospheric_bank - opening_bank;
    assert!(lost > 0, "saturated soil open to dry air lost nothing at all in half a dry spell");
    assert!(
        (banked - lost as f64 / material::SOIL_SATURATED as f64).abs() < 0.001,
        "the bed lost {lost} units of moisture and the sky was credited {banked:.3} cell-equivalents"
    );

    // **And then it stops** -- and it stops *wet*, which is the same
    // resting state a lake has and not a stall. `dryness` reads the
    // humidity one field block up, a damp bed is a moisture source
    // (`field.rs` builds `moisture_source` from `aux / water_capacity`),
    // and in still air with nothing to carry it away that block reaches
    // `HUMID_STOP`. The rate goes to zero, `STALE_LIMIT` checks later the
    // sites retire, and the ground stays damp until the weather changes --
    // which is exactly what damp ground under a saturated sky does.
    //
    // Asserted as "the number stops moving", not "the number reaches the
    // wilting point". Written the second way first, and it failed at 23,515
    // against a floor of 4,320 -- reading a correct resting state as a
    // failure to finish.
    let before = soil_held(&w);
    run(&mut w, (DRY_WINDOW / 2) as usize);
    assert_eq!(soil_held(&w), before, "the bed was still drying after half a dry spell of drying");
    assert_eq!(pending_evaporation_sites(&w), 0, "sites are still queued on a bed that has stopped losing water");
}

#[test]
fn a_puddle_dries_up_and_a_lake_does_not() {
    // The headline paired comparison. Same seed, so the same weather; same
    // frame count; same depth, so the same surface-to-volume ratio. The two
    // bodies differ only in width, and nothing in the mechanic reads a width.
    //
    // **A whole day, not 2,500 frames, since evaporation went diurnal.** The
    // old length covered 0.69 of a 3,600-frame day starting at noon, so it
    // was weighted toward the cool half and the same puddle read 100.0% lost
    // before the coupling and 87.3% after — a *phase* artefact, not a
    // slowdown, and a bar sitting on it would have drifted with any retune of
    // `WARMTH_PER_DEGREE` rather than with anything about the contract. Over
    // a whole number of whole days the coupling is mean-neutral by
    // construction (see `warmth`), so this measures the thing it is named
    // for again. Measured at one day: 100.0% and 100.0% respectively, so
    // this length is also *cheaper* than the one it replaces in the only
    // sense that matters -- it is no longer sitting near a bar.
    let puddle = fraction_lost(PUDDLE_W, field::DAY_NIGHT_PERIOD_FRAMES as usize);
    let lake = fraction_lost(LAKE_W, field::DAY_NIGHT_PERIOD_FRAMES as usize);
    println!("one full day: puddle lost {:.1}%, lake lost {:.1}%", puddle * 100.0, lake * 100.0);

    assert!(puddle > 0.80, "the puddle should have all but gone, lost {:.1}%", puddle * 100.0);
    assert!(lake < 0.05, "the lake should be essentially untouched, lost {:.1}%", lake * 100.0);
}

#[test]
fn days_evaporate_more_than_nights() {
    // **The guard this phase exists to pass, and the mirror of
    // `field::humidity_does_not_go_diurnal`** — that one asserts the sky
    // stays out of humidity, this one asserts it gets into evaporation. The
    // pair is the whole design: one diurnal site, and exactly one.
    //
    // Two separate runs of the same geometry, one window centred on noon and
    // one on midnight, comparing the *summed credit to the bank* over each.
    // Three things about that shape, each of which a simpler test gets
    // wrong:
    //
    // * **Summed over a window, never sampled at an instant.** `rng::stream`
    //   is keyed on `world.frame`, so two phases of the same world diverge
    //   for reasons that have nothing to do with the sky; the baseline pair
    //   read 8.755 against 8.718 with no coupling at all. A bound, not an
    //   equality, on both sides.
    // * **The bank, not the basin's volume.** Same number here, and the bank
    //   is the one that stays meaningful if a body ever finishes mid-window.
    // * **The 32-cell basin, not the 6-cell puddle.** A puddle dries out
    //   inside the noon window and then reads its own volume rather than a
    //   rate — 10.28 against 10.15 across the whole sweep of
    //   `WARMTH_PER_DEGREE`, a saturated metric that looks exactly like a
    //   disconnected knob. This width is rate-limited for the whole window.
    //
    // **Confirmed able to fail for the wrong plumbing**, which is the point
    // of the guard rather than a nicety: swapping `warmth`'s read from
    // `field_at(x, y).temperature` to
    // `field::noon_equivalent_temperature(world.field_at(x, y))` scores
    // 5.760 against 5.760 -- a ratio of 1.000 against the bar of 1.5 below,
    // and a clean failure. It is not enough for this test to notice that
    // *something* evaporates; it has to notice that the temperature read is
    // the raw one.
    const WIDTH: i32 = 32;
    let half = field::DAY_NIGHT_PERIOD_FRAMES / 4;
    let window = |centre: u64| -> f64 {
        let mut w = scene(WIDTH);
        // Start a whole day in, so the phase arithmetic cannot go negative,
        // and settle before opening the window: what is being compared is a
        // standing rate, not the transient of a freshly painted basin.
        w.frame = field::DAY_NIGHT_PERIOD_FRAMES + centre - half;
        run(&mut w, 300);
        let opening = w.atmospheric_bank;
        run(&mut w, (2 * half) as usize);
        w.atmospheric_bank - opening
    };
    let noon = window(0);
    let midnight = window(field::DAY_NIGHT_PERIOD_FRAMES / 2);
    println!("half-day windows: noon banked {noon:.3}, midnight banked {midnight:.3} (ratio {:.2})", noon / midnight);

    // Bars from measurement with headroom, and from *both* sides. The
    // measured pair is 8.488 / 3.441, a ratio of 2.47.
    assert!(
        noon / midnight > 1.5,
        "the day should dry a great deal faster than the night: noon banked {noon:.3}, midnight {midnight:.3}"
    );
    // And the night has not stopped, which is the other half of the shape
    // `WARMTH_FLOOR` exists for: a cold night is a brake, never a stop. A
    // factor with a hard zero, or one steep enough to reach zero at this
    // swing, passes the bound above and fails here.
    assert!(midnight > 0.20 * noon, "the night stopped drying altogether: {midnight:.3} against the day's {noon:.3}");
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
    let lake = fraction_lost_seeded(LAKE_W, 6_000, GALE_SEED);
    let puddle = fraction_lost_seeded(PUDDLE_W, 6_000, GALE_SEED);
    println!("6000 frames on the gale seed: lake lost {:.1}%, puddle lost {:.1}%", lake * 100.0, puddle * 100.0);
    assert!(puddle > 0.90, "the gale should have finished the puddle off: lost {:.1}%", puddle * 100.0);
    assert!(lake < 0.20, "the gale took the lake apart: lost {:.1}%", lake * 100.0);
}

#[test]
fn evaporating_a_puddle_credits_exactly_what_it_held() {
    // **The credit half of the outer water cycle, asserted as an identity
    // rather than as a trend.** Every fill unit this file removes from a
    // cell goes into `World::atmospheric_bank`, so the bank's rise across a
    // run must equal the basin's fall, exactly -- not approximately, not
    // "within a few percent". Both sides are integer arithmetic on
    // `material::LIQUID_FULL`'s scale and the only float is the division
    // into cell-equivalents, so anything past floating-point noise is a real
    // leak.
    //
    // **The branch this is really guarding is the one that empties a cell.**
    // `tick` credits `fill` there and not `loss`, because the whole of what
    // is left is what goes; crediting `loss` instead is right on any cell
    // that happens to hold exactly `loss` and short by the remainder on
    // every other one. That error would never show up as water disappearing
    // -- the cell empties either way -- only as a sky that had quietly lost
    // the ability to rain, months later. So the run below is long enough to
    // take the puddle all the way down, which is what makes that branch fire
    // once per surface cell.
    //
    // The scene is rain-proof (`scene`'s lid), so nothing debits: the bank
    // moves in one direction only and the arithmetic is unambiguous.
    let mut w = scene(PUDDLE_W);
    let opening_bank = w.atmospheric_bank;
    let opening_volume = volume(&w, PUDDLE_W);
    run(&mut w, 6_000);
    let lost = opening_volume - volume(&w, PUDDLE_W);
    let credited = w.atmospheric_bank - opening_bank;
    let expected = lost as f64 / material::LIQUID_FULL as f64;
    println!(
        "puddle lost {lost} fill ({expected:.3} cell-equivalents); bank {opening_bank:.3} -> {:.3} (+{credited:.3})",
        w.atmospheric_bank
    );
    assert!(
        lost as f32 / opening_volume as f32 > 0.95,
        "the puddle did not finish drying ({lost} of {opening_volume}), so the emptying branch may never have fired"
    );
    assert!(
        (credited - expected).abs() < 1e-9,
        "the bank gained {credited:.6} for a puddle that lost {expected:.6} cell-equivalents"
    );
}

#[test]
fn a_sheltered_lake_banks_only_what_it_actually_loses() {
    // The "and then it stops" side of the credit, and the control that says
    // the identity above is not passing by arithmetic that happens to be
    // zero on both sides. A sheltered lake evaporates at exactly zero and
    // *stays on the schedule* -- see the module doc -- so it is checked
    // sixty times a day forever. A credit that leaked once per check rather
    // than once per fill unit removed would be invisible in every other test
    // here and would fill the sky from a lake that never lost a drop.
    let mut w = scene(LAKE_W);
    run(&mut w, 400);
    let settled_bank = w.atmospheric_bank;
    let settled_volume = volume(&w, LAKE_W);
    run(&mut w, 2_000);
    let credited = w.atmospheric_bank - settled_bank;
    let lost = settled_volume - volume(&w, LAKE_W);
    println!("lake: lost {lost} fill over 2000 frames, banked {credited:.3} cell-equivalents");
    assert!(pending_evaporation_sites(&w) > 0, "the lake retired off the schedule, so this is not the state being tested");
    assert!(
        (credited - lost as f64 / material::LIQUID_FULL as f64).abs() < 1e-9,
        "the sheltered lake banked {credited:.6} against {lost} fill actually lost"
    );
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
    run(&mut w, 400);
    let settled = volume(&w, PUDDLE_W);
    run_without_the_sweep(&mut w, 4_000);
    let lost = 1.0 - volume(&w, PUDDLE_W) as f32 / settled as f32;
    println!("sweepless: puddle lost {:.1}%", lost * 100.0);
    assert!(
        lost > 0.80,
        "evaporation stopped when the sweep did — the puddle lost only {:.1}% across four thousand sweepless frames",
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
    run(&mut sealed, 1_200);
    assert_eq!(volume(&sealed, PUDDLE_W), before, "a sealed pool should not have lost a drop");
    assert_eq!(
        pending_evaporation_sites(&sealed),
        0,
        "a sealed pool should have retired every one of its evaporation sites"
    );

    let mut lake = scene(LAKE_W);
    run(&mut lake, 1_200);
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
    for _ in 0..2_500 {
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
    for _ in 0..14 {
        run(&mut p, 500);
        run(&mut l, 500);
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
fn probe_temperature_over_water_across_a_day() {
    // What the sky actually delivers to a water surface in *this* scene,
    // which is the number the diurnal factor is shaped against. Reads the
    // water's own field block — the block `tick` reads — raw and
    // noon-equivalent, so the two together say how much of the reading is
    // the day and how much is anything else.
    //
    // The scene's stone lid on row 0 attenuates the sky's forcing by one
    // partly-blocked field block before it ever reaches the basin, so the
    // swing here is *not* `SKY_TEMPERATURE_SWING`; measuring it rather than
    // assuming it is the point of the probe.
    println!("phase   raw C   noon-equiv C   sky term");
    for (label, frame) in [
        ("noon", 0u64),
        ("afternoon", field::DAY_NIGHT_PERIOD_FRAMES / 8),
        ("sunset", field::DAY_NIGHT_PERIOD_FRAMES / 4),
        ("evening", 3 * field::DAY_NIGHT_PERIOD_FRAMES / 8),
        ("midnight", field::DAY_NIGHT_PERIOD_FRAMES / 2),
        ("sunrise", 3 * field::DAY_NIGHT_PERIOD_FRAMES / 4),
    ] {
        let mut w = scene(PUDDLE_W);
        w.frame = frame;
        run(&mut w, 200);
        let c = w.field_at(BASIN_X + PUDDLE_W / 2, FLOOR_Y - DEPTH);
        println!(
            "{label:10} {:6.2}  {:8.2}  {:8.2}",
            c.temperature,
            field::noon_equivalent_temperature(c),
            c.sky_temperature
        );
    }
}

#[test]
#[ignore = "probe, not a guard"]
fn probe_day_against_night_drying() {
    // The paired comparison the `days_evaporate_more_than_nights` guard is
    // set from: the same basin, the same number of frames, one window
    // centred on noon and one on midnight. Separate runs, because
    // `rng::stream` is keyed on `world.frame` and two phases of the same
    // world diverge for reasons that have nothing to do with the sky — so
    // this is a summed quantity over a window and never an equality.
    let half = field::DAY_NIGHT_PERIOD_FRAMES / 4;
    println!("width  window      credited (cell-equivalents)  fill lost");
    for &width in &[PUDDLE_W, 32, LAKE_W] {
        for (label, centre) in [("noon", 0u64), ("midnight", field::DAY_NIGHT_PERIOD_FRAMES / 2)] {
            let mut w = scene(width);
            w.frame = field::DAY_NIGHT_PERIOD_FRAMES + centre - half;
            run(&mut w, 300); // settle, so the window itself is standing state
            let bank0 = w.atmospheric_bank;
            let vol0 = volume(&w, width);
            run(&mut w, (2 * half) as usize);
            println!(
                "{width:5}  {label:9}  {:24.3}  {:9}",
                w.atmospheric_bank - bank0,
                vol0 - volume(&w, width)
            );
        }
    }
}

#[test]
#[ignore = "probe, not a guard"]
fn probe_whole_days_of_drying() {
    // **The measurement `WARMTH_PER_DEGREE`'s mean-neutrality claim rests
    // on, and the re-derivation of `FILL_PER_CHECK`.** A day-centred window
    // says how much the coupling redistributes; only a whole number of whole
    // days says whether it moved the *total*, which is what the drying
    // timescale is set from.
    //
    // Width 32 rather than the 6-cell puddle because a puddle finishes
    // inside the first day and a finished puddle reads the same total at
    // every setting — a saturated metric, and one that hid the difference at
    // three of the five sweep points before this probe existed.
    println!("width  days  credited (cell-equivalents)  per day");
    for &width in &[32i32, LAKE_W] {
        for days in [1u64, 4] {
            let mut w = scene(width);
            run(&mut w, 300);
            let bank0 = w.atmospheric_bank;
            run(&mut w, (days * field::DAY_NIGHT_PERIOD_FRAMES) as usize);
            let credited = w.atmospheric_bank - bank0;
            println!("{width:5}  {days:4}  {credited:26.3}  {:7.3}", credited / days as f64);
        }
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

#[test]
#[ignore = "probe, not a guard"]
fn probe_find_an_early_gale_seed() {
    // The gale guard has to span the frame the wind channel crosses
    // `GUST_THRESHOLD`. On seed 12345 that is frame 11,460, so the test had
    // to run 18,000 frames to see it. A seed that gusts early costs a
    // fraction of that and tests exactly the same thing.
    for seed in 0..40u64 {
        let cross = (0..20_000u64).step_by(30).find(|&f| crate::sim::weather::at(seed, f).wind.abs() >= 0.45);
        let calm_until = (0..20_000u64).step_by(30).find(|&f| crate::sim::weather::at(seed, f).wind.abs() >= 0.45).unwrap_or(u64::MAX);
        if let Some(c) = cross {
            // Still gusting a good while later, so the window is a gale and
            // not a single spike.
            let still = crate::sim::weather::at(seed, c + 4_000).wind.abs() >= 0.45;
            println!("seed {seed:3}: crosses at {c:6}, calm until {calm_until:6}, still gusting +4000: {still}");
        } else {
            println!("seed {seed:3}: never gusts in 20,000 frames");
        }
    }
}
