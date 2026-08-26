//! Does the camera really jump 86 cells in one frame, through the path the
//! app actually uses?
//!
//! `perf_counters` (built by the same 2026-08-19 audit and **deliberately not
//! landed** -- it needs global atomic counters in `field::step` and a new
//! `Renderer` field, which is hot-path instrumentation this repo treats as a
//! cost decision rather than a recovery) measured one camera discontinuity in a 600-frame
//! walk: at frame 174 the gnome took an ordinary `+1, +0` step and the
//! camera moved `+86` — a sixth of the screen, in one frame, against
//! `camera_max_step = 6.0`. That harness calls `Renderer::follow` itself.
//! This one drives `App::update` / `App::draw` exactly as `main.rs` does, so
//! a jump here is the app's and not the harness's.
//!
//! `CLAUDE.md`: a reproduction has to be confirmed to show the complained-of
//! quantity before anything is built on it.

use pixel_physics::app::{App, HEIGHT, WIDTH};

fn main() {
    let mut app = App::new();
    app.summon_player(WIDTH as i32 / 2, HEIGHT as i32 / 2);
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];

    let mut previous = app.renderer.screen_to_world(0, 0).0;
    let mut jumps = 0usize;
    let mut worst = 0i32;
    for f in 0..600 {
        app.player_input.right = true;
        // Exactly `main.rs`'s order: one fixed-timestep tick, then a draw.
        // `App::draw` is what calls `Renderer::follow` in the real app.
        app.update();
        app.draw(&mut frame, None);
        let camera = app.renderer.screen_to_world(0, 0).0;
        let step = camera - previous;
        if step != 0 {
            jumps += 1;
            if step.abs() > worst.abs() {
                worst = step;
            }
            if jumps <= 12 {
                let p = app.world.player.as_ref().map(|p| p.center()).unwrap_or((0, 0));
                println!("  f{f}: camera {previous} -> {camera} ({step:+}), player at {p:?}");
            }
        }
        previous = camera;
    }
    println!("\n  {jumps} camera moves over 600 frames, largest single step {worst:+}");
    println!("  camera_max_step is 6.0, so any step past that came from `follow`'s one-frame re-centre branch");
    if let Some(p) = &app.world.player {
        println!("  player ended at {:?}", p.center());
    }
}
