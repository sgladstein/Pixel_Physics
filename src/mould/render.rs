//! Drawing a dish. One function, used by the window and by the probe's GIFs
//! alike, so a picture from the probe is the window's own frame.
//!
//! **Tube brightness is a full replace on a fixed dark-to-bright ramp**, never
//! a blend into the dish colour (brief, and `CLAUDE.md` *A debug readout must
//! not be a function of the thing it debugs*): a thin tube and a thick one
//! read apart at phone size, and the ramp means the same thing in every
//! frame. There are two ramps, and which one a cell uses is the feeding
//! search's answer: **fed** tube is yellow, **unfed** tube is pale grey. That
//! is how a starving branch "goes pale" before it is drawn back in.

use super::{Dish, Ground, Phase, Tool, H, LAMP_R, W};
use crate::hud;

/// Pixel rows under the dish for the three text lines.
pub const HUD_ROWS: u32 = 52;

pub fn frame_size(scale: u32) -> (u32, u32) {
    (W as u32 * scale, H as u32 * scale + HUD_ROWS)
}

/// What the window knows that the dish does not.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub cursor: Option<(f32, f32)>,
    pub tool: Tool,
    pub fast: bool,
    pub paused: bool,
    /// Draw the key list when there is room.
    pub keys: bool,
}

impl Default for View {
    fn default() -> Self {
        View {
            cursor: None,
            tool: Tool::Lamp,
            fast: false,
            paused: false,
            keys: true,
        }
    }
}

const AGAR: [u8; 3] = [20, 32, 30];
const RIM: [u8; 3] = [10, 10, 14];
const RIM_EDGE: [u8; 3] = [52, 60, 66];
const SCAR: [u8; 3] = [96, 22, 18];
const FOOD: [u8; 3] = [214, 172, 104];
const OATS: [u8; 3] = [240, 222, 170];
const CUT: [u8; 4] = [255, 70, 50, 255];
const TEXT: [u8; 4] = [230, 230, 220, 255];
const DIM: [u8; 4] = [130, 136, 132, 255];
const HOT: [u8; 4] = [255, 214, 60, 255];

/// Trail at which the ramp tops out. Fixed, never the frame's own maximum.
const TRAIL_TOP: f32 = 30.0;

fn ramp(stops: &[[u8; 3]], v: f32) -> [u8; 3] {
    let v = v.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let i = (v as usize).min(stops.len() - 2);
    let f = v - i as f32;
    let (a, b) = (stops[i], stops[i + 1]);
    [0, 1, 2].map(|k| (a[k] as f32 + (b[k] as f32 - a[k] as f32) * f) as u8)
}

const FED_RAMP: [[u8; 3]; 4] = [[70, 46, 6], [176, 120, 14], [250, 206, 40], [255, 250, 210]];
const PALE_RAMP: [[u8; 3]; 3] = [[44, 46, 48], [110, 112, 112], [190, 190, 184]];

/// The colour of one dish cell, before overlays.
pub fn cell_colour(d: &Dish, i: usize) -> [u8; 3] {
    match d.ground[i] {
        Ground::Rim => {
            return RIM;
        }
        Ground::Salt => {
            // Speckle, so a salt line reads as grains, not paint.
            let h = (i as u32).wrapping_mul(2_654_435_761) >> 28;
            let k = 236 - (h as u8) * 3;
            return [k - 10, k - 4, k];
        }
        Ground::Scar => return SCAR,
        Ground::Agar => {}
    }
    let tr = d.trail[i];
    if tr > d.params.tube * 0.5 {
        let v = (1.0 + tr).ln() / (1.0 + TRAIL_TOP).ln();
        // Stretch: the tube threshold is the bottom of the ramp.
        let lo = (1.0 + d.params.tube * 0.5).ln() / (1.0 + TRAIL_TOP).ln();
        let v = (v - lo) / (1.0 - lo);
        return if d.fed[i] != u16::MAX {
            ramp(&FED_RAMP, v)
        } else {
            ramp(&PALE_RAMP, v)
        };
    }
    let l = d.light[i];
    if l > 0.0 {
        // The lamp lights bare agar only; a tube under it keeps its ramp.
        return [0, 1, 2].map(|k| (AGAR[k] as f32 + (120.0 - AGAR[k] as f32) * l * 0.6) as u8);
    }
    AGAR
}

fn put(frame: &mut [u8], fw: u32, x: i32, y: i32, c: [u8; 3]) {
    let fh = frame.len() as u32 / 4 / fw;
    if x < 0 || y < 0 || x as u32 >= fw || y as u32 >= fh {
        return;
    }
    let i = (y as usize * fw as usize + x as usize) * 4;
    frame[i..i + 3].copy_from_slice(&c);
    frame[i + 3] = 255;
}

/// A ring of radius `r` cells around a dish point, one pixel thick.
fn ring(frame: &mut [u8], fw: u32, scale: u32, (cx, cy): (f32, f32), r: f32, c: [u8; 3]) {
    let s = scale as f32;
    let rp = r * s;
    let n = (rp * 6.3).ceil().max(8.0) as usize;
    for k in 0..n {
        let a = k as f32 / n as f32 * std::f32::consts::TAU;
        put(
            frame,
            fw,
            (cx * s + a.cos() * rp) as i32,
            (cy * s + a.sin() * rp) as i32,
            c,
        );
    }
}

fn text(frame: &mut [u8], (fw, fh): (u32, u32), x: i32, y: i32, s: &str, c: [u8; 4]) {
    // `draw_text_crisp` lays out in half-pixels.
    hud::draw_text_crisp(frame, (fw, fh), (x / 2, y / 2), s, c);
}

fn text_px(s: &str) -> i32 {
    hud::text_width(s) * 2
}

pub fn draw(d: &Dish, view: &View, scale: u32, frame: &mut [u8]) {
    let (fw, fh) = frame_size(scale);
    debug_assert_eq!(frame.len(), (fw * fh * 4) as usize);
    let s = scale as usize;
    // The dish, cell by cell.
    for y in 0..H {
        for x in 0..W {
            let i = y * W + x;
            let mut c = cell_colour(d, i);
            if d.ground[i] == Ground::Rim {
                // A rim line where the dish meets the outside.
                let edge = [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    nx >= 0
                        && ny >= 0
                        && (nx as usize) < W
                        && (ny as usize) < H
                        && d.ground[ny as usize * W + nx as usize] != Ground::Rim
                });
                if edge {
                    c = RIM_EDGE;
                }
            }
            for py in 0..s {
                let row = ((y * s + py) * fw as usize + x * s) * 4;
                for px in 0..s {
                    let o = row + px * 4;
                    frame[o..o + 3].copy_from_slice(&c);
                    frame[o + 3] = 255;
                }
            }
        }
    }
    // Food: drawn over the tube so a pile is always visible. A reached pile
    // gets a bright ring: the mould is eating there.
    for p in &d.piles {
        let base = if p.oats { OATS } else { FOOD };
        let frac = (p.food / p.full).clamp(0.0, 1.0);
        let c = [0, 1, 2].map(|k| (base[k] as f32 * (0.45 + 0.55 * frac)) as u8);
        let rp = p.r * scale as f32;
        let (cx, cy) = (p.x * scale as f32, p.y * scale as f32);
        let n = rp.ceil() as i32;
        for dy in -n..=n {
            for dx in -n..=n {
                let r2 = (dx * dx + dy * dy) as f32;
                if r2 <= rp * rp {
                    // Speckled grains.
                    let h = ((cx as i32 + dx) as u32).wrapping_mul(73_856_093)
                        ^ ((cy as i32 + dy) as u32).wrapping_mul(19_349_663);
                    let g = if h.is_multiple_of(5) { 0.8 } else { 1.0 };
                    put(
                        frame,
                        fw,
                        cx as i32 + dx,
                        cy as i32 + dy,
                        c.map(|v| (v as f32 * g) as u8),
                    );
                }
            }
        }
        if p.reached() {
            ring(frame, fw, scale, (p.x, p.y), p.r + 1.5, [255, 236, 120]);
        }
        if p.oats {
            // How long the oats have left, as a shrinking arc.
            let left = (p.expires.saturating_sub(d.t)) as f32 / (super::OATS_SECONDS * super::STEPS_PER_SECOND) as f32;
            let rp = (p.r + 3.0) * scale as f32;
            let m = (rp * 6.3 * left).ceil() as usize;
            for k in 0..m {
                let a = -std::f32::consts::FRAC_PI_2 + k as f32 / (rp * 6.3) * std::f32::consts::TAU;
                put(
                    frame,
                    fw,
                    (cx + a.cos() * rp) as i32,
                    (cy + a.sin() * rp) as i32,
                    [240, 222, 170],
                );
            }
        }
    }
    // Snip marks, fading over a second.
    for &(x, y, at) in &d.snip_marks {
        let age = d.t.saturating_sub(at);
        if age < super::STEPS_PER_SECOND as u64 {
            put(
                frame,
                fw,
                (x * scale as f32) as i32,
                (y * scale as f32) as i32,
                [255, 90, 70],
            );
        }
    }
    // Storm cuts.
    // Bright for the first second, so the strike reads; the scar stays.
    if let Phase::Storm(at) = d.phase {
        if d.t < at + super::STEPS_PER_SECOND as u64 {
            for &(a, b) in &d.cuts {
                let n = ((b.0 - a.0).hypot(b.1 - a.1) * scale as f32).ceil() as usize;
                for k in 0..=n {
                    let f = k as f32 / n as f32;
                    let (x, y) = (
                        (a.0 + (b.0 - a.0) * f) * scale as f32,
                        (a.1 + (b.1 - a.1) * f) * scale as f32,
                    );
                    for o in -1..=1 {
                        put(frame, fw, x as i32 + o, y as i32, [CUT[0], CUT[1], CUT[2]]);
                        put(frame, fw, x as i32, y as i32 + o, [CUT[0], CUT[1], CUT[2]]);
                    }
                }
            }
        }
    }
    // The lamp, and the cursor's tool preview.
    if let Some(l) = d.lamp {
        ring(frame, fw, scale, l, LAMP_R, [235, 240, 255]);
        ring(frame, fw, scale, l, LAMP_R * 0.5, [150, 160, 190]);
    } else if let Some(c) = view.cursor {
        let r = match view.tool {
            Tool::Lamp => LAMP_R,
            Tool::Oats => super::OATS_R,
            Tool::Salt => 1.5,
            Tool::Snip => 2.5,
        };
        ring(frame, fw, scale, c, r, [120, 126, 140]);
    }
    hud_lines(d, view, scale, frame, (fw, fh));
}

fn stock(d: &Dish, t: Tool) -> Option<u32> {
    match t {
        Tool::Lamp => None,
        Tool::Oats => Some(d.oats_left),
        Tool::Salt => Some(d.salt_left),
        Tool::Snip => Some(d.snips_left),
    }
}

fn hud_lines(d: &Dish, view: &View, scale: u32, frame: &mut [u8], (fw, fh): (u32, u32)) {
    let top = (H as u32 * scale) as i32;
    for y in top..fh as i32 {
        for x in 0..fw as i32 {
            put(frame, fw, x, y, [12, 14, 16]);
        }
    }
    let line = |n: i32| top + 4 + n * 16;
    // Line 1: the tools.
    let mut x = 6;
    for (k, t) in Tool::ALL.iter().enumerate() {
        let label = match stock(d, *t) {
            Some(n) => format!("{} {} {}", k + 1, t.name(), n),
            None => format!("{} {}", k + 1, t.name()),
        };
        let spent = stock(d, *t) == Some(0);
        let c = if *t == view.tool {
            HOT
        } else if spent {
            DIM
        } else {
            TEXT
        };
        if *t == view.tool {
            text(frame, (fw, fh), x, line(0), &format!("[{label}]"), c);
            x += text_px(&format!("[{label}]")) + 12;
        } else {
            text(frame, (fw, fh), x, line(0), &label, c);
            x += text_px(&label) + 12;
        }
    }
    // Line 2: the state of the dish.
    let joined = d.measure();
    let status = match d.phase {
        Phase::Growing => {
            let left = d.seconds_left();
            format!(
                "STORM IN {}:{:02}  FOOD JOINED {}/{}  MOULD {}",
                left / 60,
                left % 60,
                joined.reach,
                joined.piles,
                d.walkers.len()
            )
        }
        Phase::Storm(_) => format!(
            "STORM!  FOOD JOINED {}/{}  MOULD {}",
            joined.reach,
            joined.piles,
            d.walkers.len()
        ),
        Phase::Over => {
            let s = d.score.clone().unwrap_or_default();
            let st = |b: bool| if b { "*" } else { "-" };
            format!(
                "{} STARS  REACH {}  THRIFT {}  TOUGH {}",
                s.star_count(),
                st(s.stars[0]),
                st(s.stars[1]),
                st(s.stars[2])
            )
        }
    };
    text(frame, (fw, fh), 6, line(1), &status, TEXT);
    // Line 3: what a click does now, or what to do next.
    let hint = match d.phase {
        Phase::Over => "R: TRY AGAIN   N: NEXT DISH".to_string(),
        Phase::Storm(_) => "THE STORM STRUCK THE THICKEST TUBE. WATCH.".to_string(),
        Phase::Growing => {
            let spent = stock(d, view.tool) == Some(0);
            if spent {
                format!("NO {} LEFT", view.tool.name())
            } else {
                view.tool.hint().to_string()
            }
        }
    };
    text(
        frame,
        (fw, fh),
        6,
        line(2),
        &hint,
        if d.phase == Phase::Growing { DIM } else { HOT },
    );
    if view.keys {
        let keys = if view.paused {
            "PAUSED  P: GO"
        } else if view.fast {
            "FAST  SPACE: NORMAL"
        } else {
            "SPACE: FAST  P: PAUSE  R: RESTART"
        };
        let w = text_px(keys);
        let hx = 6 + text_px(&hint) + 24;
        if hx + w + 6 < fw as i32 {
            text(frame, (fw, fh), fw as i32 - w - 6, line(2), keys, DIM);
        }
    }
    if d.phase == Phase::Over {
        score_panel(d, frame, (fw, fh), scale);
    }
}

fn score_panel(d: &Dish, frame: &mut [u8], (fw, fh): (u32, u32), scale: u32) {
    let Some(s) = d.score.clone() else { return };
    let st = |b: bool| if b { "*" } else { "-" };
    let lines = [
        format!("{} OF 3 STARS", s.star_count()),
        format!("{} REACH: FOOD JOINED {} OF {}", st(s.stars[0]), s.reach, s.piles),
        format!("{} THRIFT: NETWORK {:.1}X THE SHORTEST", st(s.stars[1]), s.ratio()),
        format!(
            "{} TOUGH: STILL JOINED AFTER STORM {} OF {}",
            st(s.stars[2]),
            s.after,
            s.reach
        ),
    ];
    let w = lines.iter().map(|l| text_px(l)).max().unwrap_or(0) + 20;
    let h = 16 * lines.len() as i32 + 14;
    // At the top of the dish, so the network the score is about stays in view.
    let (x0, y0) = ((fw as i32 - w) / 2, 2 * scale as i32);
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            put(frame, fw, x, y, [16, 18, 22]);
        }
    }
    for x in x0..x0 + w {
        put(frame, fw, x, y0, [90, 96, 104]);
        put(frame, fw, x, y0 + h - 1, [90, 96, 104]);
    }
    for (k, l) in lines.iter().enumerate() {
        text(
            frame,
            (fw, fh),
            x0 + 10,
            y0 + 8 + 16 * k as i32,
            l,
            if k == 0 { HOT } else { TEXT },
        );
    }
}
