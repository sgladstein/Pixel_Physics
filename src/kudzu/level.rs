//! The levels: ground, stone and where the vine starts.

use crate::sim::cell::Cell;
use crate::sim::material::{self, BEDROCK, STONE};
use crate::sim::world::World;

use super::{HEIGHT, WIDTH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// The brief's first level: a strip of garden soil, two stone walls and
    /// one crown between them.
    Walls,
}

impl Level {
    pub fn label(self) -> &'static str {
        match self {
            Level::Walls => "TWO WALLS",
        }
    }
}

/// What a level built, for the game and the counters.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub ground_y: i32,
    /// Each wall as `(x0, y_top, x1)`, standing on the ground.
    pub walls: Vec<(i32, i32, i32)>,
    /// The cells that count as a wall face: the two columns either side of
    /// every wall, from its top to the ground, plus the two rows over its top.
    pub faces: Vec<(i32, i32)>,
    /// Where each crown was sown.
    pub crowns: Vec<(i32, i32)>,
    /// Frames the vine grows before play starts.
    pub overnight: u64,
    /// Dig-outs in hand.
    pub digs: u32,
    /// Frames of play until dawn, when the level is scored.
    pub length: u64,
    /// Whether the game keeps the soil at field capacity (`Kudzu::water_all`).
    pub watered: bool,
}

pub fn build(level: Level, world: &mut World) -> Layout {
    match level {
        Level::Walls => walls(world),
    }
}

/// Garden soil from `ground_y` down, damp, over bedrock.
fn lay_ground(world: &mut World, ground_y: i32) {
    let soil = world.materials.id_of("soil").expect("soil is compiled in");
    for x in 0..WIDTH {
        for y in ground_y..HEIGHT - 1 {
            let byte = (crate::sim::rng::jitter(x, y) * 255.0) as u8;
            world.set(x, y, Cell::new(soil, byte).with_aux(material::SOIL_FIELD_CAPACITY));
        }
        world.set(x, HEIGHT - 1, Cell::new(BEDROCK, 0));
    }
}

/// A stone wall from `top` down into the ground, so it stands on rock.
fn wall(world: &mut World, layout: &mut Layout, x0: i32, x1: i32, top: i32) {
    for x in x0..=x1 {
        for y in top..HEIGHT - 1 {
            world.set(x, y, Cell::new(STONE, 0));
        }
    }
    layout.walls.push((x0, top, x1));
    for y in top..layout.ground_y {
        for x in [x0 - 2, x0 - 1, x1 + 1, x1 + 2] {
            layout.faces.push((x, y));
        }
    }
    for x in x0..=x1 {
        layout.faces.push((x, top - 1));
        layout.faces.push((x, top - 2));
    }
}

fn walls(world: &mut World) -> Layout {
    let ground_y = 128;
    let mut layout = Layout { ground_y, overnight: super::env_or("KUDZU_OVERNIGHT", 2_400), digs: 1, length: 3 * 60 * 60, watered: true, ..Layout::default() };
    lay_ground(world, ground_y);
    wall(world, &mut layout, 70, 85, 50);
    wall(world, &mut layout, 170, 185, 40);
    // **In a crack at the left wall's foot**, as Crown B in the village
    // brief is. Sown in the middle of the box, the seedling stood out of
    // reach of either wall, grew a bush to its turgor height and stopped
    // there for good on two seeds of four (120-170 cells all game).
    let crown = (super::env_or("KUDZU_CROWN_X", 87), ground_y - 1);
    sow(world, crown);
    layout.crowns.push(crown);
    layout
}

/// Carbon a crown starts with, as `plant::endow_seed` gives it.
pub const ROOTSTOCK: f32 = 4.0;

/// **Sow a crown with a rootstock.** A bare planted seed starts on no
/// carbon, and at six times the tick its first root tip ran out before the
/// first leaf paid for anything and retired for good: two root cells for a
/// 600-cell vine, water status 0.04, dead inside 1,800 frames (seed 1; seeds
/// 2 and 3 stalled at 100-170 cells). A kudzu crown is a tuber; this is it.
fn sow(world: &mut World, (x, y): (i32, i32)) {
    world.plant_tree_species(x, y, "kudzu");
    let id = world.get(x, y).organism_id();
    let carbon = super::env_or("KUDZU_ENDOW", ROOTSTOCK);
    crate::sim::plant::endow_seed(world, id, carbon);
}
