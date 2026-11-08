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

/// Blend a cell toward `c` over whatever is already drawn there.
fn blend_cell(frame: &mut [u8], x: i32, y: i32, c: [u8; 3], a: f32) {
    if x < 0 || y < 0 || x as usize >= W || y as usize >= H {
        return;
    }
    let p = ((y as usize * SCALE) * FW + x as usize * SCALE) * 4;
    let cur = [frame[p], frame[p + 1], frame[p + 2]];
    cell(frame, x as usize, y as usize, mix(cur, c, a));
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
    (3.0 + (g.mould.reserve / 40.0).sqrt() + 0.4 * pulse).clamp(3.0, 6.0) as i32
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
                // Water the player has just put down glistens, so a stroke
                // reads as fresh, and fades as it dries back.
                let fresh = w - g.wet_floor[i];
                if fresh > 0.05 {
                    col = mix(col, [70, 135, 230], 0.55 * fresh);
                    if fresh > 0.3 && (x * 31 + y * 17 + t as usize / 5).is_multiple_of(29) {
                        col = [205, 232, 255];
                    }
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
    // **The mould** (redrawn 2026-11-08). The first drawing was a 1-2 cell
    // yellow line that the playtest read as "a painted road", with fed,
    // starving and dead all but the same colour. Now every tube is at least
    // three cells wide, thicker with flow, with a glow round the fed ones,
    // and its state is its colour: bright pulsing yellow when fed and wet,
    // straw when starving and dry (the colour that says "this one burns"),
    // grey-brown when cut off from the core, and a grey husk where a tube
    // has died. All of it is drawn only: the tube's cells, which the fire
    // and the cap read, are still the one-cell path.
    let mould = &g.mould;
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let h = mould.husk[idx(x as usize, y as usize)];
            if h > 0 && g.air.is_open(x, y) {
                let a = 0.75 * h as f32 / super::mould::HUSK_TICKS as f32;
                for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                    if g.air.is_open(x + dx, y + dy) {
                        blend_cell(frame, x + dx, y + dy, [110, 108, 112], a);
                    }
                }
            }
        }
    }
    let wet_of = |e: &super::mould::Edge| e.fed > super::mould::FED_WET || e.d > super::mould::THICK_WET;
    // The glow under the fed tubes first, so the tubes sit on it.
    for e in mould.edges.iter().filter(|e| e.alive && e.rooted && wet_of(e)) {
        let r = if e.d > 1.9 { 4 } else { 3 };
        for &(x, y) in e.cells.iter().step_by(2) {
            for dy in -r..=r {
                for dx in -r..=r {
                    let d2 = dx * dx + dy * dy;
                    if d2 > r * r || d2 <= 2 || !g.air.is_open(x + dx, y + dy) {
                        continue;
                    }
                    let a = (0.10 + 0.05 * pulse) * (1.0 - d2 as f32 / (r * r + 1) as f32);
                    blend_cell(frame, x + dx, y + dy, [255, 220, 90], a);
                }
            }
        }
    }
    for e in mould.edges.iter().filter(|e| e.alive) {
        let wet = wet_of(e);
        let fed = (e.fed / super::mould::FED_WET).min(3.0) / 3.0;
        let col = if !e.rooted {
            [128, 112, 86]
        } else if wet {
            // A slow wave runs along a fed tube: food moving.
            mix([235, 190, 50], [255, 245, 150], 0.35 * fed + 0.25 * (pulse * 0.5 + 0.5))
        } else {
            [196, 158, 92]
        };
        let r = if e.d > 1.9 { 2 } else { 1 };
        for &(x, y) in &e.cells {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dy * dy > r * r + 1 {
                        continue;
                    }
                    let (cx, cy) = (x + dx, y + dy);
                    // Drawn over the rock edge too: the mould is a film on
                    // the rock, and a tube drawn only in the air beside a
                    // floor showed half its width.
                    if cx >= 0 && cy >= 0 && (cx as usize) < W && (cy as usize) < H && g.fire[idx(cx as usize, cy as usize)] == 0 {
                        // The middle of the tube a little lighter, so it reads
                        // as a round strand, not a flat stripe.
                        let c = if dx == 0 && dy == 0 { mix(col, [255, 255, 230], 0.25) } else { col };
                        cell(frame, cx as usize, cy as usize, c);
                    }
                }
            }
        }
    }
    for tip in mould.tips.iter().filter(|t| t.alive) {
        for &(x, y) in &tip.path {
            for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                if g.air.is_open(x + dx, y + dy) {
                    cell(frame, (x + dx) as usize, (y + dy) as usize, [236, 204, 80]);
                }
            }
        }
        // A tip racing on fresh water burns white with a halo: the answer
        // to the stroke, seen at once.
        let i = idx(tip.x.clamp(0, W as i32 - 1) as usize, tip.y.clamp(0, H as i32 - 1) as usize);
        let racing = g.wet[i] - g.wet_floor[i] > 0.3;
        if racing {
            for dy in -3i32..=3 {
                for dx in -3i32..=3 {
                    let d2 = dx * dx + dy * dy;
                    if d2 > 2 && d2 <= 10 {
                        blend_cell(frame, tip.x + dx, tip.y + dy, [255, 250, 200], 0.45 * (1.0 - d2 as f32 / 11.0));
                    }
                }
            }
        }
        for dy in -1..=1 {
            for dx in -1..=1 {
                if g.air.is_open(tip.x + dx, tip.y + dy) {
                    cell(frame, (tip.x + dx) as usize, (tip.y + dy) as usize, if racing { [255, 255, 245] } else { [255, 255, 190] });
                }
            }
        }
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
    // Wind streaks: a short pale line along each wisp's motion.
    for st in &g.streaks {
        let a = 0.85 * st.life as f32 / super::STREAK_LIFE as f32;
        let sp = (st.vx * st.vx + st.vy * st.vy).sqrt().max(1e-3);
        let len = (sp * 14.0).clamp(2.0, 7.0);
        for k in 0..=(len as i32) {
            let f = k as f32 / len;
            let (px, py) = (st.x - st.vx / sp * len * f, st.y - st.vy / sp * len * f);
            if g.air.is_open(px as i32, py as i32) {
                blend_cell(frame, px as i32, py as i32, [215, 235, 255], a * (1.0 - 0.7 * f));
            }
        }
    }
    // Cracks, lamps, fuses, the air shaft.
    for c in &g.cracks {
        let open = g.tick >= c.opens_at;
        if !open {
            // A crack about to open shows as a hairline that grows and
            // flickers through its last `HISS_LEAD`.
            let lead = c.opens_at - g.tick;
            if lead <= super::HISS_LEAD {
                let grow = 1.0 - lead as f32 / super::HISS_LEAD as f32;
                let mid = (c.x0 + c.x1) as f32 / 2.0;
                let half = 1.0 + grow * ((c.x1 - c.x0) as f32 / 2.0 + 3.0);
                for x in (mid - half).round() as i32..=(mid + half).round() as i32 {
                    let lit = (x + t as i32 / 4) % 4 == 0;
                    cell(frame, x as usize, (c.y + 1) as usize, if lit { [200, 110, 160] } else { [96, 52, 80] });
                }
            }
            continue;
        }
        for x in c.x0..=c.x1 {
            cell(frame, x as usize, (c.y + 1) as usize, [40, 10, 30]);
            if (x + t as i32 / 6) % 3 == 0 {
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
        if l.safety {
            // The gauze: a safety lamp's glass is caged.
            cell(frame, x - 1, y, [150, 150, 160]);
            cell(frame, x + 1, y, [150, 150, 160]);
        } else {
            cell(frame, x - 1, y, glass);
            cell(frame, x + 1, y, glass);
        }
        cell(frame, x, y + 1, [90, 80, 60]);
        if l.state == LampState::Blown {
            for k in -3i32..=3 {
                cell(frame, (x as i32 + k) as usize, (y as i32 + k) as usize, [220, 60, 50]);
                cell(frame, (x as i32 + k) as usize, (y as i32 - k) as usize, [220, 60, 50]);
            }
        }
        let label = match l.state {
            LampState::Working => format!("{}: {} CARTS", l.name, l.carts as u32),
            LampState::Choked if l.safety => format!("{}: GAS, NO DIGGING", l.name),
            LampState::Choked => "GAS!".to_string(),
            LampState::Singed => format!("SINGED {}", (l.dark_until.saturating_sub(g.tick)) / TICKS_PER_SECOND + 1),
            LampState::Blown => "LOST".to_string(),
        };
        let lw = text_width(&label);
        let lx = (l.x - lw / 2).clamp(1, W as i32 - lw - 1);
        let ly = if l.y < 80 { l.y + 4 } else { l.y - 13 };
        let lc = match l.state {
            LampState::Working => [160, 240, 130],
            LampState::Choked => [255, 140, 170],
            LampState::Singed => [255, 170, 90],
            LampState::Blown => [255, 90, 70],
        };
        rect(frame, (lx as usize).saturating_sub(1) * SCALE, (ly as usize).saturating_sub(1) * SCALE, (lw as usize + 2) * SCALE, 9 * SCALE, [12, 10, 16]);
        text(frame, lx, ly, &label, lc);
    }
    // Rings round whatever the coach is talking about.
    for &(px, py, until) in &g.pointers {
        let left = until.saturating_sub(t) as f32;
        let r = 9.0 + 3.0 * ((t as f32 / 8.0).sin());
        let a = (left / 30.0).min(1.0);
        for k in 0..48 {
            let ang = k as f32 / 48.0 * std::f32::consts::TAU;
            let (cx, cy) = (px as f32 + r * ang.cos(), py as f32 + r * ang.sin());
            blend_cell(frame, cx.round() as i32, cy.round() as i32, [255, 236, 120], 0.9 * a);
        }
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
    star_bar(g, frame, hy + 14);
    text(frame, 2, hy + 24, tool.hint(), [160, 160, 185]);
    let here = cursor.filter(|&(cx, cy)| g.air.is_open(cx, cy)).map(|(cx, cy)| band(g.air.at(cx, cy)).words());
    let right = here.unwrap_or("H HELP");
    text(frame, W as i32 - text_width(right) - 2, hy + 24, right, if here.is_some() { [200, 200, 220] } else { [130, 130, 150] });
    if g.over() {
        let msg = format!("SHIFT OVER: {} CARTS, {} STARS. R TO GO AGAIN", g.carts(), g.stars());
        let w = text_width(&msg);
        rect(frame, 0, (H * SCALE) / 2 - 14, FW, 28, [10, 8, 14]);
        text(frame, (W as i32 - w) / 2, H as i32 / 2 - 6, &msg, [255, 236, 170]);
    }
}

/// **The star bar** (owner playtest 2026-11-08: "when you're doing well it
/// needs to be clearer"). Carts so far fill a bar with the three star marks
/// on it, which light when passed; a dimmer fill ahead of it is where the
/// shift ends if the lamps keep working as they are now, so a lamp choking
/// or clearing moves it at once. The bar flashes when a star is won.
fn star_bar(g: &Game, frame: &mut [u8], y: i32) {
    let carts = g.carts();
    text(frame, 2, y + 1, &format!("CARTS {carts}"), [240, 230, 200]);
    let (x0, x1) = (54i32, 212i32);
    let max = 96.0f32;
    let at = |c: f32| x0 + ((x1 - x0) as f32 * (c / max).min(1.0)) as i32;
    let py = (y as usize + 2) * SCALE;
    rect(frame, x0 as usize * SCALE, py, (x1 - x0) as usize * SCALE, 8, [44, 40, 56]);
    // Where the shift ends at this rate.
    let left_s = super::LEVEL_TICKS.saturating_sub(g.tick) as f32 / TICKS_PER_SECOND as f32;
    let working = g.lamps.iter().filter(|l| l.state == LampState::Working).count() as f32;
    let pace = g.lamps.iter().map(|l| l.carts).sum::<f32>() + working * super::CARTS_PER_SECOND * left_s;
    let flash = g.star_won_at.is_some_and(|w| g.tick.saturating_sub(w) < 90 && (g.tick / 8).is_multiple_of(2));
    rect(frame, x0 as usize * SCALE, py + 1, (at(pace) - x0) as usize * SCALE, 6, [92, 82, 46]);
    let fill = if flash { [255, 255, 220] } else { [240, 196, 60] };
    rect(frame, x0 as usize * SCALE, py, (at(carts as f32) - x0) as usize * SCALE, 8, fill);
    for &need in super::STAR_CARTS.iter() {
        let sx = at(need as f32);
        let won = carts >= need;
        let col = if won { [255, 236, 120] } else if pace >= need as f32 { [170, 150, 90] } else { [96, 92, 108] };
        star(frame, sx, y + 3, col);
    }
    let secs = super::LEVEL_TICKS.saturating_sub(g.tick) / TICKS_PER_SECOND;
    text(frame, x1 + 8, y + 1, &format!("{}:{:02}", secs / 60, secs % 60), [200, 200, 215]);
}

/// A five-point star about 7 cells across, centred on (cx, cy) in cells.
fn star(frame: &mut [u8], cx: i32, cy: i32, col: [u8; 3]) {
    let (ox, oy) = (cx as f32 * SCALE as f32, cy as f32 * SCALE as f32);
    let (r_out, r_in) = (8.0f32, 3.4f32);
    let pts: Vec<(f32, f32)> = (0..10)
        .map(|k| {
            let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
            let r = if k % 2 == 0 { r_out } else { r_in };
            (ox + r * a.cos(), oy + r * a.sin())
        })
        .collect();
    for py in (oy - r_out) as i32..=(oy + r_out) as i32 {
        for px in (ox - r_out) as i32..=(ox + r_out) as i32 {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            // Even-odd fill.
            let mut inside = false;
            for k in 0..10 {
                let (ax, ay) = pts[k];
                let (bx, by) = pts[(k + 1) % 10];
                if (ay > fy) != (by > fy) && fx < ax + (fy - ay) / (by - ay) * (bx - ax) {
                    inside = !inside;
                }
            }
            if inside && px >= 0 && py >= 0 {
                put(frame, px as usize, py as usize, col);
            }
        }
    }
}

/// **The help card** (2026-11-08): the goal, what each key does and what
/// counts as winning, over the mine before the first tick and on H. The
/// owner's playtests of Storm and Mould: "the game needs better instruction
/// of what you're supposed to do. What your actual options are." It names
/// the mould's two facts (it eats gas, it follows wet rock) and that sparks
/// cannot clear lamp B's stall, and stops there: leading the mould in is
/// the puzzle, so the card does not say how.
pub fn help(frame: &mut [u8]) {
    const LINES: [(&str, [u8; 3]); 18] = [
        ("GOAL: KEEP GAS OFF THE TWO LAMPS.", [255, 236, 170]),
        ("A CLEAR LAMP DIGS CARTS. GAS STOPS IT.", [225, 220, 210]),
        ("WIN: 40 / 65 / 85 CARTS IN 4 MINUTES", [255, 236, 170]),
        ("     FOR 1 / 2 / 3 STARS", [225, 220, 210]),
        ("", AIR),
        ("1 BLOW   DRAG: A GUST PUSHES GAS AWAY", [210, 210, 225]),
        ("2 SPARK  CLICK: BURNS PINK GAS ONLY.", [255, 150, 200]),
        ("         FIRE AT A LAMP WRECKS IT", [210, 210, 225]),
        ("3 DAMP   DRAG: MOULD RACES ALONG IT", [210, 210, 225]),
        ("4 CUT    DRAG TO SNIP DRY MOULD AHEAD", [210, 210, 225]),
        ("         OF FIRE OR IT RUNS TO A LAMP", [210, 210, 225]),
        ("5 FUSE   CLICK: FIRES WHEN GAS IS PINK", [210, 210, 225]),
        ("", AIR),
        ("YELLOW MOULD EATS GAS AND RUNS ALONG", [240, 210, 90]),
        ("WET ROCK. SPARKS CANNOT CLEAR LAMP B", [240, 210, 90]),
        ("IN ITS DRIPPING STALL.", [240, 210, 90]),
        ("", AIR),
        ("SPACE PAUSE  R RESTART  N NEXT  H HELP", [170, 170, 190]),
    ];
    let (x0, y0, w, h) = (4usize, 0usize, W - 8, H);
    rect(frame, x0 * SCALE, y0 * SCALE, w * SCALE, h * SCALE, [10, 8, 14]);
    for k in 0..w * SCALE {
        put(frame, x0 * SCALE + k, y0 * SCALE, [167, 125, 0]);
        put(frame, x0 * SCALE + k, (y0 + h) * SCALE - 1, [167, 125, 0]);
    }
    let mut y = y0 as i32 + 4;
    for (s, c) in LINES {
        if s.is_empty() {
            y += 5;
            continue;
        }
        text(frame, x0 as i32 + 6, y, s, c);
        y += 8;
    }
    let go = "CLICK OR PRESS ANY KEY TO START";
    text(frame, (W as i32 - text_width(go)) / 2, y + 3, go, [255, 255, 255]);
}
