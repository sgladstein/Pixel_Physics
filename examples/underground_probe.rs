//! How much open air the renderer is drawing as the inside of a cave.
//!
//! `Reports/underground-definition.md` settled *where* the sky/underground
//! boundary comes from: `World::sky_surface`, the topmost `Solid` or
//! `Powder` in each column, frozen on the world's first frame. That choice
//! removed a loud artifact (a black rectangle under every tree) and it has a
//! documented, deliberate cost — but the cost was only ever described in
//! words, and the words are about *building*: "lay a roof over a gap and the
//! space under it reads as outdoors".
//!
//! The cost that was never counted points the other way. "There is solid
//! material somewhere above me in this column" is a **column** test, so any
//! overhanging lip present at genesis marks the whole air column beneath it
//! as underground, all the way down — and the air under a cliff brow is open
//! sky, not a cave. Reported from play as dark bands under overhangs.
//!
//! So this counts the disagreement directly, against the honest answer:
//!
//! - **truly outdoors** — reachable from the top of the world by a flood
//!   fill through everything that is not `Solid` or `Powder` (the same
//!   predicate `freeze_sky_surface` uses, so this is exactly its
//!   complement). Water and gas conduct; rock and soil block. A cell under
//!   a brow is reached round the side; a sealed vault is not reached at all.
//! - **drawn underground** — `!World::is_outdoors`, which is what
//!   `render.rs`'s `sky_depth` reads to pick between sky and `UNDERGROUND`.
//!
//! A cell that is both is a **false cave**: open air the player can walk
//! through, drawn as unlit rock. Reported with the darkness actually applied
//! to it, because that is the quantity the eye reads — `render.rs`'s cave
//! ramp is `sqrt(depth / CAVE_FADE_DEPTH)` and saturates at 24 rows, so a
//! cell 24 rows under a lip is drawn at *full* `UNDERGROUND` with no sky
//! left in it at all.
//!
//! ```text
//! cargo run --release --example underground_probe
//! cargo run --release --example underground_probe -- seeds=8 preset=canyon
//! cargo run --release --example underground_probe -- seed=1 quarry=64
//! ```
//!
//! `quarry=W` is the mining half of the same report: after the world has
//! settled (so the surface is already frozen), it takes the top 40 rows off
//! a `W`-wide patch of the tallest hill through the ordinary eraser brush,
//! which is what an open-cast dig is. Nothing roofs the hole, so every cell
//! of it is open sky by inspection; the count says how much of it the
//! renderer blacks out anyway.

use pixel_physics::app::{WORLD_HEIGHT, WORLD_WIDTH};
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::world::World;

/// `render.rs`'s `CAVE_FADE_DEPTH`, duplicated because it is private there.
/// Only the *reporting* depends on it — the false-cave count itself is a
/// pure `is_outdoors` disagreement and does not read this at all, so a
/// drift here mislabels the histogram and cannot manufacture a finding.
const CAVE_FADE_DEPTH: i32 = 24;

struct Args {
    seed: u64,
    seeds: u32,
    preset: String,
    settle: usize,
    quarry: i32,
    /// How many of the largest false-cave regions to name coordinates for.
    top: usize,
}

fn main() {
    let mut a = Args { seed: 1, seeds: 1, preset: String::new(), settle: 60, quarry: 0, top: 5 };
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "seed" => a.seed = v.parse().expect("seed=N"),
            "seeds" => a.seeds = v.parse::<u32>().expect("seeds=N").max(1),
            "preset" => a.preset = v.to_string(),
            "settle" => a.settle = v.parse().expect("settle=N"),
            "quarry" => a.quarry = v.parse().expect("quarry=WIDTH"),
            "top" => a.top = v.parse().expect("top=N"),
            // An unknown argument silently ignored is how a 3.5-hour study
            // came back as one population wearing 24 logs (`CLAUDE.md`).
            _ => panic!("unknown argument {arg:?}"),
        }
    }
    // Echo the parameters, so a log that does not name its seed was written
    // by a binary that never had one.
    println!(
        "underground_probe: preset={} seeds={}..{} settle={} quarry={}",
        if a.preset.is_empty() { "<default>" } else { &a.preset },
        a.seed,
        a.seed + a.seeds as u64 - 1,
        a.settle,
        a.quarry
    );

    let mut worst = Vec::new();
    for i in 0..a.seeds {
        let seed = a.seed + i as u64;
        let mut world = build(seed, &a);
        for _ in 0..a.settle {
            pixel_physics::sim::parallel::step(&mut world);
            world.step_active_sites();
        }
        if a.quarry > 0 {
            quarry(&mut world, a.quarry);
            // One more step so the dig's own debris settles; the surface is
            // already frozen and cannot move.
            for _ in 0..30 {
                pixel_physics::sim::parallel::step(&mut world);
                world.step_active_sites();
            }
        }
        let r = survey(&world);
        println!(
            "\nseed {seed}: {} false-cave cells ({:.3}% of all open air), {} of them at full UNDERGROUND",
            r.false_cave, r.percent, r.saturated
        );
        println!(
            "  air below the frozen skyline: {} cells total, of which {} are genuinely enclosed (real caves)",
            r.false_cave + r.enclosed,
            r.enclosed
        );
        println!("  darkness applied to false-cave air: {}", histogram(&r.by_darkness));
        // The fix's own scoreboard, and the arithmetic is the point. Every
        // false-cave cell the shipped predicate now disagrees with the
        // column rule about is one it has *rescued*; the remainder are cells
        // that were `Solid` or `Powder` at genesis and are air now, so the
        // map calls them underground on purpose — the same rule that keeps a
        // dug shaft a tunnel. A zero here is the tell for a fix that never
        // reached the predicate at all.
        let rescued = r.disagreement;
        let was_ground = r.false_cave.saturating_sub(rescued);
        println!(
            "  of those, {rescued} rescued by the per-cell map; {was_ground} were ground at genesis and stay dark by design{}",
            if rescued == 0 { "  <-- the map is NOT reaching World::is_outdoors" } else { "" }
        );
        overdark_report(&world);
        for (n, (cells, x0, y0, x1, y1)) in r.regions.iter().take(a.top).enumerate() {
            println!("  #{}: {cells:>5} cells at x {x0}..{x1}, y {y0}..{y1}", n + 1);
        }
        worst.push((r.false_cave, seed));
    }
    if a.seeds > 1 {
        worst.sort_unstable();
        let max = worst.last().expect("at least one seed");
        let p90 = worst[((worst.len() as f32 * 0.9) as usize).min(worst.len() - 1)];
        println!("\nover {} seeds: p90 {} cells (seed {}), max {} cells (seed {})", a.seeds, p90.0, p90.1, max.0, max.1);
    }
}

/// How much **rock** the depth grade over-darkens, which is the residual
/// `Reports/dark-bands-diagnosis.md` left open after the per-cell map fixed
/// the air.
///
/// The decision "is this underground" is per cell now, but the *depth* handed
/// to `TerrainLight::Depth` still comes from the per-column skyline — and a
/// morphological opening clips dips, not spikes, so anything solid standing
/// over open air at genesis (a cliff brow, an arch, a boulder on legs) still
/// hands the ground beneath it a large depth and shades it as if it were
/// buried.
///
/// Measured as the gap between two depths:
///
/// - **graded** — `y - sky_surface[x]`, what the grade reads. A lower bound
///   on the real thing: `light_datum` is the *opened* skyline and only ever
///   sits higher, so the true gap is at least this.
/// - **true** — how far up you walk before hitting a cell that is outdoors.
///
/// A cave does not count and that is the point of using `is_outdoors` rather
/// than emptiness: walking up out of a sealed chamber keeps going, because
/// cave air is not outdoors, so the two depths agree and the cell is not
/// flagged. Only air the sky can actually reach shortens the true depth.
fn overdark_report(world: &World) {
    let Some(b) = world.bounds() else { return };
    // **What this measures, and what it deliberately does not.**
    //
    // The obvious metric — grade depth against the true walk-up depth — is
    // now a *tautology*. `ground_datum` is defined as the top of the lowest
    // run of cells the sky cannot reach, so for any cell in that run the two
    // quantities are the same arithmetic and the answer is exactly zero
    // whatever the code does. Measured that way it reported 0 on every seed,
    // which reads as a triumph and is worth nothing (`CLAUDE.md`: a change
    // that moves *nothing* is different evidence, and an exactly-zero delta
    // means suspect the condition is degenerate).
    //
    // So this reports the **size of the correction** instead: how far the
    // datum the grade reads moved when it stopped being the skyline. Same
    // numbers the artifact used to be reported at, now describing what was
    // fixed rather than what is wrong — and non-zero for a real reason, so a
    // regression that quietly reverted the datum would show up as a return
    // to zero.
    let skyline = world.sky_surface();
    let ground = world.ground_datum();
    let corrected = ground.len() == skyline.len();
    let mut buckets = [0usize; 3];
    let mut worst = (0i32, 0i32, 0i32);
    let mut visible: Vec<(i32, i32)> = Vec::new();
    let mut flagged = vec![false; (b.width() * b.height()) as usize];
    for x in b.min_x..=b.max_x {
        let i = (x - b.min_x) as usize;
        let Some(&sky) = skyline.get(i) else { continue };
        if sky == i32::MAX {
            continue;
        }
        let datum = if corrected { ground[i] } else { sky };
        if datum == i32::MAX {
            continue;
        }
        for y in datum..=b.max_y {
            if matches!(world.materials.kind(world.get(x, y).material), MaterialKind::Empty | MaterialKind::Gas) {
                continue;
            }
            // How much shallower this cell now reads: the old grade measured
            // it from the skyline, the new one from the top of its own
            // ground, and a brow is the whole of the difference.
            let over = datum - sky;
            if over >= 8 {
                buckets[0] += 1;
            }
            if over >= 24 {
                buckets[1] += 1;
                visible.push((x, y));
                flagged[((y - b.min_y) * b.width() + (x - b.min_x)) as usize] = true;
            }
            if over >= 64 {
                buckets[2] += 1;
            }
            if over > worst.0 {
                worst = (over, x, y);
            }
        }
    }
    println!(
        "  datum correction (rock the brow fix un-darkens): {} cells by >=8 rows, {} by >=24 (visible), {} by >=64; largest {} rows at ({}, {}){}",
        buckets[0],
        buckets[1],
        buckets[2],
        worst.0,
        worst.1,
        worst.2,
        if corrected { "" } else { "  <-- no ground datum; measuring the skyline against itself" }
    );

    // **Where the *visible* ones are.** The >=8 bucket is ~1.6% of brightness
    // and cannot be seen; >=24 is ~12% and can. Clustering only the visible
    // ones and naming coordinates is what lets someone point a camera at the
    // artifact instead of hunting for it -- a count says how much, never
    // where.
    let (w, h) = (b.width() as usize, b.height() as usize);
    let idx = |x: i32, y: i32| (y - b.min_y) as usize * w + (x - b.min_x) as usize;
    let mut seen = vec![false; w * h];
    let mut clusters: Vec<(usize, i32, i32, i32, i32)> = Vec::new();
    let mut stack: Vec<(i32, i32)> = Vec::new();
    for (x, y) in visible.iter().copied() {
        if seen[idx(x, y)] {
            continue;
        }
        let (mut n, mut x0, mut y0, mut x1, mut y1) = (0usize, x, y, x, y);
        seen[idx(x, y)] = true;
        stack.push((x, y));
        while let Some((cx, cy)) = stack.pop() {
            n += 1;
            x0 = x0.min(cx);
            y0 = y0.min(cy);
            x1 = x1.max(cx);
            y1 = y1.max(cy);
            for (nx, ny) in [(cx - 1, cy), (cx + 1, cy), (cx, cy - 1), (cx, cy + 1)] {
                if nx < b.min_x || nx > b.max_x || ny < b.min_y || ny > b.max_y {
                    continue;
                }
                if seen[idx(nx, ny)] || !flagged[idx(nx, ny)] {
                    continue;
                }
                seen[idx(nx, ny)] = true;
                stack.push((nx, ny));
            }
        }
        clusters.push((n, x0, y0, x1, y1));
    }
    clusters.sort_unstable_by_key(|c| std::cmp::Reverse(c.0));
    for (n, (cells, x0, y0, x1, y1)) in clusters.iter().take(3).enumerate() {
        println!("    corrected patch #{}: {cells:>5} cells at x {x0}..{x1}, y {y0}..{y1}", n + 1);
    }
}

fn build(seed: u64, a: &Args) -> World {
    let mut world = World::new(Rect::new(0, 0, WORLD_WIDTH as i32 - 1, WORLD_HEIGHT as i32 - 1));
    let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("{e}");
    }
    let name = if a.preset.is_empty() { presets.default_name() } else { a.preset.clone() };
    let Some(params) = presets.get(&name) else { panic!("unknown preset {name:?}") };
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed });
    world
}

/// Take the top 40 rows off a `width`-wide patch centred on the world's
/// highest ground, through the same eraser brush the player's pick uses.
/// An open pit with nothing over it: every cell of it is sky by inspection.
fn quarry(world: &mut World, width: i32) {
    let b = world.bounds().expect("a generated world has bounds");
    let surface = world.sky_surface().to_vec();
    let (mut best_x, mut best_y) = (b.min_x, i32::MAX);
    for (i, &y) in surface.iter().enumerate() {
        if y < best_y {
            best_y = y;
            best_x = b.min_x + i as i32;
        }
    }
    println!("  quarry: {width} wide x 40 deep, centred on the highest ground at x={best_x} (surface y={best_y})");
    for x in (best_x - width / 2)..=(best_x + width / 2) {
        for y in best_y..(best_y + 40) {
            world.paint_capsule((x, y), (x, y), 0, material::EMPTY, 1.0);
        }
    }
}

#[derive(Default)]
struct Survey {
    /// Open air the renderer draws as cave.
    false_cave: usize,
    /// Air below the frozen skyline that really is sealed off — genuine
    /// caves, vaults and pockets. The control: this is what the mechanism
    /// exists to darken, and it must stay large.
    enclosed: usize,
    /// False-cave cells drawn at full `UNDERGROUND` (>= `CAVE_FADE_DEPTH`
    /// below the frozen skyline) — no sky colour left in them at all.
    saturated: usize,
    percent: f64,
    /// Bucketed by the cave ramp's applied weight, in tenths.
    by_darkness: [usize; 11],
    /// `(cells, min_x, min_y, max_x, max_y)`, largest first.
    regions: Vec<(usize, i32, i32, i32, i32)>,
    /// Empty cells where the shipped predicate (`World::is_outdoors`) and
    /// the old per-column rule now answer differently — the **liveness
    /// check**. Zero means the per-cell map is not reaching the predicate at
    /// all, which is what a silently-unwired fix looks like; it should equal
    /// `false_cave` once the map is in play.
    disagreement: usize,
}

fn survey(world: &World) -> Survey {
    let b = world.bounds().expect("a generated world has bounds");
    let (w, h) = (b.width() as usize, b.height() as usize);
    let idx = |x: i32, y: i32| (y - b.min_y) as usize * w + (x - b.min_x) as usize;

    // Blocks the fill exactly as it blocks the freeze, so this is the
    // complement of `freeze_sky_surface` rather than a second opinion about
    // what ground is.
    let blocks = |x: i32, y: i32| {
        matches!(world.materials.kind(world.get(x, y).material), MaterialKind::Solid | MaterialKind::Powder)
    };

    let fill_start = std::time::Instant::now();
    // Flood from the top row. 4-connected, not 8: air passes through a
    // shared face, and a diagonal-only gap between two rocks is not a way
    // out — the same distinction `diffuse_resource` makes and for the same
    // reason (`CLAUDE.md`'s neighbourhood gotcha).
    let mut open = vec![false; w * h];
    let mut stack = Vec::new();
    for x in b.min_x..=b.max_x {
        if !blocks(x, b.min_y) {
            open[idx(x, b.min_y)] = true;
            stack.push((x, b.min_y));
        }
    }
    while let Some((x, y)) = stack.pop() {
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if nx < b.min_x || nx > b.max_x || ny < b.min_y || ny > b.max_y {
                continue;
            }
            if open[idx(nx, ny)] || blocks(nx, ny) {
                continue;
            }
            open[idx(nx, ny)] = true;
            stack.push((nx, ny));
        }
    }

    println!("  (genesis flood fill over {w}x{h} = {} cells: {:.1} ms)", w * h, fill_start.elapsed().as_secs_f64() * 1000.0);

    let mut s = Survey::default();
    let mut air = 0usize;
    let mut is_false = vec![false; w * h];
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            // The raw material, not `is_empty()`: a promoted liquid body's
            // container cell is materially empty and `render.rs` still draws
            // it as background, so "what does this position look like" is
            // the question, not "is this position available".
            if world.get(x, y).material != material::EMPTY {
                continue;
            }
            let outdoors_in_truth = open[idx(x, y)];
            if outdoors_in_truth {
                air += 1;
            }
            // **The column rule asked directly, not through
            // `World::is_outdoors`.** That predicate now reads the per-cell
            // map, which this probe's own flood fill seeds — asking it would
            // make the instrument a function of the thing it measures and
            // report a flat zero whether the fix worked or not
            // (`CLAUDE.md`: a debug readout must not be a function of the
            // thing it debugs). Asking `sky_surface` keeps the *same* number
            // before and after the fix: what the column form gets wrong, and
            // therefore what the map is buying. `disagreement` below is the
            // separate liveness check that the map is actually wired up.
            let column_says_underground = y >= world.sky_surface()[(x - b.min_x) as usize];
            let shipped_says_underground = !world.is_outdoors(x, y);
            if shipped_says_underground != column_says_underground {
                s.disagreement += 1;
            }
            if !column_says_underground {
                continue;
            }
            if !outdoors_in_truth {
                s.enclosed += 1;
                continue;
            }
            s.false_cave += 1;
            is_false[idx(x, y)] = true;
            let depth = y - world.sky_surface()[(x - b.min_x) as usize];
            let t = (depth.clamp(0, CAVE_FADE_DEPTH) as f32 / CAVE_FADE_DEPTH as f32).sqrt();
            s.by_darkness[(t * 10.0).round() as usize] += 1;
            if depth >= CAVE_FADE_DEPTH {
                s.saturated += 1;
            }
        }
    }
    s.percent = if air == 0 { 0.0 } else { s.false_cave as f64 * 100.0 / air as f64 };

    // Cluster them, so the report can say *where* rather than only how
    // much: one big patch under a brow and the same count sprinkled over a
    // thousand columns are different bugs.
    let mut seen = vec![false; w * h];
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            if !is_false[idx(x, y)] || seen[idx(x, y)] {
                continue;
            }
            let (mut n, mut x0, mut y0, mut x1, mut y1) = (0usize, x, y, x, y);
            seen[idx(x, y)] = true;
            stack.push((x, y));
            while let Some((cx, cy)) = stack.pop() {
                n += 1;
                x0 = x0.min(cx);
                y0 = y0.min(cy);
                x1 = x1.max(cx);
                y1 = y1.max(cy);
                for (nx, ny) in [(cx - 1, cy), (cx + 1, cy), (cx, cy - 1), (cx, cy + 1)] {
                    if nx < b.min_x || nx > b.max_x || ny < b.min_y || ny > b.max_y {
                        continue;
                    }
                    if !is_false[idx(nx, ny)] || seen[idx(nx, ny)] {
                        continue;
                    }
                    seen[idx(nx, ny)] = true;
                    stack.push((nx, ny));
                }
            }
            s.regions.push((n, x0, y0, x1, y1));
        }
    }
    s.regions.sort_unstable_by_key(|r| std::cmp::Reverse(r.0));
    s
}

fn histogram(buckets: &[usize; 11]) -> String {
    let mut out = String::new();
    for (i, n) in buckets.iter().enumerate() {
        if *n == 0 {
            continue;
        }
        if !out.is_empty() {
            out.push_str(", ");
        }
        out.push_str(&format!("{}0% {n}", i));
    }
    if out.is_empty() {
        "none".to_string()
    } else {
        out
    }
}
