//! **What the game looks like zoomed in, under the look the owner picked — in
//! motion, at play scale, and at what it costs.**
//!
//! The style selector itself is in `src/render.rs` ([`MagnifyStyle`]); this
//! drives the **shipped** `Renderer` rather than a copy of it, which is the
//! whole reason this file exists and not a second arm in `examples/zoomin.rs`.
//! `zoomin` answered *which look should this game have* by re-implementing
//! every candidate over one frozen 1:1 frame — admissible for a still
//! comparison, and unable to say anything about a style that has landed. Once
//! a look is in `render.rs`, the questions are *does it hold while things
//! move*, *does it still keep the thin things*, and *what does it cost on a
//! settled screen*, and all three have to be asked of the real draw path.
//!
//! ```text
//! cargo run --release --example magnify -- mode=twigs
//! cargo run --release --example magnify -- mode=cost zoom=3
//! cargo run --release --example magnify -- mode=gif zoom=3 out=/tmp/painted.gif style=painted_ink
//! cargo run --release --example magnify -- mode=gif zoom=3 out=/tmp/pair.gif pair=1
//! ```
//!
//! **`mode=twigs` is the gate this lane turns on.** A painted base thins a
//! one-cell twig into haze at play scale — the owner's *"pixels of plants and
//! other foreground things are disappearing"* arriving at the other end of the
//! zoom range from the one lane S fixed. The table reports, per style and per
//! zoom, how much of a one-cell diagonal twig reaches the screen as a fraction
//! of what today's look gives it: **peak** (is it still there at all) and
//! **mean** (how much of it survived). Both, because they disagree — at an odd
//! zoom the middle pixel of a block sits exactly on its cell centre, so plain
//! bilinear hands a twig one full-strength pixel for free and the peak alone
//! would report no loss where a quarter of the twig had gone.
//!
//! **`mode=cost` interleaves the arms inside one run** and takes a settled
//! frame as well as a moving one. Both are `CLAUDE.md` requirements with a
//! price attached: lane S measured the same fixed binary at 1.45–2.21 ms
//! across seven runs on this container, so no cross-run number is worth
//! anything; and what a per-pixel style threatens is the dirty-rect skip,
//! which only does its work on a settled world. Pin `RAYON_NUM_THREADS` before
//! believing any of it.
//!
//! **`mode=gif` is the card.** The gnome walking, water falling and a fire
//! burning, at play scale, through the style — because a look is judged
//! moving and this repo's record is that stills have twice got a rejection
//! where a GIF got a diagnosis. `pair=1` puts today's look and painted+ink
//! side by side in one image, same world, same seed, same frames, which is the
//! paired comparison a single run against a remembered impression is not.

use pixel_physics::app::App;
use pixel_physics::render::{MagnifyStyle, NotchRule, Renderer, MAGNIFY_INK};
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::fxhash::ChunkSet;
use pixel_physics::sim::material;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::world::World;

const VIEW_W: u32 = 512;
const VIEW_H: u32 = 320;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

fn style_of(name: &str) -> MagnifyStyle {
    match name.to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
        "painted" => MagnifyStyle::Painted,
        "painted_ink" | "ink" => MagnifyStyle::PaintedInk,
        "illustrated" => MagnifyStyle::Illustrated,
        "chamfer" => MagnifyStyle::Chamfer,
        _ => MagnifyStyle::CellArt,
    }
}

const EVERY_STYLE: [(&str, MagnifyStyle); 5] = [
    ("cell-art (today)", MagnifyStyle::CellArt),
    ("painted", MagnifyStyle::Painted),
    ("painted+ink", MagnifyStyle::PaintedInk),
    ("illustrated", MagnifyStyle::Illustrated),
    ("chamfer", MagnifyStyle::Chamfer),
];

// ---------------------------------------------------------------- the twig gate

/// Sky over a floor with `n` one-cell-thick **diagonal** plant twigs, each in
/// its own lane.
///
/// **Diagonal, and that is the whole scene design.** A vertical stem is thin
/// in one axis only, so bilinear blends it with the sky on one axis only and
/// it keeps 87.5% of its peak at zoom 4 — measured, on the first version of
/// this scene, which made the gate report that painted loses almost nothing. A
/// twig one cell thick in *both* axes is what a stand is actually made of and
/// what lane T watched go to haze.
fn twig_scene(spacing: i32, n: i32) -> (World, Vec<(i32, i32)>) {
    let (w, h) = (spacing * n, 64i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    for x in 0..w {
        world.set(x, h - 1, Cell::new(material::STONE, 0));
    }
    let wood = world.materials.id_of("wood").expect("the shipped registry has wood");
    let mut twigs = Vec::new();
    for i in 0..n {
        for k in 0..4 {
            world.set(i * spacing + 1 + k, 30 - k, Cell::new(wood, 0));
        }
        twigs.push((i * spacing + 2, 29));
    }
    world.end_step();
    (world, twigs)
}

/// `(peak, mean)` per twig at `zoom` under `style`, each as a fraction of the
/// same twig's figure under today's look.
///
/// **Differenced against a twig-free control, never matched against wood's
/// palette** — the reasoning lane S's `stem_columns_on_screen` records: the
/// palette goes through the sky light, the depth grade and the grain before it
/// reaches a pixel, so a colour-matching probe would be a test of `cell_colour`
/// wearing the name of a test about styles.
fn twig_strength(zoom: i32, style: MagnifyStyle) -> (Vec<f32>, Vec<f32>) {
    let (spacing, n) = (8i32, 8i32);
    let (world, twigs) = twig_scene(spacing, n);
    let (sw, sh) = ((spacing * n * zoom) as u32, (64 * zoom) as u32);
    let particles = ParticleSystem::new();
    let draw = |w: &World, style: MagnifyStyle| {
        let mut r = Renderer::new();
        r.zoom = zoom;
        r.magnify_style = style;
        let mut f = vec![0u8; (sw * sh * 4) as usize];
        r.draw(w, &particles, &ChunkSet::default(), &mut f, (sw, sh), true);
        f
    };
    let mut bare = world.clone();
    for i in 0..n {
        for k in 0..4 {
            bare.set(i * spacing + 1 + k, 30 - k, Cell::EMPTY);
        }
    }
    bare.end_step();
    let without = draw(&bare, MagnifyStyle::CellArt);
    let measure = |frame: &[u8]| -> Vec<(f32, f32)> {
        twigs
            .iter()
            .map(|&(cx, cy)| {
                // The twig cell's own block and a one-cell ring round it — a
                // style is allowed to *spread* a twig, and a probe confined to
                // its own block would score a dilated one as a lost one.
                let (x0, x1) = (((cx - 1) * zoom).max(0), ((cx + 2) * zoom).min(sw as i32));
                let (y0, y1) = (((cy - 1) * zoom).max(0), ((cy + 2) * zoom).min(sh as i32));
                let (mut peak, mut sum, mut px) = (0.0f32, 0.0f32, 0u32);
                for sy in y0..y1 {
                    for sx in x0..x1 {
                        let i = (sy as usize * sw as usize + sx as usize) * 4;
                        let d: f32 = (0..3).map(|k| (frame[i + k] as f32 - without[i + k] as f32).abs()).sum::<f32>() / 3.0;
                        peak = peak.max(d);
                        sum += d;
                        px += 1;
                    }
                }
                (peak, sum / px.max(1) as f32)
            })
            .collect()
    };
    let base = measure(&draw(&world, MagnifyStyle::CellArt));
    let arm = measure(&draw(&world, style));
    (
        arm.iter().zip(&base).map(|(a, b)| a.0 / b.0.max(1e-3)).collect(),
        arm.iter().zip(&base).map(|(a, b)| a.1 / b.1.max(1e-3)).collect(),
    )
}

fn twig_table() {
    let worst = |v: &[f32]| v.iter().copied().fold(f32::INFINITY, f32::min);
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len().max(1) as f32;
    println!("one-cell diagonal twigs reaching the screen, as a fraction of today's look");
    println!("(8 twigs per row; worst and mean over them. 1.00 = exactly as present as today)\n");
    println!("  {:>18}  {:>5}  {:>11}  {:>11}  {:>11}  {:>11}", "style", "zoom", "peak worst", "peak mean", "area worst", "area mean");
    for zoom in [2, 3, 4, 8] {
        for (name, style) in EVERY_STYLE {
            let (peak, area) = twig_strength(zoom, style);
            println!(
                "  {name:>18}  {zoom:>4}x  {:>11.3}  {:>11.3}  {:>11.3}  {:>11.3}",
                worst(&peak),
                mean(&peak),
                worst(&area),
                mean(&area)
            );
        }
        println!();
    }
    // **The dial as a control**: `ink = 1.0` turns the line off and changes
    // nothing else, so what is left is the class field's own contribution.
    // Without this the ink gets the credit for both halves of the recovery.
    println!("  where the recovery comes from, at zoom 4 (area, worst of 8):");
    let (_, plain) = twig_strength(4, MagnifyStyle::Painted);
    let field_only = {
        let (spacing, n) = (8i32, 8i32);
        let _ = (spacing, n);
        ink_arm(4, 1.0)
    };
    let inked = ink_arm(4, MAGNIFY_INK);
    println!("    painted, no class field, no ink : {:.3}", worst(&plain));
    println!("    painted+ink with the ink OFF    : {field_only:.3}   (the class field alone)");
    println!("    painted+ink as it ships         : {inked:.3}   (and the ink on top)");
}

/// Painted+ink's worst area figure at zoom `zoom` with the ink dial at `ink`.
fn ink_arm(zoom: i32, ink: f32) -> f32 {
    let (spacing, n) = (8i32, 8i32);
    let (world, twigs) = twig_scene(spacing, n);
    let (sw, sh) = ((spacing * n * zoom) as u32, (64 * zoom) as u32);
    let mut bare = world.clone();
    for i in 0..n {
        for k in 0..4 {
            bare.set(i * spacing + 1 + k, 30 - k, Cell::EMPTY);
        }
    }
    bare.end_step();
    let shot = |w: &World, style: MagnifyStyle, ink: f32| {
        let mut r = Renderer::new();
        r.zoom = zoom;
        r.magnify_style = style;
        r.magnify_ink = ink;
        let mut f = vec![0u8; (sw * sh * 4) as usize];
        r.draw(w, &ParticleSystem::new(), &ChunkSet::default(), &mut f, (sw, sh), true);
        f
    };
    let without = shot(&bare, MagnifyStyle::CellArt, 1.0);
    let base = shot(&world, MagnifyStyle::CellArt, 1.0);
    let arm = shot(&world, MagnifyStyle::PaintedInk, ink);
    let area = |frame: &[u8], cx: i32, cy: i32| -> f32 {
        let (x0, x1) = (((cx - 1) * zoom).max(0), ((cx + 2) * zoom).min(sw as i32));
        let (y0, y1) = (((cy - 1) * zoom).max(0), ((cy + 2) * zoom).min(sh as i32));
        let (mut sum, mut px) = (0.0f32, 0u32);
        for sy in y0..y1 {
            for sx in x0..x1 {
                let i = (sy as usize * sw as usize + sx as usize) * 4;
                sum += (0..3).map(|k| (frame[i + k] as f32 - without[i + k] as f32).abs()).sum::<f32>() / 3.0;
                px += 1;
            }
        }
        sum / px.max(1) as f32
    };
    twigs
        .iter()
        .map(|&(cx, cy)| area(&arm, cx, cy) / area(&base, cx, cy).max(1e-3))
        .fold(f32::INFINITY, f32::min)
}

// ---------------------------------------------------------------- the scene

/// The generated outdoor world, settled, with the gnome standing on it, a fire
/// lit in the canopy beside him and a column of water falling past.
///
/// **Three moving things on purpose.** A style has to survive movement, and
/// the three kinds of movement in this engine are a sprite drawn over the
/// world (the gnome), a liquid whose every cell is part-full (the water), and
/// an emitter whose light changes the colour of its neighbours (the fire).
/// Each reaches `cell_colour` by a different route, and a look that holds for
/// one can still fail for another.
fn burning_scene(zoom: i32, frames: u64) -> (App, (i32, i32)) {
    let mut app = App::new();
    for _ in 0..frames {
        app.update();
    }
    app.renderer.zoom = zoom;
    let bounds = app.world.bounds().expect("the generated world has bounds");
    // **The whole world, not its middle third.** The first version of this
    // searched the middle third and landed in open ocean -- ten plant cells,
    // no gnome above water, and a sheet of five identical pictures of the sea.
    // A scene that does not contain the thing being judged reads as a style
    // that does nothing, which is `CLAUDE.md`'s scene-error trap exactly.
    let mut best = (bounds.min_x, 0usize, bounds.min_y);
    for x in (bounds.min_x..=bounds.max_x).step_by(16) {
        let (mut plants, mut top) = (0usize, None);
        for y in bounds.min_y..=bounds.max_y {
            let c = app.world.get(x, y);
            if app.world.materials.kind(c.material) == material::MaterialKind::Plant {
                plants += 1;
                top.get_or_insert(y);
            }
        }
        if plants > best.1 {
            best = (x, plants, top.unwrap_or(bounds.min_y));
        }
    }
    assert!(best.1 > 0, "the generated world grew no plants -- this scene cannot show a style on a stand");
    let (span_x, span_y) = (VIEW_W as i32 / zoom, VIEW_H as i32 / zoom);
    let (cx, canopy) = (best.0, best.2);
    // The canopy a third of the way down the view, so the ground and the sky
    // are both in it.
    app.renderer.camera_x = (cx - span_x / 2).clamp(bounds.min_x, bounds.max_x - span_x + 1);
    app.renderer.camera_y = (canopy - span_y / 3).clamp(bounds.min_y, bounds.max_y - span_y + 1);
    // The gnome, on the first ground **below the canopy** -- searching from
    // the top of the world instead finds a cliff top somewhere else entirely.
    let ground = (canopy..=bounds.max_y)
        .find(|&y| {
            matches!(
                app.world.materials.kind(app.world.get(cx, y).material),
                material::MaterialKind::Solid | material::MaterialKind::Powder
            )
        })
        .unwrap_or(canopy + 8);
    let sx = (cx - app.renderer.camera_x) * zoom;
    let sy = (ground - 3 - app.renderer.camera_y) * zoom;
    app.summon_player(sx, sy);
    // **Fire on an actual plant cell**, found rather than aimed. Aiming it at
    // the canopy's top-left corner plus a couple of cells lit nothing at all
    // (`0 cells alight in view`, printed below and the reason the counter is
    // printed) -- a crown is mostly air, so a guessed point lands in the gaps.
    let fire_at = (canopy..canopy + span_y / 2)
        .flat_map(|y| (cx..cx + span_x / 3).map(move |x| (x, y)))
        .find(|&(x, y)| app.world.materials.kind(app.world.get(x, y).material) == material::MaterialKind::Plant)
        .unwrap_or((cx, canopy));
    app.world.ignite_circle(fire_at.0, fire_at.1, 3);
    app.world.end_step();
    // Let it take hold before anything is captured: an ignition that has not
    // yet spread is one frame of nothing.
    for _ in 0..40 {
        app.update();
    }
    // **The counters the picture cannot give**, and they are the difference
    // between "the scene is right" and "the scene looks plausible": a gnome
    // who never spawned, a fire that never lit and a stand of ten cells all
    // photograph identically at contact-sheet size.
    let alight = view_burning(&app, zoom);
    println!(
        "scene: camera ({}, {}) zoom {zoom}x | {} plant cells in the chosen column at x={cx} | gnome {} at ({cx}, {}) | {alight} cells alight in view",
        app.renderer.camera_x,
        app.renderer.camera_y,
        best.1,
        if app.world.player.is_some() { "standing" } else { "MISSING" },
        ground - 3
    );
    (app, (cx, ground))
}

/// Burning cells **inside the viewport**, not in the world: what a card has to
/// say is whether there is fire in the picture, and a world-wide census would
/// report a blaze off-screen as one on it.
fn view_burning(app: &App, zoom: i32) -> usize {
    let (vx, vy) = (app.renderer.camera_x, app.renderer.camera_y);
    (vy..vy + VIEW_H as i32 / zoom)
        .flat_map(|y| (vx..vx + VIEW_W as i32 / zoom).map(move |x| (x, y)))
        .filter(|&(x, y)| app.world.get(x, y).is_burning())
        .count()
}


/// Copy `crop` out of a `VIEW_W x VIEW_H` RGBA frame into `img` at `(ox, oy)`,
/// scaled up by an integer `up`.
///
/// **Nearest-neighbour, never a filter.** Every pixel here already *is* a
/// style's decision about a sub-cell; smoothing it on the way to the card
/// would put a gradient in the picture that the thing being judged does not
/// have, which is the one mistake a style card cannot afford.
fn blit(img: &mut image::RgbaImage, frame: &[u8], crop: (u32, u32, u32, u32), up: u32, ox: u32, oy: u32) {
    let (cx, cy, cw, ch) = crop;
    for y in 0..ch {
        for x in 0..cw {
            let i = (((cy + y).min(VIEW_H - 1) * VIEW_W + (cx + x).min(VIEW_W - 1)) * 4) as usize;
            let px = image::Rgba([frame[i], frame[i + 1], frame[i + 2], 255]);
            for dy in 0..up {
                for dx in 0..up {
                    img.put_pixel(ox + x * up + dx, oy + y * up + dy, px);
                }
            }
        }
    }
}

// ---------------------------------------------------------------- frame cost

fn cost_table(zoom: i32, settle: u64) {
    let (mut app, _) = burning_scene(zoom, settle);
    let mut frame = vec![0u8; (VIEW_W * VIEW_H * 4) as usize];
    // **Interleaved inside one run, and alternating**, because a cross-run
    // number on this container is worthless: the same fixed binary has been
    // measured at 1.45–2.21 ms across seven runs. Each arm gets the same world
    // at the same tick, so the only thing varying is the style.
    const REPS: usize = 9;
    let mut moving = vec![Vec::new(); EVERY_STYLE.len()];
    let mut settled = vec![Vec::new(); EVERY_STYLE.len()];
    for _ in 0..REPS {
        for (i, (_, style)) in EVERY_STYLE.iter().enumerate() {
            app.renderer.magnify_style = *style;
            // A forced full redraw: the worst case, and what a moving frame
            // pays when the camera moves (which is every frame the gnome
            // walks — a camera move invalidates every pixel).
            let t = std::time::Instant::now();
            app.renderer.draw(&app.world, &app.particles, &ChunkSet::default(), &mut frame, (VIEW_W, VIEW_H), true);
            moving[i].push(t.elapsed().as_secs_f64() * 1e3);
            // **And a settled one**: nothing touched, nothing moving. This is
            // the state the dirty-rect skip exists for, and a per-pixel style
            // is exactly the kind of change that can quietly take its winnings
            // back. The number to read is how many pixels it recomputed.
            let t = std::time::Instant::now();
            let n = app.renderer.draw(&app.world, &app.particles, &ChunkSet::default(), &mut frame, (VIEW_W, VIEW_H), false);
            settled[i].push((t.elapsed().as_secs_f64() * 1e3, n));
        }
    }
    let px = (VIEW_W * VIEW_H) as f64;
    println!("\nfull redraw at zoom {zoom}x, {REPS} alternating reps inside one run");
    println!("  (the scene is deliberately busy -- gnome, fire and falling water -- so the");
    println!("   settled columns below are measured on a separate quiet world, see the note)");
    println!("  {:>18}  {:>9}  {:>9}  {:>10}  {:>14}  {:>12}", "style", "best ms", "median", "ns/pixel", "settled ms", "settled px");
    for (i, (name, _)) in EVERY_STYLE.iter().enumerate() {
        let mut m = moving[i].clone();
        m.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
        let best = m[0];
        let median = m[m.len() / 2];
        let quiet = settled[i].iter().map(|(t, _)| *t).fold(f64::INFINITY, f64::min);
        // **The number that says whether the skip still works.** A settled
        // world must recompute *zero* pixels; anything else means the style is
        // forcing repaints and the cost above is what every frame pays.
        let recomputed = settled[i].iter().map(|(_, n)| *n).max().unwrap_or(0);
        println!("  {name:>18}  {best:>9.3}  {median:>9.3}  {:>10.1}  {quiet:>14.3}  {recomputed:>12}", best * 1e6 / px);
    }
    let base = moving[0].iter().copied().fold(f64::INFINITY, f64::min);
    println!("\n  extra over today, best of {REPS}:");
    for (i, (name, _)) in EVERY_STYLE.iter().enumerate().skip(1) {
        let arm = moving[i].iter().copied().fold(f64::INFINITY, f64::min);
        println!("    {name:>16}  {:+.3} ms  {:+.1} ns/px", arm - base, (arm - base) * 1e6 / px);
    }
    settled_skip(zoom);
}

/// **Does a settled screen still redraw nothing?**
///
/// This has to be asked on its own world, and the first version of this file
/// asked it on the scene above and got a meaningless answer: a gnome walking,
/// a fire burning and water falling keep the dirty region at the whole frame
/// under *every* style, today's included, so all five arms read 163,840 pixels
/// recomputed and the table said nothing about any of them. What a per-pixel
/// style threatens is the skip, and the skip only does its work on a world
/// where nothing is moving -- `CLAUDE.md`'s animated-grain lesson exactly:
/// *measure a cost against the state the optimisation exists for*.
///
/// So: a static world with no liquid and no glow in it (both animate on their
/// own and would defeat the skip for reasons that are not the style's), drawn
/// once to prime and then again with nothing touched.
fn settled_skip(zoom: i32) {
    let (w, h) = (256i32, 192i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    let wood = world.materials.id_of("wood").expect("wood");
    for x in 0..w {
        let top = 90 + (x / 3) % 40;
        for y in top..h {
            let m = if y > top + 20 { material::STONE } else { material::SAND };
            world.set(x, y, Cell::new(m, ((x * 7 + y * 13) % 4) as u8));
        }
        if x % 11 == 0 {
            for k in 0..12 {
                world.set(x + k % 3, top - 1 - k, Cell::new(wood, 0));
            }
        }
    }
    world.end_step();
    let particles = ParticleSystem::new();
    println!("\nsettled world, nothing touched -- pixels recomputed on the second draw:");
    println!("  {:>18}  {:>12}  {:>12}", "style", "recomputed", "ms");
    for (name, style) in EVERY_STYLE {
        let mut r = Renderer::new();
        r.zoom = zoom;
        r.magnify_style = style;
        let mut frame = vec![0u8; (VIEW_W * VIEW_H * 4) as usize];
        r.draw(&world, &particles, &ChunkSet::default(), &mut frame, (VIEW_W, VIEW_H), true);
        let t = std::time::Instant::now();
        let n = r.draw(&world, &particles, &ChunkSet::default(), &mut frame, (VIEW_W, VIEW_H), false);
        println!("  {name:>18}  {n:>12}  {:>12.3}", t.elapsed().as_secs_f64() * 1e3);
    }
}

// ---------------------------------------------------------------- the card

#[allow(clippy::too_many_arguments)]
fn gif(zoom: i32, settle: u64, frames: u64, every: u64, style: MagnifyStyle, pair: bool, crop: (u32, u32, u32, u32), up: u32, png_dir: Option<&str>, out: &str, delay_ms: u64) {
    let (mut app, _) = burning_scene(zoom, settle);
    app.renderer.magnify_style = style;
    let (cw, ch) = (crop.2 * up, crop.3 * up);
    let (w, h) = if pair { (cw * 2 + 4, ch) } else { (cw, ch) };
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    let mut a = vec![0u8; (VIEW_W * VIEW_H * 4) as usize];
    let mut b = vec![0u8; (VIEW_W * VIEW_H * 4) as usize];
    let mut burning = 0usize;
    for f in 0..frames {
        // The gnome walks the whole time — the camera follows him, so every
        // frame is a full redraw and the style is being judged on the frames
        // that actually pay for it.
        app.player_input.right = true;
        app.update();
        if f % every != 0 {
            continue;
        }
        burning = burning.max(view_burning(&app, zoom));
        app.renderer.magnify_style = if pair { MagnifyStyle::CellArt } else { style };
        app.renderer.draw(&app.world, &app.particles, &ChunkSet::default(), &mut a, (VIEW_W, VIEW_H), true);
        let mut img = image::RgbaImage::new(w, h);
        blit(&mut img, &a, crop, up, 0, 0);
        if pair {
            app.renderer.magnify_style = style;
            app.renderer.draw(&app.world, &app.particles, &ChunkSet::default(), &mut b, (VIEW_W, VIEW_H), true);
            blit(&mut img, &b, crop, up, cw + 4, 0);
        }
        // **A frame sequence as well as the GIF, and it is the one to post.**
        // `.claude/skills/review/SKILL.md`, from a head-to-head test on one
        // card: the sequence played for the owner and the GIF did not, though
        // the GIF was valid by every check available on the posting side. Two
        // arms in two directories so `review.py ab` can step both to the same
        // instant, which a side-by-side GIF cannot be scrubbed to.
        if let Some(dir) = png_dir {
            let n = shots.len();
            let mut one = image::RgbaImage::new(cw, ch);
            blit(&mut one, &a, crop, up, 0, 0);
            one.save(format!("{dir}/a_{n:03}.png")).expect("writes arm A");
            if pair {
                let mut two = image::RgbaImage::new(cw, ch);
                blit(&mut two, &b, crop, up, 0, 0);
                two.save(format!("{dir}/b_{n:03}.png")).expect("writes arm B");
            }
        }
        shots.push(img);
    }
    // **The counter beside the picture**, `CLAUDE.md`'s own card rule: a fire
    // that never lit and a fire that is out look the same in a thumbnail, and
    // only the number says which this is.
    println!("captured {} frames, most cells burning at once: {burning}", shots.len());
    let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(delay_ms));
    let gif_frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
    match std::fs::File::create(out) {
        Ok(file) => {
            let mut encoder = image::codecs::gif::GifEncoder::new(file);
            if let Err(e) = encoder.set_repeat(image::codecs::gif::Repeat::Infinite) {
                eprintln!("magnify: gif repeat failed: {e}");
            }
            if let Err(e) = encoder.encode_frames(gif_frames) {
                eprintln!("magnify: gif encode failed: {e}");
            } else {
                println!("wrote {out}");
            }
        }
        Err(e) => eprintln!("magnify: could not create {out}: {e}"),
    }
}

/// One still per style, same world, same frame — the sheet that says what the
/// styles do to *this* scene rather than to lane T's crop.
///
/// `crop=x,y,w,h` (in screen pixels) and `up=N` because the review page scales
/// client-side and a 512-wide tile of a 3x view is smaller than the 700–950 px
/// the owner has actually been able to judge things at.
fn sheet(zoom: i32, settle: u64, crop: (u32, u32, u32, u32), up: u32, out: &str) {
    let (mut app, _) = burning_scene(zoom, settle);
    let gap = 4u32;
    let (cw, ch) = (crop.2 * up, crop.3 * up);
    let img_w = cw * 2 + gap;
    let rows = EVERY_STYLE.len().div_ceil(2) as u32;
    let mut img = image::RgbaImage::new(img_w, ch * rows + gap * (rows - 1));
    let mut frame = vec![0u8; (VIEW_W * VIEW_H * 4) as usize];
    for (i, (name, style)) in EVERY_STYLE.iter().enumerate() {
        app.renderer.magnify_style = *style;
        app.renderer.draw(&app.world, &app.particles, &ChunkSet::default(), &mut frame, (VIEW_W, VIEW_H), true);
        let (col, row) = ((i % 2) as u32, (i / 2) as u32);
        blit(&mut img, &frame, crop, up, col * (cw + gap), row * (ch + gap));
        println!("  pane {i} ({}, {}): {name}", col, row);
    }
    img.save(out).expect("writes the sheet");
    println!("wrote {out} at {}x{}", img.width(), img.height());
}

fn main() {
    let mode: String = arg("mode").unwrap_or_else(|| "twigs".to_string());
    let zoom: i32 = arg("zoom").unwrap_or(3).clamp(2, 8);
    let settle: u64 = arg("settle").unwrap_or(600);
    let frames: u64 = arg("frames").unwrap_or(300);
    let every: u64 = arg("every").unwrap_or(4);
    let delay: u64 = arg("delay").unwrap_or(80);
    let pair: u32 = arg("pair").unwrap_or(0);
    let style = style_of(&arg::<String>("style").unwrap_or_else(|| "painted_ink".to_string()));
    let notch = arg::<String>("notch").unwrap_or_else(|| "deep".to_string());
    let out: String = arg("out").unwrap_or_else(|| "/tmp/magnify.gif".to_string());
    let up: u32 = arg("up").unwrap_or(1).max(1);
    let png_dir: Option<String> = arg("png_dir");
    if let Some(d) = &png_dir {
        std::fs::create_dir_all(d).expect("makes the frame directory");
    }
    let crop: (u32, u32, u32, u32) = arg::<String>("crop")
        .and_then(|s| {
            let v: Vec<u32> = s.split(',').filter_map(|p| p.trim().parse().ok()).collect();
            (v.len() == 4).then(|| (v[0], v[1], v[2].min(VIEW_W), v[3].min(VIEW_H)))
        })
        .unwrap_or((0, 0, VIEW_W, VIEW_H));
    // **The harness echoes its own parameters**, `CLAUDE.md`'s own rule after
    // a 3.5-hour study came back as three populations wearing 24 logs: a knob
    // nobody can see the value of is a knob nobody can tell is disconnected.
    println!("magnify: mode={mode} zoom={zoom}x settle={settle} frames={frames} every={every} style={style:?} notch={notch} pair={pair} crop={crop:?} up={up}");
    let _ = NotchRule::default();
    match mode.as_str() {
        "cost" => cost_table(zoom, settle),
        "gif" => gif(zoom, settle, frames, every, style, pair != 0, crop, up, png_dir.as_deref(), &out, delay),
        "sheet" => sheet(zoom, settle, crop, up, &out),
        _ => twig_table(),
    }
}
