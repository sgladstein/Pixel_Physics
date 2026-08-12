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

use std::cmp::Reverse;

use super::creature;
use super::decay;
use super::plant;
use super::structural;
use super::world::World;

/// What kind of growth an active site represents, and enough state to act
/// on it. `tree`/`tip`/`root` indices point into `World`'s tree-state
/// storage -- too much per-growth-point state (attractor lists, auxin
/// channel strength) to fit in a `Cell`'s spare bits, so it lives alongside
/// the schedule instead.
///
/// `Ord`/`Eq` (beyond the `PartialEq` this always derived) exist purely to
/// give `ActiveSite`'s own `Ord` impl a deterministic tiebreak -- see its
/// doc. The order variants compare in has no meaning of its own; it is
/// whatever `derive(Ord)`'s declaration-order rule produces, which is fine,
/// since all that's required is that it be *total* and *stable*, not
/// meaningful.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ActiveKind {
    /// An organism-owned cell (`Reports/organism-substrate-design.md`) due
    /// to run its species/cell-type behaviors — moss today, generic across
    /// any future species. `organism` is the owning organism's encoded id
    /// (`World::organism`); `x`/`y` on the containing `ActiveSite` is the
    /// specific cell. `stale_ticks` counts consecutive checks that found
    /// nothing to do — once it crosses a threshold
    /// (`organism::STALE_LIMIT`), the cell stops rescheduling itself rather
    /// than being checked forever, generalizing the same mechanism moss's
    /// own dormancy used before this rewrite (a patch fully enclosed by
    /// stone or more moss must not stay on the active-site list for the
    /// rest of the program's life — the unbounded cost the scheduler's
    /// whole design exists to avoid).
    Organism { organism: u16, stale_ticks: u8 },
    /// A tree's growing branch tip.
    TreeTip { tree: u32, tip: u32 },
    /// A root tip extending through soil/rock in search of water.
    RootTip { tree: u32, root: u32 },
    /// M17: a `Solid` cell whose distance-to-anchor may need recomputing —
    /// scheduled reactively (painting, erasing, an explosion), never at
    /// world-gen time, so pre-placed terrain is never retroactively
    /// checked. See `structural.rs`.
    StructuralCheck,
    /// M18: a creature due to make its next movement decision. `creature`
    /// indexes into `World`'s creature-state storage (too large to fit in
    /// `Cell::aux` alongside a growth stage or anchor distance would be) —
    /// see `creature::CreatureState`.
    Creature { creature: u16 },
    /// Architecture §5f: an `ash` cell due to re-check whether it's damp
    /// enough to decay into `soil`. Only scheduled reactively, by `fire.rs`
    /// at the moment a burnout actually produces ash — not for every ash
    /// cell that could ever exist (hand-painted ash, say), matching the
    /// report's own "cheap: one material, one slow transformation" framing.
    /// See `decay.rs`.
    Decay,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ActiveSite {
    pub x: i32,
    pub y: i32,
    pub kind: ActiveKind,
    pub next_frame: u64,
}

/// Ordered by `next_frame` first (soonest-due sorts smallest, matching
/// `BinaryHeap<Reverse<ActiveSite>>`'s use as a min-heap in `step` below),
/// then `(x, y, kind)` purely as a deterministic tiebreak for sites due on
/// the same frame -- not derived field-by-field in declaration order (which
/// would compare `x` before `next_frame`, backwards from what a priority
/// queue needs) precisely because `next_frame` must dominate. This is the
/// fix for issue #7's determinism half (`Reports/emergent-world-
/// architecture.md` §8b): the old `HashMap<ChunkCoord, Vec<ActiveSite>>`
/// drained in whatever order Rust's per-process-randomized hasher produced,
/// so two sites due the same frame (two moss tips racing for the same
/// empty neighbour, say) resolved differently run to run. A full,
/// stable-across-runs order removes that -- the *only* documented source
/// of non-determinism in the engine.
impl Ord for ActiveSite {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.next_frame
            .cmp(&other.next_frame)
            .then_with(|| self.x.cmp(&other.x))
            .then_with(|| self.y.cmp(&other.y))
            .then_with(|| self.kind.cmp(&other.kind))
    }
}

impl PartialOrd for ActiveSite {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Run every active site due this frame, replacing `World`'s active-site
/// heap with whatever each run asks to keep.
///
/// Structured like `parallel::run_pass`'s take-then-replace shape for the
/// same underlying reason -- `plant::tick` needs `&mut World` to read/write
/// cells and to schedule *new* sites (a moss tip spreading, a root
/// branching), which the scheduler's own storage can't be borrowed
/// alongside. Unlike the parallel sweep, this is plain sequential
/// bookkeeping, not a concurrency-safety requirement.
///
/// `BinaryHeap<Reverse<ActiveSite>>` (a min-heap on `next_frame`, see
/// `ActiveSite`'s own `Ord` impl) replaces the old
/// `HashMap<ChunkCoord, Vec<ActiveSite>>`, fixing both halves of issue #7
/// at once: **performance** -- the old version tested `next_frame > due`
/// against *every* pending site and rebuilt the whole map every frame
/// regardless of how many were actually due, which the module doc's own
/// claim ("checked only sites that are actually due") never matched; this
/// version peeks the heap's minimum and stops the instant it finds a
/// not-yet-due site, since nothing after it in a min-heap can be due
/// either, giving true O(due · log n) with no full-structure rebuild. And
/// **determinism** -- a `HashMap`'s iteration order is randomized per
/// process, so two sites due on the same frame (two moss tips racing for
/// the same empty neighbour) used to resolve differently run to run; the
/// heap's total order removes that, the one documented non-determinism
/// source in the engine (`Reports/emergent-world-architecture.md` §8b).
///
/// Newly-produced sites (a tip's own reschedule, a branch) are collected
/// into `produced_this_frame` and only pushed into the heap *after* the
/// due-processing loop below finishes, never re-examined against `due`
/// within this same call -- matching the old version's behaviour exactly
/// (it always deferred a freshly-produced site to the map that became
/// *next* frame's active sites), which matters if anything is ever
/// rescheduled with `next_frame == due` rather than strictly later.
pub fn step(world: &mut World) {
    let due = world.frame;
    let mut heap = world.take_active_sites();
    let mut produced_this_frame: Vec<ActiveSite> = Vec::new();

    while let Some(&Reverse(site)) = heap.peek() {
        if site.next_frame > due {
            break; // nothing else in a min-heap ordered by next_frame can be due either
        }
        heap.pop();
        let produced = match site.kind {
            ActiveKind::Organism { .. } | ActiveKind::TreeTip { .. } | ActiveKind::RootTip { .. } => plant::tick(world, &site),
            ActiveKind::StructuralCheck => structural::tick(world, &site),
            ActiveKind::Creature { .. } => creature::tick(world, &site),
            ActiveKind::Decay => decay::tick(world, &site),
        };
        produced_this_frame.extend(produced);
    }

    for site in produced_this_frame {
        heap.push(Reverse(site));
    }
    world.set_active_sites(heap);
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
            kind: ActiveKind::Organism { organism: 0, stale_ticks: 0 },
            next_frame: 100,
        });
        w.begin_step();
        step(&mut w);
        assert_eq!(w.active_site_count(), 1, "a not-yet-due site should not be dropped");
    }

    #[test]
    fn only_due_sites_are_processed_leaving_future_ones_untouched() {
        let mut w = test_world();
        w.schedule_active_site(ActiveSite { x: 1, y: 1, kind: ActiveKind::StructuralCheck, next_frame: 5 });
        w.schedule_active_site(ActiveSite { x: 2, y: 2, kind: ActiveKind::StructuralCheck, next_frame: 10 });
        w.schedule_active_site(ActiveSite { x: 3, y: 3, kind: ActiveKind::StructuralCheck, next_frame: 15 });

        for _ in 0..6 {
            w.begin_step();
        }
        // world.frame is now 6 -- only the next_frame: 5 site is due. Its
        // cell is empty, so structural::tick consumes it without producing
        // a reschedule (nothing to check).
        step(&mut w);
        assert_eq!(w.active_site_count(), 2, "exactly one of three sites should have been due and processed, leaving the other two pending");
    }

    #[test]
    fn active_sites_pop_in_a_fully_deterministic_tiebreak_order() {
        // Issue #7 / determinism §8b: the old HashMap<ChunkCoord, Vec<...>>
        // storage drained in whatever order Rust's per-process-randomized
        // hasher produced. Scheduled deliberately out of their eventual pop
        // order, to prove ordering comes from ActiveSite's own Ord impl
        // (next_frame, then x, then y, then kind) and not insertion order.
        let mut w = test_world();
        w.schedule_active_site(ActiveSite { x: 5, y: 5, kind: ActiveKind::StructuralCheck, next_frame: 10 });
        w.schedule_active_site(ActiveSite { x: 1, y: 1, kind: ActiveKind::StructuralCheck, next_frame: 10 });
        w.schedule_active_site(ActiveSite { x: 3, y: 3, kind: ActiveKind::StructuralCheck, next_frame: 10 });

        let mut heap = w.take_active_sites();
        let mut order = Vec::new();
        while let Some(Reverse(site)) = heap.pop() {
            order.push((site.x, site.y));
        }
        assert_eq!(order, vec![(1, 1), (3, 3), (5, 5)], "sites due on the same frame should pop in ascending (x, y) order, deterministically");
    }
}
