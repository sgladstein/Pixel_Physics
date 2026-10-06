//! **The needs-and-jobs walk: a second way for an ant to decide.** Built
//! beside today's ant and off until a harness turns it on, so if it fails
//! nothing else is lost (owner, 2026-10-06). The design of record is the doc
//! "A needs-and-jobs ant: design for a second walk"
//! (<https://claude.ai/code/artifact/6f9b6730-a887-4697-8689-906afc9afb0a>).
//!
//! **The switch is [`World::needs`], set at a frame, never an environment
//! variable**: a variable is read once per process and so cannot hand a
//! running colony over at a frame, and a `Chooser` variant would enrol the
//! walk in the 69 `Chooser::` comparisons that already exist. `None` is
//! today's ant and every hook below is one `is_some` test.
//!
//! **A hook, not the split of `act` the design planned** (slice 0). The
//! engineering review was right that the brain's outputs reach about ten
//! places in the tick and that `act` interleaves its mechanics with the hand
//! rules that gate them. Splitting it would have moved code that seven live
//! branches were editing on the day this was built
//! (`branchcheck.sh --who-touched src/sim/creature.rs`, 2026-10-06), so the
//! walk instead **drives `act` itself**: it hands `act` the urges its drive
//! wants (1 for the verb it means, 0 for the ones it does not, the brain's
//! own for the verbs it does not own yet), and `act`'s mechanics and the
//! gates on where a load may lie or a cut may go run exactly as they do for
//! today's ant. That is the design's "haul and dig as jobs ... with their
//! own act gates" taken literally. What a need must never meet is a veto,
//! and the guard in this module's tests checks that by outcome.
//!
//! **The ten places the brain reaches, and what the walk does at each**
//! (review finding E1):
//!
//! | Site | Today | Under this walk |
//! |---|---|---|
//! | step or stay (`Move`) | the brain's roll | [`kinesis`] |
//! | forage drive and pace | lifts `Move` for a needed forager | overridden by [`kinesis`] |
//! | launch (`Impulse`) | the brain | unchanged: an ant authors no `Impulse` |
//! | tumble after a lost roll | the chooser pauses | unchanged: a lost roll is a pause |
//! | `act`'s six urges | the brain | [`decide`]: the drive's verbs, the brain's for share and bite |
//! | the step (persistence, turn, noise) | `chooser_step` | [`step`] |
//! | trail A and B | the brain | unchanged until the walk owns trail laying |
//! | synapse tax | the brain, which still runs | unchanged, so starvation compares like with like |
//! | trace row | `DecisionRow` | unchanged, plus the walk's own rows |
//! | speculative brain | every ant, every tick | unchanged: the brain still runs |
//!
//! **Pass-through** ([`NeedsMode::Passthrough`]) runs every hook and hands
//! today's value straight back. It is the control for the hooks themselves:
//! a colony switched to it at any frame must stay identical to one that never
//! was, frame for frame, map for map and ledger row for ledger row.

use super::*;

/// **Which walk the ants use once [`World::needs`] is set.**
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeedsMode {
    /// Every hook runs and hands today's value back unchanged: the identity
    /// control for the hooks.
    Passthrough,
}

impl NeedsMode {
    /// `passthrough`, as a harness names it. Anything else is refused rather
    /// than read as a default, so a typo cannot run the control under another
    /// arm's label.
    pub fn parse(raw: &str) -> NeedsMode {
        match raw.trim() {
            "passthrough" => NeedsMode::Passthrough,
            other => panic!("needs={other:?}: the walk's modes are passthrough"),
        }
    }
}

/// **The walk and everything it remembers**, held on [`World::needs`] so
/// nothing of it lives on `OrganismState`: the shipped ant's state, and so
/// its random stream, cannot be touched by it.
#[derive(Clone, Debug)]
pub struct NeedsWalk {
    pub mode: NeedsMode,
    /// The frame the walk was switched on, for the switch-time burn-in.
    pub since: u64,
}

impl NeedsWalk {
    pub fn new(mode: NeedsMode, frame: u64) -> NeedsWalk {
        NeedsWalk { mode, since: frame }
    }
}

/// What [`decide`] settled for this tick's `act`.
pub(super) enum Decided {
    /// Run `act` with these urges.
    Act([f32; brain::BRAIN_OUTPUTS]),
}

/// **Before `act`: what the walk wants done with the jaws and the mouth.**
/// Pass-through hands the brain's urges back as they are.
pub(super) fn decide(
    world: &mut World,
    _x: i32,
    _y: i32,
    _organism: OrganismId,
    _def: &CreatureDef,
    _inputs: &[f32; brain::BRAIN_INPUTS],
    outputs: &[f32; brain::BRAIN_OUTPUTS],
) -> Decided {
    match world.needs.as_ref().map(|n| n.mode) {
        Some(NeedsMode::Passthrough) | None => Decided::Act(*outputs),
    }
}

/// **Step or stay**: the chance this decision steps at all. Pass-through
/// hands today's chance back.
pub(super) fn kinesis(
    world: &World,
    _organism: OrganismId,
    _def: &CreatureDef,
    _head: (i32, i32),
    _inputs: &[f32; brain::BRAIN_INPUTS],
    p_move: f32,
) -> f32 {
    match world.needs.as_ref().map(|n| n.mode) {
        Some(NeedsMode::Passthrough) | None => p_move,
    }
}

/// **The step**, once the roll said step. Pass-through takes today's step:
/// the chooser's, or `step_chain`'s with the chooser off.
#[allow(clippy::too_many_arguments)]
pub(super) fn step(
    world: &mut World,
    organism: OrganismId,
    heading: u8,
    outputs: &[f32; brain::BRAIN_OUTPUTS],
    def: &CreatureDef,
    draw: &mut rng::Rng,
    chooser: Chooser,
) -> bool {
    match world.needs.as_ref().map(|n| n.mode) {
        Some(NeedsMode::Passthrough) | None => {
            if chooser == Chooser::Off {
                step_chain(world, organism, heading, outputs, def, draw)
            } else {
                chooser_step(world, organism, heading, outputs, def, draw, chooser)
            }
        }
    }
}

/// **After the move**: what the walk remembers of this tick. Pass-through
/// remembers nothing.
pub(super) fn after(world: &mut World, _organism: OrganismId, _def: &CreatureDef, _moved: bool) {
    match world.needs.as_ref().map(|n| n.mode) {
        Some(NeedsMode::Passthrough) | None => {}
    }
}
