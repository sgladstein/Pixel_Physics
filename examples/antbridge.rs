//! **The double bridge with real ants** (Goss et al. 1989; Deneubourg et
//! al. 1990): level 2 of the bridge test, the ants of `bridgebox`'s walker
//! result on the game's body and engine.
//!
//! A stone box: a nest chamber on the left, a food chamber on the right
//! with `food=` rows of `crumbs` on its floor, and between them one neck from
//! each chamber to a fork, then two branches `gap=` rows apart. `long=<rows>`
//! makes the `longside=` branch (lower by default) dip that many rows and
//! back, so it is `2 x rows` cells longer; `opens=<frame>` keeps the other
//! (short) branch shut at both ends until then -- Goss's "short branch added
//! later", whose colonies stayed on the long one (16/18).
//!
//! `ant=textbook` (default) is the textbook ant's slice B
//! (`creature::textbook::ForageCfg`): it picks up a crumb it touches, carries
//! it home on its home vector, puts it down in the nest chamber, and walks by
//! Deneubourg's `(k + C)^n` on the food trail it lays. `mute=1` lays the
//! trail and never reads it. `ant=game` is the shipped ant in the same box.
//!
//! **Demography is fixed** in every arm: births paused, `halflife` long, and
//! every `500` frames each ant is topped up to `fed=` of its start energy
//! (1.0 by default; the textbook ant does not eat, so this is the stopgap
//! that keeps it alive; the game ant needs hunger to forage, so give it
//! less). Crumbs put down in the nest are taken away every 500 frames and
//! the food floor is restocked: the colony neither fills nor runs dry.
//!
//! Read: crossings of the two branches at the middle column, by branch and
//! by laden, per window; per seed the short branch's share of all crossings
//! over the whole run and the last half, and whether it is the majority or
//! over 80% (Goss's two bars).
//!
//! **What it found, 2026-10-15** (`release=50 brho=0.0002 bdiff=0 lay=0.1
//! k=2 long=40`, 30k frames, 8 seeds, numbers in `textbook::ForageCfg`):
//! the textbook ant passes both of Goss's bars when the short branch is
//! the lower one, and holds the long branch when a short one is added in
//! either mirror; with the short branch above it fails, because a
//! trail-following colony in a side view settles on the lower branch.
//! Three harness biases came first: the vectors aimed at the chamber
//! floors (below the fork) pulled every choice down; a trail at the
//! engine plane's 255 cap reads the same on both branches; and the
//! plane's blend carried the neck's trail into both arms of the fork.
//! `ant=game` does not forage in this box (0-3 crossings a run); its
//! bridge is `trailfollow`'s.

use pixel_physics::sim::creature::{self, textbook};
use pixel_physics::sim::parallel;
use pixel_physics::sim::pheromone::Channel;
use pixel_physics::sim::rng::Rng;
use pixel_physics::sim::{Cell, Rect, World};

const KEYS: &[&str] = &[
    "seed", "seeds", "frames", "ants", "ant", "gap", "len", "neck", "tunnel", "long", "longside", "opens", "release",
    "k", "n", "lay", "layout", "vector", "mute", "brho", "bdiff", "fed", "halflife", "food", "png", "every",
];

const FEED_EVERY: u64 = 500;

fn check_args() {
    let mut seen = std::collections::HashSet::new();
    for a in std::env::args().skip(1) {
        let key = a.split_once('=').map_or(a.as_str(), |(k, _)| k).to_string();
        assert!(KEYS.contains(&key.as_str()), "antbridge: unknown argument `{a}` (known: {})", KEYS.join(", "));
        assert!(seen.insert(key.clone()), "antbridge: `{key}` given twice");
    }
}

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}="))
            .map(|v| v.parse().ok().unwrap_or_else(|| panic!("antbridge: `{a}` does not parse")))
    })
}

/// The box, in cells.
struct Scene {
    nest: (i32, i32, i32, i32),
    food: (i32, i32, i32, i32),
    mid: i32,
    /// Rows at or above this are the upper branch.
    split_y: i32,
    neck_y: i32,
    plug: Vec<(i32, i32)>,
    short_upper: bool,
}

const CH_W: i32 = 16;
const MARGIN: i32 = 4;

/// The box's width and height for these arguments.
fn size(gap: i32, len: i32, th: i32, long: i32) -> (i32, i32) {
    (MARGIN + CH_W + len + CH_W + MARGIN, MARGIN + long + gap + 2 * th + 10 + MARGIN)
}

#[allow(clippy::too_many_arguments)]
fn build(world: &mut World, gap: i32, len: i32, neck: i32, th: i32, long: i32, long_lower: bool, food_rows: i32) -> Scene {
    let stone = world.materials.id_of("stone").expect("stone");
    let crumbs = world.materials.id_of("crumbs").expect("crumbs");
    let (ch_w, margin) = (CH_W, MARGIN);
    let ch_h = gap + 2 * th + 10;
    let up_room = if long_lower { 0 } else { long };
    let (w, h) = size(gap, len, th, long);
    for y in 0..h {
        for x in 0..w {
            world.set(x, y, Cell::new(stone, 0).with_attached(true));
        }
    }
    let ch_y0 = margin + up_room;
    let nest = (margin, ch_y0, margin + ch_w - 1, ch_y0 + ch_h - 1);
    let food = (margin + ch_w + len, ch_y0, margin + ch_w + len + ch_w - 1, ch_y0 + ch_h - 1);
    let carve = |world: &mut World, x0: i32, y0: i32, x1: i32, y1: i32| {
        for y in y0..=y1 {
            for x in x0..=x1 {
                world.set(x, y, Cell::EMPTY);
            }
        }
    };
    carve(world, nest.0, nest.1, nest.2, nest.3);
    carve(world, food.0, food.1, food.2, food.3);
    let up_y = ch_y0 + 4;
    let lo_y = up_y + th + gap;
    // The neck meets the shaft halfway between the branches, so neither
    // branch is the neck's straight continuation.
    let neck_y = (up_y + lo_y) / 2;
    let (x_a, x_b) = (nest.2 + 1 + neck, food.0 - 1 - neck);
    // Necks along each chamber's lower part, then a shaft up to both branches.
    for (x0, x1, shaft) in [(nest.2 + 1, x_a + th - 1, x_a), (x_b - th + 1, food.0 - 1, x_b - th + 1)] {
        carve(world, x0, neck_y, x1, neck_y + th - 1);
        carve(world, shaft, up_y, shaft + th - 1, lo_y + th - 1);
    }
    let mid = (nest.2 + food.0) / 2;
    let (a, b) = (nest.2 + 1 + len / 4, nest.2 + 1 + 3 * len / 4);
    for (y, is_long, dir) in [(up_y, !long_lower, -1), (lo_y, long_lower, 1)] {
        if is_long && long > 0 {
            let far = y + dir * long;
            let (ylo, yhi) = (y.min(far), y.max(far));
            carve(world, x_a, y, a + th - 1, y + th - 1);
            carve(world, a, ylo, a + th - 1, yhi + th - 1);
            carve(world, a, far, b + th - 1, far + th - 1);
            carve(world, b, ylo, b + th - 1, yhi + th - 1);
            carve(world, b, y, x_b, y + th - 1);
        } else {
            carve(world, x_a, y, x_b, y + th - 1);
        }
    }
    // Food: rows of crumbs on the food chamber's floor.
    for y in food.3 - food_rows + 1..=food.3 {
        for x in food.0..=food.2 {
            world.set(x, y, Cell::new(crumbs, 0));
        }
    }
    let short_y = if long_lower { up_y } else { lo_y };
    let (p_a, p_b) = (x_a + th, x_b - th - 1);
    let plug = [p_a, p_a + 1, p_b, p_b + 1]
        .into_iter()
        .flat_map(|x| (short_y..short_y + th).map(move |y| (x, y)))
        .collect();
    Scene {
        nest,
        food,
        mid,
        split_y: (up_y + th + lo_y) / 2,
        neck_y,
        plug,
        short_upper: long_lower,
    }
}

fn main() {
    check_args();
    creature::mutation_off_for_measuring();
    let seeds: Vec<u64> = match arg::<u64>("seed") {
        Some(s) => vec![s],
        None => (1..=arg::<u64>("seeds").unwrap_or(8)).collect(),
    };
    let frames: u64 = arg("frames").unwrap_or(60_000);
    let n_ants: usize = arg("ants").unwrap_or(60);
    let kind: String = arg("ant").unwrap_or_else(|| "textbook".into());
    assert!(kind == "textbook" || kind == "game", "antbridge: ant={kind}: textbook or game");
    let gap: i32 = arg("gap").unwrap_or(10);
    let len: i32 = arg("len").unwrap_or(80);
    let neck: i32 = arg("neck").unwrap_or(10);
    let th: i32 = arg("tunnel").unwrap_or(3);
    let long: i32 = arg("long").unwrap_or(0);
    let long_lower = arg::<String>("longside").as_deref() != Some("upper");
    let opens: u64 = arg("opens").unwrap_or(0);
    let release: u64 = arg("release").unwrap_or(0);
    let fed: f32 = arg("fed").unwrap_or(1.0);
    let food_rows: i32 = arg("food").unwrap_or(2);
    let every: u64 = arg("every").unwrap_or(frames / 10);
    println!(
        "antbridge: ant={kind} ants={n_ants} gap={gap} len={len} neck={neck} tunnel={th} long={long} longside={} opens={opens} release={release} fed={fed} frames={frames}",
        if long_lower { "lower" } else { "upper" }
    );
    let mut rows = Vec::new();
    for &seed in &seeds {
        rows.push(run(seed, frames, n_ants, &kind, gap, len, neck, th, long, long_lower, opens, release, fed, food_rows, every));
    }
    let maj = rows.iter().filter(|r| r.0 > 0.5).count();
    let over = rows.iter().filter(|r| r.0 >= 0.8).count();
    println!(
        "SUMMARY short-branch share, last half (share/crossings): [{}]; majority {maj} of {}, over 80% {over} of {}",
        rows.iter().map(|r| format!("{:.2}/{}", r.0, r.1)).collect::<Vec<_>>().join(" "),
        rows.len(),
        rows.len()
    );
    println!(
        "SUMMARY whole run: [{}]; delivered per seed [{}]",
        rows.iter().map(|r| format!("{:.2}/{}", r.2, r.3)).collect::<Vec<_>>().join(" "),
        rows.iter().map(|r| r.4.to_string()).collect::<Vec<_>>().join(" ")
    );
}

#[allow(clippy::too_many_arguments)]
fn run(
    seed: u64,
    frames: u64,
    n_ants: usize,
    kind: &str,
    gap: i32,
    len: i32,
    neck: i32,
    th: i32,
    long: i32,
    long_lower: bool,
    opens: u64,
    release: u64,
    fed: f32,
    food_rows: i32,
    every: u64,
) -> (f32, u64, f32, u64, u64) {
    let (w, h) = size(gap, len, th, long);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    world.seed = seed;
    let sc = build(&mut world, gap, len, neck, th, long, long_lower, food_rows);
    let stone = world.materials.id_of("stone").expect("stone");
    let crumbs = world.materials.id_of("crumbs").expect("crumbs");
    if opens > 0 {
        for &(x, y) in &sc.plug {
            world.set(x, y, Cell::new(stone, 0).with_attached(true));
        }
    }
    world.pheromones.set_channel_rho(Channel::B, arg("brho").unwrap_or(0.001));
    world.pheromones.set_channel_diffuse(Channel::B, arg("bdiff").unwrap_or(0.05));
    for _ in 0..100 {
        parallel::step(&mut world);
    }
    let id = world.species.id_of("ant").expect("ant ships");
    if let Some(c) = world.species.get_mut(id).creature.as_mut() {
        c.life_half_life = arg("halflife").unwrap_or(10_000_000);
    }
    world.births_paused = true;
    // Both vectors aim at the neck's level, so at a fork they favour neither
    // branch; the drop zone is the nest chamber.
    let home = ((sc.nest.0 + sc.nest.2) / 2, sc.nest.3);
    let aim_home = ((sc.nest.0 + sc.nest.2) / 2, sc.neck_y);
    let feeder = ((sc.food.0 + sc.food.2) / 2, sc.neck_y);
    if kind == "textbook" {
        let mute = arg::<u8>("mute").unwrap_or(0) == 1;
        let cfg = textbook::TextbookCfg {
            p_move: 1.0,
            turn: 0.125,
            dt: 4.0,
            drop_base: 0.025,
            drop_per: 0.11,
            pick: 0.029,
            tau: 0.0,
            template: 2.0,
            source: crumbs,
            n_rock: false,
            pick_around: true,
            base_always: true,
            build_on_box: false,
            forage: Some(textbook::ForageCfg {
                food: crumbs,
                home,
                home_r: (sc.nest.2 - sc.nest.0) / 2 + 1,
                aim_home,
                feeder,
                vector: arg("vector").unwrap_or(1.0),
                k: arg("k").unwrap_or(20.0),
                n: arg("n").unwrap_or(2),
                lay: arg("lay").unwrap_or(1.0),
                lay_out: arg::<u8>("layout").unwrap_or(1) == 1,
                read: !mute,
            }),
        };
        world.textbook = Some(Box::new(textbook::Textbook::new(id, cfg)));
    }
    let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xa27b);
    let mut colony = None;
    let mut placed = 0usize;
    let mut plant = |world: &mut World, placed: &mut usize, rng: &mut Rng| {
        for _ in 0..40 {
            let x = sc.nest.0 + 1 + rng.below((sc.nest.2 - sc.nest.0 - 1) as u32) as i32;
            if let Some(site) = creature::plant_founder_in(world, x, sc.nest.3, "ant", colony, None) {
                if colony.is_none() {
                    colony = creature::colony_of_site(world, &site);
                }
                world.schedule_active_site(site);
                *placed += 1;
                return;
            }
        }
    };
    if release == 0 {
        while placed < n_ants {
            let before = placed;
            plant(&mut world, &mut placed, &mut rng);
            if placed == before {
                break;
            }
        }
    }
    let mut last_x: std::collections::HashMap<u32, i32> = std::collections::HashMap::new();
    // [short, long] crossings: whole run, last half, this window.
    let (mut whole, mut late, mut win) = ([0u64; 2], [0u64; 2], [[0u64; 2]; 2]);
    let mut delivered = 0u64;
    let short_lane = |y: i32| -> usize {
        let upper = y < sc.split_y;
        usize::from(upper != sc.short_upper)
    };
    for f in 1..=frames {
        if release > 0 && placed < n_ants && f % release == 0 {
            plant(&mut world, &mut placed, &mut rng);
        }
        if opens > 0 && f == opens {
            for &(x, y) in &sc.plug {
                world.set(x, y, Cell::EMPTY);
            }
        }
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
        if f % FEED_EVERY == 0 {
            feed(&mut world, fed);
            // Eat what came home; restock the feeder.
            for y in sc.nest.1..=sc.nest.3 {
                for x in sc.nest.0..=sc.nest.2 {
                    if world.get(x, y).material == crumbs && world.get(x, y).organism_id() == 0 {
                        world.set(x, y, Cell::EMPTY);
                    }
                }
            }
            for y in sc.food.3 - food_rows + 1..=sc.food.3 {
                for x in sc.food.0..=sc.food.2 {
                    if world.is_empty(x, y) {
                        world.set(x, y, Cell::new(crumbs, 0));
                    }
                }
            }
        }
        for oid in world.live_organism_ids() {
            let Some(st) = world.organism(oid) else { continue };
            if st.species != id {
                continue;
            }
            let Some(&(hx, hy)) = st.chain.first() else { continue };
            let laden = usize::from(st.spoil.is_some());
            if let Some(&px) = last_x.get(&oid) {
                if (px < sc.mid) != (hx < sc.mid) && hx > sc.nest.2 && hx < sc.food.0 {
                    let lane = short_lane(hy);
                    whole[lane] += 1;
                    win[laden][lane] += 1;
                    if f > frames / 2 {
                        late[lane] += 1;
                    }
                }
            }
            last_x.insert(oid, hx);
        }
        if f % every == 0 {
            let counts = world.textbook.as_ref().map(|t| t.counts);
            let d = counts.map_or(0, |c| c.delivered);
            println!(
                "seed {seed} f {f:>7}: crossings short/long empty {}/{} laden {}/{} | alive {} | picked {} delivered {}",
                win[0][0],
                win[0][1],
                win[1][0],
                win[1][1],
                world.live_organism_ids().len(),
                counts.map_or(0, |c| c.food_picks),
                d
            );
            delivered = d;
            win = [[0; 2]; 2];
        }
    }
    if let Some(dir) = arg::<String>("png") {
        let _ = std::fs::create_dir_all(&dir);
        let img = draw(&world, w, h);
        img.save(format!("{dir}/s{seed}.png")).expect("png");
    }
    let share = |a: [u64; 2]| a[0] as f32 / (a[0] + a[1]).max(1) as f32;
    (share(late), late[0] + late[1], share(whole), whole[0] + whole[1], delivered)
}

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

/// Stone grey, open dark, crumbs tan, ants yellow, and the food trail as red
/// over open cells.
fn draw(wd: &World, w: i32, h: i32) -> image::RgbaImage {
    const PX: u32 = 3;
    let stone = wd.materials.id_of("stone");
    let crumbs = wd.materials.id_of("crumbs");
    let mut img = image::RgbaImage::new(w as u32 * PX, h as u32 * PX);
    for y in 0..h {
        for x in 0..w {
            let c = wd.get(x, y);
            let s = (f32::from(wd.pheromone_at(Channel::B, x, y)) / 65535.0).sqrt();
            let col = if c.organism_id() != 0 {
                [250, 220, 60, 255]
            } else if Some(c.material) == stone {
                [90, 90, 96, 255]
            } else if Some(c.material) == crumbs {
                [200, 150, 90, 255]
            } else {
                [(24.0 + 220.0 * s) as u8, 22, 30, 255]
            };
            for dy in 0..PX {
                for dx in 0..PX {
                    img.put_pixel(x as u32 * PX + dx, y as u32 * PX + dy, image::Rgba(col));
                }
            }
        }
    }
    img
}
