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
use pixel_physics::sim::pheromone::{Channel, DECAY_RHO, DEPOSIT, DIFFUSE, PHEROMONE_INTERVAL};
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

    // Stage 2 of the creature milestone: the stigmergy substrate, rendered
    // and measured before anything reads it.
    pheromone_decay_scene();
    trail_follow_scene();

    // Stage 3: the colony.
    forage_loop_scene();
    double_bridge_scene();
    nest_dig_scene();
    construction_scene();
}

/// A painted blob must **spread, fade and disappear**, and the plane must
/// then go back to costing nothing.
///
/// Both halves matter and only one of them is visible. The picture shows
/// the spread; only the tile counter can say whether a drained plane is
/// still being swept every pass (`CLAUDE.md`: "did it fire at all" needs a
/// counter). Modelled on `field_sleep_scene` above, at the sandbox's own
/// 512x320 scale, because the settled cost is what the hard gate in
/// `Reports/creature-direction.md` §9d is set against.
fn pheromone_decay_scene() {
    println!("\n=== pheromone: a blob spreads, drains to zero, and the plane goes back to sleep ===");
    let mut world = World::new(Rect::new(0, 0, 511, 319));
    world.end_step();

    let (cx, cy, r) = (256i32, 160i32, 20i32);
    for y in (cy - r)..=(cy + r) {
        for x in (cx - r)..=(cx + r) {
            if (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r * r {
                world.deposit_pheromone(Channel::A, x, y, 200);
            }
        }
    }

    let print_plane = |world: &World, label: &str| {
        // Downsampled 8x, one character per block, so a 512-wide plane fits
        // a terminal. Peak within the block, not the mean -- a trail is one
        // cell wide and a mean would erase it.
        println!("{label} (plane max {}):", world.pheromones.plane(Channel::A).max());
        for by in 0..14 {
            let row: String = (0..64)
                .map(|bx| {
                    let mut peak = 0u8;
                    for dy in 0..8 {
                        for dx in 0..8 {
                            peak = peak.max(world.pheromone_at(Channel::A, bx * 8 + dx, cy - 56 + by * 8 + dy));
                        }
                    }
                    match peak {
                        0 => ' ',
                        1..=15 => '.',
                        16..=63 => ':',
                        64..=127 => 'o',
                        128..=199 => 'O',
                        _ => '#',
                    }
                })
                .collect();
            println!("|{row}|");
        }
    };

    print_plane(&world, "at deposit");
    let mut worst_active = std::time::Duration::ZERO;
    let mut previous_max = world.pheromones.plane(Channel::A).max();
    // Long enough for the deposit to actually drain: `build_decay_lut`
    // forces at least -1 per *pass*, and a pass is one frame in
    // PHEROMONE_INTERVAL, so 200 of deposit needs 200 passes = 2,400
    // frames. Set from that arithmetic with headroom rather than from a
    // remembered figure -- the frame count and the pass interval are tied
    // together, and the first version of this scene broke the moment the
    // interval moved.
    for frame in 0..4000 {
        let started = std::time::Instant::now();
        world.step_pheromones();
        worst_active = worst_active.max(started.elapsed());
        world.frame += 1;
        let now = world.pheromones.plane(Channel::A).max();
        assert!(now <= previous_max, "frame {frame}: plane max rose from {previous_max} to {now} with nothing depositing");
        previous_max = now;
        if frame == 400 {
            print_plane(&world, "after 400 frames");
        }
    }
    print_plane(&world, "after 4000 frames");
    assert_eq!(
        world.pheromones.plane(Channel::A).max(),
        0,
        "the plane must drain to exactly zero -- a fixed point above zero is a permanent ghost trail"
    );

    // The settled cost, which is what the hard gate is on.
    let tiles_before = world.pheromones.stats.tiles_processed;
    let mut worst_settled = std::time::Duration::ZERO;
    for _ in 0..300 {
        let started = std::time::Instant::now();
        world.step_pheromones();
        worst_settled = worst_settled.max(started.elapsed());
        world.frame += 1;
    }
    println!(
        "passes {}, tiles processed while active {}, tiles processed once settled {} (must be 0)",
        world.pheromones.stats.passes,
        tiles_before,
        world.pheromones.stats.tiles_processed - tiles_before
    );
    println!(
        "worst pass while spreading: {:.4} ms; worst pass once settled: {:.4} ms (hard gate: settled < 0.5 ms)",
        worst_active.as_secs_f64() * 1000.0,
        worst_settled.as_secs_f64() * 1000.0,
    );
    assert_eq!(
        world.pheromones.stats.tiles_processed, tiles_before,
        "a drained plane must process zero tiles -- pheromone sleep is what keeps the pass off a settled world"
    );
    assert!(
        worst_settled.as_secs_f64() * 1000.0 < 0.5,
        "settled pheromone pass cost {:.4} ms, over the 0.5 ms gate",
        worst_settled.as_secs_f64() * 1000.0
    );
}

/// One Jones follower on a synthetic bent trail. **The proof that anything
/// can read the plane back**, before any creature depends on it.
///
/// Prints the path over the trail and both numbers: proximity *and* how
/// much of the trail was actually travelled. Proximity alone is the
/// Stage-0 trap -- a follower pinned at the start scored 0.988 on it while
/// going nowhere.
fn trail_follow_scene() {
    println!("\n=== pheromone: a follower tracks a bent trail (diffuse {DIFFUSE}, rho {DECAY_RHO}, SO 6) ===");
    let (w, h) = (256i32, 160i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));

    // The same geometry `pheromone.rs`'s own sweep uses: a horizontal run
    // with a bend, so tracking has to survive a turn rather than a straight
    // line a follower could hold by doing nothing at all.
    let trail_y_at = |x: i32| -> Option<i32> {
        match x {
            30..=127 => Some(80),
            128..=219 => Some(80 + (x - 128) * 30 / 92),
            _ => None,
        }
    };
    let lay = |world: &mut World| {
        for x in 30..220 {
            if let Some(y) = trail_y_at(x) {
                world.deposit_pheromone(Channel::B, x, y, DEPOSIT);
            }
        }
    };
    for _ in 0..30 {
        lay(&mut world);
        world.step_pheromones();
        world.frame += PHEROMONE_INTERVAL;
    }

    const SO: i32 = 6;
    let dirs = pixel_physics::sim::creature::DIRS;
    let (mut px, mut py) = (34i32, 80i32);
    let mut heading: u8 = 0;
    let (mut on_trail, mut furthest, mut path) = (0usize, 34i32, Vec::new());
    let steps = 400;
    for step in 0..steps {
        let sense = |h: u8, px: i32, py: i32| {
            let (dx, dy) = dirs[h as usize % 8];
            world.pheromone_at(Channel::B, px + dx * SO, py + dy * SO) as f32 / 255.0
        };
        let mut scores = [sense((heading + 1) % 8, px, py), sense(heading, px, py), sense((heading + 7) % 8, px, py)];
        // Rescaled across the candidate set -- the discrimination is in the
        // difference, not the level. Measured: 0.580 -> 0.817 on-trail.
        let hi = scores.iter().copied().fold(f32::MIN, f32::max);
        let lo = scores.iter().copied().fold(f32::MAX, f32::min);
        for v in &mut scores {
            *v = (*v - lo) / (hi - lo + 1e-6);
        }
        let mut rng = pixel_physics::sim::rng::stream(world.seed, 1, step as u64, 0);
        let pick = pixel_physics::sim::creature::choose_weighted(&scores, pixel_physics::sim::creature::CHOICE_EXPLORATION_K, rng.unit_f32());
        heading = match pick {
            0 => (heading + 1) % 8,
            2 => (heading + 7) % 8,
            _ => heading,
        };
        let (dx, dy) = dirs[heading as usize];
        px = (px + dx).clamp(1, w - 2);
        py = (py + dy).clamp(1, h - 2);
        path.push((px, py));
        if let Some(ty) = trail_y_at(px) {
            if (py - ty).abs() <= 2 {
                on_trail += 1;
                furthest = furthest.max(px);
            }
        }
        lay(&mut world);
        world.step_pheromones();
        world.frame += PHEROMONE_INTERVAL;
    }

    // The picture: `-` is the trail, `*` where the follower walked it, `+`
    // where it wandered off. Two world columns per character, so a 256-wide
    // world fits a terminal.
    let walked: std::collections::HashSet<(i32, i32)> = path.iter().copied().collect();
    for y in 70..118 {
        let row: String = (24..232)
            .step_by(2)
            .map(|x| {
                let on = walked.contains(&(x, y)) || walked.contains(&(x + 1, y));
                let is_trail = trail_y_at(x) == Some(y) || trail_y_at(x + 1) == Some(y);
                match (on, is_trail) {
                    (true, true) => '*',
                    (true, false) => '+',
                    (false, true) => '-',
                    (false, false) => ' ',
                }
            })
            .collect();
        println!("|{row}|");
    }
    let on = on_trail as f32 / steps as f32;
    let traversed = (furthest - 34) as f32 / (219.0 - 34.0);
    println!("on-trail {on:.3} of {steps} steps, traversed {traversed:.3} of the trail; deposits {}", world.pheromones.stats.deposits_b);
    println!("(one seed; the guard in pheromone.rs gates the 6-seed mean -- measured 0.817 on-trail / 0.961 traversed, against a 0.050 no-trail control)");
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
        "512x320 hand-authored terrain, {solid} solid cells: {:.2} ms to build and relax, {:.2} ms to build alone \
         -- the structural pass itself is {:.2} ms, paid once at generation",
        with_pass.as_secs_f64() * 1000.0,
        without_pass.as_secs_f64() * 1000.0,
        (with_pass.saturating_sub(without_pass)).as_secs_f64() * 1000.0,
    );

    // The same attribution for a *generated* world, which is what the app
    // actually builds now. Worth measuring separately rather than assuming
    // it tracks the figure above: the generated massif fills most of the
    // world instead of a floor and three ledges, and the structural pass
    // scales with how much solid there is to relax.
    let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        println!("worldgen presets unavailable ({e}); skipping generated-terrain timing");
        return;
    }
    let name = presets.default_name();
    let Some(params) = presets.get(&name) else { return };
    let spec = || pixel_physics::worldgen::Spec::Generated { params, seed: 1 };

    let mut gen = World::new(Rect::new(0, 0, 511, 319));
    let start = std::time::Instant::now();
    pixel_physics::worldgen::generate(&mut gen, spec());
    let gen_with_pass = start.elapsed();

    let mut gen_bare = World::new(Rect::new(0, 0, 511, 319));
    let start = std::time::Instant::now();
    pixel_physics::worldgen::generate_only(&mut gen_bare, spec());
    let gen_without_pass = start.elapsed();

    let gen_solid =
        (0..512).map(|x| (0..320).filter(|&y| gen.get(x, y).material != material::EMPTY).count()).sum::<usize>();
    println!(
        "512x320 generated terrain ({name}, seed 1), {gen_solid} solid cells: {:.2} ms to build and relax, \
         {:.2} ms to place alone -- the structural pass itself is {:.2} ms, paid once at generation \
         (and again on every F6 reroll)",
        gen_with_pass.as_secs_f64() * 1000.0,
        gen_without_pass.as_secs_f64() * 1000.0,
        (gen_with_pass.saturating_sub(gen_without_pass)).as_secs_f64() * 1000.0,
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
        // The "did it fire" number beside the picture (CLAUDE.md): a worm
        // is an organism now, so a live organism count says whether the
        // substrate is holding it -- and, on the second panel, whether
        // death actually returned the slot. A picture cannot show either;
        // an inert worm cell and a live one draw identically.
        let worms = (0..w).flat_map(|x| (0..h).map(move |y| (x, y))).filter(|&(x, y)| Some(world.get(x, y).material) == worm_id).count();
        let corpses = (0..w).flat_map(|x| (0..h).map(move |y| (x, y))).filter(|&(x, y)| Some(world.get(x, y).material) == corpse_id).count();
        println!(
            "{label}: {} live organisms, {worms} worm cells, {corpses} corpse cells, {} active sites still pending",
            world.live_organism_count(),
            world.active_site_count()
        );
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


/// **The scene that proves the loop, not its parts.** Ants leave a nest,
/// find a food pile, carry from it, come home, and a channel-B trail forms
/// between the two.
///
/// Counters print beside the picture because the picture cannot answer the
/// only question that matters here: a colony milling plausibly and a colony
/// genuinely foraging look identical at this zoom. `deliveries` is the
/// number that separates them.
fn forage_loop_scene() {
    println!("\n=== ants: the foraging loop (nest, food pile, 60 ants) ===");
    let (w, h) = (512i32, 120i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    let floor = h - 8;
    let nest = world.materials.id_of("nest").expect("nest is compiled in");
    let leaf = world.materials.id_of("leaf").expect("leaf is compiled in");
    let ant = world.materials.id_of("ant").expect("ant is compiled in");

    // **Real generated terrain, the configuration the loop was actually
    // verified under.** Two cheaper stand-ins were tried and both were
    // worse than the thing they stood in for: a flat floor is degenerate
    // (an ant there has one legal step, so it is not deciding anything --
    // 248 distinct cells visited against 1,670 on real terrain), and a
    // hand-built ridge profile produced one-column cliffs that ants walked
    // off constantly. `examples/ant_ablation.rs` measures 28.8 deliveries
    // per run on generated terrain across four seeds; this scene shows the
    // same thing rather than a simplification of it.
    let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        println!("worldgen presets unavailable ({e}); skipping the foraging scene");
        return;
    }
    let Some(params) = presets.get(&presets.default_name()) else { return };
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed: 1 });
    // The surface is whatever the generator left, per column.
    let surface = |world: &World, x: i32| -> i32 {
        (0..h)
            .find(|&y| matches!(world.materials.kind(world.get(x, y).material), material::MaterialKind::Solid | material::MaterialKind::Powder))
            .unwrap_or(h - 1)
    };

    // A wide home patch, not a doorway: nest scent is refreshed wherever
    // ants touch it, so the colony's *home range* is what the gradient is
    // anchored to.
    for x in 16..90 {
        let sy = surface(&world, x);
        world.set(x, sy, Cell::new(nest, 0).with_attached(true));
    }
    // **A stand of trees, not a pile of corpses, and this is the change
    // that made the loop close at all.** A corpse pile is finite and
    // concentrated: almost no ant ever found it, so the colony could not
    // demonstrate a foraging loop it never entered -- 2.5 pickups and *zero*
    // deliveries per run. Trees regrow their leaves continuously and spread
    // them over a wide area, which is both a renewable food source and a
    // findable one. Same ants, same brain, same code: 44.8 pickups and 28.8
    // deliveries.
    //
    // Herbivory needed no new code -- a `Leaf` cell is just a cell, and the
    // tree discovers it has lost one through its own connectivity check.
    for i in 0..6 {
        let x = 230 + i * 40;
        let sy = surface(&world, x);
        world.plant_tree(x, sy - 1);
    }
    // Let the stand actually grow before the ants arrive; a seedling has no
    // leaves to eat.
    for _ in 0..2400 {
        world.step_active_sites();
        world.step_fields();
    }
    for i in 0..55 {
        let ax = 24 + i * 4;
        let sy = surface(&world, ax);
        world.plant_ant(ax, sy - 1);
    }

    let print_state = |world: &World, label: &str| {
        let st = world.creature_stats;
        println!(
            "{label}: {} live organisms | moves {} blocked {} falls {} | eats {} pickups {} digs {} drops {} deliveries {} nest-visits {} deaths {}",
            world.live_organism_count(),
            st.moves,
            st.moves_blocked,
            st.falls,
            st.eats,
            st.pickups,
            st.digs,
            st.drops,
            st.deliveries,
            st.nest_visits,
            st.deaths
        );
        let food_left = (0..w).flat_map(|x| (0..h).map(move |y| (x, y))).filter(|&(x, y)| world.get(x, y).material == leaf).count();
        let phero_b: u64 = (0..w).flat_map(|x| (0..h).map(move |y| (x, y))).map(|(x, y)| world.pheromone_at(Channel::B, x, y) as u64).sum();
        let phero_a: u64 = (0..w).flat_map(|x| (0..h).map(move |y| (x, y))).map(|(x, y)| world.pheromone_at(Channel::A, x, y) as u64).sum();
        // **In thirds, not as one total.** A homing gradient that points
        // home and one that is merely present look the same in a single
        // sum; what has to be true is that channel A is strongest at the
        // nest end, which is the only reason a laden ant walking
        // up-gradient ends up somewhere useful.
        let band = |ch: Channel, lo: i32, hi: i32| -> u64 {
            (lo..hi).flat_map(|x| (0..h).map(move |y| (x, y))).map(|(x, y)| world.pheromone_at(ch, x, y) as u64).sum()
        };
        // Where the carriers are, which the verb counters alone cannot say:
        // a colony with pickups and no deliveries is either failing to pick
        // things up or failing to get home, and those want opposite fixes.
        let mut carriers = Vec::new();
        for x in 0..w {
            for y in 0..h {
                let c = world.get(x, y);
                if c.material == ant {
                    if let Some(st) = world.organism(c.organism_id()) {
                        if st.carrying.is_some() && st.chain.first() == Some(&(x, y)) {
                            carriers.push(x);
                        }
                    }
                }
            }
        }
        let mean_x = if carriers.is_empty() { -1.0 } else { carriers.iter().sum::<i32>() as f32 / carriers.len() as f32 };
        println!("  carrying right now: {} ants, mean x {mean_x:.0}", carriers.len());
        println!(
            "  food cells {food_left} | channel A {phero_a} (nest third {} / mid {} / food third {}) | channel B {phero_b} (nest {} / mid {} / food {})",
            band(Channel::A, 0, w / 3),
            band(Channel::A, w / 3, 2 * w / 3),
            band(Channel::A, 2 * w / 3, w),
            band(Channel::B, 0, w / 3),
            band(Channel::B, w / 3, 2 * w / 3),
            band(Channel::B, 2 * w / 3, w),
        );
        // Wide enough to include the canopy: the trees are the food, and a
        // scene that crops them out cannot be judged by eye at all.
        for y in (floor - 56)..h {
            let row: String = (0..w)
                .step_by(2)
                .map(|x| {
                    let pick = |x: i32| {
                        let c = world.get(x, y);
                        if c.material == ant {
                            Some('a')
                        } else if c.material == nest {
                            Some('N')
                        } else if c.material == leaf {
                            Some('f')
                        } else if c.material != material::EMPTY {
                            Some(glyph(c.material))
                        } else if world.pheromone_at(Channel::B, x, y) > 12 {
                            Some('-')
                        } else if world.pheromone_at(Channel::A, x, y) > 12 {
                            Some('.')
                        } else {
                            None
                        }
                    };
                    pick(x).or_else(|| pick(x + 1)).unwrap_or(' ')
                })
                .collect();
            println!("|{row}|");
        }
    };

    print_state(&world, "at spawn");
    // **Worst-frame, on the heaviest creature scene there is.** Nothing in
    // this harness timed a colony before -- the number CI gates was
    // measured entirely on scenes with no creatures in them, so 55 ants, 9
    // beetles and a 248-weight brain had never once been costed. `sum`
    // gives the mean beside it because a single worst frame is a sample
    // from a wide distribution (`CLAUDE.md`: compare two runs, not one run
    // against a remembered number) and the mean is what a player feels.
    let mut worst = std::time::Duration::ZERO;
    let mut sum = std::time::Duration::ZERO;
    let mut frames_run = 0usize;
    let mut run = |world: &mut World, frames: usize| {
        for _ in 0..frames {
            let started = std::time::Instant::now();
            parallel::step(world);
            world.step_active_sites();
            world.step_fields();
            world.step_pheromones();
            let took = started.elapsed();
            worst = worst.max(took);
            sum += took;
            frames_run += 1;
        }
    };
    run(&mut world, 2000);
    print_state(&world, "after 2000 frames");
    run(&mut world, 10000);
    print_state(&world, "after 12000 frames");
    println!(
        "  frame cost with {} live organisms: worst {:.3} ms, mean {:.3} ms over {frames_run} frames",
        world.live_organism_count(),
        worst.as_secs_f64() * 1000.0,
        sum.as_secs_f64() * 1000.0 / frames_run as f64
    );

    // **What holds, and what does not.** The outbound half of the loop is
    // real and asserted; the return half is not, and is printed with the
    // measurement rather than asserted into existence. See
    // `Reports/creature-direction.md` §13 for the diagnosis -- on flat
    // ground the ahead-left and ahead-right sensors both sit in open air, so
    // the lateral input is identically zero and a laden ant has no way to
    // express "turn around".
    // **A guard again, not a printed note.** This assertion was demoted
    // when the loop would not close -- carriers picked food up and never
    // got home. Both halves of that were real and both are fixed: homing
    // needed the along-heading gradient and the tumble (report §13e), and
    // the loop needed a food source ants could actually *find*, which a
    // finite corpse pile never was (§13f). Measured 28.8 deliveries per run
    // across 4 seeds; the bar sits far below that, because outcome spread
    // here is large and a bar near the measurement flakes.
    let st = world.creature_stats;
    assert!(st.moves > 0, "no ant ever moved");
    assert!(st.pickups > 0, "no ant ever picked food up -- the outbound half of the loop is broken");
    assert!(st.nest_visits > 0, "no ant ever reached the nest");
    let phero_b: u64 = (0..w).flat_map(|x| (0..h).map(move |y| (x, y))).map(|(x, y)| world.pheromone_at(Channel::B, x, y) as u64).sum();
    assert!(phero_b > 0, "carriers laid no food trail at all");
    assert!(
        st.deliveries > 0,
        "no ant completed the loop: {} pickups but nothing delivered home. This is report §9a's own criterion",
        st.pickups
    );

    // The census. Any imbalance is a bug now and an evolutionary attractor
    // once stage 4 turns mutation on.
    let live = world.live_creature_energy();
    let expected = world.energy_ledger.expected_live_total();
    println!(
        "energy census: live {live:.2} vs ledger {expected:.2} (delta {:.4}); granted {:.0} eaten {:.0} metabolized {:.0} moved {:.0} synapses {:.2} died-holding {:.0}",
        live - expected,
        world.energy_ledger.granted,
        world.energy_ledger.eaten,
        world.energy_ledger.metabolized,
        world.energy_ledger.moved,
        world.energy_ledger.synapse_tax,
        world.energy_ledger.died_holding
    );
}

/// Shared runner for the colony scenes: the full frame order the live app
/// uses, so nothing here is testing a phase the player never sees.
fn run_colony(world: &mut World, frames: usize) {
    for _ in 0..frames {
        parallel::step(world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
    }
}

/// Mean channel-B value over a rectangle, as a float — a *sum* would be
/// dominated by whichever region has more cells in it, and the two routes
/// here deliberately do not.
fn mean_b(world: &World, x0: i32, x1: i32, y0: i32, y1: i32) -> f32 {
    let mut total = 0u64;
    let mut n = 0u64;
    for x in x0..x1 {
        for y in y0..y1 {
            total += world.pheromone_at(Channel::B, x, y) as u64;
            n += 1;
        }
    }
    if n == 0 {
        0.0
    } else {
        total as f32 / n as f32
    }
}

/// **The double bridge, made of terrain.** Two routes from nest to food,
/// one short and one long, separated by a wall with a tunnel through it at
/// ground level and a climb over the top.
///
/// P-16: in side view the two routes have to come from *geometry*. Two
/// distances on flat ground is not a bridge experiment here — there is
/// nothing forcing an ant onto one branch or the other, so it tests
/// nothing. The published figures are top-down arenas and a side-view strip
/// has fewer competing paths, so expect this to look less dramatic than the
/// literature; that is the orientation, not a bug.
fn double_bridge_scene() {
    println!("\n=== ants: a double bridge made of terrain (short tunnel vs the long way over) ===");
    let (w, h) = (240i32, 120i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    let floor = h - 8;
    let nest = world.materials.id_of("nest").expect("nest");
    let corpse = world.materials.id_of("corpse").expect("corpse");

    for x in 0..w {
        for y in floor..h {
            world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
    for x in 16..48 {
        world.set(x, floor, Cell::new(nest, 0).with_attached(true));
    }
    // The wall, with a two-cell tunnel at ground level. Over the top is a
    // climb of 40 cells each way; through the tunnel is six.
    const WALL_X0: i32 = 120;
    const WALL_X1: i32 = 126;
    const WALL_TOP: i32 = 70;
    const TUNNEL_TOP: i32 = 110;
    for x in WALL_X0..WALL_X1 {
        for y in WALL_TOP..floor {
            if y < TUNNEL_TOP {
                world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
    }
    for x in 170..210 {
        for y in (floor - 5)..floor {
            world.set(x, y, Cell::new(corpse, 0));
        }
    }
    for i in 0..60 {
        world.plant_ant(20 + i * 2, floor - 1);
    }

    // **Integrated over the run, not read off the end.** A trail is a
    // standing quantity that evaporates, so a single reading at frame
    // 16,000 reports only whoever walked past in the last few seconds --
    // it says nothing about which route the colony *used*. Sampling
    // periodically and summing measures traffic, which is the question.
    let (mut short, mut long) = (0.0f32, 0.0f32);
    for _ in 0..32 {
        run_colony(&mut world, 500);
        short += mean_b(&world, WALL_X0, WALL_X1, TUNNEL_TOP, floor);
        long += mean_b(&world, WALL_X0, WALL_X1, WALL_TOP - 3, WALL_TOP);
    }
    let st = world.creature_stats;
    println!(
        "  moves {} blocked {} | pickups {} drops {} deliveries {} nest-visits {} deaths {}",
        st.moves, st.moves_blocked, st.pickups, st.drops, st.deliveries, st.nest_visits, st.deaths
    );
    println!("  summed channel B over 32 samples -- short route (tunnel) {short:.2} vs long route (over the top) {long:.2}");
    // **Recorded, not asserted, and the earlier version of this assertion
    // was measuring the wrong thing.** An intermediate build did report
    // 21.00 on the short route against 6.22 on the long one and passed --
    // but trail *following* was not what produced it. On flat ground the
    // ahead-left and ahead-right sensors both sit in open air, so the
    // lateral input is identically zero (measured directly with
    // `examples/creature_probe.rs`: an ant standing on a cell holding A=27
    // reads `pheroA_lr = 0.000`). What that run measured was ants using the
    // only route a ground-dwelling creature can use, and depositing on it:
    // geometry, not stigmergy. A guard that cannot fail for the right
    // reason is worse than none (`CLAUDE.md`), so this prints and the
    // weaker claims that *do* hold are what get asserted.
    println!("  NOTE: route selection is not yet demonstrated -- see Reports/creature-direction.md §13.");
    assert!(st.moves > 0 && st.pickups > 0, "ants should at least have crossed and foraged");
    assert!(st.nest_visits > 0, "ants should have reached the nest side");
}

/// 50+ ants and a block of diggable soil. Excavation is a *consequence* of
/// dig_force against the material's own `penetration_resistance` — soil is
/// 0.8 and an ant pushes 1.0, so it chews soil and is stopped by everything
/// harder without a single material name appearing in the code.
fn nest_dig_scene() {
    println!("\n=== ants: excavating a chamber out of soil (and stopped by stone) ===");
    let (w, h) = (200i32, 120i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    let floor = h - 8;
    let soil = world.materials.id_of("soil").expect("soil");
    let nest = world.materials.id_of("nest").expect("nest");

    for x in 0..w {
        for y in floor..h {
            world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
    // A soil bank with a stone floor under it: the ants should hollow the
    // soil and leave the stone alone.
    for x in 40..160 {
        for y in (floor - 30)..floor {
            world.set(x, y, Cell::new(soil, 0).with_attached(true));
        }
    }
    for x in 16..40 {
        world.set(x, floor, Cell::new(nest, 0).with_attached(true));
    }
    let soil_before = (0..w).flat_map(|x| (0..h).map(move |y| (x, y))).filter(|&(x, y)| world.get(x, y).material == soil).count();
    for i in 0..55 {
        world.plant_ant(20 + i % 10 * 2, floor - 1 - (i / 10));
    }

    run_colony(&mut world, 8000);

    let soil_after = (0..w).flat_map(|x| (0..h).map(move |y| (x, y))).filter(|&(x, y)| world.get(x, y).material == soil).count();
    let stone_floor: usize = (0..w).map(|x| usize::from(world.get(x, h - 1).material == material::STONE)).sum();
    let st = world.creature_stats;
    println!("  digs {} | moves {} blocked {} deaths {}", st.digs, st.moves, st.moves_blocked, st.deaths);
    println!("  soil {soil_before} -> {soil_after} ({} excavated), stone floor intact {stone_floor}/{w}", soil_before - soil_after.min(soil_before));
    assert!(st.digs > 0, "no ant ever dug -- the verb never fired, whatever the picture shows");
    assert!(soil_after < soil_before, "soil should have been excavated: {soil_before} -> {soil_after}");
    assert_eq!(stone_floor as i32, w, "ants must not have dug through stone -- dig_force 1.0 is below stone's penetration_resistance");
}

/// Carriers over a moisture gradient. **Deposition bias, not a build
/// script** (`stigmergy-research.md` §4, the eLife 2024 result): drop
/// probability is multiplied by local `|grad moisture|`, so material
/// accumulates where the gradient is steep. Pillars and walls are
/// consequences of that, and writing a "build a wall" behaviour would be
/// the signal to go and re-read that section.
fn construction_scene() {
    println!("\n=== ants: deposition follows the moisture gradient, with no build rule anywhere ===");
    let (w, h) = (240i32, 120i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    let floor = h - 8;
    let corpse = world.materials.id_of("corpse").expect("corpse");

    for x in 0..w {
        for y in floor..h {
            world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
    // Water sunk in a walled well on the left half only, so the left half
    // carries a real moisture gradient and the right half is flat and dry.
    // No nest at all: this scene is about the *route*, and an ant that can
    // deliver has no reason to build.
    for y in (floor - 6)..floor {
        world.set(60, y, Cell::new(material::STONE, 0).with_attached(true));
        world.set(68, y, Cell::new(material::STONE, 0).with_attached(true));
    }
    for x in 61..68 {
        for y in (floor - 5)..floor {
            world.set(x, y, Cell::new(material::WATER, 0));
        }
    }
    // Food spread thinly across the whole floor, so ants pick up wherever
    // they are and then carry across both halves.
    for x in (20..220).step_by(4) {
        world.set(x, floor - 1, Cell::new(corpse, 0));
    }
    for i in 0..55 {
        world.plant_ant(22 + i * 3, floor - 2);
    }

    run_colony(&mut world, 10000);

    // Which half is which is measured, not assumed: the well is on the
    // left, so the left half should be the steep one -- but a scene that
    // contradicts the code looks like a bug in the code, so check.
    let mean_grad = |x0: i32, x1: i32| -> f32 {
        let mut total = 0.0;
        let mut n = 0.0;
        for x in x0..x1 {
            for y in (floor - 10)..floor {
                let gx = world.field_at_bilinear((x + 4) as f32, y as f32).moisture - world.field_at_bilinear((x - 4) as f32, y as f32).moisture;
                let gy = world.field_at_bilinear(x as f32, (y + 4) as f32).moisture - world.field_at_bilinear(x as f32, (y - 4) as f32).moisture;
                total += (gx * gx + gy * gy).sqrt();
                n += 1.0;
            }
        }
        total / n
    };
    let (wet_grad, dry_grad) = (mean_grad(20, w / 2), mean_grad(w / 2, 220));
    // Drops land as material, so count what is standing where nothing was
    // placed: any corpse cell not on the original 4-cell lattice row.
    let dropped = |x0: i32, x1: i32| -> usize {
        (x0..x1)
            .flat_map(|x| ((floor - 12)..floor).map(move |y| (x, y)))
            .filter(|&(x, y)| world.get(x, y).material == corpse && !(y == floor - 1 && x % 4 == 0))
            .count()
    };
    let (wet_drops, dry_drops) = (dropped(20, w / 2), dropped(w / 2, 220));
    let st = world.creature_stats;
    println!("  pickups {} drops {} digs {} deaths {}", st.pickups, st.drops, st.digs, st.deaths);
    println!("  mean |grad moisture|: steep half {wet_grad:.3}, flat half {dry_grad:.3}");
    println!("  material left standing: steep half {wet_drops}, flat half {dry_drops}");
    assert!(wet_grad > dry_grad, "the scene must actually contain the gradient it is testing: {wet_grad:.3} vs {dry_grad:.3}");
    assert!(st.drops > 0, "no ant ever dropped anything -- the verb never fired");
    assert!(
        wet_drops > dry_drops,
        "deposition should cluster where the moisture gradient is steep: {wet_drops} vs {dry_drops}"
    );
}