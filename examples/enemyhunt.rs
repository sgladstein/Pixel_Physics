//! **The enemy seat played by a script**: the `demo=enemy` box (a colony
//! digging in under grow lights, a steered centipede set down on the surface
//! at frame 26,000) with the keyboard replaced by a small autopilot that
//! presses the same keys a player would, through `Lab::demo_key`. Built
//! 2026-10-25 for the enemy seat demo (`Reports/teaching-the-colony-2026-10-25/enemy-seat-demo.md`),
//! to answer the plan's measurement A0.2 -- **how often does the colony bite
//! back once the raider is inside the nest**, trained brain against the
//! hand-built one -- and to render the demo's GIF without a window.
//!
//! ```text
//! cargo run --release --example enemyhunt -- seed=1 after=12000
//! cargo run --release --example enemyhunt -- seed=1 oldant            # the hand-built colony
//! cargo run --release --example enemyhunt -- seed=1 policy=skulk gif=hunt.gif every=3 card=card.png
//! ```
//!
//! `policy=skulk` is the player who waits out on the surface for a forager
//! walking alone, away from the mound, and backs off when bitten; the
//! default (`door`) walks straight at the nearest door. `land=` is the
//! demo's own (how far from the door the raider is set down). `drawcost`
//! prints what the seat's view costs `Lab::draw`, in microseconds.
//!
//! The autopilot is deliberately dumb and the same for both brains: walk to
//! the nearest door, hold down and dig until the head is three rows under
//! the surface, then steer at the nearest ant within 24 cells and press B
//! whenever one is within two; press E after a kill; back out (hold up) once
//! the body is badly gnawed or has lost two cells, and rest on the surface
//! for 1,500 frames. A press of G after 120 frames without the head moving.
//! Every row says which state the script was in, so a number can be read
//! against what the player was doing.
//!
//! **What it counts** comes from the demo's own counters (`EnemyDemo`), which
//! read the world's encounter and kill logs -- the same numbers the HUD and
//! the season card print -- plus the frames the head spent under the surface,
//! so `taken_per_min_in_nest` is bites on the raider per 3,600 frames inside.
use pixel_physics::lab::demo::{Demo, DemoKey, DemoSeat};
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

fn flag(key: &str) -> bool {
    std::env::args().skip(1).any(|a| a == key)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Approach,
    Hunt,
    Retreat,
    Rest,
    /// `policy=skulk`: wait out on the surface for a forager walking alone.
    Stalk,
    Flee,
    Heal,
}

/// The keys the script holds this frame, as the layer's levels.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Hold {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

fn ants(world: &World) -> Vec<(i32, i32)> {
    let Some(ant) = world.species.id_of("ant") else { return Vec::new() };
    world
        .live_organism_ids()
        .into_iter()
        .filter_map(|id| world.organism(id).filter(|s| s.species == ant).and_then(|s| s.chain.first().copied()))
        .collect()
}

/// The ground row at a door nearest `x`, and that door's column.
fn nearest_door(world: &World, x: i32) -> Option<(i32, i32)> {
    world.nest_sites.iter().map(|s| (s.x, s.surface)).min_by_key(|&(dx, _)| (dx - x).abs())
}

fn main() {
    let seed: u64 = arg("seed").unwrap_or(1);
    let after: u64 = arg("after").unwrap_or(12_000);
    let oldant = flag("oldant");
    let skulk = arg::<String>("policy").as_deref() == Some("skulk");
    let gif: Option<String> = arg("gif");
    let every: u64 = arg("every").unwrap_or(6);
    let row_every: u64 = arg("rows").unwrap_or(600);
    println!("enemyhunt seed={seed} after={after} brain={} policy={} gif={gif:?} every={every}", if oldant { "hand-built" } else { "trained" }, if skulk { "skulk" } else { "door" });
    let mut home_x: Option<i32> = None;

    let mut sc = Scenario::load("demo_enemy").expect("demo_enemy ships");
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    lab.show_help = false;
    if lab.stats.showing() {
        lab.stats.toggle();
    }
    if !oldant {
        lab.set_new_ant(true);
    }
    // `Lab::start_demo` loads the scenario from disk; the seed has to be set
    // on it first, so the launcher's three steps are done here by hand.
    let mut demo = Demo::new(DemoSeat::Enemy);
    // Every `key=value` reaches the seat, as `bin/lab.rs` hands them on
    // (`land=` is how far from the door the raider is set down).
    demo.args = std::env::args().skip(1).filter_map(|a| a.split_once('=').map(|(k, v)| (k.to_string(), v.to_string()))).collect();
    lab.demo = Some(demo);
    lab.load_scenario(sc);
    lab.demo_bind_body();

    let mut hold = Hold::default();
    let mut state = if skulk { State::Stalk } else { State::Approach };
    let mut state_since = 0u64;
    let mut last_head: Option<(i32, i32)> = None;
    let mut still = 0u64;
    let mut kills_seen = 0u32;
    let mut in_nest_frames = 0u64;
    let mut taken_in_nest = 0u32;
    let mut taken_seen = 0u32;
    let mut deepest = 0i32;
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let mut landed_at: Option<u64> = None;
    let mut start_cells: Option<usize> = None;
    let drawcost = flag("drawcost");
    let mut cost: Vec<(u64, u64)> = Vec::new();

    loop {
        lab.demo_frame();
        let e = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).cloned().expect("enemy seat");
        if landed_at.is_none() && e.landed.is_some() {
            landed_at = e.landed;
            println!(
                "landed frame={} ants={} trips={} raider_cells={}",
                lab.world.frame,
                e.ants_at_landing,
                e.trips_at_landing,
                e.raider.and_then(|id| lab.world.organism(id)).map_or(0, |s| s.chain.len())
            );
        }
        let Some(t0) = landed_at else {
            lab.tick_for_harness();
            if lab.world.frame.is_multiple_of(5000) {
                println!("warm frame={} ants={}", lab.world.frame, ants(&lab.world).len());
            }
            continue;
        };
        let now = lab.world.frame;
        if now >= t0 + after || e.ended.is_some_and(|(f, _)| now > f + 300) {
            break;
        }
        let body = e
            .raider
            .and_then(|id| lab.world.organism(id))
            .filter(|_| e.ended.is_none())
            .map(|s| (s.chain.first().copied().unwrap_or((0, 0)), s.chain.len(), s.gnawed, s.crop.map_or(0.0, |c| c.worth()), s.energy));
        let mut want = Hold::default();
        if let Some((head, cells, gnawed, crop, energy)) = body {
            let start_cells = *start_cells.get_or_insert(cells);
            let (door_x, surface) = nearest_door(&lab.world, head.0).unwrap_or((head.0, head.1));
            let depth = head.1 - surface;
            deepest = deepest.max(depth);
            if depth >= 3 {
                in_nest_frames += 1;
            }
            if e.bites_taken > taken_seen {
                if depth >= 3 {
                    taken_in_nest += e.bites_taken - taken_seen;
                }
                taken_seen = e.bites_taken;
            }
            let hurt = gnawed >= 0.6 || e.cells_lost.len() >= 2 && cells + 2 <= start_cells;
            let home = *home_x.get_or_insert(head.0);
            let next = match state {
                State::Stalk if gnawed >= 0.4 || cells < start_cells => State::Flee,
                State::Flee if now - state_since >= 300 => State::Heal,
                State::Heal if gnawed < 0.05 && now - state_since >= 300 => State::Stalk,
                State::Approach | State::Hunt if hurt => State::Retreat,
                State::Approach if depth >= 3 => State::Hunt,
                State::Retreat if depth <= 0 => State::Rest,
                State::Rest if now - state_since >= 1500 => State::Approach,
                st => st,
            };
            if next != state {
                state = next;
                state_since = now;
            }
            let mut press: Option<char> = None;
            match state {
                State::Approach => {
                    if (head.0 - door_x).abs() > 1 {
                        want.left = head.0 > door_x;
                        want.right = head.0 < door_x;
                    } else {
                        want.down = true;
                    }
                }
                State::Hunt => {
                    let target = ants(&lab.world).into_iter().map(|a| (a, (a.0 - head.0).abs() + (a.1 - head.1).abs())).filter(|&(_, d)| d <= 24).min_by_key(|&(_, d)| d);
                    match target {
                        Some((a, d)) => {
                            let (dx, dy) = (a.0 - head.0, a.1 - head.1);
                            if dx.abs() >= dy.abs() {
                                want.left = dx < 0;
                                want.right = dx > 0;
                            } else {
                                want.up = dy < 0;
                                want.down = dy > 0;
                            }
                            if d <= 2 {
                                press = Some('B');
                            }
                        }
                        // Nobody near: go deeper, toward the chambers.
                        None => want.down = true,
                    }
                    if e.kills > kills_seen {
                        kills_seen = e.kills;
                        press = Some('E');
                    }
                }
                State::Retreat => want.up = true,
                State::Rest | State::Heal => {}
                State::Stalk => {
                    // A forager is fair game when it walks alone and away
                    // from the mound: no more than one other ant within 6
                    // cells of it, and 12 columns from the door or more.
                    let all = ants(&lab.world);
                    let alone = |a: (i32, i32)| all.iter().filter(|b| **b != a && (b.0 - a.0).abs() + (b.1 - a.1).abs() <= 6).count() <= 1;
                    let target = all.iter().copied().filter(|a| (a.0 - door_x).abs() >= 12 && alone(*a)).map(|a| (a, (a.0 - head.0).abs() + (a.1 - head.1).abs())).filter(|&(_, d)| d <= 16).min_by_key(|&(_, d)| d);
                    match target {
                        Some((a, d)) => {
                            let dx = a.0 - head.0;
                            want.left = dx < 0;
                            want.right = dx > 0;
                            if d <= 2 {
                                press = Some('B');
                            }
                        }
                        None if (head.0 - home).abs() > 2 => {
                            want.left = head.0 > home;
                            want.right = head.0 < home;
                        }
                        None => {}
                    }
                    if e.kills > kills_seen {
                        kills_seen = e.kills;
                        press = Some('E');
                    }
                }
                State::Flee => {
                    want.left = head.0 <= door_x;
                    want.right = head.0 > door_x;
                }
            }
            if Some(head) == last_head && (want.left || want.right || want.up || want.down) {
                still += 1;
            } else {
                still = 0;
            }
            last_head = Some(head);
            if still >= 120 && press.is_none() {
                press = Some('G');
                still = 0;
            }
            if let Some(c) = press {
                lab.demo_key(DemoKey::Letter(c), true);
                lab.demo_key(DemoKey::Letter(c), false);
            }
            if (now - t0).is_multiple_of(row_every) {
                println!(
                    "row t={} state={state:?} head={head:?} depth={depth} cells={cells} gnawed={gnawed:.2} crop={:.2} energy={:.2} kills={} given={} taken={} taken_in_nest={taken_in_nest} ants={} trips={}",
                    now - t0,
                    crop,
                    energy,
                    e.kills,
                    e.bites_given,
                    e.bites_taken,
                    ants(&lab.world).len(),
                    lab.world.creature_stats.trip_deliveries - e.trips_at_landing,
                );
            }
        }
        for (key, on, was) in [(DemoKey::Left, want.left, hold.left), (DemoKey::Right, want.right, hold.right), (DemoKey::Up, want.up, hold.up), (DemoKey::Down, want.down, hold.down)] {
            if on != was {
                lab.demo_key(key, on);
            }
        }
        hold = want;
        lab.tick_for_harness();
        // `drawcost`: every displayed frame drawn twice, the demo's view (the
        // camera following, the marks) and then the plain lab at the same
        // camera with the seat taken away, so the difference is what the
        // seat costs the draw. The plain draw follows a demo draw, so its
        // dirty-rect skip is the box's own between two frames.
        if drawcost {
            let t = std::time::Instant::now();
            lab.draw(&mut buf, 60.0);
            let with = t.elapsed().as_micros() as u64;
            let seat = lab.demo.take();
            let t = std::time::Instant::now();
            lab.draw(&mut buf, 60.0);
            let without = t.elapsed().as_micros() as u64;
            lab.demo = seat;
            cost.push((with, without));
        }
        if gif.is_some() && (lab.world.frame - t0).is_multiple_of(every) {
            lab.draw(&mut buf, 60.0);
            shots.push(image::RgbaImage::from_raw(WIDTH, HEIGHT, buf.clone()).expect("frame size"));
        }
    }
    if !cost.is_empty() {
        let stat = |v: &mut Vec<u64>| {
            v.sort_unstable();
            (v.iter().sum::<u64>() / v.len() as u64, v[v.len() * 99 / 100], v[v.len() - 1])
        };
        let (mut a, mut b): (Vec<u64>, Vec<u64>) = cost.iter().copied().unzip();
        let ((am, a99, amax), (bm, b99, bmax)) = (stat(&mut a), stat(&mut b));
        println!("drawcost frames={} demo_us mean={am} p99={a99} max={amax}  plain_us mean={bm} p99={b99} max={bmax}", cost.len());
    }
    // `card=x.png`: the last frame with the season card up (Y, as the
    // player opens it; a death has already opened it).
    if let Some(path) = arg::<String>("card") {
        if !lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).is_some_and(|e| e.card) {
            lab.demo_key(DemoKey::Letter('Y'), true);
        }
        lab.draw(&mut buf, 60.0);
        image::RgbaImage::from_raw(WIDTH, HEIGHT, buf.clone()).expect("frame size").save(&path).expect("card png");
        println!("card {path}");
    }
    let e = lab.demo.as_ref().and_then(|d| d.enemy.clone()).expect("enemy seat");
    let t0 = landed_at.unwrap_or(lab.world.frame);
    let ran = lab.world.frame - t0;
    let per_min = if in_nest_frames > 0 { taken_in_nest as f64 * 3600.0 / in_nest_frames as f64 } else { 0.0 };
    println!(
        "RESULT seed={seed} brain={} policy={} ran={ran} end={} in_nest_frames={in_nest_frames} deepest={deepest} kills={} given={} taken={} taken_in_nest={taken_in_nest} taken_per_min_in_nest={per_min:.1} cells_lost={} ants_at_landing={} ants_end={} trips_since={}",
        if oldant { "hand-built" } else { "trained" },
        if skulk { "skulk" } else { "door" },
        e.ended.map_or("alive", |(_, why)| why),
        e.kills,
        e.bites_given,
        e.bites_taken,
        e.cells_lost.len(),
        e.ants_at_landing,
        ants(&lab.world).len(),
        lab.world.creature_stats.trip_deliveries - e.trips_at_landing,
    );
    if let Some(path) = gif {
        let delay = image::Delay::from_numer_denom_ms(every as u32 * 1000, 60);
        let frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
        let n = frames.len();
        let file = std::fs::File::create(&path).expect("gif file");
        let mut enc = image::codecs::gif::GifEncoder::new_with_speed(file, 10);
        enc.set_repeat(image::codecs::gif::Repeat::Infinite).expect("repeat");
        enc.encode_frames(frames).expect("gif encode");
        println!("gif {path} frames={n}");
    }
}
