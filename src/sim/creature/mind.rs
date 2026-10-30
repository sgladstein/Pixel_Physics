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
/// (context and memory, option features, verb, its gate), and [`scorer_output`] that
/// unit's output weight: a unit's inputs move a score only through it
/// (`train::no_prune_mask`, review of the `noprune=` design, 2026-10-21).
pub fn scorer_unit(slot: usize) -> Option<usize> {
    match slot {
        s if (S_CTX..S_FEAT).contains(&s) => Some((s - S_CTX) / S_IN),
        s if (S_FEAT..S_VERB).contains(&s) => Some((s - S_FEAT) / FEATURES),
        s if (S_VERB..S_OUT).contains(&s) => Some((s - S_VERB) / VERBS),
        s if (GATE..REACH_MEM).contains(&s) => Some((s - GATE) / GATE_IN),
        s if (REACH_GATE..WRITE).contains(&s) => Some((s - REACH_GATE) / VERBS),
        _ => None,
    }
}
/// Scorer unit `u`'s context gate row ([`GATE`]): its weights on context, memory and the bias.
pub fn gate_row(u: usize) -> std::ops::Range<usize> {
    assert!(u < UNITS, "scorer unit {u}: there are {UNITS}");
    GATE + u * GATE_IN..GATE + (u + 1) * GATE_IN
}
/// Memory unit `m`'s weights on what each verb could reach ([`REACH_MEM`]), in verb order.
pub fn reach_mem_row(m: usize) -> std::ops::Range<usize> {
    assert!(m < MEMORY, "memory unit {m}: there are {MEMORY}");
    REACH_MEM + m * VERBS..REACH_MEM + (m + 1) * VERBS
}
/// Memory unit `m`'s write gate ([`WRITE`]): its weights on context, memory, each verb's reach and the bias.
pub fn write_row(m: usize) -> std::ops::Range<usize> {
    assert!(m < MEMORY, "memory unit {m}: there are {MEMORY}");
    WRITE + m * W_IN..WRITE + (m + 1) * W_IN
}
/// Scorer unit `u`'s gate weights on what each verb could reach at the last decision ([`REACH_GATE`]), in verb order.
pub fn reach_gate_row(u: usize) -> std::ops::Range<usize> {
    assert!(u < UNITS, "scorer unit {u}: there are {UNITS}");
    REACH_GATE + u * VERBS..REACH_GATE + (u + 1) * VERBS
}
/// The output weight of scorer unit `u` ([`scorer_unit`]).
pub fn scorer_output(u: usize) -> usize {
    assert!(u < UNITS, "scorer unit {u}: there are {UNITS}");
    S_OUT + u
}
/// SCRATCH (teacher lane, 2026-10-26): every weight of scorer unit `u` -- its context row and its feature row.
pub fn unit_rows(u: usize) -> Vec<usize> {
    (0..S_IN).map(|i| S_CTX + u * S_IN + i).chain((0..FEATURES).map(|i| S_FEAT + u * FEATURES + i)).collect()
}
/// SCRATCH (teacher lane, 2026-10-26): the slots a come-home lesson may train (`examples/homefit.rs`). Every link
/// from the far-from-home sense -- context `c::D_SCENT + 2` into the scorer's units, the job rows and the hidden units,
/// and option feature `f::LAST_FOOD` into the units: the far-home-sense thread's allow-list of 2026-10-22, rebuilt for
/// this layout -- and, with `unit`, that scorer unit's energy and bias inputs, its `f::HOME` and `f::LAST_FOOD`
/// features and its output weight, so a unit can gate the home pull on the tank. Both inputs read 0 at the door, so
/// what the lesson changes is how an ant away from home walks.
pub fn home_lesson_slots(unit: Option<usize>) -> Vec<usize> {
    let far = c::D_SCENT + 2;
    let mut v: Vec<usize> = (0..UNITS).map(|u| S_FEAT + u * FEATURES + options::f::LAST_FOOD).collect();
    v.extend((0..UNITS).map(|u| S_CTX + u * S_IN + far));
    v.extend((0..VERBS).map(|j| JOB + j * J_IN + far));
    v.extend((0..HIDDEN).map(|h| HID + h * H_IN + far));
    if let Some(u) = unit {
        v.extend([S_CTX + u * S_IN + c::ENERGY, S_CTX + u * S_IN + CONTEXT + MEMORY, S_FEAT + u * FEATURES + options::f::HOME, S_OUT + u]);
    }
    v.sort_unstable();
    v.dedup();
    v
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
/// **Each scorer unit's context gate** (layout 6, owner 2026-10-26): one row per unit over this decision's context,
/// memory and a bias, `y`, and the unit's per-option part -- its feature sum -- is multiplied by
/// `clamp(1 + y, 0, 2)` ([`gate`]). **Why**: a scorer unit is a sum, so the ant's own state (hunger, a load, how far
/// out) reaches a score only as a constant added to every option, and a unit that should be *off* -- a fed ant
/// ignoring the way home -- still adds its saturated level to every option and every job. A lesson copied into an
/// ungated unit reached the come-home teacher only by moving fed ants' choices 0.6-0.9 nats, and under a hold that
/// kept them still it learned nothing (0.30 from 0.28); a gated unit reading only energy, the far sense and the home
/// bearing reached 0.87 at a hold of 0.04, and on four far-food worlds starved 107 ants against the hand-built
/// home-low sense's 177 and colony 13's 1,107 (`Reports/brain-training/research/brain-design-review-2026-10-26.md`).
/// **Why a clamp**: exactly 1 at all-zero weights, so every genome saved before it scores bit for bit as it did; it
/// shuts exactly at `y = -1`, where `1 + squash` (the legal smooth form) needs `y = -9` to reach a tenth and the
/// proposed `1 + tanh` reads a transcendental, which nothing a decision reads may do (module doc). Its flat edges
/// give a gradient fit nothing to follow, which is why it was tested by mutation search, as the brain trains.
/// Appended last, as the hidden units were, so every earlier slot keeps its index ([`enter_gate`]).
const GATE: usize = HID_OUT + VERBS * HIDDEN;
/// A gate row's inputs: context, memory and a bias, the per-decision row's shape ([`S_IN`]).
pub const GATE_IN: usize = CONTEXT + MEMORY + 1;
/// **What each verb could reach, into memory** (layout 7, owner 2026-10-26 22:20Z): one row per memory unit, one
/// weight per verb, over this decision's [`reach`] -- 0.25 for having a target at all, plus up to 0.75 by how well
/// its best target scores; 0 when it has none. **Why**: memory read the context, the best and mean score over *every* option and the verb just chosen,
/// and nothing per verb, so "there was food to take at home" was not something it could hold. The come-home lesson
/// copied into a gated unit starved every far lab box at the door (hungry ants ran home to an empty nest), and the
/// owner ruled against a hand-made "food at home" sense in favour of the general one: the brain already scores what
/// each verb could do, and memory may now keep it. Added only where a weight is not zero, so every genome saved
/// before it remembers bit for bit as it did.
const REACH_MEM: usize = GATE + UNITS * GATE_IN;
/// **...and into each scorer unit's gate** ([`GATE`]): one row per unit, one weight per verb, over the [`reach`] of the
/// ant's *previous* decision (`MindState::reach`). The last one, because this decision's best targets are scored by
/// the very units the gate scales -- a gate reading them would be a loop; memory, which updates after the pick, reads
/// this decision's.
const REACH_GATE: usize = REACH_MEM + MEMORY * VERBS;
/// **Each memory unit's write gate** (layout 8, 2026-10-27; prototype, trainable only under `PP_WRITE=on`): one row
/// per memory unit over this decision's context, the memory before it, what each verb could reach ([`reach`]) and a
/// bias, `y`, and the unit moves toward its new value by `z = clamp((1 - rate) + y, 0, 1)`: `m' = m + z (squash(x) -
/// m)`. **Why**: a memory unit's one rate gene sets both how fast it writes and how long it holds, so it cannot write
/// in a few decisions at the nest and then hold through a trip; the only way to hold a fact was a flip-flop -- a
/// self-weight well past 1 balanced against set and reset inputs -- and the knock-out landscape of the hand-laid "home
/// had food" memory found no step-by-step path to one (every piece necessary, half strength acting like removal, a
/// broken set input losing 13-14 of 16 colonies; `Reports/brain-training/research/latch-landscape-2026-10-27.md`).
/// With a write gate, holding is the rate gene at its top and writing is one weight (on `c::AT_NEST`, say). At all-zero
/// weights `z` is the old `1 - rate` and the old formula is used outright, so every genome saved before it remembers
/// bit for bit as it did.
const WRITE: usize = REACH_GATE + UNITS * VERBS;
/// A write gate row's inputs: context, memory, each verb's reach and a bias.
pub const W_IN: usize = CONTEXT + MEMORY + VERBS + 1;
/// The whole genome.
pub const GENOME_LEN: usize = WRITE + MEMORY * W_IN;
/// **A genome saved before the context gates** (layouts 4-5; 2,797 slots): today's, cut short at [`GATE`].
pub const GENOME_LEN_V5: usize = GATE;
/// **A genome saved before the reach rows** (layout 6; 3,357 slots): today's, cut short at [`REACH_MEM`].
pub const GENOME_LEN_V6: usize = REACH_MEM;
/// **A genome saved before the write gates** (layout 7; 3,609 slots): today's, cut short at [`WRITE`].
pub const GENOME_LEN_V7: usize = WRITE;
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
pub const GENOME_LEN_V2: usize = GENOME_LEN_V5 - GREW;
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
    /// SCRATCH (dig-scoring thread): this genome's own dig head, under `PP_DIGHEAD_EVOLVE=1`, from the slots past
    /// [`GENOME_LEN`] ([`scratch_log::head_evolve`]); otherwise none and the side file's head is everyone's.
    pub head: Option<Arc<scratch_log::DigHead>>,
    /// SCRATCH (brood lane, 2026-10-24): this genome's lay gate ([`lay_bar_scale`]), under `PP_LAYGENE` (on by default), from the last
    /// [`LAY_LEN`] slots (past the dig head when that rides too); otherwise none, and the body lays as before.
    pub lay: Option<[f32; LAY_LEN]>,
}

impl MindGenome {
    pub fn new(mut w: Vec<f32>) -> MindGenome {
        // A genome from before the context gates or the reach rows gains them at zero ([`enter_gate`]), riders kept.
        enter_gate(&mut w, None);
        // SCRATCH (brood lane): an evolvable lay gate rides last (`PP_LAYGENE`), so it comes off before the head.
        let hl = scratch_log::head_evolve().unwrap_or(0);
        let lay = if laygene_on() && w.len() == GENOME_LEN + hl + LAY_LEN {
            let mut g = [0.0; LAY_LEN];
            g.copy_from_slice(&w[GENOME_LEN + hl..]);
            w.truncate(GENOME_LEN + hl);
            Some(g)
        } else {
            None
        };
        // SCRATCH (dig-scoring thread): an evolvable dig head rides past GENOME_LEN (`PP_DIGHEAD_EVOLVE=1`).
        let head = match scratch_log::head_evolve() {
            Some(hl) if w.len() == GENOME_LEN + hl => {
                let h = scratch_log::dighead().expect("PP_DIGHEAD_EVOLVE needs PP_DIGHEAD").from_flat(&w[GENOME_LEN..]);
                w.truncate(GENOME_LEN);
                Some(Arc::new(h))
            }
            _ => None,
        };
        assert_eq!(
            w.len(),
            GENOME_LEN,
            "a mind genome is exactly GENOME_LEN; a short one means the layout changed"
        );
        let live = w[S_CTX..].iter().filter(|v| v.abs() >= W_EPS).count() as u32;
        let plan = ScorerPlan::new(&w);
        MindGenome { w, live, plan, head, lay }
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
    /// `(lane, unit)` for every lane whose unit has a gate weight that is not zero ([`GATE`]): empty in every genome
    /// saved before the gates, which then scores exactly as it did.
    gated: Vec<(usize, usize)>,
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
        let gated = units.iter().enumerate().filter(|&(_, &u)| gated(g, u)).map(|(k, &u)| (k, u)).collect();
        ScorerPlan { units, feats, gated }
    }

    /// Every unit and every weight, zeros included: the sum as [`decide`]
    /// took it before the plan, kept as the reference the plan is held to.
    #[cfg(test)]
    fn dense(g: &[f32]) -> ScorerPlan {
        let feats = (0..FEATURES).map(|i| (i, std::array::from_fn(|u| g[S_FEAT + u * FEATURES + i]))).collect();
        // Gated exactly when the sparse plan is (a live unit with a gate weight), so both take the same sum.
        let gated = (0..UNITS).filter(|&u| g[S_OUT + u] != 0.0 && gated(g, u)).map(|u| (u, u)).collect();
        ScorerPlan { units: (0..UNITS).collect(), feats, gated }
    }
}

/// Whether scorer unit `u` has a gate weight that is not zero ([`GATE`]).
fn gated(g: &[f32], u: usize) -> bool {
    g[gate_row(u)].iter().chain(&g[reach_gate_row(u)]).any(|&w| w != 0.0)
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
/// - **Before the context gates** ([`GENOME_LEN_V5`]) it gains them, all
///   zero, which [`decide`] skips ([`enter_gate`]).
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
pub fn pad(mut g: Vec<f32>) -> Option<Vec<f32>> {
    match g.len() {
        GENOME_LEN => Some(g),
        GENOME_LEN_V5 | GENOME_LEN_V6 | GENOME_LEN_V7 => {
            enter_gate(&mut g, None);
            Some(g)
        }
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
///
/// **Layout 5** (2026-10-25) is layout 4 with `f::FOOD_WAY` live: the same
/// length and slots, but a file marked 4 or older has its weights on that
/// slot zeroed as it is read ([`enter_last_food`]), so the last-food memory
/// enters every older pool at weight 0.
///
/// **Layout 6** (2026-10-26) appends the scorer units' context gates ([`GATE`]), [`UNITS`] x [`GATE_IN`] slots after
/// the job head's hidden units, so the core grows from [`GENOME_LEN_V5`] to [`GENOME_LEN`] and every rider (the dig
/// head, the lay gate) moves along by that much. A file marked 5 or older, or unmarked, gains them at zero as it is
/// read ([`enter_gate`]), which scores exactly as it did.
///
/// **Layout 7** (2026-10-26) appends what each verb could reach, into memory ([`REACH_MEM`]) and into the gates
/// ([`REACH_GATE`]): [`MEMORY`] x [`VERBS`] and [`UNITS`] x [`VERBS`] slots after the gates, the core growing from
/// [`GENOME_LEN_V6`] to [`GENOME_LEN_V7`]. A file marked 6 or older, or unmarked, gains them at zero ([`enter_gate`]).
///
/// **Layout 8** (2026-10-27) appends each memory unit's write gate ([`WRITE`]), [`MEMORY`] x [`W_IN`] slots, the core
/// growing from [`GENOME_LEN_V7`] to [`GENOME_LEN`]. A file marked 7 or older, or unmarked, gains them at zero.
pub const LAYOUT: u32 = 8;

/// **A genome saved before the context gates, the reach rows or the write gates, in today's layout** ([`GATE`],
/// [`REACH_MEM`], [`WRITE`]): its core gains every block appended since, at zero, inserted before any rider (the dig
/// head, the lay gate), so every slot keeps its meaning and it scores exactly as it did; `true` when it did. Old or new
/// is told by `layout` where the file names one (5 or older gains every block, 6 the reach rows and write gates, 7 the
/// write gates) and by length where it does not: every layout-5 length, riders included (at most [`GENOME_LEN_V5`] plus
/// a 482-slot head and the [`LAY_LEN`] gate), is shorter than a layout-6 bare core, so `GENOME_LEN_V5..GENOME_LEN_V6`
/// is layout 5; a length in `GENOME_LEN_V6..GENOME_LEN_V7` is layout 6 (bare, or with the lay gate), and one in
/// `GENOME_LEN_V7..GENOME_LEN` layout 7. **An unmarked layout-6 genome with a dig head is not told apart** from a
/// layout-7 one by length (3,839-3,845 against 3,609-4,097): every file `train::save_genome` writes is marked, so it
/// reaches here unmarked only from code that built it, which builds today's. A V1 or V2 genome is [`pad`]'s.
/// **Every reader of a saved mind calls this before it splits the riders off** (`train::load_mind`, the lab's pools,
/// `inrun`'s founders, `labsurvive`), and [`MindGenome::new`] calls it on anything it is handed, so an example that
/// builds a mind straight from an old file runs it as saved.
pub fn enter_gate(g: &mut Vec<f32>, layout: Option<u32>) -> bool {
    let n = g.len();
    // The core length the genome was saved with, where it is older than today's.
    let saved = match layout {
        Some(l) if l >= 8 => {
            assert!(n >= GENOME_LEN, "a genome marked layout {l} is {n} slots, shorter than this layout's core {GENOME_LEN}");
            None
        }
        Some(l @ (6 | 7)) => {
            let core = if l == 6 { GENOME_LEN_V6 } else { GENOME_LEN_V7 };
            assert!(n >= core, "a genome marked layout {l} is {n} slots, shorter than that layout's core {core}");
            Some(core)
        }
        _ if (GENOME_LEN_V5..GENOME_LEN_V6).contains(&n) => Some(GENOME_LEN_V5),
        None if (GENOME_LEN_V6..GENOME_LEN_V7).contains(&n) => Some(GENOME_LEN_V6),
        None if (GENOME_LEN_V7..GENOME_LEN).contains(&n) => Some(GENOME_LEN_V7),
        _ => None,
    };
    match saved {
        Some(at) => {
            g.splice(at..at, std::iter::repeat_n(0.0, GENOME_LEN - at));
            true
        }
        None => false,
    }
}

/// **The last-food memory enters an older genome at weight 0**
/// (`options::last_food`): with the memory on and `layout` older than 5 (or
/// unmarked), every scorer unit's weight on `f::FOOD_WAY` is set to 0, and
/// the count of links cut is returned. The slot was `f::BUILDING`, which
/// nothing wrote, so those weights moved no choice and drifted freely (23
/// of the standard pool's 30 members carry one, member 0 six); left in
/// place they would act on the new sense at random the moment it reads.
/// A layout-5 file keeps them: they were selected with the memory on.
/// Every reader of a saved mind calls this (`train::load_mind`, the lab's
/// pools, `inrun`'s founders, `labsurvive`), riders past [`GENOME_LEN`]
/// untouched.
pub fn enter_last_food(g: &mut [f32], layout: Option<u32>) -> usize {
    if options::last_food().is_none() || layout.is_some_and(|l| l >= 5) {
        return 0;
    }
    let mut cut = 0;
    for u in 0..UNITS {
        let k = S_FEAT + u * FEATURES + options::f::FOOD_WAY;
        if g[k] != 0.0 {
            g[k] = 0.0;
            cut += 1;
        }
    }
    cut
}

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
    // What the verb could reach, into memory and the gates: 0 while it has no target.
    out.extend((0..MEMORY).map(|m| REACH_MEM + m * VERBS + v..REACH_MEM + m * VERBS + v + 1));
    out.extend((0..UNITS).map(|u| REACH_GATE + u * VERBS + v..REACH_GATE + u * VERBS + v + 1));
    out.extend((0..MEMORY).map(|m| WRITE + m * W_IN + CONTEXT + MEMORY + v..WRITE + m * W_IN + CONTEXT + MEMORY + v + 1));
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
/// at 0), the context gates under `PP_GATE=off` ([`gate_on`]) and the reach rows under `PP_REACH=off` ([`reach_on`];
/// the gates' reach rows under either), and the write gates unless `PP_WRITE=on` ([`write_on`]). A switch
/// the arena allows is trainable: mutation can turn a verb off, and back on.
pub fn trainable(verbs: &[Verb], emits: [bool; EMITS]) -> Vec<usize> {
    (0..GENOME_LEN)
        .filter(|&i| match i {
            i if i < SHARP => verbs.contains(&Verb::ALL[i - SWITCH]),
            i if (EMIT..HID).contains(&i) => emits[(i - EMIT) / E_IN],
            i if (GATE..REACH_MEM).contains(&i) => gate_on(),
            i if (REACH_MEM..REACH_GATE).contains(&i) => reach_on(),
            i if (REACH_GATE..WRITE).contains(&i) => gate_on() && reach_on(),
            i if i >= WRITE => write_on(),
            _ => true,
        })
        .collect()
}

/// **Every slot that serves verb `v` alone**: its switch, its job row
/// (context, memory, best target, constant), its row from the hidden units,
/// each scorer unit's weight on it, and each memory unit's weight on "this
/// verb was just chosen" and on what the verb could reach (memory's and the gates').
/// Nothing else reads them, so in a world where `v`
/// never has a target they cannot change a single decision.
pub fn verb_slots(v: Verb) -> Vec<usize> {
    let v = v as usize;
    let mut out = vec![SWITCH + v];
    out.extend(JOB + v * J_IN..JOB + (v + 1) * J_IN);
    out.extend(HID_OUT + v * HIDDEN..HID_OUT + (v + 1) * HIDDEN);
    out.extend((0..UNITS).map(|u| S_VERB + u * VERBS + v));
    out.extend((0..MEMORY).map(|m| MEM + m * M_IN + CONTEXT + 2 + v));
    out.extend((0..MEMORY).map(|m| REACH_MEM + m * VERBS + v));
    out.extend((0..UNITS).map(|u| REACH_GATE + u * VERBS + v));
    out.extend((0..MEMORY).map(|m| WRITE + m * W_IN + CONTEXT + MEMORY + v));
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

/// **The bite's entry into a colony mind** (lone predator thread, stage two, 2026-10-24): where a genome holds
/// Attack banned (its switch at exactly -1, as every rung so far has held it), the switch comes on and its job's
/// constant is set to `offset`; returns whether it changed anything. `offset` 0 is a blank verb's entry, as
/// [`enter_rung`] opens one: offered beside the others with no weight of its own, so mutation decides what it is
/// for. A genome that already has the verb on is left alone, so a later round's founders keep what they evolved.
/// Only under [`options::attack_on`] (on by default; `PIXEL_PHYSICS_MIND_ATTACK=off`, nothing changes).
pub fn enter_bite(g: &mut [f32], offset: f32) -> bool {
    let s = SWITCH + Verb::Attack as usize;
    if !options::attack_on() || g[s] != -1.0 {
        return false;
    }
    g[s] = 1.0;
    g[JOB + Verb::Attack as usize * J_IN + CONTEXT + MEMORY + 1] = offset;
    true
}

/// **The bite's two genes as a genome holds them**: its switch (-1 banned) and its job's constant, the slot
/// [`enter_bite`] writes. SCRATCH (colony-vs-colony thread, 2026-10-24): `examples/rivalbed.rs` reports them per pool,
/// so a run that never bites can be told from a pool that never could.
pub fn bite_genes(g: &[f32]) -> (f32, f32) {
    (g[SWITCH + Verb::Attack as usize], g[JOB + Verb::Attack as usize * J_IN + CONTEXT + MEMORY + 1])
}

/// **Set the bite as a harness wants it, whatever the genome evolved**: `Some(k)` switches it on at job constant `k`
/// (a blank verb at 0, held off at -100 as the predator lane's `ant_bite=-100`), `None` bans it (switch -1, the
/// option never listed). SCRATCH (colony-vs-colony thread, 2026-10-24): the arms of `examples/rivalbed.rs`, where one
/// colony bites and its neighbour cannot. [`enter_bite`] leaves an evolved bite alone, which is right for a line and
/// wrong for an arm.
pub fn set_bite(g: &mut [f32], k: Option<f32>) {
    let s = SWITCH + Verb::Attack as usize;
    match k {
        Some(k) => {
            g[s] = 1.0;
            g[JOB + Verb::Attack as usize * J_IN + CONTEXT + MEMORY + 1] = k;
        }
        None => g[s] = -1.0,
    }
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
/// every tick. **Both are written now, by default** (noted 2026-10-25,
/// not changed here): the building slot holds the last-food memory
/// (`options::f::FOOD_WAY`, which [`enter_rung`] spares while it is on),
/// and the last-food slot the far-from-home sense (`options::home_vec`),
/// which a rung entry that cuts unread links still cuts.
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
/// * a scorer unit whose output weight is 0: its whole fan-in, its gate
///   row included;
/// * an input nothing writes ([`UNWRITTEN`]), on every scorer unit;
/// * a verb the rung never offers: the scorer's weight on it, its job's
///   reads (not its offset: a ban lifted later brings the verb back at the
///   offset it has, and at 0 it would be drawn about half the time,
///   [`QUIET`]'s reason), its weights on the hidden units, and memory's
///   read of "it was chosen";
/// * a hidden unit no job reads, and a memory unit nothing but itself
///   reads (a scorer unit's gate is a reader), until none is left:
///   clearing one can leave another unread.
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
            .chain(S_VERB + u * VERBS..S_VERB + (u + 1) * VERBS)
            .chain(gate_row(u))
            .chain(reach_gate_row(u));
        kill(g, fan_in, &mut cut[0]);
    }
    // The building slot holds the last-food memory while that is on
    // (`options::f::FOOD_WAY`): written, so not cut.
    let food_way = options::last_food().is_some();
    for u in 0..UNITS {
        let unwritten = UNWRITTEN.into_iter().filter(|&k| !(food_way && k == options::f::FOOD_WAY));
        kill(g, unwritten.map(|k| S_FEAT + u * FEATURES + k), &mut cut[1]);
    }
    for v in Verb::ALL.into_iter().filter(|v| !verbs.contains(v)).map(|v| v as usize) {
        let job = JOB + v * J_IN;
        let reads = (0..UNITS)
            .map(|u| S_VERB + u * VERBS + v)
            .chain(job..job + CONTEXT + MEMORY + 1)
            .chain((0..HIDDEN).map(|h| HID_OUT + v * HIDDEN + h))
            .chain((0..MEMORY).map(|m| MEM + m * M_IN + CONTEXT + 2 + v))
            .chain((0..MEMORY).map(|m| REACH_MEM + m * VERBS + v))
            .chain((0..UNITS).map(|u| REACH_GATE + u * VERBS + v))
            .chain((0..MEMORY).map(|m| WRITE + m * W_IN + CONTEXT + MEMORY + v));
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
            let read = (0..UNITS).any(|u| g[S_CTX + u * S_IN + at] != 0.0 || g[GATE + u * GATE_IN + at] != 0.0)
                || (0..VERBS).any(|v| g[JOB + v * J_IN + at] != 0.0)
                || (0..HIDDEN).any(|h| g[HID + h * H_IN + at] != 0.0)
                || (0..EMITS).any(|e| emits[e] && g[EMIT + e * E_IN + at] != 0.0)
                || (0..MEMORY).any(|r| r != m && g[MEM + r * M_IN + CONTEXT + 2 + VERBS + m] != 0.0)
                || (0..MEMORY).any(|r| r != m && g[WRITE + r * W_IN + CONTEXT + m] != 0.0);
            if !read {
                again |= kill(g, (MEM + m * M_IN..MEM + (m + 1) * M_IN).chain(reach_mem_row(m)).chain(write_row(m)), &mut cut[3]);
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
///   which a score must not reward like a follower;
/// * `hunter` -- the lone predator's starting mind (`train::predator`):
///   bites any stranger in reach, picks up flesh, never rests, and walks
///   where the ants' scent A grows, keeping off ledges -- `follower` on the
///   ants' home trail instead of the food trail, with the bite on top.
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
        "hunter" => {
            g[S_FEAT + f::SCENT + 1] = 4.0; // a, stronger there
            g[S_FEAT + f::SCENT + 2] = 4.0; // ...and two cells on
            g[S_FEAT + f::FALL] = -4.0;
            g[S_OUT] = 3.0;
            g[job(Verb::Walk, best)] = 3.0;
            g[job(Verb::TurnAround, best)] = 3.0;
            g[job(Verb::Rest, offset)] = -3.0;
            g[job(Verb::Attack, offset)] = 6.0;
            g[job(Verb::PickUp, offset)] = 3.0;
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

/// **Which slots cross over together**: one scorer unit's whole fan-in, its
/// gate and its output weight; one job row; one memory row with its rate and its reach row; one emit
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
        s if s < GATE => UNIT_BASE + 500 + (s - HID_OUT) % HIDDEN,
        // A scorer unit's gate crosses with the unit it gates, and a reach row with the memory unit or gate it feeds.
        s if s < REACH_MEM => UNIT_BASE + (s - GATE) / GATE_IN,
        s if s < REACH_GATE => UNIT_BASE + 100 + (s - REACH_MEM) / VERBS,
        s if s < WRITE => UNIT_BASE + (s - REACH_GATE) / VERBS,
        // A write gate travels with its memory unit.
        s if s < GENOME_LEN => UNIT_BASE + 100 + (s - WRITE) / W_IN,
        // SCRATCH (dig-scoring thread): an evolvable dig head (`PP_DIGHEAD_EVOLVE`) crosses whole, from one parent
        _ => UNIT_BASE + 900,
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
        "verbs [{}] sharp {:.2} pull {:.2} | live: scorer ctx {} feat {} verb {} out {} gate {}, job {}, memory {}, emit {}, job hidden {}, reach {}, write {} (of {})",
        on.join(","),
        sharpness(g),
        pull(g),
        live(S_CTX, S_FEAT),
        live(S_FEAT, S_VERB),
        live(S_VERB, S_OUT),
        live(S_OUT, JOB),
        live(GATE, REACH_MEM),
        live(JOB, MEM),
        live(MEM, EMIT),
        live(EMIT, HID),
        live(HID, GATE),
        live(REACH_MEM, WRITE),
        live(WRITE, GENOME_LEN),
        GENOME_LEN - S_CTX,
    )
}

/// SCRATCH (teacher lane, 2026-10-26): `PP_SHARP_MAX=x` lowers the sharpness ceiling from 4. Every trained genome
/// sits past the top (the forager seed set `g[SHARP] = 3.0`, colony 13's 60 genomes read 5.3-6.7), so the gene is
/// neutral to mutation and 80% of walk scores are past +-0.95: in 72% of decisions every step is a flat "no" and the
/// draw among them is near-uniform, so a graded leaning (home as the tank drops) cannot move a pick and selection
/// cannot see it. Recomputed from logged scores (copy log, colony 13, world 32007): pinned 80% at 4x, 62% at 2x, 31%
/// at 1x, 4% at 0.5x. Colony 13 at 2x on 8 far-moving worlds: starved 257 vs 240, open nest 350 vs 398; at 1x:
/// starved 296, open 178 -- it lives, but its constants were calibrated at 4x.
fn sharpness(g: &[f32]) -> f32 {
    static MAX: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    let max = *MAX.get_or_init(|| std::env::var("PP_SHARP_MAX").ok().map_or(4.0, |v| v.parse().expect("PP_SHARP_MAX")));
    (1.0 + g[SHARP]).clamp(0.25, max.max(0.25))
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
        ) || (self == Verb::Attack && options::attack_on())
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
    /// SCRATCH (brood lane, 2026-10-24): joules nestmates have fed this ant lately and the frame that sum was last
    /// brought up to date, for the lay gate's "fed" sense ([`lay_bar_scale`]; written only under `PP_LAYGENE`).
    pub fed: (f32, u64),
    /// What each verb could reach at the last decision ([`reach`]), for the gates' reach rows ([`REACH_GATE`]); 0
    /// before the first. A harness that rebuilds a state from a log row must set it from the ant's previous row.
    pub reach: [f32; VERBS],
}

impl Default for MindState {
    fn default() -> Self {
        MindState {
            mem: [0.0; MEMORY],
            last_verb: None,
            last: None,
            fed: (0.0, 0),
            reach: [0.0; VERBS],
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
    /// **The bite** ([`attack`], `options::attack_on`): bites that wore a
    /// stranger, bites that killed it, and cells of flesh swallowed off a
    /// living body (a corpse picked up is a `swallowed`, not one of these).
    pub bites: u64,
    pub kills: u64,
    pub flesh: u64,
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
    /// **Where the colony's shared stomach is closed** (the colony mind seat's "close the stomach" verb,
    /// `Reports/teaching-the-colony-2026-10-25/seat-colony-mind.md`): discs `(x, y, radius)` inside which an adult
    /// whose head stands there is not offered Feed to another adult (`options`). Larvae are still fed. Empty -- every
    /// world but the colony mind demo -- changes nothing.
    pub closed_stomach: Vec<(i32, i32, i32)>,
}

/// **Whether every adult creature in the world decides by a mind**, so the
/// game ant's hand rules have no reader left in it. A world with no minds
/// (the game) reads false; so does one with a single creature that walks by
/// the game's rules, today's brain included.
pub fn every_ant_has_one(world: &World) -> bool {
    // SCRATCH (copy test, 2026-10-27): under `PP_SHADOW` / `PP_OLDLAB` the game ant acts (or labels in a clone), so it
    // needs its ways in every harness, not only where the harness sets `build_ways` (inrun does; the lab did not, and a
    // shadowed two_larders colony then lived 555 against the plain hand-built ant's 342 on seed 1).
    if scratch_log::shadow_on() || scratch_log::oldlab_on() {
        return false;
    }
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
    uncaste(world, organism);
}

/// **No caste for a mind** (`PP_MIND_CASTELESS`, old-ant-rules-2026-10-25 H1). The game's storeroom
/// (`Storeroom::SHIPPED`, `caste=4`) stamps one ant in four by its id as a nest worker for life, and that stamp
/// decided two things a mind ant could not choose: whether a touching larva may feed from its bank
/// (`brood::NurseMode::Workers`) and whether the founding cut counts as home for its put-downs
/// (`Storeroom::worker_home`). The owner's rule is one brain for every ant and no castes (2026-10-21). On: a mind
/// ant is never stamped ([`give`], [`inherit`] clear `nest_bound_until`), any mind adult may feed a touching larva,
/// and the cut is home for every mind ant. Off: the stamp as before. The game ant (no mind) is untouched either way.
/// `PP_MIND_CASTELESS=nurse` or `=home` opens one reader alone and keeps the stamp, to tell the two apart.
///
/// **Off when unset**, because on the pool trained under the caste it costs nest and crater. Training bed (inrun
/// sel=off, std_pool, 200k frames, seeds 101-108, paired): food carried home 12.4-14.9k -> 12.9-17.8k (more in 7
/// of 8), larva feeding 21-34 -> 21-50 kJ, but nest 532-709 -> 458-669 cells (smaller in 6 of 8) and crater
/// 1-9 -> 4-20 (bigger in 6 of 8). All of it is the `nurse` part: `home` alone is bit-identical to off on the bed
/// (no mind put-down there lands in the cut outside the door's reach). Lab, fruit 150, seeds 1-8: alive median 96
/// both ways. The test the owner's rule needs is a pool trained without it: the Teacher lane's arm.
pub fn casteless() -> bool {
    caste_parts() == (true, true)
}

/// [`casteless`]'s two readers: (any mind adult nurses, the cut is every mind ant's home).
fn caste_parts() -> (bool, bool) {
    static V: std::sync::OnceLock<(bool, bool)> = std::sync::OnceLock::new();
    *V.get_or_init(|| match std::env::var("PP_MIND_CASTELESS").as_deref().map(str::trim) {
        Ok("nurse") => (true, false),
        Ok("home") => (false, true),
        Ok(v) if super::options::switch_on_value(v) => (true, true),
        _ => (false, false),
    })
}

/// **The door column's parts a world of minds drops** (`PP_MIND_DOOR_OPEN`; see `creature::door_column_of`):
/// `(drop ban off, clear off)`. `=drop` lifts the ban on putting a pellet there, `=clear` stops the world deleting
/// soil there, `=1`/`on` both. **Off when unset**: on the pool trained under it, the clear is load-bearing.
/// Training bed (as [`casteless`]'s): with it, food carried home 12.4-14.9k; ban lifted, clear kept 11.4-19.2k
/// (nest smaller in 6 of 8, crater the same); clear stopped, ban kept 4.6-8.2k and starved 18-45 -> 25-68; both
/// 3.0-4.2k, starved 43-127. Lab, fruit 150, seeds 1-8, both off: alive median 96 -> 108 (15k frames is too short
/// for a plugged door to cost). A colony that plugs its own door has never paid for it in training.
pub fn door_open() -> (bool, bool) {
    static V: std::sync::OnceLock<(bool, bool)> = std::sync::OnceLock::new();
    *V.get_or_init(|| match std::env::var("PP_MIND_DOOR_OPEN").as_deref().map(str::trim) {
        Ok("drop") => (true, false),
        Ok("clear") => (false, true),
        Ok(v) if super::options::switch_on_value(v) => (true, true),
        _ => (false, false),
    })
}

/// Under [`casteless`] (or its `nurse` part): `organism` is a mind ant, so any larva it touches may feed from it.
pub fn casteless_nurse(world: &World, organism: OrganismId) -> bool {
    caste_parts().0 && has(world, organism)
}

/// Under [`casteless`] (or its `home` part): `organism` is a mind ant, so the founding cut is its home.
pub fn casteless_home(world: &World, organism: OrganismId) -> bool {
    caste_parts().1 && has(world, organism)
}

/// Clear the caste stamp off a mind ant under [`casteless`].
fn uncaste(world: &mut World, organism: OrganismId) {
    if casteless() {
        clear_caste(world, organism);
    }
}

/// [`uncaste`]'s write, whatever the switch says.
fn clear_caste(world: &mut World, organism: OrganismId) {
    if let Some(st) = world.organism_mut(organism) {
        st.nest_bound_until = 0;
    }
}

/// Whether `organism` decides by a mind.
pub fn has(world: &World, organism: OrganismId) -> bool {
    world.minds.as_ref().is_some_and(|m| m.ants.contains_key(&organism))
}

/// The genome `organism` decides by, if it has a mind (brood included: an egg carries its layer's).
pub fn genome_of(world: &World, organism: OrganismId) -> Option<Arc<MindGenome>> {
    world.minds.as_ref().and_then(|m| m.ants.get(&organism)).map(|m| m.genome.clone())
}

/// The verb this ant's mind chose at its last decision (a harness probe's read).
pub fn last_verb_of(world: &World, organism: OrganismId) -> Option<Verb> {
    world.minds.as_ref().and_then(|m| m.ants.get(&organism)).and_then(|m| m.state.last_verb)
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
            uncaste(world, child);
        }
        None => {
            // An id can come back after its slot's generation wraps; a
            // child of a mindless parent must not inherit a stale mind.
            minds.ants.remove(&child);
        }
    }
}

/// SCRATCH (brood lane, 2026-10-24): **how many slots the lay gate takes** -- a bias and one weight for each of
/// [`lay_bar_scale`]'s five senses.
pub const LAY_LEN: usize = 6;
/// How long the "fed" sense remembers: the sum halves every this many frames (a step, not `exp`, so a cloud run and a
/// local one agree to the bit).
const FED_HALF: u64 = 2000;

/// The joules `fed` holds at `frame`, halved once per [`FED_HALF`] frames since it was last brought up to date.
fn fed_now((j, at): (f32, u64), frame: u64) -> f32 {
    let k = (frame.saturating_sub(at) / FED_HALF).min(40) as i32;
    (0..k).fold(j, |v, _| v * 0.5)
}

/// SCRATCH (brood lane, 2026-10-24): a nestmate fed `organism` `amount` joules mouth to mouth (`feed`); kept only
/// under `PP_LAYGENE`, for the lay gate.
fn note_fed(world: &mut World, organism: OrganismId, amount: f32) {
    let frame = world.frame;
    if let Some(m) = world.minds.as_mut().and_then(|m| m.ants.get_mut(&organism)) {
        let now = fed_now(m.state.fed, frame);
        // Bring the stamp forward only by whole half-lives, so the remainder keeps counting toward the next halving.
        let at = m.state.fed.1 + (frame.saturating_sub(m.state.fed.1) / FED_HALF) * FED_HALF;
        m.state.fed = (now + amount, at);
    }
}
/// How far the lay gate's senses reach from the head, in cells (a 13 x 13 square).
const LAY_REACH: i32 = 6;

/// SCRATCH (brood lane, 2026-10-24): `PP_LAYGENE` -- a mind ant's genome carries a lay gate ([`lay_bar_scale`]).
/// **On by default** (owner, 2026-10-24: "The default should be on unless there is uncertainty"); `=0` or `=off`
/// for none. Neutral as it starts: a genome saved without a gate has none (the body's bar), and the training bed
/// gives its founders a gate of zeros, which is the body's bar exactly -- only the bed's mutation draws differ.
pub fn laygene_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| options::switch_on("PP_LAYGENE"))
}

/// SCRATCH (brood lane, 2026-10-24): **how far a mind ant raises its own laying bar**, learned rather than fixed.
/// `Some(scale)` multiplies the bar `creature::try_bud` was about to let it lay at; `None` -- no switch, no mind, or
/// no gate in its genome -- leaves the body's bar alone.
///
/// **Why**: a mind ant's laying was the body's alone (the mind passes `Lay` 0.0), and the only brake was the
/// training bed's `lay_cap`, which the lab does not have: far64 in the owner's lab box boomed to ~710 ants and
/// then starved 817 in 10k frames with 2.4 MJ still in the box (Teacher lane, 2026-10-24). The owner asked that
/// laying balance through training.
///
/// **A bar, not a hold.** The first build held the egg while `squash(x) < -0.1`, as the game ant's
/// `BrainOutput::Lay` does. On the uncapped bed (`lay_cap_p=0`, std_pool, seed 1, 60k frames) no hand-set gate
/// braked anything: adults at 20k-60k were 192-161 with no gate, 185-161 holding when crowded, 195-172 holding
/// unless rich, 178-160 holding unless fed. Traced (crowd gate, 388k reads in 40k frames): 43% passed, from the
/// same nest cells as the holds -- the crowd round a ready ant changes tick to tick, so a hold is a few ticks'
/// delay, never a slower rate. `lay_cap` brakes by raising the bar, so the ant must earn more before each egg; the
/// gate does the same, by about `2^-x` for a gate sum `x` below 0 (held to 64x; never below the body's bar).
/// Exponential, as `lay_cap` is: a linear `1 - x` (first try) at bias -3 (4x the bar) only slowed the boom, adults at
/// 10k-60k 24/47/94/145/194/198 against 73/192/164/179/160/161 ungated -- the bed's food carries ~170-200 ants,
/// and any bar a rich ant can reach only delays it there.
///
/// `x = g . [1, crop, food, crowd, brood, fed]`, read only once the ant could afford an egg at the body's bar,
/// over what the ant itself can sense -- **never a colony-wide count** (owner, 2026-10-24: seeing the store is an
/// unrealistic sense):
/// - **bias**: alone, this is the `TRAIT_REPRODUCE_AT` gene, learned ("wait for 2^-b times the bar");
/// - **crop**: food in its own crop, / the bar;
/// - **food**: food cells free in reach, / 40 -- the store it stands in;
/// - **crowd**: cells of other animals in reach (brood excluded), / 40 -- how packed the nest is;
/// - **brood**: brood cells in reach, / 10 -- how much is already being raised;
/// - **fed**: joules nestmates fed it mouth to mouth lately (halving every [`FED_HALF`] frames), / 4 x its species'
///   `start_energy` -- the real queen's signal: she lays as fast as the workers feed her.
///
/// **The scales are measured, so each sense reads about 0..2 where laying is decided** (std_pool, bed seed 1, 30k
/// frames, sel=off, 216k reads): food cells p10/p50/p90 2/25/78, crowd cells 7/31/48, brood 0/0/0 (max 11), fed
/// 0.9/2.1/4.6 start grants. **Not rare**: a ready ant held by the nest's own gates asks again every tick, so the
/// 13 x 13 read ran ~7 times a frame there.
///
/// A genome of zeros reads `x = 0`, scale 1, so it lays exactly as without the gate. Draws nothing.
pub fn lay_bar_scale(world: &World, organism: OrganismId, def: &CreatureDef, (hx, hy): (i32, i32), bar: f32) -> Option<f32> {
    if !laygene_on() {
        return None;
    }
    let mind = world.minds.as_ref()?.ants.get(&organism)?;
    let g = mind.genome.lay?;
    if g.iter().all(|v| *v == 0.0) {
        return Some(1.0);
    }
    let corpse = world.materials.known(Known::Corpse);
    let (mut food, mut crowd, mut brood) = (0u32, 0u32, 0u32);
    for dy in -LAY_REACH..=LAY_REACH {
        for dx in -LAY_REACH..=LAY_REACH {
            let cell = world.get(hx + dx, hy + dy);
            let id = cell.organism_id();
            if id == 0 {
                if world.materials.get(cell.material).food_energy > 0.0 && Some(cell.material) != corpse {
                    food += 1;
                }
            } else if id != organism {
                match world.organism(id) {
                    Some(s) if s.brood.is_some() => brood += 1,
                    Some(_) => crowd += 1,
                    None => {}
                }
            }
        }
    }
    let crop = world.organism(organism).and_then(|s| s.crop).map_or(0.0, |c| c.worth()) / bar.max(1.0);
    let fed = fed_now(mind.state.fed, world.frame) / (4.0 * def.start_energy).max(1.0);
    let x = g[0] + g[1] * crop.min(4.0) + g[2] * food as f32 / 40.0 + g[3] * crowd as f32 / 40.0 + g[4] * brood as f32 / 10.0 + g[5] * fed;
    Some(bar_scale(x))
}

/// About `2^-x` for a gate sum `x` below 0, 1 at or above it, held to 64: a whole power of two times a straight
/// line across the fraction, so no `exp2` (libm's last bit) and still exact at every whole step.
fn bar_scale(x: f32) -> f32 {
    let up = (-x).clamp(0.0, 6.0);
    let n = up.floor();
    2f32.powi(n as i32) * (1.0 + (up - n))
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
    let scents = EMIT_CHANNELS.map(|ch| options::mind_scent(world, ch, hx, hy));
    // SCRATCH (teacher lane): `PP_SCENT_BLIND=1` reads every trail as 0, to see whether a mind uses them.
    static BLIND: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let scents = if *BLIND.get_or_init(|| std::env::var("PP_SCENT_BLIND").is_ok_and(|v| v == "1")) { [0.0; 3] } else { scents };
    v[c::SCENT..c::SCENT + 3].copy_from_slice(&scents);
    // A sighted mind's home is the prey it sees (`options::sight_on`); every other mind's, its anchor.
    let home = options::sensed_home(world, organism, def, st).0;
    let dist = home.map_or(0.0, |(ax, ay)| {
        let (dx, dy) = ((ax - hx) as f32, (ay - hy) as f32);
        (dx * dx + dy * dy).sqrt()
    });
    v[c::HOME_SEEN] = f32::from(u8::from(home.is_some()));
    v[c::HOME_DIST] = squash(dist / 32.0);
    // SCRATCH (`options::home_vec`): "how far past the food band", unsquashed, on d_alarm's slot (written over below
    // only when there is a last decision; this one wins).
    let far = options::home_vec().filter(|_| home.is_some()).map(|(r, w)| options::far_ramp(dist, r, w));
    let now = [scents[0], scents[1], scents[2], e, dist];
    if let Some(last) = state.last {
        for k in 0..3 {
            v[c::D_SCENT + k] = squash(4.0 * (now[k] - last[k]));
        }
        v[c::D_ENERGY] = squash(10.0 * (now[3] - last[3]));
        v[c::D_HOME] = squash((now[4] - last[4]) / 2.0);
    }
    if let Some(far) = far {
        v[c::D_SCENT + 2] = far;
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

/// **Whether the context gates are trainable** ([`GATE`]): on unless `PP_GATE=off`. Off, [`trainable`] leaves the gate
/// rows out, so a child's mutation draws exactly as it did before them (the slots a structural event picks from are
/// the layout-5 set) and a run from a layout-5 pool reproduces one made before the gates. Scoring is the same either
/// way: a gate row that is all zero is never read, and one that is not is a link the genome evolved or was given.
pub fn gate_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| !std::env::var("PP_GATE").is_ok_and(|v| v == "off" || v == "0"))
}

/// **Whether the reach rows are trainable** ([`REACH_MEM`], [`REACH_GATE`]): on unless `PP_REACH=off`, which leaves
/// them out of [`trainable`] as `PP_GATE=off` does the gates, so a run from a layout-6 pool draws as it did before
/// them. Scoring is the same either way: a reach weight that is 0 is never added.
pub fn reach_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| !std::env::var("PP_REACH").is_ok_and(|v| v == "off" || v == "0"))
}

/// **Whether the write gates are trainable** ([`WRITE`]): **off unless `PP_WRITE=on`** (or `1`) while they are a
/// prototype on trial (owner's card, 2026-10-27): off, mutation never touches them and a genome without them remembers
/// exactly as before; a genome that carries one still runs it either way.
pub fn write_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("PP_WRITE").is_ok_and(|v| v == "on" || v == "1"))
}

/// **What each verb could reach this decision** ([`REACH_MEM`]): 0 for a verb with no target (or one switched off),
/// and for one with a target `REACH_FLOOR + (1 - REACH_FLOOR) * (best + 1) / 2`, 0.25 to 1 by its best target's score.
/// **Why a floor** (Teacher lane, 2026-10-26, from colony 13's home-low copy log, world 13007, 10.6k decisions at the
/// nest): `best[PickUp]` at the nest is bimodal (q10 -0.987, median 0.96), and 28% of the at-nest decisions that had
/// food in reach scored it near -1 -- almost all fed ants. Without a floor those read as an empty larder, so a fed
/// ant's one look at a full store would teach its memory "nothing at home". With it, "there is something here" and
/// "it is worth doing now" are both on one number: present is never under 0.25, absent is exactly 0. A weight on an
/// absent verb adds nothing. (`Eat`'s only target is the ant's own crop and scores 0, so its reach says "I am carrying
/// food", not "food is here": the larder is `PickUp`'s.)
fn reach(best: &[f32; VERBS]) -> [f32; VERBS] {
    std::array::from_fn(|v| if best[v] == f32::NEG_INFINITY { 0.0 } else { REACH_FLOOR + (1.0 - REACH_FLOOR) * (best[v] + 1.0) / 2.0 })
}
/// What a verb with any target reaches at the least ([`reach`]).
pub const REACH_FLOOR: f32 = 0.25;

/// **Scorer unit `u`'s gate for one decision** ([`GATE`]): `clamp(1 + y, 0, 2)`, `y` its row over this decision's
/// context, memory and bias, summed in that order, and its reach row over the last decision's [`reach`]
/// ([`REACH_GATE`]).
fn gate(g: &[f32], u: usize, ctx: &[f32; CONTEXT], mem: &[f32; MEMORY], last_reach: &[f32; VERBS]) -> f32 {
    let row = &g[GATE + u * GATE_IN..GATE + (u + 1) * GATE_IN];
    let mut y = row[GATE_IN - 1];
    for (w, x) in row[..CONTEXT].iter().zip(ctx) {
        y += w * x;
    }
    for (w, x) in row[CONTEXT..CONTEXT + MEMORY].iter().zip(mem) {
        y += w * x;
    }
    for (w, x) in g[reach_gate_row(u)].iter().zip(last_reach) {
        if *w != 0.0 {
            y += w * x;
        }
    }
    (1.0 + y).clamp(0.0, 2.0)
}

/// **The memory a decision writes** ([`MEM`]): each unit from the context, the best and mean option score, the verb
/// carried out, the memory before, and -- where a weight is not zero, so a genome without them writes bit for bit as
/// it did -- what each verb could reach ([`REACH_MEM`]), then blended with its old value at its rate; or, for a unit
/// whose write gate has a weight ([`WRITE`]), moved toward it by that gate's `z`.
fn remembered(g: &[f32], ctx: &[f32; CONTEXT], mem0: &[f32; MEMORY], all_best: f32, all_mean: f32, verb: Verb, reach: &[f32; VERBS]) -> [f32; MEMORY] {
    let mut fresh = [0.0f32; MEMORY];
    for (m, slot) in fresh.iter_mut().enumerate() {
        let row = &g[MEM + m * M_IN..MEM + (m + 1) * M_IN];
        let mut sum =
            row[M_IN - 1] + row[CONTEXT] * all_best + row[CONTEXT + 1] * all_mean + row[CONTEXT + 2 + verb as usize];
        for (w, x) in row[..CONTEXT].iter().zip(ctx) {
            sum += w * x;
        }
        for (w, x) in row[CONTEXT + 2 + VERBS..CONTEXT + 2 + VERBS + MEMORY].iter().zip(mem0) {
            sum += w * x;
        }
        for (w, x) in g[reach_mem_row(m)].iter().zip(reach) {
            if *w != 0.0 {
                sum += w * x;
            }
        }
        let r = rate(g, m);
        let w = &g[write_row(m)];
        *slot = if w.iter().any(|&x| x != 0.0) {
            let mut y = w[W_IN - 1];
            for (k, x) in ctx.iter().chain(mem0).chain(reach).enumerate() {
                y += w[k] * x;
            }
            let z = ((1.0 - r) + y).clamp(0.0, 1.0);
            mem0[m] + z * (squash(sum) - mem0[m])
        } else {
            r * mem0[m] + (1.0 - r) * squash(sum)
        };
    }
    fresh
}

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
    // Each gated lane's gate, once a decision ([`GATE`]); a genome with no gate weight takes the sum as it always did.
    let gates: Option<[f32; UNITS]> = (!plan.gated.is_empty()).then(|| {
        let mut gk = [1.0f32; UNITS];
        for &(k, u) in &plan.gated {
            gk[k] = gate(g, u, ctx, &state.mem, &state.reach);
        }
        gk
    });
    for (o, opt) in opts.iter().enumerate() {
        if !on(opt.verb) {
            continue;
        }
        let v = opt.verb as usize;
        debug_assert!(opt.feats.iter().all(|x| x.is_finite()), "a feature that is not finite: {:?}", opt.verb);
        // Every unit's sum at once, each over its terms in their order
        // ([`ScorerPlan`]).
        let z = if let Some(gk) = &gates {
            // A gated genome: each lane's feature sum on its own, scaled by its gate, then added to its
            // per-decision part (an ungated lane's gate is 1).
            let f = score_lanes::<UNITS>(&[0.0; UNITS], &plan.feats, &opt.feats);
            std::array::from_fn(|k| start[v][k] + gk[k] * f[k])
        } else if narrow {
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
        // SCRATCH (dig-scoring thread): the dig head's "when" read-out (`scratch_log::DigHead::e`), Dig's job only.
        if v == Verb::Dig as usize && scratch_log::dighead_on() {
            sum += scratch_log::dig_when();
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
    // SCRATCH (dig-scoring thread): with `PP_DIGHEAD`, Dig's own feature head picks the cell, and nothing else
    // reads it: the scores (each verb's best, the memory's best and mean) stay the shared scorer's. **Why it replaces
    // rather than adds**: hcs scores every Dig option at 0.97-0.99, so its cell choice is at chance (P(the old ant's
    // cell) 0.37 against 0.39 uniform), and a head added before the squash could only flatten it further -- the
    // first two trainings did exactly that (every output weight ~4.4, P 0.485 -> 0.489).
    let head = verb == Verb::Dig && scratch_log::dighead_on();
    for o in (0..opts.len()).filter(|&o| opts[o].verb == verb) {
        // The head re-ranks the cuts the shared scorer accepts (its weight times the scorer's), never overrides it:
        // a head that alone picked the cell (dh3) cut the hanging bites hcs avoids -- the old ant takes one in a
        // quarter of its cuts -- and buried its diggers, who sank to the floor and starved (8.5 of 24 against 1).
        let w = if head { (scores[o] + 1.0) / 2.0 * (squash(scratch_log::dig_bonus(opts[o].target)) + 1.0) / 2.0 } else { (scores[o] + 1.0) / 2.0 };
        (targets[n], shifted[n]) = (o, w);
        n += 1;
    }
    // The recall head (`recall`): a taught ant's remembered place tilts which way it walks, after the verb is drawn,
    // so it never changes whether the ant walks. Unset (or a list longer than the stack, which nothing builds): the
    // pick below is untouched, and the draw is consumed exactly once either way.
    let u = draw.unit_f32();
    let mut orig = [0.0f32; STACK_OPTS];
    let tilted = match super::recall::TILT.with(|t| t.get()) {
        Some(gain) if verb == Verb::Walk && n <= STACK_OPTS => {
            orig[..n].copy_from_slice(&shifted[..n]);
            let moved = super::recall::tilt(&mut shifted[..n], &orig[..n], |i| opts[targets[i]].feats[options::f::FOOD_WAY], gain, PICK_FLOOR);
            Some(moved)
        }
        _ => None,
    };
    let pick_i = choose_weighted(&shifted[..n], PICK_FLOOR, u);
    if let Some(moved) = tilted {
        let flipped = choose_weighted(&orig[..n], PICK_FLOOR, u) != pick_i;
        super::recall::MOVED.with(|m| m.set(Some((moved, flipped))));
    }
    let pick = targets[pick_i];

    // Memory, from what the ant felt, saw, could have done and chose.
    let reach = reach(&best);
    state.mem = remembered(g, ctx, &state.mem, all_best, all_mean, verb, &reach);
    state.reach = reach;
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
    let brain_tax = def.synapse_fraction * super::link_price() * def.start_energy * live as f32;
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
    // A sighted mind's eye (`options::sight_on`), billed as `creature_tick` bills every eye: a cell read costs
    // `sight_fraction x start_energy`. Every other mind casts nothing and pays 0.
    let sight_tax = if options::sight_on() && def.sight_range > 0 {
        let (seen, reads) = world.organism(organism).map_or((None, 0), |st| options::sensed_home(world, organism, def, st));
        if reads > 0 {
            options::SIGHT_CASTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            options::SIGHT_SEEN.fetch_add(u64::from(seen.is_some()), std::sync::atomic::Ordering::Relaxed);
        }
        def.sight_fraction * def.start_energy * reads as f32
    } else {
        0.0
    };
    let mut spent = idle + brain_tax + sight_tax + force_tax + armour_tax + exposure;
    world.book(
        colony,
        Account::Metabolized,
        (idle + sight_tax + force_tax + armour_tax + exposure) as f64,
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
        // **The player's body, or a body a module leads** (`steer::take_forced`):
        // read before the closure, which cannot borrow the world mutably. `None`
        // with nobody steered and no entry, every tick in the shipped game.
        let forced = crate::sim::steer::take_forced(world, organism);
        let decided = SCRATCH.with(|s| {
            let mut opts = s.borrow_mut();
            options::build_with(
                world,
                organism,
                def,
                mind.genome.w[SWITCH + Verb::Dig as usize] > 0.0,
                &mut opts,
            );
            // **An ant held on a visitor takes the one option the ladder
            // left it** (`guest::force_options`): a Rest while it inspects,
            // one Walk away after a display. Cut here, before the context
            // and the scoring, so the draw, the bills, the memory and the
            // reschedule all run as for any decision. One `Option` read off.
            if world.visitors.is_some() {
                super::guest::force_options(world, organism, &mut opts);
            }
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
            // SCRATCH (starvation-trace thread, 2026-10-22): `PP_FAR_CTX=R` puts "more than R columns from home" on the
            // borrowed context slot c::D_ALARM (9), which reads 0 on every bed decision (alarm is never laid). The
            // come-home copies hm1/hm2 leaked into foraging, guessed because c::HOME_DIST = squash(dist/32) reads 0.56 at
            // 40 columns and 0.65 at 60: too flat to separate "far" from "in the food band".
            if let Some(r) = scratch_log::far_ctx() {
                if let Some(st) = world.organism(organism) {
                    let ax = st.forage_anchor.0;
                    let nx = world.nest_sites.iter().min_by_key(|s| (s.x - ax).abs()).map_or(ax, |s| s.x);
                    ctx[c::D_SCENT + 2] = f32::from(u8::from((x - nx).abs() > r));
                }
            }
            if scratch_log::digfeat_on() {
                scratch_log::dig_prepare(world, organism, (x, y), &opts, mind.genome.head.as_deref().or(scratch_log::dighead()));
            }
            let mut draw = rng::stream(world.seed, organism as u64, world.frame, RNG_SLOT_MIND);
            let (mem0, last0) = (mind.state.mem, mind.state.last_verb);
            let tilt = super::recall::tilt_for(world, organism);
            super::recall::TILT.with(|t| t.set(tilt));
            let mut d = decide_planned(&mind.genome.w, &mind.genome.plan, &ctx, &opts, &mut mind.state, &mut draw);
            super::recall::TILT.with(|t| t.set(None));
            if tilt.is_some() {
                if let Some(r) = world.recall.as_mut() {
                    r.counts.offered += 1;
                }
            }
            if let Some((moved, flipped)) = super::recall::MOVED.with(|m| m.take()) {
                super::recall::note_tilt(world, moved, flipped);
            }
            if let Some(d) = d.as_ref() {
                scratch_log::decision(world.seed, world.frame, organism, (x, y), &ctx, &mem0, last0, &opts, d);
                // SCRATCH (foraging-loop thread): the carry-home teacher's label rides the copy log as a 16th field.
                if scratch_log::copy_on() && scratch_log::carry_act().is_some() {
                    let lab = scratch_log::carry_label(world, organism, def, (x, y), &ctx, &opts);
                    scratch_log::CARRY_LAB.with(|c| *c.borrow_mut() = if lab.is_empty() { "-".into() } else { format!("1|{}", lab.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(";")) });
                }
                if scratch_log::oldlab_on() {
                    scratch_log::oldlab_label(world, organism, (x, y), &opts);
                }
                scratch_log::copy_decision(world, organism, (x, y), &ctx, &mem0, last0, &opts, d);
            }
            // SCRATCH (dig-scoring thread): `PP_OLDLAB_ACT`, the old ant's calls acting (`scratch_log::oldlab_act`).
            if scratch_log::oldlab_act() {
                if let Some((cut, lab)) = scratch_log::OLDLAB_LAST.with(|o| o.borrow_mut().take()) {
                    let carrying = world.organism(organism).is_some_and(|s| s.spoil.is_some());
                    if let Some((pick, scores0)) = d.as_ref().map(|d| (d.pick, d.scores.clone())) {
                        let choose = if scratch_log::oldlab_cell_only() {
                            (cut == 1 && !lab.is_empty() && opts[pick].verb == Verb::Dig && opts[lab[0]].verb == Verb::Dig).then(|| (0, lab[0]))
                        } else if cut == 1 && !lab.is_empty() {
                            Some((0, lab[0]))
                        } else if !carrying && opts[pick].verb == Verb::Dig && !lab.is_empty() {
                            Some((1, lab[0]))
                        } else if scratch_log::oldlab_act_all()
                            && !(carrying && scratch_log::oldlab_empty_only())
                            && !lab.is_empty()
                            && !matches!(opts[lab[0]].verb, Verb::PickUp | Verb::Eat | Verb::Feed)
                        {
                            Some((2, lab[0]))
                        } else {
                            None
                        };
                        if let Some((k, i)) = choose.filter(|&(_, i)| i != pick) {
                            scratch_log::OLDLAB_ACTS[k].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            if let Some(d) = d.as_mut() {
                                d.pick = i;
                            }
                            mind.state.mem = scratch_log::remember(&mind.genome.w, &ctx, &mem0, &opts, &scores0, opts[i].verb);
                            mind.state.last_verb = Some(opts[i].verb);
                        }
                    }
                }
            }
            // SCRATCH (maze thread, 2026-10-22; ported by the far-from-home sense thread): `PP_TRAIL_ACT=1` (with
            // `PP_TRAIL_LABEL`) lets the recruitment-trail teacher act: when the brain chose to walk and the label
            // applies, the step is the teacher's instead. The verb is the brain's, so memory stays consistent.
            if let Some(d) = d.as_mut() {
                if scratch_log::trail_act() && opts[d.pick].verb == Verb::Walk {
                    if let Some(up) = scratch_log::trail_up(world, organism, &ctx, &opts) {
                        if !up.contains(&d.pick) {
                            d.pick = up[0];
                        }
                    }
                }
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
                                if k.is_multiple_of(1000) {
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
            // SCRATCH (foraging-loop thread): `PP_CARRY=q`, the carry-home teacher (`scratch_log::carry_label`) acting
            // on a share q of the decisions it labels.
            if redrawn.is_none() && scratch_log::carry_act().is_some() {
                if let Some((pick, scores0)) = d.as_ref().map(|d| (d.pick, d.scores.clone())) {
                    if let Some(i) = scratch_log::carry_out(world, organism, def, (x, y), &ctx, &opts, pick) {
                        if let Some(d) = d.as_mut() {
                            d.pick = i;
                        }
                        mind.state.mem = scratch_log::remember(&mind.genome.w, &ctx, &mem0, &opts, &scores0, opts[i].verb);
                        mind.state.last_verb = Some(opts[i].verb);
                    }
                }
            }
            // **The forced pick replaces the mind's** (`steer::ForcedPick`), last, so
            // no teacher above can undo the player. The memory and the pull
            // (`last_verb`) then follow the verb actually carried out, as
            // `PP_OLDLAB_ACT`'s override does, so a body handed back resumes a mind
            // whose state matches what its body did (scout seat's ask: a tandem
            // follower is led for hundreds of decisions). A mind with every verb
            // off still obeys: the decision is built here with no scores.
            if let Some(f) = forced {
                let list: &[Opt] = redrawn.as_deref().unwrap_or(&opts[..]);
                let head = world.organism(organism).and_then(|s| s.chain.first().copied()).unwrap_or((x, y));
                if let Some(i) = crate::sim::steer::choose(list, head, f, |v| mind.genome.w[SWITCH + v as usize] > 0.0) {
                    match d.as_mut() {
                        Some(d) => {
                            if redrawn.is_none() {
                                mind.state.mem = remember_done(&mind.genome.w, &ctx, &mem0, &opts, &d.scores, list[i].verb);
                            }
                            d.pick = i;
                        }
                        None => {
                            d = Some(Decision {
                                pick: i,
                                scores: vec![0.0; list.len()],
                                jobs: [None; VERBS],
                                emits: [0.0; EMITS],
                            })
                        }
                    }
                    mind.state.last_verb = Some(list[i].verb);
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
            // **What the player's key did** (`steer::Steered::last_act`), for a seat
            // to say why a key changed nothing. Only for a forced decision.
            if let Some(f) = forced {
                let frame = world.frame;
                if let Some(s) = world.steered.as_mut().filter(|s| s.id == organism) {
                    s.last_act = Some(crate::sim::steer::LastAct { frame, asked: f.verb, done: opt.verb, ok: did });
                }
            }
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
            // **A bite is jaw work too**, a cut's price per bite, as the
            // game bills a gnaw (`creature_tick`'s jaw bill).
            if did && opt.verb == Verb::Attack && def.dig_cost_in_moves > 0.0 {
                let work = def.move_cost_per_cell * body_cells * def.dig_cost_in_moves;
                spent += work;
                world.book(colony, Account::Metabolized, work as f64);
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
        Verb::Attack => options::attack_on() && attack(world, organism, def, opt.target),
    }
}

/// **Bite the stranger at `t`**, by the game's own bite (`act`'s fight and
/// its ingest, lone predator thread, 2026-10-23; `options::attack_on`): the
/// jaw's force against the cell's plate (`contest::bite_progress`) wears the
/// victim down, and when a whole cell is worn through it comes off. If this
/// gut can use it, the jaws are free and the crop has room for it, it goes
/// into the crop at face value, as the ingest takes a living cell; else it is
/// left empty, as the fight verb leaves it. Either way `reconcile_chain`
/// tells the victim: the head taken is a death (the rest is corpse), a
/// segment is an injury. Alarm is cried where the bite lands, as every bite
/// cries it.
///
/// **None of the game's fight rules**: no urge, no assessment of the odds
/// (`contest::commitment`), no stranger hold. Whether to bite is the mind's.
/// The caller bills the jaw work, a cut's price per bite, as the game bills
/// a gnaw.
fn attack(world: &mut World, organism: OrganismId, def: &CreatureDef, (tx, ty): (i32, i32)) -> bool {
    let Some((colony, jaws_free, crop, (hx, hy))) = world
        .organism(organism)
        .and_then(|s| s.chain.first().map(|&h| (s.colony, s.spoil.is_none(), s.crop, h)))
    else {
        return false;
    };
    let cell = world.get(tx, ty);
    let victim = cell.organism_id();
    if victim == 0 || victim == organism || !super::is_animal_cell(world, cell) {
        return false;
    }
    let Some(their_colony) = world.organism(victim).filter(|s| !s.chain.is_empty()).map(|s| s.colony) else {
        return false;
    };
    if their_colony == colony {
        return false;
    }
    // A sated predator does not kill (`CreatureDef::handling`): no room in the crop for this cell, no bite.
    // Unless it is a seat's player body that may bite full (`steer::bites_when_full`): then the cell
    // comes off unswallowed (`room` below is still false).
    if super::handling_on(def) && !crate::sim::steer::bites_when_full(world, organism) && !crop.is_none_or(|c| c.material == cell.material && c.worth() + c.unit <= organism_crop_capacity(world, organism, def)) {
        return false;
    }
    let gut = super::gut_of(world, organism, def);
    let damage = crate::sim::contest::bite_progress(gut.bite, super::armour_at(world, cell)) * bite_scale();
    world.creature_stats.contests += 1;
    if damage <= 0.0 {
        return false;
    }
    world.note_encounter(crate::sim::world::ENCOUNTER_BITE, organism, victim);
    super::cry_alarm(world, tx, ty);
    world.creature_stats.alarm_attack += 1;
    world.creature_stats.attacks += 1;
    let done = world.organism(victim).is_some_and(|st| st.gnawed + damage >= 1.0);
    if let Some(st) = world.organism_mut(victim) {
        st.gnawed = if done { 0.0 } else { st.gnawed + damage };
    }
    if let Some(m) = world.minds.as_mut() {
        m.counts.bites += 1;
    }
    if !done {
        return true;
    }
    // Who the victim was, read before the cell comes off: `reconcile_chain`
    // frees the slot when the cell was the deciding one.
    let victim_group = world.organism(victim).map(|s| (s.species, s.colony, s.energy));
    let victim_side = super::kill_side(world, victim);
    let worth = food_value(world, cell);
    let room = crop.is_none_or(|c| c.material == cell.material && c.worth() + c.unit <= organism_crop_capacity(world, organism, def));
    let swallow = jaws_free && room && super::diet_yield(world, cell, gut.bias) > super::EAT_YIELD_THRESHOLD;
    if swallow {
        world.loss_context = Some(DeathCause::Eaten);
    }
    world.set(tx, ty, Cell::EMPTY);
    world.loss_context = None;
    world.creature_stats.attack_cells += 1;
    if swallow {
        world.book_raid(colony, their_colony, worth as f64);
    }
    if !super::reconcile_chain(world, victim) {
        world.creature_stats.attack_kills += 1;
        if let (Some(v), Some(me)) = (victim_group, world.organism(organism).map(|s| (s.species, s.colony))) {
            let verb = if swallow { crate::sim::world::KILL_VERB_EAT } else { crate::sim::world::KILL_VERB_BITE };
            let detail = super::kill_detail(world, verb, organism, victim, victim_side);
            world.tally_kill_detailed((v.0, v.1), me, v.2, detail);
        }
        if let Some(m) = world.minds.as_mut() {
            m.counts.kills += 1;
        }
    }
    if swallow {
        crop_take(world, organism, cell.material, cell.shade, worth, None);
        let at_home = food_at_home(world, organism, def, hx, hy);
        super::note_food(world, organism, (tx, ty), super::FOOD_SWALLOW, worth, at_home);
        if let Some(state) = world.organism_mut(organism) {
            state.life.bites += 1;
        }
        world.creature_stats.pickups += 1;
        if let Some(m) = world.minds.as_mut() {
            m.counts.swallowed += 1;
            m.counts.flesh += 1;
        }
    }
    true
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
        || (world.materials.known(Known::Corpse) == Some(food) && !options::corpse_on())
        || (bite.organism_id() != 0 && !scratch_log::grazeable(world, bite))
        || crop.is_some_and(|c| c.material != food || c.worth() + c.unit > organism_crop_capacity(world, organism, def))
    {
        return false;
    }
    let at_home = food_at_home(world, organism, def, hx, hy);
    let frame = world.frame;
    let seed = plant::seed_survives_bite(world, fx, fy, draw);
    let worth = match seed {
        plant::SeedBite::SurvivedBare => food_value(world, bite) * plant::seed_provision_fraction(world, fx, fy),
        _ => food_value(world, bite),
    };
    if !seed.survived() {
        // As the game's bite books it (`act`): a living plant cell leaving the world by a mouth.
        if bite.organism_id() != 0 && world.materials.kind(food) == crate::sim::material::MaterialKind::Plant {
            world.creature_stats.eaten_plant_cells += 1;
        }
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
            // The last-food memory (`options::last_food`): kept whether or not
            // the memory is read, so turning it off changes no choice.
            state.last_food = Some((fx, fy, frame));
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

/// SCRATCH (colony-vs-colony thread, 2026-10-24): `PP_MIND_BITE_SCALE=f` scales what one mind bite takes off its
/// victim (`OrganismState::gnawed`, banked on the victim, so several mouths add up). Ant on ant the ratio is 1, so
/// one bite is one cell and the head is a death: on `examples/rivalbed.rs` 274 of 397 bites killed, and one ant
/// that could not be bitten back killed 25 in a nest in 700 frames. Below 1 a kill takes several bites, from one
/// mouth or many -- the swarm the game's own fight banks damage for. Unset: 1, the bite as it was.
fn bite_scale() -> f32 {
    static K: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *K.get_or_init(|| std::env::var("PP_MIND_BITE_SCALE").map_or(1.0, |v| v.parse().expect("PP_MIND_BITE_SCALE=f")))
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
        // The recall head's meal copy: the fed nestmate may take the giver's remembered place, weaker.
        if world.recall.is_some() {
            super::recall::share_on_feed(world, organism, other);
        }
        if laygene_on() {
            note_fed(world, other, amount);
        }
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
    // **A Walk into living wood is the trunk crossing** (`options::cross_on`, which is the only way one is offered),
    // started exactly as the game's ant starts one in `step_chain`: `creature_tick` runs it out before the mind decides
    // again. `true`: the body acted, though the head moves only when the crossing is due.
    let cell = world.get(target.0, target.1);
    if options::cross_on() && super::is_living_tissue(world, cell) && world.materials.get(cell.material).woody {
        let Some((out, thickness)) = super::trunk_crossing(world, def, body, to as u8, parting_enabled(), super::stacker_of(world, organism)) else {
            return false;
        };
        let due = world.frame + u64::from(thickness) * super::organism_tick_interval(world, organism, def);
        if let Some(state) = world.organism_mut(organism) {
            state.crossing = Some(organism::Crossing { to: out, heading: to as u8, due, thickness });
        }
        world.creature_stats.crossings += 1;
        return true;
    }
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
        if amount > 0 && !options::scent_silent(EMIT_CHANNELS[e]) {
            world.deposit_pheromone(EMIT_CHANNELS[e], hx, hy, amount);
            if let Some(m) = world.minds.as_mut() {
                m.counts.laid[e] += f64::from(s);
            }
        }
    }
    if def.emit_cost_in_moves <= 0.0 || options::emit_free() {
        return 0.0;
    }
    let cost = def.move_cost_per_cell * body_cells * def.emit_cost_in_moves * total;
    world.book(colony, Account::Metabolized, cost as f64);
    world.creature_stats.emit_energy += cost as f64;
    cost
}

#[cfg(test)]
mod tests {

    /// SCRATCH (brood lane, 2026-10-24): the lay gate's bar never drops below the body's, doubles per whole step
    /// below 0 and stops at 64; the "fed" sense halves once per `FED_HALF` and never between.
    #[test]
    fn the_lay_gate_only_raises_the_bar_and_fed_halves_by_steps() {
        assert_eq!(bar_scale(0.0), 1.0);
        assert_eq!(bar_scale(3.0), 1.0);
        assert_eq!(bar_scale(-1.0), 2.0);
        assert_eq!(bar_scale(-2.5), 6.0);
        assert_eq!(bar_scale(-40.0), 64.0);
        assert_eq!(fed_now((800.0, 100), 100 + FED_HALF - 1), 800.0);
        assert_eq!(fed_now((800.0, 100), 100 + 2 * FED_HALF), 200.0);
    }
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
    /// **Every shipped pool decides bit for bit as it did before the context gates** (layout 6, [`GATE`]): the
    /// standard pool, the spider pool and the centipede pool, each member over 300 random decisions (context, memory
    /// and 1-20 options drawn from one stream), hashed over every score, job, emit, memory unit and pick. The hash was
    /// taken on `main` at 4a66f56, the build before the gates; a pool that gained them at zero must give it again.
    /// **Put the fault back**: scoring every genome through the gated path (`gates` always `Some`, every gate 1)
    /// moves the hash, because a lane's feature sum is then rounded on its own before its per-decision part is added.
    #[test]
    fn shipped_pools_decide_as_they_did_before_the_gates() {
        let mut minds: Vec<Vec<f32>> = crate::lab::training::std_pool().into_iter().map(|g| g[..GENOME_LEN].to_vec()).collect();
        minds.extend(crate::lab::training::spider_minds().iter().map(|m| m.w.clone()));
        minds.extend(crate::lab::training::centipede_minds().iter().map(|m| m.w.clone()));
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |x: u64| {
            h ^= x;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        };
        let mut decided = 0;
        for (m, g) in minds.iter().enumerate() {
            let mut r = rng::stream(17, m as u64, 0, 7);
            for trial in 0..300u64 {
                let ctx: [f32; CONTEXT] = std::array::from_fn(|_| r.unit_f32() * 2.0 - 1.0);
                let mut state = MindState::default();
                for x in state.mem.iter_mut() {
                    *x = r.unit_f32() * 2.0 - 1.0;
                }
                let opts: Vec<Opt> = (0..1 + (r.unit_f32() * 20.0) as usize)
                    .map(|k| Opt {
                        verb: Verb::ALL[(r.unit_f32() * VERBS as f32) as usize % VERBS],
                        target: (k as i32, 0),
                        feats: std::array::from_fn(|_| if r.unit_f32() < 0.5 { 0.0 } else { r.unit_f32() * 2.0 - 1.0 }),
                    })
                    .collect();
                let mut draw = rng::stream(5, m as u64, trial, RNG_SLOT_MIND);
                let Some(d) = decide(g, &ctx, &opts, &mut state, &mut draw) else {
                    eat(u64::MAX);
                    continue;
                };
                decided += 1;
                eat(d.pick as u64);
                d.scores.iter().for_each(|s| eat(s.to_bits() as u64));
                d.jobs.iter().for_each(|j| eat(j.map_or(7, |j| j.to_bits() as u64)));
                d.emits.iter().for_each(|e| eat(e.to_bits() as u64));
                state.mem.iter().for_each(|x| eat(x.to_bits() as u64));
            }
        }
        assert!(decided > minds.len() * 200, "too few decisions: {decided} over {} minds", minds.len());
        println!("minds {} decided {decided} hash {h:#018x}", minds.len());
        assert_eq!(h, GOLDEN_PRE_GATE, "a shipped pool decides differently from the build before the context gates");
    }

    /// [`shipped_pools_decide_as_they_did_before_the_gates`]'s hash on `main` at 4a66f56, before the gates.
    const GOLDEN_PRE_GATE: u64 = 0x318b_65b1_5e53_a2b8;

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
            // Half the genomes ungated, as every genome saved before the gates is ([`GATE`]): both sums are held.
            if trial % 2 == 0 {
                g[GATE..GENOME_LEN].fill(0.0);
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
        (0..GENOME_LEN_V5).filter(|k| !old.contains(k)).collect()
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

    /// **A genome saved before the context gates reads into today's layout** ([`enter_gate`]): the gate rows enter
    /// at zero between its core and its riders, so every core weight and every rider keeps its meaning; a bare core
    /// pads the same way ([`pad`]); a file marked layout 6 is left as it is, and one marked 6 at an old length is
    /// refused. Then the gate itself: shut (`y = -1`) its unit adds exactly nothing but its per-decision part, as if
    /// its feature row were empty; half open, the scores move (the positive control).
    #[test]
    fn a_genome_from_before_the_gates_gains_them_at_zero_and_a_shut_gate_silences_its_unit() {
        let old: Vec<f32> = (0..GENOME_LEN_V5 + 482 + LAY_LEN).map(|i| (i % 97) as f32 + 1.0).collect();
        let mut g = old.clone();
        assert!(enter_gate(&mut g, Some(5)));
        assert_eq!(g.len(), GENOME_LEN + 482 + LAY_LEN);
        assert_eq!(&g[..GATE], &old[..GENOME_LEN_V5], "the core keeps every slot");
        assert!(g[GATE..GENOME_LEN].iter().all(|w| *w == 0.0), "the gates enter at zero");
        assert_eq!(&g[GENOME_LEN..], &old[GENOME_LEN_V5..], "the riders keep their order");
        let mut unmarked = old.clone();
        assert!(enter_gate(&mut unmarked, None));
        assert_eq!(unmarked, g, "an unmarked file is told by its length");
        let mut h = g.clone();
        assert!(!enter_gate(&mut h, Some(LAYOUT)), "a genome of this layout is not padded again");
        assert_eq!(h, g);
        assert_eq!(pad(old[..GENOME_LEN_V5].to_vec()).as_deref(), Some(&g[..GENOME_LEN]));
        assert!(std::panic::catch_unwind(|| enter_gate(&mut old[..GENOME_LEN_V5].to_vec(), Some(6))).is_err());

        // One live unit, so its lane is the only one the gate can move.
        let mut g = blank(&[Verb::Walk, Verb::Rest]);
        for f in 0..FEATURES {
            g[S_FEAT + f] = ((f * 31 % 17) as f32 / 8.0) - 1.0;
        }
        // `walks` differ only in this feature.
        g[S_FEAT + options::f::SCENT + 4] = 1.5;
        g[S_CTX] = 0.3;
        g[S_OUT] = 2.0;
        let mut empty = g.clone();
        empty[S_FEAT..S_FEAT + FEATURES].fill(0.0);
        let ctx: [f32; CONTEXT] = std::array::from_fn(|i| i as f32 / CONTEXT as f32);
        let scores = |g: &[f32]| {
            let mut state = MindState::default();
            let mut draw = rng::stream(1, 2, 3, RNG_SLOT_MIND);
            decide(g, &ctx, &walks(), &mut state, &mut draw).expect("walks decide").scores
        };
        let mut shut = g.clone();
        shut[GATE + GATE_IN - 1] = -1.0;
        assert_eq!(scores(&shut), scores(&empty), "a shut gate leaves its unit's per-decision part alone");
        let mut half = g.clone();
        half[GATE + GATE_IN - 1] = -0.5;
        assert_ne!(scores(&half), scores(&g), "a half-open gate moves the scores");
        assert_ne!(scores(&half), scores(&empty));
        assert_eq!(MindGenome::new(half).live, MindGenome::new(g).live + 1, "a gate weight is a link, billed");
    }

    /// **Layout 8: a memory unit's write gate** ([`WRITE`]). A layout-7 file gains the rows at zero between its core
    /// and its riders. Shut (`z = 0`) the unit holds its value exactly whatever it is shown; fully open (`z = 1`) it
    /// takes its new value in one decision; with the gate opened only by `c::AT_NEST`, it writes at the nest and holds
    /// away from it -- what a rate gene alone cannot do (the positive control: the same unit without the gate drifts).
    #[test]
    fn a_write_gate_holds_a_memory_until_it_opens() {
        let old: Vec<f32> = (0..GENOME_LEN_V7 + 482 + LAY_LEN).map(|i| (i % 83) as f32 + 1.0).collect();
        let mut g = old.clone();
        assert!(enter_gate(&mut g, Some(7)));
        assert_eq!(g.len(), GENOME_LEN + 482 + LAY_LEN);
        assert_eq!(&g[..WRITE], &old[..GENOME_LEN_V7]);
        assert!(g[WRITE..GENOME_LEN].iter().all(|w| *w == 0.0), "the write gates enter at zero");
        assert_eq!(&g[GENOME_LEN..], &old[GENOME_LEN_V7..]);
        assert_eq!(pad(old[..GENOME_LEN_V7].to_vec()).as_deref(), Some(&g[..GENOME_LEN]));
        let mut bare = old[..GENOME_LEN_V7].to_vec();
        assert!(enter_gate(&mut bare, None), "an unmarked layout-7 core is told by its length");

        // Memory unit 5 writes +1 at the nest (bias 20 through squash) and -1 away from it.
        let mut g = blank(&[Verb::Walk, Verb::Rest]);
        let row = MEM + 5 * M_IN;
        g[row + c::AT_NEST] = 40.0;
        g[row + M_IN - 1] = -20.0;
        g[RATE + 5] = 0.495; // rate 0.995: on its own the unit barely moves
        let step = |g: &[f32], at_nest: bool, mem: f32| {
            let mut ctx = [0.0f32; CONTEXT];
            ctx[c::AT_NEST] = if at_nest { 1.0 } else { 0.0 };
            let mut state = MindState::default();
            state.mem[5] = mem;
            let mut draw = rng::stream(1, 2, 3, RNG_SLOT_MIND);
            decide(g, &ctx, &walks(), &mut state, &mut draw).expect("walks decide");
            state.mem[5]
        };
        let mut gated = g.clone();
        gated[write_row(5).start + c::AT_NEST] = 1.0; // z = 0.005 + 1 at the nest -> 1
        gated[write_row(5).end - 1] = -0.005; // z = 0 away from it
        let nest = step(&gated, true, -0.9);
        assert!(nest > 0.9, "an open gate writes in one decision: {nest}");
        assert_eq!(step(&gated, false, nest).to_bits(), nest.to_bits(), "a shut gate holds exactly");
        assert!(step(&g, true, -0.9) < -0.8, "without the gate the unit barely writes at the nest");
        assert!(step(&g, false, 0.95) < 0.95, "...and without it, it drifts away from the nest");
        assert_eq!(MindGenome::new(gated).live, MindGenome::new(g).live + 2, "a write weight is a link, billed");
    }

    /// **Layout 7: what each verb could reach, into memory and the gates** ([`REACH_MEM`], [`REACH_GATE`]). A layout-6
    /// file gains both blocks at zero between its core and its riders, marked or (bare) unmarked, and one marked 7 at
    /// a layout-6 length is refused. Then the inputs: memory reads this decision's reach -- 0.25 to 1 by best target, 0 for a
    /// verb with no target -- and the gate the last decision's, so the first decision of a fresh ant scores as if the
    /// row were empty and the second does not (the positive controls).
    #[test]
    fn a_layout_6_genome_gains_the_reach_rows_at_zero_and_memory_reads_what_each_verb_could_reach() {
        let old: Vec<f32> = (0..GENOME_LEN_V6 + 482 + LAY_LEN).map(|i| (i % 89) as f32 + 1.0).collect();
        let mut g = old.clone();
        assert!(enter_gate(&mut g, Some(6)));
        assert_eq!(g.len(), GENOME_LEN + 482 + LAY_LEN);
        assert_eq!(&g[..REACH_MEM], &old[..GENOME_LEN_V6], "the core keeps every slot");
        assert!(g[REACH_MEM..GENOME_LEN].iter().all(|w| *w == 0.0), "the reach rows enter at zero");
        assert_eq!(&g[GENOME_LEN..], &old[GENOME_LEN_V6..], "the riders keep their order");
        assert_eq!(pad(old[..GENOME_LEN_V6].to_vec()).as_deref(), Some(&g[..GENOME_LEN]), "a bare layout-6 core pads");
        let mut lay = old[..GENOME_LEN_V6 + LAY_LEN].to_vec();
        assert!(enter_gate(&mut lay, None), "an unmarked layout-6 core with its lay gate is told by its length");
        assert_eq!(&lay[GENOME_LEN..], &old[GENOME_LEN_V6..GENOME_LEN_V6 + LAY_LEN]);
        assert!(std::panic::catch_unwind(|| enter_gate(&mut old[..GENOME_LEN_V6].to_vec(), Some(7))).is_err());
        assert!(g[GENOME_LEN_V7..GENOME_LEN].iter().all(|w| *w == 0.0), "...and the write gates after them");

        let mut g = blank(&[Verb::Walk, Verb::Rest]);
        for f in 0..FEATURES {
            g[S_FEAT + f] = ((f * 31 % 17) as f32 / 8.0) - 1.0;
        }
        g[S_FEAT + options::f::SCENT + 4] = 1.5;
        g[S_OUT] = 2.0;
        let ctx: [f32; CONTEXT] = std::array::from_fn(|i| i as f32 / CONTEXT as f32);
        let run = |g: &[f32], state: &mut MindState| {
            let mut draw = rng::stream(1, 2, 3, RNG_SLOT_MIND);
            decide(g, &ctx, &walks(), state, &mut draw).expect("walks decide")
        };
        let mut state = MindState::default();
        let d = run(&g, &mut state);
        let on = |s: f32| REACH_FLOOR + (1.0 - REACH_FLOOR) * (s + 1.0) / 2.0;
        assert_eq!(state.reach[Verb::Walk as usize], on(d.scores[0].max(d.scores[1])), "its best target, over the floor");
        assert_eq!(state.reach[Verb::Rest as usize], on(d.scores[3]));
        assert_eq!(reach(&std::array::from_fn(|v| if v == 0 { -1.0 } else { f32::NEG_INFINITY }))[0], REACH_FLOOR, "a target scored -1 is still there");
        assert_eq!(state.reach[Verb::TurnAround as usize], 0.0, "a verb switched off reaches nothing");
        assert_eq!(state.reach[Verb::Dig as usize], 0.0, "nor does one with no target");
        // Memory: unit 3 weighs what Walk could reach.
        let mut m = g.clone();
        m[reach_mem_row(3).start + Verb::Walk as usize] = 2.0;
        let mut ms = MindState::default();
        run(&m, &mut ms);
        assert_ne!(ms.mem[3], state.mem[3], "memory reads what a verb could reach");
        assert_eq!(ms.mem[2], state.mem[2]);
        let mut rest = m.clone();
        rest[reach_mem_row(3).start + Verb::Walk as usize] = 0.0;
        rest[reach_mem_row(3).start + Verb::Dig as usize] = 2.0;
        let mut rs = MindState::default();
        run(&rest, &mut rs);
        assert_eq!(rs.mem[3].to_bits(), state.mem[3].to_bits(), "a verb with no target adds nothing");
        // The gate: unit 0's reach row on Walk, read from the last decision.
        let mut gated = g.clone();
        gated[reach_gate_row(0).start + Verb::Walk as usize] = -0.5;
        let (mut a, mut b) = (MindState::default(), MindState::default());
        assert_eq!(run(&gated, &mut a).scores, run(&g, &mut b).scores, "a fresh ant's last reach is 0");
        assert_ne!(run(&gated, &mut a).scores, run(&g, &mut b).scores, "the next decision reads it");
        assert_eq!(MindGenome::new(gated).live, MindGenome::new(g).live + 1, "a reach weight is a link, billed");
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
        // Memory 9 is read only by unit 5 (its context row and its gate) and hidden 7, so it goes once they have.
        let m = 9;
        for u in (0..UNITS).filter(|&u| u != 5) {
            g[S_CTX + u * S_IN + CONTEXT + m] = 0.0;
            g[GATE + u * GATE_IN + CONTEXT + m] = 0.0;
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
            g[WRITE + r * W_IN + CONTEXT + m] = 0.0;
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
        // Saved before step 1: Pick up banned, its links drifted; and long before the reach rows, which a file that
        // old enters at zero ([`enter_gate`]) -- so they are not split.
        old[SWITCH + Verb::PickUp as usize] = -1.0;
        old[REACH_MEM..GENOME_LEN].fill(0.0);
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

    /// **The last-food memory reaches the options, graded and fading**
    /// (`options::last_food`): with a spot remembered east of the head, a
    /// step east reads `f::FOOD_WAY` above 0 and a step west below it, by
    /// the same cosine; the reading falls as the memory ages and is exactly
    /// 0 with nothing remembered. The specificity half: an ant that
    /// remembers nothing reads 0 on every option, so a weight on the slot
    /// moves no choice until a pick-up.
    #[test]
    fn the_last_food_memory_points_each_option_back_to_the_food() {
        let fade = options::last_food().expect("the last-food memory is on by default");
        let (mut w, def, ids) = mind_box();
        let id = ids[0];
        let head = head_of(&w, id);
        let read = |w: &World| {
            let mut opts = Vec::new();
            options::build(w, id, &def, &mut opts);
            let walk = |dx: i32| {
                opts.iter()
                    .find(|o| o.verb == Verb::Walk && o.target.0 - head.0 == dx && o.target.1 == head.1)
                    .map(|o| o.feats[options::f::FOOD_WAY])
            };
            (opts.iter().map(|o| o.feats[options::f::FOOD_WAY].abs()).fold(0.0, f32::max), walk(1), walk(-1))
        };
        assert_eq!(read(&w).0, 0.0, "an ant that remembers no food reads the slot");
        w.organism_mut(id).expect("alive").last_food = Some((head.0 + 60, head.1, w.frame));
        let (_, east, west) = read(&w);
        let (east, west) = (east.expect("a step east on flat ground"), west.expect("a step west"));
        let want = (1.0 + 60.0 / options::LAST_FOOD_SCALE).ln();
        assert!((east - want).abs() < 1e-4, "east {east} against ln(1 + 60/40) {want}");
        assert!((west + want).abs() < 1e-4, "west {west} is not the mirror of east");
        // Remembered `fade` frames ago: half as strong.
        w.organism_mut(id).expect("alive").last_food = Some((head.0 + 60, head.1, w.frame.saturating_sub(fade as u64)));
        let old = read(&w).1.expect("a step east");
        if w.frame >= fade as u64 {
            assert!((old - want / 2.0).abs() < 1e-4, "a memory {fade} frames old reads {old}, not half of {want}");
        } else {
            assert!(old < east, "an older memory reads no weaker: {old} against {east}");
        }
    }

    /// **The last-food memory enters an older genome at weight 0, and a new
    /// one keeps what evolution put there** (`enter_last_food`): a file
    /// marked layout 4, or unmarked, loses every scorer unit's weight on
    /// the slot and nothing else; layout 5 is untouched.
    #[test]
    fn the_last_food_slot_is_zeroed_only_in_genomes_saved_before_it() {
        let mut g: Vec<f32> = (0..GENOME_LEN).map(|k| ((k * 37) % 11) as f32 / 10.0 - 0.5).collect();
        let slots: Vec<usize> = (0..UNITS).map(|u| S_FEAT + u * FEATURES + options::f::FOOD_WAY).collect();
        for &k in &slots {
            g[k] = 0.7;
        }
        let mut kept = g.clone();
        assert_eq!(enter_last_food(&mut kept, Some(LAYOUT)), 0);
        assert_eq!(kept, g, "a genome saved with the memory lost links");
        for layout in [Some(4), None] {
            let mut old = g.clone();
            assert_eq!(enter_last_food(&mut old, layout), UNITS, "layout {layout:?}: links cut");
            for k in 0..GENOME_LEN {
                let want = if slots.contains(&k) { 0.0 } else { g[k] };
                assert_eq!(old[k], want, "layout {layout:?}: slot {k} moved");
            }
        }
    }

    /// **The positive and negative control for the memory as something a
    /// brain can use** (Creatures research thread, idea 1; for the teacher
    /// lane): the same walking mind, with and without one scorer unit that
    /// weighs `f::FOOD_WAY`, remembering a spot 40 columns east on flat
    /// ground. Wired, it walks there; with that one link cut it does not --
    /// so a pool that never wires the slot is not evidence the sense is
    /// dead, and one that does has something to find. Prints both, as the
    /// trip-2-against-trip-1 numbers a wired pool should show.
    #[test]
    fn a_mind_wired_to_the_last_food_memory_walks_back_to_it() {
        let run = |wired: bool| -> i32 {
            let (mut w, _def, ids) = mind_box();
            let mut g = blank(&[Verb::Walk]);
            if wired {
                g[S_FEAT + options::f::FOOD_WAY] = 8.0;
                g[S_OUT] = 4.0;
            }
            let g = Arc::new(MindGenome::new(g));
            for &id in &ids {
                give(&mut w, id, g.clone());
            }
            let id = ids[0];
            let start = head_of(&w, id);
            let spot = (start.0 + 40, start.1);
            w.organism_mut(id).expect("alive").last_food = Some((spot.0, spot.1, w.frame));
            for _ in 0..600 {
                crate::sim::update::step(&mut w);
                w.step_active_sites();
                w.step_fields();
                w.step_pheromones();
                if w.organism(id).is_none() {
                    break;
                }
            }
            w.organism(id).and_then(|s| s.chain.first()).map_or(start.0, |h| h.0) - start.0
        };
        let (wired, cut) = (run(true), run(false));
        println!("columns walked towards the remembered food in 600 frames: wired {wired}, link cut {cut}");
        assert!(wired >= 25, "a mind wired to the memory got {wired} of 40 columns towards it");
        assert!(wired - cut >= 15, "wired {wired} against cut {cut}: the memory moved the walk too little");
    }

    /// **The recall head moves a trained ant toward a remembered place, and
    /// only the head does** (`recall`, keeper seat demo). The shipped pool's
    /// weights on the last-food memory are zeroed on load, so a planted memory
    /// alone moves nothing (the gain-0 arm is the control); with the head at a
    /// scripted gain, the same ant on the same seed walks toward the place.
    /// Removing the term from `decide_planned` makes the two arms identical
    /// and this goes red.
    #[test]
    fn the_recall_head_walks_a_trained_ant_toward_a_remembered_place() {
        let pool = crate::lab::training::std_pool();
        let run = |gain: Option<f32>| -> (i32, super::super::recall::RecallCounts) {
            let (mut w, _def, ids) = mind_box();
            let g = Arc::new(MindGenome::new(pool[0].clone()));
            for &id in &ids {
                give(&mut w, id, g.clone());
            }
            w.recall = gain.map(|g| super::super::recall::Recall::new(g, 0.0));
            let id = ids[0];
            let start = head_of(&w, id);
            let spot = (start.0 + 60, start.1);
            super::super::recall::plant(&mut w, id, spot);
            let mut best = 0;
            for _ in 0..1500 {
                crate::sim::update::step(&mut w);
                w.step_active_sites();
                w.step_fields();
                w.step_pheromones();
                match w.organism(id).and_then(|s| s.chain.first().copied()) {
                    Some(h) => best = best.max(h.0 - start.0),
                    None => break,
                }
            }
            (best, w.recall.as_ref().map(|r| r.counts).unwrap_or_default())
        };
        let (off, _) = run(None);
        let (zero, c0) = run(Some(0.0));
        if let Ok(v) = std::env::var("RECALL_SWEEP") {
            for g in v.split(',').map(|x| x.parse::<f32>().expect("gain")) {
                let (d, c) = run(Some(g));
                println!("gain {g}: furthest {d}, tilted {}, mass moved {:.1}", c.tilted, c.mass_moved);
            }
        }
        let (on, c) = run(Some(6.0));
        println!("furthest columns toward the remembered place in 1,500 frames: off {off}, gain 0 {zero}, gain 6 {on}; {c:?}");
        assert_eq!(off, zero, "gain 0 must be today's ant exactly");
        assert_eq!((c0.mass_moved, c0.picks_flipped), (0.0, 0), "gain 0 moved probability: the counter is not a function of the gain");
        assert!(c.tilted > 0 && c.picks_flipped > 0, "the head never fired: {c:?}");
        assert!(on >= 40 && on - zero >= 30, "gain 6 reached {on} of 60 columns against {zero} at gain 0");
    }

    /// **Off is today's ant, and so is gain 0** (plan review M6). On a box of
    /// std_pool ants making real decisions, with one memory planted: no head,
    /// the head at gain 0 with no copy, and the head at gain 0 *with* the meal
    /// copy spreading the place must all end in the same world. That holds
    /// because the lab freezes mutation and `last_food`'s only reader is the
    /// `FOOD_WAY` feature, whose pool weights are zeroed on load; if either
    /// changes, this goes red and the honest arm is no longer honest. The
    /// positive control, gain 2, must differ, or the fingerprint is blind.
    #[test]
    fn the_recall_head_at_gain_zero_is_todays_ant_exactly() {
        let pool = crate::lab::training::std_pool();
        // The fingerprint is every live animal's head, energy and id at every frame, folded: a world that merely
        // converges after the planted ant dies (it does, in this foodless box) must still read as different.
        let run = |recall: Option<(f32, f32)>| -> (u64, u64) {
            use std::hash::{Hash, Hasher};
            let (mut w, _def, ids) = mind_box();
            let g = Arc::new(MindGenome::new(pool[0].clone()));
            for &id in &ids {
                give(&mut w, id, g.clone());
            }
            w.recall = recall.map(|(g, c)| super::super::recall::Recall::new(g, c));
            let start = head_of(&w, ids[0]);
            super::super::recall::plant(&mut w, ids[0], (start.0 + 60, start.1));
            let mut h = std::collections::hash_map::DefaultHasher::new();
            for _ in 0..2000 {
                crate::sim::update::step(&mut w);
                w.step_active_sites();
                w.step_fields();
                w.step_pheromones();
                for id in w.live_organism_ids() {
                    if let Some(s) = w.organism(id) {
                        (id, s.chain.first().copied(), s.energy.to_bits()).hash(&mut h);
                    }
                }
            }
            (h.finish(), w.recall.as_ref().map_or(0, |r| r.counts.copies))
        };
        let (off, _) = run(None);
        let (zero, _) = run(Some((0.0, 0.0)));
        let (zero_copy, copies) = run(Some((0.0, 0.5)));
        let (on, _) = run(Some((2.0, 0.0)));
        assert_eq!(off, zero, "the head at gain 0 changed the world");
        assert_eq!(off, zero_copy, "the meal copy at gain 0 changed the world");
        println!("meal copies made at gain 0: {copies}");
        assert_ne!(off, on, "gain 2 left the world unchanged: the fingerprint cannot see the head");
    }

    /// **A carried mind ant keeps living where it is set down** (keeper seat's
    /// CARRY, `creature::set_down`). A body moved outside its own tick must be
    /// rescheduled at its new head, or it freezes: never ticks, never pays,
    /// reads as healthy. So: it moved there, it owns exactly its new cells,
    /// and over the next 400 frames it keeps deciding (its head moves) and
    /// keeps paying (its energy falls). 400, not more: this box is foodless,
    /// and an uncarried ant here starves at about 680 (measured), so a longer
    /// window tests starvation, not the carry. Removing the reschedule line
    /// in `set_down` turns this red.
    #[test]
    fn a_carried_ant_keeps_living_where_it_is_set_down() {
        let pool = crate::lab::training::std_pool();
        let (mut w, _def, ids) = mind_box();
        let g = Arc::new(MindGenome::new(pool[0].clone()));
        for &id in &ids {
            give(&mut w, id, g.clone());
        }
        let id = ids[0];
        let from = head_of(&w, id);
        let ground = (0..128).find(|&y| !w.is_empty(from.0 + 70, y)).expect("ground");
        let landed = crate::sim::creature::set_down(&mut w, id, (from.0 + 70, ground - 1)).expect("room to set it down");
        assert!((landed.0 - (from.0 + 70)).abs() <= 8, "set down near the target: {landed:?}");
        let owned = |w: &World| -> usize {
            let mut n = 0;
            for y in 0..128 {
                for x in 0..256 {
                    if w.get(x, y).organism_id() == id {
                        n += 1;
                    }
                }
            }
            n
        };
        let chain = w.organism(id).expect("alive").chain.clone();
        assert!(chain.iter().all(|&(x, y)| w.get(x, y).organism_id() == id), "every chain cell carries the ant");
        assert_eq!(owned(&w), chain.len(), "no cell left behind owning the ant");
        let e0 = w.organism(id).expect("alive").energy;
        let mut moved = false;
        for _ in 0..400 {
            crate::sim::update::step(&mut w);
            w.step_active_sites();
            w.step_fields();
            w.step_pheromones();
            let Some(s) = w.organism(id) else { break };
            moved |= s.chain.first() != Some(&landed);
        }
        let s = w.organism(id).expect("a carried ant should not die of the carry within 400 frames");
        assert!(moved, "the carried ant never moved again: frozen, not rescheduled");
        assert!(s.energy < e0, "the carried ant never paid for living: {e0} -> {}", s.energy);
    }

    /// **A treat is a real, guarded meal** (keeper seat's TREAT): it fills the
    /// crop (so the ant is laden and walks home first), is refused for a
    /// different food in the crop, and is booked as granted energy.
    #[test]
    fn a_treat_fills_the_crop_with_the_pick_up_guards() {
        let (mut w, _def, ids) = mind_box();
        let id = ids[0];
        let food = w.materials.id_of("provisions").expect("provisions");
        let other = w.materials.id_of("seed").or_else(|| w.materials.id_of("windfall")).expect("another food");
        assert!(w.organism(id).expect("alive").crop.is_none());
        assert!(crate::sim::creature::treat(&mut w, id, food, 100.0));
        assert!(w.organism(id).and_then(|s| s.crop).is_some_and(|c| c.material == food));
        assert!(!crate::sim::creature::treat(&mut w, id, other, 100.0), "a crop holds one food");
    }

    /// **The meal copy is weaker than what was given**, and only fresher news
    /// replaces what the taker already holds.
    #[test]
    fn a_fed_nestmate_takes_the_place_at_reduced_freshness() {
        let (mut w, _def, ids) = mind_box();
        w.recall = Some(super::super::recall::Recall::new(1.0, 0.5));
        let (a, b, c) = (ids[0], ids[1], ids[2]);
        super::super::recall::plant(&mut w, a, (200, 50));
        super::super::recall::share_on_feed(&mut w, a, b);
        let fb = super::super::recall::knows(&w, b, (200, 50), 0).expect("b heard of it");
        assert!((fb - 0.5).abs() < 0.01, "second-hand reads half: {fb}");
        super::super::recall::share_on_feed(&mut w, b, c);
        let fc = super::super::recall::knows(&w, c, (200, 50), 0).expect("c heard of it");
        assert!((fc - 0.25).abs() < 0.01, "third-hand reads a quarter: {fc}");
        // Staler news does not overwrite fresher.
        super::super::recall::share_on_feed(&mut w, c, a);
        assert!(super::super::recall::knows(&w, a, (200, 50), 0).expect("a still") > 0.99);
        let r = w.recall.as_ref().expect("on");
        assert_eq!(r.counts.copies, 2);
        assert_eq!(r.provenance.get(&c).map(|p| p.generation), Some(2));
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

    /// **A mind ant can carry no caste** (`PP_MIND_CASTELESS`): the game's storeroom stamps one ant in four
    /// nest-bound for life by id, giving a mind keeps it while the switch is off (unset), and the switch's write
    /// clears it. The control is the box before the write, where the stamp is there to clear.
    #[test]
    fn a_mind_ant_carries_no_caste() {
        let mut w = crate::lab::scene::LabBox { founders: 4, colonies: 1, colony_ants: 12, width: 256, height: 128, ground_y: 64, soil_depth: 40, ..crate::lab::scene::LabBox::default() }.build();
        assert_eq!(super::super::storeroom_of(&w).caste, 4, "test setup: the shipped storeroom has its caste");
        let ants: Vec<OrganismId> = w.live_organism_ids().into_iter().filter(|&id| w.organism(id).is_some_and(|s| w.species.get(s.species).creature.is_some())).collect();
        let bound = |w: &World| ants.iter().filter(|&&id| w.organism(id).is_some_and(|s| super::super::is_nest_bound(w, s))).count();
        let stamped = bound(&w);
        assert!(stamped > 0, "the control: no ant was stamped, so there is nothing to clear");
        let g = Arc::new(MindGenome::new(blank(&[Verb::Walk, Verb::Rest])));
        for &id in &ants {
            give(&mut w, id, g.clone());
        }
        if !casteless() {
            assert_eq!(bound(&w), stamped, "off, a mind took the stamp away");
        }
        for &id in &ants {
            clear_caste(&mut w, id);
        }
        assert_eq!(bound(&w), 0, "a mind ant kept its caste");
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
        // A corpse is the bite's mouth (`options::corpse_on`, on with the bite since 2026-10-24): taken under it,
        // refused with it off, as before.
        assert_eq!(
            act(&mut w, id, &def, on(Verb::PickUp, t), &mut draw),
            options::corpse_on(),
            "a dead ant's pick-up did not follow the corpse switch"
        );
        w.organism_mut(id).expect("ant").crop = None;
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

    /// **The bite takes a stranger's cell, and the mouth keeps it** (lone
    /// predator thread, 2026-10-23; [`attack`]): a spider's head beside an
    /// ant wears through the cell it bites in one go (jaw 0.5 against ant
    /// flesh at 0.25), the cell leaves the world, a bitten head is a death,
    /// and the flesh is in the spider's crop at face value. Nothing is bitten
    /// in an empty cell or in the biter's own body. **And the switch**: the
    /// act is reachable from a pick only while `options::attack_on` is set,
    /// which nothing in the test process sets, so the game's minds are
    /// untouched (`Verb::acts`).
    /// **The bite's entry opens a banned Attack and nothing else**, only with its switch on: a banned genome takes
    /// the switch and the job constant, one that already bites is left alone, and with the switch off neither moves.
    #[test]
    fn the_bite_enters_a_banned_genome_only_under_its_switch() {
        let s = SWITCH + Verb::Attack as usize;
        let k = JOB + Verb::Attack as usize * J_IN + CONTEXT + MEMORY + 1;
        let mut g = vec![0.0; GENOME_LEN];
        g[s] = -1.0;
        let before = g.clone();
        let did = enter_bite(&mut g, 2.5);
        assert_eq!(did, options::attack_on());
        if did {
            assert_eq!((g[s], g[k]), (1.0, 2.5));
            assert_eq!(g.iter().zip(&before).filter(|(a, b)| a != b).count(), 2, "the entry moved another slot");
            assert!(!enter_bite(&mut g, -7.0), "a genome that bites was entered again");
            assert_eq!(g[k], 2.5);
        } else {
            assert_eq!(g, before);
        }
    }

    #[test]
    fn the_bite_takes_a_strangers_cell_and_the_mouth_keeps_it() {
        assert_eq!(Verb::Attack.acts(), options::attack_on(), "the Attack act is reachable with its switch off");
        let mut w = crate::lab::scene::LabBox {
            founders: 0,
            colonies: 0,
            width: 128,
            height: 96,
            ground_y: 60,
            soil_depth: 30,
            ..crate::lab::scene::LabBox::default()
        }
        .build();
        let gy = (0..96).find(|&y| y > 10 && w.get(64, y).material != crate::sim::material::EMPTY).expect("ground");
        let spider = match crate::sim::creature::plant_creature_seed(&mut w, 62, gy - 1, "spider").map(|s| s.kind) {
            Some(crate::sim::scheduler::ActiveKind::Creature { organism }) => organism,
            other => panic!("no spider placed: {other:?}"),
        };
        let ant = match crate::sim::creature::plant_creature_seed(&mut w, 64, gy - 1, "ant").map(|s| s.kind) {
            Some(crate::sim::scheduler::ActiveKind::Creature { organism }) => organism,
            other => panic!("no ant placed: {other:?}"),
        };
        let sdef = w.species.get(w.organism(spider).expect("spider").species).creature.clone().expect("creature");
        let h = head_of(&w, spider);
        let ant_body = w.organism(ant).expect("ant").chain.clone();
        let t = ant_body.iter().copied().find(|&(x, y)| (x - h.0).abs() <= 1 && (y - h.1).abs() <= 1).expect("the ant beside the spider's head");
        let flesh = w.get(t.0, t.1).material;
        give(&mut w, spider, Arc::new(MindGenome::new(blank(&[Verb::Walk, Verb::Attack]))));
        assert!(!attack(&mut w, spider, &sdef, (h.0, h.1 - 3)), "bit the air");
        assert!(!attack(&mut w, spider, &sdef, h), "bit itself");
        assert!(attack(&mut w, spider, &sdef, t), "the bite");
        assert_ne!(w.get(t.0, t.1).material, flesh, "the bitten cell is still there");
        let k = w.minds.as_ref().expect("minds").counts;
        assert_eq!((k.bites, k.flesh), (1, 1));
        let crop = w.organism(spider).and_then(|s| s.crop).expect("flesh in the crop");
        assert_eq!(crop.material, flesh);
        assert!((crop.worth() - w.materials.get(flesh).food_energy).abs() < 1.0, "crop worth {}", crop.worth());
        if t == ant_body[0] {
            assert!(w.organism(ant).is_none(), "a bitten head left the ant alive");
            assert_eq!(k.kills, 1);
        } else {
            assert!(w.organism(ant).is_some_and(|s| s.chain.len() == ant_body.len() - 1), "a bitten segment");
        }
    }

    /// One `species` animal and one ant side by side on the ground of a small box: (world, animal, ant).
    fn beside_an_ant(species: &str) -> (World, OrganismId, OrganismId) {
        let mut w = crate::lab::scene::LabBox {
            founders: 0,
            colonies: 0,
            width: 128,
            height: 96,
            ground_y: 60,
            soil_depth: 30,
            ..crate::lab::scene::LabBox::default()
        }
        .build();
        let gy = (0..96).find(|&y| y > 10 && w.get(64, y).material != crate::sim::material::EMPTY).expect("ground");
        let place = |w: &mut World, x: i32, name: &str| match crate::sim::creature::plant_creature_seed(w, x, gy - 1, name).map(|s| s.kind) {
            Some(crate::sim::scheduler::ActiveKind::Creature { organism }) => organism,
            other => panic!("no {name} placed: {other:?}"),
        };
        let animal = place(&mut w, 60, species);
        let ant = place(&mut w, 62, "ant");
        (w, animal, ant)
    }

    /// **The raider smells the nest from across the box** (nest raider thread, 2026-10-26; off by default,
    /// `options::nest_smell_on`): a centipede 50 columns from a founded colony, no ant in its 24-cell eye, smells the
    /// nest's door with the switch on, and with `smells_nest` cleared or the switch off smells nothing. The eye saw
    /// no prey (`sensed_home` reads no home with the switch at its default), so the door came from the smell.
    #[test]
    fn the_raider_smells_the_nest_from_across_the_box() {
        let mut w = crate::lab::scene::LabBox { founders: 4, colonies: 1, width: 160, height: 96, ground_y: 60, soil_depth: 30, ..crate::lab::scene::LabBox::default() }.build();
        let door = w.nest_sites.first().map(|s| (s.x, s.surface)).expect("a founded colony has a nest site");
        let x = if door.0 > 80 { door.0 - 50 } else { door.0 + 50 };
        let gy = (0..96).find(|&y| y > 10 && w.get(x, y).material != crate::sim::material::EMPTY).expect("ground");
        let me = match crate::sim::creature::plant_creature_seed(&mut w, x, gy - 1, "centipede").map(|s| s.kind) {
            Some(crate::sim::scheduler::ActiveKind::Creature { organism }) => organism,
            other => panic!("no centipede placed: {other:?}"),
        };
        let st = w.organism(me).expect("placed").clone();
        let mut def = w.species.get(st.species).creature.clone().expect("creature");
        let (hx, hy) = *st.chain.first().expect("a head");
        assert_eq!(options::smelt_nest_if(true, &w, &def, hx, hy), Some(door), "smelt");
        assert_eq!(options::smelt_nest_if(false, &w, &def, hx, hy), None, "switch off");
        if !options::nest_smell_on() {
            let (home, reads) = options::sensed_home(&w, me, &def, &st);
            assert!(reads > 0, "the eye was cast");
            assert_eq!(home, None, "the eye sees no prey, and the smell is off by default");
        }
        def.smells_nest = false;
        assert_eq!(options::smelt_nest_if(true, &w, &def, hx, hy), None, "a species without the smell");
    }

    /// **A predator with handling time cannot pay for a child out of living prey** (nest raider thread,
    /// 2026-10-25): `try_bud`'s second mouth (`provisions_in_reach`) counted every living ant cell round the head as
    /// food in reach and took it whole, so a centipede landed at a lab door budded three times in 23 frames. With
    /// `handling` a living animal's cell is not a provision. **The positive control is the beetle**, a carnivore
    /// without handling: the same ant beside it is still counted, so the census can see a living ant at all.
    #[test]
    fn a_predator_with_handling_cannot_pay_a_birth_with_living_prey() {
        for (species, handles) in [("centipede", true), ("spider", true), ("beetle", false)] {
            let (w, me, ant) = beside_an_ant(species);
            let def = w.species.get(w.organism(me).expect("placed").species).creature.clone().expect("creature");
            assert_eq!(def.handling, handles, "{species}: the arms of this test moved");
            let gut = crate::sim::creature::gut_of(&w, me, &def);
            let ant_cells: Vec<(i32, i32)> = w.organism(ant).expect("ant").chain.clone();
            // Bring the ant into reach of the head, whatever the two bodies' shapes.
            let reach = |w: &World| {
                let (hx, hy) = head_of(w, me);
                crate::sim::creature::provisions_in_reach(w, hx, hy, gut).filter(|&(_, x, y)| ant_cells.contains(&(x, y))).count()
            };
            let (hx, hy) = head_of(&w, me);
            assert!(
                ant_cells.iter().any(|&(x, y)| (x - hx).abs() <= 1 && (y - hy).abs() <= 1),
                "{species}: the ant is not in reach of the head at {:?} ({ant_cells:?})",
                (hx, hy)
            );
            assert_eq!(reach(&w) > 0, !crate::sim::creature::handling_on(&def), "{species}: whether a living ant pays a birth");
        }
    }

    /// **A seat's player body bites on with a full crop, and nothing else does** (enemy seat, owner's playtest
    /// 2026-10-25, `steer::Steered::bites_when_full`): the centipede's crop held three ant cells and every bite after
    /// the first kill needed a spit. The control is the same centipede, full and unmarked, refused by the sated rule;
    /// the marked one takes the cell off without swallowing it.
    #[test]
    fn a_full_player_raider_bites_on_and_swallows_nothing() {
        let (mut w, me, ant) = beside_an_ant("centipede");
        let def = w.species.get(w.organism(me).expect("placed").species).creature.clone().expect("creature");
        assert!(crate::sim::creature::handling_on(&def), "the centipede lost its sated rule");
        give(&mut w, me, Arc::new(MindGenome::new(blank(crate::train::predator::RAIDER_VERBS))));
        let h = head_of(&w, me);
        let t = w.organism(ant).expect("ant").chain.iter().copied().find(|&(x, y)| (x - h.0).abs() <= 1 && (y - h.1).abs() <= 1).expect("the ant beside the head");
        let cell = w.get(t.0, t.1);
        let worth = food_value(&w, cell);
        while w.organism(me).and_then(|s| s.crop).is_none_or(|c| c.worth() + c.unit <= organism_crop_capacity(&w, me, &def)) {
            crop_take(&mut w, me, cell.material, cell.shade, worth, None);
        }
        let full = w.organism(me).and_then(|s| s.crop).expect("crop").worth();
        assert!(!attack(&mut w, me, &def, t), "a full raider bit with nobody steering it");
        crate::sim::steer::mark_with(&mut w, me, Some(crate::sim::steer::SteerInput::default()), crate::sim::steer::SteerRole::Worker);
        assert!(!attack(&mut w, me, &def, t), "a steered raider bit full without the seat's leave");
        w.steered.as_mut().expect("steered").bites_when_full = true;
        assert!(attack(&mut w, me, &def, t), "the player's full raider was refused");
        assert_ne!(w.get(t.0, t.1).material, cell.material, "the bitten cell is still there");
        let after = w.organism(me).and_then(|s| s.crop).expect("crop").worth();
        assert!((after - full).abs() < 1e-3, "a full crop swallowed: {full} -> {after}");
    }

    /// **A centipede regrows a lost tail out of a meal, and only then** (enemy seat's hit and run, 2026-10-25,
    /// `CreatureDef::regrows`): bitten from five cells to three, healed, with a bank above a full tank plus a stamp
    /// it grows one cell behind its tail per window, paying the stamp; at a full tank, or still wounded, it does not;
    /// and an ant (`regrows` off) never does, however rich.
    #[test]
    fn a_fed_centipede_regrows_its_tail_and_a_hungry_one_does_not() {
        let (mut w, me, ant) = beside_an_ant("centipede");
        let def = w.species.get(w.organism(me).expect("placed").species).creature.clone().expect("creature");
        let adef = w.species.get(w.organism(ant).expect("placed").species).creature.clone().expect("creature");
        assert!(def.regrows && !adef.regrows, "the arms of this test moved");
        let plan = w.organism(me).expect("placed").chain.len();
        assert_eq!(plan, 5);
        for _ in 0..2 {
            let tail = *w.organism(me).expect("alive").chain.last().expect("tail");
            w.set(tail.0, tail.1, Cell::EMPTY);
            assert!(super::super::reconcile_chain(&mut w, me), "a tail bite killed it");
        }
        assert_eq!(w.organism(me).expect("alive").chain.len(), 3);
        // Step to the start of this body's window, so one call is one chance.
        let window = |w: &mut World, who: OrganismId| {
            let k = super::super::REGROW_FRAMES;
            w.frame = (w.frame / k + 1) * k - (who as u64 % k);
        };
        let set = |w: &mut World, who: OrganismId, energy: f32, gnawed: f32| {
            let st = w.organism_mut(who).expect("alive");
            st.energy = energy;
            st.gnawed = gnawed;
        };
        let len = |w: &World, who: OrganismId| w.organism(who).expect("alive").chain.len();
        // A full tank only: the stamp is not there to pay.
        set(&mut w, me, def.start_energy, 0.0);
        window(&mut w, me);
        super::super::regrow_tail(&mut w, me, &def);
        assert_eq!(len(&w, me), 3, "grew a cell out of a tank that cannot pay for it");
        // Fed, but still being chewed.
        set(&mut w, me, def.start_energy + 2.0 * def.body_energy + 1.0, 0.5);
        window(&mut w, me);
        super::super::regrow_tail(&mut w, me, &def);
        assert_eq!(len(&w, me), 3, "grew a cell with a wound open");
        // Fed and healed: a cell, paid for.
        set(&mut w, me, def.start_energy + 2.0 * def.body_energy + 1.0, 0.0);
        let stamped = w.energy_ledger.stamped;
        window(&mut w, me);
        super::super::regrow_tail(&mut w, me, &def);
        assert_eq!(len(&w, me), 4, "a fed, healed centipede did not regrow");
        let st = w.organism(me).expect("alive");
        assert!((st.energy - (def.start_energy + def.body_energy + 1.0)).abs() < 1e-3, "the stamp was not paid: {}", st.energy);
        let new = *st.chain.last().expect("tail");
        assert_eq!(w.get(new.0, new.1).organism_id(), me, "the new cell is not the centipede's");
        assert!(st.cells.contains_key(&new), "the new cell is not filed as the body's");
        assert!((w.energy_ledger.stamped - stamped - f64::from(def.body_energy)).abs() < 1e-6, "the stamp was not booked");
        assert_eq!(w.creature_stats.regrown_cells, 1);
        // The body's next tick is past its window: once a window.
        w.frame += crate::sim::creature::organism_tick_interval(&w, me, &def);
        super::super::regrow_tail(&mut w, me, &def);
        assert_eq!(len(&w, me), 4, "grew twice in one window");
        // An ant, short a cell and rich, does not.
        let tail = *w.organism(ant).expect("ant").chain.last().expect("tail");
        let alen = len(&w, ant);
        if alen >= 2 {
            w.set(tail.0, tail.1, Cell::EMPTY);
            assert!(super::super::reconcile_chain(&mut w, ant));
            set(&mut w, ant, adef.start_energy * 10.0, 0.0);
            window(&mut w, ant);
            super::super::regrow_tail(&mut w, ant, &adef);
            assert_eq!(len(&w, ant), alen - 1, "the ant regrew");
        }
    }

    /// **The raider shoves through stored food it cannot eat, and through nothing else** (nest raider thread,
    /// 2026-10-25, `CreatureDef::digs_inedible_food`): a centipede's jaws may cut a cell of crumbs, which its
    /// carnivore gut will not take, but not soil (0.8 against its 0.5 jaw), and the cut keeps the crumb's worth in
    /// the pellet. An ant, which eats crumbs, still may not dig them: food is its pick-up's.
    #[test]
    fn the_raider_shoves_through_crumbs_it_cannot_eat_and_nothing_else() {
        let (mut w, me, ant) = beside_an_ant("centipede");
        let def = w.species.get(w.organism(me).expect("placed").species).creature.clone().expect("creature");
        let adef = w.species.get(w.organism(ant).expect("placed").species).creature.clone().expect("creature");
        assert!(def.digs_inedible_food && !adef.digs_inedible_food);
        let crumbs = w.materials.id_of("crumbs").expect("crumbs");
        let (hx, hy) = head_of(&w, me);
        let t = DIRS.iter().map(|&(dx, dy)| (hx + dx, hy + dy)).find(|&(x, y)| w.get(x, y).material != crate::sim::material::EMPTY && w.get(x, y).organism_id() == 0).expect("ground beside the head");
        w.set(t.0, t.1, Cell::new(w.materials.id_of("soil").expect("soil"), 0));
        assert!(!options::diggable(&w, &def, me, w.get(t.0, t.1)), "the raider's jaw cut soil");
        let crumb = Cell::new(crumbs, 0).with_aux(400);
        w.set(t.0, t.1, crumb);
        assert!(options::diggable(&w, &def, me, w.get(t.0, t.1)), "the raider cannot cut crumbs");
        assert!(!options::diggable(&w, &adef, ant, w.get(t.0, t.1)), "an ant may dig food");
        give(&mut w, me, Arc::new(MindGenome::new(blank(crate::train::predator::RAIDER_VERBS))));
        assert!(dig(&mut w, me, &def, t), "the cut");
        let pellet = w.organism(me).and_then(|s| s.spoil).expect("the crumb in the jaws");
        assert_eq!((pellet.cell.material, pellet.cell.aux()), (crumbs, 400), "the crumb's worth was not kept");
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
        assert_eq!(
            GENOME_LEN,
            EMIT + EMITS * E_IN + HIDDEN * H_IN + VERBS * HIDDEN + UNITS * GATE_IN + MEMORY * VERBS + UNITS * VERBS
                + MEMORY * W_IN
        );
        assert_eq!(
            (GENOME_LEN_V1, GENOME_LEN_V2, GENOME_LEN_V5, GENOME_LEN_V6, GENOME_LEN_V7, GENOME_LEN),
            (2333, 2669, 2797, 3357, 3609, 4137),
            "saved genomes are 2,333 slots before the hidden units, 2,669 before PR 2's features, 2,797 before the \
             gates, 3,357 before the reach rows, 3,609 before the write gates"
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
        // ...and its gate with them.
        assert_eq!(crossover_group(GATE + 2 * GATE_IN + 7), crossover_group(S_OUT + 2));
        assert_ne!(crossover_group(GATE + 2 * GATE_IN), crossover_group(GATE + 3 * GATE_IN));
        // A reach row travels with the memory unit or gate it feeds.
        assert_eq!(crossover_group(REACH_MEM + 3 * VERBS + 1), crossover_group(RATE + 3));
        assert_eq!(crossover_group(REACH_GATE + 2 * VERBS + 8), crossover_group(S_OUT + 2));
        let t = trainable(&[Verb::Walk, Verb::Rest], [false, true, false]);
        assert!(t.contains(&(SWITCH + Verb::Walk as usize)) && !t.contains(&(SWITCH + Verb::Dig as usize)));
        assert!(t.contains(&(EMIT + E_IN)) && !t.contains(&EMIT) && !t.contains(&(EMIT + 2 * E_IN)));
        assert!(t.contains(&HID) && t.contains(&(GATE - 1)), "the job hidden units are not an emit row");
        assert_eq!(t.contains(&GATE), gate_on(), "the gates are trainable unless PP_GATE=off");
        assert_eq!(t.contains(&REACH_MEM), reach_on(), "memory's reach rows are trainable unless PP_REACH=off");
        assert_eq!(t.contains(&REACH_GATE), gate_on() && reach_on());
        assert_eq!(t.contains(&WRITE), write_on(), "the write gates are trainable only under PP_WRITE=on");
        assert_eq!(crossover_group(WRITE + 4 * W_IN + 3), crossover_group(RATE + 4), "a write gate travels with its unit");
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
/// **The memory a decision writes when its verb is `verb`**, from the decision's
/// own scores: what `decide` writes for the verb it picked, recomputed for a verb
/// carried out instead (a forced pick, `steer::ForcedPick`; the teachers'
/// overrides through `scratch_log::remember`). Out of the scratch module because
/// the player's body depends on it.
pub(crate) fn remember_done(g: &[f32], ctx: &[f32; CONTEXT], mem0: &[f32; MEMORY], opts: &[Opt], scores: &[f32], verb: Verb) -> [f32; MEMORY] {
    let on = |v: Verb| v.acts() && g[SWITCH + v as usize] > 0.0;
    let (mut best, mut per_verb, mut sum, mut n) = (f32::NEG_INFINITY, [f32::NEG_INFINITY; VERBS], 0.0f32, 0u32);
    for (o, op) in opts.iter().enumerate() {
        if on(op.verb) {
            best = best.max(scores[o]);
            per_verb[op.verb as usize] = per_verb[op.verb as usize].max(scores[o]);
            sum += scores[o];
            n += 1;
        }
    }
    let all_mean = if n > 0 { sum / n as f32 } else { 0.0 };
    remembered(g, ctx, mem0, best, all_mean, verb, &reach(&per_verb))
}

pub mod scratch_log {
    use super::*;
    use std::collections::HashMap;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Mutex, OnceLock};

    // ---- SCRATCH (foraging-loop thread, 2026-10-22): "carry it home", the old ant's laden rules as a teacher. The
    // game's own ant (chooser_step + act) delivers 25% of what it bites in the in-run bed against h1's 3%, with the
    // same body: h1 swallows 38% of its bites by choice and puts 41% back down in the field, no nearer home. The
    // label, for an ant holding food it took away from home (`trip_cells` > 0) with at least `PP_CARRY_FLOOR` tanks
    // (default 1; under it the ant may eat its load):
    //   at the nest (the delivery's own `nest_within_reach`): put it down (every Drop offered)
    //   else, food in reach the crop can take: pick it up (fill before walking, `fills_before_walking`)
    //   else: the steps that end nearest home (forage_anchor, Euclidean), if nearer than the head is now
    // So it never eats the load and never puts it down away from home. Reads only what the ant has: its crop, bank,
    // the options and where home is.
    pub fn carry_act() -> Option<f32> {
        static Q: OnceLock<Option<f32>> = OnceLock::new();
        *Q.get_or_init(|| std::env::var("PP_CARRY").ok().map(|v| v.parse().expect("PP_CARRY=q")))
    }
    pub static CARRY_COUNTS: [AtomicU64; 3] = [const { AtomicU64::new(0) }; 3];
    /// `PP_CARRY_WAY=1`: steps through the colony's tunnels to the founding hole's mouth, as a distance field over open
    /// (or ant-held) cells, rebuilt every 50 frames. Traced (pa8 seed 1, ant 23): hungry and empty in one of h1's side
    /// galleries 11 columns out, the "highest step" label climbed into the gallery roof and fell back for 3,000
    /// frames until it starved. A teacher may know the way; the copy then has to find a sense that carries it.
    pub fn way_dist(world: &World, mouth: (i32, i32), p: (i32, i32)) -> Option<u32> {
        type Field = (u64, (i32, i32), HashMap<(i32, i32), u32>);
        thread_local! { static FIELD: std::cell::RefCell<Field> = std::cell::RefCell::new((u64::MAX, (0, 0), HashMap::new())); }
        FIELD.with(|f| {
            let mut f = f.borrow_mut();
            let epoch = world.frame / 50;
            if f.0 != epoch || f.1 != mouth {
                let open = |x: i32, y: i32| (x - mouth.0).abs() <= 70 && y >= mouth.1 - 20 && y <= mouth.1 + 50 && (world.is_empty(x, y) || world.get(x, y).organism_id() != 0);
                let mut d: HashMap<(i32, i32), u32> = HashMap::new();
                let mut q = std::collections::VecDeque::new();
                for dx in -2..=2 {
                    for dy in -2..=1 {
                        let c = (mouth.0 + dx, mouth.1 + dy);
                        if open(c.0, c.1) {
                            d.insert(c, 0);
                            q.push_back(c);
                        }
                    }
                }
                while let Some((x, y)) = q.pop_front() {
                    let k = d[&(x, y)];
                    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                        let n = (x + dx, y + dy);
                        if !d.contains_key(&n) && open(n.0, n.1) {
                            d.insert(n, k + 1);
                            q.push_back(n);
                        }
                    }
                }
                *f = (epoch, mouth, d);
            }
            f.2.get(&p).copied()
        })
    }
    fn way_on() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var("PP_CARRY_WAY").is_ok())
    }
    /// The offered walks that end nearest the mouth through the tunnels, if nearer than the head.
    fn way_walks(world: &World, mouth: (i32, i32), at: (i32, i32), opts: &[Opt]) -> Option<Vec<usize>> {
        let here = way_dist(world, mouth, at)?;
        let ws: Vec<(usize, u32)> = (0..opts.len()).filter(|&i| opts[i].verb == Verb::Walk).filter_map(|i| way_dist(world, mouth, opts[i].target).map(|k| (i, k))).collect();
        let best = ws.iter().map(|w| w.1).min().filter(|&b| b < here)?;
        Some(ws.into_iter().filter(|w| w.1 == best).map(|w| w.0).collect())
    }
    /// Hungry branches: spoil-drop, spoil-up, spoil-stuck, eat, pick, empty-nothing (no label).
    pub static HUNGRY_BRANCH: [AtomicU64; 6] = [const { AtomicU64::new(0) }; 6];
    // `PP_CARRY_HUNGRY=H` (tanks) adds the old ant's hungry rules (`needs_first_act`; the hungry lane's trace: 94% of
    // starving decisions were ants holding a soil pellet at the chamber bottom beside a full store). Under H tanks:
    //   holding spoil: put it down off the hole (a Drop with no CUT and no DEPTH), else take the highest step
    //   food in the crop: eat it (a trip load at the nest included: a hungry ant does not hand its load over)
    //   empty: pick up any food in reach
    fn carry_hungry() -> Option<f32> {
        static H: OnceLock<Option<f32>> = OnceLock::new();
        *H.get_or_init(|| std::env::var("PP_CARRY_HUNGRY").ok().map(|v| v.parse().expect("PP_CARRY_HUNGRY")))
    }
    pub static HOMEFAR: AtomicU64 = AtomicU64::new(0);
    pub fn far_ctx() -> Option<i32> {
        static R: OnceLock<Option<i32>> = OnceLock::new();
        *R.get_or_init(|| std::env::var("PP_FAR_CTX").ok().map(|v| v.parse().expect("PP_FAR_CTX")))
    }
    fn homefar() -> Option<i32> {
        static R: OnceLock<Option<i32>> = OnceLock::new();
        *R.get_or_init(|| std::env::var("PP_HOMEFAR").ok().map(|v| v.parse().expect("PP_HOMEFAR")))
    }
    pub fn carry_label(world: &World, organism: OrganismId, def: &CreatureDef, at: (i32, i32), ctx: &[f32; CONTEXT], opts: &[Opt]) -> Vec<usize> {
        let Some(st) = world.organism(organism) else { return Vec::new() };
        let of = |v: Verb| -> Vec<usize> { (0..opts.len()).filter(|&i| opts[i].verb == v).collect() };
        let e = ctx[c::ENERGY] / (1.0 - ctx[c::ENERGY]).max(1e-6);
        // `PP_CARRY_DUMP=id:f0:f1`: print one ant's options while it decides, for tracing a stuck carrier.
        if let Ok(v) = std::env::var("PP_CARRY_DUMP") {
            let p: Vec<u64> = v.split(':').map(|x| x.parse().unwrap()).collect();
            if organism as u64 == p[0] && (p[1]..=p[2]).contains(&world.frame) && world.frame.is_multiple_of(30) {
                let ms = |x: i32, y: i32| { let c = world.get(x, y); format!("{}{}", world.materials.get(c.material).name.chars().take(4).collect::<String>(), if c.organism_id() != 0 { format!("#{}", c.organism_id()) } else { String::new() }) };
                let chain: Vec<(i32, i32)> = st.chain.clone();
                eprintln!("DUMP f{} e{:.2} at{:?} chain{:?} spoil{} below[{} {} {}] opts: {}", world.frame, e, at, chain, st.spoil.is_some(), ms(at.0 - 1, at.1 + 1), ms(at.0, at.1 + 1), ms(at.0 + 1, at.1 + 1),
                    opts.iter().map(|o| format!("{:?}{:?}", o.verb, o.target)).collect::<Vec<_>>().join(" "));
            }
        }
        // `PP_CARRY_HOME_R=R`: home is the body's own `nest_within_reach` AND within R columns of the founding hole.
        // The bed's home rule (`NestHome::Dug`) counts every cell the colony ever cut, and h1 cuts surface soil out at
        // the food, so the unbounded teacher "delivered" 1,994 of its 5,239 loads 12+ columns out (old ant: 0).
        static HOME_R: OnceLock<Option<i32>> = OnceLock::new();
        let home_r = *HOME_R.get_or_init(|| std::env::var("PP_CARRY_HOME_R").ok().map(|v| v.parse().expect("PP_CARRY_HOME_R")));
        // With R set, "home" is also the founding site (the nearest `nest_sites` entry, its ground row), not
        // `forage_anchor`: the anchor follows every nest contact, so it drifts out to those same field cuts.
        let site = home_r.and_then(|_| world.nest_sites.iter().min_by_key(|s| (s.x - at.0).abs()).map(|s| (s.x, s.surface)));
        let anchor = site.unwrap_or(st.forage_anchor);
        let mouth = (anchor.0, anchor.1 - 1);
        let below_ground = ctx[c::DEPTH] > 0.05;
        // Underground with a way to the mouth known: take it (`PP_CARRY_WAY`), for every walking rule below.
        let way = |opts: &[Opt]| if way_on() && below_ground { way_walks(world, mouth, at, opts) } else { None };
        let home = |p: (i32, i32)| nest_within_reach(world, organism, p.0, p.1, def) && home_r.is_none_or(|r| (p.0 - anchor.0).abs() <= r);
        // Walled in: holding spoil, and nothing offered but stepping into its own body, turning round or resting.
        if st.spoil.is_some() && opts.iter().all(|o| matches!(o.verb, Verb::TurnAround | Verb::Rest) || (o.verb == Verb::Walk && st.chain.contains(&o.target))) {
            WALLED.lock().unwrap().get_or_insert_with(HashMap::new).insert(organism, world.frame);
        }
        // SCRATCH (starvation-trace thread, 2026-10-22): `PP_HOMEFAR=R`, "come home when empty and far". An ant with
        // nothing in the crop and no pellet, more than R columns from the founding hole, takes the step that brings it
        // nearest home's column; with none (a wall face), the step down. The trace (/mnt/project-files/starve-trace/):
        // 50-62% of hA1 starvation deaths walked out fat past 60 columns, showed no homeward pull at any tank (net
        // -2% to +2% of walks) with a correct home sense, and starved at the box walls ~45k frames later.
        // SCRATCH (teacher lane, 2026-10-25): `PP_HOMEFAR_HUNGRY=1` gates the rule on hunger (`PP_CARRY_HUNGRY`'s H
        // tanks) instead of on an empty crop alone. The empty-and-far rule leaked into foraging when copied (every empty
        // ant learned to stay home); a hungry one leaves the fed forager alone. Traced (nest batch 4, colony 7, far
        // moving food): 72 of 106 starved ants reached a bed wall while fat (median 13 tanks), and at the wall the
        // brain's walk scores sit at +-0.99 whatever `f::HOME` reads -- 504 steps away, 536 into it, 403 along it -- so
        // the way back 300 columns is a random walk of tens of thousands of frames.
        let hungry_gate = std::env::var("PP_HOMEFAR_HUNGRY").is_ok_and(|v| v == "1");
        // Hungry and food in reach: the hungry branch's pick-up comes first, so an ant at a far heap eats there.
        // `PP_HOMEFAR_FUEL=k`: hungry means fewer than 1 + k x (columns from home) tanks -- enough to walk back, so a
        // far ant turns early and a near one late. The flat gate turned ants 300 columns out at 2 tanks, ~165 columns
        // of walking, and they starved on the way (ceiling, seed 23099: 139 of 303 died in the field).
        static FUEL: OnceLock<Option<f32>> = OnceLock::new();
        let fuel = *FUEL.get_or_init(|| std::env::var("PP_HOMEFAR_FUEL").ok().map(|v| v.parse().expect("PP_HOMEFAR_FUEL")));
        let fuel_ok = fuel.is_none_or(|k| {
            let nx = world.nest_sites.iter().min_by_key(|s| (s.x - st.forage_anchor.0).abs()).map_or(st.forage_anchor.0, |s| s.x);
            e < 1.0 + k * (at.0 - nx).abs() as f32
        });
        let hungry_now = if fuel.is_some() { fuel_ok } else { carry_hungry().is_some_and(|h| e < h) };
        let gate_ok = !hungry_gate || (hungry_now && of(Verb::PickUp).is_empty());
        // Under the hunger gate a pellet does not exempt the ant: it puts the pellet down wherever it may and walks
        // home. Left to the hungry branch below, a hungry carrier on the surface is walked AWAY from home's column
        // ("up, then away"): 43 of 91 dead in the climb-out ceiling (seed 21099) died holding spoil, 78 at the walls.
        if let Some(r) = homefar().filter(|_| gate_ok) {
            if st.crop.is_none() && (st.spoil.is_none() || hungry_gate) {
                let ax = st.forage_anchor.0;
                let nx = world.nest_sites.iter().min_by_key(|s| (s.x - ax).abs()).map_or(ax, |s| s.x);
                if st.spoil.is_some() && (at.0 - nx).abs() > r {
                    let drops = of(Verb::Drop);
                    if !drops.is_empty() {
                        HOMEFAR.fetch_add(1, Ordering::Relaxed);
                        return drops;
                    }
                }
                if (at.0 - nx).abs() > r {
                    // Under the hunger gate a step into the ant's own body is no step: turn round instead. Traced
                    // (ceiling, seed 23099, ant 1048627): facing away from home in a heap pit, the only nearer step
                    // was its own tail cell, labelled every decision, and it stood on one cell for 2,500 frames.
                    let walks: Vec<usize> = of(Verb::Walk).into_iter().filter(|&i| !hungry_gate || !st.chain.contains(&opts[i].target)).collect();
                    let far = |x: i32| (x - nx).abs();
                    if hungry_gate && st.chain.iter().any(|&(cx, _)| far(cx) < far(at.0)) && !walks.iter().any(|&i| far(opts[i].target.0) < far(at.0)) {
                        let turns = of(Verb::TurnAround);
                        if !turns.is_empty() {
                            HOMEFAR.fetch_add(1, Ordering::Relaxed);
                            return turns;
                        }
                    }
                    if let Some(best) = walks.iter().map(|&i| far(opts[i].target.0)).min().filter(|&b| b < far(at.0)) {
                        HOMEFAR.fetch_add(1, Ordering::Relaxed);
                        return walks.into_iter().filter(|&i| far(opts[i].target.0) == best).collect();
                    }
                    // Under the hunger gate, a hungry ant in a pit below the ground as built climbs out, and with no
                    // step up cuts towards home. Traced (ceiling, seed 21099, ant 3145756): 2 tanks in a two-cell pit
                    // dug where a heap had been, packed soil on the home side, only same-column steps offered; it sat
                    // there until it starved, as 89 of 104 dead did in that run.
                    if hungry_gate && ctx[c::DEPTH] > 0.05 {
                        if let Some(top) = walks.iter().map(|&i| opts[i].target.1).min().filter(|&t| t < at.1) {
                            HOMEFAR.fetch_add(1, Ordering::Relaxed);
                            return walks.into_iter().filter(|&i| opts[i].target.1 == top).collect();
                        }
                        let digs = of(Verb::Dig);
                        if let Some(best) = digs.iter().map(|&i| far(opts[i].target.0)).min().filter(|&b| b < far(at.0)) {
                            HOMEFAR.fetch_add(1, Ordering::Relaxed);
                            return digs.into_iter().filter(|&i| far(opts[i].target.0) == best).collect();
                        }
                    }
                    if let Some(low) = walks.iter().map(|&i| opts[i].target.1).max().filter(|&l| l > at.1) {
                        HOMEFAR.fetch_add(1, Ordering::Relaxed);
                        return walks.into_iter().filter(|&i| opts[i].target.1 == low).collect();
                    }
                }
            }
        }
        if carry_hungry().is_some_and(|h| e < h) {
            if st.spoil.is_some() {
                // A cut offered with a pellet in the jaws is the pack move (`PP_PACK`): only a walled-in ant has one.
                let packs = of(Verb::Dig);
                if !packs.is_empty() {
                    return packs;
                }
                // `PP_CARRY_ANYDROP=1`: the old ant's hungry put-down (`need_drop_site`) takes any footing beside the head, so
                // under it any offered Drop is the label; and an ant hanging in a gallery (nothing under the head) steps
                // down to the floor, where the mind's on-ground pellet rule can offer one. Traced (ph8 seed 4, ant 9):
                // hungry with a pellet in an open gallery, only Walks offered, the "highest step" label climbed and fell
                // back for 2,000 frames until it starved beside a full store.
                let any_drop = std::env::var("PP_CARRY_ANYDROP").is_ok();
                if any_drop && ctx[c::DEPTH] > 0.05 && of(Verb::Drop).is_empty() && world.is_empty(at.0, at.1 + 1) {
                    let walks = of(Verb::Walk);
                    if let Some(low) = walks.iter().map(|&i| opts[i].target.1).max().filter(|&l| l > at.1) {
                        return walks.into_iter().filter(|&i| opts[i].target.1 == low).collect();
                    }
                }
                let off: Vec<usize> = (0..opts.len())
                    .filter(|&i| opts[i].verb == Verb::Drop && (any_drop || (opts[i].feats[options::f::CUT] == 0.0 || std::env::var("PP_CARRY_ANYCUT").is_ok()) && opts[i].feats[options::f::DEPTH] <= 0.0))
                    .collect();
                if !off.is_empty() {
                    HUNGRY_BRANCH[0].fetch_add(1, Ordering::Relaxed);
                    return off;
                }
                if let Some(w) = way(opts) {
                    return w;
                }
                let walks = of(Verb::Walk);
                // On the surface: carry it away from home's column (the hungry lane's "up, then away"); climbing alone
                // ran hungry carriers up and down the mound (10,512 up-labels against 34 off-hole drops, seed 3).
                if ctx[c::DEPTH] <= 0.05 {
                    let nx = anchor.0;
                    let far = |x: i32| (x - nx).abs();
                    if let Some(best) = walks.iter().map(|&i| far(opts[i].target.0)).max().filter(|&b| b > far(at.0)) {
                        HUNGRY_BRANCH[1].fetch_add(1, Ordering::Relaxed);
                        return walks.into_iter().filter(|&i| far(opts[i].target.0) == best).collect();
                    }
                }
                let top = walks.iter().map(|&i| opts[i].target.1).min().unwrap_or(at.1);
                HUNGRY_BRANCH[if top < at.1 { 1 } else { 2 }].fetch_add(1, Ordering::Relaxed);
                // Stuck (no way up, nowhere off the hole): put it down anywhere, so the jaws can eat or dig out.
                // `PP_CARRY_STUCK=0` leaves the brain's pick instead.
                if top >= at.1 && std::env::var("PP_CARRY_STUCK").map_or(true, |v| v != "0") {
                    let digs = of(Verb::Dig);
                    return if digs.is_empty() { of(Verb::Drop) } else { digs };
                }
                return if top < at.1 { walks.into_iter().filter(|&i| opts[i].target.1 == top).collect() } else { Vec::new() };
            }
            // `PP_CARRY_NOEAT=1`: as the old ant, never Eat. It chooses to eat 0 of its bites and digests ~70% from the
            // crop it carries (bed, seeds 1-4); every old ant ends 25+ tanks. A hungry ant with food keeps holding it
            // (anything but Eat or Drop) and the body feeds it; a trip load goes on home by the carry rules below.
            let no_eat = std::env::var("PP_CARRY_NOEAT").is_ok();
            if no_eat && st.crop.is_some() && st.trip_cells == 0 {
                return (0..opts.len()).filter(|&i| !matches!(opts[i].verb, Verb::Eat | Verb::Drop)).collect();
            }
            if !no_eat && st.crop.is_some() && (st.trip_cells == 0 || home(at)) {
                HUNGRY_BRANCH[3].fetch_add(1, Ordering::Relaxed);
                return of(Verb::Eat);
            }
            if st.crop.is_none() {
                let p = of(Verb::PickUp);
                HUNGRY_BRANCH[if p.is_empty() { 5 } else { 4 }].fetch_add(1, Ordering::Relaxed);
                if !p.is_empty() {
                    return p;
                }
                // `PP_CARRY_STORE=1`: the old ant's hungry rule first -- walk to the nest's store over the way
                // (`NestWay::store_at`, the field the game ant's store pull walks) and eat there. Every starved ant
                // in pw8 died beside a store that still held food, most within 8 columns of the hole.
                if std::env::var("PP_CARRY_STORE").is_ok() {
                    if let Some(w) = world.nest_ways.iter().find(|w| w.store_at(at.0, at.1).is_some()) {
                        let here = w.store_at(at.0, at.1).unwrap();
                        let ws: Vec<(usize, u16)> = of(Verb::Walk).into_iter().filter_map(|i| w.store_at(opts[i].target.0, opts[i].target.1).map(|k| (i, k))).collect();
                        if let Some(best) = ws.iter().map(|x| x.1).min().filter(|&b| b < here) {
                            return ws.into_iter().filter(|x| x.1 == best).map(|x| x.0).collect();
                        }
                    }
                }
                // Nothing in reach: go out for it (the hungry lane's lesson, its 40-column radius), never dig. Below the
                // ground as built, the highest step; on the surface, the step furthest from home's column.
                if st.spoil.is_none() && std::env::var("PP_CARRY_OUT").map_or(true, |v| v != "0") {
                    if let Some(w) = way(opts) {
                        return w;
                    }
                    let walks = of(Verb::Walk);
                    let nx = anchor.0;
                    let below = ctx[c::DEPTH] > 0.05;
                    let key = |(tx, ty): (i32, i32)| if below { (at.1 - ty, (tx - nx).abs()) } else { ((tx - nx).abs(), at.1 - ty) };
                    if !below && (at.0 - nx).abs() >= 40 {
                        return Vec::new();
                    }
                    let here = key(at);
                    let Some(best) = walks.iter().map(|&i| key(opts[i].target)).max() else { return Vec::new() };
                    if best.0 > here.0 {
                        return walks.into_iter().filter(|&i| key(opts[i].target).0 == best.0).collect();
                    }
                }
            }
        }
        // `PP_CARRY_HUNGRYONLY=1`: the hungry rules alone, no carry-home labels (a ceiling for a hungry-only lesson).
        if st.crop.is_none() || st.trip_cells == 0 || std::env::var("PP_CARRY_HUNGRYONLY").is_ok() {
            return Vec::new();
        }
        static FLOOR: OnceLock<f32> = OnceLock::new();
        let floor = *FLOOR.get_or_init(|| std::env::var("PP_CARRY_FLOOR").ok().map_or(1.0, |v| v.parse().expect("PP_CARRY_FLOOR")));
        if e < floor {
            return Vec::new();
        }
        if home(at) {
            return of(Verb::Drop);
        }
        let picks = of(Verb::PickUp);
        if !picks.is_empty() {
            return picks;
        }
        if let Some(w) = way(opts) {
            return w;
        }
        let (ax, ay) = anchor;
        let d = |(x, y): (i32, i32)| (((x - ax) * (x - ax) + (y - ay) * (y - ay)) as f32).sqrt();
        let walks = of(Verb::Walk);
        let Some(best) = walks.iter().map(|&i| d(opts[i].target)).min_by(|a, b| a.partial_cmp(b).unwrap()) else { return Vec::new() };
        if best >= d(at) {
            return Vec::new();
        }
        walks.into_iter().filter(|&i| d(opts[i].target) == best).collect()
    }
    pub fn carry_out(world: &World, organism: OrganismId, def: &CreatureDef, at: (i32, i32), ctx: &[f32; CONTEXT], opts: &[Opt], pick: usize) -> Option<usize> {
        let q = carry_act()?;
        let lab = carry_label(world, organism, def, at, ctx, opts);
        if lab.is_empty() {
            return None;
        }
        CARRY_COUNTS[0].fetch_add(1, Ordering::Relaxed);
        if lab.contains(&pick) {
            CARRY_COUNTS[1].fetch_add(1, Ordering::Relaxed);
            return None;
        }
        if roll(world, organism, 91) >= q {
            return None;
        }
        CARRY_COUNTS[2].fetch_add(1, Ordering::Relaxed);
        Some(lab[(roll(world, organism, 92) * lab.len() as f32) as usize % lab.len()])
    }
    pub static PACKS: AtomicU64 = AtomicU64::new(0);
    pub static WALLED: Mutex<Option<HashMap<OrganismId, u64>>> = Mutex::new(None);
    // ---- SCRATCH (foraging-loop thread, 2026-10-22): `PP_SHADOW=1`, the old ant as the teacher. Every founder keeps
    // its mind but walks by the game's own rules (chooser + act), as `PP_NOMIND`; before each of its ticks the mind
    // is shown the options and context it would have had and decides (memory only, never acted on), and after the
    // tick the old ant's move is read back off the body -- the cut, put-down, pick-up or step it made -- as the option
    // indices that match it. That row (the 16th copy-log field) is the label. Hand-written rules failed five times
    // (report-2026-10-22.md, "Correction"); this asks the bar itself.
    /// SCRATCH (plant-food lane, 2026-10-23): `PP_MIND_GRAZE=1` lets a mind's Pick up take food a plant still owns
    /// -- a leaf, blade, flower or fruit on the stem, and a seed lying on the floor (a waiting seed is a one-cell
    /// plant) -- as the game's ant already bites them (`act`'s bite: the seed asks `seed_survives_bite`, a spared one
    /// rides home as the crop's passenger, the rest leaves the world as `Eaten`). Off: loose food only, as before.
    /// Measured on the planted training bed (inrun `plants=`, seed 1, hcs founder): with it off the mind took 5 bites
    /// in 2,500 frames and all 24 starved beside a stand holding ~940 leaf cells, ~90 fruit and ~180 seeds, while the
    /// game's own ant on the same bed took 712 bites (430 leaf, 109 flower) and all 24 lived to 20,000.
    /// **On by default** (owner, 2026-10-24: "Yes to grazing"); `PP_MIND_GRAZE=off` (or `0`) is loose food only.
    pub fn graze_on() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| options::switch_on("PP_MIND_GRAZE"))
    }
    /// Under [`graze_on`]: a cell a plant owns (its owner has no creature body). Off: never.
    pub fn grazeable(world: &World, cell: Cell) -> bool {
        let owner = cell.organism_id();
        graze_on() && owner != 0 && world.organism(owner).is_some_and(|s| world.species.get(s.species).creature.is_none())
    }
    pub fn shadow_on() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var("PP_SHADOW").is_ok_and(|v| v == "1"))
    }
    struct Pending {
        row: String,
        opts: Vec<Opt>,
        cells: Vec<(u16, OrganismId)>,
        ctx: [f32; CONTEXT],
        mem0: [f32; MEMORY],
        scores: Vec<f32>,
        head: (i32, i32),
        tail: Option<(i32, i32)>,
        spoil: bool,
        crop: u16,
        /// `PP_SELFAGREE`: the labels the old ant earned on the same tick in a clone with other dice, if sampled.
        alt: Option<Vec<usize>>,
    }
    thread_local! {
        static SHADOW: std::cell::RefCell<HashMap<OrganismId, Pending>> = std::cell::RefCell::new(HashMap::new());
        static SHADOW_PREV: std::cell::RefCell<HashMap<OrganismId, (bool, String)>> = std::cell::RefCell::new(HashMap::new());
        static CAPTURE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    }
    pub static SHADOW_COUNTS: [AtomicU64; 8] = [const { AtomicU64::new(0) }; 8];
    fn cell_key(world: &World, p: (i32, i32)) -> (u16, OrganismId) {
        let c = world.get(p.0, p.1);
        (c.material.0, c.organism_id())
    }
    /// The options the old ant's last tick matches: what changed on the body, and which offered target it touched.
    fn shadow_label(world: &World, organism: OrganismId, p: &Pending) -> Vec<usize> {
        let Some(st) = world.organism(organism) else { return Vec::new() };
        let head = st.chain.first().copied().unwrap_or(p.head);
        let spoil = st.spoil.is_some();
        let crop = st.crop.map_or(0, |c| c.cells);
        let changed = |i: usize| cell_key(world, p.opts[i].target) != p.cells[i];
        let with = |v: Verb, need_change: bool| -> Vec<usize> { (0..p.opts.len()).filter(|&i| p.opts[i].verb == v && (!need_change || changed(i))).collect() };
        if !p.spoil && spoil {
            SHADOW_COUNTS[1].fetch_add(1, Ordering::Relaxed);
            return with(Verb::Dig, true);
        }
        if p.spoil && !spoil {
            SHADOW_COUNTS[2].fetch_add(1, Ordering::Relaxed);
            let mut v = with(Verb::Drop, true);
            v.extend(with(Verb::Dig, true));
            return v;
        }
        if crop > p.crop {
            SHADOW_COUNTS[3].fetch_add(1, Ordering::Relaxed);
            return with(Verb::PickUp, true);
        }
        if crop < p.crop {
            let v = with(Verb::Drop, true);
            if !v.is_empty() {
                SHADOW_COUNTS[4].fetch_add(1, Ordering::Relaxed);
                return v;
            }
            // Digested, or fed to a nestmate: not a choice the options name.
            SHADOW_COUNTS[7].fetch_add(1, Ordering::Relaxed);
            return Vec::new();
        }
        if head != p.head {
            SHADOW_COUNTS[5].fetch_add(1, Ordering::Relaxed);
            if Some(head) == p.tail {
                return with(Verb::TurnAround, false);
            }
            return (0..p.opts.len()).filter(|&i| p.opts[i].verb == Verb::Walk && p.opts[i].target == head).collect();
        }
        SHADOW_COUNTS[6].fetch_add(1, Ordering::Relaxed);
        with(Verb::Rest, false)
    }
    /// Close the ant's pending row with the label its tick earned, write it (sampled), and walk the memory on with
    /// the verb the old ant used.
    fn shadow_resolve(world: &mut World, organism: OrganismId) {
        let Some(p) = SHADOW.with(|s| s.borrow_mut().remove(&organism)) else { return };
        let lab = shadow_label(world, organism, &p);
        SHADOW_COUNTS[0].fetch_add(1, Ordering::Relaxed);
        // SCRATCH (copy test, 2026-10-28): with the decision trace on (`labsurvive` `PP_DECLOG=1`), one `W` row for
        // every tick the old ant earned a turn-round, a pick-up or a dig, and one in 20 others: frame, ant, the label's
        // verb, then why its walk did what it did -- the outcome, the pull (`PULL_WHY_NAMES`) and its gain, the door
        // reader (`DOOR_WHY_NAMES`) and its term, the scout weight and give-up flag, patience, persistence, the trail
        // presence of the heading taken, the forage drive and the want. The question: at the moves the copy misses,
        // which of the walk's reasons is the mind not shown?
        if let Some(log) = world.decision_log.as_ref() {
            let lv = lab.first().map_or(-1, |&i| p.opts[i].verb as i32);
            let rare = matches!(lab.first().map(|&i| p.opts[i].verb), Some(Verb::TurnAround | Verb::PickUp | Verb::Dig));
            let take = rare || rng::stream(world.seed, organism as u64, world.frame, 4448).next_u64().is_multiple_of(20);
            if take {
                if let Some(r) = log.iter().rev().find(|r| r.id == organism && r.frame < world.frame) {
                    cline(&format!(
                        "W,{},{},{},{},{},{},{:.3},{},{:.3},{:.3},{},{:.3},{:.3},{:.3},{:.3},{:.3},{}",
                        world.frame,
                        organism,
                        lv,
                        r.frame,
                        r.outcome as u8,
                        r.pull_why,
                        r.pull_gain,
                        r.door_why,
                        r.door_f,
                        r.scout_w,
                        u8::from(r.scout_home),
                        r.patience,
                        r.persist,
                        r.chosen_route,
                        r.drive,
                        r.want,
                        r.fill
                    ));
                }
            }
        }
        // `PP_SELFAGREE`: one `A` row per sampled tick -- frame, ant, the verb of the label the old ant earned (-1 none),
        // the clone's, and whether the two label sets share an option. Read every tick, not sampled like `Q` rows.
        if let Some(alt) = &p.alt {
            let verb = |l: &[usize]| l.first().map_or(-1, |&i| p.opts[i].verb as i32);
            let agree = u8::from(lab.iter().any(|i| alt.contains(i)) || (lab.is_empty() && alt.is_empty()));
            cline(&format!("A,{},{},{},{},{}", world.frame, organism, verb(&lab), verb(alt), agree));
        }
        let field = if lab.is_empty() { "-".to_string() } else { format!("1|{}", lab.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(";")) };
        if !p.row.contains(",@L@") {
            return;
        }
        let mut row = p.row.replacen(",@L@", &format!(",{field}"), 1);
        // SCRATCH (copy test, 2026-10-28): with `PP_SHADOW_STATE=1` and the decision trace on, the old walk's own draw
        // for the step it took this tick -- its exploration `k`, its persistence, and the score it gave each option's
        // heading (`os=`, in option order, NaN for an option that is not a one-cell step) -- so the ceiling can be
        // shown the hand-built ant's distribution itself (the positive control), and then each part of it.
        if shadow_state_on() && row.contains(",S:") {
            if let Some(r) = world.decision_log.as_ref().and_then(|l| l.iter().rev().find(|r| r.id == organism && r.frame < world.frame)) {
                let os: Vec<String> = p
                    .opts
                    .iter()
                    .map(|o| {
                        let d = (o.target.0 - p.head.0, o.target.1 - p.head.1);
                        match crate::sim::creature::DIRS.iter().position(|&v| v == d) {
                            Some(i) if r.opts & (1 << i) != 0 => format!("{:.3}", r.score[i]),
                            _ => "NaN".into(),
                        }
                    })
                    .collect();
                row = format!("{row};k={:.4};pe={:.3};os={}", r.k, r.persist, os.join("/"));
            }
        }
        // `PP_SHADOW_SAMPLE=n`: keep one in n rows labelled a plain step or rest, every other labelled row; a kept row
        // is preceded by the ant's previous row marked settle=2 (memory context), as `PP_COPY_SAMPLE` does.
        static N: OnceLock<u64> = OnceLock::new();
        let n = *N.get_or_init(|| std::env::var("PP_SHADOW_SAMPLE").ok().map_or(1, |v| v.parse().expect("PP_SHADOW_SAMPLE")));
        let plain = lab.is_empty() || lab.iter().all(|&i| matches!(p.opts[i].verb, Verb::Walk | Verb::Rest | Verb::TurnAround));
        let keep = !lab.is_empty() && (!plain || n <= 1 || rng::stream(world.seed, organism as u64, world.frame, 4445).next_u64().is_multiple_of(n));
        let settle2 = |r: &str| {
            let mut f: Vec<&str> = r.splitn(7, ',').collect();
            if f.len() == 7 {
                f[5] = "2";
            }
            f.join(",")
        };
        SHADOW_PREV.with(|pm| {
            let mut pm = pm.borrow_mut();
            if keep {
                if let Some((written, prev)) = pm.get(&organism) {
                    if !*written {
                        cline(&settle2(prev));
                    }
                }
                cline(&row);
            }
            pm.insert(organism, (keep, row));
        });
        // Memory follows the verb carried out; with no label, the mind's own pick stands (decide already wrote it).
        if let Some(&i) = lab.first() {
            let verb = p.opts[i].verb;
            if let Some(m) = world.minds.as_mut().and_then(|m| m.ants.get_mut(&organism)) {
                m.state.mem = remember(&m.genome.w, &p.ctx, &p.mem0, &p.opts, &p.scores, verb);
                m.state.last_verb = Some(verb);
            }
        }
    }
    /// Before the old ant's tick: close the last row, then show the mind this tick's options and let it decide.
    pub fn shadow_pre(world: &mut World, x: i32, y: i32, organism: OrganismId, def: &CreatureDef) {
        shadow_resolve(world, organism);
        let Some(mut mind) = world.minds.as_mut().and_then(|m| m.ants.get(&organism).cloned()) else { return };
        let mut opts = Vec::new();
        options::build_with(world, organism, def, mind.genome.w[SWITCH + Verb::Dig as usize] > 0.0, &mut opts);
        if opts.is_empty() {
            return;
        }
        let (ctx, now) = context(world, organism, def, &mind.state, &opts);
        let mut draw = rng::stream(world.seed, organism as u64, world.frame, RNG_SLOT_MIND);
        let (mem0, last0) = (mind.state.mem, mind.state.last_verb);
        let Some(d) = decide(&mind.genome.w, &ctx, &opts, &mut mind.state, &mut draw) else { return };
        mind.state.last = Some(now);
        CAPTURE.with(|c| *c.borrow_mut() = Some(String::new()));
        CARRY_LAB.with(|c| *c.borrow_mut() = "@L@".into());
        copy_decision(world, organism, (x, y), &ctx, &mem0, last0, &opts, &d);
        let mut row = CAPTURE.with(|c| c.borrow_mut().take()).unwrap_or_default();
        let st = world.organism(organism);
        // SCRATCH (copy test, 2026-10-28): `PP_SHADOW_STATE=1` appends the old ant's own walk state, which no mind input
        // carries, as an 18th field `S:k=v;...`, to ask whether it is what the copy's ceiling misses at turn-rounds
        // and pick-ups: the two patiences (home, scout), the scout's give-up and dark flags, ticks stood still, ticks
        // since the nest, the lagged trail-A memory, the anchor's offset from the head, the heading, and `home_away`.
        if shadow_state_on() {
            if let Some(s) = st {
                let line = row.trim_end().to_string();
                row = format!(
                    "{line},S:hp={:.3};sp={:.3};sh={};sd={};st={};sn={};pm={:.3};ax={};ay={};hd={};ha={}",
                    s.home_patience,
                    s.scout_patience,
                    u8::from(s.scout_home),
                    u8::from(s.scout_dark),
                    s.still_ticks,
                    s.since_nest,
                    s.phero_a_mem,
                    s.forage_anchor.0 - x,
                    s.forage_anchor.1 - y,
                    s.heading,
                    s.home_away
                );
                // With the decision trace on: the errand its walk was last pulled by (`PULL_WHY_NAMES`) and where to,
                // from the head -- the walk's target, which no option feature names unless it is home or the last food.
                if let Some(r) = world.decision_log.as_ref().and_then(|l| l.iter().rev().find(|r| r.id == organism)) {
                    row = format!("{row};pw={};px={};py={}", r.pull_why, r.pull_at.0 - x, r.pull_at.1 - y);
                }
            }
        }
        let p = Pending {
            row: row.trim_end().to_string(),
            cells: opts.iter().map(|o| cell_key(world, o.target)).collect(),
            ctx,
            mem0,
            scores: d.scores.clone(),
            head: (x, y),
            tail: st.and_then(|s| s.chain.last().copied()),
            spoil: st.is_some_and(|s| s.spoil.is_some()),
            crop: st.and_then(|s| s.crop).map_or(0, |c| c.cells),
            alt: None,
            opts,
        };
        if let Some(m) = world.minds.as_mut().and_then(|m| m.ants.get_mut(&organism)) {
            m.state = mind.state;
        }
        if !p.row.is_empty() {
            SHADOW.with(|s| s.borrow_mut().insert(organism, p));
        }
    }
    /// SCRATCH (copy test, 2026-10-28): `PP_SELFAGREE=k`: under `PP_SHADOW`, one old-ant tick in k is also played in a
    /// clone whose seed is XORed with `PP_SELFAGREE_XOR` (default 0x5e1fa62e), and the two labels compared (`A` rows).
    /// The rate at which the old ant agrees with itself under other dice is the most any copy reading only this
    /// tick's state could score (Σp² of its own choice), so it splits the copy test's 0.41 ceiling into the old ant's
    /// own randomness and what the mind cannot see. `PP_SELFAGREE_XOR=0` is the positive control: it must agree 100%.
    pub fn selfagree_take(world: &World, organism: OrganismId) -> bool {
        static K: OnceLock<u64> = OnceLock::new();
        let k = *K.get_or_init(|| std::env::var("PP_SELFAGREE").ok().map_or(0, |v| v.parse().expect("PP_SELFAGREE")));
        k > 0 && SHADOW.with(|s| s.borrow().contains_key(&organism)) && rng::stream(world.seed, organism as u64, world.frame, 4447).next_u64().is_multiple_of(k)
    }
    pub fn selfagree_xor() -> u64 {
        static X: OnceLock<u64> = OnceLock::new();
        *X.get_or_init(|| std::env::var("PP_SELFAGREE_XOR").ok().map_or(0x5e1f_a62e, |v| v.parse().expect("PP_SELFAGREE_XOR")))
    }
    pub fn selfagree_store(clone: &World, organism: OrganismId) {
        let alt = SHADOW.with(|s| s.borrow().get(&organism).map(|p| shadow_label(clone, organism, p)));
        SHADOW.with(|s| {
            if let (Some(p), Some(a)) = (s.borrow_mut().get_mut(&organism), alt) {
                p.alt = Some(a);
            }
        });
    }
    pub fn shadow_state_on() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var("PP_SHADOW_STATE").is_ok_and(|v| v == "1"))
    }
    pub static DROPWHY: [AtomicU64; 6] = [const { AtomicU64::new(0) }; 6];
    pub fn carry_report() -> String {
        if let Some(m) = WALLED.lock().unwrap().as_ref() {
            let mut v: Vec<_> = m.iter().collect();
            v.sort();
            eprintln!("WALLED {}", v.iter().map(|(i, f)| format!("{i}:{f}")).collect::<Vec<_>>().join(" "));
        }
        let dw: Vec<u64> = DROPWHY.iter().map(|c| c.load(Ordering::Relaxed)).collect();
        if dw[0] > 0 {
            eprintln!("DROPWHY pellet holders under 1 tank: {} decisions; an empty cell beside the head {}; an on-ground spoil site {}; the game ant's need site {}; 3+ nestmates beside {}; no empty cell and a nestmate beside {}", dw[0], dw[1], dw[2], dw[3], dw[4], dw[5]);
        }
        let c: Vec<u64> = CARRY_COUNTS.iter().map(|c| c.load(Ordering::Relaxed)).collect();
        eprintln!("PACKS {} HOMEFAR labels {}", PACKS.load(Ordering::Relaxed), HOMEFAR.load(Ordering::Relaxed));
        let h: Vec<u64> = HUNGRY_BRANCH.iter().map(|c| c.load(Ordering::Relaxed)).collect();
        format!("carry {:?}: labelled {} | brain already agreed {} | forced {} || hungry decisions: spoil drop {} up {} stuck {} | eat {} | pick {} | empty, nothing to pick {}", carry_act(), c[0], c[1], c[2], h[0], h[1], h[2], h[3], h[4], h[5])
    }

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
        super::remember_done(g, ctx, mem0, opts, scores, verb)
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
    // ---- SCRATCH (dig-scoring thread, 2026-10-22; Teacher lane's proposal): `PP_OLDLAB=1`, the old ant labels the
    // mind's OWN states (DAgger), for a dig lesson. Before a mind ant's tick in a dig state (holding spoil, or empty,
    // not hungry, below the datum within 40 columns of the door), the world is cloned, the clone's ant loses its mind
    // and takes one tick as the game's own ant (chooser + act), and what it did -- the cut, put-down, pick-up or step,
    // read off its body as PP_SHADOW does -- becomes the option indices of the mind's decision in the real world, the
    // copy log's 16th field ("1|i;j"). The real world is untouched (events.csv byte-identical, hA1 20k frames).
    // First copies onto hA1 (all dig-state labels, or cut / no-cut only) starved 21-23 of 24: Reports in
    // /mnt/project-files/nest-score/dig-lesson/README.md.
    pub fn oldlab_on() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var("PP_OLDLAB").is_ok_and(|v| v == "1"))
    }
    /// `PP_OLDLAB_K=k`: label one in k dig-state ticks (default 4).
    pub fn oldlab_take(world: &World, organism: OrganismId) -> bool {
        static K: OnceLock<u64> = OnceLock::new();
        let k = *K.get_or_init(|| std::env::var("PP_OLDLAB_K").ok().map_or(4, |v| v.parse().expect("PP_OLDLAB_K")));
        rng::stream(world.seed, organism as u64, world.frame, 4446).next_u64().is_multiple_of(k)
    }
    /// SCRATCH (copy test, 2026-10-27): `PP_OLDLAB_ALL=1`: the old ant labels every sampled decision of a mind ant
    /// (DAgger over its whole program, for copying the hand-built ant into a mind), not only its dig states.
    pub fn oldlab_all() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var("PP_OLDLAB_ALL").is_ok_and(|v| v == "1"))
    }
    /// `PP_OLDLAB_EMPTY=1`: label only while the jaws are empty (no spoil carry; see `oldlab_empty_only`).
    pub fn oldlab_label_empty() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var("PP_OLDLAB_EMPTY").is_ok_and(|v| v == "1"))
    }
    thread_local! {
        pub static OLDLAB: std::cell::RefCell<Option<(OrganismId, Box<World>)>> = const { std::cell::RefCell::new(None) };
        pub static OLDLAB_LAST: std::cell::RefCell<Option<(u8, Vec<usize>)>> = const { std::cell::RefCell::new(None) };
    }
    pub static OLDLAB_COUNTS: [AtomicU64; 8] = [const { AtomicU64::new(0) }; 8];
    pub static OLDLAB_ACTS: [AtomicU64; 3] = [const { AtomicU64::new(0) }; 3];
    /// Take a mind off one ant (the clone's, so it ticks as the game's ant).
    pub fn strip_mind(world: &mut World, organism: OrganismId) {
        if let Some(m) = world.minds.as_mut() {
            m.ants.remove(&organism);
        }
    }
    /// `PP_OLDLAB_ACT=1`: the old ant's cut calls act on the mind's own states; `=2` its whole underground program
    /// (steps, turns, rests, cuts, spoil put-downs; not food calls); `=3` as 2 but only while the jaws are empty.
    pub fn oldlab_act() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var("PP_OLDLAB_ACT").is_ok_and(|v| v == "1" || v == "2" || v == "3" || v == "4"))
    }
    pub fn oldlab_act_all() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var("PP_OLDLAB_ACT").is_ok_and(|v| v == "2" || v == "3"))
    }
    /// Under `=2`, ants held spoil on the surface within 12 columns of the door for 61-65% of all ant-time (hA1
    /// 17-18%), in spells 3x as long, and home food fell 279 -> 101: the old ant's carry in a mind's body never lets
    /// go (traced, seeds 1-2, 2026-10-22). `=3` leaves the carry to the mind.
    pub fn oldlab_empty_only() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var("PP_OLDLAB_ACT").is_ok_and(|v| v == "3"))
    }
    /// `=4`: **the dig head's ceiling** -- only when the mind picked Dig and the old ant cut too, the old ant's cell
    /// is cut instead. Which cell, never when: all a per-option Dig head can change, since hcs's Dig job reads its
    /// best target at -0.29.
    pub fn oldlab_cell_only() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var("PP_OLDLAB_ACT").is_ok_and(|v| v == "4"))
    }
    /// The label for this decision from the clone the old ant ticked in, if one is waiting for this ant.
    pub fn oldlab_label(world: &World, organism: OrganismId, at: (i32, i32), opts: &[Opt]) {
        let Some((id, wc)) = OLDLAB.with(|o| o.borrow_mut().take()) else { return };
        if id != organism {
            return;
        }
        let key = |w: &World, p: (i32, i32)| {
            let c = w.get(p.0, p.1);
            (c.material.0, c.organism_id())
        };
        let Some(st0) = world.organism(organism) else { return };
        let (tail0, spoil0, crop0) = (st0.chain.last().copied(), st0.spoil.is_some(), st0.crop.map_or(0, |c| c.cells));
        let mut cut = false;
        let lab: Vec<usize> = match wc.organism(organism) {
            None => Vec::new(),
            Some(st) => {
                let head = st.chain.first().copied().unwrap_or(at);
                let (spoil, crop) = (st.spoil.is_some(), st.crop.map_or(0, |c| c.cells));
                let changed = |i: usize| key(&wc, opts[i].target) != key(world, opts[i].target);
                let with = |v: Verb, need: bool| -> Vec<usize> { (0..opts.len()).filter(|&i| opts[i].verb == v && (!need || changed(i))).collect() };
                let (k, v) = if !spoil0 && spoil {
                    cut = true;
                    (1, with(Verb::Dig, true))
                } else if spoil0 && !spoil {
                    let mut v = with(Verb::Drop, true);
                    v.extend(with(Verb::Dig, true));
                    (2, v)
                } else if crop > crop0 {
                    (3, with(Verb::PickUp, true))
                } else if crop < crop0 {
                    (4, with(Verb::Drop, true))
                } else if head != at {
                    if Some(head) == tail0 {
                        (5, with(Verb::TurnAround, false))
                    } else {
                        (5, (0..opts.len()).filter(|&i| opts[i].verb == Verb::Walk && opts[i].target == head).collect())
                    }
                } else {
                    (6, with(Verb::Rest, false))
                };
                OLDLAB_COUNTS[k].fetch_add(1, Ordering::Relaxed);
                if v.is_empty() {
                    OLDLAB_COUNTS[7].fetch_add(1, Ordering::Relaxed);
                }
                v
            }
        };
        OLDLAB_COUNTS[0].fetch_add(1, Ordering::Relaxed);
        if oldlab_act() {
            OLDLAB_LAST.with(|o| *o.borrow_mut() = Some((u8::from(cut), lab.clone())));
        }
        if !lab.is_empty() {
            CARRY_LAB.with(|c| *c.borrow_mut() = format!("1|{}", lab.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(";")));
        }
    }
    pub fn oldlab_report() -> String {
        let c: Vec<u64> = OLDLAB_COUNTS.iter().map(|c| c.load(Ordering::Relaxed)).collect();
        let x: Vec<u64> = OLDLAB_ACTS.iter().map(|c| c.load(Ordering::Relaxed)).collect();
        format!("oldlab acts: cut as the old ant {} | did the old ant's move instead of a cut {} | other old-ant moves {} || oldlab: labelled {} | cut {} | let go {} | took food {} | put food {} | step {} | rest {} | no option matched {}", x[0], x[1], x[2], c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7])
    }
    // ---- SCRATCH (dig-scoring thread, 2026-10-22; Teacher lane's pick of three): **Dig's own feature head**. Every
    // copy of the old ant's digging into the shipped weights failed: whole-scorer copies broke foraging (hA1 d1/d2
    // starved 21-23 of 24), and copies restricted to the Dig verb's own slots (hcs hd/hd2/hu) lost the nest (13 ->
    // 0-1.5), because those slots set *when* the ant digs, not *which cell*; the cell is scored by the shared feature
    // weights. So, as the far-home sense did: new inputs with weights nothing else reads. `PP_DIGFEAT=1` computes
    // DIG_ND per-option features for every Dig option and logs them as feature indices 100+k (the copier's head
    // reads them); `PP_DIGHEAD=path` loads a small head, H units over the option's 45 features and these, whose
    // output is added to each Dig option's raw score before the sharpness squash. Nothing set: nothing computed,
    // byte-identical. The weights live in a side file, not the genome, so a mutant cannot move them.
    pub const DIG_ND: usize = 12;
    #[derive(Debug)]
    pub struct DigHead {
        pub h: usize,
        pub w: Vec<f32>,
        pub b: Vec<f32>,
        pub a: Vec<f32>,
        pub c: f32,
        /// **When**: a second read-out of the same units, `c2 + max over the Dig options of e . units`, added to the
        /// Dig job's sum. The ceiling for a head that picks only the cell (`PP_OLDLAB_ACT=4`: the old ant's cell
        /// whenever hcs digs) moved nest 13 -> 16 (15 up / 16 down): the old ant's cut calls (ACT=1, nest 38.5) work
        /// through *when*, 782 of hcs's digs a run replaced by its move against 166 cuts added. Empty: rank only.
        pub e: Vec<f32>,
        pub c2: f32,
    }
    impl DigHead {
        /// Its weights in the side file's order: W, b, a, c, then e, c2 with a "when" read-out.
        pub fn flat(&self) -> Vec<f32> {
            let mut v = [self.w.as_slice(), &self.b, &self.a, &[self.c]].concat();
            if !self.e.is_empty() {
                v.extend_from_slice(&self.e);
                v.push(self.c2);
            }
            v
        }
        /// A head of this one's shape with weights `v` ([`Self::flat`]'s order).
        pub fn from_flat(&self, v: &[f32]) -> DigHead {
            let (h, n) = (self.h, FEATURES + DIG_ND);
            assert_eq!(v.len(), self.flat().len(), "dig head: {} weights, this shape has {}", v.len(), self.flat().len());
            let (e, c2) = if self.e.is_empty() { (Vec::new(), 0.0) } else { (v[h * n + 2 * h + 1..h * n + 3 * h + 1].to_vec(), v[h * n + 3 * h + 1]) };
            DigHead { h, w: v[..h * n].to_vec(), b: v[h * n..h * n + h].to_vec(), a: v[h * n + h..h * n + 2 * h].to_vec(), c: v[h * n + 2 * h], e, c2 }
        }
        /// (rank, when) for one Dig option.
        pub fn eval2(&self, feats: &[f32; FEATURES], d: &[f32; DIG_ND]) -> (f32, f32) {
            let n = FEATURES + DIG_ND;
            let (mut out, mut when) = (self.c, 0.0);
            for u in 0..self.h {
                let row = &self.w[u * n..(u + 1) * n];
                let mut z = self.b[u];
                for (w, x) in row[..FEATURES].iter().zip(feats) {
                    z += w * x;
                }
                for (w, x) in row[FEATURES..].iter().zip(d) {
                    z += w * x;
                }
                out += self.a[u] * squash(z);
                if let Some(e) = self.e.get(u) {
                    when += e * squash(z);
                }
            }
            (out, when)
        }
        pub fn eval(&self, feats: &[f32; FEATURES], d: &[f32; DIG_ND]) -> f32 {
            let n = FEATURES + DIG_ND;
            let mut out = self.c;
            for u in 0..self.h {
                let row = &self.w[u * n..(u + 1) * n];
                let mut z = self.b[u];
                for (w, x) in row[..FEATURES].iter().zip(feats) {
                    z += w * x;
                }
                for (w, x) in row[FEATURES..].iter().zip(d) {
                    z += w * x;
                }
                out += self.a[u] * squash(z);
            }
            out
        }
    }
    /// `PP_DIGHEAD=path`: "H ND" then W (H rows of 45 + ND), b (H), a (H), c, whitespace-separated; with a "when"
    /// read-out, "H ND 1" and then also e (H), c2.
    ///
    /// **On by default with [`DH7`]** (owner, 2026-10-24: the standard training path is default-on): unset reads the
    /// shipped head, a path reads that file, `PP_DIGHEAD=off` (or `0`) is no head -- the mind's own Dig links alone.
    pub fn dighead() -> Option<&'static DigHead> {
        static H: OnceLock<Option<DigHead>> = OnceLock::new();
        H.get_or_init(|| {
            let txt = match std::env::var("PP_DIGHEAD") {
                Err(_) => DH7.to_string(),
                Ok(p) if !options::switch_on_value(&p) => return None,
                Ok(p) => std::fs::read_to_string(&p).expect("PP_DIGHEAD file"),
            };
            let first = txt.lines().next().expect("PP_DIGHEAD header");
            let when = first.split_whitespace().count() == 3 && first.split_whitespace().nth(2) == Some("1");
            let v: Vec<f32> = txt.lines().skip(1).flat_map(|l| l.split_whitespace()).map(|x| x.parse().expect("PP_DIGHEAD number")).collect();
            let hv: Vec<usize> = first.split_whitespace().map(|x| x.parse().expect("PP_DIGHEAD header")).collect();
            let (h, nd) = (hv[0], hv[1]);
            assert_eq!(nd, DIG_ND, "PP_DIGHEAD: {nd} dig features, this build has {DIG_ND}");
            let n = FEATURES + DIG_ND;
            assert_eq!(v.len(), h * n + 2 * h + 1 + if when { h + 1 } else { 0 }, "PP_DIGHEAD: wrong length");
            let w = v[..h * n].to_vec();
            let b = v[h * n..h * n + h].to_vec();
            let a = v[h * n + h..h * n + 2 * h].to_vec();
            let c = v[h * n + 2 * h];
            let (e, c2) = if when { (v[h * n + 2 * h + 1..h * n + 3 * h + 1].to_vec(), v[h * n + 3 * h + 1]) } else { (Vec::new(), 0.0) };
            Some(DigHead { h, w, b, a, c, e, c2 })
        })
        .as_ref()
    }
    /// `PP_DIGHEAD_EVOLVE=1` (with `PP_DIGHEAD`): the head's weights ride each genome past [`GENOME_LEN`], so they
    /// mutate, cross and are inherited like the rest; the side file is the founders' start. The length they add.
    /// **Why** (Teacher lane, 2026-10-23): dh7 is one fixed head for every ant, so selection could not improve on it.
    /// On by default whenever there is a head; `PP_DIGHEAD_EVOLVE=off` (or `0`) makes the side file everyone's again.
    /// A genome of plain [`GENOME_LEN`] still runs either way, on the side file's head.
    pub fn head_evolve() -> Option<usize> {
        static ON: OnceLock<Option<usize>> = OnceLock::new();
        *ON.get_or_init(|| if options::switch_on("PP_DIGHEAD_EVOLVE") { dighead().map(|h| h.flat().len()) } else { None })
    }
    /// The standard line's dig head, dh7 (Teacher lane, 2026-10-23; `scratch/inrun/genomes/dh7.head` on the compute
    /// branch): every colony-selection founder since colsel20 started from it.
    pub const DH7: &str = include_str!("../../../assets/minds/dh7.head");
    /// `PP_DIGHEAD_GATE`: on by default; `=off` (or `0`) for the ungated head.
    pub fn dig_gate_on() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| options::switch_on("PP_DIGHEAD_GATE"))
    }
    /// `PP_DIGGATE_DOOR`: the dig head's gate measures "within 40 columns of the door" from the nearest nest site,
    /// not from column 128 ([`dig_prepare`]). On by default (Teacher lane, 2026-10-24: a fix that is bit-identical on
    /// the bed ships on); `=0` or `=off` is the old gate.
    fn door_gate_on() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| options::switch_on("PP_DIGGATE_DOOR"))
    }
    pub fn dighead_on() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| dighead().is_some())
    }
    pub fn digfeat_on() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var("PP_DIGFEAT").is_ok_and(|v| v == "1") || dighead_on())
    }
    /// One Dig option of this decision: target, its dig features, the head's addition to its raw score.
    pub type DigOpt = ((i32, i32), [f32; DIG_ND], f32);
    thread_local! {
        /// This decision's Dig options: target, its dig features, the head's addition to its raw score.
        pub static DIG_OPTS: std::cell::RefCell<Vec<DigOpt>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    thread_local! {
        /// This decision's "when": the head's addition to the Dig job's sum (0 without a when read-out).
        pub static DIG_WHEN: std::cell::Cell<f32> = const { std::cell::Cell::new(0.0) };
    }
    pub fn dig_when() -> f32 {
        DIG_WHEN.with(|w| w.get())
    }
    /// The head's addition for the Dig option aimed at `t`, 0 when none.
    pub fn dig_bonus(t: (i32, i32)) -> f32 {
        DIG_OPTS.with(|d| d.borrow().iter().find(|e| e.0 == t).map_or(0.0, |e| e.2))
    }
    /// Fill [`DIG_OPTS`] for this decision (cleared first).
    pub fn dig_prepare(world: &World, organism: OrganismId, at: (i32, i32), opts: &[Opt], head: Option<&DigHead>) {
        DIG_OPTS.with(|d| {
            let mut d = d.borrow_mut();
            d.clear();
            let mut ctxf: Option<DigCtx> = None;
            let mut when = f32::NEG_INFINITY;
            for o in opts.iter().filter(|o| o.verb == Verb::Dig) {
                let c = *ctxf.get_or_insert_with(|| dig_ctx(world, organism, at));
                let fe = dig_feats(world, organism, at, o.target, c);
                let (bonus, w) = head.map_or((0.0, 0.0), |h| h.eval2(&o.feats, &fe));
                when = when.max(w);
                d.push((o.target, fe, bonus));
            }
            // `PP_DIGHEAD_GATE=1`: the "when" read-out only in the empty-jaw dig states the old ant was asked about
            // (no spoil, crop empty, below the datum, within 40 columns of the door; hunger not read, so the copier
            // can apply the same gate from the log). Head v3/v4 ungated: P(Dig) on unlabelled decisions 0.008 ->
            // 0.018 and home food 318 -> 248-275.
            // `PP_DIGGATE_DOOR` (sense-cap audit, 2026-10-24; **on by default**, `=0`/`off` for the old gate): "the
            // door" is the nearest nest site's column ([`dig_ctx`]'s own `nx`) instead of column 128. 128 is the
            // training bed's door, so with the old gate a lab colony landing anywhere else (the lab boxes land at 256;
            // the owner's 1024-wide box at four columns) never got the head's "when" read-out at all. Bit-identical on
            // the bed (events.csv hash, 10k frames); lab, food 25 out, 2 seeds: alive 159/144 -> 199/200.
            let door = if door_gate_on() { ctxf.map_or(128, |c| c.0) } else { 128 };
            let gated = dig_gate_on()
                && !world.organism(organism).is_some_and(|s| {
                    s.spoil.is_none()
                        && s.crop.is_none_or(|c| c.cells == 0)
                        && (at.0 - door).abs() <= 40
                        && world.room_surface_at(at.0).is_some_and(|top| at.1 > top)
                });
            let add = match head {
                Some(h) if !h.e.is_empty() && when.is_finite() && !gated => h.c2 + when,
                _ => 0.0,
            };
            DIG_WHEN.with(|w| w.set(add));
        });
    }
    fn open_cell(world: &World, organism: OrganismId, x: i32, y: i32) -> bool {
        let c = world.get(x, y);
        let _ = organism;
        c.material == crate::sim::material::EMPTY
            || (c.organism_id() != 0 && world.materials.kind(c.material) == crate::sim::material::MaterialKind::Creature)
    }
    /// Above (x, y): (ground cells before the first open one, up to 8; whether the column is open to the sky up to
    /// 2 rows above the datum).
    fn column_above(world: &World, organism: OrganismId, x: i32, y: i32) -> (i32, bool) {
        let top = world.room_surface_at(x).map_or(0, |s| s - 2);
        let mut roof = 0;
        let mut yy = y - 1;
        while yy >= top.max(0) && !open_cell(world, organism, x, yy) && roof < 8 {
            roof += 1;
            yy -= 1;
        }
        let mut sky = roof == 0;
        if sky {
            let mut yy = y - 1;
            while yy >= top.max(0) {
                if !open_cell(world, organism, x, yy) {
                    sky = false;
                    break;
                }
                yy -= 1;
            }
        }
        (roof, sky)
    }
    /// What the dig features share: the door column, the ant's own depth, whether it stands sky-open, the open
    /// share of the 7x7 round it, and (`PP_DIGROOM`) the room distance, else 0.
    pub type DigCtx = (i32, f32, f32, f32, f32);
    /// `PP_DIGROOM=1` (chambersense lane, 2026-10-30; **opt-in, off by default**): dig feature slot 8 -- the ant's own
    /// light, which no hand-set head reads (H3's weights there are 0) -- reads [`room_dist`] / 8 instead (0 inside a
    /// wide space, up to 3 at [`ROOM_REACH`] or none in reach). **Why**: H3 digs branching arms and never rooms. Its
    /// only sense of "how much space is here" is the open share of the 7x7 round the ant (slot 9), which reads the same
    /// at a tunnel's tip whether the tip is 2 cells from a chamber or 30, so no weight on it can say "widen here: this
    /// tip is far from any room" against "keep the passage narrow: a room is just behind you". The new reading is a
    /// world fact (how far, walking through open ground, to the nearest place wide enough to turn round in), not an
    /// instruction; the head's weights decide what to do with it. Layout-compatible: DIG_ND stays 12, so every
    /// genome and side file loads; only a head with weight on slot 8 reads differently, and only with the switch on.
    fn room_on() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var("PP_DIGROOM").is_ok_and(|v| v == "1" || v == "open" || v == "below"))
    }
    /// How far [`room_dist`] walks before it gives up (cells of path).
    pub const ROOM_REACH: i32 = 24;
    /// **The room sense**: the shortest 4-connected walk through open cells, from the ant's head, to the nearest "wide"
    /// cell -- one whose whole 5x5 block is open (the 5x5 opening `scratch/inrun/roomshape.py` calls a room). Ants and
    /// their bodies count open (as [`open_cell`]); crumbs, spoil, stored food and soil do not. So it reads 0 inside a
    /// room, a crater or the open air, and grows along a passage with every cell walked from the last such space.
    /// [`ROOM_REACH`] when none is in reach. Cost: a flood of at most the open cells within ROOM_REACH of the ant, only
    /// on decisions with a Dig option, stopping at the first wide cell (inside a room, at once).
    /// **The open air counts as a wide space** (`PP_DIGROOM=below` walks and counts only below the datum, the first
    /// version): with the sky not counted, a young colony has no room anywhere, every ant reads "far from any room"
    /// from frame 0, and the hand head widened the door into a pit (garden worlds 61-62, round 0: open rooms 0 and 0
    /// against H3+eat's 2 and 3, a pink crater at the door in both pictures). With the sky counted, a passage is "far"
    /// only once it has run far from the surface or from a room -- which is where a chamber should go.
    ///
    /// **Loose fill counts as dug** (`PP_DIGROOM=1`; `=open` is the strict version, `=below` strict and underground
    /// only): crumbs and stored food are walked through and count toward a wide block, as `scratch/nest/selrun/
    /// tunnels.py`'s flood passes them. Strict, a crumb-packed nest reads "far from any room" from inside it, so the
    /// head digs harder exactly where the crumbs are, and the crumbs grow (garden worlds 61-62, round 0: every
    /// strict arm a crumb-packed cone, two of eight colonies dead). What an ant feels by touch: loose fill or solid soil.
    pub fn room_dist(world: &World, organism: OrganismId, at: (i32, i32)) -> i32 {
        type Mode = (bool, bool, [Option<crate::sim::material::MaterialId>; 2]);
        static MODE: OnceLock<Mode> = OnceLock::new();
        let (below_only, strict, fill) = *MODE.get_or_init(|| {
            let v = std::env::var("PP_DIGROOM").unwrap_or_default();
            (v == "below", v == "below" || v == "open", [world.materials.id_of("crumbs"), world.materials.id_of("provisions")])
        });
        let below = |x: i32, y: i32| !below_only || world.room_surface_at(x).is_some_and(|s| y > s);
        let dug = |x: i32, y: i32| open_cell(world, organism, x, y) || (!strict && fill.contains(&Some(world.get(x, y).material)));
        let open = |x: i32, y: i32| below(x, y) && dug(x, y);
        let wide = |x: i32, y: i32| (-2..=2).all(|dx| (-2..=2).all(|dy| open(x + dx, y + dy)));
        let mut seen = std::collections::HashSet::new();
        let mut q = std::collections::VecDeque::new();
        q.push_back((at, 0));
        seen.insert(at);
        while let Some(((x, y), d)) = q.pop_front() {
            if wide(x, y) {
                return d;
            }
            if d >= ROOM_REACH {
                continue;
            }
            for n in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                if !seen.contains(&n) && open(n.0, n.1) {
                    seen.insert(n);
                    q.push_back((n, d + 1));
                }
            }
        }
        ROOM_REACH
    }
    /// The room sense's readings over a run, by band of walk (0, 1-4, 5-8, 9-12, 13-16, 17-23, 24+): what share of
    /// dig decisions a head's weights on it could ever act in. `room_hist_report` prints it.
    pub static ROOM_HIST: [std::sync::atomic::AtomicU64; 7] = [const { std::sync::atomic::AtomicU64::new(0) }; 7];
    pub fn room_hist_report() -> String {
        let v: Vec<u64> = ROOM_HIST.iter().map(|c| c.load(Ordering::Relaxed)).collect();
        format!("room sense at dig decisions: 0 {} | 1-4 {} | 5-8 {} | 9-12 {} | 13-16 {} | 17-23 {} | 24+ {}", v[0], v[1], v[2], v[3], v[4], v[5], v[6])
    }
    fn dig_ctx(world: &World, organism: OrganismId, at: (i32, i32)) -> DigCtx {
        let nx = world.nest_sites.iter().min_by_key(|s| (s.x - at.0).abs()).map_or(128, |s| s.x);
        let depth = world.room_surface_at(at.0).map_or(0.0, |s| ((at.1 - s) as f32 / 8.0).clamp(-1.0, 3.0));
        let (_, sky) = column_above(world, organism, at.0, at.1);
        let mut open = 0;
        for dx in -3..=3 {
            for dy in -3..=3 {
                if open_cell(world, organism, at.0 + dx, at.1 + dy) {
                    open += 1;
                }
            }
        }
        let room = if room_on() {
            let d = room_dist(world, organism, at);
            let b = match d {
                0 => 0,
                1..=4 => 1,
                5..=8 => 2,
                9..=12 => 3,
                13..=16 => 4,
                17..=23 => 5,
                _ => 6,
            };
            ROOM_HIST[b].fetch_add(1, Ordering::Relaxed);
            d as f32 / 8.0
        } else {
            0.0
        };
        (nx, depth, f32::from(u8::from(sky)), open as f32 / 49.0, room)
    }
    /// The Dig option's features ([`DIG_ND`]): 0 away from the door (+1) or toward it (-1) in columns; 1 down (+1)
    /// or up (-1); 2 the target's depth below the datum / 8; 3 its column open to the sky; 4 ground over it / 8;
    /// 5 its open 8-neighbours / 8; 6 its distance from the door / 16; 7 the ant's depth / 8; 8 the ant stands
    /// sky-open (`PP_DIGROOM=1`: the room distance / 8, [`room_dist`]); 9 the open share of the 7x7 round the ant; 10 a level cut; 11 target row minus the lowest open
    /// row within 3 of the head / 4 (the Teacher lane's bottom sense).
    /// **Light** (owner, 2026-10-23: replace the exact soil count with "a more general light sense that ants actually
    /// have"): daylight at (x, y), halved by every non-open cell between it and the surface (2 rows above the datum),
    /// so 1 under open sky, 0.5 under one cell of roof, ~0.016 under the 6-row crust. Day and night are not modelled.
    fn light_at(world: &World, organism: OrganismId, x: i32, y: i32) -> f32 {
        let top = world.room_surface_at(x).map_or(0, |s| s - 2).max(0);
        let shade = (top..y).filter(|&yy| !open_cell(world, organism, x, yy)).count().min(12);
        0.5f32.powi(shade as i32)
    }
    fn nosky_on() -> bool {
        static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ON.get_or_init(|| std::env::var("PP_DIGNOSKY").is_ok_and(|v| v == "1"))
    }
    /// `PP_DIGLIGHT`: on by default (owner, 2026-10-24; the light sense the owner asked for on 2026-10-23 in place of
    /// the exact soil count); `=off` (or `0`) reads the sky test and the count again.
    fn light_on() -> bool {
        static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ON.get_or_init(|| options::switch_on("PP_DIGLIGHT"))
    }
    pub fn dig_feats(world: &World, organism: OrganismId, at: (i32, i32), t: (i32, i32), c: DigCtx) -> [f32; DIG_ND] {
        let (nx, adepth, asky, aopen, room) = c;
        // PP_DIGLIGHT=1: slots 3, 4 and 8 read light instead of the sky test and the exact soil count (up to 8): light
        // at the cut where the sky test was, darkness (1 - light) where the count was, so dh7's weights keep their
        // sign, and the ant's own light where its own sky test was.
        let asky = if light_on() { light_at(world, organism, at.0, at.1) } else { asky };
        // PP_DIGNOSKY=1 (owner, 2026-10-23: "depth should be the only thing they need"): slots 3, 4 and 8 -- the sky
        // test, the soil count or light -- read 0, so the head keeps depth, door distance, direction and surroundings.
        let blank = nosky_on();
        let out = ((t.0 - nx).abs() - (at.0 - nx).abs()).signum() as f32;
        let down = (t.1 - at.1).signum() as f32;
        let tdepth = world.room_surface_at(t.0).map_or(0.0, |s| ((t.1 - s) as f32 / 8.0).clamp(-1.0, 3.0));
        let (roof, sky) = column_above(world, organism, t.0, t.1);
        let mut nb = 0;
        for dx in -1..=1 {
            for dy in -1..=1 {
                if (dx, dy) != (0, 0) && open_cell(world, organism, t.0 + dx, t.1 + dy) {
                    nb += 1;
                }
            }
        }
        let low = cavity_low(world, organism, at, 3);
        [
            out,
            down,
            tdepth,
            if blank { 0.0 } else if light_on() { light_at(world, organism, t.0, t.1) } else { f32::from(u8::from(sky)) },
            if blank { 0.0 } else if light_on() { 1.0 - light_at(world, organism, t.0, t.1) } else { roof as f32 / 8.0 },
            nb as f32 / 8.0,
            ((t.0 - nx).abs() as f32 / 16.0).min(3.0),
            adepth,
            if room_on() {
                room
            } else if blank {
                0.0
            } else {
                asky
            },
            aopen,
            f32::from(u8::from(t.1 == at.1)),
            ((t.1 - low) as f32 / 4.0).clamp(-2.0, 2.0),
        ]
    }
    pub fn copy_on() -> bool {
        copy_out().is_some()
    }
    /// One row into the copy log, prefixed with this thread's world tag.
    pub fn cline(s: &str) {
        if shadow_on() && CAPTURE.with(|c| c.borrow().is_some()) {
            CAPTURE.with(|c| c.borrow_mut().as_mut().unwrap().push_str(s));
            return;
        }
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
                let mut nz: Vec<String> = op.feats.iter().enumerate().filter(|(_, x)| **x != 0.0).map(|(i, x)| format!("{i}={x}")).collect();
                // SCRATCH (dig-scoring thread): the dig head's own features as indices 100+k (`PP_DIGFEAT`)
                if op.verb == Verb::Dig && digfeat_on() {
                    DIG_OPTS.with(|dd| {
                        if let Some(e) = dd.borrow().iter().find(|e| e.0 == op.target) {
                            nz.extend(e.1.iter().enumerate().filter(|(_, x)| **x != 0.0).map(|(i, x)| format!("{}={x}", 100 + i)));
                        }
                    });
                }
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
        let lows: Vec<i32> = BOTTOM_REACH.iter().map(|&r| cavity_low(world, organism, at, r)).collect();
        let bottom: Vec<String> = opts
            .iter()
            .map(|op| if op.verb == Verb::Dig { lows.iter().map(|l| (op.target.1 - l).to_string()).collect::<Vec<_>>().join(" ") } else { "-".into() })
            .collect();
        let carry = CARRY_LAB.with(|c| std::mem::replace(&mut *c.borrow_mut(), "-".into()));
        // SCRATCH (maze thread, 2026-10-22; ported): the recruitment-trail teacher's label shares the 16th field,
        // `hp|i;j` as the copier reads it, when the carry teacher has none. `PP_TRAIL_LABEL=hp[:spread]`.
        let carry = if carry != "-" {
            carry
        } else {
            trail_label().map_or_else(
                || "-".to_string(),
                |(hp, _)| trail_up(world, organism, ctx, opts).map_or_else(|| "-".to_string(), |up| format!("{hp}|{}", up.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(";"))),
            )
        };
        // The crater refusal's label as a 17th field (teacher lane's format): the option indices `PP_RIMVETO` would
        // take off, whatever the brain picked; `-` when none or the refusal is off.
        let refused = rimveto().map_or_else(|| "-".to_string(), |(n, nx, nr)| {
            let r: Vec<String> = (0..opts.len()).filter(|&i| rim_cut(world, &opts[i], n, nx, nr)).map(|i| i.to_string()).collect();
            if r.is_empty() { "-".to_string() } else { r.join(";") }
        });
        let row = |settle: u8| {
            format!(
                "Q,{},{organism},{},{},{},{},{},{},{lab},{},{},{},{},{carry},{refused}",
                world.frame,
                at.0,
                at.1,
                settle,
                d.pick,
                last.map_or(-1, |v| v as i32),
                u8::from(carrying),
                f(ctx),
                f(mem0),
                all.join(";"),
                bottom.join(";")
            )
        };
        // `PP_COPY_SAMPLE=k[:m]` (the hungry lane's sampler): keep one in m labelled rows (default every one) and one
        // in k of the rest, each kept row preceded by the same ant's previous decision marked settle=2 (memory context
        // only, never a training row), so the copier's one-step memory path still links a kept row to the one before.
        let (k, m) = sample_km();
        let settle = u8::from(SETTLE.with(|s| s.get()));
        if k <= 1 && m <= 1 {
            cline(&row(settle));
            return;
        }
        let r = rng::stream(world.seed, organism as u64, world.frame, 4444).next_u64();
        let keep = if carry != "-" || refused != "-" { r.is_multiple_of(m) } else { r.is_multiple_of(k) };
        PREV_ROW.with(|pm| {
            let mut pm = pm.borrow_mut();
            if keep {
                if let Some((written, prev)) = pm.get(&organism) {
                    if !*written {
                        cline(prev);
                    }
                }
                cline(&row(settle));
            }
            pm.insert(organism, (keep, row(2)));
        });
    }
    thread_local! {
        pub static CARRY_LAB: std::cell::RefCell<String> = std::cell::RefCell::new("-".into());
        static PREV_ROW: std::cell::RefCell<HashMap<OrganismId, (bool, String)>> = std::cell::RefCell::new(HashMap::new());
    }
    /// The recruitment-trail teacher's steps (maze thread, 2026-10-22; ported from trail-teacher-vs-ee38ae5b.patch):
    /// for an ant with an empty crop and empty jaws, away from the nest, with 2+ Walk options whose B there-minus-here
    /// differs by at least the label's `spread`, the steps up B.
    pub fn trail_up(world: &World, organism: OrganismId, ctx: &[f32; CONTEXT], opts: &[Opt]) -> Option<Vec<usize>> {
        let (_, spread) = trail_label()?;
        let empty = world.organism(organism).is_some_and(|s| s.crop.map_or(0, |c| c.cells) == 0 && s.spoil.is_none());
        let walks: Vec<usize> = (0..opts.len()).filter(|&i| opts[i].verb == Verb::Walk).collect();
        if !empty || ctx[c::AT_NEST] > 0.0 || walks.len() < 2 {
            return None;
        }
        // `PP_TRAIL_OUT=1`: the second rule (after the first one trapped climbers on a runaway B peak short of the
        // food): stay on the trail -- Walk options whose B there is at least half the best -- and of those take the
        // step farthest from the nest's column (outward). No trail in reach (best B there < 0.05): abstain.
        if trail_out() {
            let bt = |i: usize| opts[i].feats[options::f::SCENT + 3];
            let best = walks.iter().map(|&i| bt(i)).fold(f32::NEG_INFINITY, f32::max);
            if best < 0.05 {
                return None;
            }
            let on: Vec<usize> = walks.iter().copied().filter(|&i| bt(i) >= 0.5 * best).collect();
            let out = |i: usize| (opts[i].target.0 - 128).abs();
            let far = on.iter().map(|&i| out(i)).max()?;
            return Some(on.into_iter().filter(|&i| out(i) == far).collect());
        }
        let bd = |i: usize| opts[i].feats[options::f::SCENT + 4];
        let hi = walks.iter().map(|&i| bd(i)).fold(f32::NEG_INFINITY, f32::max);
        let lo = walks.iter().map(|&i| bd(i)).fold(f32::INFINITY, f32::min);
        (hi - lo >= spread).then(|| walks.iter().copied().filter(|&i| bd(i) == hi).collect())
    }
    fn trail_out() -> bool {
        static O: OnceLock<bool> = OnceLock::new();
        *O.get_or_init(|| std::env::var("PP_TRAIL_OUT").is_ok())
    }
    pub fn trail_act() -> bool {
        static A: OnceLock<bool> = OnceLock::new();
        *A.get_or_init(|| std::env::var("PP_TRAIL_ACT").is_ok())
    }
    fn trail_label() -> Option<(f32, f32)> {
        static T: OnceLock<Option<(f32, f32)>> = OnceLock::new();
        *T.get_or_init(|| {
            let v = std::env::var("PP_TRAIL_LABEL").ok()?;
            let p: Vec<f32> = v.split(':').map(|x| x.parse().expect("PP_TRAIL_LABEL hp[:spread]")).collect();
            Some((p[0], p.get(1).copied().unwrap_or(0.2)))
        })
    }
    fn sample_km() -> (u64, u64) {
        static K: OnceLock<(u64, u64)> = OnceLock::new();
        *K.get_or_init(|| {
            std::env::var("PP_COPY_SAMPLE").ok().map_or((1, 1), |v| {
                let p: Vec<u64> = v.split(':').map(|x| x.parse().expect("PP_COPY_SAMPLE k[:m]")).collect();
                (p[0], p.get(1).copied().unwrap_or(1))
            })
        })
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
        // SCRATCH (evolvable-pheromones lane, 2026-10-25): `PP_MIND_LOG_EMITS=1` adds an `E` row per decision with
        // the three emit strengths (A, B, alarm), joined to its `M` row by seed, frame and id -- the signalling
        // census: what each emission depends on. A row of its own so every `M` parser keeps its layout.
        static EMITS: OnceLock<bool> = OnceLock::new();
        if *EMITS.get_or_init(|| std::env::var("PP_MIND_LOG_EMITS").is_ok_and(|v| v == "1")) {
            line(format!("E,{seed},{frame},{id},{}", f(&d.emits)));
        }
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
