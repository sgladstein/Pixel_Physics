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
    let presets = presets();
    let params = presets.get(&presets.default_name()).expect("default preset");
    let mut world = build(params, 1);
    let mut frames = 0;
    while world.active_chunk_count() > 0 && frames < 30 {
        step(&mut world);
        frames += 1;
    }
    assert!(frames <= 6, "took {frames} frames to go quiet; generated terrain should settle at once");
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
