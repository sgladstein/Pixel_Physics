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
//! hand filters too, one rung of the ladder at a time. `sealed=1` puts
//! `score=` and `noise=1` on the sealed hold-out (`train::sealed_layouts`)
//! instead of the one training reports. `genes=walk` gives every genome the
//! walk genes (`train::WALK`: the walk's gains and trail planes, and the
//! scout's reserve), started at the shipped walk or the start file's own,
//! and evolved beside the brain; `walkset=reserve:0.6,scout:-1` sets some
//! of them in the start (or score) genome, by `train::WALK_NAMES`.
//! `prune=0.3` lets a mutation set a live weight to 0 (`Settings::prune`).
//! `challenge=forage-far` puts the heap 120-200 columns out on a wider bed,
//! 12,000 frames unless set: the near distances never show a scout's
//! reserve as a leash, so `arenas=forage,forage-far` is how a reserve is
//! trained (`challenges::Forage::far`).
//! `heap=` (forage) is the heap's size, 300 unless set and never more: a
//! bigger heap slumps into home at the near distances and into the wall at
//! the far ones. `forget=2` is how many standard errors under its
//! reference a suite's kept arena may fall before it counts as forgotten.
//! Every argument is echoed; an unknown one stops the run.

use pixel_physics::sim::creature;
use pixel_physics::train::challenges::{Dig, Forage, Suite, Trail};
use pixel_physics::train::{self, Challenge, Settings, Start};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Copy)]
struct Arena {
    frames: Option<u64>,
    ants: Option<usize>,
    heap: i32,
    game_walk: bool,
    clones: bool,
    targets: bool,
    free: creature::targets::Free,
}

fn arena(name: &str, a: &Arena) -> Box<dyn Challenge> {
    let Arena {
        frames,
        ants,
        heap,
        game_walk,
        clones,
        targets,
        free,
    } = *a;
    match name {
        "forage" | "forage-far" => Box::new(Forage {
            ants: ants.unwrap_or(12),
            // Far food first comes home at 2,600-6,200 frames, against
            // 1,000-1,250 at 70 columns (`ladder/reserve-leash-check.md`).
            frames: frames.unwrap_or(if name == "forage" { 6000 } else { 12_000 }),
            heap,
            game_walk,
            far: name == "forage-far",
        }),
        "trail" => {
            if game_walk {
                println!("note: trail always walks by the brain (Chooser::Off); walk=game does not apply to it");
            }
            Box::new(Trail {
                ants: ants.unwrap_or(6),
                frames: frames.unwrap_or(6000),
                clones,
            })
        }
        "dig" => Box::new(Dig {
            ants: ants.unwrap_or(12),
            frames: frames.unwrap_or(20_000),
            game_walk,
            targets,
            free,
        }),
        _ => panic!("challenge={name}: forage|forage-far|trail|dig|suite"),
    }
}

fn main() {
    creature::mutation_off_for_measuring();
    let mut set = Settings::default();
    let mut challenge = "forage".to_string();
    let mut arenas = String::new();
    let mut keep = String::new();
    let mut forget = 2.0;
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
    let mut sealed = false;
    let mut genes = String::new();
    let mut heap = 300;
    let mut walkset = String::new();
    for a in std::env::args().skip(1) {
        let (k, v) = a.split_once('=').unwrap_or((a.as_str(), "1"));
        match k {
            "challenge" => challenge = v.into(),
            "arenas" => arenas = v.into(),
            "keep" => keep = v.into(),
            "forget" => forget = v.parse().unwrap(),
            // The suite's forget floor was a ratio read per layout; it is
            // now `forget` standard errors under the folded reference
            // (`challenges::Suite`), so an old `floor=` would mean nothing.
            "floor" => panic!(
                "floor= is gone: a kept arena is forgotten at forget= standard errors (default 2) under its reference"
            ),
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
            "sealed" => sealed = v == "1",
            "genes" => genes = v.into(),
            "walkset" => walkset = v.into(),
            "prune" => set.prune = v.parse().unwrap(),
            "heap" => heap = v.parse().unwrap(),
            _ => panic!("unknown argument {a}"),
        }
    }
    // **Every setting, every time**, score and noise runs included: a log
    // without it once left two "game" rows in one file with nothing to say
    // which walk made which (`design/review-rev2.md` N10).
    println!(
        "train challenge={challenge} arenas={arenas} keep={keep} forget={forget} start={start} walk={} clones={clones} frames={frames:?} ants={ants:?} score={score:?} noise={noise} targets={targets} free={free} pic={pic:?} sealed={sealed} heap={heap} genes={genes} walkset={walkset} prune={} seed={} pop={} gens={} layouts={} pm={} sigma={} cross={} holdout={} threads={}",
        if game_walk { "game" } else { "brain" },
        set.prune,
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

    // The genome's scorer goes into every world it is cloned into, so a
    // suite would carry it into the forage and trail worlds too (coordinator
    // review of PR 709). Stage 1 is a dig question; refuse rather than mix.
    assert!(
        challenge != "suite" || targets.is_empty(),
        "targets= is for challenge=dig; a suite would put the scorer in every arena's world"
    );
    let spec = Arena {
        frames,
        ants,
        heap,
        game_walk,
        clones,
        targets: !targets.is_empty(),
        free: creature::targets::Free::parse(&free),
    };
    let mut ch: Box<dyn Challenge> = if challenge == "suite" {
        let list: Vec<(Box<dyn Challenge>, bool)> = arenas
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|n| {
                (
                    arena(n, &Arena { frames: None, ..spec }),
                    keep.split(',').any(|k| k == n),
                )
            })
            .collect();
        assert!(!list.is_empty(), "challenge=suite needs arenas=a,b,...");
        Box::new(Suite::new(list, forget))
    } else {
        arena(&challenge, &spec)
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
            let w: Vec<f32> = text
                .lines()
                .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
                .map(|l| l.trim().parse().expect("one weight per line"))
                .collect();
            assert_eq!(w.len(), creature::targets::WEIGHTS, "targets={f}: wrong weight count");
            Some(w)
        }
    };
    assert_eq!(scorer.is_some(), ch.scorer(), "targets= is for the dig arena");
    assert!(free.is_empty() || scorer.is_some(), "free= needs targets=");
    assert!(genes.is_empty() || genes == "walk", "genes={genes}: walk");
    let walk = genes == "walk";
    assert!(walkset.is_empty() || walk, "walkset= needs genes=walk");
    // Only the game walk reads the walk genes and the reserve
    // (`creature::chooser_step`); the brain's own walk steps by `step_chain`,
    // so they would evolve inert (review of PR 711, N2).
    assert!(
        !walk || game_walk,
        "genes=walk needs walk=game: the walk genes are read only on the game walk"
    );
    // `walkset=` written into a genome's walk block: every genome this run
    // starts from or scores, so `score=` with it measures that setting.
    let set_walk = |mut g: Vec<f32>| -> Vec<f32> {
        if let Some(at) = train::blocks(g.len()).walk {
            for part in walkset.split(',').filter(|p| !p.is_empty()) {
                let (name, v) = part
                    .split_once(':')
                    .unwrap_or_else(|| panic!("walkset={walkset}: name:value"));
                let k = train::WALK_NAMES
                    .iter()
                    .position(|n| *n == name)
                    .unwrap_or_else(|| panic!("walkset: {name} is not one of {:?}", train::WALK_NAMES));
                g[at + k] = v.parse().unwrap();
            }
        }
        g
    };

    let probe = pixel_physics::lab::scene::LabBox {
        colonies: 0,
        founders: 0,
        ..Default::default()
    }
    .build();
    let species_genome = probe
        .species
        .get(probe.species.id_of("ant").expect("ant"))
        .genome
        .clone();
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
    let g0 = set_walk(train::start_genome(
        &st,
        &species_genome,
        ch.banned(),
        set.seed,
        start_scorer,
        walk,
    ));

    // **A picture of one genome on the first hold-out layout**, without
    // training: start= (or score=) picks the genome. For looking before
    // measuring.
    if let Some(p) = &pic {
        let g = match &score {
            Some(f) if f == "game" => set_walk(train::start_genome(
                &Start::Species,
                &species_genome,
                ch.banned(),
                0,
                scorer.as_deref(),
                walk,
            )),
            Some(f) => set_walk(train::start_genome(
                &Start::File(f.into()),
                &species_genome,
                ch.banned(),
                0,
                scorer.as_deref(),
                walk,
            )),
            None => g0.clone(),
        };
        train::picture(ch.as_ref(), &g, train::holdout_layouts(1)[0], p);
        println!("picture in {}", p.display());
        return;
    }
    if cost {
        let t = Instant::now();
        let gs: Vec<&[f32]> = vec![&g0; if ch.unit() == train::Unit::Ant { ch.ants() } else { 1 }];
        let s = ch.run(
            train::Layout {
                seed: 1,
                k: 0,
                blind: false,
                legal: false,
            },
            &gs,
            &mut |_, _| {},
        );
        println!(
            "one arena: {:.2} s; {:?}",
            t.elapsed().as_secs_f64(),
            train::mean_score(&s)
        );
        return;
    }
    if noise {
        // **The noise bar**: one fixed genome (score=, else start=) on every
        // hold-out layout, one row each, then the spread. A difference
        // between two brains smaller than this spread is not a difference.
        let g = match &score {
            Some(f) if f == "game" => set_walk(train::start_genome(
                &Start::Species,
                &species_genome,
                ch.banned(),
                0,
                scorer.as_deref(),
                walk,
            )),
            Some(f) => set_walk(train::start_genome(
                &Start::File(f.into()),
                &species_genome,
                ch.banned(),
                0,
                scorer.as_deref(),
                walk,
            )),
            None => g0.clone(),
        };
        let hold = if sealed {
            train::sealed_layouts(set.holdout)
        } else {
            train::holdout_layouts(set.holdout)
        };
        let each = train::score_each(ch.as_ref(), &g, &hold, set.threads);
        for (l, s) in hold.iter().zip(&each) {
            println!("NOISE seed={} k={} {:.3}  {:?}", l.seed, l.k, s.fit, s.parts);
        }
        let n = each.len().max(1) as f64;
        let mean = each.iter().map(|s| s.fit).sum::<f64>() / n;
        let sd = (each.iter().map(|s| (s.fit - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0)).sqrt();
        let lo = each.iter().map(|s| s.fit).fold(f64::INFINITY, f64::min);
        let hi = each.iter().map(|s| s.fit).fold(f64::NEG_INFINITY, f64::max);
        println!(
            "NOISE mean {mean:.3} sd {sd:.3} min {lo:.3} max {hi:.3} over {} layouts; standard error of the mean {:.3}",
            each.len(),
            sd / n.sqrt()
        );
        // **The number training and the hold-out report** for a mirrored
        // arena is the worse side of each pair, not the mean of single
        // layouts above (coordinator review of PR 708), so print it too: the
        // challenge's own fold of each pair, which for a suite folds each
        // arena on its own before the product.
        if ch.mirrored() {
            let worse: Vec<f64> = train::sides(true, &hold)
                .into_iter()
                .map(|(a, b)| match b {
                    Some(b) => ch.combine(&[each[a].clone(), each[b].clone()], &[hold[a], hold[b]]).fit,
                    None => ch.combine(&[each[a].clone()], &[hold[a]]).fit,
                })
                .collect();
            let m = worse.len().max(1) as f64;
            let wmean = worse.iter().sum::<f64>() / m;
            let wsd = (worse.iter().map(|f| (f - wmean).powi(2)).sum::<f64>() / (m - 1.0).max(1.0)).sqrt();
            println!(
                "NOISE worse side {wmean:.3} sd {wsd:.3} over {} pairs; standard error {:.3}",
                worse.len(),
                wsd / m.sqrt()
            );
        }
        return;
    }
    if let Some(f) = score {
        let st = if f == "game" {
            Start::Species
        } else {
            Start::File(f.into())
        };
        let g = set_walk(train::start_genome(
            &st,
            &species_genome,
            ch.banned(),
            0,
            scorer.as_deref(),
            walk,
        ));
        println!("(scored with this challenge's bans applied: {:?})", ch.banned());
        let hold = if sealed {
            train::sealed_layouts(set.holdout)
        } else {
            train::holdout_layouts(set.holdout)
        };
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
