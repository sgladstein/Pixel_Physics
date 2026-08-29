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
//! A slowed subsystem is not the same subsystem later. The claim that it is
//! was made in an earlier draft of this design, withdrawn under review, and
//! then measured -- the numbers are below and they are not small. Each
//! subsystem's *internal* economy does rescale exactly (see
//! [`Clock::growth_slowdown`]), but each one also trades with a world still
//! running at full speed, and every one of those exchanges is per real frame.
//!
//! **Measured, on the plant knob, and the effect is large.** A paired sweep
//! (`examples/plant_probe.rs`, `growth=`) grows one tree for 4,000 frames at
//! `growth_slowdown: 1` against 16,000 frames at `4` -- *the same number of
//! organism ticks either way* -- across eight world seeds. Final organism
//! cells, as a ratio of the slowed run to the baseline:
//!
//! ```text
//! seed    1     2     3     4     5     6     7     8
//! ratio 0.17  0.57  0.15  0.64  0.16  1.34  0.90  1.05     median 0.61
//! ```
//!
//! So a four-times-slower tree is typically a good deal *smaller* at the same
//! tick count, and on two seeds slightly larger: the direction is
//! seed-dependent and the spread is about 9x. Treat `growth_slowdown` as a
//! knob that changes what grows, not only how fast.
//!
//! **What it is not:** soil. The obvious explanation -- roots drink per tick
//! while soil refills per frame, so a slowed root should find a wetter world
//! -- is ruled out by measurement on the same runs, where the final soil
//! profile is essentially identical (mean 633 against 627, median 620 against
//! 623). An equally obvious one, chunk sleeping starving resource transport,
//! is ruled out by construction: transport runs on the organism tick inside
//! `plant::step_organisms`, not on sweep visits. (`plant_probe`'s own output
//! line claimed otherwise and was stale by an architecture; it was corrected
//! in the same change that measured this.)
//!
//! **What it probably is, unproven:** every exchange a plant has with the
//! world outside its own tick is per real frame, so a slowed organism meets N
//! times more of it per tick of its own -- structural checks, wind lean, fire,
//! burial. `CLAUDE.md` names a specific suspect in that family: "a structural
//! check scheduled mid-organism amputates it", because the support search is
//! hop-bounded. Not chased to ground here, and stated as an open question
//! rather than a mechanism, so that nobody reads a guess as a finding.
//!
//! **The knobs also trade with each other.** Creature energetics are per
//! creature tick, but a creature's *food* is grown on the plant knob. Running
//! `creature_slowdown: 8` against `growth_slowdown: 1` gives a colony eight
//! times the food per tick; the reverse starves it.
//!
//! None of this is fixable by scaling, because the other side of each exchange
//! is physics. It is stated here rather than papered over: all of it is
//! invisible at the default and would otherwise read as "the model broke".

use super::field::DAY_NIGHT_PERIOD_FRAMES;
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

    /// **When set, the sky stands still at this sky frame** — "make it noon
    /// and leave it there", the owner's ask, and the one thing a *rate* knob
    /// structurally cannot do. `day_minutes` at [`MAX_SLOWDOWN`] is a
    /// half-hour day, not a stopped one, and no finite slowdown ever holds a
    /// particular hour.
    ///
    /// A sky *frame*, not a phase fraction, because that is the unit every
    /// reader already takes ([`Clock::sky_frame`] is what `field::
    /// sun_elevation` and `sky.rs` are fed) — so holding is exactly "return
    /// this instead", with no second mapping to keep honest. The four cardinal
    /// values are [`SkyPin`]'s, derived from `DAY_NIGHT_PERIOD_FRAMES` rather
    /// than written out, so a change to the period moves them with it.
    ///
    /// **Held is not paused.** The physics, the weather, growth and creatures
    /// all keep running; only the sun stops. That is the split this whole
    /// module is about, one step further — `day_minutes` slows the sun,
    /// this stops it, and neither touches the sweep.
    ///
    /// Serialized, so a held sky survives a save through the panel: it is a
    /// setting someone chose, not running state like the anchors below.
    pub sky_hold: Option<u64>,

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
            sky_hold: None,
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
            // Normalised into a single period rather than range-checked. The
            // sky is a phase, so 5,400 and 1,800 are the same midnight and
            // neither is out of range -- but only one of them is recognised
            // by `SkyPin::of`, and a panel that reads its own held sky back
            // as "HELD" rather than as "MIDNIGHT" is the kind of readout
            // `CLAUDE.md` warns about. Applied on every write (`set_rates`),
            // so a hand-edited `assets/clock.ron` is normalised on load too.
            sky_hold: self.sky_hold.map(|f| f % DAY_NIGHT_PERIOD_FRAMES),
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
        // **The hold short-circuits the whole derivation**, rates and anchors
        // alike, which is what makes "stop the sun" a different thing from
        // "slow it a lot": no finite `day_minutes` ever returns a constant.
        // Checked first so the assert below is not asked a question about a
        // clock that is deliberately not advancing.
        if let Some(held) = self.sky_hold {
            return held;
        }
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
        // A held sky is emphatically not baseline, and it is the one knob
        // here whose effect a glance at the screen cannot attribute: a world
        // stuck at midnight looks exactly like a world that happens to be at
        // midnight. Said out loud in the status line, per this module's own
        // "a knob whose value you cannot see is a knob you cannot tell is
        // disconnected".
        self.sky_hold.is_none()
            && self.day_minutes == 1
            && self.weather_slowdown == 1
            && self.growth_slowdown == 1
            && self.creature_slowdown == 1
            && self.gnome_slowdown == 1
    }
}

/// **A named time of day the sky can be pinned to** — the player-facing half
/// of [`Clock::sky_hold`].
///
/// The hold itself is a bare sky frame, which is the right thing for the
/// engine and the wrong thing for a menu: "2700" is not an answer to "what
/// time is it". This is the small closed set of answers that are, in the
/// order a day runs so that stepping through the list walks the sun round
/// rather than jumping about.
///
/// **Every value is derived from `DAY_NIGHT_PERIOD_FRAMES`**, never written
/// out, so the four stay at the cardinal points if the period ever moves.
/// Dawn and dusk sit exactly on the horizon crossing (`sun_elevation == 0`),
/// which is the middle of `sky.rs`'s twilight band and therefore the most
/// strongly coloured sky the renderer draws — and, by
/// `sky_light_amplitude`'s own clipping, still ground light at the night
/// floor. That pairing is deliberate and was checked in a render rather than
/// argued: a lit twilight would need a positive elevation, which is a
/// prettier picture of a time that is not actually sunset.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SkyPin {
    /// Not held — the sun runs at whatever [`Clock::day_minutes`] says.
    #[default]
    Live,
    Dawn,
    Noon,
    Dusk,
    Midnight,
}

impl SkyPin {
    /// Listed in the order a menu shows them: the running state first, then
    /// the day in order. Not the declaration order, which puts `Live` first
    /// for `Default`'s sake and would otherwise read dawn-noon-dusk-midnight
    /// anyway -- they agree today, and `ALL` is what the panel iterates so
    /// that they may stop agreeing without the menu reordering itself.
    pub const ALL: [SkyPin; 5] = [SkyPin::Live, SkyPin::Dawn, SkyPin::Noon, SkyPin::Dusk, SkyPin::Midnight];

    pub fn label(self) -> &'static str {
        match self {
            SkyPin::Live => "LIVE",
            SkyPin::Dawn => "DAWN",
            SkyPin::Noon => "NOON",
            SkyPin::Dusk => "DUSK",
            SkyPin::Midnight => "MIDNIGHT",
        }
    }

    /// What [`Clock::sky_hold`] must be set to for this pin. `None` is the
    /// running sky, which is what makes `Live` a member of the same set
    /// rather than a separate "is it held" flag the menu would have to
    /// combine by hand.
    pub fn hold(self) -> Option<u64> {
        let p = DAY_NIGHT_PERIOD_FRAMES;
        match self {
            SkyPin::Live => None,
            // Noon is frame 0 by `field.rs`'s own convention: `cos(0) == 1`.
            SkyPin::Noon => Some(0),
            SkyPin::Dusk => Some(p / 4),
            SkyPin::Midnight => Some(p / 2),
            SkyPin::Dawn => Some(p / 4 * 3),
        }
    }

    /// Which pin a live hold corresponds to, or `None` for a hold that is
    /// not one of these.
    ///
    /// **`None` is a real answer and must not collapse to `Live`.** The hold
    /// is `pub` and lands in a `.ron` file, so "held at some other frame" is
    /// reachable — and a menu that renders it as LIVE would be showing a
    /// running sky over a stopped one, which is precisely the readout
    /// `CLAUDE.md` calls a knob you cannot tell is disconnected. The panel
    /// spells this case `HELD`.
    pub fn of(hold: Option<u64>) -> Option<SkyPin> {
        SkyPin::ALL.into_iter().find(|p| p.hold() == hold)
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
            sky_hold: _,
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

        // `sky_hold` is the one settable field that is *not* a rate, so it
        // reaches the panel through `from_pins` rather than `from_clock` --
        // as a named choice, because a bare frame number is not an answer to
        // "what time is it". Checked by name here so that the destructure
        // above stays the single place a new `Clock` field is noticed.
        let pins: Vec<String> =
            crate::tunables::from_pins(&Clock::default(), None).into_iter().map(|t| t.name).collect();
        assert!(
            pins.iter().any(|n| n == "time_of_day"),
            "sky_hold is settable but the panel never lists it: {pins:?}"
        );
    }

    /// **Every named time of day must be a distinct sky**, which is the whole
    /// claim the menu makes. Two entries landing on the same phase would be a
    /// list that looks like four choices and behaves like three -- and it is
    /// the kind of thing a change to `DAY_NIGHT_PERIOD_FRAMES` could do
    /// silently, since every value is derived from it.
    #[test]
    fn the_named_times_of_day_are_four_distinct_skies() {
        use crate::sim::field::{sun_elevation, sun_rising};
        let held: Vec<u64> = SkyPin::ALL.iter().filter_map(|p| p.hold()).collect();
        assert_eq!(held.len(), 4, "one running state and four held ones");
        for (i, a) in held.iter().enumerate() {
            for b in held.iter().skip(i + 1) {
                assert_ne!(a, b, "two named times of day are the same frame");
            }
        }
        // Named for what they are, not merely distinct: noon is the top of
        // the cosine, midnight the bottom, and the two horizon crossings are
        // told apart by which way the sun is going -- which is exactly what
        // `sky.rs` paints sunrise pink and sunset orange from.
        assert_eq!(sun_elevation(SkyPin::Noon.hold().unwrap()), 1.0);
        assert_eq!(sun_elevation(SkyPin::Midnight.hold().unwrap()), -1.0);
        for (pin, rising) in [(SkyPin::Dawn, true), (SkyPin::Dusk, false)] {
            let f = pin.hold().unwrap();
            assert!(sun_elevation(f).abs() < 1e-6, "{} should sit on the horizon", pin.label());
            assert_eq!(sun_rising(f), rising, "{} is on the wrong half of the cycle", pin.label());
        }
    }

    /// **A hold stops the sun, and releasing it resumes from where it
    /// stopped** rather than from where an unstopped clock would have
    /// reached. Both halves matter: the first is the feature, the second is
    /// what stops "stop and start" from teleporting the sky on release,
    /// which is the failure `set_rates`' anchors exist to prevent and which
    /// a hold reintroduces by another route if it bypasses them.
    #[test]
    fn a_held_sky_stands_still_and_resumes_where_it_stopped() {
        let mut c = Clock::default();
        c.set_rates(0, |c| c.sky_hold = SkyPin::Midnight.hold());
        let midnight = DAY_NIGHT_PERIOD_FRAMES / 2;
        for f in [0, 1, 900, 5_000, 1_000_000] {
            assert_eq!(c.sky_frame(f), midnight, "a held sky must not move at frame {f}");
        }
        // "Has the sky moved" must answer no, or a held world re-solves its
        // field every frame for a sun that is not going anywhere.
        assert_eq!(c.prev_sky_frame(5_000), c.sky_frame(5_000));

        c.set_rates(5_000, |c| c.sky_hold = None);
        assert_eq!(c.sky_frame(5_000), midnight, "release must not jump the sun");
        assert_eq!(c.sky_frame(5_060), midnight + 60, "and it must run again from there");
    }

    /// A hold is a phase, so an out-of-period value is normalised rather than
    /// rejected — otherwise a hand-edited asset holding at 5,400 would be the
    /// same midnight the menu offers and would still read back as `HELD`.
    #[test]
    fn a_hold_past_one_period_normalises_onto_a_named_time() {
        let mut c = Clock::default();
        c.set_rates(0, |c| c.sky_hold = Some(DAY_NIGHT_PERIOD_FRAMES + DAY_NIGHT_PERIOD_FRAMES / 2));
        assert_eq!(c.sky_hold, SkyPin::Midnight.hold());
        assert_eq!(SkyPin::of(c.sky_hold), Some(SkyPin::Midnight));
        // ...and an unnamed hold still reads as unnamed, which is the half
        // that must not be normalised away.
        c.set_rates(0, |c| c.sky_hold = Some(137));
        assert_eq!(SkyPin::of(c.sky_hold), None, "an unnamed hold must not read as one of the presets");
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

    /// **The claim the whole design exists to make: slowing the world does
    /// not slow what falls.**
    ///
    /// Run one scene at several clock settings and require a bit-identical
    /// grid. Two things about how this is built matter more than the
    /// assertion itself.
    ///
    /// # It has to be able to fail
    ///
    /// Nothing in the CA sweep reads the clock, so a scene with no sun- or
    /// weather-coupled process is bit-identical *whether or not any knob is
    /// wired to anything at all*. On its own this cannot tell "correctly
    /// separated" from "disconnected" -- `CLAUDE.md`'s "a change that moves
    /// nothing is different evidence from one that moves a little", which has
    /// already produced one whole dead-end entry here. So it carries a paired
    /// **non-grid** assertion from the same runs: the sky must have reached a
    /// different frame at each setting. If the knobs stopped doing anything,
    /// that half fails while the grid half still passes.
    ///
    /// # Calm is not enough; it also has to be dry
    ///
    /// The sky reaches the grid by more routes than wind. Rain and snow spawn
    /// cells outright; `hold_the_ground_cold` writes cell temperature, which
    /// `fire::update` compares against melting and freezing points;
    /// evaporation reads raw temperature by design; gas advection follows the
    /// wind. So the scene is bare sand and stone -- no water, no fire, no
    /// plants, no creatures, no gnome -- at a seed verified quiet on all
    /// three channels across the window.
    ///
    /// Verified across `0..FRAMES` at the *baseline*, which covers every
    /// setting: a slowdown of `k` maps the same real window onto weather
    /// frames `0..FRAMES/k`, a subset of what was checked.
    ///
    /// **This is a narrow claim, deliberately.** It says the sweep is
    /// untouched, not that a world with weather in it is. A slowed weather
    /// front genuinely does drop more rain and throw more gusts than a fast
    /// one, because it lasts longer -- see `Clock::weather_slowdown`.
    #[test]
    fn physics_is_untouched_by_every_world_clock_knob() {
        use crate::sim::cell::Cell;
        use crate::sim::chunk::Rect;
        use crate::sim::weather;
        use crate::sim::world::World;

        const FRAMES: u64 = 240;

        // A seed whose weather is quiet on all three channels for the whole
        // window. Searched rather than hardcoded so it cannot rot into a
        // seed that stopped being quiet.
        let seed = (0..4_000u64)
            .find(|&seed| {
                (0..FRAMES).all(|f| {
                    let w = weather::at(seed, f);
                    !w.is_precipitating()
                        && w.wind.abs() < weather::GUST_THRESHOLD
                        && w.chill <= weather::DRY_FROST_CHILL
                })
            })
            .expect("some seed is quiet for 240 frames");

        let run = |knobs: fn(&mut Clock)| {
            let mut w = World::new(Rect::new(0, 0, 127, 127));
            w.seed = seed;
            w.clock.set_rates(0, knobs);
            let stone = w.materials.id_of("stone").expect("stone");
            let sand = w.materials.id_of("sand").expect("sand");
            for x in 0..128 {
                w.set(x, 120, Cell::new(stone, 0));
            }
            // A column with a lip, so it topples as well as falls -- a pile
            // that only drops would be a much weaker thing to call identical.
            for y in 40..70 {
                for x in 60..68 {
                    w.set(x, y, Cell::new(sand, 0));
                }
            }
            for _ in 0..FRAMES {
                crate::sim::parallel::step(&mut w);
                w.step_active_sites();
                w.step_fields();
            }
            let grid: Vec<(crate::sim::material::MaterialId, u16)> = (0..128i32)
                .flat_map(|y| (0..128i32).map(move |x| (x, y)))
                .map(|(x, y)| {
                    let c = w.get(x, y);
                    (c.material, c.aux())
                })
                .collect();
            // The witness for the non-vacuity half. **One quantity per
            // mechanism, not one for all four**: a phase knob moves a clock
            // and an interval knob moves an interval, and probing an
            // interval knob with a clock reading is asking whether a lever
            // moved something it was never wired to. That is `CLAUDE.md`'s
            // "ask which *pixels* a lever moves" in miniature -- and it was
            // not hypothetical here: the first version of this test probed
            // only the two clocks, and the growth case failed with "left both
            // clocks at the baseline" precisely because growth is not a
            // clock.
            let witness = (
                w.sky_frame(),
                w.weather_frame(),
                w.clock.organism_interval(crate::sim::plant::ORGANISM_TICK_INTERVAL),
                w.clock.creature_interval(crate::sim::plant::ORGANISM_TICK_INTERVAL),
            );
            (grid, witness)
        };

        let (baseline, base_witness) = run(|_| {});
        // **Sensitivity, so "identical" is not trivially true.** A comparison
        // over a scene that never moved would pass whatever the knobs did.
        // The pile starts as a solid block spanning rows 40..70 and columns
        // 60..68; by the end it must have both fallen away from the top and
        // spread past its own footprint, which is only true if the sweep ran.
        let occupied = |x: i32, y: i32| {
            baseline[(y * 128 + x) as usize].0 != crate::sim::material::EMPTY
        };
        assert!(!occupied(63, 41), "the pile never fell: the grid comparison is over a static scene");
        assert!((0..128).any(|x| !(60..68).contains(&x) && occupied(x, 119)), "the pile never spread");

        for (label, knobs) in [
            ("day", (|c: &mut Clock| c.day_minutes = 8) as fn(&mut Clock)),
            ("weather", |c: &mut Clock| c.weather_slowdown = 6),
            ("growth", |c: &mut Clock| c.growth_slowdown = 5),
            ("creatures", |c: &mut Clock| c.creature_slowdown = 4),
            ("all", |c: &mut Clock| {
                c.day_minutes = 8;
                c.weather_slowdown = 6;
                c.growth_slowdown = 5;
                c.creature_slowdown = 4;
            }),
        ] {
            let (grid, witness) = run(knobs);
            let differing = grid.iter().zip(&baseline).filter(|(a, b)| a != b).count();
            assert_eq!(differing, 0, "the {label} knob moved {differing} cells of the CA grid");
            // The non-vacuity half: at least one clock must actually have
            // been somewhere else, or the paragraph above applies and this
            // test proves nothing.
            assert_ne!(
                witness, base_witness,
                "the {label} knob changed nothing the world can observe, so the grid \
                 comparison above is vacuous rather than reassuring"
            );
        }
    }

    /// **A longer day must cost *fewer* field solves per real frame, not
    /// more** — the guard for `prev_sky_frame`, which is the subtlest piece
    /// of reasoning in this module and had no test that could fail for it.
    ///
    /// The sky is what wakes a settled field: `sky_light_amplitude` and
    /// `sky_temperature_offset` are quantised precisely so that "it changed"
    /// is a crisp question, and `field::step`'s early-out fires on the frames
    /// where the answer is no. Under a lengthened day the sky clock holds
    /// still for `day_minutes` real frames at a time -- so spelling "last
    /// frame" as `sky_frame() - 1` would report a change on *every one* of
    /// those frames instead of the one it moved on, and a slower day would
    /// cost N times more solves than a fast one. Exactly backwards, and
    /// invisible in any test that only ever runs at the default.
    ///
    /// Two quantities, and the bars are set from measurement rather than
    /// from what would have been tidy:
    ///
    /// - **Per real frame it must fall sharply.** Measured on this scene:
    ///   0.393 passes per frame at one minute against 0.112 at four, a 3.5x
    ///   reduction. This is the thing anyone feels, and the thing that would
    ///   inverte outright under `sky_frame() - 1`.
    /// - **Per sky-day it is bounded, not flat**, and the first version of
    ///   this test asserted flat because that is what the argument predicts:
    ///   the sky takes the same number of quantised steps per cycle whatever
    ///   rate it runs at. Measured, it is 1,413 passes per day at one minute
    ///   and 1,615 at four -- 14% *more* for the slower day. The reason is
    ///   real and worth keeping: a pass count is not a step count, because
    ///   the field needs several passes to re-converge after each sky step.
    ///   At one minute a step lands every ~2.5 frames, before convergence
    ///   finishes, so consecutive steps are absorbed into one continuous
    ///   awake stretch; at four minutes each step gets its own full settle
    ///   and pays for it. The gap is left visible here rather than tuned
    ///   away, per `CLAUDE.md` -- the bar is 1.5x, well above the measured
    ///   1.14x and well below anything that would hide an inversion.
    ///
    /// Comparing a fixed *frame* count instead of a full cycle would have
    /// been a tautology -- 3,600 frames at 4x is a quarter of a day, so of
    /// course it costs less.
    #[test]
    fn a_longer_day_costs_fewer_field_solves_per_frame_and_stays_bounded_per_day() {
        use crate::sim::chunk::Rect;
        use crate::sim::field::DAY_NIGHT_PERIOD_FRAMES;
        use crate::sim::world::World;

        // One full sky-day at each setting, so the comparison is per cycle.
        //
        // The scene is `examples/ascii.rs`'s `field_day_scene` in miniature,
        // and every part of it is load-bearing -- an empty world was tried
        // first and reported the field solving on *every* frame at both
        // settings, which read as the early-out being broken and was really
        // the scene having no chunks to settle. Buried tiles under open sky
        // is the state the measurement is about; `end_step` is what marks
        // convergence; and the settling pass first makes the count a standing
        // state rather than the transient of a world just painted.
        let passes_per_day = |minutes: u32| {
            let mut w = World::new(Rect::new(0, 0, 127, 127));
            w.clock.set_rates(0, |c| c.day_minutes = minutes);
            for y in 64..128 {
                for x in 0..128 {
                    w.set(x, y, crate::sim::cell::Cell::new(crate::sim::material::STONE, 0));
                }
            }
            w.end_step();
            for _ in 0..200 {
                w.step_fields();
            }
            let before = w.field_stats.passes;
            let frames = DAY_NIGHT_PERIOD_FRAMES * minutes as u64;
            for _ in 0..frames {
                w.begin_step();
                w.step_fields();
                w.end_step();
            }
            (w.field_stats.passes - before, frames)
        };

        let (base_passes, base_frames) = passes_per_day(1);
        assert!(base_passes > 0, "the baseline day must solve the field at all");
        // One comparison point rather than a sweep: each extra setting costs
        // a whole simulated day at that rate, and 4x already separates the
        // correct behaviour (3.5x fewer solves per frame) from the inversion
        // it guards against (N times *more*) by a wide margin.
        {
            let minutes = 4;
            let (passes, frames) = passes_per_day(minutes);
            // Bounded per cycle -- see the doc above for why it is not flat.
            // Measured 1,615 against 1,413, so 1.5x is a real bar with
            // headroom rather than a rubber stamp.
            assert!(
                (passes as f64) < base_passes as f64 * 1.5,
                "at {minutes} minutes the field solved {passes} times per day against {base_passes} \
                 -- work per cycle should stay close to the quantiser's own step count"
            );
            // ...and therefore proportionally rarer in real time. Stated as a
            // ratio against the frame counts so it cannot be satisfied by the
            // day simply being longer.
            let base_rate = base_passes as f64 / base_frames as f64;
            let rate = passes as f64 / frames as f64;
            // Measured 3.5x lower for a 4x day; 2.0x is the bar, which an
            // inversion (N times *more* per frame) misses by a mile and a
            // legitimate change cannot trip by accident.
            assert!(
                rate < base_rate * 0.5,
                "at {minutes} minutes the field still solves {rate:.4} times per frame against \
                 {base_rate:.4} -- a slower sky is waking it as often as a fast one, which is the \
                 `sky_frame() - 1` bug"
            );
        }
    }

    /// **The premise the no-divisor decision rests on: the weather clock is
    /// an exact linear stretch, so a slowdown changes how long a front lasts
    /// and not how often one arrives.**
    ///
    /// `MAX_COLUMNS_PER_FRAME` is a per-*real*-frame deposition budget.
    /// Dividing it by the weather slowdown was drafted as "required" and is
    /// wrong: rain spends per real frame and evaporation refills the
    /// atmospheric bank per real frame, so a divisor returns 1/N the water
    /// against unchanged evaporation and the world dries out. Left alone the
    /// rate is untouched, because `weather_frame(f) = f / k` traverses the
    /// same curve more slowly -- and the share of rainy frames over a matched
    /// stretch of that curve is therefore identical, which is what this
    /// asserts.
    ///
    /// # The window has to be matched in weather frames, and that is a result
    ///
    /// Comparing a fixed *real* window across settings does not work and the
    /// numbers are worth keeping: over 400,000 real frames the rainy share
    /// measured **0.102 at baseline against 0.168 at 8x**. That is not a rate
    /// change, it is sampling -- `WEATHER_EPOCH_FRAMES` is 7,200, so the same
    /// real window covers ~55 epochs at baseline and only ~7 at 8x, and seven
    /// fronts is nowhere near the long-run mean. Anyone re-deriving this will
    /// hit the same 60% discrepancy and should not read it as a defect.
    ///
    /// # What is *not* guarded, deliberately
    ///
    /// Water actually deposited per real frame, at world level. It was
    /// attempted twice and abandoned: standing water read 3.0
    /// cell-equivalents against 2.0 over 600 frames and passed unchanged with
    /// a 4x budget divisor deliberately injected, because most of what falls
    /// never becomes a water cell; the atmospheric bank's drawdown gave the
    /// identical 3.0/2.0, because in a dry world deposition is *bank*-limited
    /// rather than budget-limited; and adding a pool so evaporation kept
    /// crediting the bank gave 0.000, because the bank is a net ledger. Three
    /// tests that cannot fail would be worse than saying this plainly, per
    /// `CLAUDE.md`. The clock mapping below is the part that is cheap, exact
    /// and genuinely breakable.
    #[test]
    fn the_weather_clock_is_an_exact_stretch_so_fronts_lengthen_rather_than_multiply() {
        use crate::sim::weather;

        // Long enough to average over many epochs (`WEATHER_EPOCH_FRAMES` is
        // 7,200) rather than over one front's worth.
        const WEATHER_FRAMES: u64 = 400_000;
        const SEED: u64 = 20_260_823;

        let share_over_matched_curve = |slowdown: u32| {
            let mut c = Clock::default();
            c.set_rates(0, |c| c.weather_slowdown = slowdown);
            let real_frames = WEATHER_FRAMES * slowdown as u64;
            let (mut wet, mut total) = (0u64, 0u64);
            for f in (0..real_frames).step_by(37 * slowdown as usize) {
                // The mapping itself, asserted rather than assumed: everything
                // above depends on it being this exact division.
                assert_eq!(c.weather_frame(f), f / slowdown as u64, "the weather clock is not a clean stretch");
                total += 1;
                if weather::at(SEED, c.weather_frame(f)).is_precipitating() {
                    wet += 1;
                }
            }
            wet as f64 / total as f64
        };

        let base = share_over_matched_curve(1);
        assert!(
            (0.02..0.6).contains(&base),
            "the baseline share is {base:.3}; a world that never rains, or always does, \
             would make every comparison below meaningless"
        );
        for slowdown in [2, 4, 8] {
            let share = share_over_matched_curve(slowdown);
            assert!(
                (share - base).abs() < 0.01,
                "over a matched stretch of the weather curve, {share:.3} of frames are rainy at \
                 {slowdown}x against {base:.3} at baseline -- the slowdown is changing how often \
                 it rains rather than how long each front lasts, and the per-frame rain budget's \
                 reasoning depends on it not doing that"
            );
        }
    }

    #[test]
    fn gnome_scale_is_a_reciprocal() {
        assert_eq!(Clock::default().gnome_scale(), 1.0);
        let mut c = Clock::default();
        c.set_rates(0, |c| c.gnome_slowdown = 4);
        assert_eq!(c.gnome_scale(), 0.25);
    }
}
