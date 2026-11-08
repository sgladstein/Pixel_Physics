//! **The Mould probe's harness**: scripted play over paired seeds, the
//! counts, GIFs of the window's own frame, and the frame cost.
//!
//! ```text
//! cargo run --release --example mouldprobe                         # 5 seeds x {do nothing, random, plan}
//! cargo run --release --example mouldprobe -- seeds=1,2 gif=out/   # ...and a GIF per run
//! cargo run --release --example mouldprobe -- lean=1               # oats-to-lean and lamp recoil timing only
//! cargo run --release --example mouldprobe -- look=out/ seeds=1    # PNG stills every 10 s, do nothing and plan
//! cargo run --release --example mouldprobe -- divide=0.05 feed_reach=60   # any Params field by name
//! cargo run --release --example mouldprobe -- looks=out/ seeds=1  # the interface looks, same moments in each (mould::look)
//! cargo run --release --example mouldprobe -- pull=1 seeds=1,2,3,4,5  # how far oats pull: the oats preview's ring
//! cargo run --release --example mouldprobe -- lesson=out/ style=B     # any picture mode, drawn in an interface look
//! cargo run --release --example mouldprobe -- wander=1 seeds=1,2,3,4,5,6,7,8 gif=out/   # the Wanderer: arrived N of M per hand
//! ```
//!
//! Everything it draws goes through `mould::render::draw`, the window's own
//! frame function, so a picture here is what a player sees.

use std::time::Instant;

use pixel_physics::mould::play::{Dump, Fresh, Idle, IdleGo, LampOnly, Leader, Plan, Player, Random, Shepherd};
use pixel_physics::mould::wander::{Outcome, FIRST_WALKERS};
use pixel_physics::mould::look::Style;
use pixel_physics::mould::render::{self, View};
use pixel_physics::mould::{Dish, Ground, Params, Phase, Score, Tool, LAMP_R, STEPS_PER_FRAME, STEPS_PER_SECOND};

struct Args {
    seeds: Vec<u64>,
    players: Vec<String>,
    gif: Option<String>,
    look: Option<String>,
    lean: bool,
    verbs: Option<String>,
    push: bool,
    flicker: bool,
    lesson: Option<String>,
    /// `pull=1`: how far from the mould's front oats still pull it.
    pull: bool,
    /// `looks=DIR`: the interface looks' scripted frames (`mould::look`).
    looks: Option<String>,
    /// `style=NAME|LETTER`: the interface look every picture is drawn in.
    style: Style,
    cut: Option<String>,
    /// `push`: how far beside the tube the lamp is held.
    offset: f32,
    scale: u32,
    every: u32,
    /// Oats per dish, if not the game's own stock.
    oats: Option<u32>,
    /// The Wanderer (`mould::wander`): journeys of `dishes` stretches.
    wander: bool,
    dishes: u32,
    /// Minutes of dish before a stretch is called (nobody pressed Go and she
    /// is still alive).
    cap_min: u32,
    params: Params,
}

fn parse() -> Args {
    let mut a = Args {
        seeds: vec![1, 2, 3, 4, 5],
        players: ["idle", "random", "plan", "fresh", "shepherd", "lamponly", "dump"]
            .map(String::from)
            .to_vec(),
        gif: None,
        look: None,
        lean: false,
        verbs: None,
        push: false,
        flicker: false,
        lesson: None,
        pull: false,
        looks: None,
        style: Style::Classic,
        cut: None,
        offset: 12.0,
        scale: 2,
        every: 15,
        oats: None,
        wander: false,
        dishes: 3,
        cap_min: 10,
        params: Params::default(),
    };
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else {
            panic!("unknown argument {arg}")
        };
        let f = || v.parse::<f32>().unwrap_or_else(|_| panic!("{k}: not a number"));
        let p = &mut a.params;
        match k {
            "seeds" => a.seeds = v.split(',').map(|s| s.parse().unwrap()).collect(),
            "players" => a.players = v.split(',').map(String::from).collect(),
            "gif" => a.gif = Some(v.into()),
            "look" => a.look = Some(v.into()),
            "lean" => a.lean = v == "1",
            "verbs" => a.verbs = Some(v.into()),
            "bend" => a.push = v == "1",
            "flicker" => a.flicker = v == "1",
            "lesson" => a.lesson = Some(v.into()),
            "pull" => a.pull = v == "1",
            "looks" => a.looks = Some(v.into()),
            "style" => a.style = Style::parse(v).unwrap_or_else(|| panic!("style: unknown {v}")),
            "cut" => a.cut = Some(v.into()),
            "scale" => a.scale = v.parse().unwrap(),
            "every" => a.every = v.parse().unwrap(),
            "oats" => a.oats = Some(v.parse().unwrap()),
            "wander" => a.wander = v == "1",
            "dishes" => a.dishes = v.parse().unwrap(),
            "cap" => a.cap_min = v.parse().unwrap(),
            "sensor_angle" => p.sensor_angle = f().to_radians(),
            "rotate_angle" => p.rotate_angle = f().to_radians(),
            "sensor_offset" => p.sensor_offset = f(),
            "deposit" => p.deposit = f(),
            "keep" => p.keep = f(),
            "scent_keep" => p.scent_keep = f(),
            "food_deposit" => p.food_deposit = f(),
            "light_weight" => p.light_weight = f(),
            "light_fade" => p.light_fade = f(),
            "tube" => p.tube = f(),
            "feed_reach" => p.feed_reach = f() as u16,
            "upkeep" => p.upkeep = f(),
            "eat" => p.eat = f(),
            "table" => p.table = f() as u32,
            "fed_scent" => p.fed_scent = f(),
            "divide" => p.divide = f(),
            "crowd" => p.crowd = f() as u32,
            "starve" => p.starve = f(),
            "hungry" => p.hungry = f(),
            "burn" => p.burn = f(),
            "lamp_reach" => p.lamp_reach = f(),
            "lamp_soft" => p.lamp_soft = v == "1",
            "push" => p.push = f(),
            "pieces" => p.pieces = v == "1",
            "wound" => p.wound = f(),
            "offset" => a.offset = f(),
            _ => panic!("unknown argument {k}"),
        }
    }
    a
}

fn player(name: &str, seed: u64) -> Box<dyn Player> {
    match name {
        "idle" => Box::new(Idle),
        "random" => Box::new(Random::new(seed)),
        "plan" => Box::new(Plan::new()),
        "fresh" => Box::new(Fresh::new()),
        "shepherd" => Box::new(Shepherd),
        "lamponly" => Box::new(LampOnly::new()),
        "dump" => Box::new(Dump),
        _ => panic!("unknown player {name}"),
    }
}

fn frame_of(d: &Dish, scale: u32, style: Style) -> image::RgbaImage {
    let (w, h) = render::frame_size(scale);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let view = View {
        keys: false,
        style,
        ..View::default()
    };
    render::draw(d, &view, scale, &mut buf);
    image::RgbaImage::from_raw(w, h, buf).expect("frame size")
}

fn write_gif(path: &str, shots: Vec<image::RgbaImage>, every_frames: u32) {
    let delay = image::Delay::from_numer_denom_ms(every_frames * 1000 / 60 / 3, 1);
    let frames: Vec<image::Frame> = shots
        .into_iter()
        .map(|img| image::Frame::from_parts(img, 0, 0, delay))
        .collect();
    let file = std::fs::File::create(path).expect("gif file");
    let mut enc = image::codecs::gif::GifEncoder::new_with_speed(file, 10);
    enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
    enc.encode_frames(frames).expect("gif encode");
}

struct Run {
    score: Score,
    births: u64,
    starved: u64,
    hungry: u64,
    burned: u64,
    oats: u32,
    snips: u32,
    lamp_s: f32,
    /// Joins flashed (`Dish::snaps`).
    joins: u32,
    peak: usize,
    /// (seconds, walkers, piles joined) every 10 s.
    trace: Vec<(u32, usize, u32)>,
}

fn run(seed: u64, who: &str, a: &Args) -> Run {
    let mut d = Dish::with_params(seed, a.params.clone());
    if let Some(n) = a.oats {
        d.oats_left = n;
    }
    let mut p = player(who, seed);
    let mut shots = Vec::new();
    let mut trace = Vec::new();
    let mut frame = 0u32;
    let mut peak = 0;
    // GIF: through the storm and a few seconds of the score panel.
    let end = Dish::dish_steps() + Dish::storm_steps() + 3 * STEPS_PER_SECOND as u64;
    while d.t < end {
        p.act(&mut d);
        for _ in 0..STEPS_PER_FRAME {
            d.step();
        }
        peak = peak.max(d.walkers.len());
        if a.gif.is_some() && frame.is_multiple_of(a.every) {
            shots.push(frame_of(&d, a.scale, a.style));
        }
        // Stills also just after each late pile turns up and during the
        // storm warning, the moments the 2026-11-08 fixes draw.
        let sps = STEPS_PER_SECOND as u64;
        if d.t.is_multiple_of(sps) && [31, 56, 87].contains(&(d.t / sps)) {
            if let Some(dir) = &a.look {
                frame_of(&d, a.scale, a.style)
                    .save(format!("{dir}/look-s{seed}-{who}-{:03}.png", d.t / sps))
                    .unwrap();
            }
        }
        if d.t.is_multiple_of(10 * STEPS_PER_SECOND as u64) {
            let s = d.measure();
            trace.push(((d.t / STEPS_PER_SECOND as u64) as u32, d.walkers.len(), s.reach));
            if let Some(dir) = &a.look {
                frame_of(&d, a.scale, a.style)
                    .save(format!(
                        "{dir}/look-s{seed}-{who}-{:03}.png",
                        d.t / STEPS_PER_SECOND as u64
                    ))
                    .unwrap();
            }
        }
        frame += 1;
    }
    assert_eq!(d.phase, Phase::Over);
    if let Some(dir) = &a.gif {
        write_gif(&format!("{dir}/mould-s{seed}-{who}.gif"), shots, a.every);
        frame_of(&d, a.scale, a.style)
            .save(format!("{dir}/mould-s{seed}-{who}-end.png"))
            .unwrap();
    }
    let c = &d.counts;
    Run {
        score: d.score.clone().unwrap(),
        births: c.births,
        starved: c.starved,
        hungry: c.hungry,
        burned: c.burned,
        oats: c.oats_placed,
        snips: c.snips_used,
        lamp_s: c.lamp_steps as f32 / STEPS_PER_SECOND as f32,
        joins: c.snaps,
        peak,
        trace,
    }
}

/// Two-sided sign test: p of a split at least this uneven, ties dropped.
fn sign_p(wins: u32, losses: u32) -> f64 {
    let n = wins + losses;
    if n == 0 {
        return 1.0;
    }
    let k = wins.min(losses);
    let mut c = 1.0f64;
    let mut sum = 0.0;
    for i in 0..=n {
        if i <= k {
            sum += c;
        }
        c = c * (n - i) as f64 / (i + 1) as f64;
    }
    (2.0 * sum / 2f64.powi(n as i32)).min(1.0)
}

/// The front of the mould: the walker farthest from home.
fn front(d: &Dish) -> (f32, f32) {
    let (hx, hy) = (d.piles[0].x, d.piles[0].y);
    d.walkers
        .iter()
        .map(|w| (w.x, w.y))
        .max_by(|a, b| (a.0 - hx).hypot(a.1 - hy).total_cmp(&(b.0 - hx).hypot(b.1 - hy)))
        .unwrap()
}

fn near(d: &Dish, (x, y): (f32, f32), r: f32) -> usize {
    d.walkers.iter().filter(|w| (w.x - x).hypot(w.y - y) <= r).count()
}

/// Oats to a visible lean, and lamp to a visible recoil, each against the
/// same dish with the tool withheld.
fn lean(a: &Args) {
    println!("\n== oats to lean, lamp to recoil (paired: the same dish with and without the tool) ==");
    println!("lean = the oats arm has 15+ more mould within 15 cells of the oats than the no-oats arm");
    println!("recoil = mould under the lamp's bright core falls to half the dark arm's");
    for &seed in &a.seeds {
        for t0s in [10u32, 25] {
            let mut base = Dish::with_params(seed, a.params.clone());
            while base.t < (t0s * STEPS_PER_SECOND) as u64 {
                base.step();
            }
            // Oats 20 cells beyond the front, on open dish.
            let (hx, hy) = (base.piles[0].x, base.piles[0].y);
            let (fx, fy) = front(&base);
            let len = (fx - hx).hypot(fy - hy).max(1.0);
            let spot = (fx + (fx - hx) / len * 20.0, fy + (fy - hy) / len * 20.0);
            let mut with = base.clone();
            let placed = with.place_oats(spot.0, spot.1);
            let mut without = base.clone();
            let mut lean_at = None;
            let mut first_touch = None;
            for k in 1..=(10 * STEPS_PER_SECOND) {
                with.step();
                without.step();
                if k % 6 == 0 {
                    let (nw, no) = (near(&with, spot, 15.0), near(&without, spot, 15.0));
                    if lean_at.is_none() && nw >= no + 15 {
                        lean_at = Some(k as f32 / STEPS_PER_SECOND as f32);
                    }
                    if first_touch.is_none() && near(&with, spot, 4.0) > 0 {
                        first_touch = Some(k as f32 / STEPS_PER_SECOND as f32);
                    }
                }
            }
            let fmt = |o: Option<f32>| o.map_or("never (10 s)".to_string(), |s| format!("{s:.2} s"));
            println!(
                "seed {seed} at {t0s:>2} s: oats {} cells past the front (placed {placed}): lean {}, first walker on the oats {}",
                20,
                fmt(lean_at),
                fmt(first_touch)
            );
            // Lamp recoil: on the thickest part of the mould near the front.
            let spot = ((fx + hx) / 2.0, (fy + hy) / 2.0);
            let mut lit = base.clone();
            lit.set_lamp(Some(spot));
            let mut dark = base.clone();
            let mut recoil_at = None;
            let start = near(&dark, spot, LAMP_R * 0.5);
            for k in 1..=(3 * STEPS_PER_SECOND) {
                lit.step();
                dark.step();
                if recoil_at.is_none() && near(&lit, spot, LAMP_R * 0.5) * 2 <= near(&dark, spot, LAMP_R * 0.5) {
                    recoil_at = Some(k as f32 / STEPS_PER_SECOND as f32);
                }
            }
            println!("              lamp on {start} walkers: recoil {}", fmt(recoil_at));
        }
    }
}

/// The thickest tube cell away from every pile, and the unit normal to the
/// line from home through it (roughly across the tube).
fn trunk(d: &Dish) -> ((f32, f32), (f32, f32)) {
    let w = pixel_physics::mould::W;
    let mut best = (0.0f32, 0usize);
    for (i, &t) in d.trail.iter().enumerate() {
        let (x, y) = ((i % w) as f32, (i / w) as f32);
        if t > best.0 && d.piles.iter().all(|p| (p.x - x).hypot(p.y - y) > p.r + LAMP_R + 4.0) {
            best = (t, i);
        }
    }
    let (tx, ty) = ((best.1 % w) as f32 + 0.5, (best.1 / w) as f32 + 0.5);
    let (hx, hy) = (d.piles[0].x, d.piles[0].y);
    let len = (tx - hx).hypot(ty - hy).max(1.0);
    ((tx, ty), (-(ty - hy) / len, (tx - hx) / len))
}

/// Trail-weighted offset of the tube along `n`, within 14 cells of `at`.
fn tube_offset(d: &Dish, at: (f32, f32), n: (f32, f32)) -> f32 {
    let w = pixel_physics::mould::W;
    let (mut sum, mut wsum) = (0.0f32, 0.0f32);
    for (i, &t) in d.trail.iter().enumerate() {
        if t <= d.params.tube {
            continue;
        }
        let (x, y) = ((i % w) as f32 + 0.5, (i / w) as f32 + 0.5);
        if (x - at.0).hypot(y - at.1) <= 14.0 {
            sum += t * ((x - at.0) * n.0 + (y - at.1) * n.1);
            wsum += t;
        }
    }
    if wsum > 0.0 {
        sum / wsum
    } else {
        f32::NAN
    }
}

/// **Does the lamp push or erase?** (playtest 2026-11-08, fix 1). Hold the
/// lamp 12 cells beside the thickest tube for 3 s, against the same dish
/// with no lamp: how far the tube moved away, whether it is still whole
/// (food joined), and how much mould the light killed.
fn push(a: &Args) {
    println!(
        "\n== lamp held {} cells beside the trunk for 3 s, against the no-lamp twin ==",
        a.offset
    );
    for &seed in &a.seeds {
        let mut base = Dish::with_params(seed, a.params.clone());
        while base.t < (25 * STEPS_PER_SECOND) as u64 {
            base.step();
        }
        let (at, n) = trunk(&base);
        let (mut lit, mut dark) = (base.clone(), base.clone());
        lit.set_lamp(Some((at.0 + n.0 * a.offset, at.1 + n.1 * a.offset)));
        let j0 = base.measure().reach;
        let mut min_join = j0;
        // With gif=DIR: the lit dish in real time, half a second before the
        // lamp, the 3 s it is held, and a second after, at 30 frames a
        // second -- the paired picture for the 2026-11-08 lamp change.
        let mut shots = Vec::new();
        let lamp = lit.lamp;
        if a.gif.is_some() {
            let mut pre = base.clone();
            for k in 0..STEPS_PER_SECOND / 2 {
                pre.step();
                if k % (2 * STEPS_PER_FRAME) == 0 {
                    shots.push(frame_of(&pre, a.scale, a.style));
                }
            }
        }
        for k in 0..3 * STEPS_PER_SECOND {
            lit.step();
            dark.step();
            if lit.t.is_multiple_of(30) {
                min_join = min_join.min(lit.measure().reach);
            }
            if a.gif.is_some() && k % (2 * STEPS_PER_FRAME) == 0 {
                shots.push(frame_of(&lit, a.scale, a.style));
            }
        }
        let shift = tube_offset(&dark, at, n) - tube_offset(&lit, at, n);
        if let Some(dir) = &a.gif {
            let mut after = lit.clone();
            after.set_lamp(None);
            for k in 0..STEPS_PER_SECOND {
                after.step();
                if k % (2 * STEPS_PER_FRAME) == 0 {
                    shots.push(frame_of(&after, a.scale, a.style));
                }
            }
            let path = format!("{dir}/push-s{seed}.gif");
            let delay = image::Delay::from_numer_denom_ms(1000, 30);
            let frames: Vec<image::Frame> = shots
                .into_iter()
                .map(|img| image::Frame::from_parts(img, 0, 0, delay))
                .collect();
            let file = std::fs::File::create(&path).expect("gif file");
            let mut enc = image::codecs::gif::GifEncoder::new_with_speed(file, 10);
            enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
            enc.encode_frames(frames).expect("gif encode");
            println!("seed {seed}: lamp at {lamp:?} -> {path}");
        }
        println!(
            "seed {seed}: tube moved {shift:+.1} cells away from the lamp; food joined {j0} -> lowest {min_join} while lit (dark twin {}); mould {} vs {} dark, burned {}",
            dark.measure().reach,
            lit.walkers.len(),
            dark.walkers.len(),
            lit.counts.burned
        );
    }
}

/// **The verbs in real time**: a lamp swept across a tube, then oats dropped
/// past the front, captured at 30 frames a second with the dish running at
/// play speed. The GIF the question is about: does the hand get an answer.
fn verbs(a: &Args, dir: &str) {
    for &seed in &a.seeds {
        let mut d = Dish::with_params(seed, a.params.clone());
        while d.t < (25 * STEPS_PER_SECOND) as u64 {
            d.step();
        }
        let mut shots = Vec::new();
        let mut frame = 0u32;
        let mut tick = |d: &mut Dish, shots: &mut Vec<image::RgbaImage>| {
            for _ in 0..STEPS_PER_FRAME {
                d.step();
            }
            if frame.is_multiple_of(2) {
                shots.push(frame_of(d, a.scale, a.style));
            }
            frame += 1;
        };
        // The thickest tube away from the piles, and the tube's direction.
        let mut best = (0.0f32, 0usize);
        for (i, &t) in d.trail.iter().enumerate() {
            let (x, y) = (
                (i % pixel_physics::mould::W) as f32,
                (i / pixel_physics::mould::W) as f32,
            );
            if t > best.0 && d.piles.iter().all(|p| (p.x - x).hypot(p.y - y) > p.r + LAMP_R + 4.0) {
                best = (t, i);
            }
        }
        let (tx, ty) = (
            (best.1 % pixel_physics::mould::W) as f32 + 0.5,
            (best.1 / pixel_physics::mould::W) as f32 + 0.5,
        );
        let (hx, hy) = (d.piles[0].x, d.piles[0].y);
        let len = (tx - hx).hypot(ty - hy).max(1.0);
        let (px, py) = (-(ty - hy) / len, (tx - hx) / len);
        // One second untouched, then the sweep: 2.5 s across the tube.
        for _ in 0..60 {
            tick(&mut d, &mut shots);
        }
        let under_before = near(&d, (tx, ty), 6.0);
        let mut min_under = usize::MAX;
        for k in 0..150 {
            let f = k as f32 / 149.0 * 2.0 - 1.0;
            d.set_lamp(Some((tx + px * 30.0 * f, ty + py * 30.0 * f)));
            tick(&mut d, &mut shots);
            if (60..90).contains(&k) {
                min_under = min_under.min(near(&d, (tx, ty), 6.0));
            }
        }
        d.set_lamp(None);
        for _ in 0..60 {
            tick(&mut d, &mut shots);
        }
        // Oats 20 cells past the front.
        let (fx, fy) = front(&d);
        let l = (fx - hx).hypot(fy - hy).max(1.0);
        let mut spot = (fx + (fx - hx) / l * 20.0, fy + (fy - hy) / l * 20.0);
        let mut placed = d.place_oats(spot.0, spot.1);
        for back in [15.0, 10.0, 5.0] {
            if placed {
                break;
            }
            spot = (fx + (fx - hx) / l * back, fy + (fy - hy) / l * back);
            placed = d.place_oats(spot.0, spot.1);
        }
        let mut touched = None;
        for k in 0..300 {
            tick(&mut d, &mut shots);
            if touched.is_none() && near(&d, spot, 4.0) > 0 {
                touched = Some(k as f32 / 60.0);
            }
        }
        let n = shots.len();
        let path = format!("{dir}/verbs-s{seed}.gif");
        let delay = image::Delay::from_numer_denom_ms(1000, 30);
        let frames: Vec<image::Frame> = shots
            .into_iter()
            .map(|img| image::Frame::from_parts(img, 0, 0, delay))
            .collect();
        let file = std::fs::File::create(&path).expect("gif file");
        let mut enc = image::codecs::gif::GifEncoder::new_with_speed(file, 10);
        enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
        enc.encode_frames(frames).expect("gif encode");
        println!(
            "verbs seed {seed}: {n} frames -> {path}; lamp over the tube: walkers within 6 cells {under_before} -> {} at the sweep's middle; oats placed {placed}, first walker on them after {}",
            min_under,
            touched.map_or("never".into(), |t| format!("{t:.2} s"))
        );
    }
}

fn cost(a: &Args) {
    println!("\n== frame cost: the whole dish awake, the window's frame (steps + draw at scale 4) ==");
    let mut d = Dish::with_params(a.seeds[0], a.params.clone());
    let mut p = Plan::new();
    let (w, h) = render::frame_size(4);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let view = View {
        style: a.style,
        ..View::default()
    };
    let (mut sim_t, mut draw_t, mut worst, mut worst_sim) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut n = 0;
    while d.phase != Phase::Over {
        let t0 = Instant::now();
        p.act(&mut d);
        for _ in 0..STEPS_PER_FRAME {
            d.step();
        }
        let t1 = Instant::now();
        render::draw(&d, &view, 4, &mut buf);
        let t2 = Instant::now();
        let (s, r) = ((t1 - t0).as_secs_f64() * 1e3, (t2 - t1).as_secs_f64() * 1e3);
        sim_t += s;
        draw_t += r;
        worst = worst.max(s + r);
        worst_sim = worst_sim.max(s);
        n += 1;
    }
    println!(
        "{n} frames, peak {} walkers: mean {:.2} ms (sim {:.2} + draw {:.2}), worst {:.2} ms (worst sim alone {:.2})",
        d.walkers.len(),
        (sim_t + draw_t) / n as f64,
        sim_t / n as f64,
        draw_t / n as f64,
        worst,
        worst_sim
    );
}

/// **Does the drawn mould flash grey?** (Owner's playtest, 2026-11-08:
/// "sometimes things were flashing gray a bit".) Plays the plan bot and, every
/// drawn frame, classes each dish cell by its drawn colour -- yellow (fed
/// tube), grey (unfed tube) or neither -- and counts cells that switch
/// between yellow and grey from one frame to the next. A cell that is
/// genuinely starving goes grey once and stays; flashing is a cell that
/// switches back and forth, so the count is also split into cells that
/// switched four or more times in a second.
fn flicker(a: &Args) {
    use pixel_physics::mould::{H, W};
    println!("\n== yellow<->grey switches per drawn frame (plan bot, 10-60 s) ==");
    let class = |c: [u8; 3]| -> u8 {
        let (r, g, b) = (c[0] as i32, c[1] as i32, c[2] as i32);
        if r > b + 40 && g > b {
            1 // yellow ramp
        } else if (r - b).abs() < 16 && (g - b).abs() < 16 && r > 40 {
            2 // pale ramp
        } else {
            0
        }
    };
    for &seed in &a.seeds {
        let mut d = Dish::with_params(seed, a.params.clone());
        let mut p = player("plan", seed);
        let mut prev = vec![0u8; W * H];
        let mut flips_in_second = vec![0u8; W * H];
        let (mut switches, mut frames, mut flashing) = (0u64, 0u64, 0u64);
        while d.t < (60 * STEPS_PER_SECOND) as u64 {
            p.act(&mut d);
            for _ in 0..STEPS_PER_FRAME {
                d.step();
            }
            let on = d.t >= (10 * STEPS_PER_SECOND) as u64;
            for i in 0..W * H {
                // Lamp-lit agar is drawn pale on purpose; leave it out.
                let c = if d.light[i] > 0.0 {
                    0
                } else {
                    class(render::cell_colour(&d, i))
                };
                if on && c != 0 && prev[i] != 0 && c != prev[i] {
                    switches += 1;
                    flips_in_second[i] = flips_in_second[i].saturating_add(1);
                }
                prev[i] = c;
            }
            if on {
                frames += 1;
                if frames.is_multiple_of(60) {
                    flashing += flips_in_second.iter().filter(|&&f| f >= 4).count() as u64;
                    flips_in_second.fill(0);
                }
            }
        }
        println!(
            "seed {seed}: {:.1} cells switch yellow<->grey per frame; {:.1} cells per second switch 4+ times (flashing)",
            switches as f64 / frames as f64,
            flashing as f64 / (frames / 60) as f64
        );
    }
}

/// **Can every step of the lesson be passed?** Plays `mould::lesson` with a
/// scripted hand that does what each banner says, prints when each step
/// began, and writes the window's frame (banner and all) half a second into
/// each step to DIR, plus the briefing card over the opening frame.
fn lesson(a: &Args, dir: &str) {
    use pixel_physics::mould::lesson::{Lesson, Step};
    use pixel_physics::mould::play::oats_toward_nearest;
    use pixel_physics::mould::render::Banner;
    use pixel_physics::mould::Tool;
    let (mut d, mut l) = Lesson::dish();
    let sps = STEPS_PER_SECOND as u64;
    let (hx, hy) = (d.piles[0].x, d.piles[0].y);
    let frame = |d: &Dish, l: &Lesson, brief: bool| {
        let view = View {
            keys: false,
            brief,
            banner: Some(Banner {
                number: l.step.number(),
                of: Step::COUNT,
                lines: l.step.lines(),
                fresh: l.fresh(d),
            }),
            style: a.style,
            ..View::default()
        };
        let (w, h) = render::frame_size(a.scale);
        let mut buf = vec![0u8; (w * h * 4) as usize];
        render::draw(d, &view, a.scale, &mut buf);
        image::RgbaImage::from_raw(w, h, buf).unwrap()
    };
    frame(&d, &l, true).save(format!("{dir}/lesson-0-brief.png")).unwrap();
    let mut shots = Vec::new();
    let mut began = d.t;
    let mut saved = false;
    let mut frames = 0u32;
    let stuck = 150 * sps;
    loop {
        match l.step {
            Step::Lamp => d.set_lamp(Some((hx + 12.0, hy))),
            Step::Oats | Step::Join | Step::Storm => {
                d.set_lamp(None);
                if d.t.is_multiple_of(sps) && d.oats_left > 0 {
                    oats_toward_nearest(&mut d, 22.0);
                }
            }
            Step::Snip if d.t == began + sps => {
                // The walker farthest from every pile: a stray branch.
                let piles: Vec<(f32, f32)> = d.dish_piles().map(|p| (p.x, p.y)).collect();
                let far = |w: &(f32, f32)| {
                    piles
                        .iter()
                        .map(|p| (p.0 - w.0).hypot(p.1 - w.1))
                        .fold(f32::INFINITY, f32::min)
                };
                if let Some((x, y)) = d
                    .walkers
                    .iter()
                    .map(|w| (w.x, w.y))
                    .max_by(|p, q| far(p).total_cmp(&far(q)))
                {
                    if d.begin_stroke(Tool::Snip, x - 8.0, y - 8.0) {
                        d.drag_stroke(x + 8.0, y + 8.0);
                        d.end_stroke();
                    }
                }
            }
            Step::Salt if d.t == began + sps && d.begin_stroke(Tool::Salt, hx - 22.0, hy - 25.0) => {
                d.drag_stroke(hx - 22.0, hy + 25.0);
                d.end_stroke();
            }
            _ => {}
        }
        for _ in 0..STEPS_PER_FRAME {
            d.step();
        }
        let before = l.step;
        l.update(&mut d);
        if l.step != before {
            println!(
                "{:?} passed at {:.1} s ({:.1} s on it); now {:?}",
                before,
                d.t as f32 / sps as f32,
                (d.t - began) as f32 / sps as f32,
                l.step
            );
            began = d.t;
            saved = false;
        }
        if !saved && d.t >= began + sps / 2 {
            frame(&d, &l, false)
                .save(format!(
                    "{dir}/lesson-{}-{}.png",
                    l.step.number(),
                    format!("{:?}", l.step).to_lowercase()
                ))
                .unwrap();
            saved = true;
        }
        if frames.is_multiple_of(a.every) {
            shots.push(frame(&d, &l, false));
        }
        frames += 1;
        if l.step == Step::Done && d.t >= began + 3 * sps {
            break;
        }
        if d.t >= began + stuck {
            println!("STUCK on {:?} for 150 s", l.step);
            break;
        }
    }
    let s = d.score.clone();
    println!(
        "lesson over at {:.1} s; score {:?}",
        d.t as f32 / sps as f32,
        s.map(|s| (s.star_count(), s.reach, s.piles, s.after))
    );
    write_gif(&format!("{dir}/lesson.gif"), shots, a.every);
}

/// One scripted moment, drawn in every look.
struct Moment {
    name: &'static str,
    what: &'static str,
    dish: Dish,
    view: View,
}

/// The counts that say whether anything happened, for the caption beside a
/// picture (`CLAUDE.md`: an image says what and where, only a number says
/// whether it fired).
fn moment_counts(d: &Dish) -> String {
    let s = d.measure();
    let c = &d.counts;
    format!(
        "t {:.1} s | food joined {}/{} | mould {} | oats placed {} | salt cells {} | snips {} | starved {} | storm-killed {}",
        d.t as f32 / STEPS_PER_SECOND as f32,
        s.reach,
        s.piles + d.pending.len() as u32,
        d.walkers.len(),
        c.oats_placed,
        c.salt_cells,
        c.snips_used,
        c.starved + c.hungry,
        c.stormed
    )
}

/// **The interface looks, frame by frame** (`mould::look`; brief: the
/// interface round, 2026-11-08). One scripted run, the same moments drawn in
/// the current look and in each candidate: (a) the lesson's oats step with
/// the oats preview under the cursor, (b) mid-dish with the lamp held,
/// (c) the storm's last seconds, (d) the score, and two more previews, (e) a
/// salt wall mid-drag, (f) a snip held over a tube, (g) the briefing card
/// and (h) oats held too far away to pull. Writes one PNG per
/// moment per look (`{letter}-{n}-{name}.png`; the current look is
/// `current`), a contact sheet per look, a GIF per look of the storm and the
/// score, `counts.txt`, and the draw cost of each look over the whole dish,
/// timed on the same dish state frame by frame.
fn looks(a: &Args, dir: &str) {
    use pixel_physics::mould::lesson::{Lesson, Step};
    use pixel_physics::mould::play::Fresh;
    use pixel_physics::mould::render::Banner;
    use pixel_physics::mould::Tool;
    let scale = 4;
    let sps = STEPS_PER_SECOND as u64;
    let seed = a.seeds[0];
    let base = View {
        keys: true,
        ..View::default()
    };
    let mut moments: Vec<Moment> = Vec::new();

    // (g) The briefing card over the lesson's opening frame.
    let (d0, l0) = Lesson::dish();
    moments.push(Moment {
        name: "brief",
        what: "the briefing card, over the lesson's first frame",
        dish: d0.clone(),
        view: View {
            brief: true,
            banner: Some(Banner {
                number: l0.step.number(),
                of: Step::COUNT,
                lines: l0.step.lines(),
                fresh: false,
            }),
            ..base
        },
    });
    // (a) The lesson, half a second into step 2, cursor where the banner
    // says to click: between the mould and the nearest pile.
    let (mut d, mut l) = Lesson::dish();
    let (hx, hy) = (d.piles[0].x, d.piles[0].y);
    let mut began = 0;
    while !(l.step == Step::Oats && d.t >= began + sps / 2) {
        if l.step == Step::Lamp {
            d.set_lamp(Some((hx + 12.0, hy)));
        } else {
            d.set_lamp(None);
        }
        for _ in 0..STEPS_PER_FRAME {
            d.step();
        }
        let before = l.step;
        l.update(&mut d);
        if l.step != before {
            began = d.t;
        }
    }
    let p = d.dish_piles().nth(1).map(|p| (p.x, p.y)).unwrap();
    let w = d
        .walkers
        .iter()
        .map(|w| (w.x, w.y))
        .min_by(|u, v| (u.0 - p.0).hypot(u.1 - p.1).total_cmp(&(v.0 - p.0).hypot(v.1 - p.1)))
        .unwrap();
    let gap = (p.0 - w.0).hypot(p.1 - w.1);
    let f = (14.0 / gap).min(0.6);
    let cursor = (w.0 + (p.0 - w.0) * f, w.1 + (p.1 - w.1) * f);
    moments.push(Moment {
        name: "lesson-oats",
        what: "lesson step 2, oats preview under the cursor",
        dish: d.clone(),
        view: View {
            tool: Tool::Oats,
            cursor: Some(cursor),
            banner: Some(Banner {
                number: l.step.number(),
                of: Step::COUNT,
                lines: l.step.lines(),
                fresh: l.fresh(&d),
            }),
            ..base
        },
    });

    // The dish: the scripted fresh player, start to score, drawn in every
    // look every frame for the cost, with the moments cut from it.
    let mut d = Dish::with_params(seed, a.params.clone());
    let mut hand = Fresh::new();
    let end = Dish::dish_steps() + Dish::storm_steps() + 3 * sps;
    let mut cost: Vec<(f64, f64, u32)> = vec![(0.0, 0.0, 0); Style::ALL.len()];
    let mut each: Vec<Vec<f64>> = vec![Vec::new(); Style::ALL.len()];
    let mut sim = (0.0f64, 0.0f64);
    let (w4, h4) = render::frame_size(scale);
    let mut buf = vec![0u8; (w4 * h4 * 4) as usize];
    let timed = View {
        tool: Tool::Oats,
        cursor: Some((128.0, 80.0)),
        ..base
    };
    // The GIF: the storm's last four seconds, the strike and four seconds
    // of the watch, four frames a second; then the score, held.
    let gif = (Dish::dish_steps() - 4 * sps)..(Dish::dish_steps() + 4 * sps);
    let mut gifs: Vec<Vec<image::RgbaImage>> = vec![Vec::new(); Style::ALL.len()];
    let mut frame = 0u32;
    while d.t < end {
        let t0 = Instant::now();
        hand.act(&mut d);
        for _ in 0..STEPS_PER_FRAME {
            d.step();
        }
        let st = (Instant::now() - t0).as_secs_f64() * 1e3;
        sim.0 += st;
        sim.1 = sim.1.max(st);
        // Every look on the same state, in a rotating order.
        for k in 0..Style::ALL.len() {
            let j = (k + frame as usize) % Style::ALL.len();
            let v = View {
                style: Style::ALL[j],
                ..timed
            };
            let t1 = Instant::now();
            render::draw(&d, &v, scale, &mut buf);
            let ms = (Instant::now() - t1).as_secs_f64() * 1e3;
            cost[j].0 += ms;
            cost[j].1 = cost[j].1.max(ms);
            cost[j].2 += 1;
            each[j].push(ms);
        }
        if gif.contains(&d.t) && frame.is_multiple_of(15) {
            for (j, style) in Style::ALL.iter().enumerate() {
                let v = View { style: *style, ..base };
                gifs[j].push(half(&draw_view(&d, &v, scale)));
            }
        }
        let at = |s: u64| d.t == s * sps;
        if at(25) {
            // (b) The lamp held beside the trunk for half a second.
            let (tp, n) = trunk(&d);
            let lamp = (tp.0 + n.0 * 10.0, tp.1 + n.1 * 10.0);
            let mut lit = d.clone();
            lit.set_lamp(Some(lamp));
            for _ in 0..sps / 2 {
                lit.step();
            }
            moments.push(Moment {
                name: "lamp-held",
                what: "mid-dish, the lamp held beside the trunk",
                dish: lit,
                view: View {
                    tool: Tool::Lamp,
                    cursor: Some(lamp),
                    ..base
                },
            });
            // (e) A salt wall, 26 cells into its 60, still being dragged.
            let mut salt = d.clone();
            salt.salt_left = salt.salt_left.max(1);
            let from = (tp.0 + n.0 * 24.0, tp.1 + n.1 * 24.0 - 13.0);
            let to = (from.0, from.1 + 26.0);
            if salt.begin_stroke(Tool::Salt, from.0, from.1) {
                salt.drag_stroke(to.0, to.1);
            }
            moments.push(Moment {
                name: "salt-drag",
                what: "a salt wall mid-drag: the length still to draw",
                dish: salt,
                view: View {
                    tool: Tool::Salt,
                    cursor: Some(to),
                    ..base
                },
            });
            // (h) Oats held where the mould is too far to be pulled: the
            // open spot whose nearest mould is closest to 44 cells away.
            let mut far = (f32::INFINITY, (0.0, 0.0));
            for gy in (12..148).step_by(4) {
                for gx in (12..244).step_by(4) {
                    let (x, y) = (gx as f32, gy as f32);
                    let open = d.ground[gy * pixel_physics::mould::W + gx] == pixel_physics::mould::Ground::Agar;
                    let on_pile = d.piles.iter().any(|p| (p.x - x).hypot(p.y - y) < p.r + 6.0);
                    if !open || on_pile {
                        continue;
                    }
                    let near = d.walkers.iter().map(|w| (w.x - x).hypot(w.y - y)).fold(f32::INFINITY, f32::min);
                    if (near - 44.0).abs() < far.0 {
                        far = ((near - 44.0).abs(), (x, y));
                    }
                }
            }
            moments.push(Moment {
                name: "oats-far",
                what: "oats held too far from the mould to pull it",
                dish: d.clone(),
                view: View {
                    tool: Tool::Oats,
                    cursor: Some(far.1),
                    ..base
                },
            });
            // (f) A snip held over the trunk, not yet pressed.
            let mut snip = d.clone();
            snip.snips_left = snip.snips_left.max(1);
            moments.push(Moment {
                name: "snip-hover",
                what: "snip held over a tube: the cells a press would cut",
                dish: snip,
                view: View {
                    tool: Tool::Snip,
                    cursor: Some(tp),
                    ..base
                },
            });
        }
        if d.t == Dish::dish_steps() - 3 * sps {
            // (c) Three seconds to the storm.
            moments.push(Moment {
                name: "storm-warning",
                what: "the storm's last seconds",
                dish: d.clone(),
                view: base,
            });
        }
        frame += 1;
    }
    // (d) The score.
    moments.push(Moment {
        name: "score",
        what: "the score",
        dish: d.clone(),
        view: base,
    });
    // a b c d first, as the brief lists them, then the extra previews.
    let order = ["lesson-oats", "lamp-held", "storm-warning", "score", "salt-drag", "snip-hover", "brief", "oats-far"];
    moments.sort_by_key(|m| order.iter().position(|o| *o == m.name).unwrap_or(9));

    let mut counts = String::new();
    for (k, m) in moments.iter().enumerate() {
        let line = format!("{}  {:<14} {}\n    {}\n", (b'a' + k as u8) as char, m.name, m.what, moment_counts(&m.dish));
        print!("{line}");
        counts.push_str(&line);
    }
    for (j, style) in Style::ALL.iter().enumerate() {
        let tag = if *style == Style::Classic { "current" } else { style.letter() };
        let mut tiles = Vec::new();
        for (k, m) in moments.iter().enumerate() {
            let v = View { style: *style, ..m.view };
            let img = draw_view(&m.dish, &v, scale);
            img.save(format!("{dir}/{tag}-{}-{}.png", (b'a' + k as u8) as char, m.name)).unwrap();
            tiles.push(half(&img));
        }
        sheet(&tiles, 4).save(format!("{dir}/{tag}-sheet.png")).unwrap();
        let mut g = std::mem::take(&mut gifs[j]);
        let score = half(&draw_view(&d, &View { style: *style, ..base }, scale));
        g.extend(std::iter::repeat_n(score, 6));
        // 45: the GIF plays at the dish's own speed (`write_gif` runs 3x).
        write_gif(&format!("{dir}/{tag}-storm.gif"), g, 45);
    }
    let n = cost[0].2 as f64;
    let mut table = format!(
        "\n== draw cost per look, scale 4, the fresh hand's whole dish (seed {seed}), {} frames, every look on the same state ==\nsim (all looks share it): mean {:.2} ms, worst {:.2} ms\n",
        cost[0].2,
        sim.0 / n,
        sim.1
    );
    for (j, style) in Style::ALL.iter().enumerate() {
        let tag = if *style == Style::Classic { "current" } else { style.letter() };
        let mut v = each[j].clone();
        v.sort_by(f64::total_cmp);
        let p99 = v[(v.len() * 99 / 100).min(v.len() - 1)];
        table.push_str(&format!(
            "{tag:>8}: draw mean {:.2} ms, 99th percentile {:.2} ms, worst {:.2} ms;  frame (sim + draw) mean {:.2} ms\n",
            cost[j].0 / n,
            p99,
            cost[j].1,
            (sim.0 + cost[j].0) / n
        ));
    }
    print!("{table}");
    counts.push_str(&table);
    std::fs::write(format!("{dir}/counts.txt"), counts).unwrap();
}

fn draw_view(d: &Dish, v: &View, scale: u32) -> image::RgbaImage {
    let (w, h) = render::frame_size(scale);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    render::draw(d, v, scale, &mut buf);
    image::RgbaImage::from_raw(w, h, buf).unwrap()
}

fn half(img: &image::RgbaImage) -> image::RgbaImage {
    image::imageops::resize(img, img.width() / 2, img.height() / 2, image::imageops::FilterType::Triangle)
}

/// Tiles in a grid, `cols` wide, with a 6-pixel gutter.
fn sheet(tiles: &[image::RgbaImage], cols: usize) -> image::RgbaImage {
    let (tw, th) = (tiles[0].width(), tiles[0].height());
    let rows = tiles.len().div_ceil(cols);
    let g = 6;
    let mut out = image::RgbaImage::from_pixel(
        cols as u32 * (tw + g) + g,
        rows as u32 * (th + g) + g,
        image::Rgba([40, 40, 44, 255]),
    );
    for (k, t) in tiles.iter().enumerate() {
        let (c, r) = ((k % cols) as u32, (k / cols) as u32);
        image::imageops::overlay(&mut out, t, (g + c * (tw + g)) as i64, (g + r * (th + g)) as i64);
    }
    out
}

/// **How far from the mould do oats still pull it?** The interface looks'
/// oats preview (`mould::look`) draws this reach around the cursor, so it is
/// measured here rather than guessed. At 6 s into each seed's dish, oats go
/// down beyond the mould's front along eight rays from home, at a range of
/// distances, each on its own copy of the dish; a placement counts as pulled
/// if the mould is on the oats within 5 s. Binned by the distance from the
/// oats to the nearest walker, which is what the preview can see.
fn pull(a: &Args) {
    println!("\n== oats pull: nearest mould D cells away; on the oats within 5 s? ==");
    let sps = STEPS_PER_SECOND as u64;
    let beyond = [6.0f32, 10.0, 14.0, 18.0, 22.0, 26.0, 30.0, 36.0, 44.0];
    // Bins of 4 cells of nearest-walker distance: (pulled, tried, seconds).
    let mut bins = [(0u32, 0u32, 0.0f32); 14];
    for &seed in &a.seeds {
        let mut d = Dish::with_params(seed, a.params.clone());
        while d.t < 6 * sps {
            d.step();
        }
        let (hx, hy) = (d.piles[0].x, d.piles[0].y);
        for k in 0..8 {
            let ang = k as f32 / 8.0 * std::f32::consts::TAU;
            let (ux, uy) = (ang.cos(), ang.sin());
            let front = d
                .walkers
                .iter()
                .filter_map(|w| {
                    let (dx, dy) = (w.x - hx, w.y - hy);
                    let r = dx.hypot(dy);
                    (r > 1.0 && (dx * ux + dy * uy) / r > 0.966).then_some(r)
                })
                .fold(0.0f32, f32::max);
            for &b in &beyond {
                let (x, y) = (hx + ux * (front + b), hy + uy * (front + b));
                let mut t = d.clone();
                if !t.place_oats(x, y) {
                    continue;
                }
                let near = t
                    .walkers
                    .iter()
                    .map(|w| (w.x - x).hypot(w.y - y))
                    .fold(f32::INFINITY, f32::min);
                let bin = ((near / 4.0) as usize).min(bins.len() - 1);
                let n = t.piles.len() - 1;
                bins[bin].1 += 1;
                let start = t.t;
                while t.t < start + 5 * sps {
                    t.step();
                    if t.piles[n].reached() {
                        bins[bin].0 += 1;
                        bins[bin].2 += (t.t - start) as f32 / sps as f32;
                        break;
                    }
                }
            }
        }
    }
    for (k, (hit, n, secs)) in bins.iter().enumerate() {
        if *n > 0 {
            println!(
                "nearest mould {:>2}-{:>2} cells: pulled {hit:>2} of {n:>2}{}",
                k * 4,
                k * 4 + 4,
                if *hit > 0 {
                    format!(", mean {:.2} s", secs / *hit as f32)
                } else {
                    String::new()
                }
            );
        }
    }
}

/// **Does a cut cost anything?** (owner's card 2026-11-08, "each piece eats
/// its own food"). The plan hand plays 40 s, then one snip goes straight
/// across the thickest tube. Every half second for 12 s: drawn tube that
/// shows grey (starving), food joined, mould, open wound cells, joins
/// flashed. Run once with the defaults and once with `pieces=0 wound=0` for
/// the old snip. With cut=DIR (not "1"): stills at 0, 1, 2, 4, 8 s after.
fn cut(a: &Args, dir: &str) {
    let w = pixel_physics::mould::W;
    println!("\n== one snip across the trunk at 40 s, plan hand ==");
    for &seed in &a.seeds {
        let mut d = Dish::with_params(seed, a.params.clone());
        let mut p = player("plan", seed);
        while d.t < (40 * STEPS_PER_SECOND) as u64 {
            p.act(&mut d);
            d.step();
        }
        let (at, n) = trunk(&d);
        let mould0 = d.walkers.len();
        let snaps0 = d.counts.snaps;
        d.snips_left = d.snips_left.max(1);
        let ok = d.begin_stroke(Tool::Snip, at.0 - n.0 * 12.0, at.1 - n.1 * 12.0);
        d.drag_stroke(at.0 + n.0 * 12.0, at.1 + n.1 * 12.0);
        d.end_stroke();
        let grey = |d: &Dish| {
            (0..d.trail.len())
                .filter(|&i| d.trail[i] > d.params.tube * 0.5 && !d.looks_fed(i))
                .count()
        };
        let mut row = Vec::new();
        let sps = STEPS_PER_SECOND as u64;
        let t0 = d.t;
        let mut max_grey = 0;
        let mut wound_open = 0;
        while d.t < t0 + 12 * sps {
            d.step();
            let k = d.t - t0;
            if k.is_multiple_of(sps / 2) {
                let g = grey(&d);
                max_grey = max_grey.max(g);
                let wounds = d.ground.iter().filter(|g| **g == Ground::Wound).count();
                if wounds > 0 {
                    wound_open = k;
                }
                row.push(format!("{:.1}s grey {g} joined {} mould {} wound {wounds}", k as f32 / sps as f32, d.measure().reach, d.walkers.len()));
            }
            if dir != "1" && [0, 1, 2, 4, 8].iter().any(|&s| k == s * sps + 1) {
                frame_of(&d, a.scale, a.style)
                    .save(format!("{dir}/cut-s{seed}-{:02}.png", k / sps))
                    .unwrap();
            }
        }
        let _ = w;
        println!(
            "seed {seed}: snip {} at ({:.0},{:.0}); most grey tube cells {max_grey}; wound open until {:.1} s; mould {mould0} -> {}; joins flashed in the 12 s {}",
            if ok { "made" } else { "REFUSED" },
            at.0,
            at.1,
            wound_open as f32 / sps as f32,
            d.walkers.len(),
            d.counts.snaps - snaps0
        );
        for r in row.iter().step_by(2) {
            println!("    {r}");
        }
    }
}

fn main() {
    let a = parse();
    println!(
        "mouldprobe: seeds {:?} players {:?} scale {} params {:?}",
        a.seeds, a.players, a.scale, a.params
    );
    if a.lean {
        lean(&a);
        return;
    }
    if a.wander {
        wander(&a);
        return;
    }
    if a.push {
        push(&a);
        return;
    }
    if a.flicker {
        flicker(&a);
        return;
    }
    if let Some(dir) = &a.lesson {
        lesson(&a, dir);
        return;
    }
    if a.pull {
        pull(&a);
        return;
    }
    if let Some(dir) = &a.looks {
        looks(&a, dir);
        return;
    }
    if let Some(dir) = &a.cut {
        cut(&a, dir);
        return;
    }
    if let Some(dir) = &a.verbs {
        verbs(&a, dir);
        return;
    }
    let mut table: Vec<(u64, String, Run)> = Vec::new();
    for &seed in &a.seeds {
        for who in &a.players {
            let r = run(seed, who, &a);
            let s = &r.score;
            println!(
                "seed {seed} {who:>7}: {} stars [{}{}{}]  joined {}/{}  after storm {}  length {:.0} vs shortest {:.0} ({:.2}x)  peak {}  births {} starved {} hungry {} burned {}  oats {} snips {} lamp {:.0}s joins {}",
                s.star_count(),
                s.stars[0] as u8,
                s.stars[1] as u8,
                s.stars[2] as u8,
                s.reach,
                s.piles,
                s.after,
                s.length,
                s.shortest,
                s.ratio(),
                r.peak,
                r.births,
                r.starved,
                r.hungry,
                r.burned,
                r.oats,
                r.snips,
                r.lamp_s,
                r.joins
            );
            println!("    trace (s, mould, joined): {:?}", r.trace);
            table.push((seed, who.clone(), r));
        }
    }
    // Per hand: the playtest's numbers (2026-11-08 review).
    println!("\n== per hand: stars by kind, and the playtest's three complaints ==");
    println!("frozen = food joined at 30 s equals food joined at 80 s; storm cost = joined after the storm < before");
    for who in &a.players {
        let runs: Vec<&Run> = table.iter().filter(|(_, w, _)| w == who).map(|(_, _, r)| r).collect();
        let stars: u32 = runs.iter().map(|r| r.score.star_count()).sum();
        let kind = |k: usize| runs.iter().filter(|r| r.score.stars[k]).count();
        let at = |r: &Run, s: u32| r.trace.iter().find(|t| t.0 == s).map(|t| t.2);
        let frozen = runs
            .iter()
            .filter(|r| at(r, 30).is_some() && at(r, 30) == at(r, 80))
            .count();
        let cost = runs.iter().filter(|r| r.score.after < r.score.reach).count();
        let per_seed: Vec<String> = runs.iter().map(|r| r.score.star_count().to_string()).collect();
        let joined: Vec<String> = runs.iter().map(|r| r.score.reach.to_string()).collect();
        println!(
            "{who:>9}: {stars:>2} stars [{}]  reach {} thrift {} tough {}  joined [{}]  frozen {frozen}/{n}  storm cost {cost}/{n}",
            per_seed.join(" "),
            kind(0),
            kind(1),
            kind(2),
            joined.join(" "),
            n = runs.len()
        );
    }
    // Paired: each player against do-nothing, seed by seed.
    let get = |seed: u64, who: &str| table.iter().find(|(s, w, _)| *s == seed && w == who).map(|(_, _, r)| r);
    println!("\n== paired against do nothing (stars, then food joined) ==");
    for who in a.players.iter().filter(|w| *w != "idle") {
        let (mut w, mut l, mut t) = (0, 0, 0);
        let (mut jw, mut jl) = (0, 0);
        let mut total = (0, 0);
        for &seed in &a.seeds {
            let (Some(x), Some(base)) = (get(seed, who), get(seed, "idle")) else {
                continue;
            };
            let (sx, sb) = (x.score.star_count(), base.score.star_count());
            total.0 += sx;
            total.1 += sb;
            match sx.cmp(&sb) {
                std::cmp::Ordering::Greater => w += 1,
                std::cmp::Ordering::Less => l += 1,
                _ => t += 1,
            }
            match x.score.reach.cmp(&base.score.reach) {
                std::cmp::Ordering::Greater => jw += 1,
                std::cmp::Ordering::Less => jl += 1,
                _ => {}
            }
        }
        println!(
            "{who:>7} vs do nothing: stars {} vs {} total; seeds better {w}, worse {l}, level {t}; sign-test p {:.3}.  food joined better {jw}, worse {jl}, p {:.3}",
            total.0,
            total.1,
            sign_p(w, l),
            sign_p(jw, jl)
        );
    }
    if a.players.iter().any(|p| p == "plan") {
        cost(&a);
    }
}

// ------------------------------------------------------------ the Wanderer

/// One stretch of a Wanderer journey, as the table prints it.
#[derive(Clone, Debug)]
struct Stretch {
    seed: u64,
    came_in: u32,
    outcome: Option<Outcome>,
    go_s: Option<f32>,
    food_at_go: f32,
    peak: u32,
    end_s: f32,
    alive_at_end: usize,
    births: u64,
    starved: u64,
    hungry: u64,
    oats: u32,
    lamp_s: f32,
    /// Seconds Go was open before it was pressed (or before the end).
    open_s: f32,
    /// Walkers that set off on Go, and that starved in the line.
    set_off: u32,
    lost_on_way: u32,
}

fn wander_player(name: &str) -> Box<dyn Player> {
    match name {
        "idle" => Box::new(Idle),
        "idlego" => Box::new(IdleGo),
        "careless" => Box::new(Leader::new(true)),
        "lead" => Box::new(Leader::new(false)),
        _ => panic!("unknown wanderer hand {name} (idle, idlego, careless, lead)"),
    }
}

fn wander_frame(d: &Dish, scale: u32, brief: bool) -> image::RgbaImage {
    let (w, h) = render::frame_size(scale);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let view = View {
        keys: false,
        brief,
        ..View::default()
    };
    render::draw(d, &view, scale, &mut buf);
    image::RgbaImage::from_raw(w, h, buf).expect("frame size")
}

/// One journey: up to `dishes` stretches, each on seed `seed + 100 (k - 1)`,
/// carrying what arrived. Stops at a loss, a stretch nobody left, or an
/// arrival of nothing. With `gif=DIR`: an end still for every stretch, the
/// briefing card, and for the `lead` hand a GIF of stretch 1 from the start
/// to the end of the pour.
fn journey(seed: u64, who: &str, a: &Args) -> Vec<Stretch> {
    let sps = STEPS_PER_SECOND as u64;
    let cap = a.cap_min as u64 * 60 * sps;
    let mut carry = FIRST_WALKERS;
    let mut out = Vec::new();
    for k in 1..=a.dishes {
        let dseed = seed + 100 * (k as u64 - 1);
        let mut d = Dish::wanderer_with_params(dseed, k, carry, a.params.clone());
        let mut p = wander_player(who);
        let gif = a.gif.is_some() && who == "lead" && k == 1;
        let mut shots = Vec::new();
        let mut frame = 0u32;
        let mut open = 0u64;
        let mut over_at = None;
        if let Some(dir) = &a.gif {
            if k <= 2 && who == "lead" {
                wander_frame(&d, a.scale, true)
                    .save(format!("{dir}/wander-s{seed}-d{k}-brief.png"))
                    .unwrap();
            }
        }
        loop {
            if d.can_go() {
                open += 1;
            }
            p.act(&mut d);
            for _ in 0..STEPS_PER_FRAME {
                d.step();
            }
            if gif {
                let pouring = matches!(d.phase, Phase::Crossing(_));
                // Two seconds of dish a frame while growing, a tenth of a
                // second through the pour: keeps the GIF near 3 MB.
                let every = if pouring || over_at.is_some() { 6 } else { 120 };
                if frame.is_multiple_of(every) {
                    shots.push(wander_frame(&d, a.scale, false));
                }
            }
            frame += 1;
            if d.phase == Phase::Over && over_at.is_none() {
                over_at = Some(d.t);
            }
            // A second of the card after the end, then stop.
            if over_at.is_some_and(|t| d.t >= t + sps) || d.t >= cap {
                break;
            }
        }
        let w = d.wander_state().cloned().expect("a wanderer dish");
        if let Some(dir) = &a.gif {
            wander_frame(&d, a.scale, false)
                .save(format!("{dir}/wander-s{seed}-{who}-d{k}-end.png"))
                .unwrap();
            if gif {
                write_wander_gif(&format!("{dir}/wander-s{seed}-lead-d1.gif"), shots);
            }
        }
        let c = &d.counts;
        out.push(Stretch {
            seed: dseed,
            came_in: w.came_in,
            outcome: w.outcome,
            go_s: w.go_at.map(|t| t as f32 / sps as f32),
            food_at_go: w.food_at_go,
            peak: w.peak,
            end_s: d.t as f32 / sps as f32,
            alive_at_end: d.walkers.len(),
            births: c.births,
            starved: c.starved,
            hungry: c.hungry,
            oats: c.oats_placed,
            lamp_s: c.lamp_steps as f32 / sps as f32,
            open_s: (open * STEPS_PER_FRAME as u64) as f32 / sps as f32,
            set_off: w.set_off,
            lost_on_way: w.lost_on_way,
        });
        match w.outcome {
            Some(Outcome::Arrived { n, .. }) if n > 0 => carry = n,
            _ => break,
        }
    }
    out
}

/// Growing frames are two seconds of dish apart and pour frames 0.1 s; the
/// GIF plays both at 10 frames a second, so the growth runs at 20x and the
/// pour at real speed, which is where the eye needs the time.
fn write_wander_gif(path: &str, shots: Vec<image::RgbaImage>) {
    let delay = image::Delay::from_numer_denom_ms(1000, 10);
    let frames: Vec<image::Frame> = shots
        .into_iter()
        .map(|img| image::Frame::from_parts(img, 0, 0, delay))
        .collect();
    let file = std::fs::File::create(path).expect("gif file");
    let mut enc = image::codecs::gif::GifEncoder::new_with_speed(file, 10);
    enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
    enc.encode_frames(frames).expect("gif encode");
}

fn show(o: &Stretch) -> String {
    match o.outcome {
        Some(Outcome::Arrived { n, of }) => format!("{n}/{of}"),
        Some(Outcome::Lost { .. }) => "died".into(),
        None => "stayed".into(),
    }
}

/// **The Wanderer's number**: arrived N of M, per hand per seed, over
/// journeys of `dishes` stretches. Hands: `idle` (never Goes; she should
/// starve), `idlego` (no hand, Go the moment it opens), `careless` (lead
/// with lamp and oats, Go the moment it opens) and `lead` (the same lead, Go
/// when the would-come count peaks).
fn wander(a: &Args) {
    let hands: Vec<String> = if a.players.len() == 7 {
        ["idle", "idlego", "careless", "lead"].map(String::from).to_vec()
    } else {
        a.players.clone()
    };
    println!(
        "\n== the Wanderer: {} stretches a journey, cap {} min a stretch; dish k is seed + 100 (k - 1) ==",
        a.dishes, a.cap_min
    );
    let jobs: Vec<(u64, String)> = a
        .seeds
        .iter()
        .flat_map(|&s| hands.iter().map(move |h| (s, h.clone())))
        .collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| loop {
                let k = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let Some((seed, who)) = jobs.get(k) else { break };
                let j = journey(*seed, who, a);
                results.lock().unwrap().push((*seed, who.clone(), j));
            });
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by(|x, y| (x.0, &x.1).cmp(&(y.0, &y.1)));
    for (seed, who, j) in &results {
        for (k, st) in j.iter().enumerate() {
            println!(
                "seed {seed} {who:>8} dish {} (seed {}): in {} -> {}  peak {}  set off {} starved in line {}  go {}  food left at go {:.0}%  Go open {:.1}s  end {:.0}s alive {}  births {} starved {} hungry {} oats {} lamp {:.0}s",
                k + 1,
                st.seed,
                st.came_in,
                show(st),
                st.peak,
                st.set_off,
                st.lost_on_way,
                st.go_s.map_or("-".into(), |s| format!("{s:.1}s")),
                st.food_at_go * 100.0,
                st.open_s,
                st.end_s,
                st.alive_at_end,
                st.births,
                st.starved,
                st.hungry,
                st.oats,
                st.lamp_s
            );
        }
    }
    println!(
        "\n== arrived N of M, per hand per seed (dish 1 | dish 2 | dish 3; 'died' = starved, 'stayed' = never left) =="
    );
    for who in &hands {
        println!("{who}:");
        let mut fracs = Vec::new();
        let mut finals = Vec::new();
        for (seed, w, j) in &results {
            if w != who {
                continue;
            }
            let cells: Vec<String> = (0..a.dishes as usize).map(|k| j.get(k).map_or("-".into(), show)).collect();
            println!("  seed {seed}: {}", cells.join(" | "));
            fracs.push(match j[0].outcome {
                Some(Outcome::Arrived { n, of }) => n as f32 / of as f32,
                _ => 0.0,
            });
            let done = j.len() == a.dishes as usize;
            finals.push(match j.last().unwrap().outcome {
                Some(Outcome::Arrived { n, .. }) if done => n,
                _ => 0,
            });
        }
        if fracs.is_empty() {
            continue;
        }
        fracs.sort_by(|x, y| x.total_cmp(y));
        finals.sort();
        let q = |v: &[f32], f: f32| v[((v.len() - 1) as f32 * f).round() as usize];
        println!(
            "  dish 1 share arrived: min {:.0}%  median {:.0}%  max {:.0}%;  walkers through the last crack: {:?}",
            q(&fracs, 0.0) * 100.0,
            q(&fracs, 0.5) * 100.0,
            q(&fracs, 1.0) * 100.0,
            finals
        );
    }
    wander_cost(a);
}

/// The window's frame (steps + draw at scale 4) over one led stretch, against
/// the scored dish's plan bot on the same seed, measured back to back.
fn wander_cost(a: &Args) {
    println!("\n== frame cost: steps + draw at scale 4, mean and worst ==");
    let (w, h) = render::frame_size(4);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let view = View::default();
    for arm in ["scored dish, plan", "wanderer, lead"] {
        let seed = a.seeds[0];
        let (mut d, mut p): (Dish, Box<dyn Player>) = if arm.starts_with("scored") {
            (Dish::new(seed), Box::new(Plan::new()))
        } else {
            (Dish::wanderer(seed, 1, FIRST_WALKERS), Box::new(Leader::new(false)))
        };
        let (mut sum, mut worst, mut n, mut peak) = (0.0f64, 0.0f64, 0u64, 0usize);
        let cap = a.cap_min as u64 * 60 * STEPS_PER_SECOND as u64;
        while d.phase != Phase::Over && d.t < cap {
            let t0 = Instant::now();
            p.act(&mut d);
            for _ in 0..STEPS_PER_FRAME {
                d.step();
            }
            render::draw(&d, &view, 4, &mut buf);
            let ms = t0.elapsed().as_secs_f64() * 1e3;
            sum += ms;
            worst = worst.max(ms);
            n += 1;
            peak = peak.max(d.walkers.len());
        }
        println!(
            "{arm}: {n} frames, peak {peak} walkers: mean {:.2} ms, worst {:.2} ms",
            sum / n as f64,
            worst
        );
    }
}
