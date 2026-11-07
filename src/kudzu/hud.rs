//! What the probe draws: the world, the preview over it, and the bar under
//! it. The window and the headless harness both draw through [`draw_frame`],
//! so a GIF from `examples/kudzuprobe.rs` is the window's own picture.
//!
//! The world itself is the shipped renderer's picture, untouched. The
//! preview is a **full replace** on top of it, never a blend into the cell's
//! own colour (`CLAUDE.md`: a readout must not be a function of the thing it
//! reads) -- the pale is one fixed colour whatever the leaf under it was.

use crate::hud::{draw_text_scaled, GLYPH_WIDTH};
use crate::render::Renderer;

use super::{Kudzu, Tool, DIG_HOLD, HEIGHT, SNIP_CHARGES, SNIP_RADIUS, TICKS_PER_SECOND, WIDTH};

/// Pixels per world cell in the window.
pub const SCALE: u32 = 3;
/// The bar under the world, in pixels.
pub const BAR: u32 = 84;
/// The whole frame.
pub const FRAME_W: u32 = WIDTH as u32 * SCALE;
pub const FRAME_H: u32 = HEIGHT as u32 * SCALE + BAR;

/// What would fall: a pale wash, the colour of a leaf with the light behind it.
pub const PREVIEW: [u8; 4] = [240, 244, 160, 255];
/// The blade's own cells.
pub const BLADE: [u8; 4] = [255, 80, 50, 255];
/// A crown the dig would pull.
pub const CROWN: [u8; 4] = [255, 150, 40, 255];

const TEXT: [u8; 4] = [235, 235, 225, 255];
const DIM: [u8; 4] = [150, 150, 140, 255];
const GOOD: [u8; 4] = [150, 230, 120, 255];
const BAD: [u8; 4] = [255, 150, 90, 255];
const STAR: [u8; 4] = [255, 214, 70, 255];
const BAR_BG: [u8; 4] = [24, 26, 22, 255];

/// Where the pointer is, in world cells, if it is over the world.
pub type Hover = Option<(i32, i32)>;

/// Draw the world at one pixel a cell into `cells` (`WIDTH * HEIGHT * 4`).
pub fn draw_world(game: &mut Kudzu, renderer: &mut Renderer, cells: &mut [u8]) {
    let touched = game.world.take_touched_chunks();
    renderer.draw(&game.world, &game.particles, &touched, cells, (WIDTH as u32, HEIGHT as u32), true);
}

/// Paint the hover preview for a pointer at `(x, y)` over a drawn
/// cell-resolution frame. Returns how many cells it said would fall.
pub fn draw_preview(game: &Kudzu, cells: &mut [u8], x: i32, y: i32) -> usize {
    match game.tool {
        Tool::Snip => {
            let (drop, blade) = game.preview(x, y);
            for &(px, py) in &drop {
                put(cells, px, py, PREVIEW);
            }
            for &(px, py) in &blade {
                put(cells, px, py, BLADE);
            }
            drop.len()
        }
        Tool::Dig => {
            let Some(id) = game.crown_near(x, y) else { return 0 };
            let Some(s) = game.world.organism(id) else { return 0 };
            let mut n = 0;
            for &(px, py) in s.cells.keys() {
                let c = game.world.get(px, py);
                if game.world.materials.get(c.material).reinforces_powder {
                    put(cells, px, py, CROWN);
                    n += 1;
                }
            }
            n
        }
    }
}

pub fn put(cells: &mut [u8], x: i32, y: i32, c: [u8; 4]) {
    if x < 0 || y < 0 || x >= WIDTH || y >= HEIGHT {
        return;
    }
    let i = ((y * WIDTH + x) * 4) as usize;
    cells[i..i + 4].copy_from_slice(&c);
}

fn fill(frame: &mut [u8], x0: i32, y0: i32, w: i32, h: i32, c: [u8; 4]) {
    for y in y0.max(0)..(y0 + h).min(FRAME_H as i32) {
        for x in x0.max(0)..(x0 + w).min(FRAME_W as i32) {
            let i = ((y as u32 * FRAME_W + x as u32) * 4) as usize;
            frame[i..i + 4].copy_from_slice(&c);
        }
    }
}

fn text(frame: &mut [u8], x: i32, y: i32, s: &str, c: [u8; 4]) {
    // The 5x7 font at 2x, laid out in frame pixels: `draw_text_scaled`
    // takes logical coordinates and multiplies them by the scale.
    draw_text_scaled(frame, (FRAME_W, FRAME_H), 2, (x / 2, y / 2), s, c);
}

fn text_w(s: &str) -> i32 {
    s.chars().count() as i32 * (GLYPH_WIDTH + 1) * 2
}

/// A five-point-ish star, filled, `r` pixels across.
fn star(frame: &mut [u8], cx: i32, cy: i32, r: i32, c: [u8; 4]) {
    for dy in -r..=r {
        for dx in -r..=r {
            let (fx, fy) = (dx as f32 / r as f32, dy as f32 / r as f32);
            let a = fy.atan2(fx);
            let d = (fx * fx + fy * fy).sqrt();
            let k = 0.55 + 0.45 * (2.5 * (a + std::f32::consts::FRAC_PI_2)).cos().abs().powf(3.0);
            if d <= k {
                fill(frame, cx + dx, cy + dy, 1, 1, c);
            }
        }
    }
}

/// The whole window: world, preview, cursor, bar. `cells` is scratch at
/// `WIDTH * HEIGHT * 4`; `frame` is `FRAME_W * FRAME_H * 4`. Returns how many
/// cells the preview lit.
pub fn draw_frame(game: &mut Kudzu, renderer: &mut Renderer, cells: &mut [u8], frame: &mut [u8], hover: Hover) -> usize {
    draw_world(game, renderer, cells);
    let lit = match hover {
        Some((x, y)) if !game.over() => draw_preview(game, cells, x, y),
        _ => 0,
    };
    // Scale the world up, nearest-neighbour.
    for y in 0..HEIGHT as u32 * SCALE {
        let src_row = (y / SCALE) * WIDTH as u32;
        let dst_row = y * FRAME_W;
        for x in 0..FRAME_W {
            let s = ((src_row + x / SCALE) * 4) as usize;
            let d = ((dst_row + x) * 4) as usize;
            frame[d..d + 4].copy_from_slice(&cells[s..s + 4]);
        }
    }
    // The blade's reach, as a ring, so an empty snip still shows where it is.
    if let (Some((x, y)), Tool::Snip, false) = (hover, game.tool, game.over()) {
        let (cx, cy) = ((x as f32 + 0.5) * SCALE as f32, (y as f32 + 0.5) * SCALE as f32);
        let r = (SNIP_RADIUS as f32 + 0.8) * SCALE as f32;
        for i in 0..48 {
            let a = i as f32 / 48.0 * std::f32::consts::TAU;
            fill(frame, (cx + r * a.cos()) as i32, (cy + r * a.sin()) as i32, 1, 1, [255, 255, 255, 255]);
        }
    }
    draw_bar(game, frame, lit, hover);
    if game.over() {
        draw_dawn(game, frame);
    }
    lit
}

fn draw_bar(game: &Kudzu, frame: &mut [u8], lit: usize, hover: Hover) {
    let top = (HEIGHT as u32 * SCALE) as i32;
    fill(frame, 0, top, FRAME_W as i32, BAR as i32, BAR_BG);
    let (l1, l2, l3) = (top + 6, top + 32, top + 58);

    // Line 1: the tools.
    let shears = format!("1 SHEARS {}/{}", game.snips, SNIP_CHARGES);
    let next = if game.snips < SNIP_CHARGES { format!(" (+1 IN {}S)", game.snip_timer.div_ceil(TICKS_PER_SECOND)) } else { String::new() };
    let mut x = 8;
    let c = if game.tool == Tool::Snip { TEXT } else { DIM };
    text(frame, x, l1, &format!("{shears}{next}"), c);
    if game.tool == Tool::Snip {
        fill(frame, x, l1 + 16, text_w(&shears), 2, TEXT);
    }
    x += text_w(&format!("{shears}{next}")) + 28;
    let dig = format!("2 DIG OUT ROOTS {}", game.digs);
    let c = if game.tool == Tool::Dig { TEXT } else { DIM };
    text(frame, x, l1, &dig, c);
    if game.tool == Tool::Dig {
        fill(frame, x, l1 + 16, text_w(&dig), 2, TEXT);
    }
    let secs = game.left().div_ceil(TICKS_PER_SECOND as u64);
    let clock = format!("DAWN IN {}:{:02}", secs / 60, secs % 60);
    text(frame, FRAME_W as i32 - text_w(&clock) - 8, l1, &clock, TEXT);

    // Line 2: the counts beside every picture.
    let s = &game.stats;
    let clear = game.wall_clear();
    let counts = format!("CUTS {}  DROPPED {}  REGROWN {}  WALLS CLEAR {:.0}%", s.cuts, s.green_dropped, s.regrown, clear * 100.0);
    text(frame, 8, l2, &counts, TEXT);
    let (mood, c) = match game.budget() {
        None => ("VINE DEAD", GOOD),
        Some(r) if r < 1.0 => ("VINE STARVING", GOOD),
        Some(r) if r < super::RUNNER_MARGIN => ("VINE HUNGRY", TEXT),
        Some(_) => ("VINE THRIVING", BAD),
    };
    text(frame, FRAME_W as i32 - text_w(mood) - 8, l2, mood, c);

    // Line 3: what the next click does -- the owner's bar: the screen says
    // what your next action will do.
    let hint = match (game.tool, hover) {
        _ if game.over() => "R PLAY AGAIN".to_string(),
        (Tool::Snip, Some(_)) if game.snips == 0 => "SHEARS BLUNT - WAIT FOR THE NEXT CHARGE".to_string(),
        (Tool::Snip, Some(_)) if lit > 0 => format!("CLICK: {lit} CELLS OF VINE COME DOWN"),
        (Tool::Snip, Some((x, y))) if !game.blade(x, y).is_empty() => "CLICK: CUTS HERE, NOTHING ELSE LETS GO".to_string(),
        (Tool::Dig, Some((x, y))) if game.digs > 0 && game.crown_near(x, y).is_some() => match game.digging {
            Some((_, n)) => format!("DIGGING... {}%", n * 100 / DIG_HOLD),
            None => "HOLD 3S: PULL THESE ROOTS, THE WHOLE VINE DIES".to_string(),
        },
        (Tool::Dig, Some(_)) if game.digs == 0 => "NO DIGS LEFT".to_string(),
        (Tool::Dig, Some(_)) => "NO ROOTS HERE - FIND WHERE THE VINE GOES INTO THE GROUND".to_string(),
        _ => "HOVER OVER THE VINE".to_string(),
    };
    text(frame, 8, l3, &hint, if lit > 0 { PREVIEW } else { DIM });
    let keys = "R RESTART";
    text(frame, FRAME_W as i32 - text_w(keys) - 8, l3, keys, DIM);
}

fn draw_dawn(game: &Kudzu, frame: &mut [u8]) {
    let clear = game.wall_clear();
    let stars = Kudzu::stars_for(clear);
    let (w, h) = (420, 120);
    let (x0, y0) = ((FRAME_W as i32 - w) / 2, (HEIGHT * SCALE as i32 - h) / 2);
    fill(frame, x0, y0, w, h, [16, 18, 14, 255]);
    let title = "DAWN";
    text(frame, x0 + (w - text_w(title)) / 2, y0 + 12, title, TEXT);
    let line = format!("{:.0}% OF THE WALLS CLEAR", clear * 100.0);
    text(frame, x0 + (w - text_w(&line)) / 2, y0 + 40, &line, TEXT);
    for i in 0..3 {
        let c = if i < stars as i32 { STAR } else { [70, 70, 60, 255] };
        star(frame, x0 + w / 2 + (i - 1) * 50, y0 + 88, 16, c);
    }
}

/// The one-line status the harness prints.
pub fn status_line(game: &Kudzu) -> String {
    let s = &game.stats;
    format!(
        "{} | snips {}/{} | cuts {} | green dropped {} | regrown {} | wall clear {:.0}%",
        game.tool.label(),
        game.snips,
        SNIP_CHARGES,
        s.cuts,
        s.green_dropped,
        s.regrown,
        game.wall_clear() * 100.0
    )
}
