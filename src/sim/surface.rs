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
use super::material::MaterialRegistry;
use super::rng::Rng;

pub trait CellSurface {
    fn get(&self, x: i32, y: i32) -> Cell;
    fn set(&mut self, x: i32, y: i32, cell: Cell);
    fn in_bounds(&self, x: i32, y: i32) -> bool;

    /// Clear a cell's moved flag once the sweep has skipped it. Always called
    /// on the position currently being visited.
    fn clear_moved(&mut self, x: i32, y: i32);

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

    #[inline]
    fn is_empty(&self, x: i32, y: i32) -> bool {
        self.get(x, y).is_empty()
    }

    /// Move the cell at `(fx, fy)` to `(tx, ty)`, exchanging with whatever is
    /// already there. See `World::move_cell` for what `revisited` means —
    /// unchanged here, just expressed in terms of `get`/`set` so every
    /// implementer gets it for free rather than reimplementing the swap.
    #[inline]
    fn move_cell(&mut self, fx: i32, fy: i32, tx: i32, ty: i32, revisited: bool) {
        let mover = self.get(fx, fy).with_moved(revisited);
        let displaced = self.get(tx, ty).with_moved(false);
        self.set(fx, fy, displaced);
        self.set(tx, ty, mover);
    }
}
