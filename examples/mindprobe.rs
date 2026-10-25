//! **Does a trained colony answer the colony mind's verbs at all?** (colony-mind seat, step 0, 2026-10-25.)
//!
//! The colony mind seat (`Reports/teaching-the-colony-2026-10-25/seat-colony-mind.md`) has the player swell or hush
//! the colony's own signals. Its feasibility review found that the trained ant -- the lab's default brain since
//! 2026-10-24 -- reads none of the forage drive the design's anchor verb amplified, so the verbs that are left act on
//! what a trained ant *does* sense: the alarm plane, the food trail (scent B) and warmth. This asks, before any demo
//! is built, whether pressing each of those moves the colony visibly and by degree.
//!
//! **Paired by construction**: every arm runs the same scenario on the same seed and is bit-identical to `arm=none`
//! until the press begins (`warm=` frames after the colony lands), so a difference in the window is the verb's.
//!
//! ```text
//! mindprobe arm=none  seed=1                       # the twin
//! mindprobe arm=echo  hold=240 seed=1              # alarm re-laid over a disc while held
//! mindprobe arm=trail hold=240 gain=4 seed=1       # every laden ant's food trail x(1+gain) while held
//! mindprobe arm=heat  hold=240 amount=2 seed=1     # warmth added over a disc each frame while held
//! ```
//!
//! Arguments (an unknown one is ignored: read the echoed first line): `scenario=` (default `steady_income`: 52 trained
//! ants at column 256, food 30 columns east every 1,000 frames), `seed=`, `warm=` (frames after landing before the
//! press, default 20,000), `window=` (frames recorded after the press starts, default 3,000), `every=` (sample
//! interval, default 50), `hold=` (frames the press lasts), `r=` (disc radius, default 6), `tx= ty=` (the press's
//! target; default the colony door's column + `dx=` (default 15) on the surface), `gain=`, `amount=`.
//!
//! Prints one `S` line per sample: frame since press, ants alive, ants within 15 cells of the target, ants out of the
//! nest (head above the surface row, or more than 20 columns from the door), laden ants, mean head speed (cells per
//! frame over the sample), the mind's own counters since the press (`delivered`, `fed_adult`, `dug`), banks, adults,
//! and `trip`, food brought home from trips. Compare arms with `scripts/mindprobe_cmp.py`.
//!
//! **Read `trip`, never `delivered`, for food home.** `delivered` books crumbs lifted inside the nest and put straight
//! back; step 0 first read the swell as "+43% food home" on it, and it was noise on `trip`
//! (`Reports/teaching-the-colony-2026-10-25/colony-mind-verdict.md`). The stake bed `demo_mind` lives beside that
//! report: `PIXEL_PHYSICS_LAB_SCENARIOS=Reports/teaching-the-colony-2026-10-25/colony-mind`.
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::Lab;
use pixel_physics::sim::pheromone::{self, Channel};
use std::collections::HashMap;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

/// Every adult ant's bank as a share of a full tank, with its head.
fn banks(lab: &Lab) -> Vec<(i32, i32, f32)> {
    let w = &lab.world;
    let Some(sp) = w.species.id_of("ant") else { return Vec::new() };
    let full = w.species.get(sp).creature.as_ref().map_or(1.0, |d| d.start_energy);
    w.live_organism_ids()
        .into_iter()
        .filter_map(|id| {
            let s = w.organism(id).filter(|s| s.species == sp && s.brood.is_none())?;
            let &(x, y) = s.chain.first()?;
            Some((x, y, s.energy / full))
        })
        .collect()
}

fn ants(lab: &Lab) -> Vec<(u64, i32, i32, bool)> {
    let w = &lab.world;
    let sp = w.species.id_of("ant");
    w.live_organism_ids()
        .into_iter()
        .filter_map(|id| {
            let s = w.organism(id).filter(|s| Some(s.species) == sp)?;
            let &(x, y) = s.chain.first()?;
            Some((id as u64, x, y, s.crop.is_some_and(|c| c.cells > 0)))
        })
        .collect()
}

fn main() {
    let name: String = arg("scenario").unwrap_or_else(|| "steady_income".to_string());
    let arm: String = arg("arm").unwrap_or_else(|| "none".to_string());
    let warm: u64 = arg("warm").unwrap_or(20_000);
    let window: u64 = arg("window").unwrap_or(3_000);
    let every: u64 = arg("every").unwrap_or(50);
    let hold: u64 = arg("hold").unwrap_or(240);
    let r: i32 = arg("r").unwrap_or(6);
    let dx: i32 = arg("dx").unwrap_or(15);
    let gain: f32 = arg("gain").unwrap_or(4.0);
    let amount: f32 = arg("amount").unwrap_or(2.0);
    let mut sc = Scenario::load(&name).unwrap_or_else(|e| panic!("scenario {name}: {e}"));
    if let Some(sd) = arg::<u64>("seed") {
        sc.bed.seed = sd;
    }
    let seed = sc.bed.seed;
    let mut lab = Lab::new(sc.bed.clone());
    lab.load_scenario(sc);
    lab.set_new_ant(true);
    // Land the colony, then warm up.
    let mut landed = None;
    while landed.is_none_or(|l| lab.world.frame < l + warm) {
        lab.tick_for_harness();
        if landed.is_none() && !ants(&lab).is_empty() {
            landed = Some(lab.world.frame);
        }
        assert!(lab.world.frame < 200_000, "the colony never landed");
    }
    let door = lab.world.nest_sites.first().map_or(256, |s| s.x);
    let surf = |lab: &Lab, x: i32| lab.world.room_surface_at(x).unwrap_or(160);
    let tx: i32 = arg("tx").unwrap_or(door + dx);
    let ty: i32 = arg("ty").unwrap_or_else(|| surf(&lab, tx) - 1);
    println!(
        "mindprobe scenario={name} seed={seed} arm={arm} warm={warm} window={window} every={every} hold={hold} r={r} tx={tx} ty={ty} gain={gain} amount={amount} door={door} landed={}",
        landed.unwrap()
    );
    let c0 = lab.world.minds.as_ref().map(|m| m.counts).unwrap_or_default();
    let trip0 = lab.world.creature_stats.trip_deliveries;
    let mut last: HashMap<u64, (i32, i32)> = ants(&lab).into_iter().map(|(id, x, y, _)| (id, (x, y))).collect();
    let mut alarm_sum = 0.0f64;
    for t in 0..window {
        if t < hold {
            match arm.as_str() {
                "none" => {}
                "echo" => {
                    for y in ty - r..=ty + r {
                        for x in tx - r..=tx + r {
                            if (x - tx).pow(2) + (y - ty).pow(2) <= r * r {
                                let have = lab.world.pheromone_at(Channel::Alarm, x, y);
                                let top = pheromone::ALARM_DEPOSIT;
                                if have < top {
                                    lab.world.deposit_pheromone(Channel::Alarm, x, y, top - have);
                                }
                            }
                        }
                    }
                }
                "trail" => {
                    let extra = (pheromone::DEPOSIT as f32 * gain) as u16;
                    for (_, x, y, laden) in ants(&lab) {
                        if laden {
                            lab.world.deposit_pheromone(Channel::B, x, y, extra);
                        }
                    }
                }
                "heat" => lab.world.add_heat(tx, ty, r, amount),
                // Only the laden ants within `r` of the target lay louder: the verb as a player would press it on
                // the foragers arriving at the door, against `trail`'s every-laden-ant upper bound.
                "trailnear" => {
                    let extra = (pheromone::DEPOSIT as f32 * gain) as u16;
                    for (_, x, y, laden) in ants(&lab) {
                        if laden && (x - tx).pow(2) + (y - ty).pow(2) <= r * r {
                            lab.world.deposit_pheromone(Channel::B, x, y, extra);
                        }
                    }
                }
                // **Hush the trail**: the food trail under every laden ant within `r` of the target, and the eight
                // cells round its head, scrubbed to zero each tick -- swell's inverse.
                "hush" => {
                    for (_, x, y, laden) in ants(&lab) {
                        if laden && (x - tx).pow(2) + (y - ty).pow(2) <= r * r {
                            for (ox, oy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                                lab.world.scale_pheromone(Channel::B, x + ox, y + oy, 0.0);
                            }
                        }
                    }
                }
                // **The stomach closed over the nest in famine only** (the `demo_mind` bed: 20,000 frames of income
                // from frame 6,000, then 15,000 without, repeating): the stake step's player policy.
                "stomachfam" => {
                    let f = lab.world.frame;
                    let famine = f >= 6_000 && (f - 6_000) % 35_000 >= 20_000;
                    if let Some(m) = lab.world.minds.as_mut() {
                        m.closed_stomach = if famine { vec![(tx, ty, r)] } else { Vec::new() };
                    }
                }
                // The shared stomach closed over a disc while held (`Minds::closed_stomach`), opened again after.
                "stomach" => {
                    if let Some(m) = lab.world.minds.as_mut() {
                        m.closed_stomach = vec![(tx, ty, r)];
                    }
                }
                // **The chaos control**: one unit of food trail at the door, once. It means nothing to any ant, but
                // it makes the run diverge from its twin, so its differences are the spread a perturbation alone
                // buys -- the noise floor every other arm is read against.
                "null" => {
                    if t == 0 {
                        lab.world.deposit_pheromone(Channel::B, door, surf(&lab, door) - 1, 1);
                    }
                }
                other => panic!("arm={other}: none|null|echo|trail|trailnear|hush|heat|stomach|stomachfam"),
            }
        }
        if t == hold {
            if let Some(m) = lab.world.minds.as_mut() {
                m.closed_stomach.clear();
            }
        }
        lab.tick_for_harness();
        if (t + 1) % every == 0 {
            let now = ants(&lab);
            let (mut near, mut out, mut laden, mut moved, mut n_moved) = (0, 0, 0, 0.0f64, 0);
            for &(id, x, y, l) in &now {
                near += usize::from((x - tx).pow(2) + (y - ty).pow(2) <= 15 * 15);
                out += usize::from(y < surf(&lab, x) || (x - door).abs() > 20);
                laden += usize::from(l);
                if let Some(&(px, py)) = last.get(&id) {
                    moved += (((x - px).pow(2) + (y - py).pow(2)) as f64).sqrt();
                    n_moved += 1;
                }
            }
            last = now.iter().map(|&(id, x, y, _)| (id, (x, y))).collect();
            alarm_sum = 0.0;
            for y in ty - 15..=ty + 15 {
                for x in tx - 15..=tx + 15 {
                    alarm_sum += lab.world.pheromone_at(Channel::Alarm, x, y) as f64 / pheromone::ALARM_DEPOSIT as f64;
                }
            }
            let b = banks(&lab);
            let lean = b.iter().filter(|&&(_, _, e)| e < 1.0).count();
            let zone: Vec<f32> = b.iter().filter(|&&(x, y, _)| (x - tx).pow(2) + (y - ty).pow(2) <= 15 * 15).map(|&(_, _, e)| e).collect();
            let zmean = zone.iter().sum::<f32>() / zone.len().max(1) as f32;
            let zsd = (zone.iter().map(|e| (e - zmean).powi(2)).sum::<f32>() / zone.len().max(1) as f32).sqrt();
            let c = lab.world.minds.as_ref().map(|m| m.counts).unwrap_or_default();
            // **`trip` is the honest food-home count** (`CreatureStats::trip_deliveries`): a crop cell put down at
            // the nest that came from a trip. `delivered` (`MindCounts::delivered`) also books food picked up inside
            // the nest and put straight back -- the plan review found the twin "delivering" 2,859-6,990 cells in
            // 30,000 frames against 1,200 of income -- so it measures handling, not food home.
            let trip = lab.world.creature_stats.trip_deliveries - trip0;
            let bank = format!(" lean={lean} zone_n={} zone_mean={zmean:.3} zone_sd={zsd:.3} adults={} trip={trip}", zone.len(), b.len());
            println!(
                "S t={} alive={} near={near} out={out} laden={laden} speed={:.4} delivered={} fed_adult={} dug={} alarm15={alarm_sum:.1}{bank}",
                t + 1,
                now.len(),
                moved / (n_moved.max(1) as f64 * every as f64),
                c.delivered - c0.delivered,
                c.fed_adult - c0.fed_adult,
                c.dug - c0.dug,
            );
        }
    }
    let _ = alarm_sum;
}
