//! **What the screen shows** on top of the world: the cloud, the leader and
//! where it will likely go, the bolt and the current it sent along the
//! scorch lines, the lamps' glow, and one strip of plain words.
//!
//! Everything here is drawn on the composed window frame, never into the
//! renderer's own buffer, so the renderer's dirty-rect skip keeps working
//! under a bolt: a strike repaints the world only where the world changed.

use super::level::{H, W};
use super::{Phase, Storm, BOLTS, CLOUD_BASE, OVERFLOW_HOLD, PATIENCE, TICKS_PER_SECOND};
use crate::hud;

/// Window pixels per world cell.
pub const SCALE: i32 = 3;
/// The strip of words under the world, in window pixels.
pub const STRIP: i32 = 60;

pub fn frame_size() -> (u32, u32) {
    ((W * SCALE) as u32, (H * SCALE + STRIP) as u32)
}

struct Canvas<'a> {
    px: &'a mut [u8],
    w: i32,
    h: i32,
}

impl Canvas<'_> {
    #[inline]
    fn blend(&mut self, x: i32, y: i32, c: [u8; 3], a: f32) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h || a <= 0.0 {
            return;
        }
        let a = a.min(1.0);
        let i = ((y * self.w + x) * 4) as usize;
        for (k, &ck) in c.iter().enumerate() {
            let d = self.px[i + k] as f32;
            self.px[i + k] = (d + (ck as f32 - d) * a) as u8;
        }
        self.px[i + 3] = 255;
    }

    #[inline]
    fn add(&mut self, x: i32, y: i32, c: [u8; 3], a: f32) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h || a <= 0.0 {
            return;
        }
        let i = ((y * self.w + x) * 4) as usize;
        for (k, &ck) in c.iter().enumerate() {
            self.px[i + k] = (self.px[i + k] as f32 + ck as f32 * a).min(255.0) as u8;
        }
    }

    /// Fill one world cell.
    fn cell(&mut self, cx: i32, cy: i32, c: [u8; 3], a: f32) {
        for dy in 0..SCALE {
            for dx in 0..SCALE {
                self.blend(cx * SCALE + dx, cy * SCALE + dy, c, a);
            }
        }
    }

    fn cell_add(&mut self, cx: i32, cy: i32, c: [u8; 3], a: f32) {
        for dy in 0..SCALE {
            for dx in 0..SCALE {
                self.add(cx * SCALE + dx, cy * SCALE + dy, c, a);
            }
        }
    }

    /// A soft additive glow around a cell, `r` cells across.
    fn glow(&mut self, cx: i32, cy: i32, r: i32, c: [u8; 3], a: f32) {
        let (px, py) = (cx * SCALE + SCALE / 2, cy * SCALE + SCALE / 2);
        let rp = r * SCALE;
        for y in (py - rp)..=(py + rp) {
            for x in (px - rp)..=(px + rp) {
                let d2 = ((x - px).pow(2) + (y - py).pow(2)) as f32;
                let f = 1.0 - d2 / (rp * rp) as f32;
                if f > 0.0 {
                    self.add(x, y, c, a * f * f);
                }
            }
        }
    }

    fn rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: [u8; 3], a: f32) {
        for y in y0..y1 {
            for x in x0..x1 {
                self.blend(x, y, c, a);
            }
        }
    }
}

const WHITE: [u8; 3] = [255, 255, 255];
const BOLT: [u8; 3] = [214, 226, 255];
const ION: [u8; 3] = [150, 110, 255];
const LAMP: [u8; 3] = [255, 214, 120];
const CURRENT: [u8; 3] = [255, 170, 70];

pub fn compose(s: &Storm, frame: &mut [u8]) {
    let (fw, fh) = frame_size();
    let mut cv = Canvas { px: frame, w: fw as i32, h: fh as i32 };
    // The world, scaled up.
    let src = s.world_pixels();
    for y in 0..H {
        for x in 0..W {
            let i = ((y * W + x) * 4) as usize;
            let c = [src[i], src[i + 1], src[i + 2], 255];
            for dy in 0..SCALE {
                let row = ((y * SCALE + dy) * cv.w + x * SCALE) as usize * 4;
                for dx in 0..SCALE as usize {
                    cv.px[row + dx * 4..row + dx * 4 + 4].copy_from_slice(&c);
                }
            }
        }
    }
    let t = s.tick as f32;

    // Fresh ions: a violet haze where a bolt has just been, which is where
    // the next one wants to go.
    for y in 0..H {
        for x in 0..W {
            let v = s.ions[(y * W + x) as usize];
            if v > 0.02 {
                cv.cell_add(x, y, ION, v * 0.32);
            }
        }
    }

    // The lamps' light: graded by how much is in them.
    for (k, lamp) in s.level.lamps.iter().enumerate() {
        let p = s.lamps[k] as f32;
        let (cx, _) = lamp.head.centre();
        let cy = lamp.head.y0 + 1;
        if p > 0.0 {
            let flicker = 0.9 + 0.1 * (t * 0.31 + k as f32 * 2.0).sin();
            cv.glow(cx, cy, 7 + 4 * s.lamps[k] as i32, LAMP, 0.10 * p * flicker);
        }
        // Three pips above each lamp, plain: how full it is.
        for i in 0..3 {
            let on = (i as u8) < s.lamps[k];
            let (px, py) = ((cx - 2 + i * 2) * SCALE, (lamp.head.y0 - 4) * SCALE);
            let c = if on { [255, 226, 140] } else { [70, 70, 84] };
            cv.rect(px, py, px + 4, py + 4, c, if on { 1.0 } else { 0.8 });
        }
    }

    // The cloud: a dark ragged band that brightens where charge is gathered.
    let (charge_x, charge) = match &s.phase {
        Phase::Calling { t: h, from, .. } => (from.0, 0.4 + 0.6 * (*h as f32 / (OVERFLOW_HOLD * TICKS_PER_SECOND as f32)).min(1.0)),
        Phase::Ready { t: r } => (s.pointer.0, 0.15 + 0.1 * ((*r as f32) * 0.25).sin().abs()),
        _ => (s.pointer.0, 0.0),
    };
    for y in 0..=CLOUD_BASE + 2 {
        for x in 0..W {
            let n = crate::sim::rng::jitter3(x / 3, y / 2, (s.tick / 9) as i32 % 64);
            let edge = CLOUD_BASE as f32 - 2.0 + 3.5 * crate::sim::rng::jitter(x / 4, 91);
            if (y as f32) > edge {
                continue;
            }
            let base = 38.0 + 22.0 * n;
            let near = (-(((x - charge_x) as f32).powi(2)) / 300.0).exp() * charge;
            let c = [(base + 150.0 * near) as u8, (base + 6.0 + 150.0 * near) as u8, (base + 20.0 + 170.0 * near) as u8];
            cv.cell(x, y, c, 0.93);
        }
    }

    // While you hold: the leader, faint feelers, and the likeliest way down.
    if let Phase::Calling { leader, .. } = &s.phase {
        for f in &s.feelers {
            for &(x, y) in &f.cells {
                cv.cell_add(x, y, BOLT, 0.18);
            }
        }
        for (k, &(x, y)) in s.likely.cells.iter().enumerate() {
            if (k / 2) % 2 == 0 {
                cv.cell(x, y, [255, 236, 150], 0.55);
            }
        }
        if let Some((x, y)) = s.likely.hit {
            for d in -2..=2 {
                cv.cell(x + d, y - 1, [255, 236, 150], 0.7);
                cv.cell(x, y - 1 + d, [255, 236, 150], 0.7);
            }
        }
        for &(x, y) in leader {
            cv.cell(x, y, BOLT, 0.9);
            cv.glow(x, y, 2, BOLT, 0.05);
        }
    }

    // The strike: white for a few frames, then the real channel fades over
    // two seconds beside where you aimed, and the current glows along every
    // wire it reached.
    if let Some(st) = &s.strike {
        let a = st.age as f32;
        let bright = if a < 6.0 { 1.0 } else { (1.0 - (a - 6.0) / 114.0).max(0.0) * 0.6 };
        if bright > 0.0 {
            let col = if a < 6.0 { WHITE } else { BOLT };
            for b in &st.branches {
                for &(x, y) in &b.cells {
                    cv.cell(x, y, col, bright * 0.55);
                }
            }
            for &(x, y) in &st.main {
                cv.cell(x, y, col, bright);
                cv.cell(x + 1, y, col, bright * 0.8);
                if a < 10.0 {
                    cv.glow(x, y, 4, BOLT, 0.05 * bright);
                }
            }
        }
        let current = (1.0 - a / 75.0).max(0.0);
        if current > 0.0 {
            let pulse = 0.65 + 0.35 * (a * 0.6).sin();
            for &(x, y) in &st.network {
                cv.cell_add(x, y, CURRENT, 0.55 * current * pulse);
            }
            for &(x, y) in &st.roots {
                cv.cell_add(x, y, [255, 240, 200], 0.7 * current);
            }
        }
        if a < 4.0 {
            if let Some((x, y)) = st.hit {
                cv.glow(x, y, 10, WHITE, 0.5);
            }
        }
    }

    // The flash lights the whole valley for a moment.
    if s.flash > 0.02 {
        let f = s.flash * 0.45;
        for y in 0..H * SCALE {
            for x in 0..W * SCALE {
                cv.add(x, y, [200, 210, 255], f);
            }
        }
    }

    // The pointer.
    let (px, py) = (s.pointer.0 * SCALE + 1, s.pointer.1 * SCALE + 1);
    for d in 3..8 {
        for (x, y) in [(px + d, py), (px - d, py), (px, py + d), (px, py - d)] {
            cv.blend(x, y, WHITE, 0.8);
        }
    }

    strip(s, &mut cv);
}

fn text(cv: &mut Canvas, (x, y): (i32, i32), words: &str, c: [u8; 3]) {
    let (w, h) = (cv.w as u32, cv.h as u32);
    hud::draw_text_crisp(cv.px, (w, h), (x / 2, y / 2), words, [c[0], c[1], c[2], 255]);
}

fn strip(s: &Storm, cv: &mut Canvas) {
    let top = H * SCALE;
    let (w, _) = frame_size();
    cv.rect(0, top, w as i32, top + STRIP, [16, 16, 22], 1.0);
    let lamp = |p: u8| -> String { format!("[{}]", (0..3).map(|i| if i < p { '*' } else { '-' }).collect::<String>()) };
    let line1 = format!(
        "BOLT {}/{}  FIELD LAMP {}  ROCK LAMP {}  BELL {}  {}",
        (BOLTS - s.bolts_left + 1).min(BOLTS),
        BOLTS,
        lamp(s.lamps[0]),
        lamp(s.lamps[1]),
        s.tally.bell_hits,
        stars(s.stars()),
    );
    text(cv, (8, top + 4), &line1, [230, 230, 236]);
    let sec = TICKS_PER_SECOND as f32;
    let (line2, warn) = match &s.phase {
        Phase::Ready { t } => {
            let left = PATIENCE - *t as f32 / sec;
            if left < 2.5 {
                (format!("HOLD SPACE - THE CLOUD STRIKES THE BELL BY ITSELF IN {}", left.ceil() as i32), true)
            } else {
                ("HOLD SPACE TO CALL LIGHTNING ABOVE THE POINTER".to_string(), false)
            }
        }
        Phase::Calling { t, .. } => {
            let h = *t as f32 / sec;
            let weight = Storm::heaviness(*t);
            let bar: String = (0..10).map(|i| if (i as f32) < weight * 10.0 { '*' } else { '-' }).collect();
            if h > OVERFLOW_HOLD - 0.7 {
                (format!("LET GO NOW OR IT HITS THE BELL  [{bar}]"), true)
            } else {
                (format!("RELEASE TO STRIKE - LONGER IS STRAIGHTER  [{bar}]"), false)
            }
        }
        Phase::Gathering { .. } => match &s.strike {
            Some(st) if st.age < 150 => (st.said.clone(), false),
            _ => ("THE CLOUD IS GATHERING CHARGE".to_string(), false),
        },
        Phase::Over => (format!("THE STORM IS OVER - {} - PRESS R TO PLAY AGAIN", stars(s.stars())), false),
    };
    text(cv, (8, top + 22), &line2, if warn { [255, 150, 110] } else { [255, 230, 160] });
    let line3 = format!("CLICK: WATER ({})  RIGHT CLICK: IRON ROD ({})  R: RESTART", s.splashes_left, s.rods_left);
    text(cv, (8, top + 40), &line3, [170, 176, 190]);

    if s.is_over() {
        let (cx, cy) = (W * SCALE / 2, H * SCALE / 2 - 30);
        cv.rect(cx - 330, cy - 40, cx + 330, cy + 54, [10, 10, 16], 0.86);
        let a = format!("THE STORM IS OVER: {}", stars(s.stars()));
        let b = format!("LAMP LIGHT {}  BELL HITS {}  HOUSES BURNED {}", s.lamps[0] + s.lamps[1], s.tally.bell_hits, s.huts_lost_penalty());
        text(cv, (cx - 6 * a.len() as i32, cy - 26), &a, [255, 230, 160]);
        text(cv, (cx - 6 * b.len() as i32, cy), &b, [220, 220, 230]);
        let c = "PRESS R TO PLAY AGAIN";
        text(cv, (cx - 6 * c.len() as i32, cy + 26), c, [170, 176, 190]);
    }
}

fn stars(n: u32) -> String {
    match n {
        0 => "NO STARS".to_string(),
        1 => "1 STAR".to_string(),
        k => format!("{k} STARS"),
    }
}

