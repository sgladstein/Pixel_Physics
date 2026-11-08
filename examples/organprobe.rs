//! **The Mould Organ's harness**: does the tube distance from the heart to
//! each bell hold still enough to carry a rhythm, can a hand move it on
//! purpose, and what does the organ sound and look like.
//!
//! ```text
//! cargo run --release --example organprobe -- step0=1                 # timing wobble, 8 seeds, both rulers
//! cargo run --release --example organprobe -- step0=1 window=60       # ...over a minute
//! cargo run --release --example organprobe -- bend=1                  # lamp pushes the high tube: how far its note moves
//! cargo run --release --example organprobe -- bend=1 look=DIR         # ...and a contact sheet of the push
//! ```
//!
//! **Two rulers.** `grid` is the brief's: breadth-first along tube cells, as
//! the feeding search counts. `geo` is what the organ plays by
//! (`organ::heart_geodesic`): the same tube, measured in straight-line
//! cells. They differ because a four-neighbour step count gives every
//! staircase between two points the same length, so a sideways bow of a
//! tube does not register on `grid` at all (`bend=1` shows it).

use pixel_physics::mould::organ;
use pixel_physics::mould::render::{self, View};
use pixel_physics::mould::{Dish, H, STEPS_PER_SECOND, W};

struct Args {
    seeds: Vec<u64>,
    step0: bool,
    /// Seconds left alone after the last oats are gone, before the window.
    settle: u32,
    window: u32,
    /// Write stills here.
    look: Option<String>,
    /// The lamp-bend test.
    bend: bool,
    /// Bend: lamp offset from the tube's middle at the start and the end of
    /// the push (cells across the tube; positive is the far side from the
    /// heart), and how long the push lasts.
    from: f32,
    to: f32,
    hold: f32,
    /// Which bell's tube to bend (1 low, 2 mid, 3 high).
    bell: usize,
    /// The `geo` ruler's core weight (`organ::geodesic`); the organ's own is
    /// `organ::CORE_WEIGHT`.
    core: f32,
    /// The scripted performance: alone, lamp on the high tube, let go, salt
    /// across the mid tube, oats between the low and mid bells.
    play: bool,
    /// Play: write frames (PNG, every 8 steps = 15 a second) and the WAV of
    /// the first seed here.
    frames: Option<String>,
}

fn parse() -> Args {
    let mut a = Args {
        seeds: (1..=8).collect(),
        step0: false,
        settle: 20,
        window: 10,
        look: None,
        bend: false,
        from: 18.0,
        to: -4.0,
        hold: 3.0,
        bell: 3,
        core: organ::CORE_WEIGHT,
        play: false,
        frames: None,
    };
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else {
            panic!("unknown argument {arg}")
        };
        match k {
            "seeds" => a.seeds = v.split(',').map(|s| s.parse().unwrap()).collect(),
            "step0" => a.step0 = v == "1",
            "settle" => a.settle = v.parse().unwrap(),
            "window" => a.window = v.parse().unwrap(),
            "look" => a.look = Some(v.into()),
            "bend" => a.bend = v == "1",
            "from" => a.from = v.parse().unwrap(),
            "to" => a.to = v.parse().unwrap(),
            "hold" => a.hold = v.parse().unwrap(),
            "bell" => a.bell = v.parse().unwrap(),
            "play" => a.play = v == "1",
            "core" => a.core = v.parse().unwrap(),
            "frames" => a.frames = Some(v.into()),
            _ => panic!("unknown argument {k}"),
        }
    }
    eprintln!(
        "organprobe: seeds={:?} step0={} bend={} play={} settle={}s window={}s from={} to={} hold={}s bell={} core={}",
        a.seeds, a.step0, a.bend, a.play, a.settle, a.window, a.from, a.to, a.hold, a.bell, a.core
    );
    a
}

fn pct(v: &[f32], p: f32) -> f32 {
    let mut v = v.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    if v.is_empty() {
        return f32::NAN;
    }
    v[((v.len() - 1) as f32 * p).round() as usize]
}

/// Both rulers, with their buffers.
#[derive(Default)]
struct Ruler {
    grid: Vec<u16>,
    geo: Vec<f32>,
    core: f32,
}

impl Ruler {
    /// Each bell's distance from the heart: (grid steps, geodesic cells).
    fn bells(&mut self, d: &Dish) -> [(Option<f32>, Option<f32>); 3] {
        organ::heart_distance(d, 2000, &mut self.grid);
        organ::heart_geodesic_core(d, 2000.0, self.core, &mut self.geo);
        [1, 2, 3].map(|b| {
            (
                organ::bell_distance(d, &self.grid, b).map(|v| v as f32),
                organ::bell_geodesic(d, &self.geo, b),
            )
        })
    }
}

fn frame_of(d: &Dish) -> image::RgbaImage {
    let scale = 2;
    let (w, h) = render::frame_size(scale);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let view = View { keys: false, ..View::default() };
    render::draw(d, &view, scale, &mut buf);
    image::RgbaImage::from_raw(w, h, buf).expect("frame size")
}

/// The geodesic map, a full replace on a fixed ramp: tube cells banded every
/// 10 cells of distance from the heart (alternate light and dark), off-tube
/// cells dark, piles white.
fn dist_map(d: &Dish, dist: &[f32]) -> image::RgbaImage {
    let s = 2u32;
    let mut img = image::RgbaImage::new(W as u32 * s, H as u32 * s);
    for y in 0..H {
        for x in 0..W {
            let i = y * W + x;
            let c = if dist[i].is_finite() {
                let v = (dist[i] / 150.0).min(1.0);
                let k = if (dist[i] as u32 / 10).is_multiple_of(2) { 1.0 } else { 0.6 };
                [((60.0 + 195.0 * v) * k) as u8, ((200.0 - 120.0 * v) * k) as u8, ((255.0 - 200.0 * v) * k) as u8]
            } else if d.trail[i] > d.params.tube {
                [90, 90, 90]
            } else {
                [12, 14, 16]
            };
            for dy in 0..s {
                for dx in 0..s {
                    img.put_pixel(x as u32 * s + dx, y as u32 * s + dy, image::Rgba([c[0], c[1], c[2], 255]));
                }
            }
        }
    }
    for p in &d.piles {
        for (ox, oy) in pixel_physics::mould::disc(p.r) {
            let (x, y) = ((p.x + ox) as u32, (p.y + oy) as u32);
            for dy in 0..s {
                for dx in 0..s {
                    img.put_pixel(x * s + dx, y * s + dy, image::Rgba([255, 255, 255, 255]));
                }
            }
        }
    }
    img
}

fn sheet(shots: &[image::RgbaImage], cols: u32) -> image::RgbaImage {
    let (w, h) = (shots[0].width(), shots[0].height());
    let rows = (shots.len() as u32).div_ceil(cols);
    let mut out = image::RgbaImage::new(w * cols, h * rows);
    for (k, img) in shots.iter().enumerate() {
        let k = k as u32;
        image::imageops::overlay(&mut out, img, ((k % cols) * w) as i64, ((k / cols) * h) as i64);
    }
    out
}

/// A joined, settled organ dish: the joining hand's oats until every bell
/// is joined to the heart, then nobody's hand until the last oats are gone
/// and `settle` more seconds have passed. `None` if the bells never all join
/// within 150 s. Also returns when they joined and how many oats it took.
fn settled(seed: u64, settle: u32) -> Option<(Dish, u64)> {
    let sps = STEPS_PER_SECOND as u64;
    let mut d = organ::new_dish(seed);
    let mut hand = organ::Joiner::new();
    let joined_at;
    loop {
        if d.t >= 150 * sps {
            return None;
        }
        if d.t.is_multiple_of(2) && hand.act(&mut d) {
            joined_at = d.t;
            break;
        }
        organ::step(&mut d);
    }
    while d.piles.iter().any(|p| p.oats) {
        organ::step(&mut d);
    }
    let end = d.t + settle as u64 * sps;
    while d.t < end {
        organ::step(&mut d);
    }
    Some((d, joined_at))
}

/// Step 0: how much each bell's distance from the heart wobbles over a
/// quiet window, on both rulers.
fn step0(a: &Args) {
    let sps = STEPS_PER_SECOND as u64;
    let mut ruler = Ruler { core: a.core, ..Ruler::default() };
    // [ruler][bell] -> per-seed range over the window.
    let mut ranges: [[Vec<f32>; 3]; 2] = Default::default();
    let mut jumps: [[Vec<f32>; 3]; 2] = Default::default();
    let mut drops_all = [0u32; 3];
    let samples_per = a.window as u64 * sps;
    println!("window {} s = {} samples, one per step; beat-jump = biggest change between samples 2 s apart", a.window, samples_per);
    println!("seed  bell   grid min-max range jump |  geo min-max   range jump | gaps");
    for &seed in &a.seeds {
        let Some((mut d, joined_at)) = settled(seed, a.settle) else {
            println!("{seed:4}  never joined all three bells in 150 s");
            continue;
        };
        let opens = d.t;
        if let Some(dir) = &a.look {
            ruler.bells(&d);
            frame_of(&d).save(format!("{dir}/step0-s{seed}-frame.png")).unwrap();
            dist_map(&d, &ruler.geo).save(format!("{dir}/step0-s{seed}-geo.png")).unwrap();
        }
        let mut s: [[Vec<f32>; 3]; 2] = Default::default();
        let mut beat: [[Vec<f32>; 3]; 2] = Default::default();
        let mut gaps = [0u32; 3];
        while d.t < opens + samples_per {
            organ::step(&mut d);
            let on_beat = (d.t - opens).is_multiple_of(organ::BEAT_STEPS);
            for (b, (g, e)) in ruler.bells(&d).into_iter().enumerate() {
                match (g, e) {
                    (Some(g), Some(e)) => {
                        s[0][b].push(g);
                        s[1][b].push(e);
                        if on_beat {
                            beat[0][b].push(g);
                            beat[1][b].push(e);
                        }
                    }
                    _ => gaps[b] += 1,
                }
            }
        }
        println!(
            "  s{seed}: joined at {:.1} s; window {:.1}-{:.1} s",
            joined_at as f32 / sps as f32,
            opens as f32 / sps as f32,
            (opens + samples_per) as f32 / sps as f32
        );
        for b in 0..3 {
            let mut cols = Vec::new();
            for r in 0..2 {
                let v = &s[r][b];
                if v.is_empty() {
                    cols.push("      never reached      ".to_string());
                    continue;
                }
                let lo = v.iter().copied().fold(f32::INFINITY, f32::min);
                let hi = v.iter().copied().fold(0.0, f32::max);
                let jump = beat[r][b].windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f32::max);
                ranges[r][b].push(hi - lo);
                jumps[r][b].push(jump);
                cols.push(format!("{lo:6.1}-{hi:6.1} {:5.1} {jump:4.1}", hi - lo));
            }
            drops_all[b] += gaps[b];
            println!("{seed:4}  {:5} {} | {} | {}", organ::NOTE_NAMES[b], cols[0], cols[1], gaps[b]);
        }
    }
    println!();
    println!("wobble = max - min of the bell's distance over the {} s window, cells of tube", a.window);
    println!("ruler bell   n  wobble min/p90/max   beat-jump min/p90/max   gaps (steps unreached, all seeds)");
    for (r, name) in ["grid", "geo"].iter().enumerate() {
        for b in 0..3 {
            let v = &ranges[r][b];
            let j = &jumps[r][b];
            println!(
                "{name:5} {:5} {:2}  {:5.1} {:5.1} {:5.1}       {:5.1} {:5.1} {:5.1}       {}",
                organ::NOTE_NAMES[b],
                v.len(),
                pct(v, 0.0),
                pct(v, 0.9),
                pct(v, 1.0),
                pct(j, 0.0),
                pct(j, 0.9),
                pct(j, 1.0),
                drops_all[b]
            );
        }
    }
}

/// **The lamp bend**: push the middle of one bell's tube sideways with the
/// lamp and measure that bell's distance against the untouched twin of the
/// same dish. The positive control for step 0: if a hand cannot move the
/// distance on purpose, a zero wobble means nothing.
fn bend(a: &Args) {
    let sps = STEPS_PER_SECOND as u64;
    let ms = 1000.0 / organ::PULSE_SPEED;
    println!(
        "bend: {} bell's tube, lamp {:+} -> {:+} cells across it over {} s, then let go; ms at {} cells/s",
        organ::NOTE_NAMES[a.bell - 1],
        a.from,
        a.to,
        a.hold,
        organ::PULSE_SPEED
    );
    println!("seed  ruler  base   peak  +ms@peak  +5s after  +15s after  kept vs twin, ms  | gaps while lit");
    let mut ruler = Ruler { core: a.core, ..Ruler::default() };
    let mut peak_ms: [Vec<f32>; 2] = Default::default();
    let mut kept_ms: [Vec<f32>; 2] = Default::default();
    for &seed in &a.seeds {
        let Some((mut lit, _)) = settled(seed, a.settle) else {
            println!("{seed:4} never joined");
            continue;
        };
        let mut dark = lit.clone();
        let base = ruler.bells(&lit)[a.bell - 1];
        let route = organ::route_to_bell(&lit, &ruler.grid, a.bell);
        let xy = |i: usize| ((i % W) as f32 + 0.5, (i / W) as f32 + 0.5);
        let n = route.len();
        let (mx, my) = xy(route[n / 2]);
        let (ax, ay) = xy(route[n / 2 - 6.min(n / 2)]);
        let (bx, by) = xy(route[(n / 2 + 6).min(n - 1)]);
        let (tx, ty) = (bx - ax, by - ay);
        let tl = tx.hypot(ty).max(0.01);
        let (mut nx, mut ny) = (-ty / tl, tx / tl);
        let h = &lit.piles[0];
        if (mx + nx - h.x).hypot(my + ny - h.y) < (mx - h.x).hypot(my - h.y) {
            nx = -nx;
            ny = -ny;
        }
        let push_steps = (a.hold * sps as f32) as u64;
        let t0 = lit.t;
        let mut peak = [0.0f32; 2];
        let mut gaps = 0;
        let mut at = [[f32::NAN; 2]; 2];
        let mut shots = Vec::new();
        while lit.t < t0 + push_steps + 15 * sps {
            let k = lit.t - t0;
            if k < push_steps {
                let f = k as f32 / push_steps as f32;
                let o = a.from + (a.to - a.from) * f;
                lit.set_lamp(Some((mx + nx * o, my + ny * o)));
            } else {
                lit.set_lamp(None);
            }
            organ::step(&mut lit);
            organ::step(&mut dark);
            let now = ruler.bells(&lit)[a.bell - 1];
            if k < push_steps + sps {
                match now {
                    (Some(g), Some(e)) => {
                        peak[0] = peak[0].max(g);
                        peak[1] = peak[1].max(e);
                    }
                    _ => gaps += 1,
                }
            }
            let since = (lit.t - t0).saturating_sub(push_steps);
            for (slot, when) in [(0, 5), (1, 15)] {
                if since == when * sps {
                    at[slot] = [now.0.unwrap_or(f32::NAN), now.1.unwrap_or(f32::NAN)];
                }
            }
            if a.look.is_some() && k.is_multiple_of(sps / 2) && k <= push_steps + 4 * sps {
                shots.push(frame_of(&lit));
            }
        }
        if let Some(dir) = &a.look {
            sheet(&shots, 4).save(format!("{dir}/bend-s{seed}.png")).unwrap();
        }
        let twin = ruler.bells(&dark)[a.bell - 1];
        for (r, name) in ["grid", "geo"].iter().enumerate() {
            let b = if r == 0 { base.0 } else { base.1 }.unwrap_or(f32::NAN);
            let tw = if r == 0 { twin.0 } else { twin.1 }.unwrap_or(f32::NAN);
            let kept = (at[1][r] - tw) * ms;
            println!(
                "{seed:4}  {name:5} {b:5.1}  {:5.1}  {:7.0}   {:8.1}  {:9.1}  {kept:15.0}   | {}",
                peak[r],
                (peak[r] - b) * ms,
                at[0][r],
                at[1][r],
                if r == 0 { gaps.to_string() } else { String::new() }
            );
            peak_ms[r].push((peak[r] - b) * ms);
            kept_ms[r].push(kept);
        }
    }
    for (r, name) in ["grid", "geo"].iter().enumerate() {
        println!(
            "{name:5}: note later while pushed, ms min/p50/max {:.0}/{:.0}/{:.0}; still later 15 s after letting go (vs untouched twin) {:.0}/{:.0}/{:.0}  n={}",
            pct(&peak_ms[r], 0.0),
            pct(&peak_ms[r], 0.5),
            pct(&peak_ms[r], 1.0),
            pct(&kept_ms[r], 0.0),
            pct(&kept_ms[r], 0.5),
            pct(&kept_ms[r], 1.0),
            peak_ms[r].len()
        );
    }
}

/// One phase of the scripted performance.
#[derive(Clone, Copy, PartialEq)]
enum Act {
    Alone,
    Lamp,
    LetGo,
    Salt,
    Oats,
}

const SCRIPT: [(Act, &str, u32); 5] = [
    (Act::Alone, "alone", 10),
    (Act::Lamp, "lamp pushes high tube", 6),
    (Act::LetGo, "let go", 10),
    (Act::Oats, "oats between low and mid", 20),
    (Act::Salt, "salt wall across mid tube", 20),
];

/// **The performance**: a settled organ, then a scripted hand through the
/// verbs, with every note counted per phase. The first seed's run is also
/// written out as frames and a WAV (`frames=DIR`).
fn play(a: &Args) {
    let sps = STEPS_PER_SECOND as u64;
    // [phase][bell]: notes, echoes, silent beats, first-arrival times (s).
    type Tally = [[(u32, u32, u32, Vec<f32>); 3]; 5];
    let mut all: Vec<(u64, Tally, u32, u32)> = Vec::new();
    let mut frame_ms: Vec<f32> = Vec::new();
    // Frames whose steps include a beat (the pulse's measuring).
    let mut beat_ms: Vec<f32> = Vec::new();
    for (si, &seed) in a.seeds.iter().enumerate() {
        let Some((d, _)) = settled(seed, a.settle) else {
            println!("{seed:4} never joined");
            continue;
        };
        // Start on a beat.
        let mut o = organ::Organ::from_dish(d);
        while !o.dish.t.is_multiple_of(organ::BEAT_STEPS) {
            o.step();
        }
        let t0 = o.dish.t;
        let first_note = o.played.len();
        let mut tally: Tally = Default::default();
        let write = si == 0 && a.frames.is_some();
        let mut buf = vec![0u8; { let (w, h) = render::frame_size(2); (w * h * 4) as usize }];
        let mut big = vec![0u8; { let (w, h) = render::frame_size(4); (w * h * 4) as usize }];
        let mut frame_no = 0;
        let (on0, off0) = (o.counts.on_beat, o.counts.off_beat);
        for (pi, &(act, _, secs)) in SCRIPT.iter().enumerate() {
            let start = o.dish.t;
            let end = start + secs as u64 * sps;
            let mid = organ::tube_middle(&o.dish, 3);
            // Oats the script drops in this phase: (step, where).
            let mut lure: Vec<(u64, (f32, f32))> = Vec::new();
            match act {
                Act::Salt => {
                    if let Some(((mx, my), (nx, ny))) = organ::tube_middle(&o.dish, 2) {
                        // Across the tube, centred on it, 40 cells long;
                        // then oats just past the wall's far end lead the
                        // regrowth round it (placed a second later, below).
                        let half = 20.0;
                        if o.dish.begin_stroke(pixel_physics::mould::Tool::Salt, mx - nx * half, my - ny * half) {
                            o.dish.drag_stroke(mx + nx * half, my + ny * half);
                            o.dish.end_stroke();
                        }
                        lure = vec![(start + sps, (mx + nx * (half + 7.0), my + ny * (half + 7.0)))];
                    }
                }
                Act::Oats => {
                    // Stepping stones for a bridge from the low bell's tube
                    // to the mid bell's, off to the side away from the
                    // heart: one beside each tube, then one between them.
                    let (l, m) = (&o.dish.piles[1], &o.dish.piles[2]);
                    let along = |f: f32| (l.x + (m.x - l.x) * f, l.y + (m.y - l.y) * f - 8.0);
                    lure = vec![(start, along(0.25)), (start, along(0.75)), (start + 6 * sps, along(0.5))];
                }
                _ => {}
            }
            while o.dish.t < end {
                for &(when, (x, y)) in &lure {
                    if o.dish.t == when {
                        o.dish.oats_left = o.dish.oats_left.max(1);
                        o.dish.place_oats(x, y);
                    }
                }
                if act == Act::Lamp {
                    if let Some(((mx, my), (nx, ny))) = mid {
                        let f = ((o.dish.t - start) as f32 / (3 * sps) as f32).min(1.0);
                        let off = a.from + (a.to - a.from) * f;
                        o.dish.set_lamp(Some((mx + nx * off, my + ny * off)));
                    }
                } else {
                    o.dish.set_lamp(None);
                }
                let t = std::time::Instant::now();
                let beat = o.dish.t.is_multiple_of(organ::BEAT_STEPS) || (o.dish.t + 1).is_multiple_of(organ::BEAT_STEPS);
                for _ in 0..2 {
                    for n in o.step() {
                        if let organ::Voice::Bell(b) = n.voice {
                            if n.echo {
                                tally[pi][b].1 += 1;
                            } else {
                                tally[pi][b].0 += 1;
                                tally[pi][b].3.push(n.after);
                            }
                        }
                    }
                }
                organ::draw(&o, &View { keys: false, ..View::default() }, 4, &mut big);
                let ms = t.elapsed().as_secs_f32() * 1000.0;
                frame_ms.push(ms);
                if beat {
                    beat_ms.push(ms);
                }
                if write && (o.dish.t - t0).is_multiple_of(8) {
                    organ::draw(&o, &View { keys: false, ..View::default() }, 2, &mut buf);
                    let (w, h) = render::frame_size(2);
                    let img = image::RgbaImage::from_raw(w, h, buf.clone()).unwrap();
                    img.save(format!("{}/f{frame_no:05}.png", a.frames.as_ref().unwrap())).unwrap();
                    frame_no += 1;
                }
            }
            let beats = (secs as u64 * sps).div_ceil(organ::BEAT_STEPS) as u32;
            for t in tally[pi].iter_mut() {
                t.2 = beats.saturating_sub(t.0);
            }
        }
        let (on, off) = (o.counts.on_beat - on0, o.counts.off_beat - off0);
        if write {
            let notes: Vec<organ::Note> = o.played[first_note..]
                .iter()
                .map(|n| organ::Note { step: n.step - t0, ..*n })
                .collect();
            let secs = (o.dish.t - t0) as f32 / sps as f32;
            let wav = organ::render_notes(&notes, secs + 1.5, 44_100);
            let path = format!("{}/organ-s{seed}.wav", a.frames.as_ref().unwrap());
            pixel_physics::audio::wav::write(std::path::Path::new(&path), &wav, 2, 44_100).unwrap();
            let mut log = String::new();
            for n in &notes {
                log += &format!(
                    "{:.3} {:?} gain {:.2} after {:.3}{}\n",
                    n.step as f32 / sps as f32,
                    n.voice,
                    n.gain,
                    n.after,
                    if n.echo { " echo" } else { "" }
                );
            }
            std::fs::write(format!("{}/organ-s{seed}-notes.txt", a.frames.as_ref().unwrap()), log).unwrap();
            println!("wrote {frame_no} frames and {path} ({secs:.1} s, {} notes)", notes.len());
        }
        all.push((seed, tally, on, off));
    }
    // Per seed, per phase: each bell's notes (+echoes) and mean time.
    println!();
    println!("each cell: notes+echoes / beats, mean first-arrival time in ms after the heart's thump");
    for (seed, tally, on, off) in &all {
        println!("seed {seed}  (on the target beat {on}, off {off})");
        for (pi, &(_, name, secs)) in SCRIPT.iter().enumerate() {
            let beats = (secs as u64 * sps).div_ceil(organ::BEAT_STEPS);
            let mut row = format!("  {name:28}");
            for (b, (n, e, _, ts)) in tally[pi].iter().enumerate() {
                let mean = if ts.is_empty() { f32::NAN } else { ts.iter().sum::<f32>() / ts.len() as f32 * 1000.0 };
                row += &format!(" | {:4} {n}+{e}/{beats} {mean:5.0}ms", organ::NOTE_NAMES[b]);
            }
            println!("{row}");
        }
    }
    // Across seeds: how far each verb moved its bell, against "alone".
    println!();
    println!("across {} seeds: change in the bell's mean time against the same seed alone, ms (min / median / max), and notes per 10 s", all.len());
    for (pi, &(_, name, secs)) in SCRIPT.iter().enumerate().skip(1) {
        let mut row = format!("  {name:28}");
        for b in 0..3 {
            let mut shifts = Vec::new();
            let mut per10 = Vec::new();
            let mut echoes = 0;
            for (_, tally, _, _) in &all {
                let m = |ts: &Vec<f32>| if ts.is_empty() { None } else { Some(ts.iter().sum::<f32>() / ts.len() as f32) };
                if let (Some(x), Some(y)) = (m(&tally[0][b].3), m(&tally[pi][b].3)) {
                    shifts.push((y - x) * 1000.0);
                }
                per10.push(tally[pi][b].0 as f32 * 10.0 / secs as f32);
                echoes += tally[pi][b].1;
            }
            row += &format!(
                " | {:4} {:+4.0}/{:+4.0}/{:+4.0}  {:.1}/10s echoes {echoes}",
                organ::NOTE_NAMES[b],
                pct(&shifts, 0.0),
                pct(&shifts, 0.5),
                pct(&shifts, 1.0),
                per10.iter().sum::<f32>() / per10.len().max(1) as f32
            );
        }
        println!("{row}");
    }
    let mean = frame_ms.iter().sum::<f32>() / frame_ms.len().max(1) as f32;
    println!(
        "frame cost (2 steps + draw at scale 4, the window's): mean {mean:.2} ms, p99 {:.2} ms, worst {:.2} ms over {} frames",
        pct(&frame_ms, 0.99),
        pct(&frame_ms, 1.0),
        frame_ms.len()
    );
    println!(
        "  of which frames with a beat in them: mean {:.2} ms, worst {:.2} ms over {} frames",
        beat_ms.iter().sum::<f32>() / beat_ms.len().max(1) as f32,
        pct(&beat_ms, 1.0),
        beat_ms.len()
    );
}

fn main() {
    let a = parse();
    if a.play {
        play(&a);
    } else if a.bend {
        bend(&a);
    } else if a.step0 {
        step0(&a);
    } else {
        eprintln!("nothing to do: pass step0=1 or bend=1");
    }
}
