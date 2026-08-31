//! What a full-screen redraw is actually spending its time on.
//!
//! `Renderer::draw`'s full branch measured 12.07 ms mean on the shipped
//! 2048x640 world — 54% of a frame, and it runs on **100% of frames while
//! the gnome is walking**, because a camera move invalidates every pixel.
//! That is 74 ns for one pixel, which is far too much for a palette lookup,
//! so the question is where it goes.
//!
//! Every case here draws the same 163,840 pixels of the same world, so this
//! is a paired comparison in the sense `CLAUDE.md` asks for: the only thing
//! varying between rows is the mechanism, and whatever the machine is doing
//! affects all of them equally.
//!
//! ```text
//! cargo run --release --example render_cost
//! ```

use pixel_physics::app::{App, HEIGHT, WIDTH};
use pixel_physics::render::Renderer;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::ChunkCoord;
use pixel_physics::sim::world::World;

const PIXELS: usize = (WIDTH * HEIGHT) as usize;

fn best_of(runs: usize, mut f: impl FnMut() -> u64) -> (f64, u64) {
    // Best of N, not mean: every source of noise on this machine only ever
    // *adds* time, so the minimum is the closest thing to the true cost.
    let mut best = f64::INFINITY;
    let mut check = 0;
    for _ in 0..runs {
        let t = std::time::Instant::now();
        check = f();
        best = best.min(t.elapsed().as_secs_f64() * 1000.0);
    }
    (best, check)
}

/// Read every visible pixel's cell through the ordinary `World::get` — one
/// `HashMap<ChunkCoord, Chunk>` lookup, and therefore one SipHash, per pixel.
/// This is what `cell_colour` does today.
fn read_via_world_get(world: &World, x0: i32, y0: i32) -> u64 {
    let mut acc = 0u64;
    for sy in 0..HEIGHT as i32 {
        for sx in 0..WIDTH as i32 {
            acc += world.get(x0 + sx, y0 + sy).material.0 as u64;
        }
    }
    acc
}

/// The same reads, with the chunk looked up once per run of pixels that
/// share one. At 1:1 zoom that is 64 consecutive pixels, so this does 1/64th
/// of the hashing for exactly the same answer.
fn read_via_chunk_hoist(world: &World, x0: i32, y0: i32) -> u64 {
    let mut acc = 0u64;
    for sy in 0..HEIGHT as i32 {
        let wy = y0 + sy;
        let mut held: Option<(ChunkCoord, &pixel_physics::sim::chunk::Chunk)> = None;
        for sx in 0..WIDTH as i32 {
            let wx = x0 + sx;
            if !world.in_bounds(wx, wy) {
                continue;
            }
            let coord = ChunkCoord::containing(wx, wy);
            let chunk = match held {
                Some((c, chunk)) if c == coord => Some(chunk),
                _ => {
                    let found = world.chunk(coord);
                    held = found.map(|c| (coord, c));
                    found
                }
            };
            let cell = chunk.map_or(Cell::EMPTY, |c| c.get_world(wx, wy));
            acc += cell.material.0 as u64;
        }
    }
    acc
}


/// The two branches of `cell_colour`, isolated by building a world that is
/// entirely one or entirely the other. Same pixel count, same renderer, same
/// machine — so the only thing varying is which branch every pixel takes.
/// This is the measurement that says where the 85% "colour work" lives, and
/// it needs no access to the private function itself.
fn branch_split() {
    use pixel_physics::sim::chunk::Rect;
    let mut frame = vec![0u8; PIXELS * 4];
    let touched = std::collections::HashSet::new();
    let particles = pixel_physics::sim::particle::ParticleSystem::default();

    let mut case = |label: &str, fill: Option<pixel_physics::sim::material::MaterialId>| {
        let mut world = World::new(Rect::new(0, 0, WIDTH as i32 - 1, HEIGHT as i32 - 1));
        if let Some(m) = fill {
            for y in 0..HEIGHT as i32 {
                for x in 0..WIDTH as i32 {
                    world.set(x, y, Cell::new(m, (x * 7 + y * 13) as u8));
                }
            }
        }
        world.end_step();
        let mut r = Renderer::new();
        let (ms, _) = best_of(20, || r.draw(&world, &particles, &touched, &mut frame, (WIDTH, HEIGHT), true) as u64);
        println!("{label:>34}  {ms:>7.3}ms  {:>7.1}", ms * 1e6 / PIXELS as f64);
        ms
    };

    println!("
  every pixel through one branch of cell_colour:");
    let sky = case("all empty sky", None);
    let stone = case("all stone", Some(pixel_physics::sim::material::STONE));
    let water = case("all water", Some(pixel_physics::sim::material::WATER));

    // **The lab's air**, and the reason it is measured here rather than in a
    // harness of its own: the row above it is the number it has to beat, on
    // the same machine, in the same run, over the same 163,840 pixels. The
    // design guide's §2 line is *"whatever fills the air above the soil must
    // not draw as sky"*, and "must not" is only checkable against sky's own
    // figure from the same minute — a ns/px quoted from a report is a
    // measurement of somebody else's machine.
    //
    // Same empty world as `all empty sky` above; the only difference is that
    // this one has declared itself a room. That is the paired comparison
    // `CLAUDE.md` asks for, with the paint path held fixed.
    let interior = {
        let mut world = World::new(Rect::new(0, 0, WIDTH as i32 - 1, HEIGHT as i32 - 1));
        world.set_enclosure(Some(
            pixel_physics::sim::enclosure::Enclosure::new(4, HEIGHT as i32 - 40)
                .with_lamps(vec![64, 192, 320, 448], 80),
        ));
        world.end_step();
        let mut r = Renderer::new();
        let (ms, _) = best_of(20, || r.draw(&world, &particles, &touched, &mut frame, (WIDTH, HEIGHT), true) as u64);
        println!("{:>34}  {ms:>7.3}ms  {:>7.1}", "all lab interior", ms * 1e6 / PIXELS as f64);
        ms
    };

    let nspx = |ms: f64| ms * 1e6 / PIXELS as f64;
    println!(
        "
  sky {:.1} ns/px, stone {:.1} ns/px, water {:.1} ns/px, lab interior {:.1} ns/px",
        nspx(sky), nspx(stone), nspx(water), nspx(interior)
    );
    println!(
        "  the lab interior is {:.2}x sky and {:.2}x stone",
        interior / sky,
        interior / stone
    );
}

fn main() {
    let mut app = App::new();
    // Let the generated world settle, so this measures a drawn world rather
    // than one still collapsing.
    for _ in 0..600 {
        app.update();
    }
    let (x0, y0) = app.renderer.screen_to_world(0, 0);
    println!("viewport origin ({x0}, {y0}), {PIXELS} pixels, {} chunks resident", app.world.chunk_count());

    let (get_ms, a) = best_of(20, || read_via_world_get(&app.world, x0, y0));
    let (hoist_ms, b) = best_of(20, || read_via_chunk_hoist(&app.world, x0, y0));
    assert_eq!(a, b, "the two read paths must return the same cells or the comparison is meaningless");

    let mut frame = vec![0u8; PIXELS * 4];
    let touched = std::collections::HashSet::new();
    let particles = pixel_physics::sim::particle::ParticleSystem::default();
    let (draw_ms, _) = best_of(20, || {
        app.renderer.draw(&app.world, &particles, &touched, &mut frame, (WIDTH, HEIGHT), true) as u64
    });

    // An empty `Renderer` over the same world isolates nothing on its own,
    // so the useful comparison is against the read paths above.
    println!("\n{:>34}  {:>9}  {:>9}", "", "ms", "ns/pixel");
    let row = |label: &str, ms: f64| println!("{label:>34}  {ms:>7.3}ms  {:>7.1}", ms * 1e6 / PIXELS as f64);
    row("full redraw (Renderer::draw)", draw_ms);
    row("just the cell reads, World::get", get_ms);
    row("just the cell reads, chunk-hoisted", hoist_ms);
    println!(
        "\n  the per-pixel chunk lookup costs {:.3} ms of the {:.3} ms redraw ({:.0}% of it)",
        get_ms - hoist_ms,
        draw_ms,
        (get_ms - hoist_ms) * 100.0 / draw_ms,
    );
    println!("  reads alone are {:.0}% of the redraw; the other {:.0}% is colour work", get_ms * 100.0 / draw_ms, (draw_ms - get_ms) * 100.0 / draw_ms);

    sky_share(&mut app.renderer, &app.world, x0, y0);
    viewport_scaling(&mut app);
    branch_split();
}

/// How much of the screen is empty sky, which takes `cell_colour`'s most
/// expensive branch — a gradient, a moon-distance test and a position hash
/// per pixel, none of which varies with anything the CA sweep does.
fn sky_share(renderer: &mut Renderer, world: &World, x0: i32, y0: i32) {
    let mut empty = 0usize;
    for sy in 0..HEIGHT as i32 {
        for sx in 0..WIDTH as i32 {
            if world.get(x0 + sx, y0 + sy).material == pixel_physics::sim::material::EMPTY {
                empty += 1;
            }
        }
    }
    let _ = renderer;
    println!("  {empty} of {PIXELS} visible pixels are empty sky ({:.0}%) — the branch with the gradient, moon and star hash", empty as f64 * 100.0 / PIXELS as f64);
}

/// **What doubling the framebuffer actually costs**, measured rather than
/// assumed to be 4x.
///
/// `Renderer::draw` takes its viewport as a runtime argument, so this needs
/// no rebuild at a different `WIDTH`/`HEIGHT` — the same renderer over the
/// same world, drawn at three sizes, is a paired comparison in the sense
/// `CLAUDE.md` asks for. The question it exists for: the resolution step
/// (`Reports/world-scale-handoff.md`'s "higher resolution later") makes the
/// viewport 1024x640 cells, and every pixel is still one cell at 1:1 zoom,
/// so there is no reuse to be had — the cost should be linear in pixels.
/// Whether it is *exactly* linear is the thing worth checking, since a
/// bigger frame buffer also falls out of L2.
fn viewport_scaling(app: &mut App) {
    let particles = pixel_physics::sim::particle::ParticleSystem::default();
    let touched = std::collections::HashSet::new();
    let (x0, y0) = app.renderer.screen_to_world(0, 0);

    // **The generated world is not a valid control for this question**, and
    // the first version of this section was wrong because of it. The camera
    // is clamped at the world origin, so a taller viewport does not show the
    // same scene bigger -- it shows *further down*, and everything it adds is
    // underground stone, which is `cell_colour`'s cheapest branch (48.8 ns/px
    // against sky's 70.3). Per-pixel cost then falls as the viewport grows and
    // the scaling reads sublinear, which is a statement about the content mix
    // and not about the renderer. So the sky share is printed per size, and
    // the uniform-world control below is the number to actually read.
    println!("\n  full redraw at increasing viewport size, GENERATED world (content mix varies -- see note):");
    println!("  {:>12}  {:>10}  {:>9}  {:>9}  {:>8}  {:>8}", "viewport", "pixels", "ms", "ns/pixel", "vs 1x", "sky");
    let mut base = f64::NAN;
    for (w, h) in [(WIDTH, HEIGHT), (WIDTH * 3 / 2, HEIGHT * 3 / 2), (WIDTH * 2, HEIGHT * 2)] {
        let px = (w * h) as usize;
        let mut frame = vec![0u8; px * 4];
        let (ms, _) = best_of(10, || {
            app.renderer.draw(&app.world, &particles, &touched, &mut frame, (w, h), true) as u64
        });
        if base.is_nan() {
            base = ms;
        }
        let mut empty = 0usize;
        for sy in 0..h as i32 {
            for sx in 0..w as i32 {
                if app.world.get(x0 + sx, y0 + sy).material == pixel_physics::sim::material::EMPTY {
                    empty += 1;
                }
            }
        }
        println!(
            "  {:>12}  {px:>10}  {ms:>7.3}ms  {:>7.1}  {:>7.2}x  {:>7.0}%",
            format!("{w}x{h}"),
            ms * 1e6 / px as f64,
            ms / base,
            empty as f64 * 100.0 / px as f64,
        );
    }

    // The control: one material everywhere, so every pixel takes the same
    // branch at every size and the only thing varying is how many there are.
    // Anything other than 4.00x at 2x linear is the renderer, not the scene.
    use pixel_physics::sim::chunk::Rect;
    for (label, fill) in [("all stone", Some(pixel_physics::sim::material::STONE)), ("all empty sky", None)] {
        let big = Rect::new(0, 0, (WIDTH * 2) as i32 - 1, (HEIGHT * 2) as i32 - 1);
        let mut world = World::new(big);
        if let Some(m) = fill {
            for y in 0..(HEIGHT * 2) as i32 {
                for x in 0..(WIDTH * 2) as i32 {
                    world.set(x, y, Cell::new(m, (x * 7 + y * 13) as u8));
                }
            }
        }
        world.end_step();
        println!("\n  CONTROL -- uniform world, {label} (identical branch at every size):");
        println!("  {:>12}  {:>10}  {:>9}  {:>9}  {:>8}", "viewport", "pixels", "ms", "ns/pixel", "vs 1x");
        let mut base = f64::NAN;
        for (w, h) in [(WIDTH, HEIGHT), (WIDTH * 3 / 2, HEIGHT * 3 / 2), (WIDTH * 2, HEIGHT * 2)] {
            let px = (w * h) as usize;
            let mut frame = vec![0u8; px * 4];
            let mut r = Renderer::new();
            let (ms, _) = best_of(10, || r.draw(&world, &particles, &touched, &mut frame, (w, h), true) as u64);
            if base.is_nan() {
                base = ms;
            }
            println!(
                "  {:>12}  {px:>10}  {ms:>7.3}ms  {:>7.1}  {:>7.2}x",
                format!("{w}x{h}"),
                ms * 1e6 / px as f64,
                ms / base
            );
        }
    }
}
