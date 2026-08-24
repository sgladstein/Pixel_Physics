//! `CellSurface`: the interface movement (`update.rs`) and fire (`fire.rs`)
//! rules read and write the world through, instead of a hardcoded `&mut
//! World`.
//!
//! Two implementers: `World` itself (`world.rs`) — thin delegation to its
//! existing methods, unchanged behaviour, used by every test and by the
//! single-threaded sweep — and `ChunkView` (`parallel.rs`) — the M5
//! multithreaded sweep's per-worker view, which applies writes inside its own
//! chunk immediately and defers anything that lands outside it. Because both
//! implementations run the exact same generic rule code, "identical
//! behaviour single- vs multi-threaded" is not something that needs proving
//! separately for every rule — there is only one rule implementation.
//!
//! # Why this can't just be `&mut World` with extra bookkeeping
//!
//! A trait, rather than a wrapper struct holding a `&mut World`, because the
//! parallel sweep's workers each need exclusive access to their *own* chunk
//! while the rest of the world stays shared and read-only — there is no
//! single `&mut World` to hand out per worker without violating aliasing.
//! `ChunkView` is what actually resolves that; this trait is just the
//! surface both paths present to the rules.

use super::cell::Cell;
use super::fire::PhaseEvent;
use super::material::{MaterialKind, MaterialRegistry};
use super::rng::Rng;
use super::scheduler::ActiveSite;

pub trait CellSurface {
    fn get(&self, x: i32, y: i32) -> Cell;
    fn set(&mut self, x: i32, y: i32, cell: Cell);
    fn in_bounds(&self, x: i32, y: i32) -> bool;

    /// Clear a cell's moved flag once the sweep has skipped it. Always called
    /// on the position currently being visited.
    fn clear_moved(&mut self, x: i32, y: i32);

    /// Clear a cell's undercut flag once the sweep has visited it. Always
    /// called on the position currently being visited. Separate from
    /// `clear_moved` rather than folded into it because the two are consumed
    /// at different moments: `moved` is consumed by the visit it *skips*,
    /// while `undercut` has to be dropped on any visit at all, including the
    /// overwhelmingly common one where the flagged cell is simply empty and
    /// the sweep does nothing else with it. Both are quiet writes — see
    /// `Chunk::set_world_quiet` for why neither may dirty its chunk.
    fn clear_undercut(&mut self, x: i32, y: i32);

    fn materials(&self) -> &MaterialRegistry;

    /// Movement tie-breaks, fire's ignition rolls, reaction chances. `World`
    /// hands out its single shared generator; `ChunkView` hands out its own
    /// chunk's — see `Chunk::rng` for why splitting the stream per chunk is
    /// what the parallel sweep needs.
    fn rng(&mut self) -> &mut Rng;

    /// Raise ambient field temperature in a filled circle around a cell —
    /// `fire::tick_burn`'s only caller. See `World::add_heat` for the general
    /// version; `ChunkView`'s implementation is the one that has to actually
    /// think about which field tile a write lands in.
    fn add_heat(&mut self, x: i32, y: i32, radius: i32, amount: f32);

    /// Raise ambient field light in a filled circle around a cell — the
    /// light-writer work from `Reports/emergent-world-architecture.md` §2,
    /// `fire::tick_burn`'s other caller alongside `add_heat`. Same shape,
    /// same reasoning, a separate method rather than a generalized
    /// `add_field(channel, ...)` — each channel's plumbing is small and
    /// mechanical enough that duplicating it stays cheaper than the
    /// abstraction, even with a moisture channel likely adding a third
    /// one soon. See `World::add_light` for the general version.
    fn add_light(&mut self, x: i32, y: i32, radius: i32, amount: f32);

    /// Ambient moisture at `(x, y)` — architecture §4's fire-resistance
    /// consumer, `try_ignite`'s only caller. A read, not a write, unlike
    /// `add_heat`/`add_light` above: `(x, y)` is always the cell currently
    /// being visited, which is always inside the caller's own chunk, so
    /// `ChunkView` can answer this from its own field tile without
    /// reaching into the shared `World` at all.
    fn field_moisture_at(&self, x: i32, y: i32) -> f32;

    /// **How wet the matter at and just below `(x, y)` is, `0..=1`** --
    /// `fire::try_ignite`'s moisture gate. See `field::ground_wetness_at`
    /// for why this is a different channel from `field_moisture_at` above
    /// and not a convenience wrapper on it: the humidity that one returns
    /// is identically zero at 96.8% of fuel cells at every ground wetness,
    /// because a field block containing a `Plant` cell is `blocked` and a
    /// blocked block never diffuses.
    ///
    /// Unlike `field_moisture_at`, this reads one field block *below* the
    /// visited cell as well as its own, so `ChunkView` cannot always
    /// answer it from its own tile -- a cell in the bottom eight rows of a
    /// chunk reaches into the next one down. It falls back to the shared
    /// `World` lookup there, which is affordable for the same reason
    /// `field_wind_at`'s own note gives about population size: this runs
    /// only for a flammable cell that has a burning neighbour, which is
    /// the fire front and nothing else, not once per visited cell.
    fn ground_wetness_at(&self, x: i32, y: i32) -> f32;

    /// Ambient wind (the field's own velocity) at `(x, y)`, as `(vx, vy)`.
    ///
    /// `update_gas`'s only caller. Same shape and same justification as
    /// `field_moisture_at` directly above — a read at the cell currently
    /// being visited, which is always inside the caller's own chunk, so
    /// `ChunkView` answers it from its own field tile without touching the
    /// shared `World`.
    ///
    /// **Why it is safe to read the field from inside a CA rule here, when
    /// `fire::diffuse_heat` had exactly this removed for cost.** Two
    /// reasons, and the second turned out to matter more than the first.
    ///
    /// The obvious one: `diffuse_heat` runs for *every visited cell*, on the
    /// order of 10⁵ a frame on a full-screen scene, where gas is a naturally
    /// tiny population (a blast leaves a few hundred smoke cells).
    ///
    /// The one that actually does the work: `diffuse_heat` called
    /// `World::field_at`, a **`HashMap` lookup per call**, which is what
    /// took its worst frame from ~16 ms to ~64 ms. The production CA sweep
    /// is `parallel::step`, so the implementation that runs in play is
    /// `ChunkView`'s — and that answers from the worker's *own* field tile
    /// by direct array index, with no hashing and no shared-`World` access
    /// at all. It is a fundamentally cheaper operation, not merely a rarer
    /// one.
    ///
    /// Measured rather than assumed, on a deliberately absurd scene —
    /// **56,640** gas cells, ~140x what a real blast produces, with a live
    /// pressure impulse over it so the read cannot be answered from a
    /// uniformly-zero grid. Mean frame across three runs each: 6.392 /
    /// 6.142 / 6.179 ms with the read, 6.455 / 6.218 / 6.157 ms without —
    /// indistinguishable. (A first attempt reported 73 ms against 30 ms and
    /// was entirely warm-up: the first ~20 frames of any scene here are
    /// dominated by allocation and first-touch page faults, which is worth
    /// remembering before believing any single worst-frame number in this
    /// repo.)
    fn field_wind_at(&self, x: i32, y: i32) -> (f32, f32);

    /// Current frame number — needed by `fire::tick_burn` to compute a
    /// newly-scheduled active site's `next_frame` (architecture §5f, ash
    /// decay). See `World::frame`.
    fn frame(&self) -> u64;

    /// An organism-schedule interval scaled by the world clock's
    /// `growth_slowdown`, as an absolute frame to be due on — `World::
    /// organism_due` reached through the trait.
    ///
    /// Exists because `fire::tick_burn` schedules a burnout's first ash-decay
    /// check, and decay rides the growth knob: litter and ash are *produced*
    /// per organism tick but weathered per real frame, so an unscaled decay
    /// leaves a slowed forest holding 1/N the standing litter. Computing it
    /// as `frame() + DECAY_TICK_INTERVAL` here would have quietly opted this
    /// one scheduling site out of that. See `sim::clock::Clock::
    /// growth_slowdown`.
    fn organism_due(&self, base_interval: u64) -> u64;

    /// Report that a denser cell just displaced near-full liquid at `(x, y)`
    /// with open air above it — a **candidate** splash site, not a splash.
    ///
    /// **The sweep reports and does not act, and that division is what makes
    /// the effect conservative.** A free particle lands as a whole cell
    /// (`particle::land`), so a droplet that was not taken out of the pool
    /// is water manufactured — and the removal and the launch therefore have
    /// to happen in the same place, or a harness that steps the world
    /// without owning a `ParticleSystem` (`examples/ascii.rs`, every unit
    /// test) would quietly drain every pool it splashed. Only
    /// `particle::throw_splashes` does both, together, and it re-checks the
    /// cell first because a site is a frame old by the time it runs.
    ///
    /// The other half of the division: this must not change movement, and it
    /// cannot — nothing here writes a cell. `Reports/open-bugs-handoff.md`
    /// §2 is about this exact code path and its striping is untouched.
    ///
    /// `World` pushes onto its own list, cleared at the top of every step so
    /// an undrained frame is discarded rather than accumulating;
    /// `ChunkView` queues and `run_pass` merges, the same shape as
    /// `schedule_active_site`.
    /// Report a splash candidate at `(x, y)`, with `strength` scaling how
    /// hard it is thrown.
    ///
    /// **A strength rather than one fixed throw**, because the two things
    /// that report splashes are not the same event: a boulder breaking the
    /// surface fans a crown, and a simmering pan spits a single drop that
    /// barely clears the water. Sharing the boulder's throw made the pan
    /// look like rain — the drops cleared ten rows.
    fn report_splash(&mut self, x: i32, y: i32, strength: f32);

    /// Record one temperature-triggered transition for the "did it fire at
    /// all" counters (`fire::PhaseCounts`, the `FailureCounts` pattern) —
    /// a steam plume and painted smoke are indistinguishable in a contact
    /// sheet, so whether the mechanism produced what is on screen has to be
    /// a count. `World` bumps `World::phase_changes` directly; `ChunkView`
    /// tallies privately and `run_pass` merges, the same queue-and-replay
    /// shape as `schedule_active_site`.
    fn count_phase_event(&mut self, event: PhaseEvent);

    /// Book meat destroyed by the sweep into `EnergyLedger::meat_lost` —
    /// `fire::tick_burn`'s burnout, the one destruction path that runs
    /// inside a CA rule rather than from a driver holding `&mut World`.
    ///
    /// Same queue-and-merge shape as `count_phase_event` directly above, and
    /// for the same reason: only `World` owns the ledger, so `ChunkView`
    /// tallies privately and `run_pass` merges. A worker adding into a shared
    /// `f64` would be a data race, and doing it under a lock would put a
    /// contended atomic on a CA rule.
    ///
    /// **An `f64` sum rather than a count**, unlike its neighbour: what is
    /// being lost is a *quantity* of energy and two corpses are rarely worth
    /// the same. Summing per chunk and adding the sums is exact for the
    /// f64 addition it replaces up to ordering, and the ordering is
    /// deterministic because `run_pass` merges chunks in a fixed order.
    fn book_meat_lost(&mut self, worth: f64);

    /// Whether `(x, y)` is above this column's frozen ground surface — the
    /// engine's stored definition of "outdoors" (`World::sky_surface`).
    ///
    /// Read by `fire::try_phase_change` to decide where a condensing gas's
    /// water goes: a plume in the open hands it to the sky
    /// (`MaterialDef::condenses_into_sky`), one sealed in a cave leaves a
    /// liquid cell where it stood. `Reports/open-bugs-handoff.md` §4b is
    /// why this is a stored answer and not a scan — every attempt to infer
    /// it from the shape of the world was wrong in a new way.
    ///
    /// One `Vec` index for `World`; `ChunkView` answers from the same slice
    /// through its own `&World`, so neither driver pays for a lookup.
    fn is_outdoors(&self, x: i32, y: i32) -> bool;

    /// Hand `fill` units of water (on `material::LIQUID_FULL`'s 0..1000
    /// scale) to `World::atmospheric_bank` — the same credit half of the
    /// outer cycle `evaporation::tick` uses, reached from inside the CA
    /// sweep instead.
    ///
    /// `World` credits immediately; `ChunkView` accumulates privately and
    /// `run_pass` merges, the same queue-and-replay shape as
    /// `count_phase_event`. The worker tally is in **whole fill units**
    /// rather than cell-equivalents so the merge is an integer sum and a
    /// pass cannot lose a fraction of a cell to float ordering.
    fn credit_atmosphere(&mut self, fill: u16);

    /// Schedule a new M16 active site (`decay.rs`'s ash → soil check is the
    /// first caller reached from inside a generic CA rule, but the seam is
    /// general). Only `World` owns the active-site heap, so `ChunkView`
    /// queues this and replays it in `parallel::run_pass` — the same shape
    /// as `field_writes`/`light_writes`.
    fn schedule_active_site(&mut self, site: ActiveSite);

    /// Record that something at `(x, y)` disturbed load-bearing material,
    /// licensing structural failures near it — see `World::chain_reach`
    /// and `World::record_disturbance`.
    ///
    /// Reachable from inside the sweep because a burnout and a phase
    /// change are both ways structural material appears or disappears
    /// without anyone touching it, and both are exactly the kind of event
    /// a collapse should be allowed to follow. Before this, only the three
    /// verbs that hold a `&mut World` (`rigid::strike`, `rigid::mine`,
    /// `explosion`) reported themselves, which was invisible while
    /// `chain_reach` defaulted to no limit and became "burn through a
    /// trunk and the tree hangs there" the moment `TIGHT` became the
    /// default.
    ///
    /// `extent` is the outer limit of the damage the verb does *itself*,
    /// as everywhere else — see `structural::Disturbance::extent`. Both
    /// callers reached from here are **per cell** (a burnout removes one,
    /// a phase change transforms one), so both pass `0` and let
    /// `World::record_disturbance`'s coalescing collect a burning region
    /// into a handful of records. A verb here that damages a *volume* must
    /// pass its real reach instead.
    ///
    /// Only `World` owns the ring, so `ChunkView` queues this and replays
    /// it in `parallel::run_pass` — the same shape as
    /// `schedule_active_site`.
    fn record_disturbance(&mut self, x: i32, y: i32, extent: i32);

    /// Absorb `fill` units into the promoted liquid body that owns
    /// `(x, y)` — `Reports/liquid-heightfield-design.md` §6b/§8b.
    /// `update_liquid`'s only caller: when a falling liquid cell's vertical
    /// transfer finds a `FLAG_MANAGED` cell of the same material below it,
    /// the source cell empties itself via the ordinary `set` and the whole
    /// amount is credited here in the same call, so the debit and credit
    /// can never be separated by a failure in between. A no-op if `(x, y)`
    /// doesn't resolve to a live body (should not happen given the caller's
    /// own `managed()` check, but not asserted — see `World::absorb_liquid`
    /// for why silently doing nothing is the right failure mode here). Only
    /// `World` owns `bodies`, so `ChunkView` queues this and replays it in
    /// `parallel::run_pass` — the same shape as `schedule_active_site`.
    fn absorb_liquid(&mut self, x: i32, y: i32, fill: u32);

    #[inline]
    fn is_empty(&self, x: i32, y: i32) -> bool {
        self.get(x, y).is_empty()
    }

    /// Move the cell at `(fx, fy)` to `(tx, ty)`, exchanging with whatever is
    /// already there. See `World::move_cell` for what `revisited` means —
    /// unchanged here, just expressed in terms of `get`/`set` so every
    /// implementer gets it for free rather than reimplementing the swap.
    ///
    /// Also marks the mover `Cell::flowing()` — every successful move sets
    /// it, not only a `Powder`'s, since this is the one seam every movement
    /// rule already goes through. Harmless for kinds that never read it
    /// (`Liquid`, `Gas`); `roll_along_slope` (`update.rs`) is the only
    /// reader, and only for `Powder` — see `FLAG_FLOWING`'s doc (`cell.rs`).
    #[inline]
    fn move_cell(&mut self, fx: i32, fy: i32, tx: i32, ty: i32, revisited: bool) {
        // `with_undercut(false)` on the mover, because the flag describes a
        // *vacancy* and never a grain: a cell that picked it up by being
        // displaced into one must not carry it along to wherever it goes
        // next. Only the two writes below can ever set it.
        let mover = self.get(fx, fy).with_moved(revisited).with_flowing(true).with_undercut(false);
        // The displaced cell travels the *opposite* direction to the mover,
        // so it needs the opposite `revisited` answer -- not an
        // unconditional `false`.
        //
        // `revisited` means "this cell landed in a row the sweep has not
        // reached yet, so skip it once or it will move again this frame."
        // Rows are swept bottom to top. When a denser cell moves *down*
        // (`revisited == false`), whatever it displaces goes *up*, into a
        // row still to come -- and clearing its flag let it be displaced
        // again by the next cell up, and again, once per row, so it
        // travelled the entire height of a falling body in a single frame.
        // Dropping a sand blob into water made the water surf the sweep
        // straight to the top of the blob and erupt out of it, instead of
        // being pushed up one cell at a time and flowing around the sides.
        // Reported from live play; the "one cell per frame" reading of this
        // line was wrong.
        //
        // `undercut` marks the vacancy this move leaves behind as one the
        // cell above may not simply drop into this frame -- see
        // `FLAG_UNDERCUT`'s own doc (`cell.rs`) for why a sideways escape and
        // a straight-down fall have to leave different kinds of hole.
        //
        // Gated on the *mover*'s kind, unlike `flowing` above, because
        // unlike `flowing` this one is read back from a cell the writer does
        // not own: `update_powder`/`update_liquid` test the hole beneath a
        // cell, and that hole may have been vacated by something of another
        // kind entirely. Leaving it ungated let a gas rising diagonally past
        // a sand pile stall the sand for a frame -- a real coupling between
        // kinds, not the "harmless for the kinds that never read it" that
        // holds for `flowing`.
        //
        // `Powder` and `Liquid` are exactly the two kinds that read it back,
        // because they are the two that pile against a free face, and both
        // were terracing on chunk seams for the same reason. A registry
        // lookup in the hottest path in the engine, but only on the moves
        // that actually go sideways.
        let undercut = tx != fx && matches!(self.materials().kind(mover.material), MaterialKind::Powder | MaterialKind::Liquid);
        let displaced = self.get(tx, ty).with_moved(!revisited).with_undercut(undercut);
        self.set(fx, fy, displaced);
        self.set(tx, ty, mover);
    }
}
