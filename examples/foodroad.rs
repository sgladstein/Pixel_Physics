//! **The food road and the harvest map, rendered and counted.**
//!
//! The instrument for the two channels `src/food_road.rs` adds: *where does a
//! colony's food come from*, and *how does it travel*. `F7` cycles them in the
//! lab; this renders them headlessly, and — the half a picture cannot do —
//! prints the numbers underneath.
//!
//! ```text
//! # the positive control: a colony at one wall, a heap at the other
//! cargo run --release --example foodroad -- scenario=far_larder start=12000 \
//!     frames=3600 every=120 mode=both out=/tmp/road.gif
//!
//! # the distribution, for setting the two ramp scales from data
//! cargo run --release --example foodroad -- scenario=far_larder start=12000 frames=0 probe=1
//!
//! # what the channel costs, paired and alternating, on a settled bed
//! cargo run --release --example foodroad -- scenario=far_larder start=12000 cost=8
//! cargo run --release --example foodroad -- scenario=far_larder start=12000 cost=8 settled=1
//! ```
//!
//! # Why `far_larder` is the default
//!
//! **Both channels need a positive control, and this scenario is one.**
//! `CLAUDE.md`: *construct the case whose answer you know is non-zero and
//! check the instrument reports it* — six arithmetically correct numbers
//! about the wrong thing were caught by nothing else. `far_larder.ron` puts
//! the colony at `x: 107` and its only food at `x: 470`, 363 cells clear. So
//! the harvest map has one answer it must give (tiles at the east wall, and
//! nowhere near the nest) and the road has one shape it must draw (a line
//! between the two). A map that lights the nest, or lights evenly, is the
//! instrument failing — and it would look exactly like a result.
//!
//! `scenario=played_bed` is the other bed worth reading: food growing all
//! over a planted box rather than heaped at one coordinate, which is the
//! picture the owner actually plays.
//!
//! # What it prints, and why each line is there
//!
//! - **`FIRED`** — laden steps, unladen steps and harvest bites over the
//!   captured window. `CLAUDE.md`'s standing rule: a collapse once read as
//!   "chunks are working" off a picture whose body count was zero for the
//!   whole run. A road drawn from zero laden steps is ants walking, not a
//!   food economy, and the two look identical at play zoom.
//! - **`SAMPLED`** — ticks observed against ticks run. The lab observes per
//!   tick; anything that only draws observes per *frame*, and a road built
//!   from one sample in two hundred is a smear. The number says which you
//!   are looking at.
//! - **`ROAD`/`HARVEST` distributions** — max, p50 and p90 of the live
//!   channel against the ramp scale it is drawn on. This is the half an
//!   overlay cannot answer: a corrected overlay was still misread as
//!   "everything at the floor" when the real value was 40% of scale.
//! - **`SOURCES`** — the heaviest harvest tiles in world coordinates, per
//!   colony. On `far_larder` these are the answer key.

use pixel_physics::food_road::FoodOverlay;
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// `x,y,w,h`, the same spelling `labgif` and `filmstrip` use.
fn crop_arg() -> Option<(u32, u32, u32, u32)> {
    let v: String = arg("crop")?;
    let bits: Vec<u32> = v.split(',').map(|b| b.trim().parse().expect("crop=x,y,w,h")).collect();
    assert_eq!(bits.len(), 4, "crop=x,y,w,h");
    Some((bits[0], bits[1], bits[2], bits[3]))
}

fn mode_arg() -> FoodOverlay {
    match arg::<String>("mode").unwrap_or_else(|| "both".into()).as_str() {
        "off" => FoodOverlay::Off,
        "road" => FoodOverlay::Road,
        "harvest" => FoodOverlay::Harvest,
        "both" => FoodOverlay::Both,
        other => panic!("mode={other}: want off|road|harvest|both"),
    }
}

/// Order statistics of a sample, as a share of `full`. Returns
/// `(n, max, p50, p90)`.
fn spread(mut v: Vec<f32>, full: f32) -> (usize, f32, f32, f32) {
    if v.is_empty() {
        return (0, 0.0, 0.0, 0.0);
    }
    v.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in a decayed weight"));
    let at = |p: f64| v[((v.len() as f64 - 1.0) * p) as usize] / full;
    (v.len(), v[v.len() - 1] / full, at(0.5), at(0.9))
}

/// The live state of both channels, in one line each. **Paired with the
/// overlay deliberately** — `CLAUDE.md` records a debug channel misread as
/// "everything at the ramp floor" when the true value was 40% of scale, on a
/// one-cell-wide twig. An overlay says what and where; only this says how
/// much.
fn probe(lab: &Lab, at: u64) {
    let food = &lab.renderer.food;
    let now = lab.world.frame;
    let road = food.road_readout(now);
    let road_scale = food.road_scale();
    let (walked_n, walked_max, walked_p50, walked_p90) = spread(road.iter().map(|r| r.2).filter(|v| *v > 0.0).collect(), road_scale);
    let (laden_n, laden_max, laden_p50, laden_p90) = spread(road.iter().map(|r| r.3).filter(|v| *v > 0.0).collect(), road_scale);
    let harvest = food.harvest_readout(now);
    let harvest_scale = food.harvest_scale();
    let (h_n, h_max, h_p50, h_p90) = spread(harvest.iter().map(|h| h.3).collect(), harvest_scale);
    // **Shares of the ramp scale, with the scale itself printed beside
    // them.** A share alone cannot say whether the ramp is aimed at the data
    // — which is the whole risk of a tracked scale — and a raw weight alone
    // cannot say what the picture will look like. Both, or neither is
    // checkable.
    println!(
        "  PROBE frame {at}: road cells {} (ramp full at {road_scale:.2}) | laden {laden_n} cells, max {laden_max:.2} p90 {laden_p90:.2} p50 {laden_p50:.2} of full | \
         walked {walked_n} cells, max {walked_max:.2} p90 {walked_p90:.2} p50 {walked_p50:.2} of full | \
         harvest {h_n} tiles (ramp full at {harvest_scale:.0}), max {h_max:.2} p90 {h_p90:.2} p50 {h_p50:.2} of full",
        road.len()
    );
}

/// How many animals are alive, and how many of them are carrying something.
///
/// **The second number is the one that matters**, and it is not derivable
/// from the first: a colony of six hundred with nothing in a crop draws no
/// road at all, and that is a finding about the colony rather than about
/// this instrument.
fn laden_now(lab: &Lab) -> (usize, usize) {
    let w = &lab.world;
    let mut alive = 0;
    let mut carrying = 0;
    for id in w.live_organism_ids() {
        let Some(st) = w.organism(id) else { continue };
        if w.species.get(st.species).creature.is_none() {
            continue;
        }
        alive += 1;
        if st.crop.is_some_and(|c| c.cells > 0) {
            carrying += 1;
        }
    }
    (alive, carrying)
}

fn main() {
    let scenario_name: String = arg("scenario").unwrap_or_else(|| "far_larder".into());
    let seed: u64 = arg("seed").unwrap_or(1);
    let start: u64 = arg("start").unwrap_or(12_000);
    let frames: u64 = arg("frames").unwrap_or(0);
    let every: u64 = arg("every").unwrap_or(120);
    let out: String = arg("out").unwrap_or_else(|| "/tmp/foodroad.gif".into());
    let png_dir: Option<String> = arg("png_dir");
    let cost: u32 = arg("cost").unwrap_or(0);
    let up: u32 = arg("up").unwrap_or(1);
    let mode = mode_arg();
    let crop = crop_arg();

    let mut sc = Scenario::load(&scenario_name).unwrap_or_else(|e| {
        eprintln!("scenario {scenario_name}: {e}");
        std::process::exit(1);
    });
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    // The key-list overlay is on by default outside the real binary and
    // paints over the whole screen; every headless harness that calls
    // `Lab::draw` turns it off or photographs the help page instead.
    lab.show_help = false;
    let msg = lab.load_scenario(sc);
    if lab.stats.showing() {
        lab.stats.toggle();
    }

    // **The channel is armed before the warm-up, not after.** The harvest
    // map is an accumulation with a half-life of a minute of play; armed at
    // the moment capturing starts it would draw an empty world for the first
    // thousand frames of the card, which reads as a dead mechanism.
    lab.renderer.food.mode = mode;
    if let Some(v) = arg::<i32>("tile") {
        lab.renderer.food.tile = v;
    }
    // `walked=0` draws the laden road alone -- the arm of the blind A/B that
    // asks whether the empty-handed traffic is context or clutter.
    if let Some(v) = arg::<u32>("walked") {
        lab.renderer.food.show_walked = v == 1;
    }
    // `ground=0` puts the round-35 drawing back: the harvest wash covering
    // its whole tile, air included. The arm of the comparison that asks
    // whether clipping to ground is what fixed the box.
    if let Some(v) = arg::<u32>("ground") {
        lab.renderer.food.harvest_on_ground = v == 1;
    }
    if let Some(v) = arg::<f32>("roadhalf") {
        lab.renderer.food.road_half_life = v;
    }
    if let Some(v) = arg::<f32>("harvesthalf") {
        lab.renderer.food.harvest_half_life = v;
    }
    // **Pinning a bar is opt-in, and off by default.** Both ramps track the
    // bed (`FoodRoad::refresh`); these fix them, which is what two sheets
    // that must be read against each other need.
    if let Some(v) = arg::<f32>("roadfull") {
        lab.renderer.food.road_full = Some(v);
    }
    if let Some(v) = arg::<f32>("harvestfull") {
        lab.renderer.food.harvest_full = Some(v);
    }
    // **Echoed unconditionally, every one of them.** `CLAUDE.md`: a 3.5-hour
    // study produced eight byte-identical logs because `worldseed=` had been
    // added to the harness after the binary was built, and a knob nobody can
    // see the value of is a knob nobody can tell is disconnected.
    let f = &lab.renderer.food;
    println!(
        "foodroad: scenario={scenario_name} seed={seed} start={start} frames={frames} every={every} mode={} \
         tile={} show_walked={} harvest_on_ground={} road_half_life={} harvest_half_life={} road_full={} harvest_full={} out={out}",
        f.mode.label(),
        f.tile,
        f.show_walked,
        f.harvest_on_ground,
        f.road_half_life,
        f.harvest_half_life,
        f.road_full.map_or_else(|| "tracked".into(), |v| v.to_string()),
        f.harvest_full.map_or_else(|| "tracked".into(), |v| v.to_string())
    );
    println!("  {msg}");

    for _ in 0..start {
        lab.tick_for_harness();
    }

    if cost > 0 {
        price(&mut lab, cost, mode, arg::<u32>("settled").unwrap_or(0) == 1, arg::<u32>("whole").unwrap_or(0) == 1);
        return;
    }

    // **Counters read at the capture boundary and reported as a delta**, so
    // the number under the card describes the window the card shows. A total
    // over a 12,000-frame warm-up plus a 3,600-frame capture is a real number
    // about neither.
    let base = (lab.renderer.food.laden_steps, lab.renderer.food.walked_steps, lab.renderer.food.harvest_bites, lab.renderer.food.harvest_value, lab.renderer.food.observed_frames);

    if let Some(dir) = &png_dir {
        std::fs::create_dir_all(dir).unwrap_or_else(|e| panic!("foodroad: png_dir {dir}: {e}"));
    }
    let (full_w, full_h) = (WIDTH, HEIGHT);
    let (cx, cy, w, h) = crop.unwrap_or((0, 0, full_w, full_h));
    let mut shots: Vec<image::RgbaImage> = Vec::new();

    for step in 0..=frames {
        if step % every == 0 {
            let mut buf = vec![0u8; (full_w * full_h * 4) as usize];
            lab.draw(&mut buf, 60.0);
            probe(&lab, lab.world.frame);
            let cropped = if crop.is_none() {
                buf
            } else {
                let mut c = vec![0u8; (w * h * 4) as usize];
                for row in 0..h {
                    let src = (((cy + row) * full_w + cx) * 4) as usize;
                    let dst = (row * w * 4) as usize;
                    c[dst..dst + (w * 4) as usize].copy_from_slice(&buf[src..src + (w * 4) as usize]);
                }
                c
            };
            if let Some(img) = image::RgbaImage::from_raw(w, h, cropped) {
                if let Some(dir) = &png_dir {
                    let path = std::path::Path::new(dir).join(format!("frame_{:04}.png", shots.len()));
                    // **`up=` scales the PNG on disk and never the GIF**, the
                    // same split `labgif` makes: the review page scales a GIF
                    // client-side under `image-rendering: pixelated` and a
                    // zoomed one is only bytes, while a still the owner is
                    // asked to judge a one-cell-wide road in has to arrive
                    // legible -- the stills he has been able to judge are
                    // 700-950 px across.
                    let saved = if up == 1 {
                        img.save(&path)
                    } else {
                        image::imageops::resize(&img, img.width() * up, img.height() * up, image::imageops::FilterType::Nearest).save(&path)
                    };
                    if let Err(e) = saved {
                        eprintln!("foodroad: {}: {e}", path.display());
                    }
                }
                shots.push(img);
            }
        }
        if step < frames {
            lab.tick_for_harness();
        }
    }

    report(&lab, base, start, frames);

    if frames > 0 && !shots.is_empty() && out.ends_with(".gif") {
        let delay_ms = arg::<u64>("delay").unwrap_or((every * 1000) / 60).max(1);
        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(delay_ms));
        let (sw, sh) = (shots[0].width(), shots[0].height());
        let n = shots.len();
        let gif: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
        match std::fs::File::create(&out) {
            Ok(file) => {
                let mut enc = image::codecs::gif::GifEncoder::new(file);
                if let Err(e) = enc.set_repeat(image::codecs::gif::Repeat::Infinite) {
                    eprintln!("foodroad: set_repeat: {e}");
                }
                if let Err(e) = enc.encode_frames(gif) {
                    eprintln!("foodroad: encode: {e}");
                }
                println!("  wrote {out} ({n} frames, {sw}x{sh}, {delay_ms} ms/frame)");
            }
            Err(e) => eprintln!("foodroad: create {out}: {e}"),
        }
    }
    if let Some(dir) = &png_dir {
        println!("  wrote frames to {dir}");
    }
}

/// The counters and the sources, at the end of a captured window.
fn report(lab: &Lab, base: (u64, u64, u64, f64, u64), start: u64, frames: u64) {
    let f = &lab.renderer.food;
    let (alive, carrying) = laden_now(lab);
    // **"Did it fire at all" needs a counter, not a picture.**
    println!(
        "  FIRED over the captured {frames} frames: laden steps {} | unladen steps {} | harvest bites {} | face value taken {:.0}",
        f.laden_steps - base.0,
        f.walked_steps - base.1,
        f.harvest_bites - base.2,
        f.harvest_value - base.3
    );
    // **How sampled the road is.** A mark is laid at the animal's *current*
    // head, so a channel observed once every two hundred ticks draws a
    // dotted line between wherever the ant happened to be when the screen
    // was painted. The lab observes per tick and this reads 1.00 there; a
    // draw-only binary does not and this says so rather than leaving the
    // picture to be read as a road.
    let observed = f.observed_frames - base.4;
    println!(
        "  SAMPLED: {observed} of {frames} ticks observed ({:.2} per tick) | alive {alive} animals, {carrying} carrying right now",
        if frames == 0 { 1.0 } else { observed as f64 / frames as f64 }
    );
    let (cells, tiles, tracked) = f.footprint();
    println!("  HELD: {cells} road cells, {tiles} harvest entries, {tracked} animals tracked (frame {})", start + frames);

    // **The answer key, on the scenario this file defaults to.** Printed in
    // world coordinates rather than tile indices: a tile index is this
    // file's own bookkeeping and cannot be checked against a scenario's
    // `x: 470`.
    let mut sources = f.harvest_readout(lab.world.frame);
    sources.sort_by(|a, b| b.3.partial_cmp(&a.3).expect("no NaN"));
    let tile = f.tile;
    let top: Vec<String> = sources
        .iter()
        .take(8)
        .map(|&(colony, tx, ty, v)| format!("colony {colony} at ({},{}) {:.0}", tx * tile + tile / 2, ty * tile + tile / 2, v))
        .collect();
    println!("  SOURCES (heaviest tiles, world coords, decayed face value): {}", if top.is_empty() { "none".into() } else { top.join(" | ") });
    let mut by_colony: std::collections::BTreeMap<u32, (usize, f32)> = std::collections::BTreeMap::new();
    for &(colony, _, _, v) in &sources {
        let e = by_colony.entry(colony).or_insert((0, 0.0));
        e.0 += 1;
        e.1 += v;
    }
    println!(
        "  BY COLONY: {}",
        by_colony.iter().map(|(c, (n, v))| format!("{c}: {n} tiles, {v:.0}")).collect::<Vec<_>>().join(" | ")
    );
}

/// **What the channel costs: two arms alternated in blocks, inside one run.**
///
/// # Two design errors this had first, both of which produced a clean number
///
/// The first version alternated *draw by draw* — off, on, off, on — and
/// reported the overlay as **1.7 ms faster than having it off**. Added work
/// cannot be faster, and the reason it read that way is that the two arms
/// were not exchangeable: only the first draw of each pair followed a tick,
/// so it paid that tick's dirty-rect repaint and the second draw of the pair
/// paid almost nothing. The arms were measuring their position in the pair.
///
/// The second is the one `CLAUDE.md` names outright — *a cost that vanishes
/// may be work that vanished*. Switching the channel off drops both maps by
/// design (`FoodRoad::observe`), so an off-arm interleaved between on-arms
/// wiped the thing being priced: the "on" arm was drawing **2 road cells and
/// 4 harvest tiles** and reporting it as the cost of a live map.
///
/// So: each arm gets a **block** of its own, long enough to re-warm, with
/// the first `DISCARD` draws of the block thrown away; every draw follows
/// exactly one tick; and the blocks alternate so machine drift falls on both.
/// The counter that says the fix worked is printed with the result — the
/// road cells and harvest tiles held *at the moment the on-arm was timed*.
///
/// **The settled arm is the one that matters and is why `settled=1` exists.**
/// An animated grain once looked free in every moving scene and cost
/// ~10 ms/frame on a settled one, because what it defeats is the dirty-rect
/// render skip — and a settled world is exactly where that skip does its
/// work. Both food channels decay every tick and so defeat it by
/// construction, the same way `FieldOverlay` already does.
fn price(lab: &mut Lab, rounds: u32, mode: FoodOverlay, settled: bool, whole: bool) {
    // **The warm has to be long enough to rebuild the map the other arm
    // dropped, and "long enough" is set by the road's own half-life rather
    // than by taste.** At the shipped 600-frame half-life and the played
    // bed's step rate the road settles near 260 cells; a 40-tick warm
    // reached **13**, so the first honest-looking version of this still
    // priced an almost-empty map and said so in its own counter.
    //
    // **Raised 1,500 -> 6,000 when the road's memory went to a minute, and
    // the old value had quietly become a confound rather than merely a short
    // warm.** Each arm re-warms from nothing (the off arm drops both maps by
    // design), so at 1,500 ticks *every* setting of `roadhalf` was priced on
    // the same young map: a run at ten seconds and a run at no decay at all
    // came out holding **420 and 431 cells**, which is `CLAUDE.md`'s own tell
    // -- identical output across a change that must have moved something.
    // At 6,000 the same two hold 623 and 1,165, which is the thing the
    // question was about. Still under two half-lives, so it is a floor on
    // what the warm has to be and not a settled map.
    let warm: u32 = arg("warm").unwrap_or(6000);
    /// Draws timed per block, after the warm.
    const TIMED: u32 = 120;
    let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let mut off: Vec<f64> = Vec::new();
    let mut on: Vec<f64> = Vec::new();
    let mut held = (0usize, 0usize);
    for round in 0..rounds {
        // Order swapped every round, so a machine that is slowing does not
        // hand the whole of its drift to whichever arm always goes second.
        let order = if round.is_multiple_of(2) { [FoodOverlay::Off, mode] } else { [mode, FoodOverlay::Off] };
        for arm in order {
            lab.renderer.food.mode = arm;
            // **The warm always ticks, settled arm included.** A settled bed
            // with an empty map is not the case this is pricing: what it
            // costs to hold a live map on a world that has stopped moving is
            // the whole question, because that is where the dirty-rect skip
            // does its work.
            for _ in 0..warm {
                lab.tick_for_harness();
            }
            lab.draw(&mut buf, 60.0);
            for _ in 0..TIMED {
                // **`whole=1` puts the tick inside the timed span**, which is
                // the figure `CLAUDE.md` says to quote: a subsystem harness
                // aims the work and the whole frame sizes it, and the same
                // field change once measured -50% in its own harness and
                // -27% through `App::update`. On the settled arm there is no
                // tick to include and the two are the same number.
                let mut t = std::time::Instant::now();
                if !settled {
                    lab.tick_for_harness();
                }
                if !whole {
                    // Restarted after the tick, which is the original
                    // instrument: the draw alone.
                    t = std::time::Instant::now();
                }
                lab.draw(&mut buf, 60.0);
                let ms = t.elapsed().as_secs_f64() * 1000.0;
                if arm == FoodOverlay::Off {
                    off.push(ms);
                } else {
                    on.push(ms);
                    let (c, h, _) = lab.renderer.food.footprint();
                    held = (c, h);
                }
            }
        }
    }
    let stat = |v: &mut Vec<f64>| {
        v.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
        (v[v.len() / 2], v[v.len() - 1])
    };
    let (off_med, off_worst) = stat(&mut off);
    let (on_med, on_worst) = stat(&mut on);
    println!(
        "  COST, {} blocks per arm, {warm}-tick warm then {TIMED} timed {} ({}): overlay OFF median {off_med:.2} ms worst {off_worst:.2} ms | \
         {} median {on_med:.2} ms worst {on_worst:.2} ms | median delta {:+.2} ms ({:+.0}%)",
        rounds,
        if whole && !settled { "whole frames -- tick AND draw" } else { "draws" },
        if settled { "settled -- no tick between draws, so the dirty-rect skip can fire" } else { "running -- one tick before every draw" },
        mode.label(),
        on_med - off_med,
        100.0 * (on_med - off_med) / off_med
    );
    // **The counter that says the priced arm was not empty.** Without it a
    // wiped map and a cheap one are the same number.
    println!("  (the timed {} arm was holding {} road cells and {} harvest tiles)", mode.label(), held.0, held.1);
}
