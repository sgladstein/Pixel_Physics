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

use super::field::DAY_NIGHT_PERIOD_FRAMES;
use super::rng;

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

#[cfg(test)]
mod tests {
    use super::*;

    /// The first frame at or after `from` whose weather satisfies `f`, within
    /// a generous budget. Most frames are clear, so nearly every test here
    /// needs to go and find its case rather than assert about frame 0.
    fn first_frame_with(seed: u64, from: u64, f: impl Fn(Weather) -> bool) -> Option<u64> {
        // Stepped rather than exhaustive: the channels are smooth over
        // epochs, so anything that lasts less than a few hundred frames is
        // not a weather event in the first place.
        (from..from + WEATHER_EPOCH_FRAMES * 64).step_by(60).find(|&frame| f(at(seed, frame)))
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
