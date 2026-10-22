//! **Can an ant feel which cut drops a roof, without being told?** — the
//! measurement behind `Reports/soil-physics-realism-2026-10-21.md` §3.
//!
//! The trained ant is warned about collapses by `f::ROOF_DROP`, which runs
//! the engine's own collapse rule one cut ahead (`creature::cut_drops_roof`).
//! The owner holds the ant to local signals and true biology, so the
//! question is whether a **present-tense touch reading** -- something a body
//! standing at the face could feel now, with no world built in its head --
//! carries the same warning.
//!
//! **Ground truth is the engine itself, not the forecast.** A colony of 55
//! ants digs a soil bank (`burrow_probe`'s colony scene). At each snapshot,
//! every cell an ant could cut (soil, packed soil or spoil touching open
//! space) is cut in a clone of the world the way `cut_cell` cuts -- the cell
//! emptied, its neighbours lined, a pellet left a pellet -- and the clone is
//! stepped `settle=` frames beside an uncut twin. A **drop** is a ground cell
//! within two of the cut that is empty in the cut world and not in the twin:
//! something fell that would not have fallen otherwise.
//!
//! Each cue is then scored against that truth on every candidate cut:
//! `flagged` (how often it fires), `recall` (of real drops, how many it
//! warned of) and `precision` (of its warnings, how many were real).
//!
//! | cue | what the ant would feel |
//! |---|---|
//! | `hold<=k` | a grain touching the bite rests on it or on air, and has at most `k` other grains holding it |
//! | `resting` | a grain touching the bite rests on it or on air (no count) |
//! | `pellet` | a spoil pellet rests on the bite |
//! | `thin` | the ground straight above the bite is one cell thick |
//!
//! `hold<=2` is the engine's crumb rule read in the present tense: it should
//! agree with the truth almost everywhere, and **where it does not is the
//! interesting part** (the doorway left loose, a nestmate holding a grain up,
//! a fall that sets off another). It is also the positive control -- if it
//! does not score near 1.0 the truth column is measuring something else.
//!
//! ```text
//! cargo run --release --example roof_cues
//! cargo run --release --example roof_cues -- seeds=4 snaps=1500,3000,4500
//! ```

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::weather::Pin;
use pixel_physics::sim::{frame, player, Cell, World};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses"))
    })
}

const N8: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// Open to a falling grain: empty, or an animal (`cut_drops_roof` reads a
/// body as open -- an animal is not a roof's support).
fn open(w: &World, x: i32, y: i32) -> bool {
    let c = w.get(x, y);
    c.material == material::EMPTY || w.materials.kind(c.material) == MaterialKind::Creature
}

fn ground(w: &World, x: i32, y: i32) -> bool {
    let c = w.get(x, y);
    c.material != material::EMPTY
        && matches!(w.materials.kind(c.material), MaterialKind::Powder | MaterialKind::Solid)
        && c.organism_id() == 0
}

/// `cut_cell`'s effect on the ground, for the default switches: the cell
/// emptied, each neighbour with a packed form packed, a pellet
/// (`needs_footing`) left a pellet. The doorway exception (`door_loose_of`)
/// is not reproduced; its cells show up as disagreements, which is a
/// finding rather than an error.
fn cut(w: &mut World, (tx, ty): (i32, i32)) {
    w.set(tx, ty, Cell::EMPTY);
    for (dx, dy) in N8 {
        let (x, y) = (tx + dx, ty + dy);
        let c = w.get(x, y);
        if c.organism_id() != 0 {
            continue;
        }
        let m = w.materials.get(c.material);
        if m.needs_footing {
            continue;
        }
        if let Some(p) = m.packs_into {
            let mut packed = c;
            packed.material = p;
            w.set(x, y, packed);
        }
    }
}

struct Cues {
    hold: [bool; 6],
    /// Fewest other holds on any grain resting on the bite or on air; 9
    /// when nothing rests.
    hold_min: usize,
    resting: bool,
    pellet: bool,
    thin: bool,
}

/// Every reading from the cells within two of `t`, as they stand now.
fn cues(w: &World, t: (i32, i32)) -> Cues {
    let mut hold = [false; 6];
    let mut hold_min = 9usize;
    let mut resting = false;
    let mut pellet = false;
    for (dx, dy) in N8 {
        let n = (t.0 + dx, t.1 + dy);
        if !ground(w, n.0, n.1) {
            continue;
        }
        let m = w.materials.get(w.get(n.0, n.1).material);
        // Only ground that can fall: a powder (stone is held by load.rs).
        if m.kind != MaterialKind::Powder {
            continue;
        }
        let below = (n.0, n.1 + 1);
        let rests = below == t || open(w, below.0, below.1);
        if !rests {
            continue;
        }
        resting = true;
        if m.needs_footing && below == t {
            pellet = true;
        }
        let others = N8
            .iter()
            .filter(|&&(ex, ey)| {
                let c = (n.0 + ex, n.1 + ey);
                c != t && !open(w, c.0, c.1)
            })
            .count();
        hold_min = hold_min.min(others);
        for (k, h) in hold.iter_mut().enumerate() {
            if others <= k {
                *h = true;
            }
        }
    }
    let thin = ground(w, t.0, t.1 - 1) && open(w, t.0, t.1 - 2);
    Cues { hold, hold_min, resting, pellet, thin }
}

fn truth_world(world: &World, t: (i32, i32), settle: u64, tuning: &player::Tuning) -> World {
    let mut a = world.clone();
    let (mut ap, mut ab) = (ParticleSystem::default(), Blasts::default());
    cut(&mut a, t);
    for _ in 0..settle {
        frame::step(&mut a, &mut ap, &mut ab, player::PlayerInput::default(), tuning);
    }
    a
}

/// Cut `t` in a clone the way an ant does, step it `settle` frames, and
/// count ground cells within two of the cut that are empty there but not in
/// the uncut `twin`.
fn truth_of(world: &World, twin: &World, t: (i32, i32), settle: u64, tuning: &player::Tuning) -> usize {
    let mut a = world.clone();
    // Particles are not cloneable and carry nothing that holds ground up.
    let (mut ap, mut ab) = (ParticleSystem::default(), Blasts::default());
    cut(&mut a, t);
    for _ in 0..settle {
        frame::step(&mut a, &mut ap, &mut ab, player::PlayerInput::default(), tuning);
    }
    let mut fell = 0;
    for dy in -2..=2 {
        for dx in -2..=2 {
            let p = (t.0 + dx, t.1 + dy);
            if p == t || !ground(world, p.0, p.1) {
                continue;
            }
            let gone_a = a.get(p.0, p.1).material == material::EMPTY;
            let gone_b = twin.get(p.0, p.1).material == material::EMPTY;
            if gone_a && !gone_b {
                fell += 1;
            }
        }
    }
    fell
}

#[derive(Default, Clone, Copy)]
struct Score {
    flagged: usize,
    hit: usize,
}

/// **The brain lane's own worlds**: the soil arena (`challenges::Soil`, step
/// 4, 8-row shaft, foot pay, 1,200 J, 12,000 frames) with a trained genome in
/// the ant, on the sealed and the hold-out layouts. Every `every` frames each
/// ant's Dig options are written with all 45 features, so the label the
/// probe predicts is the ant's own `f::ROOF_DROP` exactly as its brain saw
/// it; one frame in `truth_every` also carries the engine's verdict.
fn arena(genome_path: &str, csv: &str) {
    use pixel_physics::sim::creature::options::{self, Verb};
    use pixel_physics::train::challenges::Soil;
    use pixel_physics::train::{holdout_layouts, load_genome, sealed_layouts, Challenge};
    let every: u64 = arg("every").unwrap_or(5);
    let truth_every: u64 = arg("truth_every").unwrap_or(50);
    let settle: u64 = arg("settle").unwrap_or(3);
    let n_sealed: usize = arg("sealed").unwrap_or(24);
    let n_hold: usize = arg("holdout").unwrap_or(24);
    let frames: u64 = arg("frames").unwrap_or(12_000);
    let head = std::fs::read_to_string(genome_path).expect("genome file");
    let len: usize = head
        .lines()
        .next()
        .and_then(|l| l.strip_prefix("genome "))
        .and_then(|l| l.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .expect("a sparse genome file: `genome <len> ...` on its first line");
    let genome = load_genome(std::path::Path::new(genome_path), len);
    let soil = Soil { ants: 1, frames, step: 4, shaft: 8, budget: Some(1200.0), foot: true, step_near: 0.0, nest: false };
    println!(
        "roof_cues arena: genome {genome_path} ({len} slots), soil step 4 shaft 8 foot budget 1200, {frames} frames, \
         {n_sealed} sealed + {n_hold} hold-out layouts, every {every}, truth every {truth_every}"
    );
    let names = options::feature_names();
    let mut out = vec!["world".to_string(), "frame".into(), "ant".into(), "tx".into(), "ty".into()];
    out.extend(names.iter().map(|n| format!("f_{n}")));
    out.extend(["truth", "fell", "hold_min", "resting", "pellet", "thin"].map(String::from));
    let mut body = out.join(",") + "\n";
    let tuning = player::Tuning::default();
    let mut layouts: Vec<(String, pixel_physics::train::Layout)> =
        sealed_layouts(n_sealed).into_iter().map(|l| (format!("sealed{}", l.k), l)).collect();
    layouts.extend(holdout_layouts(n_hold).into_iter().map(|l| (format!("hold{}", l.k), l)));
    let (mut rows, mut flagged, mut truths) = (0usize, 0usize, 0usize);
    for (wi, (name, layout)) in layouts.iter().enumerate() {
        let mut opts = Vec::new();
        let (mut r_here, mut f_here) = (0usize, 0usize);
        let mut look = |f: u64, w: &World| {
            if !f.is_multiple_of(every) {
                return;
            }
            let with_truth = f.is_multiple_of(truth_every);
            let twin = with_truth.then(|| {
                let mut t = w.clone();
                let (mut tp, mut tb) = (ParticleSystem::default(), Blasts::default());
                for _ in 0..settle {
                    frame::step(&mut t, &mut tp, &mut tb, player::PlayerInput::default(), &tuning);
                }
                t
            });
            for id in w.live_organism_ids() {
                let Some(st) = w.organism(id) else { continue };
                let Some(def) = w.species.get(st.species).creature.clone() else { continue };
                options::build(w, id, &def, &mut opts);
                for o in opts.iter().filter(|o| o.verb == Verb::Dig) {
                    let q = cues(w, o.target);
                    let (truth, fell) = match &twin {
                        Some(t) => {
                            let n = truth_of(w, t, o.target, settle, &tuning);
                            (u8::from(n > 0).to_string(), n.to_string())
                        }
                        None => ("".into(), "".into()),
                    };
                    let mut r = vec![wi.to_string(), f.to_string(), id.to_string(), o.target.0.to_string(), o.target.1.to_string()];
                    r.extend(o.feats.iter().map(|v| format!("{v:.4}")));
                    r.extend([truth, fell, q.hold_min.to_string(), u8::from(q.resting).to_string(), u8::from(q.pellet).to_string(), u8::from(q.thin).to_string()]);
                    body.push_str(&r.join(","));
                    body.push('\n');
                    r_here += 1;
                    if o.feats[options::f::ROOF_DROP] > 0.5 {
                        f_here += 1;
                    }
                    if with_truth {
                        truths += 1;
                    }
                }
            }
        };
        soil.run(*layout, &[&genome], &mut look);
        println!("  {name}: {r_here} Dig options, {f_here} flagged ROOF_DROP");
        rows += r_here;
        flagged += f_here;
    }
    std::fs::write(csv, body).expect("writing the csv");
    println!("wrote {csv}: {rows} Dig-option rows, {flagged} flagged, {truths} with the engine's verdict");
}

fn main() {
    if let Some(g) = arg::<String>("genome") {
        arena(&g, &arg::<String>("csv").expect("arena mode wants csv=<file>"));
        return;
    }
    let seeds: u64 = arg("seeds").unwrap_or(3);
    let ants: i32 = arg("ants").unwrap_or(55);
    let settle: u64 = arg("settle").unwrap_or(3);
    let snaps: Vec<u64> = arg::<String>("snaps")
        .unwrap_or_else(|| "1500,3000,4500".into())
        .split(',')
        .map(|s| s.trim().parse().expect("snaps=a,b,c"))
        .collect();
    let csv: Option<String> = arg("csv");
    println!("roof_cues: seeds={seeds} ants={ants} settle={settle} snaps={snaps:?} csv={csv:?}");
    // One row per Dig option an ant is actually offered: the ant's own 45
    // option features (`options::build`, the brain's eyes, ROOF_DROP last),
    // then the engine's verdict and the touch cues. The probe in
    // `scripts/roof_probe.py` reads it.
    let mut rows: Vec<String> = Vec::new();
    let header = {
        let names = pixel_physics::sim::creature::options::feature_names();
        let mut h = vec!["world".to_string(), "frame".into(), "ant".into(), "tx".into(), "ty".into()];
        h.extend(names.iter().map(|n| format!("f_{n}")));
        h.extend(["truth", "fell", "hold_min", "resting", "pellet", "thin"].map(String::from));
        h.join(",")
    };

    let names = ["hold<=0", "hold<=1", "hold<=2", "hold<=3", "hold<=4", "hold<=5", "resting", "pellet", "thin"];
    let mut score = vec![Score::default(); names.len()];
    let (mut candidates, mut drops, mut multi) = (0usize, 0usize, 0usize);
    // Truth cases `hold<=2` misses and false alarms it raises, by kind.
    let mut dbg_left = 12usize;
    let mut miss_note: std::collections::BTreeMap<String, usize> = Default::default();

    for seed in 1..=seeds {
        let (wd, ht) = (200i32, 120i32);
        let mut world = World::new(Rect::new(0, 0, wd - 1, ht - 1));
        world.seed = seed;
        world.set_weather_pin(Pin::Clear);
        world.set_sky_hold(Some(pixel_physics::sky::frame_for_daylight(1.0)));
        let soil = world.materials.id_of("soil").expect("soil");
        let nest = world.materials.id_of("nest").expect("nest");
        let floor = ht - 8;
        let (bx0, bx1) = (40i32, 160i32);
        for x in 0..wd {
            for y in floor..ht {
                world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        for x in bx0..bx1 {
            for y in (floor - 30)..floor {
                world.set(x, y, Cell::new(soil, 0).with_attached(true));
            }
        }
        for x in 16..bx0 {
            world.set(x, floor, Cell::new(nest, 0).with_attached(true));
        }
        {
            use pixel_physics::sim::rng;
            let mut slots: Vec<(i32, i32)> =
                (16..bx0).step_by(2).flat_map(|x| (1..=6).map(move |r| (x, floor - r))).collect();
            let mut draw = rng::stream(seed, 0xB0_1707, 0, 0);
            for i in (1..slots.len()).rev() {
                slots.swap(i, draw.below(i as u32 + 1) as usize);
            }
            let mut colony = None;
            for &(x, y) in slots.iter().take(ants as usize) {
                if let Some(site) =
                    pixel_physics::sim::creature::plant_creature_seed_in(&mut world, x, y, "ant", colony)
                {
                    colony = colony.or_else(|| pixel_physics::sim::creature::colony_of_site(&world, &site));
                    world.schedule_active_site(site);
                }
            }
        }

        let mut particles = ParticleSystem::default();
        let mut blasts = Blasts::default();
        let tuning = player::Tuning::default();
        let mut f = 0u64;
        for &snap in &snaps {
            while f < snap {
                frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
                f += 1;
            }
            // The uncut twin, stepped once per snapshot: every candidate is
            // compared against the same control.
            let mut twin = world.clone();
            let (mut tp, mut tb) = (ParticleSystem::default(), Blasts::default());
            for _ in 0..settle {
                frame::step(&mut twin, &mut tp, &mut tb, player::PlayerInput::default(), &tuning);
            }
            let cuttable = ["soil", "packedsoil", "spoil"]
                .map(|n| world.materials.id_of(n).expect("compiled in"));
            let (mut n_here, mut d_here) = (0usize, 0usize);
            for y in 1..ht - 1 {
                for x in 1..wd - 1 {
                    let c = world.get(x, y);
                    if c.organism_id() != 0 || !cuttable.contains(&c.material) {
                        continue;
                    }
                    if !N8.iter().any(|&(dx, dy)| open(&world, x + dx, y + dy)) {
                        continue;
                    }
                    let q = cues(&world, (x, y));
                    let fell = truth_of(&world, &twin, (x, y), settle, &tuning);
                    let truth = fell > 0;
                    candidates += 1;
                    n_here += 1;
                    if truth {
                        drops += 1;
                        d_here += 1;
                        if fell > 1 {
                            multi += 1;
                        }
                    }
                    let flags = [q.hold[0], q.hold[1], q.hold[2], q.hold[3], q.hold[4], q.hold[5], q.resting, q.pellet, q.thin];
                    for (s, &fl) in score.iter_mut().zip(flags.iter()) {
                        if fl {
                            s.flagged += 1;
                            if truth {
                                s.hit += 1;
                            }
                        }
                    }
                    if truth != q.hold[2] && std::env::var("ROOF_DEBUG").is_ok() && dbg_left > 0 {
                        dbg_left -= 1;
                        println!("  {} at ({x},{y}) fell {fell} hold_min {}", if truth { "MISS" } else { "FALSE" }, q.hold_min);
                        let mut after = world.clone();
                        cut(&mut after, (x, y));
                        for yy in y - 3..=y + 3 {
                            let row = |w: &World| (x - 3..=x + 3).map(|xx| {
                                let c = w.get(xx, yy);
                                if (xx, yy) == (x, y) && std::ptr::eq(w, &world) { return '*'; }
                                if c.material == material::EMPTY { '.' } else if w.materials.kind(c.material) == MaterialKind::Creature { 'a' } else {
                                    match w.materials.get(c.material).name.as_str() { "soil" => 's', "packedsoil" => 'P', "spoil" => 'o', "stone" => '#', _ => '?' } }
                            }).collect::<String>();
                            println!("    {}   {}   {}", row(&world), row(&twin), row(&truth_world(&world, (x, y), settle, &tuning)));
                        }
                    }
                    if truth != q.hold[2] {
                        let below = world.get(x, y + 1);
                        let kind = format!(
                            "{} {} (cut {}, below {})",
                            if truth { "missed" } else { "false alarm" },
                            if q.pellet { "pellet" } else if q.resting { "resting" } else { "nothing resting" },
                            world.materials.get(c.material).name,
                            if open(&world, x, y + 1) { "open".to_string() } else { world.materials.get(below.material).name.clone() }
                        );
                        *miss_note.entry(kind).or_default() += 1;
                    }
                }
            }
            let mut offered = 0usize;
            if csv.is_some() {
                use pixel_physics::sim::creature::options::{self, Verb};
                let mut opts = Vec::new();
                let mut seen: std::collections::HashMap<(i32, i32), usize> = Default::default();
                for id in world.live_organism_ids() {
                    let Some(st) = world.organism(id) else { continue };
                    let Some(def) = world.species.get(st.species).creature.clone() else { continue };
                    options::build(&world, id, &def, &mut opts);
                    for o in opts.iter().filter(|o| o.verb == Verb::Dig) {
                        let fell = *seen
                            .entry(o.target)
                            .or_insert_with(|| truth_of(&world, &twin, o.target, settle, &tuning));
                        let q = cues(&world, o.target);
                        let mut r = vec![seed.to_string(), snap.to_string(), id.to_string(), o.target.0.to_string(), o.target.1.to_string()];
                        r.extend(o.feats.iter().map(|v| format!("{v:.4}")));
                        r.extend([
                            u8::from(fell > 0).to_string(),
                            fell.to_string(),
                            q.hold_min.to_string(),
                            u8::from(q.resting).to_string(),
                            u8::from(q.pellet).to_string(),
                            u8::from(q.thin).to_string(),
                        ]);
                        rows.push(r.join(","));
                        offered += 1;
                    }
                }
            }
            println!("  seed {seed} frame {snap}: {n_here} candidate cuts, {d_here} drop something; {offered} Dig options offered to ants");
        }
    }

    println!(
        "\n{candidates} candidate cuts, {drops} drop ground ({:.2}%), {multi} of those drop more than one cell\n",
        100.0 * drops as f64 / candidates.max(1) as f64
    );
    println!("{:<10} {:>8} {:>8} {:>8} {:>10}", "cue", "flagged", "recall", "precis.", "flag rate");
    for (n, s) in names.iter().zip(&score) {
        println!(
            "{:<10} {:>8} {:>8.3} {:>8.3} {:>9.2}%",
            n,
            s.flagged,
            s.hit as f64 / drops.max(1) as f64,
            if s.flagged > 0 { s.hit as f64 / s.flagged as f64 } else { 0.0 },
            100.0 * s.flagged as f64 / candidates.max(1) as f64
        );
    }
    println!("\nwhere `hold<=2` and the truth disagree:");
    for (k, v) in miss_note {
        println!("  {v:>6}  {k}");
    }
    if let Some(path) = csv {
        let mut body = header;
        body.push('\n');
        for r in &rows {
            body.push_str(r);
            body.push('\n');
        }
        std::fs::write(&path, body).expect("writing the csv");
        println!("\nwrote {path}: {} Dig-option rows", rows.len());
    }
}
