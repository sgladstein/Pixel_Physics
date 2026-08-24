//! The load-independent half of the performance picture: **how much work is
//! being asked for**, not how long it took.
//!
//! Wall-clock on this machine is only trustworthy when nothing else is
//! running on it, and this tree is worked in concurrently — a timing run
//! taken while another session's harness had the cores read 45 ms on a
//! settled world. Counts do not care. Every number here is a pure function
//! of the simulation, so it reproduces under any load, and it is the number
//! an optimisation actually has to move:
//!
//! - **awake chunks** gates `field::step`'s world-global early-out. One
//!   awake chunk anywhere means the full multi-pass field solve runs over
//!   *every* resident chunk (`PLAN.md` issue #4). So "does the world ever
//!   settle" is worth an order of magnitude, not a rounding error.
//! - **pixels recomputed** is what `Renderer::draw` actually painted.
//!   `draw` already returns it and `App::draw` throws it away, so nothing
//!   has ever printed it. The dirty-rect skip is only working to the extent
//!   this is below the 163,840 of a full frame.
//! - **active sites** is the scheduler's queue depth.
//!
//! ```text
//! cargo run --release --example perf_counters
//! cargo run --release --example perf_counters -- frames=6000
//! ```

use pixel_physics::app::{App, HEIGHT, WIDTH};
use pixel_physics::sim::{parallel, player, rigid};

const FULL_FRAME: usize = (WIDTH * HEIGHT) as usize;

struct Row {
    frame: usize,
    awake: usize,
    pixels: usize,
    sites: usize,
    solved: usize,
    read: usize,
    tiles: usize,
    full_reason: Option<&'static str>,
    camera_x: i32,
    player_x: i32,
    player_y: i32,
}

fn tick(app: &mut App, frame: &mut [u8]) -> Row {
    parallel::step(&mut app.world);
    app.world.step_liquid_bodies();
    rigid::step_chunk_bodies(&mut app.world);
    player::step(&mut app.world, app.player_input, &app.player_tuning);
    app.player_input.jump_pressed = false;
    app.world.step_active_sites();
    let mut particles = std::mem::take(&mut app.particles);
    app.blasts.step(&mut app.world, &mut particles);
    particles.step(&mut app.world);
    app.particles = particles;
    app.world.step_fields();
    app.world.step_pheromones();

    // Read *before* the draw consumes them: `take_touched_chunks` clears the
    // set, and `active_chunk_count` is what the field gate reads.
    let awake = app.world.active_chunk_count();
    let sites = app.world.active_site_count();

    // `Renderer::draw` direct rather than `App::draw`, for its return value
    // — the pixel count is the whole point of this harness and `App::draw`
    // discards it. `force_full: false` matches the app with no cursor and no
    // panel showing, which is what play looks like most of the time.
    if let Some(p) = &app.world.player {
        let target = p.center();
        let bounds = app.world.bounds();
        app.renderer.follow(target, (WIDTH, HEIGHT), bounds);
    }
    let touched = app.world.take_touched_chunks();
    let pixels = app.renderer.draw(&app.world, &app.particles, &touched, frame, (WIDTH, HEIGHT), false);

    use std::sync::atomic::Ordering::Relaxed;
    Row {
        frame: 0,
        awake,
        pixels,
        sites,
        solved: pixel_physics::sim::field::SOLVED_TILES.load(Relaxed),
        read: pixel_physics::sim::field::READ_TILES.load(Relaxed),
        tiles: pixel_physics::sim::field::TOTAL_TILES.load(Relaxed),
        full_reason: app.renderer.last_full_reason,
        camera_x: app.renderer.screen_to_world(0, 0).0,
        player_x: app.world.player.as_ref().map_or(i32::MIN, |p| p.center().0),
        player_y: app.world.player.as_ref().map_or(i32::MIN, |p| p.center().1),
    }
}

fn summarise(label: &str, rows: &[Row]) {
    println!("\n=== {label} ===");
    let n = rows.len().max(1);
    let awake: Vec<usize> = rows.iter().map(|r| r.awake).collect();
    let pixels: Vec<usize> = rows.iter().map(|r| r.pixels).collect();
    let full = pixels.iter().filter(|&&p| p >= FULL_FRAME).count();
    let quiet = awake.iter().filter(|&&a| a == 0).count();
    println!(
        "  awake chunks:      mean {:.1}, min {}, max {}  — {} of {} frames fully quiet ({:.0}%)",
        awake.iter().sum::<usize>() as f64 / n as f64,
        awake.iter().min().copied().unwrap_or(0),
        awake.iter().max().copied().unwrap_or(0),
        quiet,
        n,
        quiet as f64 * 100.0 / n as f64,
    );
    println!(
        "  pixels redrawn:    mean {:.0} of {FULL_FRAME} ({:.0}% of a full frame)  — {} frames repainted the whole screen ({:.0}%)",
        pixels.iter().sum::<usize>() as f64 / n as f64,
        pixels.iter().sum::<usize>() as f64 * 100.0 / (n * FULL_FRAME) as f64,
        full,
        full as f64 * 100.0 / n as f64,
    );
    let tiles = rows.iter().map(|r| r.tiles).max().unwrap_or(0);
    println!(
        "  field tiles:       solved mean {:.0} / read mean {:.0} of {} resident ({:.0}% of the world solved per frame)",
        rows.iter().map(|r| r.solved).sum::<usize>() as f64 / n as f64,
        rows.iter().map(|r| r.read).sum::<usize>() as f64 / n as f64,
        tiles,
        rows.iter().map(|r| r.solved).sum::<usize>() as f64 * 100.0 / (n * tiles.max(1)) as f64,
    );
    println!(
        "  active sites:      mean {:.0}, max {}",
        rows.iter().map(|r| r.sites).sum::<usize>() as f64 / n as f64,
        rows.iter().map(|r| r.sites).max().unwrap_or(0),
    );
    // **Did the camera actually pan, and on how many frames?** The audit
    // could not tell "the camera is a full-redraw trigger by construction"
    // apart from "the camera fires on every frame the gnome walks", because
    // the only run where he walked was also raining on 99% of its frames.
    // A per-frame count settles it and costs nothing.
    let camera_moves = rows.windows(2).filter(|w| w[0].camera_x != w[1].camera_x).count();
    let player_moves = rows.windows(2).filter(|w| w[0].player_x != w[1].player_x).count();
    if let (Some(a), Some(b)) = (rows.first(), rows.last()) {
        println!(
            "  camera:            moved on {} of {} frames (world x {} -> {}); player moved on {} frames (x {} -> {})",
            camera_moves, n, a.camera_x, b.camera_x, player_moves, a.player_x, b.player_x,
        );
    }
    // **The trajectory, not just the counts.** A `perf-lock` review proposed
    // that the gnome "stopping" after 126 cells and the camera's single
    // 86-cell jump (which exceeds `camera_max_step = 6.0`, so it can only be
    // `follow`'s one-frame re-centre branch) are one event, not two bugs --
    // a teleport or respawn would produce both. That is a testable claim and
    // this is the test: a walk shows many small steps, a teleport shows one
    // large one.
    if rows.iter().any(|r| r.player_x != i32::MIN) {
        let mut jumps: Vec<String> = Vec::new();
        for w in rows.windows(2) {
            let (dx, dy) = (w[1].player_x - w[0].player_x, w[1].player_y - w[0].player_y);
            let dcam = w[1].camera_x - w[0].camera_x;
            if dx.abs() > 2 || dy.abs() > 2 || dcam != 0 {
                jumps.push(format!("f{} player {:+},{:+} -> ({},{}) camera {:+}", w[1].frame, dx, dy, w[1].player_x, w[1].player_y, dcam));
            }
        }
        println!("  discontinuities ({} total, first 12):", jumps.len());
        for j in jumps.iter().take(12) {
            println!("    {j}");
        }
        let last_move = rows.windows(2).rposition(|w| w[0].player_x != w[1].player_x || w[0].player_y != w[1].player_y);
        println!("    last frame the player moved at all: {:?} of {}", last_move, rows.len());
    }
    // Which trigger actually forced each full repaint. A pixel count says
    // the screen was repainted; only this says whether to fix the camera or
    // the dirty rectangles, and they are opposite fixes.
    let mut why: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for r in rows {
        *why.entry(r.full_reason.unwrap_or("(dirty-rect path)")).or_default() += 1;
    }
    let mut why: Vec<(&str, usize)> = why.into_iter().collect();
    why.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    println!("  why a full redraw:  {}", why.iter().map(|(k, c)| format!("{k} x{c}")).collect::<Vec<_>>().join(", "));
    // The settling curve, not just its endpoint: "it settles eventually" and
    // "it settles at frame 40" are different answers to the cost question.
    let step = (rows.len() / 8).max(1);
    let trace: Vec<String> = rows.iter().step_by(step).map(|r| format!("{}:{}", r.frame, r.awake)).collect();
    println!("  awake over time (frame:chunks)  {}", trace.join("  "));
}

fn run(label: &str, frames: usize, setup: impl FnOnce(&mut App), mut each: impl FnMut(&mut App, usize)) {
    let mut app = App::new();
    setup(&mut app);
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let mut rows = Vec::with_capacity(frames);
    for i in 0..frames {
        each(&mut app, i);
        let mut r = tick(&mut app, &mut frame);
        r.frame = i;
        rows.push(r);
    }
    summarise(label, &rows);
    if let Some(pl) = &app.world.player {
        println!("  player ended at {:?} (camera at {:?})", pl.center(), app.renderer.screen_to_world(0, 0));
    }
    // What is still awake, and what is in it — "the world never settles" is
    // only actionable with the name of the material keeping it awake.
    let mut names: Vec<(String, usize)> = Vec::new();
    for chunk in app.world.chunks().filter(|c| !c.is_settled()) {
        let b = chunk.coord.bounds();
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let cell = app.world.get(x, y);
                if cell.material != pixel_physics::sim::material::EMPTY {
                    *counts.entry(app.world.materials.get(cell.material).name.as_str()).or_default() += 1;
                }
            }
        }
        let mut top: Vec<(&str, usize)> = counts.into_iter().collect();
        top.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        let desc = top.iter().take(3).map(|(m, n)| format!("{m} {n}")).collect::<Vec<_>>().join(", ");
        names.push((format!("chunk ({},{}): {desc}", chunk.coord.x, chunk.coord.y), 1));
    }
    println!("  still awake at the end: {} chunks", names.len());
    for (line, _) in names.iter().take(8) {
        println!("    {line}");
    }
}

fn main() {
    let mut frames = 3000usize;
    for arg in std::env::args().skip(1) {
        if let Some(v) = arg.strip_prefix("frames=") {
            frames = v.parse().expect("frames=N");
        }
    }

    run("a generated world, left alone", frames, |_| {}, |_, _| {});

    // **Rain masks everything else.** At the default seed it was falling for
    // nearly every frame of the runs above, and precipitation forces a full
    // redraw on its own — so no other trigger can be attributed while it is.
    // Pick a seed that is dry for the whole window first, then walk.
    run(
        "the gnome walking, on a DRY seed (camera isolated)",
        600,
        |app| {
            for candidate in 0..400u64 {
                app.worldgen_seed = candidate;
                app.reset();
                if (0..700).all(|f| !pixel_physics::sim::weather::at(app.world.seed, f).is_precipitating()) {
                    println!("  (dry world seed {} -> weather seed {:#x})", candidate, app.world.seed);
                    break;
                }
            }
            app.summon_player(WIDTH as i32 / 2, HEIGHT as i32 / 2);
        },
        |app, _| {
            app.player_input.right = true;
        },
    );

    run(
        "the gnome walking",
        frames.min(1200),
        |app| {
            app.summon_player(WIDTH as i32 / 2, HEIGHT as i32 / 2);
        },
        |app, _| {
            app.player_input.right = true;
        },
    );
}
