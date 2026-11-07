//! **The Quench prototype's headless harness** (probe 5, `src/quench/`).
//!
//! Plays scripted rounds of the gorge level and prints, per seed and plan,
//! the counts the brief asks for: forge output, houses kept dry, the share of
//! the dam outline filled, quench events (all and under a lid), drips, the
//! time from a splash to its first quench, and the worst and mean frame.
//!
//! ```text
//! cargo run --release --example quenchprobe -- plans=nothing,block,planned,random seeds=1,2,3
//! cargo run --release --example quenchprobe -- plans=planned seeds=1 gif=out.gif every=20
//! ```
//!
//! Plans:
//! - `nothing`: no action at all (the do-nothing control).
//! - `block`: every lid and splash spent early, as close to the vent as it goes.
//! - `planned`: lids over the dam site, then splashes under them as lava arrives.
//! - `random`: the same budget as `planned`, random tools at random places and times.
//! - `splashonly`: the same water as `planned`, no lids (is the lid doing anything?).
//! - `sump`: no water, no lids: dig a pit in the dam site and let the lava fill it.
use pixel_physics::quench::{floor_y, Quench, Tool, FRAME_H, FRAME_W, H, ROUND_SECONDS, TICKS_PER_SECOND, W};
use pixel_physics::render::Renderer;
use std::time::Instant;

struct Args {
    plans: Vec<String>,
    seeds: Vec<u64>,
    gif: Option<String>,
    png: Option<String>,
    every: u64,
    zoom: u32,
    fast: bool,
    window: bool,
    from: f32,
    to: f32,
    crust: u8,
    vent: u64,
    trace: bool,
    flood: u64,
}

fn parse() -> Args {
    let mut a = Args { plans: vec!["nothing".into(), "planned".into()], seeds: vec![1, 2, 3], gif: None, png: None, every: 30, zoom: 2, fast: true, window: false, from: 0.0, to: 1e9, crust: pixel_physics::quench::CRUST_DEPTH, vent: pixel_physics::quench::VENT_EVERY, trace: false, flood: 0 };
    for arg in std::env::args().skip(1) {
        let (k, v) = arg.split_once('=').unwrap_or_else(|| panic!("unknown argument {arg} (want key=value)"));
        match k {
            "plans" => a.plans = v.split(',').map(String::from).collect(),
            "seeds" => a.seeds = v.split(',').map(|s| s.parse().expect("seed")).collect(),
            "gif" => a.gif = Some(v.into()),
            "png" => a.png = Some(v.into()),
            "every" => a.every = v.parse().expect("every"),
            "zoom" => a.zoom = v.parse().expect("zoom"),
            "fast" => a.fast = v == "1" || v == "on",
            "window" => a.window = v == "1",
            "from" => a.from = v.parse().expect("from"),
            "to" => a.to = v.parse().expect("to"),
            "crust" => a.crust = v.parse().expect("crust"),
            "vent" => a.vent = v.parse().expect("vent"),
            "trace" => a.trace = v == "1",
            "flood" => a.flood = v.parse().expect("flood"),
            _ => panic!("unknown argument {k}"),
        }
    }
    println!("quenchprobe: plans {:?} seeds {:?} fast drip {} crust depth {} vent every {} flood {} s gif {:?} png {:?} every {} zoom {} window {}", a.plans, a.seeds, a.fast, a.crust, a.vent, a.flood, a.gif, a.png, a.every, a.zoom, a.window);
    a
}

/// A tiny deterministic generator for the random plan.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next() % (hi - lo) as u64) as i32
    }
}

/// The live half of a plan: what it throws, given where the front is now.
/// Called every half second.
fn react(plan: &str, q: &mut Quench) {
    let Some(front) = q.lava_front() else { return };
    if q.tank < 40 {
        return;
    }
    match plan {
        // Keep the lip of the dam site wet once the front is close: the
        // crust builds where the water waits, and the lid over the lip
        // drips onto it.
        "planned" | "splashonly" if front >= 150 && q.tick.is_multiple_of(120) => {
            let x = 174;
            q.act(Tool::Splash, x, floor_y(x) - 6);
        }
        // Throw under whichever lid the lava is flowing through now.
        "layered" | "layeredwater" if q.tick.is_multiple_of(120) => {
            let lava_in = |q: &Quench, y0: i32, y1: i32| (138..=161).any(|x| (y0..=y1).any(|y| q.world.get(x, y).material == q.lava));
            // Wet the upper gap before the lava reaches it: the pool behind
            // the dam is near the low lid's top.
            let rising = (131..=137).any(|x| (105..=109).any(|y| q.world.get(x, y).material == q.lava));
            // Only when the gap has run dry: water already under the lid is
            // the engine, and more just drains on to the village.
            let water_in = |q: &Quench, y0: i32, y1: i32| (140..=159).map(|x| (y0..=y1).filter(|&y| q.world.get(x, y).material == pixel_physics::sim::material::WATER).count()).sum::<usize>();
            if lava_in(q, 111, 117) && water_in(q, 111, 117) < 10 {
                q.act(Tool::Splash, 150, 114);
            } else if (lava_in(q, 101, 107) || rising) && water_in(q, 101, 107) < 10 {
                q.act(Tool::Splash, 150, 104);
            }
        }
        // Quench it at the vent, as fast as the tank allows.
        "block" if q.tick.is_multiple_of(120) => {
            let x = 16 + (q.tick / 120 % 3) as i32 * 6;
            q.act(Tool::Splash, x, floor_y(x) - 6);
        }
        _ => {}
    }
}

/// The scripted actions for a plan: (second, tool, x, y).
fn script(plan: &str, seed: u64) -> Vec<(f32, Tool, i32, i32)> {
    match plan {
        "nothing" => vec![],
        // No chemistry at all: dig a pit in the dam site for the lava to
        // fill, and leave it.
        "sump" => {
            let mut v = vec![];
            let mut t = 0.5;
            for x in (150..=176).step_by(5) {
                for y in (119..=137).step_by(5) {
                    v.push((t, Tool::Dig, x, y));
                    t += 0.1;
                }
            }
            v
        }
        // Quench it high: every splash at the front while it is still on
        // the upper slope (see `react`).
        "block" => vec![],
        // Roof the dam site; the splashes are thrown live, at the front
        // (see `react`).
        "planned" => vec![(1.0, Tool::Lid, 172, 106), (1.5, Tool::Lid, 148, 106)],
        "splashonly" => vec![],
        // Build the dam in layers before the lava comes: a low lid with water
        // waiting under it, a second lid on top with water between the two.
        // `layeredlids` is the same lids with no water, `layeredwater` the
        // same water with no lids.
        "layered" | "layeredlids" | "layeredwater" => {
            let mut v = vec![];
            let lids = plan != "layeredwater";
            let water = plan != "layeredlids";
            if lids {
                v.push((1.0, Tool::Lid, 150, 109));
            }
            if lids {
                v.push((3.0, Tool::Lid, 150, 99));
            }
            let _ = water; // the water is thrown live, in `react`
            v
        }
        "random" => {
            let mut r = Lcg(seed.wrapping_mul(0x9E3779B97F4A7C15) ^ 0xABCD);
            let mut v = vec![];
            for _ in 0..2 {
                v.push((r.range(0, 90) as f32, Tool::Lid, r.range(10, W - 10), r.range(20, H - 30)));
            }
            for _ in 0..8 {
                v.push((r.range(0, 90) as f32, Tool::Splash, r.range(10, W - 10), r.range(20, H - 30)));
            }
            v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            v
        }
        other => panic!("unknown plan {other}"),
    }
}

fn main() {
    let args = parse();
    println!(
        "{:<10} {:>4} {:>6} {:>6} {:>5} {:>6} {:>6} {:>7} {:>6} {:>8} {:>5} {:>8} {:>8}",
        "plan", "seed", "forge%", "houses", "dam%", "quench", "underL", "drips", "score", "stars", "acts", "worst_ms", "mean_ms"
    );
    for plan in &args.plans {
        for &seed in &args.seeds {
            let mut q = Quench::new(seed);
            q.fast_drip = args.fast;
            q.world.crust_quench_depth = args.crust;
            q.vent_every = args.vent;
            q.flood_seconds = args.flood;
            let mut todo = script(plan, seed);
            todo.reverse();
            let mut renderer = Renderer::new();
            let mut buf = vec![0u8; (W * H * 4) as usize];
            let mut frames: Vec<Vec<u8>> = Vec::new();
            let (mut worst, mut total) = (0f64, 0f64);
            let mut splash_latency: Vec<u64> = Vec::new();
            let end = ROUND_SECONDS * TICKS_PER_SECOND;
            while q.tick < end {
                while let Some(&(t, tool, x, y)) = todo.last() {
                    if q.seconds() >= t {
                        if !q.act(tool, x, y) {
                            println!("           refused: {tool:?} at ({x},{y}) t={t}: {:?}", q.note.as_ref().map(|n| &n.0));
                        }
                        todo.pop();
                    } else {
                        break;
                    }
                }
                if q.tick.is_multiple_of(30) {
                    react(plan, &mut q);
                }
                if args.trace && q.tick.is_multiple_of(120) {
                    // The dam site under the layered plan's two lids, by band.
                    let count = |q: &Quench, m: pixel_physics::sim::material::MaterialId, y0: i32, y1: i32| (138..=161).map(|x| (y0..=y1).filter(|&y| q.world.get(x, y).material == m).count()).sum::<usize>();
                    let (lava, water, stone) = (q.lava, pixel_physics::sim::material::WATER, pixel_physics::sim::material::STONE);
                    println!(
                        "    t={:>5.1} tank {:>3} gap1 lava/water/stone {:>3}/{:>3}/{:>3} gap2 {:>3}/{:>3}/{:>3} past {:>4} quench {:>4} through {:>4} drips {:>4}",
                        q.seconds(), q.tank,
                        count(&q, lava, 111, 117), count(&q, water, 111, 117), count(&q, stone, 111, 117),
                        count(&q, lava, 101, 107), count(&q, water, 101, 107), count(&q, stone, 101, 107),
                        q.counts.past, q.counts.quenched, q.world.phase_changes.reacted_through_crust, q.counts.drips
                    );
                }
                let before = q.counts.last_splash_to_quench;
                let t0 = Instant::now();
                q.step();
                let ms = t0.elapsed().as_secs_f64() * 1000.0;
                worst = worst.max(ms);
                total += ms;
                if q.counts.last_splash_to_quench != before {
                    if let Some(l) = q.counts.last_splash_to_quench {
                        splash_latency.push(l);
                    }
                }
                if (args.gif.is_some() || args.png.is_some()) && q.tick.is_multiple_of(args.every) && q.seconds() >= args.from && q.seconds() <= args.to {
                    if args.window {
                        let mut win = vec![0u8; (FRAME_W * FRAME_H * 4) as usize];
                        q.draw_frame(&mut renderer, &mut buf, &mut win, None);
                        frames.push(win);
                    } else {
                        q.draw_world(&mut renderer, &mut buf);
                        frames.push(buf.clone());
                    }
                }
            }
            let c = q.counts;
            println!(
                "{:<10} {:>4} {:>6.0} {:>4}/{} {:>5.0} {:>6} {:>6} {:>7} {:>6} {:>8} {:>5} {:>8.2} {:>8.2}",
                plan,
                seed,
                q.forge_share() * 100.0,
                q.houses_safe(),
                q.houses.len(),
                c.dam_peak * 100.0,
                c.quenched,
                c.quenched_under_lid,
                c.drips,
                q.score(),
                "*".repeat(q.stars() as usize),
                q.actions.len(),
                worst,
                total / end as f64
            );
            let burned = q.houses.iter().filter(|h| h.burned).count();
            let flooded = q.houses.iter().filter(|h| h.flooded).count();
            let lat: Vec<String> = splash_latency.iter().map(|t| format!("{:.2}", *t as f32 / 60.0)).collect();
            println!("           houses burned {burned} flooded {flooded}; lava past the dam {}; forge burned {} cells; caught back to tank {}; splash->first quench (s): [{}]", c.past, c.forge_burned, c.caught, lat.join(" "));
            if let Some(path) = &args.gif {
                let path = path.replace("{plan}", plan).replace("{seed}", &seed.to_string());
                let (fw, fh, z) = if args.window { (FRAME_W, FRAME_H, 1) } else { (W as u32, H as u32, args.zoom) };
                write_gif(&path, &frames, args.every, fw, fh, z);
                println!("           gif: {path} ({} frames)", frames.len());
            }
            if let Some(path) = &args.png {
                let path = path.replace("{plan}", plan).replace("{seed}", &seed.to_string());
                let last = frames.last().expect("a frame");
                let (fw, fh, z) = if args.window { (FRAME_W, FRAME_H, 1) } else { (W as u32, H as u32, args.zoom) };
                let img = upscale(last, fw, fh, z);
                image::save_buffer(&path, &img, fw * z, fh * z, image::ColorType::Rgba8).expect("png");
                println!("           png: {path}");
            }
        }
    }
}

fn upscale(src: &[u8], sw: u32, sh: u32, z: u32) -> Vec<u8> {
    let (w, h) = (sw * z, sh * z);
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let s = (((y / z) * sw + x / z) * 4) as usize;
            let d = ((y * w + x) * 4) as usize;
            out[d..d + 4].copy_from_slice(&src[s..s + 4]);
            out[d + 3] = 255;
        }
    }
    out
}

/// Plays at twice real time: each frame is `every` ticks of a 60-tick second.
fn write_gif(path: &str, frames: &[Vec<u8>], every: u64, sw: u32, sh: u32, z: u32) {
    let delay_ms = (every * 1000 / 60).max(20) / 2;
    let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(delay_ms));
    let file = std::fs::File::create(path).expect("gif file");
    let mut enc = image::codecs::gif::GifEncoder::new_with_speed(file, 10);
    enc.set_repeat(image::codecs::gif::Repeat::Infinite).expect("repeat");
    for f in frames {
        let img = image::RgbaImage::from_raw(sw * z, sh * z, upscale(f, sw, sh, z)).expect("frame");
        enc.encode_frame(image::Frame::from_parts(img, 0, 0, delay)).expect("encode");
    }
}
