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
/// redraw, and those frames cluster around sunrise and sunset. That is the
/// same cost the engine already pays whenever particles are on screen.
///
/// Re-measured once stars and the moon landed: a full redraw is **10.49 ms**
/// (the per-pixel star and moon tests add about 13%), and stars fading in
/// pushes the key changes to **240 of 3600 frames (6.7%), 0.70 ms averaged**.
/// The moon is *not* in this number and must not be — it moves on a dirty
/// rectangle instead, which is what keeps a disc crossing the sky from
/// repainting the screen several hundred times a night.
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

/// How many steps the per-cell lighting is quantised to.
///
/// Same argument as [`SKY_QUANTUM`] and the same reason: material colours
/// that track a continuously changing light field would change every frame,
/// and repainting for a change nobody can see costs the dirty-rect skip.
/// Sixteen steps across the whole range is under two levels of an 8-bit
/// channel per step at the brightness end, which is not visible on textured
/// rock.
pub const LIGHT_LEVELS: u8 = 16;

/// How dark a fully unlit cell is drawn, as a fraction of its own colour.
///
/// **Not zero, and this is a game decision rather than a physical one.** With
/// a true zero the underground goes black and the game becomes "you cannot
/// see without a light source" — which is a real design, and a much bigger
/// one than drawing a sky. At this floor the cross-section stays readable:
/// night is unmistakably night, a cave is clearly darker than open ground,
/// and the strata are still legible everywhere. Raise it toward 1.0 to flatten
/// the effect, lower it toward 0.0 to make light sources matter.
pub const UNLIT_FLOOR: f32 = 0.42;

/// How lit the world is at `frame`, quantised to `0..=LIGHT_LEVELS`.
pub fn daylight_level(frame: u64) -> u8 {
    (field::daylight_fraction(frame) * LIGHT_LEVELS as f32).round().min(LIGHT_LEVELS as f32) as u8
}

/// Apply a quantised light level to a material colour.
///
/// Two effects, because darkening alone reads as a screen-wide fade rather
/// than as nightfall: the colour is scaled toward black *and* tinted toward
/// the ambient the sky is casting. Unlit rock under a sunset picks up the
/// orange; under a night sky it goes cold. That tint is what ties the ground
/// to the sky rather than leaving them two unrelated layers.
pub fn apply_light(rgb: [u8; 3], level: u8, ambient: [u8; 3]) -> [u8; 3] {
    let t = level as f32 / LIGHT_LEVELS as f32;
    let scale = UNLIT_FLOOR + (1.0 - UNLIT_FLOOR) * t;
    // Strongest where the light is weakest: a fully lit surface shows its own
    // colour, an unlit one is showing only what the sky is giving it.
    let tint = (1.0 - t) * AMBIENT_TINT;
    let mut out = [0u8; 3];
    for i in 0..3 {
        let own = rgb[i] as f32 * scale;
        out[i] = (own + (ambient[i] as f32 - own) * tint).clamp(0.0, 255.0) as u8;
    }
    out
}

/// How far an unlit surface is pulled toward the ambient sky colour. Small:
/// past about a fifth the world starts to look like coloured fog.
const AMBIENT_TINT: f32 = 0.18;

/// Phase at which the moon rises and sets. Slightly inside the sun's own
/// half of the cycle, so the two are never both prominent — the moon appears
/// once dusk is well along and is gone by the time dawn has colour in it.
const MOON_RISE: f32 = 0.28;
const MOON_SET: f32 = 0.72;
/// How much of the sky's height the moon's arc rises through.
const MOON_ARC_HEIGHT: f32 = 0.72;
/// Radius of the moon's disc, in cells.
const MOON_RADIUS: i32 = 9;
/// Colour of the lit disc, and of the faint halo around it.
const MOON_COLOUR: [f32; 3] = [236.0, 238.0, 226.0];
const MOON_GLOW: [f32; 3] = [150.0, 160.0, 190.0];
/// How far the halo reaches beyond the disc.
const MOON_GLOW_RADIUS: i32 = 22;
/// Fraction of sky cells that hold a star. Sparse on purpose: a dense field
/// reads as noise rather than as sky, and at this resolution every star is a
/// single conspicuous pixel.
const STAR_DENSITY: f32 = 0.006;
/// Stars are very slightly warm rather than pure white, which stops them
/// reading as dead pixels against a blue sky.
const STAR_COLOUR: [f32; 3] = [240.0, 240.0, 226.0];

/// A stable `0..1` value for a position, for scattering stars.
fn hash01(x: i32, y: i32, salt: u64) -> f32 {
    let mut z = salt
        ^ (x as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (y as i64 as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

/// The sky at one moment: two colours and the rows they span.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sky {
    zenith: [u8; 3],
    horizon: [u8; 3],
    /// How strongly stars show, `0` by day. Quantised, and part of [`key`],
    /// so stars fading in is a thing that triggers a repaint.
    ///
    /// [`key`]: Sky::key
    star_alpha: u8,
    /// Centre of the moon, when it is up. Deliberately **not** in the key:
    /// a moon that crosses the sky would otherwise repaint the whole screen
    /// every few frames all night. It moves a few hundred pixels' worth of
    /// dirty rectangle instead — see `Sky::moon_rect`.
    moon: Option<(i32, i32)>,
    /// World x range, for placing the moon.
    min_x: i32,
    max_x: i32,
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
    /// The sky over a world at `frame`.
    pub fn at(frame: u64, min_x: i32, max_x: i32, top_y: i32, bottom_y: i32) -> Self {
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

        // Stars come out as the sun goes down and are gone well before it is
        // properly up, which is roughly how it looks: the last star goes
        // long before sunrise finishes. Quantised, because star brightness
        // changing is a reason to repaint and a continuous fade would be a
        // reason to repaint every frame.
        let starriness = smoothstep(0.10, -0.25, sun);
        let star_alpha = ((starriness * 255.0) as i32 / 16 * 16).clamp(0, 255) as u8;

        // The moon is up while the sun is not, and crosses the sky over the
        // night. Placed on an arc rather than a straight line so it reads as
        // travelling over rather than sliding across.
        let phase = (frame % field::DAY_NIGHT_PERIOD_FRAMES) as f32
            / field::DAY_NIGHT_PERIOD_FRAMES as f32;
        let moon = if (MOON_RISE..MOON_SET).contains(&phase) {
            let t = (phase - MOON_RISE) / (MOON_SET - MOON_RISE);
            let span = (max_x - min_x).max(1) as f32;
            let mx = min_x + (t * span) as i32;
            // Highest at the middle of the night; skimming the horizon at
            // both ends.
            let arc = (t * std::f32::consts::PI).sin();
            let sky_span = (horizon_y - top_y).max(1) as f32;
            let my = horizon_y - (arc * sky_span * MOON_ARC_HEIGHT) as i32;
            Some((mx, my))
        } else {
            None
        };

        Self { zenith: to_u8(zenith), horizon: to_u8(horizon), star_alpha, moon, min_x, max_x, top_y, horizon_y }
    }

    /// Sky colour at a world position, stars and moon included.
    pub fn colour_at(&self, x: i32, y: i32) -> [u8; 4] {
        let base = self.gradient_at(y);
        // Moon first: it sits in front of the stars, so a star must not show
        // through the disc.
        if let Some((mx, my)) = self.moon {
            let (dx, dy) = ((x - mx) as f32, (y - my) as f32);
            let d2 = dx * dx + dy * dy;
            let r = MOON_RADIUS as f32;
            if d2 <= r * r {
                // A soft edge, so a nine-cell disc does not read as a
                // staircase at this resolution.
                let edge = smoothstep(r * r, (r - 1.5) * (r - 1.5), d2);
                let c = lerp([base[0] as f32, base[1] as f32, base[2] as f32], MOON_COLOUR, edge);
                let c = to_u8(c);
                return [c[0], c[1], c[2], 255];
            }
            let g = MOON_GLOW_RADIUS as f32;
            if d2 <= g * g {
                // A halo, falling off fast. Without it the moon is a sticker;
                // with it, it lights the sky it is in.
                let t = (1.0 - d2.sqrt() / g).clamp(0.0, 1.0).powi(3) * 0.55;
                let c = lerp([base[0] as f32, base[1] as f32, base[2] as f32], MOON_GLOW, t);
                let c = to_u8(c);
                return [c[0], c[1], c[2], 255];
            }
        }
        if let Some(star) = self.star_at(x, y) {
            return star;
        }
        base
    }

    /// Whether a star sits at this position, and how bright.
    ///
    /// Position-hashed rather than a stored list: a star field is exactly the
    /// kind of thing that wants to be a pure function of where you are
    /// looking, so it costs no memory, needs no generation step, and is
    /// identical every time the same sky is drawn.
    fn star_at(&self, x: i32, y: i32) -> Option<[u8; 4]> {
        // "Is there a *visible* star here", not "is this a star position" --
        // the daylight check belongs with the rest of the answer, or a caller
        // (or a test) can ask this and be told about stars nobody can see.
        if self.star_alpha == 0 {
            return None;
        }
        // Stars thin out toward the horizon, where a real sky has haze and
        // this one has its brightest gradient -- a star on a pale horizon
        // would read as a stuck pixel.
        let span = (self.horizon_y - self.top_y).max(1) as f32;
        let height = 1.0 - ((y - self.top_y) as f32 / span).clamp(0.0, 1.0);
        let roll = hash01(x, y, 0x5741);
        if roll >= STAR_DENSITY * height {
            return None;
        }
        // Varied brightness, so the field has depth instead of reading as
        // one scattered stencil.
        let mag = 0.45 + hash01(x, y, 0x9E17) * 0.55;
        let a = (self.star_alpha as f32 / 255.0) * mag;
        let base = self.gradient_at(y);
        let c = lerp([base[0] as f32, base[1] as f32, base[2] as f32], STAR_COLOUR, a);
        let c = to_u8(c);
        Some([c[0], c[1], c[2], 255])
    }

    /// The plain vertical gradient, with nothing in front of it.
    fn gradient_at(&self, y: i32) -> [u8; 4] {
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

    /// The area the moon and its halo cover, if it is up.
    ///
    /// The renderer repaints this rectangle — and the one from last frame —
    /// instead of the screen. A moon crossing the sky moves about a cell
    /// every three frames, so treating it like a colour change would repaint
    /// everything several hundred times a night for a disc a few hundred
    /// pixels across. This is the same trick chunk bodies use, and for the
    /// same reason.
    /// Mute the sky under cloud. Applied after construction rather than as a
    /// constructor argument so the day/night maths above stays one thing and
    /// weather stays another — the sunset still happens behind the cloud, it
    /// is just not very visible, which is what happens.
    ///
    /// Desaturates toward the sky's *own* luminance rather than toward a
    /// fixed grey: a fixed grey would light up the night sky under a storm,
    /// which is backwards. This darkens whatever is there and drains its
    /// colour, so an overcast noon is flat white-grey and an overcast
    /// midnight is flat black — both correct, from one line.
    pub fn muted(mut self, overcast: f32) -> Self {
        if overcast <= 0.0 {
            return self;
        }
        let o = overcast.clamp(0.0, 1.0);
        let flatten = |c: [u8; 3]| {
            let luma = (c[0] as f32 * 0.30 + c[1] as f32 * 0.59 + c[2] as f32 * 0.11) * 0.78;
            let mut out = [0u8; 3];
            for i in 0..3 {
                out[i] = (c[i] as f32 * (1.0 - o) + luma * o).clamp(0.0, 255.0) as u8;
            }
            out
        };
        self.zenith = flatten(self.zenith);
        self.horizon = flatten(self.horizon);
        // Cloud hides stars and the moon. Without this a snowstorm is drawn
        // over a perfectly clear starfield, which reads as two unrelated
        // effects layered rather than as one sky.
        self.star_alpha = (self.star_alpha as f32 * (1.0 - o)) as u8;
        if o > 0.55 {
            self.moon = None;
        }
        self
    }

    pub fn moon_rect(&self) -> Option<(i32, i32, i32, i32)> {
        self.moon.map(|(mx, my)| {
            let r = MOON_GLOW_RADIUS;
            (mx - r, my - r, mx + r, my + r)
        })
    }

    /// The ambient colour this sky casts on unlit surfaces — its horizon,
    /// which is the part of it nearest the ground.
    pub fn ambient(&self) -> [u8; 3] {
        self.horizon
    }

    /// A coarse identity for this sky, for deciding whether anything on
    /// screen would actually differ.
    ///
    /// Two skies sharing a key render identically to within
    /// [`SKY_QUANTUM`], so a frame whose key is unchanged can safely skip
    /// the redraw. This is the entire reason a day/night sky does not cost
    /// the dirty-rect skip.
    pub fn key(&self) -> [i32; 7] {
        let q = |v: u8| v as i32 / SKY_QUANTUM;
        [
            q(self.zenith[0]),
            q(self.zenith[1]),
            q(self.zenith[2]),
            q(self.horizon[0]),
            q(self.horizon[1]),
            q(self.horizon[2]),
            // Stars fading in changes pixels the gradient alone would not,
            // so it belongs in the repaint trigger. Already quantised.
            self.star_alpha as i32,
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
        Sky::at(frame, 0, 512, 0, 320)
    }

    fn brightness(c: [u8; 4]) -> u32 {
        c[0] as u32 + c[1] as u32 + c[2] as u32
    }

    #[test]
    fn day_is_brighter_than_night() {
        assert!(brightness(sky(noon()).colour_at(4, 10)) > brightness(sky(midnight()).colour_at(4, 10)) * 3);
    }

    #[test]
    fn night_is_never_black() {
        // A black sky is indistinguishable from the void outside the world,
        // and reads as absence rather than as darkness.
        assert!(brightness(sky(midnight()).colour_at(4, 10)) > 20);
    }

    #[test]
    fn the_horizon_is_warm_at_both_ends_of_the_day() {
        // The actual claim of this feature: at sunrise and sunset the low sky
        // goes warm — red channel clearly ahead of blue — and at noon it does
        // not. Checked at the horizon row, which is where it should happen.
        for frame in [sunrise(), sunset()] {
            let c = sky(frame).colour_at(4, 320);
            assert!(
                c[0] as i32 > c[2] as i32 + 40,
                "twilight horizon is not warm at frame {frame}: {c:?}"
            );
        }
        let day = sky(noon()).colour_at(4, 320);
        assert!(day[2] > day[0], "the noon horizon should be blue, got {day:?}");
    }

    #[test]
    fn sunrise_and_sunset_are_told_apart() {
        assert_ne!(sky(sunrise()).colour_at(4, 320), sky(sunset()).colour_at(4, 320));
    }

    #[test]
    fn the_sky_is_darker_overhead_than_at_the_horizon() {
        // The gradient's direction, at the time of day it is most visible.
        let s = sky(noon());
        assert!(brightness(s.colour_at(4, 0)) < brightness(s.colour_at(4, 320)));
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
    fn stars_come_out_at_night_and_not_by_day() {
        // Counted rather than eyeballed: a star field is exactly the kind of
        // thing that can look present in a render and be one stuck pixel.
        let count = |frame: u64| {
            let s = sky(frame);
            (0..512).flat_map(|x| (0..60).map(move |y| (x, y))).filter(|&(x, y)| s.star_at(x, y).is_some()).count()
        };
        let night = count(midnight());
        assert!(night > 30, "only {night} stars at midnight");
        assert_eq!(count(noon()), 0, "stars are out at noon");
    }

    #[test]
    fn the_moon_is_up_at_night_and_crosses_the_sky() {
        assert!(sky(noon()).moon_rect().is_none(), "the moon is up at noon");
        let early = Sky::at(field::DAY_NIGHT_PERIOD_FRAMES * 30 / 100, 0, 512, 0, 320);
        let late = Sky::at(field::DAY_NIGHT_PERIOD_FRAMES * 70 / 100, 0, 512, 0, 320);
        let (ex, _) = early.moon.expect("moon up early in the night");
        let (lx, _) = late.moon.expect("moon up late in the night");
        assert!(lx > ex, "the moon did not travel: {ex} then {lx}");
        // And it arcs: higher in the middle of the night than at either end.
        let mid = Sky::at(field::DAY_NIGHT_PERIOD_FRAMES / 2, 0, 512, 0, 320);
        let (_, my) = mid.moon.expect("moon up at midnight");
        let (_, ey) = early.moon.unwrap();
        assert!(my < ey, "the moon should be higher at midnight: {my} vs {ey}");
    }

    #[test]
    fn the_moon_does_not_force_a_repaint_as_it_moves() {
        // The whole reason the moon is a dirty rectangle rather than part of
        // the key. If its position ever leaks into the key, a night costs
        // several hundred full-screen repaints.
        let a = Sky::at(field::DAY_NIGHT_PERIOD_FRAMES * 40 / 100, 0, 512, 0, 320);
        let b = Sky::at(field::DAY_NIGHT_PERIOD_FRAMES * 45 / 100, 0, 512, 0, 320);
        assert_ne!(a.moon, b.moon, "test setup: the moon should have moved");
        assert_eq!(a.key(), b.key(), "the moon's position reached the repaint key");
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

// ---------------------------------------------------------------------------
// Precipitation
// ---------------------------------------------------------------------------

/// One streak or flake, in **world** coordinates.
///
/// World coordinates, not screen, and that is the whole reason this is
/// generated here rather than straight into the framebuffer: the camera
/// follows the gnome now, and a pattern hashed against screen position slides
/// bodily across the world every time he walks. Rain that swims sideways when
/// you move reads as a rendering fault immediately.
pub struct Drop {
    pub from: (f32, f32),
    pub to: (f32, f32),
    /// `0.0..=1.0`. Varies per drop and per layer — uniform opacity is the
    /// single biggest tell that precipitation is a texture rather than
    /// weather.
    pub alpha: f32,
    pub colour: [f32; 3],
}

/// How the falling stuff is drawn. Distinct from `weather::Precipitation`,
/// which is what the *simulation* believes; this is the presentation of it.
#[derive(Clone, Copy, PartialEq)]
pub enum Fall {
    Rain,
    Snow,
}

/// Depth layers. Near drops are faster, longer, brighter and sparser; far
/// ones are slow, short and dim.
///
/// Parallax is doing the heavy lifting on how this reads. A single layer of
/// identical streaks is a screen-door effect no matter how it is tuned,
/// because every drop moves at exactly the same rate and the eye reads the
/// whole field as one flat sheet sliding. Three rates and the air acquires
/// depth. `(band width, fall speed, streak length, alpha, brightness)`.
const RAIN_LAYERS: [(f32, f32, f32, f32, f32); 3] =
    [(3.4, 3.1, 13.0, 0.52, 1.00), (2.4, 2.1, 8.0, 0.34, 0.86), (1.7, 1.4, 5.0, 0.21, 0.70)];
const SNOW_LAYERS: [(f32, f32, f32, f32, f32); 3] =
    [(7.0, 0.55, 2.6, 0.95, 1.00), (5.0, 0.36, 1.8, 0.70, 0.92), (3.6, 0.24, 1.2, 0.46, 0.80)];

/// Drops emitted per band per fall period.
///
/// With one, a band contributes a visible drop only when its single drop
/// happens to be inside the viewport, so the density on screen is the band
/// count scaled by `viewport height / FALL_PERIOD` -- which at the first
/// values meant roughly a hundred streaks for a downpour of intensity 0.87.
/// It looked like drizzle. Density is set here and by the band widths
/// together, and neither is meaningful without the other.
const DROPS_PER_BAND: i32 = 2;

/// How far a drop falls before its pattern repeats, in world cells.
///
/// A little taller than a viewport, so a drop crosses the whole screen and
/// wraps well off the top rather than blinking out mid-air -- but only a
/// little, because every cell of period beyond the screen is a drop that
/// exists and is not visible, and that ratio *is* the on-screen density.
/// Anchored in world space so the wrap point does not move when the camera
/// does.
const FALL_PERIOD: f32 = 420.0;

/// Sideways travel per cell of fall, at full wind.
const WIND_SLANT: f32 = 0.75;

const RAIN_COLOUR: [f32; 3] = [186.0, 208.0, 232.0];
const SNOW_COLOUR: [f32; 3] = [244.0, 246.0, 252.0];

/// Every drop visible in a world-space rectangle this frame.
///
/// Pure and stateless, like everything else about weather here: there is no
/// drop list to advance, spawn into or clean up, so nothing to desynchronise
/// and nothing whose cost grows with how long it has been raining. Two runs
/// of a seed draw the same rain on the same frame, and a paused world's rain
/// is frozen rather than accumulating.
///
/// Deliberately **not** `ParticleSystem`: a non-empty particle system forces
/// a full-screen redraw every frame for as long as it is non-empty, measured
/// at ~10 ms, and that cost would be permanent rather than confined to the
/// storm. Rain still forces full redraws while it falls — it must, it moves
/// everywhere — but only while it falls.
#[allow(clippy::too_many_arguments)]
pub fn drops(
    frame: u64,
    fall: Fall,
    intensity: f32,
    wind: f32,
    (x0, y0): (i32, i32),
    (x1, y1): (i32, i32),
) -> Vec<Drop> {
    let mut out = Vec::new();
    if intensity <= 0.0 {
        return out;
    }
    let layers: &[(f32, f32, f32, f32, f32); 3] = match fall {
        Fall::Rain => &RAIN_LAYERS,
        Fall::Snow => &SNOW_LAYERS,
    };
    let colour = match fall {
        Fall::Rain => RAIN_COLOUR,
        Fall::Snow => SNOW_COLOUR,
    };
    let t = frame as f32;

    for (layer, &(band, speed, length, alpha, bright)) in layers.iter().enumerate() {
        let salt = 0x5041_5454 + layer as u64;
        // Bands are indexed in world space, so which drops exist is a
        // property of *where you are*, not of where the camera is. Widened
        // by the slant so a drop that starts off-screen and blows into view
        // is generated rather than popping into existence at the edge.
        let slant = wind * WIND_SLANT;
        let margin = (slant.abs() * FALL_PERIOD).min(64.0) + length + 2.0;
        let g0 = ((x0 as f32 - margin) / band).floor() as i32;
        let g1 = ((x1 as f32 + margin) / band).ceil() as i32;
        for g in g0..=g1 {
            for d in 0..DROPS_PER_BAND {
            // Intensity thins the field rather than dimming it: light rain is
            // fewer drops, not fainter ones. Fading them instead produces a
            // ghost of a downpour, which reads as fog.
            let live = hash01(g, layer as i32 * 8 + d, salt ^ 0x11);
            if live > intensity.clamp(0.0, 1.0) {
                continue;
            }
            let phase = hash01(g, layer as i32 * 8 + d, salt);
            let jitter = hash01(g, layer as i32 * 8 + d, salt ^ 0x22);
            // Per-drop speed spread, so a layer is a cloud of drops rather
            // than a rigid comb descending in lockstep.
            let v = speed * (0.75 + jitter * 0.5);
            let base = (t * v + phase * FALL_PERIOD) % FALL_PERIOD;
            let x_base = g as f32 * band + jitter * band;

            // The wrap is resolved by *which* repeats of the period fall
            // inside the visible rows, so a drop crosses the screen and
            // continues past it instead of restarting at the top edge.
            let n0 = ((y0 as f32 - base) / FALL_PERIOD).floor() as i32;
            let n1 = ((y1 as f32 - base) / FALL_PERIOD).ceil() as i32;
            for n in n0..=n1 {
                let y = base + n as f32 * FALL_PERIOD;
                if y < y0 as f32 - length - 1.0 || y > y1 as f32 + 1.0 {
                    continue;
                }
                let (len, sway) = match fall {
                    Fall::Rain => (length, 0.0),
                    // Snow does not fall straight and does not streak: it
                    // drifts. The sway is per-drop in phase, or every flake
                    // in a layer swings together like a curtain.
                    Fall::Snow => {
                        // Wider, slower sway than the first attempt. Snow
                        // that barely drifts is indistinguishable from
                        // static -- and the near layer's flakes are drawn as
                        // short strokes rather than single pixels for the
                        // same reason, so that a flake has a *size* and the
                        // three layers separate by more than brightness.
                        let w = (t * 0.009 + phase * std::f32::consts::TAU).sin();
                        (length, w * 5.5)
                    }
                };
                let x = x_base + y * slant + sway;
                out.push(Drop {
                    from: (x, y),
                    to: (x - slant * len, y - len),
                    alpha: alpha * (0.7 + jitter * 0.3),
                    colour: [colour[0] * bright, colour[1] * bright, colour[2] * bright],
                });
            }
            }
        }
    }
    out
}

/// How much a storm mutes the sky, `0.0` (clear) to `1.0` (fully overcast).
///
/// Applied to the gradient rather than drawn as cloud shapes: this world has
/// no cloud layer and inventing one would be a much bigger feature than the
/// weather it is decorating. What an overcast sky actually looks like from
/// below is a flat, desaturated, darker version of the same sky, which is
/// exactly what a lerp toward grey produces.
pub fn overcast(intensity: f32) -> f32 {
    (intensity * 0.85).clamp(0.0, 1.0)
}

/// Colour of a lightning bolt's core.
const BOLT_COLOUR: [f32; 3] = [252.0, 250.0, 255.0];

/// How far a bolt may wander sideways per cell of descent.
const BOLT_WANDER: f32 = 0.55;

/// One drawn piece of a bolt: where it starts, where it ends, and how heavy
/// it is (`1.0` trunk, less for a branch).
pub type BoltSegment = ((f32, f32), (f32, f32), f32);

/// The polyline of a lightning bolt, in **world** coordinates, plus its
/// branches.
///
/// Generated from `(id, segment)` rather than stepped, for the same reason
/// the rain is: the whole of weather here is a pure function of the frame, so
/// a bolt has no state to advance and a replay draws the same one. Branches
/// hang off the trunk at hashed points and die out, because a bolt drawn as a
/// single zigzag line reads as a crack in the screen rather than as
/// lightning — the forking is most of the silhouette.
///
/// Returned as segments rather than a path so the caller can draw each with
/// its own width: the trunk is thick and the branches thin, which is what
/// gives it depth at this resolution.
pub fn bolt(id: u64, x: i32, top_y: i32, ground_y: i32) -> Vec<BoltSegment> {
    let mut out = Vec::new();
    let span = (ground_y - top_y).max(1);
    // Coarse segments: at play zoom a bolt is a few dozen pixels tall, and
    // subdividing finer than this just costs work to draw a smoother line
    // than lightning has.
    let steps = (span / 6).clamp(4, 64);
    let mut px = x as f32;
    let mut py = top_y as f32;
    for i in 0..steps {
        let t = (i + 1) as f32 / steps as f32;
        let ny = top_y as f32 + span as f32 * t;
        // Wanders, but is pulled back toward the strike point as it nears the
        // ground, so a bolt arrives roughly where it was aimed instead of
        // drifting off across the sky.
        let drift = (hash01(id as i32, i, 0xB017) - 0.5) * 2.0 * BOLT_WANDER * (ny - py);
        let nx = (px + drift) * (1.0 - t * 0.35) + x as f32 * (t * 0.35);
        out.push(((px, py), (nx, ny), 1.0));

        // A branch, sometimes, heading off and downward and stopping short.
        if hash01(id as i32, i, 0xB4A9) < 0.28 && i > 0 {
            let dir = if hash01(id as i32, i, 0xB4AA) < 0.5 { -1.0 } else { 1.0 };
            let len = span as f32 * (0.08 + hash01(id as i32, i, 0xB4AB) * 0.16);
            let (mut bx, mut by) = (nx, ny);
            for b in 0..3 {
                let ex = bx + dir * len * 0.4 * (0.6 + hash01(id as i32, i * 8 + b, 0xB4AC));
                let ey = by + len * 0.5;
                out.push(((bx, by), (ex, ey), 0.45));
                bx = ex;
                by = ey;
            }
        }
        px = nx;
        py = ny;
    }
    out
}

/// Colour of a bolt segment.
pub fn bolt_colour() -> [f32; 3] {
    BOLT_COLOUR
}
