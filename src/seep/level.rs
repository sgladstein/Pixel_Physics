//! **"Two Lamps"**, review-3 §4a, at 256x160: the shape of the mine, free
//! of the engine so a scratch harness can draw it without a `World`.
//!
//! Lower gallery y 110-140 with the mould's damp sump bottom left, crack 1
//! in its floor at x 180 under a roof dome, lamp B low at x 100. A slanted
//! shaft climbs from x 200 to the upper gallery (y 40-65), whose roof rises
//! to the right to lamp A at the high end -- where overflowing gas pools.
//! An air shaft at x 20-30 reaches the surface; a damp drip chimney at
//! x 150 joins the galleries.

use super::gas::{idx, H, W};
use super::mould::Rng;

pub struct Layout {
    pub open: Vec<bool>,
    pub wet: Vec<f32>,
    pub water: Vec<(i32, i32)>,
}

pub fn two_lamps(seed: u64) -> Layout {
    let _ = Rng(seed | 1);
    let mut open = vec![false; W * H];
    let mut wet = vec![0.0f32; W * H];
    let mut carve = |x: i32, y: i32| {
        if x >= 1 && y >= 0 && (x as usize) < W - 1 && (y as usize) < H - 1 {
            open[idx(x as usize, y as usize)] = true;
        }
    };
    // A gentle wobble so the rock does not read as ruled lines.
    let wob = |x: i32, k: f32| ((x as f32 * 0.21 + k).sin() * 1.2 + (x as f32 * 0.07 + 2.0 * k).sin() * 1.0).round() as i32;
    // Lower gallery: x 20-230, floor y 140, roof sloping up to the right so
    // the gas it collects runs toward the dome and the shaft.
    for x in 20..=230 {
        let roof = 113 - (x - 20) * 3 / 210 + wob(x, 1.0);
        let floor = 140 + wob(x, 4.0) / 2;
        for y in roof..=floor {
            carve(x, y);
        }
    }
    // The damp sump at the bottom left.
    for x in 20..=46 {
        let depth = 143 - ((x - 33) * (x - 33)) / 60;
        for y in 138..=depth.max(140) {
            carve(x, y);
        }
    }
    // The dome over crack 1.
    for x in 158..=202 {
        let t = (x - 180) as f32 / 22.0;
        let top = 112.0 - 17.0 * (1.0 - t * t).max(0.0).sqrt();
        for y in (top.round() as i32)..=112 {
            carve(x, y);
        }
    }
    // The slanted shaft, x 198-208 at the bottom climbing right to the
    // upper gallery's floor.
    for y in 60..=112 {
        let t = (112 - y) as f32 / 52.0;
        let cx = 203.0 + 18.0 * t;
        for x in (cx - 5.0).round() as i32..=(cx + 5.0).round() as i32 {
            carve(x, y);
        }
    }
    // Upper gallery: x 20-240, floor y 64, roof rising to the right.
    for x in 20..=242 {
        let roof = 50 - (x - 20) * 12 / 222 + wob(x, 7.0) / 2;
        let floor = 64 + wob(x, 9.0) / 2;
        for y in roof..=floor {
            carve(x, y);
        }
    }
    // The air shaft to the surface at the left.
    for y in 0..=52 {
        let w = wob(y, 3.0) / 2;
        for x in (20 + w)..=(30 + w) {
            carve(x, y);
        }
    }
    // The drip chimney at x 148-151.
    for y in 60..=114 {
        let w = wob(y, 5.0) / 3;
        for x in (147 + w)..=(151 + w) {
            carve(x, y);
        }
    }
    let mut water = Vec::new();
    for x in 26..=40 {
        for y in 142..=143 {
            if open[idx(x, y)] {
                water.push((x as i32, y as i32));
            }
        }
    }
    for y in 128..H as i32 {
        for x in 18..=50 {
            let i = idx(x as usize, y as usize);
            let d = (((x - 32) * (x - 32)) as f32 / 300.0 + ((y - 141) * (y - 141)) as f32 / 40.0).sqrt();
            wet[i] = (1.2 - d).clamp(0.0, 1.0);
        }
    }
    // The drip line: the chimney walls are damp top to bottom, and the
    // floor where it lands.
    for y in 58..=141 {
        for x in 144..=155 {
            wet[idx(x as usize, y as usize)] = if y > 112 { 0.6 } else { 1.0 };
        }
    }
    Layout { open, wet, water }
}
