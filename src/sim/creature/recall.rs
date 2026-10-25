//! **The recall head: a trained ant that acts on a place it remembers.**
//!
//! Built for the keeper seat demo (`Reports/keeper-seat-demo-2026-10-25.md`)
//! and shared with the scout seat (`/mnt/project-files/seat-demos/shared/
//! recall-head.md` while the seat demos run). Off unless a demo or
//! `PIXEL_PHYSICS_RECALL_GAIN` sets [`World::recall`]: `None` is one `Option`
//! read per decision and changes nothing else.
//!
//! **Why a side term and not a weight in the pool.** Every mind already
//! remembers where it last picked up food away from home
//! (`OrganismState::last_food`, read as `options::f::FOOD_WAY`), but the
//! shipped pool was selected before that sense existed, so its weights on it
//! are zeroed as the pool is read (`mind::enter_last_food`): writing the
//! memory changes no choice on today's ants. Hand-seeding the slot has no
//! clean place to go (all sixteen scorer units are live in every member, so a
//! seeded weight mixes into a unit that carries ~25 other inputs), and the
//! hand-built walk's `chooser_step` pulls never reach a mind ant (it returns
//! into `mind::tick` first). The dig head is the precedent that works: a term
//! the target pick reads beside the shared scorer's own weight, kept out of
//! the genome so nothing mutates it.
//!
//! **How it acts: a tilt on which way an unladen ant walks.** When the verb is
//! Walk, each Walk target's pick weight is multiplied by `exp(gain *
//! FOOD_WAY)`. `FOOD_WAY` is the cosine of the step to the remembered spot,
//! times a graded distance ramp, times the memory's freshness (`T / (T +
//! age)`, half strength after `options::LAST_FOOD_FADE` frames), so the pull
//! grows with distance, fades with time, and is 0 with no memory. **It never
//! changes whether the ant walks** (the job stage is untouched) **nor excludes
//! a heading** (a multiplier on a weight, under `choose_weighted`'s floor), so
//! a taught ant still digs, eats, feeds and wanders, just more often toward
//! the place. **Multiplied after the squash, not added before it**: the
//! shared scorer saturates many options near 1 (the dig head's own note), and
//! a term added before the squash is flattened by exactly that.
//!
//! **Laden ants are left alone** (`crop` or `spoil` in the jaws): a taught ant
//! carrying food home is not turned round to walk back out.
//!
//! **The meal copy** ([`share_on_feed`]): when a mind ant feeds a nestmate
//! (`mind::feed`, adult branch), the receiver takes the giver's remembered
//! place if it is fresher than its own, **at `copy` times the giver's
//! freshness** (the frame is set back so `options::last_food`'s own fade
//! reads it so). Second-hand knowledge is weaker than first-hand and
//! third-hand weaker again, so a place spreads through the colony as a graded
//! epidemic that thins with distance from the ant that was there, and fades
//! unless a real trip renews it. Feeds are common (about 70 per 1,000 frames
//! in a 30-ant colony, keeper step 0), so a full-strength copy would make the
//! whole colony "know" within a thousand frames and the share would stop
//! being graded.

use std::collections::BTreeMap;

use crate::sim::cell::OrganismId;
use crate::sim::world::World;

use super::options;

/// How an ant came to know its remembered place, for the read-outs and the
/// gate: the player's hand ([`plant_with`]), or a meal from a nestmate
/// (generation 1 is fed by a taught ant, 2 by a generation-1 ant, and so on).
/// **It describes one memory** (`place`, `at`): when the ant's own pick-up
/// overwrites `last_food` with somewhere else, the record no longer matches
/// and is ignored (and pruned), so a page never calls a memory of the larder
/// "taught by your hand".
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Provenance {
    /// 0 = planted by hand; k = the k-th meal from someone who was taught.
    pub generation: u8,
    /// Who fed it the place (`None` for a planted memory).
    pub giver: Option<OrganismId>,
    /// The frame the lesson or meal landed.
    pub frame: u64,
    /// The remembered place this record describes.
    pub place: (i32, i32),
    /// **How strongly it was learned**, in 0..=1, kept apart from the
    /// memory's age. A hand lesson is 1 (or the scout's broken-run fraction),
    /// a meal copy `copy` times the giver's effective strength. Stored here
    /// rather than folded into an aged `last_food` frame, because ageing a
    /// copy by setting its frame back saturates at frame 0 early in a run and
    /// made early second- and third-hand copies read too strong (plan review M7).
    pub strength: f32,
}

/// What the head did, for counters beside a picture (`CLAUDE.md`: did it fire at all).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RecallCounts {
    /// Walk decisions the tilt applied to (an unladen, taught ant).
    pub tilted: u64,
    /// **The probability mass the tilt moved, summed over those decisions**
    /// (total variation between the Walk target distributions with and without
    /// the term, under `choose_weighted`'s own rule). Exactly 0 at gain 0.
    pub mass_moved: f64,
    /// Decisions whose pick, under the same draw, differs with the tilt from
    /// without it: the steps the head actually changed. Exactly 0 at gain 0.
    pub picks_flipped: u64,
    /// Decisions made by a taught, unladen ant (the tilt was on offer),
    /// whatever verb the job head then drew: `tilted / offered` is the Walk
    /// share the tilt can act on, the plan's first-named risk.
    pub offered: u64,
    pub plants: u64,
    pub copies: u64,
    /// Copies by the generation they wrote (index 1..=3; 4+ in the last slot).
    pub copies_by_generation: [u64; 5],
}

#[derive(Clone, Debug, Default)]
pub struct Recall {
    /// The listening gain: how strongly a taught place tilts an unladen ant's
    /// walk. Scripted for the demos (the pool has no weight on the memory);
    /// 0 is the honest arm, today's ants.
    pub gain: f32,
    /// The meal copy's strength: a received place is learned `copy` times as
    /// strongly as the giver holds it. 0 turns the copy off.
    pub copy: f32,
    pub counts: RecallCounts,
    /// How each ant holding a taught or copied memory came by it.
    pub provenance: BTreeMap<OrganismId, Provenance>,
}

/// How far (Chebyshev cells) an ant's `last_food` may sit from a
/// provenance's place and still be that memory: its own pick-up at the taught
/// station lands on a neighbouring food cell, not the planted one. **12, not
/// 6**: the keeper demo's station is a heap of ~80 cells, a pyramid nine
/// columns either side of its peak, so a taught ant that walked there and
/// took a bite from the heap's edge would lose its lesson at 6 -- the trip that
/// should renew the memory erased it.
pub const SAME_PLACE: i32 = 12;

impl Recall {
    pub fn new(gain: f32, copy: f32) -> Self {
        Self { gain, copy, ..Self::default() }
    }

    /// `PIXEL_PHYSICS_RECALL_GAIN=g[,copy]`: the head for a harness or a
    /// playtest outside the demos. Unset: `None`, the shipped lab.
    pub fn from_env() -> Option<Self> {
        let v = std::env::var("PIXEL_PHYSICS_RECALL_GAIN").ok()?;
        let mut parts = v.split(',').map(|p| p.trim().parse::<f32>().unwrap_or_else(|_| panic!("PIXEL_PHYSICS_RECALL_GAIN={v:?}: g or g,copy")));
        let gain = parts.next().unwrap_or(0.0);
        let copy = parts.next().unwrap_or(0.0);
        Some(Self::new(gain, copy))
    }
}

/// The provenance that still describes `ant`'s memory, if any.
pub fn lesson(world: &World, ant: OrganismId) -> Option<Provenance> {
    let p = *world.recall.as_ref()?.provenance.get(&ant)?;
    let (x, y, _) = world.organism(ant)?.last_food?;
    ((x - p.place.0).abs() <= SAME_PLACE && (y - p.place.1).abs() <= SAME_PLACE).then_some(p)
}

/// Forget records for the dead. **Only the dead** (keeper demo, 2026-10-25):
/// a record whose memory has since been overwritten is kept *dormant*, and
/// wakes again ([`lesson`] matches) the next time the ant takes food back at
/// the taught place. Traced on the demo box, a carried ant that took one
/// stray cell dropped on its way home lost its lesson for good under the
/// first rule (pruned on mismatch), and its next real bite at the station a
/// thousand frames later taught it nothing: the trip that should renew the
/// lesson could not. Nothing reads a dormant record as a lesson ([`lesson`]
/// re-checks the place), so this changes only what a return trip restores.
pub fn prune(world: &mut World) {
    let Some(ids) = world.recall.as_ref().map(|r| r.provenance.keys().copied().collect::<Vec<_>>()) else { return };
    let dead: Vec<OrganismId> = ids.into_iter().filter(|&id| world.organism(id).is_none()).collect();
    if let Some(r) = world.recall.as_mut() {
        for id in dead {
            r.provenance.remove(&id);
        }
    }
}

/// **Write `ant`'s remembered place**, fresh, as if it had just picked up
/// food at `cell`, learned at `strength` in 0..=1: the keeper's treat (1),
/// the scout's tandem arrival (1, or less for a broken run). Returns false if
/// the ant is gone.
pub fn plant_with(world: &mut World, ant: OrganismId, cell: (i32, i32), strength: f32) -> bool {
    let frame = world.frame;
    let Some(st) = world.organism_mut(ant) else { return false };
    st.last_food = Some((cell.0, cell.1, frame));
    if let Some(r) = world.recall.as_mut() {
        r.counts.plants += 1;
        r.provenance.insert(ant, Provenance { generation: 0, giver: None, frame, place: cell, strength: strength.clamp(0.0, 1.0) });
    }
    true
}

/// [`plant_with`] at full strength.
pub fn plant(world: &mut World, ant: OrganismId, cell: (i32, i32)) -> bool {
    plant_with(world, ant, cell, 1.0)
}

/// The freshness `options::last_food` reads for a memory made at `at`, at
/// `frame`: `T / (T + age)` -- a half at `T` frames, a third at `2T`, a
/// quarter at `3T` (hyperbolic, not a half-life). `None` with the memory off.
pub fn freshness(frame: u64, at: u64) -> Option<f32> {
    options::last_food().map(|t| t / (t + frame.saturating_sub(at) as f32))
}

/// **How well `ant` knows `place`**: strength times freshness, if its taught
/// or copied memory is within `r` cells of `place`; `None` if it was never
/// taught, remembers somewhere else, or the memory is off. A read-out
/// counting "knows" should cut at a threshold (the demo prints its own), since
/// the hyperbolic fade never reaches 0.
pub fn knows(world: &World, ant: OrganismId, place: (i32, i32), r: i32) -> Option<f32> {
    let p = lesson(world, ant)?;
    ((p.place.0 - place.0).abs() <= r && (p.place.1 - place.1).abs() <= r).then_some(())?;
    let (_, _, at) = world.organism(ant)?.last_food?;
    Some(p.strength * freshness(world.frame, at)?)
}

/// The gain to tilt this ant's Walk pick by, or `None`: off, untaught, the
/// memory since overwritten, or laden (food or spoil in the jaws or gut).
/// **Only taught and copied memories tilt** (plan review M4): a gain on every
/// ant that ever found food would change the whole colony's foraging when
/// the demo's key is pressed, and pull hungry never-taught ants far out, the
/// harm that left `PIXEL_PHYSICS_RECRUIT` off. Read once per decision by
/// `mind::tick`.
pub fn tilt_for(world: &World, ant: OrganismId) -> Option<f32> {
    let r = world.recall.as_ref()?;
    let st = world.organism(ant)?;
    if st.crop.is_some() || st.spoil.is_some() {
        return None;
    }
    let p = lesson(world, ant)?;
    Some(r.gain * p.strength)
}

thread_local! {
    /// The gain for the decision `mind::tick` is about to make, set just
    /// before `decide_planned` and cleared just after (the dig head's
    /// `scratch_log` pattern: the scorer takes no world).
    pub static TILT: std::cell::Cell<Option<f32>> = const { std::cell::Cell::new(None) };
    /// What that decision's tilt did, if its verb was Walk: (mass moved, pick flipped).
    pub static MOVED: std::cell::Cell<Option<(f32, bool)>> = const { std::cell::Cell::new(None) };
}

/// **Re-weight the Walk targets** `weights` (each the pick weight the shared
/// scorer gave it; `orig` holds a copy) by `exp(gain * food_way(i))`, and
/// return the probability mass that moved under `creature::choose_weighted`'s
/// own rule (`(floor + w)^2`), so the counter measures what the pick will
/// actually do. No allocation: it runs on most Walk decisions of a taught ant.
pub fn tilt(weights: &mut [f32], orig: &[f32], food_way: impl Fn(usize) -> f32, gain: f32, floor: f32) -> f32 {
    debug_assert_eq!(weights.len(), orig.len());
    let b = |s: f32| (floor + s.max(0.0)).powi(2);
    for (i, w) in weights.iter_mut().enumerate() {
        *w *= (gain * food_way(i)).exp();
    }
    let t0: f32 = orig.iter().map(|&s| b(s)).sum();
    let t1: f32 = weights.iter().map(|&s| b(s)).sum();
    if t0 <= 0.0 || t1 <= 0.0 {
        return 0.0;
    }
    0.5 * orig.iter().zip(weights.iter()).map(|(&a, &c)| (b(a) / t0 - b(c) / t1).abs()).sum::<f32>()
}

/// Book one tilted decision.
pub fn note_tilt(world: &mut World, moved: f32, flipped: bool) {
    if let Some(r) = world.recall.as_mut() {
        r.counts.tilted += 1;
        r.counts.mass_moved += moved as f64;
        r.counts.picks_flipped += u64::from(flipped);
    }
}

/// **The meal copy**, called by `mind::feed` after an adult share from
/// `giver` to `taker`. Only a taught or copied memory spreads (the keeper's
/// lesson and its descendants, not every ant's own finds), at `copy` times
/// the giver's effective strength; it replaces what the taker holds only if
/// stronger. An untaught taker's own memory counts at its freshness:
/// first-hand experience outweighs hearsay of the same strength.
pub fn share_on_feed(world: &mut World, giver: OrganismId, taker: OrganismId) {
    let Some(copy) = world.recall.as_ref().map(|r| r.copy).filter(|&c| c > 0.0) else { return };
    let Some(gp) = lesson(world, giver) else { return };
    let Some((gx, gy, gat)) = world.organism(giver).and_then(|s| s.last_food) else { return };
    let frame = world.frame;
    let Some(gf) = freshness(frame, gat) else { return };
    let given = copy * gp.strength * gf;
    let held = match (lesson(world, taker), world.organism(taker).and_then(|s| s.last_food)) {
        (Some(tp), Some((_, _, at))) => tp.strength * freshness(frame, at).unwrap_or(0.0),
        (None, Some((_, _, at))) => freshness(frame, at).unwrap_or(0.0),
        _ => 0.0,
    };
    if given <= held || given <= 0.0 {
        return;
    }
    let Some(st) = world.organism_mut(taker) else { return };
    st.last_food = Some((gx, gy, frame));
    let r = world.recall.as_mut().expect("checked above");
    let generation = gp.generation.saturating_add(1);
    r.provenance.insert(taker, Provenance { generation, giver: Some(giver), frame, place: (gx, gy), strength: given });
    r.counts.copies += 1;
    r.counts.copies_by_generation[(generation as usize).min(4)] += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tilt_moves_mass_toward_the_remembered_way_and_none_at_gain_zero() {
        let orig = [0.5, 0.5, 0.5];
        let fw = [1.0, 0.0, -1.0];
        let mut w = orig;
        assert_eq!(tilt(&mut w, &orig, |i| fw[i], 0.0, 0.05), 0.0, "gain 0 must move exactly nothing: the honest arm");
        assert_eq!(w, orig);
        let moved = tilt(&mut w, &orig, |i| fw[i], 1.0, 0.05);
        assert!(w[0] > w[1] && w[1] > w[2], "toward the place weighs most: {w:?}");
        assert!(moved > 0.2 && moved < 1.0, "a real but partial shift: {moved}");
        assert!(w[2] > 0.0, "never excludes a heading");
    }
}
