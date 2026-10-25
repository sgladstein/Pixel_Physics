//! **The lone scout demo, played by a script** (`sim::tandem`; plan
//! `Reports/teaching-the-colony-2026-10-25/scout-demo/plan-v2.md` §4).
//!
//! Loads `demo_scout.ron` with the trained ant, warms the box to the hand-over
//! frame, picks the player's body (a modal-pace adult near the nest) and plays
//! her by a script: walk home, tap the nearest sister, lead her to the far
//! heap, **wait whenever her pip would turn amber** (a tell, a stop, or a
//! search), then walk home and do it again. Prints the tandem counters every
//! `every=` frames so a run can be judged by number as well as by picture.
//!
//! ```text
//! cargo run --release --example scoutdemo -- arm=A seed=1 frames=36000
//! cargo run --release --example scoutdemo -- arm=A seed=1 rude=1     # a player who never waits
//! cargo run --release --example scoutdemo -- arm=A seed=1 gain=0     # the memory pull off: the honest arm
//! cargo run --release --example scoutdemo -- arm=A seed=1 off=1      # module off, same player: frame cost
//! cargo run --release --example scoutdemo -- arm=A seed=1 frames=6000 gif=1 shot=40 zoom=6 out=a.gif
//! ```
//!
//! The run goes through the seat's own start (`lab::demo::scout::start_tandems`, `pick_body`, the worker's
//! role) and drives the body through the keys (`SteerInput`), so its pictures are the binary's.
//!
//! `rude=1` is the positive control for the pauses: a leader who walks on
//! through every stop must break runs that a waiting leader keeps.

use pixel_physics::lab::demo::{scout, Demo, DemoSeat};
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::Lab;
use pixel_physics::sim::cell::OrganismId;
use pixel_physics::sim::steer::{self, SteerInput, SteerRole};
use pixel_physics::sim::tandem::{self, Arm, Phase, TapResult};
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

use scout::{BODY_X as TAP_X, HEAP_X, NEST_X};

fn head(world: &World, id: OrganismId) -> Option<(i32, i32)> {
    world.organism(id).and_then(|s| s.chain.first().copied())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Plan {
    Home,
    Tapping,
    Leading,
}

fn main() {
    let arm = match arg::<String>("arm").unwrap_or_else(|| "A".into()).as_str() {
        "B" => Arm::Column,
        "C" => Arm::Escort,
        _ => Arm::Scout,
    };
    let seed: u64 = arg("seed").unwrap_or(1);
    let start: u64 = arg("start").unwrap_or(18_000);
    let frames: u64 = arg("frames").unwrap_or(36_000);
    let every: u64 = arg("every").unwrap_or(3_000);
    let rude = arg::<u8>("rude").unwrap_or(0) == 1;
    let c: Option<f32> = arg("c");
    let trace = arg::<u8>("trace").unwrap_or(0) == 1;
    println!("scoutdemo arm={} seed={seed} start={start} frames={frames} every={every} rude={rude} c={c:?} gain={:?}", arm.letter(), arg::<f32>("gain"));

    let mut sc = Scenario::load("demo_scout").expect("demo_scout.ron ships");
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    lab.load_scenario(sc);
    lab.set_new_ant(true);
    while lab.world.frame < start {
        lab.tick_for_harness();
    }
    let alive0 = scout::ants(&lab.world).len();
    // **The seat's own start**, so this run and `--bin lab -- demo=scout` begin the same way: the seat's
    // tandem set-up, the seat's choice of body, the worker's role (her colony keeps its own breeding rule),
    // and the seat's overlays and panel in every captured frame.
    let gain: f32 = arg("gain").unwrap_or(scout::GAIN);
    scout::start_tandems(&mut lab.world, arm, c, gain);
    let mut demo = Demo::new(DemoSeat::Scout);
    let mut args = std::collections::BTreeMap::new();
    args.insert("arm".to_string(), arm.letter().to_string());
    if let Some(c) = c {
        args.insert("c".to_string(), c.to_string());
    }
    args.insert("gain".to_string(), gain.to_string());
    let mut seat = scout::ScoutDemo::from_args(&args, seed);
    seat.started = true;
    seat.alive_at_start = alive0;
    demo.scout = Some(seat);
    demo.args = args;
    lab.demo = Some(demo);
    lab.world.death_log = Some(Vec::new());
    let Some(player) = scout::pick_body(&lab.world) else {
        println!("no player body: {alive0} ants alive at {start}");
        return;
    };
    // Driven through the keys exactly as the window drives her (`SteerInput`), not `steer::force`.
    steer::mark_with(&mut lab.world, player, Some(SteerInput::default()), SteerRole::Worker);
    println!("hand-over at {}: {alive0} ants alive, player #{player}", lab.world.frame);

    // **Capture** (`gif=1 out=x.gif shot=N zoom=Z`): the seat's own frame, the camera on the player, one shot
    // every `shot` frames, through `lab.draw` exactly as `labgif` does.
    let gif = arg::<u8>("gif").unwrap_or(0) == 1;
    let out: String = arg("out").unwrap_or_else(|| format!("target/scoutdemo-{}-s{seed}.gif", arm.letter()));
    let shot: u64 = arg("shot").unwrap_or(60);
    let zoom: i32 = arg("zoom").unwrap_or(2);
    let (full_w, full_h) = (pixel_physics::lab::WIDTH, pixel_physics::lab::HEIGHT);
    if gif {
        // The welcome card a fresh lab opens on would sit over every shot.
        lab.show_help = false;
        for _ in 1..zoom {
            lab.renderer.adjust_zoom(1);
        }
    }
    let bounds = pixel_physics::sim::chunk::Rect::new(0, 0, lab.spec.width - 1, lab.spec.height - 1);
    let (span_x, span_y) = lab.renderer.visible_span((full_w, full_h));
    let mut shots: Vec<image::RgbaImage> = Vec::new();

    let mut plan = Plan::Home;
    // `off=1`: the same player and script, the module disarmed for every tick (re-armed empty between ticks so
    // the script's reads still work). The frame-cost control: the tick time is what the module adds.
    let off = arg::<u8>("off").unwrap_or(0) == 1;
    let map_at: Option<u64> = arg("map");
    let mut tick_ns: u128 = 0;
    let mut ticks: u64 = 0;
    let mut refusals: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    let mut tap_tries = 0u32;
    let mut leads_done = 0u32;
    let mut gaps = [0u64; 8];
    let mut best_dist = i32::MAX;
    let mut last_goal = 0;
    let mut moved_at = 0u64;
    let end = start + frames;
    while lab.world.frame < end {
        let w = &mut lab.world;
        let Some(at) = head(w, player) else {
            let cause = w.death_log.as_ref().and_then(|l| l.iter().rev().find(|r| r.id == player)).map(|r| r.cause);
            println!("frame {}: the player died ({cause:?})", w.frame);
            break;
        };
        let pairs: Vec<_> = w.tandem.as_ref().map_or(&[][..], |t| &t.pairs[..]).iter().filter(|p| p.leader == player).cloned().collect();
        for p in &pairs {
            if let Some(g) = tandem::contact_gap(w, p.follower, player) {
                gaps[(g as usize).min(7)] += 1;
            }
        }
        let dir: Option<(i8, i8)> = match plan {
            Plan::Home => {
                if (at.0 - TAP_X).abs() <= 3 {
                    plan = Plan::Tapping;
                    tap_tries = 0;
                    None
                } else {
                    Some(((TAP_X - at.0).signum() as i8, 0))
                }
            }
            Plan::Tapping if tandem::candidates(w, player, 3).is_empty() => {
                // Nobody in reach: walk toward the nest, where sisters are, a step at a time.
                if w.frame.is_multiple_of(30) {
                    tap_tries += 1;
                }
                if tap_tries > 40 {
                    plan = Plan::Leading;
                }
                (at.0 > NEST_X + 6).then_some((-1, 0))
            }
            Plan::Tapping => {
                if w.frame.is_multiple_of(30) {
                    tap_tries += 1;
                    let r = tandem::tap(w, player);
                    *refusals.entry(format!("{:?}", r).split('(').next().unwrap_or("").to_string() + &match r {
                        TapResult::Refused(_, why) => format!(" {why:?}"),
                        _ => String::new(),
                    }).or_insert(0u32) += 1;
                    match r {
                        TapResult::Accepted(_) => plan = Plan::Leading,
                        _ if tap_tries > 20 => plan = Plan::Leading,
                        _ => {}
                    }
                }
                None
            }
            Plan::Leading => {
                if pairs.is_empty() {
                    leads_done += 1;
                    plan = Plan::Home;
                    None
                } else {
                    // A patient leader stands while any pupil lags, stops or searches; a rude one never does.
                    let wait = !rude
                        && pairs.iter().any(|p| {
                            let g = tandem::contact_gap(w, p.follower, player).unwrap_or(99);
                            !matches!(p.phase, Phase::Following) || g >= 3
                        });
                    if wait {
                        None
                    } else if at.0 >= HEAP_X - 8 {
                        // She stops at the near edge of the heap, not on it: the heap's next drop lands on
                        // whoever stands in it, and a body under it has no option but to turn round.
                        None
                    } else {
                        Some(((HEAP_X - at.0).signum() as i8, 0))
                    }
                }
            }
        };
        // The headless player eats at the heap (the E key: pick up, or eat from the crop), as a person would.
        let hungry = w.organism(player).is_some_and(|s| s.energy < 1_500.0);
        let food_near = w.materials.id_of("provisions").is_some_and(|f| (-3..=3).any(|dx| (-3..=3).any(|dy| w.get(at.0 + dx, at.1 + dy).material == f)));
        let eat = hungry && food_near;
        // `map=FRAME`: the cells round the player and her options at that frame (`a` an ant's cell, `@` her
        // head). How the buried-by-the-heap and walking-backwards stalls were found.
        if map_at == Some(w.frame) {
            for y in at.1 - 8..=at.1 + 3 {
                let row: String = (at.0 - 14..=at.0 + 14)
                    .map(|x| {
                        let c = w.get(x, y);
                        if (x, y) == at {
                            '@'
                        } else if c.organism_id() != 0 {
                            'a'
                        } else if c.is_empty() {
                            '.'
                        } else {
                            w.materials.get(c.material).name.chars().next().unwrap_or('?')
                        }
                    })
                    .collect();
                println!("    {y}: {row}");
            }
            let st = w.organism(player).unwrap();
            println!("    chain {:?}", st.chain);
            let ant = w.species.id_of("ant").unwrap();
            let def = w.species.creature_shared(ant).unwrap();
            let mut opts = Vec::new();
            pixel_physics::sim::creature::options::build_with(w, player, &def, true, &mut opts);
            println!("    opts {:?}", opts.iter().map(|o| (o.verb, o.target)).collect::<Vec<_>>());
        }
        if trace && w.frame.is_multiple_of(150) {
            let e = w.organism(player).map_or(0.0, |s| s.energy);
            let fs: Vec<String> = pairs.iter().map(|p| format!("{:?}@{:?} {:?} l{}/{}", p.follower, head(w, p.follower), p.phase, p.learned, p.met)).collect();
            println!("  t{} player {:?} e{:.0} {:?} dir {:?} | {}", w.frame, at, e, plan, dir, fs.join(" ; "));
        }
        // A scripted player who is stuck (a tunnel wall, a face) tries upward headings in turn, as a person
        // pressing Up would; a real player reads the screen instead.
        // Progress is getting nearer the goal column, not merely moving: a chain that turns round in place moves
        // its head every decision and gets nowhere.
        let goal = match plan {
            Plan::Home => TAP_X,
            Plan::Tapping => NEST_X,
            Plan::Leading => HEAP_X - 8,
        };
        let dist = (goal - at.0).abs();
        if dir.is_none() || dist < best_dist || goal != last_goal {
            best_dist = dist;
            last_goal = goal;
            moved_at = w.frame;
        }
        let dir = match dir {
            Some(d) if w.frame.saturating_sub(moved_at) > 40 => {
                let alts = [(d.0, -1), (0, -1), (-d.0, -1), (-d.0, 0), (d.0, 1)];
                Some(alts[((w.frame - moved_at) / 40 - 1) as usize % alts.len()])
            }
            other => other,
        };
        // Boxed in for a while (provisions and spoil pile over the nest mound), she digs her way out, as a player
        // pressing G would.
        let boxed = dir.is_some() && w.frame.saturating_sub(moved_at) > 200;
        if let Some(st) = w.steered.as_mut() {
            st.input = Some(SteerInput { dir, eat, dig: boxed && !eat, ..SteerInput::default() });
        }
        if off {
            lab.world.tandem = None;
        }
        let t0 = std::time::Instant::now();
        lab.tick_for_harness();
        tick_ns += t0.elapsed().as_nanos();
        ticks += 1;
        if off {
            lab.world.tandem = Some(Box::new(tandem::Tandems::new(arm, None)));
        }
        if gif && (lab.world.frame - start).is_multiple_of(shot) {
            if let Some((hx, hy)) = head(&lab.world, player) {
                // A third of the way down, not centred: the seat panel and the bar cover the lower half.
                lab.renderer.set_camera(hx - span_x / 2, hy - span_y / 3, (full_w, full_h), Some(bounds));
            }
            let mut full = vec![0u8; (full_w * full_h * 4) as usize];
            lab.draw(&mut full, 60.0);
            if let Some(img) = image::RgbaImage::from_raw(full_w, full_h, full) {
                shots.push(img);
            }
        }
        if (lab.world.frame - start).is_multiple_of(every) {
            let t = lab.world.tandem.as_ref().unwrap();
            let knows = t.lessons.len();
            println!(
                "f{} alive {} plan {:?} | {:?} | know {} income/min {} | gaps0-7 {:?} | recall tilted {} flipped {}",
                lab.world.frame,
                scout::ants(&lab.world).len(),
                plan,
                t.counts,
                knows,
                t.income_over(lab.world.frame, 3_600),
                gaps,
                lab.world.recall.as_ref().map_or(0, |r| r.counts.tilted),
                lab.world.recall.as_ref().map_or(0, |r| r.counts.picks_flipped)
            );
        }
    }
    if gif {
        let delay_ms = arg::<u64>("delay").unwrap_or(80).max(1);
        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(delay_ms));
        let n = shots.len();
        let frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
        match std::fs::File::create(&out) {
            Ok(file) => {
                let mut enc = image::codecs::gif::GifEncoder::new(file);
                let _ = enc.set_repeat(image::codecs::gif::Repeat::Infinite);
                if let Err(e) = enc.encode_frames(frames) {
                    eprintln!("scoutdemo: gif encode failed: {e}");
                }
                println!("gif: {n} shots, one every {shot} frames, {delay_ms} ms each, zoom {zoom} -> {out}");
            }
            Err(e) => eprintln!("scoutdemo: cannot write {out}: {e}"),
        }
    }
    println!("tap outcomes: {refusals:?}");
    println!("tick cost: {:.3} ms mean over {ticks} ticks (off={off})", tick_ns as f64 / ticks.max(1) as f64 / 1e6);
    let t = lab.world.tandem.as_ref().unwrap();
    println!("END arm={} seed={seed} rude={rude}: leads {leads_done} alive {}->{} {:?}", arm.letter(), alive0, scout::ants(&lab.world).len(), t.counts);
}
