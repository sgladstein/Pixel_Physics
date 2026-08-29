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
use super::update;
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
    /// M18: a creature due to make its next movement decision.
    ///
    /// `organism` is the owning organism's **encoded generational handle**,
    /// exactly like `Organism` above — a creature is an organism, and the
    /// only thing separating the two variants is which module gets
    /// dispatched. It used to be a raw index into a parallel
    /// `World::creatures` vector with no generation and no reclamation, so
    /// a site outliving its creature read whatever had been allocated the
    /// same index since. Now a stale handle resolves to `None` and the site
    /// drops itself, which is the entire point of the scheme
    /// (`Reports/organism-substrate-design.md` §6).
    ///
    /// `x`/`y` on the containing `ActiveSite` is the creature's **head**.
    Creature { organism: u16 },
    /// Architecture §5f: a cell of a **decayable** material due to re-check
    /// whether it is damp enough to decay into whatever its
    /// `Material::decays_into` names — `ash` into `soil`, `litter` into
    /// `soil`.
    ///
    /// **No longer reactive-only, and the reason is worth having here.**
    /// This used to be scheduled solely by `fire.rs` at the moment a
    /// burnout produced ash, explicitly *not* for every ash cell that could
    /// ever exist (hand-painted ash, say). That stopped being true when
    /// litter arrived: a shed leaf is created in a canopy and *falls*, and
    /// a decay site is a bare coordinate that nothing makes follow its
    /// cell, so scheduling at creation stranded the site every time
    /// (`Reports/open-bugs-handoff.md` §0e). `World::end_step` now scans a
    /// chunk on its **awake -> settled** transition and schedules a site
    /// for every decayable cell in it, which is both the cheap point and
    /// the correct one — weathering happens to matter that has come to
    /// rest. Hand-painted ash therefore does decay now, once it settles.
    ///
    /// Deduped through `World::pending_decay_sites`, because a drift that
    /// is disturbed and re-settles repeatedly would otherwise stack sites
    /// and make the effective rate a function of how often the ground was
    /// walked on. See `decay.rs`.
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
    /// A `Gas` cell that could not move, due for a dissipation roll —
    /// `update.rs`'s `dissipation_tick`.
    ///
    /// Here for precisely the reason `Evaporate` is, one kind along: the CA
    /// sweep can roll a gas cell's `Material::dissipation` for free while
    /// the cell is moving, and a cell that has *stopped* moving is the one
    /// the mechanic exists for and the one the sweep stops visiting. That is
    /// not a hypothesis — the sweep-only version was built first and
    /// measured: a stone box packed with 336 smoke cells lost 25 of them and
    /// then kept the other 311 for all 2,500 frames it was watched, because its
    /// chunks settled about nineteen frames after the smoke did. A buried
    /// blast's crater, the case this was built for, kept three of its five.
    ///
    /// No `stale_ticks` twin to `Evaporate`'s, deliberately. A sealed gas
    /// cell is not in a state that might later become interesting again —
    /// the roll it is waiting for is unconditional, so the site always has
    /// work to do and always terminates on its own: it retires the moment
    /// the roll succeeds, and dissipation is the one outcome nothing can
    /// stop. The unbounded-cost failure `stale_ticks` exists to prevent
    /// cannot arise here.
    Dissipate,
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
/// Per-kind wall time and site count inside [`step`], printed when
/// `SCHED_PASS=<every N frames>` is set. Off by default and free when off.
///
/// **Why this exists.** `scale_probe phases=1 load=` measured this phase at
/// **0.317 ms idle and 9.429 ms with a 64-ant colony in the world** -- a 30x
/// jump that took it from 2.4% of the frame to 35.1%, nearly tying the field.
/// A single total cannot say which of the six site kinds that is, and they
/// want completely different fixes: a creature tick is per-ant per-interval
/// work, a structural check drags the load model's flood walks behind it
/// (`open-bugs-handoff.md` §1j measured 118 ms on a destruction scene, and
/// that cost lands *here* rather than in the `blasts` row, because
/// `structural::tick` is dispatched from this loop), and decay/evaporation
/// are cheap per site but can arrive in bulk.
///
/// **Counts as well as times**, for the reason every counter in this repo
/// exists: 2,000 cheap checks and 2,000 checks that each flood a thousand
/// cells are the same number of sites and three orders of magnitude apart in
/// cost, which is the observation `step`'s own load-budget comment opens
/// with. Time alone cannot separate "many sites" from "expensive sites";
/// time *and* count can.
#[derive(Default)]
struct SchedTiming {
    every: u64,
    ms: [f64; 6],
    n: [u32; 6],
    deferred: usize,
    staged_ms: f64,
    /// Sites *scheduled by* this frame's ticks. The decisive number when a
    /// backlog will not drain: if `produced` matches the number popped, each
    /// site is replacing itself and the queue is self-sustaining however fast
    /// it is drained. `CLAUDE.md` records a reverted change with exactly this
    /// signature -- "every settling cell raised a fresh check on its parent
    /// ... faster than the queue drained".
    produced: usize,
    /// Which branch of `structural::tick` produced them -- see
    /// `structural::TickCensus`. `produced` says the queue is
    /// self-sustaining; this says what sustains it.
    census: structural::TickCensus,
}

impl SchedTiming {
    const NAMES: [&'static str; 6] =
        ["organism", "structural", "creature", "decay", "evaporate", "dissipate"];

    fn new() -> Self {
        use std::sync::OnceLock;
        static EVERY: OnceLock<u64> = OnceLock::new();
        let every = *EVERY.get_or_init(|| std::env::var("SCHED_PASS").ok().and_then(|v| v.parse().ok()).unwrap_or(0));
        SchedTiming { every, ..Default::default() }
    }

    fn slot(kind: &ActiveKind) -> usize {
        match kind {
            ActiveKind::Organism { .. } => 0,
            ActiveKind::StructuralCheck => 1,
            ActiveKind::Creature { .. } => 2,
            ActiveKind::Decay => 3,
            ActiveKind::Evaporate { .. } => 4,
            ActiveKind::Dissipate => 5,
        }
    }

    fn time<R>(&mut self, kind: &ActiveKind, f: impl FnOnce() -> R) -> R {
        if self.every == 0 {
            return f();
        }
        let slot = Self::slot(kind);
        let t = std::time::Instant::now();
        let r = f();
        self.ms[slot] += t.elapsed().as_secs_f64() * 1000.0;
        self.n[slot] += 1;
        r
    }

    fn report(&self, frame: u64) {
        if self.every == 0 || !frame.is_multiple_of(self.every) {
            return;
        }
        let total: f64 = self.ms.iter().sum::<f64>() + self.staged_ms;
        let detail: Vec<String> = Self::NAMES
            .iter()
            .zip(self.ms.iter())
            .zip(self.n.iter())
            .filter(|((_, ms), n)| **n > 0 || **ms > 0.0)
            .map(|((name, ms), n)| format!("{name} {ms:.2}/{n}"))
            .collect();
        println!(
            "  [sched] frame {frame:>6} sites {:>5} produced {:>5} deferred {:>6} total {total:>7.2}ms | staged {:.2} | {}",
            self.n.iter().sum::<u32>(),
            self.produced,
            self.deferred,
            self.staged_ms,
            detail.join("  ")
        );
        let c = self.census;
        if c.worsened + c.improved + c.unmoved > 0 {
            println!(
                "  [struct] frame {frame:>6} worsened {:>5} improved {:>5} unmoved {:>5} | budget0 {:>5} chain-deferred {:>5} uninteresting {:>5} | grounded {:>5} (flat {:>5}) | max aux {}",
                c.worsened, c.improved, c.unmoved, c.budget0, c.chain_deferred, c.uninteresting, c.grounded, c.grounded_flat, c.max_aux
            );
            // §S5: which cap answered "supported" without finishing the
            // search. Printed beside the rest rather than folded into it --
            // these are *wrong answers*, where the line above counts work.
            if c.walk_capped + c.region_capped + c.supported_budget0 + c.rootward_capped > 0 {
                println!(
                    "  [caps]   frame {frame:>6} walk {:>5} region {:>5} budget0-in-supported {:>5} rootward {:>5}",
                    c.walk_capped, c.region_capped, c.supported_budget0, c.rootward_capped
                );
            }
        }
    }
}

pub fn step(world: &mut World) {
    // Refilled once per frame, before any site is dispatched: the load
    // walks a structural check performs are the expensive half of this
    // phase, and `MAX_SITES_PER_FRAME` alone does not bound them -- 2,000
    // cheap checks and 2,000 checks that each flood a thousand cells are
    // the same number of sites and three orders of magnitude apart in cost.
    world.load_budget = if std::env::var("PROBE_NO_LOAD").is_ok() { 0 } else { crate::sim::load::MAX_LOAD_CELLS_PER_FRAME };
    world.load_cache.clear();
    let due = world.frame;
    // **Creature ticks get their own budget, and are then merged back into
    // their natural place.** Not a priority — see below, and see
    // `World::creature_sites` for why they get a separate budget at all.
    // In one line: background sites sit at a `next_frame` in the past and
    // a creature reschedules itself into the future, so while the backlog
    // is deeper than `MAX_SITES_PER_FRAME` a creature is not merely behind
    // in the queue, it is unreachable.
    //
    // Empty, and therefore a no-op, under `CREATURE_PRIORITY=0`.
    //
    // **A bound on work, never a gate on whether a creature ticks**, which
    // is the distinction `CLAUDE.md` insists on for every cap in this
    // engine: exhausting this produces *less work this frame*, not an
    // answer. Whatever is left stays in `creature_sites` at its own
    // `next_frame` and is the very first thing popped next frame, so the
    // worst case degrades to the pooled behaviour rather than to silence.
    //
    // Sized so it cannot bind in play and can still stop a pathological
    // population from owning a frame: at `ant.ron`'s `tick_interval` 6 it
    // is ~1,500 simultaneous creatures, against a `found_colony` of 52.
    let mut creature_due = Vec::new();
    while creature_due.len() < MAX_CREATURE_SITES_PER_FRAME {
        match world.pop_due_creature_site(due) {
            Some(site) => creature_due.push(site),
            None => break,
        }
    }
    let mut other_due = Vec::new();
    while other_due.len() < MAX_SITES_PER_FRAME {
        match world.pop_due_active_site(due) {
            Some(site) => other_due.push(site),
            None => break,
        }
    }
    // **Merged, not concatenated, and this is the whole safety argument.**
    // Both lists come off min-heaps, so both are already ascending in
    // `ActiveSite`'s own `Ord`; merging them reproduces exactly the order
    // a single heap would have popped. So whenever the budget is *not*
    // binding — which is every frame of ordinary play, measured at 331
    // due sites against a cap of 2,000 on a 8192x2560 world with a colony
    // in it — this dispatches the identical set in the identical order and
    // the world is bit-identical to the pooled version. The reserve can
    // only change anything in the one case it exists for.
    //
    // That matters more than it looks. Dispatch order is a behaviour
    // input here: ticks write cells, and two sites due on the same frame
    // racing for the same neighbour resolve by this order — it is the
    // engine's one documented determinism surface (`ActiveSite`'s `Ord`).
    // A version that simply ran creatures first would have changed every
    // emergent outcome in the engine to fix a condition that, measured,
    // is not currently occurring.
    //
    // A merge rather than a `sort_unstable` deliberately, and not only for
    // the linear cost: `CLAUDE.md`'s tie-order gotcha says an unstable
    // sort's treatment of equal elements depends on the *element type*,
    // not just the comparator. There is nothing to reason about here.
    let mut due_sites = Vec::with_capacity(creature_due.len() + other_due.len());
    let (mut i, mut j) = (0, 0);
    while i < creature_due.len() && j < other_due.len() {
        if creature_due[i] <= other_due[j] {
            due_sites.push(creature_due[i]);
            i += 1;
        } else {
            due_sites.push(other_due[j]);
            j += 1;
        }
    }
    due_sites.extend_from_slice(&creature_due[i..]);
    due_sites.extend_from_slice(&other_due[j..]);

    let mut timing = SchedTiming::new();
    // What is still waiting when the frame's site budget runs out. A phase
    // that is expensive *and* draining its backlog is a different problem
    // from one that is expensive and falling behind, and only this number
    // separates them.
    timing.deferred = world.active_site_count();
    for site in due_sites {
        let produced = timing.time(&site.kind, || match site.kind {
            ActiveKind::Organism { .. } => plant::tick(world, &site),
            ActiveKind::StructuralCheck => structural::tick(world, &site),
            ActiveKind::Creature { .. } => creature::tick(world, &site),
            ActiveKind::Decay => decay::tick(world, &site),
            ActiveKind::Evaporate { .. } => evaporation::tick(world, &site),
            ActiveKind::Dissipate => update::dissipation_tick(world, &site),
        });
        // Routed through the one canonical insertion point -- `world.
        // active_sites` is live for the whole loop now, so there's no
        // longer a separate "taken out" case to special-case here.
        timing.produced += produced.len();
        for produced_site in produced {
            world.schedule_active_site(produced_site);
        }
    }
    // A collapse too big to fracture in one tick comes down over several,
    // and this is what advances it. Outside the site loop above on purpose:
    // it is *work already decided*, not a question about a cell, so it
    // neither competes for `MAX_SITES_PER_FRAME` nor gets starved by the
    // load budget those sites drain — which is what happened when it was
    // driven by rescheduled checks. See `structural::advance_staged_fractures`.
    let t = std::time::Instant::now();
    structural::advance_staged_fractures(world);
    if timing.every > 0 {
        timing.staged_ms = t.elapsed().as_secs_f64() * 1000.0;
    }
    // Repair the support field over what this frame's damage invalidated,
    // outside the site loop for the same reason `advance_staged_fractures`
    // is: it is work already decided by a verb that fired, not a question
    // about a cell, so it must neither compete for `MAX_SITES_PER_FRAME`
    // nor be starved by the load budget those sites drain. Off unless
    // `STRUCT_RECONVERGE=1` -- see `structural::reconverge_from_damage`.
    let t = std::time::Instant::now();
    let r = structural::reconverge_from_damage(world);
    // **Printed on the frame it fires, not on the reporting frame.** A
    // charge lands on one frame in ten thousand, so folding this into
    // `SchedTiming`'s per-frame line would report it as zero every time it
    // was sampled -- the reporting cadence would silently hide the only
    // event worth reading. `CLAUDE.md`'s "did it fire at all needs a
    // counter, not a picture", applied to a counter that fires rarely.
    if timing.every > 0 && r.seeds > 0 {
        println!(
            "  [reconv] frame {:>6} seeds {:>6} invalidated {:>6} repathed {:>6}{} | {:.2}ms",
            world.frame,
            r.seeds,
            r.invalidated,
            r.repathed,
            if r.abandoned { " ABANDONED (over cap)" } else { "" },
            t.elapsed().as_secs_f64() * 1000.0
        );
    }
    // Drained unconditionally, not only on a reporting frame: the counters
    // are per-frame, and leaving them to accumulate on the frames between
    // reports would make every printed line a running total wearing a
    // per-frame label.
    timing.census = structural::take_tick_census();
    timing.report(world.frame);
}

/// Starting point, not empirically pinned down to the frame budget yet —
/// generous enough that ordinary play (a few dozen growing tips, the odd
/// structural check) never comes close, and a real backstop against the
/// worst case named in `step`'s own doc: a large explosion's structural-
/// check flood, even after dedup. Revisit with a real per-site cost
/// measurement if a scene is ever found where this is either too low
/// (visibly slows legitimate settling) or too high (still spikes a frame).
const MAX_SITES_PER_FRAME: usize = 2000;

/// The reserved per-frame budget for creature ticks — see
/// `World::creature_sites` for why they get one at all.
///
/// **This is a ceiling on a queue that is not supposed to reach it.** A
/// colony of 52 ants at `tick_interval` 6 offers ~9 sites a frame, so the
/// steady state is two orders of magnitude under this and the number is
/// doing nothing in ordinary play. What it is for is the pathological
/// case — a world someone has filled with creatures — where an unbounded
/// reserve would let the population own the frame outright, which is the
/// same failure the shared budget above exists to prevent, arriving from
/// the other side.
const MAX_CREATURE_SITES_PER_FRAME: usize = 256;

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

    /// **A creature keeps its own cadence while world-scale work is
    /// backlogged past the per-frame budget.**
    ///
    /// This is the guard for the owner's playtest report of 2026-08-29 —
    /// *"creatures seem to be moving slowly right now. Long pause, move a
    /// pixel, long pause, move one more pixel"* — and it fails without
    /// `World::creature_sites`, which is the point.
    ///
    /// # Why the flood is shaped the way it is
    ///
    /// Three properties, and dropping any one makes the test green for the
    /// wrong reason:
    ///
    /// - **More than the budget, every frame.** One over-budget frame
    ///   drains the next; only a source that keeps producing holds the
    ///   frontier behind the present, which is the state the whole bug
    ///   lives in. This is not contrived — `open-bugs-handoff.md` §S
    ///   measured exactly it in play, ~7,600 sites produced against 2,000
    ///   drained, for eleven thousand frames after a single charge.
    /// - **Distinct positions**, or `schedule_active_site`'s per-position
    ///   dedup collapses the flood and there is no backlog at all.
    /// - **Positions that sort ahead of the ant.** `ActiveSite`'s `Ord` is
    ///   `next_frame` then `x`, so a flood to the ant's *west* is what puts
    ///   it last among sites due on the same frame. A flood to the east
    ///   would let the ant through on the tiebreak and the test would pass
    ///   against the pooled queue as well — green, and evidence of nothing.
    ///
    /// # What it asserts, and why not `moves`
    ///
    /// `ticks`, not `moves`: whether the ant is *asked*. Whether it then
    /// moves is the brain's business and depends on a roll, so an
    /// assertion on `moves` would be a loose bound on emergent behaviour —
    /// exactly the kind `CLAUDE.md` says goes blind. Being scheduled is
    /// not emergent: a creature reschedules itself to an exact frame, so
    /// the expected count is arithmetic and the honest bar is close to it.
    #[test]
    fn a_creature_keeps_its_cadence_under_a_background_site_backlog() {
        // Wide enough that the flood has thousands of distinct positions
        // west of the ant, which is what the tiebreak argument above needs.
        let mut w = World::new(Rect::new(0, 0, 511, 255));
        for cx in 0..512 {
            w.set(cx, 101, crate::sim::Cell::new(crate::sim::material::STONE, 0));
        }
        let ant_x = 500;
        w.plant_ant(ant_x, 100);
        assert_eq!(w.creature_stats.spawned, 1, "no ant was planted -- the scene does not contain the subject");
        let interval = w
            .species
            .get(w.species.id_of("ant").expect("ant species"))
            .creature
            .as_ref()
            .expect("ant is a chain creature")
            .tick_interval;

        // **Six times the budget, and the multiplier is the whole design of
        // this test.** The flood has to leave far more than
        // `MAX_SITES_PER_FRAME` still *due* after the frame's pop, or the
        // frontier catches up and there is nothing to be starved by.
        //
        // How far behind a creature falls is `backlog / budget` frames,
        // because the heap drains oldest-first and the creature's own site
        // becomes the oldest once the frontier passes it — it is starved
        // *while* the backlog is deeper than the budget, not for ever. So
        // the multiplier chooses the severity, and it was measured rather
        // than guessed: at `* 2 + 500` the pooled arm reads 37 asks of 50
        // and a worst lateness of 2 frames, which is red but only just, and
        // an 8-ask margin over the bar is not a margin. At `* 6` it is
        // unambiguous. Both are conservative against what play produces —
        // `scale_probe load=ants:64,mine:20` on the shipped world measured
        // **late mean 5.8 frames, max 36** with the census showing zero
        // creature sites dispatched on whole frames.
        const FLOOD: usize = MAX_SITES_PER_FRAME * 6;
        const FRAMES: u64 = 300;
        for _ in 0..FRAMES {
            w.begin_step();
            // Re-flooded every frame, all due now, all west of the ant.
            // `y` is swept as well as `x` so the positions stay distinct
            // without ever reaching the ant's column.
            let due = w.frame;
            for i in 0..FLOOD {
                w.schedule_active_site(ActiveSite {
                    x: (i % 400) as i32,
                    y: 102 + (i / 400) as i32,
                    kind: ActiveKind::StructuralCheck,
                    next_frame: due,
                });
            }
            step(&mut w);
        }

        // The backlog has to be real, or this test proves nothing about
        // starvation -- it would merely be a slow way of running an ant.
        // `CLAUDE.md`: check that a guard's inputs actually vary what it
        // guards.
        assert!(
            w.active_site_count() > MAX_SITES_PER_FRAME,
            "the flood did not build a backlog ({} pending) -- the test is not exercising the condition it is named for",
            w.active_site_count()
        );

        let ideal = FRAMES / interval;
        let cs = w.creature_stats;
        assert!(
            cs.ticks * 10 >= ideal * 9,
            "the ant was asked {} times in {FRAMES} frames against the {ideal} its own tick_interval asks for -- \
             a backlog of background sites is starving the creature queue (late mean {:.1} frames, max {})",
            cs.ticks,
            if cs.ticks > 0 { cs.tick_lag_sum as f64 / cs.ticks as f64 } else { 0.0 },
            cs.tick_lag_max,
        );
        // Lateness is the same claim in the axis the complaint was made
        // in, and it is the one that reads as the owner's sentence: a
        // creature that is late by tens of frames is a creature that
        // pauses and then moves a pixel.
        //
        // **Zero is a proved property here, not a measured bar.** One ant
        // can never offer more than one due site a frame, so the reserve
        // cannot bind, so `pop_due_creature_site` takes it on the frame it
        // asked for. Asserting the property rather than a threshold is
        // what stops this drifting into a rubber stamp — `CLAUDE.md`,
        // assert the property, not two instants fitted to one trajectory.
        assert_eq!(
            cs.tick_lag_max, 0,
            "a creature site ran {} frames past its own next_frame with only one creature in the world -- \
             at tick_interval {interval} that is the pause the owner reported",
            cs.tick_lag_max
        );
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
