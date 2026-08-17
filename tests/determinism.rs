//! Same-build determinism, verified rather than assumed.
//!
//! `PLAN.md` reversed determinism from "not required" to **required**
//! (same-build deterministic replay), because off-camera catch-up is only
//! sound if outcome = f(state, elapsed time, seed). The engine has already
//! been bitten twice by the standard way this silently breaks — `HashMap`
//! iteration order (`scheduler::step`'s drain order, issue #7; the fragment
//! seeding in `load::failing_region`'s first draft) — and both were caught
//! by reading, not by any test. This file is the test.
//!
//! # Why two runs in one process is a real check
//!
//! `std::collections::HashMap` seeds its hasher **per instance**, not per
//! process, so two `World`s built identically in the same test still
//! iterate their chunk maps in different orders. Any place the frame loop
//! lets that order reach observable state — a float accumulated in map
//! order, a queue replayed in map order, a flood seeded from a set — shows
//! up here as a hash divergence between two runs that did exactly the same
//! thing. The engine's own defences (`chunks_to_sweep` sorts, the field
//! solve is Jacobi, every RNG is fixed-seed or position-derived) are
//! precisely what this asserts still hold.
//!
//! # What is deliberately not claimed
//!
//! - **Serial and parallel runs do not match each other.** Each chunk owns
//!   an `Rng` stream, so the two drivers diverge by design
//!   (`parallel::step`'s own doc). Each driver is asserted deterministic
//!   against *itself*.
//! - **Nothing across builds or platforms.** Same-build only, per the
//!   decision's own scope.
//!
//! The scene exercises every frame phase the app runs: falling powder,
//! levelling water, an oil fire (heat, light, burnout), a staged explosion
//! (field impulse, debris particles), a strike, and a structural collapse
//! (load model, fracture, chunk bodies, landing). Vacuity is guarded the
//! way `CLAUDE.md` demands — counters, not the picture: the run must have
//! consumed oil and produced rubble, or it stopped exercising what it
//! claims to.

use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::material;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{parallel, rigid, structural, update};

/// World bounds: 256x192 cells = 4x3 chunks, small enough to run twice per
/// driver in a test, large enough that the parallel driver actually runs
/// multi-chunk passes with cross-chunk writes to replay.
const BOUNDS: (i32, i32) = (255, 191);
const FRAMES: u64 = 300;
const CHECKPOINT_EVERY: u64 = 50;

/// Where the floor's top surface sits (stone from here down to the bottom
/// edge, which reads as bedrock and anchors it).
const FLOOR_TOP: i32 = 184;

fn build_scene() -> World {
    let mut w = World::new(Rect::new(0, 0, BOUNDS.0, BOUNDS.1));
    // Stone floor down to the bottom world edge.
    for y in FLOOR_TOP..=BOUNDS.1 {
        for x in 0..=BOUNDS.0 {
            w.set(x, y, Cell::new(material::STONE, (x % 3) as u8));
        }
    }
    // A stone shelf cantilevered off the left world edge — long and deep
    // enough that the load model brings part of it down during the run,
    // which drags fracture, chunk bodies and landing into the test.
    for y in 100..=103 {
        for x in 0..=40 {
            w.set(x, y, Cell::new(material::STONE, (y % 2) as u8));
        }
    }
    // A block of sand that falls and piles.
    for y in 120..=140 {
        for x in 60..=110 {
            w.set(x, y, Cell::new(material::SAND, ((x + y) % 3) as u8));
        }
    }
    // A block of water that falls and levels.
    for y in 120..=135 {
        for x in 120..=170 {
            w.set(x, y, Cell::new(material::WATER, 0));
        }
    }
    // An oil pool resting on the floor, ignited mid-run.
    for y in FLOOR_TOP - 3..FLOOR_TOP {
        for x in 180..=230 {
            w.set(x, y, Cell::new(material::OIL, 0));
        }
    }
    // The same converged pass `app::build_terrain` runs, so the structural
    // state starts settled rather than mid-relaxation.
    structural::compute_world_distances(&mut w);
    w
}

fn fnv1a(h: u64, v: u64) -> u64 {
    (h ^ v).wrapping_mul(0x100_0000_01b3)
}

/// Every observable channel of the world, folded into one number: the full
/// per-cell state, and the field grid sampled at its own resolution. Floats
/// go in as raw bits — the claim is bit-identity, not closeness.
fn world_hash(w: &World) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for y in 0..=BOUNDS.1 {
        for x in 0..=BOUNDS.0 {
            let c = w.get(x, y);
            h = fnv1a(h, c.material.0 as u64);
            h = fnv1a(h, c.shade as u64);
            h = fnv1a(h, c.aux() as u64);
            h = fnv1a(h, c.organism_id() as u64);
            h = fnv1a(h, c.temperature() as u16 as u64);
            h = fnv1a(h, c.burn_remaining() as u64);
            let flags = (c.is_burning() as u64)
                | (c.flowing() as u64) << 1
                | (c.undercut() as u64) << 2
                | (c.attached() as u64) << 3
                | (c.managed() as u64) << 4
                | (c.crack_right() as u64) << 5
                | (c.crack_down() as u64) << 6;
            h = fnv1a(h, flags);
        }
    }
    for y in (0..=BOUNDS.1).step_by(8) {
        for x in (0..=BOUNDS.0).step_by(8) {
            let f = w.field_at(x, y);
            for bits in [
                f.pressure.to_bits(),
                f.vx.to_bits(),
                f.vy.to_bits(),
                f.temperature.to_bits(),
                f.light.to_bits(),
                f.moisture.to_bits(),
            ] {
                h = fnv1a(h, bits as u64);
            }
        }
    }
    h
}

fn count_material(w: &World, id: material::MaterialId) -> usize {
    let mut n = 0;
    for y in 0..=BOUNDS.1 {
        for x in 0..=BOUNDS.0 {
            if w.get(x, y).material == id {
                n += 1;
            }
        }
    }
    n
}

/// One full run of the scene under the app's real frame order
/// (`App::update`): CA sweep, liquid bodies, chunk bodies, active sites,
/// blasts, particles, field. Returns the checkpoint hashes and the final
/// world for the vacuity guards.
fn run_scene(parallel_driver: bool) -> (Vec<u64>, World) {
    let mut world = build_scene();
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let mut hashes = Vec::new();
    for frame in 0..FRAMES {
        if frame == 30 {
            world.ignite_circle(205, FLOOR_TOP - 2, 4);
        }
        if frame == 90 {
            blasts.trigger_with(&mut world, &mut particles, 85, FLOOR_TOP - 4, 12, 180.0);
        }
        if frame == 150 {
            rigid::strike(&mut world, 20, 101, 6, 8.0);
        }
        if parallel_driver {
            parallel::step(&mut world);
        } else {
            update::step(&mut world);
        }
        world.step_liquid_bodies();
        rigid::step_chunk_bodies(&mut world);
        world.step_active_sites();
        blasts.step(&mut world, &mut particles);
        particles.step(&mut world);
        world.step_fields();
        if (frame + 1) % CHECKPOINT_EVERY == 0 {
            hashes.push(world_hash(&world));
        }
    }
    (hashes, world)
}

fn assert_deterministic(parallel_driver: bool) {
    let (a, world) = run_scene(parallel_driver);
    let (b, _) = run_scene(parallel_driver);
    assert_eq!(
        a, b,
        "two identical runs diverged — some frame phase depends on HashMap \
         iteration order, thread scheduling, or another per-instance seed"
    );

    // Vacuity guards: a determinism test over a still life proves nothing.
    assert!(
        a.windows(2).any(|w| w[0] != w[1]),
        "the world never changed between checkpoints; the scene has stopped exercising anything"
    );
    // Ash, not a drop in oil *cell count*: a `Liquid` cell holds continuous
    // fill, so the pool spreads 153 full cells into ~325 partial ones long
    // before ignition, and a count can rise while volume burns away. The
    // first draft of this guard made exactly that mistake — `CLAUDE.md`'s
    // "measure column volume, not cells" trap, in a test about fire.
    assert!(
        count_material(&world, material::ASH) > 0,
        "no ash — the oil fire never burned through to burnout in this scene"
    );
    assert!(
        count_material(&world, material::RUBBLE) > 0,
        "nothing ever broke — the strike and the shelf collapse have stopped reaching this scene"
    );
}

#[test]
fn the_parallel_driver_is_deterministic_across_identical_runs() {
    assert_deterministic(true);
}

#[test]
fn the_serial_driver_is_deterministic_across_identical_runs() {
    assert_deterministic(false);
}
