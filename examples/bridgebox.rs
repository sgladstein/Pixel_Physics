//! **Does this world's trail loop make a collective choice?** -- the
//! equal-branch double bridge (Deneubourg et al. 1990; Goss et al. 1989),
//! the third substrate test of PR 701's follow-up
//! (`/mnt/project-files/emergence-sandbox/plan-2026-10-14.md`; the report's
//! §10 item 1, as redesigned on 2026-10-14).
//!
//! A nest chamber on the left, a food chamber on the right, joined by **two
//! identical tunnels** through rock, `gap=` rows of rock apart. Walkers are
//! **not the shipped ant**: they shuttle nest -> food -> nest, and at every
//! step choose among the open cells beside them with weight
//! `persistence x goal x (k + s)^n`, `s` the **engine's own trail plane**
//! (`Channel::A` by default, `wiring=game` for the shipped pair; deposited
//! through `World::deposit_pheromone` and stepped by the engine's own
//! `Pheromones::step`), `k=`
//! (2) and `n=` (2) Deneubourg's choice constants. `goal` is a mild
//! preference (x2) for steps toward the end the walker is headed for.
//! Every step deposits `dep=` (the engine's `DEPOSIT`) unless `mute=1`.
//!
//! **There is no right answer built into the scene**: the two tunnels are
//! the same length and shape. With the trail on, positive feedback on small
//! early differences should settle the traffic on one tunnel, and which one
//! should differ between seeds. With `mute=1` the traffic should stay near
//! half and half. That is the test of the deposit -> diffuse -> decay ->
//! follow loop. **`gap=` is the test of the report's "scent leaks through
//! rock"**: the planes ignore terrain, so tunnels a row or two apart should
//! share their scent and fail to break symmetry, where tunnels far apart
//! should not.
//!
//! Read per window: crossings of each tunnel (a walker passing the tunnels'
//! midpoint), the upper tunnel's share and the plane's peak, then the
//! per-seed verdict over the last half of the run (`chose` = one tunnel
//! carried more than 80% of at least 100 crossings) and a graded
//! decisiveness, mean |share - 0.5| over the seeds with 100 or more.
//!
//! **Every argument is checked**: an unknown or repeated key panics rather
//! than silently running another arm.
//!
//! **What it found, 2026-10-14** (`goal=5`, 30k frames;
//! `/mnt/project-files/emergence-sandbox/review-pr701.md` §3 E3, corrected
//! by the code review `review-pr702-code.md` F1-F2):
//! - **At the shipped levels both planes sit at the 255 ceiling** in both
//!   tunnels and the rock between, so no arm can choose (0 of 8,
//!   decisiveness 0.008-0.019). The nest scent has no fade
//!   (`TRAIL_A_RHO` 0), so traffic piles it to the ceiling.
//! - **`wiring=game` is a closed loop**: outbound walkers lay nest scent in
//!   the tunnel they chose and homebound walkers follow it, then lay the
//!   food trail there. Kept under the ceiling (`depa=0.05 erho=0.03
//!   depb=0.1`) it chooses on 8 of 16 seeds at gap 10 (muted: 0 of 16) and
//!   6 of 8 at gap 40 -- as well as `homeb=1`, so the wiring is not the
//!   cause. (An earlier version of this header said it was.)
//! - **`reader=presence`, the shipped chooser's read** (`x / (1 + x)` over a
//!   tenth of a deposit, step score `1 + 3 x presence`), chooses on no
//!   wiring, because any used tunnel reads as a full route.
//! - **No reshaping of that read chose either** (8 seeds, gap 10, game
//!   wiring; each 0 of 8, decisiveness 0.004-0.010): a half scaled to the
//!   strongest scent in view (`reader=rel:<c>`, the game's
//!   `PIXEL_PHYSICS_ROUTE_HALF=rel:<c>`), a Hill exponent of 2 or 3
//!   (`rel:<c>:<n>`, `fixed:<deposits>:<n>`), `gain=30` in place of 3, at
//!   the shipped levels and under the ceiling alike. The one exception is
//!   `fixed:1:2 gain=30` under the ceiling, 1 of 8 (0.210). Two reasons,
//!   the first computed and the second inferred: the shipped score's
//!   elasticity, `d ln(weight) / d ln(scent)`, never exceeds 0.64 at any
//!   half, and a two-branch choice needs it above 1 to break symmetry
//!   (Deneubourg's `(k + s)^2` passes 1 once `s > k`); and a walker at
//!   the chamber wall senses two cells, so it never has both tunnel mouths
//!   in view -- a half scaled to the local top cancels the level of the one
//!   mouth it can see.
//!
//! **Goss et al. 1989's unequal bridge, 2026-10-15** (`neck=10 long=40
//! release=13 plane=own k=800 evap=0.0001 goal=5`, 40k frames, 8 seeds;
//! `k=800` is Deneubourg's k = 20 passages at one deposit a step):
//! - **Deneubourg's reader matches Goss.** Short branch over 80% on 4/8,
//!   6/8, 6/8 and 7/8 seeds at 10, 30, 100 and 300 walkers (Goss: 11/14,
//!   rising with colony size as Beekman et al. 2001 found). Short branch
//!   added later (`opens=15000`): the walkers stay on the long one 8/8 at
//!   every size (Goss: 16/18). Muted: 0.56-0.58 short, never chooses.
//! - **The shipped reader (`reader=presence`) reads exactly as muted** on
//!   both tests at 100 walkers (0.57-0.58), as does `fixed:20:2`. The
//!   reader is the block, not traffic or the scene.
//! - Three scene fixes came first, each found by a run that looked like a
//!   result: walkers released all at once make every first choice blind
//!   (`release=`); turning round at a mouth sends a walker back down its
//!   own branch, so two locked groups read as a split (`neck=` puts both
//!   branches beside one fork); a plug mid-branch made a dead end that
//!   trapped walkers in its own scent (the plug is now at the branch ends).
//!
//! **The first engine-plane runs were the harness, not the plane**:
//! `World::step_pheromones` gates on `world.frame`, which nothing here
//! advances, so the planes took a pass every frame. The loop now steps them
//! on its own frame count.

use pixel_physics::sim::pheromone::{Channel, DEPOSIT, PHEROMONE_INTERVAL, SCALE};
use pixel_physics::sim::rng::Rng;
use pixel_physics::sim::{Cell, Rect, World};

const KEYS: &[&str] = &[
    "seed", "seeds", "frames", "walkers", "gap", "len", "k", "n", "dep", "goal", "mute", "plane", "evap", "diffuse",
    "walls", "erho", "ediff", "wiring", "homeb", "gain", "depa", "depb", "brho", "bdiff", "reader", "png", "long", "opens", "release", "turn", "neck",
];

/// Refuse an argument this binary does not read, or one given twice: either
/// would silently run a different arm under the asked-for label.
fn check_args() {
    let mut seen = std::collections::HashSet::new();
    for a in std::env::args().skip(1) {
        let key = a.split_once('=').map_or(a.as_str(), |(k, _)| k).to_string();
        assert!(
            KEYS.contains(&key.as_str()),
            "bridgebox: unknown argument `{a}` (known: {})",
            KEYS.join(", ")
        );
        assert!(seen.insert(key.clone()), "bridgebox: `{key}` given twice");
    }
}

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| {
            v.parse()
                .ok()
                .unwrap_or_else(|| panic!("bridgebox: `{a}` does not parse"))
        })
    })
}

/// How a walker reads the plane it follows (`reader=`).
#[derive(Clone, Copy, Debug)]
enum Reader {
    /// Deneubourg's `(k + s)^n` on the level (the default).
    Level,
    /// The shipped chooser's `trail_presence`: the larger of the next cell
    /// and the one beyond, `x / (1 + x)` over a tenth of a deposit, the step
    /// scored `1 + 3 x presence` (`TRAIL_HALF`, `TRAIL_GAIN`) and picked in
    /// proportion to `(0.1 + score)^2` as `creature::choose_weighted` does.
    Presence,
    /// The same, with the half at `c` times the strongest scent among the
    /// walker's options (never below a tenth of a deposit):
    /// `PIXEL_PHYSICS_ROUTE_HALF=rel:<c>` in the game. `rel:<c>:<n>` raises
    /// the ratio to the `n`th power inside the saturation (a Hill curve,
    /// `x^n / (1 + x^n)`); `n = 1` is the shipped shape.
    Rel(f32, i32),
    /// A fixed half of `d` deposits with Hill exponent `n`:
    /// `fixed:<d>[:<n>]`. The level stays visible when only one branch is
    /// in view, which a half scaled to the local top cannot do.
    Fixed(f32, i32),
}

/// Draw the food trail rather than the nest scent (set under `wiring=game`).
/// `gain=`: the presence readers' `TRAIL_GAIN` (the game's is 3), as f32 bits.
static GAIN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x4040_0000);
static HOME_B_DRAW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static HOME_B: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

const DIRS: [(i32, i32); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];

struct Walker {
    x: i32,
    y: i32,
    dir: usize,
    to_food: bool,
    /// Frames since this walker last stood in the nest chamber: the
    /// odometer `wiring=game`'s nest scent fades by.
    since_nest: u32,
    /// Which tunnel this walker entered on its current leg, if any.
    lane: Option<usize>,
}

fn main() {
    check_args();
    let seeds: Vec<u64> = match arg::<u64>("seed") {
        Some(s) => vec![s],
        None => (1..=arg::<u64>("seeds").unwrap_or(8)).collect(),
    };
    let frames: u64 = arg("frames").unwrap_or(30_000);
    let n_walkers: usize = arg("walkers").unwrap_or(80);
    let gap: i32 = arg("gap").unwrap_or(10);
    let len: i32 = arg("len").unwrap_or(80);
    let k: f32 = arg("k").unwrap_or(2.0);
    let n: i32 = arg("n").unwrap_or(2);
    let dep: f32 = arg("dep").unwrap_or(1.0);
    // How much more a step toward the leg's end weighs than any other.
    let goal_w: f32 = arg("goal").unwrap_or(2.0);
    let mute = arg::<u8>("mute").unwrap_or(0) == 1;
    // `plane=own`: the harness's own trail grid instead of the engine's --
    // deposit and evaporate (`evap=`, per frame), no diffusion and nothing
    // in rock. The positive control for the rule: if the walk breaks
    // symmetry on this grid and not on the engine's plane, the plane is
    // the difference.
    let own = arg::<String>("plane").as_deref() == Some("own");
    let evap: f32 = arg("evap").unwrap_or(0.002);
    // On the own grid: blend toward the 3x3 mean by `diffuse=` every 12
    // frames, as the engine's plane does (0 = none). `walls=1` keeps the
    // blend to open cells (scent cannot enter rock); `walls=0` spreads it
    // through rock as the engine's plane does.
    let diffuse: f32 = arg("diffuse").unwrap_or(0.0);
    let walls = arg::<u8>("walls").unwrap_or(0) == 1;
    // On the engine plane: override channel A's per-pass decay and blend
    // (default: what the game ships, TRAIL_A_RHO 0 and DIFFUSE 0.25).
    let erho: Option<f32> = arg("erho");
    let ediff: Option<f32> = arg("ediff");
    // `wiring=game`: the two trails as the shipped ant wires them
    // (`how-the-ant-works.md` §7), where the default is one trail laid and
    // read both ways (Deneubourg's own bridge). Under `game`:
    // - channel A, the nest scent: laid by every walker on every move at
    //   `T / (T + since_nest)` of `depa=` x DEPOSIT (T = 40 frames, fitted
    //   to the shipped odometer's "78% gone in 141 ticks"), read by walkers
    //   heading home; channel A at the engine's settings (no fade);
    // - channel B, the food trail: laid only by walkers heading home (the
    //   laden leg) at `depb=` x DEPOSIT (the genome's EmitB 0.714), read only
    //   by walkers heading out; at the lab's pair (fade 0.03, blend 0.05)
    //   unless `brho=` / `bdiff=` say otherwise;
    // - only outbound crossings are counted, since the food trail is what
    //   chooses the outbound branch; `mute=1` silences the food trail alone,
    //   which is the report's control (`trailfollow`'s `mute`).
    // The choice is still `(k + s)^n` on the level, not the shipped brain's
    // forward difference: this tests the planes, not the brain.
    let game = arg::<String>("wiring").as_deref() == Some("game");
    // `homeb=1` (game wiring only): walkers heading home read the food
    // trail too, instead of the nest scent. Not the shipped ant; the
    // control for whether the loop through the nest scent is what breaks.
    let home_b = arg::<u8>("homeb").unwrap_or(0) == 1;
    HOME_B.store(home_b, std::sync::atomic::Ordering::Relaxed);
    HOME_B_DRAW.store(game, std::sync::atomic::Ordering::Relaxed);
    // The shipped EmitA starts at 0.819 (`creature.rs`, "0.819 -> 0.010").
    let depa: f32 = arg("depa").unwrap_or(0.819);
    let depb: f32 = arg("depb").unwrap_or(0.714);
    // Set on the world whatever `PIXEL_PHYSICS_B_RHO` / `_B_DIFFUSE` say.
    let brho: f32 = arg("brho").unwrap_or(pixel_physics::sim::pheromone::LAB_B_RHO);
    let bdiff: f32 = arg("bdiff").unwrap_or(pixel_physics::sim::pheromone::LAB_B_DIFFUSE);
    let reader = match arg::<String>("reader").as_deref() {
        None | Some("level") => Reader::Level,
        Some("presence") => Reader::Presence,
        Some(r) => {
            let fixed = r.starts_with("fixed:");
            let body = r
                .strip_prefix("rel:")
                .or_else(|| r.strip_prefix("fixed:"))
                .unwrap_or("");
            let mut it = body.split(':');
            let c = it.next().and_then(|c| c.parse::<f32>().ok());
            let n = it.next().map_or(Some(1), |n| n.parse::<i32>().ok());
            match (c, n) {
                (Some(c), Some(n)) if c > 0.0 && n >= 1 && fixed => Reader::Fixed(c, n),
                (Some(c), Some(n)) if c > 0.0 && n >= 1 => Reader::Rel(c, n),
                _ => panic!("reader={r}: want level, presence, rel:<factor>[:<hill>] or fixed:<deposits>[:<hill>]"),
            }
        }
    };
    if let Some(g) = arg::<f32>("gain") {
        GAIN.store(g.to_bits(), std::sync::atomic::Ordering::Relaxed);
    }
    let png: Option<String> = arg("png");
    // **Goss et al. 1989's unequal bridge.** `long=<rows>`: the lower tunnel
    // dips `rows` rows down and back between a quarter and three quarters
    // of the span, so it is `2 x rows` cells longer than the upper one
    // (`long=40` at `len=80` is their r = 2). `opens=<frame>`: the upper
    // (short) tunnel is shut with rock until that frame -- their "short
    // branch added later", whose colonies stayed on the long one (16/18).
    let long: i32 = arg("long").unwrap_or(0);
    let opens: u64 = arg("opens").unwrap_or(0);
    // `release=<frames>`: walkers leave the nest one every that many frames
    // instead of all at once. Goss et al. 1989's model fed the bridge at
    // 0.5 ant/s against a ~20 s crossing (about ten ants per crossing
    // time), so the first returners' trail is there before most ants
    // choose; a box that releases every walker at frame 0 makes all the
    // first choices blind at once.
    let release: u64 = arg("release").unwrap_or(0);
    // `turn=<cells>`: how far into a chamber a walker goes before turning
    // round (default 2, the mouth). Turning at the mouth sends a walker
    // straight back into the tunnel it came out of, so walkers keep their
    // own branch and two locked groups read as a "split" -- the forks never
    // mix the colony as a nest and a food source do.
    let turn: i32 = arg("turn").unwrap_or(2);
    // `neck=<cells>`: each chamber reaches the tunnels through one tunnel
    // of that length ending in a short shaft joining both, so a walker
    // chooses at a fork with both branches beside it (Goss's and
    // Deneubourg's bridges), not across a chamber where it sees one mouth.
    let neck: i32 = arg("neck").unwrap_or(0);
    let shape = Shape { long, opens, release, turn, neck };
    println!("bridgebox: reader={reader:?} long={long} opens={opens} release={release} turn={turn} neck={neck}");
    println!("bridgebox: homeb={home_b} wiring={} depa={depa} depb={depb} brho={brho} bdiff={bdiff} gap={gap} len={len} walkers={n_walkers} k={k} n={n} goal={goal_w} dep={dep}xDEPOSIT mute={mute} plane={} erho={erho:?} ediff={ediff:?} evap={evap} diffuse={diffuse} walls={walls} frames={frames}", if game { "game" } else { "one-trail" }, if own { "own" } else { "engine" });
    let mut chose = 0;
    let mut decisive = 0.0;
    let mut counted = 0;
    let mut tally = Vec::new();
    let mut whole_tally = Vec::new();
    let mut majority = 0;
    for seed in &seeds {
        let ((share, crossed), (whole, all)) = run(
            *seed,
            frames,
            n_walkers,
            gap,
            len,
            k,
            n,
            dep,
            goal_w,
            mute,
            own,
            evap,
            diffuse,
            walls,
            erho,
            ediff,
            game.then_some((depa, depb, brho, bdiff)),
            reader,
            png.as_deref(),
            shape,
        );
        if crossed > 0 && share > 0.5 {
            majority += 1;
        }
        whole_tally.push(format!("{whole:.2}/{all}"));
        // A seed only chooses if it is still crossing: a jammed run's 0/0
        // read as a share of 0 and once passed for a choice.
        if crossed >= 100 && !(0.2..=0.8).contains(&share) {
            chose += 1;
        }
        if crossed >= 100 {
            decisive += (share - 0.5).abs();
            counted += 1;
        }
        tally.push(format!("{share:.2}/{crossed}"));
    }
    println!("SUMMARY gap={gap} mute={mute}: upper share, last half, (share/crossings) per seed [{}]; chose one tunnel (>80%) on {chose} of {}; decisiveness mean|share-0.5| {:.3} over {counted} seeds with 100+ crossings", tally.join(" "), seeds.len(), decisive / counted.max(1) as f32);
    if long > 0 || opens > 0 {
        println!("SUMMARY long={long} opens={opens}: the upper tunnel is the short one; upper share over the whole run [{}]; upper (short) majority in the last half on {majority} of {}", whole_tally.join(" "), seeds.len());
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    seed: u64,
    frames: u64,
    n_walkers: usize,
    gap: i32,
    len: i32,
    k: f32,
    n: i32,
    dep: f32,
    goal_w: f32,
    mute: bool,
    own: bool,
    evap: f32,
    diffuse: f32,
    walls: bool,
    erho: Option<f32>,
    ediff: Option<f32>,
    game: Option<(f32, f32, f32, f32)>,
    reader: Reader,
    png: Option<&str>,
    shape: Shape,
) -> ((f32, u64), (f32, u64)) {
    // Geometry: chamber | tunnels | chamber, all inside rock.
    let ch_w = 16;
    let tunnel_h = 2;
    let w = 4 + ch_w + len + ch_w + 4;
    let ch_h = gap + 2 * tunnel_h + 8;
    let h = ch_h + 8 + shape.long;
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    let stone = world.materials.id_of("stone").expect("stone");
    if let Some(r) = erho {
        world.pheromones.set_channel_rho(Channel::A, r);
    }
    if let Some(d) = ediff {
        world.pheromones.set_channel_diffuse(Channel::A, d);
    }
    let home_b = HOME_B.load(std::sync::atomic::Ordering::Relaxed);
    if let Some((_, _, brho, bdiff)) = game {
        world.pheromones.set_channel_rho(Channel::B, brho);
        world.pheromones.set_channel_diffuse(Channel::B, bdiff);
    }
    for y in 0..h {
        for x in 0..w {
            world.set(x, y, Cell::new(stone, 0).with_attached(true));
        }
    }
    let (nest_x0, nest_x1) = (4, 4 + ch_w - 1);
    let (food_x0, food_x1) = (4 + ch_w + len, 4 + ch_w + len + ch_w - 1);
    let (ch_y0, ch_y1) = (4, 4 + ch_h - 1);
    let up_y0 = ch_y0 + 4;
    let lo_y0 = up_y0 + tunnel_h + gap;
    let mut carve = |x0: i32, y0: i32, x1: i32, y1: i32| {
        for y in y0..=y1 {
            for x in x0..=x1 {
                world.set(x, y, Cell::EMPTY);
            }
        }
    };
    carve(nest_x0, ch_y0, nest_x1, ch_y1);
    carve(food_x0, ch_y0, food_x1, ch_y1);
    let mid = (nest_x1 + food_x0) / 2;
    // With a neck, the branches start at the fork shafts.
    let (x_a, x_b) = (nest_x1 + 1 + shape.neck, food_x0 - 1 - shape.neck);
    carve(x_a, up_y0, x_b, up_y0 + tunnel_h - 1);
    if shape.neck > 0 {
        let neck_y = (up_y0 + lo_y0) / 2;
        for (x0, x1, shaft) in [(nest_x1 + 1, x_a, x_a), (x_b, food_x0 - 1, x_b - tunnel_h + 1)] {
            carve(x0, neck_y, x1, neck_y + tunnel_h - 1);
            carve(shaft, up_y0, shaft + tunnel_h - 1, lo_y0 + tunnel_h - 1);
        }
    }
    if shape.long > 0 {
        // Out along the lower row to a quarter of the span, down, along the
        // bottom past the midpoint, and back up at three quarters.
        let (a, b) = (nest_x1 + 1 + len / 4, nest_x1 + 1 + 3 * len / 4);
        let bot = lo_y0 + shape.long;
        carve(x_a, lo_y0, a + tunnel_h - 1, lo_y0 + tunnel_h - 1);
        carve(a, lo_y0, a + tunnel_h - 1, bot + tunnel_h - 1);
        carve(a, bot, b + tunnel_h - 1, bot + tunnel_h - 1);
        carve(b, lo_y0, b + tunnel_h - 1, bot + tunnel_h - 1);
        carve(b, lo_y0, x_b, lo_y0 + tunnel_h - 1);
    } else {
        carve(x_a, lo_y0, x_b, lo_y0 + tunnel_h - 1);
    }
    // The plug for `opens=`: rock across both ends of the short tunnel,
    // so until it opens the branch is absent, as in Goss's design. A plug
    // in the middle made it a dead end that held its own scent and
    // trapped the walkers in it (second-half crossings 0-3 on most seeds).
    let (p_a, p_b) = (x_a + tunnel_h, x_b - tunnel_h - 1);
    let plug: Vec<(i32, i32)> = [p_a, p_a + 1, p_b, p_b + 1]
        .into_iter()
        .flat_map(|x| (up_y0..up_y0 + tunnel_h).map(move |y| (x, y)))
        .collect();
    if shape.opens > 0 {
        for &(x, y) in &plug {
            world.set(x, y, Cell::new(stone, 0).with_attached(true));
        }
    }
    let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xb21d6e);
    let mut walkers: Vec<Walker> = (0..n_walkers)
        .map(|_| Walker {
            x: nest_x0 + rng.below(ch_w as u32) as i32,
            y: ch_y0 + rng.below(ch_h as u32) as i32,
            dir: rng.below(8) as usize,
            to_food: true,
            since_nest: 0,
            lane: None,
        })
        .collect();
    let open = |wd: &World, x: i32, y: i32| wd.in_bounds(x, y) && wd.is_empty(x, y);
    let lane_of = |y: i32| if y < up_y0 + tunnel_h { 0 } else { 1 };
    let window = (frames / 10).max(1);
    let mut grid = vec![0f32; (w * h) as usize];
    let gi = |x: i32, y: i32| (y * w + x) as usize;
    let mut win = [0u64; 2];
    let mut late = [0u64; 2];
    let mut whole = [0u64; 2];
    for f in 1..=frames {
        if shape.opens > 0 && f == shape.opens {
            for &(x, y) in &plug {
                world.set(x, y, Cell::EMPTY);
            }
        }
        if own {
            let keep = 1.0 - evap;
            for v in grid.iter_mut() {
                *v *= keep;
            }
            if diffuse > 0.0 && f % 12 == 0 {
                let src = grid.clone();
                for y in 1..h - 1 {
                    for x in 1..w - 1 {
                        let here_open = world.is_empty(x, y);
                        if walls && !here_open {
                            continue;
                        }
                        let (mut sum, mut cnt) = (0.0, 0.0);
                        for dy in -1..=1 {
                            for dx in -1..=1 {
                                if walls && !world.is_empty(x + dx, y + dy) {
                                    continue;
                                }
                                sum += src[gi(x + dx, y + dy)];
                                cnt += 1.0;
                            }
                        }
                        let here = src[gi(x, y)];
                        grid[gi(x, y)] = here + (sum / cnt - here) * diffuse;
                    }
                }
            }
        } else {
            // Stepped on the harness's own frame count: `step_pheromones`
            // gates on `world.frame`, which nothing here advances, so it
            // ran a pass every frame -- twelve times the game's rate. That
            // was the first engine-plane result, and it was the harness.
            world.pheromones.step(f, PHEROMONE_INTERVAL);
        }
        for (i, wk) in walkers.iter_mut().enumerate() {
            if (i as u64) * shape.release >= f {
                continue;
            }
            let (x, y) = (wk.x, wk.y);
            // Turn round at either end.
            if wk.to_food && x >= food_x0 + shape.turn {
                wk.to_food = false;
                wk.lane = None;
            } else if !wk.to_food && x <= nest_x1 - shape.turn {
                wk.to_food = true;
                wk.lane = None;
            }
            if x <= nest_x1 {
                wk.since_nest = 0;
            } else {
                wk.since_nest = wk.since_nest.saturating_add(1);
            }
            let read = if game.is_some() && (wk.to_food || home_b) {
                Channel::B
            } else {
                Channel::A
            };
            let level_at = |wd: &World, d: usize| -> f32 {
                let (dx, dy) = DIRS[d];
                if own {
                    grid[gi(x + dx, y + dy)] * SCALE as f32
                } else {
                    let near = wd.pheromone_at(read, x + dx, y + dy);
                    match reader {
                        Reader::Level => f32::from(near),
                        _ => f32::from(near.max(wd.pheromone_at(read, x + 2 * dx, y + 2 * dy))),
                    }
                }
            };
            let tenth = DEPOSIT as f32 / 10.0;
            let half = match reader {
                Reader::Rel(c, _) => {
                    let top = (0..8)
                        .filter(|&d| open(&world, x + DIRS[d].0, y + DIRS[d].1))
                        .map(|d| level_at(&world, d))
                        .fold(0.0, f32::max);
                    (c * top).max(tenth)
                }
                Reader::Fixed(d, _) => d * DEPOSIT as f32,
                _ => tenth,
            };
            let mut weights = [0f32; 8];
            let mut total = 0.0;
            for (d, &(dx, dy)) in DIRS.iter().enumerate() {
                let (nx, ny) = (x + dx, y + dy);
                if !open(&world, nx, ny) {
                    continue;
                }
                let turn = (d + 8 - wk.dir) % 8;
                let persist = [1.0, 0.5, 0.15, 0.03, 0.01, 0.03, 0.15, 0.5][turn];
                let goal = if (dx > 0) == wk.to_food && dx != 0 { goal_w } else { 1.0 };
                let raw = level_at(&world, d);
                let wgt = match reader {
                    Reader::Level => persist * goal * (k + raw / SCALE as f32).powi(n),
                    // The game's `choose_weighted` picks in proportion to
                    // `(0.1 + score)^2`, so the shipped reader is squared too.
                    Reader::Presence | Reader::Rel(..) | Reader::Fixed(..) => {
                        let hill = match reader {
                            Reader::Rel(_, n) | Reader::Fixed(_, n) => n,
                            _ => 1,
                        };
                        let xh = (raw / half).powi(hill);
                        let gain = f32::from_bits(GAIN.load(std::sync::atomic::Ordering::Relaxed));
                        (0.1 + persist * goal * (1.0 + gain * xh / (1.0 + xh))).powi(2)
                    }
                };
                weights[d] = wgt;
                total += wgt;
            }
            let mut moved = false;
            if total > 0.0 {
                let mut r = rng.unit_f32() * total;
                for (d, &wgt) in weights.iter().enumerate() {
                    if wgt <= 0.0 {
                        continue;
                    }
                    if r < wgt {
                        wk.x += DIRS[d].0;
                        wk.y += DIRS[d].1;
                        wk.dir = d;
                        moved = true;
                        break;
                    }
                    r -= wgt;
                }
            } else {
                wk.dir = rng.below(8) as usize;
            }
            // The game lays only on a successful move.
            if !moved {
                // nothing laid
            } else if let Some((depa, depb, _, _)) = game {
                let odo = 40.0 / (40.0 + wk.since_nest as f32);
                world.deposit_pheromone(Channel::A, wk.x, wk.y, (depa * odo * DEPOSIT as f32) as u16);
                if !mute && !wk.to_food {
                    world.deposit_pheromone(Channel::B, wk.x, wk.y, (depb * DEPOSIT as f32) as u16);
                }
            } else if !mute {
                if own {
                    grid[gi(wk.x, wk.y)] += dep * (DEPOSIT / SCALE) as f32;
                } else {
                    world.deposit_pheromone(Channel::A, wk.x, wk.y, (dep * DEPOSIT as f32) as u16);
                }
            }
            // A crossing: reaching the tunnels' midpoint heading the right way.
            let in_tunnel = wk.x > nest_x1 && wk.x < food_x0;
            if in_tunnel && wk.x == mid && wk.lane.is_none() && (game.is_none() || wk.to_food) {
                let l = lane_of(wk.y);
                wk.lane = Some(l);
                win[l] += 1;
                whole[l] += 1;
                if f > frames / 2 {
                    late[l] += 1;
                }
            }
        }
        if f % window == 0 {
            let t = (win[0] + win[1]).max(1);
            // The plane's peak, in units of 255: a trail pinned at the
            // ceiling on both tunnels reads equal and cannot choose.
            let peak = if own {
                grid.iter().cloned().fold(0.0, f32::max)
            } else {
                world
                    .pheromones
                    .plane(if game.is_some() { Channel::B } else { Channel::A })
                    .max() as f32
                    / SCALE as f32
            };
            println!(
                "seed {seed} f {f:>6}: crossings upper {} lower {} (upper share {:.2}) peak {peak:.1}",
                win[0],
                win[1],
                win[0] as f32 / t as f32
            );
            win = [0, 0];
        }
    }
    if let Some(dir) = png {
        let _ = std::fs::create_dir_all(dir);
        draw(&world, &walkers, w, h, stone)
            .save(format!(
                "{dir}/{}_homeb{}_depa{}_depb{}_dep{dep}_gap{gap}_mute{}_s{seed}.png",
                if game.is_some() { "game" } else { "onetrail" },
                u8::from(home_b),
                game.map_or(0.0, |g| g.0),
                game.map_or(0.0, |g| g.1),
                u8::from(mute)
            ))
            .expect("png");
    }
    (
        (late[0] as f32 / (late[0] + late[1]).max(1) as f32, late[0] + late[1]),
        (whole[0] as f32 / (whole[0] + whole[1]).max(1) as f32, whole[0] + whole[1]),
    )
}

/// The bridge's shape beyond the equal pair: `long=` and `opens=`.
#[derive(Clone, Copy)]
struct Shape {
    long: i32,
    opens: u64,
    release: u64,
    turn: i32,
    neck: i32,
}

fn draw(wd: &World, walkers: &[Walker], w: i32, h: i32, stone: pixel_physics::sim::MaterialId) -> image::RgbaImage {
    const SCALE_PX: u32 = 4;
    let mut img = image::RgbaImage::from_pixel(w as u32 * SCALE_PX, h as u32 * SCALE_PX, image::Rgba([0, 0, 0, 255]));
    for y in 0..h {
        for x in 0..w {
            let s = (wd.pheromone_at(
                if HOME_B_DRAW.load(std::sync::atomic::Ordering::Relaxed) {
                    Channel::B
                } else {
                    Channel::A
                },
                x,
                y,
            ) as f32
                / 65535.0)
                .sqrt();
            let c = if wd.get(x, y).material == stone {
                // Scent in rock drawn too: the leak is the point.
                [(70.0 + 150.0 * s) as u8, 70, 75, 255]
            } else {
                [(30.0 + 220.0 * s) as u8, (30.0 + 120.0 * s) as u8, 40, 255]
            };
            for dy in 0..SCALE_PX {
                for dx in 0..SCALE_PX {
                    img.put_pixel(x as u32 * SCALE_PX + dx, y as u32 * SCALE_PX + dy, image::Rgba(c));
                }
            }
        }
    }
    for wk in walkers {
        for dy in 0..SCALE_PX {
            for dx in 0..SCALE_PX {
                img.put_pixel(
                    wk.x as u32 * SCALE_PX + dx,
                    wk.y as u32 * SCALE_PX + dy,
                    image::Rgba(if wk.to_food {
                        [80, 200, 255, 255]
                    } else {
                        [255, 255, 255, 255]
                    }),
                );
            }
        }
    }
    img
}
