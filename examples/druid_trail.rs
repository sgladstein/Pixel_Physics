//! **How long the gnome's scent trail survives, in seconds and in ant-cells
//! walked** — and how wide a cloud it makes while it is there.
//!
//! Owner, 2026-09-14 playtest: *"my pheramone trail should last way longer.
//! it dissapears so much faster than an ant could even move. I wonder if this
//! is a problem for ant laid trails too. The animation can be improved too.
//! it should be more diffuse looking, not like a bunch of dots."*
//!
//! **Two complaints, and this measures whether they are one thing.** Lifetime
//! and width both come off the same plane: `pheromone::build_decay_lut`
//! forces every nonzero value strictly downward, so a mark's life in *passes*
//! is bounded by the value it was laid at, and `pheromone::DIFFUSE` blends on
//! a u8, so a mark laid small rounds to zero a cell or two out and never
//! becomes a cloud at all. A trail that is too faint is therefore
//! short-lived *and* dotty for one reason, which is the claim this binary
//! exists to check rather than assert.
//!
//! **Why the plane and not the app.** The question is what the *field* does
//! after the gnome stops walking, which is `Pheromones::step` and nothing
//! else — no world, no seed, no chaos, so an arm that reads badly here cannot
//! read well in a bed. `trailfollow` is the instrument for the other half
//! (*does a laid trail actually move a colony*) and this deliberately does
//! not duplicate it. The picture of the real app is `druid_shot.sh`.
//!
//! **The unit that answers the complaint is not seconds.** "Faster than an
//! ant could even move" is a distance, so every lifetime here is also
//! printed in **ant-cells**: an ant decides every `creature::tick_interval`
//! = 6 frames and moves a cell per decision, so 10 cells a second, and a
//! colony round trip is roughly 2,200 frames (`pheromone::PHEROMONE_INTERVAL`
//! cites it) ≈ 366 cells.
//!
//! ```text
//! cargo run --release --example druid_trail                       # every arm
//! cargo run --release --example druid_trail -- deposit=40 hold=0  # one arm
//! cargo run --release --example druid_trail -- selftest           # the controls
//! ```

use pixel_physics::sim::pheromone::{Channel, Pheromones, DECAY_RHO, DEPOSIT, DIFFUSE, PHEROMONE_INTERVAL};
use pixel_physics::sim::Rect;

/// The tick rate both games are driven at (`druid::TICKS_PER_SECOND`).
const FPS: f64 = 60.0;

/// **Cells an ant covers per second**, for the unit the complaint is in.
/// `creature::tick_interval`'s authored ant is 6 frames per decision and a
/// move is one cell, so 60 / 6.
const ANT_CELLS_PER_SECOND: f64 = 10.0;

/// A colony round trip, in frames — the figure `PHEROMONE_INTERVAL`'s own doc
/// sets the plane's ceiling against. The bar a trail has to clear to be worth
/// laying at all.
const ROUND_TRIP_FRAMES: f64 = 2200.0;

/// **Cells the gnome covers per tick**, `player::Tuning::default().run_max`.
///
/// Above one, which matters and is not what `Druid::lay_trail`'s doc assumed:
/// at 1.5 cells a tick he *skips* cells rather than marking each one two or
/// three times, so the laid plane is dotted before anything has decayed.
const GNOME_CELLS_PER_TICK: f64 = 1.5;

/// How far off the route the readout samples (`druid::hud::SCENT_HALO`), so
/// the "cells the HUD would draw" column is the one the player sees.
const HALO: i32 = 4;

#[derive(Clone)]
struct Arm {
    /// What one tick of `G` writes (`druid::TRAIL_DEPOSIT`).
    deposit: u8,
    /// Seconds the route keeps being re-laid after he stops walking. 0 is
    /// shipped behaviour: he marks a cell once and it is on its own.
    hold: f64,
    /// Fill the gap between this tick's cell and the last one, rather than
    /// marking the cell he happens to be standing in when the tick fires.
    /// Shipped is `false`, and at [`GNOME_CELLS_PER_TICK`] that leaves holes.
    join: bool,
    /// **Radius of the mark**, in cells. Shipped is 0 -- one cell, a
    /// pencil line one cell wide.
    ///
    /// The lever this binary exists to find. `DIFFUSE` blends a cell 0.25 of
    /// the way to its own 3x3 mean, and for a cell on a one-cell-wide line
    /// that mean is `v/3` -- so a line loses about 17% a pass to spreading
    /// against `DECAY_RHO`'s 3% to forgetting. Diffusion is five times the
    /// decay term, and it is the term nothing in the module doc's ceiling
    /// story is about. A cell in the middle of a *band* has a 3x3 mean of
    /// roughly its own value and loses almost nothing.
    radius: i32,
}

/// What one arm did.
struct Life {
    /// Peak value anywhere on the route, per pass, from the moment laying
    /// stops.
    ///
    /// **The whole curve, because the one number this started as was an
    /// occupancy metric wearing a lifetime's clothes.** "Passes until every
    /// route cell reads exactly 0" scored the shipped trail at 120 passes =
    /// 24 seconds, which is not a trail anybody could call short -- and at
    /// that moment its two ends already read 0 and its peak was 1. A `u8`
    /// plane at 1 is drawn (the readout only drops exact zeroes) and is
    /// unfollowable (an ant reads the *gradient*), so the count was measuring
    /// how long the last ember takes to round down. `CLAUDE.md`'s own metric
    /// trap, on a plane instead of a liquid: measure fill, not occupancy.
    curve: Vec<u8>,
    /// Peak value anywhere on the route, one pass after the walk ends.
    peak: u8,
    /// Cells of the route still nonzero one pass after the walk ends, against
    /// the route's length — the "bunch of dots" number at full strength.
    laid_of: (usize, usize),
    /// Cells the HUD would draw, at its widest.
    drawn: usize,
    /// Half-width of the cloud in cells at its widest: how far off the route
    /// the plane still reads nonzero.
    width: i32,
    /// Value at the oldest cell **he actually marked** against the newest,
    /// at the moment laying stops. `Druid::lay_trail` rests on this being a
    /// slope and not a flat.
    ///
    /// **Over the marked cells, not the route, and the difference was a live
    /// instrument bug.** Read over the route it sampled `x0 + cells - 1` --
    /// which at a stride above one the gnome never steps on -- so the column
    /// reported the newest end *weaker* than the oldest, the exact reverse of
    /// the property it exists to check, off a cell that only ever held a
    /// diffusion leak from its neighbour.
    slope: (u8, u8),
}

/// **The bar the lifetime column is read against**, in plane units.
///
/// A quarter of `DEPOSIT`, and the quarter is doing two jobs. Below it the
/// readout has nothing left to say -- `hud::SCENT_BANDS` quantises `v * 8 /
/// 256`, so everything under 32 draws in the bottom band -- and below it one
/// ant walking past deposits more than the gnome's standing instruction
/// holds, so what he wrote stops outranking the colony's own traffic.
const LEGIBLE: u8 = DEPOSIT / 4;

impl Life {
    /// Peak on the route `s` seconds after laying stopped. Saturates at the
    /// end of the curve, which is zero by construction.
    fn at_seconds(&self, s: f64) -> u8 {
        let pass = (s * FPS / PHEROMONE_INTERVAL as f64).round() as usize;
        self.curve.get(pass).copied().unwrap_or(0)
    }

    /// Passes the peak holds at or above [`LEGIBLE`].
    fn legible_passes(&self) -> usize {
        self.curve.iter().take_while(|&&v| v >= LEGIBLE).count()
    }
}

fn legible_seconds(l: &Life) -> f64 {
    l.legible_passes() as f64 * PHEROMONE_INTERVAL as f64 / FPS
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "selftest") {
        selftest();
        return;
    }
    if args.iter().any(|a| a.starts_with("shot=")) {
        shot(&args);
        return;
    }
    let get = |k: &str| -> Option<String> { args.iter().find_map(|a| a.strip_prefix(&format!("{k}=")).map(str::to_string)) };
    let walk_cells: i32 = get("cells").and_then(|v| v.parse().ok()).unwrap_or(200);

    // **Echoes its own parameters**, per `CLAUDE.md`: a log that does not name
    // its arguments was written by a binary that never had them.
    println!(
        "druid_trail: cells={walk_cells} deposit(shipped)={DEPOSIT} diffuse={DIFFUSE} rho={DECAY_RHO} \
         interval={PHEROMONE_INTERVAL} gnome={GNOME_CELLS_PER_TICK} cells/tick ant={ANT_CELLS_PER_SECOND} cells/s"
    );
    println!();

    let arms: Vec<(String, Arm)> = match (get("deposit"), get("hold")) {
        (None, None) => vec![
            ("shipped (one cell, deposit 40)".into(), Arm { deposit: DEPOSIT, hold: 0.0, join: false, radius: 0 }),
            ("deposit 120, still one cell".into(), Arm { deposit: 120, hold: 0.0, join: false, radius: 0 }),
            ("deposit 255, still one cell".into(), Arm { deposit: 255, hold: 0.0, join: false, radius: 0 }),
            ("band r=2, deposit 40".into(), Arm { deposit: DEPOSIT, hold: 0.0, join: false, radius: 2 }),
            ("band r=3, deposit 40".into(), Arm { deposit: DEPOSIT, hold: 0.0, join: false, radius: 3 }),
            ("band r=3, deposit 120".into(), Arm { deposit: 120, hold: 0.0, join: false, radius: 3 }),
            ("band r=4, deposit 120".into(), Arm { deposit: 120, hold: 0.0, join: false, radius: 4 }),
            ("band r=4, deposit 200".into(), Arm { deposit: 200, hold: 0.0, join: false, radius: 4 }),
            ("band r=5, deposit 200".into(), Arm { deposit: 200, hold: 0.0, join: false, radius: 5 }),
        ],
        (d, h) => {
            let d = d.and_then(|v| v.parse().ok()).unwrap_or(DEPOSIT);
            let h = h.and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let j = get("join").map(|v| v != "0").unwrap_or(false);
            let r = get("radius").and_then(|v| v.parse().ok()).unwrap_or(0);
            vec![(format!("deposit {d} hold {h}s join {j} radius {r}"), Arm { deposit: d, hold: h, join: j, radius: r })]
        }
    };

    println!("peak value on the route, after he stops laying:");
    println!(
        "{:<30} {:>5} {:>5} {:>5} {:>5} {:>5} {:>8} {:>8} {:>10} {:>9} {:>6} {:>9}",
        "arm", "0s", "5s", "10s", "20s", "40s", "legible", "seconds", "ant-cells", "laid/route", "width", "slope"
    );
    for (name, arm) in &arms {
        let l = run(arm, walk_cells);
        let secs = legible_seconds(&l);
        println!(
            "{:<30} {:>5} {:>5} {:>5} {:>5} {:>5} {:>8} {:>8.1} {:>10.0} {:>4}/{:<4} {:>6} {:>4}/{:<4}",
            name,
            l.at_seconds(0.0),
            l.at_seconds(5.0),
            l.at_seconds(10.0),
            l.at_seconds(20.0),
            l.at_seconds(40.0),
            l.legible_passes(),
            secs,
            secs * ANT_CELLS_PER_SECOND,
            l.laid_of.0,
            l.laid_of.1,
            l.width,
            l.slope.1,
            l.slope.0,
        );
        let _ = (l.peak, l.drawn);
    }
    println!();
    println!("legible:    passes the peak stays at or above {LEGIBLE} -- a quarter of one ant's fresh mark.");
    println!("            A bar, not a cliff: the curve is printed beside it so the shape is readable. Below it the");
    println!("            readout's brightest band is its bottom one and one passing ant outweighs the instruction.");
    println!("ant-cells:  that lifetime in the unit the complaint is in. A colony round trip is {:.0} of them.", ROUND_TRIP_FRAMES / FPS * ANT_CELLS_PER_SECOND);
    println!("laid/route: cells of the walked route the plane actually holds a mark in, one pass after");
    println!("            he stops -- the 'bunch of dots' before anything has decayed.");
    println!("width:      half-width in cells of the nonzero cloud at its widest, against a {HALO}-cell readout halo.");
    println!("slope:      newest end / oldest end when laying stops. `Druid::lay_trail` follows the slope, so");
    println!("            an arm reading n/n has nothing for an ant to walk up (`a_laid_trail_slopes_toward_the_newest_end`).");
}

/// Walk a straight route laying `arm.deposit` a tick exactly as
/// `Druid::lay_trail` does, optionally keep the route standing for
/// `arm.hold` seconds, then stop and watch it go.
fn run(arm: &Arm, walk_cells: i32) -> Life {
    let y = 96;
    let x0 = 20;
    let mut p = Pheromones::new(Rect::new(0, 0, 511, 191));
    let mut frame = 0u64;

    let ticks = (walk_cells as f64 / GNOME_CELLS_PER_TICK).round() as u64;
    let route: Vec<i32> = (x0..x0 + walk_cells).collect();
    // What the gnome's own memory holds: the cells he actually marked, in
    // order, which at 1.5 cells a tick is not every cell of the route.
    let mut marked: Vec<i32> = Vec::new();
    for t in 0..ticks {
        let x = x0 + (t as f64 * GNOME_CELLS_PER_TICK) as i32;
        // Shipped marks the cell he is standing in when the tick fires; the
        // `join` arm marks every cell between that and the last one.
        let from = if arm.join { marked.last().map_or(x, |&l| l + 1) } else { x };
        for cx in from.min(x)..=x {
            for dy in -arm.radius..=arm.radius {
                for dx in -arm.radius..=arm.radius {
                    if dx * dx + dy * dy > arm.radius * arm.radius {
                        continue;
                    }
                    p.deposit(Channel::A, cx + dx, y + dy, arm.deposit);
                }
            }
            if marked.last() != Some(&cx) {
                marked.push(cx);
            }
        }
        p.step(frame, PHEROMONE_INTERVAL);
        frame += 1;
    }

    // **The hold: the route is a standing instruction, so it keeps being
    // written.** Age-graded from the oldest end so the slope survives --
    // topping every cell up equally would flatten exactly the gradient the
    // mechanic is read off.
    let hold_ticks = (arm.hold * FPS) as u64;
    for t in 0..hold_ticks {
        let fade = 1.0 - t as f64 / hold_ticks.max(1) as f64;
        for (i, &x) in marked.iter().enumerate() {
            let age = 1.0 - i as f64 / marked.len().max(1) as f64;
            let amount = (arm.deposit as f64 * fade * (1.0 - 0.5 * age)) as u8;
            if amount > 0 {
                p.deposit(Channel::A, x, y, amount);
            }
        }
        p.step(frame, PHEROMONE_INTERVAL);
        frame += 1;
    }

    let at = |p: &Pheromones, x: i32, dy: i32| p.sample(Channel::A, x, y + dy);
    let peak = route.iter().map(|&x| at(&p, x, 0)).max().unwrap_or(0);
    let laid = route.iter().filter(|&&x| at(&p, x, 0) > 0).count();
    let slope = (at(&p, *marked.first().unwrap(), 0), at(&p, *marked.last().unwrap(), 0));

    let mut width = 0;
    let mut drawn = 0usize;
    for d in 0..=HALO {
        let n = route.iter().filter(|&&x| at(&p, x, d) > 0).count();
        if n > 0 {
            width = d;
        }
        drawn += n * if d == 0 { 1 } else { 2 };
    }

    // ...and now nothing reinforces it.
    //
    // **`frame` advances a whole interval per iteration, and it advanced by
    // one until it was caught.** `Pheromones::step` gates internally on
    // `frame % interval`, so a loop that ticks the counter by one runs a pass
    // every twelfth iteration -- and this curve was then read as though its
    // index were passes. It made the shipped trail look 12x longer-lived than
    // it is (a printed "12.0 seconds" that was 1.0), and the error was
    // invisible because the curve had exactly the shape a decay curve should.
    // The real app disagreed by an order of magnitude, which is the only
    // reason it surfaced: `CLAUDE.md`'s isolated-harness rule, with the
    // harness wrong rather than merely narrow.
    //
    // **And the counter has to be *aligned* as well as stepped by an
    // interval**, which the first repair was not: the walk leaves `frame` at
    // whatever tick it ended on, and advancing by 12 from a frame that is not
    // already a multiple of 12 never hits one again, so not a single pass
    // ran and every arm reported a flat curve at its laid value. Control B of
    // `selftest` is what caught it -- a curve that cannot fall is exactly as
    // plausible-looking as one that falls slowly.
    frame += PHEROMONE_INTERVAL - frame % PHEROMONE_INTERVAL;
    let mut curve = vec![peak];
    for _ in 1..2000usize {
        p.step(frame, PHEROMONE_INTERVAL);
        frame += PHEROMONE_INTERVAL;
        let v = route.iter().map(|&x| at(&p, x, 0)).max().unwrap_or(0);
        curve.push(v);
        if v == 0 {
            break;
        }
    }

    Life { curve, peak, laid_of: (laid, route.len()), drawn, width, slope }
}

/// **The positive control**, per `CLAUDE.md`: a number that cannot move is
/// indistinguishable from one that has nothing to say. Each row constructs a
/// case whose answer is known and checks the instrument reports it.
fn selftest() {
    println!("druid_trail selftest -- each row is a case whose answer is known before it runs");
    let mut bad = 0;

    // A: lifetime must rise with the deposit. The LUT floor makes life in
    // passes bounded by the value laid, so this is the instrument's whole
    // premise and it must be visible.
    let lo = run(&Arm { deposit: 8, hold: 0.0, join: false, radius: 0 }, 60).legible_passes();
    let hi = run(&Arm { deposit: 255, hold: 0.0, join: false, radius: 0 }, 60).legible_passes();
    let a = hi > lo + 10;
    println!("  A deposit 8 -> {lo} passes, deposit 255 -> {hi}: {}", if a { "ok" } else { bad += 1; "BLIND -- lifetime does not track the deposit" });

    // B: holding the route must outlast not holding it, by about the hold.
    let free = run(&Arm { deposit: DEPOSIT, hold: 0.0, join: false, radius: 0 }, 60).legible_passes();
    let held = run(&Arm { deposit: DEPOSIT, hold: 20.0, join: false, radius: 0 }, 60).legible_passes();
    // A ratio, not a margin in seconds: the margin this started as was
    // written against a clock that ran 12x slow and survived the repair
    // looking reasonable.
    let b = held > free * 3;
    println!("  B free -> {free} passes, held 20s -> {held}: {}", if b { "ok" } else { bad += 1; "BLIND -- the hold does not reach the plane" });

    // C: the width column must be able to read more than the spine, or the
    // "diffuse vs dots" half of this binary measures nothing. A standing
    // trail is the case known to spread (`DIFFUSE`'s own profile sweep).
    let thin = run(&Arm { deposit: 4, hold: 0.0, join: false, radius: 0 }, 60).width;
    let fat = run(&Arm { deposit: 255, hold: 0.0, join: false, radius: 4 }, 60).width;
    let c = fat > thin;
    println!("  C width at deposit 4 on a line -> {thin}, at 255 on an r=4 band -> {fat}: {}", if c { "ok" } else { bad += 1; "BLIND -- width cannot move" });

    // E: every unheld arm's curve must actually reach zero inside the loop's
    // cap. A curve pinned at its laid value is the failure two separate
    // repairs of this harness's frame counter produced, and it reads as a
    // very long-lived trail rather than as a broken clock.
    let ends_at = |a: Arm| *run(&a, 60).curve.last().unwrap();
    let e = ends_at(Arm { deposit: 255, hold: 0.0, join: true, radius: 0 }) == 0;
    println!("  E an unreinforced curve ends at {}: {}", ends_at(Arm { deposit: 255, hold: 0.0, join: true, radius: 0 }), if e { "ok" } else { bad += 1; "BLIND -- the plane is not decaying at all" });

    // D: the slope must be able to be flat, or the followability column
    // cannot warn about the thing it is here to warn about.
    let s = run(&Arm { deposit: 255, hold: 0.0, join: false, radius: 0 }, 20).slope;
    println!("  D slope at a saturating deposit reads {}/{} (oldest/newest)", s.0, s.1);

    println!("{}", if bad == 0 { "selftest: all controls fired" } else { "selftest: SOME CONTROLS ARE BLIND" });
}

/// **The picture**, driven through the real game loop so what is judged is
/// what the player sees.
///
/// `Druid::update` and `Druid::draw` are the calls `src/bin/druid.rs` makes,
/// and the HUD's scent layer is built inside `Druid::draw` — so nothing short
/// of the real loop renders it. No GPU and no window: the sandbox's
/// `pixels` surface is what needs a display, and `Druid::draw` writes into a
/// plain byte buffer.
///
/// `walk=` ticks of holding `D` and `G` together, then `after=` seconds of
/// standing off it, so the sheet shows the trail ageing rather than the
/// moment it was laid.
///
/// ```text
/// cargo run --release --example druid_trail -- shot=/tmp/t.png after=0,5,15,30
/// ```
fn shot(args: &[String]) {
    use pixel_physics::app::{HEIGHT, WIDTH};
    use pixel_physics::druid::{Druid, TICKS_PER_SECOND};

    let get = |k: &str| -> Option<String> { args.iter().find_map(|a| a.strip_prefix(&format!("{k}=")).map(str::to_string)) };
    let out = get("shot").unwrap_or_else(|| "/tmp/druid_trail.png".into());
    let walk: u64 = get("walk").and_then(|v| v.parse().ok()).unwrap_or(240);
    let stops: Vec<f64> = get("after")
        .unwrap_or_else(|| "0,5,15,30".into())
        .split(',')
        .filter_map(|v| v.trim().parse().ok())
        .collect();
    let zoom: u32 = get("zoom").and_then(|v| v.parse().ok()).unwrap_or(2);
    // `crop=x,y,w,h` in frame pixels, before the zoom. A 512x320 frame with a
    // trail in a 20-row band across it is a picture in which the thing being
    // judged is a few percent of the pixels.
    let crop: Option<(u32, u32, u32, u32)> = get("crop").and_then(|v| {
        let n: Vec<u32> = v.split(',').filter_map(|c| c.trim().parse().ok()).collect();
        (n.len() == 4).then(|| (n[0], n[1], n[2], n[3]))
    });

    println!("druid_trail shot: out={out} walk={walk} ticks after={stops:?}s zoom={zoom}");
    let mut game = Druid::new();
    game.unlimited = true;
    // The key legend covers the left two thirds of the frame and the trail
    // runs under it.
    game.show_keys = false;

    // Settle, so the sheet is not a picture of him falling.
    for _ in 0..30 {
        game.update();
    }
    let start = game.world.player.as_ref().map(|p| p.center());

    // **Walk and lay together**, in the order `src/bin/druid.rs` uses: the
    // mark goes down before the tick that moves him, or the first cell of the
    // route is the one he has already left.
    let mut marks = 0usize;
    for _ in 0..walk {
        game.player_input.right = true;
        if game.lay_trail() {
            marks += 1;
        }
        game.update();
    }
    game.player_input.right = false;
    let end = game.world.player.as_ref().map(|p| p.center());

    let mut shots = Vec::new();
    let mut elapsed = 0.0f64;
    for &s in &stops {
        let ticks = ((s - elapsed).max(0.0) * TICKS_PER_SECOND as f64) as u64;
        for _ in 0..ticks {
            game.update();
        }
        elapsed = s;
        let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        game.draw(&mut buf, (WIDTH, HEIGHT), true);

        // **The count beside the picture**, per `CLAUDE.md`: a sheet showing a
        // plausible green haze is exactly what a dead readout over a lit world
        // also looks like. Peak is read off the plane itself, `drawn` off the
        // readout, so a trail that is in the world and not on the screen —
        // or on the screen and not in the world — is separable.
        let (peak, live) = trail_census(&game);
        println!("  t+{s:>5.1}s  marks {marks}  route {}  plane peak {peak}  cells alive {live}  {}", game.trail.len(), geometry(&game));
        shots.push(present_rgba(&buf, WIDTH, HEIGHT, zoom, crop));
    }
    println!("  walked {:?} -> {:?}", start, end);
    write_column(&out, &shots);
    println!("  wrote {out}");
}

/// Peak value on the gnome's remembered route, and how many of its cells the
/// plane still holds anything at.
fn trail_census(game: &pixel_physics::druid::Druid) -> (u8, usize) {
    let mut peak = 0u8;
    let mut live = 0usize;
    for &(x, y) in &game.trail {
        let v = game.world.pheromone_at(game.scent, x, y);
        peak = peak.max(v);
        if v > 0 {
            live += 1;
        }
    }
    (peak, live)
}

/// **What shape the route actually is**, which is the question the clean-line
/// model in `run` cannot ask. Its straight horizontal run gives every mark two
/// same-line neighbours, so `DIFFUSE`'s 3x3 blend loses it almost nothing; a
/// real walk over real ground may not.
fn geometry(game: &pixel_physics::druid::Druid) -> String {
    let cells: Vec<(i32, i32)> = game.trail.iter().copied().collect();
    if cells.is_empty() {
        return "route empty".into();
    }
    let joined = cells.windows(2).filter(|w| (w[0].0 - w[1].0).abs() <= 1 && (w[0].1 - w[1].1).abs() <= 1).count();
    let ys: std::collections::HashSet<i32> = cells.iter().map(|c| c.1).collect();
    let xs: std::collections::HashSet<i32> = cells.iter().map(|c| c.0).collect();
    let span = cells.iter().map(|c| c.0).max().unwrap() - cells.iter().map(|c| c.0).min().unwrap() + 1;
    // **How far above the ground the mark sits.** An ant reads the plane
    // around its head as it walks the floor, so a trail laid at the gnome's
    // *centre* is laid at his chest -- half a body above anything that could
    // follow it. Measured rather than assumed: the drop to the first solid
    // cell under each mark.
    let mut clear: Vec<i32> = Vec::new();
    for &(x, y) in cells.iter().take(64) {
        let mut d = 0;
        while d < 40 && game.world.is_empty(x, y + d) {
            d += 1;
        }
        clear.push(d);
    }
    let median = {
        let mut c = clear.clone();
        c.sort_unstable();
        c.get(c.len() / 2).copied().unwrap_or(-1)
    };
    format!(
        "8-adjacent {joined}/{}  distinct x {} of span {span}  distinct y {}  median clearance to ground {median}",
        cells.len().saturating_sub(1),
        xs.len(),
        ys.len()
    )
}

fn present_rgba(buf: &[u8], w: u32, h: u32, zoom: u32, crop: Option<(u32, u32, u32, u32)>) -> image::RgbaImage {
    let full = image::RgbaImage::from_raw(w, h, buf.to_vec()).expect("frame");
    let cut = match crop {
        Some((x, y, cw, ch)) => image::imageops::crop_imm(&full, x, y, cw, ch).to_image(),
        None => full,
    };
    image::imageops::resize(&cut, cut.width() * zoom, cut.height() * zoom, image::imageops::FilterType::Nearest)
}

fn write_column(out: &str, shots: &[image::RgbaImage]) {
    let (w, h) = (shots[0].width(), shots[0].height());
    let mut sheet = image::RgbaImage::new(w, h * shots.len() as u32);
    for (i, img) in shots.iter().enumerate() {
        image::imageops::replace(&mut sheet, img, 0, (i as u32 * h) as i64);
    }
    sheet.save(out).expect("write sheet");
}
