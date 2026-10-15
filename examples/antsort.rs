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

use pixel_physics::sim::creature::{self, textbook};
use pixel_physics::sim::parallel;
use pixel_physics::sim::rng::Rng;
use pixel_physics::sim::{Cell, MaterialId, Rect, World};

const KEYS: &[&str] = &[
    "seed", "seeds", "frames", "every", "ants", "items", "width", "height", "rule", "png", "scale", "bonusm",
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
        let r = run(seed, &rule, frames, every);
        passes += usize::from(r.0);
        ratios.push(r.1);
    }
    println!(
        "SUMMARY rule={rule}: d_medium/d_small, d_large/d_small per seed [{}]; pass {passes} of {} (Franks colony A: 1.43-1.67, 2.05-2.68)",
        ratios.iter().map(|(m, l)| format!("{m:.2},{l:.2}")).collect::<Vec<_>>().join(" "),
        seeds.len()
    );
}

fn run(seed: u64, rule: &str, frames: u64, every: u64) -> (bool, (f32, f32)) {
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
    world.textbook = Some(Box::new(textbook::Textbook::new(id, cfg)));
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
    let mut result = (false, (0.0, 0.0));
    for f in 1..=frames {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        if f % FEED_EVERY == 0 {
            feed(&mut world);
        }
        if f % every == 0 {
            let counts = world.textbook.as_ref().map(|t| t.counts).unwrap_or_default();
            let alive = world.live_organism_ids().len();
            result = score(&world, brood, n_items, seed, f);
            println!(
                "      alive {alive} picks {} drops {}/{} no_site {} steps {}",
                counts.picks, counts.drops, counts.rolls, counts.no_site, counts.steps
            );
            if let Some(dir) = arg::<String>("png") {
                let _ = std::fs::create_dir_all(&dir);
                let _ = draw(&world, brood, stone, w, h).save(format!("{dir}/s{seed}_f{f:07}.png"));
            }
        }
    }
    result
}

fn feed(world: &mut World) {
    use pixel_physics::sim::world::Account;
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
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
fn score(world: &World, brood: MaterialId, n_items: usize, seed: u64, f: u64) -> (bool, (f32, f32)) {
    let b = world.bounds().expect("bounds");
    let items: Vec<(i32, i32, u8)> = (b.min_y..=b.max_y)
        .flat_map(|y| (b.min_x..=b.max_x).map(move |x| (x, y)))
        .filter(|&(x, y)| world.get(x, y).material == brood && world.get(x, y).organism_id() == 0)
        .map(|(x, y)| (x, y, world.get(x, y).shade.min(2)))
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
    let pass = share >= 0.8 && counts.iter().all(|&c| c >= 10) && rl > p95;
    println!(
        "seed {seed} f {f:>7}: cluster {share:.2} at x {cx:.0} of items {counts:?} | along the floor: small {:.1} medium {:.1} large {:.1} | ratios {rm:.2} {rl:.2} (shuffle p95 {p95:.2}) | {}",
        d[0],
        d[1],
        d[2],
        if pass { "PASS" } else { "fail" }
    );
    (pass, (rm, rl))
}

/// Small white, medium cream, large brown, ants red.
fn draw(world: &World, brood: MaterialId, stone: MaterialId, w: i32, h: i32) -> image::RgbaImage {
    const PX: u32 = 4;
    let mut img = image::RgbaImage::from_pixel(w as u32 * PX, h as u32 * PX, image::Rgba([24, 22, 28, 255]));
    for y in 0..h {
        for x in 0..w {
            let c = world.get(x, y);
            let col = if c.material == stone {
                Some([95, 95, 100, 255])
            } else if c.material == brood && c.organism_id() == 0 {
                Some([[250, 250, 250, 255], [230, 200, 120, 255], [170, 90, 40, 255]][c.shade.min(2) as usize])
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
