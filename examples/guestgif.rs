//! **The guest seat's demo, played by a script, as a GIF and a counter
//! trace** (guest seat thread, 2026-10-25; stakes and the paired mode,
//! 2026-10-30).
//!
//! `demo=guest` is a game of keys, and the owner reads chat on a phone, so
//! the demo is shown rather than described: this runs the real seat
//! (`Lab::start_demo_with(DemoSeat::Guest, ..)`, the seat's own landing,
//! respawn and HUD) with a scripted player pressing the same keys through
//! `Lab::demo_key`, and draws every `every`-th frame with `Lab::draw` -- the
//! call the window makes -- into a GIF, as `labgif` does.
//!
//! ```text
//! cargo run --release --example guestgif -- script=good out=/tmp/good.gif frames=5400
//! cargo run --release --example guestgif -- script=beggar stakes=0 out=/tmp/beggar.gif   # the sandbox
//! cargo run --release --example guestgif -- scripts=good,spam,idle seeds=1-12          # the paired table
//! ```
//!
//! Scripts: `raw` walks to the door and down the shaft pressing nothing;
//! `beggar` does the same with soak on, rubs every 20 frames for the first
//! `rub_for` frames, then begs every 90; `predator` is `beggar` that also
//! presses B every 30 frames and E every 15 after `kill_at`. The stakes
//! scripts (2026-10-30), each laying an egg (L) whenever the HUD says
//! `L READY`: `good` soaks, rubs every 20 frames until the smell bar says
//! FAMILY and the WARY bar is under 0.2, and then begs at most every
//! `space` frames (default 180), rubbing between; `spam` soaks and rubs
//! until its rove first smells of family (what the first 20 s teach
//! everyone), then begs on every press (every 30 frames) and never rubs
//! again; `idle` presses nothing and does not move.
//!
//! **The paired mode** (`scripts=` and `seeds=`) warms each seed's colony
//! once, keeps a copy of the world and the seat just before the landing, and
//! plays every script from that copy, so the scripts on one seed meet the
//! same colony. One `RESULT` row per (seed, script), with the score.
//!
//! **Every second it prints a VISIT line** (gap, energy, cells, and the
//! ladder's counts), the counter beside the picture that `CLAUDE.md` asks
//! for: a GIF shows *what* the ants did; only the counts say how often.

use pixel_physics::lab::demo::{guest, Demo, DemoKey, DemoSeat};
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::steer;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

/// What a script presses: the walk key held, and the press counts.
#[derive(Default)]
struct Player {
    held: Option<DemoKey>,
    last_beg: Option<u64>,
    y_presses: u64,
    l_presses: u64,
    r_presses: u64,
    /// The spammer: the rove it last saw, and whether that rove has once
    /// smelled of family.
    rove: Option<pixel_physics::sim::cell::OrganismId>,
    disguised: bool,
    /// The head last frame, and frames it has not moved while walking.
    last_head: Option<(i32, i32)>,
    still: u32,
}

struct Knobs {
    rub_for: u64,
    kill_at: u64,
    space: u64,
    /// The stakes scripts lay an egg once the bank reaches this (J); the
    /// seat's own floor is `EGG_J + EGG_KEEP_J`.
    lay_at: f32,
}

fn seat(lab: &Lab) -> &guest::GuestDemo {
    lab.demo.as_ref().and_then(|d| d.guest.as_ref()).expect("the guest seat")
}

/// Press a seat letter, counting the ones the table reports.
fn press(lab: &mut Lab, c: char, p: &mut Player) {
    match c {
        'Y' => p.y_presses += 1,
        'L' => p.l_presses += 1,
        'R' => p.r_presses += 1,
        _ => {}
    }
    lab.demo_key(DemoKey::Letter(c), true);
}

/// One frame of `script` at frame `f` of the run.
fn drive(lab: &mut Lab, script: &str, f: u64, p: &mut Player, door: (i32, i32), k: &Knobs) {
    if script == "none" || script == "idle" {
        return;
    }
    let me = steer::steered_id(&lab.world).and_then(|id| lab.world.organism(id).map(|s| (id, s.energy, s.chain.first().copied())));
    // Walk to the door, then down.
    let want = me.and_then(|(_, _, h)| h).map(|(hx, hy)| {
        if (hx - door.0).abs() > 1 && hy <= door.1 {
            if hx < door.0 {
                DemoKey::Right
            } else {
                DemoKey::Left
            }
        } else {
            DemoKey::Down
        }
    });
    // The stakes scripts jump (J) when the walk to the door has been stuck
    // for two seconds, as a player would: on some seeds the mound's flank
    // stops the rove a column outside the nest (seed 3, x 273 against a
    // door at 256 and a reach of 16).
    let head = me.and_then(|m| m.2);
    if matches!(script, "good" | "spam") && matches!(want, Some(DemoKey::Left | DemoKey::Right)) && head == p.last_head {
        p.still += 1;
        if p.still >= 120 {
            p.still = 0;
            press(lab, 'J', p);
        }
    } else {
        p.still = 0;
    }
    p.last_head = head;
    if want != p.held {
        if let Some(key) = p.held {
            lab.demo_key(key, false);
        }
        if let Some(key) = want {
            lab.demo_key(key, true);
        }
        p.held = want;
    }
    if script == "raw" {
        return;
    }
    if f == 0 && !seat(lab).soak {
        press(lab, 'P', p);
    }
    let frame = lab.world.frame;
    let energy = me.map_or(0.0, |m| m.1);
    match script {
        "beggar" | "predator" => {
            if f < k.rub_for && f.is_multiple_of(20) {
                press(lab, 'R', p);
            }
            if f >= k.rub_for && f.is_multiple_of(90) {
                press(lab, 'Y', p);
            }
            if script == "predator" && f >= k.kill_at {
                if f.is_multiple_of(30) {
                    press(lab, 'B', p);
                }
                if f.is_multiple_of(15) {
                    press(lab, 'E', p);
                }
            }
        }
        "good" | "spam" => {
            if f % 60 == 30 && energy >= k.lay_at.max(guest::EGG_J + guest::EGG_KEEP_J) {
                press(lab, 'L', p);
            }
            let v = lab.world.visitors.as_ref();
            let g = me.and_then(|(id, _, _)| v.and_then(|v| v.list.iter().find(|x| x.id == id))).map_or(f32::NAN, |x| x.gap);
            let wary = v.and_then(|v| v.wary.as_ref()).map_or(0.0, |w| w.colony(frame));
            if script == "spam" {
                // Disguise once per rove (the first 20 s everyone learns),
                // then beg on every press and never rub again.
                let id = me.map(|m| m.0);
                if id != p.rove {
                    p.rove = id;
                    p.disguised = false;
                }
                p.disguised |= g.is_finite() && g < 0.45;
                if !p.disguised {
                    if f.is_multiple_of(20) {
                        press(lab, 'R', p);
                    }
                } else if f.is_multiple_of(30) {
                    press(lab, 'Y', p);
                }
                return;
            }
            if !f.is_multiple_of(20) {
                return;
            }
            let spaced = p.last_beg.is_none_or(|t| frame >= t + k.space);
            if g.is_finite() && g < 0.45 && wary < 0.2 && spaced {
                p.last_beg = Some(frame);
                press(lab, 'Y', p);
            } else {
                press(lab, 'R', p);
            }
        }
        _ => {}
    }
}

/// A fresh lab on `seed`, the demo armed with `land`, `gap` and `stakes`.
fn fresh(seed: u64, land: u64, gap: Option<f32>, stakes: &str) -> Lab {
    let mut sc = Scenario::load("demo_guest").expect("demo_guest ships");
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    let mut args = std::collections::BTreeMap::new();
    args.insert("land".to_string(), land.to_string());
    if let Some(g) = gap {
        args.insert("gap".to_string(), g.to_string());
    }
    args.insert("stakes".to_string(), stakes.to_string());
    if let Some(w) = arg::<String>("wary") {
        args.insert("wary".to_string(), w);
    }
    // `start_demo_with` without its by-name load, so `seed=` reaches the bed.
    let mut demo = Demo::new(DemoSeat::Guest);
    demo.args = args;
    lab.demo = Some(demo);
    lab.load_scenario(sc);
    lab
}

fn door(lab: &Lab) -> (i32, i32) {
    let nest = lab.world.nest_sites.iter().find(|n| n.seeded).expect("a nest");
    (nest.x, nest.surface)
}

fn result_row(lab: &Lab, script: &str, seed: u64, p: &Player, frames: u64) -> String {
    let c = lab.world.visitors.as_ref().map(|v| v.counts).unwrap_or_default();
    let st = seat(lab);
    let sc = guest::score_now(lab);
    let lost = st.lives.saturating_sub(u32::from(st.died_at.is_none() && st.body.is_some_and(|id| lab.world.organism(id).is_some())));
    let brood = st.eggs.iter().filter(|e| e.brood).count();
    format!(
        "RESULT script={script} seed={seed} stakes={} frames={frames} score={} alive_s={} egg_pts={} energy_pts={} eggs={} brood_eggs={brood} lives_used={} lives_lost={lost} ended={:?} y_presses={} begs={} fed={} fed_j={:.0} refused={} empty={} accept={:.2} per_press={:.2} nips={} cells_lost={} rubs={} r_presses={} l_presses={} grade={}",
        st.stakes,
        sc.total(),
        sc.alive_s,
        sc.egg_points,
        sc.energy_points,
        st.eggs.len(),
        st.lives,
        st.ended.map(|e| e.1),
        p.y_presses,
        c.begs,
        c.fed,
        c.fed_j,
        c.refused,
        c.empty,
        c.fed as f64 / (c.fed + c.refused).max(1) as f64,
        c.fed as f64 / p.y_presses.max(1) as f64,
        c.nipped,
        c.cells_lost,
        c.rubs,
        p.r_presses,
        p.l_presses,
        if st.stakes { guest::grade_now(lab) } else { guest::grade(&c) },
    )
}

fn paired(scripts: &str, seeds: &str, land: u64, gap: Option<f32>, stakes: &str, k: &Knobs) {
    let seeds: Vec<u64> = match seeds.split_once('-') {
        Some((a, b)) => (a.parse().expect("seeds=a-b")..=b.parse().expect("seeds=a-b")).collect(),
        None => seeds.split(',').map(|s| s.parse().expect("seeds=a,b,c")).collect(),
    };
    let scripts: Vec<&str> = scripts.split(',').collect();
    let len: u64 = arg("len").unwrap_or(guest::VISIT_FRAMES + 120);
    println!("paired: scripts={scripts:?} seeds={seeds:?} land={land} gap={gap:?} stakes={stakes} len={len} space={} lay_at={}", k.space, k.lay_at);
    for seed in seeds {
        let t0 = std::time::Instant::now();
        let mut lab = fresh(seed, land, gap, stakes);
        while lab.world.frame + 1 < land {
            lab.demo_frame();
            lab.tick_for_harness();
        }
        let (world, demo) = (lab.world.clone(), lab.demo.clone());
        println!("  seed {seed}: warm-up to {} in {:.1} s", lab.world.frame, t0.elapsed().as_secs_f32());
        for script in &scripts {
            let t1 = std::time::Instant::now();
            lab.world = world.clone();
            lab.demo = demo.clone();
            while steer::steered_id(&lab.world).is_none() {
                lab.demo_frame();
                lab.tick_for_harness();
                assert!(lab.world.frame < land + 600, "no rove landed by frame {}", lab.world.frame);
            }
            let d = door(&lab);
            let mut p = Player::default();
            let mut f = 0;
            while f < len {
                drive(&mut lab, script, f, &mut p, d, k);
                lab.demo_frame();
                lab.tick_for_harness();
                f += 1;
                if seat(&lab).ended.is_some() && seat(&lab).stakes {
                    break;
                }
            }
            println!("{}", result_row(&lab, script, seed, &p, f));
            println!("    ({:.1} s)", t1.elapsed().as_secs_f32());
        }
    }
}

fn main() {
    let land: u64 = arg("land").unwrap_or(30_000);
    let gap: Option<f32> = arg("gap");
    let stakes: String = arg("stakes").unwrap_or_else(|| "1".into());
    let k = Knobs { rub_for: arg("rub_for").unwrap_or(1_200), kill_at: arg("kill_at").unwrap_or(2_400), space: arg("space").unwrap_or(180), lay_at: arg("lay_at").unwrap_or(0.0) };
    if let (Some(scripts), Some(seeds)) = (arg::<String>("scripts"), arg::<String>("seeds")) {
        return paired(&scripts, &seeds, land, gap, &stakes, &k);
    }
    let script: String = arg("script").unwrap_or_else(|| "beggar".into());
    let seed: u64 = arg("seed").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(3_600);
    let every: u64 = arg("every").unwrap_or(6).max(1);
    let zoom: i32 = arg("zoom").unwrap_or(3);
    let out: String = arg("out").unwrap_or_else(|| "guest.gif".into());
    let gif: bool = arg::<u8>("gif").unwrap_or(1) == 1;
    // `png=f1,f2` saves those run frames as stills beside `out`.
    let pngs: Vec<u64> = arg::<String>("png").map(|s| s.split(',').filter_map(|v| v.parse().ok()).collect()).unwrap_or_default();
    assert!(matches!(script.as_str(), "raw" | "beggar" | "predator" | "none" | "good" | "spam" | "idle"), "script= is raw, beggar, predator, good, spam, idle or none");
    println!("guestgif: script={script} seed={seed} land={land} gap={gap:?} stakes={stakes} frames={frames} every={every} zoom={zoom} rub_for={} kill_at={} space={} out={out} gif={gif} png={pngs:?}", k.rub_for, k.kill_at, k.space);

    let mut lab = fresh(seed, land, gap, &stakes);
    let t0 = std::time::Instant::now();
    while steer::steered_id(&lab.world).is_none() {
        lab.demo_frame();
        lab.tick_for_harness();
        assert!(lab.world.frame < land + 600, "no rove landed by frame {}", lab.world.frame);
    }
    println!("  warm-up to frame {} took {:.1} s wall", lab.world.frame, t0.elapsed().as_secs_f32());
    lab.show_help = false;
    // The lab opens zoomed out (a stride), so step in until the zoom is the
    // one asked for, as the scroll wheel does.
    for _ in 0..16 {
        if lab.renderer.zoom >= zoom && lab.renderer.zoom_out_stride <= 1 {
            break;
        }
        lab.renderer.adjust_zoom(1);
    }
    println!("  view: zoom {} stride {}", lab.renderer.zoom, lab.renderer.zoom_out_stride);
    let start = lab.world.frame;
    let rove0 = steer::steered_id(&lab.world).expect("landed");
    let d = door(&lab);
    println!("  rove {rove0} landed at {:?}, nest door at {d:?}", lab.world.organism(rove0).and_then(|s| s.chain.first().copied()));
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    let mut p = Player::default();
    for f in 0..=frames.max(1) {
        drive(&mut lab, &script, f, &mut p, d, &k);
        lab.demo_frame();
        let want_gif = gif && frames > 0 && f.is_multiple_of(every);
        if want_gif || pngs.contains(&f) {
            let mut full = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
            lab.draw(&mut full, 60.0);
            if let Some(img) = image::RgbaImage::from_raw(WIDTH, HEIGHT, full) {
                if pngs.contains(&f) {
                    let path = format!("{}-f{f}.png", out.trim_end_matches(".gif"));
                    img.save(&path).expect("png writes");
                    println!("  wrote {path}");
                }
                if want_gif {
                    shots.push(img);
                }
            }
        }
        if f.is_multiple_of(60) {
            let v = lab.world.visitors.as_ref().expect("the ladder is on");
            let c = v.counts;
            let body = steer::steered_id(&lab.world).and_then(|id| lab.world.organism(id).map(|s| (id, s.energy, s.chain.len())));
            let g = body.and_then(|(id, _, _)| v.list.iter().find(|x| x.id == id)).map_or(f32::NAN, |x| x.gap);
            let wary = v.wary.as_ref().map_or(0.0, |w| w.colony(lab.world.frame));
            let st = seat(&lab);
            let head = body.and_then(|(id, _, _)| lab.world.organism(id)).and_then(|s| s.chain.first().copied());
            println!(
                "VISIT f={} t={}s head={:?} in_nest={} gap={g:.2} wary={wary:.2} body={:?} rove={} score={} eggs={} grade={} rolls={} ant={} disp={} nip={} lost={} deaths={} rubs={} soak={} begs={} fed={} fed_j={:.0} refused={} empty={} larvae={} ate_j={:.0}",
                lab.world.frame,
                f / 60,
                head,
                head.is_some_and(|h| guest::in_nest(&lab.world, h)),
                body.map(|(id, e, n)| (id, e as i32, n)),
                st.lives,
                guest::score_now(&lab).total(),
                st.eggs.len(),
                if st.stakes { guest::grade_now(&lab) } else { guest::grade(&c) },
                c.rolls,
                c.antennated,
                c.displayed,
                c.nipped,
                c.cells_lost,
                c.deaths,
                c.rubs,
                c.soak_frames,
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
    println!("{} from={start}", result_row(&lab, &script, seed, &p, frames));
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
