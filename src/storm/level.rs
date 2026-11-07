//! **The valley** -- the one level the probe needs.
//!
//! From the brief: a cloud, a bell tower (the tallest point, which you
//! protect), one lamp in the open and one lamp under an overhang. A small
//! village of log huts sits between the two lamps, on dry grass, so a bolt
//! that lands short sets something you care about on fire.
//!
//! Every number here is a cell position in a 256x160 world. The world is
//! built from shipped materials only; the two the bolt makes -- glass and
//! scorch -- are `crystal` and `basalt` repainted in this game's own
//! material table (see [`super::Ids::repaint`]), so nothing is added to the
//! shared registry and no other game can see the change.

use crate::sim::cell::Cell;
use crate::sim::chunk::Rect;
use crate::sim::material;
use crate::sim::rng;
use crate::sim::world::World;

pub const W: i32 = 256;
pub const H: i32 = 160;

/// A box of cells, inclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Area {
    pub const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self { x0, y0, x1, y1 }
    }
    pub fn contains(&self, (x, y): (i32, i32)) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }
    pub fn centre(&self) -> (i32, i32) {
        ((self.x0 + self.x1) / 2, (self.y0 + self.y1) / 2)
    }
}

/// The valley floor's height at column `x` -- the first soil row.
pub fn ground_y(x: i32) -> i32 {
    match x {
        // The tower hill.
        ..=52 => 96,
        // Its slope down to the valley.
        53..=78 => 96 + ((x - 52) * 28) / 26,
        // The valley floor, with a little roll so it does not read as a ruler.
        79..=213 => 124 + ((x as f32 * 0.11).sin() * 1.4) as i32,
        // The cliff top.
        _ => 72,
    }
}

/// Where everything is.
#[derive(Clone, Debug)]
pub struct Level {
    pub tower: Area,
    /// The iron bell and spire at the top of the tower -- what the cloud
    /// strikes when it strikes by itself.
    pub bell: Area,
    pub lamps: [Lamp; 2],
    pub huts: Vec<Area>,
    pub ledge: Area,
    pub sand: Area,
}

#[derive(Clone, Copy, Debug)]
pub struct Lamp {
    /// Post and head together: a bolt or a current that reaches any of it
    /// lights the lamp.
    pub area: Area,
    pub head: Area,
    pub name: &'static str,
}

pub const SOIL_DEPTH: i32 = 16;
/// Dry-ish valley soil: below `SOIL_FIELD_CAPACITY`, so a splash of water
/// visibly raises how well it conducts.
pub const SOIL_MOISTURE: u16 = 260;

impl Level {
    pub fn valley() -> Self {
        let lamp = |x: i32, top: i32, name| {
            let g = ground_y(x);
            Lamp { area: Area::new(x - 2, top, x + 3, g + 3), head: Area::new(x - 2, top, x + 3, top + 3), name }
        };
        Level {
            tower: Area::new(26, 50, 38, 95),
            bell: Area::new(29, 34, 35, 49),
            lamps: [lamp(134, 106, "FIELD LAMP"), lamp(196, 112, "LAMP UNDER THE ROCK")],
            huts: vec![Area::new(142, 113, 151, 123), Area::new(155, 115, 162, 123)],
            ledge: Area::new(176, 99, 213, 104),
            sand: Area::new(84, 0, 114, 0),
        }
    }

    /// Build the world for this level.
    pub fn build(&self, ids: &super::Ids, seed: u64) -> World {
        let mut w = World::new(Rect::new(0, 0, W - 1, H - 1));
        w.seed = seed;
        for x in 0..W {
            let g = ground_y(x);
            for y in 0..H {
                let shade = (rng::jitter(x, y) * 255.0) as u8;
                let cell = if y >= H - 3 {
                    Cell::new(material::BEDROCK, 0)
                } else if x >= 214 && y >= g {
                    // The cliff is rock all the way down.
                    Cell::new(material::STONE, shade)
                } else if y >= g + SOIL_DEPTH {
                    Cell::new(material::STONE, shade)
                } else if y >= g {
                    let sandy = x >= self.sand.x0 && x <= self.sand.x1 && y < g + 7;
                    if sandy {
                        Cell::new(material::SAND, shade)
                    } else {
                        Cell::new(ids.soil, shade).with_aux(SOIL_MOISTURE)
                    }
                } else {
                    continue;
                };
                w.set(x, y, cell);
            }
        }
        // Dry grass on the valley floor, one row: what lets a short bolt
        // become a grass fire that walks into the village.
        for x in 140..166 {
            let g = ground_y(x);
            w.set(x, g - 1, Cell::new(ids.litter, (rng::jitter(x, 7) * 255.0) as u8));
        }
        // The ledge, the rock the second lamp hides under. Joined to the cliff.
        for y in self.ledge.y0..=self.ledge.y1 {
            // A ragged lip, thicker at the wall.
            let lip = self.ledge.x0 + ((self.ledge.y1 - y) as f32 * 0.8) as i32;
            for x in lip..=self.ledge.x1 {
                w.set(x, y, Cell::new(material::STONE, (rng::jitter(x, y) * 255.0) as u8));
            }
        }
        // The tower: stone, with an iron bell and spire on top.
        for y in self.tower.y0..=self.tower.y1 {
            for x in self.tower.x0..=self.tower.x1 {
                // Battlements on the top row.
                if y == self.tower.y0 && (x - self.tower.x0) % 3 == 1 {
                    continue;
                }
                // A window slit.
                if (60..68).contains(&y) && (x == 32 || x == 33) {
                    continue;
                }
                w.set(x, y, Cell::new(material::STONE, (rng::jitter(x, y) * 255.0) as u8));
            }
        }
        for y in self.bell.y0..=self.bell.y1 {
            for x in self.bell.x0..=self.bell.x1 {
                let inside = if y < 42 { x == 32 || x == 33 } else { true };
                if inside {
                    w.set(x, y, Cell::new(ids.iron, 0));
                }
            }
        }
        // The lamps: an iron post two wide, sunk three cells, under a head.
        for lamp in &self.lamps {
            let (cx, _) = lamp.area.centre();
            for y in lamp.head.y1 + 1..=lamp.area.y1 {
                w.set(cx, y, Cell::new(ids.iron, 0));
                w.set(cx + 1, y, Cell::new(ids.iron, 0));
            }
            for y in lamp.head.y0..=lamp.head.y1 {
                for x in lamp.head.x0..=lamp.head.x1 {
                    w.set(x, y, Cell::new(ids.lamp_dark, 0));
                }
            }
        }
        // Huts: solid log cabins. The first version was hollow, and its
        // roof was a log span wider than log may hang unsupported, so the
        // roofs fell in on their own with no bolt anywhere near: 21 hut cells
        // gone by the end of a storm with no fire at all.
        for hut in &self.huts {
            for y in hut.y0..=hut.y1 {
                for x in hut.x0..=hut.x1 {
                    // A pitched roof: the top rows step in.
                    let inset = (hut.y0 + 3 - y).max(0);
                    if x < hut.x0 + inset || x > hut.x1 - inset {
                        continue;
                    }
                    w.set(x, y, Cell::new(ids.log, (rng::jitter(x, y) * 255.0) as u8));
                }
            }
        }
        w
    }
}
