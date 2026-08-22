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

use std::collections::{HashMap, HashSet};

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
/// grid, not a renderer. Treating light as diffusion-with-decay is the same
/// shortcut Noita-likes generally take for ambient/bounce lighting: it blurs
/// outward from emitters and fades with distance, which looks approximately
/// right without being physically accurate. Revisit if M6's rendering needs
/// something better.
const LIGHT_DIFFUSION_RATE: f32 = 0.3;
/// Per-step decay on the **diffusive** light component only.
///
/// **Read this before reaching for it to change how bright the world is:
/// direct sunlight does not pass through here at all any more.**
/// `apply_sky` casts sun down each column and takes a `max`, so daylight's
/// reach is set by occlusion (`FieldTile::occupancy`) and never by this
/// constant. What decays here is the *bounce*: lateral bleed that softens
/// a canopy's shadow instead of leaving a hard stencil edge at field
/// resolution, and point sources like fire.
///
/// So the tuning question this constant answers is "how far does light
/// spill sideways from where it lands", and the answer wanted is *not
/// far* — a glow, not a second sun. 0.95 gives a handful of blocks.
///
/// ## The history, because it is the part that keeps being re-derived
///
/// This constant used to carry sunlight, and was pushed 0.85 → 0.997 →
/// 0.9997 chasing the consequences. Everything below is why it is not
/// carrying it now, and why moving it back would be a mistake:
///
/// - Air attenuating sunlight is not a thing air does, but a
///   diffusion-with-decay model has no way to say so: at 0.997 an
///   *empty* world read 4.00 at the surface and 0.16 at depth 128,
///   crossing `Germinate`'s 0.1 gate in vacuum. Illumination was a
///   function of distance from the world's top edge.
/// - That made height its own reward, so every scene ended with its trees
///   pinned against the boundary, and it made scene depth a cliff rather
///   than a curve — no ground depth was both well-lit and un-ceilinged.
/// - The value was then outcome-justified at 0.9997 on a 25x tree-size
///   difference, honestly measured and still treating the symptom.
///
/// `apply_sky`'s column cast (`Reports/tree-architecture-implementation-
/// plan.md` §0f) fixed the cause: **clear air does not attenuate sunlight,
/// occluders do.** With sunlight off this channel the old value was doing
/// nothing but spreading every point source across the whole world, and
/// 0.95 — close to the original local-glow reading — is what a bounce
/// term should be. The three recorded side effects of the 0.9997 era went
/// with it: the germination gate is reachable again, the deep field no
/// longer inverts for half of every day/night cycle, and caves stay dark.
///
/// One live symptom of that era survives and is *not* this constant's
/// doing: `Germinate`'s gate is degenerate in the other direction now,
/// because powders and liquids are transparent to `occupancy`, so a
/// buried seed still passes it. See `rebuild_blocked`.
const LIGHT_DECAY: f32 = 0.95;

/// Humidity spreads through air more readily than it evaporates away, unlike
/// light — a much larger diffusion rate than `LIGHT_DIFFUSION_RATE`, still
/// comfortably under the 2D stability bound. Kept well below 1.0; see
/// `HEAT_DIFFUSION_RATE`'s own comment for why that ceiling exists.
const MOISTURE_DIFFUSION_RATE: f32 = 0.4;
/// Base per-step evaporation, applied before the temperature-scaled term
/// below. Gentler than `LIGHT_DECAY`: real humidity lingers over minutes,
/// not the near-instant falloff a point light source gets away with.
const MOISTURE_BASE_DECAY: f32 = 0.97;
/// Extra evaporation per degree above ambient, on top of the base rate —
/// the "decay/evaporation modulated by temperature" extra loop
/// `Reports/emergent-world-architecture.md` §4 calls out: humidity now
/// evaporates faster near fire and, once §5h's day/night oscillator drives
/// temperature, faster at midday than at night, for free. Untuned against
/// anything real, same as every other constant in this file — the actual
/// authority is `stays_bounded_over_ten_thousand_frames` and the scene-based
/// checks, not this number in isolation.
const MOISTURE_EVAPORATION_PER_DEGREE: f32 = 0.0002;

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
pub const MAX_LIGHT: f32 = 4.0;
/// Architecture report §4. No physical unit — a relative "how much ambient
/// humidity" scalar, same spirit as `MAX_LIGHT`: not calibrated against a
/// real quantity, just a fixed ceiling the diffusion/decay constants below
/// are tuned against.
const MAX_MOISTURE: f32 = 4.0;

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
    /// Ambient humidity — architecture report §4. Sourced from `Liquid` CA
    /// cells (`apply_moisture_sources`), diffuses and evaporates like every
    /// other channel here. Replaces the two hand-rolled O(r²) grid scans
    /// `is_damp`/`strongest_water_pull` used to do this privately, per cell,
    /// with one shared field every consumer reads.
    pub moisture: f32,
}

impl FieldCell {
    /// Rest state: no pressure, no motion, room temperature, dark, dry. What
    /// every field cell starts as and what an unloaded region reads as.
    pub const AMBIENT: FieldCell = FieldCell {
        pressure: 0.0,
        vx: 0.0,
        vy: 0.0,
        temperature: AMBIENT_TEMPERATURE as f32,
        light: 0.0,
        moisture: 0.0,
    };
}

impl Default for FieldCell {
    fn default() -> Self {
        Self::AMBIENT
    }
}

/// The field data for one chunk: an 8x8 grid of [`FieldCell`], plus which of
/// those cells are blocked by CA-solid material and which contain a
/// `Liquid` CA cell.
#[derive(Clone)]
pub struct FieldTile {
    cells: Box<[FieldCell]>,
    /// Recomputed from the CA grid every step — see `rebuild_blocked`. Kept
    /// alongside the field cells rather than derived on demand during the
    /// solve, because the solve reads it many times per step and CA lookups
    /// are not free.
    blocked: Box<[bool]>,
    /// **What fraction of a downward ray this block passes**, quantized to
    /// 0..=255, recomputed in the same scan as `blocked`.
    ///
    /// `blocked` is all-or-nothing over an 8x8 region, and that is far too
    /// coarse for foliage. One twig anywhere in a block marks the whole
    /// block opaque, so a tree canopy reads as a solid wall: measured, 88%
    /// of all leaf cells in a grown stand sat below 0.05 light, and there
    /// was no threshold anywhere between "prunes nothing" and "defoliates
    /// the world". Two mechanisms are stuck behind that — shade-driven leaf
    /// abscission, and phototropism, which measured completely inert across
    /// 1,024 genomes because there is no gradient left to steer by.
    ///
    /// **Per column, not per cell count, and the difference is the whole
    /// point of this field.** It stored *occupancy* — cells filled over
    /// `FIELD_SCALE²` — which is orientation-blind, and wrong in exactly
    /// the direction of the artifact the light model is supposed to bound.
    /// A vertical 8-cell trunk in one column and a horizontal 8-cell plate
    /// spanning all eight both read 8/64, so both passed 90% of the light.
    /// For a *downward* ray only the horizontal extent matters: the trunk
    /// answer was right by luck and the plate answer was 4.5x too bright,
    /// and a flat canopy plate is the exact geometry this branch has fought
    /// three separate times.
    ///
    /// So: count opacity **depth per CA column**, read each column's
    /// transmission off `COLUMN_TRANSMISSION` (Beer-Lambert, normalised so
    /// a full `FIELD_SCALE`-deep column lands exactly on
    /// `SKY_TRANSMISSION`), and store the mean across the block's columns.
    /// A sparse spray of twigs passes most of the light, a horizontal plate
    /// shades what is under it, and solid rock still passes exactly
    /// `SKY_TRANSMISSION` — same as before, since a full block is full in
    /// every column.
    ///
    /// **Not a columns-hit mask**, which was the other candidate: counting
    /// *whether* a column is touched makes any one-cell-deep structure as
    /// opaque as rock, which is the binary-shade disease this field exists
    /// to cure, rotated ninety degrees. Depth is what Beer-Lambert reads.
    ///
    /// Costs one `u8` per field block and no extra scanning —
    /// `rebuild_blocked` already visits every CA cell in the block.
    transmission: Box<[u8]>,
    /// Also recomputed every step, in the same scan as `blocked` — whether
    /// any `Liquid` CA cell falls inside this field block. `apply_moisture_
    /// sources` reads this at the end of `step` (mirroring `apply_sky`'s use
    /// of `blocked`) to force the moisture channel back up to `MAX_MOISTURE`
    /// wherever it's still true, the same "recompute the source condition
    /// fresh every active frame, don't try to track it incrementally"
    /// approach `blocked` itself already uses.
    /// **A level, 0..1, not a flag.** Standing liquid sources at 1.0 as it
    /// always did; damp *soil* sources in proportion to how damp it is.
    ///
    /// Without this, per-cell soil moisture was invisible to everything but
    /// the roots drinking it: `is_damp` (moss) and `moisture_pull` (root
    /// hydrotropism) both read this field, and this field only knew about
    /// standing water. Roots steered toward puddles and ignored moist
    /// ground, and moss would not grow on damp earth. Grading the source is
    /// what closes the loop `Reports/plant-substrate-v2-design.md` §4d
    /// describes — infiltrate, hold, drink, deplete, *and be noticed*.
    moisture_source: Box<[f32]>,
    /// Light emitted by the cells of each block — `Material::glow`, maxed
    /// over the block in the same scan `blocked`/`moisture_source` already
    /// run (`rebuild_blocked`). Recomputed per solve like both of those,
    /// never inherited: a sleeping tile keeps its last solve, and digging
    /// out the crystal re-solves the tile and the glow with it. Seeded
    /// into the light channel *before* diffusion, so a glowing lining gets
    /// a soft halo for free and then converges — a static floor, exactly
    /// so the tile can sleep lit (the owner's local-light decision,
    /// 2026-08; the whole design is in `Material::glow`'s doc).
    glow: Box<[f32]>,
    /// Whether any block of this tile glows — the renderer's cheap gate
    /// for "is it worth sampling the field under this pixel".
    pub has_glow: bool,
    /// A lower bound on moisture that evaporation may not take a cell below.
    ///
    /// The saturated ground beneath the water table. Worldgen writes it once
    /// (`worldgen::passes::moisture_init`) and it does not change again,
    /// which makes it categorically different from every other array on this
    /// tile: `blocked` and `moisture_source` are *recomputed from the CA
    /// grid* every step, whereas this is **authored** and has no CA state to
    /// be recomputed from. That is why `step` has to carry it forward
    /// explicitly rather than getting it for free — see the copy in `step`.
    ///
    /// It exists because the aquifer cannot be liquid cells. A cell holds one
    /// material and there is no porosity, so "saturated rock" is rock whose
    /// *field* says it is wet; without a floor, evaporation would dry the
    /// deep world out within a few hundred frames and the water table would
    /// quietly cease to exist. Moisture above the floor still decays
    /// normally, so surface humidity, puddles and rain all behave as before —
    /// this is the hybrid persistence `Reports/worldgen-design.md` §8 asks
    /// for, not a second moisture channel.
    moisture_floor: Box<[f32]>,
    /// Whether every cell of this tile came out of its last solve with
    /// pressure, `vx` and `vy` at **exactly** zero.
    ///
    /// What makes skipping the momentum passes bit-identical rather than
    /// merely close. Those three passes are geometric decays — with no
    /// divergence, `pressure * PRESSURE_DAMPING` — so once a tile reaches
    /// exact zero they can only ever write zero again, and not running them
    /// is a provable no-op. Before that point they are still doing real work,
    /// however settled the tile looks: measured, skipping them the moment a
    /// tile went *settled* left a gale's residue frozen at 0.32 pressure
    /// units forever instead of decaying away, against a settle epsilon of
    /// 0.01. Settled means "changing slowly", and slowly is not never.
    momentum_zero: bool,
    /// Whether the four CA-derived arrays above hold a real scan.
    ///
    /// **Not the same question as "is there a previous tile", and the gap
    /// between the two is a real bug this cost.** `World::ensure_chunks_for`
    /// eagerly inserts a `FieldTile::new()` for every chunk it creates, so a
    /// previous tile always exists from the very first frame -- holding
    /// `blocked` all false and `transmission` all clear, because nothing has
    /// looked at the CA grid yet. `inherit_derived`'s first version keyed on
    /// `previous.is_some()` and so carried that blank scan forward for any
    /// chunk that was already settled when the field first stepped, which is
    /// every hand-built test scene: `a_settled_glow_does_not_rebuild_its_
    /// halo_every_frame` failed with "the scene never built a halo at all",
    /// because the spar's `glow` was never scanned and `has_glow` never went
    /// true. A whole-field hash over 3,600 frames of the `rolling` preset did
    /// *not* catch it, because worldgen leaves every chunk dirty and the
    /// first solves all rescanned.
    derived_valid: bool,
    /// Whether this tile converged on the last solve — see [`Self::settled`].
    settled: bool,
    /// Whether the sky walk reached this tile with any light left.
    sky_lit: bool,
    /// The sky amplitude in force the last time this tile was *solved*.
    ///
    /// Compared against the current amplitude to decide whether a lit tile
    /// may keep sleeping. Comparing consecutive *frames* instead — which is
    /// what the global `amplitude_changed` does — cannot work here:
    /// `SETTLE_EPSILON_LIGHT` is 0.005 and the sun moves at most ~0.0033 per
    /// frame, so that test is essentially never true and the day/night cycle
    /// actually advances by sub-epsilon drift the whole-world solve happened
    /// to re-notice every frame. Measuring drift since the tile last solved
    /// accumulates instead of vanishing, so the surface wakes every few
    /// frames rather than every frame or never.
    sky_amplitude: f32,
}

impl FieldTile {
    /// `pub(crate)`: `World` creates one per chunk and needs to reach in for
    /// painting impulses, but the internal storage stays out of its hands.
    pub(crate) fn new() -> Self {
        Self {
            cells: vec![FieldCell::AMBIENT; FIELD_TILE_AREA].into_boxed_slice(),
            blocked: vec![false; FIELD_TILE_AREA].into_boxed_slice(),
            // 255 is *clear*: a fresh tile passes light until the scan says
            // otherwise. Zero would mean "perfectly opaque", which is the
            // wrong default for a block nobody has looked at yet.
            transmission: vec![u8::MAX; FIELD_TILE_AREA].into_boxed_slice(),
            moisture_source: vec![0.0; FIELD_TILE_AREA].into_boxed_slice(),
            moisture_floor: vec![0.0; FIELD_TILE_AREA].into_boxed_slice(),
            glow: vec![0.0; FIELD_TILE_AREA].into_boxed_slice(),
            has_glow: false,
            // False, so a tile nobody has scanned is always scanned rather
            // than carried. See the field's own doc.
            derived_valid: false,
            // A fresh tile is all `AMBIENT`, which *is* zero momentum — but
            // claiming so before anything has looked at it would let the
            // first frame skip a pass it has no evidence about. Earned, not
            // assumed.
            momentum_zero: false,
            // A tile nobody has solved yet has not converged, by definition.
            settled: false,
            sky_lit: false,
            sky_amplitude: f32::NAN,
        }
    }

    /// Whether this tile reached its own fixed point on the last solve.
    ///
    /// The per-tile half of what used to be `World::fields_settled` alone.
    /// That single bool made `field::step` all-or-nothing: one active chunk
    /// anywhere ran all seven passes over *every* resident chunk, which is
    /// O(world) work for O(1) of activity — measured at 53 ms for one
    /// four-cell impulse on a 2048x1280 world, against 2.5 ms for the same
    /// impulse at 512x320. The global flag stays as a cheap fast path; this
    /// is what lets the slow path solve only what is actually moving.
    #[inline]
    pub fn settled(&self) -> bool {
        self.settled
    }

    #[inline]
    pub(crate) fn set_settled(&mut self, settled: bool) {
        self.settled = settled;
    }

    /// Whether a lit tile has drifted far enough from the sky it last solved
    /// against to need solving again. `NaN` (never solved) always wakes.
    #[inline]
    fn sky_drifted(&self, amplitude: f32) -> bool {
        if !self.sky_lit {
            return false;
        }
        // `is_nan` explicitly rather than a negated comparison: a tile that has
        // never solved carries `NaN`, and every ordinary comparison against it
        // is false, so it must be spelled out or a fresh tile would read as
        // "already in step with the sky".
        let drift = (amplitude - self.sky_amplitude).abs();
        drift.is_nan() || drift > SETTLE_EPSILON_LIGHT
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

    /// Fraction of a downward ray this block passes, 0.0..=1.0. See
    /// `FieldTile::transmission`.
    #[inline]
    pub fn transmission_local(&self, lx: i32, ly: i32) -> f32 {
        self.transmission[Self::local_index(lx, ly)] as f32 / 255.0
    }

    fn set_transmission_local(&mut self, lx: i32, ly: i32, transmission: f32) {
        self.transmission[Self::local_index(lx, ly)] = (transmission.clamp(0.0, 1.0) * 255.0).round() as u8;
    }

    fn set_blocked_local(&mut self, lx: i32, ly: i32, blocked: bool) {
        self.blocked[Self::local_index(lx, ly)] = blocked;
    }

    #[inline]
    fn moisture_source_local(&self, lx: i32, ly: i32) -> f32 {
        self.moisture_source[Self::local_index(lx, ly)]
    }

    #[inline]
    fn glow_local(&self, lx: i32, ly: i32) -> f32 {
        self.glow[Self::local_index(lx, ly)]
    }

    #[inline]
    fn set_glow_local(&mut self, lx: i32, ly: i32, glow: f32) {
        self.glow[Self::local_index(lx, ly)] = glow;
        if glow > 0.0 {
            self.has_glow = true;
        }
    }

    fn set_moisture_source_local(&mut self, lx: i32, ly: i32, source: f32) {
        self.moisture_source[Self::local_index(lx, ly)] = source;
    }

    #[inline]
    pub fn moisture_floor_local(&self, lx: i32, ly: i32) -> f32 {
        self.moisture_floor[Self::local_index(lx, ly)]
    }

    #[inline]
    pub(crate) fn set_moisture_floor_local(&mut self, lx: i32, ly: i32, floor: f32) {
        self.moisture_floor[Self::local_index(lx, ly)] = floor.clamp(0.0, MAX_MOISTURE);
    }

    /// Carry the authored floor across into a freshly built tile.
    ///
    /// `step` rebuilds every tile from scratch each frame, which is right for
    /// the two arrays derived from the CA grid and wrong for this one: it has
    /// no source to be derived from, so without this the floor would survive
    /// exactly one frame and the aquifer would evaporate.
    fn inherit_moisture_floor(&mut self, previous: &FieldTile) {
        self.moisture_floor.copy_from_slice(&previous.moisture_floor);
    }

    /// Carry the four CA-derived arrays across instead of rescanning for
    /// them — see the caller in `step` for when that is sound.
    ///
    /// The doc on `moisture_floor` above says these arrays are "recomputed
    /// from the CA grid every step, whereas this is authored", and treats
    /// that as the reason only the floor needs carrying. True as far as it
    /// goes, and it quietly assumes the recompute is *necessary*, which it
    /// is only when the CA grid has actually changed. Measured at 8192x2560:
    /// `rebuild_blocked` was 29.5 ms of a 59 ms sky-step frame and 11.2 of
    /// the 34 ms frame after it, every one of those tiles sitting over a
    /// chunk that was asleep. An awake *tile* is not an awake *chunk*: the
    /// sun wakes a tile over rock nothing has touched in ten thousand
    /// frames, and the old code rescanned all 4096 of its CA cells, with a
    /// material-registry lookup each, to rederive four arrays that could not
    /// have changed.
    ///
    /// `has_glow` is copied rather than left at its `false` default. It is
    /// deliberately reset each solve in `rebuild_blocked` because
    /// `set_glow_local` can only ever latch it on; on this path there is no
    /// scan to re-raise it, and the previous tile's answer is already the
    /// right one.
    /// Carry the three momentum channels across for a tile the sun woke.
    ///
    /// Pressure and velocity only — **not** temperature, light or moisture,
    /// which `step_diffusion` still writes for these tiles from `old`-based
    /// reads, exactly as it always did. And emphatically not light: the whole
    /// reason a solved tile is rebuilt fresh is that `apply_sky_to` only ever
    /// max-writes, so a carried-forward light channel can rise with the dawn
    /// and never fall again. That freezes the world at noon, which is the
    /// reverted `apply_sky_to` column subset's failure wearing the opposite
    /// sign.
    fn inherit_momentum(&mut self, previous: &FieldTile) {
        for i in 0..FIELD_TILE_AREA {
            self.cells[i].pressure = previous.cells[i].pressure;
            self.cells[i].vx = previous.cells[i].vx;
            self.cells[i].vy = previous.cells[i].vy;
        }
        self.momentum_zero = previous.momentum_zero;
    }

    /// Declare this tile's momentum channels live again, for a writer that
    /// reaches past the solver — see `World::paint_field`.
    pub(crate) fn disturb_momentum(&mut self) {
        self.momentum_zero = false;
    }

    /// Recorded after the momentum passes have written, so the next frame can
    /// tell whether running them again could change anything.
    fn record_momentum_zero(&mut self) {
        self.momentum_zero =
            self.cells.iter().all(|c| c.pressure == 0.0 && c.vx == 0.0 && c.vy == 0.0);
    }

    fn inherit_derived(&mut self, previous: &FieldTile) {
        self.blocked.copy_from_slice(&previous.blocked);
        self.transmission.copy_from_slice(&previous.transmission);
        self.moisture_source.copy_from_slice(&previous.moisture_source);
        self.glow.copy_from_slice(&previous.glow);
        self.has_glow = previous.has_glow;
        self.derived_valid = true;
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

/// Whether the field block covering this position holds standing liquid or
/// damp soil — the same flag `rebuild_blocked` records and
/// `apply_moisture_sources` forces humidity from.
///
/// Exists so `step_diffusion` can let the moisture channel read *through* a
/// blocked block, which every other channel must not do. Out of bounds is
/// `false`: the void is not wet.
/// How strongly the field block covering this position sources moisture,
/// `0.0..=1.0` — `1.0` for standing liquid, graded for damp soil, `0.0` for
/// anything dry. The value `apply_moisture_sources` forces humidity from.
///
/// **Recomputed from the CA grid every frame and untouched by any solve
/// pass**, which is the property `evaporation.rs` needs from it: unlike the
/// humidity it produces, it cannot be advected away by wind.
pub(crate) fn moisture_source_at(
    tiles: &HashMap<ChunkCoord, FieldTile>,
    bounds: Option<Rect>,
    world_x: i32,
    world_y: i32,
) -> f32 {
    if let Some(b) = bounds {
        if !b.contains(world_x, world_y) {
            return 0.0;
        }
    }
    let (fx, fy) = field_coord_of(world_x, world_y);
    let (tile_coord, lx, ly) = tile_and_local(fx, fy);
    tiles.get(&tile_coord).map_or(0.0, |tile| tile.moisture_source_local(lx, ly))
}

pub(crate) fn is_moisture_source(
    tiles: &HashMap<ChunkCoord, FieldTile>,
    bounds: Option<Rect>,
    world_x: i32,
    world_y: i32,
) -> bool {
    if let Some(b) = bounds {
        if !b.contains(world_x, world_y) {
            return false;
        }
    }
    let (fx, fy) = field_coord_of(world_x, world_y);
    let (tile_coord, lx, ly) = tile_and_local(fx, fy);
    tiles.get(&tile_coord).is_some_and(|tile| tile.moisture_source_local(lx, ly) > 0.0)
}

/// `(is_blocked, glows)` in a single tile fetch, for `step_diffusion`'s
/// neighbour read — the diffusion hot path, where asking the two questions
/// through separate functions cost a second `HashMap` fetch of the same
/// tile for every blocked neighbour (the first version did, and the paired
/// `ascii` river scene showed the within-run spring cost rising ~0.5 ms at
/// 2048x640 for it). `has_glow` short-circuits the per-block array read
/// for the overwhelmingly common tile with no glow anywhere.
///
/// The glow half exists for exactly the reason `is_moisture_source` does:
/// `blocked` goes true for a whole 8x8 block the moment one cell in it is
/// `Solid`, and a glowing lining is by definition solid, so the generic
/// wall rule in `step_diffusion` read a lit lining's block as "contributes
/// my own value" and the seed `rebuild_blocked` writes was exactly as
/// unshareable as the wet rock basin's moisture once was — a lit block
/// over a pitch-dark cavity, no halo at all. Found by the paired-cavity
/// test, not by eye. Out of bounds is a wall that does not glow; a missing
/// tile is open and dark — both halves match `is_blocked`'s own edge
/// semantics.
fn blocked_and_glow(
    tiles: &HashMap<ChunkCoord, FieldTile>,
    bounds: Option<Rect>,
    world_x: i32,
    world_y: i32,
) -> (bool, bool) {
    if let Some(b) = bounds {
        if !b.contains(world_x, world_y) {
            return (true, false);
        }
    }
    let (fx, fy) = field_coord_of(world_x, world_y);
    let (tile_coord, lx, ly) = tile_and_local(fx, fy);
    match tiles.get(&tile_coord) {
        Some(tile) => (tile.is_blocked_local(lx, ly), tile.has_glow && tile.glow_local(lx, ly) > 0.0),
        None => (false, false),
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
/// back-traced lookups and — as of architecture report §6a —
/// `World::field_at_bilinear`, the gradient-followers' entry point. Falls
/// back gracefully at the edges of loaded data because `sample` already does
/// — no special casing needed for that.
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
pub(crate) fn sample_bilinear(
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
        moisture: lerp(a.moisture, b.moisture, t),
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
    // Issue #4: skip the whole five-pass solve once the field has already
    // converged to a fixed point *and* nothing is moving on the CA grid.
    // Checking both, not just the field's own convergence, is what keeps
    // "a shockwave can cross the whole screen" (this doc's own claim above)
    // safe without any separate per-tile occupancy tracking: painting a new
    // wall (or any other CA write) always dirties its own chunk, so
    // `active_chunk_count()` stays nonzero for at least one more frame
    // after any disturbance, forcing one more full pass — which is what
    // actually notices the wall, since a newly-blocked cell resets to
    // `FieldCell::AMBIENT` in `next` (every pass below skips blocked cells,
    // leaving them at `next`'s fresh-initialized value) while `old` still
    // holds its pre-block value, a jump `is_converged` below will not miss.
    // `add_pressure_impulse`/`add_heat`/`add_light`/`add_heat_local` clear
    // `fields_settled` directly, since those bypass the CA grid entirely
    // and would otherwise sit unprocessed the next time this sees zero CA
    // activity and returns early. `parallel::ChunkView::add_heat`'s
    // same-chunk branch does too (`field_touched`, replayed in
    // `parallel::run_pass`) — a worker has no `&mut World` to clear the
    // flag on the spot, found missing by an independent review before this
    // shipped.
    //
    // **Known, self-correcting limitation** (same review): `Chunk::mark_
    // dirty` only ever sets `pending_dirty`; promotion to the `dirty` state
    // `active_chunk_count()` actually reads happens in `World::end_step`,
    // called once per frame from `parallel::step` *before*
    // `step_active_sites()`/`particle::step()` run (see `App::update`). So
    // a wall placed by plant growth (`wood` is `kind: Plant`, which blocks
    // per `rebuild_blocked`) or a landing particle depositing material is
    // invisible to `active_chunk_count()` for the one frame it happens on
    // if the field was already fully converged and quiet — this pass would
    // see the stale zero count and skip, missing that one frame's
    // occupancy change. It is never lost: the *next* frame's own
    // `end_step()` promotes the pending mark before this runs again, so
    // the wall is noticed one frame late, not never. Narrower still: CA
    // writes from the sweep itself are never subject to this, since their
    // own `mark_dirty` → `end_step` promotion happens entirely inside the
    // same `parallel::step` call, before `step_fields()` runs.
    // §5h's day/night oscillator adds a wrinkle here: `apply_sky`'s forced
    // value now changes with `world.frame` alone, with no CA write to keep
    // `active_chunk_count()` nonzero the way every *other* disturbance this
    // early-return relies on does (see the paragraph above). Without this
    // extra check, a field that settled at, say, noon's amplitude and then
    // saw the CA grid go fully quiet would stay frozen at noon forever —
    // `fields_settled()` never gets a reason to go false again just because
    // time passed. Comparing this frame's amplitude against *last* frame's
    // (not the last frame this function actually ran a full solve for) is
    // deliberately cheap — an O(1) pure-function call, not a field read —
    // and self-correcting even though it can theoretically let a few
    // frames' worth of sub-epsilon drift accumulate right at a peak/trough
    // where the curve is nearly flat anyway: the next time consecutive
    // frames actually differ by more than the epsilon, a full solve runs
    // and catches everything up, the same way any other epsilon-bounded
    // settling check in this file already tolerates.
    // `saturating_sub`, not `wrapping_sub`: at frame 0 (a world that has
    // never had `begin_step` called on it at all, e.g. `field::step` driven
    // directly in isolation, as several tests and the `field_sleep_scene`
    // ascii example do) there is no meaningful "frame -1" to compare
    // against, and wrapping to `u64::MAX` would compare against whatever
    // `sky_light_amplitude(u64::MAX)` happens to land on by coincidence of
    // `u64::MAX % DAY_NIGHT_PERIOD_FRAMES` rather than anything meaningful.
    // Saturating at 0 instead means "no time has passed since frame 0" reads
    // as exactly zero change, which is the actually correct answer.
    // Exact inequality, not an epsilon: `sky_light_amplitude` is quantised to
    // `SKY_LIGHT_STEP`, so consecutive frames either hold the identical value
    // or differ by a whole step. An epsilon here would be asking whether a
    // discrete value changed by more than a fraction of its own step, which
    // it cannot -- and the old epsilon test is precisely why this flag never
    // fired and the sky had to be driven by tiles that never slept.
    let amplitude_changed =
        sky_light_amplitude(world.frame) != sky_light_amplitude(world.frame.saturating_sub(1));
    if world.active_chunk_count() == 0 && world.fields_settled() && !amplitude_changed {
        return;
    }

    let coords: Vec<ChunkCoord> = world.chunks().map(|c| c.coord).collect();
    let bounds = world.bounds();
    let amplitude = sky_light_amplitude(world.frame);

    // **Which tiles actually need solving.**
    //
    // Getting past the early-out above used to mean solving everything, which
    // is O(world) work for whatever activity happened to be present. Measured
    // with one radius-4 impulse held in a corner: 2.5 ms at 512x320, 11.5 ms
    // at 1024x640, **53 ms at 2048x1280** — and identical to the cost of
    // disturbing the entire world, which is the defect stated as a number.
    // `examples/ascii.rs`'s `field_scaling_scene` is that measurement.
    //
    // A tile needs solving when it has not converged, when the CA chunk under
    // it is awake (something moved, so `blocked`/`moisture_source` may be
    // stale), or when it is next to such a tile — because the stencils below
    // read one field cell past their own tile, so a disturbance advances one
    // tile per frame and the ring is what lets it.
    let mut awake: HashSet<ChunkCoord> = HashSet::with_capacity(coords.len());
    // **Whether anything anywhere is awake for a reason the momentum passes
    // care about.** A tile woken only by the sun has no news for pressure,
    // velocity or advection: it is settled and the CA under it has not
    // moved, so those three spend their whole visit recomputing a fixed
    // point the tile is already sitting on. Measured at 8192x2560, that was
    // 31.6 ms of a 54 ms sky-step frame.
    //
    // **A whole-world flag, not a per-tile set, and that is a deliberate
    // retreat from a version that measured better.** Skipping the momentum
    // passes per tile — fluid seeds plus one ring, everything else carried
    // forward — is bit-identical on a calm world (0.00000 divergence in all
    // six channels over 3,600 frames, seed 3, which never gusts) and is
    // *not* identical while a gust is live: pressure diverged by 11.04
    // against a settle epsilon of 0.01. The reason is real rather than a
    // bug. With the sun up, sky-woken tiles across the whole world were
    // being pressure-stepped, so a gust relaxed through all of them; per
    // tile it advances one ring per frame instead, which is what this
    // file's own halo comments say a disturbance is supposed to do. So the
    // *old* behaviour is the accident — a gust that spreads further at noon
    // than at midnight — but it is wind the player can see, in smoke and in
    // leaning trees, and changing how it moves is the owner's call and not
    // a side effect of a performance pass. Gated globally, both regimes stay
    // bit-identical and the calm world keeps the whole saving.
    let mut any_fluid = false;
    for &coord in &coords {
        let tile = world.fields_ref().get(&coord);
        let tile_unsettled = tile.is_none_or(|t| !t.settled());
        let chunk_awake = world.chunk(coord).is_some_and(|c| !c.is_settled());
        // **Only `chunk_awake`, deliberately not `tile_unsettled`.** A tile is
        // unsettled whenever *any* of six channels moved, and after a sky
        // step ~95 tiles are unsettled purely because their light did. Those
        // kept this flag true on every frame of a dead-calm world, so the
        // fast path below never once fired -- caught by printing the
        // momentum count next to the timings, exactly the "did it fire at
        // all needs a counter, not a picture" case, since a pass costing
        // 0.00 ms and a pass that ran look identical in a timing. Whether
        // the momentum channels have anything left to say is answered
        // properly by `momentum_zero`; this is only the CA's half.
        any_fluid |= chunk_awake;
        // Light is an equilibrium, not a value that settles: `apply_sky` only
        // ever raises it and only `LIGHT_DECAY` inside `step_diffusion` lowers
        // it, so a lit tile that sleeps keeps its last brightness forever --
        // measured as midnight reading 3.999994 against a night floor of 0.2.
        let sky_moved = tile.is_some_and(|t| t.sky_drifted(amplitude));
        if tile_unsettled || chunk_awake || sky_moved {
            awake.insert(coord);
        }
    }
    // The halo. One ring is sufficient *because* `step_advection`'s source
    // displacement is clamped to a single field cell below; without that
    // clamp a fast cell could back-trace past the ring and sample a tile this
    // frame never populated, which `sample` answers with `AMBIENT` rather
    // than with the truth — a silent wrong reading, not a panic.
    for coord in awake.iter().copied().collect::<Vec<_>>() {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let n = ChunkCoord::new(coord.x + dx, coord.y + dy);
                if world.chunk(n).is_some() {
                    awake.insert(n);
                }
            }
        }
    }
    // Sorted so the solve order is deterministic — `HashSet` iteration is not,
    // and `PLAN.md` requires same-build reproducibility.
    let mut solve: Vec<ChunkCoord> = awake.iter().copied().collect();
    solve.sort_unstable_by_key(|c| (c.y, c.x, c.slice));

    // What the velocity and advection snapshots must be able to *read*: the
    // tiles being solved, plus one ring, since a cell at a tile edge samples
    // into its neighbour.
    let mut read: HashSet<ChunkCoord> = awake.clone();
    for coord in awake.iter().copied().collect::<Vec<_>>() {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let n = ChunkCoord::new(coord.x + dx, coord.y + dy);
                if world.chunk(n).is_some() {
                    read.insert(n);
                }
            }
        }
    }
    let mut read_coords: Vec<ChunkCoord> = read.into_iter().collect();
    read_coords.sort_unstable_by_key(|c| (c.y, c.x, c.slice));

    // Nothing fluid anywhere means every tile in `solve` is a sun-woken one,
    // so the three momentum passes have no tile to say anything new about --
    // *provided* their channels have actually reached zero, which is what
    // makes the skip a no-op rather than a freeze. Checked over `read_coords`
    // and not `solve`, because `step_pressure` reads one field cell past each
    // tile: a solved tile beside a *sleeping* one still holding velocity
    // would take real divergence from it, and a skip keyed on the solve set
    // alone would silently drop that.
    let skip_momentum = sky_fast()
        && !any_fluid
        && read_coords.iter().all(|c| world.fields_ref().get(c).is_some_and(|t| t.momentum_zero));
    let momentum: &[ChunkCoord] = if skip_momentum { &[] } else { &solve };

    // Old state stays untouched in `world` until the very end, so every phase
    // below can read it through `sample`/`is_blocked` while writing into
    // `next` — no aliasing, because nothing is shared.
    //
    // **`next` holds only the solved subset**, and the end of this function
    // merges it into the live map rather than replacing the map wholesale.
    // It used to hold every tile — sleeping ones carried forward by clone —
    // and a `FieldTile` owns five boxed slices, so a 640-chunk world paid a
    // few thousand allocations a frame for tiles nothing touched: ~6 µs a
    // tile, the world-area-proportional half of the idle bill the 2026-08
    // world review's field-step audit decomposed, and the half that grows
    // with M10's world sizes.
    //
    // **Merging without the rest of this design was tried and reverted** — it
    // moved `apply_sky` after the merge, onto the live map, and therefore
    // after convergence was judged; its light write then landed in `old` but
    // never in `next`, so every lit tile read as "changed" forever, nothing
    // converged, and one four-cell impulse went from 5.2 ms back to 47 ms at
    // 2048x1280. The pass order is load-bearing: whatever writes a channel
    // has to write it *before* the comparison that decides the tile is done.
    // What makes the subset safe now is the second mechanism that revert
    // note asked for: `apply_sky_to` still walks every column of the world
    // top to bottom, but *reads attenuation through the old map* for tiles
    // outside the subset and writes only into tiles inside it. That loses
    // nothing, because a tile whose light is actually changing cannot be
    // outside the subset: `sky_drifted` puts every lit tile whose amplitude
    // is stale into `awake`, so a lit tile sleeping at a plateau already
    // holds exactly the value this amplitude would write (a max-write no-op),
    // and a dark tile gets nothing either way. The other subset readers —
    // `step_velocity`/`step_advection`'s ring snapshots — fall back to the
    // old map for ring tiles the subset lacks, which is the same state the
    // clone used to hand them.
    //
    // Iterating `solve` rather than filtering `coords` by `awake` builds the
    // same set -- `awake` is drawn from `coords` and its halo only admits
    // coords `world.chunk` already knows -- in a deterministic order.
    let mut next: HashMap<ChunkCoord, FieldTile> = HashMap::with_capacity(solve.len());
    // **Which tiles need their CA-derived arrays rescanned, and which can
    // carry them forward.** See `FieldTile::inherit_derived` for the measured
    // cost this split exists to avoid.
    //
    // Sound because `Chunk::is_settled` is exactly "nothing to sweep", and
    // its own doc guarantees the direction that matters here: a chunk cannot
    // go from settled to awake without a write, and every write marks it
    // dirty through `set_world`. So a settled chunk's occupancy is unchanged
    // since the tile above it was last solved -- and it *was* solved, because
    // an awake chunk always seeds its own tile into `awake` above, and the
    // early-out at the top of this function cannot fire while any chunk is
    // awake (`active_chunk_count` is precisely the count of unsettled ones).
    // A tile with no previous state has nothing to carry and always rescans,
    // which is what makes the first solve of a fresh world correct.
    let mut rescan: Vec<ChunkCoord> = Vec::with_capacity(solve.len());
    let mut carried: Vec<ChunkCoord> = Vec::new();
    for &coord in &solve {
        let mut tile = FieldTile::new();
        let previous = world.fields_ref().get(&coord);
        // The one array on a tile that is authored rather than derived, so
        // the only one a fresh tile cannot reconstruct. See
        // `FieldTile::moisture_floor`.
        if let Some(previous) = previous {
            tile.inherit_moisture_floor(previous);
        }
        // **A sky-only tile must present its momentum channels, not zeroes.**
        // `step_velocity` and `step_advection` snapshot `next` and read one
        // cell past each tile, so a tile that is in the solve set but skipped
        // the momentum passes would hand its fluid neighbours a fresh tile's
        // `pressure == 0` -- a silent wrong reading, not a panic, and exactly
        // the shape of the light erasure that made the reverted column subset
        // look like a win. Carrying them forward is also what the tile means:
        // it is settled, and a *sleeping* tile already holds its momentum
        // channels indefinitely. This only makes a sun-woken tile behave like
        // the sleeping tile it otherwise is.
        if skip_momentum {
            if let Some(previous) = previous {
                tile.inherit_momentum(previous);
            }
        }
        match previous {
            Some(previous)
                if previous.derived_valid
                    && carry_derived()
                    && world.chunk(coord).is_some_and(|c| c.is_settled()) =>
            {
                tile.inherit_derived(previous);
                carried.push(coord);
            }
            _ => rescan.push(coord),
        }
        // Solved against the sky as it is now.
        tile.sky_amplitude = amplitude;
        next.insert(coord, tile);
    }

    // Per-pass wall time, printed when `FIELD_PASS=<every N frames>` is set.
    // The five passes were an undifferentiated 75 ms/frame at 4x until this
    // existed; a design for a light-only fast path written without it would
    // have been a guess about which of the five it was avoiding.
    let mut timing = PassTiming::new();
    timing.time("blocked", || rebuild_blocked(world, &rescan, &mut next));
    timing.time("glowseed", || seed_light_from_glow(&carried, &mut next));
    timing.time("pressure", || step_pressure(world, momentum, &mut next));
    timing.time("velocity", || step_velocity(world, momentum, &read_coords, &mut next));
    // Diffusion stays on the **full** solve set. It is what bleeds light
    // sideways so shade is soft rather than a hard stencil at field
    // resolution -- a canopy's whole appearance rests on it, and skipping it
    // for sun-woken tiles would look like a win in every timing while
    // changing what the world looks like under every tree.
    timing.time("diffusion", || step_diffusion(world, &solve, &mut next));
    timing.time("advection", || {
        step_advection(momentum, &read_coords, bounds, world.fields_ref(), &mut next)
    });
    // **Always every column, never subsetted.** Light propagates down a
    // column, so a sleeping tile between the sky and an awake one still
    // attenuates — but the deeper reason is that this pass is what *drives*
    // the day/night cycle, and subsetting it stops the world's clock.
    //
    // `SETTLE_EPSILON_LIGHT` is 0.005 and the sun's amplitude changes by at
    // most ~0.0033 per frame, so `amplitude_changed` above is essentially
    // never true. The sky does not advance because that flag fires; it
    // advances because this pass writes a slightly different value every
    // frame, `mark_converged` notices, and the tile stays awake. Subsetting to
    // columns containing an awake tile broke exactly that loop: a quiet world
    // had no awake tiles, so no column was written, so the light froze at
    // whatever it was — `the_sky_keeps_cycling_through_day_and_night_even_
    // after_the_field_goes_quiet` caught it reading noon at midnight.
    //
    // The consequence is honest and worth stating: sky-lit tiles never sleep,
    // because the sun really is always moving. What sleeps is everything the
    // light does not reach, which on a large world is most of it.
    //
    // **Subsetting this by column was tried in the round-7 scale work and
    // reverted.** Gating each column on `sky_drifted` -- now a discrete
    // event, since `sky_light_amplitude` is quantised -- looked like the
    // obvious win and measured 8.26 -> 6.55 ms at 2048x640. It was measuring
    // a bug: a tile in the solve set is rebuilt from a fresh `FieldTile` at
    // the top of this function, so a skipped column does not keep stale
    // light, it *loses* it (measured, mid-air cells going 2.43 -> 0.0 and
    // staying there, because the fresh tile also takes
    // `sky_amplitude = amplitude` and so reports no drift to ask for a
    // repair). Correcting the subset to "solved OR drifted" made it cover
    // the whole world anyway and cost 8.26 -> 9.06 ms.
    //
    // **Both of those measurements were taken inside the transient, and that
    // is the thing to know before retrying.** A generated world's field takes
    // ~4500 frames to converge (pressure churns and then decays cleanly);
    // during it the solve set is most of the world and there is nothing to
    // subset to. The *steady state* is a different regime: the sky is
    // quantised, so a full solve runs only on the ~760 frames in 3600 where
    // the sky steps, and on those the only tiles needing work are the sky-lit
    // ones. Measured settled at 8192x2560: 4.87 ms mean, 10.72 ms worst.
    // Retry this against those numbers rather than the transient ones -- and
    // keep the light-erasure trap above in mind, because it is what made the
    // first attempt look like a win.
    // `Reports/field-settling-2026-08.md` has the full measurement.
    timing.time("sky", || {
        let _lit = apply_sky_to(amplitude, &coords, world.fields_ref(), &mut next);
    });
    timing.time("moisture", || apply_moisture_sources(&solve, &mut next));

    // Convergence is per tile. A tile outside the awake set was settled *and*
    // had a settled chunk under it — that is what kept it out — so only the
    // solved tiles can have changed verdict, and the global flag needs no
    // fresh scan of the world.
    // Solved tiles merge into the live map; sleeping tiles stay where they
    // are, untouched and unallocated — the point of the subset.
    if !skip_momentum {
        for &coord in &solve {
            if let Some(tile) = next.get_mut(&coord) {
                tile.record_momentum_zero();
            }
        }
    }
    timing.time("converged", || mark_converged(world.fields_ref(), &solve, &mut next));

    let all_settled = solve.iter().all(|c| next.get(c).is_some_and(|t| t.settled()));
    timing.report(world.frame, solve.len(), momentum.len());
    debug_drift(world, &solve, &next);
    world.merge_fields(next);
    world.set_fields_settled(all_settled);
}

/// Whether the momentum passes skip sun-woken tiles; `FIELD_SKYFAST=0` turns
/// it off. Same purpose as `carry_derived` below: a paired run in one command.
///
/// Unlike that one this is **not** bit-identical and is not meant to be. It
/// stops relaxing channels on a tile the solver has already judged settled,
/// so a value that was decaying by less than `SETTLE_EPSILON_PRESSURE` per
/// frame now holds still instead. That is not an approximation of the design;
/// it is the design — a *sleeping* tile already holds those channels
/// indefinitely, and this only stops a sun-woken tile from getting
/// relaxation its sleeping neighbour never gets. The bar is therefore
/// bounded divergence, not identity: no channel may differ from the
/// unaccelerated run by more than its own settle epsilon.
fn sky_fast() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("FIELD_SKYFAST").map(|v| v != "0").unwrap_or(true))
}

/// Whether the carry-forward above is enabled; `FIELD_CARRY=0` turns it off.
///
/// Exists so the paired run that proves it costs nothing in correctness is
/// one command rather than a `git stash` cycle -- and this file's own gotcha
/// list records a stash restoring an older blob over a source file, so that a
/// commit claimed a behaviour change and shipped only its doc comment. It is
/// also the only honest way to re-measure a baseline *in the same session on
/// the same machine*: the first reading of this change looked like a 29.5 ms
/// win alongside a 50% regression in every other pass, and the regression was
/// entirely the machine having slowed between two runs an hour apart.
///
/// One `OnceLock` read per `step`, not per tile.
fn carry_derived() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("FIELD_CARRY").map(|v| v != "0").unwrap_or(true))
}

/// A checksum over **every byte of field state in the world**, for holding an
/// optimisation to producing the same field rather than to a green suite.
///
/// The suite asserts properties, and a light or blocked array that is subtly
/// different but still plausible passes all of them -- which is exactly how
/// the reverted `apply_sky_to` column subset measured as a 21% win while
/// erasing light. Two builds printing the same number over a full day/night
/// cycle is the claim worth making, and it is the only one that catches a
/// fast path that is *nearly* right.
///
/// Covers the six channels, all four arrays `rebuild_blocked` derives, the
/// authored moisture floor, and the three per-tile solve flags. Chunk order
/// is sorted rather than `HashMap` order, so the number is reproducible
/// within a build as `PLAN.md` requires.
pub fn field_hash(world: &World) -> u64 {
    let mut coords: Vec<ChunkCoord> = world.fields_ref().keys().copied().collect();
    coords.sort_unstable_by_key(|c| (c.y, c.x, c.slice));
    let mut acc: u64 = 0xcbf2_9ce4_8422_2325;
    let eat = |v: u64, acc: &mut u64| {
        *acc ^= v;
        *acc = acc.wrapping_mul(0x100_0000_01b3);
    };
    for coord in coords {
        let Some(t) = world.fields_ref().get(&coord) else { continue };
        for ly in 0..FIELD_TILE_SIZE {
            for lx in 0..FIELD_TILE_SIZE {
                let c = t.get_local(lx, ly);
                for f in [c.pressure, c.vx, c.vy, c.temperature, c.light, c.moisture] {
                    eat(u64::from(f.to_bits()), &mut acc);
                }
                eat(u64::from(t.is_blocked_local(lx, ly)), &mut acc);
                eat(u64::from(t.transmission_local(lx, ly).to_bits()), &mut acc);
                eat(u64::from(t.moisture_source_local(lx, ly).to_bits()), &mut acc);
                eat(u64::from(t.glow_local(lx, ly).to_bits()), &mut acc);
                eat(u64::from(t.moisture_floor_local(lx, ly).to_bits()), &mut acc);
            }
        }
        eat(u64::from(t.has_glow), &mut acc);
        eat(u64::from(t.derived_valid), &mut acc);
        eat(u64::from(t.momentum_zero), &mut acc);
        eat(u64::from(t.settled()), &mut acc);
        eat(u64::from(t.sky_lit), &mut acc);
        eat(u64::from(t.sky_amplitude.to_bits()), &mut acc);
    }
    acc
}

/// Every field channel in the world, flattened in the same sorted order
/// `field_hash` uses, for measuring how far one build's field drifts from
/// another's.
///
/// `field_hash` answers "identical or not", which is the right bar for a
/// change that claims to be a pure optimisation and the *wrong* bar for one
/// that deliberately stops relaxing a settled channel. There the claim is
/// bounded divergence — no channel further from the unaccelerated run than
/// its own settle epsilon — and that needs the values, not a checksum.
///
/// Six f32 per field cell, channel-major per cell: pressure, vx, vy,
/// temperature, light, moisture.
pub fn field_channels(world: &World) -> Vec<f32> {
    let mut coords: Vec<ChunkCoord> = world.fields_ref().keys().copied().collect();
    coords.sort_unstable_by_key(|c| (c.y, c.x, c.slice));
    let mut out = Vec::with_capacity(coords.len() * FIELD_TILE_AREA * 6);
    for coord in coords {
        let Some(t) = world.fields_ref().get(&coord) else { continue };
        for ly in 0..FIELD_TILE_SIZE {
            for lx in 0..FIELD_TILE_SIZE {
                let c = t.get_local(lx, ly);
                out.extend_from_slice(&[c.pressure, c.vx, c.vy, c.temperature, c.light, c.moisture]);
            }
        }
    }
    out
}

/// Wall time per pass, printed when `FIELD_PASS=<every N frames>` is set.
/// Off by default; the env var is read once and the whole struct collapses to
/// a `None` check when it is unset, so nothing here runs in a shipped frame.
struct PassTiming {
    every: u64,
    marks: Vec<(&'static str, f64)>,
}

impl PassTiming {
    fn new() -> Self {
        use std::sync::OnceLock;
        static EVERY: OnceLock<u64> = OnceLock::new();
        let every =
            *EVERY.get_or_init(|| std::env::var("FIELD_PASS").ok().and_then(|v| v.parse().ok()).unwrap_or(0));
        PassTiming { every, marks: Vec::new() }
    }

    fn time<R>(&mut self, name: &'static str, f: impl FnOnce() -> R) -> R {
        if self.every == 0 {
            return f();
        }
        let t = std::time::Instant::now();
        let r = f();
        self.marks.push((name, t.elapsed().as_secs_f64() * 1000.0));
        r
    }

    /// `momentum` is printed next to the timings deliberately: a pass that
    /// costs 0.00 ms looks the same whether it was skipped or merely fast,
    /// and this repo has already read "the feature never once executed" as
    /// "the feature is working". The count says which.
    fn report(&self, frame: u64, solved: usize, momentum: usize) {
        if self.every == 0 || !frame.is_multiple_of(self.every) {
            return;
        }
        let total: f64 = self.marks.iter().map(|(_, ms)| ms).sum();
        let detail: Vec<String> = self.marks.iter().map(|(n, ms)| format!("{n} {ms:.2}")).collect();
        println!(
            "  [pass] frame {frame:>6} solved {solved:>5} momentum {momentum:>5} total {total:>7.2}ms | {}",
            detail.join("  ")
        );
    }
}

/// Per-channel attribution of *why* the solve set is not shrinking, printed
/// when `FIELD_DRIFT=<every N frames>` is set. Off by default and reading one
/// env var per call; nothing here runs in a shipped frame.
///
/// This exists because the first attempt to attribute the load transient
/// guessed at the channel from the order of the `if` in `mark_converged` and
/// got it wrong. `mark_converged` short-circuits on the first channel over
/// epsilon, so it cannot say what *else* was also moving, and pressure is
/// tested first. Every channel is tested here, independently.
fn debug_drift(world: &World, coords: &[ChunkCoord], next: &HashMap<ChunkCoord, FieldTile>) {
    use std::sync::OnceLock;
    static EVERY: OnceLock<u64> = OnceLock::new();
    let every = *EVERY.get_or_init(|| {
        std::env::var("FIELD_DRIFT").ok().and_then(|v| v.parse().ok()).unwrap_or(0)
    });
    if every == 0 || !world.frame.is_multiple_of(every) {
        return;
    }
    let old = world.fields_ref();
    // Six channels, in `FieldCell`'s own order: pressure, vx, vy,
    // temperature, light, moisture.
    let names = ["pressure", "vx", "vy", "temp", "light", "moisture"];
    let eps = [
        SETTLE_EPSILON_PRESSURE,
        SETTLE_EPSILON_VELOCITY,
        SETTLE_EPSILON_VELOCITY,
        SETTLE_EPSILON_TEMPERATURE,
        SETTLE_EPSILON_LIGHT,
        SETTLE_EPSILON_MOISTURE,
    ];
    let mut over = [0usize; 6];
    let mut peak = [0.0f32; 6];
    let mut unsettled = 0usize;
    for &coord in coords {
        let (Some(a_t), Some(b_t)) = (old.get(&coord), next.get(&coord)) else { continue };
        if !b_t.settled() {
            unsettled += 1;
        }
        let mut tile_over = [false; 6];
        for ly in 0..FIELD_TILE_SIZE {
            for lx in 0..FIELD_TILE_SIZE {
                let a = a_t.get_local(lx, ly);
                let b = b_t.get_local(lx, ly);
                let d = [
                    (a.pressure - b.pressure).abs(),
                    (a.vx - b.vx).abs(),
                    (a.vy - b.vy).abs(),
                    (a.temperature - b.temperature).abs(),
                    (a.light - b.light).abs(),
                    (a.moisture - b.moisture).abs(),
                ];
                for c in 0..6 {
                    peak[c] = peak[c].max(d[c]);
                    if d[c] > eps[c] {
                        tile_over[c] = true;
                    }
                }
            }
        }
        for c in 0..6 {
            if tile_over[c] {
                over[c] += 1;
            }
        }
    }
    let detail: Vec<String> = (0..6)
        .map(|c| format!("{} {}/{:.4}", names[c], over[c], peak[c]))
        .collect();

    // **Why each tile is awake, recomputed exactly as `step` computed it.**
    // Safe to re-derive here because this runs *before* `merge_fields`, so
    // `world.fields_ref()` is still the pre-step map the awake set was built
    // from. Without this the solved count is a number with no explanation:
    // "555 tiles solved, 0 of them unsettled" is a contradiction until you
    // can say which of the three seeds put them there, and the halo turns
    // every seed into up to nine.
    let amplitude = sky_light_amplitude(world.frame);
    let (mut no_tile, mut unsettled_seed, mut chunk_seed, mut sky_seed) = (0, 0, 0, 0);
    for chunk in world.chunks() {
        let tile = old.get(&chunk.coord);
        match tile {
            None => no_tile += 1,
            Some(t) if !t.settled() => unsettled_seed += 1,
            _ => {}
        }
        if world.chunk(chunk.coord).is_some_and(|c| !c.is_settled()) {
            chunk_seed += 1;
        }
        if tile.is_some_and(|t| t.sky_drifted(amplitude)) {
            sky_seed += 1;
        }
    }
    println!(
        "  [drift] frame {:>6} solved {:>4} unsettled {:>4} | seeds: no-tile {} unsettled {} chunk {} sky {} | {}",
        world.frame,
        coords.len(),
        unsettled,
        no_tile,
        unsettled_seed,
        chunk_seed,
        sky_seed,
        detail.join("  ")
    );
}

/// Below this much change in a single step, per channel, a field cell counts
/// as having stopped moving. Not zero: the damping/decay constants above are
/// genuine exponential decays that approach their fixed point asymptotically
/// without ever exactly reaching it in finite time, so *some* tolerance is
/// required for anything to ever be judged settled at all — the same
/// reasoning `fire.rs`'s `THERMAL_SETTLE_EPSILON` and this file's own
/// `PRESSURE_DAMPING`/`VELOCITY_DAMPING` comments already document for the
/// same family of problem. Picked small relative to each channel's working
/// range (pressure/velocity's own coupling and damping constants, `MAX_LIGHT`)
/// rather than derived from anything more principled; `stays_bounded_over_
/// ten_thousand_frames` and the scene-based ascii checks are the actual
/// authority on whether these are tight enough to matter, the same as every
/// other tuning constant in this file.
/// How far advection may back-trace, in field tiles.
///
/// Sized from what restores the physics, then justified: four tiles is where
/// an impulse in open ground disperses exactly as it did before the awake set
/// existed (sealed 61.71126, open 2.9185925 -- byte-identical to the
/// pre-sleeping baseline), and one tile is not (open 80.8, still 27x too
/// concentrated).
///
/// Going past the one-tile halo is safe because advection only ever *reads*
/// through `sample`; it writes nothing outside its own tile, so an overlong
/// back-trace costs accuracy and never correctness. And the accuracy it costs
/// is small: `sample` answers `AMBIENT` for a tile outside `read`, and a tile
/// outside `read` is one that has converged -- so ambient is approximately
/// what is there anyway.
const ADVECTION_MAX_TILES: i32 = 4;

const SETTLE_EPSILON_PRESSURE: f32 = 0.01;
const SETTLE_EPSILON_VELOCITY: f32 = 0.001;
const SETTLE_EPSILON_TEMPERATURE: f32 = 0.02;
const SETTLE_EPSILON_LIGHT: f32 = 0.005;
const SETTLE_EPSILON_MOISTURE: f32 = 0.005;

/// Whether every field cell in `next` is within its channel's settle epsilon
/// of the corresponding cell in `old` — the field's own notion of "stopped
/// changing," independent of and in addition to the CA grid's `active_chunk_
/// count`. `old` is looked up directly from `world` (the pre-step state
/// `step` above never mutates until its very last line) rather than passed
/// in separately, since every caller already has a `&World` in hand.
/// Record, per tile, whether it reached its own fixed point this solve.
///
/// Was `is_converged`, collapsing every tile into one bool. That single answer
/// is what made the solve all-or-nothing: it could say "something, somewhere,
/// is still moving" but not *where*, so the next frame had no choice but to
/// solve everything again. The comparison itself was always per tile — this
/// only stops throwing the detail away.
///
/// Only `coords` (the tiles actually solved) are judged. A sleeping tile keeps
/// the verdict it already had, which is correct: nothing touched it.
fn mark_converged(
    old: &HashMap<ChunkCoord, FieldTile>,
    coords: &[ChunkCoord],
    next: &mut HashMap<ChunkCoord, FieldTile>,
) {
    for &coord in coords {
        let Some(old_tile) = old.get(&coord) else {
            // No previous tile to compare against — nothing to call settled.
            if let Some(t) = next.get_mut(&coord) {
                t.set_settled(false);
            }
            continue;
        };
        let mut settled = true;
        if let Some(new_tile) = next.get(&coord) {
            'tile: for ly in 0..FIELD_TILE_SIZE {
                for lx in 0..FIELD_TILE_SIZE {
                    let a = old_tile.get_local(lx, ly);
                    let b = new_tile.get_local(lx, ly);
                    if (a.pressure - b.pressure).abs() > SETTLE_EPSILON_PRESSURE
                        || (a.vx - b.vx).abs() > SETTLE_EPSILON_VELOCITY
                        || (a.vy - b.vy).abs() > SETTLE_EPSILON_VELOCITY
                        || (a.temperature - b.temperature).abs() > SETTLE_EPSILON_TEMPERATURE
                        || (a.light - b.light).abs() > SETTLE_EPSILON_LIGHT
                        || (a.moisture - b.moisture).abs() > SETTLE_EPSILON_MOISTURE
                    {
                        settled = false;
                        break 'tile;
                    }
                }
            }
        }
        if let Some(t) = next.get_mut(&coord) {
            t.set_settled(settled);
        }
    }
}

/// Frames per full day/night cycle — architecture §5h, "the same writer
/// [as §2's sky] with a time-varying amplitude." Picked so a scene running
/// for a few thousand frames (the scale of this file's own longer tests,
/// and of `plant.rs`'s multi-thousand-frame growth runs) sees at least one
/// full day and one full night, not just a sliver of one.
pub const DAY_NIGHT_PERIOD_FRAMES: u64 = 3600;
/// Floor on `sky_light_amplitude` at the darkest point of night — real
/// moon/starlight, not absolute zero. Keeps night from being a hard on/off
/// switch for everything reading the light channel (moss shade-seeking,
/// phototropism), the same reasoning `shade_factor`'s own floor uses.
const NIGHT_LIGHT_FLOOR: f32 = 0.2;

/// The sky's own light output at a given frame — a smooth oscillation
/// between `NIGHT_LIGHT_FLOOR` and `MAX_LIGHT`, spending exactly half of
/// `DAY_NIGHT_PERIOD_FRAMES` at the floor (night) and half ramping through a
/// cosine hump (day), rather than a sine that would spend the whole cycle
/// above the floor. Real daylight is roughly this shape — a smooth rise
/// from sunrise, a peak at noon, a smooth fall to sunset, then flat dark
/// until the next sunrise — not a pure sinusoid with no true night.
pub fn sky_light_amplitude(frame: u64) -> f32 {
    let daylight = sun_elevation(frame).max(0.0);
    // Quantised on the 0..1 *daylight fraction* rather than on the amplitude
    // it scales to, so the two endpoints survive exactly: `daylight == 0`
    // rounds to 0 and `daylight == 1` rounds to the top step, giving
    // `NIGHT_LIGHT_FLOOR` and `MAX_LIGHT` bit-for-bit.
    // `sky_light_amplitude_cycles_between_the_night_floor_and_max_light`
    // asserts both with `assert_eq!`, and quantising the amplitude directly
    // moved the night floor to 0.19999999 -- a float artifact of the
    // rounding, not a behaviour change, but the test is right to be exact
    // and weakening it to fit an implementation detail would be the wrong
    // way round.
    let steps = (MAX_LIGHT - NIGHT_LIGHT_FLOOR) / SKY_LIGHT_STEP;
    let daylight = (daylight * steps).round() / steps;
    NIGHT_LIGHT_FLOOR + daylight * (MAX_LIGHT - NIGHT_LIGHT_FLOOR)
    // **Quantised, and that is what lets a quiet world stop solving.**
    //
    // Measured on a settled world with *zero* awake chunks: `field::step`
    // cost 8.26 ms/frame at the shipped 2048x640 and 42.37 ms at 8192x2560.
    // Not the CA sweep -- that measured 0.00 and 0.08 ms respectively, so
    // chunk sleeping was doing its job perfectly. All of it was this
    // function moving.
    //
    // The old comment above `apply_sky_to` described the mechanism and read
    // it as a necessary cost: the sun's amplitude changes by less than
    // `SETTLE_EPSILON_LIGHT` per frame, so the `amplitude_changed` early-out
    // "is essentially never true", and the sky advanced instead because the
    // pass wrote a slightly different value every frame and the tile
    // therefore never converged. That is the clock running on an accident --
    // the flag written to drive it could not, and permanently-awake tiles
    // did it instead, at the price of a full five-pass solve every frame
    // forever.
    //
    // Rounding the amplitude to `SKY_LIGHT_STEP` makes it *piecewise
    // constant*: on the frames within a step the sky writes exactly the same
    // value, tiles converge, `fields_settled()` goes true and the early-out
    // finally fires. On the frame it steps, the value differs and the solve
    // runs -- so the clock is driven by `amplitude_changed` again, which is
    // what it was written for.
    //
    // Quantising the *amplitude* rather than the phase is deliberate:
    // `sun_elevation` is shared with the renderer's sky, and this file warns
    // that the painted sky and the light channel must not drift. The sky's
    // colour stays exactly as smooth as before; only the scalar the light
    // channel is driven by is stepped. `noon_equivalent_light` divides by
    // this same function, so every economic light read stays consistent for
    // free.
}

/// Step size for [`sky_light_amplitude`], in the same units as
/// [`MAX_LIGHT`].
///
/// Bounded on both sides, and the window is wide:
///
/// - It must be **larger than `SETTLE_EPSILON_LIGHT`** (0.005), or a step
///   would not register as a change and the field would settle at a stale
///   brightness -- the exact freeze that
///   `the_sky_keeps_cycling_through_day_and_night_even_after_the_field_goes_
///   quiet` was written to catch.
/// - It must be **small enough to be invisible**. The amplitude spans
///   `NIGHT_LIGHT_FLOOR`..`MAX_LIGHT` = 3.8, so 0.01 is 0.26% of the range
///   -- far below one step of the 8-bit colour the light is eventually
///   drawn through.
///
/// How often that leaves the field solving: total variation over a day is
/// `2 x 3.8`, so `2 x 3.8 / 0.01` = 760 steps per `DAY_NIGHT_PERIOD_FRAMES`
/// = 3600 frames, or roughly one frame in five -- and the steps place
/// themselves where the light is actually moving, densely at dawn and dusk
/// and hardly at all around noon and midnight, which is the right
/// distribution rather than a uniform one.
const SKY_LIGHT_STEP: f32 = 0.01;

/// How high the sun stands, as `-1.0` (deep night) through `0.0` (exactly
/// sunrise or sunset) to `1.0` (noon).
///
/// The same cosine [`sky_light_amplitude`] is built from, exposed because the
/// renderer draws a sky from it and the two must not drift: a sky painting
/// dawn while the light channel still says midnight would be worse than no
/// sky at all. This is the shared definition of what time it is, and the sign
/// is what tells sunrise from sunset — `sun_rising` reads the other half of
/// the cycle for that.
pub fn sun_elevation(frame: u64) -> f32 {
    let phase = (frame % DAY_NIGHT_PERIOD_FRAMES) as f32 / DAY_NIGHT_PERIOD_FRAMES as f32;
    (phase * std::f32::consts::TAU).cos()
}

/// How much daylight there is, as `0.0` (deepest night) to `1.0` (noon).
///
/// [`sky_light_amplitude`] normalised by its own range. The renderer lights
/// the world from this rather than from the light *channel*, and the reason
/// is a measurement: at noon, on open terrain, the channel reads **0.30 of
/// `MAX_LIGHT` at the ground surface and 0.00 forty cells down**. Light
/// diffuses through air and is blocked by solids, so it never meaningfully
/// enters the material it would have to light — the channel is doing its own
/// job (telling plants and moss what is shaded) perfectly well and simply is
/// not a description of how lit the *rock* looks.
///
/// Making it one is a real feature — light propagating into solids, or a
/// depth term — and would be what "caves are dark and you need a torch"
/// requires. It is not what a day/night cycle requires.
pub fn daylight_fraction(frame: u64) -> f32 {
    ((sky_light_amplitude(frame) - NIGHT_LIGHT_FLOOR) / (MAX_LIGHT - NIGHT_LIGHT_FLOOR)).clamp(0.0, 1.0)
}

/// Whether the sun is on its way up. Phase runs noon → sunset → midnight →
/// sunrise, so the second half of the cycle is the rising half.
pub fn sun_rising(frame: u64) -> bool {
    let phase = (frame % DAY_NIGHT_PERIOD_FRAMES) as f32 / DAY_NIGHT_PERIOD_FRAMES as f32;
    phase >= 0.5
}

/// A light reading rescaled to **what it would be at noon** under the same
/// occlusion — the phase-free form of the channel, for anything that makes
/// a *decision* off a light read.
///
/// The light channel oscillates 20:1 over every day (`sky_light_amplitude`),
/// and that oscillation is the single deepest measurement hazard on the
/// plant branch: three separate quantities were tuned at an arbitrary phase
/// of it, the live tip count swung 71 → 28 between noon and night on the
/// *same* stand, shade abscission was impossible at any fixed threshold
/// (every leaf in the world reads near zero at night, so any cutoff is a
/// nightly extinction event), and `q_peak` latched noon-only values.
///
/// Dividing a reading by the sky's *current* output and rescaling by its
/// peak removes the oscillation while moving nothing at noon — the factor
/// is exactly 1.0 there, so every constant derived against noon behaviour
/// keeps its meaning. The oscillator is a pure function of the frame, so
/// this costs no storage and no per-cell state; it is the free version of
/// the per-cell running mean `PLAN.md` 0e originally sketched.
///
/// Clamped to `MAX_LIGHT`: nothing can intercept more than full sun. The
/// clamp also bounds the two ways a raw reading legitimately exceeds the
/// sky's current output — the field lags the amplitude by its diffusion
/// time constant at dusk (~15% for ~60 frames), and fire floods its own
/// block via `add_light` — so a burning canopy at midnight reads as "full
/// sun", which is the right answer for a plant and for the fire.
pub(crate) fn noon_equivalent_light(light: f32, frame: u64) -> f32 {
    (light / sky_light_amplitude(frame) * MAX_LIGHT).clamp(0.0, MAX_LIGHT)
}

/// Top-of-world light boundary condition — architecture report §2's sky
/// source, now driven by §5h's day/night oscillator (`sky_light_amplitude`)
/// rather than a flat `MAX_LIGHT`. The counterpart to fire's local light
/// emission in `fire.rs`. Per column, not against one global `bounds.min_y`
/// row: worldgen does not guarantee every column reaches the same height,
/// and each column's own topmost *resident* chunk is what should read as
/// exposed, not whichever row happens to be highest across the whole world.
///
/// Runs last, after `step_advection` — diffusion and advection both
/// unconditionally overwrite every field cell they touch, sky row included,
/// so applying this any earlier in the pipeline would just get clobbered.
///
/// Deliberately does **not** call `world.set_fields_settled(false)` the way
/// `add_light`/`add_heat` do. Those mark an external disturbance the solver
/// hasn't accounted for yet; this is a stable, ever-present boundary
/// condition, and `is_converged`'s own old-vs-next comparison already
/// recognizes it as settled once the sky row — and whatever it diffuses
/// into — stops changing frame to frame. That still holds with a
/// time-varying amplitude: near noon and midnight the cosine's own
/// derivative is close to zero, so consecutive frames differ by less than
/// `SETTLE_EPSILON_LIGHT` and the field settles same as it always did;
/// only near sunrise/sunset (where the amplitude is actually changing
/// fastest) does that stop holding, and the field correctly stays awake
/// through the transition — a real property of daylight, not a bug to work
/// around. Force-waking every frame regardless, the way `add_light` does,
/// would defeat field sleeping (issue #4) for every open-sky scene, all day.
/// What fraction of direct sunlight survives one blocked field block.
///
/// A field block is `FIELD_SCALE` (8) world cells square, so one blocked
/// block is a substantial thickness of canopy or rock — but not an absolute
/// wall, which matters: a hard zero makes shade a binary stencil with a
/// visible edge at field resolution, and
/// `Reports/tree-procedural-prior-art.md` records that pairing a *graded*
/// rule (self-pruning, shedding) with a *binary* light signal makes
/// branches vanish the instant they stop extending. Keeping transmission
/// graded leaves room for that mechanism to work later.
///
/// Rock still goes effectively black after two blocks (0.04), which is what
/// keeps caves dark.
const SKY_TRANSMISSION: f32 = 0.2;

/// What one CA column of a field block passes, indexed by how many opaque
/// cells deep it is: `SKY_TRANSMISSION^(depth / FIELD_SCALE)`.
///
/// Beer-Lambert, normalised so the endpoints are the two values the model
/// already committed to — an empty column passes everything, and a full one
/// passes exactly `SKY_TRANSMISSION`, so solid rock behaves exactly as it
/// did before this table existed. Everything between is the part that is
/// new, and it is why a one-cell-thick leaf plate shades (0.82 per block,
/// compounding through a crown) without shading like rock.
///
/// A table rather than `powf` in the scan: `rebuild_blocked` runs over
/// every CA cell of every resident chunk every field step, and this is nine
/// floats. `the_column_transmission_table_is_beer_lambert` keeps it honest
/// against the formula and against `SKY_TRANSMISSION` moving.
/// The last entry is written as `SKY_TRANSMISSION` itself rather than as
/// its value, because that endpoint is the compatibility promise: a fully
/// opaque column passes exactly what a blocked block always passed.
const COLUMN_TRANSMISSION: [f32; FIELD_SCALE as usize + 1] =
    [1.0, 0.817_765, 0.668_740, 0.546_884, 0.447_214, 0.365_697, 0.299_070, 0.244_581, SKY_TRANSMISSION];

/// Direct sunlight, cast **down each column** from open sky.
///
/// **Replaces seeding only the topmost chunk's top row and letting
/// diffusion carry light downward**, which made illumination a function of
/// *distance from the world's top edge* rather than of what was in the way.
/// Four separate symptoms had that one cause, and all of them are recorded
/// with measurements in `Reports/tree-architecture-implementation-plan.md`
/// §0f:
///
/// - **Light got brighter as a plant climbed**, so growing up always paid
///   and every scene ended with its trees pinned against the world
///   boundary — chased as a plant bug across two sessions and three scenes.
///   With a sweep of ground depth the outcome was a cliff, not a curve: at
///   200 rows of sky a stand reached 8,529 cells with 3 rows of clearance,
///   at 250 it managed 179 cells, and at 400 nothing germinated at all.
///   **No depth was both well-lit and un-ceilinged.**
/// - `Germinate`'s 0.1 light gate became unreachable anywhere with sky
///   access, degrading into "am I sealed in rock".
/// - The deep field never converged inside a day/night period, so the
///   profile *inverted* for ~45% of every cycle and `phototropism_dir`
///   pointed **downward** across the top ~70 rows for nearly half of each
///   day.
/// - Caves lit up through any opening wider than `FIELD_SCALE`.
///
/// A column cast fixes all four at once, because it is the physically
/// right shape: **clear air does not attenuate sunlight, occluders do.**
/// Open sky now reads the same at any depth, so height carries no intrinsic
/// reward and a canopy shades what is beneath it because it is *in the
/// way*.
///
/// Diffusion is deliberately kept for everything else — it is what makes
/// shade soft and bleeds light sideways under a canopy rather than leaving
/// a hard stencil edge at field resolution. This pass only adds the direct
/// component, taking the max so diffusion can fill but never darken.
///
/// Cost is one downward walk per field column per field step, over the
/// blocked bitmap `rebuild_blocked` has already built for every chunk this
/// step — no CA reads and no extra scan.
fn apply_sky_to(
    amplitude: f32,
    coords: &[ChunkCoord],
    old: &HashMap<ChunkCoord, FieldTile>,
    next: &mut HashMap<ChunkCoord, FieldTile>,
) -> HashSet<ChunkCoord> {
    let mut lit: HashSet<ChunkCoord> = HashSet::new();
    if coords.is_empty() {
        return lit;
    }
    // Group the chunk grid into columns so each can be walked top to
    // bottom. A chunk row with no chunk at all is open sky and passes light
    // through untouched, which is what makes a world with sparse chunks
    // behave the same as one with empty chunks allocated.
    let (min_cy, max_cy) = coords.iter().fold((i32::MAX, i32::MIN), |(lo, hi), c| (lo.min(c.y), hi.max(c.y)));
    let mut columns: Vec<i32> = coords.iter().map(|c| c.x).collect();
    columns.sort_unstable();
    columns.dedup();

    for cx in columns {
        for lx in 0..FIELD_TILE_SIZE {
            let mut carried = amplitude;
            for cy in min_cy..=max_cy {
                let coord = ChunkCoord::new(cx, cy);
                let Some(tile) = next.get_mut(&coord) else {
                    // Not in the solved subset. A *sleeping* tile still
                    // stands in the light's way — treating it as open sky
                    // would pour daylight through a sleeping mountain onto
                    // whatever awake tile sits beneath it — so the walk
                    // attenuates through the old map, read-only. No write
                    // is lost: a lit tile whose value would change is in
                    // the subset by `sky_drifted`'s construction, so a
                    // sleeping tile either already holds this amplitude's
                    // value (max-write no-op) or is dark (nothing to
                    // write). No tile at all is genuinely open sky.
                    if let Some(sleeping) = old.get(&coord) {
                        for ly in 0..FIELD_TILE_SIZE {
                            if carried <= 0.0 {
                                break;
                            }
                            carried *= sleeping.transmission_local(lx, ly);
                        }
                    }
                    continue;
                };
                for ly in 0..FIELD_TILE_SIZE {
                    if carried <= 0.0 {
                        continue;
                    }
                    // **The light reaching this block is written here even
                    // when the block is occupied**, before attenuation.
                    // A leaf's own reading should be what arrives at it --
                    // it is the thing doing the intercepting. The previous
                    // rule skipped occupied blocks entirely, so every leaf
                    // in a canopy read only whatever lateral diffusion left
                    // behind, which is why 88% of them measured under 0.05
                    // and why both shade-pruning and phototropism had no
                    // gradient to work with.
                    let mut cell = tile.get_local(lx, ly);
                    // Max, never assignment: diffusion may legitimately
                    // have brought *more* light here from a neighbouring
                    // column, and this pass must not darken it.
                    if carried > cell.light {
                        cell.light = carried;
                        tile.set_local(lx, ly, cell);
                    }
                    // **Graded, and graded per column.** The block's own
                    // transmission is Beer-Lambert over how deep each of
                    // its CA columns is, averaged — see
                    // `FieldTile::transmission`. `blocked` is
                    // all-or-nothing over 8x8 CA cells, far too coarse for
                    // foliage (one twig made a whole block opaque and a
                    // canopy read as a wall); an occupancy *fraction* fixed
                    // that and introduced its own error, reading a
                    // horizontal plate and a vertical trunk identically
                    // when only one of them is in a downward ray's way.
                    carried *= tile.transmission_local(lx, ly);
                }
                if carried > 0.0 {
                    lit.insert(coord);
                }
                if let Some(t) = next.get_mut(&coord) {
                    t.sky_lit = carried > 0.0 || t.sky_lit;
                }
            }
        }
    }
    lit
}

/// Constant moisture source wherever a `Liquid` CA cell is present —
/// architecture report §4. Same shape and same reasoning as `apply_sky`
/// immediately above: forces `moisture` to `MAX_MOISTURE` at every field
/// block `rebuild_blocked` flagged this frame (`moisture_source`), run last
/// for the same "everything upstream overwrites unconditionally" reason, and
/// does **not** clear `fields_settled` — a body of standing water is a
/// stable condition, not a disturbance, and moving water already keeps its
/// own chunk (and therefore `active_chunk_count()`) awake independently,
/// which is what actually re-triggers `rebuild_blocked`'s scan and lets
/// `is_converged` notice a source appearing or disappearing.
///
/// Unlike `apply_sky`, this does **not** skip blocked field cells. A
/// shallow puddle resting on a thin floor — the overwhelmingly common case,
/// not an edge case — routinely shares its own coarse field block with the
/// ground right underneath it (`FIELD_SCALE` is 8 world cells; a 1-cell-deep
/// puddle and the floor it sits on are almost always within 8 cells of each
/// other), which is enough for `rebuild_blocked`'s over-blocking bias to
/// mark that whole block impassable. Gating on `!blocked` the way `apply_
/// sky` does silently zeroed out moisture for exactly the geometry this
/// channel most needs to get right — caught by `moss_spreads_over_damp_
/// stone_and_not_over_dry` going from "spreads over damp stone" to "spreads
/// over almost nothing" once that test switched from the old per-cell scan
/// to a real field read. The forced value still won't cross into an
/// *unblocked* neighbour via diffusion (`step_diffusion`'s own blocked-
/// neighbour substitution sees to that), so this only affects reads that
/// land inside the wet-but-blocked block itself — exactly where a puddle's
/// own dampness needs to still be visible.
fn apply_moisture_sources(coords: &[ChunkCoord], next: &mut HashMap<ChunkCoord, FieldTile>) {
    for &coord in coords {
        let tile = next.get_mut(&coord).expect("next was pre-populated with every coord in coords");
        for ly in 0..FIELD_TILE_SIZE {
            for lx in 0..FIELD_TILE_SIZE {
                // Two independent lower bounds meet here, and one `max`
                // serves both. The graded *source* level (this branch's
                // damp-soil work) raises humidity to at least its own
                // level — standing liquid still pins the block to
                // MAX_MOISTURE exactly as before (level 1.0), damp soil
                // lifts it part way without capping air that is humid for
                // some other reason. The authored *floor* (master's
                // aquifer) bounds what evaporation may take a cell down
                // to: moisture above it still diffuses and evaporates
                // normally, so rain and puddles behave as before and only
                // the drying-out is bounded. Applied here rather than
                // inside the diffusion loop so it lands after advection
                // too — wind may not blow the aquifer away. Where both
                // apply, the stronger bound simply wins; neither ever
                // pulls a reading down.
                let level = tile.moisture_source_local(lx, ly);
                let floor = tile.moisture_floor_local(lx, ly);
                let forced = (MAX_MOISTURE * level).max(floor);
                if forced > 0.0 {
                    let mut cell = tile.get_local(lx, ly);
                    if cell.moisture < forced {
                        cell.moisture = forced;
                        tile.set_local(lx, ly, cell);
                    }
                }
            }
        }
    }
}

/// A field cell counts as blocked when any CA-solid cell falls inside its
/// `FIELD_SCALE`-sided block. Biased toward over-blocking rather than
/// under-blocking: a field cell that is mostly open but partly solid still
/// stops air passing straight through, and the alternative (a fractional
/// "how solid" value feeding into partial blocking) is real hydraulic
/// modelling this coarse a grid was never trying to do.
/// Re-apply the glow-to-light seed for tiles that skipped `rebuild_blocked`.
///
/// **The one thing `rebuild_blocked` does on the carry-forward path that is a
/// write rather than a derivation**, and the reason this function exists
/// instead of the carry being a pure copy. That scan seeds each block's
/// `Material::glow` into the light channel *before* diffusion, so a glowing
/// lining gets a soft halo from the pass that softens every other light edge;
/// and the tile in `next` is freshly built with `light == 0`, because that is
/// what lets light *fall* again as the sun sets. Carry the arrays and skip
/// this, and every crystal in the world goes dark on the first frame its
/// chunk falls asleep — which no timing would show and no property test
/// asserts.
///
/// `has_glow` gates the whole tile, so this costs a bool check for the
/// overwhelming majority of tiles, which glow nowhere.
fn seed_light_from_glow(coords: &[ChunkCoord], next: &mut HashMap<ChunkCoord, FieldTile>) {
    for &coord in coords {
        let Some(tile) = next.get_mut(&coord) else { continue };
        if !tile.has_glow {
            continue;
        }
        for ly in 0..FIELD_TILE_SIZE {
            for lx in 0..FIELD_TILE_SIZE {
                let glow = tile.glow_local(lx, ly);
                if glow <= 0.0 {
                    continue;
                }
                // Max, never assignment — same rule as the scan this
                // replaces: sunlight down a shaft may already be brighter
                // than the crystal.
                let mut cell = tile.get_local(lx, ly);
                if cell.light < glow {
                    cell.light = glow;
                    tile.set_local(lx, ly, cell);
                }
            }
        }
    }
}

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
        // `has_glow` is a max over the scan below, so it has to start false
        // each solve or it can only ever latch on: `set_glow_local` raises
        // it and nothing else lowers it, and a tile cloned from its last
        // solve arrives with the old answer. Without this reset, mining out
        // the last of a lining would leave the renderer sampling the field
        // under that tile forever.
        tile.has_glow = false;
        // This scan is what makes the four arrays below real, and the flag is
        // what lets the next frame trust them.
        tile.derived_valid = true;
        let (ox, oy) = coord.origin();
        for ly in 0..FIELD_TILE_SIZE {
            for lx in 0..FIELD_TILE_SIZE {
                let bx0 = ox + lx * FIELD_SCALE;
                let by0 = oy + ly * FIELD_SCALE;
                let mut blocked = false;
                // Whether any `Liquid` CA cell falls in this block —
                // architecture §4's moisture source. Recorded in the same
                // scan `blocked` already runs, rather than a second pass
                // over every CA cell. Costs this function's own early exit,
                // though: a first attempt still broke out the moment `blocked`
                // went true, on the reasoning that a sealed block can't leak
                // moisture to a neighbour regardless of what's inside it —
                // true for diffusion *outward*, but wrong for a direct read
                // *of* that same block, which is exactly what a shallow puddle
                // resting on a thin floor needs (see `apply_moisture_sources`'s
                // own doc for the full story). `moss_spreads_over_damp_stone_
                // and_not_over_dry` caught it: the block containing this
                // test's own puddle also contains an unrelated solid wall
                // cell earlier in scan order than the water, so breaking on
                // the wall skipped the water entirely. Every block is now
                // scanned in full (bar the world-bounds case below, which
                // stays unconditionally true, since nothing outside the
                // world can ever be liquid) — a real, measured-and-
                // documented regression against issue #5/#6's own early-exit
                // for the dominant "solid ground" case; see the README's
                // Performance section for the honest numbers.
                //
                // No `break` anywhere in this loop any more, on either
                // condition — an independent review of an earlier version
                // (which still broke out on the out-of-bounds case) found
                // that leaves the exact same class of bug it was fixing one
                // level up: for a world whose size isn't a multiple of
                // `FIELD_SCALE`, a block straddling the boundary would abort
                // its *entire* scan on the first out-of-bounds cell it hit
                // (`dy = 0`, if the boundary runs vertically), silently
                // never examining the fully in-bounds rows below it where a
                // real `Liquid` cell could sit. Currently unreachable in
                // practice -- every `World::new` call site in this codebase
                // uses `FIELD_SCALE`-aligned dimensions -- but nothing
                // enforces that, so it stays fixed rather than relying on
                // that holding forever.
                // Strongest moisture source anywhere in this block.
                let mut moisture_level = 0.0f32;
                // Brightest glowing cell in this block (`Material::glow`) —
                // the local-light floor, gathered in the scan that is
                // already happening rather than a pass of its own.
                let mut glow_level = 0.0f32;
                // How many opaque cells deep each CA *column* of this block
                // is, counted in the scan that is already happening -- see
                // `FieldTile::transmission` for why depth per column, and
                // not cells filled per block, is the quantity a downward
                // ray cares about.
                let mut column_depth = [0u8; FIELD_SCALE as usize];
                for dy in 0..FIELD_SCALE {
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
                            column_depth[dx as usize] += 1;
                            continue;
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
                        // One registry fetch per cell, shared by every read
                        // below. This scan is the busiest standing cost in
                        // the field — it runs over every CA cell of every
                        // awake tile every solve — and the glow read was
                        // first landed as its own `materials.get(..)` per
                        // cell on top of the `kind` lookup and the
                        // conditional capacity one. Paired ascii runs showed
                        // the price: the river scene's spring-OFF mean rose
                        // from {12.770, 12.765} to {13.42..13.86} ms at
                        // 2048x640, with *zero* awake chunks — 78 unsettled
                        // field tiles re-solving was enough. Three fetches
                        // folded to one is cheaper than what the glow read
                        // was added to, not just cheaper than its first
                        // version.
                        let mat = world.materials.get(cell.material);
                        if matches!(mat.kind, super::material::MaterialKind::Solid | super::material::MaterialKind::Plant) {
                            blocked = true;
                            column_depth[dx as usize] += 1;
                        }
                        if mat.kind == super::material::MaterialKind::Liquid {
                            moisture_level = 1.0;
                        } else {
                            // Damp soil is a weaker source than standing
                            // water, in proportion to how much water it
                            // actually holds. `water_capacity == 0` (sand,
                            // gravel, anything that does not opt in) gives
                            // 0 and costs a single compare.
                            if mat.water_capacity > 0 {
                                let held = super::update::soil_moisture(cell) as f32 / mat.water_capacity as f32;
                                moisture_level = moisture_level.max(held);
                            }
                        }
                        glow_level = glow_level.max(mat.glow);
                    }
                }
                tile.set_blocked_local(lx, ly, blocked);
                // The block passes what its columns pass, averaged. Each
                // column's own depth goes through Beer-Lambert first, so a
                // deep narrow occluder and a shallow wide one come out
                // different — which is the entire reason this is per column.
                let transmission =
                    column_depth.iter().map(|&d| COLUMN_TRANSMISSION[d as usize]).sum::<f32>() / FIELD_SCALE as f32;
                tile.set_transmission_local(lx, ly, transmission);
                tile.set_moisture_source_local(lx, ly, moisture_level);
                tile.set_glow_local(lx, ly, glow_level);
                // Seed the light channel here, before diffusion runs, so a
                // glowing block gets its halo bled into the neighbouring
                // blocks by the same pass that softens every other light
                // edge — no glow-specific spreading code. Max, never
                // assignment: sunlight through a shaft may already be
                // brighter than the crystal.
                if glow_level > 0.0 {
                    let mut cell = tile.get_local(lx, ly);
                    if cell.light < glow_level {
                        cell.light = glow_level;
                        tile.set_local(lx, ly, cell);
                    }
                }
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
fn step_velocity(
    world: &World,
    coords: &[ChunkCoord],
    read: &[ChunkCoord],
    next: &mut HashMap<ChunkCoord, FieldTile>,
) {
    let old = world.fields_ref();
    let bounds = world.bounds();
    // Read the just-computed pressure from `next` as an immutable snapshot
    // before mutating `next` further. A separate cloned table sidesteps
    // borrowing `next` both immutably (for pressure) and mutably (to write
    // velocity) at once.
    //
    // Snapshotted over `read` — the solved tiles *plus one ring* — not over
    // `coords`. A cell at a tile edge samples into its neighbour, and `sample`
    // answers a tile the snapshot lacks with `AMBIENT` rather than with the
    // truth, so a snapshot narrowed to `coords` would read sleeping
    // neighbours as ambient air. Silent, and wrong in the direction that
    // looks plausible. The outer ring can sleep outside the subset `next`
    // now holds, so it falls back to the old map — the same state the
    // full-map clone used to carry forward for it.
    let new_pressure: HashMap<ChunkCoord, FieldTile> = read
        .iter()
        .filter_map(|&c| next.get(&c).or_else(|| old.get(&c)).map(|t| (c, t.clone())))
        .collect();

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
                //
                // One exception, folded into this closure and answered in
                // the same tile fetch as the blocked test itself
                // (`blocked_and_glow` — this is the diffusion hot path, and
                // both a separate light-only closure and a separate
                // `is_glow_source` call were written first; each cost an
                // extra `HashMap` fetch per neighbour, the second one
                // measurably in the river scene): **light reads through a
                // blocked neighbour that glows.** The moisture-source
                // exception below, replayed on the light channel. A glowing
                // lining is `Solid`, so its whole 8x8 block is `blocked`,
                // and the strict wall rule made the seed `rebuild_blocked`
                // writes there unshareable: the lining's own block sat at
                // 1.8 while the cavity air one block away read 0.0000004 —
                // lit crystal over pitch dark, no halo. (The paired-cavity
                // test caught it; nothing on screen had been looked at
                // yet.) Every non-glowing wall keeps the strict rule, so
                // the sealed-room optical guard is untouched: this admits
                // light *from a light source*, not through stone.
                let neighbour = |dx: i32, dy: i32| {
                    let (nx, ny) = (wx + dx, wy + dy);
                    let (blocked, glows) = blocked_and_glow(old, bounds, nx, ny);
                    if blocked {
                        if glows {
                            let mut wall = here;
                            wall.light = sample(old, bounds, nx, ny).light;
                            wall
                        } else {
                            here
                        }
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

                // **Moisture alone reads through a blocked neighbour that is
                // itself a moisture source, and the exception is not a
                // convenience — without it `apply_moisture_sources` sets a
                // source the rest of the solve then throws away.**
                //
                // `blocked` goes true for a whole 8x8 block if *one* cell in
                // it is `Solid`, and `rebuild_blocked` deliberately scans
                // the whole block anyway rather than breaking on the first
                // wall, precisely so "a shallow puddle resting on a thin
                // floor" still registers as a source (its own comment says
                // so). The generic wall rule above then discarded that: a
                // block holding both water and rock was pinned to
                // `MAX_MOISTURE` and forbidden to share a drop of it, so
                // water in a rock basin humidified nothing at all.
                //
                // Measured, over standing water four rows deep: with the
                // water's own block clear of stone the air a block above sat
                // at 2.310, and with the identical body shifted three rows
                // so its block also caught the floor, 0.000 — for a body 240
                // cells wide. The signal was not weak, it was absent, and it
                // was absent as a function of where an 8-row grid boundary
                // happened to fall.
                //
                // Physically this is the more honest of the two readings in
                // any case: wet ground is humid, which is exactly what the
                // graded damp-soil source already models for `Powder` (soil
                // is not `Solid`, so it never hit this in the first place).
                // Heat keeps the strict wall rule — a stone wall really is
                // insulating, and the sealed-room guard depends on it.
                // Light kept it too until `Material::glow` landed; it now
                // carries the same shape of exception (see `neighbour`
                // above) for blocked blocks that are themselves the light.
                let moisture_neighbour = |dx: i32, dy: i32| -> f32 {
                    let (nx, ny) = (wx + dx, wy + dy);
                    if is_blocked(old, bounds, nx, ny) && !is_moisture_source(old, bounds, nx, ny) {
                        here.moisture
                    } else {
                        sample(old, bounds, nx, ny).moisture
                    }
                };
                let neighbour_avg_m = (moisture_neighbour(-FIELD_SCALE, 0)
                    + moisture_neighbour(FIELD_SCALE, 0)
                    + moisture_neighbour(0, -FIELD_SCALE)
                    + moisture_neighbour(0, FIELD_SCALE))
                    / 4.0;

                let temperature = (here.temperature
                    + (neighbour_avg_t - here.temperature) * HEAT_DIFFUSION_RATE)
                    .clamp(MIN_TEMPERATURE, MAX_TEMPERATURE);
                let light = ((here.light + (neighbour_avg_l - here.light) * LIGHT_DIFFUSION_RATE) * LIGHT_DECAY)
                    .clamp(0.0, MAX_LIGHT);
                // Evaporation speeds up above ambient temperature -- the
                // "extra loop" architecture §4 calls out, tying moisture to
                // heat instead of decaying at one fixed rate regardless of
                // how hot the air is.
                let evaporation = (MOISTURE_BASE_DECAY
                    - (here.temperature - AMBIENT_TEMPERATURE as f32).max(0.0) * MOISTURE_EVAPORATION_PER_DEGREE)
                    .clamp(0.0, 1.0);
                let moisture = ((here.moisture + (neighbour_avg_m - here.moisture) * MOISTURE_DIFFUSION_RATE) * evaporation)
                    .clamp(0.0, MAX_MOISTURE);

                let mut cell = tile.get_local(lx, ly);
                cell.temperature = temperature;
                cell.light = light;
                cell.moisture = moisture;
                tile.set_local(lx, ly, cell);
            }
        }
    }
}

/// Semi-Lagrangian advection: trace each cell backward along the velocity
/// `next` now holds, sample the *old* snapshot there, and blend it in. This
/// is what makes wind actually carry smoke and heat sideways rather than only
/// diffusing outward symmetrically in place.
fn step_advection(
    coords: &[ChunkCoord],
    read: &[ChunkCoord],
    bounds: Option<Rect>,
    old: &HashMap<ChunkCoord, FieldTile>,
    next: &mut HashMap<ChunkCoord, FieldTile>,
) {
    // Snapshot `next` as it stands after pressure/velocity/diffusion, so the
    // sampling below reads a fixed pre-advection state rather than a mix of
    // advected and not-yet-advected cells depending on iteration order.
    // Over `read` rather than `coords` — see `step_velocity` for why, and
    // for why the ring falls back to the old map now that `next` holds only
    // the solved subset.
    let pre_advection: HashMap<ChunkCoord, FieldTile> = read
        .iter()
        .filter_map(|&c| next.get(&c).or_else(|| old.get(&c)).map(|t| (c, t.clone())))
        .collect();

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

                // Clamped to one *tile* of back-trace per axis — the width
                // of the halo, which is what makes the clamp necessary and
                // is therefore what sets its size.
                //
                // Velocity is damped but never bounded, so a blast peak (~86
                // measured) would back-trace dozens of field cells, past the
                // one-tile halo the awake set provides and into tiles this
                // frame never populated, which `sample` reports as `AMBIENT`.
                // A back-trace of up to one tile still lands inside the ring,
                // which is in `read` and is populated, so this is the largest
                // clamp the halo actually justifies.
                //
                // **It was `FIELD_SCALE` — one field cell — and that was a
                // physics change dressed up as a safety margin.** The comment
                // here used to claim it cost "only that very fast flow
                // transports at one cell per step instead of many". That was
                // wrong and never measured: an impulse in open ground stopped
                // dispersing almost entirely, and total |pressure| left in the
                // region around it after 200 steps went from 2.9 to 2177.8.
                // Pressure from a blast simply stayed where it was put. The
                // sealed-room test caught it and was misread as pre-existing
                // for most of a session, because the obvious suspect was the
                // sleeping this landed with -- and a probe on the awake set
                // showed all 16 of 16 chunks solving every step, which is what
                // finally pointed here instead.
                let max_step = (FIELD_TILE_SIZE * FIELD_SCALE * ADVECTION_MAX_TILES) as f32;
                let dx = (here.vx * FIELD_SCALE as f32).clamp(-max_step, max_step);
                let dy = (here.vy * FIELD_SCALE as f32).clamp(-max_step, max_step);
                let src_x = wx as f32 - dx;
                let src_y = wy as f32 - dy;
                let transported = sample_bilinear(&pre_advection, bounds, src_x, src_y, here);

                let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
                let blended = FieldCell {
                    pressure: lerp(here.pressure, transported.pressure, ADVECTION_BLEND),
                    vx: lerp(here.vx, transported.vx, ADVECTION_BLEND),
                    vy: lerp(here.vy, transported.vy, ADVECTION_BLEND),
                    temperature: lerp(here.temperature, transported.temperature, ADVECTION_BLEND),
                    light: lerp(here.light, transported.light, ADVECTION_BLEND),
                    moisture: lerp(here.moisture, transported.moisture, ADVECTION_BLEND),
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

    /// Carrying a settled chunk's derived arrays forward must be **exactly**
    /// a rescan, not approximately one.
    ///
    /// Stated as the equality itself rather than as any symptom of breaking
    /// it, because the first version of the carry broke it in a way no
    /// symptom-shaped test was looking for: `World::ensure_chunks_for`
    /// creates a blank `FieldTile` for every chunk up front, so "a previous
    /// tile exists" was true from frame one and the blank arrays got carried
    /// forever for any chunk already settled when the field first stepped.
    /// A whole-field hash over 3,600 frames of a generated world missed it
    /// (worldgen leaves every chunk dirty, so those first solves all
    /// rescanned); a hand-built scene, which is every test scene in the
    /// repo, hit it immediately.
    ///
    /// The scene is placed and settled *before* the first field step, which
    /// is the case that failed. Both a wall and a glowing crystal, because
    /// the four arrays fail independently -- `blocked`/`transmission` from
    /// the stone, `glow`/`has_glow` from the spar.
    #[test]
    fn a_carried_tile_holds_the_same_scan_a_rescan_would_have_produced() {
        let mut world = World::new(Rect::new(0, 0, 127, 127));
        let spar = world.materials.id_of("spar").expect("spar is compiled in");
        for y in 40..48 {
            for x in 40..48 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for y in 60..64 {
            for x in 60..64 {
                world.set(x, y, Cell::new(spar, 0));
            }
        }
        world.end_step();
        for _ in 0..200 {
            crate::sim::update::step(&mut world);
            step(&mut world);
        }

        let coord = ChunkCoord::containing(60, 60);
        assert!(
            world.chunk(coord).is_some_and(|c| c.is_settled()),
            "vacuous: the chunk never settled, so nothing was ever carried"
        );
        let carried = world.fields_ref().get(&coord).expect("resident chunk has a tile").clone();
        assert!(carried.derived_valid, "the tile never recorded a real scan");

        // The control: what `rebuild_blocked` says about this same world now.
        let mut fresh: HashMap<ChunkCoord, FieldTile> = HashMap::new();
        fresh.insert(coord, FieldTile::new());
        rebuild_blocked(&world, &[coord], &mut fresh);
        let scanned = &fresh[&coord];

        assert_eq!(carried.has_glow, scanned.has_glow, "has_glow diverged from a fresh scan");
        assert!(scanned.has_glow, "vacuous: the scene holds nothing that glows");
        for ly in 0..FIELD_TILE_SIZE {
            for lx in 0..FIELD_TILE_SIZE {
                assert_eq!(
                    carried.is_blocked_local(lx, ly),
                    scanned.is_blocked_local(lx, ly),
                    "blocked diverged at ({lx}, {ly})"
                );
                assert_eq!(
                    carried.transmission_local(lx, ly),
                    scanned.transmission_local(lx, ly),
                    "transmission diverged at ({lx}, {ly})"
                );
                assert_eq!(
                    carried.moisture_source_local(lx, ly),
                    scanned.moisture_source_local(lx, ly),
                    "moisture_source diverged at ({lx}, {ly})"
                );
                assert_eq!(carried.glow_local(lx, ly), scanned.glow_local(lx, ly), "glow diverged at ({lx}, {ly})");
            }
        }
        assert!(
            (0..FIELD_TILE_SIZE).any(|ly| (0..FIELD_TILE_SIZE).any(|lx| scanned.is_blocked_local(lx, ly))),
            "vacuous: the scene holds nothing solid"
        );
    }

    /// An impulse into a world whose momentum channels had gone quiet must
    /// still disperse.
    ///
    /// The hazard the zero-momentum fast path creates, and it is invisible to
    /// every existing gust test because those all run on worlds that never
    /// went quiet in the first place. `weather::gust` fires into **open air**
    /// -- deliberately, so the impulse has room to spread -- so no CA cell
    /// changes anywhere near it and nothing in the CA's own bookkeeping
    /// reports that the field has work to do again. If `World::paint_field`
    /// did not clear `momentum_zero`, the solver would skip the very passes
    /// meant to disperse the impulse and the gust would sit where it landed.
    ///
    /// Asserts spread, not merely "the value changed": the impulse is placed
    /// at one point and read several field cells away, which only pressure
    /// propagation can reach.
    #[test]
    fn an_impulse_still_disperses_after_the_field_has_gone_quiet() {
        // **No terrain at all, deliberately.** The skip is disarmed whenever
        // any CA chunk is awake, and a world with so much as a stone floor in
        // it is *raining on that floor* -- rain writes cells, cells wake
        // chunks, and the fast path never arms. Measured: with a floor,
        // `any_fluid` was true on every frame after the impulse and the
        // mutant below passed. An empty world has nothing for rain to land
        // on, so the chunks stay settled and the impulse is the only thing
        // happening.
        let mut world = World::new(Rect::new(0, 0, 255, 255));
        // **Seed 3, because the default seed is windy.** `weather::at` is a
        // pure function of `(seed, frame)` and seed 0 opens gusting, so a
        // bare test world quietly has a gale in it: measured, all 16 tiles
        // held zero momentum at frame 0 and none of them did by frame 80,
        // because every gust is an impulse and every impulse clears the flag
        // -- correct behaviour, and it would have read as this fast path
        // being broken. Seed 3 never crosses `GUST_THRESHOLD`.
        world.seed = 3;
        world.end_step();
        // Long enough for the momentum channels to reach the exact zero the
        // fast path waits for -- the point of the test is the state *after*
        // that, so check it was actually reached rather than assuming it.
        for _ in 0..400 {
            crate::sim::update::step(&mut world);
            step(&mut world);
        }
        // **Every** tile, not just the one under the impulse: the skip is
        // keyed on the whole read set, so one tile still holding momentum
        // anywhere nearby leaves it disarmed and the test proves nothing. A
        // first version checked only the impulse's own tile, and the mutant
        // with `disturb_momentum` deleted passed it.
        let armed = world.fields_ref().values().filter(|t| t.momentum_zero).count();
        assert_eq!(
            armed,
            world.fields_ref().len(),
            "vacuous: only {armed} tiles reached zero momentum, so the fast path was never armed"
        );

        world.add_pressure_impulse(128, 128, 4, 100.0);
        for _ in 0..24 {
            crate::sim::update::step(&mut world);
            step(&mut world);
        }
        let spread = world.field_at(128 + 40, 128).pressure;
        assert!(spread.abs() > 0.001, "the impulse never reached x+40: {spread} -- it was frozen where it landed");
    }

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

        println!("sealed {sealed_total} open {open_total}");
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

    /// A *point* source still falls off with distance — but this has to be
    /// asked underground now, and the reason is the point of `apply_sky`.
    ///
    /// The original version of this test placed a point light in open air
    /// and asserted the reading 72 rows below was dimmer. It was, because
    /// sky light itself used to decay with depth, so *everything* was
    /// dimmer further down. Once direct sunlight became a column cast, open
    /// air reads full brightness at any depth and swamps a point source
    /// entirely — the assertion failed, correctly, because it was measuring
    /// the sky's distance falloff rather than the source's.
    ///
    /// Sealed in rock the sky cannot reach, so what is left is the thing
    /// the test is named for.
    #[test]
    fn a_point_light_falls_off_with_distance_where_the_sky_cannot_reach() {
        let mut w = test_world();
        // A solid block with a hollow interior: no column from open sky
        // reaches inside it.
        // The rock above the cavity has to be genuinely thick: one blocked
        // field block only attenuates by `SKY_TRANSMISSION`, so a thin roof
        // leaves the cavity uniformly lit by transmitted sky and swamps the
        // point source all over again. 90 rows is ~11 blocks, i.e. 0.2^11.
        for x in 60..200 {
            for y in 40..250 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 100..160 {
            for y in 140..240 {
                w.set(x, y, Cell::EMPTY);
            }
        }
        w.add_light(128, 150, 2, 1.0);
        for _ in 0..10 {
            step(&mut w);
        }
        let near = w.field_at(128, 150).light;
        let far = w.field_at(128, 235).light;
        assert!(near > 0.0, "light source went dark");
        assert!(far < near, "a point light must still fall off with distance: near {near}, far {far}");
    }

    /// The invariant that replaced it: **clear air does not attenuate
    /// sunlight; occluders do.**
    ///
    /// Light used to be seeded on the topmost chunk's top row and carried
    /// down by diffusion, so illumination was a function of *distance from
    /// the world's top edge*. That gave a plant an unbounded reward for
    /// climbing and pinned every scene's trees against the world boundary —
    /// see `apply_sky` for the four symptoms and their measurements.
    #[test]
    fn open_air_is_equally_lit_at_any_depth_and_a_roof_still_shades() {
        let mut w = test_world();
        for _ in 0..40 {
            step(&mut w);
        }
        let high = w.field_at(128, 16).light;
        let low = w.field_at(128, 240).light;
        assert!(high > 0.0, "open sky went dark");
        assert!(
            (low - high).abs() < 1e-3,
            "clear air must not attenuate sunlight: {high} at row 16 against {low} at row 240"
        );

        // A roof, and the same column underneath it must go dark.
        let mut w = test_world();
        for x in 80..200 {
            for y in 100..108 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for _ in 0..40 {
            step(&mut w);
        }
        let above_roof = w.field_at(128, 64).light;
        let under_roof = w.field_at(128, 160).light;
        assert!(
            under_roof < above_roof * 0.5,
            "an occluder must shade what is beneath it: {above_roof} above against {under_roof} below"
        );
    }

    #[test]
    fn open_sky_reads_brighter_than_a_directly_blocked_cell() {
        // apply_sky (architecture report §2) is what resurrects moss shade-
        // seeking and tree phototropism -- both already read `field_at(..)
        // .light`, but had nothing to read before this, since nothing wrote
        // to the light channel except the isolated `add_light` case above.
        //
        // The probe still sits close to the sky row (one field row down),
        // even though `LIGHT_DECAY`'s own doc now describes real depth
        // penetration -- this is just checking that the sky boundary
        // condition itself works, not exercising the full depth range.
        let mut w = test_world();
        // One field cell's worth of solid rock, aligned to the field grid
        // (FIELD_SCALE = 8) so `rebuild_blocked` marks exactly this field
        // cell and no other.
        // **Nine field blocks wide, not one**, and the width is load-bearing
        // rather than incidental: lateral diffusion fills a narrow shadow
        // in from both sides within a few steps, so a single 8x8 occluder
        // leaves the block beneath it at 74% of open sky however opaque it
        // is. A shadow only exists if it is wider than the diffusion can
        // reach across, which is also true of the canopies this models.
        for x in 104..176 {
            for y in 8..16 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for _ in 0..50 {
            step(&mut w);
        }

        let open = w.field_at(20, 10).light; // open column, one field row below the sky
        let occluder = w.field_at(140, 10).light; // inside the solid block itself
        let under = w.field_at(140, 18).light; // beneath the middle of the slab
        let open_at_depth = w.field_at(20, 18).light; // same depth, open column
        assert!(open > 0.5, "the open column under the sky did not brighten: {open}");

        // **The invariant here was deliberately reversed**, and the old one
        // is worth stating because it looked obviously right: a solid field
        // block used to carry *no* light of its own. Physically that is
        // backwards — the light arriving at an occluder is exactly what
        // that occluder intercepts, and a leaf is an occluder. Under the
        // old rule every leaf inside a canopy read only whatever lateral
        // diffusion left behind, 88% of them measured under 0.05, and two
        // separate mechanisms were stuck on it: shade-driven abscission had
        // no threshold between "prunes nothing" and "defoliates the world",
        // and `light_weight` measured completely inert across 1,024 genomes
        // because there was no gradient left to steer by.
        //
        // What must still hold is the *gradient*: an occluder is lit by
        // what reaches it, and everything under it is dimmer.
        assert!(occluder > 0.5, "a solid block should read the light arriving at it, not zero: {occluder}");
        assert!(
            under < open_at_depth * 0.5,
            "the block under a solid one should be markedly darker than open sky at the same depth: {under} against {open_at_depth}"
        );
    }

    #[test]
    fn the_column_transmission_table_is_beer_lambert() {
        // The table is hand-written so the scan can stay a lookup; this is
        // what keeps it honest, and what fails if `SKY_TRANSMISSION` or
        // `FIELD_SCALE` moves without it.
        for (depth, &actual) in COLUMN_TRANSMISSION.iter().enumerate() {
            let expected = SKY_TRANSMISSION.powf(depth as f32 / FIELD_SCALE as f32);
            assert!((actual - expected).abs() < 1e-4, "COLUMN_TRANSMISSION[{depth}] is {actual}, Beer-Lambert says {expected}");
        }
        assert_eq!(COLUMN_TRANSMISSION[0], 1.0, "an empty column must pass everything");
        assert_eq!(
            COLUMN_TRANSMISSION[FIELD_SCALE as usize], SKY_TRANSMISSION,
            "a full column must pass exactly SKY_TRANSMISSION, or solid rock changes behaviour"
        );
    }

    /// **A one-cell-thick horizontal plate has to shade**, and under the
    /// occupancy rule this replaced it barely did: 8 cells over a 64-cell
    /// block read 12.5% full and passed 90% of the light, the same as a
    /// vertical 8-cell trunk. For a downward ray those two are nothing
    /// alike, and a flat canopy plate is the exact geometry the plant work
    /// keeps fighting — so the light model was under-charging the artifact
    /// it is supposed to bound.
    ///
    /// The pair of assertions is the point: the plate must shade *and* must
    /// not shade like rock. A columns-hit mask (the other candidate fix)
    /// passes the first and fails the second, which is why this test checks
    /// both ends rather than just "is it darker".
    #[test]
    fn a_one_cell_thick_plate_shades_without_shading_like_rock() {
        fn light_under(rows: i32) -> (f32, f32) {
            let mut w = test_world();
            // Nine field blocks wide, for the reason
            // `open_sky_reads_brighter_than_a_directly_blocked_cell` gives:
            // a narrow shadow fills in from both sides by diffusion within
            // a few steps, so a shadow only exists if it is wider than
            // diffusion can reach across.
            for x in 104..176 {
                for y in 16..(16 + rows) {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            for _ in 0..50 {
                step(&mut w);
            }
            // Below the occluder, and open sky at the same depth.
            (w.field_at(140, 40).light, w.field_at(20, 40).light)
        }

        let (under_plate, open) = light_under(1);
        let (under_slab, _) = light_under(8);

        assert!(under_plate < open * 0.95, "a one-cell plate should shade what is beneath it: {under_plate} against {open} in the open");
        assert!(
            under_slab < under_plate * 0.75,
            "a full 8-deep block must shade far harder than a one-cell plate, or transmission is not reading depth: \
             slab {under_slab}, plate {under_plate}"
        );
        // And the plate must stay recognisably brighter than rock -- the
        // binary-shade failure this whole channel exists to avoid.
        assert!(
            under_plate > under_slab * 1.5,
            "a one-cell plate should not shade like solid rock: plate {under_plate}, slab {under_slab}"
        );
    }

    #[test]
    fn sky_light_amplitude_cycles_between_the_night_floor_and_max_light() {
        // Architecture §5h. frame = 0 is noon (this file's own convention:
        // `cos(0) = 1`, full daylight) rather than sunrise or midnight --
        // matters only for reading these numbers, not for correctness.
        assert_eq!(sky_light_amplitude(0), MAX_LIGHT, "frame 0 should read as full daylight");
        assert_eq!(
            sky_light_amplitude(DAY_NIGHT_PERIOD_FRAMES / 2),
            NIGHT_LIGHT_FLOOR,
            "half a period later should be the deepest point of night"
        );
        assert_eq!(
            sky_light_amplitude(DAY_NIGHT_PERIOD_FRAMES),
            sky_light_amplitude(0),
            "a full period should bring the cycle back to exactly where it started"
        );
        // An eighth of the way through the cycle -- comfortably inside the
        // "day" half (which spans phase -0.25..0.25) rather than sitting
        // exactly on its edge the way the quarter-period point does (where
        // cos hits exactly zero and the reading is indistinguishable from
        // the night floor).
        let mid_morning = sky_light_amplitude(DAY_NIGHT_PERIOD_FRAMES / 8);
        assert!(
            mid_morning > NIGHT_LIGHT_FLOOR && mid_morning < MAX_LIGHT,
            "a mid-morning point should read strictly between the floor and the peak: {mid_morning}"
        );
    }

    #[test]
    fn the_sky_keeps_cycling_through_day_and_night_even_after_the_field_goes_quiet() {
        // Regression for the interaction §5h introduces with issue #4 (field
        // sleeping): apply_sky's forced value now changes with world.frame
        // alone, with no CA write to keep active_chunk_count() nonzero the
        // way every other disturbance this file's early-return relies on
        // does. Without the amplitude-delta check added alongside this
        // channel, a field that settled at noon and then saw the CA grid go
        // fully quiet would stay frozen at noon's brightness forever, never
        // producing a real day/night cycle for any scene that isn't
        // actively churning. `field::step` still has to be called every
        // frame for this to work (same as the real app's own `App::update`,
        // which calls `world.step_fields()` unconditionally every frame) --
        // what's being tested is that most of those calls can stay cheap
        // no-ops while the ones during an actual dawn/dusk transition still
        // do real work.
        // `LIGHT_DECAY` moved much closer to 1.0 (0.997) to let sunlight
        // reach real depth rather than only a local glow (owner request:
        // "add more light to the env" rather than requiring every tree to
        // be planted within a couple of field rows of open sky) -- the
        // direct cost is that genuine convergence to a static sky
        // amplitude now takes roughly 100x longer than the old decay's
        // own ~50 steps, since each step's marginal correction shrinks
        // far more slowly. Still bounded and still real, just slower; the
        // field-sleep optimization (issue #4) stays correct either way,
        // it just spends more real frames awake near each day/night
        // peak/trough before qualifying. A live perf re-check against the
        // stress scene is worth doing before this ships broadly, not done
        // here.
        let mut w = test_world();
        for _ in 0..5000 {
            step(&mut w); // converge to frame 0's (noon) amplitude
        }
        assert!(w.fields_settled(), "test setup should have reached a converged, quiet field");
        let noon = w.field_at(20, 4).light; // inside the sky row itself (field row 0 = world y 0..7)
        assert_eq!(noon, MAX_LIGHT, "test setup should have settled at full daylight");

        for _ in 0..(DAY_NIGHT_PERIOD_FRAMES / 2) {
            w.begin_step();
            step(&mut w);
            w.end_step();
        }
        let midnight = w.field_at(20, 4).light;
        assert!(
            (midnight - NIGHT_LIGHT_FLOOR).abs() < SETTLE_EPSILON_LIGHT * 2.0,
            "the sky did not reach night despite stepping through half a full cycle: noon={noon}, midnight={midnight}"
        );
    }

    #[test]
    fn standing_water_is_a_moisture_source_that_diffuses_and_decays_with_distance() {
        // Architecture §4. `apply_moisture_sources` forces `MAX_MOISTURE`
        // into every field block `rebuild_blocked` flagged as containing a
        // `Liquid` CA cell, every step -- this is the replacement for the
        // two hand-rolled O(r^2) grid scans (`is_damp`, `strongest_water_
        // pull` in `plant.rs`) that used to detect water this way privately.
        let mut w = test_world();
        for x in 124..132 {
            for y in 124..132 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        for _ in 0..50 {
            step(&mut w);
        }

        let near = w.field_at(128, 128).moisture;
        let far = w.field_at(128, 220).moisture;
        assert!(near > 0.0, "standing water did not register as a moisture source");
        assert!(far < near, "moisture did not fall off with distance");
    }

    #[test]
    fn moisture_does_not_leak_through_a_sealed_wall() {
        // Same reasoning as `heat_does_not_leak_through_a_sealed_wall_via_
        // diffusion` below, one channel over: a sealed stone room should
        // stay dry even sitting right next to standing water, since
        // `step_diffusion` is wall-aware for every channel it carries, not
        // just temperature and light.
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
        for x in 40..90 {
            for y in 124..132 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        for _ in 0..50 {
            step(&mut w);
        }

        let inside = w.field_at(128, 128).moisture;
        assert_eq!(inside, 0.0, "moisture leaked through a sealed wall: {inside}");
    }

    #[test]
    fn a_liquid_cell_is_detected_even_in_a_field_block_that_straddles_the_world_edge() {
        // Regression an independent review of §4 flagged: `rebuild_blocked`'s
        // scan used to `break` its *entire* block scan (both loops) the
        // moment it found an out-of-bounds cell, on the same "nothing more
        // to learn" reasoning that justifies stopping on a solid cell. That
        // reasoning doesn't hold here -- a world whose size isn't a multiple
        // of FIELD_SCALE has a block straddling its edge, and a *vertical*
        // edge puts an out-of-bounds cell at the same dx in every row
        // (dy = 0's own row hits it first), aborting before any later row
        // -- fully in-bounds -- is ever scanned. A `Liquid` cell sitting in
        // one of those later rows would be silently missed.
        //
        // Width 102 (0..=101) is not a multiple of `FIELD_SCALE` (8); height
        // 104 (0..=103) is, isolating the straddle to the x edge only so
        // this doesn't also trip a y-edge case at the same time.
        let mut w = World::new(Rect::new(0, 0, 101, 103));
        // Field block (12, 12) spans world (96..=103, 96..=103); x 102..103
        // is out of bounds (max_x = 101), x 96..101 is not. Row dy=0 (y=96)
        // hits the out-of-bounds column first; the water sits at dy=3
        // (y=99), a fully in-bounds row the old `break` would never reach.
        w.set(96, 99, Cell::new(material::WATER, 0));
        step(&mut w);

        let moisture = w.field_at(96, 99).moisture;
        assert!(moisture > 0.0, "a liquid cell in a later row of an edge-straddling block was not detected as a moisture source");
    }

    #[test]
    fn deplete_moisture_lowers_the_local_reading_and_floors_at_zero() {
        // Architecture §5g -- a root's own write to the channel it reads,
        // the mechanism that turns moisture from read-only into a loop
        // (a neighbouring root's `moisture_pull` can now actually notice
        // another root draining a shared puddle).
        let mut w = test_world();
        w.set(128, 128, Cell::new(material::WATER, 0));
        step(&mut w); // apply_moisture_sources forces this block to MAX_MOISTURE
        let before = w.field_at(128, 128).moisture;
        assert!(before > 0.0, "test setup should have produced a moisture source");

        // Remove the literal water cell too, mirroring what a real drink
        // does (`root_tip_tick`'s own `world.set(.., Cell::EMPTY)`) --
        // otherwise `apply_moisture_sources` would just force this straight
        // back up to `MAX_MOISTURE` on the *next* step, masking the write
        // this test exists to check. Proven, not just asserted: step the
        // field again after depleting and confirm it stays down rather than
        // snapping back, which is what would happen if the water cell were
        // still there (`rebuild_blocked` would still see it as a source).
        w.set(128, 128, Cell::EMPTY);
        w.deplete_moisture(128, 128, 1, 1.0);
        let after = w.field_at(128, 128).moisture;
        assert!(after < before, "deplete_moisture did not lower the local reading: before={before}, after={after}");

        step(&mut w);
        let after_next_step = w.field_at(128, 128).moisture;
        assert!(
            after_next_step < before,
            "moisture snapped back up on the next step -- the water cell removal isn't actually preventing \
             apply_moisture_sources from re-forcing it: before={before}, after depleting and stepping={after_next_step}"
        );

        w.deplete_moisture(128, 128, 1, 1000.0); // far more than the current reading
        assert_eq!(w.field_at(128, 128).moisture, 0.0, "deplete_moisture should floor at zero, not go negative");
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
    fn field_at_bilinear_resolves_what_field_at_flattens_within_one_block() {
        // Architecture §6a's actual claim: `field_at` (block-nearest) reads
        // byte-identical values for any two positions sharing a `FIELD_
        // SCALE`-sided block, which is precisely the geometry of a
        // gradient-follower's own short-range candidates (a worm's ±1-cell
        // neighbours, a tree tip's 4-pixel-up phototropism probe). Both land
        // inside the same 8-wide block ~7 times out of 8, degenerating
        // `min_by`-style gradient descent into "always pick the first
        // candidate." `field_at_bilinear` is what's supposed to fix that.
        let mut w = test_world();
        w.add_heat(64, 64, 1, 300.0); // field cell containing (64, 64)
        for _ in 0..3 {
            step(&mut w);
        }

        // Both inside field block [56, 64) -- the one immediately left of
        // the heated block [64, 72) -- but at opposite edges of it: `a` near
        // the far edge, `b` right up against the boundary with the hot
        // block. `field_at` reads the same stored value for the whole
        // block regardless of position within it, so both must read
        // identically through it; `field_at_bilinear` should not.
        let a = (57, 64);
        let b = (63, 64);
        assert_eq!(
            w.field_at(a.0, a.1),
            w.field_at(b.0, b.1),
            "test setup assumption broke: these two positions should read identically through field_at"
        );

        let ta = w.field_at_bilinear(a.0 as f32, a.1 as f32).temperature;
        let tb = w.field_at_bilinear(b.0 as f32, b.1 as f32).temperature;
        assert!(
            (ta - tb).abs() > 0.01,
            "field_at_bilinear should have told the two positions apart: a={ta}, b={tb}"
        );
        // `b` sits right against the boundary with the heated block; `a`
        // sits near its far edge.
        assert!(tb > ta, "the position closer to the heat source's block should read hotter: a={ta}, b={tb}");
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

    #[test]
    fn a_converged_and_ca_quiet_field_stops_recomputing() {
        // Issue #4: the actual point of field sleeping. `end_step` clears
        // the "freshly created, everything dirty" state every new `World`
        // starts in (see `Chunk::new`) without needing a real CA sweep to
        // run -- standing in for a world where the CA grid genuinely has
        // nothing left to do, the same as a settled sandbox scene.
        let mut w = test_world();
        w.add_pressure_impulse(128, 128, 4, 50.0);
        w.end_step();

        for _ in 0..2000 {
            step(&mut w);
        }
        assert_eq!(w.active_chunk_count(), 0, "test setup should have left the CA side quiet");
        assert!(w.fields_settled(), "an isolated pressure impulse should have converged to a fixed point by now");

        let before = w.field_at(128, 128);
        step(&mut w); // should be skipped entirely, not recomputed
        let after = w.field_at(128, 128);
        assert_eq!(before, after, "a settled, CA-quiet field changed after a step that should have been skipped");
    }

    #[test]
    fn an_impulse_wakes_an_already_settled_field() {
        // The other half of issue #4: a disturbance arriving from outside
        // field::step's own solve (add_pressure_impulse, add_heat, add_light,
        // add_heat_local) must still be able to wake an already-converged
        // field, or it would sit unprocessed the next time step() sees zero
        // CA activity and returns early.
        let mut w = test_world();
        w.end_step();
        // Not just one step: the sky light source (architecture §2) needs
        // several frames of diffusion to reach a fixed point throughout the
        // world's full chunk depth, same as `stays_bounded_over_ten_
        // thousand_frames`'s own reasoning for using more than one step.
        for _ in 0..500 {
            step(&mut w);
        }
        assert!(w.fields_settled(), "an undisturbed field should have converged to a fixed point by now");

        w.add_pressure_impulse(128, 128, 4, 200.0);
        assert!(!w.fields_settled(), "add_pressure_impulse should have cleared the settled flag immediately");

        step(&mut w);
        assert!(w.field_at(128, 128).pressure.abs() > 1.0, "the impulse was never actually solved after waking the field");
    }

    /// A world wide enough that a disturbance at one end has sleeping tiles
    /// between it and the far end.
    fn wide_world() -> World {
        World::new(Rect::new(0, 0, 1023, 255))
    }

    #[test]
    fn a_disturbance_in_open_ground_disperses_rather_than_freezing() {
        // **The guard the field did not have, and the absence of which cost
        // most of a session.**
        //
        // Every other field test here asserts that a disturbance *arrives*
        // somewhere. None asserted that one *goes away*. Those are different
        // claims, and the difference is exactly where two separate bugs
        // lived: an advection clamp that left blast pressure sitting where
        // it was put forever (2.9 -> 2177.8 and every test still green), and
        // a weather gust built as a pressure monopole that never
        // reconverged. Both propagated perfectly. Neither dissipated.
        //
        // Stated as dispersal *ratio* rather than an absolute, so it survives
        // retuning of the impulse, the damping and the coupling constants —
        // what it pins is the shape of the behaviour, not its magnitude.
        let region = |w: &World, half: i32| {
            let mut total = 0.0;
            let mut y = 128 - half;
            while y <= 128 + half {
                let mut x = 128 - half;
                while x <= 128 + half {
                    total += w.field_at(x, y).pressure.abs();
                    x += super::FIELD_SCALE;
                }
                y += super::FIELD_SCALE;
            }
            total
        };

        let mut w = test_world();
        w.add_pressure_impulse(128, 128, 4, 100.0);
        step(&mut w);
        let near_start = region(&w, 28);
        assert!(near_start > 1.0, "the impulse never reached the field at all, so nothing below means anything");

        for _ in 0..200 {
            step(&mut w);
        }
        let near_end = region(&w, 28);
        println!("open ground: {near_start:.3} in the impulse region -> {near_end:.3} after 200 steps");
        assert!(
            near_end < near_start * 0.25,
            "pressure in open ground did not disperse ({near_start:.3} -> {near_end:.3}).              Nothing bounds or removes pressure except spreading out, so a reading that stays              high means transport has been broken -- check the advection back-trace clamp              (`ADVECTION_MAX_TILES`) before anything else, which is what it was last time."
        );
    }

    #[test]
    fn a_disturbance_crosses_sleeping_regions_exactly_as_if_nothing_slept() {
        // **The test the per-tile solve exists to be checked by, and the only
        // shape that can catch its failure mode.**
        //
        // Solving only the awake tiles is a bet that the awake set plus its
        // halo is everywhere the disturbance can reach this frame. Lose that
        // bet and the field does not crash, does not warn, and does not look
        // wrong — it quietly stops carrying the wave, and every reading past
        // the gap is a plausible number that is simply false. A test asserting
        // "the field settles" passes trivially on a field that never woke; a
        // test asserting an absolute pressure at the far end encodes whatever
        // the solver happened to do the day it was written.
        //
        // So this is a **paired comparison** against the same disturbance in a
        // world where nothing is allowed to sleep — achieved by re-dirtying
        // every chunk each frame, which forces every tile into the awake set
        // and reproduces the old whole-world behaviour exactly. Any divergence
        // is the subsetting losing information, whatever the absolute values.
        let mut sleeping = wide_world();
        let mut control = wide_world();
        for w in [&mut sleeping, &mut control] {
            w.end_step();
            for _ in 0..400 {
                step(w);
            }
        }
        assert!(sleeping.fields_settled(), "test setup: the field should have converged before the impulse");

        sleeping.add_pressure_impulse(64, 128, 8, 400.0);
        control.add_pressure_impulse(64, 128, 8, 400.0);

        for _ in 0..120 {
            step(&mut sleeping);
            // The control never sleeps: waking every chunk keeps every tile in
            // the awake set, which is the pre-subsetting solve.
            control.wake_all();
            step(&mut control);
        }

        // Sampled right across the world, well past the impulse, so anything
        // the halo failed to carry shows up as a mismatch somewhere.
        for x in (16..1008).step_by(48) {
            let a = sleeping.field_at(x, 128);
            let b = control.field_at(x, 128);
            assert!(
                (a.pressure - b.pressure).abs() < 0.5,
                "pressure diverged at x {x}: sleeping {:.3} vs never-sleeping control {:.3}                  -- the awake set is not carrying the disturbance",
                a.pressure,
                b.pressure
            );
        }
    }

    #[test]
    fn material_written_into_a_sleeping_region_is_noticed() {
        // `rebuild_blocked` only runs for awake tiles now, so a wall built far
        // from any other activity has to wake its own tile. It does, via the CA
        // chunk being dirtied by the write — but that is exactly the kind of
        // coupling that is easy to lose, and losing it means the field treats
        // solid rock as open air indefinitely.
        let mut w = wide_world();
        w.end_step();
        for _ in 0..400 {
            step(&mut w);
        }
        assert!(w.fields_settled(), "test setup: expected a converged field");

        let far = 900;
        for y in 120..136 {
            for x in far..(far + 16) {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        w.end_step();
        step(&mut w);
        assert!(
            w.field_is_blocked(far + 8, 128),
            "a wall built in a sleeping region never reached the field's occupancy map"
        );
    }

    #[test]
    fn a_quiet_world_solves_nothing() {
        // The saving, asserted rather than assumed: once every tile has
        // converged the solve should touch none of them. Without this, a
        // regression that quietly re-woke everything would show up only as a
        // frame-time number nobody was watching.
        let mut w = wide_world();
        w.end_step();
        for _ in 0..400 {
            step(&mut w);
        }
        assert!(w.fields_settled(), "expected a converged field");
        let awake = w.fields_ref().values().filter(|t| !t.settled()).count();
        assert_eq!(awake, 0, "{awake} tiles are still marked unsettled in a quiet world");
    }
}

#[cfg(test)]
mod light_depth_probe {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::world::World;

    /// Prints the light profile against depth in a **completely empty**
    /// world — no occluders anywhere, so whatever falls off here is the
    /// model attenuating through *air*.
    ///
    /// `#[ignore]`d because it prints rather than asserts, and kept because
    /// it is the measurement that found `LIGHT_DECAY` was the binding
    /// constraint on how tall a plant could ever grow. At the previous
    /// 0.997 it read 0.16 at depth 128 — below `Germinate`'s own 0.1 gate
    /// by depth ~145, in vacuum. Reach for this before tuning anything
    /// about plant height, and before adding sky to a scene.
    ///
    /// ```text
    /// cargo test --lib print_light_versus_depth -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn print_light_versus_depth() {
        let mut w = World::new(Rect::new(0, 0, 511, 319));
        for _ in 0..4000 {
            step(&mut w);
        }
        println!("depth  light");
        for y in (0..320).step_by(16) {
            println!("{y:5}  {:.4}", w.field_at(256, y).light);
        }
    }
}

#[cfg(test)]
mod glow_tests {
    use super::*;
    use crate::sim::cell::Cell;
    use crate::sim::chunk::{ChunkCoord, Rect};
    use crate::sim::material;
    use crate::sim::world::World;

    /// A deep sealed cavity with a crystal floor: the paired-comparison
    /// scene for `Material::glow`. Everything about the two cavities is
    /// identical except the lining, so every light difference between them
    /// is the glow — the same cancellation argument as the rooted-bank
    /// tests.
    fn glow_world() -> (World, crate::sim::material::MaterialId) {
        let mut w = World::new(Rect::new(0, 0, 255, 255));
        let crystal = w.materials.id_of("crystal").expect("crystal ships in the registry");
        // Stone from y=100 down: 100+ rows of occluder, so no daylight
        // reaches either cavity and the only light down there is the glow.
        for y in 100..256 {
            for x in 0..256 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        // Two cavities in chunk row y=2..3, one per chunk column, far
        // enough apart (96 cells, three tiles) that the halo from one
        // cannot reach the other.
        for (x0, lined) in [(96, true), (192, false)] {
            for y in 180..200 {
                for x in x0..x0 + 24 {
                    w.set(x, y, Cell::EMPTY);
                }
            }
            if lined {
                for y in 200..203 {
                    for x in x0 + 4..x0 + 20 {
                        w.set(x, y, Cell::new(crystal, 0));
                    }
                }
            }
        }
        (w, crystal)
    }

    #[test]
    fn a_glowing_lining_lights_its_cavity_and_the_tile_still_sleeps() {
        let (mut w, _) = glow_world();
        for _ in 0..300 {
            step(&mut w);
        }

        // The lining's own block holds at least a substantial fraction of
        // crystal's 1.8 seed — diffusion averages the seeded block against
        // its dimmer neighbours after seeding, so the converged value sits
        // below the raw floor, and the bound is set with headroom under a
        // measured ~1.5, not on it.
        let at_lining = w.field_at(104, 201).light;
        assert!(at_lining > 0.9, "the lining's own block should be lit: {at_lining}");
        // The halo: cavity air a block above the lining, against the same
        // spot in the unlined cavity. This is the difference the renderer
        // draws.
        let lit_air = w.field_at(104, 195).light;
        let dark_air = w.field_at(200, 195).light;
        assert!(
            lit_air > dark_air + 0.05,
            "the halo should light the lined cavity's air above the unlined one's: lit {lit_air}, dark {dark_air}"
        );

        // `has_glow` marks exactly the tiles holding crystal, and the lit
        // tile still reaches its fixed point — a glow is a static floor,
        // not a permanent workload. (The whole design rests on this: a
        // light that kept its tile awake would fail the "and then it
        // stops" rule.)
        let lined_tile = w.fields_ref().get(&ChunkCoord::new(1, 3)).expect("tile exists");
        assert!(lined_tile.has_glow, "the tile holding the lining should be marked");
        assert!(lined_tile.settled(), "a glow-lit tile must still converge and sleep");
        let far_tile = w.fields_ref().get(&ChunkCoord::new(3, 3)).expect("tile exists");
        assert!(!far_tile.has_glow, "a tile with no glowing cell must not be marked");
    }

    #[test]
    fn mining_out_the_lining_puts_the_dark_back() {
        let (mut w, crystal) = glow_world();
        for _ in 0..300 {
            step(&mut w);
        }
        assert!(w.field_at(104, 201).light > 0.9, "precondition: the lining lit up");

        // Dig out every crystal cell (to empty, as mining would).
        for y in 200..203 {
            for x in 100..120 {
                if w.get(x, y).material == crystal {
                    w.set(x, y, Cell::EMPTY);
                }
            }
        }
        for _ in 0..300 {
            step(&mut w);
        }

        // The seed is gone, so decay wins: the cavity goes back to dark and
        // the tile drops its mark — `has_glow` must reset per solve, not
        // latch (the renderer samples the field under marked tiles forever
        // otherwise).
        let after = w.field_at(104, 201).light;
        assert!(after < 0.1, "light should decay away once the lining is gone: {after}");
        let tile = w.fields_ref().get(&ChunkCoord::new(1, 3)).expect("tile exists");
        assert!(!tile.has_glow, "has_glow must clear on the solve after the crystal goes");
    }
}
