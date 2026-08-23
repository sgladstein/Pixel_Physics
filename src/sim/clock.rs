//! **World time — the rate the *world* ages at, independent of the rate the
//! physics runs at.**
//!
//! The engine has one clock (`World::frame`, one tick per simulated frame) and
//! three quite different things read it:
//!
//! 1. **Physics.** The CA sweep, liquids, rigid bodies, the character. One
//!    frame is one step, and that is the definition of how fast anything
//!    falls. Nothing here touches it.
//! 2. **Time of day, and the weather.** `field::sun_elevation` and everything
//!    downstream of it — the painted sky, the light channel, the temperature
//!    swing — plus `weather::channel`. All *phases*: pure functions of a frame
//!    number modulo a period.
//! 3. **Growth.** The organism and creature schedules: an entry due at
//!    `frame + interval`.
//!
//! Categories 2 and 3 are "how fast does the world age", category 1 is "how
//! fast does it fall", and conflating them is why "slow the game down" usually
//! means slow motion. They are separable because each reads the clock through
//! a different mechanism, and neither mechanism can reach the sweep:
//!
//! - A **phase** is slowed by feeding it a slower clock ([`Clock::sky_frame`]),
//!   never by changing the period — see the warning below.
//! - A **schedule** is slowed by a longer interval
//!   ([`Clock::organism_interval`]), which is also strictly less work.
//!
//! # Why the day is not slowed by raising `DAY_NIGHT_PERIOD_FRAMES`
//!
//! That constant is load-bearing for far more than the length of a day. The
//! amplitude and temperature quantisation (`SKY_LIGHT_STEP`,
//! `SKY_TEMPERATURE_QUANTUM`) are both sized against the *per-frame* rate of
//! change it implies, and the whole field-sleeping argument — the reason a
//! settled world stops solving at all — is an inequality between that rate and
//! `SETTLE_EPSILON_*`. Rescaling the period silently re-derives all of it, in
//! the direction that breaks: a slower sky moves less per frame, so a quantum
//! sized for the old rate stops registering as a change and the field freezes
//! at whatever brightness it last saw. Feeding the same functions a slower
//! clock leaves every one of those arguments exactly as it was — in sky-frame
//! units nothing has changed at all — and the field simply solves
//! proportionally *less* often.
//!
//! # What these knobs do **not** buy: behaviour preservation
//!
//! A slowed subsystem is not the same subsystem later, and the claim that it
//! is was made in an earlier draft of this design and withdrawn under review.
//! Each subsystem's *internal* economy does rescale exactly — see
//! [`Clock::growth_slowdown`] — but each one also trades with a world that is
//! still running at full speed, and every one of those exchanges is per real
//! frame:
//!
//! - **A slowed plant gets richer.** `SOIL_UPTAKE_PER_TICK` drains a root's
//!   local soil store per *tick*, and that store is refilled by rain,
//!   infiltration and capillary flow per *frame*. At `growth_slowdown: 8`
//!   there are eight times as many real frames of re-wetting between two
//!   drinks. `Reports/open-bugs-handoff.md` §U measures wood-cell count moving
//!   43% with soil moisture, and in the perverse direction, so this is not a
//!   rounding error.
//! - **A slowed organism gets poorer.** Fire, structural failure, being eaten,
//!   dug out or buried by falling powder are all per real frame, so a slowed
//!   organism absorbs N times more damage per tick of its own.
//! - **The knobs trade with each other.** Creature energetics are per creature
//!   tick, but a creature's *food* is grown on the plant knob. Running
//!   `creature_slowdown: 8` against `growth_slowdown: 1` gives a colony eight
//!   times the food per tick; the reverse starves it.
//!
//! None of these is fixable by scaling, because the other side of each
//! exchange is physics. They are stated here rather than papered over: all
//! three are invisible at the default and would otherwise read as "the model
//! broke".

use serde::{Deserialize, Serialize};

/// How much slower than baseline each knob may be run.
///
/// **Set from what actually breaks, not from a round number.** An earlier
/// draft used 60 on the reasoning that it was "past any setting anyone has
/// asked for", which is an aspiration rather than a measurement, and two
/// things break well below it:
///
/// - The gnome's forgiveness windows are `u8` on `player::Tuning`
///   (`dig_cooldown` 8, `stroke_cooldown` 7, `coyote_frames` 6). Dividing by
///   the time scale saturates `dig_cooldown` above 31, at which point the
///   dilation silently stops being dimensionally correct.
/// - `decay::DECAY_TICK_INTERVAL` (200) would overtake the organism tick
///   (45 x N) at N >= 5, so ash and litter would weather *faster* than the
///   tree producing them. Fixed by scaling decay with growth rather than by
///   capping, but it is the kind of thing a cap has to be checked against.
///
/// 30 clears both with headroom (`dig_cooldown` reaches 240 of 255) and is
/// still half an hour per day at the app's fixed 60 Hz, which is far past
/// anything play has asked for. Raising it means re-deriving both bullets.
pub const MAX_SLOWDOWN: u32 = 30;

/// The five world-time knobs, plus the anchors the phase clocks are derived
/// from.
///
/// Lives on `World` rather than on `App` because every reader is inside the
/// simulation (`field::step` and `plant::step_organisms` take a `&World` and
/// nothing else), and because a headless harness silently running a different
/// clock from the app would be the worst possible way to measure any of this
/// — the same reasoning `weather::step`'s own doc gives for being called by
/// both drivers.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Clock {
    /// **Real minutes per full day/night cycle**, at the app's fixed 60 Hz —
    /// equivalently, how many real frames pass per frame of sky time.
    ///
    /// `1` is the historical behaviour exactly: the baseline day is
    /// `field::DAY_NIGHT_PERIOD_FRAMES` = 3,600 frames = one minute, and at
    /// `1` [`Clock::sky_frame`] returns `World::frame` itself.
    ///
    /// Named in minutes rather than as a bare multiplier because the
    /// multiplier is not the quantity anyone thinks in: "how long is a day" is
    /// the question, one press is one minute, and the two are the same number
    /// only because the baseline day happens to be one minute long.
    ///
    /// **What a longer day does *not* rebalance.** The cycle spends half its
    /// length at the night floor by design (`sky_light_amplitude` clips a
    /// cosine hump), so lengthening it lengthens the flat, unchanging dark in
    /// exactly the same proportion: an eight-minute day is four minutes of
    /// night in which the light channel does not move at all. That is the
    /// accepted cost of the owner's reading of "too fast" — the cycle comes
    /// round too often, rather than dawn and dusk going past too quickly.
    /// Reshaping the curve so the transitions take a larger share is a
    /// separate change to `sun_elevation`'s *shape*, not to its rate.
    pub day_minutes: u32,

    /// **How much slower the weather changes its mind.**
    ///
    /// Deliberately independent of [`Clock::day_minutes`], which makes
    /// `weather::WEATHER_EPOCH_FRAMES`'s "two full days" a statement about the
    /// *baseline* day rather than the live one. Coupling the two was built
    /// first and withdrawn — see `Reports/dead-ends.md`.
    ///
    /// **Scales the pattern clock and nothing else in `weather`**, and that is
    /// the whole of the design rather than an omission. Rain columns, gusts,
    /// lightning windows and the cold-hold sweep all stay per real frame, so
    /// what a slowed front does is last N times longer and deliver N times
    /// more of each — which is what a slower weather system *is*. Dividing the
    /// per-frame rain budget to compensate was drafted and is wrong: rain
    /// spends per real frame and evaporation refills the atmospheric bank per
    /// real frame, so a divisor returns 1/N the water against unchanged
    /// evaporation and the world dries out. Left alone, the precipitating
    /// *fraction* of real frames is unchanged and the ledger balances itself.
    pub weather_slowdown: u32,

    /// **How much longer than baseline plants take**, as a multiplier on
    /// `plant::ORGANISM_TICK_INTERVAL` (45 frames) and on
    /// `decay::DECAY_TICK_INTERVAL`.
    ///
    /// **The internal economy rescales exactly, which is why this is one
    /// number and not a rebalance.** Every part of the plant subsystem runs on
    /// that one cadence — the growth roll, photosynthetic credit, resource
    /// transport, upkeep, thickening, abscission — so multiplying the interval
    /// leaves per-tick income and per-tick cost untouched and changes only how
    /// often a tick happens. Had photosynthesis been per-frame and growth
    /// per-tick, this knob would have made slow trees rich ones and would have
    /// needed a matching credit divisor; that property is worth re-checking
    /// before adding any per-frame plant work. For what it does *not*
    /// preserve, see this module's own header — the external exchanges are
    /// real and are not fixed by any divisor.
    ///
    /// **Decay rides with it**, and both reasons are the same reason. Litter
    /// and ash are *produced* per organism tick and weathered per real frame,
    /// so an unscaled decay would leave a slowed forest with 1/N the standing
    /// litter — and `DECAY_TICK_INTERVAL`'s own doc asks it to "read as much
    /// less frequent than a moss or tree tick", which stops being true at
    /// N >= 5. Scaling it holds both.
    pub growth_slowdown: u32,

    /// **How much longer than baseline creatures take**, as a multiplier on
    /// `creature::WORM_TICK_INTERVAL`, on each species' own `tick_interval`,
    /// and on `pheromone::PHEROMONE_INTERVAL`.
    ///
    /// Creature energetics are strictly per tick (`idle_cost` "charged every
    /// tick", `synapse_cost` "per active synapse per tick", `move_cost` per
    /// cell moved), so the interval is the whole knob.
    ///
    /// **The pheromone plane must move with it, and this is not optional.**
    /// Ant deposits are per tick, but decay and diffusion run per *pass*, and
    /// a pass is every `PHEROMONE_INTERVAL` real frames. Slow the decisions
    /// alone and a trail gets N times fewer reinforcements per evaporation —
    /// the "0 deliveries, total channel A at 100" failure that constant's own
    /// doc records, arriving by another route. Scaling the interval keeps the
    /// passes-per-tick ratio exact, which is the only change that keeps its
    /// 255-pass trail-lifetime ceiling argument true. `DECAY_RHO`, `DIFFUSE`,
    /// `DEPOSIT` and the LUT floor are all per-pass and must **not** be
    /// touched.
    pub creature_slowdown: u32,

    /// **How much slower the gnome moves.** Not a clock at all — `player.rs`
    /// reads `World::frame` nowhere and is a per-tick integrator — so this is
    /// applied as a dimensional dilation of `player::Tuning`
    /// (`Tuning::dilated`) rather than as an interval or a phase.
    pub gnome_slowdown: u32,

    /// The real frame at which the current rates took effect, and the sky and
    /// weather frames as of that moment. See [`Clock::sky_frame`] for what
    /// they are for.
    #[serde(skip)]
    anchor_frame: u64,
    #[serde(skip)]
    anchor_sky: u64,
    #[serde(skip)]
    anchor_weather: u64,
}

impl Default for Clock {
    fn default() -> Self {
        // Baseline on every knob: a world built with no opinion about time
        // behaves exactly as it did before this module existed. Every
        // pre-existing test and harness depends on that, and the physics
        // guard depends on it being a *setting* rather than a special case.
        Self {
            day_minutes: 1,
            weather_slowdown: 1,
            growth_slowdown: 1,
            creature_slowdown: 1,
            gnome_slowdown: 1,
            anchor_frame: 0,
            anchor_sky: 0,
            anchor_weather: 0,
        }
    }
}

impl Clock {
    /// Where the tunables panel persists these, beside the other asset files.
    /// Same generated-file contract as `player.ron` and `explosion.ron`: full
    /// re-serialization on save, defaults when absent.
    ///
    /// Read with `std::fs::read_to_string` at runtime, **not** `include_str!`,
    /// so it is not subject to `CLAUDE.md`'s compiled-in-asset staleness
    /// gotcha — and, like `player.ron`, it is resolved relative to the working
    /// directory, so the shipped defaults apply to a binary launched from the
    /// repository root.
    pub const ASSET_PATH: &'static str = "assets/clock.ron";

    /// Load from [`Clock::ASSET_PATH`]. `Ok(None)` means the file is simply
    /// absent, which is the normal case for a fresh checkout.
    ///
    /// **Returns the parse error rather than swallowing it.** The first draft
    /// spelled this `.ok().and_then(...).unwrap_or_default()`, which turns a
    /// malformed file into a silent revert to baseline — and a silently
    /// baseline clock is indistinguishable from a working one until somebody
    /// wonders why the day is still a minute long. `CLAUDE.md`'s own
    /// harness-echo rule is the general form: a knob whose value you cannot
    /// see is a knob you cannot tell is disconnected.
    pub fn load() -> Result<Option<Self>, String> {
        let text = match std::fs::read_to_string(Self::ASSET_PATH) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let parsed: Self = ron::from_str(&text).map_err(|e| e.to_string())?;
        Ok(Some(parsed.clamped()))
    }

    /// Full re-serialization, like `player::Tuning::save`: a generated file
    /// with no comments to lose, every field's reasoning living on the struct
    /// itself. The anchors are `serde(skip)` running state, so a saved file
    /// carries settings only.
    pub fn save(&self) -> Result<(), String> {
        let pretty = ron::ser::PrettyConfig::new().struct_names(false);
        let text = ron::ser::to_string_pretty(self, pretty).map_err(|e| e.to_string())?;
        std::fs::write(Self::ASSET_PATH, text).map_err(|e| e.to_string())
    }

    /// Every knob forced into range.
    ///
    /// **Public, and applied at every write, not only on load.** The fields
    /// are `pub` for the panel's sake, so a hand-edited file, a harness
    /// argument or a future binding can write `0` (which would divide the
    /// world by zero on one path and stall it on the other) or a number past
    /// [`MAX_SLOWDOWN`]. Every consumer happens to defend itself with
    /// `.max(1)` today; that is one refactor away from not being true, and
    /// relying on it is how a defence-in-depth becomes the only defence.
    pub fn clamped(self) -> Self {
        Self {
            day_minutes: self.day_minutes.clamp(1, MAX_SLOWDOWN),
            weather_slowdown: self.weather_slowdown.clamp(1, MAX_SLOWDOWN),
            growth_slowdown: self.growth_slowdown.clamp(1, MAX_SLOWDOWN),
            creature_slowdown: self.creature_slowdown.clamp(1, MAX_SLOWDOWN),
            gnome_slowdown: self.gnome_slowdown.clamp(1, MAX_SLOWDOWN),
            ..self
        }
    }

    /// Change a rate without moving the clock it drives.
    ///
    /// **The whole reason the anchors exist.** `sky_frame` is a function of
    /// `frame` divided by the rate, so changing the rate alone would
    /// reinterpret the *entire history* at the new one: dragging the day
    /// length from 1 to 4 at frame 3,600 would teleport the sun from noon to
    /// sunset in a single keypress. Re-anchoring first pins the current sky
    /// and weather frames, so the new rate applies from here on and the value
    /// at this instant is unchanged.
    ///
    /// Every write to a rate goes through here. `edit` receives the knobs to
    /// change; the result is clamped.
    pub fn set_rates(&mut self, frame: u64, edit: impl FnOnce(&mut Self)) {
        self.anchor_sky = self.sky_frame(frame);
        self.anchor_weather = self.weather_frame(frame);
        self.anchor_frame = frame;
        edit(self);
        *self = self.clamped();
    }

    /// **The clock every day/night reader is fed**, in place of
    /// `World::frame`.
    ///
    /// A day is `field::DAY_NIGHT_PERIOD_FRAMES` of *these*, always — which is
    /// the point. In sky-frame units nothing about the sky has changed, so
    /// `sky_light_amplitude`'s quantisation argument, the temperature
    /// staircase, `frame_for_daylight`'s phase pin and every test that names a
    /// fraction of the period stay exactly true; only the mapping from real
    /// frames to sky frames moved.
    ///
    /// # This is derived from `frame`, and an earlier version was not
    ///
    /// The first implementation was a counter incremented from
    /// `World::begin_step`. That is wrong, and wrong in a way that looks fine:
    /// **27 places in this codebase assign `World::frame` directly**, every
    /// one of them in order to select a time of day or a weather window
    /// (`field.rs`'s `night.frame = DAY_NIGHT_PERIOD_FRAMES / 2`,
    /// `weather.rs`'s cold-snap helpers, `viewshot`'s `rain=` selector,
    /// `filmstrip`'s water-cycle sheets). A counter never sees those
    /// assignments, so it stayed at 0 — noon, clear — while `world.frame`,
    /// which those tests' own assertions still read, said something else.
    /// Seven guards would have failed outright and four more would have kept
    /// passing while testing nothing.
    ///
    /// Deriving from `frame` restores the invariant that makes the whole
    /// default-is-unchanged safety argument true:
    ///
    /// > at `day_minutes == 1`, `sky_frame(f) == f` for **every** world,
    /// > however its `frame` was set.
    ///
    /// which is `debug_assert`ed below. The assert lives here rather than in
    /// `begin_step` deliberately: `begin_step` is exactly the function the
    /// affected worlds never call.
    pub fn sky_frame(&self, frame: u64) -> u64 {
        let out = self.anchor_sky + frame.saturating_sub(self.anchor_frame) / self.day_minutes.max(1) as u64;
        debug_assert!(
            self.day_minutes != 1 || self.anchor_frame != 0 || out == frame,
            "a baseline clock must be the identity on frame: sky_frame({frame}) = {out}"
        );
        out
    }

    /// [`Clock::sky_frame`] one real frame ago — the "has the sky moved"
    /// comparison point.
    ///
    /// **Not `sky_frame(frame) - 1`.** Under a lengthened day the sky clock
    /// holds still for `day_minutes` real frames at a time, and "the previous
    /// sky frame" is a change that already happened — comparing against it
    /// would report a change on every one of those frames instead of on the
    /// one the sky actually moved, so a slower day would cost *more* field
    /// solves than a fast one. Evaluating the same map one real frame back
    /// fires exactly once per genuine step, which makes a longer day
    /// proportionally cheaper. Identical to the old
    /// `world.frame.saturating_sub(1)` at the default, including at frame 0.
    pub fn prev_sky_frame(&self, frame: u64) -> u64 {
        self.sky_frame(frame.saturating_sub(1))
    }

    /// The weather's own clock, on the same derivation and for the same
    /// reasons. See [`Clock::weather_slowdown`] for why it is not the sky's.
    pub fn weather_frame(&self, frame: u64) -> u64 {
        self.anchor_weather + frame.saturating_sub(self.anchor_frame) / self.weather_slowdown.max(1) as u64
    }

    /// A base organism-schedule interval, scaled. Floored at 1: a zero
    /// interval is a tip rescheduling itself for the frame it is already on,
    /// which is an infinite loop inside one `step_active_sites` rather than
    /// "very fast growth".
    pub fn organism_interval(&self, base: u64) -> u64 {
        (base * self.growth_slowdown.max(1) as u64).max(1)
    }

    /// As [`Clock::organism_interval`], for creatures and the pheromone plane.
    pub fn creature_interval(&self, base: u64) -> u64 {
        (base * self.creature_slowdown.max(1) as u64).max(1)
    }

    /// The gnome's time scale: `1.0` at baseline, `0.25` at four times slower.
    /// See `player::Tuning::dilated` for what is done with it.
    pub fn gnome_scale(&self) -> f32 {
        1.0 / self.gnome_slowdown.max(1) as f32
    }

    /// Whether every knob is at baseline — the status line's test for whether
    /// it has anything to say beyond the day length.
    pub fn is_baseline(&self) -> bool {
        self.day_minutes == 1
            && self.weather_slowdown == 1
            && self.growth_slowdown == 1
            && self.creature_slowdown == 1
            && self.gnome_slowdown == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::field::DAY_NIGHT_PERIOD_FRAMES;

    /// **The regression test for the defect that replaced this module's first
    /// implementation.** A world whose `frame` is assigned directly — which is
    /// how every time-of-day and weather-window test in the engine selects its
    /// phase — must read exactly that frame's sky at the default.
    #[test]
    fn a_baseline_clock_is_the_identity_on_frame_however_the_frame_was_set() {
        let c = Clock::default();
        for frame in [0, 1, 137, DAY_NIGHT_PERIOD_FRAMES / 2, DAY_NIGHT_PERIOD_FRAMES, 123_456] {
            assert_eq!(c.sky_frame(frame), frame, "baseline sky time must equal real time");
            assert_eq!(c.weather_frame(frame), frame, "baseline weather time must equal real time");
        }
        // And the "previous frame" spelling keeps `saturating_sub`'s frame-0
        // behaviour, which `field::step`'s own comment is written around.
        assert_eq!(c.prev_sky_frame(0), 0);
        assert_eq!(c.prev_sky_frame(1), 0);
        assert_eq!(c.prev_sky_frame(900), 899);
    }

    #[test]
    fn a_longer_day_takes_proportionally_more_real_frames_to_come_round() {
        for minutes in [1, 2, 5, 17, MAX_SLOWDOWN] {
            let mut c = Clock::default();
            c.set_rates(0, |c| c.day_minutes = minutes);
            let real = DAY_NIGHT_PERIOD_FRAMES * minutes as u64;
            assert_eq!(
                c.sky_frame(real),
                DAY_NIGHT_PERIOD_FRAMES,
                "at {minutes} minutes, {real} real frames should be exactly one day"
            );
        }
    }

    /// The property the anchors exist for: changing the rate changes the rate,
    /// it does not reinterpret the history.
    #[test]
    fn changing_the_day_length_does_not_jump_the_time_of_day() {
        let mut c = Clock::default();
        let at = DAY_NIGHT_PERIOD_FRAMES / 4;
        let before = c.sky_frame(at);
        assert_eq!(before, at, "a quarter day in");
        c.set_rates(at, |c| c.day_minutes = 12);
        assert_eq!(c.sky_frame(at), before, "the sun jumped on a rate change");
        // ...and from there it runs at the new rate.
        assert_eq!(c.sky_frame(at + 24), before + 2);
    }

    /// The gate `field::step` reads. It must report no movement on the frames
    /// between the sky's own ticks, or a slower day costs *more* field solves
    /// than a fast one — the exact inversion `prev_sky_frame`'s doc records.
    #[test]
    fn the_sky_reports_no_movement_on_frames_between_its_own_ticks() {
        let mut c = Clock::default();
        c.set_rates(0, |c| c.day_minutes = 6);
        let moved = (1..=600).filter(|&f| c.sky_frame(f) != c.prev_sky_frame(f)).count();
        assert_eq!(moved, 100, "600 real frames at 6 minutes/day is 100 sky frames");
    }

    /// The two phase clocks are independent — the decision that
    /// `WEATHER_EPOCH_FRAMES` is two *baseline* days rather than two live ones.
    #[test]
    fn the_weather_clock_does_not_follow_the_day_clock() {
        let mut c = Clock::default();
        c.set_rates(0, |c| c.day_minutes = 8);
        assert_eq!(c.sky_frame(8_000), 1_000);
        assert_eq!(c.weather_frame(8_000), 8_000, "weather must be untouched by the day knob");
        c.set_rates(8_000, |c| c.weather_slowdown = 4);
        assert_eq!(c.weather_frame(8_000), 8_000, "and re-anchoring must not jump it");
        assert_eq!(c.weather_frame(8_040), 8_010);
    }

    #[test]
    fn intervals_scale_and_never_return_zero() {
        assert_eq!(Clock::default().organism_interval(45), 45);
        let mut c = Clock::default();
        c.set_rates(0, |c| c.growth_slowdown = 8);
        assert_eq!(c.organism_interval(45), 360);
        assert_eq!(c.creature_interval(6), 6, "the creature knob is a different knob");
        // A base of 0 cannot arise from the engine's own constants, but a
        // self-rescheduling site is bad enough to be worth the floor.
        assert_eq!(Clock::default().organism_interval(0), 1);
    }

    /// Clamping is applied at every write, not only on load — see
    /// [`Clock::clamped`].
    #[test]
    fn a_hand_edited_or_mis_set_rate_clamps_rather_than_dividing_by_zero() {
        let mut c = Clock::default();
        c.set_rates(0, |c| {
            c.day_minutes = 0;
            c.growth_slowdown = 9_999;
        });
        assert_eq!(c.day_minutes, 1);
        assert_eq!(c.growth_slowdown, MAX_SLOWDOWN);
        assert_eq!(c.sky_frame(100), 100, "a clamped-to-baseline day is still the identity");
    }

    /// **Every settable knob must reach the panel.** The destructure binds
    /// nothing and omits `..` on purpose: adding a field to `Clock` is then a
    /// compile error here rather than a knob nobody can reach, which is the
    /// failure `tunables.rs`'s own group-reachability test was written for
    /// after a whole menu shipped empty.
    ///
    /// Lives here rather than beside `from_clock` because the anchors are
    /// private to this module -- they are running state, not settings, and
    /// the panel has no business listing them.
    #[test]
    fn every_settable_knob_is_registered_in_the_panel() {
        let Clock {
            day_minutes: _,
            weather_slowdown: _,
            growth_slowdown: _,
            creature_slowdown: _,
            gnome_slowdown: _,
            anchor_frame: _,
            anchor_sky: _,
            anchor_weather: _,
        } = Clock::default();

        let listed: Vec<String> =
            crate::tunables::from_clock(&Clock::default()).into_iter().map(|t| t.name).collect();
        for knob in
            ["day_minutes", "weather_slowdown", "growth_slowdown", "creature_slowdown", "gnome_slowdown"]
        {
            assert!(listed.iter().any(|n| n == knob), "{knob} is settable but the panel never lists it");
        }
        assert_eq!(listed.len(), 5, "the panel lists something that is not a knob: {listed:?}");
    }

    /// **The shipped asset must parse, and this test says what it contains.**
    ///
    /// `App::new` reads this file while `World::new` does not, so the app and
    /// every harness deliberately run different clocks. That is the one thing
    /// in this design that could rot silently -- a typo reverts the app to
    /// baseline and nothing anywhere says so -- and `CLAUDE.md`'s rule for a
    /// divergence like this is that it be stated rather than assumed.
    ///
    /// Naming the day length here rather than just asserting "it parses" is
    /// the point: if somebody changes the shipped default, this test is where
    /// they find out that the number is load-bearing for the app's first
    /// impression, and the diff records what it moved from.
    #[test]
    fn the_shipped_clock_asset_parses_and_says_what_it_is() {
        let text = std::fs::read_to_string(Clock::ASSET_PATH).expect("assets/clock.ron must exist");
        let c: Clock = ron::from_str(&text).expect("assets/clock.ron must parse");
        assert_eq!(c.day_minutes, 8, "the shipped day is eight real minutes");
        assert_eq!(c.weather_slowdown, 1);
        assert_eq!(c.growth_slowdown, 1);
        assert_eq!(c.creature_slowdown, 1);
        assert_eq!(c.gnome_slowdown, 1);
        // And a world built without it is still baseline, which is the
        // property every other guard in the engine leans on.
        assert_eq!(Clock::default().day_minutes, 1);
    }

    #[test]
    fn gnome_scale_is_a_reciprocal() {
        assert_eq!(Clock::default().gnome_scale(), 1.0);
        let mut c = Clock::default();
        c.set_rates(0, |c| c.gnome_slowdown = 4);
        assert_eq!(c.gnome_scale(), 0.25);
    }
}
