//! World generation.
//!
//! Sits above `sim` in the layering: worldgen writes through `World::set` and
//! calls into `structural`, and nothing under `src/sim/` imports anything
//! here. That direction is deliberate — the simulation has to stay testable
//! and meaningful with a hand-built world, which is what every scene in
//! `examples/filmstrip.rs` relies on.
//!
//! # Shape
//!
//! Generation is split in two, the way every shipped generator splits it
//! (`Reports/prior-art-worldgen-slicing.md` §6.6):
//!
//! * [`column`] **decides** — pure functions of `(seed, params, x)` with no
//!   world access and no dependence on traversal order.
//! * [`passes`] **realises** — named passes that write cells, each declaring
//!   how many columns of context it reads either side of what it writes.
//!
//! The split is what makes the eventual move to per-chunk generation
//! (`worldgen(seed, coord, world_age)`, `Reports/worldgen-design.md` §4) a
//! change of *caller* rather than a rewrite: a chunk generator plans the
//! columns it covers plus each pass's margin, then runs the same passes over
//! the same decisions, and every cell comes out identical to the whole-world
//! build. Two passes are marked [`GLOBAL`] instead of a margin, and those are
//! the honest debt — they need the whole world's shape, and paying them off
//! is exactly what the coarse `(x, z)` map of design doc §5 is for.
//!
//! # Water, and how to remove it
//!
//! The water table is the part of this design most at risk of being right and
//! not fun, so it is built to be removable by data rather than by code:
//! `table_offset` past the world height puts the table below the floor, which
//! produces no pools and no moisture floor at all. The `arid` and `flat`
//! presets both ship that way. Nothing underground ever fills with liquid
//! regardless of the setting — see [`passes::moisture_init`].
//!
//! # What is not here yet
//!
//! Plant scatter, caves, erosion, world age and streaming.

pub mod column;
pub mod legacy;
pub mod noise;
pub mod params;
pub mod passes;

use crate::sim::material::MaterialId;
use crate::sim::structural;
use crate::sim::world::World;
use column::{ColumnPlan, Terrain};
pub use params::{WorldgenParams, WorldgenPresets, LEGACY};

/// What to build.
pub enum Spec<'a> {
    /// The hand-authored sandbox terrain. See [`legacy`].
    Legacy,
    /// Seeded generation from a preset.
    Generated { params: &'a WorldgenParams, seed: u64 },
}

/// Marks a pass that reads the whole world rather than a bounded neighbourhood.
///
/// Not a number, because a number would imply the pass could be made chunk-
/// local by planning a few more columns, and these cannot: they need to know
/// about terrain arbitrarily far away. Every pass carrying this is a stated
/// prerequisite for streaming, and the coarse map is what removes it.
pub const GLOBAL: i32 = i32::MAX;

/// One named stage of generation.
struct Pass {
    /// For diagnostics and for the timing breakdown in `examples/ascii.rs`.
    name: &'static str,
    /// Columns of context this pass reads beyond the ones it writes. `0` is a
    /// pure per-column pass; [`GLOBAL`] means it reads the world.
    margin: i32,
    /// Returns how many cells it wrote.
    ///
    /// Reporting the count is not bookkeeping: a picture cannot show whether
    /// the feature that produced it is the one you built. This engine has
    /// already shipped a milestone where a collapse rendered convincingly as
    /// falling slabs, was read as the new mechanism working, and the harness
    /// count said it had never executed once (`CLAUDE.md`). Two very
    /// different generators look identical at the zoom a contact sheet is
    /// read at, so every pass says whether it fired.
    run: fn(&Ctx, &mut World) -> usize,
}

/// Generation stages, in order. Later passes overwrite earlier ones.
const PASSES: &[Pass] = &[
    Pass { name: "stone_massif", margin: 0, run: passes::stone_massif },
    Pass { name: "bedrock_floor", margin: 0, run: passes::bedrock_floor },
    Pass { name: "soil_blanket", margin: 2, run: passes::soil_blanket },
    Pass { name: "brows", margin: 4, run: passes::brows },
    Pass { name: "talus", margin: 3, run: passes::talus },
    Pass { name: "pockets", margin: 0, run: passes::pockets },
    // The two water passes read the whole world: where water stands depends
    // on the lowest rim enclosing a hollow, which can be any distance away.
    // They are the first honest `GLOBAL` entries in this table and the debt
    // the coarse map is for.
    Pass { name: "ponds", margin: GLOBAL, run: passes::ponds },
    Pass { name: "moisture_init", margin: GLOBAL, run: passes::moisture_init },
];

/// Everything the passes share: the decided columns, and the material ids
/// they write.
pub struct Ctx<'a> {
    pub terrain: Terrain<'a>,
    /// One entry per column of the world, indexed by x.
    pub plans: Vec<ColumnPlan>,
    pub stone: MaterialId,
    pub soil: MaterialId,
    pub sand: MaterialId,
    pub gravel: MaterialId,
    pub water: MaterialId,
}

impl<'a> Ctx<'a> {
    fn new(world: &World, params: &'a WorldgenParams, seed: u64) -> Self {
        let bounds = world.bounds().expect("worldgen needs a bounded world");
        let id = |name: &str| {
            world
                .materials
                .id_of(name)
                .unwrap_or_else(|| panic!("{name} is a compiled-in material"))
        };
        let (stone, soil, sand, gravel, water) =
            (id("stone"), id("soil"), id("sand"), id("gravel"), id("water"));
        // Read soil's angle of repose from the material data rather than
        // assuming it: the generator's whole at-rest guarantee is that it
        // never places soil steeper than this, and an edit to `soil.ron` that
        // the generator did not see would quietly break it.
        let soil_tan = world.materials.get(soil).friction_angle.to_radians().tan();
        let terrain =
            Terrain { seed, params, w: bounds.max_x + 1, h: bounds.max_y + 1, soil_tan };
        let plans = terrain.plan_all();
        Self { terrain, plans, stone, soil, sand, gravel, water }
    }
}

/// Build a world, then make it structurally real.
///
/// The structural pass is not optional and not deferred. Generated terrain
/// that has never been through it carries an anchor distance of zero, which
/// `structural.rs` cannot tell apart from "anchored" — the landmine
/// `Reports/worldgen-design.md` §6b names — so the first thing to disturb the
/// world would find a massif that believes it is holding itself up. Running
/// it here means every solid has a real distance from frame one, exactly as
/// the hand-authored terrain always has.
pub fn generate(world: &mut World, spec: Spec) {
    generate_only(world, spec);
    structural::compute_world_distances(world);
}

/// Like [`generate_only`], but reports how many cells each pass wrote.
///
/// The counter that goes next to the picture. A pass reporting zero has not
/// run, whatever the render suggests.
pub fn generate_reported(world: &mut World, spec: Spec) -> Vec<(&'static str, usize)> {
    match spec {
        Spec::Legacy => {
            legacy::build(world);
            Vec::new()
        }
        Spec::Generated { params, seed } => {
            let ctx = Ctx::new(world, params, seed);
            PASSES.iter().map(|pass| (pass.name, (pass.run)(&ctx, world))).collect()
        }
    }
}

/// Material placement only, without the structural pass.
///
/// Split out so `examples/ascii.rs` can time the two halves separately and
/// attribute the cost rather than just stating it — the structural pass is
/// the expensive half and scales with world area, so the two numbers move
/// for completely different reasons.
pub fn generate_only(world: &mut World, spec: Spec) {
    let _ = generate_reported(world, spec);
    // The world is left dirty on purpose, as the hand-authored terrain always
    // was. The first sweep examines everything, finds that none of it moves,
    // and settles from the second frame onward -- which is a claim the
    // at-rest tests in `tests/worldgen.rs` check rather than assume.
}

/// The pass table as `(name, margin)`, for tests and diagnostics.
///
/// Exposed so the streaming prerequisite stays visible: anything reporting
/// [`GLOBAL`] here is a pass that cannot run per-chunk yet.
pub fn pass_summary() -> Vec<(&'static str, i32)> {
    PASSES.iter().map(|p| (p.name, p.margin)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;

    #[test]
    fn only_the_water_passes_read_the_whole_world() {
        // Which passes are `GLOBAL` is the list of things standing between
        // this generator and per-chunk generation, so it is worth being an
        // assertion rather than a comment: adding a third one should be a
        // deliberate decision that fails here first, not something noticed
        // when streaming is attempted.
        let global: Vec<&str> =
            pass_summary().into_iter().filter(|(_, m)| *m == GLOBAL).map(|(n, _)| n).collect();
        assert_eq!(global, vec!["ponds", "moisture_init"]);
    }

    #[test]
    fn generation_is_deterministic() {
        let params = WorldgenParams::default();
        let build = || {
            let mut w = World::new(Rect::new(0, 0, 255, 191));
            generate(&mut w, Spec::Generated { params: &params, seed: 12345 });
            w
        };
        assert_eq!(world_hash(&build()), world_hash(&build()));
    }

    #[test]
    fn different_seeds_build_different_worlds() {
        let params = WorldgenParams::default();
        let build = |seed| {
            let mut w = World::new(Rect::new(0, 0, 255, 191));
            generate(&mut w, Spec::Generated { params: &params, seed });
            w
        };
        assert_ne!(world_hash(&build(1)), world_hash(&build(2)));
    }

    /// FNV-1a over every cell, in a fixed scan order.
    fn world_hash(world: &World) -> u64 {
        let bounds = world.bounds().unwrap();
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                let c = world.get(x, y);
                for byte in [c.material.0 as u64, c.shade as u64, c.aux() as u64] {
                    h ^= byte;
                    h = h.wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
        }
        h
    }
}
