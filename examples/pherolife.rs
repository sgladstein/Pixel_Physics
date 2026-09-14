//! **How long does a trail live once nobody is re-laying it?**
//!
//! The owner's question, 2026-09-14: *"do they fade too fast to be
//! useful?"* Nothing in the tree could answer it, and that is the reason
//! this file exists rather than a sweep of an existing one.
//!
//! `pheromone::tests::trail_following_sweep` — the harness `DIFFUSE` and
//! `DECAY_RHO` were both set from — runs **a single follower that re-lays
//! its trail every pass**, so decay never has to be *survived*. It measures
//! how well an ant tracks a trail that is being continuously rewritten
//! underneath it. That is a real question and it is not this one. A real
//! colony lays a cell once and comes back minutes later, and
//! `DECAY_RHO`'s own doc says so while noting it was still set from the
//! continuous harness (0.03, deliberately below the literature band of
//! 0.1–0.5).
//!
//! So: lay a trail, **stop**, and watch it.
//!
//! ```text
//! cargo run --release --example pherolife                 # the shipped constants
//! cargo run --release --example pherolife -- sweep=rho    # against the literature band
//! cargo run --release --example pherolife -- sweep=deposit
//! cargo run --release --example pherolife -- mode=traffic # what holds a trail up
//! cargo run --release --example pherolife -- selftest     # the positive control
//! ```
//!
//! # What it reports, and why a peak is not enough
//!
//! `CLAUDE.md`: pair every "it fired" counter with an **effect** counter
//! from the far side of the call. A trail's height is the "it fired" half —
//! the plane is nonzero, something is there. It does not say an ant can
//! *use* it, and on this engine the two come apart, because the input an
//! ant actually reads is scale-free:
//!
//! ```text
//! PheroAAlong = (ahead - here) / (ahead + here + 1)
//! ```
//!
//! A trail at 4 and a trail at 200 can give the same reading. So the
//! effect counter here is the **run drive** — the contribution the ant's
//! own authored wiring makes to `BrainOutput::Move` from that input, at
//! the weights in `assets/species/ant.ron`:
//!
//! ```text
//! 2.5 * (squash(b + 6*along) - squash(b - 6*along))
//! ```
//!
//! …which is the gated hidden pair 0/1 (`(PheroAAlong, 0, 6.0)`,
//! `(PheroAAlong, 1, -6.0)`, `(0, Move, 2.5)`, `(1, Move, -2.5)`) with the
//! gate open, i.e. a laden ant. That number going to zero *is* the trail
//! ceasing to steer, whatever the plane still holds.
//!
//! # The two scenes, and why a flat trail is the wrong one
//!
//! `ramp` is the default and is the trail channel A actually is. The
//! emitting weight is `hidden 4 -> EmitA` off a self-recurrent odometer
//! charged at the nest (`creature.rs`'s deposit site), so an ant lays a
//! lot near home and less the further out it gets: channel A is a **ramp**
//! a laden ant climbs. A flat trail has `ahead == here` everywhere in its
//! body, so the along-input is 0 by construction and the scene cannot show
//! the quantity under test — exactly the `CLAUDE.md` failure where a scene
//! does not contain the thing being measured. `flat` is kept as the
//! control that demonstrates it.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::Lab;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::pheromone::{self, Channel, Pheromones};

/// The ant's `sensor_offset` (`assets/species/ant.ron`). The along-input
/// reads `here` and the cell this far ahead.
const SENSOR_OFFSET: i32 = 6;
/// `(PheroAAlong, 0, 6.0)` / `(PheroAAlong, 1, -6.0)` in `ant.ron`.
const W_ALONG: f32 = 6.0;
/// `(0, Move, 2.5)` / `(1, Move, -2.5)`.
const W_MOVE: f32 = 2.5;
/// `Bias -45` + `Carrying 45.5` — the gate held open, i.e. a laden ant.
const GATE_OPEN: f32 = 0.5;

/// A colony round trip, in frames. The number the answer is *against*:
/// `PHEROMONE_INTERVAL`'s own doc prices it at "roughly 2,200" and that is
/// where its 255-pass ceiling argument gets its margin.
const ROUND_TRIP: u64 = 2_200;

fn squash(x: f32) -> f32 {
    x / (1.0 + x.abs())
}

/// What `ant.ron`'s gated pair puts into `Move` for a given along-reading.
fn run_drive(along: f32) -> f32 {
    W_MOVE * (squash(GATE_OPEN + W_ALONG * along) - squash(GATE_OPEN - W_ALONG * along))
}

struct Args {
    mode: String,
    sweep: String,
    deposit: u8,
    rho: f32,
    diffuse: f32,
    interval: u64,
    len: i32,
    scene: String,
    selftest: bool,
}

impl Args {
    fn parse() -> Self {
        let mut a = Args {
            mode: "life".into(),
            sweep: String::new(),
            deposit: pheromone::DEPOSIT,
            rho: pheromone::DECAY_RHO,
            diffuse: pheromone::DIFFUSE,
            interval: pheromone::PHEROMONE_INTERVAL,
            len: 120,
            scene: "ramp".into(),
            selftest: false,
        };
        for arg in std::env::args().skip(1) {
            if arg == "selftest" {
                a.selftest = true;
                continue;
            }
            let Some((k, v)) = arg.split_once('=') else { continue };
            match k {
                "mode" => a.mode = v.into(),
                "sweep" => a.sweep = v.into(),
                "deposit" => a.deposit = v.parse().unwrap_or(a.deposit),
                "rho" => a.rho = v.parse().unwrap_or(a.rho),
                "diffuse" => a.diffuse = v.parse().unwrap_or(a.diffuse),
                "interval" => a.interval = v.parse().unwrap_or(a.interval),
                "len" => a.len = v.parse().unwrap_or(a.len),
                "scene" => a.scene = v.into(),
                _ => eprintln!("pherolife: ignoring unknown argument `{arg}`"),
            }
        }
        a
    }
}

/// The world the lab bed runs at, near enough — the plane is world-sized
/// and the trail sits in one tile row of it.
fn bounds() -> Rect {
    Rect::new(0, 0, 511, 319)
}

const TRAIL_Y: i32 = 200;
const TRAIL_X0: i32 = 60;

/// Lay one trail and return the plane holding it, un-stepped.
///
/// `ramp`: what the odometer actually writes — full deposit at the nest
/// end, falling hyperbolically with distance, which is the curve
/// `creature.rs`'s deposit comment fits (`0.785 -> 0.046` over 3,000
/// ticks after a 5-tick nest touch). Approximated here as `1/(1 + k*d)`,
/// normalized so the nest end is the full deposit.
fn lay(p: &mut Pheromones, args: &Args) {
    for i in 0..args.len {
        let x = TRAIL_X0 + i;
        let amount = match args.scene.as_str() {
            "flat" => args.deposit,
            _ => {
                let d = i as f32 / args.len as f32;
                let f = 1.0 / (1.0 + 8.0 * d);
                (args.deposit as f32 * f).round().max(1.0) as u8
            }
        };
        p.deposit(Channel::A, x, TRAIL_Y, amount);
    }
}

/// The along-input an ant standing at `x` reads, exactly as
/// `creature::sense` computes it.
fn along_at(p: &Pheromones, x: i32) -> f32 {
    let here = p.sample(Channel::A, x, TRAIL_Y) as f32;
    let ahead = p.sample(Channel::A, x + SENSOR_OFFSET, TRAIL_Y) as f32;
    (ahead - here) / (ahead + here + 1.0)
}

/// **The bar for "stopped steering", and it is absolute rather than a
/// fraction of where the arm started.**
///
/// A fraction-of-initial bar cannot rank two arms, because each arm's
/// starting drive is itself quantization-dependent (the shipped deposit
/// and a saturated one open at 0.889 and 0.553 on the same ramp) — so it
/// silently asks a different question of every row, which is
/// `CLAUDE.md`'s "a pass/fail read of a graded quantity hides the
/// gradient" wearing a ratio. This is a level: `ant.ron` runs on a Move
/// baseline of 2.0, so a drive of 0.25 is the trail moving the run
/// probability by about a tenth. Below that the ant is walking, not
/// following. **The trace below prints the whole curve** — read that
/// rather than this crossing wherever the two disagree.
const DRIVE_FLOOR: f32 = 0.25;

struct Life {
    /// Frames until the plane is empty — `max() == 0`.
    frames_to_zero: u64,
    /// Frames until the ant's run drive at mid-trail falls under
    /// [`DRIVE_FLOOR`]. The trail is still *there* after this; it has
    /// stopped steering.
    frames_to_unusable: u64,
    peak0: u8,
    drive0: f32,
    /// `(frame, peak, live cells, drive)` every `TRACE_EVERY` passes,
    /// so the crossing above is never the only readout.
    trace: Vec<(u64, u8, usize, f32)>,
}

/// Passes between trace samples.
const TRACE_EVERY: u64 = 2;

fn measure_life(args: &Args, rho: f32, diffuse: f32, deposit: u8) -> Life {
    let a = Args { rho, diffuse, deposit, ..clone_args(args) };
    let mut p = Pheromones::new(bounds());
    p.set_channel_rho(Channel::A, rho);
    p.set_channel_diffuse(Channel::A, diffuse);
    lay(&mut p, &a);

    // Read the ant at the mid-trail, climbing toward the nest end. The
    // along-input is signed by direction of travel; take its magnitude,
    // since an ant facing the other way reads the same gradient inverted.
    let mid = TRAIL_X0 + args.len / 2;
    let peak0 = p.plane(Channel::A).max();
    let drive0 = run_drive(along_at(&p, mid)).abs();

    let mut frames_to_zero = 0u64;
    let mut frames_to_unusable = 0u64;
    let mut frame = 0u64;
    let mut passes = 0u64;
    let mut trace = vec![(0u64, peak0, live_cells(&p), drive0)];
    // A generous budget: three round trips. If a trail is still standing
    // at 6,600 frames the answer to "does it fade too fast" is no.
    while frame < ROUND_TRIP * 3 {
        frame += 1;
        let is_pass = frame.is_multiple_of(args.interval);
        p.step(frame, args.interval);
        let drive = run_drive(along_at(&p, mid)).abs();
        if frames_to_unusable == 0 && drive < DRIVE_FLOOR {
            frames_to_unusable = frame;
        }
        if is_pass {
            passes += 1;
            if passes.is_multiple_of(TRACE_EVERY) {
                trace.push((frame, p.plane(Channel::A).max(), live_cells(&p), drive));
            }
        }
        if p.plane(Channel::A).max() == 0 {
            frames_to_zero = frame;
            break;
        }
    }
    if frames_to_zero == 0 {
        frames_to_zero = u64::MAX;
    }
    if frames_to_unusable == 0 {
        frames_to_unusable = u64::MAX;
    }
    Life { frames_to_zero, frames_to_unusable, peak0, drive0, trace }
}

/// Cells of the trail still holding anything. The "how much of it is
/// left" half — a peak can be carried by one cell.
fn live_cells(p: &Pheromones) -> usize {
    (TRAIL_X0 - 4..TRAIL_X0 + 400).filter(|&x| p.sample(Channel::A, x, TRAIL_Y) > 0).count()
}

fn fmt(frames: u64) -> String {
    if frames == u64::MAX {
        "  >3 trips".into()
    } else {
        format!("{frames:5}  {:.2}x", frames as f32 / ROUND_TRIP as f32)
    }
}

fn main() {
    let args = Args::parse();
    if args.selftest {
        selftest(&args);
        return;
    }
    println!("pherolife: scene={} len={} interval={} round trip={ROUND_TRIP} frames", args.scene, args.len, args.interval);
    println!("           a trail is laid once at frame 0 and NOTHING re-lays it.");
    println!();
    match args.mode.as_str() {
        "traffic" => traffic(&args),
        "alarm" => alarm(&args),
        "world" => world(&args),
        _ => life(&args),
    }
}

fn life(args: &Args) {
    let rows: Vec<(String, f32, f32, u8)> = match args.sweep.as_str() {
        "rho" => vec![
            ("rho 0.03 shipped".into(), 0.03, args.diffuse, args.deposit),
            ("rho 0.10 lit low".into(), 0.10, args.diffuse, args.deposit),
            ("rho 0.25 lit mid".into(), 0.25, args.diffuse, args.deposit),
            ("rho 0.50 lit top".into(), 0.50, args.diffuse, args.deposit),
        ],
        "deposit" => vec![
            ("dep  40 one ant".into(), args.rho, args.diffuse, 40),
            ("dep  80 two ants".into(), args.rho, args.diffuse, 80),
            ("dep 240 six ants".into(), args.rho, args.diffuse, 240),
            ("dep 255 saturated".into(), args.rho, args.diffuse, 255),
        ],
        // **The decomposition, and it is the point of the whole file.**
        // `diffuse = 0` leaves decay alone on the trail; `rho = 0` leaves
        // the blend and the LUT floor. Whichever arm looks like the
        // shipped one is the term that is doing the work.
        "diffuse" => vec![
            ("shipped both".into(), args.rho, args.diffuse, args.deposit),
            ("decay only".into(), args.rho, 0.0, args.deposit),
            ("blend only".into(), 0.0, args.diffuse, args.deposit),
            ("neither (floor)".into(), 0.0, 0.0, args.deposit),
            ("blend 0.10".into(), args.rho, 0.10, args.deposit),
            ("blend 0.50".into(), args.rho, 0.50, args.deposit),
        ],
        _ => vec![(format!("shipped rho {:.2} blend {:.2}", args.rho, args.diffuse), args.rho, args.diffuse, args.deposit)],
    };
    let single = rows.len() == 1;
    println!("{:<18} {:>5} {:>7}  {:>14}  {:>14}", "arm", "peak", "drive", "trail gone", "stops steering");
    let mut traces = Vec::new();
    for (label, rho, diffuse, deposit) in rows {
        let l = measure_life(args, rho, diffuse, deposit);
        println!("{:<18} {:>5} {:>7.3}  {:>14}  {:>14}", label, l.peak0, l.drive0, fmt(l.frames_to_zero), fmt(l.frames_to_unusable));
        traces.push((label, l.trace));
    }
    println!();
    println!("`trail gone` is `plane.max() == 0`. `stops steering` is the frame the ant's own run");
    println!("drive falls under {DRIVE_FLOOR} -- about a tenth of its Move baseline. The trail is");
    println!("still on the plane after that; it has stopped being worth reading.");
    if single {
        let (label, trace) = &traces[0];
        println!();
        println!("the curve, so the crossing above is not the only readout ({label}):");
        println!("{:>7} {:>7} {:>7} {:>8}", "frame", "peak", "cells", "drive");
        for (f, peak, cells, drive) in trace.iter().take(14) {
            println!("{f:>7} {peak:>7} {cells:>7} {drive:>8.3}");
        }
    }
}

/// **The other half of the answer**, and the one that decides whether any
/// of this is a problem. A trail nobody walks *should* fade; the question
/// is what it costs to hold one up. So: re-lay one cell every `n` passes
/// and report the level it settles at.
fn traffic(args: &Args) {
    println!("how often must an ant re-lay a cell to hold it up?");
    println!("(one deposit of {} every N passes, level after 3 round trips)", args.deposit);
    println!();
    println!("{:>8}  {:>10}  {:>8}  {:>8}  verdict", "every", "frames", "holds at", "drive");
    for every in [1u64, 2, 3, 5, 8, 10, 20, 50] {
        let mut p = Pheromones::new(bounds());
        p.set_channel_rho(Channel::A, args.rho);
        p.set_channel_diffuse(Channel::A, args.diffuse);
        let mut passes = 0u64;
        for frame in 1..=(ROUND_TRIP * 3) {
            if frame.is_multiple_of(args.interval) {
                // **Re-lay the whole line, not one cell.** An isolated cell
                // blends against a 3x3 mean of `v/9` and a cell inside a
                // line against `v/3`, so a single-cell probe overstates the
                // loss by a third and would price traffic pessimistically.
                // The geometry an ant actually leaves is the line.
                if passes.is_multiple_of(every) {
                    lay(&mut p, args);
                }
                passes += 1;
            }
            p.step(frame, args.interval);
        }
        let mid = TRAIL_X0 + args.len / 2;
        let held = p.sample(Channel::A, mid, TRAIL_Y);
        let drive = run_drive(along_at(&p, mid)).abs();
        let verdict = if held == 0 {
            "gone"
        } else if drive < DRIVE_FLOOR {
            "there, but not steering"
        } else {
            "followable"
        };
        println!("{every:>8}  {:>10}  {held:>8}  {drive:>8.3}  {verdict}", every * args.interval);
    }
    println!();
    println!("A pass over this cell every N frames sustains it at the level in column three.");
    println!("`drive` is the same effect counter as above: what that level puts into Move.");
}

/// **The same question asked of the real bed, because an isolated harness
/// overstates what the app will see** (`CLAUDE.md`).
///
/// Everything above runs `Pheromones` on its own with a hand-laid trail.
/// That is the right shape for "how long does a deposit last", and it is
/// not evidence about a colony: real ants lay continuously, in a crowd, on
/// a route they re-walk. So this builds the standard lab bed, runs it
/// through `frame::step` -- the tick both binaries share -- and reports
/// what is actually standing on the two trail planes.
///
/// **The numbers to read together** are `deposits` (the "it fired" half,
/// from `PheromoneStats`) and `standing`/`peak` (the effect half, from the
/// planes themselves). A large deposit count against a near-empty plane is
/// the finding: the colony is laying, and it is not accumulating.
fn world(args: &Args) {
    let frames: u64 = std::env::args().find_map(|a| a.strip_prefix("frames=").and_then(|v| v.parse().ok())).unwrap_or(9_000);
    let seed: u64 = std::env::args().find_map(|a| a.strip_prefix("seed=").and_then(|v| v.parse().ok())).unwrap_or(1);
    let species = std::env::args().find_map(|a| a.strip_prefix("colonyspecies=").map(str::to_string)).unwrap_or_else(|| "ant".into());
    // **`founders=` is the control for who is writing the alarm plane.**
    // A plant is an organism, so the grazing path can raise an alarm; a bed
    // with no plants in it is the arm where only an animal can. Without
    // this the alarm deposit count is a sum over two writers with no way to
    // separate them -- `CLAUDE.md`'s "ask what your number counts".
    let founders: usize = std::env::args().find_map(|a| a.strip_prefix("founders=").and_then(|v| v.parse().ok())).unwrap_or(8);
    let colonies: usize = std::env::args().find_map(|a| a.strip_prefix("colonies=").and_then(|v| v.parse().ok())).unwrap_or(1);
    println!("the real bed: frames={frames} seed={seed} colonyspecies={species} founders={founders} colonies={colonies}");
    println!("(`standing` counts cells > 0 on the plane; `peak` is the tallest cell anywhere.)");
    println!();
    let spec = LabBox { width: 512, height: 320, soil_depth: 80, founders, colonies, compartments: 1, seed, colony_species: species, ..LabBox::default() };
    let mut lab = Lab::new(spec);
    // **`ants` is the control, and without it this table cannot be read.**
    // A standing-trail count falling to zero has two causes that look
    // identical here -- the trail decayed between visits, or the colony
    // that was laying it died -- and they want opposite conclusions. The
    // deposit counters are cumulative, so a colony that has stopped laying
    // shows as a *flat* deposit column, not a falling one; read the two
    // together.
    println!("{:>7}  {:>5}  {:>9} {:>7} {:>6}  {:>9} {:>7} {:>6}", "frame", "ants", "A deposits", "A cells", "A peak", "B deposits", "B cells", "B peak");
    let report = |lab: &Lab, frame: u64| {
        let p = &lab.world.pheromones;
        let (a, b) = (p.plane(Channel::A), p.plane(Channel::B));
        println!(
            "{frame:>7}  {:>5}  {:>9} {:>7} {:>6}  {:>9} {:>7} {:>6}",
            lab.world.live_creature_count(),
            p.stats.deposits_a,
            standing(a),
            a.max(),
            p.stats.deposits_b,
            standing(b),
            b.max()
        );
    };
    report(&lab, 0);
    for frame in 1..=frames {
        pixel_physics::sim::frame::step(
            &mut lab.world,
            &mut lab.particles,
            &mut lab.blasts,
            pixel_physics::sim::player::PlayerInput::default(),
            &pixel_physics::sim::player::Tuning::default(),
        );
        if frame.is_multiple_of(frames / 9) {
            report(&lab, frame);
        }
    }
    let p = &lab.world.pheromones;
    println!();
    println!("alarm plane: {} | alarm deposits {}", if p.alarm_is_live() { "live" } else { "NEVER WRITTEN" }, p.stats.deposits_alarm);
    println!("passes {} | tiles processed {}", p.stats.passes, p.stats.tiles_processed);
    println!();
    println!("A colony that lays a lot and stands up little is the trail dying between visits;");
    println!("`pherolife` (no mode=) is the isolated measurement of how long one deposit lasts.");
    let _ = args;
}

/// Cells holding anything on a plane. Walks it, so this is for the stops
/// and not for a per-frame readout.
fn standing(p: &pixel_physics::sim::pheromone::PheromonePlane) -> usize {
    let b = bounds();
    let mut n = 0;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            n += usize::from(p.sample(x, y) > 0);
        }
    }
    n
}

/// **How far does a scream carry, and for how long?**
///
/// The third plane's version of the same question, and it has a sharper
/// edge than the trail's because of where the input is read.
/// `BrainInput::Alarm` is sampled at the cell the animal **stands on**,
/// not at its forward sensor -- `creature::sense` argues that at length and
/// the measurement behind it is real (reading ahead gave 8/38/28 attacks,
/// reading here 296/258/266). What that doc also says, and names as a cost
/// rather than a win, is that a here-read *"carries no direction at
/// all"*.
///
/// So the question this answers is not "can an ant tell which way the
/// fight is" -- that is already known to be no. It is the one underneath:
/// **does an ant standing next to a fight read anything at all?** A single
/// cell's deposit has to spread before any neighbour can sample it, and it
/// is decaying at `ALARM_RHO` (0.25, six times the trail rate) while it
/// spreads. Both of the plane's writers are measured here: a wound
/// (`ALARM_DEPOSIT`, 240) and a display (`contest::DISPLAY_DEPOSIT`, 40),
/// which is the site `contest.rs`'s own doc flags as never measured.
fn alarm(args: &Args) {
    println!("how far does an alarm carry, and for how long?");
    println!("(`input` is what BrainInput::Alarm reads: value/255. `->Attack` is that");
    println!(" times ant.ron's authored `(Alarm, Attack, 2.0)`.)");
    for (label, deposit) in [("a wound  (ALARM_DEPOSIT 240)", pheromone::ALARM_DEPOSIT), ("a display (DISPLAY_DEPOSIT 40)", 40u8)] {
        println!();
        println!("{label}");
        println!("{:>7}  {:>6} {:>6} {:>6} {:>6} {:>6}", "frame", "d=0", "d=1", "d=2", "d=4", "d=8");
        let mut p = Pheromones::new(bounds());
        p.set_alarm_rho(pheromone::ALARM_RHO);
        p.deposit(Channel::Alarm, TRAIL_X0, TRAIL_Y, deposit);
        let sample = |p: &Pheromones| -> Vec<u8> { [0, 1, 2, 4, 8].iter().map(|d| p.sample(Channel::Alarm, TRAIL_X0 + d, TRAIL_Y)).collect() };
        let mut rows = vec![(0u64, sample(&p))];
        for frame in 1..=(args.interval * 14) {
            p.step(frame, args.interval);
            if frame.is_multiple_of(args.interval) {
                rows.push((frame, sample(&p)));
            }
        }
        for (f, v) in rows.iter().take(9) {
            print!("{f:>7} ");
            for x in v {
                print!(" {x:>6}");
            }
            println!();
        }
        let peak_at_1 = rows.iter().map(|(_, v)| v[1]).max().unwrap_or(0);
        let peak_at_2 = rows.iter().map(|(_, v)| v[2]).max().unwrap_or(0);
        println!("  loudest a neighbour one cell away ever hears: {peak_at_1}  (input {:.3}, ->Attack {:+.3})", f32::from(peak_at_1) / 255.0, 2.0 * f32::from(peak_at_1) / 255.0);
        println!("  ...and two cells away:                        {peak_at_2}  (input {:.3}, ->Attack {:+.3})", f32::from(peak_at_2) / 255.0, 2.0 * f32::from(peak_at_2) / 255.0);
    }

    // **The sustained arm, and it is here because a one-bite scene could
    // be the wrong scene.** A single deposit reaching nobody would read
    // identically whether the plane is quiet or the probe is looking at a
    // situation that never happens -- `CLAUDE.md`'s "a scene that
    // contradicts the code will look like a bug in the code". A real fight
    // is a bite every creature tick (`ant.ron`: `tick_interval: 6`) for as
    // long as it lasts, and an ant is a two-cell chain, so the honest
    // scene deposits repeatedly over two adjacent cells.
    println!();
    println!("a SUSTAINED fight -- a wound every 6 frames over a 2-cell body, 40 bites");
    let mut p = Pheromones::new(bounds());
    p.set_alarm_rho(pheromone::ALARM_RHO);
    let mut bites = 0u32;
    println!("{:>7}  {:>6} {:>6} {:>6} {:>6} {:>6}  {:>6}", "frame", "d=0", "d=1", "d=2", "d=4", "d=8", "bites");
    for frame in 1..=(args.interval * 22) {
        if frame.is_multiple_of(6) && bites < 40 {
            p.deposit(Channel::Alarm, TRAIL_X0, TRAIL_Y, pheromone::ALARM_DEPOSIT);
            p.deposit(Channel::Alarm, TRAIL_X0 + 1, TRAIL_Y, pheromone::ALARM_DEPOSIT);
            bites += 1;
        }
        p.step(frame, args.interval);
        if frame.is_multiple_of(args.interval * 2) {
            print!("{frame:>7} ");
            for d in [0, 1, 2, 4, 8] {
                print!(" {:>6}", p.sample(Channel::Alarm, TRAIL_X0 + d, TRAIL_Y));
            }
            println!("  {bites:>6}");
        }
    }
    let audible = |d: i32| f32::from(p.sample(Channel::Alarm, TRAIL_X0 + d, TRAIL_Y)) / 255.0;
    println!("  even sustained, an ant 2 cells off reads {:.3} -> Attack {:+.3}", audible(2), 2.0 * audible(2));

    println!();
    println!("An animal reads the alarm at the cell it STANDS ON (creature::sense), so the");
    println!("d=1 row is what the ant beside the fight hears -- not the d=0 one, which is");
    println!("the victim itself.");
}

/// **The positive control, per `CLAUDE.md`.** Every number above is a
/// *null* — a trail that went away — and a null looks the same whether the
/// mechanism decayed it or the probe never reached it. So construct the
/// case whose answer is known to be non-zero and check the instrument
/// reports it:
///
/// * a trail re-laid every single pass must NOT die (sensitivity: the
///   instrument can report a living trail);
/// * a plane nobody wrote reads zero at frame 0 (specificity);
/// * `rho = 0` must still kill the trail, because `build_decay_lut`'s
///   `min(v - 1)` floor is the real ceiling and no decay rate escapes it.
///   That last one is the claim the whole question rests on.
fn selftest(args: &Args) {
    println!("pherolife selftest");
    let mut fail = 0;

    // 1. specificity -- an unwritten plane is zero.
    let p = Pheromones::new(bounds());
    let m = p.plane(Channel::A).max();
    println!("  unwritten plane max ............ {m} (want 0)");
    fail += usize::from(m != 0);

    // 2. sensitivity -- a continuously re-laid trail survives the budget.
    let mut p = Pheromones::new(bounds());
    for frame in 1..=(ROUND_TRIP * 3) {
        if frame.is_multiple_of(args.interval) {
            p.deposit(Channel::A, TRAIL_X0, TRAIL_Y, pheromone::DEPOSIT);
        }
        p.step(frame, args.interval);
    }
    let held = p.sample(Channel::A, TRAIL_X0, TRAIL_Y);
    println!("  re-laid every pass, held at .... {held} (want > 0)");
    fail += usize::from(held == 0);

    // 3. the floor -- rho = 0 still empties the plane.
    let mut p = Pheromones::new(bounds());
    p.set_channel_rho(Channel::A, 0.0);
    p.deposit(Channel::A, TRAIL_X0, TRAIL_Y, 255);
    let mut frame = 0u64;
    let mut gone = 0u64;
    while frame < ROUND_TRIP * 10 {
        frame += 1;
        p.step(frame, args.interval);
        if p.plane(Channel::A).max() == 0 {
            gone = frame;
            break;
        }
    }
    println!("  rho=0, deposit 255, gone at .... {gone} frames (want > 0: the LUT floor, not rho)");
    fail += usize::from(gone == 0);

    // 4. the effect counter moves -- a ramp reads a drive, a flat trail
    //    reads none. If this is equal the scene does not contain the
    //    quantity and every number above is about nothing.
    let mut ramp = Pheromones::new(bounds());
    lay(&mut ramp, &Args { scene: "ramp".into(), ..clone_args(args) });
    let mut flat = Pheromones::new(bounds());
    lay(&mut flat, &Args { scene: "flat".into(), ..clone_args(args) });
    let mid = TRAIL_X0 + args.len / 2;
    let (dr, df) = (run_drive(along_at(&ramp, mid)).abs(), run_drive(along_at(&flat, mid)).abs());
    println!("  run drive: ramp {dr:.3} vs flat {df:.3} (want ramp > flat, or the scene is empty of the quantity)");
    fail += usize::from(dr <= df);

    println!();
    println!("{}", if fail == 0 { "selftest: PASS" } else { "selftest: FAIL" });
    if fail > 0 {
        std::process::exit(1);
    }
}

fn clone_args(a: &Args) -> Args {
    Args {
        mode: a.mode.clone(),
        sweep: a.sweep.clone(),
        deposit: a.deposit,
        rho: a.rho,
        diffuse: a.diffuse,
        interval: a.interval,
        len: a.len,
        scene: a.scene.clone(),
        selftest: a.selftest,
    }
}
