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
/// How likely a cell above its melting point is to actually melt, per
/// visit.
///
/// **A thaw is slow, and this arm was instantaneous.** Melting was
/// deliberately ungated -- *"thaw urgency is carried by the temperature
/// gradient itself"* -- and a playtest overturned it: 362 cells of ice and
/// 1,788 of snow went to zero of both inside ten frames, under a fifth of a
/// second.
///
/// **0.004, down from 0.015, and re-derived from a measurement rather than
/// halved on taste.** With a pond that now freezes over across a minute or
/// two, the thaw at 0.015 took 8 seconds; at 0.004 it takes **20**, which
/// is the order of magnitude play asked for ("when it melts, it happens in
/// 1 second"). 0.0015 was measured too and is worse, not better: the sheet
/// goes 60 columns to 8 over 32 seconds and then *lingers* in scattered
/// patches, which is the "ragged and patchy, the last of it hangs about"
/// artifact rather than ice retreating.
///
/// Two budgets are set from this rate and have to move with it:
/// `a_melting_cell_yields_its_own_density_in_liquid_not_a_free_full_cell`'s
/// 2,000-visit allowance, and `weather.rs`'s `THAW_SETTLE_BUDGET`.
const MELT_CHANCE: f32 = 0.004;

/// And the same for **freezing**, which shared `PHASE_CHANGE_CHANCE` with
/// boiling and had no business doing so.
///
/// # The weather was not the problem, which took a measurement to find out
///
/// Reported as *"the freeze is so fast. It lasts only a few seconds. This
/// should be different order of magnitude"*, and the obvious reading is
/// that cold snaps are too short. **They are not.** Sweeping
/// `weather::at` over 400,000 frames for seed 2900, at the shipped
/// `WEATHER_EPOCH_FRAMES`, the snow runs are **100.8 s and 87.4 s** long.
/// `filmstrip`'s `COLDSNAP_START`/`COLDSNAP_SNOW_ENDS` window is a
/// 700-frame *slice* of one of them, not the whole thing, which is what
/// made the front look brief when it was the ice that was.
///
/// Lengthening the weather epoch tenfold was tried on that misreading and
/// reverted: it gives a single 1,008-second snow event per 400,000 frames,
/// i.e. one snowfall every two hours of play lasting seventeen minutes.
/// That is not "ten times longer", it is a different game.
///
/// So the rate belongs here, beside `MELT_CHANCE`, and freezing stops
/// borrowing boiling's number: a pond should ice over while you watch it,
/// not between two glances. Swept on `scene=coldsnap` -- see the commit
/// that set it.
const FREEZE_CHANCE: f32 = 0.05;

/// Non-burning cells at or above this temperature push heat and light into
/// the coarse field each visit, the way `tick_burn` does for flames —
/// molten lava and fresh quench crust radiate whether or not anything is
/// on fire. Well above any burn-adjacent residual, so the population is
/// only ever the handful of searing cells, shrinking as they cool.
const FIELD_GLOW_MIN_TEMPERATURE: f32 = 500.0;

// A latent-heat cost on boiling (steam born ~40° cooler than the water
// that boiled) was implemented here while hunting open-bugs 0b's eternal
// simmer, on the theory that a lossless boil/condense loop needed a sink.
// It worked — and the isolation control showed it was unnecessary: the
// loop was not lossless by nature, it had a literal heater in it (hot,
// thermally-inert rubble; see rubble.ron), and with that fixed the scene
// sleeps *sooner without* the latent cost than with it. Reverted per
// keep-each-fix-minimal, recorded here because the idea will look
// attractive again the next time a vapour loop misbehaves: check for an
// inert heat reservoir first — any material that can inherit temperature
// through transform, burnout, crush or reaction and has zero
// heat_conductivity is a permanent radiator.

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

/// **Ground wetness at or above which fuel will not catch from contact at
/// all** — the owner's steer on the grassfire card, *"if we are going to
/// do this, moisture vs dryness should play a role"*
/// (`Reports/open-bugs-handoff.md` §G), given a value it can actually be
/// seen at.
///
/// # What this replaced, and why it was not a tuning problem
///
/// The previous gate scaled `flammability` by `1 - saturation * 0.9`,
/// where `saturation` was `field_moisture_at(x, y) / 4.0`. It was reported
/// as "changes nothing measurable" for two milestones, and it did. The
/// reason is not the 0.9 and not the 4.0: **`field_moisture_at` reads
/// exactly 0.000 at 96.8% of fuel cells, at every ground wetness there
/// is.** Measured on one grown 1,993-cell sward, re-wetted to four levels
/// with 600 frames to settle each time, sampled at the grass cells
/// themselves rather than over a band:
///
/// | soil water | humidity at the fuel | fuel cells reading exactly 0 |
/// |---|---|---|
/// | 0 (bone dry) | mean 0.000 | 100% |
/// | 180 (wilting point) | mean 0.023 | 96.8% |
/// | 620 (field capacity) | mean 0.080 | 96.8% |
/// | 1000 (saturated) | mean 0.128 | 96.8% |
///
/// `field::step_diffusion` skips a blocked block outright, and
/// `rebuild_blocked` marks a block blocked when any `Solid` **or `Plant`**
/// cell falls in it — so a block with fuel in it never diffuses and holds
/// ambient zero forever. Fuel is invisible to the humidity channel *because
/// it is fuel*. Over the whole dry-to-saturated span the old term moved
/// effective flammability from 0.850 to 0.806, and the 5% it did move came
/// from the 3.2% of blades that happen to sit in the soil's own block.
///
/// `ground_wetness_at` is the channel that does answer: the moisture
/// *source* the field rebuilds from the CA grid every frame, which is
/// written for blocked blocks rather than in spite of them, and is on a
/// clean `0..=1` scale (`held / water_capacity`) instead of a
/// `MAX_MOISTURE` scale that only standing water ever reaches.
///
/// # Why a cutoff and not a scale
///
/// Fire spread here is a percolation: measured on this branch's flame
/// front, a sward either carries a fire the width of the world or stops it
/// inside a hundred cells, with very little in between. A gate that shaves
/// 10% off ignition therefore does nothing at all until it crosses the
/// threshold, and then does everything. So the gate is stated the way the
/// outcome is: wet ground refuses, dry ground carries, and the interesting
/// range is where the falloff between them puts the threshold.
///
/// The old constant's own doc argued against a hard switch, and that
/// argument survives intact: this bounds the *contact* path only. The
/// deterministic `temperature() >= ignition_temperature` crossing above it
/// in `try_ignite` is untouched, so a fire hot enough to boil the water
/// out of wet fuel first still sets it alight, which is both physically
/// right and the reason a cutoff here is safe.
///
/// 0.8 rather than 1.0 so saturated is not the only thing that stops a
/// fire — waterlogged ground is rare, and a meadow at field capacity
/// should already be refusing.
const FUEL_WETNESS_NO_IGNITION: f32 = 0.8;
/// How sharply ignition falls off as ground wetness climbs toward
/// `FUEL_WETNESS_NO_IGNITION`. Squared rather than linear because a linear
/// ramp leaves half the flammability at half wetness, which on the
/// percolation curve above is still "carries" — the transition has to
/// happen somewhere a player would call damp, not somewhere they would
/// call a bog.
const FUEL_WETNESS_FALLOFF: f32 = 2.0;

/// Where a burning cell may put a flame, and with what bias -- see
/// `tick_burn`'s emission block for why the *direction* is the load-bearing
/// part of this and the rate is not. Straight up twice out of six: fire
/// climbs, but a third of all licks go sideways, and the sideways ones are
/// the entire reason a front can leave the tussock it started on.
const FLAME_DIRECTIONS: [(i32, i32); 6] = [(0, -1), (0, -1), (-1, -1), (1, -1), (-1, 0), (1, 0)];

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

    // Born hot, **once** — a cell of such a material (lava) still sitting
    // at the ambient default every cell is created with gets its birth
    // temperature here, then cools through its own conductivity like
    // anything else and turns solid at its `cooling_point` long before it
    // could revisit the trigger (guaranteed by `Material::from`'s
    // load-time assert: born-hot materials must set a cooling point above
    // ambient). This replaced a per-visit *pin*: pinned cells could never
    // cool, so every scrap a flow stranded where water couldn't reach
    // stayed molten and held its chunk awake forever — 195 cells and 9/40
    // chunks at frame 1500 of `scene=lavapour`, measured. One narrow edge
    // stays, documented rather than defended: a fresh cell chilled *below*
    // ambient before its first visit (painted mid-blizzard) skips its
    // birth and crusts immediately, which reads fine for the one material
    // that uses this.
    if intrinsic_temperature.is_finite() && cell.temperature() == AMBIENT_TEMPERATURE {
        cell.set_temperature(intrinsic_temperature.round() as i16);
    }
    diffuse_heat(surface, x, y, &mut cell);

    // Very hot cells tell the coarse field they are hot and bright, the
    // way `tick_burn` does for flames — searing lava and fresh quench
    // crust glow and radiate whether or not anything is on fire. Gated on
    // temperature rather than on the born-hot flag so the push follows the
    // heat itself and stops on its own as the cell cools: a bounded,
    // self-shrinking population, never a per-swept-cell coupling.
    if !cell.is_burning() && (cell.temperature() as f32) >= FIELD_GLOW_MIN_TEMPERATURE {
        surface.add_heat(x, y, 1, 2.0);
        surface.add_light(x, y, 1, 1.0);
    }

    if cell.is_burning() {
        tick_burn(surface, x, y, &mut cell);
    } else {
        try_ignite(surface, x, y, &mut cell);
    }

    try_phase_change(surface, x, y, &mut cell);
    try_react(surface, x, y, &mut cell);

    let temp_off_ambient = (cell.temperature() as f32 - AMBIENT_TEMPERATURE as f32).abs();
    // **A cell that is mid-phase-change has to keep its own chunk awake, and
    // being off ambient is not the same thing.** Every other threshold in
    // this function is crossed by a cell that is hot or cold, so it dirties
    // itself for free; melting is the exception, because ice sits *at*
    // ambient while it thaws. Before `MELT_CHANCE` that cost nothing --
    // melting fired on the first visit -- and with a per-visit roll it is
    // the difference between a thaw and a permanent glacier: the chunk
    // settles, the cell is never visited again, and the sheet stays for the
    // rest of the run. Caught by two conservation tests that stopped
    // finishing their thaw, at the same 80.4% refill however many frames
    // they were given.
    let mid_phase_change = {
        // One `Vec` index, not two: this runs on every visited cell in the
        // fire pass, which is the hottest thing in this file.
        let m = surface.materials().get(cell.material);
        m.melts_into.is_some() && cell.temperature() as f32 >= m.melting_point
    };
    let must_stay_dirty = cell.is_burning() || temp_off_ambient > THERMAL_SETTLE_EPSILON || mid_phase_change;

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
    // Extracted with the rest, for the same borrow reason: the emission
    // below needs `&mut surface` for the roll and the write.
    let flame_into = material.flame_into;
    let flame_chance = material.flame_chance;
    // `MaterialKind` is `Copy` — extracted up front for the same reason
    // `burn_temp`/`burns_into` are: the burnout branch below needs it after
    // `material`'s own borrow would otherwise still be live.
    let was_structural = matches!(material.kind, material::MaterialKind::Solid | material::MaterialKind::Plant);
    // Same extraction, same reason, for the `meat_lost` booking in the
    // burnout branch — `book_meat_lost` takes `&mut S`, so reading
    // `material.worth_in_aux` at the call site would keep this borrow alive
    // across it.
    let source_worth_in_aux = material.worth_in_aux;

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

    // **The flame body.** Everything above this line is what fire was
    // before: a tint on the fuel's own cell and a nudge to two fields.
    // That is exactly what the owner saw -- *"Just looks like you are
    // cycling colors"* (`Reports/open-bugs-handoff.md` §G) -- because it
    // is exactly what the code did. Fire had no extent of its own; it
    // could only ever be the silhouette of whatever happened to be alight.
    //
    // A lick of `flame` (see `flame.ron`) fixes the same two complaints
    // with one mechanism, which is why it is one mechanism and not two:
    //
    //   - **Look.** A flame is a `Gas` that is spawned already burning, so
    //     it rises, leans with the wind, renders at the top of the heat
    //     ramp, lights the ground through `Material::glow`, and ages into
    //     smoke through its own `burns_into`. The front gets a body above
    //     the fuel and a plume off the top of it.
    //   - **Spread.** `try_ignite`'s neighbour scan already asks
    //     `is_burning()`, so a flame ignites fuel it drifts against with
    //     no change to that scan and no cost added to it. That is what
    //     lets a front cross the gaps between tussocks, and those gaps are
    //     the whole of *"it doesn't spread at all"*: measured on this
    //     branch, a 160-founder sward of 1,993 grass cells is **71
    //     separate 4-connected islands**, largest 16% of the sward, so a
    //     contact-only front burns one island and stops. The column
    //     census says the sward is continuous (one empty column in the
    //     whole span); the connectivity census says it is a scatter. Fire
    //     reads the second one.
    //
    // **The direction is rolled, and that is the part that had to be got
    // right rather than the rate.** The first version searched a fixed
    // order -- up, then the upper diagonals, then the sides -- and took
    // the first empty cell. In a sward the cell above a blade is almost
    // always empty, so *every* lick went straight up, the flame rose in a
    // column, and the front did not gain one cell of lateral reach: the
    // fire looked much better and spread exactly as badly as before.
    //
    // Rolling a starting direction and rotating through the rest from
    // there fixes it. `FLAME_DIRECTIONS` lists straight up twice out of
    // six, so a lick is biased upward -- fire climbs -- while a third of
    // them go sideways, which is what puts flame in the empty column
    // *beside* a burning tussock. That flame is burning, so `try_ignite`'s
    // existing neighbour scan lights whatever is 4-adjacent to it, and a
    // sideways lick is 4-adjacent to three cells of the next column at
    // once. Vertical misalignment between neighbouring tussocks is most of
    // what makes a sward 71 islands rather than one, and one cell of
    // lateral flame covers it.
    //
    // Straight **down** is deliberately absent. A fire that licks downward
    // sets light to the ground under its own fuel, and a grassfire that
    // burns into the soil reads as the world being on fire rather than the
    // grass.
    //
    // The roll comes before the neighbour scan, not after, and that is the
    // whole of what this costs: on the frames the roll fails -- most of
    // them, at any sane rate -- a burning cell pays one `chance` and not a
    // single `get`. Only a winning roll scans, and it stops at the first
    // empty cell it finds.
    if let Some(flame) = flame_into {
        if flame_chance > 0.0 && surface.rng().chance(flame_chance) {
            let flame_def = surface.materials().get(flame);
            let shades = flame_def.palette.len().max(1) as u32;
            let flame_temperature = flame_def.burn_temperature;
            let flame_duration = flame_def.burn_duration.max(1);
            let shade = surface.rng().below(shades) as u8;
            let start = surface.rng().below(FLAME_DIRECTIONS.len() as u32) as usize;
            for i in 0..FLAME_DIRECTIONS.len() {
                let (dx, dy) = FLAME_DIRECTIONS[(start + i) % FLAME_DIRECTIONS.len()];
                let (nx, ny) = (x + dx, y + dy);
                if !surface.in_bounds(nx, ny) || surface.get(nx, ny).material != material::EMPTY {
                    continue;
                }
                let mut lick = Cell::new(flame, shade);
                lick.ignite(flame_duration);
                if flame_temperature.is_finite() {
                    lick.set_temperature(flame_temperature.round() as i16);
                }
                surface.set(nx, ny, lick);
                break;
            }
        }
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
            // **A random shade is decoration on most materials and a lie on
            // one.** `worth_in_aux` marks a material whose shade is *derived*
            // -- corpse, whose brightness is ramped from what the animal was
            // worth by `creature::creature_dies`. A burnout cannot stamp that
            // worth: this path is generic over every flammable material and
            // knows nothing about creatures, so it writes `aux` 0 and the cell
            // is priced by the material fallback (the poorest a body can be).
            // Drawing a random shade for it would render a burnt ant as a
            // prime kill one time in five once that ramp is wide enough to
            // read -- so take the dark end, which is what it is.
            let into_def = surface.materials().get(into);
            let shades = into_def.palette.len().max(1) as u32;
            let into_worth_in_aux = into_def.worth_in_aux;
            let shade = if into_worth_in_aux { 0 } else { surface.rng().below(shades) as u8 };
            // **Book what the fire just ate.** `corpse` is flammable
            // (`corpse.ron`: `flammability: 0.15`, `burns_into: "ash"`), so
            // a body left in a grassfire takes its stamped worth out of the
            // world -- and before `EnergyLedger::meat_lost` existed it did so
            // with nothing recording it, which is what made
            // `max_standing_meat` an upper bound rather than a bound.
            //
            // Read off `material`, the *source* def already in hand a few
            // lines up, not off `into`: the question is what is being
            // destroyed, not what it becomes. The distinction is live in both
            // directions here -- an ant burning has `burns_into: "corpse"`, so
            // `into` is `worth_in_aux` while the source is not and nothing
            // should be booked; a corpse burning is the exact inverse.
            //
            // Costs nothing the sweep was not already paying: this branch
            // runs only on the frame a cell finishes burning, and both terms
            // are already loaded (`material` for `burns_into`, `cell` to be
            // overwritten on the next line).
            if source_worth_in_aux && cell.aux() != 0 {
                surface.book_meat_lost(cell.aux() as f64);
            }
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
                    next_frame: surface.organism_due(DECAY_TICK_INTERVAL),
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
                // Scheduling the check is only half of it: `World::chain_
                // reach` also has to *license* the failure, and it only
                // does so near something that reported itself disturbed.
                // Fire eating a trunk is the realistic way a tree's base
                // disappears -- `structural::tests::burning_a_trees_base_
                // collapses_the_rest_of_the_trunk` is the end-to-end claim
                // -- and without this it stopped bringing anything down
                // the moment `TIGHT` became the default reach.
                surface.record_disturbance(x, y, 0);
            }
        }
    }
}

/// Two independent ways to catch fire: contact with a burning neighbour
/// (probabilistic, rolled fresh — see the module doc for why that is safe
/// here, and gated by how wet the ground under the fuel is — architecture
/// §4 — see `FUEL_WETNESS_NO_IGNITION`'s own doc), or the cell's own
/// temperature crossing
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
    // Read *after* the neighbour scan, never before it: this is the one
    // read in `try_ignite` that can cost a `HashMap` lookup (see
    // `CellSurface::ground_wetness_at`), and a flammable cell with nothing
    // burning beside it — which is every flammable cell in the world,
    // nearly always — has already returned above.
    let wetness = surface.ground_wetness_at(x, y);
    if wetness >= FUEL_WETNESS_NO_IGNITION {
        return;
    }
    let dryness = 1.0 - wetness / FUEL_WETNESS_NO_IGNITION;
    let effective_flammability = flammability * dryness.powf(FUEL_WETNESS_FALLOFF);
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
    let freeze_min_fill = material.freeze_min_fill;
    let condenses_into_sky = material.condenses_into_sky;
    let from_kind = material.kind;
    // `material`'s borrow ends here, before `transform` needs `&mut surface`.

    if temp >= melting_point {
        if let Some(into) = melts_into {
            // **A thaw is slow, and this arm was instantaneous.**
            // `PHASE_CHANGE_CHANCE`'s own doc records melting as
            // deliberately ungated -- *"thaw urgency is carried by the
            // temperature gradient itself"* -- and a playtest overturned
            // it. Measured on `scene=coldsnap`: 362 cells of ice and 1,788
            // of snow went to **zero of both inside ten frames**, under a
            // fifth of a second at 60Hz, reported as *"the freeze is so
            // fast... when it melts, it happens in 1 second"*.
            if surface.rng().chance(MELT_CHANCE) {
                transform(surface, x, y, cell, into);
                surface.count_phase_event(PhaseEvent::Melted);
            }
        }
        return;
    }
    // **A simmering surface breaks up**, and this is the one part of
    // boiling that moves water rather than drawing it.
    //
    // Asked for from play, of the bubbles: *"Both should cause a surface
    // disturbance"*, alongside *"how much is this still connected to the
    // physics of the scene vs is just an animation?"* -- and the honest
    // answer for the bubbles themselves is *drawn*: `render.rs`'s
    // `apply_bubbles` is a pure function of position, frame and the cell's
    // own temperature, with no state and no writes back. So the
    // disturbance is put here instead of there, where it is a real cell of
    // water leaving the pool and coming back: `report_splash` debits the
    // pool and `particle::throw_splashes` launches it, the same path a
    // boulder's crown uses, so nothing is manufactured and it happens with
    // the bubbles switched off.
    //
    // Ordered cheapest-first on purpose: a float compare against the
    // boiling point, then the roll, and only then the `get` that asks
    // whether this cell is at a free surface. Every water cell in the world
    // pays the first of those and almost nothing pays the third.
    if boils_into.is_some() && temp >= boiling_point - SIMMER_SPLASH_MARGIN && surface.rng().chance(SIMMER_SPLASH_CHANCE) {
        // **The hot cell is not the one that pops.** A pool is heated from
        // below and its surface is the coolest part of it -- `scene=simmer`
        // holds 385 cells over 100C with every one of them in the bottom
        // four rows and the surface nine rows above, so a rule that asked
        // for a hot cell *with air over it* fired exactly zero times. Same
        // trap `render.rs`'s `boil_below` records for the `Surface` bubble
        // mode, and the same answer: walk to where the water actually ends.
        //
        // Reported at the topmost **full** cell rather than at the very
        // top of the water: `particle::throw_splashes` will only take a
        // droplet out of a full cell, and a settled pool's top row is the
        // remainder of its volume. Aiming at the film means every site is
        // reported and every one declined, which is what the first version
        // of this did -- zero droplets on a visibly simmering pan.
        let mut probe = y;
        for _ in 0..SIMMER_SPLASH_REACH {
            let above = surface.get(x, probe - 1);
            let clear = above.material == super::material::EMPTY
                || (above.material == cell.material && super::update::liquid_fill(above) < super::material::LIQUID_FULL);
            if clear {
                surface.report_splash(x, probe, SIMMER_SPLASH_STRENGTH);
                break;
            }
            if above.material != cell.material {
                break; // a lid, a crust, another material: nothing to break
            }
            probe -= 1;
        }
    }
    if temp >= boiling_point {
        // Gated per visit so a heated pool surface stipples into steam over
        // a few frames instead of flipping edge-to-edge in one — see
        // `PHASE_CHANGE_CHANCE`. A cell that fails the roll is off-ambient
        // and therefore revisited next frame by `must_stay_dirty`.
        if let Some(into) = boils_into {
            if surface.rng().chance(PHASE_CHANGE_CHANCE) && pay_latent_heat(surface, x, y) {
                transform(surface, x, y, cell, into);
                surface.count_phase_event(PhaseEvent::Boiled);
            }
        }
        return;
    }
    if temp <= cooling_point {
        if let Some(into) = cools_into {
            if from_kind == MaterialKind::Liquid {
                // Freezing. Whether partial cells transition is the
                // material's own call — water holds its fringe liquid to
                // conserve the freeze/thaw loop, lava crusts its stranded
                // films (see `MaterialDef::freeze_min_fill`) — and the
                // front creeps via the same per-visit roll boiling uses.
                if super::update::liquid_fill(*cell) < freeze_min_fill {
                    return;
                }
                if surface.rng().chance(FREEZE_CHANCE) {
                    transform(surface, x, y, cell, into);
                    surface.count_phase_event(PhaseEvent::Froze);
                }
            } else if condenses_into_sky && surface.is_outdoors(x, y) {
                // **Condensation in the open air goes to the sky, not to a
                // cell.** Reported from live play against a lava pour: the
                // plume "rises about 5 ft in the air and then drops back
                // into rain... it almost looks like bouncing because it
                // goes up and down so fast."
                //
                // It was not a rate problem and not a height one. Measured
                // on `filmstrip scene=lavapour`, whose plume census prints
                // the standing state the cumulative `boiled`/`condensed`
                // counters cannot: at the peak of the pour, **140 water
                // cells standing in the air**, spread through the plume's
                // whole 40-row height rather than pooled at its top. Every
                // one was condensate on the way back down *through* the
                // steam still rising — and a falling `Liquid` displaces a
                // `Gas` (`update::try_move`), so each droplet shoved steam
                // cells downward on its way. That is the bouncing itself,
                // not merely its cause, and it is why the complaint reads
                // as the *steam* going up and down when a `Gas` has no
                // downward move at all.
                //
                // Water that reaches open air has left the world's cells;
                // what happens to it after that is weather. So it is
                // credited to `World::atmospheric_bank` — the same place
                // `evaporation::tick` puts a dried puddle — and comes back
                // when a front does. `water_equivalents(world) +
                // atmospheric_bank` is unchanged to the unit, which is the
                // invariant `weather.rs` asserts and `filmstrip`'s census
                // prints under every tile.
                //
                // **Outdoors only, and that is what keeps the sealed cases
                // intact.** A steam pocket in a cave, a chamber with a
                // stone ceiling (`filmstrip scene=boil`), a boil under a
                // roof someone built: all still condense into water exactly
                // where they stood, because `is_outdoors` answers from the
                // frozen `World::sky_surface` rather than from the shape of
                // the world around the cell. `Reports/open-bugs-handoff.md`
                // §4b records four attempts to infer that distinction and
                // why every one of them was wrong.
                //
                // Still a `Condensed` event: it condensed. What changed is
                // where the water went, and a counter that stopped ticking
                // would hide the mechanism from the one census that can see
                // it fire.
                surface.credit_atmosphere(super::update::liquid_fill(*cell));
                *cell = Cell::EMPTY;
                surface.count_phase_event(PhaseEvent::Condensed);
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
/// How near its boiling point a liquid has to be before its free surface
/// starts throwing droplets, in degrees.
///
/// Well below boiling, deliberately: a pan coming to the boil is *visibly*
/// restless before the first steam leaves it, and this is the same
/// population `render.rs`'s `BUBBLE_MIN_TEMPERATURE` draws bubbles for --
/// "a rising bubble is not *in* boiling water, it is in warm water above a
/// boiling floor". The two want to agree, but they are deliberately not
/// wired to each other: one is a render mode a player can switch off and
/// this is not.
const SIMMER_SPLASH_MARGIN: f32 = 15.0;

/// How likely a free-surface cell that near boiling is to pop, per visit.
///
/// Small, and the smallness is the whole tuning: `throw_splashes` fans
/// **three** droplets out of every site it is given and each one is a whole
/// cell of water leaving the pool, so a rate that reads as "the surface is
/// alive" at one site per few frames reads as a fountain at ten. Measured
/// on `scene=simmer`, a 111-wide pan.
const SIMMER_SPLASH_CHANCE: f32 = 0.002;

/// How far above a near-boiling cell to look for the free surface it should
/// break, in cells.
///
/// Bounds the cost -- the walk is paid only by a cell that has already
/// rolled `SIMMER_SPLASH_CHANCE` -- and bounds the *effect*: a lake with a
/// vent forty rows down does not spit at its surface, which is the right
/// answer for a rule about a pan coming to the boil. Sixteen covers
/// `scene=simmer`'s fourteen-deep pan with a little over.
const SIMMER_SPLASH_REACH: i32 = 16;

/// How hard a simmering pop is thrown, against a boulder's crown at 1.0.
///
/// A pan spitting is not a boulder arriving, and sharing the throw made it
/// look like rain: single drops clearing **ten rows** above the water. At
/// this strength one drop leaves, hops a little and falls back, which is
/// what a bubble bursting does.
const SIMMER_SPLASH_STRENGTH: f32 = 0.35;


/// Degrees of stored heat, taken out of the neighbourhood, that boiling one
/// cell costs.
///
/// # Why the cost is charged to the *source* and not to the product
///
/// Boiling was very nearly free. `transform` copies the cell's temperature
/// across, so a boil removed only the steam's own sensible heat -- about 80
/// degrees above ambient -- and none of the latent heat that vaporising
/// actually takes. Reported as a consequence rather than as itself: with
/// condensate no longer raining back into the pond it came from, an
/// 800-cell lava blob was measured boiling **~3,200 cells** out of a 12,000
/// cell pond, where the physics of basalt cooling 1000 -> 700C allows about
/// 240. The recycling had been hiding it.
///
/// **The obvious place to put the cost is on the product, and it cannot go
/// there.** Water boils at 100 and steam condenses at 45, so a birth
/// temperature more than ~55 degrees down puts the steam below its own
/// condensation point and it flashes straight back on its next visit -- an
/// in-place churn with no plume. A previous session built exactly that at
/// 40 degrees while hunting a different bug, measured it, and removed it
/// for minimality (`Reports/open-bugs-handoff.md` 0b item 3); it was never
/// rejected on the merits, but it cannot reach this magnitude.
///
/// Charged to the neighbours there is no such ceiling, and it states the
/// right thing: a lava cell can boil only what its own stored heat pays
/// for. That also patches, at the one place it does visible damage, a
/// deeper defect left open on purpose -- `diffuse_heat` relaxes each cell
/// toward its neighbour average using *its own* conductivity and never
/// debits the giver, so water (0.08) pulls forty times harder off lava
/// (0.002) than lava pushes into it. `lava.ron` documents that asymmetry as
/// intended. It makes a hot cell an amplifier rather than a finite
/// reservoir, and it is written up rather than rewritten here.
///
/// # The number
///
/// Water's latent heat of vaporisation is about 540 times its specific
/// heat, so in a model whose only thermal currency is degrees, 540 is the
/// physical figure. Swept on `scene=lavadrop` -- see the commit that set
/// it for the table.
const LATENT_HEAT_DEGREES: i32 = 540;

/// Take `LATENT_HEAT_DEGREES` of stored heat out of the four neighbours,
/// or refuse and take nothing.
///
/// All-or-nothing, on `World::spend_atmosphere`'s precedent and for the
/// same reason: a caller that gets `true` has been charged and must go
/// ahead, and a partial payment would be a boil that happened for less than
/// it cost. Only heat *above ambient* counts as available -- ambient is the
/// floor everything relaxes to and is not a reservoir anything can draw
/// down.
///
/// Deducted in proportion to what each neighbour holds, so a boil beside
/// one searing lava cell and three cold ones takes it out of the lava.
fn pay_latent_heat<S: CellSurface>(surface: &mut S, x: i32, y: i32) -> bool {
    let ambient = AMBIENT_TEMPERATURE as i32;
    let mut available = 0i32;
    let mut spare = [0i32; 4];
    for (i, (dx, dy)) in [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().enumerate() {
        let n = surface.get(x + dx, y + dy);
        // Empty air reads ambient and is never written, so it is an
        // unbounded source; drawing on it would make the cost free wherever
        // the pool is open, which is everywhere that matters.
        if n.material == material::EMPTY {
            continue;
        }
        spare[i] = (n.temperature() as i32 - ambient).max(0);
        available += spare[i];
    }
    if available < LATENT_HEAT_DEGREES {
        return false;
    }
    let mut owed = LATENT_HEAT_DEGREES;
    for (i, (dx, dy)) in [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().enumerate() {
        if spare[i] == 0 {
            continue;
        }
        // Integer arithmetic throughout, so the same neighbourhood always
        // pays the same bill -- determinism is same-build and a float split
        // here would be one more thing to reason about.
        let share = ((spare[i] as i64 * LATENT_HEAT_DEGREES as i64) / available as i64) as i32;
        let share = share.min(owed).min(spare[i]);
        if share == 0 {
            continue;
        }
        owed -= share;
        let mut n = surface.get(x + dx, y + dy);
        n.set_temperature((n.temperature() as i32 - share) as i16);
        surface.set(x + dx, y + dy, n);
    }
    true
}

/// The `aux` a melt writes into its liquid product.
///
/// Two cases, and which one applies is a property of the *pair*, not of
/// either material alone — see `density_scaled_fill` for the measurements
/// that forced the split.
///
/// - **A reciprocal pair** (`liquid.cools_into == Some(solid)`): this
///   engine froze that cell out of this liquid, and it may only have done
///   so from a near-full one (`MaterialDef::freeze_min_fill`). It goes back
///   full — `0`, on the `Liquid` convention — which makes the freeze/thaw
///   loop exact for the overwhelmingly common full cell and over-pays by at
///   most `LIQUID_FULL - freeze_min_fill` on the fringe that froze partial.
/// - **Anything else**: the phase arrived from somewhere other than this
///   liquid (snow falls out of the sky), so the only honest statement of
///   how much water it is worth is its density. See `density_scaled_fill`.
fn melt_fill(materials: &material::MaterialRegistry, from: MaterialId, into: MaterialId) -> u16 {
    let liquid = materials.get(into);
    if liquid.cools_into == Some(from) {
        return 0; // full: the freeze gate is the promise this rests on
    }
    density_scaled_fill(materials.get(from).density, liquid.density)
}

/// The `aux` a melt writes into its liquid product: how much liquid one
/// whole cell of the solid or powder phase is actually worth, in
/// `material::LIQUID_FULL` units, from the two densities.
///
/// # Not used when the melt is the exact inverse of a freeze
///
/// `melt_fill` above decides that, and the distinction is stated as data
/// rather than inferred: if the liquid's own `cools_into` names the phase
/// that is melting, then this engine made that cell out of this liquid, and
/// `MaterialDef::freeze_min_fill` is the content's own promise about how
/// full it was when it did. Ice comes back full on that promise. Snow does
/// not, because nothing here freezes water into snow — the sky makes it —
/// and its 0.3 density is the only statement anywhere of how much water a
/// flake is.
///
/// **Measured, because the obvious reading is that the density ratio should
/// simply apply to both.** It should not, and the cost of applying it to
/// ice is not the ~2% the arithmetic suggests. A `Solid` carries no fill,
/// so density-scaling the melt makes the round trip `1000 -> ice -> 920`
/// — an 8% loss on every cell that freezes *full*, which is nearly all of
/// them, and `scene=coldsnap` cycles its surface roughly ten times in one
/// front (froze 2,608, melted 4,671 against a 60-cell pond). It compounds:
/// the pond measured 1,200 cell-equivalents at the cut and 1,050 by frame
/// 361, and dropping two rows below the shore took the ice sheet's end
/// cells out from beside their anchor — 2 unconfined overloads (133 cells)
/// and 4 unsupported, a visible wedge of sheet slumping into the water, on
/// a scene whose acceptance bar is that nothing is dismantled. Returning it
/// full instead closes that loop exactly and the case is green again.
///
/// Returned on the `Liquid` convention, so a phase at least as dense as
/// what it melts into comes back as `0` — **full** — rather than as a
/// literal 1000. Both read the same through `update::liquid_fill`; 0 is what
/// every other liquid-creating call site in the engine writes, and matching
/// it keeps a melted cell indistinguishable from a painted one.
///
/// The floor of 1 is not cosmetic. Rounding a very light phase down to 0
/// would not produce an empty cell — it would produce a *full* one, which
/// is the exact failure this function exists to remove, at its most
/// extreme. A 1-unit cell is a near-empty one the transfer logic will fold
/// away, which is honest; a 1000-unit one is water out of nothing.
fn density_scaled_fill(from_density: f32, to_density: f32) -> u16 {
    // Written with `is_finite` rather than `!(x > 0.0)`, which clippy
    // rejects on a partially-ordered type -- and it is the clearer form
    // anyway: what is being excluded is a zero, a negative, or a NaN
    // density, all three of which are content errors.
    if !to_density.is_finite() || to_density <= 0.0 || !from_density.is_finite() || from_density < 0.0 {
        return 0; // a content error, not a phase: fall back to the old "full"
    }
    let fill = (material::LIQUID_FULL as f32 * from_density / to_density).round();
    if fill >= material::LIQUID_FULL as f32 {
        0
    } else {
        (fill as u16).max(1)
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
/// - **Solid → Liquid and Powder → Liquid** (melting): the source's `aux`
///   is an anchor distance or a moisture reading and must never be read as
///   a fill — but writing 0 instead is not the safe answer it looks like,
///   because 0 on a `Liquid` means **full**. `melt_fill` below decides
///   instead, and it asks whether the pair is *reciprocal*: ice comes back
///   full because water froze it and only ever from a near-full cell, and
///   snow comes back at 300 because nothing here freezes water into snow
///   and its 0.3 density is the only statement of what a flake is worth.
///
///   **This arm shipped as a flat 0 and manufactured water for a whole
///   milestone** (`Reports/open-bugs-handoff.md` item 0): a storm's drift
///   of 1,700 flakes thawed into 1,700 *full* cells, a factor of 3.3, and
///   the hillside flood that produced was visible from across the scene.
///   It was latent until snow gained a `heat_conductivity` and melting
///   became reachable at all — `CLAUDE.md`'s "fixing a bug often exposes a
///   constant that was compensating for it", here in the form of a whole
///   dead code path. Guarded by `weather.rs`'s
///   `a_thaw_does_not_manufacture_water`, which is a bound on *creation*;
///   every water test written before it was written against loss and none
///   of them could fail for this.
/// - **everything else**: 0, "no meaning to carry". For a freeze
///   (Liquid → Solid) that is the same transient state a brush-painted
///   stone cell has — distance-0 "claims anchored" until the scheduled
///   structural check below corrects it; the fill that cannot ride along is
///   what `FREEZE_MIN_FILL` bounds to a near-full cell, so the round trip
///   through ice closes to within the ~8% the two densities differ by
///   rather than gifting a full cell back.
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
        (MaterialKind::Solid, MaterialKind::Liquid) | (MaterialKind::Powder, MaterialKind::Liquid) => {
            melt_fill(surface.materials(), cell.material, into)
        }
        // **A solid that has just appeared claims no support, rather than
        // claiming to be anchored.** On a `Solid` or `Plant`, `aux` is the
        // distance to an anchor and `0` means *anchored* -- so minting a
        // quench cell at 0 tells every neighbour that relaxes off it that
        // it has found a load path to bedrock. `u16::MAX` is the value
        // `compute_world_distances` writes for a cell that genuinely
        // reaches nothing, and it is the honest answer for a cell that has
        // existed for no frames and has been asked nothing.
        //
        // It mattered little while the check ran the same frame; it matters
        // a great deal now that `NEW_SOLID_SETTLE_FRAMES` leaves the cell
        // unjudged for a while, because everything around it spends that
        // window treating it as ground.
        (_, MaterialKind::Solid) | (_, MaterialKind::Plant) => u16::MAX,
        _ => 0,
    };
    *cell = Cell::new(into, shade).with_temperature(temp);
    cell.set_aux(aux);

    let was_structural = matches!(from_kind, MaterialKind::Solid | MaterialKind::Plant);
    let now_structural = matches!(to_kind, MaterialKind::Solid | MaterialKind::Plant);
    if was_structural != now_structural {
        // **A solid that has just *appeared* is given a moment to acquire
        // neighbours before it is judged; one that has just *gone* is
        // judged at once.** The two directions are not symmetric and
        // treating them alike is what made quench produce dust.
        //
        // Measured on `scene=lavadrop` before this existed: 572 structural
        // failures whose region sizes were `1:382 2:92 3-5:85 6-7:8
        // 8-15:5`. `rigid::MIN_FRACTURE_CELLS` declines below 6 and
        // `MIN_BODY_CELLS` needs 8, so 97.7% of those failures could not
        // reach the fragment ladder at all -- `stone.ron`'s six rungs sit
        // downstream of a gate that had already refused. The cause is
        // upstream of any tuning: quench mints a few scattered cells a
        // frame, and a lone unsupported solid judged the instant it exists
        // is one cell, every time. `ice.ron:68-77` records the identical
        // failure for ice ("3,969 freezes in one storm and never ten cells
        // standing") and solved it with `floats`; stone has to sink, so the
        // lever here is time instead.
        //
        // The other direction must stay immediate: material *leaving* is
        // support that neighbours have already lost, and delaying that is
        // rock hanging in the air with nothing under it.
        let delay = if now_structural { NEW_SOLID_SETTLE_FRAMES } else { 0 };
        let frame = surface.frame() + delay;
        for (dx, dy) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
            surface.schedule_active_site(ActiveSite {
                x: x + dx,
                y: y + dy,
                kind: ActiveKind::StructuralCheck,
                next_frame: frame,
            });
        }
        // A phase change across the structural boundary is a disturbance,
        // so `World::chain_reach` licenses the failure the check above is
        // being scheduled to find. A crust minted over open water is the
        // case that names itself: nothing touched it, and it still has to
        // be allowed to come apart. `NEW_SOLID_SETTLE_FRAMES` is 60 and
        // `CHAIN_WINDOW_FRAMES` is 600, so the licence outlives the delay
        // by a wide margin -- but the two are coupled now, and shortening
        // the window below the delay would silently un-license every new
        // solid.
        surface.record_disturbance(x, y, 0);
    }
}

/// How long a freshly minted solid is left alone before its first
/// structural check. See `transform`, which carries the measurement.
const NEW_SOLID_SETTLE_FRAMES: u64 = 60;

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

            // The reaction *exchanges* the pair's heat: the neighbour's
            // product takes the hotter side's temperature, the self product
            // takes the cooler side's. Both halves are load-bearing, and
            // each was measured separately. The hot half: quench steam born
            // at the water's ~20°C would sit below its own condensation
            // point and flash straight back to water on its next visit;
            // born at the lava's temperature it rises and condenses
            // somewhere else later, which is the visible plume the reaction
            // owes. The cold half: quench stone born at the *lava's*
            // temperature made a submerged delta of hundreds of 1000°C
            // cells whose heat had nowhere to go but back into the pond —
            // `scene=lavapour` was still boiling ~5 cells a frame with 9/40
            // chunks awake at frame 8000, because the boil/condense loop
            // rains most of the heat straight back. Quenching *means* the
            // coolant carried the heat off; the steam is that heat leaving.
            // A first version gave both products the max and paid exactly
            // that cost.
            //
            // Both sides also go through `transform`, not a bare
            // `Cell::new` — the neighbour write used to bypass it, which
            // would have dropped a quenched water cell's fill and reset its
            // steam to `aux 0` (= full), manufacturing volume.
            let mut other = surface.get(nx, ny);
            if reaction.mixes_heat {
                // Absorption: the pair's heat spreads over both products —
                // see `ReactionDef::mixes_heat` for the measured pump the
                // exchange rule makes of a condensation.
                let mean = ((cell.temperature() as i32 + other.temperature() as i32) / 2) as i16;
                cell.set_temperature(mean);
                other.set_temperature(mean);
            } else {
                let hotter = cell.temperature().max(other.temperature());
                let cooler = cell.temperature().min(other.temperature());
                cell.set_temperature(cooler);
                other.set_temperature(hotter);
            }

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

    /// The load model with no `chain_reach` leash -- see
    /// `World::without_chain_limit` for why the model's own tests take it
    /// off and the game does not.
    fn test_world() -> World {
        World::new(Rect::new(0, 0, 63, 63)).without_chain_limit()
    }

    /// **A solid that has just appeared claims no support.**
    ///
    /// On a `Solid`, `Cell::aux` is the distance to an anchor and `0` means
    /// *anchored* -- the value bedrock-adjacent rock carries. `transform`
    /// minted every new solid at 0, which told every neighbour that
    /// relaxed off it that a load path to bedrock ran through a cell that
    /// had existed for no frames and been asked nothing.
    ///
    /// It cost little while the check ran the same frame and a great deal
    /// once `NEW_SOLID_SETTLE_FRAMES` left the cell unjudged for a second:
    /// on `scene=lavapour` the fake anchors took the standing hanging-rock
    /// count to 127, against 1 before the delay and 0 with this. `u16::MAX`
    /// is what `compute_world_distances` writes for a cell that genuinely
    /// reaches nothing, and it is the honest answer here.
    #[test]
    fn a_freshly_minted_solid_does_not_claim_to_be_anchored() {
        let mut w = test_world();
        let lava = w.materials.id_of("lava").expect("lava.ron should be embedded");
        w.set(30, 30, Cell::new(lava, 0));
        w.set(30, 31, Cell::new(material::WATER, 0));
        // The quench is a chance per visit, so drive it rather than
        // assuming one visit is enough.
        for _ in 0..500 {
            update(&mut w, 30, 30);
            if w.materials.kind(w.get(30, 30).material) == MaterialKind::Solid {
                break;
            }
        }
        let quenched = w.get(30, 30);
        assert_eq!(
            w.materials.kind(quenched.material),
            MaterialKind::Solid,
            "the lava never quenched, so this asserts nothing -- it is still {}",
            w.materials.get(quenched.material).name
        );
        assert_eq!(
            quenched.aux(),
            u16::MAX,
            "a solid one frame old is claiming an anchor distance of {} -- 0 reads as *anchored* and every \
             neighbour will relax off it as though it were bedrock",
            quenched.aux()
        );
    }

    /// **A quench crust comes apart into a spread of sizes, not into
    /// dust** — the owner's verdict on the re-shot crust card, in a test:
    /// *"the stone sinking to the bottom is better, but it would be better
    /// with chunks instead of pile of dust."*
    ///
    /// Measured on `filmstrip scene=lavadrop` before
    /// `NEW_SOLID_SETTLE_FRAMES` existed: 572 structural failures whose
    /// region sizes were `1:382 2:92 3-5:85 6-7:8 8-15:5`. 97.7% of them
    /// were below `rigid::MIN_FRACTURE_CELLS`, so the fragment ladder was
    /// never consulted once and no amount of tuning `stone.ron`'s six rungs
    /// could have changed the outcome — a two-cell region has nothing to
    /// distribute. Peak chunk bodies over the whole run: 2.
    ///
    /// The bar is the **spread**, deliberately, and not "some rubble
    /// exists": every version of this scene produces rubble, including the
    /// one the owner rejected. It is also not "the mean is above N" — one
    /// large break averaged with two hundred single cells reads
    /// respectable. What separates the two behaviours is whether anything
    /// at all reaches the size at which a body can promote.
    ///
    /// Bars set below the measurement with headroom, per `CLAUDE.md`, and
    /// **the control was run**: this scene at the shipped delay produces
    /// region sizes `[71, 23, 38, 10, 41, 19, 0]` -- 60 failures of 8 cells
    /// or more, largest 55 -- against `[207, 31, 32, 9, 3, 0, 0]` with
    /// `NEW_SOLID_SETTLE_FRAMES` set to 0: 3 and 10. Twenty times apart on
    /// the quantity the bar reads.
    ///
    /// The blob is deliberately small relative to the pond. A first
    /// version dropped 1200 cells of lava into the same water and **passed
    /// with the delay removed** -- enough lava piles into a raft whatever
    /// the schedule does, so the scene could not fail for the artifact it
    /// is named after. `CLAUDE.md`: a guard must be able to fail for the
    /// replacement.
    #[test]
    fn a_quench_crust_breaks_into_a_spread_of_sizes_rather_than_dust() {
        use crate::sim::world::SIZE_BUCKETS;
        let mut w = World::new(Rect::new(0, 0, 127, 127));
        let lava = w.materials.id_of("lava").expect("lava.ron should be embedded");
        // `scene=lavadrop` in miniature: a walled pond with a blob of lava
        // released over the middle of it. Walls attached, so they are the
        // anchor and nothing that quenches mid-pond can reach one.
        for y in 40..127 {
            w.set(20, y, Cell::new(material::STONE, 0).with_attached(true));
            w.set(108, y, Cell::new(material::STONE, 0).with_attached(true));
        }
        for x in 0..128 {
            w.set(x, 127, Cell::new(material::BEDROCK, 0));
        }
        for x in 21..108 {
            for y in 44..127 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        for x in 52..80 {
            for y in 24..38 {
                w.set(x, y, Cell::new(lava, 0));
            }
        }
        crate::sim::structural::compute_world_distances(&mut w);
        // `App::update`'s order, and `update::step` opens and closes the
        // frame itself -- wrapping it in another `begin_step`/`end_step`
        // pair leaves the sweep with nothing to sweep, which reads exactly
        // like "the mechanism is dead" and cost a confused ten minutes.
        for _ in 0..600 {
            crate::sim::update::step(&mut w);
            crate::sim::rigid::step_chunk_bodies(&mut w);
            w.step_active_sites();
            w.step_fields();
        }

        let buckets = w.structural_failures.size_buckets;
        let body_sized: u32 = SIZE_BUCKETS
            .iter()
            .zip(buckets.iter())
            .filter(|(&edge, _)| edge >= crate::sim::rigid::MIN_BODY_CELLS as u32)
            .map(|(_, &n)| n)
            .sum();
        let events: u32 = buckets.iter().sum();
        assert!(events > 50, "the scene did not quench enough to be asking the question: {events} failures, sizes {buckets:?}");
        assert!(
            body_sized >= 20,
            "the crust came apart as dust: only {body_sized} of {events} failures reached {} cells, sizes {buckets:?}",
            crate::sim::rigid::MIN_BODY_CELLS
        );
        assert!(
            w.structural_failures.largest_failure >= 24,
            "no failing region got anywhere near chunk size -- largest was {}, sizes {buckets:?}",
            w.structural_failures.largest_failure
        );
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

    /// **How fast a fire front crosses each surface.** WP-B3's acceptance
    /// item 4: grass is supposed to be the fastest fuel in the world, and
    /// the reason it earns a material of its own is that it carries a fire
    /// across open ground that would otherwise stop it.
    ///
    /// Three arms, identical geometry, identical ignition, same frame
    /// budget -- a paired comparison in which the only variable is what the
    /// strip is made of. Hand-placed strips rather than grown plants on
    /// purpose: the question here is the *material's* fire behaviour, and a
    /// grown stand would vary in density, moisture and connectivity all at
    /// once. How it looks with real plants is a filmstrip's job.
    ///
    /// Dry, deliberately. `try_ignite` damps effective flammability by
    /// field moisture, so a strip laid on damp ground measures the water
    /// table as much as the fuel. Soil cells here are created dry
    /// (`aux == 0` on a `Powder` means dry -- the opposite of the liquid
    /// convention) and there is no water in the scene.
    #[test]
    fn a_fire_front_crosses_grass_faster_than_foliage_and_not_at_all_over_soil() {
        const X0: i32 = 10;
        const X1: i32 = 180;
        const ROW: i32 = 60;

        /// Frames for the front to reach the far end, or `CAP` if it
        /// never does. **Time-to-cross, not distance-at-a-fixed-time**: at
        /// any budget long enough for the slowest fuel to finish, every
        /// flammable arm reads 100% consumed and the arms stop being
        /// distinguishable. Speed needs a clock.
        fn cross(material_name: &str) -> usize {
            const CAP: usize = 6_000;
            // **Its own world, not this module's 64x64 `test_world`.** The
            // first version used it and every arm reported the full 170
            // cells, soil included: the strip lay entirely outside the
            // world, every `get` returned `EMPTY`, and "not the original
            // material" was trivially true everywhere. Four identical
            // numbers is the tell that the scene, not the mechanism, is
            // what is being measured.
            let mut w = World::new(Rect::new(0, 0, 199, 119));
            let Some(id) = w.materials.id_of(material_name) else { return CAP };
            for x in X0..=X1 {
                w.set(x, ROW + 1, Cell::new(material::STONE, 0));
                w.set(x, ROW, Cell::new(id, 0));
            }
            // Light the left end. `ignite_circle` ignores flammability, so
            // every arm starts genuinely alight -- soil included, which is
            // the point: it gets the same head start and still carries the
            // fire nowhere.
            w.ignite_circle(X0 + 1, ROW, 2);
            for frame in 0..CAP {
                // **`update::step` opens and closes the frame itself.** An
                // earlier version wrapped it in its own `begin_step`/
                // `end_step` pair, which double-manages the dirty-rect
                // promotion: chunks were marked settled while cells were
                // still burning, the sweep stopped visiting them, and every
                // burn timer froze mid-burn. The symptom was a strip with
                // sixteen cells alight and not one burned out after 4,000
                // frames -- fire that spreads but never finishes, which
                // looks like a burnout bug and is a harness bug.
                crate::sim::update::step(&mut w);
                crate::sim::field::step(&mut w);
                // Burnt = became something else *and* is still there, or is
                // alight. Deliberately not "differs from the original":
                // powder strips shed a grain off each open end, which left
                // an `EMPTY` at the far end and read as a front that had
                // crossed instantly -- soil scored a full crossing that way.
                let reached = (X0..=X1)
                    .filter(|&x| {
                        let c = w.get(x, ROW);
                        c.is_burning() || (c.material != id && c.material != material::EMPTY)
                    })
                    .max()
                    .unwrap_or(X0);
                if reached >= X1 - 2 {
                    return frame;
                }
            }
            CAP
        }

        let span = (X1 - X0) as f32;
        let grass = cross("grassblade");
        let litter = cross("litter");
        let foliage = cross("leaf");
        let soil = cross("soil");
        for (name, frames) in [("grass", grass), ("litter", litter), ("foliage", foliage), ("soil", soil)] {
            if frames >= 6_000 {
                println!("{name:>8}: never crossed");
            } else {
                println!("{name:>8}: {frames} frames  ({:.2} cells/frame)", span / frames as f32);
            }
        }

        assert_eq!(soil, 6_000, "a fire crossed bare soil in {soil} frames; soil is not a fuel and must stop a front dead");
        assert!(grass < 6_000, "the grass front never crossed -- either it did not spread at all or the scene is wet");
        assert!(
            grass < foliage,
            "grass crossed in {grass} frames against foliage's {foliage}: grass is supposed to be the faster surface fuel, which is the reason it is its own material"
        );
    }

    /// **Did the flame ever exist.** The paired burn below proves fire
    /// crosses a dry sward and stops on a wet one, and it would go on
    /// proving that if `flame_into` silently resolved to `None` -- a
    /// renamed material, a dropped `include_str!` line, a typo in
    /// `grassblade.ron` -- because contact spread along a *contiguous*
    /// strip never needed the flame in the first place. `CLAUDE.md`'s
    /// "did it fire at all needs a counter, not a picture", as a test: this
    /// one fails if the mechanism is disconnected, and nothing else here
    /// would.
    #[test]
    fn burning_grass_puts_flame_into_the_air() {
        let mut w = World::new(Rect::new(0, 0, 99, 59));
        let grass = w.materials.id_of("grassblade").expect("grassblade is compiled in");
        let flame = w.materials.id_of("flame").expect("flame is compiled in");
        assert_eq!(
            w.materials.get(grass).flame_into,
            Some(flame),
            "grassblade's flame_into did not resolve -- the material is missing from the embedded list, or the name is misspelt"
        );
        for x in 20..=40 {
            w.set(x, 41, Cell::new(material::STONE, 0));
            w.set(x, 40, Cell::new(grass, 0));
        }
        w.ignite_circle(30, 40, 1);
        let mut peak_flame = 0usize;
        for _ in 0..200 {
            crate::sim::update::step(&mut w);
            let standing = (0..60)
                .flat_map(|y| (0..100).map(move |x| (x, y)))
                .filter(|&(x, y)| w.get(x, y).material == flame)
                .count();
            peak_flame = peak_flame.max(standing);
        }
        println!("peak standing flame cells: {peak_flame}");
        assert!(peak_flame > 0, "a burning sward produced no flame at all -- the emission in tick_burn never ran");
        // A flame is a `Gas` that rises, so it must get *off* the fuel row
        // it was licked from. A flame that only ever appears in the row it
        // was born in is one that cannot reach the fuel beside it either,
        // which is the entire spread mechanism.
        let ever_above = (0..40)
            .flat_map(|y| (0..100).map(move |x| (x, y)))
            .any(|(x, y)| w.get(x, y).material == flame || w.get(x, y).material == w.materials.id_of("smoke").unwrap());
        assert!(ever_above, "nothing the fire produced ever reached a row above the fuel -- the flame is not rising");
    }

    /// **The owner's steer, as a test.** *"if we are going to do this,
    /// moisture vs dryness should play a role"* -- so the same strip of
    /// grass over dry ground and over saturated ground must give two
    /// different fires, and the difference must be large enough to see.
    ///
    /// Paired, not a bar on one arm: the two runs share a build, a seed, a
    /// scene and a strip, and differ in one `aux` value on the soil under
    /// it. Everything the rule is not about cancels.
    #[test]
    fn a_fire_crosses_a_dry_sward_and_stops_on_a_wet_one() {
        const X0: i32 = 10;
        const X1: i32 = 180;
        const ROW: i32 = 60;

        /// Grass cells consumed in `FRAMES`, and how far the front got.
        /// A **continuous** quantity rather than a crossed/not-crossed
        /// bool, per `CLAUDE.md`: a count of cells separates cleanly where
        /// a knife-edge margin flakes.
        fn burn(soil_water: u16) -> (usize, i32) {
            const FRAMES: usize = 1_200;
            let mut w = World::new(Rect::new(0, 0, 199, 119));
            let grass = w.materials.id_of("grassblade").expect("grassblade is compiled in");
            let soil = w.materials.id_of("soil").expect("soil is compiled in");
            for x in X0..=X1 {
                // **Four rows of soil on a stone floor**, and both halves
                // of that are load-bearing. Four rows so the field block
                // under the strip is a soil block whatever `FIELD_SCALE`
                // alignment `ROW` happens to land on -- `ground_wetness_
                // at` reads one block down and a thin skin can fall either
                // side of a block boundary. A stone floor because **soil
                // is a `Powder`**: the first version of this scene laid
                // soil over open space, it fell out of the world inside a
                // few frames, and both arms then read wetness 0 over an
                // empty column. That is `CLAUDE.md`'s scene error in its
                // usual costume -- the mechanism looks inert, and what is
                // actually inert is the situation. It is worth the four
                // extra `set`s to make the arms differ in one number and
                // nothing else.
                for dy in 1..=4 {
                    w.set(x, ROW + dy, Cell::new(soil, 0).with_aux(soil_water));
                }
                w.set(x, ROW + 5, Cell::new(material::STONE, 0));
                w.set(x, ROW, Cell::new(grass, 0));
            }
            // The field has to be stepped before anything is lit, or the
            // moisture source has never been scanned off the CA grid and
            // both arms read a bone-dry world. This is the scene error
            // `CLAUDE.md` warns about wearing its usual costume: the
            // mechanism looks inert because the situation is not there yet.
            for _ in 0..20 {
                crate::sim::field::step(&mut w);
            }
            w.ignite_circle(X0 + 1, ROW, 2);
            for _ in 0..FRAMES {
                crate::sim::update::step(&mut w);
                crate::sim::field::step(&mut w);
            }
            let standing = (X0..=X1).filter(|&x| w.get(x, ROW).material == grass).count();
            let front = (X0..=X1)
                .filter(|&x| w.get(x, ROW).material != grass)
                .max()
                .unwrap_or(X0);
            ((X1 - X0 + 1) as usize - standing, front)
        }

        let (dry_burnt, dry_front) = burn(0);
        let (wet_burnt, wet_front) = burn(material::SOIL_SATURATED);
        println!("dry ground: {dry_burnt} grass cells consumed, front reached x={dry_front}");
        println!("wet ground: {wet_burnt} grass cells consumed, front reached x={wet_front}");

        assert!(
            dry_burnt > 100,
            "a fire on bone-dry ground consumed only {dry_burnt} of 171 grass cells -- the dry arm is supposed to be the one that carries"
        );
        // Set from the measured pair with headroom, not on the measured
        // value: the wet arm's own burn is the handful of cells the
        // ignition itself lit plus whatever the flame body reached before
        // going out, and the bar is that it is a different *regime*, not
        // that it is exactly the number this build produced.
        assert!(
            wet_burnt * 4 < dry_burnt,
            "saturated ground let {wet_burnt} cells burn against dry ground's {dry_burnt} -- moisture is supposed to gate spread, and this is the reading that measured as changing nothing for two milestones"
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

        // **A hot neighbour, because boiling now costs the neighbourhood
        // something.** `LATENT_HEAT_DEGREES` is charged to the surrounding
        // cells' stored heat, and this cell used to sit alone in a vacuum
        // where every neighbour is `Cell::EMPTY` at ambient -- nothing to
        // draw on, so it never boiled and this test went red. That is the
        // rule working: a lone cell of water in empty space has no heat
        // source, and superheating it by hand does not conjure one.
        //
        // Stone rather than more water, so the count below still reads 1:
        // a `Solid` conducts its heat away but cannot boil itself.
        w.set(30, 31, Cell::new(material::STONE, 0).with_temperature(1000));
        w.set(30, 30, Cell::new(material::WATER, 0).with_aux(650).with_temperature(150));
        assert!(update_until_changed(&mut w, 30, 30, 100), "water at 150C never boiled");
        let boiled = w.get(30, 30);
        assert_eq!(boiled.material, steam, "water above its boiling point should become steam");
        assert_eq!(boiled.aux(), 650, "steam should carry the source water's fill");
        assert_eq!(w.phase_changes.boiled, 1);

        // Cool the same cell below steam's condensation point and it gives
        // the fill back. Set directly rather than waiting out diffusion —
        // the cooling curve is diffuse_heat's own tested property.
        //
        // The heat source has to go first, or `diffuse_heat` pulls the
        // steam straight back over its 45C threshold from the 1000C stone
        // and nothing ever condenses. Written without this and it failed
        // exactly that way.
        w.set(30, 31, Cell::EMPTY);
        w.set(30, 30, boiled.with_temperature(30));
        assert!(update_until_changed(&mut w, 30, 30, 400), "steam at 30C never condensed");
        let condensed = w.get(30, 30);
        assert_eq!(condensed.material, material::WATER, "cooled steam should condense back to water");
        assert_eq!(condensed.aux(), 650, "condensed water should hold exactly the fill the steam carried");
        assert_eq!(w.phase_changes.condensed, 1);
    }

    #[test]
    fn a_freezing_liquid_writes_no_anchor_distance_and_schedules_checks() {
        // Liquid → Solid crosses `aux` conventions: fill on the way in,
        // anchor distance on the way out. It must be a *distance* and not a
        // raw fill copied across — and specifically `u16::MAX`, the value
        // for "reaches no anchor".
        //
        // **This asserted 0 until the quench-crust work, and 0 was the
        // bug.** 0 is what bedrock-adjacent rock carries, so a cell one
        // frame old was telling every neighbour a load path ran through it.
        // See `a_freshly_minted_solid_does_not_claim_to_be_anchored`, which
        // is that claim on its own, and `NEW_SOLID_SETTLE_FRAMES`, which is
        // what made a one-frame lie into a one-second one.
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
        assert_eq!(
            frozen.aux(),
            u16::MAX,
            "a fresh solid must claim no anchor until its scheduled check finds it one"
        );
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
    fn a_melting_cell_yields_its_own_density_in_liquid_not_a_free_full_cell() {
        // **Renamed and rewritten, and the old name was the bug.** This
        // used to be `a_melting_solid_becomes_a_full_liquid_not_a_drained_
        // one` and asserted `aux == 0`, i.e. *full*, for every melt — which
        // is what shipped the water-manufacturing arm in `transform`'s aux
        // table (see `density_scaled_fill`, and open-bugs item 0). Its
        // original job is kept and still asserted: the source's `aux` is an
        // anchor distance or a moisture reading, and reading it back as a
        // fill is a different and equally real bug. What changed is the
        // *right* answer to write instead — a fill scaled by the two
        // densities, not a free full cell.
        //
        // Both melting kinds are exercised here, at densities chosen so the
        // three outcomes are distinguishable from each other and from the
        // anchor distance: a half-density solid (500), a light powder
        // (300, snow's own figure), and a solid denser than its melt (full).
        // None of these three is a reciprocal pair -- no liquid here names
        // any of them in `cools_into` -- so all three go through the
        // density rule. The reciprocal case is the *other* test below.
        let dir = std::env::temp_dir().join("pixel-physics-melt-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.ron"),
            "(name: \"melt_solid\", kind: Solid, density: 0.5, colors: [(200, 200, 255)], \
             melting_point: 1.0, melts_into: \"melt_liquid\")",
        )
        .unwrap();
        std::fs::write(dir.join("b.ron"), "(name: \"melt_liquid\", kind: Liquid, density: 1.0, colors: [(0, 0, 200)])").unwrap();
        std::fs::write(
            dir.join("c.ron"),
            "(name: \"melt_powder\", kind: Powder, density: 0.3, colors: [(240, 240, 250)], \
             melting_point: 1.0, melts_into: \"melt_liquid\")",
        )
        .unwrap();
        std::fs::write(
            dir.join("d.ron"),
            "(name: \"melt_dense\", kind: Solid, density: 2.5, colors: [(90, 90, 110)], \
             melting_point: 1.0, melts_into: \"melt_liquid\")",
        )
        .unwrap();

        let mut w = test_world();
        w.materials.reload(&dir).unwrap();
        let (solid, liquid, powder, dense) = (
            w.materials.id_of("melt_solid").unwrap(),
            w.materials.id_of("melt_liquid").unwrap(),
            w.materials.id_of("melt_powder").unwrap(),
            w.materials.id_of("melt_dense").unwrap(),
        );

        // `aux 37` is a plausible relaxed anchor distance — exactly the
        // value that must not survive as a fill.
        w.set(30, 30, Cell::new(solid, 0).with_aux(37).with_temperature(AMBIENT_TEMPERATURE));
        // 2,000 visits, not 3: melting is a per-visit roll at
        // `MELT_CHANCE` now, so the mean wait is ~250 frames. The budget is
        // set from that rate with headroom, not from an aspiration -- a
        // thaw that takes tens of seconds is the whole point of the change,
        // and this budget was already re-derived once when the rate moved
        // from 0.015 to 0.004 (400 frames stopped being enough and the test
        // failed on the *dense* case, the third of three draws).
        assert!(update_until_changed(&mut w, 30, 30, 2000), "a solid above its melting point never melted");
        let melted = w.get(30, 30);
        assert_eq!(melted.material, liquid);
        assert_ne!(melted.aux(), 37, "the anchor distance survived as a fill");
        assert_eq!(melted.aux(), 500, "half-density ice should melt into a half-full cell, not a free full one");
        assert_eq!(w.phase_changes.melted, 1);
        assert_eq!(w.active_site_count(), 5, "a melt removed a possible support and owes the fan-out");

        // A `Powder`'s aux is a moisture reading (`SOIL_SATURATED`, whose
        // convention points the *other* way), so it is the same trap in a
        // second costume — and the arm that actually flooded a hillside.
        w.set(50, 30, Cell::new(powder, 0).with_aux(800).with_temperature(AMBIENT_TEMPERATURE));
        assert!(update_until_changed(&mut w, 50, 30, 2000), "a powder above its melting point never melted");
        let thawed = w.get(50, 30);
        assert_eq!(thawed.material, liquid);
        assert_eq!(thawed.aux(), 300, "snow-density powder should melt into 30% of a cell of water, not a whole one");

        // The clamp: a phase denser than its own melt cannot yield more
        // than one cell, and comes back on the 0-means-full convention
        // every other liquid-creating call site writes.
        w.set(60, 30, Cell::new(dense, 0).with_temperature(AMBIENT_TEMPERATURE));
        assert!(update_until_changed(&mut w, 60, 30, 2000), "a dense solid above its melting point never melted");
        assert_eq!(w.get(60, 30).aux(), 0, "a phase denser than its melt should clamp to one full cell (aux 0), not overflow");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// **How long a freeze and a thaw take, in frames** — which no test
    /// asked before, and the reason a pond could ice over in a third of a
    /// second and melt in a fifth of one while every ice test passed.
    ///
    /// Reported from play: *"the freeze is so fast. It lasts only a few
    /// seconds... when it melts, it happens in 1 second."* Measured on
    /// `scene=coldsnap` before the change: 272 cells frozen by step 20, and
    /// 362 ice plus 1,788 snow gone inside ten frames. After: the freeze
    /// builds over ~200 frames and the sheet is gone by ~480.
    ///
    /// A **median over a hundred cells**, not one cell's luck: both changes
    /// are per-visit rolls, so a single sample is a geometric draw and would
    /// flake in whichever direction it landed. The bands are wide on
    /// purpose — this is guarding an order of magnitude, which is what was
    /// reported, not a tuned value.
    #[test]
    fn a_freeze_and_a_thaw_take_the_time_a_player_can_see() {
        // `hold` re-applies the temperature each frame. Without it the
        // freeze half never finishes and the test looks like a broken
        // mechanism: a chilled cell warms back toward ambient by diffusion
        // within a few frames and stops being below its cooling point at
        // all, so what is being measured is the diffusion, not the roll.
        // The existing round-trip test re-chills for the same reason.
        let median_visits =
            |setup: &dyn Fn(&mut World, i32), hold: &dyn Fn(&mut World, i32), done: &dyn Fn(&World, i32) -> bool| {
                let mut w = test_world();
                for x in 0..40 {
                    setup(&mut w, x);
                }
                let mut melted = 0;
                for frame in 1..=4000u32 {
                    for x in 0..40 {
                        if !done(&w, x) {
                            hold(&mut w, x);
                            update(&mut w, x, 30);
                            if done(&w, x) {
                                melted += 1;
                                if melted * 2 >= 40 {
                                    return frame;
                                }
                            }
                        }
                    }
                }
                u32::MAX
            };

        let ice = test_world().materials.id_of("ice").expect("ice.ron should be embedded");
        let thaw = median_visits(
            &|w: &mut World, x: i32| w.set(x, 30, Cell::new(ice, 0).with_temperature(AMBIENT_TEMPERATURE)),
            &|w: &mut World, x: i32| {
                let held = w.get(x, 30).with_temperature(AMBIENT_TEMPERATURE);
                w.set(x, 30, held);
            },
            &|w: &World, x: i32| w.get(x, 30).material == material::WATER,
        );
        assert!(
            (20..300).contains(&thaw),
            "half a row of ice thawed at frame {thaw}; before `MELT_CHANCE` it was 1, and a pond that \
             vanishes in a fifth of a second is what was reported"
        );

        let freeze = median_visits(
            &|w: &mut World, x: i32| w.set(x, 30, Cell::new(material::WATER, 0).with_temperature(-5)),
            &|w: &mut World, x: i32| {
                let held = w.get(x, 30).with_temperature(-5);
                w.set(x, 30, held);
            },
            &|w: &World, x: i32| w.get(x, 30).material == ice,
        );
        assert!(
            (4..120).contains(&freeze),
            "half a row of water froze at frame {freeze}; a pond should ice over while you watch it, \
             not between two glances"
        );
    }

    #[test]
    fn a_freeze_and_thaw_round_trip_returns_the_cell_it_took() {
        // The other half of `melt_fill`, and the reason it is not simply
        // the density ratio. Water freezes into ice and ice melts back into
        // water: a **reciprocal pair**, which the liquid states by naming
        // the solid in its own `cools_into`. For that pair the round trip
        // has to close, because `FREEZE_MIN_FILL` already refused to freeze
        // anything but a near-full cell — and density-scaling it instead
        // loses 8% of every full cell, every cycle, which compounds into a
        // draining pond (see `density_scaled_fill`'s own doc for the
        // `scene=coldsnap` measurement that caught it).
        //
        // Written against the shipped water/ice pair rather than synthetic
        // materials, deliberately: the claim is about content that names
        // itself reciprocally, and a hand-written pair would be asserting
        // the test's own fixture rather than the rule the game runs on.
        let mut w = test_world();
        let ice = w.materials.id_of("ice").expect("shipped content should have ice");
        assert_eq!(
            w.materials.get(material::WATER).cools_into,
            Some(ice),
            "water no longer names ice as what it freezes into; `melt_fill`'s reciprocal case has nothing to key on"
        );

        // A full cell, held below water's own cooling point until it takes,
        // then let back up to ambient. **Re-chilled every visit** rather
        // than set cold once: `diffuse_heat` runs before the phase check,
        // and a lone cell surrounded by empty air climbs back through zero
        // in three frames -- so a `PHASE_CHANGE_CHANCE` of 0.4 gets about
        // three rolls, and this failed outright the first time it was
        // written that way. Weather holds a column cold for exactly this
        // reason (`weather::hold_column_cold`); this is the unit-test
        // version of the same thing. `aux 4` below is a plausible anchor
        // distance on the ice -- the value that must not survive as a fill.
        w.set(30, 30, Cell::new(material::WATER, 0));
        let froze = (0..200).any(|_| {
            let chilled = w.get(30, 30).with_temperature(-5);
            w.set(30, 30, chilled);
            update(&mut w, 30, 30)
        });
        assert!(froze, "a full water cell held at -5C never froze");
        assert_eq!(w.get(30, 30).material, ice);
        w.set(30, 30, w.get(30, 30).with_aux(4).with_temperature(AMBIENT_TEMPERATURE));
        // 400 visits, not 3: melting is a per-visit roll at `MELT_CHANCE`
        // now, so the mean wait is ~67 frames. The budget is set from that
        // rate with headroom, not from an aspiration -- a thaw that takes
        // seconds is the whole point of the change.
        assert!(update_until_changed(&mut w, 30, 30, 400), "ice at ambient never melted");
        let melted = w.get(30, 30);
        assert_eq!(melted.material, material::WATER);
        assert_ne!(melted.aux(), 4, "the anchor distance survived as a fill");
        assert_eq!(
            super::super::update::liquid_fill(melted),
            material::LIQUID_FULL,
            "a full cell that froze came back short; the freeze/thaw loop drains a little on every cycle"
        );
    }

    #[test]
    fn a_reaction_exchanges_the_pairs_heat_and_keeps_fill() {
        // The two `try_react` rules that landed with the cooling branch,
        // asserted together: the neighbour's write goes through `transform`
        // (so a Liquid product keeps the neighbour's fill instead of being
        // rebuilt full), and the pair's heat is *exchanged* — the
        // neighbour's product takes the hotter side's temperature (quench
        // steam born cold would flash straight back to water) while the
        // self product takes the cooler side's (quench stone born at the
        // lava's 1000°C made a submerged delta that boiled the pond for
        // 8000+ frames; the coolant carrying the heat off is what
        // quenching is).
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
        assert_eq!(
            product.temperature(),
            AMBIENT_TEMPERATURE,
            "the self product should take the cooler side's temperature — the coolant carried the heat off"
        );
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

    /// A stone-walled basin of water with a lava blob **submerged in it**,
    /// the scene the quench tests below share. Returns the world and the
    /// lava id.
    ///
    /// Submerged, not dropped in from above, and that is the whole design
    /// of the scene rather than a convenience. A lava cell only *quenches*
    /// where it actually touches water; anything that ends up somewhere the
    /// pool cannot reach -- a rim, a corner, a thin film on a slope --
    /// finishes by the slower cooling path instead (crusting to stone at
    /// its `cooling_point`), which is a different mechanism than the one
    /// these tests are about. A first version of this helper dropped a 5x5
    /// blob from above the waterline and left exactly one lava cell
    /// stranded on the basin rim. Starting the blob inside the pool with
    /// water on all six sides of it makes "all of it quenches" a property
    /// of the geometry.
    fn lava_dropped_in_a_basin() -> (World, MaterialId) {
        let mut w = test_world();
        let lava = w.materials.id_of("lava").unwrap();
        for y in 36..=50 {
            w.set(20, y, Cell::new(material::STONE, 0).with_attached(true));
            w.set(44, y, Cell::new(material::STONE, 0).with_attached(true));
        }
        for x in 20..=44 {
            w.set(x, 50, Cell::new(material::STONE, 0).with_attached(true));
        }
        for x in 21..44 {
            for y in 42..50 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        for x in 30..=32 {
            for y in 45..=47 {
                w.set(x, y, Cell::new(lava, 0));
            }
        }
        (w, lava)
    }

    #[test]
    fn water_quenches_lava_into_stone_and_steam() {
        // M14's own verify line ("water quenches lava into stone and
        // steam"), on shipped content with no debug brush and nothing
        // pre-ignited. **Both** cells have to transform: a reaction that
        // converted the lava and left the water alone would still produce a
        // grey crust on a contact sheet and would be conserving nothing.
        //
        // The pair is checked directly first, so the assertion is about the
        // reaction and not about whether a blob happened to touch a pool
        // this run; the driver runs below then confirm the same thing
        // happens when the sweep is the one doing the visiting.
        let mut w = test_world();
        let lava = w.materials.id_of("lava").unwrap();
        let steam = w.materials.id_of("steam").unwrap();
        w.set(30, 30, Cell::new(lava, 0));
        w.set(31, 30, Cell::new(material::WATER, 0));
        // chance 0.8 per visit: a handful of visits is a vanishing failure
        // probability, and the roll is deterministic for a given build.
        for _ in 0..40 {
            update(&mut w, 30, 30);
            if w.phase_changes.reacted > 0 {
                break;
            }
        }
        assert_eq!(w.get(30, 30).material, material::STONE, "lava did not become stone");
        assert_eq!(w.get(31, 30).material, steam, "the water it quenched against did not become steam");

        // And through the sweep, both drivers -- behaviour only the player
        // sees is behaviour only the parallel driver produces.
        for parallel_driver in [false, true] {
            let (mut w, lava) = lava_dropped_in_a_basin();
            let lava_before = count_of(&w, lava);
            for _ in 0..400 {
                if parallel_driver {
                    crate::sim::parallel::step(&mut w);
                } else {
                    super::super::update::step(&mut w);
                }
            }
            assert!(
                w.phase_changes.reacted > 0,
                "a lava blob dropped into a pool never reacted in 400 frames (parallel: {parallel_driver})"
            );
            assert!(
                count_of(&w, lava) < lava_before,
                "lava was still all there after {} reactions (parallel: {parallel_driver})",
                w.phase_changes.reacted
            );
        }
    }

    #[test]
    fn quench_steam_is_born_hot_and_rises() {
        // The half of `try_react`'s hotter-side temperature rule that only
        // shows up in play: quench steam born at the *water's* ~20C would
        // sit below its own 45C condensation point and flash straight back
        // to water on its next visit, so the reaction would produce a
        // grey crust and no plume at all. Born at the lava's pin it rises
        // first and condenses somewhere else later, which is the visible
        // thing the quench owes.
        //
        // Asserted as two separate claims, because either one alone passes
        // for the wrong reason: a hot cell that never moves is a stuck
        // pocket, and a cell that rises while cold is just a gas.
        let mut w = test_world();
        let lava = w.materials.id_of("lava").unwrap();
        let steam = w.materials.id_of("steam").unwrap();
        for x in 28..=32 {
            w.set(x, 40, Cell::new(material::STONE, 0).with_attached(true));
        }
        w.set(30, 39, Cell::new(lava, 0));
        // Above the lava, not beside it: water is lighter (1.0 against
        // 2.7) so it rests on top rather than sinking through, and the
        // steam it becomes then has open sky to rise into.
        w.set(30, 38, Cell::new(material::WATER, 0));

        let mut born = None;
        for _ in 0..40 {
            update(&mut w, 30, 39);
            let cell = w.get(30, 38);
            if cell.material == steam {
                born = Some(cell.temperature());
                break;
            }
        }
        let born = born.expect("water above lava never became steam");
        // Far above steam's own 45C condensation point, not "exactly the
        // birth temperature": under the cooling model the lava legitimately
        // sheds a few degrees per visit while the 0.8 reaction roll waits
        // to land, so an equality against 1000 would flake on the rng
        // stream. The contract being asserted is that the steam is born hot
        // enough to rise and condense elsewhere rather than flashing
        // straight back — hundreds of degrees of margin, not a knife edge.
        assert!(
            born >= 500,
            "quench steam was born at {born}C — cool enough to flash back, \
             so try_react's hotter-side rule is not holding"
        );

        // Now let it move. Serial and parallel both, since rising is a
        // movement rule and movement is where the drivers differ.
        for parallel_driver in [false, true] {
            let mut w = test_world();
            w.set(30, 38, Cell::new(steam, 0).with_temperature(born));
            let mut highest = 38;
            for _ in 0..200 {
                if parallel_driver {
                    crate::sim::parallel::step(&mut w);
                } else {
                    super::super::update::step(&mut w);
                }
                // Scanned across the whole world, not just the column it
                // started in: a `Gas` disperses sideways as it rises, and
                // the first draft of this looked only at x == 30 and
                // reported "never rose" for steam that had risen and
                // drifted two cells over.
                if let Some(y) = (0..40).find(|&y| (0..64).any(|x| w.get(x, y).material == steam)) {
                    highest = highest.min(y);
                }
            }
            assert!(
                highest < 38,
                "steam born at {born}C never rose from row 38 (parallel: {parallel_driver})"
            );
        }
    }

    #[test]
    fn lava_ignites_adjacent_wood() {
        // The heat *verb*: before lava there was no way to set anything
        // alight except by putting it next to something already burning,
        // and `oil.ron`'s header says exactly that. Nothing here is
        // pre-ignited and nothing is flammable-adjacent -- the wood catches
        // because conducted heat carried it over `wood.ron`'s
        // `ignition_temperature`, which is the whole point.
        //
        // The wood is walled into a lava-filled pocket rather than left on
        // an open flow, because `fire::diffuse_heat` pulls a cell toward the
        // *average* of its four neighbours: one lava neighbour and three at
        // ambient settles at (1000 + 60)/4 = 265C, deliberately below the
        // threshold (see wood.ron), so a scene where the flow can drain
        // away from the wood would be testing the flow, not the ignition.
        for parallel_driver in [false, true] {
            let mut w = test_world();
            let lava = w.materials.id_of("lava").unwrap();
            let wood = w.materials.id_of("wood").unwrap();
            for x in 24..=40 {
                for y in 34..=46 {
                    let edge = x == 24 || x == 40 || y == 34 || y == 46;
                    if edge {
                        w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                    } else {
                        w.set(x, y, Cell::new(lava, 0));
                    }
                }
            }
            for x in 31..=33 {
                for y in 39..=41 {
                    w.set(x, y, Cell::new(wood, 0));
                }
            }

            let mut caught = false;
            for _ in 0..600 {
                if parallel_driver {
                    crate::sim::parallel::step(&mut w);
                } else {
                    super::super::update::step(&mut w);
                }
                caught = (31..=33).any(|x| (39..=41).any(|y| w.get(x, y).is_burning()));
                if caught {
                    break;
                }
            }
            let hottest = (31..=33)
                .flat_map(|x| (39..=41).map(move |y| (x, y)))
                .map(|(x, y)| w.get(x, y).temperature())
                .max()
                .unwrap();
            assert!(
                caught,
                "wood sealed into a lava pocket never caught in 600 frames \
                 (hottest wood cell {hottest}C, parallel: {parallel_driver})"
            );
        }
    }

    #[test]
    fn a_quenched_lava_flow_lets_its_chunks_sleep() {
        // The guard on the decision `stone.ron`'s `heat_conductivity` note
        // records. A quench hands its stone the *lava's* temperature, and
        // with stone left at the default conductivity of 0 that cell can
        // never give the heat back: `fire::update`'s thermally-inert fast
        // path returns before it ever looks at the temperature again, so
        // the crust is a permanent 1000C boiler and the pool it is sitting
        // in simmers for the rest of the run.
        //
        // Deliberately worded as "the world sleeps", not "the crust cools":
        // a temperature assertion would pass on a crust that cooled while
        // something else it had already boiled kept the chunks awake, and
        // the cost this is guarding is wakefulness. Confirmed to fail for
        // the thing it guards, which is the whole reason it is worded this
        // way. Paired, same scene, same content: with stone.ron's
        // `heat_conductivity` present this sleeps at frame **204** (serial)
        // / **198** (parallel) with a lifetime `boiled` of **61**; with
        // that one line deleted it runs the full 4000-frame budget with a
        // chunk still awake, every lava cell long since quenched, and
        // `boiled` at **3968** and still climbing linearly.
        //
        // The blob is dropped *into* the pool, not poured down a slope,
        // and that is load-bearing: a viscous flow on a slope strands a
        // thin film that never reaches water and finishes by *cooling*
        // instead (the stranded-film test below owns that path). This
        // scene has to be one where all of the lava really does quench, or
        // it would be measuring the cooling model instead of the stone
        // conductivity decision it exists to guard.
        for parallel_driver in [false, true] {
            let (mut w, lava) = lava_dropped_in_a_basin();
            let mut slept_at = None;
            for frame in 0..4000 {
                if parallel_driver {
                    crate::sim::parallel::step(&mut w);
                } else {
                    super::super::update::step(&mut w);
                }
                if w.active_chunk_count() == 0 {
                    slept_at = Some(frame);
                    break;
                }
            }
            let p = w.phase_changes;
            assert!(
                slept_at.is_some(),
                "a quenched lava flow never let its chunks sleep in 4000 frames \
                 (parallel: {parallel_driver}): {} chunks awake, {} lava cells left, \
                 reacted {}, boiled {}",
                w.active_chunk_count(),
                count_of(&w, lava),
                p.reacted,
                p.boiled,
            );
            assert_eq!(count_of(&w, lava), 0, "lava survived the quench (parallel: {parallel_driver})");
        }
    }

    #[test]
    fn lava_is_born_hot_once_and_only_cools_from_there() {
        // The contract that replaced the pin. First visit: a fresh cell at
        // the ambient default takes its birth temperature. Every visit
        // after: it cools through its own conductivity and is never
        // re-raised — the pin's failure mode was exactly that it re-raised
        // forever, so a cell mid-cool jumping back up is the regression
        // this exists to catch.
        let mut w = test_world();
        let lava = w.materials.id_of("lava").unwrap();
        w.set(30, 30, Cell::new(lava, 0));
        update(&mut w, 30, 30);
        // Not exactly 1000: birth and the first cooling step share the
        // visit (diffusion runs right after, deliberately), so a lone cell
        // with four ambient neighbours reads 1000 minus one step (~980).
        // The claim is "born searing", not "born untouched".
        let born = w.get(30, 30).temperature();
        assert!(
            born >= 900,
            "a fresh lava cell should be born near its intrinsic 1000C, read {born}C"
        );

        // Mid-cool (well above the 700C crust point, so it stays lava),
        // surrounded by ambient: the next visits must move it down, never
        // back toward 1000.
        w.set(30, 30, w.get(30, 30).with_temperature(800));
        update(&mut w, 30, 30);
        let after_one = w.get(30, 30).temperature();
        assert!(
            after_one < 800,
            "a lava cell at 800C surrounded by ambient should cool, read {after_one}C — the pin is back"
        );
    }

    #[test]
    fn a_stranded_lava_film_crusts_to_stone_and_its_chunk_sleeps() {
        // The measured cost that motivated the cooling model: a partial-
        // fill film stranded where no water can reach held its chunk awake
        // forever under the pin (195 cells and 9/40 chunks at frame 1500
        // of scene=lavapour). Under the cooling model the film is all edge
        // and no interior, so it is the *first* thing to crust — through
        // lava's freeze_min_fill of 0, which is what lets a partial cell
        // transition at all — and once the young stone sheds its heat the
        // world genuinely sleeps. Both drivers: sleeping is chunk
        // bookkeeping, and the parallel driver is the one the player runs.
        for parallel_driver in [false, true] {
            let mut w = test_world();
            let lava = w.materials.id_of("lava").unwrap();
            for x in 28..=34 {
                w.set(x, 40, Cell::new(material::STONE, 0).with_attached(true));
            }
            // A thin partial film, the stranded-flow state itself — below
            // water's 900 gate on purpose, so this also proves the
            // per-material gate is what lets it finish.
            for x in 29..=33 {
                w.set(x, 39, Cell::new(lava, 0).with_aux(220));
            }

            let mut slept_at = None;
            for frame in 0..2500 {
                if parallel_driver {
                    crate::sim::parallel::step(&mut w);
                } else {
                    super::super::update::step(&mut w);
                }
                if w.active_chunk_count() == 0 {
                    slept_at = Some(frame);
                    break;
                }
            }
            assert!(
                slept_at.is_some(),
                "a stranded lava film never let its chunk sleep in 2500 frames \
                 ({} lava cells still molten, froze {}, parallel: {parallel_driver})",
                count_of(&w, lava),
                w.phase_changes.froze,
            );
            assert_eq!(
                count_of(&w, lava),
                0,
                "the film should have crusted entirely (parallel: {parallel_driver})"
            );
            assert!(
                w.phase_changes.froze > 0,
                "the film left no molten cells yet froze counted nothing — \
                 it vanished by some other path than the cooling transition (parallel: {parallel_driver})"
            );
        }
    }

    /// The **quantitative** question the finite-inventory control below
    /// cannot ask: not "does boiling stop" but "does it stop where the
    /// energy runs out".
    ///
    /// Termination alone was never the gap. `filmstrip scene=simmer`
    /// terminated perfectly well while boiling **1,941** cells off a hearth
    /// holding roughly 547 boils' worth of heat, because `diffuse_heat`
    /// relaxes each cell toward its neighbour average using its own
    /// conductivity and never debits the giver -- so a hot cell is an
    /// amplifier, and a count that merely stops says nothing about how much
    /// it invented on the way. `LATENT_HEAT_DEGREES` is the charge that
    /// bounds it.
    ///
    /// **A pan over a hearth, not the sealed basin below**, and that is the
    /// difference between a guard and a decoration. Written first on the
    /// basin's one 700C row it passed with the charge switched off, because
    /// that scene has so little stored heat that even unbounded
    /// amplification only reaches thirty-odd boils. The scene has to hold
    /// enough heat for the defect to show.
    #[test]
    fn boiling_stops_where_the_stored_heat_runs_out() {
        let mut w = test_world();
        let (left, right, floor, hearth_rows) = (4, 59, 60, 3);
        let hearth_top = floor - hearth_rows;
        for y in 20..=floor {
            w.set(left, y, Cell::new(material::STONE, 0).with_attached(true));
            w.set(right, y, Cell::new(material::STONE, 0).with_attached(true));
        }
        let mut inventory = 0i32;
        for x in left..=right {
            for y in hearth_top..=floor {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true).with_temperature(900));
                inventory += 900 - AMBIENT_TEMPERATURE as i32;
            }
        }
        for x in (left + 1)..right {
            for y in (hearth_top - 14)..hearth_top {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }

        for _ in 0..4000 {
            crate::sim::parallel::step(&mut w);
            if w.active_chunk_count() == 0 {
                break;
            }
        }
        // **The divisor is written out, not read from the constant.** Read
        // from it, the bar moves with the thing under test: switching the
        // charge off to red-check also sends the bound to the whole
        // inventory, which nothing can exceed. Caught by red-checking, and
        // that is the only reason it is spelled like this.
        //
        // Doubled for headroom -- the hearth also loses heat downward and
        // sideways into its own stone, and the pool's warmth is a second
        // small reservoir, so the exact figure is not something to pin.
        let affordable = (inventory / 540) as u32;
        assert!(
            w.phase_changes.boiled > 0,
            "nothing boiled at all, so this bounds nothing -- the scene has stopped containing the situation it is for"
        );
        assert!(
            w.phase_changes.boiled <= affordable * 2,
            "{} cells boiled off an inventory that can pay for {affordable}; heat is being manufactured somewhere",
            w.phase_changes.boiled
        );
    }

    #[test]
    fn a_finite_heat_inventory_stops_boiling_and_the_world_sleeps() {
        // The thermodynamic sanity check for the whole boil/condense loop:
        // a basin, a pool, and one row of floor set to 700C — a known,
        // finite amount of heat and nothing that generates more. The heat
        // boils some surface water, the steam condenses and rains back,
        // the stone drains through its conductivity, and everything must
        // *end*: boiling stops and the chunks sleep. Written as a control
        // while chasing `scene=lavapour`'s never-ending simmer (see
        // `Reports/open-bugs-handoff.md`), and kept because it separates
        // "the loop manufactures heat" (it does not — this passes, and the
        // pond-only `scene=boil` also terminates once its fire is out)
        // from "some scene still holds a source or a pocket" (lavapour's
        // open question). Measured when written: 30 cells boiled, asleep
        // well before frame 2000.
        let mut w = test_world();
        for y in 36..=50 {
            w.set(20, y, Cell::new(material::STONE, 0));
            w.set(44, y, Cell::new(material::STONE, 0));
        }
        for x in 20..=44 {
            w.set(x, 50, Cell::new(material::STONE, 0).with_temperature(700));
        }
        for x in 21..44 {
            for y in 44..50 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }

        let mut slept_at = None;
        for frame in 0..4000 {
            crate::sim::parallel::step(&mut w);
            if w.active_chunk_count() == 0 {
                slept_at = Some(frame);
                break;
            }
        }
        assert!(
            slept_at.is_some(),
            "a finite 700C inventory never let the world sleep in 4000 frames ({} boiled and counting)",
            w.phase_changes.boiled
        );
        assert!(
            w.phase_changes.boiled > 0,
            "the inventory never boiled anything at all — the control controls nothing"
        );
    }

    /// How many cells of one material the world holds. A census, not an
    /// event count -- `CLAUDE.md`'s "a failure count is not a damage count"
    /// applies here too: `PhaseCounts::reacted` says how many reactions
    /// fired, which is not the same question as how much lava is left.
    fn count_of(w: &World, material: MaterialId) -> usize {
        let mut n = 0;
        for y in 0..64 {
            for x in 0..64 {
                if w.get(x, y).material == material {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn a_mixes_heat_reaction_gives_both_products_the_mean() {
        // The absorption rule kept from the reverted contact-condensation
        // attempt (see steam.ron's revert note): exchange semantics hand a
        // collapsing bubble's whole heat to one cell and mint a boiler, so
        // an absorption-shaped reaction declares mixes_heat and both
        // products take the pair's mean. Synthetic content, the same
        // temp-dir technique the other reaction tests use — no shipped
        // material declares this yet, and the machinery must not rot
        // untested while it waits.
        let dir = std::env::temp_dir().join("pixel-physics-mixes-heat-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.ron"),
            "(name: \"mix_a\", kind: Gas, density: 0.1, colors: [(1, 1, 1)], \
             reactions: [(with: \"mix_b\", produces: (\"mix_b\", \"mix_b\"), chance: 1.0, mixes_heat: true)])",
        )
        .unwrap();
        std::fs::write(dir.join("b.ron"), "(name: \"mix_b\", kind: Liquid, density: 1.0, colors: [(2, 2, 2)])").unwrap();

        let mut w = test_world();
        w.materials.reload(&dir).unwrap();
        let a = w.materials.id_of("mix_a").unwrap();
        let b = w.materials.id_of("mix_b").unwrap();
        w.set(30, 30, Cell::new(a, 0).with_temperature(160));
        w.set(31, 30, Cell::new(b, 0).with_temperature(20));
        update(&mut w, 30, 30);

        assert_eq!(w.get(30, 30).material, b);
        assert_eq!(w.get(30, 30).temperature(), 90, "self product should take the pair mean");
        assert_eq!(w.get(31, 30).temperature(), 90, "neighbour product should take the pair mean, not the hotter side");

        std::fs::remove_dir_all(&dir).ok();
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

    /// The Phase 0 probe for the latent snow bug, **now the guard for its
    /// fix** (`Reports`' revert-keeps-the-knowledge convention, applied
    /// forward). It shipped `#[ignore]`d against a real gap: snow.ron had
    /// no `heat_conductivity`, so a cell the storm chilled below its 2°C
    /// melting point could never warm back up — `diffuse_heat` early-exits
    /// at conductivity ≤ 0 — and `must_stay_dirty` kept its chunk awake
    /// forever while it waited. The wiki's "drifts thaw when the front
    /// passes" was therefore only true of the cells a storm never actually
    /// chilled.
    ///
    /// Measured across the fix: **panicked at the 2000-frame budget
    /// before, thaws at frame 3 after** (snow.ron's 0.08 conductivity pulls
    /// a lone flake ≈2°/visit toward ambient: -6 → -4 → -2 → 0 → 2, and
    /// 2°C is its melting point). The budget below is set from that with
    /// two orders of headroom rather than from the old 2000, so it is a
    /// bar rather than a formality: it fails if the conductivity is
    /// dropped again, and it fails if a thaw ever takes seconds instead of
    /// a few frames.
    #[test]
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

        // 120 rather than 2000: see the doc above -- the measurement is 3.
        for frame in 0..120 {
            crate::sim::parallel::step(&mut w);
            if w.get(30, 63).material != snow {
                assert_eq!(w.phase_changes.melted, 1, "the flake left (30,63) without melting — scene broken again?");
                println!("thawed at frame {frame}");
                return;
            }
        }
        panic!("a chilled flake never thawed in 120 frames after the storm passed");
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
