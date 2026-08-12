//! A pixel physics engine.
//!
//! The engine is a library so that the sandbox binary is just one consumer of
//! it. Games built on top become separate binaries against the same crate,
//! rather than forks of a monolithic executable.
//!
//! * [`sim`] — the cellular automaton: cells, materials, chunks, the world
//! * [`render`] — turning cells into pixels
//! * [`app`] — sandbox state: brush, material picker, starting terrain
//!
//! `sim` depends on nothing above it, which is what keeps the simulation
//! testable without a window or a GPU.

pub mod app;
pub mod hud;
pub mod render;
pub mod sim;
