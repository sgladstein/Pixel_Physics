//! **How many distinguishable ways of being an ant does this system
//! actually admit?**
//!
//! Samples random genomes rather than perturbing one hand-authored point,
//! and reports the *distribution* of behaviours rather than a leaderboard.
//! Everything before this was local sensitivity analysis around a genome I
//! wrote: ~18 arms, each changing one weight. That cannot distinguish "the
//! space is flat" from "I picked a dull spot in it".
//!
//! This is also generation zero of an evolutionary run, which is the point.
//! If random genomes all behave identically, selection has nothing to act
//! on and Stage 4 would be a random walk — better to learn that in an hour
//! than after building queens, eggs and inheritance.
//!
//! # The environment has to be able to tell strategies apart
//!
//! An earlier version of this scene had abundant food and no predator, and
//! it could not have worked: with nothing scarce and nothing dangerous,
//! "sit still and eat the nearest leaf" and "range widely and commute" both
//! survive, so the distribution comes out flat as a property of the *scene*
//! rather than of the genome space. That is exactly the mistake the
//! corpse-pile ablation already made once (`Reports/creature-direction.md`
//! §13f).
//!
//! So: few trees, many ants, and beetles.
//!
//! # The metrics must not presuppose a strategy
//!
//! **Survival is the outcome**, and it is deliberately the only thing
//! treated as better-or-worse. It is strategy-agnostic: a sessile
//! leaf-camper and a wide-ranging forager are judged on the same number,
//! and it encodes no opinion of mine about how an ant ought to live.
//! Measured as mean population over the run, which is continuous —
//! `CLAUDE.md` prefers a summed quantity to a count for exactly the
//! knife-edge reason.
//!
//! Everything else is a **descriptor**, used to say *what a genome did*,
//! never how well. `travelled`, `commute`, `feeding` and `depth` span the
//! four axes that seem behaviourally independent: how far, how directed,
//! how much it ate, and how far underground it went. Depth is there because
//! burrowing is the behaviour the beetles are supposed to make worth
//! having, and it must be detectable without being rewarded.
//!
//! Diversity is then **behaviour-space coverage**: each descriptor is
//! normalised across the whole sample and binned, and the report counts how
//! many distinct cells of the resulting grid are occupied, with the best
//! survivor in each. That is MAP-Elites' coverage measure, and it answers
//! "how many distinguishable ways" without ranking them against a target.
//!
//! ```text
//! cargo run --release --example creature_space
//! cargo run --release --example creature_space -- genomes=60 seeds=3 frames=6000
//! ```

use pixel_physics::sim::brain::GENOME_LEN;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::organism::CellType;
use pixel_physics::sim::{material, organism, parallel, rng, Cell, World};

/// One run's behavioural fingerprint, plus the one outcome.
#[derive(Clone, Copy, Default)]
struct Sample {
    /// Outcome. Mean live ants over the run, as a fraction of the start.
    survival: f32,
    // --- descriptors: what it did, never how well ---
    travelled: f32,
    commute: f32,
    feeding: f32,
    depth: f32,
    /// **Diagnostic, not a descriptor** -- excluded from `descriptors()`
    /// and from the behaviour-space binning, so it cannot change what the
    /// genome sweep measures.
    ///
    /// The fraction of ants that were ever seen **above their starting
    /// energy**, which in a model where every other term is a cost can only
    /// have come from a meal.
    ///
    /// `feeding` above counts ants that ever *carried* something, which is
    /// not the same question and quietly misses the case that matters
    /// here: `act` branches on hunger, so an ant below
    /// `hunger_fraction` **eats** its find and never sets `carrying` at
    /// all. In this scene's economy (start energy 90, hungry below 45)
    /// that is most of them, so a run where every ant fed itself could
    /// still read `feeding` near zero. §13k's whole finding was "the knob
    /// was never connected because nothing was eating", and the metric
    /// that was supposed to detect that could not see eating.
    eats: f32,
    /// Ants alive at the **first** sample, i.e. how many of `ANTS` the
    /// scene actually managed to place. Diagnostic: `survival` divides by
    /// `ANTS`, so anything short of it is a constant handicap applied to
    /// every arm that has nothing to do with behaviour.
    placed: f32,
}

const DESCRIPTORS: [&str; 4] = ["travelled", "commute", "feeding", "depth"];

impl Sample {
    fn descriptors(&self) -> [f32; 4] {
        [self.travelled, self.commute, self.feeding, self.depth]
    }
}

fn main() {
    let mut genomes = 48usize;
    // **8, not 2, and the old default could not have worked.** Measured
    // spread of one genome over eight world seeds, 3,000 ticks:
    //
    //     zero        mean 0.300  sd 0.002   (a pure starvation clock)
    //     authored    mean 0.504  sd 0.116   (range 0.312 - 0.641)
    //     random r000 mean 0.284  sd 0.030
    //
    // A forager's score moves by 23% of itself on the world seed alone, so
    // at S=2 the standard error on a genome is 0.082 against a total
    // outcome range of about 0.30 -- an error bar a quarter as wide as the
    // thing being measured. S=8 brings it to 0.041.
    //
    // Note the spread is itself genome-dependent (0.002 against 0.116), so
    // no single S is efficient for every arm; this is sized for the noisy
    // end, because that is the end with the foragers in it.
    let mut seeds = 8u64;
    // **18,000 frames = 3,000 ticks, and the horizon is not a detail.**
    // An idle ant always starves at tick 900 whatever the run length, so
    // `survival` -- mean live population over the run -- is a function of
    // the horizon: doing nothing scores 0.90 over 1,000 ticks, 0.45 over
    // 2,000 and 0.30 over 3,000, while a forager that can re-feed has no
    // such ceiling. The old 6,000 was ~1.1 idle lifetimes, which is the
    // single most favourable horizon for immobility that still kills it,
    // and it is why every economy sweep so far concluded that doing
    // nothing wins. At 3,000 ticks the sign flips (-0.068 -> +0.247 at the
    // same settings) and the run asks the question the metric is named
    // for: can this way of living *sustain* itself, rather than how much
    // of one starting battery did it spend.
    let mut frames = 18000usize;
    // **2, not 3, because coverage is over `bins^4`.** At 3 bins the grid
    // is 81 cells and the default sample is 48 genomes, so coverage was
    // capped at 59% by sample size before the genome space got a say -- and
    // a low number would have been unreadable, since "the space is
    // degenerate" and "I did not draw enough samples" produce the same
    // output. 16 cells against 48 genomes is 3x oversampled and the number
    // means what it says. Raise it to 3 only with G ~ 400 (see the cost
    // note on `seeds`: about 12 h at S=8).
    let mut bins = 2usize;
    let mut mode = String::from("genomes");
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "genomes" => genomes = v.parse().expect("genomes"),
            "seeds" => seeds = v.parse().expect("seeds"),
            "frames" => frames = v.parse().expect("frames"),
            "bins" => bins = v.parse().expect("bins"),
            "mode" => mode = v.to_string(),
            other => panic!("unknown arg {other:?}; known: genomes, seeds, frames, bins, mode"),
        }
    }

    if mode == "threads" {
        thread_scaling(frames);
        return;
    }
    if mode == "cost" {
        cost_breakdown(frames);
        return;
    }
    if mode == "noise" {
        noise_floor(seeds.max(8), frames);
        return;
    }
    if mode == "spawn" {
        spawn_census();
        return;
    }
    if mode == "economy" {
        economy_sweep(seeds, frames);
        return;
    }
    println!("creature space: {genomes} random genomes + 2 references, {seeds} seeds x {frames} frames each");
    println!("scene: scarce food (2 trees), 52 ants, 4 beetles, generated terrain\n");

    // The two reference points, so the random distribution has something to
    // be read against. A zero genome cannot move at all, which makes it the
    // floor rather than a competitor.
    let mut jobs: Vec<(String, Vec<f32>)> = vec![("zero".to_string(), vec![0.0; GENOME_LEN]), ("authored".to_string(), authored_genome())];
    jobs.extend((0..genomes).map(|g| (format!("r{g:03}"), random_genome(0x5EED + g as u64))));

    // **One thread per genome, and it is a 3.5x saving rather than a
    // tidy-up.** Runs are embarrassingly parallel -- each owns its `World`
    // -- and `parallel::step`'s in-run rayon pass does not saturate the
    // machine on a world this small: four independent runs measured 43.3 s
    // sequential against 12.3 s on four threads, a 3.52x speedup on 4
    // available threads. That is the difference between a usefully-sized
    // sweep taking an hour and taking five, and sizing this sweep is
    // entirely a question of what fits in an affordable wall clock.
    //
    // Determinism is unaffected: every run is keyed by its own genome and
    // world seed and shares no state, so results do not depend on the order
    // threads happen to finish in. Chunked rather than one thread per job so
    // that a large `genomes` does not spawn hundreds of threads that then
    // contend.
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let mut labelled: Vec<(String, Sample)> = Vec::new();
    for chunk in jobs.chunks(threads) {
        let done: Vec<(String, Sample)> = std::thread::scope(|scope| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|(label, genome)| {
                    scope.spawn(move || {
                        let s = mean_of((0..seeds).map(|sd| run_one(genome, frames, 0xC0DE + sd, DEFAULT_ECONOMY)).collect());
                        (label.clone(), s)
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().expect("a sweep run panicked")).collect()
        });
        labelled.extend(done);
        eprintln!("  {} / {} genomes done", labelled.len(), jobs.len());
    }

    // --- the distribution, which is the actual output --------------------
    println!("{:<10} {:>9} {:>10} {:>9} {:>9} {:>7}", "genome", "survival", "travelled", "commute", "feeding", "depth");
    for (label, s) in &labelled {
        println!(
            "{label:<10} {:>9.3} {:>10.1} {:>9.3} {:>9.3} {:>7.1}",
            s.survival, s.travelled, s.commute, s.feeding, s.depth
        );
    }

    let random: Vec<Sample> = labelled.iter().skip(2).map(|(_, s)| *s).collect();
    println!("\n--- spread across {} random genomes ---", random.len());
    println!("{:<12} {:>8} {:>8} {:>8} {:>8}", "", "min", "median", "max", "spread");
    report_spread("survival", &random, |s| s.survival);
    for (i, name) in DESCRIPTORS.iter().enumerate() {
        report_spread(name, &random, move |s| s.descriptors()[i]);
    }

    // --- behaviour-space coverage ----------------------------------------
    // Normalised over the whole sample, so the bins describe the range the
    // system actually produced rather than a range I guessed at.
    let all: Vec<Sample> = labelled.iter().map(|(_, s)| *s).collect();
    let mut lo = [f32::MAX; 4];
    let mut hi = [f32::MIN; 4];
    for s in &all {
        for (i, v) in s.descriptors().iter().enumerate() {
            lo[i] = lo[i].min(*v);
            hi[i] = hi[i].max(*v);
        }
    }
    let cell_of = |s: &Sample| -> Vec<usize> {
        s.descriptors()
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let span = (hi[i] - lo[i]).max(1e-6);
                (((v - lo[i]) / span) * bins as f32).floor().min(bins as f32 - 1.0) as usize
            })
            .collect()
    };
    let mut elites: std::collections::HashMap<Vec<usize>, (String, f32)> = std::collections::HashMap::new();
    for (label, s) in &labelled {
        let cell = cell_of(s);
        let e = elites.entry(cell).or_insert((label.clone(), f32::MIN));
        if s.survival > e.1 {
            *e = (label.clone(), s.survival);
        }
    }
    let total = bins.pow(4);
    println!(
        "\n--- behaviour space: {} of {total} cells occupied ({} bins on each of {}) ---",
        elites.len(),
        bins,
        DESCRIPTORS.join(", ")
    );
    let mut keys: Vec<_> = elites.keys().cloned().collect();
    keys.sort();
    for k in keys {
        let (label, survival) = &elites[&k];
        println!("  cell {k:?}  best {label} at survival {survival:.3}");
    }
    println!(
        "\nRead this as: occupied cells = distinguishable ways of being an ant that the\nsystem actually produced. One or two means the space is degenerate however\nmany weights it has; a spread means selection has something to act on."
    );
}


/// **Does foraging pay, and is food still scarce?** Those two have to be
/// true at once, and the readiness check found they currently are not:
/// immobility beat the authored forager 0.554 to 0.237, because movement is
/// a pure cost whose payoff is an unreliable trip to a distant tree.
///
/// Reported as `advantage` = the forager's survival minus the immobile
/// genome's. Positive means a creature is better off doing something than
/// nothing. `zero` is the null strategy and it is the right control: it
/// cannot move, so its survival is exactly what the environment hands out
/// for free.
///
/// Scarcity is the second column: if the forager approaches 1.0 the food is
/// simply abundant, and a genome sweep there would measure nothing again.
/// The band worth having is **advantage clearly positive while survival is
/// well short of 1.0**.
/// The terrain the economy sweep runs on. **"wetland", not the default
/// "rolling"**, and the census above is why: it is the only preset where
/// ground-level food both exists and replenishes.
const PRESET: &str = "wetland";

fn economy_sweep(seeds: u64, frames: usize) {
    println!("energy economy sweep: does foraging pay, and is food still scarce? ({frames} frames = {} ticks, idle life is 900)
", frames / 6);
    println!("{:<8} {:<6} {:<8} {:>10} {:>10} {:>9} {:>11} {:>11}", "beetles", "moss", "eat", "move_cost", "forager", "immobile", "advantage", "ants fed");
    println!("(survival is against the ants actually placed; \"placed\" reports that count)");
    let zero = vec![0.0; GENOME_LEN];
    let authored = authored_genome();
    // **Does the scene contain the situation?** Asked before the sweep and
    // not after, because §13k's whole lesson was that a sweep can vary
    // three knobs faithfully while the thing they act on is not present.
    // Moss is scattered by the generator, not placed by this harness, so
    // whether there is any -- and whether it is where the ants are -- is a
    // property of the terrain and has to be looked at.
    // Slow (four presets x 8,400 frames) and it answers a question about
    // the *terrain*, which does not change between sweep runs. Behind a
    // flag so re-running the sweep does not re-pay for it: CENSUS=1.
    if std::env::var("CENSUS").is_ok() {
        scene_food_census(0xC0DE);
    }
    let mut best: Option<(f32, String)> = None;
    // **Twelve settings, not fifty-four.** The previous sweep already
    // established what `move_cost` does (-0.24 at 0.25, -0.03 at 0.02) and
    // that `trees` did nothing because the leaves were unreachable; paying
    // four hours to re-derive that would be re-deriving a known result.
    // What is open is whether ground-level food changes the sign, so `moss`
    // is the arm and everything else is held at two levels -- including
    // `eat_energy` at its extremes, which doubles as the connectivity
    // check: with nothing eating, 120 and 700 came out bit-identical.
    // Held fixed, and each for a measured reason rather than by default:
    // `trees` because §13n's census found the canopy is out of reach at
    // every density; `move_cost` because §13k already mapped it and it is
    // monotone; `beetles` because the arms came out bit-identical over
    // 6,000 frames -- a beetle cannot find an ant, so the predator is not
    // yet part of the loop (§13o).
    let (trees, move_cost, beetles) = (2i32, 0.08f32, BEETLES);
    for &moss in &[false, true] {
        for &eat_energy in &[120.0f32, 700.0] {
            {
                let econ = Economy { eat_energy, move_cost, trees, moss, preset: PRESET, beetles };
                let fs = mean_of((0..seeds).map(|s| run_one(&authored, frames, 0xC0DE + s, econ)).collect());
                let f = fs.survival;
                let z = mean_of((0..seeds).map(|s| run_one(&zero, frames, 0xC0DE + s, econ)).collect()).survival;
                let adv = f - z;
                // **Meals per ant beside the outcome, because the last
                // sweep could not tell "foraging does not pay" from
                // "nothing is eating".** A knob that never connects reads
                // as a column of identical numbers; a count of actual
                // meals says which of the two is happening, and it is what
                // makes `eat_energy` falsifiable as a knob at all.
                println!("{beetles:<8} {:<6} {eat_energy:<8.0} {move_cost:>10.2} {f:>10.3} {z:>9.3} {adv:>11.3} {:>11.2}   placed {:.0}", if moss { "yes" } else { "no" }, fs.eats, fs.placed);
                // Scarcity guard: an advantage bought by making food
                // abundant is not the band we are looking for.
                if f < 0.9 && best.as_ref().is_none_or(|(b, _)| adv > *b) {
                    best = Some((adv, format!("moss {moss}, eat {eat_energy:.0}, move {move_cost:.2} (forager {f:.3})")));
                }
            }
        }
    }
    match best {
        Some((adv, what)) if adv > 0.0 => println!("
best band with food still scarce: {what} -- advantage {adv:+.3}"),
        Some((adv, what)) => println!("
no setting made foraging pay. Least bad: {what} -- advantage {adv:+.3}"),
        None => println!("
every setting made food abundant; scarcity was never tested"),
    }
}

/// **Do independent runs get faster in parallel, or is `parallel::step`
/// already using the machine?** The whole feasibility of a large genome
/// sweep turns on this one ratio: runs are embarrassingly parallel (each
/// owns its `World`), so if the in-run rayon pass is leaving cores idle on
/// a world this small, a sweep is several times cheaper than the
/// sequential estimate says.
fn thread_scaling(frames: usize) {
    let genomes: Vec<Vec<f32>> = (0..4).map(|i| random_genome(0x5EED + i)).collect();
    let seq_start = std::time::Instant::now();
    for (i, g) in genomes.iter().enumerate() {
        run_one(g, frames, 0xC0DE + i as u64, DEFAULT_ECONOMY);
    }
    let seq = seq_start.elapsed();

    let par_start = std::time::Instant::now();
    std::thread::scope(|scope| {
        for (i, g) in genomes.iter().enumerate() {
            scope.spawn(move || {
                run_one(g, frames, 0xC0DE + i as u64, DEFAULT_ECONOMY);
            });
        }
    });
    let par = par_start.elapsed();

    println!("4 independent runs at {frames} frames each:");
    println!("  sequential {:>7.1} s  ({:.1} s per run)", seq.as_secs_f64(), seq.as_secs_f64() / 4.0);
    println!("  4 threads  {:>7.1} s  ({:.1} s per run)", par.as_secs_f64(), par.as_secs_f64() / 4.0);
    println!("  speedup    {:>7.2}x on {} available threads", seq.as_secs_f64() / par.as_secs_f64(), std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0));
}

/// **Where does a run's time actually go?** The sizing question turns on
/// it: worldgen, the 2,400-frame warmup and the tree growth are *identical
/// for every genome at a given seed* -- nothing in the scene depends on the
/// genome until the ants are planted -- so whatever fraction of a run they
/// are is a fraction the sweep is paying G times over for no information.
fn cost_breakdown(frames: usize) {
    let (w, h) = (512i32, 160i32);
    let t0 = std::time::Instant::now();
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    world.seed = 0xC0DE;
    let (presets, _) = pixel_physics::worldgen::WorldgenPresets::load();
    let Some(params) = presets.get(PRESET) else { return };
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed: 0xC0DE });
    let gen = t0.elapsed();

    let surface = |world: &World, x: i32| -> i32 {
        (0..h).find(|&y| matches!(world.materials.kind(world.get(x, y).material), material::MaterialKind::Solid | material::MaterialKind::Powder)).unwrap_or(h - 1)
    };
    let surface_at: Vec<i32> = (0..w).map(|x| surface(&world, x)).collect();
    let nest = world.materials.id_of("nest").expect("nest");
    for x in 16..90 {
        world.set(x, surface_at[x as usize], Cell::new(nest, 0).with_attached(true));
    }
    for i in 0..2 {
        let x = 150 + i * 150;
        world.plant_tree(x, surface_at[x as usize] - 1);
    }
    let t1 = std::time::Instant::now();
    for _ in 0..2400 {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
    }
    let warmup = t1.elapsed();

    // The genome-dependent half, timed at the real length.
    let t2 = std::time::Instant::now();
    for _ in 0..frames {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
    }
    let main = t2.elapsed();

    let setup = gen + warmup;
    let total = setup + main;
    println!("one run at {frames} frames, preset {PRESET}:");
    println!("  worldgen            {:>7.1} s", gen.as_secs_f64());
    println!("  warmup 2400 frames  {:>7.1} s", warmup.as_secs_f64());
    println!("  main {frames} frames  {:>7.1} s", main.as_secs_f64());
    println!("  ---");
    println!("  setup (genome-independent, cacheable) {:>6.1} s = {:.0}%", setup.as_secs_f64(), 100.0 * setup.as_secs_f64() / total.as_secs_f64());
    println!("  total                                 {:>6.1} s", total.as_secs_f64());
    println!("  note: this run carries no ants, so the main loop is a floor, not the real per-run cost.");
    println!("
threads rayon can use: {}", std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0));
}

/// **How big does the genome sweep need to be?** Both halves of that are
/// measurements, not opinions, and this is the first: how much does one
/// genome's score move when only the world seed changes?
///
/// Everything downstream depends on it. A sweep that cannot separate two
/// genomes by more than its own seed-to-seed spread is measuring weather,
/// and `CLAUDE.md` is explicit that outcomes here have enormous spread --
/// twelve identical trees from one genome span 31 to 153 cells. The number
/// of seeds per genome has to come from the standard error this reports,
/// not from a round number.
///
/// Run against three reference genomes, because the spread is not
/// necessarily the same everywhere in the space: the zero genome cannot
/// move and should be nearly noiseless (it is a pure starvation clock),
/// while a forager's outcome depends on where the terrain put the food.
/// If those two have very different spreads, seeds cannot be a constant.
fn noise_floor(seeds: u64, frames: usize) {
    println!("noise floor: one genome, {seeds} world seeds, {frames} frames ({} ticks)
", frames / 6);
    println!("{:<12} {:>9} {:>9} {:>9} {:>9} {:>9}  per-seed survival", "genome", "mean", "sd", "min", "max", "sd/mean");
    let refs: [(&str, Vec<f32>); 3] =
        [("zero", vec![0.0; GENOME_LEN]), ("authored", authored_genome()), ("random r000", random_genome(0x5EED))];
    let started = std::time::Instant::now();
    let mut runs = 0usize;
    for (label, genome) in refs {
        let vals: Vec<f32> = (0..seeds)
            .map(|s| {
                runs += 1;
                run_one(&genome, frames, 0xC0DE + s, DEFAULT_ECONOMY).survival
            })
            .collect();
        let n = vals.len() as f32;
        let mean = vals.iter().sum::<f32>() / n;
        let sd = (vals.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / (n - 1.0).max(1.0)).sqrt();
        let (min, max) = (vals.iter().cloned().fold(f32::MAX, f32::min), vals.iter().cloned().fold(f32::MIN, f32::max));
        let each: Vec<String> = vals.iter().map(|v| format!("{v:.3}")).collect();
        println!("{label:<12} {mean:>9.3} {sd:>9.3} {min:>9.3} {max:>9.3} {:>9.2}  {}", if mean > 0.0 { sd / mean } else { 0.0 }, each.join(" "));
    }
    let per_run = started.elapsed().as_secs_f64() / runs as f64;
    println!("
cost: {runs} runs in {:.0} s, {per_run:.1} s per run at {frames} frames", started.elapsed().as_secs_f64());
    println!("So a sweep of G genomes x S seeds costs about G*S*{per_run:.1} s.");
    // The seeds needed for a target standard error, from the widest sd seen.
    println!("
seeds needed to resolve a survival difference of D, at the widest sd s:");
    println!("  n = 2*(1.96*s/D)^2 per arm (two-sided, 95%); for s = 0.05: D=0.10 -> n=2, D=0.05 -> n=8, D=0.02 -> n=48");
}

/// **How many of the 52 ants asked for actually get placed?**
///
/// `survival` divides mean live population by `ANTS`, so a placement that
/// silently fails -- `plant_ant` does nothing at all if the seed will not
/// fit -- lowers every arm's score by a constant that has nothing to do
/// with behaviour. §13f already caught two metrics measuring the spawn
/// layout rather than the animals; this is the check that the third one is
/// not doing it again, and it is prompted by a number that does not add up:
/// the immobile genome pays only `idle_cost` 0.10 against 90 energy, so it
/// must starve at tick ~900 of ~1000 and score ~0.90. It scores 0.554, and
/// removing every beetle does not move it.
fn spawn_census() {
    for preset in ["wetland", "rolling"] {
        let (w, h) = (512i32, 160i32);
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
        world.seed = 0xC0DE;
        let (presets, _) = pixel_physics::worldgen::WorldgenPresets::load();
        let Some(params) = presets.get(preset) else { continue };
        pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed: 0xC0DE });
        let surface = |world: &World, x: i32| -> i32 {
            (0..h).find(|&y| matches!(world.materials.kind(world.get(x, y).material), material::MaterialKind::Solid | material::MaterialKind::Powder)).unwrap_or(h - 1)
        };
        let surface_at: Vec<i32> = (0..w).map(|x| surface(&world, x)).collect();
        for _ in 0..2400 {
            parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
        }
        let ant_material = world.materials.id_of("ant").expect("ant");
        for i in 0..ANTS {
            let x = 24 + i as i32 * 4;
            world.plant_ant(x, surface_at[x as usize] - 1);
        }
        let heads = (0..w)
            .flat_map(|x| (0..h).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                let c = world.get(x, y);
                c.material == ant_material && organism::cell_type(c.aux()) == Some(CellType::Head)
            })
            .count();
        // What was in the way, for the ones that failed.
        let mut blocked_by: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        for i in 0..ANTS {
            let x = 24 + i as i32 * 4;
            let (px, py) = (x, surface_at[x as usize] - 1);
            let c = world.get(px, py);
            if c.material != ant_material {
                let name = world.materials.get(c.material).name.clone();
                *blocked_by.entry(name).or_default() += 1;
            }
        }
        println!("{preset}: asked for {ANTS} ants, {heads} placed; cells asked for that hold no ant: {blocked_by:?}");
    }
}

/// Count the food in the scene, in the band the ants are actually spawned
/// into, before running anything on it.
fn scene_food_census(seed: u64) {
    println!("scene census (seed {seed:#x}), no ants -- what food exists at ant height, and does it grow?");
    println!("{:<10} {:<12} {:>10} {:>14} {:>16} {:>10} {:>14}", "preset", "frame", "moss", "moss surface", "moss in colony", "leaf", "leaf surface");
    for preset in ["rolling", "wetland", "terraced", "canyon"] {
        census_one(seed, preset);
    }
    println!();
}

fn census_one(seed: u64, preset: &str) {
    let (w, h) = (512i32, 160i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    world.seed = seed;
    let (presets, _) = pixel_physics::worldgen::WorldgenPresets::load();
    let Some(params) = presets.get(preset) else { return };
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed });
    let surface = |world: &World, x: i32| -> i32 {
        (0..h).find(|&y| matches!(world.materials.kind(world.get(x, y).material), material::MaterialKind::Solid | material::MaterialKind::Powder)).unwrap_or(h - 1)
    };
    let surface_at: Vec<i32> = (0..w).map(|x| surface(&world, x)).collect();
    for i in 0..2 {
        let x = 150 + i * 150;
        world.plant_tree(x, surface_at[x as usize] - 1);
    }
    // **The same step set `run_one` uses.** A first version of this census
    // warmed up with `step_active_sites` + `step_fields` only, omitting
    // `parallel::step`, and reported zero leaves forever -- a stand-in for
    // the run that was not the run. `CLAUDE.md`: prefer the real thing over
    // a controlled approximation of it, and check the scene contains the
    // situation before believing what it says about the mechanism.
    for _ in 0..2400 {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
    }
    let count = |world: &World, name: &str, band: std::ops::Range<i32>, near_surface: bool| -> usize {
        let Some(id) = world.materials.id_of(name) else { return 0 };
        band.flat_map(|x| (0..h).map(move |y| (x, y)))
            .filter(|&(x, y)| world.get(x, y).material == id && (!near_surface || (y - surface_at[x as usize]).abs() <= 3))
            .count()
    };
    // The ants are planted at `24 + i*4` for 52 of them, so 24..232 is the
    // colony's own band; the rest of the map is where it would have to
    // travel to.
    let report = |world: &World, label: &str| {
        println!(
            "{preset:<10} {label:<12} {:>10} {:>14} {:>16} {:>10} {:>14}",
            count(world, "moss", 0..w, false),
            count(world, "moss", 0..w, true),
            count(world, "moss", 24..232, true),
            count(world, "leaf", 0..w, false),
            count(world, "leaf", 0..w, true),
        );
    };
    report(&world, "warmup end");
    for _ in 0..6000 {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
    }
    report(&world, "+6000");
}

fn report_spread(name: &str, samples: &[Sample], f: impl Fn(&Sample) -> f32) {
    let mut v: Vec<f32> = samples.iter().map(&f).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if v.is_empty() {
        return;
    }
    let (min, max, med) = (v[0], v[v.len() - 1], v[v.len() / 2]);
    println!("{name:<12} {min:>8.3} {med:>8.3} {max:>8.3} {:>8.3}", max - min);
}

fn mean_of(runs: Vec<Sample>) -> Sample {
    let n = runs.len() as f32;
    Sample {
        survival: runs.iter().map(|r| r.survival).sum::<f32>() / n,
        travelled: runs.iter().map(|r| r.travelled).sum::<f32>() / n,
        commute: runs.iter().map(|r| r.commute).sum::<f32>() / n,
        feeding: runs.iter().map(|r| r.feeding).sum::<f32>() / n,
        depth: runs.iter().map(|r| r.depth).sum::<f32>() / n,
        eats: runs.iter().map(|r| r.eats).sum::<f32>() / n,
        placed: runs.iter().map(|r| r.placed).sum::<f32>() / n,
    }
}

fn authored_genome() -> Vec<f32> {
    let reg = organism::SpeciesRegistry::builtin();
    reg.get(reg.id_of("ant").expect("ant species")).genome.clone()
}

/// A random genome, sparse like an authored one.
///
/// **Sparsity is itself sampled**, from 2% to 15% of connections live. A
/// fixed density would be one more of my assumptions smuggled into the
/// "random" arm — and a dense random brain is not a creature, it is noise,
/// since every input drives every output at once.
fn random_genome(seed: u64) -> Vec<f32> {
    let mut r = rng::stream(seed, 0, 0, 0);
    let density = 0.02 + r.unit_f32() * 0.13;
    (0..GENOME_LEN)
        .map(|_| if r.unit_f32() < density { (r.unit_f32() * 2.0 - 1.0) * 3.0 } else { 0.0 })
        .collect()
}

const DEFAULT_ECONOMY: Economy = Economy { eat_energy: 120.0, move_cost: 0.25, trees: 2, moss: true, preset: "wetland", beetles: BEETLES };

/// Named because the "did this ant ever eat" detector reads against it.
const START_ENERGY: f32 = 90.0;

const ANTS: usize = 52;
const BEETLES: usize = 9;

/// The knobs that decide whether foraging is worth doing.
///
/// `moss` is here because `Reports/creature-direction.md` §13k found the
/// bottleneck was not price but **reach**: every `eat_energy` 300 row of
/// the previous sweep equalled its 700 row bit for bit, which is the
/// never-connected-knob tell -- raising the reward for eating changed
/// nothing because nothing was eating. Leaves are in the canopy and ants
/// walk on the ground. Moss lives exactly where they walk, spreads by
/// division on damp stone, and the generator already scatters it
/// (`worldgen.ron`, `moss_density: 0.10` on "rolling"), so this is one
/// string in the food list and no new mechanism at all.
///
/// Carried as an arm rather than an edit so both halves can be measured in
/// one session: a bar remembered from an earlier run is the thing
/// `CLAUDE.md` says not to compare against.
#[derive(Clone, Copy)]
struct Economy {
    eat_energy: f32,
    move_cost: f32,
    trees: i32,
    moss: bool,
    /// **Which world, because that turned out to be the knob that
    /// mattered.** A census of the generator's own presets, run before
    /// this sweep rather than after it, found that moss is only a
    /// *renewable* food on damp ground: over 6,000 frames it went 86 ->
    /// 1,194 cells on "wetland" (305 at the surface, 181 inside the
    /// colony's own band) and moved by 0, 2 and 0 cells on "rolling",
    /// "terraced" and "canyon". Moss stalling on dry sunlit ground is
    /// correct M16 behaviour, not a bug -- it just means the terrain the
    /// previous sweep ran on had no ground-level food at all.
    ///
    /// The same census quantified §13k's reachability finding: every
    /// preset ends the run with thousands of leaves and **0 to 11 of them
    /// within three cells of the ground**. The canopy grows away from the
    /// ants.
    preset: &'static str,
    /// **How many beetles, because "does foraging pay" has two possible
    /// binding constraints and only one of them is food.** A forager that
    /// eats well and is eaten anyway scores exactly like a forager that
    /// never found anything, and the food knobs cannot tell those apart --
    /// which is consistent with survival coming out bit-identical at
    /// `eat_energy` 120 and 700 while `move_cost` moved it freely. Zero
    /// beetles is the control that isolates it.
    beetles: usize,
}

fn run_one(genome: &[f32], frames: usize, seed: u64, econ: Economy) -> Sample {
    let (w, h) = (512i32, 160i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    world.seed = seed;
    let species = world.species.id_of("ant").expect("ant species");
    world.species.set_genome(species, genome.to_vec());
    // **Make starvation reachable inside the run.** At `ant.ron`'s 900
    // starting energy an ant lives ~7,500 ticks and a run is ~1,000, so the
    // first version of this scene scored every genome at survival 0.91 --
    // including the zero genome, which cannot move. An outcome that is
    // identical for every behaviour is not an outcome.
    //
    // **And low enough that sitting still is not a winning strategy.** At
    // 260 an idle ant outlives the run, so doing nothing was *cheaper* than
    // living: the zero genome, which cannot move at all, scored the best
    // survival of any arm (0.923 against the forager's 0.735). That is
    // P-20's sessile-freeloading attractor arriving before evolution was
    // even switched on, and an environment that rewards immobility selects
    // for exactly that.
    //
    // 150 was still not low enough: an idle ant lasts ~1,250 ticks against
    // a 1,500-tick run, so the zero genome *still* outlived everyone
    // (0.923). The budget has to fall below the run, not near it.
    //
    // At 90 an idle ant is dead by ~900 ticks, three-fifths of the way
    // through, so sitting still caps out around 0.6 however safe it is.
    // A single meal is worth 120 -- more than a whole starting life -- so
    // anything that feeds itself can outlast anything that does not. The
    // question survival asks is now "did this way of living feed itself",
    // which is the one worth asking.
    let mut def = world.species.get(species).creature.as_ref().expect("ant is a creature").clone();
    def.start_energy = START_ENERGY;
    def.eat_energy = econ.eat_energy;
    def.move_cost = econ.move_cost;
    // **Scale the synapse tax with the budget, or the control is not a
    // control.** `ant.ron` sets 0.002 per active synapse against a starting
    // energy of 900 -- "a dense brain costs about as much as standing
    // still". Cutting the budget to 90 to create scarcity, and leaving this
    // alone, made a brain cost **72 of 90 starting energy over a run: 80%
    // of a life, just for thinking**. The zero genome pays none of it,
    // because it has no active synapses at all -- so "forager versus
    // immobile" was really "thinks versus does not think", and the thinking
    // was the larger term by far.
    //
    // That is `CLAUDE.md`'s rule arriving in the usual disguise: when a fix
    // changes what a number *means*, re-deriving the constants that read it
    // is part of the fix. Held at the ratio ant.ron actually authored.
    def.synapse_cost = 0.002 * (def.start_energy / 900.0);
    // **Corpses are off the menu here, and finding that out was the point
    // of the readiness check.** With "corpse" in the food list, a starved
    // ant feeds the ants around it, so a colony sustains itself on its own
    // dead without anyone foraging at all -- moving the trees and the
    // beetles across the map changed the outcome by *nothing*, bit for bit,
    // because neither was ever part of the loop. Survival then measures
    // scavenging efficiency, not foraging, and immobility wins.
    //
    // Leaves only, so the food is somewhere an ant has to go and get.
    def.food = if econ.moss { vec!["leaf".to_string(), "moss".to_string()] } else { vec!["leaf".to_string()] };
    world.species.set_creature(species, def);
    let ant_material = world.materials.id_of("ant").expect("ant");

    let (presets, _) = pixel_physics::worldgen::WorldgenPresets::load();
    let Some(params) = presets.get(econ.preset) else { return Sample::default() };
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed });

    let surface = |world: &World, x: i32| -> i32 {
        (0..h)
            .find(|&y| matches!(world.materials.kind(world.get(x, y).material), material::MaterialKind::Solid | material::MaterialKind::Powder))
            .unwrap_or(h - 1)
    };
    let surface_at: Vec<i32> = (0..w).map(|x| surface(&world, x)).collect();

    let nest = world.materials.id_of("nest").expect("nest");
    for x in 16..90 {
        world.set(x, surface_at[x as usize], Cell::new(nest, 0).with_attached(true));
    }
    // **Scarce**: two trees, not six. Food that runs short is what makes
    // one way of living better than another.
    for i in 0..econ.trees {
        let x = 150 + i * (300 / econ.trees.max(1));
        world.plant_tree(x, surface_at[x as usize] - 1);
    }
    for _ in 0..2400 {
        world.step_active_sites();
        world.step_fields();
    }
    // **Place ants until there are `ANTS` of them, not at `ANTS` fixed
    // columns.** `plant_ant` silently does nothing when the seed will not
    // fit, and on wetland the target cell is standing water 23 times out of
    // 52 -- so the scene asked for 52 and stood up 23, while `survival`
    // went on dividing by 52. That put a per-seed constant of ~0.44 on the
    // ceiling of the outcome variable, which is §13f's spawn-layout trap
    // for the third time: the zero-genome control did not catch it because
    // both arms carry the same handicap, so it cancels in `advantage` and
    // survives untouched in every absolute number.
    //
    // It also matters for behaviour, not just for arithmetic: stigmergy has
    // a minimum viable population and 23 ants is below the 50 this scene
    // was designed around.
    let mut planted = 0usize;
    let mut x = 24i32;
    while planted < ANTS && x < w - 8 {
        if let Some(site) = pixel_physics::sim::creature::plant_creature_seed(&mut world, x, surface_at[x as usize] - 1, "ant") {
            world.schedule_active_site(site);
            planted += 1;
            x += 4;
        } else {
            // Spacing only matters between ants that exist; a column that
            // refuses one costs a single step, not a gap of four (§13e --
            // a dense line of ants gridlocks, so four apart is the floor
            // *between placed ants*).
            x += 1;
        }
    }
    // **Dangerous, and dangerous *where the ants are*.** Placed beyond the
    // colony's range, beetles punished only the ants that travelled -- so
    // sitting still at the nest was both cheap and safe, and the zero
    // genome (which cannot move at all) beat every forager. Danger that
    // only reaches the active is danger that selects for inactivity.
    //
    // Beetles are wide, eat ants, and cannot dig. An ant that burrows is
    // out of reach; nothing in the engine says so.
    for i in 0..econ.beetles {
        let x = 40 + i as i32 * 45;
        pixel_physics::sim::creature::plant_creature_seed(&mut world, x, surface_at[x as usize] - 1, "beetle")
            .map(|s| world.schedule_active_site(s));
    }

    let mut path: std::collections::HashMap<u16, f32> = std::collections::HashMap::new();
    let mut last: std::collections::HashMap<u16, (i32, i32)> = std::collections::HashMap::new();
    let mut start: std::collections::HashMap<u16, (i32, i32)> = std::collections::HashMap::new();
    let mut furthest: std::collections::HashMap<u16, f32> = std::collections::HashMap::new();
    let mut fed: std::collections::HashSet<u16> = std::collections::HashSet::new();
    let mut ate: std::collections::HashSet<u16> = std::collections::HashSet::new();
    let mut seen: std::collections::HashSet<u16> = std::collections::HashSet::new();
    let (mut pop_sum, mut pop_samples, mut depth_sum, mut depth_n) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut placed = 0.0f32;

    for frame in 0..frames {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
        if frame % 6 != 0 {
            continue;
        }
        let mut alive = 0.0;
        let first_sample = pop_samples == 0.0;
        for x in 0..w {
            for y in 0..h {
                let c = world.get(x, y);
                if c.material != ant_material || organism::cell_type(c.aux()) != Some(CellType::Head) {
                    continue;
                }
                alive += 1.0;
                let id = c.organism_id();
                seen.insert(id);
                // Depth below the *original* surface: positive means
                // underground. Detected, never rewarded.
                depth_sum += (y - surface_at[x as usize]) as f64;
                depth_n += 1.0;
                if let Some(&(px, py)) = last.get(&id) {
                    *path.entry(id).or_insert(0.0) += ((((x - px).pow(2) + (y - py).pow(2)) as f32).sqrt()).abs();
                } else {
                    start.insert(id, (x, y));
                }
                last.insert(id, (x, y));
                if let Some(&(sx, sy)) = start.get(&id) {
                    let d = (((x - sx).pow(2) + (y - sy).pow(2)) as f32).sqrt();
                    let e = furthest.entry(id).or_insert(0.0);
                    *e = e.max(d);
                }
                if world.organism(id).is_some_and(|s| s.carrying.is_some()) {
                    fed.insert(id);
                }
                // **An ant that has ever been above its starting energy has
                // eaten**, and nothing else in the model can put it there:
                // every other term is a cost. This is the species-specific
                // detector, and it exists because the obvious one is wrong
                // -- `creature_stats.eats` is a *global* counter and the
                // beetles eat ants, so "eats per ant" cannot tell a colony
                // that fed itself from a colony that was fed on.
                if world.organism(id).is_some_and(|s| s.energy > START_ENERGY) {
                    ate.insert(id);
                }
            }
        }
        if first_sample {
            placed = alive;
        }
        pop_sum += alive as f64;
        pop_samples += 1.0;
    }

    let mut ranges: Vec<f32> = furthest.values().copied().collect();
    ranges.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p90 = if ranges.is_empty() { 0.0 } else { ranges[(ranges.len() * 9 / 10).min(ranges.len() - 1)] };
    let mut commutes: Vec<f32> = last
        .iter()
        .filter_map(|(id, &(x, y))| {
            let &(sx, sy) = start.get(id)?;
            let p = *path.get(id)?;
            if p < 1.0 {
                return Some(0.0);
            }
            Some(((((x - sx).pow(2) + (y - sy).pow(2)) as f32).sqrt()) / p)
        })
        .collect();
    commutes.sort_by(|a, b| a.partial_cmp(b).unwrap());

    Sample {
        // Against the ants that actually exist, not the ants that were
        // asked for -- see the placement loop above.
        survival: (pop_sum / pop_samples.max(1.0)) as f32 / placed.max(1.0),
        travelled: p90,
        commute: if commutes.is_empty() { 0.0 } else { commutes[commutes.len() / 2] },
        feeding: fed.len() as f32 / seen.len().max(1) as f32,
        depth: (depth_sum / depth_n.max(1.0)) as f32,
        eats: ate.len() as f32 / seen.len().max(1) as f32,
        placed,
    }
}
