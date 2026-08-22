//! What the sky is doing, as a pure function of `(seed, frame)`.
//!
//! # Why this is a function and not a state machine
//!
//! Determinism is required (`PLAN.md`, same-build replay), and the cheapest
//! way to have it is to store nothing. There is no weather state to save,
//! restore, replay or desynchronise: two runs of the same seed agree about
//! the weather at frame 400,000 without either of them having simulated the
//! 399,999 frames in between. `ParticleSystem::rng` is explicitly *not*
//! reproducible, which is exactly the trap this avoids by having no stream
//! at all — every draw here is position-tagged through [`rng::stream`], the
//! same discipline `worldgen::noise` uses.
//!
//! It also means anything that wants to know what the weather *will* be, or
//! *was*, just asks. A forecast is `at(seed, frame + n)`.
//!
//! # Weather is an event, not noise
//!
//! The single most important shaping decision here. A `unit()` per frame
//! would give a sky that flickers between wet and dry and reads as static
//! rather than as weather. So the channels are smooth noise interpolated
//! across **epochs of several days**, and precipitation is what happens when
//! the wet channel crosses a high threshold — which is most of the time
//! nothing at all. A world should be mostly clear, so that rain is an
//! occasion.
//!
//! The consequence to keep in mind when reading a test: at a given frame the
//! answer is usually `Precipitation::None`, and a test that wants rain has to
//! go looking for a frame that has some (see `first_frame_with`).

use super::cell::{Cell, AMBIENT_TEMPERATURE};
use super::chunk::Rect;
use super::field::DAY_NIGHT_PERIOD_FRAMES;
use super::material::{self, MaterialKind};
use super::rng;
use super::world::World;

/// How long one weather epoch lasts, in frames.
///
/// Two full days. The channels below are interpolated *between* epochs, so
/// this is the timescale over which the sky changes its mind, not how long a
/// front lasts — a rain event is the part of an epoch spent above the
/// threshold and is typically a good deal shorter.
///
/// Set from what it feels like rather than from meteorology: much shorter and
/// weather is scenery that flickers past; much longer and a player can run a
/// whole session without seeing any.
pub const WEATHER_EPOCH_FRAMES: u64 = DAY_NIGHT_PERIOD_FRAMES * 2;

/// Below this, the wet channel produces nothing. Sets how much of the time a
/// world is clear — with smooth noise either side of it, a fairly high bar
/// still leaves rain a regular occurrence, because the channel spends time
/// crossing as well as sitting.
const PRECIPITATION_THRESHOLD: f32 = 0.80;

/// Above this on the chill channel, precipitation falls as snow.
const SNOW_THRESHOLD: f32 = 0.70;

/// What is falling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precipitation {
    None,
    Rain,
    Snow,
}

/// The state of the sky at one instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weather {
    /// How hard it is coming down, `0.0` (nothing) to `1.0` (downpour).
    /// Zero exactly when `kind` is [`Precipitation::None`].
    pub intensity: f32,
    pub kind: Precipitation,
    /// Signed wind strength, `-1.0` (hard from the east) through `0.0` (still)
    /// to `1.0` (hard from the west). Signed rather than a magnitude plus a
    /// direction so that it can be interpolated without the direction
    /// flipping discontinuously through calm.
    pub wind: f32,
    /// How cold the air mass is, `0.0` (mild) to `1.0` (hard freeze). Read
    /// on its own as well as through `kind`: cold without precipitation is
    /// still cold, and is what makes a clear winter night different from a
    /// clear summer one.
    pub chill: f32,
}

impl Weather {
    /// Nothing happening. The state a world is in most of the time, and the
    /// one every "does this stay settled" test asserts against.
    pub const CLEAR: Weather = Weather { intensity: 0.0, kind: Precipitation::None, wind: 0.0, chill: 0.0 };

    /// Whether anything is falling.
    pub fn is_precipitating(&self) -> bool {
        self.kind != Precipitation::None
    }
}

/// A smooth channel in `0.0..=1.0`, interpolated across weather epochs.
///
/// `tag` separates channels that share a seed and a frame — the same job
/// `worldgen::noise::Purpose` does, and for the same reason: without it the
/// wet channel and the chill channel would be the same curve, and it would
/// snow exactly when and only when it rained hardest.
fn channel(seed: u64, frame: u64, tag: u64) -> f32 {
    let epoch = frame / WEATHER_EPOCH_FRAMES;
    let t = (frame % WEATHER_EPOCH_FRAMES) as f32 / WEATHER_EPOCH_FRAMES as f32;
    let at = |e: u64| rng::stream(seed, tag, e, 0).next_u64() as f32 / u64::MAX as f32;
    // Smoothstep rather than a straight lerp: the derivative is zero at each
    // epoch boundary, so a front arrives and departs gradually instead of
    // reversing on a corner. A linear ramp is visible as a kink in the rain
    // intensity, which reads as a bug in the renderer rather than as weather.
    let s = t * t * (3.0 - 2.0 * t);
    at(epoch) * (1.0 - s) + at(epoch + 1) * s
}

/// The sky at `frame`, for a world built from `seed`.
///
/// Pure: no state, no order dependence, no allocation. Call it as often as
/// you like, in any order, from any thread.
pub fn at(seed: u64, frame: u64) -> Weather {
    let wet = channel(seed, frame, 1);
    let chill = channel(seed, frame, 2);
    // Wind is bipolar and is *not* gated on precipitation: a dry blustery day
    // is a real thing, and the gust machinery reads this channel whether or
    // not anything is falling.
    let wind = channel(seed, frame, 3) * 2.0 - 1.0;

    if wet <= PRECIPITATION_THRESHOLD {
        // Wind and chill survive; only the falling stops. Returning
        // `Weather::CLEAR` here instead would make every dry frame perfectly
        // still, which is most frames.
        return Weather { intensity: 0.0, kind: Precipitation::None, wind, chill };
    }
    // Rescaled so intensity climbs from zero *at* the threshold rather than
    // stepping to a finite value the instant it is crossed. Rain that begins
    // at full strength is the same corner problem the smoothstep above
    // exists to avoid, one level up.
    let intensity = ((wet - PRECIPITATION_THRESHOLD) / (1.0 - PRECIPITATION_THRESHOLD)).clamp(0.0, 1.0);
    let kind = if chill > SNOW_THRESHOLD { Precipitation::Snow } else { Precipitation::Rain };
    Weather { intensity, kind, wind, chill }
}

/// How much the weather has moved between two frames, as a single scalar.
///
/// The escape hatch for `field::step`'s settled-world early-out, which is
/// otherwise a trap this whole feature walks into: a world with nothing
/// moving in it stops solving its field, and weather that only ever *writes*
/// to the field would then never get the chance to. `field.rs` already
/// solves exactly this shape for the day/night cycle (`amplitude_changed`),
/// and this is the same test for the same reason.
///
/// Deliberately **not** wired into `field::step` by this commit. Waking the
/// field costs real frame time, and until something actually writes weather
/// into it there is nothing to wake *for* — the wiring lands with rain, next
/// to the write it exists to serve. Recorded here rather than in a plan,
/// because the wrong version of this is the most likely way weather undoes
/// the per-tile field sleeping this push just paid for.
pub fn drift(seed: u64, from: u64, to: u64) -> f32 {
    let a = at(seed, from);
    let b = at(seed, to);
    // Intensity dominates because it is what writes; wind matters because
    // gusts push the pressure field; chill matters least and moves slowest.
    (a.intensity - b.intensity).abs() + (a.wind - b.wind).abs() * 0.5 + (a.chill - b.chill).abs() * 0.25
}

/// How many columns one frame of the heaviest possible downpour touches.
///
/// A hard cap on work, not a rate: rain must cost the same whether the world
/// is one screen wide or thirty, or weather becomes the reason big worlds are
/// slow. Each touched column also unsettles a field tile, so this is
/// simultaneously the bound on how much field solving a storm can provoke.
const MAX_COLUMNS_PER_FRAME: f32 = 24.0;

/// The world width `MAX_COLUMNS_PER_FRAME` describes.
///
/// Without this, a fixed column count is a fixed *rate*, so rain gets denser
/// the narrower the world: 24 columns a frame across 128 columns soaks every
/// one of them every five frames, and a test world drowned under 2232 water
/// cells while the 2048-wide world that shipped at the time this was
/// measured saw an ordinary shower. The number that should be constant is
/// drops per column per second; the cap then bounds work on worlds larger
/// than this, at the price of rain that thins out on them -- which is the
/// right trade, because frame cost is a hard constraint and rain density is
/// a feel. The world that ships now (8192 wide) is already well past this
/// reference width, so it is already living in the "thinned out, capped at
/// `MAX_COLUMNS_PER_FRAME`" regime this paragraph describes rather than the
/// linear-scaling one below it -- deliberately, not an oversight, but worth
/// knowing before reading the arithmetic below as if it still applied at
/// 1:1.
const REFERENCE_WIDTH: f32 = 2048.0;

/// How wet one drop makes the ground it lands on.
///
/// Large, because a drop here is not a raindrop -- it is *this field cell got
/// rained on*, and the channel it writes is a saturation fraction that
/// evaporation is continuously pulling back down. The first value (0.10) was
/// chosen as if drops accumulated; they do not, they reach a balance against
/// evaporation, and that balance measured **0.031 of saturation across a
/// whole 128-wide world** -- real, conserved, and completely invisible. What
/// matters is the steady state under continuous rain, not the size of one
/// write.
const MOISTURE_PER_DROP: f32 = 0.45;

/// Chance that a landing drop is a real water cell rather than only a
/// moisture write, at full intensity.
///
/// Low on purpose. Most rain should soak in and be *seen* as darker, damper
/// ground, because that is what rain mostly does and because a water cell
/// per drop would flood a world in a minute. Soil consumes a landing water
/// cell outright when it has capacity (`update.rs`'s infiltration), so the
/// cells that survive are the ones landing on rock -- which is exactly where
/// puddles and runoff belong.
const WATER_CELL_CHANCE: f32 = 0.06;

/// How much a drop raises the landing cell's own saturation, at full
/// intensity, in the units `material::SOIL_SATURATED` is expressed in.
///
/// A tenth of saturation, so ground darkens over a few passes of a shower
/// rather than going instantly black under the first drop.
const SOIL_SOAK_PER_DROP: u16 = material::SOIL_SATURATED / 10;

/// Chance a landing flake becomes a snow cell, at full intensity.
///
/// Far higher than rain's, because the two are doing opposite things: rain
/// soaks in and is *seen* as darker ground, snow does not soak at all and is
/// seen only as the drift it builds. Snow that mostly failed to land would
/// be a snowstorm that leaves no snow.
const SNOW_CELL_CHANCE: f32 = 0.55;

/// How cold a snowfall makes the ground it is falling on, in degrees below
/// ambient, at full intensity.
///
/// Snow's melting point is *below* ambient, so without this every flake
/// melts the instant it lands and a blizzard produces a damp hillside. The
/// cold is delivered as a negative `add_heat`, which already existed and
/// needed no new mechanism -- and the thaw needs none either, because when
/// the front passes this stops being applied, the field warms back to
/// ambient, and the existing upward phase change turns every drift to
/// meltwater on its own.
const SNOW_CHILL: f32 = 26.0;

/// How deep into an existing drift a snowfall keeps the cold going.
const SNOW_CHILL_DEPTH: i32 = 6;

/// Wind strength past which gusts start firing.
///
/// Below it the air is moving but not eventfully, and a world should not be
/// paying for pressure impulses on an ordinary breezy afternoon.
const GUST_THRESHOLD: f32 = 0.45;

/// Frames between gusts at full wind. Gusts are *events* -- a squall
/// arriving, not a constant push -- which is what keeps this on the right
/// side of the line described at `gust`.
const GUST_INTERVAL: u64 = 26;

/// Pressure delivered by one gust at full wind.
const GUST_STRENGTH: f32 = 34.0;

/// Radius of a gust, in world cells.
const GUST_RADIUS: i32 = 26;

/// How far down a drop soaks, in cells.
///
/// Rain wet only the single topmost cell in the first version, which is
/// defensible as physics and useless as a picture: the soil blanket is tens
/// of cells thick, so a rainstorm darkened a one-pixel line along the top of
/// it and the paired wet/dry renders were indistinguishable. Soaking a few
/// cells down is both what rain does and what makes wet ground read as wet.
/// Each cell down gets less, so the profile is damp at the surface fading
/// with depth rather than a uniform slab -- a slab reads as a different
/// material lying on top of the soil.
const SOAK_DEPTH: i32 = 5;

/// How often a storm gets a chance at a strike, in frames. Roughly every
/// four seconds at 60Hz; whether the chance is taken is a separate roll, so
/// strikes cluster and gap rather than arriving on a metronome.
const STRIKE_WINDOW: u64 = 240;

/// How long one strike's flash lasts, in frames.
const STRIKE_FRAMES: u64 = 14;

/// Below this intensity a storm has rain but no lightning. Lightning is the
/// top of the scale, not a feature of every shower.
const STRIKE_MIN_INTENSITY: f32 = 0.55;

/// A lightning strike in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strike {
    /// Frames since the flash began, `0..STRIKE_FRAMES`.
    pub age: u64,
    /// Where it comes down, in world x.
    pub x: i32,
    /// Brightness of the flash *this frame*, `0.0..=1.0`. Already includes
    /// the decay and the re-strike below, so a caller just multiplies by it.
    pub flash: f32,
    /// Distinguishes one strike from the next, so the bolt drawn for it is
    /// its own shape rather than the same zigzag every time.
    pub id: u64,
}

/// The strike happening at `frame`, if any.
///
/// Pure, like the rest of this module: no strike list, no timers, and a
/// replay of the same seed puts the lightning in the same place on the same
/// frame. Storms only -- see `STRIKE_MIN_INTENSITY`.
pub fn strike(seed: u64, frame: u64, bounds: Option<Rect>) -> Option<Strike> {
    let bounds = bounds?;
    // Which window we might be in the flash of -- including the previous one,
    // since a flash that began near the end of a window is still lit in this
    // one.
    let window = frame / STRIKE_WINDOW;
    // **`checked_sub`, not `wrapping_sub`.** Inside the very first window
    // there is no previous one, and wrapping to `u64::MAX` made
    // `w * STRIKE_WINDOW` below overflow -- a panic in the opening four
    // seconds of any world, on any seed whose `u64::MAX` window happens to
    // pass the 0.55 roll.
    //
    // It hid for as long as it did because release builds do not check
    // overflow: every `cargo test --release` run was green, and only a debug
    // build ever saw it. Reach for a plain `cargo test` when touching
    // arithmetic that can run near a boundary.
    for w in [Some(window), window.checked_sub(1)].into_iter().flatten() {
        let mut r = rng::stream(seed, 0x4C49_4748, w, 0);
        // Two rolls: whether this window strikes at all, and when inside it.
        if !r.chance(0.55) {
            continue;
        }
        let start = w * STRIKE_WINDOW + r.next_u64() % (STRIKE_WINDOW - STRIKE_FRAMES);
        if frame < start || frame >= start + STRIKE_FRAMES {
            continue;
        }
        // The weather at the moment it *began*, so a strike is not cut off
        // mid-flash by the front easing below the threshold.
        let at_start = at(seed, start);
        if at_start.kind != Precipitation::Rain || at_start.intensity < STRIKE_MIN_INTENSITY {
            continue;
        }
        let age = frame - start;
        let width = (bounds.max_x - bounds.min_x + 1) as u64;
        let x = bounds.min_x + (r.next_u64() % width) as i32;
        // Two peaks, not one. A single decaying flash reads as a light being
        // switched off; real lightning strobes, and the second, weaker return
        // stroke is most of what makes it read as lightning rather than as a
        // screen flicker.
        let t = age as f32;
        let main = (1.0 - t / 5.0).max(0.0);
        let ret = (1.0 - (t - 5.0).abs() / 3.0).max(0.0) * 0.55;
        let tail = (1.0 - t / STRIKE_FRAMES as f32).max(0.0) * 0.12;
        return Some(Strike { age, x, flash: (main.max(ret) + tail).min(1.0), id: w });
    }
    None
}

/// Wind, delivered as gusts.
///
/// # The mistake this is shaped around
///
/// A **steady** global wind was built here once, measured, and reverted. A
/// uniform velocity term in a bounded world pushes air into the walls, which
/// creates divergence, which creates pressure, which creates more velocity:
/// `field::is_converged` never returns true again. Settled-field cost went
/// from 0.0002 ms to **3.55 ms on every scene in the engine, permanently**,
/// and six field tests failed. It is recorded as do-not-retry and this is
/// not a quieter version of it.
///
/// A gust is the opposite shape. It is a bounded impulse at one place at one
/// moment: the field disperses it, reaches equilibrium, and goes back to
/// sleep. The world is disturbed while the squall passes and settles after
/// it, which is both what weather does and what the field is built to
/// handle. `a_gust_settles_again` is the guard, and it exists specifically
/// to catch anyone reintroducing the steady term.
///
/// Everything downstream is free: `update_gas`'s bias blows smoke, and
/// `organism::wind_lean_dir` leans trees, both off the field this writes to.
fn gust(world: &mut World, w: Weather) {
    if w.wind.abs() < GUST_THRESHOLD {
        return;
    }
    // Fires on a schedule rather than every frame. A gust every frame is a
    // steady wind wearing a different hat, and would rediscover the whole
    // problem above.
    if !world.frame.is_multiple_of(GUST_INTERVAL) {
        return;
    }
    let Some(bounds) = world.bounds() else { return };
    let mut r = rng::stream(world.seed, 0x4755_5354, world.frame, 0);
    let width = (bounds.max_x - bounds.min_x + 1) as u64;
    let x = bounds.min_x + (r.next_u64() % width) as i32;
    // Somewhere in the upper air, where there is room for it to disperse.
    // A gust delivered inside rock is absorbed by `blocked` field cells and
    // is simply wasted work.
    let span = bounds.max_y - bounds.min_y;
    let y = bounds.min_y + span / 8 + (r.next_u64() % (span / 4).max(1) as u64) as i32;
    // **A dipole, not a single blob.** A lone positive impulse injects net
    // pressure into a closed world, and there is nowhere for it to go: it
    // drives velocity, velocity drives advection, and the tiles around it
    // never reconverge. Measured -- a calm world left 0 field tiles
    // unconverged and a gusty one left 4, permanently, which is the reverted
    // steady wind's exact failure arriving by another route.
    //
    // What a gust physically is, is air moving *from* somewhere *to*
    // somewhere. High pressure behind and low pressure ahead sums to zero,
    // pushes the air between them along the wind, and leaves the field with
    // nothing left over to keep solving once it has equalised.
    let lead = (GUST_RADIUS as f32 * 1.5 * w.wind.signum()) as i32;
    world.add_pressure_impulse(x, y, GUST_RADIUS, GUST_STRENGTH * w.wind.abs());
    world.add_pressure_impulse(x + lead, y, GUST_RADIUS, -GUST_STRENGTH * w.wind.abs());
}

/// One frame of weather acting on the world.
///
/// Called by both drivers (`parallel::step` and `update::step`), because
/// behaviour only the player sees is behaviour only the parallel driver
/// produces -- and a headless harness that silently had no weather would be
/// the worst possible way to measure it.
///
/// # Where rain is simulated
///
/// **Where it lands, not where it falls.** Nothing is simulated in the air:
/// there are no falling drops, no particles, and no writes over the sky
/// column. That is a correctness requirement rather than an optimisation --
/// the field sleeps per tile now, and a write anywhere in a sky column would
/// wake every tile between the cloud and the ground, undoing it. What the
/// player sees falling is drawn, not simulated.
pub fn step(world: &mut World) {
    let w = at(world.seed, world.frame);
    // Wind is not gated on precipitation -- a dry gale is weather too, and
    // the wind channel is generated whether or not anything is falling.
    gust(world, w);
    if !w.is_precipitating() {
        return;
    }
    let Some(bounds) = world.bounds() else { return };

    let width = bounds.max_x - bounds.min_x + 1;
    // Fractional columns are resolved by chance rather than rounded up: at
    // low intensity on a small world the honest answer is "less than one
    // column this frame", and rounding that to one is how a drizzle becomes
    // a downpour on a test world.
    let wanted = (w.intensity * MAX_COLUMNS_PER_FRAME * width as f32 / REFERENCE_WIDTH).min(MAX_COLUMNS_PER_FRAME);
    let mut pick = rng::stream(world.seed, 0x5241_494E, world.frame, u64::MAX);
    let columns = wanted.floor() as usize + usize::from(pick.chance(wanted.fract()));
    if columns == 0 {
        return;
    }
    // Hoisted: `id_of` is a string hash, and the sweep must not pay one per
    // drop. `None` means this world has no water material at all, in which
    // case rain still wets the ground and simply never puddles.
    let water = world.materials.id_of("water");
    let snow = world.materials.id_of("snow");
    let soak = (SOIL_SOAK_PER_DROP as f32 * w.intensity) as u16;

    for i in 0..columns {
        // Position drawn from `(seed, frame, i)` rather than a stream, so the
        // same frame of the same world rains on the same columns however many
        // times it is replayed, and two drops in one frame never collide by
        // sharing a generator's state.
        let mut r = rng::stream(world.seed, 0x5241_494E, world.frame, i as u64);
        let x = bounds.min_x + (r.next_u64() % width as u64) as i32;
        let Some(surface_y) = surface_under_sky(world, x, bounds.min_y, bounds.max_y) else {
            continue;
        };
        // Rain only. Snow sits on the surface and wets nothing until it
        // melts -- at which point it becomes water and the ordinary
        // infiltration path takes over, which is both correct and one less
        // mechanism to write.
        if w.kind == Precipitation::Rain {
            world.add_moisture(x, surface_y, 1, MOISTURE_PER_DROP * w.intensity);
        }
        // ...and the *cell's own* saturation, which is a different channel
        // from the field's humidity and is the one that matters twice over:
        // roots read it (`update::soil_moisture`), and it is what makes wet
        // ground look wet. Writing only the field left rain with a real,
        // conserved, entirely invisible effect.
        //
        // `aux == 0` on a Powder means **dry** -- the opposite of the
        // convention on a Liquid, where 0 means full. Getting this backwards
        // does not merely mis-shade a cell, it hands every root in the world
        // a full drink.
        for d in 0..if w.kind == Precipitation::Rain { SOAK_DEPTH } else { 0 } {
            let y = surface_y + d;
            if y > bounds.max_y {
                break;
            }
            let cell = world.get(x, y);
            // Stops at the first thing that cannot hold water, so a puddle
            // on bare rock does not wet the rock beneath it and a thin soil
            // cap over stone soaks only as deep as the soil goes.
            if world.materials.get(cell.material).water_capacity == 0 {
                break;
            }
            let share = soak / (d as u16 + 1);
            let wetter = cell.aux().saturating_add(share).min(material::SOIL_SATURATED);
            if wetter != cell.aux() {
                world.set(x, y, cell.with_aux(wetter));
            }
        }
        // **Rain does not stack on standing water.** If the topmost thing in
        // this column is already a liquid, the drop joins it as moisture and
        // nothing new is spawned. Without this the column refills as fast as
        // the puddle drains and a long storm grows water without bound --
        // measured at 2232 cells in a 128-wide world, which is seventeen rows
        // of flood. Rain landing in a lake making the lake deeper is
        // technically true and is not what anyone wants to look at.
        if w.kind == Precipitation::Snow {
            // The field's temperature channel *and* the cells' own. Only the
            // second one decides melting -- `fire::update` compares
            // `cell.temperature` against `melting_point`, and the field is a
            // separate coarse channel that does not feed it. Writing only
            // the field was the first version and every flake melted on the
            // frame it landed, flooding the surface with meltwater: a
            // snowstorm that produced a lake.
            world.add_heat(x, surface_y, 2, -SNOW_CHILL * w.intensity * w.chill);
            let cold = (AMBIENT_TEMPERATURE as f32 - SNOW_CHILL * w.intensity.max(0.4)) as i16;
            // The drift itself is re-chilled, not just the arriving flake.
            // A pile kept cold only at the moment of landing warms from the
            // ground up between flakes and rots away underneath, so the
            // storm has to hold the whole column it is falling on.
            for d in 0..SNOW_CHILL_DEPTH {
                let y = surface_y + d;
                if y > bounds.max_y {
                    break;
                }
                let cell = world.get(x, y);
                if cell.material != snow.unwrap_or(material::EMPTY) {
                    break;
                }
                if cell.temperature() > cold {
                    world.set(x, y, cell.with_temperature(cold));
                }
            }
            if let Some(snow) = snow {
                if r.chance(SNOW_CELL_CHANCE * w.intensity) && surface_y > bounds.min_y {
                    let above = surface_y - 1;
                    if world.get(x, above).material == material::EMPTY {
                        world.set(x, above, Cell::new(snow, 0).with_temperature(cold));
                    }
                }
            }
            continue;
        }
        let is_liquid = world.materials.kind(world.get(x, surface_y).material) == MaterialKind::Liquid;
        if let Some(water) = water {
            if !is_liquid && r.chance(WATER_CELL_CHANCE * w.intensity) && surface_y > bounds.min_y {
                let above = surface_y - 1;
                if world.get(x, above).material == material::EMPTY {
                    world.set(x, above, Cell::new(water, 0));
                }
            }
        }
    }
}

/// The topmost solid cell in column `x`, or `None` if the column is empty all
/// the way down.
///
/// Walks from the top, so the answer is by construction the first thing the
/// sky can see -- a cell under an overhang or inside a cave is never
/// returned, which is what makes "rain does not fall indoors" fall out of
/// the geometry rather than needing a roof test of its own.
fn surface_under_sky(world: &World, x: i32, min_y: i32, max_y: i32) -> Option<i32> {
    (min_y..=max_y).find(|&y| world.get(x, y).material != material::EMPTY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::parallel;

    /// The opening frames of a world, where `window` is 0 and the "previous
    /// window" probe has nowhere to go.
    ///
    /// **Only fails in a debug build**, which is the point of it: this
    /// panicked on `w * STRIKE_WINDOW` overflowing after `0u64.wrapping_sub(1)`,
    /// and every release-mode run of the suite passed straight over it
    /// because release does not check overflow. Swept across seeds because
    /// the multiply is behind a 0.55 roll, so any single seed has a fair
    /// chance of never reaching it.
    #[test]
    fn a_strike_in_the_opening_frames_does_not_overflow() {
        let bounds = Some(Rect::new(0, 0, 511, 319));
        for seed in 0..256u64 {
            for frame in 0..STRIKE_WINDOW * 2 {
                let _ = strike(seed, frame, bounds);
            }
        }
    }

    /// A frame at which `seed` is raining, so a test can assert about rain
    /// rather than about the 86% of frames that are clear.
    fn a_rainy_frame(seed: u64) -> u64 {
        (0..WEATHER_EPOCH_FRAMES * 40)
            .step_by(30)
            .find(|&f| at(seed, f).kind == Precipitation::Rain && at(seed, f).intensity > 0.3)
            .expect("no seed used by these tests should be rainless")
    }

    /// A world with a stone shelf across the middle and a soil floor beneath
    /// it, so the same run has both an exposed surface and a roofed one.
    fn sheltered_world(seed: u64, frame: u64) -> World {
        let mut w = World::new(Rect::new(0, 0, 127, 127));
        w.seed = seed;
        w.frame = frame;
        let soil = w.materials.id_of("soil").expect("soil must exist");
        for x in 0..128 {
            w.set(x, 100, Cell::new(soil, 0));
        }
        // A roof over the right half only, with a gap under it. Rain must
        // reach the left half's soil and nothing under the roof.
        for x in 64..128 {
            w.set(x, 60, Cell::new(material::STONE, 0));
        }
        w
    }

    /// The first frame at or after `from` whose weather satisfies `f`, within
    /// a generous budget. Most frames are clear, so nearly every test here
    /// needs to go and find its case rather than assert about frame 0.
    fn first_frame_with(seed: u64, from: u64, f: impl Fn(Weather) -> bool) -> Option<u64> {
        // Stepped rather than exhaustive: the channels are smooth over
        // epochs, so anything that lasts less than a few hundred frames is
        // not a weather event in the first place.
        (from..from + WEATHER_EPOCH_FRAMES * 64).step_by(60).find(|&frame| f(at(seed, frame)))
    }

    /// The guard this feature actually needs: a gust's disturbance must go
    /// away, because that is the *only* thing separating it from the steady
    /// wind term that was built here, measured at a permanent 3.55 ms/frame
    /// on every scene, and reverted.
    ///
    /// Measured as **unconverged field tiles**, which is the quantity
    /// `field::step` branches on. Three earlier attempts asked it through
    /// summed pressure and each measured something else: an unsupported slab
    /// falling (~16000 in both the windy and calm run), a world's
    /// construction transient, and a background relaxation an order of
    /// magnitude larger than the gust -- under which "never dispersed" and
    /// "dispersed slowly" are the same reading. Tile convergence has none of
    /// that noise: a tile is converged or it is not.
    #[test]
    fn lightning_only_happens_in_storms_and_is_reproducible() {
        let b = Some(Rect::new(0, 0, 511, 319));
        let seed = 4;
        let strikes: Vec<u64> = (0..WEATHER_EPOCH_FRAMES * 20)
            .filter(|&f| strike(seed, f, b).is_some_and(|s| s.age == 0))
            .collect();
        println!("{} strikes over 20 epochs", strikes.len());
        assert!(!strikes.is_empty(), "a storm world never produced a single strike");
        for &f in &strikes {
            let w = at(seed, f);
            assert_eq!(w.kind, Precipitation::Rain, "lightning out of a sky that was not raining, at frame {f}");
            assert!(w.intensity >= STRIKE_MIN_INTENSITY, "lightning in a drizzle ({}) at frame {f}", w.intensity);
        }
        // Pure, like everything else here: the same frame is the same strike.
        for &f in strikes.iter().take(4) {
            assert_eq!(strike(seed, f, b), strike(seed, f, b));
        }
        // ...and a flash lasts a bounded time rather than latching on. A
        // strike that never ended would leave the world permanently white
        // *and* permanently repainting, since a live strike forces a full
        // redraw.
        for &f in strikes.iter().take(4) {
            assert!(
                strike(seed, f + STRIKE_FRAMES, b).is_none_or(|s| s.age == 0),
                "a flash was still lit a full duration after it began"
            );
        }
    }

    #[test]
    fn a_clear_sky_never_flashes() {
        // The control: whatever the strike rate is, it must be exactly zero
        // when nothing is happening, or storms are not what causes lightning.
        let b = Some(Rect::new(0, 0, 511, 319));
        let dry: Vec<u64> = (0..WEATHER_EPOCH_FRAMES * 20).filter(|&f| !at(4, f).is_precipitating()).collect();
        assert!(!dry.is_empty(), "seed 4 is never dry, so this proves nothing");
        assert!(
            dry.iter().all(|&f| strike(4, f, b).is_none()),
            "lightning struck out of a clear sky"
        );
    }

    #[test]
    fn a_gust_disperses() {
        let seed = 4;
        // The control, run first: the same scene at a *calm* frame. Whatever
        // this leaves unconverged is the field's own floor and is not
        // something a gust did -- asking a new metric what it reads when
        // nothing is wrong, before trusting it about a case that is.
        let calm_floor = gust_residue(seed, false);
        let windy = (0..WEATHER_EPOCH_FRAMES * 40)
            .step_by(30)
            .find(|&f| {
                let w = at(seed, f);
                !w.is_precipitating() && w.wind.abs() > GUST_THRESHOLD + 0.2
            })
            .expect("some dry windy frame exists");
        // A stable scene containing nothing but the thing under test -- no
        // unsupported geometry, whose collapse would disturb the field far
        // more than any gust.
        let mut w = World::new(Rect::new(0, 0, 127, 127));
        w.seed = seed;
        w.frame = windy;
        for x in 0..128 {
            for y in 100..128 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for _ in 0..400 {
            parallel::step(&mut w);
            w.step_active_sites();
            w.step_fields();
        }

        // Gusts fire on an interval, so this has to span several of them --
        // one impulse dispersing is a weaker claim than the storm's whole
        // rhythm failing to accumulate.
        let mut peak = 0;
        for _ in 0..GUST_INTERVAL * 6 {
            parallel::step(&mut w);
            w.step_fields();
            peak = peak.max(w.unsettled_field_tiles());
        }
        assert!(peak > 0, "gusts never unsettled a single tile; this test would pass on a wind that was never connected");

        // Now nothing but the field relaxing: no more weather, no more CA.
        // A self-driving term stays unconverged here forever, which is
        // precisely how the reverted wind behaved.
        let mut solves = 0;
        while w.unsettled_field_tiles() > 0 && solves < 3000 {
            w.step_fields();
            solves += 1;
        }
        let left = w.unsettled_field_tiles();
        println!("gusty: up to {peak} tiles unsettled, {left} left after {solves} solves; calm floor {calm_floor}");
        assert!(
            left <= calm_floor,
            "a gust left {left} field tiles unconverged against a calm floor of {calm_floor} --              the disturbance is feeding itself, which is the signature of the reverted              self-driving wind term and means storms cost frame time forever"
        );
    }

    /// Tiles still unconverged after the same scene, the same number of
    /// frames and the same relaxation, with or without wind.
    fn gust_residue(seed: u64, windy: bool) -> usize {
        let frame = (0..WEATHER_EPOCH_FRAMES * 40)
            .step_by(30)
            .find(|&f| {
                let w = at(seed, f);
                !w.is_precipitating() && (w.wind.abs() > GUST_THRESHOLD + 0.2) == windy
            })
            .expect("a dry frame of each kind exists");
        let mut w = World::new(Rect::new(0, 0, 127, 127));
        w.seed = seed;
        w.frame = frame;
        for x in 0..128 {
            for y in 100..128 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for _ in 0..400 {
            parallel::step(&mut w);
            w.step_active_sites();
            w.step_fields();
        }
        for _ in 0..GUST_INTERVAL * 6 {
            parallel::step(&mut w);
            w.step_fields();
        }
        let mut solves = 0;
        while w.unsettled_field_tiles() > 0 && solves < 3000 {
            w.step_fields();
            solves += 1;
        }
        w.unsettled_field_tiles()
    }

    #[test]
    fn rain_wets_exposed_ground_and_not_ground_under_a_roof() {
        // The claim that makes rain read as rain rather than as a global
        // moisture tick. Paired within one run -- roofed against exposed, in
        // the same world, same frames, same weather -- so it cancels
        // everything the rule under test is not about.
        let seed = 4;
        let start = a_rainy_frame(seed);
        let mut w = sheltered_world(seed, start);
        // Summed over each half rather than sampled at one column. Rain
        // lands on scattered columns, so a single-column reading is a
        // coin-flip on whether that column was picked -- the metric has to
        // count the quantity the claim is about, which is how wet the
        // exposed ground *is*, not whether one cell of it got hit.
        let wetness = |w: &World, from: i32, to: i32| {
            (from..to).step_by(8).map(|x| w.field_at(x, 100).moisture).sum::<f32>()
        };
        let exposed_before = wetness(&w, 0, 64);
        let roofed_before = wetness(&w, 64, 128);
        for _ in 0..400 {
            parallel::step(&mut w);
        }
        let exposed_after = wetness(&w, 0, 64);
        let roofed_after = wetness(&w, 64, 128);
        println!(
            "exposed half {exposed_before:.3} -> {exposed_after:.3}, roofed half {roofed_before:.3} -> {roofed_after:.3}"
        );
        assert!(exposed_after > exposed_before + 0.05, "400 frames of rain did not wet exposed ground");
        assert!(
            roofed_after <= roofed_before + 0.001,
            "ground under a roof got wetter ({roofed_before:.3} -> {roofed_after:.3});              rain is reaching through solid rock"
        );
    }

    #[test]
    fn a_long_storm_does_not_flood_the_world() {
        // The runaway this feature is most likely to produce, and the reason
        // water cells are a low-probability event rather than a drop per
        // column. Counts *cells*, not events: a failure count is not a
        // damage count, and what floods a world is standing water.
        let seed = 4;
        let start = a_rainy_frame(seed);
        let mut w = sheltered_world(seed, start);
        let water = w.materials.id_of("water").expect("water must exist");
        for _ in 0..3000 {
            parallel::step(&mut w);
            w.step_active_sites();
        }
        let cells = (0..128).flat_map(|x| (0..128).map(move |y| (x, y))).filter(|&(x, y)| w.get(x, y).material == water).count();
        println!("after 3000 frames of weather on a 128x128 world: {cells} water cells");
        // Bar from measurement with headroom, not from an aspiration. What
        // this must catch is unbounded growth, not a puddle.
        assert!(cells < 1200, "{cells} water cells is a flood, not rain");
    }

    #[test]
    fn a_clear_sky_leaves_the_world_alone() {
        // The other half, and the one that protects per-tile field sleeping:
        // a world with no weather must not be touched at all, or every frame
        // of every clear day pays for a feature that is doing nothing.
        let seed = 4;
        let dry = (0..WEATHER_EPOCH_FRAMES * 40)
            .step_by(30)
            .find(|&f| !at(seed, f).is_precipitating())
            .expect("some frame is clear");
        let mut w = sheltered_world(seed, dry);
        let before = w.field_at(32, 100).moisture;
        for _ in 0..200 {
            parallel::step(&mut w);
        }
        assert_eq!(w.field_at(32, 100).moisture, before, "a clear sky changed the world's moisture");
    }

    #[test]
    fn the_same_seed_and_frame_always_give_the_same_weather() {
        for frame in [0, 1, 999, WEATHER_EPOCH_FRAMES, 400_000] {
            assert_eq!(at(7, frame), at(7, frame), "weather must be a pure function of (seed, frame)");
        }
    }

    #[test]
    fn different_seeds_get_different_weather() {
        // Compared over a span rather than at one frame: two seeds can agree
        // about one instant by coincidence, and asserting on a single frame
        // is the flakiest possible form of this test.
        let span: Vec<_> = (0..WEATHER_EPOCH_FRAMES * 4).step_by(600).collect();
        let a: Vec<_> = span.iter().map(|&f| at(1, f).intensity).collect();
        let b: Vec<_> = span.iter().map(|&f| at(2, f).intensity).collect();
        assert_ne!(a, b, "two seeds should not share a weather history");
    }

    #[test]
    fn a_world_is_mostly_clear() {
        // The shaping claim stated as a number, because "weather is an event"
        // is exactly the kind of intent that silently stops being true when a
        // threshold is retuned.
        //
        // Swept over seeds and gated on the **extremes**, not on one seed's
        // rate: weather here is a procedural system, so a single-seed bar is
        // a sample from a wide distribution and gets rubber-stamped by
        // whichever seed it was written against. What matters is that no
        // world is permanently sodden and none is a desert that never sees
        // rain -- both of which are per-seed failures a mean would hide.
        let rates: Vec<(u64, f32)> = (1..=12)
            .map(|seed| {
                let samples: Vec<_> = (0..WEATHER_EPOCH_FRAMES * 40).step_by(120).map(|f| at(seed, f)).collect();
                let wet = samples.iter().filter(|w| w.is_precipitating()).count();
                (seed, wet as f32 / samples.len() as f32)
            })
            .collect();
        let lowest = rates.iter().map(|&(_, r)| r).fold(f32::MAX, f32::min);
        let highest = rates.iter().map(|&(_, r)| r).fold(0.0f32, f32::max);
        let mean = rates.iter().map(|&(_, r)| r).sum::<f32>() / rates.len() as f32;
        // Printed, not just asserted. The bars are wide on purpose -- this is
        // a shaping claim over a chaotic quantity, and a narrow bar flakes --
        // so the *numbers* are what tell you whether a retune moved the feel,
        // while the assertions only catch it leaving the rails.
        println!(
            "weather shaping over 12 seeds, 40 epochs each: mean {:.0}% wet, range {:.0}%..{:.0}%",
            mean * 100.0,
            lowest * 100.0,
            highest * 100.0
        );
        assert!(lowest > 0.01, "some world is effectively rainless at {:.1}%", lowest * 100.0);
        assert!(highest < 0.60, "some world rains {:.0}% of the time, which is weather as wallpaper", highest * 100.0);
        assert!((0.05..0.40).contains(&mean), "the typical world should be clear most of the time: {:.0}% wet", mean * 100.0);
    }

    #[test]
    fn both_kinds_of_precipitation_occur_across_worlds() {
        // Snow is the rarer of the two by design, so this asks the question
        // across seeds rather than within one: a world that never snows is
        // fine, a *build* that never snows is a dead feature.
        let snowy = (1..=12).filter(|&s| first_frame_with(s, 0, |w| w.kind == Precipitation::Snow).is_some()).count();
        let rainy = (1..=12).filter(|&s| first_frame_with(s, 0, |w| w.kind == Precipitation::Rain).is_some()).count();
        println!("of 12 seeds: {rainy} see rain, {snowy} see snow");
        assert!(rainy >= 10, "only {rainy} of 12 worlds ever rain");
        assert!(snowy >= 3, "only {snowy} of 12 worlds ever snow; snow is effectively unreachable");
    }

    #[test]
    fn weather_changes_over_time_and_does_so_smoothly() {
        // Two claims that have to be tested together, because each one alone
        // is satisfied by a degenerate answer: constant weather is perfectly
        // smooth, and per-frame noise certainly changes.
        let a = at(9, 0);
        let far = at(9, WEATHER_EPOCH_FRAMES * 3);
        assert!(
            (a.intensity - far.intensity).abs() > 0.01 || (a.wind - far.wind).abs() > 0.01,
            "the weather should not be the same three epochs later"
        );
        let worst = (0..WEATHER_EPOCH_FRAMES * 4)
            .map(|f| (at(9, f).wind - at(9, f + 1).wind).abs())
            .fold(0.0f32, f32::max);
        assert!(worst < 0.01, "wind moved {worst} in a single frame, which is a step and not weather");
    }

    #[test]
    fn precipitation_begins_gently_rather_than_switching_on() {
        // Intensity is rescaled from the threshold precisely so that the
        // first frame of rain is not a downpour. Checked by finding the
        // *onset* -- a dry frame immediately followed by a wet one -- rather
        // than any wet frame, which would mostly land mid-event.
        let onset = (0..WEATHER_EPOCH_FRAMES * 40)
            .find(|&f| !at(4, f).is_precipitating() && at(4, f + 1).is_precipitating())
            .expect("seed 4 should start raining at some point");
        assert!(
            at(4, onset + 1).intensity < 0.05,
            "rain arrived at intensity {}, which is a switch rather than a front",
            at(4, onset + 1).intensity
        );
    }

    #[test]
    fn a_clear_sky_reports_no_drift() {
        // The escape hatch must not fire when nothing is happening, or it
        // wakes the field every frame forever and undoes per-tile sleeping.
        // This is the "ask what a metric counts when nothing is wrong" check
        // -- run against a case known to be quiet before trusting it about
        // one that is not.
        let quiet = (0..WEATHER_EPOCH_FRAMES * 8)
            .step_by(60)
            .filter(|&f| !at(4, f).is_precipitating())
            .map(|f| drift(4, f, f + 1))
            .fold(0.0f32, f32::max);
        assert!(quiet < 0.001, "a clear sky drifted {quiet} in one frame; the field would never sleep");
    }

    #[test]
    fn drift_notices_a_front_arriving() {
        // ...and the other half: a metric that never fires is not a hatch.
        // Measured across the whole onset rather than one frame, because the
        // per-frame change during a smooth front is genuinely tiny -- which
        // is the same accumulation problem `FieldTile::sky_amplitude` was
        // written to solve, and the reason `drift` takes two arbitrary frames
        // rather than assuming they are adjacent.
        let onset = (0..WEATHER_EPOCH_FRAMES * 40)
            .find(|&f| !at(4, f).is_precipitating() && at(4, f + 1).is_precipitating())
            .expect("seed 4 should start raining at some point");
        assert!(
            drift(4, onset, onset + 600) > 0.01,
            "ten seconds of a front arriving registered as no change at all"
        );
    }
}
