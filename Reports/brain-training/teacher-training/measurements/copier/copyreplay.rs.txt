//! SCRATCH (teacher-training step 1, not for main): the copier's reference. Re-runs the brain's own `mind::decide` on
//! the states in a copy log, with ANY genome, and writes every option's score and every verb's job score -- so the
//! Python forward pass can be checked against the real thing on a genome it was not fitted to.
//!
//! cargo run --release --example copyreplay -- <copy log> <genome as loaded> <out> [max rows]
//!
//! The genome is read raw (`train::load_genome`), not through a challenge: it must be exactly the vector the copier
//! holds. Out: one line per Q row, `tag,frame,id|score score ...|job job ...` (`nan` for a verb with no job).
use pixel_physics::sim::creature::mind::{self, MindState};
use pixel_physics::sim::creature::options::{Opt, Verb, FEATURES};
use pixel_physics::sim::rng::Rng;
use pixel_physics::train;
use std::io::{BufRead, Write};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (log, genome, out) = (&a[0], &a[1], &a[2]);
    let max: usize = a.get(3).map_or(usize::MAX, |m| m.parse().unwrap());
    let g = train::load_genome(std::path::Path::new(genome), mind::GENOME_LEN);
    let mut w = std::io::BufWriter::new(std::fs::File::create(out).unwrap());
    let mut done = 0;
    for line in std::io::BufReader::new(std::fs::File::open(log).unwrap()).lines() {
        let line = line.unwrap();
        let p: Vec<&str> = line.split(',').collect();
        if p.get(1) != Some(&"Q") {
            continue;
        }
        // tag,Q,frame,id,x,y,settle,pick,last,carrying,labels,ctx,mem0,opts
        let last: i32 = p[8].parse().unwrap();
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
                    feats[i.parse::<usize>().unwrap()] = v.parse().unwrap();
                }
                Opt { verb: Verb::ALL[q[0].parse::<usize>().unwrap()], target: (q[1].parse().unwrap(), q[2].parse().unwrap()), feats }
            })
            .collect();
        let mut state = MindState::default();
        state.mem.copy_from_slice(&mem);
        state.last_verb = (last >= 0).then(|| Verb::ALL[last as usize]);
        let ctx: [f32; mind::CONTEXT] = ctx.try_into().unwrap();
        let mut draw = Rng::new(1);
        let Some(d) = mind::decide(&g, &ctx, &opts, &mut state, &mut draw) else { continue };
        let s: Vec<String> = d.scores.iter().map(|x| format!("{x}")).collect();
        let j: Vec<String> = d.jobs.iter().map(|x| x.map_or("nan".into(), |v| format!("{v}"))).collect();
        writeln!(w, "{},{},{}|{}|{}", p[0], p[2], p[3], s.join(" "), j.join(" ")).unwrap();
        done += 1;
        if done >= max {
            break;
        }
    }
    println!("replayed {done} decisions into {out}");
}
