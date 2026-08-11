//! Renders the simulation to the terminal.
//!
//! Movement rules are far easier to judge by eye than by assertion, and this
//! needs no window or GPU — so it works over a remote shell and in CI. Run with:
//!
//! ```text
//! cargo run --example ascii
//! ```
//!
//! `X` marks sand the movement rules say should still be falling. A settled
//! world must show none; any that appear are cells the sweep stopped examining.

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::field::FIELD_SCALE;
use pixel_physics::sim::material::{self, MaterialId};
use pixel_physics::sim::{update, Cell, World};

fn main() {
    scene("sand piling on a floor", 78, 30, 400, |w| {
        w.paint_circle(39, 2, 4, material::SAND);
    });

    // The same amount of each powder, dropped from the same height. Their
    // friction angles are 45, 34 and 22 degrees, so gravel should hold a sharp
    // peak, sand a moderate one, and ash should slump almost flat.
    scene("angle of repose: gravel, sand, ash", 120, 34, 1500, |w| {
        for (x, m) in [(20, material::GRAVEL), (60, material::SAND), (100, material::ASH)] {
            for y in 2..10 {
                for dx in -3..=3 {
                    w.set(x + dx, y, Cell::new(m, 0));
                }
            }
        }
    });

    scene("water finding its level around a pillar", 78, 30, 500, |w| {
        for y in 18..29 {
            w.set(39, y, Cell::new(material::STONE, 0));
        }
        w.paint_circle(12, 4, 6, material::WATER);
    });

    scene("sand sinking through water, smoke rising through it", 78, 30, 300, |w| {
        for y in 20..29 {
            for x in 1..77 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        w.paint_circle(39, 2, 3, material::SAND);
        w.set(10, 28, Cell::new(material::SMOKE, 0));
        w.set(20, 28, Cell::new(material::SMOKE, 0));
    });

    // Large enough to straddle chunk seams in both axes (chunks are 64x64),
    // which is where sand was observed freezing in mid-air.
    scene("a block dropped across chunk seams", 128, 128, 2000, |w| {
        for y in 20..100 {
            for x in 40..90 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }
    });

    // The realistic worst case: the sandbox's own resolution, filled with
    // material that is all moving at once. The worst frame here is what has to
    // fit inside the 16.6 ms budget at 60 Hz.
    scene("stress: a full screen of sand and water", 512, 320, 400, |w| {
        for y in 20..160 {
            for x in 0..512 {
                let m = if y < 90 { material::SAND } else { material::WATER };
                w.set(x, y, Cell::new(m, 0));
            }
        }
    });

    // M13: the same worst case, plus the field step every frame — this is
    // what the live app actually does now (App::update runs both). The gap
    // between this number and the CA-only one above is the field grid's cost.
    field_stress_scene("stress: full screen + field step every frame", 512, 320, 400, |w| {
        for y in 20..160 {
            for x in 0..512 {
                let m = if y < 90 { material::SAND } else { material::WATER };
                w.set(x, y, Cell::new(m, 0));
            }
        }
        w.add_pressure_impulse(256, 100, 20, 150.0);
    });

    // Sand pouring off a ledge onto a platform below, to show the shape of the
    // free-falling stream and the slope it builds where it lands.
    scene("sand pouring off a ledge", 78, 40, 1200, |w| {
        for x in 10..34 {
            w.set(x, 12, Cell::new(material::STONE, 0));
        }
        for x in 30..70 {
            w.set(x, 30, Cell::new(material::STONE, 0));
        }
        for y in 4..12 {
            for x in 12..32 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }
    });

    // M13: a pressure impulse in open space, hitting a wall on the right.
    // Wide domain and few frames, deliberately, so the front is still
    // expanding rather than having already filled the whole visible area —
    // that's what makes it read as a travelling wave rather than a static
    // glow. Should spread outward as a roughly circular front and visibly
    // reflect rather than pass through the '#' column.
    field_scene("field: pressure impulse reflecting off a wall", 400, 120, 12, |w| {
        for y in 0..120 {
            w.set(360, y, Cell::new(material::STONE, 0));
        }
        w.add_pressure_impulse(80, 60, 6, 200.0);
    });
    // Same setup, run long enough for the front to actually reach the wall
    // (closer this time). Confirms containment the way the eye can judge it:
    // pressure fills the space right up to the '#' column and nothing
    // appears past it. The precise claim — that velocity crossing the wall is
    // zero, not just "looks contained" — is `walls_zero_the_velocity_that_
    // would_cross_them` in field.rs, which is the test actually worth
    // trusting; this is a sanity check, not a substitute for it.
    field_scene("field: pressure impulse reflecting off a wall (longer run)", 400, 120, 40, |w| {
        for y in 0..120 {
            w.set(200, y, Cell::new(material::STONE, 0));
        }
        w.add_pressure_impulse(80, 60, 6, 200.0);
    });

    // M13: same impulse, but sealed in a box. Should stay concentrated near
    // the center rather than dissipating outward, unlike the open scene above.
    field_scene("field: pressure impulse sealed in a room", 160, 80, 200, |w| {
        for x in 10..70 {
            for y in [10, 69] {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for y in 10..70 {
            for x in [10, 69] {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        w.add_pressure_impulse(40, 40, 6, 200.0);
    });
}

/// Prints pressure magnitude as a density ramp, one character per field cell
/// (`FIELD_SCALE` world cells) rather than per CA cell — the field grid is
/// coarser, and printing at CA resolution would just repeat each character
/// `FIELD_SCALE` times for no extra information. `#` marks a field cell
/// blocked by CA-solid material, so walls are visible against the pressure.
fn field_scene(title: &str, w: i32, h: i32, frames: usize, setup: impl FnOnce(&mut World)) {
    println!("\n=== {title} ===");
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    setup(&mut world);
    for _ in 0..frames {
        update::step(&mut world);
        world.step_fields();
    }

    const RAMP: [char; 10] = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
    // Chosen by eye against the impulse magnitudes used above; not a
    // physically meaningful unit.
    const NORMALIZE: f32 = 30.0;

    let mut y = 0;
    while y < h {
        let mut row = String::new();
        let mut x = 0;
        while x < w {
            let c = world.field_at(x, y);
            if world.field_is_blocked(x, y) {
                row.push('#');
            } else {
                let mag = (c.pressure.abs() / NORMALIZE).min(1.0);
                let idx = ((mag * (RAMP.len() - 1) as f32).round() as usize).min(RAMP.len() - 1);
                row.push(RAMP[idx]);
            }
            x += FIELD_SCALE;
        }
        println!("|{row}|");
        y += FIELD_SCALE;
    }
}

fn scene(title: &str, w: i32, h: i32, frames: usize, setup: impl FnOnce(&mut World)) {
    println!("\n=== {title} ===");
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    for x in 0..w {
        world.set(x, h - 1, Cell::new(material::STONE, 0));
    }
    setup(&mut world);

    // The worst frame is what has to fit in the budget; the average is
    // meaningless once the world settles and most frames cost nothing.
    let mut worst = std::time::Duration::ZERO;
    for _ in 0..frames {
        let started = std::time::Instant::now();
        update::step(&mut world);
        worst = worst.max(started.elapsed());
    }

    let bad = unstable(&world, w, h);
    println!(
        "after {frames} frames: {}/{} chunks awake, {} unsupported cells, worst frame {:.3} ms",
        world.active_chunk_count(),
        world.chunk_count(),
        bad.len(),
        worst.as_secs_f64() * 1000.0,
    );

    // Skip empty rows at the top so tall worlds stay readable.
    let first = (0..h)
        .find(|&y| (0..w).any(|x| !world.get(x, y).is_empty()))
        .unwrap_or(0);
    for y in first..h {
        let row: String = (0..w)
            .map(|x| {
                if bad.contains(&(x, y)) {
                    'X'
                } else {
                    glyph(world.get(x, y).material)
                }
            })
            .collect();
        println!("|{row}|");
    }
}

/// Sand with empty space below, below-left or below-right — it should have moved.
fn unstable(world: &World, w: i32, h: i32) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if world.get(x, y).material != material::SAND {
                continue;
            }
            for dx in [0, -1, 1] {
                if world.in_bounds(x + dx, y + 1) && world.is_empty(x + dx, y + 1) {
                    out.push((x, y));
                    break;
                }
            }
        }
    }
    out
}

/// Same worst-frame methodology as `scene`, but stepping both the CA sweep
/// and the field grid every frame — the combined cost the live app pays.
fn field_stress_scene(title: &str, w: i32, h: i32, frames: usize, setup: impl FnOnce(&mut World)) {
    println!("\n=== {title} ===");
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    for x in 0..w {
        world.set(x, h - 1, Cell::new(material::STONE, 0));
    }
    setup(&mut world);

    let mut worst = std::time::Duration::ZERO;
    for _ in 0..frames {
        let started = std::time::Instant::now();
        update::step(&mut world);
        world.step_fields();
        worst = worst.max(started.elapsed());
    }
    println!(
        "after {frames} frames: {}/{} chunks awake, worst frame {:.3} ms (CA + field combined)",
        world.active_chunk_count(),
        world.chunk_count(),
        worst.as_secs_f64() * 1000.0,
    );
}

fn glyph(id: MaterialId) -> char {
    match id {
        material::SAND => 'o',
        material::GRAVEL => 'O',
        material::ASH => '.',
        material::WATER => '~',
        material::OIL => ':',
        material::STONE => '#',
        material::SMOKE => '*',
        _ => ' ',
    }
}
