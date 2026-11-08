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

/// Pixel rows under the dish for the five text lines. Two were added on
/// 2026-11-08: the playtest review found the end-of-dish score panel, then
/// drawn over the top of the dish, covered the piles the score was about.
/// The score now has its own two lines here.
pub const HUD_ROWS: u32 = 86;

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
    /// The briefing card over the dish: the goal, the colours, the stars
    /// and the tools. The window shows it at the start of every dish, held
    /// until a key or click (owner's playtest, 2026-11-08: "I didn't really
    /// know what I was doing").
    pub brief: bool,
    /// The lesson's banner, on the first dish: step number, the two lines,
    /// and whether the step has only just begun (it flashes).
    pub banner: Option<Banner>,
    /// The dish picker (M), over everything: see `menu`.
    pub menu: Option<Menu>,
}

impl Default for View {
    fn default() -> Self {
        View {
            cursor: None,
            tool: Tool::Lamp,
            fast: false,
            paused: false,
            keys: true,
            brief: false,
            banner: None,
            menu: None,
        }
    }
}

/// One lesson step, as the banner draws it.
#[derive(Clone, Copy, Debug)]
pub struct Banner {
    pub number: u32,
    pub of: u32,
    pub lines: [&'static str; 2],
    pub fresh: bool,
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
/// A fresh snip: dark wine, fading back to agar as it heals.
const WOUND: [u8; 3] = [92, 18, 40];
/// The pulse: the colour a band blends toward, how far, how fast it rides
/// (cells a second), how far apart bands are (cells), and the fraction of
/// that spacing its tail covers.
const PULSE: [u8; 3] = [255, 252, 230];
const PULSE_STRENGTH: f32 = 0.45;
const PULSE_SPEED: f32 = 45.0;
const PULSE_SPACING: f32 = 60.0;
const PULSE_TAIL: f32 = 0.3;

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
        Ground::Wound => {
            // Raw and dark when fresh, closing back to agar over its last
            // second, so the player sees the cut heal rather than vanish.
            let left = d.wound_until[i].saturating_sub(d.t) as f32 / super::STEPS_PER_SECOND as f32;
            let f = left.clamp(0.0, 1.0);
            return [0, 1, 2].map(|k| (AGAR[k] as f32 + (WOUND[k] as f32 - AGAR[k] as f32) * f) as u8);
        }
        Ground::Agar => {}
    }
    let tr = d.trail[i];
    if tr > d.params.tube * 0.5 {
        let v = (1.0 + tr).ln() / (1.0 + TRAIL_TOP).ln();
        // Stretch: the tube threshold is the bottom of the ramp.
        let lo = (1.0 + d.params.tube * 0.5).ln() / (1.0 + TRAIL_TOP).ln();
        let v = (v - lo) / (1.0 - lo);
        return if d.looks_fed(i) {
            let c = ramp(&FED_RAMP, v);
            let b = pulse(d, i);
            if b > 0.0 {
                [0, 1, 2].map(|k| (c[k] as f32 + (PULSE[k] as f32 - c[k] as f32) * b * PULSE_STRENGTH) as u8)
            } else {
                c
            }
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

/// **The pulse: food seen moving.** A bright band rides out along every
/// fed tube from the food it comes from, one every `PULSE_SPACING /
/// PULSE_SPEED` seconds, read off the feeding search's own distances
/// (`Dish::fed`). Tube no food reaches has no distance, so a starving
/// branch's beat stops at the cut. Drawing only: the simulation never reads
/// it. (Toy-feel and research brainstorms, 2026-11-08: nothing on screen
/// showed food flowing, and real *Physarum* streams its cytoplasm in a
/// travelling wave.)
fn pulse(d: &Dish, i: usize) -> f32 {
    let dist = d.fed[i];
    if dist == u16::MAX {
        return 0.0;
    }
    let secs = d.t as f32 / super::STEPS_PER_SECOND as f32;
    let phase = (secs * PULSE_SPEED - dist as f32) / PULSE_SPACING;
    let s = phase - phase.floor();
    // Sharp at the leading edge, a tail back toward the food.
    if s < PULSE_TAIL {
        1.0 - s / PULSE_TAIL
    } else {
        0.0
    }
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
    // New food: an expanding ring for two seconds where a pile turned up,
    // and a faint dotted ring for the five seconds before one does, so a
    // pile never just appears under the player's eye unannounced.
    let sps = super::STEPS_PER_SECOND as u64;
    for &(x, y, at) in &d.arrived {
        let age = d.t.saturating_sub(at);
        if age < 2 * sps {
            let f = age as f32 / (2 * sps) as f32;
            ring(frame, fw, scale, (x, y), 6.0 + 14.0 * f, [255, 250, 220]);
        }
    }
    // Joins: where two pieces met and became one, a bright burst that
    // spreads and fades over 0.8 s.
    for &(x, y, at) in &d.snaps {
        let age = d.t.saturating_sub(at) as f32 / sps as f32;
        if age < 0.8 {
            let f = age / 0.8;
            let c = [255, 255, (230.0 * (1.0 - f) + 120.0 * f) as u8];
            ring(frame, fw, scale, (x, y), 2.0 + 10.0 * f, c);
            if f < 0.5 {
                ring(frame, fw, scale, (x, y), 1.0 + 4.0 * f, [255, 255, 255]);
            }
        }
    }
    for (at, p) in &d.pending {
        if *at <= d.t + 5 * sps && (d.t / 30).is_multiple_of(2) {
            ring(frame, fw, scale, (p.x, p.y), p.r, [120, 104, 70]);
        }
    }
    // The storm's shadow: the line it will strike, shown for the
    // `STORM_WARNING_SECONDS` before it comes, pulsing. (Added 2026-11-08:
    // the playtest found the storm read as luck because it came unannounced
    // and its target could not be guessed.)
    if d.phase == Phase::Growing {
        if let Some((a, b)) = d.storm_line {
            let on = (d.t / (sps / 4)).is_multiple_of(2);
            let c = if on { [150, 40, 150] } else { [90, 30, 96] };
            let n = ((b.0 - a.0).hypot(b.1 - a.1) * scale as f32).ceil() as usize;
            for k in 0..=n {
                // Dashed, so the tube under it still shows through.
                if (k / (2 * scale as usize)) % 2 == 1 {
                    continue;
                }
                let f = k as f32 / n as f32;
                let (x, y) = (
                    (a.0 + (b.0 - a.0) * f) * scale as f32,
                    (a.1 + (b.1 - a.1) * f) * scale as f32,
                );
                let t = scale as i32 / 2;
                for o in -t..=t {
                    put(frame, fw, x as i32 + o, y as i32, c);
                    put(frame, fw, x as i32, y as i32 + o, c);
                }
            }
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
    labels(d, scale, frame, (fw, fh));
    if d.level.is_some() {
        ruler_overlay(d, scale, frame, fw);
    }
    hud_lines(d, view, scale, frame, (fw, fh));
    if let Some(b) = view.banner {
        banner(&b, frame, (fw, fh));
    }
    if view.brief {
        match d.level {
            Some(l) => level_brief(l, frame, (fw, fh), scale),
            None => brief(view, frame, (fw, fh), scale),
        }
    }
    if let Some(m) = view.menu {
        menu(&m, frame, (fw, fh), scale);
    }
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
        if !d.has_tool(*t) {
            continue;
        }
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
        _ if d.level.is_some() => level_status(d, &joined),
        Phase::Growing if d.storm_at == u64::MAX => format!(
            "LESSON  FOOD JOINED {}/{}  MOULD {}",
            joined.reach,
            joined.piles,
            d.walkers.len()
        ),
        Phase::Growing => {
            let left = d.seconds_left();
            format!(
                "STORM IN {}:{:02}  FOOD JOINED {}/{}  MOULD {}",
                left / 60,
                left % 60,
                joined.reach,
                joined.piles + d.pending.len() as u32,
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
        Phase::Over => match &d.card {
            Some(c) => c.why.clone(),
            None => "R: TRY AGAIN   N: NEXT DISH".to_string(),
        },
        Phase::Storm(_) => "THE STORM STRUCK. WATCH WHAT STAYS JOINED.".to_string(),
        Phase::Growing if d.storm_line.is_some() => {
            format!("STORM IN {} S: IT CUTS THE PURPLE LINE", d.seconds_left())
        }
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
        let keys = if d.phase == Phase::Over {
            "R: AGAIN  N: NEXT  M: DISHES"
        } else if view.paused {
            "PAUSED  P: GO"
        } else if view.fast {
            "FAST  SPACE: NORMAL"
        } else {
            "SPACE: FAST  P: PAUSE  R: RESTART  H: HELP"
        };
        let w = text_px(keys);
        let hx = 6 + text_px(&hint) + 24;
        if hx + w + 6 < fw as i32 {
            text(frame, (fw, fh), fw as i32 - w - 6, line(2), keys, DIM);
        }
    }
    // Lines 4 and 5: what is coming while the dish grows, and the score,
    // off the dish, once it is over.
    let sps = super::STEPS_PER_SECOND as u64;
    match d.phase {
        Phase::Over if d.level.is_some() => {
            let amount = d.card.as_ref().map(|c| c.amount.clone()).unwrap_or_default();
            text(frame, (fw, fh), 6, line(3), &amount, TEXT);
            text(frame, (fw, fh), 6, line(4), "R: TRY AGAIN  N: NEXT DISH  M: DISHES", DIM);
        }
        Phase::Over => score_lines(d, frame, (fw, fh), line(3)),
        Phase::Growing => {
            let mut next = Vec::new();
            if d.oats_regrow > 0 && d.oats_left < d.oats_stock {
                let every = d.oats_regrow as u64 * sps;
                let wait = (every - d.t % every).div_ceil(sps);
                next.push(format!("OATS +1 IN {wait} S"));
            }
            if !d.pending.is_empty() {
                let at: Vec<String> = d
                    .pending
                    .iter()
                    .map(|(at, _)| format!("{}:{:02}", at / sps / 60, at / sps % 60))
                    .collect();
                next.push(format!("NEW FOOD {}", at.join(" ")));
            }
            text(frame, (fw, fh), 6, line(3), &next.join("   "), DIM);
            let stars = match d.level {
                Some(l) => l.stars_line(),
                None => "STARS: JOIN ALL, KEEP IT SHORT, SURVIVE".to_string(),
            };
            text(frame, (fw, fh), 6, line(4), &stars, DIM);
            if view.keys {
                let w = text_px("M: DISHES");
                if 6 + text_px(&stars) + 24 + w + 6 < fw as i32 {
                    text(frame, (fw, fh), fw as i32 - w - 6, line(4), "M: DISHES", DIM);
                }
            }
        }
        Phase::Storm(_) => {}
    }
}

/// Kept to about 40 characters a line, so the score reads whole in the
/// probe's half-size pictures as well as the window.
fn score_lines(d: &Dish, frame: &mut [u8], (fw, fh): (u32, u32), y: i32) {
    let Some(s) = d.score.clone() else { return };
    // Below THRIFT_MIN_JOINED there is no tree to measure and ratio() is 0,
    // which read as a perfect "0.00X" thrift.
    let thrift = if s.reach >= super::THRIFT_MIN_JOINED {
        format!("{:.2}X", s.ratio())
    } else {
        "--".to_string()
    };
    let a = format!(
        "REACH {}/{} JOINED. THRIFT {}, BAR {:.2}X",
        s.reach,
        s.piles,
        thrift,
        super::THRIFT_BAR
    );
    let b = format!("TOUGH: {} OF {} STILL JOINED AFTER STORM", s.after, s.reach);
    text(frame, (fw, fh), 6, y, &a, TEXT);
    text(frame, (fw, fh), 6, y + 16, &b, TEXT);
}

/// Words on the dish itself, where the thing they name is: the storm's
/// warning line and the scar it leaves. Owner's playtest, 2026-11-08: the
/// scar read as "a weird red rectangle ... a bug or an error".
fn labels(d: &Dish, scale: u32, frame: &mut [u8], (fw, fh): (u32, u32)) {
    let line = match d.phase {
        Phase::Growing => d.storm_line,
        _ => d.cuts.first().copied(),
    };
    let Some((a, b)) = line else { return };
    let word = match d.phase {
        Phase::Growing => "STORM WILL CUT HERE",
        _ => "STORM SCAR",
    };
    // Beside the line's lower end, nudged to stay on screen.
    let (x, y) = if a.1 > b.1 { a } else { b };
    let s = scale as f32;
    let w = text_px(word);
    let px = ((x * s) as i32 - w / 2).clamp(4, fw as i32 - w - 4);
    let py = ((y * s) as i32 + 6).clamp(4, (H as u32 * scale) as i32 - 18);
    for yy in py - 2..py + 14 {
        for xx in px - 3..px + w + 3 {
            put(frame, fw, xx, yy, [16, 12, 18]);
        }
    }
    let c = match d.phase {
        Phase::Growing => [214, 150, 230, 255],
        _ => [255, 120, 100, 255],
    };
    text(frame, (fw, fh), px, py, word, c);
}

fn panel(frame: &mut [u8], fw: u32, (x0, y0, w, h): (i32, i32, i32, i32), edge: [u8; 3]) {
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            put(frame, fw, x, y, [14, 18, 20]);
        }
    }
    for x in x0..x0 + w {
        put(frame, fw, x, y0, edge);
        put(frame, fw, x, y0 + h - 1, edge);
    }
    for y in y0..y0 + h {
        put(frame, fw, x0, y, edge);
        put(frame, fw, x0 + w - 1, y, edge);
    }
}

/// The lesson's banner, across the top of the dish.
fn banner(b: &Banner, frame: &mut [u8], (fw, fh): (u32, u32)) {
    let head = if b.number > b.of {
        "LESSON DONE".to_string()
    } else {
        format!("LESSON {} OF {}", b.number, b.of)
    };
    let w = [text_px(&head), text_px(b.lines[0]), text_px(b.lines[1])]
        .into_iter()
        .max()
        .unwrap_or(0)
        + 20;
    let x0 = (fw as i32 - w) / 2;
    let edge = if b.fresh { [255, 236, 120] } else { [120, 110, 70] };
    panel(frame, fw, (x0, 4, w, 58), edge);
    text(frame, (fw, fh), x0 + 10, 10, &head, HOT);
    text(frame, (fw, fh), x0 + 10, 26, b.lines[0], TEXT);
    text(frame, (fw, fh), x0 + 10, 42, b.lines[1], TEXT);
}

/// The briefing card: what the game is, in the order a new player needs it.
fn brief(view: &View, frame: &mut [u8], (fw, fh): (u32, u32), scale: u32) {
    let lesson = view.banner.is_some();
    let storm = if lesson {
        "A STORM COMES AT THE END.".to_string()
    } else {
        let s = super::DISH_SECONDS;
        format!("A STORM COMES AT {}:{:02}.", s / 60, s % 60)
    };
    let lines: Vec<(String, [u8; 4])> = vec![
        ("MOULD".into(), HOT),
        ("YOUR MOULD LIVES ON FOOD. JOIN THE".into(), TEXT),
        ("FOOD PILES WITH ITS TUBES.".into(), TEXT),
        (storm, TEXT),
        (String::new(), TEXT),
        ("TAN: FOOD    YELLOW: FED MOULD".into(), [214, 172, 104, 255]),
        ("GREY: CUT OFF FROM FOOD, STARVING".into(), [180, 180, 174, 255]),
        (String::new(), TEXT),
        ("STARS".into(), HOT),
        ("* JOIN ALL THE FOOD".into(), TEXT),
        ("* KEEP THE NETWORK SHORT".into(), TEXT),
        ("* LOSE NOTHING TO THE STORM".into(), TEXT),
        (String::new(), TEXT),
        ("TOOLS".into(), HOT),
        ("1 LAMP  HOLD: PUSHES THE MOULD AWAY".into(), TEXT),
        ("2 OATS  CLICK: IT GROWS TOWARD THEM".into(), TEXT),
        ("3 SALT  DRAG: A WALL IT CANNOT CROSS".into(), TEXT),
        ("4 SNIP  DRAG: CUT A TUBE".into(), TEXT),
        (String::new(), TEXT),
        ("CLICK OR PRESS A KEY TO START".into(), HOT),
    ];
    let w = lines.iter().map(|(l, _)| text_px(l)).max().unwrap_or(0) + 28;
    let h = 16 * lines.len() as i32 + 20;
    let dish_h = (H as u32 * scale) as i32;
    let (x0, y0) = ((fw as i32 - w) / 2, ((dish_h - h) / 2).max(2));
    panel(frame, fw, (x0, y0, w, h), [120, 110, 70]);
    for (k, (l, c)) in lines.iter().enumerate() {
        text(frame, (fw, fh), x0 + 14, y0 + 10 + 16 * k as i32, l, *c);
    }
}

// ------------------------------------------------- challenge dishes (level.rs)
//
// Kept to this one section and a hook each in `draw` and `hud_lines`, on the
// existing `panel` and `text`, so the interface rework can move it whole.

/// The status line on a hand-made dish: the clock to the bell and the one
/// number its win rule reads.
fn level_status(d: &Dish, joined: &super::Score) -> String {
    use super::level::{clock, Win};
    let Some(l) = d.level else { return String::new() };
    if d.phase == Phase::Over {
        let n = d.card.as_ref().map_or(0, |c| c.stars);
        return format!("{n} STAR{}  {}", if n == 1 { "" } else { "S" }, l.name);
    }
    let left = clock(d.seconds_left());
    match l.win {
        Win::JoinAll { .. } => format!(
            "TIME {left}  FOOD JOINED {}/{}  MOULD {}",
            joined.reach,
            joined.piles,
            d.walkers.len()
        ),
        Win::UnderRuler { .. } => match d.join_now {
            Some(r) => format!("TIME {left}  ALL JOINED: {r:.2}X THE RULER"),
            None => format!("TIME {left}  FOOD JOINED {}/{}", joined.reach, joined.piles),
        },
        Win::AliveAtBell { .. } => {
            let fed = d.dish_piles().filter(|p| p.reached() && p.food > 0.0).count();
            format!("TIME {left}  MOULD {}  EATING {fed} PILE{}", d.walkers.len(), if fed == 1 { "" } else { "S" })
        }
    }
}

/// A dotted line between two dish points, a dot every `every` cells.
fn dots(frame: &mut [u8], fw: u32, scale: u32, a: (f32, f32), b: (f32, f32), every: f32, c: [u8; 3]) {
    let len = (b.0 - a.0).hypot(b.1 - a.1);
    let n = (len / every).floor() as usize;
    let s = scale as f32;
    // A dot half a cell across, never under 2 pixels, so it reads at the
    // window's scale 4 as well as the probe's 2.
    let size = (scale as i32 / 2).max(2);
    for k in 0..=n {
        let f = k as f32 * every / len.max(0.01);
        let (x, y) = ((a.0 + (b.0 - a.0) * f) * s, (a.1 + (b.1 - a.1) * f) * s);
        for oy in 0..size {
            for ox in 0..size {
                put(frame, fw, x as i32 + ox, y as i32 + oy, c);
            }
        }
    }
}

/// Beat the Ruler's ruler, dotted between the piles' rims, and once the dish
/// is over the best possible answer, faint: three tubes from the meeting
/// point, so the player sees how close they came.
fn ruler_overlay(d: &Dish, scale: u32, frame: &mut [u8], fw: u32) {
    let Some(l) = d.level else { return };
    if !matches!(l.win, super::level::Win::UnderRuler { .. }) {
        return;
    }
    let rim = |a: usize, b: usize| {
        let (p, q) = (&l.piles[a], &l.piles[b]);
        let len = (q.x - p.x).hypot(q.y - p.y).max(0.01);
        let (ux, uy) = ((q.x - p.x) / len, (q.y - p.y) / len);
        ((p.x + ux * (p.r + 3.0), p.y + uy * (p.r + 3.0)), (q.x - ux * (q.r + 3.0), q.y - uy * (q.r + 3.0)))
    };
    for (a, b) in l.ruler().1 {
        let (pa, pb) = rim(a, b);
        dots(frame, fw, scale, pa, pb, 4.0, [190, 170, 120]);
    }
    if d.phase == Phase::Over {
        let (m, _) = l.meeting_point();
        for p in l.piles {
            dots(frame, fw, scale, m, (p.x, p.y), 2.5, [120, 200, 230]);
        }
        ring(frame, fw, scale, m, 3.0, [160, 220, 240]);
    }
}

/// The briefing card for a hand-made dish: its goal, its one twist, what
/// each star needs, and only the tools it gives.
fn level_brief(l: &super::level::Level, frame: &mut [u8], (fw, fh): (u32, u32), scale: u32) {
    use super::level::{clock, Win};
    let mut lines: Vec<(String, [u8; 4])> = vec![
        (l.name.to_string(), HOT),
        (l.goal[0].to_string(), TEXT),
        (l.goal[1].to_string(), TEXT),
        (l.twist.to_string(), DIM),
        (String::new(), TEXT),
        ("TAN: FOOD    YELLOW: FED MOULD".into(), [214, 172, 104, 255]),
        ("GREY: CUT OFF FROM FOOD, STARVING".into(), [180, 180, 174, 255]),
        (String::new(), TEXT),
        ("STARS".into(), HOT),
    ];
    let bell = clock(l.seconds);
    let stars: [String; 3] = match l.win {
        Win::JoinAll { by } => [
            format!("*   REACH THE FOOD BY {bell}"),
            format!("**  BY {}", clock(by[0])),
            format!("*** BY {}", clock(by[1])),
        ],
        Win::UnderRuler { bars } => [
            format!("*   ALL THREE JOINED AT {bell}"),
            format!("**  TUBE {:.2}X THE RULER OR LESS", bars[0]),
            format!("*** {:.2}X OR LESS", bars[1]),
        ],
        Win::AliveAtBell { bars } => [
            format!("*   {} MOULD ALIVE AT {bell}", bars[0]),
            format!("**  {} ALIVE", bars[1]),
            format!("*** {} ALIVE", bars[2]),
        ],
    };
    lines.extend(stars.into_iter().map(|s| (s, TEXT)));
    lines.push((String::new(), TEXT));
    lines.push(("TOOLS".into(), HOT));
    let t = l.tools;
    lines.push(("1 LAMP  HOLD: PUSHES THE MOULD AWAY".into(), TEXT));
    if t.oats > 0 {
        let back = if t.oats_regrow > 0 {
            format!(", 1 BACK / {} S", t.oats_regrow)
        } else {
            String::new()
        };
        lines.push((format!("2 OATS {}{back}: IT GROWS TO THEM", t.oats), TEXT));
    }
    if t.salt > 0 {
        lines.push((format!("3 SALT {}  DRAG: A WALL", t.salt), TEXT));
    }
    if t.snips > 0 {
        lines.push((format!("4 SNIP {}  DRAG: CUT A TUBE", t.snips), TEXT));
    }
    lines.push((String::new(), TEXT));
    lines.push(("CLICK OR PRESS A KEY TO START".into(), HOT));
    let w = lines.iter().map(|(l, _)| text_px(l)).max().unwrap_or(0) + 28;
    let h = 16 * lines.len() as i32 + 20;
    let dish_h = (H as u32 * scale) as i32;
    let (x0, y0) = ((fw as i32 - w) / 2, ((dish_h - h) / 2).max(2));
    panel(frame, fw, (x0, y0, w, h), [120, 110, 70]);
    for (k, (l, c)) in lines.iter().enumerate() {
        text(frame, (fw, fh), x0 + 14, y0 + 10 + 16 * k as i32, l, *c);
    }
}

/// The dish picker: the lesson, the challenge dishes, and free play, with
/// best stars and what opens a locked one.
#[derive(Clone, Copy, Debug)]
pub struct Menu {
    pub progress: super::level::Progress,
    /// The highlighted entry.
    pub cursor: usize,
}

/// Each picker entry's card, in frame pixels (x, y, w, h): shared by the
/// drawing and the window's click test.
pub fn menu_cards((fw, fh): (u32, u32)) -> Vec<(i32, i32, i32, i32)> {
    let n = super::level::ENTRIES as i32;
    let w = (fw as i32 - 24).min(620);
    let h = 56;
    let gap = 6;
    let top = ((fh as i32 - (n * (h + gap) + 30)) / 2).max(2) + 26;
    (0..n).map(|k| ((fw as i32 - w) / 2, top + k * (h + gap), w, h)).collect()
}

/// An entry's name and two lines, for the picker.
pub fn entry_lines(e: usize) -> (&'static str, [&'static str; 2]) {
    use super::level::{DISHES, ENTRIES};
    match e {
        0 => ("LESSON", ["SIX STEPS: EACH TOOL IN TURN,", "THEN A STORM."]),
        _ if e == ENTRIES - 1 => ("FREE DISH", ["THE SEEDED DISHES: JOIN, SHORT,", "STORM-PROOF. N: ANOTHER DISH."]),
        _ => (DISHES[e - 1].name, DISHES[e - 1].goal),
    }
}

fn menu(m: &Menu, frame: &mut [u8], (fw, fh): (u32, u32), _scale: u32) {
    use super::level::ENTRIES;
    // Dim what is behind, so the cards read.
    for px in frame.chunks_exact_mut(4) {
        for v in px.iter_mut().take(3) {
            *v /= 3;
        }
    }
    let cards = menu_cards((fw, fh));
    let title = "DISHES   1-5 OR CLICK TO PICK   M: BACK";
    let (x0, y0, _, _) = cards[0];
    text(frame, (fw, fh), x0, y0 - 22, title, HOT);
    for (e, &(x, y, w, h)) in cards.iter().enumerate() {
        let open = m.progress.unlocked(e);
        let edge = if e == m.cursor {
            [255, 236, 120]
        } else if open {
            [120, 110, 70]
        } else {
            [60, 60, 62]
        };
        panel(frame, fw, (x, y, w, h), edge);
        let (name, lines) = entry_lines(e);
        let c = if open { TEXT } else { DIM };
        text(frame, (fw, fh), x + 10, y + 6, &format!("{} {name}", e + 1), if open { HOT } else { DIM });
        // Best stars, or "DONE" for the lesson; nothing for free play.
        let right = if e == ENTRIES - 1 {
            String::new()
        } else if e == 0 {
            if m.progress.best[0] >= 1 { "DONE" } else { "" }.to_string()
        } else {
            let b = m.progress.best[e];
            (0..3).map(|k| if k < b { '*' } else { '-' }).collect()
        };
        text(frame, (fw, fh), x + w - 10 - text_px(&right), y + 6, &right, HOT);
        if open {
            text(frame, (fw, fh), x + 10, y + 22, lines[0], c);
            text(frame, (fw, fh), x + 10, y + 38, lines[1], c);
        } else {
            let (prev, _) = entry_lines(e - 1);
            text(frame, (fw, fh), x + 10, y + 22, lines[0], c);
            let need = if e == 1 {
                "LOCKED: FINISH THE LESSON".to_string()
            } else {
                format!("LOCKED: 1 STAR ON {prev}")
            };
            text(frame, (fw, fh), x + 10, y + 38, &need, [200, 120, 100, 255]);
        }
    }
}
