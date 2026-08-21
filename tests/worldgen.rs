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
    // Re-measured for round-4 task 4 (`world_age` on by default): erosion's
    // sediment fill deepens some ponds, and a bigger pond's sub-cell fill
    // takes the liquid solver a few more frames to level -- positions still
    // hold from frame one (`generated_terrain_is_already_at_rest`), this is
    // only the fill converging. Worst was rolling at 50 frames (wetland 47,
    // both up from the pre-erosion 45 bar); 70 keeps meaningful headroom
    // while staying well inside the 120-frame loop cap and the "within a
    // second" claim this test is named for.
    assert!(
        worst.0 <= 70,
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
        // `vaults` is the one pass that legitimately writes nothing here, and
        // the reason is the world *size* rather than the pass: a chamber must
        // sit `vault_min_depth` (200) rows below the surface and stop
        // `vault_bedrock_margin` (16) above bedrock, and at 512x320 -- surface
        // around y 100-200, bedrock around y 300 -- that band is empty. The
        // shipped world is 2048x640, where it fits. Asserting it separately
        // below rather than excusing it keeps the guard's teeth: if the pass
        // stops firing at the shipped size too, something still fails.
        if *name == "vaults" {
            assert_eq!(*cells, 0, "vaults placed a chamber at 512x320, where the depth band should be empty");
            continue;
        }
        // `boulders` legitimately writes nothing here too, for a different
        // reason from `vaults`'s size gap: it reads `erosion::Deposits`,
        // which is a guaranteed no-op at `world_age == 0` -- the default
        // every preset ships with until round-4 task 4 flips per-preset
        // ages on. Asserted zero here rather than skipped, and fired
        // separately below with `world_age` forced, for the same reason
        // `vaults` gets its own assertion: an exclusion that stops being
        // true should fail loudly, not silently stop checking.
        if *name == "boulders" {
            assert_eq!(*cells, 0, "boulders placed a cluster at world_age 0.0, which erosion must no-op at");
            continue;
        }
        assert!(*cells > 0, "pass {name} never wrote a cell across {} seeds", SEEDS.len());
    }
    // The other half: at the size the game actually ships, vaults fire.
    let mut vault_cells = 0;
    for seed in SEEDS {
        let mut world = World::new(Rect::new(0, 0, REVIEW_BOUNDS.0, REVIEW_BOUNDS.1));
        for (name, cells) in worldgen::generate_reported(&mut world, Spec::Generated { params, seed }) {
            if name == "vaults" {
                vault_cells += cells;
            }
        }
    }
    assert!(vault_cells > 0, "vaults never wrote a cell across {} seeds at the shipped 2048x640", SEEDS.len());

    // `boulders`'s own "the pass fires somewhere" half lives in
    // `a_forced_boulder_world_seats_stone_and_arrives_at_rest` rather than
    // here: unlike vaults (a size axis) it needs a much wider seed sweep to
    // catch a real success (see that test's comment), which is too slow to
    // repeat inside every preset this function already covers.
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
    //
    // **Summed over `SEEDS` rather than read off seed 1**, since round-4
    // task 4. Turning erosion on by default made this the chaotic-seed
    // trap CLAUDE.md's sweep rule names directly: seed 1's *legacy* talus
    // count (the isolated per-column heap-and-apron heuristic this pass
    // has always been) collapsed to single digits at several presets --
    // not because cliffs disappeared, but because `soil_blanket` now folds
    // erosion's own talus deposit into the same cells this heuristic wants
    // to write into first (`column.rs`'s `extra_cover`), so an apron
    // erosion already placed leaves nothing open for the legacy heap to
    // add. Seeds 2, 4, 5, 7 land in the hundreds for the same presets --
    // seed 1 was never representative, it was the one seed nobody had
    // re-measured after erosion started supplying the same picture a
    // different way. A sum over five seeds is stable where any one of them
    // is not.
    let presets = presets();
    for preset in ["rolling", "canyon", "terraced"] {
        let params = presets.get(preset).expect("preset");
        let (mut talus, mut brows) = (0usize, 0usize);
        let (mut talus_at_cliff, mut brows_at_cliff) = (0usize, 0usize);
        for seed in SEEDS {
            let world = build(params, seed);
            let no_talus = build(&WorldgenParams { talus_max_height: 0.0, ..params.clone() }, seed);
            let no_brows = build(&WorldgenParams { brow_chance: 0.0, ..params.clone() }, seed);

            // The plan's ground line, taken from the world with neither
            // formation, so an apron does not count as the ground it rests on.
            let bare = build(
                &WorldgenParams { talus_max_height: 0.0, brow_chance: 0.0, ..params.clone() },
                seed,
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

            let (mut seed_talus, mut seed_brows) = (0usize, 0usize);
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
                        seed_talus += 1;
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
                        seed_brows += 1;
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
                "{preset} seed {seed}: talus {seed_talus} cells, brows {seed_brows} cells; tallest heap {} cells at x {}",
                tallest.1, tallest.0
            );
        }
        println!("{preset} over {} seeds: talus {talus} total ({talus_at_cliff} near a cliff), brows {brows} total ({brows_at_cliff} over a drop)", SEEDS.len());
        // Bars set from the measurement with headroom, not from an
        // aspiration and not sitting on the measured value. Summed over
        // `SEEDS`, the weakest of the three presets post-erosion is
        // terraced at 547 talus cells and 1,438 brow cells (rolling 1,006 /
        // 2,186; canyon 1,528 / 3,825) -- 100 leaves 5x headroom on the
        // weakest total while staying far above the single-seed 0-to-34
        // range that was the state this rescue was written for.
        assert!(talus > 100, "{preset}: talus wrote only {talus} cells over {} seeds -- the rescue did not hold", SEEDS.len());
        assert!(brows > 100, "{preset}: brows wrote only {brows} cells over {} seeds -- the rescue did not hold", SEEDS.len());
        // The claim that matters. Ninety per cent rather than all of it:
        // an apron tapers away from its face by design, and its far toe can
        // legitimately sit past the window.
        assert!(
            talus_at_cliff * 10 >= talus * 9,
            "{preset}: only {talus_at_cliff} of {talus} talus cells are near any cliff at all -- \
             the detector is firing on flat ground"
        );
        // Brows' bar is 80%, not 90%: erosion's longer, gentler
        // escarpments (the far-scale detection's "gentler but tall" case,
        // now common post-erosion) spread brows across stretches long
        // enough that this cell-local `relief` window (+/-20 columns) does
        // not always catch the qualifying drop `cliff_edges` used the whole
        // escarpment to find. Measured over `SEEDS`: rolling 84.1%
        // (weakest), terraced 90.5%, canyon 99.9%.
        assert!(
            brows_at_cliff * 10 >= brows * 8,
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

    // The third population, since round-4 task 2: erosion's talus deposit
    // also recolours the top of a column's cover as buried-family gravel
    // (`passes.rs::soil_blanket`), independently of `pocket_density` -- so
    // "present with pockets off too" is no longer purely scree and soil
    // contact on a preset old enough to carry `world_age`, which `canyon`
    // now does by default. Recomputed the same way
    // `erosion_talus_draws_as_buried_gravel_at_the_top_of_the_cover` does,
    // through the public plan-side API rather than by inferring position --
    // the classifier lesson this test's own comment names, applied to the
    // new population instead of the old one.
    use pixel_physics::worldgen::column::Terrain;
    let soil = world.materials.id_of("soil").expect("soil");
    let sand_id = world.materials.id_of("sand").expect("sand");
    let soil_tan = world.materials.get(soil).friction_angle.to_radians().tan();
    let sand_tan = world.materials.get(sand_id).friction_angle.to_radians().tan();
    let terrain = Terrain::new(1, params, BOUNDS.0 + 1, BOUNDS.1 + 1, soil_tan, sand_tan);
    let (plans, deposits) = terrain.plan_all_with_deposits();
    let is_talus_cell = |x: i32, y: i32| -> bool {
        let talus = deposits.talus[x as usize];
        if talus < 1.0 {
            return false;
        }
        let plan = plans[x as usize];
        let talus_cells = (talus.round() as i32).min(plan.soil_depth);
        let top = plan.surface_y.max(0);
        y >= top && y < top + talus_cells
    };

    let (mut lens, mut other, mut talus) = (0usize, 0usize, 0usize);
    let (mut lens_wrong, mut other_wrong, mut talus_wrong) = (0usize, 0usize, 0usize);
    for y in 0..=BOUNDS.1 {
        for x in 0..=BOUNDS.0 {
            let c = world.get(x, y);
            if c.material != gravel {
                continue;
            }
            if without.get(x, y).material != gravel {
                lens += 1;
                if c.shade / 4 != 1 {
                    lens_wrong += 1;
                }
            } else if is_talus_cell(x, y) {
                // Rockfall, recoloured at the top of the cover: buried
                // family too, same reasoning as a lens (`passes.rs`'s
                // `soil_blanket`).
                talus += 1;
                if c.shade / 4 != 1 {
                    talus_wrong += 1;
                }
            } else {
                // Present with the pass off too and not a talus deposit:
                // plain scree, or the soil profile's stony contact. Must
                // stay in the scree family.
                other += 1;
                if c.shade / 4 != 0 {
                    other_wrong += 1;
                }
            }
        }
    }
    println!("canyon seed 1 gravel: {lens} cells in sealed lenses, {talus} in talus deposits, {other} in scree and soil contact");
    assert_eq!(talus_wrong, 0, "{talus_wrong} of {talus} talus-deposit cells are in the scree family, not buried -- invisible against the rock behind them");

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
fn erosion_talus_draws_as_buried_gravel_at_the_top_of_the_cover() {
    // Round-4 task 2. Checked by recomputing `Deposits::talus` independently
    // through the public plan-side API and confirming the realise side put
    // gravel exactly where the plan says rockfall landed -- not by turning
    // erosion off and diffing (world_age reshapes the whole surface, so a
    // paired diff the way `buried_gravel_is_not_the_same_colour_as_scree`
    // does it would be swamped by every other thing erosion moves).
    //
    // `world_age` forced well past any shipped preset: the tuning record
    // (`Reports/worldgen-erosion-design.md`) measured only 1-5 boulder
    // markers and comparable talus on the 2048-column probe world at age
    // 1.0, and this suite runs at the 512-column harness size -- forcing the
    // age is what gives the guard something to see, per the r2/r3 lesson
    // that a guard which cannot see the feature has no teeth.
    use pixel_physics::worldgen::column::Terrain;
    let presets = presets();
    let base = presets.get("rolling").expect("rolling preset");
    let params = WorldgenParams { world_age: 6.0, tree_density: 0.0, moss_density: 0.0, ..base.clone() };

    let mut total_talus_cells = 0usize;
    let mut wrong_family = 0usize;
    for seed in SEEDS {
        let world = build(&params, seed);
        let gravel = world.materials.id_of("gravel").expect("gravel");
        let soil = world.materials.id_of("soil").expect("soil");
        let sand = world.materials.id_of("sand").expect("sand");
        let soil_tan = world.materials.get(soil).friction_angle.to_radians().tan();
        let sand_tan = world.materials.get(sand).friction_angle.to_radians().tan();
        let terrain = Terrain::new(seed, &params, BOUNDS.0 + 1, BOUNDS.1 + 1, soil_tan, sand_tan);
        let (plans, deposits) = terrain.plan_all_with_deposits();

        for (x, plan) in plans.iter().enumerate() {
            let talus = deposits.talus[x];
            if talus < 1.0 {
                continue;
            }
            let talus_cells = (talus.round() as i32).min(plan.soil_depth);
            let top = plan.surface_y.max(0);
            for depth in 0..talus_cells {
                let (px, py) = (x as i32, top + depth);
                let c = world.get(px, py);
                if c.material != gravel {
                    // Legitimate: the soil/stone dithered contact can also
                    // claim a cell inside the talus band on a shallow
                    // column, and that draw is untouched by this feature.
                    continue;
                }
                total_talus_cells += 1;
                if c.shade / 4 != 1 {
                    wrong_family += 1;
                }
            }
        }
    }
    println!("rolling age 6.0, seeds {SEEDS:?}: {total_talus_cells} talus cells drawn as gravel");
    assert!(total_talus_cells > 0, "no column had talus >= 1 realise as gravel -- the pass never fired");
    assert_eq!(
        wrong_family, 0,
        "{wrong_family} of {total_talus_cells} talus-gravel cells are in the scree family, not buried -- invisible against the rock behind them"
    );
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

#[test]
#[ignore = "probe: prints, never asserts (round-2 task 3)"]
fn probe_r2t3_do_vaults_place_at_all() {
    // "Did it fire at all needs a counter, not a picture" -- and a vault is
    // the extreme case of that rule, because the whole feature is *invisible*
    // by design. A render can never show one until someone digs to it, so a
    // pass that silently placed nothing would look exactly like a pass that
    // worked. Print the count, the depth band it had to fit in, and the
    // rejection reason when it fails.
    let presets = presets();
    println!("\n=== vault placement at the shipped 2048x640 ===");
    println!("  {:>10} {:>5} {:>8} {:>10} {:>10} {:>9}", "preset", "seed", "cells", "crystal", "gravel", "water");
    for preset in ["rolling", "terraced", "canyon", "wetland", "arid"] {
        let params = presets.get(preset).expect("preset");
        for seed in 1u64..=8 {
            let mut world = World::new(Rect::new(0, 0, REVIEW_BOUNDS.0, REVIEW_BOUNDS.1));
            let counts = worldgen::generate_reported(&mut world, Spec::Generated { params, seed });
            let cells = counts.iter().find(|(n, _)| *n == "vaults").map(|(_, c)| *c).unwrap_or(0);
            let id = |n: &str| world.materials.id_of(n).expect(n);
            let (crystal, gravel, water) = (id("crystal"), id("gravel"), id("water"));
            // Census only well below the surface, so surface gravel and pond
            // water cannot be mistaken for a chamber's contents.
            let deep = REVIEW_BOUNDS.1 / 2;
            let (mut nc, mut ng, mut nw) = (0, 0, 0);
            for y in deep..=REVIEW_BOUNDS.1 {
                for x in 0..=REVIEW_BOUNDS.0 {
                    let m = world.get(x, y).material;
                    if m == crystal {
                        nc += 1;
                    } else if m == gravel {
                        ng += 1;
                    } else if m == water {
                        nw += 1;
                    }
                }
            }
            println!("  {preset:>10} {seed:>5} {cells:>8} {nc:>10} {ng:>10} {nw:>9}");
        }
    }
}

/// A preset that is guaranteed to place chambers at the 512x320 test size.
///
/// **`vault_min_depth` has to come down for this, and that is a fact about
/// the world size rather than a convenience.** At 512x320 the surface sits
/// around y 100-200 and bedrock around y 300, so the shipped 200-row depth
/// band and the 16-row bedrock margin do not both fit: the band is empty and
/// no chamber can be placed at all. The shipped size is 2048x640, where it
/// fits comfortably. See the round-2 finding -- this is also why the sweep's
/// `vaults` row reads zero.
fn vault_test_params(base: &WorldgenParams) -> WorldgenParams {
    WorldgenParams {
        vault_density: 4.0,
        vault_min_depth: 40,
        tree_density: 0.0,
        moss_density: 0.0,
        ..base.clone()
    }
}

#[test]
fn a_forced_vault_world_is_sealed_and_arrives_at_rest() {
    // Three claims about the vault pass in one world, because they share an
    // expensive build: it fires, every cell it wrote was solid stone before
    // it wrote there (the seal contract), and the result holds still.
    //
    // The instrument is a **paired build** against the same world with
    // `vault_density: 0.0` -- the repo's preferred shape, and here it is
    // exact rather than merely better: the vault pass writes nothing unless
    // it writes a whole chamber, and nothing downstream of it reads a vault,
    // so every difference between the two worlds is a vault cell and no
    // difference is anything else. Inferring which cells are vault cells from
    // *where they are* is the mistake that miscounted twice in round 1's
    // task 4.
    let presets = presets();
    let mut checked = 0;
    for preset in ["rolling", "canyon", "wetland"] {
        let base = presets.get(preset).expect("preset");
        let with = vault_test_params(base);
        let without = WorldgenParams { vault_density: 0.0, ..with.clone() };
        for seed in SEEDS {
            let world = build(&with, seed);
            let control = build(&without, seed);
            let stone = world.materials.id_of("stone").expect("stone");

            let mut vault_cells = Vec::new();
            for y in 0..=BOUNDS.1 {
                for x in 0..=BOUNDS.0 {
                    if world.get(x, y).material != control.get(x, y).material
                        || world.get(x, y).shade != control.get(x, y).shade
                    {
                        vault_cells.push((x, y));
                    }
                }
            }
            if vault_cells.is_empty() {
                continue;
            }
            checked += 1;

            // The seal, stated as the contract states it: every cell of the
            // envelope was stone. Checked on the *control* world, which is
            // what "before the pass ran" means.
            for &(x, y) in &vault_cells {
                assert_eq!(
                    control.get(x, y).material,
                    stone,
                    "{preset} seed {seed}: vault wrote ({x}, {y}), which was not stone before"
                );
            }
            // And no vault cell touches anything that was not stone -- which
            // is the half that actually keeps the chamber sealed. A chamber
            // whose own cells were all stone can still be flush against a
            // pre-existing void one cell beyond its edge.
            let changed: std::collections::HashSet<(i32, i32)> = vault_cells.iter().copied().collect();
            for &(x, y) in &vault_cells {
                for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if changed.contains(&(nx, ny)) {
                        continue;
                    }
                    assert_eq!(
                        control.get(nx, ny).material,
                        stone,
                        "{preset} seed {seed}: vault cell ({x}, {y}) is flush against ({nx}, {ny}), which was not stone"
                    );
                }
            }

            // And it holds still. The floor is loose gravel over a void's
            // curved bottom, and the chamber may hold standing water, so this
            // is the claim most likely to break if the floor stops being
            // filled flat.
            let mut world = world;
            let before: std::collections::HashSet<_> = snapshot(&world).into_iter().collect();
            for _ in 0..120 {
                step(&mut world);
            }
            let after: std::collections::HashSet<_> = snapshot(&world).into_iter().collect();
            let gone: Vec<_> = before.difference(&after).copied().collect();
            assert!(
                gone.is_empty(),
                "{preset} seed {seed}: {} cells left their position in a forced-vault world; first {:?}",
                gone.len(),
                gone.iter().take(6).collect::<Vec<_>>()
            );
        }
    }
    // The counter beside the claim: a suite where no world grew a chamber
    // would pass every assertion above by never running one.
    assert!(checked >= 8, "only {checked} forced-vault worlds actually placed a chamber");
}

#[test]
fn a_cave_system_survives_a_pocket_lens_inside_its_envelope() {
    // Round-5 task 1's own reproduction. Before this fix, a `pockets` lens
    // landing inside or against a cave system's envelope deleted the *whole
    // system* -- measured (see the round-5 task file's addendum): every
    // wholesale rejection across canyon/rolling/wetland was a single stray
    // `sand` or `gravel` cell. `pocket_density` cranked to 20 (the shipped
    // default is 0.6) saturates the deep massif with lenses, so one lands
    // inside a cave envelope in every seed here rather than waiting for a
    // seed that happens to produce it -- and the "lens nearby" count below
    // is what proves the collision actually happened, so this cannot pass
    // by having gotten lucky and never meeting one.
    let presets = presets();
    let base = presets.get("rolling").expect("preset");
    let with = WorldgenParams { pocket_density: 20.0, ..vault_test_params(base) };
    let without = WorldgenParams { vault_density: 0.0, ..with.clone() };
    let mut placed = 0;
    let mut overlapped = 0;
    for seed in SEEDS {
        let mut world = build(&with, seed);
        let control = build(&without, seed);
        let mut carved: Vec<(i32, i32)> = Vec::new();
        for y in 0..=BOUNDS.1 {
            for x in 0..=BOUNDS.0 {
                if world.get(x, y).material != control.get(x, y).material
                    || world.get(x, y).shade != control.get(x, y).shade
                {
                    carved.push((x, y));
                }
            }
        }
        if carved.is_empty() {
            continue;
        }
        placed += 1;

        // Evidence the reproduction is real: sand/gravel within a
        // Chebyshev-6 dilation of the carved envelope, read off the
        // *control* world -- no cave carving happened there at all, so this
        // counts what `pockets` left behind on its own, not anything the
        // vault pass wrote.
        let (x0, x1) = (
            carved.iter().map(|&(x, _)| x).min().unwrap() - 6,
            carved.iter().map(|&(x, _)| x).max().unwrap() + 6,
        );
        let (y0, y1) = (
            carved.iter().map(|&(_, y)| y).min().unwrap() - 6,
            carved.iter().map(|&(_, y)| y).max().unwrap() + 6,
        );
        let (sand, gravel) = (
            control.materials.id_of("sand").expect("sand"),
            control.materials.id_of("gravel").expect("gravel"),
        );
        let mut lens_cells = 0;
        for y in y0.max(0)..=y1.min(BOUNDS.1) {
            for x in x0.max(0)..=x1.min(BOUNDS.0) {
                let m = control.get(x, y).material;
                if m == sand || m == gravel {
                    lens_cells += 1;
                }
            }
        }
        assert!(
            lens_cells > 0,
            "seed {seed}: no pocket lens fell near the carved system -- this seed proves nothing"
        );
        overlapped += 1;

        // And it still arrives at rest, exactly like the plain seal test --
        // this is the same claim, just under a reproduction that used to
        // delete the system outright rather than merely stress it.
        let before: std::collections::HashSet<_> = snapshot(&world).into_iter().collect();
        for _ in 0..120 {
            step(&mut world);
        }
        let after: std::collections::HashSet<_> = snapshot(&world).into_iter().collect();
        let gone: Vec<_> = before.difference(&after).copied().collect();
        assert!(
            gone.is_empty(),
            "seed {seed}: {} cells left their position in a lens-stressed cave world; first {:?}",
            gone.len(),
            gone.iter().take(6).collect::<Vec<_>>()
        );
    }
    // The counters beside the claim: a run where nothing placed, or placed
    // without ever actually meeting a lens, would pass vacuously.
    assert!(placed >= 3, "only {placed}/{} lens-stressed worlds placed a system at all", SEEDS.len());
    assert_eq!(overlapped, placed, "every placed system should have a lens nearby at this density");
}

#[test]
fn vault_water_cannot_wet_the_massif_around_it() {
    // Stated as a test because the task asks for it stated: water sealed in a
    // chamber is moisture-inert. The reason is structural rather than lucky
    // -- `soil_moisture` only ever *writes* to cells with a non-zero
    // `water_capacity`, and soil is the only material that has one, so a
    // chamber 40+ rows into solid rock has nothing within reach it could wet
    // even though its water does seed the distance transform.
    let presets = presets();
    let base = presets.get("wetland").expect("preset");
    let with = vault_test_params(base);
    let without = WorldgenParams { vault_density: 0.0, ..with.clone() };
    for seed in SEEDS {
        let world = build(&with, seed);
        let control = build(&without, seed);
        let soil = world.materials.id_of("soil").expect("soil");
        for y in 0..=BOUNDS.1 {
            for x in 0..=BOUNDS.0 {
                let c = world.get(x, y);
                if c.material == soil {
                    assert_eq!(
                        c.aux(),
                        control.get(x, y).aux(),
                        "seed {seed}: soil at ({x}, {y}) changed saturation because a vault was placed"
                    );
                }
            }
        }
    }
}

#[test]
fn a_world_with_no_vaults_is_byte_identical() {
    // The opt-out has to be exact, not approximate: `flat` ships
    // `vault_density: 0.0` because the destruction workstream compares
    // against its renders, and "almost the same world" would be a silent
    // change under someone else's baseline.
    let presets = presets();
    for preset in ["rolling", "canyon", "flat"] {
        let base = presets.get(preset).expect("preset");
        let off = WorldgenParams { vault_density: 0.0, ..base.clone() };
        for seed in SEEDS {
            // Deliberately built at the size where the shipped depth band is
            // empty anyway, so this also pins that fact: at 512x320 the two
            // must agree even at the shipped density.
            let shipped = build(base, seed);
            let disabled = build(&off, seed);
            assert_eq!(
                world_hash(&shipped),
                world_hash(&disabled),
                "{preset} seed {seed}: vault_density changed a 512x320 world, where the depth band is empty"
            );
        }
    }
}

#[test]
#[ignore = "probe: prints, never asserts (round-2 task 3)"]
fn probe_r2t3_dump_a_chamber() {
    // ASCII cross-sections, one per shape. This is the instrument a render
    // cannot replace for this feature: a chamber is 40-60 cells across in a
    // 2048-wide world, so at 1:1 it is a smudge, and what has to be checked
    // is *structure* -- where the waterline sits, whether the gravel floor is
    // flat, whether the lining is a ring rather than a blob.
    let presets = presets();
    let base = presets.get("rolling").expect("preset");
    let with = vault_test_params(base);
    for (label, want_crystal) in [("geode vug (crystal-lined)", true), ("grotto (no lining)", false)] {
        let mut shown = false;
        for seed in 1u64..=12 {
            if shown {
                break;
            }
            let world = build(&with, seed);
            let id = |n: &str| world.materials.id_of(n).expect(n);
            let (stone, gravel, water, crystal) = (id("stone"), id("gravel"), id("water"), id("crystal"));
            let glyph = |x: i32, y: i32| {
                let c = world.get(x, y);
                if c.material == material::EMPTY {
                    '.'
                } else if c.material == stone {
                    '#'
                } else if c.material == gravel {
                    'g'
                } else if c.material == water {
                    '~'
                } else if c.material == crystal {
                    'X'
                } else {
                    '?'
                }
            };
            // A chamber is found as a run of air deep in the rock -- the one
            // thing `pockets` can never produce, since a lens is solid. Its
            // shape is then read off whether any crystal is within reach.
            for y in 220..=(BOUNDS.1 - 40) {
                if shown {
                    break;
                }
                for x in 40..=(BOUNDS.0 - 40) {
                    if world.get(x, y).material != material::EMPTY {
                        continue;
                    }
                    let has_crystal = (-34..=34).any(|dy| {
                        (-40..=40).any(|dx| world.get(x + dx, y + dy).material == crystal)
                    });
                    if has_crystal != want_crystal {
                        continue;
                    }
                    println!("\n=== {label}: seed {seed}, air at ({x}, {y}) ===");
                    for row in (y - 8).max(0)..=(y + 34).min(BOUNDS.1) {
                        let line: String =
                            ((x - 40).max(0)..=(x + 40).min(BOUNDS.0)).map(|c| glyph(c, row)).collect();
                        println!("  {row:>4} {line}");
                    }
                    shown = true;
                    break;
                }
            }
        }
        if !shown {
            println!("\n=== {label}: none found in seeds 1..12 ===");
        }
    }
}

#[test]
#[ignore = "probe: prints, never asserts (round-2 task 3)"]
fn probe_r2t3_what_moves_in_a_vault() {
    // Reproduce before fixing: step one frame at a time and name the first
    // cell that changes, with its neighbourhood. A 120-frame at-rest failure
    // says only that the world moved.
    let presets = presets();
    let base = presets.get("rolling").expect("preset");
    let with = vault_test_params(base);
    let mut world = build(&with, 1);
    let id = |n: &str| world.materials.id_of(n).expect(n);
    let (stone, gravel, water, crystal) = (id("stone"), id("gravel"), id("water"), id("crystal"));
    let name = move |m: material::MaterialId| {
        if m == material::EMPTY {
            "empty"
        } else if m == stone {
            "stone"
        } else if m == gravel {
            "gravel"
        } else if m == water {
            "water"
        } else if m == crystal {
            "crystal"
        } else {
            "other"
        }
    };
    let cells = |w: &World| {
        let mut v = Vec::new();
        for y in 0..=BOUNDS.1 {
            for x in 0..=BOUNDS.0 {
                let c = w.get(x, y);
                v.push((c.material, c.aux()));
            }
        }
        v
    };
    let mut prev = cells(&world);
    for frame in 0..12 {
        step(&mut world);
        let now = cells(&world);
        let mut changes = Vec::new();
        for (i, (a, b)) in prev.iter().zip(now.iter()).enumerate() {
            if a != b {
                let (x, y) = ((i as i32) % (BOUNDS.0 + 1), (i as i32) / (BOUNDS.0 + 1));
                changes.push((x, y, a.0, a.1, b.0, b.1));
            }
        }
        // Only the deep ones: the surface has ponds and weather doing
        // legitimate work, and this probe is about the chamber.
        changes.retain(|&(_, y, ..)| y > 200);
        println!("frame {frame}: {} deep cells changed", changes.len());
        for &(x, y, am, aa, bm, ba) in changes.iter().take(6) {
            println!("    ({x},{y}) {} aux {aa} -> {} aux {ba}", name(am), name(bm));
        }
        if !changes.is_empty() {
            break;
        }
        prev = now;
    }
}

/// Round-3 guard: the ceiling-span bound, walked over every roof run of a
/// forced-generation world.
///
/// The pass promises no horizontal run of carved void with rock directly
/// above it is longer than 36 cells -- the roof span the round-2 arithmetic
/// cleared for chambers -- and enforces it by dropping stone teeth from any
/// longer run (`passes::carve_cave_void`). The instrument is the same paired
/// build the seal test uses, so attribution is exact: an *open* cell is a
/// carved cell that is not solid afterwards (air, water, floor gravel), and
/// a ceiling run is a maximal horizontal run of open cells each with
/// unbroken material directly above. The 36 is restated here as a literal
/// because the pass's constant is private, and this test must fail if that
/// copy ever drifts upward.
#[test]
fn a_forced_cave_world_keeps_every_roof_span_bounded() {
    const MAX_CEILING_SPAN: i32 = 36;
    let presets = presets();
    let base = presets.get("rolling").expect("preset");
    let with = vault_test_params(base);
    let without = WorldgenParams { vault_density: 0.0, ..with.clone() };
    let mut systems = 0usize;
    let mut runs_walked = 0usize;
    for seed in SEEDS {
        let world = build(&with, seed);
        let control = build(&without, seed);
        let mut carved = std::collections::HashSet::new();
        for y in 0..=BOUNDS.1 {
            for x in 0..=BOUNDS.0 {
                if world.get(x, y).material != control.get(x, y).material
                    || world.get(x, y).shade != control.get(x, y).shade
                {
                    carved.insert((x, y));
                }
            }
        }
        if carved.is_empty() {
            continue;
        }
        systems += 1;
        let open = |x: i32, y: i32| {
            carved.contains(&(x, y))
                && world.materials.kind(world.get(x, y).material) != material::MaterialKind::Solid
        };
        for y in 0..=BOUNDS.1 {
            let mut run = 0;
            for x in 0..=BOUNDS.0 + 1 {
                let ceiling = x <= BOUNDS.0 && open(x, y) && !open(x, y - 1);
                if ceiling {
                    run += 1;
                } else {
                    if run > 0 {
                        runs_walked += 1;
                    }
                    assert!(
                        run <= MAX_CEILING_SPAN,
                        "seed {seed}: a {run}-cell roof span ends at ({x}, {y}) -- the ceiling guard let it through"
                    );
                    run = 0;
                }
            }
        }
    }
    // The counters beside the claim: a sweep where no system placed, or one
    // whose systems had no ceilings to walk, would pass vacuously.
    assert!(systems >= 3, "only {systems} forced worlds placed a system");
    assert!(runs_walked >= 50, "only {runs_walked} ceiling runs walked -- the census is too thin to trust");
}

/// Round-3 guard: a world with a real cave system in it is reproducible.
///
/// The suite's existing determinism tests build at sizes where the vault
/// pass never fires, so none of them exercises the carve, the floor
/// verifier's fixpoint, the waterline or the speleothem draws -- the r2
/// lesson that a guard that cannot see the feature has no teeth. This one
/// hashes a forced-generation world (which the placement counter proves
/// contains systems) twice, same build, same seed.
#[test]
fn a_forced_cave_world_is_deterministic() {
    let presets = presets();
    let base = presets.get("rolling").expect("preset");
    let with = vault_test_params(base);
    let mut placed = 0;
    for seed in [1u64, 4] {
        let mut a = World::new(Rect::new(0, 0, BOUNDS.0, BOUNDS.1));
        let counts = worldgen::generate_reported(&mut a, Spec::Generated { params: &with, seed });
        placed += counts.iter().find(|(n, _)| *n == "vaults").map(|(_, c)| *c).unwrap_or(0);
        // `generate_reported` skips the structural pass `build` runs, and the
        // hash covers aux, so run it here or the comparison measures that
        // difference instead of generation.
        structural::compute_world_distances(&mut a);
        let b = build(&with, seed);
        assert_eq!(world_hash(&a), world_hash(&b), "seed {seed}: two builds of a forced-cave world differ");
    }
    assert!(placed > 0, "vacuous: neither forced world placed a system");
}

/// Round-3 guard, amended by round-5 task 4c: a speleothem may narrow a
/// passage, never close it -- with exactly one deliberate exception per
/// system.
///
/// A column of rock from floor to ceiling splits the passage the player
/// walks, so the pass promises every column it decorates keeps at least one
/// open cell (a pair closes to a one-or-two-cell gap on purpose, which
/// still satisfies this) -- **except the single fused column task 4c
/// places inside the largest chamber run, which is deliberately solid
/// floor-to-ceiling** because a chamber is not a passage and blocks no
/// route through it. The guard now allows *at most one* fully-solid column
/// per system rather than none; a second one, or one outside a chamber,
/// would be the bridging bug this test was written to catch, wearing task
/// 4c's exception as cover, so the bound stays tight rather than being
/// dropped. Attribution is the paired build again: in a cave system's
/// diff, the only *solid* carved cells are speleothems -- the ceiling
/// guard's teeth are never written, so they never enter the diff -- and
/// vugs are excluded by component size, because a vug's crystal ring
/// legitimately fills its rim columns.
#[test]
fn speleothems_never_bridge_a_passage() {
    // **Diff-free, unlike every other round-2/3 vault guard**, and that
    // change is load-bearing, not a style choice. The paired-build diff
    // (`with` vs `vault_density: 0.0`) those guards share has a leak this
    // test's stricter per-run check was the first to actually trip over:
    // turning vaults on changes the *shade* of some ordinary, untouched
    // wall stone elsewhere in the world (measured -- a probe dump found
    // hundreds of such cells per world, material unchanged, only the tone
    // byte differing, at locations with no carved void anywhere near
    // them). Root cause not chased down; every shade-producing function
    // read is a pure function of `(seed, x, y)` with no dependence on
    // `vault_density`, so the leak is somewhere upstream of this pass, not
    // in it. Whatever it is, it inflates the "carved" set with cells the
    // vault pass never touched, and column-diffing a batch of ordinary
    // stone that happens to all read as `Solid` reads exactly like a
    // bridged passage.
    //
    // The fix is to stop asking the control world at all. A "formation" is
    // read back the same way `cave_probe` already reads one: a solid cell
    // with void on both horizontal sides, inside a flood-filled void
    // component found directly in the one world under test. Nothing here
    // depends on a second build being byte-identical anywhere it should
    // not differ.
    let presets = presets();
    let base = presets.get("rolling").expect("preset");
    let with = vault_test_params(base);
    let mut speleo_cells = 0usize;
    let mut columns_checked = 0usize;
    let mut systems_checked = 0usize;
    let mut fused_columns_total = 0usize;
    for seed in SEEDS {
        let world = build(&with, seed);
        let is_void = |x: i32, y: i32| {
            if x < 0 || x > BOUNDS.0 || y < 0 || y > BOUNDS.1 {
                return false;
            }
            let c = world.get(x, y);
            c.material == material::EMPTY
                || world.materials.kind(c.material) == material::MaterialKind::Liquid
        };
        let mut seen: std::collections::HashSet<(i32, i32)> = Default::default();
        for y0 in 0..=BOUNDS.1 {
            for x0 in 0..=BOUNDS.0 {
                if seen.contains(&(x0, y0)) || !is_void(x0, y0) {
                    continue;
                }
                let mut comp = Vec::new();
                let mut stack = vec![(x0, y0)];
                seen.insert((x0, y0));
                while let Some((x, y)) = stack.pop() {
                    comp.push((x, y));
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let n = (x + dx, y + dy);
                            if !seen.contains(&n) && is_void(n.0, n.1) {
                                seen.insert(n);
                                stack.push(n);
                            }
                        }
                    }
                }
                // Vugs are a few hundred cells; systems are thousands.
                if comp.len() < 1000 {
                    continue;
                }
                systems_checked += 1;
                let void_here: std::collections::HashSet<(i32, i32)> = comp.iter().copied().collect();
                let (x0b, x1b) = (
                    comp.iter().map(|c| c.0).min().unwrap(),
                    comp.iter().map(|c| c.0).max().unwrap(),
                );
                let (y0b, y1b) = (
                    comp.iter().map(|c| c.1).min().unwrap(),
                    comp.iter().map(|c| c.1).max().unwrap(),
                );
                // A formation cell, exactly as `cave_probe` reads one:
                // solid, and free-standing -- void on both flanks at the
                // same row. `void_here.contains` on both sides is what
                // keeps this from also matching the massif's own wall.
                let is_form = |x: i32, y: i32| {
                    let m = world.get(x, y).material;
                    world.materials.kind(m) == material::MaterialKind::Solid
                        && !void_here.contains(&(x, y))
                        && void_here.contains(&(x - 1, y))
                        && void_here.contains(&(x + 1, y))
                };
                let mut fused_here = 0usize;
                for x in x0b..=x1b {
                    // A column can carry more than one *run* -- round-5
                    // task 4b decorates every void run in a column, not
                    // only the bottommost, so the same x can revisit the
                    // passage network at a completely unrelated y
                    // elsewhere in the same system. "Bridged
                    // floor-to-ceiling" is a claim about one contiguous
                    // run, not the whole column.
                    let mut cells: Vec<(i32, bool)> = Vec::new();
                    for y in y0b..=y1b {
                        if void_here.contains(&(x, y)) {
                            cells.push((y, false));
                        } else if is_form(x, y) {
                            cells.push((y, true));
                        }
                    }
                    let mut runs: Vec<Vec<(i32, bool)>> = Vec::new();
                    for cell in cells {
                        match runs.last_mut() {
                            Some(run) if cell.0 - run.last().expect("non-empty").0 <= 1 => {
                                run.push(cell);
                            }
                            _ => runs.push(vec![cell]),
                        }
                    }
                    for run in runs {
                        let solids = run.iter().filter(|&&(_, s)| s).count();
                        if solids == 0 {
                            continue;
                        }
                        speleo_cells += solids;
                        columns_checked += 1;
                        // A run under 5 cells is not a passage a player
                        // would ever call one -- the same floor this pass
                        // itself uses to decide whether a run is even
                        // eligible to carry a formation (`span < 5` is
                        // skipped entirely). A column that only grazes a
                        // room's edge for one or two rows, where a
                        // formation legitimately reaches the boundary of
                        // open space without another void cell anywhere
                        // else in that column, is not a closed passage --
                        // it is a formation touching a wall.
                        if run.len() >= 5 && run.iter().all(|&(_, s)| s) {
                            fused_here += 1;
                            println!(
                                "seed {seed}: fused run at x = {x}, y {}..={} ({solids} solid rows)",
                                run[0].0,
                                run.last().expect("non-empty").0
                            );
                        }
                    }
                }
                // Round-5 task 4c allows *exactly one* fused (fully solid)
                // column per system, inside its largest chamber -- a
                // second one, anywhere, is the bridging bug this test
                // exists to catch wearing task 4c's exception as cover.
                assert!(
                    fused_here <= 1,
                    "seed {seed}: {fused_here} columns bridged floor-to-ceiling in one cave system -- \
                     task 4c allows at most one"
                );
                fused_columns_total += fused_here;
            }
        }
    }
    // The counters beside the claim: a suite where the decoration pass
    // barely fired, or where the fused column never once appeared, would
    // pass every assertion above vacuously.
    assert!(
        speleo_cells >= 40 && columns_checked >= 10,
        "only {speleo_cells} speleothem cells in {columns_checked} columns -- the decoration pass barely fired"
    );
    assert!(
        fused_columns_total >= 1,
        "the fused column never appeared across {systems_checked} systems -- task 4c's mechanic is vacuous here"
    );
}

#[test]
#[ignore = "probe: prints, never asserts (round-3 task 1)"]
fn probe_r3_dump_a_cave_system() {
    // The instrument for tuning CAVE_CELL / CAVE_THRESHOLD / CAVE_SQUASH by
    // eye: ASCII cross-sections of whole systems at the forced size, with
    // the census printed beside them ("did it fire" is a counter, and what a
    // system's anatomy is -- chambers, passages, floors -- is only legible
    // at cell scale).
    let presets = presets();
    let base = presets.get("rolling").expect("preset");
    let with = vault_test_params(base);
    let without = WorldgenParams { vault_density: 0.0, ..with.clone() };
    for seed in SEEDS {
        let world = build(&with, seed);
        let control = build(&without, seed);
        let mut carved: Vec<(i32, i32)> = Vec::new();
        for y in 0..=BOUNDS.1 {
            for x in 0..=BOUNDS.0 {
                if world.get(x, y).material != control.get(x, y).material
                    || world.get(x, y).shade != control.get(x, y).shade
                {
                    carved.push((x, y));
                }
            }
        }
        if carved.is_empty() {
            println!("\n=== seed {seed}: no system placed ===");
            continue;
        }
        let id = |n: &str| world.materials.id_of(n).expect(n);
        let (gravel, water, crystal) = (id("gravel"), id("water"), id("crystal"));
        let stone = id("stone");
        // Census: air/gravel/water/solid split, and vertical-run heights to
        // separate chambers (tall) from passages (low).
        let carved_set: std::collections::HashSet<(i32, i32)> = carved.iter().copied().collect();
        let (mut air, mut grav, mut wat, mut solid) = (0, 0, 0, 0);
        for &(x, y) in &carved {
            let m = world.get(x, y).material;
            if m == material::EMPTY {
                air += 1;
            } else if m == gravel {
                grav += 1;
            } else if m == water {
                wat += 1;
            } else {
                solid += 1;
            }
        }
        let (x0, x1) = (
            carved.iter().map(|&(x, _)| x).min().unwrap(),
            carved.iter().map(|&(x, _)| x).max().unwrap(),
        );
        let (y0, y1) = (
            carved.iter().map(|&(_, y)| y).min().unwrap(),
            carved.iter().map(|&(_, y)| y).max().unwrap(),
        );
        // Tallest open run per column, for the chamber/passage read.
        let mut tallest = 0;
        for x in x0..=x1 {
            let mut h = 0;
            for y in y0..=y1 + 1 {
                if carved_set.contains(&(x, y)) && world.get(x, y).material == material::EMPTY {
                    h += 1;
                    tallest = tallest.max(h);
                } else {
                    h = 0;
                }
            }
        }
        println!(
            "\n=== seed {seed}: {} carved cells in {}x{} at ({x0}..{x1}, {y0}..{y1}); air {air} gravel {grav} water {wat} solid {solid}; tallest open run {tallest} ===",
            carved.len(),
            x1 - x0 + 1,
            y1 - y0 + 1
        );
        for y in (y0 - 2).max(0)..=(y1 + 2).min(BOUNDS.1) {
            let line: String = ((x0 - 2).max(0)..=(x1 + 2).min(BOUNDS.0))
                .map(|x| {
                    let c = world.get(x, y);
                    if c.material == material::EMPTY {
                        '.'
                    } else if c.material == gravel {
                        // Case says who wrote it: 'G' is the vault pass's own
                        // floor fill (a carved cell), 'g' was already there
                        // (a pocket lens, a soil-contact dither).
                        if carved_set.contains(&(x, y)) { 'G' } else { 'g' }
                    } else if c.material == water {
                        '~'
                    } else if c.material == crystal {
                        'X'
                    } else if c.material == stone {
                        if carved_set.contains(&(x, y)) { 'T' } else { '#' }
                    } else {
                        '?'
                    }
                })
                .collect();
            println!("  {y:>4} {line}");
        }
    }
}

#[test]
fn pond_water_uses_its_four_tones_evenly() {
    // `ponds` draws shades 0..3 (`loose_shade` against `TONES = 4`) and
    // `render.rs` colours a cell with `palette[shade % palette.len()]`. While
    // water shipped three colours, shade 3 folded onto entry 0 and that entry
    // drew twice as often as either of the others -- a resting pool was half
    // one tone rather than even thirds.
    //
    // Asserted on the *rendered* distribution rather than on the palette
    // length, because the length is the mechanism and the weighting is the
    // claim: a future change that adds a fifth colour without touching
    // `TONES` would pass a length check and reintroduce exactly this bug in
    // the other direction.
    let presets = presets();
    let params = presets.get("wetland").expect("wetland preset");
    let mut counts = [0usize; 8];
    let mut total = 0usize;
    let mut tones = 0usize;
    for seed in SEEDS {
        let world = build(params, seed);
        let water = world.materials.id_of("water").expect("water");
        let len = world.materials.get(water).palette.len();
        assert!(len > 0, "water has no palette");
        assert!(len <= counts.len(), "water has more tones than this census array holds");
        tones = len;
        for y in 0..=BOUNDS.1 {
            for x in 0..=BOUNDS.0 {
                let c = world.get(x, y);
                if c.material == water {
                    counts[(c.shade as usize) % len] += 1;
                    total += 1;
                }
            }
        }
    }
    assert!(total > 5_000, "vacuous: only {total} water cells across {} seeds", SEEDS.len());
    // Even means every entry within a quarter of its fair share. The bar is
    // deliberately loose: this is a hash over a few thousand cells, not a
    // uniformity proof, and the failure it exists to catch is a 2:1 fold --
    // which is 100% off, not 25%.
    let fair = total as f64 / tones as f64;
    for (i, &n) in counts.iter().take(tones).enumerate() {
        let ratio = n as f64 / fair;
        assert!(
            (0.75..1.25).contains(&ratio),
            "water tone {i} drew {n} of {total} cells, {ratio:.2}x its fair share of 1/{tones}"
        );
    }
}

#[test]
fn a_forced_boulder_world_seats_stone_and_arrives_at_rest() {
    // Round-4 task 3. Boulder sockets fire rarely at the landed erosion
    // rates -- `boulders`'s own doc comment has the harness-size numbers --
    // and rejecting most of them is by design, not a bug: a marker sits
    // right at a steep drop by construction, and `canyon`'s brow_chance
    // (0.9) means the open air a dome wants to rise into is very often
    // already a brow's underside, which the collect-verify-write contract
    // correctly refuses to overwrite. So this sweeps a wide, cheap seed
    // range rather than forcing `world_age` further (CLAUDE.md: raise
    // `world_age`, never touch the erosion constants) -- widening the seed
    // pool finds real successes without changing what the pass is rating.
    let presets = presets();
    let base = presets.get("canyon").expect("canyon preset");
    let params = WorldgenParams { world_age: 1.0, tree_density: 0.0, moss_density: 0.0, ..base.clone() };

    let mut seated_cells = 0usize;
    let mut checked = 0usize;
    for seed in 1u64..=150 {
        let mut world = World::new(Rect::new(0, 0, BOUNDS.0, BOUNDS.1));
        let report = worldgen::generate_reported(&mut world, Spec::Generated { params: &params, seed });
        structural::compute_world_distances(&mut world);
        let cells = report.iter().find(|(name, _)| *name == "boulders").map_or(0, |&(_, n)| n);
        if cells == 0 {
            continue;
        }
        checked += 1;
        seated_cells += cells;

        // And it holds still, the same bar every generated-terrain claim in
        // this file is held to: attached stone cannot move on its own, but
        // the columns it displaced (talus, soil, sand) can, and a boulder
        // seated wrong is exactly the kind of thing that would show up here.
        let before: std::collections::HashSet<_> = snapshot(&world).into_iter().collect();
        for _ in 0..120 {
            step(&mut world);
        }
        let after: std::collections::HashSet<_> = snapshot(&world).into_iter().collect();
        let gone: Vec<_> = before.difference(&after).copied().collect();
        assert!(
            gone.is_empty(),
            "canyon seed {seed}: {} cells left their position in a forced-boulder world",
            gone.len()
        );
    }
    println!("canyon age 1.0, seeds 1..=150: {checked} worlds seated a boulder, {seated_cells} cells total");
    assert!(checked > 0, "no seed in 1..=150 seated a boulder -- the pass never fired");
}

#[test]
#[ignore = "probe: prints, never asserts (round-4 task 4)"]
fn probe_r4t4_valley_floor_retarget_diff() {
    // Column-level flip rate is not the claim that matters -- the check only
    // ever touches the top two rows of a column that is not already sandy,
    // so the real quantity is *cells whose drawn material actually changes*,
    // per the CLAUDE.md rule that a picture/count of the triggering
    // condition is not the same as a census of the consequence.
    use pixel_physics::worldgen::column::Terrain;
    let presets = presets();
    for name in ["wetland", "rolling", "canyon", "terraced", "arid"] {
        let base = presets.get(name).expect("preset");
        assert_eq!(base.world_age, 0.0, "age 0 for this probe");
        let mut flipped_columns = 0;
        let mut flipped_cells = 0;
        for seed in SEEDS {
            let t = Terrain::new(seed, base, 512, 320, 33.0f32.to_radians().tan(), 34.0f32.to_radians().tan());
            let plans = t.plan_all();
            let datum = base.sky_rows + base.relief_amplitude;
            for x in 0..t.w {
                let c = plans[x as usize];
                if c.soil_depth <= 0 || t.is_sandy(x) {
                    continue;
                }
                let old = t.slope(x) < 0.1 && t.elev(x) < -0.45 * base.relief_amplitude;
                let at = |xx: i32| plans[xx.clamp(0, t.w - 1) as usize].surface_y;
                let new_slope = ((at(x + 1) - at(x - 1)) as f32 / 2.0).abs();
                let new_elev = datum - c.surface_y as f32;
                let new = new_slope < 0.1 && new_elev < -0.45 * base.relief_amplitude;
                if old == new {
                    continue;
                }
                flipped_columns += 1;
                // Only the rows the branch actually reaches (`y < top + 2`),
                // and only up to the column's own depth.
                flipped_cells += 2.min(c.soil_depth) as usize;
            }
        }
        println!("{name}: {flipped_columns} columns flip is_valley_floor, ~{flipped_cells} cells would draw differently (upper bound: the stony-contact dither can still override either way)");
    }
}

#[test]
#[ignore]
fn tmp_find_waterline_shot() {
    let presets = presets();
    let with = presets.get("wetland").expect("preset").clone();
    for seed in 1u64..=16 {
        let world = build(&with, seed);
        let id = |n: &str| world.materials.id_of(n).expect(n);
        let (crystal, water) = (id("crystal"), id("water"));
        let mut best: Option<(i32, i32, i32)> = None; // (x, y, water_col_count)
        for x in 0..pixel_physics::app::WORLD_WIDTH as i32 {
            for y in 0..pixel_physics::app::WORLD_HEIGHT as i32 {
                let c = world.get(x, y);
                if c.material != crystal {
                    continue;
                }
                let mut wcount = 0;
                for dy in 0..5 {
                    let below = world.get(x, y + dy);
                    if below.material == water {
                        wcount += 1;
                    }
                }
                if wcount > 0 && best.is_none_or(|(_, _, b)| wcount > b) {
                    best = Some((x, y, wcount));
                }
            }
        }
        match best {
            Some((x, y, s)) => println!("seed {seed}: best crystal-at-waterline at ({x},{y}) score={s}"),
            None => println!("seed {seed}: none found"),
        }
    }
}
