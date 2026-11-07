//! Drawing Fault: the shipped renderer for the world, then this game's own
//! layer on top -- the cracks, the wet cracks, the shimmer, the flashes and
//! the bar of tools.
//!
//! The world is drawn at one pixel a cell into a small buffer and scaled up
//! by `SCALE`, so the cracks can be drawn *between* cells at the real
//! resolution: a crack is an edge, not a cell, and drawing it as a cell
//! would make it look like a line of white sand.

use crate::render::{Hud, Renderer};
use crate::sim::fxhash::ChunkSet;

use super::level::{self, H, W};
use super::tips::{self, Dir};
use super::{Game, Tool};

/// Screen pixels per cell.
pub const SCALE: usize = 4;
/// Height of the tool bar under the world, in screen pixels.
pub const BAR_PX: usize = 84;
/// Text is drawn at this many screen pixels per font pixel.
const TEXT: i32 = 2;

pub fn size() -> (u32, u32) {
    ((W as usize * SCALE) as u32, (H as usize * SCALE + BAR_PX) as u32)
}

/// What the window knows that the game does not: which tool is held and
/// where the pointer is.
#[derive(Clone, Copy, Debug)]
pub struct Ui {
    pub tool: Tool,
    pub cursor: Option<(i32, i32)>,
    /// Hide the bar's numbers (for the blind A/B stills).
    pub plain: bool,
}

pub struct View {
    renderer: Renderer,
    cells: Vec<u8>,
}

const WHITE: [u8; 4] = [255, 255, 255, 255];
const WET: [u8; 4] = [90, 210, 255, 255];
const HOLE_RING: [u8; 4] = [255, 196, 110, 255];

/// The ice, recoloured for this game: a deep glacier blue rather than the
/// shipped pale one, so a white crack reads against it. Set on the world's
/// own registry by `Game::new`.
pub const ICE_PALETTE: [[u8; 4]; 4] = [[70, 128, 168, 255], [62, 118, 160, 255], [80, 138, 176, 255], [56, 110, 152, 255]];

impl View {
    pub fn new() -> Self {
        View { renderer: Renderer::new(), cells: vec![0; (W * H * 4) as usize] }
    }

    pub fn draw(&mut self, g: &Game, out: &mut [u8], ui: Ui) {
        let (ow, oh) = size();
        self.renderer.draw(&g.world, &g.particles, &ChunkSet::default(), &mut self.cells, (W as u32, H as u32), true);
        if g.shimmer {
            shimmer(g, &mut self.cells);
        }
        // Scale up.
        let row = ow as usize * 4;
        for y in 0..H as usize {
            for x in 0..W as usize {
                let src = &self.cells[(y * W as usize + x) * 4..(y * W as usize + x) * 4 + 4];
                for sy in 0..SCALE {
                    let o = (y * SCALE + sy) * row + x * SCALE * 4;
                    for sx in 0..SCALE {
                        out[o + sx * 4..o + sx * 4 + 4].copy_from_slice(src);
                    }
                }
            }
        }
        draw_cracks(g, out, ow);
        draw_marks(g, out, ow, ui);
        draw_bar(g, out, (ow, oh), ui);
    }
}

impl Default for View {
    fn default() -> Self {
        Self::new()
    }
}

/// A faint warm tint over strained ice, with a slow travelling ripple so it
/// reads as a property of the ice rather than a paint layer.
fn shimmer(g: &Game, cells: &mut [u8]) {
    let unit = g.tension_unit();
    let t = g.world.frame as f32 * 0.08;
    for y in 0..H {
        for x in 0..W {
            if !g.is_ice(x, y) {
                continue;
            }
            let s = (g.tension.at(x, y) / unit / super::TENSION_SCALE).clamp(0.0, 1.4);
            let ripple = 0.75 + 0.25 * ((x as f32 * 0.35 + y as f32 * 0.2 - t).sin());
            let a = (0.42 * s * ripple).min(0.55);
            let i = ((y * W + x) * 4) as usize;
            let warm = [255.0, 236.0, 200.0];
            for k in 0..3 {
                cells[i + k] = (cells[i + k] as f32 * (1.0 - a) + warm[k] * a) as u8;
            }
        }
    }
}

fn put(out: &mut [u8], ow: u32, x: i32, y: i32, c: [u8; 4]) {
    let oh = out.len() as i32 / (ow as i32 * 4);
    if x < 0 || y < 0 || x >= ow as i32 || y >= oh {
        return;
    }
    let i = (y as usize * ow as usize + x as usize) * 4;
    out[i..i + 4].copy_from_slice(&c);
}

fn blend(out: &mut [u8], ow: u32, x: i32, y: i32, c: [u8; 4], a: f32) {
    let oh = out.len() as i32 / (ow as i32 * 4);
    if x < 0 || y < 0 || x >= ow as i32 || y >= oh {
        return;
    }
    let i = (y as usize * ow as usize + x as usize) * 4;
    for k in 0..3 {
        out[i + k] = (out[i + k] as f32 * (1.0 - a) + c[k] as f32 * a) as u8;
    }
}

/// Every cracked edge in or near the wall: the tips' and the shipped
/// collapse's alike, since both are the same bit. White, about half a cell
/// thick, with a glow while fresh; wet cracks are blue.
fn draw_cracks(g: &Game, out: &mut [u8], ow: u32) {
    let s = SCALE as i32;
    let now = g.world.frame;
    let frosting = now < g.frost_flash;
    for y in 0..H {
        for x in 0..W {
            let c = g.world.get(x, y);
            if !c.cracked() || !g.is_ice(x, y) {
                continue;
            }
            for horizontal in [true, false] {
                let on = if horizontal { c.crack_down() } else { c.crack_right() };
                if !on {
                    continue;
                }
                let i = (y * W + x) as usize;
                let wet = if horizontal { g.tips.wet_down[i] } else { g.tips.wet_right[i] };
                let cut = if horizontal { g.tips.cut_down[i] } else { g.tips.cut_right[i] };
                let age = if cut == 0 { u64::MAX } else { now.saturating_sub(cut) };
                let colour = if wet { WET } else { WHITE };
                // The line, centred on the edge, `s / 2` thick.
                let (x0, y0, len, along_x) = if horizontal { ((x * s), (y + 1) * s, s, true) } else { ((x + 1) * s, y * s, s, false) };
                for k in 0..len {
                    for w in -1..1 {
                        let (px, py) = if along_x { (x0 + k, y0 + w) } else { (x0 + w, y0 + k) };
                        put(out, ow, px, py, colour);
                    }
                    // A soft shoulder either side: the crack catches light.
                    for w in [-2, 1] {
                        let (px, py) = if along_x { (x0 + k, y0 + w) } else { (x0 + w, y0 + k) };
                        blend(out, ow, px, py, colour, 0.45);
                    }
                    // Fresh: a wide glow that fades over a third of a second.
                    if age < 20 || (frosting && wet) {
                        let a = if age < 20 { 0.7 * (1.0 - age as f32 / 20.0) } else { 0.6 };
                        for w in [-4, -3, 2, 3] {
                            let (px, py) = if along_x { (x0 + k, y0 + w) } else { (x0 + w, y0 + k) };
                            blend(out, ow, px, py, WHITE, a);
                        }
                    }
                }
            }
        }
    }
    // Live tips: a bright spark.
    for t in &g.tips.tips {
        let (cx, cy) = (t.v.0 * s, t.v.1 * s);
        for dy in -3..=3 {
            for dx in -3..=3 {
                let d = dx * dx + dy * dy;
                if d <= 9 {
                    blend(out, ow, cx + dx, cy + dy, WHITE, if d <= 2 { 1.0 } else { 0.5 });
                }
            }
        }
    }
}

/// Drill rings, event flashes and the cursor's promise.
fn draw_marks(g: &Game, out: &mut [u8], ow: u32, ui: Ui) {
    let s = SCALE as i32;
    // A ring round each drilled hole, so a stopper reads as a stopper.
    for y in 0..H {
        for x in 0..W {
            if g.tips.is_hole(x, y) {
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    if g.is_ice(x + dx, y + dy) {
                        let (px, py) = (x * s + s / 2 + dx * (s / 2), y * s + s / 2 + dy * (s / 2));
                        for k in -1..1 {
                            put(out, ow, px + if dy == 0 { k } else { 0 }, py + if dx == 0 { k } else { 0 }, HOLE_RING);
                        }
                    }
                }
            }
        }
    }
    for &(x, y, left, c) in &g.flashes {
        let r = (14 - left as i32).max(1) * 2;
        ring(out, ow, x * s + s / 2, y * s + s / 2, r, c, left as f32 / 14.0);
    }
    let Some((cx, cy)) = ui.cursor else { return };
    if g.over() {
        return;
    }
    match ui.tool {
        Tool::Tap => {
            if let Some(v) = tips::tap_corner(g, cx, cy) {
                let (px, py) = (v.0 * s, v.1 * s);
                for k in -6..=6 {
                    blend(out, ow, px + k, py, WHITE, 0.8);
                    blend(out, ow, px, py + k, WHITE, 0.8);
                }
                // Where each half will go first.
                for d in tips::tap_dirs(g, v) {
                    let (ex, ey) = match d {
                        Dir::E => (1, 0),
                        Dir::W => (-1, 0),
                        Dir::S => (0, 1),
                        Dir::N => (0, -1),
                    };
                    for k in 7..14 {
                        blend(out, ow, px + ex * k, py + ey * k, WHITE, 0.5);
                    }
                }
            } else {
                ring(out, ow, cx * s + s / 2, cy * s + s / 2, 10, [160, 160, 160, 255], 0.6);
            }
        }
        Tool::Drill => ring(out, ow, cx * s + s / 2, cy * s + s / 2, (tips::DRILL_R * s) + 2, HOLE_RING, 0.9),
        Tool::Meltwater => ring(out, ow, cx * s + s / 2, cy * s, 10, WET, 0.9),
        Tool::Frost => {}
    }
}

fn ring(out: &mut [u8], ow: u32, cx: i32, cy: i32, r: i32, c: [u8; 4], a: f32) {
    let n = (r * 7).max(16);
    for i in 0..n {
        let th = i as f32 / n as f32 * std::f32::consts::TAU;
        let (x, y) = (cx + (th.cos() * r as f32) as i32, cy + (th.sin() * r as f32) as i32);
        blend(out, ow, x, y, c, a);
        blend(out, ow, x + 1, y, c, a * 0.6);
    }
}

/// The bar: the four tools, what the held one does, the clock, the lake,
/// and the score so far.
fn button_label(g: &Game, i: usize, t: Tool) -> String {
    match t {
        Tool::Drill => format!("{} {} x{}", i + 1, t.label(), g.drills_left),
        Tool::Tap => format!("{} {} x{}", i + 1, t.label(), g.taps_left),
        _ => format!("{} {}", i + 1, t.label()),
    }
}

/// Which tool button, if any, is under a pixel of the window's buffer --
/// the same layout `draw_bar` draws.
pub fn tool_at(g: &Game, px: i32, py: i32) -> Option<Tool> {
    let top = (H as usize * SCALE) as i32 / TEXT;
    let (x, y) = (px / TEXT, py / TEXT);
    if y < top + 3 || y >= top + 16 {
        return None;
    }
    let mut left = 4;
    for (i, t) in Tool::ALL.iter().enumerate() {
        let label = button_label(g, i, *t);
        let w = crate::hud::text_width(&label) + 8;
        if x >= left && x < left + w {
            return Some(*t);
        }
        left += w + 4;
    }
    None
}

fn draw_bar(g: &Game, out: &mut [u8], (ow, oh): (u32, u32), ui: Ui) {
    let hud = Hud::new(ow, oh, TEXT);
    let top = (H as usize * SCALE) as i32 / TEXT;
    let width = ow as i32 / TEXT;
    hud.fill_rect(out, 0, top, width, BAR_PX as i32 / TEXT, [22, 26, 34, 255]);
    let mut x = 4;
    for (i, t) in Tool::ALL.iter().enumerate() {
        let label = button_label(g, i, *t);
        let w = crate::hud::text_width(&label) + 8;
        let held = *t == ui.tool;
        hud.fill_rect(out, x, top + 3, w, 13, if held { [235, 240, 250, 255] } else { [52, 60, 76, 255] });
        hud.text(out, x + 4, top + 6, &label, if held { [20, 24, 30, 255] } else { [210, 220, 235, 255] });
        x += w + 4;
    }
    let says = if g.over() { "THE THAW IS OVER. R TO PLAY AGAIN" } else { ui.tool.says() };
    hud.text(out, 4, top + 21, says, [235, 240, 250, 255]);
    if ui.plain {
        return;
    }
    let left = super::THAW_SECONDS as f32 - g.seconds();
    let score = g.score();
    let lake_rows = level::FLOOR_Y - g.level.surface;
    let wall_rows = level::FLOOR_Y - level::WALL_TOP;
    let line = format!(
        "THAW {:>3}S   LAKE {:>2}/{} HIGH   SPILLWAY {:>3}/S (MARK {})   SLABS {}",
        left.max(0.0) as i32,
        lake_rows,
        wall_rows,
        g.flow_now() as i32,
        super::FLOW_MARK as i32,
        g.counts.slabs_freed
    );
    hud.text(out, 4, top + 31, &line, [170, 200, 230, 255]);
    let stars = "*".repeat(score.stars as usize) + &".".repeat(3 - score.stars as usize);
    let verdict = if score.burst {
        "THE WALL BURST".to_string()
    } else if g.over() {
        format!("{}   DRAINED {:.0}%   {}", stars, score.drained * 100.0, if score.mill_hit { "MILL HIT" } else { "MILL SAFE" })
    } else {
        format!("DRAINED {:.0}%   {}", score.drained * 100.0, if score.mill_hit { "MILL HIT" } else { "MILL SAFE" })
    };
    hud.text(out, width - crate::hud::text_width(&verdict) - 4, top + 6, &verdict, if score.burst { [255, 120, 110, 255] } else { [250, 220, 140, 255] });
}
