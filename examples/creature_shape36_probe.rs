//! **§13d's "one run" pre-check for
//! `Reports/creature-genome-flexibility-2026-09-02.md`.** Standalone rather
//! than an edit to `creature_look.rs` -- that instrument's shapes ladder is
//! block-only (`shapes()`'s `block(w,h)` closure fills a rectangle), and
//! this needs a second body at the **same cell count and a different
//! composition**, which is exactly what `creature-appearance-design.md` §4
//! measured at 9 cells (arms C/D: a filled 3x3 against a waisted 5x2, 0.8%
//! apart). This is the same measurement at 36 cells, because the owner's
//! "it is a perfect cube" verdict was a shape reading delivered at 36 cells
//! and §4's finding was measured at 9 -- §13d's own falsifiable prediction
//! is that the two are on opposite sides of a legibility threshold.
//!
//! Reuses `creature_look.rs`'s method (paint on real generated, grown
//! terrain; measure ink and |contrast| against the surround; count decoys
//! at the achieved contrast) rather than its code, so nothing in that file
//! is touched.
//!
//! ```text
//! cargo run --release --example creature_shape36_probe -- seed=1
//! ```

use pixel_physics::app::{HEIGHT, WIDTH};
use pixel_physics::render::Renderer;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{creature, parallel, rng};

const DAYLIGHT: f32 = 1.0;
const SURROUND: i32 = 3;

fn luma(px: &[u8]) -> f32 {
    0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32
}

fn render(world: &World, frame: &mut [u8]) {
    let mut r = Renderer::new();
    r.pinned_light = Some(pixel_physics::sky::frame_for_daylight(DAYLIGHT));
    let particles = ParticleSystem::new();
    r.draw(world, &particles, &std::collections::HashSet::new(), frame, (WIDTH, HEIGHT), true);
}

fn surface(world: &World, x: i32) -> Option<i32> {
    creature::colony_ant_site(world, x, 0)
}

struct Ink {
    ink: f32,
    body_luma: f32,
    surround_luma: f32,
}

fn measure(with: &[u8], without: &[u8], cells: &[(i32, i32)]) -> Ink {
    let body: std::collections::HashSet<(i32, i32)> = cells.iter().copied().collect();
    let (mut sl, mut sn) = (0.0f32, 0usize);
    for &(x, y) in cells {
        for dy in -SURROUND..=SURROUND {
            for dx in -SURROUND..=SURROUND {
                let (sx, sy) = (x + dx, y + dy);
                if body.contains(&(sx, sy)) || sx < 0 || sy < 0 || sx >= WIDTH as i32 || sy >= HEIGHT as i32 {
                    continue;
                }
                let i = ((sy as u32 * WIDTH + sx as u32) * 4) as usize;
                sl += luma(&without[i..i + 4]);
                sn += 1;
            }
        }
    }
    let (mut bl, mut ink) = (0.0f32, 0.0f32);
    for &(x, y) in cells {
        if x < 0 || y < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
            continue;
        }
        let i = ((y as u32 * WIDTH + x as u32) * 4) as usize;
        let (a, b) = (luma(&with[i..i + 4]), luma(&without[i..i + 4]));
        bl += a;
        ink += (a - b).abs();
    }
    let n = cells.len().max(1) as f32;
    Ink { ink, body_luma: bl / n, surround_luma: sl / sn.max(1) as f32 }
}

/// Slides the body's own bounding box over the frame the body is not in and
/// counts positions at least as different from their own surround as the
/// body's own achieved contrast -- `creature_look.rs`'s `decoys`, copied
/// rather than imported so this file touches nothing else.
fn decoys(frame: &[u8], w: i32, h: i32, threshold: f32) -> usize {
    let (fw, fh) = (WIDTH as i32, HEIGHT as i32);
    let lum: Vec<f32> = (0..fw * fh).map(|i| luma(&frame[(i as usize) * 4..(i as usize) * 4 + 4])).collect();
    let at = |x: i32, y: i32| lum[(y * fw + x) as usize];
    let mut hits = 0usize;
    for y in SURROUND..fh - h - SURROUND {
        for x in SURROUND..fw - w - SURROUND {
            let mut inner = 0.0;
            for dy in 0..h {
                for dx in 0..w {
                    inner += at(x + dx, y + dy);
                }
            }
            inner /= (w * h) as f32;
            let (mut outer, mut n) = (0.0, 0);
            for dy in -SURROUND..h + SURROUND {
                for dx in -SURROUND..w + SURROUND {
                    if dx >= 0 && dx < w && dy >= 0 && dy < h {
                        continue;
                    }
                    outer += at(x + dx, y + dy);
                    n += 1;
                }
            }
            if (inner - outer / n as f32).abs() >= threshold {
                hits += 1;
            }
        }
    }
    hits
}

struct Shape {
    name: &'static str,
    w: i32,
    h: i32,
    offsets: Vec<(i32, i32)>,
}

/// A shape placed on the world: its bounding box, for `decoys`, and the
/// world cells it occupies, for `measure`.
struct PlacedShape {
    name: &'static str,
    w: i32,
    h: i32,
    cells: Vec<(i32, i32)>,
}

/// Three 36-cell bodies, differing only in shape. `block36` is
/// `creature_look.rs`'s own `block(w,h)` closure at 6x6. `waisted36` is
/// three segments -- a 3x4 head, a pinched 2x2 waist, a 4x5 abdomen -- the
/// same *kind* of arrangement `ant_wide.ron` uses at 9 cells, scaled up and
/// with a real pinch rather than a single notch so the composition change
/// is unambiguous. `narrow36` is `block(2, 18)` -- the geometry
/// `creature-shape-reachability-2026-09-02.md` §1.3 found mobility-safe
/// (footprint width <=2, blocked-move median 8-13% against >=3 wide's
/// 47-58%), asking whether "extent" pursued as *length at safe width*
/// keeps the legibility win `creature-appearance-design.md` measured for
/// *compact* blocks, or whether that win was actually about being
/// square-ish rather than merely being 36 cells.
fn shapes36() -> Vec<Shape> {
    let block = |w: i32, h: i32| -> Vec<(i32, i32)> { (0..w).flat_map(move |dx| (0..h).map(move |dy| (-dx, -dy))).collect() };
    let mut waisted = Vec::new();
    for dx in 0..3 {
        for dy in 0..4 {
            waisted.push((-dx, -dy));
        } // head, 3x4 = 12
    }
    for dx in 3..5 {
        for dy in 0..2 {
            waisted.push((-dx, -dy));
        } // waist, 2x2 = 4
    }
    for dx in 5..9 {
        for dy in 0..5 {
            waisted.push((-dx, -dy));
        } // abdomen, 4x5 = 20
    }
    assert_eq!(waisted.len(), 36);
    vec![
        Shape { name: "block36 (6x6)", w: 6, h: 6, offsets: block(6, 6) },
        Shape { name: "waisted36 (head/waist/abdomen)", w: 9, h: 5, offsets: waisted },
        Shape { name: "narrow36 (2x18, mobility-safe width)", w: 2, h: 18, offsets: block(2, 18) },
    ]
}

fn main() {
    let mut seed = 1u64;
    let mut preset = String::from("rolling");
    let mut warmup = 2400u32;
    for a in std::env::args().skip(1) {
        let (k, v) = a.split_once('=').unwrap_or_else(|| panic!("expected key=value, got {a:?}"));
        match k {
            "seed" => seed = v.parse().expect("seed"),
            "preset" => preset = v.to_string(),
            "warmup" => warmup = v.parse().expect("warmup"),
            _ => panic!("unknown argument {k:?}"),
        }
    }
    println!("creature_shape36_probe: seed={seed} preset={preset} warmup={warmup}");

    let mut world = World::new(Rect::new(0, 0, WIDTH as i32 - 1, HEIGHT as i32 - 1));
    world.seed = seed;
    let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("worldgen presets unavailable: {e}");
    }
    let params = presets.get(&preset).unwrap_or_else(|| panic!("no worldgen preset {preset:?}"));
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed });
    for _ in 0..warmup {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
    }

    let mut base = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    render(&world, &mut base);

    let vid = world.materials.id_of("ant").unwrap_or_else(|| panic!("no material named ant"));
    let shades = world.materials.get(vid).palette.len().max(1) as u32;
    let dry: Vec<i32> = (0..WIDTH as i32).filter(|&x| surface(&world, x).is_some()).collect();
    assert!(dry.len() >= 20, "only {} dry columns", dry.len());

    let shapes = shapes36();
    let n = shapes.len();
    let mut rows: Vec<PlacedShape> = Vec::new();
    for (si, shape) in shapes.iter().enumerate() {
        // Well clear of each other: each shape gets its own third of the dry
        // columns, so no shared surround ring between them.
        let x0 = dry[((si * 2 + 1) * dry.len()) / (2 * n)];
        let Some((x, base_y)) = (0..80)
            .flat_map(|d| [x0 + d, x0 - d])
            .filter_map(|x| surface(&world, x).map(|sy| (x, sy - 1)))
            .find(|&(x, by)| shape.offsets.iter().all(|&(dx, dy)| world.is_empty(x + dx, by + dy)))
        else {
            panic!("no footing for {} near x={x0}", shape.name);
        };
        let cells: Vec<(i32, i32)> = shape.offsets.iter().map(|&(dx, dy)| (x + dx, base_y + dy)).collect();
        for (i, &(cx, cy)) in cells.iter().enumerate() {
            let shd = rng::stream(world.seed, si as u64, i as u64, 0).below(shades) as u8;
            world.set(cx, cy, Cell::new(vid, shd));
        }
        println!("  placed {:<32} at ({x},{base_y})", shape.name);
        rows.push(PlacedShape { name: shape.name, w: shape.w, h: shape.h, cells });
    }

    let mut with = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    render(&world, &mut with);

    println!("\n{:<32} {:>6} {:>8} {:>8} {:>8} {:>8}", "shape", "cells", "body L", "surr L", "|contr|", "ink");
    let mut results = Vec::new();
    for row in &rows {
        let m = measure(&with, &base, &row.cells);
        let contrast = (m.body_luma - m.surround_luma).abs();
        let d = decoys(&base, row.w, row.h, contrast);
        println!("{:<32} {:>6} {:>8.1} {:>8.1} {:>8.1} {:>8.0}  decoys={d}", row.name, row.cells.len(), m.body_luma, m.surround_luma, contrast, m.ink);
        results.push((row.name, m.ink, contrast));
    }
    if results.len() == 2 {
        let (n0, ink0, c0) = &results[0];
        let (n1, ink1, c1) = &results[1];
        let ink_pct = 100.0 * (ink0 - ink1).abs() / ink0.max(*ink1).max(1.0);
        let c_pct = 100.0 * (c0 - c1).abs() / c0.max(*c1).max(1.0);
        println!("\n{n0} vs {n1}: ink differs by {ink_pct:.1}%, |contrast| differs by {c_pct:.1}%");
    }
}
