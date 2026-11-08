//! **Is aiming lightning fun, or dice?** -- the headless harness for the
//! Storm Glass probe (`cargo run --release --bin storm`).
//!
//! Plays whole storms with scripted hands and prints what happened, so the
//! question can be answered with counts beside the pictures:
//!
//! - `mode=compare` (default): three players over the same `seeds` --
//!   **idle** (touches nothing), **random** (calls at random places for
//!   random holds and spends water and rods at random) and **planned** (the
//!   script below) -- with stars, lamp light, bell hits, glass and scorch
//!   per seed, and paired sign tests between every pair. `players=` picks
//!   others: the playtest's `cheese` and `sloppy`, `spam`, `greedy` and
//!   `wirer` (see [`Who`]). They find lamps by name, so one build of this
//!   file replays the same hands on the two-lamp and the four-lamp valley.
//! - `mode=aim`: the same marked target (the field lamp) called from
//!   directly above it at three hold lengths over `seeds` seeds, with hits
//!   and the landing spread. This is the "dice" question on its own.
//! - `mode=play player=planned seed=3 gif=out.gif`: one storm, every frame
//!   of the real window (the same `Storm::draw` the binary calls), written
//!   as a GIF; `png=out.png at=TICK` writes one frame.
//!
//! Every mode prints the frame cost over the whole storm -- simulation and
//! drawing together, mean and worst -- because a storm of quick bolts is
//! the worst-frame case for this game (`review-3-synthesis.md` §5).
//!
//! ```text
//! cargo run --release --example stormprobe
//! cargo run --release --example stormprobe -- mode=aim seeds=10
//! cargo run --release --example stormprobe -- mode=play player=planned seed=1 gif=planned.gif
//! ```

use pixel_physics::storm::{level, Input, Phase, Storm, RODS, TICKS_PER_SECOND};
use pixel_physics::sim::rng::Rng;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Who {
    Idle,
    Random,
    Planned,
    /// Not a player: a showcase of the two marks the planned storm never
    /// makes -- glass in the sand, and a splash bending a held leader.
    Show,
    /// **The playtest's cheese** (`playtest-2026-11-08/storm-review.md`):
    /// right-click with the pointer just above the head of the lamp under the
    /// rock, then hold 1.25 s onto whichever lamp is least lit -- the rock
    /// lamp by striking the ledge above it. A rod driven there stood on the
    /// lamp and reached up into the ledge, so every ledge bolt lit the lamp:
    /// 3 stars on 10 of 10 seeds for the tester.
    Cheese,
    /// The same cheese played carelessly: 0.3 s taps, aim off by up to 3.
    Sloppy,
    /// Eight quick taps onto the field lamp and nothing else.
    Spam,
    /// A sensible player with no plan for wiring: every bolt held 1.25 s onto
    /// whichever lamp in the open (not the one under the rock) is least lit.
    Greedy,
    /// A plan that adapts to the layout: a rod at the mouth of the rock
    /// shelf, struck until its scorch line lights the lamp under the rock
    /// (at most five bolts), then the lamps in the open, least lit first.
    Wirer,
    /// The wirer's plan the other way round: the three open lamps first
    /// (a held bolt, then a tap to top up), then the rod at the shelf mouth
    /// and the bolts that are left onto it.
    Finisher,
}

impl Who {
    fn name(self) -> &'static str {
        match self {
            Who::Idle => "idle",
            Who::Random => "random",
            Who::Planned => "planned",
            Who::Show => "show",
            Who::Cheese => "cheese",
            Who::Sloppy => "sloppy",
            Who::Spam => "spam",
            Who::Greedy => "greedy",
            Who::Wirer => "wirer",
            Who::Finisher => "finisher",
        }
    }

    fn parse(name: &str) -> Who {
        [Who::Idle, Who::Random, Who::Planned, Who::Show, Who::Cheese, Who::Sloppy, Who::Spam, Who::Greedy, Who::Wirer, Who::Finisher]
            .into_iter()
            .find(|w| w.name() == name)
            .unwrap_or_else(|| panic!("unknown player {name}"))
    }
}

/// One planned bolt: things to do while the cloud gathers, then where to
/// call and for how long.
struct Plan {
    /// (kind, x) -- 's' pours water at the surface above x, 'r' drives a rod.
    before: &'static [(char, i32)],
    /// (tick of the hold, x) -- water poured *while* holding, to bend the
    /// leader.
    during: &'static [(u32, i32)],
    aim: i32,
    hold: f32,
}

/// **The planned player.** Written by playing the two-lamp valley, and kept
/// unchanged since as the anchor for before/after replays (it does not know
/// about the hill and sand lamps). Written by playing, not derived: two heavy bolts
/// straight down onto the field lamp, then a rod at the mouth of the rock
/// shelf struck hard so its scorch runs under the shelf to the second
/// lamp's post, then the rest onto that rod -- every hit on it lights the
/// lamp under the rock through the scorch line.
const PLAN: &[Plan] = &[
    Plan { during: &[], before: &[], aim: 135, hold: 1.25 },
    Plan { during: &[], before: &[], aim: 135, hold: 1.25 },
    Plan { during: &[], before: &[('r', 168)], aim: 166, hold: 1.4 },
    Plan { during: &[], before: &[], aim: 166, hold: 1.3 },
    Plan { during: &[], before: &[], aim: 166, hold: 1.3 },
    Plan { during: &[], before: &[('s', 178)], aim: 166, hold: 1.3 },
    Plan { during: &[], before: &[], aim: 135, hold: 1.25 },
    Plan { during: &[], before: &[], aim: 166, hold: 1.3 },
];

/// The showcase: three bolts into the sand (glass, and the third has to
/// find a way round the first two's glass), then a held bolt over the sand
/// edge bent onto wet ground by a splash poured mid-hold, then the lamps.
const SHOW: &[Plan] = &[
    Plan { during: &[], before: &[], aim: 98, hold: 1.0 },
    Plan { during: &[], before: &[], aim: 102, hold: 1.0 },
    Plan { during: &[], before: &[], aim: 100, hold: 1.0 },
    Plan { during: &[(40, 122)], before: &[], aim: 112, hold: 1.4 },
    Plan { during: &[], before: &[], aim: 134, hold: 1.25 },
    Plan { during: &[], before: &[('r', 168)], aim: 167, hold: 1.4 },
    Plan { during: &[], before: &[], aim: 167, hold: 1.3 },
    Plan { during: &[], before: &[], aim: 167, hold: 1.3 },
];

/// The scripted hands. Holds a little state between ticks.
struct Hands {
    who: Who,
    rng: Rng,
    bolt: usize,
    /// Ticks into the current Ready phase at which to press.
    wait: u32,
    hold: u32,
    aim: i32,
    did_before: bool,
    /// (kind, x, y): 's' water, 'r' rod; y < 0 means "at the surface".
    pending: Vec<(char, i32, i32)>,
    during: &'static [(u32, i32)],
}

impl Hands {
    fn new(who: Who, seed: u64) -> Self {
        Hands { who, rng: Rng::new(seed * 7919 + 3), bolt: 0, wait: 0, hold: 0, aim: 128, did_before: false, pending: Vec::new(), during: &[] }
    }

    fn surface(s: &Storm, x: i32) -> i32 {
        let mut y = pixel_physics::storm::CLOUD_BASE + 2;
        while y < level::H && s.world.get(x, y).material == pixel_physics::sim::material::EMPTY {
            y += 1;
        }
        y
    }

    /// A lamp by part of its name -- so the cheese replays on any layout.
    fn lamp(s: &Storm, part: &str) -> Option<level::Lamp> {
        s.level.lamps.iter().copied().find(|l| l.name.contains(part))
    }

    fn next(&mut self, s: &Storm) -> Input {
        let mut input = Input { at: (self.aim, pixel_physics::storm::CLOUD_BASE + 6), ..Input::default() };
        if self.who == Who::Idle {
            return input;
        }
        match &s.phase {
            Phase::Gathering { .. } => {
                if !self.did_before {
                    self.did_before = true;
                    self.pending.clear();
                    match self.who {
                        Who::Planned | Who::Show => {
                            let plan = if self.who == Who::Show { SHOW } else { PLAN };
                            let p = &plan[self.bolt.min(plan.len() - 1)];
                            self.during = p.during;
                            self.pending.extend(p.before.iter().map(|&(k, x)| (k, x, -1)));
                            self.aim = p.aim;
                            self.hold = (p.hold * TICKS_PER_SECOND as f32) as u32;
                            self.wait = 6;
                        }
                        Who::Random => {
                            if self.rng.chance(0.5) {
                                self.pending.push(('s', self.rng.below(250) as i32 + 3, -1));
                            }
                            if self.rng.chance(0.25) {
                                self.pending.push(('r', self.rng.below(250) as i32 + 3, -1));
                            }
                            self.aim = self.rng.below(250) as i32 + 3;
                            self.hold = (TICKS_PER_SECOND as f32 * (0.1 + 2.5 * self.rng.unit_f32())) as u32;
                            self.wait = self.rng.below(TICKS_PER_SECOND * 3) + 4;
                        }
                        Who::Cheese | Who::Sloppy => {
                            let rock = Self::lamp(s, "ROCK").expect("a lamp under the rock");
                            let (rx, _) = rock.area.centre();
                            // The pointer in the gap just above the lamp head, as
                            // in the tester's picture: the rod drops onto the head
                            // and its top meets the ledge. Retried before the
                            // second bolt in case the first was refused.
                            if self.bolt < 2 && s.rods_left == 2 - self.bolt as u32 {
                                self.pending.push(('r', rx, rock.head.y0 - 3));
                            }
                            // Then every bolt onto whichever lamp is least lit,
                            // the rock lamp by striking the ledge above it.
                            let (_, l) = s.level.lamps.iter().enumerate().min_by_key(|&(k, _)| (s.lamps[k], k)).expect("a lamp");
                            let sloppy = self.who == Who::Sloppy;
                            self.aim = l.area.centre().0 + if sloppy { self.rng.below(7) as i32 - 3 } else { 0 };
                            self.hold = (TICKS_PER_SECOND as f32 * if sloppy { 0.3 } else { 1.25 }) as u32;
                            self.wait = 6;
                        }
                        Who::Spam => {
                            let field = Self::lamp(s, "FIELD").expect("a field lamp");
                            self.aim = field.area.centre().0;
                            self.hold = (TICKS_PER_SECOND as f32 * 0.2) as u32;
                            self.wait = 4;
                        }
                        Who::Wirer | Who::Finisher => {
                            let rock = s.level.lamps.iter().position(|l| l.name.contains("ROCK")).expect("a lamp under the rock");
                            let open_full = s.level.lamps.iter().enumerate().all(|(k, l)| l.name.contains("ROCK") || s.lamps[k] >= 3);
                            // Two cells clear of the ledge's lip, under open sky.
                            let rx = s.level.ledge.x0 - 3;
                            let wiring = if self.who == Who::Wirer { s.lamps[rock] < 3 && self.bolt < 5 } else { open_full };
                            if wiring && s.rods_left == RODS {
                                self.pending.push(('r', rx, -1));
                            }
                            if wiring {
                                self.aim = rx + 1;
                                self.hold = (TICKS_PER_SECOND as f32 * 1.4) as u32;
                            } else {
                                let open = s.level.lamps.iter().enumerate().filter(|(_, l)| !l.name.contains("ROCK"));
                                let (k, l) = open.min_by_key(|&(k, _)| (s.lamps[k], k)).expect("an open lamp");
                                self.aim = l.area.centre().0;
                                // A lamp one short of full needs only a tap.
                                self.hold = (TICKS_PER_SECOND as f32 * if s.lamps[k] == 2 { 0.4 } else { 1.25 }) as u32;
                            }
                            self.wait = 6;
                        }
                        Who::Greedy => {
                            let open = s.level.lamps.iter().enumerate().filter(|(_, l)| !l.name.contains("ROCK"));
                            let (_, l) = open.min_by_key(|&(k, _)| (s.lamps[k], k)).expect("an open lamp");
                            self.aim = l.area.centre().0;
                            self.hold = (TICKS_PER_SECOND as f32 * 1.25) as u32;
                            self.wait = 6;
                        }
                        Who::Idle => {}
                    }
                }
                if let Some((k, x, y)) = self.pending.pop() {
                    let y = if y >= 0 { y } else { Self::surface(s, x) - if k == 's' { 6 } else { 0 } };
                    input.at = (x, y);
                    if k == 's' {
                        input.splash = true;
                    } else {
                        input.rod = true;
                    }
                }
            }
            Phase::Ready { t } => {
                input.hold = *t >= self.wait;
            }
            Phase::Calling { t, .. } => {
                input.hold = *t < self.hold;
                if let Some(&(_, x)) = self.during.iter().find(|&&(at, _)| at == *t) {
                    input.at = (x, Self::surface(s, x) - 5);
                    input.splash = true;
                }
                if !input.hold {
                    self.bolt += 1;
                    self.did_before = false;
                }
            }
            Phase::Over => {}
        }
        input
    }
}

#[derive(Default, Clone, Debug)]
struct Run {
    stars: u32,
    points: i32,
    lamp_light: u32,
    bell: u32,
    wired: u32,
    glass: u32,
    scorch: u32,
    fires: u32,
    huts_lost: u32,
    ticks: u64,
    mean_ms: f64,
    worst_ms: f64,
    said: Vec<String>,
}

fn play(who: Who, seed: u64, mut film: Option<&mut Film>) -> Run {
    let mut s = Storm::new(seed);
    let mut hands = Hands::new(who, seed);
    let (fw, fh) = Storm::frame_size();
    let mut frame = vec![0u8; (fw * fh * 4) as usize];
    let (mut total, mut worst) = (0.0f64, 0.0f64);
    let mut said = Vec::new();
    let mut last_bolts = 0;
    while !s.is_over() && s.tick < 60 * 120 {
        let input = hands.next(&s);
        let t0 = Instant::now();
        s.update(input);
        s.draw(&mut frame);
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        total += ms;
        worst = worst.max(ms);
        if s.tally.bolts != last_bolts {
            last_bolts = s.tally.bolts;
            if let Some(st) = &s.strike {
                said.push(st.said.clone());
                if std::env::var_os("STORM_TRACE").is_some() {
                    eprintln!("  bolt {} {:?} heavy {:.2} hit {:?} main {} branches {} roots {} network {} | {}", s.tally.bolts, st.cause, st.heavy, st.hit, st.main.len(), st.branches.len(), st.roots.len(), st.network.len(), st.said);
                }
            }
        }
        if let Some(f) = film.as_deref_mut() {
            f.take(s.tick, &frame, (fw, fh));
        }
    }
    // A few frames of the result panel.
    for _ in 0..90 {
        s.update(Input::default());
        s.draw(&mut frame);
        if let Some(f) = film.as_deref_mut() {
            f.take(s.tick, &frame, (fw, fh));
        }
    }
    Run {
        stars: s.stars(),
        points: s.points(),
        lamp_light: s.lamps.iter().map(|&l| l as u32).sum(),
        bell: s.tally.bell_hits,
        wired: s.tally.wired_hits,
        glass: s.tally.glass_cells,
        scorch: s.tally.scorch_cells,
        fires: s.tally.fires,
        huts_lost: s.huts_lost(),
        ticks: s.tick,
        mean_ms: total / s.tick.max(1) as f64,
        worst_ms: worst,
        said,
    }
}

/// Frames for a GIF or a still.
struct Film {
    every: u64,
    /// `video=out.mp4`: every `every`th frame at full size, piped raw into
    /// `ffmpeg` (H.264). A whole storm as a GIF is 15-30 MB, too big for a
    /// picture page; the same storm as video is a few MB at full size.
    video: Option<std::process::Child>,
    half: bool,
    frames: Vec<image::RgbaImage>,
    still_at: Option<u64>,
    still: Option<image::RgbaImage>,
}

impl Film {
    fn take(&mut self, tick: u64, frame: &[u8], (w, h): (u32, u32)) {
        let img = image::RgbaImage::from_raw(w, h, frame.to_vec()).expect("frame size");
        if self.still_at == Some(tick) {
            self.still = Some(img.clone());
        }
        if self.every > 0 && tick.is_multiple_of(self.every) {
            if let Some(v) = self.video.as_mut() {
                use std::io::Write;
                v.stdin.as_mut().expect("ffmpeg stdin").write_all(frame).expect("ffmpeg write");
                return;
            }
            let img = if self.half { image::imageops::resize(&img, w / 2, h / 2, image::imageops::FilterType::Triangle) } else { img };
            self.frames.push(img);
        }
    }

    fn write_gif(&self, path: &str) {
        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(1000 * self.every / TICKS_PER_SECOND as u64));
        let file = std::fs::File::create(path).expect("gif file");
        let mut enc = image::codecs::gif::GifEncoder::new_with_speed(file, 20);
        enc.set_repeat(image::codecs::gif::Repeat::Infinite).expect("repeat");
        for f in &self.frames {
            enc.encode_frame(image::Frame::from_parts(f.clone(), 0, 0, delay)).expect("gif frame");
        }
        println!("wrote {path}: {} frames", self.frames.len());
    }
}

fn arg(key: &str) -> Option<String> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).map(str::to_string))
}

/// Two-sided sign test p for `wins` against `losses` (ties dropped).
fn sign_p(wins: u32, losses: u32) -> f64 {
    let n = wins + losses;
    if n == 0 {
        return 1.0;
    }
    let k = wins.min(losses);
    let mut c = 1.0f64;
    let mut sum = 0.0;
    for i in 0..=n {
        if i > 0 {
            c = c * (n - i + 1) as f64 / i as f64;
        }
        if i <= k {
            sum += c;
        }
    }
    (2.0 * sum / 2f64.powi(n as i32)).min(1.0)
}

fn main() {
    for a in std::env::args().skip(1) {
        let k = a.split('=').next().unwrap_or("");
        if !["mode", "seeds", "seed", "player", "players", "gif", "png", "at", "every", "full", "video"].contains(&k) {
            panic!("unknown argument `{a}` -- an ignored argument is how a study silently measures its defaults");
        }
    }
    let mode = arg("mode").unwrap_or_else(|| "compare".into());
    let seeds: u64 = arg("seeds").map_or(10, |v| v.parse().expect("seeds"));
    println!("stormprobe: mode={mode} seeds={seeds} world={}x{}", level::W, level::H);
    match mode.as_str() {
        "compare" => {
            let whos: Vec<Who> = arg("players").unwrap_or_else(|| "idle,random,planned".into()).split(',').map(Who::parse).collect();
            compare(seeds, &whos)
        }
        "aim" => aim(seeds),
        "play" => {
            let who = Who::parse(arg("player").as_deref().unwrap_or("planned"));
            let seed: u64 = arg("seed").map_or(1, |v| v.parse().expect("seed"));
            let every: u64 = arg("every").map_or(if arg("gif").is_some() || arg("video").is_some() { 2 } else { 0 }, |v| v.parse().expect("every"));
            let video = arg("video").map(|path| {
                let (w, h) = Storm::frame_size();
                std::process::Command::new("ffmpeg")
                    .args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgba", "-s", &format!("{w}x{h}"), "-r", &format!("{}", TICKS_PER_SECOND as u64 / every.max(1))])
                    .args(["-i", "-", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "20", "-movflags", "+faststart", &path])
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                    .expect("ffmpeg on PATH")
            });
            let mut film = Film { every, video, half: arg("full").is_none(), frames: Vec::new(), still_at: arg("at").map(|v| v.parse().expect("at")), still: None };
            let r = play(who, seed, Some(&mut film));
            println!(
                "{} seed {seed}: {} stars, {} points, lamp light {}, bell hits {}, lit through wiring {}, glass {}, scorch {}, fires {}, hut cells lost {}, {} ticks, frame mean {:.2} ms worst {:.2} ms",
                who.name(),
                r.stars,
                r.points,
                r.lamp_light,
                r.bell,
                r.wired,
                r.glass,
                r.scorch,
                r.fires,
                r.huts_lost,
                r.ticks,
                r.mean_ms,
                r.worst_ms
            );
            for (i, said) in r.said.iter().enumerate() {
                println!("  bolt {}: {said}", i + 1);
            }
            if let Some(p) = arg("gif") {
                film.write_gif(&p);
            }
            if let Some(mut v) = film.video.take() {
                drop(v.stdin.take());
                v.wait().expect("ffmpeg");
                println!("wrote {}", arg("video").unwrap_or_default());
            }
            if let (Some(p), Some(img)) = (arg("png"), film.still.as_ref()) {
                img.save(&p).expect("png");
                println!("wrote {p}");
            }
        }
        other => panic!("unknown mode {other}"),
    }
}

fn compare(seeds: u64, whos: &[Who]) {
    let mut runs: Vec<Vec<Run>> = vec![Vec::new(); whos.len()];
    println!("{:>8} {:>4} | {:>5} {:>5} {:>5} {:>4} {:>5} {:>6} {:>6} {:>5} {:>4} | {:>6} {:>6}", "player", "seed", "stars", "pts", "light", "bell", "wired", "glass", "scorch", "fires", "huts", "meanms", "worst");
    for seed in 1..=seeds {
        for (k, &who) in whos.iter().enumerate() {
            let r = play(who, seed, None);
            println!(
                "{:>8} {:>4} | {:>5} {:>5} {:>5} {:>4} {:>5} {:>6} {:>6} {:>5} {:>4} | {:>6.2} {:>6.2}",
                who.name(),
                seed,
                r.stars,
                r.points,
                r.lamp_light,
                r.bell,
                r.wired,
                r.glass,
                r.scorch,
                r.fires,
                r.huts_lost,
                r.mean_ms,
                r.worst_ms
            );
            runs[k].push(r);
        }
    }
    println!();
    for (k, &who) in whos.iter().enumerate() {
        let rs = &runs[k];
        let n = rs.len() as f64;
        let mean = |f: &dyn Fn(&Run) -> f64| rs.iter().map(f).sum::<f64>() / n;
        let mut stars = [0; 4];
        for r in rs {
            stars[r.stars as usize] += 1;
        }
        println!(
            "{:>8}: stars 0/1/2/3 = {:?}  per seed {:?}  total {}/{}  mean points {:.1}  lamp light {:.1}  bell hits {:.1}  wired lights {:.1}  frame mean {:.2} ms worst {:.2} ms",
            who.name(),
            stars,
            rs.iter().map(|r| r.stars).collect::<Vec<_>>(),
            rs.iter().map(|r| r.stars).sum::<u32>(),
            3 * rs.len(),
            mean(&|r| r.points as f64),
            mean(&|r| r.lamp_light as f64),
            mean(&|r| r.bell as f64),
            mean(&|r| r.wired as f64),
            mean(&|r| r.mean_ms),
            rs.iter().map(|r| r.worst_ms).fold(0.0, f64::max)
        );
    }
    println!();
    // Paired by seed, on points (the deciding measure): every pair asked for.
    let pairs: Vec<(usize, usize)> = (0..whos.len()).flat_map(|a| (0..a).map(move |b| (a, b))).collect();
    for &(a, b) in &pairs {
        let (mut w, mut l, mut t) = (0, 0, 0);
        for (p, o) in runs[a].iter().zip(&runs[b]) {
            match p.points.cmp(&o.points) {
                std::cmp::Ordering::Greater => w += 1,
                std::cmp::Ordering::Less => l += 1,
                std::cmp::Ordering::Equal => t += 1,
            }
        }
        println!("{} vs {} on points: {w} better, {l} worse, {t} tied, sign-test p = {:.4}", whos[a].name(), whos[b].name(), sign_p(w, l));
    }
    if pairs.len() > 1 {
        println!("({} comparisons: Bonferroni bar p < {:.4})", pairs.len(), 0.05 / pairs.len() as f64);
    }
}

/// The same marked target over `seeds` seeds at three hold lengths: called
/// above the field lamp from four distances, on a fresh valley each time,
/// one bolt. Then the same three holds over open sand with nothing to pull,
/// for the landing spread alone. This is the "dice" question on its own:
/// a skill game wants hits to fall off with distance and rise with hold.
fn aim(seeds: u64) {
    let lamps = level::Level::valley().lamps;
    let k = lamps.iter().position(|l| l.name.contains("FIELD")).expect("a field lamp");
    let lamp = lamps[k];
    let (lx, _) = lamp.area.centre();
    println!("target: {} at x={} (lamp x {}..{})", lamp.name, lx, lamp.area.x0, lamp.area.x1);
    let one = |seed: u64, x: i32, hold: f32| -> (bool, Option<(i32, i32)>) {
        let mut s = Storm::new(seed);
        let ticks = (hold * TICKS_PER_SECOND as f32) as u32;
        let at = (x, pixel_physics::storm::CLOUD_BASE + 6);
        while s.tally.bolts == 0 && s.tick < 60 * 30 {
            let hold_now = match &s.phase {
                Phase::Ready { .. } => true,
                Phase::Calling { t, .. } => *t < ticks,
                _ => false,
            };
            s.update(Input { at, hold: hold_now, ..Input::default() });
        }
        (s.tally.lamp_hits[k] > 0, s.tally.landings.last().and_then(|l| l.1))
    };
    for off in [0, 6, 12, 20] {
        let mut row = format!("  aimed {off:>2} cells beside the lamp:");
        for hold in [0.2f32, 0.8, 1.4] {
            let hits = (1..=seeds).filter(|&seed| one(seed, lx + off, hold).0).count();
            row += &format!("  hold {hold:.1}s lit {hits:>2}/{seeds}");
        }
        println!("{row}");
    }
    let sand = 99;
    for hold in [0.2f32, 0.8, 1.4] {
        let lands: Vec<Option<(i32, i32)>> = (1..=seeds).map(|seed| one(seed, sand, hold).1).collect();
        if std::env::var_os("STORM_TRACE").is_some() {
            eprintln!("    landings {lands:?}");
        }
        let mut offs: Vec<i32> = lands.iter().filter_map(|l| l.map(|(x, _)| (x - sand).abs())).collect();
        offs.sort();
        println!("  over open sand at x={sand}, hold {hold:.1}s: landed this far from the aim, median {} p90 {}: {offs:?}", offs[offs.len() / 2], offs[(offs.len() * 9) / 10]);
    }
}
