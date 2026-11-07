//! The tension field: a diffusing number over the ice that says where the
//! wall wants to break.
//!
//! Two things feed it. **The lake**, on every ice cell that touches water,
//! in proportion to how far under the surface it is -- so it rises as the
//! lake rises, and the deep upstream face is where the wall is most
//! strained. **The weight of ice above**, a smaller term that grows toward
//! the wall's foot. It spreads through the ice by a screened relaxation and
//! **does not cross a crack**: ice cut off from the lake by a crack is
//! relieved, which is what makes a crack *change where the next one goes*.
//!
//! It is new and lives only in this game. Nothing in `src/sim` reads it;
//! the shipped load model decides what actually fails.

use crate::sim::material::{self, MaterialId};
use crate::sim::world::World;

use super::level::{self, Level, H, W};
use super::tips::Cracks;

/// How strongly a cell of ice is pushed per row of water above it.
const LAKE_GAIN: f32 = 1.0 / 25.0;
/// How strongly a cell is loaded per cell of ice stacked on it.
const WEIGHT_GAIN: f32 = 1.0 / 60000.0;
/// Screening: how fast tension fades with distance from where it is fed.
/// About `1 / sqrt(SCREEN)` = 12 cells, a third of the wall's thickness, so
/// the upstream face is visibly hotter than the downstream one.
const SCREEN: f32 = 0.007;
/// Gauss-Seidel sweeps a frame. The field changes slowly (the lake rises a
/// row every second and a half), so a few sweeps keep it current; a crack's
/// shadow takes a second or so to settle, which reads as the ice relaxing.
const SWEEPS_PER_FRAME: usize = 3;
/// Sweeps at the start, so the first tap meets a settled field.
const SWEEPS_AT_START: usize = 600;

#[derive(Clone)]
pub struct Field {
    pub t: Vec<f32>,
    source: Vec<f32>,
    ice: Vec<bool>,
    started: bool,
}

#[inline]
fn idx(x: i32, y: i32) -> usize {
    (y * W + x) as usize
}

impl Field {
    pub fn new() -> Self {
        let n = (W * H) as usize;
        Field { t: vec![0.0; n], source: vec![0.0; n], ice: vec![false; n], started: false }
    }

    pub fn at(&self, x: i32, y: i32) -> f32 {
        if x < 0 || y < 0 || x >= W || y >= H {
            return 0.0;
        }
        self.t[idx(x, y)]
    }

    /// Tension across an edge: the mean of the two cells it separates.
    pub fn across(&self, a: (i32, i32), b: (i32, i32)) -> f32 {
        0.5 * (self.at(a.0, a.1) + self.at(b.0, b.1))
    }

    pub fn update(&mut self, world: &World, ice: MaterialId, level: &Level, _cracks: &Cracks) {
        // Sources, recomputed every frame: cheap over 256x160, and the lake
        // and the wall both move.
        let surface = level.surface;
        for x in 0..W {
            let mut above = 0u32;
            for y in 0..H {
                let i = idx(x, y);
                let is_ice = world.get(x, y).material == ice;
                self.ice[i] = is_ice;
                if !is_ice {
                    self.source[i] = 0.0;
                    if world.get(x, y).material == material::EMPTY {
                        above = 0;
                    }
                    continue;
                }
                above += 1;
                let mut s = WEIGHT_GAIN * above as f32;
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    if world.get(x + dx, y + dy).material == material::WATER && x <= level::WALL_X1 {
                        s += LAKE_GAIN * (y - surface).max(0) as f32;
                    }
                }
                self.source[i] = s;
            }
        }
        let sweeps = if self.started { SWEEPS_PER_FRAME } else { SWEEPS_AT_START };
        self.started = true;
        for _ in 0..sweeps {
            self.sweep(world);
        }
    }

    fn sweep(&mut self, world: &World) {
        for parity in 0..2 {
            for y in 0..H {
                for x in 0..W {
                    if (x + y) & 1 != parity {
                        continue;
                    }
                    let i = idx(x, y);
                    if !self.ice[i] {
                        self.t[i] = 0.0;
                        continue;
                    }
                    let mut sum = self.source[i];
                    let mut n = 0.0;
                    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || ny < 0 || nx >= W || ny >= H || !self.ice[idx(nx, ny)] {
                            continue;
                        }
                        if crate::sim::structural::edge_is_cracked(world, x, y, dx, dy) {
                            continue;
                        }
                        sum += self.t[idx(nx, ny)];
                        n += 1.0;
                    }
                    self.t[i] = sum / (n + SCREEN);
                }
            }
        }
    }

    /// The largest tension anywhere on the ice, for normalising the shimmer.
    pub fn max(&self) -> f32 {
        self.t.iter().copied().fold(0.0, f32::max)
    }
}

impl Default for Field {
    fn default() -> Self {
        Self::new()
    }
}
