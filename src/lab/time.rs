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
//! behaviour change wearing a speed control.
//!
//! **And why the loop is bounded by wall clock, not by tick count.** A box
//! whose stand has grown costs what it costs; a dial set to 100x on a bed
//! that can only do 14x must produce 14x and *say so*, never a frozen window
//! while it tries. So `plan` asks for a number of ticks and hands over a
//! ceiling, `record` reports what was achieved, and the readout on screen is
//! the achieved figure — the same one, from the same place, so the dial
//! cannot lie about what the box is doing.

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

/// What one displayed frame actually did.
#[derive(Clone, Copy, Debug, Default)]
pub struct Advance {
    pub ticks: u32,
    pub spent: Duration,
}

/// The dial's state.
pub struct TimeControl {
    pub phase: Phase,
    /// Requested multiple of real time. 1 is a second of simulation per
    /// second of wall clock.
    pub requested: u32,
    /// Achieved multiple, smoothed — what the readout shows.
    pub achieved: f32,
    /// Leftover simulated time not yet consumed by a tick.
    accumulator: Duration,
}

/// The simulation's own rate. One tick is 1/60th of a simulated second, on
/// both binaries and in every harness — it is the unit the whole engine's
/// cadences are written in.
pub const TICKS_PER_SECOND: u32 = 60;
pub const TICK: Duration = Duration::from_nanos(1_000_000_000 / TICKS_PER_SECOND as u64);

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
            accumulator: Duration::ZERO,
        }
    }

    /// How many ticks this displayed frame should try for, and how long it
    /// may take doing them.
    pub fn plan(&mut self, elapsed: Duration) -> Plan {
        let multiple = match self.phase {
            Phase::Tending => 1,
            Phase::Running => self.requested.max(1),
        };
        self.accumulator += elapsed.min(Duration::from_millis(250)) * multiple;
        let ticks = (self.accumulator.as_nanos() / TICK.as_nanos()) as u32;
        self.accumulator -= TICK * ticks;
        Plan { ticks, budget: Duration::from_millis(24) }
    }

    /// Record what the frame achieved, and return it.
    pub fn record(&mut self, ticks: u32, spent: Duration) -> Advance {
        // A frame that hit its ceiling has a backlog it will never work off,
        // so drop it rather than carrying a debt that makes the next frame
        // worse — the same reasoning `main.rs`'s catch-up loop already
        // states.
        if spent >= Duration::from_millis(24) {
            self.accumulator = Duration::ZERO;
        }
        let rate = if spent.as_secs_f32() > 0.0 {
            ticks as f32 / TICKS_PER_SECOND as f32 / spent.as_secs_f32()
        } else {
            self.requested as f32
        };
        self.achieved += (rate - self.achieved) * 0.1;
        Advance { ticks, spent }
    }

    /// Whether the frame must be fully redrawn because this module painted
    /// over the terrain last frame.
    pub fn hud_is_dirty(&self) -> bool {
        true
    }

    pub fn draw(&self, frame: &mut [u8], world: &crate::sim::world::World) {
        let label = match self.phase {
            Phase::Tending => "TENDING".to_string(),
            Phase::Running => format!("RUNNING {}X ({:.1}X)", self.requested, self.achieved.max(0.0)),
        };
        crate::hud::draw_text(
            frame,
            super::WIDTH,
            super::HEIGHT,
            4,
            4,
            &label,
            [235, 235, 235, 255],
        );
        crate::hud::draw_text(
            frame,
            super::WIDTH,
            super::HEIGHT,
            4,
            14,
            &format!("FRAME {}", world.frame),
            [150, 150, 150, 255],
        );
    }
}

/// The dial's stops.
///
/// **Presets rather than a free number**, because the interesting range is
/// multiplicative and a linear key-repeat would spend most of its travel in
/// the part nobody wants. The top of the ladder is deliberately past what any
/// box can achieve: the dial is a *request*, and a request the machine cannot
/// meet is how the readout earns its keep.
pub const PRESETS: [u32; 6] = [1, 2, 4, 16, 64, 256];

impl TimeControl {
    pub fn toggle_phase(&mut self) {
        self.phase = match self.phase {
            Phase::Tending => Phase::Running,
            Phase::Running => Phase::Tending,
        };
        self.accumulator = Duration::ZERO;
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
        self.accumulator = Duration::ZERO;
    }
}
