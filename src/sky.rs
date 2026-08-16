//! What empty space above the ground looks like, through a day.
//!
//! Purely a rendering concern: nothing here feeds back into the simulation.
//! The *time* comes from `sim::field::sun_elevation`, which is the same
//! cosine driving the light channel — deliberately shared rather than
//! reimplemented, because a sky painting dawn while the light channel still
//! says midnight is worse than a black sky. There is one definition of what
//! time it is and this reads it.
//!
//! # Why this is a gradient and not a colour
//!
//! A flat fill reads as a backdrop; the eye needs the vertical falloff to
//! read it as air. Two colours per moment — zenith and horizon — interpolated
//! down the screen is the cheapest thing that does it, and it costs one lerp
//! per empty pixel with no lookups.
//!
//! # Why it is quantised
//!
//! A sky that changes every frame changes every pixel every frame, which
//! defeats the dirty-rect render skip — measured at ~10 ms mean on a settled
//! world, most of a 60 Hz budget spent redrawing a world that is not moving
//! (`render.rs`'s own note on the animated grain modes). So the renderer
//! redraws only on the frames the sky's *quantised* colour actually changes,
//! and skips the rest, where a redraw would produce a pixel-identical image.
//! [`Sky::key`] is that quantisation. The cost is therefore paid in
//! proportion to how fast the sky is really changing: nothing at all through
//! the middle of the night, most often through sunrise and sunset, which is
//! the one time of day it is worth paying for.

use crate::sim::field;

/// Colour steps the sky is quantised to before deciding whether a redraw is
/// needed.
///
/// Chosen by measurement, not by taste. A full redraw costs **9.29 ms** on a
/// 512x320 world, so what matters is how many frames of a day force one:
///
/// | quantum | frames per 3600 | added cost, averaged |
/// |---|---|---|
/// | 4 | 383 (10.6%) | 0.97 ms |
/// | 6 | 278 (7.7%) | 0.70 ms |
/// | 8 | 217 (6.0%) | 0.56 ms |
/// | 12 | 150 (4.2%) | 0.38 ms |
///
/// Eight, because a sunset rendered at 40-frame intervals shows no banding at
/// that step — checked by eye, which is the only way that question can be
/// settled — and going coarser buys progressively less. Note the average is
/// not the whole story: on the frames it does fire, rendering costs the full
/// 9.29 ms, and those frames cluster around sunrise and sunset. That is the
/// same cost the engine already pays whenever particles are on screen.
const SKY_QUANTUM: i32 = 8;

/// How far either side of the horizon counts as twilight, in units of
/// `sun_elevation`. Roughly a fifth of the cycle spent in transition, which
/// is what makes sunrise and sunset events rather than instants.
const TWILIGHT_WIDTH: f32 = 0.32;

// Keyframes. Deep night is not black: a black sky reads as *absence* — as
// the void outside the world, which has its own colour and should stay
// distinguishable — and the light channel already refuses to go fully dark
// for the same reason (`NIGHT_LIGHT_FLOOR`).
const NIGHT_ZENITH: [f32; 3] = [7.0, 9.0, 22.0];
const NIGHT_HORIZON: [f32; 3] = [18.0, 20.0, 38.0];
const DAY_ZENITH: [f32; 3] = [56.0, 104.0, 174.0];
const DAY_HORIZON: [f32; 3] = [146.0, 184.0, 218.0];
/// Twilight lifts the zenith toward violet, which is what makes the warm
/// horizon read as *lit from below* rather than as a coloured band.
const TWILIGHT_ZENITH: [f32; 3] = [44.0, 40.0, 86.0];
/// Sunset runs redder, sunrise pinker and paler. Not a real optical
/// difference at this fidelity — a game convention, and a cheap way for the
/// player to tell which way the day is going without a clock.
const SUNSET_HORIZON: [f32; 3] = [222.0, 112.0, 58.0];
const SUNRISE_HORIZON: [f32; 3] = [226.0, 142.0, 138.0];

/// The sky at one moment: two colours and the rows they span.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sky {
    zenith: [u8; 3],
    horizon: [u8; 3],
    /// Row the zenith colour applies at, and the row the horizon colour
    /// does. The gradient runs between them and clamps outside.
    top_y: i32,
    horizon_y: i32,
}

/// Where the horizon colour lands, as a fraction of world height.
///
/// **Not the bottom of the world**, which was the first version and wasted
/// the entire feature: ground sits somewhere around a third of the way down,
/// so a gradient reaching its horizon colour at the world floor renders that
/// colour only where it is buried under rock. Sunset happened every cycle and
/// could not be seen. Placed a little below where terrain surfaces usually
/// sit, so the warm band reads as being *behind and below* the skyline, and
/// clamped past that — sky drawn inside a cave has nowhere sensible to be on
/// this ramp and simply takes the horizon colour.
const HORIZON_FRACTION: f32 = 0.45;

fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn to_u8(c: [f32; 3]) -> [u8; 3] {
    [c[0].clamp(0.0, 255.0) as u8, c[1].clamp(0.0, 255.0) as u8, c[2].clamp(0.0, 255.0) as u8]
}

impl Sky {
    /// The sky over a world of `top_y..=bottom_y` rows at `frame`.
    pub fn at(frame: u64, top_y: i32, bottom_y: i32) -> Self {
        let sun = field::sun_elevation(frame);

        // Day fades in a little before the sun clears the horizon and is
        // fully up well before noon: daylight saturates fast, and a linear
        // ramp all the way to noon would leave mid-morning looking like
        // dawn.
        let dayness = smoothstep(-0.05, 0.40, sun);
        let mut zenith = lerp(NIGHT_ZENITH, DAY_ZENITH, dayness);
        let mut horizon = lerp(NIGHT_HORIZON, DAY_HORIZON, dayness);

        // Twilight is layered *over* that rather than being a third
        // keyframe in the same interpolation, because it is not a stage
        // between night and day — it is a thing that happens to both ends of
        // the day and has to survive being blended with either.
        let twilight = (1.0 - (sun.abs() / TWILIGHT_WIDTH)).clamp(0.0, 1.0);
        if twilight > 0.0 {
            let warm = if field::sun_rising(frame) { SUNRISE_HORIZON } else { SUNSET_HORIZON };
            // The horizon takes most of the colour and the zenith a little,
            // which is the whole shape of a real sunset.
            horizon = lerp(horizon, warm, twilight * 0.9);
            zenith = lerp(zenith, TWILIGHT_ZENITH, twilight * 0.55);
        }

        let horizon_y = top_y + (((bottom_y - top_y) as f32) * HORIZON_FRACTION) as i32;
        Self { zenith: to_u8(zenith), horizon: to_u8(horizon), top_y, horizon_y }
    }

    /// Sky colour at a world row.
    pub fn colour_at(&self, y: i32) -> [u8; 4] {
        let span = (self.horizon_y - self.top_y).max(1) as f32;
        let t = ((y - self.top_y) as f32 / span).clamp(0.0, 1.0);
        // Eased so the horizon colour concentrates in the lower sky rather
        // than washing all the way up. Gentler than a square, which put so
        // little colour in the visible band that a sunset read as a grey
        // evening.
        let t = t.powf(1.4);
        let c = lerp(
            [self.zenith[0] as f32, self.zenith[1] as f32, self.zenith[2] as f32],
            [self.horizon[0] as f32, self.horizon[1] as f32, self.horizon[2] as f32],
            t,
        );
        let c = to_u8(c);
        [c[0], c[1], c[2], 255]
    }

    /// A coarse identity for this sky, for deciding whether anything on
    /// screen would actually differ.
    ///
    /// Two skies sharing a key render identically to within
    /// [`SKY_QUANTUM`], so a frame whose key is unchanged can safely skip
    /// the redraw. This is the entire reason a day/night sky does not cost
    /// the dirty-rect skip.
    pub fn key(&self) -> [i32; 6] {
        let q = |v: u8| v as i32 / SKY_QUANTUM;
        [
            q(self.zenith[0]),
            q(self.zenith[1]),
            q(self.zenith[2]),
            q(self.horizon[0]),
            q(self.horizon[1]),
            q(self.horizon[2]),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frames at the four cardinal points of the cycle, derived from
    /// `sun_elevation` rather than hardcoded so a change to the period
    /// cannot silently point these at the wrong time of day.
    fn noon() -> u64 {
        0
    }
    fn midnight() -> u64 {
        (0..6000).find(|&f| field::sun_elevation(f) < -0.999).expect("a midnight exists")
    }
    fn sunset() -> u64 {
        (0..6000).find(|&f| field::sun_elevation(f) <= 0.0).expect("a sunset exists")
    }
    fn sunrise() -> u64 {
        (0..6000).find(|&f| field::sun_elevation(f) >= 0.0 && field::sun_rising(f)).expect("a sunrise exists")
    }

    fn sky(frame: u64) -> Sky {
        Sky::at(frame, 0, 320)
    }

    fn brightness(c: [u8; 4]) -> u32 {
        c[0] as u32 + c[1] as u32 + c[2] as u32
    }

    #[test]
    fn day_is_brighter_than_night() {
        assert!(brightness(sky(noon()).colour_at(10)) > brightness(sky(midnight()).colour_at(10)) * 3);
    }

    #[test]
    fn night_is_never_black() {
        // A black sky is indistinguishable from the void outside the world,
        // and reads as absence rather than as darkness.
        assert!(brightness(sky(midnight()).colour_at(10)) > 20);
    }

    #[test]
    fn the_horizon_is_warm_at_both_ends_of_the_day() {
        // The actual claim of this feature: at sunrise and sunset the low sky
        // goes warm — red channel clearly ahead of blue — and at noon it does
        // not. Checked at the horizon row, which is where it should happen.
        for frame in [sunrise(), sunset()] {
            let c = sky(frame).colour_at(320);
            assert!(
                c[0] as i32 > c[2] as i32 + 40,
                "twilight horizon is not warm at frame {frame}: {c:?}"
            );
        }
        let day = sky(noon()).colour_at(320);
        assert!(day[2] > day[0], "the noon horizon should be blue, got {day:?}");
    }

    #[test]
    fn sunrise_and_sunset_are_told_apart() {
        assert_ne!(sky(sunrise()).colour_at(320), sky(sunset()).colour_at(320));
    }

    #[test]
    fn the_sky_is_darker_overhead_than_at_the_horizon() {
        // The gradient's direction, at the time of day it is most visible.
        let s = sky(noon());
        assert!(brightness(s.colour_at(0)) < brightness(s.colour_at(320)));
    }

    #[test]
    fn the_key_holds_still_through_most_of_the_day() {
        // The performance claim, as a measurement rather than an assertion of
        // intent: if this number creeps up, the sky has started costing the
        // dirty-rect skip and someone should know.
        let mut changes = 0;
        let mut last = sky(0).key();
        for frame in 1..3600u64 {
            let key = sky(frame).key();
            if key != last {
                changes += 1;
                last = key;
            }
        }
        // Measured at 217 with `SKY_QUANTUM = 8`; bar set above that with
        // headroom rather than on it.
        assert!(changes < 300, "the sky redraws on {changes} of 3600 frames");
    }

    #[test]
    fn the_key_does_change_across_a_sunrise() {
        // The other half: a quantisation coarse enough to never redraw would
        // pass the test above and produce a sky that never moves.
        let before = sky(sunrise().saturating_sub(200)).key();
        let after = sky(sunrise() + 200).key();
        assert_ne!(before, after);
    }
}
