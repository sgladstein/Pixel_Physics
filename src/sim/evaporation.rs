//! Standing water dries up — and a lake does not, without anything anywhere
//! measuring the size of a body of water.
//!
//! # The idea
//!
//! Water is already a moisture source (`field::apply_moisture_sources`), so
//! it humidifies the air above itself. A wide body saturates that air and
//! then cannot evaporate into it; a thin puddle on bare rock sits in
//! whatever humidity happens to be around and goes. **One body shelters
//! itself and a small one cannot**, which is the real mechanism and not a
//! stand-in for one. Nothing here counts cells, floods a region or asks how
//! big anything is — deliberately, because a size cap that gates whether
//! something happens at all is the mistake `CLAUDE.md` names outright.
//!
//! Measured over settled standing water, humidity one field block above the
//! surface (`probe_humidity_against_width`):
//!
//! | width | 2 | 4 | 8 | 16 | 32 | 64 | 128 | 240 |
//! |---|---|---|---|---|---|---|---|---|
//! | humidity | 1.32 | 1.32 | 1.32 | 1.67 | 2.08 | 2.27 | 2.31 | 2.31 |
//!
//! Low and flat out to about eight cells, climbing through sixteen, and
//! asymptotic at 2.31 well before a hundred. The asymptote is what matters
//! and it is robust: it read 2.310 on a 640x200 world with bodies four rows
//! deep and 2.310 again on the 256x128 world with two-row bodies the guards
//! use now. `HUMID_STOP` is set at 2.0 off
//! that asymptote, so anything from about sixteen cells wide already reads
//! as sheltered on humidity alone and everything narrower grades.
//!
//! **On a calm world that is the whole mechanic.** It is not enough on a
//! windy one, and `shelter`'s own doc has that story — a gale mixes the
//! atmosphere, so the air over a puddle and the air over a lake become
//! equally dry and no reading of humidity can tell them apart. The second
//! factor exists for exactly that case and for nothing else.
//!
//! # Why it is on the scheduler and not on the CA sweep
//!
//! **This was built once on the sweep and reverted** (`Reports/weather-
//! handoff.md` §1). Firing from the `MaterialKind::Liquid` dispatch arm
//! looks right — that arm already holds the `Cell`, and "still water" is a
//! condition it can read for free. But a settled chunk is not swept, and
//! still water is exactly the state that stops being visited: the rule only
//! ran while a body was *moving*. Forced to a rate high enough to prove it
//! ran at all, the lake lost 7% and the puddle 1.7% — backwards, because a
//! lake takes longer to settle and so stays awake longer.
//!
//! The active-site scheduler is the engine's existing answer to "do
//! something to this cell later without keeping its chunk awake", so
//! evaporation is an `ActiveKind` like moss growth or ash decay. The sweep
//! still has a part, but only a cheap one: when a liquid cell fails to move
//! *and* has air above it, it schedules an evaporation site and forgets
//! about it. From then on the site reschedules itself out of
//! `scheduler::step`, and chunk sleep has nothing to do with it.
//!
//! The alternative — keeping water chunks awake — would hand back the whole
//! per-tile field-sleeping win, and is not on the table.
//!
//! # And then it stops
//!
//! Every guard this area has produced tested that a mechanism *fires*, and
//! none tested that it stops (the handoff's own single lesson). Two separate
//! stopping conditions here, and they are not the same one:
//!
//! * **Structurally finished** — the cell is no longer an evaporating liquid
//!   at all. Nothing is rescheduled, full stop.
//! * **Sealed in** — still liquid, but something is sitting on top of it.
//!   `stale_ticks` counts consecutive checks like that and the site retires
//!   at `STALE_LIMIT`, so a buried aquifer does not check itself for the
//!   rest of the program's life. Transient cover (a splash landing on a
//!   pool) costs a few wasted checks and no more; when it clears, the sweep
//!   re-schedules the cell the same way it did the first time.
//!
//! **A saturated surface is neither of those and keeps checking.** A lake
//! evaporates at exactly zero and still reschedules, because "the air above
//! me is wet" is a *value*, not a structure: it changes when the weather
//! does, and a lake that retired the first time it read as sheltered could
//! never resume when the air dried out. The cost is bounded by the exposed
//! surface area of the world's water divided by `CHECK_INTERVAL` — for a
//! lake spanning the whole 8192-wide world this ships at, about 136 reads
//! per frame (8192 / 60; it was about thirty-four at the 2048-wide world
//! this was written against) — and it buys the guarantee that this
//! mechanic has no permanent off-state it can fall into silently, which is
//! the failure it was rebuilt to avoid.
//!
//! # Water spread thin over a flat floor never dries
//!
//! Not a bug and not a regression, but the first surprising thing anyone
//! will hit. 121,000 fill poured onto a floor that spans the world runs out
//! into a film one cell deep across every column, and a film that wide reads
//! 2.34 humidity above it — over `HUMID_STOP`, so the rate is exactly zero,
//! and `shelter` says the same. A sheet of water covering everything really
//! does saturate its own air, so this is the design giving its own answer;
//! the film was permanent before evaporation existed, too. It is recorded
//! here because the `flat` preset is exactly that geometry, and because the
//! obvious next lever — reading *depth* at the cell, which is thermal mass
//! rather than size — is a third discriminator on a design that has two and
//! is a decision rather than an omission. See `Reports/weather-handoff.md`.
//!
//! # Where the water goes
//!
//! Into `World::atmospheric_bank`, and out of it again as rain
//! (`weather::step`). This file used to be one-way: it deleted water and
//! credited nothing, so a world's total only ever fell, and the handoff
//! recorded that as the outstanding half of the cycle. Both fill reductions
//! below now credit exactly what they removed — the partial one credits
//! `loss`, the one that empties the cell credits the *whole remaining fill*,
//! which is the difference between an accounting identity and a leak
//! proportional to how often puddles finish.
//!
//! Nothing about the *rate* changed, and nothing here reads the bank: a
//! puddle dries at the same speed over a full sky as over an empty one,
//! because what stops evaporation is the humidity above it, which is a local
//! reading and not a global balance.
//!
//! # Warm days dry, cool nights barely do
//!
//! The rate's third factor, and the one thing in the engine that reads an
//! oscillating channel *without* dividing the oscillation back out. The sky
//! now writes a day/night temperature (`field::apply_sky_temperature_to`),
//! and `warmth` turns it into a multiplier that is 1.0 at ambient, rises
//! above and falls below. Over the 32-cell basin, whose stone lid attenuates
//! the sky to +-4.59 degrees at the water, a noon-centred window of 1,800
//! frames loses 8.49 cell-equivalents against a midnight-centred one's 3.44
//! — two and a half to one, on the same basin at the same age.
//!
//! **`HUMID_STOP` and the width table above were re-derived against this and
//! did not move**, digit for digit, and neither did `FILL_PER_CHECK`; the
//! numbers are at `WARMTH_PER_DEGREE` and `warmth`.
//!
//! Three things about it that are load-bearing and are argued at `warmth`
//! itself: it is at **one site**, so the day cannot compound through
//! humidity as well; it is **linear**, so a day's total drying is unchanged
//! and `FILL_PER_CHECK` still means what it meant; and it is **floored above
//! zero**, so a cold night is a brake and never a stop.
//!
//! It supersedes this file's own note that there was no oscillator to worry
//! about, which was true when it was written and stopped being true the day
//! the sky started writing temperature — the note said so, and this is the
//! revisit it asked for.
//!
//! # One thing deliberately not done
//!
//! * **No roll.** The loss is a straight subtraction rather than a
//!   probability, so this draws nothing from `world.rng` and cannot shift
//!   what any other consumer of that stream sees. Water levels itself, so a
//!   uniform per-surface-cell loss does not read as uniform on screen
//!   anyway.

use super::cell::{Cell, AMBIENT_TEMPERATURE};
use super::field::FIELD_SCALE;
use super::material::MaterialKind;
use super::scheduler::{ActiveKind, ActiveSite};
use super::surface::CellSurface;
use super::update::liquid_fill;
use super::world::World;

/// Frames between an exposed surface cell's evaporation checks.
///
/// One full day/night cycle is 3600 frames (`field::DAY_NIGHT_PERIOD_
/// FRAMES`), so this is sixty checks a day per surface cell. Slow enough
/// that a world's worth of shoreline costs nothing measurable, frequent
/// enough that the loss per check stays small against a cell's own fill and
/// a puddle shrinks smoothly rather than in visible steps.
///
/// `pub(crate)`: the sweep hook needs it to date the first check.
pub(crate) const CHECK_INTERVAL: u64 = 60;

/// Humidity at or above which evaporation stops completely, on the field's
/// own `0..MAX_MOISTURE` (`0..4`) scale.
///
/// Set from the measured asymptote in the module doc — 2.31 over water wide
/// enough that widening it further changes nothing — with headroom below it
/// rather than sitting on it, so a lake reads as sheltered outright instead
/// of grazing the threshold. The practical consequence is that bodies from
/// roughly forty cells wide stop dead and everything narrower grades.
const HUMID_STOP: f32 = 2.0;

/// Humidity one whole cell-equivalent of evaporated water adds to the block
/// it left. See `vapour_of`, which is where this is argued.
const VAPOUR_PER_CELL_EQUIVALENT: f32 = 2.0;

/// Fill removed from one exposed surface cell per check in perfectly dry
/// air, on `material::LIQUID_FULL`'s 0..1000 scale. Scaled by dryness, so
/// this is a ceiling and not the typical case: a narrow puddle's own air
/// sits around 1.4, which is a little over a quarter of this.
///
/// Set from the timescale that reads right rather than from anything
/// physical — see the tests for what it actually produces.
const FILL_PER_CHECK: u16 = 100;

/// How much one degree above `AMBIENT_TEMPERATURE` adds to the drying rate,
/// as a fraction of the ambient rate. See [`warmth`] for the shape and for
/// why it is linear.
///
/// **Swept on the 32-cell basin, a noon-centred against a midnight-centred
/// window of 1,800 frames each** (`probe_day_against_night_drying`), in
/// cell-equivalents credited to the bank. The basin's lid attenuates the sky
/// to +-4.59C at the water (`probe_temperature_over_water_across_a_day`), so
/// these ratios are the *shaded* case and open sky gives more:
///
/// | per degree | noon | midnight | ratio | day + night |
/// |---|---|---|---|---|
/// | 0.00 (before) | 5.760 | 5.760 | 1.00 | 11.52 |
/// | 0.06 | 6.958 | 4.978 | 1.40 | 11.94 |
/// | 0.10 | 7.652 | 4.255 | 1.80 | 11.91 |
/// | 0.15 | 8.488 | 3.441 | **2.47** | 11.93 |
/// | 0.20 | 9.296 | 3.096 | 3.00 | 12.39 |
///
/// 0.15 is the setting. Two and a half to one is legible without watching a
/// clock — a puddle visibly shrinking through the afternoon and visibly
/// sitting there overnight — while 0.20 starts to read as a switch rather
/// than as weather, and below 0.10 nobody watching would call it a day. The
/// last column is the mean-neutrality this shape was chosen for, holding
/// across the whole sweep: the coupling moves *when* a day's water goes, not
/// how much.
///
/// **The 6-cell puddle is the wrong width to sweep on and was the first
/// thing tried.** It reads 10.28 / 10.33 / 10.24 / 10.15 at the four live
/// settings — nearly flat, and not because the knob was disconnected but
/// because a 12-cell-equivalent puddle *finishes* inside a noon window at
/// any of them. A saturated metric, and exactly the shape `CLAUDE.md` warns
/// about: ask what the number does when nothing is wrong. The 32-cell basin
/// and the 176-cell lake are both rate-limited for the whole window and
/// agree with each other to within 2% on the ratio at every point.
///
/// The lake is not slower to dry than before in absolute terms, and it is
/// not faster either: a *sheltered* surface is at exactly zero rate and zero
/// times anything is still zero, so the contract that a lake outlives a
/// drought is untouched by this constant at any value.
const WARMTH_PER_DEGREE: f32 = 0.15;

/// Floor under [`warmth`]. Cold slows drying and never stops it — see that
/// function. Reached at 13.7 degrees below ambient, which the sky cannot do
/// on its own (its swing is 6) and a blizzard's `weather::SNOW_CHILL` (26)
/// very much can.
///
/// A quarter rather than a tenth because the thing on the other side of it is
/// a mechanic with no other way to resume: a chilled surface still has to be
/// visibly *drying slowly* rather than parked, or the only difference between
/// "cold" and "sealed under rock" is how long you watch.
const WARMTH_FLOOR: f32 = 0.25;

/// Ceiling on [`warmth`]. Reached at 13.3 degrees above ambient — again, out
/// of the sky's reach and well inside a fire's.
///
/// Three, so a fire beside a pond boils it off at three times the ordinary
/// rate: fast enough to watch, bounded so that a plume reading 400 degrees
/// for two frames cannot take a lake with it. The cap bounds *work per
/// check* and gates nothing — a hot surface still evaporates, it just stops
/// getting faster (`CLAUDE.md`: a size cap must bound work, never gate
/// whether something happens).
const WARMTH_CEIL: f32 = 3.0;

/// Consecutive checks finding the cell covered before it stops rescheduling
/// itself. Three, so brief cover (a splash landing on a pool, a grain
/// falling through) costs a couple of wasted reads rather than retiring a
/// live surface — and a genuinely buried cell is off the schedule inside
/// four checks.
const STALE_LIMIT: u8 = 3;

/// How far either side, in field blocks, `shelter` looks for other standing
/// water. Three blocks is `3 * FIELD_SCALE` = 24 cells each way, so a body
/// needs to be about fifty cells across before it shelters its own middle
/// completely — comfortably wider than anything a player would call a puddle
/// and comfortably narrower than anything worldgen calls a lake.
///
/// Seven reads once per `CHECK_INTERVAL` per exposed surface cell, which is
/// the cheap end of the O(r) scans this engine already does. It is
/// deliberately *not* the O(r²) neighbourhood scan `field.rs`'s moisture
/// channel was built to replace: the vertical extent of a body is not what
/// shelters its surface, so there is nothing to gain by looking down.
const SHELTER_REACH: i32 = 3;

/// Whether `(x, y)` is liquid that evaporates, with open air directly above
/// it. The one definition of "exposed surface", shared by the sweep hook and
/// the tick so the two cannot drift apart.
///
/// `Cell::is_empty` rather than a raw `material == EMPTY` compare on the cell
/// above, deliberately: a promoted liquid body's container cells are
/// materially empty and are *not* air, and this is exactly the "is this
/// position available" question that predicate answers.
fn is_exposed_surface<S: CellSurface>(surface: &S, x: i32, y: i32) -> bool {
    let cell = surface.get(x, y);
    if cell.managed() || !surface.materials().get(cell.material).evaporates {
        return false;
    }
    surface.materials().kind(cell.material) == MaterialKind::Liquid && surface.get(x, y - 1).is_empty()
}

/// Called from the CA sweep's `Liquid` arm when a liquid cell did not move.
///
/// `World::schedule_active_site` dedups by position, so a pool settling over
/// hundreds of frames ends up with one site per surface cell rather than one
/// per surface cell per frame. **That dedup is load-bearing and not merely an
/// optimisation:** without it the number of duplicate sites a body
/// accumulates is proportional to how long it stays awake settling, which is
/// precisely the quantity that made the reverted sweep version evaporate a
/// lake faster than a puddle. It would have come back by another route.
///
/// # The first check is staggered by position, and a gate here is not
///
/// Scheduling every cell of a body on the frame it settles means all of them
/// come due on the same frame forever after — a periodic spike of one check
/// per surface cell in the world, all landing together. So the *first*
/// `next_frame` is offset by a position-derived phase; a reschedule adds
/// exactly `CHECK_INTERVAL`, so they stay spread from then on.
///
/// **The obvious cheaper thing — gate the hook itself on `(x + y) % INTERVAL
/// == frame % INTERVAL`, and skip the reads entirely on the other 59 frames
/// — was written, and it silently turned the whole mechanic off.** Nothing
/// evaporated anywhere, in any test. A chunk is swept for a handful of
/// frames after its water stops moving and then sleeps, so a gate that opens
/// one frame in sixty is a gate that mostly never opens while the sweep is
/// still visiting. It is the same shape as the mistake this whole file
/// exists to correct, arriving one level down: **the sweep's visits are the
/// scarce resource here, and anything that discards one is discarding the
/// only chance there was.**
///
/// It was written to fix a frame cost that turned out not to exist.
/// `ascii.rs`'s parallel field-stress scene — 35,840 water cells churning —
/// read 52.7 ms against an 11.9 ms baseline, which looked like a 4x
/// regression from this hook. Re-measured in the same session it reads 8.5
/// to 20.9 ms with the feature and 11.2 to 11.8 without it, and a standalone
/// probe of that exact scene (`probe_stress_scene_split`) puts the worst
/// frame at 11.5 ms with 70 sites scheduled across the entire run. Noise,
/// start to finish. Two lessons, and the second is the expensive one: the
/// repo's rule about re-measuring a baseline in the same session applies to
/// a *regression you are about to fix*, not only to one you are about to
/// report — and a cheap-looking guard on a hot path has to be checked
/// against whether the path is still reachable.
/// How much saturation a damp soil cell gives up per due check, before the
/// three rate factors scale it.
///
/// **A fifth of a liquid cell's `FILL_PER_CHECK`, and the ratio is the
/// point.** Soil does not present a free surface: water leaves it through
/// pore space against matric suction, which is why a damp field takes days
/// to dry where a puddle of the same volume takes hours. Set low enough
/// that the ground is still visibly damp long after rain has stopped --
/// what this exists to close is a *ledger* leak measured over tens of
/// thousands of frames, not to make puddle-drying happen on dirt.
const SOIL_DRY_PER_CHECK: u16 = FILL_PER_CHECK;

/// Saturation a drying soil cell will not go below.
///
/// The **permanent wilting point**, which is already the engine's stated
/// floor for water a plant can reach (`material::SOIL_WILTING_POINT`).
/// Below it the water is held too tightly for a root to take, and it is
/// held too tightly for the air to take as well -- so drying to zero would
/// be claiming that sunshine can do what a plant cannot.
///
/// It is also what makes this terminate: a cell at the floor retires and
/// stops rescheduling, exactly as a drained puddle does.
const SOIL_DRY_FLOOR: u16 = super::material::SOIL_WILTING_POINT;

/// Smallest share of atmospheric demand a drying soil cell will fall to,
/// however dry it is. See `soil_wetness_factor` — a brake that reaches zero
/// makes the sink unreachable and reopens `open-bugs-handoff.md` §F8.
const SOIL_DRY_RATE_FLOOR: f32 = 0.25;

/// **How much the soil's own wetness slows its drying** — the term this rate
/// did not have, and the reason it could not have one before.
///
/// Every other factor in the rate is a property of the *air*: humidity,
/// shelter, warmth. Together they are atmospheric demand, and demand alone
/// is the right model only while the surface is wet enough to meet it. Real
/// soil dries in two stages — a constant-rate stage set by demand, then a
/// falling-rate stage set by how fast water reaches the surface — and this
/// engine only had the first. A bare bed therefore dried at full rate right
/// down to the floor.
///
/// **It could not matter until capillary worked**, which is why it was
/// missing rather than wrong. With the old rest threshold nothing resupplied
/// a drying surface, so the sink emptied `SOIL_DRY_REACH` rows and stopped
/// against an empty cell; the brake would have changed only how fast it got
/// there. Now that `update.rs` equalises unsaturated soil, the surface is
/// refilled from below as fast as it dries, and an unbraked sink is a wick
/// straight out of the bed: measured without this, every row of the rooting
/// zone read **180, the wilting point**, by frame 12,000, which kills the
/// stand outright. `Reports/dead-ends.md` carries that run.
///
/// **`plant_available_fraction`, not a new curve.** It is already the
/// engine's answer to "how wet is this soil, in the band that matters" — 1
/// at field capacity, 0 at the wilting point, which is exactly
/// `SOIL_DRY_FLOOR`. So a bed at field capacity dries at precisely the rate
/// it always did (the factor is 1 and this change is invisible on wet
/// ground), and the rate falls to nothing as the soil approaches the floor
/// the sink was already clamped to. Reusing it also means the sink and the
/// plants agree about what counts as dry, rather than the engine carrying
/// two opinions.
///
/// **The floor is not slack, it is what keeps the sink reachable**, and it
/// was put there by a guard rather than by taste. Scaling straight by
/// `plant_available_fraction` sends the rate to zero as soil approaches the
/// wilting point, and
/// `evaporation::tests::unplanted_soil_gives_water_back_to_the_air` went red
/// on two of its three seeds: soil moisture **monotone non-decreasing across
/// four weather epochs**, which is `open-bugs-handoff.md` §F8 exactly — a
/// bed with three sources and no reachable sink. A brake that can reach zero
/// is not a brake, it is a second version of the bug this file was written
/// to fix. So the rate falls to a quarter of demand and no further: dry
/// ground still gives water back, just slowly.
///
/// **It slows the leak, it does not close it**, and that is the honest
/// bound: with any non-zero sink and no rain, a sealed box still drains
/// eventually. Closing it needs humidity to be a conserved quantity that
/// *rises* as soil dries — the field currently derives humidity *from* soil
/// moisture, so drying soil lowers humidity and speeds its own drying, which
/// is a runaway rather than a brake. Owner's call, 2026-08-31: this now, the
/// water cycle when it gets its own pass.
fn soil_wetness_factor(cell: Cell) -> f32 {
    SOIL_DRY_RATE_FLOOR + (1.0 - SOIL_DRY_RATE_FLOOR) * super::update::plant_available_fraction(cell)
}

/// Schedule a damp soil cell to be checked for drying.
///
/// **Called where soil is *wetted*, not from the sweep**, and that is the
/// whole of why it works. The sweep-hook version of liquid evaporation was
/// built and reverted because a settled body is not swept; damp soil is
/// worse, because a soil bed that has stopped moving water is *permanently*
/// unswept -- `update_soil_water` returns false the moment moisture stops
/// changing, deliberately, so that a settled damp bed sleeps like a dry one.
/// A hook there would fire exactly never.
///
/// The two places soil gets wet both run while the chunk is provably awake:
/// `weather::step`'s rain soak, and `update::update_soil_water`'s
/// infiltration. Both go through `CellSurface::schedule_active_site`, which
/// dedups by position -- load-bearing here for the same reason it is for
/// liquid, since a bed under a long storm would otherwise accumulate one
/// site per drop.
#[cfg(test)]
impl World {
    /// Test-only door onto `schedule_damp_soil`, so a guard can put a
    /// hand-built bed on the schedule without going through a storm.
    pub(crate) fn schedule_damp_soil_for_test(&mut self, x: i32, y: i32) {
        schedule_damp_soil(self, x, y);
    }
}

pub(crate) fn schedule_damp_soil<S: CellSurface>(surface: &mut S, x: i32, y: i32) {
    if !is_damp_soil_surface(surface, x, y) {
        return;
    }
    let phase = (x + y).unsigned_abs() as u64 % CHECK_INTERVAL;
    surface.schedule_active_site(ActiveSite {
        x,
        y,
        kind: ActiveKind::Evaporate { stale_ticks: 0 },
        next_frame: surface.frame() + 1 + phase,
    });
}

/// How many rows down from a bare soil surface evaporation can reach —
/// **the sink half of `weather::SOAK_DEPTH`, and set equal to it.**
///
/// This did not exist, and its absence *is*
/// `Reports/open-bugs-handoff.md` §F8. The sink below dried the surface
/// cell and only the surface cell, on the reasoning that "soil under soil
/// keeps what it has and gives it up to the surface by capillary flow".
/// Capillary flow does not do that: `update.rs`'s exchange deliberately
/// rests once the gradient falls under `SOIL_CAPILLARY_REST`
/// (`SOIL_SATURATED - SOIL_FIELD_CAPACITY`, i.e. 380), and that band is
/// wider than the range the sink can pull (`SOIL_FIELD_CAPACITY -
/// SOIL_WILTING_POINT`, 440). So the profile parks at "surface at the
/// wilting point, everything under it at up to 560" and stops: the
/// surface cell then fails the predicate below, its site retires, and the
/// rest of the bed holds what it has for ever.
///
/// Measured on a plantless 128-wide bed over ten weather epochs, three
/// seeds, before this constant existed: soil `aux` **monotone
/// non-decreasing on every one**, seed 1 climbing 230,400 -> 463,927 and
/// seeds 4 and 7 rising once and then flat to the last frame. And it was
/// not the humidity shadow §1m suspected — 19,388 soil checks on seed 1
/// with **3** of them becalmed. The sink was running and had nothing left
/// it was allowed to touch.
///
/// **Equal to `SOAK_DEPTH` on purpose**: that is how deep a drop wets, so
/// this is how deep the sun can take it back, and the source and the sink
/// then cover the same band. Water below it came down by drainage and is
/// the water table, which does not evaporate — that part of §F8's
/// "ratchets to field capacity" is correct behaviour and stays.
const SOIL_DRY_REACH: i32 = super::weather::SOAK_DEPTH;

/// The soil half of `is_exposed_surface`, and kept beside it for the same
/// reason that one exists: the scheduler and the tick have to agree.
///
/// Air *directly above*, so only the top of a bed is a drying site — but
/// the site speaks for the `SOIL_DRY_REACH` rows beneath it, not for its
/// own cell alone. It stays live while any of them holds water above the
/// floor, which is what stops a surface cell that has reached the floor
/// from retiring a column that is still full underneath.
fn is_damp_soil_surface<S: CellSurface>(surface: &S, x: i32, y: i32) -> bool {
    let m = surface.materials().get(surface.get(x, y).material);
    if m.water_capacity == 0 || m.kind != MaterialKind::Powder || !surface.get(x, y - 1).is_empty() {
        return false;
    }
    drying_front(surface, x, y).is_some()
}

/// The shallowest cell in this site's band that still holds water above the
/// floor, and how far down it is — the drying front. `None` once the whole
/// band is at the wilting point.
///
/// Stops at the first cell that cannot hold water, so a thin soil cap over
/// stone speaks only for the soil.
fn drying_front<S: CellSurface>(surface: &S, x: i32, y: i32) -> Option<(i32, Cell, u16)> {
    for d in 0..SOIL_DRY_REACH {
        let cell = surface.get(x, y + d);
        if surface.materials().get(cell.material).water_capacity == 0 {
            return None;
        }
        let moisture = super::update::soil_moisture(cell);
        if moisture > SOIL_DRY_FLOOR {
            return Some((d, cell, moisture));
        }
    }
    None
}

pub(crate) fn schedule_from_sweep<S: CellSurface>(surface: &mut S, x: i32, y: i32) {
    if !is_exposed_surface(surface, x, y) {
        return;
    }
    let phase = (x + y).unsigned_abs() as u64 % CHECK_INTERVAL;
    surface.schedule_active_site(ActiveSite {
        x,
        y,
        kind: ActiveKind::Evaporate { stale_ticks: 0 },
        next_frame: surface.frame() + 1 + phase,
    });
}

/// Dry a damp soil surface cell, crediting exactly what it loses.
///
/// The same three factors the liquid path multiplies -- dry air, an
/// unsheltered surface, warm air over it -- deliberately, because they are
/// answers about *the air*, and the air over damp ground is the same air.
/// `shelter` in particular already grades damp soil (`field.rs` builds
/// `moisture_source` from `aux / water_capacity`), so a wide wet field
/// shelters itself and dries slower than an isolated damp patch, which is
/// the same behaviour a lake gets against a puddle and is right for the
/// same reason.
///
/// # The second credit path, and what it does not close
///
/// `STORM_RESERVE`'s doc calls infiltration "an un-credited sink" and says
/// closing it "means crediting soil drainage, which is a second credit path
/// and a separate piece of work". This is that path, arrived at from the
/// other end: the water goes back to the sky rather than back to a puddle,
/// because that is what happens to water in the top inch of a field.
///
/// It does **not** close every soil leak. `plant.rs`'s `transpire` destroys
/// soil moisture and credits nothing, root uptake moves it into a plant
/// that the ledger cannot see, and worldgen fills a water table before the
/// first frame. Those are recorded rather than fixed -- see
/// `Reports/weather-handoff.md`.
fn tick_soil(world: &mut World, x: i32, y: i32, stale_ticks: u8) -> Vec<ActiveSite> {
    if !is_damp_soil_surface(world, x, y) {
        // Dried out, buried, or no longer soil. Structurally finished, so
        // it retires rather than rescheduling -- the same shape as a liquid
        // cell that has drained away entirely.
        return Vec::new();
    }
    // **The front, not the surface cell.** One cell dries per check -- the
    // shallowest that still has water -- and the rate falls with its depth,
    // the same `1/(d+1)` profile `weather.rs`'s soak uses on the way in.
    // Drying one cell per check rather than the whole band is what makes
    // this a descending front instead of a slab, and it bounds the work at
    // one write however deep `SOIL_DRY_REACH` goes.
    let Some((depth, cell, moisture)) = drying_front(world, x, y) else { return Vec::new() };
    let reschedule =
        ActiveSite { x, y, kind: ActiveKind::Evaporate { stale_ticks: 0 }, next_frame: world.frame + CHECK_INTERVAL };

    let rate = dryness_counted(world, x, y, true)
        * (1.0 - shelter(world, x, y))
        * warmth(world, x, y)
        * soil_wetness_factor(cell);
    let loss = (SOIL_DRY_PER_CHECK as f32 * rate / (depth + 1) as f32) as u16;
    if loss == 0 {
        // Sheltered or saturated air. Still on the schedule, for the reason
        // the liquid path gives: a rate of zero is a statement about the
        // weather right now, and the weather changes.
        if stale_ticks + 1 >= STALE_LIMIT {
            return Vec::new();
        }
        return vec![ActiveSite { x, y, kind: ActiveKind::Evaporate { stale_ticks: stale_ticks + 1 }, next_frame: world.frame + CHECK_INTERVAL }];
    }

    let loss = loss.min(moisture - SOIL_DRY_FLOOR);
    world.set(x, y + depth, cell.with_aux(moisture - loss));
    // Credit exactly what was removed, on the same 1:1 scale infiltration
    // already uses to move fill into `aux`.
    world.credit_atmosphere(loss);
    // ...and damp the air it went into. `credit_atmosphere` books the water
    // globally so the sky can rain it back; this is the *local* half, which
    // did not exist. See `field::FieldTile::vapour` for the runaway it
    // closes -- without it, ground that starts drying makes its own air
    // drier and so dries faster still.
    world.add_vapour(x, y + depth, vapour_of(loss));
    vec![reschedule]
}

/// What a fill reduction is worth as humidity, on the field's own
/// `0..MAX_MOISTURE` scale.
///
/// **The two scales have no physical bridge, so this is an exchange rate and
/// is sized rather than derived.** A cell's fill runs to
/// `material::LIQUID_FULL`; a field block spans `FIELD_SCALE` squared world
/// cells and reads `0..4`. The bar it has to clear is `HUMID_STOP` (2.0),
/// the humidity at which drying stops: one exposed cell giving up
/// `SOIL_DRY_PER_CHECK` should raise its block noticeably without silencing
/// a whole block's drying in a single check, or one puff would stop a
/// desert.
///
/// At this rate a full `SOIL_DRY_PER_CHECK` of 100 adds 0.2 -- a tenth of
/// the way to `HUMID_STOP` -- so closing the gate takes ten checks of
/// sustained drying in one block, and `field::VAPOUR_PERSISTENCE` is what
/// holds the earlier puffs while the later ones arrive.
fn vapour_of(fill: u16) -> f32 {
    fill as f32 / crate::sim::material::LIQUID_FULL as f32 * VAPOUR_PER_CELL_EQUIVALENT
}

/// How dry the air above `(x, y)` is, `0.0..=1.0` — the weather half of the
/// rate.
///
/// Sampled `FIELD_SCALE` cells up, which lands in exactly one field block
/// above this cell's own block whatever the alignment. Reading this cell's
/// own block instead would answer `MAX_MOISTURE` every time — a water cell
/// pins its own block to saturation by definition, so the block a puddle
/// sits in and the block a lake sits in read identically and carry no signal
/// at all.
/// Cumulative "did it fire at all" counters for evaporation, in
/// `fire::PhaseCounts`'s style and for a sharper reason than usual.
///
/// # A rate of zero and a mechanism that never ran look identical
///
/// `dryness` returns exactly zero whenever the humidity one block up is at
/// or over `HUMID_STOP`, and that is *designed* -- a calm lake is meant to
/// read as sheltered outright. But the same reading is produced by damp
/// ground, and nobody had checked how often.
///
/// A block's moisture is forced to `MAX_MOISTURE * level` by
/// `field::apply_moisture_sources`, and `field::rebuild_blocked` grades soil
/// as `soil_moisture / water_capacity`, taking the **max over the whole 8x8
/// block**. `soil.ron` has a capacity of 1000 and worldgen's `soil_moisture`
/// pass seeds soil **saturated** wherever it touches liquid or sits at or
/// below the water table -- so those blocks are pinned at 4.0, double the
/// stop, and any evaporating surface that reads one is switched off rather
/// than slowed.
///
/// Which surfaces read one is geometry: `dryness` samples the block one
/// above, spanning the eight rows 1 to 8 cells higher, so a column near the
/// top of its field block needs only a single cell of neighbouring relief to
/// be reading soil instead of air. Flat ground is unaffected; how much else
/// is, is what this counts. Split by surface kind because the water case is
/// intended and the soil case may not be.
///
/// Raised with the finding from the plant branch, whose flat per-preset soil
/// baseline widens the affected area from the wetted perimeter of a pond to
/// everywhere there is soil. The effect is not theirs -- it is already here
/// -- which is why it is measurable now.
#[derive(Clone, Copy, Debug, Default)]
pub struct DrynessCounts {
    /// Calls from a water or ice surface (`tick`).
    pub water_checks: u32,
    /// Of those, ones that read saturated air and so evaporated nothing.
    pub water_becalmed: u32,
    /// Calls from a damp soil surface (`tick_soil`).
    pub soil_checks: u32,
    /// Of those, ones that read saturated air.
    pub soil_becalmed: u32,
}

impl DrynessCounts {
    fn record(&mut self, soil: bool, dryness: f32) {
        let (checks, becalmed) = if soil {
            (&mut self.soil_checks, &mut self.soil_becalmed)
        } else {
            (&mut self.water_checks, &mut self.water_becalmed)
        };
        *checks += 1;
        if dryness <= 0.0 {
            *becalmed += 1;
        }
    }

    /// The share of soil surface checks that evaporated nothing because the
    /// air above them read saturated, or `None` if none were made.
    pub fn soil_becalmed_share(&self) -> Option<f64> {
        (self.soil_checks > 0).then(|| self.soil_becalmed as f64 / self.soil_checks as f64)
    }

    /// The same for water and ice surfaces, where a wide calm lake reading
    /// zero is the intended behaviour rather than a finding.
    pub fn water_becalmed_share(&self) -> Option<f64> {
        (self.water_checks > 0).then(|| self.water_becalmed as f64 / self.water_checks as f64)
    }
}

/// `dryness`, recording the reading. The counting wrapper rather than the
/// function itself, so the pure form stays callable from tests without
/// needing a `&mut World`.
fn dryness_counted(world: &mut World, x: i32, y: i32, soil: bool) -> f32 {
    let d = dryness(world, x, y);
    world.dryness_counts.record(soil, d);
    d
}

fn dryness(world: &World, x: i32, y: i32) -> f32 {
    let humidity = world.field_moisture_at(x, y - FIELD_SCALE);
    // **The square root is shaping, and is stated as tuning rather than as
    // physics.** Evaporative flux really is linear in the humidity deficit,
    // and linear is what this was first built as — but the deficit over a
    // puddle is a small fraction of the range (1.32 against a stop of 2.0),
    // which put a six-cell puddle at tens of thousands of frames to dry --
    // many in-game days, and minutes of continuous play. A puddle that
    // outlasts a week of weather is not a puddle.
    //
    // Taking the root lifts the shallow end of the range and leaves the deep
    // end almost alone — the puddle goes 0.09 -> 0.30, while a gale's already
    // near-maximal 0.885 only reaches 0.94. That is the right place to spend
    // it: dry air should evaporate fast whatever the shaping, and the case
    // that needed help was specifically air that is *nearly* saturated.
    // Raising `HUMID_STOP` instead would have done the same job to the puddle
    // and also lifted the calm lake off zero, which is the one reading that
    // has to stay exactly zero.
    ((HUMID_STOP - humidity) / HUMID_STOP).clamp(0.0, 1.0).sqrt()
}

/// How fast this surface dries for being warm or cold, as a multiplier on
/// the rate — the third factor, and **the one place in the engine that reads
/// a channel the sky oscillates without dividing the oscillation back out**.
///
/// # Why the raw temperature, when everything else takes the noon-equivalent
///
/// `CLAUDE.md`'s rule is that a channel which oscillates by design must be
/// divided out of *decisions*, and every other consumer obeys it:
/// `field.rs`'s moisture decay, the worm's heat threshold and its thermotaxis
/// all go through `field::noon_equivalent_temperature`. This one does the
/// opposite on purpose, because the diurnal signal *is the effect being
/// asked for* — warm days dry a puddle and cool nights barely touch it — and
/// there is nothing else in the reading to be aliased by. The rule's hazard
/// is a fixed threshold sampled at an arbitrary phase; this is not a
/// threshold, it is a continuous factor whose whole job is to follow the
/// phase.
///
/// **One site, and one site only.** Moisture decay stays noon-equivalent
/// (`field.rs`'s `step_diffusion`, and its comment says so from the other
/// end). If it went diurnal too, the day would reach evaporation twice —
/// once here directly, and once through a humidity channel that itself dried
/// out faster by day — and the two would compound into a swing nobody set
/// and nobody could tune, because `dryness` and this factor multiply.
///
/// # Linear, because linear is what keeps `FILL_PER_CHECK` meaning what it meant
///
/// `1 + slope * (T - ambient)`, clamped. The sky's forcing
/// (`field::sky_temperature_offset`) is a zero-mean cosine over a day, so a
/// factor *linear* in it has a day-mean of exactly 1.0 wherever the clamps do
/// not bite — the timescale `FILL_PER_CHECK` was set from survives per day
/// unchanged, and all the coupling does is redistribute a day's drying inside
/// the day.
///
/// **That is a measurement and not only an argument**
/// (`probe_whole_days_of_drying`, cell-equivalents credited over a whole
/// number of whole days, with the coupling off and at 0.15):
///
/// | basin | days | before | after | delta |
/// |---|---|---|---|---|
/// | 32-wide | 1 | 11.520 | 11.929 | +3.6% |
/// | 32-wide | 4 | 62.920 | 62.132 | -1.3% |
/// | 176-wide | 1 | 10.560 | 10.894 | +3.2% |
/// | 176-wide | 4 | 59.357 | 59.275 | -0.1% |
///
/// One day still carries a percent or three of phase (a run that starts
/// settled at noon has spent its settling frames in the warm half); four
/// days is within a tenth of a percent on the lake. So `FILL_PER_CHECK` was
/// re-derived and **did not move**, and neither did `HUMID_STOP` — the
/// humidity-against-width table in the module doc reads identically at 0.00
/// and at 0.15, digit for digit, which is what it should do given that
/// moisture decay is still on the noon-equivalent reading.
///
/// An Arrhenius or exponential shape is the physical one and was rejected for
/// exactly that reason: `exp(k * dT)` has a day-mean *above* 1.0 (Jensen),
/// so it silently speeds every body of water in the world up by an amount
/// that depends on the swing, and `FILL_PER_CHECK` would have to be
/// re-derived against a constant it does not otherwise care about. This
/// project does not want exactness (`CLAUDE.md`), and it does want a knob
/// that means the same thing after the change as before it.
///
/// # The clamps, and what actually reaches them
///
/// The sky alone cannot reach either: `SKY_TEMPERATURE_SWING` is 6 degrees
/// and attenuates with depth, so open-air day/night lands at 1.60 / 0.40.
/// What reaches them is everything else that writes field temperature.
/// `weather.rs`'s blizzard drives a chilled column up to `SNOW_CHILL` (26)
/// degrees below ambient, which without a floor would be a *negative* rate;
/// and fire's plume goes hundreds of degrees the other way. So:
///
/// * `WARMTH_FLOOR` is above zero, deliberately. A cold night, and a cold
///   snap far more so, should slow drying to a crawl and never stop it —
///   a hard zero here would give the mechanic a second permanent off-state,
///   which is the failure the module doc's "and then it stops" section
///   exists to keep out. Freezing is `fire.rs`'s job and happens to the
///   *cell*; this is the air over it.
/// * `WARMTH_CEIL` bounds a fire boiling a pond dry in a handful of checks.
///   It still does it fast — three times the ordinary rate — which is the
///   satisfying answer, and bounded so that a plume that briefly reads 400
///   degrees does not delete a lake between two frames.
fn warmth(world: &World, x: i32, y: i32) -> f32 {
    // **The water's own block, not the block above.** `dryness` reads one
    // block up because a water cell pins its own block's *moisture* to
    // saturation and so carries no signal there. Temperature has no such
    // degeneracy — the field's temperature channel is not forced by the
    // presence of water — and the block a surface actually sits in is the
    // air that is in contact with it.
    let here = world.field_at(x, y).temperature;
    (1.0 + (here - AMBIENT_TEMPERATURE as f32) * WARMTH_PER_DEGREE).clamp(WARMTH_FLOOR, WARMTH_CEIL)
}

/// How much standing water there is either side of `(x, y)`, `0.0..=1.0` —
/// the shelter half of the rate, and the half that survives a gale.
///
/// # Why this is here at all
///
/// The humidity above a body was the whole design, and on a calm world it is
/// enough: measured over settled water four rows deep, the air one block up
/// reads 1.32 over a six-cell puddle and 2.31 over a lake, so a threshold
/// between them separates the two without anything counting cells.
///
/// **Wind destroys that reading, and wind is a designed, frequent state.**
/// `weather::gust` fires every 26 frames for as long as the wind channel
/// stays over its threshold, which is most of a windy epoch — thousands of
/// frames. Traced on seed 12345: the channel crosses at frame 11460 and the
/// air over a lake goes 2.31 -> 0.23 within ten frames and stays there,
/// because advection back-traces two field blocks up into dry air faster
/// than diffusion can rebuild the humid layer. On the full 2048x640 world —
/// the size this was traced at; the world has since grown to 8192x2560 and
/// this has not been re-traced there — it is slower and no better in the
/// end — 2.31 -> 0.42 by frame 14000 with the lake down to 39% of its
/// volume, and an 800-cell lake behaves exactly like a 240-cell one. Every
/// lake in the world would go in a gale.
///
/// No function of humidity can fix that, because a gale mixes the whole
/// atmosphere: over a puddle and over a lake the air is equally dry, so a
/// puddle and a lake read *identically*. The shelter term therefore has to
/// come from something the wind cannot move, and `field::moisture_source_at`
/// is exactly that — rebuilt from the CA grid every frame by
/// `rebuild_blocked`, never advected, and already graded (standing liquid is
/// 1.0, damp soil a fraction, dry ground 0).
///
/// # Why this is still not a size measurement
///
/// It is a fixed-radius stencil, `SHELTER_REACH` field blocks either side at
/// the water's own height, and it knows nothing about *bodies*: no flood
/// fill, no connectivity, no total. It cannot tell a fifty-cell pond from an
/// ocean and does not need to. Two separate puddles a few cells apart
/// shelter each other, which is right and which a body-size measure would
/// get wrong, and a long thin sheet shelters itself exactly as much as a
/// deep one of the same width. Nor does it gate anything: it is a continuous
/// factor, so the outcome grades rather than switching.
///
/// If the gust field is ever retuned so that a windy epoch stops scouring
/// the humid layer, this term can go and the humidity deficit alone will do
/// the whole job again. It is a second factor precisely so that it can.
fn shelter(world: &World, x: i32, y: i32) -> f32 {
    let mut sum = 0.0;
    for i in -SHELTER_REACH..=SHELTER_REACH {
        sum += world.field_moisture_source_at(x + i * FIELD_SCALE, y);
    }
    (sum / (2 * SHELTER_REACH + 1) as f32).clamp(0.0, 1.0)
}

/// Dispatch a due `ActiveKind::Evaporate` site. `scheduler::step` never
/// routes any other `ActiveKind` here.
pub fn tick(world: &mut World, site: &ActiveSite) -> Vec<ActiveSite> {
    let ActiveKind::Evaporate { stale_ticks } = site.kind else {
        debug_assert!(false, "scheduler::step only routes ActiveKind::Evaporate here");
        return Vec::new();
    };
    let (x, y) = (site.x, site.y);
    let cell = world.get(x, y);

    // **One `ActiveKind` for both, dispatched on what the cell is.** A
    // second variant would need its own arm in `World::schedule_active_site`
    // and `pop_due_active_site` or it would get no dedup at all, and that
    // dedup is load-bearing for the *rate* and not only the frame cost
    // (`pending_evaporation`'s own doc). One kind, keyed by position, keeps
    // that property for free -- and a cell cannot be both a liquid surface
    // and damp soil, so there is nothing for the two paths to fight over.
    if world.materials.kind(cell.material) == MaterialKind::Powder {
        return tick_soil(world, x, y, stale_ticks);
    }

    // Structurally finished: whatever was here has flowed away, frozen,
    // burned off or been dug out. Nothing to reschedule — if liquid comes
    // back, the sweep is awake for it and schedules afresh.
    let evaporates = world.materials.get(cell.material).evaporates;
    if cell.managed() || !evaporates || world.materials.kind(cell.material) != MaterialKind::Liquid {
        return Vec::new();
    }

    // Still liquid, but covered. Keep checking for a little while in case
    // the cover is transient, then retire — see the module doc.
    if !world.get(x, y - 1).is_empty() {
        if stale_ticks + 1 >= STALE_LIMIT {
            return Vec::new();
        }
        return vec![ActiveSite {
            x,
            y,
            kind: ActiveKind::Evaporate { stale_ticks: stale_ticks + 1 },
            next_frame: world.frame + CHECK_INTERVAL,
        }];
    }

    let reschedule =
        ActiveSite { x, y, kind: ActiveKind::Evaporate { stale_ticks: 0 }, next_frame: world.frame + CHECK_INTERVAL };

    // The three factors multiply: dry air, an unsheltered surface, and warm
    // air over it. Either of the first two at zero stops evaporation, which
    // is what makes a lake safe in a gale and a puddle safe in a downpour —
    // and `warmth` is deliberately *not* one of those, because it is floored
    // above zero (see it) so that cold is a brake and never a stop.
    let rate = dryness_counted(world, x, y, false) * (1.0 - shelter(world, x, y)) * warmth(world, x, y);
    let loss = (FILL_PER_CHECK as f32 * rate) as u16;
    if loss == 0 {
        // Sheltered. Deliberately still on the schedule — see the module
        // doc's "and then it stops": this is a value, not a structure.
        return vec![reschedule];
    }

    let fill = liquid_fill(cell);
    if fill <= loss {
        // **`fill`, not `loss`** — the whole of what is left, because the
        // whole of what is left is what goes. Crediting `loss` here instead
        // is the silent leak this branch is built to avoid: it is correct on
        // any cell that happens to hold exactly `loss`, wrong by the
        // remainder on every other one, and the size of the error is a
        // function of how often puddles finish rather than of anything
        // physical. It would not show up as water disappearing; it would
        // show up, months later, as a world whose sky had quietly stopped
        // being able to rain.
        world.credit_atmosphere(fill);
        world.add_vapour(x, y, vapour_of(fill));
        // Gone. `Cell::EMPTY`, never `with_aux(0)` — on a `Liquid`, `aux ==
        // 0` means *full*, so writing a drained cell that way manufactures a
        // full one out of nothing.
        world.set(x, y, Cell::EMPTY);
        // The cell below is the new surface. Scheduled directly rather than
        // left for the sweep to notice: the sweep does get there too (this
        // write dirties the chunk), but a body that is only losing its top
        // cell and never moving otherwise should not depend on that to keep
        // going, and the dedup makes the overlap free.
        if is_exposed_surface(world, x, y + 1) {
            return vec![ActiveSite {
                x,
                y: y + 1,
                kind: ActiveKind::Evaporate { stale_ticks: 0 },
                next_frame: world.frame + CHECK_INTERVAL,
            }];
        }
        return Vec::new();
    }

    // The ordinary case: `loss` came off the cell, so `loss` goes into the
    // sky. The two credit sites are the only two places a fill reduction
    // happens in this file, and each credits exactly what it removed.
    world.credit_atmosphere(loss);
    world.add_vapour(x, y, vapour_of(loss));
    world.set(x, y, cell.with_aux(fill - loss));
    vec![reschedule]
}

#[cfg(test)]
#[path = "evaporation_tests.rs"]
mod tests;
