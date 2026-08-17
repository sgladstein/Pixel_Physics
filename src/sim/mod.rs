//! The pixel simulation.
//!
//! Layering, innermost first:
//!
//! * `cell` — one simulated pixel, packed to 12 bytes (M12: material, shade,
//!   flags, temperature, aux, burn timer, organism id)
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
//!   distance to an anchor, recomputed reactively via `scheduler`. That
//!   distance is now purely a support-ordering potential — which way is
//!   downhill toward an anchor — not a failure criterion in itself
//! * `load` — the support forest that distance induces, and what each cell
//!   carries: a cell breaks when the bending moment of everything hanging
//!   off it exceeds what its section can hold. See
//!   `Reports/fracture-mechanics-design.md` for why reach was the wrong
//!   question
//! * `creature` — M18 Phase 1, cell-based creatures (a burrowing worm),
//!   dispatched from `scheduler` like plant growth and structural checks;
//!   fire/burning is deliberately not reimplemented here, since `fire.rs`
//!   already applies to every material kind uniformly
//! * `rigid` — M8, rigid bodies, deliberately started narrow: connected-
//!   component labeling only (the pipeline's first stage). Marching
//!   squares, Douglas-Peucker simplification, `earcutr` triangulation and
//!   the `rapier2d` collider/step/re-rasterize loop are not yet built —
//!   see the module doc and `PLAN.md`'s M8 section.
//! * `player` — M9, the summonable character: off-grid like a rigid body
//!   but input-driven, never settling, and a ghost to the CA (its own
//!   module doc has the trade-offs)
//!
//! Nothing below `update` knows about rendering, windowing or input.

pub mod cell;
pub mod chunk;
pub mod creature;
pub mod decay;
pub mod explosion;
pub mod field;
pub mod fire;
pub mod liquid;
pub mod load;
pub mod material;
pub mod organism;
pub mod parallel;
pub mod particle;
pub mod plant;
pub mod player;
pub mod rigid;
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
