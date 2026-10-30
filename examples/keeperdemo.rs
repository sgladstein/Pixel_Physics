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
//! **Stakes** (on by default, `stakes=0` the sandbox): `keeper=` scripts a
//! player over the windfalls -- `idle` (nobody), `good` (as each windfall
//! lands, every charge on the nearest unladen surface ants, set down at it)
//! or `careless` (the same ants and charges, set down on empty ground clear
//! of every windfall). Every [`keeper::Rules`] field passes through by name
//! (`patch_every=`, `patch_life=` ...). `twin=1` runs the window's idle twin
//! in-process and prints whether it agrees with this run (run it on
//! `keeper=idle`: they must agree to the ant). `endshot=PATH` grabs the box
//! with the verdict card up after the run (waiting for the twin).
//!
//! ```text
//! cargo run --release --example keeperdemo -- seed=1 keeper=good
//! cargo run --release --example keeperdemo -- seed=1 keeper=idle twin=1
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
            ["seed", "frames", "every", "carry", "gain", "copy", "station", "ration", "listening", "trace", "warmup", "png", "shots", "gif", "gif_every", "gif_from", "overlay", "up", "nodemo", "costab", "keeper", "twin", "endshot", "stakes", "oldant", "handshot"].contains(&k.as_str())
                || RULES.contains(&k.as_str()),
            "unknown argument {k}= (an unknown key would be silently ignored)"
        );
    }
    println!("keeperdemo: {}", args.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(" "));

    let sc = pixel_physics::lab::scenario::Scenario::load("demo_keeper").expect("demo_keeper ships");
    let mut lab = Lab::new(sc.bed.clone());
    // `oldant=1`: the hand-built ant (the lab's `-- oldant`), as the window opens it.
    let oldant = get("oldant", 0) == 1;
    lab.set_new_ant(!oldant);
    let mut kv = args.clone();
    kv.retain(|k, _| ["gain", "copy", "station", "ration", "listening", "warmup", "stakes"].contains(&k.as_str()) || RULES.contains(&k.as_str()));
    kv.insert("seed".to_string(), seed.to_string());
    // The window's twin is started by the window's frame hook, which a
    // harness never calls; `twin=1` starts it here instead.
    kv.insert("twin".to_string(), "0".to_string());
    kv.insert("twin_oldant".to_string(), if oldant { "1" } else { "0" }.to_string());
    let script = args.get("keeper").map_or("idle", |s| s.as_str()).to_string();
    assert!(["idle", "good", "careless"].contains(&script.as_str()), "keeper={script}: idle, good or careless");
    // Exactly the window's path: the scenario, `seed=`, then the warm-up
    // (`keeper::on_start`), so frame numbers below are the box's own.
    let msg = lab.start_demo_with(DemoSeat::Keeper, kv.clone());
    println!("  {msg} -- opened at frame {}", lab.world.frame);
    lab.show_help = false;
    let twin = (get("twin", 0) == 1).then(|| {
        let until = lab.world.frame + lab.demo.as_ref().and_then(|d| d.keeper.as_ref()).map_or(0, |k| k.rules.round);
        let log = keeper::spawn_twin(kv.clone(), until);
        if let Some(k) = lab.demo.as_mut().and_then(|d| d.keeper.as_mut()) {
            k.twin = Some(log.clone());
        }
        log
    });
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
    println!("  gain {} copy {} listening {} station x {} ration {}/1000f stakes {} rules {:?} keeper {script}", k0.gain, k0.copy, k0.listening, k0.station_x, k0.ration, k0.stakes, k0.rules);
    let mut patches_seen = 0u64;
    let minds = lab.world.minds.is_some();
    assert!(minds != oldant, "the box's brain is not the one asked for (oldant={oldant})");

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
        // **The scripted keeper**: as each windfall lands, every charge.
        let landed = keeper::counts(&lab).map_or(0, |c| c.patches);
        if landed > patches_seen {
            patches_seen = landed;
            if script != "idle" {
                scripted_keeper(&mut lab, &script);
            }
        }
        // `handshot=F`: the player's own mouse path at frame F -- CARRY,
        // press on the nearest surface ant, drag it half way to the newest
        // windfall, a still with the ant riding the hand, then the drag on
        // to the windfall and the release.
        if get("handshot", 0) == f {
            use pixel_physics::lab::demo::MouseAct;
            let target = lab.demo.as_ref().and_then(|d| d.keeper.as_ref()).and_then(|k| k.patches.last().copied());
            let picked = surface_ants(&lab, 1).first().and_then(|&id| lab.world.organism(id).and_then(|s| s.chain.first().copied()));
            if let (Some(p), Some(from)) = (target, picked) {
                keeper::seat_key(&mut lab, 'Y');
                keeper::mouse(&mut lab, (from.0 + 2, from.1 - 1), MouseAct::Press { erase: false });
                let mid = ((from.0 + p.x) / 2, p.y - 30);
                keeper::mouse(&mut lab, mid, MouseAct::Drag);
                let dir = args.get("png").map_or(".", |s| s.as_str());
                let path = format!("{dir}/keeper_s{seed}_hand_f{f}.png");
                grab(&mut lab).save(&path).expect("save png");
                println!("  wrote {path} (pressed beside {from:?}, held {:?})", lab.demo.as_ref().and_then(|d| d.keeper.as_ref()).and_then(|k| k.held));
                keeper::mouse(&mut lab, (p.x - 3, p.y - 2), MouseAct::Drag);
                keeper::mouse(&mut lab, (p.x - 3, p.y - 2), MouseAct::Release);
                keeper::seat_key(&mut lab, 'Y');
            }
        }
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
    if let Some(t) = twin.as_ref() {
        while !t.lock().map(|l| l.done).unwrap_or(true) {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        let l = t.lock().expect("twin log");
        let last = l.points.last().copied();
        let alive = keeper::census(&lab).0;
        let home = keeper::counts(&lab).map_or(0, |c| c.patch_home);
        let print = lab.demo.as_ref().and_then(|d| d.keeper.as_ref()).and_then(|k| k.opening_print);
        println!(
            "  TWIN opening {} | twin at frame {:?}: ants {:?} home {:?} | this run: ants {alive} home {home} | {}",
            if l.opening == print { "MATCHES" } else { "DIFFERS" },
            last.map(|p| p.frame),
            last.map(|p| p.ants),
            last.map(|p| p.home),
            if last.is_some_and(|p| p.ants == alive && p.home == home && p.saved == keeper::counts(&lab).map_or(0, |c| c.saved)) { "AGREE" } else { "DISAGREE" },
        );
    }
    if let Some(path) = args.get("endshot") {
        if let Some(k) = lab.demo.as_mut().and_then(|d| d.keeper.as_mut()) {
            k.card = true;
        }
        grab(&mut lab).save(path).expect("save endshot");
        println!("  wrote {path}");
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
    let kd = lab.demo.as_ref().and_then(|d| d.keeper.as_ref()).expect("keeper state");
    let (end_frame, end_why, end_ants, end_home, end_saved) = kd.ended.map_or((0, "-".to_string(), alive, c.patch_home, c.saved), |e| (e.frame, format!("{:?}", e.why), e.ants, e.home, e.saved));
    for p in &kd.patches {
        println!("  PATCH x {} landed {} size {} left {} home {} taught {} ended {:?}", p.x, p.landed, p.size, p.left, p.home, p.taught, p.ended);
    }
    println!(
        "STAKES seed={seed} keeper={script} stakes={} opened_ants={} end_frame={end_frame} end={end_why} end_ants={end_ants} end_home={end_home} end_saved={end_saved} min_ants={} patches={} stripped={} rotted={} rotted_cells={} carried={} to_food={} refused={} pantry={} treated={}",
        kd.stakes,
        kd.ants_at_open,
        c.min_ants,
        c.patches,
        c.stripped,
        c.rotted,
        c.rotted_cells,
        c.carried,
        c.carried_to_food,
        c.refused,
        kd.pantry,
        c.treated,
    );
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

/// Every [`keeper::Rules`] field, by its demo-argument name.
const RULES: [&str; 10] = ["patch_every", "patch_life", "patch_cells", "charges", "recharge", "round", "lose_at", "stakes_ration", "treat_cost", "lift_cost"];

/// **A scripted keeper's turn**, as a windfall lands: every charge in the
/// hand, on the nearest unladen ants on the surface by the nest (the ones a
/// player can see and click), through the player's own path
/// (`keeper::hand_carry`, charges and all). `good` sets them down at the
/// new windfall; `careless` on empty ground clear of every live windfall.
fn scripted_keeper(lab: &mut Lab, script: &str) {
    let Some(k) = lab.demo.as_ref().and_then(|d| d.keeper.as_ref()) else { return };
    let Some(p) = k.patches.last().copied() else { return };
    let charges = k.charges as usize;
    let to = if script == "good" {
        (p.x - 3, p.y - 2)
    } else {
        let live: Vec<i32> = k.patches.iter().filter(|q| q.live()).map(|q| q.x).collect();
        let x = [250, 470, 330, 400, 290, 440]
            .into_iter()
            .find(|&x| live.iter().all(|&q| (q - x).abs() > 2 * keeper::PATCH_R + 8))
            .unwrap_or(250);
        (x, keeper::surface_under(&lab.world, x) - 2)
    };
    // The careless hand does not look which ant it takes: the nearest to the
    // nest site at any depth (nurses and diggers as often as foragers).
    let picks = if script == "good" { surface_ants(lab, charges) } else { nearest_nest_ants(lab, charges) };
    for id in picks {
        match keeper::hand_carry(lab, id, to) {
            Ok((at, taught)) => println!("  f {}: {script} hand: ant {id:?} to {at:?}, taught {taught:?}", lab.world.frame),
            Err(e) => println!("  f {}: {script} hand: ant {id:?} refused: {e}", lab.world.frame),
        }
    }
}

/// The `n` liftable unladen ants nearest the nest mouth, those on the
/// surface first.
fn surface_ants(lab: &Lab, n: usize) -> Vec<pixel_physics::sim::cell::OrganismId> {
    let Some(site) = lab.world.nest_sites.first().copied() else { return Vec::new() };
    let Some(ant) = lab.world.species.id_of("ant") else { return Vec::new() };
    let mut v: Vec<(bool, i32, pixel_physics::sim::cell::OrganismId)> = lab
        .world
        .live_organism_ids()
        .into_iter()
        .filter_map(|id| {
            let s = lab.world.organism(id)?;
            if s.species != ant || s.crop.is_some() || !pixel_physics::sim::creature::can_carry(&lab.world, id) {
                return None;
            }
            let h = *s.chain.first()?;
            let under = h.1 > keeper::surface_under(&lab.world, h.0) + 1;
            Some((under, (h.0 - site.x).abs() + (h.1 - site.surface).abs(), id))
        })
        .collect();
    v.sort();
    v.into_iter().take(n).map(|(_, _, id)| id).collect()
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
