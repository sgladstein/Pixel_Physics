//! **The textbook ant sorting brood by size, in our side view** -- slice C
//! on the game's body and engine (`creature::textbook::SortCfg`), the
//! real-ant tier of `examples/broodsort`'s walker box.
//!
//! A stone chamber `width` x `height`, `items=` inert brood cells of each
//! of three sizes (the cell's shade: 0 small, 1 medium, 2 large) dropped on
//! the floor, and `ants=` textbook ants. Births are paused, and every ant is
//! topped up to full every 500 frames, so the colony neither grows nor
//! starves. `rule=room` is Don & Amos's annular rule (`SortCfg::room`).
//! `rule=off` is the same ants with every pick-up and put-down at a flat
//! 0.1: the gravity control, which still makes one heap.
//!
//! Scored as the walker box scores its side view (`along=floor`, the
//! translation to the slice in
//! `/mnt/project-files/emergence-sandbox/translating-tests-to-the-slice-2026-10-15.md`):
//! in the largest cluster (items joined within 4 cells), each size's mean
//! distance along the floor from the pile's middle, as a ratio to the small
//! items', against 200 label shuffles. Franks & Sendova-Franks 1992's
//! colony A: medium 1.43-1.67, large 2.05-2.68. Pass: the cluster holds at
//! least 0.8 of the items with at least 10 of each size, and
//! `d_large/d_small` is above the shuffles' 95th percentile.
//!
//! ```text
//! cargo run --release --example antsort -- seeds=8 frames=600000
//! cargo run --release --example antsort -- rule=off seed=1 png=out
//! ```

use pixel_physics::sim::brood;
use pixel_physics::sim::creature::{self, textbook};
use pixel_physics::sim::organism::BroodStage;
use pixel_physics::sim::parallel;
use pixel_physics::sim::rng::Rng;
use pixel_physics::sim::{Cell, MaterialId, Rect, World};

const KEYS: &[&str] = &[
    "seed", "seeds", "frames", "every", "ants", "items", "width", "height", "rule", "png", "scale", "bonusm", "ant",
    "carry",
];

const FEED_EVERY: u64 = 500;

fn check_args() {
    let mut seen = std::collections::HashSet::new();
    for a in std::env::args().skip(1) {
        let key = a.split_once('=').map_or(a.as_str(), |(k, _)| k).to_string();
        assert!(
            KEYS.contains(&key.as_str()),
            "antsort: unknown argument `{a}` (known: {})",
            KEYS.join(", ")
        );
        assert!(seen.insert(key.clone()), "antsort: `{key}` given twice");
    }
}

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| {
            v.parse()
                .ok()
                .unwrap_or_else(|| panic!("antsort: `{a}` does not parse"))
        })
    })
}

fn main() {
    check_args();
    creature::mutation_off_for_measuring();
    let seeds: Vec<u64> = match arg::<u64>("seed") {
        Some(s) => vec![s],
        None => (1..=arg::<u64>("seeds").unwrap_or(4)).collect(),
    };
    let frames: u64 = arg("frames").unwrap_or(600_000);
    let every: u64 = arg("every").unwrap_or(frames / 6);
    let rule: String = arg("rule").unwrap_or_else(|| "room".into());
    assert!(rule == "room" || rule == "off", "rule={rule}: room or off");
    let kind: String = arg("ant").unwrap_or_else(|| "textbook".into());
    assert!(
        kind == "textbook" || kind == "game" || kind == "walk",
        "ant={kind}: textbook, game or walk"
    );
    if kind != "textbook" {
        // The game ant and the new walk move brood only by the game's brood
        // carry, which reads live brood: hold every stage (no hatching, no
        // pupating), and let the harness, not nurses, keep larvae fed, so
        // the three sizes stay fixed. `carry=` is `BROOD_CARRY` (off on main).
        let carry: String = arg("carry").unwrap_or_else(|| "off".into());
        std::env::set_var("PIXEL_PHYSICS_BROOD_CARRY", &carry);
        std::env::set_var("PIXEL_PHYSICS_BROOD_FRAMES", "100000000,100000000");
        std::env::set_var("PIXEL_PHYSICS_NURSE", "off");
        println!("antsort: ant={kind} BROOD_CARRY={carry} (rule= is the textbook ant's only)");
    }
    println!(
        "antsort: rule={rule} ants={} items={} per size box={}x{} frames={frames}",
        arg::<usize>("ants").unwrap_or(30),
        arg::<usize>("items").unwrap_or(25),
        arg::<i32>("width").unwrap_or(80),
        arg::<i32>("height").unwrap_or(40)
    );
    let mut passes = 0;
    let mut ratios = Vec::new();
    for &seed in &seeds {
        let r = run(seed, &rule, &kind, frames, every);
        passes += usize::from(r.0);
        ratios.push(r.1);
    }
    println!(
        "SUMMARY rule={rule}: d_medium/d_small, d_large/d_small per seed [{}]; pass {passes} of {} (Franks colony A: 1.43-1.67, 2.05-2.68)",
        ratios.iter().map(|(m, l)| format!("{m:.2},{l:.2}")).collect::<Vec<_>>().join(" "),
        seeds.len()
    );
}

fn run(seed: u64, rule: &str, kind: &str, frames: u64, every: u64) -> (bool, (f32, f32)) {
    let (w, h): (i32, i32) = (arg("width").unwrap_or(80), arg("height").unwrap_or(40));
    let n_items: usize = arg("items").unwrap_or(25);
    let n_ants: usize = arg("ants").unwrap_or(30);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    world.seed = seed;
    let stone = world.materials.id_of("stone").expect("stone");
    let brood = world.materials.id_of("brood").expect("brood");
    for y in 0..h {
        for x in 0..w {
            if x < 2 || y < 2 || x >= w - 2 || y >= h - 2 {
                world.set(x, y, Cell::new(stone, 0).with_attached(true));
            }
        }
    }
    let floor = h - 3;
    let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xa5017);
    // Items are dropped from random heights and fall to the floor.
    for size in 0..3u8 {
        let mut placed = 0;
        while placed < n_items {
            let (x, y) = (
                2 + rng.below((w - 4) as u32) as i32,
                2 + rng.below((h - 6) as u32) as i32,
            );
            if world.is_empty(x, y) {
                world.set(x, y, Cell::new(brood, size));
                placed += 1;
            }
        }
    }
    for _ in 0..200 {
        parallel::step(&mut world);
    }
    let id = world.species.id_of("ant").expect("ant ships");
    if let Some(c) = world.species.get_mut(id).creature.as_mut() {
        c.life_half_life = 10_000_000;
    }
    world.births_paused = true;
    let mut sort = textbook::SortCfg::room(brood);
    sort.scale = arg("scale").unwrap_or(sort.scale);
    sort.bonus_m = arg("bonusm").unwrap_or(sort.bonus_m);
    if rule == "off" {
        // `broodsort`'s `off`: every score 0.1, so picks at 0.9, drops at 0.1.
        sort.flat = Some(0.1);
    }
    let cfg = textbook::TextbookCfg {
        p_move: 1.0,
        turn: 0.125,
        dt: 4.0,
        drop_base: 0.025,
        drop_per: 0.11,
        pick: 0.029,
        tau: 0.0,
        template: 2.0,
        source: brood,
        n_rock: false,
        pick_around: true,
        base_always: true,
        build_on_box: false,
        forage: None,
        sort: Some(sort),
    };
    if kind == "textbook" {
        world.textbook = Some(Box::new(textbook::Textbook::new(id, cfg)));
    } else {
        // The new walk reads home from a founded site; a built chamber has
        // none (`antbridge`'s smoke run). Harmless for the game ant.
        world.register_nest_site(w / 2, floor, w / 2 - 2);
    }
    let mut colony = None;
    let mut placed = 0;
    let mut tries = 0;
    while placed < n_ants && tries < 10_000 {
        tries += 1;
        // Anywhere in the chamber: on the floor row most fail, since the
        // brood lies there.
        let x = 3 + rng.below((w - 6) as u32) as i32;
        let y = 3 + rng.below((floor - 3) as u32) as i32;
        if let Some(site) = creature::plant_founder_in(&mut world, x, y, "ant", colony, None) {
            if colony.is_none() {
                colony = creature::colony_of_site(&world, &site);
            }
            world.schedule_active_site(site);
            placed += 1;
        }
    }
    println!("  seed {seed}: {placed} of {n_ants} ants placed");
    // **Live brood for the game ant and the new walk**, at the inert items'
    // own cells so every arm starts from the same layout: small = egg,
    // medium = pupa, large = larva (Franks's order: pupae lie between the
    // medium and the largest larvae). The cell's shade is then the stage.
    let parent = world.live_organism_ids().into_iter().next();
    let mut larva_target = 0.0f32;
    if kind != "textbook" {
        let parent = parent.expect("an ant to be the brood's parent");
        let distinct: std::collections::HashSet<String> = world
            .live_organism_ids()
            .iter()
            .filter_map(|&o| world.organism(o).map(|s| format!("{:?}", s.traits)))
            .collect();
        println!("  founders' distinct trait sets: {}", distinct.len());
        let start = world.species.get(id).creature.as_ref().map_or(0.0, |c| c.start_energy);
        let b = world.bounds().expect("bounds");
        let cells: Vec<(i32, i32, u8)> = (b.min_y..=b.max_y)
            .flat_map(|y| (b.min_x..=b.max_x).map(move |x| (x, y)))
            .filter(|&(x, y)| world.get(x, y).material == brood && world.get(x, y).organism_id() == 0)
            .map(|(x, y)| (x, y, world.get(x, y).shade))
            .collect();
        for (x, y, size) in cells {
            let stage = [BroodStage::Egg, BroodStage::Pupa, BroodStage::Larva][usize::from(size.min(2))];

            world.set(x, y, Cell::EMPTY);
            let site = brood::place_for_harness(&mut world, parent, (x, y), stage, start * 0.5)
                .expect("brood fits where the item lay");
            world.schedule_active_site(site);
        }
        larva_target = start * 0.5;
    }
    let mut result = (false, (0.0, 0.0));
    if kind == "walk" {
        // As `antbridge`'s `ant=walk`: the new walk from the first frame
        // (the chamber is built, so there is no founding to wait out).
        let mut walk = creature::needs::NeedsWalk::new(creature::needs::NeedsMode::Walk, world.frame);
        walk.parts = creature::needs::WalkParts::NEW_ANT;
        world.needs = Some(Box::new(walk));
    }
    for f in 1..=frames {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        if f % FEED_EVERY == 0 {
            feed(&mut world, larva_target);
        }
        if f % every == 0 {
            let counts = world.textbook.as_ref().map(|t| t.counts).unwrap_or_default();
            let alive = world.live_organism_ids().len();
            result = score(&world, brood, kind != "textbook", n_items, seed, f);
            println!(
                "      alive {alive} picks {} drops {}/{} no_site {} steps {} | live brood {} carried {} starved {} lost {}",
                counts.picks,
                counts.drops,
                counts.rolls,
                counts.no_site,
                counts.steps,
                world.live_brood_ids().len(),
                world.creature_stats.brood_carried,
                world.creature_stats.larvae_starved,
                world.creature_stats.brood_lost
            );
            if alive < n_ants {
                println!(
                    "      deaths by cause {:?} brood ate {:.0} J",
                    world.deaths_by_cause, world.creature_stats.brood_ate_j
                );
                for &(_, _, m, n) in &world.vital_losses {
                    println!("      head cell now {} x{n}", world.materials.get(m).name);
                }
                for g in &world.group_deaths {
                    println!("      group colony {} killed_by {:?}", g.colony, g.killed_by);
                }
            }
            if let Some(dir) = arg::<String>("png") {
                let _ = std::fs::create_dir_all(&dir);
                let _ = draw(&world, brood, stone, kind != "textbook", w, h).save(format!("{dir}/s{seed}_f{f:07}.png"));
            }
        }
    }
    result
}

/// Tops every adult up to its start bank, and every larva up to
/// `larva` (half the adult's start: never its target, so it never pupates).
fn feed(world: &mut World, larva: f32) {
    use pixel_physics::sim::world::Account;
    // `live_organism_ids` leaves brood out; the larvae are fed here too.
    for id in world.live_organism_ids().into_iter().chain(world.live_brood_ids()) {
        let Some(st) = world.organism(id) else { continue };
        if let Some(b) = st.brood {
            // **The brood takes the nest's odour**, as groomed brood does.
            // Without it a larva held for 300k frames kept its founder's
            // scent while the nest's drifted, came to smell foreign, and
            // ate the ants beside it: 30 of 30 `Killed` with their head
            // cells left empty, `brood_ate_j` 5,476 J, on 4 of 16 seeds in
            // both the game and walk rows (traced 2026-10-15). In the game a
            // larva lives far shorter than this harness holds it.
            if let Some(nest) = world.nest_sites.first().filter(|n| n.seeded).map(|n| n.scent) {
                for (k, slot) in pixel_physics::sim::organism::SCENT_SLOTS.iter().enumerate() {
                    world.set_organism_trait(id, *slot, nest[k]);
                }
            }
            let Some(st) = world.organism(id) else { continue };
            if b.stage == BroodStage::Larva && st.energy < larva {
                let short = larva - st.energy;
                let colony = world.colony_of(id);
                world.set_organism_energy(id, larva);
                world.book(colony, Account::Granted, f64::from(short));
            }
            continue;
        }
        let Some(start) = world.species.get(st.species).creature.as_ref().map(|c| c.start_energy) else {
            continue;
        };
        let short = start - st.energy;
        if short <= 0.0 {
            continue;
        }
        let colony = world.colony_of(id);
        world.set_organism_energy(id, start);
        world.book(colony, Account::Granted, f64::from(short));
    }
}

/// The largest cluster's order along the floor, printed; returns (pass,
/// ratios). The same score as `broodsort`'s `along=floor`.
/// `live`: the items are live brood (the game ant's and the new walk's
/// rows), sized by stage -- egg small, pupa medium, larva large.
fn score(world: &World, brood: MaterialId, live: bool, n_items: usize, seed: u64, f: u64) -> (bool, (f32, f32)) {
    let b = world.bounds().expect("bounds");
    let items: Vec<(i32, i32, u8)> = (b.min_y..=b.max_y)
        .flat_map(|y| (b.min_x..=b.max_x).map(move |x| (x, y)))
        .filter(|&(x, y)| world.get(x, y).material == brood && (world.get(x, y).organism_id() != 0) == live)
        .map(|(x, y)| {
            let shade = world.get(x, y).shade.min(2);
            (x, y, if live { [0, 2, 1][usize::from(shade)] } else { shade })
        })
        .collect();
    let index: std::collections::HashMap<(i32, i32), usize> =
        items.iter().enumerate().map(|(i, &(x, y, _))| ((x, y), i)).collect();
    let mut label = vec![usize::MAX; items.len()];
    let mut best: Vec<usize> = Vec::new();
    for start in 0..items.len() {
        if label[start] != usize::MAX {
            continue;
        }
        let mut group = vec![start];
        label[start] = start;
        let mut k = 0;
        while k < group.len() {
            let (x, y, _) = items[group[k]];
            for dy in -4..=4 {
                for dx in -4..=4 {
                    if let Some(&j) = index.get(&(x + dx, y + dy)) {
                        if label[j] == usize::MAX {
                            label[j] = start;
                            group.push(j);
                        }
                    }
                }
            }
            k += 1;
        }
        if group.len() > best.len() {
            best = group;
        }
    }
    let cl: Vec<(i32, i32, u8)> = best.iter().map(|&i| items[i]).collect();
    let n = cl.len().max(1) as f32;
    let cx = cl.iter().map(|c| c.0 as f32).sum::<f32>() / n;
    let dist: Vec<f32> = cl.iter().map(|c| (c.0 as f32 - cx).abs()).collect();
    let mean_by = |labels: &[u8], s: u8| -> f32 {
        let (sum, cnt) = labels
            .iter()
            .zip(&dist)
            .filter(|(l, _)| **l == s)
            .fold((0.0, 0), |(a, c), (_, d)| (a + d, c + 1));
        if cnt == 0 {
            f32::NAN
        } else {
            sum / cnt as f32
        }
    };
    let labels: Vec<u8> = cl.iter().map(|c| c.2).collect();
    let counts = [0u8, 1, 2].map(|s| labels.iter().filter(|&&l| l == s).count());
    let d = [0u8, 1, 2].map(|s| mean_by(&labels, s));
    let (rm, rl) = (d[1] / d[0], d[2] / d[0]);
    let mut rng = Rng::new(seed ^ f ^ 0x57f);
    let mut null: Vec<f32> = (0..200)
        .map(|_| {
            let mut l = labels.clone();
            for i in (1..l.len()).rev() {
                let j = rng.below((i + 1) as u32) as usize;
                l.swap(i, j);
            }
            mean_by(&l, 2) / mean_by(&l, 0)
        })
        .collect();
    null.sort_by(|a, b| a.total_cmp(b));
    let p95 = null[189];
    let share = cl.len() as f32 / (3 * n_items) as f32;
    let beats_shuffle = share >= 0.8 && counts.iter().all(|&c| c >= 10) && rl > p95;
    // **Space per item** (Franks & Sendova-Franks 1992 measured each item's
    // Dirichlet tile and found it grows from the centre out): a discrete
    // Voronoi over the open and brood cells of the cluster's own span, up to
    // `TILE_ROWS` above the floor, each cell going to its nearest cluster
    // item. Mean cells per item, by size.
    const TILE_ROWS: i32 = 8;
    let (x0, x1) = cl
        .iter()
        .fold((i32::MAX, i32::MIN), |(a, b), c| (a.min(c.0), b.max(c.0)));
    let floor_y = cl.iter().map(|c| c.1).max().unwrap_or(0);
    let mut tile = [0usize; 3];
    for y in (floor_y - TILE_ROWS)..=floor_y {
        for x in x0..=x1 {
            let c = world.get(x, y);
            if !(c.material == brood || world.is_empty(x, y) || c.organism_id() != 0) {
                continue;
            }
            if let Some(near) = cl.iter().min_by_key(|i| (i.0 - x).pow(2) + (i.1 - y).pow(2)) {
                tile[usize::from(near.2)] += 1;
            }
        }
    }
    let t = [0, 1, 2].map(|s| tile[s] as f32 / counts[s].max(1) as f32);
    // **The paper's bar** (2026-10-15, after Scott read the picture: "small
    // and medium are mixed together and the brown are all left as separate
    // individual pixels"). Beating label shuffles only says large lies
    // further out than chance; it passed seeds whose large items ran to the
    // walls (ratio 9.65). So, per seed: one cluster (0.8 of the items, 10 of
    // each size), the order ratios inside the published span with headroom
    // -- medium 1.37-1.76, large 2.05-2.68 over Franks's colony A before and
    // after emigration and Don & Amos's two experiments; bar medium
    // 1.2-2.0, large 1.7-3.2, medium below large -- and space per item
    // growing with size, as Franks's tiles did.
    let in_band = (1.2..=2.0).contains(&rm) && (1.7..=3.2).contains(&rl) && rm < rl;
    let tiles_grow = t[0] < t[1] && t[1] < t[2];
    let pass = share >= 0.8 && counts.iter().all(|&c| c >= 10) && in_band && tiles_grow;
    println!(
        "seed {seed} f {f:>7}: cluster {share:.2} at x {cx:.0} span {x0}-{x1} of items {counts:?} | along the floor: small {:.1} medium {:.1} large {:.1} | ratios {rm:.2} {rl:.2} (shuffle p95 {p95:.2}{}) | space per item {:.1} {:.1} {:.1} | {}",
        d[0],
        d[1],
        d[2],
        if beats_shuffle { ", beaten" } else { "" },
        t[0],
        t[1],
        t[2],
        if pass { "PASS" } else { "fail" }
    );
    (pass, (rm, rl))
}

/// Small white, medium cream, large brown, ants red.
fn draw(world: &World, brood: MaterialId, stone: MaterialId, live: bool, w: i32, h: i32) -> image::RgbaImage {
    const PX: u32 = 4;
    let mut img = image::RgbaImage::from_pixel(w as u32 * PX, h as u32 * PX, image::Rgba([24, 22, 28, 255]));
    for y in 0..h {
        for x in 0..w {
            let c = world.get(x, y);
            let col = if c.material == stone {
                Some([95, 95, 100, 255])
            } else if c.material == brood {
                let class = if live {
                    [0, 2, 1][usize::from(c.shade.min(2))]
                } else {
                    usize::from(c.shade.min(2))
                };
                Some([[250, 250, 250, 255], [230, 200, 120, 255], [170, 90, 40, 255]][class])
            } else if c.organism_id() != 0 {
                Some([220, 50, 50, 255])
            } else {
                None
            };
            if let Some(col) = col {
                for dy in 0..PX {
                    for dx in 0..PX {
                        img.put_pixel(x as u32 * PX + dx, y as u32 * PX + dy, image::Rgba(col));
                    }
                }
            }
        }
    }
    img
}
