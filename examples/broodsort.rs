//! **Brood sorting by stage, scored the way Franks & Sendova-Franks 1992
//! scored it** -- the walker box for the textbook ant's slice C
//! (`/mnt/project-files/emergence-sandbox/sorting/slice-c-design-2026-10-15.md`).
//!
//! *Leptothorax* keeps its brood in **one cluster of concentric rings**:
//! the smallest items in the centre, the largest at the edge. Franks's
//! colony A, as Don & Amos (arXiv nlin/0507045, table) give it: mean
//! distance from the cluster centroid, over the small items' distance, is
//! **1.43-1.67 for medium and 2.05-2.68 for large** (before and after an
//! emigration). That ratio is this binary's score.
//!
//! A bare stone box, `items=` brood cells of each of three sizes (the cell's
//! shade: 0 small, 1 medium, 2 large; inert, no development), and walkers
//! that are not the shipped ant: a persistent random walk, one item in the
//! jaws, and one published pick-up/put-down rule (`rule=`):
//! - `off` -- pick up and put down at a flat `0.1`: the control.
//! - `deneubourg` -- Deneubourg et al. 1991, `f` the share of the 24 cells
//!   round holding the **same size**: `(k1/(k1+f))^2`, `(f/(k2+f))^2`. Its
//!   own model sorts kinds into separate clusters, not rings.
//! - `brood` -- the same with `f` over **all** brood: today's
//!   `BROOD_CARRY=pile`. It cannot tell sizes apart.
//! - `room` -- Don & Amos's annular rule (after Wilson et al. 2004's
//!   robots): a placement score `base + bonus * (items within bonus_m *
//!   size) + penalty * (items within penalty_m * size)`, clamped to 0..1;
//!   put down where a random draw is under it, picked up where a draw is
//!   over it. Sizes 0.5, 1.5, 3; `penalty -50`, `bonus 0.1`, multipliers
//!   0.4 and 4, base 0, a 4-step delay after each act: the paper's
//!   Experiment 1. A big item needs more clear room, so it lies further
//!   out. **On the grid** every item is one cell and "within" is the
//!   centre-to-centre distance (theirs is edge to edge).
//! - `pullback` -- Wilson et al. 2004's extended differential pullback: a
//!   laden walker that touches brood puts a small item down there and
//!   backs `pull=` cells (0,3,5) away with a bigger one first; picked up by
//!   the all-brood share (`brood`'s pick). **It clusters and does not
//!   order** (2026-10-15, 8 seeds top-down, `pull` 0,2,4 / 0,3,5 / 0,3,6 /
//!   1,3,6 / 0,4,8: 0 in the paper's band, ratios ~1); with a flat `pick=`
//!   nothing clusters at all.
//!
//! - `body=1,2,3` (owner's card "Bigger brood", 2026-10-15): small,
//!   medium and large are 1x1, 2x2 and 3x3 cells, top-down only, scored
//!   per item. With Don & Amos's own penalty form (`penm=0.4`: room 0.4 x
//!   width, edge to edge) and reach `bonusm=5` x width, 16 seeds at 200k:
//!   **one cluster of rings on nearly every seed (largest 0.67-1.00) and
//!   the order right on 16 of 16**, but spread ~1.3x the paper's (medium
//!   1.28-3.27, large 2.05-4.92), so 5/16 in the band; random carrying
//!   0/16 (largest cluster 0.11-0.25). Reach 4 (the paper's): 1/16, half
//!   the seeds without order; `penm` 0.6 or 0.8: no cluster. The old
//!   `room=` x (width - 1) form at reach 6: 5/16, small items packed
//!   solid and rings further out still.
//! - `body=1,2,3 medium=game along=floor` (side view; bodies fall whole,
//!   nothing slides): 80 wide, **0/16, no better than random** (`off`
//!   0/16, ratios ~1). 240 wide, 8 seeds at 300k: one heap against an end
//!   wall, small deepest in the corner and large on top and outside, so
//!   large is further out on 5/8 (1.53-2.32) but medium is mixed through
//!   (1.0-1.49); 1/8 in the band, random 0/8 and no cluster. Gravity turns
//!   the rings into a layered corner heap. `layer=1` (put down only on the
//!   floor, one layer) never clusters: 22-27 of 30 walkers left holding.
//!
//! **The bar is the paper's** (2026-10-15, after the owner read a "13/16"
//! picture with medium mixed into small and large strung along the floor):
//! the ratios inside the published span with headroom, medium 1.2-2.0 and
//! large 1.7-3.2. Beating label shuffles is printed as "beaten" and is not
//! a pass. Under it the tuned room rule is 3/16 top-down (large 2.3-4.8)
//! and 6/16 side view; every item is one cell, so the room a size needs
//! rounds to whole cells.
//!
//! `medium=` `flat` (items stay put, walkers go anywhere: the paper's flat
//! nest between glass, the positive control), `game` (the game's physics
//! moves items, walkers need footing) or `rest` (walkers need footing; an
//! item falls straight down onto whatever is under it but never slides off
//! a heap, so it stays where it was put -- brood placed, not poured).
//!
//! Read per seed at the end, in the largest cluster (items joined within
//! `join=` cells, 4): its share of all items, the count per size, `d_small`,
//! `d_medium`, `d_large` from its centroid, the two ratios, and where
//! `d_large/d_small` stands in 200 label shuffles of the same cells.
//! **Pass** (paper-bars-audit section 2): one cluster holding at least 0.8
//! of the items with at least 10 of each size, and `d_large/d_small` above
//! the shuffles' 95th percentile.
//!
//! **What it found, 2026-10-15** (200k frames, 25 a size, 30 walkers, 16
//! seeds, after the turn-order fix below; full table
//! `/mnt/project-files/emergence-sandbox/sorting/broodsort-results-2026-10-15.md`):
//! - Flat: `off`, `deneubourg` and `brood` pass 0/16. `room` passes 7/16 at
//!   the paper's constants and **13/16 at `scale=1.5 bonusm=3`** (large
//!   ratio 2.3-4.8, about twice as loose as Franks's 2.05-2.68). One cell
//!   per item cannot pack small items as densely as their 0.5 size.
//! - `scatter=150000`: re-forms 8/8, 15k-60k frames after the scatter.
//! - **Side view (`medium=game along=floor`): `room` tuned 11/16**, with
//!   medians 1.74 and 1.97 against Franks's 1.43-1.67 and 2.05-2.68;
//!   `brood` 3/16, `off` 0/16.
//! - `medium=rest` sorts worse (tuned 4/16): stopping brood sliding does
//!   not help.
//!
//! Two harness traps were found here. `join=2` cut the spaced outer ring
//! off the cluster, which is why the default is 4. And a fixed turn order
//! put every cluster in one corner, which inflated `rest` to 6/8.
//!
//! ```text
//! cargo run --release --example broodsort -- rule=room scale=1.5 bonusm=3 seeds=8 frames=200000 every=200000
//! cargo run --release --example broodsort -- rule=brood medium=game seed=1 frames=200000 png=out
//! ```

use pixel_physics::sim::parallel;
use pixel_physics::sim::rng::Rng;
use pixel_physics::sim::{Cell, MaterialId, Rect, World};

const KEYS: &[&str] = &[
    "seed", "seeds", "frames", "every", "walkers", "width", "height", "rule", "medium", "items", "k1", "k2", "png", "scale", "join", "bonusm", "scatter", "along", "layer",
    "pull", "pick", "body", "room", "penm",
];

fn check_args() {
    let mut seen = std::collections::HashSet::new();
    for a in std::env::args().skip(1) {
        let key = a.split_once('=').map_or(a.as_str(), |(k, _)| k).to_string();
        assert!(
            KEYS.contains(&key.as_str()),
            "broodsort: unknown argument `{a}` (known: {})",
            KEYS.join(", ")
        );
        assert!(seen.insert(key.clone()), "broodsort: `{key}` given twice");
    }
}

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}="))
            .map(|v| v.parse().ok().unwrap_or_else(|| panic!("broodsort: `{a}` does not parse")))
    })
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Rule {
    Off,
    Deneubourg,
    Brood,
    Room,
    Pullback,
}

const DIRS: [(i32, i32); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];

/// Don & Amos's sizes for small, medium and large, and their Experiment 1.
const SIZES: [f32; 3] = [0.5, 1.5, 3.0];
const PENALTY: f32 = -50.0;
const BONUS: f32 = 0.1;
const PENALTY_M: f32 = 0.4;
const BONUS_M: f32 = 4.0;
const DELAY: u32 = 4;

struct Walker {
    x: i32,
    y: i32,
    dir: usize,
    held: Option<u8>,
    wait: u32,
    /// `pullback`: steps still to back away before putting the item down.
    back: u32,
}

fn main() {
    check_args();
    let rule = match arg::<String>("rule").as_deref().unwrap_or("room") {
        "off" => Rule::Off,
        "deneubourg" => Rule::Deneubourg,
        "brood" => Rule::Brood,
        "room" => Rule::Room,
        "pullback" => Rule::Pullback,
        r => panic!("rule={r}: off, deneubourg, brood, room or pullback"),
    };
    let medium: String = arg("medium").unwrap_or_else(|| "flat".into());
    assert!(matches!(medium.as_str(), "flat" | "game" | "rest"), "medium={medium}: flat, game or rest");
    let seeds: Vec<u64> = match arg::<u64>("seed") {
        Some(s) => vec![s],
        None => (1..=arg::<u64>("seeds").unwrap_or(4)).collect(),
    };
    let frames: u64 = arg("frames").unwrap_or(40_000);
    let every: u64 = arg("every").unwrap_or(10_000);
    let n_walkers: usize = arg("walkers").unwrap_or(30);
    let n_items: usize = arg("items").unwrap_or(25);
    let (w, h): (i32, i32) = (arg("width").unwrap_or(80), arg("height").unwrap_or(60));
    println!(
        "broodsort: rule={rule:?} medium={} frames={frames} walkers={n_walkers} items={n_items} per size box={w}x{h}",
        medium
    );
    let mut passes = 0;
    let mut ratios = Vec::new();
    for &seed in &seeds {
        let r = run(seed, rule, &medium, frames, every, n_walkers, n_items, w, h);
        passes += usize::from(r.0);
        ratios.push(r.1);
    }
    println!(
        "SUMMARY rule={rule:?} medium={}: d_medium/d_small, d_large/d_small per seed [{}]; pass {passes} of {} (Franks colony A: 1.43-1.67, 2.05-2.68)",
        medium,
        ratios.iter().map(|(m, l)| format!("{m:.2},{l:.2}")).collect::<Vec<_>>().join(" "),
        seeds.len()
    );
}

/// The size (0 small, 1 medium, 2 large) of the brood item at `(x, y)`.
fn size_at(world: &World, brood: MaterialId, x: i32, y: i32) -> Option<u8> {
    (world.in_bounds(x, y) && world.get(x, y).material == brood).then(|| world.get(x, y).shade)
}

/// The rule's chance to put an item of `size` down at `(x, y)` (`drop`) or
/// to pick one up from there, `skip` being the item itself.
#[allow(clippy::too_many_arguments)]
fn chance(world: &World, brood: MaterialId, rule: Rule, size: u8, (x, y): (i32, i32), skip: (i32, i32), drop: bool, k: (f32, f32)) -> f32 {
    match rule {
        Rule::Off => 0.1,
        // Not read: `pullback` decides in the walker loop.
        Rule::Pullback => 0.0,
        Rule::Deneubourg | Rule::Brood => {
            let mut n = 0;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if (dx, dy) == (0, 0) || (x + dx, y + dy) == skip {
                        continue;
                    }
                    if let Some(s) = size_at(world, brood, x + dx, y + dy) {
                        n += u32::from(rule == Rule::Brood || s == size);
                    }
                }
            }
            let f = n as f32 / 24.0;
            if drop {
                (f / (k.1 + f)).powi(2)
            } else {
                (k.0 / (k.0 + f)).powi(2)
            }
        }
        Rule::Room => {
            let s = SIZES[size as usize] * arg::<f32>("scale").unwrap_or(1.0);
            let (near, far) = (PENALTY_M * s, arg::<f32>("bonusm").unwrap_or(BONUS_M) * s);
            let r = far.ceil() as i32;
            let mut score = 0.0f32;
            for dy in -r..=r {
                for dx in -r..=r {
                    if (dx, dy) == (0, 0) || (x + dx, y + dy) == skip || size_at(world, brood, x + dx, y + dy).is_none() {
                        continue;
                    }
                    let d = ((dx * dx + dy * dy) as f32).sqrt();
                    if d <= near {
                        score += PENALTY;
                    } else if d <= far {
                        score += BONUS;
                    }
                }
            }
            let score = score.clamp(0.0, 1.0);
            if drop {
                score
            } else {
                1.0 - score
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run(seed: u64, rule: Rule, medium: &str, frames: u64, every: u64, n_walkers: usize, n_items: usize, w: i32, h: i32) -> (bool, (f32, f32)) {
    let flat = medium == "flat";
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    let stone = world.materials.id_of("stone").expect("stone");
    let brood = world.materials.id_of("brood").expect("brood");
    let k = (arg("k1").unwrap_or(0.1), arg("k2").unwrap_or(0.3));
    for y in 0..h {
        for x in 0..w {
            if x < 2 || y < 2 || x >= w - 2 || y >= h - 2 {
                world.set(x, y, Cell::new(stone, 0).with_attached(true));
            }
        }
    }
    let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xb00d);
    let open = |wd: &World, x: i32, y: i32| wd.in_bounds(x, y) && wd.is_empty(x, y);
    // **`body=a,b,c`: brood that takes up room** (owner, 2026-10-15, card
    // "Bigger brood"). Each size is a square `a`/`b`/`c` cells on a side
    // (`1,2,3`: area 1, 4, 9). Franks's whole reading of the rings is space
    // per item; with every item one cell the room a size needs rounds to
    // whole cells, and the room rule managed 3/16 at best. Top-down
    // (`medium=flat`), or a side view (`medium=game`) where a body falls
    // whole: the game's powder step would tear it into cells.
    let body: [i32; 3] = arg::<String>("body").map_or([1, 1, 1], |v| {
        let p: Vec<i32> = v.split(',').map(|x| x.parse().expect("body=a,b,c")).collect();
        [p[0], p[1], p[2]]
    });
    let bodies = body != [1, 1, 1];
    let layer = arg::<u32>("layer") == Some(1);
    let mut owner = vec![usize::MAX; (w * h) as usize];
    let mut things: Vec<Option<(i32, i32, u8)>> = Vec::new();
    if bodies {
        assert!(medium != "rest" && arg::<u64>("scatter").is_none(), "body= runs flat or game, with no scatter");
        assert!(matches!(rule, Rule::Room | Rule::Off), "body= runs room or off");
        for size in 0..3u8 {
            let mut placed = 0;
            while placed < n_items {
                let (x, y) = (2 + rng.below((w - 4) as u32) as i32, 2 + rng.below((h - 4) as u32) as i32);
                let fp = footprint(body, x, y, size);
                if fp.iter().all(|&(a, b)| open(&world, a, b)) {
                    for &(a, b) in &fp {
                        world.set(a, b, Cell::new(brood, size));
                        owner[(b * w + a) as usize] = things.len();
                    }
                    things.push(Some((x, y, size)));
                    placed += 1;
                }
            }
        }
    }
    for size in 0..3u8 {
        let mut placed = 0;
        while placed < n_items && !bodies {
            let (x, y) = (2 + rng.below((w - 4) as u32) as i32, 2 + rng.below((h - 4) as u32) as i32);
            if open(&world, x, y) {
                world.set(x, y, Cell::new(brood, size));
                placed += 1;
            }
        }
    }
    let mut walkers: Vec<Walker> = Vec::new();
    while walkers.len() < n_walkers {
        let (x, y) = (2 + rng.below((w - 4) as u32) as i32, 2 + rng.below((h - 4) as u32) as i32);
        if open(&world, x, y) {
            walkers.push(Walker { x, y, dir: rng.below(8) as usize, held: None, wait: 0, back: 0 });
        }
    }
    let footing =
        |wd: &World, x: i32, y: i32| DIRS.iter().any(|&(dx, dy)| wd.in_bounds(x + dx, y + dy) && !wd.is_empty(x + dx, y + dy));
    let (mut picks, mut drops) = (0u64, 0u64);
    let mut result = (false, (0.0, 0.0));
    // **`scatter=<frame>`: Franks's emigration, as a scatter.** Every item,
    // held ones included, is put back at random open cells, so the order
    // has to re-form from nothing. `formed` is the first scored frame that
    // passes, from the start and again after the scatter.
    let scatter: Option<u64> = arg("scatter");
    let (mut formed, mut reformed): (Option<u64>, Option<u64>) = (None, None);
    for f in 0..=frames {
        if Some(f) == scatter {
            for y in 0..h {
                for x in 0..w {
                    if world.get(x, y).material == brood {
                        world.set(x, y, Cell::EMPTY);
                    }
                }
            }
            for wk in walkers.iter_mut() {
                wk.held = None;
            }
            for size in 0..3u8 {
                let mut placed = 0;
                while placed < n_items {
                    let (x, y) = (2 + rng.below((w - 4) as u32) as i32, 2 + rng.below((h - 4) as u32) as i32);
                    if open(&world, x, y) && !walkers.iter().any(|o| (o.x, o.y) == (x, y)) {
                        world.set(x, y, Cell::new(brood, size));
                        placed += 1;
                    }
                }
            }
            println!("seed {seed} f {f:>6}: SCATTERED");
        }
        if f > 0 {
            if bodies && medium == "game" {
                // **A body falls whole**: an item with nothing under any of
                // its bottom cells drops a row, lowest items first. Nothing
                // slides off a heap (the powder's slide would tear a 3x3
                // larva into nine cells); a body rests on whatever is under
                // any one of its cells.
                let mut order: Vec<usize> = (0..things.len()).filter(|&j| things[j].is_some()).collect();
                order.sort_by_key(|&j| std::cmp::Reverse(things[j].map_or(0, |t| t.1)));
                for j in order {
                    let (ax, ay, sz) = things[j].expect("filtered");
                    let b = body[usize::from(sz)];
                    if (0..b).all(|dx| open(&world, ax + dx, ay + b)) {
                        for dx in 0..b {
                            world.set(ax + dx, ay, Cell::EMPTY);
                            owner[(ay * w + ax + dx) as usize] = usize::MAX;
                            world.set(ax + dx, ay + b, Cell::new(brood, sz));
                            owner[((ay + b) * w + ax + dx) as usize] = j;
                        }
                        for dy in 1..b {
                            for dx in 0..b {
                                owner[((ay + dy) * w + ax + dx) as usize] = j;
                            }
                        }
                        things[j] = Some((ax, ay + 1, sz));
                    }
                }
            } else if medium == "game" {
                parallel::step(&mut world);
            } else if medium == "rest" {
                // An item with nothing under it drops straight down; nothing
                // slides off a heap, so an item stays where it was put.
                for y in (2..h - 3).rev() {
                    for x in 2..w - 2 {
                        if world.get(x, y).material == brood && world.is_empty(x, y + 1) {
                            let c = world.get(x, y);
                            world.set(x, y, Cell::EMPTY);
                            world.set(x, y + 1, c);
                        }
                    }
                }
            }
            for i in 0..walkers.len() {
                if !flat && !footing(&world, walkers[i].x, walkers[i].y) {
                    if open(&world, walkers[i].x, walkers[i].y + 1) {
                        walkers[i].y += 1;
                    }
                    continue;
                }
                let (x, y) = (walkers[i].x, walkers[i].y);
                // **`pullback`** (Wilson, Melhuish, Sendova-Franks et al.
                // 2004, *Autonomous Robots* 17:115-136, "extended differential
                // pullback"): an empty walker picks up an item it touches
                // by the all-brood share round it (below); a laden one that touches another item
                // puts a small one down there, and otherwise backs away
                // `pull=` cells for its size (0,3,5: their 0, 2.5 and 5 puck
                // diameters for three types) and puts it down where it
                // stops. It counts nothing and measures no distance to any
                // item: the size sets how far back it goes.
                let pull: Vec<u32> = arg::<String>("pull")
                    .unwrap_or_else(|| "0,3,5".into())
                    .split(',')
                    .map(|v| v.parse().expect("pull=a,b,c"))
                    .collect();
                if bodies && walkers[i].wait == 0 {
                    let chance_of = |owner: &[usize], things: &[Option<(i32, i32, u8)>], fp: &[(i32, i32)], s: u8, skip: usize, drop: bool| -> f32 {
                        if rule == Rule::Off {
                            return 0.1;
                        }
                        let sc = body_score(owner, things, w, h, body, fp, s, skip);
                        if drop {
                            sc
                        } else {
                            1.0 - sc
                        }
                    };
                    match walkers[i].held {
                        None => {
                            let mut cands: Vec<usize> = DIRS
                                .iter()
                                .filter(|&&(dx, dy)| world.in_bounds(x + dx, y + dy))
                                .map(|&(dx, dy)| owner[((y + dy) * w + x + dx) as usize])
                                .filter(|&j| j != usize::MAX)
                                .collect();
                            cands.sort_unstable();
                            cands.dedup();
                            if !cands.is_empty() {
                                let j = cands[rng.below(cands.len() as u32) as usize];
                                let (ax, ay, sz) = things[j].expect("an owned cell's item is on the board");
                                let fp = footprint(body, ax, ay, sz);
                                if rng.unit_f32() < chance_of(&owner, &things, &fp, sz, j, false) {
                                    for &(a, b) in &fp {
                                        world.set(a, b, Cell::EMPTY);
                                        owner[(b * w + a) as usize] = usize::MAX;
                                    }
                                    things[j] = None;
                                    walkers[i].held = Some(sz);
                                    walkers[i].wait = DELAY;
                                    picks += 1;
                                }
                            }
                        }
                        Some(sz) => {
                            // Any placement of the body that covers a cell
                            // beside the walker, every cell of it open.
                            let b = body[usize::from(sz)];
                            let mut anchors: Vec<(i32, i32)> = Vec::new();
                            for &(dx, dy) in &DIRS {
                                for oy in 0..b {
                                    for ox in 0..b {
                                        let (ax, ay) = (x + dx - ox, y + dy - oy);
                                        let fp = footprint(body, ax, ay, sz);
                                        // `layer=1`: one layer on the chamber floor, as a
                                        // Leptothorax nest's brood lies; nothing goes on top
                                        // of other brood.
                                        let floor_only = layer && medium == "game" && ay + b != h - 2;
                                        if !floor_only && fp.iter().all(|&(a, c)| open(&world, a, c) && !walkers.iter().any(|o| (o.x, o.y) == (a, c))) {
                                            anchors.push((ax, ay));
                                        }
                                    }
                                }
                            }
                            anchors.sort_unstable();
                            anchors.dedup();
                            if !anchors.is_empty() {
                                let (ax, ay) = anchors[rng.below(anchors.len() as u32) as usize];
                                let fp = footprint(body, ax, ay, sz);
                                if rng.unit_f32() < chance_of(&owner, &things, &fp, sz, usize::MAX, true) {
                                    let id = things.iter().position(Option::is_none).unwrap_or(things.len());
                                    if id == things.len() {
                                        things.push(None);
                                    }
                                    for &(a, c) in &fp {
                                        world.set(a, c, Cell::new(brood, sz));
                                        owner[(c * w + a) as usize] = id;
                                    }
                                    things[id] = Some((ax, ay, sz));
                                    walkers[i].held = None;
                                    walkers[i].wait = DELAY;
                                    drops += 1;
                                }
                            }
                        }
                    }
                } else if bodies {
                    walkers[i].wait -= 1;
                } else if rule == Rule::Pullback && walkers[i].wait == 0 {
                    let touching = DIRS.iter().any(|&(dx, dy)| size_at(&world, brood, x + dx, y + dy).is_some());
                    let spots: Vec<(i32, i32)> = DIRS
                        .iter()
                        .map(|&(dx, dy)| (x + dx, y + dy))
                        .filter(|&(px, py)| open(&world, px, py) && !walkers.iter().any(|o| (o.x, o.y) == (px, py)))
                        .collect();
                    match walkers[i].held {
                        Some(s) if walkers[i].back > 0 => {
                            walkers[i].back -= 1;
                            if walkers[i].back == 0 && !spots.is_empty() {
                                let (px, py) = spots[rng.below(spots.len() as u32) as usize];
                                world.set(px, py, Cell::new(brood, s));
                                walkers[i].held = None;
                                walkers[i].wait = DELAY;
                                drops += 1;
                            } else if walkers[i].back == 0 {
                                walkers[i].back = 1;
                            }
                        }
                        Some(s) if touching => {
                            if pull[usize::from(s)] == 0 {
                                if !spots.is_empty() {
                                    let (px, py) = spots[rng.below(spots.len() as u32) as usize];
                                    world.set(px, py, Cell::new(brood, s));
                                    walkers[i].held = None;
                                    walkers[i].wait = DELAY;
                                    drops += 1;
                                }
                            } else {
                                walkers[i].back = pull[usize::from(s)];
                                walkers[i].dir = (walkers[i].dir + 4) % 8;
                            }
                        }
                        None => {
                            let cands: Vec<(i32, i32, u8)> = DIRS
                                .iter()
                                .filter_map(|&(dx, dy)| size_at(&world, brood, x + dx, y + dy).map(|s| (x + dx, y + dy, s)))
                                .collect();
                            if !cands.is_empty() {
                                let (ix, iy, s) = cands[rng.below(cands.len() as u32) as usize];
                                // Picked up by the all-brood share round the
                                // item (`brood`'s pick, Deneubourg's
                                // `(k1/(k1+f))^2`), so a loose item goes and a
                                // packed one stays -- what makes Beckers's and
                                // Wilson's pucks cluster at all. A flat
                                // `pick=` instead: no cluster formed (8/8
                                // seeds, largest 13-41% of items).
                                let p = arg::<f32>("pick")
                                    .unwrap_or_else(|| chance(&world, brood, Rule::Brood, s, (ix, iy), (ix, iy), false, k));
                                if rng.unit_f32() < p {
                                    world.set(ix, iy, Cell::EMPTY);
                                    walkers[i].held = Some(s);
                                    walkers[i].wait = DELAY;
                                    picks += 1;
                                }
                            }
                        }
                        _ => {}
                    }
                } else if rule == Rule::Pullback || walkers[i].wait > 0 {
                    walkers[i].wait -= 1;
                } else {
                    match walkers[i].held {
                        None => {
                            let cands: Vec<(i32, i32, u8)> = DIRS
                                .iter()
                                .filter_map(|&(dx, dy)| size_at(&world, brood, x + dx, y + dy).map(|s| (x + dx, y + dy, s)))
                                .collect();
                            if !cands.is_empty() {
                                let (ix, iy, s) = cands[rng.below(cands.len() as u32) as usize];
                                if rng.unit_f32() < chance(&world, brood, rule, s, (ix, iy), (ix, iy), false, k) {
                                    world.set(ix, iy, Cell::EMPTY);
                                    walkers[i].held = Some(s);
                                    walkers[i].wait = DELAY;
                                    picks += 1;
                                }
                            }
                        }
                        Some(s) => {
                            let spots: Vec<(i32, i32)> = DIRS
                                .iter()
                                .map(|&(dx, dy)| (x + dx, y + dy))
                                .filter(|&(px, py)| open(&world, px, py) && !walkers.iter().any(|o| (o.x, o.y) == (px, py)))
                                .collect();
                            if !spots.is_empty() {
                                let (px, py) = spots[rng.below(spots.len() as u32) as usize];
                                if rng.unit_f32() < chance(&world, brood, rule, s, (px, py), (px, py), true, k) {
                                    world.set(px, py, Cell::new(brood, s));
                                    walkers[i].held = None;
                                    walkers[i].wait = DELAY;
                                    drops += 1;
                                }
                            }
                        }
                    }
                }
                // Backing away goes straight.
                if walkers[i].back == 0 && rng.below(8) == 0 {
                    walkers[i].dir = (walkers[i].dir + if rng.flip() { 1 } else { 7 }) % 8;
                }
                // **Which way a blocked walker tries first is drawn each step.**
                // A fixed order (clockwise first, as sortbox has) gives every
                // walker the same handedness along a wall: they circulate one
                // way and carry brood into one corner. On `medium=rest` the
                // room rule's cluster ended in the bottom-left corner on 8 of
                // 8 seeds, the brood rule's in the bottom-right on 7 of 8.
                let order = if rng.flip() { [0, 1, 7, 2, 6, 3, 5, 4] } else { [0, 7, 1, 6, 2, 5, 3, 4] };
                let mut moved = false;
                for turn in order {
                    let d = (walkers[i].dir + turn) % 8;
                    let (nx, ny) = (x + DIRS[d].0, y + DIRS[d].1);
                    if open(&world, nx, ny) && (flat || footing(&world, nx, ny)) {
                        (walkers[i].x, walkers[i].y, walkers[i].dir) = (nx, ny, d);
                        moved = true;
                        break;
                    }
                }
                if !moved {
                    walkers[i].dir = rng.below(8) as usize;
                }
            }
            for wk in walkers.iter_mut() {
                while !open(&world, wk.x, wk.y) && wk.y > 2 {
                    wk.y -= 1;
                }
            }
        }
        if f % every == 0 {
            let held = walkers.iter().filter(|w| w.held.is_some()).count();
            let pts: Vec<(f32, f32, u8, i32)> = things
                .iter()
                .flatten()
                .map(|&(ax, ay, sz)| {
                    let b = body[usize::from(sz)];
                    (ax as f32 + (b - 1) as f32 / 2.0, ay as f32 + (b - 1) as f32 / 2.0, sz, b)
                })
                .collect();
            result = score(&world, brood, bodies.then_some(&pts[..]), n_items, seed, f, held, picks, drops);
            if result.0 {
                match scatter {
                    Some(sc) if f > sc => reformed = reformed.or(Some(f - sc)),
                    _ => formed = formed.or(Some(f)),
                }
            }
            if let Some(dir) = arg::<String>("png") {
                let _ = std::fs::create_dir_all(&dir);
                let _ = draw(&world, brood, stone, &walkers, w, h).save(format!("{dir}/s{seed}_f{f:06}.png"));
            }
        }
    }
    let show = |v: Option<u64>| v.map_or("never".to_string(), |f| f.to_string());
    println!(
        "seed {seed}: first pass at frame {}{}",
        show(formed),
        scatter.map_or(String::new(), |_| format!("; after the scatter, again {} frames later", show(reformed)))
    );
    result
}

/// The cells of a `body=` item of `size` anchored at its top-left `(x, y)`.
fn footprint(body: [i32; 3], x: i32, y: i32, size: u8) -> Vec<(i32, i32)> {
    let b = body[usize::from(size)];
    (0..b).flat_map(|dy| (0..b).map(move |dx| (x + dx, y + dy))).collect()
}

/// **The room rule for items with bodies**: Don & Amos's score, counted
/// per item and measured **edge to edge** as theirs is -- the gap from the
/// nearest cell of the placement `fp` to the nearest cell of each other
/// item (0 touching, 0.41 at a corner). An item of width `b` needs a gap of
/// `room= x (b - 1)` (default 1: none for a one-cell item, one clear cell
/// for a 2x2, two for a 3x3) and counts its neighbours within `bonusm= x
/// b` (default 4); `penalty -50`, `bonus 0.1`, clamped to 0..1.
#[allow(clippy::too_many_arguments)]
fn body_score(owner: &[usize], things: &[Option<(i32, i32, u8)>], w: i32, h: i32, body: [i32; 3], fp: &[(i32, i32)], s: u8, skip: usize) -> f32 {
    let b = body[usize::from(s)] as f32;
    // `penm=`: Don & Amos's own form instead, `penalty_m x size` with the
    // body's width as its size (theirs is 0.4), so a one-cell item needs
    // room too.
    let room = match arg::<f32>("penm") {
        Some(m) => m * b,
        None => arg::<f32>("room").unwrap_or(1.0) * (b - 1.0),
    };
    let reach = arg::<f32>("bonusm").unwrap_or(BONUS_M) * b;
    let r = reach.ceil() as i32 + 1;
    let (x0, x1) = fp.iter().fold((i32::MAX, i32::MIN), |(a, c), p| (a.min(p.0), c.max(p.0)));
    let (y0, y1) = fp.iter().fold((i32::MAX, i32::MIN), |(a, c), p| (a.min(p.1), c.max(p.1)));
    let mut gap: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
    for y in (y0 - r).max(0)..=(y1 + r).min(h - 1) {
        for x in (x0 - r).max(0)..=(x1 + r).min(w - 1) {
            let j = owner[(y * w + x) as usize];
            if j == usize::MAX || j == skip || things[j].is_none() {
                continue;
            }
            let g = fp
                .iter()
                .map(|&(a, c)| ((((a - x).pow(2) + (c - y).pow(2)) as f32).sqrt() - 1.0).max(0.0))
                .fold(f32::MAX, f32::min);
            let e = gap.entry(j).or_insert(f32::MAX);
            *e = e.min(g);
        }
    }
    let score: f32 = gap.values().map(|&g| if g < room { PENALTY } else if g <= reach { BONUS } else { 0.0 }).sum();
    score.clamp(0.0, 1.0)
}

/// The largest cluster's sorting, printed; returns (pass, ratios).
#[allow(clippy::too_many_arguments)]
fn score(world: &World, brood: MaterialId, bodies: Option<&[(f32, f32, u8, i32)]>, n_items: usize, seed: u64, f: u64, held: usize, picks: u64, drops: u64) -> (bool, (f32, f32)) {
    // One point per item: its centre and body width. A one-cell item is its
    // cell; `body=` items are passed in (their cells would weight a 3x3 item
    // nine times).
    let items: Vec<(f32, f32, u8, i32)> = match bodies {
        Some(b) => b.to_vec(),
        None => {
            let b = world.bounds().expect("bounds");
            (b.min_y..=b.max_y)
                .flat_map(|y| (b.min_x..=b.max_x).map(move |x| (x, y)))
                .filter_map(|(x, y)| size_at(world, brood, x, y).map(|s| (x as f32, y as f32, s, 1)))
                .collect()
        }
    };
    // **Joined within `join=` cells (4)**, edge to edge: the paper's rings
    // space out toward the edge (Franks's tile area grows with distance), so
    // a cluster of touching items cuts the spaced outer ring off and scores
    // it as missing -- which the first runs here did (2 cells: large 0-2 of
    // 25). Chebyshev between centres less half of each body beyond its own
    // cell, so for one-cell items it is the old `join`-cell window exactly.
    let join = arg::<i32>("join").unwrap_or(4) as f32;
    let near = |a: &(f32, f32, u8, i32), b: &(f32, f32, u8, i32)| {
        (a.0 - b.0).abs().max((a.1 - b.1).abs()) <= join + (a.3 + b.3) as f32 / 2.0 - 1.0
    };
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
            let a = items[group[k]];
            for j in 0..items.len() {
                if label[j] == usize::MAX && near(&a, &items[j]) {
                    label[j] = start;
                    group.push(j);
                }
            }
            k += 1;
        }
        if group.len() > best.len() {
            best = group;
        }
    }
    let cl: Vec<(f32, f32, u8)> = best.iter().map(|&i| (items[i].0, items[i].1, items[i].2)).collect();
    let n = cl.len().max(1) as f32;
    let (cx, cy) = (cl.iter().map(|c| c.0).sum::<f32>() / n, cl.iter().map(|c| c.1).sum::<f32>() / n);
    // **`along=floor`: the paper's rings, cut by a vertical slice.** A flat
    // brood floor seen edge-on is one row, so "further from the centre"
    // becomes "further along the floor from the pile's middle": the
    // distance counts columns only. The default (`along=plane`) is the
    // paper's own, for the top-down medium.
    let floor = arg::<String>("along").as_deref() == Some("floor");
    let dist: Vec<f32> = cl
        .iter()
        .map(|c| if floor { (c.0 - cx).abs() } else { ((c.0 - cx).powi(2) + (c.1 - cy).powi(2)).sqrt() })
        .collect();
    let mean_by = |labels: &[u8], s: u8| -> f32 {
        let (sum, cnt) = labels.iter().zip(&dist).filter(|(l, _)| **l == s).fold((0.0, 0), |(a, c), (_, d)| (a + d, c + 1));
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
    // The null: the same cells, the sizes shuffled among them.
    let mut rng = Rng::new(seed ^ f ^ 0x57f_u64.wrapping_mul(0x9E37));
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
    // **The paper's bar** (2026-10-15; `antsort` has the reasoning): one
    // cluster and the order ratios inside the published span with headroom
    // -- medium 1.2-2.0, large 1.7-3.2. Beating label shuffles (printed as
    // "beaten") only says large lies further out than chance: it passed 13
    // of 16 top-down seeds whose large ring sat at 2.3-4.8.
    let beaten = share >= 0.8 && counts.iter().all(|&c| c >= 10) && rl > p95;
    let pass = share >= 0.8 && counts.iter().all(|&c| c >= 10) && (1.2..=2.0).contains(&rm) && (1.7..=3.2).contains(&rl) && rm < rl;
    println!(
        "seed {seed} f {f:>6}: cluster {:.2} at ({cx:.0},{cy:.0}) of items {:?} | d small {:.1} medium {:.1} large {:.1} | ratios {rm:.2} {rl:.2} (shuffle p95 {p95:.2}{}) | {} | held {held} picks {picks} drops {drops}",
        share,
        counts,
        d[0],
        d[1],
        d[2],
        if beaten { ", beaten" } else { "" },
        if pass { "PASS" } else { "fail" }
    );
    (pass, (rm, rl))
}

/// Small white, medium cream, large tan-brown; walkers red, laden yellow.
fn draw(world: &World, brood: MaterialId, stone: MaterialId, walkers: &[Walker], w: i32, h: i32) -> image::RgbaImage {
    const PX: u32 = 4;
    let mut img = image::RgbaImage::from_pixel(w as u32 * PX, h as u32 * PX, image::Rgba([24, 22, 28, 255]));
    let mut put = |x: i32, y: i32, c: [u8; 4]| {
        for dy in 0..PX {
            for dx in 0..PX {
                img.put_pixel(x as u32 * PX + dx, y as u32 * PX + dy, image::Rgba(c));
            }
        }
    };
    for y in 0..h {
        for x in 0..w {
            let c = world.get(x, y);
            if c.material == stone {
                put(x, y, [95, 95, 100, 255]);
            } else if c.material == brood {
                put(x, y, [[250, 250, 250, 255], [230, 200, 120, 255], [170, 90, 40, 255]][c.shade.min(2) as usize]);
            }
        }
    }
    for wk in walkers {
        put(wk.x, wk.y, if wk.held.is_some() { [255, 220, 40, 255] } else { [220, 50, 50, 255] });
    }
    img
}
