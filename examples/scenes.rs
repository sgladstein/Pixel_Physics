//! **The movement plan's scenes: small, controlled places where the ant's
//! walk has a known answer** (`Reports/ant-movement-plan-2026-09-22.md` §6).
//!
//! ```text
//! cargo run --release --example scenes -- scene=s0              # 24 seeds, all arms
//! cargo run --release --example scenes -- scene=s0 seeds=4 frames=6000
//! ```
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
use pixel_physics::sim::brain::{self, BrainOutput as O};
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::creature::{self, DecisionOutcome as D, DecisionRow};
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
    let corridor = mine.iter().filter(|r| r.usable == (1 << 0) | (1 << 4)).count();
    if (corridor as f64) < 0.99 * mine.len() as f64 {
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
            "only {corridor} of {} decisions saw a bare-slab corridor; (usable mask, head row) -> {masks:?}; (usable, heading) -> {by_heading:?}; first odd: {odd:?}",
            mine.len()
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
            D::Stepped => run.stepped += 1,
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
    assert_eq!(scene, "s0", "only scene=s0 is built so far");
    let _ = creature::DECISION_OUTCOME_NAMES;

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
