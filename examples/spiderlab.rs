//! **Does a spider population live in a lab box?** (lone predator thread, 2026-10-24; owner: "We should test if a
//! spider population survives / is sustainable in the lab".) Training never asks: a training bed refills every
//! spider's place from the pool and breeds by selection. This lands a lab scenario through the lab's own `Lab` with
//! the shipped ant pool, then sets down `spiders=N` spiders with minds from `pool=` once the colony stands, and lets
//! the game alone decide who lives and who is born -- a spider buds by its species' own bar
//! (`reproduce_threshold`), its child inheriting its mind (`mind::inherit`). Nothing is refilled.
//!
//! ```text
//! PIXEL_PHYSICS_LAB_SCENARIOS=scratch/inrun/labscen cargo run --release --example spiderlab -- \
//!     scenario=lab_mid_fruit pool=DIR spiders=4 frames=400000 seed=1 [every=5000] [out=DIR] [ant=old|std] [delay=20000]
//! ```
//!
//! Prints one row per `every` frames (`frame, spiders, ants, spider births, spider deaths by cause, ants killed`)
//! and a `RESULT` line -- with `coexist`, the share of samples since the spiders landed in which spiders and ants were
//! both alive, and each headcount's mean -- and `out=` also writes the rows as `spiderlab.csv`.
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::sim::organism::DeathCause;
use pixel_physics::train;
use std::io::Write;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

fn main() {
    let name: String = arg("scenario").unwrap_or_else(|| "lab_mid_fruit".to_string());
    let frames: u64 = arg("frames").unwrap_or(400_000);
    let every: u64 = arg("every").unwrap_or(5_000);
    let n: usize = arg("spiders").unwrap_or(4);
    // `delay=`: frames after the first ant stands before the spiders land. Set down beside a colony still landing,
    // they ate it before it had a nest: every one of 18 first runs (2 boxes x 1/2/4 spiders x 3 seeds) wiped it.
    let delay: u64 = arg("delay").unwrap_or(20_000);
    let pool: String = arg("pool").expect("pool=DIR of spider .raw.txt");
    let mut sc = Scenario::load(&name).unwrap_or_else(|e| panic!("scenario {name}: {e}"));
    if let Some(sd) = arg::<u64>("seed") {
        sc.bed.seed = sd;
    }
    let seed = sc.bed.seed;
    let mut files: Vec<_> = std::fs::read_dir(&pool)
        .unwrap_or_else(|e| panic!("pool={pool}: {e}"))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(".raw.txt"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "pool={pool}: no .raw.txt");
    let minds: Vec<_> = files
        .iter()
        .map(|p| {
            let text = std::fs::read_to_string(p).unwrap();
            pixel_physics::train::predator::spider_mind(&train::parse_genome(&text, pixel_physics::sim::creature::mind::GENOME_LEN, &p.display().to_string()))
        })
        .collect();
    // Each mind's breeding genes (`NN.body.txt`, `predator::BREEDING`): here they act, for spiders breed by the game.
    let bodies: Vec<Option<[f32; 2]>> = files.iter().map(|p| pixel_physics::train::predator::read_breeding(p)).collect();
    println!("spiderlab scenario={name} seed={seed} spiders={n} pool={pool} ({} minds) frames={frames}", minds.len());
    let mut lab = pixel_physics::lab::Lab::new(sc.bed.clone());
    lab.load_scenario(sc);
    // `ant=std` the shipped trained pool; the default is the lab's own ant today, the hand-built one (the trained
    // pool is held off the lab until a pool survives its boxes -- and in lab_mid_fruit it does not, spiders or none).
    if arg::<String>("ant").as_deref() == Some("std") {
        lab.set_new_ant(true);
    }
    let count = |lab: &pixel_physics::lab::Lab, species: &str| {
        let w = &lab.world;
        let sp = w.species.id_of(species);
        w.live_organism_ids().into_iter().filter(|&id| w.organism(id).is_some_and(|s| Some(s.species) == sp)).count()
    };
    let spider = lab.world.species.id_of("spider").expect("spider species");
    let mut rng = train::Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x5D1D);
    let (mut placed, mut colony) = (0usize, None);
    let mut seen: std::collections::HashSet<u32> = Default::default();
    let (mut births, mut deaths) = (0u64, [0u64; 64]);
    lab.world.death_log = Some(Vec::new());
    let mut out = arg::<String>("out").map(|d| {
        std::fs::create_dir_all(&d).unwrap();
        std::fs::File::create(std::path::Path::new(&d).join("spiderlab.csv")).unwrap()
    });
    if let Some(f) = out.as_mut() {
        writeln!(f, "frame,spiders,ants,spider_births,spider_starved,spider_killed,spider_old,ants_killed").unwrap();
    }
    let killed0 = lab.world.deaths_by_cause[DeathCause::Killed as usize];
    let mut peak = 0usize;
    let mut landed: Option<u64> = None;
    // Graded, over the samples since the spiders were set down: how often both kinds were alive (`coexist`, the
    // sustainability the owner asked about), and the mean of each headcount.
    let (mut samples, mut both, mut s_sum, mut a_sum) = (0u64, 0u64, 0u64, 0u64);
    while lab.world.frame < frames {
        lab.tick_for_harness();
        let w = &mut lab.world;
        // Set the spiders down once the colony stands, away from every nest door, the first founding the predators'
        // colony so spiders are kin and do not bite each other.
        if landed.is_none() && w.live_organism_ids().iter().any(|&id| w.organism(id).is_some_and(|s| Some(s.species) == w.species.id_of("ant"))) {
            landed = Some(w.frame);
        }
        if placed < n && landed.is_some_and(|l| w.frame >= l + delay) {
            let (x0, _, x1, _) = w.bounds().map(|b| (b.min_x, b.min_y, b.max_x, b.max_y)).expect("a lab box is bounded");
            let doors: Vec<i32> = w.nest_sites.iter().map(|s| s.x).collect();
            for _ in 0..200 {
                if placed >= n {
                    break;
                }
                let x = x0 + 8 + rng.below((x1 - x0 - 16) as usize) as i32;
                if doors.iter().any(|d| (d - x).abs() < 30) {
                    continue;
                }
                if let Some(id) = pixel_physics::train::predator::place_spider(w, x, colony, minds[placed % minds.len()].clone(), 1.0) {
                    if let Some(genes) = bodies[placed % minds.len()] {
                        pixel_physics::train::predator::set_breeding(w, id, genes);
                    }
                    colony = colony.or_else(|| w.organism(id).map(|s| s.colony));
                    seen.insert(id);
                    placed += 1;
                }
            }
        }
        // Births: a living spider this harness never placed or saw.
        for id in w.live_organism_ids() {
            if w.organism(id).is_some_and(|s| s.species == spider) && seen.insert(id) {
                births += 1;
            }
        }
        for r in w.death_log.as_mut().map(std::mem::take).unwrap_or_default() {
            if r.species == spider {
                deaths[r.cause.index().min(63)] += 1;
            }
        }
        let f = w.frame;
        let s = count(&lab, "spider");
        peak = peak.max(s);
        if f.is_multiple_of(every) {
            let a = count(&lab, "ant");
            if placed > 0 {
                samples += 1;
                both += u64::from(s > 0 && a > 0);
                s_sum += s as u64;
                a_sum += a as u64;
            }
            let row = format!(
                "{f},{s},{},{births},{},{},{},{}",
                count(&lab, "ant"),
                deaths[DeathCause::Starved.index()],
                deaths[DeathCause::Killed.index()],
                deaths[DeathCause::OldAge.index()],
                lab.world.deaths_by_cause[DeathCause::Killed as usize] - killed0
            );
            println!("{row}");
            if let Some(o) = out.as_mut() {
                writeln!(o, "{row}").unwrap();
            }
        }
    }
    // The minds' own tallies: with the lab's hand-built ant (`ant=old`) the spiders are the only minds in the box, so
    // these are theirs -- bites landed, kills, cells of flesh swallowed, and Eats (a cell finished at once rather
    // than at the gut's rate).
    let mc = lab.world.minds.as_ref().map(|m| (m.counts.bites, m.counts.kills, m.counts.flesh, m.counts.ate)).unwrap_or_default();
    println!(
        "RESULT spiders={} peak={peak} placed={placed} births={births} starved={} killed={} old={} ants={} ants_killed={} coexist={:.2} spider_mean={:.1} ant_mean={:.1} mind_bites={} mind_kills={} mind_flesh={} mind_ate={}",
        count(&lab, "spider"),
        deaths[DeathCause::Starved.index()],
        deaths[DeathCause::Killed.index()],
        deaths[DeathCause::OldAge.index()],
        count(&lab, "ant"),
        lab.world.deaths_by_cause[DeathCause::Killed as usize] - killed0,
        both as f64 / samples.max(1) as f64,
        s_sum as f64 / samples.max(1) as f64,
        a_sum as f64 / samples.max(1) as f64,
        mc.0,
        mc.1,
        mc.2,
        mc.3,
    );
}
