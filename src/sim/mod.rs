//! The pixel simulation.
//!
//! Layering, innermost first:
//!
//! * `cell`     — one simulated pixel, packed to 4 bytes
//! * `material` — what a cell *is*, as data rather than code
//! * `chunk`    — 64x64 tiles plus the coordinate maths and dirty rectangles
//! * `world`    — the sparse chunk map and the `get`/`set` seam
//! * `update`   — the cellular automaton step
//!
//! Nothing below `update` knows about rendering, windowing or input.

pub mod cell;
pub mod chunk;
pub mod material;
pub mod rng;
pub mod update;
pub mod world;

pub use cell::Cell;
pub use chunk::{ChunkCoord, Rect, CHUNK_SIZE};
pub use material::{MaterialId, MaterialKind, MaterialRegistry};
pub use world::World;
