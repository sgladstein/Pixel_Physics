//! "The Mill", scaled to the phone-legible 256x160 the probe brief fixes.
//!
//! ```text
//!   melt stream
//!   v
//!   ~~~~~~~~~~~~~~~ |######|
//!   ~~~ lake ~~~~~~ |# ice#|                     __mill__
//!   ~~~~~~~~~~~~~~~ |# wall|                     |      |   -> river
//!   ===================== bedrock =================================
//! ```
//!
//! The lake rises on a clock from a melt stream at the left edge. If it
//! reaches the wall's original top the wall bursts, which is the worst
//! outcome. Water that gets past the wall runs down the valley, past the
//! mill, and leaves at the right edge -- that leaving water is the
//! spillway flow the score reads.

use crate::sim::cell::Cell;
use crate::sim::chunk::Rect;
use crate::sim::material;
use crate::sim::structural;
use crate::sim::world::World;

use super::Counts;

pub const W: i32 = 256;
pub const H: i32 = 160;
/// Top of the bedrock floor.
pub const FLOOR_Y: i32 = 150;
/// The ice wall: columns `WALL_X0..=WALL_X1`, rows `WALL_TOP..FLOOR_Y`.
pub const WALL_X0: i32 = 96;
pub const WALL_X1: i32 = 131;
pub const WALL_TOP: i32 = 40;
/// Where the lake's surface starts.
pub const LAKE_START: i32 = 100;
/// The mill's left wall.
pub const MILL_X0: i32 = 200;
/// The gorge: columns `GORGE_X0..GORGE_X1`, cut down through the floor.
/// 36 wide since the 2026-11-08 playtest: at 16 it was dammed by the debris
/// of every spillway that drained half the lake, and the flood went to the
/// mill on 4 runs of 4 -- a staged spillway could not be mill-safe at all.
pub const GORGE_X0: i32 = 140;
pub const GORGE_X1: i32 = 176;
/// The terrace the mill stands on: rock from `TERRACE_X0` to the right
/// edge, its top at `TERRACE_Y`. A spillway's flood runs past its foot into
/// the gorge; only a wall breaking low, with the whole lake behind it, comes
/// up over it. Added with the gorge widening, same playtest.
pub const TERRACE_X0: i32 = 184;
pub const TERRACE_Y: i32 = 136;
/// The safe line: the lake must never stand above this row. Fifteen rows
/// under the crest, painted on the ice. The playtest's proposal: what a
/// spillway is *for* is holding the lake down, and the old grading (share
/// drained) scored a held lake as a failure.
pub const SAFE_LINE: i32 = WALL_TOP + 15;
/// The mill race: the lake must end the thaw at or above this row, because
/// the mill runs on it. Without it the cheapest way to keep the lake under
/// the safe line was to empty it: a deep cut early, while the lake was
/// still low, drained it to the floor of the cut for good, and tapping the
/// wall's foot under water scored best of six strategies (24 stars of 30,
/// to a careful notch's 18; 2026-11-08). What the game asks is a spillway
/// that *holds* the lake between the lines.
pub const MILL_RACE: i32 = 80;
/// Top of the mill's walls.
const MILL_TOP: i32 = TERRACE_Y - 22;
/// Anything at or below this row in the gorge is carried away.
pub const DRAIN_Y: i32 = H - 4;
/// Melt stream, full water cells per second. Sized so that with nothing
/// done the lake reaches the wall's top at about 90 s of a 120 s thaw:
/// 60 rows x 96 columns over 90 s.
pub const INFLOW_PER_SEC: f32 = 64.0;
/// What the river below the gorge carries away, full water cells a second:
/// about three times the melt stream. See `Level::drain`.
pub const RIVER_PER_SEC: f32 = 200.0;

#[derive(Clone)]
pub struct Level {
    /// The mill's box, `(x0, y0, x1, y1)`.
    pub mill: (i32, i32, i32, i32),
    /// Lake surface row, smoothed: the highest row in the lake's middle
    /// that is mostly water.
    pub surface: i32,
    inflow_acc: f32,
    pub burst_started: bool,
    /// Cells of mill standing at the start, to count losses against.
    mill_cells: usize,
}

pub fn build(seed: u64) -> World {
    let mut world = World::new(Rect::new(0, 0, W - 1, H - 1));
    world.seed = seed;
    let ice = world.materials.id_of("ice").expect("ice");
    let log = world.materials.id_of("log").expect("log");
    for y in FLOOR_Y..H {
        for x in 0..W {
            world.set(x, y, Cell::new(material::BEDROCK, 0));
        }
    }
    // The gorge the river leaves by: a slot in the floor between the wall
    // and the mill. Whatever reaches its bottom is carried off the screen
    // (`drain`). A spillway that runs harder than the gorge can take
    // overflows it and reaches the mill.
    for y in FLOOR_Y..H - 1 {
        for x in GORGE_X0..GORGE_X1 {
            world.set(x, y, Cell::EMPTY);
        }
    }
    // The terrace, before the sky is frozen: it is ground.
    for y in TERRACE_Y..FLOOR_Y {
        for x in TERRACE_X0..W {
            world.set(x, y, Cell::new(material::BEDROCK, 0));
        }
    }
    // **Fix "underground" before the wall goes up.** The renderer draws
    // everything below each column's first solid cell -- recorded once, at
    // the first step, and never revised -- as the inside of a cave. With the
    // wall in place at that moment, every notch cut in it showed a black
    // cave wall behind it instead of sky. Recorded now, the ground is the
    // valley floor and the wall stands in the open, which it does.
    world.freeze_sky_surface();
    world.freeze_underground_map();
    let shades = world.materials.get(ice).palette.len().max(1) as u32;
    for y in WALL_TOP..FLOOR_Y {
        for x in WALL_X0..=WALL_X1 {
            // A slight batter on the downstream face: real ice walls are
            // not square, and a sloped face lets a slab slide off it.
            if x > WALL_X1 - 6 + (y - WALL_TOP) / 18 {
                continue;
            }
            let shade = (crate::sim::rng::jitter(x, y) * shades as f32) as u8 % shades as u8;
            world.set(x, y, Cell::new(ice, shade));
        }
    }
    for y in LAKE_START..FLOOR_Y {
        for x in 0..WALL_X0 {
            world.set(x, y, Cell::new(material::WATER, 0).with_aux(material::LIQUID_FULL));
        }
    }
    // The mill: two stone walls, a log roof, on the terrace.
    let (mx0, mx1, my0) = (MILL_X0, MILL_X0 + 32, MILL_TOP);
    for y in my0..TERRACE_Y {
        for x in [mx0, mx0 + 1, mx1 - 1, mx1] {
            world.set(x, y, Cell::new(material::STONE, 1));
        }
    }
    for y in (my0 - 3)..my0 {
        for x in (mx0 - 2)..=(mx1 + 2) {
            world.set(x, y, Cell::new(log, 0));
        }
    }
    structural::compute_world_distances(&mut world);
    // Held at noon. The probe's clock is the lake, and a sun that sets
    // twenty seconds in -- the shipped day is short -- turns the second half
    // of every run into a night scene nobody can read.
    world.clock.sky_hold = crate::sim::clock::SkyPin::Noon.hold();
    world
}

impl Level {
    pub fn new(world: &World) -> Self {
        let mill = (MILL_X0 - 2, MILL_TOP - 6, MILL_X0 + 34, TERRACE_Y - 1);
        let mut l = Level { mill, surface: LAKE_START, inflow_acc: 0.0, burst_started: false, mill_cells: 0 };
        l.mill_cells = l.mill_standing(world);
        l
    }

    pub fn mill_standing(&self, world: &World) -> usize {
        let (x0, y0, x1, y1) = self.mill;
        let mut n = 0;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let m = world.get(x, y).material;
                if m == material::STONE || world.materials.get(m).name == "log" {
                    n += 1;
                }
            }
        }
        n
    }

    pub fn mill_lost(&self, world: &World) -> usize {
        self.mill_cells.saturating_sub(self.mill_standing(world))
    }

    /// The melt stream, and the burst check.
    pub fn rise(&mut self, world: &mut World, counts: &mut Counts) {
        self.inflow_acc += INFLOW_PER_SEC / super::FPS as f32;
        while self.inflow_acc >= 1.0 {
            self.inflow_acc -= 1.0;
            // A thin fall down the left edge.
            let x = 1 + (world.frame % 3) as i32;
            for y in 2..H {
                if world.get(x, y).material == material::EMPTY && world.get(x, y + 1).material != material::EMPTY {
                    world.set(x, y, Cell::new(material::WATER, 0).with_aux(material::LIQUID_FULL));
                    break;
                }
                if world.get(x, y).material != material::EMPTY {
                    // Top of the column is water: add on top of it.
                    if y > 2 && world.get(x, y - 1).material == material::EMPTY {
                        world.set(x, y - 1, Cell::new(material::WATER, 0).with_aux(material::LIQUID_FULL));
                    }
                    break;
                }
            }
        }
        if world.frame.is_multiple_of(10) {
            self.surface = self.measure_surface(world);
        }
        if self.surface <= WALL_TOP + 1 {
            counts.burst = true;
        }
    }

    fn measure_surface(&self, world: &World) -> i32 {
        for y in 0..FLOOR_Y {
            let mut wet = 0;
            for x in 20..80 {
                if world.get(x, y).material == material::WATER {
                    wet += 1;
                }
            }
            if wet > 40 {
                return y;
            }
        }
        FLOOR_Y
    }

    /// Water still behind the wall, in fill units.
    pub fn lake_volume(&self, world: &World) -> u64 {
        let mut v = 0u64;
        for y in 0..FLOOR_Y {
            for x in 0..WALL_X0 {
                let c = world.get(x, y);
                if c.material == material::WATER {
                    v += c.aux() as u64;
                }
            }
        }
        v
    }

    /// Water leaving down the gorge, in fill units, removed -- and any ice
    /// or slush that has come to rest in its bottom, which the river carries
    /// off too (a slab that plugged the gorge for good would flood the mill
    /// from a spillway that was the right size).
    ///
    /// **The river carries `RIVER_PER_SEC` and no more.** Water past that
    /// stays, backs up the gorge and over the valley floor, and a flood big
    /// enough comes up over the terrace into the mill. A spillway the size
    /// of the melt stream drains for ever; a wall split low dumps the lake
    /// faster than the river can take it. Without the cap the gorge took
    /// anything, and once the mill was on its terrace a random deep cut that
    /// emptied the lake was the safest play there was (random 14 stars of
    /// 30 to the plan's 8, 2026-11-08).
    pub fn drain(&self, world: &mut World) -> u32 {
        let mut out = 0u32;
        let budget = (RIVER_PER_SEC / super::FPS as f32 * material::LIQUID_FULL as f32) as u32;
        // Bottom row first, then up; columns alternate direction by frame
        // so neither side of the gorge is favoured.
        for y in (DRAIN_Y..H - 1).rev() {
            for i in 0..(GORGE_X1 - GORGE_X0) {
                let x = if world.frame.is_multiple_of(2) { GORGE_X0 + i } else { GORGE_X1 - 1 - i };
                let c = world.get(x, y);
                if c.material == material::EMPTY || c.material == material::BEDROCK {
                    continue;
                }
                if world.materials.kind(c.material) == material::MaterialKind::Liquid {
                    if out + c.aux() as u32 > budget {
                        continue;
                    }
                    out += c.aux() as u32;
                }
                world.set(x, y, Cell::EMPTY);
            }
        }
        out
    }

    /// Water standing inside the mill, in full cells: the flood reached it.
    pub fn mill_water(&self, world: &World) -> u32 {
        let (x0, y0, x1, y1) = self.mill;
        let mut n = 0u32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let c = world.get(x, y);
                if c.material == material::WATER {
                    n += c.aux() as u32;
                }
            }
        }
        n / material::LIQUID_FULL as u32
    }

    /// The lake's push on what has broken free: a slab with water at its
    /// back is shoved downstream, harder the deeper it sits. This is the
    /// one force the shipped engine does not have -- water does not press on
    /// solids -- and without it a slab cut from the wall drops back into the
    /// notch it came from and plugs it.
    pub fn push_bodies(&self, world: &mut World) {
        let mut pushes = Vec::new();
        for (i, b) in world.chunk_bodies.iter().enumerate() {
            let (bx, by) = (b.x.round() as i32, b.y.round() as i32);
            // Only in the gap it came out of. Pushed all the way down the
            // valley, every block a spillway shed rode the flood onto the
            // terrace and hit the mill (seen 2026-11-08, wall 1: 8 hits from
            // a notch that held the lake 17 rows under the safe line).
            if bx > WALL_X1 + 4 {
                continue;
            }
            let mut wet = 0;
            for c in &b.cells {
                let (x, y) = (bx + c.dx, by + c.dy);
                if world.get(x - 1, y).material == material::WATER {
                    wet += (y - self.surface).max(1);
                }
            }
            if wet > 0 {
                pushes.push((i, wet as f32 / b.cells.len().max(1) as f32));
            }
        }
        for (i, p) in pushes {
            let b = &mut world.chunk_bodies[i];
            b.vx = (b.vx + 0.02 * p.min(20.0)).min(2.5);
        }
    }
}
