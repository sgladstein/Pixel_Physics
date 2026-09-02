//! Renders the simulation to the terminal.
//!
//! Movement rules are far easier to judge by eye than by assertion, and this
//! needs no window or GPU — so it works over a remote shell and in CI. Run with:
//!
//! ```text
//! cargo run --example ascii                        # every scene
//! cargo run --example ascii -- scene=foraging      # just the one
//! cargo run --example ascii -- scene=ants,field    # any that match either
//! cargo run --example ascii -- skip=foraging       # everything but that one
//! cargo run --example ascii -- list=1              # the catalogue, run nothing
//! ```
//!
//! `scene=` matches a case-insensitive substring of the title a scene prints,
//! and an unknown argument or a term that matches nothing is an error rather
//! than a silent empty run — see [`SceneFilter`].
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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

/// Which scenes this run should execute: `scene=<term>[,<term>...]`, plus
/// `list=1` to print the catalogue and run nothing.
///
/// **Why this exists.** `ascii` is the repo's behaviour-and-worst-frame
/// harness, and it had no way to run one scene. Two costs followed, and
/// neither was hypothetical. Iterating on a single scene meant a full pass
/// over all nineteen — 3m28s, measured on this machine — so a scene was
/// re-run about as rarely as that price implies. And CI could only quarantine
/// the *whole example* when one scene went red: `.github/workflows/ci.yml`
/// marked this job `continue-on-error` over bug H with the reason written out
/// in its own comment — "ascii has no scene selection, so bug H cannot be
/// excluded by name". A second and much larger regression then landed behind
/// that blanket quarantine and went unseen through two commits
/// (`forage_loop_scene`, 98 round trips -> 2). ci.yml names "once `ascii`
/// learns to select scenes" as one of the two things that lets the job gate
/// again; this is that.
///
/// **Matched against the scene's own printed title, case-insensitively, as a
/// substring** — deliberately *not* a second registry of short keys. A
/// parallel list of names is one more thing to fall out of step with what is
/// actually on screen, and the title is already there. `scene=ants` runs the
/// four ant scenes; `scene=foraging` runs one.
///
/// With no `scene=` argument every scene runs, in the same order, printing
/// the same text — the default path is unchanged, so every baseline measured
/// before this still compares.
struct SceneFilter {
    /// Lower-cased substrings. Empty means "run everything".
    terms: Vec<String>,
    /// Lower-cased substrings to *exclude*, applied after `terms`.
    ///
    /// This is what lets CI gate on the healthy scenes while a known red is
    /// quarantined **by name** rather than by turning the whole example
    /// non-blocking -- the failure mode that let a second, larger regression
    /// in behind bug H unnoticed. The same shape as the `--skip` the test
    /// jobs already use for bug A.
    skip: Vec<String>,
    /// `list=1`: print every scene's title and run none of them. Costs one
    /// fast pass precisely because every scene is skipped.
    list_only: bool,
}

impl SceneFilter {
    /// **An unknown argument is rejected, never ignored.** `CLAUDE.md`
    /// records a 3.5-hour study that produced eight byte-identical logs
    /// because `worldseed=` reached a binary that had never heard of it: a
    /// harness that shrugs at a typo cannot be trusted to have run what you
    /// asked for, and it looks exactly like one that did.
    fn from_args() -> Self {
        let mut terms = Vec::new();
        let mut skip = Vec::new();
        let mut list_only = false;
        for arg in std::env::args().skip(1) {
            if let Some(v) = arg.strip_prefix("scene=") {
                terms.extend(v.split(',').filter(|s| !s.is_empty()).map(str::to_ascii_lowercase));
            } else if let Some(v) = arg.strip_prefix("skip=") {
                skip.extend(v.split(',').filter(|s| !s.is_empty()).map(str::to_ascii_lowercase));
            } else if let Some(v) = arg.strip_prefix("list=") {
                list_only = v != "0";
            } else {
                eprintln!("ascii: unknown argument {arg:?}");
                eprintln!("usage: ascii [scene=<term>[,<term>...]] [skip=<term>[,<term>...]] [list=1]");
                std::process::exit(2);
            }
        }
        Self { terms, skip, list_only }
    }
}

static FILTER: OnceLock<SceneFilter> = OnceLock::new();
static SCENES_RUN: AtomicUsize = AtomicUsize::new(0);
static SCENES_SKIPPED: AtomicUsize = AtomicUsize::new(0);

fn filter() -> &'static SceneFilter {
    FILTER.get_or_init(SceneFilter::from_args)
}

/// Print a scene's header, and say whether the scene should run at all.
///
/// Every scene here opens with this instead of printing its own
/// `=== title ===` line, and that is what makes the filter total: a scene
/// that printed its own header would be a scene the filter could not skip,
/// and there would be no second place to notice the omission.
fn begin(title: &str) -> bool {
    let f = filter();
    if f.list_only {
        println!("  {title}");
        SCENES_SKIPPED.fetch_add(1, Ordering::Relaxed);
        return false;
    }
    let lower = title.to_ascii_lowercase();
    if !f.terms.is_empty() && !f.terms.iter().any(|t| lower.contains(t.as_str())) {
        SCENES_SKIPPED.fetch_add(1, Ordering::Relaxed);
        return false;
    }
    if f.skip.iter().any(|t| lower.contains(t.as_str())) {
        SCENES_SKIPPED.fetch_add(1, Ordering::Relaxed);
        return false;
    }
    SCENES_RUN.fetch_add(1, Ordering::Relaxed);
    println!("\n=== {title} ===");
    true
}

/// **A filter that matches nothing is an error, not an empty run.** A typo in
/// `scene=` would otherwise exit 0 having asserted nothing whatsoever, which
/// is indistinguishable from a clean pass — the same shape of failure as the
/// skipped-CI-step one that hid bug H for a month.
fn finish() {
    let (ran, skipped) = (SCENES_RUN.load(Ordering::Relaxed), SCENES_SKIPPED.load(Ordering::Relaxed));
    if filter().list_only {
        println!("ascii: {skipped} scenes available");
        return;
    }
    println!("\nascii: {ran} scenes run, {skipped} skipped");
    if ran == 0 {
        eprintln!(
            "ascii: scene={} skip={} left none of the {skipped} scenes to run",
            filter().terms.join(","),
            filter().skip.join(",")
        );
        std::process::exit(2);
    }
}

fn main() {
    // The harness names its own parameters on line one, so a log always says
    // what produced it (`CLAUDE.md`: "a knob nobody can see the value of is a
    // knob nobody can tell is disconnected").
    let f = filter();
    if f.list_only {
        println!("ascii: list=1 -- the scene catalogue; nothing is run");
    } else {
        let picked = if f.terms.is_empty() { "<unset>".to_string() } else { f.terms.join(",") };
        let dropped = if f.skip.is_empty() { "<none>".to_string() } else { f.skip.join(",") };
        println!("ascii: scene={picked} skip={dropped}");
    }

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
    // Harness size first (the historical series), then THE SHIPPED WORLD
    // SIZE — the second is the number that actually decides the rivers
    // track, because the field-step's per-sleeping-tile clone grows with
    // world area while the river's own awake ring does not.
    river_cost_scene(512, 320);
    river_cost_scene(pixel_physics::app::WORLD_WIDTH as i32, pixel_physics::app::WORLD_HEIGHT as i32);

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
    field_scaling_scene();
    field_day_scene();

    // Stage 2 of the creature milestone: the stigmergy substrate, rendered
    // and measured before anything reads it.
    pheromone_decay_scene();
    trail_follow_scene();

    // Stage 3: the colony.
    forage_loop_scene();
    double_bridge_scene();
    nest_dig_scene();
    construction_scene();

    finish();
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
    if !begin("pheromone: a blob spreads, drains to zero, and the plane goes back to sleep") {
        return;
    }
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
    if !begin(&format!("pheromone: a follower tracks a bent trail (diffuse {DIFFUSE}, rho {DECAY_RHO}, SO 6)")) {
        return;
    }
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
    if !begin("field: sleeping after convergence (issue #4)") {
        return;
    }
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

/// **What one whole day/night cycle costs a settled world**, in awake field
/// tiles and in frame time.
///
/// `field_sleep_scene` above measures a world nothing is driving. This one
/// measures the world the sky *is* driving: 3600 frames is exactly
/// `DAY_NIGHT_PERIOD_FRAMES`, so this is one sunrise, one noon, one sunset and
/// one midnight over ground that has otherwise stopped moving.
///
/// The number to watch is `awake tile-frames`. A picture cannot answer this
/// and neither can a timing at this scale — `CLAUDE.md`: "did it fire at all"
/// needs a counter. The specific hazard it is aimed at is a sky channel that
/// forces the temperature of the surface and then lets it diffuse downward:
/// field temperature has no decay term (unlike light's `LIGHT_DECAY`), so an
/// over-strong or under-attenuated forcing propagates a thermal wave into
/// buried tiles that were asleep and keeps waking them, all night, forever.
/// Buried tiles are what the ground below is here to provide — the
/// reassurance that "sky-lit tiles never sleep anyway" does not cover them.
fn field_day_scene() {
    if !begin("field: one full day/night cycle over settled ground") {
        return;
    }
    let mut world = World::new(Rect::new(0, 0, 511, 319));
    // 160 rows of stone under 160 rows of sky: half the tiles are buried, and
    // buried is the state the measurement is about.
    for y in 160..320 {
        for x in 0..512 {
            world.set(x, y, Cell::new(material::STONE, 0));
        }
    }
    world.end_step();
    // Settle first: the count below must be the *standing* state, not the
    // transient of a world that has just been painted.
    for _ in 0..200 {
        world.step_fields();
    }

    let day = 3600;
    let mut awake_tile_frames: u64 = 0;
    let mut worst = std::time::Duration::ZERO;
    let before = world.field_stats;
    let started = std::time::Instant::now();
    for _ in 0..day {
        world.begin_step(); // the sky is a function of `world.frame`, so a day only passes if the frame counter does
        let t = std::time::Instant::now();
        world.step_fields();
        worst = worst.max(t.elapsed());
        awake_tile_frames += world.awake_field_tiles() as u64;
        world.end_step();
    }
    let total = started.elapsed();
    // Both counters, because they answer different questions and the first
    // one alone reads zero for two opposite reasons -- see `FieldStats`.
    // `solved` is tiles the solver actually ran passes over; `awake` is tiles
    // left unconverged afterwards, which is what the *next* frame inherits.
    println!(
        "over {day} frames: {} solves over {} passes, {awake_tile_frames} awake tile-frames left behind, worst frame {:.4} ms, total {:.1} ms",
        world.field_stats.tiles_solved - before.tiles_solved,
        world.field_stats.passes - before.passes,
        worst.as_secs_f64() * 1000.0,
        total.as_secs_f64() * 1000.0,
    );
}

/// How the field grid's cost scales with world size, and — the number that
/// actually matters for a bigger world — what a *localised* disturbance costs.
///
/// `field_sleep_scene` above measures the two easy cases: a world-wide
/// disturbance, and a fully settled world (which hits `field::step`'s global
/// early-out and costs nothing). Neither is the case a large world lives in.
///
/// The case that decides whether the world can grow is **one small thing
/// happening somewhere**: a gnome running in a corner, a fire, a trickle of
/// sand. `field::step`'s gate is world-global — `active_chunk_count() == 0 &&
/// fields_settled()` — so a single active chunk anywhere makes all seven
/// passes run over *every* resident chunk. That is O(world) per frame for
/// O(1) of activity, and it is what `PLAN.md` issue #4 is really about.
///
/// Reported per size so the scaling is visible rather than asserted.
fn field_scaling_scene() {
    if !begin("field: cost vs world size (issue #4 baseline)") {
        return;
    }
    println!("{:>12}  {:>10}  {:>10}  {:>10}", "size", "settled", "local", "global");
    for &(w, h) in &[(512, 320), (1024, 640), (2048, 1280)] {
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
        world.end_step();

        // Settled: the global early-out should make this free at any size.
        for _ in 0..8 {
            world.step_fields();
        }
        let mut settled = std::time::Duration::ZERO;
        for _ in 0..60 {
            let t = std::time::Instant::now();
            world.step_fields();
            settled = settled.max(t.elapsed());
        }

        // Localised: one small impulse re-applied in a corner every frame, so
        // exactly one tile has any reason to be awake. Today this costs the
        // same as the global case; after per-tile sleeping it should cost
        // roughly the same as `settled` regardless of world size.
        let mut local = std::time::Duration::ZERO;
        for _ in 0..60 {
            world.add_pressure_impulse(24, 24, 4, 40.0);
            let t = std::time::Instant::now();
            world.step_fields();
            local = local.max(t.elapsed());
        }

        // Global: a disturbance big enough to wake the world, for reference.
        let mut global = std::time::Duration::ZERO;
        world.add_pressure_impulse(w / 2, h / 2, (w / 4).max(8), 400.0);
        for _ in 0..60 {
            let t = std::time::Instant::now();
            world.step_fields();
            global = global.max(t.elapsed());
        }

        println!(
            "{:>12}  {:>8.3}ms  {:>8.3}ms  {:>8.3}ms",
            format!("{w}x{h}"),
            settled.as_secs_f64() * 1000.0,
            local.as_secs_f64() * 1000.0,
            global.as_secs_f64() * 1000.0,
        );
    }
}

/// Prints pressure magnitude as a density ramp, one character per field cell
/// (`FIELD_SCALE` world cells) rather than per CA cell — the field grid is
/// coarser, and printing at CA resolution would just repeat each character
/// `FIELD_SCALE` times for no extra information. `#` marks a field cell
/// blocked by CA-solid material, so walls are visible against the pressure.
fn field_scene(title: &str, w: i32, h: i32, frames: usize, setup: impl FnOnce(&mut World)) {
    if !begin(title) {
        return;
    }
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
    if !begin(title) {
        return;
    }
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
    if !begin(title) {
        return;
    }
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
    if !begin(title) {
        return;
    }
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
    if !begin("M17: structural distances for a freshly generated world") {
        return;
    }
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

    // ...and at the size the app actually ships, which is the number that
    // decides whether `R` and `F6` feel instant or feel like a stall. The
    // 512x320 figures above stay because they are the historical series;
    // this is the live one. What this watches for is a *worse* than linear
    // term, which is what `compute_world_distances` would contribute if its
    // per-cell `World::get` started missing chunk lookups.
    //
    // **Ratioed against solid cells, not against area, and the difference
    // reversed the verdict.** Against area it read "199x the build for 128x
    // the area -- WORSE THAN LINEARLY" at 8192x2560, which is true and
    // means nothing: `sky_rows` does not scale with world height, so a
    // taller world is a proportionally *more solid* one -- 59% filled at
    // 512x320 against 94% at the shipped size. Solid cells went up 204x for
    // that 128x of area, so the same generator doing the same per-cell work
    // is bound to look super-linear in area and is in fact slightly
    // sub-linear in the thing it actually writes. `PASS_TIMING=1` confirms
    // it: `stone_massif` is 3946 of 5188 ms and 201 ns per cell placed,
    // against ~300 ns/cell at 512x320.
    //
    // This is `CLAUDE.md`'s "ask what a metric counts when nothing is
    // wrong", and it cost a wrong hypothesis and a reverted change before
    // anyone asked it (`Reports/world-scale-phase-2.md` §7).
    let (ww, wh) = (pixel_physics::app::WORLD_WIDTH as i32, pixel_physics::app::WORLD_HEIGHT as i32);
    let mut big = World::new(Rect::new(0, 0, ww - 1, wh - 1));
    let start = std::time::Instant::now();
    pixel_physics::worldgen::generate(&mut big, spec());
    let big_with_pass = start.elapsed();

    let mut big_bare = World::new(Rect::new(0, 0, ww - 1, wh - 1));
    let start = std::time::Instant::now();
    pixel_physics::worldgen::generate_only(&mut big_bare, spec());
    let big_without_pass = start.elapsed();

    let big_solid = (0..ww)
        .map(|x| (0..wh).filter(|&y| big.get(x, y).material != material::EMPTY).count())
        .sum::<usize>();
    let area_ratio = (ww as f64 * wh as f64) / (512.0 * 320.0);
    let solid_ratio = big_solid as f64 / (gen_solid as f64).max(1.0);
    let cost_ratio = big_with_pass.as_secs_f64() / gen_with_pass.as_secs_f64().max(f64::EPSILON);
    println!(
        "{ww}x{wh} generated terrain -- THE SHIPPED WORLD SIZE ({name}, seed 1), {big_solid} solid cells: \
         {:.2} ms to build and relax, {:.2} ms to place alone -- structural pass {:.2} ms. That is \
         {cost_ratio:.2}x the 512x320 build for {area_ratio:.1}x the area but {solid_ratio:.1}x the solid \
         cells, so the build scales {} in the cells it writes. Paid on start, R, F6, F7 and F8.",
        big_with_pass.as_secs_f64() * 1000.0,
        big_without_pass.as_secs_f64() * 1000.0,
        (big_with_pass.saturating_sub(big_without_pass)).as_secs_f64() * 1000.0,
        if cost_ratio > solid_ratio * 1.35 { "WORSE THAN LINEARLY" } else { "linearly or better" },
    );
}

/// The instrument that decides the rivers track (`Reports/world-review-2026-08.md`
/// §4): what a spring held at steady state actually costs, measured, before
/// any river mechanics are built. A budgeted emitter pours water onto real
/// generated canyon terrain; it falls, pools, and a capped drain at the
/// basin floor removes it — the plausible-flux shape, harness-local (nothing
/// here is a shipped mechanic). Paired same-session against the identical
/// world with the spring off, per the compare-two-runs convention.
///
/// What it prints, and why each number:
/// - **worst/mean frame over the steady window** (CA + field combined, the
///   app's real per-frame work): the paired delta is the standing bill. The
///   pre-registered kill criterion reads it: a delta in the class of the
///   reverted global wind (~3.5 ms standing) kills the approach.
/// - **awake chunks and unsettled field tiles**: the second kill criterion —
///   if the awake set does not stabilize at a bounded handful, continuous
///   inflow is re-levelling the pool forever (open bug §4's O(width²)) and
///   continuous emission is dead regardless of the frame numbers.
/// - **emitted / drained / standing census**: volume, not cell counts (the
///   recorded metric trap), and the three must reconcile — a leak here means
///   the harness is measuring something other than what it claims.
///
/// The spring throttles itself while its outlet cell is occupied (the
/// drowned-spring guard), so a blocked fall self-limits instead of flooding
/// the measurement.
fn river_cost_scene(w: i32, h: i32) {
    if !begin(&format!("river-cost at {w}x{h}: a spring, a fall and a pool held at steady state (world review §4)")) {
        return;
    }
    let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        println!("worldgen presets unavailable ({e}); skipping river-cost scene");
        return;
    }
    let Some(params) = presets.get("canyon") else {
        println!("no canyon preset; skipping river-cost scene");
        return;
    };
    let spec = || pixel_physics::worldgen::Spec::Generated { params, seed: 1 };
    const SETTLE_FRAMES: usize = 600;
    const MEASURE_FRAMES: usize = 1400;

    // The geography is a pure function of the seed, so one probe world
    // serves both arms. Ground is Solid|Powder — the same definition the
    // renderer's skyline uses; water and plants are not ground.
    let mut probe = World::new(Rect::new(0, 0, w - 1, h - 1));
    pixel_physics::worldgen::generate(&mut probe, spec());
    let surf: Vec<i32> = (0..w)
        .map(|x| {
            (0..h)
                .find(|&y| {
                    matches!(
                        probe.materials.kind(probe.get(x, y).material),
                        material::MaterialKind::Solid | material::MaterialKind::Powder
                    )
                })
                .unwrap_or(h)
        })
        .collect();
    // The spring stands on the high side of the steepest drop an 8-column
    // window finds — a brow over a fall, which is the geometry the review
    // says this terrain already supplies. The drain sits at the lowest
    // basin floor in the world. Both printed, so the same spot can be
    // rendered with `filmstrip scene=worldgen preset=canyon seed=1` and a
    // crop.
    let mut spring_edge = 8usize;
    let mut best_drop = 0i32;
    for x in 8..(w as usize - 16) {
        let drop = surf[x + 8] - surf[x];
        if drop.abs() > best_drop.abs() {
            best_drop = drop;
            spring_edge = x;
        }
    }
    let sx = if best_drop > 0 { spring_edge as i32 } else { spring_edge as i32 + 8 };
    let sy = surf[sx as usize] - 1;
    let dx = (0..w).max_by_key(|&x| surf[x as usize]).expect("world has columns");
    println!(
        "spring at ({sx}, {sy}) over a {}-cell drop; drain in column {dx} (floor y {})",
        best_drop.abs(),
        surf[dx as usize],
    );

    let run = |spring: bool| {
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
        pixel_physics::worldgen::generate(&mut world, spec());
        let census_start = update::liquid_volume(&world, material::WATER);
        let mut emitted = 0u64;
        let mut drained = 0u64;
        let mut worst = std::time::Duration::ZERO;
        let mut total = std::time::Duration::ZERO;
        let mut awake_max = 0usize;
        let mut awake_sum = 0usize;
        for frame in 0..SETTLE_FRAMES + MEASURE_FRAMES {
            let started = std::time::Instant::now();
            parallel::step(&mut world);
            world.step_fields();
            if spring {
                // Emit one full cell per frame while the outlet is clear —
                // roughly the whole-world creation rate of a maximum storm
                // concentrated at one lip.
                if world.get(sx, sy).material == material::EMPTY {
                    world.set(sx, sy, Cell::new(material::WATER, 0));
                    emitted += material::LIQUID_FULL as u64;
                }
                // Drain: the topmost water standing in the drain column,
                // capped at one cell per frame — the spring's exact inverse.
                let floor = surf[dx as usize];
                for y in (floor - 60).max(0)..=floor.min(h - 1) {
                    let c = world.get(dx, y);
                    if c.material == material::WATER {
                        drained += update::liquid_fill(c) as u64;
                        world.set(dx, y, Cell::EMPTY);
                        break;
                    }
                }
            }
            let dt = started.elapsed();
            if frame >= SETTLE_FRAMES {
                worst = worst.max(dt);
                total += dt;
                let awake = world.active_chunk_count();
                awake_max = awake_max.max(awake);
                awake_sum += awake;
            }
        }
        let census_end = update::liquid_volume(&world, material::WATER);
        (worst, total, awake_max, awake_sum, world.unsettled_field_tiles(), emitted, drained, census_start, census_end, world.chunk_count())
    };

    let (c_worst, c_total, c_awake_max, c_awake_sum, c_tiles, _, _, _, _, chunks) = run(false);
    println!(
        "spring OFF: worst {:.3} ms, mean {:.3} ms over {MEASURE_FRAMES} frames; awake chunks max {c_awake_max} mean {:.1} of {chunks}; unsettled field tiles at end {c_tiles}",
        c_worst.as_secs_f64() * 1000.0,
        c_total.as_secs_f64() * 1000.0 / MEASURE_FRAMES as f64,
        c_awake_sum as f64 / MEASURE_FRAMES as f64,
    );
    let (s_worst, s_total, s_awake_max, s_awake_sum, s_tiles, emitted, drained, census_start, census_end, _) = run(true);
    println!(
        "spring ON:  worst {:.3} ms, mean {:.3} ms over {MEASURE_FRAMES} frames; awake chunks max {s_awake_max} mean {:.1} of {chunks}; unsettled field tiles at end {s_tiles}",
        s_worst.as_secs_f64() * 1000.0,
        s_total.as_secs_f64() * 1000.0 / MEASURE_FRAMES as f64,
        s_awake_sum as f64 / MEASURE_FRAMES as f64,
    );
    let delta_ms = (s_total.as_secs_f64() - c_total.as_secs_f64()) * 1000.0 / MEASURE_FRAMES as f64;
    println!(
        "standing bill: mean delta {delta_ms:.3} ms/frame (kill bar: wind-revert class ~3.5 ms; pre-registered 2.0 ms)"
    );
    // Volume, not cell counts, and the ledger must close: what went in is
    // what came out plus what is standing.
    let standing = census_end as i64 - census_start as i64;
    let leak = emitted as i64 - drained as i64 - standing;
    println!(
        "water ledger: emitted {emitted}, drained {drained}, standing delta {standing}, unaccounted {leak} \
         (evaporation and infiltration legitimately absorb some; a large residual means the harness lies)"
    );
}

fn structural_scene(title: &str, w: i32, h: i32) {
    if !begin(title) {
        return;
    }
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
    if !begin(title) {
        return;
    }
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
    if !begin(title) {
        return;
    }
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
    if !begin(title) {
        return;
    }
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    setup(&mut world);

    // **Timed, because a plant scene is where per-organism work shows up
    // and nothing else here measures it.** Every other helper reports a
    // worst frame; this one did not, so a change to the organism passes
    // (upkeep walks, root branching, abscission) had no standing number to
    // be paired against and CI carried none. The settled half is the one
    // that matters for the same reason the animated-grain measurement did:
    // a mature stand is exactly where the dirty-rect skip earns its keep.
    let mut worst_growing = std::time::Duration::ZERO;
    let mut worst_settled = std::time::Duration::ZERO;
    for frame in 0..frames {
        let started = std::time::Instant::now();
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        let elapsed = started.elapsed();
        // The first half is a plant establishing and extending; the second
        // is what the player actually looks at for most of a session.
        if frame * 2 < frames {
            worst_growing = worst_growing.max(elapsed);
        } else {
            worst_settled = worst_settled.max(elapsed);
        }
    }

    let wood = world.materials.id_of("wood");
    let moss = world.materials.id_of("moss");
    let water_left = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| world.get(x, y).material == material::WATER).count();
    println!(
        "after {frames} frames: {} active chunks, {} active sites, {water_left} water cells remaining; worst frame growing {:.4} ms, settled {:.4} ms",
        world.active_chunk_count(),
        world.active_site_count(),
        worst_growing.as_secs_f64() * 1000.0,
        worst_settled.as_secs_f64() * 1000.0,
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
    if !begin("ants: the foraging loop (nest, food pile, 60 ants)") {
        return;
    }
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
    // The **ground** the generator left, per column — organism cells
    // explicitly skipped, and that exclusion is load-bearing rather than
    // tidy.
    //
    // This used to be "the topmost Solid or Powder", which is the ground
    // right up until something is standing on it. A `seed` is a `Powder` and
    // a grown blade is a `Solid`, so as soon as worldgen sowed a ground
    // layer this started returning the top of a plant: the nest patch below
    // got stamped a row above the soil wherever a tussock had landed, and
    // the ants were planted into the vegetation rather than onto the
    // hillside. Raising `grass_density` from 0.35 to 0.50 was enough to put
    // grass inside x=16..90 and take the scene from green to **1,901
    // pickups and zero deliveries** — the colony's loop never closed.
    //
    // The failure read exactly like an ant bug and was a scene bug
    // (`CLAUDE.md`: a scene that contradicts the code will look like a bug in
    // the code). Asking for ground rather than for "anything solid" makes it
    // immune to whatever the flora does next, which is the point — this
    // scene wants a hillside, and a plant is not one.
    let surface = |world: &World, x: i32| -> i32 {
        (0..h)
            .find(|&y| {
                let c = world.get(x, y);
                c.organism_id() == 0
                    && matches!(world.materials.kind(c.material), material::MaterialKind::Solid | material::MaterialKind::Powder)
            })
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
    //
    // **Drives the CA too, and that is a repair rather than a tidy-up.**
    // This loop used to call only `step_active_sites` + `step_fields`, which
    // is every phase a growing plant needs *except* the one that runs the
    // weather: `weather::step` is the first thing both CA drivers do
    // (`update.rs`, `parallel.rs`). So the 2,400 frames this stand grows in
    // were rainless by construction -- and so were infiltration and
    // capillary flow, which are also CA-dispatched.
    //
    // That did not matter while a plant ran on one currency. It matters now
    // that a root drinks soil moisture, because this scene builds its world
    // from `worldgen`, which leaves soil away from water at `aux == 0` --
    // and a warmup that cannot rain is a warmup that can never wet it. Six
    // trees grew **zero** leaves here, which is this scene's own food
    // supply, and the ants starved (`Reports/open-bugs-handoff.md`, the
    // blocking entry).
    //
    // Matches `run` below, minus the timing instrumentation and pheromones
    // -- there are no creatures yet at this point to leave a trail.
    for _ in 0..2400 {
        parallel::step(&mut world);
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
            "{label}: {} live organisms ({} creatures) | moves {} blocked {} falls {} | eats {} pickups {} digs {} drops {} deliveries {} nest-visits {} deaths {}",
            world.live_organism_count(),
            world.live_creature_count(),
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
        // **The damage counters, printed even while they are zero, for the
        // reason the reproduction ones below are.** `injuries` counts every
        // survived body-cell loss and has been here in spirit since
        // `reconcile_chain` existed; `severed` is the one that says the
        // 2026-09 severing rule *fired* rather than merely shortening a
        // chain, and `refused` says armour turned a bite away. Two very
        // different mechanisms produce the same `eats` and the same
        // `deaths`, and only these separate them -- `CLAUDE.md`, *"did it
        // fire at all" needs a counter, not a picture*. A colony with
        // nothing armoured in it reads `refused 0`, which is the negative
        // control the first non-zero is read against.
        println!(
            "  damage: injuries {} severings {} severed cells {} | bites refused by armour {}",
            st.injuries, st.severings, st.severed_body_cells, st.bites_refused
        );
        // **The reproduction counters, printed even while they are zero.**
        // That is the point of shipping them ahead of the mechanism
        // (`Reports/creature-review-2026-08.md` §T4): a colony that is not
        // breeding and a colony whose birth path never executes look
        // identical in every other counter on this line, and a zero here
        // is the negative control the first non-zero reading is read
        // against. `spawned` sits beside them because `births` is only
        // meaningful against the founders it is *not* counting.
        // **The richest ant in the colony, and the bar it has to clear.**
        // `births 0` has two completely different causes and this is the
        // only line that separates them: a threshold nobody has *reached*
        // (the economy is poor, or the bank is capped below the bar) and a
        // birth path that does not fire when they do. Without it a zero
        // reads as "reproduction is broken" either way, which is the
        // did-it-fire-at-all failure in its usual costume -- the counter
        // is honest and answers a different question than the one asked.
        let richest = (0..world.bounds().map_or(0, |b| b.width()))
            .flat_map(|x| (0..world.bounds().map_or(0, |b| b.height())).map(move |y| (x, y)))
            .filter_map(|(x, y)| world.organism(world.get(x, y).organism_id()))
            .filter(|s| world.species.get(s.species).creature.is_some())
            .map(|s| s.energy)
            .fold(0.0f32, f32::max);
        let bar = world
            .species
            .id_of("ant")
            .and_then(|id| world.species.get(id).creature.as_ref().map(pixel_physics::sim::creature::birth_cost))
            .unwrap_or(0.0);
        println!(
            "  population: spawned {} births {} births-denied-no-space {} refused-no-slot {} | richest bank {richest:.0} against a birth cost of {bar:.0}",
            st.spawned,
            st.births,
            st.births_denied_no_space,
            world.organisms_refused()
        );
        // **Range, printed next to the verb counters, because none of them
        // can say it.** `nest-visits` above counts loitering, not trips --
        // see `CreatureStats::nest_visits`. A colony that never leaves the
        // nest mouth scores high on it and zero here.
        println!(
            "{label}: forage trips {} (bar {}) mean depth {:.1} deepest {} | reach {:?}",
            st.forage_trips,
            pixel_physics::sim::creature::FORAGE_TRIP_MIN,
            if st.forage_trips > 0 { st.forage_depth_sum as f64 / st.forage_trips as f64 } else { 0.0 },
            st.forage_depth_max,
            st.forage_reach
        );
        // **The standing food stock, in energy rather than in cells.** A
        // count of edible cells rises as a stand of trees grows whatever is
        // happening to the animals, which is how §13m's tree-killing bug
        // hid: the metric that would have shown it was a cell count, and it
        // kept going up. Summed worth also makes the corpse stamp visible as
        // a "did it fire" number -- corpse cells with zero worth mean
        // `creature_dies` did not stamp them, which a picture of a corpse
        // pile cannot show.
        // **Attributed by material, and derived rather than named.** Pricing
        // everything fixed the blindness a hardcoded `leaf` column had (S4
        // shipped inert behind exactly that), but it traded attribution away
        // with it: one summed number cannot tell "the canopy feeds them"
        // from "the floor feeds them", and those are opposite answers to the
        // question of how scarce the forest floor should be. The tally is
        // keyed on whatever materials actually carry worth in this run, so
        // it stays blind to nothing -- a food invented tomorrow shows up in
        // it without this line being edited.
        let mut per_material = vec![0.0f64; world.materials.len()];
        let mut food_stock = 0.0f64;
        let mut corpse_stock = 0.0f64;
        for x in 0..w {
            for y in 0..h {
                let cell = world.get(x, y);
                let v = pixel_physics::sim::creature::food_value(world, cell) as f64;
                if v <= 0.0 {
                    continue;
                }
                food_stock += v;
                per_material[cell.material.0 as usize] += v;
                if world.materials.get(cell.material).worth_in_aux {
                    corpse_stock += v;
                }
            }
        }
        let mut breakdown: Vec<(f64, &str)> = per_material
            .iter()
            .enumerate()
            .filter(|(_, &v)| v > 0.0)
            .map(|(id, &v)| (v, world.materials.get(MaterialId(id as u16)).name.as_str()))
            .collect();
        breakdown.sort_by(|a, b| b.0.total_cmp(&a.0));
        let attributed = breakdown
            .iter()
            .map(|(v, name)| format!("{name} {v:.0} ({:.0}%)", 100.0 * v / food_stock.max(1.0)))
            .collect::<Vec<_>>()
            .join(", ");
        println!("  food stock {food_stock:.0} energy, of which corpse {corpse_stock:.0} | {attributed}");
        // **Every material on the menu, not just `leaf`.** This counted leaf
        // alone, which sat directly under the `food stock` line above --
        // which prices everything -- so the two disagreed by construction the
        // moment litter became edible. A named-material column is blind by
        // construction to any food invented later; that is exactly how S4
        // shipped inert and three rounds of measurement missed it.
        let food_left = (0..w)
            .flat_map(|x| (0..h).map(move |y| (x, y)))
            .filter(|&(x, y)| pixel_physics::sim::creature::food_value(world, world.get(x, y)) > 0.0)
            .count();
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
                        if st.crop.is_some() && st.chain.first() == Some(&(x, y)) {
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
    // **Kept, but demoted to what it actually proves, and given the guard
    // it was mistaken for.** `nest_visits` counts loitering, not arrivals:
    // it increments on any move made while nest-adjacent, and its
    // `since_nest > 0` guard is false exactly once per lifetime because
    // `since_nest` is bumped every tick (`CreatureStats::nest_visits`). So
    // `> 0` here is not the sessility guard it reads as -- **a colony that
    // never left the nest mouth passes it trivially**.
    assert!(st.nest_visits > 0, "no ant was ever next to the nest at all");
    // The real one. **Bars set from measurement on this tree, with
    // headroom** -- not from an aspiration, and not inherited.
    //
    // **Re-baselined 2026-08-29 on `main` at `ba6fc98`, and the old figures
    // are kept beside the new ones because the gap is the finding.** This
    // scene measured **98 trips, deepest 18, mean depth 10.3** after the
    // litter merge (2026-08-23), and the bar below was set at a seventh of
    // that. It now measures **23 trips, deepest 28, mean depth 11.0** --
    // four times fewer excursions. Neither number was touched by creature
    // code: the sky/soil worldgen work (`39e6f36`) reshaped the terrain the
    // colony forages over, and the plant organs merge (`f96c08d`, the same
    // day) changed what grows on it. Both were measured, four runs, and the
    // trip count is the stable half: 24 on `ba6fc98` and 23 after the organs
    // merge, while the depth moved 37 -> 28 across the same step. **Depth is
    // the volatile column; do not set a tight bar on it.**
    //
    // **The bar had therefore gone fragile without anyone editing it.** At
    // `>= 14` against a measurement of 24 it sat at 58% of the value, which
    // is the "bar near the measurement flakes" case this comment warns
    // about, while still telling the reader the measurement was 98. It is
    // now **6**: the largest legitimate drift on record is the 4.1x this
    // paragraph documents, and 23/4.1 is 5.6, so another drift as big as the
    // one that just happened still passes while the failure the guard is
    // named for -- a sessile colony, which scores exactly 0 -- still fails.
    // Lowering a bar weakens it, and that trade is stated rather than
    // hidden: a genuine 2x foraging regression would now pass here.
    //
    // **What this bar cannot do, recorded rather than fixed.** The scene
    // generates terrain at a hardcoded `seed: 1`, so this is a single-seed
    // bar over procedural content -- `CLAUDE.md`'s "a guard over a
    // procedural system has to sweep the procedure, and it should gate an
    // order statistic". Run-to-run it is exact (two full `ascii` runs are
    // bit-identical), so the spread it is blind to is seed spread, not
    // noise. `forage_probe seeds=N` is the instrument that has that axis.
    //
    // What this catches that nothing else did: the colony going sessile.
    // Every counter above stays healthy for a colony milling around the
    // nest -- `moves`, `pickups`, `drops` and `nest_visits` all climb --
    // and this is the only one that goes to zero.
    assert!(
        st.forage_trips >= 6,
        "the colony has gone sessile: {} round trips of {}+ cells (measured 23 here on 2026-08-29 at `f96c08d`, 24 at `ba6fc98`; was 98 on 2026-08-23), deepest excursion {} cells, reach profile {:?}",
        st.forage_trips,
        pixel_physics::sim::creature::FORAGE_TRIP_MIN,
        st.forage_depth_max,
        st.forage_reach
    );
    assert!(
        st.forage_depth_max >= 8,
        "no ant got further than {} cells from home (measured 28 here on 2026-08-29 at `f96c08d`, 37 at `ba6fc98`; was 18 on 2026-08-23 -- the bar stays at 8, which is 3.5x headroom and this column is the volatile one)",
        st.forage_depth_max
    );
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
    let led = world.energy_ledger;
    println!(
        "energy census: live {live:.2} vs ledger {expected:.2} (delta {:.4}); granted {:.0} plant {:.0} corpse {:.0} metabolized {:.0} moved {:.0} synapses {:.2} stored-in-meat {:.0} dissipated {:.0}",
        live - expected,
        led.granted,
        led.harvested_plant,
        led.harvested_corpse,
        led.metabolized,
        led.moved,
        led.synapse_tax,
        led.stored_in_meat,
        led.dissipated
    );
    // **The other stock, and the bound it has to sit under.** The live
    // identity above only says the charges landed; it cannot say whether
    // value was created, because `granted` and `harvested_plant` are free
    // by construction. `harvested_corpse` is not free -- every joule of it
    // came out of meat that was booked when a body was built or when one
    // died -- so this line is the one that would show §13l's pump running
    // again. `<=` rather than `==` on purpose; see `standing_meat`.
    let meat = pixel_physics::sim::creature::standing_meat(&world, Rect::new(0, 0, w - 1, h - 1)) + pixel_physics::sim::creature::carried_meat(&world);
    println!(
        "meat census: standing {meat:.0} vs ceiling {:.0} (headroom {:.0}); stamped {:.0}",
        led.max_standing_meat(),
        led.max_standing_meat() - meat,
        led.stamped
    );
}

/// Shared runner for the colony scenes: the full frame order the live app
/// uses, so nothing here is testing a phase the player never sees.
fn run_colony(world: &mut World, frames: usize) {
    run_colony_with(world, frames, |_, _| {});
}

/// `run_colony` with a per-frame hook.
///
/// A scene that needs a condition *maintained* — a spring that goes on
/// running, a sample taken through the run rather than at the end — cannot
/// express it against the plain loop, which hands control back only once
/// everything is over. Bug H was both of those at once.
fn run_colony_with(world: &mut World, frames: usize, mut each: impl FnMut(&mut World, usize)) {
    for frame in 0..frames {
        parallel::step(world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
        each(world, frame);
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
    if !begin("ants: a double bridge made of terrain (short tunnel vs the long way over)") {
        return;
    }
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
    println!("  forage trips {} deepest {} | reach {:?}", st.forage_trips, st.forage_depth_max, st.forage_reach);
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
    if !begin("ants: excavating a chamber out of soil (and stopped by stone)") {
        return;
    }
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
    // **Bank, not `soil`.** An ant now works the wall of the gallery it cuts
    // into `packedsoil` (`creature::line_burrow`), so a census of the `soil`
    // id alone would book every *lined* cell as excavated and report a dig
    // rate several times the real one -- a number that is arithmetically
    // correct and answers a different question, which `CLAUDE.md` names as
    // this repo's worst-recurring failure. What the scene claims is that the
    // ants removed material from the bank, so what it counts is the bank:
    // both forms of the ground, whether worked or loose.
    let packed = world.materials.id_of("packedsoil").expect("packedsoil");
    let bank = |world: &World| -> usize {
        (0..w)
            .flat_map(|x| (0..h).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                let m = world.get(x, y).material;
                m == soil || m == packed
            })
            .count()
    };
    let soil_before = bank(&world);
    for i in 0..55 {
        world.plant_ant(20 + i % 10 * 2, floor - 1 - (i / 10));
    }

    run_colony(&mut world, 8000);

    let soil_after = bank(&world);
    let stone_floor: usize = (0..w).map(|x| usize::from(world.get(x, h - 1).material == material::STONE)).sum();
    // **Standing void inside the bank footprint** -- the quantity a player
    // would call a nest, and the one this scene never had. `digs` says the
    // verb fired and `soil -> bank` says material left; neither can say
    // whether anything is still *open*, and before the wall was lined the
    // answer was almost nothing: a dug hole in loose soil closes in a
    // handful of frames (`examples/burrow_probe.rs`).
    //
    // **Two numbers, because the obvious one is the wrong one.** `void`
    // counts every empty cell in the footprint, and a colony that quarries
    // the bank away from its open face produces a great deal of that without
    // ever roofing anything: measured on this scene, the *ablated* build
    // (`PIXEL_PHYSICS_BURROW_LINING=off`) leaves **more** raw void than the
    // lined one, which reads as the feature making things worse and is an
    // artifact of counting an open pit as a tunnel. `roofed` is the claim
    // actually being made -- empty cells with ground standing over them --
    // and it is the column to read.
    let roofed_over = |world: &World, x: i32, y: i32| -> bool {
        (0..y).rev().any(|uy| {
            matches!(
                world.materials.kind(world.get(x, uy).material),
                material::MaterialKind::Powder | material::MaterialKind::Solid
            )
        })
    };
    let mut void = 0usize;
    let mut roofed = 0usize;
    for x in 40..160 {
        for y in (floor - 30)..floor {
            if world.get(x, y).material == material::EMPTY {
                void += 1;
                if roofed_over(&world, x, y) {
                    roofed += 1;
                }
            }
        }
    }
    let lined: usize = (0..w)
        .flat_map(|x| (0..h).map(move |y| (x, y)))
        .filter(|&(x, y)| world.get(x, y).material == packed)
        .count();
    let st = world.creature_stats;
    println!("  digs {} | moves {} blocked {} deaths {}", st.digs, st.moves, st.moves_blocked, st.deaths);
    println!("  bank {soil_before} -> {soil_after} standing, stone floor intact {stone_floor}/{w}");
    println!("  standing void inside the bank {void}, of it roofed {roofed} | wall cells packed {} (standing {lined})", st.packed);
    assert!(st.digs > 0, "no ant ever dug -- the verb never fired, whatever the picture shows");
    // The far-side effect counter on the same call `digs` counts. A renamed
    // `packedsoil` or a dropped `packs_into` leaves every dig firing and
    // every wall unlined, and only this reads 0 when that happens.
    // Skipped when the lining is deliberately ablated, which is the control
    // this scene's own numbers are read against -- otherwise the control run
    // aborts here and never reaches the scenes after it.
    if std::env::var("PIXEL_PHYSICS_BURROW_LINING").as_deref() != Ok("off") {
        assert!(st.packed > 0, "ants dug and lined nothing -- Material::packs_into never resolved");
        assert!(roofed > 0, "ants dug {} cells and left no roofed void at all -- no tunnel stood", st.digs);
    }
    assert!(void > 0, "the ants dug {} cells and left no standing void at all", st.digs);
    // **This used to assert that the bank got *smaller*, and that claim was
    // a statement about the old rule rather than about the ants.** Digging
    // destroyed its spoil, so excavation and material loss were the same
    // event; an ant now carries the cell it dug and puts it down again, so
    // the bank is conserved and the old assertion is false by design.
    //
    // Replaced rather than deleted, and by a *stronger* claim than the one it
    // made: the ground is conserved exactly, counting what is standing plus
    // what is still in a mandible, less the pellets that died with their
    // carrier with nowhere to land. The excavation half it was standing in
    // for is `roofed > 0` above -- which is the column that says a nest
    // exists, and the one `dead-ends.md` records as the repair for censusing
    // a dug volume at all.
    let laden = world
        .live_organism_ids()
        .into_iter()
        .filter(|&id| world.organism(id).is_some_and(|s| s.spoil.is_some()))
        .count();
    // Skipped under the ablation for the reason the lining's assertions are:
    // the control arm is the old behaviour, which does not conserve, and a
    // control run that aborts here never reaches the scenes after it.
    if std::env::var("PIXEL_PHYSICS_DIG_SPOIL").as_deref() == Ok("destroy") {
        return;
    }
    assert_eq!(
        soil_after + laden + st.spoil_lost as usize,
        soil_before,
        "the bank is not conserved: {soil_before} -> {soil_after} standing, {laden} in mandibles, {} lost with their carriers",
        st.spoil_lost
    );
    assert_eq!(stone_floor as i32, w, "ants must not have dug through stone -- dig_force 1.0 is below stone's penetration_resistance");
}

/// Carriers over a moisture gradient. **Deposition bias, not a build
/// script** (`stigmergy-research.md` §4, the eLife 2024 result): drop
/// probability is multiplied by local `|grad moisture|`, so material
/// accumulates where the gradient is steep. Pillars and walls are
/// consequences of that, and writing a "build a wall" behaviour would be
/// the signal to go and re-read that section.
fn construction_scene() {
    if !begin("ants: deposition follows the moisture gradient, with no build rule anywhere") {
        return;
    }
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
    // Food spread thinly across the whole floor, so ants pick up wherever
    // they are and then carry across both halves.
    for x in (20..220).step_by(4) {
        world.set(x, floor - 1, Cell::new(corpse, 0));
    }
    // **The ablation, as a switch rather than as a `.ron` edit and a
    // rebuild** -- the shape `PIXEL_PHYSICS_BURROW_LINING` and
    // `PIXEL_PHYSICS_DIG_SPOIL` already use here, and the reason is the one
    // `CLAUDE.md` gives after the `relax_region` night: *the control is to
    // hold the semantic rule fixed, not to add another metric.* An A/B run by
    // editing a species file is two binaries and two worlds; this is one
    // binary and one edit to one genome slot.
    //
    // **The offset is not optional and it is not a second change.** Zeroing
    // `(MoistureGrad, Drop, w)` alone does not remove a bias, it removes the
    // whole drop probability: this scene has no nest, so `AtNest` is 0, and
    // `squash(-0.2 + 0.2)` is exactly 0 -- measured, `drops 715 -> 0`. Folding
    // the weight back in at the mean input the ants actually read (0.75, from
    // the run below) leaves the *rate* where it was and takes away only the
    // *dependence*, which is the arm this experiment needs.
    // **Before the ants, and that is load-bearing.** `place_creature` copies
    // the species genome into each animal at spawn, so a `set_genome` after
    // the colony exists changes nothing at all -- written the other way round
    // first, and the two arms came back **byte-identical**, which is
    // `CLAUDE.md`'s standing tell for a change that must have moved something
    // and did not.
    let ablate = std::env::var("PIXEL_PHYSICS_DROP_MOISTURE").ok().and_then(|v| {
        // `off` folds at the calibrated default; `off:0.9` overrides it. The
        // fraction is a **calibration of the control**, not a result, and it
        // is a knob rather than a literal because matching the rate has to be
        // re-doable without a rebuild whenever the drop weights are re-authored.
        let rest = v.strip_prefix("off")?;
        Some(rest.strip_prefix(':').and_then(|f| f.parse::<f32>().ok()).unwrap_or(0.98))
    });
    if let Some(fold) = ablate {
        use pixel_physics::sim::brain::{io_slot, BrainInput, BrainOutput};
        let id = world.species.id_of("ant").expect("ant");
        let mut g = world.species.get(id).genome.clone();
        let slot = io_slot(BrainInput::MoistureGrad, BrainOutput::Drop);
        let w_moist = g[slot];
        g[slot] = 0.0;
        g[io_slot(BrainInput::Bias, BrainOutput::Drop)] += w_moist * fold;
        world.species.set_genome(id, g);
        println!("  ABLATED: (MoistureGrad, Drop, {w_moist:.4}) folded into the bias at {fold} of its ceiling -- the dependence gone, the rate held");
    }

    for i in 0..55 {
        world.plant_ant(22 + i * 3, floor - 2);
    }

    // **A spring, not a puddle — bug H's cause, measured rather than
    // guessed.** The well was filled once at spawn and then left to the
    // world, and the world took it: instrumented per 1,000 frames it went
    // 34 cells -> 30 -> 39 -> 52 -> 66 -> 76 -> 98 -> 47 -> 1 -> 0. It does
    // not simply evaporate; it *rises* first, because `weather::step` runs
    // inside both CA drivers and rains into it, and then a dry spell takes
    // the lot. So the moisture field this scene asserts on was being read at
    // an arbitrary phase of a designed oscillator, and the phase frame
    // 10,000 lands on is a dry one: steep half and flat half both measured
    // 0.000000, peak included, and the guard `wet_grad > dry_grad` was
    // deciding on a residue below the sixth decimal. That is what made it
    // flip between CI runs 137 and 139 while printing identical numbers.
    //
    // CLAUDE.md's rule for a channel that oscillates by design is to divide
    // the oscillator out of the decision. There is no `noon_equivalent`
    // for weather, so the scene holds the *source* constant instead: the
    // well is topped up every frame, which makes the left half wet at every
    // phase, and rain can still wet the right half without ever making it a
    // spring. The gradient is then a property of the scene rather than of
    // the frame it was sampled on.
    let refill_spring = |world: &mut World| {
        for x in 61..68 {
            for y in (floor - 5)..floor {
                // The raw material, not `is_empty()`, which is
                // managed-aware and reads a promoted body's container cells
                // as occupied (`Cell::is_empty`'s own doc).
                if world.get(x, y).material == material::EMPTY {
                    world.set(x, y, Cell::new(material::WATER, 0));
                }
            }
        }
    };
    refill_spring(&mut world);

    let count_water = |world: &World| -> usize {
        (0..w).flat_map(|x| (0..h).map(move |y| (x, y))).filter(|&(x, y)| world.get(x, y).material == material::WATER).count()
    };
    let water_before = count_water(&world);

    // **Which half is which is measured, not assumed**, and it is measured
    // *through* the run rather than at one instant. Two instants fitted to
    // one trajectory is the failure CLAUDE.md names by name; a mean over 40
    // samples cannot be decided by whichever weather the last frame landed
    // in, and it is a continuous quantity rather than a knife-edge count.
    let mean_grad = |world: &World, x0: i32, x1: i32| -> f64 {
        let mut total = 0.0f64;
        let mut n = 0.0f64;
        for x in x0..x1 {
            for y in (floor - 10)..floor {
                let gx = world.field_at_bilinear((x + 4) as f32, y as f32).moisture - world.field_at_bilinear((x - 4) as f32, y as f32).moisture;
                let gy = world.field_at_bilinear(x as f32, (y + 4) as f32).moisture - world.field_at_bilinear(x as f32, (y - 4) as f32).moisture;
                total += ((gx * gx + gy * gy).sqrt()) as f64;
                n += 1.0;
            }
        }
        total / n
    };

    const GRAD_SAMPLE_EVERY: usize = 250;
    // **Set from measurement with headroom, not from an aspiration.** This
    // tree measures a margin of 1.8146 over 40 samples (steep 1.9206
    // against flat 0.1061, on `field::MAX_MOISTURE` = 4.0). The bar is a
    // little over a quarter of that: far enough below the measurement to
    // survive an honest change to the weather or the field solve, and far
    // enough above the flat half's own 0.1061 that a world where both
    // halves are merely damp cannot clear it.
    const MARGIN_BAR: f64 = 0.5;
    /// **The bar the scene's headline claim is actually asserted on.** See the
    /// five-arm table at the bottom of this function for where it comes from
    /// and what to run before changing it.
    const MATCHED_BAR: f64 = 1.03;
    let mut wet_sum = 0.0f64;
    let mut dry_sum = 0.0f64;
    let mut samples = 0u32;
    // **The setup check reads the scene as built, not as the ants left it.**
    //
    // This assertion says "the scene must actually contain the gradient it is
    // testing", and it was checked against a *run mean* -- forty samples taken
    // while fifty-five ants rearranged the very field being measured. That is
    // not a setup check, and it fired for the wrong reason on 2026-08-31:
    // ants that stopped churning the same few cells and started leaving real
    // deposits (4 cells standing before, 63 after) put water-holding powder
    // into the dry half, which raised the flat half's own gradient from 0.89
    // to 1.20 and closed the margin from below. The scene was working; the
    // measurement could not tell the scene's premise from the mechanism's
    // effect on it.
    //
    // **The first sample, not frame zero.** A reading taken before the run
    // is 0.0000 on both halves: the water is placed but the moisture field
    // has not been stepped, so there is no gradient to find yet and the check
    // would fail on a scene that is perfectly well formed. The first sample
    // lands at frame 249 -- field solved, colony barely started -- which is
    // the earliest point at which the question is answerable at all. The run
    // mean is still printed, because what the ants do to the field afterwards
    // is the interesting half and this is the file where it gets read.
    let mut initial_margin = f64::NAN;

    // **The matched control the standing guard does not have** -- §5e of
    // `Reports/creature-genome-flexibility-2026-09-02.md`, built after the
    // hypothesis in it was measured and confirmed.
    //
    // The `uphill` ratio below compares |grad m| at drop cells against a
    // 12-row band. **Drops land where an ant is standing, which is a surface,
    // and a surface is exactly where |grad m| is high**; the band average
    // includes deep air and buried soil, both near zero. So ~3x is what
    // "ants drop on surfaces" reads, with no contribution from the response
    // coefficient at all -- and measured on this build across three arms it
    // is exactly that: **2.96x shipped, 3.00x with `(MoistureGrad, Drop, w)`
    // deleted outright, 3.59x with the coefficient present as a constant at
    // its own ceiling.** The arm with the mechanism reads *lowest* of the
    // three, which is its predecessor's failure over again.
    //
    // The reference set has to be **matched on the confound**, and the only
    // set that is, is *where the ants themselves were*. An ant that is laden
    // and standing somewhere had the option of dropping there and did not; if
    // the coefficient does anything, the gradient where it *did* drop is
    // higher than the gradient where it stood. Same animals, same surfaces,
    // same phase of the weather -- the ratio divides all of it out and is 1.0
    // when the bias is absent, which is a bar that can go red.
    //
    // **Drop sites are attributed rather than censused.** Standing corpse
    // cells cannot be the numerator: this scene lays *corpse* as food and a
    // dead ant also becomes corpse, so the old census counts every ant that
    // starved on the floor as a deposit -- 44-48 deaths against 66 "standing
    // drops". Instead the band is diffed frame by frame, and new corpse cells
    // are credited to the drop verb only on frames where `deaths` did not
    // move. Frames where both happened are discarded rather than guessed at.
    let band = |x: i32, y: i32| (20..220).contains(&x) && ((floor - 12)..floor).contains(&y);
    let corpse_set = |world: &World| -> std::collections::HashSet<(i32, i32)> {
        (20..220)
            .flat_map(|x| ((floor - 12)..floor).map(move |y| (x, y)))
            .filter(|&(x, y)| world.get(x, y).material == corpse)
            .collect()
    };
    // **Both the raw field gradient and the value the brain is actually
    // handed**, because they are not the same number and only one of them can
    // be quoted next to the old column. `creature::moisture_gradient` divides
    // by `WORM_MOISTURE_SATURATION` and **clamps to 1.0**, and the shipped
    // reading at drop cells is 3.99 on a 4.0 saturation -- i.e. pinned at the
    // ceiling. A sense that is saturated everywhere its owner stands has no
    // range to express a preference with, which is a finding about the channel
    // rather than about the weight, and it is invisible in the raw column.
    let raw_grad = |world: &World, x: i32, y: i32| -> f64 {
        let gx = world.field_at_bilinear((x + 4) as f32, y as f32).moisture - world.field_at_bilinear((x - 4) as f32, y as f32).moisture;
        let gy = world.field_at_bilinear(x as f32, (y + 4) as f32).moisture - world.field_at_bilinear(x as f32, (y - 4) as f32).moisture;
        ((gx * gx + gy * gy).sqrt()) as f64
    };
    let mut prev_corpse = corpse_set(&world);
    let (mut prev_drops, mut prev_deaths) = (0u64, 0u64);
    let (mut event_grad, mut event_in, mut event_n) = (0.0f64, 0.0f64, 0u32);
    let (mut stood_grad, mut stood_in, mut stood_n) = (0.0f64, 0.0f64, 0u32);
    let mut ambiguous = 0u32;
    let mut unattributed = 0u64;
    let (mut paired_sum, mut paired_n, mut paired_up) = (0.0f64, 0u32, 0u32);
    run_colony_with(&mut world, 10000, |world, frame| {
        refill_spring(world);
        if frame % GRAD_SAMPLE_EVERY == GRAD_SAMPLE_EVERY - 1 {
            let (wet, dry) = (mean_grad(world, 20, w / 2), mean_grad(world, w / 2, 220));
            if samples == 0 {
                initial_margin = wet - dry;
            }
            wet_sum += wet;
            dry_sum += dry;
            samples += 1;
        }

        let st = world.creature_stats;
        let (d_drops, d_deaths) = (st.drops - prev_drops, st.deaths - prev_deaths);
        prev_drops = st.drops;
        prev_deaths = st.deaths;
        let now = corpse_set(world);
        if d_drops > 0 && d_deaths == 0 {
            // **A new corpse cell in the band is not automatically a
            // deposit, and the first version of this believed it was** --
            // it credited 2,230 events against a `drops` counter reading
            // 1,495, which is the tell that the numerator was counting
            // something else. `corpse` is `kind: Powder`, so a deposit
            // settles, and every cell it settles into reads as new.
            //
            // Two conditions, and both are needed. It must not be a cell
            // something fell *into* -- nothing held corpse above it last
            // frame, diagonals included, because `roll_along_slope` moves a
            // powder sideways as well as down. And it must be within one
            // cell of a live head, because that is where the drop verb puts
            // a pellet (`act`: the first empty 8-neighbour).
            //
            // `credited` against `d_drops` is the pairing `CLAUDE.md`
            // requires -- the near-side counter is the engine's own, the
            // far-side one is this attribution, and the gap between them is
            // printed rather than hidden.
            let heads: Vec<(i32, i32)> =
                world.live_organism_ids().iter().filter_map(|&id| world.organism(id).and_then(|st| st.chain.first().copied())).collect();
            let mut credited = 0u64;
            let mut frame_drop_grad = 0.0f64;
            // **Sorted, because a `HashSet`'s iteration order is randomised
            // per process and float addition is not associative.** Summed
            // straight off `difference` these totals differ in their last bits
            // between two runs of the *same binary*, which is a guard that can
            // flake for no reason at all. The elements are set members, so
            // there are no equal keys and no tie order to worry about.
            let mut new_cells: Vec<(i32, i32)> = now.difference(&prev_corpse).copied().collect();
            new_cells.sort_unstable();
            for (x, y) in new_cells {
                let fell = [(-1, -1), (0, -1), (1, -1)].iter().any(|&(dx, dy)| prev_corpse.contains(&(x + dx, y + dy)));
                let by_ant = heads.iter().any(|&(hx, hy)| (hx - x).abs() <= 1 && (hy - y).abs() <= 1);
                if fell || !by_ant {
                    continue;
                }
                let g = raw_grad(world, x, y);
                event_grad += g;
                frame_drop_grad += g;
                event_in += pixel_physics::sim::creature::moisture_gradient(world, x, y) as f64;
                event_n += 1;
                credited += 1;
            }
            unattributed += d_drops.abs_diff(credited);

            // **The denominator is taken on the same frame as the numerator,
            // and that is not tidiness.** `CLAUDE.md`'s rule for a channel
            // that oscillates by design: this world's moisture is fed by
            // weather and by a topped-up spring and drifts the whole length of
            // the run -- the two arms of this experiment ended at a flat-half
            // mean of 3.978 and 0.108. A numerator sampled on drop frames
            // against a denominator sampled every 25th frame is two different
            // phases of that drift as much as it is two different sets of
            // cells. Sampled together, the phase cancels exactly.
            //
            // **And the headline is a sign test, not a difference of means.**
            // Outcomes here have enormous spread, so "1.08x" over one run is a
            // sample from a wide distribution; "the drop side was higher on
            // 214 of 300 frames" is a claim about the same run that a wide
            // distribution cannot manufacture. Under no bias it sits at half.
            let mut laden = 0.0f64;
            let mut laden_n = 0u32;
            for id in world.live_organism_ids() {
                let Some(state) = world.organism(id) else { continue };
                if state.crop.is_none_or(|c| c.cells == 0) {
                    continue;
                }
                let Some(&(hx, hy)) = state.chain.first() else { continue };
                if !band(hx, hy) {
                    continue;
                }
                let g = raw_grad(world, hx, hy);
                laden += g;
                stood_grad += g;
                stood_in += pixel_physics::sim::creature::moisture_gradient(world, hx, hy) as f64;
                stood_n += 1;
                laden_n += 1;
            }
            if credited > 0 && laden_n > 0 {
                let (dg, lg) = (frame_drop_grad / credited as f64, laden / laden_n as f64);
                paired_sum += dg - lg;
                paired_n += 1;
                if dg > lg {
                    paired_up += 1;
                }
            }
        } else if d_drops > 0 {
            ambiguous += d_drops as u32;
        }
        prev_corpse = now;

    });

    let water_after = count_water(&world);
    let (wet_grad, dry_grad) = (wet_sum / samples as f64, dry_sum / samples as f64);
    // Drops land as material, so count what is standing where nothing was
    // placed: any corpse cell not on the original 4-cell lattice row.
    let dropped = |x0: i32, x1: i32| -> usize {
        (x0..x1)
            .flat_map(|x| ((floor - 12)..floor).map(move |y| (x, y)))
            .filter(|&(x, y)| world.get(x, y).material == corpse && !(y == floor - 1 && x % 4 == 0))
            .count()
    };
    let (wet_drops, dry_drops) = (dropped(20, w / 2), dropped(w / 2, 220));

    // **What the scene's headline claim actually needs measured.** The
    // left/right split above cannot see the deposition bias: with
    // `moisture_gradient` deleted from `creature.rs`'s drop probability
    // entirely, it read steep 18 / flat 0 against steep 6 / flat 0 with the
    // bias in -- it passed *harder* for the broken build, because removing
    // a multiplier below 1.0 simply raises the drop rate, and the flat half
    // reads zero either way since the ants never get there. A guard that
    // cannot fail for the replacement artifact is not a guard (CLAUDE.md).
    //
    // The claim is "deposition follows the moisture gradient", so measure
    // that directly and as a ratio: the mean |grad moisture| *at the cells
    // ants actually dropped on*, against the mean over the whole band they
    // could have dropped on.
    //
    // **That reasoning is right and this reference set is not, and it has now
    // been measured rather than suspected** (§5e of
    // `Reports/creature-genome-flexibility-2026-09-02.md`, which named the
    // control; this is its result). Drops land where an ant is standing,
    // which is a **surface**, and a surface is where |grad m| is high; the
    // band average includes deep air and buried soil, both near zero. So the
    // ratio reads ~3x for "ants drop on surfaces" with no contribution from
    // the response coefficient at all. Across five arms of this binary it
    // reads **2.96x shipped, 3.00x with the weight deleted outright, 3.59x
    // with it folded into the bias at its ceiling** -- the arm carrying the
    // mechanism reads *lowest of the three*, which is exactly the failure its
    // own predecessor was replaced for.
    //
    // It stays **printed and unasserted**, as the thing that shows the trap --
    // the same way `void` is kept beside `roofed` in `burrow_probe`. What
    // replaced it as the claim is the matched pair below.
    let grad_at = |world: &World, x: i32, y: i32| -> f64 {
        let gx = world.field_at_bilinear((x + 4) as f32, y as f32).moisture - world.field_at_bilinear((x - 4) as f32, y as f32).moisture;
        let gy = world.field_at_bilinear(x as f32, (y + 4) as f32).moisture - world.field_at_bilinear(x as f32, (y - 4) as f32).moisture;
        ((gx * gx + gy * gy).sqrt()) as f64
    };
    let (mut drop_grad, mut drop_n) = (0.0f64, 0u32);
    let (mut band_grad, mut band_n) = (0.0f64, 0u32);
    for x in 20..220 {
        for y in (floor - 12)..floor {
            band_grad += grad_at(&world, x, y);
            band_n += 1;
            if world.get(x, y).material == corpse && !(y == floor - 1 && x % 4 == 0) {
                drop_grad += grad_at(&world, x, y);
                drop_n += 1;
            }
        }
    }
    let ambient = band_grad / band_n.max(1) as f64;
    let at_drops = drop_grad / drop_n.max(1) as f64;
    let uphill = if ambient > 0.0 { at_drops / ambient } else { 0.0 };
    let st = world.creature_stats;
    println!("  pickups {} drops {} digs {} deaths {}", st.pickups, st.drops, st.digs, st.deaths);
    // The level the gradient is a gradient *of*: a flat field at zero and a
    // flat field at saturation both print 0.000 as a gradient and mean
    // opposite things, and it was the *level* being zero that turned out to
    // be bug H.
    let mean_moisture = |x0: i32, x1: i32| -> (f32, f32) {
        let (mut total, mut peak, mut n) = (0.0f32, 0.0f32, 0.0f32);
        for x in x0..x1 {
            for y in (floor - 10)..floor {
                let m = world.field_at_bilinear(x as f32, y as f32).moisture;
                total += m;
                peak = peak.max(m);
                n += 1.0;
            }
        }
        (total / n, peak)
    };
    let (wet_mean, wet_peak) = mean_moisture(20, w / 2);
    let (dry_mean, dry_peak) = mean_moisture(w / 2, 220);
    let margin = wet_grad - dry_grad;
    println!("  spring: {water_before} water cells at spawn, {water_after} standing at the end");
    println!("  mean |grad moisture| over {samples} samples: steep half {wet_grad:.4}, flat half {dry_grad:.4}, margin {margin:.4}");
    println!("  moisture level at the end: steep mean {wet_mean:.3} peak {wet_peak:.3} | flat mean {dry_mean:.3} peak {dry_peak:.3}");
    println!("  material left standing: steep half {wet_drops}, flat half {dry_drops}");
    println!("  |grad moisture| at the {drop_n} standing drops {at_drops:.4} vs {ambient:.4} ambient over the band -- {uphill:.2}x");
    // **The matched pair, and it is the one to read.** See its construction
    // above for why the band version cannot answer the question.
    let (drop_raw, stood_raw) = (event_grad / event_n.max(1) as f64, stood_grad / stood_n.max(1) as f64);
    let (drop_input, stood_input) = (event_in / event_n.max(1) as f64, stood_in / stood_n.max(1) as f64);
    let matched = if stood_raw > 0.0 { drop_raw / stood_raw } else { 0.0 };
    let matched_input = if stood_input > 0.0 { drop_input / stood_input } else { 0.0 };
    println!(
        "  matched: |grad moisture| at {event_n} attributed drop events {drop_raw:.4} vs {stood_raw:.4} where {stood_n} laden ants stood -- {matched:.2}x\n           (of {} drops: {ambiguous} discarded for a death in the same frame, {unattributed} not matched to a cell)",
        st.drops
    );
    println!(
        "  ...and on the value the brain is handed (clamped at 1.0): {drop_input:.4} at drops vs {stood_input:.4} standing -- {matched_input:.2}x"
    );
    let paired_mean = paired_sum / paired_n.max(1) as f64;
    let paired_share = paired_up as f64 / paired_n.max(1) as f64;
    println!(
        "  paired within frame: drop side higher on {paired_up} of {paired_n} frames ({:.1}%), mean lift {paired_mean:+.4}",
        100.0 * paired_share
    );
    // **The spring has to still be a spring**, checked before anything is
    // concluded from the field it feeds. This is the assertion bug H
    // actually needed: the old one asked whether the *gradient* was ordered,
    // which a world with no water in it answers with two zeroes and a
    // coin flip.
    assert!(
        water_after >= 20,
        "the spring dried up, so the scene no longer contains the gradient it tests: {water_before} cells at spawn, {water_after} at the end"
    );
    // **A continuous margin with headroom, not two floats compared with
    // `>`.** See `MARGIN_BAR` above for where the number comes from. The
    // predecessor turned on a difference below the sixth decimal and
    // flipped between two CI runs that printed identical numbers.
    assert!(
        initial_margin > MARGIN_BAR,
        "the scene must actually contain the gradient it is testing, measured before the colony touches it: {initial_margin:.4} <= {MARGIN_BAR} (the run mean was steep {wet_grad:.4} vs flat {dry_grad:.4}, margin {margin:.4})"
    );
    assert!(st.drops > 0, "no ant ever dropped anything -- the verb never fired");
    // **`wet_drops > dry_drops` was here and has been demoted to the print
    // above, because it was measured to be vacuous.** Deleting
    // `moisture_gradient` from the drop probability in `creature.rs`
    // entirely -- the whole mechanism this scene is named for -- left it
    // passing *harder*, steep 18 / flat 0 against steep 6 / flat 0: removing
    // a multiplier below 1.0 raises the drop rate everywhere, and the flat
    // half reads zero in both arms because the ants never travel that far.
    // A green light that cannot go red is the same thing as a skipped gate,
    // which is what this whole area has just cost us once already, so it
    // does not stay as an assertion.
    //
    // Its successor was the `uphill` ratio printed above, on 6 and 18 standing
    // drops, left unasserted because a bar set from a ratio of six cells is
    // the knife-edge this scene has already been bitten by. **That caution was
    // right and it was not enough: the ratio is not knife-edge, it is blind**
    // -- see its own comment above for the five arms that settle it.
    //
    // **What the claim is asserted on now.** `matched` compares |grad
    // moisture| at attributed drop *events* against |grad moisture| where
    // laden ants were standing **on the same frames**. Both sets are ant
    // positions, so "ants are on surfaces" divides out exactly; both are
    // sampled simultaneously, so the weather phase divides out too
    // (`CLAUDE.md`'s designed-oscillator rule -- the two arms below ended at
    // flat-half moisture means of 3.978 and 4.000 and the drift between
    // frames is larger than the effect). Under no bias it is 1.0 by
    // construction rather than by tuning.
    //
    // **Measured on this build, one seed, `PIXEL_PHYSICS_DROP_MOISTURE`
    // sweeping the fold that rate-matches the control:**
    //
    // | arm | drops | band ratio | **matched** | paired sign |
    // |---|---|---|---|---|
    // | **shipped** | **715** | 2.96x | **1.10x** | 55.5% |
    // | ablated, fold 0.85 | 179 | 3.00x | 0.93x | 46.7% |
    // | ablated, fold 0.90 | 581 | 3.00x | 0.84x | 43.2% |
    // | ablated, fold 0.95 | 1466 | **4.60x** | 0.80x | 30.2% |
    // | ablated, fold 0.98 | 1511 | 3.13x | 0.86x | 48.4% |
    //
    // Read the three numeric columns against each other, because they do not
    // agree and only one of them is a guard:
    //
    // * **`matched` separates the arms and nothing else does.** Every ablated
    //   arm reads under 1.0; the shipped arm reads 1.10x. **The four controls
    //   are what make this a bar rather than a number**: the drop rate spans
    //   195 to 1,513 across them and the sign never flips, so the separation
    //   is not the rate. `fold 0.90` is the rate-matched one to quote.
    // * **The band ratio is worse than useless, not merely weak.** Its best
    //   score in the table belongs to an arm with the mechanism *deleted*
    //   (4.60x), and the shipped arm is the **lowest row** in the column.
    // * **The paired sign test is printed and not trusted, because it did not
    //   survive being re-measured against a different colony.** Here it orders
    //   the arms correctly and looked like the robust statistic -- a sign
    //   test's null is exactly half, which is a principled bar rather than a
    //   fitted one. Run against an ant carrying one extra authored dig weight,
    //   all five arms sat at **34-46%** and it separated nothing, while
    //   `matched` held its ordering across both. One seed of a chaotic colony
    //   is a sample from a wide distribution (`CLAUDE.md`), and this is what
    //   that costs: the statistic that looked most principled was the one that
    //   moved when the world did.
    //
    // The bar is 1.03 -- above the null with headroom, and clear of the
    // strongest control (0.93) by about as much as the shipped arm clears it
    // (1.10). **It is a thin margin on one seed and that is stated rather
    // than dressed up.** If it ever fires, the first thing to do is not to
    // widen it: run `PIXEL_PHYSICS_DROP_MOISTURE=off:0.9` and read that arm --
    // it measured 0.84x. If the *ablated* arm also clears 1.03, the
    // replacement has gone blind in its turn and wants replacing rather than
    // retuning.
    assert!(
        event_n > 0,
        "no drop was attributed to a cell, so the matched ratio is over nothing -- the numerator is broken, not the mechanism"
    );
    assert!(
        stood_n > 0,
        "no laden ant was ever sampled, so the ratio has no denominator: {} drops over {paired_n} crediting frames",
        st.drops
    );
    assert!(
        matched > MATCHED_BAR,
        "deposition no longer follows the moisture gradient: |grad m| at {event_n} drop events {drop_raw:.4} against {stood_raw:.4} where laden ants stood -- {matched:.2}x, under the {MATCHED_BAR} bar. \
         Run PIXEL_PHYSICS_DROP_MOISTURE=off:0.9 before touching this number: that arm measured 0.84x, and if it now clears the bar too then this guard has gone blind rather than the mechanism having broken."
    );
}