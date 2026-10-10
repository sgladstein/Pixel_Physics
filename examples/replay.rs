//! **Replay of the owner's 2026-10-09 `herb_ant` playtest -- the same bed with and without the colonies.**
//!
//! Rebuilds the bed from the playtest's own actions log
//! (`Reports/playtest-2026-10-09-herb-ant/playtest/actions.csv`): a 1,024 x 512 box with 176 rows of soil, no
//! founders, the `PLANT_LOAD_FAILURE=false` and `DEVELOPMENTAL_KEY=1` dials, the thirteen `PLANTED` lines at frame
//! 0, and the two colonies at the frames the log gives. **`ants=0` is the same bed with no colonies: the control a
//! playtest cannot have.** `Reports/playtest-2026-10-09-herb-ant/README.md` is what it was built to answer, and
//! `run.sh` beside it is every arm that report ran.
//!
//! **Measured on the playtest's own build, `1bb916c1`** (branch `claude/eloquent-johnson-axjvu1-nest-life`), whose
//! switch bundle includes `LAY_BRAKE` and `RECRUIT`. This file compiles on main, which lacks both, so a run there
//! will not reproduce that report's numbers: run it on that branch, or on main once the branch lands.
//!
//! ```text
//! set -o pipefail; cargo build --release --examples      # then export the chronicle header's SWITCHES line
//! replay frames=450000 seed=1 ants=1 rain=off pace=half out=DIR    # as played
//! replay frames=450000 seed=1 ants=0 rain=off pace=half out=DIR    # the no-ant twin
//! ```
//!
//! **Two bed settings the actions log does not record: `rain=off pace=half`.** `CycleRain`, `CyclePlantPace`,
//! `ToggleWindfallRot` and the lamp toggles are not logged, and the shipped rain is light; rain off + half pace is the
//! only pair that reproduces the playtest's census exactly through tick 40,000. It does not stay exact (adults at
//! 100k: 205 here, 165 played), so a replay is *the owner's bed*, not *the owner's run*: compare arms against each
//! other, paired by seed, never against the chronicle's own numbers.
//!
//! Arguments (**an unknown one is ignored: read the echoed first line**): `frames= seed= ants=0|1 rain=off|light|
//! steady|heavy pace=full|half|quarter out=DIR probe= snap= antsnap= t1= t2=` (probe/snap/antsnap are tick intervals;
//! t1/t2 are the two placement frames), `assets=DIR` (`materials.reload` over a directory of `.ron` files, how the
//! leaf-inedible and leaf-dose arms are made), `doortree=1` (a tree at x=748 and a conifer at x=304 planted at frame
//! 0, about 20 columns from where the colonies found), `png=F,F,..` (a 2 px/cell raster of rows 150-345).
//!
//! Writes into `out=DIR`:
//!   probe.csv     one row per `probe=` ticks: stand and larder in RAW CELLS, and the roots / standing plant /
//!                 empty-below counts in a +-40 column box round each colony (x=284 and x=728)
//!   ledger.csv    per-colony energy accounts and diet by material (`diet:<material>`), long form
//!   deaths.csv    the world's death histogram by cause
//!   stats.csv     eats, forage trips and returns, pickups, store deliveries
//!   bites.csv     where adults bit, by (10k period, colony, material, 16-column bin), with the face joules
//!   digs.csv      the decision trace's dig attempts: (period, colony, `DigWhy`, target material), only the
//!                 decisions where the dig roll was won
//!   plants_F.csv  every plant at the snapshot frames, with its books (defence, water, nutrient status)
//!   leaf_F.csv    every leaf cell at the snapshot frames, with its height above the ground
//!   seeds_F.csv   every dormant or carried seed
//!   grazed_F.csv  bites attributed to the nearest plant, by plant id
//!   ants.csv      adult positions, energy and whether laden, every `antsnap=` ticks
//!   events.txt    the formatted run log at the end
//!   chron/        the lab's own chronicle files (the same format as the playtest's)

use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::ui::format_log_line;
use pixel_physics::lab::Lab;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::organism::{DevelopmentalKey, DEATH_CAUSE_LIST};
use pixel_physics::sim::world::{Account, LogKind, World};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}="))?.parse().ok())
}

const PLANTS: [(&str, i32, i32); 13] = [
    ("conifer", 60, 206),
    ("herb", 392, 216),
    ("herb", 440, 220),
    ("herb", 464, 216),
    ("herb", 506, 208),
    ("herb", 540, 220),
    ("herb", 542, 220),
    ("herb", 462, 238),
    ("herb", 424, 228),
    ("herb", 556, 186),
    ("herb", 602, 204),
    ("herb", 582, 218),
    ("tree", 944, 202),
];

struct Mats {
    leafy: HashSet<u16>,
    roots: HashSet<u16>,
    flower: HashSet<u16>,
    fruit: HashSet<u16>,
    litter: HashSet<u16>,
    seed: HashSet<u16>,
}

fn ids(w: &World, names: &[&str]) -> HashSet<u16> {
    names.iter().filter_map(|n| w.materials.id_of(n)).map(|m| m.0).collect()
}

fn nest_xs(w: &World) -> Vec<i32> {
    w.nest_sites.iter().map(|s| s.x).collect()
}

fn dist_nest(xs: &[i32], x: i32) -> i32 {
    xs.iter().map(|n| (n - x).abs()).min().unwrap_or(9999)
}

fn surface(w: &World, x: i32, fallback: i32) -> i32 {
    w.room_surface_at(x).unwrap_or(fallback)
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(440_000);
    let seed: u64 = arg("seed").unwrap_or(1);
    let ants: u32 = arg("ants").unwrap_or(1);
    let out: String = arg("out").expect("out=DIR");
    let probe_every: u64 = arg("probe").unwrap_or(5_000);
    let snap_every: u64 = arg("snap").unwrap_or(25_000);
    let antsnap: u64 = arg("antsnap").unwrap_or(2_000);
    // Planted-balance trace (2026-10-10): `herbtrace=N` writes every plant's
    // books every N frames to herbs.csv and every leaf bite to leafbites.csv.
    let herbtrace: u64 = arg("herbtrace").unwrap_or(0);
    let t1: u64 = arg("t1").unwrap_or(83_271);
    let t2: u64 = arg("t2").unwrap_or(84_889);
    let png_at: Vec<u64> = arg::<String>("png")
        .map(|v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect())
        .unwrap_or_default();
    println!("replay: frames={frames} seed={seed} ants={ants} out={out} probe={probe_every} snap={snap_every} antsnap={antsnap} t1={t1} t2={t2} herbtrace={herbtrace}");
    std::fs::create_dir_all(&out).unwrap();
    std::env::set_var(Lab::CHRONICLE_DIR_ENV, format!("{out}/chron"));
    std::env::set_var(Lab::CHRONICLE_CENSUS_EVERY_ENV, "10000");

    let mut spec = LabBox {
        width: 1024,
        height: 512,
        soil_depth: 176,
        ground_y: 256,
        founders: 0,
        colonies: 0,
        seed,
        ..LabBox::default()
    };
    match arg::<String>("rain").as_deref() {
        None => {}
        Some("off") => spec.rain = pixel_physics::lab::rain::Rain::Off,
        Some("light") => spec.rain = pixel_physics::lab::rain::Rain::Light,
        Some("steady") => spec.rain = pixel_physics::lab::rain::Rain::Steady,
        Some("heavy") => spec.rain = pixel_physics::lab::rain::Rain::Heavy,
        Some(o) => panic!("rain={o}"),
    }
    match arg::<String>("pace").as_deref() {
        None => {}
        Some("full") => spec.plant_pace = pixel_physics::lab::pace::PlantPace::Full,
        Some("half") => spec.plant_pace = pixel_physics::lab::pace::PlantPace::Half,
        Some("quarter") => spec.plant_pace = pixel_physics::lab::pace::PlantPace::Quarter,
        Some(o) => panic!("pace={o}"),
    }
    println!(
        "  bed: rain={} pace={:?} windfall_rots={}",
        spec.rain.label(),
        spec.plant_pace,
        spec.windfall_rots
    );
    let ground_y = spec.ground_y;
    let mut lab = Lab::new(spec);
    if let Some(dir) = arg::<String>("assets") {
        let n = lab.world.materials.reload(&dir).expect("assets reload");
        println!("  materials reloaded from {dir}: {n} files applied over the compiled-in set");
    }
    // The two dials the playtest's header names as changed from shipped.
    lab.world.plant_load_failure = false;
    lab.world.developmental_key = DevelopmentalKey::Plant { coarseness: 0 };
    lab.world.refold_developmental_seeds();
    for (name, x, y) in PLANTS {
        let ok = lab.world.plant_tree_species(x, y, name);
        println!("  plant {name} at {x},{y}: {ok}");
    }
    if arg::<u32>("doortree").unwrap_or(0) == 1 {
        // Intervention: a tree and a conifer planted ~20 columns from where the two colonies will found.
        for (name, x, y) in [("tree", 748, 230), ("conifer", 304, 230)] {
            let ok = lab.world.plant_tree_species(x, y, name);
            println!("  EXTRA plant {name} at {x},{y}: {ok}");
        }
    }
    let mats = Mats {
        leafy: ids(&lab.world, &["leaf", "grassblade", "moss"]),
        roots: ids(&lab.world, &["grassroot", "rootwood", "reedroot"]),
        flower: ids(&lab.world, &["flower"]),
        fruit: ids(&lab.world, &["fruit", "windfall", "provisions"]),
        litter: ids(&lab.world, &["litter", "deadleaf"]),
        seed: ids(&lab.world, &["seed", "pip"]),
    };

    let mut probe = BufWriter::new(File::create(format!("{out}/probe.csv")).unwrap());
    writeln!(
        probe,
        "frame,adults,brood,plants,bank,plant_cells,leaf_cells,root_cells,flower_cells,fruit_cells,other_cells,\
leaf_h0_3,leaf_h4_15,leaf_h16_60,leaf_h61p,leaf_d0_64,leaf_d64_150,leaf_d150_300,leaf_d300p,\
litter_free,seed_free,fruit_free,flower_free,corpse_free,plants_big_cells,plants_big_leaf,\
plants_small,plants_mid,plants_big,\
box284_roots,box284_plant_above,box284_empty_below,box728_roots,box728_plant_above,box728_empty_below"
    )
    .unwrap();
    let mut ledger = BufWriter::new(File::create(format!("{out}/ledger.csv")).unwrap());
    writeln!(ledger, "frame,colony,key,value").unwrap();
    let mut deaths = BufWriter::new(File::create(format!("{out}/deaths.csv")).unwrap());
    write!(deaths, "frame").unwrap();
    for c in DEATH_CAUSE_LIST {
        write!(deaths, ",{}", c.label().replace(' ', "_")).unwrap();
    }
    writeln!(deaths).unwrap();
    let mut stats = BufWriter::new(File::create(format!("{out}/stats.csv")).unwrap());
    writeln!(stats, "frame,eats,forage_trips,forage_depth_sum,forage_depth_max,forage_returns,pickups,store_pickups,store_delivered").unwrap();
    let mut grazed: HashMap<u32, (u64, String, i32)> = HashMap::new();
    let mut antlog = BufWriter::new(File::create(format!("{out}/ants.csv")).unwrap());
    writeln!(antlog, "frame,id,colony,x,y,energy,laden,nest_x,dx").unwrap();

    // Bite log: (period of 10k, colony, material, x bin of 16) -> (bites, face J)
    let mut bites: BTreeMap<(u64, u32, String, i32), (u64, f64)> = BTreeMap::new();
    let mut last_bites: HashMap<u32, u32> = HashMap::new();
    // Dig decisions: (period of 10k, colony, why, target material) -> count
    let mut digs: BTreeMap<(u64, u32, u8, String), u64> = BTreeMap::new();

    let mut herbs = (herbtrace > 0).then(|| {
        let mut f = BufWriter::new(File::create(format!("{out}/herbs.csv")).unwrap());
        writeln!(f, "frame,id,species,cells,leaf_mat,leaf_type,buds,tips,body,root,flower,fruit,carbon,income,maintenance,unpaid,starving_ticks,age_ticks,cx,top_h,low_leaf,dist_nest,grazed_leaf").unwrap();
        f
    });
    let mut leafbites = (herbtrace > 0).then(|| {
        let mut f = BufWriter::new(File::create(format!("{out}/leafbites.csv")).unwrap());
        writeln!(f, "frame,ant,colony,energy,material,plant,plant_species,plant_leaf,plant_cells,hx,hy,height,dist_nest,litter8,seed8,fruit8,leaf8,trip_src").unwrap();
        f
    });
    while lab.world.frame < frames {
        lab.tick_for_harness();
        let f = lab.world.frame;
        if ants == 1 {
            if f == t1 {
                let n = lab.world.found_colony_of(728, 230, "ant", 52);
                println!("  colony 1 at x=728 placed {n} at frame {f}");
            }
            if f == t2 {
                let n = lab.world.found_colony_of(284, 230, "ant", 52);
                println!("  colony 2 at x=284 placed {n} at frame {f}");
                lab.world.decision_log = Some(Vec::new());
            }
            if f > t2 && f.is_multiple_of(200) {
                drain_digs(&mut lab.world, &mut digs, f);
            }
            if f > t1 {
                bite_scan(&lab.world, &mut last_bites, &mut bites, &mut grazed, &mats, f, ground_y, leafbites.as_mut());
            }
        }
        if let Some(h) = herbs.as_mut() {
            if f.is_multiple_of(herbtrace) {
                herb_rows(&lab.world, &mats, ground_y, f, h);
            }
        }
        if f.is_multiple_of(probe_every) {
            if let Some(h) = herbs.as_mut() {
                h.flush().unwrap();
            }
            if let Some(b) = leafbites.as_mut() {
                b.flush().unwrap();
            }
            probe_row(&lab.world, &mats, ground_y, &mut probe);
            ledger_rows(&lab.world, f, &mut ledger);
            {
                let c = &lab.world.creature_stats;
                writeln!(
                    stats,
                    "{f},{},{},{},{},{},{},{},{}",
                    c.eats,
                    c.forage_trips,
                    c.forage_depth_sum,
                    c.forage_depth_max,
                    c.forage_returns,
                    c.pickups,
                    c.store_pickups,
                    c.store_delivered
                )
                .unwrap();
                stats.flush().unwrap();
            }
            write!(deaths, "{f}").unwrap();
            for v in lab.world.deaths_by_cause.iter() {
                write!(deaths, ",{v}").unwrap();
            }
            writeln!(deaths).unwrap();
            probe.flush().unwrap();
            ledger.flush().unwrap();
            deaths.flush().unwrap();
        }
        if ants == 1 && f.is_multiple_of(antsnap) && f >= t1 {
            ant_rows(&lab.world, f, &mut antlog);
        }
        if f.is_multiple_of(snap_every) || f == frames {
            plant_table(&lab.world, &mats, ground_y, &format!("{out}/plants_{f}.csv"));
            leaf_map(&lab.world, &mats, ground_y, &format!("{out}/leaf_{f}.csv"));
            seed_map(&lab.world, &format!("{out}/seeds_{f}.csv"));
            grazed_dump(&lab.world, &grazed, &format!("{out}/grazed_{f}.csv"));
        }
        if png_at.contains(&f) {
            snapshot_png(&lab.world, &mats, 150, 345, &format!("{out}/world_{f}.png"));
        }
        if f.is_multiple_of(20_000) {
            println!("  frame {f}  adults {}  plants-ish ok", count_adults(&lab.world));
            let _ = std::io::stdout().flush();
        }
    }

    let final_frame = lab.world.frame;
    drain_digs(&mut lab.world, &mut digs, final_frame);
    let mut dw = BufWriter::new(File::create(format!("{out}/digs.csv")).unwrap());
    writeln!(dw, "period,colony,why,material,count").unwrap();
    for ((p, c, why, m), n) in &digs {
        writeln!(dw, "{p},{c},{why},{m},{n}").unwrap();
    }
    dw.flush().unwrap();
    // bites.csv
    let mut bw = BufWriter::new(File::create(format!("{out}/bites.csv")).unwrap());
    writeln!(bw, "period,colony,material,xbin,bites,face_j").unwrap();
    for ((p, c, m, xb), (n, j)) in &bites {
        writeln!(bw, "{p},{c},{m},{xb},{n},{j}").unwrap();
    }
    bw.flush().unwrap();

    // events.txt
    let mut ev = BufWriter::new(File::create(format!("{out}/events.txt")).unwrap());
    let mut events: Vec<_> = lab.world.run_log.recent().cloned().collect();
    events.reverse();
    for e in &events {
        if matches!(
            e.kind,
            LogKind::Died | LogKind::Born | LogKind::PlayerAction | LogKind::LineEnded
        ) {
            let (what, _, _) = format_log_line(&lab.world, e);
            let sp = lab.world.species.get(e.species).name.clone();
            writeln!(ev, "F{:>8}\t{:?}\t{}\tid={}\t{}", e.frame, e.kind, sp, e.id, what).unwrap();
        }
    }
    ev.flush().unwrap();
    lab.write_chronicle();
    println!("replay done at frame {}", lab.world.frame);
}

fn count_adults(w: &World) -> usize {
    w.live_organism_ids()
        .into_iter()
        .filter_map(|id| w.organism(id))
        .filter(|s| w.species.get(s.species).creature.is_some() && s.brood.is_none())
        .count()
}

#[allow(clippy::too_many_arguments)]
fn bite_scan(
    w: &World,
    last: &mut HashMap<u32, u32>,
    log: &mut BTreeMap<(u64, u32, String, i32), (u64, f64)>,
    grazed: &mut HashMap<u32, (u64, String, i32)>,
    m: &Mats,
    f: u64,
    ground_y: i32,
    mut leafbites: Option<&mut BufWriter<File>>,
) {
    let nest = nest_xs(w);
    for id in w.live_organism_ids() {
        let Some(s) = w.organism(id) else { continue };
        if w.species.get(s.species).creature.is_none() || s.brood.is_some() {
            continue;
        }
        let b = s.life.bites;
        let prev = last.insert(id, b).unwrap_or(b);
        if b > prev {
            let Some(head) = s.chain.first() else { continue };
            let Some(crop) = s.crop.as_ref() else { continue };
            let name = w.materials.get(crop.material).name.clone();
            let n = u64::from(b - prev);
            let key = (f / 10_000, s.colony, name.clone(), head.0.div_euclid(16) * 16);
            let e = log.entry(key).or_insert((0, 0.0));
            e.0 += n;
            e.1 += f64::from(crop.unit) * f64::from(b - prev);
            // Which plant was grazed: the nearest plant cell to the mouth (the
            // bitten cell itself is gone, its neighbours remain).
            if m.leafy.contains(&crop.material.0) {
                let mut best: Option<(i32, u32)> = None;
                for dy in -2..=2 {
                    for dx in -2..=2 {
                        let c = w.get(head.0 + dx, head.1 + dy);
                        let oid = c.organism_id();
                        if oid == 0 || oid == id {
                            continue;
                        }
                        let Some(os) = w.organism(oid) else { continue };
                        if w.species.get(os.species).creature.is_some() {
                            continue;
                        }
                        let d = dx.abs().max(dy.abs());
                        if best.is_none_or(|(bd, _)| d < bd) {
                            best = Some((d, oid));
                        }
                    }
                }
                if let Some(lb) = leafbites.as_deref_mut() {
                    // What else was within 8 cells of the mouth: loose food the ant
                    // could have eaten instead (counted by material, plant leaf apart).
                    let (mut lit, mut sd, mut fr, mut lf) = (0, 0, 0, 0);
                    for dy in -8..=8 {
                        for dx in -8..=8 {
                            let c = w.get(head.0 + dx, head.1 + dy);
                            let mid = c.material.0;
                            if m.litter.contains(&mid) {
                                lit += 1;
                            } else if m.seed.contains(&mid) {
                                sd += 1;
                            } else if m.fruit.contains(&mid) {
                                fr += 1;
                            } else if m.leafy.contains(&mid) {
                                lf += 1;
                            }
                        }
                    }
                    let (pid, psp, pleaf, pcells) = match best.and_then(|(_, oid)| w.organism(oid as _).map(|os| (oid, os))) {
                        Some((oid, os)) => {
                            let pl = os.cells.keys().filter(|&&(x, y)| m.leafy.contains(&w.get(x, y).material.0)).count();
                            (oid, w.species.get(os.species).name.clone(), pl, os.cells.len())
                        }
                        None => (0, String::new(), 0, 0),
                    };
                    writeln!(
                        lb,
                        "{f},{id},{},{:.1},{name},{pid},{psp},{pleaf},{pcells},{},{},{},{},{lit},{sd},{fr},{lf},{}",
                        s.colony,
                        s.energy,
                        head.0,
                        head.1,
                        surface(w, head.0, ground_y) - head.1,
                        dist_nest(&nest, head.0),
                        s.trip_src
                    )
                    .unwrap();
                }
                if let Some((_, oid)) = best {
                    let sp = w
                        .organism(oid as _)
                        .map(|os| w.species.get(os.species).name.clone())
                        .unwrap_or_default();
                    let e = grazed.entry(oid).or_insert((0, sp, head.0));
                    e.0 += n;
                }
            }
        }
    }
}

/// One row per living plant (not seeds, not single cells): its books and its
/// cell census by material and by cell type, for following one herb's life.
fn herb_rows(w: &World, m: &Mats, ground_y: i32, f: u64, out: &mut BufWriter<File>) {
    use pixel_physics::sim::organism::{cell_type, CellType};
    let nest = nest_xs(w);
    for id in w.live_organism_ids() {
        let Some(s) = w.organism(id) else { continue };
        if w.species.get(s.species).creature.is_some() || w.is_carried_seed(id) || s.cells.len() == 1 {
            continue;
        }
        let (mut leaf_m, mut leaf_t, mut buds, mut tips, mut body, mut root, mut flower, mut fruit) = (0, 0, 0, 0, 0, 0, 0, 0);
        let (mut carbon, mut sumx, mut n, mut top, mut low_leaf) = (0.0f32, 0i64, 0i64, i32::MAX, 0);
        for &(x, y) in s.cells.keys() {
            let c = w.get(x, y);
            let mid = c.material.0;
            carbon += w.carbon_at(x, y);
            if m.leafy.contains(&mid) {
                leaf_m += 1;
                if surface(w, x, ground_y) - y <= 15 {
                    low_leaf += 1;
                }
            }
            if m.roots.contains(&mid) {
                root += 1;
            } else {
                sumx += i64::from(x);
                n += 1;
                top = top.min(y);
            }
            if m.flower.contains(&mid) {
                flower += 1;
            } else if m.fruit.contains(&mid) {
                fruit += 1;
            }
            match cell_type(c.aux()) {
                Some(CellType::Leaf) => leaf_t += 1,
                Some(CellType::DormantBud) => buds += 1,
                Some(CellType::GrowingTip) => tips += 1,
                Some(CellType::MatureBody) => body += 1,
                _ => {}
            }
        }
        let cx = if n > 0 { (sumx / n) as i32 } else { 0 };
        let top_h = if n > 0 { surface(w, cx, ground_y) - top } else { 0 };
        writeln!(
            out,
            "{f},{id},{},{},{leaf_m},{leaf_t},{buds},{tips},{body},{root},{flower},{fruit},{carbon:.2},{:.3},{:.3},{:.3},{},{},{cx},{top_h},{low_leaf},{},{}",
            w.species.get(s.species).name,
            s.cells.len(),
            s.income,
            s.maintenance,
            s.maintenance_unpaid,
            s.starving_ticks,
            s.age_ticks,
            dist_nest(&nest, cx),
            s.grazed_leaf
        )
        .unwrap();
    }
}

fn grazed_dump(w: &World, grazed: &HashMap<u32, (u64, String, i32)>, path: &str) {
    let mut out = BufWriter::new(File::create(path).unwrap());
    writeln!(out, "id,species,bites,x_at_bite,alive,cells_now").unwrap();
    let mut rows: Vec<_> = grazed.iter().collect();
    rows.sort_by_key(|(id, _)| **id);
    for (id, (n, sp, x)) in rows {
        let (alive, cells) = match w.organism(*id as _) {
            Some(s) => (1, s.cells.len()),
            None => (0, 0),
        };
        writeln!(out, "{id},{sp},{n},{x},{alive},{cells}").unwrap();
    }
    out.flush().unwrap();
}

fn seed_map(w: &World, path: &str) {
    let mut out = BufWriter::new(File::create(path).unwrap());
    writeln!(out, "x,y,species,carried").unwrap();
    for id in w.live_organism_ids() {
        let Some(s) = w.organism(id) else { continue };
        if w.species.get(s.species).creature.is_some() {
            continue;
        }
        let carried = w.is_carried_seed(id);
        if carried || s.cells.len() == 1 {
            let (x, y) = s.cells.keys().next().copied().unwrap_or((-1, -1));
            writeln!(out, "{x},{y},{},{}", w.species.get(s.species).name, carried as u8).unwrap();
        }
    }
    out.flush().unwrap();
}

fn nest_box(w: &World, m: &Mats, refx: i32, ground_y: i32) -> (usize, usize, usize) {
    let (mut roots, mut above, mut empty) = (0, 0, 0);
    for x in (refx - 40)..=(refx + 40) {
        let sfc = surface(w, x, ground_y);
        for y in (sfc - 60)..(sfc + 80) {
            let c = w.get(x, y);
            if y >= sfc {
                if m.roots.contains(&c.material.0) {
                    roots += 1;
                } else if c.material.0 == 0 {
                    empty += 1;
                }
            } else if w.materials.kind(c.material) == MaterialKind::Plant {
                above += 1;
            }
        }
    }
    (roots, above, empty)
}

fn probe_row(w: &World, m: &Mats, ground_y: i32, probe: &mut BufWriter<File>) {
    let nest = nest_xs(w);
    let (mut adults, mut brood, mut plants, mut bank) = (0usize, 0usize, 0usize, 0usize);
    let (mut cells, mut leaf, mut root, mut flower, mut fruit, mut other) =
        (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let mut hbin = [0usize; 4];
    let mut dbin = [0usize; 4];
    let (mut big_cells, mut big_leaf) = (0usize, 0usize);
    let (mut small, mut mid, mut big) = (0usize, 0usize, 0usize);
    for id in w.live_organism_ids() {
        let Some(s) = w.organism(id) else { continue };
        if w.species.get(s.species).creature.is_some() {
            if s.brood.is_some() {
                brood += 1;
            } else {
                adults += 1;
            }
            continue;
        }
        if w.is_carried_seed(id) || s.cells.len() == 1 {
            bank += 1;
            continue;
        }
        plants += 1;
        let (mut pc, mut pl) = (0usize, 0usize);
        for &(x, y) in s.cells.keys() {
            let mid = w.get(x, y).material.0;
            pc += 1;
            if m.leafy.contains(&mid) {
                pl += 1;
                let h = surface(w, x, ground_y) - y;
                let d = dist_nest(&nest, x);
                hbin[if h <= 3 {
                    0
                } else if h <= 15 {
                    1
                } else if h <= 60 {
                    2
                } else {
                    3
                }] += 1;
                dbin[if d < 64 {
                    0
                } else if d < 150 {
                    1
                } else if d < 300 {
                    2
                } else {
                    3
                }] += 1;
                leaf += 1;
            } else if m.roots.contains(&mid) {
                root += 1;
            } else if m.flower.contains(&mid) {
                flower += 1;
            } else if m.fruit.contains(&mid) {
                fruit += 1;
            } else {
                other += 1;
            }
        }
        cells += pc;
        if pc < 50 {
            small += 1;
        } else if pc < 500 {
            mid += 1;
        } else {
            big += 1;
        }
        if pc >= 1000 {
            big_cells += pc;
            big_leaf += pl;
        }
    }
    // free (ground) edible cells, raw
    let (mut litter, mut seed, mut fruit_free, mut flower_free, mut corpse) = (0, 0, 0, 0, 0);
    let corpse_id = w.materials.id_of("corpse").map(|x| x.0);
    if let Some(b) = w.bounds() {
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let c = w.get(x, y);
                if c.organism_id() != 0 {
                    continue;
                }
                let mid = c.material.0;
                if mid == 0 {
                    continue;
                }
                if m.litter.contains(&mid) {
                    litter += 1;
                } else if m.seed.contains(&mid) {
                    seed += 1;
                } else if m.fruit.contains(&mid) {
                    fruit_free += 1;
                } else if m.flower.contains(&mid) {
                    flower_free += 1;
                } else if Some(mid) == corpse_id {
                    corpse += 1;
                }
            }
        }
    }
    let b284 = nest_box(w, m, 284, ground_y);
    let b728 = nest_box(w, m, 728, ground_y);
    writeln!(
        probe,
        "{},{adults},{brood},{plants},{bank},{cells},{leaf},{root},{flower},{fruit},{other},{},{},{},{},{},{},{},{},{litter},{seed},{fruit_free},{flower_free},{corpse},{big_cells},{big_leaf},{small},{mid},{big},{},{},{},{},{},{}",
        w.frame, hbin[0], hbin[1], hbin[2], hbin[3], dbin[0], dbin[1], dbin[2], dbin[3],
        b284.0, b284.1, b284.2, b728.0, b728.1, b728.2
    )
    .unwrap();
}

fn ledger_rows(w: &World, f: u64, out: &mut BufWriter<File>) {
    for (c, books) in w.all_colony_books().iter().enumerate() {
        if books.income() == 0.0 && books.outgo() == 0.0 {
            continue;
        }
        for a in Account::ALL {
            writeln!(out, "{f},{c},{:?},{}", a, books.get(a)).unwrap();
        }
        for (mid, j) in books.diet() {
            writeln!(out, "{f},{c},diet:{},{}", w.materials.get(mid).name, j).unwrap();
        }
    }
}

fn ant_rows(w: &World, f: u64, out: &mut BufWriter<File>) {
    let nest = w.nest_sites.iter().map(|s| (s.x, s.y)).collect::<Vec<_>>();
    for id in w.live_organism_ids() {
        let Some(s) = w.organism(id) else { continue };
        if w.species.get(s.species).creature.is_none() || s.brood.is_some() {
            continue;
        }
        let Some(h) = s.chain.first() else { continue };
        let nx = nest.iter().map(|n| n.0).min_by_key(|n| (n - h.0).abs()).unwrap_or(0);
        writeln!(
            out,
            "{f},{id},{},{},{},{:.1},{},{nx},{}",
            s.colony,
            h.0,
            h.1,
            s.energy,
            s.crop.is_some() as u8,
            h.0 - nx
        )
        .unwrap();
    }
}

fn plant_table(w: &World, m: &Mats, ground_y: i32, path: &str) {
    let mut out = BufWriter::new(File::create(path).unwrap());
    writeln!(
        out,
        "id,species,cells,leaf,root,flower,fruit,other,min_x,max_x,cx,top_h,dist_nest,income,maintenance,unpaid,starved_cells,age_ticks,starving_ticks,defence,water_status,nutrient_status"
    )
    .unwrap();
    let nest = nest_xs(w);
    for id in w.live_organism_ids() {
        let Some(s) = w.organism(id) else { continue };
        if w.species.get(s.species).creature.is_some() || w.is_carried_seed(id) || s.cells.len() == 1 {
            continue;
        }
        let (mut leaf, mut root, mut flower, mut fruit, mut other) = (0, 0, 0, 0, 0);
        let (mut minx, mut maxx, mut sumx, mut n, mut top) = (i32::MAX, i32::MIN, 0i64, 0i64, i32::MAX);
        for &(x, y) in s.cells.keys() {
            let mid = w.get(x, y).material.0;
            if m.leafy.contains(&mid) {
                leaf += 1;
            } else if m.roots.contains(&mid) {
                root += 1;
            } else if m.flower.contains(&mid) {
                flower += 1;
            } else if m.fruit.contains(&mid) {
                fruit += 1;
            } else {
                other += 1;
            }
            if !m.roots.contains(&mid) {
                minx = minx.min(x);
                maxx = maxx.max(x);
                sumx += i64::from(x);
                n += 1;
                top = top.min(y);
            }
        }
        let cx = if n > 0 { (sumx / n) as i32 } else { 0 };
        let top_h = if n > 0 { surface(w, cx, ground_y) - top } else { 0 };
        writeln!(
            out,
            "{id},{},{},{leaf},{root},{flower},{fruit},{other},{minx},{maxx},{cx},{top_h},{},{:.2},{:.2},{:.2},{},{},{},{:.3},{:.3},{:.3}",
            w.species.get(s.species).name,
            s.cells.len(),
            dist_nest(&nest, cx),
            s.income,
            s.maintenance,
            s.maintenance_unpaid,
            s.starved_cells,
            s.age_ticks,
            s.starving_ticks,
            s.defence,
            s.water_status,
            s.nutrient_status
        )
        .unwrap();
    }
    out.flush().unwrap();
}

fn leaf_map(w: &World, m: &Mats, ground_y: i32, path: &str) {
    let mut out = BufWriter::new(File::create(path).unwrap());
    writeln!(out, "x,y,h,org,species").unwrap();
    for id in w.live_organism_ids() {
        let Some(s) = w.organism(id) else { continue };
        if w.species.get(s.species).creature.is_some() || w.is_carried_seed(id) || s.cells.len() == 1 {
            continue;
        }
        let sp = w.species.get(s.species).name.clone();
        for &(x, y) in s.cells.keys() {
            let mid = w.get(x, y).material.0;
            if m.leafy.contains(&mid) {
                writeln!(out, "{x},{y},{},{id},{sp}", surface(w, x, ground_y) - y).unwrap();
            }
        }
    }
    out.flush().unwrap();
}

/// A plain raster of the box (full width, rows `y0..y1`), two pixels a cell:
/// leaf bright green, roots orange, flowers magenta, fruit red, ants white,
/// brood cream, everything else its material's first palette colour.
fn snapshot_png(w: &World, m: &Mats, y0: i32, y1: i32, path: &str) {
    let Some(b) = w.bounds() else { return };
    let width = (b.max_x - b.min_x + 1) as u32;
    let height = (y1 - y0) as u32;
    let zoom = 2u32;
    let (pw, ph) = (width * zoom, height * zoom);
    let mut buf = vec![0u8; (pw * ph * 4) as usize];
    for y in y0..y1 {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            let mid = c.material.0;
            let kind = w.materials.kind(c.material);
            let rgb: [u8; 3] = if mid == 0 {
                [12, 16, 30]
            } else if m.leafy.contains(&mid) {
                [70, 210, 80]
            } else if m.roots.contains(&mid) {
                [235, 135, 40]
            } else if m.flower.contains(&mid) {
                [230, 60, 200]
            } else if m.fruit.contains(&mid) {
                [230, 50, 50]
            } else if kind == MaterialKind::Creature {
                let brood = w.materials.get(c.material).name == "brood";
                if brood {
                    [255, 235, 160]
                } else {
                    [255, 255, 255]
                }
            } else {
                {
                    let p = w
                        .materials
                        .get(c.material)
                        .palette
                        .first()
                        .copied()
                        .unwrap_or([128, 128, 128, 255]);
                    [p[0], p[1], p[2]]
                }
            };
            for dy in 0..zoom {
                for dx in 0..zoom {
                    let px = ((x - b.min_x) as u32) * zoom + dx;
                    let py = ((y - y0) as u32) * zoom + dy;
                    let i = ((py * pw + px) * 4) as usize;
                    buf[i] = rgb[0];
                    buf[i + 1] = rgb[1];
                    buf[i + 2] = rgb[2];
                    buf[i + 3] = 255;
                }
            }
        }
    }
    image::save_buffer(path, &buf, pw, ph, image::ColorType::Rgba8).expect("write png");
}

/// Drain the decision trace into per-(period, colony, DigWhy, target material)
/// counts, keeping only the decisions where the dig roll was won (`Cue`, `Roof`,
/// `NoGround`, `Cut`, `Face`) -- the ones where a target cell was looked at.
fn drain_digs(w: &mut World, digs: &mut BTreeMap<(u64, u32, u8, String), u64>, f: u64) {
    let rows = w.decision_log.as_mut().map(std::mem::take).unwrap_or_default();
    for r in rows {
        let why = r.dig as u8;
        if why < 3 {
            continue;
        }
        let colony = w.organism(r.id).map_or(0, |s| s.colony);
        let mat = if r.dig_mat == 0 {
            "empty".to_string()
        } else {
            w.materials
                .get(pixel_physics::sim::material::MaterialId(r.dig_mat))
                .name
                .clone()
        };
        *digs.entry((r.frame / 10_000, colony, why, mat)).or_insert(0) += 1;
    }
    let _ = f;
}
