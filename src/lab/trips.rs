//! **One ant's trips** -- the last of the five player readouts the owner
//! picked on 2026-10-03 ("All five in order"): pin an ant and the cell page
//! shows how far from its nest it has been over the last few minutes of box
//! time, each bar coloured by what it was carrying. Every dip to the floor is
//! a visit home, so a forager that commutes draws a sawtooth and one that
//! has stopped coming back draws a plateau.
//!
//! It is the shortlist mock's `loopchart.py` picture (one ant, time across,
//! distance from the nest up), moved into the game. The trail the box
//! already draws behind a pinned ant says *where*; this says *how often it
//! got home, and with what*.
//!
//! **Its own ring, not the trail's.** The trail samples every
//! `WATCH_EVERY` (12) frames into 128 samples, 1,536 frames of history,
//! which is shorter than one round trip on the played bed -- a strip over it
//! would almost never contain a dip. This one keeps [`TRIP_SAMPLES`] buckets
//! of [`TRIP_EVERY`] frames, 12,288 frames, the window the mock was drawn
//! over.
//!
//! **Home is latched every tick, not sampled.** A delivery can touch the nest
//! for a handful of frames, well under one bucket, and a bucket that only
//! looked at its last frame would draw that visit as an ant that never went
//! home -- the one event the strip exists to show. So `observe` asks
//! `creature::is_at_home` (the `AtNest` sense's own test, the definition the
//! BIOSPHERE activity bands use too) on every tick and a bucket is home if
//! the ant was home at any tick in it. That is one reach test for one
//! animal per tick, and only while something is pinned.

use std::collections::VecDeque;

use crate::sim::cell::OrganismId;
use crate::sim::world::World;

use super::stats::Activity;

/// Simulated frames per bucket. Eight trail samples.
pub const TRIP_EVERY: u64 = 96;

/// Buckets kept: 12,288 frames at [`TRIP_EVERY`].
pub const TRIP_SAMPLES: usize = 128;

/// One bucket of the strip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leg {
    /// Simulated frame the bucket closed on.
    pub frame: u64,
    /// Cells from the head to the nearest nest site, the larger of the two
    /// axes (the lifetrace's `nest_d`), at the bucket's close. Read as 0 when
    /// `home` is set, because the strip's floor means "touched home".
    pub dist: u16,
    /// Touched home at any tick in the bucket.
    pub home: bool,
    /// How full its crop was at the close, 0..1 of its own capacity, the
    /// `CropFill` sense's arithmetic. A storeroom load reads full.
    pub fill: f32,
    /// Holding dug soil at the close.
    pub soil: bool,
}

impl Leg {
    /// The bar height the strip draws.
    pub fn height(&self) -> u16 {
        if self.home {
            0
        } else {
            self.dist
        }
    }

    /// **The bar's colour, from the BIOSPHERE activity key** so the two
    /// readouts share one legend: home violet, soil brown, empty slate, and
    /// any food at all green -- from half the key's brightness for a crumb
    /// up to the key's own green for a full crop.
    ///
    /// **Any food is green, not a ramp from slate.** The ramp was the first
    /// build and drew the first rendered ant's four food trips as slate: the
    /// shipped ant comes home with its crop a small fraction full, so a
    /// slate-to-green ramp put every laden leg within a shade of an empty
    /// one and the strip's caption said 4 WITH FOOD over a picture with no
    /// green in it.
    pub fn colour(&self) -> [u8; 4] {
        if self.home {
            return Activity::Home.colour();
        }
        if self.soil {
            return Activity::Soil.colour();
        }
        if self.fill <= 0.0 {
            return Activity::Out.colour();
        }
        let food = Activity::Food.colour();
        let k = 0.5 + 0.5 * self.fill.clamp(0.0, 1.0);
        let dim = |i: usize| (food[i] as f32 * k).round() as u8;
        [dim(0), dim(1), dim(2), 255]
    }
}

/// The ring, for one individual. Owned by the cell page's `Watch`, which
/// already clears its own ring when the pin moves to somebody else, and
/// clears this one with it.
#[derive(Clone, Debug, Default)]
pub struct Trips {
    legs: VecDeque<Leg>,
    next_at: u64,
    /// Home seen at any tick since the last bucket closed.
    home_latch: bool,
}

impl Trips {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Called every simulated tick while `id` is pinned and alive.
    pub fn observe(&mut self, world: &World, id: OrganismId) {
        // A rebuild puts the frame counter back; a strip carried across it
        // would join two different worlds -- `Watch::observe`'s own guard.
        if self.legs.back().is_some_and(|l| world.frame < l.frame) {
            self.clear();
        }
        let Some(home) = crate::sim::creature::is_at_home(world, id) else {
            return;
        };
        self.home_latch |= home;
        if world.frame < self.next_at {
            return;
        }
        let Some(state) = world.organism(id) else { return };
        let Some(&(hx, hy)) = state.chain.first() else { return };
        let Some(site) = world.nearest_nest_site(hx, hy).map(|i| world.nest_sites[i]) else {
            return;
        };
        let dist = (site.x - hx)
            .abs()
            .max((site.surface - hy).abs())
            .clamp(0, u16::MAX as i32) as u16;
        let store = state.spoil.is_some_and(|sp| sp.store);
        let fill = if store {
            1.0
        } else {
            let def = world.species.get(state.species).creature.as_ref();
            state.crop.map_or(0.0, |c| {
                let cap = def.map_or(0.0, |d| crate::sim::creature::organism_crop_capacity(world, id, d));
                if cap > 0.0 {
                    (c.worth() / cap).clamp(0.0, 1.0)
                } else {
                    1.0
                }
            })
        };
        self.legs.push_back(Leg {
            frame: world.frame,
            dist,
            home: self.home_latch,
            fill,
            soil: state.spoil.is_some() && !store,
        });
        while self.legs.len() > TRIP_SAMPLES {
            self.legs.pop_front();
        }
        self.home_latch = false;
        self.next_at = world.frame + TRIP_EVERY;
    }

    pub fn legs(&self) -> impl Iterator<Item = &Leg> + '_ {
        self.legs.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.legs.is_empty()
    }

    /// Frames the strip covers.
    pub fn span(&self) -> u64 {
        match (self.legs.front(), self.legs.back()) {
            (Some(a), Some(b)) => b.frame.saturating_sub(a.frame),
            _ => 0,
        }
    }

    /// **Visits home, and how many came back with food.** A visit is a home
    /// bucket after an away one; a run of home buckets is one visit, and the
    /// first bucket counts only if something away came before it inside the
    /// window, so an ant pinned at home does not open on a trip it never
    /// made. "With food" reads the last away bucket before the dip: a crop
    /// with anything in it, or a storeroom load.
    pub fn visits(&self) -> (usize, usize) {
        let (mut trips, mut fed) = (0, 0);
        let mut prev: Option<&Leg> = None;
        for leg in &self.legs {
            if let Some(p) = prev {
                if leg.home && !p.home {
                    trips += 1;
                    if p.fill > 0.0 {
                        fed += 1;
                    }
                }
            }
            prev = Some(leg);
        }
        (trips, fed)
    }

    /// The farthest it got inside the window.
    pub fn farthest(&self) -> u16 {
        self.legs.iter().map(Leg::height).max().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leg(dist: u16, home: bool, fill: f32) -> Leg {
        Leg {
            frame: 0,
            dist,
            home,
            fill,
            soil: false,
        }
    }

    /// Two laden returns and an empty one, between stretches at home, read
    /// as three trips with two fed -- and a window that opens at home does
    /// not count that opening as a trip.
    #[test]
    fn visits_count_each_return_once_and_read_the_load_it_came_back_with() {
        let mut t = Trips::default();
        let seq = [
            leg(0, true, 0.0),
            leg(0, true, 0.0),
            leg(20, false, 0.0),
            leg(40, false, 0.6),
            leg(0, true, 0.0),
            leg(0, true, 0.0),
            leg(30, false, 0.0),
            leg(0, true, 0.0),
            leg(50, false, 0.2),
            leg(10, false, 1.0),
            leg(0, true, 0.0),
        ];
        t.legs.extend(seq);
        assert_eq!(t.visits(), (3, 2));
        assert_eq!(t.farthest(), 50);
    }

    /// Empty is the key's slate, a full crop the key's green, a crumb a
    /// green and never slate, home violet whatever it carries -- and none of
    /// it a function of the distance the bar is drawn at.
    #[test]
    fn colour_reads_the_load_on_the_activity_key() {
        let food = Activity::Food.colour();
        assert_eq!(leg(12, false, 0.0).colour(), Activity::Out.colour());
        assert_eq!(leg(99, false, 1.0).colour(), food);
        let crumb = leg(12, false, 0.05).colour();
        assert!(
            crumb[1] > crumb[0] && crumb[1] > crumb[2],
            "a crumb draws green: {crumb:?}"
        );
        assert_ne!(crumb, Activity::Out.colour());
        assert_eq!(leg(12, true, 1.0).colour(), Activity::Home.colour());
        assert_eq!(leg(12, false, 0.5).colour(), leg(80, false, 0.5).colour());
    }

    /// **The count is bracketed by what every ant really did, tick by tick,
    /// and the per-tick home latch is what keeps it there.**
    ///
    /// Every ant on the shipped bed is followed for 24,000 frames, and each
    /// time it gets home the length of the absence it is ending is recorded.
    /// A strip bucket is [`TRIP_EVERY`] frames, so two things are provable of
    /// a latched strip, per ant, over its own window: it counts **every**
    /// absence of two buckets or more (one whole bucket of it was away, and
    /// the return lands in a later one) and **nothing** that was not an
    /// absence of at least one bucket. Both bounds are asserted.
    ///
    /// **The positive control** is the same strip offered only the tick each
    /// bucket closes on -- the strip without the latch. It must break a bound
    /// on some ant: a stay home shorter than a bucket goes unseen and two
    /// trips merge (under the floor), or an ant hovering at the edge of home
    /// is counted in and out (over the ceiling). If it cannot, the latch
    /// costs a reach test per tick and buys nothing.
    #[test]
    fn the_trip_count_is_bracketed_by_what_each_ant_really_did() {
        use std::collections::HashMap;
        let mut world = super::super::scene::LabBox::default().build();
        let mut particles = crate::sim::particle::ParticleSystem::default();
        let mut blasts = crate::sim::explosion::Blasts::default();
        let tuning = crate::sim::player::Tuning::default();
        let mut strips: HashMap<OrganismId, (Trips, Trips)> = HashMap::new();
        // Per ant: whether it was home last tick, when its current absence
        // began, and every (absence began, got home) pair.
        let mut truth: HashMap<OrganismId, (bool, u64, Vec<(u64, u64)>)> = HashMap::new();
        for _ in 0..24_000 {
            crate::sim::frame::step(
                &mut world,
                &mut particles,
                &mut blasts,
                crate::sim::player::PlayerInput::default(),
                &tuning,
            );
            for id in world.live_organism_ids() {
                let Some(home) = crate::sim::creature::is_at_home(&world, id) else {
                    continue;
                };
                let t = truth.entry(id).or_insert((true, world.frame, Vec::new()));
                if home && !t.0 {
                    t.2.push((t.1, world.frame));
                } else if !home && t.0 {
                    t.1 = world.frame;
                }
                t.0 = home;
                let (every, closing) = strips.entry(id).or_default();
                every.observe(&world, id);
                if world.frame >= closing.next_at {
                    closing.observe(&world, id);
                }
            }
        }
        // `(floor, ceiling)` for one ant over the frames a strip covers.
        let bounds = |strip: &Trips, returns: &[(u64, u64)]| {
            let (Some(first), Some(last)) = (strip.legs.front(), strip.legs.back()) else {
                return (0, 0);
            };
            let inside = |&&(_, back): &&(u64, u64)| back > first.frame && back <= last.frame;
            let floor = returns
                .iter()
                .filter(inside)
                .filter(|(gone, back)| back - gone >= 2 * TRIP_EVERY && *gone > first.frame)
                .count();
            let ceiling = returns
                .iter()
                .filter(inside)
                .filter(|(gone, back)| back - gone >= TRIP_EVERY)
                .count();
            (floor, ceiling)
        };
        let (mut trips, mut fed, mut broken, mut control_broken) = (0, 0, Vec::new(), 0);
        for (id, (every, closing)) in &strips {
            let returns = &truth[id].2;
            let (t, f) = every.visits();
            trips += t;
            fed += f;
            let (lo, hi) = bounds(every, returns);
            if t < lo || t > hi {
                broken.push((*id, lo, t, hi));
            }
            let (clo, chi) = bounds(closing, returns);
            let c = closing.visits().0;
            if c < clo || c > chi {
                control_broken += 1;
            }
        }
        eprintln!(
            "ants {} trips {trips} with food {fed}; control out of bounds on {control_broken} ants",
            strips.len()
        );
        assert!(
            trips > 0 && fed > 0,
            "no ant got home, or none with food, in 24,000 frames: {trips} trips, {fed} fed"
        );
        assert!(
            broken.is_empty(),
            "(ant, floor, counted, ceiling) out of bounds: {broken:?}"
        );
        assert!(
            control_broken > 0,
            "the strip without the latch stayed in bounds on every ant: the latch catches nothing"
        );
    }
}
