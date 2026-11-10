//! **The looks pass's paired pictures** (2026-11-10): one dish, played by
//! the scripted plan hand, drawn at the same instant in each look -- the
//! old one (grey starving tube, no walkers) and the new ones (the food
//! ramp, walkers as grains or glints) -- so a before/after differs only in
//! the drawing. Also the draw cost of each, and the numbers the food ramp
//! was scaled from.
//!
//! ```text
//! cargo run --release --example mouldlooks -- out=DIR flow=1 angle=22.5 seed=1 at=20,40,60 gif=40 scale=4 [tools=1] [hand=idle]
//! ```
use std::sync::atomic::Ordering::Relaxed;
use std::time::Instant;

use pixel_physics::mould::look::{Style, SMOOTH, STAIN, STARVE_LOOK, WALKER_LOOK};
use pixel_physics::mould::play::{Plan, Player};
use pixel_physics::mould::render::{self, View};
use pixel_physics::mould::{Dish, Ground, Params, Tool, STEPS_PER_FRAME, STEPS_PER_SECOND};

/// (tag, starving look, walker look, smoothing)
const LOOKS: [(&str, u8, u8, bool); 5] =
    [("before", 0, 0, false), ("rough", 3, 1, false), ("ramp", 3, 0, true), ("grains", 3, 1, true), ("glints", 3, 2, true)];

fn pct(mut v: Vec<f32>, ps: &[f32]) -> String {
    if v.is_empty() {
        return "-".into();
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ps.iter().map(|p| format!("p{:.0} {:.2}", p * 100.0, v[((v.len() - 1) as f32 * p) as usize])).collect::<Vec<_>>().join("  ")
}

fn frame(d: &Dish, scale: u32, look: (u8, u8, bool)) -> (image::RgbaImage, f64) {
    STARVE_LOOK.store(look.0, Relaxed);
    WALKER_LOOK.store(look.1, Relaxed);
    SMOOTH.store(look.2, Relaxed);
    STAIN.store(look.2, Relaxed);
    let (w, h) = render::frame_size(scale);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let view = View { keys: false, style: Style::GAME, ..View::default() };
    let t0 = Instant::now();
    render::draw(d, &view, scale, &mut buf);
    let ms = t0.elapsed().as_secs_f64() * 1e3;
    (image::RgbaImage::from_raw(w, h, buf).unwrap(), ms)
}

fn main() {
    let (mut flow, mut angle, mut seed, mut scale) = (false, 45.0f32, 1u64, 4u32);
    let mut out = String::from("out");
    let mut at: Vec<u64> = vec![20, 40, 60];
    let mut gif: Option<u64> = None;
    // `tools=1`: a salt wall at 25 s, a snip at 35 s and the lamp held
    // from 44 s, so the pictures have each mark in them.
    let mut tools = false;
    // `hand=idle`: nobody plays, so the mould eats its piles and starves:
    // the case that shows hungry tube going cold.
    let mut idle = false;
    let (mut salted, mut snipped) = (false, false);
    for a in std::env::args().skip(1) {
        let (k, v) = a.split_once('=').unwrap_or_else(|| panic!("{a}: want key=value"));
        match k {
            "flow" => flow = v == "1",
            "angle" => angle = v.parse().unwrap(),
            "seed" => seed = v.parse().unwrap(),
            "scale" => scale = v.parse().unwrap(),
            "out" => out = v.into(),
            "at" => at = v.split(',').map(|s| s.parse().unwrap()).collect(),
            "gif" => gif = Some(v.parse().unwrap()),
            "tools" => tools = v == "1",
            "hand" => idle = v == "idle",
            _ => panic!("unknown {k}"),
        }
    }
    println!("flow={flow} angle={angle} seed={seed} scale={scale} at={at:?} gif={gif:?} tools={tools} idle={idle} out={out}");
    std::fs::create_dir_all(&out).unwrap();
    let p = Params { flow, sensor_angle: angle.to_radians(), ..Params::default() };
    let mut d = Dish::with_params(seed, p);
    let mut pl = Plan::new();
    let sps = STEPS_PER_SECOND as u64;
    let tag = format!("f{}-a{}-s{seed}", flow as u8, angle);
    let end = at.iter().copied().chain(gif.map(|g| g + 3)).max().unwrap();
    let mut gifs: Vec<Vec<image::RgbaImage>> = vec![Vec::new(); LOOKS.len()];
    let mut cost = vec![(0.0f64, 0u32); LOOKS.len()];
    let mut frame_no = 0u64;
    while d.t < end * sps {
        if !idle {
            pl.act(&mut d);
        }
        if tools {
            let t = d.t;
            if t >= 25 * sps && !salted {
                salted = true;
                let ok = d.begin_stroke(Tool::Salt, 70.0, 120.0);
                for k in 1..=30 {
                    d.drag_stroke(70.0 + k as f32 * 1.5, 120.0 - k as f32 * 0.6);
                }
                d.end_stroke();
                println!("salt at {} s: {ok}", t / sps);
            }
            if t >= 35 * sps && !snipped {
                snipped = true;
                let ok = d.begin_stroke(Tool::Snip, 128.0, 50.0);
                for k in 1..=25 {
                    d.drag_stroke(128.0 + k as f32 * 0.4, 50.0 + k as f32 * 1.2);
                }
                d.end_stroke();
                println!("snip at {} s: {ok}", t / sps);
            }
            if t >= 44 * sps {
                d.set_lamp(Some((150.0, 70.0)));
            }
        }
        for _ in 0..STEPS_PER_FRAME {
            d.step();
        }
        frame_no += 1;
        // The stain is kept by the drawing, frame to frame, as the window
        // draws: keep it current between the pictures.
        if frame_no.is_multiple_of(6) {
            // At the pictures' scale: the rim layer is kept per scale.
            frame(&d, scale, (3, 1, true));
        }
        let sec = d.t / sps;
        let on_second = d.t.is_multiple_of(sps);
        if on_second && at.contains(&sec) {
            let thr = d.params.tube * 0.5;
            let tube: Vec<usize> = (0..d.trail.len()).filter(|&i| d.ground[i] == Ground::Agar && d.trail[i] > thr).collect();
            let fed = tube.iter().filter(|&&i| d.looks_fed(i)).count();
            let lvl: Vec<f32> = tube.iter().filter_map(|&i| pixel_physics::mould::look::food_level(&d, i)).collect();
            println!(
                "t={sec}s walkers {} tube cells {} fed {} ({:.0}%)  food level on tube: {}",
                d.walkers.len(),
                tube.len(),
                fed,
                100.0 * fed as f32 / tube.len().max(1) as f32,
                pct(lvl, &[0.1, 0.25, 0.5, 0.75, 0.9])
            );
            for (k, (name, a, b, c)) in LOOKS.iter().enumerate() {
                let (img, ms) = frame(&d, scale, (*a, *b, *c));
                cost[k].0 += ms;
                cost[k].1 += 1;
                img.save(format!("{out}/{tag}-t{sec:03}-{name}.png")).unwrap();
            }
        }
        // A GIF of 3 s from `gif`, every second frame (30 fps).
        if let Some(g) = gif {
            if d.t >= g * sps && d.t < (g + 3) * sps && frame_no.is_multiple_of(2) {
                for (k, (_, a, b, c)) in LOOKS.iter().enumerate() {
                    let (img, ms) = frame(&d, scale, (*a, *b, *c));
                    cost[k].0 += ms;
                    cost[k].1 += 1;
                    gifs[k].push(img);
                }
            }
        }
    }
    for (k, (name, _, _, _)) in LOOKS.iter().enumerate() {
        println!("draw {name:>7}: mean {:.2} ms over {} frames (scale {scale})", cost[k].0 / cost[k].1.max(1) as f64, cost[k].1);
        if !gifs[k].is_empty() {
            let f = std::fs::File::create(format!("{out}/{tag}-gif{}-{name}.gif", gif.unwrap())).unwrap();
            let mut enc = image::codecs::gif::GifEncoder::new_with_speed(f, 10);
            enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
            let delay = image::Delay::from_numer_denom_ms(33, 1);
            enc.encode_frames(gifs[k].drain(..).map(|im| image::Frame::from_parts(im, 0, 0, delay))).unwrap();
        }
    }
    STARVE_LOOK.store(3, Relaxed);
    WALKER_LOOK.store(1, Relaxed);
    SMOOTH.store(true, Relaxed);
    STAIN.store(true, Relaxed);
}
