//! **The bed.** What a lab box is made of, and the one builder that makes it.
//!
//! `Reports/evolution-lab-design-guide-2026-08-30.md` Gate 1: *"There is no
//! such scene today"* — `filmstrip scene=colony` grows its plants out of
//! worldgen, `PlantScene` builds beds with no creatures, `creature_probe`
//! builds floors with no plants. This is the bed with both in it, and the
//! guide's reason for putting it here rather than in a harness is that **a
//! bed that is not the game's bed produces results that do not transfer**.
//! So the binary and the measuring harnesses must call *this*, never a
//! private copy of it.
//!
//! What the measurements already decided, and is therefore not a knob:
//!
//! - **A ceiling, not a sky.** Under a moving sun the field solves every tile
//!   in the world every frame; under a held light it does not (feasibility
//!   §2, §3b). This is simultaneously the fiction and the single largest
//!   performance decision, and it is free — a thing not to have.
//! - **Weather pinned clear.** Same reason. The air simulation still runs;
//!   what is removed is the thing that wakes every tile every frame.
//! - **Soil is required** — owner decision, 2026-08-30: *"We need soil.
//!   Plants grow roots into it and creatures need to dig into it and ideally
//!   create homes."* 40 → 240 rows costs **1.9x the frame**, so §2a's
//!   obligation is that something actually reaches the depth being paid for.
//! - **Partitions are the strongest single finding in the guide** (§2c):
//!   walling a fanned 2048-wide bed into 16 compartments took it from 4.1x to
//!   7.6x real time at a stand held to within 0.2%, and the same wall buys
//!   evolutionary isolation and a scoring move. One object, three payoffs.

use crate::sim::cell::Cell;
use crate::sim::field;
use crate::sim::material;
use crate::sim::weather::Pin;
use crate::sim::world::World;

/// A lab box, as data. `build` is the only way a lab world is made.
#[derive(Clone, Debug)]
pub struct LabBox {
    pub width: i32,
    pub height: i32,
    /// Rows of soil under the bed's surface.
    pub soil_depth: i32,
    /// The surface the founders are planted on, in world rows from the top.
    pub ground_y: i32,
    /// Sealed compartments, floor to ceiling. 1 is an open box.
    pub compartments: usize,
    /// Plant founders, spread across the bed.
    pub founders: usize,
    /// Which species the founders are. `herb` because it is the only shipped
    /// plant whose life cycle evolution can act on — generation 5 in 45,000
    /// frames, against `tree`'s generation 1 in 200,000.
    pub species: String,
    /// Ant colonies to found, spread across the bed.
    pub colonies: usize,
    pub seed: u64,
}

impl Default for LabBox {
    fn default() -> Self {
        Self {
            width: 512,
            height: 320,
            soil_depth: 80,
            ground_y: 160,
            compartments: 1,
            founders: 8,
            species: "herb".to_string(),
            colonies: 1,
            seed: 1,
        }
    }
}

/// Rows of stone under the soil, so the bed has a floor to sit on rather
/// than falling out of the world — the scene error `PlantScene` records
/// having paid for twice.
const FLOOR_ROWS: i32 = 8;
/// Thickness of the shell: floor edges, side walls and the ceiling.
const SHELL: i32 = 4;

impl LabBox {
    /// The frame the grow light is held at.
    ///
    /// **Measured, not assumed** — the sun's hump is a cosine over half the
    /// period and the phase belongs to `sky_light_amplitude`, so this picks
    /// the brightest frame rather than guessing a quarter or a half. Guessing
    /// wrong pins the light at midnight, which reads in a census as
    /// *"constant light shrinks the stand"*.
    pub fn noon() -> u64 {
        (0..field::DAY_NIGHT_PERIOD_FRAMES)
            .max_by(|a, b| field::sky_light_amplitude(*a).total_cmp(&field::sky_light_amplitude(*b)))
            .expect("the day has frames in it")
    }

    pub fn build(&self) -> World {
        let mut w = World::new(crate::sim::chunk::Rect::new(0, 0, self.width - 1, self.height - 1));
        w.seed = self.seed;
        let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");

        // Soil, then the stone floor under it.
        for x in 0..self.width {
            for y in self.ground_y..(self.ground_y + self.soil_depth) {
                w.set(x, y, Cell::new(soil, (crate::sim::rng::jitter(x, y) * 255.0) as u8)
                    .with_aux(material::SOIL_FIELD_CAPACITY));
            }
            let floor = self.ground_y + self.soil_depth;
            for y in floor..(floor + FLOOR_ROWS) {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }

        // The shell: side walls and a ceiling. The ceiling is the point — it
        // is what makes this a lab and not a field, and it is why the field
        // does not have to solve every tile every frame.
        let ceiling = (self.ground_y - self.height / 2).max(0);
        for x in 0..self.width {
            for y in ceiling..(ceiling + SHELL) {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for y in ceiling..(self.ground_y + self.soil_depth + FLOOR_ROWS) {
            for k in 0..SHELL {
                w.set(k, y, Cell::new(material::STONE, 0));
                w.set(self.width - 1 - k, y, Cell::new(material::STONE, 0));
            }
        }

        // Partitions, written after the bed so they cut through soil and air
        // alike — a real wall, of the same stone the floor is.
        if self.compartments > 1 {
            for k in 1..self.compartments {
                let x = (self.width * k as i32) / self.compartments as i32;
                for y in ceiling..(self.ground_y + self.soil_depth + FLOOR_ROWS) {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
        }

        // A grow light, not a sun, and calm air.
        w.set_sky_hold(Some(Self::noon()));
        w.set_weather_pin(Pin::Clear);

        let usable = self.width - 2 * SHELL;
        let spacing = usable / (self.founders as i32 + 1);
        for i in 0..self.founders {
            let x = SHELL + spacing * (i as i32 + 1);
            w.plant_tree_species(x, self.ground_y - 2, &self.species);
        }
        let colony_spacing = usable / (self.colonies as i32 + 1);
        for i in 0..self.colonies {
            let x = SHELL + colony_spacing * (i as i32 + 1);
            w.found_colony(x, self.ground_y - 2);
        }
        w
    }
}
