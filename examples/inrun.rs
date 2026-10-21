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
    // The raw genome behind each mix entry, for the selection loop's children.
    let mut raw: Vec<Vec<f32>> = Vec::new();
    let mut glen = 0;
    for part in arg(&a, "founders").expect("founders=path:count,...").split(',') {
        let (p, n) = part.rsplit_once(':').expect("path:count");
        // path@tag:count names the genome's family (a later round's founders keep their line).
        let (p, tag_over) = p.split_once('@').map_or((p, None), |(p, t)| (p, Some(t.to_string())));
        let n: usize = n.parse().unwrap();
        if p == "zero" {
            mix.push((p.to_string(), Arc::new(creature::mind::MindGenome::new(vec![0.0; creature::mind::GENOME_LEN])), n));
            raw.push(Vec::new());
        } else {
            // A `.raw.txt` genome is taken as saved (a harness's own output), not through the suite's rung entry.
            let g = if p.ends_with(".raw.txt") { train::load_genome(std::path::Path::new(p), creature::mind::GENOME_LEN) } else { train::start_mind(&suite, p) };
            raw.push(g.clone());
            glen = g.len();
            let tag = tag_over.unwrap_or_else(|| std::path::Path::new(p).file_stem().unwrap().to_string_lossy().to_string());
            mix.push((tag, inrun::mind(&g), n));
        }
    }
    // mutants=N:pm:sigma:mseed -- N trainer-style mutants of the first genome, one founder each (m00..),
    // replacing the founders list's counts; the same mseed gives the same mutants on every colony seed.
    if let Some(spec) = arg(&a, "mutants") {
        let v: Vec<&str> = spec.split(':').collect();
        let (n, pm, sigma, ms): (usize, f32, f32, u64) = (v[0].parse().unwrap(), v[1].parse().unwrap(), v[2].parse().unwrap(), v[3].parse().unwrap());
        let base = train::start_mind(&suite, arg(&a, "founders").unwrap().split(',').next().unwrap().rsplit_once(':').unwrap().0);
        let slots = inrun::slots();
        let mut rng = train::Rng(ms);
        mix.clear();
        raw.clear();
        let mut f = std::fs::File::create(out.join("mutants.txt")).unwrap();
        for k in 0..n {
            let g = if k == 0 { base.clone() } else { train::mutate(&base, &slots, pm, sigma, 0.0, 1.0, &[], &mut rng) };
            let d: f32 = g.iter().zip(&base).map(|(x, y)| (x - y).abs()).sum();
            writeln!(f, "m{k:02} changed {} L1 {d:.2}", g.iter().zip(&base).filter(|(x, y)| x != y).count()).unwrap();
            mix.push((format!("m{k:02}"), inrun::mind(&g), 1));
            raw.push(g);
        }
    }
    for (m, r) in mix.iter_mut().zip(raw.iter_mut()).filter(|(m, _)| m.0 == "zero") {
        m.1 = inrun::mind(&vec![0.0; glen]);
        *r = vec![0.0; glen];
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
    // ---- The in-run selection loop (sel=score|random; off by default) ----
    let sel = arg(&a, "sel").unwrap_or("off").to_string();
    let fnum = |k: &str, d: f64| arg(&a, k).map_or(d, |v| v.parse::<f64>().unwrap());
    let (min_age, judge_every, cooldown) = (num("min_age", 15_000), num("judge", 500), num("cooldown", 5_000));
    let (zb0, zc0, kprice) = (fnum("zbirth", 2.0), fnum("zcut", 2.0), fnum("k", 2.0));
    let (lo, hi) = (num("lo", 24) as usize, num("hi", 36) as usize);
    let (kpm, ksigma) = (fnum("kpm", 0.03) as f32, fnum("ksigma", 1.0) as f32);
    let kid_grants = fnum("kid_grants", 1.0) as f32;
    // Exchange rates between jobs, and a debit per food cell swallowed away from home (default: the ledger's 1,1,1,0).
    let (w_del, w_dig, w_clear, w_field) = (fnum("w_deliver", 1.0), fnum("w_dig", 1.0), fnum("w_clear", 1.0), fnum("w_field", 0.0));
    // Pay by depth: a delivery `d` rows below the datum is worth (1 + w_depth * min(d, 10) / 10) times (owner, 2026-10-21:
    // food dropped at the door is not food stored in the nest).
    let w_depth = fnum("w_depth", 0.0);
    // Pay by how far the spoil went: a pellet let go within `rim` columns of the nest pays nothing for its carry (nor the
    // cutter's half), rising to full `rim_ramp` columns further out. Spoil heaped on the rim rolls back into the pit:
    // after the pit's peak, 764 pellets let go within 30 columns against 198 dropped inside it (p5, 2026-10-21).
    let (rim, rim_ramp) = (fnum("rim", 0.0), fnum("rim_ramp", 10.0).max(1.0));
    // A meal at home after work (owner, 2026-10-21: "the nest needs a purpose"): a food cell swallowed at home pays
    // `w_home_eat` once per stretch of work (a trip out, a dig, a carry), so working and coming back to eat pays and
    // sitting at home eating the store does not (the stay-at-homes are who drained the store and starved together in the first pilots).
    let w_home_eat = fnum("w_home_eat", 0.0);
    // A floor on turnover: when `turnover` frames pass with no birth, the adult with the lowest z is replaced by a
    // child of the adult with the highest (random arm: both at random). A healthy colony otherwise froze: in the
    // nest-purpose rounds, once the headcount sat in its band no ant cleared the bars and rounds 2-3 had 0-47 births
    // in 200k frames, so selection stopped exactly when the colony was doing well (p6, 2026-10-21).
    let turnover = num("turnover", 0);
    // turn=T (owner, 2026-10-21: "a target turnover and the bar moves based on the distribution of score"): each
    // adult is replaced about once per T frames. The bar is a rank, not a fixed z, so it holds whatever scale the
    // score has after a reweighting. Clear failures (z below -zhard) go on top of the quota; no ant below the median
    // breeds to fill it; the best get up to three children (passes over the top half and quarter); headcount
    // outside lo..hi adds births or culls. Replaces the z bars and `turnover` when set.
    let turn = num("turn", 0);
    let zhard = fnum("zhard", 3.0);
    // Parents stay (owner's debate, 2026-10-21), but each child is earned fresh: fresh_kid=1 judges a parent's next
    // child on its record since its last one (at least min_age frames of it), and kid_cap bounds a lifetime's children.
    let fresh_kid = num("fresh_kid", 0) == 1;
    let kid_cap = num("kid_cap", 1000) as u32;
    // retire=1: a parent leaves once it has had `kid_cap` children (its last child takes its place). Without it the
    // capped parents, immortal with old age off, sat in the colony unable to breed: 307 of 342 judgements owing a
    // replacement found no eligible parent (p7 seed 4, 2026-10-21) -- the "old ants clogging" the owner and I agreed
    // to watch for.
    let retire = num("retire", 0) == 1;
    let mut retiring: Vec<u32> = Vec::new();
    let mut capped_only = false;
    let mut quota = 0.0f64;
    let mut quota_cut: Vec<u32> = Vec::new();
    // Diagnostics: judgements with a quota due, how many found no eligible parent, and placements that failed.
    let (mut q_due, mut q_noparent, mut birth_fail) = (0u64, 0u64, 0u64);
    let mut last_birth = 0u64;
    let mut srng = train::Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xA5A5);
    let kslots = inrun::slots();
    struct Who {
        born: u64,
        credit: f64,
        g2: f64,
        family: String,
        gen: u32,
        genome: Vec<f32>,
        last_kid: u64,
        kids: u32,
        // Worked since its last paid meal at home: out on a trip (head 15+ columns from the nest), a cut of the
        // ground, or a pellet carried out (owner, 2026-10-21: "also reward diggers who eat in the nest").
        tripped: bool,
        // The record at its last child (credit, sum of squares): with `fresh_kid`, the next child is earned on the
        // record since then.
        at_kid: (f64, f64),
    }
    let mut book: HashMap<u32, Who> = HashMap::new();
    for (k, id) in founders.iter().enumerate() {
        book.insert(*id, Who { born: 0, credit: 0.0, g2: 0.0, family: mix[slot(k)].0.clone(), gen: 0, genome: raw[slot(k)].clone(), last_kid: 0, kids: 0, tripped: false, at_kid: (0.0, 0.0) });
    }
    let mut csv_ants = std::fs::OpenOptions::new().append(true).open(out.join("ants.csv")).unwrap();
    let mut births = std::io::BufWriter::new(std::fs::File::create(out.join("births.csv")).unwrap());
    writeln!(births, "frame,id,parent,family,gen,kind").unwrap();
    let mut sel_log = std::io::BufWriter::new(std::fs::File::create(out.join("sel.csv")).unwrap());
    writeln!(sel_log, "frame,alive,adults,births,removals,bias,median,food_price,dig_price,clear_price,ghosts").unwrap();
    let mut bias = 0.0f64;
    // Ants culled this frame whose death record has not come in yet. `slay` frees the slot at once and the record lands
    // a frame later, so a child born in the same judgement can take the culled ant's id; without this, that record
    // struck the child out of the book and it lived on unjudged (a "ghost": up to 16 of 40 ants in the first pilots).
    let mut culled_pending: std::collections::HashSet<u32> = Default::default();
    // The prices (the ledger's): food from delivered / eaten worth over 5,000 frames; dig from open nest cells per
    // ant against 25; clear from pellets let go inside, net of re-cuts, per ant.
    let mut delivered: std::collections::VecDeque<(u64, f64)> = Default::default();
    let mut eaten: std::collections::VecDeque<(u64, f64)> = Default::default();
    let (mut cell_worth, mut n_field) = (0.0f64, 0u64);
    let mut loose_n = 0i64;
    let mut dig_price = 1.0 + kprice;
    let mut base_open: Option<i64> = None;
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
                if let Some(b) = book.get_mut(&who) {
                    b.tripped = true;
                    let g = 0.5 * w_dig * dig_price;
                    b.credit += g;
                    b.g2 += g * g;
                }
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
                if tag != 0 {
                    loose_n -= 1;
                }
            }
        }
        for r in w.food_log.as_mut().unwrap().drain(..) {
            let (x, y) = r.at;
            let home = (x - inrun::NEST).abs() <= 12;
            match r.kind {
                creature::FOOD_SWALLOW if r.flag => {
                    if let Some(b) = book.get_mut(&r.who) {
                        if b.tripped && w_home_eat > 0.0 {
                            b.tripped = false;
                            b.credit += w_home_eat;
                            b.g2 += w_home_eat * w_home_eat;
                        }
                    }
                    writeln!(ev, "{frame},{},homeeat,{x},{y},{},0", r.who, r.worth).unwrap()
                }
                creature::FOOD_SWALLOW if !r.flag => {
                    n_field += 1;
                    cell_worth += (r.worth as f64 - cell_worth) / n_field as f64;
                    if let Some(b) = book.get_mut(&r.who) {
                        b.credit -= w_field;
                        b.g2 += w_field * w_field;
                    }
                    writeln!(ev, "{frame},{},field,{x},{y},{},0", r.who, r.worth).unwrap()
                }
                creature::FOOD_DROP if r.flag => {
                    delivered.push_back((frame, if n_field > 0 { cell_worth } else { 1.0 }));
                    if let Some(b) = book.get_mut(&r.who) {
                        let got: f64 = delivered.iter().map(|d| d.1).sum();
                        let ate: f64 = eaten.iter().map(|d| d.1).sum();
                        let deep = 1.0 + w_depth * ((y - surf[x.clamp(0, bw - 1) as usize]).clamp(0, 10) as f64 / 10.0);
                        let g = deep * w_del * (1.0 + kprice * (1.0 - if ate > 0.0 { got / ate } else { 1.0 }).max(0.0));
                        b.credit += g;
                        b.g2 += g * g;
                    }
                    writeln!(ev, "{frame},{},deliver,{x},{y},0,0", r.who).unwrap()
                }
                creature::FOOD_DROP if home => writeln!(ev, "{frame},{},churn,{x},{y},0,0", r.who).unwrap(),
                _ => {}
            }
        }
        for (who, worth, _) in w.digest_log.as_mut().unwrap().drain(..) {
            writeln!(ev, "{frame},{who},eat,0,0,{worth},0").unwrap();
            eaten.push_back((frame, worth as f64));
        }
        for r in w.death_log.as_mut().unwrap().drain(..) {
            if r.species == sp {
                writeln!(ev, "{frame},{},death,0,0,{},0", r.id, r.cause.index()).unwrap();
                held.remove(&r.id);
                if !culled_pending.remove(&r.id) {
                    book.remove(&r.id);
                }
            }
        }
        // A pellet let go: where the head is now.
        let gone: Vec<u32> = held.keys().copied().filter(|&id| w.organism(id).is_none_or(|s| s.spoil.is_none())).collect();
        for id in gone {
            let tag = held.remove(&id).unwrap();
            let Some(&(x, y)) = w.organism(id).and_then(|s| s.chain.first()) else { continue };
            if y < surf[x.clamp(0, bw - 1) as usize] {
                writeln!(ev, "{frame},{id},out,{x},{y},0,{tag}").unwrap();
                let alive = book.len().max(1) as f64;
                let far = ((((x - inrun::NEST).abs() as f64) - rim) / rim_ramp).clamp(0.0, 1.0);
                let far = if rim > 0.0 { far } else { 1.0 };
                let pc = far * w_clear * (1.0 + kprice * (loose_n as f64 / (2.0 * alive)).clamp(0.0, 1.0));
                if let Some(b) = book.get_mut(&id) {
                    b.tripped = true;
                    b.credit += pc;
                    b.g2 += pc * pc;
                }
                if tag != 0 {
                    if let Some(b) = book.get_mut(&tag) {
                        let g = far * 0.5 * w_dig * dig_price;
                        b.credit += g;
                        b.g2 += g * g;
                    }
                }
            } else {
                loose.insert((x, y), tag);
                loose_n += 1;
                writeln!(ev, "{frame},{id},in,{x},{y},0,{tag}").unwrap();
            }
        }
        if fr % 100 == 0 {
            for (id, b) in book.iter_mut() {
                if let Some(&(x, _)) = w.organism(*id).and_then(|s| s.chain.first()) {
                    if (x - inrun::NEST).abs() >= 15 {
                        b.tripped = true;
                    }
                }
            }
        }
        while delivered.front().is_some_and(|d| d.0 + 5000 < frame) {
            delivered.pop_front();
        }
        while eaten.front().is_some_and(|d| d.0 + 5000 < frame) {
            eaten.pop_front();
        }
        if fr % 1000 == 0 || fr == 1 {
            let mut open_n = 0i64;
            for x in (inrun::NEST - 100)..=(inrun::NEST + 100) {
                for y in surf[x as usize].max(40)..bh {
                    if matches!(inrun::cell_char(&w, x, y), '.' | 'a') {
                        open_n += 1;
                    }
                }
            }
            let b0 = *base_open.get_or_insert(open_n);
            let per = (open_n - b0) as f64 / book.len().max(1) as f64;
            dig_price = 1.0 + kprice * (1.0 - per / 25.0).max(0.0);
        }
        if sel != "off" && fr % judge_every == 0 {
            // Each ant's rate per 1,000 frames alive, against the adults' median, in units of its own standard error:
            // a sum of credits is Poisson-like, so its error is sqrt(sum of squares), floored at what the colony's
            // average ant would have shown over the same age (an ant with no events is not certain).
            let ids: Vec<u32> = { let mut v: Vec<u32> = book.keys().copied().collect(); v.sort_unstable(); v };
            let n = ids.len();
            let (sg2, st): (f64, f64) = ids.iter().map(|i| { let b = &book[i]; (b.g2, (frame - b.born) as f64 / 1000.0) }).fold((0.0, 0.0), |a, c| (a.0 + c.0, a.1 + c.1));
            let rate_g2 = sg2 / st.max(1e-9);
            let adults: Vec<u32> = ids.iter().copied().filter(|i| frame - book[i].born >= min_age).collect();
            let mut births_n = 0;
            let (mut q_now, mut hard_n) = (0usize, 0usize);
            let mut removals_n = 0;
            let mut med = f64::NAN;
            // One adult is enough to judge by once `turn` is on: a colony crashed to two ants by a dry store could
            // otherwise never refill (seed 4: 24 -> 2 at 120k, then no judgement for the rest of the round).
            if adults.len() >= if turn > 0 { 1 } else { 3 } {
                if n < lo {
                    bias = (bias - 0.05).max(-1.5);
                } else if n > hi {
                    bias = (bias + 0.05).min(1.5);
                }
                let score = |b: &Who| b.credit / ((frame - b.born) as f64 / 1000.0).max(0.1);
                let mut sc: Vec<f64> = adults.iter().map(|i| score(&book[i])).collect();
                sc.sort_by(|a, b| a.partial_cmp(b).unwrap());
                med = sc[sc.len() / 2];
                let z = |b: &Who| {
                    let t = ((frame - b.born) as f64 / 1000.0).max(0.1);
                    (score(b) - med) / (b.g2.max(rate_g2 * t).sqrt() / t).max(1e-6)
                };
                let (zb, zc) = (zb0 + bias, zc0 - bias);
                // The family cap: no founder line past a quarter of the colony.
                let mut fam: HashMap<String, usize> = HashMap::new();
                for i in &ids {
                    *fam.entry(book[i].family.clone()).or_default() += 1;
                }
                // A quarter of the band's top, not of the colony now: as a quarter of `n` it shut every family out
                // once a colony was small (n 7: cap 2), so a starving colony could not refill (p7, 2026-10-21).
                let cap_f = (hi / 4).max(2);
                // Who the score picks: parents with their kid counts, and removals.
                let mut parents: Vec<(u32, u32)> = Vec::new();
                for i in &adults {
                    let b = &book[i];
                    let zi = z(b);
                    if zi > zb && frame - b.last_kid >= cooldown && (b.last_kid > 0 || frame - b.born >= min_age) {
                        parents.push((*i, ((zi - zb).floor() as u32 + 1).min(3)));
                    }
                }
                let mut cut: Vec<u32> = adults.iter().copied().filter(|i| z(&book[i]) < -zc).collect();
                if turnover > 0 && parents.is_empty() && frame - last_birth >= turnover {
                    let zs: Vec<(u32, f64)> = adults.iter().map(|i| (*i, z(&book[i]))).collect();
                    let best = zs.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then(b.0.cmp(&a.0))).unwrap().0;
                    let worst = zs.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then(a.0.cmp(&b.0))).unwrap().0;
                    if best != worst {
                        parents.push((best, 1));
                        if !cut.contains(&worst) {
                            cut.push(worst);
                        }
                    }
                }
                if turn > 0 {
                    let mut zs: Vec<(u32, f64)> = adults.iter().map(|i| (*i, z(&book[i]))).collect();
                    zs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then(a.0.cmp(&b.0)));
                    // Owed replacements carry over until a child is actually born (capped at 5, so a dry spell does
                    // not end in a burst): taken off at the moment of the birth, they were lost whenever no parent was
                    // eligible (58 of 146 due judgements in a 200k test).
                    quota = (quota + adults.len() as f64 * judge_every as f64 / turn as f64).min(5.0);
                    let q = quota.floor() as usize;
                    q_now = q;
                    cut = zs.iter().filter(|p| p.1 < -zhard).map(|p| p.0).collect();
                    let hard = cut.len();
                    hard_n = hard;
                    let extra_cut = n.saturating_sub(hi);
                    for p in &zs {
                        if cut.len() >= hard + extra_cut || p.1 >= 0.0 {
                            break;
                        }
                        if !cut.contains(&p.0) {
                            cut.push(p.0);
                        }
                    }
                    // The quota's removals wait for its births: each is made only once a child has taken its place,
                    // so a colony that cannot breed is not also emptied (p7: 36 culls against 30 births, 24 -> 7).
                    quota_cut = zs.iter().filter(|p| p.1 < 0.0 && !cut.contains(&p.0)).take(q).map(|p| p.0).collect();
                    let want = (hard + quota_cut.len() + lo.saturating_sub(n)) as u32;
                    // Breeding z: lifetime, or with fresh_kid the record since the last child.
                    let zb_of = |i: u32| -> Option<f64> {
                        let b = &book[&i];
                        if b.kids >= kid_cap || frame - b.last_kid < cooldown {
                            return None;
                        }
                        if !fresh_kid || b.kids == 0 {
                            return Some(z(b));
                        }
                        let since = frame - b.last_kid;
                        if since < min_age {
                            return None;
                        }
                        let t = since as f64 / 1000.0;
                        let (c, g2) = (b.credit - b.at_kid.0, b.g2 - b.at_kid.1);
                        Some((c / t - med) / (g2.max(rate_g2 * t).sqrt() / t).max(1e-6))
                    };
                    // Parents whose family is at its cap are passed over here, so their children go to the next in line
                    // rather than not being born (the cap otherwise stopped most births once a line did well).
                    if std::env::var("INRUN_DEBUG").is_ok() && frame % 20000 == 0 {
                        for p in &zs {
                            let b = &book[&p.0];
                            eprintln!("dbg f{frame} n{n} id{} z{:.2} zb{:?} kids{} since{} fam{}={} cap{cap_f}", p.0, p.1, zb_of(p.0), b.kids, frame - b.last_kid, b.family, fam.get(&b.family).copied().unwrap_or(0));
                        }
                    }
                    let mut okz: Vec<(u32, f64)> = zs
                        .iter()
                        .filter_map(|p| zb_of(p.0).map(|v| (p.0, v)))
                        .filter(|p| p.1 > 0.0 || (n < lo && p.1 >= 0.0))
                        .collect();
                    // The family cap steers births to other lines; it never stops them. When every eligible parent is
                    // in a capped line (a colony down to one family: seed 4, all nine ants m20), they breed anyway.
                    let open: Vec<(u32, f64)> = okz.iter().copied().filter(|p| fam.get(&book[&p.0].family).copied().unwrap_or(0) < cap_f).collect();
                    capped_only = open.is_empty();
                    // A colony below its band with no eligible parent breeds from its best ant by lifetime record:
                    // the evidence still picks which ant, the bar is not lowered to "anyone". Seed 4 sat at two ants
                    // for 120k frames because the better one's record since its last child was below the median.
                    if okz.is_empty() && n < lo {
                        if let Some(best) = zs.last() {
                            okz.push(*best);
                        }
                    }
                    if !capped_only {
                        okz = open;
                    }
                    okz.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
                    let ok: Vec<u32> = okz.into_iter().map(|p| p.0).collect();
                    let mut kids = vec![0u32; ok.len()];
                    let mut given = 0;
                    'passes: for span in [ok.len(), ok.len().div_ceil(2), ok.len().div_ceil(4)] {
                        for k in kids.iter_mut().take(span) {
                            if given >= want {
                                break 'passes;
                            }
                            *k += 1;
                            given += 1;
                        }
                    }
                    if want > 0 {
                        q_due += 1;
                        if ok.is_empty() {
                            q_noparent += 1;
                        }
                    }
                    parents = ok.into_iter().zip(kids).filter(|p| p.1 > 0).collect();
                }
                let room = (hi + 4).saturating_sub(n);
                let floor_room = n.saturating_sub(lo.saturating_sub(8));
                cut.truncate(floor_room);
                if sel == "random" {
                    // Same counts, chosen blind.
                    let kids: u32 = parents.iter().map(|p| p.1).sum();
                    let pool: Vec<u32> = adults.iter().copied().filter(|i| frame - book[i].last_kid >= cooldown || book[i].last_kid == 0).collect();
                    parents = (0..kids).filter(|_| !pool.is_empty()).map(|_| (pool[srng.below(pool.len())], 1)).collect();
                    let mut ad = adults.clone();
                    let mut c2 = Vec::new();
                    for _ in 0..cut.len().min(ad.len()) {
                        c2.push(ad.swap_remove(srng.below(ad.len())));
                    }
                    cut = c2;
                    let mut qc = Vec::new();
                    for _ in 0..quota_cut.len().min(ad.len()) {
                        qc.push(ad.swap_remove(srng.below(ad.len())));
                    }
                    quota_cut = qc;
                }
                for &id in &cut {
                    if inrun::cull(&mut w, id) {
                        culled_pending.insert(id);
                        removals_n += 1;
                        writeln!(births, "{frame},{id},0,{},{},cull", book[&id].family, book[&id].gen).unwrap();
                        book.remove(&id);
                    }
                }
                for (pid, k) in parents {
                    for _ in 0..k {
                        if births_n >= room {
                            break;
                        }
                        let Some(p) = book.get(&pid) else { break };
                        let f = p.family.clone();
                        if fam.get(&f).copied().unwrap_or(0) >= cap_f && !(turn > 0 && capped_only) {
                            break;
                        }
                        let child = train::mutate(&p.genome, &kslots, kpm, ksigma, 0.0, 1.0, &[], &mut srng);
                        let gen = p.gen + 1;
                        let placed = inrun::birth(&mut w, sp, inrun::mind(&child), kid_grants);
                        if placed.is_none() {
                            birth_fail += 1;
                        }
                        if let Some(cid) = placed {
                            births_n += 1;
                            last_birth = frame;
                            *fam.entry(f.clone()).or_default() += 1;
                            let pb = book.get_mut(&pid).unwrap();
                            pb.last_kid = frame;
                            pb.kids += 1;
                            pb.at_kid = (pb.credit, pb.g2);
                            if retire && pb.kids >= kid_cap {
                                retiring.push(pid);
                            }
                            writeln!(births, "{frame},{cid},{pid},{f},{gen},birth").unwrap();
                            writeln!(csv_ants, "{cid},{f}").unwrap();
                            book.insert(cid, Who { born: frame, credit: 0.0, g2: 0.0, family: f, gen, genome: child, last_kid: 0, kids: 0, tripped: false, at_kid: (0.0, 0.0) });
                        }
                    }
                }
                for id in std::mem::take(&mut retiring) {
                    if book.contains_key(&id) && inrun::cull(&mut w, id) {
                        culled_pending.insert(id);
                        removals_n += 1;
                        writeln!(births, "{frame},{id},0,{},{},retire", book[&id].family, book[&id].gen).unwrap();
                        book.remove(&id);
                    }
                }
                let refill = lo.saturating_sub(n) as usize;
                let paid = births_n.saturating_sub(refill + hard_n).min(q_now);
                quota -= paid as f64;
                for id in std::mem::take(&mut quota_cut).into_iter().take(births_n.saturating_sub(refill + hard_n)) {
                    if book.contains_key(&id) && inrun::cull(&mut w, id) {
                        culled_pending.insert(id);
                        removals_n += 1;
                        writeln!(births, "{frame},{id},0,{},{},cull", book[&id].family, book[&id].gen).unwrap();
                        book.remove(&id);
                    }
                }
            }
            let got: f64 = delivered.iter().map(|d| d.1).sum();
            let ate: f64 = eaten.iter().map(|d| d.1).sum();
            let fp = 1.0 + kprice * (1.0 - if ate > 0.0 { got / ate } else { 1.0 }).max(0.0);
            let cp = 1.0 + kprice * (loose_n as f64 / (2.0 * book.len().max(1) as f64)).clamp(0.0, 1.0);
            // Live ants the book does not know (must stay 0).
            let ghosts = inrun::ants(&w, sp).iter().filter(|i| !book.contains_key(i)).count();
            writeln!(sel_log, "{frame},{},{},{births_n},{removals_n},{bias:.2},{med:.3},{fp:.2},{dig_price:.2},{cp:.2},{ghosts}", book.len(), adults.len()).unwrap();
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
    if sel != "off" {
        sel_log.flush().unwrap();
        births.flush().unwrap();
        let dir = out.join("final");
        std::fs::create_dir_all(&dir).unwrap();
        let mut f = std::fs::File::create(dir.join("index.csv")).unwrap();
        writeln!(f, "id,family,gen,born,kids,score").unwrap();
        let frame = w.frame;
        for (id, b) in &book {
            train::save_genome(&dir.join(format!("{id}.raw.txt")), &b.genome).unwrap();
            writeln!(f, "{id},{},{},{},{},{:.3}", b.family, b.gen, b.born, b.kids, b.credit / ((frame - b.born) as f64 / 1000.0).max(0.1)).unwrap();
        }
    }
    if sel != "off" {
        println!("selection: quota due {q_due}, no eligible parent {q_noparent}, placements failed {birth_fail}");
    }
    println!("done: {frames} frames in {:.1}s ({:.0} frames/s)", t0.elapsed().as_secs_f64(), frames as f64 / t0.elapsed().as_secs_f64());
}
