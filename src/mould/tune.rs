//! **The tuning panel**: every part of the food flow (`Params::flow` and
//! what rides on it) as a switch or a number the owner can change while
//! the dish runs (owner, 2026-11-09: "each part of it built behind a
//! switch or a slider/number that I could edit and play with").
//!
//! T in the window opens it; Up and Down pick a row, Left and Right change
//! it (Shift for five steps), Enter flips a switch, Backspace puts the row
//! back to its default. What is set carries over to every dish the window
//! opens, and is saved to `TUNE_FILE` in the working directory, which is
//! read back at the next start and can be edited by hand (`name=value`,
//! one a line). `mouldprobe tune=FILE` reads the same file.

use super::Params;
use crate::hud;

/// The file the panel saves to and the window reads at start.
pub const TUNE_FILE: &str = "mould-tune.txt";

/// One row of the panel.
pub struct Knob {
    /// The name in the file and on the panel.
    pub name: &'static str,
    /// What it does, in a few words, shown under the panel for the row
    /// picked.
    pub help: &'static str,
    pub get: fn(&Params) -> f32,
    pub set: fn(&mut Params, f32),
    pub step: f32,
    pub min: f32,
    pub max: f32,
    /// On or off rather than a number.
    pub switch: bool,
}

fn b(v: bool) -> f32 {
    if v {
        1.0
    } else {
        0.0
    }
}

/// The rows, in panel order: level 1 (food carried), level 2 (food in the
/// tube, its stream and pulse), level 3 (walls), then the old rule's reach.
pub static KNOBS: [Knob; 15] = [
    Knob {
        name: "flow",
        help: "Food flows. Off: the old rule, food reaches feed_reach cells of tube at once",
        get: |p| b(p.flow),
        set: |p, v| p.flow = v > 0.5,
        step: 1.0,
        min: 0.0,
        max: 1.0,
        switch: true,
    },
    Knob {
        name: "want",
        help: "Load a walker tops up to from the tube, and needs to divide",
        get: |p| p.want,
        set: |p, v| p.want = v,
        step: 0.1,
        min: 0.1,
        max: 5.0,
        switch: false,
    },
    Knob {
        name: "carry_cap",
        help: "Most a walker loads on a pile (food in the tube off)",
        get: |p| p.carry_cap,
        set: |p, v| p.carry_cap = v,
        step: 0.25,
        min: 0.25,
        max: 10.0,
        switch: false,
    },
    Knob {
        name: "share",
        help: "How much two walkers side by side even out their loads a step",
        get: |p| p.share,
        set: |p, v| p.share = v,
        step: 0.05,
        min: 0.0,
        max: 1.0,
        switch: false,
    },
    Knob {
        name: "sap",
        help: "Food in the tube: piles pump into the tube, walkers draw from it",
        get: |p| b(p.sap),
        set: |p, v| p.sap = v > 0.5,
        step: 1.0,
        min: 0.0,
        max: 1.0,
        switch: true,
    },
    Knob {
        name: "absorb",
        help: "Share of the tube's food on its cell a hungry walker takes a step",
        get: |p| p.absorb,
        set: |p, v| p.absorb = v,
        step: 0.01,
        min: 0.0,
        max: 1.0,
        switch: false,
    },
    Knob {
        name: "sap_speed",
        help: "Spreading passes a step: how fast food soaks along the tube",
        get: |p| p.sap_speed as f32,
        set: |p, v| p.sap_speed = v.round().max(0.0) as u32,
        step: 1.0,
        min: 0.0,
        max: 40.0,
        switch: false,
    },
    Knob {
        name: "stream",
        help: "Stream: how far the tube's food surges along it each step",
        get: |p| p.stream,
        set: |p, v| p.stream = v,
        step: 0.05,
        min: 0.0,
        max: 1.0,
        switch: false,
    },
    Knob {
        name: "stream_bias",
        help: "How much more the stream goes out from food than back",
        get: |p| p.stream_bias,
        set: |p, v| p.stream_bias = v,
        step: 0.05,
        min: -1.0,
        max: 1.0,
        switch: false,
    },
    Knob {
        name: "pulse_strength",
        help: "Pulse: how hard piles pump and the stream surges on the beat (0 steady)",
        get: |p| p.pulse_strength,
        set: |p, v| p.pulse_strength = v,
        step: 0.05,
        min: 0.0,
        max: 1.0,
        switch: false,
    },
    Knob {
        name: "pulse_seconds",
        help: "Seconds per beat",
        get: |p| p.pulse_seconds,
        set: |p, v| p.pulse_seconds = v,
        step: 0.25,
        min: 0.25,
        max: 20.0,
        switch: false,
    },
    Knob {
        name: "wall_grow",
        help: "Walls: how fast food moving through a tube builds its wall (0 off)",
        get: |p| p.wall_grow,
        set: |p, v| p.wall_grow = v,
        step: 0.25,
        min: 0.0,
        max: 20.0,
        switch: false,
    },
    Knob {
        name: "wall_half_life",
        help: "Seconds an unused wall takes to fade to half",
        get: |p| p.wall_half_life,
        set: |p, v| p.wall_half_life = v,
        step: 5.0,
        min: 1.0,
        max: 600.0,
        switch: false,
    },
    Knob {
        name: "wall_pull",
        help: "How strongly walkers follow a wall, against the trail",
        get: |p| p.wall_pull,
        set: |p, v| p.wall_pull = v,
        step: 0.1,
        min: 0.0,
        max: 10.0,
        switch: false,
    },
    Knob {
        name: "feed_reach",
        help: "Old rule only (flow off): cells of tube food reaches",
        get: |p| p.feed_reach as f32,
        set: |p, v| p.feed_reach = v.round().clamp(1.0, 1000.0) as u16,
        step: 5.0,
        min: 5.0,
        max: 400.0,
        switch: false,
    },
];

/// Copy every knob's value from `from` into `to`, leaving the rest of `to`
/// (a challenge dish's own rules) alone.
pub fn apply(from: &Params, to: &mut Params) {
    for k in &KNOBS {
        (k.set)(to, (k.get)(from));
    }
}

/// Step knob `row` by `steps` (negative goes down), within its range; a
/// switch flips whatever `steps` is.
pub fn nudge(p: &mut Params, row: usize, steps: i32) {
    let k = &KNOBS[row];
    let v = (k.get)(p);
    let v = if k.switch { 1.0 - v } else { (v + k.step * steps as f32).clamp(k.min, k.max) };
    // Snap to the step, so repeated nudges do not drift (0.30000001).
    let v = if k.switch { v } else { (v / k.step).round() * k.step };
    (k.set)(p, v);
}

/// Put knob `row` back to its default.
pub fn reset(p: &mut Params, row: usize) {
    let k = &KNOBS[row];
    (k.set)(p, (k.get)(&Params::default()));
}

pub fn value_text(p: &Params, row: usize) -> String {
    let k = &KNOBS[row];
    let v = (k.get)(p);
    if k.switch {
        (if v > 0.5 { "ON" } else { "off" }).to_string()
    } else if k.step >= 1.0 {
        format!("{v:.0}")
    } else if k.step >= 0.1 {
        format!("{v:.2}")
    } else {
        format!("{v:.3}")
    }
}

/// The knobs as `name=value` lines.
pub fn to_text(p: &Params) -> String {
    let mut s = String::from("# Mould tuning (T in the window). name=value, one a line.\n");
    for (r, k) in KNOBS.iter().enumerate() {
        s.push_str(&format!("{}={}\n", k.name, value_text(p, r).replace("ON", "1").replace("off", "0")));
    }
    s
}

/// Read `name=value` lines into `p`; unknown names and bad numbers are
/// reported, not ignored silently.
pub fn from_text(text: &str, p: &mut Params) -> Vec<String> {
    let mut bad = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, v)) = line.split_once('=') else {
            bad.push(line.to_string());
            continue;
        };
        let Some(k) = KNOBS.iter().find(|k| k.name == name.trim()) else {
            bad.push(line.to_string());
            continue;
        };
        let v = v.trim();
        let v = match v {
            "on" | "ON" | "true" => Some(1.0),
            "off" | "OFF" | "false" => Some(0.0),
            _ => v.parse::<f32>().ok(),
        };
        match v {
            Some(v) => (k.set)(p, v.clamp(k.min, k.max)),
            None => bad.push(line.to_string()),
        }
    }
    bad
}

/// The panel, top right of the frame, over whatever the look drew: one
/// row a knob, the picked one marked, its help line under the panel.
pub fn draw(frame: &mut [u8], (fw, fh): (u32, u32), p: &Params, cursor: usize) {
    // Doubled glyphs in the window's frame (1024 wide); single in a
    // probe's half-size frame, where doubled ones covered the dish.
    let k_big = fw >= 900;
    #[allow(non_snake_case)]
    let K: i32 = if k_big { 2 } else { 1 };
    let line = (hud::GLYPH_HEIGHT + 3) * K;
    let rows: Vec<(String, String)> = (0..KNOBS.len()).map(|r| (KNOBS[r].name.to_string(), value_text(p, r))).collect();
    let name_w = rows.iter().map(|(n, _)| hud::text_width(n)).max().unwrap_or(0) * K;
    let w = name_w + 12 * K + 7 * 6 * K;
    let head = ["TUNE  (T closes)", "Up/Down pick  Left/Right change"];
    let h = line * (rows.len() as i32 + head.len() as i32 + 1) + 10;
    let x0 = fw as i32 - w - 8;
    let y0 = 8;
    fill(frame, (fw, fh), (x0, y0, w, h), [6, 10, 14], 0.9);
    let txt = |frame: &mut [u8], x: i32, y: i32, s: &str, c: [u8; 4]| {
        hud::draw_text_scaled(frame, (fw, fh), K, (x / K, y / K), s, c);
    };
    let mut y = y0 + 6;
    for hl in head {
        txt(frame, x0 + 8, y, hl, [150, 170, 180, 255]);
        y += line;
    }
    y += line / 2;
    for (r, (n, v)) in rows.iter().enumerate() {
        let on = r == cursor;
        if on {
            fill(frame, (fw, fh), (x0 + 2, y - 3, w - 4, line), [60, 52, 20], 1.0);
        }
        let c = if on { [255, 220, 110, 255] } else { [220, 228, 232, 255] };
        txt(frame, x0 + 8, y, n, c);
        txt(frame, x0 + 8 + name_w + 8 * K, y, v, c);
        y += line;
    }
    // The picked row's help, under the panel, wrapped to its width.
    let help = KNOBS[cursor.min(KNOBS.len() - 1)].help;
    let per = ((w - 16) / (6 * K)).max(8) as usize;
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in help.split(' ') {
        if !cur.is_empty() && cur.len() + 1 + word.len() > per {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    let hy = y0 + h + 4;
    fill(frame, (fw, fh), (x0, hy, w, line * lines.len() as i32 + 8), [6, 10, 14], 0.9);
    for (k, l) in lines.iter().enumerate() {
        txt(frame, x0 + 8, hy + 5 + k as i32 * line, l, [255, 220, 110, 255]);
    }
}

fn fill(frame: &mut [u8], (fw, fh): (u32, u32), (x, y, w, h): (i32, i32, i32, i32), c: [u8; 3], a: f32) {
    for py in y.max(0)..(y + h).min(fh as i32) {
        for px in x.max(0)..(x + w).min(fw as i32) {
            let o = (py as usize * fw as usize + px as usize) * 4;
            for k in 0..3 {
                frame[o + k] = (frame[o + k] as f32 * (1.0 - a) + c[k] as f32 * a) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::needless_range_loop)]
    fn the_file_round_trips_every_knob() {
        let mut p = Params::default();
        for r in 0..KNOBS.len() {
            nudge(&mut p, r, 3);
        }
        let mut q = Params::default();
        let bad = from_text(&to_text(&p), &mut q);
        assert!(bad.is_empty(), "{bad:?}");
        for r in 0..KNOBS.len() {
            assert_eq!(value_text(&p, r), value_text(&q, r), "{}", KNOBS[r].name);
        }
    }

    #[test]
    fn apply_leaves_a_dish_s_own_rules() {
        let mut to = Params { stray_reach: 24.0, ..Params::default() };
        let from = Params { flow: true, ..Params::default() };
        apply(&from, &mut to);
        assert!(to.flow);
        assert_eq!(to.stray_reach, 24.0);
    }
}
