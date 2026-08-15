//! The one scene the plant harnesses share.
//!
//! **This exists because two harnesses were compared as if they were the
//! same scene and were not.** `plant_probe` and `filmstrip`'s `grove` each
//! built their own, differing in tree count (`trees=N` against a hard-coded
//! 3), spacing (56 cells against 140 at eight trees), seed placement
//! (resting on the surface against dropped from 25 rows up) and soil depth
//! (30 rows against 34). Numbers were quoted from one and pictures shot
//! from the other, and a commit tuning *stand spacing* (crown shyness) was
//! measured at one density and eyeballed at another.
//!
//! `Reports/tree-architecture-implementation-plan.md` Phase 0a. Anything
//! that measures plants builds its world from here.

use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material;
use pixel_physics::sim::rng;
use pixel_physics::sim::world::World;

/// Rows of soil above the stone floor — deep enough for a real root system
/// to spread without hitting rock, which is what `MAX_ROOT_FRACTION` and
/// the allometry work are calibrated against.
pub const SOIL_DEPTH: i32 = 34;
/// Rows of stone under the soil, so the bed has a floor and does not fall
/// out of the world. A soil column with nothing beneath it slumps, which
/// reads exactly like "the mechanism does nothing" — recorded in
/// `CLAUDE.md` as a scene error that cost real time, twice.
pub const STONE_DEPTH: i32 = 6;
/// How far above the ground a seed is released. Seeds are `Powder` and
/// fall, so this exercises the fall and landing rather than pre-placing
/// each one — and it is the path that found the `ChunkView` cell-list bug.
pub const SEED_DROP: i32 = 25;

/// A world sized so the *plant* is the limit rather than the ceiling.
///
/// **The ceiling has bitten three times**: `forest` at 40 rows of sky,
/// `grove` at 96, and `ground=200` — where median shoot height still
/// measured 203 in a 200-row sky, with five of eight trees pinned against
/// the boundary. Every conclusion drawn in a ceiling-bound scene is
/// contaminated, so the harness now reports `canopy_top` and a run whose
/// canopy reaches row 0 should be discarded rather than interpreted.
///
/// Empty sky is close to free — chunks with nothing in them sleep, and the
/// settled worst frame in `examples/ascii` reads 0.0002 ms — so headroom
/// is bought cheaply.
///
/// # There is currently no depth that is both well-lit and un-ceilinged
///
/// Measured, 8 trees / 30,000 frames, varying only `ground_y`:
///
/// | ground | cells | clearance at end |
/// |---|---|---|
/// | 200 | 8,529 | **3 rows** |
/// | 250 | 179 | 196 rows |
/// | 300 | 62 | 295 rows |
/// | 400 | 8 (nothing germinated) | 399 rows |
///
/// A cliff, not a curve. Below ~220 rows of depth there is enough light for
/// vigorous growth and the tree fills the sky; past it there is not enough
/// light to grow at all.
///
/// **The cause is structural, and no choice of this constant fixes it.**
/// `field.rs` seeds light on the topmost chunk's top row and diffuses it
/// downward, so **light gets brighter as a tree climbs** — an unbounded
/// incentive to grow toward the world's top edge, which is exactly where
/// every scene has ended up pinned. Real sunlight is uniform above the
/// canopy; the gradient belongs *under* occluders, not in open air.
///
/// So `Reports/tree-architecture-implementation-plan.md` Phase 0b cannot be
/// completed by picking a number here. It is blocked on the light model,
/// and until that lands **every shape conclusion carries a ceiling caveat**
/// — check `canopy_top` in the output and discard runs that reach row 0.
pub struct PlantScene {
    pub width: i32,
    pub height: i32,
    pub ground_y: i32,
    pub trees: usize,
}

impl Default for PlantScene {
    fn default() -> Self {
        // 200 is the best available compromise and is **not** a good one --
        // see `PlantScene`'s doc. It leaves about 3 rows of clearance.
        Self { width: 512, height: 320, ground_y: 200, trees: 8 }
    }
}

impl PlantScene {
    pub fn build(&self) -> World {
        let mut w = World::new(Rect::new(0, 0, self.width - 1, self.height - 1));
        let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
        for x in 0..self.width {
            for y in (self.ground_y + SOIL_DEPTH)..(self.ground_y + SOIL_DEPTH + STONE_DEPTH) {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
            for y in self.ground_y..(self.ground_y + SOIL_DEPTH) {
                w.set(x, y, Cell::new(soil, (rng::jitter(x, y) * 255.0) as u8).with_aux(material::SOIL_FIELD_CAPACITY));
            }
        }
        // Evenly spaced across the world so spacing is a function of tree
        // count and nothing else — the property the old pair of scenes
        // lacked, and the reason a spacing mechanism could be tuned at one
        // density and judged at another.
        let spacing = self.width / (self.trees as i32 + 1);
        for i in 0..self.trees {
            let x = spacing * (i as i32 + 1);
            w.plant_tree(x, self.ground_y - SEED_DROP);
        }
        w
    }
}

/// The highest row any organism cell occupies, or `None` if nothing grew.
///
/// **The ceiling detector.** If this reaches 0 the run hit the top of the
/// world and every shape number from it is void — trees that cannot go up
/// spread sideways, which is the "canopies merge into a slab" symptom that
/// two sessions were spent chasing as a plant bug.
pub fn canopy_top(w: &World) -> Option<i32> {
    let b = w.bounds()?;
    (b.min_y..=b.max_y).find(|&y| (b.min_x..=b.max_x).any(|x| w.get(x, y).organism_id() != 0))
}
