//! **The dig heat map: where the colony has been cutting, and how lately.**
//!
//! The owner, after the 2026-10-07 playtests: *"Digging in the mound is
//! obviously a problem to solve"*, and *"would any of the visual tools that we
//! have developed to analyze digging be useful for me to see during
//! playtests"*. `deeptrace`'s `cuts.csv` answers who cut where, but only after
//! a headless run and a reader; this draws the same fact live, so a player can
//! watch the mound being churned while it happens.
//!
//! **One source, read only.** Every cut an animal makes is pushed onto
//! `World::cut_log` beside `World::dug_cells`, in the one place a cut is made
//! (`creature::act`'s dig arm), and only while this map is on (`Some`).
//! Recording draws nothing and changes nothing, so a box with the map on runs
//! the same game as one with it off. [`DigMarks::observe`] drains it once per
//! simulated tick from the lab's tick loop -- the food road's reason
//! (`crate::food_road`): read once per displayed frame, a fast box would lose
//! the order of its cuts, and draw-time observation would read another
//! chamber's world into this one.
//!
//! **Drawn as a full replace on a fixed ramp**, bright for a cut made just now
//! fading to dark over [`DigMarks::window`] frames, and never a blend into the
//! cell's own colour: `CLAUDE.md`'s rule for a debug readout. A cut cell is
//! empty, so the mark sits in the hole the ant left; a hole refilled since is
//! drawn as the ground that filled it, because a map of holes that are no
//! longer there would be a picture of the past dressed as the present.
//!
//! **Free while off**: `observe` returns on one enum compare after making
//! sure the world is not recording, and the map is emptied.

use std::collections::HashMap;

use crate::sim::cell::OrganismId;
use crate::sim::world::World;

/// Whether the dig heat map is drawn. `Off` by default: a look ships
/// default-off pending the owner's eye (the food road's rule).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DigOverlay {
    #[default]
    Off,
    /// Every cut of the last [`DigMarks::window`] frames, newest brightest.
    Cuts,
}

impl DigOverlay {
    pub fn next(self) -> Self {
        match self {
            DigOverlay::Off => DigOverlay::Cuts,
            DigOverlay::Cuts => DigOverlay::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DigOverlay::Off => "OFF",
            DigOverlay::Cuts => "RECENT DIGGING (WHITE JUST CUT, FADING TO DARK TEAL)",
        }
    }
}

/// The ramp a cut fades along, newest first: white, cyan, teal, a dark teal
/// that still stands off soil. **Teal, not the fire ramp it first had**: the
/// first sheet drew yellow-orange cuts under amber foragers (`BY JOB`) and
/// the two could not be told apart, so the dig map takes a hue no ant colour
/// uses and the bed does not have.
const RAMP: [[f32; 3]; 4] = [
    [255.0, 255.0, 255.0],
    [60.0, 235.0, 230.0],
    [0.0, 160.0, 150.0],
    [0.0, 80.0, 80.0],
];

/// Colour of a cut `age` frames old in a window of `window` frames: `t` runs
/// 0 (just cut) to 1 (about to drop off the map).
pub fn ramp(age: u64, window: u64) -> [f32; 3] {
    let t = (age as f32 / window.max(1) as f32).clamp(0.0, 1.0) * (RAMP.len() - 1) as f32;
    let i = (t as usize).min(RAMP.len() - 2);
    let f = t - i as f32;
    let (a, b) = (RAMP[i], RAMP[i + 1]);
    [
        a[0] + (b[0] - a[0]) * f,
        a[1] + (b[1] - a[1]) * f,
        a[2] + (b[2] - a[2]) * f,
    ]
}

/// The map itself: the newest cut at each cell, and who made it.
#[derive(Debug, Default)]
pub struct DigMarks {
    pub mode: DigOverlay,
    /// Frames a cut stays on the map. 20,000 is a few minutes at 64x, long
    /// enough to see where the colony has been working, short enough that
    /// the mound's churn reads as current rather than as the whole history.
    pub window: u64,
    cuts: HashMap<(i32, i32), (u64, OrganismId)>,
    /// The last frame and bounds observed -- [`DigMarks::describes`].
    last_frame: Option<u64>,
    last_bounds: Option<crate::sim::chunk::Rect>,
    /// Cuts drained since the map was turned on, for the on-screen count.
    pub total: u64,
}

impl DigMarks {
    pub fn new() -> Self {
        DigMarks {
            window: 20_000,
            ..Default::default()
        }
    }

    /// Forget everything (the world changed under the map, or it was turned
    /// off).
    pub fn forget(&mut self) {
        self.cuts.clear();
        self.last_frame = None;
        self.last_bounds = None;
        self.total = 0;
    }

    /// Take this tick's cuts off the world. Turns the world's recording on
    /// while the map is on and off while it is off, so the log costs nothing
    /// when nobody is looking at it.
    pub fn observe(&mut self, world: &mut World) {
        if self.mode == DigOverlay::Off {
            world.cut_log = None;
            if !self.cuts.is_empty() || self.last_frame.is_some() {
                self.forget();
            }
            return;
        }
        // A different world (a rebuild, a rack swap): its cells are not this
        // map's cells.
        if self.last_bounds.is_some() && self.last_bounds != world.bounds()
            || self.last_frame.is_some_and(|f| world.frame < f)
        {
            self.forget();
        }
        let log = world.cut_log.get_or_insert_with(Vec::new);
        for (x, y, frame, id) in log.drain(..) {
            self.cuts.insert((x, y), (frame, id));
            self.total += 1;
        }
        let now = world.frame;
        let window = self.window;
        // Pruned on the census cadence, not every tick: the map is a few
        // thousand cells at most and a stale entry is never drawn anyway
        // (`mark_at` checks the age).
        if now.is_multiple_of(1000) {
            self.cuts.retain(|_, (f, _)| now.saturating_sub(*f) < window);
        }
        self.last_frame = Some(now);
        self.last_bounds = world.bounds();
    }

    pub fn on(&self) -> bool {
        self.mode != DigOverlay::Off
    }

    /// **Is this the world the map was built from?** The food road's guard,
    /// so a chamber thumbnail is not painted with the active box's cuts.
    pub fn describes(&self, world: &World) -> bool {
        self.on() && self.last_frame == Some(world.frame) && self.last_bounds == world.bounds()
    }

    /// The colour to draw at `(x, y)` now, if a cut there is still on the map.
    /// The caller passes whether the cell is empty: a hole refilled since is
    /// not drawn (module doc).
    pub fn mark_at(&self, x: i32, y: i32, now: u64, empty: bool) -> Option<[f32; 3]> {
        if !empty {
            return None;
        }
        let &(frame, _) = self.cuts.get(&(x, y))?;
        let age = now.saturating_sub(frame);
        (age < self.window).then(|| ramp(age, self.window))
    }

    /// Cuts still on the map, and how many distinct animals made them -- the
    /// discrete count `CLAUDE.md` asks to sit beside any picture of an event.
    pub fn readout(&self, now: u64) -> (usize, usize) {
        let live: Vec<_> = self
            .cuts
            .values()
            .filter(|(f, _)| now.saturating_sub(*f) < self.window)
            .collect();
        let mut who: Vec<OrganismId> = live.iter().map(|(_, id)| *id).collect();
        who.sort_unstable();
        who.dedup();
        (live.len(), who.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ramp only ever darkens with age, so "brighter" always means "more
    /// recent"; and a cut past the window is not drawn.
    #[test]
    fn the_ramp_darkens_with_age_and_the_window_ends_it() {
        let luma = |c: [f32; 3]| c[0] * 0.299 + c[1] * 0.587 + c[2] * 0.114;
        let mut last = f32::MAX;
        for age in (0..=20_000).step_by(500) {
            let l = luma(ramp(age, 20_000));
            assert!(l <= last, "the ramp brightens at age {age}");
            last = l;
        }
        let mut m = DigMarks::new();
        m.mode = DigOverlay::Cuts;
        m.cuts.insert((3, 4), (100, OrganismId::default()));
        assert!(m.mark_at(3, 4, 100, true).is_some(), "a cut made now is drawn");
        assert!(m.mark_at(3, 4, 100, false).is_none(), "a refilled hole is not drawn");
        assert!(
            m.mark_at(3, 4, 100 + m.window, true).is_none(),
            "a cut past the window is not drawn"
        );
        assert_eq!(m.readout(100), (1, 1));
    }
}
