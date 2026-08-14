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

use pixel_physics::render::Renderer;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::field::FIELD_SCALE;
use pixel_physics::sim::material::{self, MaterialId};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::{parallel, update, Cell, World};

fn main() {
    scene("sand piling on a floor", 78, 30, 400, |w| {
        w.paint_circle(39, 2, 4, material::SAND);
    });

    // The same amount of each powder, dropped from the same height. Their
    // friction angles are 55, 45, 34 and 22 degrees, so rubble should hold the
    // sharpest peak of all, gravel a sharp one, sand a moderate one, and ash
    // should slump almost flat.
    //
    // Rubble is here to keep an honest record of how little its steeper angle
    // actually buys: it lands at rows 1/5/7/11/15/17 against gravel's
    // 1/5/8/11/14/17 -- marginally steeper, not visibly blockier, because
    // reach is quantised and 55 degrees only shifts the per-grain chance of
    // stepping sideways. `rubble.ron`'s header explains why chasing a bigger
    // difference here is the wrong lever. Kept in the scene so the comparison
    // stays visible rather than being an assertion in a comment nobody
    // re-checks.
    scene("angle of repose: rubble, gravel, sand, ash", 160, 34, 1500, |w| {
        let rubble = w.materials.id_of("rubble").expect("rubble is a compiled-in material");
        for (x, m) in [(20, rubble), (60, material::GRAVEL), (100, material::SAND), (140, material::ASH)] {
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
    // fit inside the 16.6 ms budget at 60 Hz. Run against both drivers back to
    // back — M5's whole point is that the second number should be smaller,
    // and the "unsupported cells" / awake-chunk counts should agree, which is
    // the closest this headless tool comes to "identical visual behavior
    // single- vs multi-threaded" from the plan's own verification bullet.
    let stress_setup = |w: &mut World| {
        for y in 20..160 {
            for x in 0..512 {
                let m = if y < 90 { material::SAND } else { material::WATER };
                w.set(x, y, Cell::new(m, 0));
            }
        }
    };
    scene_with("stress: a full screen of sand and water (serial)", 512, 320, 400, update::step, stress_setup);
    scene_with("stress: a full screen of sand and water (parallel, M5)", 512, 320, 400, parallel::step, stress_setup);

    // M13: the same worst case, plus the field step every frame — this is
    // what the live app actually does now (App::update runs both). The gap
    // between this number and the CA-only one above is the field grid's cost.
    let field_stress_setup = |w: &mut World| {
        stress_setup(w);
        w.add_pressure_impulse(256, 100, 20, 150.0);
    };
    field_stress_scene(
        "stress: full screen + field step every frame (serial)",
        512,
        320,
        400,
        update::step,
        field_stress_setup,
    );
    field_stress_scene(
        "stress: full screen + field step every frame (parallel, M5)",
        512,
        320,
        400,
        parallel::step,
        field_stress_setup,
    );

    // M19 first measured this with no dirty-rect skip in rendering at all --
    // a densely-filled *settled* world paid `cell_colour`'s per-pixel cost
    // (grain, heat glow) forever, not just as a stress-test edge case: 6.6ms
    // worst frame, on this exact scene. §11 added the skip; this scene now
    // shows it working (0.0ms once every chunk has settled) rather than only
    // asserting it does, since a wall-clock assertion inside `cargo test`'s
    // parallel runner is unreliable (it competes for CPU with every other
    // concurrently-running test, M5's rayon-based ones especially).
    render_stress_scene("stress: render a full screen of sand", 512, 320, stress_setup);

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

    // M16: a tree grown from a single seed, with a puddle nearby for its
    // roots to find. Should show a trunk with some branching, not a bare
    // straight line, and the puddle should visibly shrink as roots drink it.
    plant_scene("M16: a tree grows from a seed near water", 90, 70, 4000, |w| {
        for x in 0..90 {
            w.set(x, 69, Cell::new(material::STONE, 0));
        }
        w.plant_tree(45, 68);
        w.paint_circle(60, 60, 6, material::WATER);
    });

    // M16: moss spreading along a damp ledge vs. staying put on a dry one.
    plant_scene("M16: moss spreads on damp stone, stalls on dry", 90, 20, 4000, |w| {
        for x in 5..40 {
            w.set(x, 15, Cell::new(material::STONE, 0));
        }
        w.set(4, 14, Cell::new(material::STONE, 0));
        w.set(40, 14, Cell::new(material::STONE, 0));
        for x in 18..22 {
            w.set(x, 14, Cell::new(material::WATER, 0));
        }
        w.plant_moss_seed(24, 14);
        for x in 55..90 {
            w.set(x, 15, Cell::new(material::STONE, 0));
        }
        w.plant_moss_seed(72, 14);
    });

    // M17: a stone bridge anchored at both world edges, cut on one side
    // after settling. Should print twice: intact (nothing broken, since
    // every cell sits within stone's span of one end or the other), then
    // with the far span collapsed into gravel after the right anchor is
    // erased.
    //
    // **Width tracks stone's span and has to.** This was 7 wide against a
    // span of 3. Confinement and direction-weighted steps raised the span
    // well past that, and at 7 wide the scene silently stopped demonstrating
    // anything -- cutting the anchor removed one cell and the remaining six
    // stood, since all of them were now comfortably in reach of the left
    // edge. It printed a perfectly healthy-looking bridge twice and proved
    // nothing. Keep this wider than 2x the span, or it degrades the same way
    // again.
    structural_scene("M17: cutting a bridge's far support collapses the far span", 30, 15);

    terrain_generation_cost();

    // M18: a worm burrows through a sand field (should visibly relocate from
    // its seed position over the run), then a fire is lit nearby partway
    // through -- it should flee rather than burrow toward it. Prints twice:
    // after burrowing alone, and again after the fire starts.
    creature_scene("M18: a worm burrows through sand, then flees from fire", 90, 20, 100);

    // Architecture §5f/5e: closes M16's own verify criterion ("a forest
    // burns and regrows"), whose regrow half didn't exist until this
    // session. Grows a tree by a pool, burns the whole trunk down, then
    // keeps running long past burnout: `.` ash should give way to `s` soil,
    // and -- RESEED_CHANCE is a real chance, not a certainty, so this can
    // legitimately print an unlucky bare-soil patch on a given run -- `Y`
    // wood or `,` moss can reappear on top of it.
    regrowth_scene("architecture §5f/5e: a burned tree's ash decays into soil, and sometimes regrows", 60, 30);

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

    // Issue #4: field sleeping. A small isolated impulse converges within a
    // few hundred frames; the worst frame measured well after that should
    // be dramatically cheaper than the worst frame measured while it's
    // still actively propagating -- the actual, measurable claim the issue
    // asked for, not just a passing unit test.
    field_sleep_scene();
}

/// Issue #4: measures the field grid's own worst-frame cost twice on the
/// same isolated disturbance -- once while it's still actively propagating,
/// once well after it should have converged and gone quiet -- to make the
/// sleeping win a measured number rather than an assertion. Runs at the
/// sandbox's own scale (512x320, 40 chunks) since that is the scene the
/// README's own performance numbers are measured against.
fn field_sleep_scene() {
    println!("\n=== field: sleeping after convergence (issue #4) ===");
    let mut world = World::new(Rect::new(0, 0, 511, 319));
    world.end_step(); // clears the "freshly created, everything dirty" CA state without needing a real sweep -- nothing was painted, so there is genuinely nothing for it to do
    world.add_pressure_impulse(256, 160, 10, 150.0);

    let mut worst_active = std::time::Duration::ZERO;
    for _ in 0..300 {
        let started = std::time::Instant::now();
        world.step_fields();
        worst_active = worst_active.max(started.elapsed());
    }
    // 300 more frames with nothing further disturbing it -- comfortably past
    // convergence for an impulse this size (see field.rs's own settle-epsilon
    // tests, which converge within a couple hundred steps at similar scale).
    let mut worst_settled = std::time::Duration::ZERO;
    for _ in 0..300 {
        let started = std::time::Instant::now();
        world.step_fields();
        worst_settled = worst_settled.max(started.elapsed());
    }
    println!(
        "worst frame while the impulse was active: {:.4} ms; worst frame once settled: {:.4} ms",
        worst_active.as_secs_f64() * 1000.0,
        worst_settled.as_secs_f64() * 1000.0,
    );
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
    scene_with(title, w, h, frames, update::step, setup);
}

/// Same as `scene`, but with the CA step driver as a parameter — `update::step`
/// (serial) or `parallel::step` (M5) — so the same scene can be run through
/// both and compared directly.
fn scene_with(
    title: &str,
    w: i32,
    h: i32,
    frames: usize,
    step_fn: fn(&mut World),
    setup: impl FnOnce(&mut World),
) {
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
        step_fn(&mut world);
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
/// `step_fn` is the CA driver (`update::step` or `parallel::step`).
fn field_stress_scene(
    title: &str,
    w: i32,
    h: i32,
    frames: usize,
    step_fn: fn(&mut World),
    setup: impl FnOnce(&mut World),
) {
    println!("\n=== {title} ===");
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    for x in 0..w {
        world.set(x, h - 1, Cell::new(material::STONE, 0));
    }
    setup(&mut world);

    let mut worst = std::time::Duration::ZERO;
    for _ in 0..frames {
        let started = std::time::Instant::now();
        step_fn(&mut world);
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

/// M19: worst-frame timing for `Renderer::draw` alone, static scene (the
/// world never steps) -- isolates rendering's own per-pixel cost from
/// simulation cost, the way `field_stress_scene` isolates the combined
/// figure. See that scene's call site for why this lives here rather than
/// as a `cargo test` assertion.
///
/// `force_full: false` on every call after the warm-up -- exactly the case
/// §11's dirty-rect skip exists for (`render.rs`'s own doc on `Renderer::
/// draw`): a static scene sitting idle should recompute close to zero
/// pixels per frame instead of the full frame every time. `setup`'s own
/// `World::set` calls leave every touched chunk merely *dirtied*, not
/// settled (a write only arms `pending_dirty`; it takes one `end_step`
/// with nothing further written to actually promote and clear it) -- two
/// calls here, not the one call that would leave every chunk permanently
/// re-dirtying itself against its own initial paint.
fn render_stress_scene(title: &str, w: i32, h: i32, setup: impl FnOnce(&mut World)) {
    println!("\n=== {title} ===");
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    setup(&mut world);
    world.end_step();
    world.end_step();
    let mut renderer = Renderer::new();
    let particles = ParticleSystem::new();
    let mut frame = vec![0u8; (w as usize) * (h as usize) * 4];

    let warm_up_touched = world.take_touched_chunks();
    renderer.draw(&world, &particles, &warm_up_touched, &mut frame, (w as u32, h as u32), true); // warm up
    let mut worst = std::time::Duration::ZERO;
    for _ in 0..30 {
        let touched = world.take_touched_chunks();
        let started = std::time::Instant::now();
        renderer.draw(&world, &particles, &touched, &mut frame, (w as u32, h as u32), false);
        worst = worst.max(started.elapsed());
    }
    println!("worst render frame: {:.3} ms", worst.as_secs_f64() * 1000.0);
}

/// M17: builds a stone bridge exactly `w` cells wide, so its two end cells
/// touch the world's own edges (the `Cell::OUT_OF_BOUNDS` sentinel
/// `structural.rs` treats as an anchor, same as literal bedrock) -- the same
/// double-anchored geometry `cutting_a_bridges_support_makes_the_far_side_
/// collapse` exercises as a unit test, printed here so the shape of a real
/// collapse is visible rather than just asserted. Built through
/// `paint_capsule`, not raw `World::set`, deliberately: that is the same
/// entry point the player's brush uses, and it is what actually schedules
/// the reactive structural checks (see `World::paint_capsule`) -- raw `set`
/// calls (as `scene`/`scene_with` use for their pre-placed floors) leave
/// `structural.rs` untouched by design, matching how the sandbox's own
/// world-gen terrain is exempt.
/// What computing structural distances for a whole freshly-generated world
/// actually costs, on the sandbox's real 512x320 terrain.
///
/// Reported because the claim this rests on has to be measured, not argued.
/// Confinement is what makes the *search* cheap -- cells inside bulk rock
/// are anchors by local test alone and never relax, so it runs along free
/// surfaces rather than through volumes. But the measurement says the
/// seeding scan dominates (one hashed `World::get` per cell across the whole
/// world, issue #5's pattern in a new place), so the function as written
/// still scales with world volume even though its search does not. Watch
/// this number if terrain gets thicker or the world gets bigger; under M10
/// streaming it becomes a per-chunk pass and stops being world-sized at all
/// (`Reports/worldgen-design.md` §6b).
///
/// One-off generation cost, not a frame cost -- nothing here runs per frame.
fn terrain_generation_cost() {
    println!("\n=== M17: structural distances for a freshly generated world ===");
    let mut world = World::new(Rect::new(0, 0, 511, 319));
    let start = std::time::Instant::now();
    pixel_physics::app::build_terrain(&mut world);
    let with_pass = start.elapsed();

    // The same terrain again, timed without the structural pass, so the
    // figure above is attributed rather than just stated.
    let mut bare = World::new(Rect::new(0, 0, 511, 319));
    let start = std::time::Instant::now();
    pixel_physics::app::build_terrain_only(&mut bare);
    let without_pass = start.elapsed();

    let solid = (0..512).map(|x| (0..320).filter(|&y| world.get(x, y).material != material::EMPTY).count()).sum::<usize>();
    println!(
        "512x320 terrain, {solid} solid cells: {:.2} ms to build and relax, {:.2} ms to build alone \
         -- the structural pass itself is {:.2} ms, paid once at generation",
        with_pass.as_secs_f64() * 1000.0,
        without_pass.as_secs_f64() * 1000.0,
        (with_pass.saturating_sub(without_pass)).as_secs_f64() * 1000.0,
    );
}

fn structural_scene(title: &str, w: i32, h: i32) {
    println!("\n=== {title} ===");
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    let bridge_y = h / 2;
    for x in 0..w {
        world.paint_capsule((x, bridge_y), (x, bridge_y), 0, material::STONE, 1.0);
    }

    let run = |world: &mut World, frames: usize| {
        for _ in 0..frames {
            parallel::step(world);
            world.step_active_sites();
        }
    };
    let print_state = |world: &World, label: &str| {
        println!("{label} ({} active sites still pending):", world.active_site_count());
        for y in 0..h {
            let row: String = (0..w).map(|x| glyph(world.get(x, y).material)).collect();
            println!("|{row}|");
        }
    };

    run(&mut world, 400);
    print_state(&world, "before cutting the right support");

    // Erase the right anchor -- the same reactive hook the player's own
    // eraser brush goes through.
    world.paint_capsule((w - 1, bridge_y), (w - 1, bridge_y), 0, material::EMPTY, 1.0);
    run(&mut world, 400);
    print_state(&world, "after cutting the right support");
}

/// M18: plants a worm in a walled sand field, lets it burrow for
/// `frames`, then lights a fire near its original seed position and runs
/// the same number of frames again -- `w` for a live worm, `c` for a
/// corpse (starved or burned), `*` for burning cells (reusing `glyph`'s
/// existing smoke/fire-agnostic mapping would hide the fire itself, so
/// burning cells get their own marker here regardless of material).
fn creature_scene(title: &str, w: i32, h: i32, frames: usize) {
    println!("\n=== {title} ===");
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    for x in 0..w {
        world.set(x, h - 1, Cell::new(material::STONE, 0));
        world.set(x, 0, Cell::new(material::STONE, 0));
    }
    for x in 1..w - 1 {
        for y in 1..h - 1 {
            world.set(x, y, Cell::new(material::SAND, 0));
        }
    }
    let seed = (w / 4, h / 2);
    world.set(seed.0, seed.1, Cell::EMPTY);
    world.plant_worm(seed.0, seed.1);

    let run = |world: &mut World, frames: usize| {
        for _ in 0..frames {
            parallel::step(world);
            world.step_active_sites();
        }
    };
    let worm_id = world.materials.id_of("worm");
    let corpse_id = world.materials.id_of("corpse");
    let print_state = |world: &World, label: &str| {
        println!("{label} ({} active sites still pending):", world.active_site_count());
        for y in 0..h {
            let row: String = (0..w)
                .map(|x| {
                    let cell = world.get(x, y);
                    if cell.is_burning() {
                        '*'
                    } else if Some(cell.material) == worm_id {
                        'w'
                    } else if Some(cell.material) == corpse_id {
                        'c'
                    } else {
                        glyph(cell.material)
                    }
                })
                .collect();
            println!("|{row}|");
        }
    };

    run(&mut world, frames);
    print_state(&world, "after burrowing alone");

    // Ignite wherever the worm actually is now (not its original seed --
    // burrowing constantly through a dense field is expensive, and by now
    // it has both moved and spent real energy doing it), so the fire is
    // guaranteed to be an immediate threat rather than possibly landing
    // somewhere already empty of any worm to react to it.
    if let Some(worm_id) = worm_id {
        if let Some((wx, wy)) = (0..w).flat_map(|x| (0..h).map(move |y| (x, y))).find(|&(x, y)| world.get(x, y).material == worm_id) {
            world.ignite_circle(wx, wy, 4);
        }
    }
    run(&mut world, frames);
    print_state(&world, "after a fire started where the worm was");
}

/// Architecture §5f/5e. Grows a tree, burns it to the ground, then keeps
/// running the *full* frame order (CA sweep + active sites + field) long
/// past burnout -- unlike `plant_scene`'s and `creature_scene`'s own loops,
/// this one cannot skip `step_fields`, since ash decay is moisture-gated
/// and moisture only ever gets written during that phase.
fn regrowth_scene(title: &str, w: i32, h: i32) {
    println!("\n=== {title} ===");
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    for x in 0..w {
        world.set(x, h - 1, Cell::new(material::STONE, 0));
    }
    world.plant_tree(w / 2, h - 2);
    world.paint_circle(w / 2 + 10, h - 8, 4, material::WATER);

    let run = |world: &mut World, frames: usize| {
        for _ in 0..frames {
            parallel::step(world);
            world.step_active_sites();
            world.step_fields();
        }
    };
    let wood = world.materials.id_of("wood");
    let moss = world.materials.id_of("moss");
    let ash = material::ASH;
    let soil = world.materials.id_of("soil");
    let print_state = |world: &World, label: &str| {
        let counts = |m: Option<MaterialId>| -> usize {
            let Some(m) = m else { return 0 };
            (0..w).flat_map(|x| (0..h).map(move |y| (x, y))).filter(|&(x, y)| world.get(x, y).material == m).count()
        };
        println!(
            "{label}: {} wood, {} moss, {} ash, {} soil",
            counts(wood),
            counts(moss),
            counts(Some(ash)),
            counts(soil)
        );
        for y in 0..h {
            let row: String = (0..w)
                .map(|x| {
                    let m = world.get(x, y).material;
                    if Some(m) == wood {
                        'Y'
                    } else if Some(m) == moss {
                        ','
                    } else if Some(m) == soil {
                        's'
                    } else {
                        glyph(m)
                    }
                })
                .collect();
            println!("|{row}|");
        }
    };

    run(&mut world, 4000);
    print_state(&world, "grown");

    world.ignite_circle(w / 2, h - 2, 6);
    run(&mut world, 4000);
    print_state(&world, "burned");

    run(&mut world, 20_000);
    print_state(&world, "long after (ash decaying into soil, maybe regrowing)");
}

fn glyph(id: MaterialId) -> char {
    match id {
        material::SAND => 'o',
        material::GRAVEL => 'O',
        // Distinct from gravel's 'O' on purpose: the M17 scenes below are
        // read to tell collapsed stone from material that was already loose,
        // and without its own glyph rubble fell through to ' ' -- a collapsed
        // span printed as empty space, which reads as "vanished" rather than
        // "came down".
        material::RUBBLE => '@',
        material::ASH => '.',
        material::WATER => '~',
        material::OIL => ':',
        material::STONE => '#',
        material::SMOKE => '*',
        _ => ' ',
    }
}

/// M16: runs the CA sweep, the field, and the active-site scheduler
/// together (what the live app actually does), and prints growth stats
/// alongside the usual ASCII view — `Y` for wood, `,` for moss.
fn plant_scene(title: &str, w: i32, h: i32, frames: usize, setup: impl FnOnce(&mut World)) {
    println!("\n=== {title} ===");
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    setup(&mut world);

    for _ in 0..frames {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
    }

    let wood = world.materials.id_of("wood");
    let moss = world.materials.id_of("moss");
    let water_left = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| world.get(x, y).material == material::WATER).count();
    println!(
        "after {frames} frames: {} active chunks, {} active sites, {water_left} water cells remaining",
        world.active_chunk_count(),
        world.active_site_count(),
    );

    for y in 0..h {
        let row: String = (0..w)
            .map(|x| {
                let m = world.get(x, y).material;
                if Some(m) == wood {
                    'Y'
                } else if Some(m) == moss {
                    ','
                } else {
                    glyph(m)
                }
            })
            .collect();
        println!("|{row}|");
    }
}
