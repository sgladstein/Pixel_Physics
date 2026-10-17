//! **An evolved target scorer: which cell an ant digs, and where it puts a
//! pellet down** -- stage 1 of the brain plan (`brain-training/design/
//! general-brain-design-rev3.md`, approved by the owner 2026-10-17).
//!
//! Today's brain still decides *whether* to dig and *whether* to put a pellet
//! down (`Dig`, `DropSpoil`). This decides *where*. Every legal target -- each
//! neighbouring cell the jaws can cut, each free neighbouring cell a pellet
//! can go in -- is described by the same short list of features, and one
//! small layer scores them all; one more option, "none", has a learned
//! constant score of its own. The pick is the engine's own
//! [`super::choose_weighted`] over the scores shifted from -1..1 to 0..1, so a
//! bad target and a forbidden one stay ordered, with the floor
//! [`FLOOR`] stated as a chance: at full confidence a target scored -1 beside
//! one scored +1 is drawn about 0.2% as often.
//!
//! **What it replaces, for the species it is set on, and nothing else: the
//! rules that pick *where*, one at a time, as the design says.** This first
//! rung replaces only the choice of cell. On the dig side, the cell straight
//! ahead and the four rules that move it (the widening cut, the turn down,
//! the face turn, the shaft-room turn); every veto still stands, judged on
//! each candidate as it would be on the cell ahead (`dig_veto`). On the
//! spoil side, the first free cell in neighbour order; the holds, the cover
//! scale, the roll and the lift still decide whether a pellet goes down now
//! and where it goes when no cell beside the carrier will take it. Each
//! rule that stays is a filter the next rung can take away and test.
//!
//! **The first version replaced everything at once and failed its own
//! positive control.** With the vetoes and holds gone, a hand-built scorer
//! cut at 20-30% of its rolls against the hand rules' 2% (108 cuts from
//! 5,849 rolls), churned an open bowl (pit +85 to +158 against +1 to +8)
//! and scored 12.8 rooms' worth of cells against the hand rules' 70.7 and a
//! uniform pick's 10.6 (ladder lane, `ladder/probes/stage1-handbuilt/`).
//! The features could not say what the vetoes say, and an arena that
//! cannot be passed by hand cannot tell a failed run from a broken arena.
//!
//! **"None" is kept on both sides.** On the spoil side it is "keep
//! carrying", and it is nearly the whole decision there: the neighbour
//! choice alone almost never has two kinds of place to choose between --
//! counted at the drop, about one in a hundred (`Reports/dead-ends.md`, the
//! Khuong deposition entry, re-counted 2026-09-27).
//!
//! **What has been tried before, and what differs.** Two hand-written
//! dig-target preferences (burial, then cover overhead) were swept across
//! their range in 2026-08 and moved nothing, the colony already tunnelling
//! as one connected run (`dead-ends.md`, "weighting the roll by how much
//! cover"). Those were single terms added on top of every existing rule and
//! tuned by hand on one bed. This replaces the rules, reads several features
//! at once and is selected on joined room, so a failure here says something
//! different; if it fails the same way, that entry's conclusion -- the target
//! is not where nest shape comes from -- is the reading to start from.
//!
//! Off unless the training rig sets [`World::targets`], so the game is
//! unchanged.
//!
//! [`World::targets`]: crate::sim::World::targets

use super::{choose_weighted, inside_nest, jaw_can_cut, CreatureDef, DIRS};
use crate::sim::cell::OrganismId;
use crate::sim::material::{self, MaterialKind};
use crate::sim::organism::SpeciesId;
use crate::sim::pheromone::{self, Channel};
use crate::sim::World;

/// Features per target, in [`feature_names`] order.
pub const FEATURES: usize = 15;
/// Which verb the target belongs to, one-hot after the features: dig, drop.
pub const VERBS: usize = 2;
/// Hidden units of the shared layer.
pub const HIDDEN: usize = 8;
const INPUTS: usize = FEATURES + VERBS + 1;
/// Weights: input->hidden with a bias row, hidden->out with its bias, and
/// one "none" constant per verb.
pub const WEIGHTS: usize = INPUTS * HIDDEN + HIDDEN + 1 + VERBS;
/// The pick's floor: `choose_weighted`'s `k`, over scores shifted to 0..1.
/// `(0.05 / 1.05)^2` is 0.23%: how often a target scored -1 is drawn against
/// one scored +1.
pub const FLOOR: f32 = 0.05;

pub const VERB_DIG: usize = 0;
pub const VERB_DROP: usize = 1;

/// The names of the features, for readouts.
pub fn feature_names() -> [&'static str; FEATURES] {
    ["up", "forward", "roof", "footing", "open", "depth", "spoil", "crowd", "homeward", "nest_scent", "inside", "packed", "ahead", "door", "is_spoil"]
}

/// Index of the input->hidden weight from input `i` to hidden unit `h`.
pub fn w_in(i: usize, h: usize) -> usize {
    i * HIDDEN + h
}
/// Index of hidden unit `h`'s weight into the score.
pub fn w_out(h: usize) -> usize {
    INPUTS * HIDDEN + h
}
/// Index of the score's own bias.
pub const W_OUT_BIAS: usize = INPUTS * HIDDEN + HIDDEN;
/// Index of verb `v`'s "none" constant.
pub fn w_none(v: usize) -> usize {
    W_OUT_BIAS + 1 + v
}
/// Input index of feature `f`, of verb `v`'s one-hot, and of the bias.
pub fn in_feature(f: usize) -> usize {
    f
}
pub fn in_verb(v: usize) -> usize {
    FEATURES + v
}
pub const IN_BIAS: usize = FEATURES + VERBS;

/// What one world's scorer did, for readouts: the "it fired" counters and
/// the margin between the two best options.
#[derive(Clone, Debug, Default)]
pub struct TargetStats {
    /// Decisions, by verb.
    pub decisions: [u64; VERBS],
    /// Decisions that had at least one legal target, by verb.
    pub with_targets: [u64; VERBS],
    /// Decisions that picked "none", by verb.
    pub none: [u64; VERBS],
    /// Decisions with two or more legal targets: the ones where where it
    /// went was a choice at all.
    pub choices: [u64; VERBS],
    /// Sum of (best - second best) score over `choices`.
    pub margin_sum: [f64; VERBS],
    /// Targets picked, by the target's row offset (-1, 0, +1 -> 0, 1, 2).
    pub picked_dy: [[u64; 3]; VERBS],
    /// Pellets put down under cover (`inside_nest`) and outside it.
    pub dropped_inside: u64,
    pub dropped_outside: u64,
}

/// **Which of the hand rules' filters this scorer replaces too**, beyond the
/// choice of cell: the rungs of the design's one-rule-at-a-time ladder.
/// Each one freed is a decision the scorer must now make itself, through
/// its "none" option and how low it scores the targets the rule refused.
/// Ordered by how much of the hand rules' refusal each carries, smallest
/// first (ladder lane, `probes/hand-rules-dig-refusals.log`, 12 ants,
/// 20,000 frames, of about 5,500-6,000 dig rolls a run): the spoil holds;
/// the heap cue, about 10% of rolls; the face trip, about 12%; the company
/// rule, about 37%. The roof, the mound's roof and the shaft rooms stay:
/// they refused 1.5%, 0 and 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Free {
    /// The spoil branch's holds -- kept inside, near the door, below
    /// ground, the ring's, the sky's -- and with them the lift: with no
    /// cell beside it a carrier keeps its pellet.
    pub holds: bool,
    /// The heap cue's veto on a cut that would open the ground to the sky.
    pub cue: bool,
    /// The face trip's veto: walking back to its face, a digger cuts only
    /// near it.
    pub face: bool,
    /// The company rule: a lone ant at a home wall does not cut.
    pub company: bool,
}

impl Free {
    /// `holds,cue,face,company`, any of them, comma-separated; empty is
    /// none. An unknown name stops the run rather than freeing nothing.
    pub fn parse(raw: &str) -> Free {
        let mut f = Free::default();
        for part in raw.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            match part {
                "holds" => f.holds = true,
                "cue" => f.cue = true,
                "face" => f.face = true,
                "company" => f.company = true,
                _ => panic!("free={raw}: holds,cue,face,company"),
            }
        }
        f
    }
}

/// A scorer for one species' animals.
#[derive(Clone, Debug)]
pub struct Targets {
    pub species: SpeciesId,
    pub weights: Vec<f32>,
    pub stats: TargetStats,
    /// **The matched-rate legality control** (the training rig's `legal`
    /// row): the scorer still decides between acting and "none", and the
    /// target is then drawn evenly from the legal ones. Every target's
    /// weight in the draw is set to their mean, so the chance of "none" is
    /// exactly the scorer's own and only *which* target changes.
    pub even: bool,
    /// The hand filters it replaces as well ([`Free`]).
    pub free: Free,
}

impl Targets {
    pub fn new(species: SpeciesId, weights: &[f32]) -> Self {
        assert_eq!(weights.len(), WEIGHTS, "a target scorer has {WEIGHTS} weights");
        Targets { species, weights: weights.to_vec(), stats: TargetStats::default(), even: false, free: Free::default() }
    }
}

/// The filters `organism`'s scorer replaces, none when it has no scorer.
pub fn free_of(world: &World, organism: OrganismId) -> Free {
    if active(world, organism) {
        world.targets.as_ref().map_or(Free::default(), |t| t.free)
    } else {
        Free::default()
    }
}

/// The brain's squash, `x / (1 + |x|)`: no transcendental in anything a
/// decision reads (`brain.rs`).
fn squash(x: f32) -> f32 {
    x / (1.0 + x.abs())
}

/// Whether `organism` picks its targets with an evolved scorer.
pub fn active(world: &World, organism: OrganismId) -> bool {
    world.targets.as_ref().is_some_and(|t| world.organism(organism).is_some_and(|s| s.species == t.species))
}

fn is_ground(world: &World, x: i32, y: i32) -> bool {
    let c = world.get(x, y);
    c.material != material::EMPTY && matches!(world.materials.kind(c.material), MaterialKind::Powder | MaterialKind::Solid) && c.organism_id() == 0
}

fn is_filled(world: &World, x: i32, y: i32) -> bool {
    world.get(x, y).material != material::EMPTY
}

fn is_animal(world: &World, x: i32, y: i32) -> bool {
    world.materials.kind(world.get(x, y).material) == MaterialKind::Creature
}

/// One target's features, each roughly in -1..1. `facing` is the sign of the
/// ant's heading along x (0 when it faces straight up or down), so "forward"
/// means the same thing facing either way and a rule learned facing east
/// works facing west; `ahead` is the cell its heading points at.
pub fn features(world: &World, (x, y): (i32, i32), (px, py): (i32, i32), facing: i32, ahead: (i32, i32)) -> [f32; FEATURES] {
    let mut f = [0.0f32; FEATURES];
    f[0] = (y - py) as f32;
    f[1] = ((px - x) * facing) as f32;
    f[2] = (1..=3).filter(|&d| is_ground(world, px, py - d)).count() as f32 / 3.0;
    f[3] = (-1..=1).filter(|&d| is_filled(world, px + d, py + 1)).count() as f32 / 3.0;
    f[4] = DIRS.iter().filter(|&&(dx, dy)| world.get(px + dx, py + dy).material == material::EMPTY).count() as f32 / 8.0;
    // Below the frozen ground the colony started on: positive deeper,
    // negative above (the mound), in fives of rows: a founding bowl is two
    // or three deep, and tens read it as level ground.
    f[5] = world.room_surface_at(px).map_or(0.0, |s| ((py - s) as f32 / 5.0).clamp(-1.0, 1.0));
    let spoil = world.materials.id_of("spoil");
    let mut near = 0;
    for dy in -2..=2 {
        for dx in -2..=2 {
            if spoil.is_some_and(|s| world.get(px + dx, py + dy).material == s) {
                near += 1;
            }
        }
    }
    f[6] = (near as f32 / 8.0).min(1.0);
    f[7] = DIRS.iter().filter(|&&(dx, dy)| is_animal(world, px + dx, py + dy)).count() as f32 / 8.0;
    // Which way home is from there: closer to the nearest door than where
    // the ant stands (+1 per cell). Cut in a home-blind world, the training
    // rig's senses-cut control.
    if !world.home_blind {
        if let Some(site) = world.nearest_nest_site(x, y).and_then(|i| world.nest_sites.get(i)) {
            let d = |a: i32, b: i32| (((a - site.x).pow(2) + (b - site.surface).pow(2)) as f32).sqrt();
            f[8] = (d(x, y) - d(px, py)).clamp(-1.5, 1.5);
        }
    }
    f[9] = world.pheromone_at(Channel::A, px, py) as f32 / pheromone::Scent::MAX as f32;
    f[10] = if inside_nest(world, px, py) { 1.0 } else { 0.0 };
    let packed = world.materials.id_of("packedsoil");
    f[11] = if packed.is_some_and(|p| world.get(px, py).material == p) { 1.0 } else { 0.0 };
    // The cell the ant faces: the hand rules' own first choice, and a
    // sense of the body's, not of the rules.
    f[12] = if (px, py) == ahead { 1.0 } else { 0.0 };
    // How far the target is from the nearest door, in 32s of cells, up to
    // 2: "homeward" says only which way, and taking a pellet out to a ring
    // needs how far (ladder lane, the first version's failed control). Cut
    // in a home-blind world.
    if !world.home_blind {
        if let Some(site) = world.nearest_nest_site(px, py).and_then(|i| world.nest_sites.get(i)) {
            f[13] = ((((px - site.x).pow(2) + (py - site.surface).pow(2)) as f32).sqrt() / 32.0).min(2.0);
        }
    }
    // A pellet: loose spoil is the cheapest cut there is, and cutting it is
    // moving a pellet, not digging.
    f[14] = if spoil.is_some_and(|s| world.get(px, py).material == s) { 1.0 } else { 0.0 };
    f
}

/// The score of one target, -1..1.
pub fn score(w: &[f32], feats: &[f32; FEATURES], verb: usize) -> f32 {
    let mut input = [0.0f32; INPUTS];
    input[..FEATURES].copy_from_slice(feats);
    input[in_verb(verb)] = 1.0;
    input[IN_BIAS] = 1.0;
    let mut out = w[W_OUT_BIAS];
    for h in 0..HIDDEN {
        let mut a = 0.0;
        for (i, &v) in input.iter().enumerate() {
            a += w[w_in(i, h)] * v;
        }
        out += w[w_out(h)] * squash(a);
    }
    squash(out)
}

/// **Pick a target for `verb`**, or `None` for the "none" option or when the
/// list is empty and "none" is all there is. `candidates` are the legal
/// targets; `draw` is one uniform from the caller's stream.
pub fn pick(world: &mut World, organism: OrganismId, verb: usize, candidates: &[(i32, i32)], draw: f32) -> Option<(i32, i32)> {
    let st = world.organism(organism)?;
    let &(x, y) = st.chain.first()?;
    let (hx, hy) = DIRS[st.heading as usize];
    let (facing, ahead) = (hx.signum(), (x + hx, y + hy));
    let t = world.targets.as_ref()?;
    let w = &t.weights;
    let mut scores: Vec<f32> = candidates.iter().map(|&c| (score(w, &features(world, (x, y), c, facing, ahead), verb) + 1.0) * 0.5).collect();
    if t.even && !candidates.is_empty() {
        // `choose_weighted` weights a score s by (FLOOR + s)^2 and walks the
        // list in order with "none" last, so giving every target the mean
        // weight leaves the targets' total, and with it the draw that lands
        // on "none", exactly as it was.
        let mean = scores.iter().map(|&s| (FLOOR + s.max(0.0)).powi(2)).sum::<f32>() / scores.len() as f32;
        scores.fill(mean.sqrt() - FLOOR);
    }
    scores.push((squash(w[w_none(verb)]) + 1.0) * 0.5);
    let i = choose_weighted(&scores, FLOOR, draw);
    let mut sorted: Vec<f32> = scores[..candidates.len()].to_vec();
    sorted.sort_by(|a, b| b.total_cmp(a));
    let stats = &mut world.targets.as_mut().expect("checked above").stats;
    stats.decisions[verb] += 1;
    stats.with_targets[verb] += u64::from(!candidates.is_empty());
    if candidates.len() >= 2 {
        stats.choices[verb] += 1;
        stats.margin_sum[verb] += (sorted[0] - sorted[1]) as f64 * 2.0;
    }
    match candidates.get(i) {
        Some(&(px, py)) => {
            stats.picked_dy[verb][(py - y + 1) as usize] += 1;
            Some((px, py))
        }
        None => {
            stats.none[verb] += 1;
            None
        }
    }
}

/// The cells the jaws can cut beside the head. The dig branch then keeps
/// only those its vetoes allow (`dig_veto`); the drop's candidates are the
/// spoil branch's own open sites (`spoil_site_open`).
pub fn dig_candidates(world: &World, def: &CreatureDef, organism: OrganismId, (x, y): (i32, i32)) -> Vec<(i32, i32)> {
    DIRS.iter().map(|&(dx, dy)| (x + dx, y + dy)).filter(|&(px, py)| jaw_can_cut(world, def, organism, world.get(px, py))).collect()
}

/// **A scorer that should pass**, built by hand before any evolution, so a
/// stalled run can be told from a broken arena (the design's positive
/// control). Dig: the cell the ant faces, which is the hand rules' own first
/// choice, and otherwise mostly "none" -- the hand rules refuse about 98% of
/// their rolls, most for facing no ground, and a scorer that cuts whatever
/// is beside it churns an open bowl. Drop: a cell with footing outside the
/// nest, and rarely "keep carrying" once the holds have let the pellet go.
///
/// Built on one trick, since a single layer cannot multiply: each verb's
/// hidden unit takes -50 from the other verb's input, so it sits at about
/// -0.98 for every target of that verb, and the output's bias puts that
/// constant back. Worked through: a dig at the cell ahead scores about
/// 0.88, any other cut about -0.90; a drop with footing outside about 0.88,
/// inside about -0.86.
pub fn hand_built() -> Vec<f32> {
    let mut w = vec![0.0f32; WEIGHTS];
    let other = -50.0;
    // h0: dig the cell ahead.
    w[w_in(in_feature(12), 0)] = 8.0;
    w[w_in(in_verb(VERB_DIG), 0)] = 4.0;
    w[w_in(in_verb(VERB_DROP), 0)] = other;
    w[w_in(IN_BIAS, 0)] = -8.0;
    w[w_out(0)] = 10.0;
    // h1: drop on footing, not inside.
    w[w_in(in_feature(3), 1)] = 3.0;
    w[w_in(in_feature(10), 1)] = -6.0;
    w[w_in(in_verb(VERB_DROP), 1)] = 4.0;
    w[w_in(in_verb(VERB_DIG), 1)] = other;
    w[w_in(IN_BIAS, 1)] = -2.0;
    w[w_out(1)] = 10.0;
    // The other verb's unit, put back: 10 x 0.98, less 1.
    w[W_OUT_BIAS] = 8.8;
    // "None": declines about one cut in eight when the cell ahead is
    // allowed, and takes most rolls with no cell ahead to cut; rarely keeps
    // a pellet the holds have let go.
    w[w_none(VERB_DIG)] = -0.48;
    w[w_none(VERB_DROP)] = -3.0;
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The slot map covers the genome exactly once.
    #[test]
    fn every_weight_has_one_slot() {
        let mut seen = vec![0; WEIGHTS];
        for i in 0..INPUTS {
            for h in 0..HIDDEN {
                seen[w_in(i, h)] += 1;
            }
        }
        for h in 0..HIDDEN {
            seen[w_out(h)] += 1;
        }
        seen[W_OUT_BIAS] += 1;
        for v in 0..VERBS {
            seen[w_none(v)] += 1;
        }
        assert!(seen.iter().all(|&n| n == 1), "{seen:?}");
    }

    /// **The verb reaches the score**: one cell, two verbs, two scores, so a
    /// dig and a drop aimed at the same place can mean different things.
    #[test]
    fn the_verb_changes_the_score() {
        let w = hand_built();
        let f = [0.0; FEATURES];
        assert_ne!(score(&w, &f, VERB_DIG), score(&w, &f, VERB_DROP));
    }

    /// **The even draw changes which target, never whether.** Over a spread
    /// of draws, "none" comes up on exactly the same draws with and without
    /// [`Targets::even`], and the even draw spreads its picks where the
    /// scorer's own piles them on one target -- the positive control that
    /// the flag reaches the pick at all.
    #[test]
    fn the_even_draw_keeps_the_rate_of_none() {
        let raw = [0.9f32, 0.1, 0.0, 0.4];
        let none = 0.6f32;
        let mean = raw.iter().map(|&s| (FLOOR + s).powi(2)).sum::<f32>() / raw.len() as f32;
        let even = [mean.sqrt() - FLOOR; 4];
        let (mut picked_raw, mut picked_even) = ([0; 5], [0; 5]);
        for n in 0..10_000 {
            let draw = n as f32 / 10_000.0;
            let a = choose_weighted(&[raw[0], raw[1], raw[2], raw[3], none], FLOOR, draw);
            let b = choose_weighted(&[even[0], even[1], even[2], even[3], none], FLOOR, draw);
            assert_eq!(a == 4, b == 4, "draw {draw}: one said none and the other did not");
            picked_raw[a] += 1;
            picked_even[b] += 1;
        }
        assert!(picked_raw[0] > 3 * picked_raw[3], "the scorer's own draw should favour its best target: {picked_raw:?}");
        let (lo, hi) = (picked_even[..4].iter().min().unwrap(), picked_even[..4].iter().max().unwrap());
        assert!(hi - lo <= 1, "the even draw should split its picks evenly: {picked_even:?}");
    }

    /// **The jaws never get a living thing to cut.** Every head in a working
    /// colony, over a stretch of frames: no dig candidate is a creature's or
    /// a plant's cell, or any cell an organism owns. `dig_candidates` is the
    /// only gate between the scorer and the cut (the hand path also asks
    /// `jaw_can_cut` at the cut), so it must hold this on its own (ladder
    /// lane, `stage1-code-review.md` 6). The control: the ants stand beside
    /// each other, so creature cells are in reach of a head on many frames.
    #[test]
    fn no_dig_candidate_is_alive() {
        let spec = crate::lab::scene::LabBox { founders: 4, colonies: 1, colony_ants: 12, width: 256, height: 128, ground_y: 64, soil_depth: 40, ..crate::lab::scene::LabBox::default() };
        let mut w = spec.build();
        let sp = w.species.id_of("ant").expect("ant");
        let def = w.species.get(sp).creature.clone().expect("creature");
        let (mut checked, mut beside_animal) = (0, 0);
        for f in 0..3000 {
            crate::sim::update::step(&mut w);
            w.step_active_sites();
            w.step_fields();
            w.step_pheromones();
            if f % 25 != 0 {
                continue;
            }
            for id in w.live_organism_ids() {
                if w.organism(id).map(|s| s.species) != Some(sp) {
                    continue;
                }
                let Some(&head) = w.organism(id).and_then(|s| s.chain.first()) else { continue };
                beside_animal += DIRS.iter().filter(|&&(dx, dy)| is_animal(&w, head.0 + dx, head.1 + dy)).count();
                for (px, py) in dig_candidates(&w, &def, id, head) {
                    let c = w.get(px, py);
                    let kind = w.materials.kind(c.material);
                    assert!(kind != MaterialKind::Creature && kind != MaterialKind::Plant && c.organism_id() == 0, "frame {f}: ant {id:?} could cut a living cell at ({px},{py})");
                    checked += 1;
                }
            }
        }
        assert!(checked > 100, "only {checked} candidates checked");
        assert!(beside_animal > 0, "no head ever stood beside another animal, so this saw nothing to refuse");
    }

    /// The hand-built scorer says what its doc works through: the cell
    /// ahead near the top for a dig and every other cut near the bottom,
    /// whatever else the target looks like; a drop with footing outside near
    /// the top and one inside near the bottom, whether or not it is ahead.
    #[test]
    fn the_hand_scorer_says_what_it_was_built_to() {
        let w = hand_built();
        let mut ahead = [0.0; FEATURES];
        ahead[12] = 1.0;
        let mut beside = [0.0; FEATURES];
        beside[2] = 1.0;
        beside[5] = 1.0;
        assert!(score(&w, &ahead, VERB_DIG) > 0.8, "{}", score(&w, &ahead, VERB_DIG));
        assert!(score(&w, &beside, VERB_DIG) < -0.8, "{}", score(&w, &beside, VERB_DIG));
        let mut out = [0.0; FEATURES];
        out[3] = 1.0;
        let mut inside = out;
        inside[10] = 1.0;
        for a in [0.0, 1.0] {
            out[12] = a;
            inside[12] = a;
            assert!(score(&w, &out, VERB_DROP) > 0.8, "{}", score(&w, &out, VERB_DROP));
            assert!(score(&w, &inside, VERB_DROP) < -0.8, "{}", score(&w, &inside, VERB_DROP));
        }
    }
}
