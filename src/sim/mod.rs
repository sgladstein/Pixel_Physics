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
//!
//! Nothing below `update` knows about rendering, windowing or input.

pub mod cell;
pub mod chunk;
pub mod explosion;
pub mod field;
pub mod fire;
pub mod material;
pub mod particle;
pub mod rng;
pub mod update;
pub mod world;

pub use cell::Cell;
pub use chunk::{ChunkCoord, Rect, CHUNK_SIZE};
pub use field::FieldCell;
pub use material::{MaterialId, MaterialKind, MaterialRegistry};
pub use particle::{Particle, ParticleSystem};
pub use world::World;
