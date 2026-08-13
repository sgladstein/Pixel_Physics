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

    /// Current frame number — needed by `fire::tick_burn` to compute a
    /// newly-scheduled active site's `next_frame` (architecture §5f, ash
    /// decay). See `World::frame`.
    fn frame(&self) -> u64;

    /// Schedule a new M16 active site (`decay.rs`'s ash → soil check is the
    /// first caller reached from inside a generic CA rule, but the seam is
    /// general). Only `World` owns the active-site heap, so `ChunkView`
    /// queues this and replays it in `parallel::run_pass` — the same shape
    /// as `field_writes`/`light_writes`.
    fn schedule_active_site(&mut self, site: ActiveSite);

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
        // Gated on the *mover* being a `Powder`, unlike `flowing` above,
        // because unlike `flowing` this one is read from a cell the writer
        // does not own: `update_powder` tests the hole beneath a grain, and
        // that hole may perfectly well have been vacated by a liquid flowing
        // out from under the grain or a gas rising diagonally past it.
        // Leaving it ungated made either of those stall the sand above for a
        // frame -- a real coupling between kinds, not the "harmless for the
        // kinds that never read it" that holds for `flowing`. A registry
        // lookup in the hottest path in the engine, but only on the moves
        // that actually go sideways.
        let undercut = tx != fx && self.materials().kind(mover.material) == MaterialKind::Powder;
        let displaced = self.get(tx, ty).with_moved(!revisited).with_undercut(undercut);
        self.set(fx, fy, displaced);
        self.set(tx, ty, mover);
    }
}
