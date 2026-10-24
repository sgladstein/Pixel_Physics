//! **Two trained colonies on one bed: do they fight, who wins, and what does it cost them?** (colony-vs-colony
//! thread, 2026-10-24; the owner asked whether colonies are ready to fight each other.)
//!
//! The training bed (`colony_bed::scenario`, 256 x 128, 40 rows of soil) with **two** colonies founded heaped at
//! nests `gap` columns apart either side of the centre, each fed as the in-run bed feeds its one colony (a capped
//! drip `dist` columns either side of its own nest, scattered over `spread`). Every founder takes a mind from a pool
//! (default the shipped `std_pool`, one founder per member), breeding is the game's own life cycle with the
//! hatchling keeping its layer's mind (inrun's `sel=off`), and nobody is selected: this is a look at what two
//! colonies of today's ant do to each other, not training.
//!
//! **Why not `inrun`**: that harness books one colony -- its ledger, its selection and its culls read every ant of
//! the species as one colony -- so a neighbour would be scored, bred and culled as kin. A second colony there is a
//! change to the Teacher lane's harness and waits until this says it is worth it.
//!
//! ```text
//! cargo run --release --example rivalbed -- seed=1 frames=100000                  # both colonies, bite blank (0)
//! cargo run --release --example rivalbed -- seed=1 bite=off,off                    # neither can bite: competition only
//! cargo run --release --example rivalbed -- seed=1 bite=0,off                      # only the west colony can bite
//! cargo run --release --example rivalbed -- seed=1 solo=a                          # the west colony alone, same bed
//! cargo run --release --example rivalbed -- seed=1 seq=DIR every_shot=500 zoom=3   # PNG frames, colonies coloured
//! ```
//!
//! `bite=A,B`, one per colony (west, east): a number is the bite's job constant with the switch on (0 is the blank
//! verb `enter_bite` gives inrun's founders; -100 holds it off as the predator lane's `ant_bite=-100` did), `off`
//! bans it (the option is never listed), `keep` leaves each genome as the pool holds it. `pool=A,B`: `std`, or a
//! directory of `.raw.txt`. `lay_cap=T:D` is per colony (default inrun's 48:8); the game's laying bar is one per
//! species, so the cap is applied to the bed's total at `T x colonies : D x colonies` -- a colony that wins the bed
//! may grow into its loser's share, which is part of what winning means here.
//!
//! Prints a `RESULT` line per colony: adults alive at the end, the mean adults over the run (`lived`,
//! graded where alive is all-or-nothing), food delivered home (a trip cell put down within 12 columns of its own
//! nest, inrun's `deliver`), nest cuts (original ground at or below the datum within 20 columns of its own nest,
//! and `deep`, 3+ rows down, where the crater is not), deaths killed / starved, and kills it made of the other
//! colony (`World::kills_log`, which names both colonies). `bites` is every mind's bite (the counter is the
//! world's, not a colony's), so read a one-sided arm for who bit. `out=DIR` writes `timeline.csv` every 1,000 frames.
use pixel_physics::lab::scenario::Placement;
use pixel_physics::lab::training;
use pixel_physics::sim::creature;
use pixel_physics::sim::organism::DeathCause;
use pixel_physics::train;
use pixel_physics::train::challenges::{colony_bed, inrun};
use std::collections::HashMap;
use std::io::Write;

fn arg<'a>(a: &'a [String], k: &str) -> Option<&'a str> {
    a.iter().find_map(|s| s.strip_prefix(&format!("{k}=")))
}

/// One colony's book over the run.
#[derive(Default)]
struct Side {
    x: i32,
    colony: u32,
    delivered: u64,
    nest_cuts: u64,
    deep_cuts: u64,
    other_cuts: u64,
    killed: u64,
    starved: u64,
    kills_of_foe: u64,
    lived: u64,
    peak: usize,
    first_kill: Option<u64>,
    /// Bites this colony's ants gave and strangers they touched (`World::encounter_log`; a touch is logged only for
    /// the game's stranger alarm).
    bites_given: u64,
    touches: u64,
}

/// `std`: the shipped pool; anything else, a directory of `.raw.txt` (sorted by name).
fn load_pool(spec: &str) -> Vec<Vec<f32>> {
    if spec == "std" {
        return training::std_pool();
    }
    let mut paths: Vec<_> = std::fs::read_dir(spec).unwrap_or_else(|e| panic!("pool={spec}: {e}")).filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.to_string_lossy().ends_with(".raw.txt")).collect();
    paths.sort();
    assert!(!paths.is_empty(), "pool={spec}: no .raw.txt in it");
    paths
        .iter()
        .map(|p| {
            let text = std::fs::read_to_string(p).unwrap();
            let ps = p.to_string_lossy();
            let mut g = train::parse_genome(&text, train::genome_text_len(&text, &ps), &ps);
            training::fit_head(&mut g);
            g
        })
        .collect()
}

/// `bite=` for one colony: `Some(Some(k))` on at k, `Some(None)` banned, `None` as the pool holds it.
fn bite_of(s: &str) -> Option<Option<f32>> {
    match s {
        "keep" => None,
        "off" => Some(None),
        k => Some(Some(k.parse().expect("bite=K|off|keep"))),
    }
}

fn main() {
    std::env::set_var("PIXEL_PHYSICS_MUTATION", "off");
    std::env::set_var("PIXEL_PHYSICS_NEST_DOOR_FOUNDERS", "pile");
    let a: Vec<String> = std::env::args().skip(1).collect();
    let num = |k: &str, d: f64| arg(&a, k).map_or(d, |v| v.parse::<f64>().unwrap_or_else(|_| panic!("{k}=")));
    let (seed, frames, gap) = (num("seed", 1.0) as u64, num("frames", 100_000.0) as u64, num("gap", 80.0) as i32);
    let (dist, spread, cap, drip, every) = (num("dist", 25.0) as i32, num("spread", 10.0) as i32, num("cap", 60.0) as usize, num("drip", 20.0) as usize, num("every", 500.0) as u64);
    let grants = num("grants", 3.0) as f32;
    let solo = arg(&a, "solo").unwrap_or("none");
    let pools: Vec<&str> = arg(&a, "pool").unwrap_or("std,std").split(',').collect();
    let bites: Vec<&str> = arg(&a, "bite").unwrap_or("0,0").split(',').collect();
    assert!(pools.len() == 2 && bites.len() == 2, "pool=A,B and bite=A,B take one per colony");
    let (lay_t, lay_d) = arg(&a, "lay_cap").unwrap_or("48:8").split_once(':').map(|(t, d)| (t.parse::<f64>().unwrap(), d.parse::<f64>().unwrap().max(1.0))).expect("lay_cap=T:D");
    let xs = [colony_bed::NEST - gap / 2, colony_bed::NEST + gap / 2];
    let present = [solo != "b", solo != "a"];
    let n_col = present.iter().filter(|p| **p).count() as f64;

    let genomes: Vec<Vec<Vec<f32>>> = (0..2)
        .map(|s| {
            let mut pool = load_pool(pools[s]);
            for g in pool.iter_mut() {
                if let Some(b) = bite_of(bites[s]) {
                    creature::mind::set_bite(g, b);
                }
            }
            pool
        })
        .collect();
    let founders = num("ants", genomes[0].len() as f64) as i32;
    for s in 0..2 {
        let (sw, k) = creature::mind::bite_genes(&genomes[s][0]);
        let on = genomes[s].iter().filter(|g| creature::mind::bite_genes(g).0 > 0.0).count();
        println!("colony {} at x={}: pool {} ({} members), bite {} -> member 0 switch {sw} job {k}, {on} members with it on{}", ["west", "east"][s], xs[s], pools[s], genomes[s].len(), bites[s], if present[s] { "" } else { " (ABSENT: solo)" });
    }

    let mut sc = colony_bed::scenario(seed, founders as usize);
    for s in 0..2 {
        if present[s] {
            sc.placements.push(Placement::Colony { species: "ant".into(), x: xs[s], count: founders });
        }
    }
    let (mut w, _, _) = sc.build();
    let sp = w.species.id_of("ant").expect("ant");
    let mut def = w.species.get(sp).creature.clone().expect("creature");
    // As inrun's bed: no budding (the brood block lays instead) and no old age.
    def.reproduce_threshold = 1.0e30;
    def.life_half_life = 0;
    w.species.set_creature(sp, def);
    let base_lay_at = w.species.get(sp).creature.as_ref().and_then(|d| d.brood.as_ref()).map_or(0.0, |b| b.lay_at);
    assert!(base_lay_at > 0.0, "the ant has no brood block");
    w.births_paused = false;

    // The colonies as founded, west to east by where their ants stand.
    let mut by_colony: HashMap<u32, Vec<u32>> = HashMap::new();
    for id in inrun::ants(&w, sp) {
        let c = w.organism(id).unwrap().colony;
        by_colony.entry(c).or_default().push(id);
    }
    let mut cols: Vec<(f64, u32)> = by_colony
        .iter()
        .map(|(&c, ids)| (ids.iter().map(|&i| w.organism(i).and_then(|s| s.chain.first().map(|h| h.0)).unwrap_or(0) as f64).sum::<f64>() / ids.len() as f64, c))
        .collect();
    cols.sort_by(|p, q| p.0.partial_cmp(&q.0).unwrap());
    assert_eq!(cols.len() as f64, n_col, "founded {} colonies, wanted {n_col}", cols.len());
    let mut sides: Vec<Side> = (0..2).map(|s| Side { x: xs[s], ..Default::default() }).collect();
    let mut k = 0;
    for s in 0..2 {
        if present[s] {
            sides[s].colony = cols[k].1;
            k += 1;
        }
    }
    let side_of_colony = |c: u32, sides: &[Side]| (0..2).find(|&s| present[s] && sides[s].colony == c);
    for s in 0..2 {
        if !present[s] {
            continue;
        }
        let mut ids = by_colony[&sides[s].colony].clone();
        ids.sort_unstable();
        for (i, id) in ids.into_iter().enumerate() {
            let g = &genomes[s][i % genomes[s].len()];
            creature::mind::give(&mut w, id, colony_bed::mind(g));
        }
    }
    inrun::flat_reserves(&mut w, sp, grants);
    w.cut_log = Some(Vec::new());
    w.food_log = Some(Vec::new());
    w.death_log = Some(Vec::new());
    w.encounter_log = Some(Default::default());
    let surf: Vec<i32> = (0..colony_bed::SIZE.0).map(|x| inrun::surface(&w, &sc, x)).collect();
    println!("datum at the nests: west row {}, east row {}", surf[xs[0] as usize], surf[xs[1] as usize]);
    // Who is in which colony: founders now, hatchlings as they appear (a dead ant's row comes after it is gone).
    let mut who: HashMap<u32, usize> = HashMap::new();
    let refresh = |w: &pixel_physics::sim::World, who: &mut HashMap<u32, usize>, sides: &[Side]| {
        for id in inrun::ants(w, sp) {
            if let Some(s) = w.organism(id).and_then(|st| side_of_colony(st.colony, sides)) {
                who.insert(id, s);
            }
        }
    };
    refresh(&w, &mut who, &sides);
    let mut seen_births = w.creature_stats.births;
    let mut kills_seen = w.kills_log.len();
    let mut kill_csv = arg(&a, "out").map(|d| {
        std::fs::create_dir_all(d).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::path::Path::new(d).join("kills.csv")).unwrap());
        writeln!(f, "frame,side,attacker,victim,x,y,from_own_nest,from_foe_nest,attacker_energy,attacker_crop").unwrap();
        f
    });
    let mut timeline = arg(&a, "out").map(|d| {
        std::fs::create_dir_all(d).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::path::Path::new(d).join("timeline.csv")).unwrap());
        writeln!(f, "frame,side,adults,brood,delivered,nest_cuts,deep_cuts,killed,starved,kills_of_foe,bites").unwrap();
        f
    });
    // seq=DIR: the bed as the game draws it, animals by colony, every `every_shot` frames.
    let seq = arg(&a, "seq").map(|d| {
        std::fs::create_dir_all(d).unwrap();
        d.to_string()
    });
    let every_shot = num("every_shot", 500.0) as u64;
    let zoom = num("zoom", 3.0) as u32;
    // The bed's surface is near row 24; `look_y`/`rows` frame the sky above it and the nests below.
    let (look_y, rows) = (num("look_y", 44.0) as i32, num("rows", 56.0) as u32);
    let mut renderer = seq.as_ref().map(|_| {
        let mut r = pixel_physics::render::Renderer::new();
        r.creature_colour = pixel_physics::render::CreatureColour::Colony;
        for _ in 1..zoom {
            r.adjust_zoom(1);
        }
        let (bw, bh) = colony_bed::SIZE;
        r.set_camera(bw / 2, look_y, ((bw as u32) * zoom, rows * zoom), Some(pixel_physics::sim::chunk::Rect::new(0, 0, bw - 1, bh - 1)));
        r
    });
    let particles = pixel_physics::sim::particle::ParticleSystem::default();
    let t0 = std::time::Instant::now();
    let bites_now = |w: &pixel_physics::sim::World| w.minds.as_ref().map_or(0, |m| m.counts.bites);
    for f in 0..frames {
        if f % every == 0 {
            // Both nests' food, solo or not: the bed is the same bed with or without a neighbour.
            for x in xs {
                inrun::top_up(&mut w, &sc, x - dist, cap, drip, spread);
                inrun::top_up(&mut w, &sc, x + dist, cap, drip, spread);
            }
        }
        if f % 250 == 0 {
            // inrun's lay cap, on the bed's total (one laying bar per species).
            // Adults and brood, as inrun's cap counts them (eggs, larvae and pupae are `live_brood_ids`, not ants).
            let heads = (inrun::ants(&w, sp).len() + w.live_brood_ids().len()) as f64;
            let lay_at = base_lay_at as f64 * 2f64.powf((heads - lay_t * n_col).max(0.0) / (lay_d * n_col));
            let mut d = w.species.get(sp).creature.clone().unwrap();
            if let Some(b) = d.brood.as_mut() {
                if (b.lay_at as f64 - lay_at).abs() > 0.5 {
                    b.lay_at = lay_at as f32;
                    w.species.set_creature(sp, d);
                }
            }
        }
        inrun::step(&mut w, &sc);
        if w.creature_stats.births != seen_births {
            seen_births = w.creature_stats.births;
            refresh(&w, &mut who, &sides);
        }
        for (x, y, _, id) in w.cut_log.as_mut().unwrap().drain(..) {
            let Some(&s) = who.get(&id) else { continue };
            let below = y >= surf[x.clamp(0, colony_bed::SIZE.0 - 1) as usize];
            if below && (x - sides[s].x).abs() <= 20 {
                sides[s].nest_cuts += 1;
                if y >= surf[x as usize] + 3 {
                    sides[s].deep_cuts += 1;
                }
            } else {
                sides[s].other_cuts += 1;
            }
        }
        for r in w.food_log.as_mut().unwrap().drain(..) {
            if r.kind == creature::FOOD_DROP && r.flag {
                if let Some(&s) = who.get(&r.who) {
                    if (r.at.0 - sides[s].x).abs() <= 12 {
                        sides[s].delivered += 1;
                    }
                }
            }
        }
        for r in w.death_log.as_mut().unwrap().drain(..) {
            if r.species != sp {
                continue;
            }
            if let Some(s) = who.remove(&r.id) {
                match r.cause {
                    DeathCause::Killed => sides[s].killed += 1,
                    DeathCause::Starved => sides[s].starved += 1,
                    _ => {}
                }
            }
        }
        for e in w.encounter_log.as_mut().unwrap().events.drain(..) {
            if let Some(&s) = who.get(&e.a) {
                match e.kind {
                    pixel_physics::sim::world::ENCOUNTER_BITE => sides[s].bites_given += 1,
                    pixel_physics::sim::world::ENCOUNTER_TOUCH | pixel_physics::sim::world::ENCOUNTER_TOUCH_HELD => sides[s].touches += 1,
                    _ => {}
                }
            }
        }
        for kr in &w.kills_log[kills_seen..] {
            if kr.attacker_species == sp && kr.victim_species == sp && kr.attacker_colony != kr.victim_colony {
                if let Some(s) = side_of_colony(kr.attacker_colony, &sides) {
                    sides[s].kills_of_foe += 1;
                    sides[s].first_kill.get_or_insert(f);
                    // kills.csv: where each killing happened, against both nests -- a border skirmish and a raid on
                    // the other nest look the same in every count above.
                    if let Some(kf) = kill_csv.as_mut() {
                        let (ax, ay) = w.organism(kr.detail.attacker).and_then(|st| st.chain.first().copied()).unwrap_or((-1, -1));
                        let (en, ex) = w.organism(kr.detail.attacker).map_or((0.0, 0.0), |st| (st.energy, st.crop.map_or(0.0, |c| c.worth())));
                        writeln!(kf, "{},{},{},{},{},{},{},{},{:.0},{:.0}", f + 1, ["west", "east"][s], kr.detail.attacker, kr.detail.victim, ax, ay, ax - sides[s].x, ax - sides[1 - s].x, en, ex).unwrap();
                    }
                }
            }
        }
        kills_seen = w.kills_log.len();
        let fr = f + 1;
        let mut adults = [0usize; 2];
        let mut brood = [0usize; 2];
        if fr % 100 == 0 || fr % 1000 == 0 {
            for id in inrun::ants(&w, sp) {
                if let Some(s) = w.organism(id).and_then(|st| side_of_colony(st.colony, &sides)) {
                    adults[s] += 1;
                }
            }
            for id in w.live_brood_ids() {
                if let Some(s) = w.organism(id).and_then(|st| side_of_colony(st.colony, &sides)) {
                    brood[s] += 1;
                }
            }
            for s in 0..2 {
                sides[s].lived += adults[s] as u64 * 100;
                sides[s].peak = sides[s].peak.max(adults[s] + brood[s]);
            }
        }
        if fr % 1000 == 0 {
            if let Some(t) = timeline.as_mut() {
                for s in (0..2).filter(|&s| present[s]) {
                    let b = &sides[s];
                    writeln!(t, "{fr},{},{},{},{},{},{},{},{},{},{}", ["west", "east"][s], adults[s], brood[s], b.delivered, b.nest_cuts, b.deep_cuts, b.killed, b.starved, b.kills_of_foe, bites_now(&w)).unwrap();
                }
            }
            if fr % 10_000 == 0 {
                eprintln!("frame {fr}: west {}+{} east {}+{} | kills W->E {} E->W {} | bites {}", adults[0], brood[0], adults[1], brood[1], sides[0].kills_of_foe, sides[1].kills_of_foe, bites_now(&w));
            }
        }
        if let (Some(dir), Some(r)) = (seq.as_ref(), renderer.as_mut()) {
            if fr % every_shot == 0 {
                let (vw, vh) = ((colony_bed::SIZE.0 as u32) * zoom, rows * zoom);
                let mut buf = vec![0u8; (vw * vh * 4) as usize];
                let touched = w.take_touched_chunks();
                r.draw(&w, &particles, &touched, &mut buf, (vw, vh), true);
                image::RgbaImage::from_raw(vw, vh, buf).unwrap().save(format!("{dir}/f{fr:07}.png")).unwrap();
            }
        }
    }
    let k = w.minds.as_ref().map_or(Default::default(), |m| m.counts);
    let mut alive = [0usize; 2];
    for id in inrun::ants(&w, sp) {
        if let Some(s) = w.organism(id).and_then(|st| side_of_colony(st.colony, &sides)) {
            alive[s] += 1;
        }
    }
    let arm = format!("bite={},{} solo={solo} gap={gap}", bites[0], bites[1]);
    for s in (0..2).filter(|&s| present[s]) {
        let b = &sides[s];
        println!(
            "RESULT seed={seed} {arm} side={} alive={} lived={:.1} peak={} delivered={} nest_cuts={} deep_cuts={} other_cuts={} killed={} starved={} kills_of_foe={} first_kill={} bites_given={} touches={}",
            ["west", "east"][s],
            alive[s],
            b.lived as f64 / frames as f64,
            b.peak,
            b.delivered,
            b.nest_cuts,
            b.deep_cuts,
            b.other_cuts,
            b.killed,
            b.starved,
            b.kills_of_foe,
            b.first_kill.map_or("-".to_string(), |f| f.to_string()),
            b.bites_given,
            b.touches
        );
    }
    println!("mind acts: bites {} kills {} flesh {} | alarm laid {:.1}", k.bites, k.kills, k.flesh, k.laid[2]);
    println!("done: {frames} frames in {:.1}s", t0.elapsed().as_secs_f64());
}
