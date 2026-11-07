//! **Seep's headless harness** (probe 4): does doing nothing lose, does a
//! plan beat random play, and is the mould used or decoration?
//!
//! ```text
//! cargo run --release --example seepprobe -- seeds=1..6 players=nothing,planned,gasonly,nomould,random
//! cargo run --release --example seepprobe -- seeds=1 players=nothing,planned gif=out/seep secs=240
//! ```
//!
//! Every player plays the same seeds (paired). Per run it prints carts,
//! stars, lamps blown and when, face-seconds worked, cracks capped, gas
//! eaten by the mould against gas burnt and vented, every tool's use count
//! split at the first minute (the stop rule), and the mean and worst tick --
//! the simulation step *plus the full repaint*, since this game never has a
//! settled frame to skip.
//!
//! `gif=PREFIX` writes `PREFIX-<player>-s<seed>.gif` (the real window's
//! frames, one cell per pixel, every 20 ticks) and PNG stills of the full
//! two-pixel window at fixed times.

use pixel_physics::seep::draw::{self, FH, FW};
use pixel_physics::seep::script::Player;
use pixel_physics::seep::{Game, LampState, Tool, LEVEL_TICKS, TICKS_PER_SECOND};
use std::time::Instant;

struct Run {
    player: String,
    seed: u64,
    carts: u32,
    stars: u32,
    blown: u32,
    first_blown_s: Option<f32>,
    face_s: f32,
    capped: usize,
    open: usize,
    eaten: f64,
    burnt: f64,
    vented: f64,
    emitted: f64,
    left: f64,
    buried: f64,
    singes: u32,
    explosions: u32,
    uses_early: [usize; 5],
    uses_late: [usize; 5],
    tubes_peak: usize,
    tubes_burnt: u64,
    tubes_cut: u64,
    mean_ms: f64,
    worst_ms: f64,
    worst_burn_ms: f64,
}

fn arg(name: &str, default: &str) -> String {
    for a in std::env::args().skip(1) {
        if let Some(v) = a.strip_prefix(&format!("{name}=")) {
            return v.to_string();
        }
    }
    default.to_string()
}

fn seeds(spec: &str) -> Vec<u64> {
    if let Some((a, b)) = spec.split_once("..") {
        (a.parse().unwrap()..=b.parse().unwrap()).collect()
    } else {
        spec.split(',').map(|s| s.parse().unwrap()).collect()
    }
}

fn downsample(frame: &[u8]) -> Vec<u8> {
    let (w, h) = (FW / 2, FH / 2);
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let s = ((y * 2) * FW + x * 2) * 4;
            let d = (y * w + x) * 4;
            out[d..d + 4].copy_from_slice(&frame[s..s + 4]);
        }
    }
    out
}

fn main() {
    for a in std::env::args().skip(1) {
        let known = ["seeds=", "players=", "gif=", "secs=", "stills="];
        if !known.iter().any(|k| a.starts_with(k)) {
            eprintln!("unknown argument {a}");
            std::process::exit(2);
        }
    }
    let seeds = seeds(&arg("seeds", "1..6"));
    let players: Vec<String> = arg("players", "nothing,planned,random").split(',').map(String::from).collect();
    let gif = arg("gif", "");
    let secs: u32 = arg("secs", &(LEVEL_TICKS / TICKS_PER_SECOND).to_string()).parse().unwrap();
    let stills: Vec<u32> = arg("stills", "15,30,45,60,90,120,180,239").split(',').map(|s| s.parse().unwrap()).collect();
    println!("seepprobe seeds={seeds:?} players={players:?} secs={secs} gif={gif:?}");
    let mut runs = Vec::new();
    let mut frame = vec![0u8; FW * FH * 4];
    for &seed in &seeds {
        for pname in &players {
            let mut player = Player::by_name(pname, seed).expect("player: nothing, planned, gasonly, nomould or random");
            let mut g = Game::new(seed);
            // `nomould` is `gasonly`'s plan in a mine with no mould at all:
            // Firedamp alone. `gasonly` keeps the mould but never Damps or
            // Cuts, so the two together split "the mould helps by itself"
            // from "the player's mould verbs help".
            g.mould_on = pname != "nomould";
            let mut gif_frames: Vec<Vec<u8>> = Vec::new();
            let (mut total, mut worst, mut worst_burn) = (0.0f64, 0.0f64, 0.0f64);
            let mut tubes_peak = 0;
            let mut first_blown = None;
            for t in 0..secs * TICKS_PER_SECOND {
                let s = Instant::now();
                let acts = player.acts(&g);
                g.step(&acts);
                draw::render(&g, &mut frame, None, Tool::Blow);
                let dt = s.elapsed().as_secs_f64() * 1000.0;
                total += dt;
                worst = worst.max(dt);
                if g.fire.iter().any(|&f| f > 0) && g.mould.live_tubes() > 0 {
                    worst_burn = worst_burn.max(dt);
                }
                tubes_peak = tubes_peak.max(g.mould.live_tubes());
                if first_blown.is_none() {
                    if let Some(l) = g.lamps.iter().filter_map(|l| l.blown_at).min() {
                        first_blown = Some(l as f32 / TICKS_PER_SECOND as f32);
                    }
                }
                if !gif.is_empty() {
                    if t % 20 == 0 {
                        gif_frames.push(downsample(&frame));
                    }
                    if t % TICKS_PER_SECOND == 0 && stills.contains(&(t / TICKS_PER_SECOND)) {
                        let path = format!("{gif}-{pname}-s{seed}-t{:03}.png", t / TICKS_PER_SECOND);
                        image::save_buffer(&path, &frame, FW as u32, FH as u32, image::ColorType::Rgba8).unwrap();
                    }
                }
                if g.over() && g.tick >= LEVEL_TICKS {
                    break;
                }
            }
            let split = |k: usize| {
                let v = &g.counts.uses[k];
                let early = v.iter().filter(|&&t| t < 60 * TICKS_PER_SECOND).count();
                (early, v.len() - early)
            };
            let mut ue = [0; 5];
            let mut ul = [0; 5];
            for k in 0..5 {
                let (e, l) = split(k);
                ue[k] = e;
                ul[k] = l;
            }
            let r = Run {
                player: pname.clone(),
                seed,
                carts: g.carts(),
                stars: g.stars(),
                blown: g.lamps.iter().filter(|l| l.state == LampState::Blown).count() as u32,
                first_blown_s: first_blown,
                face_s: g.face_seconds(),
                capped: g.cracks_capped(),
                open: g.cracks_open(),
                eaten: g.mould.counts.eaten,
                burnt: g.counts.burnt,
                vented: g.air.vented,
                emitted: g.counts.emitted,
                left: g.air.total(),
                buried: g.air.buried,
                singes: g.counts.singes,
                explosions: g.counts.explosions,
                uses_early: ue,
                uses_late: ul,
                tubes_peak,
                tubes_burnt: g.mould.counts.tubes_burnt,
                tubes_cut: g.mould.counts.tubes_cut,
                mean_ms: total / g.tick.max(1) as f64,
                worst_ms: worst,
                worst_burn_ms: worst_burn,
            };
            println!(
                "run player={} seed={} carts={} stars={} blown={} first_blown_s={} face_s={:.0} capped={}/{} eaten={:.1} burnt={:.1} vented={:.1} left={:.1} buried={:.1} emitted={:.1} singes={} booms={} uses_0-60s(B,S,D,C,F)={:?} uses_60s+={:?} tubes_peak={} tubes_burnt={} tubes_cut={} tick_mean_ms={:.2} tick_worst_ms={:.2} worst_during_burn_ms={:.2}",
                r.player,
                r.seed,
                r.carts,
                r.stars,
                r.blown,
                r.first_blown_s.map(|s| format!("{s:.0}")).unwrap_or("-".into()),
                r.face_s,
                r.capped,
                r.open,
                r.eaten,
                r.burnt,
                r.vented,
                r.left,
                r.buried,
                r.emitted,
                r.singes,
                r.explosions,
                r.uses_early,
                r.uses_late,
                r.tubes_peak,
                r.tubes_burnt,
                r.tubes_cut,
                r.mean_ms,
                r.worst_ms,
                r.worst_burn_ms
            );
            if !gif.is_empty() {
                let path = format!("{gif}-{pname}-s{seed}.gif");
                let file = std::fs::File::create(&path).unwrap();
                let mut enc = image::codecs::gif::GifEncoder::new_with_speed(file, 10);
                enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
                for f in gif_frames {
                    let img = image::RgbaImage::from_raw((FW / 2) as u32, (FH / 2) as u32, f).unwrap();
                    let fr = image::Frame::from_parts(img, 0, 0, image::Delay::from_numer_denom_ms(66, 1));
                    enc.encode_frame(fr).unwrap();
                }
                println!("wrote {path}");
            }
            runs.push(r);
        }
    }
    // Paired summary: per seed, planned against each other player.
    println!("\nsummary (paired by seed)");
    for p in &players {
        let rs: Vec<&Run> = runs.iter().filter(|r| &r.player == p).collect();
        let n = rs.len() as f64;
        let mean = |f: &dyn Fn(&Run) -> f64| rs.iter().map(|r| f(r)).sum::<f64>() / n;
        println!(
            "  {:8} n={} carts {:.1} stars {:.2} lamps_blown {:.2} face_s {:.0} eaten {:.1} burnt {:.1} vented {:.1} damp+cut_after_60s {:.1} tick_mean {:.2}ms worst {:.2}ms",
            p,
            rs.len(),
            mean(&|r| r.carts as f64),
            mean(&|r| r.stars as f64),
            mean(&|r| r.blown as f64),
            mean(&|r| r.face_s as f64),
            mean(&|r| r.eaten),
            mean(&|r| r.burnt),
            mean(&|r| r.vented),
            mean(&|r| (r.uses_late[Tool::Damp as usize] + r.uses_late[Tool::Cut as usize]) as f64),
            mean(&|r| r.mean_ms),
            rs.iter().map(|r| r.worst_ms).fold(0.0, f64::max)
        );
    }
    for a in &players {
        for b in &players {
            if a >= b {
                continue;
            }
            let (mut wa, mut wb, mut tie) = (0, 0, 0);
            for &s in &seeds {
                let ra = runs.iter().find(|r| &r.player == a && r.seed == s).unwrap();
                let rb = runs.iter().find(|r| &r.player == b && r.seed == s).unwrap();
                let (ka, kb) = ((ra.stars, ra.carts), (rb.stars, rb.carts));
                if ka > kb {
                    wa += 1;
                } else if kb > ka {
                    wb += 1;
                } else {
                    tie += 1;
                }
            }
            // Two-sided sign test on the untied pairs.
            let n = wa + wb;
            let k = wa.min(wb);
            let mut p = 0.0;
            for i in 0..=k {
                p += binom(n, i);
            }
            p = (2.0 * p / 2f64.powi(n as i32)).min(1.0);
            println!("  {a} vs {b}: {a} better on {wa}, {b} better on {wb}, tied {tie} (by stars then carts), sign-test p={p:.3}");
        }
    }
}

fn binom(n: u32, k: u32) -> f64 {
    let mut r = 1.0;
    for i in 0..k {
        r = r * (n - i) as f64 / (i + 1) as f64;
    }
    r
}
