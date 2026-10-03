//! **The lab bench's per-run tally: where eggs were laid, how many ants are
//! underground, which colonies were lost and how far the box fell from its
//! peak.**
//!
//! Built 2026-10-03 for the lab-first bench (`scripts/labbench.py`). The
//! owner's ruling that set its scope: the simple beds (food box, trailfollow,
//! labnest) stay where problems are solved, and the lab is **the final check
//! that a fix also holds in the game**. So this answers the questions the
//! laying and nest lanes were each answering their own way on the lab box,
//! in one place, so two lanes' numbers can be read side by side.
//!
//! **Read-only and outside the engine.** Nothing here is called by
//! `frame::step`; a harness calls [`Bench::observe`] once per frame after
//! stepping, and it does its own cadence. It never writes to the world, so a
//! run with it is byte-identical to a run without it.
//!
//! **Eggs are counted by sighting, not at laying**, so that this needs no
//! counter inside `brood.rs` (the laying lane's file). Every
//! [`EGG_EVERY`] frames each live brood organism not seen before is
//! classified by the cell it sits on. An egg that is laid and lost inside one
//! interval is never seen, which is why the line prints `seen` beside the
//! engine's own `eggs_laid` -- the coverage is the positive control, and a
//! coverage well under 1 means the classification is of a biased sample.
//! An egg is a powder and can fall a few cells before it is seen; "at home"
//! below is a ring of 1 around the home cells for that reason.

use crate::sim::cell::OrganismId;
use crate::sim::fxhash::FxHashSet;
use crate::sim::world::World;

/// Frames between egg sightings. An egg stage outlasts this by far, so an
/// egg is missed only if it is lost (eaten, crushed) inside one interval.
pub const EGG_EVERY: u64 = 30;
/// Frames between population samples (underground share, peak, fall).
pub const POP_EVERY: u64 = 300;
/// A running peak below this is too small for a fall from it to mean a
/// crash rather than a handful of deaths -- the same bar the long-run
/// steadiness readout used (`stab.py`, peak >= 20).
pub const CRASH_MIN_PEAK: u32 = 20;
/// A group that never held this many animals is not counted as a colony
/// that was lost -- one stray beetle dying is not a colony dying.
/// `World`'s own `MIN_SPLIT_GROUP` is the same idea.
pub const COLONY_MIN: u32 = 3;
/// Column bands for "how far from the nearest nest site was the egg".
pub const EGG_BANDS: [i32; 2] = [8, 32];

#[derive(Clone, Debug, Default)]
pub struct Bench {
    ground_y: i32,
    seen: FxHashSet<(OrganismId, u64)>,
    /// Eggs first seen on or beside a home cell (dug home or nest material).
    pub eggs_home: u64,
    /// Eggs first seen by column distance from the nearest nest site:
    /// `<= EGG_BANDS[0]`, `<= EGG_BANDS[1]`, further (or no site at all).
    pub eggs_by_dist: [u64; 3],
    pub eggs_seen: u64,
    /// Sum over population samples of ants with the head below the original
    /// ground row, and of all live animals, so the share is a time average.
    under_sum: u64,
    animal_sum: u64,
    pub under_last: u32,
    pub animals_last: u32,
    pub peak: u32,
    pub peak_frame: u64,
    running_peak: u32,
    /// Deepest fall below the running peak, as a fraction of it, counted only
    /// while that peak is at least [`CRASH_MIN_PEAK`].
    pub worst_fall: f32,
    /// Times the count fell to half its running peak (peak >= `CRASH_MIN_PEAK`);
    /// the running peak restarts from the low after each one.
    pub halvings: u32,
    /// `(species, colony) -> most animals the group ever held`.
    colony_peak: std::collections::BTreeMap<(u16, u32), u32>,
    pub colonies_now: u32,
    /// The groups alive at the last sample.
    live_now: std::collections::BTreeSet<(u16, u32)>,
}

impl Bench {
    /// `ground_y` is the bed's original surface row; "underground" is a head
    /// strictly below it.
    pub fn new(ground_y: i32) -> Self {
        Bench {
            ground_y,
            ..Default::default()
        }
    }

    /// Call once per frame, after the step. Does nothing off its cadence.
    pub fn observe(&mut self, world: &World) {
        let f = world.frame;
        if f.is_multiple_of(EGG_EVERY) {
            self.sight_eggs(world);
        }
        if f.is_multiple_of(POP_EVERY) {
            self.sample_population(world);
        }
    }

    fn sight_eggs(&mut self, world: &World) {
        for id in world.live_brood_ids() {
            let Some(state) = world.organism(id) else { continue };
            if !self.seen.insert((id, state.born_frame)) {
                continue;
            }
            let Some(&(x, y)) = state.cells.keys().min_by_key(|&&(x, y)| (y, x)) else {
                continue;
            };
            self.eggs_seen += 1;
            let nest = world
                .species
                .get(state.species)
                .creature
                .as_ref()
                .and_then(|d| world.materials.id_of(&d.nest));
            let home = (-1..=1).any(|dy| {
                (-1..=1).any(|dx| {
                    let (cx, cy) = (x + dx, y + dy);
                    world.nest_dug.contains(&(cx, cy)) || nest.is_some_and(|m| world.get(cx, cy).material == m)
                })
            });
            if home {
                self.eggs_home += 1;
            }
            let d = world
                .nest_sites
                .iter()
                .map(|s| (s.x - x).abs())
                .min()
                .unwrap_or(i32::MAX);
            let band = EGG_BANDS.iter().position(|&edge| d <= edge).unwrap_or(EGG_BANDS.len());
            self.eggs_by_dist[band] += 1;
        }
    }

    fn sample_population(&mut self, world: &World) {
        let mut under = 0u32;
        let mut animals = 0u32;
        for id in world.live_organism_ids() {
            let Some(state) = world.organism(id) else { continue };
            if world.species.get(state.species).creature.is_none() {
                continue;
            }
            animals += 1;
            if state.chain.first().is_some_and(|&(_, hy)| hy > self.ground_y) {
                under += 1;
            }
        }
        self.under_sum += u64::from(under);
        self.animal_sum += u64::from(animals);
        self.under_last = under;
        self.animals_last = animals;
        if animals > self.peak {
            self.peak = animals;
            self.peak_frame = world.frame;
        }
        self.running_peak = self.running_peak.max(animals);
        if self.running_peak >= CRASH_MIN_PEAK {
            let fall = 1.0 - animals as f32 / self.running_peak as f32;
            self.worst_fall = self.worst_fall.max(fall);
            if animals * 2 <= self.running_peak {
                self.halvings += 1;
                self.running_peak = animals;
            }
        }
        // A group missing from this sample's list holds nobody now; its
        // peak stays recorded, and `colonies_lost` reads it back.
        self.live_now.clear();
        for g in world.live_creature_groups() {
            let e = self.colony_peak.entry((g.species.0, g.colony)).or_insert(0);
            *e = (*e).max(g.alive);
            self.live_now.insert((g.species.0, g.colony));
        }
        self.colonies_now = self.live_now.len() as u32;
    }

    /// Groups that once held at least [`COLONY_MIN`] animals and hold none now.
    pub fn colonies_lost(&self) -> u32 {
        self.colony_peak
            .iter()
            .filter(|(k, &p)| p >= COLONY_MIN && !self.live_now.contains(k))
            .count() as u32
    }

    /// Groups that ever held at least [`COLONY_MIN`] animals.
    pub fn colonies_ever(&self) -> u32 {
        self.colony_peak.values().filter(|&&p| p >= COLONY_MIN).count() as u32
    }

    /// Time-averaged share of live animals with the head underground, 0..1.
    pub fn under_share(&self) -> f32 {
        if self.animal_sum == 0 {
            0.0
        } else {
            self.under_sum as f32 / self.animal_sum as f32
        }
    }

    /// The one line a harness prints at the end: `BENCH key=value ...`, read
    /// by `scripts/labpair.py`. `eggs_laid` is the engine's own counter,
    /// passed in so the coverage of the sighting count sits beside it.
    pub fn line(&self, eggs_laid: u64) -> String {
        let pct = |n: u64| {
            if self.eggs_seen == 0 {
                0.0
            } else {
                100.0 * n as f64 / self.eggs_seen as f64
            }
        };
        format!(
            "BENCH eggs_laid={eggs_laid} eggs_seen={} eggs_home={} eggs_home_pct={:.1} eggs_within8={} eggs_within32={} eggs_beyond32={} \
             under_now={} under_pct={:.1} animals_now={} peak={} peak_frame={} worst_fall_pct={:.1} halvings={} \
             colonies_ever={} colonies_now={} colonies_lost={}",
            self.eggs_seen,
            self.eggs_home,
            pct(self.eggs_home),
            self.eggs_by_dist[0],
            self.eggs_by_dist[1],
            self.eggs_by_dist[2],
            self.under_last,
            100.0 * self.under_share(),
            self.animals_last,
            self.peak,
            self.peak_frame,
            100.0 * self.worst_fall,
            self.halvings,
            self.colonies_ever(),
            self.colonies_now,
            self.colonies_lost()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::scene::LabBox;

    /// Both directions on a real box: an empty bed reads nothing, and a bed
    /// with its shipped colony reads that colony, mostly above ground.
    #[test]
    fn the_tally_reads_zero_on_an_empty_bed_and_the_colony_on_a_stocked_one() {
        let empty = LabBox {
            founders: 0,
            colonies: 0,
            ..LabBox::default()
        };
        let w = empty.build();
        let mut b = Bench::new(empty.ground_y);
        b.observe(&w);
        assert_eq!((b.animals_last, b.peak, b.colonies_ever(), b.eggs_seen), (0, 0, 0, 0));

        let stocked = LabBox {
            founders: 0,
            ..LabBox::default()
        };
        let w = stocked.build();
        let mut b = Bench::new(stocked.ground_y);
        b.observe(&w);
        assert_eq!(b.animals_last as usize, w.live_creature_count());
        assert!(
            b.animals_last >= COLONY_MIN,
            "the shipped colony must be seen; read {}",
            b.animals_last
        );
        assert_eq!((b.colonies_ever(), b.colonies_now, b.colonies_lost()), (1, 1, 0));
        // Founders are seated on the surface; the odd one can land in the
        // founding cut, which is below the ground row and counts.
        assert!(
            b.under_last * 4 < b.animals_last,
            "most founders stand on the surface; read {} of {} under",
            b.under_last,
            b.animals_last
        );
        assert!(b.line(0).starts_with("BENCH eggs_laid=0 eggs_seen=0 "));
    }
}
