//! **The stats page — what the box is doing, in numbers.**
//!
//! §8.9 of the design guide is the uncomfortable one this answers:
//! *"What does the player watch during a Running phase?"* Measured by
//! `creature_look` and `motion_look`, an ant is **two dark cells at play
//! zoom, findable only because it moves** — and a dead one has stopped
//! moving, so it is unfindable by the very channel that finds a live one. A
//! phase whose whole content is *watch evolution happen* has a legibility
//! problem this repo has already measured and not solved, and a page of
//! numbers beside the box is the cheapest half of the answer.
//!
//! **Read the count next to the picture, never the picture alone** —
//! `CLAUDE.md`'s standing rule, learned when a collapse rendered as coherent
//! falling slabs was read as *"chunks are working"* while the harness's own
//! body count was zero for the whole run. The same trap is live here: a box
//! full of green that is not reproducing looks exactly like a box full of
//! green that is.

/// The census, and the page that draws it.
pub struct Stats {
    show: bool,
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}

impl Stats {
    pub fn new() -> Self {
        Self { show: true }
    }

    pub fn showing(&self) -> bool {
        self.show
    }

    pub fn toggle(&mut self) {
        self.show = !self.show;
    }

    /// Called once per displayed frame, after the ticks have run.
    pub fn observe(&mut self, _world: &crate::sim::world::World) {}

    pub fn draw(&self, frame: &mut [u8], world: &crate::sim::world::World) {
        if !self.show {
            return;
        }
        let orgs = world.live_organism_count();
        crate::hud::draw_text(
            frame,
            super::WIDTH,
            super::HEIGHT,
            super::WIDTH as i32 - 90,
            4,
            &format!("ORGS {orgs}"),
            [200, 220, 200, 255],
        );
    }
}
