//! The coarse field grid: pressure, velocity, ambient temperature and light.
//!
//! Modelled on [The Powder Toy's `Air.cpp`](https://github.com/The-Powder-Toy/The-Powder-Toy/blob/master/src/simulation/Air.cpp),
//! the one falling-sand engine with a real air simulation. Verified against
//! its source: pressure accumulates velocity divergence, velocity accumulates
//! the pressure gradient, walls zero velocity, and edges blend toward ambient.
//! `SimulationConfig.h` — where its exact tuning constants live — could not be
//! retrieved, so the coefficients here are tuned empirically for *this* grid's
//! scale and timestep against the stability test at the bottom of this file,
//! not copied numbers. The algorithmic structure is the part that transfers;
//! the constants never were going to.
//!
//! **One deliberate departure from the reference.** TPT updates its grid
//! in place, so later cells in a sweep see already-updated earlier ones
//! (Gauss-Seidel: faster convergence, order-dependent). This implementation
//! reads a full snapshot of the old state and writes a fresh new one every
//! pass (Jacobi: slightly more memory, but every pass is order-independent and
//! trivially parallelizable later — which matters, because M5 threads the CA
//! sweep the same way).
//!
//! # Resolution and ownership
//!
//! One [`FieldTile`] per [`Chunk`], at 1/8 the resolution — an 8x8 grid of
//! [`FieldCell`]s covering the chunk's 64x64 world cells. Tying tile lifetime
//! to chunk lifetime means no separate coordinate system and no separate
//! loading/unloading logic to keep in sync when M10 streaming arrives.
//!
//! # What this milestone does not do
//!
//! Cells do not yet exchange heat with the ambient field, and nothing emits
//! light — both are M14's job. M13 is scoped to the field grid's own internal
//! dynamics: does an impulse propagate and reflect, does a sealed room hold
//! pressure, does the whole thing stay bounded over time. Testing it without
//! coupling to the CA grid is deliberate; it isolates bugs in the field solver
//! from bugs in the coupling.

use std::collections::HashMap;

use super::cell::AMBIENT_TEMPERATURE;
use super::chunk::{ChunkCoord, Rect, CHUNK_SIZE};
use super::world::World;

/// World cells per field cell. A chunk is `CHUNK_SIZE / FIELD_SCALE` field
/// cells on a side. Coarser than the CA grid because pressure and light are
/// smooth, low-frequency fields — simulating them at CA resolution would cost
/// 64x the work for detail nothing reads.
pub const FIELD_SCALE: i32 = 8;

/// Field cells per side of one tile. `CHUNK_SIZE` must stay a multiple of
/// `FIELD_SCALE`, checked by `chunk_size_is_a_multiple_of_field_scale` below —
/// a chunk that didn't divide evenly would leave a partial row of field cells
/// with no consistent size.
pub const FIELD_TILE_SIZE: i32 = CHUNK_SIZE / FIELD_SCALE;
pub const FIELD_TILE_AREA: usize = (FIELD_TILE_SIZE * FIELD_TILE_SIZE) as usize;

// --- Tuning constants -------------------------------------------------------
//
// All tuned against `stays_bounded_over_ten_thousand_frames` below, which is
// the actual authority on whether these are safe — treat that test as the
// specification, and these comments as the reasoning behind the values it
// accepts.

/// Couples pressure and velocity: how strongly divergence drives pressure, and
/// pressure gradient drives velocity, per step. Analogous to TPT's
/// `AIR_TSTEPP` / `AIR_TSTEPV`, collapsed into one constant since this
/// implementation does not distinguish the two rates. Higher values propagate
/// disturbances faster but need more damping to stay stable; this value holds
/// with the damping below across the stability test.
const PRESSURE_VELOCITY_COUPLING: f32 = 0.15;

/// Multiplicative damping applied to velocity every step. Without this the
/// pressure/velocity exchange is only marginally stable — like an undamped
/// wave equation, discretization error accumulates and it rings forever
/// instead of settling. This is the one thing the reference doesn't need in
/// the same way, because Gauss-Seidel updates bleed energy through their own
/// order-dependence; the Jacobi scheme here has no equivalent free lunch, so
/// damping is explicit instead of incidental.
const VELOCITY_DAMPING: f32 = 0.97;

/// Same purpose as `VELOCITY_DAMPING`, weaker because pressure should persist
/// longer than the motion that created it — a sealed room holding pressure
/// after the initial disturbance settles is the whole point of simulating
/// pressure at all.
const PRESSURE_DAMPING: f32 = 0.995;

/// Heat diffusion rate per step. Explicit finite-difference diffusion is
/// unstable above a Fourier number of 0.5 in 1D and 0.25 in 2D (this is a 2D
/// grid). This sits well inside that margin — the CA-cell version in M14,
/// which runs at the fine grid and cannot afford as much margin, will need to
/// respect the same bound with less room to spare.
const HEAT_DIFFUSION_RATE: f32 = 0.2;

/// Light does not physically diffuse — it travels in straight lines and falls
/// off with distance and occlusion. Modelling that properly means casting
/// rays or solving a transport equation, and this is a coarse approximation
/// grid, not a renderer. Treating light as "diffuse fast, decay hard" is the
/// same shortcut Noita-likes generally take for ambient/bounce lighting: it
/// blurs outward from emitters and fades with distance, which looks
/// approximately right without being physically accurate. Revisit if M6's
/// rendering needs something better.
const LIGHT_DIFFUSION_RATE: f32 = 0.3;
const LIGHT_DECAY: f32 = 0.85;

/// How much of a cell's new value comes from sampling the old snapshot at the
/// back-traced position (transport) versus the locally computed value
/// (diffusion/pressure/velocity). 1.0 would be pure advection with no local
/// smoothing; 0.0 would never transport anything. This blend is what actually
/// carries smoke sideways on wind instead of only diffusing it in place.
const ADVECTION_BLEND: f32 = 0.6;

const MAX_PRESSURE: f32 = 256.0;
const MIN_PRESSURE: f32 = -256.0;
const MAX_TEMPERATURE: f32 = 4000.0;
/// Was `-MAX_TEMPERATURE` (i.e. -4000°C) until review pointed out that
/// permits temperatures roughly 14x past absolute zero (-273.15°C) — a
/// numerical safety clamp does not need to also be physically nonsensical.
/// Left with a little headroom above the true floor rather than pinned
/// exactly to it, since this is a coarse diffusion approximation, not a
/// thermodynamics engine, and something can legitimately overshoot slightly
/// during a single damped step without that being a bug worth chasing.
const MIN_TEMPERATURE: f32 = -270.0;
const MAX_LIGHT: f32 = 4.0;

/// One coarse cell: ambient conditions for an `FIELD_SCALE`-sided block of the
/// CA grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldCell {
    pub pressure: f32,
    pub vx: f32,
    pub vy: f32,
    /// Ambient temperature in Celsius. Distinct from `Cell::temperature` — see
    /// the module doc on `Cell` for why both exist.
    pub temperature: f32,
    pub light: f32,
}

impl FieldCell {
    /// Rest state: no pressure, no motion, room temperature, dark. What every
    /// field cell starts as and what an unloaded region reads as.
    pub const AMBIENT: FieldCell = FieldCell {
        pressure: 0.0,
        vx: 0.0,
        vy: 0.0,
        temperature: AMBIENT_TEMPERATURE as f32,
        light: 0.0,
    };
}

impl Default for FieldCell {
    fn default() -> Self {
        Self::AMBIENT
    }
}

/// The field data for one chunk: an 8x8 grid of [`FieldCell`], plus which of
/// those cells are blocked by CA-solid material.
#[derive(Clone)]
pub struct FieldTile {
    cells: Box<[FieldCell]>,
    /// Recomputed from the CA grid every step — see `rebuild_blocked`. Kept
    /// alongside the field cells rather than derived on demand during the
    /// solve, because the solve reads it many times per step and CA lookups
    /// are not free.
    blocked: Box<[bool]>,
}

impl FieldTile {
    /// `pub(crate)`: `World` creates one per chunk and needs to reach in for
    /// painting impulses, but the internal storage stays out of its hands.
    pub(crate) fn new() -> Self {
        Self {
            cells: vec![FieldCell::AMBIENT; FIELD_TILE_AREA].into_boxed_slice(),
            blocked: vec![false; FIELD_TILE_AREA].into_boxed_slice(),
        }
    }

    #[inline]
    fn local_index(lx: i32, ly: i32) -> usize {
        (ly * FIELD_TILE_SIZE + lx) as usize
    }

    #[inline]
    pub fn get_local(&self, lx: i32, ly: i32) -> FieldCell {
        self.cells[Self::local_index(lx, ly)]
    }

    #[inline]
    pub(crate) fn set_local(&mut self, lx: i32, ly: i32, cell: FieldCell) {
        self.cells[Self::local_index(lx, ly)] = cell;
    }

    #[inline]
    pub fn is_blocked_local(&self, lx: i32, ly: i32) -> bool {
        self.blocked[Self::local_index(lx, ly)]
    }

    #[inline]
    fn set_blocked_local(&mut self, lx: i32, ly: i32, blocked: bool) {
        self.blocked[Self::local_index(lx, ly)] = blocked;
    }
}

/// Field-cell address, analogous to `ChunkCoord` but one level coarser.
/// `pub(crate)`: `World::paint_field` needs the same conversion.
#[inline]
pub(crate) fn field_coord_of(world_x: i32, world_y: i32) -> (i32, i32) {
    (world_x.div_euclid(FIELD_SCALE), world_y.div_euclid(FIELD_SCALE))
}

/// Which chunk owns a field coordinate, and the field cell's local position
/// within that chunk's tile.
#[inline]
pub(crate) fn tile_and_local(field_x: i32, field_y: i32) -> (ChunkCoord, i32, i32) {
    let tile = ChunkCoord::new(
        field_x.div_euclid(FIELD_TILE_SIZE),
        field_y.div_euclid(FIELD_TILE_SIZE),
    );
    (tile, field_x.rem_euclid(FIELD_TILE_SIZE), field_y.rem_euclid(FIELD_TILE_SIZE))
}

/// Read a field cell from a snapshot by world-cell coordinate. Any world
/// coordinate inside the same `FIELD_SCALE` block reads the same cell. Out of
/// bounds and unloaded regions both read as ambient — the field has no
/// equivalent of the CA grid's solid out-of-bounds sentinel, because pressure
/// blocking is handled by `blocked`, not by what a plain read returns.
///
/// `pub(crate)`: `World::field_at` is the public read API; this is the shared
/// implementation it and the solve passes both call.
pub(crate) fn sample(tiles: &HashMap<ChunkCoord, FieldTile>, bounds: Option<Rect>, world_x: i32, world_y: i32) -> FieldCell {
    if let Some(b) = bounds {
        if !b.contains(world_x, world_y) {
            return FieldCell::AMBIENT;
        }
    }
    let (fx, fy) = field_coord_of(world_x, world_y);
    let (tile_coord, lx, ly) = tile_and_local(fx, fy);
    match tiles.get(&tile_coord) {
        Some(tile) => tile.get_local(lx, ly),
        None => FieldCell::AMBIENT,
    }
}

pub(crate) fn is_blocked(tiles: &HashMap<ChunkCoord, FieldTile>, bounds: Option<Rect>, world_x: i32, world_y: i32) -> bool {
    if let Some(b) = bounds {
        // The world edge is a wall — every CA rule already treats it as one
        // via `Cell::OUT_OF_BOUNDS`; the field must agree, or pressure would
        // leak through a boundary the particles themselves cannot cross.
        if !b.contains(world_x, world_y) {
            return true;
        }
    }
    let (fx, fy) = field_coord_of(world_x, world_y);
    let (tile_coord, lx, ly) = tile_and_local(fx, fy);
    match tiles.get(&tile_coord) {
        Some(tile) => tile.is_blocked_local(lx, ly),
        None => false,
    }
}

/// Bilinear sample at a fractional world position, for advection's
/// back-traced lookups. Falls back gracefully at the edges of loaded data
/// because `sample` already does — no special casing needed for that.
///
/// Blocked-corner handling is not automatic, though, and matters: a plain
/// bilinear sample reads whatever is stored at all four surrounding corners
/// with no idea that one of them might be on the far side of a wall from
/// where the trace started. `fallback` — the destination cell's own
/// pre-advection value — is substituted for any corner that is blocked,
/// rather than letting a wall's (usually near-ambient, but nonzero) contents
/// leak into the interpolation. TPT's reference implementation handles this
/// with a full raycast along the trace path, checking for a wall anywhere
/// along it, not just at the four sample corners; that is more correct for a
/// long trace that clips a thin wall between grid points without landing a
/// sample corner on it, and is deliberately not implemented here — a coarse
/// grid with the modest velocities this solver produces makes that gap small
/// in practice, and it is exactly the kind of thing worth revisiting if a
/// future milestone needs sharper containment than this gives.
fn sample_bilinear(
    tiles: &HashMap<ChunkCoord, FieldTile>,
    bounds: Option<Rect>,
    fx: f32,
    fy: f32,
    fallback: FieldCell,
) -> FieldCell {
    // `x0`/`y0` are the world coordinate of the *origin* (not center) of the
    // field cell containing `(fx, fy)`, and `tx`/`ty` are the fractional
    // position within that field cell's full `FIELD_SCALE`-wide span — not
    // within a single world unit. Getting this wrong is subtle and was
    // caught by review, not by any test: `fx.floor()` alone finds the right
    // *world* cell, and stepping by `FIELD_SCALE` from there does find the
    // correct pair of field cells to blend between, but the naive weight
    // `fx - fx.floor()` only ever spans 0..1 across a single world unit. It
    // would swing through its whole range within the first world-cell step
    // of any field cell and then sit pinned at whichever extreme for the
    // other `FIELD_SCALE - 1` world-cells — nowhere close to the smooth 0..1
    // ramp bilinear interpolation is supposed to produce across the actual
    // width of a field cell, and for small velocities (the common case) it
    // could hand nearly all the weight to the wrong side of the pair.
    let field_x0 = (fx / FIELD_SCALE as f32).floor() * FIELD_SCALE as f32;
    let field_y0 = (fy / FIELD_SCALE as f32).floor() * FIELD_SCALE as f32;
    let tx = (fx - field_x0) / FIELD_SCALE as f32;
    let ty = (fy - field_y0) / FIELD_SCALE as f32;
    let (x0, y0) = (field_x0 as i32, field_y0 as i32);

    let at = |wx: i32, wy: i32| {
        if is_blocked(tiles, bounds, wx, wy) {
            fallback
        } else {
            sample(tiles, bounds, wx, wy)
        }
    };
    let c00 = at(x0, y0);
    let c10 = at(x0 + FIELD_SCALE, y0);
    let c01 = at(x0, y0 + FIELD_SCALE);
    let c11 = at(x0 + FIELD_SCALE, y0 + FIELD_SCALE);

    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let mix = |a: FieldCell, b: FieldCell, t: f32| FieldCell {
        pressure: lerp(a.pressure, b.pressure, t),
        vx: lerp(a.vx, b.vx, t),
        vy: lerp(a.vy, b.vy, t),
        temperature: lerp(a.temperature, b.temperature, t),
        light: lerp(a.light, b.light, t),
    };
    mix(mix(c00, c10, tx), mix(c01, c11, tx), ty)
}

/// Advance the field grid by one step. A whole-grid pass, run once per frame
/// as its own phase — separate from the CA sweep, and **not** bound by
/// `MAX_REACH`: that limit exists because CA dirty rectangles only widen by so
/// much, and a rule reading further acts on cells that never wake it. This
/// pass reads everything every step regardless of what changed, so no such
/// staleness is possible; it is the mechanism by which a shockwave can cross
/// the whole screen without violating the invariant that governs the CA sweep.
pub fn step(world: &mut World) {
    let coords: Vec<ChunkCoord> = world.chunks().map(|c| c.coord).collect();
    let bounds = world.bounds();

    // Old state stays untouched in `world` until the very end, so every phase
    // below can read it through `sample`/`is_blocked` while writing into
    // `next` — no aliasing, because nothing is shared.
    let mut next: HashMap<ChunkCoord, FieldTile> = HashMap::with_capacity(coords.len());
    for &coord in &coords {
        next.insert(coord, FieldTile::new());
    }

    rebuild_blocked(world, &coords, &mut next);
    step_pressure(world, &coords, &mut next);
    step_velocity(world, &coords, &mut next);
    step_diffusion(world, &coords, &mut next);
    step_advection(&coords, bounds, &mut next);

    world.replace_fields(next);
}

/// A field cell counts as blocked when any CA-solid cell falls inside its
/// `FIELD_SCALE`-sided block. Biased toward over-blocking rather than
/// under-blocking: a field cell that is mostly open but partly solid still
/// stops air passing straight through, and the alternative (a fractional
/// "how solid" value feeding into partial blocking) is real hydraulic
/// modelling this coarse a grid was never trying to do.
fn rebuild_blocked(world: &World, coords: &[ChunkCoord], next: &mut HashMap<ChunkCoord, FieldTile>) {
    for &coord in coords {
        // Fetched once per chunk instead of once per *CA cell scanned*
        // (previously up to `FIELD_TILE_SIZE^2 * FIELD_SCALE^2` = 4096
        // `World::get` calls per chunk, each a bounds check plus a
        // `HashMap<ChunkCoord, Chunk>` lookup). `coords` comes from
        // `world.chunks()` (see `step` above), so every entry is
        // guaranteed resident -- this can never be `None`. `Chunk::get_world`
        // still takes global coordinates and does its own local-index
        // conversion, so nothing below needs to change.
        let chunk = world.chunk(coord).expect("coord came from world.chunks(), so it must be resident");
        // Hoisted out of the `ly`/`lx` loops too (issue #6): the tile
        // pointer is invariant across every field cell in this chunk, but
        // was previously looked up fresh on every single one of them.
        let tile = next.get_mut(&coord).expect("next was pre-populated with every coord in coords");
        let (ox, oy) = coord.origin();
        for ly in 0..FIELD_TILE_SIZE {
            for lx in 0..FIELD_TILE_SIZE {
                let bx0 = ox + lx * FIELD_SCALE;
                let by0 = oy + ly * FIELD_SCALE;
                let mut blocked = false;
                'scan: for dy in 0..FIELD_SCALE {
                    for dx in 0..FIELD_SCALE {
                        // `World::new` eagerly creates every chunk
                        // overlapping `bounds` (`ensure_chunks_for`), so a
                        // chunk's own 64x64 span can extend past a world
                        // whose size isn't a multiple of `CHUNK_SIZE` (the
                        // sandbox itself is 512x320 -- fine -- but
                        // `plant.rs`/`creature.rs`'s own 200x200 test
                        // worlds are not). `Chunk::get_world` has no
                        // concept of world bounds at all, unlike
                        // `World::get` (which this replaced for the
                        // resident-cell case, but must not silently drop
                        // this check along with it) -- an independent
                        // review caught that the out-of-world sliver of
                        // such a chunk read as *not* blocked here, though
                        // it stayed a currently-inert bug (every actual
                        // consumer of field data re-checks bounds itself
                        // via `sample`/`is_blocked`) rather than a visible
                        // one. Treated the same as `Cell::OUT_OF_BOUNDS`
                        // reads elsewhere: solid, so blocked.
                        if !world.in_bounds(bx0 + dx, by0 + dy) {
                            blocked = true;
                            break 'scan;
                        }
                        let cell = chunk.get_world(bx0 + dx, by0 + dy);
                        // `Plant` blocks too, not just `Solid` -- a tree
                        // trunk or canopy is exactly as solid as a rock
                        // wall for this purpose. Missing this reopened the
                        // bug this function's own doc already recounts
                        // fixing once for `Solid` alone: light/heat/
                        // pressure passing straight through as if the
                        // material were transparent air, which for plants
                        // specifically undermines M16's own moss mechanic
                        // (shade under a canopy reading as no shade at all).
                        // `Creature` (M18) deliberately does *not* block --
                        // a single mobile worm cell isn't a wall the way a
                        // stationary structure is, and the field only
                        // resolves at one field cell per `FIELD_SCALE`
                        // world cells regardless, so one moving creature
                        // blocking a whole field tile would be a coarser,
                        // wronger approximation than just letting it pass.
                        let kind = world.materials.kind(cell.material);
                        if matches!(kind, super::material::MaterialKind::Solid | super::material::MaterialKind::Plant) {
                            blocked = true;
                            break 'scan;
                        }
                    }
                }
                tile.set_blocked_local(lx, ly, blocked);
            }
        }
    }
}

/// `pv += divergence(velocity) * coupling`, damped. Matches Air.cpp's
/// pressure-from-velocity step: inflow from the left/top and outflow to the
/// right/bottom raises pressure, and vice versa.
fn step_pressure(world: &World, coords: &[ChunkCoord], next: &mut HashMap<ChunkCoord, FieldTile>) {
    let old = world.fields_ref();
    let bounds = world.bounds();
    for &coord in coords {
        let (ox, oy) = coord.origin();
        // Hoisted out of the `ly`/`lx` loops (issue #6): this pointer is
        // invariant across every field cell in the chunk. A `&mut` still
        // supports the read-only `is_blocked_local` call below, so this
        // also replaces the separate `next.get(&coord)` lookup that used
        // to run once per field cell right before it.
        let tile = next.get_mut(&coord).unwrap();
        for ly in 0..FIELD_TILE_SIZE {
            for lx in 0..FIELD_TILE_SIZE {
                let (wx, wy) = (ox + lx * FIELD_SCALE, oy + ly * FIELD_SCALE);
                // The *current* frame's occupancy, not `old`'s. `rebuild_blocked`
                // already wrote this frame's accurate blocked map into `next`
                // before this function runs; checking `old` here would be
                // reading last step's occupancy for a decision this step is
                // making, one frame stale exactly when a CA solid has just
                // moved. Caught by review, not by any test — every existing
                // wall test keeps the CA grid static for the whole run, which
                // can't distinguish "this step's map" from "last step's".
                if tile.is_blocked_local(lx, ly) {
                    continue; // stays ambient — a wall cell has no pressure of its own
                }
                let here = sample(old, bounds, wx, wy);
                let left = sample(old, bounds, wx - FIELD_SCALE, wy);
                let right = sample(old, bounds, wx + FIELD_SCALE, wy);
                let up = sample(old, bounds, wx, wy - FIELD_SCALE);
                let down = sample(old, bounds, wx, wy + FIELD_SCALE);

                let divergence = (left.vx - right.vx) + (up.vy - down.vy);
                let mut pressure = (here.pressure + divergence * PRESSURE_VELOCITY_COUPLING) * PRESSURE_DAMPING;
                pressure = pressure.clamp(MIN_PRESSURE, MAX_PRESSURE);

                let mut cell = tile.get_local(lx, ly);
                cell.pressure = pressure;
                tile.set_local(lx, ly, cell);
            }
        }
    }
}

/// `v += -gradient(pressure) * coupling`, damped, reflected off walls. Reads
/// the pressure `step_pressure` just wrote into `next` — the deliberate
/// pressure-then-velocity coupling order that makes disturbances propagate
/// instead of the two fields updating independently of each other.
fn step_velocity(world: &World, coords: &[ChunkCoord], next: &mut HashMap<ChunkCoord, FieldTile>) {
    let old = world.fields_ref();
    let bounds = world.bounds();
    // Read the just-computed pressure from `next` as an immutable snapshot
    // before mutating `next` further. A separate cloned table sidesteps
    // borrowing `next` both immutably (for pressure) and mutably (to write
    // velocity) at once.
    let new_pressure: HashMap<ChunkCoord, FieldTile> =
        coords.iter().map(|&c| (c, next.get(&c).unwrap().clone())).collect();

    for &coord in coords {
        let (ox, oy) = coord.origin();
        let tile = next.get_mut(&coord).unwrap(); // hoisted (issue #6) -- invariant across the ly/lx loops below
        for ly in 0..FIELD_TILE_SIZE {
            for lx in 0..FIELD_TILE_SIZE {
                let (wx, wy) = (ox + lx * FIELD_SCALE, oy + ly * FIELD_SCALE);
                let mut cell = tile.get_local(lx, ly);

                if is_blocked(&new_pressure, bounds, wx, wy) {
                    cell.vx = 0.0;
                    cell.vy = 0.0;
                    tile.set_local(lx, ly, cell);
                    continue;
                }

                let old_here = sample(old, bounds, wx, wy);
                let p_left = sample(&new_pressure, bounds, wx - FIELD_SCALE, wy);
                let p_right = sample(&new_pressure, bounds, wx + FIELD_SCALE, wy);
                let p_up = sample(&new_pressure, bounds, wx, wy - FIELD_SCALE);
                let p_down = sample(&new_pressure, bounds, wx, wy + FIELD_SCALE);

                let mut vx = (old_here.vx + (p_left.pressure - p_right.pressure) * PRESSURE_VELOCITY_COUPLING)
                    * VELOCITY_DAMPING;
                let mut vy = (old_here.vy + (p_up.pressure - p_down.pressure) * PRESSURE_VELOCITY_COUPLING)
                    * VELOCITY_DAMPING;

                // No-penetration: only the component actually flowing *into*
                // a wall is stopped, and it is clamped to zero, not reflected.
                //
                // Two things worth recording about how this line was reached.
                // First, zeroing whichever axis merely *touched* a blocked
                // neighbour regardless of direction — the original version —
                // was a real bug, not a cosmetic one: in a small sealed room
                // almost every interior cell borders some wall, so that
                // version force-zeroed velocity there on nearly every step
                // no matter which way it was flowing, bleeding energy out of
                // a sealed room *faster* than open ground, which never
                // triggers the check at all. Measured backwards: a sealed
                // room was retaining less total pressure than open ground.
                //
                // Second, the fix was not "reflect it instead" (flip the
                // sign to bounce, conserving the kinetic energy in that
                // component). That was tried, and it made the same
                // measurement *worse* (0.99 vs the zeroing bug's 44), most
                // likely energy pooling at wall-adjacent cells and hitting
                // the pressure/temperature clamps repeatedly, which are
                // themselves lossy. Reflection is a billiard-ball model,
                // appropriate for discrete particles; the textbook boundary
                // condition for a continuum velocity *field* like this one is
                // no-penetration — the normal component goes to zero at the
                // wall, full stop, nothing bounces. That is what is here now.
                let flowing_into_blocked_left = vx < 0.0 && is_blocked(&new_pressure, bounds, wx - FIELD_SCALE, wy);
                let flowing_into_blocked_right = vx > 0.0 && is_blocked(&new_pressure, bounds, wx + FIELD_SCALE, wy);
                if flowing_into_blocked_left || flowing_into_blocked_right {
                    vx = 0.0;
                }
                let flowing_into_blocked_up = vy < 0.0 && is_blocked(&new_pressure, bounds, wx, wy - FIELD_SCALE);
                let flowing_into_blocked_down = vy > 0.0 && is_blocked(&new_pressure, bounds, wx, wy + FIELD_SCALE);
                if flowing_into_blocked_up || flowing_into_blocked_down {
                    vy = 0.0;
                }

                cell.vx = vx;
                cell.vy = vy;
                tile.set_local(lx, ly, cell);
            }
        }
    }
}

/// Explicit diffusion for temperature and light, each cell relaxing toward
/// the average of its four neighbours. `HEAT_DIFFUSION_RATE` and
/// `LIGHT_DIFFUSION_RATE` are both held under the 2D stability bound.
///
/// Wall-aware, unlike an earlier version of this function that diffused
/// straight through blocked cells as though they were transparent — caught by
/// review, since every other phase (pressure, velocity, advection) treats a
/// wall as a hard stop and this one alone did not. A sealed stone room
/// insulating its interior at all, thermally or optically, depended on this:
/// without it, heat and light passed through a wall exactly as freely as
/// through open air, which defeats the entire point of `is_blocked` existing.
fn step_diffusion(world: &World, coords: &[ChunkCoord], next: &mut HashMap<ChunkCoord, FieldTile>) {
    let old = world.fields_ref();
    let bounds = world.bounds();
    for &coord in coords {
        let (ox, oy) = coord.origin();
        let tile = next.get_mut(&coord).unwrap(); // hoisted (issue #6), also replaces the per-cell next.get(&coord) below
        for ly in 0..FIELD_TILE_SIZE {
            for lx in 0..FIELD_TILE_SIZE {
                let (wx, wy) = (ox + lx * FIELD_SCALE, oy + ly * FIELD_SCALE);
                // Same-frame occupancy, for the same reason `step_pressure`
                // needs it — `next`, not `old`.
                if tile.is_blocked_local(lx, ly) {
                    continue; // stays ambient — a wall has no temperature or light of its own
                }
                let here = sample(old, bounds, wx, wy);

                // A blocked neighbour contributes `here`'s own value instead
                // of whatever the wall cell happens to hold, so it pulls the
                // average toward "no change" rather than toward the wall's
                // contents — the same fallback idea `sample_bilinear` uses
                // for advection, applied here to diffusion's neighbour read.
                let neighbour = |dx: i32, dy: i32| {
                    let (nx, ny) = (wx + dx, wy + dy);
                    if is_blocked(old, bounds, nx, ny) {
                        here
                    } else {
                        sample(old, bounds, nx, ny)
                    }
                };
                let left = neighbour(-FIELD_SCALE, 0);
                let right = neighbour(FIELD_SCALE, 0);
                let up = neighbour(0, -FIELD_SCALE);
                let down = neighbour(0, FIELD_SCALE);
                let neighbour_avg_t = (left.temperature + right.temperature + up.temperature + down.temperature) / 4.0;
                let neighbour_avg_l = (left.light + right.light + up.light + down.light) / 4.0;

                let temperature = (here.temperature
                    + (neighbour_avg_t - here.temperature) * HEAT_DIFFUSION_RATE)
                    .clamp(MIN_TEMPERATURE, MAX_TEMPERATURE);
                let light = ((here.light + (neighbour_avg_l - here.light) * LIGHT_DIFFUSION_RATE) * LIGHT_DECAY)
                    .clamp(0.0, MAX_LIGHT);

                let mut cell = tile.get_local(lx, ly);
                cell.temperature = temperature;
                cell.light = light;
                tile.set_local(lx, ly, cell);
            }
        }
    }
}

/// Semi-Lagrangian advection: trace each cell backward along the velocity
/// `next` now holds, sample the *old* snapshot there, and blend it in. This
/// is what makes wind actually carry smoke and heat sideways rather than only
/// diffusing outward symmetrically in place.
fn step_advection(coords: &[ChunkCoord], bounds: Option<Rect>, next: &mut HashMap<ChunkCoord, FieldTile>) {
    // Snapshot `next` as it stands after pressure/velocity/diffusion, so the
    // sampling below reads a fixed pre-advection state rather than a mix of
    // advected and not-yet-advected cells depending on iteration order.
    let pre_advection: HashMap<ChunkCoord, FieldTile> =
        coords.iter().map(|&c| (c, next.get(&c).unwrap().clone())).collect();

    for &coord in coords {
        let (ox, oy) = coord.origin();
        let tile = next.get_mut(&coord).unwrap(); // hoisted (issue #6) -- invariant across the ly/lx loops below
        for ly in 0..FIELD_TILE_SIZE {
            for lx in 0..FIELD_TILE_SIZE {
                let (wx, wy) = (ox + lx * FIELD_SCALE, oy + ly * FIELD_SCALE);
                if is_blocked(&pre_advection, bounds, wx, wy) {
                    continue;
                }
                let here = sample(&pre_advection, bounds, wx, wy);

                let src_x = wx as f32 - here.vx * FIELD_SCALE as f32;
                let src_y = wy as f32 - here.vy * FIELD_SCALE as f32;
                let transported = sample_bilinear(&pre_advection, bounds, src_x, src_y, here);

                let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
                let blended = FieldCell {
                    pressure: lerp(here.pressure, transported.pressure, ADVECTION_BLEND),
                    vx: lerp(here.vx, transported.vx, ADVECTION_BLEND),
                    vy: lerp(here.vy, transported.vy, ADVECTION_BLEND),
                    temperature: lerp(here.temperature, transported.temperature, ADVECTION_BLEND),
                    light: lerp(here.light, transported.light, ADVECTION_BLEND),
                };

                tile.set_local(lx, ly, blended);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::cell;
    use crate::sim::cell::Cell;
    use crate::sim::material;

    #[test]
    fn chunk_size_is_a_multiple_of_field_scale() {
        // A remainder here would leave a ragged edge of field cells with no
        // consistent size, breaking every piece of local-index math above.
        assert_eq!(CHUNK_SIZE % FIELD_SCALE, 0);
    }

    #[test]
    fn field_coord_floors_toward_negative_infinity() {
        // The same truncating-division trap as ChunkCoord::containing.
        assert_eq!(field_coord_of(0, 0), (0, 0));
        assert_eq!(field_coord_of(7, 7), (0, 0));
        assert_eq!(field_coord_of(8, 8), (1, 1));
        assert_eq!(field_coord_of(-1, -1), (-1, -1));
        assert_eq!(field_coord_of(-8, -8), (-1, -1));
        assert_eq!(field_coord_of(-9, -9), (-2, -2));
    }

    #[test]
    fn tile_and_local_round_trips_across_chunk_boundaries() {
        // Field coordinate 7 (last cell of chunk 0's tile) and 8 (first cell
        // of chunk 1's tile) must land in different chunks.
        let (t0, ..) = tile_and_local(7, 0);
        let (t1, ..) = tile_and_local(8, 0);
        assert_ne!(t0, t1);
        assert_eq!(t0, ChunkCoord::new(0, 0));
        assert_eq!(t1, ChunkCoord::new(1, 0));
    }

    fn test_world() -> World {
        World::new(Rect::new(0, 0, 255, 255))
    }

    #[test]
    fn an_undisturbed_field_stays_at_ambient() {
        let mut w = test_world();
        for _ in 0..50 {
            step(&mut w);
        }
        let c = w.field_at(128, 128);
        assert_eq!(c.pressure, 0.0);
        assert_eq!(c.vx, 0.0);
        assert_eq!(c.vy, 0.0);
    }

    #[test]
    fn a_pressure_impulse_propagates_outward() {
        let mut w = test_world();
        w.add_pressure_impulse(128, 128, 4, 100.0);

        let before = w.field_at(128, 128).pressure;
        assert!(before > 0.0, "impulse did not raise local pressure");

        for _ in 0..5 {
            step(&mut w);
        }

        // Pressure must have spread to a point well outside the impulse
        // radius that started at zero.
        let spread = w.field_at(128 + 40, 128).pressure;
        assert!(spread.abs() > 0.001, "pressure never reached x+40: {spread}");
    }

    #[test]
    fn a_sealed_room_holds_pressure_better_than_open_ground() {
        // Total |pressure| over a region, not a single point at a single
        // instant: a sealed cavity sloshes, so a one-point one-frame reading
        // can land on a trough and look emptier than the open case even while
        // holding strictly more energy overall. Summing over the room's
        // interior area is insensitive to where the standing wave's peaks
        // currently are.
        fn total_abs_pressure(w: &World, cx: i32, cy: i32, half_extent: i32) -> f32 {
            let mut total = 0.0;
            let mut y = cy - half_extent;
            while y <= cy + half_extent {
                let mut x = cx - half_extent;
                while x <= cx + half_extent {
                    total += w.field_at(x, y).pressure.abs();
                    x += super::FIELD_SCALE;
                }
                y += super::FIELD_SCALE;
            }
            total
        }

        let mut w = test_world();
        // A closed box of stone around the impulse point.
        for x in 96..160 {
            for y in [96, 159] {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for y in 96..160 {
            for x in [96, 159] {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        w.add_pressure_impulse(128, 128, 4, 100.0);
        for _ in 0..200 {
            step(&mut w);
        }
        let sealed_total = total_abs_pressure(&w, 128, 128, 28);

        let mut open = test_world();
        open.add_pressure_impulse(128, 128, 4, 100.0);
        for _ in 0..200 {
            step(&mut open);
        }
        let open_total = total_abs_pressure(&open, 128, 128, 28);

        assert!(
            sealed_total > open_total,
            "sealed room ({sealed_total}) should retain more total pressure than open ground ({open_total})"
        );
    }

    #[test]
    fn walls_zero_the_velocity_that_would_cross_them() {
        let mut w = test_world();
        for y in 90..170 {
            w.set(130, y, Cell::new(material::STONE, 0));
        }
        w.add_pressure_impulse(100, 128, 4, 200.0);
        for _ in 0..50 {
            step(&mut w);
        }
        // Immediately past the wall, air must not be flowing rightward
        // through solid stone.
        let past_wall = w.field_at(136, 128);
        assert!(
            past_wall.vx.abs() < 0.01,
            "velocity leaked through a wall: vx = {}",
            past_wall.vx
        );
    }

    #[test]
    fn light_diffuses_from_a_source_and_decays_with_distance() {
        let mut w = test_world();
        w.add_light(128, 128, 2, 1.0);
        for _ in 0..10 {
            step(&mut w);
        }
        let near = w.field_at(128, 128).light;
        let far = w.field_at(128, 200).light;
        assert!(near > 0.0, "light source went dark");
        assert!(far < near, "light did not fall off with distance");
    }

    #[test]
    fn heat_does_not_leak_through_a_sealed_wall_via_diffusion() {
        // Regression for a bug caught by independent review: step_diffusion
        // originally had no wall-awareness at all, unlike every other phase,
        // so a sealed room provided zero thermal insulation — heat diffused
        // straight through solid stone as if it were open air.
        let mut w = test_world();
        for x in 96..160 {
            for y in [96, 159] {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for y in 96..160 {
            for x in [96, 159] {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        w.add_heat(128, 128, 4, 500.0);
        for _ in 0..300 {
            step(&mut w);
        }

        let inside = w.field_at(128, 128).temperature;
        let outside = w.field_at(128, 40).temperature;
        assert!(
            inside > cell::AMBIENT_TEMPERATURE as f32 + 5.0,
            "heat source cooled to near-ambient even inside its own sealed room: {inside}"
        );
        assert!(
            (outside - cell::AMBIENT_TEMPERATURE as f32).abs() < 1.0,
            "heat leaked through a sealed wall via diffusion: outside reads {outside}, ambient is {}",
            cell::AMBIENT_TEMPERATURE
        );
    }

    #[test]
    fn pressure_stops_at_a_wall_the_same_frame_it_appears() {
        // Regression: step_pressure originally checked blocked status against
        // `old` (the previous field step's occupancy) instead of the
        // just-rebuilt `next`, so a CA solid placed mid-run took one extra
        // field step to actually start blocking pressure.
        //
        // The wall is placed *before* anything disturbs the field, and the
        // impulse is added right at it in the same step blocking is supposed
        // to take effect — no pre-existing velocity anywhere for the wall to
        // race against. An earlier version of this test built up velocity
        // over 20 steps *before* placing the wall, which made the far side
        // already flowing outward on its own account by the time the wall
        // appeared — a confound that could fail this assertion for a reason
        // having nothing to do with the bug being guarded against here.
        let mut w = test_world();
        for y in 90..170 {
            w.set(130, y, Cell::new(material::STONE, 0));
        }
        w.add_pressure_impulse(124, 128, 2, 200.0); // right against the wall's near side

        step(&mut w); // the single step the bug would fail on
        let just_past = w.field_at(136, 128);
        assert!(
            just_past.vx.abs() < 0.01,
            "velocity crossed a wall on the very frame pressure first appeared next to it: vx = {}",
            just_past.vx
        );
    }

    #[test]
    fn advection_weight_varies_smoothly_across_a_field_cells_full_width() {
        // Regression: sample_bilinear's interpolation weight was computed as
        // `fx - fx.floor()`, which only spans 0..1 across a single WORLD
        // unit rather than across a field cell's full FIELD_SCALE-wide span
        // — so the weight would swing through its entire range within the
        // first world-cell of any field cell and then sit pinned at an
        // extreme for the rest, instead of ramping smoothly. This sets up two
        // field cells with very different pressure and checks that a source
        // position a quarter of the way across the boundary cell reads much
        // closer to the near cell's value than the far one's — which the
        // buggy version would get backwards for small offsets like this.
        let mut w = test_world();
        w.add_pressure_impulse(64, 64, 1, 100.0); // field cell containing (64,64)
        for _ in 0..3 {
            step(&mut w);
        }
        let near = w.field_at(64, 64).pressure;
        let far = w.field_at(64 + FIELD_SCALE, 64).pressure;
        assert!(
            (near - far).abs() > 1.0,
            "test setup needs the two cells to actually differ: near={near}, far={far}"
        );

        // A quarter of the way from the near cell toward the far one should
        // read much closer to `near` than to `far`. `fields_ref` is
        // `pub(crate)` on `World` for exactly this kind of direct access from
        // this module's own tests.
        let quarter = sample_bilinear(
            w.fields_ref(),
            w.bounds(),
            64.0 + FIELD_SCALE as f32 * 0.25,
            64.0,
            FieldCell::AMBIENT,
        );
        let dist_to_near = (quarter.pressure - near).abs();
        let dist_to_far = (quarter.pressure - far).abs();
        assert!(
            dist_to_near < dist_to_far,
            "a sample 25% of the way across should read closer to the near \
             cell ({near}) than the far one ({far}), got {} \
             (dist to near {dist_to_near}, dist to far {dist_to_far})",
            quarter.pressure
        );
    }

    #[test]
    fn stays_bounded_over_ten_thousand_frames() {
        // The actual authority on whether the tuning constants above are
        // safe. An unstable scheme grows without bound; this would fail by
        // producing NaN or a value outside the clamps, not by a clean panic.
        //
        // Checks several points, not just the impulse's own center — review
        // pointed out that a single-point check could miss a divergence
        // localized elsewhere (e.g. right at the wall, where the boundary
        // condition does extra work every step). A full-grid scan every 500
        // steps would also work but costs much more for the same coverage
        // this sparse a set of representative points already gives.
        let mut w = test_world();
        w.add_pressure_impulse(128, 128, 4, 200.0);
        w.set(150, 128, Cell::new(material::STONE, 0)); // give it something to reflect off

        let watch_points = [
            (128, 128), // the impulse's own center
            (144, 128), // between the impulse and the wall
            (152, 128), // just past the wall — should stay near ambient
            (128, 90),  // off-axis, away from the wall entirely
            (0, 0),     // a far corner
        ];

        for i in 0..10_000 {
            step(&mut w);
            if i % 500 == 0 {
                for &(x, y) in &watch_points {
                    let c = w.field_at(x, y);
                    assert!(c.pressure.is_finite(), "pressure diverged at ({x},{y}), step {i}");
                    assert!(c.vx.is_finite() && c.vy.is_finite(), "velocity diverged at ({x},{y}), step {i}");
                    assert!(c.temperature.is_finite(), "temperature diverged at ({x},{y}), step {i}");
                    assert!(
                        c.pressure.abs() <= MAX_PRESSURE,
                        "pressure escaped its clamp at ({x},{y}), step {i}: {}",
                        c.pressure
                    );
                }
            }
        }
    }

    #[test]
    fn field_step_does_not_move_ca_material() {
        // M13 is scoped to the field's own dynamics. Coupling the field back
        // into CA-cell movement is explicitly out of scope until M14/M15.
        let mut w = test_world();
        w.set(128, 128, Cell::new(material::SAND, 0));
        w.add_pressure_impulse(128, 128, 4, 200.0);
        for _ in 0..20 {
            step(&mut w);
        }
        assert_eq!(w.get(128, 128).material, material::SAND, "field step moved a CA cell");
    }

    #[test]
    fn a_chunks_out_of_world_sliver_reads_as_blocked_in_a_non_aligned_world() {
        // Regression: an independent review of the issue #5 perf fix
        // (rebuild_blocked switched from World::get, which is bounds-aware,
        // to Chunk::get_world, which is not) caught that the out-of-world
        // slice of a chunk whose 64x64 span extends past a non-64-aligned
        // world (the sandbox's own 512x320 happens to divide evenly, but
        // plant.rs/creature.rs's 200x200 test worlds -- and any future
        // arbitrary size -- don't) silently stopped reading as blocked.
        //
        // World::field_is_blocked itself can't detect this: it goes through
        // `is_blocked`, which re-checks world bounds *before* ever
        // consulting the stored tile, so the wrong stored value is masked
        // for every real caller. This test deliberately reaches past that
        // mask, through the same crate-internal seam `rebuild_blocked`
        // itself writes through (`World::fields_ref` -> the raw
        // `FieldTile`), to check what actually got stored -- the thing the
        // bug was actually in, not the thing that happened to hide it.
        let mut w = World::new(Rect::new(0, 0, 199, 199)); // 200 is not a multiple of CHUNK_SIZE (64)
        step(&mut w);

        // World x/y 200 is one cell past the world's own right/bottom edge
        // (max valid is 199), but still inside chunk (3, 3)'s 64-cell span
        // (192..256) -- exactly the sliver the bug affected.
        let (fx, fy) = field_coord_of(200, 200);
        let (coord, lx, ly) = tile_and_local(fx, fy);
        let tile = w.fields_ref().get(&coord).expect("the chunk covering (200, 200) should be resident");
        assert!(tile.is_blocked_local(lx, ly), "a field cell entirely outside the world's own bounds should read as blocked");
    }
}
