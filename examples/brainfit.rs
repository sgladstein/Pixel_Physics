//! **Throwaway feasibility probe: can a brain be *trained* here by challenge
//! and selection, and what does one generation cost?** Built for the
//! brain-training thread (2026-10-17); not an instrument of record.
//!
//! The challenge: a fixed colony of clones (every ant carries the genome
//! under test) on a small bed, one heap of provisions `dist=` columns to the
//! east or the west of the nest (the side alternates with the replicate, so
//! "always walk east" scores half). Score = provisions cells gone from the
//! heap at the end (eaten or carried off). Selection: keep the best quarter,
//! refill by mutating them, every generation on fresh paired seeds; the
//! starting genome is scored beside them on the same seeds as the control.
//!
//! ```text
//! cargo run --release --example brainfit -- gens=20 pop=24 seeds=2 frames=6000 start=founder chooser=off
//! cargo run --release --example brainfit -- start=zero ...     # every live brain weight 0: from scratch
//! cargo run --release --example brainfit -- cost=1             # time one evaluation and stop
//! ```
//!
//! Serial `update::step` per world, one world per thread: the parallel
//! driver's rayon pool would serialise four callers. Mutation is off inside
//! the world (births are also out of reach); the only variation is the one
//! this loop makes between evaluations.

use pixel_physics::lab::scenario::Scenario;
use pixel_physics::sim::{brain, creature, update};
use std::time::Instant;

#[derive(Clone, Copy)]
struct Cfg {
    frames: u64,
    ants: i32,
    dist: i32,
    chooser_off: bool,
    heap: i32,
    /// `home`: provisions cells within 12 columns of the nest at the end, plus
    /// crop food digested at the nest in cell-equivalents (960 energy a cell).
    /// `taken`: cells gone from the heap's window, wherever they went.
    fit_home: bool,
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    fn gauss(&mut self) -> f32 {
        let (u1, u2) = (self.unit().max(1e-7), self.unit());
        (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos()
    }
}

const NEST_X: i32 = 128;

fn scenario(seed: u64, heap_x: i32, ants: i32, heap: i32) -> Scenario {
    let text = format!(
        r#"(
    name: "brainfit", title: "BRAINFIT", question: "training probe",
    bed: (width: 256, height: 128, soil_depth: 40, ground_y: 64, compartments: 1, founders: 0,
          species: "grass", colonies: 0, colony_ants: {ants}, colony_species: "ant", predators: 0,
          lamp_spacing: 33, seed: {seed}, extra_walls: []),
    settings: [(subject: "the bed", field: "rain", value: 0)],
    placements: [
        Colony(species: "ant", x: {NEST_X}, count: {ants}),
        Heap(material: "provisions", x: {heap_x}, cells: {heap}),
    ],
    timeline: [],
    horizon: 100000,
)"#
    );
    ron::from_str(&text).expect("probe scenario parses")
}

fn heap_cells(
    w: &pixel_physics::sim::World,
    heap_x: i32,
    half: i32,
    food: pixel_physics::sim::material::MaterialId,
) -> i64 {
    let mut n = 0;
    for x in (heap_x - half)..=(heap_x + half) {
        for y in 0..128 {
            if w.get(x, y).material == food {
                n += 1;
            }
        }
    }
    n
}

/// Printed whole with `{:?}`; only some fields feed the fitness.
#[allow(dead_code)]
#[derive(Clone, Copy, Default, Debug)]
struct Score {
    taken: f64,
    home: f64,
    eats: u64,
    pickups: u64,
    at_nest: f64,
    alive: usize,
    impulses: u64,
    flight_frames: u64,
    falls: u64,
}

fn eval(genome: &[f32], seed: u64, side: i32, cfg: Cfg) -> Score {
    eval_world(genome, seed, side, cfg, &mut |_, _| {})
}

fn eval_world(
    genome: &[f32],
    seed: u64,
    side: i32,
    cfg: Cfg,
    look: &mut dyn FnMut(u64, &pixel_physics::sim::World),
) -> Score {
    let heap_x = NEST_X + side * cfg.dist;
    let (mut w, _, _) = scenario(seed, heap_x, cfg.ants, cfg.heap).build();
    let sp = w.species.id_of("ant").expect("ant");
    let mut def = w.species.get(sp).creature.clone().expect("creature");
    def.reproduce_threshold = 1.0e30;
    w.species.set_creature(sp, def);
    if cfg.chooser_off {
        w.chooser = Some(creature::Chooser::Off);
    }
    for id in w.live_organism_ids() {
        if w.organism(id).map(|s| s.species) == Some(sp) {
            w.set_organism_genome(id, genome.to_vec());
        }
    }
    let food = w.materials.id_of("provisions").expect("provisions");
    let before = heap_cells(&w, heap_x, 20, food);
    for f in 0..cfg.frames {
        update::step(&mut w);
        w.step_active_sites();
        w.step_fields();
        w.step_pheromones();
        look(f + 1, &w);
    }
    let after = heap_cells(&w, heap_x, 20, food);
    let at_home = heap_cells(&w, NEST_X, 12, food);
    let alive = w
        .live_organism_ids()
        .iter()
        .filter(|&&id| w.organism(id).map(|s| s.species) == Some(sp))
        .count();
    Score {
        taken: (before - after) as f64,
        home: at_home as f64 + w.creature_stats.digested_at_nest_face / 960.0,
        eats: w.creature_stats.eats,
        pickups: w.creature_stats.pickups,
        at_nest: w.creature_stats.digested_at_nest_face,
        alive,
        impulses: w.creature_stats.impulses,
        flight_frames: w.creature_stats.flight_frames,
        falls: w.creature_stats.falls,
    }
}

/// Score every genome on every (seed, side) case, four worlds at a time.
fn eval_all(genomes: &[Vec<f32>], cases: &[(u64, i32)], cfg: Cfg) -> Vec<Vec<Score>> {
    let jobs: Vec<(usize, usize)> = (0..genomes.len())
        .flat_map(|g| (0..cases.len()).map(move |c| (g, c)))
        .collect();
    let threads = std::env::var("BRAINFIT_THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4usize);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let out = std::sync::Mutex::new(vec![vec![Score::default(); cases.len()]; genomes.len()]);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let j = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(&(g, c)) = jobs.get(j) else { break };
                let r = eval(&genomes[g], cases[c].0, cases[c].1, cfg);
                out.lock().unwrap()[g][c] = r;
            });
        }
    });
    out.into_inner().unwrap()
}

fn fit(s: &Score, cfg: Cfg) -> f64 {
    if cfg.fit_home {
        s.home
    } else {
        s.taken
    }
}

fn mean(v: &[Score], f: impl Fn(&Score) -> f64) -> f64 {
    v.iter().map(f).sum::<f64>() / v.len() as f64
}

fn main() {
    creature::mutation_off_for_measuring();
    let mut gens = 20usize;
    let mut pop = 24usize;
    let mut seeds = 2u64;
    let mut cfg = Cfg {
        frames: 6000,
        ants: 12,
        dist: 50,
        chooser_off: true,
        heap: 60,
        fit_home: false,
    };
    let mut pic: Option<String> = None;
    let mut nohop = false;
    let mut start = "founder".to_string();
    let mut cost = false;
    let mut pm = 0.03f32;
    let mut sigma = 1.0f32;
    let mut holdout = 8u64;
    for a in std::env::args().skip(1) {
        let (k, v) = a.split_once('=').unwrap_or((a.as_str(), "1"));
        match k {
            "gens" => gens = v.parse().unwrap(),
            "pop" => pop = v.parse().unwrap(),
            "seeds" => seeds = v.parse().unwrap(),
            "frames" => cfg.frames = v.parse().unwrap(),
            "ants" => cfg.ants = v.parse().unwrap(),
            "dist" => cfg.dist = v.parse().unwrap(),
            "chooser" => cfg.chooser_off = v == "off",
            "start" => start = v.to_string(),
            "cost" => cost = v == "1",
            "pm" => pm = v.parse().unwrap(),
            "sigma" => sigma = v.parse().unwrap(),
            "holdout" => holdout = v.parse().unwrap(),
            "heap" => cfg.heap = v.parse().unwrap(),
            "fit" => cfg.fit_home = v == "home",
            "pic" => pic = Some(v.to_string()),
            "nohop" => nohop = v == "1",
            _ => panic!("unknown argument {a}"),
        }
    }
    println!(
        "brainfit gens={gens} pop={pop} seeds={seeds} frames={} ants={} dist={} heap={} fit={} chooser={} start={start} pm={pm} sigma={sigma}",
        cfg.frames,
        cfg.ants,
        cfg.dist,
        cfg.heap,
        if cfg.fit_home { "home" } else { "taken" },
        if cfg.chooser_off { "off" } else { "shipped" }
    );

    let probe = scenario(1, NEST_X + cfg.dist, cfg.ants, cfg.heap).build().0;
    let founder = probe.species.get(probe.species.id_of("ant").unwrap()).genome.clone();
    // `nohop=1`: the hop and flight outputs are not ant verbs; their rows are
    // zeroed and never mutated, so training can only walk.
    let banned = |i: usize| -> bool {
        let o = if i < 4096 {
            Some(i / 64)
        } else if i >= 8256 {
            Some((i - 8256) / 64)
        } else {
            None
        };
        nohop
            && matches!(o, Some(o) if o == brain::BrainOutput::Impulse as usize || o == brain::BrainOutput::Fly as usize)
    };
    let live: Vec<usize> = brain::live_slots()
        .filter(|&i| i < brain::GENOME_LEN - brain::TRAIT_SLOTS && !banned(i))
        .collect();
    let base = match start.as_str() {
        "founder" => founder.clone(),
        f if f.starts_with('/') || f.ends_with(".txt") => {
            let text = std::fs::read_to_string(f).expect("genome file");
            let g: Vec<f32> = text.lines().map(|l| l.parse().unwrap()).collect();
            assert_eq!(g.len(), founder.len());
            g
        }
        "zero" => {
            let mut g = founder.clone();
            for &i in &live {
                g[i] = 0.0;
            }
            g
        }
        _ => panic!("start=founder|zero"),
    };
    let mut base = base;
    for (i, w) in base.iter_mut().enumerate().take(brain::GENOME_LEN - brain::TRAIT_SLOTS) {
        if banned(i) {
            *w = 0.0;
        }
    }
    println!(
        "live brain slots: {}, non-zero in start genome: {}",
        live.len(),
        live.iter().filter(|&&i| base[i] != 0.0).count()
    );

    if let Some(out) = pic {
        // Five snapshots of one run (seed 50,000, heap east), ants painted
        // magenta, stacked top to bottom.
        let shots = [
            cfg.frames / 5,
            2 * cfg.frames / 5,
            3 * cfg.frames / 5,
            4 * cfg.frames / 5,
            cfg.frames,
        ];
        let (vw, vh) = (256u32, 128u32);
        let mut sheet: Vec<u8> = Vec::new();
        let mut renderer = pixel_physics::render::Renderer::new();
        let particles = pixel_physics::sim::particle::ParticleSystem::new();
        let s = eval_world(&base, 50_000, 1, cfg, &mut |f, w| {
            if shots.contains(&f) {
                let mut buf = vec![0u8; (vw * vh * 4) as usize];
                let mut ww = w.clone();
                let touched = ww.take_touched_chunks();
                renderer.draw(&ww, &particles, &touched, &mut buf, (vw, vh), true);
                for id in w.live_organism_ids() {
                    let Some(st) = w.organism(id) else { continue };
                    for &(x, y) in &st.chain {
                        if x >= 0 && y >= 0 && (x as u32) < vw && (y as u32) < vh {
                            let i = ((y as u32 * vw + x as u32) * 4) as usize;
                            buf[i..i + 4].copy_from_slice(&[255, 0, 255, 255]);
                        }
                    }
                }
                sheet.extend_from_slice(&buf);
            }
        });
        let rows = (sheet.len() / (vw as usize * 4)) as u32;
        image::save_buffer(&out, &sheet, vw, rows, image::ColorType::Rgba8).expect("png");
        println!("picture {out}: {s:?}");
        return;
    }
    if cost {
        let t = Instant::now();
        let s = eval(&base, 1, 1, cfg);
        let dt = t.elapsed().as_secs_f64();
        println!(
            "one evaluation: {dt:.2} s for {} frames ({:.1} us/frame); {s:?}",
            cfg.frames,
            dt * 1e6 / cfg.frames as f64
        );
        return;
    }

    let mut rng = Rng(0x0B4A_1F17);
    let mutate = |g: &[f32], rng: &mut Rng| -> Vec<f32> {
        let mut c = g.to_vec();
        for &i in &live {
            if rng.unit() < pm {
                c[i] = (c[i] + sigma * rng.gauss()).clamp(-40.0, 40.0);
            }
        }
        c
    };
    let mut popn: Vec<Vec<f32>> = (0..pop)
        .map(|k| if k == 0 { base.clone() } else { mutate(&base, &mut rng) })
        .collect();
    let elites = (pop / 4).max(1);
    let t0 = Instant::now();
    println!("gen  best  median  start(control)  best:taken/home/eats/pickups/alive  secs");
    let mut best = base.clone();
    for gen in 0..gens {
        let cases: Vec<(u64, i32)> = (0..seeds)
            .map(|k| (1000 + gen as u64 * seeds + k, if k % 2 == 0 { 1 } else { -1 }))
            .collect();
        let mut all = popn.clone();
        all.push(base.clone());
        let tg = Instant::now();
        let scores = eval_all(&all, &cases, cfg);
        let control = mean(&scores[pop], |s| fit(s, cfg));
        let mut ranked: Vec<(f64, usize)> = (0..pop).map(|g| (mean(&scores[g], |s| fit(s, cfg)), g)).collect();
        ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let b = ranked[0].1;
        println!(
            "{gen:3} {:6.1} {:6.1} {:8.1}   {:.1}/{:.1}/{:.0}/{:.0}/{:.1}  {:.0}",
            ranked[0].0,
            ranked[pop / 2].0,
            control,
            mean(&scores[b], |s| s.taken),
            mean(&scores[b], |s| s.home),
            mean(&scores[b], |s| s.eats as f64),
            mean(&scores[b], |s| s.pickups as f64),
            mean(&scores[b], |s| s.alive as f64),
            tg.elapsed().as_secs_f64()
        );
        best = popn[b].clone();
        let parents: Vec<Vec<f32>> = ranked[..elites].iter().map(|&(_, g)| popn[g].clone()).collect();
        popn = parents.clone();
        while popn.len() < pop {
            let p = &parents[(rng.next() % elites as u64) as usize];
            popn.push(mutate(p, &mut rng));
        }
    }
    println!("total {:.0} s", t0.elapsed().as_secs_f64());

    // Hold-out: fresh seeds, both sides each, best vs start.
    let cases: Vec<(u64, i32)> = (0..holdout).flat_map(|k| [(50_000 + k, 1), (50_000 + k, -1)]).collect();
    let scores = eval_all(&[best.clone(), base.clone()], &cases, cfg);
    let row = |s: &[Score]| {
        s.iter()
            .map(|x| format!("{:.0}", fit(x, cfg)))
            .collect::<Vec<_>>()
            .join(",")
    };
    for (name, k) in [("best ", 0usize), ("start", 1)] {
        println!(
            "HOLDOUT {name} fit mean {:.1} [{}]  taken {:.1} home {:.1} alive {:.1}",
            mean(&scores[k], |s| fit(s, cfg)),
            row(&scores[k]),
            mean(&scores[k], |s| s.taken),
            mean(&scores[k], |s| s.home),
            mean(&scores[k], |s| s.alive as f64)
        );
    }
    let changed: Vec<String> = live
        .iter()
        .filter(|&&i| (best[i] - base[i]).abs() > 0.05)
        .map(|&i| format!("{i}:{:+.2}", best[i] - base[i]))
        .collect();
    println!("slots moved in best ({}): {}", changed.len(), changed.join(" "));
    let path = std::env::var("BRAINFIT_OUT").unwrap_or_else(|_| "brainfit-best.txt".into());
    std::fs::write(&path, best.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("\n")).ok();
    println!("best genome written to {path}");
}
