//! **The training rig's command line** (`pixel_physics::train`): train an ant
//! brain on one arena, or on a suite of them at once.
//!
//! ```text
//! cargo run --release --example train -- challenge=forage start=zero gens=30 out=runs/forage-1
//! cargo run --release --example train -- challenge=trail start=zero seed=2
//! cargo run --release --example train -- challenge=dig start=runs/forage-1/best.txt
//! cargo run --release --example train -- challenge=suite arenas=forage,trail keep=forage start=runs/forage-1/best.txt
//! cargo run --release --example train -- challenge=forage cost=1        # time one arena and stop
//! cargo run --release --example train -- challenge=dig score=runs/x/best.txt   # hold-out score one genome
//! ```
//!
//! `start=zero|random|game|<file>`: `zero` is from scratch (every brain
//! weight 0; the run seed decides which mutations come first), `random` draws
//! every trainable weight from N(0, 0.5), `game` (or `species`) is the brain
//! `ant.ron` ships. `walk=game` keeps the shipped walk instead of letting the
//! brain steer. `clones=1` makes the trail arena select whole arenas of
//! clones, as its hold-out scores them. `targets=zero|hand|own|<file>` (dig)
//! is the target-choice arena: genomes carry an evolved target scorer after
//! the brain (`creature::targets`), started blank, from the hand-built one,
//! from the start (or score) file's own, or from a file of weights; the
//! hand rules that pick the dig cell and the spoil site are off, and only
//! the scorer evolves. `free=holds,cue,face,company` hands the scorer those
//! hand filters too, one rung of the ladder at a time. Every argument is
//! echoed; an unknown one stops the run.

use pixel_physics::sim::creature;
use pixel_physics::train::challenges::{Dig, Forage, Suite, Trail};
use pixel_physics::train::{self, Challenge, Settings, Start};
use std::path::PathBuf;
use std::time::Instant;

fn arena(name: &str, frames: Option<u64>, ants: Option<usize>, game_walk: bool, clones: bool, targets: bool, free: creature::targets::Free) -> Box<dyn Challenge> {
    match name {
        "forage" => Box::new(Forage { ants: ants.unwrap_or(12), frames: frames.unwrap_or(6000), heap: 300, game_walk }),
        "trail" => {
            if game_walk {
                println!("note: trail always walks by the brain (Chooser::Off); walk=game does not apply to it");
            }
            Box::new(Trail { ants: ants.unwrap_or(6), frames: frames.unwrap_or(6000), clones })
        }
        "dig" => Box::new(Dig { ants: ants.unwrap_or(12), frames: frames.unwrap_or(20_000), game_walk, targets, free }),
        _ => panic!("challenge={name}: forage|trail|dig|suite"),
    }
}

fn main() {
    creature::mutation_off_for_measuring();
    let mut set = Settings::default();
    let mut challenge = "forage".to_string();
    let mut arenas = String::new();
    let mut keep = String::new();
    let mut floor = 0.8;
    let mut start = "zero".to_string();
    let mut out: Option<PathBuf> = None;
    let mut frames = None;
    let mut ants = None;
    let mut game_walk = false;
    let mut clones = false;
    let mut cost = false;
    let mut score: Option<String> = None;
    let mut noise = false;
    let mut targets = String::new();
    let mut pic: Option<PathBuf> = None;
    let mut free = String::new();
    for a in std::env::args().skip(1) {
        let (k, v) = a.split_once('=').unwrap_or((a.as_str(), "1"));
        match k {
            "challenge" => challenge = v.into(),
            "arenas" => arenas = v.into(),
            "keep" => keep = v.into(),
            "floor" => floor = v.parse().unwrap(),
            "start" => start = v.into(),
            "out" => out = Some(v.into()),
            "gens" => set.gens = v.parse().unwrap(),
            "pop" => set.pop = v.parse().unwrap(),
            "layouts" => set.layouts = v.parse().unwrap(),
            "pm" => set.pm = v.parse().unwrap(),
            "sigma" => set.sigma = v.parse().unwrap(),
            "cross" => set.cross = v.parse().unwrap(),
            "holdout" => set.holdout = v.parse().unwrap(),
            "every" => set.every = v.parse().unwrap(),
            "seed" => set.seed = v.parse().unwrap(),
            "threads" => set.threads = v.parse().unwrap(),
            "pics" => set.pictures = v == "1",
            "frames" => frames = Some(v.parse().unwrap()),
            "ants" => ants = Some(v.parse().unwrap()),
            "walk" => game_walk = v == "game",
            "clones" => clones = v == "1",
            "cost" => cost = v == "1",
            "score" => score = Some(v.into()),
            "noise" => noise = v == "1",
            "targets" => targets = v.into(),
            "pic" => pic = Some(v.into()),
            "free" => free = v.into(),
            _ => panic!("unknown argument {a}"),
        }
    }
    // **Every setting, every time**, score and noise runs included: a log
    // without it once left two "game" rows in one file with nothing to say
    // which walk made which (`design/review-rev2.md` N10).
    println!(
        "train challenge={challenge} arenas={arenas} keep={keep} floor={floor} start={start} walk={} clones={clones} frames={frames:?} ants={ants:?} score={score:?} noise={noise} targets={targets} free={free} pic={pic:?} seed={} pop={} gens={} layouts={} pm={} sigma={} cross={} holdout={} threads={}",
        if game_walk { "game" } else { "brain" },
        set.seed,
        set.pop,
        set.gens,
        set.layouts,
        set.pm,
        set.sigma,
        set.cross,
        set.holdout,
        set.threads
    );

    let mut ch: Box<dyn Challenge> = if challenge == "suite" {
        let list: Vec<(Box<dyn Challenge>, bool)> = arenas
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|n| (arena(n, None, ants, game_walk, clones, !targets.is_empty(), creature::targets::Free::parse(&free)), keep.split(',').any(|k| k == n)))
            .collect();
        assert!(!list.is_empty(), "challenge=suite needs arenas=a,b,...");
        Box::new(Suite::new(list, floor))
    } else {
        arena(&challenge, frames, ants, game_walk, clones, !targets.is_empty(), creature::targets::Free::parse(&free))
    };
    let scorer: Option<Vec<f32>> = match targets.as_str() {
        "" => None,
        "zero" => Some(vec![0.0; creature::targets::WEIGHTS]),
        "hand" => Some(creature::targets::hand_built()),
        // The genome file's own scorer (`train::start_genome`).
        "own" => Some(vec![]),
        // A scorer file: one weight per line, `creature::targets::WEIGHTS`
        // of them, in its slot order.
        f => {
            let text = std::fs::read_to_string(f).unwrap_or_else(|e| panic!("targets={f}: zero|hand|<file> ({e})"));
            let w: Vec<f32> = text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')).map(|l| l.trim().parse().expect("one weight per line")).collect();
            assert_eq!(w.len(), creature::targets::WEIGHTS, "targets={f}: wrong weight count");
            Some(w)
        }
    };
    assert_eq!(scorer.is_some(), ch.scorer(), "targets= is for the dig arena (or a suite holding it)");
    assert!(free.is_empty() || scorer.is_some(), "free= needs targets=");

    let probe = pixel_physics::lab::scene::LabBox { colonies: 0, founders: 0, ..Default::default() }.build();
    let species_genome = probe.species.get(probe.species.id_of("ant").expect("ant")).genome.clone();
    let st = match start.as_str() {
        "zero" => Start::Zero,
        "random" => Start::Random(0.5),
        "species" | "game" => Start::Species,
        f => Start::File(f.into()),
    };
    // With `targets=own`, a `start=` that is not a file has no scorer of its
    // own, so it starts beside a blank one (the score modes' "start" row).
    let blank = vec![0.0; creature::targets::WEIGHTS];
    let start_scorer = match &scorer {
        Some(s) if s.is_empty() && !matches!(st, Start::File(_)) => Some(blank.as_slice()),
        s => s.as_deref(),
    };
    let g0 = train::start_genome(&st, &species_genome, ch.banned(), set.seed, start_scorer);

    // **A picture of one genome on the first hold-out layout**, without
    // training: start= (or score=) picks the genome. For looking before
    // measuring.
    if let Some(p) = &pic {
        let g = match &score {
            Some(f) if f == "game" => train::start_genome(&Start::Species, &species_genome, ch.banned(), 0, scorer.as_deref()),
            Some(f) => train::start_genome(&Start::File(f.into()), &species_genome, ch.banned(), 0, scorer.as_deref()),
            None => g0.clone(),
        };
        train::picture(ch.as_ref(), &g, train::holdout_layouts(1)[0], p);
        println!("picture in {}", p.display());
        return;
    }
    if cost {
        let t = Instant::now();
        let gs: Vec<&[f32]> = vec![&g0; if ch.unit() == train::Unit::Ant { ch.ants() } else { 1 }];
        let s = ch.run(train::Layout { seed: 1, k: 0, blind: false, legal: false }, &gs, &mut |_, _| {});
        println!("one arena: {:.2} s; {:?}", t.elapsed().as_secs_f64(), train::mean_score(&s));
        return;
    }
    if noise {
        // **The noise bar**: one fixed genome (score=, else start=) on every
        // hold-out layout, one row each, then the spread. A difference
        // between two brains smaller than this spread is not a difference.
        let g = match &score {
            Some(f) if f == "game" => train::start_genome(&Start::Species, &species_genome, ch.banned(), 0, scorer.as_deref()),
            Some(f) => train::start_genome(&Start::File(f.into()), &species_genome, ch.banned(), 0, scorer.as_deref()),
            None => g0.clone(),
        };
        let hold = train::holdout_layouts(set.holdout);
        let each = train::score_each(ch.as_ref(), &g, &hold, set.threads);
        for (l, s) in hold.iter().zip(&each) {
            println!("NOISE seed={} k={} {:.3}  {:?}", l.seed, l.k, s.fit, s.parts);
        }
        let n = each.len().max(1) as f64;
        let mean = each.iter().map(|s| s.fit).sum::<f64>() / n;
        let sd = (each.iter().map(|s| (s.fit - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0)).sqrt();
        let lo = each.iter().map(|s| s.fit).fold(f64::INFINITY, f64::min);
        let hi = each.iter().map(|s| s.fit).fold(f64::NEG_INFINITY, f64::max);
        println!("NOISE mean {mean:.3} sd {sd:.3} min {lo:.3} max {hi:.3} over {} layouts; standard error of the mean {:.3}", each.len(), sd / n.sqrt());
        return;
    }
    if let Some(f) = score {
        let st = if f == "game" { Start::Species } else { Start::File(f.into()) };
        let g = train::start_genome(&st, &species_genome, ch.banned(), 0, scorer.as_deref());
        println!("(scored with this challenge's bans applied: {:?})", ch.banned());
        let hold = train::holdout_layouts(set.holdout);
        for (who, g, lay) in train::controls(ch.as_ref(), &g, &g0, &hold) {
            let s = train::score_clones(ch.as_ref(), &g, &lay, set.threads);
            println!("HOLDOUT {who:12} {:.3}  {:?}  {:?}", s.fit, ch.parts(), s.parts);
        }
        return;
    }
    // The suite's references are its start genome's own scores, so it is
    // calibrated after the start genome exists and before training -- on its
    // own layouts, never the hold-out ones (`train::calibration_layouts`).
    ch.calibrate(&g0, &train::calibration_layouts(set.holdout), set.threads);
    let out = out.unwrap_or_else(|| PathBuf::from(format!("train-{challenge}-s{}", set.seed)));
    train::train(ch.as_ref(), g0, &set, &out).expect("write run files");
    println!("run files in {}", out.display());
}
