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
/// Salt for the neutral hazard's own RNG substream, so its rolls cannot
/// collide with any founding or growth draw keyed on the same world seed.
const HAZARD_SALT: u64 = 0x4861_7A61_7264_5F31;
/// How far above the ground a seed is released. Seeds are `Powder` and
/// fall, so this exercises the fall and landing rather than pre-placing
/// each one — and it is the path that found the `ChunkView` cell-list bug.
pub const SEED_DROP: i32 = 25;

/// **How the bed's resources vary across it.**
///
/// The flat bed cannot speak to selection at all, and that is a statement
/// about the *experiment*, not about the plants. A cost creates a real
/// trade-off only when three things hold: the cost exists, it is paid in the
/// same currency as the benefit, and **the environment varies which arm
/// wins** (`Reports/plant-equilibrium-costs-2026-08-27.md` §8 — *"Only
/// condition 3 buys diversity"*). In a uniform bed every plant faces one
/// optimum, so a lever with a perfectly good interior optimum still produces
/// one answer, repeated.
///
/// The canonical plant-evolution result says the same from the other side:
/// *"multi-task fitness landscapes have many near-equal optima; single-task
/// landscapes have one … selecting on at least three conflicting tasks is not
/// a nice-to-have; it is the mechanism"* (Niklas, via
/// `plant-simulation-research.md` §7b).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Relief {
    /// Uniform moisture, uniform depth, evenly spaced founders — the
    /// historical bed.
    ///
    /// **The default, and it must stay the default.** Every stored contact
    /// sheet and every number ever quoted from `plant_probe`, `divergence` or
    /// `grove` was taken here; changing what `PlantScene::default()` builds
    /// would void all of them at once. A new bed is a new named arm, exactly
    /// as `PlantScene::dormant()` is.
    #[default]
    Flat,
    /// Three conflicting tasks, on axes chosen to be **independent** so that
    /// all four corners exist (wet-deep, wet-shallow, dry-deep, dry-shallow)
    /// rather than one axis wearing three names:
    ///
    /// 1. **Water** — a moisture gradient across x, wet end to dry end.
    /// 2. **Rooting volume** — soil depth varying on a different period, so
    ///    depth and moisture are not collinear.
    /// 3. **Light** — founders clumped rather than evenly spaced, so some
    ///    grow crowded and shaded and others in the open. This reuses the
    ///    shading and `canopy_density` competition the engine already has
    ///    rather than adding terrain to cast shadows.
    ///
    /// The dry end stays **above** the wilting point deliberately. Below it
    /// `plant_available_fraction` is exactly zero and nothing germinates, so
    /// the comparison would be a living stand against an empty field rather
    /// than two morphologies — the scene error `CLAUDE.md` records as having
    /// cost real time twice.
    Varied,
}

/// **A neutral random hazard** — the disturbance half of selection.
///
/// Each interval, each established plant independently faces a fixed
/// probability of being marked senescent, and `plant::rot_remains` then
/// thins it out at the species' own half-life. Reusing that seam rather than
/// deleting cells is what keeps the death graded rather than a
/// disappearance.
///
/// **Neutral means independent of age, size and genotype, and that is the
/// whole design constraint.** Age-biased removal is itself a selective force
/// favouring fast reproducers, so culling by age would *manufacture* the
/// ruderal-strategy result an experiment like this is hoping to observe
/// (`Reports/plant-evolvability-handoff-2026-08-27.md` §5). Size-biased
/// removal has the same defect one step removed, since size correlates with
/// strategy. So the roll reads nothing about the plant it is applied to.
///
/// Keyed on `(world seed, frame, organism id)` through `rng::stream`, so it
/// is deterministic, reproducible, and does not consume from any generator
/// the simulation itself uses — a hazard that advanced a shared stream would
/// change the run it is supposed to be observing.
#[derive(Clone, Copy, Debug)]
pub struct Hazard {
    /// Probability per plant per interval. `0.0` disables it entirely, which
    /// is what every existing harness run gets.
    pub chance: f32,
    /// Frames between rolls.
    pub interval: u64,
}

impl Default for Hazard {
    fn default() -> Self {
        Self { chance: 0.0, interval: 1800 }
    }
}

/// Roll the hazard for this frame, returning how many plants it marked.
///
/// Call once per stepped frame; it is a no-op except on interval boundaries
/// and when `chance > 0.0`. **Returns the count so a harness can print it**
/// — a disturbance that silently never fires looks exactly like a disturbance
/// that fired and changed nothing, and this project has shipped that
/// confusion more than once.
pub fn apply_hazard(w: &mut World, hazard: Hazard, established_cells: usize) -> usize {
    if hazard.chance <= 0.0 || hazard.interval == 0 || !w.frame.is_multiple_of(hazard.interval) {
        return 0;
    }
    let (seed, frame) = (w.seed, w.frame);
    let mut killed = 0;
    for id in w.live_organism_ids() {
        // Established plants only. A seed or a two-cell seedling dying tells
        // us nothing about morphology, and including them would make the
        // hazard mostly a seed-bank tax.
        let big_enough = w.organism(id).is_some_and(|s| s.cells.len() >= established_cells && !s.senescent);
        if !big_enough {
            continue;
        }
        // A fixed salt so the hazard's stream cannot collide with any
        // other `rng::stream` keyed on the same world seed.
        let mut rng = rng::stream(seed ^ HAZARD_SALT, frame, id as u64, 0);
        if rng.chance(hazard.chance) && w.mark_organism_senescent(id) {
            killed += 1;
        }
    }
    killed
}

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
    /// **How resources vary across the bed** — see [`Relief`]. `Flat` is the
    /// historical bed and the default, so every existing harness run and
    /// every stored contact sheet is unaffected by this field existing.
    pub relief: Relief,
    /// A species defined at runtime, registered **before** the bed plants
    /// anything.
    ///
    /// The ordering is the whole reason this is a field rather than something
    /// a caller does around `build`: `plant_tree_species` resolves the name at
    /// planting time, so a variant registered after `build` is registered into
    /// a world where nothing was ever planted — a silent empty stand that
    /// reads exactly like "this mutation is nonviable". Caught by a positive
    /// control reporting 0/3 for the *unmutated* table.
    pub species_ron: Option<String>,
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

    /// The bed with three conflicting tasks in it — see [`Relief::Varied`].
    ///
    /// More founders than the flat bed's eight, because the point is a
    /// *population* spread over a gradient rather than a row of specimens:
    /// at eight, each environment holds one plant and a per-environment
    /// median is one sample from a distribution that spans 31 to 153 cells.
    pub fn varied() -> Self {
        Self { relief: Relief::Varied, trees: 24, ..Self::default() }
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
            relief: Relief::Flat,
            species_ron: None,
        }
    }
}

impl PlantScene {
    /// The deepest soil any column of this bed can hold — what the stone
    /// floor has to sit beneath so that no column is left bottomless.
    ///
    /// Mirrors the swing in `build`'s `Relief::Varied` arm, and `build`
    /// asserts the two agree on every column.
    ///
    /// **A runtime assert rather than a unit test, deliberately.** `#[test]`
    /// functions in `examples/` are never collected by `cargo test`, so a
    /// guard written here would be exactly the shape `CLAUDE.md` warns about:
    /// green because it never ran. Checking it in the builder costs one
    /// comparison per column, once per scene, and turns the failure it guards
    /// -- a bottomless soil column that slumps and looks like "the mechanism
    /// does nothing" -- from silent into loud.
    fn max_soil_depth(&self) -> i32 {
        match self.relief {
            Relief::Flat => self.soil_depth,
            Relief::Varied => (self.soil_depth as f32 * 1.55).ceil() as i32,
        }
    }

    pub fn build(&self) -> World {
        let mut w = World::new(Rect::new(0, 0, self.width - 1, self.height - 1));
        w.frame = self.start_frame;
        if let Some(source) = &self.species_ron {
            w.species.register_ron(source).expect("the runtime species parses");
        }
        let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
        for x in 0..self.width {
            // **Two independent axes, on different periods.** Collinear axes
            // would be one axis wearing two names, and the whole point is
            // that all four corners exist -- wet-deep, wet-shallow, dry-deep,
            // dry-shallow -- so a plant cannot satisfy both with one strategy.
            let (moisture, depth) = match self.relief {
                Relief::Flat => (self.soil_moisture, self.soil_depth),
                Relief::Varied => {
                    let t = x as f32 / (self.width - 1).max(1) as f32;
                    // Water: wet end to dry end, linear. The dry end stops
                    // well above the wilting point -- below it nothing
                    // germinates and the dry half would be an empty field
                    // rather than a second morphology.
                    let wet = material::SOIL_FIELD_CAPACITY as f32;
                    let dry = (material::SOIL_WILTING_POINT as f32) * 1.6;
                    // Rooting volume: a full sine period across the bed, so
                    // depth is uncorrelated with the linear moisture ramp.
                    let swing = (t * std::f32::consts::TAU).sin();
                    let depth = (self.soil_depth as f32 * (1.0 + 0.55 * swing)).round() as i32;
                    ((wet + (dry - wet) * t) as u16, depth.max(4))
                }
            };
            // **The stone floor is pinned to the deepest column, not to this
            // one.** Written as `..(ground_y + self.soil_depth + STONE_DEPTH)`
            // the range *inverts* wherever the varied bed digs deeper than
            // nominal, so those columns got no floor at all, the soil fell out
            // of the world and the bed slumped -- while the cell census stayed
            // perfectly healthy and said nothing. That is the scene error
            // `CLAUDE.md` records as having cost real time twice, and it was
            // caught by looking at the render rather than by any number here.
            let floor_top = self.ground_y + depth;
            let floor_bottom = self.ground_y + self.max_soil_depth() + STONE_DEPTH;
            assert!(
                floor_top < floor_bottom,
                "column {x} would get no stone floor: soil {depth} rows deep against a floor pinned \
                 at {} -- `max_soil_depth` disagrees with the swing in this arm, and the bed would \
                 slump silently",
                self.max_soil_depth()
            );
            for y in floor_top..floor_bottom {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
            for y in self.ground_y..(self.ground_y + depth) {
                w.set(x, y, Cell::new(soil, (rng::jitter(x, y) * 255.0) as u8).with_aux(moisture));
            }
        }
        // Evenly spaced across the world so spacing is a function of tree
        // count and nothing else — the property the old pair of scenes
        // lacked, and the reason a spacing mechanism could be tuned at one
        // density and judged at another.
        let spacing = self.width / (self.trees as i32 + 1);
        for i in 0..self.trees {
            let even = spacing * (i as i32 + 1);
            // **Light, as the third task.** Even spacing gives every founder
            // the same light environment, so light competition is a constant
            // and cannot select for anything. Clumping some and isolating
            // others makes shade tolerance pay in one place and height pay in
            // another -- reusing the shading and `canopy_density` competition
            // the engine already has, rather than building terrain to cast
            // shadows.
            //
            // Deterministic in `(i, world)` so a bed is reproducible, and
            // clamped inside the world so no founder is planted off the edge.
            let x = match self.relief {
                Relief::Flat => even,
                Relief::Varied => {
                    let shove = (rng::jitter(i as i32, 7) - 0.5) * 1.7 * spacing as f32;
                    (even + shove as i32).clamp(2, self.width - 3)
                }
            };
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
