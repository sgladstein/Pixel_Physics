//! **What does carrying the whole zoomed-out view at one cell per pixel
//! cost?** — the price of the owner's question, *"why my screen resolution can
//! solve all of the pixels, why cannot there just be more pixels when you zoom
//! out?"* (2026-09-13).
//!
//! Every arm draws the **same span of world cells** from the same camera, and
//! varies only how that span is divided between *buffer resolution* and
//! *stride*: `512x320 @ stride 4` is what ships, `2048x1280 @ stride 1`
//! discards nothing. The span is asserted rather than assumed, for
//! `subpixel_cost`'s reason — an arm that quietly shows a different amount of
//! world is measuring content, and would look exactly like a result.
//!
//! **Why this is neither `subpixel_cost` nor `render_cost`'s
//! `viewport_scaling`.** `subpixel_cost` grows the buffer at `zoom > 1`,
//! magnifying: one cell is read once and painted `zoom²` times, so cell reads
//! *fall* per pixel. `viewport_scaling` grows the buffer at stride 1, so a
//! bigger frame shows **more world** and the extra is content (its own note
//! says the extra is cheap underground stone). This holds the world span fixed
//! and trades stride against resolution, which has a cost shape neither has:
//! **total cell reads are constant across the arms** — `pixels x stride²` is
//! the same product — so whatever the bigger buffer costs is per-*pixel* work
//! (`cell_colour`, the byte writes, the overlays) and not re-reading the world.
//! That is the hypothesis the owner's question rests on, and it is testable.
//!
//! **Two timings per arm, because the brief's risk is the second one.** A
//! forced full redraw is the moving case; a world nobody is disturbing is where
//! the dirty-rect render skip earns its keep, and `CLAUDE.md` carries a
//! measured case of a change that looked free in every moving scene and cost
//! ~10 ms/frame at rest. The settled arm follows `zoomfilter`'s technique
//! exactly — `force_full` false with nothing touched and the world **not**
//! ticked between draws — and each arm owns its own `Renderer`, because a
//! stride change forces a full redraw by design (`Renderer::draw`'s
//! `scale_changed`) and a shared renderer would measure that instead.
//!
//! **The whole-frame column is the one to quote.** A render-phase figure alone
//! is half a number (`instruments.md`), and this repo has a measured case of a
//! change that removed 91% of a phase's work and made the frame slower. `sim`
//! is `frame::step` on the same world, measured after the draws.
//!
//! **Both games, because `src/render.rs` is shared.** The lab is the case the
//! round-32 brief expects to be nearly free: its boxes are far below
//! `MAX_BOX`, so a grown buffer shows a typical bed whole at one cell per pixel
//! — and its simulation half is a fraction of the outdoor world's, which is
//! exactly what decides whether a render multiplier is affordable.
//!
//! ```text
//! cargo run --release --example zoomout_pixels
//! cargo run --release --example zoomout_pixels -- game=lab reps=12
//! cargo run --release --example zoomout_pixels -- game=world settle=1200 seed=7
//! ```

use pixel_physics::app::{App, HEIGHT, MAX_PIXEL_SCALE, WIDTH, WORLD_HEIGHT, WORLD_WIDTH};
use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::Lab;
use pixel_physics::render::{Renderer, ZoomOutFilter};
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::fxhash::ChunkSet;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(str::to_string))
        .and_then(|v| v.parse().ok())
}

/// The widest zoom-out the app allows (`render::MAX_ZOOM_OUT_STRIDE`, which is
/// private). Every arm has to cover exactly `WIDTH * this` x `HEIGHT * this`
/// cells; the assertion below is what says so.
const SPAN_STRIDE: u32 = 4;

/// One way of dividing the span: `scale` buffer multiples at `stride` cells per
/// pixel, with `scale * stride == SPAN_STRIDE`.
struct Arm {
    scale: u32,
    stride: i32,
    renderer: Renderer,
    frame: Vec<u8>,
    full: Vec<f64>,
    settled: Vec<f64>,
    settled_px: usize,
}

impl Arm {
    fn new(scale: u32, camera: (i32, i32), bounds: Option<Rect>) -> Self {
        let stride = (SPAN_STRIDE / scale) as i32;
        let (w, h) = (WIDTH * scale, HEIGHT * scale);
        let mut renderer = Renderer::new();
        renderer.zoom = 1;
        renderer.zoom_out_filter = ZoomOutFilter::Coverage;
        renderer.zoom_out_stride = stride;
        // The camera is in *cells* and `visible_span` is `buffer x stride`,
        // which is the same product in every arm — so one camera position
        // clamps identically across them, and the covered region is shared.
        renderer.set_camera(camera.0, camera.1, (w, h), bounds);
        Arm { scale, stride, renderer, frame: vec![0u8; (w * h * 4) as usize], full: Vec::new(), settled: Vec::new(), settled_px: 0 }
    }

    fn buffer(&self) -> (u32, u32) {
        (WIDTH * self.scale, HEIGHT * self.scale)
    }
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).expect("no NaN from a clock"));
    if v.is_empty() {
        f64::NAN
    } else {
        v[v.len() / 2]
    }
}

/// The lab bed, sized as `zoomfilter`'s is and for the same reason: the cap is
/// derived from the *box*, so the shipped 512x320 bed cannot zoom out at all
/// and every arm would collapse onto stride 1.
fn lab_world(frames: u64) -> World {
    let base = LabBox::default();
    let (width, height): (i32, i32) = (arg("width").unwrap_or(2048), arg("height").unwrap_or(1280));
    let ground_y = base.ground_y * height / base.height.max(1);
    let mut lab = Lab::new(LabBox {
        width,
        height,
        ground_y,
        founders: arg("founders").unwrap_or(48),
        colonies: arg("colonies").unwrap_or(4),
        compartments: arg("walls").unwrap_or(1),
        seed: arg("seed").unwrap_or(base.seed),
        ..base
    });
    let tuning = player::Tuning::default();
    for _ in 0..frames {
        pixel_physics::sim::frame::step(&mut lab.world, &mut lab.particles, &mut lab.blasts, player::PlayerInput::default(), &tuning);
    }
    lab.world
}

fn outdoor_world(settle: u64) -> World {
    let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("{e}");
    }
    let name: String = arg("preset").unwrap_or_else(|| presets.default_name());
    let Some(params) = presets.get(&name) else { panic!("unknown worldgen preset {name:?}") };
    let seed: u32 = arg("seed").unwrap_or(1);
    let mut world = World::new(Rect::new(0, 0, WORLD_WIDTH as i32 - 1, WORLD_HEIGHT as i32 - 1));
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed: seed as u64 });
    let mut particles = ParticleSystem::new();
    let mut blasts = pixel_physics::sim::explosion::Blasts::new();
    let tuning = player::Tuning::default();
    for _ in 0..settle {
        pixel_physics::sim::frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
    }
    world
}

/// Where to point the camera, and **what in the view is worth cropping**.
///
/// Returns `(camera, interest)`. The two are separate because at the widest
/// zoom-out they cannot coincide: the span is 1280 rows and the outdoor
/// skyline sits ~400 rows down an 2560-row world, so a camera that would put
/// the skyline in the middle of the screen is above the world and
/// `set_camera` clamps it. Centring the crop on the *span* after that clamp
/// photographs 250 rows of underground stone — which is what the first run of
/// this harness did, and it reads as a render finding rather than as a camera
/// that went somewhere else.
///
/// `aim=life` (the default) is `zoomfilter`'s reason restated: flora is sparse
/// in 8192x2560, so a crop taken anywhere but the skyline prices a sheet of
/// rock and tells you nothing about the thing the owner complained about.
fn aim(world: &World, span: (i32, i32)) -> ((i32, i32), (i32, i32)) {
    let b = world.bounds().unwrap_or(Rect::new(0, 0, 0, 0));
    let (w, h) = (b.max_x - b.min_x + 1, b.max_y - b.min_y + 1);
    let centre = (b.min_x + w / 2, b.min_y + h / 2);
    if arg::<String>("aim").as_deref() == Some("world") || h <= span.1 {
        return ((b.min_x + (w - span.0) / 2, b.min_y + (h - span.1) / 2), centre);
    }
    // The skyline, found by walking down the middle column to the first
    // non-empty cell. That cell is the interest point; the camera is put half a
    // span above it and left to clamp, which is what the app itself does.
    let x = b.min_x + w / 2;
    let mut y = b.min_y;
    while y < b.max_y && world.get(x, y).material == pixel_physics::sim::material::EMPTY {
        y += 1;
    }
    ((x - span.0 / 2, y - span.1 / 2), (x, y))
}

fn price(label: &str, world: &World, reps: usize) {
    let particles = ParticleSystem::new();
    let bounds = world.bounds();
    let span = ((WIDTH * SPAN_STRIDE) as i32, (HEIGHT * SPAN_STRIDE) as i32);
    let (want_camera, interest) = aim(world, span);
    let mut arms: Vec<Arm> = (0..3).map(|i| Arm::new(1u32 << i, want_camera, bounds)).collect();
    // **The camera the renderer actually has**, read back rather than assumed:
    // `set_camera` clamps to the world, and every crop below is offset from
    // this. Taking the requested position instead put the first run's outdoor
    // crop 245 rows underground while its label said "skyline".
    let camera = arms[0].renderer.screen_to_world(0, 0);

    // Assert the shared span before timing anything, so a broken arm fails
    // loudly rather than reporting a cheap number about a smaller view.
    for arm in &arms {
        let (w, h) = arm.buffer();
        let (x0, y0) = arm.renderer.screen_to_world(0, 0);
        let (x1, y1) = arm.renderer.screen_to_world(w as i32 - 1, h as i32 - 1);
        assert_eq!(
            (x1 - x0 + arm.stride, y1 - y0 + arm.stride, x0, y0),
            (span.0, span.1, camera.0, camera.1),
            "arm {w}x{h} @ stride {} must cover the same {}x{} cells from the same corner",
            arm.stride,
            span.0,
            span.1
        );
    }

    // The moving case, interleaved round-robin: two byte-identical runs have
    // disagreed 2.42x on this box, so arms are never compared across blocks.
    for _ in 0..reps {
        for arm in arms.iter_mut() {
            let (w, h) = arm.buffer();
            let t = std::time::Instant::now();
            arm.renderer.draw(world, &particles, &ChunkSet::default(), &mut arm.frame, (w, h), true);
            arm.full.push(t.elapsed().as_secs_f64() * 1000.0);
        }
    }
    // The settled case. The loop above has warmed every arm's buffer, so each
    // skip has a valid frame of its own to reuse.
    for _ in 0..reps {
        for arm in arms.iter_mut() {
            let (w, h) = arm.buffer();
            let t = std::time::Instant::now();
            let px = arm.renderer.draw(world, &particles, &ChunkSet::default(), &mut arm.frame, (w, h), false);
            arm.settled.push(t.elapsed().as_secs_f64() * 1000.0);
            arm.settled_px = px;
        }
    }

    // The simulation half, after the draws, so the render figure can be quoted
    // against a whole frame. One `frame::step` is one `App::update`.
    let mut sim = Vec::with_capacity(reps);
    {
        let mut w = world.clone();
        let mut p = ParticleSystem::new();
        let mut blasts = pixel_physics::sim::explosion::Blasts::new();
        let tuning = player::Tuning::default();
        for _ in 0..reps {
            let t = std::time::Instant::now();
            pixel_physics::sim::frame::step(&mut w, &mut p, &mut blasts, player::PlayerInput::default(), &tuning);
            sim.push(t.elapsed().as_secs_f64() * 1000.0);
        }
    }
    let sim_ms = median(&mut sim);

    let in_view = (span.0 as u64) * (span.1 as u64);
    println!("\n=== {label} — same {}x{} cell view ({in_view} cells), camera {camera:?}, median of {reps}, interleaved", span.0, span.1);
    println!("    sim half (frame::step, arm-independent): {sim_ms:.3} ms");
    println!(
        "{:>12}  {:>6}  {:>10}  {:>7}  {:>9}  {:>9}  {:>8}  {:>11}  {:>9}",
        "buffer", "stride", "carried", "dropped", "draw ms", "frame ms", "vs ship", "settled ms", "set px"
    );
    let base_frame = median(&mut arms[0].full.clone()) + sim_ms;
    for arm in arms.iter_mut() {
        let (w, h) = arm.buffer();
        let carried = (w as u64) * (h as u64);
        let draw_ms = median(&mut arm.full);
        let settled_ms = median(&mut arm.settled);
        let frame_ms = draw_ms + sim_ms;
        println!(
            "{:>12}  {:>6}  {:>10}  {:>6.0}%  {:>8.3}  {:>8.3}  {:>7.2}x  {:>10.3}  {:>9}",
            format!("{w}x{h}"),
            arm.stride,
            carried,
            100.0 * (in_view - carried) as f64 / in_view as f64,
            draw_ms,
            frame_ms,
            frame_ms / base_frame,
            settled_ms,
            arm.settled_px
        );
    }
    // The images, if asked for. Written after the timings so the frames hold
    // the last full redraw rather than a partial one from the settled arm.
    if let Some(dir) = arg::<String>("png") {
        // Default crop: the middle 512x320 *cells* of the span — one quarter of
        // its width. At the grown arm's stride 1 that is 512x320 buffer pixels,
        // i.e. a 1:1 crop; the shipped arm holds the same cells in 128x80 and
        // they are magnified back up to meet it.
        let (cw, ch) = (WIDTH as i32, HEIGHT as i32);
        // Centred on the interest point, then clamped inside the span — not on
        // the middle of the span, which underground is all there is to see.
        let cx = (interest.0 - cw / 2).clamp(camera.0, camera.0 + span.0 - cw);
        let cy = (interest.1 - ch / 2).clamp(camera.1, camera.1 + span.1 - ch);
        // Snap to the coarsest stride's lattice so every arm crops the *same*
        // cells rather than cells offset by up to stride-1 — an off-by-three
        // crop would read as a render difference.
        let snap = SPAN_STRIDE as i32;
        let cell = (camera.0 + ((cx - camera.0) / snap) * snap, camera.1 + ((cy - camera.1) / snap) * snap, cw, ch);
        println!("    png: same {cw}x{ch} cells at ({}, {}), one file per arm:", cell.0, cell.1);
        for arm in arms.iter() {
            println!("      {}", write_pngs(&format!("{dir}/{label}"), arm, camera, cell, arg("up").unwrap_or(2)));
        }
    }
}

/// Write the arms as **comparable images**: the same world rectangle, at the
/// same apparent size on screen.
///
/// This is the only honest way to put the two buffers side by side. The shipped
/// arm holds that rectangle in `cw/stride x ch/stride` buffer pixels and the
/// display magnifies them by `stride`; the grown arm holds it at one pixel per
/// cell. So each arm's crop is taken in *buffer* pixels and nearest-upscaled by
/// its own stride, which is exactly what the GPU already does to the shipped
/// frame — both files come out `cw x ch` and a pixel in one is the same patch
/// of world as a pixel in the other. Comparing the raw buffers instead would be
/// comparing two different amounts of world at two different sizes, twice over.
///
/// **Nearest, never a filter.** The question is what reaches the screen, and a
/// smoothing upscale would answer it with its own invention.
fn write_pngs(dir: &str, arm: &Arm, camera: (i32, i32), cell: (i32, i32, i32, i32), up: i32) -> String {
    let (bw, _bh) = arm.buffer();
    let s = arm.stride;
    let (cx, cy, cw, ch) = cell;
    // Buffer pixel of the crop's top-left, and how many buffer pixels it spans.
    let (px0, py0) = ((cx - camera.0) / s, (cy - camera.1) / s);
    let (pw, ph) = (cw / s, ch / s);
    // `up` is a further whole-number magnification of *both* arms alike, for
    // legibility on the review page — the stills the owner has been able to
    // judge are 700-950 px across and a 512-wide crop is under that. It cannot
    // change the comparison, because it is the same integer on every arm.
    let (ow, oh) = (cw * up, ch * up);
    let mut out = vec![0u8; (ow * oh * 4) as usize];
    for oy in 0..oh {
        for ox in 0..ow {
            let (sx, sy) = (px0 + ox / (s * up), py0 + oy / (s * up));
            let src = ((sy * bw as i32 + sx) * 4) as usize;
            let dst = ((oy * ow + ox) * 4) as usize;
            out[dst..dst + 4].copy_from_slice(&arm.frame[src..src + 4]);
        }
    }
    let _ = (pw, ph);
    let path = format!("{dir}/stride{}_x{}.png", arm.stride, arm.scale);
    std::fs::create_dir_all(dir).expect("png dir");
    image::save_buffer(&path, &out, ow as u32, oh as u32, image::ColorType::Rgba8).expect("write png");
    path
}

/// **The shipped path, end to end** — `App::update` + `App::draw` at each
/// `App::pixel_budget`, which is the only arm that includes the HUD and the
/// buffer the real game actually allocates.
///
/// The three arms above drive `Renderer::draw` directly, which is right for
/// pricing a change *before* it exists and is one call short of the frame the
/// player waits for. This closes that gap now that the setting is real: same
/// world, same rung, same camera, and the only thing varying is the budget.
/// `App::viewport()` sizes the buffer exactly as `main.rs` does.
fn price_app(reps: usize) {
    let mut app = App::new();
    for _ in 0..arg::<u64>("settle").unwrap_or(600) {
        app.update();
    }
    app.renderer.zoom = 1;
    app.renderer.zoom_out_stride = SPAN_STRIDE as i32;
    // The window is not in the way here; the question is what the budget
    // costs, not what a particular window allows.
    app.pixel_scale_cap = MAX_PIXEL_SCALE;

    println!("\n=== outdoor, the shipped path (App::update + App::draw, HUD included), median of {reps}");
    println!("{:>8}  {:>12}  {:>10}  {:>10}  {:>10}  {:>9}", "budget", "buffer", "update ms", "draw ms", "frame ms", "vs ship");
    let mut base = f64::NAN;
    for budget in [1, 2, 4] {
        app.pixel_budget = budget;
        let (w, h) = app.viewport();
        let mut frame = vec![0u8; (w * h * 4) as usize];
        // One warm draw per arm: a budget change resizes the buffer, and the
        // first draw into a new one has no previous frame to reuse.
        app.draw(&mut frame, None);
        let (mut up, mut dr) = (Vec::new(), Vec::new());
        for _ in 0..reps {
            let t = std::time::Instant::now();
            app.update();
            up.push(t.elapsed().as_secs_f64() * 1000.0);
            let t = std::time::Instant::now();
            app.draw(&mut frame, None);
            dr.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        let (u, d) = (median(&mut up), median(&mut dr));
        if budget == 1 {
            base = u + d;
        }
        println!("{budget:>8}  {:>12}  {u:>9.3}  {d:>9.3}  {:>9.3}  {:>8.2}x", format!("{w}x{h}"), u + d, (u + d) / base);
    }
}

fn main() {
    let reps: usize = arg("reps").unwrap_or(9);
    let game: String = arg("game").unwrap_or_else(|| "both".to_string());
    println!("zoomout_pixels: game={game} reps={reps} span_stride={SPAN_STRIDE}");
    println!("  'carried' is cells that reach a pixel of their own; 'dropped' is the share of");
    println!("  the view the buffer cannot hold. Cell *reads* are equal across arms");
    println!("  (pixels x stride^2 is constant), so 'vs ship' prices per-pixel work alone.");
    println!("  'vs ship' is the WHOLE frame (sim + draw) against the 512x320/stride-4 row.");
    println!("  'set px' is pixels the dirty-rect skip repainted with nothing touched: a");
    println!("  settled screen should be near 0, and a number near 'carried' means the skip");
    println!("  did not fire and that row's settled figure is a full redraw wearing its name.");
    if game == "app" {
        return price_app(reps);
    }
    if game != "lab" {
        let settle: u64 = arg("settle").unwrap_or(600);
        println!("\n[outdoor] generating {WORLD_WIDTH}x{WORLD_HEIGHT} and settling {settle} frames...");
        let world = outdoor_world(settle);
        price("outdoor", &world, reps);
    }
    if game != "world" {
        let frames: u64 = arg("frames").unwrap_or(3000);
        println!("\n[lab] growing the bed {frames} frames...");
        let world = lab_world(frames);
        price("lab", &world, reps);
    }
}
