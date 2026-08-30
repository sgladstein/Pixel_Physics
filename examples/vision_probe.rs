//! **How far would a beetle have to see?** The E15 pre-flight, before any
//! vision is wired.
//!
//! `predation_probe` settled the question ahead of this one and settled it
//! against the smell hypothesis: the kill works — a hungry beetle beside an
//! ant feeds, ant cells 24 -> 22 -> 21 under a saturated control — and the
//! **search** is what fails. Its numbers, re-run 2026-08-30 and not
//! re-derived here:
//!
//! * channel-B mass **294** over **33** nonzero cells in an 81,920-cell world;
//! * **77%** of ants stand within a sensor offset of a nonzero cell, against
//!   **32%** of beetles;
//! * mean beetle -> nearest nonzero cell **46 cells**, against a **6**-cell
//!   sensor span;
//! * the two sensor reads differ **1.3%** of the time, `|along|` **0.0067** —
//!   there is no gradient anywhere a beetle stands.
//!
//! No sensor in `brain::BrainInput` reports another organism at a distance
//! at all: `FoodAdjacent` and `AtNest` are contact-range, and the two
//! pheromone planes are the only distal sense — the two measured failing.
//! That is the whole of E15's case, and the owner authorised the direction
//! on 2026-08-30. **A direction is not a design.** This file parameterises
//! one, and it can do so *today, with no vision implemented*, because the
//! question is pure geometry over `World::get`.
//!
//! # The question
//!
//! What fraction of beetles have a prey animal in **unobstructed line of
//! sight** at radius 8 / 16 / 32 / 64, and what does terrain occlusion do to
//! that fraction?
//!
//! # Why this is a new file and not a mode on `predation_probe`
//!
//! `Reports/instruments.md` was checked first, and so was every creature row
//! in it. `predation_probe` censuses the two **pheromone planes** — every
//! number it prints is about a signal field, and its distance column is
//! beetle -> nearest *nonzero cell*, not beetle -> nearest *animal*, which
//! is a different quantity over a different population. `forage_probe` asks
//! how far the colony ranges; `creature_probe` prints one creature's inputs
//! per tick and has no beetles; `creature_look` asks whether a body is
//! findable **by a human eye in a rendered picture**, which is a luminance
//! question about the renderer and not a geometry question about the world.
//! None of them can trace a ray. The scene, however, *is*
//! `predation_probe`'s, reproduced deliberately: a pre-flight measured on a
//! different world than the null it is explaining explains nothing, and a
//! hand-built stand-in is 3-for-3 against this codebase
//! (`creature-evolution-plan.md` §6).
//!
//! # The five things this file is careful about
//!
//! 1. **The call counter and the effect counter are the same pair, read at
//!    two radii of the same test.** `range` is "there was an ant close
//!    enough to see" and `los` is "it could actually be seen"; the gap
//!    between them is what occlusion costs, and it is the only thing that
//!    can tell a *range* failure from an *occlusion* failure. A probe that
//!    printed `los` alone could report 0.02 and could not say which of the
//!    two designs — a longer radius or a raised eye — that argues for.
//!    `blocked_by` names the material that stopped the ray, which turns
//!    "occlusion is expensive" into something a design can act on.
//! 2. **The positive control runs in the same binary** (`mode=control`), and
//!    it is a *pair* of arms rather than one. `clear` proves the sight test
//!    can report 1.000 — sensitivity. `walled` puts a stone slab between the
//!    same beetle and the same ants and proves it drops to 0.000 while
//!    `range` stays at 1.000 — specificity, and the half this repo has paid
//!    six times for conflating. A null is exactly where a broken probe
//!    hides: `predation_probe` itself once returned a clean counter-based
//!    negative because the counter counted *calls* — 23 swings, 0 cells
//!    removed, and 1,157 once the aim was corrected.
//! 3. **The gradient-degeneracy trap is ruled out by construction and said
//!    out loud.** `CLAUDE.md` records it hit four times on three lines: a
//!    coarse-field read is block-nearest, so two sensors a cell apart land
//!    in the same `FIELD_SCALE` block roughly seven times in eight and their
//!    difference is a constant zero. **Nothing here reads a field.** Every
//!    read is `World::get` at CA resolution, and the quantity is not a
//!    difference of two samples at all — it is a boolean over a traced line.
//!    There is no pair of reads to be degenerate.
//! 4. **The seeds are a sweep and the answer is an order statistic.** Six
//!    seeds is not a sweep, measured: a census read 1.64x over its first six
//!    and 1.08x over the next twelve, pooling to a per-seed median of zero.
//!    Default `seeds=18`, and the summary prints min / p10 / median / p90 /
//!    max, because a mean over a chaotic population is a number no design
//!    can be sized from.
//! 5. **The cost is quoted whole-frame and paired.** An isolated harness
//!    overstates what the app will see — measured −50% in a subsystem
//!    harness against −27% at app level, same change, same machine — so
//!    `mode=cost` alternates a sighted arm against a blind one over the same
//!    settled world and reports the *frame* delta, with a deterministic
//!    cells-probed counter beside it because a wall clock is only as
//!    trustworthy as the box was quiet.
//!
//! # What this cannot answer
//!
//! Whether a beetle that *can* see an ant will *catch* it. That is a
//! movement and a brain question, and `predation_probe`'s control already
//! says the kill itself works at contact range. This sizes the sense and
//! nothing downstream of it.
//!
//! ```text
//! cargo run --release --example vision_probe -- mode=control
//! cargo run --release --example vision_probe                      # the survey
//! cargo run --release --example vision_probe -- mode=occlusion
//! cargo run --release --example vision_probe -- mode=cost
//! cargo run --release --example vision_probe -- mode=overlay out=/tmp/sight.png
//! ```

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::organism::{self, CellType, CreatureDef};
use pixel_physics::sim::material::MaterialId;
use pixel_physics::sim::{creature, material, parallel, Cell, World};

/// The scene `creature_space::run_one` builds, which is the scene the
/// `beetles=0`/`beetles=9` null and `predation_probe`'s whole census were
/// measured in. Reproduced rather than invented — see the module doc.
const W: i32 = 512;
const H: i32 = 160;
const ANTS: usize = 52;
const BEETLES: usize = 9;
const TREES: i32 = 2;
/// **The default is `wetland` because that is the scene the null was
/// measured in**, not because it is representative — and `preset=` exists
/// because "measured on one scene" is the same defect as "measured on six
/// seeds" wearing different clothes. Terrain relief and ground clutter are
/// exactly what a sight line runs into, so a result taken on one preset is
/// a statement about that preset until another one says otherwise.
const PRESET: &str = "wetland";
/// Frames of world settling before any creature is placed.
const WARMUP: usize = 2400;

/// The radii the design is being sized against. 8 is a little over the
/// beetle's authored `sensor_offset` of 6, so bucket 0 is roughly "what it
/// already has"; the rest double, so the shape of the curve says how much
/// reach buys how much prey.
const RADII: [i32; 4] = [8, 16, 32, 64];

/// Cap on the nearest-animal search. The answer "further than this" is all
/// the decision needs, and an uncapped ring search over a world with no
/// prey walks the whole world per beetle per sample.
const NEAR_CAP: i32 = 128;

const SEED_BASE: u64 = pixel_physics::sim::world::DEFAULT_WORLD_SEED;

static PRESET_NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();

fn preset_name() -> &'static str {
    PRESET_NAME.get().map(|s| s.as_str()).unwrap_or(PRESET)
}

fn main() {
    let mut frames = 3000usize;
    let mut every = 100usize;
    let mut seeds = 18u64;
    let mut mode = "survey".to_string();
    let mut beetles = BEETLES;
    let mut occl = "opaque".to_string();
    let mut cone = 60.0f32;
    let mut eye = 0i32;
    let mut rays = 16usize;
    let mut settle = 0usize;
    let mut preset = PRESET.to_string();
    let mut crop = String::new();
    let mut zoom = 2i32;
    let mut out = std::env::temp_dir().join("vision_probe.png").display().to_string();
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "frames" => frames = v.parse().expect("frames"),
            "every" => every = v.parse().expect("every"),
            "seeds" => seeds = v.parse().expect("seeds"),
            "beetles" => beetles = v.parse().expect("beetles"),
            "mode" => mode = v.to_string(),
            "occl" => occl = v.to_string(),
            "cone" => cone = v.parse().expect("cone"),
            "eye" => eye = v.parse().expect("eye"),
            "rays" => rays = v.parse().expect("rays"),
            "out" => out = v.to_string(),
            "settle" => settle = v.parse().expect("settle"),
            "preset" => preset = v.to_string(),
            "crop" => crop = v.to_string(),
            "zoom" => zoom = v.parse().expect("zoom"),
            // **An unknown argument is silently ignored, and that has cost
            // this repo a 3.5-hour study** (`CLAUDE.md`). Panic instead.
            other => panic!(
                "unknown arg {other:?}; known: mode, frames, every, seeds, beetles, occl, cone, eye, rays, settle, preset, out, crop, zoom"
            ),
        }
    }
    let occl_mode = Occl::parse(&occl);
    // Set once, before anything builds a world. A `OnceLock` rather than an
    // argument threaded through nine signatures: every scene builder in this
    // file wants the same preset for the whole process, and a run that
    // silently mixed two would be a study of nothing.
    PRESET_NAME.set(preset.clone()).expect("preset set once");

    // **Echo the parameters.** A knob nobody can see the value of is a knob
    // nobody can tell is disconnected.
    println!(
        "vision_probe: mode={mode} frames={frames} every={every} seeds={seeds} beetles={beetles} \
         occl={} cone=+-{cone}deg eye={eye} rays={rays} settle={settle} crop={} zoom={zoom} radii={RADII:?} \
         scene={preset} {W}x{H} ants={ANTS} trees={TREES} warmup={WARMUP}\n",
        occl_mode.label(),
        if crop.is_empty() { "full" } else { &crop },
    );

    match mode.as_str() {
        "survey" => survey(frames, every, seeds, beetles, occl_mode, cone, eye, settle),
        "occlusion" => occlusion(frames, every, seeds, beetles, cone, settle),
        "control" => control(cone),
        "cost" => cost(frames, beetles, rays),
        "overlay" => overlay(frames, beetles, occl_mode, eye, &out, &crop, zoom),
        other => panic!("unknown mode {other:?}; known: survey, occlusion, control, cost, overlay"),
    }
}

// ---------------------------------------------------------------------------
// what blocks a sight line
// ---------------------------------------------------------------------------

/// What counts as opaque.
///
/// **Swept rather than chosen**, because which of these the design picks is
/// itself a design decision and the whole point of the occlusion axis is to
/// price it. `None` is not a plausible setting — it is the *geometric
/// ceiling*, the fraction a sense would reach if the world were transparent,
/// and every occluded arm must come in at or below it. An arm that comes in
/// above its own ceiling is an arithmetic bug, which is why it is run.
#[derive(Clone, Copy, PartialEq)]
enum Occl {
    /// Nothing blocks. The ceiling, and a free control on every other arm.
    None,
    /// Rock and soil block; foliage and water do not.
    Opaque,
    /// ...and plant matter blocks too. A canopy you cannot see through.
    Dense,
    /// ...and liquid blocks as well. Everything but air, gas and bodies.
    All,
}

impl Occl {
    fn parse(s: &str) -> Self {
        match s {
            "none" => Occl::None,
            "opaque" => Occl::Opaque,
            "dense" => Occl::Dense,
            "all" => Occl::All,
            other => panic!("unknown occl {other:?}; known: none, opaque, dense, all"),
        }
    }
    fn label(self) -> &'static str {
        match self {
            Occl::None => "none",
            Occl::Opaque => "opaque",
            Occl::Dense => "dense",
            Occl::All => "all",
        }
    }
    /// Does this cell stop a ray?
    ///
    /// **A creature cell never blocks, in any mode.** An ant standing behind
    /// another ant is still a sighting, and the ray terminates on its target
    /// anyway; making bodies opaque would make a colony blind itself and
    /// would measure crowding rather than terrain.
    fn blocks(self, world: &World, c: Cell) -> bool {
        use material::MaterialKind as K;
        match world.materials.kind(c.material) {
            K::Solid | K::Powder => self != Occl::None,
            K::Plant => matches!(self, Occl::Dense | Occl::All),
            K::Liquid => self == Occl::All,
            _ => false,
        }
    }
}

/// Is the straight line from `(x0,y0)` to `(x1,y1)` clear?
///
/// Bresenham, **exclusive of both endpoints**: the beetle is standing in its
/// own cell and the ant is standing in the ant's, and counting either as an
/// obstruction would make every sighting fail for the trivial reason that
/// prey is made of prey. Returns `Ok(())` when clear, or `Err(material)` of
/// the **first** cell that stopped it — which is what makes "occlusion is
/// expensive" into a statement a design can act on.
fn sight_line(world: &World, occl: Occl, x0: i32, y0: i32, x1: i32, y1: i32) -> Result<(), MaterialId> {
    if occl == Occl::None {
        return Ok(());
    }
    let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
    let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
    let (mut err, mut x, mut y) = (dx + dy, x0, y0);
    loop {
        if (x, y) == (x1, y1) {
            return Ok(());
        }
        if (x, y) != (x0, y0) {
            let c = world.get(x, y);
            if occl.blocks(world, c) {
                return Err(c.material);
            }
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Where a creature's eye sits, given `eye` cells of lift.
///
/// **A sensitivity knob, not a model.** Both animals in this world are
/// ground-hugging — head cells sit on the terrain — so every sight line
/// between them grazes the ground for its whole length, and a single soil
/// hummock two cells high blocks a 40-cell line. Whether that is what the
/// number is measuring is exactly the kind of thing that must be checked
/// rather than assumed, and one parameter answers it. Lift only through
/// cells that do not themselves block: raising an eye *into* the terrain
/// would manufacture sight lines out of nothing.
fn eye_at(world: &World, occl: Occl, x: i32, y: i32, eye: i32) -> (i32, i32) {
    let mut ey = y;
    for _ in 0..eye {
        let c = world.get(x, ey - 1);
        if occl.blocks(world, c) {
            break;
        }
        ey -= 1;
    }
    (x, ey)
}

// ---------------------------------------------------------------------------
// the measurement
// ---------------------------------------------------------------------------

/// What one seed reports. Every fraction is over **beetle samples** — one
/// beetle at one sampled frame — so a seed with more beetles alive longer
/// weighs more of its own frames, and the seeds are pooled as an order
/// statistic over these per-seed values rather than over samples.
#[derive(Default, Clone, Copy)]
struct Row {
    /// Fraction of beetle samples with a live ant head within radius, with
    /// **no occlusion test**. The call counter: "there was something to
    /// see."
    range: [f64; 4],
    /// ...and with an unobstructed line to it. The effect counter, from the
    /// far side of the same test. `range - los` is what terrain costs.
    los: [f64; 4],
    /// ...and with that line inside the beetle's forward cone. What a
    /// directional eye, rather than an all-round one, would deliver.
    cone: [f64; 4],
    /// Mean Euclidean distance from a beetle head to the nearest live ant
    /// head, capped at `NEAR_CAP`. **The animal-distance analogue of
    /// `predation_probe`'s 46-cell pheromone figure, and a different
    /// quantity** — that one measured distance to a signal, this measures
    /// distance to a body.
    nearest: f64,
    /// The same, over ants that are actually *visible*. Uncapped-visible
    /// samples are counted in `nearest_seen_n`; a beetle that can see
    /// nothing contributes to neither.
    nearest_seen: f64,
    nearest_seen_n: f64,
    /// Beetle samples, and the census of live heads at the last sample.
    samples: f64,
    ants_alive: usize,
    beetles_alive: usize,
    /// Beetles that had *any* ant in sight at radius 64 at least once —
    /// the "ever" figure beside the "at any instant" one, because a sense
    /// that fires rarely but reliably is a different design from one that
    /// never fires.
    beetles_ever_saw: usize,
    /// What stopped the rays: `(material name, count)` over every
    /// in-range pair that failed the sight test at radius 64.
    blocked_by: [u64; 8],
    /// Material ids, raw, because `Row` derives `Default` and
    /// `MaterialId` does not.
    blocked_names: [u16; 8],
    blocked_kinds: usize,
    /// Total in-range pairs at radius 64, and how many of them were blocked.
    /// The pair census, one level below the per-beetle fractions.
    pairs_in_range: u64,
    pairs_blocked: u64,
}

#[allow(clippy::too_many_arguments)]
fn survey(frames: usize, every: usize, seeds: u64, beetles: usize, occl: Occl, cone: f32, eye: i32, settle: usize) {
    println!(
        "range = fraction of beetle samples with a live ant within radius (no occlusion test) -- \"there was something to see\";\n\
         los   = ...and with an unobstructed line to it -- the effect counter from the far side of the same test;\n\
         cone  = ...and with that line inside +-{cone:.0}deg of the beetle's heading.\n\
         range - los is what terrain costs. All distances Euclidean; d = mean beetle -> nearest ant head (cap {NEAR_CAP}).\n"
    );
    let hdr: Vec<String> = RADII.iter().map(|r| format!("r{r}")).collect();
    println!(
        "{:>4} | {:>6} {:>6} {:>6} {:>6} | {:>6} {:>6} {:>6} {:>6} | {:>6} {:>6} {:>6} {:>6} | {:>6} {:>7} {:>5} {:>5}",
        "seed", hdr[0], hdr[1], hdr[2], hdr[3], hdr[0], hdr[1], hdr[2], hdr[3], hdr[0], hdr[1], hdr[2], hdr[3],
        "d", "d(seen)", "ants", "btl"
    );
    println!("{:>4} | {:^27} | {:^27} | {:^27} |", "", "range", "los", "cone");
    let mut rows = Vec::new();
    for s in 0..seeds {
        let r = run(SEED_BASE + s, frames, every, beetles, occl, cone, eye, settle, Scene::Generated);
        println!(
            "{:>4} | {:>6.3} {:>6.3} {:>6.3} {:>6.3} | {:>6.3} {:>6.3} {:>6.3} {:>6.3} | {:>6.3} {:>6.3} {:>6.3} {:>6.3} | {:>6.1} {:>7.1} {:>5} {:>5}",
            s, r.range[0], r.range[1], r.range[2], r.range[3],
            r.los[0], r.los[1], r.los[2], r.los[3],
            r.cone[0], r.cone[1], r.cone[2], r.cone[3],
            r.nearest, r.nearest_seen, r.ants_alive, r.beetles_alive
        );
        rows.push(r);
    }
    summarise(&rows, occl, cone);
}

/// min / p10 / median / p90 / max over seeds, per radius.
///
/// **An order statistic, not a mean.** Outcomes here are chaotic in the
/// seed, so a mean is a number no design can be sized from and a single
/// seed is a sample from a wide distribution. The spread is also the
/// staleness tell: if every seed agrees to three decimals, suspect the
/// binary before believing the result.
fn stats(v: &mut [f64]) -> (f64, f64, f64, f64, f64) {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    let at = |q: f64| v[((n as f64 - 1.0) * q).round() as usize];
    (v[0], at(0.10), at(0.50), at(0.90), v[n - 1])
}

fn summarise(rows: &[Row], occl: Occl, cone: f32) {
    println!("\norder statistics over {} seed(s) -- occl={} cone=+-{cone:.0}deg:", rows.len(), occl.label());
    println!("{:>10} {:>6} | {:>7} {:>7} {:>7} {:>7} {:>7}", "quantity", "radius", "min", "p10", "median", "p90", "max");
    for (label, pick) in [
        ("range", 0usize),
        ("los", 1usize),
        ("cone", 2usize),
    ] {
        for (i, r) in RADII.iter().enumerate() {
            let mut v: Vec<f64> = rows
                .iter()
                .map(|row| match pick {
                    0 => row.range[i],
                    1 => row.los[i],
                    _ => row.cone[i],
                })
                .collect();
            let (mn, p10, med, p90, mx) = stats(&mut v);
            println!("{label:>10} {r:>6} | {mn:>7.3} {p10:>7.3} {med:>7.3} {p90:>7.3} {mx:>7.3}");
        }
    }
    let mut d: Vec<f64> = rows.iter().map(|r| r.nearest).collect();
    let (mn, p10, med, p90, mx) = stats(&mut d);
    println!("{:>10} {:>6} | {mn:>7.1} {p10:>7.1} {med:>7.1} {p90:>7.1} {mx:>7.1}", "nearest", "-");
    let mut ds: Vec<f64> = rows.iter().filter(|r| r.nearest_seen_n > 0.0).map(|r| r.nearest_seen).collect();
    if !ds.is_empty() {
        let (mn, p10, med, p90, mx) = stats(&mut ds);
        println!("{:>10} {:>6} | {mn:>7.1} {p10:>7.1} {med:>7.1} {p90:>7.1} {mx:>7.1}", "d(seen)", "-");
    }

    let ever: usize = rows.iter().map(|r| r.beetles_ever_saw).sum();
    let alive: usize = rows.iter().map(|r| r.beetles_alive).sum();
    let in_range: u64 = rows.iter().map(|r| r.pairs_in_range).sum();
    let blocked: u64 = rows.iter().map(|r| r.pairs_blocked).sum();
    println!(
        "\npair census at r{}: {in_range} beetle-ant pairs in range, {blocked} of them blocked ({:.1}%).",
        RADII[3],
        100.0 * blocked as f64 / in_range.max(1) as f64
    );
    println!("{ever} of {alive} live beetles saw an ant at least once at r{} across all seeds.", RADII[3]);

    // What stopped the rays. Pooled across seeds by material id.
    let mut pooled: std::collections::HashMap<MaterialId, u64> = std::collections::HashMap::new();
    for r in rows {
        for i in 0..r.blocked_kinds {
            *pooled.entry(MaterialId(r.blocked_names[i])).or_default() += r.blocked_by[i];
        }
    }
    if !pooled.is_empty() {
        let world = World::new(Rect::new(0, 0, 1, 1));
        let mut v: Vec<(MaterialId, u64)> = pooled.into_iter().collect();
        v.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
        let total: u64 = v.iter().map(|&(_, n)| n).sum();
        let named: Vec<String> = v
            .iter()
            .take(6)
            .map(|&(m, n)| format!("{} {:.0}%", world.materials.get(m).name, 100.0 * n as f64 / total as f64))
            .collect();
        println!("what stopped the rays: {}", named.join(", "));
    }
}

/// Which scene a run builds. The control scenes are the same code path as
/// the survey, so a control cannot pass because it took a different route.
#[derive(Clone, Copy, PartialEq)]
enum Scene {
    Generated,
    /// Flat stone floor, one beetle, ants at a spread of clear distances.
    ControlClear,
    /// The same, with a stone slab between the beetle and every ant.
    ControlWalled,
    /// The same, with every ant beyond the largest radius.
    ControlFar,
}

#[allow(clippy::too_many_arguments)]
fn run(seed: u64, frames: usize, every: usize, beetles: usize, occl: Occl, cone: f32, eye: i32, settle: usize, scene: Scene) -> Row {
    let (mut world, ww, wh) = match scene {
        Scene::Generated => (build_scene(seed, beetles), W, H),
        _ => (build_control_scene(scene), CONTROL_W, CONTROL_H),
    };
    let ant_mat = world.materials.id_of("ant").expect("ant");
    let beetle_mat = world.materials.id_of("beetle").expect("beetle");
    let cos_cone = (cone.to_radians()).cos();

    let mut row = Row::default();
    let mut ever: std::collections::HashSet<u16> = std::collections::HashSet::new();
    let mut blocked: std::collections::HashMap<MaterialId, u64> = std::collections::HashMap::new();

    // **`settle` exists because the placement is a confound and had to be
    // ruled out rather than argued away.** Beetles are stood up at x = 40 +
    // 45i and ants from x = 24 upward, so at frame 0 every beetle is
    // standing inside the colony and a short-radius sighting is a statement
    // about the placement loop, not about the world. Skipping the first
    // `settle` frames re-asks the question of a dispersed population; if the
    // two answers agree, the placement was not driving it, and if they do
    // not, the settled one is the answer.
    for frame in 0..(settle + frames) {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
        if frame < settle || !(frame - settle).is_multiple_of(every) {
            continue;
        }

        // --- where the heads are ------------------------------------------
        let mut prey: Vec<(i32, i32)> = Vec::new();
        let mut preds: Vec<(i32, i32, u16)> = Vec::new();
        for py in 0..wh {
            for px in 0..ww {
                let c = world.get(px, py);
                if organism::cell_type(c.aux()) != Some(CellType::Head) {
                    continue;
                }
                if c.material == ant_mat {
                    prey.push((px, py));
                } else if c.material == beetle_mat {
                    preds.push((px, py, c.organism_id()));
                }
            }
        }
        row.ants_alive = prey.len();
        row.beetles_alive = preds.len();
        if preds.is_empty() || prey.is_empty() {
            continue;
        }

        for &(bx, by, id) in &preds {
            let (ex, ey) = eye_at(&world, occl, bx, by, eye);
            let heading = world.organism(id).map(|s| creature::DIRS[s.heading as usize % 8]).unwrap_or((1, 0));
            let hlen = ((heading.0 * heading.0 + heading.1 * heading.1) as f32).sqrt();

            let mut best = NEAR_CAP as f64;
            let mut best_seen = f64::INFINITY;
            let mut hit_range = [false; 4];
            let mut hit_los = [false; 4];
            let mut hit_cone = [false; 4];
            for &(ax, ay) in &prey {
                let (dx, dy) = ((ax - bx) as f64, (ay - by) as f64);
                let d = (dx * dx + dy * dy).sqrt();
                best = best.min(d);
                if d > RADII[3] as f64 {
                    continue;
                }
                let (tx, ty) = eye_at(&world, occl, ax, ay, eye);
                let clear = sight_line(&world, occl, ex, ey, tx, ty);
                let in_cone = hlen > 0.0
                    && d > 0.0
                    && ((dx * heading.0 as f64 + dy * heading.1 as f64) / (d * hlen as f64)) >= cos_cone as f64;
                row.pairs_in_range += 1;
                match clear {
                    Ok(()) => {
                        best_seen = best_seen.min(d);
                        ever.insert(id);
                    }
                    Err(m) => {
                        row.pairs_blocked += 1;
                        *blocked.entry(m).or_default() += 1;
                    }
                }
                for (i, &r) in RADII.iter().enumerate() {
                    if d <= r as f64 {
                        hit_range[i] = true;
                        if clear.is_ok() {
                            hit_los[i] = true;
                            if in_cone {
                                hit_cone[i] = true;
                            }
                        }
                    }
                }
            }
            for i in 0..RADII.len() {
                row.range[i] += hit_range[i] as u8 as f64;
                row.los[i] += hit_los[i] as u8 as f64;
                row.cone[i] += hit_cone[i] as u8 as f64;
            }
            row.nearest += best;
            if best_seen.is_finite() {
                row.nearest_seen += best_seen;
                row.nearest_seen_n += 1.0;
            }
            row.samples += 1.0;
        }
    }

    let s = row.samples.max(1.0);
    for i in 0..RADII.len() {
        row.range[i] /= s;
        row.los[i] /= s;
        row.cone[i] /= s;
    }
    row.nearest /= s;
    if row.nearest_seen_n > 0.0 {
        row.nearest_seen /= row.nearest_seen_n;
    }
    row.beetles_ever_saw = ever.len();
    let mut v: Vec<(MaterialId, u64)> = blocked.into_iter().collect();
    v.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    row.blocked_kinds = v.len().min(8);
    for (i, &(m, n)) in v.iter().take(8).enumerate() {
        row.blocked_names[i] = m.0;
        row.blocked_by[i] = n;
    }
    row
}

// ---------------------------------------------------------------------------
// the scenes
// ---------------------------------------------------------------------------

fn build_scene(seed: u64, beetles: usize) -> World {
    let mut world = World::new(Rect::new(0, 0, W - 1, H - 1));
    world.seed = seed;

    let (presets, _) = pixel_physics::worldgen::WorldgenPresets::load();
    let params = presets.get(preset_name()).unwrap_or_else(|| panic!("no worldgen preset named {:?}", preset_name()));
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed });

    let surface_at: Vec<i32> = (0..W).map(|x| surface(&world, x)).collect();

    let nest = world.materials.id_of("nest").expect("nest");
    for x in 16..90 {
        world.set(x, surface_at[x as usize], Cell::new(nest, 0).with_attached(true));
    }
    for i in 0..TREES {
        let x = 150 + i * (300 / TREES.max(1));
        world.plant_tree(x, surface_at[x as usize] - 1);
    }
    for _ in 0..WARMUP {
        world.step_active_sites();
        world.step_fields();
    }

    // Place ants until there are ANTS of them, not at ANTS fixed columns:
    // `plant_creature_seed` refuses a site it does not fit and returns
    // quietly, and on wetland the target cell is standing water often
    // enough that a fixed-column loop stands up half the colony it asked
    // for (`creature_space`'s own note).
    let mut planted = 0usize;
    let mut x = 24i32;
    while planted < ANTS && x < W - 8 {
        if let Some(site) = creature::plant_creature_seed(&mut world, x, surface_at[x as usize] - 1, "ant") {
            world.schedule_active_site(site);
            planted += 1;
            x += 4;
        } else {
            x += 1;
        }
    }
    let mut beetles_placed = 0usize;
    for i in 0..beetles {
        let bx = 40 + i as i32 * 45;
        if let Some(s) = creature::plant_creature_seed(&mut world, bx, surface_at[bx as usize] - 1, "beetle") {
            world.schedule_active_site(s);
            beetles_placed += 1;
        }
    }
    assert!(planted > 0, "no ant was placed; the scene does not contain the situation this probe is about");
    assert!(
        beetles == 0 || beetles_placed > 0,
        "no beetle was placed; a vision probe with no viewer measures nothing"
    );
    world
}

/// The control scenes, all three from one builder so that the only thing
/// differing between the arms is the thing under test.
const CONTROL_W: i32 = 200;
const CONTROL_H: i32 = 60;

fn build_control_scene(scene: Scene) -> World {
    let (w, h) = (CONTROL_W, CONTROL_H);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    world.seed = SEED_BASE;
    let floor = h - 8;
    for x in 0..w {
        for y in floor..h {
            world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
    let bx = 30i32;
    let beetle = creature::plant_creature_seed(&mut world, bx, floor - 1, "beetle").expect("the beetle is placed");
    world.schedule_active_site(beetle);

    // Distances chosen to straddle every radius: 4 and 6 inside r8, 12
    // inside r16, 24 inside r32, 50 inside r64. `ControlFar` puts every one
    // of them past r64 instead, so the *same* placement loop produces the
    // out-of-range arm.
    let gaps: [i32; 5] = match scene {
        Scene::ControlFar => [100, 110, 120, 130, 140],
        _ => [4, 6, 12, 24, 50],
    };
    let mut ants = 0;
    for g in gaps {
        if creature::plant_creature_seed(&mut world, bx + g, floor - 1, "ant")
            .map(|s| world.schedule_active_site(s))
            .is_some()
        {
            ants += 1;
        }
    }
    assert!(ants > 0, "no prey was placed; the control cannot see a sighting that cannot happen");

    if scene == Scene::ControlWalled {
        // A slab from the floor to the roof of the world, one cell to the
        // right of the beetle. It cannot be walked through and it cannot be
        // seen through: `range` must be unchanged and `los` must go to zero.
        for x in bx + 2..bx + 4 {
            for y in 0..floor {
                world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
    }
    world
}

fn surface(world: &World, x: i32) -> i32 {
    (0..H)
        .find(|&y| {
            world.get(x, y).organism_id() == 0
                && matches!(
                    world.materials.kind(world.get(x, y).material),
                    material::MaterialKind::Solid | material::MaterialKind::Powder
                )
        })
        .unwrap_or(H - 1)
}

// ---------------------------------------------------------------------------
// the positive controls
// ---------------------------------------------------------------------------

/// **Sensitivity and specificity are different checks, and this repo has
/// paid six times for conflating them.**
///
/// `clear` is the sensitivity arm: the sight test must be able to report
/// 1.000 when there genuinely is an unobstructed ant. `walled` is the
/// specificity arm and is the one that matters more: the *same* ants at the
/// *same* distances behind a stone slab, where `range` must stay at 1.000
/// and `los` must fall to 0.000. A probe whose `los` is broken to always-
/// zero passes `walled` and fails `clear`; one whose occlusion test never
/// fires passes `clear` and fails `walled`. Neither arm alone can tell.
fn control(cone: f32) {
    println!("positive controls -- the instrument reporting both answers, on scenes whose answers are known.\n");

    // The ray tracer itself, before any scene: a line through open air is
    // clear, and the same line with one stone cell in it is not.
    let mut w = World::new(Rect::new(0, 0, 63, 63));
    assert!(sight_line(&w, Occl::Opaque, 4, 32, 60, 32).is_ok(), "a line through empty world is reported blocked");
    w.set(30, 32, Cell::new(material::STONE, 0).with_attached(true));
    let blocked = sight_line(&w, Occl::Opaque, 4, 32, 60, 32);
    assert!(blocked.is_err(), "a line through a stone cell is reported clear: the tracer never reads the world");
    assert_eq!(blocked.unwrap_err(), material::STONE, "the tracer named the wrong blocking material");
    // ...and the endpoints must be exclusive, or prey being made of prey
    // would blind every sighting.
    let mut w2 = World::new(Rect::new(0, 0, 63, 63));
    w2.set(4, 32, Cell::new(material::STONE, 0).with_attached(true));
    w2.set(60, 32, Cell::new(material::STONE, 0).with_attached(true));
    assert!(sight_line(&w2, Occl::Opaque, 4, 32, 60, 32).is_ok(), "the endpoints are not exclusive");
    println!("  PASS: the tracer reads the world, names what stopped it, and excludes both endpoints.");

    let arms = [
        ("clear", Scene::ControlClear),
        ("walled", Scene::ControlWalled),
        ("out of range", Scene::ControlFar),
    ];
    println!(
        "\n{:>14} | {:>6} {:>6} {:>6} {:>6} | {:>6} {:>6} {:>6} {:>6} | {:>7} {:>8}",
        "arm", "rng8", "rng16", "rng32", "rng64", "los8", "los16", "los32", "los64", "d", "blocked"
    );
    let mut got = Vec::new();
    for (label, scene) in arms {
        let r = run(SEED_BASE, 600, 60, 1, Occl::Opaque, cone, 0, 0, scene);
        println!(
            "{label:>14} | {:>6.3} {:>6.3} {:>6.3} {:>6.3} | {:>6.3} {:>6.3} {:>6.3} {:>6.3} | {:>7.1} {:>8}",
            r.range[0], r.range[1], r.range[2], r.range[3],
            r.los[0], r.los[1], r.los[2], r.los[3],
            r.nearest, r.pairs_blocked
        );
        got.push(r);
    }
    let (clear, walled, far) = (got[0], got[1], got[2]);

    assert!(clear.los[3] > 0.99, "an unobstructed ant 50 cells away was not seen: the sight test is blind, not quiet");
    // **`los == range` at every radius, not `los == 1.000` at every
    // radius.** The first draft asserted the latter and the control caught
    // it: the animals walk, so the ant placed 4 cells away is within r8 in
    // only a tenth of the samples, and `rng8` reads 0.100 for the same
    // reason `los8` does. The claim the arm actually supports is that
    // *nothing on an open stone floor is ever occluded* — which is a
    // tighter statement than the one that was wrong, since it has to hold
    // at all four radii and at whatever fraction the wandering produces.
    for (i, r) in RADII.iter().enumerate() {
        assert!(
            (clear.los[i] - clear.range[i]).abs() < 1e-9,
            "on a bare stone floor the sight test lost a sighting the range test found at r{r}: {} vs {}",
            clear.los[i], clear.range[i]
        );
    }
    assert_eq!(clear.pairs_blocked, 0, "an ant across an empty floor was reported blocked");

    assert!(walled.range[3] > 0.99, "the walled arm lost its ants: the two arms differ in more than the wall");
    assert!(walled.los[3] < 0.001, "a stone slab did not stop a sight line: the occlusion test never fires");
    assert!(walled.pairs_blocked > 0, "no pair was recorded blocked behind a full-height wall");

    assert!(far.range[3] < 0.001, "an ant 100 cells away was counted within radius 64: the range test is wrong");
    assert!(far.nearest > 90.0, "the nearest-ant distance does not reach 100 cells");

    println!(
        "\n  PASS: sight reads 1.000 across open floor and 0.000 through a slab **at unchanged range**, \n\
         and 0.000 at 100 cells. The gap between `range` and `los` is occlusion and nothing else."
    );
}

// ---------------------------------------------------------------------------
// what it costs
// ---------------------------------------------------------------------------

/// **The whole-frame cost of the sense, paired and alternating.**
///
/// The implementation priced is the one a sensor would actually use: a fan
/// of `rays` rays swept over the beetle's cone, each marched outward until
/// it hits something or reaches the radius. Its cost is a function of the
/// radius and the ray count and **not** of how many prey exist, which is
/// what makes it the shippable shape — the alternative, testing every prey
/// in the world pairwise, needs a prey index the engine does not have and
/// scales with the colony.
///
/// Two rules govern this and neither is optional (`CLAUDE.md`). *Quote the
/// whole-frame figure*: a sub-phase row that falls while the frame does not
/// move is usually the cost relocating, and a change that removed 91% of a
/// phase's work once made the frame slower. And *a wall clock is only as
/// trustworthy as the box was quiet*, so the arms alternate rather than
/// running in two blocks, and `cells probed` is printed beside the
/// milliseconds as the deterministic half.
fn cost(frames: usize, beetles: usize, rays: usize) {
    // **The arms are rebuilt, not cloned, and that is checked rather than
    // assumed.** `World` is not `Clone`, so each arm constructs and settles
    // its own — which is sound only because determinism is required here
    // (same-build, `PLAN.md`). `world_checksum` is asserted equal across
    // every arm, so a run whose arms silently diverged fails loudly instead
    // of publishing a delta that is really two different worlds.
    let settle = 4000usize;
    let build = || -> World {
        let mut w = build_scene(SEED_BASE, beetles);
        for _ in 0..settle {
            parallel::step(&mut w);
            w.step_active_sites();
            w.step_fields();
            w.step_pheromones();
        }
        w
    };
    let probe_world = build();
    let start_sum = world_checksum(&probe_world);
    let beetle_mat = probe_world.materials.id_of("beetle").expect("beetle");
    let def: CreatureDef = probe_world
        .species
        .get(probe_world.species.id_of("beetle").expect("beetle species"))
        .creature
        .as_ref()
        .expect("beetle is a creature")
        .clone();
    let interval = def.tick_interval.max(1);
    drop(probe_world);
    println!(
        "settled world ({W}x{H}, {beetles} beetles), {frames} frames per arm after a {settle}-frame settle.\n\
         The sense is a fan of {rays} rays per beetle, cast once every {interval} frames (the beetle's own tick_interval).\n\
         Arms alternate; `cells probed` is the deterministic half of every row.\n"
    );
    println!(
        "**Three arms, not two, and the middle one is why.** `cast_fan` has to find the beetles before it\n\
         can cast from them, and this harness finds them by scanning all {} cells -- which an engine\n\
         implementation never does, because the active-site scheduler dispatches a creature at its own\n\
         position. Timing scan+rays against nothing would price the harness. `locate` is the scan alone;\n\
         **the sense costs `rN` minus `locate`**, and `locate` minus `blind` is this file's own overhead.\n",
        W * H
    );
    println!("{:>10} {:>12} {:>12} {:>12} {:>14} {:>16}", "arm", "ms/frame", "vs blind", "vs locate", "cells read", "per beetle/cast");

    #[derive(Clone, Copy, PartialEq)]
    enum Arm {
        Blind,
        Locate,
        Fan(i32),
    }
    // Alternating, with `locate` between every sighted arm, because a noise
    // bar belongs to the job it was measured on and the box can drift under
    // a seven-minute run.
    let mut plan = vec![Arm::Blind, Arm::Locate];
    for r in RADII {
        plan.push(Arm::Fan(r));
        plan.push(Arm::Locate);
    }
    plan.push(Arm::Blind);

    let casts_u64 = (frames as u64).div_ceil(interval);
    let mut blind_ms: Vec<f64> = Vec::new();
    let mut locate_ms: Vec<f64> = Vec::new();
    let mut rows: Vec<(i32, f64, u64, u64)> = Vec::new();
    for arm in plan {
        let mut world = build();
        assert_eq!(
            world_checksum(&world),
            start_sum,
            "two arms started from different worlds; the delta would be a statement about the scene, not the sense"
        );
        let mut probed = 0u64;
        let mut located = 0u64;
        let t0 = std::time::Instant::now();
        for frame in 0..frames {
            parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
            world.step_pheromones();
            if arm != Arm::Blind && (frame as u64).is_multiple_of(interval) {
                let radius = match arm {
                    Arm::Fan(r) => r,
                    _ => 0,
                };
                let (p, l) = cast_fan(&world, beetle_mat, radius, rays);
                probed += p;
                located += l;
            }
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0 / frames as f64;
        match arm {
            Arm::Blind => {
                blind_ms.push(ms);
                println!("{:>10} {ms:>12.4} {:>12} {:>12} {:>14} {:>16}", "blind", "-", "-", 0, "-");
            }
            Arm::Locate => {
                assert!(located > 0, "the locate arm found no beetle at all: it is timing an empty scan");
                locate_ms.push(ms);
                // The scan's own volume is printed because it is what makes
                // the per-read cost derivable: this arm reads every cell of
                // the world once per cast and nothing else, so its delta
                // over `blind` divided by these reads is the price of one
                // `World::get` on this box — the number that transfers.
                println!(
                    "{:>10} {ms:>12.4} {:>12} {:>12} {:>14} {:>16.1}",
                    "locate",
                    "-",
                    "-",
                    (W * H) as u64 * casts_u64,
                    located as f64 / casts_u64 as f64
                );
            }
            Arm::Fan(r) => {
                assert!(probed > 0, "the sighted arm probed no cells at all: it is timing nothing");
                rows.push((r, ms, probed, located));
            }
        }
    }
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let (blind, locate) = (mean(&blind_ms), mean(&locate_ms));
    // **Divided by beetles *located*, never by beetles placed.** Nine are
    // stood up and three to six are alive by the time the world has
    // settled, so the placed count is 1.5-3x the real denominator —
    // `CLAUDE.md` records exactly this trap live in two other harnesses
    // (`ascii` plants 55 ants and runs 27; `forage_probe` runs 46 and
    // divides by 55). `located` is counted on the far side of the same
    // call that does the casting, so the two columns cannot disagree.
    for (r, ms, probed, located) in &rows {
        println!(
            "{:>10} {ms:>12.4} {:>12.4} {:>12.4} {:>14} {:>16.0}",
            format!("r{r}"),
            ms - blind,
            ms - locate,
            probed,
            *probed as f64 / (*located).max(1) as f64,
        );
    }
    let spread = |v: &[f64]| v.iter().cloned().fold(f64::MIN, f64::max) - v.iter().cloned().fold(f64::MAX, f64::min);
    println!(
        "\nblind {:?} ms (spread {:.4}); locate {:?} ms (spread {:.4}).\n\
         **Read every delta against those spreads** -- a noise bar belongs to the job it was measured on,\n\
         and it applies to the deltas that flatter the design as much as to the ones that do not.",
        blind_ms.iter().map(|v| (v * 10000.0).round() / 10000.0).collect::<Vec<_>>(),
        spread(&blind_ms),
        locate_ms.iter().map(|v| (v * 10000.0).round() / 10000.0).collect::<Vec<_>>(),
        spread(&locate_ms),
    );
    println!(
        "`cells probed` is bit-identical across repeats by construction; a delta smaller than the arm spread\n\
         is a statement about the machine, and the cells-probed column is what transfers to another one."
    );
}

/// A cheap order-sensitive checksum of every occupied cell. Not a hash with
/// any guarantees — it exists only to catch two arms that started from
/// different worlds, which is the one way a paired timing can lie without
/// anything looking wrong.
fn world_checksum(world: &World) -> u64 {
    let mut sum = 0u64;
    for y in 0..H {
        for x in 0..W {
            let c = world.get(x, y);
            if c.material == material::EMPTY {
                continue;
            }
            sum = sum
                .wrapping_mul(31)
                .wrapping_add((c.material.0 as u64) << 24 | (c.aux() as u64) << 8 | (x as u64 ^ y as u64) & 0xff);
        }
    }
    sum
}

/// One beetle's worth of sensing: `rays` rays over the full circle, each
/// marched to `radius` or to the first blocker.
///
/// Returns `(cells probed, beetles located)`. **Both, and that is the point.**
/// A fan that silently probed nothing would time as free and read as a
/// bargain, so the counter that says it fired is returned from the far side
/// of the call and asserted nonzero by the caller. `beetles located` is the
/// second half: `radius == 0` runs the scan and casts nothing, which is the
/// arm that prices this harness's own head-finding sweep out of the answer.
fn cast_fan(world: &World, beetle_mat: MaterialId, radius: i32, rays: usize) -> (u64, u64) {
    let mut probed = 0u64;
    let mut located = 0u64;
    for py in 0..H {
        for px in 0..W {
            let c = world.get(px, py);
            if c.material != beetle_mat || organism::cell_type(c.aux()) != Some(CellType::Head) {
                continue;
            }
            located += 1;
            if radius == 0 {
                continue;
            }
            for i in 0..rays {
                let a = std::f32::consts::TAU * i as f32 / rays as f32;
                let (dx, dy) = (a.cos(), a.sin());
                for step in 1..=radius {
                    let sx = px + (dx * step as f32).round() as i32;
                    let sy = py + (dy * step as f32).round() as i32;
                    probed += 1;
                    let cell = world.get(sx, sy);
                    if Occl::Opaque.blocks(world, cell) {
                        break;
                    }
                }
            }
        }
    }
    (probed, located)
}

// ---------------------------------------------------------------------------
// the picture
// ---------------------------------------------------------------------------

/// **Look before you measure**, and post the picture rather than describing
/// it. Draws the settled scene through the shipped `Renderer` and stamps
/// every beetle -> ant line on it: green where the line is clear, red where
/// terrain stopped it, and a red dot at the cell that stopped it. The
/// counts go in the caption, because an image says *what* and *where* and
/// only a number says *whether it fired*.
fn overlay(frames: usize, beetles: usize, occl: Occl, eye: i32, out: &str, crop: &str, zoom: i32) {
    use pixel_physics::render::Renderer;
    use pixel_physics::sim::particle::ParticleSystem;
    use std::collections::HashSet;

    let mut world = build_scene(SEED_BASE, beetles);
    for _ in 0..frames {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
    }
    let (w, h) = (W as u32, H as u32);
    let mut frame = vec![0u8; (w * h * 4) as usize];
    let mut r = Renderer::new();
    r.pinned_light = Some(pixel_physics::sky::frame_for_daylight(0.5));
    r.draw(&world, &ParticleSystem::new(), &HashSet::new(), &mut frame, (w, h), true);

    let ant_mat = world.materials.id_of("ant").expect("ant");
    let beetle_mat = world.materials.id_of("beetle").expect("beetle");
    let mut prey = Vec::new();
    let mut preds = Vec::new();
    for py in 0..H {
        for px in 0..W {
            let c = world.get(px, py);
            if organism::cell_type(c.aux()) != Some(CellType::Head) {
                continue;
            }
            if c.material == ant_mat {
                prey.push((px, py));
            } else if c.material == beetle_mat {
                preds.push((px, py));
            }
        }
    }
    let put = |frame: &mut [u8], x: i32, y: i32, rgb: [u8; 3]| {
        if x < 0 || y < 0 || x >= W || y >= H {
            return;
        }
        let i = ((y as u32 * w + x as u32) * 4) as usize;
        frame[i..i + 3].copy_from_slice(&rgb);
    };
    let (mut clear, mut blocked) = (0u32, 0u32);
    for &(bx, by) in &preds {
        let (ex, ey) = eye_at(&world, occl, bx, by, eye);
        for &(ax, ay) in &prey {
            let d = (((ax - bx).pow(2) + (ay - by).pow(2)) as f64).sqrt();
            if d > RADII[3] as f64 {
                continue;
            }
            let (tx, ty) = eye_at(&world, occl, ax, ay, eye);
            let ok = sight_line(&world, occl, ex, ey, tx, ty).is_ok();
            if ok {
                clear += 1;
            } else {
                blocked += 1;
            }
            // Walk the same line again to paint it. Cheap, and it means the
            // picture is drawn by the tracer under test rather than by a
            // second copy of the geometry that could disagree with it.
            let (mut x, mut y) = (ex, ey);
            let (ddx, ddy) = ((tx - ex).abs(), -(ty - ey).abs());
            let (sx, sy) = (if ex < tx { 1 } else { -1 }, if ey < ty { 1 } else { -1 });
            let mut err = ddx + ddy;
            loop {
                if (x, y) == (tx, ty) {
                    break;
                }
                if (x, y) != (ex, ey) {
                    if occl.blocks(&world, world.get(x, y)) {
                        put(&mut frame, x, y, [255, 40, 40]);
                        break;
                    }
                    put(&mut frame, x, y, if ok { [70, 255, 90] } else { [190, 70, 70] });
                }
                let e2 = 2 * err;
                if e2 >= ddy {
                    err += ddy;
                    x += sx;
                }
                if e2 <= ddx {
                    err += ddx;
                    y += sy;
                }
            }
        }
    }
    for &(bx, by) in &preds {
        put(&mut frame, bx, by, [80, 160, 255]);
    }
    for &(ax, ay) in &prey {
        put(&mut frame, ax, ay, [255, 230, 60]);
    }
    // **Cropped tight, then magnified.** A 512x160 sheet with two-cell
    // animals in it is a card the owner can see nothing in -- the review
    // skill's own note records one posted at 190x130 that showed nothing,
    // and the stills that have been judgeable are 700-950 px across.
    let (cx, cy, cw, ch) = if crop.is_empty() {
        (0, 0, W, H)
    } else {
        let v: Vec<i32> = crop.split(',').map(|t| t.parse().expect("crop=x,y,w,h")).collect();
        assert_eq!(v.len(), 4, "crop wants x,y,w,h");
        (v[0], v[1], v[2], v[3])
    };
    let zoom = zoom.max(1) as u32;
    let (ow, oh) = ((cw as u32) * zoom, (ch as u32) * zoom);
    let mut buf = vec![0u8; (ow * oh * 4) as usize];
    for y in 0..ch {
        for x in 0..cw {
            let (sx, sy) = (cx + x, cy + y);
            if sx < 0 || sy < 0 || sx >= W || sy >= H {
                continue;
            }
            let src = ((sy as u32 * w + sx as u32) * 4) as usize;
            for zy in 0..zoom {
                for zx in 0..zoom {
                    let d = (((y as u32 * zoom + zy) * ow + (x as u32 * zoom + zx)) * 4) as usize;
                    buf[d..d + 4].copy_from_slice(&frame[src..src + 4]);
                }
            }
        }
    }
    image::save_buffer(out, &buf, ow, oh, image::ColorType::Rgba8).expect("write png");
    println!(
        "wrote {out} ({}x{}) -- {} beetles, {} ants, {clear} clear sight lines and {blocked} blocked within r{} (occl={}, eye={eye})",
        ow,
        oh,
        preds.len(),
        prey.len(),
        RADII[3],
        occl.label()
    );
    assert!(
        clear + blocked > 0,
        "no beetle-ant pair was within r{} at all: the picture shows nothing this probe is about",
        RADII[3]
    );
}

// ---------------------------------------------------------------------------
// the occlusion axis
// ---------------------------------------------------------------------------

/// **What terrain costs, priced across every plausible reading of
/// "opaque".** `none` is the geometric ceiling and is a free control on
/// every other row: an occluded arm above its own ceiling is an arithmetic
/// bug. `eye` is swept beside it because both animals here are ground-
/// hugging, so a sight line between two heads grazes the terrain for its
/// whole length and a two-cell hummock blocks a forty-cell line — whether
/// *that* is what the survey is measuring has to be checked, not assumed.
fn occlusion(frames: usize, every: usize, seeds: u64, beetles: usize, cone: f32, settle: usize) {
    println!("los at each radius, by what counts as opaque and by how high the eye sits. Median over seeds.\n");
    println!("{:>8} {:>4} | {:>7} {:>7} {:>7} {:>7} | {:>9}", "occl", "eye", "r8", "r16", "r32", "r64", "blocked%");
    let mut ceiling = [0.0f64; 4];
    for (occl, eye) in [
        (Occl::None, 0),
        (Occl::Opaque, 0),
        (Occl::Opaque, 1),
        (Occl::Opaque, 3),
        (Occl::Dense, 0),
        (Occl::Dense, 3),
        (Occl::All, 0),
    ] {
        let rows: Vec<Row> = (0..seeds)
            .map(|s| run(SEED_BASE + s, frames, every, beetles, occl, cone, eye, settle, Scene::Generated))
            .collect();
        let mut med = [0.0f64; 4];
        for (i, slot) in med.iter_mut().enumerate() {
            let mut v: Vec<f64> = rows.iter().map(|r| r.los[i]).collect();
            *slot = stats(&mut v).2;
        }
        let in_range: u64 = rows.iter().map(|r| r.pairs_in_range).sum();
        let blocked: u64 = rows.iter().map(|r| r.pairs_blocked).sum();
        println!(
            "{:>8} {eye:>4} | {:>7.3} {:>7.3} {:>7.3} {:>7.3} | {:>8.1}%",
            occl.label(),
            med[0], med[1], med[2], med[3],
            100.0 * blocked as f64 / in_range.max(1) as f64
        );
        if occl == Occl::None {
            ceiling = med;
            continue;
        }
        for i in 0..4 {
            assert!(
                med[i] <= ceiling[i] + 1e-9,
                "occl={} eye={eye} reports more sightings at r{} than the transparent-world ceiling: arithmetic bug",
                occl.label(),
                RADII[i]
            );
        }
    }
    println!(
        "\n`none` is the transparent-world ceiling, not a setting: every other row is asserted at or below it.\n\
         The gap between `none` and `opaque` at a radius is what terrain costs a sense of that reach."
    );
}
