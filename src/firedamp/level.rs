//! **"The Dip"**: the one level of the probe. A fresh-air shaft at the top
//! left drops into a U-shaped gallery. The bottom of the U is a sump with a
//! leak in its floor; on the right the gallery climbs to two work faces, one
//! low and one high, each with a lamp. An ore seam runs under the sump, a
//! smaller one sits in the rock beside the low face, and a sealed empty void
//! lies behind a thin wall to the left of the sump.
//!
//! Built as rock cells in an ordinary engine `World`, so the shipped
//! explosion code breaks it, rubble falls, and fracture opens new holes.

use crate::sim::cell::Cell;
use crate::sim::chunk::Rect;
use crate::sim::material::{self, MaterialId};
use crate::sim::structural;
use crate::sim::world::World;

use super::gas::{coin, idx, H, W};

pub struct Level {
    /// The two faces' lamps, low first.
    pub lamps: [(i32, i32); 2],
    /// Leak cells (where gas enters), and the crack's own position for the
    /// marker.
    pub leak: Vec<usize>,
    pub leak_at: (i32, i32),
    /// Top-of-shaft cells, where gas leaves the mine.
    pub vents: Vec<usize>,
    /// Cells fresh air comes in from, for the "is this face still breathing"
    /// fill.
    pub air: Vec<usize>,
    /// Every cell that started as ore.
    pub ore: Vec<usize>,
    pub ore_material: MaterialId,
    /// Gas the leak feeds per tick, before the seed's surges.
    pub leak_rate: f32,
    /// Ticks at which the leak surges (seeded), each for `SURGE_TICKS`.
    pub surges: Vec<u64>,
}

pub const SURGE_TICKS: u64 = 240;

fn seg_dist2(px: f32, py: f32, (ax, ay): (f32, f32), (bx, by): (f32, f32)) -> f32 {
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 { 0.0 } else { (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0) };
    let (qx, qy) = (ax + t * dx - px, ay + t * dy - py);
    qx * qx + qy * qy
}

/// A straight run of tunnel: from, to, radius.
type Tunnel = ((f32, f32), (f32, f32), f32);

/// The tunnels.
const TUNNELS: [Tunnel; 8] = [
    // the air shaft
    ((19.0, -4.0), (19.0, 48.0), 5.0),
    // the top gallery
    ((19.0, 48.0), (58.0, 50.0), 5.0),
    // down into the dip
    ((58.0, 50.0), (102.0, 112.0), 5.0),
    // the sump's own floor run
    ((104.0, 114.0), (140.0, 114.0), 5.0),
    // up out of the dip
    ((140.0, 114.0), (176.0, 96.0), 5.0),
    // the low face: a gallery off the climb that runs gently *down* to
    // its end, so gas that spills over the lip slides straight to the lamp
    ((158.0, 104.0), (190.0, 109.0), 5.0),
    // the high face, at the top of the climb
    ((176.0, 96.0), (204.0, 62.0), 5.0),
    ((204.0, 62.0), (240.0, 56.0), 5.0),
];

pub const SUMP: (f32, f32, f32, f32) = (122.0, 115.0, 21.0, 9.0);
const VOID: (f32, f32, f32, f32) = (78.0, 128.0, 12.0, 7.0);

pub fn build(world: &mut World, seed: u64) -> Level {
    let rock = material::STONE;
    let ore_material = world.materials.id_of("crystal").unwrap_or(material::STONE);
    let mut ore = Vec::new();
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let (fx, fy) = (x as f32, y as f32);
            let mut open = TUNNELS.iter().any(|&(a, b, r)| seg_dist2(fx, fy, a, b) <= r * r);
            let e = |(cx, cy, rx, ry): (f32, f32, f32, f32)| ((fx - cx) / rx).powi(2) + ((fy - cy) / ry).powi(2);
            if e(SUMP) <= 1.0 || e(VOID) <= 1.0 {
                open = true;
            }
            let edge = x < 2 || x >= W as i32 - 2 || y >= H as i32 - 2;
            let cell = if open && !edge {
                Cell::EMPTY
            } else if edge {
                Cell::new(material::BEDROCK, 0)
            } else {
                // The seam under the sump: a curved band four cells thick.
                let s = e((122.0, 116.0, 25.0, 11.5));
                let under_sump = (0.76..1.0).contains(&s) && fy > 117.0;
                // The tempting one under the low face.
                let by_face = ((fx - 172.0) / 8.0).powi(2) + ((fy - 125.0) / 3.0).powi(2) <= 1.0;
                let shade = (coin(seed, 7, idx(x, y)) * 4.0) as u8;
                if under_sump || by_face {
                    ore.push(idx(x, y));
                    Cell::new(ore_material, shade)
                } else {
                    Cell::new(rock, shade)
                }
            };
            world.set(x, y, cell);
        }
    }
    structural::compute_world_distances(world);

    // The leak: a crack in the sump floor, its position jittered by the seed.
    let jitter = (coin(seed, 1, 0) * 12.0) as i32 - 6;
    let leak_x = 120 + jitter;
    let floor = (0..H as i32).rev().find(|&y| world.get(leak_x, y).is_empty()).unwrap_or(120);
    let mut leak = Vec::new();
    for dx in -1..=1 {
        let x = leak_x + dx;
        let y = (0..H as i32).rev().find(|&y| world.get(x, y).is_empty()).unwrap_or(floor);
        leak.push(idx(x, y));
    }
    let vents = (0..3).flat_map(|y| (14..=24).map(move |x| idx(x, y))).collect();
    let air = (14..=24).map(|x| idx(x, 4)).collect();
    let leak_rate = 0.36 * (0.9 + 0.2 * coin(seed, 2, 0));
    // Surges: three to five through the three minutes.
    let n = 3 + (coin(seed, 3, 0) * 3.0) as usize;
    let surges = (0..n).map(|k| 600 + (k as u64) * 2000 + (coin(seed, 4, k) * 1200.0) as u64).collect();
    Level {
        lamps: [(188, 109), (235, 55)],
        leak,
        leak_at: (leak_x, floor + 1),
        vents,
        air,
        ore,
        ore_material,
        leak_rate,
        surges,
    }
}

pub fn new_world() -> World {
    let mut world = World::new(Rect::new(0, 0, W as i32 - 1, H as i32 - 1));
    world.weather_override = Some(crate::sim::weather::Weather::CLEAR);
    world
}
