//! **How fast the lab's plants grow against its ants** -- the owner's
//! plant:ant speed dial (2026-10-03: plants "feel far too fast" next to the
//! ants), as a three-stop row on the BOX page.
//!
//! It drives one engine knob, `clock::Clock::growth_slowdown`, which
//! multiplies the organism tick interval: plant growth, seeding and litter
//! rot all run on that one cadence, so the plants' internal economy rescales
//! exactly and only how often a tick happens changes (`clock.rs` explains
//! why this is one number and not a rebalance). The ants' clock is not
//! touched.
//!
//! **Half is the default, and that is a measurement** (owner's card,
//! 2026-10-03, *"Half speed"*; the garden lane, `labgarden played_bed`, 6
//! seeds x 300k): at full speed every colony was dead by 222k; at half 5 of
//! 6 were alive at 300k; at quarter the colony never passed ~70 ants,
//! starved of falling litter and seed. Quarter is on the dial anyway so a
//! player can see that for themselves.
//!
//! Not the speed dial (`lab::time`), which multiplies *every* tick and is
//! exact; this changes how the two kingdoms run against each other, which
//! is a behaviour change by design.

/// One stop on the dial. On `LabBox` like `Rain`, so a saved box and a
/// scenario's `bed:` block carry it and a rebuild keeps it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PlantPace {
    Full,
    #[default]
    Half,
    Quarter,
}

impl PlantPace {
    /// `Clock::growth_slowdown` for this stop.
    pub fn slowdown(self) -> u32 {
        match self {
            PlantPace::Full => 1,
            PlantPace::Half => 2,
            PlantPace::Quarter => 4,
        }
    }

    /// The next stop, wrapping: full, half, quarter, full.
    pub fn next(self) -> Self {
        match self {
            PlantPace::Full => PlantPace::Half,
            PlantPace::Half => PlantPace::Quarter,
            PlantPace::Quarter => PlantPace::Full,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PlantPace::Full => "FULL SPEED",
            PlantPace::Half => "HALF SPEED",
            PlantPace::Quarter => "QUARTER SPEED",
        }
    }

    /// Put this stop on a live world, from now on. `Clock::set_rates`
    /// re-anchors the clock at `world.frame` first, so the sky and weather
    /// readers do not jump when the rate changes.
    pub fn apply(self, world: &mut crate::sim::world::World) {
        let frame = world.frame;
        world.clock.set_rates(frame, |c| c.growth_slowdown = self.slowdown());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The dial reaches every stop and comes back, and each stop lands on
    /// the clock as the slowdown it names -- through `set_rates`' clamp,
    /// which would quietly pin a stop it does not allow.
    #[test]
    fn every_stop_reaches_the_clock() {
        let mut world = crate::sim::world::World::new(crate::sim::chunk::Rect::new(0, 0, 15, 15));
        let mut pace = PlantPace::default();
        assert_eq!(pace, PlantPace::Half, "the owner picked half speed as the default");
        for _ in 0..3 {
            pace.apply(&mut world);
            assert_eq!(world.clock.growth_slowdown, pace.slowdown(), "{pace:?} did not reach the clock");
            pace = pace.next();
        }
        assert_eq!(pace, PlantPace::Half, "three presses go all the way round");
    }
}
