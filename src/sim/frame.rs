//! **One simulated tick, in the order the phases must run in.**
//!
//! This is the whole frame sequence and the *only* copy of it. It lives here
//! rather than in `app.rs` because the library now has more than one game
//! binary against it — the sandbox (`src/main.rs`) and the lab
//! (`src/bin/lab.rs`) — and the ordering below is not a detail either of them
//! is free to reproduce from memory. Every comment in `step` records a
//! frame-order constraint that was arrived at once and must not be
//! rediscovered; a second binary that re-typed the sequence would be a fork
//! of the simulation wearing the name of a second game.
//!
//! `Reports/evolution-lab-design-guide-2026-08-30.md` §7a names this exact
//! risk from the other end: *"The lab's speed comes from what is not in the
//! box, not from what is not in the binary."* Nothing here is skipped for the
//! lab. A sealed box with no rock, no blast and no gnome pays ~0 for the
//! phases it does not use — measured, feasibility §3c: blasts 0.000 ms,
//! particles 0.000 ms, the player 0.001 ms, the structural scheduler 0.028 ms
//! against 3.389 ms outdoors. So the lab runs the identical tick and gets its
//! frame time from its scene.
//!
//! **This must stay a faithful move, not a rewrite.** `App::update` delegates
//! to it, and `frame_step_matches_app_update` in `app.rs` hashes a world
//! stepped both ways to say so.

use crate::sim::explosion::Blasts;
use crate::sim::particle::{self, ParticleSystem};
use crate::sim::player;
use crate::sim::world::World;
use crate::sim::{parallel, rigid};

/// Advance `world` by exactly one tick.
///
/// Every argument is a system that owns state outside the cell grid, so it
/// cannot live on `World` and has to be threaded through: the particle
/// system, the blast staging list, and the character's input for this tick.
///
/// **The tick is the unit of simulated time and it is never scaled.** A
/// caller that wants the world to run faster calls this more times per
/// displayed frame; it does not speed anything up inside. `clock.rs` measured
/// what the other reading costs — the same number of organism ticks at 4x
/// `growth_slowdown` produced a median **0.61x** final cells across 8 seeds —
/// so "more ticks" is exact where "faster subsystems" is a behaviour change.
pub fn step(
    world: &mut World,
    particles: &mut ParticleSystem,
    blasts: &mut Blasts,
    player_input: player::PlayerInput,
    player_tuning: &player::Tuning,
) {
    parallel::step(world);
    // Liquid heightfield bodies (`Reports/liquid-heightfield-design.md`
    // §8a) after the sweep -- the sweep is what produces this frame's
    // absorptions once a later step adds them -- and before active
    // sites, so `plant::Absorb` reading an adjacent liquid cell sees
    // this frame's settled body state, the same reasoning the comment
    // below already gives for active sites running after the sweep.
    // Its own serial phase, not inside `parallel::step`, for the reason
    // that design doc section states: a body spanning two same-parity
    // active chunks writing its own columns from both workers would
    // violate the write-disjointness proof `parallel.rs`'s module doc
    // rests on. A no-op today -- step 1 of that design's build order
    // gives every promoted body no solver, so there is nothing yet for
    // this phase to do; wired in now so later steps land here rather
    // than needing frame-order surgery.
    world.step_liquid_bodies();
    // M8 chunk bodies in the same slot and for the same reason: a body
    // spanning two same-parity chunks would write to both from separate
    // workers and break `parallel.rs`'s write-disjointness proof
    // (`Reports/coupling-research.md` §4 states this outright), so it
    // gets its own serial phase. Before active sites, so a structural
    // check this frame sees a landed chunk's cells already in the grid
    // rather than a frame-old hole where they used to be.
    rigid::step_chunk_bodies(world);
    // M9: the character in the same serial slot as the bodies, right
    // after them — so standing on a body that settled this frame sees
    // its cells already in the grid, not a frame-old gap. The
    // edge-triggered press is consumed by the caller so that when a
    // catch-up loop runs several ticks in one frame, one press means one
    // jump; see `App::update` and `bin/lab.rs`, which both clear it
    // after this returns.
    player::step(world, player_input, player_tuning);
    // M16 active sites after the CA sweep too, for the same reason as
    // particles below: a root deciding whether to drink an adjacent
    // water cell needs this frame's settled position, not last frame's.
    world.step_active_sites();
    // Particles after the CA sweep, not before: a landing check needs
    // this frame's fully-settled CA state, not last frame's, or a
    // particle could land inside material that has since moved out from
    // under it. Field after that — order between the two does not
    // currently matter, since particles do not read or write the field,
    // but keeping the CA-derived phases grouped together here is easier
    // to reason about than interleaving them.
    // Blasts before particles: a blast stage clears cells and spawns
    // debris, and that debris should get its first `particle::step`
    // against the cavity this stage just opened rather than waiting a
    // frame for it -- which is the whole reason staging helps debris
    // escape at all (`sim::explosion::Tuning::duration`).
    blasts.step(world, particles);
    // Splashes between the two, for the same reason blasts come before
    // particles: the sweep reported these sites against this frame's
    // state, so they should be taken and thrown before the step that
    // moves everything, not left a frame stale. See
    // `particle::throw_splashes` -- this is the only place a splash
    // droplet's water is actually debited from the pool.
    particle::throw_splashes(world, particles);
    particles.step(world);
    world.step_fields();
    // Beside the field step, and for the same reason: a coarse
    // environmental channel with its own cadence, decoupled from the CA
    // sweep. `step_pheromones` gates itself on `PHEROMONE_INTERVAL`, so
    // this is called every frame like its neighbour above.
    world.step_pheromones();
}

#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::sim::cell::Cell;
    use crate::sim::material;

    /// A cheap order-sensitive digest of the whole grid. Same shape as
    /// `tests/determinism.rs`'s, deliberately: what it has to catch is a
    /// phase running in the wrong order, which moves cells rather than
    /// counts.
    fn world_hash(w: &crate::sim::world::World) -> u64 {
        fn fnv1a(h: u64, v: u64) -> u64 {
            (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
        }
        let b = w.bounds().expect("the scene sets bounds");
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
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
                    | (c.managed() as u64) << 4;
                h = fnv1a(h, flags);
            }
        }
        h
    }

    /// A scene that exercises every phase `step` orders: falling powder over
    /// standing liquid over a floor, so the CA sweep, the splash path, the
    /// particle step and the field all have something to do.
    fn scene() -> App {
        let mut app = App::new_pending();
        let (w, h) = (192i32, 128i32);
        let sand = app.world.materials.id_of("sand").expect("sand is compiled in");
        let water = app.world.materials.id_of("water").expect("water is compiled in");
        for x in 0..w {
            for y in (h - 8)..h {
                app.world.set(x, y, Cell::new(material::STONE, 0));
            }
            for y in (h - 40)..(h - 8) {
                app.world.set(x, y, Cell::new(water, 0));
            }
            for y in 20..44 {
                app.world.set(x, y, Cell::new(sand, ((x * 7 + y * 13) % 256) as u8));
            }
        }
        app
    }

    /// **The positive control for the move out of `App::update`, not a
    /// self-comparison.** Two copies of the sequence agreeing proves nothing
    /// (`CLAUDE.md`: a superseded mechanism's tests keep passing while
    /// testing nothing), so the number below was taken by running this exact
    /// scene through the *inline* `App::update` on `origin/main`, in a
    /// separate worktree, before the extraction landed. It is a value from
    /// the other side of the change.
    ///
    /// If this ever goes red, the tick sequence moved. That is either a
    /// deliberate simulation change — in which case re-take the number from
    /// the same scene and say what moved in the commit message — or a phase
    /// that was added to one binary's loop and not to `frame::step`, which is
    /// the failure this whole module exists to prevent.
    #[test]
    fn frame_step_matches_the_sequence_app_update_ran_before_extraction() {
        let mut app = scene();
        for _ in 0..120 {
            app.update();
        }
        assert_eq!(
            world_hash(&app.world),
            PRE_EXTRACTION_HASH,
            "120 ticks of the mixed sand/water scene no longer reproduce the state \
             `App::update`'s inline phase sequence produced on origin/main"
        );
    }

    /// Recorded 2026-08-30 from `origin/main`, one commit before
    /// `sim::frame` existed. See the test above for why it is a constant
    /// rather than a second run of the code under test.
    ///
    /// **Re-taken 2026-08-30 for `FIELD_SCALE` 8 -> 16**, which is a
    /// deliberate simulation change and so the first of the two cases the
    /// test's own doc names. Light, pressure, wind and moisture are all
    /// solved on a grid with a quarter as many cells, so this scene's
    /// water surface and settled sand differ from the first tick onward;
    /// the sequence of phases is untouched.
    ///
    /// The attribution is clean rather than assumed: with `FIELD_SCALE` put
    /// back to 8 and every other edit on this branch left in place, this
    /// scene reproduces the old value `15_147_976_901_438_684_952`
    /// **exactly**, so the move is `FIELD_SCALE`'s and nothing else's.
    ///
    /// **Re-taken 2026-08-31 for the evaporation vapour term**, again the
    /// first of the two cases the test's own doc names. Evaporated water is
    /// now added to the air block *above* the cell -- the one
    /// `evaporation::dryness` actually reads -- and into the diffusing
    /// `moisture` channel rather than a separate non-diffusing floor, so this
    /// scene's standing water humidifies the air over itself from the first
    /// tick and its own drying rate changes with it. The sequence of phases
    /// is untouched: `frame.rs`, `app.rs`, `parallel.rs` and `update.rs` are
    /// byte-identical to `origin/main` on that branch.
    ///
    /// The attribution is again clean rather than assumed, and it is a
    /// two-sided control. With `VAPOUR_PER_CELL_OPEN` and
    /// `VAPOUR_PER_CELL_ENCLOSED` both set to 0 and every other edit left in
    /// place, this scene reproduces the previous value
    /// `2_716_942_592_370_077_923` **exactly** -- and `origin/main` with
    /// *its* `VAPOUR_PER_CELL_EQUIVALENT` zeroed reproduces the same number,
    /// which says the old floor was a no-op on this scene (water pins its own
    /// block to saturation, so a floor beneath that reading changed nothing).
    /// So the move is the vapour term becoming *effective*, and the deletion
    /// of `FieldTile::vapour` that went with it is behaviour-neutral here.
    /// Stable across `RAYON_NUM_THREADS` 1 and 4.
    ///
    /// **What this constant no longer carries is its provenance**, and that
    /// is a real loss worth stating rather than papering over. The old value
    /// was taken from the *inline* `App::update` on the other side of the
    /// extraction, which is what made it a positive control instead of a
    /// self-comparison. That binary no longer exists, so the value below
    /// could only be taken from `frame::step` itself. It still pins the
    /// second failure the test names -- a phase added to one loop and not
    /// the other -- but it is a regression pin now, not a cross-check.
    const PRE_EXTRACTION_HASH: u64 = 6_411_500_948_612_927_299;
}
