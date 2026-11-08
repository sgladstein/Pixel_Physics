//! **Firedamp probe harness.** Scripted play of `--bin firedamp` with no
//! window: a do-nothing control, a planned player and a random clicker, on
//! paired seeds, with the counts, the frame cost and GIFs of the window's own
//! pixels (`firedamp::paint::paint` is the function the window draws with).
//!
//! ```text
//! cargo run --release --example firedampprobe -- seeds=12 out=DIR gif=1
//! ```
//!
//! - `seeds=N` -- seeds 1..=N, every policy on every seed (paired).
//! - `out=DIR` -- where GIFs and the CSV go (default: the temp dir).
//! - `gif=1` -- write a whole-shift time-lapse per policy for seed 1, and a
//!   real-time clip of the planned player's first bang.
//! - `only=idle|plan|random` -- one policy.
//! - `previews=1` -- instead of the table: what the match preview (the
//!   outlined patch, POP / BANG / BIG BANG, the blast circle) showed at each
//!   of the planned player's matches on every seed against what then
//!   happened, and stills of seed 1 with and without the preview drawn.
//!
//! **Run it from the repository root**, like every harness here.
//!
//! The planned player is a script of what the screen tells a player: a fan
//! churning the sump, a fuse on the ore seam that lights itself when the gas
//! over it turns yellow, matches when the fuses run out, and a door slammed
//! on the climb to the faces while gas is creeping up it. The random player
//! clicks a random tool on a random open cell every 1.5 to 4 seconds. Neither
//! reads anything a player could not see.

use std::time::Instant;

use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, RgbaImage};
use pixel_physics::firedamp::gas::{self, idx};
use pixel_physics::firedamp::paint::{self, FH, FW};
use pixel_physics::firedamp::{Event, FaceState, Game, Outcome, Risk, Tool, LEVEL_TICKS, TICKS_PER_SECOND};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Policy {
    Idle,
    Plan,
    Random,
}

impl Policy {
    fn name(self) -> &'static str {
        match self {
            Policy::Idle => "idle",
            Policy::Plan => "plan",
            Policy::Random => "random",
        }
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let mut z = self.0;
        z = (z ^ (z >> 33)).wrapping_mul(0xff51afd7ed558ccd);
        z ^ (z >> 33)
    }
    fn f(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// Where the planned player puts things. Read off the level's picture.
const SUMP_FAN: (i32, i32) = (106, 110);
const SUMP_AIM: (i32, i32) = (126, 121);
const SEAM_FUSE: (i32, i32) = (122, 122);
/// A spot on the climb to the faces: gas here means it is spilling over.
const CLIMB_WATCH: (i32, i32) = (152, 106);
const CLIMB_DOOR: (i32, i32) = (156, 104);
/// Where the planned player drops its matches (above the seam).
const PLAN_MATCH: (i32, i32) = (SEAM_FUSE.0, SEAM_FUSE.1 - 14);

#[derive(Default)]
struct Planner {
    next_light: u64,
    door: Option<usize>,
    /// Light any yellow at all, not only a pocket worth a bang (the
    /// playtest's "eager" player): the source of POPs for `previews=1`.
    eager: bool,
}

/// How much gas the planned player waits for before lighting (the inside
/// gas of the yellow pocket, `Game::predict`). A player judges the same by
/// how much of the sump has turned yellow.
const GREED: f32 = 30.0;

/// **The planned player**: a fan churning the sump from the start, so the
/// pool never settles thick; a match into the yellow over the seam once the
/// yellow pocket is big enough to be worth a bang; fuses on the seam once
/// the matches are gone (a fuse lights itself the moment the gas turns
/// yellow, which is early and small). A door on the climb if gas spills
/// toward the faces anyway.
fn plan(g: &mut Game, p: &mut Planner) {
    let t = g.tick;
    if t == TICKS_PER_SECOND / 2 {
        let a = ((SUMP_AIM.1 - SUMP_FAN.1) as f32).atan2((SUMP_AIM.0 - SUMP_FAN.0) as f32);
        g.place_fan(SUMP_FAN.0, SUMP_FAN.1, a);
    }
    if t >= p.next_light && g.fuses.is_empty() {
        let (x, y) = (SEAM_FUSE.0, SEAM_FUSE.1 - 4);
        debug_assert_eq!((x, y - 10), PLAN_MATCH);
        let greed = if p.eager { 0.0 } else { GREED };
        let ready = g.predict(x, y).is_some_and(|(_, inner)| inner >= greed);
        if g.stock.matches > 0 {
            if ready {
                g.throw_match(x, y - 10);
                p.next_light = t + TICKS_PER_SECOND;
            }
        } else if g.stock.fuses > 0 {
            g.place_fuse(SEAM_FUSE.0, SEAM_FUSE.1);
            p.next_light = t + TICKS_PER_SECOND;
        }
    }
    // Gas spilling up the climb: slam the door on it; open it once the
    // climb below is clear again, so the faces get their air back.
    let spilling = g.gas.max_near(CLIMB_WATCH.0, CLIMB_WATCH.1, 3) > gas::BURN_LO;
    match p.door {
        None if spilling => {
            if g.place_door(CLIMB_DOOR.0, CLIMB_DOOR.1) {
                p.door = Some(g.doors.len() - 1);
            }
        }
        // Shut while gas spills over the climb, open again once it stops.
        Some(d) if d < g.doors.len() && g.doors[d].closed != spilling => {
            g.tap_door(d);
        }
        _ => {}
    }
}

fn random_play(g: &mut Game, rng: &mut Rng, next: &mut u64) {
    if g.tick < *next {
        return;
    }
    *next = g.tick + (90.0 + rng.f() * 150.0) as u64;
    // A random open cell, as a player clicking around the tunnels would.
    let (x, y) = loop {
        let (x, y) = ((rng.f() * gas::W as f32) as i32, (rng.f() * gas::H as f32) as i32);
        if g.cell_open(x, y) || g.doors.iter().any(|d| d.cells.contains(&idx(x, y))) {
            break (x, y);
        }
    };
    g.tool = Tool::ALL[(rng.f() * 4.0) as usize % 4];
    g.press(x, y);
    if g.aiming.is_some() {
        let a = rng.f() * std::f32::consts::TAU;
        g.release(x + (a.cos() * 10.0) as i32, y + (a.sin() * 10.0) as i32);
    }
}

#[derive(Default)]
struct Run {
    first_blown: Option<f32>,
    standing: usize,
    ore: u32,
    ore_blasted: u32,
    carts: u32,
    stars: u32,
    working_s: f32,
    step_ms_mean: f64,
    step_ms_worst: f64,
    burn_step_worst: f64,
    paint_ms_worst: f64,
    frame_worst: f64,
}

fn half(frame: &[u8]) -> RgbaImage {
    let (w, h) = (FW / 2, FH / 2);
    let mut img = RgbaImage::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            let i = ((y * 2) * FW + x * 2) * 4;
            img.put_pixel(x as u32, y as u32, image::Rgba([frame[i], frame[i + 1], frame[i + 2], 255]));
        }
    }
    img
}

fn gif(path: &str) -> GifEncoder<std::fs::File> {
    let f = std::fs::File::create(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut enc = GifEncoder::new_with_speed(f, 10);
    enc.set_repeat(Repeat::Infinite).unwrap();
    enc
}

fn run(seed: u64, policy: Policy, out: &str, gifs: bool) -> (Run, pixel_physics::firedamp::Counts) {
    let mut g = Game::new(seed);
    let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) ^ 0xF1DE);
    let mut next_random = 2 * TICKS_PER_SECOND;
    let mut planner = Planner::default();
    let mut frame = vec![0u8; FW * FH * 4];
    let mut lapse = gifs.then(|| gif(&format!("{out}/firedamp-{}-seed{seed}-timelapse.gif", policy.name())));
    let mut clip: Option<(GifEncoder<std::fs::File>, u64)> = None;
    let mut clip_done = false;
    let mut r = Run::default();
    let mut total_ms = 0.0;
    while !g.over {
        // Events from the last tick are what the planner reacts to.
        let seen: Vec<Event> = g.events.clone();
        g.events.clear();
        g.events.extend(seen);
        match policy {
            Policy::Idle => {}
            Policy::Plan => plan(&mut g, &mut planner),
            Policy::Random => random_play(&mut g, &mut rng, &mut next_random),
        }
        g.events.clear();
        let t0 = Instant::now();
        g.step();
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        total_ms += ms;
        r.step_ms_worst = r.step_ms_worst.max(ms);
        let burning = g.gas.burning_count() > 0 || !g.blasts.is_empty();
        if burning {
            r.burn_step_worst = r.burn_step_worst.max(ms);
        }
        let banged = g.events.iter().any(|e| matches!(e, Event::Bang { .. }));
        // Paint cost is sampled every 10th tick (the window draws once per
        // frame, so this is the draw half of a frame).
        let mut pms = 0.0;
        if g.tick.is_multiple_of(10) {
            let t1 = Instant::now();
            // The planned player holds a match over its spot, so its frames
            // carry the preview (and its cost) as the window's would.
            if policy == Policy::Plan && g.stock.matches > 0 {
                g.tool = Tool::Match;
                g.aim_preview(Some(PLAN_MATCH));
            } else {
                g.preview = None;
            }
            paint::paint(&g, &mut frame, None);
            pms = t1.elapsed().as_secs_f64() * 1000.0;
            r.paint_ms_worst = r.paint_ms_worst.max(pms);
        }
        r.frame_worst = r.frame_worst.max(ms + pms);
        if let Some(enc) = &mut lapse {
            if g.tick.is_multiple_of(60) {
                if pms == 0.0 {
                    paint::paint(&g, &mut frame, None);
                }
                enc.encode_frame(Frame::from_parts(half(&frame), 0, 0, Delay::from_numer_denom_ms(60, 1))).unwrap();
            }
        }
        // A real-time clip around the first bang of the run: 1 s before
        // (from a rolling start) is not kept, so the clip starts at the bang
        // and runs 3 s, every 2 ticks.
        if gifs && banged && clip.is_none() && !clip_done {
            clip = Some((gif(&format!("{out}/firedamp-{}-seed{seed}-first-bang.gif", policy.name())), g.tick));
        }
        if let Some((enc, start)) = &mut clip {
            if g.tick.is_multiple_of(2) {
                if g.preview.is_some() {
                    g.aim_preview(Some(PLAN_MATCH));
                }
                paint::paint(&g, &mut frame, None);
                enc.encode_frame(Frame::from_parts(half(&frame), 0, 0, Delay::from_numer_denom_ms(33, 1))).unwrap();
            }
            if g.tick >= *start + 3 * TICKS_PER_SECOND {
                clip = None;
                clip_done = true;
            }
        }
    }
    if gifs {
        paint::paint(&g, &mut frame, None);
        image::save_buffer(format!("{out}/firedamp-{}-seed{seed}-end.png", policy.name()), &frame, FW as u32, FH as u32, image::ColorType::Rgba8).unwrap();
        if let Some(enc) = &mut lapse {
            for _ in 0..15 {
                enc.encode_frame(Frame::from_parts(half(&frame), 0, 0, Delay::from_numer_denom_ms(100, 1))).unwrap();
            }
        }
    }
    r.first_blown = g.faces.iter().filter_map(|f| f.blown_at).min().map(|t| t as f32 / TICKS_PER_SECOND as f32);
    r.standing = g.faces_standing();
    r.ore = g.ore();
    r.ore_blasted = g.ore_blasted;
    r.carts = g.faces.iter().map(|f| f.carts).sum();
    r.stars = g.stars();
    r.working_s = g.faces.iter().map(|f| f.working_ticks).sum::<u64>() as f32 / TICKS_PER_SECOND as f32;
    r.step_ms_mean = total_ms / g.tick.max(1) as f64;
    let _ = FaceState::Working;
    let _ = LEVEL_TICKS;
    (r, g.counts)
}

fn main() {
    let mut seeds = 12u64;
    let mut out = std::env::temp_dir().display().to_string();
    let mut gifs = false;
    let mut only: Option<Policy> = None;
    let mut previews_only = false;
    for a in std::env::args().skip(1) {
        let (k, v) = a.split_once('=').unwrap_or((a.as_str(), ""));
        match k {
            "seeds" => seeds = v.parse().expect("seeds=N"),
            "out" => out = v.to_string(),
            "gif" => gifs = v == "1",
            "previews" => previews_only = v == "1",
            "only" => {
                only = Some(match v {
                    "idle" => Policy::Idle,
                    "plan" => Policy::Plan,
                    "random" => Policy::Random,
                    _ => panic!("only=idle|plan|random"),
                })
            }
            _ => panic!("unknown argument {a}: seeds=N out=DIR gif=1 only=POLICY previews=1"),
        }
    }
    println!("firedampprobe seeds=1..={seeds} out={out} gif={gifs} only={only:?}");
    std::fs::create_dir_all(&out).expect("cannot make the out directory");
    if previews_only {
        previews(seeds, &out);
        return;
    }
    let policies: Vec<Policy> = [Policy::Idle, Policy::Plan, Policy::Random].into_iter().filter(|p| only.is_none_or(|o| o == *p)).collect();
    let mut csv = String::from("policy,seed,first_face_blown_s,faces_standing,stars,ore,ore_blasted,carts,face_working_s,bangs,biggest_bang_gas,lights,matches,fizzle_thick,fizzle_thin,fuses_fired,door_taps,predicted_cells,burnt_inside,burnt_outside,step_ms_mean,step_ms_worst,burn_step_ms_worst,paint_ms_worst,frame_ms_worst\n");
    let mut table: Vec<(Policy, u64, Run)> = Vec::new();
    let mut legible = vec![(0u64, 0u64, 0u64); policies.len()];
    for seed in 1..=seeds {
        for &p in &policies {
            let (r, c) = run(seed, p, &out, gifs && seed == 1);
            println!(
                "{:6} seed {seed:2}: first face blown {:>6} standing {} stars {} ore {:3} (blasts {:3} carts {:3}) bangs {:2} biggest {:5.1} lights {:2} matches {:2} fizzles thick/thin {}/{} flame in/out of yellow {}/{} of {} | step mean {:.2} worst {:.2} burn-worst {:.2} paint-worst {:.2} ms",
                p.name(),
                r.first_blown.map_or("never".into(), |t| format!("{t:.0}s")),
                r.standing,
                r.stars,
                r.ore,
                r.ore_blasted,
                r.carts,
                c.bangs,
                c.biggest_bang,
                c.lights,
                c.matches,
                c.fizzle_thick,
                c.fizzle_thin,
                c.burnt_inside,
                c.burnt_outside,
                c.predicted_cells,
                r.step_ms_mean,
                r.step_ms_worst,
                r.burn_step_worst,
                r.paint_ms_worst
            );
            csv += &format!(
                "{},{seed},{},{},{},{},{},{},{:.1},{},{:.1},{},{},{},{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.3}\n",
                p.name(),
                r.first_blown.map_or("".into(), |t| format!("{t:.1}")),
                r.standing,
                r.stars,
                r.ore,
                r.ore_blasted,
                r.carts,
                r.working_s,
                c.bangs,
                c.biggest_bang,
                c.lights,
                c.matches,
                c.fizzle_thick,
                c.fizzle_thin,
                c.fuses_fired,
                c.door_taps,
                c.predicted_cells,
                c.burnt_inside,
                c.burnt_outside,
                r.step_ms_mean,
                r.step_ms_worst,
                r.burn_step_worst,
                r.paint_ms_worst,
                r.frame_worst
            );
            legible[policies.iter().position(|q| *q == p).unwrap()].0 += c.predicted_cells as u64;
            legible[policies.iter().position(|q| *q == p).unwrap()].1 += c.burnt_inside as u64;
            legible[policies.iter().position(|q| *q == p).unwrap()].2 += c.burnt_outside as u64;
            table.push((p, seed, r));
        }
    }
    std::fs::write(format!("{out}/firedampprobe.csv"), &csv).unwrap();
    // Paired comparison: plan against each other policy, seed by seed, on
    // ore and on stars. Sign test, two-sided, ties dropped.
    for other in [Policy::Idle, Policy::Random] {
        if !policies.contains(&Policy::Plan) || !policies.contains(&other) {
            continue;
        }
        for (what, key) in [("ore", 0usize), ("stars", 1usize)] {
            let (mut win, mut lose) = (0u32, 0u32);
            for seed in 1..=seeds {
                let get = |p: Policy| table.iter().find(|r| r.0 == p && r.1 == seed).map(|r| if key == 0 { r.2.ore as f64 } else { r.2.stars as f64 }).unwrap();
                let (a, b) = (get(Policy::Plan), get(other));
                if a > b {
                    win += 1;
                } else if a < b {
                    lose += 1;
                }
            }
            println!("plan vs {:6} on {what:5}: plan better on {win}, worse on {lose}, ties {} -- sign test p = {:.4}", other.name(), seeds as u32 - win - lose, sign_p(win, lose));
        }
    }
    for (k, p) in policies.iter().enumerate() {
        let (pred, inside, outside) = legible[k];
        if pred > 0 {
            println!(
                "{:6}: flame burnt {:.0}% of the yellow pocket it was lit in; {:.0}% of what burnt was outside that pocket ({pred} cells predicted)",
                p.name(),
                100.0 * inside as f64 / pred as f64,
                100.0 * outside as f64 / (inside + outside).max(1) as f64
            );
        }
    }
    let worst = table.iter().map(|r| r.2.frame_worst).fold(0.0, f64::max);
    let mean = table.iter().map(|r| r.2.step_ms_mean).sum::<f64>() / table.len() as f64;
    println!("frame cost over every run: mean tick {mean:.2} ms, worst tick+draw {worst:.2} ms");
    println!("wrote {out}/firedampprobe.csv");
}

/// Two-sided sign test.
fn sign_p(win: u32, lose: u32) -> f64 {
    let n = win + lose;
    if n == 0 {
        return 1.0;
    }
    let k = win.min(lose);
    let mut tail = 0.0;
    for i in 0..=k {
        tail += binom(n, i);
    }
    (2.0 * tail / 2f64.powi(n as i32)).min(1.0)
}

fn binom(n: u32, k: u32) -> f64 {
    let mut r = 1.0;
    for i in 0..k {
        r = r * (n - i) as f64 / (i + 1) as f64;
    }
    r
}

fn save(path: &str, frame: &[u8]) {
    image::save_buffer(path, frame, FW as u32, FH as u32, image::ColorType::Rgba8).unwrap();
}

/// **Does the match preview tell the truth?** At every match the planned
/// player throws, read what the preview would have shown at that spot, then
/// watch the next two seconds: did it light, did it bang, did a face go.
/// Prints the agreement table; writes seed 1's stills with and without the
/// preview (same frame), one second later, and a lamp-danger case found by
/// scanning the do-nothing run.
fn previews(seeds: u64, out: &str) {
    let mut frame = vec![0u8; FW * FH * 4];
    // (preview said, what happened) counts.
    let mut agree = std::collections::BTreeMap::<(String, String), u32>::new();
    let mut lost_said = 0;
    let mut lost_seen = 0;
    // 0 the planned player, 1 the eager one, 2 the random clicker (whose
    // matches land anywhere: the POPs and fizzles come from it).
    for (seed, who) in (1..=seeds).flat_map(|s| [(s, 0), (s, 1), (s, 2)]) {
        let eager = who == 1;
        let mut g = Game::new(seed);
        g.tool = Tool::Match;
        let mut planner = Planner { eager, ..Planner::default() };
        let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) ^ 0xF1DE);
        let mut next_random = 2 * TICKS_PER_SECOND;
        // (tick thrown, what the preview said, face-losses foreseen)
        let mut watching: Vec<(u64, String, bool)> = Vec::new();
        let mut bangs_at: Vec<u64> = Vec::new();
        let mut lit_at: Vec<u64> = Vec::new();
        let mut shots = 0;
        let mut after_shot: Option<(u64, usize)> = None;
        let blown_before = |g: &Game| g.faces.iter().filter(|f| f.state == FaceState::Blown).count();
        let mut blown_log: Vec<(u64, usize)> = Vec::new();
        while !g.over {
            let before = g.counts.matches;
            if who == 2 {
                random_play(&mut g, &mut rng, &mut next_random);
            } else {
                plan(&mut g, &mut planner);
                g.tool = Tool::Match;
            }
            if g.counts.matches > before {
                let m = g.matches.last().expect("a match was just thrown");
                let (mx, my) = (m.x as i32, m.y as i32);
                // The stock already paid for this one; preview as if in hand.
                g.stock.matches += 1;
                let p = g.preview_at(Tool::Match, mx, my);
                g.stock.matches -= 1;
                let said = match &p {
                    None => "nothing".to_string(),
                    Some(p) => match p.outcome {
                        Outcome::Burns => p.size.label().to_string(),
                        Outcome::FizzleThick => "fizzle thick".into(),
                        Outcome::FizzleThin => "goes out".into(),
                        Outcome::Waits => "waits".into(),
                    },
                };
                let foresees_loss = p.as_ref().is_some_and(|p| p.risk.contains(&Risk::Lost));
                watching.push((g.tick, said, foresees_loss));
                if seed == 1 && who == 0 && shots < 3 {
                    shots += 1;
                    g.preview = p;
                    paint::paint(&g, &mut frame, Some((mx, my)));
                    save(&format!("{out}/preview-light{shots}-with.png"), &frame);
                    g.preview = None;
                    paint::paint(&g, &mut frame, Some((mx, my)));
                    save(&format!("{out}/preview-light{shots}-without.png"), &frame);
                    after_shot = Some((g.tick + TICKS_PER_SECOND, shots));
                }
            }
            g.step();
            for e in &g.events {
                match e {
                    Event::Bang { .. } => bangs_at.push(g.tick),
                    Event::Lit { .. } => lit_at.push(g.tick),
                    _ => {}
                }
            }
            g.events.clear();
            let b = blown_before(&g);
            if blown_log.last().map_or(b > 0, |l| l.1 != b) {
                blown_log.push((g.tick, b));
            }
            if let Some((t, k)) = after_shot {
                if g.tick == t {
                    paint::paint(&g, &mut frame, None);
                    save(&format!("{out}/preview-light{k}-1s-later.png"), &frame);
                    after_shot = None;
                }
            }
        }
        for (t0, said, foresees_loss) in watching {
            let win = |v: &[u64], s: u64| v.iter().any(|&t| t > t0 && t <= t0 + s * TICKS_PER_SECOND);
            let happened = if !win(&lit_at, 2) {
                "no light"
            } else if win(&bangs_at, 2) {
                "bang"
            } else {
                "burnt, no bang"
            };
            let lost = blown_log.iter().any(|&(t, _)| t > t0 && t <= t0 + 2 * TICKS_PER_SECOND);
            if lost && !foresees_loss {
                let when: Vec<String> = blown_log.iter().filter(|&&(t, _)| t > t0 && t <= t0 + 2 * TICKS_PER_SECOND).map(|&(t, n)| format!("{:.1} s after (faces down {n})", (t - t0) as f32 / TICKS_PER_SECOND as f32)).collect();
                println!("  unforeseen loss: seed {seed} player {who} match at {:.1} s said {said}: {}", t0 as f32 / TICKS_PER_SECOND as f32, when.join(", "));
            }
            lost_said += foresees_loss as u32;
            lost_seen += lost as u32;
            *agree.entry((said, happened.to_string())).or_default() += 1;
        }
    }
    println!("preview at each match of the planned, eager and random players (seeds 1..={seeds}) -> what the next 2 s did:");
    for ((said, happened), n) in &agree {
        println!("  said {said:12} -> {happened:15} {n:4}");
    }
    println!("  a face foreseen lost: {lost_said}; a face actually lost within 2 s: {lost_seen}");

    // A lamp in danger, as a new player meets it: seed 1 with one fan in
    // the sump left at its default aim (right, toward the faces), which the
    // playtest found shoves the gas onto the low lamp. Scan for the first
    // moment some spot shows a light that would singe a face, and the
    // first moment a match dropped into the sump's floor would fizzle thick.
    let mut g = Game::new(1);
    let (mut amber, mut thick) = (false, false);
    while !g.over && !(amber && thick) && g.tick < 60 * TICKS_PER_SECOND {
        if g.tick == 3 * TICKS_PER_SECOND {
            g.place_fan(SUMP_FAN.0, SUMP_FAN.1, 0.0);
        }
        g.step();
        g.tool = Tool::Match;
        if !g.tick.is_multiple_of(5) {
            continue;
        }
        if !thick {
            if let Some(p) = g.preview_at(Tool::Match, 122, 119).filter(|p| p.outcome == Outcome::FizzleThick) {
                g.preview = Some(p);
                paint::paint(&g, &mut frame, Some((122, 119)));
                save(&format!("{out}/preview-fizzle-thick.png"), &frame);
                println!("fizzle-thick still at {:.1} s", g.tick as f32 / TICKS_PER_SECOND as f32);
                thick = true;
            }
        }
        if !amber {
            if let Some((x, y)) = danger(&mut g, Risk::Singed) {
                paint::paint(&g, &mut frame, Some((x, y)));
                save(&format!("{out}/preview-lamp-singed.png"), &frame);
                println!("lamp-singed still at {:.1} s, pointer ({x},{y})", g.tick as f32 / TICKS_PER_SECOND as f32);
                amber = true;
            }
        }
        g.preview = None;
    }
    planted_red(out, &mut frame);
    // A light that costs a face: the random clicker's mines, scanned twice a
    // second until some spot shows one. Rare, because the lamp itself lights
    // yellow that reaches it before anyone can, so a red outline needs a
    // bang whose blast reaches a lamp.
    'seeds: for seed in 1..=seeds {
        let mut g = Game::new(seed);
        let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) ^ 0xF1DE);
        let mut next_random = 2 * TICKS_PER_SECOND;
        while !g.over {
            random_play(&mut g, &mut rng, &mut next_random);
            g.step();
            if g.tick.is_multiple_of(30) {
                let tool = g.tool;
                g.tool = Tool::Match;
                if let Some((x, y)) = danger(&mut g, Risk::Lost) {
                    paint::paint(&g, &mut frame, Some((x, y)));
                    save(&format!("{out}/preview-lamp-lost.png"), &frame);
                    println!("lamp-lost still: random player, seed {seed}, {:.1} s, pointer ({x},{y})", g.tick as f32 / TICKS_PER_SECOND as f32);
                    break 'seeds;
                }
                g.tool = tool;
                g.preview = None;
            }
        }
    }
}

/// The red case made on purpose, as `a_red_preview_costs_the_face_it_names`
/// makes it: a burnable pocket planted in the low face's gallery.
fn planted_red(out: &str, frame: &mut [u8]) {
    let mut g = Game::new(1);
    for _ in 0..5 * TICKS_PER_SECOND {
        g.step();
    }
    g.plant_pocket(176, 106, 6, 0.35);
    g.tool = Tool::Match;
    g.preview = g.preview_at(Tool::Match, 176, 103);
    paint::paint(&g, frame, Some((176, 103)));
    save(&format!("{out}/preview-lamp-lost-planted.png"), frame);
    g.preview = None;
    g.throw_match(176, 103);
    for _ in 0..TICKS_PER_SECOND {
        g.step();
    }
    paint::paint(&g, frame, None);
    save(&format!("{out}/preview-lamp-lost-planted-1s-later.png"), frame);
    println!("planted red: low face {:?} one second after the match", g.faces[0].state);
}

/// The first spot (scanning the mine's middle and right on a 2-cell grid)
/// where a match would burn a patch of more than 20 cells at this risk to
/// some face. Leaves that preview in `g.preview`.
fn danger(g: &mut Game, want: Risk) -> Option<(i32, i32)> {
    for y in (40..125).step_by(2) {
        for x in (100..245).step_by(2) {
            let Some(p) = g.preview_at(Tool::Match, x, y) else { continue };
            if p.outcome == Outcome::Burns && p.cells.len() > 20 && p.risk.contains(&want) {
                g.preview = Some(p);
                return Some((x, y));
            }
        }
    }
    None
}
