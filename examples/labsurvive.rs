//! **Does a training batch's colony live in a lab box?** (Teacher lane, 2026-10-24.) The owner's playtest of the
//! trained ant (ANT BRAIN: NEW) found colonies starving within ~5,000 frames of landing beside 8.9 MJ of fruit, and
//! the played bed reproduced it: 41 of 42 starved within 2,000 frames, the hand-built ant 0. Training never asked:
//! its colonies start on 3 full tanks beside a pile of provisions, a lab colony lands on 1 tank among plants and
//! fruit. This runs a lab scenario through the lab's own `Lab` with a given pool as the new ant, so colony
//! selection can score each colony's carry on where the ant will actually live (`colsel.py`'s `+lab` case).
//!
//! ```text
//! cargo run --release --example labsurvive -- founders=a.raw.txt@x:1,b.raw.txt@y:1 scenario=played_bed seed=1 after=15000
//! cargo run --release --example labsurvive -- founders=std scenario=played_bed     # the shipped pool
//! cargo run --release --example labsurvive -- founders=old scenario=played_bed     # the hand-built ant
//! ```
//!
//! `founders=` takes inrun's list (`path[@tag]:count`; the count is ignored, each genome is one pool member), `std`
//! for the shipped pool, or `old` for the hand-built brain. The colony is whatever the scenario lands; the run goes on
//! for `after=` frames past the first frame an ant stands in the box. Prints one `RESULT` line: ants alive at the
//! end, `lived` (the mean headcount over the `after=` frames, graded where alive is all-or-nothing), the most there were, starved, births, and the frame the colony landed. `grant=K` fills every ant to K full
//! tanks on the landing frame, the training bed's founding grant (`grants=3`), to tell a thin start from the brain.
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::training;
use pixel_physics::sim::organism::DeathCause;
use pixel_physics::train;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

fn main() {
    let name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let after: u64 = arg("after").unwrap_or(15_000);
    let limit: u64 = arg("limit").unwrap_or(40_000);
    let grant: f32 = arg("grant").unwrap_or(0.0);
    // `probe=N` (old-ant thread, 2026-10-25): every N frames after landing, one `PROBE` line -- the food trail (scent
    // B) laid over the box (its sum, in full deposits, and the cells holding a tenth of one or more), and how far out
    // the ants are (median and 90th-percentile columns from the colony's first nest site, empty ants beyond 100 and
    // 150 columns, ants carrying food). Unset: no line, the run unchanged.
    let probe: u64 = arg("probe").unwrap_or(0);
    // `trace=N` (Teacher lane, 2026-10-25): every N frames after landing, one `TRACE` line per ant -- frames since
    // landing, its id, energy, lifetime bites, what its crop holds, and how far the nearest plant-owned cell is
    // (Chebyshev, within 12; 99 = none). Built to ask why trained founders starve beside herbs (`lab_herbs`: the
    // hand-built ant keeps ~65, the trained pool ~17) one ant at a time. Unset: no line, the run unchanged.
    let trace: u64 = arg("trace").unwrap_or(0);
    // `food=N` (famine lane, 2026-10-30): every N frames after landing, one `FOOD` line per material that is food
    // (`creature::food_value` > 0, plant-owned cells left out): its cell count and summed worth in the west and east
    // halves of the box, and the highest (smallest y) such cell -- to ask whether a heap the colony starves beside is
    // still there, and whether it is on the surface. Unset: no line, the run unchanged.
    let food: u64 = arg("food").unwrap_or(0);
    // `png=DIR [pngat=F,F,...]` (famine lane, 2026-10-30): the box through `Lab::draw` -- the game's own render, help
    // page off -- saved as `DIR/box_fF.png` (2x, nearest) at each F frames after landing, so a trace can be shown
    // beside the picture of where the ants are. Unset: no file, the run unchanged.
    let png: Option<String> = arg("png");
    let pngat: Vec<u64> = arg::<String>("pngat").map_or(vec![0], |v| v.split(',').map(|x| x.parse().expect("pngat=F,F")).collect());
    let founders: String = arg("founders").expect("founders=LIST|std|old");
    // `nestmap=DIR [nestat=F,F,...]` (nest lane, 2026-10-27): at each F frames after landing (default: the end, `after`),
    // the ground round the first colony's door as `DIR/nestmap_fF.txt` and `DIR/occupants_fF.txt`, in
    // `examples/inrun.rs`'s format, plus `DIR/surface.txt` and one `NESTAT` line (alive, starved so far). The map is
    // **shifted so the door sits at column 128**, inrun's `NEST`, so `scratch/inrun/nestcount.py` and `roomshape.py`
    // read a lab box unchanged. Built for the nest lane's baseline, which judges the trained nest beside the hand-built
    // ant's in the game's own boxes, where labsurvive printed no nest at all. Unset: no file, the run unchanged.
    let nestmap: Option<String> = arg("nestmap");
    let nestat: Vec<u64> = arg::<String>("nestat").map_or(vec![after], |v| {
        v.split(',').map(|x| x.parse().expect("nestat=F,F")).collect()
    });
    let mut sc = Scenario::load(&name).unwrap_or_else(|e| panic!("scenario {name}: {e}"));
    if let Some(sd) = arg::<u64>("seed") {
        sc.bed.seed = sd;
    }
    let bed = sc.bed.clone();
    println!("labsurvive scenario={name} seed={} after={after} grant={grant} founders={founders}", sc.bed.seed);
    let mut lab = pixel_physics::lab::Lab::new(sc.bed.clone());
    lab.load_scenario(sc);
    lab.show_help = false;
    // `tax=K` (old-ant thread, 2026-10-25): the brain's per-link bill (`synapse_fraction`) times K for every ant, to
    // price what a trained mind's ~1,900 live links cost it against the hand-built brain's ~50. Unset: no change.
    if let Some(k) = arg::<f32>("tax") {
        let w = &mut lab.world;
        let sp = w.species.id_of("ant").expect("ant");
        let mut def = w.species.get(sp).creature.clone().expect("creature");
        def.synapse_fraction *= k;
        w.species.set_creature(sp, def);
        println!("tax x{k}: synapse_fraction {}", w.species.get(sp).creature.as_ref().unwrap().synapse_fraction);
    }
    match founders.as_str() {
        "old" => {}
        "std" => match arg::<usize>("member") {
            // `member=K` (queen seat, 2026-10-25): the shipped pool's member K alone, every ant thinking with it --
            // a common garden of the pool, which asks whether choosing *whose* mind an egg carries can change what a
            // colony does. Unset: the whole pool, as before.
            Some(k) => {
                let all = training::new_ant_minds();
                println!("std member {k} of {}", all.len());
                lab.set_new_ant_pool(vec![all[k].clone()]);
            }
            // `knock=F,F,...` (famine lane, 2026-10-30): the shipped pool with every scorer unit's weight on option
            // features F zeroed (`options::feature_names` order: 39 home, 40 last_food, 21 climb, 25 up), a knock-out
            // to ask whether those links are what walks the colony to the end walls and holds it there. A diagnostic,
            // never a candidate ant. Unset: the pool as shipped.
            None => match arg::<String>("knock") {
                Some(k) => {
                    use pixel_physics::sim::creature::{mind, options};
                    let fs: Vec<usize> = k.split(',').map(|x| x.parse().expect("knock=F,F")).collect();
                    let s_feat = mind::FIRST_LINK + mind::UNITS * (mind::CONTEXT + mind::MEMORY + 1);
                    let minds: Vec<_> = training::std_pool()
                        .into_iter()
                        .map(|mut g| {
                            let mut n = 0;
                            for u in 0..mind::UNITS {
                                for &f in &fs {
                                    let i = s_feat + u * options::FEATURES + f;
                                    n += usize::from(g[i] != 0.0);
                                    g[i] = 0.0;
                                }
                            }
                            println!("knock {k}: {n} live links zeroed");
                            train::challenges::colony_bed::mind(&g)
                        })
                        .collect();
                    lab.set_new_ant_pool(minds);
                }
                None => {
                    lab.set_new_ant(true);
                }
            },
        },
        list => {
            let minds: Vec<_> = list
                .split(',')
                .map(|part| {
                    let p = part.rsplit_once(':').map_or(part, |(p, _)| p);
                    let p = p.split_once('@').map_or(p, |(p, _)| p);
                    let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("genome file {p}: {e}"));
                    let mut g = train::parse_genome(&text, train::genome_text_len(&text, p), p);
                    pixel_physics::sim::creature::mind::enter_gate(&mut g, train::genome_text_layout(&text, p));
                    pixel_physics::sim::creature::mind::enter_last_food(&mut g, train::genome_text_layout(&text, p));
                    training::fit_head(&mut g);
                    train::challenges::colony_bed::mind(&g)
                })
                .collect();
            println!("pool of {}", minds.len());
            lab.set_new_ant_pool(minds);
        }
    }
    let starved = DeathCause::Starved as usize;
    let count = |lab: &pixel_physics::lab::Lab| {
        let w = &lab.world;
        let sp = w.species.id_of("ant");
        w.live_organism_ids().into_iter().filter(|&id| w.organism(id).is_some_and(|s| Some(s.species) == sp)).count()
    };
    // `lived` sums the headcount over every frame after landing: a graded score where `alive` is all-or-nothing on a
    // bed where the whole colony starves (a colony that dies at 9,000 frames beats one that dies at 2,000).
    let (mut landed, mut peak, mut lived) = (None::<u64>, 0usize, 0u64);
    let s0 = lab.world.deaths_by_cause[starved];
    // SCRATCH (copy test, 2026-10-28): `PP_DECLOG=1` turns the decision trace on (it draws nothing and changes no
    // branch), so the shadow log's `W` rows can say why the old ant's walk did what it did. Only the last few frames
    // are kept: a row is read on the ant's next tick.
    let declog = std::env::var("PP_DECLOG").is_ok_and(|v| v == "1");
    if declog {
        lab.world.decision_log = Some(Vec::new());
    }
    loop {
        lab.tick_for_harness();
        let f = lab.world.frame;
        if declog && f.is_multiple_of(16) {
            if let Some(log) = lab.world.decision_log.as_mut() {
                log.retain(|r| r.frame + 4 >= f);
            }
        }
        let n = count(&lab);
        peak = peak.max(n);
        if landed.is_some() {
            lived += n as u64;
        }
        if landed.is_none() && n > 0 {
            landed = Some(f);
            if grant > 0.0 {
                println!("granted {} ants {grant} tanks", lab.grant_ants(grant));
            }
        }
        if probe > 0 && landed.is_some_and(|l| f > l && (f - l).is_multiple_of(probe)) {
            probe_line(&lab, f - landed.unwrap());
        }
        if let (Some(dir), Some(l)) = (&nestmap, landed) {
            if f > l && nestat.contains(&(f - l)) {
                write_nest(&lab, &bed, std::path::Path::new(dir), f - l);
                println!("NESTAT f={} alive={} starved={}", f - l, count(&lab), lab.world.deaths_by_cause[starved] - s0);
            }
        }
        if let (Some(dir), Some(l)) = (&png, landed) {
            if pngat.contains(&(f - l)) {
                use pixel_physics::lab::{HEIGHT, WIDTH};
                std::fs::create_dir_all(dir).unwrap();
                let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
                lab.draw(&mut buf, 60.0);
                let img = image::RgbaImage::from_raw(WIDTH, HEIGHT, buf).expect("frame");
                let img = image::imageops::resize(&img, WIDTH * 2, HEIGHT * 2, image::imageops::FilterType::Nearest);
                img.save(std::path::Path::new(dir).join(format!("box_f{:05}.png", f - l))).unwrap();
                println!("PNG f={} alive={}", f - l, count(&lab));
            }
        }
        if food > 0 && landed.is_some_and(|l| f >= l && (f - l).is_multiple_of(food)) {
            food_lines(&lab, f - landed.unwrap());
        }
        if trace > 0 && landed.is_some_and(|l| f > l && (f - l).is_multiple_of(trace)) {
            trace_lines(&lab, f - landed.unwrap());
        }
        if landed.is_some_and(|l| f >= l + after) || (landed.is_none() && f >= limit) {
            break;
        }
    }
    let w = &lab.world;
    let births = w.creature_stats.births;
    println!(
        "RESULT alive={} lived={:.1} peak={peak} starved={} births={births} landed={} frame={} digs={} deliveries={} trip_deliveries={}",
        count(&lab),
        lived as f64 / after.max(1) as f64,
        w.deaths_by_cause[starved] - s0,
        landed.map_or(-1, |l| l as i64),
        w.frame,
        w.creature_stats.digs,
        w.creature_stats.deliveries,
        // Food bitten on a trip and put down at home, once each: `deliveries` also counts crumbs shuffled inside
        // the nest (copy test, 2026-10-27; `trip_deliveries` is the foraging measure, PR 614).
        w.creature_stats.trip_deliveries
    );
    // The copy log is buffered: without this a short run writes nothing and a long one loses its tail.
    pixel_physics::sim::creature::mind::scratch_log::copy_flush();
}

/// `nestmap=` ([`main`]): the ground round the first nest, as `examples/inrun.rs`'s `write_nestmap` and
/// `write_occupants` write it (same window: 100 columns each side of the door, rows 40..128 below the box's own datum
/// row less 48), with every column shifted so the door is at 128. `surface.txt` is the datum per shifted column.
fn write_nest(lab: &pixel_physics::lab::Lab, bed: &pixel_physics::lab::scene::LabBox, dir: &std::path::Path, f: u64) {
    use pixel_physics::sim::material;
    use std::io::Write;
    const NEST: i32 = 128;
    const HW: i32 = 100;
    let w = &lab.world;
    std::fs::create_dir_all(dir).unwrap();
    let door = w.nest_sites.first().map_or(bed.width / 2, |s| s.x);
    let shift = door - NEST;
    let surf = |x: i32| w.room_surface_at(x).unwrap_or(bed.ground_y);
    let mut sf = std::fs::File::create(dir.join("surface.txt")).unwrap();
    for x in 0..NEST + HW + 1 {
        writeln!(sf, "{}", surf(x + shift)).unwrap();
    }
    let (y0, y1) = (surf(door) - 48, (surf(door) + 40).min(bed.height));
    let food = w.materials.id_of("provisions");
    let mut legend: Vec<String> = Vec::new();
    let (mut rows, mut occ) = (Vec::new(), Vec::new());
    for y in y0..y1 {
        let (mut row, mut orow) = (String::new(), String::new());
        for x in door - HW..=door + HW {
            let c = w.get(x, y);
            let id = c.organism_id();
            let creature = w.materials.kind(c.material) == material::MaterialKind::Creature;
            orow.push(if let Some(b) = (id != 0).then(|| w.organism(id).and_then(|s| s.brood)).flatten() {
                ['e', 'l', 'p'][b.stage as usize]
            } else if id != 0 || creature {
                'a'
            } else if Some(c.material) == food {
                'f'
            } else {
                '.'
            });
            if id != 0 && w.organism(id).is_some_and(|s| w.species.get(s.species).creature.is_none()) {
                row.push('%');
                continue;
            }
            if id != 0 || Some(c.material) == food || creature {
                row.push('?');
                continue;
            }
            let name = w.materials.get(c.material).name.to_string();
            let k = legend.iter().position(|n| *n == name).unwrap_or_else(|| {
                legend.push(name);
                legend.len() - 1
            });
            row.push(char::from(b'A' + k as u8));
        }
        rows.push(row);
        occ.push(orow);
    }
    let mut m = std::fs::File::create(dir.join(format!("nestmap_f{f:07}.txt"))).unwrap();
    writeln!(m, "{} {y0} {}", NEST - HW, legend.join(",")).unwrap();
    for r in rows {
        writeln!(m, "{r}").unwrap();
    }
    let mut o = std::fs::File::create(dir.join(format!("occupants_f{f:07}.txt"))).unwrap();
    writeln!(o, "{} {y0}", NEST - HW).unwrap();
    for r in occ {
        writeln!(o, "{r}").unwrap();
    }
}

/// One `probe=` line ([`main`]): the food trail and how far out the colony is.
fn probe_line(lab: &pixel_physics::lab::Lab, since: u64) {
    use pixel_physics::sim::pheromone::{Channel, Scent};
    let w = &lab.world;
    let (mut b_sum, mut b_cells) = (0.0f64, 0usize);
    for y in 0..320 {
        for x in 0..512 {
            let b = w.pheromone_at(Channel::B, x, y) as f64 / Scent::MAX as f64;
            b_sum += b;
            b_cells += usize::from(b >= 0.1);
        }
    }
    let door = w.nest_sites.first().map_or(256, |s| s.x);
    let sp = w.species.id_of("ant");
    let mut d: Vec<i32> = Vec::new();
    let (mut far100, mut far150, mut laden) = (0, 0, 0);
    for id in w.live_organism_ids() {
        let Some(s) = w.organism(id).filter(|s| Some(s.species) == sp) else { continue };
        let Some(&(hx, _)) = s.chain.first() else { continue };
        let dx = (hx - door).abs();
        d.push(dx);
        let full = s.crop.is_some_and(|c| c.cells > 0);
        laden += usize::from(full);
        far100 += usize::from(!full && dx > 100);
        far150 += usize::from(!full && dx > 150);
    }
    d.sort();
    let q = |p: f64| d.get(((d.len() as f64 - 1.0) * p).round() as usize).copied().unwrap_or(-1);
    println!("PROBE since={since} ants={} b_sum={b_sum:.2} b_cells={b_cells} col_med={} col_p90={} empty_past100={far100} empty_past150={far150} laden={laden}", d.len(), q(0.5), q(0.9));
}

/// The `food=` lines ([`main`]): per food material, cells and worth in each half of the box.
fn food_lines(lab: &pixel_physics::lab::Lab, since: u64) {
    use std::collections::BTreeMap;
    let w = &lab.world;
    let mut by: BTreeMap<String, [f64; 5]> = BTreeMap::new();
    for y in 0..320 {
        for x in 0..512 {
            let c = w.get(x, y);
            let oid = c.organism_id();
            if oid != 0 && w.organism(oid).is_some() {
                continue;
            }
            let v = pixel_physics::sim::creature::food_value(w, c);
            if v <= 0.0 {
                continue;
            }
            let e = by.entry(w.materials.get(c.material).name.to_string()).or_insert([0.0, 0.0, 0.0, 0.0, 999.0]);
            let k = if x < 256 { 0 } else { 2 };
            e[k] += 1.0;
            e[k + 1] += v as f64;
            e[4] = e[4].min(y as f64);
        }
    }
    for (m, e) in by {
        println!("FOOD {since} {m} west={} {:.0} east={} {:.0} top_y={}", e[0], e[1], e[2], e[3], e[4]);
    }
}

/// The `trace=` lines ([`main`]): one per ant.
fn trace_lines(lab: &pixel_physics::lab::Lab, since: u64) {
    let w = &lab.world;
    let sp = w.species.id_of("ant");
    let plant_owned = |x: i32, y: i32| {
        let oid = w.get(x, y).organism_id();
        oid != 0 && w.organism(oid).is_some_and(|o| w.species.get(o.species).creature.is_none())
    };
    for id in w.live_organism_ids() {
        let Some(s) = w.organism(id).filter(|s| Some(s.species) == sp && s.brood.is_none()) else { continue };
        let Some(&(hx, hy)) = s.chain.first() else { continue };
        let mut pd = 99;
        'r: for d in 0..=12i32 {
            for dy in -d..=d {
                for dx in -d..=d {
                    if dx.abs().max(dy.abs()) == d && plant_owned(hx + dx, hy + dy) {
                        pd = d;
                        break 'r;
                    }
                }
            }
        }
        let crop = s.crop.as_ref().map(|c| w.materials.get(c.material).name.clone()).unwrap_or_default();
        println!("TRACE {since} {id} {:.1} {} {pd} {hx} {hy} {crop}", s.energy, s.life.bites);
    }
}
