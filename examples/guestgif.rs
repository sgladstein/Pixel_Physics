//! **The guest seat's demo, played by a script, as a GIF and a counter
//! trace** (guest seat thread, 2026-10-25).
//!
//! `demo=guest` is a game of keys, and the owner reads chat on a phone, so
//! the demo is shown rather than described: this runs the real seat
//! (`Lab::start_demo_with(DemoSeat::Guest, ..)`, the seat's own landing,
//! respawn and HUD) with a scripted player pressing the same keys through
//! `Lab::demo_key`, and draws every `every`-th frame with `Lab::draw` -- the
//! call the window makes -- into a GIF, as `labgif` does.
//!
//! ```text
//! cargo run --release --example guestgif -- script=beggar out=/tmp/beggar.gif
//! cargo run --release --example guestgif -- script=raw out=/tmp/raw.gif      # the control: no rub, no soak
//! cargo run --release --example guestgif -- script=beggar frames=0 seed=2    # counters only
//! ```
//!
//! Scripts: `raw` walks to the door and down the shaft pressing nothing;
//! `beggar` does the same with soak on, rubs every 20 frames for the first
//! `rub_for` frames, then begs every 90; `predator` is `beggar` that also
//! presses B every 30 frames and E every 15 after `kill_at`.
//!
//! **Every second it prints a VISIT line** (gap, energy, cells, and the
//! ladder's counts), the counter beside the picture that `CLAUDE.md` asks
//! for: a GIF shows *what* the ants did; only the counts say how often.

use pixel_physics::lab::demo::{Demo, DemoKey, DemoSeat};
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::steer;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

fn main() {
    let script: String = arg("script").unwrap_or_else(|| "beggar".into());
    let seed: u64 = arg("seed").unwrap_or(1);
    let land: u64 = arg("land").unwrap_or(30_000);
    let gap: f32 = arg("gap").unwrap_or(1.5);
    let frames: u64 = arg("frames").unwrap_or(3_600);
    let every: u64 = arg("every").unwrap_or(6).max(1);
    let zoom: i32 = arg("zoom").unwrap_or(3);
    let rub_for: u64 = arg("rub_for").unwrap_or(1_200);
    let kill_at: u64 = arg("kill_at").unwrap_or(2_400);
    let out: String = arg("out").unwrap_or_else(|| "guest.gif".into());
    assert!(matches!(script.as_str(), "raw" | "beggar" | "predator" | "none"), "script= is raw, beggar, predator or none");
    println!("guestgif: script={script} seed={seed} land={land} gap={gap} frames={frames} every={every} zoom={zoom} rub_for={rub_for} kill_at={kill_at} out={out}");

    let mut sc = Scenario::load("demo_guest").expect("demo_guest ships");
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    let mut args = std::collections::BTreeMap::new();
    args.insert("land".to_string(), land.to_string());
    args.insert("gap".to_string(), gap.to_string());
    // `start_demo_with` without its by-name load, so `seed=` reaches the bed.
    let mut demo = Demo::new(DemoSeat::Guest);
    demo.args = args;
    lab.demo = Some(demo);
    println!("  {}", lab.load_scenario(sc));
    let t0 = std::time::Instant::now();
    while steer::steered_id(&lab.world).is_none() {
        lab.demo_frame();
        lab.tick_for_harness();
        assert!(lab.world.frame < land + 600, "no rove landed by frame {}", lab.world.frame);
    }
    println!("  warm-up to frame {} took {:.1} s wall", lab.world.frame, t0.elapsed().as_secs_f32());
    for _ in 1..zoom {
        lab.renderer.adjust_zoom(1);
    }
    let start = lab.world.frame;
    let rove0 = steer::steered_id(&lab.world).expect("landed");
    let nest = lab.world.nest_sites.iter().find(|n| n.seeded).cloned().expect("a nest");
    println!("  rove {rove0} landed at {:?}, nest door at ({}, {})", lab.world.organism(rove0).and_then(|s| s.chain.first().copied()), nest.x, nest.surface);
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    let mut held: Option<DemoKey> = None;
    for f in 0..=frames.max(1) {
        // The script: walk to the door, then down; press the seat's keys.
        if script != "none" {
            let want = steer::steered_id(&lab.world).and_then(|id| lab.world.organism(id)).and_then(|s| s.chain.first().copied()).map(|(hx, hy)| {
                if (hx - nest.x).abs() > 1 && hy <= nest.surface {
                    if hx < nest.x {
                        DemoKey::Right
                    } else {
                        DemoKey::Left
                    }
                } else {
                    DemoKey::Down
                }
            });
            if want != held {
                if let Some(k) = held {
                    lab.demo_key(k, false);
                }
                if let Some(k) = want {
                    lab.demo_key(k, true);
                }
                held = want;
            }
            if script != "raw" {
                if f == 0 {
                    lab.demo_key(DemoKey::Letter('P'), true);
                }
                if f < rub_for && f % 20 == 0 {
                    lab.demo_key(DemoKey::Letter('R'), true);
                }
                if f >= rub_for && f % 90 == 0 {
                    lab.demo_key(DemoKey::Letter('Y'), true);
                }
                if script == "predator" && f >= kill_at {
                    if f % 30 == 0 {
                        lab.demo_key(DemoKey::Letter('B'), true);
                    }
                    if f % 15 == 0 {
                        lab.demo_key(DemoKey::Letter('E'), true);
                    }
                }
            }
        }
        lab.demo_frame();
        if frames > 0 && f % every == 0 {
            let mut full = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
            lab.draw(&mut full, 60.0);
            if let Some(img) = image::RgbaImage::from_raw(WIDTH, HEIGHT, full) {
                shots.push(img);
            }
        }
        if f % 60 == 0 {
            let v = lab.world.visitors.as_ref().expect("the ladder is on");
            let c = v.counts;
            let body = steer::steered_id(&lab.world).and_then(|id| lab.world.organism(id).map(|s| (id, s.energy, s.chain.len())));
            let g = body.and_then(|(id, _, _)| v.list.iter().find(|x| x.id == id)).map_or(f32::NAN, |x| x.gap);
            println!(
                "VISIT f={} t={}s gap={g:.2} body={:?} rolls={} ign={} ant={} disp={} nip={} lost={} deaths={} rubs={} soak={} refused_soak={} begs={} fed={} fed_j={:.0} refused={} empty={} larvae={} ate_j={:.0}",
                lab.world.frame,
                f / 60,
                body.map(|(id, e, n)| (id, e as i32, n)),
                c.rolls,
                c.ignored,
                c.antennated,
                c.displayed,
                c.nipped,
                c.cells_lost,
                c.deaths,
                c.rubs,
                c.soak_frames,
                c.soak_refused,
                c.begs,
                c.fed,
                c.fed_j,
                c.refused,
                c.empty,
                c.larvae_killed,
                c.eaten_j
            );
        }
        if f < frames {
            lab.tick_for_harness();
        }
    }
    let c = lab.world.visitors.as_ref().expect("on").counts;
    println!(
        "RESULT script={script} seed={seed} gap={gap} frames={frames} from={start} rolls={} ign={} ant={} disp={} nip={} lost={} deaths={} fed={} fed_j={:.0} refused={} empty={} larvae={} ate_j={:.0} trip_deliveries={}",
        c.rolls,
        c.ignored,
        c.antennated,
        c.displayed,
        c.nipped,
        c.cells_lost,
        c.deaths,
        c.fed,
        c.fed_j,
        c.refused,
        c.empty,
        c.larvae_killed,
        c.eaten_j,
        lab.world.creature_stats.trip_deliveries
    );
    if shots.is_empty() {
        return;
    }
    let delay = image::Delay::from_numer_denom_ms((every as u32 * 1000) / 60, 1);
    let n = shots.len();
    let gif_frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
    let file = std::fs::File::create(&out).expect("out= is writable");
    let mut encoder = image::codecs::gif::GifEncoder::new(file);
    encoder.set_repeat(image::codecs::gif::Repeat::Infinite).expect("repeat");
    encoder.encode_frames(gif_frames).expect("gif");
    println!("  wrote {out}: {n} frames at {} ms", every * 1000 / 60);
}
