//! **The keeper seat demo, headless** (`demo=keeper` without a window).
//!
//! Builds the lab exactly as `cargo run --bin lab -- demo=keeper` does (the
//! trained ant, then `Lab::start_demo_with`), ticks it, and prints the
//! demo's own counters: ants, starved, digs, station bites and deliveries,
//! the first station delivery, how many ants know the station, and the
//! recall head's counts. With `carry=N@F` it carries the N ants nearest the
//! nest to the station at frame F (the player's teaching, scripted).
//!
//! ```text
//! cargo run --release --example keeperdemo -- seed=1 frames=30000
//! cargo run --release --example keeperdemo -- seed=1 carry=10@6000 gain=6 copy=0.5
//! cargo run --release --example keeperdemo -- seed=1 carry=10@6000 listening=0   # the honest arm
//! ```
//!
//! The demo opens at frame 6,000 (`keeper::DEFAULT_WARMUP`; `warmup=` sets
//! it), so `carry=N@6000` is "the player's first act". `trace=F` prints each
//! taught ant every F frames: head, crop, memory, record, lesson awake.
//!
//! ```text
//! ```
//!
//! One summary line at the end (`SUMMARY key=value ...`), for a sweep to
//! parse. Every argument is echoed first.

use std::collections::BTreeMap;

use pixel_physics::lab::demo::{keeper, DemoSeat};
use pixel_physics::lab::Lab;
use pixel_physics::sim::creature::recall;

fn main() {
    let args: BTreeMap<String, String> = std::env::args().skip(1).filter_map(|a| a.split_once('=').map(|(k, v)| (k.to_string(), v.to_string()))).collect();
    let get = |k: &str, d: u64| args.get(k).map_or(d, |v| v.parse().unwrap_or_else(|_| panic!("{k}={v}: not a number")));
    let seed = get("seed", 1);
    let frames = get("frames", 30_000);
    let every = get("every", 3_000);
    let trace = get("trace", 0);
    let carry: Option<(usize, u64)> = args.get("carry").map(|v| {
        let (n, f) = v.split_once('@').unwrap_or_else(|| panic!("carry={v}: N@FRAME"));
        (n.parse().expect("carry N"), f.parse().expect("carry FRAME"))
    });
    for k in args.keys() {
        assert!(
            ["seed", "frames", "every", "carry", "gain", "copy", "station", "ration", "listening", "trace", "warmup", "png", "shots", "gif", "gif_every", "gif_from", "overlay", "up", "nodemo", "costab"].contains(&k.as_str()),
            "unknown argument {k}= (an unknown key would be silently ignored)"
        );
    }
    println!("keeperdemo: {}", args.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(" "));

    let sc = pixel_physics::lab::scenario::Scenario::load("demo_keeper").expect("demo_keeper ships");
    let mut lab = Lab::new(sc.bed.clone());
    lab.set_new_ant(true);
    let mut kv = args.clone();
    kv.retain(|k, _| ["gain", "copy", "station", "ration", "listening", "warmup"].contains(&k.as_str()));
    kv.insert("seed".to_string(), seed.to_string());
    // Exactly the window's path: the scenario, `seed=`, then the warm-up
    // (`keeper::on_start`), so frame numbers below are the box's own.
    let msg = lab.start_demo_with(DemoSeat::Keeper, kv);
    println!("  {msg} -- opened at frame {}", lab.world.frame);
    lab.show_help = false;
    if let Some(o) = args.get("overlay") {
        let k = lab.demo.as_mut().and_then(|d| d.keeper.as_mut()).expect("keeper state");
        k.overlay = match o.as_str() {
            "knows" => keeper::Overlay::Knows,
            "traffic" => keeper::Overlay::Traffic,
            _ => keeper::Overlay::Off,
        };
    }
    // Pictures: `png=DIR shots=F,F` stills, `gif=PATH gif_every=N gif_from=F`
    // an animation, both through `Lab::draw` (the window's own call, HUD and
    // all); `up=N` a nearest-neighbour upscale.
    let shots: Vec<u64> = args.get("shots").map(|v| v.split(',').map(|f| f.parse().expect("shots=F,F")).collect()).unwrap_or_default();
    let gif_every = get("gif_every", 0);
    let gif_from = get("gif_from", 0);
    let up = get("up", 2) as u32;
    let mut gif_frames: Vec<image::RgbaImage> = Vec::new();
    let grab = |lab: &mut Lab| -> image::RgbaImage {
        let (w, h) = (pixel_physics::lab::WIDTH, pixel_physics::lab::HEIGHT);
        let mut full = vec![0u8; (w * h * 4) as usize];
        lab.draw(&mut full, 60.0);
        let img = image::RgbaImage::from_raw(w, h, full).expect("frame size");
        image::imageops::resize(&img, w * up, h * up, image::imageops::FilterType::Nearest)
    };
    let k0 = lab.demo.as_ref().and_then(|d| d.keeper.as_ref()).expect("keeper state").clone();
    // `nodemo=1`: the same box at the same frame with the demo's hooks off
    // (no counters, no top-up, no ration, no recall) -- the frame-cost
    // control. Its dynamics differ from then on; only its clock is compared.
    let nodemo = get("nodemo", 0) == 1;
    if nodemo {
        lab.demo = None;
        lab.world.recall = None;
    }
    let clock = std::time::Instant::now();
    let opened = lab.world.frame;
    println!("  gain {} copy {} listening {} station x {} ration {}/1000f", k0.gain, k0.copy, k0.listening, k0.station_x, k0.ration);
    let minds = lab.world.minds.is_some();
    assert!(minds, "the box has no minds: this would measure the old ant");

    let mut taught: Vec<pixel_physics::sim::cell::OrganismId> = Vec::new();
    // Each taught ant's lifetime `trip_deliveries` when it was taught: what
    // it delivered before is not the lesson's.
    let mut taught_base: Vec<u32> = Vec::new();
    // Taught-ant frames: laden, unladen with the lesson awake (the tilt on
    // offer), unladen with it dormant or gone.
    let mut tf = [0u64; 3];
    // Per taught ant: at the station last frame, and returns (entries into
    // the station's columns after the carry that put it there).
    let mut taught_here: Vec<bool> = Vec::new();
    let mut returns: Vec<u32> = Vec::new();
    let digs0 = lab.world.creature_stats.digs;
    // The opening line, so a count "since teaching" has its baseline.
    status(&lab, &taught, &taught_base, tf, &returns, digs0);
    while lab.world.frame < frames {
        // The carry before the tick, so `carry=N@6000` lands on the frame the
        // demo opens on (the default warm-up ends at 6,000).
        let f = lab.world.frame;
        if let Some((n, at)) = carry {
            if f == at {
                taught = nearest_nest_ants(&lab, n);
                taught_here = vec![true; taught.len()];
                returns = vec![0; taught.len()];
                taught_base = taught.iter().map(|&id| lab.world.organism(id).map_or(0, |s| s.life.trip_deliveries)).collect();
                for &id in &taught {
                    match keeper::carry_to_station(&mut lab, id) {
                        Ok(to) => println!("  f {f}: carried ant {id:?} to {to:?}"),
                        Err(e) => println!("  f {f}: carry of {id:?} refused: {e}"),
                    }
                }
            }
        }
        lab.tick_for_harness();
        let f = lab.world.frame;
        if shots.contains(&f) {
            let dir = args.get("png").map_or(".", |s| s.as_str());
            let path = format!("{dir}/keeper_s{seed}_f{f}.png");
            grab(&mut lab).save(&path).expect("save png");
            println!("  wrote {path}");
        }
        if gif_every > 0 && f >= gif_from && (f - gif_from).is_multiple_of(gif_every) {
            gif_frames.push(grab(&mut lab));
        }
        let station_x = lab.demo.as_ref().and_then(|d| d.keeper.as_ref()).map_or(0, |k| k.station_x);
        for (i, &id) in taught.iter().enumerate() {
            if let Some(h) = lab.world.organism(id).and_then(|st| st.chain.first().copied()) {
                let here = (h.0 - station_x).abs() <= keeper::STATION_R;
                if here && !taught_here[i] {
                    returns[i] += 1;
                }
                taught_here[i] = here;
            }
        }
        for &id in &taught {
            if let Some(st) = lab.world.organism(id) {
                let awake = recall::lesson(&lab.world, id).is_some();
                let laden = st.crop.is_some() || st.spoil.is_some();
                tf[if laden { 0 } else if awake { 1 } else { 2 }] += 1;
            }
        }
        if trace > 0 && !taught.is_empty() && f.is_multiple_of(trace) {
            for &id in &taught {
                let Some(st) = lab.world.organism(id) else {
                    println!("    trace f {f} ant {id:?}: dead");
                    continue;
                };
                let head = st.chain.first().copied();
                let prov = lab.world.recall.as_ref().and_then(|r| r.provenance.get(&id).copied());
                println!(
                    "    trace f {f} ant {id:?}: head {head:?} crop {:?} last_food {:?} prov {:?} lesson {} energy {:.0}",
                    st.crop.map(|c| c.cells),
                    st.last_food,
                    prov.map(|p| (p.place, p.frame, p.generation)),
                    recall::lesson(&lab.world, id).is_some(),
                    st.energy,
                );
            }
        }
        if f.is_multiple_of(every) || f == frames {
            status(&lab, &taught, &taught_base, tf, &returns, digs0);
        }
    }
    let ms = clock.elapsed().as_secs_f64() * 1000.0 / (lab.world.frame - opened).max(1) as f64;
    println!("  TIME {:.4} ms a frame over {} frames ({})", ms, lab.world.frame - opened, if nodemo { "demo hooks off" } else { "demo on" });
    if nodemo {
        return;
    }
    // **The hooks' own cost on the same population**: 1,200 ticks with the
    // demo, then the hooks taken off and 1,200 more from where that left
    // off (the colony barely changes in 2,400 frames), alternated three
    // times so drift in the box does not land on one arm.
    if get("costab", 0) == 1 {
        let (mut on, mut off) = (0.0f64, 0.0f64);
        let saved = lab.demo.clone();
        for _ in 0..3 {
            lab.demo = saved.clone();
            let t = std::time::Instant::now();
            for _ in 0..1200 {
                lab.tick_for_harness();
            }
            on += t.elapsed().as_secs_f64();
            lab.demo = None;
            let t = std::time::Instant::now();
            for _ in 0..1200 {
                lab.tick_for_harness();
            }
            off += t.elapsed().as_secs_f64();
        }
        let (alive, _, _) = {
            lab.demo = saved.clone();
            keeper::census(&lab)
        };
        println!("  COSTAB {alive} ants: demo on {:.4} ms a frame, hooks off {:.4} ms a frame (3 x 1,200 each, alternated)", on * 1000.0 / 3600.0, off * 1000.0 / 3600.0);
        return;
    }
    if let Some(path) = args.get("gif") {
        let n = gif_frames.len();
        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(80));
        let file = std::fs::File::create(path).expect("gif file");
        let mut enc = image::codecs::gif::GifEncoder::new(file);
        enc.set_repeat(image::codecs::gif::Repeat::Infinite).expect("gif repeat");
        enc.encode_frames(gif_frames.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay))).expect("gif encode");
        println!("  wrote {path} ({n} frames)");
    }
    let (alive, knows, starved) = keeper::census(&lab);
    let c = keeper::counts(&lab).unwrap_or_default();
    let r = lab.world.recall.as_ref().map(|r| r.counts).unwrap_or_default();
    println!(
        "SUMMARY seed={seed} gain={} copy={} listening={} carry={} carried={} ants={alive} starved={starved} digs={} bites={} deliveries={} first={} arrivals={} taught_arrivals={} knows={knows} ration={} tilted={} flipped={} copies={} trip_deliveries={}",
        k0.gain,
        k0.copy,
        k0.listening,
        args.get("carry").map_or("-", |s| s.as_str()),
        c.carried,
        lab.world.creature_stats.digs - digs0,
        c.station_bites,
        c.station_deliveries,
        c.first_station_delivery.map_or(-1, |v| v as i64),
        c.arrivals,
        c.taught_arrivals,
        c.ration_given,
        r.tilted,
        r.picks_flipped,
        r.copies,
        lab.world.creature_stats.trip_deliveries,
    );
}

/// The `n` live ants nearest the nest site, unladen: the ones a keeper's hand
/// would pick up off the mound.
fn nearest_nest_ants(lab: &Lab, n: usize) -> Vec<pixel_physics::sim::cell::OrganismId> {
    let Some(site) = lab.world.nest_sites.first().copied() else { return Vec::new() };
    let Some(ant) = lab.world.species.id_of("ant") else { return Vec::new() };
    let mut v: Vec<(i32, pixel_physics::sim::cell::OrganismId)> = lab
        .world
        .live_organism_ids()
        .into_iter()
        .filter_map(|id| {
            let s = lab.world.organism(id)?;
            if s.species != ant || s.crop.is_some() || !pixel_physics::sim::creature::can_carry(&lab.world, id) {
                return None;
            }
            let h = *s.chain.first()?;
            Some(((h.0 - site.x).abs() + (h.1 - site.surface).abs(), id))
        })
        .collect();
    v.sort();
    v.into_iter().take(n).map(|(_, id)| id).collect()
}

/// One status line: the demo's counters, the recall head's, and the taught
/// ants' own.
fn status(lab: &Lab, taught: &[pixel_physics::sim::cell::OrganismId], taught_base: &[u32], tf: [u64; 3], returns: &[u32], digs0: u64) {
    let f = lab.world.frame;
    let (alive, knows, starved) = keeper::census(lab);
    let c = keeper::counts(lab).unwrap_or_default();
    let r = lab.world.recall.as_ref().map(|r| r.counts).unwrap_or_default();
    let taught_alive = taught.iter().filter(|&&id| lab.world.organism(id).is_some()).count();
    let taught_trips: u32 = taught.iter().zip(taught_base).filter_map(|(&id, &b)| lab.world.organism(id).map(|s| s.life.trip_deliveries.saturating_sub(b))).sum();
    println!(
        "  f {f:>6}: ants {alive:>3} starved {starved:>3} digs {:>6} | station bites {:>4} deliveries {:>4} first {:>6} | arrivals {:>4} taught {:>4} | knows {knows:>3} | taught alive {taught_alive}/{} trips {taught_trips} | ration {} | offered {} tilted {} flipped {} copies {:?} | taught frames laden {} unladen-awake {} dormant {} | returns {:?}",
        lab.world.creature_stats.digs - digs0,
        c.station_bites,
        c.station_deliveries,
        c.first_station_delivery.map_or("-".to_string(), |v| v.to_string()),
        c.arrivals,
        c.taught_arrivals,
        taught.len(),
        c.ration_given,
        r.offered,
        r.tilted,
        r.picks_flipped,
        &r.copies_by_generation[..4],
        tf[0],
        tf[1],
        tf[2],
        returns,
    );
}
