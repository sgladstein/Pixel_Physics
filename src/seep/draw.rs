//! **Drawing Seep.** One full repaint per frame into an RGBA buffer two
//! pixels per cell, with a tool bar under the mine.
//!
//! No dirty-rect skip, and that is a stated cost rather than an oversight:
//! the gas and the mould both move every tick, so there is never a settled
//! frame to skip (brief: "the mould never settles, so the dirty-rect skip is
//! lost; quote the cost"). `examples/seepprobe.rs` times this function.
//!
//! **The gas colours are a fixed ramp, not a blend into the rock**
//! (`CLAUDE.md`, "a debug readout must not be a function of the thing it
//! debugs"): pale blue, pink, deep purple, chosen by band and only their
//! opacity set by how much gas there is, so a pink cell reads pink over any
//! rock and the burn rule is readable at a glance on a phone.

use super::gas::{band, idx, Band, H, W};
use super::{Game, LampState, Tool, TICKS_PER_SECOND};
use crate::hud::draw_text_crisp;
use crate::sim::material::{self, MaterialKind};

pub const SCALE: usize = 2;
pub const BAR: usize = 66;
pub const FW: usize = W * SCALE;
pub const FH: usize = H * SCALE + BAR;

const AIR: [u8; 3] = [14, 13, 20];
const THIN_C: [u8; 3] = [120, 175, 255];
const BURN_C: [u8; 3] = [255, 92, 170];
const THICK_C: [u8; 3] = [92, 30, 150];

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
    ]
}

fn put(frame: &mut [u8], x: usize, y: usize, c: [u8; 3]) {
    if x < FW && y < FH {
        let i = (y * FW + x) * 4;
        frame[i] = c[0];
        frame[i + 1] = c[1];
        frame[i + 2] = c[2];
        frame[i + 3] = 255;
    }
}

fn cell(frame: &mut [u8], x: usize, y: usize, c: [u8; 3]) {
    for dy in 0..SCALE {
        for dx in 0..SCALE {
            put(frame, x * SCALE + dx, y * SCALE + dy, c);
        }
    }
}

fn rect(frame: &mut [u8], x0: usize, y0: usize, w: usize, h: usize, c: [u8; 3]) {
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            put(frame, x, y, c);
        }
    }
}

fn text(frame: &mut [u8], x: i32, y: i32, s: &str, c: [u8; 3]) {
    // Crisp text is laid out in half-resolution coordinates: one unit is
    // one cell here, since the buffer is two pixels per cell.
    draw_text_crisp(frame, (FW as u32, FH as u32), (x, y), s, [c[0], c[1], c[2], 255]);
}

fn text_width(s: &str) -> i32 {
    crate::hud::text_width(s)
}

fn rr_of(g: &Game, pulse: f32) -> i32 {
    (2.0 + (g.mould.reserve / 40.0).sqrt() + 0.4 * pulse).clamp(2.0, 6.0) as i32
}

pub fn render(g: &Game, frame: &mut [u8], cursor: Option<(i32, i32)>, tool: Tool) {
    let t = g.tick;
    let pulse = ((t as f32 / TICKS_PER_SECOND as f32) * std::f32::consts::TAU / 1.6).sin();
    let mats = &g.world.materials;
    for y in 0..H {
        for x in 0..W {
            let i = idx(x, y);
            let c = g.world.get(x as i32, y as i32);
            let kind = mats.kind(c.material);
            let mut col = if c.material == material::EMPTY {
                AIR
            } else {
                let pal = &mats.get(c.material).palette;
                let [r, gg, b, _] = pal[c.shade as usize % pal.len()];
                [r, gg, b]
            };
            if matches!(kind, MaterialKind::Solid | MaterialKind::Powder) {
                // Rock darkens and cools when damp: the mould's road.
                let w = g.wet[i];
                if w > 0.05 {
                    col = mix(col, [40, 62, 110], 0.8 * w);
                }
            } else if c.material == material::EMPTY {
                // Air beside damp rock gets a faint sheen so a Damp stroke is
                // visible even where the rock edge is thin.
                let w = g.wet[i];
                if w > 0.3 {
                    col = mix(col, [26, 40, 72], 0.7 * w);
                }
                let gc = g.air.c[i];
                let (bc, alpha) = match band(gc) {
                    Band::Clear => (AIR, 0.0),
                    Band::Thin => (THIN_C, 0.18 + 0.5 * (gc / super::gas::THIN)),
                    Band::Burns => (BURN_C, 0.62 + 0.3 * ((gc - super::gas::THIN) / (super::gas::THICK - super::gas::THIN))),
                    Band::Thick => (THICK_C, 0.9),
                };
                if alpha > 0.0 {
                    col = mix(col, bc, alpha);
                }
            }
            let f = g.fire[i];
            if f > 0 {
                let flick = ((x * 7 + y * 13 + t as usize * 5) % 9) as f32 / 9.0;
                col = mix([255, 120, 30], [255, 240, 150], flick * (f as f32 / 7.0));
            }
            cell(frame, x, y, col);
        }
    }
    // The mould. Thick fed tubes bright yellow, pulsing; starving tubes the
    // brown of dry straw -- the colour that says "this one will burn".
    for e in g.mould.edges.iter().filter(|e| e.alive) {
        let fed = (e.fed / super::mould::FED_WET).min(3.0) / 3.0;
        let wet = e.fed > super::mould::FED_WET || e.d > super::mould::THICK_WET;
        let base = if wet { mix([210, 170, 40], [255, 236, 90], fed) } else { [150, 110, 50] };
        let glow = if wet { 0.12 * pulse } else { 0.0 };
        let col = mix(base, [255, 255, 200], glow.max(0.0));
        let r = if e.d > 1.9 { 2 } else if e.d > 0.95 { 1 } else { 0 };
        for &(x, y) in &e.cells {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dy * dy > r * r {
                        continue;
                    }
                    let (cx, cy) = (x + dx, y + dy);
                    if g.air.is_open(cx, cy) && g.fire[idx(cx as usize, cy as usize)] == 0 {
                        cell(frame, cx as usize, cy as usize, col);
                    }
                }
            }
        }
    }
    for tip in g.mould.tips.iter().filter(|t| t.alive) {
        for &(x, y) in &tip.path {
            cell(frame, x as usize, y as usize, [230, 200, 70]);
        }
        cell(frame, tip.x as usize, tip.y as usize, [255, 255, 170]);
    }
    // The core: a blob whose size is the reserve.
    let core = &g.mould.nodes[g.mould.core];
    let rr = if g.mould_on { rr_of(g, pulse) } else { -1 };
    for dy in -rr..=rr {
        for dx in -rr..=rr {
            if dx * dx + dy * dy <= rr * rr && g.air.is_open(core.x + dx, core.y + dy) {
                cell(frame, (core.x + dx) as usize, (core.y + dy) as usize, [255, 214, 60]);
            }
        }
    }
    // Particles: the spark puffs and the blast's debris.
    for p in g.particles.iter() {
        let (x, y) = (p.x as i32, p.y as i32);
        if x >= 0 && y >= 0 && (x as usize) < W && (y as usize) < H {
            let pal = &mats.get(p.material).palette;
            let [r, gg, b, _] = pal[p.shade as usize % pal.len()];
            cell(frame, x as usize, y as usize, [r, gg, b]);
        }
    }
    // Cracks, lamps, fuses, the air shaft.
    for c in &g.cracks {
        let open = g.tick >= c.opens_at;
        for x in c.x0..=c.x1 {
            let col = if open { [40, 10, 30] } else { [70, 70, 76] };
            cell(frame, x as usize, (c.y + 1) as usize, col);
            if open && (x + t as i32 / 6) % 3 == 0 {
                cell(frame, x as usize, c.y as usize, [255, 150, 200]);
            }
        }
    }
    for f in &g.fuses {
        let blink = (t / 15).is_multiple_of(2);
        let col = if blink { [255, 170, 60] } else { [180, 90, 30] };
        cell(frame, f.x as usize, f.y as usize, col);
        cell(frame, f.x as usize, (f.y + 1) as usize, [120, 80, 40]);
    }
    for l in &g.lamps {
        let (x, y) = (l.x as usize, l.y as usize);
        let (glass, halo) = match l.state {
            LampState::Working => ([255, 230, 120], Some([80, 70, 30])),
            LampState::Choked => ([150, 120, 160], None),
            LampState::Singed => ([200, 110, 40], None),
            LampState::Blown => ([90, 40, 30], None),
        };
        if let Some(h) = halo {
            for dy in -4i32..=4 {
                for dx in -4i32..=4 {
                    let d2 = dx * dx + dy * dy;
                    if d2 <= 16 && d2 > 2 && g.air.is_open(x as i32 + dx, y as i32 + dy) {
                        let p = ((y as i32 + dy) as usize * SCALE * FW + (x as i32 + dx) as usize * SCALE) * 4;
                        let cur = [frame[p], frame[p + 1], frame[p + 2]];
                        cell(frame, (x as i32 + dx) as usize, (y as i32 + dy) as usize, mix(cur, h, 0.35 * (1.0 - d2 as f32 / 17.0) + 0.15));
                    }
                }
            }
        }
        cell(frame, x, y - 2, [60, 50, 40]);
        cell(frame, x - 1, y - 1, [90, 80, 60]);
        cell(frame, x, y - 1, [90, 80, 60]);
        cell(frame, x + 1, y - 1, [90, 80, 60]);
        cell(frame, x, y, glass);
        cell(frame, x - 1, y, glass);
        cell(frame, x + 1, y, glass);
        cell(frame, x, y + 1, [90, 80, 60]);
        if l.state == LampState::Blown {
            for k in -3i32..=3 {
                cell(frame, (x as i32 + k) as usize, (y as i32 + k) as usize, [220, 60, 50]);
                cell(frame, (x as i32 + k) as usize, (y as i32 - k) as usize, [220, 60, 50]);
            }
        }
        let label = match l.state {
            LampState::Working => format!("{}: {} CARTS", l.name, l.carts as u32),
            LampState::Choked => "GAS!".to_string(),
            LampState::Singed => format!("SINGED {}", (l.dark_until.saturating_sub(g.tick)) / TICKS_PER_SECOND + 1),
            LampState::Blown => "LOST".to_string(),
        };
        let lw = text_width(&label);
        let lx = (l.x - lw / 2).clamp(1, W as i32 - lw - 1);
        let ly = if l.y < 80 { l.y + 4 } else { l.y - 13 };
        text(frame, lx, ly, &label, [240, 230, 200]);
    }
    text(frame, 18, 2, "AIR", [170, 200, 230]);
    // Floating words.
    for f in &g.floaters {
        let age = (f.until - g.tick.min(f.until)) as i32;
        let rise = (90 - age) / 12;
        let w = text_width(&f.text);
        let x = (f.x - w / 2).clamp(1, W as i32 - w - 1);
        text(frame, x, f.y - rise, &f.text, f.colour);
    }
    // Top: the banner, or the hover readout.
    if let Some((msg, _)) = &g.banner {
        let w = text_width(msg);
        rect(frame, 0, 0, FW, 18, [10, 8, 14]);
        text(frame, (W as i32 - w) / 2, 1, msg, [255, 236, 170]);
    }
    // The key to the gas colours, top right.
    let kx = W as i32 - 92;
    for (k, (c, s)) in [(THIN_C, "TOO THIN"), (BURN_C, "BURNS"), (THICK_C, "TOO THICK")].iter().enumerate() {
        let ky = 10 + k as i32 * 8;
        rect(frame, (kx as usize) * SCALE, (ky as usize) * SCALE + 2, 10, 10, *c);
        text(frame, kx + 7, ky, s, [210, 210, 220]);
    }
    // The bar, in three rows: the tools with their meters, then the score
    // and what is under the cursor, then what the held tool will do.
    // Text here is laid out in cells (half the buffer's pixels): 42
    // characters to a line.
    let by = H * SCALE;
    rect(frame, 0, by, FW, BAR, [24, 22, 30]);
    let hy = H as i32;
    for (k, tl) in Tool::ALL.iter().enumerate() {
        let x0 = 2 + k * 102;
        let sel = *tl == tool;
        rect(frame, x0, by + 3, 98, 22, if sel { [92, 78, 36] } else { [42, 40, 54] });
        let m = g.meter(*tl);
        let mcol = if m < 0.2 { [210, 70, 60] } else { [230, 200, 80] };
        rect(frame, x0 + 2, by + 21, ((94.0 * m) as usize).min(94), 3, mcol);
        let label = if *tl == Tool::Fuse { format!("{} FUSE {}", k + 1, g.fuses_left) } else { format!("{} {}", k + 1, tl.name()) };
        text(frame, (x0 as i32 + 4) / 2, hy + 2, &label, if sel { [255, 240, 190] } else { [190, 190, 200] });
    }
    let secs = super::LEVEL_TICKS.saturating_sub(g.tick) / TICKS_PER_SECOND;
    let stars = format!("{}{}", "*".repeat(g.stars() as usize), "-".repeat(3 - g.stars() as usize));
    let status = format!("CARTS {}/{} {}:{:02} {}", g.carts(), super::GOAL_CARTS, secs / 60, secs % 60, stars);
    text(frame, 2, hy + 15, &status, [240, 230, 200]);
    if let Some((cx, cy)) = cursor {
        if g.air.is_open(cx, cy) {
            let here = band(g.air.at(cx, cy)).words();
            let w = text_width(here);
            text(frame, W as i32 - w - 2, hy + 15, here, [200, 200, 220]);
        }
    }
    text(frame, 2, hy + 24, tool.hint(), [160, 160, 185]);
    if g.over() {
        let msg = format!("SHIFT OVER: {} CARTS, {} STARS. R TO GO AGAIN", g.carts(), g.stars());
        let w = text_width(&msg);
        rect(frame, 0, (H * SCALE) / 2 - 14, FW, 28, [10, 8, 14]);
        text(frame, (W as i32 - w) / 2, H as i32 / 2 - 6, &msg, [255, 236, 170]);
    }
}
