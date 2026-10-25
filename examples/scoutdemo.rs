//! **The lone scout demo, played by a script** (`sim::tandem`; plan
//! `Reports/teaching-the-colony-2026-10-25/scout-demo/plan-v2.md` §4).
//!
//! Loads `demo_scout.ron` with the trained ant, warms the box to the hand-over
//! frame, picks the player's body (a modal-pace adult near the nest) and plays
//! her by a script: walk home, tap the nearest sister, lead her to the far
//! heap, **wait whenever her pip would turn amber** (a tell, a stop, or a
//! search), then walk home and do it again. Prints the tandem counters every
//! `every=` frames so a run can be judged by number as well as by picture.
//!
//! ```text
//! cargo run --release --example scoutdemo -- arm=A seed=1 frames=36000
//! cargo run --release --example scoutdemo -- arm=A seed=1 rude=1     # a player who never waits
//! ```
//!
//! `rude=1` is the positive control for the pauses: a leader who walks on
//! through every stop must break runs that a waiting leader keeps.

use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::Lab;
use pixel_physics::sim::cell::OrganismId;
use pixel_physics::sim::steer;
use pixel_physics::sim::tandem::{self, Arm, Patch, Phase, TapResult, Tandems};
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

const NEST_X: i32 = 128;
const HEAP_X: i32 = 248;
/// Where the scripted player taps: by the near trickle (column 153), where foragers gather and the surface east is
/// open. A tap inside a tunnel jammed leader and pupil in the first run, and the spoil and food piled over the
/// mound boxed a scripted walker in for 9,000 frames in the second.
const TAP_X: i32 = 160;

fn head(world: &World, id: OrganismId) -> Option<(i32, i32)> {
    world.organism(id).and_then(|s| s.chain.first().copied())
}

fn ants(world: &World) -> Vec<OrganismId> {
    let Some(ant) = world.species.id_of("ant") else { return Vec::new() };
    world.live_organism_ids().into_iter().filter(|&i| world.organism(i).is_some_and(|s| s.species == ant)).collect()
}

/// The player's body: an adult on the surface within 25 columns of the tap
/// spot whose decision interval is the colony's commonest (review 8),
/// nearest the spot first.
fn pick_player(world: &World) -> Option<OrganismId> {
    let ant = world.species.id_of("ant")?;
    let def = world.species.creature_shared(ant)?;
    let ids = ants(world);
    let interval = |id| pixel_physics::sim::creature::organism_tick_interval(world, id, &def);
    let mut counts = std::collections::BTreeMap::new();
    for &id in &ids {
        *counts.entry(interval(id)).or_insert(0) += 1;
    }
    let modal = counts.into_iter().max_by_key(|&(_, n)| n)?.0;
    ids.into_iter()
        .filter(|&id| interval(id) == modal)
        .filter_map(|id| head(world, id).filter(|h| world.room_surface_at(h.0).is_some_and(|top| h.1 <= top + 1)).map(|h| ((h.0 - TAP_X).abs(), id)))
        .filter(|&(d, _)| d <= 25)
        .min()
        .map(|(_, id)| id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Plan {
    Home,
    Tapping,
    Leading,
}

fn main() {
    let arm = match arg::<String>("arm").unwrap_or_else(|| "A".into()).as_str() {
        "B" => Arm::Column,
        "C" => Arm::Escort,
        _ => Arm::Scout,
    };
    let seed: u64 = arg("seed").unwrap_or(1);
    let start: u64 = arg("start").unwrap_or(18_000);
    let frames: u64 = arg("frames").unwrap_or(36_000);
    let every: u64 = arg("every").unwrap_or(3_000);
    let rude = arg::<u8>("rude").unwrap_or(0) == 1;
    let c: Option<f32> = arg("c");
    let trace = arg::<u8>("trace").unwrap_or(0) == 1;
    println!("scoutdemo arm={} seed={seed} start={start} frames={frames} every={every} rude={rude} c={c:?}", arm.letter());

    let mut sc = Scenario::load("demo_scout").expect("demo_scout.ron ships");
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    lab.load_scenario(sc);
    lab.set_new_ant(true);
    while lab.world.frame < start {
        lab.tick_for_harness();
    }
    let alive0 = ants(&lab.world).len();
    let mut t = Tandems::new(arm, Some(Patch { x: HEAP_X, r: 10 }));
    if let Some(c) = c {
        t.knobs.cascade_c = c;
    }
    lab.world.tandem = Some(Box::new(t));
    let Some(player) = pick_player(&lab.world) else {
        println!("no player body: {alive0} ants alive at {start}");
        return;
    };
    // The headless player is led through the same slot her pupils are (`steer::force`), not `steer::mark`:
    // marking would put her colony on the queen rule, which freezes its breeding (until the queen seat's
    // `SteerRole::Worker` lands), and that would confound "alive at the end".
    println!("hand-over at {}: {alive0} ants alive, player #{player}", lab.world.frame);

    let mut plan = Plan::Home;
    let mut tap_tries = 0u32;
    let mut leads_done = 0u32;
    let mut gaps = [0u64; 8];
    let mut best_dist = i32::MAX;
    let mut last_goal = 0;
    let mut moved_at = 0u64;
    let end = start + frames;
    while lab.world.frame < end {
        let w = &mut lab.world;
        let Some(at) = head(w, player) else {
            println!("frame {}: the player died", w.frame);
            break;
        };
        let pairs: Vec<_> = w.tandem.as_ref().unwrap().pairs.iter().filter(|p| p.leader == player).cloned().collect();
        for p in &pairs {
            if let Some(f) = head(w, p.follower) {
                let g = (f.0 - at.0).abs().max((f.1 - at.1).abs()) as usize;
                gaps[g.min(7)] += 1;
            }
        }
        let dir: Option<(i8, i8)> = match plan {
            Plan::Home => {
                if (at.0 - TAP_X).abs() <= 3 {
                    plan = Plan::Tapping;
                    tap_tries = 0;
                    None
                } else {
                    Some(((TAP_X - at.0).signum() as i8, 0))
                }
            }
            Plan::Tapping if tandem::candidates(w, player, 3).is_empty() => {
                // Nobody in reach: walk toward the nest, where sisters are, a step at a time.
                if w.frame.is_multiple_of(30) {
                    tap_tries += 1;
                }
                if tap_tries > 40 {
                    plan = Plan::Leading;
                }
                (at.0 > NEST_X + 6).then_some((-1, 0))
            }
            Plan::Tapping => {
                if w.frame.is_multiple_of(30) {
                    tap_tries += 1;
                    match tandem::tap(w, player) {
                        TapResult::Accepted(_) => plan = Plan::Leading,
                        _ if tap_tries > 20 => plan = Plan::Leading,
                        _ => {}
                    }
                }
                None
            }
            Plan::Leading => {
                if pairs.is_empty() {
                    leads_done += 1;
                    plan = Plan::Home;
                    None
                } else {
                    // A patient leader stands while any pupil lags, stops or searches; a rude one never does.
                    let wait = !rude
                        && pairs.iter().any(|p| {
                            let g = head(w, p.follower).map_or(99, |f| (f.0 - at.0).abs().max((f.1 - at.1).abs()));
                            !matches!(p.phase, Phase::Following) || g >= 3
                        });
                    if wait {
                        None
                    } else if (at.0 - HEAP_X).abs() <= 4 {
                        None
                    } else {
                        Some(((HEAP_X - at.0).signum() as i8, 0))
                    }
                }
            }
        };
        // The headless player eats at the heap (the E key: pick up, or eat from the crop), as a person would.
        let hungry = w.organism(player).is_some_and(|s| s.energy < 1_500.0);
        let food_near = w.materials.id_of("provisions").is_some_and(|f| (-3..=3).any(|dx| (-3..=3).any(|dy| w.get(at.0 + dx, at.1 + dy).material == f)));
        let eat = hungry && food_near;
        if trace && w.frame == start + 600 {
            for y in at.1 - 6..=at.1 + 3 {
                let row: String = (at.0 - 10..=at.0 + 10)
                    .map(|x| {
                        let c = w.get(x, y);
                        if (x, y) == at {
                            '@'
                        } else if c.is_empty() {
                            '.'
                        } else {
                            w.materials.get(c.material).name.chars().next().unwrap_or('?')
                        }
                    })
                    .collect();
                println!("    {y}: {row}");
            }
            let st = w.organism(player).unwrap();
            println!("    chain {:?} flight {:?} forced {:?}", st.chain, st.flight.is_some(), w.mind_force.get(&player).map(|f| f.pick));
            let ant = w.species.id_of("ant").unwrap();
            let def = w.species.creature_shared(ant).unwrap();
            let mut opts = Vec::new();
            pixel_physics::sim::creature::options::build_with(w, player, &def, true, &mut opts);
            println!("    opts {:?}", opts.iter().map(|o| (o.verb, o.target)).collect::<Vec<_>>());
        }
        if trace && w.frame.is_multiple_of(150) {
            let e = w.organism(player).map_or(0.0, |s| s.energy);
            let fs: Vec<String> = pairs.iter().map(|p| format!("{:?}@{:?} {:?} l{}/{}", p.follower, head(w, p.follower), p.phase, p.learned, p.met)).collect();
            println!("  t{} player {:?} e{:.0} {:?} dir {:?} | {}", w.frame, at, e, plan, dir, fs.join(" ; "));
        }
        // A scripted player who is stuck (a tunnel wall, a face) tries upward headings in turn, as a person
        // pressing Up would; a real player reads the screen instead.
        // Progress is getting nearer the goal column, not merely moving: a chain that turns round in place moves
        // its head every decision and gets nowhere.
        let goal = match plan {
            Plan::Home => TAP_X,
            Plan::Tapping => NEST_X,
            Plan::Leading => HEAP_X,
        };
        let dist = (goal - at.0).abs();
        if dir.is_none() || dist < best_dist || goal != last_goal {
            best_dist = dist;
            last_goal = goal;
            moved_at = w.frame;
        }
        let dir = match dir {
            Some(d) if w.frame.saturating_sub(moved_at) > 40 => {
                let alts = [(d.0, -1), (0, -1), (-d.0, -1), (-d.0, 0), (d.0, 1)];
                Some(alts[((w.frame - moved_at) / 40 - 1) as usize % alts.len()])
            }
            other => other,
        };
        // Boxed in for a while (provisions and spoil pile over the nest mound), she digs her way out, as a player
        // pressing G would.
        let boxed = dir.is_some() && w.frame.saturating_sub(moved_at) > 200;
        let verb = if eat {
            Some(pixel_physics::sim::creature::options::Verb::PickUp)
        } else if boxed {
            Some(pixel_physics::sim::creature::options::Verb::Dig)
        } else {
            None
        };
        steer::force(w, player, steer::ForcedPick { dir, verb, once: false });
        lab.tick_for_harness();
        if (lab.world.frame - start).is_multiple_of(every) {
            let t = lab.world.tandem.as_ref().unwrap();
            let knows = t.lessons.len();
            println!(
                "f{} alive {} plan {:?} | {:?} | know {} income/min {} | gaps0-7 {:?}",
                lab.world.frame,
                ants(&lab.world).len(),
                plan,
                t.counts,
                knows,
                t.income_over(lab.world.frame, 3_600),
                gaps
            );
        }
    }
    let t = lab.world.tandem.as_ref().unwrap();
    println!("END arm={} seed={seed} rude={rude}: leads {leads_done} alive {}->{} {:?}", arm.letter(), alive0, ants(&lab.world).len(), t.counts);
}
