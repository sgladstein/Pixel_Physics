//! **Whose mind an egg carries: a colony's store of fathers** (seat demos,
//! 2026-10-25; the shared "who lays" piece, owned by the queen seat and
//! driven by the colony mind seat too).
//!
//! Today an egg carries its layer's mind (`mind::inherit` at `try_bud`), so a
//! colony founded by one queen is one mind, and "choosing who lays" has
//! nothing to choose between (the queen plan's review, must-change 1). A
//! real queen stores sperm from several fathers and draws on it egg by egg.
//! This is that store, per colony:
//!
//! - **fathers**, each a mind with the flat genome it was built from and a
//!   weight; an egg draws one by weight with chance `p`, else keeps its
//!   layer's mind (today's rule);
//! - **a queue**, spent one per egg ahead of the weighted draw: the queen
//!   seat's "my next five eggs carry *her* line";
//! - **a wild level**, spent by the next egg: its mind is a mutated copy of
//!   the one it would have carried (the queen seat's gamble). The mutation is
//!   the caller's function ([`Fathers::mutate`]), so `sim` never reaches
//!   into `lab` or `train`.
//!
//! `queen_only`: the queue, the wild level *and* the draw apply only to eggs
//! of the steered queen (`steer`), so workers laying while her hold lapses
//! lay their own lines. Off it, every egg of the colony draws (the colony
//! mind seat's weighting).
//!
//! **Off is off.** `World::fathers` is `None` unless a demo fills it; the
//! hook at `try_bud` is then one `Option` read. Every draw is keyed
//! (`rng::stream(seed, child, frame, RNG_SLOT_FATHER)`), never the shared
//! `World::rng`, so turning a store on moves no other system's dice.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::sim::cell::OrganismId;
use crate::sim::creature::mind::{self, MindGenome};
use crate::sim::world::World;

/// The keyed stream's slot for the father draw (apart from `creature`'s
/// birth slots, which key on the same child and frame).
const RNG_SLOT_FATHER: u64 = 0xFA7E;

/// One father: a mind, the flat genome it was built from (what a wild copy
/// mutates), and its weight in the draw.
#[derive(Clone, Debug)]
pub struct Father {
    pub mind: Arc<MindGenome>,
    pub genome: Arc<Vec<f32>>,
    pub weight: f32,
}

/// A colony's store.
#[derive(Clone, Debug, Default)]
pub struct Store {
    pub fathers: Vec<Father>,
    /// The chance an egg draws from `fathers` at all; 0 is today's rule.
    pub p: f32,
    /// Spent one per egg before the draw.
    pub queue: std::collections::VecDeque<Father>,
    /// Spent by the next egg: 0 none, else a level the mutator reads.
    pub wild: u8,
    /// Only the steered queen's eggs use the store.
    pub queen_only: bool,
}

/// What an egg got from the store, for a demo's notices and counters.
#[derive(Clone, Debug)]
pub struct Fathered {
    pub child: OrganismId,
    pub layer: OrganismId,
    pub colony: u32,
    pub frame: u64,
    /// The father's mind (the line), after any wild mutation.
    pub mind: Arc<MindGenome>,
    /// The flat genome of `mind`.
    pub genome: Arc<Vec<f32>>,
    /// From the queue rather than the draw.
    pub queued: bool,
    /// The wild level spent on this egg (0 none).
    pub wild: u8,
}

/// Builds a mind from a flat genome (the lab's `colony_bed::mind`).
pub type Build = fn(&[f32]) -> Arc<MindGenome>;
/// Mutates a flat genome at a wild level, from a keyed seed.
pub type Mutate = fn(&[f32], u8, u64) -> Vec<f32>;

/// Every colony's store, and the functions that turn genomes into minds.
#[derive(Clone, Debug)]
pub struct Fathers {
    pub colonies: BTreeMap<u32, Store>,
    pub build: Build,
    pub mutate: Mutate,
    /// Every egg the store fathered, oldest first; a demo drains it.
    pub log: Vec<Fathered>,
    /// The flat genome of each mind the store has handed out, by the mind's
    /// address (the store holds a strong ref, so an address is never reused):
    /// how a wild copy of a mind the store did not draw (the layer's own)
    /// finds what to mutate.
    pub genomes: Vec<(Arc<MindGenome>, Arc<Vec<f32>>)>,
}

impl Fathers {
    pub fn new(build: Build, mutate: Mutate) -> Fathers {
        Fathers { colonies: BTreeMap::new(), build, mutate, log: Vec::new(), genomes: Vec::new() }
    }

    /// The flat genome a mind was built from, if the store knows it.
    pub fn genome_of(&self, m: &Arc<MindGenome>) -> Option<Arc<Vec<f32>>> {
        self.genomes.iter().find(|(a, _)| Arc::ptr_eq(a, m)).map(|(_, g)| g.clone())
    }

    /// Remember a mind and its genome (idempotent).
    pub fn know(&mut self, m: &Arc<MindGenome>, g: &Arc<Vec<f32>>) {
        if self.genome_of(m).is_none() {
            self.genomes.push((m.clone(), g.clone()));
        }
    }
}

/// **The hook, at `try_bud` after `mind::inherit`**: overwrite `child`'s mind
/// from its colony's store if the store says so. Nothing when no store exists
/// for the layer's colony, when the layer has no mind, or (with `queen_only`)
/// when the layer is not the steered queen.
pub fn on_egg(world: &mut World, layer: OrganismId, child: OrganismId) {
    let Some(colony) = world.fathers.as_ref().and_then(|_| world.organism(layer)).map(|s| s.colony) else { return };
    if !mind::has(world, layer) {
        return;
    }
    let is_queen = crate::sim::steer::is_steered(world, layer) && world.steered.as_ref().is_some_and(|s| s.role == crate::sim::steer::SteerRole::Queen);
    let (seed, frame) = (world.seed, world.frame);
    let layer_mind = mind::genome_of(world, layer).expect("has a mind");
    let fathers = world.fathers.as_mut().expect("read above");
    let Some(store) = fathers.colonies.get_mut(&colony) else { return };
    if store.queen_only && !is_queen {
        return;
    }
    let mut rng = crate::sim::rng::stream(seed, u64::from(child), frame, RNG_SLOT_FATHER);
    let mut queued = false;
    let mut pick: Option<Father> = None;
    if let Some(f) = store.queue.pop_front() {
        pick = Some(f);
        queued = true;
    } else if store.p > 0.0 && rng.unit_f32() < store.p {
        let total: f32 = store.fathers.iter().map(|f| f.weight.max(0.0)).sum();
        if total > 0.0 {
            let mut at = rng.unit_f32() * total;
            for f in &store.fathers {
                at -= f.weight.max(0.0);
                if at < 0.0 {
                    pick = Some(f.clone());
                    break;
                }
            }
            if pick.is_none() {
                pick = store.fathers.iter().rev().find(|f| f.weight > 0.0).cloned();
            }
        }
    }
    let wild = std::mem::take(&mut store.wild);
    let (build, mutate) = (fathers.build, fathers.mutate);
    // The line this egg would carry before any gamble: the drawn father's, or
    // the layer's own when nothing was drawn.
    let base = match pick {
        Some(f) => Some((f.mind, f.genome)),
        None => fathers.genome_of(&layer_mind).map(|g| (layer_mind.clone(), g)),
    };
    let Some((mut m, mut g)) = base else {
        // Nothing drawn and the layer's genome unknown: a wild level cannot be
        // spent on a mind the store cannot rebuild, so it waits for the next.
        if wild > 0 {
            if let Some(s) = world.fathers.as_mut().and_then(|f| f.colonies.get_mut(&colony)) {
                s.wild = wild;
            }
        }
        return;
    };
    if wild > 0 {
        let seed = rng.next_u64();
        let flat = mutate(&g, wild, seed);
        m = build(&flat);
        g = Arc::new(flat);
    }
    let fathers = world.fathers.as_mut().expect("read above");
    fathers.know(&m, &g);
    fathers.log.push(Fathered { child, layer, colony, frame, mind: m.clone(), genome: g, queued, wild });
    if !Arc::ptr_eq(&m, &layer_mind) {
        mind::give(world, child, m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(g: &[f32]) -> Arc<MindGenome> {
        crate::train::challenges::colony_bed::mind(g)
    }
    fn mutate(g: &[f32], level: u8, seed: u64) -> Vec<f32> {
        crate::lab::training::mutate_genome(g, 0.1 * f32::from(level), 1.0, 30, seed)
    }

    /// A lab colony on the trained pool, landed early, with the steered
    /// queen marked; returns the lab and her id.
    fn colony() -> (crate::lab::Lab, OrganismId) {
        use crate::lab::scenario::{Placement, Scenario};
        let mut sc = Scenario::load("steady_income").expect("steady_income ships");
        for e in &mut sc.timeline {
            if matches!(e.what, Placement::Colony { .. } | Placement::Heap { .. }) {
                e.at = 200;
            }
        }
        let mut lab = crate::lab::Lab::new(sc.bed.clone());
        lab.load_scenario(sc);
        lab.set_new_ant(true);
        for _ in 0..400 {
            lab.tick_for_harness();
        }
        let mut ids = lab.world.live_organism_ids();
        ids.sort();
        let id = *ids.iter().find(|&&i| mind::has(&lab.world, i)).expect("a mind ant");
        assert!(crate::sim::steer::mark(&mut lab.world, id, None));
        (lab, id)
    }

    /// **The store's three routes, egg by egg, on the real hook**: with no
    /// store an egg keeps the layer's mind; a queued father is spent by
    /// exactly one egg (and not again when the egg hatches, which is why the
    /// hook is at `try_bud` and not inside `mind::inherit`); a wild level
    /// gives a mind that differs from its father's genome; and with
    /// `queen_only` a worker's egg is left alone.
    #[test]
    fn an_egg_takes_the_queue_then_the_draw_and_a_wild_level_once() {
        let (mut lab, queen) = colony();
        let pool = crate::lab::training::std_pool();
        let fa = Father { mind: build(&pool[3]), genome: Arc::new(pool[3].clone()), weight: 1.0 };
        let child_a = OrganismId::MAX - 1;
        // No store: nothing happens.
        on_egg(&mut lab.world, queen, child_a);
        assert!(!mind::has(&lab.world, child_a));
        let mut f = Fathers::new(build, mutate);
        let colony = lab.world.organism(queen).expect("live").colony;
        let mut store = Store { p: 0.0, queen_only: true, ..Store::default() };
        store.queue.push_back(fa.clone());
        f.colonies.insert(colony, store);
        lab.world.fathers = Some(Box::new(f));
        // Queued: the first egg takes it, the second does not (p = 0).
        mind::inherit(&mut lab.world, queen, child_a);
        on_egg(&mut lab.world, queen, child_a);
        assert!(Arc::ptr_eq(&mind::genome_of(&lab.world, child_a).expect("a mind"), &fa.mind), "the queued father was not used");
        let child_b = OrganismId::MAX - 2;
        mind::inherit(&mut lab.world, queen, child_b);
        on_egg(&mut lab.world, queen, child_b);
        let hers = mind::genome_of(&lab.world, queen).expect("hers");
        assert!(Arc::ptr_eq(&mind::genome_of(&lab.world, child_b).expect("a mind"), &hers), "a spent queue was used twice");
        // Hatching copies the egg's mind on; the queue is not touched again.
        assert_eq!(lab.world.fathers.as_ref().unwrap().log.len(), 1, "an egg with nothing drawn was logged");
        // Wild: a mutated copy of the father.
        {
            let s = lab.world.fathers.as_mut().unwrap().colonies.get_mut(&colony).unwrap();
            s.queue.push_back(fa.clone());
            s.wild = 1;
        }
        let child_c = OrganismId::MAX - 3;
        mind::inherit(&mut lab.world, queen, child_c);
        on_egg(&mut lab.world, queen, child_c);
        let last = lab.world.fathers.as_ref().unwrap().log.last().cloned().expect("logged");
        assert_eq!(last.wild, 1);
        let differ = last.genome.iter().zip(fa.genome.iter()).filter(|(a, b)| a != b).count();
        assert!(differ > 0, "a wild egg's genome is its father's");
        assert!(last.genome.len() == fa.genome.len(), "the wild copy lost its lay gate or head");
        assert_eq!(lab.world.fathers.as_ref().unwrap().colonies[&colony].wild, 0, "the wild level was not spent");
        // A worker (not the steered queen) under queen_only: untouched.
        let worker = lab
            .world
            .live_organism_ids()
            .into_iter()
            .find(|&i| i != queen && mind::has(&lab.world, i) && lab.world.organism(i).is_some_and(|s| s.colony == colony))
            .expect("a worker");
        lab.world.fathers.as_mut().unwrap().colonies.get_mut(&colony).unwrap().queue.push_back(fa.clone());
        let child_d = OrganismId::MAX - 4;
        mind::inherit(&mut lab.world, worker, child_d);
        on_egg(&mut lab.world, worker, child_d);
        assert!(Arc::ptr_eq(&mind::genome_of(&lab.world, child_d).unwrap(), &mind::genome_of(&lab.world, worker).unwrap()), "a worker's egg spent the queen's queue");
    }
}
