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

// ---------------------------------------------------------------- the lab bed

// **The evolution lab's bed, which the scene above does not
// cover at all.**
//
// `build_scene` is sand, water, oil, a fire, a blast and a collapse — the
// *physics* path. It contains **no plants and no creatures**, so the whole
// organism half of the engine, and every genome draw in it, has never been
// under this gate. That matters now because the lab runs racks of chambers
// and compares them: a comparison between two boxes is only a statement
// about the boxes if a box run twice is the same box twice.
//
// **The vacuity guards had to be replaced rather than extended, and that is
// the interesting part.** The ones above assert ash and rubble exist —
// fire and fracture — and a sealed bed of soil under a grow light has
// neither, so *both would pass on a completely dead bed*. What replaces
// them has to prove the **heritable** path ran, not merely that something
// grew.
//
// **`organisms_born` is not that proof, and this is the trap worth
// recording**: `World::allocate_organism` increments it for *every*
// organism including the eight founders, so `organisms_born > 0` is true of
// a bed where nothing ever reproduced. `CLAUDE.md`'s *ask what your number
// counts when nothing is wrong*, in the exact costume that fools you.
//
// The guard that does work is **`plant_generation > 0`**: founders are
// generation 0 by construction, so any positive value means a seed set by a
// plant in this run germinated and carried an inherited genome. Measured on
// this bed with `labstats`, that first happens at **frame 1,800** (`borne 1
// ... gen p1`), against 0 at frame 900 — so `LAB_FRAMES` is 2,400, which
// clears it with a third to spare without paying for the 45,000 frames the
// stand needs to reach generation 5.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::stats::Stats;
use pixel_physics::sim::explosion::Blasts as LabBlasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::particle::ParticleSystem as LabParticles;
use pixel_physics::sim::player;

const LAB_FRAMES: u64 = 2_400;
const LAB_CHECKPOINT_EVERY: u64 = 600;

/// The shipped bed, with its colony. **Both halves deliberately**: the ants
/// are the creature RNG path (`creature.rs`'s six `rng::stream` sites), and
/// a plants-only scene would leave it as unguarded as the physics scene
/// leaves the plants.
fn lab_bed() -> LabBox {
    LabBox::default()
}

/// The same digest as `world_hash`, but over the bed's own bounds rather
/// than this file's `BOUNDS` const, plus the organism counters — a genotype
/// difference reaches the *cells* only once it has grown differently, and
/// this has to be able to see it before then.
fn lab_hash(w: &World) -> u64 {
    let b = w.bounds().expect("the lab bed sets bounds");
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            h = fnv1a(h, c.material.0 as u64);
            h = fnv1a(h, c.shade as u64);
            h = fnv1a(h, c.aux() as u64);
            h = fnv1a(h, c.organism_id() as u64);
            h = fnv1a(h, c.temperature() as u16 as u64);
        }
    }
    let (born, died) = w.organism_turnover();
    for v in [w.live_organism_count() as u64, w.live_creature_count() as u64, born, died, w.germinations] {
        h = fnv1a(h, v);
    }
    h
}

/// One headless run of the bed through `frame::step` — the *shared* tick,
/// the same one `Lab::tick` and `App::update` run, so this is not a second
/// copy of the sequence.
fn run_lab_bed() -> (Vec<u64>, World, Stats) {
    let mut world = lab_bed().build();
    let mut particles = LabParticles::new();
    let mut blasts = LabBlasts::new();
    let tuning = player::Tuning::default();
    let mut stats = Stats::new();
    let mut hashes = Vec::new();
    for f in 0..LAB_FRAMES {
        frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        // **Inside the loop, not after it.** `Stats::observe` gates on
        // `frame >= last + interval` — a `>=`, so it never skips and never
        // catches up: called once per N ticks it yields one sample spaced N
        // apart rather than N/interval samples, and the strip's resolution
        // becomes the call cadence instead of simulated time.
        stats.observe(&world);
        if (f + 1) % LAB_CHECKPOINT_EVERY == 0 {
            hashes.push(lab_hash(&world));
        }
    }
    (hashes, world, stats)
}

/// **A lab chamber run twice is the same chamber twice.**
///
/// The gate every rack comparison rests on. If this is red, a difference
/// between two chambers is partly the engine's own noise and no batch result
/// means anything — which is a finding to report, not a bug to route around.
#[test]
fn the_lab_bed_is_deterministic_across_identical_runs() {
    let (a, world, stats) = run_lab_bed();
    let (b, _, _) = run_lab_bed();
    assert_eq!(
        a, b,
        "two identical lab chambers diverged — a rack comparison would be \
         reporting the engine's own noise as a difference between boxes"
    );

    // Vacuity. Not the ash/rubble pair above: this bed has no fire and
    // nothing to break, so both of those would pass on a dead box.
    assert!(
        a.windows(2).any(|w| w[0] != w[1]),
        "the bed never changed between checkpoints — it has stopped exercising anything"
    );
    let census = stats.census().expect("a census after 2,400 frames");
    assert!(census.plants > 0, "nothing is alive in the bed");
    assert!(
        world.live_creature_count() > 0,
        "no animals — the creature RNG path is not under this test after all"
    );
    // **The one that proves reproduction happened**, and the reason
    // `organisms_born` is not used: that counter includes the founders, so
    // it is positive on a bed where nothing ever bred. Founders are
    // generation 0, so a positive deepest generation is an inherited genome.
    assert!(
        census.plant_generation > 0,
        "deepest plant generation is 0 — every plant is a founder, so this run \
         never reached a birth and the heritable path is untested (measured: \
         generation 1 arrives at frame 1,800 on this bed)"
    );
}
