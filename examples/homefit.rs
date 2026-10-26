//! SCRATCH (teacher lane, 2026-10-26): **write a teacher's calls into a mind genome's own weights**, on the game's
//! own `mind::decide`, for the come-home lesson. The Python copier (`copier_c.py`) mirrors a 2,797-slot brain; the
//! standard pool is 3,285 slots (dig head, last-food memory), so this fits on the real forward pass instead.
//!
//! ```text
//! cargo run --release --example homefit -- logs=a.log,b.log start=G.raw.txt out=H.raw.txt [unit=U] [lab=20000]
//!     [keep=20000] [steps=300] [batch=4000] [eps=0.05] [lr=0.05] [wkeep=1] [seed=1] [ref=G0.raw.txt] [eval=G2.raw.txt]
//! ```
//!
//! Rows are copy-log decisions (`PP_COPY_LOG`) with the teacher's label as the 16th field (`1|i;j`, the carry
//! teacher's format; `PP_CARRY=0` labels without acting). Loss: for labelled rows, -ln P(the brain picks a labelled
//! option); for the rest, KL(start's choice || new choice), so what the teacher does not ask about stays the start
//! genome's. P is the brain's two draws (`choose_weighted` over the verbs' jobs, then over the verb's options'
//! scores), read off `Decision`; the dig head's re-rank and the recall tilt are left out (neither moves a Walk).
//! Only `mind::home_lesson_slots(unit)` move: Adam on central differences, a fresh minibatch each step.
use pixel_physics::sim::creature::mind::{self, MindState};
use pixel_physics::sim::creature::options::{Opt, Verb, FEATURES, VERBS};
use pixel_physics::sim::rng::Rng;
use pixel_physics::train;
use rayon::prelude::*;
use std::io::BufRead;

struct Row {
    ctx: [f32; mind::CONTEXT],
    mem: Vec<f32>,
    last: Option<Verb>,
    opts: Vec<Opt>,
    label: Option<Vec<usize>>,
    dx: i32,
}

const K: f32 = 0.05; // mind's PICK_FLOOR

fn cw(ws: &[f32]) -> Vec<f32> {
    let b: Vec<f32> = ws.iter().map(|&s| (K + s.max(0.0)).powi(2)).collect();
    let t: f32 = b.iter().sum();
    if t <= 0.0 { vec![1.0 / ws.len() as f32; ws.len()] } else { b.iter().map(|x| x / t).collect() }
}

/// The probability of each option under `g`, or None when the brain has nothing to choose.
fn probs(g: &[f32], r: &Row) -> Option<Vec<f32>> {
    let mut state = MindState::default();
    state.mem.copy_from_slice(&r.mem);
    state.last_verb = r.last;
    let mut draw = Rng::new(1);
    let d = mind::decide(g, &r.ctx, &r.opts, &mut state, &mut draw)?;
    let present: Vec<usize> = (0..VERBS).filter(|&v| d.jobs[v].is_some()).collect();
    let pv = cw(&present.iter().map(|&v| (d.jobs[v].unwrap() + 1.0) / 2.0).collect::<Vec<_>>());
    let mut p = vec![0.0f32; r.opts.len()];
    for (k, &v) in present.iter().enumerate() {
        let idx: Vec<usize> = (0..r.opts.len()).filter(|&o| r.opts[o].verb as usize == v).collect();
        let po = cw(&idx.iter().map(|&o| (d.scores[o] + 1.0) / 2.0).collect::<Vec<_>>());
        for (j, &o) in idx.iter().enumerate() {
            p[o] = pv[k] * po[j];
        }
    }
    Some(p)
}

fn parse(line: &str) -> Option<Row> {
    let p: Vec<&str> = line.split(',').collect();
    if p.get(1) != Some(&"Q") || p[6] == "2" {
        return None;
    }
    let last: i32 = p[8].parse().ok()?;
    let ctx: Vec<f32> = p[11].split(' ').map(|x| x.parse().unwrap()).collect();
    let mem: Vec<f32> = p[12].split(' ').map(|x| x.parse().unwrap()).collect();
    let opts: Vec<Opt> = p[13]
        .split(';')
        .filter(|o| !o.is_empty())
        .map(|o| {
            let q: Vec<&str> = o.splitn(5, ':').collect();
            let mut feats = [0.0f32; FEATURES];
            for kv in q[4].split(' ').filter(|s| !s.is_empty()) {
                let (i, v) = kv.split_once('=').unwrap();
                let i: usize = i.parse().unwrap();
                if i < FEATURES {
                    feats[i] = v.parse().unwrap();
                }
            }
            Opt { verb: Verb::ALL[q[0].parse::<usize>().unwrap()], target: (q[1].parse().unwrap(), q[2].parse().unwrap()), feats }
        })
        .collect();
    let label = p.get(15).and_then(|f| f.strip_prefix("1|")).map(|l| l.split(';').map(|i| i.parse().unwrap()).collect::<Vec<usize>>());
    let dx = (p[4].parse::<i32>().ok()? - 128).abs();
    Some(Row { ctx: ctx.try_into().ok()?, mem, last: (last >= 0).then(|| Verb::ALL[last as usize]), opts, label, dx })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |k: &str| args.iter().find_map(|a| a.strip_prefix(&format!("{k}=")).map(str::to_string));
    let num = |k: &str, d: f64| arg(k).map_or(d, |v| v.parse().unwrap());
    let logs = arg("logs").expect("logs=");
    let start = arg("start").expect("start=");
    let out = arg("out").expect("out=");
    let unit = arg("unit").map(|u| u.parse::<usize>().unwrap());
    let (n_lab, n_keep) = (num("lab", 20000.0) as usize, num("keep", 20000.0) as usize);
    let (steps, batch, eps, lr, wkeep) = (num("steps", 300.0) as usize, num("batch", 4000.0) as usize, num("eps", 0.05) as f32, num("lr", 0.05) as f32, num("wkeep", 1.0) as f32);
    let mut rng = Rng::new(num("seed", 1.0) as u64);

    // Reservoir-sample each group so a multi-gigabyte log never sits in memory.
    let (mut lab, mut keep): (Vec<Row>, Vec<Row>) = (Vec::new(), Vec::new());
    let (mut seen_l, mut seen_k) = (0usize, 0usize);
    for path in logs.split(',') {
        for line in std::io::BufReader::new(std::fs::File::open(path).unwrap()).lines() {
            let line = line.unwrap();
            let Some(r) = parse(&line) else { continue };
            let (v, seen, cap) = if r.label.is_some() { (&mut lab, &mut seen_l, n_lab) } else { (&mut keep, &mut seen_k, n_keep) };
            *seen += 1;
            if v.len() < cap {
                v.push(r);
            } else {
                let j = (rng.unit_f32() as f64 * *seen as f64) as usize;
                if j < cap {
                    v[j] = r;
                }
            }
        }
    }
    eprintln!("rows: labelled {} of {seen_l}, kept {} of {seen_k}", lab.len(), keep.len());

    let text = std::fs::read_to_string(&start).unwrap();
    let len = text.strip_prefix("genome ").and_then(|t| t.split_whitespace().next()).and_then(|n| n.parse().ok()).unwrap();
    let mut g = train::parse_genome(&text, len, &start);
    // A start saved before the context gates gains them at zero (`mind::enter_gate`), so slots are today's.
    mind::enter_gate(&mut g, train::genome_text_layout(&text, &start));
    let len = g.len();
    let g0 = g.clone();
    // `wide=1` (with `unit=U`): the unit's whole context and feature rows move, not just its home links -- the
    // narrow set plateaued at P(label) ~0.41 on the fitted ant's own situations (DAgger round 1, 2026-10-26).
    // `feat=F`: only option feature F's weight in every scorer unit (e.g. 18, `PP_HOMELOW`'s "home, when low").
    let mut slots = match arg("feat").map(|f| f.parse::<usize>().unwrap()) {
        Some(f) => (0..mind::UNITS).map(|u| mind::unit_rows(u)[mind::unit_rows(u).len() - FEATURES + f]).collect(),
        None => mind::home_lesson_slots(unit),
    };
    if let (Some(u), true) = (unit, arg("wide").is_some_and(|v| v == "1")) {
        slots.extend(mind::unit_rows(u));
        slots.sort_unstable();
        slots.dedup();
    }
    // SCRATCH (brain-design review, 2026-10-26): `rows=all|feat` (with `unit=U`) trains only that unit -- `all` its
    // context and feature rows, `feat` its feature row -- and its output weight; `fresh=1` first zeroes the unit's
    // context, feature and verb rows (the KL hold stays on the start as loaded, so the fit must rebuild what the unit
    // did); `gate=1` trains unit U's context gate row too (`mind::gate_row`).
    if let (Some(u), Some(rows)) = (unit, arg("rows")) {
        let r = mind::unit_rows(u);
        slots = if rows == "feat" { r[r.len() - FEATURES..].to_vec() } else { r.clone() };
        slots.push(mind::scorer_output(u));
        // `drop=F`: never train option feature F of the unit (18, `PP_HOMELOW`'s slot, so a plain-senses arm cannot
        // read the instinct the logs carry).
        if let Some(f) = arg("drop").map(|f| f.parse::<usize>().unwrap()) {
            let s = r[r.len() - FEATURES + f];
            slots.retain(|&x| x != s);
        }
    }
    if let (Some(u), true) = (unit, arg("fresh").is_some_and(|v| v == "1")) {
        // Every slot that feeds the unit -- its context, feature AND verb rows (`mind::scorer_unit`): a verb row left
        // live is a per-verb constant that the unit's output weight carries into every score as that weight grows.
        for s in (0..mind::GENOME_LEN).filter(|&s| mind::scorer_unit(s) == Some(u)) {
            g[s] = 0.0;
        }
        g[mind::scorer_output(u)] = 0.0;
        eprintln!("unit {u}: zeroed (fresh)");
    }
    if let (Some(u), true) = (unit, arg("gate").is_some_and(|v| v == "1")) {
        slots.extend(mind::gate_row(u));
        eprintln!("unit {u}: gated");
    }
    // `init=PATH` (lines `slot value`, today's slots, gate rows included): set these weights
    // after `fresh` and `gate`, so a fit or search can start from a hand-laid solution.
    if let Some(p) = arg("init") {
        for l in std::fs::read_to_string(&p).unwrap().lines().filter(|l| !l.is_empty()) {
            let (i, w) = l.split_once(' ').unwrap();
            g[i.parse::<usize>().unwrap()] = w.parse().unwrap();
        }
        eprintln!("init: {p}");
    }
    if let Some(u) = unit {
        let o = mind::scorer_output(u);
        eprintln!("unit {u}: output weight {} (seeded 0.1 if 0)", g[o]);
        if g[o] == 0.0 {
            g[o] = 0.1;
        }
    }
    eprintln!("training {} slots", slots.len());
    // `ref=G`: the genome unlabelled rows are held to (default the start). A refit from an earlier fit's genome, on
    // that fit's own situations, keeps the ORIGINAL ant's choices there rather than the earlier fit's leak.
    let g_ref = arg("ref").map_or_else(
        || g0.clone(),
        |r| {
            let t = std::fs::read_to_string(&r).unwrap();
            let mut h = train::parse_genome(&t, train::genome_text_len(&t, &r), &r);
            mind::enter_gate(&mut h, train::genome_text_layout(&t, &r));
            assert_eq!(h.len(), len, "ref= {r}: not the start's shape");
            h
        },
    );
    let p_ref: Vec<Option<Vec<f32>>> = keep.par_iter().map(|r| probs(&g_ref, r)).collect();

    let loss = |g: &[f32], li: &[usize], ki: &[usize]| -> (f32, f32) {
        let l: f32 = li
            .par_iter()
            .map(|&i| {
                let r = &lab[i];
                probs(g, r).map_or(0.0, |p| -(r.label.as_ref().unwrap().iter().map(|&o| p.get(o).copied().unwrap_or(0.0)).sum::<f32>().max(1e-6)).ln())
            })
            .sum::<f32>()
            / li.len().max(1) as f32;
        let k: f32 = ki
            .par_iter()
            .map(|&i| match (&p_ref[i], probs(g, &keep[i])) {
                (Some(a), Some(b)) => a.iter().zip(&b).filter(|(x, _)| **x > 0.0).map(|(x, y)| x * (x / y.max(1e-6)).ln()).sum::<f32>(),
                _ => 0.0,
            })
            .sum::<f32>()
            / ki.len().max(1) as f32;
        (l, k)
    };
    let all_l: Vec<usize> = (0..lab.len()).collect();
    let all_k: Vec<usize> = (0..keep.len()).collect();
    let (l0, k0) = loss(&g, &all_l, &all_k);
    let agree = |g: &[f32]| -> f32 {
        lab.par_iter().map(|r| probs(g, r).map_or(0.0, |p| r.label.as_ref().unwrap().iter().map(|&o| p.get(o).copied().unwrap_or(0.0)).sum::<f32>())).sum::<f32>() / lab.len().max(1) as f32
    };
    // `eval=G2`: no fit; P(label) for the start genome and G2 by columns from home (the nest is column 128), to see
    // where a fit's agreement lands -- a mean can rise on easy rows near home while the wall rows stay at zero.
    if let Some(e) = arg("eval") {
        let t2 = std::fs::read_to_string(&e).unwrap();
        let g2 = train::parse_genome(&t2, len, &e);
        for (lo, hi) in [(0, 80), (80, 160), (160, 240), (240, 999)] {
            let rows: Vec<&Row> = lab.iter().filter(|r| r.dx >= lo && r.dx < hi).collect();
            let pl = |g: &[f32]| -> (f32, f32) {
                let ps: Vec<f32> = rows.iter().map(|r| probs(g, r).map_or(0.0, |p| r.label.as_ref().unwrap().iter().map(|&o| p.get(o).copied().unwrap_or(0.0)).sum::<f32>())).collect();
                (ps.iter().sum::<f32>() / ps.len().max(1) as f32, ps.iter().filter(|&&x| x > 0.5).count() as f32 / ps.len().max(1) as f32)
            };
            let ((a, af), (b, bf)) = (pl(&g0), pl(&g2));
            eprintln!("dx {lo}-{hi}: rows {} | start P {a:.3} (>0.5 {af:.2}) | eval P {b:.3} (>0.5 {bf:.2})", rows.len());
        }
        return;
    }
    // SCRATCH (brain-design review): P(label) = P(a labelled option's verb) x P(the labelled target | that verb), so a
    // plateau can be read as the job head's or the step scorer's.
    let verb_split = |g: &[f32]| -> String {
        let (mut pv, mut pl, mut n) = (0.0f32, 0.0f32, 0usize);
        for r in &lab {
            let Some(p) = probs(g, r) else { continue };
            let l = r.label.as_ref().unwrap();
            let vs: Vec<Verb> = l.iter().filter_map(|&o| r.opts.get(o).map(|x| x.verb)).collect();
            pv += (0..r.opts.len()).filter(|&o| vs.contains(&r.opts[o].verb)).map(|o| p[o]).sum::<f32>();
            pl += l.iter().map(|&o| p.get(o).copied().unwrap_or(0.0)).sum::<f32>();
            n += 1;
        }
        format!("P(label's verb) {:.4}, P(label) {:.4}, P(label | verb) {:.4}", pv / n as f32, pl / n as f32, pl / pv.max(1e-9))
    };
    eprintln!("start: label loss {l0:.4} keep KL {k0:.4} P(label) {:.4}; {}", agree(&g), verb_split(&g));

    // SCRATCH (brain-design review, 2026-10-26): `es=N` -- instead of the gradient fit, a (1+1) mutation search on the
    // same loss over a FIXED batch (`batch` rows of each group, so a child is judged on the rows its parent was): each
    // child moves `es_k` of the trained slots by `sigma` x the in-run step (uniform, sd 1), and is kept if no worse.
    // A proxy for "can mutation plus selection find this lesson in these slots", free of any optimiser step budget.
    if let Some(n) = arg("es").map(|v| v.parse::<usize>().unwrap()) {
        let (k, sigma) = (num("es_k", 1.0) as usize, num("sigma", 1.0) as f32);
        let mut trng = train::Rng(num("seed", 1.0) as u64 ^ 0x5EED);
        let li: Vec<usize> = (0..batch.min(lab.len())).collect();
        let ki: Vec<usize> = (0..batch.min(keep.len())).collect();
        let f = |g: &[f32]| {
            let (l, k) = loss(g, &li, &ki);
            l + wkeep * k
        };
        let mut best = f(&g);
        let mut kept = 0;
        for it in 1..=n {
            let mut c = g.clone();
            for _ in 0..k {
                let s = slots[trng.below(slots.len())];
                c[s] = (c[s] + sigma * trng.step()).clamp(-40.0, 40.0);
            }
            let fc = f(&c);
            if fc <= best {
                (g, best) = (c, fc);
                kept += 1;
            }
            if it % (n / 10).max(1) == 0 {
                let (l, kl) = loss(&g, &all_l, &all_k);
                eprintln!("es {it}: kept {kept} label loss {l:.4} keep KL {kl:.4} P(label) {:.4}", agree(&g));
            }
        }
        train::save_genome(std::path::Path::new(&out), &g).unwrap();
        let big = slots.iter().map(|&s| g[s].abs()).fold(0.0f32, f32::max);
        eprintln!("es trained weights: largest |w| {big:.2}; {}", verb_split(&g));
        return;
    }
    let (mut m, mut v) = (vec![0.0f32; slots.len()], vec![0.0f32; slots.len()]);
    let (b1, b2) = (0.9f32, 0.999f32);
    for step in 1..=steps {
        let li: Vec<usize> = (0..batch.min(lab.len())).map(|_| (rng.unit_f32() * lab.len() as f32) as usize % lab.len().max(1)).collect();
        let ki: Vec<usize> = (0..batch.min(keep.len())).map(|_| (rng.unit_f32() * keep.len() as f32) as usize % keep.len().max(1)).collect();
        let f = |g: &[f32]| {
            let (l, k) = loss(g, &li, &ki);
            l + wkeep * k
        };
        let mut grad = vec![0.0f32; slots.len()];
        for (j, &s) in slots.iter().enumerate() {
            let w = g[s];
            g[s] = w + eps;
            let up = f(&g);
            g[s] = w - eps;
            let dn = f(&g);
            g[s] = w;
            grad[j] = (up - dn) / (2.0 * eps);
        }
        for (j, &s) in slots.iter().enumerate() {
            m[j] = b1 * m[j] + (1.0 - b1) * grad[j];
            v[j] = b2 * v[j] + (1.0 - b2) * grad[j] * grad[j];
            let mh = m[j] / (1.0 - b1.powi(step as i32));
            let vh = v[j] / (1.0 - b2.powi(step as i32));
            g[s] -= lr * mh / (vh.sqrt() + 1e-8);
        }
        if step % 25 == 0 || step == steps {
            let (l, k) = loss(&g, &all_l, &all_k);
            eprintln!("step {step}: label loss {l:.4} keep KL {k:.4} P(label) {:.4}", agree(&g));
            train::save_genome(std::path::Path::new(&out), &g).unwrap();
        }
    }
    let w0 = |s: usize| g0.get(s).copied().unwrap_or(0.0);
    let moved: Vec<String> = slots.iter().filter(|&&s| g[s] != w0(s)).map(|&s| format!("{s}:{:.3}->{:.3}", w0(s), g[s])).collect();
    let big = slots.iter().map(|&s| g[s].abs()).fold(0.0f32, f32::max);
    let mut mags: Vec<f32> = slots.iter().map(|&s| g[s].abs()).filter(|&x| x > 0.0).collect();
    mags.sort_by(|a, b| a.partial_cmp(b).unwrap());
    eprintln!("trained weights: largest |w| {big:.2}, median {:.2}, over {} slots; {}", mags.get(mags.len() / 2).copied().unwrap_or(0.0), slots.len(), verb_split(&g));
    eprintln!("moved {} slots: {}", moved.len(), moved.join(" "));
}
