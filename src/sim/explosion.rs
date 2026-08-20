//! M15: explosions, built from the three systems that came before — the M13
//! field (pressure impulse, shock propagation), M14 (heat, ignition), and M7
//! (debris as free particles).
//!
//! Per the plan: "an explosion writes three things: a pressure impulse into
//! the field, a temperature spike, and a radius of cells converted to free
//! particles or vacuum."
//!
//! # What a diagnosis pass changed, and why
//!
//! `Reports/explosion-mechanics-diagnosis.md` recorded the measurements
//! behind everything below; the short version is that the original one-frame
//! version had three structural problems that no amount of constant-tuning
//! would reach.
//!
//! - **It happened entirely on one frame.** An explosion is a *sequence* —
//!   flash, expanding front, debris, residue — and this had no time axis at
//!   all. [`Blast`] now expands over [`Tuning::duration`] frames.
//! - **Two of its three writes were inert.** Field pressure has exactly one
//!   gameplay consumer (`debris_velocity`, read at trigger time before the
//!   field has stepped once), and the field *temperature* spike was read by
//!   nothing that could ignite or glow: `fire::diffuse_heat` deliberately
//!   does not read the field, and both `fire::try_ignite` and `render.rs`'s
//!   heat glow key off `Cell::temperature()`, the per-CA-cell value. Every
//!   flame came from `World::ignite_circle`, the M14 *debug* force-ignite
//!   tool, which ignores `flammability` entirely — so stone burned. The
//!   fireball now writes CA cell temperature (see `scorch`), which glows,
//!   respects `ignition_temperature`, and lets `fire.rs` do the igniting.
//! - **Nothing could move through material.** Measured on a flat sand bed:
//!   material thrown clear of the blast fell to *exactly zero* past ~15
//!   cells of cover, and to zero at every depth in water, because a free
//!   particle lands the instant its next substep is occupied and a buried
//!   blast is enclosed on all sides. `particle::Particle::pierce` is the
//!   answer to that one; see its own doc.

use super::cell::Cell;
use super::field::FIELD_SCALE;
use super::material;
use super::particle::ParticleSystem;
use super::rng;
use super::world::World;

/// Offset applied to the *x* input of the *y* jitter sample, so a cell's x
/// and y jitter values are not the same number twice. Without this, jitter
/// would only ever push diagonally (`vx`/`vy` jitter identically), not
/// scatter in every direction. Arbitrary — any fixed nonzero offset works.
const JITTER_AXIS_OFFSET: i32 = 7919;

/// Every live-adjustable number an explosion has, in one place.
///
/// These were module constants until the tuning panel needed to reach them.
/// Their documented reasoning lives on each field; the values are still what
/// the diagnosis pass measured or picked by eye, not anything physical —
/// same honesty as the rest of this engine's constants.
///
/// `#[serde(default)]` on the struct: this is persisted to
/// `assets/explosion.ron` by the tuning panel, and a file written by an
/// older build — or hand-edited down to only the two fields someone cares
/// about — must still load, filling the rest from `Default`.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Tuning {
    /// Blast radius in world cells, for the player's own `X` key.
    ///
    /// **Deliberately not the brush radius**, which is what `App::explode`
    /// used to pass. The brush defaults to 6, and a 6-cell blast excavates
    /// 113 cells: measured, that is four faint orange smudges on an
    /// undisturbed surface with *zero* particles still in flight two frames
    /// later. A sensible painting default is a terrible explosion default,
    /// and no amount of correct physics on 113 cells reads as an explosion.
    pub radius: f32,
    /// Feeds the pressure impulse, debris speed, the heat spike (scaled by
    /// `heat_fraction`) and the pierce budget.
    pub strength: f32,
    /// Frames the cavity takes to expand from nothing to `radius`. `1.0`
    /// reproduces the original instantaneous behaviour exactly.
    ///
    /// This is the single most valuable number here. Beyond looking like a
    /// detonation rather than a deletion, staging is what lets debris
    /// *leave*: material cleared on a later frame launches into the cavity
    /// earlier frames already opened, instead of every grain being thrown
    /// simultaneously into material that is still solid.
    pub duration: f32,
    /// Fraction of `radius` that genuinely vaporizes — no debris, nothing
    /// left. Small and deliberate: real high explosives do pulverize a small
    /// core, but an explosion's visual signature is *material flying
    /// outward*, not a clean hole. An earlier version instead rolled
    /// `1.0 - sqrt(dist / radius)` odds of debris per cell, which — since a
    /// circle's area is dominated by its outer band — vaporized most of the
    /// affected area with nothing to show for it, reading as "a clean circle
    /// disappears, thin ring of sparks."
    pub vaporize_fraction: f32,
    /// Chance a cleared cell becomes a debris particle rather than simply
    /// vanishing, outside the vaporize core.
    ///
    /// Was effectively 1.0 (every cell, unconditionally) — which at radius
    /// 20 spawns ~2500 particles, of which 86% landed again on the very
    /// first frame. Fewer, longer-lived debris both looks better and costs
    /// less; this is the one item on the diagnosis list whose frame cost is
    /// *negative*.
    pub debris_fraction: f32,
    /// How far past `radius` the shockwave still has a chance to pick up
    /// loose material, as a multiple of `radius`. `1.0` would mean no
    /// shockwave at all (the annulus is empty).
    pub shockwave_multiplier: f32,
    /// Fraction of `radius` the scorch ring extends past the crater — the
    /// "fireball," matching the everyday intuition that a blast reaches
    /// further than the flame it leaves behind.
    pub fireball_fraction: f32,
    /// Peak CA cell temperature written into the scorch ring, in Celsius.
    ///
    /// `render.rs` maps cell temperature onto its fire tint over
    /// `HEAT_GLOW_RANGE` (400 degrees above ambient), so anything past ~420
    /// draws at the top of that ramp; the excess buys ignition headroom
    /// rather than extra brightness. Materials with a finite
    /// `ignition_temperature` below this catch fire through `fire.rs`'s own
    /// deterministic path; materials without one (stone) just glow and cool,
    /// which is the behaviour the old `ignite_circle` call got wrong.
    pub flash_temperature: f32,
    /// Chance a cleared cell is backfilled with `SMOKE` instead of left
    /// empty. Nothing else in the simulation has ever produced a smoke cell
    /// — not fire, not explosions — despite the material existing, rising,
    /// and `field.rs`'s advection describing itself as what carries smoke on
    /// wind. This is the blast's residue: without it, once debris lands and
    /// the glow fades there is no trace anything happened.
    ///
    /// The one item here with a genuine ongoing frame cost: gas cells keep
    /// their chunk awake while they rise, so this is deliberately a fraction
    /// of the crater rather than all of it.
    pub smoke_fraction: f32,
    /// How much of `strength` becomes the field's heat spike. Heat needs to
    /// be smaller in absolute terms — `strength` values large enough to
    /// throw debris convincingly would otherwise overshoot
    /// `field::MAX_TEMPERATURE` immediately, clamping rather than spiking.
    pub heat_fraction: f32,
    /// Base debris speed per unit `strength`, flat across the blast radius;
    /// only *direction* varies with position (`debris_velocity`'s own
    /// pressure-gradient read). Picked by eye against `App::spawn_burst`'s
    /// 3.0–6.0 range, not measured against anything physical.
    pub speed_per_strength: f32,
    /// Scales the position-keyed jitter added to each cell's launch
    /// velocity, as a fraction of that cell's own computed speed — not a
    /// flat value, and deliberately **not** scaled by raw `strength`.
    /// `strength` values large enough to throw debris convincingly already
    /// push speed at or past `particle::MAX_SPEED_PER_AXIS`, so a
    /// `* strength` jitter term would pin every particle to that clamp and
    /// make debris *more* uniform, not less — caught during planning, before
    /// it was ever implemented that way. Roughly ±20% of launch speed is
    /// enough to break the same-field-tile cohesion `debris_velocity`
    /// describes, while staying small enough that the gradient's own shape
    /// still dominates.
    pub debris_jitter: f32,
    /// Rays `score_cracks` fires from the blast site once, at trigger time,
    /// into the rock the crater will leave standing — the radial fracture
    /// halo real confined blasting produces (`Reports/
    /// explosion-stone-review.md` §2). Fewer than `strike`'s own
    /// `CRACK_RAYS` because a blast's rays run much further (`crack_reach`
    /// below is a multiple of the whole blast radius, not a fixed few
    /// cells), and more rays at that length would mean more overlapping
    /// writes for no extra visible spread.
    pub crack_rays: u32,
    /// How far past the crater wall the crack halo reaches, as a multiple
    /// of `radius`. The rays themselves start beyond `BLAST_SHELL_REACH`
    /// past the crater (see the trigger-time call site's own comment for
    /// why) and run this many blast-radii further from there — a confined
    /// shot's halo is supposed to reach well past the visible hole, which
    /// is the whole point of R1 (`Reports/explosion-stone-review.md` §4).
    pub crack_reach: f32,
    /// How much resistance-weighted cost a confinement ray may spend before
    /// its sector counts as "contained" rather than "open", as a multiple
    /// of the blast's own `radius`. See `probe_confinement`. Set past 1.0
    /// on purpose: a ray that reaches air at *exactly* `radius` cells of
    /// stone (cost 1.0 per cell) has barely more headroom than the crater
    /// itself is about to carve, and the blast would read as "contained"
    /// for cover it is actually about to break straight through. `1.4`
    /// gives a genuine margin past the crater's own reach before a sector
    /// is judged sealed. `f32::INFINITY` reproduces every sector open,
    /// which is what the pre-R2 code always did — the trick
    /// `debris_fraction`'s own tests already use to isolate an older
    /// contract from a newer one.
    pub containment_floor: f32,
    /// Effective clear radius for a *contained* sector, as a fraction of
    /// `radius` — the crush pocket a fully buried charge leaves instead of
    /// the old uniform circle. Small and deliberate, the same reasoning
    /// `vaporize_fraction` uses for the vaporize core: real confined
    /// blasting pulverizes a small volume and spends the rest of its energy
    /// on the crack halo (`crack_rays`/`crack_reach` above), not on
    /// widening a cavity that has nowhere to vent into.
    pub confined_cavity_fraction: f32,
    /// Divides `strength` to give each debris particle its pierce budget —
    /// cells of loose material it may punch through before coming to rest
    /// (`particle::Particle::pierce`).
    ///
    /// Scaled by `strength` rather than fixed, so a bigger charge reaches
    /// further through cover, but deliberately *not* by `radius`, which
    /// already sets how much material is thrown; conflating the two would
    /// make a wide, weak blast punch as far as a narrow, violent one.
    pub pierce_divisor: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            radius: 22.0,
            strength: 180.0,
            duration: 10.0,
            vaporize_fraction: 0.12,
            debris_fraction: 0.4,
            shockwave_multiplier: 1.8,
            fireball_fraction: 0.5,
            flash_temperature: 900.0,
            smoke_fraction: 0.18,
            heat_fraction: 3.0,
            speed_per_strength: 0.05,
            debris_jitter: 0.4,
            pierce_divisor: 12.0,
            crack_rays: 12,
            crack_reach: 1.5,
            containment_floor: 1.4,
            confined_cavity_fraction: 0.35,
        }
    }
}

impl Tuning {
    /// Total number of expansion stages — at least one, however short
    /// `duration` is set.
    fn stages(&self) -> u16 {
        self.duration.round().clamp(1.0, u16::MAX as f32) as u16
    }
}

impl Tuning {
    /// Where the panel persists these, alongside the material `.ron` files.
    pub const ASSET_PATH: &'static str = "assets/explosion.ron";

    /// Load from `ASSET_PATH`, falling back to defaults when the file is
    /// absent or unreadable — absent is the normal case for a fresh
    /// checkout, not an error worth failing startup over.
    pub fn load() -> Self {
        std::fs::read_to_string(Self::ASSET_PATH)
            .ok()
            .and_then(|text| ron::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Serialize back to `ASSET_PATH`.
    ///
    /// A full re-serialization, unlike `tunables::write_field_value`'s
    /// careful span-edit of the material files. That care exists because
    /// material `.ron` files carry hand-written reasoning in comments that a
    /// `ron::ser` round trip would silently destroy; this file is generated,
    /// has no comments to lose, and every field's actual reasoning lives on
    /// `Tuning` itself in the source.
    pub fn save(&self) -> Result<(), String> {
        let pretty = ron::ser::PrettyConfig::new().struct_names(false);
        let text = ron::ser::to_string_pretty(self, pretty).map_err(|e| e.to_string())?;
        std::fs::write(Self::ASSET_PATH, text).map_err(|e| e.to_string())
    }
}

/// Number of fixed directions `probe_confinement` casts, and the number of
/// 22.5-degree pie-slice sectors `sector_of` buckets a cell offset into.
/// Fixed rather than data-driven: `Blast::sector_reach` below is stored as a
/// plain array sized by this constant, and both `clear_annulus` and
/// `rigid::fracture_shell`'s per-cell test have to agree with
/// `probe_confinement`'s own ray order on what "sector 7" means.
pub(crate) const CONFINEMENT_SECTORS: usize = 16;

/// tan(22.5 degrees) = sqrt(2) - 1. See `sector_of`.
const TAN_22_5: f32 = 0.414_213_57;

/// Which of the 16 fixed 22.5-degree pie slices a cell offset `(dx, dy)`
/// from the epicentre falls into — matching the direction order
/// `probe_confinement` fires its rays in (`theta = i * TAU / 16`, sweeping
/// from +x toward +y).
///
/// No `atan2` here: both `clear_annulus` and `fracture_shell`'s annulus loop
/// call this once per cell in their scan box, which at a cavern-scene radius
/// is tens of thousands of calls per blast stage — a transcendental call per
/// cell there is real, avoidable cost for a question that only needs "which
/// 45-degree wedge, then which half of it". Standard octant bucketing (sign
/// of `dx`/`dy`, `|dx|` vs `|dy|`) picks the wedge in one comparison;
/// splitting that wedge into its near and far 22.5-degree halves needs one
/// more comparison against `tan(22.5 degrees)` — but which side of that
/// comparison is the "far" half **alternates with octant parity** (the minor
/// axis grows across the wedge in half the octants and shrinks across it in
/// the other half, depending on which axis is "starting" versus "ending" the
/// 45-degree sweep). Getting that alternation backwards in only some octants
/// produces a blast that is inexplicably lopsided in four of its sixteen
/// directions and nothing else wrong — verified exhaustively against an
/// `atan2` reference over a 400x400-cell offset grid before this was trusted
/// (`Reports/explosion-stone-review.md` §7f), rather than re-derived by eye,
/// because that is exactly the kind of bug eyeballing the trig would not
/// catch.
pub(crate) fn sector_of(dx: i32, dy: i32) -> usize {
    if dx == 0 && dy == 0 {
        return 0; // the epicentre itself; every sector's reach admits dist2 == 0 regardless
    }
    let (ax, ay) = (dx.unsigned_abs(), dy.unsigned_abs());
    let octant = match (dx >= 0, dy >= 0, ax >= ay) {
        (true, true, true) => 0,
        (true, true, false) => 1,
        (false, true, false) => 2,
        (false, true, true) => 3,
        (false, false, true) => 4,
        (false, false, false) => 5,
        (true, false, false) => 6,
        (true, false, true) => 7,
    };
    let (minor, major) = (ax.min(ay) as f32, ax.max(ay) as f32);
    let far_half = if octant % 2 == 0 { minor > major * TAN_22_5 } else { minor < major * TAN_22_5 };
    (octant * 2 + usize::from(far_half)) % CONFINEMENT_SECTORS
}

/// Hard cap on how far one confinement ray may march, independent of
/// resistance — without it a material with `blast_resistance` of exactly
/// `0.0` (a data mistake, not a shipped value; every material ships above
/// zero) would march forever instead of quietly reading as "contained".
/// Generous relative to `containment_floor * radius` at every shipped
/// material's resistance (worst case today is snow at 0.2, so even a
/// radius-60 cavern blast needs only ~420 cells to clear the floor) — a
/// safety rail, not a tuned distance.
const PROBE_MARCH_CAP: i32 = 800;

/// What one confinement ray found: whether it reached a free face (air, a
/// gas cell, or a `Liquid` cell — see `cast_confinement_ray`) within its
/// resistance budget, and how far it got.
struct RayResult {
    /// Accumulated `blast_resistance` cost when the ray stopped, either
    /// because it vented or because it exceeded the floor.
    cost: f32,
    /// Whether the ray reached a free face at all (as opposed to running out
    /// of budget, or hitting the march cap, while still inside solid rock).
    vented: bool,
    /// Whether this ray crossed at least one `Solid`-kind cell — gates
    /// whether the trigger-time crack halo call is worth making at all (an
    /// airburst, or a blast entirely inside loose powder, has no rock to
    /// crack).
    struck_solid: bool,
}

/// March 16 fixed rays out from the epicentre, accumulating each cell's
/// `Material::blast_resistance` as they go, to decide which of the blast's
/// 16 confinement sectors have a nearby free face (R2,
/// `Reports/explosion-stone-review.md` §4) and which are buried.
///
/// Fixed directions, **not** `world.rng` draws: an extra roll here would
/// shift every later draw in the frame's RNG sequence, breaking replay
/// determinism (`CLAUDE.md`, "no `world.rng` draws in the probe"). And
/// sector membership has to be the *same* 16 slices on every stage of one
/// blast — recomputing per stage would let a cell cross from "contained" to
/// "open" mid-blast for no physical reason, which is why this runs once, at
/// trigger, and the result is stored on `Blast` rather than derived fresh
/// each stage.
///
/// Run **before** anything is cleared, for the same reason `score_cracks`'s
/// trigger-time call has to be: by the blast's last stage the crater is
/// empty and `fracture_shell` has already removed the annulus around it, so
/// a probe run then would read a hole where the rock used to be instead of
/// the rock itself.
fn probe_confinement(world: &World, cx: i32, cy: i32, radius: i32, tuning: &Tuning) -> ([u8; CONFINEMENT_SECTORS], bool) {
    let mut reach = [0u8; CONFINEMENT_SECTORS];
    let mut any_solid = false;
    let floor = tuning.containment_floor * radius as f32;
    for (i, slot) in reach.iter_mut().enumerate() {
        let theta = i as f32 * std::f32::consts::TAU / CONFINEMENT_SECTORS as f32;
        let (dx, dy) = (theta.cos(), theta.sin());
        let result = cast_confinement_ray(world, cx, cy, dx, dy, floor);
        any_solid |= result.struck_solid;
        *slot = if result.vented && result.cost <= floor {
            radius.clamp(0, u8::MAX as i32) as u8
        } else {
            ((radius as f32) * tuning.confined_cavity_fraction).clamp(0.0, u8::MAX as f32) as u8
        };
    }
    (reach, any_solid)
}

/// One ray of `probe_confinement` — split out so the loop above stays a
/// simple per-direction dispatch rather than a nested loop with two exit
/// conditions tangled into one body.
fn cast_confinement_ray(world: &World, cx: i32, cy: i32, dx: f32, dy: f32, floor: f32) -> RayResult {
    let mut cost = 0.0f32;
    let mut struck_solid = false;
    for r in 1..=PROBE_MARCH_CAP {
        let (x, y) = (cx + (dx * r as f32).round() as i32, cy + (dy * r as f32).round() as i32);
        if !world.in_bounds(x, y) {
            // The world edge counts as a free face -- there is nothing left
            // out there to push against, the same reading `debris_velocity`
            // already gives an unblocked field sample.
            return RayResult { cost, vented: true, struck_solid };
        }
        let cell = world.get(x, y);
        // A raw material test, not `cell.is_empty()` -- this question is
        // "is there material here to push through", the same one
        // `clear_annulus` asks and for the same reason: `is_empty()` is
        // managed-aware and would read a promoted body's reserved container
        // cell as occupied even though it holds `material::EMPTY`.
        let kind = world.materials.kind(cell.material);
        if cell.material == material::EMPTY || kind == material::MaterialKind::Gas || kind == material::MaterialKind::Liquid {
            // Vented: air and gas offer no resistance, and a `Liquid` is
            // treated the same way rather than read through its own
            // `blast_resistance` (which is never read here as a result --
            // water and oil need no entry in their `.ron` files). A liquid
            // cannot brace a blast on the timescale a detonation happens
            // on: it displaces instead of confining, which is exactly what
            // round 3 of `Reports/explosion-mechanics-diagnosis.md` found
            // and fixed ("water, which previously threw *nothing* at any
            // depth, now opens real cavities") -- a charge under water is a
            // free face to this probe, full stop, the same as a charge
            // under open sky. Confining a blast is a *solid's* job.
            return RayResult { cost, vented: true, struck_solid };
        }
        if kind == material::MaterialKind::Solid {
            struck_solid = true;
        }
        cost += world.materials.get(cell.material).blast_resistance;
        if cost > floor {
            // Already past the containment floor -- this sector cannot be
            // open regardless of what lies further out, so stop spending
            // march budget on it.
            return RayResult { cost, vented: false, struck_solid };
        }
    }
    // Ran the whole march cap while still under the floor and never hit a
    // free face -- effectively "contained", the same as exceeding the floor
    // outright.
    RayResult { cost, vented: false, struck_solid }
}

/// R2's confinement result, bundled for `rigid::fracture_shell` — the reach
/// array and the radius it is measured against always travel together (a
/// sector is "open" exactly when its reach equals the radius), and passing
/// them as one `Copy` struct rather than two parameters is also what keeps
/// `fracture_shell` under clippy's `too_many_arguments`.
#[derive(Clone, Copy, Debug)]
pub struct Confinement {
    pub sector_reach: [u8; CONFINEMENT_SECTORS],
    pub radius: i32,
}

/// What one blast actually did — R5's "did it fire at all" line
/// (`CLAUDE.md`: a discrete event needs a counter printed next to the
/// image, not just a picture). Filled in at trigger time and updated as the
/// blast's stages run; left on `Blasts` until the next blast overwrites it,
/// rather than printed from inside this module, so a test that fires a
/// blast is not forced to see it on stdout too — a caller that wants the
/// line (the app's HUD, `filmstrip`'s `boom:` hook) reads
/// `Blasts::last_blast_report` once the blast has finished.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BlastReport {
    /// How many of the 16 confinement sectors read as open (a free face
    /// within `containment_floor * radius` resistance-weighted cells).
    pub open_sectors: u32,
    /// The remaining sectors: buried, and cleared only to
    /// `confined_cavity_fraction * radius`.
    pub contained_sectors: u32,
    /// Cells actually converted to debris/vacuum across every stage.
    pub cells_cleared: u32,
    /// Cells that would have cleared this stage under the old (uniform)
    /// geometry but were left standing because their sector is contained —
    /// the quantity that used to be zero for every blast, always.
    pub cells_held_by_containment: u32,
    /// Cells `score_cracks` scored with a fresh fissure — the radial halo,
    /// R1's whole point.
    pub cells_fissured: u32,
}

impl std::fmt::Display for BlastReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cleared {} cells, {} held by containment, {} fissured, sectors open {}/{} contained {}/{}",
            self.cells_cleared,
            self.cells_held_by_containment,
            self.cells_fissured,
            self.open_sectors,
            CONFINEMENT_SECTORS,
            self.contained_sectors,
            CONFINEMENT_SECTORS,
        )
    }
}

/// One explosion in progress: a cavity front expanding outward from
/// `(cx, cy)`, one stage per frame.
#[derive(Clone, Copy, Debug)]
pub struct Blast {
    cx: i32,
    cy: i32,
    radius: i32,
    strength: f32,
    /// Stages already run. The blast is finished once this reaches
    /// `Tuning::stages`.
    stage: u16,
    /// Effective clear radius per confinement sector, in cells — `radius`
    /// itself for an open sector, `confined_cavity_fraction * radius` for a
    /// contained one. Computed once by `probe_confinement` at trigger time
    /// and read by every later stage's `clear_annulus`/`fracture_shell`
    /// call, so a cell's sector membership and reach never change mid-blast.
    sector_reach: [u8; CONFINEMENT_SECTORS],
    /// Running totals for `Blasts::last_blast_report`. `open_sectors`,
    /// `contained_sectors` and `cells_fissured` are set once at
    /// construction; `cells_cleared`/`cells_held_by_containment` accumulate
    /// as `clear_annulus` runs on each stage.
    report: BlastReport,
}

/// Every blast currently expanding, plus the tuning they all read.
///
/// Lives alongside `World` and `ParticleSystem` rather than inside either,
/// for the same reason `ParticleSystem` itself does: a blast is not part of
/// the CA grid's own state, and keeping it separate is what makes "does the
/// CA grid need to know explosions exist" a question with an easy answer of
/// no.
#[derive(Clone, Debug, Default)]
pub struct Blasts {
    active: Vec<Blast>,
    pub tuning: Tuning,
    /// See `BlastReport`'s own doc. `Default::default()` (all zero) before
    /// any blast has ever fired, which is indistinguishable from "the last
    /// blast fissured nothing" — harmless, since nothing reads this before
    /// a first blast exists to ask about.
    last_report: BlastReport,
}

impl Blasts {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start from tuning loaded off disk rather than the compiled defaults.
    pub fn with_tuning(tuning: Tuning) -> Self {
        Self { active: Vec::new(), tuning, last_report: BlastReport::default() }
    }

    /// Begin a blast at `(cx, cy)` using the current tuning's own radius and
    /// strength, and run its first stage immediately — so pressing the key
    /// produces a visible flash on the same frame rather than one later.
    pub fn trigger(&mut self, world: &mut World, particles: &mut ParticleSystem, cx: i32, cy: i32) {
        let (radius, strength) = (self.tuning.radius.max(1.0) as i32, self.tuning.strength);
        self.trigger_with(world, particles, cx, cy, radius, strength);
    }

    /// `trigger`, with an explicit radius and strength — for callers that
    /// have their own (tests, and any future gameplay source that is not the
    /// player's own key).
    pub fn trigger_with(&mut self, world: &mut World, particles: &mut ParticleSystem, cx: i32, cy: i32, radius: i32, strength: f32) {
        // The pressure impulse and field heat spike are written once, at
        // detonation, not per stage — the field carries the shock from here
        // and nothing else in this module propagates it. `debris_velocity`
        // reads the impulse back on every stage, which is why it is written
        // before the first stage runs rather than after.
        world.add_pressure_impulse(cx, cy, radius, strength);
        world.add_heat(cx, cy, radius, strength / self.tuning.heat_fraction);
        // A blast is a disturbance: it licenses failures near it. See
        // `World::chain_reach`.
        world.record_disturbance(cx, cy);
        let mut blast = Blast::new(world, cx, cy, radius, strength, &self.tuning);
        let still_going = blast.advance(world, particles, &self.tuning);
        self.last_report = blast.report;
        if still_going {
            self.active.push(blast);
        }
    }

    /// Advance every blast in progress by one stage, dropping the finished
    /// ones. Its own frame phase, called from `App::update`.
    pub fn step(&mut self, world: &mut World, particles: &mut ParticleSystem) {
        let tuning = self.tuning;
        // Split the borrow so the closure can read the blast it just
        // advanced back into `self.last_report` while `retain_mut` still
        // holds `self.active` mutably -- these are disjoint fields, so the
        // borrow checker allows both at once as long as neither expression
        // goes through `self` as a whole.
        let last_report = &mut self.last_report;
        self.active.retain_mut(|blast| {
            let still_going = blast.advance(world, particles, &tuning);
            *last_report = blast.report;
            still_going
        });
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    pub fn len(&self) -> usize {
        self.active.len()
    }

    /// What the most recently triggered blast did, updated as its stages
    /// run and left in place once it finishes. See `BlastReport`.
    pub fn last_blast_report(&self) -> BlastReport {
        self.last_report
    }
}

impl Blast {
    /// Build a blast at trigger time: probe confinement (R2) and score the
    /// crack halo (R1) if the probe crossed real rock, both before a single
    /// cell has been cleared. Shared by `Blasts::trigger_with` (the staged,
    /// player-facing path) and the synchronous `trigger_tuned` below, the
    /// same "two drivers, one set of rules" shape `Blast::advance` already
    /// uses for `update::step`/`parallel::step` -- a second copy of this
    /// logic in the synchronous path would drift from this one the first
    /// time either changed.
    fn new(world: &mut World, cx: i32, cy: i32, radius: i32, strength: f32, tuning: &Tuning) -> Self {
        let (sector_reach, struck_solid) = probe_confinement(world, cx, cy, radius, tuning);
        let open_sectors = sector_reach.iter().filter(|&&r| r as i32 >= radius).count() as u32;
        let contained_sectors = CONFINEMENT_SECTORS as u32 - open_sectors;

        // R1: the radial fracture halo, scored now rather than on any later
        // stage. `score_cracks`'s rays `break` on the first non-`Solid`
        // cell (`rigid::is_body_material`), and by the blast's last stage
        // the crater is empty *and* `fracture_shell` has already removed
        // the annulus out to `radius + BLAST_SHELL_REACH` -- a ray starting
        // at `from = radius` would die on its very first cell there, or
        // inside re-landed debris that eats it the same way. Calling here,
        // before anything is cleared, means the rays start in what will
        // remain standing rock once the crater finishes expanding, so
        // `from` has to clear the band the shell fracture is about to take:
        // `radius + BLAST_SHELL_REACH + 1` (`Reports/
        // explosion-stone-review.md` §5's adversarial-verification finding
        // -- the one place the first draft of this spec was wrong).
        //
        // Gated on having actually crossed rock: an airburst, or a blast
        // entirely inside loose powder, has no rock to crack and should not
        // pay for the ray scan.
        let cells_fissured = if struck_solid {
            let from = radius + BLAST_SHELL_REACH + 1;
            let length = (radius as f32 * tuning.crack_reach) as i32 + from;
            super::rigid::score_cracks(world, cx, cy, from, length, tuning.crack_rays)
        } else {
            0
        };

        Self {
            cx,
            cy,
            radius,
            strength,
            stage: 0,
            sector_reach,
            report: BlastReport { open_sectors, contained_sectors, cells_cleared: 0, cells_held_by_containment: 0, cells_fissured },
        }
    }

    /// Run one stage. Returns whether the blast is still going.
    fn advance(&mut self, world: &mut World, particles: &mut ParticleSystem, tuning: &Tuning) -> bool {
        let stages = tuning.stages();
        let radius = self.radius as f32;
        // The cavity front, before and after this stage. Squared radii
        // throughout, so the per-cell test stays a comparison of integers
        // promoted to f32 rather than a square root per cell.
        let front_prev = radius * self.stage as f32 / stages as f32;
        let front_now = radius * (self.stage + 1) as f32 / stages as f32;
        // The inner bound is *exclusive* everywhere except the very first
        // stage, where it has to admit the epicentre itself. `front_prev` is
        // exactly 0.0 at stage 0, and `clear_annulus` skips `dist2 <= prev2`
        // to avoid re-clearing the previous stage's ring — which silently
        // spared the one cell at `dist2 == 0`, leaving the material directly
        // under the charge untouched. Caught by
        // `an_explosion_clears_material_within_its_radius`, which has
        // asserted exactly this since M15 and is the reason it exists.
        let prev2 = if self.stage == 0 { -1.0 } else { front_prev * front_prev };
        let now2 = front_now * front_now;
        let vaporize2 = (radius * tuning.vaporize_fraction).powi(2);
        let last = self.stage + 1 >= stages;

        self.clear_annulus(world, particles, tuning, prev2, now2, vaporize2);
        // The hot shell rides just ahead of the front, so the glow expands
        // with the cavity instead of appearing all at once at the final
        // radius. On the last stage it is written at the full fireball
        // radius, which is what actually leaves the surroundings scorched.
        let scorch_to = if last {
            radius * (1.0 + tuning.fireball_fraction)
        } else {
            front_now + SCORCH_SHELL_THICKNESS
        };
        self.scorch(world, tuning, now2, scorch_to * scorch_to);

        if last {
            self.shockwave(world, particles, tuning);
            self.backfill_smoke(world, tuning);
        }

        self.stage += 1;
        !last
    }

    /// Clear the annulus between two fronts, converting material to debris.
    fn clear_annulus(&mut self, world: &mut World, particles: &mut ParticleSystem, tuning: &Tuning, prev2: f32, now2: f32, vaporize2: f32) {
        let reach = front_reach(now2);
        let mut struck_rock = false;
        for y in (self.cy - reach)..=(self.cy + reach) {
            for x in (self.cx - reach)..=(self.cx + reach) {
                let (dx, dy) = (x - self.cx, y - self.cy);
                let dist2 = (dx * dx + dy * dy) as f32;
                if dist2 <= prev2 || dist2 > now2 {
                    continue; // already cleared by an earlier stage, or not reached yet
                }
                let cell = world.get(x, y);
                // A raw material test, not `cell.is_empty()`. This function's
                // own question is "is there material here to destroy", not
                // "is this position available to use" -- and `is_empty()`
                // answers the second, treating a promoted liquid body's
                // reserved container cells as occupied even though they hold
                // `material::EMPTY`. Through `is_empty()` an explosion
                // overlapping a lake's outline spawned debris particles whose
                // material was `EMPTY`: invisible flying nothing, which then
                // lands and writes itself into the world as a cell.
                // `render.rs` and `World::ignite_circle` already made exactly
                // this switch, each for its own version of the same reason.
                // Found by review.
                //
                // Bedrock is the world's own boundary material and never
                // destructible by anything, the same way it is never a target
                // for painting (`World::paint_circle`) or ignition.
                if cell.material == material::EMPTY || cell.material == material::BEDROCK {
                    continue;
                }
                // Smoke this blast laid down on an earlier stage must not be
                // re-thrown as debris by a later one -- it would spawn
                // particles made of gas and empty the crater the residue was
                // just added to.
                if cell.material == material::SMOKE {
                    continue;
                }
                // R2: a contained sector never clears past its own effective
                // radius, however far this stage's front has expanded --
                // this is what turns a fully buried charge into a crush
                // pocket instead of the old uniform circle. Open sectors are
                // unaffected: `sector_reach` there is exactly `radius`,
                // which `now2` (capped at `radius^2` on the last stage)
                // never exceeds, so this test never fires for them. Checked
                // after the material/SMOKE skips above so the counter below
                // reports cells that would genuinely have cleared, not
                // candidates that were already air.
                let sector_limit = self.sector_reach[sector_of(dx, dy)] as f32;
                if dist2 > sector_limit * sector_limit {
                    self.report.cells_held_by_containment += 1;
                    continue;
                }
                self.report.cells_cleared += 1;

                if dist2 > vaporize2 && world.rng.chance(tuning.debris_fraction) {
                    let (vx, vy) = debris_velocity(world, x, y, self.cx, self.cy, self.strength, tuning);
                    let pierce = pierce_budget(self.strength, tuning);
                    particles.spawn_piercing((x as f32, y as f32), (vx, vy), cell.material, cell.shade, pierce);
                }
                let was_structural = matches!(world.materials.kind(cell.material), material::MaterialKind::Solid | material::MaterialKind::Plant);
                world.set(x, y, Cell::EMPTY);
                // M17: an explosion is exactly the kind of disturbance
                // structural checks exist for -- clearing a `Solid`/`Plant`
                // cell (the latter added by architecture item 9) may have
                // just dropped whatever it was propping up.
                if was_structural {
                    world.schedule_structural_check_around(x, y);
                    // A blast is the loudest possible version of "this rock
                    // has been broken": the crater wall stops being part of
                    // the mass behind it, so what the explosion undercuts
                    // can actually come down as pieces afterwards rather
                    // than hanging over a perfectly clean hole. Same
                    // transition the eraser goes through -- see
                    // `structural::detach_exposed_neighbours`.
                    super::structural::detach_exposed_neighbours(world, x, y);
                    struck_rock = true;
                }
            }
        }

        // Crack the crater wall into pieces. Everything above turns the
        // blast's own volume into single-cell debris particles, which is the
        // right treatment for sand and the wrong one for stone -- against
        // rock it produced a clean hole and a spray of grit, never a piece.
        // The rim has just been loosened by `detach_exposed_neighbours`, so
        // it is no longer braced by the mass behind it and can come away as
        // chunks thrown outward from the charge.
        //
        // Guarded on having actually hit something structural: an airburst,
        // or a blast inside a sand pile, should not pay for a shell scan it
        // has no rock to find.
        if struck_rock {
            // R2: a contained sector's rim must stay attached. Without
            // gating this too, "contained" only ever meant `clear_annulus`
            // -- the shell scan below would still find that rim, strip its
            // attachment bonus and fracture it, quietly reproducing the
            // self-refilling bruise this gating exists to remove even
            // though the sector never lost a cell (`Reports/
            // explosion-stone-review.md` §5, "sector gating must gate
            // `fracture_shell` too, not just `clear_annulus`").
            super::rigid::fracture_shell(
                world,
                (self.cx, self.cy),
                reach,
                reach + BLAST_SHELL_REACH,
                self.strength * BLAST_SHELL_FORCE,
                1,
                Confinement { sector_reach: self.sector_reach, radius: self.radius },
            );
        }
    }

    /// Write CA cell temperature into the shell of intact material between
    /// two radii — the visible fireball, and the blast's ignition source.
    ///
    /// This replaces a `World::ignite_circle` call, and the replacement is
    /// the point. `ignite_circle` is M14's *debug* force-ignite tool: it
    /// sets any material burning regardless of `flammability`, so a stone
    /// wall next to a blast caught fire exactly as readily as an oil pool.
    /// Worse, it wrote one fixed burn duration to every cell on the same
    /// frame, so the ring lit instantly at full strength, held perfectly
    /// constant for its whole span, and switched off all at once — measured
    /// at exactly 520 burning cells from frame 1 to frame 180, then 0.
    ///
    /// Temperature has none of those problems and needs no special cases:
    /// `render.rs` already draws a continuous heat glow from it,
    /// `fire::diffuse_heat` already spreads and decays it (so the fireball
    /// fades raggedly rather than switching off), and `fire::try_ignite`
    /// already ignites on it *only* where `ignition_temperature` is finite
    /// and reached — which is precisely the flammability check the old path
    /// skipped.
    fn scorch(&self, world: &mut World, tuning: &Tuning, inner2: f32, outer2: f32) {
        let (inner, outer) = (inner2.max(0.0).sqrt(), outer2.max(0.0).sqrt());
        let reach = front_reach(outer2);
        for y in (self.cy - reach)..=(self.cy + reach) {
            for x in (self.cx - reach)..=(self.cx + reach) {
                let (dx, dy) = (x - self.cx, y - self.cy);
                let dist2 = (dx * dx + dy * dy) as f32;
                if dist2 < inner2 || dist2 > outer2 {
                    continue;
                }
                let mut cell = world.get(x, y);
                if cell.material == material::EMPTY || cell.material == material::BEDROCK {
                    continue; // nothing there to heat
                }
                // Falls off across the shell's own *width in cells*, not in
                // squared distance — squared space is steep enough near the
                // inner edge that the hot end of the ramp landed almost
                // entirely on cells the blast had already cleared, wasting
                // it. Linear distance puts the peak on the first ring of
                // intact material, which is the ring anyone actually sees.
                let t = if outer > inner { ((dist2.sqrt() - inner) / (outer - inner)).clamp(0.0, 1.0) } else { 0.0 };
                let peak = tuning.flash_temperature * (1.0 - t * SCORCH_FALLOFF);
                // Position-keyed, so the ring's own edge is ragged rather
                // than a clean circle -- the same stable-per-position
                // primitive the fire flicker and `roll_reach_at` already use.
                let ragged = peak * (1.0 - rng::jitter(x, y) * SCORCH_RAGGEDNESS);
                let target = ragged.clamp(0.0, i16::MAX as f32) as i16;

                // Heat alone cannot light anything in this engine as shipped:
                // `fire::try_ignite`'s temperature path fires only where
                // `ignition_temperature` is finite, and **no shipped material
                // sets one** — oil's own file says so explicitly ("left at
                // its default of 'never'"). Oil and wood catch by *neighbour
                // contact*, rolled against `flammability`. So a blast that
                // only wrote temperature would glow beautifully and never
                // start a fire, which is the opposite failure to the one
                // being fixed.
                //
                // Rolling `flammability` here is the honest way to seed that
                // first burning cell: it is the same property `try_ignite`
                // itself rolls, so stone (0.0) can never light and oil (0.5)
                // readily does — which was the actual bug, since the old
                // `World::ignite_circle` path ignored the property entirely
                // and set stone burning. Fire then spreads from these seeds
                // through `fire.rs` normally, rather than this module
                // painting a finished fireball.
                let m = world.materials.get(cell.material);
                let (flammability, burn_duration, burn_temperature) = (m.flammability, m.burn_duration, m.burn_temperature);
                // Fades across the shell the same way the heat does, so the
                // fireball is densest against the crater and thins outward
                // rather than lighting the whole ring uniformly.
                let ignite_odds = flammability * (1.0 - t);
                if !cell.is_burning() && flammability > 0.0 && burn_duration > 0 && world.rng.chance(ignite_odds) {
                    // Jittered duration, so the ring does not switch off in
                    // lockstep. The old force-ignite path gave every cell the
                    // same timer on the same frame, measured at exactly 520
                    // burning cells held constant from frame 1 to frame 180
                    // and then 0 — a step function, which is what made the
                    // fireball read as a frozen decal.
                    let spread = 1.0 + (rng::jitter(x + JITTER_AXIS_OFFSET, y + JITTER_AXIS_OFFSET) - 0.5) * 2.0 * BURN_DURATION_JITTER;
                    let duration = ((burn_duration as f32 * spread).round() as u16).max(1);
                    cell.ignite(duration);
                    if burn_temperature.is_finite() {
                        cell.set_temperature(burn_temperature.round() as i16);
                    }
                    world.set(x, y, cell);
                    continue;
                }

                // Never *cool* a cell that is already hotter -- two
                // overlapping blasts, or a blast over an existing fire.
                if cell.temperature() < target {
                    cell.set_temperature(target);
                    world.set(x, y, cell);
                }
            }
        }
    }

    /// Loose material (`Powder`/`Liquid`) just outside the crater has a
    /// fading chance to be picked up and thrown too, not just left to fall
    /// into the hole the blast dug. Without this, an explosion in the middle
    /// of a big sand pile reads as "a hole appears, the surroundings quietly
    /// avalanche into it" — ordinary settling under gravity is the only
    /// thing that ever moves a loose CA cell that wasn't itself inside the
    /// blast radius, since the pressure impulse only ever pushes free
    /// particles, never settled grid material.
    ///
    /// Restricted to loose material specifically (not `Solid`/`Plant`, which
    /// shouldn't be uprooted by a shockwave that didn't even clear them) — a
    /// blast can fling sand it never touched, but it does not casually rip a
    /// wall out by the same mechanism that would need to actually break it
    /// structurally first.
    fn shockwave(&self, world: &mut World, particles: &mut ParticleSystem, tuning: &Tuning) {
        let radius = self.radius as f32;
        let r2 = radius * radius;
        let outer2 = r2 * tuning.shockwave_multiplier * tuning.shockwave_multiplier;
        let reach = front_reach(outer2);
        for y in (self.cy - reach)..=(self.cy + reach) {
            for x in (self.cx - reach)..=(self.cx + reach) {
                let (dx, dy) = (x - self.cx, y - self.cy);
                let dist2 = (dx * dx + dy * dy) as f32;
                if dist2 <= r2 || dist2 > outer2 {
                    continue; // inside the crater, or beyond the shockwave's reach
                }
                let cell = world.get(x, y);
                if !matches!(world.materials.kind(cell.material), material::MaterialKind::Powder | material::MaterialKind::Liquid) {
                    continue;
                }
                let chance = shockwave_pickup_chance(self.radius, dist2.sqrt(), tuning);
                if world.rng.chance(chance * tuning.debris_fraction) {
                    let (vx, vy) = debris_velocity(world, x, y, self.cx, self.cy, self.strength, tuning);
                    let pierce = pierce_budget(self.strength, tuning);
                    particles.spawn_piercing((x as f32, y as f32), (vx, vy), cell.material, cell.shade, pierce);
                    world.set(x, y, Cell::EMPTY);
                }
            }
        }
    }

    /// Leave smoke behind in the crater — see `Tuning::smoke_fraction`.
    ///
    /// Written into cells that are *materially* empty only, so this can
    /// never overwrite debris that has already landed back inside the
    /// crater, nor a promoted body's reserved container cell (checked with a
    /// raw material test for the first reason and `is_empty` for the
    /// second — both questions are being asked here, unusually).
    fn backfill_smoke(&self, world: &mut World, tuning: &Tuning) {
        if tuning.smoke_fraction <= 0.0 {
            return;
        }
        let r2 = (self.radius * self.radius) as f32;
        for y in (self.cy - self.radius)..=(self.cy + self.radius) {
            for x in (self.cx - self.radius)..=(self.cx + self.radius) {
                let (dx, dy) = (x - self.cx, y - self.cy);
                if ((dx * dx + dy * dy) as f32) > r2 {
                    continue;
                }
                let cell = world.get(x, y);
                if cell.material != material::EMPTY || !cell.is_empty() {
                    continue;
                }
                if world.rng.chance(tuning.smoke_fraction) {
                    world.set(x, y, Cell::new(material::SMOKE, (rng::jitter(x, y) * 255.0) as u8));
                }
            }
        }
    }
}

/// How much the scorch shell cools across its own width, as a fraction of
/// the peak — so the ring's inner edge reads hotter than its outer one.
const SCORCH_FALLOFF: f32 = 0.75;
/// How much position-keyed variation the scorch ring gets, as a fraction of
/// its local peak. The old force-ignite path drew a geometrically perfect
/// annulus of uniform colour, which read as a stamped decal; this is what
/// breaks the circle up.
const SCORCH_RAGGEDNESS: f32 = 0.45;
/// How far ahead of the expanding cavity front the hot shell sits, in cells.
const SCORCH_SHELL_THICKNESS: f32 = 3.0;
/// Fractional spread applied to each ignited cell's burn duration, so a
/// fireball's cells burn out at staggered times instead of all at once. See
/// the call site for the measurement that motivated it.
/// How far past the blast front to look for rock the charge has loosened.
/// Small: the shell that can come away is the crater wall itself, not the
/// countryside around it.
const BLAST_SHELL_REACH: i32 = 3;

/// Fraction of a blast's strength that gets spent throwing crater-wall
/// chunks, as opposed to the debris particles and pressure it already wrote.
const BLAST_SHELL_FORCE: f32 = 0.06;

const BURN_DURATION_JITTER: f32 = 0.5;

/// Integer loop bound for a squared radius — the smallest box that can
/// contain it. `ceil`, not `round`: a bound that rounds *down* silently
/// clips the outermost ring of cells off whatever is being scanned.
fn front_reach(radius2: f32) -> i32 {
    radius2.max(0.0).sqrt().ceil() as i32
}

/// Trigger an explosion and run it to completion immediately, in one call.
///
/// The synchronous counterpart to [`Blasts`], and deliberately built on the
/// exact same [`Blast::advance`] — the same "two drivers over one set of
/// rules" shape `update::step` and `parallel::step` already use for the CA
/// sweep, for the same reason: a second implementation would drift.
///
/// `App` uses [`Blasts`] so the blast is staged across frames where a player
/// can see it. Tests use this, so a single call leaves the world in the
/// blast's final state with nothing to step.
pub fn trigger(world: &mut World, particles: &mut ParticleSystem, cx: i32, cy: i32, radius: i32, strength: f32) {
    trigger_tuned(world, particles, cx, cy, radius, strength, &Tuning::default())
}

/// `trigger`, with explicit tuning.
pub fn trigger_tuned(world: &mut World, particles: &mut ParticleSystem, cx: i32, cy: i32, radius: i32, strength: f32, tuning: &Tuning) {
    world.add_pressure_impulse(cx, cy, radius, strength);
    world.add_heat(cx, cy, radius, strength / tuning.heat_fraction);
    // `Blast::new` runs the R2 probe and the R1 crack halo, exactly as
    // `Blasts::trigger_with` does — see its own doc for why both
    // constructors have to do this rather than just one.
    let mut blast = Blast::new(world, cx, cy, radius, strength, tuning);
    while blast.advance(world, particles, tuning) {}
}

/// Cells of loose material a blast's debris may punch through before it has
/// to come to rest (`particle::Particle::pierce`).
///
/// The divisor is set from the measurement this mechanic exists to fix
/// rather than from anything physical: on a flat sand bed at `strength =
/// 180`, cells thrown clear of the blast zone fell to exactly zero once
/// cover exceeded roughly 15 cells. A budget of `180 / 12 = 15` puts the
/// reach back at about that threshold before `particle::
/// PIERCE_SPEED_RETENTION`'s own decay is accounted for, which is the point
/// at which the mechanic starts to matter rather than a value tuned for a
/// target number.
fn pierce_budget(strength: f32, tuning: &Tuning) -> u8 {
    if tuning.pierce_divisor <= 0.0 {
        return 0;
    }
    (strength / tuning.pierce_divisor).clamp(0.0, u8::MAX as f32) as u8
}

/// Chance a loose cell at distance `dist` from the epicentre is picked up by
/// the shockwave, fading linearly from 1.0 at `radius` to 0.0 at
/// `radius * shockwave_multiplier`. Deliberately built from the *continuous*
/// radius, not a rounded one: an earlier version divided by a rounded
/// `shockwave_radius - radius`, so whenever the multiplier rounded the outer
/// radius down, cells between the true and rounded edge still passed the
/// caller's zone check but produced a negative chance instead of fading to
/// exactly zero — `Rng::chance` silently treats negative as "never," so the
/// annulus quietly narrowed below what the constant says it should be.
fn shockwave_pickup_chance(radius: i32, dist: f32, tuning: &Tuning) -> f32 {
    let span = radius as f32 * (tuning.shockwave_multiplier - 1.0);
    if span <= 0.0 {
        return 0.0;
    }
    // Clamped defensively: right at the outer edge, `dist` and the
    // continuous radius are equal in exact math but not always in float
    // math, so the unclamped formula can land a hair below zero there.
    // `Rng::chance` already treats negative as "never," but `chance` is a
    // probability and should read as one.
    (1.0 - (dist - radius as f32) / span).clamp(0.0, 1.0)
}

/// Debris velocity from the local pressure gradient — not a naive radial
/// burst — so a blast throws material away from the centre and around
/// corners rather than in a perfect circle regardless of what is in the way.
///
/// The gradient is read from the field as it stands after
/// `add_pressure_impulse`, before the field has taken many `field::step`s —
/// so what actually gives this its shape is checking `field_is_blocked` at
/// each neighbour and skipping a blocked one, rather than reading its
/// (still-ambient) pressure as if it were open ground. A neighbour on the
/// far side of a wall is excluded from the gradient the same way the field's
/// own `step_velocity` excludes it, just computed directly here instead of
/// waiting a frame for the field to do it.
fn debris_velocity(world: &World, x: i32, y: i32, cx: i32, cy: i32, strength: f32, tuning: &Tuning) -> (f32, f32) {
    let sample = |dx: i32, dy: i32| -> Option<f32> {
        let (nx, ny) = (x + dx, y + dy);
        if world.field_is_blocked(nx, ny) {
            None
        } else {
            Some(world.field_at(nx, ny).pressure)
        }
    };

    let left = sample(-FIELD_SCALE, 0);
    let right = sample(FIELD_SCALE, 0);
    let up = sample(0, -FIELD_SCALE);
    let down = sample(0, FIELD_SCALE);

    // Missing (wall-blocked) sides simply do not contribute — treating a
    // blocked neighbour as "equal to here" would flatten the gradient right
    // where a wall should be steering it instead.
    let gx = match (left, right) {
        (Some(l), Some(r)) => l - r,
        (Some(l), None) => l, // only the open side pushes, away from it
        (None, Some(r)) => -r,
        (None, None) => 0.0,
    };
    let gy = match (up, down) {
        (Some(u), Some(d)) => u - d,
        (Some(u), None) => u,
        (None, Some(d)) => -d,
        (None, None) => 0.0,
    };

    let mag = (gx * gx + gy * gy).sqrt();
    let speed = strength * tuning.speed_per_strength;

    let (vx, vy) = if mag > 0.01 {
        (gx / mag * speed, gy / mag * speed)
    } else {
        // No usable gradient (dead centre, or walled in on every side) —
        // fall back to a purely radial push away from the epicentre so a
        // symmetric position still gets thrown *somewhere* rather than
        // sitting motionless in an otherwise fully cleared blast radius.
        let (dx, dy) = ((x - cx) as f32, (y - cy) as f32);
        let d = (dx * dx + dy * dy).sqrt().max(1.0);
        (dx / d * speed, dy / d * speed)
    };

    // Position-keyed (not frame-keyed) so a given cell's jitter is stable.
    // See `Tuning::debris_jitter` for why this is scaled by `speed`, not raw
    // `strength`.
    let jx = (rng::jitter(x, y) - 0.5) * tuning.debris_jitter * speed;
    let jy = (rng::jitter(x + JITTER_AXIS_OFFSET, y) - 0.5) * tuning.debris_jitter * speed;
    (vx + jx, vy + jy)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::cell::AMBIENT_TEMPERATURE;

    /// The tests exercise the shipped defaults unless they say otherwise.
    fn tuning() -> Tuning {
        Tuning::default()
    }

    use crate::sim::chunk::Rect;
    use crate::sim::material;

    fn test_world() -> World {
        let mut w = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            w.set(x, 127, Cell::new(material::STONE, 0));
        }
        w
    }

    #[test]
    fn an_explosion_clears_material_within_its_radius() {
        let mut w = test_world();
        for y in 30..50 {
            for x in 30..50 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 40, 40, 8, 150.0);

        // Not `is_empty()`: `Tuning::smoke_fraction` backfills part of the
        // crater with `SMOKE`, so the epicentre is legitimately allowed to
        // hold a gas cell afterwards. What must be true is that the *stone*
        // is gone, which is what this test has always actually been about.
        assert_ne!(w.get(40, 40).material, material::STONE, "the epicentre was not cleared");
    }

    /// Smoke is the blast's residue, and nothing else in the simulation has
    /// ever produced a `SMOKE` cell — not fire, not explosions — despite the
    /// material existing and rising correctly since M4. See
    /// `Tuning::smoke_fraction`.
    #[test]
    fn an_explosion_leaves_smoke_behind_in_its_crater() {
        let mut w = test_world();
        for y in 20..60 {
            for x in 20..60 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        let radius = 12;
        trigger(&mut w, &mut particles, 40, 40, radius, 180.0);

        let smoke = ((40 - radius)..=(40 + radius))
            .flat_map(|y| ((40 - radius)..=(40 + radius)).map(move |x| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == material::SMOKE)
            .count();
        assert!(smoke > 0, "the blast left no smoke at all");
    }

    /// A blast with smoke disabled must leave a genuinely empty crater —
    /// the guard that `backfill_smoke` is gated on its own tuning rather
    /// than unconditional, so anyone who turns it off gets the old
    /// behaviour exactly.
    #[test]
    fn smoke_can_be_turned_off_entirely() {
        let mut w = test_world();
        for y in 20..60 {
            for x in 20..60 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        let off = Tuning { smoke_fraction: 0.0, ..Tuning::default() };
        trigger_tuned(&mut w, &mut particles, 40, 40, 12, 180.0, &off);

        let smoke = (28..=52)
            .flat_map(|y| (28..=52).map(move |x| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == material::SMOKE)
            .count();
        assert_eq!(smoke, 0, "smoke_fraction = 0 still produced {smoke} smoke cells");
    }

    #[test]
    fn an_explosion_leaves_bedrock_untouched() {
        let mut w = test_world();
        w.set(40, 40, Cell::new(material::BEDROCK, 0));
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 40, 40, 8, 150.0);
        assert_eq!(w.get(40, 40).material, material::BEDROCK, "bedrock was destroyed");
    }

    #[test]
    fn an_explosion_raises_pressure_and_temperature() {
        let mut w = test_world();
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 40, 40, 8, 150.0);

        assert!(w.field_at(40, 40).pressure.abs() > 1.0, "no pressure impulse");
        assert!(w.field_at(40, 40).temperature > 20.0, "no heat spike");
    }

    #[test]
    fn debris_velocity_varies_within_a_single_field_tile() {
        // Diagnosis this section fixed: `debris_velocity` samples the field
        // (via `World::field_at`, a coarse block lookup -- see its own doc)
        // at exactly +/- FIELD_SCALE, so every cell within roughly one field
        // tile read the exact same quantized pressure gradient and launched
        // with identical velocity, reading as a moving block rather than a
        // scatter of debris. An entirely open world (no walls to trip the
        // blocked-fallback path instead of the real gradient one) with a
        // real pressure impulse, so `x = 34` and `x = 35` (`y` held fixed)
        // land in the same coarse blocks for all four samples and would
        // read a bit-identical gradient before this section's jitter --
        // confirmed to fail (`vx1 == vx2 && vy1 == vy2` exactly) with
        // `DEBRIS_JITTER_STRENGTH` temporarily zeroed.
        let mut w = test_world();
        w.add_pressure_impulse(40, 40, 8, 200.0);
        let (vx1, vy1) = debris_velocity(&w, 34, 34, 40, 40, 200.0, &tuning());
        let (vx2, vy2) = debris_velocity(&w, 35, 34, 40, 40, 200.0, &tuning());
        assert!(
            (vx1 - vx2).abs() > 0.01 || (vy1 - vy2).abs() > 0.01,
            "adjacent cells reading the same coarse field block launched with identical velocity: \
             ({vx1}, {vy1}) vs ({vx2}, {vy2})"
        );
    }

    #[test]
    fn an_explosion_at_the_centre_throws_debris() {
        let mut w = test_world();
        for y in 30..50 {
            for x in 30..50 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 40, 40, 8, 150.0);

        assert!(!particles.is_empty(), "no debris was thrown at all");
        // At least some debris near the centre should be moving with real
        // speed, not sitting at zero velocity.
        let any_fast = particles.iter().any(|p| p.vx.abs() > 0.5 || p.vy.abs() > 0.5);
        assert!(any_fast, "debris was thrown with no meaningful velocity");
    }

    #[test]
    fn most_of_the_blast_radius_becomes_debris_not_vaporized() {
        // A dense fill spanning past the whole blast radius, so every
        // cleared cell had material to begin with (no early-continue on an
        // already-empty cell skewing the count). Checks that the *bulk* of
        // the affected area is thrown as debris, not just a lucky handful
        // near the epicentre -- the actual complaint the old `1.0 -
        // sqrt(dist / radius)` curve produced (most of a circle's area sits
        // in its outer band, where that curve's odds were already down to
        // single digits).
        let mut w = test_world();
        for y in 20..60 {
            for x in 20..60 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        //
        // Run at `debris_fraction: 1.0` deliberately. That fraction is a
        // separate, later dial for how *many* of the eligible cells throw
        // debris (see its own doc — the shipped default is below 1.0 because
        // ~2500 particles per blast was both slower and worse-looking than
        // fewer, longer-lived ones). What this test guards is the thing that
        // actually regressed: the *vaporize curve*, i.e. how much of the
        // blast area is eligible at all. Holding the sampling dial at 1.0
        // isolates that, so tuning debris density later can never silently
        // satisfy this test while the old curve creeps back.
        let mut w = test_world();
        for y in 20..60 {
            for x in 20..60 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        let radius = 10;
        // R2: this fill is exactly "enclosed on all sides by 20+ cells of
        // material" (the block spans 20 cells past the epicentre on every
        // side) -- under the new contract that is a *contained* charge, and
        // a contained charge deliberately clears only its small crush core
        // (`confined_cavity_fraction`), not the vaporize-curve question
        // this test exists to isolate. `containment_floor: INFINITY`
        // reproduces the pre-R2 geometry exactly (every sector reads open),
        // which is what "most of the blast radius" below is measured
        // against -- same trick `debris_fraction`'s own isolation above
        // uses for a different axis of the same tuning.
        let all_debris = Tuning { debris_fraction: 1.0, smoke_fraction: 0.0, containment_floor: f32::INFINITY, ..Tuning::default() };
        trigger_tuned(&mut w, &mut particles, 40, 40, radius, 150.0, &all_debris);

        let cleared = ((40 - radius)..=(40 + radius))
            .flat_map(|y| ((40 - radius)..=(40 + radius)).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let (dx, dy) = (x - 40, y - 40);
                dx * dx + dy * dy <= radius * radius
            })
            .filter(|&(x, y)| w.get(x, y).is_empty())
            .count();
        // Debris particles *and* chunk bodies. Both are "the blast turned
        // this into something that flies", which is what this test actually
        // guards -- the alternative being vaporized, i.e. silently gone.
        // Counting only particles used to be the same thing; it stopped
        // being so once a blast started cracking its crater wall into
        // coherent pieces, which takes rock that would have been grit and
        // makes it chunks instead. Scoring that as a loss would have this
        // test pushing against the feature.
        let body_cells: usize = w.chunk_bodies.iter().map(|b| b.cells.len()).sum();
        let debris_count = particles.iter().count() + body_cells;
        assert!(cleared > 0, "test setup: nothing was cleared at all");
        assert!(
            (debris_count as f32) > (cleared as f32) * 0.7,
            "most of the cleared blast radius should have become debris, not vaporized: \
             {debris_count} debris (particles + chunk cells) out of {cleared} cleared cells"
        );
    }

    /// The shipped defaults must still throw a substantial amount of debris.
    ///
    /// `most_of_the_blast_radius_becomes_debris_not_vaporized` above pins
    /// its own `debris_fraction` to isolate the vaporize curve, which means
    /// it would happily pass with the shipped fraction set to something
    /// invisible. This is the other half: whatever that dial is set to, a
    /// default blast has to produce debris on the order of hundreds of
    /// cells, because "I want to see sand flying" is the report that got
    /// this whole mechanism rebuilt once already.
    #[test]
    fn the_shipped_defaults_still_throw_plenty_of_debris() {
        let mut w = test_world();
        for y in 10..120 {
            for x in 10..120 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 64, 64, 20, 180.0);
        let n = particles.len();
        assert!(n > 300, "a default blast threw only {n} debris particles");
    }

    /// R2's whole point, stated positively (§7c(i)): a stone charge with
    /// 20+ cells of cover in every direction is contained, and a contained
    /// charge crushes a small pocket and drives fissures into the standing
    /// rock instead of clearing the old uniform circle. `boom_stone`'s own
    /// acceptance bar (`Reports/explosion-stone-review.md` §7d) is the same
    /// shape at world scale: cracked census up, cells lost down.
    #[test]
    fn a_fully_buried_stone_blast_crushes_a_pocket_and_scores_a_crack_star() {
        let mut w = test_world();
        // 108 cells of stone around the epicentre -- comfortably past
        // `containment_floor * radius` (28 at the shipped default) in every
        // direction, including the diagonals, so every one of the 16
        // probe sectors reads contained regardless of the fill's square
        // shape.
        for y in 10..118 {
            for x in 10..118 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        let radius = 20;
        trigger(&mut w, &mut particles, 64, 64, radius, 180.0);

        let in_disc = |x: i32, y: i32| {
            let (dx, dy) = (x - 64, y - 64);
            dx * dx + dy * dy <= radius * radius
        };
        let disc_cells: Vec<(i32, i32)> = ((64 - radius)..=(64 + radius))
            .flat_map(|y| ((64 - radius)..=(64 + radius)).map(move |x| (x, y)))
            .filter(|&(x, y)| in_disc(x, y))
            .collect();
        // Not `is_empty()`: `smoke_fraction` backfills part of a cleared
        // cell with `SMOKE`, which is not `EMPTY` and would undercount
        // "left the rock" the same way `an_explosion_clears_material_
        // within_its_radius`'s own comment explains. "No longer solid
        // stone" is the honest question for both directions of this test.
        let cleared = disc_cells.iter().filter(|&&(x, y)| w.get(x, y).material != material::STONE).count();
        // The crack halo's own rays start *beyond* `radius` on purpose
        // (`from = radius + BLAST_SHELL_REACH + 1`, see `Blast::new`'s own
        // doc) -- a census confined to the nominal blast disc would find
        // none of them. `3 * radius` is the same box `filmstrip`'s own
        // cracked-cell census uses for exactly this reason.
        let box_r = radius * 3;
        let cracked = ((64 - box_r)..=(64 + box_r))
            .flat_map(|y| ((64 - box_r)..=(64 + box_r)).map(move |x| (x, y)))
            .filter(|&(x, y)| w.get(x, y).cracked())
            .count();

        assert!(cracked >= 100, "a fully-buried stone blast scored only {cracked} cracked cells within 3x its radius");
        assert!(
            cleared < disc_cells.len() / 2,
            "a fully-contained stone blast cleared {cleared} of {} disc cells -- containment did not shrink the cavity",
            disc_cells.len()
        );
    }

    /// R2's material term, the other direction from the test above (§7c(ii)):
    /// sand's `blast_resistance` (0.35) is well under stone's 1.0, so the
    /// *same* 20+-cells-of-cover geometry reads as open for sand and the
    /// blast clears its old uniform circle unchanged.
    #[test]
    fn sand_at_the_same_buried_geometry_still_clears_the_full_disc() {
        let mut w = test_world();
        for y in 10..118 {
            for x in 10..118 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        let radius = 20;
        trigger(&mut w, &mut particles, 64, 64, radius, 180.0);

        let in_disc = |x: i32, y: i32| {
            let (dx, dy) = (x - 64, y - 64);
            dx * dx + dy * dy <= radius * radius
        };
        let disc_cells: Vec<(i32, i32)> = ((64 - radius)..=(64 + radius))
            .flat_map(|y| ((64 - radius)..=(64 + radius)).map(move |x| (x, y)))
            .filter(|&(x, y)| in_disc(x, y))
            .collect();
        let cleared = disc_cells.iter().filter(|&&(x, y)| w.get(x, y).material != material::SAND).count();

        assert!(
            cleared > disc_cells.len() * 3 / 4,
            "sand under the same 20+-cell cover as the stone test cleared only {cleared} of {} disc cells -- containment should not have engaged for a low-resistance material",
            disc_cells.len()
        );
    }

    #[test]
    fn a_shockwave_flings_loose_material_beyond_the_crater() {
        // The other half of "I want to see sand flying": an explosion in
        // the *middle* of a large sand pile should actively throw sand from
        // beyond the crater too, not just leave a hole for the surrounding
        // pile to quietly avalanche into under gravity -- gravity/settling
        // is the only thing that ever moves a loose CA cell the blast
        // radius itself never touched, since the field's own pressure
        // impulse only ever pushes free particles, never settled grid
        // material.
        let mut w = test_world();
        for y in 10..70 {
            for x in 10..70 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        let radius = 8;
        trigger(&mut w, &mut particles, 40, 40, radius, 200.0);

        // Just past the crater's own edge, still well inside the shockwave
        // reach (`radius * tuning().shockwave_multiplier`).
        let just_beyond = radius + 2;
        let cleared_beyond_crater = ((40 - just_beyond)..=(40 + just_beyond))
            .flat_map(|y| ((40 - just_beyond)..=(40 + just_beyond)).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let (dx, dy) = (x - 40, y - 40);
                let dist2 = dx * dx + dy * dy;
                dist2 > radius * radius && dist2 <= just_beyond * just_beyond
            })
            .filter(|&(x, y)| w.get(x, y).is_empty())
            .count();
        assert!(
            cleared_beyond_crater > 0,
            "no sand just beyond the crater was picked up by the shockwave at all"
        );
    }

    #[test]
    fn the_shockwave_does_not_uproot_solid_material_beyond_the_crater() {
        // The shockwave (step 2.5) is scoped to loose material specifically
        // -- a blast can fling sand it never directly touched, but it
        // should not casually rip out a stone wall by the same mechanism,
        // only by actually clearing it (step 2) or breaking it structurally.
        let mut w = test_world();
        for y in 10..70 {
            for x in 10..70 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        let radius = 8;
        trigger(&mut w, &mut particles, 40, 40, radius, 200.0);

        let shockwave_radius = (radius as f32 * tuning().shockwave_multiplier).round() as i32;
        let untouched_beyond_crater = ((40 - shockwave_radius)..=(40 + shockwave_radius))
            .flat_map(|y| ((40 - shockwave_radius)..=(40 + shockwave_radius)).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let (dx, dy) = (x - 40, y - 40);
                let dist2 = dx * dx + dy * dy;
                dist2 > radius * radius && dist2 <= shockwave_radius * shockwave_radius
            })
            .all(|(x, y)| w.get(x, y).material == material::STONE);
        assert!(untouched_beyond_crater, "the shockwave uprooted stone that was never inside the crater");
    }

    #[test]
    fn shockwave_pickup_chance_never_goes_negative_across_the_whole_annulus() {
        // A rounding mismatch bug: an earlier version divided by the
        // *rounded* `shockwave_radius - radius` while the caller's loop
        // admitted cells against the *continuous* `radius *
        // tuning().shockwave_multiplier`. Whenever the multiplier rounds the
        // outer radius down, cells between the true and rounded edge still
        // pass the zone check but produced a negative chance -- caught
        // concretely at radius 3 (3 * 1.8 = 5.4, rounds to 5): a cell at
        // (dx, dy) = (5, 2), dist ~= 5.385, is inside the true continuous
        // radius but the old formula gave `1.0 - (5.385 - 3.0) / (5 - 3)` =
        // -0.19. `Rng::chance` treats negative as "never," so this bug
        // wouldn't panic, it would just silently narrow the annulus.
        for radius in 1..30 {
            let shockwave_radius = (radius as f32 * tuning().shockwave_multiplier).round() as i32;
            for dy in -shockwave_radius..=shockwave_radius {
                for dx in -shockwave_radius..=shockwave_radius {
                    let dist2 = (dx * dx + dy * dy) as f32;
                    if dist2 <= (radius * radius) as f32 || dist2 > (radius as f32 * tuning().shockwave_multiplier).powi(2) {
                        continue; // outside the annulus this radius actually admits
                    }
                    let chance = shockwave_pickup_chance(radius, dist2.sqrt(), &tuning());
                    assert!(
                        (0.0..=1.0).contains(&chance),
                        "radius={radius} dx={dx} dy={dy} dist={:.3} produced out-of-range chance {chance}",
                        dist2.sqrt()
                    );
                }
            }
        }
    }

    #[test]
    fn debris_is_thrown_away_from_the_epicentre_not_toward_it() {
        let mut w = test_world();
        for y in 30..50 {
            for x in 30..50 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 40, 40, 8, 200.0);

        // For every fast-moving particle, its velocity should point broadly
        // away from (40, 40), not toward it — checked via the cosine of the
        // angle between (position - centre) and velocity, which should be
        // strongly positive on average.
        //
        // The per-particle bound is a tolerance, not a strict `> 0.0`:
        // `debris_velocity` reads a *pressure gradient*, which already
        // legitimately grazes close to perpendicular for some positions near
        // a filled square's corner even with `DEBRIS_JITTER_STRENGTH` at 0 —
        // measured directly (temporarily zeroing the constant) at cos ~=
        // 0.14 (~81.9 degrees) for this exact scene, a thin pre-existing
        // margin that has nothing to do with jitter. `DEBRIS_JITTER_STRENGTH`
        // (added this section) then spends some of that margin on purpose —
        // the whole point is to scatter debris rather than have it launch in
        // lockstep — so a small number of already-marginal cells can graze
        // a few degrees past perpendicular. `COS_TOLERANCE` allows that
        // without allowing what this test actually exists to catch: a
        // genuine sign error that sends debris *backward into* the blast
        // (a strongly negative cosine, not a graze).
        const COS_TOLERANCE: f32 = -0.2;
        let mut checked = 0;
        let mut cos_sum = 0.0;
        for p in particles.iter() {
            let (dx, dy) = (p.x - 40.0, p.y - 40.0);
            let dist = (dx * dx + dy * dy).sqrt();
            let speed = (p.vx * p.vx + p.vy * p.vy).sqrt();
            if dist < 0.5 || speed < 0.5 {
                continue; // too close to the centre or too slow to judge direction
            }
            let cos = (dx * p.vx + dy * p.vy) / (dist * speed);
            assert!(
                cos > COS_TOLERANCE,
                "debris at ({}, {}) moving ({}, {}) points strongly toward the epicentre, not away (cos = {cos})",
                p.x,
                p.y,
                p.vx,
                p.vy
            );
            cos_sum += cos;
            checked += 1;
        }
        assert!(checked > 0, "no particle was far/fast enough to check direction on");
        // The population as a whole must skew strongly outward -- a real
        // sign-flip bug would show up here as a mean well below this, not
        // just one grazing cell.
        assert!(
            cos_sum / checked as f32 > 0.5,
            "debris does not skew outward on average: mean cos = {}",
            cos_sum / checked as f32
        );
    }

    #[test]
    fn an_explosion_in_a_corridor_does_not_throw_debris_through_the_wall() {
        // A vertical wall with a narrow corridor opening below it — debris at
        // the opening should be pushed along the corridor, not straight
        // through solid stone to the other side.
        let mut w = test_world();
        for y in 0..60 {
            w.set(60, y, Cell::new(material::STONE, 0));
        }
        // A one-cell gap in the wall at y=60..64 for the corridor.
        for x in 55..65 {
            w.set(x, 70, Cell::new(material::STONE, 0)); // floor of the corridor
        }
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 50, 65, 6, 150.0);

        // No particle should end up with a large positive vx (rightward,
        // through the wall at x=60) while still left of the wall.
        for p in particles.iter() {
            if p.x < 60.0 {
                assert!(
                    p.vx < 5.0,
                    "debris at x={} got a strong rightward push toward/through the wall: vx={}",
                    p.x,
                    p.vx
                );
            }
        }
    }

    #[test]
    fn an_explosion_ignites_material_just_beyond_the_cleared_radius() {
        // Oil spans a much wider area than the blast will clear, so there is
        // intact, flammable material left in the ring the fireball is
        // supposed to reach.
        let mut w = test_world();
        for y in 10..70 {
            for x in 10..70 {
                w.set(x, y, Cell::new(material::OIL, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        // `containment_floor: INFINITY` reproduces the pre-R2 geometry
        // exactly (every sector reads as open) -- same trick
        // `most_of_the_blast_radius_becomes_debris_not_vaporized` uses.
        // This test's own contract is about the clear/ignite *ordering*
        // bug its comment describes, not about confinement: a 60x60 fill
        // of oil (`blast_resistance` unset, defaults to stone's 1.0) reads
        // as buried under R2's model the same as solid rock would, and
        // without this pin the inner disc would only partially clear,
        // which is a different, true thing this test was never written to
        // check.
        let no_smoke = Tuning { smoke_fraction: 0.0, containment_floor: f32::INFINITY, ..Tuning::default() };
        trigger_tuned(&mut w, &mut particles, 40, 40, 8, 150.0, &no_smoke);

        // The clearing radius (8) must be empty — nothing left to ignite
        // there, which is exactly the bug this test is a regression guard
        // for: an earlier version tried to ignite this same inner region
        // *before* clearing it, and the clearing step then silently erased
        // every cell it had just set on fire.
        let inner_clear = (36..=44).all(|y| (36..=44).all(|x| w.get(x, y).is_empty()));
        assert!(inner_clear, "the clearing radius was not actually cleared");

        // The ring beyond it must be *hot*. Ignition itself is no longer
        // this module's job: `scorch` writes CA cell temperature and
        // `fire::try_ignite` decides, during the sweep, whether that is
        // enough to light a given material. Asserting on temperature here
        // rather than `is_burning()` is the honest boundary between the two
        // — see `oil_beside_a_blast_ignites_but_stone_does_not` below for
        // the end-to-end version that actually runs the sweep.
        let ring_hot = (25..55).any(|y| {
            (25..55).any(|x| {
                let (dx, dy) = (x - 40, y - 40);
                let d2 = dx * dx + dy * dy;
                d2 > 64 && w.get(x, y).temperature() as f32 > AMBIENT_TEMPERATURE as f32 + 100.0
            })
        });
        assert!(ring_hot, "explosion did not heat the intact ring around the blast");
    }

    /// The fireball must respect `flammability` — the bug that motivated
    /// replacing `World::ignite_circle`.
    ///
    /// `ignite_circle` is M14's debug force-ignite tool and sets *any*
    /// material burning regardless of its own properties, so a stone wall
    /// beside a blast caught fire exactly as readily as an oil pool. On a
    /// stone scene that burning ring was not a detail, it was the dominant
    /// visual of the entire explosion. `scorch` writes temperature instead
    /// and lets `fire::try_ignite` gate on `ignition_temperature`, which is
    /// finite for oil and infinite for stone.
    ///
    /// Runs the real sweep, because ignition happens there and not in
    /// `trigger` — the whole point of the change.
    #[test]
    fn oil_beside_a_blast_ignites_but_stone_does_not() {
        let burning_after = |fill: material::MaterialId| {
            let mut w = test_world();
            for y in 10..70 {
                for x in 10..70 {
                    w.set(x, y, Cell::new(fill, 0));
                }
            }
            let mut particles = ParticleSystem::new();
            trigger(&mut w, &mut particles, 40, 40, 8, 150.0);
            for _ in 0..30 {
                crate::sim::update::step(&mut w);
            }
            (10..70)
                .flat_map(|y| (10..70).map(move |x| (x, y)))
                .filter(|&(x, y)| w.get(x, y).is_burning())
                .count()
        };

        let oil = burning_after(material::OIL);
        let stone = burning_after(material::STONE);
        assert!(oil > 0, "oil beside a blast never caught fire");
        assert_eq!(stone, 0, "{stone} stone cells caught fire -- stone is not flammable");
    }

    #[test]
    fn a_zero_radius_explosion_does_not_panic() {
        let mut w = test_world();
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 40, 40, 0, 150.0);
        // Reaching this line without panicking is the assertion.
    }

    /// An explosion must not turn a promoted body's reserved container cells
    /// into flying nothing.
    ///
    /// `Cell::is_empty()` is managed-aware: a body's container cells hold
    /// `material::EMPTY` but report as *not* empty, because for the callers
    /// that motivated that behaviour the question is "is this position
    /// available to use". An explosion's question is the other one — "is
    /// there material here to destroy" — so routing it through `is_empty()`
    /// made it treat those cells as destructible and spawn debris particles
    /// carrying `material::EMPTY`, which then land and write themselves back
    /// into the world.
    ///
    /// Latent rather than live today: nothing in production promotes a body
    /// (`127e177`). Found by review.
    #[test]
    fn an_explosion_does_not_spawn_debris_made_of_nothing() {
        let mut w = test_world();
        let mut particles = ParticleSystem::new();

        // A reserved container cell: materially empty, but managed. This is
        // exactly the shape `LiquidBody` rasterizes around its own edges.
        let container = Cell::EMPTY.with_managed(true);
        for x in 60..70 {
            w.set_owned(x, 60, container);
        }
        assert!(!w.get(64, 60).is_empty(), "test setup: a container cell reads as not-empty");
        assert_eq!(w.get(64, 60).material, material::EMPTY, "test setup: but holds no material");

        trigger(&mut w, &mut particles, 64, 60, 12, 4.0);

        let nothing: usize = particles.iter().filter(|p| p.material == material::EMPTY).count();
        assert_eq!(nothing, 0, "{nothing} debris particles were spawned with no material at all");
    }
}
