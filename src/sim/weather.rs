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
//!
//! # Rain comes out of a bank, and the bank is real
//!
//! The sky above is a pure function and stores nothing. What *falls* out of
//! it is not free: every water cell this file spawns is charged to
//! `World::atmospheric_bank`, which `evaporation::tick` is the only thing
//! that fills. Before that existed the two halves did not know each other --
//! evaporation deleted water and credited nothing, precipitation created it
//! out of nothing, and a world's total water was whatever the difference
//! between two independently tuned rates happened to be.
//!
//! Three consequences worth having in mind before editing anything here:
//!
//! * **The storm scales with the balance**, through [`supply`], and the
//!   *renderer reads the same factor*. Falling precipitation is drawn from
//!   `at(seed, frame)` and never simulated (see [`step`]'s own doc), so a
//!   gate on the landing side alone would draw a downpour that deposits
//!   nothing.
//! * **The bank is not part of the pure function.** `at(seed, frame)` still
//!   answers the same thing forever; what a given frame of weather *does* to
//!   a world now depends on that world's history. Determinism is unaffected
//!   -- it is f(seed, initial world, frame sequence), and nothing here draws
//!   from `world.rng` -- but a forecast is no longer a prediction of how
//!   much water will land.
//! * **Two things a storm does are deliberately outside the ledger**: the
//!   field moisture write and the soil soak. Both are commented at their own
//!   sites with why.

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
/// cells while the shipped 2048-wide world saw an ordinary shower. The
/// number that should be constant is drops per column per second; the cap
/// then bounds work on worlds larger than this, at the price of rain that
/// thins out on them -- which is the right trade, because frame cost is a
/// hard constraint and rain density is a feel.
const REFERENCE_WIDTH: f32 = 2048.0;

/// The bank balance, in liquid-water cell-equivalents, at which a storm runs
/// at full strength. Below it the whole storm scales down in proportion; at
/// zero the sky is empty and nothing falls.
///
/// **Set from measurement, with headroom, per `probe_storm_yield`.** That
/// probe pins the bank full and runs one whole front end to end over a
/// 512-wide world — the filmstrip's width — counting what the sky actually
/// spends. Five seeds, three surfaces, cell-equivalents per front:
///
/// | seed | frames | bare rock | dry soil | damp soil |
/// |---|---|---|---|---|
/// | 7 | 3,660 | 55 | 141 | 141 |
/// | 20 | 3,480 | 51 | 170 | 170 |
/// | 12345 | 6,120 | 42 | 788 | 788 |
/// | 31337 | 4,980 | 87 | 616 | 616 |
/// | 2900 | 18,660 | 14 | 6,944 | 6,147 |
///
/// **The surface matters far more than the seed, and the reason is a rule
/// in `step` itself.** Rain refuses to stack on standing water, so a storm
/// over bare rock covers the shelf with its own first few dozen cells and
/// then delivers almost nothing for the rest of the front — the rock column
/// is a storm that throttled *itself*, and a reserve sized off it would be
/// sized off an artefact. Over soil the landing cells are swallowed by
/// infiltration, the surface never becomes liquid, and the storm spends at
/// its full rate start to finish. Damp soil (at `SOIL_FIELD_CAPACITY`)
/// reads the same as bone-dry soil on four seeds of five, so this is not a
/// first-fill transient: soil drains as fast as it drinks at these rates.
///
/// Seed 2900 is left in the table and out of the sizing. Its 18,660 frames
/// are two and a half wet *epochs* run together, not a front, and it is
/// mostly snow — `SNOW_CELL_CHANCE` is nine times rain's, so it spends at
/// 330/1000 frames against rain's 40 to 130. It is the case the throttle
/// exists for rather than the case it should be sized around.
///
/// So: 788 is the worst genuine front, and **2,500 is a shade over three of
/// them**. Three rather than one for two separate reasons. A world that
/// could afford exactly one storm would spend the whole of it on the first
/// front and thin visibly through the second, which reads as the weather
/// breaking rather than as a dry spell. And the factor is *linear* in the
/// balance, so a reserve sized at one storm means a world halfway through
/// its first front is already raining at half strength — the throttle would
/// be a constant presence instead of the thing that shows up when a world
/// has genuinely run dry.
///
/// This is also the endowment a fresh world starts with (`World::new`), so
/// it is simultaneously "how much rain a brand-new world gets before
/// evaporation has paid anything back".
///
/// # Soil is on the books now, and this number wants re-deriving
///
/// **Infiltration used to be an un-credited sink.** `update::
/// update_soil_water` turns a landing water cell's fill into soil wetness,
/// `water_equivalents` did not count soil, and nothing gave it back — so a
/// soil world was a bank with a leak that could not even be measured.
///
/// Both halves are closed. The ledger counts held water (see
/// `water_equivalents`'s soil arm), which makes infiltration
/// **neutral** rather than a loss, since it moves `taken` units 1:1 out of
/// a fill and into an `aux`. And damp soil at an open surface now
/// evaporates and credits the bank (`evaporation::tick_soil`), which is
/// the second credit path this section used to ask for. The rain soak is
/// charged rather than free, or the first change alone would mint water on
/// every drop.
///
/// Measured (`probe_long_run_balance`, 60,000 frames, seed 12345, the same
/// world with the two surfaces):
///
/// | frame | bare rock: bank / supply | soil, before | soil, now |
/// |---|---|---|---|
/// | 25,000 | 2,500 / 1.00 | 2,500 / 1.00 | 2,500 / 1.00 |
/// | 45,000 | 2,494 / 1.00 | 1,357 / 0.54 | 1,823 / 0.73 |
/// | 60,000 | **2,500 / 1.00** | 1,360 / 0.54 | **1,903 / 0.76** |
///
/// Over rock the loop closes exactly and always did. Over soil it now
/// settles at three-quarters supply instead of a half, and — the part that
/// matters more than the number — `water_equivalents + bank` is flat to the
/// unit on a soil world, where before it was not a conservation law at all.
///
/// **What is left, and it is a sizing question rather than a leak.** Soil
/// storage is enormous next to this constant: 24 rows of soil across a
/// 512-wide shelf hold **12,288** cell-equivalents against a reserve of
/// 2,500. Charging for wetting it therefore competes directly with rain,
/// and the equilibrium above is where that competition lands. Two knobs
/// were swept against it and both are recorded rather than guessed at:
/// raising the soil drying rate twentyfold only reached 0.28 (the reservoir
/// is the constraint, not the rate), and cutting `SOIL_SOAK_PER_DROP`
/// tenfold — which is what shipped — reached 0.76.
///
/// **This constant's own sizing moved with it and has not been changed.**
/// `probe_storm_yield`'s worst *genuine* front went from 788 to ~1,028
/// cell-equivalents now that the soak is charged, so 2,500 is a shade over
/// **2.4** storms where the argument below is written for three. Recorded
/// rather than adjusted: raising it also raises a fresh world's endowment,
/// which is a feel decision about how wet a new world starts.
pub const STORM_RESERVE: f64 = 2500.0;

/// How much of a full-strength storm a bank holding `bank` cell-equivalents
/// can pay for, `0.0..=1.0`.
///
/// Linear and clamped, deliberately not a curve: the one thing this must do
/// is reach exactly zero when the sky is empty, and every shaping function
/// anyone reaches for here (a root, a smoothstep) either lifts the empty end
/// off zero or makes the interesting middle of the range harder to reason
/// about. `evaporation::dryness` takes a root and says why in its own
/// comment — that case needed the *shallow* end lifted and this one must not
/// be.
///
/// Read through `World::storm_supply` by both the simulation and the
/// renderer. Free function so it stays testable without a world.
pub fn supply(bank: f64) -> f32 {
    (bank / STORM_RESERVE).clamp(0.0, 1.0) as f32
}

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
const SOIL_SOAK_PER_DROP: u16 = material::SOIL_SATURATED / 100;

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

/// How deep into a body of standing water a snowfall keeps the cold going,
/// in cells.
///
/// Shallower than `SNOW_CHILL_DEPTH`, and for a different reason rather
/// than as a saving: a drift has to be held cold all the way through or it
/// rots from underneath, while water only has to be held cold at the
/// *surface*, because that is the only place a sheet can form. Three cells
/// rather than one so that the water immediately under a fresh sheet is
/// cold too and the sheet thickens downward -- a single chilled row freezes
/// one cell thick onto water sitting at ambient, and then melts from below
/// while the storm is still falling on it.
const WATER_CHILL_DEPTH: i32 = 3;

/// How cold a snowfall holds standing water, in degrees below ambient at
/// full intensity and full chill -- the same shape as `SNOW_CHILL`, and
/// deliberately so.
///
/// # The number is set by *when* a pond should freeze, not by how cold
///
/// The first version was an absolute target rather than a magnitude
/// (`-2 - 8 * intensity * chill`), which put water below its 0-degree
/// freezing point in **any** snowfall however light. That is wrong on its
/// own terms -- a trace of snow melts on contact with open water, it does
/// not freeze a lake -- and it broke a guard that had nothing to do with
/// ice: `tests/worldgen.rs`'s `generated_terrain_is_already_at_rest`
/// reported 411 cells "leaving their position" on the `terraced` preset,
/// seed 3, which turned out to be a lake quietly turning to ice under a
/// 0.36-intensity flurry (`worldgen::generate` sets `World::seed` from the
/// generation seed, so each preset seed gets its own weather). A material
/// change in place reads as movement to that test, and it was right to
/// complain.
///
/// So freezing is tied to the same bar snow already has to clear to *lie*:
/// a landing flake melts unless `SNOW_CHILL` puts it under snow's own
/// 2-degree melting point, which needs intensity above ~0.69, and
/// `SNOW_THRESHOLD` already requires chill above 0.70 for snow at all --
/// so the product crosses ~0.48 exactly when snow starts settling. 40
/// crosses zero at 0.50. **Water freezes when snow lies**, which is one
/// rule to remember instead of two thresholds to reconcile, and it is what
/// a player would guess.
///
/// At the other end it is deep: `scene=coldsnap`'s front (intensity 0.97,
/// chill 0.76) holds water at -9.5, which is margin enough that a sheet
/// survives the gaps between drops on its own conductivity.
const WATER_CHILL: f32 = 40.0;

/// How deep into open water the front's own sweep holds cold, in cells.
///
/// # One is the difference between a sheet and a churning slush
///
/// This was **zero**, so nothing but a landing flake could ever start a
/// freeze — the sweep stopped at open water and freeze-over crept one drop
/// at a time, which was deliberate ("freeze-over starts somewhere rather
/// than everywhere"). What that actually produced, once the freeze rate
/// was slowed to something a player could watch, was a **standing
/// equilibrium**: cold arrives in per-column pulses, a chilled cell that
/// does not roll a freeze before its column warms back up never freezes,
/// and an ice cell whose column has moved on melts. Measured on
/// `scene=coldsnap`: **491 freezes and 510 melts across 340 frames for a
/// net of minus nineteen**, forever, reported from play as *"it never
/// really freezes... the pixels seem to be constantly shifting."*
///
/// With the surface held cold continuously the churn disappears outright —
/// same scene, `melted +0` across the whole freeze and every freeze
/// sticking (churn 1.0), reaching a closed sheet over 100% of the pond.
///
/// One and not three. Three chills enough water to freeze 781 cells in a
/// single sample window, which is a pond going solid in under a second and
/// is the artifact this work started from. Depth here sets *how much* water
/// is eligible per sweep; `FREEZE_CHANCE` sets how fast it converts. Keep
/// them separate: this one is not the pace knob.
const SWEEP_LIQUID_DEPTH: i32 = 1;

/// How many cells of ice halve the rate at which the water underneath it
/// freezes.
///
/// # Stefan's law, replacing a cliff that was standing in for a curve
///
/// **Ice insulates the water it is made of.** The interface freezes at a
/// rate set by the temperature gradient across the crust already above it,
/// so `dx/dt` goes as `1/x` and thickness goes as the **square root of
/// time** — fast at first, then ever slower, never quite stopping. That
/// self-limiting shape is the whole reason a real pond does not freeze to
/// its bed on one cold night.
///
/// This replaced `SHEET_MAX_THICKNESS`, a hard nine-cell cutoff on how much
/// crust the sweep would reach through. That constant existed for the right
/// reason -- without a bound a 20-deep pond went to **833 frozen cells
/// against 148 of liquid**, solid to within a row of its bed -- and got
/// there the wrong way: a wall where the physics has a curve. It also had
/// to be *chosen*, and nothing could say what it should be, because the
/// quantity it capped was not a quantity anything measured. The readout it
/// needed (`examples/filmstrip.rs`'s `sheet:` line) did not exist until
/// this change either, and the first thing it showed was every column of
/// the pond pinned at 10 to 12 cells: the cap, being a cap.
///
/// # Realised as how often a visit reaches the water, not how deep it goes
///
/// A depth rule cannot produce a curve. A *rate* rule can: a column is
/// visited many times, so a `1/x` chance of reaching the water per visit is
/// a `1/x` growth rate, which integrates to the square root. Nothing has to
/// remember a column's history for this -- the crust above the water is
/// itself the record of how much has frozen there.
///
/// Four is set from what it has to beat: at four cells the sheet grows at
/// half speed, at twelve at a quarter, so the old nine-cell wall is now a
/// place the ice passes slowly rather than one it stops at.
const STEFAN_HALVING_DEPTH: f32 = 4.0;

/// How many cells of crust the *temperature* falloff halves over -- the
/// linear profile through the slab, and the thing that makes the sheet stop.
///
/// **This stands in for a heat flux the engine does not have, and saying so
/// is the point.** Real lake ice stops thickening when the heat conducted up
/// through it equals the heat arriving from the water below. Nothing in this
/// engine supplies that upward flux, so Stefan's rate law alone -- correct as
/// it is -- describes growth that slows forever and never stops. Measured:
/// at a one-cell halving depth the pond went from 944 cell-equivalents of
/// liquid to **180 against 971 frozen** over nine thousand frames,
/// thickening monotonically throughout. That is exactly the runaway the old
/// `SHEET_MAX_THICKNESS` cap existed to prevent, arriving by a slower road.
///
/// So the cold delivered at depth is attenuated as well as its rate, and
/// below the point where it no longer clears the freezing point nothing
/// freezes and what is there melts back. The sheet settles where chilling and
/// conduction balance.
///
/// Sixteen, set by measurement against the sheet the previous build produced
/// and play accepted: paired on `scene=coldsnap`, the old nine-cell cap gave
/// a sheet **5.9 cells thick on average and 12 at the thickest**, and this
/// gives 5.3 and 10. Swept -- 12 gives 4.1/8, 20 gives 6.5/13.
///
/// It is not arbitrary the way nine was, because it is not the answer: it
/// scales one. The stall lands at about `G * (2.5 * bite - 1)` cells, so **a
/// harder night really does make thicker ice**, where nine was one number for
/// every night.
const CRUST_GRADIENT_DEPTH: f32 = 16.0;

/// How many cells of ice one cell of lying snow is worth, as insulation.
///
/// **Snow is the best insulator in the scene and the engine had it as an
/// accelerant.** Real snow has roughly a tenth of ice's thermal
/// conductivity -- it is mostly trapped air -- so a snow blanket is what
/// stops a lake freezing deeper, and lake ice grows *fastest* on cold clear
/// nights with the snow blown off. The engine did the reverse: a landing
/// flake chilled three cells into the water across nine columns, against
/// the clear-sky sweep's one, so snowfall multiplied the freezing rate.
///
/// Reported from play, off the six-minute arc: *"it seems to slowly grow
/// and then jumps to fully frozen"*. Measured across that step, freezing
/// went **+40 cells in one window to +246** in the next, while lying snow
/// went 55 to 114.
///
/// Four rather than the real ten: at ten a single flake settling on a sheet
/// stops it dead, which is a cliff again with the sign flipped. What is
/// wanted is that snow cover slows thickening markedly, not that it
/// forbids it.
///
/// Measured by `a_drift_on_the_ice_slows_the_freeze_underneath_it`, which had
/// to be *built* rather than found -- no shipped scene puts snow on ice, and
/// this constant swept at 1, 4 and 8 gave bit-identical output on
/// `scene=coldsnap` because seed 2900's cold spell is dry from end to end.
/// On the paired pond, ice under a drift against bare ice in the same run:
/// **8.47 against 6.10 cells with the old behaviour, 6.23 against 6.83 with
/// this**. The sign of the effect is what changed, not just its size.
///
/// Note this insulates *lying* snow only. Falling snow still delivers cold,
/// and that is not the inversion: snow falls out of cold air, and a snowy
/// night really does ice a pond over faster across its *surface*. It is the
/// thickening underneath that a blanket should slow.
const SNOW_INSULATION: f32 = 4.0;

/// Above this on the chill channel, a clear sky freezes standing water on
/// its own.
///
/// # Cold without precipitation is still cold
///
/// `Weather::chill`'s own doc has said so since it was written — *"a clear
/// winter night is different from a clear summer one"* — and nothing acted
/// on it: `step` returned early whenever nothing was falling, so the only
/// thing in the world that could freeze water was a landing flake. A pond
/// therefore had only the **overlap** of "raining hard" and "cold" to
/// freeze in, which on `scene=coldsnap` is about 700 frames, while the cold
/// air mass itself lasts far longer: measured over 400 minutes of play per
/// seed, a spell of `chill > SNOW_THRESHOLD` runs a **mean of 140 seconds,
/// median 115, max 12 minutes**, and the world is in one about a quarter of
/// the time (`probe_cold_spells`).
///
/// Asked for from play — *"the freeze is so fast, it lasts only a few
/// seconds; this should be a different order of magnitude"* — and this is
/// where the order of magnitude comes from. It is **not** a longer weather
/// cycle, which was the other candidate and was measured and rejected: the
/// cycle is already long enough, it was only the freezing that could not
/// reach most of it. Lengthening it would have made snow rarer for nothing.
///
/// Set to `SNOW_THRESHOLD` exactly, so there is one bar to remember rather
/// than two to reconcile: **it freezes when it is cold enough to snow**,
/// whether or not anything is actually falling.
const DRY_FROST_CHILL: f32 = SNOW_THRESHOLD;

/// How deep a *crust* the front's own sweep will hold cold, in cells./// How deep a *crust* the front's own sweep will hold cold, in cells.
///
/// Generous, and it needs to be, because the crust is two materials
/// stacked: a sheet of ice with a snow drift lying on it. Sharing
/// `SNOW_CHILL_DEPTH` between them was tried and is what kept freeze-over
/// from ever completing -- the drift spent the whole budget, the ice
/// beneath it was never reached, and the middle of the pond (where the
/// drift settles first) melted out from under the snow while the storm was
/// still falling. Patches of ice appeared and never joined up.
///
/// Costs nothing to set high: the walk stops at the first cell that is
/// neither crust nor freezable liquid, so on bare rock it reads one cell
/// and stops, and it can only ever be as deep as the crust actually is.
///
/// Deliberately *not* shared with the landing drops, which keep the tighter
/// `SNOW_CHILL_DEPTH` -- see `hold_column_cold`'s `crust_depth`. That
/// asymmetry is what bounds how thick a sheet gets: a drop can only chill
/// water it can reach through the crust, so once the ice is
/// `SNOW_CHILL_DEPTH` thick nothing below it is chilled again and the sheet
/// stops growing downward at about nine cells. Without the bound a pond
/// froze solid to its bed.
const CRUST_CHILL_DEPTH: i32 = 16;

/// How far to either side of a landing drop the cold reaches *into water*,
/// in columns.
///
/// # A nucleus, and then how much of the surface holds ice
///
/// 0 does not work at all. A single chilled column freezes a single cell of
/// ice, and a lone ice cell cannot survive: its neighbours are 20-degree
/// water either side and 20-degree air above, so it warms past its own
/// melting point in four frames and melts straight back. Measured on
/// `scene=coldsnap` at radius 0 -- **654 freezes and 645 melts over 450
/// frames**, a one-for-one churn, with never more than a dozen cells of ice
/// standing on a 60-cell pond. Freeze-over could not start. A run of
/// columns freezes together and the middle of the run sees ice either side,
/// which is what lets a patch survive its first few frames; after that the
/// sheet grows by accretion, because a chilled column next to the sheet
/// keeps its ice while one out in open water still loses it.
///
/// # What it does *not* control, measured rather than assumed
///
/// It is not the pacing knob it looks like. At this storm's intensity a
/// 512-wide world takes ~6 drops a frame, so a 60-cell pond is reached
/// within a few frames whatever this is set to, and the ice standing on it
/// is at an **equilibrium between freezing and the warm water underneath**,
/// not partway through a march across the surface. Swept on `scene=coldsnap`
/// (rebuilding between points, since materials and this file are compiled
/// in), reading standing ice cells against the ~540 the surface could hold:
///
/// | radius | columns/drop | standing ice | reads as |
/// |---|---|---|---|
/// | 0 | 1 | ~10, churning 1:1 | nothing forms |
/// | 2 | 5 | 114 by frame 100, ~150 after | permanently patchy |
/// | 4 | 9 | 217 by frame 20, 250-360 after | a crust with holes, consolidating |
/// | 6 | 13 | 355 by frame 60, flat after | closed almost at once |
///
/// So "freeze-over creeps" is true of a **wide** body of water and not of a
/// small one: on `pond=300` the same setting goes 668 cells at frame 40 to
/// 2,026 at frame 600, visibly joining up across a contact sheet, while a
/// 60-cell pond is done before the first tile. That is the right way round
/// -- a puddle should freeze in seconds and a lake should not -- and it is
/// what `CLAUDE.md`'s "thin ice far from shore" note anticipated.
///
/// 4 is the setting: a small pond gets a crust immediately, a large one
/// takes hundreds of frames, and neither closes so completely that the
/// water underneath stops being visible through the gaps.
///
/// Far smaller than the crust sweep's coverage below, and deliberately so:
/// holding lying snow cold is a *maintenance* job that has to reach
/// everywhere often, while freezing water is an *event* that should start
/// where the snow is actually falling.
const WATER_CHILL_RADIUS: i32 = 4;

/// How often the front comes back to any given column with its cold, in
/// frames -- the period of the banded sweep in `hold_the_ground_cold`.
///
/// # Why the cold cannot be delivered by the drops alone
///
/// A drop chills the column it lands in, and one column per drop is what
/// makes a freeze-over *creep*: the storm has to come back to a pond column
/// by column, and a 60-cell pond takes a few hundred frames to close over.
/// That is the behaviour to keep, and this sweep deliberately does not
/// freeze anything -- it holds a *crust* (a drift, or a sheet of ice) and
/// stops at open water.
///
/// A drift cannot live on the drops' terms. Empty air in this engine is a
/// fixed 20-degree reservoir (`Cell::EMPTY` carries `AMBIENT_TEMPERATURE`
/// and nothing cools it), so a snow cell's warmest neighbour is always the
/// sky above it, and once snow has real conductivity it warms back at 2
/// degrees a *frame*: -5, -3, -1, 1, melt. Measured directly -- a lone
/// flake written at -8 melts after 4 frames, at -30 after 11, and a
/// six-deep drift is entirely gone 9 frames after the cold stops, because
/// depth buys nothing once the first row's meltwater arrives at 20 degrees
/// and eats downward faster than the air did.
///
/// At full intensity a 512-wide world takes ~6 drops a frame, so a given
/// column sees one about every 90 frames, and the drops are Poisson: even
/// widening each drop's cold to 21 columns leaves gaps of ten frames and
/// more. Both were measured on `scene=coldsnap`, and both produced the same
/// artifact -- 3.1 flakes a frame landing, never more than 40 cells
/// standing, and every one of them melting into a **meltwater flood that
/// spread across the whole world and drowned the pond the scene is about**,
/// with the world's water doubling over one storm. That is the real cost of
/// giving snow conductivity, and it is `CLAUDE.md`'s "fixing a bug often
/// exposes a constant that was compensating for it": snow's missing
/// conductivity was standing in for there being no cold-air model.
///
/// So the front holds the ground it is over, on a fixed period rather than
/// wherever flakes happen to land. Two frames leaves two frames of margin
/// against the four-frame melt; three would just fit and one is twice the
/// work for nothing.
///
/// **It is nearly free, and for a reason worth keeping**: the walk writes
/// only to cells that hold cold or can freeze, so a column of bare rock
/// costs one surface lookup and one material read and *writes nothing*.
/// Bare terrain therefore does not get dirtied and its chunks still sleep
/// through a snowstorm -- which is what keeps this off the list of things
/// that cost the dirty-rect render skip.
const CHILL_REVISIT_FRAMES: u64 = 2;

/// How far up or down a neighbouring column's surface may be from the one
/// beside it and still be found, when the crust chill walks outward from a
/// landing drop.
///
/// The chill walk deliberately does **not** re-run `surface_under_sky` per
/// column: that scans a column from the top of the sky (240-odd rows here)
/// and doing it 21 times a drop would put a real cost on weather, which is
/// exactly what `MAX_COLUMNS_PER_FRAME`'s own doc says must not happen. It
/// instead carries the previous column's surface as a hint and searches a
/// short window around it, updating the hint as it goes, so a slope is
/// tracked one step at a time and a cliff simply ends the walk. 8 is about
/// the steepest single-column step generated terrain produces.
const LOCAL_RELIEF: i32 = 8;

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
    let Some(bounds) = world.bounds() else { return };
    if !w.is_precipitating() {
        // **A clear freezing night still freezes.** See `DRY_FROST_CHILL`.
        // Only the ground sweep, and no drops: there is nothing to deposit,
        // nothing to wet, and nothing to charge the bank for.
        if w.chill > DRY_FROST_CHILL {
            let snow = world.materials.id_of("snow");
            hold_the_ground_cold(world, w, bounds, snow);
        }
        return;
    }

    let width = bounds.max_x - bounds.min_x + 1;
    // Fractional columns are resolved by chance rather than rounded up: at
    // low intensity on a small world the honest answer is "less than one
    // column this frame", and rounding that to one is how a drizzle becomes
    // a downpour on a test world.
    //
    // **The whole storm scales with what the sky is holding**, rather than
    // only the spawns being refused one at a time at the bottom. A front
    // that goes through all its motions and then declines to deposit
    // anything is the "no verb behind the effect" failure in miniature: the
    // rain would be *drawn*, the ground would be *wetted*, and the one
    // visible consequence would silently stop. Thinning the column budget
    // makes a low bank read as a thinner shower -- fewer columns touched,
    // less of everything, all the way down -- which is what running out of
    // water should look like. `World::spend_atmosphere` is still the hard
    // floor underneath it; see `storm_supply`.
    let supply = world.storm_supply();
    let wanted =
        (w.intensity * MAX_COLUMNS_PER_FRAME * width as f32 / REFERENCE_WIDTH).min(MAX_COLUMNS_PER_FRAME) * supply;
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
    // What one spawned cell of each phase costs the bank, in liquid-water
    // cell-equivalents. A full water cell is 1.0 by the definition of the
    // unit; a flake is worth its density ratio, read off the materials
    // rather than written as a literal so that it is *the same number*
    // `fire::melt_fill` hands back when the flake thaws. Snow at density 0.3
    // melts into 0.3 of a water cell, so a sky that charged itself a whole
    // cell for one would be quietly destroying 0.7 of a cell per flake --
    // conservation broken by the very code that exists to close it, and in
    // the direction no test looking for manufactured water would ever see.
    let snow_cost = snow.map_or(0.0, |s| {
        world.materials.get(s).density as f64 / world.materials.get(material::WATER).density as f64
    });
    // The front's cold over the *whole* ground it is above, on a fixed
    // period, independent of where flakes land -- see
    // `CHILL_REVISIT_FRAMES` for why the drops cannot carry this on their
    // own. Crust only: it never freezes water, so freeze-over still creeps
    // one drop at a time.
    //
    // Deliberately *after* the `columns == 0` return above, so a frame that
    // delivers no precipitation at all delivers no cold either. At full
    // intensity that never happens; at a drizzle's intensity it happens
    // most frames, and the drift then melts between them. That is the
    // "light snow does not lie" behaviour rather than an oversight -- the
    // same bar `SNOW_CHILL` and `WATER_CHILL` are both set against.
    if w.kind == Precipitation::Snow {
        hold_the_ground_cold(world, w, bounds, snow);
    }
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
        //
        // **Outside the water ledger, on purpose.** This writes the field's
        // moisture channel, which is not water and is not conserved by
        // anybody: `field.rs`'s forcing manufactures it and its own decay
        // destroys it every step, quite independently of anything the sky or
        // a puddle does. Charging the bank for it would be charging for a
        // quantity that evaporates back into nothing, so the bank would
        // drain to zero over any long run and never recover. And the
        // consequence would be the wrong one to look at: a bankrupt sky that
        // cannot even *wet the ground* is a front passing overhead with no
        // effect at all, which is exactly the inert mechanic this file's
        // storm scaling exists to avoid. The storm as a whole already thins
        // with the bank; a column that does run wets what it lands on.
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
        //
        // **Also outside the ledger**, and for a different reason from the
        // moisture write above: a Powder's `aux` is a *wetness*, not a fill.
        // `material::SOIL_SATURATED` is a saturation fraction on its own
        // scale, there is no conversion from it to a cell-equivalent that
        // means anything, and nothing anywhere gives it back as water --
        // `update::soil_moisture` spends it on roots and evaporation never
        // sees it. It is a state a cell is in, not a quantity the world
        // holds. The day soil genuinely stores and releases water, this line
        // joins the ledger and `evaporation` grows a second credit path;
        // until then, pretending would be worse than not accounting.
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
            let added = wetter - cell.aux();
            // **Charged to the bank, now that held water is on the books.**
            // The comment above used to end "until then, pretending would be
            // worse than not accounting" -- and this is the day it names.
            // `water_equivalents` counts soil moisture, so a soak that wrote
            // itself for free would mint water on every drop and the
            // conservation law would read as growth instead of a leak.
            //
            // Refused rather than partially paid, on `spend_atmosphere`'s
            // own all-or-nothing rule: there is no such thing as
            // three-tenths of a wetted cell, and a caller that gets `false`
            // must not write. A bankrupt sky therefore stops wetting the
            // ground, which is the same throttle the storm as a whole is
            // already under -- the column budget is scaled by the bank
            // before we get here, so a refusal is the rounding at the bottom
            // of the barrel rather than the usual path.
            if added > 0 && world.spend_atmosphere(added as f64 / material::SOIL_SATURATED as f64) {
                world.set(x, y, cell.with_aux(wetter));
                super::evaporation::schedule_damp_soil(world, x, y);
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
            //
            // **This one push is the field half for the whole column**,
            // water included -- it fires before the walk below has decided
            // whether this column is ground, a drift or a pond, and its
            // radius already spans the cells that walk reaches. A second
            // push for the water would double the cold in the coarse
            // channel and wake field tiles a frame's worth of nothing, and
            // the field does not decide freezing any more than it decides
            // melting.
            world.add_heat(x, surface_y, 2, -SNOW_CHILL * w.intensity * w.chill);
            let cold = (AMBIENT_TEMPERATURE as f32 - SNOW_CHILL * w.intensity.max(0.4)) as i16;
            let water_cold = (AMBIENT_TEMPERATURE as f32 - WATER_CHILL * w.intensity * w.chill) as i16;
            // A short run of columns centred on where the drop landed,
            // each walked down through drift and ice crust and into the top
            // `WATER_CHILL_DEPTH` cells of the water -- see
            // `hold_column_cold` for what it recognises, and
            // `WATER_CHILL_RADIUS` for why the run is five columns wide and
            // not one. This is the only thing in the file that can freeze
            // water, and a drop lands in a randomly chosen column, so
            // freeze-over starts somewhere rather than everywhere.
            hold_column_cold(world, x, surface_y, bounds, snow, cold, water_cold, SNOW_CHILL_DEPTH, WATER_CHILL_DEPTH);
            for dir in [-1i32, 1] {
                let mut hint = surface_y;
                for step in 1..=WATER_CHILL_RADIUS {
                    let cx = x + dir * step;
                    if cx < bounds.min_x || cx > bounds.max_x {
                        break;
                    }
                    // Hinted from the last column found rather than
                    // re-walking the sky, the same way the crust sweep
                    // does; a cliff simply ends the run.
                    let Some(cy) = surface_near(world, cx, hint, bounds) else {
                        break;
                    };
                    hint = cy;
                    hold_column_cold(world, cx, cy, bounds, snow, cold, water_cold, SNOW_CHILL_DEPTH, WATER_CHILL_DEPTH);
                }
            }
            // **Snow does not lie on open water**, the same refusal the
            // rain path below makes and for a stronger reason. Snow is a
            // `Powder` lighter than water, so a flake landing on a pond
            // floats -- and flakes kept coming, so a raft built up. Two
            // things went wrong at once: the raft is what the sky can see,
            // so `surface_under_sky` returned *it* instead of the water,
            // and once it was `SNOW_CHILL_DEPTH` deep the walk above spent
            // its whole crust budget inside it and the water underneath
            // stopped being chilled at all. A pond under a blizzard
            // therefore never froze -- measured at 9 to 20 cells of ice
            // over a whole storm, against a 60-cell surface.
            //
            // A flake that lands in water has simply joined it, which is
            // both what happens and what leaves the surface visible. Once
            // the surface *is* ice -- a `Solid`, not a `Liquid` -- this
            // stops applying and drifts build on the frozen pond with no
            // code of their own, which is the behaviour the milestone
            // wanted.
            let open_water = world.materials.kind(world.get(x, surface_y).material) == MaterialKind::Liquid;
            if let Some(snow) = snow {
                if !open_water && r.chance(SNOW_CELL_CHANCE * w.intensity) && surface_y > bounds.min_y {
                    let above = surface_y - 1;
                    // The roll happens whether or not the bank can pay, and
                    // deliberately: `r` is a per-(seed, frame, column)
                    // stream shared with the chill walk above, so making a
                    // draw conditional on an accumulated balance would make
                    // the *rest* of this column's weather a function of how
                    // much it had rained earlier. Determinism here is
                    // f(seed, initial world, frame sequence) and the bank is
                    // part of the initial world, not part of the stream.
                    if world.get(x, above).material == material::EMPTY && world.spend_atmosphere(snow_cost) {
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
                // 1.0: a full water cell is the unit the bank is counted in.
                // Charged *before* the write and only if it goes through, so
                // the identity "cells the sky made == cell-equivalents the
                // bank lost" holds exactly rather than approximately -- which
                // is what lets `the_worlds_water_is_flat_across_storm_and_
                // drought_cycles` be a conservation assertion instead of a
                // trend.
                if world.get(x, above).material == material::EMPTY && world.spend_atmosphere(1.0) {
                    world.set(x, above, Cell::new(water, 0));
                }
            }
        }
    }
}

/// How much *water* a world holds, in liquid-water cell-equivalents,
/// counting every phase at what it would come back as. The standing half of
/// the conservation law whose other half is `World::atmospheric_bank`, so
/// `water_equivalents(w) + w.atmospheric_bank` is the quantity that should
/// not move.
///
/// **Deliberately a different question from a phase census**, and the
/// difference is the whole point of it. "How much of each phase is standing"
/// counts a flake and a full water cell alike as one cell — correct for
/// asking whether a pond froze over, useless for conservation, because the
/// two are not the same amount of water. Here a cell's *density* is the
/// conversion factor: fresh snow is 0.3 the density of water, so 1,700
/// flakes are ~510 cells of meltwater and anything more than that was
/// manufactured.
///
/// Liquids counted as **fill, not occupancy** (`CLAUDE.md`'s metric traps: a
/// resting body wears a fringe of near-empty cells, and six full cells
/// spreading into a film across thirty-eight is a 6 -> 228 *increase* by cell
/// count at constant volume).
///
/// Steam is counted from its `aux` rather than from its density, because
/// steam's `aux` is not a density at all — `fire::transform` carries the
/// source water's fill across the boil and gives it back on condensing, on
/// `LIQUID_FULL`'s 0-means-full convention (steam.ron's header documents it
/// on the content side), so that loop is already exact per cell and a
/// density factor would double-count it.
///
/// Keyed on the materials' own phase-change fields rather than on names, so
/// a new liquid that freezes, or a new solid that melts back, is counted
/// without this function learning about it.
///
/// **A whole-world scan**, so it is a measurement tool — tests and
/// `examples/filmstrip.rs` — and not something to call per frame.
pub fn water_equivalents(w: &World) -> f64 {
    let Some(b) = w.bounds() else { return 0.0 };
    let full = material::LIQUID_FULL as f64;
    let water_density = w.materials.get(material::WATER).density as f64;
    let mut total = 0.0f64;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let cell = w.get(x, y);
            let m = w.materials.get(cell.material);
            match m.kind {
                // Water itself. **Identity, not "any liquid that freezes"**
                // — which is what this arm asked before, and it counted
                // *lava* as water at 2.7 cell-equivalents each, because
                // `lava.ron` has a finite `cooling_point` (700C, where it
                // crusts to stone) exactly as water has one at 0C. The
                // predicate was written when water was the only liquid with
                // a phase below it and was true for the whole of that
                // period; lava made it wrong and nothing noticed, because
                // no conservation test had lava in it.
                //
                // It is not a small error. On `filmstrip scene=lavapour`
                // the ledger printed under each tile fell by ~1,530
                // cell-equivalents over 300 frames with no water going
                // anywhere at all — 647 lava cells crusting, at 2.7 apiece
                // — so the one line that is supposed to *be* the
                // conservation law read as a leak on every scene with lava
                // in it. Found while verifying a change against it.
                MaterialKind::Liquid if cell.material == material::WATER => {
                    total += crate::sim::update::liquid_fill(cell) as f64 / full;
                }
                // Steam and anything else that condenses *back into water*.
                MaterialKind::Gas if m.cools_into == Some(material::WATER) => {
                    total += crate::sim::update::liquid_fill(cell) as f64 / full;
                }
                // Ice, snow — a whole cell of a solid or powder phase that
                // melts *into water*, worth its own density in water.
                MaterialKind::Solid | MaterialKind::Powder if m.melts_into == Some(material::WATER) => {
                    total += m.density as f64 / water_density;
                }
                // **Water held in soil.** `update::update_soil_water`'s
                // infiltration moves `taken` units straight from a liquid's
                // fill into a `Powder`'s `aux`, one for one, so the two
                // scales are the same scale and the exchange rate is not a
                // choice anyone has to make -- it is what the code already
                // does.
                //
                // This arm is what makes infiltration **ledger-neutral**.
                // Before it, water soaking into ground simply left the
                // books: `STORM_RESERVE`'s own doc measures a soil world
                // settling at 0.54 of full rain supply over 45,000 frames
                // and calls it "a bank with a leak". Half of that leak was
                // never a leak at all, only an un-counted store; the other
                // half is real and is what `evaporation`'s soil path gives
                // back.
                //
                // `water_capacity > 0` rather than a material name, so a
                // second water-holding powder joins automatically -- which
                // is the prerequisite `MaterialDef::water_capacity` names
                // ("widening it later means teaching those tallies about
                // held water first").
                MaterialKind::Powder if m.water_capacity > 0 => {
                    total += crate::sim::update::soil_moisture(cell) as f64 / material::SOIL_SATURATED as f64;
                }
                _ => {}
            }
        }
    }
    total
}

/// Hold the crust cold across the ground the front is over: a band of
/// columns per frame, the band chosen by the frame so that every column is
/// reached once per `CHILL_REVISIT_FRAMES`.
///
/// **A band rather than a scatter**, because the columns are then
/// consecutive and each one's surface can be found from its neighbour's
/// (`surface_near`) instead of re-scanning a 240-row sky each time. And a
/// band rather than every column every frame, because the period is what
/// sets the cost and two frames is all the drift needs.
///
/// Derived from the frame, so it stores nothing and stays a pure function
/// of `(seed, frame)` like the rest of this file -- two runs of the same
/// seed chill the same columns on the same frames.
///
/// **It reaches one cell into open water** (`SWEEP_LIQUID_DEPTH`), which it
/// did not use to, and that one cell is what turns a slush into a sheet.
fn hold_the_ground_cold(world: &mut World, w: Weather, bounds: Rect, snow: Option<material::MaterialId>) {
    // **How hard the front bites: the air mass is a floor, and anything
    // falling can only add to it.**
    //
    // The two halves used to be separate cases -- `intensity` while
    // something was falling, `chill` when nothing was -- and that is
    // discontinuous in the wrong direction: a *light* snowfall inside a
    // cold spell has `intensity * chill` near zero, so it bit less hard
    // than the clear sky either side of it and **cancelled the frost**.
    // Measured: a pond under six thousand frames of unbroken cold froze
    // **zero** cells, because the spell was lightly snowing throughout.
    // Reading it as a floor also fixes the drift: `intensity.max(0.4)`
    // puts `cold` at 9.6 degrees, above snow's own 2-degree melting point,
    // so a drift would rot away on the coldest night of the year.
    let frost = if w.chill > DRY_FROST_CHILL { w.chill } else { 0.0 };
    let bite = if w.is_precipitating() { w.intensity.max(0.4) } else { 0.0 }.max(frost);
    let water_bite = (w.intensity * w.chill).max(frost);
    let cold = (AMBIENT_TEMPERATURE as f32 - SNOW_CHILL * bite) as i16;
    let water_cold = (AMBIENT_TEMPERATURE as f32 - WATER_CHILL * water_bite) as i16;
    let width = bounds.max_x - bounds.min_x + 1;
    let per_frame = (width as u64).div_ceil(CHILL_REVISIT_FRAMES) as i32;
    let band = (world.frame % CHILL_REVISIT_FRAMES) as i32;
    let from = bounds.min_x + band * per_frame;
    let to = (from + per_frame).min(bounds.max_x + 1);
    // The first column of the band pays a full sky walk; the rest follow
    // the surface from their neighbour. A column whose surface is more than
    // `LOCAL_RELIEF` from the last one found -- a cliff, a chasm -- resets
    // to a full walk rather than being skipped, or the far side of every
    // cliff in the world would never be chilled.
    let mut hint = None;
    for x in from..to {
        let found = match hint {
            Some(h) => surface_near(world, x, h, bounds).or_else(|| surface_under_sky(world, x, bounds.min_y, bounds.max_y)),
            None => surface_under_sky(world, x, bounds.min_y, bounds.max_y),
        };
        let Some(y) = found else {
            hint = None;
            continue;
        };
        hint = Some(y);
        hold_column_cold(world, x, y, bounds, snow, cold, water_cold, CRUST_CHILL_DEPTH, SWEEP_LIQUID_DEPTH);
    }
}

/// Hold one column of the surface cold: **one walk down whatever the storm
/// is falling on.**
///
/// Three kinds of cell can appear, in this order from the top, and the walk
/// ends at anything else:
///
/// - **a drift** the storm has already built, held at `cold`. The drift
///   itself is re-chilled, not just the arriving flake: a pile kept cold
///   only at the moment of landing warms from the ground up between flakes
///   and rots away underneath, so the storm has to hold the whole column.
/// - **a crust the cold itself made** out of the liquid below it -- a sheet
///   of ice. Recognised by its melting point sitting *below ambient*, which
///   is `snow.ron`'s way of saying a material exists only while something
///   holds it there, and the storm is the only thing in the world that can.
///   Held at `water_cold` rather than `cold`: without this the surface froze
///   over and then melted from underneath while the snow was still falling
///   on it, because the loop this replaced stopped at the first cell that
///   was not snow -- i.e. at the ice it had just made.
/// - **the liquid itself**, held at `water_cold` for `liquid_depth` cells,
///   which is what freezes it (`fire.rs`'s downward phase change against
///   water.ron's `cooling_point`). `liquid_depth` is 0 for every column the
///   front's own sweep reaches, so only a landing drop can start a freeze.
///
/// The two depths are separate budgets and the caller sets them apart on
/// purpose: `CRUST_CHILL_DEPTH` for the sweep, so a drift on a sheet of ice
/// is held all the way through, and the tighter `SNOW_CHILL_DEPTH` for a
/// drop, so it can only reach water through a *thin* crust and a sheet
/// therefore stops thickening. See both constants.
///
/// Gated on the *materials' own fields*, never on `id_of("water")` -- that
/// is a string hash per cell in the sweep, and a rule that would only ever
/// have worked for one name (`CLAUDE.md`'s hot-path convention). The one id
/// compare left is against the already-hoisted `snow`, and it decides which
/// of two temperatures to write rather than whether to act at all.
///
/// Only the *cells'* own temperature. The field's coarse channel is written
/// once per landing drop by the caller and is a separate thing that does
/// not decide melting or freezing -- `fire::update` compares
/// `cell.temperature` against the thresholds. Writing only the field was
/// the first version of snow and every flake melted on the frame it
/// landed, flooding the surface with meltwater: a snowstorm that produced
/// a lake.
#[allow(clippy::too_many_arguments)]
fn hold_column_cold(
    world: &mut World,
    x: i32,
    surface_y: i32,
    bounds: Rect,
    snow: Option<material::MaterialId>,
    cold: i16,
    water_cold: i16,
    crust_depth: i32,
    liquid_depth: i32,
) {
    let (mut crust_chilled, mut liquid_chilled) = (0, 0);
    // Weighted crust above the water, in ice-equivalent cells -- see
    // `STEFAN_HALVING_DEPTH` and `SNOW_INSULATION`. Accumulated as the walk
    // descends, so by the time it reaches liquid it holds exactly the
    // insulation that liquid is under.
    let mut insulation = 0.0f32;
    // Position-tagged, per this module's rule: one draw per column per
    // visit, so two columns at the same depth do not share a fate and the
    // same world always freezes the same way.
    let mut r = rng::stream(world.seed, 0x53_5445_4641, world.frame, x as u64);
    for d in 0..(crust_depth + liquid_depth) {
        let y = surface_y + d;
        if y > bounds.max_y {
            break;
        }
        let cell = world.get(x, y);
        let (freezable, holds_cold) = {
            let m = world.materials.get(cell.material);
            (
                m.kind == MaterialKind::Liquid && m.cooling_point.is_finite(),
                m.melting_point < AMBIENT_TEMPERATURE as f32,
            )
        };
        // **How thick a sheet gets is decided here**, and it is a rate
        // rather than a wall. The crust already above this water is what
        // slows the water below it freezing -- Stefan's law -- so the walk
        // reaches the liquid with a probability that falls off with the
        // insulation it came through, drawn once on arrival. Repeated over
        // many visits that is a `1/x` growth rate, and thickness that goes
        // as the square root of time. See `STEFAN_HALVING_DEPTH` for why a
        // depth cutoff could not express this and what it cost.
        let conducted = 1.0 / (1.0 + insulation / CRUST_GRADIENT_DEPTH);
        let reaches = 1.0 / (1.0 + insulation / STEFAN_HALVING_DEPTH);
        if freezable {
            if liquid_chilled >= liquid_depth {
                break;
            }
            if liquid_chilled == 0 && insulation > 0.0 && !r.chance(reaches) {
                break;
            }
        }
        if !freezable && !holds_cold {
            break;
        }
        let lying_snow = cell.material == snow.unwrap_or(material::EMPTY);
        // **The same falloff, applied to how cold the cell is actually
        // made** -- which is the linear temperature profile through a slab,
        // and is what makes the sheet *stop*.
        //
        // The rate rule above alone does not stop it. `1/x` growth is slow
        // but unbounded, and measured that way: at a one-cell halving depth
        // the pond went from 944 cell-equivalents of liquid to **180 against
        // 971 frozen** over nine thousand frames, thickening monotonically
        // the whole time. That is the runaway `SHEET_MAX_THICKNESS` was put
        // there to stop, arriving by a slower road.
        //
        // With the gradient, the air's cold reaches a depth and no further:
        // below it the target no longer clears the freezing point, so
        // nothing freezes, and ice already there drifts back toward ambient
        // by ordinary conduction and melts. The sheet settles where chilling
        // and conduction balance, which is what sets the thickness of real
        // lake ice -- and it is set by *how cold the night is* rather than
        // by a constant, so a hard freeze does reach deeper.
        let base = if lying_snow { cold } else { water_cold };
        let target = (AMBIENT_TEMPERATURE as f32 + (base - AMBIENT_TEMPERATURE) as f32 * conducted) as i16;
        if cell.temperature() > target {
            world.set(x, y, cell.with_temperature(target));
        }
        if !freezable {
            insulation += if lying_snow { SNOW_INSULATION } else { 1.0 };
        }
        // Each class keeps its own depth budget, so a drift is chilled
        // exactly as deep as it always was and the water under six cells of
        // snow is insulated from the storm rather than reached through it.
        if freezable {
            liquid_chilled += 1;
        } else {
            crust_chilled += 1;
        }
        if crust_chilled >= crust_depth {
            break;
        }
    }
}

/// The surface of a column near one already found, without re-walking the
/// sky -- see `LOCAL_RELIEF` for why this exists rather than a second
/// `surface_under_sky` call.
///
/// Searches downward from `LOCAL_RELIEF` above the hint, so a drift that
/// has built up beside the hinted column is found at its own top rather
/// than part way into it. `None` means nothing solid within the window,
/// which is a cliff edge or a hole, and the caller stops there rather than
/// hunting: the cold is a local effect and a column whose surface is a long
/// way from its neighbour's is not under the same part of the shower.
fn surface_near(world: &World, x: i32, hint_y: i32, bounds: Rect) -> Option<i32> {
    let from = (hint_y - LOCAL_RELIEF).max(bounds.min_y);
    let to = (hint_y + LOCAL_RELIEF).min(bounds.max_y);
    (from..=to).find(|&y| world.get(x, y).material != material::EMPTY)
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

    /// Prints how long a cold spell lasts and how much of the time the
    /// world is in one, per seed.
    ///
    /// A probe rather than a guard, and the number it exists to settle is a
    /// design one: how long a pond has to freeze in. Reported from play as
    /// *"this should be a different order of magnitude"*, and the answer
    /// depends on a distribution rather than on one seed — outcomes here
    /// are chaotic in the seed (`CLAUDE.md`), so a spell length read off
    /// `scene=coldsnap`'s 2900 is a sample, not the figure.
    ///
    /// `chill` alone, not `kind == Snow`: cold without precipitation is
    /// still cold and is what `hold_the_ground_cold`'s dry arm reads.
    /// The seed `examples/filmstrip.rs`'s `scene=coldsnap` uses, printed
    /// alongside the general population so its hand-found start frame can
    /// be re-derived rather than guessed at.
    const COLDSNAP_PROBE_SEED: u64 = 2900;

    #[test]
    #[ignore = "probe, not a guard"]
    fn probe_cold_spells() {
        const SPAN: u64 = WEATHER_EPOCH_FRAMES * 200;
        for seed in [1, 2, 3, 4, 5, 6, 7, 8, COLDSNAP_PROBE_SEED] {
            let (mut runs, mut run, mut cold_frames, mut start) = (Vec::new(), 0u64, 0u64, 0u64);
            let mut spells: Vec<(u64, u64)> = Vec::new();
            for frame in (0..SPAN).step_by(30) {
                if at(seed, frame).chill > SNOW_THRESHOLD {
                    if run == 0 {
                        start = frame;
                    }
                    run += 30;
                    cold_frames += 30;
                } else if run > 0 {
                    runs.push(run);
                    spells.push((run, start));
                    run = 0;
                }
            }
            if run > 0 {
                runs.push(run);
                spells.push((run, start));
            }
            spells.sort_unstable();
            spells.reverse();
            println!("  seed {seed} longest spells (frames, start): {:?}", &spells[..spells.len().min(5)]);
            runs.sort_unstable();
            let mean = if runs.is_empty() { 0 } else { runs.iter().sum::<u64>() / runs.len() as u64 };
            println!(
                "seed {seed}: {} spells over {} min of play, {:.0}% of the time cold, spell mean {:.0} s median {:.0} s max {:.0} s",
                runs.len(),
                SPAN / 3600,
                100.0 * cold_frames as f64 / SPAN as f64,
                mean as f64 / 60.0,
                runs.get(runs.len() / 2).copied().unwrap_or(0) as f64 / 60.0,
                runs.last().copied().unwrap_or(0) as f64 / 60.0,
            );
        }
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
    // ---- the freezing half of the water cycle (the ice milestone) -------

    /// A pond in a walled basin cut into an attached stone shelf, under
    /// open sky, at a frame `seed` is snowing hard on.
    ///
    /// The shore is `attached` terrain because it is the pond's only
    /// neighbour that anchors, and a shoreline that had to hold itself up
    /// would erode and take the answer about the ice with it. Flush with
    /// the shelf top, so the sky sees the water rather than a lip
    /// (`surface_under_sky` returns the first non-empty cell, so a sunken
    /// pond is a pond under an overhang).
    ///
    /// **Deliberately deeper than `WATER_CHILL_DEPTH`.** The claim under
    /// test is that a cold snap freezes the *surface*, and a pond three
    /// cells deep would freeze to its bed and prove nothing about which
    /// cells the cold reaches.
    fn frozen_pond_world(seed: u64, frame: u64) -> World {
        const SHORE_Y: i32 = 40;
        const DEPTH: i32 = 16;
        let mut w = World::new(Rect::new(0, 0, 127, 79));
        w.seed = seed;
        w.frame = frame;
        for x in 0..128 {
            for y in SHORE_Y..80 {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        for x in POND.0..=POND.1 {
            for y in SHORE_Y..(SHORE_Y + DEPTH) {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        w
    }

    /// Snow lying on ice slows the ice thickening under it.
    ///
    /// # Built as a paired test because no shipped scene exercises it
    ///
    /// `SNOW_INSULATION` was added, swept at 1, 4 and 8, and produced
    /// **bit-identical output at all three** on `scene=coldsnap` -- the tell
    /// `CLAUDE.md` records for a knob that is not connected. It was
    /// connected; the scene simply never presents the situation. Seed 2900's
    /// cold spell is entirely dry, twenty-four thousand frames with **zero
    /// cells of lying snow**, so the pond there freezes under a clear sky and
    /// there is nothing to insulate it. (Snow also never lies on *open*
    /// water, by design -- see `a_snowstorm_leaves_no_snow_raft_insulating
    /// _the_pond` -- so the case needs ice first and snowfall second.)
    ///
    /// So the situation is built rather than found: one pond, already
    /// skinned with ice, with a drift laid over half of it. Both halves get
    /// the same sky and the same frames, which cancels everything the rule
    /// under test is not about.
    #[test]
    fn a_drift_on_the_ice_slows_the_freeze_underneath_it() {
        let seed = 2900;
        let mut w = frozen_pond_world(seed, a_long_cold_spell(seed, 8000));
        let ice = w.materials.id_of("ice").expect("ice.ron should be embedded");
        let snow = w.materials.id_of("snow").expect("snow.ron should be embedded");
        // Skin the whole pond with one row of ice, so both halves start
        // level and the only difference between them is what is on top.
        for x in POND.0..=POND.1 {
            w.set(x, POND_SURFACE, Cell::new(ice, 0));
        }
        // A drift three cells deep over the left half only.
        let mid = (POND.0 + POND.1) / 2;
        for x in POND.0..=mid {
            for d in 1..=3 {
                w.set(x, POND_SURFACE - d, Cell::new(snow, 0));
            }
        }
        advance(&mut w, 3000, true);

        let thickness = |from: i32, to: i32| -> f64 {
            let mut total = 0usize;
            for x in from..=to {
                let mut run = 0;
                for d in 0..16 {
                    if w.get(x, POND_SURFACE + d).material == ice {
                        run += 1;
                    } else if run > 0 {
                        break;
                    }
                }
                total += run;
            }
            total as f64 / (to - from + 1) as f64
        };
        let (under_snow, bare) = (thickness(POND.0, mid), thickness(mid + 1, POND.1));
        println!("ice under a drift {under_snow:.2} cells, bare ice {bare:.2}");
        assert!(bare > 0.0, "test setup: the bare half never thickened, so this compares nothing");
        assert!(
            under_snow < bare,
            "ice under a drift reached {under_snow:.2} cells against {bare:.2} bare -- \
             snow is not insulating, or is accelerating the freeze as it used to"
        );
    }

    /// The pond's columns in `frozen_pond_world`, and its surface row.
    const POND: (i32, i32) = (34, 93);
    const POND_SURFACE: i32 = 40;

    /// A frame at which `seed` is snowing hard enough for snow to lie.
    ///
    /// The intensity bar is not decoration: `SNOW_CHILL` is a magnitude
    /// below ambient, so at intensity 0.7 a landing flake is written at
    /// 2 degrees, which is snow's own melting point, and it melts on the
    /// frame it lands. Any test that wants a drift has to find a real
    /// front, not merely a snowy one.
    fn a_hard_snowy_frame(seed: u64) -> u64 {
        first_frame_with(seed, 0, |w| w.kind == Precipitation::Snow && w.intensity > 0.8)
            .expect("the seeds used by these tests should each have a hard snowfall")
    }

    /// The start of a cold spell that lasts at least `frames`.
    ///
    /// `a_hard_snowy_frame` finds a moment, not a spell, and a moment can
    /// sit at the *end* of one: a test that then runs for thousands of
    /// frames measures the thaw rather than the thing it asked about. Cold
    /// spells here run a mean of 140 seconds and a maximum of twelve
    /// minutes (`probe_cold_spells`), so a long one is always findable.
    fn a_long_cold_spell(seed: u64, frames: u64) -> u64 {
        (0..WEATHER_EPOCH_FRAMES * 64)
            .step_by(60)
            .find(|&from| (from..from + frames).step_by(60).all(|f| at(seed, f).chill > SNOW_THRESHOLD))
            .expect("every seed has a cold spell of a few thousand frames somewhere")
    }

    /// How much water the world holds, in cell-equivalents, in whatever
    /// phase it is in.
    ///
    /// Liquid measured as **fill, not occupancy** (`CLAUDE.md`'s metric
    /// traps: a resting body wears a fringe of near-empty cells), ice and
    /// snow as whole cells, because they are whole cells -- a `Solid`'s
    /// `aux` is an anchor distance and carries no fill, which is why
    /// `fire.rs` will only freeze a near-full cell.
    fn water_census(w: &World) -> (f64, i64, i64) {
        let (mut liquid, mut ice, mut snow) = (0.0f64, 0i64, 0i64);
        let ice_id = w.materials.id_of("ice");
        let snow_id = w.materials.id_of("snow");
        for y in 0..80 {
            for x in 0..128 {
                let cell = w.get(x, y);
                if cell.material == material::WATER {
                    liquid += crate::sim::update::liquid_fill(cell) as f64 / material::LIQUID_FULL as f64;
                } else if Some(cell.material) == ice_id {
                    ice += 1;
                } else if Some(cell.material) == snow_id {
                    snow += 1;
                }
            }
        }
        (liquid, ice, snow)
    }

    fn advance(w: &mut World, frames: usize, parallel_driver: bool) {
        for _ in 0..frames {
            if parallel_driver {
                parallel::step(w);
            } else {
                crate::sim::update::step(w);
            }
        }
    }

    #[test]
    fn pond_freezes_at_the_surface_in_a_cold_snap() {
        // Both drivers, per `CLAUDE.md`: `App::update` calls the parallel
        // one, so behaviour only the player sees is behaviour only that
        // driver produces -- and this mechanism runs entirely inside the
        // sweep, through `fire::update`.
        //
        // Asserted as **surface against interior**, not as a cell count: a
        // count says "some ice appeared" and would pass just as well if the
        // whole pond froze solid, which is the wrong behaviour and the one
        // `WATER_CHILL_DEPTH` exists to bound.
        for parallel_driver in [false, true] {
            let seed = 2900;
            let mut w = frozen_pond_world(seed, a_hard_snowy_frame(seed));
            let ice = w.materials.id_of("ice").expect("ice.ron should be embedded");
            advance(&mut w, 400, parallel_driver);

            // **Ice within the top few rows, not in the top row.**
            // Measured before this was written that way: a fixed-row
            // reading found 7 of 60 columns frozen while the pond plainly
            // had a crust on it, because water evaporates from its own
            // surface and a settled pond's top row is a fringe of
            // partly-filled cells that `FREEZE_MIN_FILL` refuses to
            // freeze. A column reads `water, ice, ice, ice, ice, water`
            // from row 40 down -- a film over a sheet, which is what a
            // frozen pond looks like here and is the case fire.rs's fill
            // gate documents as the accepted loss of coverage.
            let surface_ice = (POND.0..=POND.1)
                .filter(|&x| (POND_SURFACE..POND_SURFACE + 4).any(|y| w.get(x, y).material == ice))
                .count();
            let deep_ice = (POND.0..=POND.1).filter(|&x| w.get(x, POND_SURFACE + 14).material == ice).count();
            let deep_water = (POND.0..=POND.1).filter(|&x| w.get(x, POND_SURFACE + 14).material == material::WATER).count();
            assert!(
                surface_ice > 20,
                "only {surface_ice} of 60 surface columns froze in a hard cold snap (parallel: {parallel_driver}); froze {}",
                w.phase_changes.froze
            );
            assert_eq!(deep_ice, 0, "the cold reached 14 cells down; freezing is a surface effect (parallel: {parallel_driver})");
            assert!(
                deep_water > 50,
                "the pond's interior should still be water, found {deep_water} of 60 columns (parallel: {parallel_driver})"
            );
        }
    }

    /// **A freeze that sticks.** Almost every freeze event must still be
    /// ice at the end of the spell.
    ///
    /// Reported from play: *"it never really freezes and has snow
    /// accumulate on top. The pixels seem to be constantly shifting."* The
    /// cell count could not see it — a pond can hold three hundred ice
    /// cells forever as a **churning slush** and never close into a sheet.
    /// Measured on `scene=coldsnap` before `SWEEP_LIQUID_DEPTH`: **491
    /// freezes and 510 melts across 340 frames for a net of minus
    /// nineteen**, with the band stuck at a quarter of the pond.
    ///
    /// So the quantity is gross events per net cell, and it is ~1.0 when
    /// the front is doing what it looks like it is doing. Two is a wide
    /// bar: this fixture is a real front over a real pond and a little
    /// re-freezing at the edges is not the artifact.
    #[test]
    fn a_freezing_pond_is_not_a_churning_slush() {
        let seed = 2900;
        let mut w = frozen_pond_world(seed, a_hard_snowy_frame(seed));
        advance(&mut w, 1200, true);
        let (_, ice, _) = water_census(&w);
        assert!(ice > 100, "test setup: only {ice} cells froze, so there is nothing to be churning");
        let froze = w.phase_changes.froze as f64;
        assert!(
            froze <= ice as f64 * 2.0,
            "{froze} freeze events left {ice} cells of ice standing ({:.1} events per cell) -- \
             the pond is cycling rather than freezing",
            froze / ice as f64
        );
    }

    /// **A clear freezing night freezes standing water**, and a clear mild
    /// one does not.
    ///
    /// `Weather::chill`'s own doc has said since it was written that *"a
    /// clear winter night is different from a clear summer one"*, and until
    /// `DRY_FROST_CHILL` nothing acted on it: `step` returned early
    /// whenever nothing was falling, so the only thing in the world that
    /// could freeze water was a landing flake. A pond therefore had only
    /// the overlap of "snowing hard" and "cold" to freeze in, which is a
    /// small fraction of a cold spell.
    ///
    /// The paired negative is what makes this a test of the *cold* rather
    /// than of the passage of time: the same pond on a clear mild frame
    /// must stay open.
    #[test]
    fn a_clear_cold_night_freezes_a_pond_and_a_clear_mild_one_does_not() {
        let seed = 2900;
        let ice_at = |pick: fn(Weather) -> bool| {
            let frame = first_frame_with(seed, 0, pick).expect("seed 2900 should have this kind of frame");
            let mut w = frozen_pond_world(seed, frame);
            advance(&mut w, 1200, true);
            water_census(&w).1
        };
        let cold = ice_at(|x| !x.is_precipitating() && x.chill > DRY_FROST_CHILL + 0.05);
        let mild = ice_at(|x| !x.is_precipitating() && x.chill < DRY_FROST_CHILL - 0.2);
        assert!(cold > 100, "a clear freezing night made only {cold} cells of ice");
        assert_eq!(mild, 0, "a clear mild night made {mild} cells of ice");
    }

    /// How long a spell the guard below watches, in frames.
    ///
    /// **Two thousand, and the number is a finding rather than a
    /// convenience.** Under *unbroken hard* cold this fixture's 16-deep
    /// pond does reach its bed, between 3,000 and 4,000 frames, and
    /// saturates at 898 cells of ice — conduction through the sheet carries
    /// the cold down after `SHEET_MAX_THICKNESS` has stopped the sweep
    /// reaching the water directly, and bounding *that* would mean bounding
    /// `diffuse_heat`. Left alone deliberately: a 16-deep pond under a
    /// minute of hard freeze reaching its bed is not an artifact anyone
    /// reported, and real weather varies.
    const PROBE_SPELL: usize = 2000;

    /// **A sheet, with a pond still under it**, thousands of frames into a
    /// spell — which `pond_freezes_at_the_surface_in_a_cold_snap` cannot
    /// see, because it runs 400 frames and the freeze now takes minutes.
    ///
    /// **This is not the guard for `SHEET_MAX_THICKNESS`**, and that was
    /// checked rather than assumed: it passes with the cap removed, for the
    /// reason `PROBE_SPELL` records. The cap is guarded in
    /// `scripts/acceptance.sh`'s `coldsheet` case, on a scene where snow
    /// lies — there it is worth 450 cells of ice against 823.
    #[test]
    fn a_long_cold_spell_leaves_water_under_the_ice() {
        let seed = 2900;
        // **The start of a long spell, not the first hard snowfall.** A
        // hard snowfall can sit at the end of a spell, and this test then
        // measures the thaw: `a_hard_snowy_frame` left 38 cells of ice at
        // frame 6,000 and the setup assertion caught it.
        let mut w = frozen_pond_world(seed, a_long_cold_spell(seed, 8000));
        advance(&mut w, PROBE_SPELL, true);
        let (liquid, ice, _) = water_census(&w);
        assert!(ice > 100, "test setup: only {ice} cells froze over six thousand frames of unbroken cold");
        // Measured 674 of water under 281 of ice at this point. The bar is
        // two rows' worth of the pond's width, well under a third of that:
        // a bar sitting on the measurement would flake on the spell this
        // seed happens to find.
        let floor = (POND.1 - POND.0 + 1) as f64 * 2.0;
        assert!(
            liquid > floor,
            "after a long cold spell the pond holds {liquid:.0} cell-equivalents of water under {ice} of ice; \
             less than {floor:.0} is a pond frozen to its bed rather than a sheet on one"
        );
    }

    #[test]
    fn ice_melts_back_and_the_pool_refills() {
        // The round trip, as a **bound rather than an equality**, for two
        // reasons that both matter: `fire.rs` will not freeze a cell below
        // `FREEZE_MIN_FILL`, so the partial fringe at the surface never
        // becomes ice and never comes back as a full cell; and the storm
        // drops snow on the shore that melts into the pond as well, so the
        // world ends with *more* water than it started with rather than the
        // same. What must not happen is water going missing.
        for parallel_driver in [false, true] {
            let seed = 2900;
            let snowy = a_hard_snowy_frame(seed);
            let mut w = frozen_pond_world(seed, snowy);
            let (before, _, _) = water_census(&w);

            advance(&mut w, 400, parallel_driver);
            let (during, ice, _) = water_census(&w);
            assert!(ice > 20, "nothing froze, so there is no thaw to measure (parallel: {parallel_driver})");
            assert!(
                during < before - 10.0,
                "freezing should take real volume out of the liquid tally: {during:.1} against {before:.1} (parallel: {parallel_driver})"
            );

            // The front passes. Jump the clock rather than waiting out a
            // real front: weather is a pure function of `(seed, frame)`, so
            // moving the frame *is* the front passing, and it keeps the
            // test to a few hundred frames instead of a few thousand.
            w.frame = first_frame_with(seed, snowy, |x| !x.is_precipitating()).expect("seed 2900's front should end");
            // **1,500 frames for the thaw, not 300.** Melting is a per-visit
            // roll at `fire::MELT_CHANCE` now — a mean of ~67 frames per
            // cell, against the single visit it used to take — because a
            // pond that thawed in a fifth of a second was reported from
            // play. The budget is read off that rate rather than guessed:
            // measured on `scene=coldsnap`, a 356-cell sheet is gone in
            // about 480 frames.
            advance(&mut w, 4000, parallel_driver);

            let (after, ice_left, snow_left) = water_census(&w);
            println!("pool refill (parallel: {parallel_driver}): {after:.1} against {before:.1} ({:.1}%)", after / before * 100.0);
            assert_eq!(ice_left, 0, "every cell of ice should be above its melting point once the front has gone (parallel: {parallel_driver})");
            assert_eq!(snow_left, 0, "and so should every flake (parallel: {parallel_driver})");
            // **A bound, and the direction of the slack is stated.** The
            // pool comes back a little *under* what it started at, not
            // over: this test world is small enough that the snow melting
            // into it does not make up for the fringe that never froze.
            //
            // **The 90% bar was written when a melting flake manufactured a
            // full cell of water, and the number under it moved when that
            // was fixed** -- `CLAUDE.md`'s "fixing a bug often exposes a
            // constant that was compensating for it". It read 941.9 against
            // 960.0 (98.1%) with the drift quietly topping the pond up by
            // more than it was worth; with `fire::melt_fill` in place a
            // flake is worth its own 0.3 and the same run reads **925.4
            // against 960.0, 96.4%** (97.2% under the parallel driver).
            // Still comfortably inside the bar, because the ice half of the
            // loop closes exactly and the snow half was never what this
            // test was measuring. Kept at 90% on that evidence rather than
            // moved for the sake of moving.
            //
            // Also worth recording, because a plausible alternative fix
            // would have broken this badly: density-scaling the *ice* melt
            // as well (ice 0.92, so `1000 -> ice -> 920`) takes this run to
            // **788.4, 82.1%**. Evaporation is not the difference -- the
            // control, this same pond over 700 frames with no front, holds
            // 960.0 to 960.0 -- it is 8% off every cell that freezes full,
            // compounding over a surface that cycles ten times in a storm.
            // See `fire::density_scaled_fill` for why that is not what
            // shipped.
            assert!(
                after > before * 0.90,
                "the pool did not refill: {after:.1} against {before:.1} before the freeze (parallel: {parallel_driver})"
            );
            assert!(
                after > during,
                "the thaw put nothing back: {after:.1} against {during:.1} while frozen (parallel: {parallel_driver})"
            );
            assert!(
                w.phase_changes.melted > 0 && w.phase_changes.froze > 0,
                "counters say the mechanism never fired (parallel: {parallel_driver})"
            );
        }
    }

    /// How long a world gets to settle after a cold spell lifts, in frames.
    ///
    /// Re-derived from measurement when `MELT_CHANCE` was slowed: a sheet
    /// now takes about 1,200 frames to go, so a 1,200-frame budget was
    /// measuring the thaw rather than whether anything was *stuck*. Set at
    /// four times the measured thaw.
    const THAW_SETTLE_BUDGET: u32 = 4800;

    #[test]
    fn thawed_world_sleeps() {
        // **The reason snow.ron gained a `heat_conductivity` at all.** A
        // cell the storm chilled and nothing can warm sits permanently off
        // ambient, and `fire.rs`'s `must_stay_dirty` then keeps its chunk
        // awake for the rest of the run -- so a world that has been snowed
        // on never sleeps again, and the dirty-rect render skip is gone
        // with it. Ice is a far larger version of the same cell, which is
        // why ice.ron has one too.
        //
        // **This test can fail, and that was checked rather than assumed**
        // (`CLAUDE.md`: a guard that cannot fail is not a guard). It sleeps
        // 42 frames after the front passes as shipped. Setting ice.ron's
        // `heat_conductivity` to 0.0 locally and rebuilding leaves 2 chunks
        // awake at the 1,200-frame budget; taking snow.ron's back to 0 --
        // the state this milestone found it in -- does exactly the same.
        // Either material alone is enough to hold a world awake forever.
        let seed = 2900;
        let snowy = a_hard_snowy_frame(seed);
        let mut w = frozen_pond_world(seed, snowy);
        advance(&mut w, 400, true);
        assert!(w.phase_changes.froze > 0, "nothing froze, so there is nothing to thaw and the test is vacuous");

        // **"The front passed" is not "nothing is falling" any more.**
        // `DRY_FROST_CHILL` gave the cold air mass its own effect, so a
        // clear frame inside a cold spell still holds the pond frozen --
        // correctly, and this test read it as a world that could not
        // settle. It has to wait for the *cold* to lift, not the snow.
        w.frame = first_frame_with(seed, snowy, |x| !x.is_precipitating() && x.chill <= SNOW_THRESHOLD)
            .expect("seed 2900's cold spell should end");
        for frame in 0..THAW_SETTLE_BUDGET {
            parallel::step(&mut w);
            if w.active_chunk_count() == 0 {
                println!("the world went to sleep {frame} frames after the front passed");
                return;
            }
        }
        panic!(
            "{} chunks were still awake {THAW_SETTLE_BUDGET} frames after the cold lifted -- something it touched cannot settle",
            w.active_chunk_count()
        );
    }

    #[test]
    fn a_snowstorm_leaves_no_snow_raft_insulating_the_pond() {
        // Snow is lighter than water, so a flake landing on a pond floats,
        // and flakes keep coming: a raft builds up, `surface_under_sky`
        // starts returning *it* instead of the water, and once it is
        // `SNOW_CHILL_DEPTH` deep the cold never reaches the pond at all.
        // Measured before the refusal went in: 9 to 20 cells of ice over a
        // whole storm, against a 60-cell surface.
        //
        // **Rewritten from a proxy to the property, because the proxy moved
        // for a reason that was not the artifact.** It used to count pond
        // columns holding a snow cell directly above *any* liquid cell in
        // the top 16 rows, bar 6. Fixing `fire::transform`'s melt fill took
        // that count from 5 to 11 -- and the reason is that the pond is no
        // longer brimming with meltwater a melting flake manufactured out
        // of nothing. Before, the surface stood at the shore line and the
        // metric saw open water; after, it sits lower under a thicker
        // crust, and the same two grains that rolled off the bank now sit
        // over a thin meltwater layer *on top of the ice*. The pond did not
        // stop freezing, which is what the count was standing in for.
        //
        // So the guard now asserts the two things a raft would actually do,
        // either of which can still fail: it would be **deep** (that is
        // what insulates, and the bar is `SNOW_CHILL_DEPTH` itself rather
        // than a number picked near the measured 2 -- at that depth the
        // chill walk spends its whole budget inside the drift and the water
        // below stops being chilled at all), and the pond **would not
        // freeze** (the bar is 100 against 167 measured here, and against
        // the 9-to-20 the pre-refusal artifact produced -- the standing ice
        // count at a fixed frame is a mid-freeze sample and moves by a
        // factor of two on any change to the melt, so the bar is set well
        // clear of it rather than under it).
        let seed = 2900;
        let mut w = frozen_pond_world(seed, a_hard_snowy_frame(seed));
        let snow = w.materials.id_of("snow").unwrap();
        advance(&mut w, 400, true);

        // The deepest run of snow lying on the pond's own surface -- the
        // topmost thing the sky sees in that column, not a flake buried
        // anywhere in the crust.
        let deepest = (POND.0..=POND.1)
            .map(|x| {
                let Some(top) = surface_under_sky(&w, x, 0, 79) else {
                    return 0;
                };
                (top..80).take_while(|&y| w.get(x, y).material == snow).count()
            })
            .max()
            .unwrap_or(0);
        let (_, ice, _) = water_census(&w);
        println!("deepest raft on the pond: {deepest} cells; ice standing: {ice}");
        assert!(
            (deepest as i32) < SNOW_CHILL_DEPTH,
            "a raft {deepest} cells deep is lying on the pond; at {SNOW_CHILL_DEPTH} the cold stops reaching the water at all"
        );
        assert!(ice > 100, "only {ice} cells of ice formed over the storm; something on the surface is insulating the pond");
    }

    // `water_equivalents` used to live here, `#[cfg(test)]` and hardcoded to
    // this module's 128x80 scene. It is a real function in the module above
    // now (reached through `use super::*`), because the bank made it the
    // standing half of a conservation law rather than one test's helper:
    // `examples/filmstrip.rs` prints it beside the bank on every scene, and
    // a census the harness and the guards do not share is a census that can
    // disagree with itself.

    #[test]
    fn a_thaw_does_not_manufacture_water() {
        // **The conservation bound nothing here was watching for**, because
        // every water test written before it was written against *loss*:
        // `ice_melts_back_and_the_pool_refills` asserts the pool comes back
        // to at least 90% and would pass just as happily at 400%.
        //
        // The error it catches runs the other way. `fire::transform`'s aux
        // table wrote a flat 0 for any pair that is not Liquid→Liquid or
        // Liquid↔Gas, and 0 on a `Liquid` means **full** — so a snowflake at
        // density 0.3 melted into a full water cell and gained a factor of
        // 3.3, and a cell of ice at 0.92 gained 8.7%. Measured on this
        // world before the fix: 949.5 cell-equivalents standing when the
        // front passed, 1,171.9 once it had all thawed — **123.4%**, water
        // out of nothing. After: 960.8 → 998.3, **103.9%** (the standing
        // figure moves between the two runs because the broken melt was
        // itself feeding the pond meltwater all through the storm).
        //
        // Both drivers, because `App::update` runs the parallel one.
        for parallel_driver in [false, true] {
            let seed = 2900;
            let snowy = a_hard_snowy_frame(seed);
            let mut w = frozen_pond_world(seed, snowy);

            // Let the front lay a drift and freeze the surface. The census
            // is taken *after* the storm, not before it: precipitation
            // creates material out of the sky by design, so a baseline
            // taken before the snowfall would be measuring the weather
            // rather than the phase change.
            //
            // **1,200 frames and not 400**, which the freeze-over tests get
            // away with, because a drift is the slower half of the two:
            // snow refuses to lie on open water, so nothing accumulates on
            // the pond until it has frozen over, and the bank alone is
            // narrow here. Measured trajectory (ice / snow standing):
            // 145/1 at 400, 436/41 at 700, 470/264 at 1,200. At 400 the
            // ice alone would have inflated the tally by ~1%, comfortably
            // inside this bound -- the test would have run, passed against
            // the broken code, and proved nothing.
            advance(&mut w, 1200, parallel_driver);
            w.frame = first_frame_with(seed, snowy, |x| !x.is_precipitating()).expect("seed 2900's front should end");
            let before = water_equivalents(&w);
            let (_, ice, snow) = water_census(&w);
            assert!(ice > 20 && snow > 100, "no sheet ({ice}) or no drift ({snow}) to thaw; the test would be vacuous (parallel: {parallel_driver})");

            // Same rate change as the sibling test above: 2,400 frames,
            // and a drift of several hundred flakes is the slow half.
            advance(&mut w, 5000, parallel_driver);
            let after = water_equivalents(&w);
            let (_, ice_left, snow_left) = water_census(&w);
            assert_eq!((ice_left, snow_left), (0, 0), "the thaw did not finish, so this is not measuring one (parallel: {parallel_driver})");
            assert!(w.phase_changes.melted > 0, "counters say nothing melted at all (parallel: {parallel_driver})");

            // **110% against a measured 103.9%**, and the residual is
            // understood rather than tolerated: a reciprocal freeze comes
            // back full (`fire::melt_fill`) on the strength of
            // `MaterialDef::freeze_min_fill`, so a cell that froze at the
            // gate's 900 rather than at 1,000 gives back 11% more than it
            // took. That is a structural ceiling, not a tuning knob --
            // `LIQUID_FULL - freeze_min_fill` -- and closing it would mean
            // paying a matching loss on the far commoner full cell, which
            // is measurably worse (see `fire::density_scaled_fill`).
            //
            // It can still fail by 13 points, which is what the pre-fix run
            // establishes. The slack is one-sided on purpose: loss is
            // `ice_melts_back_and_the_pool_refills`'s job and this must not
            // quietly take it over.
            println!("thaw (parallel: {parallel_driver}): {before:.1} -> {after:.1} cell-equivalents ({:.1}%)", after / before * 100.0);
            assert!(
                after <= before * 1.10,
                "the thaw manufactured water: {after:.1} cell-equivalents against {before:.1} standing before it (parallel: {parallel_driver})"
            );
        }
    }

    // ---------------------------------------------------------------
    // The water bank: the sky spends what evaporation has banked.
    // ---------------------------------------------------------------

    /// A 512-wide world with a flat stone shelf under open sky — the width
    /// `examples/filmstrip.rs` runs at, which is what `STORM_RESERVE` is
    /// quoted against.
    ///
    /// **Attached stone**, so the shelf is terrain rather than something
    /// stacked in front of it: an unattached slab has to hold itself up and
    /// erodes inward from every free face, which would slowly change where
    /// `surface_under_sky` finds the ground underneath a measurement that
    /// runs for thousands of frames.
    /// `soil` picks which of the two surfaces a storm can land on, and the
    /// difference between them is enormous rather than cosmetic — see
    /// `probe_storm_yield`.
    fn open_shelf_world(seed: u64, frame: u64, soil: Option<u16>) -> World {
        const SHELF_Y: i32 = 240;
        const SOIL_DEPTH: i32 = 24;
        let mut w = World::new(Rect::new(0, 0, 511, 319));
        w.seed = seed;
        w.frame = frame;
        let soil_id = w.materials.id_of("soil").expect("soil must exist");
        for x in 0..512 {
            for y in SHELF_Y..320 {
                let cell = match soil {
                    Some(wetness) if y < SHELF_Y + SOIL_DEPTH => Cell::new(soil_id, 0).with_aux(wetness),
                    _ => Cell::new(material::STONE, 0),
                };
                w.set(x, y, cell.with_attached(true));
            }
        }
        w
    }

    /// The frame one frame after the front that starts at or after `from`
    /// has finished — so `(start, end)` brackets exactly one storm.
    fn one_storm(seed: u64, from: u64) -> Option<(u64, u64)> {
        let start = first_frame_with(seed, from, |w| w.is_precipitating())?;
        let end = first_frame_with(seed, start + 60, |w| !w.is_precipitating())?;
        Some((start, end))
    }

    #[test]
    #[ignore = "probe, not a guard"]
    fn probe_find_a_dry_lead_before_a_storm() {
        // **What `filmstrip`'s `watercycle` scene's frame window is picked
        // from.** That scene has to show the sky filling across whole days
        // *before* a storm empties it again, so it needs a dry lead of at
        // least two full 3,600-frame days ending at a front — which is a
        // property of `(seed, frame)` alone and so costs nothing to search.
        //
        // Reported per seed rather than picking one here, because which
        // window reads best on a contact sheet is a judgement about the
        // picture and not about the numbers.
        println!("seed   storm start    end   dry lead (frames / days)");
        for seed in [7u64, 20, 2900, 12345, 31337] {
            let mut from = 0u64;
            for _ in 0..6 {
                let Some((coarse, end)) = one_storm(seed, from) else { break };
                // `first_frame_with` walks the frame axis in steps of 60, so
                // the frame it reports is up to a minute *inside* the front.
                // Walk back to where it really began before measuring the dry
                // lead in front of it -- reading the lead from the coarse
                // frame reports zero for every storm in the table, which is
                // what this probe did first and what a plain "0.00 everywhere"
                // column should always be read as.
                let mut start = coarse;
                while start > 0 && at(seed, start - 1).is_precipitating() {
                    start -= 1;
                }
                let mut lead = 0u64;
                while lead < start && !at(seed, start - lead - 1).is_precipitating() {
                    lead += 1;
                }
                println!(
                    "{seed:5}  {start:11}  {end:5}   {lead:8} / {:.2}{}",
                    lead as f64 / DAY_NIGHT_PERIOD_FRAMES as f64,
                    if lead >= 2 * DAY_NIGHT_PERIOD_FRAMES { "   <- two clear days" } else { "" }
                );
                from = end + 1;
            }
        }
    }

    #[test]
    #[ignore = "probe, not a guard"]
    fn probe_storm_yield() {
        // **What `STORM_RESERVE` is set from.** One whole front, end to end,
        // over a 512-wide world, with the bank pinned full every frame so
        // the supply factor is 1.0 throughout and the storm runs
        // unthrottled — the question is what a storm *wants* to spend, not
        // what a throttled one does spend.
        //
        // Measured as the drop across `parallel::step` alone, which is where
        // `weather::step` lives; the scheduler (and therefore evaporation's
        // credit) runs after and is read separately, so the two directions
        // never net each other out. A single before/after on the bank would
        // have reported debits minus credits and called it yield.
        //
        // Swept over seeds and reported as a max, not a mean, per
        // `CLAUDE.md`: outcomes here are chaotic in the seed, and a constant
        // set from one sample is a sample from a wide distribution.
        // **Two surfaces, because the difference between them is the whole
        // spread of the answer and neither one alone is the number.** Rain
        // refuses to stack on standing water, so a storm over bare rock
        // covers the shelf with its own first few hundred cells and then
        // delivers essentially nothing for the rest of the front. Over soil
        // the landing cells are swallowed by infiltration, the surface never
        // becomes liquid, and the storm spends at its full rate from the
        // first frame to the last. A reserve sized off the rock figure would
        // be a reserve sized off a storm that had throttled *itself*.
        println!("surface  seed   frames   spawned (cell-equivalents)   per 1000 frames");
        let mut worst = 0.0f64;
        for (label, soil) in
            [("rock", None), ("drysoil", Some(0)), ("dampsoil", Some(material::SOIL_FIELD_CAPACITY))]
        {
            for seed in [7u64, 20, 2900, 12345, 31337] {
                let Some((start, end)) = one_storm(seed, 0) else { continue };
                let mut w = open_shelf_world(seed, start, soil);
                let mut spent = 0.0f64;
                for _ in start..end {
                    let before = w.atmospheric_bank;
                    parallel::step(&mut w);
                    spent += (before - w.atmospheric_bank).max(0.0);
                    w.step_active_sites();
                    w.step_fields();
                    w.atmospheric_bank = STORM_RESERVE;
                }
                let frames = end - start;
                println!("{label:7}  {seed:5}  {frames:6}   {spent:26.1}   {:.1}", spent * 1000.0 / frames as f64);
                worst = worst.max(spent);
            }
        }
        println!("worst storm: {worst:.1} cell-equivalents; STORM_RESERVE is {STORM_RESERVE:.0} ({:.1} storms)", STORM_RESERVE / worst.max(1.0));
    }

    #[test]
    #[ignore = "probe, not a guard"]
    /// How wet rain actually makes the ground, which is the look
    /// `SOIL_SOAK_PER_DROP` exists for and the thing charging it to the
    /// bank puts at risk. Prints the mean saturation of the top soil row
    /// through one front, as a fraction of `SOIL_SATURATED`.
    fn probe_ground_wetness_under_a_storm() {
        let seed = 12345;
        let start = first_frame_with(seed, 0, |w| w.kind == Precipitation::Rain && w.intensity > 0.5)
            .expect("this seed has a wet epoch in it");
        let mut w = open_shelf_world(seed, start, Some(0));
        let top = (0..w.bounds().unwrap().max_y)
            .find(|&y| (0..512).any(|x| w.materials.get(w.get(x, y).material).water_capacity > 0))
            .expect("the shelf has soil in it");
        println!("wetness: frame  mean top-row saturation  bank");
        for step in 0..8 {
            advance(&mut w, 500, true);
            let cells: Vec<u16> = (0..512)
                .filter(|&x| w.materials.get(w.get(x, top).material).water_capacity > 0)
                .map(|x| crate::sim::update::soil_moisture(w.get(x, top)))
                .collect();
            let mean = cells.iter().map(|&m| m as f64).sum::<f64>() / cells.len().max(1) as f64;
            println!(
                "wetness: {:6}  {:.3}  {:.0}",
                start + (step + 1) * 500u64,
                mean / material::SOIL_SATURATED as f64,
                w.atmospheric_bank
            );
        }
    }

    #[test]
    #[ignore = "probe, not a guard"]
    fn probe_long_run_balance() {
        // **Where the balance actually goes over a long run**, which no
        // single-storm number can answer and which is the question the
        // owner would ask second: does a world settle into a cycle, or does
        // the sky simply run out?
        //
        // Two surfaces, and the contrast is the whole readout. Bare rock is
        // the closed case — rain puddles, puddles evaporate, the credit
        // comes back. Soil is the leaky case: infiltration takes a landing
        // cell's fill into wetness and nothing credits it back, so whatever
        // the sky spends over soil is spent for good. Run side by side so
        // the difference is attributable rather than inferred.
        for (label, soil) in [("rock", None), ("soil", Some(material::SOIL_FIELD_CAPACITY))] {
            let seed = 12345;
            let mut w = open_shelf_world(seed, 0, soil);
            println!("{label}: frame   bank   supply   standing water");
            for block in 0..12 {
                for _ in 0..5_000 {
                    parallel::step(&mut w);
                    w.step_active_sites();
                    w.step_fields();
                }
                println!(
                    "{label}: {:6}  {:6.0}  {:5.2}  {:8.1}",
                    (block + 1) * 5_000,
                    w.atmospheric_bank,
                    w.storm_supply(),
                    water_equivalents(&w)
                );
            }
        }
    }

    #[test]
    fn a_bankrupt_sky_spawns_no_water() {
        // **The "and then it stops" test, written first and confirmed red
        // against the pre-bank code**, per `Reports/weather-handoff.md`'s
        // single lesson: every guard this area has produced tested that a
        // mechanism fires and none tested that it stops. Before the gate was
        // wired, this exact world -- bank at zero, a hard front overhead --
        // filled with rain regardless, because rain came out of nothing and
        // nothing was counting.
        //
        // Two halves, and the second is the one that would have been left
        // out. Zero *spawned* is the simulation half. Zero *drawn supply* is
        // the render half, and it is not cosmetic: falling precipitation is
        // drawn straight from `at(seed, frame)` and is never simulated, so a
        // gate on the landing side alone leaves a bankrupt sky drawing a
        // full downpour that deposits nothing, for as long as the front
        // lasts.
        for (label, seed, kind) in [("rain", 12345u64, Precipitation::Rain), ("snow", 2900, Precipitation::Snow)] {
            let start =
                first_frame_with(seed, 0, |w| w.kind == kind && w.intensity > 0.8).expect("these seeds each have a hard front");
            let mut w = open_shelf_world(seed, start, None);
            w.atmospheric_bank = 0.0;

            assert_eq!(w.storm_supply(), 0.0, "{label}: a drained bank must draw no rain either");
            let before = water_equivalents(&w);
            assert_eq!(before, 0.0, "{label}: the shelf world starts dry, or this measures the wrong thing");

            for _ in 0..600 {
                parallel::step(&mut w);
            }
            let spawned = water_equivalents(&w);
            assert_eq!(w.atmospheric_bank, 0.0, "{label}: the bank went negative -- {}", w.atmospheric_bank);
            assert_eq!(spawned, 0.0, "{label}: a bankrupt sky put {spawned:.1} cell-equivalents into the world");

            // The control. Same seed, same frames, same scene, bank full --
            // and it must deposit something, or the assertion above passed
            // because the front was not raining rather than because the gate
            // held. `CLAUDE.md`: an exactly-zero delta means suspect the
            // condition you keyed on is degenerate, before concluding the
            // lever works.
            let mut c = open_shelf_world(seed, start, None);
            for _ in 0..600 {
                parallel::step(&mut c);
            }
            let control = water_equivalents(&c);
            println!("{label}: bankrupt sky deposited {spawned:.1}, full sky deposited {control:.1}");
            assert!(control > 0.0, "{label}: the control front deposited nothing either, so this guard is vacuous");
        }
    }

    #[test]
    fn a_thinning_sky_thins_the_drawn_rain_by_the_same_factor() {
        // The other half of the render agreement, at the settings between
        // the two ends: the drawn storm and the landing storm have to scale
        // *together*, not merely switch off together. `render.rs` multiplies
        // this factor into the intensity it hands `sky::drops`, which thins
        // the drop field rather than dimming it, and `step` multiplies the
        // identical factor into its column budget.
        //
        // Asserted on the factor itself rather than on pixels, because what
        // can drift here is the two call sites reading *different* numbers,
        // and a pixel count cannot tell that apart from a tuning change.
        let mut w = open_shelf_world(12345, 0, None);
        for (bank, want) in [(0.0, 0.0), (STORM_RESERVE / 4.0, 0.25), (STORM_RESERVE, 1.0), (STORM_RESERVE * 3.0, 1.0)] {
            w.atmospheric_bank = bank;
            assert!(
                (w.storm_supply() - want).abs() < 1e-6,
                "a bank of {bank} should supply {want}, got {}",
                w.storm_supply()
            );
        }
        // Clamped at both ends: a surplus does not make it rain harder than
        // the weather says, and a deficit cannot go negative and invert the
        // drawn field.
        assert_eq!(supply(-1.0), 0.0);
    }

    /// A walled stone basin holding a pond, under open sky.
    ///
    /// **Stone and not soil, deliberately.** `update::update_soil_water`
    /// turns a landing water cell's fill into soil wetness and nothing ever
    /// credits it back -- a real, pre-existing, one-way sink out of the water
    /// ledger (see `STORM_RESERVE`'s own doc). A conservation guard built on
    /// soil would measure that leak instead of the thing it is named for.
    ///
    /// **Walled**, for the reason `evaporation_tests.rs`'s scene is walled:
    /// an unwalled pond spreads away across the floor and the test measures
    /// spreading. No lid, though -- this one *wants* the rain.
    fn walled_basin_world(seed: u64, frame: u64) -> World {
        const FLOOR_Y: i32 = 100;
        const DEPTH: i32 = 6;
        const BASIN: (i32, i32) = (40, 200);
        let mut w = World::new(Rect::new(0, 0, 255, 127));
        w.seed = seed;
        w.frame = frame;
        for x in 0..256 {
            for y in FLOOR_Y..128 {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        for y in (FLOOR_Y - DEPTH)..FLOOR_Y {
            w.set(BASIN.0 - 1, y, Cell::new(material::STONE, 0).with_attached(true));
            w.set(BASIN.1 + 1, y, Cell::new(material::STONE, 0).with_attached(true));
            for x in BASIN.0..=BASIN.1 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        w
    }

    /// The same conservation law **over soil**, which is the case the
    /// stone basin exists to avoid.
    ///
    /// `walled_basin_world`'s own doc says why it is stone: infiltration
    /// used to be a one-way sink out of the ledger, so "a conservation
    /// guard built on soil would measure that leak instead of the thing it
    /// is named for". Three changes closed it -- `water_equivalents` counts
    /// held water, the rain soak is charged rather than free, and damp soil
    /// at an open surface evaporates and credits the bank -- and this is
    /// the test that says so.
    ///
    /// **Confirmed red against each of the three separately**: without the
    /// soil arm in the census the ledger falls as ground wets; without the
    /// soak charge it rises; without the drying path it falls and does not
    /// come back.
    ///
    /// Deliberately no plants. `plant.rs`'s `transpire` destroys soil
    /// moisture and credits nothing and root uptake moves it into a plant
    /// the ledger cannot see, so a scene with a tree in it would measure
    /// those instead -- they are recorded in
    /// `Reports/weather-handoff.md` rather than fixed.
    #[test]
    fn the_worlds_water_is_flat_over_soil_too() {
        for parallel_driver in [true, false] {
            let seed = 12345;
            let start = first_frame_with(seed, 0, |w| w.kind == Precipitation::Rain && w.intensity > 0.5)
                .expect("this seed has a wet epoch in it");
            let mut w = open_shelf_world(seed, start, Some(0));
            // Settle first, so the opening reading is not taken mid-fall
            // while worldgen's own water is still arriving somewhere.
            advance(&mut w, 200, parallel_driver);

            let opening = water_equivalents(&w) + w.atmospheric_bank;
            let (mut wet_frames, mut dry_frames) = (0, 0);
            for _ in 0..40 {
                advance(&mut w, 150, parallel_driver);
                if at(w.seed, w.frame).kind == Precipitation::Rain && at(w.seed, w.frame).intensity > 0.1 {
                    wet_frames += 1;
                } else {
                    dry_frames += 1;
                }
            }
            let closing = water_equivalents(&w) + w.atmospheric_bank;

            assert!(
                wet_frames > 3 && dry_frames > 3,
                "the window has to contain both weathers or it proves nothing (wet {wet_frames}, dry {dry_frames})"
            );
            let drift = (closing - opening) / opening.max(1.0);
            assert!(
                drift.abs() < 0.001,
                "water over soil drifted {:.4}% across a storm and a dry spell: {opening:.1} -> {closing:.1} (parallel: {parallel_driver})",
                drift * 100.0
            );
        }
    }

    #[test]
    fn the_worlds_water_is_flat_across_storm_and_drought_cycles() {
        // **The conservation law, end to end**: standing water plus banked
        // water is the quantity that must not move, across a window that
        // contains both a front (the sky spending) and a dry spell (the
        // pond paying it back). Either half alone proves nothing -- a world
        // that only rains looks conserved if the debit is right and the
        // credit is dead, and a world that only dries looks conserved the
        // other way round.
        //
        // Both drivers, per `CLAUDE.md`: `App::update` runs the parallel one,
        // and `weather::step` and the scheduler are called by both.
        for parallel_driver in [false, true] {
            let seed = 12345;
            // A window that starts dry, meets a front, and comes out the
            // other side. `first_frame_with` finds the front; starting a
            // little before it is what buys the leading dry spell.
            let storm = first_frame_with(seed, 0, |x| x.is_precipitating()).expect("seed 12345 has a front");
            let start = storm.saturating_sub(1_500);
            let mut w = walled_basin_world(seed, start);

            let opening = water_equivalents(&w) + w.atmospheric_bank;
            let (mut wet_frames, mut dry_frames) = (0u32, 0u32);
            for _ in 0..6_000 {
                if parallel_driver {
                    parallel::step(&mut w);
                } else {
                    crate::sim::update::step(&mut w);
                }
                w.step_active_sites();
                w.step_fields();
                if at(seed, w.frame).is_precipitating() {
                    wet_frames += 1;
                } else {
                    dry_frames += 1;
                }
            }
            let closing = water_equivalents(&w) + w.atmospheric_bank;
            let drift = (closing - opening) / opening;
            println!(
                "conservation (parallel: {parallel_driver}): {opening:.2} -> {closing:.2} ({:+.4}%); {wet_frames} wet frames, {dry_frames} dry; bank {:.1}, standing {:.1}",
                drift * 100.0,
                w.atmospheric_bank,
                water_equivalents(&w)
            );
            assert!(
                wet_frames > 500 && dry_frames > 500,
                "the window did not span both a storm and a dry spell ({wet_frames} wet, {dry_frames} dry); this is not measuring a cycle"
            );
            // A tenth of a percent. The slack is not tolerance for a leak --
            // both halves are exact arithmetic on `LIQUID_FULL` units -- it
            // is for the freeze/melt round trip, which gives a cell that
            // froze at `freeze_min_fill` back full and is a structural
            // ceiling `a_thaw_does_not_manufacture_water` documents at up to
            // 10% on a world that is *all* ice. Bounded on both sides:
            // manufacture and loss are the same bug wearing different signs.
            assert!(
                drift.abs() < 0.001,
                "the world's water moved by {:+.3}% across a storm and a drought (parallel: {parallel_driver})",
                drift * 100.0
            );
        }
    }
}
