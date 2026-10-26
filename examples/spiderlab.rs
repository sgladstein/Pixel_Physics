//! **Does a spider population live in a lab box?** (lone predator thread, 2026-10-24; owner: "We should test if a
//! spider population survives / is sustainable in the lab".) Training never asks: a training bed refills every
//! spider's place from the pool and breeds by selection. This lands a lab scenario through the lab's own `Lab` with
//! the shipped ant pool, then sets down `spiders=N` spiders with minds from `pool=` once the colony stands, and lets
//! the game alone decide who lives and who is born -- a spider buds by its species' own bar
//! (`reproduce_threshold`), its child inheriting its mind (`mind::inherit`). Nothing is refilled.
//!
//! ```text
//! PIXEL_PHYSICS_LAB_SCENARIOS=scratch/inrun/labscen cargo run --release --example spiderlab -- \
//!     scenario=lab_mid_fruit pool=DIR spiders=4 frames=400000 seed=1 [every=5000] [out=DIR] [ant=old|std] [delay=20000] [snap=F1,F2 out=DIR]
//!     [species=spider|centipede] [gif=START:END:EVERY crop=X,Y,W,H scale=3 out=DIR] [far=30]
//!     (`pool=hunter`: a `mind::preset` on the predator's verbs)
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
    // `pool=lab`: the lab game's own pool for the species (`lab::training::predator_minds`), the one a hand-stocked
    // predator gets.
    let pool: String = arg("pool").expect("pool=DIR of spider .raw.txt, or pool=lab");
    // `species=`: the predator, `spider` (default) or the nest raider `centipede`.
    let species: String = arg("species").unwrap_or_else(|| "spider".to_string());
    let mut sc = Scenario::load(&name).unwrap_or_else(|e| panic!("scenario {name}: {e}"));
    if let Some(sd) = arg::<u64>("seed") {
        sc.bed.seed = sd;
    }
    let seed = sc.bed.seed;
    // `pool=hunter` (or any `mind::preset` name): that hand-built mind on the predator's verbs.
    let preset = pixel_physics::sim::creature::mind::preset(&pool, pixel_physics::train::predator::verbs_for(&species));
    let mut files: Vec<std::path::PathBuf> = if pool == "lab" || preset.is_some() {
        Vec::new()
    } else {
        std::fs::read_dir(&pool)
            .unwrap_or_else(|e| panic!("pool={pool}: {e}"))
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.to_string_lossy().ends_with(".raw.txt"))
            .collect()
    };
    files.sort();
    let minds: Vec<_> = if let Some(g) = &preset {
        vec![pixel_physics::train::predator::mind_for(&species, g)]
    } else if pool == "lab" {
        pixel_physics::lab::training::predator_minds(&species).unwrap_or_else(|| panic!("pool=lab: no lab pool for {species}"))
    } else {
        files
            .iter()
            .map(|p| {
                let text = std::fs::read_to_string(p).unwrap();
                pixel_physics::train::predator::mind_for(&species, &train::parse_genome(&text, pixel_physics::sim::creature::mind::GENOME_LEN, &p.display().to_string()))
            })
            .collect()
    };
    assert!(!minds.is_empty(), "pool={pool}: no .raw.txt");
    // Each mind's breeding genes (`NN.body.txt`, `predator::BREEDING`): here they act, for spiders breed by the game.
    let bodies: Vec<Option<[f32; 2]>> = if files.is_empty() { vec![None; minds.len()] } else { files.iter().map(|p| pixel_physics::train::predator::read_breeding(p)).collect() };
    // `snap=F1,F2,...` with `out=DIR`: the lab's own picture of the box at those frames, `DIR/snap_FRAME.png`.
    let snaps: Vec<u64> = arg::<String>("snap").map(|v| v.split(',').map(|f| f.parse().expect("snap=F1,F2,...")).collect()).unwrap_or_default();
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
    let spider = lab.world.species.id_of(&species).unwrap_or_else(|| panic!("species={species}: not in the table"));
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
    // `gif=START:END:EVERY` with `out=`: `DIR/lab.gif`, the cells of `crop=X,Y,W,H` (default the whole box) every
    // EVERY frames, by `predator::picture` (ants yellow, the predator red, tunnels dark, a white ring where an ant
    // was killed).
    let gif: Option<(u64, u64, u64)> = arg::<String>("gif").map(|v| {
        let p: Vec<u64> = v.split(':').map(|x| x.parse().expect("gif=START:END:EVERY")).collect();
        (p[0], p[1], p[2].max(1))
    });
    let crop = arg::<String>("crop").map_or_else(
        || lab.world.bounds().map(|b| (b.min_x, b.min_y, b.max_x - b.min_x + 1, b.max_y - b.min_y + 1)).expect("a bounded box"),
        |v| {
            let p: Vec<i32> = v.split(',').map(|x| x.parse().expect("crop=X,Y,W,H")).collect();
            (p[0], p[1], p[2], p[3])
        },
    );
    let scale: u32 = arg("scale").unwrap_or(3);
    // `far=`: the fewest columns from any nest door a predator lands (default 30). `far=0` lands it within 6
    // columns of a door: the raid itself, without the walk to find the nest.
    let far: i32 = arg("far").unwrap_or(30);
    let ant_sp = lab.world.species.id_of("ant").expect("ant");
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    let mut kills_seen: Vec<(u64, (i32, i32))> = Vec::new();
    let mut last_at: std::collections::HashMap<u32, (i32, i32)> = Default::default();
    let mut nest_kills = 0u64;
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
            if placed == 0 {
                println!("landing at frame {}: nest doors {:?}, box {:?}", w.frame, w.nest_sites.iter().map(|s| (s.x, s.y)).collect::<Vec<_>>(), (x0, x1));
            }
            for _ in 0..200 {
                if placed >= n {
                    break;
                }
                let x = x0 + 8 + rng.below((x1 - x0 - 16) as usize) as i32;
                if doors.iter().any(|d| (d - x).abs() < far) || (far == 0 && !doors.iter().any(|d| (d - x).abs() <= 6)) {
                    continue;
                }
                if let Some(id) = pixel_physics::train::predator::place_predator(w, &species, x, colony, minds[placed % minds.len()].clone(), 1.0) {
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
                // `PP_BIRTHLOG=1`: each birth as it is seen -- how a centipede budding three times in 23 frames at a door
                // was found (the second mouth, `creature::provision_at`).
                if std::env::var("PP_BIRTHLOG").is_ok() {
                    let s = w.organism(id).expect("alive");
                    eprintln!("birth f{} id {id} cells {} energy {:.0} crop {:?} head {:?}", w.frame, s.cells.len(), s.energy, s.crop, s.chain.first());
                }
            }
        }
        for r in w.death_log.as_mut().map(std::mem::take).unwrap_or_default() {
            if r.species == spider {
                deaths[r.cause.index().min(63)] += 1;
            }
            if r.species == ant_sp && r.cause == DeathCause::Killed {
                if let Some(&at) = last_at.get(&r.id) {
                    kills_seen.push((r.frame, at));
                    // Killed under the ground: a raid (`nest_kills` in the RESULT line).
                    let top = pixel_physics::train::predator::surface(w, at.0);
                    if at.1 > top {
                        nest_kills += 1;
                    }
                    if std::env::var("PP_KILLLOG").is_ok() {
                        eprintln!("kill f{} at {:?} surface {top}", r.frame, at);
                    }
                }
            }
        }
        let f = w.frame;
        // Every ant's head every frame once predators are in, so a kill has a place (its body is gone by the time the
        // death log is read).
        if placed > 0 {
            last_at.clear();
            for id in pixel_physics::train::predator::of_species(w, ant_sp) {
                if let Some(h) = w.organism(id).and_then(|s| s.chain.first().copied()) {
                    last_at.insert(id, h);
                }
            }
        }
        if let Some((g0, g1, ge)) = gif {
            if f + 1 >= g0 && f <= g1 {
                kills_seen.retain(|(t, _)| f.saturating_sub(*t) < 120);
                if f >= g0 && (f - g0).is_multiple_of(ge) {
                    shots.push(pixel_physics::train::predator::picture(w, crop, scale, &kills_seen, f, ant_sp, spider));
                }
            }
        }
        if snaps.contains(&f) {
            if let Some(d) = arg::<String>("out") {
                let (pw, ph) = (pixel_physics::lab::WIDTH, pixel_physics::lab::HEIGHT);
                let mut buf = vec![0u8; (pw * ph * 4) as usize];
                lab.show_help = false;
                lab.draw(&mut buf, 60.0);
                let heads: Vec<(i32, i32)> = pixel_physics::train::predator::of_species(&lab.world, spider).iter().filter_map(|&id| lab.world.organism(id).and_then(|s| s.chain.first().copied())).collect();
                println!("snap {f}: spider heads {heads:?}");
                image::RgbaImage::from_raw(pw, ph, buf).expect("a frame").save(std::path::Path::new(&d).join(format!("snap_{f}.png"))).unwrap();
            }
        }
        let s = count(&lab, &species);
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
    if !shots.is_empty() {
        let d = arg::<String>("out").expect("gif= needs out=DIR");
        pixel_physics::train::predator::save_gif(shots, &std::path::Path::new(&d).join("lab.gif"), 80);
    }
    // The minds' own tallies: with the lab's hand-built ant (`ant=old`) the spiders are the only minds in the box, so
    // these are theirs -- bites landed, kills, cells of flesh swallowed, and Eats (a cell finished at once rather
    // than at the gut's rate).
    let mc = lab.world.minds.as_ref().map(|m| (m.counts.bites, m.counts.kills, m.counts.flesh, m.counts.ate)).unwrap_or_default();
    println!(
        "RESULT spiders={} peak={peak} placed={placed} births={births} starved={} killed={} old={} ants={} ants_killed={} coexist={:.2} spider_mean={:.1} ant_mean={:.1} mind_bites={} mind_kills={} mind_flesh={} mind_ate={} nest_kills={nest_kills}",
        count(&lab, &species),
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
