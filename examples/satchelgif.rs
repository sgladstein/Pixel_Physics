//! **The satchel seat, played by a script, as a GIF and a counter trace**
//! (2026-10-31).
//!
//! Runs the real seat (`Lab::start_demo_with(DemoSeat::Satchel, ..)`: its
//! warm-up, spiders, favour and HUD), with a scripted hand playing cards
//! through `satchel::play_at` -- the call a click makes -- and draws every
//! `every`-th frame with `Lab::draw`, the window's own call, into a GIF.
//!
//! ```text
//! cargo run --release --example satchelgif -- script=trap out=/tmp/trap.gif
//! cargo run --release --example satchelgif -- scripts=trap,burn,idle seeds=1-6 gif=0   # the paired table
//! ```
//!
//! Scripts: `idle` plays nothing. `burn` plays the next card onto each
//! spider once it is within `reach` columns of the nest door. `trap` is the
//! designed combo: as each spider lands it arms the satchel's `TRAP`
//! on the nearest spider, then plays the next card onto
//! the spider when it comes within `reach`; the spider's death sets the
//! `TRAP` off, and the deaths that fire causes carry the chain on.
//!
//! **Every second it prints a COUNT line** (favour, ants, chains, deepest
//! depth, puffs, card kills against natural deaths, spiders in and killed):
//! the counter beside the picture that `CLAUDE.md` asks for. One `RESULT`
//! row per (seed, script) at the end, the same seed meeting every script.

use pixel_physics::lab::demo::{satchel, DemoSeat, MouseAct};
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

fn seat(lab: &Lab) -> &satchel::SatchelDemo {
    lab.demo.as_ref().and_then(|d| d.satchel.as_ref()).expect("the satchel seat")
}

fn door(lab: &Lab) -> (i32, i32) {
    lab.world.nest_sites.iter().find(|n| n.seeded).or(lab.world.nest_sites.first()).map_or((256, 160), |n| (n.x, n.surface))
}

/// Every live spider's head.
fn spiders(lab: &Lab) -> Vec<(i32, i32)> {
    let Some(sp) = lab.world.species.id_of(seat(lab).rules.raider) else { return Vec::new() };
    lab.world
        .live_organism_ids()
        .into_iter()
        .filter_map(|id| lab.world.organism(id).filter(|s| s.species == sp).and_then(|s| s.chain.first().copied()))
        .collect()
}

#[derive(Default)]
struct Player {
    armed_for: u32,
    plays: u32,
}

fn drive(lab: &mut Lab, script: &str, p: &mut Player, reach: i32) {
    if script == "idle" || seat(lab).ended.is_some() {
        return;
    }
    let d = door(lab);
    let s = seat(lab);
    let in_now = s.counts.raiders_in;
    let ready = lab.world.frame >= s.ready_at;
    let next_is_wake = s.deck.get(s.pos) == Some(&satchel::Card::Trap);
    let nearest = spiders(lab).into_iter().min_by_key(|h| (h.0 - d.0).abs());
    if script != "greedy" && nearest.is_none() {
        return;
    }
    let spider = nearest.unwrap_or(d);
    if script == "greedy" {
        // Push your luck at home: arm the TRAP on the nest door, then play
        // everything after it into the crowd there.
        if ready {
            let f = lab.world.frame;
            println!("  PLAY f={f} {}", satchel::play_at(lab, d));
            p.plays += 1;
        }
        return;
    }
    if script == "trap" && next_is_wake && ready && p.armed_for < in_now {
        let f = lab.world.frame;
        println!("  PLAY f={f} {}", satchel::play_at(lab, spider));
        p.armed_for = in_now;
        p.plays += 1;
        return;
    }
    if ready && (spider.0 - d.0).abs() <= reach && !(script == "trap" && next_is_wake) {
        let f = lab.world.frame;
        println!("  PLAY f={f} {}", satchel::play_at(lab, spider));
        p.plays += 1;
    }
}

fn row(lab: &Lab, script: &str, seed: u64, p: &Player) -> String {
    let s = seat(lab);
    let c = s.counts;
    let ants = satchel::live_of(&lab.world, "ant");
    format!(
        "RESULT seed={seed} script={script} frame={} round={} ended={} stars={} ants={ants} at_open={} min_ants={} clicks={} plays={} refused={} effects={} chains={} deepest={} puffs={} card_kills={} natural={} raiders_in={} raiders_killed={} raiders_by_cards={} score={} waves={} hatched={} burnt={} soaked={}",
        lab.world.frame,
        s.round_no,
        s.ended.map_or("no".to_string(), |e| if e.won { "won".to_string() } else { "lost".to_string() }),
        s.ended.map_or("-".to_string(), |e| e.stars.iter().map(|&b| if b { '*' } else { '.' }).collect::<String>()),
        s.ants_at_open,
        c.min_ants,
        p.plays,
        c.plays,
        c.refused,
        c.effects,
        c.chains,
        c.deepest,
        c.puffs,
        c.card_kills,
        c.natural,
        c.raiders_in,
        c.raiders_killed,
        c.raiders_by_cards,
        c.score,
        c.waves,
        c.hatched,
        c.cells_burnt,
        c.cells_soaked,
    )
}

fn seeds() -> Vec<u64> {
    let s: String = arg("seeds").unwrap_or_else(|| arg::<u64>("seed").unwrap_or(1).to_string());
    match s.split_once('-') {
        Some((a, b)) => (a.parse().unwrap_or(1)..=b.parse().unwrap_or(1)).collect(),
        None => s.split(',').filter_map(|x| x.parse().ok()).collect(),
    }
}

fn main() {
    let scripts: String = arg("scripts").or_else(|| arg("script")).unwrap_or_else(|| "trap".into());
    let frames: u64 = arg("frames").unwrap_or(18_000);
    let every: u64 = arg("every").unwrap_or(6);
    let gif_from: u64 = arg("gif_from").unwrap_or(0);
    let gif_len: u64 = arg("gif_len").unwrap_or(3_600);
    let reach: i32 = arg("reach").unwrap_or(120);
    let zoom: i32 = arg("zoom").unwrap_or(2);
    let warmup: u64 = arg("warmup").unwrap_or(9_000);
    let gif = arg::<u8>("gif").unwrap_or(1) == 1;
    let out: String = arg("out").unwrap_or_else(|| "satchel.gif".into());
    // `png_at=F,F,...`: a still of the whole window (HUD included) at each
    // of these frames of the run, beside the GIF. `playon=1`: after a round
    // ends, click once to play on, as a player would, instead of stopping.
    let png_at: Vec<u64> = arg::<String>("png_at").map(|v| v.split(',').filter_map(|x| x.parse().ok()).collect()).unwrap_or_default();
    let playon = arg::<u8>("playon").unwrap_or(0) == 1;
    let satchel_name: String = arg("satchel").unwrap_or_else(|| "WILDFIRE".into());
    let raider: String = arg("raider").unwrap_or_else(|| "spider".into());
    println!("satchelgif: scripts={scripts} seeds={:?} frames={frames} every={every} gif_from={gif_from} gif_len={gif_len} reach={reach} zoom={zoom} warmup={warmup} satchel={satchel_name} raider={raider} gif={gif} out={out} png_at={png_at:?} playon={playon}", seeds());
    for seed in seeds() {
        for script in scripts.split(',') {
            let mut lab = Lab::new(Default::default());
            let mut args = std::collections::BTreeMap::new();
            args.insert("seed".to_string(), seed.to_string());
            args.insert("warmup".to_string(), warmup.to_string());
            args.insert("satchel".to_string(), satchel_name.clone());
            args.insert("raider".to_string(), raider.clone());
            // The seat's own rules, passed through when given.
            for k in ["raider_every", "wave_cap", "round", "lose_at", "favour"] {
                if let Some(v) = arg::<String>(k) {
                    args.insert(k.to_string(), v);
                }
            }
            let t0 = std::time::Instant::now();
            lab.start_demo_with(DemoSeat::Satchel, args);
            lab.show_help = false;
            println!("  seed {seed} {script}: warm-up to frame {} took {:.1} s wall, {} ants, box seed {}, fingerprint {:x}", lab.world.frame, t0.elapsed().as_secs_f32(), seat(&lab).ants_at_open, lab.spec.seed, pixel_physics::lab::demo::keeper::fingerprint(&lab.world));
            for _ in 0..16 {
                if lab.renderer.zoom >= zoom && lab.renderer.zoom_out_stride <= 1 {
                    break;
                }
                lab.renderer.adjust_zoom(1);
            }
            let bounds = lab.world.bounds();
            lab.renderer.follow(door(&lab), (WIDTH, HEIGHT), bounds);
            let mut p = Player::default();
            let mut shots: Vec<image::RgbaImage> = Vec::new();
            let mut worst = 0f32;
            let mut total = 0f64;
            for f in 0..frames {
                drive(&mut lab, script, &mut p, reach);
                if png_at.contains(&f) {
                    let mut full = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
                    lab.draw(&mut full, 60.0);
                    let path = out.replace(".gif", &format!("-s{seed}-{script}-f{f}.png"));
                    if let Some(img) = image::RgbaImage::from_raw(WIDTH, HEIGHT, full) {
                        img.save(&path).expect("png_at= is writable");
                        println!("  STILL f={f} round={} notice={:?} wrote {path}", seat(&lab).round_no, lab.ui.notice_text());
                    }
                }
                if playon && seat(&lab).ended.is_some_and(|e| lab.world.frame == e.frame + satchel::NEXT_ROUND_AFTER) {
                    let e = seat(&lab).ended.expect("ended");
                    println!("{} (round over, playing on)", row(&lab, script, seed, &p));
                    let d = door(&lab);
                    satchel::mouse(&mut lab, d, MouseAct::Press { erase: false });
                    println!("  PLAYON f={f} was {} stars={} now round={} notice={:?}", if e.won { "won" } else { "lost" }, e.star_count(), seat(&lab).round_no, lab.ui.notice_text());
                }
                if gif && f >= gif_from && f < gif_from + gif_len && f.is_multiple_of(every) {
                    let mut full = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
                    lab.draw(&mut full, 60.0);
                    if let Some(img) = image::RgbaImage::from_raw(WIDTH, HEIGHT, full) {
                        shots.push(img);
                    }
                }
                let t = std::time::Instant::now();
                lab.tick_for_harness();
                let ms = t.elapsed().as_secs_f32() * 1000.0;
                worst = worst.max(ms);
                total += f64::from(ms);
                if f.is_multiple_of(60) {
                    let s = seat(&lab);
                    let c = s.counts;
                    println!(
                        "COUNT t={}s favour={} ants={} chains={} deepest={} puffs={} effects={} card_kills={} natural={} spiders_in={} spiders_killed={} queued={} armed={}",
                        f / 60,
                        s.favour,
                        satchel::live_of(&lab.world, "ant"),
                        c.chains,
                        c.deepest,
                        c.puffs,
                        c.effects,
                        c.card_kills,
                        c.natural,
                        c.raiders_in,
                        c.raiders_killed,
                        s.queue.len(),
                        s.armed.len()
                    );
                }
                if !playon && seat(&lab).ended.is_some_and(|e| lab.world.frame > e.frame + 120) {
                    break;
                }
            }
            println!("{} worst_tick_ms={worst:.1} mean_tick_ms={:.2}", row(&lab, script, seed, &p), total / (lab.world.frame.saturating_sub(warmup).max(1)) as f64);
            if shots.is_empty() {
                continue;
            }
            let path = if scripts.contains(',') || seeds().len() > 1 { out.replace(".gif", &format!("-s{seed}-{script}.gif")) } else { out.clone() };
            let delay = image::Delay::from_numer_denom_ms((every as u32 * 1000) / 60, 1);
            let n = shots.len();
            let gif_frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
            let file = std::fs::File::create(&path).expect("out= is writable");
            let mut encoder = image::codecs::gif::GifEncoder::new(file);
            encoder.set_repeat(image::codecs::gif::Repeat::Infinite).expect("repeat");
            encoder.encode_frames(gif_frames).expect("gif");
            println!("  wrote {path} ({n} frames)");
        }
    }
}
