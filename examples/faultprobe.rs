//! Headless harness for the Fault probe (`src/fault/`, `--bin fault`).
//!
//! Plays scripted runs of "The Mill" with no window and prints the counts a
//! picture cannot carry: cracks run, forks, stops at holes, slabs freed,
//! the spillway's peak flow, the share drained, stars -- and the frame cost.
//!
//! ```text
//! cargo run --release --example faultprobe                         # the paired table: nothing / random / planned over 10 seeds
//! cargo run --release --example faultprobe -- plan=planned seed=3 gif=1 out=/tmp/fault
//! cargo run --release --example faultprobe -- repeat=1              # same tap, same seed twice; same tap over 10 seeds
//! cargo run --release --example faultprobe -- shot=1 out=/tmp/fault # stills for the shimmer A/B
//! ```
//!
//! A plan is a list of `(second, tool, x, y)`. `nothing` does nothing (the
//! control: the lake must win). `random` taps at random places on the ice
//! at the same rate a player would (it must lose to a plan). `planned` is
//! the spillway recipe: stoppers first, then a notch cut down to them.

use std::path::{Path, PathBuf};
use std::time::Instant;

use pixel_physics::fault::level;
use pixel_physics::fault::view::{self, Ui, View};
use pixel_physics::fault::{Action, Game, Tool, FPS, THAW_SECONDS};
use pixel_physics::sim::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Plan {
    Nothing,
    Random,
    Planned,
    /// One tap, nothing else: the repeatability case.
    OneTap,
}

impl Plan {
    fn parse(s: &str) -> Option<Plan> {
        match s {
            "nothing" => Some(Plan::Nothing),
            "random" => Some(Plan::Random),
            "planned" => Some(Plan::Planned),
            "onetap" => Some(Plan::OneTap),
            _ => None,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Plan::Nothing => "nothing",
            Plan::Random => "random",
            Plan::Planned => "planned",
            Plan::OneTap => "onetap",
        }
    }
}

/// A player: asked once a frame what to do, if anything.
trait Player {
    fn decide(&mut self, g: &Game, frame: u64) -> Option<Action>;
}

/// Does nothing. The control: the lake must win.
struct Idle;
impl Player for Idle {
    fn decide(&mut self, _: &Game, _: u64) -> Option<Action> {
        None
    }
}

/// One tap at a fixed place, two seconds in: the repeatability case.
struct OneTap(i32, i32);
impl Player for OneTap {
    fn decide(&mut self, _: &Game, frame: u64) -> Option<Action> {
        (frame == 2 * FPS as u64).then_some(Action { tool: Tool::Tap, x: self.0, y: self.1 })
    }
}

/// Someone who has not read the ice: every two seconds, a random tool at a
/// random place on the wall, mostly taps.
struct Random(Rng);
impl Player for Random {
    fn decide(&mut self, _: &Game, frame: u64) -> Option<Action> {
        if !frame.is_multiple_of(2 * FPS as u64) || frame == 0 {
            return None;
        }
        let r = &mut self.0;
        let x = level::WALL_X0 + r.below((level::WALL_X1 - level::WALL_X0) as u32) as i32;
        let y = level::WALL_TOP + r.below((level::FLOOR_Y - level::WALL_TOP) as u32) as i32;
        let tool = match r.below(10) {
            0 => Tool::Drill,
            1 => Tool::Meltwater,
            2 => Tool::Frost,
            _ => Tool::Tap,
        };
        let y = if tool == Tool::Meltwater { level::WALL_TOP - 2 } else { y };
        Some(Action { tool, x, y })
    }
}

/// **The planned player**: cuts a spillway in stages, each a notch from the
/// wall's current top down to a target row, made before the lake gets
/// there -- so that the lake heaves it out when it arrives and then drains
/// to the notch's floor. Then the next notch, deeper.
///
/// It plans by trying moves in its head (`Game::trial` and
/// `settle_cracks`): every candidate tap in a band around the target row,
/// scored on how much of the wall's width the cracks now cross in that band
/// and how far they dived below it. That is a stand-in for a player who can
/// read where a crack will run -- **which is the question the probe asks of
/// a person, not something this answers**; it only shows that a plan
/// exists and what it is worth against random play.
struct Planner {
    /// Target notch floors, shallow first.
    stages: Vec<i32>,
    stage: usize,
    next_at: u64,
    pending: Vec<Action>,
}

impl Planner {
    fn new() -> Self {
        Planner { stages: vec![76, 96, 112], stage: 0, next_at: FPS as u64, pending: Vec::new() }
    }
}

/// Topmost ice row in the wall's middle columns: the wall's current top.
fn wall_top(g: &Game) -> i32 {
    for y in 0..level::FLOOR_Y {
        for x in level::WALL_X0 + 4..level::WALL_X1 - 8 {
            if g.is_ice(x, y) {
                return y;
            }
        }
    }
    level::FLOOR_Y
}

/// The top of the ice still joined to the bedrock through uncracked ice:
/// everything above it has been cut off, whether in one piece or several.
fn anchored_top(g: &Game) -> i32 {
    let mut seen = vec![false; (level::W * level::H) as usize];
    let mut stack = Vec::new();
    for x in level::WALL_X0..=level::WALL_X1 {
        if g.is_ice(x, level::FLOOR_Y - 1) {
            stack.push((x, level::FLOOR_Y - 1));
            seen[((level::FLOOR_Y - 1) * level::W + x) as usize] = true;
        }
    }
    let mut top = level::FLOOR_Y;
    while let Some((x, y)) = stack.pop() {
        top = top.min(y);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if !g.is_ice(nx, ny) || seen[(ny * level::W + nx) as usize] {
                continue;
            }
            if pixel_physics::sim::structural::edge_is_cracked(&g.world, x, y, dx, dy) {
                continue;
            }
            seen[(ny * level::W + nx) as usize] = true;
            stack.push((nx, ny));
        }
    }
    top
}

/// How good a crack pattern is for a notch floored at `d`. Once everything
/// above the band is cut off from the bedrock, a large bonus less how far
/// the cut is from `d`. Before that: minus the columns still joined top to
/// bottom through the band (a "ligament" -- no crossing crack, no hole), and
/// a price on every edge cracked deep below it (a dive there frees a piece
/// the lake will throw hard).
fn notch_score(g: &Game, d: i32) -> f32 {
    let at = anchored_top(g);
    if at >= d - 8 {
        return 1000.0 - (at - d).abs() as f32 * 8.0;
    }
    let (mut ligaments, mut deep) = (0, 0);
    for x in level::WALL_X0..=level::WALL_X1 {
        if !g.is_ice(x, d) {
            continue;
        }
        let cut = (d - 10..=d + 4).any(|y| g.tips.is_hole(x, y) || (g.is_ice(x, y) && g.world.get(x, y).crack_down()));
        if !cut {
            ligaments += 1;
        }
        for y in d + 10..level::FLOOR_Y {
            let c = g.world.get(x, y);
            if g.is_ice(x, y) && c.cracked() {
                deep += 1;
            }
        }
    }
    -3.0 * ligaments as f32 - 1.0 * deep as f32
}

impl Player for Planner {
    fn decide(&mut self, g: &Game, frame: u64) -> Option<Action> {
        if !g.tips.tips.is_empty() || frame < self.next_at || g.over() || g.counts.burst {
            return None;
        }
        if let Some(a) = self.pending.pop() {
            self.next_at = frame + FPS as u64 / 2;
            return Some(a);
        }
        let &d = self.stages.get(self.stage)?;
        let top = wall_top(g);
        // This stage's notch is out and the lake has come down to it: next.
        if top >= d - 8 {
            if g.level.surface >= top - 4 {
                self.stage += 1;
            }
            self.next_at = frame + FPS as u64;
            return None;
        }
        // Cut and waiting for the lake.
        if anchored_top(g) >= d - 8 {
            self.next_at = frame + FPS as u64;
            return None;
        }
        let here = notch_score(g, d);
        let mut best: Option<(f32, Vec<Action>)> = None;
        let mut consider = |score: f32, acts: Vec<Action>| {
            if best.as_ref().is_none_or(|b| score > b.0) {
                best = Some((score, acts));
            }
        };
        let try_tap = |base: &Game, x: i32, y: i32| -> f32 {
            let mut t = base.trial();
            t.act(Action { tool: Tool::Tap, x, y });
            t.settle_cracks();
            notch_score(&t, d)
        };
        for y in (d - 10..=d + 2).step_by(3) {
            for x in (level::WALL_X0 + 1..level::WALL_X1).step_by(3) {
                if g.is_ice(x, y) {
                    consider(try_tap(g, x, y), vec![Action { tool: Tool::Tap, x, y }]);
                }
            }
        }
        // A stopper under the band, then the best tap above it.
        if g.drills_left > 0 {
            for x in (level::WALL_X0 + 3..level::WALL_X1 - 2).step_by(5) {
                let drill = Action { tool: Tool::Drill, x, y: d + 5 };
                let mut t = g.trial();
                if !t.act(drill) {
                    continue;
                }
                let mut inner: Option<(f32, Action)> = None;
                for y in (d - 10..=d + 2).step_by(3) {
                    for tx in (x - 6..=x + 6).step_by(3) {
                        if t.is_ice(tx, y) {
                            let s = try_tap(&t, tx, y);
                            if inner.is_none_or(|b| s > b.0) {
                                inner = Some((s, Action { tool: Tool::Tap, x: tx, y }));
                            }
                        }
                    }
                }
                if let Some((s, tap)) = inner {
                    // A drill is spent, so it has to buy something.
                    consider(s - 6.0, vec![tap, drill]);
                }
            }
        }
        // Meltwater on the wall's top, then Frost: wakes every soaked end.
        if !g.tips.ends.is_empty() {
            for x in (level::WALL_X0 + 2..level::WALL_X1 - 4).step_by(6) {
                let mut t = g.trial();
                t.act(Action { tool: Tool::Meltwater, x, y: top - 2 });
                for _ in 0..45 {
                    t.step();
                }
                t.act(Action { tool: Tool::Frost, x, y: top });
                t.settle_cracks();
                consider(notch_score(&t, d) - 4.0, vec![Action { tool: Tool::Frost, x, y: top }, Action { tool: Tool::Meltwater, x, y: top - 2 }]);
            }
        }
        let (score, mut acts) = best?;
        if score <= here + 2.0 {
            self.next_at = frame + FPS as u64;
            return None;
        }
        let first = acts.pop();
        self.pending = acts;
        // Meltwater needs time to soak before the Frost that follows it.
        self.next_at = frame + if first.map(|a| a.tool) == Some(Tool::Meltwater) { 45 } else { FPS as u64 / 2 };
        first
    }
}

fn player(plan: Plan, seed: u64) -> Box<dyn Player> {
    match plan {
        Plan::Nothing => Box::new(Idle),
        Plan::OneTap => Box::new(OneTap(112, 60)),
        Plan::Random => Box::new(Random(Rng::new(seed ^ 0xF00D))),
        Plan::Planned => Box::new(Planner::new()),
    }
}

struct Run {
    plan: Plan,
    seed: u64,
    game: Game,
    worst_ms: f64,
    mean_ms: f64,
    /// Worst frame within a second of a slab breaking loose.
    worst_break_ms: f64,
}

fn arg<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter().find_map(|a| a.strip_prefix(&format!("{key}=")))
}

fn play(plan: Plan, seed: u64, shimmer: bool, frames: u64, mut each: impl FnMut(&Game, u64)) -> Run {
    let mut game = Game::new(seed);
    game.shimmer = shimmer;
    let mut who = player(plan, seed);
    let (mut worst, mut total, mut worst_break) = (0f64, 0f64, 0f64);
    let mut last_slabs = 0;
    let mut since_break = u64::MAX;
    for f in 0..frames {
        if let Some(a) = who.decide(&game, f) {
            game.act(a);
        }
        let t0 = Instant::now();
        game.step();
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        if f > 5 {
            worst = worst.max(ms);
        }
        total += ms;
        if game.counts.slabs_freed > last_slabs {
            since_break = 0;
            last_slabs = game.counts.slabs_freed;
        }
        if since_break < FPS as u64 {
            worst_break = worst_break.max(ms);
            since_break += 1;
        }
        each(&game, f);
    }
    Run { plan, seed, game, worst_ms: worst, mean_ms: total / frames as f64, worst_break_ms: worst_break }
}

fn row(r: &Run) -> String {
    let c = r.game.counts;
    let s = r.game.score();
    format!(
        "{:<8} seed={:<2} stars={} burst={:<5} drained={:>3.0}% peak_flow={:>5.1}/s mill_hits={:<3} pieces={:<3}/{:<4} released={:<3} jammed={:<2} unheld={:<3} blocks={:<3} grit={:<4} slabs={:<3} cracks={:<3} edges={:<4} forks={:<3} holes_stopped={:<3} face={:<3} joined={:<3} died={:<3} frost_tips={:<3} taps={} drills={} pours={} frosts={} worst={:.1}ms mean={:.2}ms break_worst={:.1}ms",
        r.plan.label(),
        r.seed,
        s.stars,
        s.burst,
        s.drained * 100.0,
        s.peak_flow,
        c.mill_hits,
        c.pieces_cut,
        c.piece_cells,
        c.pieces_released,
        c.jammed,
        c.unheld,
        c.blocks_thrown,
        c.grit_cells,
        c.slabs_freed,
        c.cracks_run,
        c.edges_cracked,
        c.forks,
        c.stopped_by_hole,
        c.stopped_at_face,
        c.joined,
        c.died,
        c.frost_tips,
        c.taps,
        c.drills,
        c.pours,
        c.frosts,
        r.worst_ms,
        r.mean_ms,
        r.worst_break_ms
    )
}

fn png(game: &Game, view: &mut View, ui: Ui, path: &Path) {
    let (w, h) = view::size();
    let mut buf = vec![0u8; (w * h * 4) as usize];
    view.draw(game, &mut buf, ui);
    image::save_buffer(path, &buf, w, h, image::ColorType::Rgba8).expect("png");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = PathBuf::from(arg(&args, "out").unwrap_or("target/faultprobe"));
    std::fs::create_dir_all(&out).expect("out dir");
    let frames: u64 = arg(&args, "frames").and_then(|s| s.parse().ok()).unwrap_or((THAW_SECONDS * FPS) as u64);
    let shimmer = arg(&args, "shimmer") == Some("1");
    println!("faultprobe frames={frames} shimmer={shimmer} out={}", out.display());

    if let Some(r) = arg(&args, "ring") {
        // The positive control for the hand-off: a ring closed by hand.
        let v: Vec<i32> = r.split(',').filter_map(|s| s.parse().ok()).collect();
        let mut g = Game::new(1);
        let shots: u64 = arg(&args, "shots").and_then(|s| s.parse().ok()).unwrap_or(30);
        for _ in 0..30 {
            g.step();
        }
        pixel_physics::fault::tips::cut_ring(&mut g, v[0], v[1], v[2], v[3]);
        let mut view = View::new();
        let ui = Ui { tool: Tool::Tap, cursor: None, plain: false };
        for f in 0..frames {
            g.step();
            if f.is_multiple_of(shots) {
                let inside = (v[1]..=v[3]).flat_map(|y| (v[0]..=v[2]).map(move |x| (x, y))).filter(|&(x, y)| g.is_ice(x, y)).count();
                println!("f={f} pieces={} released={} slabs={} bodies={} ice_in_ring={} lake_surface={} flow={:.0} failures={:?}", g.counts.pieces_cut, g.counts.pieces_released, g.counts.slabs_freed, g.world.chunk_bodies.len(), inside, g.level.surface, g.flow_now(), g.world.structural_failures);
                png(&g, &mut view, ui, &out.join(format!("ring-f{f:04}.png")));
            }
        }
        return;
    }

    if arg(&args, "repeat") == Some("1") {
        // **Repeatability**: one tap at the same spot, same moment. The
        // same wall twice must give the same crack exactly; ten different
        // walls (different grain) say how far "the same tap" can be read
        // ahead. Rows: where the crack went, as cut edges, the rows it
        // spans, the columns it crossed, and its overlap with wall 1's.
        let seeds: Vec<u64> = (1..=arg(&args, "seeds").and_then(|s| s.parse().ok()).unwrap_or(10)).collect();
        let at = 3 * FPS as u64;
        let crack = |seed: u64| -> (Game, std::collections::BTreeSet<(i32, i32, bool)>) {
            let r = play(Plan::OneTap, seed, false, at, |_, _| {});
            let g = r.game;
            let mut set = std::collections::BTreeSet::new();
            for y in 0..level::H {
                for x in 0..level::W {
                    let c = g.world.get(x, y);
                    if g.is_ice(x, y) && c.crack_right() {
                        set.insert((x, y, true));
                    }
                    if g.is_ice(x, y) && c.crack_down() {
                        set.insert((x, y, false));
                    }
                }
            }
            (g, set)
        };
        let (_, a) = crack(1);
        let (_, b) = crack(1);
        println!("same wall twice: {} vs {} cut edges, identical={}", a.len(), b.len(), a == b);
        let mut view = View::new();
        let ui = Ui { tool: Tool::Tap, cursor: None, plain: true };
        for &seed in &seeds {
            let (g, set) = crack(seed);
            let rows = set.iter().map(|e| e.1);
            let (y0, y1) = (rows.clone().min().unwrap_or(0), rows.max().unwrap_or(0));
            let cols: std::collections::BTreeSet<i32> = set.iter().map(|e| e.0).collect();
            let shared = set.intersection(&a).count();
            let union = set.union(&a).count().max(1);
            println!(
                "repeat seed={seed:<2} edges={:<4} rows={y0}..{y1} cols={}..{} crossed_wall={} overlap_with_wall1={:.0}% stopped_face={} forks={}",
                set.len(),
                cols.iter().next().unwrap_or(&0),
                cols.iter().last().unwrap_or(&0),
                cols.len() as i32 >= level::WALL_X1 - level::WALL_X0 - 4,
                100.0 * shared as f64 / union as f64,
                g.counts.stopped_at_face,
                g.counts.forks
            );
            png(&g, &mut view, ui, &out.join(format!("repeat-s{seed}.png")));
        }
        return;
    }

    if let Some(p) = arg(&args, "plan").and_then(Plan::parse) {
        let seed: u64 = arg(&args, "seed").and_then(|s| s.parse().ok()).unwrap_or(1);
        let every: u64 = arg(&args, "every").and_then(|s| s.parse().ok()).unwrap_or(0);
        let from: u64 = arg(&args, "from").and_then(|s| s.parse().ok()).unwrap_or(0);
        let mut view = View::new();
        let ui = Ui { tool: Tool::Tap, cursor: None, plain: false };
        let dir = out.clone();
        let trace = arg(&args, "trace") == Some("1");
        // `gif=path`: the whole game as an animation, one frame every
        // `gstep` frames, at half size, played back at `gstep` x 60 / 20
        // times real speed.
        let gif = arg(&args, "gif").map(PathBuf::from);
        let gstep: u64 = arg(&args, "gstep").and_then(|s| s.parse().ok()).unwrap_or(30);
        let mut shots: Vec<image::RgbaImage> = Vec::new();
        let mut gview = View::new();
        let (gw, gh) = view::size();
        let mut gbuf = vec![0u8; (gw * gh * 4) as usize];
        let r = play(p, seed, shimmer, frames, |g, f| {
            if gif.is_some() && f.is_multiple_of(gstep) {
                gview.draw(g, &mut gbuf, ui);
                let full = image::RgbaImage::from_raw(gw, gh, gbuf.clone()).expect("frame");
                shots.push(image::imageops::resize(&full, gw / 2, gh / 2, image::imageops::FilterType::Nearest));
            }
            if trace && f >= from && f.is_multiple_of(4) {
                let bodies: Vec<(usize, f32, f32, f32, f32)> = g.world.chunk_bodies.iter().map(|b| (b.cells.len(), b.x, b.y, b.vx, b.vy)).collect();
                println!("f={f} slides={:?} bodies={:?}", g.tips.slides.iter().map(|s| s.cells.len()).collect::<Vec<_>>(), bodies);
            }
            if trace && f.is_multiple_of(60) {
                println!("t={:>3}s surface={} severed={:?} slides={} released={} jammed={}", f / 60, g.level.surface, g.tips.severed.iter().map(|p| (p.cells.len(), p.cells.iter().map(|c| c.1).max().unwrap_or(0))).collect::<Vec<_>>(), g.tips.slides.len(), g.counts.pieces_released, g.counts.jammed);
            }
            if every > 0 && f.is_multiple_of(every) && f >= from {
                png(g, &mut view, ui, &dir.join(format!("{}-s{}-f{:05}.png", p.label(), seed, f)));
            }
        });
        println!("{}", row(&r));
        if let Some(path) = gif {
            let delay = image::Delay::from_numer_denom_ms(50, 1);
            let frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
            let n = frames.len();
            let file = std::fs::File::create(&path).expect("gif file");
            let mut enc = image::codecs::gif::GifEncoder::new_with_speed(file, 10);
            enc.set_repeat(image::codecs::gif::Repeat::Infinite).expect("repeat");
            enc.encode_frames(frames).expect("gif encode");
            println!("gif {} frames={n}", path.display());
        }
        if let Some(d) = arg(&args, "dump") {
            // What is in a box at the end, by material: for "what is that
            // lump hanging in the air".
            let v: Vec<i32> = d.split(',').filter_map(|s| s.parse().ok()).collect();
            let mut by: std::collections::BTreeMap<String, u32> = Default::default();
            for y in v[1]..=v[3] {
                for x in v[0]..=v[2] {
                    let m = r.game.world.get(x, y).material;
                    *by.entry(r.game.world.materials.get(m).name.to_string()).or_default() += 1;
                }
            }
            println!("dump {d}: {by:?}");
        }
        let mut view = View::new();
        png(&r.game, &mut view, ui, &out.join(format!("{}-s{}-end.png", p.label(), seed)));
        return;
    }

    // The paired table.
    let seeds: Vec<u64> = (1..=arg(&args, "seeds").and_then(|s| s.parse().ok()).unwrap_or(10)).collect();
    for plan in [Plan::Nothing, Plan::Random, Plan::Planned] {
        for &seed in &seeds {
            let r = play(plan, seed, shimmer, frames, |_, _| {});
            println!("{}", row(&r));
        }
    }
}
