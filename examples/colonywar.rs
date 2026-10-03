//! **Two or more colonies in one lab box: who fights whom, where, and who
//! wins the food.** Owner's ask, 2026-10-03: *"explore how two or multiple
//! ant colonies fight each other, visual, user-friendly ways to track the
//! battle for the user, resource competition between the colonies."*
//!
//! Drives the real game (`Lab::tick_for_harness`, so the scenario timeline,
//! the mister and `regroup_by_scent` all run as they do on screen) over a
//! scenario named on the command line, and prints three kinds of line:
//!
//! - `ROW frame colony alive brood bank_j intake_j raided_j lost_j starved
//!   killed old by_rival by_own terr nests` -- one per colony every
//!   `every=` frames. `intake_j` is `HarvestedPlant + HarvestedCorpse` off
//!   that colony's `ColonyBooks`; `raided_j`/`lost_j` are living flesh eaten
//!   off / by other colonies (face value). `starved`/`killed`/`old` are
//!   `group_deaths` (animal-only, cumulative). `by_rival`/`by_own` split
//!   `killed` by whether the killer carried another colony's label.
//!   `terr` is columns this colony held over the last window (>= 70% of the
//!   ant presence sampled in that column, and at least `TERR_MIN` samples).
//! - `KILL frame victim attacker x y place depth brood e vsp asp gang friends` -- every
//!   killing off `World::kills_log` with a tracked species on either side,
//!   then `gang` and `friends`: the killer's and the victim's colony-mates
//!   within `gang=` cells (default 2) at that frame, located by its `Grave`. `place` is
//!   `victim_home` / `killer_home` / `other_nest` / `open` by the nearest
//!   nest within `NEST_REACH` columns and who holds it; `depth` is
//!   `under` below the founding ground line, else `surface`; `brood` is
//!   whether the victim was an egg/larva/pupa when last seen.
//! - `SUMMARY` at the end.
//!
//! With `shots=F1,F2,..` it also writes, per shot, the plain box
//! (`DIR/plain-F.png`) and a **mock** of a battle view drawn over it
//! (`DIR/mock-F.png`): territory tint in the air above each held column,
//! kill markers for the last `MARK_WINDOW` frames, a per-colony strength
//! strip under the box and a battle log beside it. The mock is drawn here,
//! in the harness, so nothing in `src/lab/ui.rs` moves to try it.
//!
//! ```text
//! PIXEL_PHYSICS_LAB_SCENARIOS=DIR cargo run --release --example colonywar -- \
//!     scenario=war_two seed=1 frames=120000 every=1000 shots=30000,60000 out=DIR
//! ```

use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::Lab;
use pixel_physics::render;
use pixel_physics::sim::cell::OrganismId;
use pixel_physics::sim::organism::DeathCause;
use pixel_physics::sim::world::World;
use std::collections::{BTreeMap, HashMap};

/// A kill is "at" a nest when it lands within this many columns of the
/// nest's centre. The founding cut and the first rooms sit well inside it.
const NEST_REACH: i32 = 20;
/// A killer's gang is its colony's adults whose head is within this many
/// cells of the victim's grave (Chebyshev). An ant's body is two cells and a
/// bite reaches the 8-neighbourhood, so 2 holds the heads of everyone that
/// could have been biting. `gang=` overrides it.
fn gang_reach() -> i32 {
    static G: std::sync::OnceLock<i32> = std::sync::OnceLock::new();
    *G.get_or_init(|| arg("gang").unwrap_or(2))
}
/// Presence is sampled every this many frames.
const PRESENCE_EVERY: u64 = 25;
/// A column needs this many presence samples in a window to be held at all.
const TERR_MIN: u32 = 4;
/// Kill markers on the mock show this many frames back.
const MARK_WINDOW: u64 = 6000;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

/// The pseudo-colony a non-colony party (any tracked species after the
/// first -- a beetle on `species=ant,beetle`) is booked under on the mock, so
/// a predator's kills draw and log as one more side rather than a second
/// visual system.
const PRED: u32 = u32::MAX;

#[derive(Clone, Copy)]
struct Kill {
    frame: u64,
    victim: u32,
    attacker: u32,
    x: i32,
    y: i32,
    place: &'static str,
    under: bool,
    brood: bool,
}

#[derive(Default, Clone)]
struct Snap {
    alive: BTreeMap<u32, u32>,
    intake: BTreeMap<u32, f64>,
}

/// The species whose colonies get a `ROW` (`species=ant,longant`, default
/// `ant`). A `KILL` line is written when **either** party is one of them, and
/// names both species, so an ant-versus-beetle bed reads off the same log.
fn tracked() -> &'static [String] {
    static T: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    T.get_or_init(|| arg::<String>("species").unwrap_or_else(|| "ant".into()).split(',').map(str::to_string).collect())
}

fn is_ant(world: &World, sid: pixel_physics::sim::organism::SpeciesId) -> bool {
    tracked().iter().any(|n| *n == world.species.get(sid).name)
}

/// The first tracked species: the one whose colonies get `ROW` lines and a
/// line on the scoreboard. Every other tracked species is a predator side.
fn is_colony_species(world: &World, sid: pixel_physics::sim::organism::SpeciesId) -> bool {
    tracked()[0] == world.species.get(sid).name
}

/// Which colony holds each nest site: the colony with the most living
/// adults within `NEST_REACH` columns of it. `0` when nobody is there.
fn nest_owners(world: &World) -> Vec<(i32, u32)> {
    let mut near: Vec<BTreeMap<u32, u32>> = vec![BTreeMap::new(); world.nest_sites.len()];
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        if !is_colony_species(world, st.species) || st.brood.is_some() || st.colony == 0 {
            continue;
        }
        let Some(&(x, _)) = st.chain.first() else { continue };
        for (i, n) in world.nest_sites.iter().enumerate() {
            if (n.x - x).abs() <= NEST_REACH {
                *near[i].entry(st.colony).or_default() += 1;
            }
        }
    }
    world
        .nest_sites
        .iter()
        .zip(near)
        .map(|(n, m)| (n.x, m.into_iter().max_by_key(|&(c, k)| (k, std::cmp::Reverse(c))).map(|(c, _)| c).unwrap_or(0)))
        .collect()
}

/// Living adults of `colony` (of `species`) whose head is within
/// `gang_reach()` cells of `at` -- the gang on a kill, and the friends the
/// victim had beside it. Read the frame the kill is logged, so the killer is
/// still standing where it bit.
fn near(world: &World, colony: u32, species: pixel_physics::sim::organism::SpeciesId, at: (i32, i32)) -> u32 {
    let mut n = 0;
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        if st.species != species || st.colony != colony || st.brood.is_some() {
            continue;
        }
        let Some(&(x, y)) = st.chain.first() else { continue };
        if (x - at.0).abs() <= gang_reach() && (y - at.1).abs() <= gang_reach() {
            n += 1;
        }
    }
    n
}

fn plant_near(world: &World, x: i32, y: i32) -> bool {
    for dy in -3..=3 {
        for dx in -3..=3 {
            let Some(id) = pixel_physics::sim::specimen::organism_at(world, x + dx, y + dy) else { continue };
            if let Some(st) = world.organism(id) {
                if world.species.get(st.species).creature.is_none() {
                    return true;
                }
            }
        }
    }
    false
}

fn main() {
    let name: String = arg("scenario").unwrap_or_else(|| "war_two".into());
    let seed: Option<u64> = arg("seed");
    let frames: u64 = arg("frames").unwrap_or(120_000);
    let every: u64 = arg("every").unwrap_or(1000);
    let out: String = arg("out").unwrap_or_else(|| ".".into());
    let shots: Vec<u64> = arg::<String>("shots")
        .map(|s| s.split(',').filter_map(|t| t.parse().ok()).collect())
        .unwrap_or_default();
    let mut sc = Scenario::load(&name).unwrap_or_else(|e| panic!("scenario {name}: {e}"));
    if let Some(s) = seed {
        sc.bed.seed = s;
    }
    println!(
        "colonywar: scenario={name} species={:?} seed={} frames={frames} every={every} shots={shots:?} nest_reach={NEST_REACH} kin_gate={:?} contest={:?}",
        tracked(),
        sc.bed.seed,
        std::env::var("PIXEL_PHYSICS_NEST_KIN_GATE").ok(),
        std::env::var("PIXEL_PHYSICS_CONTEST").ok()
    );
    let ground_y = sc.bed.ground_y;
    let mut lab = Lab::new(sc.bed.clone());
    lab.show_help = false;
    if lab.stats.showing() {
        lab.stats.toggle();
    }
    println!("  {}", lab.load_scenario(sc));
    let (w, h) = (lab.spec.width, lab.spec.height);

    let mut seen_kills = 0usize;
    let mut kills: Vec<Kill> = Vec::new();
    let mut unmatched = 0u64;
    let mut brood_of: HashMap<OrganismId, bool> = HashMap::new();
    // presence[colony][x] over the current window.
    let mut presence: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut held: BTreeMap<u32, Vec<bool>> = BTreeMap::new();
    let mut owners: Vec<(i32, u32)> = Vec::new();
    let mut snaps: Vec<Snap> = Vec::new();
    let mut peak: BTreeMap<u32, u32> = BTreeMap::new();

    for f in 0..=frames {
        if f > 0 {
            lab.tick_for_harness();
        }
        let world = &lab.world;
        // ---- kills, every frame, located by their grave
        if world.kills_log.len() > seen_kills {
            let new: Vec<_> = world.kills_log[seen_kills..].to_vec();
            seen_kills = world.kills_log.len();
            let mut used: Vec<OrganismId> = Vec::new();
            for k in new {
                if !is_ant(world, k.victim_species) && !is_ant(world, k.attacker_species) {
                    continue;
                }
                let grave = world.graveyard.recent().find(|g| {
                    g.died_frame == k.frame
                        && g.colony == k.victim_colony
                        && g.creature
                        && g.cause == DeathCause::Killed
                        && !used.contains(&g.id)
                });
                let Some(g) = grave else {
                    unmatched += 1;
                    continue;
                };
                used.push(g.id);
                let (x, y) = g.at;
                let nearest = owners.iter().filter(|(nx, _)| (nx - x).abs() <= NEST_REACH).min_by_key(|(nx, _)| (nx - x).abs());
                let place = match nearest {
                    Some(&(_, c)) if c == k.victim_colony => "victim_home",
                    Some(&(_, c)) if c == k.attacker_colony => "killer_home",
                    Some(_) => "other_nest",
                    None if plant_near(world, x, y) => "open_plants",
                    None => "open",
                };
                let kill = Kill {
                    frame: k.frame,
                    victim: if is_colony_species(world, k.victim_species) { k.victim_colony } else { PRED },
                    attacker: if is_colony_species(world, k.attacker_species) { k.attacker_colony } else { PRED },
                    x,
                    y,
                    place,
                    under: y > ground_y + 1,
                    brood: brood_of.get(&g.id).copied().unwrap_or(false),
                };
                println!(
                    "KILL {} {} {} {} {} {} {} {} {:.0} {} {} {} {}",
                    kill.frame,
                    kill.victim,
                    kill.attacker,
                    x,
                    y,
                    place,
                    if kill.under { "under" } else { "surface" },
                    u8::from(kill.brood),
                    k.victim_energy,
                    world.species.get(k.victim_species).name,
                    world.species.get(k.attacker_species).name,
                    near(world, k.attacker_colony, k.attacker_species, (x, y)),
                    near(world, k.victim_colony, k.victim_species, (x, y))
                );
                kills.push(kill);
            }
        }
        // ---- presence + who-is-brood, every PRESENCE_EVERY
        if f % PRESENCE_EVERY == 0 {
            brood_of.clear();
            for id in world.live_organism_ids() {
                let Some(st) = world.organism(id) else { continue };
                if !is_colony_species(world, st.species) || st.colony == 0 {
                    continue;
                }
                brood_of.insert(id, st.brood.is_some());
                if st.brood.is_some() {
                    continue;
                }
                let Some(&(x, _)) = st.chain.first() else { continue };
                if (0..w).contains(&x) {
                    presence.entry(st.colony).or_insert_with(|| vec![0; w as usize])[x as usize] += 1;
                }
            }
        }
        // ---- the per-colony row
        if f % every == 0 {
            owners = nest_owners(world);
            // territory over the window just closed
            held.clear();
            for x in 0..w as usize {
                let tot: u32 = presence.values().map(|v| v[x]).sum();
                if tot < TERR_MIN {
                    continue;
                }
                for (&c, v) in &presence {
                    if v[x] * 10 >= tot * 7 {
                        held.entry(c).or_insert_with(|| vec![false; w as usize])[x] = true;
                    }
                }
            }
            let mut alive: BTreeMap<u32, (u32, u32, f64)> = BTreeMap::new();
            for id in world.live_organism_ids() {
                let Some(st) = world.organism(id) else { continue };
                if !is_colony_species(world, st.species) || st.colony == 0 {
                    continue;
                }
                let e = alive.entry(st.colony).or_default();
                if st.brood.is_some() {
                    e.1 += 1;
                } else {
                    e.0 += 1;
                    e.2 += st.energy as f64;
                }
            }
            let mut cols: Vec<u32> = alive.keys().copied().collect();
            for g in &world.group_deaths {
                if is_colony_species(world, g.species) && g.colony != 0 && !cols.contains(&g.colony) {
                    cols.push(g.colony);
                }
            }
            cols.sort();
            let mut snap = Snap::default();
            for c in cols {
                let (a, b, bank) = alive.get(&c).copied().unwrap_or_default();
                let books = world.colony_books(c);
                let intake = books.get(pixel_physics::sim::world::Account::HarvestedPlant)
                    + books.get(pixel_physics::sim::world::Account::HarvestedCorpse);
                let gd = world.group_deaths.iter().find(|g| g.colony == c && is_colony_species(world, g.species));
                let (st, kl, old) = gd
                    .map(|g| (g.by_cause[DeathCause::Starved.index()], g.by_cause[DeathCause::Killed.index()], g.by_cause[DeathCause::OldAge.index()]))
                    .unwrap_or_default();
                let by_rival = kills.iter().filter(|k| k.victim == c && k.attacker != c).count();
                let by_own = kills.iter().filter(|k| k.victim == c && k.attacker == c).count();
                let terr = held.get(&c).map(|v| v.iter().filter(|b| **b).count()).unwrap_or(0);
                let nests = owners.iter().filter(|(_, o)| *o == c).count();
                println!(
                    "ROW {f} {c} {a} {b} {bank:.0} {intake:.0} {:.0} {:.0} {st} {kl} {old} {by_rival} {by_own} {terr} {nests}",
                    books.raided, books.raided_by_others
                );
                snap.alive.insert(c, a);
                snap.intake.insert(c, intake);
                let p = peak.entry(c).or_default();
                *p = (*p).max(a);
            }
            snaps.push(snap);
            presence.clear();
        }
        if shots.contains(&f) {
            shoot(&mut lab, &out, f, &kills, &held, &owners, &snaps, ground_y, (w, h));
        }
    }

    // ---- summary
    let world = &lab.world;
    let last = snaps.last().cloned().unwrap_or_default();
    println!("SUMMARY frames={frames} kills_total={} unmatched={unmatched}", kills.len());
    for (&c, &pk) in &peak {
        let a = last.alive.get(&c).copied().unwrap_or(0);
        let lost_rival = kills.iter().filter(|k| k.victim == c && k.attacker != c).count();
        let won = kills.iter().filter(|k| k.attacker == c && k.victim != c).count();
        let own = kills.iter().filter(|k| k.victim == c && k.attacker == c).count();
        println!(
            "SUMMARY-colony {c} peak={pk} end={a} kills_of_rivals={won} lost_to_rivals={lost_rival} own_kills={own} intake={:.0}",
            last.intake.get(&c).copied().unwrap_or(0.0)
        );
    }
    let mut places: BTreeMap<&str, (u32, u32, u32)> = BTreeMap::new();
    for k in &kills {
        let e = places.entry(k.place).or_default();
        if k.victim == k.attacker {
            e.2 += 1;
        } else if k.brood {
            e.1 += 1;
        } else {
            e.0 += 1;
        }
    }
    for (p, (adult, brood, own)) in places {
        println!("SUMMARY-place {p} rival_adults={adult} rival_brood={brood} own={own}");
    }
    let under = kills.iter().filter(|k| k.under && k.victim != k.attacker).count();
    println!("SUMMARY-depth rival_kills_underground={under} of {}", kills.iter().filter(|k| k.victim != k.attacker).count());
    let _ = world;
}

fn colour_of(c: u32) -> [u8; 4] {
    if c == PRED {
        return [235, 60, 40, 255];
    }
    let p = render::group_palette(c.saturating_sub(1) as usize);
    [p[0] as u8, p[1] as u8, p[2] as u8, 255]
}

fn side(c: u32) -> String {
    if c == PRED { "BEETLE".into() } else { format!("ANT {c}") }
}

fn put(buf: &mut [u8], w: u32, h: u32, x: i32, y: i32, c: [u8; 4]) {
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        return;
    }
    let i = ((y as u32 * w + x as u32) * 4) as usize;
    buf[i..i + 4].copy_from_slice(&c);
}

fn blend(buf: &mut [u8], w: u32, h: u32, x: i32, y: i32, c: [u8; 4], a: f32) {
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        return;
    }
    let i = ((y as u32 * w + x as u32) * 4) as usize;
    for k in 0..3 {
        buf[i + k] = (buf[i + k] as f32 * (1.0 - a) + c[k] as f32 * a) as u8;
    }
}

#[allow(clippy::too_many_arguments)]
fn shoot(
    lab: &mut Lab,
    out: &str,
    f: u64,
    kills: &[Kill],
    held: &BTreeMap<u32, Vec<bool>>,
    owners: &[(i32, u32)],
    snaps: &[Snap],
    ground_y: i32,
    (w, h): (i32, i32),
) {
    let (vw, vh) = (w as u32, h as u32);
    let mut world_buf = vec![0u8; (vw * vh * 4) as usize];
    let touched = lab.world.take_touched_chunks();
    lab.renderer.draw(&lab.world, &lab.particles, &touched, &mut world_buf, (vw, vh), true);
    let plain = format!("{out}/plain-{f}.png");
    image::save_buffer(&plain, &world_buf, vw, vh, image::ColorType::Rgba8).expect("plain png");

    // ---- the mock: box on top-left, log on the right, strip below.
    let log_w = 270u32;
    let strip_h = 90u32;
    let (mw, mh) = (vw + log_w, vh + strip_h);
    let mut m = vec![0u8; (mw * mh * 4) as usize];
    for p in m.chunks_mut(4) {
        p.copy_from_slice(&[18, 18, 22, 255]);
    }
    for y in 0..vh {
        let src = (y * vw * 4) as usize;
        let dst = (y * mw * 4) as usize;
        m[dst..dst + (vw * 4) as usize].copy_from_slice(&world_buf[src..src + (vw * 4) as usize]);
    }
    // territory: a tint in the air above each held column, and a solid band
    // two rows thick on the ground line so it reads at play zoom.
    for (&c, cols) in held {
        let col = colour_of(c);
        for (x, &on) in cols.iter().enumerate() {
            if !on {
                continue;
            }
            for y in (ground_y - 70).max(0)..ground_y - 4 {
                blend(&mut m, mw, mh, x as i32, y, col, 0.16);
            }
            for y in 2..6 {
                put(&mut m, mw, mh, x as i32, y, col);
            }
        }
    }
    // nests: a flag post in the owner's colour.
    for &(nx, c) in owners {
        let col = if c == 0 { [160, 160, 160, 255] } else { colour_of(c) };
        for y in 8..ground_y - 70 {
            if y % 2 == 0 {
                put(&mut m, mw, mh, nx, y, col);
            }
        }
        for dy in 0..4 {
            for dx in 1..6 {
                put(&mut m, mw, mh, nx + dx, 8 + dy, col);
            }
        }
    }
    // kill markers: an X in the victim's colour with a white core, last MARK_WINDOW frames.
    for k in kills.iter().filter(|k| k.frame + MARK_WINDOW >= f && k.frame <= f && k.victim != k.attacker) {
        let col = colour_of(k.victim);
        if k.attacker == PRED {
            // a predator's kill: a red ring round the victim's colour
            for d in -3..=3 {
                for (dx, dy) in [(d, -3), (d, 3), (-3, d), (3, d)] {
                    put(&mut m, mw, mh, k.x + dx, k.y + dy, colour_of(PRED));
                }
            }
            for d in -1..=1 {
                put(&mut m, mw, mh, k.x + d, k.y, col);
                put(&mut m, mw, mh, k.x, k.y + d, col);
            }
            continue;
        }
        for d in -2..=2 {
            put(&mut m, mw, mh, k.x + d, k.y + d, col);
            put(&mut m, mw, mh, k.x + d, k.y - d, col);
        }
        put(&mut m, mw, mh, k.x, k.y, [255, 255, 255, 255]);
    }
    // the strength strip: ants alive per colony over time (solid), food
    // intake rate per colony (thin, bottom half).
    let top = vh as i32 + 6;
    let hh = 40i32;
    let max_alive = snaps.iter().flat_map(|s| s.alive.values()).copied().max().unwrap_or(1).max(1);
    let n = snaps.len().max(2);
    let xs = |i: usize| (i as i64 * (vw as i64 - 8) / (n as i64 - 1)) as i32 + 4;
    let colonies: Vec<u32> = snaps.iter().flat_map(|s| s.alive.keys().copied()).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    for &c in &colonies {
        let col = colour_of(c);
        let mut prev: Option<(i32, i32)> = None;
        for (i, s) in snaps.iter().enumerate() {
            let a = s.alive.get(&c).copied().unwrap_or(0);
            let p = (xs(i), top + hh - (a as i32 * hh / max_alive as i32));
            if let Some(q) = prev {
                line(&mut m, mw, mh, q, p, col);
            }
            prev = Some(p);
        }
    }
    pixel_physics::hud::draw_text(&mut m, mw, mh, 4, top - 2, "ANTS ALIVE", [200, 200, 200, 255]);
    // food intake per window
    let top2 = top + hh + 6;
    let hh2 = 30i32;
    let rate = |i: usize, c: u32| -> f64 {
        if i == 0 {
            return 0.0;
        }
        snaps[i].intake.get(&c).copied().unwrap_or(0.0) - snaps[i - 1].intake.get(&c).copied().unwrap_or(0.0)
    };
    let max_rate = (1..snaps.len()).flat_map(|i| colonies.iter().map(move |&c| (i, c))).map(|(i, c)| rate(i, c)).fold(1.0, f64::max);
    for &c in &colonies {
        let col = colour_of(c);
        let mut prev: Option<(i32, i32)> = None;
        for i in 1..snaps.len() {
            let p = (xs(i), top2 + hh2 - (rate(i, c) / max_rate * hh2 as f64) as i32);
            if let Some(q) = prev {
                line(&mut m, mw, mh, q, p, col);
            }
            prev = Some(p);
        }
    }
    pixel_physics::hud::draw_text(&mut m, mw, mh, 4, top2 - 2, "FOOD IN", [200, 200, 200, 255]);
    // the side panel: a scoreboard and the battle log.
    let lx = vw as i32 + 6;
    let mut ly = 4;
    pixel_physics::hud::draw_text(&mut m, mw, mh, lx, ly, &format!("FRAME {f}"), [230, 230, 230, 255]);
    ly += 12;
    let last = snaps.last().cloned().unwrap_or_default();
    for &c in &colonies {
        let a = last.alive.get(&c).copied().unwrap_or(0);
        let won = kills.iter().filter(|k| k.attacker == c && k.victim != c && k.frame <= f).count();
        let lost = kills.iter().filter(|k| k.victim == c && k.attacker != c && k.frame <= f).count();
        pixel_physics::hud::draw_text(&mut m, mw, mh, lx, ly, &format!("ANT {c} {a:>3} ALIVE"), colour_of(c));
        ly += 9;
        pixel_physics::hud::draw_text(&mut m, mw, mh, lx + 6, ly, &format!("KILLED {won} LOST {lost}"), [190, 190, 190, 255]);
        ly += 9;
        let terr = held.get(&c).map(|v| v.iter().filter(|b| **b).count()).unwrap_or(0);
        pixel_physics::hud::draw_text(&mut m, mw, mh, lx + 6, ly, &format!("HOLDS {terr} COLS"), [190, 190, 190, 255]);
        ly += 12;
    }
    // the predators, as one more side
    let preds = lab
        .world
        .live_organism_ids()
        .into_iter()
        .filter(|id| lab.world.organism(*id).is_some_and(|st| is_ant(&lab.world, st.species) && !is_colony_species(&lab.world, st.species)))
        .count();
    if tracked().len() > 1 {
        let won = kills.iter().filter(|k| k.attacker == PRED && k.victim != PRED && k.frame <= f).count();
        let lost = kills.iter().filter(|k| k.victim == PRED && k.attacker != PRED && k.frame <= f).count();
        pixel_physics::hud::draw_text(&mut m, mw, mh, lx, ly, &format!("BEETLES {preds:>2} ALIVE"), colour_of(PRED));
        ly += 9;
        pixel_physics::hud::draw_text(&mut m, mw, mh, lx + 6, ly, &format!("ATE {won} ANTS LOST {lost}"), [190, 190, 190, 255]);
        ly += 12;
    }
    ly += 4;
    pixel_physics::hud::draw_text(&mut m, mw, mh, lx, ly, "BATTLE LOG", [230, 230, 230, 255]);
    ly += 11;
    // group the log into bouts: kills of one pair within 1,500 frames and 40 columns.
    let mut bouts: Vec<(u64, u64, u32, u32, i32, u32, &'static str)> = Vec::new();
    for k in kills.iter().filter(|k| k.frame <= f && k.victim != k.attacker) {
        if let Some(b) = bouts.iter_mut().rev().find(|b| b.2 == k.attacker && b.3 == k.victim && k.frame <= b.1 + 1500 && (b.4 - k.x).abs() <= 40) {
            b.1 = k.frame;
            b.5 += 1;
        } else {
            bouts.push((k.frame, k.frame, k.attacker, k.victim, k.x, 1, k.place));
        }
    }
    for b in bouts.iter().rev().take(((mh as i32 - ly) / 9).max(0) as usize) {
        let place = match b.6 {
            "victim_home" => "RAIDING NEST".to_string(),
            "killer_home" => "DEFENDING NEST".to_string(),
            "open_plants" => "AT FOOD".to_string(),
            _ => "IN THE OPEN".to_string(),
        };
        // "57K  3 KILLED 4 OF 2  AT FOOD", each colony number in its colour.
        let mut cx = lx;
        let mut say = |m: &mut Vec<u8>, t: &str, c: [u8; 4]| {
            pixel_physics::hud::draw_text(m, mw, mh, cx, ly, t, c);
            cx += 6 * t.chars().count() as i32;
        };
        let grey = [170, 170, 170, 255];
        say(&mut m, &format!("{:>3}K ", b.0 / 1000), [130, 130, 130, 255]);
        say(&mut m, &side(b.2), colour_of(b.2));
        say(&mut m, &format!(" KILLED {} OF ", b.5), grey);
        say(&mut m, &side(b.3), colour_of(b.3));
        say(&mut m, &format!(" {place}"), grey);
        ly += 9;
    }
    let mock = format!("{out}/mock-{f}.png");
    image::save_buffer(&mock, &m, mw, mh, image::ColorType::Rgba8).expect("mock png");
    println!("SHOT {f} {plain} {mock}");
}

fn line(buf: &mut [u8], w: u32, h: u32, (x0, y0): (i32, i32), (x1, y1): (i32, i32), c: [u8; 4]) {
    let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for i in 0..=n {
        let x = x0 + (x1 - x0) * i / n;
        let y = y0 + (y1 - y0) * i / n;
        put(buf, w, h, x, y, c);
        put(buf, w, h, x, y + 1, c);
    }
}
