//! **What the lab looks like at every zoom step, before and after the clamp.**
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
//! ```
//!
//! **The number beside each tile is what the picture cannot say**: how many of
//! the viewport's cells are outside the box. A tile that is three quarters
//! black and one that is a dark bed look similar at contact-sheet size, and
//! only the count separates them.

use pixel_physics::hud;
use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::render::Renderer;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

const STEPS: i32 = 3;
const GAP: u32 = 6;
const LABEL: i32 = 9;

fn main() {
    let frames: u64 = arg("frames").unwrap_or(3_000);
    let width: i32 = arg("width").unwrap_or(512);
    let height: i32 = arg("height").unwrap_or(320);
    let out: String = arg("out").unwrap_or_else(|| "labzoom.png".to_string());
    let zoom: u32 = arg("zoom").unwrap_or(2);
    println!("labzoom: frames={frames} width={width} height={height} out={out} zoom={zoom}");

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
    let bounds = lab.world.bounds();
    let start = (width / 2 - WIDTH as i32 / 2 + 40, height / 2 - HEIGHT as i32 / 2);

    let mut tiles: Vec<(String, Vec<u8>)> = Vec::new();
    for arm in 0..2 {
        let mut r = Renderer::new();
        r.set_camera(start.0, start.1, (WIDTH, HEIGHT), bounds);
        for step in 0..=STEPS {
            if step > 0 {
                if arm == 0 {
                    r.adjust_zoom(-1);
                } else {
                    r.zoom_within(-1, (WIDTH, HEIGHT), bounds);
                }
            }
            let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
            // Every tile a full redraw: the arms share no state and a
            // dirty-rect skip across a camera move would photograph the
            // previous tile.
            let touched = lab.world.take_touched_chunks();
            r.draw(&lab.world, &lab.particles, &touched, &mut buf, (WIDTH, HEIGHT), true);
            let (span_x, span_y) = r.visible_span((WIDTH, HEIGHT));
            let inside_x = (r.camera_x.max(0)..(r.camera_x + span_x).min(width)).len() as i64;
            let inside_y = (r.camera_y.max(0)..(r.camera_y + span_y).min(height)).len() as i64;
            let void = 100.0 - (inside_x * inside_y) as f32 * 100.0 / (span_x as i64 * span_y as i64) as f32;
            let centre = (r.camera_x + span_x / 2, r.camera_y + span_y / 2);
            let label = format!(
                "{} STEP {step}  VIEW {span_x}x{span_y}  VOID {void:.0}%  MID {},{}",
                if arm == 0 { "BEFORE" } else { "AFTER " },
                centre.0,
                centre.1
            );
            println!("  {label}");
            tiles.push((label, buf));
        }
    }

    // Two columns, one row per step: before on the left, after on the right,
    // so a step is read across rather than down.
    let cols = 2u32;
    let rows = STEPS as u32 + 1;
    let tile_h = HEIGHT + LABEL as u32;
    let sheet_w = cols * WIDTH + (cols + 1) * GAP;
    let sheet_h = rows * tile_h + (rows + 1) * GAP;
    let mut sheet = vec![0u8; (sheet_w * sheet_h * 4) as usize];
    for px in sheet.chunks_exact_mut(4) {
        px.copy_from_slice(&[16, 16, 20, 255]);
    }
    for (i, (label, buf)) in tiles.iter().enumerate() {
        let col = (i / (STEPS as usize + 1)) as u32;
        let row = (i % (STEPS as usize + 1)) as u32;
        let ox = GAP + col * (WIDTH + GAP);
        let oy = GAP + row * (tile_h + GAP);
        hud::draw_text(&mut sheet, sheet_w, sheet_h, ox as i32, oy as i32, label, [220, 220, 230, 255]);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let src = ((y * WIDTH + x) * 4) as usize;
                let dst = (((oy + LABEL as u32 + y) * sheet_w + ox + x) * 4) as usize;
                sheet[dst..dst + 4].copy_from_slice(&buf[src..src + 4]);
            }
        }
    }

    // Nearest-neighbour integer upscale, `labstats`' rule: the tiles carry
    // 5x7 glyphs and a smoothing resize turns one into a smear.
    let (zw, zh) = (sheet_w * zoom, sheet_h * zoom);
    let mut big = vec![0u8; (zw * zh * 4) as usize];
    for y in 0..zh {
        for x in 0..zw {
            let src = (((y / zoom) * sheet_w + (x / zoom)) * 4) as usize;
            let dst = ((y * zw + x) * 4) as usize;
            big[dst..dst + 4].copy_from_slice(&sheet[src..src + 4]);
        }
    }
    image::save_buffer(&out, &big, zw, zh, image::ColorType::Rgba8).expect("writing the sheet");
    println!("wrote {out} ({zw}x{zh}, {zoom}x)");
}
