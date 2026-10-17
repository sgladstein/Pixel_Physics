//! **Layer 1 of the ant rebuild: does a fixed colony of textbook ants dig a
//! nest and run a food trail in the lab box?** (owner, 2026-10-16: "rebuild
//! ant", in layers, in today's world; design of record
//! `/mnt/project-files/emergence-sandbox/textbook-ant/layer1-dig-and-forage-2026-10-16.md`).
//!
//! The scene is the lab's goal box (`nest_goal`: the bed dry, no plants, one
//! heap of player food `foodgap=` columns east of the nest, kept topped up to
//! `food=` cells). The colony lands as the scenario lands it, with `ants=`
//! ants. From the landing on, the colony is **fixed**: births paused, nobody
//! ages out, and every 500 frames each ant is topped up to `fed=` of its
//! start energy (1.0 for the textbook ant, which does not eat; 0.5 for the
//! game ant, which needs hunger to forage).
//!
//! - `ant=textbook` (default): the colony decides by
//!   `creature::textbook::NestCfg` -- dig where crowded, carry spoil out and
//!   heap it by Khuong's rule, forage on a food trail with its own memory.
//! - `ant=game`: the same colony, same feeding, deciding as the game ant.
//!
//! Every `every=` frames (10,000) it prints a census line and writes two
//! pictures to `out=`: the lab's own render round the nest
//! (`shot_fNNNNNN.png`) and a cell map of the nest region with the food
//! trail tinted (`map_fNNNNNN.png`). The census, all within `NEST_HALF`
//! columns of the nest:
//! - `dug`: cells of the bed open now that were ground at the landing;
//! - `roofed`: open cells below the old ground line with ground above them
//!   in the column (what a player calls a nest);
//! - `mound`: ground above the old ground line (spoil heaped outside);
//! - `under`: ants with their head below the ground line under a roof;
//! - `taken`: food cells refilled into the heap so far (eaten or carried);
//! - `home`: food cells lying within `HOME_FOOD` columns of the nest door,
//!   and `eaten`, those taken away so far (textbook arm only);
//! - `trail`: cells holding food trail above a tenth of a deposit;
//! - the textbook ant's own counts (digs, drops, food picked, delivered).
//!
//! ```text
//! cargo run --release --example antnest -- seed=1 frames=100000
//! cargo run --release --example antnest -- ant=game seed=1 frames=100000
//! cargo run --release --example antnest -- digcrowd=0        # the control: digging ignores company
//! ```
use pixel_physics::lab::scenario::{Placement, Scenario};
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::creature::{self, textbook};
use pixel_physics::sim::material;
use pixel_physics::sim::pheromone::{self, Channel};
use pixel_physics::sim::world::World;

/// Every key the harness reads; anything else is refused, so a typo cannot
/// run silently at the default.
const KEYS: &[&str] = &[
    "ant",
    "ants",
    "seed",
    "frames",
    "every",
    "out",
    "food",
    "foodgap",
    "fed",
    "forager",
    "dig",
    "digcrowd",
    "roof",
    "up",
    "down",
    "home",
    "vector",
    "vectorout",
    "k",
    "n",
    "lay",
    "giveup",
    "search",
    "sense",
    "tau",
    "turn",
    "dt",
    "homer",
    "clear",
    "store",
    "jam",
    "loose",
    "straight",
    "insideonly",
];

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}="))
            .map(|v| v.parse().ok().expect("parses"))
    })
}

/// Columns either side of the nest that the census reads.
const NEST_HALF: i32 = 60;
/// Columns either side of the door that count as food brought home.
const HOME_FOOD: i32 = 14;
/// How often the colony is topped up, in frames.
const FEED_EVERY: u64 = 500;

fn main() {
    for a in std::env::args().skip(1) {
        let key = a.split_once('=').map_or(a.as_str(), |(k, _)| k);
        assert!(KEYS.contains(&key), "antnest: unknown key {key:?}; keys are {KEYS:?}");
    }
    creature::mutation_off_for_measuring();
    let kind: String = arg("ant").unwrap_or_else(|| "textbook".into());
    assert!(kind == "textbook" || kind == "game", "ant={kind}: textbook or game");
    let textbook_ant = kind == "textbook";
    let n_ants: i32 = arg("ants").unwrap_or(150);
    let seed: u64 = arg("seed").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(100_000);
    let every: u64 = arg("every").unwrap_or(10_000);
    let target: usize = arg("food").unwrap_or(120);
    let food_gap: i32 = arg("foodgap").unwrap_or(30);
    let fed: f32 = arg("fed").unwrap_or(if textbook_ant { 1.0 } else { 0.5 });
    let out: String = arg("out").unwrap_or_else(|| format!("antnest-out/{kind}-s{seed}"));
    std::fs::create_dir_all(&out).expect("out dir");

    let mut sc = Scenario::load("nest_goal").expect("nest_goal ships");
    sc.bed.seed = seed;
    let mut colony_x = 0;
    for e in &mut sc.timeline {
        match &mut e.what {
            Placement::Colony { x, count, .. } => {
                *count = n_ants as _;
                colony_x = *x;
            }
            Placement::Heap { material, x, .. } if material == "provisions" => *x = colony_x + food_gap,
            _ => {}
        }
    }
    let land = sc
        .timeline
        .iter()
        .find_map(|e| matches!(e.what, Placement::Colony { .. }).then_some(e.at))
        .expect("nest_goal lands a colony");
    let mut lab = Lab::new(sc.bed.clone());
    let msg = lab.load_scenario(sc);
    lab.show_help = false;
    if lab.stats.showing() {
        lab.stats.toggle();
    }
    for _ in 1..3 {
        lab.renderer.adjust_zoom(1);
    }
    let ground_y = lab.spec.ground_y;
    let width = lab.spec.width;
    println!("antnest: ant={kind} ants={n_ants} seed={seed} frames={frames} food={target} foodgap={food_gap} fed={fed} out={out}");
    println!("  {msg}");

    let soil = lab.world.materials.id_of("soil").expect("soil");
    let provisions = lab.world.materials.id_of("provisions").expect("provisions");
    let mut nest_x = None;
    let mut ground_at_land: Vec<bool> = Vec::new();
    let mut taken = 0usize;
    let mut eaten = 0usize;
    for f in 0..=frames {
        if nest_x.is_none() && f > land {
            if let Some(s) = lab.world.nest_sites.first() {
                let nx = s.x;
                nest_x = Some(nx);
                let w = &mut lab.world;
                w.births_paused = true;
                let id = w.species.id_of("ant").expect("ant ships");
                if let Some(c) = w.species.get_mut(id).creature.as_mut() {
                    c.life_half_life = 10_000_000;
                }
                ground_at_land = (0..width * lab.spec.height)
                    .map(|i| ground(w, i % width, i / width))
                    .collect();
                if textbook_ant {
                    let cfg = cfg(soil, provisions, (nx, ground_y - 1), (nx + food_gap, ground_y - 1));
                    println!("  textbook {cfg:?}");
                    let mut tb = textbook::Textbook::new(id, cfg);
                    tb.nest_log = Some(Vec::new());
                    w.textbook = Some(Box::new(tb));
                }
                println!(
                    "  landed by frame {f}: nest x {nx}, food x {}, {} ants, births paused",
                    nx + food_gap,
                    ants(w).len()
                );
            }
        }
        if let Some(nx) = nest_x {
            if f % FEED_EVERY == 0 {
                feed(&mut lab.world, fed);
                taken += top_up(&mut lab.world, provisions, nx + food_gap, ground_y, target);
                if textbook_ant {
                    eaten += eat_home(&mut lab.world, provisions, nx, ground_y);
                }
            }
            if f % every == 0 {
                let w = &lab.world;
                println!(
                    "{} eaten {eaten}",
                    census(w, nx, ground_y, &ground_at_land, width, provisions, taken, f)
                );
                shot(&mut lab, &out, f, (nx + food_gap / 2, ground_y + 12));
                map(&lab.world, &out, f, nx, ground_y, provisions);
                if let Some(log) = lab.world.textbook.as_mut().and_then(|t| t.nest_log.take()) {
                    use std::io::Write;
                    let path = std::path::Path::new(&out).join("nestlog.csv");
                    let new = !path.exists();
                    let mut fh = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                        .expect("nestlog");
                    if new {
                        writeln!(fh, "frame,ant,dx,dy,material,put").unwrap();
                    }
                    for &(fr, ant, x, y, m, put) in &log {
                        let name = &lab.world.materials.get(m).name;
                        writeln!(fh, "{fr},{ant},{},{},{name},{}", x - nx, y - ground_y, u8::from(put)).unwrap();
                    }
                    if let Some(t) = lab.world.textbook.as_mut() {
                        t.nest_log = Some(Vec::new());
                    }
                }
            }
        }
        lab.tick_for_harness();
    }
    // The final ground as one character a cell, for tracing where the
    // put-down pellets ended (`nestlog.csv` holds where they went).
    if let Some(nx) = nest_x {
        use std::io::Write;
        let w = &lab.world;
        let mut fh = std::fs::File::create(std::path::Path::new(&out).join("final.txt")).expect("final");
        writeln!(
            fh,
            "# rows {}..{} cols {}..{} relative to the ground line and nest",
            -60, 85, -NEST_HALF, NEST_HALF
        )
        .unwrap();
        for y in ground_y - 60..ground_y + 85 {
            let row: String = (nx - NEST_HALF..=nx + NEST_HALF)
                .map(|x| {
                    let c = w.get(x, y);
                    if c.organism_id() != 0 {
                        'a'
                    } else if c.material == material::EMPTY {
                        '.'
                    } else {
                        w.materials.get(c.material).name.chars().next().unwrap_or('?')
                    }
                })
                .collect();
            writeln!(fh, "{row}").unwrap();
        }
    }
}

/// Layer 1's parameters, each from the command line with its default.
fn cfg(
    soil: material::MaterialId,
    food: material::MaterialId,
    home: (i32, i32),
    feeder: (i32, i32),
) -> textbook::TextbookCfg {
    let forage = textbook::ForageCfg {
        food,
        home,
        home_r: arg("homer").unwrap_or(4),
        aim_home: home,
        feeder,
        vector: arg("vector").unwrap_or(1.0),
        vector_out: arg("vectorout").unwrap_or(1.0),
        out_flat: false,
        k: arg("k").unwrap_or(2.0),
        n: arg("n").unwrap_or(2),
        lay: arg("lay").unwrap_or(0.1),
        lay_out: false,
        out_frac: 1.0,
        read: true,
        give_up: arg("giveup").unwrap_or(300),
        search_r: arg("search").unwrap_or(12),
        sense: arg("sense").unwrap_or(6),
        crowd: 0.0,
        no_entry: 0.0,
        repel: 0.0,
        mark_fork: false,
        mark_to_fork: false,
        explore: 0.0,
        explore_c: 0.0,
        explore_k: 0.0,
        explore_max: 0.0,
        explore_lay: 0.0,
        kn: false,
    };
    let nest = textbook::NestCfg {
        forager_frac: arg("forager").unwrap_or(0.3),
        dig: arg("dig").unwrap_or(0.2),
        dig_crowd: arg("digcrowd").unwrap_or(3.0),
        roof: arg("roof").unwrap_or(6),
        up: arg("up").unwrap_or(1.0),
        down: arg("down").unwrap_or(0.5),
        home: arg("home").unwrap_or(1.0),
        clear_r: arg("clear").unwrap_or(4),
        jam: arg("jam").unwrap_or(3),
        loose: arg::<u8>("loose").unwrap_or(0) == 1,
        straight: arg::<u8>("straight").unwrap_or(0) == 1,
        inside_only: arg::<u8>("insideonly").unwrap_or(1) == 1,
        store_r: arg("store").unwrap_or(12),
    };
    textbook::TextbookCfg {
        p_move: 1.0,
        turn: arg("turn").unwrap_or(0.125),
        dt: arg("dt").unwrap_or(4.0),
        drop_base: 0.025,
        drop_per: 0.11,
        pick: 0.029,
        tau: arg("tau").unwrap_or(1_800.0),
        template: 2.0,
        source: soil,
        n_rock: false,
        pick_around: true,
        base_always: true,
        build_on_box: true,
        forage: Some(forage),
        sort: None,
        nest: Some(nest),
    }
}

/// Ground for the census: anything solid that is not an animal.
fn ground(w: &World, x: i32, y: i32) -> bool {
    w.in_bounds(x, y) && {
        let c = w.get(x, y);
        c.material != material::EMPTY
            && c.organism_id() == 0
            && w.materials.kind(c.material) != material::MaterialKind::Liquid
            && w.materials.kind(c.material) != material::MaterialKind::Gas
    }
}

fn ants(w: &World) -> Vec<pixel_physics::sim::cell::OrganismId> {
    w.live_organism_ids()
        .into_iter()
        .filter(|&id| {
            w.organism(id)
                .is_some_and(|s| w.species.get(s.species).creature.is_some() && !s.chain.is_empty())
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn census(
    w: &World,
    nx: i32,
    gy: i32,
    at_land: &[bool],
    width: i32,
    provisions: material::MaterialId,
    taken: usize,
    f: u64,
) -> String {
    let all = ants(w);
    let (x0, x1) = ((nx - NEST_HALF).max(0), (nx + NEST_HALF).min(width - 1));
    let (mut dug, mut roofed, mut mound, mut home, mut trail) = (0, 0, 0, 0, 0);
    for x in x0..=x1 {
        for y in (gy - 60).max(0)..gy + 85 {
            let g = ground(w, x, y);
            let i = (y * width + x) as usize;
            if !g && at_land.get(i).copied().unwrap_or(false) && w.get(x, y).organism_id() == 0 {
                dug += 1;
            }
            if y > gy && !g && w.get(x, y).material == material::EMPTY && (gy..y).any(|uy| ground(w, x, uy)) {
                roofed += 1;
            }
            if y < gy && g && w.get(x, y).material != provisions {
                mound += 1;
            }
            if (x - nx).abs() <= HOME_FOOD && w.get(x, y).material == provisions {
                home += 1;
            }
            if f32::from(w.pheromone_at(Channel::B, x, y)) > 0.1 * 0.1 * f32::from(pheromone::SCALE) {
                trail += 1;
            }
        }
    }
    let (mut under, mut laden_food, mut laden_soil) = (0, 0, 0);
    for &id in &all {
        let Some(s) = w.organism(id) else { continue };
        if let Some(&(x, y)) = s.chain.first() {
            under += usize::from(y > gy && (1..=6).any(|d| ground(w, x, y - d)));
        }
        match s.spoil.map(|l| l.cell.material) {
            Some(m) if m == provisions => laden_food += 1,
            Some(_) => laden_soil += 1,
            None => {}
        }
    }
    let tb = w
        .textbook
        .as_ref()
        .map(|t| {
            let c = t.counts;
            format!(
                " | textbook digs {} drops {} food_picks {} delivered {} stuck {}",
                c.digs, c.drops, c.food_picks, c.delivered, c.stuck
            )
        })
        .unwrap_or_default();
    let all = ants(w);
    let mut by_mat: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for x in x0..=x1 {
        for y in (gy - 60).max(0)..gy + 85 {
            let c = w.get(x, y);
            if c.material != material::EMPTY && c.organism_id() == 0 {
                *by_mat.entry(w.materials.get(c.material).name.clone()).or_default() += 1;
            }
        }
    }
    println!("  cells by material: {by_mat:?}");
    if std::env::var("ANTNEST_DUMP").is_ok() {
        for y in gy - 4..gy + 24 {
            let row: String = (nx - 30..=nx + 30)
                .map(|x| {
                    let c = w.get(x, y);
                    if c.organism_id() != 0 {
                        'a'
                    } else if c.material == material::EMPTY {
                        '.'
                    } else {
                        w.materials.get(c.material).name.chars().next().unwrap_or('?')
                    }
                })
                .collect();
            println!("  {:+3} {row}", y - gy);
        }
    }
    // Where the diggers stand (NestCfg's roles: the same hash as
    // `textbook::forager`), by depth below the old ground line, in bands of 4.
    if let Some(nc) = w.textbook.as_ref().and_then(|t| t.cfg.nest) {
        let mut bands: std::collections::BTreeMap<i32, (usize, usize, usize)> = std::collections::BTreeMap::new();
        for &id in &all {
            let h = (id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40;
            let forages = (h as f32 / (1u64 << 24) as f32) < nc.forager_frac;
            let Some(&(x, y)) = w.organism(id).and_then(|s| s.chain.first()) else {
                continue;
            };
            if (x - nx).abs() > NEST_HALF {
                continue;
            }
            let e = bands.entry((y - gy).div_euclid(4) * 4).or_default();
            let laden = w.organism(id).is_some_and(|s| s.spoil.is_some());
            if forages {
                e.2 += 1
            } else if laden {
                e.1 += 1
            } else {
                e.0 += 1
            }
        }
        println!("  (empty diggers, laden diggers, foragers) by depth band (rows below ground line): {bands:?}");
    }
    format!(
        "f={f} ants={} under={under} laden food/soil {laden_food}/{laden_soil} | dug {dug} roofed {roofed} mound {mound} | taken {taken} home {home} trail {trail}{tb}",
        all.len()
    )
}

/// **The colony eats what is brought home** (textbook arm only, which does
/// not eat): every food cell within [`HOME_FOOD`] columns of the door is
/// taken away, as `antbridge` takes away the crumbs put down in its nest.
/// Left, it lay on the pit's floor where diggers cannot cut, and sealed
/// the nest (first run, 2026-10-16). Returns cells taken.
fn eat_home(w: &mut World, provisions: material::MaterialId, nx: i32, gy: i32) -> usize {
    let mut n = 0;
    for x in nx - HOME_FOOD..=nx + HOME_FOOD {
        for y in gy - 60..gy + 85 {
            if w.in_bounds(x, y) && w.get(x, y).material == provisions {
                w.set(x, y, Cell::new(material::EMPTY, 0));
                n += 1;
            }
        }
    }
    n
}

/// Every ant topped up to `floor` of its start energy, the grant booked so
/// the ledger closes (as `antbridge`'s `feed`).
fn feed(world: &mut World, floor: f32) {
    use pixel_physics::sim::world::Account;
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        let Some(start) = world.species.get(st.species).creature.as_ref().map(|c| c.start_energy) else {
            continue;
        };
        let short = floor * start - st.energy;
        if short <= 0.0 {
            continue;
        }
        let colony = world.colony_of(id);
        world.set_organism_energy(id, floor * start);
        world.book(colony, Account::Granted, f64::from(short));
    }
}

/// The heap topped back up to `target` cells, dropped from a few rows up so
/// it lands as a heap (as `deeptrace`'s `top_up`). Returns cells added.
fn top_up(w: &mut World, provisions: material::MaterialId, fx: i32, gy: i32, target: usize) -> usize {
    const REACH: i32 = 6;
    let mut have = 0;
    for y in (gy - 40)..=(gy + 4) {
        for x in (fx - REACH)..=(fx + REACH) {
            have += usize::from(w.in_bounds(x, y) && w.get(x, y).material == provisions);
        }
    }
    let mut want = target.saturating_sub(have);
    let mut dropped = 0;
    let top = gy - 30;
    'rows: for y in top..(top + 6) {
        for k in 0..=4 {
            for x in [fx - k, fx + k] {
                if want == 0 {
                    break 'rows;
                }
                if w.in_bounds(x, y) && w.get(x, y).material == material::EMPTY {
                    w.set(x, y, Cell::new(provisions, 0));
                    want -= 1;
                    dropped += 1;
                }
            }
        }
    }
    dropped
}

/// The lab's own picture round `centre` (as `deeptrace`'s `shot`).
fn shot(lab: &mut Lab, dir: &str, frame: u64, centre: (i32, i32)) {
    let (full_w, full_h) = (WIDTH, HEIGHT);
    let bounds = pixel_physics::sim::chunk::Rect::new(0, 0, lab.spec.width - 1, lab.spec.height - 1);
    let (span_x, span_y) = lab.renderer.visible_span((full_w, full_h));
    lab.renderer.set_camera(
        centre.0 - span_x / 2,
        centre.1 - span_y / 2,
        (full_w, full_h),
        Some(bounds),
    );
    let mut full = vec![0u8; (full_w * full_h * 4) as usize];
    lab.draw(&mut full, 60.0);
    let (y0, rows) = (24u32, 216u32);
    let crop = full[(y0 * full_w * 4) as usize..((y0 + rows) * full_w * 4) as usize].to_vec();
    if let Some(img) = image::RgbaImage::from_raw(full_w, rows, crop) {
        let path = std::path::Path::new(dir).join(format!("shot_f{frame:06}.png"));
        let _ = image::imageops::resize(&img, full_w * 2, rows * 2, image::imageops::FilterType::Nearest).save(&path);
    }
}

/// A cell map of the nest region, four pixels a cell: open dark (the food
/// trail as red over it), the bed's soil brown, lining darker, spoil tan,
/// stone grey, food green, ants yellow.
fn map(w: &World, dir: &str, frame: u64, nx: i32, gy: i32, provisions: material::MaterialId) {
    const S: u32 = 4;
    let (x0, y0) = (nx - NEST_HALF, gy - 40);
    let (cw, ch) = (2 * NEST_HALF + 1, 40 + 80);
    let soil = w.materials.id_of("soil");
    let packed = w.materials.id_of("packedsoil");
    let spoil = w.materials.id_of("spoil");
    let mut img = image::RgbImage::new(cw as u32 * S, ch as u32 * S);
    for cy in 0..ch {
        for cx in 0..cw {
            let (x, y) = (x0 + cx, y0 + cy);
            let px = if !w.in_bounds(x, y) {
                [0, 0, 0]
            } else {
                let c = w.get(x, y);
                if c.organism_id() != 0 {
                    [240, 210, 40]
                } else if c.material == material::EMPTY {
                    let b = f32::from(w.pheromone_at(Channel::B, x, y)) / f32::from(pheromone::SCALE);
                    let r = (b / 0.5).clamp(0.0, 1.0);
                    [(20.0 + 200.0 * r) as u8, 20, 28]
                } else if c.material == provisions {
                    [60, 200, 70]
                } else if Some(c.material) == spoil {
                    [210, 170, 110]
                } else if Some(c.material) == packed {
                    [95, 60, 35]
                } else if Some(c.material) == soil {
                    [140, 95, 55]
                } else {
                    [110, 110, 115]
                }
            };
            for dy in 0..S {
                for dx in 0..S {
                    img.put_pixel(cx as u32 * S + dx, cy as u32 * S + dy, image::Rgb(px));
                }
            }
        }
    }
    // the old ground line, a faint tick at each edge
    for dy in 0..S {
        for dx in 0..S * 2 {
            img.put_pixel(dx, 40 * S + dy, image::Rgb([255, 255, 255]));
            img.put_pixel(cw as u32 * S - 1 - dx, 40 * S + dy, image::Rgb([255, 255, 255]));
        }
    }
    let _ = img.save(std::path::Path::new(dir).join(format!("map_f{frame:06}.png")));
}
