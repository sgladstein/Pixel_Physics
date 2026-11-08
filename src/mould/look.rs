//! **Interface looks: three ways the dish and its controls could read.**
//!
//! Round 1 of the interface pass (brief: `/mnt/project-files/game-design/
//! mould-game-brainstorm-2026-11-08/prototypes/interface.md`). The owner,
//! on his first play: "the interface could use a lot of work". The lesson
//! fixed *instruction*; this is how the dish and its controls *look and
//! read*. Three looks, each a whole answer to the same list -- a tool bar
//! with icons, keys, stock and selection; a status with a storm countdown
//! nobody can miss in its last 5 s; cursor previews of what a press will do;
//! the briefing card, lesson banner and score card; the dish itself; and a
//! font -- so the owner can pick one by eye, blind (`Style::letter`).
//!
//! **Round 2 built the pick** (`Style::Night`, the window's look since
//! 2026-11-08): B's dish and mood with C's tool tiles and storm timeline,
//! drawing everything `render.rs` draws -- the pulse along fed tube, the
//! join flashes, open wounds, and the Wanderer's crack, label, bar, card
//! and arrival -- inside about 1.7 ms a frame over the classic look (the
//! cuts: a half-size glow grid computed every other frame, the food's glow
//! folded into it, the tile bar redrawn only when it changes, the rim's
//! alarm flash and the bare dish kept per look, opaque corner chips).
//!
//! **A sibling of `render.rs`, not a rewrite of it.** `render::draw` hands
//! over to `draw` here when `View::style` is not `Classic`, and nothing else
//! in `render.rs` changed: other builds are editing it (a pulse along the
//! tubes, a snap flash, open wounds), and the picked look will be rebased
//! onto those. Drawing only: no look reads or writes anything the
//! simulation reads, so every look is the same dish, step for step.
//!
//! **Previews draw only what the code knows** (testers' top ask in the
//! 8-demo playtest: show what an action will do before it is committed):
//! the lamp's reach (`Params::lamp_reach`) with arrows outward because the
//! push carries trail outward; oats where they land, refused where
//! `Dish::place_oats` would refuse them, and the measured pull reach
//! (`OATS_PULL`) with a thread to the nearest mould; the salt or snip
//! stroke's length still to draw (`Dish::stroke_state`); and the tube cells
//! a snip would clear (the stroke's own 2.5-cell disc).
//!
//! Fed-yellow / starving-grey stays the game's main readout in every look
//! (`theme.md`: the mould's colour already *is* its health; no meter
//! covers it).

use super::level::{Win, FREE};
use super::render::{entry_lines, frame_size, menu_cards, thousands, Menu, View};
use super::wander::{self, Outcome, Wander};
use super::{
    disc, Dish, Ground, Phase, Tool, H, OATS_R, OATS_SECONDS, SALT_LEN, SALT_STOCK,
    SNIP_LEN, SNIP_STOCK, STEPS_PER_SECOND, STORM_WARNING_SECONDS, THRIFT_BAR, THRIFT_MIN_JOINED, W,
};
use crate::hud;
use std::f32::consts::{PI, TAU};

/// How far from the nearest mould oats still pull it, in cells: the ring the
/// oats preview draws. **Measured, not chosen** (`mouldprobe pull=1`, seeds
/// 1-5, 8 rays each, oats beyond the front at 6 s, "pulled" = mould on the
/// oats within 5 s), binned by distance to the nearest walker:
///
/// | nearest mould | pulled | mean time |
/// |---|---|---|
/// | 4-20 cells | 152 of 152 | 0.06-0.32 s |
/// | 20-28 | 61 of 61 | 0.51-0.87 s |
/// | 28-36 | 15 of 18 | 1.7-2.2 s |
/// | 36-40 | 2 of 7 | 3.3 s |
/// | 40-48 | 0 of 4 | |
///
/// So the ring is drawn at the edge of the every-time band, where a click
/// is a promise the dish keeps, and `OATS_MAYBE` marks the slow, usual band
/// beyond it: the preview says which of the three a click is in.
pub const OATS_PULL: f32 = 28.0;
/// Beyond `OATS_PULL`, out to here, oats usually pull, slowly (15 of 18).
pub const OATS_MAYBE: f32 = 36.0;

/// The snip's own cutting radius (`Dish::stroke_dab`), for the preview of
/// what a press would clear.
const SNIP_R: f32 = 2.5;
/// The salt dab's radius, likewise.
const SALT_R: f32 = 1.5;

/// The dish ellipse (`Dish::with_params`).
const CX: f32 = W as f32 / 2.0;
const CY: f32 = H as f32 / 2.0;
const RX: f32 = W as f32 / 2.0 - 3.0;
const RY: f32 = H as f32 / 2.0 - 3.0;

/// Trail at which the tube ramp tops out (as `render.rs`).
const TRAIL_TOP: f32 = 30.0;

// ------------------------------------------------------------------ style

/// Which look draws the frame. `Classic` is `render.rs` unchanged, the
/// reference every look is compared against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Style {
    #[default]
    Classic,
    /// **The game's look** (`Style::GAME`), built from the owner's pick
    /// (2026-11-08, judged blind before the designer's note, and the same
    /// pick): B's dish and mood -- glowing tube on a dark culture, the lamp
    /// lighting the agar, the mixed-case font, the storm's big count -- with
    /// C's labelled tool tiles (name, key, stock pips, "+1 in 5s", "3 left")
    /// and C's timeline to the storm with the late food marked on it.
    Night,
    /// A clean specimen bench: a glass dish on graph paper, glossy embossed
    /// tubes, tiles with stock pips, a timeline to the storm.
    Bench,
    /// A culture in the dark: glowing tubes, a lamp that lights the agar,
    /// round buttons, a storm dial, and a mixed-case font.
    Glow,
    /// A picture book: inked outlines, chunky sticker type, big coloured
    /// buttons on a wooden bench, hazard tape for the storm, a fuse for time.
    Toy,
}

impl Style {
    /// What the window opens in. `View::default()` stays `Classic`, because
    /// the Mould Organ (`organ.rs`) paints its own rows over the classic
    /// frame and is left alone.
    pub const GAME: Style = Style::Night;

    /// F2's order: the game's look, then the old one, then the three
    /// candidates it was picked from.
    pub const ALL: [Style; 5] = [Style::Night, Style::Classic, Style::Bench, Style::Glow, Style::Toy];

    pub fn next(self) -> Style {
        let i = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    pub fn name(self) -> &'static str {
        match self {
            Style::Classic => "classic",
            Style::Night => "night",
            Style::Bench => "bench",
            Style::Glow => "glow",
            Style::Toy => "toy",
        }
    }

    /// The letter the owner judges this look by, blind. Which letter is which
    /// look is written down in the round's `interface/key.md`; keep the two
    /// in step. The window shows only this letter, in a corner.
    pub fn letter(self) -> &'static str {
        match self {
            Style::Classic => "-",
            Style::Night => "N",
            Style::Toy => "A",
            Style::Glow => "B",
            Style::Bench => "C",
        }
    }

    /// By name or by letter.
    pub fn parse(s: &str) -> Option<Style> {
        Self::ALL
            .into_iter()
            .find(|st| st.name().eq_ignore_ascii_case(s) || st.letter().eq_ignore_ascii_case(s))
    }
}

// ------------------------------------------------------------------ theme

#[derive(Clone, Copy, PartialEq, Eq)]
enum Font {
    /// `hud::draw_text_crisp`: the engine's anti-aliased capitals.
    Crisp,
    /// Capitals from the dot font plus this module's lower case, doubled.
    Mixed,
    /// The dot font doubled, with an ink outline: sticker type.
    Chunky,
}

type Rgb = [u8; 3];

struct Theme {
    /// Which look's dish, piles, storm and cards this draws: `Night` draws
    /// them as `Glow`.
    style: Style,
    /// Which look's tool bar and corner chips: `Night` draws them as
    /// `Bench`.
    hud: Style,
    /// The look itself.
    look: Style,
    font: Font,
    agar: Rgb,
    agar_edge: Rgb,
    outside: Rgb,
    rim_light: Rgb,
    rim_dark: Rgb,
    fed: [Rgb; 4],
    pale: [Rgb; 3],
    emboss: f32,
    ink: Option<Rgb>,
    glow: f32,
    food: Rgb,
    oats: Rgb,
    storm: Rgb,
    hot: Rgb,
    text: Rgb,
    dim: Rgb,
    panel: Rgb,
    panel_edge: Rgb,
    panel_alpha: f32,
    radius: f32,
    hud_bg: Rgb,
    lamp: Rgb,
    bad: Rgb,
}

fn theme(style: Style) -> Theme {
    match style {
        Style::Night => Theme {
            hud: Style::Bench,
            look: Style::Night,
            ..theme(Style::Glow)
        },
        Style::Glow => Theme {
            style,
            hud: style,
            look: style,
            font: Font::Mixed,
            agar: [9, 17, 22],
            agar_edge: [3, 7, 10],
            outside: [3, 4, 7],
            rim_light: [70, 150, 175],
            rim_dark: [10, 22, 28],
            fed: [[110, 60, 6], [214, 140, 20], [255, 214, 80], [255, 250, 225]],
            pale: [[46, 50, 58], [112, 118, 128], [182, 188, 198]],
            emboss: 0.0,
            ink: None,
            glow: 0.55,
            food: [236, 170, 84],
            oats: [250, 234, 186],
            storm: [196, 96, 255],
            hot: [255, 212, 90],
            text: [222, 234, 242],
            dim: [112, 134, 148],
            panel: [8, 14, 20],
            panel_edge: [70, 150, 180],
            panel_alpha: 0.88,
            radius: 12.0,
            hud_bg: [4, 7, 11],
            lamp: [190, 220, 255],
            bad: [255, 96, 116],
        },
        Style::Toy => Theme {
            style,
            hud: style,
            look: style,
            font: Font::Chunky,
            agar: [160, 204, 186],
            agar_edge: [124, 172, 156],
            outside: [128, 88, 56],
            rim_light: [252, 250, 240],
            rim_dark: [170, 160, 150],
            fed: [[206, 136, 12], [240, 176, 24], [255, 214, 52], [255, 240, 150]],
            pale: [[118, 118, 114], [150, 150, 146], [190, 190, 184]],
            emboss: 1.0,
            ink: Some([58, 36, 24]),
            glow: 0.0,
            food: [214, 156, 90],
            oats: [248, 232, 186],
            storm: [228, 52, 40],
            hot: [255, 204, 40],
            text: [255, 248, 230],
            dim: [226, 204, 170],
            panel: [86, 58, 38],
            panel_edge: [252, 238, 204],
            panel_alpha: 1.0,
            radius: 14.0,
            hud_bg: [124, 86, 54],
            lamp: [255, 236, 150],
            bad: [228, 52, 40],
        },
        // Bench, and the fallback.
        _ => Theme {
            style: Style::Bench,
            hud: Style::Bench,
            look: Style::Bench,
            font: Font::Crisp,
            agar: [28, 46, 44],
            agar_edge: [15, 26, 26],
            outside: [17, 20, 23],
            rim_light: [168, 190, 196],
            rim_dark: [34, 40, 44],
            fed: [[70, 46, 6], [176, 120, 14], [250, 206, 40], [255, 250, 210]],
            pale: [[48, 50, 52], [114, 116, 116], [194, 194, 188]],
            emboss: 0.8,
            ink: None,
            glow: 0.0,
            food: [214, 172, 104],
            oats: [240, 222, 170],
            storm: [214, 84, 222],
            hot: [255, 214, 60],
            text: [228, 232, 226],
            dim: [130, 140, 138],
            panel: [18, 22, 25],
            panel_edge: [72, 82, 86],
            panel_alpha: 0.95,
            radius: 6.0,
            hud_bg: [15, 18, 21],
            lamp: [225, 238, 255],
            bad: [255, 92, 72],
        },
    }
}

// ----------------------------------------------------------------- canvas

fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    [0, 1, 2].map(|k| (a[k] as f32 + (b[k] as f32 - a[k] as f32) * t) as u8)
}

fn mul(a: Rgb, k: f32) -> Rgb {
    a.map(|v| (v as f32 * k).clamp(0.0, 255.0) as u8)
}

fn ramp(stops: &[Rgb], v: f32) -> Rgb {
    let v = v.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let i = (v as usize).min(stops.len() - 2);
    lerp(stops[i], stops[i + 1], v - i as f32)
}

/// A length without `hypot`'s overflow care, which was the single largest
/// cost in drawing rings and discs.
#[inline]
fn len2(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt()
}

fn hash(x: u32, y: u32) -> u32 {
    (x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663)).wrapping_mul(2_654_435_761)
}

struct Cv<'a> {
    f: &'a mut [u8],
    w: i32,
    h: i32,
}

impl Cv<'_> {
    #[inline]
    fn put(&mut self, x: i32, y: i32, c: Rgb) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let i = (y as usize * self.w as usize + x as usize) * 4;
        self.f[i..i + 3].copy_from_slice(&c);
        self.f[i + 3] = 255;
    }

    #[inline]
    fn mix(&mut self, x: i32, y: i32, c: Rgb, a: f32) {
        if a <= 0.004 || x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let i = (y as usize * self.w as usize + x as usize) * 4;
        if a >= 0.999 {
            self.f[i..i + 3].copy_from_slice(&c);
            self.f[i + 3] = 255;
            return;
        }
        for (px, &ck) in self.f[i..i + 3].iter_mut().zip(&c) {
            let d = *px as f32;
            *px = (d + (ck as f32 - d) * a) as u8;
        }
        self.f[i + 3] = 255;
    }

    #[inline]
    fn add(&mut self, x: i32, y: i32, c: Rgb, a: f32) {
        if a <= 0.004 || x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let i = (y as usize * self.w as usize + x as usize) * 4;
        for (px, &ck) in self.f[i..i + 3].iter_mut().zip(&c) {
            *px = (*px as f32 + ck as f32 * a).min(255.0) as u8;
        }
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb, a: f32) {
        for yy in y.max(0)..(y + h).min(self.h) {
            for xx in x.max(0)..(x + w).min(self.w) {
                if a >= 1.0 {
                    self.put(xx, yy, c);
                } else {
                    self.mix(xx, yy, c, a);
                }
            }
        }
    }

    /// A rounded box: filled, outlined (`edge` = colour and thickness), or both.
    #[allow(clippy::too_many_arguments)]
    fn rbox(&mut self, x: i32, y: i32, w: i32, h: i32, r: f32, fill: Option<(Rgb, f32)>, edge: Option<(Rgb, f32)>) {
        let (cx, cy) = (x as f32 + w as f32 / 2.0, y as f32 + h as f32 / 2.0);
        let (bx, by) = (w as f32 / 2.0 - r, h as f32 / 2.0 - r);
        // Pixels this far inside every side are plain fill: no distance.
        let m = (r + edge.map_or(0.0, |e| e.1)) as i32 + 2;
        for yy in y.max(0)..(y + h).min(self.h) {
            let mid_row = yy >= y + m && yy < y + h - m;
            for xx in x.max(0)..(x + w).min(self.w) {
                if mid_row && xx >= x + m && xx < x + w - m {
                    if let Some((c, a)) = fill {
                        self.mix(xx, yy, c, a);
                    }
                    continue;
                }
                let qx = ((xx as f32 + 0.5 - cx).abs() - bx).max(-1e9);
                let qy = ((yy as f32 + 0.5 - cy).abs() - by).max(-1e9);
                let out = len2(qx.max(0.0), qy.max(0.0)) + qx.max(qy).min(0.0) - r;
                let cov = (0.5 - out).clamp(0.0, 1.0);
                if cov <= 0.0 {
                    continue;
                }
                if let Some((c, a)) = fill {
                    self.mix(xx, yy, c, a * cov);
                }
                if let Some((c, t)) = edge {
                    let e = (0.5 - (out + t / 2.0).abs() + t / 2.0).clamp(0.0, 1.0);
                    self.mix(xx, yy, c, e.min(cov));
                }
            }
        }
    }

    /// Visits every pixel within `r + pad` of a centre with its distance.
    fn around(&mut self, cx: f32, cy: f32, reach: f32, f: impl FnMut(&mut Self, i32, i32, f32)) {
        self.annulus(cx, cy, 0.0, reach, f);
    }

    /// Visits every pixel between two radii, row by row over the two runs
    /// of pixels a row crosses, so a thin ring costs its own pixels and not
    /// its bounding box's.
    fn annulus(&mut self, cx: f32, cy: f32, inner: f32, reach: f32, mut f: impl FnMut(&mut Self, i32, i32, f32)) {
        let (y0, y1) = ((cy - reach).floor() as i32, (cy + reach).ceil() as i32);
        for y in y0.max(0)..=y1.min(self.h - 1) {
            let dy = (y as f32 + 0.5 - cy).abs();
            if dy > reach {
                continue;
            }
            let outer = (reach * reach - dy * dy).sqrt();
            let hole = if dy < inner { (inner * inner - dy * dy).sqrt() } else { 0.0 };
            let runs = [
                ((cx - outer).floor() as i32, (cx - hole).ceil() as i32),
                ((cx + hole).floor() as i32, (cx + outer).ceil() as i32),
            ];
            let mut last = i32::MIN;
            for (a, b) in runs {
                for x in a.max(0).max(last.saturating_add(1))..=b.min(self.w - 1) {
                    let d = len2(x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                    if d <= reach && d + 1.0 >= inner {
                        f(self, x, y, d);
                    }
                    last = x;
                }
            }
        }
    }

    fn disc(&mut self, cx: f32, cy: f32, r: f32, c: Rgb, a: f32) {
        self.around(cx, cy, r + 1.0, |cv, x, y, d| {
            cv.mix(x, y, c, a * (r - d + 0.5).clamp(0.0, 1.0));
        });
    }

    /// A ring `t` pixels thick; `dash` = (pieces, phase) leaves gaps.
    #[allow(clippy::too_many_arguments)]
    fn ring(&mut self, cx: f32, cy: f32, r: f32, t: f32, c: Rgb, a: f32, dash: Option<(u32, f32)>) {
        self.annulus(cx, cy, (r - t).max(0.0), r + t, |cv, x, y, d| {
            let cov = (t / 2.0 - (d - r).abs() + 0.5).clamp(0.0, 1.0);
            if cov <= 0.0 {
                return;
            }
            if let Some((n, ph)) = dash {
                let ang = (y as f32 + 0.5 - cy).atan2(x as f32 + 0.5 - cx);
                let u = (ang / TAU * n as f32 + ph).rem_euclid(1.0);
                if u > 0.55 {
                    return;
                }
            }
            cv.mix(x, y, c, a * cov);
        });
    }

    /// The first `frac` of a ring, clockwise from the top.
    #[allow(clippy::too_many_arguments)]
    fn arc(&mut self, cx: f32, cy: f32, r: f32, t: f32, frac: f32, c: Rgb, a: f32) {
        if frac <= 0.0 {
            return;
        }
        self.annulus(cx, cy, (r - t).max(0.0), r + t, |cv, x, y, d| {
            let cov = (t / 2.0 - (d - r).abs() + 0.5).clamp(0.0, 1.0);
            if cov <= 0.0 {
                return;
            }
            let ang = (x as f32 + 0.5 - cx).atan2(-(y as f32 + 0.5 - cy));
            if ang.rem_euclid(TAU) / TAU <= frac {
                cv.mix(x, y, c, a * cov);
            }
        });
    }

    /// Pixels within `half` of the segment a-b: `f(along, across, x, y)`.
    fn band(&mut self, a: (f32, f32), b: (f32, f32), half: f32, mut f: impl FnMut(&mut Self, f32, f32, i32, i32)) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy).max(1e-3);
        let (ux, uy) = (dx / len, dy / len);
        let x0 = (a.0.min(b.0) - half - 1.0).floor() as i32;
        let x1 = (a.0.max(b.0) + half + 1.0).ceil() as i32;
        let y0 = (a.1.min(b.1) - half - 1.0).floor() as i32;
        let y1 = (a.1.max(b.1) + half + 1.0).ceil() as i32;
        for y in y0.max(0)..=y1.min(self.h - 1) {
            for x in x0.max(0)..=x1.min(self.w - 1) {
                let (px, py) = (x as f32 + 0.5 - a.0, y as f32 + 0.5 - a.1);
                let along = px * ux + py * uy;
                let across = -px * uy + py * ux;
                let along_c = along.clamp(0.0, len);
                let dist = len2(along - along_c, across);
                if dist <= half + 0.5 {
                    f(self, along, across, x, y);
                }
            }
        }
    }

    /// A line `t` pixels thick; `dash` > 0 is the dash length in pixels.
    #[allow(clippy::too_many_arguments)]
    fn seg(&mut self, a: (f32, f32), b: (f32, f32), t: f32, c: Rgb, al: f32, dash: f32) {
        let len = (b.0 - a.0).hypot(b.1 - a.1).max(1e-3);
        let (ux, uy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
        self.band(a, b, t / 2.0, |cv, along, _across, x, y| {
            if dash > 0.0 && (along / dash).rem_euclid(2.0) >= 1.0 {
                return;
            }
            let (px, py) = (x as f32 + 0.5 - a.0, y as f32 + 0.5 - a.1);
            let ac = along.clamp(0.0, len);
            let d = len2(px - ux * ac, py - uy * ac);
            cv.mix(x, y, c, al * (t / 2.0 - d + 0.5).clamp(0.0, 1.0));
        });
    }
}

// ------------------------------------------------------------------ fonts

/// Lower case for the mixed font, 5 wide and 9 tall: rows 0-6 share the dot
/// font's capital height, the x-height starts at row 2, and rows 7-8 are
/// descenders. Drawn on the same doubled grid as the dot font's capitals, so
/// a mixed line keeps one baseline.
fn lower(c: char) -> Option<[u8; 9]> {
    let rows: [&str; 9] = match c {
        'a' => [".....", ".....", ".###.", "....#", ".####", "#...#", ".####", ".....", "....."],
        'b' => ["#....", "#....", "####.", "#...#", "#...#", "#...#", "####.", ".....", "....."],
        'c' => [".....", ".....", ".####", "#....", "#....", "#....", ".####", ".....", "....."],
        'd' => ["....#", "....#", ".####", "#...#", "#...#", "#...#", ".####", ".....", "....."],
        'e' => [".....", ".....", ".###.", "#...#", "#####", "#....", ".####", ".....", "....."],
        'f' => ["..##.", ".#...", "####.", ".#...", ".#...", ".#...", ".#...", ".....", "....."],
        'g' => [".....", ".....", ".####", "#...#", "#...#", "#...#", ".####", "....#", ".###."],
        'h' => ["#....", "#....", "####.", "#...#", "#...#", "#...#", "#...#", ".....", "....."],
        'i' => ["..#..", ".....", ".##..", "..#..", "..#..", "..#..", ".###.", ".....", "....."],
        'j' => ["...#.", ".....", "..##.", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##.."],
        'k' => ["#....", "#....", "#..#.", "#.#..", "##...", "#.#..", "#..#.", ".....", "....."],
        'l' => [".##..", "..#..", "..#..", "..#..", "..#..", "..#..", ".###.", ".....", "....."],
        'm' => [".....", ".....", "##.#.", "#.#.#", "#.#.#", "#.#.#", "#.#.#", ".....", "....."],
        'n' => [".....", ".....", "####.", "#...#", "#...#", "#...#", "#...#", ".....", "....."],
        'o' => [".....", ".....", ".###.", "#...#", "#...#", "#...#", ".###.", ".....", "....."],
        'p' => [".....", ".....", "####.", "#...#", "#...#", "#...#", "####.", "#....", "#...."],
        'q' => [".....", ".....", ".####", "#...#", "#...#", "#...#", ".####", "....#", "....#"],
        'r' => [".....", ".....", "#.##.", "##..#", "#....", "#....", "#....", ".....", "....."],
        's' => [".....", ".....", ".####", "#....", ".###.", "....#", "####.", ".....", "....."],
        't' => [".#...", ".#...", "####.", ".#...", ".#...", ".#..#", "..##.", ".....", "....."],
        'u' => [".....", ".....", "#...#", "#...#", "#...#", "#..##", ".##.#", ".....", "....."],
        'v' => [".....", ".....", "#...#", "#...#", "#...#", ".#.#.", "..#..", ".....", "....."],
        'w' => [".....", ".....", "#...#", "#...#", "#.#.#", "#.#.#", ".#.#.", ".....", "....."],
        'x' => [".....", ".....", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", ".....", "....."],
        'y' => [".....", ".....", "#...#", "#...#", "#...#", "#...#", ".####", "....#", ".###."],
        'z' => [".....", ".....", "#####", "...#.", "..#..", ".#...", "#####", ".....", "....."],
        // The dot font's comma reads as a full stop at this size, and
        // "1,265" read as "1.265": a comma with a tail below the line.
        ',' => [".....", ".....", ".....", ".....", ".....", ".##..", ".##..", "..#..", ".#..."],
        _ => return None,
    };
    let mut g = [0u8; 9];
    for (r, row) in rows.iter().enumerate() {
        for (k, ch) in row.bytes().enumerate() {
            if ch == b'#' {
                g[r] |= 1 << (4 - k);
            }
        }
    }
    Some(g)
}

/// Body text at the theme's font. Returns the width drawn.
fn text(cv: &mut Cv, th: &Theme, x: i32, y: i32, s: &str, c: Rgb) -> i32 {
    match th.font {
        Font::Crisp => {
            hud::draw_text_crisp(cv.f, (cv.w as u32, cv.h as u32), (x / 2, y / 2), s, [c[0], c[1], c[2], 255]);
            hud::text_width(s) * 2
        }
        Font::Mixed => mixed(cv, 2, x, y, s, c),
        Font::Chunky => chunky(cv, th, 2, x, y, s, c),
    }
}

fn text_w(th: &Theme, s: &str) -> i32 {
    match th.font {
        Font::Crisp | Font::Mixed => hud::text_width(s) * 2,
        Font::Chunky => hud::text_width(s) * 2 + 4,
    }
}

/// A heading `k` times the dot grid.
fn big(cv: &mut Cv, th: &Theme, k: i32, x: i32, y: i32, s: &str, c: Rgb) -> i32 {
    match th.font {
        Font::Crisp => {
            hud::draw_text_scaled(cv.f, (cv.w as u32, cv.h as u32), k, (x / k, y / k), s, [c[0], c[1], c[2], 255]);
            hud::text_width(s) * k
        }
        Font::Mixed => mixed(cv, k, x, y, s, c),
        Font::Chunky => chunky(cv, th, k, x, y, s, c),
    }
}

fn big_w(th: &Theme, k: i32, s: &str) -> i32 {
    hud::text_width(s) * k + if th.font == Font::Chunky { 2 * k } else { 0 }
}

fn mixed(cv: &mut Cv, k: i32, x: i32, y: i32, s: &str, c: Rgb) -> i32 {
    let (x, y) = (x - x.rem_euclid(k), y - y.rem_euclid(k));
    let mut cx = x;
    let mut buf = [0u8; 4];
    for ch in s.chars() {
        if let Some(g) = lower(ch) {
            for (r, bits) in g.iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        cv.rect(cx + col * k, y + r as i32 * k, k, k, c, 1.0);
                    }
                }
            }
        } else {
            let one = ch.encode_utf8(&mut buf);
            hud::draw_text_scaled(cv.f, (cv.w as u32, cv.h as u32), k, (cx / k, y / k), one, [c[0], c[1], c[2], 255]);
        }
        cx += 6 * k;
    }
    hud::text_width(s) * k
}

fn chunky(cv: &mut Cv, th: &Theme, k: i32, x: i32, y: i32, s: &str, c: Rgb) -> i32 {
    let ink = th.ink.unwrap_or([0, 0, 0]);
    let (x, y) = (x - x.rem_euclid(k) + k, y - y.rem_euclid(k) + k);
    let size = (cv.w as u32, cv.h as u32);
    let ik = [ink[0], ink[1], ink[2], 255];
    for (ox, oy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1), (1, 2), (0, 2)] {
        hud::draw_text_scaled(cv.f, size, k, (x / k + ox, y / k + oy), s, ik);
    }
    hud::draw_text_scaled(cv.f, size, k, (x / k, y / k), s, [c[0], c[1], c[2], 255]);
    hud::text_width(s) * k + 2 * k
}

/// Line heights: body, and the gap a row of body text takes.
fn line_h(th: &Theme) -> i32 {
    match th.font {
        Font::Crisp => 18,
        Font::Mixed => 20,
        Font::Chunky => 20,
    }
}

/// Lesson lines arrive in capitals; the mixed font sets them in sentence
/// case. Capitals stay where a sentence starts, and for a single-letter key
/// name ("N: DISH 1").
fn sentence(lines: &[&str]) -> Vec<String> {
    let mut cap = true;
    let mut out = Vec::new();
    for l in lines {
        let chars: Vec<char> = l.chars().collect();
        let mut s = String::new();
        for (k, &ch) in chars.iter().enumerate() {
            let solo = ch.is_ascii_alphabetic()
                && (k == 0 || !chars[k - 1].is_ascii_alphabetic())
                && chars.get(k + 1) == Some(&':');
            if ch.is_ascii_alphabetic() {
                s.push(if cap || solo { ch } else { ch.to_ascii_lowercase() });
                cap = false;
            } else {
                s.push(ch);
                if ch == '.' || ch == '!' || ch == '?' {
                    cap = true;
                }
            }
        }
        out.push(s);
    }
    out
}

// ------------------------------------------------------------------ icons

const ICON_LAMP: [&str; 12] = [
    ".....aa.....",
    ".a...aa...a.",
    "..a......a..",
    "....kkkk....",
    "...kbbbak...",
    "aa.kbbaak.aa",
    "aa.kbaaak.aa",
    "...kaaaak...",
    "..a.kkkk.a..",
    ".a........a.",
    ".....aa.....",
    ".....aa.....",
];
const ICON_OATS: [&str; 12] = [
    "............",
    "..kkk.......",
    ".kbbak......",
    ".kbaak..kkk.",
    "..kaak.kbbak",
    "...kk..kbaak",
    ".......kaaak",
    "...kkk..kkk.",
    "..kbbak.....",
    "..kbaaak....",
    "...kaaak....",
    "....kkk.....",
];
const ICON_SALT: [&str; 12] = [
    "............",
    "....kkkk....",
    "....kbbk....",
    "....kbak....",
    "....kkkk....",
    "............",
    ".kkkk..kkkk.",
    ".kbbk..kbbk.",
    ".kbak..kbak.",
    ".kkkk..kkkk.",
    "............",
    "............",
];
const ICON_SNIP: [&str; 12] = [
    "a.........a.",
    ".a.......a..",
    "..a.....a...",
    "...a...a....",
    "....a.a.....",
    ".....k......",
    "....k.k.....",
    "..kkk.kkk...",
    ".kbbk.kbbk..",
    "kb..k.k..bk.",
    "kb..k.k..bk.",
    ".kkk...kkk..",
];
const STAR: [&str; 9] = [
    "....#....",
    "...###...",
    "...###...",
    "#########",
    ".#######.",
    "..#####..",
    "..#####..",
    ".###.###.",
    ".##...##.",
];

fn tool_colour(th: &Theme, t: Tool) -> Rgb {
    match (th.style, t) {
        (Style::Toy, Tool::Lamp) => [255, 214, 90],
        (Style::Toy, Tool::Oats) => [236, 206, 150],
        (Style::Toy, Tool::Salt) => [190, 224, 240],
        (Style::Toy, Tool::Snip) => [240, 124, 104],
        (_, Tool::Lamp) => th.lamp,
        (_, Tool::Oats) => th.oats,
        (_, Tool::Salt) => [236, 242, 250],
        (_, Tool::Snip) => th.bad,
    }
}

/// A 12x12 icon at `k` pixels a dot. `faded` draws it greyed (spent).
fn icon(cv: &mut Cv, th: &Theme, t: Tool, x: i32, y: i32, k: i32, faded: bool) {
    let rows = match t {
        Tool::Lamp => &ICON_LAMP,
        Tool::Oats => &ICON_OATS,
        Tool::Salt => &ICON_SALT,
        Tool::Snip => &ICON_SNIP,
    };
    let (a, b, ink) = match t {
        Tool::Lamp => ([255, 210, 70], [255, 250, 210], th.ink.unwrap_or([120, 90, 30])),
        Tool::Oats => ([226, 196, 130], [252, 240, 200], th.ink.unwrap_or([120, 92, 50])),
        Tool::Salt => ([210, 222, 236], [255, 255, 255], th.ink.unwrap_or([90, 100, 116])),
        Tool::Snip => ([200, 210, 220], [240, 110, 90], th.ink.unwrap_or([70, 74, 82])),
    };
    for (r, row) in rows.iter().enumerate() {
        for (cidx, ch) in row.bytes().enumerate() {
            let c: Rgb = match ch {
                b'a' => a,
                b'b' => b,
                b'k' => ink,
                _ => continue,
            };
            let c = if faded { lerp(c, [90, 90, 90], 0.7) } else { c };
            cv.rect(x + cidx as i32 * k, y + r as i32 * k, k, k, c, 1.0);
        }
    }
}

fn star(cv: &mut Cv, th: &Theme, x: i32, y: i32, k: i32, on: bool) {
    let (fill, ink) = if on { (th.hot, th.ink) } else { (mul(th.dim, 0.6), th.ink) };
    if let Some(ink) = ink {
        for (r, row) in STAR.iter().enumerate() {
            for (c, ch) in row.bytes().enumerate() {
                if ch == b'#' {
                    cv.rect(x + c as i32 * k - k / 2, y + r as i32 * k - k / 2, k * 2, k * 2, ink, 1.0);
                }
            }
        }
    }
    for (r, row) in STAR.iter().enumerate() {
        for (c, ch) in row.bytes().enumerate() {
            if ch == b'#' {
                cv.rect(x + c as i32 * k, y + r as i32 * k, k, k, fill, 1.0);
            }
        }
    }
}

// ----------------------------------------------------------------- status

/// What the HUD reads, gathered once a frame. The joined piles are the best
/// piece's, the same search `Dish::measure` runs (and only that much of it:
/// the HUD used to run the whole measure, Prim's tree and all, every frame).
struct Status {
    /// Per dish pile, in `Dish::dish_piles` order: in the best-joined piece.
    joined: Vec<bool>,
    reach: u32,
    coming: u32,
    lesson: bool,
    left: u32,
    alarm: bool,
    /// Alarm's flash phase: on for a quarter second, off for a quarter.
    flash: bool,
}

/// The pulse along fed tube (`render.rs`'s `pulse`, the same numbers, so
/// every look shows food moving the same way): the colour a band blends
/// toward, how far, cells a second, cells between bands, and the share of
/// that spacing its tail covers.
const PULSE: Rgb = [255, 252, 230];
const PULSE_STRENGTH: f32 = 0.45;
const PULSE_SPEED: f32 = 45.0;
const PULSE_SPACING: f32 = 60.0;
const PULSE_TAIL: f32 = 0.3;
/// The band's leading edge, as a share of the spacing: a ramp up over
/// about four cells instead of a jump from nothing to full in one.
const PULSE_RISE: f32 = 0.07;
/// Cells from a pile inside which the band's distance is measured
/// straight from the pile's edge rather than by the feeding search.
const PULSE_ROUND: f32 = 12.0;

/// The pulse's brightness on a cell, 0 to 1: a band riding out from the
/// food along the feeding search's own distances (`Dish::fed`), so a
/// starving branch's beat stops at the cut.
fn pulse(d: &Dish, i: usize) -> f32 {
    let dist = d.fed[i];
    if dist == u16::MAX {
        return 0.0;
    }
    // The feeding search steps four ways, so its distances round a pile
    // are diamonds: every band left the pile as a pale square of 4-pixel
    // blocks round it (review 1, "the blocky square halo"). Near a pile
    // the distance is the straight one from the pile's edge, handed back
    // to the search's own by PULSE_ROUND cells out, so a cut still stops
    // the beat.
    let mut dist = dist as f32;
    if dist < PULSE_ROUND {
        if let Some(p) = d.piles.get(d.fed_src[i] as usize) {
            let (x, y) = ((i % W) as f32 + 0.5, (i / W) as f32 + 0.5);
            let straight = ((x - p.x).hypot(y - p.y) - (p.r + 1.0)).max(0.0);
            let w = dist / PULSE_ROUND;
            dist = straight * (1.0 - w) + dist * w;
        }
    }
    let secs = d.t as f32 / STEPS_PER_SECOND as f32;
    let phase = (secs * PULSE_SPEED - dist) / PULSE_SPACING;
    let s = phase - phase.floor();
    if s < PULSE_TAIL {
        1.0 - s / PULSE_TAIL
    } else if s > 1.0 - PULSE_RISE {
        (s - (1.0 - PULSE_RISE)) / PULSE_RISE
    } else {
        0.0
    }
}

/// The Wanderer's crack (`wander::in_exit`) on bare agar: a cool glow,
/// brighter toward the rim, beating once fed mould stands in it and Go is
/// open; full while she pours. Teal, never tan, yellow or grey.
fn exit_colour(th: &Theme, d: &Dish, w: &Wander, x: usize, y: usize, agar: Rgb) -> Rgb {
    let depth = (x as f32 - wander::EXIT_X as f32) / (W - wander::EXIT_X) as f32;
    let edge = 1.0 - (y as f32 + 0.5 - wander::EXIT_Y).abs() / wander::EXIT_HALF;
    let mut g = 0.35 + 0.45 * depth.clamp(0.0, 1.0) + 0.2 * edge.clamp(0.0, 1.0);
    if w.touching && d.phase == Phase::Growing {
        let secs = d.t as f32 / STEPS_PER_SECOND as f32;
        g *= 0.8 + 0.2 * ((secs * 4.0).sin() * 0.5 + 0.5);
    }
    if matches!(d.phase, Phase::Crossing(_)) {
        g = g.max(0.9);
    }
    lerp(agar, exit_teal(th), g)
}

fn exit_teal(th: &Theme) -> Rgb {
    match th.style {
        Style::Glow => [64, 214, 198],
        Style::Toy => [96, 214, 200],
        _ => [70, 190, 178],
    }
}

fn cell_at(x: f32, y: f32) -> Option<usize> {
    if x < 0.0 || y < 0.0 {
        return None;
    }
    let (ix, iy) = (x as usize, y as usize);
    (ix < W && iy < H).then_some(iy * W + ix)
}

fn status(d: &Dish) -> Status {
    let (label, n) = d.pieces();
    let piles: Vec<_> = d.dish_piles().collect();
    let mut touched: Vec<Vec<usize>> = vec![Vec::new(); n as usize + 1];
    for (pi, p) in piles.iter().enumerate() {
        let mut seen = std::collections::BTreeSet::new();
        for (ox, oy) in disc(p.r + 1.0) {
            if let Some(i) = cell_at(p.x + ox, p.y + oy) {
                if label[i] != 0 {
                    seen.insert(label[i]);
                }
            }
        }
        for l in seen {
            touched[l as usize].push(pi);
        }
    }
    let mut best: Vec<usize> = Vec::new();
    for t in touched {
        if t.len() > best.len() {
            best = t;
        }
    }
    let mut joined = vec![false; piles.len()];
    for &p in &best {
        joined[p] = true;
    }
    let sps = STEPS_PER_SECOND as u64;
    let lesson = d.storm_at == u64::MAX;
    let left = d.seconds_left();
    Status {
        reach: best.len() as u32,
        joined,
        coming: d.pending.len() as u32,
        lesson,
        left,
        alarm: d.phase == Phase::Growing && !lesson && left <= STORM_WARNING_SECONDS,
        flash: (d.t / (sps / 4)).is_multiple_of(2),
    }
}

/// (have, of, share of the next one grown) for a tool. The dish's own
/// stock: a challenge dish (`level.rs`) sets its own.
fn stock(d: &Dish, t: Tool) -> Option<(u32, u32, f32)> {
    let sps = STEPS_PER_SECOND as u64;
    match t {
        Tool::Lamp => None,
        Tool::Oats => {
            let every = d.oats_regrow as u64 * sps;
            let grown = if every > 0 && d.oats_left < d.oats_stock {
                (d.t % every) as f32 / every as f32
            } else {
                0.0
            };
            Some((d.oats_left, d.oats_stock, grown))
        }
        Tool::Salt => Some((d.salt_left, d.level.map_or(SALT_STOCK, |l| l.tools.salt), 0.0)),
        Tool::Snip => Some((d.snips_left, d.level.map_or(SNIP_STOCK, |l| l.tools.snips), 0.0)),
    }
}

/// Seconds to the next oat back, or `None` where none come back.
fn oats_wait(d: &Dish) -> Option<u64> {
    let sps = STEPS_PER_SECOND as u64;
    let every = d.oats_regrow as u64 * sps;
    (every > 0 && d.oats_left < d.oats_stock).then(|| (every - d.t % every).div_ceil(sps))
}

/// One of the game's capital lines in sentence case (the mixed font's), with
/// a ratio's "X" set small ("0.88x the ruler"): `sentence` keeps a lone
/// capital as a key's name.
/// A line that opens on a number keeps its first word small ("400 mould
/// alive at the bell").
fn up(s: &str) -> String {
    let line = sentence(&[s]).remove(0);
    let number_first = line.starts_with(|c: char| c.is_ascii_digit());
    let mut out = String::with_capacity(line.len());
    let mut prev = ' ';
    let mut first_word = true;
    let chars: Vec<char> = line.chars().collect();
    for (k, &c) in chars.iter().enumerate() {
        let lone = !chars.get(k + 1).is_some_and(|n| n.is_alphabetic());
        let c = if c == 'X' && prev.is_ascii_digit() {
            'x'
        } else if number_first && first_word && c.is_ascii_uppercase() && prev == ' ' && !lone {
            first_word = false;
            c.to_ascii_lowercase()
        } else {
            c
        };
        out.push(c);
        prev = c;
    }
    out
}

fn clock(secs: u32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// What the next press does, in two short lines (the corner chip).
fn hint(d: &Dish, view: &View, st: &Status) -> [String; 2] {
    let two = |a: &str, b: &str| [a.to_string(), b.to_string()];
    if let Some(w) = &d.wander {
        match d.phase {
            Phase::Over if matches!(w.outcome, Some(Outcome::Arrived { .. })) => {
                return two("N: next dish, with her", "R: try this dish again");
            }
            Phase::Over => return two("R: try this dish again", "N: a new journey"),
            Phase::Crossing(_) => {
                return [
                    format!("Through {}", thousands(w.through)),
                    format!("In line {}, died {}", thousands(w.pouring.len() as u32), thousands(w.lost_on_way)),
                ]
            }
            _ if d.can_go() => return two("G: go now. She leaves", "through the crack"),
            _ => {}
        }
    }
    match d.phase {
        Phase::Over => two("Dish over.", "R: again  N: next  M: dishes"),
        Phase::Storm(_) | Phase::Crossing(_) => two("The storm struck.", "Watch what stays joined"),
        Phase::Growing if d.storm_line.is_some() && !st.lesson => {
            [format!("Storm in {} s:", st.left), "it cuts the marked line".into()]
        }
        Phase::Growing => match view.tool {
            Tool::Lamp => two("Hold the mouse:", "light pushes the mould"),
            Tool::Oats => two("Click: drop oats.", "The mould grows to them"),
            Tool::Salt => two("Drag: a salt wall", "the mould can't cross"),
            Tool::Snip => two("Drag across a tube", "to cut it"),
        },
    }
}

fn keys_lines(view: &View) -> [&'static str; 2] {
    if view.paused {
        ["Paused", "P: go"]
    } else if view.fast {
        ["Fast, 4x", "Space: normal"]
    } else {
        ["Space: fast  P: pause", "R: again  H: help  M: dishes"]
    }
}

// ------------------------------------------------------------------- draw

pub fn draw(d: &Dish, view: &View, scale: u32, frame: &mut [u8]) {
    let (fw, fh) = frame_size(scale);
    debug_assert_eq!(frame.len(), (fw * fh * 4) as usize);
    let th = theme(view.style);
    let mut cv = Cv {
        f: frame,
        w: fw as i32,
        h: fh as i32,
    };
    let s = scale as i32;
    let st = status(d);
    dish(&mut cv, d, &th, s, &st);
    piles(&mut cv, d, &th, s, &st);
    marks(&mut cv, d, &th, s);
    storm(&mut cv, d, &th, s, &st);
    exit_label(&mut cv, d, &th, s);
    ruler(&mut cv, d, &th, s);
    lamp(&mut cv, d, &th, s);
    preview(&mut cv, d, view, &th, s);
    corners(&mut cv, d, view, &th, &st, s);
    hud_strip(&mut cv, d, view, &th, s, &st);
    // The card covers the dish; the banner behind it was clutter.
    if let (Some(b), false) = (view.banner, view.brief) {
        banner(&mut cv, &th, &b, d);
    }
    alarm(&mut cv, d, view, &th, s, &st);
    if d.phase == Phase::Over {
        arrival(&mut cv, d, &th);
    }
    if view.brief {
        brief(&mut cv, d, view, &th, s);
    }
    if let Some(m) = &view.menu {
        menu(&mut cv, &th, m);
    }
    if view.quit_armed {
        // The first Escape on a dish (review 1): a second within 2 s quits.
        let words = "Press Esc again to quit";
        let w = text_w(&th, words) + 16;
        let y = H as i32 * s - line_h(&th) - 22;
        let x = (cv.w - w) / 2;
        chip(&mut cv, &th, x, y, &[words], th.hot);
    }
    // The debug corner: which look this is, by its blind letter only.
    hud::draw_text(cv.f, fw, fh, fw as i32 - 8, fh as i32 - 9, view.style.letter(), [90, 96, 100, 255]);
}

// ------------------------------------------------------------------- dish

/// Half the dish's width and height: the glow fields' grid.
const HW: usize = W / 2;
const HH: usize = H / 2;

/// The glow for `Theme::glow`: fed tube, starving tube and food, each a
/// mask box-blurred twice. **On a half-size grid**, read back bilinearly:
/// at full size the two tube masks cost 0.70 ms a frame of the 3.4 ms the
/// look took (seed 1 at 40 s), and the piles' own glow, drawn pixel by
/// pixel around each pile, another 0.2 ms; a glow is soft by nature, so
/// the half grid shows no blocks. The food's glow rides in the same pass.
struct Halo {
    fed: Vec<f32>,
    pale: Vec<f32>,
    food: Vec<f32>,
}

impl Halo {
    fn new(d: &Dish, thr: f32) -> Halo {
        let mut h = Halo {
            fed: vec![0.0; HW * HH],
            pale: vec![0.0; HW * HH],
            food: vec![0.0; HW * HH],
        };
        for y in 0..H {
            for x in 0..W {
                let i = y * W + x;
                if d.ground[i] == Ground::Agar && d.trail[i] > thr {
                    let m = if d.looks_fed(i) { &mut h.fed } else { &mut h.pale };
                    m[(y / 2) * HW + x / 2] += 0.25;
                }
            }
        }
        for p in d.piles.iter().filter(|p| !p.oats) {
            let w = 0.25 * (0.4 + 0.6 * (p.food / p.full).clamp(0.0, 1.0));
            for (ox, oy) in disc(p.r) {
                if let Some(i) = cell_at(p.x + ox, p.y + oy) {
                    h.food[(i / W / 2) * HW + (i % W) / 2] += w;
                }
            }
        }
        let mut tmp = vec![0.0f32; HW * HH];
        for a in [&mut h.fed, &mut h.pale, &mut h.food] {
            for _ in 0..2 {
                blur(a, &mut tmp, HW, HH, 2);
            }
        }
        h
    }

    /// **Worked out every other frame**: a glow 1/60 s old cannot be seen,
    /// and this was the look's largest single cost after the dish itself.
    /// Kept per look and per dish (by its address, so the window's one dish
    /// hits and a picture of another dish does not) and reused while the
    /// dish is under four steps on, or paused.
    fn cached(d: &Dish, th: &Theme, thr: f32) -> std::sync::Arc<Halo> {
        use std::sync::{Arc, Mutex};
        type Slot = Option<(usize, u64, Arc<Halo>)>;
        static CACHE: Mutex<[Slot; 5]> = Mutex::new([const { None }; 5]);
        let k = Style::ALL.iter().position(|s| *s == th.look).unwrap_or(0);
        let addr = d as *const Dish as usize;
        let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((a, t, h)) = &cache[k] {
            if *a == addr && d.t >= *t && d.t < *t + 4 {
                return h.clone();
            }
        }
        let h = Arc::new(Halo::new(d, thr));
        cache[k] = Some((addr, d.t, h.clone()));
        h
    }

    /// The three fields at a dish cell, read between half-grid centres.
    #[inline]
    fn at(&self, x: usize, y: usize) -> (f32, f32, f32) {
        let fx = (x as f32 + 0.5) * 0.5 - 0.5;
        let fy = (y as f32 + 0.5) * 0.5 - 0.5;
        let (x0, y0) = (fx.max(0.0) as usize, fy.max(0.0) as usize);
        let (x1, y1) = ((x0 + 1).min(HW - 1), (y0 + 1).min(HH - 1));
        let (ax, ay) = ((fx - x0 as f32).clamp(0.0, 1.0), (fy - y0 as f32).clamp(0.0, 1.0));
        let k = [y0 * HW + x0, y0 * HW + x1, y1 * HW + x0, y1 * HW + x1];
        let wt = [(1.0 - ax) * (1.0 - ay), ax * (1.0 - ay), (1.0 - ax) * ay, ax * ay];
        let mut o = (0.0, 0.0, 0.0);
        for (k, w) in k.iter().zip(wt) {
            o.0 += self.fed[*k] * w;
            o.1 += self.pale[*k] * w;
            o.2 += self.food[*k] * w;
        }
        o
    }
}

fn blur(a: &mut [f32], tmp: &mut [f32], w: usize, h: usize, r: usize) {
    let n = (2 * r + 1) as f32;
    for y in 0..h {
        let row = &a[y * w..(y + 1) * w];
        let mut sum: f32 = row[..r.min(w)].iter().sum();
        for x in 0..w {
            if x + r < w {
                sum += row[x + r];
            }
            if x > r {
                sum -= row[x - r - 1];
            }
            tmp[y * w + x] = sum / n;
        }
    }
    for x in 0..w {
        let mut sum = 0.0f32;
        for y in 0..r.min(h) {
            sum += tmp[y * w + x];
        }
        for y in 0..h {
            if y + r < h {
                sum += tmp[(y + r) * w + x];
            }
            if y > r {
                sum -= tmp[(y - r - 1) * w + x];
            }
            a[y * w + x] = sum / n;
        }
    }
}

fn ellipse(x: usize, y: usize) -> (f32, f32, f32) {
    let dx = (x as f32 + 0.5 - CX) / RX;
    let dy = (y as f32 + 0.5 - CY) / RY;
    (dx * dx + dy * dy, dx / RX, dy / RY)
}

fn rim_colour(th: &Theme, x: usize, y: usize, st: &Status) -> Rgb {
    let (e, nx, ny) = ellipse(x, y);
    let n = nx.hypot(ny).max(1e-6);
    // Light from the top left: the lip facing it is bright.
    let lit = -(nx + ny) / n * std::f32::consts::FRAC_1_SQRT_2;
    let alarm = st.alarm && st.flash;
    let lip = |c: Rgb| if alarm { lerp(c, th.storm, 0.85) } else { c };
    match th.style {
        Style::Glow => {
            if e < 1.035 {
                lip(lerp(th.rim_dark, th.rim_light, 0.25 + 0.5 * (lit + 1.0) / 2.0))
            } else {
                let h = hash(x as u32, y as u32);
                if h.is_multiple_of(211) {
                    [22, 26, 34]
                } else {
                    th.outside
                }
            }
        }
        Style::Toy => {
            let ink = th.ink.unwrap_or([0, 0, 0]);
            if e < 1.02 {
                ink
            } else if e < 1.075 {
                lip(lerp(th.rim_dark, th.rim_light, 0.55 + 0.45 * lit))
            } else if e < 1.095 {
                ink
            } else {
                // Wood: streaks along the grain.
                let g = ((y as f32 * 0.9 + (x as f32 * 0.05).sin() * 3.0) as u32).wrapping_mul(2_654_435_761) >> 29;
                mul(th.outside, 0.86 + g as f32 * 0.035)
            }
        }
        _ => {
            if e < 1.05 {
                lip(lerp(th.rim_dark, th.rim_light, {
                    let l = (lit + 1.0) / 2.0;
                    l * l
                }))
            } else if x.is_multiple_of(8) || y.is_multiple_of(8) {
                [24, 28, 32]
            } else {
                th.outside
            }
        }
    }
}

/// The bare dish and what is outside it, which never change within a
/// look: worked out once per look and kept (it was a tenth of the dish's
/// cost every frame).
/// `flash`: the rim lit by the storm alarm's flash, kept likewise (worked
/// out each flash frame it was 0.3 ms of the storm's last seconds).
fn background(th: &Theme, flash: bool) -> &'static [Rgb] {
    use std::sync::OnceLock;
    static CACHE: [OnceLock<Vec<Rgb>>; 10] = [const { OnceLock::new() }; 10];
    let k = Style::ALL.iter().position(|s| *s == th.style).unwrap_or(0) * 2 + flash as usize;
    CACHE[k].get_or_init(|| {
        let calm = Status {
            joined: Vec::new(),
            reach: 0,
            coming: 0,
            lesson: false,
            left: 0,
            alarm: flash,
            flash,
        };
        let mut v = Vec::with_capacity(W * H);
        for y in 0..H {
            for x in 0..W {
                let (e, _, _) = ellipse(x, y);
                v.push(if e <= 1.0 { agar_colour(th, x, y) } else { rim_colour(th, x, y, &calm) });
            }
        }
        v
    })
}

fn agar_colour(th: &Theme, x: usize, y: usize) -> Rgb {
    let (e, _, _) = ellipse(x, y);
    let h = hash(x as u32, y as u32);
    match th.style {
        Style::Glow => {
            let c = lerp(th.agar, th.agar_edge, e * e.sqrt());
            if h.is_multiple_of(97) {
                mul(c, 1.6)
            } else {
                c
            }
        }
        // Flat, as a picture book is (and a GIF of grain is three times
        // the size of one without).
        Style::Toy => lerp(th.agar, th.agar_edge, e.powi(3)),
        _ => mul(lerp(th.agar, th.agar_edge, e * e), 1.0 - (h >> 30) as f32 * 0.02),
    }
}

fn salt_colour(th: &Theme, d: &Dish, x: usize, y: usize) -> Rgb {
    let h = hash(x as u32, y as u32);
    match th.style {
        Style::Glow => {
            let sparkle = hash(x as u32 ^ (d.t / 24) as u32, y as u32).is_multiple_of(29);
            if sparkle {
                [255, 255, 255]
            } else {
                mul([150, 196, 236], 0.7 + (h >> 30) as f32 * 0.1)
            }
        }
        Style::Toy => {
            let edge = [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                nx >= 0
                    && ny >= 0
                    && (nx as usize) < W
                    && (ny as usize) < H
                    && d.ground[ny as usize * W + nx as usize] == Ground::Agar
            });
            if edge {
                th.ink.unwrap_or([0, 0, 0])
            } else {
                mul([250, 252, 255], 1.0 - (h >> 30) as f32 * 0.03)
            }
        }
        _ => {
            let k = 238 - (h >> 28) as u8 * 3;
            [k - 10, k - 4, k]
        }
    }
}

fn scar_colour(th: &Theme, x: usize, y: usize, t: u64) -> Rgb {
    let h = hash(x as u32, y as u32);
    match th.style {
        Style::Glow => {
            let f = ((t as f32 * 0.08 + x as f32 * 0.7 + y as f32 * 0.3).sin() + 1.0) / 2.0;
            [(110.0 + 70.0 * f) as u8, 22, (34.0 + 10.0 * f) as u8]
        }
        Style::Toy => mul([66, 40, 34], 1.0 - (h >> 30) as f32 * 0.08),
        _ => mul([100, 26, 22], 0.85 + (h >> 30) as f32 * 0.08),
    }
}

fn pulsed(c: Rgb, beat: f32) -> Rgb {
    if beat > 0.0 {
        lerp(c, PULSE, beat * PULSE_STRENGTH)
    } else {
        c
    }
}

/// An open snip (`Ground::Wound`): raw and dark while open, closing back
/// to the agar under it over its last second, as `render.rs` draws it, so
/// the cut is seen to heal rather than vanish.
fn wound_colour(th: &Theme, d: &Dish, i: usize, agar: Rgb) -> Rgb {
    let left = d.wound_until[i].saturating_sub(d.t) as f32 / STEPS_PER_SECOND as f32;
    let raw = match th.style {
        Style::Glow => [132, 26, 58],
        Style::Toy => [120, 30, 44],
        _ => [92, 18, 40],
    };
    lerp(agar, raw, left.clamp(0.0, 1.0))
}

fn tube_colour(th: &Theme, v: f32, fed: bool, shade: f32) -> Rgb {
    let stops: &[Rgb] = if fed { &th.fed } else { &th.pale };
    match th.style {
        Style::Toy => {
            // Cel shading: one flat tone (a paler one for a thin feeler), a
            // light edge and a dark one.
            let q = if v < 0.2 { 0.5 } else { 0.72 };
            let base = ramp(stops, q);
            if shade > 0.4 {
                lerp(base, [255, 255, 245], 0.5)
            } else if shade < -0.4 {
                mul(base, 0.8)
            } else {
                base
            }
        }
        _ => {
            let base = ramp(stops, v);
            if th.emboss <= 0.0 {
                base
            } else if shade > 0.0 {
                lerp(base, [255, 255, 236], shade * th.emboss * 0.55)
            } else {
                mul(base, 1.0 + shade * th.emboss * 0.4)
            }
        }
    }
}

/// The dish, cell by cell. Cells on a tube's edge are drawn pixel by pixel
/// from the trail sampled between cell centres, so a tube has a smooth edge
/// rather than a staircase of 4x4 blocks; every other cell is one block.
/// Measured: about one cell in six is an edge cell in a full dish.
fn dish(cv: &mut Cv, d: &Dish, th: &Theme, s: i32, st: &Status) {
    let thr = d.params.tube * 0.5;
    let top = (1.0 + TRAIL_TOP).ln();
    let lo = (1.0 + thr).ln() / top;
    // The ramp position of a trail value, from a table: a logarithm per
    // edge pixel was a third of the dish's cost.
    let lut: Vec<f32> = (0..1024)
        .map(|k| {
            let t = k as f32 / 16.0;
            (((1.0 + t).ln() / top - lo) / (1.0 - lo)).clamp(0.0, 1.0)
        })
        .collect();
    let vof = |t: f32| lut[((t * 16.0) as usize).min(1023)];
    let halo = (th.glow > 0.0).then(|| Halo::cached(d, th, thr));
    // Trail on open dish only, padded by two cells of nothing all round.
    const PAD: usize = 2;
    let pw = W + 2 * PAD;
    let ph = H + 2 * PAD;
    let mut raw = vec![0.0f32; pw * ph];
    for y in 0..H {
        for x in 0..W {
            let i = y * W + x;
            if d.ground[i] == Ground::Agar {
                raw[(y + PAD) * pw + x + PAD] = d.trail[i];
            }
        }
    }
    // Softened once (1-2-1 each way) for drawing only, so a tube reads as
    // gel rather than as the trail's cell-by-cell grain. The feeding search
    // and the score still read the raw trail.
    let mut across = vec![0.0f32; pw * ph];
    for y in 0..ph {
        for x in 1..pw - 1 {
            let k = y * pw + x;
            across[k] = (raw[k - 1] + 2.0 * raw[k] + raw[k + 1]) * 0.25;
        }
    }
    let mut padded = vec![0.0f32; pw * ph];
    for y in 1..ph - 1 {
        for x in 0..pw {
            let k = y * pw + x;
            padded[k] = (across[k - pw] + 2.0 * across[k] + across[k + pw]) * 0.25;
        }
    }
    let tr = |x: i32, y: i32| -> f32 { padded[(y + PAD as i32) as usize * pw + (x + PAD as i32) as usize] };
    let shadow_of_salt = th.style != Style::Glow;
    let sf = s as f32;
    let bg = background(th, false);
    let rim = background(th, st.alarm && st.flash);
    for y in 0..H {
        for x in 0..W {
            let i = y * W + x;
            let (xi, yi) = (x as i32, y as i32);
            let mut tube: Option<Rgb> = None;
            let mut edge = false;
            let mut fed = false;
            let mut shade = 0.0;
            let mut beat = 0.0;
            let ground = match d.ground[i] {
                Ground::Rim => rim[i],
                Ground::Salt => salt_colour(th, d, x, y),
                Ground::Scar => scar_colour(th, x, y, d.t),
                Ground::Wound => wound_colour(th, d, i, bg[i]),
                Ground::Agar => {
                    let mut c = bg[i];
                    if let Some(w) = &d.wander {
                        if wander::in_exit(x, y) {
                            c = exit_colour(th, d, w, x, y, c);
                        }
                    }
                    if shadow_of_salt && x > 0 && y > 0 && d.ground[i - W - 1] == Ground::Salt {
                        c = mul(c, 0.68);
                    }
                    if let Some(h) = &halo {
                        let (gf, gp, gd) = h.at(x, y);
                        if gf + gp + gd > 0.004 {
                            for (k, ck) in c.iter_mut().enumerate() {
                                let add = th.fed[2][k] as f32 * gf * th.glow * 0.55
                                    + th.pale[1][k] as f32 * gp * th.glow * 0.5
                                    + th.food[k] as f32 * gd * th.glow * 0.7;
                                *ck = (*ck as f32 + add).min(255.0) as u8;
                            }
                        }
                    }
                    let t0 = tr(xi, yi);
                    let is_tube = t0 > thr;
                    let nb = [(xi - 1, yi), (xi + 1, yi), (xi, yi - 1), (xi, yi + 1)];
                    let mut any = false;
                    let mut all = true;
                    let mut nfed = false;
                    for (nx, ny) in nb {
                        let t = tr(nx, ny);
                        if t > thr {
                            any = true;
                            if !nfed && d.looks_fed(ny as usize * W + nx as usize) {
                                nfed = true;
                                if !is_tube {
                                    beat = pulse(d, ny as usize * W + nx as usize);
                                }
                            }
                        } else {
                            all = false;
                        }
                    }
                    edge = (is_tube && !all) || (!is_tube && any);
                    if is_tube || any {
                        fed = if is_tube { d.looks_fed(i) } else { nfed };
                        // Two cells either side, so the light reads the
                        // tube's shape rather than the trail's grain.
                        shade = ((tr(xi + 2, yi + 2) - tr(xi - 2, yi - 2)) / (t0 + 6.0)).clamp(-1.0, 1.0);
                    }
                    if is_tube && fed {
                        beat = pulse(d, i);
                    }
                    if !fed {
                        beat = 0.0;
                    }
                    if is_tube {
                        tube = Some(pulsed(tube_colour(th, vof(t0), fed, shade), beat));
                    }
                    c
                }
            };
            let (px0, py0) = (xi * s, yi * s);
            if !edge {
                let c = tube.unwrap_or(ground);
                let px4 = [c[0], c[1], c[2], 255];
                for py in 0..s {
                    let row = ((py0 + py) as usize * cv.w as usize + px0 as usize) * 4;
                    for p in cv.f[row..row + 4 * s as usize].chunks_exact_mut(4) {
                        p.copy_from_slice(&px4);
                    }
                }
                continue;
            }
            for py in 0..s {
                for px in 0..s {
                    let fx = x as f32 + (px as f32 + 0.5) / sf - 0.5;
                    let fy = y as f32 + (py as f32 + 0.5) / sf - 0.5;
                    let (ix, iy) = (fx.floor() as i32, fy.floor() as i32);
                    let (ax, ay) = (fx - ix as f32, fy - iy as f32);
                    let v = (tr(ix, iy) * (1.0 - ax) + tr(ix + 1, iy) * ax) * (1.0 - ay)
                        + (tr(ix, iy + 1) * (1.0 - ax) + tr(ix + 1, iy + 1) * ax) * ay;
                    let c = if v > thr {
                        match th.ink {
                            Some(ink) if v < thr * 1.9 => ink,
                            _ => pulsed(tube_colour(th, vof(v), fed, shade), beat),
                        }
                    } else {
                        ground
                    };
                    cv.put(px0 + px, py0 + py, c);
                }
            }
        }
    }
}

// -------------------------------------------------------------- the piles

fn grain_disc(cv: &mut Cv, cx: f32, cy: f32, r: f32, c: Rgb, a: f32) {
    cv.around(cx, cy, r + 1.0, |cv, x, y, d| {
        let h = hash(x as u32, y as u32);
        let g = if h.is_multiple_of(5) {
            0.8
        } else if h.is_multiple_of(7) {
            1.1
        } else {
            1.0
        };
        cv.mix(x, y, mul(c, g), a * (r - d + 0.5).clamp(0.0, 1.0));
    });
}

/// Oats flakes at (cx, cy) in pixels, `a` opaque. Four flakes, turned by
/// where they lie so no two handfuls look the same.
fn flakes(cv: &mut Cv, th: &Theme, cx: f32, cy: f32, sf: f32, c: Rgb, a: f32) {
    let turn = (hash(cx as u32, cy as u32) % 628) as f32 / 100.0;
    for k in 0..4 {
        let ang = turn + k as f32 * PI / 2.0;
        let (fx, fy) = (cx + ang.cos() * 1.3 * sf, cy + ang.sin() * 1.3 * sf);
        if let Some(ink) = th.ink {
            cv.disc(fx, fy, 1.35 * sf + 2.0, ink, a);
        }
        if th.glow > 0.0 {
            cv.around(fx, fy, 3.0 * sf, |cv, x, y, d| {
                cv.add(x, y, c, a * 0.12 * (1.0 - d / (3.0 * sf)));
            });
        }
        cv.disc(fx, fy, 1.35 * sf, c, a);
        cv.disc(fx - 0.35 * sf, fy - 0.35 * sf, 0.55 * sf, lerp(c, [255, 255, 255], 0.5), a * 0.7);
    }
}

fn piles(cv: &mut Cv, d: &Dish, th: &Theme, s: i32, st: &Status) {
    let sf = s as f32;
    let sps = STEPS_PER_SECOND as u64;
    let mut dish_index = 0;
    for p in &d.piles {
        let (cx, cy) = (p.x * sf, p.y * sf);
        let rp = p.r * sf;
        let frac = (p.food / p.full).clamp(0.0, 1.0);
        if p.oats {
            flakes(cv, th, cx, cy, sf, mul(th.oats, 0.6 + 0.4 * frac), 1.0);
            // Time left, as a gauge that empties clockwise.
            let left = p.expires.saturating_sub(d.t) as f32 / (OATS_SECONDS as u64 * sps) as f32;
            let r = (OATS_R + 2.2) * sf;
            let c = if p.reached() { th.hot } else { th.oats };
            match th.style {
                Style::Glow => {
                    cv.ring(cx, cy, r, 2.0, th.oats, 0.15, None);
                    cv.arc(cx, cy, r, 2.0, left, c, 0.8);
                }
                Style::Toy => {
                    cv.ring(cx, cy, r, 7.0, th.ink.unwrap_or([0, 0, 0]), 1.0, None);
                    cv.ring(cx, cy, r, 3.0, [240, 236, 226], 1.0, None);
                    cv.arc(cx, cy, r, 3.0, left, c, 1.0);
                }
                _ => {
                    cv.ring(cx, cy, r, 3.0, [52, 58, 60], 0.8, None);
                    cv.arc(cx, cy, r, 3.0, left, c, 1.0);
                }
            }
            continue;
        }
        let joined = st.joined.get(dish_index).copied().unwrap_or(false);
        dish_index += 1;
        match th.style {
            Style::Glow => {
                // Its glow is the halo's food field (`Halo`), drawn with
                // the dish.
                grain_disc(cv, cx, cy, rp, mul(th.food, 0.5 + 0.5 * frac), 1.0);
                if p.reached() {
                    let beat = 0.55 + 0.45 * ((d.t as f32 / sps as f32) * TAU).sin();
                    cv.ring(cx, cy, rp + 1.5 * sf, 3.0, th.hot, beat, None);
                }
                cv.arc(cx, cy, rp + 3.2 * sf, 2.0, frac, th.food, 0.6);
            }
            Style::Toy => {
                let ink = th.ink.unwrap_or([0, 0, 0]);
                cv.disc(cx + 3.0, cy + 4.0, rp + 3.0, ink, 0.3);
                cv.disc(cx, cy, rp + 3.0, ink, 1.0);
                grain_disc(cv, cx, cy, rp, mul(th.food, 0.55 + 0.45 * frac), 1.0);
                cv.disc(cx - rp * 0.35, cy - rp * 0.38, rp * 0.32, lerp(th.food, [255, 255, 255], 0.5), 0.55);
                let r = rp + 2.3 * sf;
                cv.ring(cx, cy, r, 9.0, ink, 1.0, None);
                cv.ring(cx, cy, r, 4.0, [236, 226, 206], 1.0, None);
                cv.arc(cx, cy, r, 4.0, frac, if p.reached() { th.hot } else { th.food }, 1.0);
            }
            _ => {
                grain_disc(cv, cx, cy, rp, mul(th.food, 0.55 + 0.45 * frac), 1.0);
                cv.ring(cx, cy, rp, 1.5, mul(th.food, 0.45), 1.0, None);
                // How much food is left, as a gauge: bright while eaten.
                let r = rp + 1.7 * sf;
                cv.ring(cx, cy, r, 3.0, [46, 54, 56], 0.85, None);
                cv.arc(cx, cy, r, 3.0, frac, if p.reached() { th.hot } else { th.food }, 1.0);
            }
        }
        // At the end, each pile says whether it ended joined.
        if d.phase == Phase::Over && d.wander.is_none() {
            badge(cv, th, cx + rp * 0.8, cy - rp * 0.8, joined);
        }
    }
    // Food on its way: a ring where it will land and the seconds to go.
    for (at, p) in &d.pending {
        if *at <= d.t + 5 * sps {
            let (cx, cy) = (p.x * sf, p.y * sf);
            let spin = (d.t as f32 / sps as f32) * 0.25;
            let c = th.ink.unwrap_or(th.food);
            cv.ring(cx, cy, p.r * sf, 3.0, c, 0.85, Some((10, spin)));
            let secs = (at - d.t).div_ceil(sps).to_string();
            let w = text_w(th, &secs);
            if th.font == Font::Chunky {
                hud::draw_text_scaled(cv.f, (cv.w as u32, cv.h as u32), 2, ((cx as i32 - w / 2 + 2) / 2, (cy as i32 - 7) / 2), &secs, [c[0], c[1], c[2], 255]);
            } else {
                text(cv, th, cx as i32 - w / 2, cy as i32 - 7, &secs, c);
            }
        }
    }
    for &(x, y, at) in &d.arrived {
        let age = d.t.saturating_sub(at);
        if age < 2 * sps {
            let f = age as f32 / (2 * sps) as f32;
            cv.ring(x * sf, y * sf, (6.0 + 14.0 * f) * sf, 4.0, [255, 250, 220], 1.0 - f, None);
        }
    }
}

fn badge(cv: &mut Cv, th: &Theme, x: f32, y: f32, ok: bool) {
    let (fill, mark) = if ok { (th.hot, [30, 30, 20]) } else { ([70, 70, 74], [230, 230, 230]) };
    if let Some(ink) = th.ink {
        cv.disc(x, y, 13.0, ink, 1.0);
    }
    cv.disc(x, y, 11.0, fill, 1.0);
    if ok {
        cv.seg((x - 5.0, y), (x - 1.0, y + 4.0), 3.0, mark, 1.0, 0.0);
        cv.seg((x - 1.0, y + 4.0), (x + 6.0, y - 5.0), 3.0, mark, 1.0, 0.0);
    } else {
        cv.seg((x - 4.5, y - 4.5), (x + 4.5, y + 4.5), 3.0, mark, 1.0, 0.0);
        cv.seg((x - 4.5, y + 4.5), (x + 4.5, y - 4.5), 3.0, mark, 1.0, 0.0);
    }
}

/// Snip strokes as a cut: the stroke's own width, fading over a second.
/// And joins (`Dish::snaps`): where two pieces met and became one, a burst
/// that spreads and fades over 0.8 s, as `render.rs` draws it.
fn marks(cv: &mut Cv, d: &Dish, th: &Theme, s: i32) {
    let sf = s as f32;
    let sps = STEPS_PER_SECOND as u64;
    for &(x, y, at) in &d.snaps {
        let age = d.t.saturating_sub(at) as f32 / sps as f32;
        if age >= 0.8 {
            continue;
        }
        let f = age / 0.8;
        let (cx, cy) = (x * sf, y * sf);
        let r = (2.0 + 10.0 * f) * sf;
        if th.glow > 0.0 {
            cv.around(cx, cy, r + 4.0 * sf, |cv, px, py, dd| {
                let g = (1.0 - dd / (r + 4.0 * sf)).max(0.0);
                cv.add(px, py, th.hot, 0.55 * (1.0 - f) * g * g);
            });
        }
        if let Some(ink) = th.ink {
            cv.ring(cx, cy, r, 6.0, ink, 1.0 - f, None);
        }
        cv.ring(cx, cy, r, 3.0, [255, 255, (230.0 * (1.0 - f) + 120.0 * f) as u8], 1.0 - 0.5 * f, None);
        if f < 0.5 {
            cv.ring(cx, cy, (1.0 + 4.0 * f) * sf, 2.0, [255, 255, 255], 1.0, None);
        }
    }
    for &(x, y, at) in &d.snip_marks {
        let age = d.t.saturating_sub(at);
        if age < sps {
            let f = 1.0 - age as f32 / sps as f32;
            cv.disc(x * sf, y * sf, SNIP_R * sf, th.bad, 0.75 * f);
            cv.disc(x * sf, y * sf, SNIP_R * sf * 0.4, [255, 240, 230], 0.8 * f);
        }
    }
}

// -------------------------------------------------------------- the storm

fn chip(cv: &mut Cv, th: &Theme, x: i32, y: i32, lines: &[&str], c: Rgb) -> (i32, i32) {
    let w = lines.iter().map(|l| text_w(th, l)).max().unwrap_or(0) + 16;
    let h = line_h(th) * lines.len() as i32 + 8;
    let x = x.clamp(4, cv.w - w - 4);
    match th.style {
        Style::Toy => {
            let ink = th.ink.unwrap_or([0, 0, 0]);
            cv.rbox(x + 3, y + 4, w, h, 10.0, Some((ink, 0.35)), None);
            cv.rbox(x, y, w, h, 10.0, Some((th.panel_edge, 1.0)), Some((ink, 3.0)));
        }
        Style::Glow => {
            // Opaque in the game's look: a see-through chip was blended
            // pixel by pixel, a tenth of a millisecond for the two corners.
            let a = if th.look == Style::Night { 1.0 } else { 0.82 };
            cv.rbox(x, y, w, h, 8.0, Some((th.panel, a)), Some((mul(c, 0.7), 1.5)));
        }
        _ => {
            cv.rbox(x, y, w, h, 5.0, Some((th.panel, 0.92)), Some((mul(c, 0.6), 1.0)));
        }
    }
    let tc = if th.style == Style::Toy { th.ink.unwrap_or([0, 0, 0]) } else { c };
    for (k, l) in lines.iter().enumerate() {
        let y = y + 5 + k as i32 * line_h(th);
        if th.font == Font::Chunky {
            // Ink on cream: no outline.
            hud::draw_text_scaled(cv.f, (cv.w as u32, cv.h as u32), 2, ((x + 8) / 2, (y + 2) / 2), l, [tc[0], tc[1], tc[2], 255]);
        } else {
            text(cv, th, x + 8, y, l, tc);
        }
    }
    (w, h)
}

fn storm(cv: &mut Cv, d: &Dish, th: &Theme, s: i32, st: &Status) {
    let sf = s as f32;
    let sps = STEPS_PER_SECOND as u64;
    let px = |p: (f32, f32)| (p.0 * sf, p.1 * sf);
    if d.phase == Phase::Growing {
        if let Some((a, b)) = d.storm_line {
            let (a, b) = (px(a), px(b));
            let pulse = if st.flash { 1.0 } else { 0.7 };
            match th.style {
                Style::Glow => {
                    // Crackling: a jittered line that redraws four times a second.
                    let n = 14;
                    let len = (b.0 - a.0).hypot(b.1 - a.1).max(1.0);
                    let (nx, ny) = (-(b.1 - a.1) / len, (b.0 - a.0) / len);
                    let tick = (d.t / (sps / 4)) as u32;
                    let mut prev = a;
                    for k in 1..=n {
                        let f = k as f32 / n as f32;
                        let j = if k == n {
                            0.0
                        } else {
                            ((hash(k, tick) % 100) as f32 / 100.0 - 0.5) * 2.4 * sf
                        };
                        let p = (a.0 + (b.0 - a.0) * f + nx * j, a.1 + (b.1 - a.1) * f + ny * j);
                        cv.seg(prev, p, 14.0, th.storm, 0.16 * pulse, 0.0);
                        cv.seg(prev, p, 4.0, lerp(th.storm, [255, 255, 255], 0.5), pulse, 0.0);
                        prev = p;
                    }
                }
                Style::Toy => {
                    // Hazard tape.
                    let ink = th.ink.unwrap_or([0, 0, 0]);
                    cv.band(a, b, 9.0, |cv, along, across, x, y| {
                        let c = if across.abs() > 6.5 {
                            ink
                        } else if ((along + across) / 9.0).rem_euclid(2.0) < 1.0 {
                            th.storm
                        } else {
                            [255, 250, 240]
                        };
                        cv.mix(x, y, c, if across.abs() > 6.5 { 1.0 } else { pulse });
                    });
                }
                _ => {
                    // A hatched band the width of a tube's core.
                    cv.band(a, b, 2.5 * sf * 0.5 + 2.0, |cv, _along, _across, x, y| {
                        let on = ((x + y) / 5).rem_euclid(2) == 0;
                        cv.mix(x, y, th.storm, if on { 0.9 * pulse } else { 0.25 });
                    });
                }
            }
            let low = if a.1 > b.1 { a } else { b };
            let words = if st.lesson || st.left > STORM_WARNING_SECONDS {
                "Storm will cut here".to_string()
            } else {
                format!("Storm cuts here in {}", st.left)
            };
            let w = text_w(th, &words) + 16;
            let y = (low.1 as i32 + 10).clamp(4, H as i32 * s - 36);
            chip(cv, th, low.0 as i32 - w / 2, y, &[&words], th.storm);
        }
    }
    if let Phase::Storm(at) = d.phase {
        if d.t < at + sps {
            let f = 1.0 - (d.t - at) as f32 / sps as f32;
            for &(a, b) in &d.cuts {
                cv.seg(px(a), px(b), 18.0, th.storm, 0.3 * f, 0.0);
                cv.seg(px(a), px(b), 6.0, [255, 240, 235], f, 0.0);
            }
        }
    }
    if d.phase != Phase::Growing {
        if let Some(&(a, b)) = d.cuts.first() {
            let (a, b) = (px(a), px(b));
            let low = if a.1 > b.1 { a } else { b };
            let w = text_w(th, "Storm scar") + 16;
            let y = (low.1 as i32 + 10).clamp(4, H as i32 * s - 36);
            chip(cv, th, low.0 as i32 - w / 2, y, &["Storm scar"], th.bad);
        }
    }
}

/// The last seconds before the storm: a big count at the top of the dish,
/// flashing, and the dish's rim flashing with it (`rim_colour`).
fn alarm(cv: &mut Cv, d: &Dish, view: &View, th: &Theme, s: i32, st: &Status) {
    if !st.alarm || view.brief {
        return;
    }
    let n = st.left.to_string();
    let y = if view.banner.is_some() { 84 } else { 14 };
    let cx = cv.w / 2;
    let k = 8;
    let w = big_w(th, k, &n);
    // A challenge dish has no storm (`level.rs`): its clock runs to a bell.
    let words = if d.level.is_some() { "Bell in" } else { "Storm in" };
    let tw = text_w(th, words);
    let bw = w.max(tw) + 40;
    let bh = 7 * k + line_h(th) + 24;
    let c = if st.flash { th.storm } else { [255, 255, 255] };
    match th.style {
        Style::Toy => {
            let ink = th.ink.unwrap_or([0, 0, 0]);
            let shake = if st.flash { 3 } else { -3 };
            cv.rbox(cx - bw / 2 + 4, y + 5, bw, bh, 18.0, Some((ink, 0.4)), None);
            let back = if st.flash { [255, 250, 236] } else { [255, 214, 200] };
            cv.rbox(cx - bw / 2 + shake, y, bw, bh, 18.0, Some((back, 1.0)), Some((ink, 4.0)));
            hud::draw_text_scaled(cv.f, (cv.w as u32, cv.h as u32), 2, ((cx - tw / 2 + shake + 2) / 2, (y + 10) / 2), words, [ink[0], ink[1], ink[2], 255]);
            chunky(cv, th, k, cx - w / 2 + shake, y + 10 + line_h(th), &n, th.storm);
        }
        Style::Glow => {
            cv.around(cx as f32, (y + bh / 2) as f32, bh as f32, |cv, x, yy, dd| {
                let f = 1.0 - dd / bh as f32;
                cv.add(x, yy, th.storm, 0.35 * f * f);
            });
            text(cv, th, cx - tw / 2, y + 8, words, [255, 255, 255]);
            mixed(cv, k, cx - w / 2, y + 12 + line_h(th), &n, if st.flash { [255, 255, 255] } else { th.storm });
        }
        _ => {
            cv.rbox(cx - bw / 2, y, bw, bh, 8.0, Some((mul(th.storm, 0.25), 0.94)), Some((th.storm, 3.0)));
            text(cv, th, cx - tw / 2, y + 8, words, [255, 255, 255]);
            big(cv, th, k, cx - w / 2, y + 12 + line_h(th), &n, c);
        }
    }
    let _ = (d, s);
}

// --------------------------------------------------------------- the lamp

/// Arrowheads pointing out of a ring: the push carries the mould outward.
#[allow(clippy::too_many_arguments)]
fn chevrons(cv: &mut Cv, cx: f32, cy: f32, r: f32, n: u32, c: Rgb, a: f32, t: f32) {
    for k in 0..n {
        let ang = k as f32 / n as f32 * TAU + 0.2;
        let (ux, uy) = (ang.cos(), ang.sin());
        let tip = (cx + ux * (r + 9.0), cy + uy * (r + 9.0));
        for side in [-1.0f32, 1.0] {
            let back = (tip.0 - ux * 6.0 - uy * 5.0 * side, tip.1 - uy * 6.0 + ux * 5.0 * side);
            cv.seg(back, tip, t, c, a, 0.0);
        }
    }
}

fn lamp(cv: &mut Cv, d: &Dish, th: &Theme, s: i32) {
    let Some((lx, ly)) = d.lamp else { return };
    let sf = s as f32;
    let (cx, cy) = (lx * sf, ly * sf);
    let r = d.params.lamp_reach * sf;
    match th.style {
        Style::Glow => {
            cv.around(cx, cy, r, |cv, x, y, dd| {
                let f = 1.0 - dd / r;
                cv.add(x, y, th.lamp, 0.10 + 0.32 * f * f);
            });
            cv.ring(cx, cy, r, 2.0, th.lamp, 0.9, None);
            cv.ring(cx, cy, r + 4.0, 6.0, th.lamp, 0.12, None);
            chevrons(cv, cx, cy, r, 8, th.lamp, 0.8, 2.0);
        }
        Style::Toy => {
            cv.around(cx, cy, r, |cv, x, y, dd| {
                let f = 1.0 - dd / r;
                cv.mix(x, y, th.lamp, 0.32 * f.sqrt());
            });
            cv.ring(cx, cy, r, 4.0, th.ink.unwrap_or([0, 0, 0]), 1.0, Some((16, 0.0)));
            chevrons(cv, cx, cy, r, 8, th.ink.unwrap_or([0, 0, 0]), 1.0, 3.0);
        }
        _ => {
            cv.around(cx, cy, r, |cv, x, y, dd| {
                let f = 1.0 - dd / r;
                cv.add(x, y, th.lamp, 0.24 * f.powf(0.7));
            });
            cv.ring(cx, cy, r, 3.0, th.lamp, 0.95, None);
            chevrons(cv, cx, cy, r, 8, th.lamp, 0.9, 2.5);
        }
    }
}

// ------------------------------------------------------------ the previews

/// Text beside the cursor, nudged to stay on the dish.
fn tip(cv: &mut Cv, th: &Theme, s: i32, (cx, cy): (f32, f32), r: f32, words: &str, c: Rgb) {
    let w = text_w(th, words) + 16;
    let x = (cx + r + 10.0) as i32;
    let x = if x + w > cv.w - 4 { (cx - r - 10.0) as i32 - w } else { x };
    let y = ((cy - 12.0) as i32).clamp(4, H as i32 * s - 30);
    chip(cv, th, x, y, &[words], c);
}

fn refuse(cv: &mut Cv, th: &Theme, s: i32, (cx, cy): (f32, f32), words: &str) {
    let c = th.bad;
    if let Some(ink) = th.ink {
        cv.seg((cx - 9.0, cy - 9.0), (cx + 9.0, cy + 9.0), 7.0, ink, 1.0, 0.0);
        cv.seg((cx - 9.0, cy + 9.0), (cx + 9.0, cy - 9.0), 7.0, ink, 1.0, 0.0);
    }
    cv.seg((cx - 8.0, cy - 8.0), (cx + 8.0, cy + 8.0), 3.5, c, 1.0, 0.0);
    cv.seg((cx - 8.0, cy + 8.0), (cx + 8.0, cy - 8.0), 3.5, c, 1.0, 0.0);
    tip(cv, th, s, (cx, cy), 10.0, words, c);
}

fn preview(cv: &mut Cv, d: &Dish, view: &View, th: &Theme, s: i32) {
    if view.brief || d.lamp.is_some() {
        return;
    }
    let Some((x, y)) = view.cursor else { return };
    let sf = s as f32;
    let (cx, cy) = (x * sf, y * sf);
    let line = if th.style == Style::Toy { th.ink.unwrap_or([0, 0, 0]) } else { [235, 240, 250] };
    if d.phase != Phase::Growing {
        if view.tool != Tool::Lamp {
            refuse(cv, th, s, (cx, cy), "Not now: the storm");
        }
        return;
    }
    let stroke = d.stroke_state();
    match view.tool {
        Tool::Lamp => {
            let r = d.params.lamp_reach * sf;
            cv.ring(cx, cy, r, 3.0, line, 0.6, Some((16, 0.0)));
            chevrons(cv, cx, cy, r, 8, line, 0.5, 2.0);
            cv.disc(cx, cy, 3.0, line, 0.8);
        }
        Tool::Oats => {
            let open = cell_at(x, y).is_some_and(|i| d.ground[i] == Ground::Agar);
            if d.oats_left == 0 {
                let w = match oats_wait(d) {
                    Some(w) => format!("No oats: next in {w} s"),
                    None => "No oats left".to_string(),
                };
                refuse(cv, th, s, (cx, cy), &w);
                return;
            }
            if !open {
                refuse(cv, th, s, (cx, cy), "Oats can't go here");
                return;
            }
            // The nearest mould, and whether it is in the measured pull.
            let near = d
                .walkers
                .iter()
                .map(|w| (len2(w.x - x, w.y - y), (w.x, w.y)))
                .min_by(|a, b| a.0.total_cmp(&b.0));
            let pull = OATS_PULL * sf;
            let reach = near.is_some_and(|(dist, _)| dist <= OATS_PULL);
            // Toy draws its marks in ink: yellow on its pale agar vanished.
            let good = if th.style == Style::Toy { line } else { th.hot };
            cv.ring(cx, cy, pull, 2.5, if reach { good } else { line }, 0.55, Some((24, 0.0)));
            if let Some((dist, (wx, wy))) = near {
                let c = if reach { good } else { mul(line, 0.7) };
                cv.seg((wx * sf, wy * sf), (cx, cy), 3.0, c, if reach { 0.95 } else { 0.5 }, 6.0);
                let words = if reach {
                    "It will reach these".to_string()
                } else if dist <= OATS_MAYBE {
                    "It may reach these, slowly".to_string()
                } else {
                    format!("Too far to pull: {} cells", dist.round() as i32)
                };
                flakes(cv, th, cx, cy, sf, th.oats, 0.55);
                tip(cv, th, s, (cx, cy), pull.min(48.0), &words, if reach { th.hot } else { th.text });
            } else {
                flakes(cv, th, cx, cy, sf, th.oats, 0.55);
            }
        }
        Tool::Salt | Tool::Snip => {
            let (len, have, r, noun) = if view.tool == Tool::Salt {
                (SALT_LEN, d.salt_left, SALT_R, "Wall")
            } else {
                (SNIP_LEN, d.snips_left, SNIP_R, "Cut")
            };
            let c = if view.tool == Tool::Salt { [240, 246, 255] } else { th.bad };
            match stroke {
                Some((t, last, left)) if t == view.tool => {
                    // Mid-stroke: how far the stroke can still go.
                    let (lx, ly) = (last.0 * sf, last.1 * sf);
                    cv.ring(lx, ly, left * sf, 3.0, c, 0.75, Some((20, 0.0)));
                    let words = format!("{noun} left: {} of {}", left.round() as i32, len as i32);
                    tip(cv, th, s, (cx, cy), 12.0, &words, c);
                    // A bar of what is left.
                    let (bx, by) = (cx as i32 - 30, cy as i32 + 16);
                    cv.rect(bx, by, 60, 6, [30, 30, 30], 0.8);
                    cv.rect(bx, by, (60.0 * left / len) as i32, 6, c, 1.0);
                }
                _ if have == 0 => {
                    let w = if view.tool == Tool::Salt { "Salt used up" } else { "No snips left" };
                    refuse(cv, th, s, (cx, cy), w);
                }
                _ => {
                    cv.ring(cx, cy, len * sf, 2.0, c, 0.3, Some((30, 0.0)));
                    if view.tool == Tool::Snip {
                        // Exactly the cells a press here would clear.
                        let thr = d.params.tube * 0.5;
                        let mut hit = 0;
                        for (ox, oy) in disc(r) {
                            if let Some(i) = cell_at(x + ox, y + oy) {
                                if d.ground[i] == Ground::Agar && d.trail[i] > thr {
                                    hit += 1;
                                    let (gx, gy) = ((i % W) as i32 * s, (i / W) as i32 * s);
                                    cv.rect(gx, gy, s, s, c, 0.8);
                                }
                            }
                        }
                        cv.ring(cx, cy, r * sf, 2.0, c, 0.9, None);
                        let words = if hit > 0 { "Drag to cut this tube" } else { "No tube here" };
                        tip(cv, th, s, (cx, cy), r * sf, words, if hit > 0 { c } else { th.text });
                    } else {
                        cv.disc(cx, cy, r * sf, c, 0.6);
                        let words = format!("Drag: a wall {} long", len as i32);
                        tip(cv, th, s, (cx, cy), r * sf, &words, th.text);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------- corners

/// The two corner chips on the rim, outside the dish, just above the tool
/// bar: what a press does now (bottom left) and the keys (bottom right).
/// The top of the dish is the lesson banner's and the storm count's. Glow
/// keeps its hint in the bar.
fn corners(cv: &mut Cv, d: &Dish, view: &View, th: &Theme, st: &Status, s: i32) {
    if view.brief {
        return;
    }
    // A challenge dish: what each star needs, top left, while it runs.
    if let (Some(l), Phase::Growing) = (d.level, d.phase) {
        chip(cv, th, 6, 6, &[&up(&l.stars_line())], th.dim);
    }
    let bottom = H as i32 * s - 2 * line_h(th) - 14;
    let h = hint(d, view, st);
    let hc = if d.phase == Phase::Growing && !st.alarm { th.text } else { th.hot };
    let hc = if d.can_go() { exit_teal(th) } else { hc };
    if th.hud != Style::Glow {
        chip(cv, th, 6, bottom, &[&h[0], &h[1]], hc);
    }
    if view.keys {
        let k = keys_lines(view);
        let w = k.iter().map(|l| text_w(th, l)).max().unwrap_or(0) + 16;
        if th.hud == Style::Glow {
            for (n, l) in k.iter().enumerate() {
                text(cv, th, cv.w - w, bottom + 6 + n as i32 * line_h(th), l, th.dim);
            }
        } else {
            chip(cv, th, cv.w - w - 6, bottom, &[k[0], k[1]], th.dim);
        }
    }
}

// ------------------------------------------------------------------- HUD

fn hud_strip(cv: &mut Cv, d: &Dish, view: &View, th: &Theme, s: i32, st: &Status) {
    let y0 = H as i32 * s;
    if d.wander.is_some() {
        // The Wanderer's bar is the tile bar in every look: the candidates
        // were drawn before she existed.
        return hud_wander(cv, d, view, th, y0);
    }
    match th.hud {
        Style::Glow => hud_glow(cv, d, view, th, y0, st),
        Style::Toy => hud_toy(cv, d, view, th, y0, st),
        _ => {
            // The tile bar is redrawn only when something on it changes; the
            // mould count, which changes nearly every frame, goes on after.
            let key = bench_key(d, view, th, cv.w, st);
            hud_cached(cv, th, y0, key, |cv| hud_bench(cv, d, view, th, y0, st));
            if d.score.is_none() {
                let m = format!("Mould {}", d.walkers.len());
                let mw = text_w(th, &m);
                text(cv, th, cv.w - mw - 10, y0 + 10, &m, th.dim);
            }
        }
    }
}

/// Everything `hud_bench` draws, but the mould count: the tile bar's cache
/// key. Pixel positions where something moves, so a bar that moved less than
/// a pixel is the same bar.
fn bench_key(d: &Dish, view: &View, th: &Theme, w: i32, st: &Status) -> String {
    let stocks: Vec<_> = Tool::ALL
        .iter()
        .map(|t| (d.has_tool(*t), stock(d, *t).map(|(h, o, g)| (h, o, (10.0 * g) as i32))))
        .collect();
    let bw = w - 616 - 10;
    let line = timeline(d).map(|(f, ticks)| {
        let px: Vec<i32> = ticks.iter().map(|t| (bw as f32 * t) as i32).collect();
        ((bw as f32 * f) as i32, px)
    });
    format!(
        "{:?} {:?} {:?} {w} {:?} {:?} {:?} {} {} {} {:?} {:?} {} {} {:?} {:?}",
        d.level.map(|l| l.key),
        level_words(d),
        th.look,
        view.tool,
        stocks,
        oats_wait(d),
        phase_words(d, st),
        st.alarm,
        matches!(d.phase, Phase::Storm(_)),
        line,
        st.joined,
        st.reach,
        st.coming,
        score_words(d),
        stars_of(d).0,
    )
}

/// The HUD strip below the dish (rows `y0` on), drawn by `draw` only when
/// `key` differs from the last frame's for this look, and otherwise copied.
fn hud_cached(cv: &mut Cv, th: &Theme, y0: i32, key: String, draw: impl FnOnce(&mut Cv)) {
    use std::sync::Mutex;
    type Slot = Option<(String, Vec<u8>)>;
    static CACHE: Mutex<[Slot; 5]> = Mutex::new([const { None }; 5]);
    let k = Style::ALL.iter().position(|s| *s == th.look).unwrap_or(0);
    let from = (y0 * cv.w * 4) as usize;
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((was, px)) = &cache[k] {
        if *was == key && px.len() == cv.f.len() - from {
            cv.f[from..].copy_from_slice(px);
            return;
        }
    }
    draw(cv);
    cache[k] = Some((key, cv.f[from..].to_vec()));
}

fn score_words(d: &Dish) -> Option<[String; 3]> {
    // A challenge dish: its amount, its name and stars, and why.
    if let (Some(l), Some(c)) = (d.level, &d.card) {
        let n = c.stars;
        return Some([
            up(&c.amount),
            format!("{}: {n} star{}", title(l.name), if n == 1 { "" } else { "s" }),
            up(&c.why),
        ]);
    }
    let s = d.score.clone()?;
    // The why line (`level::free_card`): what to do differently. It names
    // the network's ratio itself when shape is what cost the star.
    let third = match &d.card {
        Some(c) => up(&c.why),
        None if s.reach >= THRIFT_MIN_JOINED => {
            format!("Network {:.2}x the shortest (star at {:.2}x)", s.ratio(), THRIFT_BAR)
        }
        None => "Too little joined to measure the network".to_string(),
    };
    // The amount first, and the amount that counts is what is still joined
    // once the storm has had its say (owner: "arrived with N of M" over
    // stars); the stars sit beside it.
    Some([
        format!("{} of {} food still joined", s.after, s.piles),
        format!("Joined {} of {} before the storm struck", s.reach, s.piles),
        third,
    ])
}

/// The three stars a finished dish earned, and their names: a challenge
/// dish's stars are tiers of one rule, so they go unnamed.
fn stars_of(d: &Dish) -> ([bool; 3], [&'static str; 3]) {
    if let (Some(_), Some(c)) = (d.level, &d.card) {
        return ([c.stars >= 1, c.stars >= 2, c.stars >= 3], ["", "", ""]);
    }
    (d.score.clone().unwrap_or_default().stars, ["Join", "Short", "Storm"])
}

/// A challenge dish's one number, where its rule reads something other than
/// the food joined (`render.rs`'s `level_status`, in words).
fn level_words(d: &Dish) -> Option<String> {
    let l = d.level?;
    match l.win {
        Win::JoinAll { .. } => None,
        Win::UnderRuler { .. } => d.join_now.map(|r| format!("All joined: {r:.2}x the ruler")),
        Win::AliveAtBell { .. } => {
            let fed = d.dish_piles().filter(|p| p.reached() && p.food > 0.0).count();
            Some(format!("Eating {fed} pile{}", if fed == 1 { "" } else { "s" }))
        }
    }
}

fn phase_words(d: &Dish, st: &Status) -> String {
    let sps = STEPS_PER_SECOND as u64;
    match d.phase {
        Phase::Over => "Dish over".into(),
        Phase::Crossing(_) => "On her way".into(),
        Phase::Storm(at) => {
            let end = at + super::Dish::storm_steps();
            format!("Storm! Score in {} s", end.saturating_sub(d.t).div_ceil(sps))
        }
        Phase::Growing if st.lesson => "Lesson: no storm".into(),
        Phase::Growing if d.level.is_some() => format!("Bell in {}", clock(st.left)),
        Phase::Growing => format!("Storm in {}", clock(st.left)),
    }
}

/// Fraction of the run to the storm gone, and the late food's marks on it.
fn timeline(d: &Dish) -> Option<(f32, Vec<f32>)> {
    if d.storm_at == u64::MAX {
        return None;
    }
    let start = timeline_start(d);
    let span = (d.storm_at - start).max(1) as f32;
    let f = (d.t.saturating_sub(start) as f32 / span).clamp(0.0, 1.0);
    let ticks = d
        .pending
        .iter()
        .map(|(at, _)| (at.saturating_sub(start) as f32 / span).clamp(0.0, 1.0))
        .collect();
    Some((f, ticks))
}

/// Where the timeline starts: the dish's start on a challenge dish (its
/// bell is its whole length), otherwise the scored dish's length before the
/// storm.
fn timeline_start(d: &Dish) -> u64 {
    if d.level.is_some() {
        0
    } else {
        d.storm_at.saturating_sub(d.storm_warning.max(super::Dish::dish_steps()))
    }
}

fn warn_frac(d: &Dish) -> f32 {
    let start = timeline_start(d);
    let span = (d.storm_at - start).max(1) as f32;
    (STORM_WARNING_SECONDS * STEPS_PER_SECOND) as f32 / span
}

/// Food pips: one per dish pile, filled when joined, then the ones coming.
fn food_pips(cv: &mut Cv, th: &Theme, x: i32, y: i32, st: &Status) -> i32 {
    let mut cx = x + 8;
    for &j in &st.joined {
        if let Some(ink) = th.ink {
            cv.disc(cx as f32, y as f32, 9.0, ink, 1.0);
        }
        if j {
            cv.disc(cx as f32, y as f32, 7.0, th.food, 1.0);
            cv.ring(cx as f32, y as f32, 7.0, 2.0, th.hot, 1.0, None);
        } else {
            cv.disc(cx as f32, y as f32, 7.0, mul(th.food, 0.3), 1.0);
            cv.ring(cx as f32, y as f32, 6.5, 1.5, mul(th.food, 0.8), 1.0, None);
        }
        cx += 20;
    }
    for _ in 0..st.coming {
        cv.ring(cx as f32, y as f32, 6.5, 1.5, th.dim, 0.9, Some((6, 0.0)));
        cx += 20;
    }
    cx - 10 - x
}

/// The tile bar's own greys: C's, or drawn from the look's panel for
/// `Night`. (selected tile, key badge, timeline track)
fn tones(th: &Theme) -> (Rgb, Rgb, Rgb) {
    if th.look == Style::Night {
        (lerp(th.panel, th.hot, 0.12), lerp(th.panel, th.panel_edge, 0.32), lerp(th.panel, th.panel_edge, 0.24))
    } else {
        ([38, 42, 30], [44, 50, 56], [30, 36, 40])
    }
}

fn hud_back(cv: &mut Cv, th: &Theme, y0: i32) {
    cv.rect(0, y0, cv.w, cv.h - y0, th.hud_bg, 1.0);
    if th.style == Style::Glow {
        for k in 0..4 {
            cv.rect(0, y0 + k, cv.w, 1, th.panel_edge, 0.5 / (k + 1) as f32);
        }
    } else {
        cv.rect(0, y0, cv.w, 1, th.panel_edge, 1.0);
    }
}

/// C's tool tiles: key, name, icon, stock pips with the next one growing,
/// and what is left in words.
fn tiles(cv: &mut Cv, d: &Dish, view: &View, th: &Theme, y0: i32) {
    let (sel_fill, badge_fill, _) = tones(th);
    for (k, t) in Tool::ALL.iter().enumerate() {
        let x = 8 + k as i32 * 150;
        let y = y0 + 8;
        if !d.has_tool(*t) {
            // A challenge dish without this tool: its slot stays, empty, so
            // the keys keep their places.
            cv.rbox(x, y, 142, 70, th.radius, None, Some((mul(th.panel_edge, 0.45), 1.0)));
            text(cv, th, x + 10, y + 8, &(k + 1).to_string(), mul(th.dim, 0.7));
            text(cv, th, x + 32, y + 8, "Not here", mul(th.dim, 0.7));
            continue;
        }
        let sel = *t == view.tool;
        let st_ = stock(d, *t);
        let spent = st_.is_some_and(|(h, _, _)| h == 0);
        let fill = if sel { sel_fill } else { th.panel };
        let edge = if sel { (th.hot, 2.0) } else { (th.panel_edge, 1.0) };
        cv.rbox(x, y, 142, 70, th.radius, Some((fill, 1.0)), Some(edge));
        cv.rbox(x + 6, y + 6, 18, 18, 4.0, Some((if sel { th.hot } else { badge_fill }, 1.0)), None);
        text(cv, th, x + 10, y + 8, &(k + 1).to_string(), if sel { [20, 20, 20] } else { th.text });
        let nc = if sel {
            th.hot
        } else if spent {
            th.dim
        } else {
            th.text
        };
        text(cv, th, x + 32, y + 8, t.name(), nc);
        icon(cv, th, *t, x + 8, y + 34, 2, spent);
        let tc = tool_colour(th, *t);
        match st_ {
            None => {
                text(cv, th, x + 42, y + 36, "Hold", th.dim);
            }
            Some((have, of, grown)) => {
                for p in 0..of {
                    let px = x + 42 + p as i32 * 14;
                    let py = y + 32;
                    if p < have {
                        cv.rbox(px, py, 10, 10, 2.0, Some((tc, 1.0)), None);
                    } else {
                        cv.rbox(px, py, 10, 10, 2.0, None, Some((th.dim, 1.0)));
                        if p == have && grown > 0.0 {
                            let h = (10.0 * grown) as i32;
                            cv.rect(px + 1, py + 10 - h, 8, h, mul(tc, 0.6), 1.0);
                        }
                    }
                }
                let sub = match (t, oats_wait(d)) {
                    (Tool::Oats, Some(w)) => format!("+1 in {w}s"),
                    (Tool::Oats, None) if have == of => "Full".into(),
                    (Tool::Salt, _) if have == 1 => "1 wall".into(),
                    (Tool::Salt, _) if have > 1 => format!("{have} walls"),
                    (Tool::Salt, _) => "Used".into(),
                    _ => format!("{have} left"),
                };
                // "+1 in 13s" is wider than the tile at x + 42 in the mixed font.
                let sx = if th.font == Font::Mixed { x + 34 } else { x + 42 };
                text(cv, th, sx, y + 50, &sub, th.dim);
            }
        }
    }
}

fn hud_bench(cv: &mut Cv, d: &Dish, view: &View, th: &Theme, y0: i32, st: &Status) {
    hud_back(cv, th, y0);
    let right = 616;
    if let Some(words) = score_words(d) {
        // "Not reached: only starving strays got there" is wider at three
        // times the dot than the room left of the stars: a size down.
        if big_w(th, 3, &words[0]) <= right - 24 {
            big(cv, th, 3, 12, y0 + 10, &words[0], th.hot);
        } else {
            text(cv, th, 12, y0 + 14, &words[0], th.hot);
        }
        text(cv, th, 12, y0 + 40, &words[1], th.text);
        // The why line (`level.rs`) is advice: as bright as the rest.
        let wc = if d.card.is_some() { th.text } else { th.dim };
        text(cv, th, 12, y0 + 60, &words[2], wc);
        let (stars, names) = stars_of(d);
        for (k, name) in names.iter().enumerate() {
            let x = right + 20 + k as i32 * 96;
            star(cv, th, x + 10, y0 + 10, 4, stars[k]);
            let w = text_w(th, name);
            text(cv, th, x + 28 - w / 2, y0 + 58, name, if stars[k] { th.text } else { th.dim });
        }
        return;
    }
    tiles(cv, d, view, th, y0);
    // Status: the storm, a timeline to it, the food joined, the mould.
    let ph = phase_words(d, st);
    let pc = if st.alarm || matches!(d.phase, Phase::Storm(_)) { th.storm } else { th.text };
    text(cv, th, right, y0 + 10, &ph, pc);
    let bw = cv.w - right - 10;
    let by = y0 + 32;
    if let Some((f, ticks)) = timeline(d) {
        cv.rbox(right, by, bw, 10, 3.0, Some((tones(th).2, 1.0)), None);
        let wf = warn_frac(d);
        let wx = right + (bw as f32 * (1.0 - wf)) as i32;
        cv.band((wx as f32, (by + 5) as f32), ((right + bw) as f32, (by + 5) as f32), 5.0, |cv, _a, _c, x, y| {
            let on = ((x + y) / 4).rem_euclid(2) == 0;
            cv.mix(x, y, th.storm, if on { 0.8 } else { 0.3 });
        });
        cv.rbox(right, by, (bw as f32 * f) as i32, 10, 3.0, Some((th.dim, 0.9)), None);
        for t in ticks {
            let tx = right as f32 + bw as f32 * t;
            cv.disc(tx, (by + 5) as f32, 5.0, th.food, 1.0);
        }
        let mx = right + (bw as f32 * f) as i32;
        cv.rect(mx - 1, by - 3, 3, 16, [255, 255, 255], 1.0);
    } else {
        text(cv, th, right, by - 2, "The storm comes in the last step", th.dim);
    }
    let fy = y0 + 64;
    if let Some(words) = level_words(d) {
        text(cv, th, right, fy - 7, &words, th.text);
        return;
    }
    let w = text(cv, th, right, fy - 7, "Food", th.text);
    let pw = food_pips(cv, th, right + w + 8, fy, st);
    let total = st.joined.len() as u32 + st.coming;
    text(cv, th, right + w + 18 + pw, fy - 7, &format!("{} of {} joined", st.reach, total), th.dim);
}

/// The Wanderer's bar: the tiles, then her dish, her size, the food left
/// under her (the clock she has instead of a storm), and what Go would do.
fn hud_wander(cv: &mut Cv, d: &Dish, view: &View, th: &Theme, y0: i32) {
    let Some(w) = &d.wander else { return };
    hud_back(cv, th, y0);
    tiles(cv, d, view, th, y0);
    let right = 616;
    let bw = cv.w - right - 10;
    text(cv, th, right, y0 + 10, &format!("Dish {}", w.stretch), th.text);
    let m = format!(
        "Mould {}  best {}",
        thousands(d.walkers.len() as u32 + w.pouring.len() as u32),
        thousands(w.peak)
    );
    let mw = text_w(th, &m);
    text(cv, th, cv.w - mw - 10, y0 + 10, &m, th.dim);
    // Food left on the dish's piles, draining: her clock.
    let food = d.food_left().clamp(0.0, 1.0);
    let fl = text(cv, th, right, y0 + 30, "Food", th.text);
    let pct = format!("{}%", (food * 100.0).round() as u32);
    let pw = text_w(th, &pct);
    let (bx, by, bl) = (right + fl + 10, y0 + 34, bw - fl - pw - 22);
    cv.rbox(bx, by, bl, 10, 3.0, Some((tones(th).2, 1.0)), None);
    let low = food < 0.2;
    let fc = if low { th.bad } else { th.food };
    cv.rbox(bx, by, (bl as f32 * food) as i32, 10, 3.0, Some((fc, 1.0)), None);
    text(cv, th, cv.w - pw - 10, y0 + 30, &pct, if low { th.bad } else { th.dim });
    let teal = exit_teal(th);
    let (line, c) = match (d.phase, w.outcome) {
        (Phase::Crossing(_), _) => (
            format!(
                "Through {}  in line {}  died {}",
                thousands(w.through),
                thousands(w.pouring.len() as u32),
                thousands(w.lost_on_way)
            ),
            teal,
        ),
        (Phase::Over, Some(Outcome::Arrived { n, .. })) => (format!("N: next dish, with {}", thousands(n)), th.hot),
        (Phase::Over, _) => ("R: try again  N: new journey".to_string(), th.hot),
        _ if d.can_go() => (
            format!("G: go. {} of {} would come", thousands(w.joined), thousands(d.walkers.len() as u32)),
            teal,
        ),
        _ => ("Lead her to the crack, then G".to_string(), th.dim),
    };
    if d.can_go() && th.glow > 0.0 {
        cv.rbox(right - 6, y0 + 52, bw + 12, 26, 8.0, Some((teal, 0.12)), Some((teal, 1.0)));
    }
    text(cv, th, right, y0 + 56, &line, c);
}

/// Beat the Ruler (`level.rs`): the ruler, dotted between the piles' rims;
/// once the dish is over, the best answer too, faint, three spokes from the
/// meeting point, with how short it is.
fn ruler(cv: &mut Cv, d: &Dish, th: &Theme, s: i32) {
    let Some(l) = d.level else { return };
    if !matches!(l.win, Win::UnderRuler { .. }) {
        return;
    }
    let sf = s as f32;
    let dots = |cv: &mut Cv, a: (f32, f32), b: (f32, f32), every: f32, c: Rgb, r: f32| {
        let len = (b.0 - a.0).hypot(b.1 - a.1).max(0.01);
        let n = (len / every).floor() as usize;
        for k in 0..=n {
            let f = k as f32 * every / len;
            let (x, y) = ((a.0 + (b.0 - a.0) * f) * sf, (a.1 + (b.1 - a.1) * f) * sf);
            if th.glow > 0.0 {
                cv.disc(x, y, r * 2.2, c, 0.18);
            }
            if let Some(ink) = th.ink {
                cv.disc(x, y, r + 1.5, ink, 1.0);
            }
            cv.disc(x, y, r, c, 1.0);
        }
    };
    let rule = lerp(th.food, [255, 255, 255], 0.35);
    let mut label = None;
    for (a, b) in l.ruler().1 {
        let (p, q) = (&l.piles[a], &l.piles[b]);
        let len = (q.x - p.x).hypot(q.y - p.y).max(0.01);
        let (ux, uy) = ((q.x - p.x) / len, (q.y - p.y) / len);
        let pa = (p.x + ux * (p.r + 3.0), p.y + uy * (p.r + 3.0));
        let pb = (q.x - ux * (q.r + 3.0), q.y - uy * (q.r + 3.0));
        dots(cv, pa, pb, 4.0, rule, 2.0);
        label.get_or_insert(((pa.0 + pb.0) / 2.0, (pa.1 + pb.1) / 2.0));
    }
    if let Some((x, y)) = label {
        let w = text_w(th, "Ruler") + 16;
        chip(cv, th, (x * sf) as i32 - w / 2, (y * sf) as i32 + 10, &["Ruler"], rule);
    }
    if d.phase == Phase::Over {
        let (m, best) = l.meeting_point();
        let c = lerp(th.lamp, [120, 200, 230], 0.5);
        for p in l.piles {
            dots(cv, m, (p.x, p.y), 2.5, c, 1.5);
        }
        cv.ring(m.0 * sf, m.1 * sf, 3.0 * sf, 2.0, c, 1.0, None);
        let words = format!("Best: {best:.2}x the ruler");
        let w = text_w(th, &words) + 16;
        chip(cv, th, (m.0 * sf) as i32 - w / 2, (m.1 * sf) as i32 + 16, &[&words], c);
    }
}

/// The dish picker (M): the lesson, the challenge dishes, free play and the
/// Wanderer, each a card at `render::menu_cards`' places (the window's click
/// test reads the same rectangles), with best stars and what opens a locked
/// one.
fn menu(cv: &mut Cv, th: &Theme, m: &Menu) {
    for px in cv.f.chunks_exact_mut(4) {
        for v in px.iter_mut().take(3) {
            *v /= 3;
        }
    }
    let cards = menu_cards((cv.w as u32, cv.h as u32));
    let (x0, y0, _, _) = cards[0];
    let tw = text(cv, th, x0, y0 - 24, "Dishes", th.hot);
    text(cv, th, x0 + tw + 24, y0 - 24, "1-6 or click to pick   M: back", th.dim);
    for (e, &(x, y, w, h)) in cards.iter().enumerate() {
        let open = m.progress.unlocked(e);
        let sel = e == m.cursor;
        let edge = if sel {
            (th.hot, 2.0)
        } else if open {
            (th.panel_edge, 1.0)
        } else {
            (mul(th.panel_edge, 0.45), 1.0)
        };
        cv.rbox(x, y, w, h, 8.0, Some((if sel { lerp(th.panel, th.hot, 0.08) } else { th.panel }, 1.0)), Some(edge));
        let (name, lines) = entry_lines(e);
        let nc = if open { th.hot } else { th.dim };
        text(cv, th, x + 12, y + 5, &format!("{} {}", e + 1, title(name)), nc);
        if (1..FREE).contains(&e) {
            let b = m.progress.best[e];
            for k in 0..3 {
                star(cv, th, x + w - 92 + k as i32 * 28, y + 5, 2, k < b as usize);
            }
        } else if e == 0 && m.progress.best[0] >= 1 {
            let dw = text_w(th, "Done");
            text(cv, th, x + w - 12 - dw, y + 5, "Done", th.hot);
        }
        let c = if open { th.text } else { th.dim };
        let lines = sentence(&lines);
        text(cv, th, x + 12, y + 22, &lines[0], c);
        if open {
            text(cv, th, x + 12, y + 41, &lines[1], c);
        } else {
            let need = if e == 1 {
                "Locked: finish the lesson".to_string()
            } else {
                format!("Locked: 1 star on {}", title(entry_lines(e - 1).0))
            };
            text(cv, th, x + 12, y + 41, &need, th.bad);
        }
    }
}

/// "The way on", above the crack: dim until Go is open.
fn exit_label(cv: &mut Cv, d: &Dish, th: &Theme, s: i32) {
    if d.wander.is_none() || d.phase == Phase::Over {
        return;
    }
    let word = "The way on";
    let w = text_w(th, word) + 16;
    let y = ((wander::EXIT_Y - wander::EXIT_HALF) as i32 * s - line_h(th) - 18).max(4);
    let c = if d.can_go() { exit_teal(th) } else { mul(exit_teal(th), 0.6) };
    chip(cv, th, cv.w - w - 4, y, &[word], c);
}

/// The end of a stretch, across the top of the dish so what stayed behind
/// shows under it. The amount leads: how much of her came.
fn arrival(cv: &mut Cv, d: &Dish, th: &Theme) {
    let Some(w) = &d.wander else { return };
    let sps = STEPS_PER_SECOND as u64;
    let name = title(wander::NAME);
    let (head, hc, rest): (String, Rgb, Vec<String>) = match w.outcome {
        Some(Outcome::Arrived { n, of }) => {
            let pct = (n as f32 / of.max(1) as f32 * 100.0).round() as u32;
            let at = w.go_at.unwrap_or(0) / sps;
            (
                format!("{name} arrived with {} of {}", thousands(n), thousands(of)),
                th.hot,
                vec![
                    format!("{pct}%. Left at {}, {} died in line", clock(at as u32), thousands(w.lost_on_way)),
                    format!("N: dish {} with {}   R: again", w.stretch + 1, thousands(n)),
                ],
            )
        }
        Some(Outcome::Lost { at }) => (
            format!("{name} didn't make it"),
            th.bad,
            vec![
                format!("She starved at {}", clock((at / sps) as u32)),
                "R: try again   N: new journey".into(),
            ],
        ),
        None => return,
    };
    let k = 3;
    let lh = line_h(th);
    let tw = rest.iter().map(|l| text_w(th, l)).max().unwrap_or(0).max(big_w(th, k, &head));
    let (w_, h) = (tw + 40, 7 * k + 2 * lh + 34);
    let x0 = (cv.w - w_) / 2;
    card(cv, th, x0, 8, w_, h, Some(if matches!(w.outcome, Some(Outcome::Arrived { .. })) { exit_teal(th) } else { th.bad }));
    big(cv, th, k, x0 + 20, 20, &head, hc);
    text(cv, th, x0 + 20, 20 + 7 * k + 10, &rest[0], th.text);
    text(cv, th, x0 + 20, 20 + 7 * k + 10 + lh, &rest[1], th.dim);
}

fn hud_glow(cv: &mut Cv, d: &Dish, view: &View, th: &Theme, y0: i32, st: &Status) {
    cv.rect(0, y0, cv.w, cv.h - y0, th.hud_bg, 1.0);
    for k in 0..4 {
        cv.rect(0, y0 + k, cv.w, 1, th.panel_edge, 0.5 / (k + 1) as f32);
    }
    let (dcx, dcy, dr) = ((cv.w - 52) as f32, (y0 + 44) as f32, 32.0);
    if let Some(words) = score_words(d) {
        mixed(cv, 3, 24, y0 + 8, &words[0], th.hot);
        text(cv, th, 24, y0 + 38, &words[1], th.text);
        text(cv, th, 24, y0 + 60, &words[2], th.dim);
        let (stars, _) = stars_of(d);
        for (k, &on) in stars.iter().enumerate() {
            let x = cv.w - 290 + k as i32 * 64;
            if on {
                cv.around((x + 18) as f32, (y0 + 34) as f32, 30.0, |cv, xx, yy, dd| {
                    cv.add(xx, yy, th.hot, 0.25 * (1.0 - dd / 30.0));
                });
            }
            star(cv, th, x, y0 + 16, 4, on);
        }
        return;
    }
    for (k, t) in Tool::ALL.iter().enumerate() {
        let cx = 44.0 + k as f32 * 80.0;
        let cy = (y0 + 48) as f32;
        let r = 24.0;
        let sel = *t == view.tool;
        let st_ = stock(d, *t);
        let spent = st_.is_some_and(|(h, _, _)| h == 0);
        if sel {
            cv.around(cx, cy, r + 14.0, |cv, x, y, dd| {
                let f = (1.0 - (dd - r).max(0.0) / 14.0).clamp(0.0, 1.0);
                cv.add(x, y, th.hot, 0.22 * f * f);
            });
        }
        cv.disc(cx, cy, r, [12, 20, 28], 1.0);
        cv.ring(cx, cy, r, if sel { 3.0 } else { 1.5 }, if sel { th.hot } else { mul(th.panel_edge, 0.7) }, 1.0, None);
        icon(cv, th, *t, cx as i32 - 12, cy as i32 - 12, 2, spent);
        let key = (k + 1).to_string();
        text(cv, th, cx as i32 - 5, y0 + 4, &key, if sel { th.hot } else { th.dim });
        if let Some((have, of, grown)) = st_ {
            let tc = tool_colour(th, *t);
            for p in 0..of {
                let ang = PI / 2.0 + (p as f32 - (of - 1) as f32 / 2.0) * 0.42;
                let (px, py) = (cx + ang.cos() * (r + 7.0), cy + ang.sin() * (r + 7.0));
                let a = if p < have {
                    1.0
                } else if p == have {
                    0.15 + 0.6 * grown
                } else {
                    0.15
                };
                cv.disc(px, py, 2.6, tc, a);
            }
        }
    }
    // The middle: the selected tool, what a press does, food and mould.
    let mx = 350;
    let tool = view.tool;
    let line1 = match stock(d, tool) {
        None => format!("{}: always to hand", title(tool.name())),
        Some((have, of, _)) => {
            let mut s = format!("{}: {have} of {of} left", title(tool.name()));
            if let (Tool::Oats, Some(w)) = (tool, oats_wait(d)) {
                s.push_str(&format!(", +1 in {w} s"));
            }
            s
        }
    };
    text(cv, th, mx, y0 + 6, &line1, th.hot);
    let h = hint(d, view, st);
    let hl = format!("{} {}", h[0], h[1]);
    text(cv, th, mx, y0 + 30, &hl, if st.alarm { th.storm } else { th.text });
    let w = food_pips(cv, th, mx, y0 + 66, st);
    let total = st.joined.len() as u32 + st.coming;
    text(cv, th, mx + w + 12, y0 + 58, &format!("{} of {} food joined, mould {}", st.reach, total, d.walkers.len()), th.dim);
    // The storm dial.
    cv.ring(dcx, dcy, dr, 5.0, [20, 30, 40], 1.0, None);
    let label = match d.phase {
        Phase::Growing if !st.lesson => clock(st.left),
        Phase::Storm(_) => "now".into(),
        _ => "--".into(),
    };
    if let Some((f, ticks)) = timeline(d) {
        let left = 1.0 - f;
        let c = if st.alarm { th.storm } else { lerp(th.storm, th.text, 0.4) };
        if st.alarm {
            cv.disc(dcx, dcy, dr - 4.0, th.storm, if st.flash { 0.55 } else { 0.25 });
        }
        cv.arc(dcx, dcy, dr, 5.0, left, c, 1.0);
        for t in ticks {
            // Late food on the dial, where the hand will pass it.
            let ang = (1.0 - t) * TAU;
            cv.disc(dcx + ang.sin() * dr, dcy - ang.cos() * dr, 4.0, th.food, 1.0);
        }
    }
    let lw = text_w(th, &label);
    text(cv, th, dcx as i32 - lw / 2, dcy as i32 - 10, &label, if st.alarm { [255, 255, 255] } else { th.text });
    let sw = text_w(th, "storm");
    text(cv, th, dcx as i32 - dr as i32 - sw - 10, dcy as i32 - 10, "storm", th.dim);
}

fn title(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_string() + &c.as_str().to_ascii_lowercase(),
        None => String::new(),
    }
}

fn hud_toy(cv: &mut Cv, d: &Dish, view: &View, th: &Theme, y0: i32, st: &Status) {
    let ink = th.ink.unwrap_or([0, 0, 0]);
    // A wooden bench.
    let wave: Vec<f32> = (0..256).map(|k| 1.0 + (k as f32 / 256.0 * TAU).sin() * 0.04).collect();
    for y in y0..cv.h {
        let g = ((y as u32).wrapping_mul(2_654_435_761) >> 29) as f32;
        let c = mul(th.hud_bg, 0.86 + g * 0.03);
        let shades: Vec<Rgb> = wave.iter().map(|w| mul(c, *w)).collect();
        let shift = (y * 16) as usize;
        for x in 0..cv.w {
            cv.put(x, y, shades[(x as usize / 2 + shift) & 255]);
        }
    }
    cv.rect(0, y0, cv.w, 3, ink, 1.0);
    if let Some(words) = score_words(d) {
        chunky(cv, th, 3, 14, y0 + 6, &words[0], th.hot);
        chunky(cv, th, 2, 14, y0 + 38, &words[1], th.text);
        chunky(cv, th, 2, 14, y0 + 60, &words[2], th.dim);
        let (stars, _) = stars_of(d);
        for (k, &on) in stars.iter().enumerate() {
            star(cv, th, cv.w - 250 + k as i32 * 78, y0 + 14, 6, on);
        }
        return;
    }
    for (k, t) in Tool::ALL.iter().enumerate() {
        let sel = *t == view.tool;
        let x = 10 + k as i32 * 124;
        let y = y0 + 14 - if sel { 5 } else { 0 };
        let st_ = stock(d, *t);
        let spent = st_.is_some_and(|(h, _, _)| h == 0);
        let base = tool_colour(th, *t);
        let fill = if spent {
            [150, 140, 130]
        } else if sel {
            base
        } else {
            mul(base, 0.72)
        };
        cv.rbox(x + 3, y0 + 18, 114, 62, 12.0, Some((ink, 0.45)), None);
        if sel {
            // The tool in hand: lifted, and ringed in cream outside its ink.
            cv.rbox(x - 4, y - 4, 122, 70, 15.0, Some(([255, 250, 236], 1.0)), None);
        }
        cv.rbox(x, y, 114, 62, 12.0, Some((fill, 1.0)), Some((ink, 3.0)));
        cv.rbox(x + 6, y + 5, 102, 10, 5.0, Some(([255, 255, 255], if sel { 0.4 } else { 0.18 })), None);
        cv.rbox(x + 7, y + 13, 42, 42, 8.0, Some(([255, 250, 236], 1.0)), Some((ink, 2.0)));
        icon(cv, th, *t, x + 10, y + 16, 3, spent);
        // Key badge.
        cv.disc((x + 6) as f32, (y + 4) as f32, 14.0, ink, 1.0);
        cv.disc((x + 6) as f32, (y + 4) as f32, 11.5, [255, 250, 236], 1.0);
        hud::draw_text_scaled(cv.f, (cv.w as u32, cv.h as u32), 2, ((x + 2) / 2, (y - 2) / 2), &(k + 1).to_string(), [ink[0], ink[1], ink[2], 255]);
        match st_ {
            None => {
                chunky(cv, th, 2, x + 54, y + 20, "Hold", [255, 255, 255]);
            }
            Some((have, of, grown)) => {
                chunky(cv, th, 3, x + 54, y + 10, &format!("x{have}"), if spent { [210, 200, 190] } else { [255, 255, 255] });
                if *t == Tool::Oats && have < of {
                    cv.rbox(x + 56, y + 42, 48, 10, 4.0, Some(([250, 244, 230], 1.0)), Some((ink, 2.0)));
                    cv.rbox(x + 58, y + 44, (44.0 * grown) as i32, 6, 3.0, Some((th.hot, 1.0)), None);
                }
            }
        }
    }
    // Status stickers.
    let sx = 516;
    let ph = phase_words(d, st);
    let (bg, fg) = if st.alarm && st.flash { (th.storm, [255, 255, 255]) } else { ([252, 244, 222], ink) };
    cv.rbox(sx, y0 + 10, 236, 32, 10.0, Some((bg, 1.0)), Some((ink, 3.0)));
    hud::draw_text_scaled(cv.f, (cv.w as u32, cv.h as u32), 2, ((sx + 12) / 2, (y0 + 20) / 2), &ph, [fg[0], fg[1], fg[2], 255]);
    let fx = sx + 248;
    let fw = cv.w - fx - 8;
    cv.rbox(fx, y0 + 10, fw, 32, 10.0, Some(([252, 244, 222], 1.0)), Some((ink, 3.0)));
    hud::draw_text_scaled(cv.f, (cv.w as u32, cv.h as u32), 2, ((fx + 10) / 2, (y0 + 20) / 2), "FOOD", [ink[0], ink[1], ink[2], 255]);
    let pw = food_pips(cv, th, fx + 64, y0 + 26, st);
    let total = st.joined.len() as u32 + st.coming;
    hud::draw_text_scaled(cv.f, (cv.w as u32, cv.h as u32), 2, ((fx + 80 + pw) / 2, (y0 + 20) / 2), &format!("{}/{}", st.reach, total), [ink[0], ink[1], ink[2], 255]);
    // The fuse to the storm, and the mould's size.
    let m = format!("Mould {}", d.walkers.len());
    chunky(cv, th, 2, sx, y0 + 52, &m, th.text);
    let (bx, bw, by) = (sx + 170, cv.w - sx - 178, y0 + 56);
    if let Some((f, ticks)) = timeline(d) {
        cv.rbox(bx, by, bw, 14, 7.0, Some(([250, 244, 230], 1.0)), Some((ink, 3.0)));
        let wf = warn_frac(d);
        let wx = bx + (bw as f32 * (1.0 - wf)) as i32;
        cv.rbox(wx, by + 2, bx + bw - wx - 2, 10, 5.0, Some((th.storm, 1.0)), None);
        cv.rbox(bx + 2, by + 2, (bw as f32 * f) as i32, 10, 5.0, Some(([90, 70, 60], 1.0)), None);
        for t in ticks {
            let tx = bx as f32 + bw as f32 * t;
            cv.disc(tx, (by - 4) as f32, 6.0, ink, 1.0);
            cv.disc(tx, (by - 4) as f32, 4.0, th.food, 1.0);
        }
        // The spark where the fuse is burning.
        let px = bx as f32 + bw as f32 * f;
        let flick = if st.flash { 7.0 } else { 5.5 };
        cv.disc(px, (by + 7) as f32, flick + 2.0, ink, 1.0);
        cv.disc(px, (by + 7) as f32, flick, [255, 170, 40], 1.0);
        cv.disc(px, (by + 7) as f32, flick * 0.5, [255, 250, 200], 1.0);
    }
}

// ------------------------------------------------------------------ cards

fn card(cv: &mut Cv, th: &Theme, x: i32, y: i32, w: i32, h: i32, edge: Option<Rgb>) {
    match th.style {
        Style::Toy => {
            let ink = th.ink.unwrap_or([0, 0, 0]);
            cv.rbox(x + 5, y + 6, w, h, th.radius, Some((ink, 0.45)), None);
            cv.rbox(x, y, w, h, th.radius, Some((th.panel, 1.0)), Some((ink, 4.0)));
            cv.rbox(x + 6, y + 6, w - 12, h - 12, th.radius - 5.0, None, Some((edge.unwrap_or(th.panel_edge), 2.0)));
        }
        Style::Glow => {
            let e = edge.unwrap_or(th.panel_edge);
            cv.rbox(x - 6, y - 6, w + 12, h + 12, th.radius + 6.0, None, Some((e, 6.0)));
            cv.rbox(x - 6, y - 6, w + 12, h + 12, th.radius + 6.0, Some((th.panel, 0.0)), None);
            cv.rbox(x, y, w, h, th.radius, Some((th.panel, th.panel_alpha)), Some((e, 1.5)));
            // Soften the outer glow: redraw the panel's surround faintly.
            cv.rbox(x - 6, y - 6, w + 12, h + 12, th.radius + 6.0, None, Some((th.hud_bg, 3.0)));
        }
        _ => {
            cv.rbox(x, y, w, h, th.radius, Some((th.panel, th.panel_alpha)), Some((edge.unwrap_or(th.panel_edge), 1.0)));
            cv.rect(x + 1, y + 1, w - 2, 3, edge.unwrap_or(th.hot), 1.0);
        }
    }
}

/// One step of the lesson, across the top of the dish: the step's tool,
/// where it is in the six, and the two lines.
fn banner(cv: &mut Cv, th: &Theme, b: &super::render::Banner, d: &Dish) {
    let lines: Vec<String> = if th.font == Font::Mixed {
        sentence(&b.lines)
    } else {
        b.lines.iter().map(|s| s.to_string()).collect()
    };
    let head = if b.number > b.of {
        "Lesson done".to_string()
    } else {
        format!("Lesson {} of {}", b.number, b.of)
    };
    let tool = match b.number {
        1 => Some(Tool::Lamp),
        2 | 3 => Some(Tool::Oats),
        4 => Some(Tool::Snip),
        5 => Some(Tool::Salt),
        _ => None,
    };
    let lh = line_h(th);
    let tw = lines.iter().map(|l| text_w(th, l)).max().unwrap_or(0).max(text_w(th, &head) + 16 * b.of as i32 + 16);
    let icon_w = 48;
    let w = tw + icon_w + 32;
    let h = 3 * lh + 22;
    let x0 = (cv.w - w) / 2;
    let y0 = 6;
    let edge = if b.fresh { Some(th.hot) } else { None };
    card(cv, th, x0, y0, w, h, edge);
    match tool {
        Some(t) => icon(cv, th, t, x0 + 14, y0 + (h - 36) / 2, 3, false),
        None => star(cv, th, x0 + 16, y0 + (h - 36) / 2, 4, true),
    }
    let tx = x0 + 14 + icon_w;
    let hw = text(cv, th, tx, y0 + 10, &head, th.hot);
    for k in 0..b.of {
        let px = (tx + hw + 18 + k as i32 * 16) as f32;
        let py = (y0 + 10 + 7) as f32;
        let n = k + 1;
        if n < b.number {
            cv.disc(px, py, 5.0, th.hot, 1.0);
        } else if n == b.number {
            let beat = 0.6 + 0.4 * ((d.t as f32 / STEPS_PER_SECOND as f32) * TAU).sin();
            cv.ring(px, py, 5.0, 2.0, th.hot, 1.0, None);
            cv.disc(px, py, 2.5, th.hot, beat);
        } else {
            cv.ring(px, py, 5.0, 1.5, th.dim, 1.0, None);
        }
    }
    for (k, l) in lines.iter().enumerate() {
        text(cv, th, tx, y0 + 10 + (k as i32 + 1) * lh, l, th.text);
    }
}

enum Row {
    Gap,
    Stars(u32, String),
    Crack(&'static str),
    Head(&'static str),
    Line(String, Rgb),
    Swatch(u8, &'static str),
    Star(&'static str),
    Tool(Tool, String),
}

/// The briefing card: what the game is, in the order a new player needs it,
/// with the real colours and icons beside the words that name them.
fn brief(cv: &mut Cv, d: &Dish, view: &View, th: &Theme, s: i32) {
    let lesson = view.banner.is_some();
    let storm = if lesson {
        "A storm comes at the end.".to_string()
    } else {
        format!("A storm comes at {}.", clock(super::DISH_SECONDS))
    };
    let tools = [
        Row::Head("Tools"),
        Row::Tool(Tool::Lamp, "Lamp   hold: pushes the mould away".into()),
        Row::Tool(Tool::Oats, "Oats   click: it grows toward them".into()),
        Row::Tool(Tool::Salt, "Salt   drag: a wall it can't cross".into()),
        Row::Tool(Tool::Snip, "Snip   drag: cut a tube".into()),
    ];
    let name = title(wander::NAME);
    if let Some(l) = d.level {
        return brief_card(cv, th, s, &title(l.name), &level_rows(d, l, th));
    }
    let (heading, rows) = match &d.wander {
        Some(w) => {
            let heading = if w.stretch == 1 {
                format!("{name}, the Wanderer")
            } else {
                format!("{name}, dish {}", w.stretch)
            };
            let mut rows = vec![];
            if w.stretch > 1 {
                rows.push(Row::Line(format!("{} of her came this far.", thousands(w.came_in)), th.hot));
            }
            rows.extend([
                Row::Line(format!("{name} is your slime mould. The food"), th.text),
                Row::Line("under her runs out. Lead her to the".into(), th.text),
                Row::Line("glowing crack on the right, then G: go.".into(), exit_teal(th)),
                Row::Gap,
                Row::Swatch(0, "Yellow: fed mould"),
                Row::Swatch(1, "Grey: cut off from food, starving"),
                Row::Swatch(2, "Tan: food"),
                Row::Crack("Teal: the way on"),
                Row::Gap,
                Row::Head("Who comes with her"),
                Row::Line("All fed mould joined to the crack pours".into(), th.text),
                Row::Line("out along its tubes, and starves in a".into(), th.text),
                Row::Line("long or narrow line: build a thick, short".into(), th.text),
                Row::Line("trunk. What arrives starts the next dish.".into(), th.text),
                Row::Gap,
            ]);
            rows.extend(tools);
            (heading, rows)
        }
        None => {
            let mut rows = brief_rows(th, storm);
            rows.extend(tools);
            ("Mould".to_string(), rows)
        }
    };
    brief_card(cv, th, s, &heading, &rows);
}

/// A challenge dish's card: its goal, its one twist, what each star needs,
/// and only the tools it gives, with how many.
fn level_rows(d: &Dish, l: &super::level::Level, th: &Theme) -> Vec<Row> {
    use super::level::clock;
    let bell = clock(l.seconds);
    let stars = match l.win {
        Win::JoinAll { by } => [
            format!("Reach the food by {bell}"),
            format!("By {}", clock(by[0])),
            format!("By {}", clock(by[1])),
        ],
        Win::UnderRuler { bars } => [
            format!("All three joined at {bell}"),
            format!("Tube {:.2}x the ruler or less", bars[0]),
            format!("{:.2}x or less", bars[1]),
        ],
        Win::AliveAtBell { bars } => [
            format!("{} mould alive at {bell}", bars[0]),
            format!("{} alive", bars[1]),
            format!("{} alive", bars[2]),
        ],
    };
    let goal = sentence(&l.goal);
    let mut rows = vec![
        Row::Line(goal[0].clone(), th.text),
        Row::Line(goal[1].clone(), th.text),
        Row::Line(up(l.twist), th.dim),
        Row::Gap,
        Row::Swatch(0, "Yellow: fed mould"),
        Row::Swatch(1, "Grey: cut off from food, starving"),
        Row::Swatch(2, "Tan: food"),
        Row::Gap,
        Row::Head("Stars"),
    ];
    for (k, line) in stars.into_iter().enumerate() {
        rows.push(Row::Stars(k as u32 + 1, line));
    }
    rows.push(Row::Gap);
    rows.push(Row::Head("Tools"));
    let t = l.tools;
    rows.push(Row::Tool(Tool::Lamp, "Lamp   hold: pushes the mould away".into()));
    if d.has_tool(Tool::Oats) {
        let back = if t.oats_regrow > 0 {
            format!(", 1 back each {} s", t.oats_regrow)
        } else {
            String::new()
        };
        rows.push(Row::Tool(Tool::Oats, format!("Oats {}{back}: it grows to them", t.oats)));
    }
    if d.has_tool(Tool::Salt) {
        rows.push(Row::Tool(Tool::Salt, format!("Salt {}   drag: a wall", t.salt)));
    }
    if d.has_tool(Tool::Snip) {
        rows.push(Row::Tool(Tool::Snip, format!("Snip {}   drag: cut a tube", t.snips)));
    }
    rows
}

/// The scored dish's card, above its tools.
fn brief_rows(th: &Theme, storm: String) -> Vec<Row> {
    vec![
        Row::Line("Your mould lives on food. Join the".into(), th.text),
        Row::Line("food piles with its tubes.".into(), th.text),
        Row::Line(storm, th.text),
        Row::Gap,
        Row::Swatch(0, "Yellow: fed mould"),
        Row::Swatch(1, "Grey: cut off from food, starving"),
        Row::Swatch(2, "Tan: food"),
        Row::Gap,
        Row::Head("Stars"),
        Row::Star("Join all the food"),
        // A ladder (review 1): shape and the storm count only once all
        // the food is joined.
        Row::Star("All joined, and the network short"),
        Row::Star("All joined, nothing lost to the storm"),
        Row::Gap,
    ]
}

fn brief_card(cv: &mut Cv, th: &Theme, s: i32, heading: &str, rows: &[Row]) {
    let lh = line_h(th) + 4;
    let indent = 40;
    let tool_indent = 52;
    let stars_indent = 72;
    let tw = rows
        .iter()
        .map(|r| match r {
            Row::Line(l, _) => text_w(th, l),
            Row::Swatch(_, l) | Row::Star(l) | Row::Crack(l) => text_w(th, l) + indent,
            Row::Stars(_, l) => text_w(th, l) + stars_indent,
            Row::Tool(_, l) => text_w(th, l) + tool_indent,
            _ => 0,
        })
        .max()
        .unwrap_or(0)
        .max(big_w(th, 4, heading));
    let w = tw + 48;
    let title_h = 7 * 4 + 16;
    let prompt = "Click or press a key to start";
    let h = title_h + lh * rows.len() as i32 + lh + 36;
    let dish_h = H as i32 * s;
    let (x0, y0) = ((cv.w - w) / 2, ((dish_h - h) / 2).max(4));
    card(cv, th, x0, y0, w, h, None);
    let x = x0 + 24;
    big(cv, th, 4, x, y0 + 14, heading, th.hot);
    let mut y = y0 + 14 + title_h;
    for r in rows {
        match r {
            Row::Gap => {}
            Row::Head(l) => {
                text(cv, th, x, y, l, th.hot);
            }
            Row::Line(l, c) => {
                text(cv, th, x, y, l, *c);
            }
            Row::Swatch(kind, l) => {
                let (sx, sy) = (x, y + 2);
                match kind {
                    0 | 1 => {
                        // A short length of real tube in the look's own ramp.
                        for k in 0..28 {
                            let v = 1.0 - ((k as f32 - 14.0) / 14.0).abs();
                            let c = tube_colour(th, 0.25 + 0.7 * v, *kind == 0, 0.0);
                            cv.rect(sx + k, sy, 1, 12, c, 1.0);
                        }
                        if let Some(ink) = th.ink {
                            cv.rbox(sx - 2, sy - 2, 32, 16, 6.0, None, Some((ink, 2.0)));
                        }
                    }
                    _ => {
                        if let Some(ink) = th.ink {
                            cv.disc((sx + 14) as f32, (sy + 6) as f32, 9.0, ink, 1.0);
                        }
                        grain_disc(cv, (sx + 14) as f32, (sy + 6) as f32, 7.0, th.food, 1.0);
                    }
                }
                text(cv, th, x + indent, y, l, th.text);
            }
            Row::Crack(l) => {
                let teal = exit_teal(th);
                for k in 0..28 {
                    let g = 0.4 + 0.6 * k as f32 / 27.0;
                    cv.rect(x + k, y + 2, 1, 12, lerp(th.agar, teal, g), 1.0);
                }
                text(cv, th, x + indent, y, l, th.text);
            }
            Row::Star(l) => {
                star(cv, th, x + 6, y, 2, true);
                text(cv, th, x + indent, y, l, th.text);
            }
            Row::Stars(n, l) => {
                for k in 0..*n {
                    star(cv, th, x + 2 + k as i32 * 18, y, 2, true);
                }
                text(cv, th, x + stars_indent, y, l, th.text);
            }
            Row::Tool(t, l) => {
                let k = Tool::ALL.iter().position(|u| u == t).unwrap_or(0) + 1;
                icon(cv, th, *t, x, y - 4, 2, false);
                text(cv, th, x + 30, y, &k.to_string(), th.hot);
                text(cv, th, x + tool_indent, y, l, th.text);
            }
        }
        y += lh;
    }
    let pw = text_w(th, prompt);
    text(cv, th, x0 + (w - pw) / 2, y + 10, prompt, th.hot);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every string a look draws must be in the fonts: a missing glyph draws
    /// a silent gap (`hud::has_glyph`'s doc: it has shipped three times).
    #[test]
    fn every_look_string_has_its_glyphs() {
        let mut words: Vec<String> = Vec::new();
        for t in Tool::ALL {
            let v = View {
                tool: t,
                ..View::default()
            };
            let st = Status {
                joined: vec![],
                reach: 0,
                coming: 0,
                lesson: false,
                left: 3,
                alarm: true,
                flash: true,
            };
            let d = Dish::new(1);
            words.extend(hint(&d, &v, &st));
        }
        words.extend(
            [
                "Storm cuts here in 4",
                "Storm will cut here",
                "Storm scar",
                "No oats: next in 7 s",
                "Oats can't go here",
                "It will reach these",
                "It may reach these, slowly",
                "Too far to pull: 31 cells",
                "Wall left: 30 of 60",
                "Drag to cut this tube",
                "No tube here",
                "Drag: a wall 60 long",
                "Salt used up",
                "No snips left",
                "Not now: the storm",
                "Space: fast  P: pause",
                "Fast, 4x",
                "Network 1.32x the shortest (star at 1.15x)",
                "Joined 4 of 5 before the storm struck",
                "3 of 6 food still joined",
                "+1 in 7s",
                "Lesson: no storm",
                "Storm! Score in 6 s",
                "Mabel arrived with 1,234 of 1,900",
                "67%. Left at 0:38, 307 died in line",
                "N: dish 2 with 926   R: again",
                "Mabel didn't make it",
                "She starved at 2:10",
                "Through 532  in line 507  died 194",
                "G: go. 1,258 of 1,265 would come",
                "Lead her to the crack, then G",
                "Mould 1,265  best 1,387",
                "G: go now. She leaves",
                "The way on",
                "Teal: the way on",
                "Mabel, the Wanderer",
                "Who comes with her",
                "long or narrow line: build a thick, short",
                "N: next dish, with her",
                "On her way",
            ]
            .map(String::from),
        );
        words.extend(
            [
                "Not here",
                "Bell in 0:41",
                "All joined: 1.32x the ruler",
                "Eating 2 piles",
                "Best: 0.87x the ruler",
                "Ruler",
                "Dishes",
                "1-6 or click to pick   M: back",
                "Locked: finish the lesson",
                "R: again  N: next  M: dishes",
                "Tube 1.15x the ruler or less",
                "Oats 3, 1 back each 9 s: it grows to them",
                "Done",
                "2 walls",
                "Press Esc again to quit",
                "All joined, and the network short",
                "All joined, nothing lost to the storm",
                "The mould died at 1:23",
                "Not reached: only starving strays got there",
            ]
            .map(String::from),
        );
        for l in &super::super::level::DISHES {
            words.extend([title(l.name), up(l.goal[0]), up(l.goal[1]), up(l.twist), up(&l.stars_line())]);
        }
        for e in 0..super::super::level::ENTRIES {
            let (name, lines) = entry_lines(e);
            words.extend([title(name), up(lines[0]), up(lines[1])]);
        }
        for w in &words {
            for c in w.chars() {
                assert!(c == ' ' || hud::has_glyph(c) || lower(c).is_some(), "{c:?} in {w:?}");
            }
        }
    }

    #[test]
    fn up_sets_a_ratio_small() {
        assert_eq!(up("ALL 3 JOINED, 0.88X THE RULER"), "All 3 joined, 0.88x the ruler");
        assert_eq!(up("N: NEXT DISH"), "N: next dish");
        assert_eq!(up("400 MOULD ALIVE AT THE BELL"), "400 mould alive at the bell");
        assert_eq!(up("3 OF 6 FOODS"), "3 of 6 foods");
    }

    #[test]
    fn sentence_case_keeps_key_letters() {
        let s = sentence(&["THAT'S THE GAME: JOIN ALL THE FOOD,", "SHORT, AND STORM-PROOF. N: DISH 1"]);
        assert_eq!(s[0], "That's the game: join all the food,");
        assert_eq!(s[1], "short, and storm-proof. N: dish 1");
    }

    #[test]
    fn styles_parse_by_name_and_letter_and_cycle() {
        for st in Style::ALL {
            assert_eq!(Style::parse(st.name()), Some(st));
            assert_eq!(Style::parse(st.letter()).filter(|_| st != Style::Classic), (st != Style::Classic).then_some(st));
        }
        let mut st = Style::Classic;
        for _ in 0..Style::ALL.len() {
            st = st.next();
        }
        assert_eq!(st, Style::Classic);
    }

    /// The HUD's own joined count must agree with the score's.
    #[test]
    fn status_joined_matches_measure() {
        let mut d = Dish::new(1);
        for _ in 0..(20 * STEPS_PER_SECOND) {
            d.step();
        }
        assert_eq!(status(&d).reach, d.measure().reach);
    }
}

#[cfg(test)]
mod cost {
    use super::*;
    use std::time::Instant;

    /// Where a look's frame time goes, section by section, on a full dish.
    /// `cargo test --release --lib mould::look::cost -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn sections() {
        let mut d = Dish::new(1);
        let (fw, fh) = frame_size(4);
        let mut buf = vec![0u8; (fw * fh * 4) as usize];
        for (at, style) in [40, 87].into_iter().flat_map(|a| [Style::Night, Style::Bench, Style::Glow, Style::Toy].map(|s| (a, s))) {
            while d.t < at * STEPS_PER_SECOND as u64 {
                d.step();
            }
            let view = View {
                style,
                tool: Tool::Oats,
                cursor: Some((128.0, 80.0)),
                ..View::default()
            };
            let th = theme(style);
            let n = 100;
            let mut t = [0.0f64; 7];
            for _ in 0..n {
                let mut cv = Cv {
                    f: &mut buf,
                    w: fw as i32,
                    h: fh as i32,
                };
                let mut at = Instant::now();
                let mut lap = |k: usize, at: &mut Instant| {
                    t[k] += at.elapsed().as_secs_f64() * 1e3;
                    *at = Instant::now();
                };
                let st = status(&d);
                lap(0, &mut at);
                dish(&mut cv, &d, &th, 4, &st);
                lap(1, &mut at);
                piles(&mut cv, &d, &th, 4, &st);
                lap(2, &mut at);
                marks(&mut cv, &d, &th, 4);
                storm(&mut cv, &d, &th, 4, &st);
                lamp(&mut cv, &d, &th, 4);
                lap(3, &mut at);
                preview(&mut cv, &d, &view, &th, 4);
                lap(4, &mut at);
                corners(&mut cv, &d, &view, &th, &st, 4);
                lap(5, &mut at);
                hud_strip(&mut cv, &d, &view, &th, 4, &st);
                lap(6, &mut at);
            }
            let names = ["status", "dish", "piles", "marks+storm+lamp", "preview", "corners", "hud"];
            let parts: Vec<String> = names
                .iter()
                .zip(t)
                .map(|(m, v)| format!("{m} {:.2}", v / n as f64))
                .collect();
            let hold = n as f64;
            let mut t2 = Instant::now();
            for _ in 0..n {
                let mut cv = Cv {
                    f: &mut buf,
                    w: fw as i32,
                    h: fh as i32,
                };
                let st = status(&d);
                alarm(&mut cv, &d, &view, &th, 4, &st);
            }
            let al = t2.elapsed().as_secs_f64() * 1e3 / hold;
            t2 = Instant::now();
            for _ in 0..n {
                draw(&d, &view, 4, &mut buf);
            }
            let all = t2.elapsed().as_secs_f64() * 1e3 / hold;
            println!("{at} s {:?}: {}, alarm {al:.2}; whole draw {all:.2}", style, parts.join(", "));
        }
    }
}
