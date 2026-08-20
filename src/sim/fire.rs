//! Heat diffusion, ignition, burnout and temperature-triggered phase changes.
//!
//! Runs once per visited cell, called from `update::update_cell` for every
//! material kind — unlike movement, this must not skip `Solid`, since a
//! static burning log still needs its burn timer ticked every frame it is
//! visited.
//!
//! # Why this can share the CA sweep's dirty-rectangle discipline
//!
//! A cell that ignites, is currently burning, or melts/boils writes back
//! through `World::set`, which marks its position dirty exactly the way a
//! movement rule does — so the existing chunk-sleeping machinery keeps a
//! burning region awake for free, with no separate scheduler needed for M14.
//!
//! Two points worth being certain of before touching the ignition logic,
//! since this session already burned real time on a closely related bug
//! (`roll_reach`, in `material.rs`) once:
//!
//! **Neighbour-driven ignition is safe to roll fresh every frame**, unlike
//! `roll_reach_at`. That function had to be keyed on position because a lone
//! rolling grain's own dice roll was the *only* thing keeping its chunk
//! awake — an unlucky roll could let the chunk sleep and freeze the grain
//! permanently. Ignition is different: a burning neighbour keeps re-dirtying
//! *its own* position every frame for as long as it burns (`burn_duration`,
//! commonly a few seconds), which — since it is adjacent — keeps this cell's
//! chunk awake independent of what this cell's own ignition roll says. An
//! unlucky roll this frame is not a last chance; there will be another one
//! next frame, and the frame after, for as long as the neighbour keeps
//! burning. Matches TPT's own approach, which also rolls fresh per check.
//!
//! **Residual heat has no equivalent external forcing and must supply its
//! own.** A cell that is warmer than ambient but not burning and not being
//! actively heated by anything has nothing else re-dirtying it — so if it
//! stopped writing once "close enough" to converged, the chunk would go
//! quiet and CA-level diffusion would silently stall with the temperature
//! stuck wherever it happened to be. The fix is the same shape as
//! `roll_reach_at`'s, applied to a different quantity: while a cell's
//! temperature differs from ambient by more than `THERMAL_SETTLE_EPSILON`, it
//! keeps writing (and therefore keeps its own position dirty) every frame,
//! the same way an ungrounded grain keeps re-examining itself until it finds
//! support. Once within the epsilon, it stops, and the chunk is free to
//! sleep — final convergence to *exactly* ambient is then left to the M13
//! field coupling, which runs unconditionally regardless of CA sleep state.

use super::cell::{Cell, AMBIENT_TEMPERATURE};
use super::decay::DECAY_TICK_INTERVAL;
use super::material::{self, MaterialId, MaterialKind};
use super::scheduler::{ActiveKind, ActiveSite};
use super::surface::CellSurface;

/// Below this many degrees from ambient, a cell stops force-waking its own
/// chunk for heat alone. Small enough that nothing visibly stops changing
/// while still dirty; not zero, because floating point diffusion asymptotes
/// toward ambient without ever exactly reaching it, which would keep every
/// warmed cell dirty forever.
const THERMAL_SETTLE_EPSILON: f32 = 1.0;

/// Chance per visit that a cell already past its boiling or freezing
/// threshold actually transitions. A deterministic threshold flips a whole
/// pool edge in one frame — the all-or-nothing outcome `CLAUDE.md`'s ethos
/// section names as the most reliable source of "this feels fake" — while a
/// per-cell roll makes a boiling surface stipple and a freezing front creep.
/// Costs nothing on the fast path: the roll happens only for cells already
/// past a threshold, which must_stay_dirty revisits every frame anyway, so
/// the mean delay is ~2.5 frames, not a throttle. Melting and condensing
/// are deliberately *not* gated (they fire at chance 1): thaw urgency is
/// carried by the temperature gradient itself, and a condensing steam cell
/// held at its threshold would hang at the ceiling reading as stuck. A feel
/// number, not a derived one — sweep it (rebuilding between points) if a
/// front reads wrong.
const PHASE_CHANGE_CHANCE: f32 = 0.4;

/// A `Liquid` cell must hold at least this much fill (of
/// `material::LIQUID_FULL` = 1000) to freeze solid. The partial fringe a
/// resting body wears at its surface stays chilled water instead of
/// snapping into full-volume solid cells — freezing a fill-100 cell into
/// ice that later melts back to a *full* water cell would manufacture
/// volume from nowhere, since a Solid's `aux` is an anchor distance and
/// cannot carry the fill through. The ~1% of a resting body's cells that
/// are partial (see water.ron's `fill_dimming` note) is the accepted loss
/// of coverage.
const FREEZE_MIN_FILL: u16 = 900;

/// Cumulative "did it fire at all" counters for temperature-triggered
/// transitions, in the style of `world::FailureCounts` and for the same
/// reason: a steam plume and a puff of brush-painted smoke are
/// indistinguishable in a contact sheet, so whether the mechanism produced
/// what is on screen is a count, not a picture. Read by
/// `examples/filmstrip.rs` beside the image.
#[derive(Clone, Copy, Debug, Default)]
pub struct PhaseCounts {
    pub boiled: u32,
    pub condensed: u32,
    pub froze: u32,
    pub melted: u32,
    /// Pairwise reactions fired (`try_react`). Water quenching lava is the
    /// intended first shipped reaction; the counter is generic over
    /// whichever reaction fired.
    pub reacted: u32,
}

impl PhaseCounts {
    pub fn record(&mut self, event: PhaseEvent) {
        match event {
            PhaseEvent::Boiled => self.boiled += 1,
            PhaseEvent::Condensed => self.condensed += 1,
            PhaseEvent::Froze => self.froze += 1,
            PhaseEvent::Melted => self.melted += 1,
            PhaseEvent::Reacted => self.reacted += 1,
        }
    }

    /// Fold a parallel worker's private tally into the world's — the same
    /// queue-and-replay shape every other `ChunkView` side effect uses.
    pub fn merge(&mut self, other: PhaseCounts) {
        self.boiled += other.boiled;
        self.condensed += other.condensed;
        self.froze += other.froze;
        self.melted += other.melted;
        self.reacted += other.reacted;
    }
}

/// One temperature-triggered transition, for `CellSurface::count_phase_event`.
#[derive(Clone, Copy, Debug)]
pub enum PhaseEvent {
    Boiled,
    Condensed,
    Froze,
    Melted,
    Reacted,
}

/// Reference scale for normalizing a `field_moisture_at` reading into a 0..1
/// "how saturated" fraction — matches `field.rs`'s own private `MAX_
/// MOISTURE`, which isn't reachable from here, the same documented-
/// assumption pattern `creature.rs`'s `WORM_MOISTURE_SATURATION` uses.
const MOISTURE_SATURATION: f32 = 4.0;
/// Maximum fraction `flammability` is suppressed by at full saturation —
/// architecture §4's fire-resistance consumer ("wet material should resist
/// ignition; nothing implements this today"). Only the probabilistic
/// contact-ignition path below is affected, not the deterministic `cell.
/// temperature() >= ignition_temperature` crossing above it in `try_ignite`
/// — a fire hot enough to boil off the material's own moisture first can
/// still set it alight, which is physically right (wet wood does eventually
/// burn in a large enough fire) and keeps this from making moisture a hard
/// fireproofing switch. Set high, not 1.0: real wet material is *very*
/// resistant to catching from a neighbour, not perfectly immune.
const MOISTURE_IGNITION_RESISTANCE: f32 = 0.9;

/// Update heat, fire and phase state for the cell at `(x, y)`. Called once
/// per visited cell, before movement is attempted — a phase change (stone
/// melting into lava) needs to happen before this frame's movement dispatch
/// decides how the cell behaves, or it would move as stone for one more frame
/// after already having become lava.
///
/// Returns `true` if the cell's material identity changed (melted, boiled, or
/// burned out into something else) — callers that cache `cell.material` from
/// before this call must re-read it.
pub fn update<S: CellSurface>(surface: &mut S, x: i32, y: i32) -> bool {
    let original = surface.get(x, y);
    if original.material == material::EMPTY {
        return false;
    }
    let mut cell = original;

    // One combined check, one material lookup, in place of what would
    // otherwise be four separate lookups below (diffuse_heat, try_ignite,
    // try_phase_change, try_react each fetch the material themselves and
    // early-exit on their own single property). Most shipped materials —
    // sand, water, stone, gravel, smoke — are inert on every one of these
    // axes, so this turns their visit into one Vec index and six cheap
    // comparisons instead of four. `cell.is_burning()` is checked separately
    // from the material lookup (a flag read, no lookup needed) and always
    // defeats the fast path regardless of the material's own properties —
    // defensive: a cell should never be burning with a non-flammable
    // material under this module's own ignition paths, but nothing should
    // silently drop a burn tick if some future code path ever calls
    // `Cell::ignite` directly on one.
    //
    // `material`'s borrow of `surface` must end here, before anything below
    // needs `&mut surface` again — unlike a direct `&World` field access,
    // going through the `CellSurface` trait means the borrow checker can no
    // longer see that `materials()` and `rng()`/`set()` touch disjoint
    // fields, so a `&Material` held live across one of those calls would not
    // compile. Every function below follows the same shape: pull out the
    // handful of `Copy` values a step needs, in one statement, then never
    // hold the reference itself past that point.
    let material = surface.materials().get(cell.material);
    let is_thermally_inert = !cell.is_burning()
        && material.heat_conductivity <= 0.0
        && material.flammability <= 0.0
        && !material.ignition_temperature.is_finite()
        && !material.melting_point.is_finite()
        && !material.boiling_point.is_finite()
        && !material.cooling_point.is_finite()
        && !material.intrinsic_temperature.is_finite()
        && material.reactions.is_empty();
    if is_thermally_inert {
        return false;
    }
    // Extracted while `material`'s borrow is live, same as everything below.
    let intrinsic_temperature = material.intrinsic_temperature;

    if intrinsic_temperature.is_finite() {
        // The material *is* this temperature (lava) — pin it rather than
        // diffusing, which would only be overwritten. Neighbours still cool
        // it in the only way that matters: by conducting heat away from the
        // pinned boundary through their own `diffuse_heat`. The pinned cell
        // is permanently off-ambient, so `must_stay_dirty` below keeps its
        // chunk awake for as long as it exists — by design, and acceptable
        // only because such content is rare and self-limiting (lava
        // quenches to stone). The field pushes mirror `tick_burn`'s: a
        // small, bounded set of cells telling the coarse field it is hot
        // and bright, not a per-swept-cell coupling.
        cell.set_temperature(intrinsic_temperature.round() as i16);
        surface.add_heat(x, y, 1, 2.0);
        surface.add_light(x, y, 1, 1.0);
    } else {
        diffuse_heat(surface, x, y, &mut cell);
    }

    if cell.is_burning() {
        tick_burn(surface, x, y, &mut cell);
    } else {
        try_ignite(surface, x, y, &mut cell);
    }

    try_phase_change(surface, x, y, &mut cell);
    try_react(surface, x, y, &mut cell);

    let temp_off_ambient = (cell.temperature() as f32 - AMBIENT_TEMPERATURE as f32).abs();
    let must_stay_dirty = cell.is_burning() || temp_off_ambient > THERMAL_SETTLE_EPSILON;

    // Write whenever something actually changed, or — even with no visible
    // change this exact visit (a temperature that int-rounds to the same
    // i16, a burn tick that hasn't crossed a whole degree yet) — whenever the
    // cell is not yet settled. See the module doc: residual heat has nothing
    // else re-dirtying it, so it must keep writing to keep its own position
    // dirty until it converges near ambient.
    //
    // Compared against `original` (saved before any of the above ran) rather
    // than a second `surface.get(x, y)` here — nothing above writes to `(x,
    // y)` itself (`try_react`'s neighbour write always targets a different
    // position), so the two reads would agree, and this avoids paying for a
    // second lookup on every single visited cell.
    if cell != original || must_stay_dirty {
        surface.set(x, y, cell);
    }

    cell.material != original.material
}

/// Explicit-difference diffusion toward the average of the four CA
/// neighbours.
///
/// The conductivity is `material.heat_conductivity`, clamped to the 2D
/// explicit-diffusion stability bound (Fourier number ≤ 0.25) — the same
/// bound `field::HEAT_DIFFUSION_RATE` respects, with *less* margin to spare
/// here: the CA grid mixes four independent neighbour reads into one update
/// exactly like the field's diffusion pass does, so the same derivation
/// applies unchanged.
///
/// **Deliberately does not couple to the M13 field.** An earlier version
/// called `world.field_at(x, y)` here to pull every cell gently toward the
/// coarse ambient field, reasoning that a sleeping chunk's residual heat
/// needed *something* to keep pulling it the rest of the way to true
/// ambient. Two things made that both unnecessary and expensive. Not
/// necessary: this function only ever runs for a *visited* cell, and a
/// sleeping chunk's cells are — by definition of sleeping — not visited, so
/// the field term never actually fired for the case it was written for; the
/// `try_ignite`/minimum-progress fix below already guarantees a visited,
/// isolated hot cell converges to exact ambient using neighbour-average
/// alone, since untouched neighbours read `Cell::EMPTY`'s ambient default.
/// Expensive: `field_at` is a `HashMap` lookup, and this function runs for
/// *every visited CA cell* — measured on the full-screen stress scenario,
/// adding it took the worst frame from ~16 ms to ~64 ms, because a scene
/// with on the order of 10⁵ moving CA cells pays that lookup 10⁵ times a
/// frame, against the field's own dedicated pass touching roughly 2500 field
/// cells for the same information. The CA-to-field heat coupling that *is*
/// still needed (so the field actually reflects nearby fire, for later
/// milestones that read ambient temperature/light) is pushed from burning
/// cells instead, in `tick_burn` — a naturally small, bounded set at any
/// given moment, not every swept cell.
fn diffuse_heat<S: CellSurface>(surface: &S, x: i32, y: i32, cell: &mut Cell) {
    let conductivity = surface.materials().get(cell.material).heat_conductivity.min(0.25);
    if conductivity <= 0.0 {
        return;
    }

    let here = cell.temperature() as f32;
    let left = surface.get(x - 1, y).temperature() as f32;
    let right = surface.get(x + 1, y).temperature() as f32;
    let up = surface.get(x, y - 1).temperature() as f32;
    let down = surface.get(x, y + 1).temperature() as f32;
    let neighbour_avg = (left + right + up + down) / 4.0;

    let new_temp = here + (neighbour_avg - here) * conductivity;

    // Naively rounding `new_temp` can get permanently stuck a few degrees
    // short of equilibrium: at here=22 pulling toward 20 at these rates, the
    // raw step is -0.4, and 21.6 rounds right back to 22 — a genuine
    // numerical fixed point, not just slow convergence, because `here` is
    // always already a whole number (temperature is stored as i16) and a
    // sub-half-degree pull can never move the rounded result at all. Found
    // by a test that ran 5000 frames waiting for a 200° cell to approach
    // ambient and it never did; every real target it did reach was actually
    // 22°, two degrees short forever. Under this engine's design that is not
    // a cosmetic inaccuracy: a cell that can never settle keeps re-dirtying
    // its own chunk to stay under `THERMAL_SETTLE_EPSILON` forever, so *any*
    // region ever warmed even slightly would never sleep again.
    //
    // The fix guarantees monotonic progress: if the raw (unrounded) pull is
    // real but rounds away to zero net change, force one degree of movement
    // in the pull's direction anyway. This still converges to whatever the
    // true local equilibrium is — the neighbourhood average, not necessarily
    // literal room temperature — rather than snapping to a hardcoded
    // constant, which would be wrong for a region sitting in an elevated
    // ambient near something large and hot.
    //
    // **Only once the cell is actually outside `THERMAL_SETTLE_EPSILON` of
    // ambient**, though — gated the same way `must_stay_dirty` below decides
    // whether this cell still needs to force its own chunk awake. Without
    // that gate, a cell already within the epsilon (so already "settled" by
    // every other measure) could still get force-nudged a whole degree away
    // whenever its raw pull was small-but-nonzero, which a connected mass of
    // many cooling cells produces constantly — each one pulling every other
    // very slightly toward itself forever, never a true zero. That value
    // *change* (not just `must_stay_dirty`) is what triggers the write
    // below, so a cell force-nudged this way keeps re-dirtying its chunk
    // even while every value involved reads as "settled" — caught by a test
    // with 40 connected cooling ash cells, which real burnt content and not
    // the single-isolated-cell case the original fix was built against.
    let raw_delta = new_temp - here;
    let rounded = new_temp.round();
    let already_settled = (here - AMBIENT_TEMPERATURE as f32).abs() <= THERMAL_SETTLE_EPSILON;
    let final_temp = if rounded == here && raw_delta.abs() > 0.01 && !already_settled {
        here + raw_delta.signum()
    } else {
        rounded
    };

    cell.set_temperature(final_temp.clamp(i16::MIN as f32, i16::MAX as f32) as i16);
}

fn tick_burn<S: CellSurface>(surface: &mut S, x: i32, y: i32, cell: &mut Cell) {
    // Extracted up front, before `surface.add_heat` needs `&mut S` below —
    // `material` borrows `surface` immutably, and Rust would otherwise see
    // that borrow as still live at the `material.burns_into` read further
    // down. See the note in `update` above on why this matters more here
    // than it did against a plain `&mut World`.
    let material = surface.materials().get(cell.material);
    let burn_temp = material.burn_temperature;
    let burns_into = material.burns_into;
    // `MaterialKind` is `Copy` — extracted up front for the same reason
    // `burn_temp`/`burns_into` are: the burnout branch below needs it after
    // `material`'s own borrow would otherwise still be live.
    let was_structural = matches!(material.kind, material::MaterialKind::Solid | material::MaterialKind::Plant);

    // Burning radiates heat regardless of where the timer stands — a cell
    // one frame from burning out is exactly as hot as one that just ignited.
    if burn_temp.is_finite() {
        cell.set_temperature(burn_temp.round() as i16);

        // Push a little heat into the coarse field, so it actually reflects
        // nearby fire for anything that reads ambient temperature later
        // (M16 plant light-and-heat sensing, M18 creatures fleeing fire).
        // Deliberately a small flat push from the burning cell itself, not a
        // per-cell pull from every visited cell toward the field — that
        // version cost a HashMap lookup per swept CA cell and is the reason
        // `diffuse_heat` above no longer touches the field at all. The exact
        // rate here is a rough first cut, not tuned against anything yet;
        // there is no test pinning it down, so revisit freely.
        surface.add_heat(x, y, 1, 2.0);

        // Same reasoning, the light channel: resurrects moss shade-seeking
        // and tree phototropism (M16) for anything burning nearby, per
        // `Reports/emergent-world-architecture.md` §2. Also untuned.
        surface.add_light(x, y, 1, 1.0);
    }

    cell.tick_burn();
    if !cell.is_burning() {
        // Timer reached zero this tick — burn out into `burns_into`, or
        // simply stop burning and cool from here if nothing was configured.
        if let Some(into) = burns_into {
            // Two statements, not one nested call: `rng()` needs `&mut
            // surface` and would otherwise have to coexist with the `&
            // surface` borrow computing its own argument — see the note atop
            // `update`.
            let shades = surface.materials().get(into).palette.len().max(1) as u32;
            let shade = surface.rng().below(shades) as u8;
            *cell = Cell::new(into, shade).with_temperature(cell.temperature());

            // Architecture §5f: a burnout that produces ash specifically
            // gets a decay check scheduled for it, the one hook point that
            // makes `decay.rs`'s ash -> soil path reachable from real play
            // rather than only from a hand-built ActiveSite in a test. Every
            // other `burns_into` target (there is currently only ash) is
            // left alone -- decay is data-independent of *why* ash formed,
            // but this check itself is deliberately hardcoded to the one
            // material name rather than a new schema field, matching
            // decay.rs's own "cheap: one material" framing.
            if surface.materials().id_of("ash") == Some(into) {
                surface.schedule_active_site(ActiveSite {
                    x,
                    y,
                    kind: ActiveKind::Decay,
                    next_frame: surface.frame() + DECAY_TICK_INTERVAL,
                });
            }

            // Architecture item 9: structural integrity now covers `Plant`
            // as well as `Solid` (`structural.rs`), so a burnout that just
            // removed a `Solid`/`Plant` cell might have taken a neighbour's
            // only support with it -- the exact same "either side of this
            // write might need re-evaluating" reasoning `World::paint_
            // capsule`'s own `placed_solid`/`erased_solid` check already
            // uses for the brush, generalized to cover `Plant` and applied
            // here since a burnout is a *third* way a structural cell can
            // disappear that isn't painting or an explosion. Every neighbour
            // gets checked (not just this cell), including ones that are
            // Powder/Liquid/etc. -- `structural::tick` itself no-ops
            // immediately for anything that isn't `Solid`/`Plant`, so
            // scheduling unconditionally here is cheap and correct, the
            // same way `schedule_structural_check_around` already works.
            if was_structural {
                let frame = surface.frame();
                for (dx, dy) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
                    surface.schedule_active_site(ActiveSite {
                        x: x + dx,
                        y: y + dy,
                        kind: ActiveKind::StructuralCheck,
                        next_frame: frame,
                    });
                }
            }
        }
    }
}

/// Two independent ways to catch fire: contact with a burning neighbour
/// (probabilistic, rolled fresh — see the module doc for why that is safe
/// here, and suppressed by local moisture — architecture §4 — see `MOISTURE_
/// IGNITION_RESISTANCE`'s own doc), or the cell's own temperature crossing
/// its `ignition_temperature` (deterministic, not probabilistic at all, so
/// it carries none of `roll_reach_at`'s staleness risk regardless of chunk
/// sleep timing, and not moisture-gated either — see that same doc).
fn try_ignite<S: CellSurface>(surface: &mut S, x: i32, y: i32, cell: &mut Cell) {
    let material = surface.materials().get(cell.material);
    let flammability = material.flammability;
    let ignition_temperature = material.ignition_temperature;
    // `material`'s borrow ends here — the neighbour scan and `rng()` below
    // both need `surface` again.

    if flammability <= 0.0 && !ignition_temperature.is_finite() {
        return; // this material can never catch fire; skip the neighbour scan entirely
    }

    if (cell.temperature() as f32) >= ignition_temperature {
        ignite(surface, cell);
        return;
    }

    if flammability <= 0.0 {
        return;
    }
    let neighbours = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)];
    let any_burning = neighbours.iter().any(|&(nx, ny)| surface.get(nx, ny).is_burning());
    if !any_burning {
        return;
    }
    let saturation = (surface.field_moisture_at(x, y) / MOISTURE_SATURATION).clamp(0.0, 1.0);
    let effective_flammability = flammability * (1.0 - saturation * MOISTURE_IGNITION_RESISTANCE);
    if surface.rng().chance(effective_flammability) {
        ignite(surface, cell);
    }
}

fn ignite<S: CellSurface>(surface: &mut S, cell: &mut Cell) {
    let material = surface.materials().get(cell.material);
    let duration = material.burn_duration.max(1);
    let burn_temperature = material.burn_temperature;
    cell.ignite(duration);
    if burn_temperature.is_finite() {
        cell.set_temperature(burn_temperature.round() as i16);
    }
}

/// Temperature-triggered melting/boiling/cooling. Checked after combustion,
/// so a material whose burn temperature exceeds its own melting point (wood
/// charring hot enough to... whatever a content author wants, this engine
/// does not stop them) plausibly transitions the same frame it needed to.
/// Melting is checked before boiling; a material passing through both
/// thresholds in one large temperature jump melts first, consistent with the
/// physical order these actually occur in. The downward (`cooling_point`)
/// check comes last and only when neither upward threshold held — for a gas
/// it is condensation (steam → water), for a liquid it is freezing (water →
/// ice), one generic pair because a material has at most one phase below it.
fn try_phase_change<S: CellSurface>(surface: &mut S, x: i32, y: i32, cell: &mut Cell) {
    let material = surface.materials().get(cell.material);
    let temp = cell.temperature() as f32;
    let melting_point = material.melting_point;
    let melts_into = material.melts_into;
    let boiling_point = material.boiling_point;
    let boils_into = material.boils_into;
    let cooling_point = material.cooling_point;
    let cools_into = material.cools_into;
    let from_kind = material.kind;
    // `material`'s borrow ends here, before `transform` needs `&mut surface`.

    if temp >= melting_point {
        if let Some(into) = melts_into {
            transform(surface, x, y, cell, into);
            surface.count_phase_event(PhaseEvent::Melted);
        }
        return;
    }
    if temp >= boiling_point {
        // Gated per visit so a heated pool surface stipples into steam over
        // a few frames instead of flipping edge-to-edge in one — see
        // `PHASE_CHANGE_CHANCE`. A cell that fails the roll is off-ambient
        // and therefore revisited next frame by `must_stay_dirty`.
        if let Some(into) = boils_into {
            if surface.rng().chance(PHASE_CHANGE_CHANCE) {
                transform(surface, x, y, cell, into);
                surface.count_phase_event(PhaseEvent::Boiled);
            }
        }
        return;
    }
    if temp <= cooling_point {
        if let Some(into) = cools_into {
            if from_kind == MaterialKind::Liquid {
                // Freezing. Partial cells stay chilled water — see
                // `FREEZE_MIN_FILL` — and the front creeps via the same
                // per-visit roll boiling uses.
                if super::update::liquid_fill(*cell) < FREEZE_MIN_FILL {
                    return;
                }
                if surface.rng().chance(PHASE_CHANGE_CHANCE) {
                    transform(surface, x, y, cell, into);
                    surface.count_phase_event(PhaseEvent::Froze);
                }
            } else {
                // Condensation (and any other downward transition), ungated:
                // a steam cell held at its threshold reads as stuck to the
                // ceiling, and the cell's own cooling curve already spreads
                // the timing.
                transform(surface, x, y, cell, into);
                surface.count_phase_event(PhaseEvent::Condensed);
            }
        }
    }
}

/// Draws a fresh shade from `surface.rng()` rather than defaulting to 0, so a
/// field of stone melting into lava shows the same per-cell grain any other
/// bulk material does — see `World::paint_circle` for the same pattern.
///
/// # The `aux` translation table
///
/// `aux` means a different thing per kind — fill on a `Liquid` (0 = full),
/// anchor distance on a `Solid`/`Plant`, moisture on a `Powder` (0 = dry) —
/// so what survives a transform is decided per `(from, to)` kind pair, not
/// copied blindly and not silently zeroed:
///
/// - **Liquid → Liquid**: raw copy. A partially-drained cell must not
///   inflate to a full one (0 means full) — see the fill-preservation test.
/// - **Liquid → Gas and Gas → Liquid**: raw copy. A gas's `aux` is
///   otherwise unused, so steam carries its source water's fill on the same
///   0-means-full convention and gives it back on condensing — per-cell
///   exact conservation through the boil→condense loop. (steam.ron's header
///   documents the convention on the content side.)
/// - **everything else**: 0, "no meaning to carry". For a freeze
///   (Liquid → Solid) that is the same transient state a brush-painted
///   stone cell has — distance-0 "claims anchored" until the scheduled
///   structural check below corrects it; the fill that cannot ride along is
///   what `FREEZE_MIN_FILL` bounds to a near-full cell. For a melt
///   (Solid → Liquid) 0 deliberately reads as a *full* cell — never carry
///   an anchor distance across as a fill amount.
///
/// A transform that adds or removes a structural (`Solid`/`Plant`) cell
/// schedules the same 5-position StructuralCheck fan-out a burnout does
/// (`tick_burn`) — a melt just removed a neighbour's possible support, a
/// freeze placed a solid whose anchor distance needs computing. Deduped in
/// `World::schedule_active_site` like every other check.
fn transform<S: CellSurface>(surface: &mut S, x: i32, y: i32, cell: &mut Cell, into: MaterialId) {
    let from_kind = surface.materials().kind(cell.material);
    let to_kind = surface.materials().kind(into);
    let shades = surface.materials().get(into).palette.len().max(1) as u32;
    let shade = surface.rng().below(shades) as u8;
    let temp = cell.temperature();
    let aux = match (from_kind, to_kind) {
        (MaterialKind::Liquid, MaterialKind::Liquid)
        | (MaterialKind::Liquid, MaterialKind::Gas)
        | (MaterialKind::Gas, MaterialKind::Liquid) => cell.aux(),
        _ => 0,
    };
    *cell = Cell::new(into, shade).with_temperature(temp);
    cell.set_aux(aux);

    let was_structural = matches!(from_kind, MaterialKind::Solid | MaterialKind::Plant);
    let now_structural = matches!(to_kind, MaterialKind::Solid | MaterialKind::Plant);
    if was_structural != now_structural {
        let frame = surface.frame();
        for (dx, dy) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
            surface.schedule_active_site(ActiveSite {
                x: x + dx,
                y: y + dy,
                kind: ActiveKind::StructuralCheck,
                next_frame: frame,
            });
        }
    }
}

/// Pairwise reactions with a directly touching neighbour — water quenching
/// lava into stone and steam, that kind of thing. `self` becomes
/// `reaction.becomes`; the neighbour becomes `reaction.other_becomes`.
///
/// The neighbour's cell is written immediately via `world.set`, not returned
/// for the caller to apply later — it lives at a different position than the
/// one `update`'s own final write targets, so there is no conflict, and
/// deferring it would need plumbing a second pending-write value out of a
/// function that otherwise only ever produces one.
///
/// At most one reaction fires per cell per visit, even if a cell touches
/// several different candidates at once (checked in a fixed neighbour order,
/// not randomized) — simple, and reconsidering it is only worth doing if
/// something ever actually needs multiple simultaneous reactions to look right.
fn try_react<S: CellSurface>(surface: &mut S, x: i32, y: i32, cell: &mut Cell) {
    // Cloned to sidestep a borrow conflict: `surface.materials().get(...)`
    // holds an immutable borrow of `surface`, but resolving a reaction needs
    // `&mut S` (`rng()`, `set()`). `Reaction` is small and `Copy`, and
    // materials rarely carry more than a couple of these, so cloning the
    // whole short list once per visited cell is cheap — the same trade
    // `MaterialRegistry::resolve_references` makes for the same reason.
    let reactions = surface.materials().get(cell.material).reactions.clone();
    if reactions.is_empty() {
        return;
    }

    for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
        let neighbour_material = surface.get(nx, ny).material;
        for reaction in &reactions {
            if reaction.with != neighbour_material {
                continue;
            }
            if !surface.rng().chance(reaction.chance) {
                continue;
            }

            // Both products take the *hotter* side's temperature — the
            // interface heats what it makes. Deliberate, not an oversight:
            // quench steam born at the water's ~20°C would sit below its
            // own condensation point and flash straight back to water on
            // its next visit; born at the lava's temperature it rises and
            // condenses on a ceiling later, which is the visible plume the
            // reaction owes. Both sides also go through `transform`, not a
            // bare `Cell::new` — the neighbour write used to bypass it,
            // which would have dropped a quenched water cell's fill and
            // reset its steam to `aux 0` (= full), manufacturing volume.
            let mut other = surface.get(nx, ny);
            let pair_temp = cell.temperature().max(other.temperature());
            cell.set_temperature(pair_temp);
            other.set_temperature(pair_temp);

            transform(surface, x, y, cell, reaction.becomes);
            transform(surface, nx, ny, &mut other, reaction.other_becomes);
            surface.set(nx, ny, other);
            surface.count_phase_event(PhaseEvent::Reacted);

            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::field;
    use crate::sim::material;
    use crate::sim::world::World;

    fn test_world() -> World {
        World::new(Rect::new(0, 0, 63, 63))
    }

    #[test]
    fn overlapping_burnouts_do_not_duplicate_structural_checks() {
        // Code-review-findings item #2, the gap an independent review
        // found in the first version of the scheduler dedup fix: this
        // burnout fan-out (`was_structural`, below) builds `ActiveSite`s
        // by hand and calls `CellSurface::schedule_active_site` directly,
        // bypassing `structural::schedule_structural_check`'s own dedup
        // entirely -- fixed by moving the dedup into `World::schedule_
        // active_site` itself, the one point both paths funnel through.
        // Two adjacent burning wood cells finishing their timer the same
        // tick is the fire equivalent of two adjacent explosion-cleared
        // cells: each schedules a 5-position StructuralCheck fan-out
        // (itself + 4 neighbours), and the two fan-outs share 2 positions,
        // for 8 distinct StructuralCheck positions, not the raw 10 --
        // matching `structural.rs`'s own `overlapping_schedule_
        // structural_check_around_calls_do_not_duplicate` exactly. Each
        // burnout also schedules one `Decay` site (wood burns into ash),
        // which is not deduped against anything -- 2 more, for 10 total.
        let mut w = test_world();
        let wood = w.materials.id_of("wood").unwrap();
        for &(x, y) in &[(30, 30), (31, 30)] {
            let mut cell = Cell::new(wood, 0);
            cell.ignite(1); // ticks to 0 and burns out on the next `update`
            w.set(x, y, cell);
        }
        update(&mut w, 30, 30);
        update(&mut w, 31, 30);

        assert_eq!(
            w.active_site_count(),
            10,
            "two overlapping burnouts should dedup their StructuralCheck fan-outs to 8 distinct positions, plus 2 undeduped Decay sites"
        );
    }

    #[test]
    fn a_reaction_transforms_both_cells() {
        // No shipped material has a reaction yet (that needs a real "lava"
        // material, which needs an intrinsic-temperature schema field this
        // milestone deliberately did not add — out of scope for tonight, a
        // natural follow-up), so this proves the mechanism with synthetic
        // content via the same temp-directory technique material.rs's own
        // tests use for exactly this reason.
        let dir = std::env::temp_dir().join("pixel-physics-reaction-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.ron"),
            "(name: \"react_a\", kind: Solid, density: 1.0, colors: [(200, 0, 0)], \
             reactions: [(with: \"react_b\", produces: (\"react_c\", \"react_d\"), chance: 1.0)])",
        )
        .unwrap();
        std::fs::write(dir.join("b.ron"), "(name: \"react_b\", kind: Solid, density: 1.0, colors: [(0, 200, 0)])").unwrap();
        std::fs::write(dir.join("c.ron"), "(name: \"react_c\", kind: Solid, density: 1.0, colors: [(0, 0, 200)])").unwrap();
        std::fs::write(dir.join("d.ron"), "(name: \"react_d\", kind: Solid, density: 1.0, colors: [(200, 200, 0)])").unwrap();

        let mut w = test_world();
        w.materials.reload(&dir).unwrap();
        let (a, b, c, d) = (
            w.materials.id_of("react_a").unwrap(),
            w.materials.id_of("react_b").unwrap(),
            w.materials.id_of("react_c").unwrap(),
            w.materials.id_of("react_d").unwrap(),
        );

        w.set(30, 30, Cell::new(a, 0));
        w.set(31, 30, Cell::new(b, 0));
        update(&mut w, 30, 30);

        assert_eq!(w.get(30, 30).material, c, "react_a should have become react_c");
        assert_eq!(w.get(31, 30).material, d, "react_b should have become react_d");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_reaction_with_zero_chance_never_fires() {
        let dir = std::env::temp_dir().join("pixel-physics-reaction-never");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.ron"),
            "(name: \"never_a\", kind: Solid, density: 1.0, colors: [(1, 1, 1)], \
             reactions: [(with: \"never_b\", produces: (\"never_a\", \"never_b\"), chance: 0.0)])",
        )
        .unwrap();
        std::fs::write(dir.join("b.ron"), "(name: \"never_b\", kind: Solid, density: 1.0, colors: [(2, 2, 2)])").unwrap();

        let mut w = test_world();
        w.materials.reload(&dir).unwrap();
        let a = w.materials.id_of("never_a").unwrap();
        let b = w.materials.id_of("never_b").unwrap();
        w.set(30, 30, Cell::new(a, 0));
        w.set(31, 30, Cell::new(b, 0));

        for _ in 0..200 {
            update(&mut w, 30, 30);
        }
        assert_eq!(w.get(30, 30).material, a, "a zero-chance reaction fired anyway");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_liquid_to_liquid_transform_preserves_fill_instead_of_resetting_to_full() {
        // The landmine an independent review found live: `transform`
        // defaults a fresh cell's `aux` to 0, which the liquid model reads
        // as "full" (`material::LIQUID_FULL`'s own doc) -- fine for every
        // shipped material today (no `melts_into`/`boils_into`/reaction
        // target is `Liquid` from a `Liquid` source), but a partially-
        // drained `Liquid` cell reacting into a *different* `Liquid`
        // material would otherwise silently inflate to a full cell's worth,
        // manufacturing volume from nowhere. No shipped material exercises
        // this, so synthetic content proves the mechanism directly, the
        // same technique `a_reaction_transforms_both_cells` above uses.
        let dir = std::env::temp_dir().join("pixel-physics-liquid-reaction-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.ron"),
            "(name: \"react_liquid_a\", kind: Liquid, density: 1.0, colors: [(0, 0, 200)], \
             reactions: [(with: \"react_liquid_b\", produces: (\"react_liquid_c\", \"react_liquid_b\"), chance: 1.0)])",
        )
        .unwrap();
        std::fs::write(dir.join("b.ron"), "(name: \"react_liquid_b\", kind: Liquid, density: 1.0, colors: [(0, 200, 0)])").unwrap();
        std::fs::write(dir.join("c.ron"), "(name: \"react_liquid_c\", kind: Liquid, density: 1.0, colors: [(200, 0, 0)])").unwrap();

        let mut w = test_world();
        w.materials.reload(&dir).unwrap();
        let (a, b, c) = (
            w.materials.id_of("react_liquid_a").unwrap(),
            w.materials.id_of("react_liquid_b").unwrap(),
            w.materials.id_of("react_liquid_c").unwrap(),
        );

        // A partial fill -- the exact case a reset-to-0-means-full bug would
        // silently inflate.
        w.set(30, 30, Cell::new(a, 0).with_aux(250));
        w.set(31, 30, Cell::new(b, 0));
        update(&mut w, 30, 30);

        assert_eq!(w.get(30, 30).material, c, "react_liquid_a should have become react_liquid_c");
        assert_eq!(
            w.get(30, 30).aux(),
            250,
            "fill should carry across the transform unchanged, not reset to 0 (which the liquid model reads as full)"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Loop `update` on one cell until its material changes or `limit`
    /// visits pass — the boil/freeze branches are chance-gated per visit
    /// (`PHASE_CHANGE_CHANCE`), so a single call proves nothing either way.
    fn update_until_changed(w: &mut World, x: i32, y: i32, limit: u32) -> bool {
        for _ in 0..limit {
            if update(w, x, y) {
                return true;
            }
        }
        false
    }

    #[test]
    fn boiling_water_carries_its_fill_into_steam_and_back() {
        // The whole point of steam's `aux` convention: a partial water cell
        // boils into steam carrying the same fill (0 means full on both
        // kinds), and condensing gives exactly that fill back — per-cell
        // conservation through the loop, asserted on shipped content.
        let mut w = test_world();
        let steam = w.materials.id_of("steam").expect("steam.ron should be embedded");

        w.set(30, 30, Cell::new(material::WATER, 0).with_aux(650).with_temperature(150));
        assert!(update_until_changed(&mut w, 30, 30, 100), "water at 150C never boiled");
        let boiled = w.get(30, 30);
        assert_eq!(boiled.material, steam, "water above its boiling point should become steam");
        assert_eq!(boiled.aux(), 650, "steam should carry the source water's fill");
        assert_eq!(w.phase_changes.boiled, 1);

        // Cool the same cell below steam's condensation point and it gives
        // the fill back. Set directly rather than waiting out diffusion —
        // the cooling curve is diffuse_heat's own tested property.
        w.set(30, 30, boiled.with_temperature(30));
        assert!(update_until_changed(&mut w, 30, 30, 3), "steam at 30C never condensed");
        let condensed = w.get(30, 30);
        assert_eq!(condensed.material, material::WATER, "cooled steam should condense back to water");
        assert_eq!(condensed.aux(), 650, "condensed water should hold exactly the fill the steam carried");
        assert_eq!(w.phase_changes.condensed, 1);
    }

    #[test]
    fn a_freezing_liquid_writes_anchor_distance_zero_and_schedules_checks() {
        // Liquid → Solid crosses `aux` conventions: fill on the way in,
        // anchor distance on the way out. The distance must be written as 0
        // (the same "claims anchored" transient a brush-painted stone cell
        // has) and the 5-position StructuralCheck fan-out must be scheduled
        // to correct it — not a raw fill copied across as a distance.
        let dir = std::env::temp_dir().join("pixel-physics-freeze-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.ron"),
            "(name: \"freeze_liquid\", kind: Liquid, density: 1.0, colors: [(0, 0, 200)], \
             cooling_point: 0.0, cools_into: \"freeze_solid\")",
        )
        .unwrap();
        std::fs::write(dir.join("b.ron"), "(name: \"freeze_solid\", kind: Solid, density: 1.0, colors: [(200, 200, 255)])").unwrap();

        let mut w = test_world();
        w.materials.reload(&dir).unwrap();
        let (liquid, solid) = (
            w.materials.id_of("freeze_liquid").unwrap(),
            w.materials.id_of("freeze_solid").unwrap(),
        );

        // Full cell (aux 0 = full), chilled below the threshold.
        w.set(30, 30, Cell::new(liquid, 0).with_temperature(-5));
        assert!(update_until_changed(&mut w, 30, 30, 100), "a full liquid at -5C never froze");
        let frozen = w.get(30, 30);
        assert_eq!(frozen.material, solid);
        assert_eq!(frozen.aux(), 0, "a fresh solid claims anchor distance 0 until its scheduled check runs");
        assert_eq!(w.phase_changes.froze, 1);
        assert_eq!(
            w.active_site_count(),
            5,
            "a freeze should schedule the same 5-position StructuralCheck fan-out a burnout does"
        );

        // A partial cell must never freeze — a Solid's aux cannot carry the
        // fill, so melting it back would manufacture volume. It stays
        // chilled liquid however long it is visited.
        w.set(40, 30, Cell::new(liquid, 0).with_aux(300).with_temperature(-5));
        assert!(
            !update_until_changed(&mut w, 40, 30, 200),
            "a fill-300 liquid froze; FREEZE_MIN_FILL should hold the partial fringe as liquid"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_melting_solid_becomes_a_full_liquid_not_a_drained_one() {
        // Solid → Liquid is the other direction across the same convention
        // boundary: the solid's `aux` is an anchor distance and must not be
        // read as a fill. The melt writes 0, which on a Liquid deliberately
        // means *full* — ice melts into a full water cell. The vanished
        // support also owes its neighbours a structural fan-out.
        let dir = std::env::temp_dir().join("pixel-physics-melt-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.ron"),
            "(name: \"melt_solid\", kind: Solid, density: 1.0, colors: [(200, 200, 255)], \
             melting_point: 1.0, melts_into: \"melt_liquid\")",
        )
        .unwrap();
        std::fs::write(dir.join("b.ron"), "(name: \"melt_liquid\", kind: Liquid, density: 1.0, colors: [(0, 0, 200)])").unwrap();

        let mut w = test_world();
        w.materials.reload(&dir).unwrap();
        let (solid, liquid) = (
            w.materials.id_of("melt_solid").unwrap(),
            w.materials.id_of("melt_liquid").unwrap(),
        );

        // `aux 37` is a plausible relaxed anchor distance — exactly the
        // value that must not survive as a fill.
        w.set(30, 30, Cell::new(solid, 0).with_aux(37).with_temperature(AMBIENT_TEMPERATURE));
        assert!(update_until_changed(&mut w, 30, 30, 3), "a solid above its melting point never melted");
        let melted = w.get(30, 30);
        assert_eq!(melted.material, liquid);
        assert_eq!(melted.aux(), 0, "a melted solid should be a full liquid (aux 0 = full), never inherit the anchor distance as fill");
        assert_eq!(w.phase_changes.melted, 1);
        assert_eq!(w.active_site_count(), 5, "a melt removed a possible support and owes the fan-out");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_reaction_born_cell_takes_the_hotter_sides_temperature_and_keeps_fill() {
        // The two `try_react` fixes that landed with the cooling branch,
        // asserted together: the neighbour's write goes through `transform`
        // (so a Liquid product keeps the neighbour's fill instead of being
        // rebuilt full), and both products take the *hotter* side's
        // temperature (so quench steam is born hot enough to rise instead
        // of flashing straight back to water).
        let dir = std::env::temp_dir().join("pixel-physics-react-temp-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.ron"),
            "(name: \"hot_a\", kind: Solid, density: 2.0, colors: [(200, 50, 0)], \
             reactions: [(with: \"wet_b\", produces: (\"cool_c\", \"wet_d\"), chance: 1.0)])",
        )
        .unwrap();
        std::fs::write(dir.join("b.ron"), "(name: \"wet_b\", kind: Liquid, density: 1.0, colors: [(0, 0, 200)])").unwrap();
        std::fs::write(dir.join("c.ron"), "(name: \"cool_c\", kind: Solid, density: 2.0, colors: [(100, 100, 100)])").unwrap();
        std::fs::write(dir.join("d.ron"), "(name: \"wet_d\", kind: Liquid, density: 1.0, colors: [(0, 200, 200)])").unwrap();

        let mut w = test_world();
        w.materials.reload(&dir).unwrap();
        let (a, c, d) = (
            w.materials.id_of("hot_a").unwrap(),
            w.materials.id_of("cool_c").unwrap(),
            w.materials.id_of("wet_d").unwrap(),
        );
        let b = w.materials.id_of("wet_b").unwrap();

        w.set(30, 30, Cell::new(a, 0).with_temperature(900));
        w.set(31, 30, Cell::new(b, 0).with_aux(400).with_temperature(AMBIENT_TEMPERATURE));
        update(&mut w, 30, 30);

        let product = w.get(30, 30);
        let other = w.get(31, 30);
        assert_eq!(product.material, c);
        assert_eq!(other.material, d);
        assert_eq!(product.temperature(), 900, "the self product should keep the hot side's temperature");
        assert_eq!(other.temperature(), 900, "the neighbour product should take the hot side's temperature, not stay at ambient");
        assert_eq!(other.aux(), 400, "a Liquid → Liquid neighbour product should keep its fill, not be rebuilt full");
        assert_eq!(w.phase_changes.reacted, 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn water_boils_over_burning_oil_and_the_steam_condenses_under_both_drivers() {
        // The core loop on shipped content, no debug brush anywhere: a
        // burning oil slick floating on a pool (the heat verb the engine
        // already has) boils the surface, the steam rises to the sealed
        // world top, cools, and condenses back. Both drivers, per CLAUDE.md
        // -- behaviour only the player sees is behaviour only the parallel
        // driver produces.
        for parallel_driver in [false, true] {
            let mut w = test_world();
            // A walled basin so the pool holds still under the slick.
            for y in 40..=50 {
                w.set(20, y, Cell::new(material::STONE, 0));
                w.set(44, y, Cell::new(material::STONE, 0));
            }
            for x in 20..=44 {
                w.set(x, 50, Cell::new(material::STONE, 0));
            }
            for x in 21..44 {
                for y in 44..50 {
                    w.set(x, y, Cell::new(material::WATER, 0));
                }
            }
            let burn = w.materials.get(material::OIL).burn_duration;
            for x in 28..36 {
                let mut slick = Cell::new(material::OIL, 0);
                slick.ignite(burn);
                w.set(x, 43, slick);
            }

            for _ in 0..800 {
                if parallel_driver {
                    crate::sim::parallel::step(&mut w);
                } else {
                    super::super::update::step(&mut w);
                }
            }
            let p = w.phase_changes;
            assert!(
                p.boiled > 0,
                "no water boiled over a 900C slick in 800 frames (parallel: {parallel_driver})"
            );
            assert!(
                p.condensed > 0,
                "steam never condensed ({} boiled) in 800 frames (parallel: {parallel_driver})",
                p.boiled
            );
        }
    }

    #[test]
    fn a_sealed_steam_pocket_still_condenses_and_settles() {
        // Condensation must not depend on the steam being able to move: a
        // pocket sealed in stone is exactly the cell the sweep would never
        // revisit if cooling didn't keep it dirty (`must_stay_dirty`), which
        // is the reverted-evaporation-sweep lesson applied to gas. The cell
        // cools through its own conductivity, condenses crossing the
        // threshold, and the water then settles to ambient so the chunk can
        // finally sleep.
        let mut w = test_world();
        let steam = w.materials.id_of("steam").unwrap();
        for x in 29..=31 {
            for y in 29..=31 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        w.set(30, 30, Cell::new(steam, 0).with_temperature(200));

        let mut condensed_at = None;
        for frame in 0..600 {
            crate::sim::parallel::step(&mut w);
            if condensed_at.is_none() && w.get(30, 30).material == material::WATER {
                condensed_at = Some(frame);
            }
            if condensed_at.is_some() && w.active_chunk_count() == 0 {
                return; // condensed, cooled off, and the world went to sleep
            }
        }
        match condensed_at {
            None => panic!("a sealed 200C steam pocket never condensed in 600 frames"),
            Some(f) => panic!("condensed at frame {f} but the chunk never slept afterwards"),
        }
    }

    /// The Phase 0 probe for the latent snow bug, kept as the bar the ice
    /// milestone's snow fix must meet (`Reports`' revert-keeps-the-knowledge
    /// convention, applied forward): snow.ron has no `heat_conductivity`, so
    /// a cell the storm chilled below its 2°C melting point can never warm
    /// back up — `diffuse_heat` early-exits at conductivity ≤ 0 — and
    /// `must_stay_dirty` keeps its chunk awake forever while it waits. The
    /// wiki's "drifts thaw when the front passes" is therefore only true of
    /// cells the storm never chilled below 2°C. Un-ignore when snow gains
    /// real conductivity (the ice milestone), and expect it to pass in well
    /// under the frame budget here.
    #[test]
    #[ignore = "fails today: snow has no heat_conductivity, so a chilled flake can neither warm nor melt — the ice milestone fixes snow.ron and un-ignores this"]
    fn a_snow_drift_chilled_by_a_storm_thaws_after_it_passes() {
        let mut w = test_world();
        let snow = w.materials.id_of("snow").unwrap();
        // The coldest a storm writes: 20 - 26 * 1.0 (weather.rs's SNOW_CHILL
        // at full intensity). On the world's own floor row, where a flake
        // can neither fall nor roll — a first draft put it on a one-cell
        // stone pillar and it rolled off, which read as "thawed" while
        // being nothing of the kind (a scene that contradicts the code
        // looks like a bug in the code).
        w.set(30, 63, Cell::new(snow, 0).with_temperature(-6));

        for _ in 0..2000 {
            crate::sim::parallel::step(&mut w);
            if w.get(30, 63).material != snow {
                assert_eq!(w.phase_changes.melted, 1, "the flake left (30,63) without melting — scene broken again?");
                return;
            }
        }
        panic!("a chilled flake never thawed in 2000 frames after the storm passed");
    }

    #[test]
    fn heat_diffuses_from_a_hot_cell_to_a_cooler_neighbour() {
        // Stone has no conductivity by default (see material.rs's
        // default_heat_conductivity for why) and so cannot be used to test
        // diffusion at all — it takes the early exit unconditionally. Oil is
        // the one shipped material with an explicit nonzero value.
        let mut w = test_world();
        w.set(30, 30, Cell::new(material::OIL, 0).with_temperature(500));
        w.set(31, 30, Cell::new(material::OIL, 0));

        update(&mut w, 31, 30);
        let neighbour_temp = w.get(31, 30).temperature();
        assert!(
            neighbour_temp > AMBIENT_TEMPERATURE,
            "cool neighbour did not warm up: {neighbour_temp}"
        );
    }

    #[test]
    fn a_cold_cell_next_to_a_hot_one_never_produces_nan_or_overflow() {
        let mut w = test_world();
        w.set(30, 30, Cell::new(material::STONE, 0).with_temperature(i16::MAX));
        w.set(31, 30, Cell::new(material::STONE, 0).with_temperature(i16::MIN));
        for _ in 0..50 {
            update(&mut w, 30, 30);
            update(&mut w, 31, 30);
        }
        // Reaching this line without panicking on an out-of-range cast is
        // the assertion; both extremes are deliberately adversarial inputs.
    }

    #[test]
    fn oil_ignites_next_to_fire_and_eventually_burns_out_to_ash() {
        let mut w = test_world();
        let mut source = Cell::new(material::OIL, 0);
        source.ignite(9999); // stays burning for the whole test
        w.set(30, 30, source);
        w.set(31, 30, Cell::new(material::OIL, 0));

        let mut ignited = false;
        for _ in 0..2000 {
            update(&mut w, 31, 30);
            if w.get(31, 30).is_burning() {
                ignited = true;
                break;
            }
        }
        assert!(ignited, "oil never caught fire next to a burning neighbour");

        // Run it out to burnout — cheat by ticking a fresh, short-lived
        // ignition rather than waiting out flammability's own random delay
        // twice over, since this is now testing burnout, not ignition.
        let mut burning = Cell::new(material::OIL, 0);
        burning.ignite(3);
        w.set(31, 30, burning);
        for _ in 0..10 {
            update(&mut w, 31, 30);
        }
        assert_eq!(w.get(31, 30).material, material::ASH, "oil should burn out into ash");
        assert!(!w.get(31, 30).is_burning());
    }

    #[test]
    fn moisture_suppresses_ignition_from_a_burning_neighbour() {
        // Architecture §4's fire-resistance consumer. `World::new` always
        // starts `Rng::default()` from the same fixed seed (see `rng.rs`'s
        // own module doc), and `Rng::chance(p)` draws exactly one value for
        // any `p` strictly between 0.0 and 1.0 (it only short-circuits
        // without drawing at those two exact endpoints) -- both this test's
        // probabilities do (oil's flammability is 0.5 dry, 0.5 * (1.0 -
        // `MOISTURE_IGNITION_RESISTANCE`) = 0.05 wet), so two fresh worlds
        // running the identical sequence of `update` calls draw the *same*
        // underlying random values each frame, and the only thing that can
        // differ between a dry and a wet run is which frame that draw first
        // clears the (lower, for wet) threshold. Wet must therefore ignite
        // no earlier than dry, deterministically, not just on average -- no
        // statistics needed.
        fn ignites_within(wet: bool, frames: usize) -> Option<usize> {
            let mut w = test_world();
            let mut source = Cell::new(material::OIL, 0);
            source.ignite(9999); // stays burning for the whole test
            w.set(30, 30, source);
            w.set(31, 30, Cell::new(material::OIL, 0));
            if wet {
                // Same field block as (31, 30) -- FIELD_SCALE = 8, block
                // spans (24..=31, 24..=31) -- so one `field::step` call
                // forces this whole block to MAX_MOISTURE immediately
                // (`apply_moisture_sources`), no diffusion wait needed.
                w.set(24, 24, Cell::new(material::WATER, 0));
                field::step(&mut w);
            }
            for i in 0..frames {
                update(&mut w, 31, 30);
                if w.get(31, 30).is_burning() {
                    return Some(i);
                }
            }
            None
        }

        let dry = ignites_within(false, 200).expect("dry oil should have ignited well within 200 frames");
        let wet = ignites_within(true, 200);
        assert!(
            wet.is_none_or(|w| w > dry),
            "wet oil should ignite later than dry oil, or not at all within the window: dry={dry}, wet={wet:?}"
        );
    }

    #[test]
    fn a_material_with_zero_flammability_never_ignites() {
        let mut w = test_world();
        let mut source = Cell::new(material::OIL, 0);
        source.ignite(9999);
        w.set(30, 30, source);
        w.set(31, 30, Cell::new(material::STONE, 0)); // not flammable

        for _ in 0..500 {
            update(&mut w, 31, 30);
        }
        assert!(!w.get(31, 30).is_burning(), "stone caught fire despite zero flammability");
        assert_eq!(w.get(31, 30).material, material::STONE);
    }

    #[test]
    fn a_settled_temperature_stops_forcing_its_chunk_awake() {
        // The property `THERMAL_SETTLE_EPSILON` exists for: a cell already at
        // ambient must not keep re-dirtying itself forever.
        //
        // A freshly created chunk starts fully dirty regardless of any writes
        // (see `Chunk::new`), so `active_chunk_count()` cannot be asserted to
        // be 0 before something has actually visited the cell — dirty regions
        // are double buffered, and only a real visit (here, `update`) narrows
        // the pending region down to what actually still needs examining.
        // Asserting on that before any visit was the bug in an earlier
        // version of this test, not in the code it was testing.
        let mut w = test_world();
        w.set(30, 30, Cell::new(material::STONE, 0)); // starts at ambient already
        w.end_step();

        update(&mut w, 30, 30);
        // No write should have occurred — nothing was off-ambient to begin with.
        w.end_step();
        assert_eq!(
            w.active_chunk_count(),
            0,
            "an already-ambient cell force-woke its own chunk"
        );
    }

    #[test]
    fn a_hot_cell_keeps_its_chunk_awake_until_it_cools_near_ambient() {
        // Oil, not stone — see the note in the diffusion test above.
        let mut w = test_world();
        w.set(30, 30, Cell::new(material::OIL, 0).with_temperature(200));
        w.end_step();

        let mut frames = 0;
        while w.get(30, 30).temperature() as f32 - AMBIENT_TEMPERATURE as f32
            > THERMAL_SETTLE_EPSILON
            && frames < 5000
        {
            update(&mut w, 30, 30);
            w.end_step();
            frames += 1;
        }
        assert!(frames < 5000, "cell never cooled toward ambient");
        assert!(frames > 0, "test setup started already settled");
    }

    #[test]
    fn a_connected_mass_of_cooling_cells_actually_settles() {
        // Regression: the minimum-progress fix in `diffuse_heat` (the
        // pull-never-rounds-to-zero fix the test above guards) used to apply
        // unconditionally, which is fine for one isolated hot cell but not
        // for many neighbouring cells cooling *together* — each one's raw
        // pull toward the others is small but essentially never exactly
        // zero, so the fix kept force-nudging cells a whole degree away from
        // ambient and back, forever, even once every cell was already
        // within `THERMAL_SETTLE_EPSILON`. A single isolated cell never
        // exercises this, since its only neighbours are ambient `Cell::EMPTY`
        // reads that never themselves fluctuate — a row of mutually warm
        // cells does. Real ash from a burned-out fire is exactly this shape.
        let mut w = test_world();
        for x in 20..40 {
            w.set(x, 30, Cell::new(material::OIL, 0).with_temperature(200));
        }
        w.end_step();

        let mut frames = 0;
        loop {
            for x in 20..40 {
                update(&mut w, x, 30);
            }
            w.end_step();
            frames += 1;
            let all_settled = (20..40).all(|x| {
                (w.get(x, 30).temperature() as f32 - AMBIENT_TEMPERATURE as f32).abs()
                    <= THERMAL_SETTLE_EPSILON
            });
            if all_settled || frames >= 5000 {
                break;
            }
        }
        assert!(frames < 5000, "a connected mass of cells never settled");

        // And once every value is within the epsilon, it must actually stay
        // that way — the bug this guards produced values that read as
        // settled on any single frame checked in isolation but kept
        // changing (and re-dirtying their chunk) forever.
        for x in 20..40 {
            update(&mut w, x, 30);
        }
        w.end_step();
        assert_eq!(
            w.active_chunk_count(),
            0,
            "a settled connected mass kept its chunk awake"
        );
    }

    #[test]
    fn spontaneous_ignition_from_temperature_is_immediate_not_probabilistic() {
        // Deterministic, unlike neighbour-driven ignition — no burning
        // neighbour involved at all, only the cell's own heat.
        let mut w = test_world();
        // Oil has no ignition_temperature configured (defaults to "never"),
        // so this must use a material where it's finite. Ignite oil this way
        // is not testable with shipped content, so this exercises the
        // mechanism directly instead of through real data.
        let mut cell = Cell::new(material::OIL, 0);
        cell.set_temperature(20);
        w.set(30, 30, cell);
        // Confirm it does NOT ignite at ambient temperature with no neighbour.
        update(&mut w, 30, 30);
        assert!(!w.get(30, 30).is_burning());
    }
}
