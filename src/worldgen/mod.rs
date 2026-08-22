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
//! Caves, erosion, world age and streaming.

pub mod column;
pub mod erosion;
pub mod legacy;
pub mod noise;
pub mod params;
pub mod passes;
pub mod region;
pub mod residual;

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
    /// Roughly what share of generation this pass is, in parts per thousand
    /// of the whole — **for the loading bar only**.
    ///
    /// Measured at 8192x2560 with `PASS_TIMING=1`, where three stages are 98%
    /// of the wall time: `stone_massif` 3882 ms, `compute_world_distances`
    /// 2665, `soil_moisture` 888, and everything else together 250. A bar
    /// driven by the *count* of stages would therefore sit still through
    /// half the wait and then leap, which is the thing a progress bar exists
    /// not to do.
    ///
    /// Being somewhat wrong here is cosmetic and nothing reads it but the
    /// bar, so it does not need re-deriving every time a pass changes — only
    /// when one of the big three moves.
    weight: u16,
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
    Pass { name: "stone_massif", margin: 0, weight: 514, run: passes::stone_massif },
    Pass { name: "bedrock_floor", margin: 0, weight: 1, run: passes::bedrock_floor },
    Pass { name: "soil_blanket", margin: 2, weight: 4, run: passes::soil_blanket },
    // The two formation passes read further than they write, and by a lot.
    // Both numbers were re-derived when cliff detection gained its
    // escarpment scale, and one of them was already wrong before that:
    // `talus` declared 3 while walking up to `MAX_FALL` = 120 columns to
    // find the foot of a fall. A margin is the contract a per-chunk
    // generator will plan against, so an understated one is a promise to
    // produce different cells at a chunk edge -- worth stating honestly even
    // though it is large. Shrinking them is a job for the coarse map, not
    // for optimism here.
    //
    // Both are now *expressions* over the constants that produce them
    // (`passes::BROWS_MARGIN`, `passes::TALUS_MARGIN`) rather than the
    // numbers those expressions happened to equal. Every margin in this
    // table has been silently wrong at least once, and each time the number
    // was right on the day and had no way to stay right;
    // `every_local_pass_declares_the_margin_it_reaches` in
    // `tests/worldgen.rs` is the check, and the expressions are what leave
    // it nothing to catch.
    //
    //   brows: RUN_FAR (20) of detection + MAX_BROW_REACH (20) of writing
    //   talus: RUN_FAR (20) + MAX_FALL (120) walking to the foot
    //          + 2 * MAX_TALUS_PEAK of apron, which is 2x because the heap
    //          runs out at a slope of about a half
    Pass { name: "brows", margin: passes::BROWS_MARGIN, weight: 1, run: passes::brows },
    Pass { name: "talus", margin: passes::TALUS_MARGIN, weight: 1, run: passes::talus },
    Pass { name: "pockets", margin: 0, weight: 6, run: passes::pockets },
    // Residual landforms -- tors, stacks, pinnacles authored directly
    // (`residual.rs`; B1 measured plan-space erosion never offers one to
    // protect, `Reports/worldgen-implementation-tasks-round6-formations.md`
    // B1/B2). A site's footprint reaches at most its own half-width; the
    // derivation lives with the constants it reads
    // (`residual::RESIDUALS_MARGIN`), because the version restated here had
    // gone stale -- it cited an aspect floor of 0.8 that was measured
    // prominence-inert and withdrawn in favour of 1.1, leaving a margin that
    // was safe by accident. Runs after `pockets` (so a residual can stand on
    // a settled surface, including any buried lens the collect-verify-write
    // contract must still respect) and before `boulders` (so a boulder's own
    // collect-verify-write sees a residual that already claimed a site as
    // solid stone and correctly declines to overlap it, rather than the two
    // colliding).
    Pass { name: "residuals", margin: residual::RESIDUALS_MARGIN, weight: 1, run: residual::residuals },
    // Boulder sockets from erosion's shed markers. Zero margin: a marker at
    // `x` seats a cluster whose footprint is at most a handful of columns
    // either side of `x`, well inside a single column's worth of slack, and
    // the marker itself is plan-space data already computed for the whole
    // world rather than something this pass has to look sideways for.
    Pass { name: "boulders", margin: 0, weight: 1, run: passes::boulders },
    // Finite, and **now derived in the source rather than restated here**: a
    // cave system is placed at a column and reaches at most
    // `MAX_CAVE_HALF_W` either side of it, plus `VAULT_RIND` of stone that
    // must be checked. An understated margin is a promise to produce
    // different cells at a chunk edge.
    //
    // **Was 96, derived from a fixed 90-cell half-width.** Round 6's A2
    // made the envelope a per-system draw reaching `MAX_CAVE_HALF_W` = 200,
    // so the true reach became 202 and this number was silently wrong --
    // silently because nothing checks it at runtime: `pass_summary()`'s only
    // consumer looks at the GLOBAL list, not at the numbers. It was then
    // written as the literal 224, which is the same trap set again: the
    // number was right on the day and had no way to stay right. Writing the
    // *expression* is what removes the class, and
    // `a_cave_cannot_reach_past_its_declared_margin` in `tests/worldgen.rs`
    // still asserts it against the constants so a future reach that the
    // expression fails to capture fails the test instead of the world.
    Pass { name: "vaults", margin: passes::VAULTS_MARGIN, weight: 1, run: passes::vaults },
    // The two water passes read the whole world: where water stands depends
    // on the lowest rim enclosing a hollow, which can be any distance away.
    // They are the first honest `GLOBAL` entries in this table and the debt
    // the coarse map is for.
    Pass { name: "ponds", margin: GLOBAL, weight: 1, run: passes::ponds },
    Pass { name: "soil_moisture", margin: GLOBAL, weight: 118, run: passes::soil_moisture },
    Pass { name: "moisture_init", margin: GLOBAL, weight: 2, run: passes::moisture_init },
    // Last, so it can see the finished ground -- including whether a column
    // ended up under water.
    Pass { name: "life_scatter", margin: 0, weight: 1, run: passes::life_scatter },
];

/// Everything the passes share: the decided columns, and the material ids
/// they write.
pub struct Ctx<'a> {
    pub terrain: Terrain<'a>,
    /// One entry per column of the world, indexed by x.
    pub plans: Vec<ColumnPlan>,
    /// What plan-space erosion moved and left behind on the way to `plans`
    /// (`erosion.rs`) — talus and sediment depths per column, boulder-socket
    /// markers, and the volume counters. `plan_all` already folded the
    /// deposit depths into `plans[x].soil_depth`; this is what lets a
    /// realise pass tell a deposit apart from the native blanket it landed
    /// in, which `plan_all` alone cannot answer.
    pub deposits: erosion::Deposits,
    pub stone: MaterialId,
    pub soil: MaterialId,
    pub sand: MaterialId,
    pub gravel: MaterialId,
    pub water: MaterialId,
    /// The vault pass's lining. `shard` is not held here: nothing at genesis
    /// writes it, it exists as crystal's `breaks_into` for when a player
    /// mines one out.
    pub crystal: MaterialId,
    /// The two *formation* materials — stalagmites, stalactites, columns.
    ///
    /// Distinct from `stone`/`crystal`, which is what lets a formation be
    /// walk-through scenery while the cave wall stays a wall: the flag lives
    /// on the material, so sharing an id with the massif would make the
    /// massif walk-through too. See `Material::scenery`.
    pub flowstone: MaterialId,
    pub spar: MaterialId,
    /// Tangent of gravel's angle of repose, for the scree pass.
    pub gravel_tan: f32,
    /// Cover cells `soil_blanket` recoloured as talus (round-4 task 5's
    /// counter). A `Cell` rather than a return value: `Pass::run` only
    /// reports cells *written*, and this is a subset of those, counted
    /// separately so the printed erosion detail line can say how much of
    /// the deposit actually realised distinct from the raw plan-side sum.
    pub talus_recolored: std::cell::Cell<usize>,
    /// Boulders (not boulder *cells* — the pass-table row already has
    /// those) `boulders` actually seated, out of however many markers
    /// `erosion::Deposits::boulder` proposed.
    pub boulders_seated: std::cell::Cell<usize>,
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
        let crystal = id("crystal");
        let (flowstone, spar) = (id("flowstone"), id("spar"));
        // Read soil's angle of repose from the material data rather than
        // assuming it: the generator's whole at-rest guarantee is that it
        // never places soil steeper than this, and an edit to `soil.ron` that
        // the generator did not see would quietly break it.
        let soil_tan = world.materials.get(soil).friction_angle.to_radians().tan();
        let sand_tan = world.materials.get(sand).friction_angle.to_radians().tan();
        let gravel_tan = world.materials.get(gravel).friction_angle.to_radians().tan();
        let terrain =
            Terrain::new(seed, params, bounds.max_x + 1, bounds.max_y + 1, soil_tan, sand_tan);
        let (plans, deposits) = terrain.plan_all_with_deposits();
        Self {
            terrain,
            plans,
            deposits,
            stone,
            soil,
            sand,
            gravel,
            water,
            crystal,
            flowstone,
            spar,
            gravel_tan,
            talus_recolored: std::cell::Cell::new(0),
            boulders_seated: std::cell::Cell::new(0),
        }
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
    generate_reporting(world, spec, &mut |_, _| {});
}

/// Shares of the whole for the two stages that are not rows in `PASSES`, on
/// the same per-thousand scale as `Pass::weight` and measured the same way.
/// The plan phase is `Ctx::new` (erosion included) at 134 ms; `structure` is
/// `compute_world_distances` at 2665 ms, the second-largest stage there is.
const PLAN_WEIGHT: u16 = 18;
const STRUCTURE_WEIGHT: u16 = 353;

/// Called with `(fraction_done, stage_name)` **before** each stage runs, so a
/// caller can put a name and a bar on screen while it waits.
///
/// A fraction rather than a stage index, because the stages are wildly
/// uneven — see `Pass::weight`. The plan phase and `compute_world_distances`
/// are reported even though neither is a row in `PASSES`: between them they
/// are over a third of the wall time, so a caller driven by the pass table
/// alone would show nothing happening through two of the longest waits.
pub type Progress<'a> = &'a mut dyn FnMut(f32, &'static str);

/// [`generate`], announcing each stage before it starts.
pub fn generate_reporting(world: &mut World, spec: Spec, progress: Progress) {
    let total = f32::from(PLAN_WEIGHT + STRUCTURE_WEIGHT)
        + PASSES.iter().map(|p| f32::from(p.weight)).sum::<f32>();
    let _ = generate_reported_with(world, spec, &mut |done, name| progress(done as f32 / total, name));
    progress((total - f32::from(STRUCTURE_WEIGHT)) / total, "structure");
    structural::compute_world_distances(world);
}

/// Like [`generate_only`], but reports how many cells each pass wrote.
///
/// The counter that goes next to the picture. A pass reporting zero has not
/// run, whatever the render suggests.
pub fn generate_reported(world: &mut World, spec: Spec) -> Vec<(&'static str, usize)> {
    generate_reported_with(world, spec, &mut |_, _| {})
}

/// [`generate_reported`] with a stage announcement before each pass.
fn generate_reported_with(
    world: &mut World,
    spec: Spec,
    stage: &mut dyn FnMut(u32, &'static str),
) -> Vec<(&'static str, usize)> {
    match spec {
        Spec::Legacy => {
            legacy::build(world);
            Vec::new()
        }
        Spec::Generated { params, seed } => {
            // The world keeps its own seed, because generation is not the
            // only thing that needs to know which world this is: a plant's
            // genotype is drawn from it (`plant::seed_genotype`), so two
            // worlds grown from different seeds must not produce the same
            // individual at the same coordinate.
            world.seed = seed;
            // The plan phase is inside `Ctx::new` -- erosion included -- and
            // it is not a row in `PASSES`, so it has to be timed here or it
            // is invisible to any per-pass accounting.
            let plan_started = std::time::Instant::now();
            stage(0, "plan");
            let mut done = u32::from(PLAN_WEIGHT);
            let ctx = Ctx::new(world, params, seed);
            let plan_ms = plan_started.elapsed().as_secs_f64() * 1000.0;
            // `PASS_TIMING=1` prints wall time per pass. Gated behind an env
            // var rather than always on, for the same reason `vaults detail`
            // is gated inside its own pass: this runs in every test that
            // builds a world, and the pass table is long.
            let timing = std::env::var("PASS_TIMING").is_ok();
            let report: Vec<(&'static str, usize)> = PASSES
                .iter()
                .map(|pass| {
                    let started = std::time::Instant::now();
                    stage(done, pass.name);
                    done += u32::from(pass.weight);
                    let written = (pass.run)(&ctx, world);
                    if timing {
                        println!(
                            "  pass {:>14}: {:>8.1}ms  {written} cells",
                            pass.name,
                            started.elapsed().as_secs_f64() * 1000.0
                        );
                    }
                    (pass.name, written)
                })
                .collect();
            if timing {
                println!("  pass {:>14}: {plan_ms:>8.1}ms  (plans + erosion, inside Ctx::new)", "[plan phase]");
            }
            // Erosion runs in the plan phase (`Terrain::plan_all_with_
            // deposits`, inside `Ctx::new` above), so it has no row of its
            // own in `PASSES` -- there is nowhere in the pass loop to
            // report it. A bare `println!` in `plan_all` would fire on
            // every call, including the many pure per-column tests that
            // never build a world at all (round-4 task 5); gating it here,
            // the same way `vaults detail` is gated inside its own pass
            // function, means it prints only when a real world is
            // generated, and the `iterations == 0` guard means an age-0
            // world -- every preset until task 4, `flat` still -- prints
            // nothing at all.
            if ctx.deposits.iterations > 0 {
                println!(
                    "  erosion detail: moved {:.1} exported {:.1} talus {:.1} sediment {:.1} \
                     boulder-markers {} boulders-seated {} talus-recoloured {} | {:.1}ms",
                    ctx.deposits.volume_moved,
                    ctx.deposits.exported,
                    ctx.deposits.talus.iter().sum::<f32>(),
                    ctx.deposits.sediment.iter().sum::<f32>(),
                    ctx.deposits.boulder.iter().filter(|&&b| b).count(),
                    ctx.boulders_seated.get(),
                    ctx.talus_recolored.get(),
                    ctx.deposits.wall_time_ms,
                );
            }
            report
        }
    }
}

/// Names of the generation passes, in order — the ablation harness's
/// vocabulary.
///
/// Exposed so `examples/pass_ablation.rs` can name a pass to skip without
/// duplicating the table. Duplicating it was the first design and it is the
/// wrong one: a harness carrying its own copy of the pass list goes stale
/// silently the moment a pass is added or reordered, and an ablation run
/// against a stale list reports interference between the wrong pair.
pub fn pass_names() -> Vec<&'static str> {
    PASSES.iter().map(|p| p.name).collect()
}

/// Generate with one pass switched off, reporting what every *other* pass
/// wrote.
///
/// **The instrument for pass interference, which is this generator's
/// recurring defect class.** Five separate times a shipped mechanism has
/// been found producing nothing visible because an earlier pass had already
/// taken the cells it wanted, and every one of them was found by accident,
/// one per round: `pockets` lenses rejecting whole cave systems, `brows`
/// lips refusing boulder domes (round-4 finding R4-1), `soil_blanket`
/// folding erosion's talus in before the legacy talus pass could add its
/// own (R4-2), `brows` roofing water that `ponds` then filled from both
/// sides (R4-3, open bug 0), and plan-space erosion flattening the
/// formation-scale relief the raw heightfield had
/// (`worldgen-erosion-design.md`). None of those is visible in a pass's own
/// cell count, because each pass reports only what *it* wrote — a pass that
/// wrote nothing because its cells were taken looks exactly like a pass
/// whose noise draw came up empty.
///
/// Differencing the whole report vector across an ablation makes the
/// interaction a number: if switching off `brows` raises what `boulders`
/// writes, `brows` was eating boulders, and the size of the rise is how
/// much. Genesis-only and read-only — nothing here runs per frame.
///
/// `skip` names a pass from [`pass_names`]; an unknown name is a caller
/// error and panics rather than silently ablating nothing, because "no
/// interference detected" and "the ablation never happened" are the same
/// output otherwise.
pub fn generate_ablated(
    world: &mut World,
    spec: Spec,
    skip: &str,
) -> Vec<(&'static str, usize)> {
    match spec {
        Spec::Legacy => {
            legacy::build(world);
            Vec::new()
        }
        Spec::Generated { params, seed } => {
            assert!(
                skip.is_empty() || PASSES.iter().any(|p| p.name == skip),
                "no pass named {skip:?}; the table has {:?}",
                pass_names()
            );
            world.seed = seed;
            let ctx = Ctx::new(world, params, seed);
            PASSES
                .iter()
                .map(|pass| {
                    let wrote = if pass.name == skip { 0 } else { (pass.run)(&ctx, world) };
                    (pass.name, wrote)
                })
                .collect()
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

    /// The loading bar's input is monotonic, starts at zero and ends near
    /// one, and no single stage is most of the bar.
    ///
    /// The last clause is the one worth having: `Pass::weight` is a set of
    /// hand-entered numbers that nothing else reads, so nothing else would
    /// ever notice them drifting away from the truth. A bar that jumps from
    /// 5% to 95% in one step is the failure this guards, and it is invisible
    /// to every other test in the repo.
    #[test]
    fn generation_reports_progress_that_moves_steadily() {
        let (presets, _) = crate::worldgen::WorldgenPresets::load();
        let name = presets.default_name();
        let params = presets.get(&name).expect("the default preset resolves").clone();
        let mut world = crate::sim::world::World::new(Rect::new(0, 0, 511, 319));

        let mut seen: Vec<(f32, &'static str)> = Vec::new();
        generate_reporting(&mut world, Spec::Generated { params: &params, seed: 1 }, &mut |f, n| {
            seen.push((f, n))
        });

        assert_eq!(seen.len(), PASSES.len() + 2, "one report per stage, plus plan and structure");
        assert_eq!(seen[0].0, 0.0, "the bar does not start empty: {:?}", seen[0]);
        assert!(seen.windows(2).all(|w| w[0].0 <= w[1].0), "progress went backwards: {seen:?}");
        let last = seen.last().expect("at least one stage");
        assert_eq!(last.1, "structure", "the last stage reported is not the structural pass");
        assert!(last.0 > 0.5 && last.0 < 1.0, "the final stage starts at {} of the bar", last.0);

        let biggest = seen.windows(2).map(|w| w[1].0 - w[0].0).fold(0.0f32, f32::max);
        assert!(biggest < 0.6, "one stage is {:.0}% of the bar in a single jump", biggest * 100.0);
    }

    #[test]
    fn only_the_water_passes_read_the_whole_world() {
        // Which passes are `GLOBAL` is the list of things standing between
        // this generator and per-chunk generation, so it is worth being an
        // assertion rather than a comment: adding a third one should be a
        // deliberate decision that fails here first, not something noticed
        // when streaming is attempted.
        let global: Vec<&str> =
            pass_summary().into_iter().filter(|(_, m)| *m == GLOBAL).map(|(n, _)| n).collect();
        assert_eq!(global, vec!["ponds", "soil_moisture", "moisture_init"]);
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
