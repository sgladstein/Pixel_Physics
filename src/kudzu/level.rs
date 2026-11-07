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
    let mut layout = Layout { ground_y, overnight: 2_400, digs: 1, length: 3 * 60 * 60, watered: true, ..Layout::default() };
    lay_ground(world, ground_y);
    wall(world, &mut layout, 70, 85, 50);
    wall(world, &mut layout, 170, 185, 40);
    let crown = (128, ground_y - 1);
    world.plant_tree_species(crown.0, crown.1, "kudzu");
    layout.crowns.push(crown);
    layout
}
