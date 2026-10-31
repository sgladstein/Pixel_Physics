//! **The lab, heard: a WAV and a video of one box with its sound.**
//!
//! Sound's stage 1 (`Reports/sound-design-2026-10-31.md`) is judged by ear,
//! and the owner listens on a phone, so the live device is not the instrument:
//! this is. It drives a lab box the way `labgif` does -- `tick_for_harness`
//! then `Lab::draw`, the call the real binary makes -- with the listener
//! switched on, so the commands it hears are the commands `bin/lab.rs` would
//! send to the speaker. It feeds them to the **same** `VoiceBank` the device
//! runs, rendering exactly one drawn frame's worth of samples per frame, so
//! picture and sound share one clock and cannot drift.
//!
//! ```text
//! cargo run --release --example soundtrack -- out=/tmp/snd
//! cargo run --release --example soundtrack -- out=/tmp/snd speeds=2:12,24:8,240:6 center=auto zoom=2
//! cargo run --release --example soundtrack -- out=/tmp/snd events=60:water:300,40-330,40;200:soil:120,30
//! ```
//!
//! Writes `out/soundtrack.wav`, `out/frames/NNNNN.png`, and -- when `ffmpeg`
//! is on the path -- `out/soundtrack.mp4` with the sound and
//! `out/silent.mp4` without it: **the paired page's two arms are the same
//! frames**, so the only difference a listener can hear is the sound.
//!
//! - `speeds=T:S,...` -- ticks per drawn frame for S seconds, in order. At
//!   `fps=30`, `2` is real time, `24` is time-lapse and `240` is the overview
//!   (`audio::listen::Mode`), so one run walks the time dial.
//! - `events=F:tool:x,y[-x2,y2];...` -- at drawn frame F, press the tool at
//!   logical pixel (x, y) and drag to (x2, y2). Tools: water, soil, food,
//!   dig, fire, fling. Goes through `Lab::press`/`drag`/`release`, the
//!   mouse's own route, so the tool sounds are the ones a player hears.
//! - `F:alert:W` in `events=` stages a line-ended alert of weight W (0..1)
//!   for a demo of the grading; it is printed as staged, never as heard.
//! - `center=auto|x,y zoom=N` -- a real camera (`labgif`'s): `auto` centres on
//!   the colony's mean position at the end of the warm-up.
//! - `style=hybrid|natural|musical` -- the design's three sketches.
//!
//! Prints the counts the page puts beside the video: what the listener heard
//! (ants, digs per second, alerts sent/held/dropped, verbs) and what the
//! voice bank played (grains per texture, bursts, alerts, steals). A video
//! says *what it sounds like*; only the counts say *whether it fired*.

use pixel_physics::audio::synth::VoiceBank;
use pixel_physics::audio::{wav, AlertKind, SoundCmd, Style};
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::ui::{Action, Tool};
use pixel_physics::lab::Lab;

const SAMPLE_RATE: u32 = 48_000;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().unwrap_or_else(|| panic!("{key}= does not parse"))))
}

struct Event {
    frame: u64,
    tool: Tool,
    from: (i32, i32),
    to: Option<(i32, i32)>,
    /// `F:alert:W` -- a line-ended alert of weight W pushed straight to the
    /// voice bank, **staged, not heard from the world**: a demo of how the
    /// alert grades with the share of the colony lost, for the listening
    /// page, since a real line ending big enough to sound is rare in a short
    /// clip. Printed as staged, and counted apart from heard alerts.
    alert: Option<f32>,
}

fn parse_xy(s: &str) -> (i32, i32) {
    let (x, y) = s.split_once(',').unwrap_or_else(|| panic!("{s:?} wants x,y"));
    (x.trim().parse().expect("x"), y.trim().parse().expect("y"))
}

fn parse_events(spec: &str) -> Vec<Event> {
    spec.split(';')
        .filter(|e| !e.trim().is_empty())
        .map(|e| {
            let bits: Vec<&str> = e.split(':').collect();
            assert_eq!(bits.len(), 3, "event {e:?} wants F:tool:x,y[-x2,y2]");
            if bits[1].eq_ignore_ascii_case("alert") {
                let w: f32 = bits[2].parse().expect("alert weight");
                return Event { frame: bits[0].parse().expect("event frame"), tool: Tool::Look, from: (0, 0), to: None, alert: Some(w) };
            }
            let tool = match bits[1].to_ascii_lowercase().as_str() {
                "water" => Tool::Water,
                "soil" => Tool::Soil,
                "food" => Tool::Food,
                "dig" => Tool::Dig,
                "fire" => Tool::Fire,
                "fling" => Tool::Fling,
                other => panic!("unknown tool {other:?}; water, soil, food, dig, fire, fling"),
            };
            let (from, to) = match bits[2].split_once('-') {
                Some((a, b)) => (parse_xy(a), Some(parse_xy(b))),
                None => (parse_xy(bits[2]), None),
            };
            Event { frame: bits[0].parse().expect("event frame"), tool, from, to, alert: None }
        })
        .collect()
}

/// The colony's mean head position, for `center=auto`.
fn colony_centre(lab: &Lab) -> Option<(i32, i32)> {
    let w = &lab.world;
    let sid = w.species.id_of(&lab.spec.colony_species)?;
    let (mut sx, mut sy, mut n) = (0i64, 0i64, 0i64);
    for id in w.live_organism_ids() {
        if let Some(s) = w.organism(id) {
            if s.species == sid {
                if let Some(&(x, y)) = s.chain.first() {
                    sx += x as i64;
                    sy += y as i64;
                    n += 1;
                }
            }
        }
    }
    (n > 0).then(|| ((sx / n) as i32, (sy / n) as i32))
}

fn main() {
    let scenario_name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let seed: u64 = arg("seed").unwrap_or(1);
    let start: u64 = arg("start").unwrap_or(30_000);
    let fps: u32 = arg("fps").unwrap_or(30);
    let speeds: String = arg("speeds").unwrap_or_else(|| "2:20".to_string());
    let events = parse_events(&arg::<String>("events").unwrap_or_default());
    let style_name: String = arg("style").unwrap_or_else(|| "hybrid".to_string());
    let style = Style::parse(&style_name).unwrap_or_else(|| panic!("style={style_name}: hybrid, natural or musical"));
    let zoom: u32 = arg("zoom").unwrap_or(1).max(1);
    let center: Option<String> = arg("center");
    let out: String = arg("out").unwrap_or_else(|| "soundtrack_out".to_string());
    let no_frames = std::env::args().any(|a| a == "no_frames");

    let plan: Vec<(u64, f32)> = speeds
        .split(',')
        .map(|p| {
            let (t, s) = p.split_once(':').unwrap_or_else(|| panic!("speeds entry {p:?} wants T:S"));
            (t.parse().expect("ticks per frame"), s.parse().expect("seconds"))
        })
        .collect();

    let out_dir = std::path::PathBuf::from(&out);
    let frame_dir = out_dir.join("frames");
    std::fs::create_dir_all(&frame_dir).expect("out dir");

    let mut sc = Scenario::load(&scenario_name).unwrap_or_else(|e| {
        eprintln!("scenario {scenario_name}: {e}");
        std::process::exit(1);
    });
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    lab.show_help = false;
    let msg = lab.load_scenario(sc);
    if lab.stats.showing() {
        lab.stats.toggle();
    }
    println!(
        "soundtrack: scenario={scenario_name} seed={seed} start={start} fps={fps} speeds={speeds} style={} zoom={zoom} center={} events={} out={out}",
        style.label(),
        center.as_deref().unwrap_or("none"),
        events.len()
    );
    println!("  {msg}");

    for _ in 0..start {
        lab.tick_for_harness();
    }

    let (full_w, full_h) = lab.viewport();
    if let Some(c) = &center {
        for _ in 1..zoom {
            lab.renderer.adjust_zoom(1);
        }
        let at = if c == "auto" { colony_centre(&lab).unwrap_or((lab.spec.width / 2, lab.spec.ground_y)) } else { parse_xy(c) };
        let bounds = pixel_physics::sim::chunk::Rect::new(0, 0, lab.spec.width - 1, lab.spec.height - 1);
        let (span_x, span_y) = lab.renderer.visible_span((full_w, full_h));
        lab.renderer.set_camera(at.0 - span_x / 2, at.1 - span_y / 2, (full_w, full_h), Some(bounds));
        println!("  camera centred on ({},{}) at {zoom}x -- {span_x}x{span_y} world cells visible", at.0, at.1);
    }

    // The listener is on from here: `enable_sound` is what `bin/lab.rs` calls
    // when the device opens. The voice bank is the device's, run offline.
    lab.enable_sound();
    let mut bank = VoiceBank::new(SAMPLE_RATE, seed ^ 0x50_0D);
    bank.apply(SoundCmd::Style(style));

    let per_frame = (SAMPLE_RATE / fps) as usize;
    let mut samples: Vec<f32> = Vec::new();
    let mut block = vec![0.0f32; per_frame * 2];
    let mut buf = vec![0u8; (full_w * full_h * 4) as usize];
    let mut frame: u64 = 0;
    let mut ants_min = u32::MAX;
    let mut ants_max = 0u32;
    let mut lines = Vec::new();
    let mut staged = 0u32;
    let mut frame_costs: Vec<(f64, u64)> = Vec::new();
    for &(tpf, secs) in &plan {
        let n = (secs * fps as f32).round() as u64;
        let first = frame;
        for _ in 0..n {
            for e in events.iter().filter(|e| e.frame == frame) {
                if let Some(weight) = e.alert {
                    lab.sound.push(SoundCmd::Alert { kind: AlertKind::LineEnded, weight, pan: 0.0 });
                    staged += 1;
                    continue;
                }
                lab.act(Action::Tool(e.tool));
                lab.set_cursor(Some(e.from));
                lab.press(e.from.0, e.from.1);
                if let Some(to) = e.to {
                    // A drag in eight steps, the way a hand moves, so the
                    // stroke is heard as a stroke and not as one blob.
                    for k in 1..=8 {
                        let x = e.from.0 + (to.0 - e.from.0) * k / 8;
                        let y = e.from.1 + (to.1 - e.from.1) * k / 8;
                        lab.drag(x, y);
                    }
                }
                let end = e.to.unwrap_or(e.from);
                lab.release(end.0, end.1);
                lab.set_cursor(None);
                lab.act(Action::Tool(Tool::Look));
            }
            for _ in 0..tpf {
                lab.tick_for_harness();
            }
            let before = lab.sound.stats.observe_secs;
            lab.draw(&mut buf, fps as f32);
            frame_costs.push((lab.sound.stats.observe_secs - before, frame));
            for cmd in lab.sound.drain() {
                bank.apply(cmd);
            }
            bank.render(&mut block, 2);
            samples.extend_from_slice(&block);
            ants_min = ants_min.min(lab.sound.stats.ants);
            ants_max = ants_max.max(lab.sound.stats.ants);
            if !no_frames {
                let path = frame_dir.join(format!("{frame:05}.png"));
                image::save_buffer(&path, &buf, full_w, full_h, image::ColorType::Rgba8).expect("png");
            }
            frame += 1;
        }
        lines.push(format!(
            "  segment {tpf} ticks/frame x {secs}s (frames {first}-{}): mode {:?}, ants heard {}, digs/s {:.1}",
            frame.saturating_sub(1),
            lab.sound.stats.mode,
            lab.sound.stats.ants,
            lab.sound.stats.digs_per_sec
        ));
    }
    for l in &lines {
        println!("{l}");
    }

    let wav_path = out_dir.join("soundtrack.wav");
    wav::write(&wav_path, &samples, 2, SAMPLE_RATE).expect("wav");
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt();
    let ls = &lab.sound.stats;
    let ss = &bank.stats;
    println!("  frames {frame}, audio {:.1}s, peak {peak:.3}, rms {rms:.4}", samples.len() as f32 / 2.0 / SAMPLE_RATE as f32);
    println!(
        "  HEARD: ants {ants_min}..{ants_max}, alerts sent {} held {} too light {}, verbs {}, rebaselines {}, observes {}",
        ls.alerts_sent, ls.alerts_held, ls.alerts_too_light, ls.verbs, ls.rebaselines, ls.observes
    );
    if staged > 0 {
        println!("  STAGED: {staged} alerts pushed by events=, not heard from the world");
    }
    println!(
        "  PLAYED: grains {} (walk {} dig {} carry {}), verb grains {}, bursts {}, alerts {}, steals {}",
        ss.grains, ss.grains_by_texture[0], ss.grains_by_texture[1], ss.grains_by_texture[2], ss.verb_grains, ss.bursts, ss.alerts, ss.steals
    );
    println!(
        "  COST: listener {:.4} ms a drawn frame on average, worst {:.4} ms (budget 0.25)",
        ls.observe_secs * 1e3 / ls.observes.max(1) as f64,
        ls.observe_worst * 1e3
    );
    // The worst frame alone is a poor summary: the first call grows the
    // listener's maps, and a page fault or a scheduler hiccup lands on
    // whichever frame it lands on. The order statistics say which it is.
    frame_costs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let at = |q: f64| frame_costs[((frame_costs.len() - 1) as f64 * q) as usize].0 * 1e3;
    if let Some(&(worst, wf)) = frame_costs.last() {
        println!("  COST by frame: median {:.4} ms, p99 {:.4} ms, worst {:.4} ms at frame {wf}", at(0.5), at(0.99), worst * 1e3);
    }
    println!("  wrote {}", wav_path.display());

    if no_frames {
        return;
    }
    let ffmpeg = |args: &[&str]| std::process::Command::new("ffmpeg").args(args).status().map(|s| s.success()).unwrap_or(false);
    let pattern = frame_dir.join("%05d.png");
    let pattern = pattern.to_string_lossy();
    let rate = fps.to_string();
    let mp4 = out_dir.join("soundtrack.mp4");
    let silent = out_dir.join("silent.mp4");
    let scale = "scale=trunc(iw*2/2)*2:trunc(ih*2/2)*2:flags=neighbor";
    let ok = ffmpeg(&[
        "-y", "-loglevel", "error", "-framerate", &rate, "-i", &pattern, "-i", &wav_path.to_string_lossy(), "-vf", scale, "-c:v", "libx264",
        "-pix_fmt", "yuv420p", "-crf", "20", "-c:a", "aac", "-b:a", "160k", "-shortest", &mp4.to_string_lossy(),
    ]) && ffmpeg(&["-y", "-loglevel", "error", "-i", &mp4.to_string_lossy(), "-c:v", "copy", "-an", &silent.to_string_lossy()]);
    if ok {
        println!("  wrote {} and {}", mp4.display(), silent.display());
    } else {
        println!("  (no ffmpeg, or it failed: the WAV and the frames are the result)");
    }
}
