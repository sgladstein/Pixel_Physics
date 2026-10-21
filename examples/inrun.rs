//! SCRATCH (teacher lane, in-run selection pilot, 2026-10-21; never for main): one colony on the training bed with
//! Dig on and surface food, every founder given a mind from a mix of genomes, the body's egg-laying off. Writes the
//! per-ant events the pilot's score is built from, and the nest's open cells every `window` frames.
//!
//! cargo run --release --example inrun -- founders=v2.txt:15,s171.txt:15[,zero:4] frames=200000 seed=1 out=DIR
//!     [dist=40] [drip=5] [every=500] [window=2000] [maps=1]
//!
//! Genomes load through the copylog suite, as training loads them. `zero` is an all-zero mind (every option scored
//! alike: a random chooser, the do-nothing-useful control).
//!
//! out/ants.csv     id,tag                     founders, in id order
//! out/events.csv   frame,id,kind,x,y,v,cutter
//!     dig       a cut of a cell that was ground below the datum at frame 0 (v: depth below the datum)
//!     recut     a cut of anything else (spoil, mound, fill); cutter: the tag found within 2 cells, or 0
//!     out       a pellet let go above the datum (left the nest); cutter: the pellet's cutter, or 0
//!     in        a pellet let go at or below the datum; its tag is kept at that spot for a recut
//!     field     a food cell taken into the crop away from home (v: its worth)
//!     deliver   a trip cell put down at the nest: the game's own delivery (`trip_deliveries`)
//!     churn     any other food cell put down within 12 columns of the nest
//!     eat       the crop's digestion paid this ant v (face worth)
//!     death     v: the cause's index
//! out/map_fNNNNNNN.txt  `x0 y0 w h`, then rows of `.` open / `a` animal / `f` provisions / `#` other, the nest box
//! out/surface.txt       the datum, one row per column
use pixel_physics::sim::creature;
use pixel_physics::train::challenges::{inrun, Scarce, Soil, Suite, Trail};
use pixel_physics::train::{self};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

fn arg<'a>(a: &'a [String], k: &str) -> Option<&'a str> {
    a.iter().find_map(|s| s.strip_prefix(&format!("{k}=")))
}

fn main() {
    std::env::set_var("PIXEL_PHYSICS_MUTATION", "off");
    let a: Vec<String> = std::env::args().skip(1).collect();
    // pile=1 (default): every founder starts at the nest door, heaped (the game's own
    // `PIXEL_PHYSICS_NEST_DOOR_FOUNDERS=pile`), not spread along the ground (owner, 2026-10-21).
    if arg(&a, "pile") != Some("0") {
        std::env::set_var("PIXEL_PHYSICS_NEST_DOOR_FOUNDERS", "pile");
    }
    let num = |k: &str, d: u64| arg(&a, k).map_or(d, |v| v.parse().unwrap());
    let (frames, seed, dist, drip, every, window) = (num("frames", 200_000), num("seed", 1), num("dist", 40) as i32, num("drip", 5) as u32, num("every", 500), num("window", 2000));
    let maps = num("maps", 1) == 1;
    let cap = num("cap", 60) as usize;
    let spread = num("spread", 0) as i32;
    let out = std::path::PathBuf::from(arg(&a, "out").expect("out=DIR"));
    std::fs::create_dir_all(&out).unwrap();
    let suite = Suite::new(
        vec![
            (Box::new(Soil { ants: 1, frames: 12000, step: 4, shaft: 8, budget: Some(2400.0), foot: true, step_near: 0.0 }), false),
            (
                Box::new(Scarce { ants: 1, frames: 6000, drip: 5, every: 1000, game_walk: false, brake: Some(creature::FoodBrake::FORAGE), dist: (10, 30), steps: true, fed: true, dig: false }),
                true,
            ),
            (Box::new(Trail { ants: 6, frames: 6000, clones: false }), true),
        ],
        0.8,
    );
    // founders=path:count,...
    let mut mix: Vec<(String, Arc<creature::mind::MindGenome>, usize)> = Vec::new();
    let mut glen = 0;
    for part in arg(&a, "founders").expect("founders=path:count,...").split(',') {
        let (p, n) = part.rsplit_once(':').expect("path:count");
        let n: usize = n.parse().unwrap();
        if p == "zero" {
            mix.push((p.to_string(), Arc::new(creature::mind::MindGenome::new(vec![0.0; creature::mind::GENOME_LEN])), n));
        } else {
            let g = train::start_mind(&suite, p);
            glen = g.len();
            let tag = std::path::Path::new(p).file_stem().unwrap().to_string_lossy().to_string();
            mix.push((tag, inrun::mind(&g), n));
        }
    }
    for m in mix.iter_mut().filter(|m| m.0 == "zero") {
        m.1 = inrun::mind(&vec![0.0; glen]);
    }
    let ants: usize = mix.iter().map(|m| m.2).sum();
    // Interleaved round-robin, so no genome owns one end of the founding heap.
    let mut order: Vec<usize> = Vec::with_capacity(ants);
    let mut left: Vec<usize> = mix.iter().map(|m| m.2).collect();
    while order.len() < ants {
        for (i, n) in left.iter_mut().enumerate() {
            if *n > 0 {
                *n -= 1;
                order.push(i);
            }
        }
    }
    let slot = |k: usize| order[k.min(ants - 1)];
    let (mut w, sc, sp) = inrun::colony(seed, ants, dist, drip, every, &|k| mix[slot(k)].1.clone());
    w.cut_log = Some(Vec::new());
    w.food_log = Some(Vec::new());
    w.digest_log = Some(Vec::new());
    w.death_log = Some(Vec::new());
    let founders = inrun::ants(&w, sp);
    // flat=1: every founder starts on one full grant (the founding deals staggered reserves).
    // grants=G: every founder starts on G full grants (default: the founding's staggered reserves).
    if let Some(g) = arg(&a, "grants") {
        inrun::flat_reserves(&mut w, sp, g.parse().unwrap());
    }
    println!(
        "inrun: seed {seed}, {} founders ({}), {frames} frames, food {dist} cols out, {drip} cells every {every} up to {cap} a side, out {}",
        founders.len(),
        mix.iter().map(|m| format!("{}x{}", m.0, m.2)).collect::<Vec<_>>().join(" "),
        out.display()
    );
    let mut f = std::fs::File::create(out.join("ants.csv")).unwrap();
    writeln!(f, "id,tag").unwrap();
    for (k, id) in founders.iter().enumerate() {
        writeln!(f, "{id},{}", mix[slot(k)].0).unwrap();
    }
    // The datum and the ground below it at frame 0.
    let (bw, bh) = (256i32, 128i32); // the training bed (`bed_timed`)
    let surf: Vec<i32> = (0..bw).map(|x| inrun::surface(&w, &sc, x)).collect();
    let mut f = std::fs::File::create(out.join("surface.txt")).unwrap();
    for s in &surf {
        writeln!(f, "{s}").unwrap();
    }
    let mut orig = vec![false; (bw * bh) as usize];
    for x in 0..bw {
        for y in surf[x as usize]..bh {
            orig[(y * bw + x) as usize] = inrun::ground(&w, x, y);
        }
    }
    let mut ev = std::io::BufWriter::new(std::fs::File::create(out.join("events.csv")).unwrap());
    writeln!(ev, "frame,id,kind,x,y,v,cutter").unwrap();
    // Who cut the pellet each ant holds; pellets let go inside, by spot; field food each ant holds.
    let mut held: HashMap<u32, u32> = HashMap::new();
    let mut loose: HashMap<(i32, i32), u32> = HashMap::new();
    let t0 = std::time::Instant::now();
    for fr in 1..=frames {
        inrun::step(&mut w, &sc);
        if fr % every == 1 || every == 1 {
            for x in [inrun::NEST + dist, inrun::NEST - dist] {
                inrun::top_up(&mut w, &sc, x, cap, drip as usize, spread);
            }
        }
        let frame = w.frame;
        for (x, y, _, who) in w.cut_log.as_mut().unwrap().drain(..) {
            let i = (y * bw + x) as usize;
            if (0..bw).contains(&x) && (0..bh).contains(&y) && orig[i] {
                orig[i] = false;
                held.insert(who, who);
                writeln!(ev, "{frame},{who},dig,{x},{y},{},{who}", y - surf[x as usize]).unwrap();
            } else {
                let mut tag = 0;
                'find: for r in 0..=2 {
                    for dy in -r..=r {
                        for dx in -r..=r {
                            if let Some(t) = loose.remove(&(x + dx, y + dy)) {
                                tag = t;
                                break 'find;
                            }
                        }
                    }
                }
                held.insert(who, tag);
                writeln!(ev, "{frame},{who},recut,{x},{y},0,{tag}").unwrap();
            }
        }
        for r in w.food_log.as_mut().unwrap().drain(..) {
            let (x, y) = r.at;
            let home = (x - inrun::NEST).abs() <= 12;
            match r.kind {
                creature::FOOD_SWALLOW if !r.flag => writeln!(ev, "{frame},{},field,{x},{y},{},0", r.who, r.worth).unwrap(),
                creature::FOOD_DROP if r.flag => writeln!(ev, "{frame},{},deliver,{x},{y},0,0", r.who).unwrap(),
                creature::FOOD_DROP if home => writeln!(ev, "{frame},{},churn,{x},{y},0,0", r.who).unwrap(),
                _ => {}
            }
        }
        for (who, worth, _) in w.digest_log.as_mut().unwrap().drain(..) {
            writeln!(ev, "{frame},{who},eat,0,0,{worth},0").unwrap();
        }
        for r in w.death_log.as_mut().unwrap().drain(..) {
            if r.species == sp {
                writeln!(ev, "{frame},{},death,0,0,{},0", r.id, r.cause.index()).unwrap();
                held.remove(&r.id);
            }
        }
        // A pellet let go: where the head is now.
        let gone: Vec<u32> = held.keys().copied().filter(|&id| w.organism(id).is_none_or(|s| s.spoil.is_none())).collect();
        for id in gone {
            let tag = held.remove(&id).unwrap();
            let Some(&(x, y)) = w.organism(id).and_then(|s| s.chain.first()) else { continue };
            if y < surf[x.clamp(0, bw - 1) as usize] {
                writeln!(ev, "{frame},{id},out,{x},{y},0,{tag}").unwrap();
            } else {
                loose.insert((x, y), tag);
                writeln!(ev, "{frame},{id},in,{x},{y},0,{tag}").unwrap();
            }
        }
        if fr % window == 0 {
            let alive = inrun::ants(&w, sp).len();
            println!("frame {fr:7}  alive {alive:3}  {:.1}s", t0.elapsed().as_secs_f64());
            let mut pf = std::fs::File::create(out.join(format!("ants_f{fr:07}.txt"))).unwrap();
            for id in inrun::ants(&w, sp) {
                if let Some(st) = w.organism(id) {
                    let (x, y) = st.chain.first().copied().unwrap_or((0, 0));
                    writeln!(pf, "{id} {x} {y} {:.1} {}", st.energy, u8::from(st.spoil.is_some())).unwrap();
                }
            }
            if maps {
                let (x0, x1, y0, y1) = (inrun::NEST - 100, inrun::NEST + 100, 40, bh - 1);
                let mut m = std::fs::File::create(out.join(format!("map_f{fr:07}.txt"))).unwrap();
                writeln!(m, "{x0} {y0} {} {}", x1 - x0 + 1, y1 - y0 + 1).unwrap();
                for y in y0..=y1 {
                    let row: String = (x0..=x1).map(|x| inrun::cell_char(&w, x, y)).collect();
                    writeln!(m, "{row}").unwrap();
                }
            }
        }
    }
    ev.flush().unwrap();
    println!("done: {frames} frames in {:.1}s ({:.0} frames/s)", t0.elapsed().as_secs_f64(), frames as f64 / t0.elapsed().as_secs_f64());
}
