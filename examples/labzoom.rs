//! **What a world looks like at every zoom step, before and after the clamp,
//! and at every pixel budget.**
//!
//! Owner, from play: *"you can currently zoom out farther than the lab
//! fullscreen, making the lab small in the corner and everything else is just
//! black, that shouldn't be possible... Also when you zoom, the screen should
//! stay centered where it is."*
//!
//! Both halves of that are judge-by-eye, and `labshot` cannot answer either --
//! it renders the box at its own size rather than through the 512x320
//! viewport, so the void a zoom step opens is not in its picture at all. This
//! drives the shipped `Renderer` at the real viewport and lays the two
//! controls side by side, one row per step:
//!
//! - **left column** `Renderer::adjust_zoom`, the sandbox control the lab used
//!   to call;
//! - **right column** `Renderer::zoom_within`, which derives its limit from
//!   the world's own bounds and holds the middle of the screen.
//!
//! ```text
//! cargo run --release --example labzoom
//! cargo run --release --example labzoom -- height=640 frames=3000 out=zoom.png
//! cargo run --release --example labzoom -- world=druid budget=4 zoom=4 tiles=/tmp/rung
//! cargo run --release --example labzoom -- world=druid budget=4 spend=any tiles=/tmp/any
//! ```
//!
//! **The number beside each tile is what the picture cannot say**: how many of
//! the viewport's cells are outside the world, and -- since 2026-09-13 -- how
//! many buffer pixels each logical pixel actually got.
//!
//! ## `budget=` and the rung the budget cannot reach
//!
//! `Renderer::pixel_scale_for` only ever returns a power of two that
//! **divides** the rung, so a budget of 4 is spent in full at rungs 2 and 4
//! and **not at all at rung 3** -- 3 has no power-of-two divisor above 1.
//! Walking out from close up the player gets sharp, sharp, blurry, sharp.
//! `src/app.rs` names this in a guard; this is where you can see it.
//! `PIXELS x{n}` on each tile is the scale actually in force, so a rung that
//! refused the budget says so in its own label.
//!
//! **`spend=any` draws the third option** -- the largest divisor of the rung
//! rather than the largest power-of-two divisor, so rung 3 spends 3. It needs
//! no change to `pixel_scale_for`; see the comment on the arm for why it is the
//! proposal's own output rather than a mock-up of it.
//!
//! **`world=druid` is why that matters enough to photograph.** The held world
//! is 2560x960, which caps the ladder at rung 4 -- and rung 3 spans 1536x960,
//! the first rung that shows the world's entire height and so the one a player
//! wanting to see where they are will stop at. It is also the only soft rung
//! of the four. Rung 4 is not a substitute: it overshoots vertically by 320
//! rows of void and still shows only 2048 of 2560 columns.
//!
//! ## Every tile is drawn at the same physical size, whatever its buffer
//!
//! A x4 tile holds 2048x1280 buffer pixels and a x1 tile holds 512x320, and on
//! screen `Pixels` upscales both to the *same window*. So a sheet that laid
//! them out at their buffer sizes would compare a big picture with a small one
//! and call the difference resolution. Each tile is instead upscaled by
//! `zoom / pixel_scale` into one common `512*zoom x 320*zoom` box -- exactly
//! what the window does -- which is why `zoom` must be a multiple of every
//! scale in play, and why it defaults to the budget.

use pixel_physics::druid::Druid;
use pixel_physics::hud;
use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::render::Renderer;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

const STEPS: i32 = 3;
const GAP: u32 = 6;
const LABEL: i32 = 9;
const BACKDROP: [u8; 4] = [16, 16, 20, 255];

/// Nearest-neighbour upscale of `src` into `dst` at `(ox, oy)`, to `(dst_w_px,
/// dst_h_px)`.
///
/// `labstats`' rule, and it now has a second job: the tiles carry 5x7 glyphs
/// and a smoothing resize turns one into a smear, *and* a picture drawn at x4
/// must not be shown four times larger than the x1 it is being compared with.
///
/// **Rational rather than integer**, because `spend=any` puts a x3 tile on the
/// same sheet as a x1 and a x4 one and no common size under 6144 px is an
/// integer multiple of all three. Where the ratio *is* whole this is exactly
/// the old integer path, to the pixel; where it is not, every third row and
/// column is doubled -- which is what a window does to a buffer that does not
/// divide it, and `main` says so on stdout for the tile it happens to.
#[allow(clippy::too_many_arguments)]
fn blit(dst: &mut [u8], dst_stride: u32, ox: u32, oy: u32, src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) {
    for y in 0..dst_h {
        let sy = y * src_h / dst_h;
        for x in 0..dst_w {
            let sx = x * src_w / dst_w;
            let s = ((sy * src_w + sx) * 4) as usize;
            let d = (((oy + y) * dst_stride + ox + x) * 4) as usize;
            dst[d..d + 4].copy_from_slice(&src[s..s + 4]);
        }
    }
}

fn filled(w: u32, h: u32) -> Vec<u8> {
    let mut buf = vec![0u8; (w * h * 4) as usize];
    for px in buf.chunks_exact_mut(4) {
        px.copy_from_slice(&BACKDROP);
    }
    buf
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(3_000);
    let width: i32 = arg("width").unwrap_or(512);
    let height: i32 = arg("height").unwrap_or(320);
    let out: String = arg("out").unwrap_or_else(|| "labzoom.png".to_string());
    let budget: i32 = arg("budget").unwrap_or(1);
    let zoom: u32 = arg("zoom").unwrap_or_else(|| budget.max(2) as u32);
    let which: String = arg("world").unwrap_or_else(|| "lab".to_string());
    let spend: String = arg("spend").unwrap_or_else(|| "pow2".to_string());
    let any = spend == "any";
    let tiles_to: Option<String> = arg("tiles");
    println!("labzoom: world={which} spend={spend} frames={frames} width={width} height={height} budget={budget} out={out} zoom={zoom}");

    // `arms` is 2 for the lab, where the subject is the *control* -- the
    // superseded `adjust_zoom` beside `zoom_within`. The held world calls
    // neither today, so photographing its ladder against a control it will
    // never ship would put two questions on one sheet.
    let (mut world, particles, centre, arms): (World, ParticleSystem, (i32, i32), usize) = if which == "druid" {
        let d = Druid::new();
        // Where you are, which is the whole question this world's ladder
        // raises. Falls back to the middle of the world rather than failing,
        // so a build with no spawn point still photographs.
        let c = d.world.player.as_ref().map(|p| p.center()).unwrap_or_else(|| match d.world.bounds() {
            Some(b) => ((b.min_x + b.max_x) / 2, (b.min_y + b.max_y) / 2),
            None => (0, 0),
        });
        (d.world, d.particles, c, 1)
    } else {
        // **The ground rides the height, as it does on the parameters page.**
        // Built without that, a 640-row box puts its soil surface in the top
        // quarter and 390 rows are empty stone -- which `params.rs`' own note
        // calls out, and which would photograph as a broken bed rather than a
        // tall one, in a sheet whose whole subject is what the view shows.
        let base = LabBox::default();
        let ground_y = base.ground_y * height / base.height.max(1);
        let mut lab = Lab::new(LabBox { width, height, ground_y, founders: 8, colonies: 1, ..base });
        for _ in 0..frames {
            pixel_physics::sim::frame::step(
                &mut lab.world,
                &mut lab.particles,
                &mut lab.blasts,
                pixel_physics::sim::player::PlayerInput::default(),
                &pixel_physics::sim::player::Tuning::default(),
            );
        }
        // Both arms start from the same view, put somewhere off the origin so
        // "held the middle of the screen" is a claim the picture can fail.
        (lab.world, lab.particles, (width / 2 + 40, height / 2), 2)
    };

    let bounds = world.bounds();
    let (min_x, min_y, max_x, max_y) = match bounds {
        Some(b) => (b.min_x, b.min_y, b.max_x, b.max_y),
        None => (0, 0, width - 1, height - 1),
    };

    let mut tiles: Vec<(String, u32, Vec<u8>)> = Vec::new();
    for arm in 0..arms {
        let mut r = Renderer::new();
        // The budget is pushed once and never moves, so the renderer's copy
        // and the authoritative value are the same number here -- the ordering
        // hazard `pixel_scale_for`'s doc records cannot arise.
        r.pixel_budget = budget;
        r.set_camera(centre.0 - WIDTH as i32 / 2, centre.1 - HEIGHT as i32 / 2, (WIDTH, HEIGHT), bounds);
        for step in 0..=STEPS {
            if step > 0 {
                if arms == 2 && arm == 0 {
                    r.adjust_zoom(-1);
                } else {
                    r.zoom_within(-1, (WIDTH, HEIGHT), bounds);
                }
            }
            let rung = r.zoom_out_stride.max(1);
            // **`spend=any` is the third option of the ladder question, drawn
            // without touching `pixel_scale_for`.**
            //
            // The shipped rule returns the largest *power of two* dividing the
            // rung; the proposal is the largest divisor full stop, so rung 3
            // spends 3 of a budget of 4 instead of none of it. That picture is
            // 1536x960 cells into 1536x960 buffer pixels -- `sampling_stride`
            // exactly 1 -- and the shipped renderer already draws precisely
            // that when `zoom_out_stride` is 1 and the viewport it is handed is
            // 1536x960. So the arm below is the proposal's own output, cell for
            // cell, and not a mock-up of it: the span is the same number
            // (`WIDTH * rung`), the camera is the same clamped value, the cell
            // walk is `sampling_stride` in both, and `minifying()` is false in
            // both because that stride is 1. What it cannot show is the *cost*,
            // which is the other half of §6 and is not a look question.
            let scale = if any {
                (1..=budget.max(1)).filter(|s| rung % s == 0).max().unwrap_or(1) as u32
            } else {
                r.pixel_scale().max(1) as u32
            };
            let (buf_w, buf_h) = (WIDTH * scale, HEIGHT * scale);
            let mut buf = vec![0u8; (buf_w * buf_h * 4) as usize];
            // Every tile a full redraw: the arms share no state and a
            // dirty-rect skip across a camera move would photograph the
            // previous tile.
            let touched = world.take_touched_chunks();
            let held = (r.pixel_budget, r.zoom_out_stride);
            if any {
                r.pixel_budget = 1;
                r.zoom_out_stride = rung / scale as i32;
            }
            r.draw(&world, &particles, &touched, &mut buf, (buf_w, buf_h), true);
            assert_eq!(r.sampling_stride(), rung / scale as i32, "the cell walk must be the rung the ladder says");
            (r.pixel_budget, r.zoom_out_stride) = held;
            // The span the *ladder* promises, which is what `visible_span`
            // answers for the logical viewport and is unaffected by how many
            // buffer pixels carried it.
            let (span_x, span_y) = (WIDTH as i32 * rung, HEIGHT as i32 * rung);
            let inside_x = (r.camera_x.max(min_x)..=(r.camera_x + span_x - 1).min(max_x)).count() as i64;
            let inside_y = (r.camera_y.max(min_y)..=(r.camera_y + span_y - 1).min(max_y)).count() as i64;
            let void = 100.0 - (inside_x * inside_y) as f32 * 100.0 / (span_x as i64 * span_y as i64) as f32;
            let centre_cell = (r.camera_x + span_x / 2, r.camera_y + span_y / 2);
            let label = format!(
                "{} S{step} RUNG {}  VIEW {span_x}x{span_y}  PIXELS X{scale}  VOID {void:.0}%  MID {},{}",
                if arms == 1 {
                    "HELD  "
                } else if arm == 0 {
                    "BEFORE"
                } else {
                    "AFTER "
                },
                rung,
                centre_cell.0,
                centre_cell.1
            );
            println!("  {label}");
            tiles.push((label, scale, buf));
        }
    }

    // One column per arm, one row per step: a step is read across rather than
    // down. Laid out at the *display* size, since the tiles no longer agree
    // about how many buffer pixels they hold.
    let (disp_w, disp_h) = (WIDTH * zoom, HEIGHT * zoom);
    let band_h = LABEL as u32 * zoom;
    let gap = GAP * zoom;
    let cols = arms as u32;
    let rows = STEPS as u32 + 1;
    let sheet_w = cols * disp_w + (cols + 1) * gap;
    let sheet_h = rows * (disp_h + band_h) + (rows + 1) * gap;
    let mut sheet = filled(sheet_w, sheet_h);
    for (i, (label, scale, buf)) in tiles.iter().enumerate() {
        let col = (i / (STEPS as usize + 1)) as u32;
        let row = (i % (STEPS as usize + 1)) as u32;
        let ox = gap + col * (disp_w + gap);
        let oy = gap + row * (disp_h + band_h + gap);
        // The label is drawn at 1x into its own band and upscaled with
        // everything else, so the 5x7 glyphs stay the size they have always
        // been on this sheet whatever the tile's buffer is doing.
        let mut band = filled(WIDTH, LABEL as u32);
        hud::draw_text(&mut band, WIDTH, LABEL as u32, 0, 0, label, [220, 220, 230, 255]);
        blit(&mut sheet, sheet_w, ox, oy, &band, WIDTH, LABEL as u32, WIDTH * zoom, band_h);
        blit(&mut sheet, sheet_w, ox, oy + band_h, buf, WIDTH * scale, HEIGHT * scale, disp_w, disp_h);
        if disp_w % (WIDTH * scale) != 0 {
            // **Say it rather than let a card be judged on it.** A tile whose
            // buffer does not divide the display size has every nth row and
            // column doubled, and on a sheet whose subject is sharpness that
            // is an artifact of the comparison, not of the thing compared.
            println!("  note: x{scale} tile resampled {}x{} -> {disp_w}x{disp_h} (not a whole multiple)", WIDTH * scale, HEIGHT * scale);
        }
    }
    image::save_buffer(&out, &sheet, sheet_w, sheet_h, image::ColorType::Rgba8).expect("writing the sheet");
    println!("wrote {out} ({sheet_w}x{sheet_h}, {zoom}x)");

    // **One tile per file, unlabelled**, for a blind A/B: the label names the
    // arm, and a card that tells the owner which option he is looking at is
    // not blind. Same display size as the sheet's tiles, for the same reason.
    if let Some(prefix) = &tiles_to {
        for (i, (label, scale, buf)) in tiles.iter().enumerate() {
            let arm = i / (STEPS as usize + 1);
            let step = i % (STEPS as usize + 1);
            let mut one = vec![0u8; (disp_w * disp_h * 4) as usize];
            blit(&mut one, disp_w, 0, 0, buf, WIDTH * scale, HEIGHT * scale, disp_w, disp_h);
            let path = format!("{prefix}_arm{arm}_step{step}.png");
            image::save_buffer(&path, &one, disp_w, disp_h, image::ColorType::Rgba8).expect("writing a tile");
            println!("  wrote {path}  ({label})");
        }
    }
}
