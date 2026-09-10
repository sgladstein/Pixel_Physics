//! **A short animated GIF of the lab's mister, through the real renderer.**
//!
//! Neither of `CLAUDE.md`'s two named routes reaches the lab as it stands:
//! `filmstrip`'s `gif=1` only builds outdoor scenes, and `bin/lab.rs`'s own
//! headless hook (`PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES`) writes one still,
//! never a sequence. This is the minimal third thing -- `Lab::draw` (the
//! same call the interactive binary makes every frame, bar and all) into a
//! buffer, at an interval, assembled into a GIF the identical way `main.rs`'s
//! own `CaptureSequence::finish` does (`image::codecs::gif::GifEncoder`,
//! infinite repeat, a real per-frame delay) -- so the pixels in the card are
//! the pixels a player would see, not a bespoke render path of this file's
//! own that could quietly disagree with them.
//!
//! ```text
//! cargo run --release --example labgif -- rain=steady out=/tmp/steady.gif
//! cargo run --release --example labgif -- rain=off out=/tmp/off.gif        # the control
//! cargo run --release --example labgif -- rain=heavy frames=900 every=6 out=/tmp/heavy.gif
//! ```
//!
//! **`rain=` overrides the scenario's own saved rate** (`played_bed.ron`
//! ships `Off`, the shipped default) so this can render any rate on demand
//! without a throwaway scenario file for each one. **`start=`** advances the
//! box before capturing begins -- default 30,000, past the point the played
//! bed's own population has established (`Reports/...`: plant cells and
//! organism counts are already in the thousands by 30,000) but well before
//! the late-run population crash the unwatered measurement in `lab::rain`'s
//! own header shows -- so the card is judged on a representative bed, not an
//! empty one or a dying one.
//!
//! Prints the soil-water total and the mister's own effect count
//! (`World::rain_cells`) at the start and end of the captured window, the
//! two figures `CLAUDE.md`'s review-card convention asks for beside the
//! image -- an image says whether the rate *looks* like rain, and only the
//! counters say whether it *did* anything.

use pixel_physics::lab::rain::Rain;
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::update;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// The bed's whole soil-water total, `soil_drawdown.rs`'s own census over
/// `water_capacity`-bearing cells -- duplicated rather than imported since
/// that file's helper is private to it and this is three lines.
fn soil_total(lab: &Lab) -> u64 {
    let spec = &lab.spec;
    let w = &lab.world;
    let mut t = 0u64;
    for y in spec.ground_y..(spec.ground_y + spec.soil_depth) {
        for x in 0..spec.width {
            let c = w.get(x, y);
            if w.materials.get(c.material).water_capacity > 0 {
                t += update::soil_moisture(c) as u64;
            }
        }
    }
    t
}

// **A "cells of water still in the air" census was tried here and pulled,
// per `CLAUDE.md`'s own rule to check a fresh number against a case known to
// be quiet.** The `rain=off` control read **30** cells "in the air" over
// the identical window with the mister's own counter flat at 0 the whole
// time -- `weather::condense_under_a_lid` (`soil_drawdown.rs`'s own doc:
// the sealed box's real water cycle) drops condensation from the ceiling on
// its own, unrelated to this feature, so a census of the band could not
// tell the two sources apart. `World::rain_cells` is the number that stays
// specific to this mechanism -- it is incremented nowhere else -- so that
// is what this file reports instead.

fn main() {
    let scenario_name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let seed: u64 = arg("seed").unwrap_or(1);
    let rain_name: String = arg("rain").unwrap_or_else(|| "steady".to_string());
    let rain = match rain_name.to_ascii_lowercase().as_str() {
        "off" => Rain::Off,
        "light" => Rain::Light,
        "heavy" => Rain::Heavy,
        _ => Rain::Steady,
    };
    let start: u64 = arg("start").unwrap_or(30_000);
    let frames: u64 = arg("frames").unwrap_or(600);
    let every: u64 = arg("every").unwrap_or(5);
    let zoom: u32 = arg("zoom").unwrap_or(1).max(1);
    let out: String = arg("out").unwrap_or_else(|| "/tmp/labrain.gif".to_string());

    let mut sc = Scenario::load(&scenario_name).unwrap_or_else(|e| {
        eprintln!("scenario {scenario_name}: {e}");
        std::process::exit(1);
    });
    sc.bed.seed = seed;
    // **The warm-up runs at `Off`, whatever `rain=` asked for** -- the
    // requested rate is armed only once capturing starts, below. A card
    // built by pre-soaking the bed for `start` frames at the target rate
    // first shows a puddled bed getting marginally more puddled, which
    // answers a much weaker question than "does turning this on read as
    // rain arriving" -- and the file's own header claims the latter.
    sc.bed.rain = Rain::Off;
    let mut lab = Lab::new(sc.bed.clone());
    // The key-list overlay is on by default outside the real binary
    // (`Lab::new`'s `PIXEL_PHYSICS_LAB_HELP` gate) and paints over the whole
    // screen -- every other headless harness that calls `Lab::draw` turns it
    // off first, and this one must too or the card is a picture of the help
    // page rather than the box.
    lab.show_help = false;
    let msg = lab.load_scenario(sc);
    // **After `load_scenario`, not before**: `reset()` (which it calls)
    // replaces `self.stats` with a fresh `Stats::new()`, which opens
    // *showing* -- the biosphere overlay, covering most of the bed with
    // numbers -- so closing it earlier would be undone by the load.
    // `labstats.rs`'s own `page=ants` branch closes it for the identical
    // reason: a picture taken before this is a picture of the readout, not
    // of the rain.
    //
    // **The HUD keeps reading "PAUSED", and that is left alone rather than
    // forced.** `load_scenario` itself sets `Phase::Paused` (the moment a
    // player expects to look at what got placed before running it), and
    // this harness drives frames through `tick_for_harness` rather than the
    // dial -- forcing `Phase::Running` without also feeding `TimeControl`'s
    // `plan`/`record` loop left the corner readout printing a stale
    // "0 ticks per frame" instead, which is a worse lie than "paused". The
    // box is genuinely ticking; the label is a harness artifact either way,
    // and `context_md` on the card says so.
    if lab.stats.showing() {
        lab.stats.toggle();
    }
    println!(
        "labgif: scenario={scenario_name} seed={seed} rain={} start={start} frames={frames} every={every} zoom={zoom} out={out}",
        rain.label()
    );
    println!("  {msg}");

    for _ in 0..start {
        lab.tick_for_harness();
    }
    // The requested rate arms right here -- the captured window is the
    // player's own experience of pressing the RAIN key on an established
    // bed, not a bed that was already soaked in it.
    lab.spec.rain = rain;
    let water_before = soil_total(&lab);
    let rain_before = lab.world.rain_cells;
    println!("  at frame {start}: soil water {water_before}, rain cells so far {rain_before} (rain now armed at {})", rain.label());

    let (w, h) = (WIDTH, HEIGHT);
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    for f in 0..=frames {
        if f % every == 0 {
            let mut buf = vec![0u8; (w * h * 4) as usize];
            lab.draw(&mut buf, 60.0);
            let (zw, zh) = (w * zoom, h * zoom);
            let zoomed = if zoom == 1 {
                buf
            } else {
                let mut out_buf = vec![0u8; (zw * zh * 4) as usize];
                for y in 0..zh {
                    for x in 0..zw {
                        let src = (((y / zoom) * w + (x / zoom)) * 4) as usize;
                        let dst = ((y * zw + x) * 4) as usize;
                        out_buf[dst..dst + 4].copy_from_slice(&buf[src..src + 4]);
                    }
                }
                out_buf
            };
            if let Some(img) = image::RgbaImage::from_raw(zw, zh, zoomed) {
                shots.push(img);
            }
        }
        if f < frames {
            lab.tick_for_harness();
        }
    }

    let water_after = soil_total(&lab);
    let rain_after = lab.world.rain_cells;
    println!(
        "  at frame {}: soil water {water_after} ({:+}), rain cells so far {rain_after} ({:+})",
        start + frames,
        water_after as i64 - water_before as i64,
        rain_after - rain_before
    );

    // Real playback speed and a loop, `main.rs`'s own `CaptureSequence::
    // finish` convention exactly: 60 ticks/second is the shipped sim rate,
    // so `every` ticks between captures maps directly to real elapsed time.
    let delay_ms = ((every * 1000) / 60).max(1);
    let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(delay_ms));
    let gif_frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
    let n = gif_frames.len();
    match std::fs::File::create(&out) {
        Ok(file) => {
            let mut encoder = image::codecs::gif::GifEncoder::new(file);
            if let Err(e) = encoder.set_repeat(image::codecs::gif::Repeat::Infinite) {
                eprintln!("labgif: gif set_repeat failed: {e}");
            }
            if let Err(e) = encoder.encode_frames(gif_frames) {
                eprintln!("labgif: gif encode failed: {e}");
            }
            println!("  wrote {out} ({n} frames, {}x{} each)", w * zoom, h * zoom);
        }
        Err(e) => eprintln!("labgif: failed to create {out}: {e}"),
    }
}
