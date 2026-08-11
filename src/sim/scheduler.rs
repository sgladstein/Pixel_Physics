//! M16: the active-site scheduler.
//!
//! Dirty rectangles answer "does the CA sweep need to look here" and are
//! keyed on *movement* -- a settled chunk with a fully-grown, motionless
//! forest in it is exactly the case they're built to let sleep. But a plant
//! that isn't moving still needs to grow, and waking a chunk's CA sweep to
//! give it that chance would re-examine every mobile cell in the chunk on
//! every tick, defeating sleeping in any world that contains vegetation.
//!
//! The active-site list is a second, much smaller schedule: a handful of
//! `(position, kind, next_frame)` entries, one per growing tip, checked each
//! frame in a pass that costs time proportional to the number of *growing
//! things*, not the size of the world. Explicitly a separate frame phase
//! from the CA sweep (`app.rs` runs it after `parallel::step`, before
//! particles) -- it writes cells via the ordinary `World::set`, which is
//! exactly as safe here as it is for painting or particle landings, since
//! nothing else is concurrently touching the world at this point in the
//! frame.
//!
//! Unlike the CA sweep's movement decisions, an active site's own growth
//! roll does *not* need to be position-keyed the way `roll_reach_at` does --
//! the scheduler itself is what guarantees a site gets re-examined at
//! `next_frame`, independent of chunk sleep state, so there's no equivalent
//! of "an unlucky roll could let the chunk sleep and freeze this forever."

use std::collections::HashMap;

use super::chunk::ChunkCoord;
use super::plant;
use super::structural;
use super::world::World;

/// What kind of growth an active site represents, and enough state to act
/// on it. `tree`/`tip`/`root` indices point into `World`'s tree-state
/// storage -- too much per-growth-point state (attractor lists, auxin
/// channel strength) to fit in a `Cell`'s spare bits, so it lives alongside
/// the schedule instead.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ActiveKind {
    /// A moss/lichen tip that may spread to a qualifying neighbour.
    /// `stale_ticks` counts consecutive checks that found nowhere to grow —
    /// once it crosses a threshold (`plant.rs::MOSS_STALE_LIMIT`), the tip
    /// stops rescheduling itself rather than being checked forever. Without
    /// this a moss patch that fully encloses itself (every neighbour either
    /// stone or more moss, permanently) would stay on the active-site list
    /// for the rest of the program's life, exactly the unbounded cost the
    /// scheduler's whole design exists to avoid.
    Moss { stale_ticks: u8 },
    /// A tree's growing branch tip.
    TreeTip { tree: u32, tip: u32 },
    /// A root tip extending through soil/rock in search of water.
    RootTip { tree: u32, root: u32 },
    /// M17: a `Solid` cell whose distance-to-anchor may need recomputing —
    /// scheduled reactively (painting, erasing, an explosion), never at
    /// world-gen time, so pre-placed terrain is never retroactively
    /// checked. See `structural.rs`.
    StructuralCheck,
}

#[derive(Clone, Copy, Debug)]
pub struct ActiveSite {
    pub x: i32,
    pub y: i32,
    pub kind: ActiveKind,
    pub next_frame: u64,
}

/// Run every active site due this frame, replacing `World`'s active-site
/// map with whatever each run asks to keep.
///
/// Structured like `parallel::run_pass`'s take-then-replace shape for the
/// same underlying reason -- `plant::tick` needs `&mut World` to read/write
/// cells and to schedule *new* sites (a moss tip spreading, a root
/// branching), which the scheduler's own storage can't be borrowed
/// alongside. Unlike the parallel sweep, this is plain sequential
/// bookkeeping, not a concurrency-safety requirement.
pub fn step(world: &mut World) {
    let due = world.frame;
    let mut sites = world.take_active_sites();
    let mut kept: HashMap<ChunkCoord, Vec<ActiveSite>> = HashMap::new();

    for (_, list) in sites.drain() {
        for site in list {
            if site.next_frame > due {
                push(&mut kept, site);
                continue;
            }
            let produced = match site.kind {
                ActiveKind::Moss { .. } | ActiveKind::TreeTip { .. } | ActiveKind::RootTip { .. } => plant::tick(world, &site),
                ActiveKind::StructuralCheck => structural::tick(world, &site),
            };
            for new_site in produced {
                push(&mut kept, new_site);
            }
        }
    }

    world.set_active_sites(kept);
}

fn push(map: &mut HashMap<ChunkCoord, Vec<ActiveSite>>, site: ActiveSite) {
    let coord = ChunkCoord::containing(site.x, site.y);
    map.entry(coord).or_default().push(site);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;

    fn test_world() -> World {
        World::new(Rect::new(0, 0, 127, 127))
    }

    #[test]
    fn a_site_scheduled_for_the_future_is_kept_untouched_until_due() {
        let mut w = test_world();
        w.schedule_active_site(ActiveSite {
            x: 10,
            y: 10,
            kind: ActiveKind::Moss { stale_ticks: 0 },
            next_frame: 100,
        });
        w.begin_step();
        step(&mut w);
        assert_eq!(w.active_site_count(), 1, "a not-yet-due site should not be dropped");
    }
}
