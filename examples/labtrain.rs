//! **The lab's COLONY TRAINING loop, headless** (`lab::training`): the same
//! rounds the TRAINING page runs, written as the teacher lane's in-run
//! harness writes them, so the two can be compared file for file.
//!
//! cargo run --release --example labtrain -- seed=1 rounds=2 frames=200000 out=DIR [sel=score|random|off]
//!     [step=harness|game] [kid_grants=3] [turn=30000]
//!
//! `step=harness` (the default) steps the bed with the harness's own subset of
//! the tick (`update::step`, active sites, fields, pheromones), which is what
//! makes `births.csv` and `sel.csv` comparable byte for byte with
//! `examples/inrun.rs` run with `scratch/inrun/jobs/ctl.txt`'s arguments on
//! the compute branch (`claude/project-thread-8y5ak0`). `step=game` steps it
//! with the game's own tick (`lab::tick_world`), as the TRAINING page does.
//!
//! out/rN/births.csv  frame,id,parent,family,gen,kind,links   (the harness's)
//! out/rN/sel.csv     frame,alive,adults,births,removals,bias,median,food_price,dig_price,clear_price,ghosts
//! out/rN/ants.csv    id,tag   the round's founders, in id order
//! stdout             one line per round: food home, births, removals, starved, alive at the end
use pixel_physics::lab::training::{Mode, Training};
use pixel_physics::sim::creature;
use std::io::Write;

fn arg<'a>(a: &'a [String], k: &str) -> Option<&'a str> {
    a.iter().find_map(|s| s.strip_prefix(&format!("{k}=")))
}

fn main() {
    creature::mutation_off_for_measuring();
    let a: Vec<String> = std::env::args().skip(1).collect();
    let num = |k: &str, d: u64| arg(&a, k).map_or(d, |v| v.parse().unwrap_or_else(|_| panic!("{k}={v}: not a number")));
    let known = ["seed", "rounds", "frames", "out", "sel", "step", "kid_grants", "turn"];
    for x in &a {
        let k = x.split('=').next().unwrap_or("");
        assert!(known.contains(&k), "unknown argument {x:?}; known: {known:?}");
    }
    let (seed, rounds) = (num("seed", 1), num("rounds", 1) as usize);
    let out = std::path::PathBuf::from(arg(&a, "out").expect("out=DIR"));
    let game = match arg(&a, "step").unwrap_or("harness") {
        "harness" => false,
        "game" => true,
        other => panic!("step={other}: harness or game"),
    };
    let mut s = pixel_physics::lab::training::Settings { round_frames: num("frames", 200_000), ..Default::default() };
    s.mode = match arg(&a, "sel").unwrap_or("score") {
        "score" => Mode::Score,
        "random" => Mode::Random,
        "off" => Mode::Off,
        other => panic!("sel={other}: score, random or off"),
    };
    if let Some(v) = arg(&a, "kid_grants") {
        s.kid_grants = v.parse().expect("kid_grants");
    }
    s.turn = num("turn", s.turn);
    println!("labtrain: seed {seed}, {rounds} rounds of {} frames, sel {:?}, kid_grants {}, turn {}, step {}", s.round_frames, s.mode, s.kid_grants, s.turn, if game { "game" } else { "harness" });
    let t0 = std::time::Instant::now();
    let (mut t, mut w, mut sc) = Training::start(s, seed);
    let mut particles = pixel_physics::sim::ParticleSystem::new();
    let mut blasts = pixel_physics::sim::explosion::Blasts::new();
    let tuning = pixel_physics::sim::player::Tuning::default();
    for r in 0..rounds {
        let dir = out.join(format!("r{r}"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut f = std::fs::File::create(dir.join("ants.csv")).unwrap();
        writeln!(f, "id,tag").unwrap();
        let mut ids: Vec<_> = t.round.book.keys().copied().collect();
        ids.sort_unstable();
        for id in ids {
            writeln!(f, "{id},{}", t.round.book[&id].family).unwrap();
        }
        while !t.round_done() {
            t.before_step(&mut w);
            if game {
                pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &t.round.bed, Some(&sc));
            } else {
                pixel_physics::sim::update::step(&mut w);
                w.step_active_sites();
                w.step_fields();
                w.step_pheromones();
            }
            t.after_step(&mut w);
        }
        let mut b = std::io::BufWriter::new(std::fs::File::create(dir.join("births.csv")).unwrap());
        writeln!(b, "frame,id,parent,family,gen,kind,links").unwrap();
        for e in &t.round.events {
            writeln!(b, "{}", e.csv()).unwrap();
        }
        let mut sl = std::io::BufWriter::new(std::fs::File::create(dir.join("sel.csv")).unwrap());
        writeln!(sl, "frame,alive,adults,births,removals,bias,median,food_price,dig_price,clear_price,ghosts").unwrap();
        for row in &t.round.sel {
            writeln!(sl, "{}", row.csv()).unwrap();
        }
        let rd = &t.round;
        println!(
            "round {r} seed {}: food home {}, births {}, removals {}, starved {}, alive {}, nest score {}, {:.1}s",
            rd.seed,
            rd.food_home,
            rd.births,
            rd.removals,
            rd.starved,
            rd.alive(&w),
            rd.nest_score.map_or("--".to_string(), |n| format!("{n:.1}")),
            t0.elapsed().as_secs_f64()
        );
        if r + 1 < rounds {
            match t.next_round(&w) {
                Some((nw, nsc)) => {
                    w = nw;
                    sc = nsc;
                    particles = pixel_physics::sim::ParticleSystem::new();
                    blasts = pixel_physics::sim::explosion::Blasts::new();
                }
                None => {
                    println!("extinct after round {r}");
                    break;
                }
            }
        }
    }
}
