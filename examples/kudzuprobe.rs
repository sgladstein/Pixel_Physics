//! **The Kudzu probe's headless harness** -- see `src/kudzu/mod.rs`.
//!
//! ```text
//! cargo run --release --example kudzuprobe -- mode=grow frames=6000 every=600 out=/tmp/kudzu
//! cargo run --release --example kudzuprobe -- mode=play policy=nothing,random,planned seed=1 seeds=8
//! cargo run --release --example kudzuprobe -- mode=play policy=nothing,planned pictures=true out=/tmp/kudzu
//! cargo run --release --example kudzuprobe -- mode=cut pre=1200 after=600 out=/tmp/kudzu
//! ```
//!
//! `mode=play` plays whole games headless under a policy (`nothing`,
//! `random` snipping, `planned` snipping where the preview lights most,
//! `dig` the crown out first) and prints one row per seed: wall clear at the
//! start and at dawn, stars, every count the HUD shows, and tick timings
//! (mean, worst, worst while a cut sheet is falling). `pictures=true` also
//! writes the real window's frames (`kudzu::hud::draw_frame`, the window's
//! own drawing) as a GIF, the dawn frame, and the readability pair: the
//! frame before the first cut with no pointer, then with the preview.
//! `mode=collapse` reports the first frame the vine loses 100+ cells at once
//! and what left just before; `mode=trace` sprouts runners by hand and
//! follows them. Tuning knobs are environment variables (`KUDZU_*`, see
//! `kudzu::env_or`).

use pixel_physics::kudzu::{hud, Kudzu, Level, HEIGHT, WIDTH};
use pixel_physics::render::Renderer;

fn arg<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
        .unwrap_or(default)
}

fn save(frame: &[u8], scale: u32, path: &str) {
    let (w, h) = (WIDTH as u32 * scale, HEIGHT as u32 * scale);
    let mut img = image::RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let i = (((y / scale) * WIDTH as u32 + x / scale) * 4) as usize;
            img.put_pixel(x, y, image::Rgba([frame[i], frame[i + 1], frame[i + 2], 255]));
        }
    }
    img.save(path).expect("write png");
}

fn to_image(frame: &[u8], scale: u32) -> image::RgbaImage {
    let (w, h) = (WIDTH as u32 * scale, HEIGHT as u32 * scale);
    let mut img = image::RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let i = (((y / scale) * WIDTH as u32 + x / scale) * 4) as usize;
            img.put_pixel(x, y, image::Rgba([frame[i], frame[i + 1], frame[i + 2], 255]));
        }
    }
    img
}

fn write_gif(shots: Vec<image::RgbaImage>, delay_ms: u32, out: &str) {
    let delay = image::Delay::from_numer_denom_ms(delay_ms, 1);
    let frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
    let n = frames.len();
    let file = std::fs::File::create(out).expect("create gif");
    let mut encoder = image::codecs::gif::GifEncoder::new_with_speed(file, 10);
    encoder.set_repeat(image::codecs::gif::Repeat::Infinite).expect("repeat");
    encoder.encode_frames(frames).expect("encode gif");
    println!("  wrote {out} ({n} frames)");
}

/// The snip that would drop the most vine: every vine cell tried as a blade
/// centre. What a player who reads the preview perfectly would pick.
fn best_cut(game: &Kudzu) -> Option<((i32, i32), usize)> {
    let mut cells: Vec<(i32, i32)> = game.vine_cells().into_iter().collect();
    cells.sort_unstable();
    let mut best: Option<((i32, i32), usize)> = None;
    for &(x, y) in cells.iter().step_by(2) {
        let n = game.preview(x, y).0.len();
        if best.is_none_or(|b| n > b.1) {
            best = Some(((x, y), n));
        }
    }
    best
}

fn cut_mode(seed: u64, pre: u64, after: u64, out: &str) {
    let mut game = Kudzu::new(Level::Walls, seed);
    for _ in 0..pre {
        game.tick();
    }
    let mut renderer = Renderer::new();
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let before = game.vine_cells().len();
    let Some(((x, y), n)) = best_cut(&game) else {
        println!("  no vine to cut");
        return;
    };
    println!("  best cut at {x},{y}: preview says {n} of {before} vine cells let go");
    hud::draw_world(&mut game, &mut renderer, &mut frame);
    to_image(&frame, 3).save(format!("{out}/cut-0-still.png")).unwrap();
    let lit = hud::draw_preview(&game, &mut frame, x, y);
    to_image(&frame, 3).save(format!("{out}/cut-1-preview.png")).unwrap();
    println!("  preview lit {lit}");
    let mut shots = vec![to_image(&frame, 2)];
    game.snip(x, y);
    let t = std::time::Instant::now();
    let mut worst = 0.0f64;
    for f in 1..=after {
        let ft = std::time::Instant::now();
        game.tick();
        worst = worst.max(ft.elapsed().as_secs_f64() * 1000.0);
        if f % 4 == 0 {
            hud::draw_world(&mut game, &mut renderer, &mut frame);
            shots.push(to_image(&frame, 2));
        }
        if [30, 60, 120, 240, 600].contains(&f) {
            println!(
                "  +{f}: vine {} | predicted {} fell {} | green dropped {} | regrown {}",
                game.vine_cells().len(),
                game.stats.predicted,
                game.stats.fell,
                game.stats.green_dropped,
                game.stats.regrown
            );
        }
    }
    println!("  {after} frames after the cut: mean {:.2} ms, worst {:.2} ms", t.elapsed().as_secs_f64() * 1000.0 / after as f64, worst);
    write_gif(shots, 66, &format!("{out}/cut.gif"));
}

/// What the player does each frame, by policy.
#[derive(Clone, Copy, PartialEq)]
enum Policy {
    /// Never touches anything.
    Nothing,
    /// Snips a random vine cell whenever a charge is in hand.
    Random,
    /// Snips where the preview lights the most, whenever a charge is in
    /// hand: what a player who reads the preview perfectly would do.
    Planned,
    /// Digs the crown out at once, then snips as `Planned`.
    Dig,
}

impl Policy {
    fn parse(s: &str) -> Policy {
        match s {
            "nothing" => Policy::Nothing,
            "random" => Policy::Random,
            "planned" => Policy::Planned,
            "dig" => Policy::Dig,
            _ => panic!("policy={s}: nothing|random|planned|dig"),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Policy::Nothing => "nothing",
            Policy::Random => "random",
            Policy::Planned => "planned",
            Policy::Dig => "dig",
        }
    }
}

/// Downsample the window's frame by 2 for a GIF that fits on a phone page.
fn half(frame: &[u8]) -> image::RgbaImage {
    let (w, h) = (hud::FRAME_W / 2, hud::FRAME_H / 2);
    let mut img = image::RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let i = (((y * 2) * hud::FRAME_W + x * 2) * 4) as usize;
            img.put_pixel(x, y, image::Rgba([frame[i], frame[i + 1], frame[i + 2], 255]));
        }
    }
    img
}

fn window_png(frame: &[u8], path: &str) {
    image::save_buffer(path, frame, hud::FRAME_W, hud::FRAME_H, image::ColorType::Rgba8).expect("write png");
}

/// One whole game under a policy. Prints one row; writes the window's GIF
/// and the readability pair when `pictures` is set.
fn play(seed: u64, policy: Policy, pictures: Option<&str>) {
    let t0 = std::time::Instant::now();
    let mut game = Kudzu::new(Level::Walls, seed);
    let overnight_s = t0.elapsed().as_secs_f64();
    let start_clear = game.wall_clear();
    let mut renderer = Renderer::new();
    let mut cells = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let mut frame = vec![0u8; (hud::FRAME_W * hud::FRAME_H * 4) as usize];
    let mut shots = Vec::new();
    let mut rng = pixel_physics::sim::rng::Rng::new(seed ^ 0x5241_4E44);
    let (mut worst, mut sum, mut n) = (0.0f64, 0.0f64, 0u64);
    let (mut worst_fall, mut peak) = (0.0f64, 0usize);
    let mut hover = None;
    let mut fall_until = 0u64;
    let mut first_cut_shown = false;
    let mut decide_ms = 0.0f64;
    let mut fall_shots = Vec::new();
    let mut all_ms: Vec<f64> = Vec::new();
    let mut fall_from: Option<u64> = None;
    while !game.over() {
        // The decision, as a player would make it, outside the timed tick.
        let mut cut = None;
        if policy == Policy::Dig && game.digs > 0 {
            let (cx, cy) = game.layout.crowns[0];
            game.tool = pixel_physics::kudzu::Tool::Dig;
            hover = Some((cx, cy));
            game.hold_dig(cx, cy);
            if game.digs == 0 {
                game.tool = pixel_physics::kudzu::Tool::Snip;
            }
        } else if policy != Policy::Nothing && game.snips > 0 {
            let d = std::time::Instant::now();
            cut = match policy {
                Policy::Random => {
                    let mut v: Vec<(i32, i32)> = game.vine_cells().into_iter().filter(|&(_, y)| y < game.layout.ground_y).collect();
                    v.sort_unstable();
                    (!v.is_empty()).then(|| v[rng.below(v.len() as u32) as usize])
                }
                _ => best_cut(&game).map(|b| b.0),
            };
            decide_ms += d.elapsed().as_secs_f64() * 1000.0;
        }
        if let Some((x, y)) = cut {
            if let (Some(dir), false) = (pictures, first_cut_shown) {
                // The readability pair: the same frame, without and with
                // the pointer where this player is about to cut.
                hud::draw_frame(&mut game, &mut renderer, &mut cells, &mut frame, None);
                window_png(&frame, &format!("{dir}/{}-still.png", policy.name()));
                let lit = hud::draw_frame(&mut game, &mut renderer, &mut cells, &mut frame, Some((x, y)));
                window_png(&frame, &format!("{dir}/{}-preview.png", policy.name()));
                println!("    readability pair at {x},{y}: preview lit {lit}");
                first_cut_shown = true;
            }
            hover = Some((x, y));
            if game.snip(x, y).is_some_and(|k| k > 0) {
                fall_until = game.world.frame + 120;
                if fall_from.is_none() {
                    fall_from = Some(game.played);
                }
            }
        }
        let t = std::time::Instant::now();
        game.tick();
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        worst = worst.max(ms);
        all_ms.push(ms);
        sum += ms;
        n += 1;
        if game.world.frame < fall_until {
            worst_fall = worst_fall.max(ms);
        }
        if game.played % 600 == 0 {
            peak = peak.max(game.vine_cells().len());
        }
        if pictures.is_some() && game.played % 60 == 0 {
            hud::draw_frame(&mut game, &mut renderer, &mut cells, &mut frame, hover);
            shots.push(half(&frame));
        }
        // The first sheet to come down, at real speed: every other frame
        // for two seconds after the cut.
        if pictures.is_some() && first_cut_shown && fall_shots.len() < 60 && game.played % 2 == 0 && fall_from.is_some() {
            hud::draw_frame(&mut game, &mut renderer, &mut cells, &mut frame, hover);
            fall_shots.push(half(&frame));
        }
    }
    let clear = game.wall_clear();
    let s = game.stats;
    all_ms.sort_by(f64::total_cmp);
    let p99 = all_ms.get(all_ms.len() * 99 / 100).copied().unwrap_or(0.0);
    let over = all_ms.iter().filter(|&&m| m > 16.7).count();
    println!(
        "  seed {seed} {:<8} wall clear {:>3.0}% -> {:>3.0}% stars {} | cuts {} predicted {} fell {} green dropped {} regrown {} dug {} | vine {} peak {} | tick mean {:.2} p99 {:.2} worst {:.2} ms ({} over 16.7), worst while falling {:.2} ms | decide {:.0} ms total, overnight {:.1}s",
        policy.name(),
        start_clear * 100.0,
        clear * 100.0,
        Kudzu::stars_for(clear),
        s.cuts,
        s.predicted,
        s.fell,
        s.green_dropped,
        s.regrown,
        s.dug,
        game.vine_cells().len(),
        peak,
        sum / n as f64,
        p99,
        worst,
        over,
        worst_fall,
        decide_ms,
        overnight_s
    );
    if let Some(dir) = pictures {
        hud::draw_frame(&mut game, &mut renderer, &mut cells, &mut frame, None);
        window_png(&frame, &format!("{dir}/{}-dawn.png", policy.name()));
        write_gif(shots, 66, &format!("{dir}/{}.gif", policy.name()));
        if !fall_shots.is_empty() {
            write_gif(fall_shots, 33, &format!("{dir}/{}-first-cut.gif", policy.name()));
        }
    }
}

fn main() {
    let mode: String = arg("mode", "grow".to_string());
    let seed: u64 = arg("seed", 1);
    let frames: u64 = arg("frames", 6000);
    let every: u64 = arg("every", 600);
    let out: String = arg("out", "/tmp/kudzu".to_string());
    println!("kudzuprobe: mode={mode} seed={seed} frames={frames} every={every} out={out}");
    std::fs::create_dir_all(&out).ok();
    if mode == "play" {
        let policies: String = arg("policy", "nothing,random,planned".to_string());
        let seeds: u64 = arg("seeds", 1);
        let pictures: bool = arg("pictures", false);
        for p in policies.split(',').map(Policy::parse) {
            for s in seed..seed + seeds {
                play(s, p, pictures.then_some(out.as_str()));
            }
        }
        return;
    }
    if mode == "collapse" {
        // Watch for the frame the vine loses most of itself, and print what
        // left in the frames just before.
        let mut game = Kudzu::new(Level::Walls, seed);
        let mut prev = game.vine_cells();
        let mut recent: Vec<(u64, Vec<(i32, i32, String)>)> = Vec::new();
        for f in 0..frames {
            game.tick();
            let now = game.vine_cells();
            let lost: Vec<(i32, i32, String)> = prev
                .difference(&now)
                .map(|&(x, y)| {
                    let c = game.world.get(x, y);
                    (x, y, game.world.materials.get(c.material).name.to_string())
                })
                .collect();
            if lost.len() > 100 {
                println!("  frame {f}: lost {} at once; the frames before:", lost.len());
                for (rf, l) in recent.iter().rev().take(6) {
                    println!("    {rf}: {:?}", l.iter().take(12).collect::<Vec<_>>());
                }
                let mut ys: Vec<i32> = lost.iter().map(|l| l.1).collect();
                ys.sort();
                println!("    lost rows {}..{}; now {:?}", ys[0], ys[ys.len() - 1], lost.iter().take(6).collect::<Vec<_>>());
                return;
            }
            if !lost.is_empty() {
                recent.push((f, lost));
            }
            prev = now;
        }
        return;
    }
    if mode == "trace" {
        // Sprout one runner by hand and follow it: where it started, what
        // the cell is each frame, and how the census moved.
        let mut game = Kudzu::new(Level::Walls, seed);
        let id = game.vines()[0];
        for k in 0..arg("n", 5u32) {
            let mut i = k as usize;
            let at = pixel_physics::sim::plant::sprout_climber(&mut game.world, id, |n| { i %= n; i });
            println!("  runner {k} at {at:?} braced {}", at.is_some_and(|(x, y)| pixel_physics::sim::plant::touches_climb_surface(&game.world, x, y)));
            let Some((x, y)) = at else { continue };
            let before = game.world.grow_census[game.vine.0 as usize].clone();
            for f in 0..30 {
                game.tick();
                let c = game.world.get(x, y);
                let t = pixel_physics::sim::organism::cell_type(c.aux());
                if f % 7 == 6 {
                    let after = &game.world.grow_census[game.vine.0 as usize];
                    let dv: Vec<i64> = (0..12).map(|j| after.visits[0][j] as i64 - before.visits[0][j] as i64).collect();
                    let dr: Vec<i64> = (0..12).map(|j| after.retired[0][j] as i64 - before.retired[0][j] as i64).collect();
                    println!("    +{f}: {t:?} carbon {:.2} | shoot visits d {dv:?} retired d {dr:?}", game.world.carbon_at(x, y));
                }
            }
        }
        return;
    }
    if mode == "cut" {
        cut_mode(seed, arg("pre", 1200), arg("after", 600), &out);
        return;
    }
    let t = std::time::Instant::now();
    let mut game = Kudzu::new(Level::Walls, seed);
    println!("  overnight {} frames in {:.1}s", game.layout.overnight, t.elapsed().as_secs_f32());
    let mut renderer = Renderer::new();
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    for f in 0..=frames {
        if f % every == 0 {
            let cells = game.vine_cells().len();
            println!("  frame {f}: vine cells {cells}, vines {}, wall clear {:.0}%, runners {}", game.vines().len(), game.wall_clear() * 100.0, game.runners);
            for id in game.vines() {
                let s = game.world.organism(id).unwrap();
                let carbon: Vec<f32> = s.cells.keys().map(|&(x, y)| game.world.carbon_at(x, y)).collect();
                let max = carbon.iter().cloned().fold(0.0f32, f32::max);
                let sum: f32 = carbon.iter().sum();
                println!("    carbon max {max:.2} mean {:.3} tips {}", sum / carbon.len().max(1) as f32, game.world.organism_active_tip_count(id, pixel_physics::sim::organism::CellType::GrowingTip));
            }
            if let Some(c) = game.world.grow_census.get(game.vine.0 as usize) {
                let names = ["notasked", "organ", "poor", "height", "taper", "rootshare", "tipcap", "boxed", "hard", "nodir", "grew", "flowered"];
                for kind in 0..2 {
                    let v: Vec<String> = names.iter().enumerate().filter(|(i, _)| c.visits[kind][*i] > 0).map(|(i, n)| format!("{n} {}", c.visits[kind][i])).collect();
                    let r: Vec<String> = names.iter().enumerate().filter(|(i, _)| c.retired[kind][*i] > 0).map(|(i, n)| format!("{n} {}", c.retired[kind][i])).collect();
                    println!("    {} visits: {} | retired: {}", if kind == 0 { "shoot" } else { "root" }, v.join(", "), r.join(", "));
                }
            }
            for id in game.vines() {
                let s = game.world.organism(id).unwrap();
                let mut types = std::collections::BTreeMap::new();
                for &(x, y) in s.cells.keys() {
                    let t = pixel_physics::sim::organism::cell_type(game.world.get(x, y).aux());
                    *types.entry(format!("{t:?}")).or_insert(0) += 1;
                }
                println!("    types {types:?}");
            }
            for id in game.vines() {
                let s = game.world.organism(id).unwrap();
                println!(
                    "    #{id} shoot {} root {} anchors {} income {:.2} maint {:.2} unpaid {:.2} starving {} water {:.2} nutr {:.2} age {} senescent {} starved_cells {}",
                    s.shoot_cells, s.root_cells, s.anchor_cells, s.income, s.maintenance, s.maintenance_unpaid, s.starving_ticks,
                    s.water_status, s.nutrient_status, s.age_ticks, s.senescent, s.starved_cells
                );
            }
            hud::draw_world(&mut game, &mut renderer, &mut frame);
            save(&frame, 2, &format!("{out}/grow-{f:06}.png"));
        }
        game.tick();
    }
}
