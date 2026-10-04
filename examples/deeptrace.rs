//! **Every tick of a few ants' lives, in the owner's goal box: what each one
//! sensed, what its brain asked for, what the dice said and what it actually
//! did.**
//!
//! Built 2026-10-04 for the owner: *"a super in-depth trace of what's going on
//! in our current test environment ... tracking individual ants' brain
//! decisions at a much higher frequency than our other traces to understand
//! everything that ant is doing and why."* The other per-ant traces sample
//! every 200-2,000 frames (`digbox antlife=`, `nestdoor anttrace=`), which
//! sees *where* an ant was and never the decision that put it there.
//!
//! Runs the `nest_goal` scenario (dry, no plants, colony lands at 6,000,
//! one heap of player food 30 columns east of the nest kept topped up, as
//! `nestgoal` does) with the engine's own decision trace on
//! (`World::decision_log`, which takes no RNG draw and changes no branch),
//! and follows `ants=` focal ants (8):
//!
//! - **`ticks.csv.gz`**: one row per focal ant per decision (the ant thinks every
//!   `tick_interval` frames, 6 for the shipped ant), plus any frame between
//!   in which something happened to it (moved, load changed, died). Before
//!   the tick, every brain input, hidden unit and output read through
//!   `creature::probe_full` (non-mutating; it re-senses the ant where it
//!   stands, so an ant ticked late in the sweep may see a neighbour already
//!   moved -- one sweep of lag at most). After the tick, that ant's own
//!   `DecisionRow` (rolls, branch, chooser scores, drop, trail laid) and what
//!   changed in its state: crop cells, digestion, spoil, energy, its own life
//!   counters (moves, bites, digs, deliveries). Plus where it is (`zone`), the
//!   cell its jaw faces, and the brood touching its head.
//! - **`colony.csv`**: every live ant every `colonyevery=` frames (1,000):
//!   where, what it holds, energy, worker flag. Context for the focal lives.
//! - **`events.txt`**: focal picks, deaths (cause read off the world's death
//!   counters in the frame it died, so *inferred* when two die at once), and
//!   replacements.
//! - **`map_fNNNNNN.txt`**: the ground round the nest every `mapevery=`
//!   frames (25,000) as one character a cell, for drawing paths on.
//! - with `shots=1`, the lab's own picture at the same frames.
//!
//! **Focal ants.** Half the slots are founders picked at landing, the rest
//! the first ants born after `bornafter=` (20,000). A focal ant that dies is
//! replaced by the youngest untraced adult, so the trace always follows
//! `ants=` animals; every row carries its slot and the ant's age.
//!
//! ```text
//! cargo run --release --example deeptrace -- seed=1 frames=300000 out=/mnt/project-files/deep-trace/s1
//! ```
//!
//! `scripts/deeptrace.py OUT` reads the result into a life story per ant.

use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::brain::{BRAIN_HIDDEN, BRAIN_INPUTS, BRAIN_OUTPUTS, INPUT_NAMES, OUTPUT_NAMES};
use pixel_physics::sim::cell::{Cell, OrganismId};
use pixel_physics::sim::creature::{
    self, DecisionRow, DECISION_LEG_NAMES, DECISION_OUTCOME_NAMES, DIRS, DROP_WHY_NAMES, HOMEWARD_WHY_NAMES,
};
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::organism::{self, BroodStage};
use pixel_physics::sim::world::World;
use std::collections::{HashMap, HashSet};
use std::io::Write;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

/// How often the food heap is topped up, and how wide it is -- `nestgoal`'s.
const TOP_EVERY: u64 = 250;
const FOOD_REACH: i32 = 12;
/// Columns either side of the nest counted as the mound.
const MOUND_REACH: i32 = 40;

struct Geo {
    ground_y: i32,
    nest_x: i32,
    food_x: i32,
}

/// Where an ant's head is, in the goal box's own words.
/// `food` on or beside the heap; `nest` below the old ground line;
/// `mound_in` above it near the nest with ground over the head (inside the
/// spoil mound); `mound_top` near the nest open to the sky; `surface`
/// anywhere else above ground.
fn zone(w: &World, g: &Geo, (x, y): (i32, i32)) -> &'static str {
    if (x - g.food_x).abs() <= FOOD_REACH && y <= g.ground_y + 4 && y >= g.ground_y - 40 {
        return "food";
    }
    if y > g.ground_y {
        return "nest";
    }
    if (x - g.nest_x).abs() <= MOUND_REACH {
        let covered = (1..=30).any(|k| {
            let (cx, cy) = (x, y - k);
            w.in_bounds(cx, cy) && {
                let c = w.get(cx, cy);
                c.material != material::EMPTY
                    && c.organism_id() == 0
                    && matches!(w.materials.kind(c.material), MaterialKind::Powder | MaterialKind::Solid)
            }
        });
        return if covered { "mound_in" } else { "mound_top" };
    }
    "surface"
}

/// The cell the jaw faces, as `digbox antlife=` names it.
fn ahead(w: &World, (hx, hy): (i32, i32), heading: u8) -> &'static str {
    let (dx, dy) = DIRS[(heading % 8) as usize];
    let (x, y) = (hx + dx, hy + dy);
    if !w.in_bounds(x, y) {
        return "wall";
    }
    let c = w.get(x, y);
    let m = w.materials.get(c.material);
    if c.material == material::EMPTY {
        "empty"
    } else if w.materials.kind(c.material) == MaterialKind::Creature {
        "ant"
    } else if Some(c.material) == w.materials.id_of("brood") {
        "brood"
    } else if w.materials.kind(c.material) == MaterialKind::Liquid {
        "water"
    } else if c.organism_id() == 0 && creature::food_value(w, c) > 0.0 {
        "food"
    } else if c.organism_id() == 0 && m.penetration_resistance <= 1.0 {
        "cuttable"
    } else {
        "hard"
    }
}

/// Brood touching the head (its eight neighbours): eggs, larvae, hungry
/// larvae (bank under the pupation target), pupae.
fn brood_round(w: &World, (hx, hy): (i32, i32)) -> [u8; 4] {
    let mut n = [0u8; 4];
    let mut seen = HashSet::new();
    for dy in -1..=1 {
        for dx in -1..=1 {
            if (dx, dy) == (0, 0) || !w.in_bounds(hx + dx, hy + dy) {
                continue;
            }
            let id = w.get(hx + dx, hy + dy).organism_id();
            if id == 0 || !seen.insert(id) {
                continue;
            }
            let Some(s) = w.organism(id) else { continue };
            let Some(b) = s.brood else { continue };
            match b.stage {
                BroodStage::Egg => n[0] += 1,
                BroodStage::Larva => {
                    n[1] += 1;
                    n[2] += u8::from(s.energy < b.target);
                }
                BroodStage::Pupa => n[3] += 1,
            }
        }
    }
    n
}

#[derive(Clone, Copy)]
struct Snap {
    head: (i32, i32),
    energy: f32,
    crop_cells: u16,
    crop_worth: f32,
    digesting: f32,
    spoil: u8,
    moves: u32,
    blocked: u32,
    bites: u32,
    digs: u32,
    deliveries: u32,
}

fn snap(w: &World, id: OrganismId) -> Option<Snap> {
    let s = w.organism(id)?;
    let head = *s.chain.first()?;
    Some(Snap {
        head,
        energy: s.energy,
        crop_cells: s.crop.as_ref().map_or(0, |c| c.cells),
        crop_worth: s.crop.as_ref().map_or(0.0, |c| c.worth()),
        digesting: s.crop.as_ref().map_or(0.0, |c| c.digesting),
        spoil: s.spoil.as_ref().map_or(0, |sp| 1 + u8::from(sp.store)),
        moves: s.life.moves,
        blocked: s.life.moves_blocked,
        bites: s.life.bites,
        digs: s.life.digs,
        deliveries: s.life.deliveries,
    })
}

/// Floats short: integers bare, otherwise three places, trailing zeros cut.
fn fl(v: f32) -> String {
    if v.is_nan() {
        return String::new();
    }
    if v == v.trunc() && v.abs() < 1e7 {
        return format!("{}", v as i64);
    }
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.into()
    }
}

fn is_ant(w: &World, sid: organism::SpeciesId, id: OrganismId) -> bool {
    w.organism(id)
        .is_some_and(|s| s.species == sid && s.brood.is_none() && !s.chain.is_empty())
}

/// One tracked ant's reading before the tick, kept raw until it is written.
struct Pre {
    k: usize,
    id: OrganismId,
    sn: Snap,
    worker: bool,
    colony: u32,
    zone: &'static str,
    heading: u8,
    ahead: &'static str,
    brood: [u8; 4],
    since_nest: u16,
    hungry_home: bool,
    lunch: bool,
    nest_bound: u64,
    anchor: (i32, i32),
    spoil_ring: Option<i32>,
    dig_return: Option<(i32, i32)>,
    // The body behind the numbers: what the crop and the jaw hold, how well
    // this gut takes the crop's food (`creature::diet_quality`, 0..1), and
    // the expressed traits that set how fast it digests and decides. Added
    // after a young starver was seen holding 475 J in its crop at 25 J of
    // body energy and gaining 0.02 J a frame from it.
    crop_mat: Option<material::MaterialId>,
    crop_q: f32,
    spoil_mat: Option<material::MaterialId>,
    gut: f32,
    digest: f32,
    tick: u64,
    inp: [f32; BRAIN_INPUTS],
    hid: [f32; BRAIN_HIDDEN],
    outp: [f32; BRAIN_OUTPUTS],
}

impl Pre {
    fn format(&self, f: u64, born: u64, names: &[String]) -> String {
        let sn = &self.sn;
        let mut s = format!(
            "{f},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            if self.k == usize::MAX {
                String::new()
            } else {
                self.k.to_string()
            },
            self.id,
            f.saturating_sub(born),
            u8::from(self.worker),
            self.colony,
            sn.head.0,
            sn.head.1,
            self.zone,
            self.heading,
            self.ahead,
            self.brood[0],
            self.brood[1],
            self.brood[2],
            self.brood[3],
            fl(sn.energy),
            sn.crop_cells,
            fl(sn.crop_worth),
            fl(sn.digesting),
            sn.spoil,
            self.since_nest,
            u8::from(self.hungry_home),
            u8::from(self.lunch),
            if self.nest_bound == u64::MAX {
                "always".to_string()
            } else {
                self.nest_bound.saturating_sub(f).to_string()
            },
            self.anchor.0,
            self.anchor.1,
            self.spoil_ring.map_or(String::new(), |c| c.to_string()),
            self.dig_return.map_or(String::new(), |(x, y)| format!("{x} {y}")),
        );
        let name = |m: Option<material::MaterialId>| m.map_or(String::new(), |m| names[m.0 as usize].clone());
        s.push_str(&format!(
            ",{},{},{},{},{},{}",
            name(self.crop_mat),
            if self.crop_mat.is_some() {
                fl(self.crop_q)
            } else {
                String::new()
            },
            name(self.spoil_mat),
            fl(self.gut),
            fl(self.digest),
            self.tick
        ));
        for v in self.inp.iter().chain(self.hid.iter()).chain(self.outp.iter()) {
            s.push(',');
            s.push_str(&fl(*v));
        }
        s
    }
}

fn main() {
    let seed: u64 = arg("seed").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(300_000);
    // `ants=all` (the default) records every live ant at every decision; a
    // number follows that many focal ants instead (see the module doc).
    let ants_arg: String = arg("ants").unwrap_or_else(|| "all".to_string());
    let all_ants = ants_arg == "all";
    // `only=id,id,...` with `ants=all`: write (and probe) just these ants. The
    // world is the same run either way -- the trace takes no draw -- so a
    // re-run with `only=` reproduces any ant of an earlier full run cheaply.
    let only: HashSet<OrganismId> = arg::<String>("only")
        .map(|v| v.split(',').filter_map(|t| t.trim().parse().ok()).collect())
        .unwrap_or_default();
    let n_ants: usize = if all_ants {
        0
    } else {
        ants_arg.parse().expect("ants=all or a number")
    };
    let born_after: u64 = arg("bornafter").unwrap_or(20_000);
    let colony_every: u64 = arg("colonyevery").unwrap_or(1_000);
    let map_every: u64 = arg("mapevery").unwrap_or(25_000);
    let report_every: u64 = arg("every").unwrap_or(10_000);
    let target: usize = arg("food").unwrap_or(120);
    let shots = arg::<u8>("shots").unwrap_or(0) != 0;
    let scenario: String = arg("scenario").unwrap_or_else(|| "nest_goal".to_string());
    let out: String = arg("out").unwrap_or_else(|| "deeptrace-out".to_string());
    println!(
        "deeptrace: scenario={scenario} seed={seed} frames={frames} ants={n_ants} bornafter={born_after} colonyevery={colony_every} mapevery={map_every} food={target} shots={} out={out}",
        u8::from(shots)
    );
    std::fs::create_dir_all(&out).expect("out dir");
    let mut sc = Scenario::load(&scenario).unwrap_or_else(|e| {
        eprintln!("scenario {scenario}: {e}");
        std::process::exit(1);
    });
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    let msg = lab.load_scenario(sc);
    lab.show_help = false;
    if lab.stats.showing() {
        lab.stats.toggle();
    }
    println!("  {msg}");
    if shots {
        for _ in 1..3 {
            lab.renderer.adjust_zoom(1);
        }
    }
    let ground_y = lab.spec.ground_y;

    // ticks.csv.gz through the system gzip: a 300k-frame run is ~2M rows.
    let mut gz = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("gzip -4 > '{out}/ticks.csv.gz'"))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("gzip");
    let mut ticks = std::io::BufWriter::with_capacity(1 << 20, gz.stdin.take().expect("gzip stdin"));
    let mut header = String::from(
        "frame,slot,id,age,worker,colony,hx,hy,zone,heading,ahead,eggs,larvae,hungry_larvae,pupae,energy_j,crop_cells,crop_j,digesting,spoil,since_nest,hungry_home,lunch,nest_bound,anchor_x,anchor_y,spoil_ring,dig_return,crop_mat,crop_q,spoil_mat,gut,digest,tick",
    );
    for n in INPUT_NAMES {
        header.push_str(&format!(",i_{n}"));
    }
    for h in 0..BRAIN_HIDDEN {
        header.push_str(&format!(",h{h}"));
    }
    for n in OUTPUT_NAMES {
        header.push_str(&format!(",o_{n}"));
    }
    header.push_str(
        ",row,leg,fill,outcome,p_move,roll_move,roll_tumble,turn,homeward,home_cos,moved,drop,drop_p,drop_roll,free8,drop_reach,pick,patience,chosen_cos,chosen_route,drive,scout_w,scout_home,dig_turned,trip_load,forage_max,since_trip,trip_src,emit_a,emit_b,emit_b_brain,usable,opts,cross,k,chose,reads_b,b_near,heading_after,hx_after,hy_after,d_energy,d_crop_cells,d_crop_j,d_digesting,spoil_after,d_moves,d_blocked,d_bites,d_digs,d_deliveries",
    );
    writeln!(ticks, "{header}").unwrap();
    let mut colony_csv = std::io::BufWriter::new(std::fs::File::create(format!("{out}/colony.csv")).unwrap());
    writeln!(colony_csv, "frame,id,worker,hx,hy,zone,crop_cells,spoil,energy_j,home").unwrap();
    let mut events = std::io::BufWriter::new(std::fs::File::create(format!("{out}/events.txt")).unwrap());

    let sid = lab.world.species.id_of("ant").expect("ant ships");
    let def = lab
        .world
        .species
        .get(sid)
        .creature
        .as_ref()
        .cloned()
        .expect("ant is a creature");
    lab.world.decision_log = Some(Vec::new());
    let names: Vec<String> = (0..lab.world.materials.len())
        .map(|i| lab.world.materials.get(material::MaterialId(i as u16)).name.clone())
        .collect();
    // **Experiment dials, harness-only.** `mutation=<rate>` overrides the
    // ant's per-slot mutation rate (0 freezes the founders' brain);
    // `knockin=<slot>:<value>[,...]` writes those genome slots into the
    // species genome before the colony lands, to test one allele in a whole colony.
    if let Some(m) = arg::<f32>("mutation") {
        if let Some(c) = lab.world.species.get_mut(sid).creature.as_mut() {
            c.mutation_rate = m;
        }
        println!("  mutation rate set to {m}");
    }
    let knockin: Vec<(usize, f32)> = arg::<String>("knockin")
        .map(|s| {
            s.split(',')
                .filter_map(|kv| {
                    kv.split_once(':')
                        .and_then(|(k, v)| Some((k.parse().ok()?, v.parse().ok()?)))
                })
                .collect()
        })
        .unwrap_or_default();
    if !knockin.is_empty() {
        // Into the species genome before the colony lands, so every founder carries it.
        let mut genome = lab.world.species.get(sid).genome.clone();
        for &(k, v) in &knockin {
            genome[k] = v;
        }
        lab.world.species.set_genome(sid, genome);
        println!("  knock-in {knockin:?}");
    }

    let mut geo: Option<Geo> = None;
    let mut slots: Vec<Option<OrganismId>> = vec![None; n_ants];
    let mut ever: HashSet<OrganismId> = HashSet::new();
    let mut born: HashMap<OrganismId, u64> = HashMap::new();
    let mut known: HashSet<OrganismId> = HashSet::new();
    let mut founders_picked = false;
    // The founders' consensus genome, slot by slot (the most common value),
    // and a `genomes_fNNNNNN.txt` at every map frame listing, per live ant,
    // only the slots where it differs. Brains drift by mutation, so the
    // weights in `ant.ron` are not the weights a 200k-frame colony runs.
    let mut consensus: Option<Vec<f32>> = None;
    let mut genome_out = std::io::BufWriter::new(std::fs::File::create(format!("{out}/genomes.txt")).unwrap());
    let mut dropped = 0usize;
    let mut rows_written = 0u64;
    let started = std::time::Instant::now();
    // Simple deterministic picker: the seed's own LCG, so a rerun picks the same ants.
    let mut lcg = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let mut next = move || {
        lcg = lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (lcg >> 33) as usize
    };

    for f in 0..=frames {
        if geo.is_none() {
            if let Some(s) = lab.world.nest_sites.first() {
                let g = Geo {
                    ground_y,
                    nest_x: s.x,
                    food_x: s.x + 30,
                };
                println!(
                    "  colony founded by frame {f}: nest at x {}, food spot x {}",
                    g.nest_x, g.food_x
                );
                writeln!(
                    events,
                    "{f} FOUNDED nest_x={} food_x={} ground_y={ground_y}",
                    g.nest_x, g.food_x
                )
                .unwrap();
                geo = Some(g);
            }
        }
        let Some(g) = geo.as_ref() else {
            lab.tick_for_harness();
            if let Some(log) = lab.world.decision_log.as_mut() {
                log.clear();
            }
            continue;
        };
        if f % TOP_EVERY == 0 {
            dropped += top_up(&mut lab.world, g, target);
        }
        // Who is alive now: births are booked the first frame an id is seen.
        let live: Vec<OrganismId> = lab
            .world
            .live_organism_ids()
            .into_iter()
            .filter(|&id| is_ant(&lab.world, sid, id))
            .collect();
        for &id in &live {
            if known.insert(id) {
                born.insert(id, if founders_picked { f } else { 0 });
            }
        }
        // Fill empty slots: founders first, then the first born after `bornafter`, then the youngest.
        let half = n_ants / 2;
        if !founders_picked && !live.is_empty() {
            let mut pool = live.clone();
            for (k, slot) in slots.iter_mut().enumerate().take(half) {
                if pool.is_empty() {
                    break;
                }
                let id = pool.swap_remove(next() % pool.len());
                *slot = Some(id);
                ever.insert(id);
                writeln!(events, "{f} PICK slot={k} founder id={id}").unwrap();
            }
            founders_picked = true;
        }
        for (k, slot) in slots.iter_mut().enumerate() {
            if slot.is_some() || !founders_picked {
                continue;
            }
            if k >= half && f < born_after {
                continue;
            }
            // The youngest untraced adult (latest booked birth; ties by id).
            let pick = live
                .iter()
                .filter(|id| !ever.contains(id))
                .max_by_key(|id| (born.get(id).copied().unwrap_or(0), **id))
                .copied();
            if let Some(id) = pick {
                *slot = Some(id);
                ever.insert(id);
                writeln!(
                    events,
                    "{f} PICK slot={k} id={id} born={}",
                    born.get(&id).copied().unwrap_or(0)
                )
                .unwrap();
            }
        }

        // Before the tick: the brain's reading and the state. Raw values
        // only; a row is formatted only if it is written (`all` reads every
        // live ant every frame and writes about one in six).
        let tracked: Vec<(usize, OrganismId)> = if all_ants {
            live.iter()
                .filter(|id| only.is_empty() || only.contains(id))
                .map(|&id| (usize::MAX, id))
                .collect()
        } else {
            slots
                .iter()
                .enumerate()
                .filter_map(|(k, s)| s.map(|id| (k, id)))
                .collect()
        };
        let mut pre: Vec<Pre> = Vec::with_capacity(tracked.len());
        for (k, id) in tracked {
            let Some(sn) = snap(&lab.world, id) else { continue };
            let w = &lab.world;
            let st = w.organism(id).expect("snapped");
            let (inp, hid, outp, _) = creature::probe_full(w, sn.head.0, sn.head.1, id, &def);
            let traits = creature::expressed_traits(st, w.plasticity, w.trait_reach);
            pre.push(Pre {
                k,
                id,
                sn,
                worker: st.nest_bound_until == u64::MAX,
                colony: st.colony,
                zone: zone(w, g, sn.head),
                heading: st.heading,
                ahead: ahead(w, sn.head, st.heading),
                brood: brood_round(w, sn.head),
                since_nest: st.since_nest,
                hungry_home: st.hungry_home,
                lunch: st.lunch,
                nest_bound: st.nest_bound_until,
                anchor: st.forage_anchor,
                spoil_ring: st.spoil_ring,
                dig_return: st.dig_return,
                crop_mat: st.crop.as_ref().map(|c| c.material),
                crop_q: st.crop.as_ref().map_or(0.0, |c| {
                    creature::diet_quality(w, c.material, traits[organism::TRAIT_GUT_BIAS])
                }),
                spoil_mat: st.spoil.as_ref().map(|sp| sp.cell.material),
                gut: traits[organism::TRAIT_GUT_BIAS],
                digest: creature::digest_rate_of(&def, &traits),
                tick: creature::tick_interval_of(&def, &traits),
                inp,
                hid,
                outp,
            });
        }
        let deaths_before = lab.world.deaths_by_cause;

        if f < frames {
            lab.tick_for_harness();
        }

        // After the tick: the decision rows and what changed.
        let rows: Vec<DecisionRow> = lab.world.decision_log.as_mut().map(std::mem::take).unwrap_or_default();
        let mut by_id: HashMap<OrganismId, DecisionRow> = HashMap::new();
        let wanted: HashSet<OrganismId> = pre.iter().map(|p| p.id).collect();
        for r in rows {
            if wanted.contains(&r.id) {
                // One decision a tick is the normal case; keep the last if not.
                by_id.insert(r.id, r);
            }
        }
        let w = &lab.world;
        for p in pre {
            let (k, id, before) = (p.k, p.id, p.sn);
            // **One row per decision.** The ant thinks every `tick_interval`
            // frames (6 for the shipped ant); between decisions it only gets
            // a row if something happened to it -- it moved (fell, was
            // pushed), its crop or soil load changed, or it died.
            if !by_id.contains_key(&id) {
                let quiet = snap(w, id).is_some_and(|a| {
                    a.head == before.head
                        && a.crop_cells == before.crop_cells
                        && a.spoil == before.spoil
                        && a.digs == before.digs
                });
                if quiet {
                    continue;
                }
            }
            let mut s = p.format(f, born.get(&id).copied().unwrap_or(0), &names);
            match by_id.get(&id) {
                Some(r) => s.push_str(&format!(
                    ",1,{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                    DECISION_LEG_NAMES.get(r.leg as usize).unwrap_or(&"?"),
                    fl(r.fill),
                    DECISION_OUTCOME_NAMES[r.outcome as usize],
                    fl(r.p_move),
                    fl(r.roll_move),
                    fl(r.roll_tumble),
                    fl(r.turn),
                    HOMEWARD_WHY_NAMES[r.homeward as usize],
                    fl(r.home_cos),
                    u8::from(r.moved),
                    DROP_WHY_NAMES[r.drop as usize],
                    fl(r.drop_p),
                    fl(r.drop_roll),
                    r.free8,
                    r.drop_reach,
                    if r.pick == creature::NO_PICK { String::new() } else { r.pick.to_string() },
                    fl(r.patience),
                    fl(r.chosen_cos),
                    fl(r.chosen_route),
                    fl(r.drive),
                    fl(r.scout_w),
                    u8::from(r.scout_home),
                    u8::from(r.dig_turned),
                    u8::from(r.trip_load),
                    r.forage_max,
                    r.since_trip,
                    r.trip_src,
                    r.emit_a_laid,
                    r.emit_b_laid,
                    fl(r.emit_b_brain),
                    r.usable.count_ones(),
                    r.opts,
                    u8::from(r.cross),
                    fl(r.k),
                    r.chose,
                    u8::from(r.reads_b),
                    r.b_near.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "),
                    r.heading_after,
                    r.head_after.0,
                    r.head_after.1,
                )),
                None => s.push_str(",0"),
            }
            if !by_id.contains_key(&id) {
                // Pad the decision columns so every row has the same width.
                s.push_str(&",".repeat(40));
            }
            match snap(w, id) {
                Some(a) => s.push_str(&format!(
                    ",{},{},{},{},{},{},{},{},{},{}",
                    fl(a.energy - before.energy),
                    i32::from(a.crop_cells) - i32::from(before.crop_cells),
                    fl(a.crop_worth - before.crop_worth),
                    fl(a.digesting - before.digesting),
                    a.spoil,
                    a.moves - before.moves,
                    a.blocked - before.blocked,
                    a.bites - before.bites,
                    a.digs - before.digs,
                    a.deliveries - before.deliveries,
                )),
                None => {
                    // Died this tick. The cause is whichever counter moved.
                    let causes: Vec<&str> = organism::DEATH_CAUSE_LIST
                        .iter()
                        .filter(|c| w.deaths_by_cause[c.index()] > deaths_before[c.index()])
                        .map(|c| c.label())
                        .collect();
                    writeln!(
                        events,
                        "{f} DIED slot={k} id={id} age={} at=({},{}) zone={} energy={:.0} crop_cells={} cause={}",
                        f.saturating_sub(born.get(&id).copied().unwrap_or(0)),
                        before.head.0,
                        before.head.1,
                        zone(w, g, before.head),
                        before.energy,
                        before.crop_cells,
                        if causes.is_empty() {
                            "?".to_string()
                        } else {
                            causes.join("+")
                        }
                    )
                    .unwrap();
                    if k < slots.len() {
                        slots[k] = None;
                    }
                    s.push_str(",,,,,,,,,,");
                }
            }
            writeln!(ticks, "{s}").unwrap();
            rows_written += 1;
        }

        if f % colony_every == 0 {
            for &id in &live {
                let Some(st) = w.organism(id) else { continue };
                let Some(&h) = st.chain.first() else { continue };
                let home = (-1..=1).any(|dy| (-1..=1).any(|dx| w.nest_dug.contains(&(h.0 + dx, h.1 + dy))));
                writeln!(
                    colony_csv,
                    "{f},{id},{},{},{},{},{},{},{:.0},{}",
                    u8::from(st.nest_bound_until == u64::MAX),
                    h.0,
                    h.1,
                    zone(w, g, h),
                    st.crop.as_ref().map_or(0, |c| c.cells),
                    st.spoil.as_ref().map_or(0, |_| 1),
                    st.energy,
                    u8::from(home)
                )
                .unwrap();
            }
        }
        if consensus.is_none() && !live.is_empty() {
            let gs: Vec<&Vec<f32>> = live
                .iter()
                .filter_map(|&id| w.organism(id).map(|s| &s.genome))
                .collect();
            let len = gs.first().map_or(0, |g| g.len());
            let mut c = vec![0.0f32; len];
            for (i, slot) in c.iter_mut().enumerate() {
                let mut counts: Vec<(u32, usize)> = Vec::new();
                for g in &gs {
                    let b = g[i].to_bits();
                    match counts.iter_mut().find(|(v, _)| *v == b) {
                        Some(e) => e.1 += 1,
                        None => counts.push((b, 1)),
                    }
                }
                *slot = f32::from_bits(counts.iter().max_by_key(|e| e.1).map_or(0, |e| e.0));
            }
            writeln!(
                genome_out,
                "{f} CONSENSUS {}",
                c.iter()
                    .enumerate()
                    .filter(|(_, v)| **v != 0.0)
                    .map(|(i, v)| format!("{i}:{v}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
            .unwrap();
            consensus = Some(c);
        }
        if f % map_every == 0 && f > 0 {
            if let Some(c) = consensus.as_ref() {
                for &id in &live {
                    let Some(st) = w.organism(id) else { continue };
                    let diff: Vec<String> = st
                        .genome
                        .iter()
                        .zip(c.iter())
                        .enumerate()
                        .filter(|(_, (a, b))| a != b)
                        .map(|(i, (a, _))| format!("{i}:{a}"))
                        .collect();
                    writeln!(genome_out, "{f} {id} {}", diff.join(" ")).unwrap();
                }
            }
            write_map(w, g, &format!("{out}/map_f{f:06}.txt"));
            if shots {
                let centre = (g.nest_x + 10, ground_y + 12);
                shot(&mut lab, &out, f, centre);
            }
        }
        if f % report_every == 0 {
            let brood = lab.world.live_brood_ids().len();
            println!(
                "frame={f} ants={} brood={brood} traced={} rows={rows_written} food_dropped={dropped} wall={:.0}s",
                live.len(),
                slots.iter().flatten().count(),
                started.elapsed().as_secs_f32()
            );
        }
    }
    ticks.flush().unwrap();
    drop(ticks);
    let _ = gz.wait();
    events.flush().unwrap();
    genome_out.flush().unwrap();
    colony_csv.flush().unwrap();
    println!("deeptrace: done, {rows_written} focal rows");
}

/// The box round the nest and food as one character a cell: `.` air, `#`
/// ground, `s` loose spoil (powder), `~` water, `a` ant, `b` brood, `f` food.
/// First line: `x0 y0 width height`.
fn write_map(w: &World, g: &Geo, path: &str) {
    let (x0, x1) = (g.nest_x - 80, g.food_x + 40);
    let (y0, y1) = (g.ground_y - 60, g.ground_y + 70);
    let brood = w.materials.id_of("brood");
    let mut s = format!("{x0} {y0} {} {}\n", x1 - x0 + 1, y1 - y0 + 1);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let ch = if !w.in_bounds(x, y) {
                '#'
            } else {
                let c = w.get(x, y);
                let kind = w.materials.kind(c.material);
                if c.material == material::EMPTY {
                    '.'
                } else if Some(c.material) == brood {
                    'b'
                } else if kind == MaterialKind::Creature {
                    'a'
                } else if kind == MaterialKind::Liquid {
                    '~'
                } else if c.organism_id() == 0 && creature::food_value(w, c) > 0.0 {
                    'f'
                } else if kind == MaterialKind::Powder {
                    's'
                } else {
                    '#'
                }
            };
            s.push(ch);
        }
        s.push('\n');
    }
    let _ = std::fs::write(path, s);
}

fn top_up(w: &mut World, g: &Geo, target: usize) -> usize {
    let provisions = w.materials.id_of("provisions").expect("provisions ships");
    let mut have = 0;
    for y in (g.ground_y - 40)..=(g.ground_y + 4) {
        for x in (g.food_x - FOOD_REACH)..=(g.food_x + FOOD_REACH) {
            have += usize::from(w.in_bounds(x, y) && w.get(x, y).material == provisions);
        }
    }
    if have >= target {
        return 0;
    }
    let mut want = target - have;
    let mut dropped = 0;
    let top = g.ground_y - 30;
    'rows: for y in top..(top + 6) {
        for k in 0..=4 {
            for x in [g.food_x - k, g.food_x + k] {
                if want == 0 {
                    break 'rows;
                }
                if w.in_bounds(x, y) && w.get(x, y).material == material::EMPTY {
                    w.set(x, y, Cell::new(provisions, 0));
                    want -= 1;
                    dropped += 1;
                }
            }
        }
    }
    dropped
}

fn shot(lab: &mut Lab, dir: &str, frame: u64, centre: (i32, i32)) {
    let (full_w, full_h) = (WIDTH, HEIGHT);
    let bounds = pixel_physics::sim::chunk::Rect::new(0, 0, lab.spec.width - 1, lab.spec.height - 1);
    let (span_x, span_y) = lab.renderer.visible_span((full_w, full_h));
    lab.renderer.set_camera(
        centre.0 - span_x / 2,
        centre.1 - span_y / 2,
        (full_w, full_h),
        Some(bounds),
    );
    let mut full = vec![0u8; (full_w * full_h * 4) as usize];
    lab.draw(&mut full, 60.0);
    let (y0, rows) = (24u32, 216u32);
    let crop = full[(y0 * full_w * 4) as usize..((y0 + rows) * full_w * 4) as usize].to_vec();
    if let Some(img) = image::RgbaImage::from_raw(full_w, rows, crop) {
        let path = std::path::Path::new(dir).join(format!("shot_f{frame:06}.png"));
        let _ = image::imageops::resize(&img, full_w * 2, rows * 2, image::imageops::FilterType::Nearest).save(&path);
    }
}
