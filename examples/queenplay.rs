//! **The play-as-an-ant game's queen mode, played by a script through the
//! keys, and filmed through the real screen** (`lab::queen`,
//! `Reports/play-as-ant-slice1-2026-10-11.md`).
//!
//! Opens `queen_start` exactly as `cargo run --bin lab -- scenario=queen_start`
//! does, then drives the queen only through `Lab::queen_key` -- the call the
//! window's key handler makes -- with `queenprobe`'s scripted player turned
//! into key-downs and key-ups: out of the founding chamber, east to the food
//! spot, eat at its edge, home down the shaft, lay, repeat. Every `every=`
//! frames it draws `Lab::draw` (the panel, the camera and the bar, the same
//! call the window makes) into a GIF, so the card shows what a player would
//! see. It is a floor on a person at the keys, not a model of one.
//!
//! Prints her landing, every trip, eggs, and the goal frame, and at the end a
//! `RESULT` line with her colony's census.
//!
//! ```text
//! cargo run --release --example queenplay -- frames=30000 every=120 out=/tmp/queen.gif
//! cargo run --release --example queenplay -- seed=3 frames=60000 gif=0
//! ```
//!
//! Args: `seed=1`, `frames=30000`, `every=120` (frames between GIF frames),
//! `out=queenplay.gif`, `gif=1`, `speed=1` (the dial, which the cap reads),
//! `still=F` (also write the screen at frame F as a PNG beside `out`),
//! `trail=1` (hold T on every walk home, so she lays her food trail),
//! `delay=N` (idle N frames before the script starts: the one-frame nudged
//! baseline the nudge rule asks for, `Reports/how-we-test.md` section 4).
//!
//! The `RESULT` line also counts `at_food`, the distinct workers of her colony
//! ever inside the food spot's reach (12 columns, at most 4 rows under the
//! ground line), and `first_food`, the frame the first one got there: the
//! trail's own metric, for `trail=1` against `trail=0` paired by seed.

use pixel_physics::lab::queen::{self, Key};
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::Lab;
use pixel_physics::sim::steer::SteerInput;

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{name}=")).and_then(|v| v.parse().ok()))
}

/// The food spot's column and reach, as `queen_start.ron` places it.
const FOOD_X: i32 = 316;
const FOOD_REACH: i32 = 12;

/// Press and release whatever differs between what the script wants and
/// what is down, so the lab sees key edges the way a keyboard makes them.
fn send(lab: &mut Lab, down: &mut SteerInput, want: SteerInput) {
    let dir = |d: Option<(i8, i8)>| d.unwrap_or((0, 0));
    let (was, now) = (dir(down.dir), dir(want.dir));
    let axes = [
        (Key::Left, was.0 < 0, now.0 < 0),
        (Key::Right, was.0 > 0, now.0 > 0),
        (Key::Up, was.1 < 0, now.1 < 0),
        (Key::Down, was.1 > 0, now.1 > 0),
        (Key::Eat, down.eat, want.eat),
        (Key::Drop, down.drop, want.drop),
        (Key::Share, down.share, want.share),
        (Key::Dig, down.dig, want.dig),
        (Key::Bite, down.bite, want.bite),
        (Key::Lay, down.lay, want.lay),
        (Key::Trail, down.trail, want.trail),
    ];
    for (key, before, after) in axes {
        if before != after {
            lab.queen_key(key, after);
        }
    }
    *down = want;
}

fn main() {
    let seed: u64 = arg("seed").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(30_000);
    let every: u64 = arg("every").unwrap_or(120);
    let out: String = arg("out").unwrap_or_else(|| "queenplay.gif".into());
    let gif = arg::<u8>("gif").unwrap_or(1) == 1;
    let speed: u32 = arg("speed").unwrap_or(1);
    let still: Option<u64> = arg("still");
    let trail = arg::<u8>("trail").unwrap_or(0) == 1;
    let delay: u64 = arg("delay").unwrap_or(0);
    println!(
        "queenplay: seed={seed} frames={frames} every={every} out={out} gif={gif} speed={speed} still={still:?} trail={trail} delay={delay}"
    );

    let mut sc = Scenario::load("queen_start").unwrap_or_else(|e| {
        eprintln!("queen_start: {e}");
        std::process::exit(1);
    });
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    println!("  {}", lab.load_scenario(sc));
    lab.show_help = false;
    lab.time.requested = speed;
    let ground_y = lab.spec.ground_y;
    let (qid, colony) = lab
        .world
        .steered
        .as_ref()
        .map(|s| (s.id, s.colony))
        .expect("queen_start marks a queen");
    let colony_x = lab
        .world
        .organism(qid)
        .and_then(|s| s.chain.first().copied())
        .map_or(256, |h| h.0);

    let mut shots: Vec<image::RgbaImage> = Vec::new();
    let mut down = SteerInput::default();
    let (mut state, mut since, mut stuck, mut trips) = (0u8, 0u64, 0u32, 0u32);
    let mut last_head = (i32::MIN, i32::MIN);
    let mut last_crop = 0.0f32;
    let mut at_food: std::collections::HashSet<pixel_physics::sim::cell::OrganismId> = Default::default();
    let mut first_food: Option<u64> = None;
    let species = lab.world.organism(qid).map(|s| s.species);
    for tick in 0..frames {
        lab.tick_for_harness();
        let frame = lab.world.frame;
        for id in lab.world.live_organism_ids() {
            let Some(s) = lab.world.organism(id) else { continue };
            if id == qid || s.colony != colony || Some(s.species) != species {
                continue;
            }
            if let Some(&(x, y)) = s.chain.first() {
                if (x - FOOD_X).abs() <= FOOD_REACH && y <= ground_y + 4 && at_food.insert(id) && first_food.is_none() {
                    first_food = Some(frame);
                }
            }
        }
        let mut want = SteerInput::default();
        if let Some(st) = lab
            .world
            .organism(qid)
            .filter(|_| lab.queen.died.is_none() && tick >= delay)
        {
            let (hx, hy) = *st.chain.first().expect("a head");
            let crop = st.crop.map_or(0.0, |c| c.worth());
            let door_x = lab.world.nest_sites.first().map_or(colony_x, |n| n.x);
            if (hx - last_head.0).abs().max((hy - last_head.1).abs()) < 2 {
                stuck += 1;
            } else {
                stuck = 0;
                last_head = (hx, hy);
            }
            let up = |below: bool| -> i8 {
                if below || stuck > 60 {
                    -1
                } else {
                    0
                }
            };
            match state {
                0 => {
                    if (hx - FOOD_X).abs() <= FOOD_REACH && hy <= ground_y + 4 {
                        state = 1;
                        since = frame;
                    } else if hy > ground_y && (hx - door_x).abs() > 1 {
                        want.dir = Some(((door_x - hx).signum() as i8, up(false)));
                    } else if hy > ground_y {
                        want.dir = Some((0, -1));
                    } else {
                        want.dir = Some(((FOOD_X - hx).signum() as i8, up(hy > ground_y - 1)));
                    }
                }
                1 => {
                    want.eat = true;
                    let food = lab.world.materials.id_of("provisions");
                    let beside =
                        (-1..=1).any(|ox| (-1..=1).any(|oy| Some(lab.world.get(hx + ox, hy + oy).material) == food));
                    if !beside {
                        want.dir = Some(((FOOD_X - hx).signum() as i8, up(false)));
                    }
                    if crop > last_crop + 1e-3 {
                        since = frame;
                    }
                    if (crop > 0.0 && frame - since > 240) || frame - since > 3000 {
                        state = 2;
                        trips += 1;
                        println!(
                            "  frame {frame}: trip {trips} home with {crop:.0} J in the crop, bank {:.0} J",
                            st.energy
                        );
                    }
                }
                2 if crop <= 0.0 => state = 0,
                2 => {
                    want.trail = trail;
                    if (hx - door_x).abs() <= 1 {
                        want.dir = Some((0, 1));
                        if stuck > 30 {
                            state = 3;
                            since = frame;
                        }
                    } else {
                        want.dir = Some(((door_x - hx).signum() as i8, up(hy > ground_y - 1)));
                    }
                }
                _ => {
                    want.lay = true;
                    if crop <= 0.0 && frame - since > 300 {
                        state = 0;
                    }
                }
            }
            if let (true, Some((dx, dy))) = (matches!(state, 0 | 2) && stuck > 60, want.dir) {
                let dirs = &pixel_physics::sim::creature::DIRS;
                if let Some(i) = dirs.iter().position(|&d| d == (i32::from(dx), i32::from(dy))) {
                    let off = [0i32, 1, -1, 2, -2][((stuck / 60) % 5) as usize];
                    let (rx, ry) = dirs[(i as i32 + off).rem_euclid(8) as usize];
                    want.dir = Some((rx as i8, ry as i8));
                }
            }
            if matches!(state, 0 | 2) && stuck > 30 {
                want.dig = true;
            }
            want.drop = st.spoil.is_some();
            last_crop = crop;
        }
        send(&mut lab, &mut down, want);
        let shoot = gif && frame.is_multiple_of(every);
        if shoot || still == Some(frame) {
            let (w, h) = lab.viewport();
            let mut buf = vec![0u8; (w * h * 4) as usize];
            lab.draw(&mut buf, 60.0);
            if std::env::var("QP_DEBUG").is_ok() {
                if let Some(st) = lab.world.organism(qid) {
                    for &(x, y) in &st.chain {
                        let c = lab.world.get(x, y);
                        println!(
                            "  debug: body ({x},{y}) id {} type {:?} on screen {:?}",
                            c.organism_id(),
                            pixel_physics::sim::organism::cell_type(c.aux()),
                            lab.renderer.world_to_screen(x, y)
                        );
                    }
                }
            }
            if let Some(img) = image::RgbaImage::from_raw(w, h, buf) {
                if still == Some(frame) {
                    let path = format!("{}-{frame}.png", out.trim_end_matches(".gif"));
                    match img.save(&path) {
                        Ok(()) => println!("  wrote {path}"),
                        Err(e) => eprintln!("  still {path}: {e}"),
                    }
                }
                if shoot {
                    shots.push(img);
                }
            }
        }
    }
    let c = queen::census(&lab.world, colony, qid);
    println!(
        "RESULT seed={seed} frames={frames} trail={trail} delay={delay} trips={trips} landed={:?} died={:?} goal_met={:?} workers={} eggs={} larvae={} pupae={} bank={:.0} at_food={} first_food={:?}",
        lab.queen.landed.map(|l| l.0),
        lab.queen.died,
        lab.queen.goal_met,
        c.workers,
        c.eggs,
        c.larvae,
        c.pupae,
        lab.world.organism(qid).map_or(0.0, |s| s.energy),
        at_food.len(),
        first_food
    );
    if gif && !shots.is_empty() {
        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(80));
        let n = shots.len();
        let gif_frames: Vec<image::Frame> = shots
            .into_iter()
            .map(|img| image::Frame::from_parts(img, 0, 0, delay))
            .collect();
        match std::fs::File::create(&out) {
            Ok(file) => {
                let mut encoder = image::codecs::gif::GifEncoder::new(file);
                let _ = encoder.set_repeat(image::codecs::gif::Repeat::Infinite);
                match encoder.encode_frames(gif_frames) {
                    Ok(()) => println!("  wrote {out} ({n} frames, 80 ms each)"),
                    Err(e) => eprintln!("  gif {out}: {e}"),
                }
            }
            Err(e) => eprintln!("  gif {out}: {e}"),
        }
    }
}
