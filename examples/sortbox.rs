//! **Can this world carry a sorted pile?** -- a sandbox for the first of
//! the substrate tests in PR 701's follow-up
//! (`/mnt/project-files/emergence-sandbox/plan-2026-10-14.md`, E1).
//!
//! A bare stone box, two kinds of loose item (`brood` and `crumbs`), and
//! walkers that are **not the shipped ant**: no brain, no energy, no
//! breeding, no chooser, no pulls. Each walker runs one published local
//! rule, Deneubourg et al. 1991's: an unladen walker beside an item picks it
//! up with `(k1/(k1+f))^2`, a laden one drops its item into an open cell
//! beside it with `(f/(k2+f))^2`, `f` the share of the 24 cells round the
//! item (or the walker) holding items of the same kind. `k1`, `k2` default
//! to the shipped `STOREROOM` `pile` part's 0.1 and 0.3. Nothing else.
//!
//! **The arms change the medium, never the rule** (`arm=`):
//! - `flat` -- items stay where they are put and walkers roam every open
//!   cell. This is the textbook top-down setting, and it is the **positive
//!   control**: if the rule does not sort here the harness is wrong.
//! - `fall` -- the game's physics runs (`parallel::step`), so a dropped item
//!   falls and piles at its own repose. Walkers still roam freely.
//! - `game` -- items fall, and a walker may only stand in an open cell
//!   touching something (wall, floor, item), else it falls a cell, as the
//!   shipped ant does.
//!
//! Walkers are not cells (they are drawn, not placed), so they never block
//! an item or hold one up. Determinism: one `Rng` per seed, walkers stepped
//! in a fixed order.
//!
//! Read per stop: per kind, piles (8-connected groups of that kind), the
//! largest pile's share, and **mixing** -- of every item's item neighbours,
//! the share that are the *other* kind (0 = fully sorted; ~0.5 = random).
//! `apart` is the distance in cells between the centres of the two kinds'
//! largest piles (the box is 160 wide). `items` and `held` must sum to the starting count (printed as `lost`).
//! `png=DIR` writes a frame per stop and `gif=PATH` an animation.
//!
//! ```text
//! cargo run --release --example sortbox -- arm=flat seeds=4
//! cargo run --release --example sortbox -- arm=game seed=1 gif=out.gif
//! ```

use pixel_physics::sim::parallel;
use pixel_physics::sim::rng::Rng;
use pixel_physics::sim::{Cell, MaterialId, Rect, World};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}="))
            .map(|v| v.parse().ok().expect("parses"))
    })
}

#[derive(Clone, Copy, PartialEq)]
enum Arm {
    Flat,
    Fall,
    Game,
}

struct Walker {
    x: i32,
    y: i32,
    /// Heading as one of the eight neighbour offsets, kept with a small
    /// turn chance so a walker covers ground rather than jittering in place.
    dir: usize,
    held: Option<MaterialId>,
}

const DIRS: [(i32, i32); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];

struct Box_ {
    w: i32,
    h: i32,
    wall: i32,
}

fn main() {
    let arm = match arg::<String>("arm").as_deref().unwrap_or("flat") {
        "flat" => Arm::Flat,
        "fall" => Arm::Fall,
        "game" => Arm::Game,
        a => panic!("arm={a}: flat, fall or game"),
    };
    let seeds: Vec<u64> = match arg::<u64>("seed") {
        Some(s) => vec![s],
        None => (1..=arg::<u64>("seeds").unwrap_or(4)).collect(),
    };
    let frames: u64 = arg("frames").unwrap_or(40_000);
    let every: u64 = arg("every").unwrap_or(5_000);
    let n_walkers: usize = arg("walkers").unwrap_or(60);
    let n_items: usize = arg("items").unwrap_or(150);
    let k1: f32 = arg("k1").unwrap_or(0.1);
    let k2: f32 = arg("k2").unwrap_or(0.3);
    let width: i32 = arg("width").unwrap_or(160);
    let height: i32 = arg("height").unwrap_or(100);
    let png: Option<String> = arg("png");
    let gif: Option<String> = arg("gif");
    let gif_every: u64 = arg("gifevery").unwrap_or(500);
    let kinds: String = arg("kinds").unwrap_or_else(|| "brood,crumbs".to_string());
    let layout: String = arg("layout").unwrap_or_else(|| "box".to_string());
    println!(
        "sortbox: arm={} frames={frames} walkers={n_walkers} items={n_items} per kind k1={k1} k2={k2} box={width}x{height} kinds={kinds}",
        match arm {
            Arm::Flat => "flat",
            Arm::Fall => "fall",
            Arm::Game => "game",
        }
    );
    for seed in seeds {
        run(
            seed,
            arm,
            frames,
            every,
            n_walkers,
            n_items,
            k1,
            k2,
            Box_ {
                w: width,
                h: height,
                wall: 3,
            },
            &kinds,
            &layout,
            png.as_deref(),
            gif.as_deref(),
            gif_every,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    seed: u64,
    arm: Arm,
    frames: u64,
    every: u64,
    n_walkers: usize,
    n_items: usize,
    k1: f32,
    k2: f32,
    b: Box_,
    kinds: &str,
    layout: &str,
    png: Option<&str>,
    gif: Option<&str>,
    gif_every: u64,
) {
    let mut world = World::new(Rect::new(0, 0, b.w - 1, b.h - 1));
    let stone = world.materials.id_of("stone").expect("stone");
    let kind_ids: Vec<MaterialId> = kinds
        .split(',')
        .map(|k| world.materials.id_of(k).unwrap_or_else(|| panic!("no material {k}")))
        .collect();
    for y in 0..b.h {
        for x in 0..b.w {
            if x < b.wall || y < b.wall || x >= b.w - b.wall || y >= b.h - b.wall {
                world.set(x, y, Cell::new(stone, 0));
            }
        }
    }
    let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x5eed);
    // `layout=nest`: solid ground with three chambers at different depths,
    // joined by tunnels two cells high -- the question is whether the rule
    // puts the two kinds in different rooms when there are rooms to use.
    let rooms = nest_rooms(&b);
    if layout == "nest" {
        for y in b.wall..b.h - b.wall {
            for x in b.wall..b.w - b.wall {
                if !rooms.iter().any(|r| r.contains(x, y)) {
                    world.set(x, y, Cell::new(stone, 0));
                }
            }
        }
    } else {
        assert_eq!(layout, "box", "layout=box or nest");
    }
    let open = |w: &World, x: i32, y: i32| w.in_bounds(x, y) && w.is_empty(x, y);
    // Scatter the items over the open interior.
    for &k in &kind_ids {
        let mut placed = 0;
        while placed < n_items {
            let x = b.wall + rng.below((b.w - 2 * b.wall) as u32) as i32;
            let y = b.wall + rng.below((b.h - 2 * b.wall) as u32) as i32;
            if open(&world, x, y) {
                world.set(x, y, Cell::new(k, 0));
                placed += 1;
            }
        }
    }
    let total = n_items * kind_ids.len();
    let mut walkers: Vec<Walker> = Vec::new();
    while walkers.len() < n_walkers {
        let x = b.wall + rng.below((b.w - 2 * b.wall) as u32) as i32;
        let y = b.wall + rng.below((b.h - 2 * b.wall) as u32) as i32;
        if open(&world, x, y) {
            walkers.push(Walker {
                x,
                y,
                dir: rng.below(8) as usize,
                held: None,
            });
        }
    }
    let item_kind = |w: &World, x: i32, y: i32| -> Option<MaterialId> {
        if !w.in_bounds(x, y) {
            return None;
        }
        let m = w.get(x, y).material;
        kind_ids.contains(&m).then_some(m)
    };
    // `f`: the share of the 24 cells round (x, y) holding kind k.
    let share = |w: &World, x: i32, y: i32, k: MaterialId| -> f32 {
        let mut n = 0;
        for dy in -2..=2 {
            for dx in -2..=2 {
                if (dx, dy) != (0, 0) && item_kind(w, x + dx, y + dy) == Some(k) {
                    n += 1;
                }
            }
        }
        n as f32 / 24.0
    };
    let footing = |w: &World, x: i32, y: i32| {
        DIRS.iter()
            .any(|&(dx, dy)| w.in_bounds(x + dx, y + dy) && !w.is_empty(x + dx, y + dy))
    };
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    let mut picks = 0u64;
    let mut drops = 0u64;
    for f in 0..=frames {
        if f > 0 {
            if arm != Arm::Flat {
                parallel::step(&mut world);
            }
            for i in 0..walkers.len() {
                // The game arm's walker falls when nothing touches it.
                if arm == Arm::Game && !footing(&world, walkers[i].x, walkers[i].y) {
                    if open(&world, walkers[i].x, walkers[i].y + 1) {
                        walkers[i].y += 1;
                    }
                    continue;
                }
                // Pick up or drop, then step.
                let (x, y) = (walkers[i].x, walkers[i].y);
                match walkers[i].held {
                    None => {
                        let mut cands: Vec<(i32, i32, MaterialId)> = Vec::new();
                        for &(dx, dy) in &DIRS {
                            if let Some(k) = item_kind(&world, x + dx, y + dy) {
                                cands.push((x + dx, y + dy, k));
                            }
                        }
                        if !cands.is_empty() {
                            let (ix, iy, k) = cands[rng.below(cands.len() as u32) as usize];
                            let fk = share(&world, ix, iy, k);
                            let p = (k1 / (k1 + fk)).powi(2);
                            if rng.unit_f32() < p {
                                world.set(ix, iy, Cell::EMPTY);
                                walkers[i].held = Some(k);
                                picks += 1;
                            }
                        }
                    }
                    Some(k) => {
                        let fk = share(&world, x, y, k);
                        let p = (fk / (k2 + fk)).powi(2);
                        if rng.unit_f32() < p {
                            let spots: Vec<(i32, i32)> = DIRS
                                .iter()
                                .map(|&(dx, dy)| (x + dx, y + dy))
                                .filter(|&(px, py)| {
                                    open(&world, px, py) && !walkers.iter().any(|o| (o.x, o.y) == (px, py))
                                })
                                .collect();
                            if !spots.is_empty() {
                                let (px, py) = spots[rng.below(spots.len() as u32) as usize];
                                world.set(px, py, Cell::new(k, 0));
                                walkers[i].held = None;
                                drops += 1;
                            }
                        }
                    }
                }
                // Persistent random walk over open cells.
                if rng.below(8) == 0 {
                    let turn = if rng.flip() { 1 } else { 7 };
                    walkers[i].dir = (walkers[i].dir + turn) % 8;
                }
                let mut moved = false;
                for attempt in 0..8 {
                    let d = (walkers[i].dir + [0, 1, 7, 2, 6, 3, 5, 4][attempt]) % 8;
                    let (nx, ny) = (x + DIRS[d].0, y + DIRS[d].1);
                    let ok = open(&world, nx, ny) && (arm != Arm::Game || footing(&world, nx, ny));
                    if ok {
                        walkers[i].x = nx;
                        walkers[i].y = ny;
                        walkers[i].dir = d;
                        moved = true;
                        break;
                    }
                }
                if !moved {
                    walkers[i].dir = rng.below(8) as usize;
                }
            }
            // A walker buried by a falling item is put on the nearest open cell above it.
            for wk in walkers.iter_mut() {
                while !open(&world, wk.x, wk.y) && wk.y > b.wall {
                    wk.y -= 1;
                }
            }
        }
        if gif.is_some() && f % gif_every == 0 {
            shots.push(draw(&world, &b, &walkers, &kind_ids, stone));
        }
        if f % every == 0 {
            let held = walkers.iter().filter(|w| w.held.is_some()).count();
            let (items, per_kind, mixing) = census(&world, &kind_ids, &b);
            let piles: Vec<String> = per_kind
                .iter()
                .map(|(n, piles, big)| {
                    format!(
                        "{n} items/{piles} piles/largest {:.0}%",
                        100.0 * *big as f32 / (*n).max(1) as f32
                    )
                })
                .collect();
            println!(
                "seed {seed} f {f:>6}: {} | mixing {:.2} apart {:.0} | held {held} lost {} | picks {picks} drops {drops}",
                piles.join(" ; "),
                mixing,
                apart(&world, &kind_ids, &b),
                total as i64 - (items + held) as i64
            );
            if layout == "nest" {
                let mut line = String::new();
                for (ri, r) in rooms.iter().take(3).enumerate() {
                    let counts: Vec<usize> = kind_ids
                        .iter()
                        .map(|&k| {
                            (r.x0..=r.x1)
                                .flat_map(|x| (r.y0..=r.y1).map(move |y| (x, y)))
                                .filter(|&(x, y)| world.get(x, y).material == k)
                                .count()
                        })
                        .collect();
                    line.push_str(&format!(" room{ri} {:?}", counts));
                }
                println!("        rooms (brood, crumbs):{line}");
            }
            if let Some(dir) = png {
                let _ = std::fs::create_dir_all(dir);
                let _ = draw(&world, &b, &walkers, &kind_ids, stone).save(format!("{dir}/s{seed}_f{f:06}.png"));
            }
        }
    }
    if let Some(path) = gif {
        let delay = image::Delay::from_numer_denom_ms(120, 1);
        let frames: Vec<image::Frame> = shots
            .into_iter()
            .map(|img| image::Frame::from_parts(img, 0, 0, delay))
            .collect();
        let file = std::fs::File::create(path).expect("gif path");
        let mut enc = image::codecs::gif::GifEncoder::new(file);
        enc.set_repeat(image::codecs::gif::Repeat::Infinite).expect("repeat");
        enc.encode_frames(frames).expect("encode");
        println!("sortbox: wrote {path}");
    }
}

/// Items, per kind (count, piles, largest pile), and mixing.
fn census(w: &World, kinds: &[MaterialId], b: &Box_) -> (usize, Vec<(usize, usize, usize)>, f32) {
    let mut per = Vec::new();
    let mut items = 0;
    for &k in kinds {
        let mut seen = std::collections::HashSet::new();
        let (mut n, mut piles, mut big) = (0, 0, 0);
        for y in 0..b.h {
            for x in 0..b.w {
                if w.get(x, y).material != k || seen.contains(&(x, y)) {
                    continue;
                }
                piles += 1;
                let mut stack = vec![(x, y)];
                seen.insert((x, y));
                let mut size = 0;
                while let Some((cx, cy)) = stack.pop() {
                    size += 1;
                    for &(dx, dy) in &DIRS {
                        let (nx, ny) = (cx + dx, cy + dy);
                        if w.in_bounds(nx, ny) && w.get(nx, ny).material == k && seen.insert((nx, ny)) {
                            stack.push((nx, ny));
                        }
                    }
                }
                n += size;
                big = big.max(size);
            }
        }
        items += n;
        per.push((n, piles, big));
    }
    let (mut same, mut other) = (0u64, 0u64);
    for y in 0..b.h {
        for x in 0..b.w {
            let m = w.get(x, y).material;
            if !kinds.contains(&m) {
                continue;
            }
            for &(dx, dy) in &DIRS {
                if !w.in_bounds(x + dx, y + dy) {
                    continue;
                }
                let n = w.get(x + dx, y + dy).material;
                if n == m {
                    same += 1;
                } else if kinds.contains(&n) {
                    other += 1;
                }
            }
        }
    }
    (items, per, other as f32 / (same + other).max(1) as f32)
}

/// Distance in cells between the centres of the first two kinds' largest
/// piles: how far apart the brood pile and the food pile are.
fn apart(w: &World, kinds: &[MaterialId], b: &Box_) -> f32 {
    let mut centres = Vec::new();
    for &k in kinds.iter().take(2) {
        let mut seen = std::collections::HashSet::new();
        let mut best: (usize, f32, f32) = (0, 0.0, 0.0);
        for y in 0..b.h {
            for x in 0..b.w {
                if w.get(x, y).material != k || !seen.insert((x, y)) {
                    continue;
                }
                let mut stack = vec![(x, y)];
                let (mut n, mut sx, mut sy) = (0usize, 0i64, 0i64);
                while let Some((cx, cy)) = stack.pop() {
                    n += 1;
                    sx += i64::from(cx);
                    sy += i64::from(cy);
                    for &(dx, dy) in &DIRS {
                        let (nx, ny) = (cx + dx, cy + dy);
                        if w.in_bounds(nx, ny) && w.get(nx, ny).material == k && seen.insert((nx, ny)) {
                            stack.push((nx, ny));
                        }
                    }
                }
                if n > best.0 {
                    best = (n, sx as f32 / n as f32, sy as f32 / n as f32);
                }
            }
        }
        centres.push((best.1, best.2));
    }
    if centres.len() < 2 {
        return 0.0;
    }
    ((centres[0].0 - centres[1].0).powi(2) + (centres[0].1 - centres[1].1).powi(2)).sqrt()
}

fn draw(w: &World, b: &Box_, walkers: &[Walker], kinds: &[MaterialId], stone: MaterialId) -> image::RgbaImage {
    const SCALE: u32 = 3;
    let (bw, bh) = (b.w as u32, b.h as u32);
    let mut img = image::RgbaImage::from_pixel(bw * SCALE, bh * SCALE, image::Rgba([24, 22, 28, 255]));
    let palette = [[235, 225, 190, 255], [170, 90, 40, 255], [80, 170, 90, 255]];
    for y in 0..bh {
        for x in 0..bw {
            let m = w.get(x as i32, y as i32).material;
            let c = if m == stone {
                Some([95, 95, 100, 255])
            } else {
                kinds.iter().position(|&k| k == m).map(|i| palette[i % palette.len()])
            };
            if let Some(c) = c {
                for dy in 0..SCALE {
                    for dx in 0..SCALE {
                        img.put_pixel(x * SCALE + dx, y * SCALE + dy, image::Rgba(c));
                    }
                }
            }
        }
    }
    for wk in walkers {
        let c = match wk.held {
            None => [220, 50, 50, 255],
            Some(k) => {
                let i = kinds.iter().position(|&kk| kk == k).unwrap_or(0);
                if i == 0 {
                    [255, 140, 200, 255]
                } else {
                    [255, 200, 40, 255]
                }
            }
        };
        for dy in 0..SCALE {
            for dx in 0..SCALE {
                img.put_pixel(wk.x as u32 * SCALE + dx, wk.y as u32 * SCALE + dy, image::Rgba(c));
            }
        }
    }
    img
}

#[derive(Clone, Copy)]
struct Room {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl Room {
    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }
}

/// Three chambers (the first three entries) and the tunnels joining them,
/// placed by proportion so `width=`/`height=` still make a sane nest.
fn nest_rooms(b: &Box_) -> Vec<Room> {
    let (w, h) = (b.w, b.h);
    let cw = w / 5;
    let ch = h / 6;
    let a = Room {
        x0: w / 10,
        y0: h / 6,
        x1: w / 10 + cw,
        y1: h / 6 + ch,
    };
    let c = Room {
        x0: w - w / 10 - cw,
        y0: h / 6 + h / 8,
        x1: w - w / 10,
        y1: h / 6 + h / 8 + ch,
    };
    let m = Room {
        x0: w / 2 - cw / 2,
        y0: h - h / 6 - ch,
        x1: w / 2 + cw / 2,
        y1: h - h / 6,
    };
    // Tunnels: a to m and m to c, as an L of two-high runs.
    let t1 = Room {
        x0: a.x1 - 2,
        y0: a.y1 - 1,
        x1: a.x1 - 1,
        y1: m.y1,
    };
    let t2 = Room {
        x0: a.x1 - 2,
        y0: m.y1 - 1,
        x1: m.x0,
        y1: m.y1,
    };
    let t3 = Room {
        x0: m.x1,
        y0: m.y1 - 1,
        x1: c.x0 + 1,
        y1: m.y1,
    };
    let t4 = Room {
        x0: c.x0,
        y0: c.y1 - 1,
        x1: c.x0 + 1,
        y1: m.y1,
    };
    vec![a, m, c, t1, t2, t3, t4]
}
