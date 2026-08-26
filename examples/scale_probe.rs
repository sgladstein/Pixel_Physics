//! What a bigger world actually costs, measured rather than extrapolated.
//!
//! **Written when the owner had asked for a 4x-linear world (8192x2560,
//! sixteen times the cells) and it was still a proposal; it now ships** —
//! `app::WORLD_WIDTH`/`WORLD_HEIGHT` are 8192x2560. At the time, every
//! number available for that decision was an *extrapolation* from the then-
//! shipped 2048x640 size — generation scales with area, so 542 ms becomes
//! ~8.7 s; the field solve was 7.2 ms at 2048x1280 and touches every field
//! cell in the world every frame, so ~57 ms. Extrapolations across a 16x
//! range are exactly the kind of number this repo keeps being wrong about,
//! and the whole of the scale plan's first phase was sized off them — this
//! tool is what replaced the extrapolation with a measurement.
//!
//! So: build the thing and time it. Reports, per size:
//!
//! - **generation**, split into the two halves that matter — the pass table
//!   (`generate_only`) and `structural::compute_world_distances`, which a
//!   review put at 45% of the total and which allocates a second full-grid
//!   mirror.
//! - **peak RSS**, read from `/proc/self/status: VmHWM`, so the transient
//!   generation spike is visible and not just the steady grid.
//! - **frame cost**, split into the CA sweep (`parallel::step`, the driver
//!   the app runs) and `field::step`, separately while the world is still
//!   settling and once it has gone quiet. The settled number is the one that
//!   decides whether a big world is playable, because a world that costs
//!   57 ms/frame with nothing happening is not a world.
//!
//! ```text
//! cargo run --release --example scale_probe                  # 1x, 2x, 3x, 4x
//! cargo run --release --example scale_probe -- scales=1,4
//! cargo run --release --example scale_probe -- size=8192x2560 frames=120
//! ```
//!
//! `scales=` multiplies `BASE_W`/`BASE_H` below, which are still 2048x640 —
//! so the default `1,2,3,4` now tops out at `4` = 8192x2560, the size that
//! ships, rather than at some multiple beyond it. To measure past the
//! shipped size, `size=WxH` takes an explicit size directly and ignores
//! `scales` entirely.

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::world::World;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::particle::{self, ParticleSystem};
use pixel_physics::sim::{field, parallel, player, rigid, structural, weather};
use pixel_physics::worldgen::{self, Spec, WorldgenPresets};
use std::time::Instant;

/// How still counts as still, and for how long. The pressure epsilon is the
/// engine's own notion of "not changing"; `QUIET_RUN` frames of it in a row
/// is what makes the verdict robust to a field that is only stepped on some
/// frames.
const QUIET_MS: f64 = 1.0;
const QUIET_RUN: usize = 120;

/// The size every `scales=` multiplier is taken against. **Was** the
/// shipped size when this was written; the world has grown since and this
/// was deliberately left unmoved (see the module doc), so `scales=4` now
/// lands exactly on the shipped 8192x2560 rather than a size beyond it.
/// `size=WxH` is the way to probe past that without touching this.
/// Brush radius for the `strike` and `mine` load components.
///
/// `App::brush_radius`'s own default -- the app scales both verbs off the
/// brush deliberately ("the tool the player is already sizing is the tool
/// that decides how hard they hit"), so a probe that picks its own number is
/// measuring a swing nobody can make.
const DIG_RADIUS: i32 = 6;

const BASE_W: i32 = 2048;
const BASE_H: i32 = 640;

/// Peak resident set size in MiB, from the kernel rather than from a guess.
///
/// `VmHWM` is the high-water mark, which is the point: the generation spike
/// (`compute_world_distances` holds a second `Vec<Cell>` the size of the
/// whole grid, plus a `Vec<u16>` of distances) is invisible to any
/// steady-state accounting and is what decides whether a size fits.
fn peak_rss_mib() -> f64 {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else { return f64::NAN };
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            if let Some(kb) = rest.split_whitespace().next().and_then(|t| t.parse::<f64>().ok()) {
                return kb / 1024.0;
            }
        }
    }
    f64::NAN
}

fn main() {
    let mut scales: Vec<i32> = vec![1, 2, 3, 4];
    let mut explicit: Option<(i32, i32)> = None;
    let mut frames = 240usize;
    let mut preset = String::new();
    let mut seed = 1u64;
    // `phases=1` switches this probe from "what does a bigger world cost"
    // to "where does one frame go" -- see `phase_probe` below.
    let mut phases = false;
    // `load=` puts a *subject* in the world -- see `phase_probe`'s doc on why
    // the default (an empty world) measures the wrong frame.
    let mut load = String::new();
    // `chain=SPREAD|LOCAL|TIGHT|NONE` -- `structural::CHAIN_MODES`, the
    // runtime selector for how far a structural failure propagates. The
    // default is `CHAIN_MODES[0]`, SPREAD, whose reach is `i32::MAX`.
    let mut chain = String::new();
    // Frames run before measurement starts, discarded. Terrain is at rest by
    // frame 7, but light and moisture start at zero on a fresh world and have
    // to fill it once; `field_cost.rs` uses the same 1500 for the same reason.
    let mut warm = 1500usize;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "scales" => scales = v.split(',').map(|t| t.parse().expect("scales=1,2,4")).collect(),
            "phases" => phases = v != "0",
            "load" => load = v.to_string(),
            "chain" => chain = v.to_string(),
            "warm" => warm = v.parse().expect("warm=N"),
            "size" => {
                let (w, h) = v.split_once('x').expect("size=WxH");
                explicit = Some((w.parse().expect("width"), h.parse().expect("height")));
            }
            "frames" => frames = v.parse().expect("frames=N"),
            "preset" => preset = v.to_string(),
            "seed" => seed = v.parse().expect("seed=N"),
            _ => eprintln!("ignoring unknown argument {arg}"),
        }
    }

    let (presets, err) = WorldgenPresets::load();
    if let Some(e) = err {
        eprintln!("preset load: {e}");
    }
    let name = if preset.is_empty() { presets.default_name() } else { preset.clone() };
    let params = presets.get(&name).unwrap_or_else(|| panic!("unknown preset {name:?}")).clone();

    let sizes: Vec<(i32, i32)> = match explicit {
        Some(wh) => vec![wh],
        None => scales.iter().map(|&s| (BASE_W * s, BASE_H * s)).collect(),
    };

    if phases {
        let (w, h) = explicit.unwrap_or((BASE_W * 4, BASE_H * 4));
        phase_probe(ProbeArgs { w, h, params: &params, name: &name, seed, warm, frames, load: &load, chain: &chain });
        return;
    }

    println!("preset {name}, seed {seed}, {frames} frames per size\n");
    println!(
        "{:>11} {:>10} {:>9} {:>10} {:>9} {:>9} {:>9} {:>9}   settled mean/worst, and awake chunks",
        "size", "cells", "place", "structural", "gen", "peakRSS", "sweep!", "field!"
    );
    println!("{}", "-".repeat(140));

    for (w, h) in sizes {
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));

        let t = Instant::now();
        worldgen::generate_only(&mut world, Spec::Generated { params: &params, seed });
        let place_ms = t.elapsed().as_secs_f64() * 1000.0;

        let t = Instant::now();
        structural::compute_world_distances(&mut world);
        let structural_ms = t.elapsed().as_secs_f64() * 1000.0;

        // A checksum of the finished world, so an "optimisation" can be held
        // to producing the *same world*. Tests passing is not that: the suite
        // asserts properties, and a distance field that is subtly different
        // but still plausible passes every one of them. Two builds printing
        // the same number is the claim worth making.
        if std::env::var("WORLD_HASH").is_ok() {
            let mut acc: u64 = 0xcbf2_9ce4_8422_2325;
            for y in 0..h {
                for x in 0..w {
                    let c = world.get(x, y);
                    for byte in [
                        c.material.0 as u64,
                        c.aux() as u64,
                        c.shade as u64,
                        u64::from(c.organism_id()),
                    ] {
                        acc ^= byte;
                        acc = acc.wrapping_mul(0x100_0000_01b3);
                    }
                }
            }
            println!("  world hash {w}x{h}: {acc:016x}");
        }

        // **Step until the world is genuinely quiet, and report how long that
        // took**, rather than stepping a fixed count and calling the result
        // "settled".
        //
        // The first version of this probe did the latter and got it badly
        // wrong: it ran 400 frames, reported 8.26 ms as the settled field
        // cost at the shipped size, and that number went into a report as a
        // standing defect. It was the *transient*. A generated world's field
        // takes thousands of frames to converge -- pressure decays cleanly
        // but slowly -- and 400 frames is nowhere near it. `frames` is now a
        // cap, not a target, and `to_quiet` is the headline number: it is how
        // long a player waits before the world stops costing what it costs
        // here.
        let (mut still, mut worst_sweep, mut worst_field) = (0usize, 0.0f64, 0.0f64);
        let mut to_quiet: Option<usize> = None;
        for i in 0..frames {
            let t = Instant::now();
            parallel::step(&mut world);
            worst_sweep = worst_sweep.max(t.elapsed().as_secs_f64() * 1000.0);
            let t = Instant::now();
            field::step(&mut world);
            let field_ms = t.elapsed().as_secs_f64() * 1000.0;
            worst_field = worst_field.max(field_ms);
            // **Quiet is measured by the cost itself, after two other
            // definitions of it turned out to be blind.**
            //
            // `world.fields_settled()` latched true at frame 47 while the
            // field was demonstrably still churning -- it is a verdict over
            // the tiles actually solved that frame, so an empty or lucky
            // solve set reads as "everything settled". And a lattice of
            // `field_at_bilinear` probe points reported quiet at frame 0,
            // because points spread evenly over a world mostly land in solid
            // rock where the pressure never moves at all. Both were asking a
            // question that can be answered vacuously.
            //
            // The wall time cannot be. Measured, the two states are 40x
            // apart -- 42 ms/frame while converging against 0.01 ms once
            // converged -- so a 1 ms threshold separates them at every world
            // size, and `QUIET_RUN` consecutive frames under it is robust to
            // a field that only steps on some frames.
            if field_ms < QUIET_MS {
                still += 1;
            } else {
                still = 0;
            }
            if to_quiet.is_none() && still >= QUIET_RUN {
                to_quiet = Some(i + 1 - QUIET_RUN);
            }
        }

        // Then the state the optimisation exists for: nothing moving. A
        // settled world is where the dirty-rect skip and the field's own
        // early-out do their work, and where a whole-world-per-frame pass
        // has nowhere to hide.
        //
        // **Split, and reported next to the awake-chunk count**, because the
        // first version of this probe reported 11 ms "settled" at the shipped
        // size against a recorded 0.008 ms, and a number that disagrees with
        // the record by three orders of magnitude is a question about the
        // probe first. Without the awake count there is no way to tell a slow
        // settled world from a world that never settled.
        let (mut worst_ss, mut worst_sf) = (0.0f64, 0.0f64);
        let (mut sum_ss, mut sum_sf) = (0.0f64, 0.0f64);
        for _ in 0..60 {
            let t = Instant::now();
            parallel::step(&mut world);
            let d = t.elapsed().as_secs_f64() * 1000.0;
            worst_ss = worst_ss.max(d);
            sum_ss += d;
            let t = Instant::now();
            field::step(&mut world);
            let d = t.elapsed().as_secs_f64() * 1000.0;
            worst_sf = worst_sf.max(d);
            sum_sf += d;
        }
        let awake = world.active_chunk_count();
        let quiet = match to_quiet {
            Some(f) => format!("{f}"),
            None => format!(">{frames}"),
        };

        println!(
            "{:>11} {:>10} {:>8.0}ms {:>9.0}ms {:>8.0}ms {:>7.0}MiB {:>7.2}ms {:>7.2}ms | \
             sweep {:>6.2}/{:<6.2} field {:>6.2}/{:<6.2} awake {} quiet@{}",
            format!("{w}x{h}"),
            w as i64 * h as i64,
            place_ms,
            structural_ms,
            place_ms + structural_ms,
            peak_rss_mib(),
            worst_sweep,
            worst_field,
            sum_ss / 60.0,
            worst_ss,
            sum_sf / 60.0,
            worst_sf,
            awake,
            quiet,
        );
    }

    println!(
        "\nplace = the pass table; structural = compute_world_distances (allocates a second\n\
         full grid); sweep/field = worst single frame over the run; settled = worst frame\n\
         of both together once the world has gone quiet. peakRSS is cumulative across sizes\n\
         in one process, so read it from the largest row or run one size at a time."
    );
}

/// A per-phase cost sample set: enough to report mean, p90 and worst.
///
/// **p90 and not just mean**, because a phase's whole character here is how
/// bursty it is: the field is free on most frames and enormous on a sky step,
/// and a mean over both says neither number. **Worst as well as p90**, because
/// the 16.6 ms budget is a per-frame bound, not an average one -- a phase that
/// means 2 ms and peaks at 70 has already dropped a frame.
#[derive(Default)]
struct Samples {
    name: &'static str,
    ms: Vec<f64>,
}

impl Samples {
    fn new(name: &'static str) -> Self {
        Self { name, ms: Vec::new() }
    }
    fn total(&self) -> f64 {
        self.ms.iter().sum()
    }
    fn mean(&self) -> f64 {
        if self.ms.is_empty() { 0.0 } else { self.total() / self.ms.len() as f64 }
    }
    /// Sorts a copy rather than the samples themselves -- the caller reads
    /// several statistics off one set and a destructive sort would make the
    /// second read a lie.
    fn quantile(&self, q: f64) -> f64 {
        if self.ms.is_empty() {
            return 0.0;
        }
        let mut v = self.ms.clone();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[(((v.len() - 1) as f64) * q).round() as usize]
    }
    fn worst(&self) -> f64 {
        self.ms.iter().copied().fold(0.0f64, f64::max)
    }
}

/// One frame's total, bucketed by the two designed oscillators.
#[derive(Default, Clone, Copy)]
struct Bucket {
    frames: usize,
    total: f64,
    worst: f64,
}

impl Bucket {
    fn add(&mut self, ms: f64) {
        self.frames += 1;
        self.total += ms;
        self.worst = self.worst.max(ms);
    }
    fn mean(&self) -> f64 {
        if self.frames == 0 { 0.0 } else { self.total / self.frames as f64 }
    }
}

/// **Where one frame of the real app actually goes, at the shipped world
/// size.** `phases=1`.
///
/// # Why this exists
///
/// Every frame-cost number in this repo before it was measured on *part* of a
/// frame, and the three that existed were taken at three different world
/// sizes: `ascii` times the CA sweep at 512x320, `field_cost` times the field
/// at 8192x2560, and this probe's own default mode times sweep-plus-field.
/// Nothing timed `App::update`. So "the field is the problem" was a reading
/// off two numbers that had never been put beside the other eight phases, and
/// the load model's own §1j measurement (118 ms) had no frame to be a share
/// *of*.
///
/// This runs `App::update`'s exact phase order (`src/app.rs`) and times each
/// phase separately, bucketing whole frames by sky-step and gust the way
/// `field_cost` does -- because a window shorter than a day/night cycle
/// samples a designed oscillator at an arbitrary phase, and three 600-frame
/// windows on one world once gave 0.00, 4.98 and 7.04 ms/frame.
///
/// # What it can answer beyond the question it was built for
///
/// - **Any "is this phase worth optimising" question**, since it prints each
///   phase's share of the frame. A phase at 2% cannot repay work whatever its
///   own internal cost looks like.
/// - **Whether a *new* per-frame subsystem fits**, by adding it to the list
///   and reading its share against the budget before it ships.
/// - **The idle cost of a loaded world**, which is what a player experiences
///   for most of a session and what M10's streaming has to hold down as
///   resident chunks outnumber active ones.
///
/// # `load=` -- and why the default measures the wrong frame
///
/// With no `load=`, this is a *generated world left alone*: no player, no
/// blasts, no creatures, nothing digging. That is a real regime -- it is what
/// a whole-world-per-frame pass has nowhere to hide in -- but it is **not the
/// regime the app is judged in**, and reading it as if it were is a mistake
/// this probe made on its first outing.
///
/// The phases that need a subject report their *empty-case* cost without one:
/// `player`, `blasts`, `splashes`, `particles` and `rigid bodies` all read
/// ~0.00 ms because nothing was happening, and it is easy to conclude they do
/// not matter. The repo's own record says otherwise -- `open-bugs-handoff.md`
/// §1j measured the load model at a **118 ms worst frame** on a destruction
/// scene, and `dead-ends.md` carries the owner's playtest report that "when
/// something big breaks into lots of little pieces, the performance gets
/// bad". Saving milliseconds on the idle frame while a blast frame costs a
/// hundred is optimising the half that was already playable.
///
/// So `load=` puts a subject in the world. Comma-separated, any combination:
///
/// | | |
/// |---|---|
/// | `ants:N` | plant N ants along the surface -- `creature::tick` scheduling, pheromones, foraging |
/// | `gnome` | summon the player and drive him: run right, jump every second |
/// | `blast:EVERY[:COUNT]` | fire an `explosion::trigger` at the surface every EVERY frames -- debris, fracture, chunk bodies, the load model, the field impulse. `COUNT` stops after that many, which is the *only* way to ask whether a blast's cost ends: with charges still arriving, a queue that never drains and one that drains slower than it fills look identical |
/// | `strike:EVERY[:COUNT]` | swing `rigid::strike` at the surface every EVERY frames -- the *hammer*, at the app's own `brush_radius * STRIKE_FORCE_PER_RADIUS` |
/// | `mine:EVERY[:COUNT]` | cut with `rigid::mine_swept` every EVERY frames -- the *pick*, the verb a player spends most of their time in |
///
/// **`strike` and `mine` exist because §S is not an explosion bug.** Reading
/// the three destructive verbs, `World::paint_capsule` is the only one that
/// pays for `structural::relax_region`; `rigid::strike` and
/// `rigid::mine_swept` both stop at `record_disturbance` plus
/// `schedule_structural_check_around` and hand the whole correction to the
/// reactive wavefront, exactly as the explosion does. Whether that leaks at a
/// pick's radius the way it does at a charge's is a question about *ordinary
/// play*, and only a counter answers it.
/// | `all` | `ants:64,gnome,blast:600` |
///
/// **Read the two runs as a pair.** The point is not the loaded number on its
/// own; it is which phases move between idle and loaded, because that is what
/// says where the optimisation effort belongs. A phase that is 0.00 ms idle
/// and 40 ms under a blast is the whole game.
/// Everything `phases=1` was invoked with, in one place. A struct rather than
/// nine positional arguments because the list grows every time a question
/// needs a new knob (`load=` and `chain=` are both recent), and a run whose
/// parameters cannot be printed back is a run nobody can tell is
/// misconfigured -- see the megastudy that produced 24 logs of 3 populations
/// because `worldseed=` post-dated the binary.
struct ProbeArgs<'a> {
    w: i32,
    h: i32,
    params: &'a pixel_physics::worldgen::WorldgenParams,
    name: &'a str,
    seed: u64,
    warm: usize,
    frames: usize,
    load: &'a str,
    chain: &'a str,
}

fn phase_probe(args: ProbeArgs) {
    let ProbeArgs { w, h, params, name, seed, warm, frames, load, chain } = args;
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    let t = Instant::now();
    worldgen::generate_only(&mut world, Spec::Generated { params, seed });
    structural::compute_world_distances(&mut world);
    let gen_ms = t.elapsed().as_secs_f64() * 1000.0;

    let mut particles = ParticleSystem::default();
    let mut blasts = Blasts::default();
    let tuning = player::Tuning::default();

    for _ in 0..warm {
        parallel::step(&mut world);
        world.step_fields();
    }

    // **The subject goes in after the warm-up**, so the light and moisture
    // channels have already filled and the colony is not measured against a
    // world still converging.
    // **Set before the subject goes in**, so every scheduled check is judged
    // under the mode being measured rather than a few thousand of them being
    // seeded under the default first.
    if !chain.is_empty() {
        let mode = pixel_physics::sim::structural::CHAIN_MODES
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(chain))
            .unwrap_or_else(|| panic!("chain= must be one of SPREAD, LOCAL, TIGHT, NONE"));
        world.chain_reach = mode.reach;
        println!("chain mode: {} (reach {}) -- {}", mode.name, mode.reach, mode.note);
    }
    let spec = if load == "all" { "ants:64,gnome,blast:600" } else { load };
    let (mut ants, mut gnome, mut blast_every) = (0usize, false, 0usize);
    // `usize::MAX` is "keep firing", so the common case needs no sentinel
    // check at the trigger site below.
    let mut blast_limit = usize::MAX;
    // Env rather than an argument: `ProbeArgs` is already at clippy's
    // `too_many_arguments` limit, and this is a one-shot diagnostic.
    let reconverge_at: usize = std::env::var("RECONVERGE_AT").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let (mut strike_every, mut strike_limit) = (0usize, usize::MAX);
    let (mut mine_every, mut mine_limit) = (0usize, usize::MAX);
    // `EVERY[:COUNT]`, shared by the three verbs so they cannot drift apart.
    let cadence = |n: &str, what: &str| -> (usize, usize) {
        match n.split_once(':') {
            Some((every, count)) => (
                every.parse().unwrap_or_else(|_| panic!("{what}:EVERY:COUNT")),
                count.parse().unwrap_or_else(|_| panic!("{what}:EVERY:COUNT")),
            ),
            None => (n.parse().unwrap_or_else(|_| panic!("{what}:EVERY")), usize::MAX),
        }
    };
    for part in spec.split(',').filter(|p| !p.is_empty()) {
        match part.split_once(':') {
            Some(("ants", n)) => ants = n.parse().expect("ants:N"),
            Some(("blast", n)) => (blast_every, blast_limit) = cadence(n, "blast"),
            Some(("strike", n)) => (strike_every, strike_limit) = cadence(n, "strike"),
            Some(("mine", n)) => (mine_every, mine_limit) = cadence(n, "mine"),
            None if part == "gnome" => gnome = true,
            None if part == "ants" => ants = 64,
            None if part == "blast" => blast_every = 600,
            _ => eprintln!("ignoring unknown load component {part:?}"),
        }
    }

    // The ground line, found the way `ascii`'s river-cost scene finds it:
    // topmost `Solid` or `Powder`. Water and plants are not ground, so an ant
    // is not planted onto a pond or into a canopy.
    let surface_at = |world: &World, x: i32| -> Option<i32> {
        (0..h).find(|&y| {
            let k = world.materials.kind(world.get(x, y).material);
            matches!(k, MaterialKind::Solid | MaterialKind::Powder)
        })
    };

    // **Where the rock is**, which is not where the surface is. `strike` and
    // `mine_swept` both go through `rigid::is_tool_target`, which takes
    // `Solid | Plant` and refuses bedrock -- so a swing aimed at the topmost
    // `Solid | Powder` cell lands in *soil* and removes nothing at all. The
    // first version of this probe did exactly that and reported 23 cuts and
    // **0 cells removed**, which would have been published as "the pick does
    // not leak" and been a statement about the aim.
    let rock_at = |world: &World, x: i32| -> Option<i32> {
        (0..h).find(|&y| {
            let c = world.get(x, y);
            world.materials.kind(c.material) == MaterialKind::Solid && c.material != pixel_physics::sim::material::BEDROCK
        })
    };

    // `Solid` cells in a square around a point -- the before/after census the
    // hammer needs to prove it removed anything. `Solid` only: the pick and
    // hammer turn rock into rubble (a `Powder`) rather than into nothing, so
    // counting occupancy would read a cut as zero change.
    let solid_in_box = |world: &World, cx: i32, cy: i32, r: i32| -> usize {
        let mut n = 0;
        for y in (cy - r)..=(cy + r) {
            for x in (cx - r)..=(cx + r) {
                if world.materials.kind(world.get(x, y).material) == MaterialKind::Solid {
                    n += 1;
                }
            }
        }
        n
    };

    // Spread across the middle of the world rather than from the left edge:
    // a colony bunched at x=0 shares one chunk column and would understate
    // both the pheromone plane and the scheduler.
    let mut planted = 0usize;
    for i in 0..ants {
        let x = w / 4 + (i as i32 * (w / 2)) / ants.max(1) as i32;
        if let Some(sy) = surface_at(&world, x) {
            world.plant_ant(x, sy - 1);
            planted += 1;
        }
    }
    if gnome {
        let x = w / 2;
        if let Some(sy) = surface_at(&world, x) {
            world.player = Some(pixel_physics::sim::player::Player::at(x, sy - 4));
        }
    }

    println!(
        "preset {name}, seed {seed}, {w}x{h} ({} cells), generated in {gen_ms:.0} ms",
        w as i64 * h as i64
    );
    println!(
        "{warm} warm-up frames discarded, {frames} measured ({:.1} day/night cycles)\n",
        frames as f64 / field::DAY_NIGHT_PERIOD_FRAMES as f64
    );

    // In `App::update`'s order, and the order is load-bearing: several phases
    // read state an earlier one settled this frame. Timing them in a
    // different order would measure a frame the app never runs.
    let mut p = [
        Samples::new("sweep (parallel::step)"),
        Samples::new("liquid bodies"),
        Samples::new("rigid bodies"),
        Samples::new("player"),
        Samples::new("active sites: scheduler"),
        Samples::new("active sites: organisms"),
        Samples::new("blasts"),
        Samples::new("splashes"),
        Samples::new("particles"),
        Samples::new("field"),
        Samples::new("pheromones"),
    ];
    let mut buckets = [[Bucket::default(); 2]; 2]; // [sky stepped][gusting]
    let mut frame_total = Samples::new("WHOLE FRAME");

    let mut blasts_fired = 0usize;
    let (mut strikes, mut mines) = (0usize, 0usize);
    // Cells the pick and hammer actually removed, as against swings taken.
    let mut dug_cells = 0usize;
    for step in 0..frames {
        // **Scripted, not random.** A filmstrip's gnome is scripted for the
        // same reason (`examples/filmstrip.rs`): a run that varied per
        // invocation would make two measurements incomparable, which is the
        // whole point of taking them.
        let input = player::PlayerInput {
            right: gnome,
            jump_pressed: gnome && step % 60 == 0,
            ..Default::default()
        };
        // Fired at the surface near the middle, where there is material to
        // break. Radius and strength are `X`'s own debug defaults.
        if blast_every > 0 && step > 0 && step % blast_every == 0 && blasts_fired < blast_limit {
            let x = w / 2 + ((step / blast_every) as i32 % 8 - 4) * 64;
            if let Some(sy) = surface_at(&world, x) {
                blasts.trigger_with(&mut world, &mut particles, x, sy + 8, 20, 200.0);
                blasts_fired += 1;
            }
        }
        // **The oracle for §S.** `RECONVERGE_AT=<frame>` runs the converged
        // whole-world pass once, at that frame, and prints the pending count
        // either side of it.
        //
        // It exists to answer a question no amount of tuning the reactive
        // path can: *is a converged support field a fixpoint under the
        // scheduler at all?* If the queue goes quiet and stays quiet, §S is
        // a convergence bug and `Reports/structural-reconvergence-design.md`
        // is aimed correctly. If it refills, the churn is being driven by
        // something that is not the field's disagreement with itself, and
        // the whole scope is aimed at the wrong quantity.
        //
        // This is a *probe*, not a proposal: `compute_world_distances` walks
        // all 21M cells and takes seconds. Nothing would ship it per blast.
        if reconverge_at > 0 && step == reconverge_at {
            let before = world.active_site_count();
            // **The size of the affected set, which is what decides whether
            // the real fix needs amortising.** Snapshot every body cell's
            // stored distance, converge, and count how many actually moved.
            // The whole-world pass is the numerator's oracle; this is the
            // denominator -- a scoped reconvergence has to touch at least
            // the cells that changed and (being conservative) not many more.
            let mut before_aux: Vec<((i32, i32), u16)> = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    let c = world.get(x, y);
                    if structural::is_body_material(&world, c.material) && c.organism_id() == 0 {
                        before_aux.push(((x, y), c.aux()));
                    }
                }
            }
            let t = Instant::now();
            structural::compute_world_distances(&mut world);
            let cost = t.elapsed().as_secs_f64() * 1000.0;
            let mut changed = 0usize;
            let mut rose = 0usize;
            let mut max_delta = 0u16;
            // **How much of `changed` is a rule difference, not staleness.**
            // `compute_world_distances` anchors on bedrock and the world edge
            // only. `structural::tick` additionally roots a cell at 0 as a
            // *last resort* when nothing else reaches it and it is resting on
            // ground. So every cell `tick` ground-rooted reads 0 before this
            // pass and a long distance (or `u16::MAX`) after -- two rules
            // disagreeing, not a stale value the reactive path failed to
            // correct. Splitting them is what says whether this oracle is
            // measuring the bug or measuring the difference.
            let mut was_ground_root = 0usize;
            for ((x, y), old) in &before_aux {
                let now = world.get(*x, *y).aux();
                if now != *old {
                    changed += 1;
                    if *old == 0 {
                        was_ground_root += 1;
                    }
                    if now > *old {
                        rose += 1;
                        max_delta = max_delta.max(now.saturating_sub(*old));
                    }
                }
            }
            let body = before_aux.len().max(1);
            println!(
                "  [oracle] frame {step:>6} compute_world_distances {cost:.1}ms over {body} body cells |                  changed {changed} ({:.2}% of body), of which rose {rose}, largest rise {max_delta} | of changed, was at 0 (tick ground-root) {was_ground_root} |                  pending {before} -> {}",
                100.0 * changed as f64 / body as f64,
                world.active_site_count()
            );
        }
        // The hammer and the pick, at the app's own sizes: `App::strike`
        // passes `brush_radius * STRIKE_FORCE_PER_RADIUS` (0.9) as force, and
        // `Player::dig` passes `Tuning::dig_yield`. Both walk along the
        // surface rather than hitting one spot for ever, because a player
        // does and because a single hole stops finding fresh rock.
        if strike_every > 0 && step > 0 && step % strike_every == 0 && strikes < strike_limit {
            let x = w / 2 + (strikes as i32 % 64 - 32) * 6;
            if let Some(sy) = rock_at(&world, x) {
                // **Census the swing's own neighbourhood either side of it**,
                // because `strike` returns nothing and a count of *calls* is
                // the vacuous counter `CLAUDE.md` warns about: a hammer aimed
                // at soil it cannot break swings happily and removes nothing,
                // and "this verb does not leak" would then be a statement
                // about the aim rather than about the verb. The box is the
                // crack reach (`radius * CRACK_REACH`, 3) plus slack.
                let box_r = DIG_RADIUS * 4;
                let before = solid_in_box(&world, x, sy + 2, box_r);
                pixel_physics::sim::rigid::strike(&mut world, x, sy + 2, DIG_RADIUS, DIG_RADIUS as f32 * 0.9);
                dug_cells += before.saturating_sub(solid_in_box(&world, x, sy + 2, box_r));
                strikes += 1;
            }
        }
        if mine_every > 0 && step > 0 && step % mine_every == 0 && mines < mine_limit {
            let x = w / 2 + (mines as i32 % 64 - 32) * 6;
            if let Some(sy) = rock_at(&world, x) {
                let to = (x, sy + 2);
                let from = (x, sy + 1);
                // `mine_swept` returns the cells it loosened -- the same
                // "did it fire" question as the strike above, already
                // answered by the function itself.
                dug_cells += pixel_physics::sim::rigid::mine_swept(&mut world, from, to, DIG_RADIUS, 0.0);
                mines += 1;
            }
        }
        let frame = world.frame;
        let sky_stepped =
            field::sky_light_amplitude(frame) != field::sky_light_amplitude(frame.saturating_sub(1));
        let gusting = weather::at(seed, frame).wind.abs() >= 0.45;

        let frame_start = Instant::now();
        let mut t = Instant::now();
        let lap = |p: &mut Samples, t: &mut Instant| {
            p.ms.push(t.elapsed().as_secs_f64() * 1000.0);
            *t = Instant::now();
        };

        parallel::step(&mut world);
        lap(&mut p[0], &mut t);
        world.step_liquid_bodies();
        lap(&mut p[1], &mut t);
        rigid::step_chunk_bodies(&mut world);
        lap(&mut p[2], &mut t);
        player::step(&mut world, input, &tuning);
        lap(&mut p[3], &mut t);
        // `step_active_sites` split into its two halves, because they are
        // bounded differently and the distinction decides what to do about
        // the cost: `scheduler::step` is capped at `MAX_SITES_PER_FRAME`
        // sites plus a load budget, while `plant::step_organisms` runs once
        // per *live organism* with no cap at all -- so one scales with how
        // much is happening and the other with how much world has been sown.
        // Reading them as one number cannot tell those apart, and they want
        // opposite fixes.
        pixel_physics::sim::scheduler::step(&mut world);
        lap(&mut p[4], &mut t);
        pixel_physics::sim::plant::step_organisms(&mut world);
        lap(&mut p[5], &mut t);
        blasts.step(&mut world, &mut particles);
        lap(&mut p[6], &mut t);
        particle::throw_splashes(&mut world, &mut particles);
        lap(&mut p[7], &mut t);
        particles.step(&mut world);
        lap(&mut p[8], &mut t);
        world.step_fields();
        lap(&mut p[9], &mut t);
        world.step_pheromones();
        lap(&mut p[10], &mut t);

        let whole = frame_start.elapsed().as_secs_f64() * 1000.0;
        frame_total.ms.push(whole);
        buckets[usize::from(sky_stepped)][usize::from(gusting)].add(whole);
    }

    // The counter beside the timing: an organism row costing 7 ms means
    // something very different at 40 organisms than at 40,000, and the
    // timing alone cannot say which. `CLAUDE.md`: "did it fire at all needs
    // a counter, not a picture" -- here, "is this per-item cost or item
    // count".
    // The load counters, beside the timings for the same reason every other
    // counter in this repo is: a loaded run and an idle one produce the same
    // *table*, and only these numbers say which one you are looking at.
    if !spec.is_empty() {
            {
        // **The demolition check.** A queue that goes quiet is not evidence
        // of convergence -- `CLAUDE.md`, *a cost that vanishes may be work
        // that vanished*. If a pass raises 70,000 cells' distances, some of
        // them exceed their span, fail, and fall; the world then goes quiet
        // because there is less of it. Counting the body cells that survive
        // to the end of the run is what separates the two, and it costs one
        // scan of a run that has already finished.
        let mut body = 0usize;
        for y in 0..h {
            for x in 0..w {
                if structural::is_body_material(&world, world.get(x, y).material) {
                    body += 1;
                }
            }
        }
        println!("body cells standing at end: {body}");
    }
        println!(
        "load: {planted} ants planted, gnome {}, {blasts_fired} blasts fired, {strikes} strikes, {mines} mine cuts, {dug_cells} cells actually removed by them, {} particles in flight at end",
            if gnome { "on" } else { "off" },
            particles.len(),
        );
    }
    println!(
        "live organisms: {}   chunks: {}   awake chunks: {}\n",
        world.live_organism_count(),
        world.chunk_count(),
        world.active_chunk_count(),
    );

    let grand: f64 = p.iter().map(|s| s.total()).sum();
    println!("{:>24} {:>10} {:>10} {:>10} {:>9}", "phase", "mean", "p90", "worst", "share");
    println!("{}", "-".repeat(68));
    // Sorted by total cost, so the thing to work on is the top line rather
    // than something to be found by reading down a fixed list.
    let mut order: Vec<usize> = (0..p.len()).collect();
    order.sort_by(|&a, &b| p[b].total().partial_cmp(&p[a].total()).unwrap());
    for i in order {
        println!(
            "{:>24} {:>8.3}ms {:>8.3}ms {:>8.3}ms {:>8.1}%",
            p[i].name,
            p[i].mean(),
            p[i].quantile(0.90),
            p[i].worst(),
            if grand > 0.0 { 100.0 * p[i].total() / grand } else { 0.0 },
        );
    }
    println!("{}", "-".repeat(68));
    println!(
        "{:>24} {:>8.3}ms {:>8.3}ms {:>8.3}ms {:>8.1}%",
        frame_total.name,
        frame_total.mean(),
        frame_total.quantile(0.90),
        frame_total.worst(),
        100.0,
    );
    println!(
        "{:>24} {:>8} {:>10} {:>10}",
        "budget @60Hz", "", "", "16.600ms"
    );

    println!("\n{:>12} {:>9} {:>8} {:>9} {:>10} {:>10}", "sky", "wind", "frames", "share", "frame mean", "frame max");
    println!("{}", "-".repeat(64));
    for (sky, by_wind) in buckets.iter().enumerate() {
        for (wind, b) in by_wind.iter().enumerate() {
            if b.frames == 0 {
                continue;
            }
            println!(
                "{:>12} {:>9} {:>8} {:>8.1}% {:>9.2}ms {:>9.2}ms",
                if sky == 1 { "stepping" } else { "flat" },
                if wind == 1 { "gusting" } else { "calm" },
                b.frames,
                100.0 * b.frames as f64 / frames as f64,
                b.mean(),
                b.worst,
            );
        }
    }
    println!("{}", "-".repeat(64));
    println!(
        "{:>12} {:>9} {:>8} {:>8.1}% {:>9.2}ms {:>9.2}ms",
        "amortised", "", frames, 100.0,
        frame_total.mean(),
        frame_total.worst(),
    );

    // The frames over budget, counted rather than inferred from the mean --
    // "did it fire at all needs a counter" applied to the thing that actually
    // matters here, which is dropped frames and not average cost.
    let over = frame_total.ms.iter().filter(|&&ms| ms > 16.6).count();
    println!(
        "\n{over} of {frames} frames ({:.1}%) exceeded the 16.6 ms budget; worst {:.2}ms.",
        100.0 * over as f64 / frames as f64,
        frame_total.worst(),
    );
    println!(
        "Timing overhead: 11 Instant::now() calls per frame, ~20-30 ns each -- under 0.001 ms,\n\
         which is above some of these phases' own cost. Read a sub-0.005 ms row as 'free',\n\
         not as a measurement."
    );
}
