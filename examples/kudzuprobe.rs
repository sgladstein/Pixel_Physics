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
//! `random` snipping, `planned` snipping where the preview lights most --
//! falls plus wilts, or falls only with `KUDZU_PLAN_FALL_ONLY=1`, the plan
//! before WILTS was shown -- `dig` the crown out first, and the 2026-11-08
//! playtest's players: `roots` cuts at the soil nearest the crown, `fresh`
//! tries 6 spots near a random patch once a second, `digonly`, `wait` does
//! nothing until 25 s are left) and prints one row per seed: wall clear at the
//! start and at dawn, stars, every count the HUD shows, and tick timings
//! (mean, worst, worst while a cut sheet is falling). `pictures=true` also
//! writes the real window's frames (`kudzu::hud::draw_frame`, the window's
//! own drawing) as a GIF, the dawn frame, and the readability pair: the
//! frame before the first cut with no pointer, then with the preview.
//! `mode=hover at=X,Y` draws the readability pair at one spot at the start
//! of play, times the preview, and with `after=N` cuts there and writes
//! what follows as a GIF. `mode=rootcut cuts=N` (or `dig=true`) parts the
//! vine at the crown and follows every cell parted from the roots;
//! `mode=promise` follows every WILTS promise the planned player makes and
//! says why the cells that lived did.
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

/// How much of the vine a snip here takes, as a preview-reader counts it:
/// what falls now plus what wilts, or only what falls with
/// `KUDZU_PLAN_FALL_ONLY=1` (the plan before WILTS was shown).
fn preview_score(game: &Kudzu, x: i32, y: i32) -> usize {
    let p = game.preview(x, y);
    if pixel_physics::kudzu::env_or("KUDZU_PLAN_FALL_ONLY", 0) == 1 {
        p.fall.len()
    } else {
        p.fall.len() + p.wilt.len()
    }
}

/// The snip that would take the most vine: every other vine cell tried as a
/// blade centre. What a player who reads the preview perfectly would pick.
fn best_cut(game: &Kudzu) -> Option<((i32, i32), usize)> {
    let mut cells: Vec<(i32, i32)> = game.vine_cells().into_iter().collect();
    cells.sort_unstable();
    let mut best: Option<((i32, i32), usize)> = None;
    for &(x, y) in cells.iter().step_by(2) {
        let n = preview_score(game, x, y);
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
    println!("  preview lit {lit:?}");
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

/// One WILTS promise followed by `mode=promise`: when it was made, its
/// cells, the vine as it stood, and where the cut was.
type Promise = (u64, Vec<(i32, i32)>, pixel_physics::sim::fxhash::PosSet, (i32, i32));

/// Cells that left the vine in one frame: position and what is there now.
type Lost = Vec<(i32, i32, String)>;

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
    /// Digs the crown out at once and does nothing else (the playtest's
    /// "dig only").
    DigOnly,
    /// Cuts the vine just above the soil, nearest the crown first: parts
    /// the shoot from its roots (the 2026-11-08 playtest's "cut at roots").
    Roots,
    /// The playtest's fresh player: once a second, picks a patch of vine at
    /// random, tries the pointer on 6 spots near it, and clicks whichever
    /// the preview lights most (falls plus wilts).
    Fresh,
    /// Does nothing until [`WAIT_LAST`] frames are left, then snips as
    /// `Planned` (the playtest's "wait" cheese).
    Wait,
}

/// The `wait` policy starts cutting with this many frames to dawn.
const WAIT_LAST: u64 = 25 * 60;

/// The `roots` player's cut: the vine cell just above the soil nearest the
/// crown, among those a blade there would touch.
fn root_cut(game: &Kudzu) -> Option<(i32, i32)> {
    let g = game.layout.ground_y;
    let (cx, _) = game.layout.crowns[0];
    let mut v: Vec<(i32, i32)> = game.vine_cells().into_iter().filter(|&(_, y)| y < g && y >= g - 2).collect();
    v.sort_unstable_by_key(|&(x, y)| ((x - cx).abs(), -y, x));
    v.first().copied()
}

impl Policy {
    fn parse(s: &str) -> Policy {
        match s {
            "nothing" => Policy::Nothing,
            "random" => Policy::Random,
            "planned" => Policy::Planned,
            "dig" => Policy::Dig,
            "roots" => Policy::Roots,
            "digonly" => Policy::DigOnly,
            "fresh" => Policy::Fresh,
            "wait" => Policy::Wait,
            _ => panic!("policy={s}: nothing|random|planned|dig|digonly|roots|fresh|wait"),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Policy::Nothing => "nothing",
            Policy::Random => "random",
            Policy::Planned => "planned",
            Policy::Dig => "dig",
            Policy::Roots => "roots",
            Policy::DigOnly => "digonly",
            Policy::Fresh => "fresh",
            Policy::Wait => "wait",
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
    // The first frame (to the half second) the walls stood 95% clear: how
    // soon this player had three stars in hand.
    let mut win_at: Option<u64> = None;
    while !game.over() {
        // The decision, as a player would make it, outside the timed tick.
        let mut cut = None;
        if matches!(policy, Policy::Dig | Policy::DigOnly) && game.digs > 0 {
            let (cx, cy) = game.layout.crowns[0];
            game.tool = pixel_physics::kudzu::Tool::Dig;
            hover = Some((cx, cy));
            game.hold_dig(cx, cy);
            if game.digs == 0 {
                game.tool = pixel_physics::kudzu::Tool::Snip;
            }
        } else if !matches!(policy, Policy::Nothing | Policy::DigOnly)
            && game.snips > 0
            && (policy != Policy::Wait || game.left() <= WAIT_LAST)
            && (policy != Policy::Fresh || game.played.is_multiple_of(60))
        {
            let d = std::time::Instant::now();
            cut = match policy {
                Policy::Roots => root_cut(&game),
                Policy::Fresh => {
                    let mut v: Vec<(i32, i32)> = game.vine_cells().into_iter().filter(|&(_, y)| y < game.layout.ground_y).collect();
                    v.sort_unstable();
                    (!v.is_empty()).then(|| {
                        let (px, py) = v[rng.below(v.len() as u32) as usize];
                        (0..6)
                            .map(|_| (px + rng.below(13) as i32 - 6, py + rng.below(13) as i32 - 6))
                            .max_by_key(|&(x, y)| preview_score(&game, x, y))
                            .unwrap_or((px, py))
                    })
                }
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
                println!("    readability pair at {x},{y}: preview lit {} fall {} wilt", lit.fall, lit.wilt);
                first_cut_shown = true;
            }
            hover = Some((x, y));
            let parts = game.stats.wilt_predicted;
            if game.snip(x, y).is_some_and(|k| k > 0) || game.stats.wilt_predicted > parts {
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
        if game.played.is_multiple_of(600) {
            peak = peak.max(game.vine_cells().len());
        }
        if win_at.is_none() && game.played.is_multiple_of(30) && game.wall_clear() >= 0.95 {
            win_at = Some(game.played);
        }
        if pictures.is_some() && game.played.is_multiple_of(60) {
            hud::draw_frame(&mut game, &mut renderer, &mut cells, &mut frame, hover);
            shots.push(half(&frame));
        }
        // The first sheet to come down, at real speed: every other frame
        // for two seconds after the cut.
        if pictures.is_some() && first_cut_shown && fall_shots.len() < 60 && game.played.is_multiple_of(2) && fall_from.is_some() {
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
        "  seed {seed} {:<8} wall clear {:>3.0}% -> {:>3.0}% stars {} | cuts {} predicted {} fell {} wilt promised {} judged {} wilted {} | 95% clear at {} | green dropped {} regrown {} dug {} | vine {} peak {} | tick mean {:.2} p99 {:.2} worst {:.2} ms ({} over 16.7), worst while falling {:.2} ms | decide {:.0} ms total, overnight {:.1}s",
        policy.name(),
        start_clear * 100.0,
        clear * 100.0,
        Kudzu::stars_for(clear),
        s.cuts,
        s.predicted,
        s.fell,
        s.wilt_predicted,
        s.wilt_judged,
        s.wilted,
        win_at.map_or("never".to_string(), |f| format!("{:.1}s", f as f64 / 60.0)),
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
        let mut recent: Vec<(u64, Lost)> = Vec::new();
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
            let before = game.world.grow_census[game.vine.0 as usize];
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
    if mode == "hover" {
        // The readability pair at a chosen spot, at the start of play, in
        // the window's own drawing; then `after` frames of what the cut
        // does, as a GIF.
        let at: String = arg("at", "87,127".to_string());
        let (x, y) = at.split_once(',').map(|(a, b)| (a.parse().unwrap(), b.parse().unwrap())).expect("at=x,y");
        let tag: String = arg("tag", "hover".to_string());
        let mut game = Kudzu::new(Level::Walls, seed);
        let mut renderer = Renderer::new();
        let mut cells = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        let mut frame = vec![0u8; (hud::FRAME_W * hud::FRAME_H * 4) as usize];
        hud::draw_frame(&mut game, &mut renderer, &mut cells, &mut frame, None);
        window_png(&frame, &format!("{out}/{tag}-still.png"));
        let lit = hud::draw_frame(&mut game, &mut renderer, &mut cells, &mut frame, Some((x, y)));
        window_png(&frame, &format!("{out}/{tag}-preview.png"));
        println!("  seed {seed} at {x},{y}: preview lit {lit:?}, walls clear {:.0}%", game.wall_clear() * 100.0);
        // What the preview costs a frame while the pointer is over the vine.
        let t = std::time::Instant::now();
        for _ in 0..50 {
            std::hint::black_box(game.preview(x, y));
        }
        println!("  preview: {:.2} ms a frame at {} vine cells", t.elapsed().as_secs_f64() * 1000.0 / 50.0, game.vine_cells().len());
        let after: u64 = arg("after", 0);
        if after > 0 {
            let mut shots: Vec<image::RgbaImage> = (0..15).map(|_| half(&frame)).collect();
            game.snip(x, y);
            for f in 1..=after {
                game.tick();
                if f % 6 == 0 {
                    hud::draw_frame(&mut game, &mut renderer, &mut cells, &mut frame, Some((x, y)));
                    shots.push(half(&frame));
                }
            }
            hud::draw_frame(&mut game, &mut renderer, &mut cells, &mut frame, None);
            window_png(&frame, &format!("{out}/{tag}-after.png"));
            println!("  {after} frames after the cut: walls clear {:.0}%, vine {}", game.wall_clear() * 100.0, game.vine_cells().len());
            write_gif(shots, 100, &format!("{out}/{tag}.gif"));
        }
        return;
    }
    if mode == "promise" {
        // Every WILTS promise of the planned player, followed: how many of
        // its cells are still vine 15 s on, and why -- joined to a root
        // again, or in a piece that now has roots of its own, or neither.
        let mut game = Kudzu::new(Level::Walls, seed);
        let mut open: Vec<Promise> = Vec::new();
        while !game.over() {
            if game.snips > 0 {
                if let Some(((x, y), _)) = best_cut(&game) {
                    let p = game.preview(x, y);
                    if arg("verbose", false) {
                        let g = game.layout.ground_y;
                        let hi_root: Vec<(i32, i32, String, String)> = game
                            .vine_cells()
                            .into_iter()
                            .filter(|&(_, y)| y < g)
                            .filter(|&(x, y)| {
                                let c = game.world.get(x, y);
                                game.world.materials.get(c.material).reinforces_powder
                                    || pixel_physics::sim::organism::cell_type(c.aux()) == Some(pixel_physics::sim::organism::CellType::RootTip)
                            })
                            .map(|(x, y)| {
                                let c = game.world.get(x, y);
                                (x, y, game.world.materials.get(c.material).name.to_string(), format!("{:?}", pixel_physics::sim::organism::cell_type(c.aux())))
                            })
                            .collect();
                        println!("  play {} cut at {x},{y}: fall {} wilt {} | root-test cells above ground {} e.g. {:?}", game.played, p.fall.len(), p.wilt.len(), hi_root.len(), hi_root.iter().take(5).collect::<Vec<_>>());
                    }
                    let v0 = game.vine_cells();
                    game.snip(x, y);
                    if !p.wilt.is_empty() {
                        open.push((game.played, p.wilt, v0, (x, y)));
                    }
                }
            }
            game.tick();
            open.retain(|(at, cells, v0, cut)| {
                if game.played < at + 900 {
                    return true;
                }
                let vine = game.vine_cells();
                let left: Vec<(i32, i32)> = cells.iter().copied().filter(|q| vine.contains(q)).collect();
                // Flood from roots (cells in soil) through the vine now.
                let g = game.layout.ground_y;
                let mut reached: std::collections::HashSet<(i32, i32)> = vine.iter().copied().filter(|&(_, y)| y >= g).collect();
                let mut stack: Vec<(i32, i32)> = reached.iter().copied().collect();
                while let Some((x, y)) = stack.pop() {
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            let q = (x + dx, y + dy);
                            if vine.contains(&q) && reached.insert(q) {
                                stack.push(q);
                            }
                        }
                    }
                }
                let rejoined = left.iter().filter(|q| reached.contains(q)).count();
                // Survivors touching tissue grown since the cut: the bridge.
                let promised: std::collections::HashSet<(i32, i32)> = cells.iter().copied().collect();
                let mut bridge: Vec<(i32, i32)> = Vec::new();
                for &(x, y) in &left {
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            let q = (x + dx, y + dy);
                            if vine.contains(&q) && !v0.contains(&q) && !promised.contains(&q) {
                                bridge.push(q);
                            }
                        }
                    }
                }
                bridge.sort_unstable();
                bridge.dedup();
                let in_soil = bridge.iter().filter(|&&(_, y)| y >= g).count();
                let soil_roots_near = bridge.len();
                if !left.is_empty() {
                    println!("    cut was at {cut:?}; new tissue touching survivors {} ({in_soil} of it in the soil) e.g. {:?}", bridge.len(), bridge.iter().take(6).collect::<Vec<_>>());
                }
                println!(
                    "  cut at play {:.1}s: promised {} still vine {} (joined to a root now {}, new tissue touching them {}) e.g. {:?}",
                    *at as f64 / 60.0, cells.len(), left.len(), rejoined, soil_roots_near, left.iter().take(4).collect::<Vec<_>>()
                );
                false
            });
        }
        return;
    }
    if mode == "rootcut" {
        // The playtest's root cut, followed: one organism or two, and who
        // starves.
        let mut game = Kudzu::new(Level::Walls, seed);
        let cuts: u32 = arg("cuts", 1);
        if arg("dig", false) {
            let (cx, cy) = game.layout.crowns[0];
            game.tool = pixel_physics::kudzu::Tool::Dig;
            while game.digs > 0 {
                game.hold_dig(cx, cy);
            }
            game.tool = pixel_physics::kudzu::Tool::Snip;
            let g = game.layout.ground_y;
            let below: Vec<_> = game.vine_cells().into_iter().filter(|&(_, y)| y >= g).collect();
            println!("  dug: vine cells left in soil {} e.g. {:?}", below.len(), below.iter().take(5).map(|&(x, y)| (x, y, game.world.materials.get(game.world.get(x, y).material).name.to_string())).collect::<Vec<_>>());
        }
        for k in 0..cuts {
            let Some((x, y)) = root_cut(&game) else { break };
            game.snips = 3;
            let n = game.snip(x, y);
            println!("  cut {k} at {x},{y}: preview fell {n:?}");
        }
        // Reachable from the roots (cells in soil), eight-connected through
        // the vine's own tissue.
        let all = game.vine_cells();
        let g = game.layout.ground_y;
        let mut reached: std::collections::HashSet<(i32, i32)> = all.iter().copied().filter(|&(_, y)| y >= g).collect();
        let mut stack: Vec<(i32, i32)> = reached.iter().copied().collect();
        while let Some((x, y)) = stack.pop() {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let q = (x + dx, y + dy);
                    if all.contains(&q) && reached.insert(q) {
                        stack.push(q);
                    }
                }
            }
        }
        let cut_off: std::collections::HashSet<(i32, i32)> = all.iter().copied().filter(|p| !reached.contains(p)).collect();
        println!("  vine {} reached from roots {} cut off {}", all.len(), reached.len(), cut_off.len());
        let on_soil: Vec<(i32, i32)> = cut_off.iter().copied().filter(|&(x, y)| y == g - 1 && game.world.get(x, y + 1).organism_id() == 0).collect();
        println!("  cut-off cells lying on bare soil: {}", on_soil.len());
        let mut mats: std::collections::BTreeMap<String, u32> = Default::default();
        for f in 0..frames {
            if f % every == 0 {
                let now = game.vine_cells();
                let off_left = cut_off.iter().filter(|p| now.contains(p)).count();
                let on_left = reached.iter().filter(|p| now.contains(p)).count();
                let soil_left = on_soil.iter().filter(|p| now.contains(p)).count();
                println!("  +{f} on-soil cut-off left {soil_left} | cut-off still vine {off_left}/{} | rooted still vine {on_left}/{} | gone as {mats:?}", cut_off.len(), reached.len());
                for id in game.vines() {
                    let s = game.world.organism(id).unwrap();
                    println!(
                        "  +{f} #{id} cells {} shoot {} root {} contact_root {} collar {:?} starving {} senescent {} water {:.2} income {:.3} maint {:.3}",
                        s.cells.len(), s.shoot_cells, s.root_cells, s.contact_root_cells, s.collar_y, s.starving_ticks, s.senescent, s.water_status, s.income, s.maintenance
                    );
                }
            }
            let before = game.vine_cells();
            game.tick();
            let now = game.vine_cells();
            for &(x, y) in before.difference(&now) {
                let c = game.world.get(x, y);
                let key = format!("{}{}", if cut_off.contains(&(x, y)) { "off:" } else { "on:" }, game.world.materials.get(c.material).name);
                *mats.entry(key).or_insert(0) += 1;
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
