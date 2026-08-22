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

// Shared by several harness binaries, and Rust compiles each example as
// its own crate -- so anything only one of them uses reads as dead here.
#![allow(dead_code)]

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
/// # The depth cliff is fixed, and the fix was not a number here
///
/// This scene used to carry a table showing that **no** ground depth was
/// both well-lit and un-ceilinged — 8,529 cells with 3 rows of clearance
/// at `ground=200`, against 8 cells and nothing germinating at 400. A
/// cliff, not a curve.
///
/// The cause was the light model, not this constant: `field.rs` seeded
/// light on the topmost chunk's top row and diffused it downward, so light
/// got *brighter* as a tree climbed — an unbounded reward for growing into
/// the world's top edge, which is where every scene ended up pinned.
/// `apply_sky`'s per-column cast (`Reports/tree-architecture-
/// implementation-plan.md` §0f) removed it: open air reads the same at any
/// depth, and shade comes from what is in the way.
///
/// What bounds height now is the plant's own turgor budget, which is a
/// *derived* ceiling around 120 rows — so `ground_y: 200` leaves roughly 78
/// rows of clearance and the tree, not the scene, is the limit.
///
/// **`canopy_top` is still worth checking on every run.** A ceiling would
/// still void a shape conclusion; it is simply no longer the expected
/// outcome. A run whose canopy reaches row 0 should be discarded, not
/// interpreted.
pub struct PlantScene {
    pub width: i32,
    pub height: i32,
    pub ground_y: i32,
    pub trees: usize,
    /// Which species the stand plants -- Phase 2 grew real architectural
    /// alternatives (`conifer`, `shrub`), and a harness that can only grow
    /// `tree` cannot show them.
    pub species: String,
    /// **How wet the bed starts**, on `SOIL_SATURATED`'s scale.
    ///
    /// A parameter for the same reason `soil_depth` is one, and the file's
    /// own history says why: a comparison that cannot be expressed cannot
    /// be run. The two settings that matter are already named --
    /// `Self::default()` is ideal growing ground at field capacity, and
    /// `Self::dormant()` is below the wilting point, where
    /// `plant_available_fraction` is exactly zero and a seed waits.
    ///
    /// One builder with a knob rather than two scenes, deliberately: two
    /// independent scenes drifting apart is the exact failure this module
    /// was created to end.
    pub soil_moisture: u16,
    /// **The frame the world starts on**, which pins the weather.
    ///
    /// `weather::at` is a pure function of `(seed, frame)` and both CA
    /// drivers call `weather::step`, so a scene that does not pin its frame
    /// window is at the mercy of rain -- and at the default seed the first
    /// rain lands at frame 14,400, which is inside the window a long run
    /// uses. A dormancy comparison is meaningless if the dry arm gets
    /// rained on.
    ///
    /// **Note a dry pin is not a calm pin**: gusts fire before the
    /// precipitation early-return, and `organism::wind_lean_dir` steers
    /// growth off the velocity they inject. Pinning this makes rain
    /// reproducible, not absent.
    ///
    /// Multiples of `DAY_NIGHT_PERIOD_FRAMES` (3600) are worth preferring:
    /// 3600 = 80 x 45, so a multiple pins the day phase, the rendered sky
    /// and every organism's `(frame + id) % ORGANISM_TICK_INTERVAL` tick
    /// offset at once.
    pub start_frame: u64,
    /// Rows of soil above the stone floor.
    ///
    /// A parameter rather than the `SOIL_DEPTH` constant because the
    /// paired comparison the water economy needs is *deep soil against a
    /// thin skin over rock, same seeds* -- and with soil depth compiled in,
    /// that experiment could not be run at all. A shallow bed holds less
    /// water and gives a root system nowhere to go, so it is the case where
    /// having roots is supposed to stop being free.
    pub soil_depth: i32,
}

impl PlantScene {
    /// The same bed, below the permanent wilting point -- the dormancy arm.
    ///
    /// `plant_available_fraction` is exactly zero at or under
    /// `SOIL_WILTING_POINT`, so nothing germinates here until the ground is
    /// wetted. Paired with `default()` it is the two-arm comparison the
    /// seed-dormancy work needs, from one builder.
    pub fn dormant() -> Self {
        Self { soil_moisture: material::SOIL_WILTING_POINT, ..Self::default() }
    }
}

impl Default for PlantScene {
    fn default() -> Self {
        // 200 leaves the turgor bound, not the world edge, as the limit --
        // see `PlantScene`'s doc.
        Self {
            width: 512,
            height: 320,
            ground_y: 200,
            trees: 8,
            species: "tree".to_string(),
            soil_depth: SOIL_DEPTH,
            soil_moisture: material::SOIL_FIELD_CAPACITY,
            start_frame: 0,
        }
    }
}

impl PlantScene {
    pub fn build(&self) -> World {
        let mut w = World::new(Rect::new(0, 0, self.width - 1, self.height - 1));
        w.frame = self.start_frame;
        let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
        for x in 0..self.width {
            for y in (self.ground_y + self.soil_depth)..(self.ground_y + self.soil_depth + STONE_DEPTH) {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
            for y in self.ground_y..(self.ground_y + self.soil_depth) {
                w.set(x, y, Cell::new(soil, (rng::jitter(x, y) * 255.0) as u8).with_aux(self.soil_moisture));
            }
        }
        // Evenly spaced across the world so spacing is a function of tree
        // count and nothing else — the property the old pair of scenes
        // lacked, and the reason a spacing mechanism could be tuned at one
        // density and judged at another.
        let spacing = self.width / (self.trees as i32 + 1);
        for i in 0..self.trees {
            let x = spacing * (i as i32 + 1);
            w.plant_tree_species(x, self.ground_y - SEED_DROP, &self.species);
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
