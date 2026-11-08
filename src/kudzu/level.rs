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
    /// **The village** (round 2, the owner's pick of 2026-11-08): five
    /// houses and a bell tower on a strip of garden, three crowns -- one in
    /// the garden, one at the tower's foot, one inside house 4 -- and six
    /// cuts for the whole night. Scored on how clear the house fronts stay
    /// *across the night*, so a vine left to grow costs while it grows.
    Village,
}

impl Level {
    pub fn label(self) -> &'static str {
        match self {
            Level::Walls => "TWO WALLS",
            Level::Village => "THE VILLAGE",
        }
    }

    /// `KUDZU_LEVEL=walls|village`; the village when unset.
    pub fn from_env() -> Level {
        match std::env::var("KUDZU_LEVEL").as_deref() {
            Ok("walls") => Level::Walls,
            _ => Level::Village,
        }
    }
}

/// One building the vine can swallow: its name and the cells that count as
/// its fronts (the same strips [`wall`] records into `Layout::faces`).
#[derive(Clone, Debug, Default)]
pub struct Building {
    pub name: &'static str,
    pub faces: Vec<(i32, i32)>,
}

/// A building is **lost** once this much of its fronts is under vine.
pub const LOST_AT: f32 = 0.5;

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
    /// Shears charges at the start of play.
    pub snips: u32,
    /// Frames to earn a charge back, up to `snips`; `None` for no recharge.
    pub recharge: Option<u32>,
    /// The buildings, for the houses-lost count. Empty on the walls level.
    pub buildings: Vec<Building>,
    /// Stars from the fronts' clear fraction averaged over the night
    /// (`Kudzu::night_clear`) rather than at dawn.
    pub night_scored: bool,
}

pub fn build(level: Level, world: &mut World) -> Layout {
    match level {
        Level::Walls => walls(world),
        Level::Village => village(world),
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
fn wall(world: &mut World, layout: &mut Layout, x0: i32, x1: i32, top: i32) -> Vec<(i32, i32)> {
    for x in x0..=x1 {
        for y in top..HEIGHT - 1 {
            world.set(x, y, Cell::new(STONE, 0));
        }
    }
    layout.walls.push((x0, top, x1));
    let mut faces = Vec::new();
    for y in top..layout.ground_y {
        for x in [x0 - 2, x0 - 1, x1 + 1, x1 + 2] {
            faces.push((x, y));
        }
    }
    for x in x0..=x1 {
        faces.push((x, top - 1));
        faces.push((x, top - 2));
    }
    layout.faces.extend(faces.iter().copied());
    faces
}

/// **A house with a room inside**: a stone shell `x0..=x1` from `top` to
/// the ground, two cells thick, hollow above a soil floor, with a doorway
/// in its right wall at ground level. The room is where a crown hides.
fn hollow_house(world: &mut World, layout: &mut Layout, x0: i32, x1: i32, top: i32) {
    wall(world, layout, x0, x1, top);
    let soil = world.materials.id_of("soil").expect("soil is compiled in");
    for x in x0 + 2..=x1 - 2 {
        for y in top + 2..layout.ground_y {
            world.set(x, y, Cell::EMPTY);
        }
        for y in layout.ground_y..HEIGHT - 1 {
            let byte = (crate::sim::rng::jitter(x, y) * 255.0) as u8;
            world.set(x, y, Cell::new(soil, byte).with_aux(material::SOIL_FIELD_CAPACITY));
        }
    }
    // The doorway, five cells high, through the right wall.
    for x in x1 - 1..=x1 {
        for y in layout.ground_y - 5..layout.ground_y {
            world.set(x, y, Cell::EMPTY);
        }
    }
}

fn walls(world: &mut World) -> Layout {
    let ground_y = 128;
    let mut layout = Layout { ground_y, overnight: super::env_or("KUDZU_OVERNIGHT", 2_400), digs: 1, length: 3 * 60 * 60, watered: true, ..Layout::default() };
    lay_ground(world, ground_y);
    wall(world, &mut layout, 70, 85, 50);
    wall(world, &mut layout, 170, 185, 40);
    layout.snips = super::SNIP_CHARGES;
    layout.recharge = Some(super::SNIP_RECHARGE);
    // **In a crack at the left wall's foot**, as Crown B in the village
    // brief is. Sown in the middle of the box, the seedling stood out of
    // reach of either wall, grew a bush to its turgor height and stopped
    // there for good on two seeds of four (120-170 cells all game).
    let crown = (super::env_or("KUDZU_CROWN_X", 87), ground_y - 1);
    sow(world, crown);
    layout.crowns.push(crown);
    layout
}

/// **The village**: five houses and a bell tower on one strip of garden,
/// three crowns. Positions are fixed so seeds pair; the seed moves only the
/// vine.
fn village(world: &mut World) -> Layout {
    let ground_y = 128;
    let mut layout = Layout {
        ground_y,
        overnight: super::env_or("KUDZU_OVERNIGHT", super::VILLAGE_OVERNIGHT),
        digs: 1,
        length: 3 * 60 * 60,
        watered: true,
        snips: super::env_or("KUDZU_SNIPS", super::VILLAGE_SNIPS),
        recharge: None,
        night_scored: true,
        ..Layout::default()
    };
    lay_ground(world, ground_y);
    // (name, x0, x1, top). House 4 is hollow: crown C grows in its room.
    let plan: [(&str, i32, i32, i32); 6] = [
        ("HOUSE 1", 8, 29, 108),
        ("HOUSE 2", 48, 69, 106),
        ("TOWER", 100, 109, 52),
        ("HOUSE 3", 132, 153, 110),
        ("HOUSE 4", 172, 195, 104),
        ("HOUSE 5", 222, 243, 108),
    ];
    let log = world.materials.id_of("log").expect("log is compiled in");
    for (name, x0, x1, top) in plan {
        if name == "HOUSE 4" {
            hollow_house(world, &mut layout, x0, x1, top);
        } else {
            wall(world, &mut layout, x0, x1, top);
        }
        // **A roof, so a house reads as a house**: timber stepped in a cell
        // a side per row, resting on the wall top (no eaves to hang). The
        // tower gets a short spire the same way.
        let mut k = 0;
        while x0 + k <= x1 - k {
            for x in x0 + k..=x1 - k {
                world.set(x, top - 1 - k, Cell::new(log, ((x * 7 + k) & 3) as u8));
            }
            k += 1;
        }
    }
    // **The fronts are recomputed from what was built**: every empty cell
    // above the ground within two cells of a building's stone or roof,
    // outside its room. `wall`'s strips assumed a flat top, and a roof
    // would have stood in them and counted as clear for ever.
    layout.faces.clear();
    for (name, x0, x1, top) in plan {
        let ridge = top - (x1 - x0) / 2 - 2;
        let mut faces = Vec::new();
        for y in (ridge - 3).max(0)..ground_y {
            for x in x0 - 2..=x1 + 2 {
                if world.get(x, y).material != crate::sim::material::EMPTY {
                    continue;
                }
                if (x0..=x1).contains(&x) && y > top {
                    continue; // house 4's room
                }
                let near = (-2..=2).any(|dy: i32| {
                    (-2..=2).any(|dx: i32| {
                        let (bx, by) = (x + dx, y + dy);
                        (x0..=x1).contains(&bx) && by < ground_y && by >= ridge - 1 && world.get(bx, by).material != crate::sim::material::EMPTY
                    })
                });
                if near {
                    faces.push((x, y));
                }
            }
        }
        layout.faces.extend(faces.iter().copied());
        layout.buildings.push(Building { name, faces });
    }
    // Crown A in the garden between houses 1 and 2; crown B in a crack at
    // the tower's foot; crown C on the floor of house 4's room.
    for crown in [(39, ground_y - 1), (111, ground_y - 1), (183, ground_y - 1)] {
        sow(world, crown);
        layout.crowns.push(crown);
    }
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
