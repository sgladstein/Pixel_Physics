//! **The general brain: one shared scorer over every option, a job head,
//! memory that forgets at its own rate, and emits** -- stage 2, step 6 of
//! the brain plan (`brain-training/design/general-brain-design-rev3.md`,
//! *How it decides*, *Memory*, *Jobs and switching*; stage 2 chosen by the
//! owner 2026-10-17).
//!
//! An ant with a mind does not sense, run `brain.rs` or act by `act`: each
//! decision the body lists what it could legally do ([`options::build`]),
//! the mind scores every option with one shared scorer, picks a job (a
//! verb) from how the ant feels and the best of that verb's targets, then
//! picks a target, and the body does it by the world's rules only
//! ([`tick`]). Nothing that a real ant could have chosen otherwise is
//! decided here by hand: that is the design's *Physics vs policy* line.
//!
//! **Off unless an ant is given a mind** ([`give`]): the game ant has none,
//! so the game is unchanged with this module compiled in. The rig gives
//! minds to its arenas' ants (`train::mind_genome`).
//!
//! **The verbs come in one at a time.** Every verb but Attack acts now
//! (walk, turn around, rest, dig, pick up, put down, eat, feed); Attack is
//! listed and described by `options`, but [`Verb::acts`] keeps it out of
//! every pick until its act lands, whatever the switch genes say. **Each
//! does one thing** (fix plan step 1): Pick up takes food into the crop,
//! Eat eats from the crop, Dig cuts ground ([`act`]).
//!
//! **A blank mind picks uniformly**: every weight 0 scores every option the
//! same, so the job is a fair draw among the verbs with a target and the
//! target a fair draw among that verb's. That is the random null each rung
//! is measured against, not a do-nothing control (ladder lane, 2026-10-17).
//!
//! **No transcendental anywhere a decision reads** (`brain.rs`): squashes
//! are `x / (1 + |x|)`, and the one draw per pick comes from a stream keyed
//! on the seed, the ant and the frame.

use crate::sim::material::Known;
use super::options::{self, Opt, Verb, FEATURES, VERBS};
use super::{
    adjacent_nest, armour_of, bite_force_of, blend_with_nest, carried_cells, choose_weighted, commit_step,
    dig_force_of, fall_now_if_unsupported, flipped_body, is_sheltered, live_body_cells, organism_crop_capacity,
    parting_enabled, relocate_chain, segment_authored, settle, traits_of, under_cover, BodyShape, BodySide,
    CreatureDef, DIRS,
};
use super::{
    blend_with_kin, crop_take, crop_to_feed, food_at_home, food_value, nest_within_reach, note_drop, note_feed,
    quantise_worth, DropWhy, FEED_SHARE, SHARE_FRACTION,
};
use super::{cut_cell, place_spoil, spoil_site_open};
use crate::sim::cell::Cell;
use crate::sim::cell::OrganismId;
use crate::sim::organism::{self, BroodStage, Crop, DeathCause};
use crate::sim::pheromone::{self, Channel};
use crate::sim::plant;
use crate::sim::rng;
use crate::sim::scheduler::ActiveSite;
use crate::sim::world::Account;
use crate::sim::World;
use std::cell::RefCell;
use std::sync::Arc;

/// Units in the shared scorer.
pub const UNITS: usize = 16;
/// Memory units, each with its own forgetting rate.
pub const MEMORY: usize = 12;
/// Scents the ant can lay alongside its verb: A, B and alarm. The building
/// scent comes later, with the dig rung.
pub const EMITS: usize = 3;
/// Hidden units in the job head ([`HID`]).
pub const HIDDEN: usize = 8;
const EMIT_CHANNELS: [Channel; EMITS] = [Channel::A, Channel::B, Channel::Alarm];

/// What the mind reads about the ant itself, once a decision.
pub mod c {
    /// Energy, `e / (1 + e)` of the start grant: 0.5 at birth.
    pub const ENERGY: usize = 0;
    /// How full the crop is.
    pub const CROP: usize = 1;
    /// Carrying a pellet of soil.
    pub const SPOIL: usize = 2;
    /// At the nest.
    pub const AT_NEST: usize = 3;
    /// Scent A, B and alarm where the head stands.
    pub const SCENT: usize = 4;
    /// ...and how much each moved since the last decision.
    pub const D_SCENT: usize = 7;
    /// How much energy moved since the last decision.
    pub const D_ENERGY: usize = 10;
    /// **The home vector's present flag**: 0 in a home-blind arena, so a
    /// blind ant reads "absent", not "at home".
    pub const HOME_SEEN: usize = 11;
    /// How far home is, and how much nearer or further since last time.
    pub const HOME_DIST: usize = 12;
    pub const D_HOME: usize = 13;
    /// Decisions since the ant last stood at the nest.
    pub const SINCE_NEST: usize = 14;
    pub const AGE: usize = 15;
    /// **A number of its own, fixed at birth**, so clones can split work by
    /// threshold (Bonabeau 1996).
    pub const SELF: usize = 16;
    /// Animals touching the head.
    pub const CROWD: usize = 17;
    /// **Rows below the ground as it was built**, at the head's column
    /// ([`depth_at`]). Read 0 for every mind until PR 2.
    pub const DEPTH: usize = 18;
    /// Ground overhead.
    pub const ROOFED: usize = 19;
    /// **Every cut in reach drops a roof cell** ([`super::roof_cut`]): 1
    /// when the ant has a Dig option and each one carries
    /// [`super::options::f::ROOF_DROP`]. **Why a context input and not
    /// only the target feature** (PR 2): a scorer unit can re-pick among a
    /// verb's targets but not stop the verb, since no trained job row
    /// reads its `best`, and at the nest every Dig target already scores
    /// at the floor (median -0.94 on round 5's four champions); the rim cut
    /// that drops the founding shaft's shoulder was the only Dig option in
    /// about half the roof cuts. The job has to see it.
    pub const ROOF_CUT: usize = 20;
    /// **Every put-down spot in reach is in the hole** ([`super::hole_drop`]):
    /// 1 when the ant has a Drop option and each carries
    /// [`super::options::f::CUT`]. Round 5's pellets back in the hole were
    /// every one a put-down just after a cut in the open, the ant standing
    /// in its own scrape with nowhere else to put the pellet; the target
    /// feature cannot re-pick when there is no other target.
    pub const HOLE_DROP: usize = 21;
}

/// Context inputs.
pub const CONTEXT: usize = 22;

/// The names of the context inputs, in order, for readouts and for mapping
/// a genome onto a later layout by name.
pub fn context_names() -> [&'static str; CONTEXT] {
    [
        "energy",
        "crop",
        "spoil",
        "at_nest",
        "a",
        "b",
        "alarm",
        "d_a",
        "d_b",
        "d_alarm",
        "d_energy",
        "home_seen",
        "home_dist",
        "d_home",
        "since_nest",
        "age",
        "self",
        "crowd",
        "depth",
        "roofed",
        "roof_cut",
        "hole_drop",
    ]
}

// --- the genome -------------------------------------------------------------
//
// One flat `Vec<f32>`, in these blocks. Every block is a weight matrix read
// row by row; the rig crosses whole rows ([`crossover_group`]).

/// Verb switches: a verb is offered only while its gene is above 0 (*Size,
/// cost and growth*: a new verb at score 0 would steal picks at once).
const SWITCH: usize = 0;
/// How sharp the scores are, `1 + g` held to [0.25, 4].
const SHARP: usize = SWITCH + VERBS;
/// The pull towards the verb chosen last time, held to [0, 1].
const PULL: usize = SHARP + 1;
/// Each memory unit's forgetting rate, `0.5 + g` held to [0, 0.995].
const RATE: usize = PULL + 1;
/// The scorer's per-decision part: context, memory and a bias into each unit.
const S_CTX: usize = RATE + MEMORY;
/// **The first slot that is a connection**: everything before it (the verb
/// switches, the sharpness, the pull and the memory rates) is a setting,
/// costs nothing and is never counted ([`MindGenome::live`]), so trimming
/// connections must not touch it (`train::mutate`).
pub const FIRST_LINK: usize = S_CTX;
/// **Which scorer unit a slot feeds**, if it is one of a unit's inputs
/// (context and memory, option features, verb), and [`scorer_output`] that
/// unit's output weight: a unit's inputs move a score only through it
/// (`train::no_prune_mask`, review of the `noprune=` design, 2026-10-21).
pub fn scorer_unit(slot: usize) -> Option<usize> {
    match slot {
        s if (S_CTX..S_FEAT).contains(&s) => Some((s - S_CTX) / S_IN),
        s if (S_FEAT..S_VERB).contains(&s) => Some((s - S_FEAT) / FEATURES),
        s if (S_VERB..S_OUT).contains(&s) => Some((s - S_VERB) / VERBS),
        _ => None,
    }
}
/// The output weight of scorer unit `u` ([`scorer_unit`]).
pub fn scorer_output(u: usize) -> usize {
    assert!(u < UNITS, "scorer unit {u}: there are {UNITS}");
    S_OUT + u
}
const S_IN: usize = CONTEXT + MEMORY + 1;
/// The scorer's per-option part: the option's features into each unit...
const S_FEAT: usize = S_CTX + UNITS * S_IN;
/// ...and which verb the option belongs to, so a step and a drop aimed at
/// the same free cell need not score the same.
const S_VERB: usize = S_FEAT + UNITS * FEATURES;
/// The units into one score, and its bias.
const S_OUT: usize = S_VERB + UNITS * VERBS;
/// The job head: one row per verb, reading context, memory, the best of the
/// verb's targets and a constant.
const JOB: usize = S_OUT + UNITS + 1;
const J_IN: usize = CONTEXT + MEMORY + 2;
/// Memory: each unit reads context, the best and the mean option score,
/// the verb just chosen, every memory unit and a bias.
const MEM: usize = JOB + VERBS * J_IN;
const M_IN: usize = CONTEXT + 2 + VERBS + MEMORY + 1;
/// Emits: each scent's strength from context, memory and a bias.
const EMIT: usize = MEM + MEMORY * M_IN;
const E_IN: usize = CONTEXT + MEMORY + 1;
/// **The job head's hidden units**: each reads this decision's context,
/// memory and a bias, squashed as a memory unit is (no sharpness), and
/// feeds every job ([`HID_OUT`]). **Why**: a job is a weighted sum, and "put
/// food down at the nest, never soil" is an XOR that no sum holds -- one
/// `Drop` serves both. Memory cannot carry it (it updates after the pick,
/// so the decision just after a cut puts the pellet down beside the face
/// about half the time), and a scorer unit read through a job's `best`
/// can veto but not raise: `best` sits at 0.95-0.99 in every state
/// (ladder lane, `dig-rung-design.md` 5; `review-brain-architecture.md`
/// E5-E8). Appended last, so every earlier slot keeps its index, and all
/// zero in a genome saved before them ([`pad`]).
///
/// **A unit nobody wired is a two-step valley for evolution**: its fan-in
/// does nothing until a job reads it (an unread unit is skipped), and a
/// job's weight on it does nothing while its fan-in is empty (`squash(0)`
/// is 0), yet each is a live link the brain is taxed for. So recruiting
/// one of units 2-7 takes two coincident mutations that selection removes
/// one at a time: "the net never used them" is not evidence they are
/// useless (review of PR 721, N5).
const HID: usize = EMIT + EMITS * E_IN;
const H_IN: usize = CONTEXT + MEMORY + 1;
/// Each job's weight on each hidden unit, one row of [`HIDDEN`] per verb.
const HID_OUT: usize = HID + HIDDEN * H_IN;
/// The whole genome.
pub const GENOME_LEN: usize = HID_OUT + VERBS * HIDDEN;
/// **Features per option before `f::LINED` and `f::ROOF_DROP`** (PR 2).
const FEATURES_V2: usize = 43;
/// **Context inputs before `c::ROOF_CUT` and `c::HOLE_DROP`** (PR 2).
const CONTEXT_V2: usize = 20;
/// What PR 2 added: two feature weights in every scorer unit, and one
/// context weight in every row of the five blocks that read context.
const GREW: usize =
    UNITS * (FEATURES - FEATURES_V2) + (CONTEXT - CONTEXT_V2) * (UNITS + VERBS + MEMORY + EMITS + HIDDEN);
/// **A genome saved with the job head's hidden units, before PR 2**
/// (2,669 slots).
pub const GENOME_LEN_V2: usize = GENOME_LEN - GREW;
/// **...and one saved before them** (2026-10-18; 2,333 slots): the hidden
/// units were appended last, so it is a V2 genome cut short. [`pad`]
/// reads it as one whose hidden units are all zero, which run exactly as
/// it did.
pub const GENOME_LEN_V1: usize = GENOME_LEN_V2 - HIDDEN * (CONTEXT_V2 + MEMORY + 1) - VERBS * HIDDEN;

/// A weight below this is not a connection: it costs nothing and is not
/// counted ([`MindGenome::live`]), as `brain::W_EPS` is for today's brain.
const W_EPS: f32 = 1.0e-6;

/// **The draw floor, on scores shifted to [0, 1]**: a choice at the bottom
/// of the range is drawn `(0.05 / 1.05)^2`, about 0.23%, as often as one at
/// the top. The design asks for 0.1-0.3% at full confidence: a lower floor
/// is an argmax, the recorded dead end P-10 (exploration dies), and a
/// higher one walked an ant into thin air 16% of the time (`dead-ends.md`,
/// step scoring "Added, not multiplied"). The genome sharpens the scores
/// ([`SHARP`]), never the floor.
pub const PICK_FLOOR: f32 = 0.05;

/// The rng slot of the mind's draws, beside `creature.rs`'s own.
const RNG_SLOT_MIND: u64 = 40;
/// ...and of each ant's own number.
const RNG_SLOT_SELF: u64 = 41;
/// ...and of what the act draws (whether a bitten seed survives the mouth).
const RNG_SLOT_ACT: u64 = 42;

/// A genome, with what it costs to run: the connections it holds.
///
/// **`w` is read-only once built**: [`MindGenome::plan`] is derived from it
/// here, so a caller that wants other weights builds a new genome rather
/// than editing this one (nothing in the tree writes `w` after `new`; the
/// field is public for reading).
#[derive(Clone, Debug)]
pub struct MindGenome {
    pub w: Vec<f32>,
    /// Weights at or above [`W_EPS`], switches, rates and the two bounds
    /// excluded: what the body pays for every decision, priced like today's
    /// brain's active synapses (`CreatureDef::synapse_fraction`).
    pub live: u32,
    /// The scorer's per-option part with its zero weights left out
    /// ([`ScorerPlan`]), built once per genome and shared by its clones.
    plan: ScorerPlan,
}

impl MindGenome {
    pub fn new(w: Vec<f32>) -> MindGenome {
        assert_eq!(
            w.len(),
            GENOME_LEN,
            "a mind genome is exactly GENOME_LEN; a short one means the layout changed"
        );
        let live = w[S_CTX..].iter().filter(|v| v.abs() >= W_EPS).count() as u32;
        let plan = ScorerPlan::new(&w);
        MindGenome { w, live, plan }
    }
}

/// **The per-option scorer with its zero weights left out** -- the same
/// sums [`decide`] takes over every one of a unit's [`FEATURES`] weights, in
/// the same order, skipping only the terms whose weight is exactly zero, and
/// skipping a unit whose output weight is exactly zero altogether.
///
/// **Why**: the per-option part is [`UNITS`] x [`FEATURES`] multiplies per
/// option, every option, every decision, and is most of what a decision
/// costs; a trained genome holds a few hundred live links over the whole
/// head, so most of those multiplies are by zero (performance lane,
/// 2026-10-21).
///
/// **Why it picks exactly as the dense sum does**: a skipped term is
/// `0 * x`, which is a zero for any finite `x` (every feature is a squash or
/// a flag, so finite: asserted in debug builds), and adding a zero to a sum
/// leaves its value unchanged. The only bit it can move is the sign of a sum
/// that is itself zero (`-0 + 0` is `+0`), and nothing downstream reads that
/// sign: the squash keeps it, products and sums with anything nonzero erase
/// it, and every comparison and the `(s + 1) / 2` shift treat `-0` as `0`.
/// The kept terms are added in their original order, so no rounding moves.
/// `ScorerPlan::dense` (tests only) is the dense sum as it was, and
/// `the_sparse_scorer_decides_exactly_as_the_dense_one` holds the two equal,
/// bit for bit, over sparse random genomes.
///
/// **All units at once, each in its own order** (2026-10-22): the plan
/// keeps a feature if any live unit weighs it, with every unit's weight on
/// it side by side, so one pass over the option's features adds each one
/// to all [`UNITS`] sums together -- work the compiler does several lanes
/// at a time. A unit's own sum still takes its terms in feature order, and
/// the terms it gains are `0 * x`, the zeros above, so nothing rounds
/// differently. The sparse walk it replaces took 184 `(feature, weight)`
/// steps one at a time for v2-C3, 8% of the mind colony's frame (ant cost
/// review, 2026-10-22), which priced a dense sum as a rule change on the
/// reading that it must add in a new order; across units it need not.
///
/// The live units sit in the first lanes, in unit order; a genome with
/// eight or fewer takes eight lanes rather than sixteen ([`score_lanes`]),
/// which is what keeps a sparse one (the trail arena's dig-4b4: 8 units,
/// 89 weights) from paying for lanes it does not use.
#[derive(Clone, Debug, Default)]
struct ScorerPlan {
    /// Units whose output weight is not zero, in unit order: lane `k` is
    /// unit `units[k]`.
    units: Vec<usize>,
    /// Features some unit of `units` weighs, in feature order, each with
    /// every lane's weight on it (zero where its unit does not weigh it,
    /// and in every lane past `units`).
    feats: Vec<(usize, [f32; UNITS])>,
}

impl ScorerPlan {
    fn new(g: &[f32]) -> ScorerPlan {
        let out = &g[S_OUT..S_OUT + UNITS];
        ScorerPlan::of((0..UNITS).filter(|&u| out[u] != 0.0).collect(), g)
    }

    fn of(units: Vec<usize>, g: &[f32]) -> ScorerPlan {
        let feats = (0..FEATURES)
            .filter(|&i| units.iter().any(|&u| g[S_FEAT + u * FEATURES + i] != 0.0))
            .map(|i| (i, std::array::from_fn(|k| units.get(k).map_or(0.0, |&u| g[S_FEAT + u * FEATURES + i]))))
            .collect();
        ScorerPlan { units, feats }
    }

    /// Every unit and every weight, zeros included: the sum as [`decide`]
    /// took it before the plan, kept as the reference the plan is held to.
    #[cfg(test)]
    fn dense(g: &[f32]) -> ScorerPlan {
        let feats = (0..FEATURES).map(|i| (i, std::array::from_fn(|u| g[S_FEAT + u * FEATURES + i]))).collect();
        ScorerPlan { units: (0..UNITS).collect(), feats }
    }
}

/// The first `N` lanes' sums for one option: each lane from its `start`,
/// over `feats` in their order.
#[inline]
fn score_lanes<const N: usize>(start: &[f32; UNITS], feats: &[(usize, [f32; UNITS])], x: &[f32; FEATURES]) -> [f32; UNITS] {
    let mut z = *start;
    for (i, w) in feats {
        let x = x[*i];
        for u in 0..N {
            z[u] += w[u] * x;
        }
    }
    z
}

/// **A saved genome in today's layout**, its length naming its layout. One
/// already [`GENOME_LEN`] long is returned as it is; `None` for a length
/// no layout had. **The length is the version only while no two layouts
/// share one**; a named-block file is the real fix when that stops holding
/// (review-brain-architecture.md F9). An older one picks exactly as it did:
///
/// - **Before the job head had hidden units** ([`GENOME_LEN_V1`]) it gains
///   them, all zero, which [`decide`] skips.
/// - **Before PR 2** ([`GENOME_LEN_V1`] and [`GENOME_LEN_V2`]) each scorer
///   unit gains `f::LINED` and `f::ROOF_DROP`, and every row that reads
///   context gains `c::ROOF_CUT` and `c::HOLE_DROP` at zero, so this is a
///   relayout, not a resize ([`v2_slots`]). Packed
///   soil read as `f::SPOIL` until then and reads as `f::LINED` now, so
///   each unit's `f::LINED` weight starts as its `f::SPOIL` weight: the wall
///   scores as it did, and only evolution can now tell it from a heap.
///   `f::ROOF_DROP` starts at zero.
/// - **Inputs that were constant until PR 2 wrote them are zeroed
///   wherever a weight reads them**: `c::DEPTH` and `f::DEPTH` (a mind's
///   depth read 0, `last_out_row` having no writer on its path) and
///   `f::CUT` (declared, never written). A weight on an input that never
///   moves drifts freely and costs nothing, and would act the moment the
///   input goes live (ladder lane, `dig-rung-design.md` 9, PR 2 item 1:
///   25 and 10 such live links in `col30-s11-seed33`). It is
///   [`enter_dig`]'s `c::SPOIL` reasoning, done once, for every start.
pub fn pad(g: Vec<f32>) -> Option<Vec<f32>> {
    match g.len() {
        GENOME_LEN => Some(g),
        GENOME_LEN_V1 | GENOME_LEN_V2 => {
            let mut v = vec![0.0; GENOME_LEN];
            for (w, to) in g.iter().zip(v2_slots()) {
                v[to] = *w;
            }
            for u in 0..UNITS {
                v[S_FEAT + u * FEATURES + options::f::LINED] = v[S_FEAT + u * FEATURES + options::f::SPOIL];
            }
            for k in newly_live() {
                v[k] = 0.0;
            }
            Some(v)
        }
        _ => None,
    }
}

/// **Where each slot of a genome saved before PR 2 lands today**, in the
/// order it was saved ([`GENOME_LEN_V2`] of them; a V1 genome is the first
/// [`GENOME_LEN_V1`]). Each block is read row by row; a row keeps its
/// leading slots where they were and moves the rest along by what the row
/// grew: a context-reading row grew `c::ROOF_CUT` and `c::HOLE_DROP` after
/// its 20 context inputs, a scorer unit's feature row two features at its
/// end.
fn v2_slots() -> Vec<usize> {
    let other = UNITS * VERBS + UNITS + 1;
    // (rows, the old width, slots that stay put, the new start, the new width)
    let blocks = [
        (1, S_CTX, S_CTX, 0, S_CTX),
        (UNITS, CONTEXT_V2 + MEMORY + 1, CONTEXT_V2, S_CTX, S_IN),
        (UNITS, FEATURES_V2, FEATURES_V2, S_FEAT, FEATURES),
        (1, other, other, S_VERB, other),
        (VERBS, CONTEXT_V2 + MEMORY + 2, CONTEXT_V2, JOB, J_IN),
        (MEMORY, CONTEXT_V2 + 2 + VERBS + MEMORY + 1, CONTEXT_V2, MEM, M_IN),
        (EMITS, CONTEXT_V2 + MEMORY + 1, CONTEXT_V2, EMIT, E_IN),
        (HIDDEN, CONTEXT_V2 + MEMORY + 1, CONTEXT_V2, HID, H_IN),
        (1, VERBS * HIDDEN, VERBS * HIDDEN, HID_OUT, VERBS * HIDDEN),
    ];
    let mut to = Vec::with_capacity(GENOME_LEN_V2);
    for (rows, old, kept, start, width) in blocks {
        for r in 0..rows {
            to.extend((0..old).map(|i| start + r * width + if i < kept { i } else { i + width - old }));
        }
    }
    debug_assert_eq!(to.len(), GENOME_LEN_V2);
    to
}

/// **Every slot that reads an input PR 2 brought to life** ([`pad`]):
/// `c::DEPTH` in each block that reads context, `f::DEPTH` and `f::CUT` in
/// each scorer unit.
fn newly_live() -> Vec<usize> {
    let mut out = Vec::new();
    for (base, rows, width) in [
        (S_CTX, UNITS, S_IN),
        (JOB, VERBS, J_IN),
        (MEM, MEMORY, M_IN),
        (EMIT, EMITS, E_IN),
        (HID, HIDDEN, H_IN),
    ] {
        out.extend((0..rows).map(|r| base + r * width + c::DEPTH));
    }
    for u in 0..UNITS {
        out.extend([options::f::DEPTH, options::f::CUT].map(|k| S_FEAT + u * FEATURES + k));
    }
    out
}

/// **The layout a genome file names in its header from fix plan step 1 on**
/// (`train::save_genome`: `genome 2797 layout=4`). **Why a mark**: step 1
/// moved the swallow from Eat to Pick up without changing the genome's
/// length, so the length can no longer say which a file is (review of the
/// actions audit, item 3). A file with no mark is read as layout 3 or
/// older and converted once ([`split_pick_up`]); a marked one is not.
/// Layouts 1-3 are [`GENOME_LEN_V1`], [`GENOME_LEN_V2`] and today's length
/// before step 1 ([`pad`]).
pub const LAYOUT: u32 = 4;

/// **A genome saved before Pick up had its own act, as one that has it**
/// (fix plan step 1; review of the actions audit, fix 1). Until then Eat
/// aimed at food in reach swallowed it, which is how every forager and
/// fetch champion picks up, and Pick up's genes did nothing but drift.
/// Every link Eat had is copied to Pick up -- its switch, its column in
/// every scorer unit, its job row, memory's "Eat was chosen" input and its
/// weights on the hidden units -- so a converted ant picks up where it
/// swallowed. Then **Eat is quieted**: its job row zeroed with its constant
/// at [`QUIET`], and its weights on the hidden units zeroed; otherwise a
/// converted champion eats its load wherever it used to pick up (trap 1 of
/// `trace-trained-ant.md`). Eat keeps its scorer column (it scores the one
/// eat-now target, which still feeds memory's best and mean) and memory's
/// read of it (an eat-now is still an eat). Quieting Eat starves nobody
/// while the crop digests on its own; the brood rung reopens it.
pub fn split_pick_up(g: &mut [f32]) {
    let (eat, pick) = (Verb::Eat as usize, Verb::PickUp as usize);
    g[SWITCH + pick] = g[SWITCH + eat];
    for u in 0..UNITS {
        g[S_VERB + u * VERBS + pick] = g[S_VERB + u * VERBS + eat];
    }
    g.copy_within(JOB + eat * J_IN..JOB + (eat + 1) * J_IN, JOB + pick * J_IN);
    for m in 0..MEMORY {
        let chose = MEM + m * M_IN + CONTEXT + 2;
        g[chose + pick] = g[chose + eat];
    }
    g.copy_within(HID_OUT + eat * HIDDEN..HID_OUT + (eat + 1) * HIDDEN, HID_OUT + pick * HIDDEN);
    g[JOB + eat * J_IN..JOB + (eat + 1) * J_IN].fill(0.0);
    g[JOB + eat * J_IN + CONTEXT + MEMORY + 1] = QUIET;
    g[HID_OUT + eat * HIDDEN..HID_OUT + (eat + 1) * HIDDEN].fill(0.0);
}

/// **Every link that belongs to one verb**: its column in each scorer unit,
/// its job row, memory's "it was chosen" input and its weights on the
/// hidden units. What drifts while the verb is banned: mutation moves
/// them and selection never sees them, yet each is billed as a live link.
fn verb_links(v: Verb) -> Vec<std::ops::Range<usize>> {
    let v = v as usize;
    let mut out: Vec<std::ops::Range<usize>> = (0..UNITS).map(|u| S_VERB + u * VERBS + v..S_VERB + u * VERBS + v + 1).collect();
    out.push(JOB + v * J_IN..JOB + (v + 1) * J_IN);
    out.push(HID_OUT + v * HIDDEN..HID_OUT + (v + 1) * HIDDEN);
    out.extend((0..MEMORY).map(|m| {
        let k = MEM + m * M_IN + CONTEXT + 2 + v;
        k..k + 1
    }));
    out
}

/// **A blank genome**: every weight 0, so every option scores the same, with
/// `verbs` switched on and the rest off.
pub fn blank(verbs: &[Verb]) -> Vec<f32> {
    let mut g = vec![0.0; GENOME_LEN];
    for v in Verb::ALL {
        g[SWITCH + v as usize] = if verbs.contains(&v) { 1.0 } else { -1.0 };
    }
    g
}

/// **The slots the rig may move**: every weight but the switches of verbs
/// the arena does not allow (held off) and the rows of scents it bans (held
/// at 0). A switch the arena allows is trainable: mutation can turn a verb
/// off, and back on.
pub fn trainable(verbs: &[Verb], emits: [bool; EMITS]) -> Vec<usize> {
    (0..GENOME_LEN)
        .filter(|&i| match i {
            i if i < SHARP => verbs.contains(&Verb::ALL[i - SWITCH]),
            i if (EMIT..HID).contains(&i) => emits[(i - EMIT) / E_IN],
            _ => true,
        })
        .collect()
}

/// **Every slot that serves verb `v` alone**: its switch, its job row
/// (context, memory, best target, constant), its row from the hidden units,
/// each scorer unit's weight on it, and each memory unit's weight on "this
/// verb was just chosen". Nothing else reads them, so in a world where `v`
/// never has a target they cannot change a single decision.
pub fn verb_slots(v: Verb) -> Vec<usize> {
    let v = v as usize;
    let mut out = vec![SWITCH + v];
    out.extend(JOB + v * J_IN..JOB + (v + 1) * J_IN);
    out.extend(HID_OUT + v * HIDDEN..HID_OUT + (v + 1) * HIDDEN);
    out.extend((0..UNITS).map(|u| S_VERB + u * VERBS + v));
    out.extend((0..MEMORY).map(|m| MEM + m * M_IN + CONTEXT + 2 + v));
    out
}

/// **Leave untestable verbs alone** (owner, 2026-10-20): drop from `slots`
/// every slot that serves only a verb in `verbs`, so mutation and pruning
/// never touch it. **Why**: a suite's kept arenas offer verbs its learning
/// arena never can (Eat while digging, with no food in the soil box), and
/// those verbs' links are then judged only by the kept arena's 0.8 floor
/// while the per-connection cost pays for deleting them. That is how s161's
/// run zeroed the eat job's constant of -10 ("don't eat the cargo") and
/// its fetch fell to 0.75 of s91's (investigation 2026-10-20, part 3).
/// Returns how many slots it took out.
pub fn protect(slots: &mut Vec<usize>, verbs: &[Verb]) -> usize {
    let mut keep: Vec<usize> = verbs.iter().flat_map(|&v| verb_slots(v)).collect();
    keep.sort_unstable();
    let before = slots.len();
    slots.retain(|i| keep.binary_search(i).is_err());
    before - slots.len()
}

/// Hold an arena's bans on `g`: the verbs it does not allow off, the scents
/// it bans silent.
pub fn apply_bans(g: &mut [f32], verbs: &[Verb], emits: [bool; EMITS]) {
    for v in Verb::ALL {
        if !verbs.contains(&v) {
            g[SWITCH + v as usize] = -1.0;
        }
    }
    for (e, &on) in emits.iter().enumerate() {
        if !on {
            g[EMIT + e * E_IN..EMIT + (e + 1) * E_IN].fill(0.0);
        }
    }
}

/// **How quiet a reopened verb comes back** ([`reopen`]): its job's
/// constant, so it is drawn about one decision in a hundred until mutation
/// wires it. A forage champion with turning round switched back on at its
/// own job constant (0) turned on a fifth of its decisions and its colony
/// fell from 2.0 to 0.8 on four hold-outs; at -10 it turned on 1.3% and
/// read 1.5 (`brain-training/stage2/forage-big/quiet`).
pub const QUIET: f32 = -10.0;

/// **Verbs an earlier rung switched off, back on, quietly**, for a
/// training run's start (not for scoring a champion, which must run as it
/// was trained): each verb `verbs` offers whose switch evolved to 0 or
/// below -- not the exact -1 a ban holds, which [`enter_rung`] owns -- comes
/// on with its job's constant at [`QUIET`] or lower. **Why**: the trail rung
/// switched turning round off, since it never paid there, and every forage
/// champion inherited it: an ant that cannot turn cannot carry food home.
/// A child whose switch mutates back on turns at random and scores worse,
/// so selection never kept one in 16 generations of 100 brains (owner,
/// 2026-10-18; the connection trim had also been zeroing switches,
/// `train::mutate`). Quiet, the verb costs little until it is wired to
/// something. Returns what it reopened, for the run's log.
pub fn reopen(g: &mut [f32], verbs: &[Verb]) -> Vec<&'static str> {
    let mut did = vec![];
    for &v in verbs {
        let s = SWITCH + v as usize;
        if v.acts() && g[s] <= 0.0 && g[s] != -1.0 {
            g[s] = 1.0;
            let k = JOB + v as usize * J_IN + CONTEXT + MEMORY + 1;
            g[k] = g[k].min(QUIET);
            did.push(v.name());
        }
    }
    did
}

/// **A rung's entry into a genome an earlier rung trained**: each verb
/// this rung offers that the earlier one banned is switched on (a banned
/// switch is held at exactly -1, [`trainable`]), at no weight, so it enters
/// as a blank verb does; and where this rung lets the ant lay scent B and
/// the genome has never laid it, B gets the hand-built layer: laid while
/// the crop holds food (design rev3, *Scents need senders and receivers
/// together*: today's brain has that rule by hand, so the new one starts
/// with it and mutation may change it); where Dig comes on, the dig
/// start ([`enter_dig`]); where Dig is on and the genome has never had
/// them, the soil vetoes ([`lay_vetoes`]); and on that same entry -- the one
/// that opens Dig or lays the vetoes -- the links that can never move a
/// decision zeroed ([`zero_dead`], R11). Returns what it changed, for the
/// run's log.
pub fn enter_rung(g: &mut [f32], verbs: &[Verb], emits: [bool; EMITS]) -> Vec<String> {
    let mut did = vec![];
    let mut opened = false;
    for v in Verb::ALL {
        if verbs.contains(&v) && g[SWITCH + v as usize] == -1.0 {
            g[SWITCH + v as usize] = 1.0;
            did.push(v.name().to_string());
            if v == Verb::Dig {
                did.extend(enter_dig(g));
                opened = true;
            }
            // Pick up and Attack had genes and no act, so every run moved
            // them and billed them (review of the actions audit, item 11):
            // they come on blank (fix plan step 1), as Dig's drift does.
            // **Eat too** (review of PR 724, N7): a trail genome saved after
            // step 1 is not converted, and without this its Eat came on
            // with whatever drifted onto it while banned. A converted one
            // whose Eat was banned comes on blank here too, constant 0, not
            // at the [`QUIET`] that [`split_pick_up`] left (N7a); only an
            // Eat that was already on, as every forager's is, stays quiet.
            // The cost: a forager sent back to a trail-only rung and then
            // to a food rung loses its trained Eat and Pick up here. No
            // suite in use takes that route (review of PR 724).
            if matches!(v, Verb::PickUp | Verb::Attack | Verb::Eat) {
                let mut zeroed = 0;
                for r in verb_links(v) {
                    zeroed += g[r.clone()].iter().filter(|w| **w != 0.0).count();
                    g[r].fill(0.0);
                }
                if zeroed > 0 {
                    did.push(format!("{zeroed} drifted {} links zeroed", v.name()));
                }
            }
        }
    }
    let b = EMIT + E_IN..EMIT + 2 * E_IN;
    if emits[1] && g[b.clone()].iter().all(|w| *w == 0.0) {
        // One cell (a sixth of the crop) lays at 0.79; nothing lays 0.
        g[b.start + c::CROP] = 40.0;
        g[b.end - 1] = -3.0;
        did.push("scent B while carrying".to_string());
    }
    // Keyed on the vetoes' own marks rather than on the ban, so a start that
    // entered the dig rung before the vetoes existed gets them too (step 4
    // starts from a step-3 champion). A genome saved before PR 2 has both
    // at zero ([`pad`]: `c::ROOF_CUT` is new), and [`enter_dig`] zeroes
    // what drifted onto either while Dig was banned. **Both, not the roof
    // column alone** (review of PR 722, N2): `prune=` can zero the roof
    // veto's one weight, and keyed on that alone the next entry (shaft 4 ->
    // 8) would take two more units and lay a second Dig read and a second
    // Drop gate over the first.
    let dig = verbs.contains(&Verb::Dig) && g[SWITCH + Verb::Dig as usize] > 0.0;
    let marked = (0..UNITS).any(|u| g[S_FEAT + u * FEATURES + options::f::ROOF_DROP] != 0.0)
        || g[JOB + Verb::Dig as usize * J_IN + c::ROOF_CUT] != 0.0;
    let laid = dig && !marked;
    if laid {
        did.extend(lay_vetoes(g));
    }
    // After the vetoes, which write every verb's column on their two units:
    // a banned verb's goes here. **Only on the entry that opens Dig or lays
    // the vetoes** (review of PR 722, S1): every `score=`, `start=`,
    // `noise=`, `trace=` and `pic=` load enters the rung, and training
    // regrows dead links (`grow=` makes a link on an empty slot, a dead
    // unit's included) that its hold-out paid for, so cutting them on every
    // entry scored a genome other than the one selection picked: a step-3
    // digger's hold-best read 71.25 in training and 67.00 through `score=`,
    // 24 links cut (519 -> 495).
    if opened || laid {
        let cut = zero_dead(g, verbs, emits);
        if cut.iter().any(|&n| n > 0) {
            did.push(format!(
                "dead links zeroed: {} ({} on scorer units nothing reads, {} on inputs nothing writes, {} on verbs never offered, {} on hidden and memory units nothing reads)",
                cut.iter().sum::<usize>(),
                cut[0],
                cut[1],
                cut[2],
                cut[3]
            ));
        }
    }
    did
}

/// **The dig start, for a genome whose Dig was banned until now** (ladder
/// lane, `dig-rung-design.md` 5, rev 2.1; keyed on the ban, R11):
///
/// * **what drifted, zeroed**: every weight that only ever met a zero while
///   Dig was banned -- Dig's job row and its weights on the hidden units,
///   the scorer's Dig column and its read of [`options::f::ROOF_DROP`]
///   (only a Dig option carries it), memory's read of "Dig was chosen",
///   and every read of `c::SPOIL` (nothing held soil). Mutation moved them and
///   selection never saw them; a drifted Dig weight on hidden unit 0 or 1
///   left in place would also read as "in use" and stop the gate below
///   being laid (review of PR 721, S1);
/// * **Dig's job**: at the nest, cut (`AT_NEST` +6, offset -4, the
///   forager's Drop numbers);
/// * **where soil goes down, on hidden units 0 and 1** when both are
///   empty: h0 is "soil, and at the nest or under cover", h1 "soil, and
///   far from home" (crosses 0 about 8 cells out). Drop reads h0 at -10
///   and h1 at +6, **added** to the start's own Drop row, and its offset
///   moves by what the two read with food at the nest (about -3.7), so a
///   forager's food put-down at the nest is where it was. A genome that
///   already uses either unit keeps them, and the log says the gate was
///   not laid.
///
/// The numbers are rev 1's, which in the review's toy corridor delivered
/// food at 1.30 (start 1.20), soil +0.95, and put 1 pellet in 64 down at
/// the nest from generation 0, the best start of every arm tried there
/// (`review-brain-architecture.md` E8). Returns what it changed.
fn enter_dig(g: &mut [f32]) -> Vec<String> {
    let mut drifted = verb_links(Verb::Dig);
    drifted.extend((0..UNITS).map(|u| {
        let k = S_FEAT + u * FEATURES + options::f::ROOF_DROP;
        k..k + 1
    }));
    for (base, rows, width) in [
        (S_CTX, UNITS, S_IN),
        (JOB, VERBS, J_IN),
        (MEM, MEMORY, M_IN),
        (EMIT, EMITS, E_IN),
        (HID, HIDDEN, H_IN),
    ] {
        drifted.extend((0..rows).map(|r| base + r * width + c::SPOIL..base + r * width + c::SPOIL + 1));
    }
    let mut zeroed = 0;
    for r in drifted {
        zeroed += g[r.clone()].iter().filter(|w| **w != 0.0).count();
        g[r].fill(0.0);
    }
    let job = |v: Verb, k: usize| JOB + v as usize * J_IN + k;
    g[job(Verb::Dig, c::AT_NEST)] = 6.0;
    g[job(Verb::Dig, CONTEXT + MEMORY + 1)] = -4.0;
    let mut did = vec![format!("{zeroed} drifted Dig and soil links zeroed"), "Dig at the nest".to_string()];

    let unit = |h: usize| HID + h * H_IN..HID + (h + 1) * H_IN;
    let read = |g: &[f32], h: usize| (0..VERBS).any(|v| g[HID_OUT + v * HIDDEN + h] != 0.0);
    if g[unit(0).start..unit(2).start].iter().all(|w| *w == 0.0) && !read(g, 0) && !read(g, 1) {
        let (h0, h1) = (unit(0), unit(1));
        g[h0.start + c::SPOIL] = 40.0;
        g[h0.start + c::AT_NEST] = 20.0;
        g[h0.start + c::ROOFED] = 20.0;
        g[h0.end - 1] = -45.0;
        g[h1.start + c::SPOIL] = 40.0;
        g[h1.start + c::HOME_DIST] = 40.0;
        g[h1.end - 1] = -48.0;
        let (w0, w1) = (-10.0, 6.0);
        let drop = HID_OUT + Verb::Drop as usize * HIDDEN;
        g[drop] += w0;
        g[drop + 1] += w1;
        // What h0 and h1 add to Drop with food at the nest in the open
        // (no soil, at home): put back, so that row reads as the start's.
        let at_nest = w0 * squash(20.0 - 45.0) + w1 * squash(-48.0);
        g[job(Verb::Drop, CONTEXT + MEMORY + 1)] -= at_nest;
        did.push(format!("soil put down away (hidden 0 and 1; Drop offset {:+.2})", -at_nest));
    } else {
        did.push("soil gate not laid: hidden 0 or 1 already in use".to_string());
    }
    did
}

/// **How hard each soil veto pushes** ([`lay_vetoes`]): the unit's output
/// weight, negated.
const DROP_VETO: f32 = 15.0;
const DIG_VETO: f32 = 40.0;
/// **How the two jobs see them** ([`lay_vetoes`]): the Drop job's weight on
/// its hidden unit for "soil, and only the hole", and the Dig job's weight
/// on [`c::ROOF_CUT`].
const DROP_HOLE: f32 = -10.0;
const DIG_ROOF_CUT: f32 = -20.0;

/// **The soil vetoes, on the two highest scorer units nothing reads**
/// (ladder lane, `dig-rung-design.md` 9.5; step 3's two misses,
/// `stage2/dig/README.md` round 5):
///
/// * **no soil put down in the hole**: a Drop target in the hole
///   ([`options::f::CUT`]) while the ant carries soil scores low. Round 5's
///   gated ants put 0.17-0.83 pellets a world back, every one into the
///   one-row surface scrape beside them, where their own depth reads 0
///   (`probes/round5/refills/`);
/// * **no cut that drops a roof cell** ([`options::f::ROOF_DROP`]): 65 of
///   the 83 grains that fell into the founding chamber came from one rim
///   cell, whose cut drops the shaft's shoulder (`probes/round5/falls/`).
///
/// A scorer unit can only re-pick among one verb's targets: a vetoed
/// target is drawn about as often as the pick floor allows. **Stopping the
/// verb takes the job**, and no trained job row read `best` for either
/// verb (both at exactly 0 on round 5's four champions), so laid on the
/// unit alone the two vetoes moved neither number: over seeds 71-74, 12
/// hold-out and 12 sealed worlds each, pellets back 4.0 -> 4.5 a seed
/// summed, falls 0.96 -> 0.78 a world, every pellet into the hole a
/// decision just after a cut in the open with every Drop target in the
/// hole (`probes/round5/vetoes/`). So each job is wired to see it:
///
/// * **Drop reads a job-head hidden unit** at -10: "soil, and every
///   put-down spot is in the hole" (`c::SPOIL` +40, [`c::HOLE_DROP`] +40,
///   bias -60, past the rig's bound as h0's and h1's are; a mutation there
///   clamps it and selection has kept theirs). Off, the unit still adds
///   9.5-9.8 to Drop, by state; the job's offset and its own reads of the
///   two inputs take that back exactly in all three off states, since a
///   leak centred on food at the nest alone put 1.4x the food down away
///   in the fetch arena. **Not `best`**: read at +6 it cut the colony's food
///   by 9% on the same four starts, since with food at the nest under
///   cover the best Drop target already scores near the floor half the
///   time (P(Drop) there 0.32 -> 0.07 on seed 72);
/// * **Dig reads [`c::ROOF_CUT`]** at -20. Its `best` cannot carry the
///   veto: at the nest every Dig target already scores at the floor.
///
/// **The Drop veto is a three-way AND in one sum**, which is why it
/// leaks: soil +30 and the hole +30, against -45 for a Drop and -80 for
/// any other verb (a bias of -40 plus the verb's own weight; the rig holds
/// every weight to 40). So it reads +15 only for soil, Drop and the hole,
/// -15 for a Drop with one of the two, and for every other verb -80 with
/// neither, -50 with one and -20 with both, so walking in the shaft with a
/// pellet is not vetoed. It is not untouched either (review of PR 722, N1):
/// such a target scores 0.53 below the leak level (`-15 * squash(-20)`
/// against `-15 * squash(-80)`) and an out-of-hole one with soil 0.11
/// below, so a carrier's targets in the hole sit 0.42 under its targets
/// out of it, all alike: none in the hole is re-ranked against another, and
/// at the rim it leans a carrier out, the way the veto wants. Off, it
/// still adds 0.94-0.99 of its weight to every option; the score's bias
/// takes back the level of the commonest option (no soil, out of the
/// hole, not a Drop, -80), which leaves a Drop with food into the hole
/// 0.75 below where it was (out of a raw score in the tens, where the
/// food put-down at the nest saturates).
///
/// **The Dig veto does not leak**: [`options::f::ROOF_DROP`] is only ever
/// set on a Dig option, so the unit reads 40 on a roof-dropping cut and
/// exactly 0 on everything else.
///
/// A unit whose output weight is 0 has fan-in that buys nothing but its
/// links' cost; the two taken are cleared first. Returns what it did.
fn lay_vetoes(g: &mut [f32]) -> Vec<String> {
    use options::f;
    let free: Vec<usize> = (0..UNITS).rev().filter(|&u| g[S_OUT + u] == 0.0).take(2).collect();
    let &[drop, dig] = free.as_slice() else {
        return vec!["soil vetoes not laid: fewer than two scorer units unread".to_string()];
    };
    let mut cleared = 0;
    for u in [drop, dig] {
        for r in [
            S_CTX + u * S_IN..S_CTX + (u + 1) * S_IN,
            S_FEAT + u * FEATURES..S_FEAT + (u + 1) * FEATURES,
            S_VERB + u * VERBS..S_VERB + (u + 1) * VERBS,
        ] {
            cleared += g[r.clone()].iter().filter(|w| **w != 0.0).count();
            g[r].fill(0.0);
        }
    }
    let (wd, wg) = (DROP_VETO, DIG_VETO);
    // Soil, a Drop, into the hole.
    g[S_CTX + drop * S_IN + c::SPOIL] = 30.0;
    g[S_CTX + drop * S_IN + S_IN - 1] = -40.0;
    g[S_FEAT + drop * FEATURES + f::CUT] = 30.0;
    for v in Verb::ALL {
        g[S_VERB + drop * VERBS + v as usize] = if v == Verb::Drop { -5.0 } else { -40.0 };
    }
    g[S_OUT + drop] = -wd;
    // A cut that drops a roof cell.
    g[S_FEAT + dig * FEATURES + f::ROOF_DROP] = 40.0;
    g[S_OUT + dig] = -wg;
    let leak = -wd * squash(-80.0);
    g[S_OUT + UNITS] -= leak;
    // The jobs, added to what the start has.
    let job = |v: Verb, k: usize| JOB + v as usize * J_IN + k;
    g[job(Verb::Dig, c::ROOF_CUT)] += DIG_ROOF_CUT;
    let unit = |h: usize| HID + h * H_IN..HID + (h + 1) * H_IN;
    let read = |g: &[f32], h: usize| (0..VERBS).any(|v| g[HID_OUT + v * HIDDEN + h] != 0.0);
    let gate = match (0..HIDDEN).find(|&h| g[unit(h)].iter().all(|w| *w == 0.0) && !read(g, h)) {
        Some(h) => {
            g[unit(h).start + c::SPOIL] = 40.0;
            g[unit(h).start + c::HOLE_DROP] = 40.0;
            g[unit(h).end - 1] = -60.0;
            g[HID_OUT + Verb::Drop as usize * HIDDEN + h] += DROP_HOLE;
            // What the unit adds to Drop in each of its three off states,
            // taken back by the offset and the job's own reads of the two
            // inputs, so food anywhere and soil with somewhere else to go
            // sum exactly as they did.
            let none = DROP_HOLE * squash(-60.0);
            let one = DROP_HOLE * squash(-20.0);
            g[job(Verb::Drop, CONTEXT + MEMORY + 1)] -= none;
            g[job(Verb::Drop, c::SPOIL)] += none - one;
            g[job(Verb::Drop, c::HOLE_DROP)] += none - one;
            format!(
                "Drop off with soil and only the hole (hidden {h}; Drop offset {:+.2})",
                -none
            )
        }
        None => "Drop's hole gate not laid: no hidden unit free".to_string(),
    };
    vec![format!(
        "soil vetoes: Drop into the hole on unit {drop}, roof-dropping cuts on unit {dig} ({cleared} unread links cleared; score bias {:+.2}); Dig off when every cut drops the roof; {gate}",
        -leak
    )]
}

/// **Inputs nothing writes yet**, 0 on every option: the building scent
/// (it comes with a building emit) and the way back to the last food (a
/// body memory not kept yet). A weight on one moves nothing and is billed
/// every tick.
const UNWRITTEN: [usize; 2] = [options::f::BUILDING, options::f::LAST_FOOD];

/// **Links that can never move a decision, zeroed** (R11,
/// `dig-rung-design.md` 9 item 3), from [`enter_rung`] on the entry that
/// opens Dig or lays the soil vetoes, never on a later one.
/// Every live link is billed every tick (`synapse_fraction`) and the dig
/// rung runs on a budget (R13), so a link that only costs is a bill to
/// cut: about a third of the links in the colony starts, 0.18-0.20 J a
/// tick, twice what an idle ant burns (`stage2/colony`, where
/// `probes/prune_dead.py` cut them by hand and fetch and trail stayed
/// byte-identical). Cut:
///
/// * a scorer unit whose output weight is 0: its whole fan-in;
/// * an input nothing writes ([`UNWRITTEN`]), on every scorer unit;
/// * a verb the rung never offers: the scorer's weight on it, its job's
///   reads (not its offset: a ban lifted later brings the verb back at the
///   offset it has, and at 0 it would be drawn about half the time,
///   [`QUIET`]'s reason), its weights on the hidden units, and memory's
///   read of "it was chosen";
/// * a hidden unit no job reads, and a memory unit nothing but itself
///   reads, until none is left: clearing one can leave another unread.
///
/// **Left alone**: inputs constant only in today's arenas (`c::SPOIL` out
/// of soil, the larva senses where nothing hatches), since which arenas a
/// rung runs is the trainer's to say and the soil gate reads `c::SPOIL` by
/// hand; and the emits, which [`apply_bans`] clears when banned.
///
/// Every pick, score, job, emit and memory still read stays bit-identical
/// (`zeroing_dead_links_leaves_every_choice_bit_identical`); only the bill
/// moves. Returns the links cut of each kind, in the order above.
fn zero_dead(g: &mut [f32], verbs: &[Verb], emits: [bool; EMITS]) -> [usize; 4] {
    // Zeroes `ks`, counting the live ones in `n`; whether any was not 0.
    fn kill(g: &mut [f32], ks: impl IntoIterator<Item = usize>, n: &mut usize) -> bool {
        let mut any = false;
        for k in ks {
            any |= g[k] != 0.0;
            *n += usize::from(g[k].abs() >= W_EPS);
            g[k] = 0.0;
        }
        any
    }
    let mut cut = [0; 4];
    let unread: Vec<usize> = (0..UNITS).filter(|&u| g[S_OUT + u] == 0.0).collect();
    for u in unread {
        let fan_in = (S_CTX + u * S_IN..S_CTX + (u + 1) * S_IN)
            .chain(S_FEAT + u * FEATURES..S_FEAT + (u + 1) * FEATURES)
            .chain(S_VERB + u * VERBS..S_VERB + (u + 1) * VERBS);
        kill(g, fan_in, &mut cut[0]);
    }
    for u in 0..UNITS {
        kill(g, UNWRITTEN.map(|k| S_FEAT + u * FEATURES + k), &mut cut[1]);
    }
    for v in Verb::ALL.into_iter().filter(|v| !verbs.contains(v)).map(|v| v as usize) {
        let job = JOB + v * J_IN;
        let reads = (0..UNITS)
            .map(|u| S_VERB + u * VERBS + v)
            .chain(job..job + CONTEXT + MEMORY + 1)
            .chain((0..HIDDEN).map(|h| HID_OUT + v * HIDDEN + h))
            .chain((0..MEMORY).map(|m| MEM + m * M_IN + CONTEXT + 2 + v));
        kill(g, reads, &mut cut[2]);
    }
    loop {
        let mut again = false;
        for h in 0..HIDDEN {
            if (0..VERBS).all(|v| g[HID_OUT + v * HIDDEN + h] == 0.0) {
                again |= kill(g, HID + h * H_IN..HID + (h + 1) * H_IN, &mut cut[3]);
            }
        }
        for m in 0..MEMORY {
            let at = CONTEXT + m;
            let read = (0..UNITS).any(|u| g[S_CTX + u * S_IN + at] != 0.0)
                || (0..VERBS).any(|v| g[JOB + v * J_IN + at] != 0.0)
                || (0..HIDDEN).any(|h| g[HID + h * H_IN + at] != 0.0)
                || (0..EMITS).any(|e| emits[e] && g[EMIT + e * E_IN + at] != 0.0)
                || (0..MEMORY).any(|r| r != m && g[MEM + r * M_IN + CONTEXT + 2 + VERBS + m] != 0.0);
            if !read {
                again |= kill(g, MEM + m * M_IN..MEM + (m + 1) * M_IN, &mut cut[3]);
            }
        }
        if !again {
            break;
        }
    }
    cut
}

/// The scent reads among the context inputs.
const SCENT_CTX: [usize; 6] = [
    c::SCENT,
    c::SCENT + 1,
    c::SCENT + 2,
    c::D_SCENT,
    c::D_SCENT + 1,
    c::D_SCENT + 2,
];

/// **The senses-cut control, as the trail rung needs it: every scent read
/// zeroed, and nothing else** -- each option's scent features and the
/// scent in the context, wherever a weight reads them. A walker that keeps
/// its heading (by `back`, its memory or its job's pull) keeps doing it
/// blind and scores the same; only an ant that reads the trail loses.
/// Cutting every sense would turn that walker into a random one and read
/// as "it reads the world" (ladder lane, `trail-rung-controls.md`).
pub fn blinded(g: &[f32]) -> Vec<f32> {
    let mut v = g.to_vec();
    let scent_feats = options::f::SCENT..=options::f::BUILDING;
    for u in 0..UNITS {
        for k in SCENT_CTX {
            v[S_CTX + u * S_IN + k] = 0.0;
        }
        for k in scent_feats.clone() {
            v[S_FEAT + u * FEATURES + k] = 0.0;
        }
    }
    for (base, rows, width) in [(JOB, VERBS, J_IN), (MEM, MEMORY, M_IN), (EMIT, EMITS, E_IN), (HID, HIDDEN, H_IN)] {
        for r in 0..rows {
            for k in SCENT_CTX {
                v[base + r * width + k] = 0.0;
            }
        }
    }
    v
}

/// **Hand-built minds, for the controls each rung is measured against**
/// (ladder lane, `trail-rung-controls.md`), over the verbs in `verbs`:
///
/// * `blank` -- every weight 0: the random null;
/// * `rest` -- rest alone switched on: an ant that never moves, the floor;
/// * `follower` -- walks where scent B grows (there, and two cells on),
///   turns round when it grows behind, keeps off ledges: the positive
///   control a trail-following score must reward;
/// * `persist` -- walks on in whatever way it faces and never turns or
///   rests: the walker that half its clones carry the right way by luck,
///   which a score must not reward like a follower.
///
/// `None` for any other name.
/// **The `forager` preset's egg bar switched off**, for an arena whose ant
/// cannot starve: a fed ant banks 1e9 J, which the energy sense reads as a
/// full tank many times over (`CreatureDef::sensed_tank`), so the preset's
/// "past the egg bar, go home and feed" flag is on from the first frame and
/// it never forages. Before 2026-10-22 the sense read a fed ant's bank
/// against the 1e9 budget, a flat 0.5, and the flag stayed off by accident.
#[cfg(test)]
pub(crate) fn below_the_egg_bar(g: &mut [f32]) {
    g[MEM + 2 * M_IN + c::ENERGY] = 0.0;
}

pub fn preset(name: &str, verbs: &[Verb]) -> Option<Vec<f32>> {
    use options::f;
    let mut g = blank(verbs);
    let job = |v: Verb, k: usize| JOB + v as usize * J_IN + k;
    let (best, offset) = (CONTEXT + MEMORY, CONTEXT + MEMORY + 1);
    match name {
        "blank" => {}
        "rest" => g = blank(&[Verb::Rest]),
        "follower" => {
            g[S_FEAT + f::SCENT + 4] = 4.0; // b, stronger there
            g[S_FEAT + f::SCENT + 5] = 4.0; // ...and two cells on
            g[S_FEAT + f::FALL] = -4.0;
            g[S_OUT] = 3.0;
            g[job(Verb::Walk, best)] = 3.0;
            g[job(Verb::TurnAround, best)] = 3.0;
            g[job(Verb::Rest, offset)] = -2.0;
        }
        "persist" => {
            g[S_FEAT + f::FALL] = -4.0;
            g[S_OUT] = 3.0;
            g[job(Verb::Walk, offset)] = 3.0;
            g[job(Verb::TurnAround, offset)] = -3.0;
            g[job(Verb::Rest, offset)] = -3.0;
        }
        // **The forage rung's hand-built forager** (`challenges::Scarce`):
        // home while the crop holds food or the bank could pay for an egg,
        // otherwise on the way it faces, wall to wall, and up out of the
        // nest; pick food up in reach, put food down at the nest, feed a larva
        // before a nestmate. The pass arm the rung needs before anything
        // trains. Every weight inside the rig's bound of 40 but the score's
        // bias, so a trained mind could reach it.
        //
        // **What it scores, and the three ways it starved before.** On the
        // 80-column bed, 8 hold-out layouts, worse side: 2.75 (6.25 adults,
        // 0.40 of founders kept), against the game ant's 16.3 and 0 for a
        // blank mind. Traced ant by ant (2026-10-17):
        // * **"The way it faces" was not in the weights.** A two-cell body
        //   has eight steps, and back into its own neck is one of them
        //   (`f::BACK`); scored like the forward step it made a random walk
        //   that never left 50 columns of the nest, 0 eaten. Unit 2 now
        //   forbids it, and a turn round, which also lands there, is let
        //   off. The rest of the reversals come in through the draw floor.
        // * **A sweep that ends at 96 columns and turns home** bounced
        //   between the wall and the nest on the side it started, and half
        //   the colony starts on the empty side; now it sweeps wall to wall.
        // * **Down the shaft, home is everywhere**: an ant that walked in
        //   spent 2,600 frames there. Unit 5 pulls it up at the nest, where
        //   unit 4 stops charging for the climb.
        // Laying scent B on the way home and following it out scored 0.38:
        // tried and dropped.
        //
        // **Why the memory chain.** A unit is `squash` of a sum, so a gate
        // is a sum pushed far from 0 for one state and near 0 for the other;
        // a graded input (one crop cell is 1/6, the egg bar is 0.846 on the
        // energy scale) cannot do that inside the bound. So the first
        // version, gated on the crop directly, pulled home by about 0.3 of a
        // score when carrying and away by 1.1 when not: a random walk that
        // carried the food it found into its own bank and climbed the far
        // wall (3.0 a side). Here memory squares each into a flag of +-1 a
        // decision or two late, and one flag gates the home unit.
        "forager" => {
            let unit = |u: usize| (S_CTX + u * S_IN, S_FEAT + u * FEATURES);
            let mem = |m: usize| MEM + m * M_IN;
            let mem_in = CONTEXT + 2 + VERBS;
            for m in 0..5 {
                g[RATE + m] = -0.5; // no forgetting: each is this decision's
            }
            // The top sharpness, 4.
            g[SHARP] = 3.0;
            // 0 -> 1: carrying anything (-0.75 / +0.79), squared to +-0.97.
            g[mem(0) + c::CROP] = 40.0;
            g[mem(0) + M_IN - 1] = -3.0;
            g[mem(1) + mem_in] = 40.0;
            // 2 -> 3: the bank past the egg bar (1,100 J reads 0.846).
            g[mem(2) + c::ENERGY] = 40.0;
            g[mem(2) + M_IN - 1] = -34.0;
            g[mem(3) + mem_in + 2] = 40.0;
            g[mem(3) + M_IN - 1] = -10.0;
            // 4: either.
            g[mem(4) + mem_in + 1] = 40.0;
            g[mem(4) + mem_in + 3] = 40.0;
            g[mem(4) + M_IN - 1] = 38.0;
            // 0: food in reach scores above anything beside it.
            g[unit(0).1 + f::FOOD] = 6.0;
            g[S_OUT] = 3.0;
            // 1: home, only under the flag (off at -39 without it).
            g[unit(1).1 + f::HOME] = 6.0;
            g[unit(1).0 + CONTEXT + 4] = 20.0;
            g[unit(1).0 + CONTEXT + MEMORY] = -20.0;
            g[S_OUT + 1] = 4.0;
            // 2: never step back into its own neck; a turn round, which
            // also lands there, is the verb for reversing.
            g[unit(2).1 + f::BACK] = -40.0;
            g[S_VERB + 2 * VERBS + Verb::TurnAround as usize] = 40.0;
            g[S_OUT + 2] = 20.0;
            // 3: a larva, not a grown nestmate.
            g[unit(3).1 + f::LARVA] = 8.0;
            g[unit(3).1 + f::NESTMATE] = -4.0;
            g[S_OUT + 3] = 3.0;
            // 4: not off a ledge, not up a wall -- except at the nest, where
            // the way out is up the shaft.
            g[unit(4).1 + f::FALL] = -6.0;
            g[unit(4).1 + f::CLIMB] = -4.0;
            g[unit(4).0 + c::AT_NEST] = 10.0;
            g[S_OUT + 4] = 6.0;
            // 5: up and out at the nest, only without the flag.
            g[unit(5).1 + f::UP] = 8.0;
            g[unit(5).0 + c::AT_NEST] = 20.0;
            g[unit(5).0 + CONTEXT + 4] = -20.0;
            g[unit(5).0 + CONTEXT + MEMORY] = -39.0;
            g[S_OUT + 5] = 4.0;
            // An off unit reads -1, not 0: the score's bias puts back the 4
            // each of units 1 and 5 takes from every option, or every step
            // scores below a rest.
            g[S_OUT + UNITS] = 10.0;
            // Walk while a step scores; turn round when none does.
            g[job(Verb::Walk, best)] = 6.0;
            g[job(Verb::Walk, offset)] = 2.0;
            g[job(Verb::TurnAround, best)] = 3.0;
            g[job(Verb::TurnAround, offset)] = -6.0;
            g[job(Verb::Rest, offset)] = -3.0;
            // Pick up what is in reach; eat from the crop hardly ever (fix
            // plan step 1: these were Eat's, which then also swallowed, and
            // Eat is quiet as [`split_pick_up`] leaves a converted ant).
            g[job(Verb::PickUp, best)] = 4.0;
            g[job(Verb::PickUp, offset)] = -1.0;
            g[job(Verb::Eat, offset)] = QUIET;
            g[job(Verb::Drop, c::AT_NEST)] = 6.0;
            g[job(Verb::Drop, offset)] = -4.0;
            // Feed while the bank is past the egg bar.
            g[job(Verb::Feed, best)] = 4.0;
            g[job(Verb::Feed, CONTEXT + 3)] = 4.0;
            g[job(Verb::Feed, offset)] = -2.0;
        }
        _ => return None,
    }
    Some(g)
}

/// **Which slots cross over together**: one scorer unit's whole fan-in and
/// its output weight; one job row; one memory row with its rate; one emit
/// row; one job hidden unit's fan-in with its weight into every job; each
/// switch and bound on its own. The rig takes each group whole
/// from one parent, as it takes today's brain's output rows.
pub fn crossover_group(slot: usize) -> usize {
    const UNIT_BASE: usize = 1000;
    match slot {
        s if s < RATE => s,
        s if s < S_CTX => UNIT_BASE + 100 + (s - RATE),
        s if s < S_FEAT => UNIT_BASE + (s - S_CTX) / S_IN,
        s if s < S_VERB => UNIT_BASE + (s - S_FEAT) / FEATURES,
        s if s < S_OUT => UNIT_BASE + (s - S_VERB) / VERBS,
        s if s < S_OUT + UNITS => UNIT_BASE + (s - S_OUT),
        s if s < JOB => UNIT_BASE + 200,
        s if s < MEM => UNIT_BASE + 300 + (s - JOB) / J_IN,
        s if s < EMIT => UNIT_BASE + 100 + (s - MEM) / M_IN,
        s if s < HID => UNIT_BASE + 400 + (s - EMIT) / E_IN,
        s if s < HID_OUT => UNIT_BASE + 500 + (s - HID) / H_IN,
        s => UNIT_BASE + 500 + (s - HID_OUT) % HIDDEN,
    }
}

/// One line on a genome: its connections, its switches and its bounds.
pub fn describe(g: &[f32]) -> String {
    let on: Vec<&str> = Verb::ALL
        .iter()
        .filter(|v| g[SWITCH + **v as usize] > 0.0)
        .map(|v| v.name())
        .collect();
    let live = |a: usize, b: usize| g[a..b].iter().filter(|v| v.abs() >= W_EPS).count();
    format!(
        "verbs [{}] sharp {:.2} pull {:.2} | live: scorer ctx {} feat {} verb {} out {}, job {}, memory {}, emit {}, job hidden {} (of {})",
        on.join(","),
        sharpness(g),
        pull(g),
        live(S_CTX, S_FEAT),
        live(S_FEAT, S_VERB),
        live(S_VERB, S_OUT),
        live(S_OUT, JOB),
        live(JOB, MEM),
        live(MEM, EMIT),
        live(EMIT, HID),
        live(HID, GENOME_LEN),
        GENOME_LEN - S_CTX,
    )
}

fn sharpness(g: &[f32]) -> f32 {
    (1.0 + g[SHARP]).clamp(0.25, 4.0)
}

fn pull(g: &[f32]) -> f32 {
    g[PULL].clamp(0.0, 1.0)
}

fn rate(g: &[f32], m: usize) -> f32 {
    (0.5 + g[RATE + m]).clamp(0.0, 0.995)
}

fn squash(x: f32) -> f32 {
    x / (1.0 + x.abs())
}

/// **How deep `(x, y)` is**: rows below the top of the ground in its column
/// as the bed was built (`World::room_surface_at`), negative above it,
/// squashed at 16 rows; 0 where no datum is frozen (no box, or a column
/// with no ground).
///
/// **Why this datum**: it is frozen on the first frame and again, earlier,
/// just before the founding cut (`cut_founding_shaft_with` freezes it
/// first), so the founding shaft's own column reads the surface it was cut
/// from, and nothing an ant digs later moves it. It is the datum the nest
/// census reads, so "deep" means here what "under the original ground"
/// means there. It is right on bare ground too: there is no founding cut
/// for it to need (ladder lane, `dig-rung-design.md` R3).
///
/// **What it replaces**: both depth reads took `last_out_row`, whose one
/// writer (`depth_slowed`) is on the game ant's path behind `DEPTH_SLOW`
/// (off), so a mind's depth read 0 everywhere -- a sense with no writer
/// (`.claude/rules/src-sim-cells.md`, the channel rule).
pub(crate) fn depth_at(world: &World, x: i32, y: i32) -> f32 {
    world.room_surface_at(x).map_or(0.0, |s| squash((y - s) as f32 / 16.0))
}

/// **Whether every cut in reach drops a roof cell** ([`c::ROOF_CUT`]): 1
/// when `opts` holds a Dig option and each carries
/// [`options::f::ROOF_DROP`], else 0. Read off the options, so it is 0
/// wherever the roof sense is not read (`options::build_with`).
pub(crate) fn roof_cut(opts: &[Opt]) -> f32 {
    every(opts, Verb::Dig, options::f::ROOF_DROP)
}

/// **Whether every put-down spot in reach is in the hole**
/// ([`c::HOLE_DROP`]): [`roof_cut`]'s reading of Drop and
/// [`options::f::CUT`].
pub(crate) fn hole_drop(opts: &[Opt]) -> f32 {
    every(opts, Verb::Drop, options::f::CUT)
}

fn every(opts: &[Opt], verb: Verb, feature: usize) -> f32 {
    let mut of = opts.iter().filter(|o| o.verb == verb).peekable();
    let all = of.peek().is_some() && of.all(|o| o.feats[feature] == 1.0);
    f32::from(u8::from(all))
}

/// The verbs whose act has landed ([`act`]); the rest are never picked.
/// Pick up landed with fix plan step 1, taking the swallow from Eat.
impl Verb {
    pub fn acts(self) -> bool {
        matches!(
            self,
            Verb::Walk | Verb::TurnAround | Verb::Rest | Verb::PickUp | Verb::Eat | Verb::Drop | Verb::Feed | Verb::Dig
        )
    }
}

// --- the ant's own memory -------------------------------------------------

/// What one ant's mind carries from one decision to the next.
#[derive(Clone, Copy, Debug)]
pub struct MindState {
    pub mem: [f32; MEMORY],
    /// The verb chosen last time, for the job head's pull.
    pub last_verb: Option<Verb>,
    /// Scent A, B, alarm, energy and home distance at the last decision,
    /// for the change senses. `None` before the first.
    pub last: Option<[f32; 5]>,
}

impl Default for MindState {
    fn default() -> Self {
        MindState {
            mem: [0.0; MEMORY],
            last_verb: None,
            last: None,
        }
    }
}

/// One ant's mind: its genome (shared by its clones) and its own state.
#[derive(Clone, Debug)]
pub struct Mind {
    pub genome: Arc<MindGenome>,
    pub state: MindState,
}

/// What every mind did, summed, for a harness to read: "did it fire at all"
/// needs a counter, not a picture (`CLAUDE.md`).
#[derive(Clone, Copy, Debug, Default)]
pub struct MindCounts {
    pub decisions: u64,
    pub falls: u64,
    /// Options built, and the verbs chosen.
    pub options: u64,
    pub chose: [u64; VERBS],
    /// **Decisions at which each verb had at least one target**: the verbs
    /// that can act at all in this world. A verb an arena lists but never
    /// offers (Eat in the soil box, which has no food) cannot be tested
    /// there, so the trainer leaves its connections alone
    /// (`train::Challenge::acting_verbs`, `Settings::protect`).
    pub offered: [u64; VERBS],
    /// Picks whose act did nothing (a step the body could not take after
    /// all). Should stay 0: the list is built from the body's own checks.
    pub failed: u64,
    /// Scent laid, in [`pheromone::DEPOSIT`] units.
    pub laid: [f64; EMITS],
    /// Food: cells picked up into the crop, crop cells put down (and of
    /// those, at the nest), cells eaten from the crop by choice ([`eat_now`];
    /// the crop also digests on its own, which only `creature_stats.eats`
    /// sees), and feeds -- to a nestmate's bank, to a larva from the crop,
    /// to a larva from the bank.
    pub swallowed: u64,
    pub put_down: u64,
    pub delivered: u64,
    pub fed_adult: u64,
    pub fed_larva_crop: u64,
    pub fed_larva_bank: u64,
    pub ate: u64,
    /// Soil: cells cut into the jaws, and pellets put down.
    pub dug: u64,
    /// Pack moves made: a walled-in cut with a pellet in the jaws
    /// (`options::pack_on`).
    pub packed: u64,
    pub dumped: u64,
    /// **Ants that starved holding soil**, and of those, with food in
    /// reach (one of the eight cells round the head). **Why**: jaws
    /// holding soil cannot pick food up (fix plan step 1), and soil goes
    /// down only under three empty rows, so in a two-row tunnel an ant can
    /// starve beside food it cannot take until it finds headroom (review
    /// of the fix plan, finding 4). Dig step 4's colony prints it; if it is
    /// not zero, a hungry ant may put soil down on any footed cell.
    pub starved_soil: u64,
    pub starved_soil_food: u64,
}

/// **Every mind in the world**, by ant: a side table on the world, as the
/// needs walk keeps its own (`World::needs`), so the game ant's
/// `OrganismState` is untouched.
#[derive(Clone, Debug, Default)]
pub struct Minds {
    /// Fx-hashed: asked three times an ant a tick (`has`, the bill's `get`,
    /// the decision's `get`/`get_mut`), and nothing reads it in its
    /// iteration order (`prune`'s `retain` keeps by id).
    ants: crate::sim::fxhash::FxHashMap<OrganismId, Mind>,
    pruned_at: u64,
    pub counts: MindCounts,
    /// **Every pellet a mind puts down**, as `(x, y, frame, ant)`: the soil
    /// trip's other end, beside `World::cut_log`'s cut. Off (`None`) unless
    /// a reader sets `Some(Vec::new())`; recording draws nothing and changes
    /// nothing. Kept here rather than on `World` because only a mind's
    /// put-down writes it.
    pub dump_log: Option<Vec<(i32, i32, u64, OrganismId)>>,
    /// **Build the game ant's ways out even in a world of minds**
    /// ([`every_ant_has_one`]): the control for the skip, off unless a
    /// paired check sets it.
    pub build_ways: bool,
}

/// **Whether every adult creature in the world decides by a mind**, so the
/// game ant's hand rules have no reader left in it. A world with no minds
/// (the game) reads false; so does one with a single creature that walks by
/// the game's rules, today's brain included.
pub fn every_ant_has_one(world: &World) -> bool {
    let Some(m) = world.minds.as_ref().filter(|m| !m.is_empty() && !m.build_ways) else {
        return false;
    };
    world.live_organism_ids().into_iter().all(|id| {
        m.ants.contains_key(&id)
            || world
                .organism(id)
                .is_none_or(|s| world.species.get(s.species).creature.is_none())
    })
}

impl Minds {
    pub fn len(&self) -> usize {
        self.ants.len()
    }
    pub fn is_empty(&self) -> bool {
        self.ants.is_empty()
    }
    /// Take one ant's mind away, so it walks by the game's own rules: the
    /// mixed world the way skip must notice ([`every_ant_has_one`]).
    #[cfg(test)]
    pub(crate) fn forget(&mut self, id: OrganismId) {
        self.ants.remove(&id);
    }
}

/// **Give `organism` a mind**: from its next decision it decides by
/// `genome` and acts by the body's rules alone.
pub fn give(world: &mut World, organism: OrganismId, genome: Arc<MindGenome>) {
    world.minds.get_or_insert_with(Default::default).ants.insert(
        organism,
        Mind {
            genome,
            state: MindState::default(),
        },
    );
}

/// Whether `organism` decides by a mind.
pub fn has(world: &World, organism: OrganismId) -> bool {
    world.minds.as_ref().is_some_and(|m| m.ants.contains_key(&organism))
}

/// **A newborn inherits its parent's mind, blank in memory**; mutation is
/// frozen, so the genome is the parent's own.
pub fn inherit(world: &mut World, parent: OrganismId, child: OrganismId) {
    let Some(minds) = world.minds.as_mut() else { return };
    let genome = minds.ants.get(&parent).map(|m| m.genome.clone());
    match genome {
        Some(genome) => {
            minds.ants.insert(
                child,
                Mind {
                    genome,
                    state: MindState::default(),
                },
            );
        }
        None => {
            // An id can come back after its slot's generation wraps; a
            // child of a mindless parent must not inherit a stale mind.
            minds.ants.remove(&child);
        }
    }
}

const PRUNE_EVERY: u64 = 600;

/// Let go of the minds of ants no longer alive.
fn prune(world: &mut World) {
    let frame = world.frame;
    if !world.minds.as_ref().is_some_and(|m| frame >= m.pruned_at + PRUNE_EVERY) {
        return;
    }
    // **Brood keeps its mind too.** An egg inherits its layer's mind at
    // laying (`creature::try_bud`) and hands it on at hatching
    // (`brood::hatch`), but the list of the living is adults only, so this
    // let every egg's mind go at the next prune and the ant it hatched into
    // decided by the game's own rules: in a scarce colony of foragers, a
    // hatchling with no mind 2,069 frames in, found because the ways skip
    // ([`every_ant_has_one`]) stopped firing (2026-10-18).
    let live: std::collections::HashSet<OrganismId> = world
        .live_organism_ids()
        .into_iter()
        .chain(world.live_brood_ids())
        .collect();
    if let Some(m) = world.minds.as_mut() {
        m.ants.retain(|id, _| live.contains(id));
        m.pruned_at = frame;
    }
}

// --- reading the ant --------------------------------------------------------

/// **What the ant feels**, once a decision: a pure read, with the change
/// senses taken against `state.last` and [`c::ROOF_CUT`] off this
/// decision's options. Returns the context and this decision's values for
/// the next one's change senses.
pub fn context(world: &World, organism: OrganismId, def: &CreatureDef, state: &MindState, opts: &[Opt]) -> ([f32; CONTEXT], [f32; 5]) {
    context_at(world, organism, def, state, opts, None)
}

/// [`context`], given the caller's own [`adjacent_nest`] at a cell, this
/// tick, with no cell written since.
fn context_at(
    world: &World,
    organism: OrganismId,
    def: &CreatureDef,
    state: &MindState,
    opts: &[Opt],
    at_nest: Option<((i32, i32), bool)>,
) -> ([f32; CONTEXT], [f32; 5]) {
    let mut v = [0.0f32; CONTEXT];
    v[c::ROOF_CUT] = roof_cut(opts);
    v[c::HOLE_DROP] = hole_drop(opts);
    let Some(st) = world.organism(organism) else {
        return (v, [0.0; 5]);
    };
    let Some(&(hx, hy)) = st.chain.first() else {
        return (v, [0.0; 5]);
    };
    let tank = def.sensed_tank.unwrap_or(def.start_energy);
    let e = (st.energy / tank.max(f32::EPSILON)).max(0.0);
    v[c::ENERGY] = e / (1.0 + e);
    let cap = organism_crop_capacity(world, organism, def);
    v[c::CROP] = st.crop.map_or(0.0, |cr| {
        if cap > 0.0 {
            (cr.worth() / cap).clamp(0.0, 1.0)
        } else {
            1.0
        }
    });
    v[c::SPOIL] = f32::from(u8::from(st.spoil.is_some()));
    // `at_nest`: the caller's own `adjacent_nest` at a cell, this tick,
    // with no cell written since -- the same answer when it is the head.
    let home = match at_nest {
        Some((at, home)) if at == (hx, hy) => home,
        _ => adjacent_nest(world, hx, hy, def),
    };
    v[c::AT_NEST] = f32::from(u8::from(home));
    let scents = EMIT_CHANNELS.map(|ch| world.pheromone_at(ch, hx, hy) as f32 / pheromone::Scent::MAX as f32);
    // SCRATCH (teacher lane): `PP_SCENT_BLIND=1` reads every trail as 0, to see whether a mind uses them.
    static BLIND: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let scents = if *BLIND.get_or_init(|| std::env::var("PP_SCENT_BLIND").is_ok_and(|v| v == "1")) { [0.0; 3] } else { scents };
    v[c::SCENT..c::SCENT + 3].copy_from_slice(&scents);
    let home = (!world.home_blind).then_some(st.forage_anchor);
    let dist = home.map_or(0.0, |(ax, ay)| {
        let (dx, dy) = ((ax - hx) as f32, (ay - hy) as f32);
        (dx * dx + dy * dy).sqrt()
    });
    v[c::HOME_SEEN] = f32::from(u8::from(home.is_some()));
    v[c::HOME_DIST] = squash(dist / 32.0);
    let now = [scents[0], scents[1], scents[2], e, dist];
    if let Some(last) = state.last {
        for k in 0..3 {
            v[c::D_SCENT + k] = squash(4.0 * (now[k] - last[k]));
        }
        v[c::D_ENERGY] = squash(10.0 * (now[3] - last[3]));
        v[c::D_HOME] = squash((now[4] - last[4]) / 2.0);
    }
    v[c::SINCE_NEST] = squash(st.since_nest as f32 / 200.0);
    v[c::AGE] = squash(world.frame.saturating_sub(st.born_frame) as f32 / 20_000.0);
    v[c::SELF] = rng::stream(0, organism as u64, 0, RNG_SLOT_SELF).unit_f32() * 2.0 - 1.0;
    let mut touching = 0;
    for (dx, dy) in DIRS {
        let id = world.get(hx + dx, hy + dy).organism_id();
        if id != 0
            && id != organism
            && world
                .organism(id)
                .is_some_and(|o| world.species.get(o.species).creature.is_some())
        {
            touching += 1;
        }
    }
    v[c::CROWD] = touching as f32 / 8.0;
    v[c::DEPTH] = depth_at(world, hx, hy);
    v[c::ROOFED] = f32::from(u8::from(under_cover(world, hx, hy)));
    (v, now)
}

// --- deciding -----------------------------------------------------------------

/// What one decision chose, and why: the readouts the design asks for (the
/// score rose and the job) come from here.
#[derive(Clone, Debug)]
pub struct Decision {
    /// Index into the option list.
    pub pick: usize,
    /// Every option's score, in list order (0 for verbs that are off).
    pub scores: Vec<f32>,
    /// Each verb's job score; `None` for a verb with no target, or off.
    pub jobs: [Option<f32>; VERBS],
    /// Each scent's strength, 0 to 1.
    pub emits: [f32; EMITS],
}

/// **The job head's hidden units for one decision**, from its context and
/// the memory before this decision's update, each squashed with its bias
/// ([`HID`]): only those some job reads, so a genome without them runs as
/// it did.
fn hidden_units(g: &[f32], ctx: &[f32; CONTEXT], mem: &[f32; MEMORY]) -> [f32; HIDDEN] {
    let mut hid = [0.0f32; HIDDEN];
    let read = |h: usize| (0..VERBS).any(|v| g[HID_OUT + v * HIDDEN + h] != 0.0);
    for (h, slot) in hid.iter_mut().enumerate().filter(|(h, _)| read(*h)) {
        let row = &g[HID + h * H_IN..HID + (h + 1) * H_IN];
        let mut sum = row[H_IN - 1];
        for (w, x) in row[..CONTEXT].iter().zip(ctx) {
            sum += w * x;
        }
        for (w, x) in row[CONTEXT..CONTEXT + MEMORY].iter().zip(mem) {
            sum += w * x;
        }
        *slot = squash(sum);
    }
    hid
}

/// **One decision**: score every option, pick the job, pick its target,
/// update memory. `None` when nothing the mind may pick is on the list
/// (every verb with a target switched off).
///
/// A pure function of the genome, the context, the options, the state and
/// the draw, so it can be tested without a world.
pub fn decide(
    g: &[f32],
    ctx: &[f32; CONTEXT],
    opts: &[Opt],
    state: &mut MindState,
    draw: &mut rng::Rng,
) -> Option<Decision> {
    decide_planned(g, &ScorerPlan::new(g), ctx, opts, state, draw)
}

/// How many options [`decide_planned`] lists on the stack.
const STACK_OPTS: usize = 48;

/// [`decide`] with the scorer's plan built once per genome
/// ([`MindGenome::plan`]) rather than once a call: what [`tick`] runs.
fn decide_planned(
    g: &[f32],
    plan: &ScorerPlan,
    ctx: &[f32; CONTEXT],
    opts: &[Opt],
    state: &mut MindState,
    draw: &mut rng::Rng,
) -> Option<Decision> {
    debug_assert_eq!(g.len(), GENOME_LEN);
    let on = |v: Verb| v.acts() && g[SWITCH + v as usize] > 0.0;
    let sharp = sharpness(g);

    // The scorer's per-decision part, once.
    let mut pre = [0.0f32; UNITS];
    for (u, p) in pre.iter_mut().enumerate() {
        let row = &g[S_CTX + u * S_IN..S_CTX + (u + 1) * S_IN];
        let mut sum = row[CONTEXT + MEMORY];
        for (w, x) in row[..CONTEXT].iter().zip(ctx) {
            sum += w * x;
        }
        for (w, x) in row[CONTEXT..CONTEXT + MEMORY].iter().zip(&state.mem) {
            sum += w * x;
        }
        *p = sum;
    }
    // Every option, by the same weights.
    let out = &g[S_OUT..S_OUT + UNITS + 1];
    let mut scores = vec![0.0f32; opts.len()];
    let mut best = [f32::NEG_INFINITY; VERBS];
    let (mut all_sum, mut all_n) = (0.0f32, 0u32);
    // Each lane's sum as it starts for each verb, before the option's terms.
    let start: [[f32; UNITS]; VERBS] =
        std::array::from_fn(|v| std::array::from_fn(|k| plan.units.get(k).map_or(0.0, |&u| pre[u] + g[S_VERB + u * VERBS + v])));
    let narrow = plan.units.len() <= UNITS / 2;
    for (o, opt) in opts.iter().enumerate() {
        if !on(opt.verb) {
            continue;
        }
        let v = opt.verb as usize;
        debug_assert!(opt.feats.iter().all(|x| x.is_finite()), "a feature that is not finite: {:?}", opt.verb);
        // Every unit's sum at once, each over its terms in their order
        // ([`ScorerPlan`]).
        let z = if narrow {
            score_lanes::<{ UNITS / 2 }>(&start[v], &plan.feats, &opt.feats)
        } else {
            score_lanes::<UNITS>(&start[v], &plan.feats, &opt.feats)
        };
        let mut raw = out[UNITS];
        for (k, &u) in plan.units.iter().enumerate() {
            raw += out[u] * squash(z[k]);
        }
        let s = squash(sharp * raw);
        scores[o] = s;
        best[v] = best[v].max(s);
        all_sum += s;
        all_n += 1;
    }
    if all_n == 0 {
        return None;
    }
    let all_best = best.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let all_mean = all_sum / all_n as f32;

    let hid = hidden_units(g, ctx, &state.mem);

    // The job: context, memory, the hidden units, the verb's best target,
    // its offset, and the pull towards last time's verb.
    let mut jobs = [None; VERBS];
    let mut weights = [0.0f32; VERBS];
    let pulled = pull(g);
    for v in 0..VERBS {
        if best[v] == f32::NEG_INFINITY {
            continue;
        }
        let row = &g[JOB + v * J_IN..JOB + (v + 1) * J_IN];
        let mut sum = row[CONTEXT + MEMORY] * best[v] + row[CONTEXT + MEMORY + 1];
        for (w, x) in row[..CONTEXT].iter().zip(ctx) {
            sum += w * x;
        }
        for (w, x) in row[CONTEXT..CONTEXT + MEMORY].iter().zip(&state.mem) {
            sum += w * x;
        }
        for (w, x) in g[HID_OUT + v * HIDDEN..HID_OUT + (v + 1) * HIDDEN].iter().zip(&hid) {
            if *w != 0.0 {
                sum += w * x;
            }
        }
        if state.last_verb == Some(Verb::ALL[v]) {
            sum += pulled;
        }
        let j = squash(sharp * sum);
        jobs[v] = Some(j);
        weights[v] = (j + 1.0) / 2.0;
    }
    let (mut present, mut shifted, mut n) = ([0usize; VERBS], [0.0f32; VERBS], 0);
    for v in (0..VERBS).filter(|&v| jobs[v].is_some()) {
        (present[n], shifted[n]) = (v, weights[v]);
        n += 1;
    }
    let verb = Verb::ALL[present[choose_weighted(&shifted[..n], PICK_FLOOR, draw.unit_f32())]];

    // Its target. The verb's and the target's lists sit on the stack for
    // any list a body makes (~11 options, at most a few dozen; a longer one,
    // which nothing builds, takes the heap): four of the small allocations
    // the ant cost review priced as churn (2026-10-22).
    let (mut t_stack, mut s_stack, mut t_heap, mut s_heap) = ([0usize; STACK_OPTS], [0.0f32; STACK_OPTS], Vec::new(), Vec::new());
    let (targets, shifted): (&mut [usize], &mut [f32]) = if opts.len() <= STACK_OPTS {
        (&mut t_stack[..], &mut s_stack[..])
    } else {
        t_heap.resize(opts.len(), 0);
        s_heap.resize(opts.len(), 0.0);
        (&mut t_heap[..], &mut s_heap[..])
    };
    let mut n = 0;
    for o in (0..opts.len()).filter(|&o| opts[o].verb == verb) {
        (targets[n], shifted[n]) = (o, (scores[o] + 1.0) / 2.0);
        n += 1;
    }
    let pick = targets[choose_weighted(&shifted[..n], PICK_FLOOR, draw.unit_f32())];

    // Memory, from what the ant felt, saw and chose.
    let mut fresh = [0.0f32; MEMORY];
    for (m, slot) in fresh.iter_mut().enumerate() {
        let row = &g[MEM + m * M_IN..MEM + (m + 1) * M_IN];
        let mut sum =
            row[M_IN - 1] + row[CONTEXT] * all_best + row[CONTEXT + 1] * all_mean + row[CONTEXT + 2 + verb as usize];
        for (w, x) in row[..CONTEXT].iter().zip(ctx) {
            sum += w * x;
        }
        for (w, x) in row[CONTEXT + 2 + VERBS..CONTEXT + 2 + VERBS + MEMORY]
            .iter()
            .zip(&state.mem)
        {
            sum += w * x;
        }
        let r = rate(g, m);
        *slot = r * state.mem[m] + (1.0 - r) * squash(sum);
    }
    state.mem = fresh;
    state.last_verb = Some(verb);

    // What it lays alongside.
    let mut emits = [0.0f32; EMITS];
    for (e, slot) in emits.iter_mut().enumerate() {
        let row = &g[EMIT + e * E_IN..EMIT + (e + 1) * E_IN];
        let mut sum = row[E_IN - 1];
        for (w, x) in row[..CONTEXT].iter().zip(ctx) {
            sum += w * x;
        }
        for (w, x) in row[CONTEXT..CONTEXT + MEMORY].iter().zip(&state.mem) {
            sum += w * x;
        }
        *slot = squash(sum).clamp(0.0, 1.0);
    }
    Some(Decision {
        pick,
        scores,
        jobs,
        emits,
    })
}

// --- the tick -----------------------------------------------------------------

thread_local! {
    static SCRATCH: RefCell<Vec<Opt>> = const { RefCell::new(Vec::new()) };
}

/// **One decision of an ant with a mind**, in place of `creature_tick`'s
/// sense, brain, `act` and walk. What stays is the body's: the colony odour
/// blending at the nest, the fall, the energy bill, digestion, starvation
/// and birth (`settle`). Called from `creature_tick` once old age, wounds,
/// crossings and flight are done.
pub(super) fn tick(world: &mut World, x: i32, y: i32, organism: OrganismId, def: &CreatureDef) -> Vec<ActiveSite> {
    prune(world);
    let colony = world.colony_of(organism);
    let body_cells = live_body_cells(world, organism, def);
    let idle = def.idle_cost_per_cell * body_cells;
    // **Every standing bill, every tick, as `creature_tick` charges it** --
    // a fall tick included. The game bills its brain before its fall check;
    // billing a mind only when it decided left a trail champion stepping
    // into the air on 36% of its ticks, where thinking was free (review of
    // PR 713, F2). The jaw and the plate are carried whether used or not,
    // and today's ant paid for them while a mind did not. The same lines
    // as `creature_tick`'s, in the same order; seeing and feeling the
    // ground are not here because a mind casts no rays (its options read
    // the cells near it, unpriced).
    let live = world
        .minds
        .as_ref()
        .and_then(|m| m.ants.get(&organism))
        .map_or(0, |m| m.genome.live);
    let brain_tax = def.synapse_fraction * def.start_energy * live as f32;
    let jaw_traits = traits_of(world, organism, def);
    let force_tax = def.force_fraction
        * def.start_energy
        * dig_force_of(def, &jaw_traits, world.trait_reach).max(bite_force_of(def, &jaw_traits, world.trait_reach));
    let armour_tax = def.armour_fraction
        * def.start_energy
        * world.materials.get(world.get(x, y).material).penetration_resistance
        * armour_of(&jaw_traits, world.trait_reach);
    let exposure = if def.exposure_cost_per_cell > 0.0 && !is_sheltered(world, x, y) {
        def.exposure_cost_per_cell * body_cells
    } else {
        0.0
    };
    let mut spent = idle + brain_tax + force_tax + armour_tax + exposure;
    world.book(
        colony,
        Account::Metabolized,
        (idle + force_tax + armour_tax + exposure) as f64,
    );
    world.book(colony, Account::SynapseTax, brain_tax as f64);
    world.creature_stats.armour_energy += armour_tax as f64;
    world.creature_stats.force_energy += force_tax as f64;
    world.creature_stats.exposure_energy += exposure as f64;
    let at_nest = adjacent_nest(world, x, y, def);
    if at_nest {
        // Colony odour is the body's: without the blend a colony's odours
        // drift apart until it reads its own children as strangers
        // (`creature_tick`'s cohesion note).
        world.creature_stats.at_nest_ticks += 1;
        blend_with_nest(world, organism, x, y);
    }

    // **Falling is physics, and comes first**: an unsupported body falls,
    // and that is its decision.
    let fell = fall_now_if_unsupported(world, organism, def);
    let mut moved = fell;
    if fell {
        if let Some(m) = world.minds.as_mut() {
            m.counts.falls += 1;
        }
    } else if let Some(mut mind) = world.minds.as_mut().and_then(|m| m.ants.get(&organism).cloned()) {
        let decided = SCRATCH.with(|s| {
            let mut opts = s.borrow_mut();
            options::build_with(
                world,
                organism,
                def,
                mind.genome.w[SWITCH + Verb::Dig as usize] > 0.0,
                &mut opts,
            );
            // `at_nest` was read above at `(x, y)`; since then only the
            // colony odour was blended and no fall happened, so no cell
            // moved (ant cost review A4, 2026-10-22).
            let (mut ctx, now) = context_at(world, organism, def, &mind.state, &opts, Some(((x, y), at_nest)));
            // SCRATCH (investigation): PP_ABLATE=name,name zeroes those senses before every decision.
            scratch_log::ablate(&mut ctx);
            // SCRATCH (teacher lane): `PP_BOTTOM_CTX=1` (with `PP_BOTTOM`) puts "a cut at the bottom of my hole, below
            // the ground as built, is in reach" on the borrowed context slot c::CROWD (17), which reads 0 in every solo
            // arena: the job stage reads it straight, past the scorer's saturated best-score channel.
            static BOTTOM_CTX: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            if *BOTTOM_CTX.get_or_init(|| std::env::var("PP_BOTTOM_CTX").is_ok()) {
                ctx[17] = f32::from(u8::from(opts.iter().any(|o| o.verb == Verb::Dig && o.feats[options::f::BUILDING] == 1.0 && o.feats[options::f::DEPTH] > 0.0)));
            }
            let mut draw = rng::stream(world.seed, organism as u64, world.frame, RNG_SLOT_MIND);
            let (mem0, last0) = (mind.state.mem, mind.state.last_verb);
            let mut d = decide_planned(&mind.genome.w, &mind.genome.plan, &ctx, &opts, &mut mind.state, &mut draw);
            if let Some(d) = d.as_ref() {
                scratch_log::decision(world.seed, world.frame, organism, (x, y), &ctx, &mem0, last0, &opts, d);
                scratch_log::copy_decision(world, organism, (x, y), &ctx, &mem0, last0, &opts, d);
            }
            // SCRATCH (teacher-training ceiling test): the hand-written teacher's three calls (`scratch_log::teach`).
            let mut redrawn: Option<Vec<Opt>> = None;
            // SCRATCH (dig-scoring thread, 2026-10-22; owner: "refusal as a teacher makes sense"): `PP_RIMVETO=N`
            // refuses a cut of ground (not spoil) fewer than N rows below the frozen datum within `PP_RIMVETO_R`
            // (default 20) columns of `PP_RIMVETO_X` (default 128): when the brain picks one, every such Dig option is
            // taken off and it redraws from the rest, deep cuts included. The crater's teacher, for copying; the
            // 2026-10-21 version (`PP_STORE_VERB=dig`, inrun-wt) took off every Dig and is not this.
            if let Some((n, nx, nr)) = scratch_log::rimveto() {
                let rim = |o: &Opt| scratch_log::rim_cut(world, o, n, nx, nr);
                if let Some(i) = d.as_ref().map(|d| d.pick) {
                    if rim(&opts[i]) {
                        let allowed: Vec<Opt> = opts.iter().copied().filter(|o| !rim(o)).collect();
                        if !allowed.is_empty() {
                            mind.state.mem = mem0;
                            mind.state.last_verb = last0;
                            let mut again = rng::stream(world.seed, organism as u64, world.frame, RNG_SLOT_MIND + 1002);
                            if let Some(d2) = decide_planned(&mind.genome.w, &mind.genome.plan, &ctx, &allowed, &mut mind.state, &mut again) {
                                d = Some(d2);
                                redrawn = Some(allowed);
                                static V: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                                let k = V.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                                if k % 1000 == 0 {
                                    eprintln!("PP_RIMVETO refusals {k}");
                                }
                            }
                        }
                    }
                }
            }
            // SCRATCH (dig-scoring thread, 2026-10-22): `PP_SPOILVETO=R` refuses letting go of a pellet of spoil below
            // the frozen datum (inside the nest or the crater) or within R columns of `PP_RIMVETO_X` (default 128): when
            // the brain picks such a Drop with spoil in the jaws, every such Drop is taken off and it redraws. Late in a
            // 200k round a third of carried spoil was let go in the shaft (median 6 rows down, 6 columns from the door)
            // and a third of re-cuts were soil sliding back from beside the door (hA1, 8 seeds); with PP_RIMVETO on, a
            // third of spoil went down within 5 columns of the door. The spoil teacher, for copying.
            static SPOILVETO: std::sync::OnceLock<Option<(i32, i32)>> = std::sync::OnceLock::new();
            let sv = *SPOILVETO.get_or_init(|| {
                let r = std::env::var("PP_SPOILVETO").ok()?.parse::<i32>().ok()?;
                Some((r, std::env::var("PP_RIMVETO_X").ok().and_then(|v| v.parse().ok()).unwrap_or(128)))
            });
            if let Some((r, nx)) = sv {
                if world.organism(organism).is_some_and(|s| s.spoil.is_some()) {
                    let near = |o: &Opt| {
                        let (tx, ty) = o.target;
                        o.verb == Verb::Drop && ((tx - nx).abs() <= r || world.room_surface_at(tx).is_some_and(|s| ty >= s))
                    };
                    if let Some(i) = d.as_ref().map(|d| d.pick) {
                        if near(&opts[i]) {
                            let allowed: Vec<Opt> = opts.iter().copied().filter(|o| !near(o)).collect();
                            if !allowed.is_empty() {
                                mind.state.mem = mem0;
                                mind.state.last_verb = last0;
                                let mut again = rng::stream(world.seed, organism as u64, world.frame, RNG_SLOT_MIND + 1003);
                                if let Some(d2) = decide_planned(&mind.genome.w, &mind.genome.plan, &ctx, &allowed, &mut mind.state, &mut again) {
                                    d = Some(d2);
                                    redrawn = Some(allowed);
                                }
                            }
                        }
                    }
                }
            }
            if scratch_log::teach_on() {
                if let Some((pick, jobs, scores0)) = d.as_ref().map(|d| (d.pick, d.jobs, d.scores.clone())) {
                    let own_verb = opts[pick].verb;
                    // Positive control for `remember`: the brain's own verb must reproduce the memory decide wrote.
                    let check = scratch_log::remember(&mind.genome.w, &ctx, &mem0, &opts, &scores0, own_verb);
                    if check.iter().zip(&mind.state.mem).any(|(a, b)| (a - b).abs() > 1e-6) {
                        scratch_log::COUNTS[10].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    match scratch_log::teach(world, organism, &opts, pick, &jobs) {
                        scratch_log::Teach::Keep => {}
                        scratch_log::Teach::Force(i) => {
                            if let Some(d) = d.as_mut() {
                                d.pick = i;
                            }
                            mind.state.last_verb = Some(opts[i].verb);
                        }
                        scratch_log::Teach::Veto(v) => {
                            let allowed: Vec<Opt> = opts.iter().copied().filter(|o| o.verb != v).collect();
                            if !allowed.is_empty() {
                                mind.state.mem = mem0;
                                mind.state.last_verb = last0;
                                let mut again = rng::stream(world.seed, organism as u64, world.frame, RNG_SLOT_MIND + 1000);
                                if let Some(d2) = decide_planned(&mind.genome.w, &mind.genome.plan, &ctx, &allowed, &mut mind.state, &mut again) {
                                    d = Some(d2);
                                    redrawn = Some(allowed);
                                }
                            }
                        }
                    }
                    let list: &[Opt] = redrawn.as_deref().unwrap_or(&opts[..]);
                    if let Some(d) = d.as_mut() {
                        if let Some(i) = scratch_log::walk_home(world, organism, (x, y), list, d.pick) {
                            d.pick = i;
                        }
                    }
                    // Memory and the job pull follow the verb actually carried out, summed over the full option list.
                    if let Some(d) = d.as_ref() {
                        let done = list[d.pick].verb;
                        mind.state.mem = scratch_log::remember(&mind.genome.w, &ctx, &mem0, &opts, &scores0, done);
                        mind.state.last_verb = Some(done);
                    }
                }
            }
            mind.state.last = Some(now);
            let mut offered = [false; VERBS];
            for o in opts.iter() {
                offered[o.verb as usize] = true;
            }
            let list: &[Opt] = redrawn.as_deref().unwrap_or(&opts[..]);
            (d.map(|d| (list[d.pick], d.emits, list.len())), offered)
        });
        let (decided, offered) = decided;
        if let Some(m) = world.minds.as_mut() {
            m.counts.decisions += 1;
            for (n, &o) in m.counts.offered.iter_mut().zip(&offered) {
                *n += u64::from(o);
            }
            if let Some(slot) = m.ants.get_mut(&organism) {
                slot.state = mind.state;
            }
        }
        if let Some((opt, emits, n)) = decided {
            let mut draw = rng::stream(world.seed, organism as u64, world.frame, RNG_SLOT_ACT);
            let did = act(world, organism, def, opt, &mut draw);
            // **Feeding is mandible to mandible, priced as the game prices
            // a share** (`creature_tick`'s jaw bill): the same work whoever
            // decided it, from the bank or the crop.
            if did && opt.verb == Verb::Feed && def.dig_cost_in_moves > 0.0 {
                let work = def.move_cost_per_cell * body_cells * def.dig_cost_in_moves;
                spent += work;
                world.book(colony, Account::Metabolized, work as f64);
                world.creature_stats.share_energy += work as f64;
            }
            // **A cut is jaw work, priced as the game prices one**
            // (`creature_tick`'s dig bill), to the same account.
            if did && opt.verb == Verb::Dig && def.dig_cost_in_moves > 0.0 {
                let cost = def.move_cost_per_cell * body_cells * def.dig_cost_in_moves;
                spent += cost;
                world.book(colony, Account::Metabolized, cost as f64);
                world.creature_stats.dig_energy += cost as f64;
            }
            if did {
                moved = matches!(opt.verb, Verb::Walk | Verb::TurnAround);
                if opt.verb == Verb::Walk {
                    let step = def.move_cost_per_cell * (body_cells + carried_cells(world, organism, def));
                    spent += step;
                    world.book(colony, Account::Moved, step as f64);
                }
            }
            spent += lay(world, organism, def, colony, body_cells, emits);
            if let Some(m) = world.minds.as_mut() {
                m.counts.options += n as u64;
                m.counts.chose[opt.verb as usize] += 1;
                m.counts.failed += u64::from(!did);
            }
        }
    }

    if let Some(state) = world.organism_mut(organism) {
        state.since_nest = state.since_nest.saturating_add(1);
        state.since_trip = state.since_trip.saturating_add(1);
        if moved {
            state.still_ticks = 0;
        } else {
            state.still_ticks = state.still_ticks.saturating_add(1);
        }
    }
    let (hx, hy) = world
        .organism(organism)
        .and_then(|s| s.chain.first().copied())
        .unwrap_or((x, y));
    // **Starved holding soil** ([`MindCounts::starved_soil`]): read before
    // the settle, since a dead ant's body is corpse, and corpse is food.
    let soil = world.organism(organism).is_some_and(|s| s.spoil.is_some());
    let food_near = soil && {
        let corpse = world.materials.known(Known::Corpse);
        DIRS.iter().any(|&(dx, dy)| {
            let cell = world.get(hx + dx, hy + dy);
            world.materials.get(cell.material).food_energy > 0.0 && cell.organism_id() == 0 && Some(cell.material) != corpse
        })
    };
    // No brain outputs gate birth: what the body's own bars allow, it does.
    let sites = settle(world, organism, def, (hx, hy), colony, at_nest, spent, 0.0, 0.0);
    if soil && world.organism(organism).is_none() {
        if let Some(m) = world.minds.as_mut() {
            m.counts.starved_soil += 1;
            m.counts.starved_soil_food += u64::from(food_near);
        }
    }
    sites
}

/// **Do the chosen option by the world's rules.** `false` when the body
/// could not after all; the list is built from the same checks, so that is
/// a bug to count ([`MindCounts::failed`]), not a choice.
fn act(world: &mut World, organism: OrganismId, def: &CreatureDef, opt: Opt, draw: &mut rng::Rng) -> bool {
    let head = world.organism(organism).and_then(|s| s.chain.first().copied());
    match opt.verb {
        Verb::Rest => true,
        Verb::Walk => walk(world, organism, def, opt.target),
        Verb::TurnAround => turn_around(world, organism, def),
        Verb::PickUp => pick_up(world, organism, def, opt.target, draw),
        Verb::Eat => head == Some(opt.target) && eat_now(world, organism),
        Verb::Drop => put_down(world, organism, def, opt.target),
        Verb::Feed => feed(world, organism, def, opt.target),
        Verb::Dig => dig(world, organism, def, opt.target),
        _ => false,
    }
}

/// **Eat from the crop**: finish the cell being digested this tick
/// (`eat_lunch_now`). Digestion runs whatever the ant chose, so this only
/// hurries it; the crop is not a choice to keep food from the body.
/// Counted as [`MindCounts::ate`]: food eaten by choice, which the settle's
/// `eats` (every cell finished, chosen or not) cannot tell apart.
fn eat_now(world: &mut World, organism: OrganismId) -> bool {
    match world.organism_mut(organism) {
        Some(s) if s.crop.is_some() => {
            s.eat_lunch_now = true;
            if let Some(m) = world.minds.as_mut() {
                m.counts.ate += 1;
            }
            true
        }
        _ => false,
    }
}

/// **Pick up a loose cell of food into the crop**, by the game's own
/// physics (`act`'s ingest): a windfall's seed asks whether it survives the
/// mouth, the cell leaves the world as `Eaten`, and the crop takes it at face
/// value (`crop_take`), to be paid out by digestion. The list offered only
/// what the pick-up allows (`options::build`), so the checks here are the
/// ones the world could have changed since, and the jaws: a pellet held
/// there takes no food.
///
/// **Until fix plan step 1 this was Eat's** (aimed at food in reach): one
/// choice both carried and ate, and trained ants ate 73 of 75 cells they
/// picked up (`brain-training/ladder/actions-audit.md`). Pick up had genes
/// and no act.
///
/// **None of the game's food rules**: no store keep, no pile, no sated
/// leave, no pellet block. Its counters, so a harness reads a mind's
/// foraging as it reads the game's: `pickups` (and at the nest),
/// `best_bite`, the life's `bites`, and the trip marks a delivery books
/// (`trip_load`, `trip_cells`) -- written here for any food taken away from
/// home, the game's trip reach and minimum being walk rules. **Not
/// `eats`**: a pick-up is not an eat, and the settle counts every cell the
/// crop finishes (`creature_stats.eats`), so counting here too read about
/// twice the pick-ups (review of the actions audit, item 9).
fn pick_up(
    world: &mut World,
    organism: OrganismId,
    def: &CreatureDef,
    (fx, fy): (i32, i32),
    draw: &mut rng::Rng,
) -> bool {
    let Some((crop, jaws, (hx, hy))) = world
        .organism(organism)
        .and_then(|s| s.chain.first().map(|&h| (s.crop, s.spoil.is_some(), h)))
    else {
        return false;
    };
    let bite = world.get(fx, fy);
    let food = bite.material;
    if jaws
        || world.materials.get(food).food_energy <= 0.0
        || world.materials.known(Known::Corpse) == Some(food)
        || bite.organism_id() != 0
        || crop.is_some_and(|c| c.material != food || c.worth() + c.unit > organism_crop_capacity(world, organism, def))
    {
        return false;
    }
    let at_home = food_at_home(world, organism, def, hx, hy);
    let seed = plant::seed_survives_bite(world, fx, fy, draw);
    let worth = match seed {
        plant::SeedBite::SurvivedBare => food_value(world, bite) * plant::seed_provision_fraction(world, fx, fy),
        _ => food_value(world, bite),
    };
    if !seed.survived() {
        world.loss_context = Some(DeathCause::Eaten);
        world.set(fx, fy, Cell::EMPTY);
        world.loss_context = None;
    }
    let passenger = (seed.survived() && crop.is_none_or(|c| c.passenger.is_none()))
        .then(|| plant::take_seed_passenger(world, fx, fy))
        .flatten();
    if passenger.is_some() && seed == plant::SeedBite::SurvivedBare {
        world.bare_seeds_carried += 1;
    }
    crop_take(world, organism, food, bite.shade, worth, passenger);
    // SCRATCH (teacher lane, in-run pilot): the harness's food ledger.
    super::note_food(world, organism, (fx, fy), super::FOOD_SWALLOW, worth, at_home);
    let fresh = crop.is_none_or(|c| c.worth() <= 0.0);
    if let Some(state) = world.organism_mut(organism) {
        state.life.bites += 1;
        if fresh {
            state.trip_load = false;
            state.trip_src = 0;
            state.trip_cells = 0;
        }
        if !at_home {
            state.foraged = true;
            state.trip_load = true;
            state.since_trip = 0;
            state.trip_cells = state.trip_cells.saturating_add(1);
        }
        // A packed lunch is the game's rule; a mind's crop never is one,
        // so a larva can always be fed from it (`crop_to_feed`).
        state.lunch = false;
    }
    let st = &mut world.creature_stats;
    st.pickups += 1;
    st.pickups_at_nest += u64::from(at_home);
    st.best_bite = st.best_bite.max(worth);
    if let Some(m) = world.minds.as_mut() {
        m.counts.swallowed += 1;
    }
    true
}

/// **Cut one cell of ground into the jaws**, by the game's own cut
/// (`cut_cell`): the pellet into the jaws, the cell emptied, the burrow
/// lined. **None of the game's vetoes** -- the heap cue, the face, the roof,
/// shaft rooms, company: which cell is the mind's, by the owner's ruling
/// that a spatial need is a brain input, not a predicate in the act
/// (`dead-ends.md`, the two hand-written placement rules). The list offered
/// only ground the jaw can cut, with free jaws (`options::diggable`); the
/// checks here are the ones the world could have changed since. The caller
/// bills the jaw work.
///
/// **Ground only, and a full crop no longer stops it** (fix plan step 1):
/// the jaws cut, and the crop is inside. Until then a mind could cut food
/// or a dead ant into the jaws, a second way to carry food that counted no
/// delivery, and an ant that had picked food up could not dig until the
/// crop had chewed all of it.
fn dig(world: &mut World, organism: OrganismId, def: &CreatureDef, (tx, ty): (i32, i32)) -> bool {
    let Some((head, free)) = world
        .organism(organism)
        .and_then(|s| s.chain.first().map(|&h| (h, s.spoil.is_none())))
    else {
        return false;
    };
    let cell = world.get(tx, ty);
    // **The pack move** (`options::pack_on`): walled in with a pellet, the cut
    // is `pack_behind`'s -- aimed at the target, the body steps into it and
    // the old pellet goes where the tail was, the new cut in the jaws. No
    // backfill: the pellet stays a pellet.
    if !free && options::pack_on() && options::diggable(world, def, organism, cell) && super::walled_in(world, organism, head) {
        let Some(load) = world.organism(organism).and_then(|s| s.spoil).filter(|l| !l.store) else {
            return false;
        };
        let Some(aim) = DIRS.iter().position(|&d| d == (tx - head.0, ty - head.1)) else {
            return false;
        };
        let packed = super::pack_behind(world, organism, def, head, load, false, Some(aim as u8));
        if packed {
            if let Some(m) = world.minds.as_mut() {
                m.counts.packed += 1;
            }
        }
        return packed;
    }
    if !free || !options::diggable(world, def, organism, cell) {
        return false;
    }
    cut_cell(world, organism, head, (tx, ty), cell);
    if let Some(m) = world.minds.as_mut() {
        m.counts.dug += 1;
    }
    true
}

/// **Put the pellet in the jaws down**, by the game's spoil drop (`act`'s
/// spoil branch): the cell goes down as it was cut -- tamped, so it needs
/// ground under it (`spoil_site_open` with the footing rule's test, which
/// the list already checked) -- and the jaws are free to cut again. **None of the game's placement
/// rules** (first empty neighbour, the lift, the door column's sky test):
/// where soil goes is the mind's, held and carried as food is (owner,
/// `dead-ends.md`'s two hand-written placement rules).
fn put_down_pellet(world: &mut World, organism: OrganismId, (dx, dy): (i32, i32)) -> bool {
    let Some(spoil) = world.organism(organism).and_then(|s| s.spoil) else {
        return false;
    };
    // On ground, as the list offered it (`options::build`) -- or, under scratch `PP_NEEDDROP`, the hungry need's site.
    let need = options::need_drop_on()
        && world.organism(organism).is_some_and(|s| {
            !spoil.store
                && s.chain.first().is_some_and(|&h| {
                    world.species.get(s.species).creature.as_ref().is_some_and(|def| super::needs_hungry(world, s, def, h)) && super::need_drop_site(world, h) == Some((dx, dy))
                })
        });
    if !spoil_site_open(world, dx, dy, true) && !need {
        return false;
    }
    // where it comes to rest (`creature::spoil_rest`), which is also where the dump log scores it
    let (dx, dy) = place_spoil(world, (dx, dy), spoil.cell);
    if let Some(state) = world.organism_mut(organism) {
        state.spoil = None;
        state.spoil_ring = None;
    }
    world.creature_stats.spoil_dumped += 1;
    let frame = world.frame;
    if let Some(m) = world.minds.as_mut() {
        m.counts.dumped += 1;
        if let Some(log) = m.dump_log.as_mut() {
            log.push((dx, dy, frame, organism));
        }
    }
    true
}

/// **Put one crop cell down in a free cell**, by the game's food drop
/// (`act`'s drop branch): the cell leaves at `unit - digesting`, a seed
/// riding in the crop goes down whole with its fruit, and the chewing goes
/// with the cell. At the nest (`nest_within_reach`, the head) it is a
/// delivery, counted as the game counts one. None of the game's holds,
/// relays, store or door rules: where the food goes is the mind's.
fn put_down(world: &mut World, organism: OrganismId, def: &CreatureDef, (dx, dy): (i32, i32)) -> bool {
    // A pellet in the jaws goes down first: the list offered the drop at a
    // spoil site (`options::build`), and a crop never empties past one.
    if world.organism(organism).is_some_and(|s| s.spoil.is_some()) {
        return put_down_pellet(world, organism, (dx, dy));
    }
    let Some((held, (hx, hy))) = world
        .organism(organism)
        .and_then(|s| s.chain.first().map(|&h| (s.crop.filter(|c| c.cells > 0), h)))
    else {
        return false;
    };
    let Some(held) = held else { return false };
    if !world.is_empty(dx, dy) {
        return false;
    }
    let at_nest = nest_within_reach(world, organism, hx, hy, def);
    match held.passenger {
        Some(passenger) => plant::deliver_seed_passenger_uneaten(world, dx, dy, passenger),
        None => {
            let cell = held.unit_cell(quantise_worth).into_cell(world);
            world.set(dx, dy, cell);
        }
    }
    let lumpy = organism::digest_is_lumpy();
    let mut trip_cell = false;
    let mut returned = false;
    if let Some(state) = world.organism_mut(organism) {
        state.crop = state.crop.and_then(|c| {
            let left = c.cells.saturating_sub(1);
            let digesting = if lumpy { c.digesting } else { 0.0 };
            (left > 0).then_some(Crop {
                cells: left,
                digesting,
                passenger: None,
                ..c
            })
        });
        trip_cell = state.trip_cells > 0;
        state.trip_cells = state
            .trip_cells
            .saturating_sub(1)
            .min(state.crop.map_or(0, |c| c.cells));
        if at_nest {
            state.life.deliveries += 1;
            state.life.trip_deliveries += u32::from(trip_cell);
            returned = std::mem::take(&mut state.trip_load);
            state.trip_src = 0;
        }
    }
    let st = &mut world.creature_stats;
    st.drops += 1;
    if at_nest {
        st.deliveries += 1;
        st.trip_deliveries += u64::from(trip_cell);
        st.forage_returns += u64::from(returned);
    }
    note_drop(world, if at_nest { DropWhy::Delivered } else { DropWhy::Placed });
    // SCRATCH (teacher lane, in-run pilot): flag = a trip cell put down at the nest, the game's own delivery.
    super::note_food(world, organism, (dx, dy), super::FOOD_DROP, 0.0, trip_cell && at_nest);
    if let Some(m) = world.minds.as_mut() {
        m.counts.put_down += 1;
        m.counts.delivered += u64::from(at_nest);
    }
    true
}

/// **Feed the nestmate or larva at `target`.** A nestmate poorer than the
/// ant gets a share of the bank, by the game's share (`SHARE_FRACTION` of
/// the gap, downhill only, the odour blended along with it). A larva short
/// of its target gets crop food if the crop holds any it can take
/// (`brood::crop_feed_from`, the nurse's own transfer), else a share of the
/// bank, downhill and no more than it needs. Who, and whether, is the
/// mind's; the amount is fixed, like a mouthful (design rev3, *Physics vs
/// policy*). The caller bills the jaw work.
fn feed(world: &mut World, organism: OrganismId, def: &CreatureDef, (tx, ty): (i32, i32)) -> bool {
    let other = world.get(tx, ty).organism_id();
    let colony = world.colony_of(organism);
    let Some((mine, me_crop)) = world.organism(organism).map(|s| (s.energy, crop_to_feed(world, s))) else {
        return false;
    };
    let Some((theirs, brood, their_colony)) = (other != 0 && other != organism)
        .then(|| world.organism(other))
        .flatten()
        .filter(|o| world.species.get(o.species).creature.is_some())
        .map(|o| (o.energy, o.brood, o.colony))
    else {
        return false;
    };
    if their_colony != world.organism(organism).map_or(0, |s| s.colony) {
        return false;
    }
    let need = match brood {
        None => None,
        Some(b) if b.stage == BroodStage::Larva && theirs < b.target => Some(b.target - theirs),
        Some(_) => return false,
    };
    if let (Some(need), Some(c)) = (need, me_crop) {
        let gain = crate::sim::brood::crop_feed_from(world, other, (tx, ty), colony, def, organism, c, need);
        if gain > 0.0 {
            if let Some(m) = world.minds.as_mut() {
                m.counts.fed_larva_crop += 1;
            }
            return true;
        }
    }
    if theirs >= mine {
        return false;
    }
    let amount = need.map_or(SHARE_FRACTION * (mine - theirs), |n| {
        (SHARE_FRACTION * (mine - theirs)).min(n)
    });
    let frame = world.frame;
    if let Some(s) = world.organism_mut(organism) {
        s.energy -= amount;
        s.last_share_frame = frame;
    }
    if let Some(s) = world.organism_mut(other) {
        s.energy += amount;
        s.last_share_frame = frame;
    }
    let st = &mut world.creature_stats;
    st.shares += 1;
    st.shared_j += amount as f64;
    let taker = world.colony_of(other);
    world.book(colony, Account::SharedOut, amount as f64);
    world.book(taker, Account::SharedIn, amount as f64);
    if need.is_some() {
        world.creature_stats.brood_shared_j += amount as f64;
        note_feed(world, other, (tx, ty), FEED_SHARE, organism, amount);
        if let Some(m) = world.minds.as_mut() {
            m.counts.fed_larva_bank += 1;
        }
    } else {
        blend_with_kin(world, organism, other, world.nest_blend);
        world.creature_stats.share_blends += 1;
        if let Some(m) = world.minds.as_mut() {
            m.counts.fed_adult += 1;
        }
    }
    true
}

/// A step onto `target`, committed exactly as the game's walks commit
/// theirs (`commit_step`), so the body climbs, squeezes and falls by the
/// same rules as any ant.
fn walk(world: &mut World, organism: OrganismId, def: &CreatureDef, target: (i32, i32)) -> bool {
    let Some((chain, groups, fates, heading)) = world
        .organism(organism)
        .map(|s| (s.chain.clone(), s.segment_groups.clone(), s.fates, s.heading))
    else {
        return false;
    };
    let Some(&head) = chain.first() else { return false };
    let Some(to) = DIRS.iter().position(|&d| (head.0 + d.0, head.1 + d.1) == target) else {
        return false;
    };
    let authored = segment_authored(def, fates);
    let authored_widths: Vec<u8> = authored
        .iter()
        .map(|s| if s.lateral.is_some() { 2 } else { 1 })
        .collect();
    let body = BodyShape {
        chain: &chain,
        groups: &groups,
        authored: &authored_widths,
    };
    commit_step(
        world,
        organism,
        def,
        body,
        &authored,
        &authored_widths,
        heading,
        to as u8,
        parting_enabled(),
    );
    // Whether the head actually moved: `true` here unconditionally left
    // `MindCounts::failed` blind to walks (review of PR 713, F8).
    world.organism(organism).and_then(|s| s.chain.first().copied()) != Some(head)
}

/// The body flipped end for end (`flipped_body`), facing the other way.
fn turn_around(world: &mut World, organism: OrganismId, def: &CreatureDef) -> bool {
    let Some((chain, groups, fates, heading)) = world
        .organism(organism)
        .map(|s| (s.chain.clone(), s.segment_groups.clone(), s.fates, s.heading))
    else {
        return false;
    };
    let authored = segment_authored(def, fates);
    let authored_widths: Vec<u8> = authored
        .iter()
        .map(|s| if s.lateral.is_some() { 2 } else { 1 })
        .collect();
    let Some((cells, widths)) = flipped_body(world, &chain, &groups, &authored_widths, parting_enabled()) else {
        return false;
    };
    relocate_chain(
        world,
        organism,
        def,
        &authored,
        BodySide {
            cells: &chain,
            groups: &groups,
        },
        BodySide {
            cells: &cells,
            groups: &widths,
        },
    );
    if let Some(state) = world.organism_mut(organism) {
        state.segment_groups = widths;
        state.heading = (heading + 4) % 8;
    }
    world.creature_stats.reversals += 1;
    true
}

/// **Lay each scent at the strength the mind chose**, under the head,
/// priced as the game prices a deposit (`emit_cost_in_moves`). Returns the
/// energy spent.
fn lay(
    world: &mut World,
    organism: OrganismId,
    def: &CreatureDef,
    colony: u32,
    body_cells: f32,
    emits: [f32; EMITS],
) -> f32 {
    let total: f32 = emits.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let Some(&(hx, hy)) = world.organism(organism).and_then(|s| s.chain.first()) else {
        return 0.0;
    };
    for (e, &s) in emits.iter().enumerate() {
        let amount = (s * pheromone::DEPOSIT as f32) as pheromone::Scent;
        if amount > 0 {
            world.deposit_pheromone(EMIT_CHANNELS[e], hx, hy, amount);
            if let Some(m) = world.minds.as_mut() {
                m.counts.laid[e] += f64::from(s);
            }
        }
    }
    if def.emit_cost_in_moves <= 0.0 {
        return 0.0;
    }
    let cost = def.move_cost_per_cell * body_cells * def.emit_cost_in_moves * total;
    world.book(colony, Account::Metabolized, cost as f64);
    world.creature_stats.emit_energy += cost as f64;
    cost
}

#[cfg(test)]
mod tests {
    use super::super::jaw_can_cut;
    use super::*;

    /// An option whose only feature is how much stronger scent B is there.
    fn opt(verb: Verb, target: (i32, i32), b_diff: f32) -> Opt {
        let mut feats = [0.0; FEATURES];
        feats[options::f::SCENT + 4] = b_diff;
        Opt { verb, target, feats }
    }

    fn walks() -> Vec<Opt> {
        vec![
            opt(Verb::Walk, (1, 0), 0.6),
            opt(Verb::Walk, (-1, 0), -0.6),
            opt(Verb::TurnAround, (-2, 0), 0.0),
            opt(Verb::Rest, (0, 0), 0.0),
        ]
    }

    fn picks(g: &[f32], opts: &[Opt], n: u64) -> Vec<u64> {
        let mut count = vec![0u64; opts.len()];
        for i in 0..n {
            let mut st = MindState::default();
            let mut draw = rng::stream(7, 1, i, RNG_SLOT_MIND);
            let d = decide(g, &[0.0; CONTEXT], opts, &mut st, &mut draw).expect("a pick");
            count[d.pick] += 1;
        }
        count
    }

    /// **The sparse scorer is a speed-up and nothing else** ([`ScorerPlan`]):
    /// over sparse random genomes (about the density a trained one has, with
    /// whole units switched off at the output), random contexts, memories
    /// and option lists, every decision's pick, scores, jobs, emits and new
    /// memory match the dense sum bit for bit, a zero's sign aside.
    #[test]
    fn the_sparse_scorer_decides_exactly_as_the_dense_one() {
        let same = |a: f32, b: f32| a.to_bits() == b.to_bits() || (a == 0.0 && b == 0.0);
        let mut r = rng::stream(11, 2, 3, 99);
        let mut compared = 0;
        for trial in 0..400 {
            let mut g = blank(&Verb::ALL);
            let density = [0.05, 0.25, 0.6][trial % 3];
            for w in g.iter_mut().skip(S_CTX) {
                if r.unit_f32() < density {
                    *w = r.unit_f32() * 4.0 - 2.0;
                }
            }
            for v in Verb::ALL {
                g[SWITCH + v as usize] = if r.unit_f32() < 0.8 { 1.0 } else { -1.0 };
            }
            let ctx: [f32; CONTEXT] = std::array::from_fn(|_| r.unit_f32() * 2.0 - 1.0);
            let mut state = MindState::default();
            for m in state.mem.iter_mut() {
                *m = r.unit_f32() * 2.0 - 1.0;
            }
            let opts: Vec<Opt> = (0..1 + (r.unit_f32() * 20.0) as usize)
                .map(|k| Opt {
                    verb: Verb::ALL[(r.unit_f32() * VERBS as f32) as usize % VERBS],
                    target: (k as i32, 0),
                    feats: std::array::from_fn(|_| if r.unit_f32() < 0.5 { 0.0 } else { r.unit_f32() * 2.0 - 1.0 }),
                })
                .collect();
            let (sparse, dense) = (ScorerPlan::new(&g), ScorerPlan::dense(&g));
            let (mut s1, mut s2) = (state, state);
            let (mut d1, mut d2) = (rng::stream(5, trial as u64, 0, RNG_SLOT_MIND), rng::stream(5, trial as u64, 0, RNG_SLOT_MIND));
            let a = decide_planned(&g, &sparse, &ctx, &opts, &mut s1, &mut d1);
            let b = decide_planned(&g, &dense, &ctx, &opts, &mut s2, &mut d2);
            let (a, b) = match (a, b) {
                (Some(a), Some(b)) => (a, b),
                (None, None) => continue,
                _ => panic!("trial {trial}: one decided and the other did not"),
            };
            compared += 1;
            assert_eq!(a.pick, b.pick, "trial {trial}");
            assert!(a.scores.iter().zip(&b.scores).all(|(x, y)| same(*x, *y)), "trial {trial}: scores {:?} vs {:?}", a.scores, b.scores);
            assert!(a.jobs.iter().zip(&b.jobs).all(|(x, y)| match (x, y) {
                (Some(x), Some(y)) => same(*x, *y),
                (None, None) => true,
                _ => false,
            }), "trial {trial}: jobs");
            assert!(a.emits.iter().zip(&b.emits).all(|(x, y)| same(*x, *y)), "trial {trial}: emits");
            assert!(s1.mem.iter().zip(&s2.mem).all(|(x, y)| same(*x, *y)), "trial {trial}: memory");
            assert_eq!(s1.last_verb, s2.last_verb);
        }
        assert!(compared > 300, "too few trials decided anything: {compared}");
        // The positive control: a plan that drops one live weight is caught.
        let mut g = blank(&Verb::ALL);
        for (i, w) in g.iter_mut().enumerate().skip(S_CTX) {
            *w = ((i * 7919 % 1000) as f32 / 500.0 - 1.0) * 0.5;
        }
        let mut broken = ScorerPlan::new(&g);
        broken.feats[0].1[0] = 0.0;
        // Every feature nonzero, so whichever weight was dropped is read.
        let opts: Vec<Opt> = walks()
            .into_iter()
            .map(|mut o| {
                o.feats = std::array::from_fn(|i| 0.1 + i as f32 / FEATURES as f32);
                o
            })
            .collect();
        let ctx = [0.3; CONTEXT];
        let mut caught = false;
        for k in 0..50 {
            let (mut s1, mut s2) = (MindState::default(), MindState::default());
            let a = decide_planned(&g, &broken, &ctx, &opts, &mut s1, &mut rng::stream(1, k, 0, 0)).expect("pick");
            let b = decide_planned(&g, &ScorerPlan::dense(&g), &ctx, &opts, &mut s2, &mut rng::stream(1, k, 0, 0)).expect("pick");
            caught |= a.scores.iter().zip(&b.scores).any(|(x, y)| !same(*x, *y));
        }
        assert!(caught, "a plan missing a live weight scored the same: the comparison is blind");
    }

    #[test]
    fn a_blank_mind_picks_each_job_and_each_target_fairly() {
        let g = blank(&[Verb::Walk, Verb::TurnAround, Verb::Rest]);
        let c = picks(&g, &walks(), 30_000);
        // Three jobs at a third each; the walk's third split over two steps.
        let near = |got: u64, want: f64| (got as f64 - want).abs() < 0.05 * want;
        assert!(
            near(c[0], 5_000.0) && near(c[1], 5_000.0) && near(c[2], 10_000.0) && near(c[3], 10_000.0),
            "{c:?}"
        );
    }

    #[test]
    fn one_weight_on_the_trail_feature_follows_it() {
        // **The positive control for the trail rung**: a scorer that reads
        // where scent B grows, and a walk job that follows its best target
        // while turning and resting do not pay, should take the stronger
        // side almost always: about 0.95 walk x 0.97 east.
        let mut g = blank(&[Verb::Walk, Verb::TurnAround, Verb::Rest]);
        g[S_FEAT + options::f::SCENT + 4] = 8.0; // unit 0 reads b_diff
        g[S_OUT] = 4.0; // ...into the score
        let job = |v: Verb, k: usize| JOB + v as usize * J_IN + k;
        g[job(Verb::Walk, CONTEXT + MEMORY)] = 4.0; // the walk follows its best target
        g[job(Verb::Walk, CONTEXT + MEMORY + 1)] = 4.0;
        g[job(Verb::TurnAround, CONTEXT + MEMORY + 1)] = -4.0;
        g[job(Verb::Rest, CONTEXT + MEMORY + 1)] = -4.0;
        let c = picks(&g, &walks(), 4_000);
        assert!(c[0] > 3_500, "the strong side should win nearly every draw: {c:?}");
        assert!(c[1] > 0 && c[3] > 0, "and the weak ones keep their floor: {c:?}");
    }

    #[test]
    fn a_switched_off_verb_leaves_every_choice_bit_identical() {
        // *Size, cost and growth*: a new verb enters switched off, and while
        // it is off it must change nothing -- not its share of the pick, and
        // not which option a given draw lands on. Turning around is off here
        // so that the positive control below can switch on a verb that acts.
        let mut g = blank(&[Verb::Walk, Verb::Rest]);
        for (i, w) in g.iter_mut().enumerate().skip(S_CTX) {
            *w = ((i * 7919 % 1000) as f32 / 500.0 - 1.0) * 0.5;
        }
        let ctx: [f32; CONTEXT] = std::array::from_fn(|i| i as f32 / CONTEXT as f32);
        let base: Vec<Opt> = walks().into_iter().filter(|o| o.verb != Verb::TurnAround).collect();
        // The same list with three off verbs that act (turning around,
        // putting down, digging) threaded through it.
        let more = vec![
            base[0],
            opt(Verb::Dig, (1, 1), 0.3),
            base[1],
            opt(Verb::TurnAround, (-2, 0), 0.0),
            base[2],
            opt(Verb::Drop, (0, 1), 0.0),
        ];
        let map = [0usize, 2, 4];
        let run = |g: &[f32], i: u64| {
            let (mut s1, mut s2) = (MindState::default(), MindState::default());
            (0..5).all(|step| {
                let mut d1 = rng::stream(3, 9, i * 10 + step, RNG_SLOT_MIND);
                let mut d2 = rng::stream(3, 9, i * 10 + step, RNG_SLOT_MIND);
                let a = decide(g, &ctx, &base, &mut s1, &mut d1).expect("pick");
                let b = decide(g, &ctx, &more, &mut s2, &mut d2).expect("pick");
                map[a.pick] == b.pick
                    && s1.mem.map(f32::to_bits) == s2.mem.map(f32::to_bits)
                    && a.emits.map(f32::to_bits) == b.emits.map(f32::to_bits)
            })
        };
        let moved = (0..2_000).filter(|&i| !run(&g, i)).count();
        assert_eq!(
            moved, 0,
            "the off verbs moved a pick, the memory or an emit in {moved} of 2000 runs"
        );
        // The positive control: switch turning around on and the same runs
        // do move -- the guard can see an extra verb when one is live.
        g[SWITCH + Verb::TurnAround as usize] = 1.0;
        let moved = (0..2_000).filter(|&i| !run(&g, i)).count();
        assert!(
            moved > 500,
            "an on, acting verb must move the picks: {moved} of 2000 runs moved"
        );
    }

    /// `g` in the layout before PR 2 ([`GENOME_LEN_V2`]): each unit's
    /// `f::LINED` and `f::ROOF_DROP` weights and every `c::ROOF_CUT` and
    /// `c::HOLE_DROP` read dropped.
    fn older(g: &[f32]) -> Vec<f32> {
        v2_slots().into_iter().map(|k| g[k]).collect()
    }

    /// Every slot PR 2 added: no slot of an older genome lands there.
    fn grown() -> Vec<usize> {
        let old: std::collections::HashSet<usize> = v2_slots().into_iter().collect();
        (0..GENOME_LEN).filter(|k| !old.contains(k)).collect()
    }

    /// `g` as [`pad`] would leave it: what it zeroes zeroed, each unit's
    /// `f::LINED` weight its `f::SPOIL` one, and every other slot PR 2
    /// added at zero.
    fn as_padded(g: &mut [f32]) {
        for k in newly_live().into_iter().chain(grown()) {
            g[k] = 0.0;
        }
        for u in 0..UNITS {
            g[S_FEAT + u * FEATURES + options::f::LINED] = g[S_FEAT + u * FEATURES + options::f::SPOIL];
        }
    }

    /// **A genome saved before PR 2 reads into today's layout slot by
    /// slot** ([`pad`]): every older weight lands on the input it read, the
    /// wall's weight is the heap's, the roof feature and the roof-cut
    /// sense start at zero, and the inputs PR 2 brought to life are zeroed
    /// -- `c::DEPTH` in all five blocks that read context, `f::DEPTH` and
    /// `f::CUT` in all 16 units. The input each slot reads is checked by
    /// name, so a relayout that kept the count and moved a slot fails.
    #[test]
    fn a_genome_saved_before_pr2_reads_slot_by_slot() {
        let old: Vec<f32> = (0..GENOME_LEN_V2).map(|i| (i % 997) as f32 + 1.0).collect();
        let g = pad(old.clone()).expect("a V2 genome pads");
        let live = newly_live();
        assert_eq!(live.len(), UNITS + VERBS + MEMORY + EMITS + HIDDEN + 2 * UNITS);
        for &k in &live {
            assert_eq!(g[k], 0.0, "slot {k} reads an input PR 2 brought to life");
        }
        let to = v2_slots();
        assert_eq!(to.len(), GENOME_LEN_V2);
        for (k, &n) in to.iter().enumerate().filter(|(_, n)| !live.contains(n)) {
            assert_eq!(g[n], old[k], "old slot {k} -> {n}");
        }
        // What each moved slot reads, by name, in the layout it was saved
        // in and in today's: a context read stays on its input.
        let ctx_old = context_names();
        for (base_old, width_old, base, width, rows) in [
            (S_CTX, CONTEXT_V2 + MEMORY + 1, S_CTX, S_IN, UNITS),
            (
                S_CTX + UNITS * (CONTEXT_V2 + MEMORY + 1) + UNITS * FEATURES_V2 + UNITS * VERBS + UNITS + 1,
                CONTEXT_V2 + MEMORY + 2,
                JOB,
                J_IN,
                VERBS,
            ),
        ] {
            for r in 0..rows {
                for i in 0..CONTEXT_V2 {
                    assert_eq!(
                        to[base_old + r * width_old + i],
                        base + r * width + i,
                        "{} in row {r}",
                        ctx_old[i]
                    );
                }
                // The first memory read follows the new input.
                assert_eq!(to[base_old + r * width_old + CONTEXT_V2], base + r * width + CONTEXT);
            }
        }
        for k in grown() {
            let feat = k
                .checked_sub(S_FEAT)
                .filter(|&d| d < UNITS * FEATURES)
                .map(|d| d % FEATURES);
            if feat == Some(options::f::LINED) {
                let spoil = k - options::f::LINED + options::f::SPOIL;
                assert_eq!(g[k], g[spoil], "slot {k}: the wall scores as the heap did");
            } else {
                assert_eq!(g[k], 0.0, "slot {k} is new in PR 2");
            }
        }
        assert_eq!(grown().len(), GREW);
        // A V1 genome is a V2 one without the hidden units.
        let v1 = pad(old[..GENOME_LEN_V1].to_vec()).expect("a V1 genome pads");
        assert_eq!(&v1[..HID], &g[..HID]);
        assert!(v1[HID..].iter().all(|w| *w == 0.0));
    }

    /// **The job head's hidden units cost a genome saved before them
    /// nothing**: it pads with them at zero ([`pad`]); a unit with a fan-in
    /// that no job reads is skipped, so every pick, memory and emit is
    /// bit-identical to the same genome without it; and the positive
    /// control, one job's weight on that unit, moves the picks.
    #[test]
    fn unread_hidden_units_leave_every_choice_bit_identical() {
        let mut g = blank(&[Verb::Walk, Verb::TurnAround, Verb::Rest]);
        for (i, w) in g.iter_mut().enumerate().take(HID).skip(S_CTX) {
            *w = ((i * 7919 % 1000) as f32 / 500.0 - 1.0) * 0.5;
        }
        as_padded(&mut g);
        assert_eq!(pad(older(&g)[..GENOME_LEN_V1].to_vec()).as_deref(), Some(&g[..]));
        assert_eq!(pad(g.clone()).as_deref(), Some(&g[..]));
        assert!(pad(vec![0.0; GENOME_LEN_V1 - 1]).is_none());
        let mut h = g.clone();
        for (i, w) in h.iter_mut().enumerate().take(HID_OUT).skip(HID) {
            *w = (i * 104_729 % 1000) as f32 / 250.0 - 2.0;
        }
        let ctx: [f32; CONTEXT] = std::array::from_fn(|i| i as f32 / CONTEXT as f32);
        let run = |h: &[f32], i: u64| {
            let (mut s1, mut s2) = (MindState::default(), MindState::default());
            (0..5).all(|step| {
                let mut d1 = rng::stream(3, 9, i * 10 + step, RNG_SLOT_MIND);
                let mut d2 = rng::stream(3, 9, i * 10 + step, RNG_SLOT_MIND);
                let a = decide(&g, &ctx, &walks(), &mut s1, &mut d1).expect("pick");
                let b = decide(h, &ctx, &walks(), &mut s2, &mut d2).expect("pick");
                a.pick == b.pick
                    && a.jobs.map(|j| j.map(f32::to_bits)) == b.jobs.map(|j| j.map(f32::to_bits))
                    && s1.mem.map(f32::to_bits) == s2.mem.map(f32::to_bits)
                    && a.emits.map(f32::to_bits) == b.emits.map(f32::to_bits)
            })
        };
        let moved = (0..2_000).filter(|&i| !run(&h, i)).count();
        assert_eq!(moved, 0, "unread hidden units moved a decision in {moved} of 2000 runs");
        h[HID_OUT + Verb::Walk as usize * HIDDEN + 3] = 2.0;
        let moved = (0..2_000).filter(|&i| !run(&h, i)).count();
        assert!(moved > 500, "a read hidden unit must move the walk's job: {moved} of 2000 runs moved");
    }

    /// **A hidden unit reads its bias, this decision's context and the
    /// memory before the update** ([`hidden_units`]), and an unread one stays
    /// 0 whatever its fan-in (review of PR 721, N1: removing the memory read
    /// left every test green).
    #[test]
    fn a_hidden_unit_reads_bias_context_and_memory() {
        let mut g = blank(&[Verb::Walk, Verb::TurnAround, Verb::Rest]);
        let (h, m) = (2, 4);
        let row = HID + h * H_IN;
        g[row + H_IN - 1] = 0.3;
        g[row + c::CROP] = 1.0;
        g[row + CONTEXT + m] = 2.0;
        g[HID_OUT + Verb::Walk as usize * HIDDEN + h] = 1.0;
        g[HID + 5 * H_IN + c::CROP] = 9.0;
        let mut ctx = [0.0; CONTEXT];
        ctx[c::CROP] = 0.25;
        let mut mem = [0.0; MEMORY];
        mem[m] = 0.5;
        let hid = hidden_units(&g, &ctx, &mem);
        assert_eq!(hid[h], squash(0.3 + 0.25 + 1.0), "bias + context + memory");
        assert_eq!(hid[5], 0.0, "unit 5 has a fan-in but no job reads it");
        assert!(hid.iter().enumerate().all(|(k, x)| k == h || *x == 0.0));
    }

    /// **Entering the dig rung** ([`enter_dig`]): the weights that drifted
    /// while Dig was banned are zeroed, Dig cuts at the nest, and the soil
    /// gate on hidden units 0 and 1 leaves food's put-down at the nest
    /// where the start had it while soil is held at the nest and put down
    /// far out (`dig-rung-design.md` 5's truth table, on the forager).
    #[test]
    fn entering_dig_zeroes_the_drift_and_lays_the_soil_gate() {
        let forage = [Verb::Walk, Verb::TurnAround, Verb::Rest, Verb::PickUp, Verb::Eat, Verb::Drop, Verb::Feed];
        let soil = [Verb::Walk, Verb::TurnAround, Verb::Rest, Verb::PickUp, Verb::Eat, Verb::Drop, Verb::Feed, Verb::Dig];
        let start = preset("forager", &forage).expect("forager");
        let dig = Verb::Dig as usize;
        let drift = [
            JOB + dig * J_IN + c::ENERGY,
            S_CTX + 5 * S_IN + c::SPOIL,
            JOB + Verb::Walk as usize * J_IN + c::SPOIL,
            MEM + 2 * M_IN + CONTEXT + 2 + dig,
            S_VERB + 4 * VERBS + dig,
            HID + 6 * H_IN + c::SPOIL,
            // Dig's weight on h0 (review of PR 721, S1): left in place it
            // would read as h0 in use, and no gate would be laid.
            HID_OUT + dig * HIDDEN,
        ];
        let mut g = start.clone();
        for (k, &i) in drift.iter().enumerate() {
            g[i] = 0.3 + k as f32;
        }
        let did = enter_rung(&mut g, &soil, [true, true, false]);
        assert_eq!(did[..3], ["dig", "7 drifted Dig and soil links zeroed", "Dig at the nest"]);
        assert!(did[3].starts_with("soil put down away"), "{did:?}");
        assert!(drift.iter().all(|&i| g[i] == 0.0));
        assert_eq!(enter_rung(&mut g, &soil, [true, true, false]), Vec::<&str>::new(), "entered twice");
        // A genome whose Walk already reads h0 keeps it, and says the gate
        // was not laid (review of PR 721, N3).
        let mut busy = start.clone();
        busy[HID_OUT + Verb::Walk as usize * HIDDEN] = 1.0;
        let did = enter_rung(&mut busy, &soil, [true, true, false]);
        assert_eq!(did[3], "soil gate not laid: hidden 0 or 1 already in use", "{did:?}");
        assert_eq!(busy[HID + c::SPOIL], 0.0);

        let drop_job = |g: &[f32], set: &[(usize, f32)]| {
            let mut ctx = [0.0; CONTEXT];
            for &(k, v) in set {
                ctx[k] = v;
            }
            let opts = [opt(Verb::Walk, (1, 0), 0.0), opt(Verb::Drop, (1, 1), 0.0)];
            let mut draw = rng::stream(1, 1, 1, RNG_SLOT_MIND);
            decide(g, &ctx, &opts, &mut MindState::default(), &mut draw).expect("pick").jobs[Verb::Drop as usize].expect("drop")
        };
        let food = 1.0 / 6.0;
        for (row, set) in [
            ("food at the nest", vec![(c::CROP, food), (c::AT_NEST, 1.0)]),
            ("food away", vec![(c::CROP, food), (c::HOME_DIST, 0.3)]),
        ] {
            let (was, now) = (drop_job(&start, &set), drop_job(&g, &set));
            assert!((was - now).abs() < 0.02, "{row}: Drop's job {was} at the start, {now} entered");
        }
        let at_nest = drop_job(&g, &[(c::SPOIL, 1.0), (c::AT_NEST, 1.0)]);
        let roofed = drop_job(&g, &[(c::SPOIL, 1.0), (c::ROOFED, 1.0), (c::HOME_DIST, 0.1)]);
        let near = drop_job(&g, &[(c::SPOIL, 1.0), (c::HOME_DIST, 0.05)]);
        let far = drop_job(&g, &[(c::SPOIL, 1.0), (c::HOME_DIST, 0.4)]);
        // On the forager's sharpness (4): food at the nest reads +0.89.
        assert!(at_nest < -0.97 && roofed < -0.97, "soil at the nest {at_nest}, under cover {roofed}: must hold");
        assert!(near < -0.9 && far > 0.9, "soil near home {near}, far out {far}");
    }


    /// **Zeroing the dead links moves no decision** ([`zero_dead`]): a
    /// genome wired everywhere, with three scorer units nothing reads, two
    /// hidden units no job reads, a memory unit read only by those, and the
    /// verbs the rung does not offer, picks, scores, weighs its jobs, lays
    /// and remembers (what is still read) bit for bit as before over 2,000
    /// five-step runs; every kind is cut, the count is the bill's, a second
    /// pass finds nothing, and a banned verb keeps its job's offset.
    /// Positive control: one live link zeroed moves decisions.
    #[test]
    fn zeroing_dead_links_leaves_every_choice_bit_identical() {
        use options::f;
        let offered = [
            Verb::Walk,
            Verb::TurnAround,
            Verb::Rest,
            Verb::Eat,
            Verb::Drop,
            Verb::Dig,
        ];
        let emits = [true, false, false];
        let mut g = blank(&offered);
        for (i, w) in g.iter_mut().enumerate().skip(S_CTX) {
            *w = ((i * 7919 % 1000) as f32 / 500.0 - 1.0) * 0.5;
        }
        apply_bans(&mut g, &offered, emits);
        for u in [2, 5, 11] {
            g[S_OUT + u] = 0.0;
        }
        for h in [6, 7] {
            for v in 0..VERBS {
                g[HID_OUT + v * HIDDEN + h] = 0.0;
            }
        }
        // Memory 9 is read only by unit 5 and hidden 7, so it goes once they have.
        let m = 9;
        for u in (0..UNITS).filter(|&u| u != 5) {
            g[S_CTX + u * S_IN + CONTEXT + m] = 0.0;
        }
        for v in 0..VERBS {
            g[JOB + v * J_IN + CONTEXT + m] = 0.0;
        }
        for h in (0..HIDDEN).filter(|&h| h != 7) {
            g[HID + h * H_IN + CONTEXT + m] = 0.0;
        }
        for e in 0..EMITS {
            g[EMIT + e * E_IN + CONTEXT + m] = 0.0;
        }
        for r in (0..MEMORY).filter(|&r| r != m) {
            g[MEM + r * M_IN + CONTEXT + 2 + VERBS + m] = 0.0;
        }
        let mut z = g.clone();
        let cut = zero_dead(&mut z, &offered, emits);
        assert!(cut.iter().all(|&n| n > 0), "every kind cut: {cut:?}");
        assert!(
            z[MEM + m * M_IN..MEM + (m + 1) * M_IN].iter().all(|w| *w == 0.0),
            "memory {m}, read only by the dead, goes too"
        );
        let attack = JOB + Verb::Attack as usize * J_IN + CONTEXT + MEMORY + 1;
        assert!(
            z[attack] == g[attack] && g[attack] != 0.0,
            "a banned verb keeps its job's offset"
        );
        let live = |g: &[f32]| MindGenome::new(g.to_vec()).live as usize;
        assert_eq!(
            live(&g) - live(&z),
            cut.iter().sum::<usize>(),
            "the count is the bill's"
        );
        assert_eq!(
            zero_dead(&mut z.clone(), &offered, emits),
            [0; 4],
            "a second pass finds nothing"
        );
        let kept: Vec<usize> = (0..MEMORY)
            .filter(|&r| z[MEM + r * M_IN..MEM + (r + 1) * M_IN].iter().any(|w| *w != 0.0))
            .collect();
        let verbs = [
            Verb::Walk,
            Verb::Walk,
            Verb::TurnAround,
            Verb::Rest,
            Verb::Eat,
            Verb::Drop,
            Verb::Dig,
            Verb::Attack,
            Verb::Feed,
        ];
        let opts: Vec<Opt> = verbs
            .iter()
            .enumerate()
            .map(|(o, &verb)| {
                let mut feats: [f32; FEATURES] = std::array::from_fn(|k| ((o * 31 + k * 17) % 11) as f32 / 10.0);
                for k in UNWRITTEN {
                    feats[k] = 0.0;
                }
                Opt {
                    verb,
                    target: (o as i32, 0),
                    feats,
                }
            })
            .collect();
        let run = |h: &[f32], i: u64| {
            let (mut s1, mut s2) = (MindState::default(), MindState::default());
            (0..5u64).all(|step| {
                let ctx: [f32; CONTEXT] =
                    std::array::from_fn(|k| ((i * 7 + step * 13) as usize + k * 5) as f32 % 9.0 / 8.0);
                let mut d1 = rng::stream(3, 9, i * 10 + step, RNG_SLOT_MIND);
                let mut d2 = rng::stream(3, 9, i * 10 + step, RNG_SLOT_MIND);
                let a = decide(&g, &ctx, &opts, &mut s1, &mut d1).expect("pick");
                let b = decide(h, &ctx, &opts, &mut s2, &mut d2).expect("pick");
                a.pick == b.pick
                    && a.scores
                        .iter()
                        .map(|x| x.to_bits())
                        .eq(b.scores.iter().map(|x| x.to_bits()))
                    && a.jobs.map(|j| j.map(f32::to_bits)) == b.jobs.map(|j| j.map(f32::to_bits))
                    && a.emits.map(f32::to_bits) == b.emits.map(f32::to_bits)
                    && kept.iter().all(|&r| s1.mem[r].to_bits() == s2.mem[r].to_bits())
            })
        };
        let moved = (0..2_000).filter(|&i| !run(&z, i)).count();
        assert_eq!(moved, 0, "dead links zeroed moved a decision in {moved} of 2000 runs");
        let mut one = z.clone();
        assert!(
            z[S_OUT] != 0.0 && z[S_FEAT + f::FOOD] != 0.0,
            "unit 0 is read and reads food"
        );
        one[S_FEAT + f::FOOD] = 0.0;
        let moved = (0..2_000).filter(|&i| !run(&one, i)).count();
        assert!(
            moved > 100,
            "a live link zeroed must move decisions: {moved} of 2000 runs moved"
        );
    }
    /// **A genome saved before Pick up had its act picks up where it
    /// swallowed** ([`split_pick_up`]): wired everywhere, with food aimed
    /// at by Eat before and by Pick up after, over 2,000 decisions from
    /// varied context, memory and last verb, every option scores and every
    /// job weighs bit for bit as before (Eat's job read as Pick up's), so
    /// each verb and target is drawn with the same chance; and wherever the
    /// two pick the same option, memory and the scents laid are bit for bit
    /// the same too. Not the same pick for the same draw: the job is drawn
    /// in [`Verb::ALL`]'s order, where Pick up sits before Put down and Eat
    /// after it, so one draw lands on a different verb about half the time
    /// (1,027 of 2,000 when this test first asked for that). Eat is quiet
    /// (its job reads nothing, its constant is [`QUIET`]) and reads no
    /// hidden unit. Positive control: the same conversion without Eat's job
    /// row copied moves the jobs.
    #[test]
    fn a_saved_forager_converts_to_pick_up_and_picks_as_it_swallowed() {
        let offered = [Verb::Walk, Verb::TurnAround, Verb::Rest, Verb::PickUp, Verb::Eat, Verb::Drop, Verb::Feed];
        let mut old = blank(&offered);
        for (i, w) in old.iter_mut().enumerate().skip(S_CTX) {
            *w = ((i * 7919 % 1000) as f32 / 500.0 - 1.0) * 0.5;
        }
        // Saved before step 1: Pick up banned, its links drifted.
        old[SWITCH + Verb::PickUp as usize] = -1.0;
        let (eat, pick) = (Verb::Eat as usize, Verb::PickUp as usize);
        let mut new = old.clone();
        split_pick_up(&mut new);
        assert_eq!(new[SWITCH + pick], old[SWITCH + eat]);
        let eat_job = &new[JOB + eat * J_IN..JOB + (eat + 1) * J_IN];
        assert!(eat_job[..J_IN - 1].iter().all(|w| *w == 0.0) && eat_job[J_IN - 1] == QUIET, "Eat quiet");
        assert!(new[HID_OUT + eat * HIDDEN..HID_OUT + (eat + 1) * HIDDEN].iter().all(|w| *w == 0.0));
        let was = [Verb::Walk, Verb::Walk, Verb::TurnAround, Verb::Rest, Verb::Eat, Verb::Eat, Verb::Drop, Verb::Feed];
        let opts = |eat_is: Verb| -> Vec<Opt> {
            was.iter()
                .enumerate()
                .map(|(o, &verb)| {
                    let feats: [f32; FEATURES] = std::array::from_fn(|k| ((o * 31 + k * 17) % 11) as f32 / 10.0);
                    let verb = if verb == Verb::Eat { eat_is } else { verb };
                    Opt { verb, target: (o as i32, 0), feats }
                })
                .collect()
        };
        let (before, after) = (opts(Verb::Eat), opts(Verb::PickUp));
        let agreed = std::cell::Cell::new(0);
        let same = |h: &[f32], i: u64| {
            let mem: [f32; MEMORY] = std::array::from_fn(|k| ((i as usize * 13 + k * 7) % 17) as f32 / 8.0 - 1.0);
            let last = [Verb::Walk, Verb::Rest, Verb::Eat, Verb::Drop][i as usize % 4];
            let mut s1 = MindState { mem, last_verb: Some(last), ..MindState::default() };
            let renamed = if last == Verb::Eat { Verb::PickUp } else { last };
            let mut s2 = MindState { mem, last_verb: Some(renamed), ..MindState::default() };
            let ctx: [f32; CONTEXT] = std::array::from_fn(|k| ((i * 7) as usize + k * 5) as f32 % 9.0 / 8.0);
            let mut d1 = rng::stream(3, 9, i, RNG_SLOT_MIND);
            let mut d2 = rng::stream(3, 9, i, RNG_SLOT_MIND);
            let a = decide(&old, &ctx, &before, &mut s1, &mut d1).expect("pick");
            let b = decide(h, &ctx, &after, &mut s2, &mut d2).expect("pick");
            let mut jobs = a.jobs;
            jobs.swap(eat, pick);
            agreed.set(agreed.get() + usize::from(a.pick == b.pick));
            a.scores.iter().map(|x| x.to_bits()).eq(b.scores.iter().map(|x| x.to_bits()))
                && jobs.map(|j| j.map(f32::to_bits)) == b.jobs.map(|j| j.map(f32::to_bits))
                && (a.pick != b.pick
                    || (a.emits.map(f32::to_bits) == b.emits.map(f32::to_bits) && s1.mem.map(f32::to_bits) == s2.mem.map(f32::to_bits)))
        };
        let moved = (0..2_000).filter(|&i| !same(&new, i)).count();
        assert_eq!(moved, 0, "the converted genome moved a score or a job in {moved} of 2000 decisions");
        assert!(agreed.get() > 500, "the memory check ran on {} decisions", agreed.get());
        let mut half = new.clone();
        half.copy_within(JOB + pick * J_IN..JOB + (pick + 1) * J_IN, JOB + eat * J_IN);
        half[JOB + pick * J_IN..JOB + (pick + 1) * J_IN].copy_from_slice(&old[JOB + pick * J_IN..JOB + (pick + 1) * J_IN]);
        let moved = (0..2_000).filter(|&i| !same(&half, i)).count();
        assert!(moved > 100, "Pick up's own drifted job row must move the jobs: {moved} of 2000 decisions moved");
    }

    /// **The soil vetoes** ([`lay_vetoes`]), on the forager entering the
    /// dig rung: they take the two highest scorer units it leaves unread;
    /// with soil, a Drop target in the hole scores below one out of it,
    /// and the Drop job goes off when every target is in the hole
    /// ([`c::HOLE_DROP`]); a
    /// roof-dropping cut scores below a safe one, and the Dig job goes off
    /// when every cut drops the roof ([`c::ROOF_CUT`]); with food, the hole
    /// is no worse a place to put it down. A start already digging gets
    /// them once.
    #[test]
    fn the_soil_vetoes_stop_soil_into_the_hole_and_roof_dropping_cuts() {
        use options::f;
        let forage = [
            Verb::Walk,
            Verb::TurnAround,
            Verb::Rest,
            Verb::PickUp,
            Verb::Eat,
            Verb::Drop,
            Verb::Feed,
        ];
        let soil = [
            Verb::Walk,
            Verb::TurnAround,
            Verb::Rest,
            Verb::PickUp,
            Verb::Eat,
            Verb::Drop,
            Verb::Feed,
            Verb::Dig,
        ];
        let start = preset("forager", &forage).expect("forager");
        let mut g = start.clone();
        let did = enter_rung(&mut g, &soil, [true, true, false]);
        let laid = did
            .iter()
            .find(|l| l.starts_with("soil vetoes"))
            .expect("the vetoes logged");
        assert!(
            laid.starts_with("soil vetoes: Drop into the hole on unit 15, roof-dropping cuts on unit 14"),
            "{did:?}"
        );
        let run = |g: &[f32], set: &[(usize, f32)], opts: &[Opt]| {
            let mut ctx = [0.0; CONTEXT];
            for &(k, v) in set {
                ctx[k] = v;
            }
            let mut draw = rng::stream(1, 1, 1, RNG_SLOT_MIND);
            decide(g, &ctx, opts, &mut MindState::default(), &mut draw).expect("pick")
        };
        let with = |verb: Verb, k: usize, on: bool| {
            let mut o = opt(verb, (1, 1), 0.0);
            o.feats[k] = f32::from(u8::from(on));
            o
        };
        let walk = opt(Verb::Walk, (1, 0), 0.0);
        let (drop, dig) = (Verb::Drop as usize, Verb::Dig as usize);
        let hole = |on: bool| with(Verb::Drop, f::CUT, on);
        let roof = |on: bool| with(Verb::Dig, f::ROOF_DROP, on);

        // Soil far out, where h1 says put it down.
        let far = [(c::SPOIL, 1.0), (c::HOME_DIST, 0.4)];
        let d = run(&g, &far, &[walk, hole(true), hole(false)]);
        assert!(
            d.scores[1] < -0.9 && d.scores[2] > 0.5,
            "soil: into the hole {}, beside it {}",
            d.scores[1],
            d.scores[2]
        );
        let only_hole = [(c::SPOIL, 1.0), (c::HOME_DIST, 0.4), (c::HOLE_DROP, 1.0)];
        let (only, open) = (
            run(&g, &only_hole, &[walk, hole(true)]).jobs[drop],
            run(&g, &far, &[walk, hole(false)]).jobs[drop],
        );
        assert!(
            only.expect("drop") < -0.9 && open.expect("drop") > 0.9,
            "soil far out: Drop's job {only:?} with only the hole, {open:?} beside it"
        );
        // Food: the founding chamber is the hole, and food still goes down
        // there, and away, as the start put it down.
        let nest = [(c::CROP, 1.0 / 6.0), (c::AT_NEST, 1.0), (c::HOLE_DROP, 1.0)];
        let d = run(&g, &nest, &[walk, hole(true), hole(false)]);
        assert!(
            (d.scores[1] - d.scores[2]).abs() < 0.05,
            "food: into the hole {}, beside it {}",
            d.scores[1],
            d.scores[2]
        );
        for (row, set, o) in [
            ("at the nest", &nest[..], hole(true)),
            ("away", &[(c::CROP, 1.0 / 6.0), (c::HOME_DIST, 0.3)][..], hole(false)),
        ] {
            let (was, now) = (
                run(&start, set, &[walk, o]).jobs[drop],
                run(&g, set, &[walk, o]).jobs[drop],
            );
            assert!(
                (was.expect("drop") - now.expect("drop")).abs() < 0.02,
                "food {row}: Drop's job {was:?} at the start, {now:?} entered"
            );
        }

        // A cut at the nest, where Dig's job is on.
        let at = [(c::AT_NEST, 1.0)];
        let d = run(&g, &at, &[walk, roof(true), roof(false)]);
        assert!(
            d.scores[1] < d.scores[2] - 1.0,
            "a roof-dropping cut {}, a safe one {}",
            d.scores[1],
            d.scores[2]
        );
        assert_eq!(
            (
                roof_cut(&[walk, roof(true)]),
                roof_cut(&[walk, roof(true), roof(false)]),
                roof_cut(&[walk])
            ),
            (1.0, 0.0, 0.0)
        );
        let (safe, only) = (
            run(&g, &at, &[walk, roof(false)]).jobs[dig].expect("dig"),
            run(&g, &[(c::AT_NEST, 1.0), (c::ROOF_CUT, 1.0)], &[walk, roof(true)]).jobs[dig].expect("dig"),
        );
        assert!(
            safe > 0.5 && only < -0.9,
            "Dig's job at the nest: {safe} with a safe cut, {only} when every cut drops the roof"
        );

        // Entered twice, nothing more; a start already digging gets them.
        assert!(enter_rung(&mut g, &soil, [true, true, false]).is_empty());
        // A dead link training regrew survives a later entry (a `score=`
        // load, review of PR 722, S1): only the entry that opens Dig or lays
        // the vetoes cuts. The control: the link is dead, `zero_dead` cuts it.
        let unread = (0..UNITS)
            .find(|&u| g[S_OUT + u] == 0.0)
            .expect("an unread scorer unit");
        let link = S_FEAT + unread * FEATURES + f::FOOD;
        let mut regrown = g.clone();
        regrown[link] = 0.5;
        assert_eq!(
            zero_dead(&mut regrown.clone(), &soil, [true, true, false]),
            [1, 0, 0, 0],
            "the control: the regrown link is dead"
        );
        assert!(
            enter_rung(&mut regrown, &soil, [true, true, false]).is_empty(),
            "a re-entry cut a link training paid for"
        );
        assert_eq!(regrown[link], 0.5);
        // The roof veto's one weight pruned away: the Dig job's read still
        // marks the vetoes laid, so no second set goes over the first (N2).
        let mut pruned = g.clone();
        for u in 0..UNITS {
            pruned[S_FEAT + u * FEATURES + f::ROOF_DROP] = 0.0;
        }
        assert!(
            enter_rung(&mut pruned, &soil, [true, true, false]).is_empty(),
            "the vetoes laid twice"
        );
        let mut digging = start.clone();
        digging[SWITCH + dig] = 1.0;
        let before = digging.clone();
        let did = enter_rung(&mut digging, &soil, [true, false, false]);
        assert_eq!(
            did.iter().filter(|l| l.starts_with("soil vetoes")).count(),
            1,
            "{did:?}"
        );
        assert!(
            did.iter()
                .all(|l| l.starts_with("soil vetoes") || l.starts_with("dead links zeroed")),
            "{did:?}"
        );
        // ...and wherever the hole gate is off, Drop's job is what it was:
        // its off level is taken back exactly, state by state.
        for set in [
            &[(c::CROP, 1.0 / 6.0), (c::AT_NEST, 1.0), (c::HOLE_DROP, 1.0)][..],
            &[(c::CROP, 1.0 / 6.0), (c::HOME_DIST, 0.3)],
            &[(c::SPOIL, 1.0), (c::HOME_DIST, 0.4)],
        ] {
            let (was, now) = (
                run(&before, set, &[walk, hole(false)]).jobs[drop],
                run(&digging, set, &[walk, hole(false)]).jobs[drop],
            );
            assert!(
                (was.expect("drop") - now.expect("drop")).abs() < 1e-4,
                "{set:?}: Drop's job {was:?} before the vetoes, {now:?} after"
            );
        }
    }

    /// A lab box's colony after 300 frames of the game, every adult then
    /// given a blank mind with the forage rung's verbs.
    fn mind_box() -> (World, CreatureDef, Vec<OrganismId>) {
        let mut w = crate::lab::scene::LabBox {
            founders: 4,
            colonies: 1,
            colony_ants: 12,
            width: 256,
            height: 128,
            ground_y: 64,
            soil_depth: 40,
            ..crate::lab::scene::LabBox::default()
        }
        .build();
        for _ in 0..300 {
            crate::sim::update::step(&mut w);
            w.step_active_sites();
            w.step_fields();
            w.step_pheromones();
        }
        let sp = w.species.id_of("ant").expect("ant");
        let def = w.species.get(sp).creature.clone().expect("creature");
        let g = Arc::new(MindGenome::new(blank(&[
            Verb::Walk,
            Verb::TurnAround,
            Verb::Rest,
            Verb::PickUp,
            Verb::Eat,
            Verb::Drop,
            Verb::Feed,
        ])));
        let ids: Vec<OrganismId> = w
            .live_organism_ids()
            .into_iter()
            .filter(|&id| w.organism(id).is_some_and(|s| s.species == sp && s.brood.is_none()))
            .collect();
        assert!(ids.len() >= 6, "{} adults", ids.len());
        for &id in &ids {
            give(&mut w, id, g.clone());
        }
        (w, def, ids)
    }

    /// **An ant that starves holding soil is counted, and with food in
    /// reach as stuck** ([`MindCounts::starved_soil`]): jaws holding soil
    /// cannot pick food up, so this is the count that says whether that
    /// rule starves anyone. Three ants run out of energy on one tick: one
    /// holding soil beside food, one holding soil with none in reach, and
    /// the control, beside food with free jaws, which dies uncounted.
    #[test]
    fn an_ant_that_starves_holding_soil_is_counted() {
        let (mut w, def, ids) = mind_box();
        let food = w.materials.id_of("provisions").expect("provisions");
        let soil = w.materials.id_of("soil").expect("soil");
        let open: Vec<(OrganismId, (i32, i32))> = ids
            .iter()
            .map(|&id| (id, head_of(&w, id)))
            .filter(|&(_, (x, y))| w.get(x + 1, y).material == crate::sim::material::EMPTY)
            .collect();
        assert!(open.len() >= 3, "{} ants with open ground ahead", open.len());
        let cases = [(true, true), (true, false), (false, true)];
        for (&(id, (hx, hy)), &(holding, beside)) in open.iter().zip(&cases) {
            // Clear the head's ring of food first, so "none in reach" is.
            for &(dx, dy) in DIRS.iter() {
                if w.materials.get(w.get(hx + dx, hy + dy).material).food_energy > 0.0 && w.get(hx + dx, hy + dy).organism_id() == 0 {
                    w.set(hx + dx, hy + dy, Cell::EMPTY);
                }
            }
            // Food in every open cell of the ring, so the ant's own move on
            // this tick cannot leave it out of reach.
            if beside {
                for &(dx, dy) in DIRS.iter() {
                    if w.get(hx + dx, hy + dy).material == crate::sim::material::EMPTY {
                        w.set(hx + dx, hy + dy, Cell::new(food, 0));
                    }
                }
            }
            let st = w.organism_mut(id).expect("ant");
            st.spoil = holding.then(|| organism::Spoil { cell: Cell::new(soil, 0), store: false });
            st.energy = 0.0;
            st.crop = None;
            let _ = tick(&mut w, hx, hy, id, &def);
            assert!(w.organism(id).is_none(), "ant {id} (soil {holding}, food {beside}) lived on no energy");
        }
        let k = w.minds.as_ref().expect("minds").counts;
        assert_eq!((k.starved_soil, k.starved_soil_food), (2, 1));
    }

    /// **A mind knows how deep it is** ([`depth_at`]): rows below the ground
    /// as the bed was built, in its own column, the founding shaft's column
    /// included -- the datum is frozen before the cut, so the chamber reads
    /// deep and the rim reads 0. Until PR 2 both reads took `last_out_row`,
    /// which nothing on a mind's path writes, and read 0 everywhere.
    #[test]
    fn a_mind_reads_its_depth_below_the_ground_as_built() {
        let (w, def, ids) = mind_box();
        let site = w
            .nest_sites
            .iter()
            .find_map(|s| s.shaft)
            .expect("the box has a founding cut");
        let x = site.x0;
        let top = w.room_surface_at(x).expect("a datum at the shaft");
        assert!(
            top < site.bottom,
            "the shaft's datum {top} is its surface, not its floor {}",
            site.bottom
        );
        assert_eq!(depth_at(&w, x, top), 0.0, "the rim");
        assert_eq!(depth_at(&w, x, top + 16), 0.5, "16 rows down");
        assert_eq!(depth_at(&w, x, top - 16), -0.5, "16 rows up");
        assert!(
            depth_at(&w, site.chamber_x0, site.chamber_bottom) > 0.1,
            "the chamber floor reads deep"
        );
        // Every ant's context reads it at its head, and some ant is off
        // the datum's row (standing on the ground is a row above it).
        let mut off_row = 0;
        for &id in &ids {
            let (hx, hy) = head_of(&w, id);
            let (ctx, _) = context(&w, id, &def, &MindState::default(), &[]);
            assert_eq!(ctx[c::DEPTH], depth_at(&w, hx, hy), "ant {id} at ({hx},{hy})");
            off_row += usize::from(ctx[c::DEPTH] != 0.0);
        }
        assert!(off_row > 0, "every ant read depth 0");
    }

    /// **Walled in with a pellet, a mind can pack its way out**
    /// ([`options::pack_on`]): an ant sealed in soil holding a pellet is
    /// offered cuts (and only then: the same ant with open ground beside it
    /// is offered none), and acting one moves the head into the cut, puts the
    /// old pellet in the cell the tail left, and counts a pack. Delete the
    /// options' `pack` and the first assert goes red; delete `dig`'s pack
    /// branch and the act fails.
    #[test]
    fn a_walled_in_mind_packs_its_pellet_behind() {
        let (mut w, def, ids) = mind_box();
        let soil = w.materials.id_of("soil").expect("soil");
        let load = organism::Spoil { cell: Cell::new(soil, 0), store: false };
        let mut opts = Vec::new();
        // An ant with open ground ahead, holding a pellet: no cut on offer.
        let id = ids[0];
        w.organism_mut(id).expect("live").spoil = Some(load);
        options::build(&w, id, &def, &mut opts);
        let open_digs = opts.iter().filter(|o| o.verb == Verb::Dig).count();
        let walled = super::super::walled_in(&w, id, head_of(&w, id));
        assert!(walled || open_digs == 0, "a pellet carrier with room to move was offered {open_digs} cuts");
        // Seal it in: every cell within 3 of the head that is not a body becomes soil.
        let chain = w.organism(id).expect("live").chain.clone();
        let (hx, hy) = chain[0];
        for x in hx - 3..=hx + 3 {
            for y in hy - 3..=hy + 3 {
                if !chain.contains(&(x, y)) && w.get(x, y).organism_id() == 0 {
                    w.set(x, y, Cell::new(soil, 0));
                }
            }
        }
        assert!(super::super::walled_in(&w, id, (hx, hy)), "test setup: the ant is not walled in (a nestmate beside it?)");
        options::build(&w, id, &def, &mut opts);
        let digs: Vec<Opt> = opts.iter().copied().filter(|o| o.verb == Verb::Dig).collect();
        assert!(!digs.is_empty(), "walled in with a pellet, no cut was offered: {:?}", opts.iter().map(|o| (o.verb, o.target)).collect::<Vec<_>>());
        assert!(opts.iter().all(|o| o.verb != Verb::Drop), "test setup: a put-down is open, so this is not the trap");
        let up = *digs.iter().min_by_key(|o| o.target.1).expect("a cut");
        let tail = *chain.last().expect("a tail");
        let mut draw = rng::stream(1, id as u64, w.frame, RNG_SLOT_ACT);
        assert!(act(&mut w, id, &def, up, &mut draw), "the pack move failed");
        let st = w.organism(id).expect("alive");
        assert_eq!(st.chain[0], up.target, "the head did not step into its cut");
        assert!(st.spoil.is_some(), "the cut's own pellet is not in the jaws");
        assert_eq!(w.get(tail.0, tail.1).material, soil, "the old pellet is not where the tail was");
        assert_eq!(w.minds.as_ref().expect("minds").counts.packed, 1, "the pack was not counted");
    }

    fn head_of(w: &World, id: OrganismId) -> (i32, i32) {
        w.organism(id)
            .and_then(|s| s.chain.first().copied())
            .expect("a live ant")
    }

    fn on(verb: Verb, target: (i32, i32)) -> Opt {
        Opt {
            verb,
            target,
            feats: [0.0; FEATURES],
        }
    }

    /// **The food verbs move food by the world's rules, one job each**: a
    /// pick-up takes the cell into the crop and counts a pick-up, not an
    /// eat; Eat does not take food where it lies, and eats from the crop
    /// only aimed at the head, counted as eaten by choice; a put-down puts
    /// it back as a cell and counts a delivery exactly when the head is at
    /// the nest (`nest_within_reach`); an empty crop has nothing to put
    /// down, and soil, a dead ant, or food with soil in the jaws is not
    /// picked up. A richer ant feeds a poorer one a quarter of the gap
    /// (`SHARE_FRACTION`), and the poorer cannot feed back uphill.
    #[test]
    fn the_food_verbs_move_food_by_the_worlds_rules() {
        let (mut w, def, ids) = mind_box();
        let food = w.materials.id_of("provisions").expect("provisions");
        let (id, head) = ids
            .iter()
            .map(|&id| (id, head_of(&w, id)))
            .find(|&(_, (x, y))| w.get(x + 1, y).material == crate::sim::material::EMPTY)
            .expect("an ant with open ground ahead");
        let t = (head.0 + 1, head.1);
        let mut draw = rng::stream(1, id as u64, 0, RNG_SLOT_ACT);
        let (pickups, eats) = (w.creature_stats.pickups, w.creature_stats.eats);
        w.set(t.0, t.1, Cell::new(food, 0));
        assert!(!act(&mut w, id, &def, on(Verb::Eat, t), &mut draw), "eaten where it lay");
        assert_eq!(w.get(t.0, t.1).material, food);
        assert!(act(&mut w, id, &def, on(Verb::PickUp, t), &mut draw), "the pick-up");
        assert_ne!(w.get(t.0, t.1).material, food, "the cell is still there");
        assert!(w
            .organism(id)
            .and_then(|s| s.crop)
            .is_some_and(|c| c.worth() > 0.0 && c.cells == 1));
        assert_eq!(w.creature_stats.pickups, pickups + 1);
        assert_eq!(w.creature_stats.eats, eats, "a pick-up counted as an eat");
        let at_nest = nest_within_reach(&w, id, head.0, head.1, &def);
        let deliveries = w.creature_stats.deliveries;
        assert!(act(&mut w, id, &def, on(Verb::Drop, t), &mut draw), "the put-down");
        assert_eq!(w.get(t.0, t.1).material, food, "the cell did not go back down");
        assert!(w.organism(id).is_some_and(|s| s.crop.is_none()));
        assert_eq!(w.creature_stats.deliveries, deliveries + u64::from(at_nest));
        let k = w.minds.as_ref().expect("minds").counts;
        assert_eq!((k.swallowed, k.put_down, k.delivered), (1, 1, u64::from(at_nest)));
        // Nothing left to put down; soil is not food.
        w.set(t.0, t.1, Cell::EMPTY);
        assert!(
            !act(&mut w, id, &def, on(Verb::Drop, t), &mut draw),
            "an empty crop put something down"
        );
        let soil = w.materials.id_of("soil").expect("soil");
        w.set(t.0, t.1, Cell::new(soil, 0));
        assert!(!act(&mut w, id, &def, on(Verb::PickUp, t), &mut draw), "soil was picked up");
        let corpse = w.materials.id_of("corpse").expect("corpse");
        w.set(t.0, t.1, Cell::new(corpse, 0));
        assert!(!act(&mut w, id, &def, on(Verb::PickUp, t), &mut draw), "a dead ant was picked up");
        // Soil in the jaws: no food goes into the crop.
        w.set(t.0, t.1, Cell::new(food, 0));
        w.organism_mut(id).expect("ant").spoil = Some(organism::Spoil { cell: Cell::new(soil, 0), store: false });
        assert!(!act(&mut w, id, &def, on(Verb::PickUp, t), &mut draw), "food picked up past a pellet");
        w.organism_mut(id).expect("ant").spoil = None;
        // Eat is the crop's own, aimed at the head, and counts as eaten by
        // choice.
        assert!(act(&mut w, id, &def, on(Verb::PickUp, t), &mut draw), "the second pick-up");
        assert!(act(&mut w, id, &def, on(Verb::Eat, head), &mut draw), "eat from the crop");
        assert!(w.organism(id).is_some_and(|s| s.eat_lunch_now));
        assert_eq!(w.minds.as_ref().expect("minds").counts.ate, 1);

        // Two ants touching: a's head beside a cell of b's body.
        let (a, b, at) = ids
            .iter()
            .flat_map(|&a| ids.iter().map(move |&b| (a, b)))
            .filter(|(a, b)| a != b)
            .find_map(|(a, b)| {
                let h = head_of(&w, a);
                let body = w.organism(b)?.chain.clone();
                body.into_iter()
                    .find(|&(x, y)| (x - h.0).abs() <= 1 && (y - h.1).abs() <= 1)
                    .map(|c| (a, b, c))
            })
            .expect("two ants touching in the nest");
        w.organism_mut(a).expect("a").energy = 300.0;
        w.organism_mut(b).expect("b").energy = 100.0;
        assert!(act(&mut w, a, &def, on(Verb::Feed, at), &mut draw), "the feed");
        let (ea, eb) = (w.organism(a).expect("a").energy, w.organism(b).expect("b").energy);
        assert!(
            (ea - (300.0 - SHARE_FRACTION * 200.0)).abs() < 1e-3
                && (eb - (100.0 + SHARE_FRACTION * 200.0)).abs() < 1e-3,
            "{ea} {eb}"
        );
        let a_at = w.organism(a).expect("a").chain.iter().copied().find(|&(x, y)| {
            let h = head_of(&w, b);
            (x - h.0).abs() <= 1 && (y - h.1).abs() <= 1
        });
        if let Some(cell) = a_at {
            assert!(!act(&mut w, b, &def, on(Verb::Feed, cell), &mut draw), "fed uphill");
        }
        assert_eq!(w.minds.as_ref().expect("minds").counts.fed_adult, 1);
    }

    /// **The soil verbs move soil by the world's rules**: a cut takes the
    /// cell into the jaws as a pellet and empties it; full jaws cannot cut
    /// again; the pellet goes down only where it will stand
    /// (`spoil_site_open`), as the cell it was cut as, and the put-down is
    /// logged when a reader asked for it. A crop of food is not soil: with
    /// the jaws empty a put-down is the food drop's, untouched.
    #[test]
    fn the_soil_verbs_move_soil_by_the_worlds_rules() {
        let (mut w, def, ids) = mind_box();
        w.minds.as_mut().expect("minds").dump_log = Some(Vec::new());
        let (id, cut) = ids
            .iter()
            .find_map(|&id| {
                let (x, y) = head_of(&w, id);
                DIRS.iter()
                    .map(|&(dx, dy)| (x + dx, y + dy))
                    .find(|&(tx, ty)| jaw_can_cut(&w, &def, id, w.get(tx, ty)))
                    .map(|t| (id, t))
            })
            .expect("an ant beside ground it can cut");
        let ground = w.get(cut.0, cut.1);
        assert_ne!(ground.material, crate::sim::material::EMPTY);
        let (digs, dumped) = (w.creature_stats.digs, w.creature_stats.spoil_dumped);
        let mut draw = rng::stream(1, id as u64, 0, RNG_SLOT_ACT);
        assert!(act(&mut w, id, &def, on(Verb::Dig, cut), &mut draw), "the cut");
        assert!(w.is_empty(cut.0, cut.1), "the cut cell is still ground");
        let pellet = w.organism(id).and_then(|s| s.spoil).expect("a pellet in the jaws");
        assert_eq!(w.creature_stats.digs, digs + 1);
        // Full jaws cannot cut: any other cuttable cell is refused.
        let near = |w: &World| {
            let (x, y) = head_of(w, id);
            (-3..=3)
                .flat_map(|dy| (-3..=3).map(move |dx| (x + dx, y + dy)))
                .find(|&(tx, ty)| jaw_can_cut(w, &def, id, w.get(tx, ty)))
        };
        let t = near(&w).expect("another cell the jaws could cut, to refuse");
        assert!(!act(&mut w, id, &def, on(Verb::Dig, t), &mut draw), "cut with full jaws");
        // Not where it would hang: a cell of open air with nothing under it.
        let (hx, hy) = head_of(&w, id);
        let air = (0..hy)
            .rev()
            .map(|y| (hx, y))
            .find(|&(x, y)| !spoil_site_open(&w, x, y, true) && w.is_empty(x, y))
            .expect("open air above the ant");
        assert!(!act(&mut w, id, &def, on(Verb::Drop, air), &mut draw), "a pellet hung in the air");
        assert!(w.organism(id).is_some_and(|s| s.spoil.is_some()), "a refused put-down let go");
        // Where it will stand: the cut cell's own hole, if it has a footing,
        // or the nearest site that does.
        let site = (-6..=6)
            .flat_map(|dy| (-6..=6).map(move |dx| (hx + dx, hy + dy)))
            .find(|&(x, y)| spoil_site_open(&w, x, y, true))
            .expect("a site a pellet can stand on");
        assert!(act(&mut w, id, &def, on(Verb::Drop, site), &mut draw), "the pellet put-down");
        assert_eq!(w.get(site.0, site.1), pellet.cell, "the pellet went down as something else");
        assert!(w.organism(id).is_some_and(|s| s.spoil.is_none() && s.spoil_ring.is_none()));
        assert_eq!(w.creature_stats.spoil_dumped, dumped + 1);
        let m = w.minds.as_ref().expect("minds");
        assert_eq!((m.counts.dug, m.counts.dumped, m.counts.put_down), (1, 1, 0));
        assert_eq!(m.dump_log.as_deref(), Some(&[(site.0, site.1, w.frame, id)][..]));

        // **Ground only, and a crop of food does not stop a cut** (fix plan
        // step 1): food and a dead ant are not cut, though the jaw could
        // take the food (the positive control: it would have before); with
        // food picked up, the jaws still cut ground.
        let food = w.materials.id_of("provisions").expect("provisions");
        let corpse = w.materials.id_of("corpse").expect("corpse");
        let open = DIRS
            .iter()
            .map(|&(dx, dy)| (hx + dx, hy + dy))
            .find(|&(x, y)| w.is_empty(x, y))
            .expect("an open cell beside the head");
        w.set(open.0, open.1, Cell::new(food, 0));
        assert!(jaw_can_cut(&w, &def, id, w.get(open.0, open.1)), "the jaw can take food: the control");
        for m in [food, corpse] {
            w.set(open.0, open.1, Cell::new(m, 0));
            assert!(!act(&mut w, id, &def, on(Verb::Dig, open), &mut draw), "{} cut into the jaws", w.materials.get(m).name);
        }
        w.set(open.0, open.1, Cell::new(food, 0));
        assert!(act(&mut w, id, &def, on(Verb::PickUp, open), &mut draw), "the pick-up");
        assert!(w.organism(id).is_some_and(|s| s.crop.is_some()));
        let ground = (-3..=3)
            .flat_map(|dy| (-3..=3).map(move |dx| (hx + dx, hy + dy)))
            .find(|&(x, y)| options::diggable(&w, &def, id, w.get(x, y)))
            .expect("ground to cut");
        assert!(act(&mut w, id, &def, on(Verb::Dig, ground), &mut draw), "a crop of food stopped the cut");
    }

    /// **A rung's entry**: the verbs the earlier rung banned come on, an
    /// evolved-off switch stays off, Pick up and Eat come on blank whatever
    /// drifted while they were banned (fix plan step 1; review of PR 724,
    /// N7), and scent B gets the
    /// carrying layer only where the arena allows it and the genome never
    /// laid it.
    #[test]
    fn a_rung_switches_on_its_new_verbs_and_the_carrying_scent() {
        let trail = [Verb::Walk, Verb::TurnAround, Verb::Rest];
        let forage = [
            Verb::Walk,
            Verb::TurnAround,
            Verb::Rest,
            Verb::PickUp,
            Verb::Eat,
            Verb::Drop,
            Verb::Feed,
        ];
        let mut g = blank(&trail);
        g[SWITCH + Verb::TurnAround as usize] = -0.4; // evolved off
        let pick = Verb::PickUp as usize;
        let drift = [
            JOB + pick * J_IN + c::ENERGY,
            JOB + pick * J_IN + CONTEXT + MEMORY + 1,
            S_VERB + 3 * VERBS + pick,
            MEM + 4 * M_IN + CONTEXT + 2 + pick,
            HID_OUT + pick * HIDDEN + 5,
        ];
        let eat = Verb::Eat as usize;
        let eat_drift = [JOB + eat * J_IN + c::CROP, HID_OUT + eat * HIDDEN + 2];
        for &i in drift.iter().chain(&eat_drift) {
            g[i] = 0.7;
        }
        let did = enter_rung(&mut g, &forage, [true, true, false]);
        assert_eq!(did, vec!["pick_up", "5 drifted pick_up links zeroed", "drop", "eat", "2 drifted eat links zeroed", "feed", "scent B while carrying"]);
        assert!(drift.iter().chain(&eat_drift).all(|&i| g[i] == 0.0));
        assert!(g[SWITCH + Verb::Eat as usize] > 0.0 && g[SWITCH + Verb::TurnAround as usize] == -0.4);
        assert_eq!(
            g[SWITCH + Verb::Dig as usize],
            -1.0,
            "a verb this rung does not offer came on"
        );
        // Lays B with food in the crop, nothing without.
        let lay = |crop: f32| {
            let row = &g[EMIT + E_IN..EMIT + 2 * E_IN];
            squash(row[c::CROP] * crop + row[E_IN - 1]).clamp(0.0, 1.0)
        };
        assert!(
            lay(0.0) == 0.0 && lay(1.0 / 6.0) > 0.7,
            "{} {}",
            lay(0.0),
            lay(1.0 / 6.0)
        );
        // Entered once, nothing more to do; and no layer where B is banned.
        assert!(enter_rung(&mut g, &forage, [true, true, false]).is_empty());
        let mut h = blank(&trail);
        assert_eq!(enter_rung(&mut h, &trail, [false, false, false]), Vec::<&str>::new());
    }

    /// **A training start reopens what an earlier rung switched off,
    /// quietly** ([`reopen`]): an evolved-off switch and a trimmed one come
    /// on with the job's constant at [`QUIET`], a lower constant stays, a
    /// banned verb and one the rung does not offer are left alone, and a
    /// second call has nothing to do.
    #[test]
    fn a_training_start_reopens_switched_off_verbs_quietly() {
        let forage = [
            Verb::Walk,
            Verb::TurnAround,
            Verb::Rest,
            Verb::Eat,
            Verb::Drop,
            Verb::Feed,
        ];
        let mut g = blank(&forage);
        let k = |v: Verb| JOB + v as usize * J_IN + CONTEXT + MEMORY + 1;
        g[SWITCH + Verb::TurnAround as usize] = -0.4; // evolved off
        g[SWITCH + Verb::Rest as usize] = 0.0; // trimmed
        g[k(Verb::Rest)] = -20.0;
        g[SWITCH + Verb::Drop as usize] = -1.0; // banned: enter_rung's
        g[SWITCH + Verb::Dig as usize] = -0.3; // not offered here
        assert_eq!(reopen(&mut g, &forage), vec!["turn_around", "rest"]);
        assert_eq!(g[SWITCH + Verb::TurnAround as usize], 1.0);
        assert_eq!(g[k(Verb::TurnAround)], QUIET);
        assert_eq!((g[SWITCH + Verb::Rest as usize], g[k(Verb::Rest)]), (1.0, -20.0));
        assert_eq!(g[SWITCH + Verb::Drop as usize], -1.0);
        assert_eq!(g[SWITCH + Verb::Dig as usize], -0.3);
        assert!(reopen(&mut g, &forage).is_empty());
    }

    #[test]
    fn the_layout_adds_up_and_crosses_in_whole_rows() {
        assert_eq!(GENOME_LEN, EMIT + EMITS * E_IN + HIDDEN * H_IN + VERBS * HIDDEN);
        assert_eq!(
            (GENOME_LEN_V1, GENOME_LEN_V2, GENOME_LEN),
            (2333, 2669, 2797),
            "saved genomes are 2,333 slots before the hidden units, 2,669 before PR 2's features"
        );
        let mut groups: Vec<usize> = (0..GENOME_LEN).map(crossover_group).collect();
        groups.dedup();
        let n = groups.iter().collect::<std::collections::BTreeSet<_>>().len();
        // switches, sharp, pull; units; output bias; jobs; memory rows; emits; job hidden units
        assert_eq!(n, VERBS + 2 + UNITS + 1 + VERBS + MEMORY + EMITS + HIDDEN, "crossover groups");
        // A job hidden unit's fan-in and its weights into the jobs travel together.
        assert_eq!(crossover_group(HID + 2 * H_IN + 4), crossover_group(HID_OUT + 5 * HIDDEN + 2));
        assert_ne!(crossover_group(HID + 2 * H_IN), crossover_group(HID_OUT + 5 * HIDDEN + 3));
        // Memory unit m's rate travels with its row.
        assert_eq!(crossover_group(RATE + 3), crossover_group(MEM + 3 * M_IN + 5));
        // A scorer unit's fan-in and its output weight travel together.
        assert_eq!(
            crossover_group(S_CTX + 2 * S_IN),
            crossover_group(S_FEAT + 2 * FEATURES + 1)
        );
        assert_eq!(crossover_group(S_VERB + 2 * VERBS), crossover_group(S_OUT + 2));
        let t = trainable(&[Verb::Walk, Verb::Rest], [false, true, false]);
        assert!(t.contains(&(SWITCH + Verb::Walk as usize)) && !t.contains(&(SWITCH + Verb::Dig as usize)));
        assert!(t.contains(&(EMIT + E_IN)) && !t.contains(&EMIT) && !t.contains(&(EMIT + 2 * E_IN)));
        assert!(t.contains(&HID) && t.contains(&(GENOME_LEN - 1)), "the job hidden units are not an emit row");
        assert_eq!(
            MindGenome::new(blank(&Verb::ALL)).live,
            0,
            "a blank mind costs nothing to run"
        );
    }
}


/// SCRATCH (box-rank / investigation): `PP_MIND_LOG=<file>` writes one row per mind decision
/// and the soil arena's cut and put-down events, for tracing individuals offline.
///
/// **Every `PP_*` switch here is off unless its variable is set**, and each is
/// read once (`OnceLock`), so a run with none set takes the plain path: the
/// teacher lane's harness (`examples/inrun.rs`, `examples/copylog.rs`) landed
/// on `main` 2026-10-22 with these hooks re-threaded into the restructured
/// [`tick`] (`context_at`, `decide_planned`), and with none set an hA1 batch
/// came back byte-identical to the compute branch it came from.
pub mod scratch_log {
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Mutex, OnceLock};

    // ---- SCRATCH (teacher-training ceiling test): a hand-written teacher that makes three of the brain's calls.
    // `PP_TEACH=dig,drop,home[,margin=N]` turns each correction on. The teacher reads only what the copyability probe
    // showed the ant can sense (frame-0 ground, depth, the hole as it stands), held per thread by the soil arena for
    // the run's frames and cleared before the settle, so it never acts while the score is being taken.
    //   dig   empty jaws: dig the deepest foot cell when one is in reach (force); a dig with none in reach is vetoed
    //   drop  carrying spoil: put it down on a cell clear of the hole when one is offered (force); a put-down inside
    //         the hole's zone is vetoed. `margin=N` widens "clear" by N cells (a pellet at exactly 3 may slide back)
    //   home  empty, on the surface, 3+ columns from the mouth, walking: take the step nearest the mouth's column
    //   down  empty, below ground, no foot cell in reach, walking: take the deepest step, if one goes deeper. Added
    //         after the first ceiling run: with wall cuts vetoed and nothing in their place, the ant walked up and down
    //         the shaft for 6,000 frames and never reached the foot (s161 had only ever got down by digging its way)
    // A veto removes the verb and lets the brain redraw from what is left, so the teacher never invents behaviour
    // where it abstains. Shut in (no Walk option offered), the teacher abstains on dig and drop: digging out is legal.
    pub struct TeachCtx {
        pub x0: i32,
        pub y0: i32,
        pub nw: usize,
        pub nh: usize,
        pub ground0: Vec<bool>,
        pub datum: Vec<Option<i32>>,
        pub founding: Vec<(i32, i32)>,
        pub shaft: i32,
        pub mouth_x: f32,
        /// Chebyshev radius a put-down must keep from every hole cell (SOIL_AWAY - 1, plus the margin).
        pub clear: i32,
    }
    thread_local! {
        pub static TEACH: std::cell::RefCell<Option<TeachCtx>> = const { std::cell::RefCell::new(None) };
    }
    /// 0 decisions under a teacher, 1 dig forced, 2 dig vetoed, 3 drop forced, 4 drop vetoed, 5 walk-home forced,
    /// 6 shut-in abstains, 7 decisions with the teacher on but no facts on this thread (should be the settle only),
    /// 8 go-down forced, 9 calls skipped by compliance (q or floor), 10 positive-control failures of `remember`
    /// (re-running the brain's own verb must give the memory `decide` wrote).
    pub static COUNTS: [AtomicU64; 11] = [const { AtomicU64::new(0) }; 11];
    pub fn down_on() -> bool {
        static D: OnceLock<bool> = OnceLock::new();
        *D.get_or_init(|| std::env::var("PP_TEACH").unwrap_or_default().split(',').any(|p| p == "down"))
    }
    pub fn arms() -> (bool, bool, bool, i32) {
        static A: OnceLock<(bool, bool, bool, i32)> = OnceLock::new();
        *A.get_or_init(|| {
            let s = std::env::var("PP_TEACH").unwrap_or_default();
            let parts: Vec<&str> = s.split(',').filter(|p| !p.is_empty()).collect();
            let margin = parts.iter().find_map(|p| p.strip_prefix("margin=")).map_or(0, |m| m.parse().expect("PP_TEACH margin"));
            for p in &parts {
                assert!(matches!(*p, "dig" | "drop" | "home" | "down") || p.starts_with("margin="), "PP_TEACH: unknown part {p}");
            }
            (parts.contains(&"dig"), parts.contains(&"drop"), parts.contains(&"home"), margin)
        })
    }
    /// The brain's own memory update (`decide`'s "Memory" block), re-run for the verb actually carried out, with
    /// all_best / all_mean over the brain's FULL option list. Review fix (2026-10-21): the first ceiling test left the
    /// memory a forced dig wrote as if the brain had done its own pick, and a veto's redraw summed the options without
    /// the vetoed verb's, so the ceiling's ant remembered something other than what it did.
    pub fn remember(g: &[f32], ctx: &[f32; CONTEXT], mem0: &[f32; MEMORY], opts: &[Opt], scores: &[f32], verb: Verb) -> [f32; MEMORY] {
        let on = |v: Verb| v.acts() && g[SWITCH + v as usize] > 0.0;
        let (mut best, mut sum, mut n) = (f32::NEG_INFINITY, 0.0f32, 0u32);
        for (o, op) in opts.iter().enumerate() {
            if on(op.verb) {
                best = best.max(scores[o]);
                sum += scores[o];
                n += 1;
            }
        }
        let all_mean = if n > 0 { sum / n as f32 } else { 0.0 };
        let mut fresh = [0.0f32; MEMORY];
        for (m, slot) in fresh.iter_mut().enumerate() {
            let row = &g[MEM + m * M_IN..MEM + (m + 1) * M_IN];
            let mut s = row[M_IN - 1] + row[CONTEXT] * best + row[CONTEXT + 1] * all_mean + row[CONTEXT + 2 + verb as usize];
            for (w, x) in row[..CONTEXT].iter().zip(ctx) {
                s += w * x;
            }
            for (w, x) in row[CONTEXT + 2 + VERBS..CONTEXT + 2 + VERBS + MEMORY].iter().zip(mem0) {
                s += w * x;
            }
            let r = rate(g, m);
            *slot = r * mem0[m] + (1.0 - r) * squash(s);
        }
        fresh
    }
    /// `PP_TEACH_Q=q`: each call is applied with probability q (the compliance curve). `PP_TEACH_FLOOR=1`: each call
    /// is applied only as often as a brain saturated towards it would obey under `choose_weighted`'s floor (labelled
    /// options at w = 1, the rest at w = 0; a veto leaves Dig at w = 0 against the other verbs' own weights) -- the
    /// most a copied brain could deliver (method review M1).
    fn q() -> f32 {
        static Q: OnceLock<f32> = OnceLock::new();
        *Q.get_or_init(|| std::env::var("PP_TEACH_Q").ok().map_or(1.0, |v| v.parse().expect("PP_TEACH_Q")))
    }
    fn floor_mode() -> bool {
        static F: OnceLock<bool> = OnceLock::new();
        *F.get_or_init(|| std::env::var("PP_TEACH_FLOOR").is_ok_and(|v| v == "1" || v == "2"))
    }
    /// `PP_TEACH_FLOOR=2`: the strict floor. Every decision a rule covers is drawn from the saturated policy, not only
    /// those where the brain's own draw was wrong -- so a copy that is right on average still digs a wall now and then
    /// with no foot cell in reach (method review M1's ~2.8 a world).
    fn strict_floor() -> bool {
        static F: OnceLock<bool> = OnceLock::new();
        *F.get_or_init(|| std::env::var("PP_TEACH_FLOOR").is_ok_and(|v| v == "2"))
    }
    fn roll(world: &World, organism: OrganismId, salt: u64) -> f32 {
        rng::stream(world.seed, organism as u64, world.frame, 4300 + salt).unit_f32()
    }
    /// `PP_TEACH_SAT=s` (default 1): how far a copied brain's scores reach -- labelled options at score +s, the rest
    /// at -s (w = (1 + s) / 2 and (1 - s) / 2). 1 is full saturation; 0.9 is what squash reaches at |x| = 9.
    fn sat() -> f32 {
        static S: OnceLock<f32> = OnceLock::new();
        *S.get_or_init(|| std::env::var("PP_TEACH_SAT").ok().map_or(1.0, |v| v.parse().expect("PP_TEACH_SAT")))
    }
    fn lo_w() -> f32 {
        (PICK_FLOOR + (1.0 - sat()) / 2.0).powi(2)
    }
    /// Obedience among `labelled` options against `others`, at saturation `sat()`.
    fn obey(labelled: usize, others: usize) -> f32 {
        let hi = (PICK_FLOOR + (1.0 + sat()) / 2.0).powi(2);
        labelled as f32 * hi / (labelled as f32 * hi + others as f32 * lo_w())
    }
    fn applies(world: &World, organism: OrganismId, salt: u64, obedience: f32) -> bool {
        let p = q() * if floor_mode() { obedience } else { 1.0 };
        if p >= 1.0 {
            return true;
        }
        let ok = rng::stream(world.seed, organism as u64, world.frame, 4200 + salt).unit_f32() < p;
        if !ok {
            COUNTS[9].fetch_add(1, Ordering::Relaxed);
        }
        ok
    }
    pub fn teach_on() -> bool {
        let a = arms();
        a.0 || a.1 || a.2 || down_on()
    }
    impl TeachCtx {
        fn ground0(&self, x: i32, y: i32) -> bool {
            let (i, j) = (x - self.x0, y - self.y0);
            i >= 0 && j >= 0 && (i as usize) < self.nw && (j as usize) < self.nh && self.ground0[j as usize * self.nw + i as usize]
        }
        /// As the arena's `depth_below_ground`: rows below the lowest datum within 3 columns.
        fn depth(&self, x: i32, y: i32) -> i32 {
            (x - 3..=x + 3)
                .filter_map(|c| usize::try_from(c - self.x0).ok().and_then(|i| self.datum.get(i).copied().flatten()))
                .max()
                .map_or(0, |low| y - low)
        }
    }
    pub enum Teach {
        Keep,
        Force(usize),
        Veto(Verb),
    }
    fn shut_in(opts: &[Opt]) -> bool {
        !opts.iter().any(|o| o.verb == Verb::Walk)
    }
    /// The dig and drop corrections, on the brain's pick.
    pub fn teach(world: &World, organism: OrganismId, opts: &[Opt], pick: usize, jobs: &[Option<f32>; VERBS]) -> Teach {
        let (dig, drop, _, _) = arms();
        TEACH.with(|t| {
            let t = t.borrow();
            let Some(t) = t.as_ref() else {
                COUNTS[7].fetch_add(1, Ordering::Relaxed);
                return Teach::Keep;
            };
            COUNTS[0].fetch_add(1, Ordering::Relaxed);
            let cuts: &[(i32, i32, u64, OrganismId)] = world.cut_log.as_deref().unwrap_or(&[]);
            let carrying = world.organism(organism).is_some_and(|s| s.spoil.is_some());
            let chosen = opts[pick];
            if (dig || drop) && shut_in(opts) {
                COUNTS[6].fetch_add(1, Ordering::Relaxed);
                return Teach::Keep;
            }
            if dig && !carrying {
                let foot = |o: &Opt| {
                    let (x, y) = o.target;
                    t.ground0(x, y) && !cuts.iter().any(|c| (c.0, c.1) == (x, y)) && t.depth(x, y) >= t.shaft
                };
                let best = opts
                    .iter()
                    .enumerate()
                    .filter(|(_, o)| o.verb == Verb::Dig && foot(o))
                    .max_by(|a, b| {
                        (t.depth(a.1.target.0, a.1.target.1), -(a.1.target.0 as f32 - t.mouth_x).abs())
                            .partial_cmp(&(t.depth(b.1.target.0, b.1.target.1), -(b.1.target.0 as f32 - t.mouth_x).abs()))
                            .unwrap()
                    })
                    .map(|(i, _)| i);
                let others = (0..VERBS).filter(|&v| v != Verb::Dig as usize && jobs[v].is_some()).count();
                let n_foot = opts.iter().filter(|o| o.verb == Verb::Dig && foot(o)).count();
                let n_dig = opts.iter().filter(|o| o.verb == Verb::Dig).count();
                if strict_floor() {
                    let lo = lo_w();
                    let rest: f32 = (0..VERBS)
                        .filter(|&v| v != Verb::Dig as usize)
                        .filter_map(|v| jobs[v].map(|j| (PICK_FLOOR + (j + 1.0) / 2.0).powi(2)))
                        .sum();
                    let u = roll(world, organism, 1);
                    match best {
                        // A foot cell in reach: obey with the saturated probability, else leave the brain's pick.
                        Some(i) => {
                            if u < obey(1, others) * obey(n_foot, n_dig - n_foot) {
                                if !(chosen.verb == Verb::Dig && foot(&chosen)) {
                                    COUNTS[1].fetch_add(1, Ordering::Relaxed);
                                    return Teach::Force(i);
                                }
                            } else {
                                COUNTS[9].fetch_add(1, Ordering::Relaxed);
                                if chosen.verb == Verb::Dig && foot(&chosen) {
                                    return Teach::Veto(Verb::Dig);
                                }
                            }
                        }
                        // None in reach: a saturated copy still digs with probability lo / (lo + rest) -- at the
                        // brain's best-scored Dig option -- and otherwise does not dig.
                        None if n_dig > 0 => {
                            if u < lo / (lo + rest) {
                                COUNTS[9].fetch_add(1, Ordering::Relaxed);
                                if chosen.verb != Verb::Dig {
                                    let leak = opts.iter().enumerate().filter(|(_, o)| o.verb == Verb::Dig).map(|(i, _)| i).next();
                                    if let Some(i) = leak {
                                        return Teach::Force(i);
                                    }
                                }
                            } else if chosen.verb == Verb::Dig {
                                COUNTS[2].fetch_add(1, Ordering::Relaxed);
                                return Teach::Veto(Verb::Dig);
                            }
                        }
                        None => {}
                    }
                    return Teach::Keep;
                }
                match best {
                    Some(i) if !(chosen.verb == Verb::Dig && foot(&chosen)) => {
                        if applies(world, organism, 1, obey(1, others) * obey(n_foot, n_dig - n_foot)) {
                            COUNTS[1].fetch_add(1, Ordering::Relaxed);
                            return Teach::Force(i);
                        }
                    }
                    None if chosen.verb == Verb::Dig => {
                        let lo = lo_w();
                        let rest: f32 = (0..VERBS)
                            .filter(|&v| v != Verb::Dig as usize)
                            .filter_map(|v| jobs[v].map(|j| (PICK_FLOOR + (j + 1.0) / 2.0).powi(2)))
                            .sum();
                        if applies(world, organism, 2, 1.0 - lo / (lo + rest)) {
                            COUNTS[2].fetch_add(1, Ordering::Relaxed);
                            return Teach::Veto(Verb::Dig);
                        }
                    }
                    _ => {}
                }
            }
            if drop && carrying {
                let mut hole: std::collections::HashSet<(i32, i32)> = t.founding.iter().copied().collect();
                hole.extend(cuts.iter().filter(|c| t.ground0(c.0, c.1)).map(|c| (c.0, c.1)));
                let r = t.clear;
                let gap = |(x, y): (i32, i32)| {
                    (-r - 3..=r + 3)
                        .flat_map(|dy| (-r - 3..=r + 3).map(move |dx| (dx, dy)))
                        .filter(|d| hole.contains(&(x + d.0, y + d.1)))
                        .map(|(dx, dy)| dx.abs().max(dy.abs()))
                        .min()
                        .unwrap_or(i32::MAX)
                };
                let clear = |o: &Opt| gap(o.target) > r;
                // "The first cell clear": of the clear put-downs, the one nearest the hole (the shortest carry).
                let best = opts
                    .iter()
                    .enumerate()
                    .filter(|(_, o)| o.verb == Verb::Drop && clear(o))
                    .min_by_key(|(_, o)| gap(o.target))
                    .map(|(i, _)| i);
                match best {
                    Some(i) if !(chosen.verb == Verb::Drop && clear(&chosen)) => {
                        COUNTS[3].fetch_add(1, Ordering::Relaxed);
                        return Teach::Force(i);
                    }
                    None if chosen.verb == Verb::Drop => {
                        COUNTS[4].fetch_add(1, Ordering::Relaxed);
                        return Teach::Veto(Verb::Drop);
                    }
                    _ => {}
                }
            }
            Teach::Keep
        })
    }
    /// The walk-home correction, on the final pick (after any veto's redraw).
    pub fn walk_home(world: &World, organism: OrganismId, at: (i32, i32), opts: &[Opt], pick: usize) -> Option<usize> {
        if !(arms().2 || down_on()) || opts[pick].verb != Verb::Walk {
            return None;
        }
        TEACH.with(|t| {
            let t = t.borrow();
            let t = t.as_ref()?;
            let carrying = world.organism(organism).is_some_and(|s| s.spoil.is_some());
            if carrying {
                return None;
            }
            if down_on() && t.depth(at.0, at.1) > 0 {
                let cuts: &[(i32, i32, u64, OrganismId)] = world.cut_log.as_deref().unwrap_or(&[]);
                let foot = |o: &Opt| {
                    let (x, y) = o.target;
                    o.verb == Verb::Dig && t.ground0(x, y) && !cuts.iter().any(|c| (c.0, c.1) == (x, y)) && t.depth(x, y) >= t.shaft
                };
                if opts.iter().any(foot) {
                    return None;
                }
                let here = t.depth(at.0, at.1);
                let walks: Vec<usize> = (0..opts.len()).filter(|&i| opts[i].verb == Verb::Walk).collect();
                let deepest = *walks.iter().max_by_key(|&&i| (t.depth(opts[i].target.0, opts[i].target.1), -(opts[i].target.0 - at.0).abs()))?;
                let dd = t.depth(opts[deepest].target.0, opts[deepest].target.1);
                if dd > here && dd > t.depth(opts[pick].target.0, opts[pick].target.1) {
                    let n_deep = walks.iter().filter(|&&i| t.depth(opts[i].target.0, opts[i].target.1) == dd).count();
                    if applies(world, organism, 3, obey(n_deep, walks.len() - n_deep)) {
                        COUNTS[7 + 1].fetch_add(1, Ordering::Relaxed);
                        return Some(deepest);
                    }
                }
                return None;
            }
            if !arms().2 || t.depth(at.0, at.1) > 0 || (at.0 as f32 - t.mouth_x).abs() < 3.0 {
                return None;
            }
            let walks: Vec<usize> = (0..opts.len()).filter(|&i| opts[i].verb == Verb::Walk).collect();
            if walks.len() < 2 {
                return None;
            }
            let d = |i: usize| (opts[i].target.0 as f32 - t.mouth_x).abs();
            let best = *walks.iter().min_by(|a, b| d(**a).partial_cmp(&d(**b)).unwrap())?;
            let n_near = walks.iter().filter(|&&i| d(i) == d(best)).count();
            if d(best) < d(pick) && applies(world, organism, 5, obey(n_near, walks.len() - n_near)) {
                COUNTS[5].fetch_add(1, Ordering::Relaxed);
                Some(best)
            } else {
                None
            }
        })
    }
    // ---- SCRATCH (teacher-training step 1, the copier's log): `PP_COPY_LOG=<file>` writes, per decision, everything a
    // copier needs to recompute the brain's choice for ANY genome -- every option of every verb at full precision
    // (Rust's shortest round-trip float form), the brain's own scores, its memory before the decision -- and, in the
    // soil arena with `PP_TEACH_LABEL` set, the teacher's state labels: pure functions of the state, written whatever
    // the brain picked (review finding 1). Every row starts with a world tag (`WORLD`, set by the example per job) so
    // worlds can run on several threads into one file; `SETTLE` marks the scored settle's rest-mind rows.
    thread_local! {
        pub static WORLD: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
        pub static SETTLE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    fn copy_out() -> Option<&'static Mutex<std::io::BufWriter<std::fs::File>>> {
        static F: OnceLock<Option<Mutex<std::io::BufWriter<std::fs::File>>>> = OnceLock::new();
        F.get_or_init(|| {
            std::env::var("PP_COPY_LOG")
                .ok()
                .map(|p| Mutex::new(std::io::BufWriter::with_capacity(1 << 20, std::fs::File::create(p).expect("copy log"))))
        })
        .as_ref()
    }
    /// SCRATCH (teacher lane, 2026-10-22): `PP_RIMVETO=N` with `PP_RIMVETO_X` / `PP_RIMVETO_R`, read once (the dig
    /// thread's first cut read the environment on every decision).
    pub fn rimveto() -> Option<(i32, i32, i32)> {
        static V: OnceLock<Option<(i32, i32, i32)>> = OnceLock::new();
        *V.get_or_init(|| {
            let n = std::env::var("PP_RIMVETO").ok()?.parse::<i32>().ok()?;
            let nx = std::env::var("PP_RIMVETO_X").ok().and_then(|v| v.parse().ok()).unwrap_or(128);
            let nr = std::env::var("PP_RIMVETO_R").ok().and_then(|v| v.parse().ok()).unwrap_or(20);
            Some((n, nx, nr))
        })
    }
    /// The refusal's test: a Dig of ground (not spoil) fewer than `n` rows below the frozen datum within `nr` columns
    /// of `nx`. One function for the acting veto and the copy log's label, so the two cannot drift apart.
    pub fn rim_cut(world: &World, o: &Opt, n: i32, nx: i32, nr: i32) -> bool {
        let (tx, ty) = o.target;
        o.verb == Verb::Dig
            && (tx - nx).abs() <= nr
            && world.room_surface_at(tx).is_some_and(|s| ty >= s && ty - s < n)
            && Some(world.get(tx, ty).material) != world.materials.id_of("spoil")
    }
    pub fn copy_on() -> bool {
        copy_out().is_some()
    }
    /// One row into the copy log, prefixed with this thread's world tag.
    pub fn cline(s: &str) {
        if let Some(f) = copy_out() {
            let tag = WORLD.with(|w| w.get());
            let _ = writeln!(f.lock().unwrap(), "{tag},{s}");
        }
    }
    pub fn copy_flush() {
        if let Some(f) = copy_out() {
            let _ = f.lock().unwrap().flush();
        }
    }
    pub fn label_on() -> bool {
        static L: OnceLock<bool> = OnceLock::new();
        *L.get_or_init(|| std::env::var("PP_TEACH_LABEL").is_ok())
    }
    /// The teacher's state labels for one decision, as option indices into `opts`. Each mirrors the acting rule's test
    /// in `teach` / `walk_home` exactly, but is computed whatever the brain picked:
    ///   dig   `M` must-dig (a foot cell in reach; `digset` = the acting rule's choice key's arg-max set), `N`
    ///         must-not-dig (Dig offered, no foot cell), `-` abstain (carrying, shut in, or no Dig offered)
    ///   down  the deepest steps (key: depth, then nearest column), when empty, below ground, no foot cell in reach,
    ///         and the deepest step is deeper than here
    ///   home  the steps nearest the mouth's column, when empty, on the surface, 3+ columns from the mouth, 2+ steps
    fn labels(world: &World, organism: OrganismId, at: (i32, i32), opts: &[Opt]) -> String {
        TEACH.with(|t| {
            let t = t.borrow();
            let Some(t) = t.as_ref() else { return "-|-|-|-".to_string() };
            let cuts: &[(i32, i32, u64, OrganismId)] = world.cut_log.as_deref().unwrap_or(&[]);
            let carrying = world.organism(organism).is_some_and(|s| s.spoil.is_some());
            let foot = |o: &Opt| {
                let (x, y) = o.target;
                o.verb == Verb::Dig && t.ground0(x, y) && !cuts.iter().any(|c| (c.0, c.1) == (x, y)) && t.depth(x, y) >= t.shaft
            };
            let join = |v: &[usize]| if v.is_empty() { "-".to_string() } else { v.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(";") };
            let any_foot = opts.iter().any(foot);
            let (mut dig, mut digset) = ("-", Vec::new());
            if !carrying && !shut_in(opts) && opts.iter().any(|o| o.verb == Verb::Dig) {
                if any_foot {
                    dig = "M";
                    let key = |o: &Opt| (t.depth(o.target.0, o.target.1), -(o.target.0 as f32 - t.mouth_x).abs());
                    let best = opts.iter().filter(|o| foot(o)).map(key).fold(None, |m: Option<(i32, f32)>, k| match m {
                        Some(b) if b >= k => Some(b),
                        _ => Some(k),
                    });
                    digset = (0..opts.len()).filter(|&i| foot(&opts[i]) && Some(key(&opts[i])) == best).collect();
                } else {
                    dig = "N";
                }
            }
            let walks: Vec<usize> = (0..opts.len()).filter(|&i| opts[i].verb == Verb::Walk).collect();
            let here = t.depth(at.0, at.1);
            let mut down = Vec::new();
            if !carrying && here > 0 && !any_foot && !walks.is_empty() {
                let key = |i: usize| (t.depth(opts[i].target.0, opts[i].target.1), -(opts[i].target.0 - at.0).abs());
                let best = walks.iter().map(|&i| key(i)).max().unwrap();
                if best.0 > here {
                    down = walks.iter().copied().filter(|&i| key(i) == best).collect();
                }
            }
            let mut home = Vec::new();
            if !carrying && here <= 0 && (at.0 as f32 - t.mouth_x).abs() >= 3.0 && walks.len() >= 2 {
                let d = |i: usize| (opts[i].target.0 as f32 - t.mouth_x).abs();
                let best = walks.iter().map(|&i| d(i)).fold(f32::INFINITY, f32::min);
                home = walks.iter().copied().filter(|&i| d(i) == best).collect();
            }
            format!("{dig}|{}|{}|{}", join(&digset), join(&down), join(&home))
        })
    }
    /// The copy log's decision row: `Q,frame,id,x,y,settle,pick,last_verb,carrying,labels,ctx,mem0,opts` -- opts as
    /// `verb:tx:ty:score:i=v i=v ...` (nonzero features), `;`-joined.
    #[allow(clippy::too_many_arguments)]
    pub fn copy_decision(world: &World, organism: OrganismId, at: (i32, i32), ctx: &[f32; CONTEXT], mem0: &[f32; MEMORY], last: Option<Verb>, opts: &[Opt], d: &Decision) {
        if !copy_on() {
            return;
        }
        let f = |v: &[f32]| v.iter().map(|x| format!("{x}")).collect::<Vec<_>>().join(" ");
        let carrying = world.organism(organism).is_some_and(|s| s.spoil.is_some());
        let lab = if label_on() { labels(world, organism, at, opts) } else { "-|-|-|-".to_string() };
        let all: Vec<String> = opts
            .iter()
            .enumerate()
            .map(|(k, op)| {
                let nz: Vec<String> = op.feats.iter().enumerate().filter(|(_, x)| **x != 0.0).map(|(i, x)| format!("{i}={x}")).collect();
                format!("{}:{}:{}:{}:{}", op.verb as usize, op.target.0, op.target.1, d.scores[k], nz.join(" "))
            })
            .collect();
        // SCRATCH (teacher lane, 2026-10-21): the "bottom of the hole" candidate sense, appended as a 15th field so
        // every reader of fields 0-13 is unchanged. Per Dig option, for reach R in BOTTOM_REACH: target row minus the
        // deepest open row of the cavity the head stands in, flooded through open cells (empty, or this ant's own
        // body) within Chebyshev R of the head -- what an ant feels by touch and gravity: is there open space lower
        // than the cell it is about to cut? `-` for other verbs.
        // SCRATCH (teacher lane, 2026-10-22): the crater refusal's label as a 17th field (the 16th, `-` here, is the
        // foraging-loop thread's teacher label): the option indices `PP_RIMVETO` would take off, a pure function of
        // the state, written whatever the brain picked; `-` when none or the refusal is off.
        let refused = rimveto().map_or_else(|| "-".to_string(), |(n, nx, nr)| {
            let r: Vec<String> = (0..opts.len()).filter(|&i| rim_cut(world, &opts[i], n, nx, nr)).map(|i| i.to_string()).collect();
            if r.is_empty() { "-".to_string() } else { r.join(";") }
        });
        let lows: Vec<i32> = BOTTOM_REACH.iter().map(|&r| cavity_low(world, organism, at, r)).collect();
        let bottom: Vec<String> = opts
            .iter()
            .map(|op| if op.verb == Verb::Dig { lows.iter().map(|l| (op.target.1 - l).to_string()).collect::<Vec<_>>().join(" ") } else { "-".into() })
            .collect();
        cline(&format!(
            "Q,{},{organism},{},{},{},{},{},{},{lab},{},{},{},{},-,{}",
            world.frame,
            at.0,
            at.1,
            u8::from(SETTLE.with(|s| s.get())),
            d.pick,
            last.map_or(-1, |v| v as i32),
            u8::from(carrying),
            f(ctx),
            f(mem0),
            all.join(";"),
            bottom.join(";"),
            refused
        ));
    }
    pub const BOTTOM_REACH: [i32; 4] = [2, 3, 4, 6];
    /// The deepest (largest y) open row connected to the head within Chebyshev `r` of it, 8-connected; the head's
    /// own row if nothing else is open.
    pub fn cavity_low(world: &World, organism: OrganismId, head: (i32, i32), r: i32) -> i32 {
        let open = |x: i32, y: i32| {
            let c = world.get(x, y);
            c.material == crate::sim::material::EMPTY || c.organism_id() == organism
        };
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![head];
        seen.insert(head);
        let mut low = head.1;
        while let Some((x, y)) = stack.pop() {
            low = low.max(y);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let n = (x + dx, y + dy);
                    if (n.0 - head.0).abs() <= r && (n.1 - head.1).abs() <= r && !seen.contains(&n) && open(n.0, n.1) {
                        seen.insert(n);
                        stack.push(n);
                    }
                }
            }
        }
        low
    }

    pub fn teach_report() -> String {
        let c: Vec<u64> = COUNTS.iter().map(|c| c.load(Ordering::Relaxed)).collect();
        format!(
            "teacher {:?}: decisions {} | dig forced {} vetoed {} | drop forced {} vetoed {} | walk-home forced {} | shut-in abstains {} | no facts {} | down forced {} | skipped (q/floor) {} | remember mismatches {} | q {} floor {} sat {}",
            arms(), c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7], c[8], c[9], c[10], q(), floor_mode(), sat()
        )
    }

    fn out() -> Option<&'static Mutex<std::io::LineWriter<std::fs::File>>> {
        static F: OnceLock<Option<Mutex<std::io::LineWriter<std::fs::File>>>> = OnceLock::new();
        F.get_or_init(|| {
            std::env::var("PP_MIND_LOG").ok().map(|p| Mutex::new(std::io::LineWriter::new(std::fs::File::create(p).expect("log"))))
        })
        .as_ref()
    }
    pub fn ablate(ctx: &mut [f32; CONTEXT]) {
        static IDX: OnceLock<Vec<usize>> = OnceLock::new();
        let idx = IDX.get_or_init(|| {
            let names = context_names();
            std::env::var("PP_ABLATE").unwrap_or_default().split(',').filter(|n| !n.is_empty())
                .map(|n| names.iter().position(|m| *m == n).unwrap_or_else(|| panic!("PP_ABLATE: no sense {n}"))).collect()
        });
        for &i in idx {
            ctx[i] = 0.0;
        }
    }
    pub fn line(s: String) {
        if let Some(f) = out() {
            let _ = writeln!(f.lock().unwrap(), "{s}");
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn decision(seed: u64, frame: u64, id: OrganismId, at: (i32, i32), ctx: &[f32; CONTEXT], mem: &[f32; MEMORY], last: Option<Verb>, opts: &[Opt], d: &Decision) {
        if out().is_none() {
            return;
        }
        let o = &opts[d.pick];
        let mut best = [f32::NAN; VERBS];
        for (k, op) in opts.iter().enumerate() {
            let v = op.verb as usize;
            if best[v].is_nan() || d.scores[k] > best[v] {
                best[v] = d.scores[k];
            }
        }
        let f = |v: &[f32]| v.iter().map(|x| format!("{x:.4}")).collect::<Vec<_>>().join(" ");
        let jobs: Vec<String> = d.jobs.iter().map(|j| j.map_or("nan".into(), |j| format!("{j:.4}"))).collect();
        line(format!(
            "M,{seed},{frame},{id},{},{},{},{},{},{},{},{},{},{}",
            at.0, at.1, o.verb as usize, o.target.0, o.target.1,
            last.map_or(-1, |v| v as i32), f(ctx), f(mem), jobs.join(" "), f(&best)
        ) + &format!(",{}", f(&o.feats)));
        // SCRATCH (teacher-training measurement 2): every Walk, Dig and Drop option, not only the chosen one, so a
        // copier can see the whole choice. Per option `verb tx ty brain_score|k:v k:v` with only nonzero features.
        let all: Vec<String> = opts
            .iter()
            .enumerate()
            .filter(|(_, op)| matches!(op.verb, Verb::Walk | Verb::Dig | Verb::Drop))
            .map(|(k, op)| {
                let nz: Vec<String> = op.feats.iter().enumerate().filter(|(_, x)| **x != 0.0).map(|(i, x)| format!("{i}:{x:.3}")).collect();
                format!("{} {} {} {:.4}|{}", op.verb as usize, op.target.0, op.target.1, d.scores[k], nz.join(" "))
            })
            .collect();
        line(format!("O,{seed},{frame},{id},{},{}", d.pick, all.join(";")));
    }
}
