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
//! **The verbs come in one at a time.** Walk, turn around and rest act now,
//! because the first rung of the ladder is the trail; the others are listed
//! and described by `options`, but [`Verb::acts`] keeps them out of every
//! pick until their act lands, whatever the switch genes say.
//!
//! **A blank mind picks uniformly**: every weight 0 scores every option the
//! same, so the job is a fair draw among the verbs with a target and the
//! target a fair draw among that verb's. That is the random null each rung
//! is measured against, not a do-nothing control (ladder lane, 2026-10-17).
//!
//! **No transcendental anywhere a decision reads** (`brain.rs`): squashes
//! are `x / (1 + |x|)`, and the one draw per pick comes from a stream keyed
//! on the seed, the ant and the frame.

use super::options::{self, Opt, Verb, FEATURES, VERBS};
use super::{
    adjacent_nest, armour_of, bite_force_of, blend_with_nest, carried_cells, choose_weighted, commit_step,
    dig_force_of, fall_now_if_unsupported, flipped_body, is_sheltered, live_body_cells, organism_crop_capacity,
    parting_enabled, relocate_chain, segment_authored, settle, traits_of, under_cover, BodyShape, BodySide,
    CreatureDef, DIRS,
};
use crate::sim::cell::OrganismId;
use crate::sim::pheromone::{self, Channel};
use crate::sim::rng;
use crate::sim::scheduler::ActiveSite;
use crate::sim::world::Account;
use crate::sim::World;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// Units in the shared scorer.
pub const UNITS: usize = 16;
/// Memory units, each with its own forgetting rate.
pub const MEMORY: usize = 12;
/// Scents the ant can lay alongside its verb: A, B and alarm. The building
/// scent comes later, with the dig rung.
pub const EMITS: usize = 3;
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
    /// Rows below the ant's last surface row.
    pub const DEPTH: usize = 18;
    /// Ground overhead.
    pub const ROOFED: usize = 19;
}

/// Context inputs.
pub const CONTEXT: usize = 20;

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
/// The whole genome.
pub const GENOME_LEN: usize = EMIT + EMITS * E_IN;

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

/// A genome, with what it costs to run: the connections it holds.
#[derive(Clone, Debug)]
pub struct MindGenome {
    pub w: Vec<f32>,
    /// Weights at or above [`W_EPS`], switches, rates and the two bounds
    /// excluded: what the body pays for every decision, priced like today's
    /// brain's active synapses (`CreatureDef::synapse_fraction`).
    pub live: u32,
}

impl MindGenome {
    pub fn new(w: Vec<f32>) -> MindGenome {
        assert_eq!(
            w.len(),
            GENOME_LEN,
            "a mind genome is exactly GENOME_LEN; a short one means the layout changed"
        );
        let live = w[S_CTX..].iter().filter(|v| v.abs() >= W_EPS).count() as u32;
        MindGenome { w, live }
    }
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
            i if i >= EMIT => emits[(i - EMIT) / E_IN],
            _ => true,
        })
        .collect()
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
    for (base, rows, width) in [(JOB, VERBS, J_IN), (MEM, MEMORY, M_IN), (EMIT, EMITS, E_IN)] {
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
        _ => return None,
    }
    Some(g)
}

/// **Which slots cross over together**: one scorer unit's whole fan-in and
/// its output weight; one job row; one memory row with its rate; one emit
/// row; each switch and bound on its own. The rig takes each group whole
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
        s => UNIT_BASE + 400 + (s - EMIT) / E_IN,
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
        "verbs [{}] sharp {:.2} pull {:.2} | live: scorer ctx {} feat {} verb {} out {}, job {}, memory {}, emit {} (of {})",
        on.join(","),
        sharpness(g),
        pull(g),
        live(S_CTX, S_FEAT),
        live(S_FEAT, S_VERB),
        live(S_VERB, S_OUT),
        live(S_OUT, JOB),
        live(JOB, MEM),
        live(MEM, EMIT),
        live(EMIT, GENOME_LEN),
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

/// The verbs whose act has landed ([`act`]); the rest are never picked.
impl Verb {
    pub fn acts(self) -> bool {
        matches!(self, Verb::Walk | Verb::TurnAround | Verb::Rest)
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
    /// Picks whose act did nothing (a step the body could not take after
    /// all). Should stay 0: the list is built from the body's own checks.
    pub failed: u64,
    /// Scent laid, in [`pheromone::DEPOSIT`] units.
    pub laid: [f64; EMITS],
}

/// **Every mind in the world**, by ant: a side table on the world, as the
/// needs walk keeps its own (`World::needs`), so the game ant's
/// `OrganismState` is untouched.
#[derive(Clone, Debug, Default)]
pub struct Minds {
    ants: HashMap<OrganismId, Mind>,
    pruned_at: u64,
    pub counts: MindCounts,
}

impl Minds {
    pub fn len(&self) -> usize {
        self.ants.len()
    }
    pub fn is_empty(&self) -> bool {
        self.ants.is_empty()
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
    let live: std::collections::HashSet<OrganismId> = world.live_organism_ids().into_iter().collect();
    if let Some(m) = world.minds.as_mut() {
        m.ants.retain(|id, _| live.contains(id));
        m.pruned_at = frame;
    }
}

// --- reading the ant --------------------------------------------------------

/// **What the ant feels**, once a decision: a pure read, with the change
/// senses taken against `state.last`. Returns the context and this
/// decision's values for the next one's change senses.
pub fn context(
    world: &World,
    organism: OrganismId,
    def: &CreatureDef,
    state: &MindState,
) -> ([f32; CONTEXT], [f32; 5]) {
    let mut v = [0.0f32; CONTEXT];
    let Some(st) = world.organism(organism) else {
        return (v, [0.0; 5]);
    };
    let Some(&(hx, hy)) = st.chain.first() else {
        return (v, [0.0; 5]);
    };
    let e = (st.energy / def.start_energy.max(f32::EPSILON)).max(0.0);
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
    v[c::AT_NEST] = f32::from(u8::from(adjacent_nest(world, hx, hy, def)));
    let scents = EMIT_CHANNELS.map(|ch| world.pheromone_at(ch, hx, hy) as f32 / pheromone::Scent::MAX as f32);
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
    v[c::DEPTH] = st.last_out_row.map_or(0.0, |s| squash((hy - s) as f32 / 16.0));
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
    for (o, opt) in opts.iter().enumerate() {
        if !on(opt.verb) {
            continue;
        }
        let v = opt.verb as usize;
        let mut raw = out[UNITS];
        for u in 0..UNITS {
            let feat = &g[S_FEAT + u * FEATURES..S_FEAT + (u + 1) * FEATURES];
            let mut z = pre[u] + g[S_VERB + u * VERBS + v];
            for (w, x) in feat.iter().zip(&opt.feats) {
                z += w * x;
            }
            raw += out[u] * squash(z);
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

    // The job: context, memory, the verb's best target, its offset, and
    // the pull towards last time's verb.
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
        if state.last_verb == Some(Verb::ALL[v]) {
            sum += pulled;
        }
        let j = squash(sharp * sum);
        jobs[v] = Some(j);
        weights[v] = (j + 1.0) / 2.0;
    }
    let present: Vec<usize> = (0..VERBS).filter(|&v| jobs[v].is_some()).collect();
    let shifted: Vec<f32> = present.iter().map(|&v| weights[v]).collect();
    let verb = Verb::ALL[present[choose_weighted(&shifted, PICK_FLOOR, draw.unit_f32())]];

    // Its target.
    let targets: Vec<usize> = (0..opts.len()).filter(|&o| opts[o].verb == verb).collect();
    let shifted: Vec<f32> = targets.iter().map(|&o| (scores[o] + 1.0) / 2.0).collect();
    let pick = targets[choose_weighted(&shifted, PICK_FLOOR, draw.unit_f32())];

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
            options::build(world, organism, def, &mut opts);
            let (ctx, now) = context(world, organism, def, &mind.state);
            let mut draw = rng::stream(world.seed, organism as u64, world.frame, RNG_SLOT_MIND);
            let d = decide(&mind.genome.w, &ctx, &opts, &mut mind.state, &mut draw);
            mind.state.last = Some(now);
            d.map(|d| (opts[d.pick], d.emits, opts.len()))
        });
        if let Some(m) = world.minds.as_mut() {
            m.counts.decisions += 1;
            if let Some(slot) = m.ants.get_mut(&organism) {
                slot.state = mind.state;
            }
        }
        if let Some((opt, emits, n)) = decided {
            let did = act(world, organism, def, opt);
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
    // No brain outputs gate birth: what the body's own bars allow, it does.
    settle(world, organism, def, (hx, hy), colony, at_nest, spent, 0.0, 0.0)
}

/// **Do the chosen option by the world's rules.** `false` when the body
/// could not after all; the list is built from the same checks, so that is
/// a bug to count ([`MindCounts::failed`]), not a choice.
fn act(world: &mut World, organism: OrganismId, def: &CreatureDef, opt: Opt) -> bool {
    match opt.verb {
        Verb::Rest => true,
        Verb::Walk => walk(world, organism, def, opt.target),
        Verb::TurnAround => turn_around(world, organism, def),
        _ => false,
    }
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
        // The same list with an off verb that acts (turning around) and two
        // that do not act yet (dig, drop) threaded through it.
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

    #[test]
    fn the_layout_adds_up_and_crosses_in_whole_rows() {
        assert_eq!(GENOME_LEN, EMIT + EMITS * E_IN);
        let mut groups: Vec<usize> = (0..GENOME_LEN).map(crossover_group).collect();
        groups.dedup();
        let n = groups.iter().collect::<std::collections::BTreeSet<_>>().len();
        // switches, sharp, pull; units; output bias; jobs; memory rows; emits
        assert_eq!(n, VERBS + 2 + UNITS + 1 + VERBS + MEMORY + EMITS, "crossover groups");
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
        assert_eq!(
            MindGenome::new(blank(&Verb::ALL)).live,
            0,
            "a blank mind costs nothing to run"
        );
    }
}
