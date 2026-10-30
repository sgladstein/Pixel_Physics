//! **The queen seat demo, played by a script** (`demo=queen`, 2026-10-25).
//!
//! Opens the lab on the demo exactly as `cargo run --bin lab -- demo=queen`
//! does (`Lab::start_demo`), and plays it through the demo's own key layer
//! (`Lab::demo_key`, `Lab::demo_frame`), so what it measures is what a
//! person at the keys gets. The script is `queenprobe`'s `play=1` founder --
//! walk to the heap, eat until the crop stops filling, walk home and down,
//! hold L until the crop is spent -- plus the seat's own verbs on a fixed
//! schedule: a wild egg (P) every `wild_every=` frames once she has
//! `wild_from=` workers, every third one feral (P twice), and once a wild
//! line has `keep_at=` adults that beat the stock at digs, Y on it (at most
//! three times, 8,000 frames apart).
//!
//! Prints the demo's own panel every `print=` frames, and a `RESULT` line:
//! her eggs, wild and kept eggs, worker eggs, hold lapses and returns, the
//! frames from landing to 10 and 20 workers, and her lay intervals (p50 and
//! p90) -- the **M0c** numbers that set `queen::HOLD`.
//!
//! `gif=out.gif` writes the run as an animation, cropped to the nest and
//! upscaled (`crop=x,y,w,h up=3 every=400`); `stills=dir` writes full-frame
//! PNGs (HUD and ledger included) at `still_at=f1,f2,...`.
//!
//! `cargo run --release --example queendemo -- seed=1 frames=60000 gif=/tmp/q.gif`
use pixel_physics::lab::demo::{queen, DemoKey, DemoSeat};
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::Lab;
use pixel_physics::sim::steer::{self, SteerInput};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

const KNOWN: [&str; 23] = [
    "wise", "rest_before", "reclaim", "found", "seed", "frames", "print", "wild_every", "wild_from", "keep_at", "gif", "crop", "up", "every", "stills", "still_at", "hold", "play", "stakes", "first", "summer", "winter", "grow",
];
/// The demo's own args, passed through to `start_demo_with` as the command line would.
const DEMO_ARGS: [&str; 5] = ["stakes", "first", "summer", "winter", "grow"];

fn main() {
    for a in std::env::args().skip(1) {
        let k = a.split('=').next().unwrap_or("");
        assert!(KNOWN.contains(&k), "unknown argument {a} (known: {KNOWN:?})");
    }
    let seed: u64 = arg("seed").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(60_000);
    let print: u64 = arg("print").unwrap_or(5_000);
    let wild_every: u64 = arg("wild_every").unwrap_or(4_000);
    let wild_from: u32 = arg("wild_from").unwrap_or(4);
    let keep_at: u32 = arg("keep_at").unwrap_or(1);
    let hold: Option<u64> = arg("hold");
    // `play=0`: the keyboard lets go once her colony has workers (her genome
    // drives her, as above the steering cap) -- the lazy player.
    let play: u8 = arg("play").unwrap_or(1);
    // **`wise=1` (default with the stakes): the player knows winter is
    // coming.** From `rest_before=` frames before a winter until it ends she
    // stays home and lays only to keep her hold (its bar under a fifth), so
    // no brood hatches into the famine and the heap's last summer food is
    // left for the workers to store. `wise=0` is the old script, which eats
    // and lays flat out all year.
    //
    // **`wise=2`: she reins the workers in.** Once the colony has `found=`
    // workers she stops laying for the middle of each summer, so her hold
    // lapses and the workers breed on what they forage; `reclaim=` frames
    // before her rest she goes back to eating and laying, which takes the
    // hold back, and through the winter she lays only to keep it.
    let wise: u8 = arg("wise").unwrap_or(1);
    let rest_before: u64 = arg("rest_before").unwrap_or(3_000);
    let reclaim: u64 = arg("reclaim").unwrap_or(3_000);
    let found: u32 = arg("found").unwrap_or(8);
    let gif: Option<String> = arg("gif");
    let crop: Vec<u32> = arg::<String>("crop").unwrap_or_else(|| "196,120,144,90".into()).split(',').map(|v| v.parse().expect("crop=x,y,w,h")).collect();
    let up: u32 = arg("up").unwrap_or(3);
    let every: u64 = arg("every").unwrap_or(400);
    let stills: Option<String> = arg("stills");
    let still_at: Vec<u64> = arg::<String>("still_at").map(|s| s.split(',').map(|v| v.parse().expect("still_at=f1,f2")).collect()).unwrap_or_default();
    println!("queendemo seed={seed} frames={frames} wise={wise} rest_before={rest_before} wild_every={wild_every} wild_from={wild_from} keep_at={keep_at} hold={hold:?} play={play} hold_default={}", queen::HOLD);

    let mut sc = Scenario::load("demo_queen").expect("demo_queen ships");
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    let mut args = std::collections::BTreeMap::new();
    args.insert("seed".to_string(), seed.to_string());
    for k in DEMO_ARGS {
        if let Some(v) = arg::<String>(k) {
            args.insert(k.to_string(), v);
        }
    }
    println!("  demo args {args:?}");
    let msg = lab.start_demo_with(DemoSeat::Queen, args);
    assert_eq!(lab.world.seed, seed, "the demo did not take seed=");
    println!("  {msg}");
    let (w, h) = (pixel_physics::app::WIDTH, pixel_physics::app::HEIGHT);
    let food_x = 274;
    let ground_y = sc.bed.ground_y;
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    let mut buf = vec![0u8; (w * h * 4) as usize];

    let mut qid = None;
    let mut land = None;
    let (mut play_state, mut play_since, mut last_crop, mut stuck, mut last_head) = (0u8, 0u64, 0.0f32, 0u32, (i32::MIN, i32::MIN));
    let mut held = [false; 4];
    let mut wilds = 0u32;
    let mut last_wild = 0u64;
    let mut kept = 0u32;
    let mut last_keep = 0u64;
    let (mut at10, mut at20) = (None, None);
    lab.world.death_log = Some(Vec::new());
    let mut queen_died = None;
    // Workers alive the frame winter 2 ends (the deciding measure), whether or
    // not she is.
    let mut w2_end: Option<u32> = None;
    let (mut eggs_seen, mut last_egg) = (0u32, 0u64);
    for _ in 0..frames {
        lab.demo_frame();
        let frame = lab.world.frame;
        if qid.is_none() {
            if let Some(id) = steer::steered_id(&lab.world) {
                qid = Some(id);
                land = Some(frame);
                if let (Some(hd), Some(s)) = (hold, lab.world.steered.as_mut()) {
                    s.hold = Some(hd);
                }
                println!("  frame {frame}: she landed (id {id})");
            }
        }
        let tally = lab.demo.as_ref().and_then(|d| d.queen.as_ref()).map(|q| q.tally).unwrap_or_default();
        if tally.eggs > eggs_seen {
            eggs_seen = tally.eggs;
            last_egg = frame;
        }
        if let (Some(q), Some(l)) = (qid, land) {
            if let Some(st) = lab.world.organism(q).filter(|_| steer::is_steered(&lab.world, q)) {
                if play == 1 || tally.workers < 3 {
                    let (hx, hy) = *st.chain.first().expect("a head");
                    let crop = st.crop.map_or(0.0, |c| c.worth());
                    let door_x = lab.world.nest_sites.first().map_or(256, |n| n.x);
                    if (hx - last_head.0).abs().max((hy - last_head.1).abs()) < 2 {
                        stuck += 1;
                    } else {
                        stuck = 0;
                        last_head = (hx, hy);
                    }
                    let up = |below: bool| -> i8 { if below || stuck > 60 { -1 } else { 0 } };
                    let mut input = SteerInput::default();
                    // (rest, lay only to hold): see `wise=` above.
                    let (rest, hold_lay) = lab.demo.as_ref().and_then(|d| d.queen.as_ref()).filter(|q| wise > 0 && q.stakes && q.outcome.is_none()).map_or((false, false), |q| {
                        let t = frame.saturating_sub(q.start);
                        let s = q.seasons.at(t);
                        let endless = s.end == u64::MAX;
                        if s.winter || (!endless && t + rest_before >= s.end) {
                            (true, true)
                        } else if wise == 2 && tally.workers >= found && (endless || t + rest_before + reclaim < s.end) {
                            (true, false)
                        } else {
                            (false, false)
                        }
                    });
                    if rest {
                        let provisions = lab.world.materials.id_of("provisions");
                        let beside = (-1..=1).any(|ox| (-1..=1).any(|oy| Some(lab.world.get(hx + ox, hy + oy).material) == provisions));
                        if hy > ground_y + 2 {
                            // Home: eat what the workers stored if she is low, and lay only to hold.
                            input.eat = beside && st.energy < 1_500.0;
                            input.lay = hold_lay && steer::hold_left(&lab.world).is_some_and(|l| l < 0.2);
                            play_state = 3;
                            play_since = frame;
                        } else if (hx - door_x).abs() <= 1 {
                            input.dir = Some((0, 1));
                        } else {
                            input.dir = Some(((door_x - hx).signum() as i8, up(hy > ground_y - 1)));
                        }
                    }
                    if !rest {
                    match play_state {
                        0 => {
                            if (hx - food_x).abs() <= 12 && hy <= ground_y + 4 {
                                play_state = 1;
                                play_since = frame;
                            } else if hy > ground_y && (hx - door_x).abs() > 1 {
                                input.dir = Some(((door_x - hx).signum() as i8, up(false)));
                            } else if hy > ground_y {
                                input.dir = Some((0, -1));
                            } else {
                                input.dir = Some(((food_x - hx).signum() as i8, up(hy > ground_y - 1)));
                            }
                        }
                        1 => {
                            input.eat = true;
                            let provisions = lab.world.materials.id_of("provisions");
                            let beside = (-1..=1).any(|ox| (-1..=1).any(|oy| Some(lab.world.get(hx + ox, hy + oy).material) == provisions));
                            // **Eat from the heap's west flank, never its middle**
                            // (2026-10-30): the next heap lands on column `food_x`, and
                            // a queen standing there with a full crop was buried in
                            // provisions she could neither dig nor pick up -- the
                            // founding stall on seeds 2, 4, 15 and 16 (`QD_DEBUG`'s map).
                            if !beside && hx < food_x - 6 {
                                input.dir = Some(((food_x - hx).signum() as i8, up(false)));
                            } else if !beside {
                                input.dir = Some((-1, up(false)));
                            }
                            if crop > last_crop + 1e-3 {
                                play_since = frame;
                            }
                            if (crop > 0.0 && frame - play_since > 240) || frame - play_since > 3000 {
                                play_state = 2;
                            }
                        }
                        2 if crop <= 0.0 => play_state = 0,
                        2 => {
                            if (hx - door_x).abs() <= 1 {
                                input.dir = Some((0, 1));
                                if stuck > 30 {
                                    play_state = 3;
                                    play_since = frame;
                                }
                            } else {
                                input.dir = Some(((door_x - hx).signum() as i8, up(hy > ground_y - 1)));
                            }
                        }
                        _ => {
                            input.lay = true;
                            // **No egg for a while: cut a chamber.** The notice
                            // says "no room"; the player digs sideways, one
                            // side then the other, as a person would.
                            if frame > last_egg + 800 {
                                let side = if (frame / 400).is_multiple_of(2) { 1 } else { -1 };
                                input.dir = Some((side, 0));
                                input.dig = true;
                            }
                            if crop <= 0.0 && frame - play_since > 300 {
                                play_state = 0;
                            }
                        }
                    }
                    }
                    if let (true, Some((dx, dy))) = (matches!(play_state, 0 | 2) && stuck > 60, input.dir) {
                        let dirs = &pixel_physics::sim::creature::DIRS;
                        if let Some(i) = dirs.iter().position(|&d| d == (i32::from(dx), i32::from(dy))) {
                            let off = [0i32, 1, -1, 2, -2][((stuck / 60) % 5) as usize];
                            let (rx, ry) = dirs[(i as i32 + off).rem_euclid(8) as usize];
                            input.dir = Some((rx as i8, ry as i8));
                        }
                    }
                    if matches!(play_state, 0 | 2) && stuck > 30 {
                        input.dig = true;
                    }
                    if st.spoil.is_some() {
                        input.drop = true;
                    }
                    last_crop = crop;
                    if std::env::var("QD_DEBUG").is_ok() && frame.is_multiple_of(1000) {
                        let cs = &lab.world.creature_stats;
                        println!("  debug {frame}: state {play_state} head ({hx},{hy}) crop {crop:.0} energy {:.0} stuck {stuck} door {door_x} input {input:?}", st.energy);
                        for y in hy - 4..=hy + 3 {
                            let row: String = (hx - 10..=hx + 10)
                                .map(|x| {
                                    let c = lab.world.get(x, y);
                                    if (x, y) == (hx, hy) {
                                        '@'
                                    } else if c.organism_id() != 0 {
                                        'a'
                                    } else if c.is_empty() {
                                        '.'
                                    } else {
                                        lab.world.materials.get(c.material).name.chars().next().unwrap_or('?')
                                    }
                                })
                                .collect();
                            println!("      {y}: {row}");
                        }
                        println!("    held_for_nest {} declined {} food_brake {} eggs_laid {} brood_held {} hold_left {:?}", cs.buds_held_for_nest, cs.lays_declined, cs.food_brake_held, cs.eggs_laid, cs.brood_held, steer::hold_left(&lab.world));
                    }
                    // **Through the demo's key layer**: arrows as levels,
                    // letters as presses, as the window sends them.
                    let (dx, dy) = input.dir.unwrap_or((0, 0));
                    let want = [dx < 0, dx > 0, dy < 0, dy > 0];
                    for (i, k) in [DemoKey::Left, DemoKey::Right, DemoKey::Up, DemoKey::Down].into_iter().enumerate() {
                        if want[i] != held[i] {
                            lab.demo_key(k, want[i]);
                            held[i] = want[i];
                        }
                    }
                    for (on, c) in [(input.eat, 'E'), (input.dig, 'G'), (input.lay, 'L'), (input.drop, 'Q')] {
                        if on {
                            lab.demo_key(DemoKey::Letter(c), true);
                        }
                    }
                } else if held.iter().any(|&h| h) {
                    for (i, k) in [DemoKey::Left, DemoKey::Right, DemoKey::Up, DemoKey::Down].into_iter().enumerate() {
                        lab.demo_key(k, false);
                        held[i] = false;
                    }
                }
            } else if queen_died.is_none() && lab.world.organism(q).is_none() {
                queen_died = Some(frame - l);
                let cause = lab.world.death_log.as_ref().and_then(|log| log.iter().rev().find(|r| r.id == q)).map(|r| r.cause);
                println!("  frame {frame}: SHE DIED ({cause:?})");
            }
            // The seat's verbs on a schedule.
            if tally.workers >= wild_from && frame >= last_wild + wild_every && wilds < 9 {
                lab.demo_key(DemoKey::Letter('P'), true);
                if wilds % 3 == 2 {
                    lab.demo_key(DemoKey::Letter('P'), true);
                }
                wilds += 1;
                last_wild = frame;
            }
            if kept < 3 && frame >= last_keep + 8_000 && frame.is_multiple_of(500) {
                if let Some(q) = lab.demo.as_ref().and_then(|d| d.queen.as_ref()) {
                    let stock = q.lines[0].frames.max(1) as f64;
                    let stock_rate = q.lines[0].digs as f64 * 1000.0 / stock;
                    let best = q
                        .lines
                        .iter()
                        .enumerate()
                        .skip(1)
                        .filter(|(_, l)| !l.gone && l.adults >= keep_at && l.frames > 0)
                        .map(|(i, l)| (i, l.digs as f64 * 1000.0 / l.frames as f64))
                        .filter(|&(_, r)| r > stock_rate)
                        .max_by(|a, b| a.1.total_cmp(&b.1));
                    if let Some((i, r)) = best {
                        let rows: Vec<usize> = (0..q.lines.len()).filter(|&j| j == 0 || !q.lines[j].gone).collect();
                        let target = rows.iter().position(|&j| j == i).unwrap_or(0);
                        let name = q.lines[i].name.clone();
                        let cur = q.cursor;
                        let n = rows.len();
                        for _ in 0..(target + n - cur) % n {
                            lab.demo_key(DemoKey::Letter('K'), true);
                        }
                        lab.demo_key(DemoKey::Letter('Y'), true);
                        kept += 1;
                        last_keep = frame;
                        println!("  frame {frame}: Y on {name} (digs {r:.2}/1k vs stock {stock_rate:.2})");
                    }
                }
            }
            let workers = tally.workers;
            if at10.is_none() && workers >= 10 {
                at10 = Some(frame - l);
            }
            if at20.is_none() && workers >= 20 {
                at20 = Some(frame - l);
            }
        }
        lab.tick_for_harness();
        let frame = lab.world.frame;
        if let Some(q) = lab.demo.as_ref().and_then(|d| d.queen.as_ref()).filter(|q| q.ready) {
            let w2 = q.seasons.list().into_iter().find(|s| s.winter && s.n == 2).map_or(u64::MAX, |s| q.start + s.end);
            if w2_end.is_none() && frame >= w2 + 50 {
                w2_end = Some(q.tally.workers);
            }
        }
        if land.is_some() && (gif.is_some() && frame.is_multiple_of(every) || still_at.contains(&frame)) {
            lab.draw(&mut buf, 60.0);
            let img = image::RgbaImage::from_raw(w, h, buf.clone()).expect("frame size");
            if still_at.contains(&frame) {
                if let Some(dir) = &stills {
                    std::fs::create_dir_all(dir).ok();
                    let p = format!("{dir}/queen_s{seed}_f{frame}.png");
                    img.save(&p).expect("write still");
                    println!("  wrote {p}");
                }
            }
            if gif.is_some() && frame.is_multiple_of(every) {
                let c = image::imageops::crop_imm(&img, crop[0], crop[1], crop[2], crop[3]).to_image();
                shots.push(image::imageops::resize(&c, crop[2] * up, crop[3] * up, image::imageops::FilterType::Nearest));
            }
        }
        if frame.is_multiple_of(print) {
            // Deaths since the last print, by cause: what a winter actually kills.
            let mut causes: std::collections::BTreeMap<String, u32> = Default::default();
            if let Some(log) = lab.world.death_log.as_mut() {
                for r in log.drain(..).filter(|r| r.creature) {
                    *causes.entry(format!("{:?}", r.cause)).or_default() += 1;
                }
            }
            println!("  frame {frame}: deaths {causes:?}");
            for (t, _) in queen::panel(&lab) {
                println!("    {t}");
            }
        }
    }
    let t = lab.demo.as_ref().and_then(|d| d.queen.as_ref()).map(|q| q.tally).unwrap_or_default();
    // Her lay intervals, from the store's log of her own eggs.
    let mut gaps: Vec<u64> = Vec::new();
    if let (Some(f), Some(q)) = (lab.world.fathers.as_ref(), qid) {
        let mine: Vec<u64> = f.log.iter().filter(|e| e.layer == q).map(|e| e.frame).collect();
        gaps = mine.windows(2).map(|p| p[1] - p[0]).collect();
    }
    gaps.sort_unstable();
    let pct = |p: f64| gaps.get(((gaps.len() as f64 - 1.0) * p).round() as usize).copied().map_or(-1, |v| v as i64);
    let lines = lab.demo.as_ref().and_then(|d| d.queen.as_ref()).map_or(0, |q| q.lines.len() - 1);
    let (score, outcome) = lab.demo.as_ref().and_then(|d| d.queen.as_ref()).map_or((String::new(), String::new()), |q| {
        (q.winters.iter().map(|w| w.to_string()).collect::<Vec<_>>().join("/"), format!("{:?}", q.outcome).replace(' ', ""))
    });
    println!(
        "RESULT seed={seed} play={play} w2_end={} score={score} outcome={outcome} eggs={} wild_eggs={} kept_eggs={} worker_eggs={} lapses={} returns={} workers={} brood={} hungry_larvae={} wild_lines={lines} at10={} at20={} lay_gaps={} gap_p50={} gap_p90={} gap_max={} queen_died={}",
        w2_end.map_or(-1, |v| v as i64),
        t.eggs,
        t.wild_eggs,
        t.kept_eggs,
        t.worker_eggs,
        t.lapses,
        t.returns,
        t.workers,
        t.brood,
        t.hungry_larvae,
        at10.map_or(-1, |v| v as i64),
        at20.map_or(-1, |v| v as i64),
        gaps.len(),
        pct(0.5),
        pct(0.9),
        gaps.last().map_or(-1, |&v| v as i64),
        queen_died.map_or(-1, |v| v as i64),
    );
    if let Some(out) = gif {
        let frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, image::Delay::from_numer_denom_ms(120, 1))).collect();
        let n = frames.len();
        let file = std::fs::File::create(&out).expect("create gif");
        let mut enc = image::codecs::gif::GifEncoder::new(file);
        enc.set_repeat(image::codecs::gif::Repeat::Infinite).ok();
        enc.encode_frames(frames).expect("encode gif");
        println!("  wrote {out} ({n} frames)");
    }
}
