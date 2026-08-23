//! Design round for the open-cast-dig case: what would a **sky-visibility**
//! model actually say, on the five geometries that decide it?
//!
//! `Reports/dark-bands-diagnosis.md` left one case unfixed — a pit you dig in
//! daylight still draws as cave, because those cells really were rock and no
//! per-cell classification can reach them.
//! `Reports/prior-art-underground-lighting.md` says the answer is
//! propagation rather than a better boolean. This measures that claim before
//! anyone writes it into the engine, because two candidate models look
//! identical in prose and disagree completely on a shaft.
//!
//! **The candidates**
//!
//! - `field` — the light channel this engine already computes. `apply_sky`
//!   casts sun straight *down* each field column and attenuates only through
//!   occluders, on the deliberate principle that clear air does not attenuate
//!   sunlight. Read it and the shaft case answers itself: the air above a dug
//!   shaft is clear, so the shaft is lit to the bottom. That is the exact bug
//!   the owner reported from the era before dug stone became background —
//!   *"if you dug a tall skinny shaft all the way down, it looked like sunny
//!   sky all the way down"*.
//! - `propagated` — Terraria's shape. Sky light is **seeded only where the
//!   cell is outdoors** (`World::is_outdoors`, the per-cell genesis map that
//!   shipped — the first bit of a wall layer), and then spreads by distance
//!   with a per-cell decay: `AIR_DECAY` through empty space, `SOLID_DECAY`
//!   through material, taking the max over all paths. A dug shaft is not
//!   outdoors, so nothing is seeded in it and its light has to walk down from
//!   the mouth.
//!
//! The difference is entirely in the *seeding*, and that is the finding this
//! probe exists to make checkable: both models propagate, and only one of
//! them refuses to hand a dug shaft free daylight.
//!
//! ```text
//! cargo run --release --example sky_light_probe
//! cargo run --release --example sky_light_probe -- air=0.91 solid=0.56
//! cargo run --release --example sky_light_probe -- out=/tmp/skylight.png
//! ```
//!
//! Writes a PNG of the propagated field on a fixed dark→bright ramp — a full
//! replace, never a blend into the world's own colours, per `CLAUDE.md`'s
//! rule about debug channels that read as blank.

use std::collections::VecDeque;

use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::world::World;

const W: i32 = 512;
const H: i32 = 320;
/// Where the flat ground surface sits in the test world.
const GROUND: i32 = 100;

fn main() {
    let mut air_decay = 0.91f32;
    let mut solid_decay = 0.56f32;
    let mut out = String::from("target/filmstrips/sky_light_probe.png");
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            // Terraria's own constants, from the decompiled `Lighting.cs`
            // (`negLight` / `negLight2`). Exposed so the shape can be swept
            // rather than argued about.
            "air" => air_decay = v.parse().expect("air=F"),
            "solid" => solid_decay = v.parse().expect("solid=F"),
            "out" => out = v.to_string(),
            _ => panic!("unknown argument {arg:?}"),
        }
    }
    println!("sky_light_probe: air={air_decay} solid={solid_decay} out={out}");
    println!("  (0.1 of full light after {:.0} air cells, {:.0} solid cells)", ln_steps(air_decay), ln_steps(solid_decay));

    let mut world = build();
    // Run it, so the genesis map exists — everything below depends on
    // `is_outdoors` being the per-cell answer and not the unstepped
    // fallback. **`step_fields` as well as `parallel::step`**, or the
    // channel this probe exists to compare against reads zero everywhere
    // including open sky, and the comparison column silently measures
    // nothing (`CLAUDE.md`: sanity-check a metric against a case you know
    // is fine).
    for _ in 0..2 {
        pixel_physics::sim::parallel::step(&mut world);
        world.step_fields();
    }
    // The geometries are cut *after* the freeze, which is what makes them
    // digs rather than terrain. A pit cut before the freeze would be a
    // valley and would prove nothing.
    dig(&mut world);
    // Long enough for the field to converge on the new geometry — it is a
    // relaxation, so one step after a dig shows the world before it.
    for _ in 0..400 {
        pixel_physics::sim::parallel::step(&mut world);
        world.step_fields();
    }

    let t = std::time::Instant::now();
    let (light, pushes) = propagate(&world, air_decay, solid_decay);
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    let dijkstra_ms = ms;
    // The cost question the design has to answer, and the relaxation's
    // shape is the thing to watch: pushes per cell says whether the FIFO
    // converges in one sweep or thrashes. Scaled to the shipped world so
    // the number is comparable to a frame budget rather than to a test bed.
    let cells = (W * H) as f64;
    println!(
        "\npropagation: {ms:.1} ms over {} cells ({:.2} pushes/cell); at 2048x640 that is ~{:.0} ms",
        cells as u64,
        pushes as f64 / cells,
        ms * (2048.0 * 640.0) / cells
    );
    let t = std::time::Instant::now();
    let sweeps = propagate_sweeps(&world, air_decay, solid_decay);
    let sweep_ms = t.elapsed().as_secs_f64() * 1000.0;
    let mut worst = 0.0f32;
    let mut worst_at = (0, 0);
    for y in 0..H {
        for x in 0..W {
            let d = (light[(y * W + x) as usize] - sweeps[(y * W + x) as usize]).abs();
            if d > worst {
                worst = d;
                worst_at = (x, y);
            }
        }
    }
    println!(
        "four-sweep: {sweep_ms:.2} ms over {} cells; at 2048x640 ~{:.0} ms, at one 512x320 viewport {sweep_ms:.2} ms",
        cells as u64,
        sweep_ms * (2048.0 * 640.0) / cells
    );
    println!("  worst disagreement with the exact solve: {worst:.3} at {worst_at:?}");

    // **Sweep the block size rather than argue about it.** 8 is what
    // `FIELD_SCALE` already is, and it is not obviously the right answer
    // here — the cost falls as 1/scale^2 while the thing being lost is a
    // one-cell shaft, so the two pull opposite ways and only a table
    // settles it.
    println!("\n{:<7} {:>9} {:>9} {:>12} {:>12}    pit rim", "scale", "blocks", "sweep ms", "1-wide top", "tunnel mouth");
    for sc in [1, 2, 4, 8] {
        let (g, gw, gh, _, sms) = propagate_coarse(&world, air_decay, solid_decay, sc);
        let s1 = coarse_at(&g, gw, gh, sc, 60, GROUND + 1);
        let s2 = coarse_at(&g, gw, gh, sc, 300, 74);
        let s3 = coarse_at(&g, gw, gh, sc, 232, GROUND + 1);
        println!("{sc:<7} {:>9} {sms:>9.3} {s1:>12.3} {s2:>12.3} {s3:>10.3}", gw * gh);
    }
    println!("{:<7} {:>9} {:>9.3} {:>12.3} {:>12.3} {:>10.3}  <- exact", "dijkstra", W * H, dijkstra_ms, light[((GROUND + 1) * W + 60) as usize], light[(74 * W + 300) as usize], light[((GROUND + 1) * W + 232) as usize]);

    let (coarse, bw, bh, build_ms, coarse_sweep_ms) = propagate_coarse(&world, air_decay, solid_decay, 8);
    println!(
        "coarse (FIELD_SCALE=8): sweep {coarse_sweep_ms:.3} ms over {}x{} = {} blocks ({:.0}x cheaper than per-pixel); \
         building the block grid cost {build_ms:.2} ms on top, but the engine already has that (FieldTile::occupancy)",
        bw,
        bh,
        bw * bh,
        sweep_ms / coarse_sweep_ms.max(1e-6)
    );
    let upsampled: Vec<f32> = (0..H).flat_map(|y| (0..W).map(move |x| (x, y))).map(|(x, y)| coarse_at(&coarse, bw, bh, 8, x, y)).collect();

    report(&world, &light, &upsampled);
    write_png(&light, &out);
    write_png(&upsampled, &out.replace(".png", "-coarse8.png"));
    // Scale 4 as well, because the sweep above picks it: 10,240 blocks for a
    // 512x320 viewport, which is within spitting distance of the ~8,400-tile
    // light map a Terraria screen carries. The prior art turns out to be the
    // same *grid size*, not just the same algorithm.
    let (c4, w4, h4, _, _) = propagate_coarse(&world, air_decay, solid_decay, 4);
    let up4: Vec<f32> = (0..H).flat_map(|y| (0..W).map(move |x| (x, y))).map(|(x, y)| coarse_at(&c4, w4, h4, 4, x, y)).collect();
    write_png(&up4, &out.replace(".png", "-coarse4.png"));
    write_png(&sweeps, &out.replace(".png", "-sweeps.png"));
}

/// Cells to reach a tenth of full brightness at a given per-cell decay.
fn ln_steps(decay: f32) -> f32 {
    (0.1f32).ln() / decay.ln()
}

fn build() -> World {
    let mut world = World::new(Rect::new(0, 0, W - 1, H - 1));
    for x in 0..W {
        for y in GROUND..H {
            world.set(x, y, Cell::new(material::STONE, 0));
        }
    }
    // A cliff to drive a horizontal tunnel into, and a lip on it — the
    // overhang case, which the shipped per-cell map already gets right and
    // which any propagation model must not break.
    // Deep enough that the tunnel below dead-ends well inside it. The first
    // version was 40 wide and the tunnel punched clean through, so the far
    // samples read a second mouth and the model looked wrong when the scene
    // was (`CLAUDE.md`: a scene that contradicts the code looks like a bug
    // in the code).
    for x in 300..400 {
        for y in 40..GROUND {
            world.set(x, y, Cell::new(material::STONE, 0));
        }
    }
    for x in 400..430 {
        for y in 40..46 {
            world.set(x, y, Cell::new(material::STONE, 0));
        }
    }
    // A sealed chamber, cut before the freeze so it is a worldgen cave
    // rather than something dug: it must stay dark under every model.
    for x in 420..470 {
        for y in 180..215 {
            world.set(x, y, Cell::EMPTY);
        }
    }
    world
}

/// Everything the *player* does, cut after the surface has frozen.
fn dig(world: &mut World) {
    // A 1-wide shaft, 150 deep. The case that decides between the two
    // models.
    for y in GROUND..250 {
        world.set(60, y, Cell::EMPTY);
    }
    // An 8-wide shaft, same depth.
    for x in 120..128 {
        for y in GROUND..250 {
            world.set(x, y, Cell::EMPTY);
        }
    }
    // A 64-wide open pit, 40 deep — nothing overhead.
    for x in 200..264 {
        for y in GROUND..GROUND + 40 {
            world.set(x, y, Cell::EMPTY);
        }
    }
    // A horizontal tunnel driven into the cliff from its face.
    for x in 250..380 {
        for y in 70..78 {
            world.set(x, y, Cell::EMPTY);
        }
    }
}

/// Multi-source max-decay propagation from every outdoors cell.
///
/// A Dijkstra in disguise, but the weights take only two values, so a plain
/// FIFO relaxation converges without a priority queue and stays O(cells)
/// in practice — each cell is re-pushed only when a strictly brighter path
/// reaches it, and brightness is bounded by 1.
///
/// **Seeded on `is_outdoors`, which is the whole point.** Air the sky could
/// reach when the world was made starts at full; everything else — dug
/// shafts, tunnels, sealed chambers, solid rock — starts dark and only gets
/// what walks in.
fn propagate(world: &World, air_decay: f32, solid_decay: f32) -> (Vec<f32>, u64) {
    let idx = |x: i32, y: i32| (y as usize) * W as usize + x as usize;
    let mut light = vec![0.0f32; (W * H) as usize];
    let mut queue: VecDeque<(i32, i32)> = VecDeque::new();
    let mut pushes: u64 = 0;
    for y in 0..H {
        for x in 0..W {
            if world.is_outdoors(x, y) {
                light[idx(x, y)] = 1.0;
                queue.push_back((x, y));
            }
        }
    }
    // 4-connected, matching the genesis fill: light crosses a face.
    while let Some((x, y)) = queue.pop_front() {
        let here = light[idx(x, y)];
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if !(0..W).contains(&nx) || !(0..H).contains(&ny) {
                continue;
            }
            // The decay is charged for entering the *neighbour*, so a cell
            // of rock costs rock and a cell of air costs air. Powders and
            // liquids count as material: standing in a flooded shaft is not
            // standing in open air.
            let solid = !matches!(world.materials.kind(world.get(nx, ny).material), MaterialKind::Empty | MaterialKind::Gas);
            let v = here * if solid { solid_decay } else { air_decay };
            // The epsilon is load-bearing, not defensive: without it a
            // cell is re-pushed for improvements far below anything the
            // eye or the ramp can resolve, and the relaxation stops being
            // linear. At 0.0005 it is well under one step of an 8-bit
            // ramp.
            if v > light[idx(nx, ny)] + 0.0005 {
                light[idx(nx, ny)] = v;
                pushes += 1;
                queue.push_back((nx, ny));
            }
        }
    }
    (light, pushes)
}

fn report(world: &World, light: &[f32], coarse: &[f32]) {
    let idx = |x: i32, y: i32| (y as usize) * W as usize + x as usize;
    let at = |x: i32, y: i32| light[idx(x, y)];
    // The field channel's own answer at the same place, for the comparison
    // the report is about. `sky_lit` is the coarse "did the column cast
    // reach this block" flag; the continuous value is the light channel.
    // Normalised against the channel's own reading in **open sky at the same
    // frame**, not against `MAX_LIGHT`. The channel swings 20:1 over the
    // day/night cycle by design, so a raw number means nothing without the
    // phase it was taken at (`CLAUDE.md`); dividing by open sky cancels the
    // oscillator exactly, needs no private helper, and puts both columns on
    // the same 0..1 scale so the table is a comparison.
    let open_sky = world.field_at(20, 60).light.max(1e-6);
    let field = |x: i32, y: i32| world.field_at(x, y).light / open_sky;

    println!("\n{:<34} {:>8} {:>10} {:>9}  wanted", "sample", "field", "exact", "coarse/8");
    let row = |label: &str, x: i32, y: i32, wanted: &str| {
        println!(
            "{:<34} {:>8.2} {:>10.3} {:>9.3}  {}",
            label,
            field(x, y),
            at(x, y),
            coarse[(y * W + x) as usize],
            wanted
        );
    };
    println!("-- controls --");
    row("open sky above the ground", 20, 60, "bright");
    row("just under the flat surface", 20, GROUND + 2, "dark");
    row("deep rock", 20, 260, "dark");
    row("sealed chamber (worldgen cave)", 445, 195, "dark");

    println!("-- 1-wide shaft, the deciding case --");
    for d in [1, 5, 12, 24, 48, 100] {
        row(&format!("  {d:>3} cells down"), 60, GROUND + d, if d <= 5 { "lit" } else { "dark" });
    }

    println!("-- 8-wide shaft --");
    for d in [1, 12, 24, 48, 100] {
        row(&format!("  {d:>3} cells down"), 124, GROUND + d, if d <= 12 { "lit" } else { "dark" });
    }

    println!("-- 64-wide open pit, 40 deep --");
    for d in [1, 10, 20, 30, 39] {
        row(&format!("  {d:>3} cells down, centre"), 232, GROUND + d, "a gradient, never black");
    }

    println!("-- horizontal tunnel into a cliff --");
    // The mouth is the **cliff face at x=300**, not the start of the cut:
    // everything left of the cliff was open air at genesis and is outdoors
    // by inspection, so sampling there measures the sky, not the tunnel.
    for n in [0, 5, 12, 24, 39, 60] {
        row(&format!("  {n:>3} cells in from the mouth"), 300 + n, 74, if n <= 5 { "lit" } else { "dark" });
    }

    println!("-- under the cliff's overhanging lip (must stay outdoors) --");
    for d in [2, 10, 30] {
        row(&format!("  {d:>3} cells below the lip"), 415, 46 + d, "bright");
    }

    // The headline numbers, so the design question is a comparison and not
    // a reading of a table.
    let pit_top = at(232, GROUND + 1);
    let pit_bottom = at(232, GROUND + 39);
    let shaft_deep = at(60, GROUND + 48);
    println!(
        "\nheadline: pit {:.2} at the rim -> {:.2} at the floor (a gradient); 1-wide shaft {:.4} at 48 down (dark)",
        pit_top, pit_bottom, shaft_deep
    );
}

/// **Terraria's actual algorithm**, for the cost comparison that decides
/// whether this can run per frame at all.
///
/// Four directional sweeps — left-to-right, right-to-left, top-to-bottom,
/// bottom-to-top — each carrying `light = max(light, neighbour * decay)`.
/// No queue, no priority, one linear pass each, perfectly branch-predictable
/// and cache-friendly in the two horizontal directions.
///
/// It is an **approximation** of the Dijkstra above: a separable sweep
/// cannot route light around a corner in one go, so an L-shaped tunnel
/// needs the pass order to happen to favour it. `report_sweep_error` prints
/// how far apart the two land on the geometries that matter, which is the
/// only honest way to accept an approximation — the question is not whether
/// it differs but whether it differs anywhere the eye is looking.
fn propagate_sweeps(world: &World, air_decay: f32, solid_decay: f32) -> Vec<f32> {
    let idx = |x: i32, y: i32| (y as usize) * W as usize + x as usize;
    let mut light = vec![0.0f32; (W * H) as usize];
    let mut decay = vec![0.0f32; (W * H) as usize];
    for y in 0..H {
        for x in 0..W {
            if world.is_outdoors(x, y) {
                light[idx(x, y)] = 1.0;
            }
            let solid = !matches!(world.materials.kind(world.get(x, y).material), MaterialKind::Empty | MaterialKind::Gas);
            decay[idx(x, y)] = if solid { solid_decay } else { air_decay };
        }
    }
    let carry = |light: &mut Vec<f32>, i: usize, prev: usize| {
        let v = light[prev] * decay[i];
        if v > light[i] {
            light[i] = v;
        }
    };
    for y in 0..H {
        for x in 1..W {
            carry(&mut light, idx(x, y), idx(x - 1, y));
        }
        for x in (0..W - 1).rev() {
            carry(&mut light, idx(x, y), idx(x + 1, y));
        }
    }
    for x in 0..W {
        for y in 1..H {
            carry(&mut light, idx(x, y), idx(x, y - 1));
        }
        for y in (0..H - 1).rev() {
            carry(&mut light, idx(x, y), idx(x, y + 1));
        }
    }
    light
}

/// The same four sweeps at **one value per `scale`-sided block**, then read
/// back bilinearly — which is the only version that can actually run, and
/// the reason is worth stating because it is not obvious from the prior art.
///
/// Terraria's light map is per *tile*, and a tile is sixteen screen pixels,
/// so a Terraria screen is roughly 120x70 = 8,400 cells. This engine's cell
/// *is* the pixel: a 512x320 viewport is 163,840. The algorithm is the same
/// and the problem is two orders of magnitude bigger, so "Terraria does it
/// every frame" does not transfer. Noita's answer is the one that does —
/// a coarse grid (32x32 there) blurred back up — and this engine already
/// has the machinery for it at `FIELD_SCALE` = 8 with bilinear sampling.
///
/// Per-block decay is Beer-Lambert over the block's occupancy, the same
/// shape `FieldTile::transmission` already uses: a block that is a fraction
/// `f` solid costs `solid^(scale*f) * air^(scale*(1-f))`. So a shaft one
/// cell wide inside an eight-cell block is charged mostly-solid and dims
/// faster than an eight-wide shaft that fills its block — the two differ in
/// *rate*, which is the acceptable version of the quantisation. What the
/// existing channel does instead is let a block-aligned eight-wide shaft
/// read full daylight forever, which is not.
fn propagate_coarse(world: &World, air_decay: f32, solid_decay: f32, scale: i32) -> (Vec<f32>, i32, i32, f64, f64) {
    let (bw, bh) = (W / scale, H / scale);
    let bi = |x: i32, y: i32| (y as usize) * bw as usize + x as usize;
    // **Timed apart from the sweep, and the split is the finding.** Building
    // the block grid reads every world cell, so it dominates by two orders
    // of magnitude — and in the engine it is already built: `FieldTile`
    // carries occupancy and `rebuild_blocked` maintains it every field step.
    // Charging that scan to this approach would condemn it for a cost it
    // would not actually pay.
    let build_start = std::time::Instant::now();
    let mut light = vec![0.0f32; (bw * bh) as usize];
    let mut decay = vec![0.0f32; (bw * bh) as usize];
    for by in 0..bh {
        for bx in 0..bw {
            let (mut solid, mut outdoors) = (0i32, 0i32);
            for dy in 0..scale {
                for dx in 0..scale {
                    let (x, y) = (bx * scale + dx, by * scale + dy);
                    if !matches!(world.materials.kind(world.get(x, y).material), MaterialKind::Empty | MaterialKind::Gas) {
                        solid += 1;
                    }
                    if world.is_outdoors(x, y) {
                        outdoors += 1;
                    }
                }
            }
            let n = (scale * scale) as f32;
            let f = solid as f32 / n;
            // A block counts as sky-seeded when most of it is outdoors.
            // Majority rather than "any", or a single outdoors cell at a
            // cave mouth would seed the whole block at full brightness and
            // hand the tunnel behind it a free eight cells of daylight.
            if outdoors as f32 / n > 0.5 {
                light[bi(bx, by)] = 1.0;
            }
            decay[bi(bx, by)] = solid_decay.powf(scale as f32 * f) * air_decay.powf(scale as f32 * (1.0 - f));
        }
    }
    let build_ms = build_start.elapsed().as_secs_f64() * 1000.0;

    let sweep_start = std::time::Instant::now();
    let carry = |light: &mut Vec<f32>, i: usize, prev: usize| {
        let v = light[prev] * decay[i];
        if v > light[i] {
            light[i] = v;
        }
    };
    for y in 0..bh {
        for x in 1..bw {
            carry(&mut light, bi(x, y), bi(x - 1, y));
        }
        for x in (0..bw - 1).rev() {
            carry(&mut light, bi(x, y), bi(x + 1, y));
        }
    }
    for x in 0..bw {
        for y in 1..bh {
            carry(&mut light, bi(x, y), bi(x, y - 1));
        }
        for y in (0..bh - 1).rev() {
            carry(&mut light, bi(x, y), bi(x, y + 1));
        }
    }
    (light, bw, bh, build_ms, sweep_start.elapsed().as_secs_f64() * 1000.0)
}

/// Bilinear read of the coarse grid at a world cell — the same shape
/// `field_at_bilinear` already provides, so this is a stand-in and not a new
/// mechanism.
fn coarse_at(grid: &[f32], bw: i32, bh: i32, scale: i32, x: i32, y: i32) -> f32 {
    let fx = (x as f32 + 0.5) / scale as f32 - 0.5;
    let fy = (y as f32 + 0.5) / scale as f32 - 0.5;
    let (x0, y0) = (fx.floor() as i32, fy.floor() as i32);
    let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
    let g = |gx: i32, gy: i32| grid[(gy.clamp(0, bh - 1) as usize) * bw as usize + gx.clamp(0, bw - 1) as usize];
    let a = g(x0, y0) * (1.0 - tx) + g(x0 + 1, y0) * tx;
    let b = g(x0, y0 + 1) * (1.0 - tx) + g(x0 + 1, y0 + 1) * tx;
    a * (1.0 - ty) + b * ty
}

/// A fixed dark→bright ramp, full replace. Never a blend into the world's
/// own colours — that is exactly how a canopy-density sheet once read as
/// blank (`CLAUDE.md`).
fn write_png(light: &[f32], out: &str) {
    let mut buf = vec![0u8; (W * H * 4) as usize];
    for (i, v) in light.iter().enumerate() {
        let t = v.clamp(0.0, 1.0).powf(0.45);
        let p = i * 4;
        buf[p] = (t * 255.0) as u8;
        buf[p + 1] = (t * 235.0) as u8;
        buf[p + 2] = (40.0 + t * 180.0) as u8;
        buf[p + 3] = 255;
    }
    if let Some(dir) = std::path::Path::new(out).parent() {
        std::fs::create_dir_all(dir).expect("creating the output directory");
    }
    image::save_buffer(out, &buf, W as u32, H as u32, image::ColorType::Rgba8).expect("writing the png");
    println!("propagated field: {out}");
}
