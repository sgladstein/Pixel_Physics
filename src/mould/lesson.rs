//! **The first dish teaches the game, one tool at a time.**
//!
//! Owner's playtest, 2026-11-08: "The game needs better instruction of what
//! you're supposed to do. What your actual options are ... I didn't really
//! know what I was doing ... I have no idea what the salt or the snip did."
//! So `cargo run --bin mould` now opens on this lesson: an ordinary dish with
//! the storm held back, and a banner over the dish that names one thing to
//! do, why, and moves on when the dish shows it was done. Every step is
//! judged from the dish's own counters, never from a key press, so a step
//! passes because the mould did the thing, not because a button was hit.
//!
//! The last step lets the storm come, with its warning, and the dish is
//! scored like any other. The probe (`mouldprobe lesson=1`) plays it with a
//! scripted hand to show every step can be passed.

use super::{Dish, STEPS_PER_SECOND};

/// One step of the lesson: the banner's two lines, and what passes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Lamp,
    Oats,
    Join,
    Snip,
    Salt,
    Storm,
    Done,
}

/// Seconds every step stays up at least, so its banner can be read even
/// when the dish passes it at once (on the first scripted run, the mould
/// joined three piles 1.6 s into the join step, before anyone could read
/// what it asked).
pub const MIN_STEP_SECONDS: u32 = 4;
/// The dish seed the lesson is played on (a layout with three piles within
/// easy reach of home).
pub const LESSON_SEED: u64 = 1;
/// Seconds of storm warning the lesson gives: longer than a real dish's,
/// so there is time to act on it the first time.
pub const LESSON_STORM_SECONDS: u32 = 20;

impl Step {
    pub fn number(self) -> u32 {
        self as u32 + 1
    }
    pub const COUNT: u32 = 6;
    /// The banner: about 40 characters a line, so it reads at half size too.
    pub fn lines(self) -> [&'static str; 2] {
        match self {
            Step::Lamp => ["HOLD THE MOUSE NEAR THE MOULD.", "THE LAMP'S LIGHT PUSHES IT AWAY."],
            Step::Oats => [
                "PRESS 2. CLICK OATS BETWEEN THE MOULD",
                "AND A FOOD PILE. IT GROWS TO THEM.",
            ],
            Step::Join => ["NOW JOIN ALL THE FOOD. MORE OATS", "COME BACK, ONE EVERY 15 S."],
            Step::Snip => [
                "PRESS 4 AND DRAG ACROSS A TUBE.",
                "CUT-OFF MOULD GOES GREY AND STARVES.",
            ],
            Step::Salt => ["PRESS 3 AND DRAG A SALT WALL.", "THE MOULD CANNOT CROSS SALT."],
            Step::Storm => ["A STORM WILL CUT THE PURPLE LINE.", "GROW A SECOND WAY ROUND IT."],
            Step::Done => [
                "THAT'S THE GAME: JOIN ALL THE FOOD,",
                "SHORT, AND STORM-PROOF. N: DISH 1",
            ],
        }
    }
}

/// The lesson's progress over one dish.
#[derive(Clone, Debug)]
pub struct Lesson {
    pub step: Step,
    /// When the current step began, for the hand's "it moved on" flash.
    pub since: u64,
    lamp_at_start: u64,
    oats_at_start: u32,
    snips_at_start: u32,
    salt_at_start: u32,
}

impl Lesson {
    /// The lesson's dish: the storm held back until the last step, and no
    /// late food (one new thing at a time).
    pub fn dish() -> (Dish, Lesson) {
        let mut d = Dish::new(LESSON_SEED);
        d.storm_at = u64::MAX;
        d.pending.clear();
        let l = Lesson {
            step: Step::Lamp,
            since: 0,
            lamp_at_start: 0,
            oats_at_start: 0,
            snips_at_start: 0,
            salt_at_start: 0,
        };
        (d, l)
    }

    fn next(&mut self, d: &mut Dish, step: Step) {
        self.step = step;
        self.since = d.t;
        let c = &d.counts;
        self.lamp_at_start = c.lamp_steps;
        self.oats_at_start = c.oats_placed;
        self.snips_at_start = c.snips_used;
        self.salt_at_start = c.salt_cells;
        if step == Step::Storm {
            d.storm_at = d.t + (LESSON_STORM_SECONDS * STEPS_PER_SECOND) as u64;
            // Aimed and shown at once: the banner points at the line.
            d.storm_warning = (LESSON_STORM_SECONDS * STEPS_PER_SECOND) as u64;
        }
    }

    /// Call once per drawn frame, after the dish has stepped.
    pub fn update(&mut self, d: &mut Dish) {
        // A step stays up long enough to read, so one never flashes past.
        if d.t < self.since + (MIN_STEP_SECONDS * STEPS_PER_SECOND) as u64 {
            return;
        }
        let c = d.counts.clone();
        let all_joined = || {
            let s = d.measure();
            s.reach == s.piles
        };
        // The oats step passes when the mould is eating oats the player put
        // down, not merely when two piles are joined: the lesson's home blob
        // touches a second pile on its own within a few seconds.
        let on_oats = d.piles.iter().any(|p| p.oats && p.reached());
        let next = match self.step {
            Step::Lamp if c.lamp_steps >= self.lamp_at_start + 2 * STEPS_PER_SECOND as u64 => Some(Step::Oats),
            Step::Oats if c.oats_placed > self.oats_at_start && on_oats => Some(Step::Join),
            Step::Join if all_joined() => Some(Step::Snip),
            Step::Snip if c.snips_used > self.snips_at_start => Some(Step::Salt),
            Step::Salt if c.salt_cells > self.salt_at_start => Some(Step::Storm),
            Step::Storm if d.score.is_some() => Some(Step::Done),
            _ => None,
        };
        if let Some(s) = next {
            self.next(d, s);
        }
    }

    /// A step the player has just passed flashes for this long.
    pub fn fresh(&self, d: &Dish) -> bool {
        self.step != Step::Lamp && d.t < self.since + STEPS_PER_SECOND as u64
    }
}
