//! **What a frame of the shipped 8192x2560 world costs, and whether a speed-up
//! changed anything** -- the big-world twin of `perfbench` (which is the lab
//! box) and of `frame_profile` (which times the same world but prints no
//! digest, so it cannot say whether two builds ran the same simulation).
//!
//! Built 2026-10-30 for the big-world speed lane, from the repo review's
//! finding that frames at 8192x2560 average ~26 ms and 9 in 10 miss 16.6 ms.
//!
//! One scene per run, `App::new()`'s world ticked exactly as `App::update`
//! ticks it (`frame::step`) and drawn exactly as `main.rs` draws it
//! (`App::draw`), then:
//!
//! - **whole-frame milliseconds**, split into the tick and the draw, as mean,
//!   p50, p99 and the share of frames over 16.6 ms;
//! - **`RESULT` lines that must match across a speed-up**: the grid, the
//!   field and the organisms (`common/hashes.rs`, the digests every perf
//!   harness here prints), plus a digest of every drawn frame buffer, so a
//!   render change is held to the same bar as a simulation change.
//!
//! **Run it from the repository root.** Some assets are read from disk
//! relative to the working directory, and a run from elsewhere builds a
//! different (equally deterministic) world -- whose digests then match
//! nothing, which looks exactly like a speed-up that changed the
//! simulation.
//!
//! ```text
//! cargo run --release --example bigworld -- scene=settled frames=300
//! cargo run --release --example bigworld -- scene=walk frames=300
//! RAYON_NUM_THREADS=1 valgrind --tool=callgrind --instr-atstart=no \
//!     target/release/examples/bigworld scene=settled frames=60 cg=1
//! ```
//!
//! Scenes: `settled` (no input; the default seed rains, which forces a full
//! redraw on its own), `walk` (the gnome walking right, so the camera pans and
//! every pixel is invalidated), `dry` (the first seed with no rain in the run:
//! the dirty-rect path actually in use), `dig` (a blast every 60 frames).
//!
//! `cg=1` switches callgrind instrumentation on for the measured window only;
//! natively the client requests are no-ops. Echoes its own parameters; an
//! unknown argument is refused rather than ignored.

use pixel_physics::app::{App, HEIGHT, WIDTH};
use pixel_physics::sim::frame;

#[path = "common/hashes.rs"]
mod hashes;

#[cfg(target_arch = "x86_64")]
fn callgrind(request: u64, arg1: u64) {
    // `needscost`'s inline client request; see its doc for the sequence.
    let args: [u64; 6] = [request, arg1, 0, 0, 0, 0];
    let mut result: u64 = 0;
    // SAFETY: the rotations sum to a whole turn and the exchange is with
    // itself, so every register is left as found; reads `args` through `rax`.
    unsafe {
        std::arch::asm!(
            "rol rdi, 3",
            "rol rdi, 13",
            "rol rdi, 61",
            "rol rdi, 51",
            "xchg rbx, rbx",
            inout("rdx") result,
            in("rax") args.as_ptr(),
            options(nostack),
        );
    }
    let _ = result;
}

#[cfg(not(target_arch = "x86_64"))]
fn callgrind(_request: u64, _arg1: u64) {}

const CG_ZERO_STATS: u64 = 0x4354_0001;
const CG_DUMP_STATS_AT: u64 = 0x4354_0003;
const CG_START_INSTRUMENTATION: u64 = 0x4354_0004;
const CG_STOP_INSTRUMENTATION: u64 = 0x4354_0005;

fn pct(v: &[f64], p: f64) -> f64 {
    v[((v.len() as f64 - 1.0) * p).round() as usize]
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn main() {
    let mut scene = "settled".to_string();
    let mut frames = 300usize;
    let mut warmup = 60usize;
    let mut cg = false;
    let mut draw = true;
    for arg in std::env::args().skip(1) {
        let (k, v) = arg.split_once('=').unwrap_or_else(|| panic!("bigworld: expected key=value, got {arg}"));
        match k {
            "scene" => scene = v.to_string(),
            "frames" => frames = v.parse().expect("frames=N"),
            "warmup" => warmup = v.parse().expect("warmup=N"),
            "cg" => cg = v == "1",
            "draw" => draw = v != "0",
            _ => panic!("bigworld: unknown argument {k}"),
        }
    }
    let threads = std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "default".into());
    println!("bigworld: scene={scene} frames={frames} warmup={warmup} draw={} cg={} RAYON_NUM_THREADS={threads}", u8::from(draw), u8::from(cg));

    let mut app = App::new();
    match scene.as_str() {
        "settled" | "dig" => {}
        "walk" => {
            app.summon_player(WIDTH as i32 / 2, HEIGHT as i32 / 2);
        }
        "dry" => {
            for candidate in 0..400u64 {
                app.worldgen_seed = candidate;
                app.reset();
                if (0..(warmup + frames + 100) as u64).all(|f| !pixel_physics::sim::weather::at(app.world.seed, f).is_precipitating()) {
                    break;
                }
            }
        }
        other => panic!("bigworld: unknown scene {other}"),
    }
    let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let (mut tick_ms, mut draw_ms) = (Vec::new(), Vec::new());
    // Every drawn frame folded in, not just the last: a render change that
    // differs on one frame mid-run and converges again must still show.
    let mut frames_digest: u64 = 0xcbf2_9ce4_8422_2325;
    for i in 0..warmup + frames {
        let measured = i >= warmup;
        if measured && i == warmup && cg {
            callgrind(CG_ZERO_STATS, 0);
            callgrind(CG_START_INSTRUMENTATION, 0);
        }
        match scene.as_str() {
            "walk" => app.player_input.right = true,
            "dig" if i % 60 == 0 => app.explode(WIDTH as i32 / 2, HEIGHT as i32 * 3 / 4),
            _ => {}
        }
        let t0 = std::time::Instant::now();
        frame::step(&mut app.world, &mut app.particles, &mut app.blasts, app.player_input, &app.player_tuning);
        app.player_input.jump_pressed = false;
        let t1 = std::time::Instant::now();
        if draw {
            app.draw(&mut buf, None);
        }
        let t2 = std::time::Instant::now();
        if draw {
            frames_digest = (frames_digest ^ fnv(&buf)).wrapping_mul(0x0000_0100_0000_01b3);
        }
        if measured {
            tick_ms.push((t1 - t0).as_secs_f64() * 1000.0);
            draw_ms.push((t2 - t1).as_secs_f64() * 1000.0);
        }
    }
    if cg {
        callgrind(CG_DUMP_STATS_AT, c"bigworld".as_ptr() as u64);
        callgrind(CG_STOP_INSTRUMENTATION, 0);
    }
    let mut total: Vec<f64> = tick_ms.iter().zip(&draw_ms).map(|(a, b)| a + b).collect();
    for (name, v) in [("tick", &mut tick_ms), ("draw", &mut draw_ms), ("FRAME", &mut total)] {
        let mean = v.iter().sum::<f64>() / v.len() as f64;
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!("{name:>6}  mean {mean:7.3} ms  p50 {:7.3}  p99 {:7.3}  max {:7.3}", pct(v, 0.5), pct(v, 0.99), v[v.len() - 1]);
    }
    let over = total.iter().filter(|&&t| t > 16.6).count();
    println!("  {over}/{} frames over 16.6 ms ({:.0}%)", total.len(), over as f64 * 100.0 / total.len() as f64);
    println!("RESULT {} frames {:016x} at {}", hashes::line(&app.world), frames_digest, app.world.frame);
}
