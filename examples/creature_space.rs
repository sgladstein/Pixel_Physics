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
}

const DESCRIPTORS: [&str; 4] = ["travelled", "commute", "feeding", "depth"];

impl Sample {
    fn descriptors(&self) -> [f32; 4] {
        [self.travelled, self.commute, self.feeding, self.depth]
    }
}

fn main() {
    let mut genomes = 48usize;
    let mut seeds = 2u64;
    let mut frames = 6000usize;
    let mut bins = 3usize;
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

    if mode == "economy" {
        economy_sweep(seeds, frames);
        return;
    }
    println!("creature space: {genomes} random genomes + 2 references, {seeds} seeds x {frames} frames each");
    println!("scene: scarce food (2 trees), 52 ants, 4 beetles, generated terrain\n");

    let mut labelled: Vec<(String, Sample)> = Vec::new();

    // The two reference points, so the random distribution has something to
    // be read against. A zero genome cannot move at all, which makes it the
    // floor rather than a competitor.
    for (label, genome) in [("zero".to_string(), vec![0.0; GENOME_LEN]), ("authored".to_string(), authored_genome())] {
        labelled.push((label, mean_of((0..seeds).map(|s| run_one(&genome, frames, 0xC0DE + s, DEFAULT_ECONOMY)).collect())));
    }
    for g in 0..genomes {
        let genome = random_genome(0x5EED + g as u64);
        let s = mean_of((0..seeds).map(|s| run_one(&genome, frames, 0xC0DE + s, DEFAULT_ECONOMY)).collect());
        labelled.push((format!("r{g:03}"), s));
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
fn economy_sweep(seeds: u64, frames: usize) {
    println!("energy economy sweep: does foraging pay, and is food still scarce?
");
    println!("{:<10} {:>10} {:>7} {:>10} {:>9} {:>11}", "eat", "move_cost", "trees", "forager", "immobile", "advantage");
    let zero = vec![0.0; GENOME_LEN];
    let authored = authored_genome();
    let mut best: Option<(f32, String)> = None;
    for &eat_energy in &[120.0f32, 300.0, 700.0] {
        for &move_cost in &[0.25f32, 0.08, 0.02] {
            for &trees in &[2i32, 6, 14] {
                let econ = Economy { eat_energy, move_cost, trees };
                let f = mean_of((0..seeds).map(|s| run_one(&authored, frames, 0xC0DE + s, econ)).collect()).survival;
                let z = mean_of((0..seeds).map(|s| run_one(&zero, frames, 0xC0DE + s, econ)).collect()).survival;
                let adv = f - z;
                println!("{eat_energy:<10.0} {move_cost:>10.2} {trees:>7} {f:>10.3} {z:>9.3} {adv:>11.3}");
                // Scarcity guard: an advantage bought by making food
                // abundant is not the band we are looking for.
                if f < 0.9 {
                    if best.as_ref().is_none_or(|(b, _)| adv > *b) {
                        best = Some((adv, format!("eat {eat_energy:.0}, move {move_cost:.2}, {trees} trees (forager {f:.3})")));
                    }
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

const DEFAULT_ECONOMY: Economy = Economy { eat_energy: 120.0, move_cost: 0.25, trees: 2 };

const ANTS: usize = 52;
const BEETLES: usize = 9;

/// The three knobs that decide whether foraging is worth doing.
#[derive(Clone, Copy)]
struct Economy {
    eat_energy: f32,
    move_cost: f32,
    trees: i32,
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
    def.start_energy = 90.0;
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
    def.food = vec!["leaf".to_string()];
    world.species.set_creature(species, def);
    let ant_material = world.materials.id_of("ant").expect("ant");

    let (presets, _) = pixel_physics::worldgen::WorldgenPresets::load();
    let Some(params) = presets.get(&presets.default_name()) else { return Sample::default() };
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
    for i in 0..ANTS {
        let x = 24 + i as i32 * 4;
        world.plant_ant(x, surface_at[x as usize] - 1);
    }
    // **Dangerous, and dangerous *where the ants are*.** Placed beyond the
    // colony's range, beetles punished only the ants that travelled -- so
    // sitting still at the nest was both cheap and safe, and the zero
    // genome (which cannot move at all) beat every forager. Danger that
    // only reaches the active is danger that selects for inactivity.
    //
    // Beetles are wide, eat ants, and cannot dig. An ant that burrows is
    // out of reach; nothing in the engine says so.
    for i in 0..BEETLES {
        let x = 40 + i as i32 * 45;
        pixel_physics::sim::creature::plant_creature_seed(&mut world, x, surface_at[x as usize] - 1, "beetle")
            .map(|s| world.schedule_active_site(s));
    }

    let mut path: std::collections::HashMap<u16, f32> = std::collections::HashMap::new();
    let mut last: std::collections::HashMap<u16, (i32, i32)> = std::collections::HashMap::new();
    let mut start: std::collections::HashMap<u16, (i32, i32)> = std::collections::HashMap::new();
    let mut furthest: std::collections::HashMap<u16, f32> = std::collections::HashMap::new();
    let mut fed: std::collections::HashSet<u16> = std::collections::HashSet::new();
    let mut seen: std::collections::HashSet<u16> = std::collections::HashSet::new();
    let (mut pop_sum, mut pop_samples, mut depth_sum, mut depth_n) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);

    for frame in 0..frames {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
        if frame % 6 != 0 {
            continue;
        }
        let mut alive = 0.0;
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
            }
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
        survival: (pop_sum / pop_samples.max(1.0)) as f32 / ANTS as f32,
        travelled: p90,
        commute: if commutes.is_empty() { 0.0 } else { commutes[commutes.len() / 2] },
        feeding: fed.len() as f32 / seen.len().max(1) as f32,
        depth: (depth_sum / depth_n.max(1.0)) as f32,
    }
}
