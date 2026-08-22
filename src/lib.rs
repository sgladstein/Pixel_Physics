//! A pixel physics engine.
//!
//! The engine is a library so that the sandbox binary is just one consumer of
//! it. Games built on top become separate binaries against the same crate,
//! rather than forks of a monolithic executable.
//!
//! * [`sim`] — the cellular automaton: cells, materials, chunks, the world
//! * [`worldgen`] — what fills a world before the simulation touches it
//! * [`sky`] — what empty space looks like through a day
//! * [`render`] — turning cells into pixels
//! * [`app`] — sandbox state: brush, material picker, starting terrain
//! * [`perf`] — measurement hygiene for the headless timing harnesses
//!
//! `sim` depends on nothing above it, which is what keeps the simulation
//! testable without a window or a GPU. `worldgen` sits above `sim` and below
//! everything else, for the same reason: a generated world has to be
//! buildable headlessly.

pub mod app;
pub mod hud;
pub mod perf;
pub mod render;
pub mod sky;
pub mod sim;
pub mod tunables;
pub mod worldgen;
