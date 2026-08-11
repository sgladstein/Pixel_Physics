//! The pixel simulation.
//!
//! Layering, innermost first:
//!
//! * `cell` — one simulated pixel, packed to 8 bytes (M12: material, shade,
//!   flags, temperature, kind-specific aux)
//! * `material` — what a cell *is*, as data rather than code
//! * `chunk` — 64x64 tiles plus the coordinate maths and dirty rectangles
//! * `field` — the coarse pressure/velocity/temperature/light grid (M13), one
//!   tile per chunk, its own frame phase separate from the CA sweep and not
//!   bound by `MAX_REACH`
//! * `world` — the sparse chunk map and the `get`/`set` seam
//! * `update` — the cellular automaton step
//! * `fire` — heat diffusion, ignition, burnout and phase changes (M14),
//!   called once per visited cell from inside the sweep
//! * `particle` — free (off-grid) particles for explosions and splashes
//!   (M7), a separate system from the CA grid entirely
//! * `explosion` — M15, built entirely from `field`, `fire` and `particle`
//!   triggered together; no new simulation primitive of its own
//! * `surface` — the `CellSurface` trait `update`/`fire`'s rules run
//!   against, implemented once for `World` (serial) and once for
//!   `parallel`'s `ChunkView` (multithreaded) — see M5
//! * `parallel` — M5, the multithreaded checkerboard sweep; an alternative
//!   driver for the exact same rules in `update`/`fire`, not a second copy
//!   of them
//! * `scheduler` — M16, the active-site list: a per-chunk schedule of
//!   growing plant tips, checked once per frame in its own phase separate
//!   from the CA sweep, at a cost proportional to how much is growing
//!   rather than to world size
//! * `plant` — M16, moss and tree/root growth, dispatched from `scheduler`
//! * `structural` — M17, destructible building: solid cells track their
//!   distance to an anchor, recomputed reactively via `scheduler`, and
//!   convert to loose material once that distance exceeds their material's
//!   tolerance
//!
//! Nothing below `update` knows about rendering, windowing or input.

pub mod cell;
pub mod chunk;
pub mod explosion;
pub mod field;
pub mod fire;
pub mod material;
pub mod parallel;
pub mod particle;
pub mod plant;
pub mod rng;
pub mod scheduler;
pub mod structural;
pub mod surface;
pub mod update;
pub mod world;

pub use cell::Cell;
pub use chunk::{ChunkCoord, Rect, CHUNK_SIZE};
pub use field::FieldCell;
pub use material::{MaterialId, MaterialKind, MaterialRegistry};
pub use particle::{Particle, ParticleSystem};
pub use world::World;
