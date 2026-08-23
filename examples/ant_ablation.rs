//! **Is the authored ant brain doing anything, or is the substrate?**
//!
//! Runs one colony scene repeatedly, varying only the genome, and reports
//! behaviour rather than event totals. Three kinds of arm:
//!
//! * `zero` — no instincts at all. The control, and the most important arm:
//!   without it there is no way to say whether any number here is good.
//!   Every metric below is sanity-checked against it before any ablation is
//!   trusted (`CLAUDE.md`: ask what a metric counts when nothing is wrong).
//! * `authored` — the shipped `ant.ron` genome.
//! * `-<Input>→<Output>` — authored with exactly one instinct zeroed.
//!
//! # Why these metrics and not the ones the scenes print
//!
//! `examples/ascii.rs` prints event counts summed over the whole run and
//! the whole colony — moves, pickups, deliveries. Those cannot tell a
//! colony vibrating on the spot from one commuting, cannot tell five busy
//! ants from fifty, and have nothing at all to say about spatial range,
//! which is the thing actually failing. The range problem was originally
//! diagnosed by *reading an ascii picture*, because no number described it.
//!
//! So:
//!
//! * **travelled** — p90 over ants of the furthest each got from *its own
//!   starting cell*. An order statistic, because outcome spread here is
//!   enormous and a mean hides it. Measured from the nest instead, this
//!   read 118 for a colony that provably never moved — see `run_one`.
//! * **commute** — median per-ant net displacement over path length. The
//!   metric the Stage-0 resolution experiment used to catch its own false
//!   positive (0.988 proximity while advancing 21 cells in 400 steps) and
//!   which was then not carried over to the colony. Milling reads ~0;
//!   travelling reads toward 1.
//! * **coverage** — distinct cells any ant ever occupied.
//! * **roamed** — fraction of ants that ever got more than 16 cells from
//!   where they started, and **foraged** — fraction that ever picked
//!   something up. Participation, which every colony-summed counter hides.
//! * **first-pickup** — frames to the first one. A rate question, not a
//!   total.
//!
//! ```text
//! cargo run --release --example ant_ablation
//! cargo run --release --example ant_ablation -- seeds=6 frames=8000
//! ```

use std::collections::HashSet;

use pixel_physics::sim::brain::{BrainInput, BrainOutput, Instinct};
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material;
use pixel_physics::sim::organism::CellType;
use pixel_physics::sim::{organism, parallel, rng, Cell, World};

/// The authored instincts, mirrored from `ant.ron` so an arm can drop one.
/// Kept in step with the file by `authored_matches_the_species_file` below,
/// which fails loudly rather than letting the sweep silently ablate a
/// genome nobody ships.
const AUTHORED: &[(BrainInput, BrainOutput, f32)] = &[
    (BrainInput::TempAboveAmb, BrainOutput::Turn, -0.8),
    (BrainInput::Bias, BrainOutput::Move, 2.0),
    (BrainInput::FoodAdjacent, BrainOutput::Move, -1.5),
    (BrainInput::Crowding, BrainOutput::Move, -0.3),
    (BrainInput::Carrying, BrainOutput::EmitB, 2.5),
    (BrainInput::Bias, BrainOutput::EmitA, 2.0),
    (BrainInput::Bias, BrainOutput::Dig, 0.4),
    (BrainInput::FoodAdjacent, BrainOutput::Dig, 0.8),
    (BrainInput::Bias, BrainOutput::Feed, 0.4),
    (BrainInput::FoodAdjacent, BrainOutput::Feed, 0.8),
    (BrainInput::AtNest, BrainOutput::Drop, 0.9),
    (BrainInput::Carrying, BrainOutput::Drop, 0.2),
];

/// **The guard the comment above has been promising.** It named this
/// function and this function did not exist, so the copy was unchecked for
/// as long as anyone has been reading that sentence and believing it.
///
/// **This list is a copy of `ant.ron`'s, and a copy drifts.** It cannot
/// simply *be* the species list, because the whole harness is "the authored
/// animal minus one named connection" and that needs the sparse entries
/// rather than the expanded genome. So it is checked instead: expand this
/// list with the species' own hidden wiring and require it to equal the
/// compiled genome, exactly.
///
/// The Feed/Dig split is what this is for. Adding `BrainOutput::Feed`
/// without touching this file would have left the `authored` arm with no
/// Feed weights at all — an ant that never eats — and the ablation would
/// have reported that as a result about feeding rather than as a stale
/// copy. Silent, plausible, and wrong in the direction the experiment was
/// looking.
fn authored_matches_the_species_file(world: &World) {
    let species = world.species.id_of("ant").expect("ant species");
    let def = world.species.get(species).creature.as_ref().expect("ant is a creature");
    let from_copy = pixel_physics::sim::brain::genome_from_wiring(
        &AUTHORED.iter().map(|&(i, o, w)| Instinct(i, o, w)).collect::<Vec<_>>(),
        &def.hidden_wiring,
        &def.hidden_outputs,
    );
    assert_eq!(
        from_copy,
        world.species.get(species).genome,
        "AUTHORED has drifted from assets/species/ant.ron -- every arm below would be an ablation of an animal that does not exist"
    );
}

#[derive(Default, Clone, Copy)]
struct Metrics {
    range: f32,
    commute: f32,
    coverage: f32,
    left_nest: f32,
    foraged: f32,
    first_pickup: f32,
    pickups: f32,
    deliveries: f32,
    /// **The pair the Feed/Dig split exists to separate.** Reported side by
    /// side because the claim is about their *independence*: before the
    /// split one weight moved both, so this pair could only ever move
    /// together, and "they move together" would have looked like a result
    /// rather than like a missing gene.
    eats: f32,
    digs: f32,
}

fn main() {
    let mut seeds = 5u64;
    let mut frames = 6000usize;
    let mut rough = true;
    let mut world_terrain = false;
    let mut trees = false;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "seeds" => seeds = v.parse().expect("seeds"),
            "frames" => frames = v.parse().expect("frames"),
            "food" => trees = v == "trees",
            "terrain" => {
                rough = v != "flat";
                world_terrain = v == "world";
            }
            other => panic!("unknown arg {other:?}; known: seeds, frames, terrain, food"),
        }
    }

    // **The two knobs that change what the arms are measured *on*, echoed.**
    // `terrain=` and `food=` pick the scene; neither appeared in the output,
    // so two logs from different worlds were indistinguishable at a glance
    // and a flag that never reached the binary looked exactly like one that
    // did (`CLAUDE.md`, the megastudy post-mortem).
    let terrain_label = if world_terrain {
        "world (generated)"
    } else if rough {
        "rough"
    } else {
        "flat"
    };
    println!("ant_ablation: terrain={terrain_label} food={} seeds={seeds} frames={frames}", if trees { "trees" } else { "corpse pile" });

    authored_matches_the_species_file(&World::new(Rect::new(0, 0, 15, 15)));

    // Arm 0 is the control and arm 1 the full genome; everything after is
    // the full genome minus one instinct.
    let mut arms: Vec<(String, Vec<Instinct>)> = vec![
        ("zero (CONTROL)".into(), Vec::new()),
        ("authored".into(), AUTHORED.iter().map(|&(i, o, w)| Instinct(i, o, w)).collect()),
    ];
    // **The new locomotion outputs, swept at both extremes.** An ablation
    // can only show that removing something matters; for a knob whose
    // silent value is mid-scale, the question is whether *either
    // direction* changes anything. If these read identical to `authored`,
    // the genome still has no authority over locomotion and evolution
    // would have nothing to select on.
    for (out, label) in [(BrainOutput::Persist, "Persist"), (BrainOutput::Tumble, "Tumble"), (BrainOutput::Caution, "Caution")] {
        for (w, sign) in [(-4.0f32, "lo"), (4.0, "hi")] {
            let mut v: Vec<Instinct> = AUTHORED.iter().map(|&(i, o, w)| Instinct(i, o, w)).collect();
            v.push(Instinct(BrainInput::Bias, out, w));
            arms.push((format!("  {label}={sign}"), v));
        }
    }
    for (dropped, &(i, o, _)) in AUTHORED.iter().enumerate() {
        arms.push((
            format!("-{i:?}->{o:?}"),
            AUTHORED
                .iter()
                .enumerate()
                .filter(|(n, _)| *n != dropped)
                .map(|(_, &(i, o, w))| Instinct(i, o, w))
                .collect(),
        ));
    }

    println!("ant ablation: {seeds} seeds x {frames} frames per arm, hidden wiring left intact in every arm\n");
    println!(
        "{:<26} {:>7} {:>8} {:>9} {:>10} {:>9} {:>12} {:>8} {:>7} {:>7} {:>7}",
        "arm", "travelled", "commute", "coverage", "roamed", "foraged", "first-pickup", "pickups", "deliv", "eats", "digs"
    );

    let mut control = Metrics::default();
    for (n, (label, instincts)) in arms.iter().enumerate() {
        let runs: Vec<Metrics> = (0..seeds).map(|s| run_one(instincts, frames, 0xA17 + s, rough, world_terrain, trees)).collect();
        let m = mean(&runs);
        if n == 0 {
            control = m;
        }
        println!(
            "{label:<26} {:>7.1} {:>8.3} {:>9.0} {:>10.2} {:>9.2} {:>12} {:>8.1} {:>7.1} {:>7.1} {:>7.1}",
            m.range,
            m.commute,
            m.coverage,
            m.left_nest,
            m.foraged,
            if m.first_pickup < 0.0 { "never".to_string() } else { format!("{:.0}", m.first_pickup) },
            m.pickups,
            m.deliveries,
            m.eats,
            m.digs
        );
        if n == 1 {
            // **The sanity check, before any ablation number is read.** If
            // the full genome does not separate from a brain with no
            // connections at all, these metrics are not measuring
            // behaviour and nothing below them means anything.
            println!(
                "{:<26} {:>7.1} {:>8.3} {:>9.0} {:>10.2} {:>9.2}   <- authored minus control",
                "  SEPARATION",
                m.range - control.range,
                m.commute - control.commute,
                m.coverage - control.coverage,
                m.left_nest - control.left_nest,
                m.foraged - control.foraged
            );
            println!();
        }
    }
}

fn mean(runs: &[Metrics]) -> Metrics {
    let n = runs.len() as f32;
    let picked: Vec<f32> = runs.iter().map(|r| r.first_pickup).filter(|v| *v >= 0.0).collect();
    Metrics {
        range: runs.iter().map(|r| r.range).sum::<f32>() / n,
        commute: runs.iter().map(|r| r.commute).sum::<f32>() / n,
        coverage: runs.iter().map(|r| r.coverage).sum::<f32>() / n,
        left_nest: runs.iter().map(|r| r.left_nest).sum::<f32>() / n,
        foraged: runs.iter().map(|r| r.foraged).sum::<f32>() / n,
        first_pickup: if picked.is_empty() { -1.0 } else { picked.iter().sum::<f32>() / picked.len() as f32 },
        pickups: runs.iter().map(|r| r.pickups).sum::<f32>() / n,
        deliveries: runs.iter().map(|r| r.deliveries).sum::<f32>() / n,
        eats: runs.iter().map(|r| r.eats).sum::<f32>() / n,
        digs: runs.iter().map(|r| r.digs).sum::<f32>() / n,
    }
}

const NEST_X0: i32 = 16;
const NEST_X1: i32 = 90;
const ANTS: usize = 52;

fn run_one(instincts: &[Instinct], frames: usize, seed: u64, rough: bool, world_terrain: bool, trees: bool) -> Metrics {
    let (w, h) = (512i32, 120i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    world.seed = seed;
    let floor = h - 8;
    let nest = world.materials.id_of("nest").expect("nest");
    let corpse = world.materials.id_of("corpse").expect("corpse");
    let ant_material = world.materials.id_of("ant").expect("ant");
    let species = world.species.id_of("ant").expect("ant species");

    // Only the input->output block varies; the hidden wiring is the same in
    // every arm, so an ablation cannot be confounded by the gate changing
    // underneath it.
    let def = world.species.get(species).creature.as_ref().expect("ant is a creature").clone();
    let genome = pixel_physics::sim::brain::genome_from_wiring(instincts, &def.hidden_wiring, &def.hidden_outputs);
    world.species.set_genome(species, genome);

    // Real generated terrain, when asked for: the hand-built profile below
    // is a controlled stand-in, and a result that only holds for it is a
    // result about my sine wave rather than about the game.
    if world_terrain {
        let (presets, _) = pixel_physics::worldgen::WorldgenPresets::load();
        let name = presets.default_name();
        if let Some(params) = presets.get(&name) {
            pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed: seed as u32 as u64 });
        }
    }

    // **Terrain, because a flat floor gives a creature nothing to decide.**
    // On level ground an ant's up-diagonal candidates have no foothold and
    // its down-diagonal is inside the floor, so it usually has exactly one
    // viable step — and a knob that scores candidates cannot matter when
    // there is only one. Measured on flat ground, `Persist` swept from 0
    // to full scale moved commute from 0.023 to 0.023.
    //
    // A deterministic ridge-and-pit profile: pure function of x, so it is
    // identical across arms and cannot confound an ablation, while still
    // presenting slopes, steps and hollows where more than one step is
    // available.
    let hand_height = |x: i32| -> i32 {
        let a = (rng::jitter(x / 24, 7) * 10.0) as i32;
        let b = (rng::jitter(x / 7, 13) * 4.0) as i32;
        floor - if rough { a + b } else { 0 }
    };
    if !world_terrain {
        for x in 0..w {
            for y in hand_height(x)..h {
                world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
    }
    // The surface, whichever way the world was made: the topmost solid or
    // powder cell in each column. Generated terrain has no fixed floor row,
    // so everything below has to ask the world rather than assume.
    let surface_of = |world: &World, x: i32| -> i32 {
        (0..h)
            .find(|&y| matches!(world.materials.kind(world.get(x, y).material), material::MaterialKind::Solid | material::MaterialKind::Powder))
            .unwrap_or(h - 1)
    };
    let height = |world: &World, x: i32| -> i32 { if world_terrain { surface_of(world, x) } else { hand_height(x) } };
    for x in NEST_X0..NEST_X1 {
        let sy = height(&world, x);
        world.set(x, sy, Cell::new(nest, 0).with_attached(true));
    }
    if trees {
        // **A renewable food source.** A living tree regrows leaves, so a
        // stand of them replenishes while a corpse pile does not -- which
        // is the difference between a run that ends in guaranteed
        // starvation and one where a genome can be better or worse at
        // staying alive. Selection needs the second kind.
        for i in 0..6 {
            let x = 230 + i * 40;
            let sy = height(&world, x);
            world.plant_tree(x, sy - 1);
        }
        for _ in 0..2400 {
            world.step_active_sites();
            world.step_fields();
        }
    } else {
        for x in 250..300 {
            let sy = height(&world, x);
            for y in (sy - 5)..sy {
                world.set(x, y, Cell::new(corpse, 0));
            }
        }
    }
    for i in 0..ANTS {
        let x = 24 + i as i32 * 4;
        let sy = height(&world, x);
        world.plant_ant(x, sy - 1);
    }

    // Per-ant tracking, keyed on the organism handle so a died-and-reused
    // slot is not silently averaged into its predecessor's path.
    let mut path_len: std::collections::HashMap<u16, f32> = std::collections::HashMap::new();
    let mut last_pos: std::collections::HashMap<u16, (i32, i32)> = std::collections::HashMap::new();
    let mut start_pos: std::collections::HashMap<u16, (i32, i32)> = std::collections::HashMap::new();
    let mut max_range: std::collections::HashMap<u16, f32> = std::collections::HashMap::new();
    // Silence the now-unused nest bounds without deleting them: they name
    // the scene's geometry and the next metric may well want them.
    let _ = (NEST_X0, NEST_X1);
    let mut ever_carried: HashSet<u16> = HashSet::new();
    let mut visited: HashSet<(i32, i32)> = HashSet::new();
    let mut first_pickup = -1.0f32;

    for frame in 0..frames {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();

        // Sampled rather than every frame: an ant ticks once in six, so
        // per-frame sampling would multiply every path length by six
        // without adding a single bit of information.
        if frame % 6 != 0 {
            continue;
        }
        for x in 0..w {
            for y in 0..h {
                let c = world.get(x, y);
                if c.material != ant_material || organism::cell_type(c.aux()) != Some(CellType::Head) {
                    continue;
                }
                let id = c.organism_id();
                visited.insert((x, y));
                if let Some(&(px, py)) = last_pos.get(&id) {
                    let d = (((x - px).pow(2) + (y - py).pow(2)) as f32).sqrt();
                    *path_len.entry(id).or_insert(0.0) += d;
                } else {
                    start_pos.insert(id, (x, y));
                }
                last_pos.insert(id, (x, y));
                // **Displacement from this ant's own start, not from the
                // nest** -- and the difference is the whole reason the
                // control arm exists. Measured from the nest, a colony
                // with a brain of literally zero connections, which never
                // moves a single cell, scored range 118 and "left the
                // nest" 0.63: the ants are *placed* across x=24..228, so
                // both numbers were reporting the spawn layout. A metric
                // that reads high when nothing whatsoever has happened is
                // measuring the initial condition (`CLAUDE.md`: ask what a
                // metric counts when nothing is wrong).
                if let Some(&(sx, sy)) = start_pos.get(&id) {
                    let moved = ((((x - sx).pow(2) + (y - sy).pow(2)) as f32).sqrt()).abs();
                    let e = max_range.entry(id).or_insert(0.0);
                    *e = e.max(moved);
                }
                if world.organism(id).is_some_and(|s| s.carrying.is_some())
                    && ever_carried.insert(id)
                    && first_pickup < 0.0
                {
                    first_pickup = frame as f32;
                }
            }
        }
    }

    let mut ranges: Vec<f32> = max_range.values().copied().collect();
    ranges.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p90 = if ranges.is_empty() { 0.0 } else { ranges[(ranges.len() * 9 / 10).min(ranges.len() - 1)] };

    let mut commutes: Vec<f32> = last_pos
        .iter()
        .filter_map(|(id, &(x, y))| {
            let &(sx, sy) = start_pos.get(id)?;
            let path = *path_len.get(id)?;
            if path < 1.0 {
                return Some(0.0);
            }
            Some(((((x - sx).pow(2) + (y - sy).pow(2)) as f32).sqrt()) / path)
        })
        .collect();
    commutes.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let commute = if commutes.is_empty() { 0.0 } else { commutes[commutes.len() / 2] };

    let tracked = max_range.len().max(1) as f32;
    let st = world.creature_stats;
    Metrics {
        range: p90,
        commute,
        coverage: visited.len() as f32,
        left_nest: max_range.values().filter(|v| **v > 16.0).count() as f32 / tracked,
        foraged: ever_carried.len() as f32 / tracked,
        first_pickup,
        pickups: st.pickups as f32,
        deliveries: st.deliveries as f32,
        eats: st.eats as f32,
        digs: st.digs as f32,
    }
}
