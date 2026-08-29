//! Does the colony actually *range*, and how far?
//!
//! **Nothing measured foraging range before this, and the counter that
//! looked like it did was measuring loitering.**
//! `CreatureStats::nest_visits` increments on any move made while
//! nest-adjacent, guarded on `OrganismState::since_nest > 0` — but
//! `since_nest` is incremented unconditionally every tick, so that guard is
//! false exactly once in a creature's life. An ant that never leaves home
//! scores one per tick. `examples/ascii.rs`'s `assert!(st.nest_visits > 0,
//! "no ant ever reached the nest")` is therefore not the sessility guard it
//! reads as: a colony that never leaves passes it trivially.
//!
//! The fix is not to repair `since_nest`. It cannot be repaired: it also
//! accumulates while an ant stands *on* the nest, and it counts ticks while
//! `tick_interval` is 6, so its scale is a species constant rather than a
//! distance. `CreatureStats::forage_trips` is a **spatial** excursion depth
//! that re-anchors at every nest contact — see `OrganismState::forage_anchor`.
//!
//! # The two scenes, and why both
//!
//! - **`control`** — one ant, a nest, and no food anywhere. Nothing to
//!   forage for, nowhere to go. This is the arm that the old counter fails:
//!   it reads hundreds of "nest visits" from an ant that has been home the
//!   whole time.
//! - **`forage`** — a colony, a nest, and a food pile a known distance away.
//!   The arm that proves the new counter is not merely always-zero. A probe
//!   that cannot produce the thing it counts has already cost this project
//!   four wrong conclusions; a metric only trustworthy when it reads 0 is
//!   one of them waiting to happen.
//!
//! Read them as a pair. Neither is worth anything alone.
//!
//! ```text
//! cargo run --release --example forage_probe
//! cargo run --release --example forage_probe -- frames=8000
//! ```

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material;
use pixel_physics::sim::{parallel, Cell, World};

/// Distance from the near edge of the nest patch to the near edge of the
/// food pile, in the `forage` scene. Printed beside the measured depth so
/// the number has something to be judged against: a colony whose deepest
/// excursion falls well short of this never reached the food at all.
const FOOD_GAP: i32 = 135 - 48;

/// The world seed this probe sweeps from (`BASE_SEED + i`).
///
/// The scene is hand-built, so there used to be no seed here at all and the
/// world simply kept `DEFAULT_WORLD_SEED`. That is not the same as having no
/// randomness: every creature draw is `rng::stream(world.seed, organism,
/// frame, slot)`, so the seed decides what the colony does, and a probe that
/// neither sets nor prints it was reporting one sample from a wide
/// distribution as though it were the number.
///
/// **`DEFAULT_WORLD_SEED`, not a fresh literal**, so `seeds=1` reproduces
/// every figure measured before this change bit for bit -- that is what the
/// hand-built scene was silently running on. Picking any other base would
/// have quietly invalidated the record while looking like a tidy-up: it moved
/// the control's moves from 64 to 68 on the first attempt here.
const BASE_SEED: u64 = pixel_physics::sim::world::DEFAULT_WORLD_SEED;

/// One run's headline figures, so `seeds=N` can order-statistic them.
///
/// House rule for a guard over a procedural system is an order statistic over
/// N seeds rather than any single seed, because which seed is worst reshuffles
/// on any legitimate change (`CLAUDE.md`, Conventions).
#[derive(Clone, Copy)]
struct Row {
    deliveries: u64,
    trips: u64,
    deepest: u64,
    moves: u64,
    blocked: u64,
    /// **`falls` is the guard `creature-motion-design.md` §7 names first**,
    /// and until 2026-08-29 it could only be read off `ascii`'s foraging
    /// scene -- one seed, one frame budget, no order statistic. That made
    /// the only number gating the impulse verb a single sample from a
    /// distribution this repo knows is wide. Carried per seed here so the
    /// ratio can be quoted the way every other bar in this file is.
    falls: u64,
    /// The pair `falls` needs to be read against, for the same reason
    /// `blocked` is printed beside `moves`: an ant that does not move has
    /// either been stopped (`blocked`), decided against it (`tumbles`) or
    /// left the ground (`falls`), and a rise in one is only interpretable
    /// beside the other two.
    tumbles: u64,
    /// **The instrument, carried per seed.** `seeds=N` used to report
    /// `deepest` and nothing else about the shape, and WP-9's decision rule
    /// is written on the `>=32`/`>=64` buckets -- so the multi-seed mode
    /// could not answer the question it was quoted for. `deepest` is one
    /// point on this curve: it says the furthest excursion happened, never
    /// how much weight is out there. One ant that wandered and 300 that did
    /// read identically on it.
    reach: [u64; 8],
}

fn run(world: &mut World, frames: usize) {
    for _ in 0..frames {
        parallel::step(world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
    }
}

fn row(world: &World) -> Row {
    let st = world.creature_stats;
    Row {
        deliveries: st.deliveries,
        trips: st.forage_trips,
        deepest: st.forage_depth_max,
        moves: st.moves,
        blocked: st.moves_blocked,
        falls: st.falls,
        tumbles: st.tumbles,
        reach: st.forage_reach,
    }
}

/// **The falls-per-move bar, and the frame budget it is only true at.**
///
/// `Reports/creature-motion-design.md` §7 names falls per move as the first
/// guard the impulse verb ships with, because it is the failure mode of both
/// earlier attempts at airborne creatures — 59-80% of all moves, twice, and
/// reverted twice. This is the bar, and two things about it are load-bearing:
///
/// **It sits above the measured maximum, not on the median.** Twelve seeds on
/// `main` at 2026-08-29 read min 0.208, median 0.225, **max 0.334**. A bar at
/// the median gets rubber-stamped the moment which seed is worst reshuffles,
/// which `CLAUDE.md` records happening to a six-seed anchor census that read
/// 1.64x and then 1.08x over the next twelve. 0.40 is ~20% clear of the worst
/// seed and less than half the 0.59 floor of the failure it is named for.
///
/// **It is only meaningful at one frame budget.** The statistic does not
/// settle: lane C measured 0.239 / 0.225 / 0.215 at 6,000 / 12,000 / 24,000
/// frames on the same scene. A bar quoted without its budget is therefore not
/// reproducible, so `gate=1` **refuses to run** at any other budget rather
/// than comparing against a number that does not apply — the same reason a
/// noise bar does not transfer between jobs.
const FALLS_PER_MOVE_BAR: f64 = 0.40;
/// The pinned operand [`FALLS_PER_MOVE_BAR`] was measured at.
const GATE_FRAMES: usize = 12_000;
/// And the smallest sweep it may be read over. Six seeds is not a sweep.
const GATE_MIN_SEEDS: u64 = 12;

/// min / median / max of one column, printed rather than a mean.
///
/// A mean hides which end moved; the spread is the whole reason this sweeps
/// seeds at all.
fn order_stats(mut v: Vec<f64>) -> (f64, f64, f64) {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    let median = if n == 0 {
        0.0
    } else if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    };
    (*v.first().unwrap_or(&0.0), median, *v.last().unwrap_or(&0.0))
}

fn report(label: &str, world: &World, ants: usize) {
    let st = world.creature_stats;
    println!("\n--- {label} ({ants} ant(s)) ---");
    println!(
        "  moves {} | blocked {} | pickups {} | drops {} | deliveries {} | deaths {}",
        st.moves, st.moves_blocked, st.pickups, st.drops, st.deliveries, st.deaths
    );
    // **`falls` beside `moves`, because §7's guard is the ratio of the two.**
    // `tumbles` beside them because a tick that produced no move either was
    // stopped, was declined, or ended in the air, and only all three
    // together say which.
    let fpm = if st.moves > 0 { st.falls as f64 / st.moves as f64 } else { 0.0 };
    let tpm = if st.moves > 0 { st.tumbles as f64 / st.moves as f64 } else { 0.0 };
    println!("  falls {} ({fpm:.3} of moves) | tumbles {} ({tpm:.3} of moves)", st.falls, st.tumbles);
    // **The scheduling cadence, and this scene is its positive control.**
    // Nothing else is on the active-site queue here -- a hand-built floor,
    // no worldgen, no forest, no destruction -- so `late mean` must read
    // 0.0 and `asked` must reach its ideal. An instrument that cannot
    // report "on time" in the one case built to be on time cannot report
    // "late" anywhere else either (`CLAUDE.md`: run the positive control).
    println!(
        "  ticks {} | late mean {:.2} frames, max {} | moves/tick {:.3}",
        st.ticks,
        if st.ticks > 0 { st.tick_lag_sum as f64 / st.ticks as f64 } else { 0.0 },
        st.tick_lag_max,
        if st.ticks > 0 { st.moves as f64 / st.ticks as f64 } else { 0.0 },
    );
    // **The verb's own "did it fire at all" line.** `CLAUDE.md`'s house
    // rule, and the case it was paid for with: a collapse read as "chunks
    // are working" off a picture whose event count was zero for the whole
    // run. `impulses` is the firing count, `refused` its effect-side pair
    // (a launch the body could not make), and `frames/impulse` is the mean
    // time aloft -- the quantity a body is supposed to change, since the
    // same launch keeps a slab up 2.3x as long as a block.
    //
    // **Kept beside the cadence line rather than folded into it**, because
    // the two disagree about what a tick is: a creature in the air is
    // rescheduled every frame and does not think, so its frames land in
    // `airborne frames` and never in `ticks`. Summing them would produce a
    // cadence figure belonging to neither.
    //
    // Printed unconditionally, including the zeros. A species with no
    // `Impulse` weight must read `impulses 0` here, and a line that
    // disappears when it is zero cannot make that claim.
    let per = if st.impulses > 0 { st.flight_frames as f64 / st.impulses as f64 } else { 0.0 };
    println!(
        "  impulses {} (refused {}) | airborne frames {} ({per:.1} per launch) | flight moves {}",
        st.impulses, st.impulses_refused, st.flight_frames, st.flight_moves
    );
    // **The old counter and the new one on the same line, on the same run.**
    // Two mechanisms that look identical in a summary are exactly what this
    // pair exists to separate, so they are never printed apart.
    let ratio = if st.moves > 0 { st.nest_visits as f64 / st.moves as f64 } else { 0.0 };
    println!("  nest_visits {} ({ratio:.3} of moves -- loitering, NOT trips)", st.nest_visits);
    let mean = if st.forage_trips > 0 { st.forage_depth_sum as f64 / st.forage_trips as f64 } else { 0.0 };
    println!(
        "  forage_trips {} (bar {} cells) | mean depth {mean:.1} cells | deepest {} cells",
        st.forage_trips,
        pixel_physics::sim::creature::FORAGE_TRIP_MIN,
        st.forage_depth_max
    );
    // **The profile, always, next to the count.** The count is one point on
    // this curve and cannot show its shape; the shape is what says whether
    // a colony hops or ranges.
    let prof: Vec<String> = pixel_physics::sim::creature::FORAGE_REACH_BUCKETS
        .iter()
        .zip(&st.forage_reach)
        .map(|(edge, n)| format!(">={edge}: {n}"))
        .collect();
    println!("  excursions reaching at least N cells -- {}", prof.join("  "));
    // **Per ant, because the two scenes do not have the same number of
    // them.** A raw count from 55 ants against a raw count from 1 compares
    // colony size, not foraging, and reading it as foraging is how a jammed
    // colony looks busy.
    println!(
        "  per ant: {:.2} moves, {:.3} trips, deepest {} cells",
        st.moves as f64 / ants as f64,
        st.forage_trips as f64 / ants as f64,
        st.forage_depth_max
    );
}

/// The two knobs that define an *arm* of a paired run, together because they
/// travel together: a comparison is only worth anything when everything except
/// the arm is held identical, and passing them as loose parameters is how one
/// of them gets forgotten at a call site.
#[derive(Clone, Copy)]
struct Arm {
    /// `None` defers to `ant.ron`; `Some(_)` forces the flag at runtime.
    climb: Option<bool>,
    /// Cells between planted ants. 2 is historical, 4 is `COLONY_ANT_SPACING`.
    spacing: i32,
}

/// Stone floor, a nest patch, and nothing else. `plant_ant` needs solid
/// ground under it or the ant spends the run falling, which is its own
/// wrong conclusion.
fn scene(w: i32, h: i32, floor: i32, ants: usize, food: bool, seed: u64, arm: Arm) -> (World, usize) {
    let Arm { climb, spacing } = arm;
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    // Set before anything is planted: every creature draw keys off it.
    world.seed = seed;
    // **The arm switch, applied to the loaded species rather than the
    // asset.** WP-9's paired arms were first run by editing
    // `ant.ron` and rebuilding between them, which works and is exactly the
    // shape of the `include_str!` disaster this repo has paid for twice: the
    // knob lives in a file the binary only reads at compile time, so a
    // forgotten rebuild produces two byte-identical "arms" that look like a
    // null result. Overriding the def here makes both arms run from **one**
    // binary in one session, so no rebuild sits between them and there is
    // nothing to forget. `None` changes nothing and defers to the asset.
    if let Some(on) = climb {
        let id = world.species.id_of("ant").expect("ant is compiled in");
        let mut def = world.species.get(id).creature.as_ref().expect("ant has a creature block").clone();
        def.climbs_over_kin = on;
        world.species.set_creature(id, def);
    }
    let nest = world.materials.id_of("nest").expect("nest is compiled in");
    let corpse = world.materials.id_of("corpse").expect("corpse is compiled in");
    for x in 0..w {
        for y in floor..h {
            world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
    for x in 16..48 {
        world.set(x, floor, Cell::new(nest, 0).with_attached(true));
    }
    if food {
        for x in 135..170 {
            for y in (floor - 5)..floor {
                world.set(x, y, Cell::new(corpse, 0));
            }
        }
    }
    // **How far apart the colony starts, and why it is a knob.**
    //
    // This was a bare `* 2`, chosen when the scene existed only to give the
    // range instrument something to measure. Nobody picked 2 as a jam -- but
    // 2 *is* the jam: an ant is a two-cell body, so spacing 2 is shoulder to
    // shoulder, which is the exact configuration dead ends 775/829 record as
    // gridlock (27,386 blocked ticks against a single pickup) and the reason
    // `COLONY_ANT_SPACING` is **4**.
    //
    // That made this scene jammed *by construction* while every shipping
    // colony -- `found_colony` and `ascii`'s foraging loop alike -- is
    // spaced 4 and is not. Measured at both, flag off: spacing alone takes
    // deepest 23.5 -> 46 and the `>=32` bucket 0 -> 4.5, which is most of
    // what WP-9 arm 1's headline 22x was crediting to climb-over.
    //
    // **It is not the whole story, and the honest limit is worth writing
    // down.** At spacing 4 this scene still shows a large climb-over gain
    // (deepest 46 -> 84), while `ascii`'s foraging loop -- also spaced 4 --
    // shows none at all (19 -> 19). So spacing explains the gap between this
    // probe's headline and a real colony's, but something else explains
    // `ascii`: different horizon, real terrain rather than a flat stone
    // floor, and food that grows and falls rather than one pile at a known
    // 87 cells. Untested which.
    //
    // A confound nobody can vary is a confound nobody can see, so it varies
    // now. Default 2 keeps every previously recorded figure exact.
    for i in 0..ants {
        world.plant_ant(20 + i as i32 * spacing, floor - 1);
    }
    (world, ants)
}

fn main() {
    let mut frames = 6000usize;
    let mut seeds = 1u64;
    // `None` = whatever `ant.ron` says. `climb=0` / `climb=1` force the arm.
    let mut climb: Option<bool> = None;
    // 2 = the historical value (shoulder to shoulder). 4 = COLONY_ANT_SPACING,
    // what a real colony is founded at.
    let mut spacing = 2i32;
    // See `FALLS_PER_MOVE_BAR`. Off by default: this is a measurement
    // harness first, and a bar that fires on every exploratory run gets
    // ignored rather than read.
    let mut gate = false;
    for arg in std::env::args().skip(1) {
        // **A bare argument is an error, not a shrug.** This loop used to
        // `continue` past anything without an `=`, so `forage_probe 8` ran
        // the default and said nothing about it.
        let Some((k, v)) = arg.split_once('=') else {
            panic!("unknown arg {arg:?}; known: frames, seeds, climb, spacing");
        };
        match k {
            "frames" => frames = v.parse().expect("frames"),
            "seeds" => seeds = v.parse().expect("seeds"),
            "climb" => {
                climb = Some(match v {
                    "0" | "off" | "false" => false,
                    "1" | "on" | "true" => true,
                    other => panic!("climb takes 0/1 (or off/on), got {other:?}"),
                })
            }
            "spacing" => spacing = v.parse().expect("spacing"),
            "gate" => gate = v != "0" && v != "false",
            other => panic!("unknown arg {other:?}; known: frames, seeds, climb, spacing"),
        }
    }
    println!(
        "forage_probe: frames={frames} seeds={seeds} base_seed={BASE_SEED:#x} FORAGE_TRIP_MIN={} climbs_over_kin={} ant_spacing={spacing}",
        pixel_physics::sim::creature::FORAGE_TRIP_MIN,
        match climb {
            None => "(from ant.ron)".to_string(),
            Some(on) => format!("{on} (forced)"),
        }
    );

    let arm = Arm { climb, spacing };

    let (w, h) = (320i32, 120i32);
    let floor = h - 8;

    // One seed keeps the old single-run output verbatim, so every figure
    // measured before this still compares.
    if seeds == 1 {
        let (mut control, n) = scene(w, h, floor, 1, false, BASE_SEED, arm);
        run(&mut control, frames);
        report("control: one ant, a nest, no food", &control, n);

        let (mut forage, n) = scene(w, h, floor, 55, true, BASE_SEED, arm);
        run(&mut forage, frames);
        report("forage: 55 ants, a nest, food at range", &forage, n);
        println!("  (nest edge to food edge is {FOOD_GAP} cells -- the depth a real round trip has to reach)");
        return;
    }

    // **The instrument every foraging claim has to quote from here on.**
    // Both scenes per seed, per-seed rows, and an order statistic over them
    // -- not a mean, and not one seed. Outcomes here have a standard
    // deviation around 0.1, so a bar set from a single run is a sample from
    // a wide distribution and flakes in whichever direction that run landed.
    println!("\n  both scenes per seed; control is one ant with no food, forage is 55 ants with food at {FOOD_GAP} cells\n");
    // The offset, not the full 16-hex-digit seed: line one already names the
    // base, and `base+3` is what a reader needs to reproduce a row.
    println!(
        "{:>6}  {:>10} {:>7} {:>8} {:>6} {:>6} {:>8} {:>8}  |  {:>10} {:>9} {:>8}",
        "seed", "deliveries", "trips", "deepest", ">=32", ">=64", "moves", "blocked", "ctl moves", "ctl trips", "ctl deep"
    );
    let mut forage_rows: Vec<Row> = Vec::new();
    for i in 0..seeds {
        let seed = BASE_SEED + i;
        let (mut control, _) = scene(w, h, floor, 1, false, seed, arm);
        run(&mut control, frames);
        let c = row(&control);

        let (mut forage, _) = scene(w, h, floor, 55, true, seed, arm);
        run(&mut forage, frames);
        let f = row(&forage);
        println!(
            "{:>6}  {:>10} {:>7} {:>8} {:>6} {:>6} {:>8} {:>8}  |  {:>10} {:>9} {:>8}",
            format!("+{i}"),
            f.deliveries,
            f.trips,
            f.deepest,
            f.reach[5],
            f.reach[6],
            f.moves,
            f.blocked,
            c.moves,
            c.trips,
            c.deepest
        );
        forage_rows.push(f);
    }
    println!("\n{:>10}  {:>8} {:>8} {:>8}", "", "min", "median", "max");
    for (name, col) in [
        ("deliveries", forage_rows.iter().map(|r| r.deliveries as f64).collect::<Vec<_>>()),
        ("trips", forage_rows.iter().map(|r| r.trips as f64).collect()),
        ("deepest", forage_rows.iter().map(|r| r.deepest as f64).collect()),
        ("moves", forage_rows.iter().map(|r| r.moves as f64).collect()),
        // **The two buckets WP-9's decision rule is actually written on.**
        // "Weight appears in the >=32/>=64 reach buckets" is the success
        // condition; `deepest` is a max and cannot show weight.
        (">=32", forage_rows.iter().map(|r| r.reach[5] as f64).collect()),
        (">=64", forage_rows.iter().map(|r| r.reach[6] as f64).collect()),
    ] {
        let (lo, med, hi) = order_stats(col);
        println!("{name:>10}  {lo:>8.1} {med:>8.1} {hi:>8.1}");
    }
    // Blocked-move fraction is the jam readout, and it is the pair the
    // traffic work (WP-9) has to quote beside any reach claim.
    let blocked: Vec<f64> = forage_rows.iter().map(|r| if r.moves > 0 { r.blocked as f64 / r.moves as f64 } else { 0.0 }).collect();
    let (lo, med, hi) = order_stats(blocked);
    println!("{:>10}  {lo:>8.3} {med:>8.3} {hi:>8.3}", "blocked/mv");
    // **The §7 guard, as an order statistic rather than one seed.** Printed
    // as a ratio and not a count because that is how the bar is written
    // ("falls at 59-80% of all moves" is the failure it exists to catch),
    // and a raw fall count rises with any change that simply makes ants
    // move more.
    let falls: Vec<f64> = forage_rows.iter().map(|r| if r.moves > 0 { r.falls as f64 / r.moves as f64 } else { 0.0 }).collect();
    let (lo, med, hi) = order_stats(falls);
    println!("{:>10}  {lo:>8.3} {med:>8.3} {hi:>8.3}", "falls/mv");
    // Tumbles per *tick* would be the cleaner denominator, but the tick
    // count is not on `CreatureStats`; against moves it still separates
    // "stopped" from "chose not to", which is the fork a slowdown report
    // has to answer.
    let tumbles: Vec<f64> = forage_rows.iter().map(|r| if r.moves > 0 { r.tumbles as f64 / r.moves as f64 } else { 0.0 }).collect();
    let (lo, med, hi) = order_stats(tumbles);
    println!("{:>10}  {lo:>8.3} {med:>8.3} {hi:>8.3}", "tumbles/mv");

    // **The gate, when asked for.** Off by default because this harness is a
    // measurement first; `gate=1` turns the order statistic above into a
    // pass/fail with a non-zero exit, so it can be put in front of any change
    // to how a creature moves.
    if gate {
        let falls_max = order_stats(forage_rows.iter().map(|r| if r.moves > 0 { r.falls as f64 / r.moves as f64 } else { 0.0 }).collect()).2;
        // **Both terms, because the ratio alone cannot say which one moved.**
        // Measured 2026-08-29, wiring the impulse verb into `ant.ron` and
        // re-running this exact sweep: falls/move went 0.225 -> 0.298 and the
        // gate fired -- while the *absolute* falls went from ~7,430 to
        // ~1,520. The colony fell a fifth as often. `moves` counts walking
        // steps only, so a species that hops instead of walking collapses the
        // denominator (33,020 -> 5,100 median) and the ratio rises on a
        // numerator that fell. Read the ratio for the species it was
        // baselined on; read these two for anything that leaves the ground.
        let falls_med = order_stats(forage_rows.iter().map(|r| r.falls as f64).collect()).1;
        let moves_med = order_stats(forage_rows.iter().map(|r| r.moves as f64).collect()).1;
        println!("      falls  {falls_med:>8.0}  (median, absolute) against {moves_med:>8.0} walking moves");
        if frames != GATE_FRAMES || seeds < GATE_MIN_SEEDS {
            eprintln!(
                "gate=1 refused: the bar is {FALLS_PER_MOVE_BAR:.2} measured at frames={GATE_FRAMES} over >={GATE_MIN_SEEDS} seeds, \
                 and this run is frames={frames} seeds={seeds}. The statistic does not settle -- 0.239/0.225/0.215 at \
                 6k/12k/24k on this scene -- so comparing across budgets compares nothing."
            );
            std::process::exit(2);
        }
        if falls_max > FALLS_PER_MOVE_BAR {
            eprintln!(
                "GATE FAIL: worst seed falls/move {falls_max:.3} is over the {FALLS_PER_MOVE_BAR:.2} bar \
                 (median absolute falls {falls_med:.0} against {moves_med:.0} walking moves). \
                 Reports/creature-motion-design.md §2d, §7. \
                 **Check which term moved before concluding anything**: this ratio is the failure \
                 two reverted attempts had (59-80%) only when the numerator is what rose. A species \
                 that hops walks less, so `moves` collapses and the ratio climbs on falls that fell."
            );
            std::process::exit(1);
        }
        println!("\nGATE PASS: worst seed falls/move {falls_max:.3} <= {FALLS_PER_MOVE_BAR:.2} over {seeds} seeds at {frames} frames");
    }

    // **The shape, pooled over every seed.** An order statistic per bucket
    // answers "is it reliable"; the pooled curve answers "what does the
    // colony do" -- a sessile colony is a spike at `>=1` that has vanished
    // by `>=2`, a ranging one carries weight outward. Printed summed rather
    // than averaged so it stays a count of real excursions.
    let mut pooled = [0u64; 8];
    for r in &forage_rows {
        for (p, n) in pooled.iter_mut().zip(&r.reach) {
            *p += n;
        }
    }
    let prof: Vec<String> = pixel_physics::sim::creature::FORAGE_REACH_BUCKETS
        .iter()
        .zip(&pooled)
        .map(|(edge, n)| format!(">={edge}: {n}"))
        .collect();
    println!("\n  pooled excursion profile over {seeds} seed(s) -- {}", prof.join("  "));
}
