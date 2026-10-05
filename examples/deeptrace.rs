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
//! - with `dig=1`, **the digging record** (see `DigLog`): every decision of
//!   every ant in a narrow row with the dig's whole funnel (`digrows.csv.gz`),
//!   every cut with the shape of the ground round it (`cuts.csv`), every cell
//!   that turned from ground to open or back and who did it
//!   (`cells.csv.gz`), the brood (`brood.csv`), every egg laid and every
//!   brood item moved, frame by frame (`broodlog.csv`), and the nest as a
//!   picture (`nest_fNNNNNN.txt`) every `nestevery=` frames (2,500). No brain probe,
//!   so `ants=0 dig=1` records the whole colony for a fraction of `ants=all`.
//!
//! `founder=evolved` lands the colony with lane 2's evolved founder (the six
//! scenario rows in `EVOLVED_FOUNDER`), before any `gut=`.
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
//! `scripts/deeptrace.py OUT` reads the result into a life story per ant;
//! `scripts/deeptrace_dig.py dig|soil|rooms|brood|journeys|face OUT` reads the
//! `dig=1` record.

use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::brain::{BRAIN_HIDDEN, BRAIN_INPUTS, BRAIN_OUTPUTS, INPUT_NAMES, OUTPUT_NAMES};
use pixel_physics::sim::cell::{Cell, OrganismId};
use pixel_physics::sim::creature::{
    self, DecisionRow, DigWhy, DECISION_LEG_NAMES, DECISION_OUTCOME_NAMES, DIG_NO_TARGET, DIG_WHY_NAMES, DIRS,
    DROP_WHY_NAMES, HOMEWARD_WHY_NAMES,
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

/// **Lane 2's evolved founder** (2026-10-04): the six scenario rows that are
/// bit-identical to draft PR 617's `PIXEL_PHYSICS_LAB_ANT=evolved`. The last
/// two are species rows on purpose. `founder=evolved` pushes them.
const EVOLVED_FOUNDER: [(&str, f32); 6] = [
    ("gut_bias", -0.8),
    ("birth_grant", -0.27),
    ("reproduce_at", -0.14),
    ("pace", 0.21),
    ("curvature_radius", 3.0),
    ("digest_rate", 3.96),
];

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
    // Births inherit exactly unless `PIXEL_PHYSICS_MUTATION=on` (PR 611).
    // The seed-1 recording of 2026-10-04 was made with mutation ON (before
    // the switch); to replay it, run with `PIXEL_PHYSICS_MUTATION=on`.
    // Mutation-off runs: the lab founders' gut (0) gets only a quarter of the
    // player's plant-class food -- set founders' gut_bias to -0.5 in the
    // scenario until the owner picks how tests start the diet.
    creature::mutation_off_for_measuring();
    let seed: u64 = arg("seed").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(300_000);
    // `ants=all` (the default) records every live ant at every decision; a
    // number follows that many focal ants instead (see the module doc).
    let ants_arg: String = arg("ants").unwrap_or_else(|| "all".to_string());
    let all_ants = ants_arg == "all";
    // `only=id,id,...` with `ants=all`: write (and probe) just these ants. The
    // world is the same run either way -- the trace takes no draw -- so a
    // re-run with `only=` reproduces any ant of an earlier full run cheaply.
    let census = arg::<u8>("census").unwrap_or(0) == 1;
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
    let dig = arg::<u8>("dig").unwrap_or(0) == 1;
    let nest_every: u64 = arg("nestevery").unwrap_or(2_500);
    println!(
        "deeptrace: scenario={scenario} seed={seed} frames={frames} ants={n_ants} bornafter={born_after} colonyevery={colony_every} mapevery={map_every} food={target} shots={} dig={} nestevery={nest_every} out={out}",
        u8::from(shots),
        u8::from(dig)
    );
    std::fs::create_dir_all(&out).expect("out dir");
    let mut sc = Scenario::load(&scenario).unwrap_or_else(|e| {
        eprintln!("scenario {scenario}: {e}");
        std::process::exit(1);
    });
    sc.bed.seed = seed;
    if arg::<String>("founder").as_deref() == Some("evolved") {
        for (field, value) in EVOLVED_FOUNDER {
            sc.settings.push(pixel_physics::lab::scenario::Setting {
                subject: "ant".into(),
                field: field.into(),
                value,
            });
        }
        println!("  evolved founder: {EVOLVED_FOUNDER:?}");
    }
    // `gut=<v>`: the founders' diet (`gut_bias`), as a scenario setting. With
    // mutation off the authored 0 takes only a quarter of plant-class food
    // and the colony stalls near 35 ants; -0.5 is what seed 1 drifted to.
    if let Some(v) = arg::<f32>("gut") {
        sc.settings.push(pixel_physics::lab::scenario::Setting {
            subject: "ant".into(),
            field: "gut_bias".into(),
            value: v,
        });
        println!("  founders' gut_bias set to {v}");
    }
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
        ",row,leg,fill,outcome,p_move,roll_move,roll_tumble,turn,homeward,home_cos,moved,drop,drop_p,drop_roll,free8,drop_reach,pick,patience,chosen_cos,chosen_route,drive,scout_w,scout_home,dig_turned,trip_load,forage_max,since_trip,trip_src,emit_a,emit_b,emit_b_brain,usable,opts,cross,k,chose,reads_b,b_near,dig,dig_p,dig_x,dig_y,dig_mat,dig_flags,curvature,moisture_grad,heading_after,hx_after,hy_after,d_energy,d_crop_cells,d_crop_j,d_digesting,spoil_after,d_moves,d_blocked,d_bites,d_digs,d_deliveries",
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
    let mut diglog = dig.then(|| DigLog::new(&out, &lab.world, &names));
    // **Experiment dials, harness-only.** `mutation=<rate>` overrides the
    // ant's per-slot brain mutation rate (0 freezes the founders' brain only;
    // traits and body still mutate -- the game switch above freezes all);
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
                if let Some(d) = diglog.as_mut() {
                    d.found(&lab.world, &g);
                }
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
        let wanted_pre: HashSet<OrganismId> = pre.iter().map(|p| p.id).collect();
        let dig_pre = diglog.as_ref().map(|d| d.pre(&lab.world, g, &live)).unwrap_or_default();
        let deaths_before = lab.world.deaths_by_cause;
        // `census=1`: a cheap death record for every ant not being traced
        // (no brain probe), so a first pass can find who starved where and a
        // second `only=` pass can replay them. Same line shape as DIED.
        let census_pre: Vec<(OrganismId, Snap)> = if census {
            live.iter()
                .filter(|id| !wanted_pre.contains(id))
                .filter_map(|&id| snap(&lab.world, id).map(|s| (id, s)))
                .collect()
        } else {
            Vec::new()
        };

        if f < frames {
            lab.tick_for_harness();
        }
        for (id, before) in &census_pre {
            if lab.world.organism(*id).is_none() {
                let w = &lab.world;
                let causes: Vec<&str> = organism::DEATH_CAUSE_LIST
                    .iter()
                    .filter(|c| w.deaths_by_cause[c.index()] > deaths_before[c.index()])
                    .map(|c| c.label())
                    .collect();
                writeln!(
                    events,
                    "{f} DIED census=1 id={id} age={} at=({},{}) zone={} energy={:.0} crop_cells={} cause={}",
                    f.saturating_sub(born.get(id).copied().unwrap_or(0)),
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
            }
        }

        // After the tick: the decision rows and what changed.
        let rows: Vec<DecisionRow> = lab.world.decision_log.as_mut().map(std::mem::take).unwrap_or_default();
        if let Some(d) = diglog.as_mut() {
            d.after(&lab.world, g, f, &rows, &dig_pre, &born, f % colony_every == 0);
            if f % nest_every == 0 {
                d.nest_map(&lab.world, &format!("{out}/nest_f{f:06}.txt"));
            }
        }
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
                    ",1,{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
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
                    DIG_WHY_NAMES[r.dig as usize],
                    fl(r.dig_p),
                    if r.dig_at == DIG_NO_TARGET { String::new() } else { r.dig_at.0.to_string() },
                    if r.dig_at == DIG_NO_TARGET { String::new() } else { r.dig_at.1.to_string() },
                    if r.dig_at == DIG_NO_TARGET { String::new() } else { names[r.dig_mat as usize].clone() },
                    r.dig_flags,
                    fl(r.curvature),
                    fl(r.moisture_grad),
                    r.heading_after,
                    r.head_after.0,
                    r.head_after.1,
                )),
                None => s.push_str(",0"),
            }
            if !by_id.contains_key(&id) {
                // Pad the decision columns so every row has the same width.
                s.push_str(&",".repeat(48));
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
                "frame={f} ants={} brood={brood} traced={} rows={rows_written} cuts={} food_dropped={dropped} wall={:.0}s",
                live.len(),
                slots.iter().flatten().count(),
                diglog.as_ref().map_or(0, |d| d.cut_count),
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
    if let Some(d) = diglog {
        d.finish();
    }
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

/// One ant's state before the tick, for the digging record: what its jaws
/// and crop held when it decided, and what was round its head.
struct DigPre {
    /// 0 jaws and crop free, 1 food in the crop, 2 a pellet in the jaws --
    /// the two that `act` returns on before it reaches the dig.
    hold: u8,
    worker: bool,
    zone: &'static str,
    ahead: &'static str,
    /// Of the head's eight neighbours, how many are ground (`DigLog::is_ground`).
    ground8: u8,
    /// The face this digger is walking back to (`OrganismState::dig_return`,
    /// set by a cut inside the nest and cleared on arrival, on giving up, or
    /// once it holds food or is hungry), and the patience its home pull has
    /// left (`home_patience`). Added 2026-10-05 for *what pulls a digger off
    /// its face*: the row where `ret` goes from the cut cell to empty, read
    /// against what else changed there, says which of those it was.
    ret: Option<(i32, i32)>,
    patience: f32,
}

/// Where a brood item was seen (its cell) and its stage (`BroodStage as u8`,
/// 3 for a brood cell whose organism has no brood record).
type BroodSeen = ((i32, i32), u8);

/// **The digging deep dive's record** (`dig=1`), added 2026-10-04 for the
/// owner's *"deep dive on digging ... and why those who should dig are
/// not"*. Five outputs, none of which needs the brain probe `ants=` pays for:
///
/// - `digrows.csv.gz`: every decision of every ant, in a narrow row -- where
///   it was, what it held, the five senses on the dig's wires (with the
///   brain's fixed weights a reader can rebuild the urge term by term), and
///   how far the dig got (`creature::DigWhy`): not reached, lean, roll lost,
///   refused by the heap cue or the roof, nothing to cut, or cut. Last, the
///   face the ant is walking back to and its home pull's patience
///   (`ret_x`, `ret_y`, `patience`; see `DigPre::ret`).
/// - `cuts.csv`: every cut, with the ground round it -- open neighbours,
///   ground in the 5x5, whether it joined two spaces that were apart within
///   six cells (`local_joins`), the nearest brood, the nestmates within
///   three cells, and whether it is under a roof and in the dug home.
/// - `cells.csv.gz`: every cell in the nest region that turned from ground to
///   open or back, with the cut or the dropped pellet that did it where one
///   did. Anything left blank fell, slid or was lifted out of sight.
/// - `brood.csv`: every brood cell in the region every 1,000 frames.
/// - `broodlog.csv`: every brood item in the region, frame by frame: `laid`
///   (first seen as an egg, with the layer and where it stood), `moved`
///   (from and to: brood is a powder, so a fall shows as a run of one-cell
///   drops, and anything else is something carrying it), `stage` (a new
///   stage in place) and `gone` (with what stands on its last cell: empty
///   for a hatch, `corpse` for a starved larva). Added 2026-10-05 for the
///   owner's *"not sure why that has been so hard to solve"* about the
///   brood column under the door.
/// - `nest_fNNNNNN.txt`: the region as one character a cell every
///   `nestevery=` frames: `.` open and never dug, `o` open and dug, `#`
///   untouched ground, `=` packed wall, `s` spoil, `r` ground the jaw cannot
///   take, `~` water, `a` an adult, `e`/`l`/`p` egg, larva, pupa, `f` food.
///
/// "Ground" is any powder or solid cell owned by no organism and holding no
/// food, which is what a cut can take and a pellet becomes.
struct DigLog {
    rows: std::io::BufWriter<std::process::ChildStdin>,
    cells: std::io::BufWriter<std::process::ChildStdin>,
    cuts: std::io::BufWriter<std::fs::File>,
    brood: std::io::BufWriter<std::fs::File>,
    broodlog: std::io::BufWriter<std::fs::File>,
    /// Where each brood item in the region was last frame, and its stage.
    brood_at: HashMap<OrganismId, BroodSeen>,
    zips: Vec<std::process::Child>,
    /// x0, y0, width, height of the watched region, once the nest is founded.
    region: Option<(i32, i32, i32, i32)>,
    prev: Vec<bool>,
    prev_mat: Vec<u16>,
    ground: Vec<bool>,
    hard: Vec<bool>,
    names: Vec<String>,
    brood_mat: Option<material::MaterialId>,
    packed: Option<material::MaterialId>,
    spoil: Option<material::MaterialId>,
    cut_count: u64,
}

fn gzip_to(path: &str) -> (std::process::Child, std::io::BufWriter<std::process::ChildStdin>) {
    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("gzip -4 > '{path}'"))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("gzip");
    let w = std::io::BufWriter::with_capacity(1 << 20, child.stdin.take().expect("gzip stdin"));
    (child, w)
}

impl DigLog {
    fn new(out: &str, w: &World, names: &[String]) -> Self {
        let (z1, mut rows) = gzip_to(&format!("{out}/digrows.csv.gz"));
        let (z2, mut cells) = gzip_to(&format!("{out}/cells.csv.gz"));
        let mut cuts = std::io::BufWriter::new(std::fs::File::create(format!("{out}/cuts.csv")).unwrap());
        let mut brood = std::io::BufWriter::new(std::fs::File::create(format!("{out}/brood.csv")).unwrap());
        writeln!(
            rows,
            "frame,id,age,worker,hx,hy,zone,heading,hold,ahead,ground8,at_nest,crowding,curvature,food_adj,moisture_grad,energy,dig,dig_p,dig_flags,dig_x,dig_y,dig_mat,outcome,moved,hx_after,hy_after,ret_x,ret_y,patience"
        )
        .unwrap();
        writeln!(cells, "frame,x,y,from,to,cause,id").unwrap();
        writeln!(
            cuts,
            "frame,id,age,worker,hx,hy,x,y,zone,depth,mat,flags,dig_p,at_nest,crowding,open8,ground24,joins,brood_d,brood5,ants3,roofed,home"
        )
        .unwrap();
        writeln!(brood, "frame,x,y,id,stage").unwrap();
        let mut broodlog = std::io::BufWriter::new(std::fs::File::create(format!("{out}/broodlog.csv")).unwrap());
        writeln!(
            broodlog,
            "frame,id,event,x,y,stage,from_x,from_y,parent,parent_x,parent_y,parent_zone,parent_energy,cell_now"
        )
        .unwrap();
        let n = w.materials.len();
        let mut ground = vec![false; n];
        let mut hard = vec![false; n];
        for (i, g) in ground.iter_mut().enumerate() {
            let m = material::MaterialId(i as u16);
            *g = matches!(w.materials.kind(m), MaterialKind::Powder | MaterialKind::Solid)
                && creature::food_value(w, Cell::new(m, 0)) <= 0.0;
            hard[i] = *g && w.materials.get(m).penetration_resistance > 1.0;
        }
        DigLog {
            rows,
            cells,
            cuts,
            brood,
            broodlog,
            brood_at: HashMap::new(),
            zips: vec![z1, z2],
            region: None,
            prev: Vec::new(),
            prev_mat: Vec::new(),
            ground,
            hard,
            names: names.to_vec(),
            brood_mat: w.materials.id_of("brood"),
            packed: w.materials.id_of("packedsoil"),
            spoil: w.materials.id_of("spoil"),
            cut_count: 0,
        }
    }

    fn is_ground(&self, c: Cell) -> bool {
        c.organism_id() == 0 && self.ground[c.material.0 as usize]
    }

    /// Ground at a position; outside the world counts as ground.
    fn ground_at(&self, w: &World, x: i32, y: i32) -> bool {
        !w.in_bounds(x, y) || self.is_ground(w.get(x, y))
    }

    /// The region round the nest, fixed once it is founded, and its first
    /// snapshot.
    fn found(&mut self, w: &World, g: &Geo) {
        let (x0, y0) = (g.nest_x - 80, g.ground_y - 60);
        let (wd, ht) = (161, 141);
        self.region = Some((x0, y0, wd, ht));
        self.prev = vec![true; (wd * ht) as usize];
        self.prev_mat = vec![0; (wd * ht) as usize];
        for j in 0..ht {
            for i in 0..wd {
                let (x, y) = (x0 + i, y0 + j);
                let k = (j * wd + i) as usize;
                if w.in_bounds(x, y) {
                    let c = w.get(x, y);
                    self.prev[k] = self.is_ground(c);
                    self.prev_mat[k] = c.material.0;
                }
            }
        }
    }

    fn pre(&self, w: &World, g: &Geo, live: &[OrganismId]) -> HashMap<OrganismId, DigPre> {
        let mut m = HashMap::with_capacity(live.len());
        for &id in live {
            let Some(st) = w.organism(id) else { continue };
            let Some(&head) = st.chain.first() else { continue };
            let ground8 = (-1..=1)
                .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
                .filter(|&(dx, dy)| (dx, dy) != (0, 0) && self.ground_at(w, head.0 + dx, head.1 + dy))
                .count() as u8;
            m.insert(
                id,
                DigPre {
                    hold: if st.spoil.is_some() {
                        2
                    } else {
                        u8::from(st.crop.is_some())
                    },
                    worker: st.nest_bound_until == u64::MAX,
                    zone: zone(w, g, head),
                    ahead: ahead(w, head, st.heading),
                    ground8,
                    ret: st.dig_return,
                    patience: st.home_patience,
                },
            );
        }
        m
    }

    #[allow(clippy::too_many_arguments)]
    fn after(
        &mut self,
        w: &World,
        g: &Geo,
        f: u64,
        rows: &[DecisionRow],
        pre: &HashMap<OrganismId, DigPre>,
        born: &HashMap<OrganismId, u64>,
        brood_now: bool,
    ) {
        let mut cut_at: HashMap<(i32, i32), OrganismId> = HashMap::new();
        for r in rows {
            let Some(p) = pre.get(&r.id) else { continue };
            let age = f.saturating_sub(born.get(&r.id).copied().unwrap_or(0));
            let target = r.dig_at != DIG_NO_TARGET;
            writeln!(
                self.rows,
                "{f},{},{age},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                r.id,
                u8::from(p.worker),
                r.head.0,
                r.head.1,
                p.zone,
                r.heading,
                p.hold,
                p.ahead,
                p.ground8,
                fl(r.at_nest),
                fl(r.crowding),
                fl(r.curvature),
                fl(r.food_adjacent),
                fl(r.moisture_grad),
                fl(r.energy),
                DIG_WHY_NAMES[r.dig as usize],
                fl(r.dig_p),
                r.dig_flags,
                if target { r.dig_at.0.to_string() } else { String::new() },
                if target { r.dig_at.1.to_string() } else { String::new() },
                if target {
                    self.names[r.dig_mat as usize].as_str()
                } else {
                    ""
                },
                DECISION_OUTCOME_NAMES[r.outcome as usize],
                u8::from(r.moved),
                r.head_after.0,
                r.head_after.1,
                p.ret.map_or(String::new(), |c| c.0.to_string()),
                p.ret.map_or(String::new(), |c| c.1.to_string()),
                fl(p.patience),
            )
            .unwrap();
            if r.dig == DigWhy::Cut {
                cut_at.insert(r.dig_at, r.id);
                self.cut_row(w, g, f, r, p, age);
            }
        }
        // Pellets put down this tick: an ant that held one before and does not now.
        let mut drops: Vec<(OrganismId, (i32, i32))> = pre
            .iter()
            .filter(|(_, p)| p.hold == 2)
            .filter_map(|(&id, _)| {
                let st = w.organism(id)?;
                st.spoil
                    .is_none()
                    .then(|| (id, st.chain.first().copied().unwrap_or((i32::MIN, i32::MIN))))
            })
            .collect();
        drops.sort_unstable();
        self.scan(w, g, f, &cut_at, &drops, brood_now);
    }

    fn cut_row(&mut self, w: &World, g: &Geo, f: u64, r: &DecisionRow, p: &DigPre, age: u64) {
        self.cut_count += 1;
        let (cx, cy) = r.dig_at;
        let open = |x: i32, y: i32| !self.ground_at(w, x, y);
        let mut open8 = 0;
        let mut ground24 = 0;
        for dy in -2..=2 {
            for dx in -2..=2 {
                if (dx, dy) == (0, 0) {
                    continue;
                }
                let o = open(cx + dx, cy + dy);
                if dx.abs() <= 1 && dy.abs() <= 1 && o {
                    open8 += 1;
                }
                if !o {
                    ground24 += 1;
                }
            }
        }
        let joins = local_joins(&open, (cx, cy));
        let (mut brood_d, mut brood5) = (-1i32, 0u32);
        let mut ants = HashSet::new();
        for dy in -15..=15i32 {
            for dx in -15..=15i32 {
                let (x, y) = (cx + dx, cy + dy);
                if !w.in_bounds(x, y) {
                    continue;
                }
                let c = w.get(x, y);
                let d = dx.abs().max(dy.abs());
                if c.organism_id() != 0 && Some(c.material) == self.brood_mat {
                    if brood_d < 0 || d < brood_d {
                        brood_d = d;
                    }
                    if d <= 5 {
                        brood5 += 1;
                    }
                } else if d <= 3
                    && c.organism_id() != 0
                    && c.organism_id() != r.id
                    && w.materials.kind(c.material) == MaterialKind::Creature
                {
                    ants.insert(c.organism_id());
                }
            }
        }
        let roofed = (1..=30).any(|k| w.in_bounds(cx, cy - k) && self.is_ground(w.get(cx, cy - k)));
        let home = (-1..=1).any(|dy| (-1..=1).any(|dx| w.nest_dug.contains(&(cx + dx, cy + dy))));
        writeln!(
            self.cuts,
            "{f},{},{age},{},{},{},{cx},{cy},{},{},{},{},{},{},{},{open8},{ground24},{joins},{},{brood5},{},{},{}",
            r.id,
            u8::from(p.worker),
            r.head.0,
            r.head.1,
            zone(w, g, (cx, cy)),
            cy - g.ground_y,
            self.names[r.dig_mat as usize],
            r.dig_flags,
            fl(r.dig_p),
            fl(r.at_nest),
            fl(r.crowding),
            if brood_d < 0 {
                String::new()
            } else {
                brood_d.to_string()
            },
            ants.len(),
            u8::from(roofed),
            u8::from(home),
        )
        .unwrap();
    }

    /// The region, cell by cell: every ground/open flip since last frame, and
    /// the brood when `brood_now`.
    fn scan(
        &mut self,
        w: &World,
        g: &Geo,
        f: u64,
        cut_at: &HashMap<(i32, i32), OrganismId>,
        drops: &[(OrganismId, (i32, i32))],
        brood_now: bool,
    ) {
        let Some((x0, y0, wd, ht)) = self.region else { return };
        let cheb = |a: (i32, i32), b: (i32, i32)| (a.0 - b.0).abs().max((a.1 - b.1).abs());
        let mut brood_here: HashMap<OrganismId, BroodSeen> = HashMap::new();
        for j in 0..ht {
            for i in 0..wd {
                let (x, y) = (x0 + i, y0 + j);
                if !w.in_bounds(x, y) {
                    continue;
                }
                let k = (j * wd + i) as usize;
                let c = w.get(x, y);
                let gr = self.is_ground(c);
                if gr != self.prev[k] {
                    let (cause, id) = if !gr {
                        cut_at.get(&(x, y)).map_or(("", 0), |&id| ("cut", id))
                    } else if let Some(&(id, _)) = drops
                        .iter()
                        .filter(|(_, h)| cheb(*h, (x, y)) <= 3)
                        .min_by_key(|(id, h)| (cheb(*h, (x, y)), *id))
                    {
                        ("drop", id)
                    } else if let Some(&(id, _)) = drops.iter().find(|(_, h)| h.0 == x && y < h.1) {
                        ("lift", id)
                    } else {
                        ("", 0)
                    };
                    writeln!(
                        self.cells,
                        "{f},{x},{y},{},{},{cause},{}",
                        self.names[self.prev_mat[k] as usize],
                        self.names[c.material.0 as usize],
                        if id == 0 { String::new() } else { id.to_string() }
                    )
                    .unwrap();
                }
                self.prev[k] = gr;
                self.prev_mat[k] = c.material.0;
                if c.organism_id() != 0 && Some(c.material) == self.brood_mat {
                    let stage = w
                        .organism(c.organism_id())
                        .and_then(|s| s.brood)
                        .map_or(3, |b| b.stage as u8);
                    brood_here.insert(c.organism_id(), ((x, y), stage));
                    if brood_now {
                        writeln!(self.brood, "{f},{x},{y},{},{}", c.organism_id(), stage_name(stage)).unwrap();
                    }
                }
            }
        }
        // The brood frame by frame, against last frame: new, moved, a new
        // stage, gone. Sorted, so the file is the same run to run.
        let mut ids: Vec<OrganismId> = brood_here.keys().copied().collect();
        ids.sort_unstable();
        for id in ids {
            let ((x, y), stage) = brood_here[&id];
            match self.brood_at.get(&id).copied() {
                None => {
                    let parent = w.organism(id).and_then(|s| s.brood).map(|b| b.parent);
                    let layer = parent
                        .and_then(|p| w.organism(p))
                        .and_then(|s| s.chain.first().copied().map(|h| (h, s.energy)));
                    let (px, py, pz, pe) = match layer {
                        Some((h, e)) => (h.0.to_string(), h.1.to_string(), zone(w, g, h), format!("{e:.0}")),
                        None => (String::new(), String::new(), "", String::new()),
                    };
                    writeln!(
                        self.broodlog,
                        "{f},{id},{},{x},{y},{},,,{},{px},{py},{pz},{pe},",
                        if stage == BroodStage::Egg as u8 { "laid" } else { "seen" },
                        stage_name(stage),
                        parent.map_or(String::new(), |p| p.to_string()),
                    )
                    .unwrap();
                }
                Some((at, was)) => {
                    if at != (x, y) {
                        writeln!(
                            self.broodlog,
                            "{f},{id},moved,{x},{y},{},{},{},,,,,,",
                            stage_name(stage),
                            at.0,
                            at.1
                        )
                        .unwrap();
                    }
                    if was != stage {
                        writeln!(self.broodlog, "{f},{id},stage,{x},{y},{},,,,,,,,", stage_name(stage)).unwrap();
                    }
                }
            }
        }
        let mut gone: Vec<(OrganismId, BroodSeen)> = self
            .brood_at
            .iter()
            .filter(|(id, _)| !brood_here.contains_key(id))
            .map(|(&id, &v)| (id, v))
            .collect();
        gone.sort_unstable_by_key(|&(id, _)| id);
        for (id, ((x, y), stage)) in gone {
            let now = if w.in_bounds(x, y) {
                self.names[w.get(x, y).material.0 as usize].as_str()
            } else {
                ""
            };
            writeln!(
                self.broodlog,
                "{f},{id},gone,{x},{y},{},,,,,,,,{now}",
                stage_name(stage)
            )
            .unwrap();
        }
        self.brood_at = brood_here;
    }

    fn nest_map(&self, w: &World, path: &str) {
        let Some((x0, y0, wd, ht)) = self.region else { return };
        let mut s = format!("{x0} {y0} {wd} {ht}\n");
        for y in y0..y0 + ht {
            for x in x0..x0 + wd {
                let ch = if !w.in_bounds(x, y) {
                    'r'
                } else {
                    let c = w.get(x, y);
                    let kind = w.materials.kind(c.material);
                    if c.material == material::EMPTY {
                        if w.dug_cells.contains(&(x, y)) {
                            'o'
                        } else {
                            '.'
                        }
                    } else if c.organism_id() != 0 && Some(c.material) == self.brood_mat {
                        match w.organism(c.organism_id()).and_then(|s| s.brood).map(|b| b.stage) {
                            Some(BroodStage::Egg) => 'e',
                            Some(BroodStage::Larva) => 'l',
                            Some(BroodStage::Pupa) => 'p',
                            None => 'b',
                        }
                    } else if kind == MaterialKind::Creature {
                        'a'
                    } else if kind == MaterialKind::Liquid {
                        '~'
                    } else if self.is_ground(c) {
                        if self.hard[c.material.0 as usize] {
                            'r'
                        } else if Some(c.material) == self.spoil {
                            's'
                        } else if Some(c.material) == self.packed {
                            '='
                        } else {
                            '#'
                        }
                    } else if c.organism_id() == 0 && creature::food_value(w, c) > 0.0 {
                        'f'
                    } else {
                        '?'
                    }
                };
                s.push(ch);
            }
            s.push('\n');
        }
        let _ = std::fs::write(path, s);
    }

    fn finish(mut self) {
        self.rows.flush().unwrap();
        self.cells.flush().unwrap();
        self.cuts.flush().unwrap();
        self.brood.flush().unwrap();
        self.broodlog.flush().unwrap();
        drop(self.rows);
        drop(self.cells);
        for mut z in self.zips {
            let _ = z.wait();
        }
    }
}

/// A brood stage by its `BroodStage` number, as `brood.csv` names it.
fn stage_name(stage: u8) -> &'static str {
    match stage {
        0 => "egg",
        1 => "larva",
        2 => "pupa",
        _ => "?",
    }
}

/// **Did this cut join two spaces?** The open 4-neighbours of `(cx, cy)`,
/// grouped by whether a 4-connected walk through open cells joins them within
/// six cells of the cut with the cut cell itself still ground: 1 is a cut
/// into one space's wall, 2 or more a cut through a wall between spaces that
/// were apart there -- two rooms, or a room and a tunnel, meeting. 0 is a cut
/// with no open side (reached diagonally). 4-connected, as `step_nest_dug`
/// walks the home.
fn local_joins(open: &impl Fn(i32, i32) -> bool, (cx, cy): (i32, i32)) -> u8 {
    const R: i32 = 6;
    let n = (2 * R + 1) as usize;
    let idx = |x: i32, y: i32| ((y - cy + R) as usize) * n + (x - cx + R) as usize;
    let mut label = vec![0u8; n * n];
    let mut seen: Vec<u8> = Vec::new();
    let mut next = 0u8;
    for (sx, sy) in [(cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)] {
        if !open(sx, sy) {
            continue;
        }
        let l = label[idx(sx, sy)];
        if l != 0 {
            if !seen.contains(&l) {
                seen.push(l);
            }
            continue;
        }
        next += 1;
        label[idx(sx, sy)] = next;
        let mut stack = vec![(sx, sy)];
        while let Some((x, y)) = stack.pop() {
            for (nx, ny) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                if (nx - cx).abs() > R || (ny - cy).abs() > R || (nx, ny) == (cx, cy) {
                    continue;
                }
                let j = idx(nx, ny);
                if label[j] == 0 && open(nx, ny) {
                    label[j] = next;
                    stack.push((nx, ny));
                }
            }
        }
        seen.push(next);
    }
    seen.len() as u8
}
