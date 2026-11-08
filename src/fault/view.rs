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

/// A worked-out tap preview: the corner it was worked out for, the frame,
/// and the edges, `(owner x, owner y, horizontal)`.
type Preview = ((i32, i32), u64, Vec<(i32, i32, bool)>);

pub struct View {
    renderer: Renderer,
    cells: Vec<u8>,
    /// Where the tap under the cursor would run.
    preview: Option<Preview>,
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
        View { renderer: Renderer::new(), cells: vec![0; (W * H * 4) as usize], preview: None }
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
        draw_safe_line(g, out, (ow, oh));
        draw_cracks(g, out, ow);
        draw_severed(g, out, (ow, oh));
        self.draw_preview(g, out, ow, ui);
        draw_marks(g, out, ow, ui);
        draw_bar(g, out, (ow, oh), ui);
    }
}

impl View {
    /// **The tap's run, shown before the click** (2026-11-08 playtest, fix
    /// 2): an amber line where the crack would go, worked out by playing the
    /// tap on a copy of the game (`Game::trial` + `settle_cracks`) -- the
    /// planned player's own look-ahead, given to the player. Recomputed when
    /// the corner under the cursor changes, or every half second as the lake
    /// moves the strain.
    fn draw_preview(&mut self, g: &Game, out: &mut [u8], ow: u32, ui: Ui) {
        let corner = match (ui.tool, ui.cursor) {
            (Tool::Tap, Some((cx, cy))) if !g.over() && g.taps_left > 0 && g.tips.tips.is_empty() => tips::tap_corner(g, cx, cy),
            _ => None,
        };
        let Some(v) = corner else {
            self.preview = None;
            return;
        };
        let stale = self.preview.as_ref().is_none_or(|(pv, f, _)| *pv != v || g.world.frame >= f + 30);
        if stale {
            self.preview = Some((v, g.world.frame, super::preview_tap(g, v)));
        }
        let s = SCALE as i32;
        let Some((_, _, edges)) = &self.preview else { return };
        for &(x, y, horizontal) in edges {
            // A solid amber line a pixel either side of the edge: the colour,
            // not a dash, says "not cut yet" -- a dotted line at one pixel
            // was invisible at play scale.
            let (x0, y0, along_x) = if horizontal { (x * s, (y + 1) * s, true) } else { ((x + 1) * s, y * s, false) };
            for k in 0..s {
                for w in -1..1 {
                    let (px, py) = if along_x { (x0 + k, y0 + w) } else { (x0 + w, y0 + k) };
                    blend(out, ow, px, py, PREVIEW, 0.75);
                }
            }
        }
    }
}

const PREVIEW: [u8; 4] = [255, 186, 60, 255];
const SAFE: [u8; 4] = [255, 196, 70, 255];
const DANGER: [u8; 4] = [255, 110, 90, 255];
const RACE: [u8; 4] = [190, 235, 255, 255];
const CUT_FREE: [u8; 4] = [255, 226, 90, 255];

/// **The safe line** (playtest fix 1): a dashed amber line across the lake
/// and the wall, fifteen rows under the crest, named on the water. Red once
/// the lake has stood above it.
fn draw_safe_line(g: &Game, out: &mut [u8], (ow, oh): (u32, u32)) {
    let s = SCALE as i32;
    let crossed = g.lake_high < level::SAFE_LINE - 1;
    let c = if crossed { DANGER } else { SAFE };
    let py = level::SAFE_LINE * s;
    for px in 0..(level::WALL_X1 + 2) * s {
        if (px / 6) % 2 == 0 {
            put(out, ow, px, py, c);
            put(out, ow, px, py + 1, c);
        }
    }
    let hud = Hud::new(ow, oh, TEXT);
    let label = if crossed { "SAFE LINE - CROSSED" } else { "SAFE LINE - KEEP THE LAKE UNDER IT" };
    hud.text(out, 4, py / TEXT - 9, label, c);
    // And the mill race below it: the lake must not be emptied either. Red
    // only once the lake has been up past it and fallen back, or at the end:
    // it starts below it and rises.
    let low = g.level.surface > level::MILL_RACE && (g.lake_high <= level::MILL_RACE || g.over());
    let c = if low { DANGER } else { RACE };
    let py = level::MILL_RACE * s;
    for px in 0..level::WALL_X0 * s {
        if (px / 6) % 2 == 0 {
            put(out, ow, px, py, c);
            put(out, ow, px, py + 1, c);
        }
    }
    let label = if low { "MILL RACE - THE LAKE IS TOO LOW" } else { "MILL RACE - KEEP THE LAKE ABOVE IT" };
    hud.text(out, 4, py / TEXT + 3, label, c);
}

/// **A piece cut free** (playtest fix 3): outlined the moment its ring
/// closes, and a tick on the upstream face at the row the lake must reach
/// to push it -- so a tap whose payoff is forty seconds off says so now.
fn draw_severed(g: &Game, out: &mut [u8], (ow, _): (u32, u32)) {
    let s = SCALE as i32;
    let pulse = 0.55 + 0.35 * ((g.world.frame as f32) * 0.15).sin();
    for p in &g.tips.severed {
        let set: std::collections::HashSet<(i32, i32)> = p.cells.iter().copied().collect();
        for &(x, y) in &p.cells {
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                if set.contains(&(x + dx, y + dy)) {
                    continue;
                }
                // The cell's side facing out, drawn just inside it.
                for k in 0..s {
                    let (px, py) = match (dx, dy) {
                        (-1, _) => (x * s, y * s + k),
                        (1, _) => (x * s + s - 1, y * s + k),
                        (_, -1) => (x * s + k, y * s),
                        _ => (x * s + k, y * s + s - 1),
                    };
                    blend(out, ow, px, py, CUT_FREE, pulse);
                }
            }
        }
        if let Some(row) = tips::push_row(g, p) {
            let py = row * s + s / 2;
            for px in (level::WALL_X0 - 6) * s..level::WALL_X0 * s {
                put(out, ow, px, py, CUT_FREE);
                put(out, ow, px, py + 1, CUT_FREE);
            }
        }
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
    // A cut-free piece takes the hint's line: the two side by side ran into
    // each other at the window's width.
    let (says, colour) = if g.over() {
        ("THE THAW IS OVER. R TO PLAY AGAIN", [235, 240, 250, 255])
    } else if !g.tips.severed.is_empty() {
        ("CUT FREE - THE LAKE WILL PUSH IT OUT WHEN IT REACHES THE MARK", [255, 226, 90, 255])
    } else {
        (ui.tool.says(), [235, 240, 250, 255])
    };
    hud.text(out, 4, top + 21, says, colour);
    if ui.plain {
        return;
    }
    let left = super::THAW_SECONDS as f32 - g.seconds();
    let score = g.score();
    let under = g.level.surface - level::SAFE_LINE;
    let lake = if under < 0 {
        format!("LAKE {} ROWS OVER THE SAFE LINE", -under)
    } else if g.level.surface > level::MILL_RACE {
        format!("LAKE {} ROWS UNDER THE MILL RACE", g.level.surface - level::MILL_RACE)
    } else {
        format!("LAKE {under} ROWS UNDER THE SAFE LINE")
    };
    let line = format!("THAW {:>3}S   {}   SPILLWAY {:>3}/S   SLABS {}", left.max(0.0) as i32, lake, g.flow_now() as i32, g.counts.slabs_freed);
    hud.text(out, 4, top + 31, &line, [170, 200, 230, 255]);
    let stars = "*".repeat(score.stars as usize) + &".".repeat(3 - score.stars as usize);
    let held = if g.lake_high < level::SAFE_LINE - 1 {
        "LINE CROSSED"
    } else if g.level.surface > level::MILL_RACE && (g.lake_high <= level::MILL_RACE || g.over()) {
        "LAKE TOO LOW"
    } else {
        "LAKE HELD"
    };
    let mill = if score.mill_hit { "MILL HIT" } else { "MILL SAFE" };
    let verdict = if score.burst {
        "THE WALL BURST".to_string()
    } else if g.over() {
        format!("{stars}   {held}   {mill}")
    } else {
        format!("{held}   {mill}")
    };
    hud.text(out, width - crate::hud::text_width(&verdict) - 4, top + 6, &verdict, if score.burst { [255, 120, 110, 255] } else { [250, 220, 140, 255] });
}
