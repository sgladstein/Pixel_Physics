//! **What does the shipped slime mould look like outside its dish?** A
//! design sketch for `Reports/mould-real-world-2026-11-09.md`, not a game.
//!
//! The mould game (`src/mould/`) runs Jones's Physarum agents on a private
//! 256x160 dish. This drops that **same model, unchanged** (the shipped
//! `Params`, via `Dish::from_level_with`) onto two stand-in grounds and lets
//! it grow untouched, so the page can show real simulation rather than a
//! painting:
//!
//! * `view=side` -- a 256x160 window of the **lab box (`played_bed`), aged
//!   `age=` frames** with the game's own step, timeline and mister, chosen
//!   as the window with the most food cells near the surface. Where the
//!   mould may live is read off the world's own cells: litter, dead leaves,
//!   moss, logs, dead wood, fungus, fruit, windfall, corpses, crumbs, seeds
//!   and pips, the top `skin=` rows of soil-like ground, and with `film=N`
//!   the N cells of air above the ground (a walker *on* a surface is blind
//!   side-on: `Reports/dead-ends.md`, creature-direction §13b). Everything
//!   else is rim. Piles sit on the world's own food cells; `buried=N` adds
//!   N piles 12+ rows down, `so=` sets the sensor reach.
//! * `view=top` -- a seeded forest floor seen from above: two fallen logs,
//!   stones, a puddle, scattered leaf food. The ground is painted from the
//!   same mask the mould is stepped on, so nothing drawn is not in the sim.
//!
//! Neither writes to the engine; the side view reads the aged box once.
//! Measured on 2026-11-09 for the report: side-on with surface food it grows
//! one runner (6 of 6 piles in 20 s, untouched); with `buried=5` it roots
//! down (10 of 11).
//!
//! ```text
//! cargo run --release --example mould_world_sketch -- view=side out=DIR
//! cargo run --release --example mould_world_sketch -- view=top out=DIR seed=3
//! ```
//!
//! Writes `DIR/<view>-mask.png` (where it may live), `DIR/<view>-tNN.png`
//! stills, `DIR/<view>.gif`, and prints walkers, tube cells, piles joined
//! and the time per mould step beside each still.

use std::time::Instant;

use pixel_physics::mould::level::{Level, PileSpec, Tools, Win};
use pixel_physics::mould::{render as mrender, Dish, Ground, H, STEPS_PER_SECOND, W};
use pixel_physics::render::Renderer;
use pixel_physics::sim::material::MaterialId;
use pixel_physics::sim::particle::ParticleSystem;

const SCALE: usize = 3;

struct Args {
    view: String,
    out: String,
    seed: u64,
    skin: i32,
    film: i32,
    seeds: bool,
    buried: usize,
    so: f32,
    seconds: u32,
    every: u32,
    world_seed: u64,
    age: u64,
}

fn args() -> Args {
    let mut a = Args {
        view: "side".into(),
        out: "/tmp/mould-world".into(),
        seed: 1,
        skin: 14,
        film: 0,
        seeds: false,
        buried: 0,
        so: 0.0,
        seconds: 60,
        every: 4,
        world_seed: 1,
        age: 20_000,
    };
    for s in std::env::args().skip(1) {
        let (k, v) = s.split_once('=').unwrap_or((s.as_str(), ""));
        match k {
            "view" => a.view = v.into(),
            "out" => a.out = v.into(),
            "seed" => a.seed = v.parse().unwrap(),
            "skin" => a.skin = v.parse().unwrap(),
            "film" => a.film = v.parse().unwrap(),
            "seeds" => a.seeds = v == "1",
            "buried" => a.buried = v.parse().unwrap(),
            "so" => a.so = v.parse().unwrap(),
            "seconds" => a.seconds = v.parse().unwrap(),
            "every" => a.every = v.parse().unwrap(),
            "world_seed" => a.world_seed = v.parse().unwrap(),
            "age" => a.age = v.parse().unwrap(),
            _ => panic!("unknown argument {s}"),
        }
    }
    println!(
        "mould_world_sketch view={} seed={} skin={} film={} seeds={} buried={} seconds={} every={} world_seed={} out={}",
        a.view, a.seed, a.skin, a.film, a.seeds, a.buried, a.seconds, a.every, a.world_seed, a.out
    );
    a
}

/// What the side view hands the mould: where it may live, where food is,
/// and the engine's own picture of the window under it.
struct Ground2 {
    mask: Vec<bool>,
    piles: Vec<(f32, f32)>,
    under: Vec<[u8; 3]>,
    note: String,
}

fn side(a: &Args) -> Ground2 {
    // The lab's played bed (the main game's box), aged `age=` frames with
    // the game's own step, timeline and mister, so it has dropped leaves,
    // fruit and dead ants on it.
    let mut sc = pixel_physics::lab::scenario::Scenario::load("played_bed").expect("played_bed");
    sc.bed.seed = a.world_seed;
    let spec = sc.bed.clone();
    let (mut world, _, _) = sc.build();
    let mut particles = ParticleSystem::new();
    let mut blasts = pixel_physics::sim::explosion::Blasts::new();
    let tuning = pixel_physics::sim::player::Tuning::default();
    let t0 = Instant::now();
    for _ in 0..a.age {
        pixel_physics::sim::frame::step(&mut world, &mut particles, &mut blasts, pixel_physics::sim::player::PlayerInput::default(), &tuning);
        pixel_physics::lab::scenario::tick_timeline(&sc, &mut world, &spec);
        pixel_physics::lab::rain::tick(&mut world, &spec, spec.rain);
    }
    println!("lab box aged {} frames in {:.1}s", a.age, t0.elapsed().as_secs_f32());
    let (ww, wh) = (spec.width, spec.height);
    let m = &world.materials;
    let id = |n: &str| m.id_of(n);
    let rot: Vec<MaterialId> = ["litter", "deadleaf", "moss", "log", "deadwood", "fungus", "fruit", "windfall", "corpse", "crumbs", "seed", "pip"]
        .iter()
        .filter_map(|n| id(n))
        .collect();
    // `seeds=1` also counts buried seeds and pips as food, which puts food
    // under the ground and not only on it.
    let mut food_names = vec!["deadleaf", "fruit", "windfall", "fungus", "corpse", "crumbs", "litter"];
    if a.seeds {
        food_names.extend(["seed", "buriedseed", "pip"]);
    }
    let food: Vec<MaterialId> = food_names.iter().filter_map(|n| id(n)).collect();
    let soil: Vec<MaterialId> = ["soil", "packedsoil", "spoil", "grassroot", "rootwood", "nest"].iter().filter_map(|n| id(n)).collect();
    // Ground surface per column: first soil-like or rotting cell from the
    // top, below the room's ceiling.
    let surface = |x: i32| -> i32 {
        (spec.room_top().max(0)..wh)
            .find(|&y| {
                let c = world.get(x, y).material;
                soil.contains(&c) || rot.contains(&c)
            })
            .unwrap_or(wh - 1)
    };
    let surf: Vec<i32> = (0..ww).map(surface).collect();
    // Pick the window: most rotting-matter cells, sampled every 16 columns.
    let mut best = (-1i64, 0i32, 0i32);
    let mut x0 = 0;
    while x0 + W as i32 <= ww {
        let mid = surf[(x0 + W as i32 / 2) as usize];
        let y0 = (mid - 70).clamp(0, wh - H as i32);
        let mut n = 0i64;
        for y in y0..y0 + H as i32 {
            for x in x0..x0 + W as i32 {
                if food.contains(&world.get(x, y).material) {
                    n += 1;
                }
            }
        }
        if n > best.0 {
            best = (n, x0, y0);
        }
        x0 += 16;
    }
    let (_, x0, y0) = best;
    let mut mask = vec![false; W * H];
    let mut food_cells = Vec::new();
    let mut census = std::collections::BTreeMap::<String, u32>::new();
    for y in 0..H {
        for x in 0..W {
            let (wx, wy) = (x0 + x as i32, y0 + y as i32);
            let c = world.get(wx, wy).material;
            let depth = wy - surf[wx as usize];
            let live = rot.contains(&c)
                || (soil.contains(&c) && (0..a.skin).contains(&depth))
                // `film=N`: the air just above the ground, so it can run over the top.
                || (c == pixel_physics::sim::material::EMPTY && (-a.film..0).contains(&depth));
            mask[y * W + x] = live;
            if live {
                *census.entry(m.get(c).name.to_string()).or_default() += 1;
            }
            if food.contains(&c) {
                food_cells.push((x as f32, y as f32));
            }
        }
    }
    // Greedy piles on the world's own food, 24 cells apart, at most 9.
    let mut piles: Vec<(f32, f32)> = Vec::new();
    // A pile must sit in ground the mould can stand round (20 of the 49
    // cells about it living), not on a lone seed up a stem; and the cells
    // are taken in a shuffled order so the top row does not win every pile.
    food_cells.retain(|&(x, y)| {
        let (x, y) = (x as i32, y as i32);
        let mut n = 0;
        for dy in -3..=3 {
            for dx in -3..=3 {
                let (xx, yy) = (x + dx, y + dy);
                if xx >= 0 && yy >= 0 && (xx as usize) < W && (yy as usize) < H && mask[yy as usize * W + xx as usize] {
                    n += 1;
                }
            }
        }
        n >= 20
    });
    food_cells.sort_by_key(|&(x, y)| ((x as u32 * 7919) ^ (y as u32 * 104_729)).wrapping_mul(2_654_435_761));
    for &(x, y) in &food_cells {
        if x < 6.0 || y < 6.0 || x > W as f32 - 6.0 || y > H as f32 - 6.0 {
            continue;
        }
        if piles.iter().all(|p| (p.0 - x).hypot(p.1 - y) >= 24.0) && piles.len() < 9 {
            piles.push((x, y));
        }
    }
    // `buried=N`: N more piles at living cells at least 12 rows under the
    // surface -- a stand-in for food the world could bury (a dead ant, a
    // root, a larder), to ask whether food in depth gives a network.
    let mut k = 0u64;
    let mut tries = 0;
    let mut extra = 0;
    while extra < a.buried && tries < 100_000 {
        tries += 1;
        k = k.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let (x, y) = (8 + (k >> 33) as usize % (W - 16), 8 + (k >> 13) as usize % (H - 16));
        let depth = (y0 + y as i32) - surf[(x0 + x as i32) as usize];
        if mask[y * W + x] && depth >= 12 && piles.iter().all(|p| (p.0 - x as f32).hypot(p.1 - y as f32) >= 30.0) {
            piles.push((x as f32, y as f32));
            extra += 1;
        }
    }
    // A pile up in a bush is no use to a mould on the ground: drop each one
    // down its column to the first cell it may live in.
    for p in piles.iter_mut() {
        let x = p.0 as usize;
        if let Some(y) = (p.1 as usize..H).find(|&y| mask[y * W + x]) {
            p.1 = y as f32 + 2.0;
        }
    }
    // Home: the leftmost pile, so the mould has the window to cross.
    piles.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    // The inoculum lands only inside the dish's own ellipse (`agar_ground`),
    // so home is the leftmost pile well inside it.
    if let Some(k) = piles.iter().position(|p| {
        let (dx, dy) = ((p.0 - W as f32 / 2.0) / (W as f32 / 2.0 - 3.0), (p.1 - H as f32 / 2.0) / (H as f32 / 2.0 - 3.0));
        dx * dx + dy * dy < 0.6
    }) {
        let h = piles.remove(k);
        piles.insert(0, h);
    }
    // Engine picture of the window, at noon, off the shipped renderer.
    let mut r = Renderer::new();
    r.pinned_light = Some(pixel_physics::sky::frame_for_daylight(1.0));
    r.set_camera(x0, y0, (W as u32, H as u32), world.bounds());
    let mut frame = vec![0u8; W * H * 4];
    r.draw(&world, &ParticleSystem::new(), &pixel_physics::sim::fxhash::ChunkSet::default(), &mut frame, (W as u32, H as u32), true);
    let under = (0..W * H).map(|i| [frame[i * 4], frame[i * 4 + 1], frame[i * 4 + 2]]).collect();
    let note = format!(
        "lab played_bed window x={x0} y={y0} (bed seed {}, aged {} frames), living cells {}, by material {:?}, food cells {}, piles {}",
        a.world_seed,
        a.age,
        mask.iter().filter(|&&b| b).count(),
        census,
        food_cells.len(),
        piles.len()
    );
    Ground2 { mask, piles, under, note }
}

struct Rng(u64);
impl Rng {
    fn f(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// A forest floor from above. Moss everywhere it can live; logs are bark
/// (lives, and is drawn as bark); stones and a puddle are rim.
fn top(a: &Args) -> Ground2 {
    let mut rng = Rng(a.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut mask = vec![true; W * H];
    let mut under = vec![[0u8; 3]; W * H];
    let mut kind = vec![0u8; W * H]; // 0 moss, 1 bark, 2 stone, 3 water, 4 leaf
    // Two logs: thick lines.
    let logs = [((20.0, 120.0), (150.0, 70.0), 9.0f32), ((140.0, 150.0), (240.0, 30.0), 7.0)];
    for &((ax, ay), (bx, by), r) in &logs {
        for y in 0..H {
            for x in 0..W {
                let (px, py) = (x as f32, y as f32);
                let (dx, dy) = (bx - ax, by - ay);
                let t = (((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
                let d = (px - ax - t * dx).hypot(py - ay - t * dy);
                if d < r {
                    kind[y * W + x] = 1;
                }
            }
        }
    }
    // Stones and a puddle.
    let mut blobs = Vec::new();
    for _ in 0..7 {
        blobs.push((rng.f() * W as f32, rng.f() * H as f32, 5.0 + rng.f() * 9.0, 2u8));
    }
    blobs.push((80.0 + rng.f() * 60.0, 25.0 + rng.f() * 20.0, 20.0, 3));
    for &(cx, cy, r, k) in &blobs {
        for y in 0..H {
            for x in 0..W {
                let (dx, dy) = (x as f32 - cx, (y as f32 - cy) * if k == 3 { 1.6 } else { 1.0 });
                if dx.hypot(dy) < r && kind[y * W + x] != 1 {
                    kind[y * W + x] = k;
                }
            }
        }
    }
    let mut piles = vec![(40.0f32, 80.0f32)];
    let mut tries = 0;
    while piles.len() < 8 && tries < 1000 {
        tries += 1;
        let (x, y) = (10.0 + rng.f() * (W as f32 - 20.0), 10.0 + rng.f() * (H as f32 - 20.0));
        let i = y as usize * W + x as usize;
        if kind[i] < 2 && piles.iter().all(|p| (p.0 - x).hypot(p.1 - y) >= 40.0) {
            piles.push((x, y));
        }
    }
    for &(px, py) in &piles {
        for y in 0..H {
            for x in 0..W {
                if (x as f32 - px).hypot(y as f32 - py) < 6.5 && kind[y * W + x] < 2 {
                    kind[y * W + x] = 4;
                }
            }
        }
    }
    for i in 0..W * H {
        let h = ((i as u32).wrapping_mul(2_654_435_761) >> 27) as i32 - 16;
        let j = |c: [i32; 3]| c.map(|v| (v + h).clamp(0, 255) as u8);
        under[i] = match kind[i] {
            0 => j([46, 70, 38]),
            1 => j([92, 64, 40]),
            2 => j([120, 120, 116]),
            3 => j([40, 70, 110]),
            _ => j([150, 92, 34]),
        };
        mask[i] = kind[i] < 2 || kind[i] == 4;
    }
    let note = format!("forest floor seed {}, piles {}", a.seed, piles.len());
    Ground2 { mask, piles, under, note }
}

fn main() {
    let a = args();
    std::fs::create_dir_all(&a.out).unwrap();
    let g = if a.view == "side" { side(&a) } else { top(&a) };
    println!("{}", g.note);
    assert!(!g.piles.is_empty(), "no food in the window");
    let specs: Vec<PileSpec> =
        g.piles.iter().map(|&(x, y)| PileSpec { x, y, r: 5.0, food: 20_000.0, arrives: None }).collect();
    let level: &'static Level = Box::leak(Box::new(Level {
        key: "sketch",
        name: "sketch",
        goal: ["", ""],
        twist: "",
        piles: Box::leak(specs.into_boxed_slice()),
        salt: &[],
        tools: Tools { oats: 0, oats_regrow: 0, salt: 0, snips: 0, water: 0, water_regrow: 0 },
        seconds: 100_000,
        win: Win::AliveAtBell { bars: [1, 2, 3] },
        // The shipped free dish's angle (`Params::default`).
        sensor_deg: 45.0,
    }));
    // `so=`: Jones's sensor offset, the knob that sets how wide a tube is
    // (0 keeps the shipped value).
    let mut params = Level::params();
    if a.so > 0.0 {
        params.sensor_offset = a.so;
    }
    println!("sensor_offset {}", params.sensor_offset);
    let mut d = Dish::from_level_with(level, a.seed, params);
    // The window's outer ring is always rim. `Dish::spread` only updates
    // cells 1..W-2 and 1..H-2 (the shipped ellipse never reaches the edge),
    // so trail laid on an edge cell would never fade; the code review of
    // 2026-11-09 found it standing as bright tube at x=0 and x=255.
    let mut g = g;
    for y in 0..H {
        for x in 0..W {
            if x == 0 || y == 0 || x == W - 1 || y == H - 1 {
                g.mask[y * W + x] = false;
            }
        }
    }
    for i in 0..W * H {
        d.ground[i] = if g.mask[i] { Ground::Agar } else { Ground::Rim };
    }
    // Mask picture: where it may live, drawn as a full replace.
    let mut mask_img = vec![0u8; W * H * 3];
    for i in 0..W * H {
        let base = g.under[i];
        let px = if g.mask[i] { [230, 210, 120] } else { [base[0] / 3, base[1] / 3, base[2] / 3] };
        mask_img[i * 3..i * 3 + 3].copy_from_slice(&px);
    }
    for &(x, y) in &g.piles {
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                let (xx, yy) = (x as i32 + dx, y as i32 + dy);
                if xx >= 0 && yy >= 0 && (xx as usize) < W && (yy as usize) < H {
                    let i = yy as usize * W + xx as usize;
                    mask_img[i * 3..i * 3 + 3].copy_from_slice(&[220, 60, 40]);
                }
            }
        }
    }
    save(&format!("{}/{}-mask.png", a.out, a.view), &mask_img);
    save(&format!("{}/{}-under.png", a.out, a.view), &g.under.iter().flatten().copied().collect::<Vec<u8>>());

    let file = std::fs::File::create(format!("{}/{}.gif", a.out, a.view)).unwrap();
    let mut enc = image::codecs::gif::GifEncoder::new_with_speed(file, 10);
    enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
    let sps = STEPS_PER_SECOND;
    let mut step_ns = 0u128;
    let mut steps = 0u64;
    for sec in 0..=a.seconds {
        if sec > 0 {
            for _ in 0..sps {
                let t = Instant::now();
                d.step();
                step_ns += t.elapsed().as_nanos();
                steps += 1;
            }
        }
        if sec % a.every == 0 || sec == a.seconds {
            let img = composite(&d, &g);
            let tube = (0..W * H).filter(|&i| d.trail[i] > d.params.tube).count();
            // Piles with mould on them at the last census, home included
            // (home counts from t=0, so "besides home" is this minus one).
            let joined = d.piles.iter().filter(|p| p.reached()).count();
            println!(
                "t={sec:>3}s walkers={:>5} tube_cells={:>6} piles_reached={}/{} step_ms={:.3}",
                d.walkers.len(),
                tube,
                joined,
                d.piles.len(),
                if steps > 0 { step_ns as f64 / steps as f64 / 1e6 } else { 0.0 }
            );
            save(&format!("{}/{}-t{:02}.png", a.out, a.view, sec), &img);
            let big = upscale(&img);
            let rgba: Vec<u8> = big.chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect();
            let buf = image::RgbaImage::from_raw((W * SCALE) as u32, (H * SCALE) as u32, rgba).unwrap();
            enc.encode_frame(image::Frame::from_parts(buf, 0, 0, image::Delay::from_numer_denom_ms(400, 1))).unwrap();
        }
    }
}

/// The mould's own colours (`render::cell_colour`) where it has laid tube,
/// the ground underneath everywhere else. A full replace, never a blend, so
/// the picture cannot invent a tube.
fn composite(d: &Dish, g: &Ground2) -> Vec<u8> {
    let mut out = vec![0u8; W * H * 3];
    for i in 0..W * H {
        let c = if d.ground[i] == Ground::Agar && d.trail[i] > d.params.tube * 0.5 { mrender::cell_colour(d, i) } else { g.under[i] };
        out[i * 3..i * 3 + 3].copy_from_slice(&c);
    }
    out
}

fn upscale(img: &[u8]) -> Vec<u8> {
    let (bw, bh) = (W * SCALE, H * SCALE);
    let mut big = vec![0u8; bw * bh * 3];
    for y in 0..bh {
        for x in 0..bw {
            let s = ((y / SCALE) * W + x / SCALE) * 3;
            let t = (y * bw + x) * 3;
            big[t..t + 3].copy_from_slice(&img[s..s + 3]);
        }
    }
    big
}

fn save(path: &str, img: &[u8]) {
    let big = upscale(img);
    image::save_buffer(path, &big, (W * SCALE) as u32, (H * SCALE) as u32, image::ColorType::Rgb8).unwrap();
}
