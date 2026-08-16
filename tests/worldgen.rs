//! Generated worlds: reproducible, at rest, and structurally honest.
//!
//! The three claims worldgen makes that nothing else checks.
//!
//! **At rest** is the one unique to a falling-sand engine
//! (`Reports/worldgen-design.md` §6a). Every other generator can emit
//! whatever shape it likes; here, a world that is not already in equilibrium
//! slumps the moment it loads, and the player watches their world visibly
//! settle before they can touch it. The generator's defence is placement,
//! not exemption: solids cannot move at all, and powders are only ever put
//! where their own angle of repose keeps them. This file is what says that
//! defence actually holds, across every preset and a spread of seeds, rather
//! than on the one world someone happened to look at.
//!
//! The at-rest check reports a **count**, not a bool, per `CLAUDE.md`: a bare
//! assertion tells you the world moved, and the number tells you whether it
//! was one grain on one ledge or the whole surface avalanching — which are
//! different bugs.

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{parallel, structural};
use pixel_physics::worldgen::{self, Spec, WorldgenParams, WorldgenPresets};

/// Full sandbox dimensions. Worth the cost here specifically: the base relief
/// wave has a period of one world width, so a smaller world is a *different*
/// composition, and the slopes this is all about would not be the ones the
/// player sees.
const BOUNDS: (i32, i32) = (511, 319);

/// Seeds every preset is checked against. Five is a sample, not a proof —
/// mashing `F6` in the app is still the real sweep — but it is enough to
/// catch a rule that only holds for the seed it was tuned on.
const SEEDS: [u64; 5] = [1, 2, 3, 4, 5];

fn build(params: &WorldgenParams, seed: u64) -> World {
    let mut world = World::new(Rect::new(0, 0, BOUNDS.0, BOUNDS.1));
    worldgen::generate(&mut world, Spec::Generated { params, seed });
    world
}

/// One full frame of everything that can move material.
fn step(world: &mut World) {
    parallel::step(world);
    world.step_liquid_bodies();
    world.step_active_sites();
    world.step_fields();
}

/// Every non-empty cell as `(x, y, material)`.
fn snapshot(world: &World) -> Vec<(i32, i32, u16)> {
    let mut out = Vec::new();
    for y in 0..=BOUNDS.1 {
        for x in 0..=BOUNDS.0 {
            let c = world.get(x, y);
            if c.material != material::EMPTY {
                out.push((x, y, c.material.0));
            }
        }
    }
    out
}

fn world_hash(world: &World) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for y in 0..=BOUNDS.1 {
        for x in 0..=BOUNDS.0 {
            let c = world.get(x, y);
            for byte in [c.material.0 as u64, c.shade as u64, c.aux() as u64] {
                h ^= byte;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
    }
    h
}

fn presets() -> WorldgenPresets {
    let (p, err) = WorldgenPresets::load();
    assert!(err.is_none(), "assets/worldgen.ron failed to parse: {err:?}");
    p
}

#[test]
fn generated_terrain_is_already_at_rest() {
    let presets = presets();
    let mut worst = 0usize;
    let mut report = String::new();
    for (name, params) in &presets.presets {
        for seed in SEEDS {
            let mut world = build(params, seed);
            let before: std::collections::HashSet<_> = snapshot(&world).into_iter().collect();
            for _ in 0..120 {
                step(&mut world);
            }
            let after: std::collections::HashSet<_> = snapshot(&world).into_iter().collect();
            let mut gone: Vec<_> = before.difference(&after).copied().collect();
            if gone.len() > worst {
                // Sorted and named, because the count alone says the world
                // moved and not which rule let it. Where the cells were and
                // what they were made of is what points at the pass.
                gone.sort();
                worst = gone.len();
                let sample: Vec<String> = gone
                    .iter()
                    .take(8)
                    .map(|(x, y, m)| {
                        let name = world.materials.get(pixel_physics::sim::material::MaterialId(*m)).name.clone();
                        format!("({x},{y}) {name}")
                    })
                    .collect();
                report = format!("{name} seed {seed}: {} cells left their position; first: {}", gone.len(), sample.join(", "));
            }
        }
    }
    assert_eq!(worst, 0, "generated terrain settled — {report}");
}

#[test]
fn generated_terrain_stops_sweeping_almost_immediately() {
    // The other half of at-rest, and the one the frame budget cares about:
    // terrain that never moves but keeps its chunks awake costs the
    // dirty-rect skip every frame forever. The world is generated dirty on
    // purpose, so the first sweep has to examine everything; what matters is
    // that it then goes quiet.
    // Measured per preset rather than on one world, because the cost is not
    // the terrain: a dry preset is quiet in a handful of frames, and standing
    // water takes longer because the liquid solver has to shuffle sub-cell
    // fill to convergence even though no cell ever changes position (the
    // at-rest test above is what says the positions hold). Bar set from the
    // measurement with headroom, per `CLAUDE.md` — not from an aspiration,
    // and not sitting on the measured value.
    let presets = presets();
    let mut worst = (0, String::new());
    for (name, params) in &presets.presets {
        let mut world = build(params, 1);
        let mut frames = 0;
        while world.active_chunk_count() > 0 && frames < 120 {
            step(&mut world);
            frames += 1;
        }
        println!("{name}: quiet after {frames} frames");
        if frames > worst.0 {
            worst = (frames, name.clone());
        }
    }
    assert!(
        worst.0 <= 45,
        "{} took {} frames to go quiet; a generated world should settle within a second of opening",
        worst.1,
        worst.0
    );
}

#[test]
fn the_same_seed_builds_the_same_world() {
    let presets = presets();
    for (name, params) in &presets.presets {
        for seed in SEEDS {
            assert_eq!(
                world_hash(&build(params, seed)),
                world_hash(&build(params, seed)),
                "{name} seed {seed} is not reproducible"
            );
        }
    }
}

#[test]
fn different_seeds_build_different_worlds() {
    let presets = presets();
    let params = presets.get(&presets.default_name()).expect("default preset");
    let hashes: Vec<u64> = SEEDS.iter().map(|s| world_hash(&build(params, *s))).collect();
    for (i, a) in hashes.iter().enumerate() {
        for b in &hashes[i + 1..] {
            assert_ne!(a, b, "two seeds produced identical worlds");
        }
    }
}

#[test]
fn a_generated_world_survives_a_replay() {
    // Determinism over generation *and* simulation together. Generation
    // being reproducible and the sweep being reproducible are separate
    // properties, and catch-up needs both.
    let presets = presets();
    let params = presets.get(&presets.default_name()).expect("default preset");
    let run = || {
        let mut world = build(params, 7);
        for _ in 0..60 {
            step(&mut world);
        }
        world_hash(&world)
    };
    assert_eq!(run(), run());
}

#[test]
fn every_solid_is_anchored_and_no_liquid_carries_a_stale_fill() {
    // §6b's landmine: an anchor distance of zero is indistinguishable from
    // "anchored", so terrain that never went through the structural pass
    // reads as fine and collapses the first time anything disturbs it.
    let presets = presets();
    let params = presets.get(&presets.default_name()).expect("default preset");
    let world = build(params, 3);
    let stone = world.materials.id_of("stone").unwrap();

    let mut attached_stone = 0;
    for y in 0..=BOUNDS.1 {
        for x in 0..=BOUNDS.0 {
            let c = world.get(x, y);
            if c.material == stone {
                assert!(c.attached(), "unattached stone in the massif at ({x}, {y})");
                assert!(c.aux() < u16::MAX, "stone at ({x}, {y}) never reached an anchor");
                attached_stone += 1;
            }
            if c.material == material::BEDROCK {
                assert!(c.attached(), "bedrock must be attached at ({x}, {y})");
            }
        }
    }
    assert!(attached_stone > 10_000, "vacuous: only {attached_stone} stone cells in the world");
}

#[test]
fn every_pass_writes_something() {
    // The counter that a picture cannot replace. A pass that silently never
    // fires leaves terrain that still looks plausible — this engine has
    // already shipped one feature that rendered convincingly and had never
    // executed once.
    let presets = presets();
    let params = presets.get(&presets.default_name()).expect("default preset");
    // Brows and talus depend on the world containing genuine cliffs, which
    // is a per-seed property; checked across the sweep rather than per seed.
    let mut totals: std::collections::BTreeMap<&str, usize> = Default::default();
    for seed in SEEDS {
        let mut world = World::new(Rect::new(0, 0, BOUNDS.0, BOUNDS.1));
        for (name, cells) in worldgen::generate_reported(&mut world, Spec::Generated { params, seed }) {
            *totals.entry(name).or_default() += cells;
        }
    }
    for (name, cells) in &totals {
        assert!(*cells > 0, "pass {name} never wrote a cell across {} seeds", SEEDS.len());
    }
}

#[test]
fn every_pool_has_a_level_surface() {
    // The guard for the two ways generated water has already failed, both of
    // which the at-rest sweep catches only *after* 120 frames of simulation
    // and neither of which is legible in a render.
    //
    // A pool's surface must be flat, because a sloped one is a head
    // difference and head differences flow. The first version took each
    // column's own `max(spill, table)`, and since the table is a subdued
    // replica of the ground it varies across a basin — so the lake came out
    // tilted. The second grouped contiguous *wet* columns, which splits a
    // basin at any submerged ridge and gives the two halves different levels;
    // that one drained 686 cells of water into itself on the first sweep.
    //
    // Checked per contiguous run of water, so both failures show up here as a
    // difference in top-of-water between neighbouring columns.
    let presets = presets();
    for (name, params) in &presets.presets {
        for seed in SEEDS {
            let world = build(params, seed);
            let water = world.materials.id_of("water").unwrap();
            // The *free* surface: the topmost water with open air directly
            // above it. Not simply the topmost water cell, because water
            // standing under an overhanging brow has rock above it and its
            // top sits a cell lower without that being a slope — the first
            // version of this test flagged exactly that and was wrong to.
            let free_surface = |x: i32| {
                (0..=BOUNDS.1).find(|&y| {
                    world.get(x, y).material == water && (y == 0 || world.get(x, y - 1).material == material::EMPTY)
                })
            };
            let mut previous: Option<(i32, i32)> = None;
            for x in 0..=BOUNDS.0 {
                match free_surface(x) {
                    Some(top) => {
                        if let Some((px, ptop)) = previous {
                            if px == x - 1 {
                                assert_eq!(
                                    ptop, top,
                                    "{name} seed {seed}: pool surface steps from {ptop} to {top} between x {px} and {x}"
                                );
                            }
                        }
                        previous = Some((x, top));
                    }
                    None => previous = None,
                }
            }
        }
    }
}

#[test]
fn generated_water_is_full_and_never_inside_the_ground() {
    // `aux == 0` on a `Liquid` cell means **full**, so the generator must
    // leave it alone; writing a literal fill is the documented way to
    // manufacture a full cell out of nothing. And the saturated zone is a
    // field value, never liquid cells in the rock -- a cell holds one
    // material and there is no porosity, which is the reason a high water
    // table cannot flood the underground however it is tuned.
    let presets = presets();
    let params = presets.get("wetland").expect("wetland preset");
    let world = build(params, 1);
    let water = world.materials.id_of("water").unwrap();
    let mut wet = 0;
    for y in 0..=BOUNDS.1 {
        for x in 0..=BOUNDS.0 {
            let c = world.get(x, y);
            if c.material == water {
                assert_eq!(c.aux(), 0, "generated water at ({x}, {y}) carries a fill value");
                // Nothing solid above it in the same column below the surface:
                // water only ever stands in open hollows, so the cell directly
                // above is water or air, never rock.
                if y > 0 {
                    let above = world.get(x, y - 1).material;
                    assert!(
                        above == water || above == material::EMPTY,
                        "water at ({x}, {y}) is buried under {:?}",
                        world.materials.get(above).name
                    );
                }
                wet += 1;
            }
        }
    }
    assert!(wet > 200, "vacuous: wetland seed 1 generated only {wet} water cells");
}

#[test]
fn the_saturated_zone_does_not_dry_out() {
    // The moisture floor's whole purpose. Without it, evaporation takes the
    // deep world to zero within a few hundred frames and the water table
    // quietly stops existing -- and because `field::step` rebuilds every tile
    // from scratch each frame, the floor also has to be carried forward
    // explicitly or it survives exactly one frame.
    let presets = presets();
    let params = presets.get("wetland").expect("wetland preset");
    let mut world = build(params, 1);
    // A column with a table well inside the world, probed below it.
    let probe_x = BOUNDS.0 / 2;
    let deep_y = BOUNDS.1 - 40;
    let floor = world.field_moisture_floor(probe_x, deep_y);
    assert!(floor > 0.5, "test setup: expected saturated ground at ({probe_x}, {deep_y}), floor is {floor}");
    for _ in 0..300 {
        step(&mut world);
    }
    let moisture = world.field_at(probe_x, deep_y).moisture;
    assert!(
        moisture >= floor * 0.95,
        "the aquifer dried out: floor {floor}, moisture after 300 frames {moisture}"
    );
    // And the sky is still dry, so this is a floor and not a blanket.
    assert_eq!(world.field_moisture_floor(probe_x, 4), 0.0, "the sky was given a moisture floor");
}

#[test]
fn switching_water_off_switches_all_of_it_off() {
    // The stated pivot: if the water table turns out not to be fun, one
    // preset removes it entirely. That is only true if it removes the
    // moisture floor as well as the pools -- a preset with no lakes but damp
    // ground everywhere would be a half-measure, and the point of this lever
    // is that it is total.
    let presets = presets();
    for name in ["arid", "flat"] {
        let params = presets.get(name).unwrap_or_else(|| panic!("{name} preset"));
        let world = build(params, 3);
        let water = world.materials.id_of("water").unwrap();
        for y in 0..=BOUNDS.1 {
            for x in 0..=BOUNDS.0 {
                assert_ne!(world.get(x, y).material, water, "{name} generated water at ({x}, {y})");
                assert_eq!(world.field_moisture_floor(x, y), 0.0, "{name} left a moisture floor at ({x}, {y})");
            }
        }
    }
}

#[test]
fn the_world_arrives_with_both_moss_and_trees_in_it() {
    // Counts each *kind*, not the pass's total, and that distinction is the
    // whole test. The pass reported a healthy 13 cells while planting zero
    // trees: `last_tree` started at `i32::MIN`, so the spacing check
    // `x - last_tree` overflowed, wrapped negative, and rejected every tree
    // in every world forever. The total looked fine, the render looked like
    // a world where trees are rare, and only splitting the count by species
    // said otherwise.
    let presets = presets();
    let params = presets.get(&presets.default_name()).expect("default preset");
    let (mut trees, mut moss_cells) = (0, 0);
    for seed in SEEDS {
        let world = build(params, seed);
        let wood = world.materials.id_of("wood").unwrap();
        let moss = world.materials.id_of("moss").unwrap();
        for y in 0..=BOUNDS.1 {
            for x in 0..=BOUNDS.0 {
                match world.get(x, y).material {
                    m if m == wood => trees += 1,
                    m if m == moss => moss_cells += 1,
                    _ => {}
                }
            }
        }
    }
    assert!(trees > 0, "no tree was planted in any of {} worlds", SEEDS.len());
    assert!(moss_cells > 0, "no moss was planted in any of {} worlds", SEEDS.len());
}

#[test]
fn planted_life_is_clustered_rather_than_evenly_spaced() {
    // The claim the squared cluster field exists to make. Evenly spaced
    // vegetation is the tell that a world was populated by a loop, and a
    // uniform random scatter is only slightly better — what reads as natural
    // is stands with clearings between them.
    //
    // Measured as the spread of gaps between neighbouring plants: clustered
    // placement produces both very small gaps (inside a stand) and very large
    // ones (between stands), so the largest gap is many times the smallest.
    let presets = presets();
    let params = presets.get(&presets.default_name()).expect("default preset");
    let mut widest_ratio = 0.0f32;
    for seed in SEEDS {
        let world = build(params, seed);
        let wood = world.materials.id_of("wood").unwrap();
        let moss = world.materials.id_of("moss").unwrap();
        let columns: Vec<i32> = (0..=BOUNDS.0)
            .filter(|&x| {
                (0..=BOUNDS.1).any(|y| {
                    let m = world.get(x, y).material;
                    m == wood || m == moss
                })
            })
            .collect();
        if columns.len() < 4 {
            continue;
        }
        let gaps: Vec<i32> = columns.windows(2).map(|w| w[1] - w[0]).collect();
        let (smallest, largest) = (*gaps.iter().min().unwrap(), *gaps.iter().max().unwrap());
        widest_ratio = widest_ratio.max(largest as f32 / smallest.max(1) as f32);
    }
    assert!(
        widest_ratio >= 8.0,
        "plants are too evenly spread: widest gap is only {widest_ratio:.1}x the narrowest"
    );
}

#[test]
fn the_legacy_terrain_is_unchanged_by_the_move() {
    // `worldgen::legacy` is a verbatim move of what `app::build_terrain_only`
    // used to contain. Several filmstrip scenes and app tests erase or probe
    // its exact coordinates, so "close enough" is a silent way to make those
    // start testing something else.
    let mut world = World::new(Rect::new(0, 0, BOUNDS.0, BOUNDS.1));
    worldgen::generate(&mut world, Spec::Legacy);
    let stone = world.materials.id_of("stone").unwrap();

    for y in 318..=319 {
        assert_eq!(world.get(256, y).material, material::BEDROCK, "bedrock row {y}");
    }
    for y in 312..=317 {
        assert_eq!(world.get(256, y).material, stone, "stone floor row {y}");
    }
    // The three ledges and the pillar, at the coordinates other tests use.
    for &(x, y) in &[(60, 202), (460, 152), (250, 262), (250, 300)] {
        assert_eq!(world.get(x, y).material, stone, "ledge cell ({x}, {y})");
        assert!(world.get(x, y).attached(), "ledge cell ({x}, {y}) must be attached");
    }
    // And nothing above them.
    assert_eq!(world.get(60, 199).material, material::EMPTY, "something above the left ledge");
}

#[test]
fn the_default_preset_matches_the_compiled_in_fallback() {
    // `WorldgenParams::default` is the fallback for any field a preset
    // omits, and it is documented as being the `rolling` values. Nothing
    // enforces that but this: left to drift, a preset that omits a field
    // would silently inherit a number nobody has looked at since.
    let presets = presets();
    let rolling = presets.get("rolling").expect("rolling preset");
    assert_eq!(*rolling, WorldgenParams::default(), "assets/worldgen.ron's `rolling` has drifted from WorldgenParams::default");
}

#[test]
fn structural_distances_are_computed_once_and_hold() {
    // `generate` runs the structural pass itself, so a caller that only
    // places material would be building the §6b landmine by hand. Asserts
    // the split behaves: placement alone leaves distances unset, and the
    // full call sets them.
    let presets = presets();
    let params = presets.get(&presets.default_name()).expect("default preset");
    let mut placed = World::new(Rect::new(0, 0, BOUNDS.0, BOUNDS.1));
    worldgen::generate_only(&mut placed, Spec::Generated { params, seed: 2 });
    let placed_hash = world_hash(&placed);
    structural::compute_world_distances(&mut placed);
    assert_ne!(placed_hash, world_hash(&placed), "the structural pass changed nothing — distances were already set?");
}

#[test]
fn the_flat_preset_is_a_usable_structural_test_bed() {
    // `flat` exists so that "does this building stand" can be asked without
    // the world's own shape being part of the answer, and it is a *preset*
    // rather than a code path precisely so it cannot drift away from how a
    // real world is built. That only pays off if it actually delivers what
    // the structural work needs, which is four things -- and each one is a
    // way it could look fine on screen and quietly ruin a measurement.
    let presets = presets();
    let params = presets.get("flat").expect("assets/worldgen.ron must ship a `flat` preset");
    let world = build(params, 7);

    let surface = |x: i32| (0..=BOUNDS.1).find(|&y| world.get(x, y).material != material::EMPTY);
    let heights: Vec<i32> = (0..=BOUNDS.0).filter_map(surface).collect();
    assert_eq!(heights.len() as i32, BOUNDS.0 + 1, "some column has no ground at all");

    // 1. Flat. A one-cell step is still a step: a wall stamped across it
    //    stands on two different heights and the load path is not the one
    //    being tested.
    let (lo, hi) = (*heights.iter().min().unwrap(), *heights.iter().max().unwrap());
    assert_eq!(hi - lo, 0, "the flat preset's surface varies by {} cells (from y={lo} to y={hi})", hi - lo);

    // 2. Bare rock, not soil. A cell resting on loose grain keeps a
    //    sixty-fourth of its bending capacity (`GRANULAR_CAPACITY_DIVISOR`),
    //    so a test bed with a skin of sand on it measures something else
    //    entirely and gives no sign that it did.
    let sand = world.materials.id_of("sand").expect("sand is compiled in");
    let grains = (0..=BOUNDS.0).filter(|&x| world.get(x, lo).material == sand).count();
    assert_eq!(grains, 0, "{grains} columns have sand at the surface — structures would be standing on powder");

    // 3. Headroom for the reference room `B` stamps. At the shipped
    //    `sky_rows` of 95 there is not enough air and the key correctly
    //    refuses, which would make the test bed useless for the one thing
    //    it was added for.
    assert!(lo > 160 + 8, "only {lo} cells of sky — `B` cannot stamp a 200x160 reference room here");

    // 4. Nothing standing on it. A tree or a boulder in the middle of the
    //    bed is something a structure can lean on, and load arriving by a
    //    route nobody intended is this engine's most expensive recurring
    //    bug.
    let clutter = (0..=BOUNDS.0).filter(|&x| surface(x).is_some_and(|y| y < lo)).count();
    assert_eq!(clutter, 0, "{clutter} columns have something standing above the ground line");
}
