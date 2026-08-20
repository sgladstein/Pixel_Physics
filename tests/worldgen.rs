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
    // A *mineral* claim, so the life pass is switched off for it. Since the
    // plant merge, planted seeds germinate within frames of generation (the
    // column-cast light reaches the surface immediately) and a growing
    // plant rewrites cells by design — a world with living flora is never
    // "at rest" and never should be. Flora presence has its own test
    // (`the_world_arrives_with_both_moss_and_trees_in_it`); this one is
    // about whether the generated terrain itself holds still.
    let presets = presets();
    let mut worst = 0usize;
    let mut report = String::new();
    for (name, params) in &presets.presets {
        let mut params = params.clone();
        params.tree_density = 0.0;
        params.moss_density = 0.0;
        let params = &params;
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
    // Life pass off, same reasoning as `generated_terrain_is_already_at_
    // rest`: growing flora legitimately keeps its own chunks awake, and this
    // test's claim is about the *terrain* not costing the dirty-rect skip.
    let presets = presets();
    let mut worst = (0, String::new());
    for (name, params) in &presets.presets {
        let mut params = params.clone();
        params.tree_density = 0.0;
        params.moss_density = 0.0;
        let mut world = build(&params, 1);
        let mut frames = 0;
        while world.active_chunk_count() > 0 && frames < 120 {
            // Which chunks, and what inside them changed across one frame —
            // a count alone says the world is awake and not which rule is
            // keeping it up. Sampled sparsely to keep the output readable.
            if frames % 30 == 29 {
                let awake: Vec<_> = world.chunks_to_sweep();
                let mut before = Vec::new();
                for &coord in awake.iter().take(2) {
                    let (ox, oy) = coord.origin();
                    for dy in 0..64 {
                        for dx in 0..64 {
                            let c = world.get(ox + dx, oy + dy);
                            before.push((ox + dx, oy + dy, c.material, c.aux()));
                        }
                    }
                }
                step(&mut world);
                frames += 1;
                let mut changed = Vec::new();
                for (x, y, m, aux) in before {
                    let c = world.get(x, y);
                    if c.material != m || c.aux() != aux {
                        let name = world.materials.get(m).name.clone();
                        changed.push(format!("({x},{y}) {name} aux {aux}->{}", c.aux()));
                    }
                }
                println!("{name}: frame {frames}, awake {awake:?}; changed: {}", changed.join(", "));
                continue;
            }
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
        // A freshly planted tree is a `seed` cell now, not wood: the seed
        // material arrived with the plant merge, and `plant_tree_species`
        // only falls back to wood when it is absent — which is what this
        // test was unknowingly counting before. Wood still counts too, so
        // the test keeps passing the moment a seed germinates.
        let seed_material = world.materials.id_of("seed").unwrap();
        for y in 0..=BOUNDS.1 {
            for x in 0..=BOUNDS.0 {
                match world.get(x, y).material {
                    m if m == wood || m == seed_material => trees += 1,
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

#[test]
fn every_generated_shade_indexes_a_real_palette_entry() {
    // The guard for the region palette families. `render.rs` indexes with
    // `shade % palette.len()`, which never panics -- it silently *aliases*,
    // so a palette shortened by one entry would draw sandstone cells as
    // damp grey and nothing anywhere would fail. That is the failure this
    // catches, and it has to be an assertion because no picture shows it:
    // the world would still look like a world.
    //
    // Checked over every preset x seed rather than one, because which
    // families a world reaches depends on the region draw.
    let presets = presets();
    for (name, params) in &presets.presets {
        for seed in SEEDS {
            let world = build(params, seed);
            for material in ["stone", "soil", "sand", "gravel"] {
                let id = world.materials.id_of(material).expect("compiled-in material");
                let len = world.materials.get(id).palette.len();
                let mut worst = 0u8;
                for y in 0..=BOUNDS.1 {
                    for x in 0..=BOUNDS.0 {
                        let c = world.get(x, y);
                        if c.material == id {
                            worst = worst.max(c.shade);
                        }
                    }
                }
                assert!(
                    (worst as usize) < len,
                    "{name} seed {seed}: worldgen wrote {material} shade {worst} into a \
                     {len}-entry palette -- it will alias onto another family"
                );
            }
        }
    }
}

#[test]
fn cliff_formations_land_at_cliffs_and_are_visibly_present() {
    // The canary the world review asked for. `brows` wrote 34 cells and
    // `talus` 148 in a 1.3M-cell world -- passes that fired, reported a
    // count, and were invisible, because cliff detection asked for six cells
    // of drop within four columns and a regional escarpment has that
    // nowhere along it.
    //
    // Two claims, and the second is why this is not just a bigger number:
    // the extra cells have to land **at cliffs**, not smeared over ordinary
    // hillside. A detector loosened until it fires everywhere would move the
    // counts just as well and would be strictly worse.
    //
    // Paired against the same world with each pass switched off, which is
    // the only way to attribute a cell to a pass -- the classifier lesson
    // from `buried_gravel_is_not_the_same_colour_as_scree` below.
    let presets = presets();
    for preset in ["rolling", "canyon", "terraced"] {
        let params = presets.get(preset).expect("preset");
        let world = build(params, 1);
        let no_talus = build(&WorldgenParams { talus_max_height: 0.0, ..params.clone() }, 1);
        let no_brows = build(&WorldgenParams { brow_chance: 0.0, ..params.clone() }, 1);

        // The plan's ground line, taken from the world with neither
        // formation, so an apron does not count as the ground it rests on.
        let bare = build(
            &WorldgenParams { talus_max_height: 0.0, brow_chance: 0.0, ..params.clone() },
            1,
        );
        let ground = |x: i32| {
            (0..=BOUNDS.1).find(|&y| bare.get(x, y).material != material::EMPTY).unwrap_or(BOUNDS.1)
        };
        // Largest height *difference* within 20 columns either side -- the
        // escarpment scale the detector now works at.
        //
        // Absolute, and that is not a detail: measuring only "does the ground
        // fall away from here" scores zero on exactly the cells this test is
        // about. A brow cell hangs out over ground that has *already* fallen,
        // and an apron sits at the foot of a face that *rises* beside it, so
        // a downhill-only reading called 91 of 216 brow cells and 72 of 164
        // talus cells misplaced when every one of them was where it belonged.
        // Third time this shape of mistake has appeared in this track.
        let relief = |x: i32| {
            (1..=20)
                .flat_map(|d| [ground((x - d).max(0)), ground((x + d).min(BOUNDS.0))])
                .map(|g| (g - ground(x)).abs())
                .max()
                .unwrap_or(0)
        };

        let (mut talus, mut brows) = (0usize, 0usize);
        let (mut talus_at_cliff, mut brows_at_cliff) = (0usize, 0usize);
        let mut tallest = (0i32, 0i32);
        let mut heap: std::collections::BTreeMap<i32, i32> = Default::default();
        for y in 0..=BOUNDS.1 {
            for x in 0..=BOUNDS.0 {
                let here = world.get(x, y).material;
                if here == material::EMPTY {
                    continue;
                }
                if no_talus.get(x, y).material != here {
                    talus += 1;
                    *heap.entry(x).or_default() += 1;
                    // An apron is at a cliff if a real drop exists within
                    // reach of it. `MAX_FALL` is 120, but the apron itself
                    // sits at the foot, so the face is close by.
                    // Six, not twenty: six is the pass's own near-scale bar
                    // (`passes::CLIFF_DROP`), and an apron under a modest
                    // terrace riser is a legitimate apron. Asking for an
                    // escarpment scored 72% and would have been a test
                    // failing for wanting something the code never claimed.
                    if (0..=40).any(|d| relief((x - d).max(0)) >= 6 || relief((x + d).min(BOUNDS.0)) >= 6) {
                        talus_at_cliff += 1;
                    }
                } else if no_brows.get(x, y).material != here {
                    brows += 1;
                    if relief(x) >= 6 {
                        brows_at_cliff += 1;
                    }
                }
            }
        }
        for (x, h) in &heap {
            if *h > tallest.1 {
                tallest = (*x, *h);
            }
        }
        println!(
            "{preset} seed 1: talus {talus} cells ({talus_at_cliff} near a cliff), \
             brows {brows} ({brows_at_cliff} over a drop); tallest heap {} cells at x {}",
            tallest.1, tallest.0
        );
        // Bars set from the measurement with headroom, not from an
        // aspiration and not sitting on the measured value. The weakest of
        // the three at 512x320 seed 1 is terraced, at 47 talus and 102 brow
        // cells; 30 leaves room for the reshuffle any legitimate change to
        // the surface causes, while still being far above the 0-to-34 range
        // that was the state this rescue was written for.
        assert!(talus > 30, "{preset}: talus wrote only {talus} cells -- the rescue did not hold");
        assert!(brows > 30, "{preset}: brows wrote only {brows} cells -- the rescue did not hold");
        // The claim that matters. Ninety per cent rather than all of it:
        // an apron tapers away from its face by design, and its far toe can
        // legitimately sit past the window.
        assert!(
            talus_at_cliff * 10 >= talus * 9,
            "{preset}: only {talus_at_cliff} of {talus} talus cells are near any cliff at all -- \
             the detector is firing on flat ground"
        );
        assert!(
            brows_at_cliff * 10 >= brows * 9,
            "{preset}: only {brows_at_cliff} of {brows} brow cells hang over a drop"
        );
    }
}

#[test]
fn buried_gravel_is_not_the_same_colour_as_scree() {
    // The counter behind the gravel legibility fix, and it needs to be a
    // counter rather than a strip: a lens drawn in scree grey inside grey
    // rock is invisible, which looks exactly like the pockets pass not
    // having run -- and the pass *does* run, at full count. That is the
    // failure the world review actually found.
    //
    // Two claims, and both matter. A talus apron that quietly moved into the
    // buried family would make every cliff foot stop reading as broken rock,
    // which is the trade this split exists to avoid.
    //
    // **A paired comparison against the same world with `pocket_density: 0`,
    // which is the only thing that separates the two populations exactly.**
    // Two cheaper classifiers were tried and both miscounted, for the same
    // reason: they inferred which pass wrote a cell from where the cell is.
    // "More than ten cells below the ground line" called 37 soil-contact
    // cells buried, because a soil blanket is up to 34 cells deep and its
    // stony base is family 0 by design. "Fully surrounded by rock" called
    // 78, because a contact cell at the very bottom of a blanket often is.
    // Turning the pass off and diffing asks the question directly -- pockets
    // writes only into solid stone and nothing downstream reads a buried
    // lens, so the two worlds differ in exactly this pass's cells.
    let presets = presets();
    let params = presets.get("canyon").expect("canyon preset");
    let world = build(params, 1);
    let without = build(&WorldgenParams { pocket_density: 0.0, ..params.clone() }, 1);
    let gravel = world.materials.id_of("gravel").expect("gravel");

    let (mut lens, mut other) = (0usize, 0usize);
    let (mut lens_wrong, mut other_wrong) = (0usize, 0usize);
    for y in 0..=BOUNDS.1 {
        for x in 0..=BOUNDS.0 {
            let c = world.get(x, y);
            if c.material != gravel {
                continue;
            }
            if without.get(x, y).material == gravel {
                // Present with the pass off too: talus, or the soil profile's
                // stony contact. Must stay in the scree family.
                other += 1;
                if c.shade / 4 != 0 {
                    other_wrong += 1;
                }
            } else {
                lens += 1;
                if c.shade / 4 != 1 {
                    lens_wrong += 1;
                }
            }
        }
    }
    println!("canyon seed 1 gravel: {lens} cells in sealed lenses, {other} in scree and soil contact");

    // And the shape, because "lenses now lie along the bedding" is a claim a
    // colour census cannot support and a contact sheet reads at the wrong
    // zoom to settle. An unrotated lens is `2b` tall -- 4 to 8 cells -- so a
    // mean bounding-box height meaningfully above that is the rotation
    // having fired. Sand lenses count here too: the shape change applies to
    // both materials, only the palette split is gravel-only.
    let sand = world.materials.id_of("sand").expect("sand");
    let mut seen: std::collections::HashSet<(i32, i32)> = Default::default();
    let mut boxes: Vec<(i32, i32)> = Vec::new();
    for y in 0..=BOUNDS.1 {
        for x in 0..=BOUNDS.0 {
            let m = world.get(x, y).material;
            if (m != gravel && m != sand) || without.get(x, y).material == m || seen.contains(&(x, y)) {
                continue;
            }
            // Flood fill at 8 neighbours, matching how the ellipse was drawn.
            let (mut lo_x, mut hi_x, mut lo_y, mut hi_y) = (x, x, y, y);
            let mut stack = vec![(x, y)];
            seen.insert((x, y));
            while let Some((px, py)) = stack.pop() {
                lo_x = lo_x.min(px);
                hi_x = hi_x.max(px);
                lo_y = lo_y.min(py);
                hi_y = hi_y.max(py);
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (px + dx, py + dy);
                        if nx < 0 || ny < 0 || nx > BOUNDS.0 || ny > BOUNDS.1 || seen.contains(&(nx, ny)) {
                            continue;
                        }
                        let nm = world.get(nx, ny).material;
                        if (nm == gravel || nm == sand) && without.get(nx, ny).material != nm {
                            seen.insert((nx, ny));
                            stack.push((nx, ny));
                        }
                    }
                }
            }
            boxes.push((hi_x - lo_x + 1, hi_y - lo_y + 1));
        }
    }
    let n = boxes.len().max(1) as f32;
    let mean_w = boxes.iter().map(|b| b.0 as f32).sum::<f32>() / n;
    let mean_h = boxes.iter().map(|b| b.1 as f32).sum::<f32>() / n;
    println!(
        "canyon seed 1: {} sealed lenses, mean bounding box {mean_w:.1} x {mean_h:.1} cells (aspect {:.1}:1)",
        boxes.len(),
        mean_w / mean_h.max(1.0)
    );
    assert!(lens > 0, "the pockets pass placed no gravel lens at all");
    assert!(other > 0, "talus and the soil contact placed no gravel at all");
    assert_eq!(lens_wrong, 0, "{lens_wrong} of {lens} lens cells are in the scree family -- invisible in rock");
    assert_eq!(other_wrong, 0, "{other_wrong} of {other} scree cells drifted into the buried family");
}

#[test]
fn a_varied_world_uses_more_than_one_rock_family() {
    // "Did it fire at all needs a counter, not a picture" -- and this
    // mechanism is *exactly* the case that rule was written for, because
    // both outcomes look like a plausible world. A generator that silently
    // returned family 0 everywhere would render as the grey massif it
    // rendered as before the change, and the strips would be read as
    // "the shift is subtle" rather than "the shift never happened".
    let presets = presets();
    let families = |world: &World, id| {
        let mut seen: std::collections::BTreeSet<u8> = Default::default();
        for y in 0..=BOUNDS.1 {
            for x in 0..=BOUNDS.0 {
                let c = world.get(x, y);
                if c.material == id {
                    seen.insert(c.shade / 4);
                }
            }
        }
        seen
    };
    for preset in ["rolling", "canyon", "arid", "wetland"] {
        let params = presets.get(preset).expect("preset");
        let world = build(params, 1);
        let stone = world.materials.id_of("stone").expect("stone");
        let seen = families(&world, stone);
        // Printed as well as asserted: the bar is "more than one", and the
        // number next to the strip is what says whether that means a token
        // scattering or a world that genuinely changes country.
        let mut counts: std::collections::BTreeMap<u8, usize> = Default::default();
        for y in 0..=BOUNDS.1 {
            for x in 0..=BOUNDS.0 {
                let c = world.get(x, y);
                if c.material == stone {
                    *counts.entry(c.shade / 4).or_default() += 1;
                }
            }
        }
        println!("{preset} seed 1 rock families (0 neutral, 1 wet, 2 dry, 3 cap-rock): {counts:?}");
        assert!(seen.len() > 1, "{preset} seed 1: every rock cell is in family {seen:?}");
    }
    // And the control: `flat` asks for no regional variation, so it must get
    // none. Without this the test above passes just as well for a generator
    // that ignores `region_variation`, and the structural test bed would
    // have quietly changed colour under the destruction workstream.
    let flat = presets.get("flat").expect("flat preset");
    let world = build(flat, 1);
    let stone = world.materials.id_of("stone").expect("stone");
    assert_eq!(
        families(&world, stone),
        [0].into_iter().collect(),
        "flat asked for no regional variation and got a palette family anyway"
    );
}

// ---------------------------------------------------------------------------
// Step-0 probes for the August 2026 world review
// ---------------------------------------------------------------------------
//
// `#[ignore]`d on purpose: these print rather than assert, and they exist to
// answer three sightings the review left open
// (`Reports/worldgen-implementation-tasks-2026-08.md` task 1). Kept rather
// than deleted because the answers are what the Findings section of that file
// cites, and a finding whose reproduction has been thrown away is a claim
// nobody can re-check. Run with:
//
//   cargo test --test worldgen -- --ignored --nocapture
//
// They build at the **shipped** 2048x640, not the 512x320 the suite above
// uses, because the review's coordinates are shipped-size coordinates and the
// base relief wave has a period of one world width -- at 512 these columns
// are a different composition entirely.

/// The size the app ships, which is the size the review's x coordinates mean.
const REVIEW_BOUNDS: (i32, i32) = (2047, 639);

fn build_review_world(preset: &str, seed: u64) -> World {
    let presets = presets();
    let params = presets.get(preset).unwrap_or_else(|| panic!("preset {preset}"));
    let mut world = World::new(Rect::new(0, 0, REVIEW_BOUNDS.0, REVIEW_BOUNDS.1));
    worldgen::generate(&mut world, Spec::Generated { params, seed });
    world
}

/// Every non-empty cell in a column, top down, as `(y, material, shade, aux)`.
fn column_dump(world: &World, x: i32, limit: usize) -> Vec<(i32, String, u8, u16)> {
    let mut out = Vec::new();
    for y in 0..=REVIEW_BOUNDS.1 {
        let c = world.get(x, y);
        if c.material == material::EMPTY {
            continue;
        }
        out.push((y, world.materials.get(c.material).name.clone(), c.shade, c.aux()));
        if out.len() >= limit {
            break;
        }
    }
    out
}

#[test]
#[ignore = "probe: prints, never asserts (review task 1a)"]
fn probe_1a_the_blue_slivers() {
    // The sighting: canyon seed 1 generated **zero** pond cells per the
    // per-pass counter, and the rendered strip still shows a 1-2 column blue
    // sliver at world x~920 (and arid seed 1 at x~1215). Three candidate
    // explanations -- a leaked `Liquid`, moisture shading, or a sprite -- and
    // only a census of the actual cells separates them.
    //
    // A colour test cannot do it, and reaching for one first was a mistake
    // worth recording: the deep sky renders at (56,104,174) and water's
    // palette starts at (64,116,208), so "count the water-coloured pixels"
    // matched the entire sky and returned 2048 columns of nothing. The
    // material census below is the metric that means what it says.
    for (preset, centre) in [("canyon", 947i32), ("arid", 1237i32)] {
        let world = build_review_world(preset, 1);
        let water = world.materials.id_of("water").expect("water is compiled in");

        // 1. World-wide census. If the pass counter said zero and the world
        //    holds zero, no writer leaked water and the blue is not water.
        let mut total_water = 0usize;
        for y in 0..=REVIEW_BOUNDS.1 {
            for x in 0..=REVIEW_BOUNDS.0 {
                if world.get(x, y).material == water {
                    total_water += 1;
                }
            }
        }
        println!("\n=== {preset} seed 1 @ {}x{} ===", REVIEW_BOUNDS.0 + 1, REVIEW_BOUNDS.1 + 1);
        println!("  water cells in the whole world: {total_water}");

        // 2. Where the sky reaches down into the ground. A "sliver" of sky is
        //    a column whose first solid cell sits well below its neighbours'
        //    on both sides -- which is the same object task 1b is chasing,
        //    seen from the render's side rather than the plan's.
        let first_solid = |x: i32| -> i32 {
            (0..=REVIEW_BOUNDS.1)
                .find(|&y| world.get(x, y).material != material::EMPTY)
                .unwrap_or(REVIEW_BOUNDS.1)
        };
        let tops: Vec<i32> = (0..=REVIEW_BOUNDS.0).map(first_solid).collect();
        // Measured against the *shoulders*, not the immediate neighbours.
        // The first version of this compared three columns out and reported
        // zero notches in a world that plainly has them: the canyon slot is
        // seven columns wide, so x-3 and x+3 both land inside it and the
        // depth comes out as nothing. A metric written before its subject was
        // looked at measured the wrong thing, exactly as the method rule
        // says it will. The shoulder is the highest ground within a short
        // reach either side, which is what the eye reads as "the ridge this
        // slot is cut into".
        const REACH: i32 = 12;
        let shoulder = |x: i32, dir: i32| -> i32 {
            (1..=REACH)
                .map(|d| tops[(x + dir * d).clamp(0, REVIEW_BOUNDS.0) as usize])
                .fold(REVIEW_BOUNDS.1, i32::min)
        };
        let mut notches: Vec<(i32, i32)> = Vec::new();
        for x in REACH..REVIEW_BOUNDS.0 - REACH {
            let depth = tops[x as usize] - shoulder(x, -1).max(shoulder(x, 1));
            if depth >= 5 {
                notches.push((x, depth));
            }
        }
        // Reported as a distribution rather than against one bar. A single
        // threshold picked before looking is how the previous version of this
        // metric reported "zero notches" about a world with a plainly visible
        // one: the canyon slot is 6 cells deep and the bar was 8.
        let deep = notches.iter().filter(|(_, d)| *d >= 8).count();
        println!(
            "  sky notches within {REACH} columns of both shoulders: {} at >= 5 cells, {deep} at >= 8",
            notches.len()
        );
        notches.sort_by_key(|(_, d)| -d);
        for (x, d) in notches.iter().take(12) {
            println!("    x {x:>5}  {d:>4} cells deep");
        }

        // 3. An ASCII map of the one the review cited, so the *shape* is on
        //    the record and not just its depth. `.` is air.
        let (x0, x1) = (centre - 22, centre + 22);
        let y0 = (x0..=x1).map(|x| tops[x as usize]).min().unwrap_or(0) - 6;
        let y1 = (x0..=x1).map(|x| tops[x as usize]).max().unwrap_or(0) + 6;
        println!("  map x {x0}..{x1}, y {y0}..{y1}:");
        for y in y0.max(0)..=y1.min(REVIEW_BOUNDS.1) {
            let mut row = String::new();
            for x in x0..=x1 {
                let c = world.get(x, y);
                row.push(if c.material == material::EMPTY {
                    '.'
                } else {
                    let n = world.materials.get(c.material).name.clone();
                    let ch = n.chars().next().unwrap_or('?');
                    // Attached rock is the brow pass's signature, and telling
                    // it from massif rock is the whole question about a
                    // lintel over a slot.
                    if c.attached() && ch == 's' { 'S' } else { ch }
                });
            }
            println!("   {y:>4} {row}");
        }
    }
}

#[test]
#[ignore = "probe: prints, never asserts (review task 1c)"]
fn probe_1c_the_wetland_white_dashes() {
    // The review's conclusion, offered for confirmation: the pale dashes
    // along every pond surface are shoreline **sand** plus `water.ron`'s
    // `fill_dimming: 0.0`, not the monolayer/whisker artifact (which cannot
    // fire on a settled pond -- it requires air *below* the water cell, and a
    // pond's surface has water below it).
    //
    // One material census across a waterline settles it. Printed as the top
    // non-empty cell per column across the rim, so the sand/water alternation
    // -- if that is what it is -- is visible as a row.
    let world = build_review_world("rolling", 1);
    println!("\n=== rolling seed 1, pond rim x 760..980 ===");
    println!("  fill_dimming on water: {}", world.materials.get(world.materials.id_of("water").unwrap()).fill_dimming);
    println!("  {:>6} {:>8} {:>10} {:>6} {:>5}", "x", "first_y", "material", "shade", "aux");
    for x in 760..=980 {
        let top = column_dump(&world, x, 1);
        match top.first() {
            Some((y, name, shade, aux)) => println!("  {x:>6} {y:>8} {name:>10} {shade:>6} {aux:>5}"),
            None => println!("  {x:>6} {:>8} {:>10}", -1, "EMPTY"),
        }
    }
    // And the tally, because a 220-line listing is a picture and the question
    // ("what is the pale thing") is quantitative.
    let mut tally: std::collections::BTreeMap<String, usize> = Default::default();
    for x in 760..=980 {
        if let Some((_, name, _, _)) = column_dump(&world, x, 1).first() {
            *tally.entry(name.clone()).or_default() += 1;
        }
    }
    println!("  top-of-column tally across the rim: {tally:?}");

    // The top of the column is not where the dashes are. Scanning the
    // rendered strip for pale warm pixels puts them at rows 162..167, and the
    // pond's own surface is higher than that -- so the census that matters is
    // a *rectangle* around the bright pixels, not the skyline. Written this
    // way after the column tally came back "water 212, stone 9, sand 0" for a
    // strip that visibly has cream dashes in it: the metric was looking at
    // the wrong row.
    println!("  cells under the pale dashes (x 810..830, y 158..170):");
    for y in 158..=170 {
        let mut row = String::new();
        for x in 810..=830 {
            let c = world.get(x, y);
            // Distinct letters, because sand, soil and stone all start with
            // `s` and the first version of this map printed all three as the
            // same character -- a legend that cannot answer the question the
            // map was drawn for.
            row.push(match world.materials.get(c.material).name.as_str() {
                _ if c.material == material::EMPTY => '.',
                "water" => '~',
                "sand" => 'A',
                "soil" => 'O',
                "stone" => '#',
                "gravel" => 'v',
                "moss" => 'm',
                other => other.chars().next().unwrap_or('?'),
            });
        }
        println!("   {y:>4} {row}");
    }
    let mut rect: std::collections::BTreeMap<String, usize> = Default::default();
    for y in 158..=170 {
        for x in 760..=980 {
            let c = world.get(x, y);
            if c.material != material::EMPTY {
                *rect.entry(world.materials.get(c.material).name.clone()).or_default() += 1;
            }
        }
    }
    println!("  material tally in the dash band (x 760..980, y 158..170): {rect:?}");
}

#[test]
#[ignore = "probe: prints, never asserts (round-2 task 2)"]
fn probe_r2t2_how_columnar_is_a_family_boundary() {
    // The artifact is *shape*: a family whose probability is constant down a
    // column paints a full-height vertical band. Measure it at the scale the
    // band has -- blocks of BLOCK cells -- as the family mix per block, then
    // ask how much that mix changes between vertically adjacent blocks
    // against horizontally adjacent ones. A pier is a column of blocks that
    // all agree (vertical change ~ 0) sitting beside blocks that do not
    // (horizontal change large), so the ratio is the number to read, and it
    // rises toward 1 as the boundary starts to wander.
    //
    // **The first version of this probe measured per-cell run lengths and was
    // useless**, in the way CLAUDE.md warns about: it reported y/x = 0.99 for
    // every preset, at every setting, before and after. That is not a null
    // result, it is the wrong question -- the per-cell dither is white noise
    // drawn from `unit(seed, Palette, x, y)`, so its run lengths are
    // isotropic by construction whatever the probability behind them does.
    // It was measuring the stipple, never the band. The tell was the answer
    // being identical in cases known to differ.
    //
    // Paired against the same world with the field off, which here is exact:
    // `palette_field` changes no cell's material and no cell's position, only
    // which family byte it takes.
    const BLOCK: i32 = 8;
    let presets = presets();
    println!("\n=== family mix change between adjacent blocks, vertical vs horizontal ===");
    println!(
        "  {:>10} {:>5} {:>8} {:>9} {:>9} {:>7} {:>9}",
        "preset", "seed", "field", "d-vert", "d-horiz", "v/h", "top fam"
    );
    for (preset, seed) in [("canyon", 7u64), ("canyon", 13), ("rolling", 1), ("wetland", 1), ("arid", 1)] {
        let base = presets.get(preset).expect("preset");
        for field in [0.0f32, 0.15, 0.30, 0.45, 0.60] {
            let params = WorldgenParams { palette_field: field, ..base.clone() };
            let mut world = World::new(Rect::new(0, 0, REVIEW_BOUNDS.0, REVIEW_BOUNDS.1));
            worldgen::generate(&mut world, Spec::Generated { params: &params, seed });
            let stone = world.materials.id_of("stone").expect("stone");
            let (bw, bh) = ((REVIEW_BOUNDS.0 + 1) / BLOCK, (REVIEW_BOUNDS.1 + 1) / BLOCK);
            // Per block: the fraction of its stone cells in each family, and
            // how many stone cells it had. Blocks with too little rock to
            // characterise are dropped rather than counted as zero -- a
            // mostly-air block near the skyline would otherwise read as a
            // huge "change" against the solid one beneath it.
            let mut mix = vec![None; (bw * bh) as usize];
            for by in 0..bh {
                for bx in 0..bw {
                    let mut counts = [0f32; 4];
                    let mut total = 0f32;
                    for y in by * BLOCK..(by + 1) * BLOCK {
                        for x in bx * BLOCK..(bx + 1) * BLOCK {
                            let c = world.get(x, y);
                            if c.material == stone {
                                counts[(c.shade / 4).min(3) as usize] += 1.0;
                                total += 1.0;
                            }
                        }
                    }
                    if total >= (BLOCK * BLOCK) as f32 * 0.75 {
                        mix[(by * bw + bx) as usize] = Some(counts.map(|c| c / total));
                    }
                }
            }
            // L1 distance between two blocks' family mixes, averaged over
            // every adjacent pair that has both.
            let delta = |a: &[f32; 4], b: &[f32; 4]| (0..4).map(|i| (a[i] - b[i]).abs()).sum::<f32>();
            let (mut sv, mut nv, mut sh, mut nh) = (0.0f32, 0usize, 0.0f32, 0usize);
            for by in 0..bh {
                for bx in 0..bw {
                    let Some(here) = mix[(by * bw + bx) as usize] else { continue };
                    if by + 1 < bh {
                        if let Some(down) = mix[((by + 1) * bw + bx) as usize] {
                            sv += delta(&here, &down);
                            nv += 1;
                        }
                    }
                    if bx + 1 < bw {
                        if let Some(right) = mix[(by * bw + bx + 1) as usize] {
                            sh += delta(&here, &right);
                            nh += 1;
                        }
                    }
                }
            }
            let (dv, dh) = (sv / nv.max(1) as f32, sh / nh.max(1) as f32);
            // Beside the shape number, how much regional identity is left:
            // the share of all stone taken by its most common family. The
            // point of families is that `character(x)` picks them, so a field
            // strong enough to drive this toward an even 25/25/25/25 has
            // dissolved the country it was meant to make legible. v/h = 1 is
            // therefore NOT the target -- some horizontal anisotropy is the
            // signal, and only the part pinned to a column is the artifact.
            let mut tot = [0f32; 4];
            for m in mix.iter().flatten() {
                for i in 0..4 {
                    tot[i] += m[i];
                }
            }
            let sum: f32 = tot.iter().sum();
            let top = tot.iter().cloned().fold(0.0f32, f32::max) / sum.max(1e-6);
            println!(
                "  {preset:>10} {seed:>5} {field:>8.2} {dv:>9.4} {dh:>9.4} {:>7.2} {top:>9.2}",
                dv / dh.max(1e-6)
            );
        }
    }
}
