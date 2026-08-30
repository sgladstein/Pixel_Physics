//! **The speed dial, and the two-phase loop it lives in.**
//!
//! `Reports/evolution-lab-design-guide-2026-08-30.md` §1: two phases,
//! alternating. **Tending** is real time — the player plants, installs,
//! adjusts, and everything is responsive. **Running** is the experiment —
//! interaction closes and the simulation fast-forwards while the player
//! watches generations turn over.
//!
//! **The split is free rather than expensive, and that is a measurement, not
//! a hope** (feasibility §4c): determinism is required same-build and the
//! frame loop is already a fixed timestep with a catch-up loop, so raising
//! the tick budget runs the *identical* tick sequence in the identical order.
//! A fast-forwarded experiment and a real-time one are the same simulation,
//! not an approximation. The player can be told the result is exact.
//!
//! **Which is why the dial multiplies ticks and never touches a cadence.**
//! `clock.rs` measured the other reading: the same number of organism ticks
//! at 4x `growth_slowdown` produced a median **0.61x** final cells across 8
//! seeds. Running more ticks is exact; running subsystems faster is a
//! behaviour change wearing a speed control. Nothing in this file reads or
//! writes a cadence, and nothing in it may start to.
//!
//! **And why the loop is bounded by wall clock, not by tick count.** A box
//! whose stand has grown costs what it costs; a dial set to 256x on a bed
//! that can only do 41x must produce 41x and *say so*, never a frozen window
//! while it tries. So `plan` asks for a number of ticks and hands over a
//! ceiling, `record` reports what was achieved, and the readout on screen is
//! the achieved figure — the same one, from the same place, so the dial
//! cannot lie about what the box is doing.
//!
//! # The three things this file owns
//!
//! **1. The debt, and what happens to it when the box cannot keep up.** The
//! accumulator holds simulated time owed but not yet run. `plan` adds real
//! time times the multiplier; `record` subtracts *only the ticks that
//! actually ran*, and then clamps what is left to **one displayed frame's
//! worth** ([`TimeControl::carry_ceiling`]). That single invariant is what
//! makes a dial past the box's capability behave: a brief hiccup is caught
//! up, exactly as a fixed timestep is meant to, while a *sustained*
//! shortfall is discarded rather than banked. Banking it would compound —
//! every frame asking for more than the last, the window falling further
//! behind, and a catch-up sprint whenever the load lifted.
//!
//! **2. The display rate, decoupled from the frame loop.** Gate 3's call:
//! *"the display rate during Running is a design choice, not a constant. At
//! 60 Hz display the render eats a fifth of the budget; at 20 Hz it eats 7%
//! and the tick multiplier roughly triples."* So in Running, `plan` decides
//! whether this pass through the loop draws at all — and the frames that do
//! not draw spend their whole budget ticking. **This is the mechanism that
//! buys the top of the dial**, and it is why [`Advance::draw`] exists.
//! Tending never skips a frame: there the loop is throttled by the display's
//! own vsync, the player is interacting, and responsiveness is the whole
//! point.
//!
//! **3. The crossover.** Past some number of ticks between displayed frames
//! the screen stops showing *motion* and starts showing *fast-forward* — a
//! sequence of states rather than a thing moving. [`MOTION_TICKS_PER_FRAME`]
//! is where that is, measured in this box rather than argued from the
//! arithmetic, and the readout says which side of it you are on.

use std::time::Duration;

/// Which phase the lab is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Real time. The player interacts; the world runs at 1x.
    Tending,
    /// The experiment. Interaction closes and the world fast-forwards.
    Running,
}

/// What one displayed frame is allowed to do.
pub struct Plan {
    /// Ticks requested for this frame.
    pub ticks: u32,
    /// Wall-clock ceiling. The loop stops at whichever comes first, so a dial
    /// set past what the box can do costs responsiveness rather than buying
    /// it.
    pub budget: Duration,
}

/// What one pass through the frame loop actually did.
#[derive(Clone, Copy, Debug, Default)]
pub struct Advance {
    pub ticks: u32,
    pub spent: Duration,
    /// **Whether this pass should draw.** In Tending this is always true. In
    /// Running it is true only on display-rate boundaries, and the passes
    /// where it is false are the ones that buy the top of the dial: they
    /// spend their whole budget on ticks and pay nothing for a render.
    ///
    /// A caller that ignores this and draws every pass gets the behaviour
    /// this module had before the display rate was settable — correct, just
    /// slower at the top of the ladder.
    pub draw: bool,
    /// **How long the caller may sleep before coming back.** Non-zero only
    /// when there is genuinely nothing to do: no tick is due and no frame is
    /// due. Capped at [`MAX_IDLE`] so input is still pumped promptly.
    ///
    /// Without this, a Running phase whose dial the box can easily meet
    /// spins a core between displayed frames — the frame loop is
    /// `ControlFlow::Poll`, and skipping the draw also skips the vsync that
    /// was throttling it.
    pub idle: Duration,
}

/// The dial's state.
pub struct TimeControl {
    pub phase: Phase,
    /// Requested multiple of real time. 1 is a second of simulation per
    /// second of wall clock.
    pub requested: u32,
    /// **Achieved multiple of real time, measured over real seconds** — what
    /// the readout shows, and the honest half of the dial.
    ///
    /// Simulated seconds advanced divided by real seconds elapsed, where the
    /// real seconds are the frame loop's own wall clock and therefore include
    /// the render, the event pump and everything else a frame pays for. An
    /// earlier draft divided by the *tick loop's* time only, which flatters
    /// the number by exactly the quantity Gate 3 is about.
    pub achieved: f32,
    /// Displayed frames per second of real time. See [`DISPLAY_RATES`].
    display_hz: u32,
    /// Simulated time owed and not yet run. See the module note.
    sim_debt: Duration,
    /// Real time since the last displayed frame.
    display_accum: Duration,
    /// Whether the pass now in flight is a displayed one — decided in `plan`,
    /// read in `record` and by the caller.
    drawing: bool,
    /// What `plan` decided the caller may sleep for; handed out by `record`.
    idle: Duration,
    /// Everything a *displayed* pass costs that is not ticking: the render,
    /// the present, the event pump. Measured rather than assumed, because it
    /// is what the tick budget has to leave room for and it differs by an
    /// order of magnitude between a settled box and a moving one.
    overhead: Duration,
    /// The previous pass's tick time and whether it drew — the two operands
    /// of the `overhead` sample.
    last_spent: Duration,
    last_drew: bool,
    /// Ticks run since the last displayed frame, and the last displayed
    /// frame's total. The second is the crossover's input and it is a
    /// **counter**, not a timing: identical under any machine load.
    pending_ticks: u32,
    shown_ticks: u32,
    /// The rolling window `achieved` is computed over. Real time comes from
    /// `plan`, ticks from `record`.
    window_real: Duration,
    window_ticks: u64,
}

/// The simulation's own rate. One tick is 1/60th of a simulated second, on
/// both binaries and in every harness — it is the unit the whole engine's
/// cadences are written in.
pub const TICKS_PER_SECOND: u32 = 60;
pub const TICK: Duration = Duration::from_nanos(1_000_000_000 / TICKS_PER_SECOND as u64);

/// How much real time one `plan` call will believe. A frame that took longer
/// than this is a stall — the window was dragged, the machine swapped — and
/// billing the simulation for it produces a lurch nobody asked for.
const MAX_ELAPSED: Duration = Duration::from_millis(250);

/// The tick loop never gets *all* of a displayed frame: `overhead` is
/// subtracted first, and this is the floor under what is left. Without it a
/// box whose render alone exceeds the display interval would tick zero times
/// and the simulation would stop dead rather than merely running slowly.
const MIN_BUDGET: Duration = Duration::from_millis(4);

/// The longest [`Advance::idle`] ever handed out. Input is pumped between
/// passes, so this is the worst-case latency a keypress can see while the
/// loop is idling.
pub const MAX_IDLE: Duration = Duration::from_millis(2);

/// How much real time `achieved` averages over. Long enough that a single
/// slow frame does not move the readout, short enough that the number
/// responds when the dial does.
const RATE_WINDOW: Duration = Duration::from_millis(500);

/// **Where the screen stops showing motion, in ticks between displayed
/// frames.** Measured in the lab box by `examples/labdial.rs mode=census`,
/// not derived from the arithmetic.
///
/// The arithmetic — `60*M/D` ticks a frame, a falling cell moving about one
/// cell a tick — is only an **upper bound**, and in this box it is a loose
/// one: nothing in a sealed lab is in free fall for long, and the only things
/// that translate at all are the ants. Censused over 6,000 ticks on three
/// seeds, an ant's mean displacement is **0.06–0.07 cells per tick**, so the
/// arithmetic overstates by about **14x**.
///
/// The criterion is that apparent motion holds while a mover's net
/// displacement between two displayed frames stays inside **its own body** —
/// past that the eye can no longer match a feature to itself and a sequence
/// of frames stops being a moving thing. An ant is **2.00 cells** (measured,
/// mean over 227k–241k organism-frames), and the fastest tenth of ants
/// crosses two cells at exactly 12 ticks a frame on every seed tried:
///
/// | ticks/frame | 4 | 8 | **12** | 16 | 24 | 48 | 192 | 768 |
/// |---|---|---|---|---|---|---|---|---|
/// | p90 displacement, cells | 1.00 | 1.41 | **2.00** | 2.24 | 3.00 | 5.00 | 11.3 | 21.4 |
///
/// **The multiplier this becomes depends on the display rate**, which is the
/// consequence worth carrying: `M* = MOTION_TICKS_PER_FRAME * D / 60`, so 12x
/// at a 60 Hz display and **4x at 20 Hz**. A lower display rate buys tick
/// throughput (Gate 3) and spends the motion half of the dial to do it.
///
/// The positive control is printed by the census and it fired: at **one** tick
/// between frames the largest displacement any ant managed was **1.41 cells**,
/// which is a single diagonal step and the hard bound. A reading of 0.00 there
/// would have meant the probe never ran, and a null and a dead probe look
/// identical.
pub const MOTION_TICKS_PER_FRAME: u32 = 12;

impl Default for TimeControl {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeControl {
    pub fn new() -> Self {
        Self {
            phase: Phase::Tending,
            requested: 1,
            achieved: 1.0,
            display_hz: DISPLAY_RATES[0],
            sim_debt: Duration::ZERO,
            display_accum: Duration::ZERO,
            drawing: true,
            idle: Duration::ZERO,
            overhead: Duration::ZERO,
            last_spent: Duration::ZERO,
            last_drew: false,
            pending_ticks: 0,
            shown_ticks: 0,
            window_real: Duration::ZERO,
            window_ticks: 0,
        }
    }

    /// The multiplier actually in force. Tending is 1x by definition — the
    /// phase *is* the statement that the world runs at real time.
    pub fn multiple(&self) -> u32 {
        match self.phase {
            Phase::Tending => 1,
            Phase::Running => self.requested.max(1),
        }
    }

    pub fn display_hz(&self) -> u32 {
        self.display_hz
    }

    /// One displayed frame's share of real time.
    pub fn display_interval(&self) -> Duration {
        Duration::from_nanos(1_000_000_000 / self.display_hz.max(1) as u64)
    }

    /// **The most simulated time the debt may hold: one displayed frame's
    /// worth.**
    ///
    /// The whole of the accumulator's behaviour under a dial the box cannot
    /// meet is this one line. Above the ceiling the shortfall is *discarded*,
    /// which is what makes the achieved rate converge to what the box can do
    /// instead of compounding into an unpayable backlog. Below it the debt
    /// carries, which is what makes a single slow frame get caught up rather
    /// than dropped — a fixed timestep's whole purpose.
    fn carry_ceiling(&self) -> Duration {
        (self.display_interval() * self.multiple()).max(TICK)
    }

    /// How many ticks this pass through the loop should try for, how long it
    /// may take doing them, and — via `record` — whether it draws.
    ///
    /// `elapsed` is real time since the previous call. **Nothing in here
    /// reads a clock**, so the whole dial is testable with no window and no
    /// world.
    pub fn plan(&mut self, elapsed: Duration) -> Plan {
        let elapsed = elapsed.min(MAX_ELAPSED);

        // The render's cost, sampled off the pass that actually paid it. Two
        // conditions, and both were found by asking what the number says when
        // nothing is wrong:
        //
        // - only a **displayed** pass carries a render, so a skipped one is
        //   not a sample; folding it in would report the render as free and
        //   hand the tick loop a budget it cannot honour;
        // - only in **Running**. In Tending every pass draws and the loop is
        //   vsync-throttled, so `elapsed - spent` there is the wait for the
        //   display, not the cost of drawing -- about 16 ms, which would
        //   collapse the budget to its floor the moment the player asked for
        //   speed and hold it there for the first tenth of a second.
        if self.last_drew && self.phase == Phase::Running {
            let sample = elapsed.saturating_sub(self.last_spent);
            self.overhead = ewma(self.overhead, sample, 0.25);
        }

        // Whether this pass draws. Tending always does: the loop is vsync-
        // throttled there and the player is interacting.
        self.display_accum += elapsed;
        self.drawing = match self.phase {
            Phase::Tending => true,
            Phase::Running => {
                let interval = self.display_interval();
                if self.display_accum >= interval {
                    // Subtract one interval rather than zeroing, so the
                    // display rate is held on average; clamp the carry to one
                    // interval so a stall cannot buy a burst of draws.
                    self.display_accum = (self.display_accum - interval).min(interval);
                    true
                } else {
                    false
                }
            }
        };

        self.sim_debt += elapsed * self.multiple();
        let ticks = self.owed_ticks();

        // What is left to do decides whether the caller may sleep. Idling is
        // only ever correct when *both* halves are quiet: no tick is due and
        // no frame is due.
        self.idle = if ticks == 0 && !self.drawing {
            let to_tick = TICK.saturating_sub(self.sim_debt) / self.multiple().max(1);
            let to_frame = self.display_interval().saturating_sub(self.display_accum);
            to_tick.min(to_frame).min(MAX_IDLE)
        } else {
            Duration::ZERO
        };

        self.window_real += elapsed;

        Plan { ticks, budget: self.budget() }
    }

    /// Whole ticks the debt currently owes. Separate from `plan` so a caller
    /// — a test, a harness — can read the request without the side effects of
    /// planning a frame. An earlier draft of the tests below "peeked" by
    /// calling `plan(ZERO)`, which silently consumed a display interval and
    /// zeroed the render-cost sample: exactly `CLAUDE.md`'s *a debug readout
    /// must not be a function of the thing it debugs*, one level down.
    fn owed_ticks(&self) -> u32 {
        (self.sim_debt.as_nanos() / TICK.as_nanos()) as u32
    }

    /// The wall-clock ceiling on one pass's tick loop.
    ///
    /// **Derived from the display rate and the measured render cost**, not a
    /// constant — which is the whole of Gate 3's mechanism. Dropping from 60
    /// Hz to 20 Hz triples the interval *and* pays the render a third as
    /// often, and both show up here.
    fn budget(&self) -> Duration {
        let interval = self.display_interval();
        interval.saturating_sub(self.overhead).max(MIN_BUDGET).min(interval.max(MIN_BUDGET))
    }

    /// Record what the pass achieved, and return it.
    pub fn record(&mut self, ticks: u32, spent: Duration) -> Advance {
        // Only what actually ran is paid off. The placeholder this replaced
        // deducted the whole *request* in `plan` and then zeroed the
        // accumulator whenever the ceiling was hit, which meant the debt
        // could not distinguish "this frame ran short" from "this box cannot
        // do this at all".
        self.sim_debt = self.sim_debt.saturating_sub(TICK * ticks);
        // ...and the shortfall is not banked. See `carry_ceiling`.
        self.sim_debt = self.sim_debt.min(self.carry_ceiling());

        self.pending_ticks = self.pending_ticks.saturating_add(ticks);
        if self.drawing {
            self.shown_ticks = self.pending_ticks;
            self.pending_ticks = 0;
        }

        self.window_ticks += ticks as u64;
        if self.window_real >= RATE_WINDOW {
            self.achieved = self.window_ticks as f32
                / TICKS_PER_SECOND as f32
                / self.window_real.as_secs_f32();
            self.window_real = Duration::ZERO;
            self.window_ticks = 0;
        }

        self.last_spent = spent;
        self.last_drew = self.drawing;

        Advance { ticks, spent, draw: self.drawing, idle: self.idle }
    }

    /// **Ticks between displayed frames** — the crossover's input, and the
    /// one number on the readout that is a counter rather than a clock.
    pub fn ticks_per_frame(&self) -> u32 {
        self.shown_ticks
    }

    /// Which side of the crossover the screen is on. `true` while what is
    /// displayed still reads as a thing moving.
    pub fn reads_as_motion(&self) -> bool {
        self.ticks_per_frame() <= MOTION_TICKS_PER_FRAME
    }

    /// What the dial *would* put on screen at the requested multiplier, if
    /// the box could meet it. Distinct from `ticks_per_frame`, which is what
    /// it actually did — the two diverge exactly when the dial is past the
    /// box's capability, which is the case the readout exists for.
    pub fn requested_ticks_per_frame(&self) -> u32 {
        (TICKS_PER_SECOND * self.multiple()).div_ceil(self.display_hz.max(1))
    }

    /// Whether the frame must be fully redrawn because this module painted
    /// over the terrain last frame. Always: the readout is never hidden, so
    /// last frame's text is always sitting on this frame's terrain.
    pub fn hud_is_dirty(&self) -> bool {
        true
    }

    /// The readout, as the lines it draws. Split out from `draw` so a test
    /// can check every character against the font — a missing glyph draws as
    /// a silent blank and that has shipped three times in this repo
    /// (`hud.rs`'s own `[`/`]`, `_`/`<`/`>` and `;`/`'` notes).
    pub fn readout(&self, frame: u64) -> Vec<(String, [u8; 4])> {
        let white = [235u8, 235, 235, 255];
        let grey = [150u8, 150, 150, 255];
        let mut lines = Vec::new();

        match self.phase {
            Phase::Tending => lines.push(("TENDING - 1X REAL TIME".to_string(), white)),
            Phase::Running => {
                lines.push((format!("RUNNING - ASKED {}X", self.requested), white))
            }
        }
        lines.push((
            format!("GOT {:.1}X AT {}HZ", self.achieved.max(0.0), self.display_hz),
            white,
        ));
        lines.push((format!("SIM {} PER REAL SECOND", sim_per_second(self.achieved)), grey));
        lines.push((format!("{} TICKS PER FRAME", self.ticks_per_frame()), grey));
        // The crossover, named on screen rather than left to be inferred.
        lines.push(if self.reads_as_motion() {
            (format!("MOTION - UP TO {MOTION_TICKS_PER_FRAME} PER FRAME"), [140, 210, 140, 255])
        } else {
            (format!("FAST-FORWARD - OVER {MOTION_TICKS_PER_FRAME} PER FRAME"), [235, 185, 90, 255])
        });
        lines.push((format!("FRAME {frame}"), grey));
        lines
    }

    pub fn draw(&self, frame: &mut [u8], world: &crate::sim::world::World) {
        for (i, (line, colour)) in self.readout(world.frame).into_iter().enumerate() {
            crate::hud::draw_text(
                frame,
                super::WIDTH,
                super::HEIGHT,
                4,
                4 + 10 * i as i32,
                &line,
                colour,
            );
        }
    }
}

/// Simulated world time per real second, in words the player thinks in.
///
/// The guide asks for *simulated-time-per-real-second* on screen and it is
/// numerically the same quantity as the multiplier — which is exactly why it
/// earns a second line only if it is stated in a different unit. "64X" is a
/// ratio; "1M 4S" is how much world goes by while you watch.
fn sim_per_second(rate: f32) -> String {
    let seconds = rate.max(0.0).round() as u64;
    match seconds {
        0 => format!("{:.1}S", rate.max(0.0)),
        // The break is at a real minute, not at a round-looking 100. Above 60
        // the second line has to carry a *different* unit from the multiplier
        // to be worth its row: at 64x, "64S" restates the dial and "1M 4S"
        // says how much world goes by while you watch.
        1..=59 => format!("{seconds}S"),
        60..=3599 => format!("{}M {}S", seconds / 60, seconds % 60),
        _ => format!("{}H {}M", seconds / 3600, (seconds % 3600) / 60),
    }
}

fn ewma(current: Duration, sample: Duration, alpha: f64) -> Duration {
    if current.is_zero() {
        return sample;
    }
    let blended = current.as_secs_f64() * (1.0 - alpha) + sample.as_secs_f64() * alpha;
    Duration::from_secs_f64(blended.max(0.0))
}

/// The dial's stops.
///
/// **Presets rather than a free number**, because the interesting range is
/// multiplicative and a linear key-repeat would spend most of its travel in
/// the part nobody wants. The top of the ladder is deliberately past what any
/// box can achieve: the dial is a *request*, and a request the machine cannot
/// meet is how the readout earns its keep.
///
/// The first six are what `bin/lab.rs`'s number row is bound to and their
/// meaning must not be reshuffled; the arrow keys reach the seventh.
pub const PRESETS: [u32; 7] = [1, 2, 4, 16, 64, 256, 1024];

/// The display rates the player can pick between, fastest first.
///
/// Gate 3: *"At 60 Hz display the render eats a fifth of the budget; at 20 Hz
/// it eats 7% and the tick multiplier roughly triples."* 10 Hz is included
/// because the top of the dial is already far past the crossover, and a
/// player watching a fast-forward is not watching for smoothness.
pub const DISPLAY_RATES: [u32; 4] = [60, 30, 20, 10];

impl TimeControl {
    pub fn toggle_phase(&mut self) {
        self.phase = match self.phase {
            Phase::Tending => Phase::Running,
            Phase::Running => Phase::Tending,
        };
        self.reset_pacing();
    }

    /// Step up the ladder. Entering Running on the first press up from 1x is
    /// deliberate: asking for speed is the same gesture as starting the
    /// experiment.
    pub fn faster(&mut self) {
        let i = PRESETS.iter().position(|p| *p >= self.requested).unwrap_or(0);
        self.set_preset((i + 1).min(PRESETS.len() - 1));
    }

    pub fn slower(&mut self) {
        let i = PRESETS.iter().position(|p| *p >= self.requested).unwrap_or(0);
        self.set_preset(i.saturating_sub(1));
    }

    pub fn set_preset(&mut self, i: usize) {
        self.requested = PRESETS[i.min(PRESETS.len() - 1)];
        self.phase = if self.requested > 1 { Phase::Running } else { Phase::Tending };
        self.reset_pacing();
    }

    /// Step through [`DISPLAY_RATES`], wrapping.
    pub fn cycle_display_rate(&mut self) {
        let i = DISPLAY_RATES.iter().position(|r| *r == self.display_hz).unwrap_or(0);
        self.set_display_hz(DISPLAY_RATES[(i + 1) % DISPLAY_RATES.len()]);
    }

    /// Set the display rate directly. Clamped rather than asserted, so a
    /// harness sweeping it cannot stall the loop with a zero.
    pub fn set_display_hz(&mut self, hz: u32) {
        self.display_hz = hz.clamp(1, 240);
        self.display_accum = Duration::ZERO;
        self.reset_pacing();
    }

    /// Drop the debt and the rate window. Called wherever the dial changes:
    /// the debt was accrued at the old multiplier and the window's ticks were
    /// run at it, so carrying either across reports the *previous* setting
    /// for half a second after the player moved the dial.
    fn reset_pacing(&mut self) {
        self.sim_debt = Duration::ZERO;
        self.window_real = Duration::ZERO;
        self.window_ticks = 0;
        self.pending_ticks = 0;
        // The display phase and the render sample are as stale as the debt.
        // A Tending stretch accrues `display_accum` it never spends (it draws
        // every pass), so without this the first Running pass after a long
        // Tending one would find the accumulator arbitrarily large; and the
        // render cost measured at the previous dial setting is not this one's.
        self.display_accum = Duration::ZERO;
        self.overhead = Duration::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake frame loop, closed over its own clock.
    ///
    /// **Every duration here is an input, never a reading.** `CLAUDE.md`: gate
    /// on counters, never on wall clock — three other agents compiling on this
    /// box moved a byte-identical timing 2.42x, and a test that asserts on
    /// `Instant::now()` is a flake generator. So this harness *states* what a
    /// tick costs and what a render costs, and closes the loop the way a real
    /// one does: the elapsed time handed to the next `plan` is exactly what
    /// the previous pass consumed — ticks, plus the render if it drew, plus
    /// the event pump, plus whatever it was told it could idle for.
    ///
    /// Closing that loop is the whole point. An earlier draft fed `plan` a
    /// *fixed* frame gap independent of what the ticks cost, which let a pass
    /// spend 16.7 ms ticking inside a 16 ms frame and flattered the achieved
    /// rate by exactly the amount being tested.
    struct Machine {
        per_tick: Duration,
        render: Duration,
        pump: Duration,
        /// What the next `plan` will be told elapsed.
        gap: Duration,
        /// Total elapsed, so a test can run "one second" without a clock.
        real: Duration,
        ticks: u64,
        draws: u64,
    }

    impl Machine {
        /// `per_tick_ns` rather than microseconds because the fastest fake
        /// machine below has to be able to meet 256x, and a microsecond a tick
        /// cannot.
        fn new(per_tick_ns: u64, render_ms: u64) -> Self {
            Self {
                per_tick: Duration::from_nanos(per_tick_ns),
                render: Duration::from_millis(render_ms),
                pump: Duration::from_micros(100),
                gap: Duration::from_millis(1),
                real: Duration::ZERO,
                ticks: 0,
                draws: 0,
            }
        }

        /// One pass through the loop: `plan`, the wall-clock-bounded catch-up
        /// loop `Lab::advance` runs, then `record`.
        fn pass(&mut self, t: &mut TimeControl) -> Advance {
            let plan = t.plan(self.gap);
            let mut ran = 0u32;
            let mut spent = Duration::ZERO;
            while ran < plan.ticks {
                ran += 1;
                spent += self.per_tick;
                if spent >= plan.budget {
                    break;
                }
            }
            let a = t.record(ran, spent);
            self.gap =
                spent + self.pump + a.idle + if a.draw { self.render } else { Duration::ZERO };
            self.real += self.gap;
            self.ticks += ran as u64;
            self.draws += a.draw as u64;
            a
        }

        fn run_for(&mut self, t: &mut TimeControl, real: Duration) {
            let until = self.real + real;
            while self.real < until {
                self.pass(t);
            }
        }
    }

    #[test]
    fn tending_runs_one_tick_per_sixtieth_of_a_second() {
        let mut t = TimeControl::new();
        let mut m = Machine::new(10_000, 3);
        m.run_for(&mut t, Duration::from_secs(10));
        // A fixed timestep: whatever the frame rate, the tick count is the
        // real time divided by the tick.
        let want = (m.real.as_nanos() / TICK.as_nanos()) as u64;
        assert!(
            m.ticks.abs_diff(want) <= 2,
            "tending drifted: {} ticks against {want} of real time",
            m.ticks
        );
        assert_eq!(t.phase, Phase::Tending);
    }

    #[test]
    fn tending_always_draws() {
        let mut t = TimeControl::new();
        let mut m = Machine::new(10_000, 3);
        for _ in 0..500 {
            assert!(m.pass(&mut t).draw, "tending must never skip a frame");
        }
    }

    /// **The one the brief names.** A dial set past the machine's capability
    /// must report the achieved rate, not the requested one.
    ///
    /// This machine is *stated* to cost 200 microseconds a tick, so 256x —
    /// which needs 15,360 ticks a second — would need 3.07 seconds of ticking
    /// per second of wall clock. It cannot be met, and the readout has to say
    /// so rather than echoing the dial back.
    #[test]
    fn a_dial_past_the_machine_reports_what_the_machine_did() {
        let mut t = TimeControl::new();
        t.set_preset(5);
        assert_eq!(t.requested, 256);
        let mut m = Machine::new(200_000, 3);
        m.run_for(&mut t, Duration::from_secs(5));

        assert!(t.achieved > 1.0, "it should still be fast-forwarding: {}", t.achieved);
        assert!(
            t.achieved < 200.0,
            "the readout claimed {}x on a machine that cannot do 256x",
            t.achieved
        );
        // The truth check on the readout: what it says has to match what the
        // loop actually ran, which the harness counted independently.
        let truth = m.ticks as f32 / TICKS_PER_SECOND as f32 / m.real.as_secs_f32();
        assert!(
            (t.achieved - truth).abs() < truth * 0.25,
            "readout says {}x against a counted {truth}x",
            t.achieved
        );
        // ...and the request stays on screen unchanged, so the player sees the
        // gap rather than being quietly handed a different dial.
        assert_eq!(t.requested, 256);
    }

    /// The sensitivity half of the test above. `CLAUDE.md`: *put the fault the
    /// guard is named for back and watch it go red.* The fault is "reports the
    /// request rather than the achievement" — and a `TimeControl` that always
    /// reported a low number would pass the test above and fail this one.
    #[test]
    fn a_dial_the_machine_can_meet_reports_the_full_rate() {
        let mut t = TimeControl::new();
        t.set_preset(5);
        // One nanosecond a tick: 256x needs 15,360 ticks a second, and this
        // machine does a billion.
        let mut m = Machine::new(1, 3);
        m.run_for(&mut t, Duration::from_secs(5));
        assert!(
            t.achieved > 200.0,
            "a machine that can easily do 256x reported only {}x",
            t.achieved
        );
    }

    /// **The accumulator under a budget hit, repeatedly.** The failure this
    /// guards is compounding: a debt that banks every frame's shortfall grows
    /// without bound, the request climbs for ever, and the world sprints the
    /// moment the load lifts.
    #[test]
    fn a_sustained_shortfall_is_not_banked() {
        let mut t = TimeControl::new();
        t.set_preset(6);
        assert_eq!(t.requested, 1024);
        let mut m = Machine::new(200_000, 3);
        let mut peak = 0u32;
        let until = Duration::from_secs(8);
        while m.real < until {
            peak = peak.max(t.owed_ticks());
            m.pass(&mut t);
        }

        let ceiling = t.carry_ceiling();
        assert!(
            t.sim_debt <= ceiling,
            "debt {:?} exceeded the one-frame ceiling {ceiling:?}",
            t.sim_debt
        );
        // One displayed frame at 1024x and 60 Hz is 1,024 ticks. Eight real
        // seconds of an unbounded debt would be asking for 8 * 60 * 1024.
        assert!(
            peak <= 1200,
            "the request grew to {peak} ticks a frame, which is a banked backlog"
        );
    }

    /// The other half of the ceiling: a *brief* hiccup is still caught up,
    /// which is what a fixed timestep is for. Without this, "always discard"
    /// would pass the test above and be wrong.
    #[test]
    fn one_missed_frame_is_caught_up() {
        let mut t = TimeControl::new();
        t.set_preset(1); // 2x
        let mut m = Machine::new(1, 3);
        m.pass(&mut t);
        // A pass with no budget at all: plan it, run nothing, record it.
        let _ = t.plan(Duration::from_millis(50));
        t.record(0, Duration::from_millis(50));
        assert!(t.owed_ticks() > 0, "a skipped frame's ticks were dropped rather than owed");
    }

    #[test]
    fn the_display_rate_is_what_gets_drawn() {
        for hz in DISPLAY_RATES {
            let mut t = TimeControl::new();
            t.set_preset(4); // 64x, so Running
            t.set_display_hz(hz);
            let mut m = Machine::new(1, 2);
            m.run_for(&mut t, Duration::from_secs(4));
            let per_second = m.draws as f64 / m.real.as_secs_f64();
            assert!(
                (per_second - hz as f64).abs() <= 1.5,
                "asked for {hz} Hz and drew {per_second:.1} frames a second"
            );
        }
    }

    /// **Gate 3's mechanism, as a counter.** A lower display rate must buy
    /// ticks: the render is paid a third as often and the tick loop gets the
    /// difference. Nothing here reads a clock — the render's cost is stated.
    #[test]
    fn a_lower_display_rate_buys_ticks() {
        let run = |hz: u32| {
            let mut t = TimeControl::new();
            t.set_preset(6); // 1024x, far past this machine
            t.set_display_hz(hz);
            let mut m = Machine::new(50_000, 8);
            m.run_for(&mut t, Duration::from_secs(8));
            m.ticks
        };
        let at_60 = run(60);
        let at_20 = run(20);
        assert!(
            at_20 > at_60,
            "20 Hz ran {at_20} ticks against 60 Hz's {at_60}; the display rate \
             is not decoupled from the tick loop"
        );
    }

    #[test]
    fn the_budget_leaves_room_for_a_measured_render() {
        let mut t = TimeControl::new();
        t.set_preset(5);
        t.set_display_hz(20); // 50 ms interval
        let mut m = Machine::new(50_000, 8);
        m.run_for(&mut t, Duration::from_secs(4));
        assert!(
            t.budget() <= t.display_interval(),
            "budget {:?} exceeds the display interval {:?}",
            t.budget(),
            t.display_interval()
        );
        assert!(t.budget() >= MIN_BUDGET, "budget collapsed to {:?}", t.budget());
        // The render was stated at 8 ms and the pump at 0.1, so the measured
        // overhead has to have found roughly that and left the rest.
        assert!(
            t.overhead >= Duration::from_millis(7) && t.overhead <= Duration::from_millis(10),
            "overhead measured {:?} for an 8 ms render",
            t.overhead
        );
    }

    #[test]
    fn idling_only_happens_when_there_is_nothing_to_do() {
        let mut t = TimeControl::new();
        t.set_preset(1); // 2x
        t.set_display_hz(20);
        let mut m = Machine::new(1, 2);
        let mut idled = 0;
        for _ in 0..5000 {
            let a = m.pass(&mut t);
            if !a.idle.is_zero() {
                assert_eq!(a.ticks, 0, "idled with ticks to run");
                assert!(!a.draw, "idled on a frame that draws");
                assert!(a.idle <= MAX_IDLE);
                idled += 1;
            }
        }
        assert!(idled > 0, "a 2x dial on an instant machine never once idled");
    }

    #[test]
    fn ticks_per_frame_counts_the_gap_between_displayed_frames() {
        let mut t = TimeControl::new();
        t.set_preset(4); // 64x
        t.set_display_hz(20);
        let mut m = Machine::new(1, 2);
        m.run_for(&mut t, Duration::from_secs(4));
        // 64x at 20 Hz is 60*64/20 = 192 ticks between displayed frames, and
        // this machine can meet it.
        let n = t.ticks_per_frame();
        assert!((175..=210).contains(&n), "{n} ticks per frame, expected about 192");
        assert_eq!(t.requested_ticks_per_frame(), 192);
        assert!(!t.reads_as_motion(), "192 ticks a frame is not motion");
    }

    #[test]
    fn the_crossover_is_reported_on_both_sides() {
        let mut t = TimeControl::new();
        t.shown_ticks = MOTION_TICKS_PER_FRAME;
        assert!(t.reads_as_motion());
        t.shown_ticks = MOTION_TICKS_PER_FRAME + 1;
        assert!(!t.reads_as_motion());
    }

    /// **The readout must be drawable.** A missing glyph draws as a silent
    /// blank, which `hud.rs` records having shipped three separate times.
    #[test]
    fn every_readout_line_uses_glyphs_the_font_has() {
        let mut t = TimeControl::new();
        let mut seen = 0;
        for preset in 0..PRESETS.len() {
            t.set_preset(preset);
            for hz in DISPLAY_RATES {
                t.set_display_hz(hz);
                // Rates that reach every arm of `sim_per_second`.
                for rate in [0.0f32, 0.4, 1.0, 41.2, 99.0, 100.0, 3599.0, 3600.0, 61234.0] {
                    t.achieved = rate;
                    for ticks in [0u32, MOTION_TICKS_PER_FRAME, MOTION_TICKS_PER_FRAME + 1, 4096] {
                        t.shown_ticks = ticks;
                        for (line, _) in t.readout(1_234_567) {
                            for c in line.chars() {
                                assert!(
                                    crate::hud::has_glyph(c),
                                    "readout line {line:?} needs a glyph for {c:?}"
                                );
                                seen += 1;
                            }
                        }
                    }
                }
            }
        }
        assert!(seen > 1000, "the glyph sweep barely ran: {seen} characters");
    }

    /// The sensitivity control for the sweep above: a character the font does
    /// not have has to actually fail it. Without this the sweep passes for a
    /// `has_glyph` that returns `true` for everything.
    #[test]
    fn the_glyph_check_can_fail() {
        assert!(!crate::hud::has_glyph('*'), "the font gained a glyph this check relied on");
        assert!(!crate::hud::has_glyph('~'));
        assert!(crate::hud::has_glyph('X'));
    }

    #[test]
    fn the_readout_fits_the_viewport() {
        let mut t = TimeControl::new();
        t.set_preset(6);
        t.set_display_hz(10);
        t.achieved = 1234.5;
        t.shown_ticks = 65535;
        for (line, _) in t.readout(u32::MAX as u64) {
            let w = crate::hud::text_width(&line) + 4;
            assert!(w <= super::super::WIDTH as i32, "{line:?} is {w}px wide");
        }
    }

    #[test]
    fn presets_keep_the_number_rows_meaning() {
        // `bin/lab.rs` binds digits 1-6 to indices 0-5; those meanings are not
        // this file's to reshuffle.
        assert_eq!(&PRESETS[..6], &[1, 2, 4, 16, 64, 256]);
        let mut t = TimeControl::new();
        t.set_preset(0);
        assert_eq!(t.phase, Phase::Tending);
        for _ in 0..20 {
            t.faster();
        }
        assert_eq!(t.requested, *PRESETS.last().unwrap());
        assert_eq!(t.phase, Phase::Running);
        for _ in 0..20 {
            t.slower();
        }
        assert_eq!(t.requested, 1);
        assert_eq!(t.phase, Phase::Tending);
    }

    #[test]
    fn the_display_rate_cycles_and_clamps() {
        let mut t = TimeControl::new();
        assert_eq!(t.display_hz(), 60);
        for expected in [30, 20, 10, 60] {
            t.cycle_display_rate();
            assert_eq!(t.display_hz(), expected);
        }
        t.set_display_hz(0);
        assert!(t.display_hz() >= 1, "a zero display rate would stall the loop");
        assert!(!t.display_interval().is_zero());
    }

    #[test]
    fn sim_per_second_reads_in_world_time() {
        assert_eq!(sim_per_second(1.0), "1S");
        assert_eq!(sim_per_second(64.0), "1M 4S");
        assert_eq!(sim_per_second(3600.0), "1H 0M");
        assert_eq!(sim_per_second(0.4), "0.4S");
    }
}
