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
//!
//! # What a blast does to rock, and why it stopped being a walked star
//!
//! The fracture pattern is no longer drawn. It is **read off the rock's own
//! grain** — see `sim::fracture_field` for the field and `JointSeams` below
//! for what a blast does with it. The short version, because three rejected
//! contact sheets are the reason it changed: a walker that re-rolls its
//! heading every cell cannot draw a straight segment, and a walker cannot
//! enclose a piece except by luck. Worley domain boundaries are straight by
//! construction and closed by construction, and that is the whole argument.
//!
//! `Tuning::crack_rays` still drives the old radial walker; what changed is
//! that it defaults to **zero**, so the fabric is the shipped pattern and
//! the walker is a knob for re-asking the question. `strike` and the chisel
//! keep `structural::FissureWalks` untouched — their sheets are archived and
//! their short odd-ray fans never showed the artifact.

use super::cell::{Cell, AMBIENT_TEMPERATURE};
use super::field::FIELD_SCALE;
use super::material;
use super::particle::ParticleSystem;
use super::rng;
use super::structural;
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
    ///
    /// **900 was the wrong end of that ramp and it was measured, not
    /// guessed.** The non-burning blend is capped at half strength, so every
    /// value past ~420 drew the *same* flat wash of `FIRE_TINT_HIGH`
    /// ([255, 210, 110]) at 50% into grey stone — desaturated bone-tan, not
    /// ember, which is the owner's "the orange glow around also doesn't look
    /// great". 260 lands in the deep-red half of the ramp where
    /// `FIRE_TINT_LOW` ([180, 55, 20]) still dominates, and the headroom
    /// costs nothing to give up: no shipped material has a finite
    /// `ignition_temperature` at all (oil catches on a flammability roll in
    /// `scorch`, which this does not touch), so the excess was buying
    /// ignition that nothing performs.
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
    /// How many steps each crack ray takes per frame while the star is
    /// growing — the *speed* the fissures race outward at.
    ///
    /// The whole star used to be written in one call on the bang frame and
    /// never touched again, and the owner's verdict was that it read as
    /// "a graphic stamped on the stone" rather than a fissure: a thing that
    /// appears whole and never moves is a decal whatever its outline. At
    /// `2`, twelve rays with ~10-55-step budgets finish over roughly 25-35
    /// frames — about half a second of tips visibly extending after the
    /// flash, which is what the crack *reads* as rather than what any
    /// physical crack front does (a real one crosses that rock in
    /// microseconds; legibility wins, per `CLAUDE.md`'s ethos section).
    ///
    /// Larger is faster and less visible; `0` would freeze the star
    /// half-drawn forever, so it is clamped to at least 1 where it is read.
    /// `crack_growth` huge together with `crack_stagger` 0 restores the old
    /// instant star exactly.
    pub crack_growth: u32,
    /// The longest a crack ray may wait, in frames, before it starts —
    /// each ray draws its own delay from a position-keyed jitter over
    /// `0..crack_stagger`.
    ///
    /// Without it every ray leaves the crater on the same frame and the
    /// star grows as one synchronised starburst, which is the stamp
    /// complaint again one derivative up. At `8.0` a few rays leap out with
    /// the flash and the rest join in a ragged wave. `0.0` starts them all
    /// together.
    pub crack_stagger: f32,
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
    /// How deep a collar the crack star calves off the crater rim once it
    /// has finished growing, in cells past `radius`.
    ///
    /// This is the mechanic that answers "I don't see the pieces moving at
    /// all after the crack". The star was a *graphic*: the fissures were
    /// written, the load model heard about them, and nothing was ever
    /// released, so the two-beat the whole mechanism promises — cracks race
    /// outward, then the rim lets go — only ever played its first beat.
    /// `rigid::take_fragment` refuses to flood across a cracked edge, so the
    /// collar comes apart **along the fissures the player just watched
    /// grow**: wedge pieces bounded by visible cracks, not BFS rings.
    ///
    /// 8 is a rim, not a second crater: deep enough that a piece is a piece
    /// rather than a chip, shallow enough that the calving cannot be
    /// mistaken for the blast having a larger radius than it does. `0` turns
    /// calving off entirely.
    pub calve_depth: u32,
    /// What fraction of a scorched cell's heat *above ambient* survives each
    /// frame once the blast's own stages are done — the afterglow fade.
    ///
    /// # Why the blast has to own this
    ///
    /// Scorched stone never cools by itself, and that is not a bug in
    /// `fire.rs`: stone's `heat_conductivity` is 0, which puts it on
    /// `fire::update`'s thermally-inert fast path, and that path returns
    /// before any decay runs. It is a real and deliberate optimisation —
    /// most shipped materials are inert on every thermal axis, and a hot
    /// disc's *interior* would not cool by diffusion anyway. The
    /// consequence was that a blast's 900-degree ring was **permanent**:
    /// measured at 2,665 cells still above ambient and unchanged from frame
    /// 121 to frame 361 on `boom_stone`, which is a sticker of a fireball
    /// pasted on the rock forever.
    ///
    /// Giving stone conductivity would fix it by making every stone cell in
    /// the world pay for the one event that ever heats them. So the blast
    /// cools what the blast heated, for as long as the blast exists, and
    /// then both are gone.
    ///
    /// `0.94` takes the default flash (260, i.e. 240 above ambient) back
    /// under a degree in about 90 frames — a second and a half of visible
    /// fade. `1.0` (or anything above) turns the fade off and restores the
    /// permanent halo exactly, which is how the older tests pin the
    /// contract they were written against. It cannot hang the game whatever
    /// it is set to: `AFTERGLOW_MAX_FRAMES` bounds the fade regardless.
    pub afterglow_retention: f32,
    /// Temperature the crack walkers write into the rock they score, in
    /// Celsius — the incandescent tip. `0` is off.
    ///
    /// Separate from `flash_temperature` rather than reusing it: they are
    /// different events (a fireball venting into a cavity, and rock parting
    /// under a shock front) and the tip wants to read a shade hotter than
    /// the halo it is racing away from, or it disappears into it. Stone
    /// cannot ignite from this (`flammability` 0, and `fire::try_ignite`
    /// needs a finite `ignition_temperature` nothing shipped has); a walker
    /// crossing wood heats it, which is honest — it is a hot fracture — and
    /// the walk stops at anything that is not `Solid` anyway.
    pub crack_glow_temperature: f32,
    /// Divides `strength` to give each debris particle its pierce budget —
    /// cells of loose material it may punch through before coming to rest
    /// (`particle::Particle::pierce`).
    ///
    /// Scaled by `strength` rather than fixed, so a bigger charge reaches
    /// further through cover, but deliberately *not* by `radius`, which
    /// already sets how much material is thrown; conflating the two would
    /// make a wide, weak blast punch as far as a narrow, violent one.
    pub pierce_divisor: f32,
    /// How far the **joint fabric** reaches, as a multiple of `radius` —
    /// the outer edge of the region where a blast wakes the rock's own
    /// grain (`sim::fracture_field`, and `JointSeams` below).
    ///
    /// Distinct from `crack_reach`, which belongs to the radial walker and
    /// still means what it always did. Both exist because `crack_rays` is a
    /// hybrid knob: at its default of `0` the fabric is the whole pattern,
    /// and at `4`-`6` a fan of walked rays rides on top of it for an A/B.
    ///
    /// This is the **nominal** radius: `JointSeams::wake` scales it by how
    /// confined the charge is and how far it stands off the ground, so an
    /// unconfined shot reaches up to twice this and a standoff shot much less
    /// (`JointExposure`). A fully buried, in-contact charge gets exactly this.
    ///
    /// There is **no hard cut at this radius.** It is the distance at which
    /// the activation ramp reaches zero, and a joint activates only if its
    /// own draw falls under the ramp — so the damaged region's edge is
    /// ragged, with some joints reaching much further out than their
    /// neighbours. Clipping the output at a radius instead is the mistake
    /// that shipped the round-3 caves with a sawn-off face at their
    /// envelope edge, diagnosed and fixed once already in this repo
    /// (`CAVE_EDGE_FADE`).
    #[serde(default = "default_joint_reach")]
    pub joint_reach: f32,
    /// The inner zone, as a fraction of `joint_reach * radius`: inside it an
    /// activated joint **opens** into a one-cell seam of void and rubble;
    /// outside it, out to the full reach, it is only **scored**.
    ///
    /// This is the knob that decides how bold the pattern is *and* what it
    /// costs, and it is deliberately the same number for both. Opening
    /// removes material, so if a seed sweep says the blast is eating the
    /// world this is the first lever to reach for — the black lines in the
    /// reference the owner asked for are carved void, and void is the only
    /// thing that reads like that, so the trade is real and has to be made
    /// in the open rather than hidden behind a darker crack colour.
    ///
    /// `0.0` scores everything and removes nothing, which is the "cracks
    /// only" behaviour the pattern had before the fabric existed.
    #[serde(default = "default_joint_open_fraction")]
    pub joint_open_fraction: f32,
    /// The height of the activation ramp at the crater wall — the fraction
    /// of joints that wake in the thick of it.
    ///
    /// At `1.0` every boundary inside the crater's own radius activates and
    /// the near rock is fully diced into closed polygons. Lower values
    /// thin the whole pattern out evenly, which is the second density lever
    /// after `Material::joint_spacing`: that one changes how *big* the
    /// polygons are, this one changes how many of their edges are there at
    /// all. Prefer the spacing when the pattern reads as too fine, and this
    /// when it reads as too complete.
    #[serde(default = "default_joint_density")]
    pub joint_density: f32,
}

/// `2.4` — a radius-20 charge wakes joints out to about 48 cells, a little
/// over twice the crater. Sized against the walker star it replaces (whose
/// rays reached `radius * crack_reach` past `radius + 4`, so 55-ish cells at
/// the long end) so the halo covers comparable ground: this is a change of
/// *pattern*, not a change of how far a blast is felt.
fn default_joint_reach() -> f32 {
    2.4
}

/// `0.30`, and it is the number the nine-blast seed sweep chose.
///
/// This is the knob that pays for the look, so it was swept rather than
/// picked. Against `d9eec7f`, over seeds 1/3/7/24301, max of the four:
///
/// | setting | rock destroyed | promoted cells (max / **min**) | sites at the final tile |
/// |---|---|---|---|
/// | baseline, no fabric | 4,136 | 4,862 / **654** | 8,643 |
/// | `joint_spacing` 13, this at 0.45 | 10,539 | 16,266 / 11,641 | 21,577 |
/// | `joint_spacing` 16, this at 0.45 | 9,978 | 17,467 / 10,953 | 22,448 |
/// | **`joint_spacing` 13, this at 0.30** | **8,472** | **11,021 / 6,043** | **13,056** |
///
/// Two things to read out of that. **The spacing is not the cost lever** --
/// coarsening 13 to 16 cuts the boundary length by a fifth and the material
/// bill by a twentieth, because most of what a blast removes is not the seam
/// cells but the blocks the seams cut free. This is the lever. And the
/// *minimum* of the promoted column is the one that matters: 654 cells over
/// nine charges was the "no pieces move, ever" complaint as a number, and
/// every fabric setting clears it by an order of magnitude.
///
/// 0.30 also read best by eye at 60 frames after the bang -- at 0.45 the
/// near field is opened so completely that it collapses into one dark
/// region and the polygons stop being legible, which is losing the thing
/// this was built for in order to be bolder.
fn default_joint_open_fraction() -> f32 {
    0.30
}

/// `0.9` — near-complete polygons at the crater, thinning to nothing at the
/// reach. Not `1.0`: a handful of missing edges even in the thick of it is
/// what stops the near field reading as a drawn tessellation.
fn default_joint_density() -> f32 {
    0.9
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
            // The halo hugs the crater. At 0.5 the scorch ring reached half
            // a blast radius past a hole that is already 22 cells across,
            // which is a wash of tint over a quarter of the screen rather
            // than a rim of hot rock.
            fireball_fraction: 0.3,
            flash_temperature: 260.0,
            smoke_fraction: 0.18,
            heat_fraction: 3.0,
            speed_per_strength: 0.05,
            debris_jitter: 0.4,
            pierce_divisor: 12.0,
            // **Zero, and that is the shipped pattern.** The radial walker
            // is what the owner called a scribble three times over; the
            // fabric replaced it rather than joining it. The knob stays
            // reachable (4-6 gives fabric-plus-radials) so the question can
            // be re-asked from the tuning panel without a rebuild, which is
            // `CLAUDE.md`'s "ship a runtime selector rather than choosing".
            crack_rays: 0,
            crack_reach: 1.5,
            crack_growth: 2,
            crack_stagger: 8.0,
            containment_floor: 1.4,
            confined_cavity_fraction: 0.35,
            calve_depth: 8,
            afterglow_retention: 0.94,
            crack_glow_temperature: 300.0,
            joint_reach: default_joint_reach(),
            joint_open_fraction: default_joint_open_fraction(),
            joint_density: default_joint_density(),
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

/// Blur the probe's per-sector reach around the ring, twice.
///
/// The probe answers a *binary* question per direction -- did this ray
/// reach open air -- so the stored array is effectively two-valued
/// (`radius`, or `resistance x radius`), and nothing interpolated between
/// neighbouring sectors. A charge half under a face therefore cleared its
/// full radius in one 22.5-degree wedge and a third of it in the wedge
/// immediately beside it, with the step falling exactly on the sector line:
/// a hard pie cut, which is the second half of the owner's "geometrically
/// perfect" verdict. Two passes of `(a + 2b + c) / 4` turn that cliff into
/// a ramp across about three sectors, which is roughly a 70-degree
/// transition -- wide enough that the eye reads a crater lip rather than a
/// wedge, narrow enough that a fully contained charge (every sector equal)
/// is left bit-identical, since the kernel is normalised.
///
/// `u16` arithmetic with a `+2` before the divide, so the rounding is
/// to-nearest rather than always-down; two passes of always-down erosion
/// would visibly shrink an evenly contained crater for no reason.
///
/// # The blur may only raise a sector, never lower it
///
/// Measured, not assumed. A plain blur shaves the *peak* off any open run
/// narrower than about five sectors: a wall shot with three open sectors
/// came out at 19 against a radius of 20, so **no** sector passed the
/// `reach >= radius` test any more and the blast reported itself 16/16
/// contained. That is not a softer edge, it is a different blast --
/// `fracture_shell` gates on the same comparison, so the cave-wall case
/// lost its 703-cell overburden failure and both of its thrown bodies, and
/// the crater it left was *larger* while throwing nothing at all.
///
/// Clamping each pass against the probe's own reading keeps the venting
/// directions exactly as open as the probe found them and spreads the ramp
/// into the contained side only, which is the shape that was wanted: 7, 8,
/// 12, 20 across four sectors instead of 7, 20 across a line. The cost is
/// that a near-contained sector beside an open one clears a little further
/// than the probe said -- which is the correct direction for rock beside a
/// free face, and is the ramp itself.
fn smooth_sectors(reach: [u8; CONFINEMENT_SECTORS]) -> [u8; CONFINEMENT_SECTORS] {
    let mut out = reach;
    for _ in 0..2 {
        let src = out;
        for i in 0..CONFINEMENT_SECTORS {
            let prev = src[(i + CONFINEMENT_SECTORS - 1) % CONFINEMENT_SECTORS] as u16;
            let next = src[(i + 1) % CONFINEMENT_SECTORS] as u16;
            out[i] = (((prev + 2 * src[i] as u16 + next + 2) / 4) as u8).max(reach[i]);
        }
    }
    out
}

/// How far this blast reaches in the direction of `(dx, dy)`, with the
/// sector's own reach roughed up per cell.
///
/// The one place the crater's edge shape is decided, called from both
/// `Blast::clear_annulus` (which compares it against the cell's distance)
/// and `rigid::fracture_shell` (which compares it against the blast radius,
/// asking whether this cell's wedge is contained at all). Sharing the
/// function rather than the constant is deliberate: the two used to make
/// that decision in two different shapes -- a distance test and a binary
/// sector skip -- and a rim that clears in one and holds in the other is
/// exactly the self-refilling bruise R2 was built to remove.
///
/// Position-keyed (`rng::jitter`), never `world.rng`: the shape of a hole
/// is a property of the rock it is cut in, and determinism is required.
pub(crate) fn ragged_sector_limit(reach: &[u8; CONFINEMENT_SECTORS], dx: i32, dy: i32, x: i32, y: i32) -> f32 {
    reach[sector_of(dx, dy)] as f32 * (1.0 - (rng::jitter(x, y) - 0.5) * CRATER_RAGGEDNESS)
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

/// How the charge sits against the rock it is meant to crack -- the two
/// numbers `JointSeams` scales its halo by, bundled for the same reason
/// `Confinement` below is bundled: they always travel together, and one
/// `Copy` struct is also what keeps `JointSeams::wake` under clippy's
/// argument limit.
#[derive(Clone, Copy, Debug)]
pub(crate) struct JointExposure {
    /// Fraction of the 16 smoothed confinement sectors that vent to a free
    /// face: `0.0` fully buried, `1.0` an open-air burst.
    ///
    /// The fabric uses this to **compensate**, not to gate, and that is the
    /// opposite of what it means to `clear_annulus` and `fracture_shell`. A
    /// vented sector is a direction with no rock in it, so a halo of fixed
    /// radius wakes proportionally fewer joints on a surface shot than on a
    /// buried one purely because half its disc is sky. That is physically
    /// right and it reads as boring: 105 joints for the surface burst
    /// against 606 for the buried one on the owner's ten-GIF card, and his
    /// verdict was *"I don't like 3, not much happening."* So the reach is
    /// stretched by `1/sqrt(contained)` -- an unconfined charge reaches
    /// further into the rock it *does* have. `contained == 1` leaves the
    /// arithmetic bit-identical, which is what keeps the buried case from
    /// paying for the fix.
    vented: f32,
    /// Cells from the epicentre to the nearest ground, `0` when the charge
    /// is in it or on it. See `standoff_to_ground`.
    standoff: f32,
}

/// How far the charge stands off the ground it would crack, in cells.
///
/// **This is the distance term `Reports/explosion-stone-review.md` §15d
/// asked for and deliberately did not build.** Four of that sweep's 36
/// charges woke no joints at all, gated by `probe_confinement`'s
/// `struck_solid`: a charge whose every probe ray vents to air before
/// crossing a `Solid` cell reads as "no rock to crack" even with a hillside
/// two cells under it. The recorded reason for leaving it was that simply
/// dropping the gate makes an *airburst* dice the ground beneath it at full
/// ramp -- the ramp is flat out to the crater wall, and an airburst's
/// crater wall is over the ground. A standoff is what tells those two
/// apart: a surface burst is *in* the ground (`0`), an airburst is not.
///
/// Expanding square rings, the same shape as `structural::burial_depth` and
/// for the same reason: the first ring holding ground is the answer, and
/// this runs **once per blast, at trigger**, never per cell per frame.
///
/// Ground is `Solid`, `Powder` or `Plant`. A liquid is not, which is the
/// same reading `cast_confinement_ray` already gives it -- water displaces
/// rather than bracing on a detonation's timescale, so a charge under water
/// is a charge under open sky to this probe. Nothing in the harness stands
/// a charge off *through* water, so that is recorded rather than measured.
fn standoff_to_ground(world: &World, cx: i32, cy: i32, cap: i32) -> f32 {
    let is_ground = |x: i32, y: i32| {
        world.in_bounds(x, y)
            && matches!(
                world.materials.kind(world.get(x, y).material),
                material::MaterialKind::Solid | material::MaterialKind::Powder | material::MaterialKind::Plant
            )
    };
    if is_ground(cx, cy) {
        return 0.0;
    }
    for r in 1..=cap {
        let hit = (-r..=r).any(|d| is_ground(cx + d, cy - r) || is_ground(cx + d, cy + r) || is_ground(cx - r, cy + d) || is_ground(cx + r, cy + d));
        if hit {
            return r as f32;
        }
    }
    cap as f32
}

/// The floor under the fraction of a charge's surroundings taken as rock,
/// and therefore the cap on how far the vent compensation may stretch the
/// halo: `1/sqrt(0.25)` is `2.0`, so no charge reaches more than twice its
/// nominal joint radius however open it is.
///
/// A cap is needed at all because the compensation divides by the contained
/// fraction and a fully open charge's is zero. Two is the number because
/// the shortfall it compensates for is about that: a surface shot's disc is
/// roughly half rock, and area goes as the square of the radius.
const JOINT_VENT_FLOOR: f32 = 0.25;

/// How much clear air, as a fraction of the crater radius, takes a charge's
/// coupling into the grain from full to nothing.
///
/// `0.5` -- half a crater radius. Air is a bad coupler: a charge that is
/// not touching the rock puts its energy into the atmosphere and what
/// reaches the grain falls off fast. It scales the reach **and** the
/// activation density, so a standoff charge leaves a small, sparse
/// craquelure instead of the fully diced near field a contact charge
/// leaves, which is exactly the outcome §15d refused to ship.
///
/// Every charge in the harness except the airburst has a standoff of `0`
/// and is untouched by this.
const JOINT_CONTACT_STANDOFF: f32 = 0.5;

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
    /// Cells the crack walkers scored with a fresh fissure — the radial
    /// halo, R1's whole point.
    ///
    /// **Accumulates while the star grows**, rather than being final on the
    /// bang frame the way it was when the whole halo was written in one
    /// call. Read it once the blast has finished (`Blasts::is_empty`), which
    /// is what `filmstrip`'s report line already waits for; sampled mid-blast
    /// it is a progress reading, not a total.
    pub cells_fissured: u32,
    /// Cells the calving collar handed to the fragmenter once the star had
    /// finished growing — rim that let go along the cracks.
    ///
    /// The "did it fire at all" counter for K2, and it needs to be a counter
    /// rather than a look at the sheet for the reason `CLAUDE.md` records
    /// against exactly this mechanism: loose rubble that happens to hold its
    /// shape and a calved wedge tumbling into the crater are the same grey
    /// pixels at the zoom a contact sheet is read at. A buried blast scored
    /// **zero** bodies before this existed, which is the number to watch.
    pub calved: u32,
    /// Joints the fabric **opened** into seams of void and rubble, on the
    /// bang frame. See `JointSeams`.
    ///
    /// The "did it fire at all" counter for the joint fabric, and it needs
    /// to be one for the reason `CLAUDE.md` keeps re-learning: a walked
    /// crack star and a Worley boundary web are the same grey scratches at
    /// the zoom a contact sheet is read at, and the sheet cannot say which
    /// mechanism drew them. Sanity-checked against cases that are known to
    /// be right -- an airburst, and a charge inside sand -- where both of
    /// these must read exactly zero, because there is no jointed rock to
    /// wake.
    pub joints_opened: u32,
    /// Joints the fabric **scored** -- severed without removing anything --
    /// accumulating as the front travels outward. Read it once the blast
    /// has finished, like `cells_fissured`: sampled mid-blast it is a
    /// progress reading, not a total.
    pub joints_scored: u32,
    /// Joints activated in total, counted once at trigger. Deliberately not
    /// derivable as `joints_opened + joints_scored`: those two only add up
    /// to this once the front has finished, and the *gap* between them
    /// mid-flight is the growth beat being visible.
    pub joints_activated: u32,
}

impl std::fmt::Display for BlastReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cleared {} cells, {} held by containment, {} fissured, {} calved, joints {} activated ({} opened, {} scored), sectors open {}/{} contained {}/{}",
            self.cells_cleared,
            self.cells_held_by_containment,
            self.cells_fissured,
            self.calved,
            self.joints_activated,
            self.joints_opened,
            self.joints_scored,
            self.open_sectors,
            CONFINEMENT_SECTORS,
            self.contained_sectors,
            CONFINEMENT_SECTORS,
        )
    }
}

/// One explosion in progress: a cavity front expanding outward from
/// `(cx, cy)`, one stage per frame.
/// Not `Copy` any more: `fissures` below owns a queue of walkers and a set
/// of scored cells, and a blast that could be duplicated by an accidental
/// move would duplicate a crack star in mid-growth with it. `Clone` stays
/// (`Blasts` derives it).
#[derive(Clone, Debug)]
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
    /// The crack star, still growing. `None` when the confinement probe
    /// never crossed rock (an airburst, or a charge entirely inside loose
    /// powder) — there is nothing to crack and the walks are not paid for.
    ///
    /// Built at construction and stepped a little on every frame, which is
    /// the whole of this mechanic: the geometry is exactly what the
    /// one-call version drew, only the *timing* of the writes changed. See
    /// `structural::FissureWalks`.
    fissures: Option<structural::FissureWalks>,
    /// The joint fabric's scored halo, still spreading. `None` for the same
    /// reason `fissures` is: nothing jointed was struck, so there is no
    /// grain to wake and the scan is not paid for. Its *opened* half has
    /// already happened by the time this exists -- that is at trigger, by
    /// design (see `JointSeams`).
    seams: Option<JointSeams>,
    /// Running totals for `Blasts::last_blast_report`. `open_sectors` and
    /// `contained_sectors` are set once at construction;
    /// `cells_cleared`/`cells_held_by_containment` accumulate as
    /// `clear_annulus` runs on each stage, and `cells_fissured` accumulates
    /// as the crack star grows — which now outlives the last stage.
    report: BlastReport,
    /// The collar is calved **once**, on the frame the star finishes. A flag
    /// rather than a test on the walks themselves, because `is_done` stays
    /// true for every frame afterwards and the blast is still alive through
    /// its afterglow — without this it would re-scan and re-fracture the rim
    /// once a frame for the rest of the fade.
    calved: bool,
    /// Frames of afterglow already run, against `AFTERGLOW_MAX_FRAMES`. The
    /// hard bound on a blast's life: no setting of `afterglow_retention` can
    /// make a `Blast` immortal, the same way `crack_growth` is clamped where
    /// it is read so that `0` cannot freeze the star half-drawn forever.
    afterglow_frames: u16,
    /// How far this charge's **own** damage reaches from the epicentre, in
    /// cells — the extent `Blasts::trigger_with` records with the
    /// disturbance (`structural::Disturbance::extent`).
    ///
    /// Computed here, at construction, from the numbers the blast actually
    /// used rather than from the nominal tuning: `joint_halo_reach` already
    /// carries C3's `1/sqrt(contained)` stretch and the standoff coupling,
    /// so an unconfined charge records the wider licence it earns and an
    /// airburst records almost none. Recomputing `radius * joint_reach` at
    /// the trigger site instead would drift from the halo the first time
    /// either was tuned.
    damage_extent: i32,
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
    /// Reports of blasts that have finished, in the order they finished,
    /// each with the site it fired at. Drained by the harness; the app's HUD
    /// keeps reading `last_blast_report` and is untouched.
    ///
    /// A queue rather than one slot because the owner's play surface fires
    /// several charges in a run and `last_report` keeps only the last: eight
    /// of nine reports were being silently overwritten, which is the
    /// "did it fire at all" failure `CLAUDE.md` records, one level up.
    ///
    /// Capped at `FINISHED_REPORT_CAP`, oldest dropped. That is a leak
    /// guard and not a policy — the harness drains this every frame, so the
    /// cap is only ever reached by a caller that never drains at all.
    finished: Vec<(i32, i32, BlastReport)>,
}

/// How many finished reports `Blasts::finished` will hold before it starts
/// dropping the oldest. See the field's own doc: a leak guard, not a policy.
const FINISHED_REPORT_CAP: usize = 64;

/// Push one finished report, dropping the oldest if the queue is full.
///
/// A free function rather than a method because `Blasts::step` pushes from
/// inside `retain_mut`'s closure, which already holds `self.active` mutably
/// — the same split-borrow `last_report` needs there, and a `&mut self`
/// method would take the whole struct.
fn push_finished(finished: &mut Vec<(i32, i32, BlastReport)>, cx: i32, cy: i32, report: BlastReport) {
    if finished.len() >= FINISHED_REPORT_CAP {
        finished.remove(0);
    }
    finished.push((cx, cy, report));
}

impl Blasts {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start from tuning loaded off disk rather than the compiled defaults.
    pub fn with_tuning(tuning: Tuning) -> Self {
        Self { active: Vec::new(), tuning, last_report: BlastReport::default(), finished: Vec::new() }
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
        let mut blast = Blast::new(world, cx, cy, radius, strength, &self.tuning);
        // A blast is a disturbance: it licenses failures near it. See
        // `World::chain_reach`, and `Blast::damage_extent` for why the
        // licence is a *volume* -- a leash measured from the epicentre alone
        // is inside the charge's own crater at TIGHT.
        //
        // **Recorded after `Blast::new`, not before**, because the extent is
        // read off the blast rather than recomputed. Nothing between the two
        // consults `disturbances`: the only reader is `structural::tick`,
        // which runs from the scheduler on a later frame.
        world.record_disturbance(cx, cy, blast.damage_extent);
        let still_going = blast.advance(world, particles, &self.tuning);
        self.last_report = blast.report;
        if still_going {
            self.active.push(blast);
        } else {
            // A blast whose whole life is its trigger frame never reaches
            // `step`, so it has to be queued here or it is the one blast in
            // a run that produces no per-site report at all.
            push_finished(&mut self.finished, cx, cy, blast.report);
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
        let finished = &mut self.finished;
        self.active.retain_mut(|blast| {
            let still_going = blast.advance(world, particles, &tuning);
            *last_report = blast.report;
            if !still_going {
                push_finished(finished, blast.cx, blast.cy, blast.report);
            }
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

    /// Take every report queued since the last call, each with the site its
    /// blast fired at, in the order the blasts finished.
    ///
    /// The reason this exists rather than `last_blast_report` being read
    /// once per blast: a caller cannot tell from outside *which* frame a
    /// given blast finished on, and two overlapping blasts collapse into one
    /// slot. See `Blasts::finished`.
    pub fn drain_finished_reports(&mut self) -> Vec<(i32, i32, BlastReport)> {
        std::mem::take(&mut self.finished)
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
        let (probed, struck_solid) = probe_confinement(world, cx, cy, radius, tuning);
        // Smoothed before anything reads it, so the contained-to-open
        // transition is a ramp over a few sectors rather than a cliff on a
        // 22.5-degree line. See `smooth_sectors`. The open/contained
        // *report* is counted off the smoothed array too, because the
        // smoothed array is what `clear_annulus` and `fracture_shell`
        // actually obey -- a report counted off the raw probe would
        // describe a blast that never happened.
        let sector_reach = smooth_sectors(probed);
        let open_sectors = sector_reach.iter().filter(|&&r| r as i32 >= radius).count() as u32;
        let contained_sectors = CONFINEMENT_SECTORS as u32 - open_sectors;

        // R1: the radial fracture halo, scored now rather than on any later
        // stage. A fissure walk `break`s on the first non-body cell, and by
        // the blast's last stage the crater is empty *and* `fracture_shell`
        // has already removed the annulus out to
        // `radius + BLAST_SHELL_REACH` -- a walk starting at `from = radius`
        // would die on its very first cell there, or inside re-landed
        // debris that eats it the same way. Starting here, before anything
        // is cleared, means the walks begin in what will remain standing
        // rock once the crater finishes expanding, so `from` has to clear
        // the band the shell fracture is about to take:
        // `radius + BLAST_SHELL_REACH + 1` (`Reports/
        // explosion-stone-review.md` §5's adversarial-verification finding
        // -- the one place the first draft of this spec was wrong).
        //
        // Gated on having actually crossed rock: an airburst, or a blast
        // entirely inside loose powder, has no rock to crack and should not
        // pay for the walks.
        //
        // **Not `rigid::score_cracks`, which is what this used to call.**
        // That draws a fan of exact straight rays at one shared length,
        // rigidly rotated by a single site-keyed jitter -- which at five
        // short rays reads as fissures and at twelve long ones reads as an
        // asterisk. Worse, an even ray count makes opposite rays exact
        // point reflections of each other, so the owner's verdict off the
        // first contact sheet was "perfectly uniform and mirrored"
        // (`Reports/explosion-stone-review.md` §8b). `score_cracks` stays
        // exactly as it is for `strike`/`mine_swept`, whose short odd-ray
        // fans never showed the artifact; the blast simply stopped calling
        // it. Three things break the symmetry here, and all three are
        // needed:
        //
        // - **per-ray jitter inside its own fan slot**, not one rotation of
        //   the whole fan, so the spacing is uneven as well as unaligned;
        // - **heavy-tailed per-ray length** -- `j` squared biases short, so
        //   most spokes are stubs and the occasional one runs a long way,
        //   the same distribution `fragment_rungs` uses for the same
        //   "a distribution, not a uniform" reason;
        // - **the organic walker**, which wanders and forks, so no two rays
        //   are congruent even where their slots and lengths agree.
        //
        // **The rays are built here and walked over the following frames**,
        // which is the one thing that changed after the owner watched the
        // star arrive fully formed: "it looks the same everytime, it looks
        // like a graphic stamped on the stone and not a realistic fissure.
        // It would be cool if you could see it grow." Every heading, length
        // and jitter key below is exactly what the one-call version used --
        // the pattern for a site is still a property of the rock, so a
        // repeat charge still retraces and deepens its own fissures -- and
        // only the *timing* of the writes moved. `Blast::advance` steps the
        // walks; the blast stays alive until they finish.
        let fissures = if struck_solid {
            let from = (radius + BLAST_SHELL_REACH + 1) as f32;
            let base = radius as f32 * tuning.crack_reach;
            // Brittle, and glowing: the two halves of what a blast's own
            // fissures are and a crush's are not. See
            // `structural::CrackStyle` for the shape and
            // `Tuning::crack_glow_temperature` for the heat.
            let glow = tuning.crack_glow_temperature.clamp(0.0, i16::MAX as f32) as i16;
            let mut walks = structural::FissureWalks::empty(true, structural::CrackStyle::Brittle, glow);
            for i in 0..tuning.crack_rays {
                let slot = rng::jitter(cx + i as i32 * 7919, cy);
                let theta = (i as f32 + slot) * std::f32::consts::TAU / tuning.crack_rays as f32;
                let j = rng::jitter(cx + i as i32 * 104_729, cy + i as i32);
                let length = (base * (CRACK_LENGTH_FLOOR + CRACK_LENGTH_SPREAD * j * j)) as usize;
                let start = (cx + (theta.cos() * from).round() as i32, cy + (theta.sin() * from).round() as i32);
                // One fork contributed by every third ray, ~`crack_rays / 3`
                // in total, and *where* any of them gets thrown is still the
                // walker's own position-keyed chance. The pool is shared
                // across the fan now rather than handed out one ray at a
                // time, because a resumable star is one queue: which ray
                // spends a fork depends on the round-robin order, and that
                // is the acknowledged difference between the two drivers
                // (`FissureWalks`' doc). It stays a *budget* either way, so
                // the fan cannot run away with branches.
                let forks = usize::from(i % 3 == 0);
                // Position-keyed start delay -- no `world.rng` draw, so a
                // replayed input sequence cannot diverge here and the same
                // site staggers the same way every time. Keyed on `i * 31`
                // so neighbouring rays do not draw neighbouring delays.
                let delay = (rng::jitter(cx + i as i32 * 31, cy) * tuning.crack_stagger).max(0.0) as u16;
                walks.add_ray(start, theta, length.max(1), forks, delay);
            }
            Some(walks)
        } else {
            None
        };

        // F: the joint fabric. Same trigger-time timing as the walker star
        // above, with one deliberate difference that the owner's other
        // complaint on that card decides: the **near** joints are opened
        // *now*, on the bang frame, not handed to the growth front.
        // Breakage used to arrive 7-15 seconds after the flash because it
        // waited on a structural relaxation wavefront travelling one cell
        // per five frames; opening the near seams at detonation puts
        // visible breakage at the bang and leaves the slower beat to the
        // scored halo, which is what should look like it is spreading.
        //
        // **No `struck_solid` gate here any more**, and that is the fix for
        // `Reports/explosion-stone-review.md` §15d: four of the 36 sweep
        // charges woke no joints at all because every probe ray vented to
        // air before crossing a `Solid` cell, which is what a shallow
        // charge on a slope looks like from the epicentre. The walker star
        // above keeps the gate -- it is the archived A/B path and stays
        // byte-identical -- and the fabric replaces it with the two
        // continuous terms §15d asked for: a standoff distance and a
        // confinement scale, both in `JointExposure`. `wake` still returns
        // `None` when nothing in reach is jointed, and now also when the
        // coupling has fallen to nothing.
        let mut joints_opened = 0;
        let mut joints_activated = 0;
        let exposure = JointExposure {
            vented: open_sectors as f32 / CONFINEMENT_SECTORS as f32,
            // Capped at the halo it would scale: past a whole nominal joint
            // reach of clear air the answer has stopped mattering, and the
            // ring search is quadratic in it.
            standoff: standoff_to_ground(world, cx, cy, (radius.max(1) as f32 * tuning.joint_reach).ceil() as i32),
        };
        let seams = JointSeams::wake(world, cx, cy, radius, tuning, exposure).map(|(seams, opened, activated)| {
            joints_opened = opened;
            joints_activated = activated;
            seams
        });

        // **What this charge does to the rock by itself**, as one radius.
        // See `Blast::damage_extent`, and note this is deliberately the
        // *widest* of the three things a blast writes rather than any one of
        // them: a licence that did not cover the whole wound would leash the
        // blast's own seams, which is the thing four commits just fixed.
        let damage_extent = {
            // The crater is a floor: a charge always removes its own hole,
            // even when nothing in reach is jointed and `wake` returned
            // `None`.
            let crater = radius.max(1);
            let fabric = joint_halo_reach(radius, tuning, exposure).ceil() as i32;
            // The archived walker star. `crack_rays` defaults to `0`, so
            // this contributes nothing on any shipped setting -- it is here
            // so that a sweep which turns the rays back on does not silently
            // record a licence narrower than the damage it draws. Longest
            // possible ray: the length draw is `base * (FLOOR + SPREAD * j*j)`
            // with `j` in `[0, 1)`.
            let star = if tuning.crack_rays > 0 {
                radius + BLAST_SHELL_REACH + 1 + (radius as f32 * tuning.crack_reach * (CRACK_LENGTH_FLOOR + CRACK_LENGTH_SPREAD)).ceil() as i32
            } else {
                0
            };
            crater.max(fabric).max(star)
        };

        Self {
            cx,
            cy,
            radius,
            strength,
            stage: 0,
            sector_reach,
            fissures,
            seams,
            report: BlastReport {
                open_sectors,
                contained_sectors,
                cells_cleared: 0,
                cells_held_by_containment: 0,
                cells_fissured: 0,
                calved: 0,
                joints_opened,
                joints_scored: 0,
                joints_activated,
            },
            calved: false,
            afterglow_frames: 0,
            damage_extent,
        }
    }

    /// Run one frame of the blast: one cavity stage while any are left, then
    /// one pass of crack growth. Returns whether the blast is still going.
    ///
    /// **A blast now outlives its own cavity.** The stages finish in
    /// `duration` frames (10 by default) and the fissures keep racing
    /// outward for another twenty or forty, so "still going" is the OR of
    /// the two rather than the stage count alone. The stage work is guarded
    /// on `stage < stages` for exactly that reason: without the guard the
    /// extra frames would keep expanding the annulus past `radius` and eat
    /// the world one ring per frame of crack growth. The synchronous
    /// `trigger_tuned` (`while blast.advance {}`) therefore still leaves the
    /// world in a final state with nothing left to step -- it just spins a
    /// few dozen more times.
    ///
    /// **And it now outlives its own cracks too**, by however long its
    /// afterglow takes to fade (`Tuning::afterglow_retention`), which is
    /// bounded by `AFTERGLOW_MAX_FRAMES` however that is tuned. The three
    /// phases overlap rather than queue: the cavity expands while the first
    /// rays leave, and the fade runs under the tail of the crack growth.
    fn advance(&mut self, world: &mut World, particles: &mut ParticleSystem, tuning: &Tuning) -> bool {
        let stages = tuning.stages();
        if self.stage < stages {
            self.advance_cavity(world, particles, tuning, stages);
        }
        // After the stage work, so a ray's first cells are scored into rock
        // this frame's annulus has already finished with rather than into
        // rock it is about to clear.
        let growing = if let Some(walks) = self.fissures.as_mut() {
            // Clamped to at least one step: `crack_growth: 0` would freeze
            // the star half-drawn and keep the blast alive forever, which is
            // a tuning value that hangs the game rather than one that turns
            // a feature off.
            self.report.cells_fissured += walks.advance(world, tuning.crack_growth.max(1) as usize);
            !walks.is_done()
        } else {
            false
        };
        // The fabric's own growth beat, and it is not optional: a pattern
        // that arrives whole is a decal whatever its outline, which is
        // precisely the complaint ("a graphic stamped on the stone") that
        // the walker star was staged to answer. The *opening* happened at
        // trigger; this is the scored halo spreading outward behind it, on
        // the same two knobs (`crack_growth` sets the front's speed in
        // cells per frame, `crack_stagger` how ragged its arrival is).
        let spreading = if let Some(seams) = self.seams.as_mut() {
            self.report.joints_scored += seams.advance(world, tuning);
            !seams.is_done()
        } else {
            false
        };
        // The second beat: the star has finished racing, so the rim it cut
        // free lets go. Deliberately gated on the *cavity* being finished
        // too, not on the star alone -- `crack_growth` set high enough
        // finishes the walks while the front is still expanding, and calving
        // a collar that `clear_annulus` is about to clear anyway would throw
        // pieces out of rock that is about to stop existing.
        if !self.calved && !growing && self.stage >= stages && self.fissures.is_some() {
            self.calved = true;
            self.report.calved += self.calve(world, tuning);
        }
        // Only after the last stage: while the front is still expanding,
        // `scorch` is writing the ring this would be taking back down, and
        // the two would fight over the same cells every frame.
        let cooling = self.stage >= stages && self.afterglow(world, tuning);
        self.stage < stages || growing || spreading || cooling
    }

    /// Break the crater rim off along the cracks, once, when the star is
    /// done — K2, the answer to "I don't see the pieces moving at all after
    /// the crack".
    ///
    /// Two collars, because a blast has two kinds of sector and *neither*
    /// released anything before this:
    ///
    /// - **Open sectors** get the real one: `radius .. radius + calve_depth`,
    ///   the same scan `fracture_shell` runs on each stage but deeper, and
    ///   thrown *inward*. A rim wedge that has been cut free on both sides
    ///   by fissures falls into the hole; it does not fly away from it,
    ///   which is what the shell fracture already does on the bang frame for
    ///   the rock the blast actually broke.
    /// - **Contained sectors** get a thin one, two cells into the crush
    ///   pocket. A fully buried charge has nowhere to throw anything and
    ///   used to produce *no* moving pieces at all (peak bodies 0, measured);
    ///   two cells of pocket wall dropping into its own cavity is small,
    ///   honest, and visible.
    ///
    /// `rigid::take_fragment` refuses to cross a cracked edge, so both
    /// collars come apart along the star rather than along BFS rings — the
    /// cracks the player just watched grow are the seams the pieces break
    /// on, which is the entire reason this is worth doing at the end of the
    /// growth instead of on the bang frame.
    fn calve(&mut self, world: &mut World, tuning: &Tuning) -> u32 {
        if tuning.calve_depth == 0 {
            return 0;
        }
        let confinement = Confinement { sector_reach: self.sector_reach, radius: self.radius };
        let depth = tuning.calve_depth as i32;
        let mut cells = 0;
        // Inward, so the force is negative: `rigid::promote` throws a piece
        // *away* from the origin it is given, and a calving rim goes the
        // other way.
        let force = -(self.strength * CALVE_FORCE);
        if self.report.open_sectors > 0 {
            cells += super::rigid::calve_collar(
                world,
                (self.cx, self.cy),
                self.radius,
                self.radius + depth,
                force,
                CALVE_SIZE_BIAS,
                confinement,
                super::rigid::ShellSectors::Open,
            );
        }
        if self.report.contained_sectors > 0 {
            // The pocket wall, wherever the smoothed reach put it. Taken off
            // the *tuning* rather than off `sector_reach` so the collar sits
            // just outside the pocket in every contained sector at once; the
            // per-sector raggedness inside `calve_collar` is what keeps it
            // from being a drawn ring.
            let pocket = ((self.radius as f32 * tuning.confined_cavity_fraction).round() as i32).max(1);
            cells += super::rigid::calve_collar(
                world,
                (self.cx, self.cy),
                pocket,
                pocket + POCKET_COLLAR_THICKNESS,
                force,
                0,
                confinement,
                super::rigid::ShellSectors::Contained,
            );
        }
        cells
    }

    /// Cool everything this blast heated, a little, every frame — K3's
    /// second half, and the one that makes the glow *honest*.
    ///
    /// Returns whether anything is still warm, which is what keeps the blast
    /// alive: a blast dies when its own heat is gone, so when the object
    /// goes there is no baked halo left behind it. See
    /// `Tuning::afterglow_retention` for why this cannot live in `fire.rs`.
    ///
    /// Three guards, all of them load-bearing:
    ///
    /// - **Never raise a temperature.** The new value is `min`'d against the
    ///   old one, so however this is tuned it cannot become a second heat
    ///   source; a blast may only ever take back what it put in.
    /// - **Never touch a burning cell.** `fire.rs` owns a cell that is
    ///   alight — its temperature is that fire's business, and cooling it
    ///   from here would put out a fire the blast started.
    /// - **Never write to a cell already at ambient.** Both a correctness
    ///   guard (it is the termination condition) and the whole frame cost:
    ///   settled cells are read and skipped, never written, so the box does
    ///   not keep its chunks awake once it has finished cooling.
    ///
    /// The box is the scorch disc's own bounding box, plus the walker-scored
    /// cells outside it (the star reaches much further than the fireball
    /// does). It is a *superset* of what the blast heated — a cell inside
    /// the box that some other event left warm gets cooled too. Accepted
    /// knowingly: the alternative is a per-blast set of every scorched cell,
    /// which is thousands of entries for a mechanism whose entire purpose is
    /// to return the area to ambient anyway.
    fn afterglow(&mut self, world: &mut World, tuning: &Tuning) -> bool {
        // At or past 1.0 the fade is off: nothing cools, and the blast does
        // not stay alive pretending to wait for it. This is the setting that
        // reproduces the permanent halo exactly, which is what the tests
        // written against that contract pin themselves with.
        if tuning.afterglow_retention >= 1.0 || self.afterglow_frames >= AFTERGLOW_MAX_FRAMES {
            return false;
        }
        self.afterglow_frames += 1;
        let retention = tuning.afterglow_retention.clamp(0.0, AFTERGLOW_MAX_RETENTION);
        let reach = front_reach((self.radius as f32 * (1.0 + tuning.fireball_fraction)).powi(2));
        let mut warm = false;
        for y in (self.cy - reach)..=(self.cy + reach) {
            for x in (self.cx - reach)..=(self.cx + reach) {
                warm |= cool_toward_ambient(world, x, y, retention);
            }
        }
        // The star's own cells, which run far outside the box above. Only
        // the ones that do: the overlap is most of the star and re-cooling a
        // cell twice in one frame would fade the near half of every fissure
        // at double rate.
        if let Some(walks) = self.fissures.as_ref() {
            for (x, y) in walks.scored() {
                if (x - self.cx).abs() > reach || (y - self.cy).abs() > reach {
                    warm |= cool_toward_ambient(world, x, y, retention);
                }
            }
        }
        // And the fabric's, for the same reason and with the same
        // don't-cool-twice guard: the joint halo reaches `joint_reach`
        // blast-radii out, well past the fireball's own box.
        if let Some(seams) = self.seams.as_ref() {
            for &(x, y) in &seams.revealed {
                if (x - self.cx).abs() > reach || (y - self.cy).abs() > reach {
                    warm |= cool_toward_ambient(world, x, y, retention);
                }
            }
        }
        warm
    }

    /// One stage of the cavity front — everything `advance` used to be.
    fn advance_cavity(&mut self, world: &mut World, particles: &mut ParticleSystem, tuning: &Tuning, stages: u16) {
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
                //
                // Roughed up per cell (`ragged_sector_limit`): the reach
                // itself is a smooth ramp round the ring now, and this is
                // what stops the *arc* it describes from being a drawn
                // circle. Without it a contained blast's edge is a clean
                // conic section, which is `Reports/design-philosophy.md`
                // §0a's named "reads as fake on sight" failure.
                let sector_limit = ragged_sector_limit(&self.sector_reach, dx, dy, x, y);
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

/// The joint fabric in action: what one blast does to the grain of the rock
/// around it.
///
/// # The three zones, and why the near one costs material
///
/// One field (`fracture_field::domain`), one distance, three outcomes:
///
/// | zone | what happens |
/// |---|---|
/// | inner, out to `joint_open_fraction` of the halo | the joint **opens**: the boundary cell becomes void or rubble — a visible dark seam, and grit |
/// | middle, out to `joint_reach * radius` | the joint is **scored**: the edge is severed, nothing is removed |
/// | outer | nothing |
///
/// The opening is the half that makes the pattern read like the reference
/// the owner pointed at, and it is not free. Those black lines in his image
/// are **carved void** — the cave field's own thresholded passages — not
/// darkened rock. A crack bit draws as a one-cell darkening that removes
/// nothing, and no amount of drawing it more boldly turns it into that.
/// Gas wedging the near joints open is the physical reading, and it buys the
/// look at material cost only where the blast really was.
///
/// # Two things it deliberately does *not* do
///
/// **No hard radius.** A joint activates iff its own position-keyed draw
/// falls under a ramp that decays to zero at the reach, so the damaged
/// region has a ragged edge with some joints reaching much further out than
/// their neighbours. Clipping at a radius instead is how the round-3 caves
/// shipped with a 70-row sawn-off face at their envelope edge — the same
/// artifact, diagnosed and fixed once already in this repo.
///
/// **No sector gating, and no gate of any kind.** `clear_annulus` and
/// `fracture_shell` both obey the confinement probe, because a contained
/// sector must not vent. Joints are the opposite case: a *confined* charge
/// is exactly the one whose energy goes into the grain instead of into a
/// cavity, so a fully buried shot wakes its joints all the way round while
/// clearing almost nothing. That asymmetry is the mechanism, not an
/// oversight.
///
/// Confinement does reach the fabric, but as **two continuous scales on
/// one radius**, never as a yes/no (`JointExposure`):
///
/// - the reach is stretched by `1/sqrt(contained)`, capped at 2x, so an
///   unconfined charge reaches further into the rock it *does* have. This
///   is compensation, not reward: a fixed radius over a half-sky disc wakes
///   half the joints for reasons of geometry alone, and the owner's verdict
///   on that was *"I don't like 3, not much happening."* Measured on his
///   own ten-GIF card, seed 1: the surface burst 105 -> 335 joints and the
///   shallow crater 144 -> 461, with the buried shot bit-identical at 606
///   because its `contained` is exactly 1.
/// - the reach *and* the activation density are both scaled down by the
///   charge's **standoff**, so a shot in clear air leaves a faint
///   craquelure rather than a diced field. The airburst at `depth=-8` goes
///   from 0 joints (it was gated out entirely) to 12 scored and **none
///   opened** -- it marks the ground and removes nothing from it.
#[derive(Clone, Debug)]
struct JointSeams {
    /// The scored halo, sorted by the distance at which the front reaches
    /// it, and consumed from `cursor` forward. Not a set: each entry is one
    /// edge, and an edge appears exactly once because it is owned by
    /// exactly one of its two cells (`Cell::FLAG_CRACK_RIGHT`).
    pending: Vec<PendingJoint>,
    cursor: usize,
    /// How far the fracture front has travelled, in cells. Compared against
    /// `PendingJoint::key`, which is a distance plus a per-boundary delay.
    front: f32,
    /// Cells the front has actually written to, kept only so the blast's
    /// afterglow can cool the ones it heated: the halo reaches
    /// `joint_reach` blast-radii out, far outside the fireball's own
    /// bounding box, so a box scan wide enough to contain it would be
    /// almost entirely untouched rock. Same trick, same reason as
    /// `FissureWalks::scored`.
    revealed: Vec<(i32, i32)>,
    /// Written into the rock a joint parts, if it is colder — the fracture
    /// front's own incandescence, shared with the walker star so the two
    /// halves of the hybrid knob glow alike. `0` is off.
    glow: i16,
}

/// One severed edge waiting for the front. `down` picks which of the two
/// edges `(x, y)` owns: its bottom edge, or its right one.
#[derive(Clone, Copy, Debug)]
struct PendingJoint {
    key: f32,
    x: i32,
    y: i32,
    down: bool,
}

/// How well the charge is coupled into the grain, `0.0` (no coupling at
/// all) to `1.0` (in or on the ground).
///
/// Scales the reach **and** the activation density, so a standoff charge
/// leaves a small sparse craquelure rather than the same diced near field
/// in a smaller patch. See `JOINT_CONTACT_STANDOFF`.
fn joint_contact(radius: i32, exposure: JointExposure) -> f32 {
    let crater = radius.max(1) as f32;
    (1.0 - exposure.standoff / (crater * JOINT_CONTACT_STANDOFF)).clamp(0.0, 1.0)
}

/// The outer limit of the joint fabric a charge wakes, in cells from the
/// epicentre — **the blast's own damage reach**, which is what
/// `World::record_disturbance` is given as this charge's extent.
///
/// Pulled out of `JointSeams::wake` rather than recomputed at the trigger
/// site, and that is the whole reason it is a function: the licence a blast
/// records and the halo it actually writes must be the *same* number, and
/// two copies of this expression would drift the first time either is
/// tuned. Note in particular that C3's surface-burst fix stretches the
/// reach by `1/sqrt(contained)` up to 2x, so the nominal `radius *
/// joint_reach` is **not** what an unconfined charge reaches.
///
/// **Confinement scales the halo; it never gates it.** See
/// `JointExposure::vented` for the measurement that asked for this and
/// `JOINT_VENT_FLOOR` for the cap. A fully contained charge has
/// `contained == 1`, so this multiplies by exactly one and the buried case
/// is bit-identical to what shipped.
fn joint_halo_reach(radius: i32, tuning: &Tuning, exposure: JointExposure) -> f32 {
    let crater = radius.max(1) as f32;
    let contained = (1.0 - exposure.vented).clamp(JOINT_VENT_FLOOR, 1.0);
    crater * tuning.joint_reach / contained.sqrt() * joint_contact(radius, exposure)
}

impl JointSeams {
    /// Wake the fabric around a charge: open the near joints **now**, and
    /// queue the far ones for the growth front.
    ///
    /// Returns `None` when nothing in reach is jointed at all — an airburst,
    /// a charge inside a sand pile, a shot into soil. That is the case the
    /// counters are sanity-checked against (`CLAUDE.md`: check what a metric
    /// says when nothing is wrong), and it must read zero rather than
    /// something small.
    ///
    /// Otherwise `(seams, joints opened, joints activated)`.
    fn wake(world: &mut World, cx: i32, cy: i32, radius: i32, tuning: &Tuning, exposure: JointExposure) -> Option<(Self, u32, u32)> {
        let crater = radius.max(1) as f32;
        let contact = joint_contact(radius, exposure);
        let reach = joint_halo_reach(radius, tuning, exposure);
        // A charge with no coupling left wakes nothing, and that is the
        // gate the `struck_solid` flag used to be -- except that it is a
        // *distance* now, so a shot two cells over a hillside is not the
        // same thing as one thirty cells over it.
        if reach < 1.0 {
            return None;
        }
        // Where the flat core of the ramp ends. It is the crater wall
        // whenever the halo is at least twice the crater, which is every
        // shipped setting and therefore leaves the arithmetic below exactly
        // as it was; it shrinks with the reach only for a standoff charge,
        // whose halo can be smaller than its own crater. Without that the
        // ramp would divide by a negative and an airburst would sit inside
        // its own flat core -- the "dices the ground at full ramp" outcome
        // §15d named.
        let flat = crater.min(reach * 0.5);
        // The opened zone is measured **from the crater wall outward**
        // (`flat`), not from the epicentre. Measured from the centre it
        // would be swallowed
        // whole by the hole on an open surface shot -- the crater clears
        // that ground anyway -- and the bold near seams would only ever
        // appear on buried charges. As a fraction of the halo it means the
        // same thing to both: the inner part of whatever rock is left
        // standing comes apart, and the rest is scored.
        let open_fraction = tuning.joint_open_fraction.clamp(0.0, 1.0);
        // `0.0` is a **hard off**, not the bottom of the ramp, and the
        // difference is not cosmetic: the zone is measured from the crater
        // wall *outward*, so the smallest positive setting still opens every
        // joint inside the nominal radius -- which for a contained charge is
        // rock that is standing there and would be removed. Its doc says
        // "scores everything and removes nothing", and this is what makes
        // that true. Caught by `debris_is_thrown_away_from_the_epicentre`,
        // which sets it to zero to take the fabric's removals out of a scene
        // whose pressure gradient it is reading, and still saw them.
        let open_to = if open_fraction <= 0.0 { f32::NEG_INFINITY } else { flat + (reach - flat) * open_fraction };
        // Scaled by the coupling for the same reason the reach is: what a
        // standoff charge leaves has to be a sparse craquelure, not the
        // same diced near field a contact charge leaves in a smaller patch.
        let density = tuning.joint_density.clamp(0.0, 1.0) * contact;

        // The domain map for the box, computed once. A cell's domain costs
        // nine hashes, and the edge test needs both cells' domains, so
        // caching halves the work outright -- and the box is scanned once
        // per blast, at trigger, never per frame.
        let r = reach.ceil() as i32;
        let (x0, y0) = (cx - r, cy - r);
        // One extra row and column: the last cell in the box still has to
        // be able to ask about the neighbour it owns an edge with.
        let (w, h) = ((2 * r + 2) as usize, (2 * r + 2) as usize);
        let idx = |x: i32, y: i32| ((y - y0) as usize) * w + (x - x0) as usize;
        // `(domain, pitch)`. The pitch travels with the domain because it is
        // per material: two different jointed materials meeting have
        // *different lattices*, and comparing a domain from one against a
        // domain from the other would be comparing lattice coordinates that
        // do not mean the same thing.
        let mut map: Vec<Option<((i32, i32), f32)>> = vec![None; w * h];
        let mut any = false;
        for y in y0..(y0 + h as i32) {
            for x in x0..(x0 + w as i32) {
                let (dx, dy) = (x - cx, y - cy);
                // A cell just outside the reach still has to be mapped: the
                // edge it shares with the last cell *inside* is a real
                // boundary and would otherwise read as "no domain".
                if ((dx * dx + dy * dy) as f32) > (reach + 1.0) * (reach + 1.0) {
                    continue;
                }
                if !world.in_bounds(x, y) {
                    continue;
                }
                let cell = world.get(x, y);
                if !structural::is_body_material(world, cell.material) {
                    continue;
                }
                // The hot-path gate, at the call site that already holds the
                // `Cell`: a `Vec` index on the resolved `Material`, never an
                // `id_of("stone")` string hash in a loop. Sand, soil, gravel
                // and snow leave this at `0.0` and are skipped here.
                // Banded, not the flat material constant: see
                // `fracture_field::pitch_at`. Two cells in different bands
                // read different pitches and the `other_pitch != pitch`
                // guard below stops the web at the contact, which is the
                // mechanism rather than a hole in it.
                let pitch = {
                    let m = world.materials.get(cell.material);
                    super::fracture_field::pitch_at(world.seed, x, y, m.joint_spacing, m.joint_band_contrast)
                };
                if pitch <= 0.0 {
                    continue;
                }
                map[idx(x, y)] = Some((super::fracture_field::domain(world.seed, x, y, pitch), pitch));
                any = true;
            }
        }
        if !any {
            return None;
        }

        let glow = tuning.crack_glow_temperature.clamp(0.0, i16::MAX as f32) as i16;
        let mut seams = Self { pending: Vec::new(), cursor: 0, front: 0.0, revealed: Vec::new(), glow };
        let mut opened = 0;
        let mut activated = 0;
        let mut to_open: Vec<(i32, i32)> = Vec::new();

        for y in y0..(y0 + h as i32 - 1) {
            for x in x0..(x0 + w as i32 - 1) {
                let Some((home, pitch)) = map[idx(x, y)] else { continue };
                let (dx, dy) = (x - cx, y - cy);
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                if d > reach {
                    continue;
                }
                // The activation ramp: flat at `joint_density` out to the
                // crater wall (`flat`, which is the crater wall for every
                // charge that is touching the rock), then linear to zero at
                // the reach. Flat rather
                // than falling from the epicentre because the near field has
                // to come apart into *closed* polygons -- half the boundary
                // of a cell is not a cell, it is a scribble again -- and
                // linear rather than clipped for the ragged edge.
                let t = ((d - flat) / (reach - flat)).clamp(0.0, 1.0);
                let ramp = density * (1.0 - t);
                for down in [false, true] {
                    let (nx, ny) = if down { (x, y + 1) } else { (x + 1, y) };
                    let Some((other, other_pitch)) = map[idx(nx, ny)] else { continue };
                    // Different lattices never share a joint: the web stops
                    // where the rock type does, which is what a bed of sand
                    // against a cliff actually looks like.
                    if other_pitch != pitch || other == home {
                        continue;
                    }
                    // The severing rule, in one line and with no threshold:
                    // an edge is a joint iff its two cells sit in different
                    // domains. That set is *exactly* the boundary of each
                    // domain on the 4-connected grid, so a domain whose
                    // boundary is fully severed is enclosed by construction
                    // -- which is the entire reason to prefer this to more
                    // walker work. See `fracture_field`'s module doc.
                    if super::fracture_field::joint_draw(world.seed, home, other) >= ramp {
                        continue;
                    }
                    activated += 1;
                    if d <= open_to {
                        to_open.push((x, y));
                    } else {
                        // `distance + delay` order, one slice per frame.
                        // The delay is per *boundary*, not per edge, so a
                        // whole straight segment races outward as one line
                        // instead of arriving in a dashed scatter.
                        let delay = super::fracture_field::joint_delay(world.seed, home, other);
                        seams.pending.push(PendingJoint { key: d + delay * tuning.crack_stagger.max(0.0), x, y, down });
                    }
                }
            }
        }

        // Opened last, in one pass, and that ordering matters: opening
        // *removes* cells, and a removed cell has no domain, so interleaving
        // it with the scan above would have later edges consulting a map
        // that no longer describes the world. Deduplicated, because a cell
        // that owns two activated joints is still one cell.
        let mut uniq = to_open.clone();
        uniq.sort_unstable();
        uniq.dedup();
        let mut done: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
        for (x, y) in uniq {
            if open_seam(world, x, y) {
                done.insert((x, y));
            }
        }
        // Counted in **edges**, not in cells removed, so that
        // `opened + scored == activated` once the front has finished and the
        // three numbers can be read against each other. A cell that owns two
        // activated joints is removed once and counts twice, which is right:
        // two joints did open there. How much material went is the census's
        // question, and the census answers it.
        opened += to_open.iter().filter(|c| done.contains(c)).count() as u32;

        // A total order, with the position as the tiebreak: determinism is
        // required (`PLAN.md`), and two joints at the same key would
        // otherwise reveal in whatever order the scan happened to push them.
        seams.pending.sort_by(|a, b| {
            a.key.partial_cmp(&b.key).unwrap_or(std::cmp::Ordering::Equal).then((a.y, a.x, a.down).cmp(&(b.y, b.x, b.down)))
        });
        Some((seams, opened, activated))
    }

    /// Advance the front by one frame and sever everything it has reached.
    /// Returns how many joints were newly scored.
    fn advance(&mut self, world: &mut World, tuning: &Tuning) -> u32 {
        if self.is_done() {
            return 0;
        }
        // `crack_growth` is "steps per frame" for a walker; here it is the
        // same thing measured in cells of front travel, so the two halves of
        // the hybrid knob spread at the same speed and one number still
        // means "how fast does fracture race outward". Clamped to at least
        // one for the reason it is clamped there: `0` would freeze the
        // pattern half-drawn and keep the blast alive forever.
        self.front += tuning.crack_growth.max(1) as f32;
        let mut scored = 0;
        while self.cursor < self.pending.len() && self.pending[self.cursor].key <= self.front {
            let j = self.pending[self.cursor];
            self.cursor += 1;
            if sever(world, j.x, j.y, j.down, self.glow, &mut self.revealed) {
                scored += 1;
            }
        }
        scored
    }

    fn is_done(&self) -> bool {
        self.cursor >= self.pending.len()
    }
}

/// Sever one edge: the crack bit, the glow, and the two pieces of
/// bookkeeping without which a fissure is decoration.
///
/// Returns whether this was *fresh* damage — a cell whose bit was already
/// set and which this only reheated is not new, the same meaning
/// `FissureWalks` gives `cells_fissured`.
///
/// **Only the one edge, and no mirror write.** The walker has to score both
/// perpendicular edges of every cell it visits, because a line drawn through
/// cells leaves the visited cell joined to the rock on the far side of the
/// line and a 4-connected flood threads straight out through the gap. A
/// domain boundary is not a line drawn through cells: it *is* the edge set,
/// it already contains every edge that separates the two domains, and adding
/// the perpendicular ones would cut into the polygons it is supposed to
/// enclose.
fn sever(world: &mut World, x: i32, y: i32, down: bool, glow: i16, revealed: &mut Vec<(i32, i32)>) -> bool {
    let cell = world.get(x, y);
    if !structural::is_body_material(world, cell.material) {
        return false; // the rock moved on since the scan; nothing left to part
    }
    let mut scored = if down { cell.with_crack_down(true) } else { cell.with_crack_right(true) };
    let fresh = scored != cell;
    // Never cools a cell that is already hotter -- the same one-way rule
    // `scorch` and the walker both use, for the same reason: two overlapping
    // blasts, or a joint opening through something on fire.
    if glow > 0 && scored.temperature() < glow {
        scored.set_temperature(glow);
        revealed.push((x, y));
    }
    if scored != cell {
        world.set(x, y, scored);
    }
    // A fissure is where rock has parted company with the mass behind the
    // slice, so both sides stop claiming to be braced by it, and both get
    // re-evaluated. Done whether or not the bit was already set: a repeat
    // charge has to re-loosen rock it has already scored, or the second shot
    // throws nothing.
    let (nx, ny) = if down { (x, y + 1) } else { (x + 1, y) };
    for (ax, ay) in [(x, y), (nx, ny)] {
        structural::detach_around_crack(world, ax, ay);
        world.schedule_structural_check_around(ax, ay);
    }
    fresh
}

/// Open one joint into a seam: the boundary cell becomes void, or grit.
///
/// **One cell wide, always.** The seam follows the boundary on the side of
/// the cell that *owns* the edge, so a run of boundary edges removes a run
/// of single cells. The reference's thickness comes from the lines being
/// void rather than from their being fat, and a two-cell seam eats the world
/// for no extra legibility.
///
/// Mostly void, some rubble. The void is what reads: a seam that is merely
/// darker is the crack bit again, which is the thing this exists to get past.
/// The rubble is the other half of the owner's brief — *"a few blocks, more
/// cobbles, a lot of grit"* — and it is real grit, converted through
/// `rigid::shatter_to_rubble` so it lands in the same `record_shattered`
/// census as every other kind, and so it falls, trickles down the seam and
/// piles up the way loose material in an open joint does.
fn open_seam(world: &mut World, x: i32, y: i32) -> bool {
    let cell = world.get(x, y);
    if !structural::is_body_material(world, cell.material) {
        return false;
    }
    // Position-keyed, never `world.rng`: geometry that drew from the world
    // stream would put a blast into the replay draw order and stop a repeat
    // charge from retracing its own seams. `JITTER_AXIS_OFFSET` reused as an
    // arbitrary fixed decorrelating offset, so this draw is not the same
    // number some other position-keyed choice at this cell already took.
    if rng::jitter(x + JITTER_AXIS_OFFSET, y) < SEAM_VOID_FRACTION {
        world.set(x, y, Cell::EMPTY);
        world.schedule_structural_check_around(x, y);
        // The loudest possible version of "this rock has been broken": what
        // the seam exposes stops being part of the mass behind it, so the
        // polygons it cuts out can actually come down. Without this every
        // joint in attached terrain would be a decoration, which is exactly
        // what the crack bits were before `detach_around_crack` existed.
        structural::detach_exposed_neighbours(world, x, y);
    } else {
        super::rigid::shatter_to_rubble(world, x, y);
    }
    true
}

/// What fraction of an opened seam cell is removed outright rather than
/// turned to grit.
///
/// Void-dominant, and picked by eye rather than measured: the seam has to
/// read as a **black line** at play zoom, and rubble is a mid-grey powder
/// that draws as more rock, so a seam made mostly of it would be the crack
/// bit again in a costlier form. Two thirds was not swept against a half or
/// a quarter -- if the seams ever read as too bold or too faint, this is
/// the first thing to try, and it is cheap to sweep.
///
/// The remaining third is not decoration. Grit is the small end of *"a few
/// blocks, more cobbles, a lot of grit"*, and the nine-blast sweep at
/// `d9eec7f` put promoted cells against shattered ones at roughly 1.6-1.9:1
/// per seed (1,785:1,086, 654:991, 4,862:2,542, 2,710:1,507) -- so the
/// engine was not short of grit in the ratio, it was short of *both*, which
/// is what the promoted-cells minimum of 654 across nine charges says.
const SEAM_VOID_FRACTION: f32 = 0.66;

/// Take one cell a fraction of the way back to ambient, and say whether it
/// is still warm afterwards.
///
/// Free-standing rather than a method because it is the *rule*, and the
/// rule is what the guard test drives directly: every one of the three
/// promises in `Blast::afterglow`'s doc is enforced here, in four lines, so
/// there is one place to read them and one place to break when checking
/// that the test can fail.
///
/// The excess is truncated to `i16`, which is what makes this terminate:
/// `retention` is strictly below 1, so an excess of 1 or more strictly
/// decreases, and anything under 1 snaps to exactly ambient.
fn cool_toward_ambient(world: &mut World, x: i32, y: i32, retention: f32) -> bool {
    if !world.in_bounds(x, y) {
        return false;
    }
    let mut cell = world.get(x, y);
    // `fire.rs` owns a burning cell's temperature, and nothing here may
    // touch it. Materially-empty cells hold no heat worth tracking.
    if cell.material == material::EMPTY || cell.is_burning() {
        return false;
    }
    let now = cell.temperature();
    if now <= AMBIENT_TEMPERATURE {
        return false; // at ambient, or genuinely cold -- either way, not ours to warm
    }
    let excess = (now - AMBIENT_TEMPERATURE) as f32 * retention;
    let next = if excess < 1.0 { AMBIENT_TEMPERATURE } else { AMBIENT_TEMPERATURE + excess as i16 };
    // Never *raise* a temperature, whatever the arithmetic above produced.
    let next = next.min(now);
    if next != now {
        cell.set_temperature(next);
        world.set(x, y, cell);
    }
    next > AMBIENT_TEMPERATURE
}

/// How much the scorch shell cools across its own width, as a fraction of
/// the peak — so the ring's inner edge reads hotter than its outer one.
const SCORCH_FALLOFF: f32 = 0.75;
/// How much position-keyed variation the scorch ring gets, as a fraction of
/// its local peak. The old force-ignite path drew a geometrically perfect
/// annulus of uniform colour, which read as a stamped decal; this is what
/// breaks the circle up.
const SCORCH_RAGGEDNESS: f32 = 0.45;
/// The crater's equivalent of `SCORCH_RAGGEDNESS`, and it exists because
/// the crater had none: the scorch ring was already broken up while the
/// hole it surrounds was a clean arc-of-a-circle cut, sector by sector.
/// Read as a fraction of a sector's own effective reach and applied **both
/// ways** about it (`jitter - 0.5`), so the edge bites into the rock in
/// some places and leaves teeth standing in others -- a one-way version
/// only ever shrinks the crater, which reads as a smaller clean circle
/// rather than a ragged one.
///
/// Used identically by `Blast::clear_annulus` and by `rigid::fracture_shell`
/// through `ragged_sector_limit`, so the hole and the shell that gets
/// thrown off its rim agree about where the rim is.
const CRATER_RAGGEDNESS: f32 = 0.35;
/// The shortest and the extra span a blast fissure may run, as fractions of
/// `radius * crack_reach`. The jitter driving the spread is **squared**, so
/// the distribution is heavy-tailed the way `fragment_rungs` is: most
/// spokes are stubs around the crater and the occasional one runs right out
/// into the massif. A single shared length is what made the first version
/// read as an asterisk.
const CRACK_LENGTH_FLOOR: f32 = 0.35;
const CRACK_LENGTH_SPREAD: f32 = 1.45;
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

/// The calving collar's own version of `BLAST_SHELL_FORCE`, and half of it:
/// this is rock *letting go* seconds after the bang, not rock being thrown
/// by it. Most of what moves a calved wedge is gravity; the impulse only has
/// to unstick it from the rim and point it into the hole.
const CALVE_FORCE: f32 = 0.03;
/// One rung up the fragment ladder for the open-sector collar, so what comes
/// off the rim reads as slabs rather than chips — the same reason
/// `fracture_shell` biases its own shell. The pocket collar takes no bias: it
/// is two cells thick, and biasing it up would only ask for pieces that do
/// not fit in it.
const CALVE_SIZE_BIAS: u32 = 1;
/// How far into a contained blast's crush pocket the thin collar reaches.
/// Two cells: enough that a piece can be a piece, little enough that a
/// buried charge does not quietly widen its own pocket every time it fires.
const POCKET_COLLAR_THICKNESS: i32 = 2;

/// The hard ceiling on a blast's afterglow, in frames — three seconds at
/// 60fps, against the ~90 frames the shipped `afterglow_retention` actually
/// takes.
///
/// This exists so that **no tuning value can make a `Blast` immortal**.
/// `afterglow_retention` at 0.9999 would otherwise cool a 240-degree excess
/// over some 55,000 frames with the blast alive and scanning its box for
/// every one of them, which is a setting that hangs the game rather than one
/// that tunes it — exactly what `crack_growth`'s `max(1)` clamp exists to
/// prevent one field along. A blast that hits this cap simply stops fading
/// and dies; the halo it leaves is the old permanent one, which is a
/// visible, honest consequence of a silly setting rather than a freeze.
const AFTERGLOW_MAX_FRAMES: u16 = 180;
/// And the retention actually used is clamped strictly below 1, so the
/// arithmetic itself is monotone even before the frame cap is reached.
const AFTERGLOW_MAX_RETENTION: f32 = 0.995;

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

    /// ...and then that smoke has to go away again.
    ///
    /// The other half of the test directly above, and the pair only means
    /// anything together: one says the blast lays smoke down, this says the
    /// crater does not keep it. Until `Material::dissipation` landed there
    /// was **no removal rule for gas anywhere in the simulation**
    /// (`Reports/explosion-stone-review.md` §11, open item 8), so a charge
    /// fired inside rock left a grey cap in its own pocket for the rest of
    /// the session — and the guard above passed the whole time, because
    /// "smoke exists" is exactly what a permanent cap satisfies best.
    ///
    /// Enclosed on purpose: the crater here is a void inside a solid block,
    /// which is the geometry the complaint came from and the only one where
    /// the smoke cannot simply rise out of frame. Budget measured, with
    /// headroom — the charge here is fully confined, so it lays only 5 smoke
    /// cells into the pocket it crushes, and the last of them goes at frame
    /// 430. 2,500 is nearly 6x that, and deliberately loose: what this is
    /// guarding against is *permanence*, and the difference between 430 and
    /// 2,500 frames is not a difference anyone can see.
    #[test]
    fn blast_smoke_does_not_stay_in_the_crater() {
        let mut w = test_world();
        for y in 20..60 {
            for x in 20..60 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 40, 40, 12, 180.0);

        let smoke = |w: &World| {
            (0..128)
                .flat_map(|y| (0..128).map(move |x| (x, y)))
                .filter(|&(x, y)| w.get(x, y).material == material::SMOKE)
                .count()
        };
        let laid = smoke(&w);
        assert!(laid > 0, "the blast left no smoke to clear");

        let mut cleared_at = None;
        for frame in 1..=2_500 {
            crate::sim::update::step(&mut w);
            // The scheduler half matters here specifically: a confined
            // charge's smoke is sealed into its own pocket, so the sweep
            // stops visiting it within a couple of dozen frames.
            w.step_active_sites();
            if smoke(&w) == 0 {
                cleared_at = Some(frame);
                break;
            }
        }
        assert!(
            cleared_at.is_some(),
            "{} of the blast's {laid} smoke cells were still in the crater after 2,500 frames",
            smoke(&w)
        );
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

    /// The crack star **grows**, and the staged driver is the one that
    /// shows it.
    ///
    /// The owner's complaint was not about the star's shape -- it was that
    /// the whole thing arrived on the bang frame and never moved again,
    /// which reads as a decal stamped on the stone. So the property under
    /// test is a *trajectory*, not an end state: cracked cells at frame 2
    /// strictly under cracked cells at frame 30, and the growth finished by
    /// frame 60. An end-state assertion cannot fail for the artifact this
    /// exists to catch, because the end state is exactly what it always was
    /// (`a_fully_buried_stone_blast_crushes_a_pocket_and_scores_a_crack_star`
    /// above is that half, and it still passes through the synchronous
    /// driver).
    ///
    /// **Deliberately not asserting the two drivers agree cell for cell.**
    /// They do not, and are not meant to: the fork pool is shared and
    /// round-robin spends it in a different order than depth-first
    /// (`structural::FissureWalks`' doc). What has to hold of both is that
    /// each draws a real star, so they are banded rather than compared.
    #[test]
    fn a_staged_blast_grows_its_crack_star_over_frames() {
        let cracked = |w: &World, cx: i32, cy: i32, radius: i32| -> usize {
            let box_r = radius * 3;
            ((cy - box_r)..=(cy + box_r))
                .flat_map(|y| ((cx - box_r)..=(cx + box_r)).map(move |x| (x, y)))
                .filter(|&(x, y)| w.get(x, y).cracked())
                .count()
        };
        let buried_stone = || {
            let mut w = test_world();
            for y in 10..118 {
                for x in 10..118 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            w
        };
        let (cx, cy, radius) = (64, 64, 20);

        let mut w = buried_stone();
        let mut particles = ParticleSystem::new();
        let mut blasts = Blasts::new();
        blasts.trigger_with(&mut w, &mut particles, cx, cy, radius, 180.0);
        let mut at_2 = 0;
        let mut at_30 = 0;
        // **The star finishing and the blast dying are two different
        // events now.** They were the same event when the blast's whole
        // life was its stages plus its cracks; a blast outlives its star by
        // however long the afterglow takes to fade (`Blast::afterglow`), so
        // asserting on `blasts.is_empty()` here would be asserting the
        // cooling schedule under a name about crack growth. What this test
        // is *for* is that the star stops growing inside about half a
        // second, so it watches the census stop moving.
        let mut star_settled_at = None;
        let mut previous = 0;
        for frame in 1..=60 {
            blasts.step(&mut w, &mut particles);
            let now = cracked(&w, cx, cy, radius);
            match frame {
                2 => at_2 = now,
                30 => at_30 = now,
                _ => {}
            }
            if now == previous {
                if star_settled_at.is_none() {
                    star_settled_at = Some(frame);
                }
            } else {
                star_settled_at = None; // it moved again -- not settled after all
            }
            previous = now;
        }
        let staged = cracked(&w, cx, cy, radius);

        // The growth itself. This is the assertion the old one-call star
        // could not have passed -- it was already finished at frame 0.
        assert!(at_2 < at_30, "the crack star did not grow: {at_2} cracked cells at frame 2, {at_30} at frame 30");
        assert!(
            star_settled_at.is_some(),
            "the crack star was still growing at frame 60 -- it has to finish inside about half a second"
        );
        assert!(staged >= 100, "the staged driver's finished star is only {staged} cracked cells");

        // And the synchronous driver, which the tests and every headless
        // caller use, still draws a comparable star in one call.
        let mut w2 = buried_stone();
        let mut particles2 = ParticleSystem::new();
        trigger(&mut w2, &mut particles2, cx, cy, radius, 180.0);
        let synchronous = cracked(&w2, cx, cy, radius);
        assert!(synchronous >= 100, "the synchronous driver's star is only {synchronous} cracked cells");
    }

    /// **Two charges, two reports, each with its own site.**
    ///
    /// The guard for the queue `Blasts::finished` exists to be. Before it,
    /// `last_blast_report` was one slot: fire nine charges in a run and
    /// eight of the nine reports are overwritten before anything reads
    /// them, and the harness prints one line that looks exactly like a
    /// complete answer. That is `CLAUDE.md`'s "did it fire at all" failure
    /// one level up — the counter itself going missing rather than the
    /// mechanism — so it is asserted as *how many came back and where from*,
    /// not merely that draining returns something.
    ///
    /// Deliberately fires the second charge well away from the first, so a
    /// swapped or duplicated site is visible in the assertion rather than
    /// hidden by two blasts at nearly the same place.
    #[test]
    fn every_blast_leaves_its_own_report_behind_not_just_the_last_one() {
        let mut w = buried_stone();
        let mut particles = ParticleSystem::new();
        let mut blasts = Blasts::new();
        blasts.trigger_with(&mut w, &mut particles, 40, 64, 12, 180.0);
        blasts.trigger_with(&mut w, &mut particles, 96, 64, 12, 180.0);
        let mut frames = 0;
        while !blasts.is_empty() && frames < 600 {
            blasts.step(&mut w, &mut particles);
            frames += 1;
        }
        let reports = blasts.drain_finished_reports();
        // Sorted, and that is the assertion this test wants rather than a
        // weakening of it. The queue hands reports back in **completion**
        // order, which is not trigger order: each charge's crack star runs
        // until its walkers spend their budget or leave the rock, and how
        // many frames that takes is a property of the rock under that
        // particular charge. So anything that changes how far a walker
        // travels reorders these two without touching the claim -- W3's
        // diagonal decomposition did exactly that, flipping (40, 64) and
        // (96, 64) while both still reported their own site and their own
        // cleared count. The claim is *how many came back and where from*,
        // per this test's own doc; the arrival order never was one.
        let mut sites: Vec<(i32, i32)> = reports.iter().map(|&(x, y, _)| (x, y)).collect();
        sites.sort_unstable();
        assert_eq!(sites, vec![(40, 64), (96, 64)], "both charges must report, each from its own site");
        for (x, y, report) in &reports {
            assert!(report.cells_cleared > 0, "the blast at ({x}, {y}) reported clearing nothing");
        }
        // Drained means drained: a second call must not hand the same
        // reports out again, or the harness would print every blast once
        // per frame for the rest of the run.
        assert!(blasts.drain_finished_reports().is_empty(), "draining twice returned the same reports again");
    }

    /// The queue is a leak guard, so it has to actually bound itself.
    ///
    /// A caller that never drains — anything that is not the harness — must
    /// not grow this without limit for the life of a session. Oldest goes.
    #[test]
    fn the_finished_report_queue_stops_growing_at_its_cap() {
        let mut w = buried_stone();
        let mut particles = ParticleSystem::new();
        let mut blasts = Blasts::new();
        for i in 0..(FINISHED_REPORT_CAP + 5) {
            // Radius 1 at strength 1: the smallest blast that still ends,
            // so the loop is about the queue and not about the rock.
            let x = 20 + (i % 80) as i32;
            blasts.trigger_with(&mut w, &mut particles, x, 64, 1, 1.0);
            let mut frames = 0;
            while !blasts.is_empty() && frames < 600 {
                blasts.step(&mut w, &mut particles);
                frames += 1;
            }
        }
        let reports = blasts.drain_finished_reports();
        assert_eq!(reports.len(), FINISHED_REPORT_CAP, "the queue must cap rather than grow");
        // The oldest were dropped, not the newest: the last charge fired is
        // the one still in there.
        let last_x = 20 + ((FINISHED_REPORT_CAP + 4) % 80) as i32;
        assert_eq!(reports.last().map(|&(x, _, _)| x), Some(last_x), "the cap must drop the oldest, not refuse the newest");
    }

    /// A buried stone world with nothing else in it — the geometry every
    /// test below shares, and the one a blast has the least to work with.
    fn buried_stone() -> World {
        let mut w = test_world();
        for y in 10..118 {
            for x in 10..118 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        w
    }

    /// Run a staged blast to its own end, and report `(frames, peak bodies,
    /// report)`. Bounded so a blast that never dies fails the bound instead
    /// of hanging the suite.
    fn run_staged(w: &mut World, tuning: Tuning, cx: i32, cy: i32, radius: i32, limit: u32) -> (u32, usize, BlastReport) {
        let mut particles = ParticleSystem::new();
        let mut blasts = Blasts::with_tuning(tuning);
        blasts.trigger_with(w, &mut particles, cx, cy, radius, 180.0);
        let mut peak = w.chunk_bodies.len();
        let mut frames = 0;
        while !blasts.is_empty() && frames < limit {
            blasts.step(w, &mut particles);
            peak = peak.max(w.chunk_bodies.len());
            frames += 1;
        }
        (frames, peak, blasts.last_blast_report())
    }

    /// The hottest cell in a box, which is how "is the glow still there"
    /// gets asked as a number rather than looked at.
    fn hottest(w: &World, cx: i32, cy: i32, box_r: i32) -> i16 {
        ((cy - box_r)..=(cy + box_r))
            .flat_map(|y| ((cx - box_r)..=(cx + box_r)).map(move |x| (x, y)))
            .map(|(x, y)| w.get(x, y).temperature())
            .max()
            .unwrap_or(AMBIENT_TEMPERATURE)
    }

    /// **The blast takes its own glow away with it.**
    ///
    /// Scorched stone never cools on its own — `heat_conductivity` 0 puts it
    /// on `fire::update`'s thermally-inert fast path, which returns before
    /// any decay — so before this the ring a blast wrote was permanent:
    /// measured at 2,665 cells above ambient, unchanged from frame 121 to
    /// frame 361 and forever after, a fireball sticker pasted on the rock.
    ///
    /// Asserted as a *trajectory with an end*, not an end state: hot while
    /// the blast lives, exactly ambient once it dies. An end-state check
    /// alone would pass on a blast that never heated anything.
    #[test]
    fn the_afterglow_takes_the_glow_with_it_when_the_blast_dies() {
        let mut w = buried_stone();
        let (cx, cy, radius) = (64, 64, 20);
        let box_r = radius * 3;

        let mut particles = ParticleSystem::new();
        let mut blasts = Blasts::new();
        blasts.trigger_with(&mut w, &mut particles, cx, cy, radius, 180.0);
        let while_alive = hottest(&w, cx, cy, box_r);

        let mut frames = 0;
        while !blasts.is_empty() && frames < 600 {
            blasts.step(&mut w, &mut particles);
            frames += 1;
        }

        assert!(
            while_alive > AMBIENT_TEMPERATURE + 100,
            "the blast never heated anything: hottest cell {while_alive} C on the bang frame"
        );
        assert!(frames < 600, "the blast never finished -- still alive after {frames} frames");
        assert_eq!(
            hottest(&w, cx, cy, box_r),
            AMBIENT_TEMPERATURE,
            "the blast died leaving a permanent halo behind it -- the whole point of the afterglow is that it does not"
        );
    }

    /// **No tuning value may make a `Blast` immortal.**
    ///
    /// The afterglow keeps the blast alive until what it heated is cool, so
    /// a retention set absurdly close to 1 is a setting that would otherwise
    /// hang the game rather than one that tunes it — the exact shape of trap
    /// `crack_growth`'s `max(1)` clamp exists for one field along.
    /// `AFTERGLOW_MAX_FRAMES` is the bound, and this is the test that says
    /// the bound is real.
    #[test]
    fn no_tuning_value_can_keep_a_blast_alive_forever() {
        let mut w = buried_stone();
        // Would take some 55,000 frames to fade on the arithmetic alone.
        let glacial = Tuning { afterglow_retention: 0.999_99, ..Tuning::default() };
        let limit = 4 * AFTERGLOW_MAX_FRAMES as u32;
        let (frames, ..) = run_staged(&mut w, glacial, 64, 64, 20, limit);
        assert!(
            frames < limit,
            "a blast with afterglow_retention 0.99999 was still alive after {frames} frames -- the cap is not bounding it"
        );
    }

    /// The afterglow's three promises, tested on the rule itself rather than
    /// through a blast: it never raises a temperature, never touches a cell
    /// that is on fire, and always terminates at exactly ambient.
    ///
    /// Driven directly at `cool_toward_ambient` because that is where all
    /// three live — a blast-level test would exercise them through 4,000
    /// cells of noise and could pass with any one of them broken.
    #[test]
    fn the_afterglow_never_warms_anything_and_never_touches_a_fire() {
        let mut w = test_world();
        let cold = AMBIENT_TEMPERATURE - 50;
        w.set(20, 20, Cell::new(material::STONE, 0).with_temperature(cold));
        w.set(21, 20, Cell::new(material::STONE, 0)); // exactly ambient
        let mut burning = Cell::new(material::OIL, 0).with_temperature(500);
        burning.ignite(200);
        w.set(22, 20, burning);
        w.set(23, 20, Cell::new(material::STONE, 0).with_temperature(500));

        for _ in 0..500 {
            for x in 20..=23 {
                cool_toward_ambient(&mut w, x, 20, 0.94);
            }
        }

        assert_eq!(w.get(20, 20).temperature(), cold, "the afterglow warmed a cell that was colder than ambient");
        assert_eq!(w.get(21, 20).temperature(), AMBIENT_TEMPERATURE, "the afterglow moved a cell that was already at ambient");
        assert_eq!(w.get(22, 20).temperature(), 500, "the afterglow cooled a burning cell -- fire.rs owns those");
        assert!(w.get(22, 20).is_burning(), "the afterglow put a fire out");
        assert_eq!(w.get(23, 20).temperature(), AMBIENT_TEMPERATURE, "a hot cell never reached ambient");

        // And monotonically, one step at a time -- a fade that overshot into
        // *cold* would still land on ambient eventually above.
        let mut w2 = test_world();
        w2.set(20, 20, Cell::new(material::STONE, 0).with_temperature(500));
        let mut last = 500;
        for _ in 0..500 {
            cool_toward_ambient(&mut w2, 20, 20, 0.94);
            let now = w2.get(20, 20).temperature();
            assert!(now <= last, "the afterglow raised a temperature: {last} -> {now}");
            assert!(now >= AMBIENT_TEMPERATURE, "the afterglow cooled a cell past ambient: {now}");
            last = now;
        }
    }

    /// **The cracks release pieces.** K2, and the counter half of the
    /// owner's "I don't see the pieces moving at all after the crack".
    ///
    /// A fully buried blast used to end with peak bodies **0**: the star was
    /// drawn, the load model was told, and nothing ever came away, so the
    /// whole mechanic read as a graphic. The collar calved into the crush
    /// pocket is the smallest honest answer -- a buried charge has nowhere
    /// else to put anything.
    ///
    /// A **paired** comparison against `calve_depth: 0`, per `CLAUDE.md`:
    /// outcomes here have enormous spread, and the pair cancels everything
    /// the calving is not about. The control is also the proof the bodies
    /// come from *this* mechanism and not from the blast's own shell
    /// fracture.
    #[test]
    fn a_buried_blast_calves_pieces_once_its_star_has_finished() {
        let (cx, cy, radius) = (64, 64, 20);
        let mut with = buried_stone();
        let (_, calving_bodies, report) = run_staged(&mut with, Tuning::default(), cx, cy, radius, 600);

        let mut without = buried_stone();
        let off = Tuning { calve_depth: 0, ..Tuning::default() };
        let (_, control_bodies, control_report) = run_staged(&mut without, off, cx, cy, radius, 600);

        assert_eq!(control_report.calved, 0, "calve_depth 0 still calved {} cells", control_report.calved);
        assert!(report.calved > 0, "the collar released nothing at all -- the star finished and the rim did not let go");
        assert!(
            calving_bodies > control_bodies,
            "calving produced no more moving pieces than the control: {calving_bodies} bodies against {control_bodies}"
        );
    }

    /// The crack tips are incandescent while they race, and that heat is the
    /// walker's own — not a spill from the fireball.
    ///
    /// Paired against `crack_glow_temperature: 0`, and with the fireball
    /// turned off in both, so the only thing that can put heat on a cracked
    /// cell is the walker.
    ///
    /// **`crack_rays` is set explicitly**, because the default went to `0`
    /// when the joint fabric replaced the radial star as the shipped
    /// pattern. Without it this test measured a mechanism that no longer
    /// runs by default and failed — which is the honest failure and the
    /// reason it is spelled out here rather than deleted: the walker is
    /// still reachable through the hybrid knob, so its glow is still a
    /// contract. The fabric's own glow has its own test below
    /// (`the_joint_front_glows_as_it_spreads`).
    #[test]
    fn the_crack_tips_glow_as_they_run() {
        let hot_cracked_cells = |glow: f32| {
            let mut w = buried_stone();
            let mut particles = ParticleSystem::new();
            let tuning = Tuning { flash_temperature: 0.0, crack_glow_temperature: glow, crack_rays: 12, ..Tuning::default() };
            let mut blasts = Blasts::with_tuning(tuning);
            blasts.trigger_with(&mut w, &mut particles, 64, 64, 20, 180.0);
            // Mid-growth: the star is still extending, so the tips are the
            // freshest thing in the world.
            for _ in 0..12 {
                blasts.step(&mut w, &mut particles);
            }
            (10..118)
                .flat_map(|y| (10..118).map(move |x| (x, y)))
                .filter(|&(x, y)| {
                    let c = w.get(x, y);
                    c.cracked() && c.temperature() > AMBIENT_TEMPERATURE
                })
                .count()
        };

        assert_eq!(hot_cracked_cells(0.0), 0, "crack_glow_temperature 0 still heated the rock it cracked");
        assert!(hot_cracked_cells(300.0) > 0, "the crack tips never lit up");
    }


    // ---- The joint fabric (F) --------------------------------------------

    /// **Did it fire at all.** Not a look at a sheet: a walked crack star
    /// and a Worley boundary web draw the same grey scratches at the zoom a
    /// contact sheet is read at, and `CLAUDE.md` records a whole feature
    /// that had never once executed while its sheet was being read as proof
    /// it had.
    ///
    /// Also the arithmetic the three counters promise: once the front has
    /// finished, `opened + scored == activated`.
    #[test]
    fn a_blast_wakes_the_joint_fabric_and_the_counters_add_up() {
        let mut w = buried_stone();
        let (_frames, _peak, report) = run_staged(&mut w, Tuning::default(), 64, 64, 20, 600);
        assert!(report.joints_activated > 100, "only {} joints activated in solid stone", report.joints_activated);
        assert!(report.joints_opened > 0, "no joint opened -- the seams are the half that reads as void");
        assert!(report.joints_scored > 0, "no joint was scored -- the halo never spread past the opened zone");
        assert_eq!(
            report.joints_opened + report.joints_scored,
            report.joints_activated,
            "the fabric's three counters disagree once the front has finished"
        );
    }

    /// What the counters say when nothing is wrong — the sanity check
    /// `CLAUDE.md` asks for *before* a new metric is trusted about the case
    /// it was written for. Three situations with no jointed rock in reach
    /// must read exactly zero, not something small.
    #[test]
    fn nothing_jointed_in_reach_wakes_no_joints() {
        // An airburst: nothing but air within the whole halo.
        let mut air = test_world();
        let mut particles = ParticleSystem::new();
        let mut blasts = Blasts::new();
        blasts.trigger_with(&mut air, &mut particles, 64, 40, 20, 180.0);
        assert_eq!(blasts.last_blast_report().joints_activated, 0, "an airburst woke joints");

        // A charge inside loose material. Sand has no `joint_spacing`, and
        // that is the *content* gate this is here to prove: no id test in
        // the code says "stone", so a material that never asked for joints
        // must never get them.
        //
        // The sand goes over `test_world`'s stone floor rather than
        // stopping above it, and that is not tidiness: an unconfined charge
        // now stretches its halo by up to 2x to compensate for the rock
        // that is not there (`JointExposure`), and a sand pile *is* an
        // unconfined charge, so a 48-cell halo becomes a 96-cell one and
        // the floor 63 cells below is inside it. The scene then contains
        // jointed rock in reach and the test's own name stops describing
        // it. `CLAUDE.md`: check the scene still contains the situation you
        // think it does.
        let mut sand = test_world();
        for y in 10..128 {
            for x in 10..118 {
                sand.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        let mut blasts = Blasts::new();
        blasts.trigger_with(&mut sand, &mut particles, 64, 64, 20, 180.0);
        assert_eq!(blasts.last_blast_report().joints_activated, 0, "a charge in sand woke joints");

        // And the knob's own off position, in rock that would otherwise
        // wake hundreds: a `joint_spacing` of 0 is "not jointed".
        let mut stone = buried_stone();
        let id = stone.materials.id_of("stone").expect("stone exists");
        stone.materials.get_mut(id).joint_spacing = 0.0;
        let (_f, _p, report) = run_staged(&mut stone, Tuning::default(), 64, 64, 20, 600);
        assert_eq!(report.joints_activated, 0, "joint_spacing 0 still woke joints");
    }

    /// **Breakage arrives at the bang.** The owner's second complaint on the
    /// same card was that it turns up 7-15 seconds late, which it did:
    /// nothing moved until the structural relaxation wavefront got there at
    /// one cell per five frames.
    ///
    /// A paired comparison rather than a bar (`CLAUDE.md`: compare two runs,
    /// not one run against a remembered number) — the same charge with the
    /// opening stood down is the control, and the only difference between
    /// the two runs is the mechanism under test.
    #[test]
    fn the_near_joints_open_on_the_bang_frame() {
        let rock_after_one_frame = |open_fraction: f32| -> usize {
            let mut w = buried_stone();
            let mut particles = ParticleSystem::new();
            let tuning = Tuning { joint_open_fraction: open_fraction, ..Tuning::default() };
            let mut blasts = Blasts::with_tuning(tuning);
            // `trigger_with` runs `Blast::new` *and* the first `advance`, so
            // this is the state of the world on the frame of the flash --
            // no relaxation, no failure pass, no falling.
            blasts.trigger_with(&mut w, &mut particles, 64, 64, 20, 180.0);
            (10..118)
                .flat_map(|y| (10..118).map(move |x| (x, y)))
                .filter(|&(x, y)| w.get(x, y).material == material::STONE)
                .count()
        };
        let scored_only = rock_after_one_frame(0.0);
        let opened = rock_after_one_frame(Tuning::default().joint_open_fraction);
        assert!(
            scored_only > opened + 100,
            "opening the near joints took only {} extra cells on the bang frame ({scored_only} standing against {opened}) -- the seams are not arriving with the flash",
            scored_only - opened
        );
    }

    /// The fabric's fracture front is incandescent while it spreads, the
    /// same way the walker's tip is — `the_crack_tips_glow_as_they_run`'s
    /// sibling, for the mechanism that now ships by default.
    ///
    /// Sampled *mid-spread* on purpose: the afterglow takes this heat away
    /// again, so an end-state check would read zero on a working front.
    #[test]
    fn the_joint_front_glows_as_it_spreads() {
        let hot_cracked_cells = |glow: f32| {
            let mut w = buried_stone();
            let mut particles = ParticleSystem::new();
            let tuning = Tuning { flash_temperature: 0.0, crack_glow_temperature: glow, ..Tuning::default() };
            let mut blasts = Blasts::with_tuning(tuning);
            blasts.trigger_with(&mut w, &mut particles, 64, 64, 20, 180.0);
            for _ in 0..30 {
                blasts.step(&mut w, &mut particles);
            }
            (10..118)
                .flat_map(|y| (10..118).map(move |x| (x, y)))
                .filter(|&(x, y)| {
                    let c = w.get(x, y);
                    c.cracked() && c.temperature() > AMBIENT_TEMPERATURE
                })
                .count()
        };
        assert_eq!(hot_cracked_cells(0.0), 0, "crack_glow_temperature 0 still heated the rock the fabric parted");
        assert!(hot_cracked_cells(300.0) > 0, "the joint front never lit up");
    }

    /// The growth beat, and it is not optional: a pattern that arrives whole
    /// is *"a graphic stamped on the stone"*, which is a complaint this
    /// engine has already answered once and must not walk back into.
    ///
    /// Asserted as a trajectory — scored joints strictly increasing across
    /// the spread, and finished inside a budget — rather than as two
    /// instants, so it survives a retune of the reach.
    #[test]
    fn the_scored_halo_spreads_over_frames_rather_than_arriving_whole() {
        let mut w = buried_stone();
        let mut particles = ParticleSystem::new();
        let mut blasts = Blasts::new();
        blasts.trigger_with(&mut w, &mut particles, 64, 64, 20, 180.0);
        let at_trigger = blasts.last_blast_report().joints_scored;
        for _ in 0..6 {
            blasts.step(&mut w, &mut particles);
        }
        let early = blasts.last_blast_report().joints_scored;
        let mut frames = 6;
        while !blasts.is_empty() && frames < 600 {
            blasts.step(&mut w, &mut particles);
            frames += 1;
        }
        let done = blasts.last_blast_report().joints_scored;
        assert_eq!(at_trigger, 0, "the scored halo was already {at_trigger} joints deep on the bang frame -- it is a stamp");
        assert!(early < done, "the halo did not spread: {early} scored at frame 6, {done} at the end");
        assert!(frames < 200, "the halo was still spreading at frame {frames} -- it has to finish inside a few seconds");
    }

    /// Same rock, same grain. A repeat charge on the same spot has to
    /// retrace and deepen its own joints rather than draw a new pattern, and
    /// that is only true if nothing here draws from `world.rng` — which
    /// would put the blast into the replay draw order.
    #[test]
    fn the_same_charge_wakes_the_same_joints_every_time() {
        let run = || {
            let mut w = buried_stone();
            let (_f, _p, report) = run_staged(&mut w, Tuning::default(), 64, 64, 20, 600);
            let cracked: Vec<(i32, i32)> = (10..118)
                .flat_map(|y| (10..118).map(move |x| (x, y)))
                .filter(|&(x, y)| w.get(x, y).cracked())
                .collect();
            (report.joints_activated, report.joints_opened, cracked)
        };
        let a = run();
        let b = run();
        assert_eq!(a.0, b.0, "two identical charges activated different numbers of joints");
        assert_eq!(a.1, b.1, "two identical charges opened different numbers of joints");
        assert_eq!(a.2, b.2, "two identical charges cut different rock");
    }

    /// **Both drivers**, because the app runs the parallel one and behaviour
    /// only the player sees is behaviour only `parallel::step` produces.
    ///
    /// The fabric itself is written by the blast rather than by the sweep,
    /// so what is actually at risk here is everything *downstream*: the
    /// structural checks the seams schedule, the pieces that promote, the
    /// grit that falls. Both drivers must move rock, and neither may run
    /// away with it.
    #[test]
    fn both_drivers_break_rock_along_the_woken_joints() {
        let settle = |parallel: bool| -> (u32, u32) {
            let mut w = buried_stone();
            let mut particles = ParticleSystem::new();
            let mut blasts = Blasts::new();
            blasts.trigger_with(&mut w, &mut particles, 64, 64, 20, 180.0);
            for _ in 0..400 {
                if !blasts.is_empty() {
                    blasts.step(&mut w, &mut particles);
                }
                if parallel {
                    crate::sim::parallel::step(&mut w);
                } else {
                    crate::sim::update::step(&mut w);
                }
            }
            (w.structural_failures.promoted_cells, w.structural_failures.shattered_cells)
        };
        let (serial_promoted, serial_grit) = settle(false);
        let (par_promoted, par_grit) = settle(true);
        assert!(serial_promoted > 0, "the serial driver promoted nothing -- no piece ever moved");
        assert!(par_promoted > 0, "the parallel driver promoted nothing -- no piece ever moved");
        // Grit is the other half of "a few blocks, more cobbles, a lot of
        // grit": the seams convert part of what they open rather than
        // deleting all of it, so a run with no shattered cells at all means
        // the rubble half of `open_seam` stopped happening.
        assert!(serial_grit > 0 && par_grit > 0, "no grit at all: serial {serial_grit}, parallel {par_grit}");
        // Not equality: the two drivers legitimately differ in *order*
        // (`FissureWalks`' own doc says so for the walker, and the same
        // applies to which piece falls first). What must hold is that
        // neither takes the mountain apart while the other does not.
        let (lo, hi) = (serial_promoted.min(par_promoted), serial_promoted.max(par_promoted));
        assert!(hi < lo * 4 + 200, "the drivers disagree wildly: serial {serial_promoted} cells promoted, parallel {par_promoted}");
    }

    /// The hybrid knob still works in both positions. `crack_rays` defaults
    /// to `0` — pure fabric — but the walker is deliberately left reachable
    /// so the A/B can be re-run from the tuning panel without a rebuild, and
    /// a knob nobody can move is not a knob.
    #[test]
    fn the_hybrid_knob_puts_the_radial_walker_back() {
        let fissured = |rays: u32| {
            let mut w = buried_stone();
            let (_f, _p, report) = run_staged(&mut w, Tuning { crack_rays: rays, ..Tuning::default() }, 64, 64, 20, 600);
            (report.cells_fissured, report.joints_activated)
        };
        let (none, fabric_off_rays) = fissured(0);
        let (some, fabric_on_rays) = fissured(6);
        assert_eq!(none, 0, "crack_rays 0 still walked {none} cells of radial fissure");
        assert!(some > 0, "crack_rays 6 walked nothing");
        assert!(fabric_off_rays > 0 && fabric_on_rays > 0, "the fabric must run in both positions of the hybrid knob");
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
        // **The joint fabric's opening is turned off, and nothing else is.**
        // It legitimately removes stone in exactly this annulus -- that is
        // the whole of `JointSeams`' inner zone -- so leaving it on would
        // make this test fail for a mechanism it is not about, which is
        // `CLAUDE.md`'s "a scene that contradicts the code will look like a
        // bug in the code" from the other side. The shockwave itself, and
        // the `Powder`-only rule this asserts, are untouched: scoring still
        // happens, the fabric still runs, only the removal is stood down.
        let tuning = Tuning { joint_open_fraction: 0.0, ..Tuning::default() };
        trigger_tuned(&mut w, &mut particles, 40, 40, radius, 200.0, &tuning);

        let shockwave_radius = (radius as f32 * tuning.shockwave_multiplier).round() as i32;
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
        // Fabric opening off, and only that -- see
        // `the_shockwave_does_not_uproot_solid_material_beyond_the_crater`
        // for the same scoping and the same reason. `debris_velocity` reads
        // the *pressure gradient*, which is a function of what is standing
        // where; seams opened before the first stage put voids inside a
        // 20x20 test block and tilt that gradient. Measured with them on,
        // the worst particle in this scene grazed to cos -0.209 against a
        // -0.2 tolerance that already exists for grazes -- a scene change,
        // not the sign error this test is here to catch, and widening the
        // tolerance to absorb it would have blunted the only thing it
        // measures.
        let tuning = Tuning { joint_open_fraction: 0.0, ..Tuning::default() };
        trigger_tuned(&mut w, &mut particles, 40, 40, 8, 200.0, &tuning);

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
        // `afterglow_retention: 1.0` pins the *pre-afterglow* contract, the
        // same way `containment_floor: INFINITY` above pins the pre-R2 one.
        // This test asks whether the scorch write happened at all, and it
        // reads the world after `trigger_tuned` has run the blast to
        // completion -- which now includes the blast cooling everything it
        // heated back to ambient before it dies. Left un-pinned it would
        // still pass, but only via the oil the fireball set *burning*
        // (afterglow never touches a burning cell), which is a different
        // claim than the one in the name.
        let no_smoke = Tuning { smoke_fraction: 0.0, containment_floor: f32::INFINITY, afterglow_retention: 1.0, ..Tuning::default() };
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

    /// The uniform case is the one that must come through untouched: a
    /// fully buried charge reads the same reach in all sixteen directions,
    /// and a blur that shaved it would quietly shrink every crush pocket in
    /// the game for no reason at all.
    #[test]
    fn smoothing_leaves_an_evenly_confined_probe_exactly_as_it_found_it() {
        let flat = [7u8; CONFINEMENT_SECTORS];
        assert_eq!(smooth_sectors(flat), flat);
    }

    /// The regression that made this a `max` rather than a plain blur, kept
    /// as its own reproduction.
    ///
    /// A wall shot vents in three of sixteen directions. A plain
    /// `(a + 2b + c) / 4` shaves the peak of a run that narrow -- measured
    /// 19 against a radius of 20 -- so **no** sector passed `reach >=
    /// radius` any more and the blast reported itself 16/16 contained.
    /// `Blast::new` counts its open sectors off this array and
    /// `rigid::fracture_shell` gates on the same comparison, so the cave
    /// wall lost its 703-cell overburden failure and both thrown bodies
    /// while leaving a *larger* hole. The ramp must only ever be added on
    /// the contained side.
    #[test]
    fn smoothing_never_lowers_a_sector_below_what_the_probe_read() {
        let radius = 20u8;
        let contained = 7u8;
        let mut probed = [contained; CONFINEMENT_SECTORS];
        for slot in probed.iter_mut().skip(5).take(3) {
            *slot = radius;
        }
        let smoothed = smooth_sectors(probed);
        for i in 0..CONFINEMENT_SECTORS {
            assert!(smoothed[i] >= probed[i], "sector {i} was lowered: {} -> {}", probed[i], smoothed[i]);
        }
        let open = smoothed.iter().filter(|&&r| r >= radius).count();
        assert_eq!(open, 3, "three vented directions must still read as open, not be blurred away");
    }

    /// The point of the blur, stated as a property rather than as three
    /// magic numbers: between a contained direction and an open one there
    /// has to be something in between, or the crater's edge is a cliff on a
    /// 22.5-degree line and reads as a pie cut.
    #[test]
    fn smoothing_ramps_the_contained_side_instead_of_stepping() {
        let (radius, contained) = (20u8, 7u8);
        let mut probed = [contained; CONFINEMENT_SECTORS];
        for slot in probed.iter_mut().skip(5).take(3) {
            *slot = radius;
        }
        let smoothed = smooth_sectors(probed);
        // Sectors 4 and 8 flank the open run; 3 and 9 are one further out.
        assert!(smoothed[4] > contained && smoothed[4] < radius, "sector 4 should be part-way up the ramp, got {}", smoothed[4]);
        assert!(smoothed[3] > contained, "the ramp should still be climbing two sectors out, got {}", smoothed[3]);
        assert!(smoothed[3] < smoothed[4], "the ramp should fall off with distance from the open run");
        assert!(smoothed[8] > contained && smoothed[8] < radius, "the ramp must be symmetric, got {}", smoothed[8]);
    }

    /// Bounds on the crater's per-cell raggedness, and the fact that it is
    /// a *property of the position*: an identical charge fired twice in the
    /// same place must cut the same hole, because determinism is required
    /// and because `rng::jitter` is the whole reason no `world.rng` draw
    /// happens here.
    #[test]
    fn the_ragged_crater_edge_bites_both_ways_and_is_position_stable() {
        let reach = [20u8; CONFINEMENT_SECTORS];
        let (mut over, mut under) = (false, false);
        for y in 0..40 {
            for x in 0..40 {
                let limit = ragged_sector_limit(&reach, x - 20, y - 20, x, y);
                assert_eq!(limit, ragged_sector_limit(&reach, x - 20, y - 20, x, y), "the same cell must give the same limit twice");
                let band = (20.0 * (1.0 - CRATER_RAGGEDNESS / 2.0) - 0.001)..=(20.0 * (1.0 + CRATER_RAGGEDNESS / 2.0) + 0.001);
                assert!(band.contains(&limit), "limit {limit} left the +/- CRATER_RAGGEDNESS/2 band around the sector reach");
                over |= limit > 20.0;
                under |= limit < 20.0;
            }
        }
        assert!(over && under, "the edge must both bite past the sector reach and leave teeth short of it");
    }
}
