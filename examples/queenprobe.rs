//! **The three numbers the play-as-an-ant game needs before any game code**
//! (`Reports/play-as-ant-slice0-2026-10-11.md`). Each runs the lab's own
//! box headless with mutation off, one seed a run, and ends with one
//! `RESULT` line for a sweep to collect.
//!
//! - `mode=trail`: **does a trail laid at queen pace pull empty workers?** A
//!   colony of `workers=` (20) lands; at `start=` frames after landing one
//!   founder becomes the steered body (`sim::steer`) and walks out to a
//!   target with no trail, then back to the nest. `arm=line` holds T on the
//!   way back from the food, so she lays trail B one cell a step at walking
//!   pace through `steer`'s bypass of `food_trail_lay` -- the stimulus the
//!   game gives, not the scent tool's instant ramp. `arm=none` walks the
//!   same route with T up. `arm=placebo` walks the same distance the other
//!   way and lays the line back from there, away from the food. The count
//!   is **workers that reach the food** (distinct ants, the steered one
//!   excluded) in the `window=` frames (6,000) from the moment she turns
//!   for home, and ant-ticks spent there. The nudged baseline is
//!   `arm=none land=6001`. **`arm=ramp` is the positive control**: her walk
//!   as `none`, and from the moment she turns for home the harness itself
//!   keeps a trail B ramp laid along the ground from the nest (0) to the
//!   food (`pheromone::DEPOSIT`, the scent tool's peak) every 12 frames for
//!   the whole window -- the strongest trail the game could ever give. If it
//!   moves nothing either, the count cannot see a trail's pull in this bed.
//! - `mode=found`: **can one queen reach twenty workers?** A colony of one,
//!   the steered body with no keyboard (her genome drives her), so no old
//!   age, no caste and the queen rule for her colony; `reserve=` joules
//!   granted on landing, booked as `Granted`. Every 1,000 frames her
//!   colony's adults, eggs, larvae and pupae; the frame she first has
//!   `goal=` (20) adults besides herself; and every death in her colony by
//!   cause.
//! - `mode=beetle`: **how often does a beetle kill a lone ant?** As `found`,
//!   with `beetles=` (4) released at landing (the `one_nest_beetles` bed).
//!   Reports her death frame and cause.
//!
//! Common args: `seed=1 frames=<per mode> scenario=steady_income gap=60`
//! (the heap's distance east of the colony), `land=` (the colony's landing
//! frame, 6,000 by default; 6,001 is the nudged baseline), `mutation=on` to
//! leave births mutating. Run with `RAYON_NUM_THREADS=1` for determinism
//! across machines.
//!
//! `cargo run --release --example queenprobe -- mode=found seed=1 reserve=900`

use pixel_physics::lab::scenario::{Placement, Scenario};
use pixel_physics::lab::Lab;
use pixel_physics::sim::cell::OrganismId;
use pixel_physics::sim::organism::{self, BroodStage};
use pixel_physics::sim::steer::{self, SteerInput};
use pixel_physics::sim::world::World;
use std::collections::BTreeSet;

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{name}=")).and_then(|v| v.parse().ok()))
}

/// The food zone, as `deeptrace` draws it: within 12 columns of the heap and
/// from 40 rows above the ground to 4 below.
const FOOD_REACH: i32 = 12;

fn in_food(x: i32, y: i32, food_x: i32, ground_y: i32) -> bool {
    (x - food_x).abs() <= FOOD_REACH && y <= ground_y + 4 && y >= ground_y - 40
}

/// Her colony's census: adults (herself included), eggs, larvae, pupae.
fn census(w: &World, colony: u32, ant: organism::SpeciesId) -> (usize, usize, usize, usize) {
    let mut n = (0, 0, 0, 0);
    for id in w.live_organism_ids() {
        if w.organism(id).is_some_and(|s| s.colony == colony && s.species == ant) {
            n.0 += 1;
        }
    }
    for id in w.live_brood_ids() {
        let Some(s) = w.organism(id) else { continue };
        if s.colony != colony {
            continue;
        }
        match s.brood.map(|b| b.stage) {
            Some(BroodStage::Egg) => n.1 += 1,
            Some(BroodStage::Larva) => n.2 += 1,
            Some(BroodStage::Pupa) => n.3 += 1,
            None => {}
        }
    }
    n
}

fn causes_since(w: &World, before: &[u64; organism::DEATH_CAUSES]) -> String {
    let c: Vec<&str> = organism::DEATH_CAUSE_LIST
        .iter()
        .filter(|c| w.deaths_by_cause[c.index()] > before[c.index()])
        .map(|c| c.label())
        .collect();
    if c.is_empty() {
        "?".into()
    } else {
        c.join("+")
    }
}

fn main() {
    let mutation = arg::<String>("mutation").as_deref() == Some("on");
    if !mutation {
        pixel_physics::sim::creature::mutation_off_for_measuring();
    }
    let mode: String = arg("mode").unwrap_or_else(|| "found".into());
    let seed: u64 = arg("seed").unwrap_or(1);
    let land: u64 = arg("land").unwrap_or(6000);
    let gap: i32 = arg("gap").unwrap_or(60);
    let reserve: f32 = arg("reserve").unwrap_or(0.0);
    let workers: i32 = arg("workers").unwrap_or(20);
    let beetles: usize = arg("beetles").unwrap_or(4);
    let goal: usize = arg("goal").unwrap_or(20);
    let start: u64 = arg("start").unwrap_or(1000);
    let window: u64 = arg("window").unwrap_or(6000);
    let arm: String = arg("arm").unwrap_or_else(|| "line".into());
    let default_frames = match mode.as_str() {
        "trail" => land + start + 40_000,
        "beetle" => land + 100_000,
        _ => land + 432_000,
    };
    let frames: u64 = arg("frames").unwrap_or(default_frames);
    let scenario: String = arg("scenario").unwrap_or_else(|| "steady_income".into());
    assert!(
        matches!(mode.as_str(), "trail" | "found" | "beetle"),
        "mode= is trail, found or beetle"
    );
    assert!(
        matches!(arm.as_str(), "line" | "none" | "placebo" | "ramp"),
        "arm= is line, none, placebo or ramp"
    );
    println!(
        "queenprobe: mode={mode} seed={seed} frames={frames} scenario={scenario} land={land} gap={gap} reserve={reserve} workers={workers} beetles={beetles} goal={goal} start={start} window={window} arm={arm} mutation={}",
        if mutation { "on" } else { "off" }
    );

    let mut sc = Scenario::load(&scenario).unwrap_or_else(|e| {
        eprintln!("scenario {scenario}: {e}");
        std::process::exit(1);
    });
    sc.bed.seed = seed;
    let mut colony_x = None;
    for e in &mut sc.timeline {
        if let Placement::Colony { x, count, .. } = &mut e.what {
            *count = if mode == "trail" { workers } else { 1 };
            e.at = land;
            colony_x = Some(*x);
        }
    }
    let colony_x = colony_x.expect("the scenario needs a Colony event");
    for e in &mut sc.timeline {
        if let Placement::Heap { material, x, .. } = &mut e.what {
            if material == "provisions" {
                *x = colony_x + gap;
            }
        }
    }
    sc.timeline.retain(|e| !matches!(e.what, Placement::Predators { .. }));
    if mode == "beetle" {
        sc.timeline.push(pixel_physics::lab::scenario::Event {
            at: land,
            every: 0,
            until: 0,
            what: Placement::Predators {
                species: "beetle".into(),
                count: beetles,
            },
        });
    }
    let food_x = colony_x + gap;
    let mut lab = Lab::new(sc.bed.clone());
    let msg = lab.load_scenario(sc);
    println!("  {msg}");
    let ground_y = lab.spec.ground_y;
    let ant = lab.world.species.id_of("ant").expect("ant ships");

    let mut queen: Option<(OrganismId, u32)> = None;
    let mut queen_died: Option<(u64, String)> = None;
    let mut reached: Option<u64> = None;
    let mut peak = 0usize;
    // trail: 0 walking out, 1 walking home, 2 released; the frame home began.
    let mut phase = 0u8;
    let mut home_from: Option<u64> = None;
    let mut home_at: Option<u64> = None;
    let mut reached_food: BTreeSet<OrganismId> = BTreeSet::new();
    let mut ticks_at_food = 0u64;
    let mut stuck = 0u32;
    let mut last_head = (i32::MIN, i32::MIN);
    let mut deaths_before = lab.world.deaths_by_cause;
    let mut colony_deaths: Vec<(u64, String)> = Vec::new();
    let mut known: BTreeSet<OrganismId> = BTreeSet::new();

    for f in 0..frames {
        let before = lab.world.deaths_by_cause;
        lab.tick_for_harness();
        let frame = lab.world.frame;
        // Mark her the frame the colony lands.
        if queen.is_none() && frame > land {
            let mut ids: Vec<OrganismId> = lab
                .world
                .live_organism_ids()
                .into_iter()
                .filter(|&id| lab.world.organism(id).is_some_and(|s| s.species == ant))
                .collect();
            ids.sort();
            if let Some(&id) = ids.first() {
                let input = (mode == "trail").then(SteerInput::default);
                assert!(steer::mark(&mut lab.world, id, input));
                let granted = steer::grant(&mut lab.world, reserve);
                let colony = lab.world.organism(id).map_or(0, |s| s.colony);
                let e = lab.world.organism(id).map_or(0.0, |s| s.energy);
                println!("  frame {frame}: steered body {id} of colony {colony}, granted {granted} J, bank {e:.0} J, {} ants landed", ids.len());
                queen = Some((id, colony));
                deaths_before = lab.world.deaths_by_cause;
            }
        }
        let Some((qid, colony)) = queen else { continue };
        let alive = lab.world.organism(qid).is_some();
        if !alive && queen_died.is_none() {
            let cause = causes_since(&lab.world, &before);
            println!("  frame {frame}: the steered body died ({cause})");
            queen_died = Some((frame, cause));
        }
        // Colony deaths by cause: an adult of hers that is gone this frame.
        if mode != "trail" {
            let now: BTreeSet<OrganismId> = lab
                .world
                .live_organism_ids()
                .into_iter()
                .filter(|&id| {
                    lab.world
                        .organism(id)
                        .is_some_and(|s| s.colony == colony && s.species == ant)
                })
                .collect();
            for gone in known.difference(&now) {
                colony_deaths.push((frame, format!("{gone}:{}", causes_since(&lab.world, &before))));
            }
            known = now;
        }

        match mode.as_str() {
            "trail" => {
                let head = lab.world.organism(qid).and_then(|s| s.chain.first().copied());
                if phase < 2 && frame >= land + start {
                    if let Some((hx, hy)) = head {
                        let out_x = if arm == "placebo" { colony_x - gap } else { food_x };
                        let tx = if phase == 0 { out_x } else { colony_x };
                        if (hx - tx).abs() <= 2 {
                            if phase == 0 {
                                phase = 1;
                                home_from = Some(frame);
                                println!(
                                    "  frame {frame}: reached x {hx} after {} frames; turning for home, trail {}",
                                    frame - land - start,
                                    if arm == "line" || arm == "placebo" {
                                        "held"
                                    } else {
                                        "up"
                                    }
                                );
                            } else {
                                phase = 2;
                                home_at = Some(frame);
                                let b = |x: i32| {
                                    (ground_y - 30..=ground_y + 2)
                                        .map(|y| {
                                            lab.world.pheromone_at(pixel_physics::sim::pheromone::Channel::B, x, y)
                                        })
                                        .max()
                                        .unwrap_or(0)
                                };
                                let (q1, mid, q3) = (
                                    colony_x + (out_x - colony_x) / 4,
                                    colony_x + (out_x - colony_x) / 2,
                                    colony_x + 3 * (out_x - colony_x) / 4,
                                );
                                println!(
                                    "  frame {frame}: home after {} frames; trail B max over the column at the near quarter {}, middle {}, far quarter {}",
                                    frame - home_from.unwrap(),
                                    b(q1),
                                    b(mid),
                                    b(q3)
                                );
                            }
                        }
                        if head == Some(last_head) {
                            stuck += 1;
                        } else {
                            stuck = 0;
                        }
                        last_head = (hx, hy);
                        // Toward the target along the ground; up out of the ground
                        // when below it, and straight up when stuck a while.
                        let sx = (tx - hx).signum() as i8;
                        let sy: i8 = if hy > ground_y - 1 || stuck > 60 { -1 } else { 0 };
                        let input = SteerInput {
                            dir: Some((if stuck > 120 { 0 } else { sx }, sy)),
                            trail: phase == 1 && (arm == "line" || arm == "placebo"),
                            ..Default::default()
                        };
                        if let Some(s) = lab.world.steered.as_mut() {
                            s.input = if phase < 2 { Some(input) } else { None };
                        }
                    }
                    if frame > land + start + 30_000 && phase < 2 {
                        println!("RESULT mode=trail seed={seed} land={land} arm={arm} gap={gap} status=script_timeout phase={phase}");
                        return;
                    }
                }
                if phase == 2 {
                    if let Some(s) = lab.world.steered.as_mut() {
                        s.input = None;
                    }
                }
                if let Some(t0) = home_from {
                    if arm == "ramp" && frame < t0 + window && frame.is_multiple_of(12) {
                        let span = (food_x - colony_x).max(1) as f32;
                        for x in colony_x.min(food_x)..=colony_x.max(food_x) {
                            let t = ((x - colony_x) as f32 / span).clamp(0.0, 1.0);
                            let amount = (t * pixel_physics::sim::pheromone::DEPOSIT as f32)
                                as pixel_physics::sim::pheromone::Scent;
                            for y in ground_y - 4..=ground_y {
                                let here = lab.world.pheromone_at(pixel_physics::sim::pheromone::Channel::B, x, y);
                                if amount > here {
                                    lab.world.deposit_pheromone(
                                        pixel_physics::sim::pheromone::Channel::B,
                                        x,
                                        y,
                                        amount - here,
                                    );
                                }
                            }
                        }
                    }
                    if frame < t0 + window {
                        for id in lab.world.live_organism_ids() {
                            if id == qid {
                                continue;
                            }
                            let Some(s) = lab.world.organism(id) else { continue };
                            if s.species != ant || s.colony != colony {
                                continue;
                            }
                            if let Some(&(x, y)) = s.chain.first() {
                                if in_food(x, y, food_x, ground_y) {
                                    reached_food.insert(id);
                                    ticks_at_food += 1;
                                }
                            }
                        }
                    } else {
                        let (adults, ..) = census(&lab.world, colony, ant);
                        println!(
                            "RESULT mode=trail seed={seed} land={land} arm={arm} gap={gap} status=ok out_frames={} home_frames={} workers_at_food={} ant_ticks_at_food={ticks_at_food} adults_end={adults}",
                            home_from.map_or(0, |h| h - land - start),
                            home_at.zip(home_from).map_or(-1, |(a, h)| (a - h) as i64),
                            reached_food.len()
                        );
                        return;
                    }
                }
            }
            _ => {
                let (adults, eggs, larvae, pupae) = census(&lab.world, colony, ant);
                let others = adults.saturating_sub(usize::from(alive));
                peak = peak.max(others);
                if reached.is_none() && others >= goal {
                    reached = Some(frame);
                    println!(
                        "  frame {frame}: {others} adults besides her ({} frames after landing)",
                        frame - land
                    );
                }
                if frame.is_multiple_of(1000) {
                    let e = lab.world.organism(qid).map_or(0.0, |s| s.energy);
                    println!(
                        "  frame {frame}: adults {adults} eggs {eggs} larvae {larvae} pupae {pupae} queen_j {e:.0}"
                    );
                }
                let extinct = adults + eggs + larvae + pupae == 0;
                if extinct || f + 1 == frames || (mode == "beetle" && !alive) {
                    let mut by: std::collections::BTreeMap<String, u32> = Default::default();
                    for (_, c) in &colony_deaths {
                        *by.entry(c.split_once(':').map_or(c.as_str(), |(_, k)| k).to_string())
                            .or_default() += 1;
                    }
                    let st = &lab.world.creature_stats;
                    println!(
                        "RESULT mode={mode} seed={seed} land={land} reserve={reserve} gap={gap} reached_{goal}={} peak_workers={peak} queen_died={} adults_end={adults} brood_end={} eggs_laid={} births={} colony_deaths={:?} end_frame={frame}{}",
                        reached.map_or("never".to_string(), |r| (r - land).to_string()),
                        queen_died.as_ref().map_or("no".to_string(), |(fr, c)| format!("{}:{c}", fr - land)),
                        eggs + larvae + pupae,
                        st.eggs_laid,
                        st.births,
                        by,
                        if extinct { " extinct" } else { "" }
                    );
                    let _ = deaths_before;
                    return;
                }
            }
        }
    }
}
