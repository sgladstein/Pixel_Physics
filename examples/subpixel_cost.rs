//! **What does drawing the same world view at more pixels per cell cost?**
//!
//! `render_cost`'s `viewport_scaling` section answers a neighbouring question
//! and not this one: it grows the *viewport* at `zoom == 1`, so a bigger frame
//! shows **more world**, and its own note says the extra is cheap underground
//! stone. Supersampling shows the **same** world at a finer output lattice,
//! which is a different cost shape -- the same cell is read once and painted
//! `zoom^2` times, so the cell data is hot in cache where the bigger viewport
//! is walking new chunks.
//!
//! Every row here draws the identical world region from the identical camera,
//! so this is a paired comparison in the sense `CLAUDE.md` asks for: the only
//! thing varying is how many pixels the region is drawn into.
//!
//! ```text
//! cargo run --release --example subpixel_cost
//! ```

use pixel_physics::app::{App, HEIGHT, WIDTH};

/// Best of N, not mean: every source of noise on this machine only ever
/// *adds* time, so the minimum is the closest thing to the true cost. Same
/// reasoning as `render_cost::best_of`, which this deliberately mirrors so the
/// two are comparable.
fn best_of(runs: usize, mut f: impl FnMut()) -> f64 {
    let mut best = f64::INFINITY;
    for _ in 0..runs {
        let t = std::time::Instant::now();
        f();
        best = best.min(t.elapsed().as_secs_f64() * 1000.0);
    }
    best
}

fn main() {
    let mut app = App::new();
    // Let the generated world settle, so this measures a drawn world rather
    // than one still collapsing.
    for _ in 0..600 {
        app.update();
    }
    let touched = std::collections::HashSet::new();
    let base_pixels = (WIDTH * HEIGHT) as f64;

    println!("same world region, same camera, drawn at increasing pixels per cell:\n");
    println!("{:>6}  {:>12}  {:>11}  {:>10}  {:>9}  {:>9}", "px/cell", "buffer", "pixels", "ms", "vs 1x", "ns/px");

    let mut base_ms = 0.0;
    for zoom in [1, 2, 3] {
        let (w, h) = (WIDTH * zoom as u32, HEIGHT * zoom as u32);
        let pixels = (w * h) as f64;
        let mut frame = vec![0u8; (w * h * 4) as usize];
        app.renderer.zoom = zoom;
        // The camera is in *cells* and the buffer grew with the zoom, so the
        // visible region is unchanged -- which is the whole point. Asserted
        // rather than assumed: a supersample that quietly shows a different
        // amount of world is measuring the wrong thing, and would look
        // exactly like a result.
        let (x0, y0) = app.renderer.screen_to_world(0, 0);
        let (x1, y1) = app.renderer.screen_to_world(w as i32 - 1, h as i32 - 1);
        assert_eq!(
            (x1 - x0, y1 - y0),
            (WIDTH as i32 - 1, HEIGHT as i32 - 1),
            "px/cell {zoom} must cover the same {WIDTH}x{HEIGHT} cells as 1x"
        );

        let ms = best_of(10, || {
            // `force_full` on every run: the dirty-rect skip would otherwise
            // make run 2 onward free and the minimum would be a measurement
            // of the skip.
            app.renderer.draw(&app.world, &app.particles, &touched, &mut frame, (w, h), true);
        });
        if zoom == 1 {
            base_ms = ms;
        }
        println!(
            "{zoom:>6}  {:>12}  {pixels:>11.0}  {ms:>9.3}ms  {:>8.2}x  {:>9.1}",
            format!("{w}x{h}"),
            ms / base_ms,
            ms * 1e6 / pixels
        );
    }
    println!(
        "\n  1x is {base_pixels:.0} pixels; 3x is {:.0}, nine times as many.",
        base_pixels * 9.0
    );
    println!("  Read the vs-1x column against 4x and 9x -- anything well under is per-draw");
    println!("  fixed cost (sky light, horizon, glow scan) that a finer lattice does not repeat.");
}
