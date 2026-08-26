//! Where a frame's time actually goes, phase by phase, on the world the app ships.
//!
//! `examples/ascii` reports a *worst frame* per scene, which answers "does
//! this fit in 16.6 ms" and nothing else. It cannot say whether a 60 ms
//! frame was the CA sweep, the field grid, or the renderer — and guessing
//! wrong about that is how a session spends an afternoon optimising the
//! wrong phase. This runs the exact phase list `App::update` runs, times
//! each one separately, and prints a distribution rather than a single
//! sample (`CLAUDE.md`: outcomes here have enormous spread, so a max on its
//! own is a sample from a wide distribution).
//!
//! ```text
//! cargo run --release --example frame_profile
//! cargo run --release --example frame_profile -- frames=600
//! ```
//!
//! **The phase list is duplicated from `App::update`, not called through
//! it.** `update` is one opaque call and the whole question here is which
//! part of it costs what. If a phase is added there and not here, this
//! harness silently stops measuring it — check the two against each other
//! before trusting a total.
//!
//! Warm-up frames are discarded. The first frames of any scene in this repo
//! are dominated by allocation and first-touch page faults, which has
//! already produced one entirely bogus 73-ms-against-30-ms report (see
//! `sim/surface.rs`, `field_wind_at`).

use pixel_physics::app::{App, HEIGHT, WIDTH};
use pixel_physics::sim::{parallel, player, rigid};

const PHASES: [&str; 10] = [
    "ca_sweep",
    "liquid_bodies",
    "chunk_bodies",
    "player",
    "active_sites",
    "blasts",
    "particles",
    "fields",
    "pheromones",
    "draw",
];

struct Samples {
    /// One row per phase, one column per measured frame, milliseconds.
    rows: Vec<Vec<f64>>,
}

impl Samples {
    fn new() -> Self {
        Self { rows: vec![Vec::new(); PHASES.len()] }
    }

    fn report(&mut self, label: &str, note: &str) {
        println!("\n=== {label} ===");
        println!("{note}");
        println!("{:>14}  {:>9}  {:>9}  {:>9}  {:>9}  {:>7}", "phase", "mean", "p50", "p99", "max", "share");
        let total_mean: f64 = self.rows.iter().map(|r| mean(r)).sum();
        for (i, name) in PHASES.iter().enumerate() {
            let row = &mut self.rows[i];
            row.sort_by(|a, b| a.partial_cmp(b).expect("no NaN timings"));
            let m = mean(row);
            println!(
                "{:>14}  {:>7.3}ms  {:>7.3}ms  {:>7.3}ms  {:>7.3}ms  {:>6.1}%",
                name,
                m,
                pct(row, 0.50),
                pct(row, 0.99),
                row.last().copied().unwrap_or(0.0),
                if total_mean > 0.0 { m / total_mean * 100.0 } else { 0.0 },
            );
        }
        // Summed per frame, not per phase: the worst *frame* is what has to
        // fit the budget, and the phases do not peak together.
        let frames = self.rows[0].len();
        let mut totals: Vec<f64> = (0..frames).map(|f| self.rows.iter().map(|r| r[f]).sum()).collect();
        totals.sort_by(|a, b| a.partial_cmp(b).expect("no NaN timings"));
        println!(
            "{:>14}  {:>7.3}ms  {:>7.3}ms  {:>7.3}ms  {:>7.3}ms   (16.6 ms budget)",
            "FRAME",
            mean(&totals),
            pct(&totals, 0.50),
            pct(&totals, 0.99),
            totals.last().copied().unwrap_or(0.0),
        );
        let over = totals.iter().filter(|&&t| t > 16.6).count();
        println!("  {over}/{frames} frames over budget ({:.0}% of the run)", over as f64 * 100.0 / frames.max(1) as f64);
    }
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.iter().sum::<f64>() / v.len() as f64
}

/// `v` must already be sorted.
fn pct(v: &[f64], p: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v[((v.len() as f64 - 1.0) * p).round() as usize]
}

/// One tick of `App::update`, split into the phases it runs, plus the draw.
/// See the module doc on why this duplicates rather than calls `update`.
fn tick(app: &mut App, frame: &mut [u8], s: &mut Samples, record: bool) {
    // Timestamps first, differences after. The obvious form -- a `lap!` macro
    // that overwrites a running `Instant` -- leaves the final expansion
    // assigning a value nothing reads, which is a `-D warnings` clippy
    // failure (`unused_assignments`) in a repo whose CI gates on it. It also
    // broke a concurrent session's clippy run on code they had not touched.
    // `Instant` is `Copy`, so an array of marks costs nothing and has no
    // dead store in it.
    let start = std::time::Instant::now();
    let mut marks = [start; PHASES.len()];

    parallel::step(&mut app.world);
    marks[0] = std::time::Instant::now();
    app.world.step_liquid_bodies();
    marks[1] = std::time::Instant::now();
    rigid::step_chunk_bodies(&mut app.world);
    marks[2] = std::time::Instant::now();
    player::step(&mut app.world, app.player_input, &app.player_tuning);
    app.player_input.jump_pressed = false;
    marks[3] = std::time::Instant::now();
    app.world.step_active_sites();
    marks[4] = std::time::Instant::now();
    // `App` owns both, and `Blasts::step` needs `&mut` to each at once.
    let mut particles = std::mem::take(&mut app.particles);
    app.blasts.step(&mut app.world, &mut particles);
    marks[5] = std::time::Instant::now();
    particles.step(&mut app.world);
    app.particles = particles;
    marks[6] = std::time::Instant::now();
    app.world.step_fields();
    marks[7] = std::time::Instant::now();
    app.world.step_pheromones();
    marks[8] = std::time::Instant::now();
    app.draw(frame, None);
    marks[9] = std::time::Instant::now();

    if record {
        let mut previous = start;
        for (row, mark) in s.rows.iter_mut().zip(marks) {
            row.push(mark.duration_since(previous).as_secs_f64() * 1000.0);
            previous = mark;
        }
    }
}

fn run(
    label: &str,
    note: &str,
    warmup: usize,
    frames: usize,
    setup: impl FnOnce(&mut App),
    mut each: impl FnMut(&mut App, usize),
) {
    let mut app = App::new();
    setup(&mut app);
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let mut s = Samples::new();
    for i in 0..warmup {
        each(&mut app, i);
        tick(&mut app, &mut frame, &mut s, false);
    }
    for i in 0..frames {
        each(&mut app, warmup + i);
        tick(&mut app, &mut frame, &mut s, true);
    }
    s.report(label, note);
    println!("  world: {} chunks, {} awake at the end", app.world.chunk_count(), app.world.active_chunk_count());
}

fn main() {
    let mut frames = 300usize;
    let mut warmup = 120usize;
    for arg in std::env::args().skip(1) {
        if let Some(v) = arg.strip_prefix("frames=") {
            frames = v.parse().expect("frames=N");
        }
        if let Some(v) = arg.strip_prefix("warmup=") {
            warmup = v.parse().expect("warmup=N");
        }
    }

    // The baseline nobody plays but everything is measured against: a
    // generated world left to settle, nothing on screen moving.
    run(
        "settled world, no input",
        "The floor. Anything above zero here is being paid for nothing.",
        warmup,
        frames,
        |_| {},
        |_, _| {},
    );

    // Same gnome, standing still: isolates the camera from everything else
    // the character costs (`CLAUDE.md`: prefer a paired comparison).
    run(
        "the gnome standing still",
        "The control for the walk below: same character, same phases, camera parked.",
        warmup,
        frames,
        |app| {
            app.summon_player(WIDTH as i32 / 2, HEIGHT as i32 / 2);
        },
        |_, _| {},
    );

    // What the player actually does. The camera follows the gnome, and a
    // camera move invalidates every pixel — so this is the case the
    // dirty-rect skip does not cover.
    run(
        "the gnome walking (camera panning)",
        "Camera moves, so `Renderer::draw` takes its full-redraw branch every frame.",
        warmup,
        frames,
        |app| {
            app.summon_player(WIDTH as i32 / 2, HEIGHT as i32 / 2);
        },
        |app, _| {
            app.player_input.right = true;
        },
    );

    // A blast every 60 frames, which is roughly as fast as anyone can
    // trigger one, into ground that has settled.
    run(
        "digging: a blast every 60 frames",
        "The destructive case — rubble, chunk bodies and debris all live at once.",
        warmup,
        frames,
        |_| {},
        |app, i| {
            if i.is_multiple_of(60) {
                app.explode(WIDTH as i32 / 2, HEIGHT as i32 * 3 / 4);
            }
        },
    );

    // The same world with the renderer's full-redraw triggers absent. Rain
    // was falling for 99% of every run above and forces a full repaint on its
    // own, so nothing else about the draw could be attributed while it was.
    run(
        "a DRY seed, nothing forcing a full redraw",
        "The control for the draw cost: same world, same phases, dirty-rect path actually in use.",
        warmup,
        frames,
        |app| {
            for candidate in 0..400u64 {
                app.worldgen_seed = candidate;
                app.reset();
                if (0..(warmup + frames + 100) as u64).all(|f| !pixel_physics::sim::weather::at(app.world.seed, f).is_precipitating()) {
                    break;
                }
            }
        },
        |_, _| {},
    );

    worldgen_cost();
}

/// `F6` and `F8` rebuild the whole world, and the owner rerolls seeds
/// constantly by design (`App::next_seed`'s own doc). A reroll that hitches
/// is a reroll that does not get done.
fn worldgen_cost() {
    println!("\n=== worldgen: the cost of one reroll (F6) ===");
    let mut app = App::new();
    let mut times = Vec::new();
    for _ in 0..8 {
        let t = std::time::Instant::now();
        app.next_seed();
        times.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(|a, b| a.partial_cmp(b).expect("no NaN timings"));
    println!(
        "  {} rerolls at {}x{}: median {:.1} ms, worst {:.1} ms",
        times.len(),
        pixel_physics::app::WORLD_WIDTH,
        pixel_physics::app::WORLD_HEIGHT,
        pct(&times, 0.5),
        times.last().copied().unwrap_or(0.0),
    );
}
