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

use super::creature;
use super::decay;
use super::evaporation;
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
    ///
    /// `plastochron` counts successful growth steps along *this lineage* —
    /// the interval between successive leaf primordia at a shoot apex, and
    /// the standard botanical term for the periodicity that spaces leaves
    /// along a shoot. `plant.rs`'s `Grow` advances it parent→child and
    /// retires a parent to `Leaf` instead of `MatureBody` when it comes
    /// round, which is what makes `CellType::Leaf` a cell anything actually
    /// produces.
    ///
    /// **Why it lives here and not in `Cell::aux`.** `aux` is full for an
    /// organism cell (4 bits type + 8 resource + 4 canopy density), and
    /// `Reports/plant-substrate-v2-design.md` §3a lists a plastochron
    /// counter among the four new scalars that make its sidecar migration a
    /// prerequisite for real leaves. That is true of the *other* three and
    /// not of this one: a plastochron is a property of a growing lineage,
    /// not of a cell — a retired `MatureBody` has no use for it and a fresh
    /// tip needs its parent's value, which is exactly the parent→child
    /// hand-off an `ActiveSite` already performs for `stale_ticks`. Riding
    /// here costs no `aux` bits and lets leaves land before the migration
    /// rather than behind it.
    ///
    /// `research/m16-plant-biology.md` §2 already recommends this same
    /// oscillator shape for lateral-root priming, over a flat per-tick
    /// branch probability, citing Moreno-Risueno et al. (2010) — one
    /// mechanism, two eventual users, neither invented here.
    Organism { organism: u16, stale_ticks: u8, plastochron: u8 },
    /// M17: a `Solid` cell whose distance-to-anchor may need recomputing —
    /// scheduled by whatever disturbs a structure (painting, erasing, an
    /// explosion). Generated terrain's distances are *not* built up through
    /// this queue: `structural::compute_world_distances` computes them in
    /// one converged pass at generation instead, deliberately, because
    /// `MAX_SITES_PER_FRAME` would spread a world's worth of terrain over
    /// many frames and cells can break spuriously mid-convergence. See
    /// `structural.rs`.
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
    /// An exposed liquid surface cell due to check whether it evaporates —
    /// `evaporation.rs`, which has the whole story. Scheduled by the CA
    /// sweep the moment a liquid cell stops moving with air above it, and
    /// self-rescheduling from then on: this is the one mechanic whose
    /// entire subject matter is *still* water, so it cannot live on a sweep
    /// that by design stops visiting cells the instant they settle.
    ///
    /// `stale_ticks` counts consecutive checks that found the cell covered,
    /// retiring the site at `evaporation::STALE_LIMIT` — the same shape
    /// `Organism` above uses, and for the same reason: a body sealed under
    /// rock must not check itself for the rest of the program's life. Note
    /// what it deliberately does *not* count — a surface that is exposed
    /// but too humid to evaporate is not stale, because that is a value
    /// that changes with the weather rather than a structure that does not.
    Evaporate { stale_ticks: u8 },
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
/// Every due site is popped out of `world`'s own heap up front, into a
/// plain `Vec`, before any dispatch runs — never taken out as a whole
/// heap and swapped back in at the end (`World::pop_due_active_site`'s own
/// doc explains why: that older shape left `world`'s heap genuinely empty
/// for the whole dispatch loop, silently discarding any `schedule_active_
/// site` call made *from inside* a dispatched tick). Because the due batch
/// is committed to a fixed list before dispatch starts, a site a tick
/// produces this frame — whether returned in its own `Vec<ActiveSite>` or
/// scheduled directly via `world.schedule_active_site` mid-tick — is never
/// re-examined against `due` within this same call even if it comes back
/// with `next_frame == due`, matching the scheduler's long-standing
/// "a freshly-produced site always waits for *next* frame" rule.
///
/// **Per-frame budget** (`MAX_SITES_PER_FRAME`): due-ness alone bounds
/// *which* sites this loop is allowed to touch, not *how many* — every
/// `StructuralCheck` a disturbance schedules is due immediately
/// (`next_frame` is always "now"), so a single large explosion can put
/// thousands of sites in front of this loop on the very frame it
/// happens. Deduping at the source (`World::schedule_active_site`) cuts
/// the raw count a lot, but a big-enough disturbance can still
/// legitimately have hundreds of genuinely distinct positions to check.
/// The cap stops popping after `MAX_SITES_PER_FRAME` regardless of how
/// many more are due, leaving the rest sitting in the heap exactly where
/// `pop_due_active_site` will find them again next frame — not lost, not
/// requeued, just deferred, spreading a big backlog across several frames
/// instead of spiking one.
pub fn step(world: &mut World) {
    // Refilled once per frame, before any site is dispatched: the load
    // walks a structural check performs are the expensive half of this
    // phase, and `MAX_SITES_PER_FRAME` alone does not bound them -- 2,000
    // cheap checks and 2,000 checks that each flood a thousand cells are
    // the same number of sites and three orders of magnitude apart in cost.
    world.load_budget = if std::env::var("PROBE_NO_LOAD").is_ok() { 0 } else { crate::sim::load::MAX_LOAD_CELLS_PER_FRAME };
    world.load_cache.clear();
    let due = world.frame;
    let mut due_sites = Vec::new();
    while due_sites.len() < MAX_SITES_PER_FRAME {
        match world.pop_due_active_site(due) {
            Some(site) => due_sites.push(site),
            None => break,
        }
    }

    for site in due_sites {
        let produced = match site.kind {
            ActiveKind::Organism { .. } => plant::tick(world, &site),
            ActiveKind::StructuralCheck => structural::tick(world, &site),
            ActiveKind::Creature { .. } => creature::tick(world, &site),
            ActiveKind::Decay => decay::tick(world, &site),
            ActiveKind::Evaporate { .. } => evaporation::tick(world, &site),
        };
        // Routed through the one canonical insertion point -- `world.
        // active_sites` is live for the whole loop now, so there's no
        // longer a separate "taken out" case to special-case here.
        for produced_site in produced {
            world.schedule_active_site(produced_site);
        }
    }
}

/// Starting point, not empirically pinned down to the frame budget yet —
/// generous enough that ordinary play (a few dozen growing tips, the odd
/// structural check) never comes close, and a real backstop against the
/// worst case named in `step`'s own doc: a large explosion's structural-
/// check flood, even after dedup. Revisit with a real per-site cost
/// measurement if a scene is ever found where this is either too low
/// (visibly slows legitimate settling) or too high (still spikes a frame).
const MAX_SITES_PER_FRAME: usize = 2000;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;

    fn test_world() -> World {
        World::new(Rect::new(0, 0, 127, 127))
    }

    #[test]
    fn scheduler_processes_at_most_the_per_frame_budget() {
        // Code-review-findings item #2: an explosion (or any single event)
        // can put far more due sites in front of `step` than it's safe to
        // process in one frame. `schedule_active_site` now dedups
        // `StructuralCheck` by position (see its own doc), so this needs
        // genuinely distinct positions -- a 2D spread, not a repeated
        // single row -- or the dedup this same test suite also checks
        // would collapse them long before the budget ever came into play.
        let mut w = test_world();
        let total = MAX_SITES_PER_FRAME + 500;
        for i in 0..total {
            w.schedule_active_site(ActiveSite {
                x: (i % 127) as i32,
                y: (i / 127) as i32,
                kind: ActiveKind::StructuralCheck,
                next_frame: 0,
            });
        }
        w.begin_step();
        step(&mut w);
        assert_eq!(
            w.active_site_count(),
            500,
            "the budget should cap processing at MAX_SITES_PER_FRAME this frame, leaving the rest pending"
        );

        // Due-ness hasn't changed (still frame 0) -- a second call should
        // drain exactly what the first one deferred, not lose or re-cap it.
        step(&mut w);
        assert_eq!(w.active_site_count(), 0, "a second step call should finish draining what the budget deferred");
    }

    #[test]
    fn a_site_scheduled_for_the_future_is_kept_untouched_until_due() {
        let mut w = test_world();
        w.schedule_active_site(ActiveSite {
            x: 10,
            y: 10,
            kind: ActiveKind::Organism { organism: 0, stale_ticks: 0, plastochron: 0 },
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

        let mut order = Vec::new();
        while let Some(site) = w.pop_due_active_site(10) {
            order.push((site.x, site.y));
        }
        assert_eq!(order, vec![(1, 1), (3, 3), (5, 5)], "sites due on the same frame should pop in ascending (x, y) order, deterministically");
    }
}
