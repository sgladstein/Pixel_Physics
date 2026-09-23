//! **The movement plan's scenes: small, controlled places where the ant's
//! walk has a known answer** (`Reports/ant-movement-plan-2026-09-22.md` §6).
//!
//! ```text
//! cargo run --release --example scenes -- scene=s0              # 24 seeds, all arms
//! cargo run --release --example scenes -- scene=s0 seeds=4 frames=6000
//! ```
//!
//! **Every scene also runs under stage 1's chooser**
//! (`PIXEL_PHYSICS_CHOOSER=on`, or `nopatience` for its ablation); the header
//! echoes which. Its predictions, written before its first run, and its
//! results are in `Reports/ant-scenes-2026-09-23.md` §3.
//!
//! Every scene is read from the engine's own decision trace
//! (`creature::DecisionRow`), so an outcome is the branch the decision took,
//! not one inferred from how the head moved. Each scene asserts its setup
//! checklist (plan §6) from the trace before it reports anything: a scene
//! whose setup drifted is stopped, not summarised.
//!
//! # S0: an empty ant on a bare slab, no trail
//!
//! One ant, a flat stone floor, air above, nothing else, and a small pile of
//! fruit 90 cells away **on each side**. Not two mirrored arms, deliberately:
//! the ant's draws do not depend on where the food is, so a run with the
//! food east and one with it west are the same walk until it arrives, and
//! counting both would count one sample twice. With food on both sides a
//! directional bias in the scene (the sweep is chunked left to right) shows
//! as one side being found first more often. The ant lays no
//! trail (every `EmitA`/`EmitB` weight zeroed), cannot breed, and its energy
//! is pinned every frame at the arm's value, so `Energy` does not drift. The
//! run ends when the ant first stands beside the food, or at `frames=`.
//!
//! **The prediction, written before the scene was first run** (2026-09-23),
//! from `how-the-ant-works.md` §4 and §6:
//!
//! - On a bare slab the usable headings are east and west only, so the cone
//!   always goes straight and a tumble picks one of the two at random.
//! - `p_move = squash(2.0 - 1.75 * Energy)`: **0.20 at Energy 1, 0.53 at 0.5**
//!   (stillness adds a little during long pauses). A failed roll tumbles with
//!   probability 0.5, and half the tumbles reverse.
//! - **Energy 1: per decision, step 0.20, reverse 0.20, same-heading re-roll
//!   0.20, nothing 0.40.** A step is as likely as a reversal, so successive
//!   steps are uncorrelated: a random walk at 0.2 cells per decision. Over a
//!   24,000-frame run (4,000 decisions) the typical excursion is about 28
//!   cells, and **reaching food 90 cells away should happen in well under 1%
//!   of runs**.
//! - **Energy 0.5: step 0.53, reverse 0.12, re-roll 0.12, nothing 0.23.** Runs
//!   average about 5 steps between reversals, the typical excursion is about
//!   100 cells, and **roughly a third of runs should reach the food**.
//!
//! **Restated for food on both sides** (the scene was changed to it after a
//! two-seed smoke test showed the mirrored arms were one walk counted twice,
//! and before the full run): at Energy 1 still **well under 1%**; at Energy
//! 0.5 an exit at either end is about twice as likely as at one, so **roughly
//! half to two thirds of runs**. The smoke test's per-decision shares already
//! matched to within two points: step 19.1 / reverse 22.0 / re-roll 20.4 /
//! nothing 38.5 at Energy 1, and 52.0 / 12.1 / 12.9 / 22.9 at 0.5.
//!
//! Research expects long straight runs from an explorer; this is the
//! baseline the chooser's turning preference must change (plan §4e, §4i).
//!
//! # S1–S3: a laden ant getting home
//!
//! One ant carrying a full crop of fruit, its home point set explicitly
//! (`forage_anchor`), no nest material anywhere so nothing re-anchors it and
//! no trail A is laid. Energy is pinned at 1.0 and **the crop is pinned full
//! every frame**, because digestion would otherwise empty it over a long
//! scene and turn the ant into an empty one that no longer homes. The run
//! ends when the head is within one cell of home, or at `frames=`.
//!
//! **S1: flat slab, home 40 cells away**, two arms: home to the east (the ant
//! spawns facing east, so facing home) and to the west (facing away).
//! **Prediction, from the plan's §6** (written 2026-09-22, before any run):
//! facing home, `p_move = squash(2.0 - 1.75 + 3.0) = 0.76`, and a failed roll
//! re-picks the same homeward heading; facing away, `p_move` is 0 and the
//! homeward re-roll turns it round at about 0.5 per decision. **It arrives in
//! about 50–60 decisions, the facing-away start about two decisions slower.**
//!
//! **S2: the same, home 40 cells west, with a one-cell-wide stone wall of
//! height 1, 2, 3, 6 or 12 halfway.** Prediction (plan §6): climbing points the
//! ant away from a home on its own level, so `(HomeAligned, Move, 3.0)` cuts
//! its stepping up the face -- about 0.15, 0.09, 0.02 and then 0 at heights 1
//! to 4 -- and each failed roll risks the re-roll pointing it back down. **Each
//! extra cell of height makes the crossing much less likely; from height 4
//! only `Stillness` (up to +1.5 after 192 still decisions) gets it over.**
//!
//! **S3: a U-bend.** The ant starts at the blind east end of a one-high
//! tunnel; the only way out runs 70 cells west, up a shaft and back east
//! along a second tunnel to home, which lies east of the start. Prediction
//! (plan §6): **trapped.** At the blind end the only usable heading is west,
//! which faces away from home, where `p_move` is 0 even at full `Stillness`
//! (`squash(0.25 - 3.0 + 1.5) < 0`). Run it again with
//! `PIXEL_PHYSICS_REVERSE=off`, so the reversal rule cannot take the credit.
use pixel_physics::sim::brain::{self, BrainOutput as O};
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::creature::{self, DecisionOutcome as D, DecisionRow, DropWhy as D2};
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::material;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::{frame, player, Cell, World};

fn arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.parse().ok().expect("parses")))
        .unwrap_or(default)
}
fn arg_str(name: &str, default: &str) -> String {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.to_string())).unwrap_or_else(|| default.into())
}

/// Zero every weight into `EmitA` and `EmitB`, direct and via the hidden
/// layer, so the ant cannot lay the trail it might then read. Returns how
/// many slots moved, so a silent no-op is visible. (`onetrail`'s helper.)
fn silence_emission(g: &mut [f32]) -> usize {
    let mut moved = 0;
    for out in [O::EmitA, O::EmitB] {
        for i in 0..brain::BRAIN_INPUTS {
            let slot = brain::io_slot(brain::INPUTS[i], out);
            if g[slot] != 0.0 {
                g[slot] = 0.0;
                moved += 1;
            }
        }
        for h in 0..brain::BRAIN_HIDDEN {
            let slot = brain::ho_slot(h, out);
            if g[slot] != 0.0 {
                g[slot] = 0.0;
                moved += 1;
            }
        }
    }
    moved
}

/// One S0 run's result.
struct S0Run {
    seed: u64,
    energy: f32,
    /// Which side's food it reached first, if any.
    found_east: Option<bool>,
    decisions: u64,
    stepped: u64,
    reversed: u64,
    rerolled: u64,
    idle: u64,
    other: u64,
    /// Decisions until the ant first stood beside the food, if it did.
    found_at: Option<u64>,
    /// Furthest the head got from the start, either way.
    reach: i32,
    /// Mean cells covered per decision (steps are one cell each).
    cells_per_decision: f64,
    /// Steps between reversals: stepped / (reversed + 1).
    run_length: f64,
}

fn s0(seed: u64, energy: f32, frames: u64, gap: i32) -> S0Run {
    let (w_cells, h) = (400i32, 64i32);
    let mut world = World::new(Rect::new(0, 0, w_cells - 1, h - 1));
    world.seed = seed;
    let floor = h - 8;
    let head_y = floor - 1;
    let stone = Cell::new(material::STONE, 0).with_attached(true);
    // Bare stone slab, and walls at both ends far out of reach.
    for x in 0..w_cells {
        for y in floor..h {
            world.set(x, y, stone);
        }
    }
    for x in [0, 1, w_cells - 2, w_cells - 1] {
        for y in 0..floor {
            world.set(x, y, stone);
        }
    }
    let species = world.species.id_of("ant").expect("the ant species is compiled in");
    {
        let mut def = world.species.get(species).creature.clone().expect("ant is a creature");
        def.reproduce_threshold = 1.0e30;
        // No death of old age: the ant's half-life is 40,000 frames, and the
        // first full run lost an ant to it mid-scene (a corpse on the slab).
        def.life_half_life = 0;
        world.species.set_creature(species, def);
    }
    let def = world.species.get(species).creature.clone().expect("ant is a creature");
    let mut genome = world.species.get(species).genome.clone();
    assert!(silence_emission(&mut genome) > 0, "no EmitA/EmitB weight was zeroed, so the ant is still laying trail");
    world.species.set_genome(species, genome);

    let start_x = w_cells / 2;
    let fruit = world.materials.id_of("fruit").expect("fruit material");
    for food_x in [start_x + gap, start_x - gap] {
        for dy in 0..3 {
            world.set(food_x, head_y - dy, Cell::new(fruit, 0));
        }
    }
    // **Clear sky, pinned.** The app's frame step runs the weather, and rain
    // pools on a flat slab: the first full run of this scene had 364 cells of
    // standing water on the "bare" floor by the end, and an ant cannot step
    // into water, so half its decisions saw a one-way floor. The slab has to
    // stay the scene it claims to be; the census below checks it did.
    world.set_weather_pin(pixel_physics::sim::weather::Pin::Clear);
    world.plant_ant(start_x, head_y);
    let ant = world.live_organism_ids().into_iter().find(|&id| world.organism(id).is_some_and(|s| s.species == species)).expect("the ant was placed");
    let pin = |w: &mut World| {
        w.set_organism_energy(ant, energy * def.start_energy);
    };
    pin(&mut world);
    world.decision_log = Some(Vec::new());

    let (mut particles, mut blasts, tuning) = (ParticleSystem::default(), Blasts::default(), player::Tuning::default());
    let mut rows: Vec<DecisionRow> = Vec::new();
    let mut found_at = None;
    for _ in 0..frames {
        frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        if let Some(log) = world.decision_log.as_mut() {
            rows.append(log);
        }
        pin(&mut world);
        if found_at.is_none() && rows.iter().rev().take(1).any(|r| r.food_adjacent > 0.0) {
            found_at = Some(rows.len() as u64);
            break;
        }
    }

    // The setup checklist (plan §6), asserted from the trace and the world.
    // Nothing may stand above the slab but the ant and its food.
    for y in 0..floor {
        for x in 2..w_cells - 2 {
            let m = world.get(x, y).material;
            assert!(
                m == material::EMPTY || m == fruit || world.get(x, y).organism_id() == ant,
                "the slab is no longer bare: {} at ({x}, {y})",
                world.materials.get(m).name
            );
        }
    }
    let mine: Vec<&DecisionRow> = rows.iter().filter(|r| r.id == ant).collect();
    assert_eq!(mine.len(), rows.len(), "another creature made decisions in a one-ant scene");
    assert!(mine.len() > 10, "the ant made only {} decisions", mine.len());
    for r in &mine {
        assert!((r.energy - energy).abs() < 1e-3, "energy drifted to {} against a pin of {energy}: {r:?}", r.energy);
        assert_eq!(r.leg, 0, "the ant became laden before the food was reached: {r:?}");
        assert_eq!(r.along_b, 0.0, "a trail-B reading on a scene with no trail: {r:?}");
    }
    // **Away from the piles.** A pile is three cells of fruit, and within a
    // few cells of it the fruit is footing, so a diagonal opens: that is the
    // scene, not drift. The shipped walk rarely got there; the chooser gets
    // there in a few hundred decisions and then waits beside it on the 0.2
    // step roll, so the piles' neighbourhood is excluded rather than
    // tolerated in the 1%.
    let near_food = |r: &&&DecisionRow| (r.head.0 - (start_x + gap)).abs() <= 3 || (r.head.0 - (start_x - gap)).abs() <= 3;
    let open: Vec<&&DecisionRow> = mine.iter().filter(|r| !near_food(r)).collect();
    let corridor = open.iter().filter(|r| r.usable == (1 << 0) | (1 << 4)).count();
    if (corridor as f64) < 0.99 * open.len() as f64 {
        let mut masks: std::collections::BTreeMap<(u8, i32), u64> = std::collections::BTreeMap::new();
        for r in &mine {
            *masks.entry((r.usable, r.head.1)).or_default() += 1;
        }
        let mut by_heading: std::collections::BTreeMap<(u8, u8), u64> = std::collections::BTreeMap::new();
        for r in &mine {
            *by_heading.entry((r.usable, r.heading)).or_default() += 1;
        }
        let odd: Vec<String> = mine
            .iter()
            .filter(|r| r.usable != (1 << 0) | (1 << 4))
            .take(6)
            .map(|r| format!("frame {} head {:?} heading {} usable {} outcome {:?}", r.frame, r.head, r.heading, r.usable, r.outcome))
            .collect();
        panic!(
            "only {corridor} of {} decisions away from the piles saw a bare-slab corridor; (usable mask, head row) -> {masks:?}; (usable, heading) -> {by_heading:?}; first odd: {odd:?}",
            open.len()
        );
    }
    let paced = mine.windows(2).filter(|p| p[1].frame - p[0].frame == 6).count();
    assert!(paced + 1 >= mine.len(), "decisions are not one in six frames ({paced} of {})", mine.len() - 1);

    let mut run = S0Run {
        seed,
        energy,
        found_east: found_at.map(|_| mine.last().is_some_and(|r| r.head.0 > start_x)),
        decisions: mine.len() as u64,
        stepped: 0,
        reversed: 0,
        rerolled: 0,
        idle: 0,
        other: 0,
        found_at,
        reach: 0,
        cells_per_decision: 0.0,
        run_length: 0.0,
    };
    for r in &mine {
        match r.outcome {
            D::Stepped => {
                run.stepped += 1;
                // The shipped cone only steps forward, so this is the
                // chooser's reversal: a step that turned the ant round.
                if r.heading_after == (r.heading + 4) % 8 {
                    run.reversed += 1;
                }
            }
            D::RollFailedTumbled | D::BlockedTumbled => {
                if r.heading_after == (r.heading + 4) % 8 {
                    run.reversed += 1;
                } else {
                    run.rerolled += 1;
                }
            }
            D::RollFailedIdle => run.idle += 1,
            _ => run.other += 1,
        }
        run.reach = run.reach.max((r.head_after.0 - start_x).abs());
    }
    run.cells_per_decision = run.stepped as f64 / run.decisions as f64;
    run.run_length = run.stepped as f64 / (run.reversed + 1) as f64;
    run
}

/// A laden scene's geometry.
#[derive(Clone, Copy, PartialEq)]
enum Ground {
    /// S1: a bare slab, home `dx` cells from the start (negative is west).
    Flat { dx: i32 },
    /// S2: a bare slab, home 40 cells west, a wall of this height halfway.
    Wall { height: i32 },
    /// S3: the U-bend.
    UBend,
}

/// One laden run's result.
struct LadenRun {
    seed: u64,
    decisions: u64,
    arrived_at: Option<u64>,
    stepped: u64,
    tumbled: u64,
    idle: u64,
    blocked: u64,
    /// Decisions facing home (cosine to home > 0.5) and away (< -0.5), and
    /// the mean `p_move` in each. The cosine is computed here from the head,
    /// the anchor and the heading, not read from `HomeAligned`: the chooser
    /// feeds the brain `(1 + cos) / 2` there.
    facing_home: u64,
    facing_away: u64,
    pmove_home: f64,
    pmove_away: f64,
    /// Homeward re-roll firings, and those that pointed away from home.
    fired: u64,
    fired_away: u64,
    /// Steps the chooser picked, those whose heading pointed away from home,
    /// and the lowest patience it scored with.
    chose: u64,
    chose_away: u64,
    min_patience: f32,
    /// The longest run of consecutive decisions in which the head did not
    /// move: the frozen runs of `ant-scenes-2026-09-23.md` §2 read ~3,980.
    longest_still: u64,
    /// S2: the highest the head got above the floor, and decisions spent at
    /// the wall (head within two columns of it). S3: the furthest west.
    peak: i32,
    at_wall: u64,
    /// S2: `Stillness` at the decision the head first got past the wall.
    stillness_at_cross: Option<f32>,
}

/// The cosine between a decision's heading and home, from the head and the
/// anchor; NaN on the anchor. What `HomeAligned` reads on the shipped walk.
fn true_home_cos(r: &DecisionRow) -> f32 {
    let (vx, vy) = ((r.anchor.0 - r.head.0) as f32, (r.anchor.1 - r.head.1) as f32);
    let len = (vx * vx + vy * vy).sqrt();
    if len < 1.0 {
        return f32::NAN;
    }
    let (dx, dy) = creature::DIRS[r.heading as usize];
    (dx as f32 * vx + dy as f32 * vy) / (len * ((dx * dx + dy * dy) as f32).sqrt())
}

fn laden(seed: u64, ground: Ground, frames: u64) -> LadenRun {
    let (w_cells, h) = (400i32, 96i32);
    let mut world = World::new(Rect::new(0, 0, w_cells - 1, h - 1));
    world.seed = seed;
    let floor = h - 8;
    let head_y = floor - 1;
    let stone = Cell::new(material::STONE, 0).with_attached(true);
    for x in 0..w_cells {
        for y in floor..h {
            world.set(x, y, stone);
        }
    }
    for x in [0, 1, w_cells - 2, w_cells - 1] {
        for y in 0..floor {
            world.set(x, y, stone);
        }
    }
    let start_x = w_cells / 2;
    let wall_x = start_x - 20;
    let (start, home) = match ground {
        Ground::Flat { dx } => ((start_x, head_y), (start_x + dx, head_y)),
        Ground::Wall { height } => {
            for y in (head_y - height + 1)..=head_y {
                world.set(wall_x, y, stone);
            }
            ((start_x, head_y), (start_x - 40, head_y))
        }
        Ground::UBend => {
            // A solid block with two one-high tunnels joined by a shaft at
            // the west end: lower row 80 (the ant starts at its blind east
            // end), upper row 70 (home at its east end).
            for x in 100..=300 {
                for y in 60..floor {
                    world.set(x, y, stone);
                }
            }
            for x in 150..=220 {
                world.set(x, 80, Cell::EMPTY);
            }
            for x in 150..=270 {
                world.set(x, 70, Cell::EMPTY);
            }
            for y in 70..=80 {
                world.set(150, y, Cell::EMPTY);
            }
            ((220, 80), (262, 70))
        }
    };
    let species = world.species.id_of("ant").expect("the ant species is compiled in");
    {
        let mut def = world.species.get(species).creature.clone().expect("ant is a creature");
        def.reproduce_threshold = 1.0e30;
        def.life_half_life = 0;
        world.species.set_creature(species, def);
    }
    let def = world.species.get(species).creature.clone().expect("ant is a creature");
    let mut genome = world.species.get(species).genome.clone();
    assert!(silence_emission(&mut genome) > 0, "no EmitA/EmitB weight was zeroed, so the ant is still laying trail");
    world.species.set_genome(species, genome);
    world.set_weather_pin(pixel_physics::sim::weather::Pin::Clear);
    world.plant_ant(start.0, start.1);
    let ant = world.live_organism_ids().into_iter().find(|&id| world.organism(id).is_some_and(|s| s.species == species)).expect("the ant was placed");
    let fruit = world.materials.id_of("fruit").expect("fruit material");
    // Three whole cells at the capacity's per-cell worth: a full crop.
    let unit = pixel_physics::sim::creature::organism_crop_capacity(&world, ant, &def) / 3.0;
    let full = pixel_physics::sim::organism::Crop { material: fruit, cells: 3, digesting: 0.0, unit, shade: 0, passenger: None };
    let pin = |w: &mut World| {
        w.set_organism_energy(ant, def.start_energy);
        w.set_organism_crop(ant, Some(full));
    };
    pin(&mut world);
    world.set_organism_forage_anchor(ant, home);
    world.decision_log = Some(Vec::new());

    let (mut particles, mut blasts, tuning) = (ParticleSystem::default(), Blasts::default(), player::Tuning::default());
    let mut rows: Vec<DecisionRow> = Vec::new();
    let mut arrived_at = None;
    for _ in 0..frames {
        frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        if let Some(log) = world.decision_log.as_mut() {
            rows.append(log);
        }
        pin(&mut world);
        let head = world.organism(ant).and_then(|s| s.chain.first().copied()).expect("the ant is alive");
        if (head.0 - home.0).abs() <= 1 && (head.1 - home.1).abs() <= 1 {
            arrived_at = Some(rows.len() as u64);
            break;
        }
    }

    // `dump=FIRST,COUNT`: print those decisions of every run, one row each --
    // position, heading, what was usable, what the home fix read, the step
    // chance, the branch taken and why the re-roll did what it did.
    if let Some((first, count)) = arg_str("dump", "").split_once(',').map(|(a, b)| (a.parse::<usize>().expect("dump=first,count"), b.parse::<usize>().expect("dump=first,count"))) {
        const NAMES: [&str; 8] = ["E", "NE", "N", "NW", "W", "SW", "S", "SE"];
        let heading = |d: u8| NAMES[d as usize];
        let mask = |m: u8| (0..8).filter(|i| m >> i & 1 == 1).map(|i| NAMES[i]).collect::<Vec<_>>().join("|");
        println!("    dump, seed {seed}: decision frame head -> head_after heading -> after | usable | home_aligned p_move stillness | outcome homeward home_cos | patience chosen_cos");
        for (i, r) in rows.iter().enumerate().skip(first).take(count) {
            println!(
                "    {i:>5} {:>6} {:?} -> {:?} {} -> {} | {} | {:+.3} {:.3} {:.3} | {:?} {:?} {:+.3} | {:.3} {:+.3}",
                r.frame,
                r.head,
                r.head_after,
                heading(r.heading),
                heading(r.heading_after),
                mask(r.usable),
                r.home_aligned,
                r.p_move,
                r.stillness,
                r.outcome,
                r.homeward,
                r.home_cos,
                r.patience,
                r.chosen_cos
            );
        }
        // The whole body at the end, head first: the trace carries only the
        // head, and whether a stopped ant is hanging or held needs the rest.
        println!("    final body, seed {seed}: {:?}", world.organism(ant).map(|s| s.chain.clone()).unwrap_or_default());
    }

    // The setup checklist, asserted from the trace and the world.
    assert!(rows.len() > 5, "the ant made only {} decisions", rows.len());
    for r in &rows {
        assert_eq!(r.id, ant, "another creature made decisions in a one-ant scene");
        assert!((r.energy - 1.0).abs() < 1e-3, "energy drifted to {}: {r:?}", r.energy);
        assert_eq!(r.leg, 1, "the ant was not laden: {r:?}");
        assert!(r.fill > 0.99, "the crop was not full: {r:?}");
        assert_eq!(r.anchor, home, "home moved: {r:?}");
        assert!(!matches!(r.drop, D2::Placed | D2::Delivered), "food was put down: {r:?}");
    }
    let paced = rows.windows(2).filter(|p| p[1].frame - p[0].frame == 6).count();
    assert!(paced + 1 >= rows.len(), "decisions are not one in six frames ({paced} of {})", rows.len() - 1);
    for y in 0..floor {
        for x in 2..w_cells - 2 {
            let c = world.get(x, y);
            assert!(
                c.material == material::EMPTY || c.material == material::STONE || c.organism_id() == ant,
                "the scene is no longer what it was built as: {} at ({x}, {y})",
                world.materials.get(c.material).name
            );
        }
    }

    let mut run = LadenRun {
        seed,
        decisions: rows.len() as u64,
        arrived_at,
        stepped: 0,
        tumbled: 0,
        idle: 0,
        blocked: 0,
        facing_home: 0,
        facing_away: 0,
        pmove_home: 0.0,
        pmove_away: 0.0,
        fired: 0,
        fired_away: 0,
        chose: 0,
        chose_away: 0,
        min_patience: f32::NAN,
        longest_still: 0,
        peak: 0,
        at_wall: 0,
        stillness_at_cross: None,
    };
    let mut still = 0u64;
    for r in &rows {
        still = if r.head_after == r.head { still + 1 } else { 0 };
        run.longest_still = run.longest_still.max(still);
        match r.outcome {
            D::Stepped | D::Fell => run.stepped += 1,
            D::RollFailedTumbled => run.tumbled += 1,
            D::RollFailedIdle => run.idle += 1,
            _ => run.blocked += 1,
        }
        let cos = true_home_cos(r);
        if cos > 0.5 {
            run.facing_home += 1;
            run.pmove_home += r.p_move as f64;
        } else if cos < -0.5 {
            run.facing_away += 1;
            run.pmove_away += r.p_move as f64;
        }
        if matches!(r.homeward, creature::HomewardWhy::Fired) {
            run.fired += 1;
            if r.home_cos < -0.01 {
                run.fired_away += 1;
            }
        }
        if !r.chosen_cos.is_nan() && r.outcome == D::Stepped {
            run.chose += 1;
            if r.chosen_cos < -0.01 {
                run.chose_away += 1;
            }
        }
        if !r.patience.is_nan() {
            run.min_patience = if run.min_patience.is_nan() { r.patience } else { run.min_patience.min(r.patience) };
        }
        match ground {
            Ground::Wall { .. } => {
                run.peak = run.peak.max(head_y - r.head_after.1);
                if (r.head_after.0 - wall_x).abs() <= 2 {
                    run.at_wall += 1;
                }
                if run.stillness_at_cross.is_none() && r.head_after.0 < wall_x {
                    run.stillness_at_cross = Some(r.stillness);
                }
            }
            Ground::UBend => run.peak = run.peak.max(start.0 - r.head_after.0),
            Ground::Flat { .. } => {}
        }
    }
    run.pmove_home /= run.facing_home.max(1) as f64;
    run.pmove_away /= run.facing_away.max(1) as f64;
    run
}

fn report_laden(label: &str, runs: &[LadenRun]) {
    let n: u64 = runs.iter().map(|r| r.decisions).sum();
    let share = |f: &dyn Fn(&LadenRun) -> u64| 100.0 * runs.iter().map(f).sum::<u64>() as f64 / n.max(1) as f64;
    let arrived: Vec<f64> = runs.iter().filter_map(|r| r.arrived_at.map(|d| d as f64)).collect();
    let fired: u64 = runs.iter().map(|r| r.fired).sum();
    let fired_away: u64 = runs.iter().map(|r| r.fired_away).sum();
    let home_n: u64 = runs.iter().map(|r| r.facing_home).sum();
    let away_n: u64 = runs.iter().map(|r| r.facing_away).sum();
    let frozen = runs.iter().map(|r| r.longest_still).max().unwrap_or(0);
    let chose: u64 = runs.iter().map(|r| r.chose).sum();
    let chose_away: u64 = runs.iter().map(|r| r.chose_away).sum();
    let min_pat = runs.iter().map(|r| r.min_patience).filter(|p| !p.is_nan()).reduce(f32::min).map_or("-".to_string(), |p| format!("{p:.3}"));
    let pm = |sel: &dyn Fn(&LadenRun) -> (f64, u64)| {
        let (s, c) = runs.iter().fold((0.0, 0u64), |(s, c), r| {
            let (m, k) = sel(r);
            (s + m * k as f64, c + k)
        });
        if c == 0 { f64::NAN } else { s / c as f64 }
    };
    println!(
        "  {label}: arrived {} of {} (median {} decisions) | step {:.1}%  tumble {:.1}%  idle {:.1}%  blocked {:.1}% | facing home {:.1}% (p_move {:.2}), away {:.1}% (p_move {:.2}) | re-roll fired {fired}, {fired_away} of them pointing away | longest stand-still {frozen} decisions | chooser stepped {chose}, {chose_away} of them pointing away, patience fell to {min_pat}",
        arrived.len(),
        runs.len(),
        if arrived.is_empty() { "-".to_string() } else { format!("{:.0}", median(arrived)) },
        share(&|r| r.stepped),
        share(&|r| r.tumbled),
        share(&|r| r.idle),
        share(&|r| r.blocked),
        100.0 * home_n as f64 / n.max(1) as f64,
        pm(&|r| (r.pmove_home, r.facing_home)),
        100.0 * away_n as f64 / n.max(1) as f64,
        pm(&|r| (r.pmove_away, r.facing_away)),
    );
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn main() {
    let scene = arg_str("scene", "s0");
    let seeds: u64 = arg("seeds", 24);
    let seed0: u64 = arg("seed0", 1);
    let frames: u64 = arg("frames", 24_000);
    let gap: i32 = arg("gap", 90);
    let energies: Vec<f32> = arg_str("energies", "1.0,0.5").split(',').map(|v| v.parse().expect("energies=a,b")).collect();
    println!(
        "scenes: scene={scene} seeds={seeds} seed0={seed0} frames={frames} gap={gap} energies={energies:?} DROP_REACH={}",
        std::env::var("PIXEL_PHYSICS_DROP_REACH").unwrap_or_else(|_| "shipped".into())
    );
    println!(
        "  REVERSE={} CHOOSER={}",
        std::env::var("PIXEL_PHYSICS_REVERSE").unwrap_or_else(|_| "shipped".into()),
        std::env::var("PIXEL_PHYSICS_CHOOSER").unwrap_or_else(|_| "off".into())
    );
    if scene != "s0" {
        let arms: Vec<(String, Ground)> = match scene.as_str() {
            "s1" => vec![("S1 home 40 east (facing home)".into(), Ground::Flat { dx: 40 }), ("S1 home 40 west (facing away)".into(), Ground::Flat { dx: -40 })],
            "s2" => arg_str("heights", "1,2,3,6,12")
                .split(',')
                .map(|v| v.parse::<i32>().expect("heights=a,b"))
                .map(|hgt| (format!("S2 wall of height {hgt}"), Ground::Wall { height: hgt }))
                .collect(),
            "s3" => vec![("S3 U-bend".into(), Ground::UBend)],
            other => panic!("unknown scene {other}"),
        };
        for (label, ground) in arms {
            println!("\n{label}");
            println!("  {:>4} {:>9} {:>8} {:>7} {:>7} {:>7} {:>7} {:>6} {:>6} {:>9} {:>7}", "seed", "decisions", "arrived", "step", "tumble", "idle", "blocked", "peak", "atwall", "still@x", "frozen");
            let mut runs = Vec::new();
            for s in seed0..seed0 + seeds {
                let r = laden(s, ground, frames);
                let n = r.decisions as f64;
                println!(
                    "  {:>4} {:>9} {:>8} {:>6.1}% {:>6.1}% {:>6.1}% {:>6.1}% {:>6} {:>6} {:>9} {:>7}",
                    r.seed,
                    r.decisions,
                    r.arrived_at.map_or("-".into(), |d| d.to_string()),
                    100.0 * r.stepped as f64 / n,
                    100.0 * r.tumbled as f64 / n,
                    100.0 * r.idle as f64 / n,
                    100.0 * r.blocked as f64 / n,
                    r.peak,
                    r.at_wall,
                    r.stillness_at_cross.map_or("-".into(), |v| format!("{v:.2}")),
                    r.longest_still
                );
                runs.push(r);
            }
            report_laden(&label, &runs);
        }
        return;
    }

    println!("\nS0: an empty ant on a bare slab, no trail, food {gap} cells away (prediction in this file's header)");
    println!(
        "  {:>4} {:>6} {:>9} {:>8} {:>8} {:>8} {:>8} {:>8} {:>7} {:>6} {:>7}",
        "seed", "energy", "decisions", "step", "reverse", "re-roll", "nothing", "cells/d", "run", "reach", "found@"
    );
    let mut all: Vec<S0Run> = Vec::new();
    for &e in &energies {
        for s in seed0..seed0 + seeds {
            let r = s0(s, e, frames, gap);
            let n = r.decisions as f64;
            println!(
                "  {:>4} {:>6.2} {:>9} {:>7.1}% {:>7.1}% {:>7.1}% {:>7.1}% {:>8.3} {:>7.1} {:>6} {:>7}",
                r.seed,
                r.energy,
                r.decisions,
                100.0 * r.stepped as f64 / n,
                100.0 * r.reversed as f64 / n,
                100.0 * r.rerolled as f64 / n,
                100.0 * r.idle as f64 / n,
                r.cells_per_decision,
                r.run_length,
                r.reach,
                r.found_at.map_or("-".to_string(), |d| format!("{d}{}", if r.found_east == Some(true) { "E" } else { "W" }))
            );
            assert_eq!(r.other, 0, "a decision on a bare slab was neither a step, a tumble nor idle");
            all.push(r);
        }
    }

    println!("\n  SUMMARY (pooled shares over decisions; medians over runs; key cardinality {} runs = {} energies x {seeds} seeds)", all.len(), energies.len());
    for &e in &energies {
        {
            let arm: Vec<&S0Run> = all.iter().filter(|r| r.energy == e).collect();
            let n: u64 = arm.iter().map(|r| r.decisions).sum();
            let share = |f: &dyn Fn(&S0Run) -> u64| 100.0 * arm.iter().map(|r| f(r)).sum::<u64>() as f64 / n as f64;
            let found = arm.iter().filter(|r| r.found_at.is_some()).count();
            let east = arm.iter().filter(|r| r.found_east == Some(true)).count();
            println!(
                "  energy {e:.2}: step {:.1}%  reverse {:.1}%  re-roll {:.1}%  nothing {:.1}% | steps between reversals, median {:.1} | furthest from the start, median {:.0} cells | found food in {found} of {} runs, east side {east} (median decisions to find it: {})",
                share(&|r| r.stepped),
                share(&|r| r.reversed),
                share(&|r| r.rerolled),
                share(&|r| r.idle),
                median(arm.iter().map(|r| r.run_length).collect()),
                median(arm.iter().map(|r| r.reach as f64).collect()),
                arm.len(),
                {
                    let v: Vec<f64> = arm.iter().filter_map(|r| r.found_at.map(|d| d as f64)).collect();
                    if v.is_empty() { "-".to_string() } else { format!("{:.0}", median(v)) }
                }
            );
        }
    }
}
