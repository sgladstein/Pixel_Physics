//! **Where the evolution lab's frame actually goes, and which phase is
//! keeping the box awake.**
//!
//! `lab_cost phases=1` says *which phase* the tick's milliseconds land in and
//! stops there. In the lab bed the answer is always the same two — the CA
//! sweep and the field — and neither is a thing anybody wrote: both are
//! *driven*, by how much of the box is dirty and how much of the field is
//! unsolved. So the phase table names the till, never the spender, and the
//! speed dial's ceiling is set by the spender.
//!
//! This harness measures the spender. Three counters, all read off the world
//! rather than a clock, so they are identical under any machine load:
//!
//! | | |
//! |---|---|
//! | `awake` / `swept` / `passes` | what the CA sweep is *asked* for — chunks, the cells inside their expanded dirty rects, and how many rayon dispatches `parallel::step` cuts that into |
//! | `moved` | how many of those swept cells the tick actually *changed*. `swept / moved` is the waste ratio, and it is the number the whole speed dial hangs on |
//! | `writes[phase]` | cells each phase of `frame::step` changed, and how many distinct chunks it touched — **the attribution**, since a phase that writes into a chunk is what marks it dirty for the next tick |
//!
//! **Why the attribution has to be a diff and not a counter.** A phase does
//! not announce what it dirtied; `Chunk::mark_dirty` is called from inside
//! `World::set` on a path shared by every writer in the engine, and
//! `pending_dirty` is one rect per chunk, so by the end of a tick it says
//! *that* a chunk was written and never *by whom*. Snapshotting the grid
//! between phases and diffing costs about a millisecond a phase and answers
//! it outright. `CLAUDE.md`'s rule about a debug readout not being a function
//! of the thing it debugs is why the diff is taken from a copy of the grid
//! and never from the dirty rects themselves.
//!
//! **The positive control is `arm=empty`.** A sealed box with no plant and no
//! ant in it must report `awake 0`, `swept 0` and zero writes in every phase;
//! a reading of anything else there means the probe is measuring its own
//! bookkeeping, and every attribution below it would be noise wearing a
//! table. It is printed first for that reason.
//!
//! ```text
//! cargo run --release --example labperf                      # the shipped bed
//! cargo run --release --example labperf -- arms=empty,plants,ants,both
//! cargo run --release --example labperf -- settle=8000 probe=200 map=1
//! ```

use std::collections::HashSet;
use std::time::Instant;

use pixel_physics::lab::scene::LabBox;
use pixel_physics::render::Renderer;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::{ChunkCoord, CHUNK_SIZE};
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::particle::{self, ParticleSystem};
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{parallel, rigid};

/// The phases of `sim::frame::step`, in the order it runs them. Re-typed
/// here, which is the fork `sim/frame.rs` exists to prevent — so `main`
/// hashes a world stepped both ways before quoting a single row, exactly as
/// `lab_cost` does.
const PHASES: [&str; 8] = [
    "ca_sweep",
    "liquid_bodies",
    "chunk_bodies",
    "player",
    "active_sites",
    "particles",
    "field",
    "pheromones",
];

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// One arm: the same bed with a different population in it. The point of the
/// split is the owner's own sentence — *"I add plants and creatures and it
/// slows down"* — so plants and creatures have to be separable.
struct Arm {
    name: &'static str,
    founders: usize,
    colonies: usize,
}

const ARMS: [Arm; 4] = [
    Arm { name: "empty", founders: 0, colonies: 0 },
    Arm { name: "plants", founders: 8, colonies: 0 },
    Arm { name: "ants", founders: 0, colonies: 1 },
    Arm { name: "both", founders: 8, colonies: 1 },
];

#[derive(Default, Clone)]
struct Counts {
    /// Cells this phase changed, and the chunks they landed in.
    cells: u64,
    chunks: u64,
    ms: f64,
}

#[derive(Default)]
struct Totals {
    frames: u64,
    awake: u64,
    swept: u64,
    passes: u64,
    moved: u64,
    /// **What a tighter dirty region would have asked for.** Both are
    /// computed from the cells the *previous* tick actually changed, so they
    /// answer the same question `swept` does and can be read against it:
    ///
    /// - `est_bbox` reproduces today's rule — one bounding rect per chunk,
    ///   expanded by the chunk's own `reach` sideways and one row up and
    ///   down. **It is the control**: if it does not land near `swept`, the
    ///   diff is not a faithful stand-in for what marked the chunk dirty and
    ///   `est_rows` below means nothing.
    /// - `est_rows` is the union of every changed cell's own neighbourhood,
    ///   accumulated as one x-span per row instead of one rect per chunk.
    ///   Strictly a subset of `est_bbox` and still a superset of every cell
    ///   the current rule would visit *and act on*.
    est_bbox: u64,
    est_rows: u64,
    /// The same two shapes with the horizontal expansion cut to **one cell**
    /// instead of the chunk's `reach`.
    ///
    /// This is not a proposal to sweep narrower blindly — the reach
    /// expansion is what lets a grain fourteen cells away flow *into* the
    /// changed cell, and dropping it for a *movement* write would lose cells.
    /// It is here because 92% of what changes in this bed is soil **moisture**
    /// (see the churn census), and a moisture write is not a movement: it
    /// needs its own cell and the four it exchanges with reconsidered, and
    /// nothing further. So this pair is the ceiling on separating the two
    /// kinds of dirty mark, and the gap between it and `est_bbox` is what
    /// that separation would be worth.
    est_bbox_local: u64,
    est_rows_local: u64,
    /// Summed `Chunk::reach` over awake chunks — the multiplier on the
    /// horizontal expansion, and the other half of why a rect gets large.
    reach: u64,
    tick_ms: f64,
    render_ms: f64,
    renders: u64,
    phase: Vec<Counts>,
    /// How many times each chunk was awake, laid out as the chunk grid.
    heat: Vec<u64>,
    /// **What is still moving in a box the player would call settled.** Per
    /// material, how many cells of it changed per tick — read against the
    /// per-cell change frequency below, which separates *a lot of things
    /// moving once* from *a few things moving every tick*.
    churn: std::collections::HashMap<u16, u64>,
    /// How many ticks each cell changed on, as a histogram over the probe
    /// window: `freq[k]` is cells that changed on exactly `k` of the ticks.
    /// A settled box should be almost all low `k`; a tail at `k == probe` is
    /// something oscillating, which is work bought with nothing.
    changes_per_cell: Vec<u32>,
}

/// Flatten the whole grid into a buffer the diff can compare against. The bed
/// is 512x320, so this is 160k cells — a memcpy, not a walk.
fn snapshot(world: &World, w: i32, h: i32, into: &mut Vec<Cell>) {
    into.clear();
    for y in 0..h {
        for x in 0..w {
            into.push(world.get(x, y));
        }
    }
}

/// Every cell that differs, as coordinates — the input to both region
/// estimates. Separate from [`diff`] because the estimates need *where*, and
/// the hot counter only needs *how many*.
fn changed_cells(before: &[Cell], world: &World, w: i32, h: i32) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if before[(y * w + x) as usize] != world.get(x, y) {
                out.push((x, y));
            }
        }
    }
    out
}

/// What the next tick's sweep would be asked for under two dirty-region
/// rules, given the cells this tick changed.
///
/// **Both use the chunk's own `reach`**, so the only thing that differs
/// between them is the shape the dirty marks are accumulated into: one rect
/// per chunk, as `Chunk::pending_dirty` does today, against one x-span per
/// row. Anything else — which cells were written, how far a rule can see —
/// is held fixed, which is what makes the pair a paired comparison rather
/// than two measurements.
fn region_estimates(
    changed: &[(i32, i32)],
    world: &World,
    w: i32,
    h: i32,
    local: bool,
) -> (u64, u64) {
    use std::collections::HashMap;
    // Per chunk: the bounding box of its changed cells, and per row the
    // min/max x of them.
    let mut boxes: HashMap<ChunkCoord, (i32, i32, i32, i32)> = HashMap::new();
    let mut spans: HashMap<(ChunkCoord, i32), (i32, i32)> = HashMap::new();
    for &(x, y) in changed {
        let c = ChunkCoord::containing(x, y);
        boxes
            .entry(c)
            .and_modify(|b| {
                b.0 = b.0.min(x);
                b.1 = b.1.min(y);
                b.2 = b.2.max(x);
                b.3 = b.3.max(y);
            })
            .or_insert((x, y, x, y));
        // A changed cell constrains the rows either side of it as well as its
        // own, which is exactly the `expanded_xy(reach, 1)` the current rule
        // applies to the whole rect.
        for row in (y - 1)..=(y + 1) {
            spans
                .entry((c, row))
                .and_modify(|s| {
                    s.0 = s.0.min(x);
                    s.1 = s.1.max(x);
                })
                .or_insert((x, x));
        }
    }

    let mut bbox = 0u64;
    for (c, b) in &boxes {
        let reach = if local { 1 } else { world.chunk_reach(*c).unwrap_or(1) };
        let bounds = c.bounds();
        let min_x = (b.0 - reach).max(bounds.min_x).max(0);
        let max_x = (b.2 + reach).min(bounds.max_x).min(w - 1);
        let min_y = (b.1 - 1).max(bounds.min_y).max(0);
        let max_y = (b.3 + 1).min(bounds.max_y).min(h - 1);
        if max_x >= min_x && max_y >= min_y {
            bbox += ((max_x - min_x + 1) as i64 * (max_y - min_y + 1) as i64) as u64;
        }
    }

    let mut rows = 0u64;
    for ((c, row), sp) in &spans {
        let bounds = c.bounds();
        if *row < bounds.min_y || *row > bounds.max_y || *row < 0 || *row >= h {
            continue;
        }
        let reach = if local { 1 } else { world.chunk_reach(*c).unwrap_or(1) };
        let min_x = (sp.0 - reach).max(bounds.min_x).max(0);
        let max_x = (sp.1 + reach).min(bounds.max_x).min(w - 1);
        if max_x >= min_x {
            rows += (max_x - min_x + 1) as u64;
        }
    }
    (bbox, rows)
}

/// Cells that differ, and the distinct chunks holding them.
fn diff(before: &[Cell], world: &World, w: i32, h: i32, seen: &mut HashSet<ChunkCoord>) -> u64 {
    seen.clear();
    let mut n = 0u64;
    for y in 0..h {
        for x in 0..w {
            if before[(y * w + x) as usize] != world.get(x, y) {
                n += 1;
                seen.insert(ChunkCoord::containing(x, y));
            }
        }
    }
    n
}

fn main() {
    let settle: u64 = arg("settle").unwrap_or(8_000);
    let probe: u64 = arg("probe").unwrap_or(200);
    let map: bool = arg::<u32>("map").unwrap_or(0) == 1;
    let names: String = arg("arms").unwrap_or_else(|| "empty,plants,ants,both".to_string());
    let d = LabBox::default();
    let width: i32 = arg("width").unwrap_or(d.width);
    let height: i32 = arg("height").unwrap_or(d.height);
    let seed: u64 = arg("seed").unwrap_or(d.seed);
    let attribute: bool = arg::<u32>("attribute").unwrap_or(1) == 1;

    // Names its own parameters, first line — `instruments.md`'s standing
    // rule, from the megastudy whose 24 logs were three populations.
    println!(
        "labperf: {width}x{height} seed={seed} settle={settle} probe={probe} \
         attribute={} arms={names} (bed defaults from LabBox::default())",
        u8::from(attribute),
    );
    println!(
        "  the probe diffs the grid between phases, so its own tick cost is inflated; \
         read `tick ms` from `lab_cost`, and read the counters here."
    );

    // **The split-tick control, run before a single row is quoted.** The
    // attribution path re-types `frame::step`'s sequence, which is the fork
    // `sim/frame.rs` exists to prevent, so this is the positive control on
    // that re-typing: the same small bed stepped both ways must hash equal.
    if attribute {
        let small = LabBox { width, height, founders: 2, colonies: 1, seed, ..d.clone() };
        let a = hash_after(&small, 200, false);
        let b = hash_after(&small, 200, true);
        println!(
            "  split-tick control: whole-step {a:#018x} vs per-phase {b:#018x} — {}",
            if a == b { "MATCH" } else { "DIVERGED, do not quote the attribution table" }
        );
        assert_eq!(a, b, "the re-typed phase list is not frame::step's sequence");
    }

    let chunks_x = (width + CHUNK_SIZE - 1) / CHUNK_SIZE;
    let chunks_y = (height + CHUNK_SIZE - 1) / CHUNK_SIZE;

    let mut rows: Vec<(String, Totals)> = Vec::new();
    for name in names.split(',') {
        let Some(arm) = ARMS.iter().find(|a| a.name == name) else {
            println!("unknown arm {name}, skipping");
            continue;
        };
        let spec = LabBox {
            width,
            height,
            founders: arm.founders,
            colonies: arm.colonies,
            seed,
            ..d.clone()
        };
        rows.push((
            arm.name.to_string(),
            run(&spec, settle, probe, attribute, chunks_x, chunks_y),
        ));
    }

    // **What the CA sweep is asked for, against what it finds.** `swept` is
    // the summed area of every awake chunk's expanded dirty rect — the cells
    // `update::sweep` walks — and `moved` is how many of them the whole tick
    // actually changed. The ratio is the waste, and the reason it is printed
    // beside the chunk count rather than instead of it is that they are
    // different fixes: a high chunk count is *which* chunks stay awake, a
    // high ratio is *how much of one* gets swept for a single write.
    println!("\n=== what the sweep is asked for, and what it finds ===");
    println!(
        "{:<8} {:>7} {:>6} {:>9} {:>7} {:>8} {:>8} {:>9} {:>9} {:>8}",
        "arm", "awake", "reach", "swept", "passes", "moved", "waste", "est_bbox", "est_rows", "draw ms"
    );
    let _ = ();
    for (name, t) in &rows {
        let f = t.frames.max(1) as f64;
        let moved = t.moved as f64 / f;
        let swept = t.swept as f64 / f;
        println!(
            "{:<8} {:>7.1} {:>6.1} {:>9.0} {:>7.1} {:>8.1} {:>8} {:>9.0} {:>9.0} {:>8.3}",
            name,
            t.awake as f64 / f,
            if t.awake > 0 { t.reach as f64 / t.awake as f64 } else { 0.0 },
            swept,
            t.passes as f64 / f,
            moved,
            if moved > 0.0 { format!("{:.0}x", swept / moved) } else { "-".to_string() },
            t.est_bbox as f64 / f,
            t.est_rows as f64 / f,
            if t.renders > 0 { t.render_ms / t.renders as f64 } else { 0.0 },
        );
        println!(
            "{:<8} {:>60} {:>9.0} {:>9.0}",
            "", "...with the horizontal expansion cut to one cell:",
            t.est_bbox_local as f64 / f,
            t.est_rows_local as f64 / f,
        );
    }
    println!(
        "  est_bbox is today's rule recomputed from the diff — read it against `swept` first,\n           because it is the control on the whole estimate. est_rows is the same neighbourhoods\n           accumulated one x-span per row instead of one rect per chunk."
    );
    println!(
        "  awake: chunks the sweep will visit, of {} in the box. swept: cells inside their\n  \
         expanded dirty rects. passes: rayon dispatches per tick — `parallel::step` cuts the\n  \
         sweep into one pass per (chunk row, cx parity), so a short box pays this many joins\n  \
         for at most {} chunks each.",
        chunks_x * chunks_y,
        (chunks_x + 1) / 2,
    );

    if attribute {
        println!("\n=== who writes, and therefore who keeps the box awake ===");
        print!("{:<8}", "arm");
        for p in PHASES {
            print!(" {p:>14}");
        }
        println!();
        for (name, t) in &rows {
            let f = t.frames.max(1) as f64;
            print!("{name:<8}");
            for c in &t.phase {
                print!(" {:>14}", format!("{:.1}c/{:.1}k", c.cells as f64 / f, c.chunks as f64 / f));
            }
            println!();
        }
        println!("  cells changed per tick / distinct chunks they landed in. A phase with a large");
        println!("  chunk count is the one holding the sweep open, whatever its own milliseconds say.");
    }

    println!("\n=== what is still moving, in a box the player would call settled ===");
    for (name, t) in &rows {
        let f = t.frames.max(1) as f64;
        let mut by: Vec<(u16, u64)> = t.churn.iter().map(|(k, v)| (*k, *v)).collect();
        by.sort_unstable_by_key(|a| std::cmp::Reverse(a.1));
        let names = material_names(width, height, seed);
        let top: Vec<String> = by
            .iter()
            .take(6)
            .map(|(m, n)| {
                format!(
                    "{} {:.1}",
                    names.get(m).cloned().unwrap_or_else(|| format!("id{m}")),
                    *n as f64 / f
                )
            })
            .collect();
        // The tail is the interesting half: a cell that changes on most ticks
        // of the window is oscillating, and oscillation is sweep cost bought
        // with no visible motion at all.
        let ever = t.changes_per_cell.iter().filter(|&&c| c > 0).count();
        let often = t
            .changes_per_cell
            .iter()
            .filter(|&&c| c as u64 * 2 >= t.frames)
            .count();
        let always = t.changes_per_cell.iter().filter(|&&c| c as u64 == t.frames).count();
        println!(
            "{name:<8} cells/tick by material: {}\n         {ever} distinct cells changed at all, \
             {often} on half the ticks or more, {always} on every tick",
            top.join(", ")
        );
    }

    if map {
        println!("\n=== how often each chunk was awake (0-9, . = never) ===");
        for (name, t) in &rows {
            let f = t.frames.max(1) as f64;
            println!("{name}:");
            for cy in 0..chunks_y {
                let mut line = String::new();
                for cx in 0..chunks_x {
                    let v = t.heat[(cy * chunks_x + cx) as usize] as f64 / f;
                    line.push(if v <= 0.0 {
                        '.'
                    } else {
                        char::from_digit(((v * 9.0).round() as u32).min(9), 10).expect("0-9")
                    });
                }
                println!("  {line}");
            }
        }
    }
}

fn run(
    spec: &LabBox,
    settle: u64,
    probe: u64,
    attribute: bool,
    chunks_x: i32,
    chunks_y: i32,
) -> Totals {
    let mut world = spec.build();
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    let mut renderer = Renderer::new();
    let mut frame_buf = vec![0u8; (spec.width * spec.height * 4) as usize];

    for _ in 0..settle {
        pixel_physics::sim::frame::step(
            &mut world,
            &mut particles,
            &mut blasts,
            player::PlayerInput::default(),
            &tuning,
        );
    }

    let mut t = Totals {
        phase: vec![Counts::default(); PHASES.len()],
        heat: vec![0; (chunks_x * chunks_y) as usize],
        changes_per_cell: vec![0; (spec.width * spec.height) as usize],
        ..Totals::default()
    };
    let mut before = Vec::with_capacity((spec.width * spec.height) as usize);
    let mut frame_before = Vec::with_capacity((spec.width * spec.height) as usize);
    let mut seen = HashSet::new();

    for f in 0..probe {
        // Read off the world *before* the step: this is exactly the set
        // `parallel::step` snapshots and the region `update::sweep` walks.
        let active = world.chunks_to_sweep();
        t.awake += active.len() as u64;
        let mut keys: Vec<(i32, i32)> =
            active.iter().map(|c| (c.y, c.x.rem_euclid(2))).collect();
        keys.sort_unstable();
        keys.dedup();
        t.passes += keys.len() as u64;
        for c in &active {
            if let Some(r) = world.sweep_region(*c) {
                t.swept += ((r.max_x - r.min_x + 1) as i64 * (r.max_y - r.min_y + 1) as i64) as u64;
            }
            t.reach += world.chunk_reach(*c).unwrap_or(0) as u64;
            let (cx, cy) = (c.x, c.y);
            if (0..chunks_x).contains(&cx) && (0..chunks_y).contains(&cy) {
                t.heat[(cy * chunks_x + cx) as usize] += 1;
            }
        }

        snapshot(&world, spec.width, spec.height, &mut frame_before);

        let tick = Instant::now();
        if attribute {
            // The phase list, re-typed, with a grid diff after each. Order is
            // `frame::step`'s and the hash control in `main` is what says so.
            macro_rules! phase {
                ($i:expr, $body:expr) => {{
                    if $i > 0 {
                        snapshot(&world, spec.width, spec.height, &mut before);
                    }
                    let t0 = Instant::now();
                    $body;
                    let ms = t0.elapsed().as_secs_f64() * 1000.0;
                    let base: &[Cell] = if $i == 0 { &frame_before } else { &before };
                    let n = diff(base, &world, spec.width, spec.height, &mut seen);
                    t.phase[$i].cells += n;
                    t.phase[$i].chunks += seen.len() as u64;
                    t.phase[$i].ms += ms;
                }};
            }
            phase!(0, parallel::step(&mut world));
            phase!(1, world.step_liquid_bodies());
            phase!(2, rigid::step_chunk_bodies(&mut world));
            phase!(3, player::step(&mut world, player::PlayerInput::default(), &tuning));
            phase!(4, world.step_active_sites());
            phase!(5, {
                blasts.step(&mut world, &mut particles);
                particle::throw_splashes(&mut world, &mut particles);
                particles.step(&mut world);
            });
            phase!(6, world.step_fields());
            phase!(7, world.step_pheromones());
        } else {
            pixel_physics::sim::frame::step(
                &mut world,
                &mut particles,
                &mut blasts,
                player::PlayerInput::default(),
                &tuning,
            );
        }
        t.tick_ms += tick.elapsed().as_secs_f64() * 1000.0;

        let changed = changed_cells(&frame_before, &world, spec.width, spec.height);
        t.moved += changed.len() as u64;
        for &(x, y) in &changed {
            t.changes_per_cell[(y * spec.width + x) as usize] += 1;
            *t.churn.entry(world.get(x, y).material.0).or_insert(0) += 1;
        }
        let (bbox, rows) = region_estimates(&changed, &world, spec.width, spec.height, false);
        t.est_bbox += bbox;
        t.est_rows += rows;
        let (bbox_l, rows_l) = region_estimates(&changed, &world, spec.width, spec.height, true);
        t.est_bbox_local += bbox_l;
        t.est_rows_local += rows_l;
        seen.clear();

        // The draw the way the game draws it: dirty-rect, not forced full.
        if f.is_multiple_of(20) {
            let touched = world.take_touched_chunks();
            let t0 = Instant::now();
            renderer.draw(
                &world,
                &particles,
                &touched,
                &mut frame_buf,
                (spec.width as u32, spec.height as u32),
                false,
            );
            t.render_ms += t0.elapsed().as_secs_f64() * 1000.0;
            t.renders += 1;
        }
        t.frames += 1;
    }
    t
}

/// A cheap order-sensitive digest of the bed, the same shape `frame.rs`'s own
/// guard uses: what it has to catch is a phase running in the wrong order,
/// which moves cells rather than counting them.
fn world_hash(w: &World, width: i32, height: i32) -> u64 {
    fn fnv1a(h: u64, v: u64) -> u64 {
        (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for y in 0..height {
        for x in 0..width {
            let c = w.get(x, y);
            h = fnv1a(h, c.material.0 as u64);
            h = fnv1a(h, c.shade as u64);
            h = fnv1a(h, c.aux() as u64);
            h = fnv1a(h, c.organism_id() as u64);
        }
    }
    h
}

/// Step a bed `frames` times, either through `frame::step` or through the
/// re-typed phase list, and hash it. The two must agree.
fn hash_after(spec: &LabBox, frames: u64, split: bool) -> u64 {
    let mut world = spec.build();
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    for _ in 0..frames {
        if split {
            parallel::step(&mut world);
            world.step_liquid_bodies();
            rigid::step_chunk_bodies(&mut world);
            player::step(&mut world, player::PlayerInput::default(), &tuning);
            world.step_active_sites();
            blasts.step(&mut world, &mut particles);
            particle::throw_splashes(&mut world, &mut particles);
            particles.step(&mut world);
            world.step_fields();
            world.step_pheromones();
        } else {
            pixel_physics::sim::frame::step(
                &mut world,
                &mut particles,
                &mut blasts,
                player::PlayerInput::default(),
                &tuning,
            );
        }
    }
    world_hash(&world, spec.width, spec.height)
}

/// Material names for the churn census, read off a freshly built bed so the
/// ids are the ones the registry actually assigned. Built once and thrown
/// away — this is a printing convenience, not a measurement.
fn material_names(width: i32, height: i32, seed: u64) -> std::collections::HashMap<u16, String> {
    let w = LabBox { width, height, founders: 0, colonies: 0, seed, ..LabBox::default() }.build();
    (0..w.materials.len())
        .map(|i| (i as u16, w.materials.get(pixel_physics::sim::material::MaterialId(i as u16)).name.clone()))
        .collect()
}
