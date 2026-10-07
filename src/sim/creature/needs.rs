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
//! and the guard in this module's tests checks that by outcome. The three
//! things a need does that no urge can ask `act` for -- eat a load held in
//! the jaws, put soil down where it closes no way, and cut out of a sealed
//! pocket -- are done here, before `act`, with `act`'s own helpers.
//!
//! **The ten places the brain reaches, and what the walk does at each**
//! (review finding E1):
//!
//! | Site | Today | Under this walk |
//! |---|---|---|
//! | step or stay (`Move`) | the brain's roll | [`kinesis`] |
//! | forage drive and pace | lifts `Move` for a needed forager | kept for the jobs, overridden for needs and idle by [`kinesis`] |
//! | launch (`Impulse`) | the brain | unchanged: an ant authors no `Impulse` |
//! | tumble after a lost roll | the chooser pauses | unchanged: a lost roll is a pause |
//! | `act`'s six urges | the brain | [`act`]: the drive's verbs, the brain's for the rest |
//! | the step (persistence, turn, noise) | `chooser_step` | [`step`]: the brain's persistence, turn and noise, the drive's pull and terms |
//! | trail A and B | the brain | unchanged until the walk owns trail laying |
//! | synapse tax | the brain, which still runs | unchanged, so starvation compares like with like |
//! | trace row | `DecisionRow` | unchanged, plus the walk's own [`WalkRow`] |
//! | speculative brain | every ant, every tick | unchanged: the brain still runs |
//!
//! **Pass-through** ([`NeedsMode::Passthrough`]) runs every hook and hands
//! today's value straight back. It is the control for the hooks themselves:
//! a colony switched to it at any frame must stay identical to one that never
//! was, frame for frame, map for map and ledger row for ledger row. **Checked
//! 2026-10-06** (`deeptrace`, 60k frames, the switch at 20k): seeds 1 and 2
//! shipped, and seed 1 under `NEEDS_FIRST`, `CARRY_HOME` and `NEST_REST`,
//! byte-identical in every map, ledger, colony, stats, hunger and event row
//! to the build before the hooks, switch off and pass-through alike. The
//! positive control: a build nudging any one of the three live hooks (the
//! urges halved, the step chance cut by a tenth, one extra draw before the
//! step) differs at the first map after the switch and at none before.
//!
//! **The walk** ([`NeedsMode::Walk`], slice 1). Every decision, in order:
//!
//! 1. **The needs.** Hunger is a smooth ramp of what the ant has, its energy
//!    and the food in its crop, from 0 at [`HUNGER_ONSET`] of its budget to
//!    1 at [`HUNGER_FULL`]: no cliff, so a mildly hungry ant drifts out and a
//!    starving one runs. It wins over the ant's job when its urge passes the
//!    job's hold, which is the job's own patience under a ceiling
//!    ([`HOLD_CEILING`]) set below hunger's urge at 30% of the budget, so no
//!    job can outvote an ant that is starving. A hungry ant eats what it
//!    holds, puts soil down where it closes no way, goes to food it senses,
//!    goes out when inside, and searches when outside. **Escape** is hunger
//!    that has stopped getting anywhere: an ant inside whose way out has made
//!    no progress for [`ESCAPE_STALL`] decisions, pushing at soil, cuts it,
//!    and goes on cutting while its only progress is into its own cuts.
//! 2. **The job**, when no need wins: haul while a pellet is in the jaws,
//!    carry home while a trip's food is in the crop, walk back to the face
//!    while it remembers one, forage while its forage stimulus holds, and
//!    otherwise idle. Foraging is taken with probability `r^2 / (1 + r^2)`
//!    of the ratio of its stimulus to the ant's own threshold (Bonabeau,
//!    Theraulaz & Deneubourg 1996): a small floor so fed ants still scout,
//!    and a leaky count of the ant's meetings with foragers coming home with
//!    food, so ants near a busy door are the ones sent out; it is braked by
//!    food lying uneaten where the ant unloads (the owner's "more than enough
//!    food", 2026-10-06).
//! 3. **The step**, by the winning drive: the pull it aims at (food, the way
//!    out, home, the door for soil, the face) with today's patience, and
//!    today's route, scouting and door-reader terms for an ant going out.
//!    An idle ant inside has no pull at all: **the kinesis** decides where
//!    it stops, its chance of stepping falling as it nears its own preferred
//!    depth ([`PREF_DEPTH`]), so idle ants gather deep without being pulled
//!    there. An idle ant outside, or in the spoil mound, walks home.
//!
//! **Labelled stand-ins** (the design's *Locality* table): depth is rows
//! below the founding ground, and the way out inside is the nest's way
//! (`NestWay`) below that ground and the mound's way above it. Both are
//! registry maps no ant could sense. The local replacements -- depth below
//! the door vector's origin, the door vector and the walls -- come later;
//! the door vector is kept already, and used where neither way reaches.
//!
//! **The no-veto guard** is this module's tests: an ant at 30% of its budget
//! in each of six places, holding each of three loads, in either role, must
//! eat, reach the open air or dig if sealed, before it starves. It reads what
//! happened in the scene, never a weight, and today's ant fails 16 of its 36
//! rows (`the_guard_on_todays_ant`).

use super::*;
use std::collections::HashMap;

/// **Which walk the ants use once [`World::needs`] is set.**
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeedsMode {
    /// Every hook runs and hands today's value back unchanged: the identity
    /// control for the hooks.
    Passthrough,
    /// The needs-and-jobs walk (slice 1).
    Walk,
}

impl NeedsMode {
    /// `passthrough` or `walk`, as a harness names it. Anything else is
    /// refused rather than read as a default, so a typo cannot run the
    /// control under another arm's label.
    pub fn parse(raw: &str) -> NeedsMode {
        match raw.trim() {
            "passthrough" => NeedsMode::Passthrough,
            "walk" => NeedsMode::Walk,
            other => panic!("needs={other:?}: the walk's modes are passthrough and walk"),
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
    /// Each ant's memory under the walk, made on its first decision.
    minds: HashMap<OrganismId, Mind>,
    /// When the memories of the dead were last let go.
    pruned_at: u64,
    /// **The trace**: every decision of every ant whose id is a multiple of
    /// this is kept in [`NeedsWalk::rows`] for a harness to drain. 0 keeps
    /// none.
    pub trace_every: u32,
    pub rows: Vec<WalkRow>,
    /// What the walk did, summed over every ant, for a harness to read.
    pub counts: WalkCounts,
    /// **The guard's positive control**: needs never win, so every ant does
    /// its job or idles whatever its hunger. Never set outside a test.
    pub veto_needs: bool,
}

impl NeedsWalk {
    pub fn new(mode: NeedsMode, frame: u64) -> NeedsWalk {
        NeedsWalk {
            mode,
            since: frame,
            minds: HashMap::new(),
            pruned_at: frame,
            trace_every: 0,
            rows: Vec::new(),
            counts: WalkCounts::default(),
            veto_needs: false,
        }
    }

    /// How many ants the walk is remembering (live ones, to within
    /// [`PRUNE_EVERY`] frames).
    pub fn minds(&self) -> usize {
        self.minds.len()
    }

    /// The drive and job of every ant the walk remembers, for a harness's
    /// census of who is doing what.
    pub fn census(&self) -> impl Iterator<Item = (OrganismId, Drive, Job)> + '_ {
        self.minds.iter().map(|(&id, m)| (id, m.drive, m.job))
    }

    /// **One ant's walk as it last decided**, for a harness that writes it
    /// beside the ant's position: `None` for an ant that has not decided
    /// under the walk yet.
    pub fn view(&self, organism: OrganismId) -> Option<WalkView> {
        let m = self.minds.get(&organism)?;
        Some(WalkView {
            drive: m.drive,
            job: m.job,
            hunger: m.hunger,
            hold: m.hold,
            forage: m.forage,
            threshold: FORAGE_THRESHOLD * m.jitter,
            pref: m.pref,
            p_move: m.p_move,
            stall: m.stall,
            meet: m.meet,
            glut: m.glut,
            door: m.door,
        })
    }
}

/// **What [`NeedsWalk::view`] shows of one ant**: its last decision's drive,
/// job, hunger and hold, its forage stimulus against its own threshold, its
/// preferred depth, its step chance, how long its way out has stalled, the
/// meetings and glut its stimulus is made of, and its door vector's origin.
#[derive(Clone, Copy, Debug)]
pub struct WalkView {
    pub drive: Drive,
    pub job: Job,
    pub hunger: f32,
    pub hold: f32,
    pub forage: f32,
    pub threshold: f32,
    pub pref: f32,
    pub p_move: f32,
    pub stall: u16,
    pub meet: f32,
    pub glut: f32,
    pub door: Option<(i32, i32)>,
}

/// **What wins a decision**: a need, the job, or idling. Set before `act`
/// and read by the kinesis and the step after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Drive {
    /// Hungry, with food in reach: go to it and eat.
    Eat,
    /// Hungry inside, nothing in reach: out by the way out.
    Out,
    /// Hungry inside and the way out has stalled against soil: cut.
    Escape,
    /// Hungry outside, nothing in reach: search, as a hungry scout does.
    Seek,
    /// On the forage job, empty, going out.
    Forage,
    /// Food from a trip in the crop: carry it home.
    Carry,
    /// A pellet in the jaws: take it out.
    Haul,
    /// Its pellet down, walking back to the face it cut.
    Dig,
    /// Idle outside or in the spoil mound: walk home.
    Home,
    /// Idle inside: no pull; the kinesis decides where it stops.
    #[default]
    Rest,
}

impl Drive {
    pub const ALL: [Drive; DRIVES] = [
        Drive::Eat,
        Drive::Out,
        Drive::Escape,
        Drive::Seek,
        Drive::Forage,
        Drive::Carry,
        Drive::Haul,
        Drive::Dig,
        Drive::Home,
        Drive::Rest,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Drive::Eat => "eat",
            Drive::Out => "out",
            Drive::Escape => "escape",
            Drive::Seek => "seek",
            Drive::Forage => "forage",
            Drive::Carry => "carry",
            Drive::Haul => "haul",
            Drive::Dig => "dig",
            Drive::Home => "home",
            Drive::Rest => "rest",
        }
    }

    /// A need, as against a job or idling.
    pub fn is_need(self) -> bool {
        matches!(self, Drive::Eat | Drive::Out | Drive::Escape | Drive::Seek)
    }
}

pub const DRIVES: usize = 10;

/// **The ant's job**: what it does when no need wins. Kept across a need, so
/// an ant that went to eat goes back to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Job {
    #[default]
    Idle,
    Forage,
    Haul,
    Dig,
}

impl Job {
    pub fn label(self) -> &'static str {
        match self {
            Job::Idle => "idle",
            Job::Forage => "forage",
            Job::Haul => "haul",
            Job::Dig => "dig",
        }
    }
}

/// **What the walk did**, summed over every ant since the switch.
#[derive(Clone, Copy, Debug, Default)]
pub struct WalkCounts {
    /// Decisions won by each drive, in [`Drive::ALL`]'s order.
    pub decisions: [u64; DRIVES],
    /// Idle ants that took the forage job, and foragers home that quit it.
    pub took_forage: u64,
    pub quit_forage: u64,
    /// A need won over a job that was not idle.
    pub need_over_job: u64,
    /// Store loads a hungry ant put down to eat, and pellets put down.
    pub ate_held: u64,
    pub pellets_down: u64,
    /// Cuts made escaping, and pellets packed behind while escaping.
    pub escape_cuts: u64,
    pub escape_packs: u64,
    /// Foragers that unloaded beside food already lying uneaten.
    pub gluts: u64,
}

/// **One decision of a traced ant** ([`NeedsWalk::trace_every`]).
#[derive(Clone, Copy, Debug)]
pub struct WalkRow {
    pub frame: u64,
    pub id: OrganismId,
    /// The head when it decided, and after its step.
    pub at: (i32, i32),
    pub to: (i32, i32),
    pub drive: Drive,
    pub job: Job,
    pub energy: f32,
    pub crop: f32,
    pub hunger: f32,
    pub hold: f32,
    /// The forage stimulus and this ant's threshold for it.
    pub forage: f32,
    pub threshold: f32,
    /// Rows below the founding ground (the stand-in depth) and its own
    /// preferred depth.
    pub depth: i32,
    pub pref: f32,
    pub p_move: f32,
    pub moved: bool,
    pub stall: u16,
    /// Where the drive's pull aimed, if it had one.
    pub target: Option<(i32, i32)>,
}

/// **One ant's memory under the walk.** Set at its first decision from what
/// the shipped ant was doing ([`Mind::at_switch`], the switch-time table),
/// then kept here.
#[derive(Clone, Copy, Debug)]
struct Mind {
    job: Job,
    /// The frame the job was last taken or renewed.
    since: u64,
    /// Its own factor on every threshold, drawn once from its id.
    jitter: f32,
    /// Its preferred resting depth, rows below the founding ground.
    pref: f32,
    /// **The door vector's origin**: the last open-air cell its head stood
    /// in. `None` for an ant that has not been out since the switch.
    door: Option<(i32, i32)>,
    /// The way out's progress: which measure, the best it has reached, and
    /// how many decisions since it last got better.
    out_kind: u8,
    out_best: i32,
    stall: u16,
    /// Meetings with foragers home with food, leaking at [`MEET_TAU`].
    meet: f32,
    met_seen: u64,
    meet_at: u64,
    /// Food lying uneaten where it last unloaded, 1 fading at [`GLUT_TAU`].
    glut: f32,
    glut_at: u64,
    /// This decision's: the drive, its hunger and hold, the forage
    /// stimulus, food it sensed, the step chance.
    drive: Drive,
    hunger: f32,
    hold: f32,
    forage: f32,
    food: Option<(i32, i32)>,
    p_move: f32,
    /// Where it stood when it decided, for the trace.
    at: (i32, i32),
}

impl Mind {
    /// **The switch-time table**: what each item of the walk's memory is set
    /// to on an ant's first decision under it.
    ///
    /// | Item | Set from |
    /// |---|---|
    /// | job | haul with a pellet in the jaws; forage with food in the crop, or empty out in the open having foraged before; dig with a face to go back to; else idle |
    /// | threshold jitter, preferred depth | drawn from its id, once |
    /// | door vector | its head, if it stands in the open; else blank |
    /// | way-out progress | blank |
    /// | meetings | 1 if it met a forager home with food within the return window, else 0 |
    /// | food lying where it unloads | 0 |
    fn at_switch(
        world: &World,
        organism: OrganismId,
        st: &crate::sim::organism::OrganismState,
        head: (i32, i32),
        outside: bool,
    ) -> Mind {
        let laden = st.crop.is_some_and(|c| c.worth() > 0.0);
        let job = if st.spoil.is_some_and(|s| !s.store) {
            Job::Haul
        } else if laden || (outside && st.foraged) {
            Job::Forage
        } else if st.dig_return.is_some() {
            Job::Dig
        } else {
            Job::Idle
        };
        let mut r = rng::stream(world.seed, organism as u64, 0, RNG_SLOT_NEEDS_JITTER);
        let jitter = (JITTER * (2.0 * r.unit_f32() - 1.0)).exp();
        let pref = PREF_DEPTH + PREF_SPREAD * (2.0 * r.unit_f32() - 1.0);
        let met = st.return_met > 0 && (world.frame.saturating_sub(st.return_met) as f32) < return_window();
        Mind {
            job,
            since: world.frame,
            jitter,
            pref,
            door: outside.then_some(head),
            out_kind: OUT_NONE,
            out_best: i32::MAX,
            stall: 0,
            meet: if met { 1.0 } else { 0.0 },
            met_seen: st.return_met,
            meet_at: world.frame,
            glut: 0.0,
            glut_at: world.frame,
            drive: Drive::Rest,
            hunger: 0.0,
            hold: 0.0,
            forage: 0.0,
            food: None,
            p_move: f32::NAN,
            at: head,
        }
    }
}

/// The random stream for the walk's own rolls (taking a job), keyed on the
/// ant and the frame as the shipped streams are. Slots 0-11 are
/// `creature.rs`'s.
const RNG_SLOT_NEEDS: u64 = 12;
/// ...and the one each ant's jitter is drawn from, once, at frame 0.
const RNG_SLOT_NEEDS_JITTER: u64 = 13;

/// **Hunger's ramp**: 0 at and above this share of the budget (200 J), 1 at
/// and below [`HUNGER_FULL`], smooth between. The design's starting values;
/// newborns hatch at about half the budget, so they start mildly hungry.
pub const HUNGER_ONSET: f32 = 0.6;
pub const HUNGER_FULL: f32 = 0.25;

/// **The most a job can hold an ant against hunger**: below hunger's urge at
/// 30% of the budget (0.94), so a starving ant always leaves its job. A job
/// holds an ant by its patience times this, so a job that is stuck holds it
/// by nothing.
pub const HOLD_CEILING: f32 = 0.8;

/// Decisions on the way out with no progress, pushing at soil, before the
/// ant cuts: about 180 frames at the ant's six-frame decision. A hungry ant
/// at 30% has about 1,400 frames to live.
pub const ESCAPE_STALL: u16 = 30;

/// **The forage stimulus**: a floor a fed ant still feels, so fed scouts
/// leave a colony that has no returns to meet (the design's founding-bed
/// risk), and the leaky count of meetings with foragers home with food,
/// which leaks over the shipped return window. The floor takes an idle ant
/// at threshold 1 about once in 2,500 decisions (15,000 frames); one
/// meeting a window takes it about every other decision.
pub const FORAGE_FLOOR: f32 = 0.02;
pub const FORAGE_THRESHOLD: f32 = 1.0;
const MEET_TAU: f32 = 1_400.0;
/// A forager stays on its job at least this long after taking or renewing
/// it, so it is out of the door before it asks again.
const FORAGE_DWELL: u64 = 600;
/// **More than enough food** (the owner, 2026-10-06): a forager that
/// unloads with this many food cells already lying within 3 cells of its
/// head remembers a glut, which brakes its forage stimulus and fades over
/// [`GLUT_TAU`]. Real foragers regulate on their own crop's unloading, not on
/// a colony total (Greenwald, Baltiansky & Feinerman 2018, eLife 7:e31730).
const GLUT_FOOD: u32 = 6;
const GLUT_TAU: f32 = 6_000.0;
/// How hard a forager going out pushes out: today's `always` drive.
const FORAGE_WANT: f32 = 1.0;

/// Each ant's thresholds are scaled by `exp(JITTER * u)`, `u` uniform in
/// [-1, 1): a factor from 0.67 to 1.5. Mutation is off while measuring, so
/// without it every ant would be the founder's copy.
const JITTER: f32 = 0.405;

/// **The preferred resting depth**: rows below the founding ground, plus or
/// minus [`PREF_SPREAD`] by ant. "Deep" in the scorecard is more than 10.
pub const PREF_DEPTH: f32 = 14.0;
const PREF_SPREAD: f32 = 3.0;
/// **The kinesis**: an idle ant steps at [`P_SHALLOW`] when [`DEPTH_BAND`]
/// rows or more above its preferred depth, at [`P_DEEP`] at or below it, and
/// in between linearly. `P_DEEP` is a fed ant's step rate today (about 3% of
/// its decisions); `P_SHALLOW` is a guess the first trace checks.
const P_SHALLOW: f32 = 0.5;
const P_DEEP: f32 = 0.03;
const DEPTH_BAND: f32 = 6.0;
/// An idle ant outside or in the mound, walking home.
const P_HOME: f32 = 0.5;

/// The pull to food a hungry ant senses, twice the home pull (design).
const FOOD_GAIN: f32 = 2.0;
/// How far the food sense reaches, in cells (a 13x13 box, as `larva_scent`).
const FOOD_REACH: i32 = 6;
/// The pull out never falls below this share of the home pull, however
/// mildly hungry the ant.
const OUT_GAIN_MIN: f32 = 0.3;

/// Memories of ants no longer alive are let go this often.
const PRUNE_EVERY: u64 = 5_000;

/// Which measure the way out's progress is on ([`Mind::out_kind`]).
const OUT_NONE: u8 = 0;
const OUT_NEST_WAY: u8 = 1;
const OUT_MOUND_WAY: u8 = 2;
const OUT_DOOR: u8 = 3;
const OUT_UP: u8 = 4;

/// **Hunger's urge** at `have` (energy plus crop, as a share of the budget).
pub fn hunger_urge(have: f32) -> f32 {
    let t = ((HUNGER_ONSET - have) / (HUNGER_ONSET - HUNGER_FULL)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mind_of(world: &World, organism: OrganismId) -> Option<Mind> {
    world.needs.as_ref()?.minds.get(&organism).copied()
}

fn keep_mind(world: &mut World, organism: OrganismId, mind: Mind) {
    if let Some(n) = world.needs.as_mut() {
        n.minds.insert(organism, mind);
    }
}

/// **Before the move: what the jaws and the mouth do.** Pass-through runs
/// `act` on the brain's urges, as today's ant does.
#[allow(clippy::too_many_arguments)]
pub(super) fn act(
    world: &mut World,
    x: i32,
    y: i32,
    organism: OrganismId,
    def: &CreatureDef,
    inputs: &[f32; brain::BRAIN_INPUTS],
    outputs: &[f32; brain::BRAIN_OUTPUTS],
    draw: &mut rng::Rng,
) -> Did {
    match world.needs.as_ref().map(|n| n.mode) {
        Some(NeedsMode::Passthrough) | None => super::act(world, x, y, organism, def, outputs, draw),
        Some(NeedsMode::Walk) => walk_act(world, (x, y), organism, def, inputs, outputs, draw),
    }
}

/// **Step or stay**: the chance this decision steps at all. Pass-through
/// hands today's chance back.
pub(super) fn kinesis(
    world: &mut World,
    organism: OrganismId,
    _def: &CreatureDef,
    head: (i32, i32),
    _inputs: &[f32; brain::BRAIN_INPUTS],
    p_move: f32,
) -> f32 {
    match world.needs.as_ref().map(|n| n.mode) {
        Some(NeedsMode::Passthrough) | None => p_move,
        Some(NeedsMode::Walk) => {
            let Some(mut mind) = mind_of(world, organism) else {
                return p_move;
            };
            let p = walk_kinesis(world, head, &mind, p_move);
            mind.p_move = p;
            keep_mind(world, organism, mind);
            p
        }
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
        Some(NeedsMode::Walk) => walk_step(world, organism, heading, outputs, def, draw),
    }
}

/// **After the move**: what the walk remembers of this tick. Pass-through
/// remembers nothing.
pub(super) fn after(world: &mut World, organism: OrganismId, _def: &CreatureDef, moved: bool) {
    match world.needs.as_ref().map(|n| n.mode) {
        Some(NeedsMode::Passthrough) | None => {}
        Some(NeedsMode::Walk) => walk_after(world, organism, moved),
    }
}

// --- the walk: act ------------------------------------------------------

/// **The needs, the job, and what the jaws do**, for one decision.
fn walk_act(
    world: &mut World,
    head: (i32, i32),
    organism: OrganismId,
    def: &CreatureDef,
    inputs: &[f32; brain::BRAIN_INPUTS],
    outputs: &[f32; brain::BRAIN_OUTPUTS],
    draw: &mut rng::Rng,
) -> Did {
    use brain::BrainOutput as O;
    prune(world);
    let frame = world.frame;
    let Some(st) = world.organism(organism) else {
        return super::act(world, head.0, head.1, organism, def, outputs, draw);
    };
    let start = def.start_energy.max(1.0);
    let crop_worth = st.crop.map_or(0.0, |c| c.worth().max(0.0));
    let hunger = hunger_urge((st.energy + crop_worth) / start);
    let spoil = st.spoil;
    let laden = crop_worth > 0.0 && !carries_lunch(world, st);
    let trip = st.trip_load;
    let face = st.dig_return.is_some();
    let return_met = st.return_met;
    let (home_patience, scout_patience) = (st.home_patience, st.scout_patience);
    let surface = world
        .nearest_nest_site(head.0, head.1)
        .map(|i| world.nest_sites[i].surface);
    let below = surface.is_some_and(|s| head.1 > s);
    let covered = under_cover(world, head.0, head.1);
    let outside = !below && !covered;
    let at_home = inputs[brain::BrainInput::AtNest as usize] > 0.0;
    let mut mind = mind_of(world, organism).unwrap_or_else(|| Mind::at_switch(world, organism, st, head, outside));
    mind.at = head;
    mind.food = None;
    mind.hunger = hunger;

    // --- memory ---------------------------------------------------------
    if outside {
        mind.door = Some(head);
    }
    let dt = frame.saturating_sub(mind.meet_at) as f32;
    mind.meet *= (-dt / MEET_TAU).exp();
    mind.meet_at = frame;
    if return_met > mind.met_seen {
        mind.meet += 1.0;
        mind.met_seen = return_met;
    }
    let dt = frame.saturating_sub(mind.glut_at) as f32;
    mind.glut *= (-dt / GLUT_TAU).exp();
    mind.glut_at = frame;
    let threshold = FORAGE_THRESHOLD * mind.jitter;
    mind.forage = (FORAGE_FLOOR + mind.meet) * (1.0 - mind.glut).max(0.0);

    // --- the job --------------------------------------------------------
    // Forced by what the ant holds or remembers first: a pellet is hauled,
    // a trip's food (or any food, outside the nest) carried home, a face
    // walked back to. Then foraging, by its stimulus against the threshold.
    let carrying_home = laden && (trip || !below);
    let job = if spoil.is_some_and(|s| !s.store) {
        Job::Haul
    } else if carrying_home {
        Job::Forage
    } else if face && spoil.is_none() && !laden {
        Job::Dig
    } else {
        match mind.job {
            Job::Forage if at_home && frame.saturating_sub(mind.since) >= FORAGE_DWELL => {
                // Home and empty: stay on while the stimulus holds half the
                // threshold (the design's hysteresis), else stop.
                if mind.forage >= 0.5 * threshold {
                    mind.since = frame;
                    Job::Forage
                } else {
                    world.needs.as_mut().expect("walking").counts.quit_forage += 1;
                    Job::Idle
                }
            }
            Job::Forage => Job::Forage,
            Job::Haul | Job::Dig | Job::Idle => {
                let r = mind.forage / threshold;
                let p = r * r / (1.0 + r * r);
                if rng::stream(world.seed, organism as u64, frame, RNG_SLOT_NEEDS).unit_f32() < p {
                    mind.since = frame;
                    world.needs.as_mut().expect("walking").counts.took_forage += 1;
                    Job::Forage
                } else {
                    Job::Idle
                }
            }
        }
    };
    if job != mind.job && matches!(job, Job::Haul | Job::Dig) {
        mind.since = frame;
    }
    mind.job = job;
    // **The job's hold**: its own patience under the ceiling. Idle holds
    // nothing.
    mind.hold = HOLD_CEILING
        * match (job, carrying_home) {
            (Job::Idle, _) => 0.0,
            (Job::Forage, false) => scout_patience,
            _ => home_patience,
        };
    let veto = world.needs.as_ref().is_some_and(|n| n.veto_needs);
    let need = !veto && hunger > 0.0 && hunger > mind.hold;
    if need && job != Job::Idle {
        world.needs.as_mut().expect("walking").counts.need_over_job += 1;
    }

    let mut urges = *outputs;
    let did = if need {
        hungry_act(
            world, head, organism, def, &mut mind, below, covered, outside, &mut urges,
        )
    } else {
        mind.drive = match job {
            Job::Haul => Drive::Haul,
            Job::Forage if laden => Drive::Carry,
            Job::Forage => Drive::Forage,
            Job::Dig => Drive::Dig,
            Job::Idle if below => Drive::Rest,
            Job::Idle => Drive::Home,
        };
        if mind.drive == Drive::Haul {
            // **Put it down where `act` lets it lie**: inside, the keep rule
            // holds it until it is out.
            urges[O::DropSpoil as usize] = 1.0;
            urges[O::Dig as usize] = 0.0;
        }
        None
    };
    world.needs.as_mut().expect("walking").counts.decisions[mind.drive as usize] += 1;
    let did = match did {
        Some(did) => did,
        None => {
            let delivered = world.creature_stats.trip_deliveries;
            let did = super::act(world, head.0, head.1, organism, def, &urges, draw);
            // **Unloaded beside food already lying uneaten**: more than
            // enough food, remembered.
            if world.creature_stats.trip_deliveries > delivered {
                let (hx, hy) = world
                    .organism(organism)
                    .and_then(|s| s.chain.first().copied())
                    .unwrap_or(head);
                if food_lying(world, (hx, hy), 3) >= GLUT_FOOD {
                    mind.glut = 1.0;
                    mind.glut_at = world.frame;
                    world.needs.as_mut().expect("walking").counts.gluts += 1;
                }
            }
            did
        }
    };
    keep_mind(world, organism, mind);
    did
}

/// **A need has won.** Eat what it holds, put soil down where it closes no
/// way, and pick the drive: food in reach, the way out inside (cutting when
/// it has stalled against soil), or the search outside. Returns what it did
/// itself, or `None` with `urges` set for `act`.
#[allow(clippy::too_many_arguments)]
fn hungry_act(
    world: &mut World,
    head: (i32, i32),
    organism: OrganismId,
    def: &CreatureDef,
    mind: &mut Mind,
    below: bool,
    covered: bool,
    outside: bool,
    urges: &mut [f32; brain::BRAIN_OUTPUTS],
) -> Option<Did> {
    use brain::BrainOutput as O;
    let spoil = world.organism(organism).and_then(|s| s.spoil);
    let in_mound = !below && covered;
    urges[O::Feed as usize] = 1.0;
    urges[O::Drop as usize] = 0.0;
    urges[O::Dig as usize] = 0.0;
    // Outside, a pellet goes down where `act` lets it lie; inside it is put
    // down here or kept.
    urges[O::DropSpoil as usize] = if outside { 1.0 } else { 0.0 };
    // **Eats what it holds**: a store load is put down beside the head and
    // `act`'s mouth takes it this same tick. No doorway rule: food put down
    // and eaten at once blocks no way.
    if let Some(load) = spoil.filter(|s| s.store) {
        if let Some(p) = food_down_site(world, head) {
            world.set(p.0, p.1, load.cell);
            if let Some(s) = world.organism_mut(organism) {
                s.spoil = None;
                s.spoil_ring = None;
            }
            world.needs.as_mut().expect("walking").counts.ate_held += 1;
            mind.drive = Drive::Eat;
            mind.food = Some(p);
            return None;
        }
    }
    let escaping = mind.stall >= ESCAPE_STALL && (below || in_mound);
    // **Lets go of soil safely**: where the cell left keeps the way open
    // (`need_drop_site`), and never in the mound's tunnels unless it is
    // shut in there.
    if let Some(load) = spoil.filter(|s| !s.store) {
        if !outside && (!in_mound || escaping) {
            if let Some((px, py)) = need_drop_site(world, head) {
                world.set(px, py, load.cell);
                if let Some(s) = world.organism_mut(organism) {
                    s.spoil = None;
                    s.spoil_ring = None;
                }
                world.creature_stats.spoil_dumped += 1;
                world.needs.as_mut().expect("walking").counts.pellets_down += 1;
            }
        }
    }
    if let Some(food) = sense_food(world, organism, def, head) {
        mind.drive = Drive::Eat;
        mind.food = Some(food);
        return None;
    }
    if outside {
        mind.drive = Drive::Seek;
        return None;
    }
    mind.drive = Drive::Out;
    if !escaping {
        return None;
    }
    // **Escape**: the way out has stalled. Cut toward it, or up, where the
    // cell ahead is soil -- a crowd ahead is waited out, not dug round.
    let aim = escape_aim(world, organism, head, mind);
    let (dx, dy) = DIRS[aim as usize];
    let ahead = world.get(head.0 + dx, head.1 + dy);
    if in_a_doorway(world, head) || !jaw_can_cut(world, def, organism, ahead) {
        return None;
    }
    mind.drive = Drive::Escape;
    if let Some(s) = world.organism_mut(organism) {
        s.heading = aim;
    }
    match world.organism(organism).and_then(|s| s.spoil) {
        Some(load) if !load.store => {
            // Still holding soil: walled in, it packs the pellet behind as it
            // cuts on; otherwise it found nowhere to put it down and waits.
            if walled_in(world, organism, head) && pack_behind(world, organism, def, head, load) {
                world.needs.as_mut().expect("walking").counts.escape_packs += 1;
                return Some(Did {
                    dug: 1,
                    packed: true,
                    ..Did::default()
                });
            }
            None
        }
        Some(_) => None,
        None => {
            if escape_cut(world, organism, def, head, aim) {
                world.needs.as_mut().expect("walking").counts.escape_cuts += 1;
                Some(Did {
                    dug: 1,
                    ..Did::default()
                })
            } else {
                None
            }
        }
    }
}

/// **Where a hungry ant puts a store load down to eat it**: an empty cell
/// beside the head, one with ground under it first.
fn food_down_site(world: &World, head: (i32, i32)) -> Option<(i32, i32)> {
    let empty = |p: &(i32, i32)| world.in_bounds(p.0, p.1) && world.get(p.0, p.1).material == material::EMPTY;
    let mut around = NEIGHBOURS_8.iter().map(|&(dx, dy)| (head.0 + dx, head.1 + dy));
    around
        .clone()
        .filter(empty)
        .find(|&p| drop_footing(world, p))
        .or_else(|| around.find(empty))
}

/// **The food sense**: the nearest cell this ant's mouth would take within
/// [`FOOD_REACH`], by Chebyshev distance and then scan order. Loose food and
/// plants; never a living animal, and never its own kind.
fn sense_food(world: &World, organism: OrganismId, def: &CreatureDef, head: (i32, i32)) -> Option<(i32, i32)> {
    let gut = gut_of(world, organism, def);
    let mut best: Option<(i32, (i32, i32))> = None;
    for dy in -FOOD_REACH..=FOOD_REACH {
        for dx in -FOOD_REACH..=FOOD_REACH {
            let (x, y) = (head.0 + dx, head.1 + dy);
            if !world.in_bounds(x, y) {
                continue;
            }
            let c = world.get(x, y);
            if c.material == material::EMPTY
                || is_living_kin(world, c, gut)
                || (c.organism_id() != 0 && is_animal_cell(world, c))
            {
                continue;
            }
            if diet_yield(world, c, gut.bias) <= EAT_YIELD_THRESHOLD {
                continue;
            }
            let d = dx.abs().max(dy.abs());
            if best.is_none_or(|(b, _)| d < b) {
                best = Some((d, (x, y)));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Loose food cells within `reach` of `head`: what a forager unloading sees
/// already lying there.
fn food_lying(world: &World, head: (i32, i32), reach: i32) -> u32 {
    let mut n = 0;
    for y in head.1 - reach..=head.1 + reach {
        for x in head.0 - reach..=head.0 + reach {
            if world.in_bounds(x, y) {
                let c = world.get(x, y);
                n += u32::from(c.organism_id() == 0 && food_value(world, c) > 0.0);
            }
        }
    }
    n
}

/// **Where the way out aims from here**, and which measure says whether a
/// step got nearer: the nest's way below the founding ground, the mound's
/// way in the mound (both labelled stand-ins), else the door vector's
/// origin, else straight up.
fn out_target(world: &World, organism: OrganismId, head: (i32, i32), mind: &Mind) -> (u8, Option<(i32, i32)>) {
    if let Some(p) = way_out_from(world, organism, head) {
        return (OUT_NEST_WAY, Some(p));
    }
    if let Some(p) = mound_way_out(world, organism, head) {
        return (OUT_MOUND_WAY, Some(p));
    }
    match mind.door {
        Some(d) if d != head => (OUT_DOOR, Some(d)),
        _ => (OUT_UP, None),
    }
}

/// [`REST_LOOKAHEAD`] steps out along the mound's way (`MoundOut`'s map),
/// as `mound_out_pull` reads it, without its switch or its hunger gate.
fn mound_way_out(world: &World, organism: OrganismId, head: (i32, i32)) -> Option<(i32, i32)> {
    let site = world.nearest_nest_site(head.0, head.1)?;
    let way = world.mound_ways.iter().find(|w| w.site == site)?;
    step_down_way(way, organism, head)
}

/// **How far out this ant is, on the measure `kind` names** -- smaller is
/// nearer the open air -- for the escape's stall.
fn out_score(world: &World, head: (i32, i32), kind: u8, mind: &Mind) -> i32 {
    let way = |ways: &[NestWay]| {
        world
            .nearest_nest_site(head.0, head.1)
            .and_then(|site| ways.iter().find(|w| w.site == site))
            .and_then(|w| w.at(head.0, head.1))
            .map_or(i32::MAX, i32::from)
    };
    match kind {
        OUT_NEST_WAY => way(&world.nest_ways),
        OUT_MOUND_WAY => way(&world.mound_ways),
        OUT_DOOR => mind
            .door
            .map_or(i32::MAX, |(dx, dy)| (head.0 - dx).abs().max((head.1 - dy).abs())),
        _ => head.1,
    }
}

/// **Which way an escaping ant cuts**: the heading nearest its way out's
/// target, or straight up with none.
fn escape_aim(world: &World, organism: OrganismId, head: (i32, i32), mind: &Mind) -> u8 {
    let (_, target) = out_target(world, organism, head, mind);
    let Some((tx, ty)) = target else { return UP_DIR };
    let (vx, vy) = ((tx - head.0) as f32, (ty - head.1) as f32);
    let mut best = (f32::NEG_INFINITY, UP_DIR);
    for d in 0..8u8 {
        let (dx, dy) = DIRS[d as usize];
        let c = (dx as f32 * vx + dy as f32 * vy) / DIR_LEN[(d & 1) as usize];
        if c > best.0 {
            best = (c, d);
        }
    }
    best.1
}

/// **An escape's cut**: `act`'s dig mechanics -- the cell becomes the pellet
/// in the jaws, tamped as spoil, the gallery lined -- without the dig's
/// vetoes (the heap cue, the face trip, the roof, the turn downward), which
/// are about where a nest may grow; this grows nothing, it gets an ant out.
fn escape_cut(world: &mut World, organism: OrganismId, def: &CreatureDef, head: (i32, i32), aim: u8) -> bool {
    let (dx, dy) = DIRS[aim as usize];
    let (tx, ty) = (head.0 + dx, head.1 + dy);
    let target = world.get(tx, ty);
    if !jaw_can_cut(world, def, organism, target) {
        return false;
    }
    let mut pellet = target;
    let ground = world.materials.get(target.material);
    let hauled = if crate::sim::update::spoil_footing() {
        ground.spoils_into.or(ground.packs_into)
    } else {
        ground.packs_into
    };
    if let Some(hauled) = hauled {
        pellet.material = hauled;
    }
    world.set(tx, ty, Cell::EMPTY);
    world.dug_cells.insert((tx, ty));
    if spoil_kept() {
        if let Some(s) = world.organism_mut(organism) {
            s.spoil = Some(Spoil {
                cell: pellet,
                store: false,
            });
        }
    }
    world.creature_stats.digs += 1;
    if let Some(s) = world.organism_mut(organism) {
        s.life.digs += 1;
    }
    line_burrow(world, tx, ty);
    true
}

/// Let go of the memories of ants no longer alive, every [`PRUNE_EVERY`]
/// frames.
fn prune(world: &mut World) {
    let frame = world.frame;
    let due = world.needs.as_ref().is_some_and(|n| frame >= n.pruned_at + PRUNE_EVERY);
    if !due {
        return;
    }
    let live: std::collections::HashSet<OrganismId> = world.live_organism_ids().into_iter().collect();
    if let Some(n) = world.needs.as_mut() {
        n.minds.retain(|id, _| live.contains(id));
        n.pruned_at = frame;
    }
}

// --- the walk: the kinesis ----------------------------------------------

/// **Step or stay, by drive.** The jobs keep the brain's chance (and the
/// forage drive's pace); an idle ant inside steps less the nearer it is to
/// its own preferred depth; an idle ant outside walks home at
/// [`P_HOME`]; a need lifts the ant's chance toward 1 as its hunger rises.
fn walk_kinesis(world: &World, head: (i32, i32), mind: &Mind, p_move: f32) -> f32 {
    let depth = depth_of(world, head);
    let rest = match depth {
        Some(d) if d > 0 => {
            let t = ((d as f32 - (mind.pref - DEPTH_BAND)) / DEPTH_BAND).clamp(0.0, 1.0);
            P_SHALLOW + (P_DEEP - P_SHALLOW) * t
        }
        _ => P_HOME,
    };
    match mind.drive {
        Drive::Escape => 1.0,
        Drive::Eat => rest + (1.0 - rest) * mind.hunger.max(0.5),
        Drive::Out | Drive::Seek => rest + (1.0 - rest) * mind.hunger,
        Drive::Forage | Drive::Carry | Drive::Haul | Drive::Dig => p_move,
        Drive::Home => p_move.max(P_HOME),
        Drive::Rest => rest,
    }
}

/// **Rows below the founding ground** (the depth stand-in), or `None` with
/// no nest.
fn depth_of(world: &World, head: (i32, i32)) -> Option<i32> {
    let i = world.nearest_nest_site(head.0, head.1)?;
    Some(head.1 - world.nest_sites[i].surface)
}

// --- the walk: the step -------------------------------------------------

/// **The step, by the winning drive** -- `chooser_step`'s machinery (usable
/// headings, the crossing, the weighted pick, the commit, both patiences)
/// with the drive choosing the pull and which of today's terms count.
fn walk_step(
    world: &mut World,
    organism: OrganismId,
    heading: u8,
    outputs: &[f32; brain::BRAIN_OUTPUTS],
    def: &CreatureDef,
    draw: &mut rng::Rng,
) -> bool {
    let Some(mind) = mind_of(world, organism) else {
        return chooser_step(world, organism, heading, outputs, def, draw, Chooser::TrailAway);
    };
    let Some((chain, groups, fates)) = world
        .organism(organism)
        .map(|s| (s.chain.clone(), s.segment_groups.clone(), s.fates))
    else {
        note_outcome(world, DecisionOutcome::NoBody);
        return false;
    };
    let Some(&(hx, hy)) = chain.first() else {
        note_outcome(world, DecisionOutcome::NoBody);
        return false;
    };
    let usable = usable_headings(world, organism, def);
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
    let push = parting_enabled();
    let stacker = stacker_of(world, organism);
    let crossing = if crossing_enabled() && !usable.contains(&heading) {
        trunk_crossing(world, def, body, heading, push, stacker)
    } else {
        None
    };
    if usable.is_empty() && crossing.is_none() {
        return step_chain(world, organism, heading, outputs, def, draw);
    }
    let drive = mind.drive;
    // **The carry home fills up before it walks** (`CarryHome`'s `fill`,
    // part of the carry here whatever that switch says).
    if drive == Drive::Carry
        && fills_before_walking(
            world,
            organism,
            def,
            (hx, hy),
            outputs[brain::BrainOutput::Feed as usize],
        )
    {
        world.creature_stats.carry_fills += 1;
        note_outcome(world, DecisionOutcome::Filling);
        return false;
    }
    // **The trip back to the face ends**, as `chooser_step` ends it.
    if let Some(s) = world
        .organism(organism)
        .filter(|s| s.dig_return.is_some() && s.spoil.is_none())
    {
        let site = s.dig_return.expect("filtered above");
        if dig_trip_over(world, def, s, (hx, hy), site) {
            if let Some(s) = world.organism_mut(organism) {
                s.dig_return = None;
            }
        }
    }
    let st = world.organism(organism).expect("live: its chain was just read");
    let pull: Option<((i32, i32), f32)> = match drive {
        Drive::Eat => mind.food.map(|t| (t, FOOD_GAIN)),
        Drive::Out | Drive::Escape => {
            let (_, target) = out_target(world, organism, (hx, hy), &mind);
            let gain = def.home_bias.max(1.0) * mind.hunger.max(OUT_GAIN_MIN);
            Some((target.unwrap_or((hx, hy - COVER_REACH)), gain))
        }
        Drive::Carry | Drive::Home => Some((home_target(world, st), def.home_bias.max(1.0))),
        Drive::Haul => soil_way_pull(world, organism, def, st, (hx, hy))
            .or_else(|| spoil_haul_target(world, st, (hx, hy)))
            .map(|t| (t, spoil_haul().unwrap_or(1.0))),
        Drive::Dig => dig_return_target(world, def, st).map(|t| (t, spoil_haul().unwrap_or(1.0))),
        Drive::Seek | Drive::Forage | Drive::Rest => None,
    };
    // The pull's memory, kept exactly as `chooser_step` keeps it: started
    // again at every new target, cleared with no pull.
    let patience = {
        let state = world.organism_mut(organism).expect("live: its chain was just read");
        match pull {
            Some((target, _)) if state.home_best_for != target => {
                state.home_best_for = target;
                state.home_best = f32::INFINITY;
                state.home_away = 0;
                state.home_patience = 1.0;
                state.home_search_loops = 0;
            }
            Some(_) => {}
            None => {
                state.home_best = f32::INFINITY;
                state.home_away = 0;
                state.home_patience = 1.0;
                state.home_search_loops = 0;
            }
        }
        state.home_patience
    };
    let home_cos = |d: u8| -> Option<f32> {
        let ((ax, ay), _) = pull?;
        let (vx, vy) = ((ax - hx) as f32, (ay - hy) as f32);
        let len = (vx * vx + vy * vy).sqrt();
        if len < 1.0 {
            return None;
        }
        let (dx, dy) = DIRS[d as usize];
        Some((dx as f32 * vx + dy as f32 * vy) / (len * DIR_LEN[(d & 1) as usize]))
    };
    let persist = brain::unit_scale(outputs[brain::BrainOutput::Persist as usize], PERSIST_MAX);
    let turn = outputs[brain::BrainOutput::Turn as usize];
    let k = CHOICE_EXPLORATION_K * brain::unit_scale(outputs[brain::BrainOutput::Tumble as usize], 2.0);
    let walk = traits_of(world, organism, def);
    let trail_gain = TRAIL_GAIN * walk_gain(&walk, organism::TRAIT_TRAIL_HOLD);
    let away_gain = AWAY_GAIN * walk_gain(&walk, organism::TRAIT_ROUTE_AWAY);
    let decay = patience_decay_of(&walk);
    let gain = pull.map_or(0.0, |(_, g)| g * walk_gain(&walk, organism::TRAIT_HOME_PULL) * patience);
    let laden = world
        .organism(organism)
        .is_some_and(|s| s.crop.is_some_and(|c| c.worth() > 0.0) && !carries_lunch(world, s));
    let trip = world.organism(organism).is_some_and(|s| s.trip_load);
    // **Going out**: an empty forager, or a hungry ant outside, reads
    // today's away, scouting and door terms against home.
    let going_out = matches!(drive, Drive::Seek | Drive::Forage) && !laden;
    let away_from = if going_out {
        world.organism(organism).map(|s| home_target(world, s))
    } else {
        None
    };
    let want = match drive {
        Drive::Seek => mind.hunger,
        Drive::Forage => mind.hunger.max(FORAGE_WANT),
        _ => 0.0,
    };
    let scout_w = if away_from.is_some() {
        scout_of(world) * walk_gain(&walk, organism::TRAIT_SCOUT) * want
    } else {
        0.0
    };
    let away_home_cos = |d: u8| -> Option<f32> {
        let (ax, ay) = away_from?;
        let (vx, vy) = ((ax - hx) as f32, (ay - hy) as f32);
        let len = (vx * vx + vy * vy).sqrt();
        if len < 1.0 {
            return None;
        }
        let (dx, dy) = DIRS[d as usize];
        Some((dx as f32 * vx + dy as f32 * vy) / (len * DIR_LEN[(d & 1) as usize]))
    };
    // The excursion's memory, as `chooser_step` keeps it.
    let (scout_patience, scout_home, scout_dark) = match away_from {
        Some((ax, _)) if scout_w > 0.0 => {
            let st = world.organism_mut(organism).expect("live: its chain was just read");
            if st.scout_for != away_from.expect("matched Some") {
                st.scout_for = away_from.expect("matched Some");
                st.scout_best = (hx - ax).abs() as f32;
                st.scout_patience = 1.0;
                st.scout_home = false;
                st.scout_lit = false;
                st.scout_dark = false;
                st.scout_e0 = st.energy;
            }
            (st.scout_patience, st.scout_home, st.scout_dark)
        }
        _ => (1.0, false, false),
    };
    let door = if going_out && pull.is_none() && !scout_home && food_trail_of(world).read {
        walk_door_read(world, organism, def, (hx, hy), want)
            .map(|(side, f)| (side, f * walk_gain(&walk, organism::TRAIT_DOOR_READ)))
    } else {
        None
    };
    let door_walk = door
        .and_then(|_| door_site(world, hx, hy))
        .map(|i| world.nest_sites[i].surface - 1);
    let door_term = |d: u8| -> f32 {
        let (Some((side, f)), Some(walk)) = (door, door_walk) else {
            return 0.0;
        };
        let (dx, dy) = DIRS[d as usize];
        if dx == side && hy + dy <= walk {
            f
        } else {
            0.0
        }
    };
    let planes = trail_planes(&walk, laden);
    let route = |d: u8| trail_presence(world, (hx, hy), d, planes);
    let spent = scout_home && scout_dark && scout_w > 0.0 && food_trail_of(world).giveup;
    // **The carry home is held by a route's contrast, not its level**
    // (`CarryHome`'s `turn`, part of the carry here).
    let options: Vec<u8> = usable.iter().copied().chain(crossing.map(|_| heading)).collect();
    let route_floor = if drive == Drive::Carry && trip {
        let f = options.iter().map(|&d| route(d)).fold(f32::INFINITY, f32::min);
        if f.is_finite() {
            f
        } else {
            0.0
        }
    } else {
        0.0
    };
    let hold = |d: u8| {
        if spent {
            1.0
        } else {
            1.0 + trail_gain * (route(d) - route_floor).max(0.0)
        }
    };
    let scout_cos = |d: u8| -> Option<f32> {
        let (ax, _) = away_from?;
        let vx = ax - hx;
        if vx == 0 {
            return None;
        }
        let (dx, _) = DIRS[d as usize];
        Some((dx * vx.signum()) as f32 / DIR_LEN[(d & 1) as usize])
    };
    let score = |d: u8| -> f32 {
        let rel = (d + 8 - heading) % 8;
        let side = match rel {
            1..=3 => turn.max(0.0),
            5..=7 => (-turn).max(0.0),
            _ => 0.0,
        };
        persist * TURN_PREF[rel.min(8 - rel) as usize] * hold(d)
            + side
            + home_cos(d).map_or(0.0, |c| gain * c)
            + if spent {
                0.0
            } else {
                away_home_cos(d).map_or(0.0, |c| -away_gain * route(d) * c)
            }
            + if scout_w <= 0.0 {
                0.0
            } else if spent {
                away_home_cos(d).map_or(0.0, |c| scout_w * c)
            } else if scout_home {
                away_home_cos(d).map_or(0.0, |c| scout_w * (1.0 - route(d)) * c)
            } else {
                scout_cos(d).map_or(0.0, |c| -scout_w * scout_patience * (1.0 - route(d)) * c)
            }
            + door_term(d)
    };
    let scores: Vec<f32> = options.iter().map(|&d| score(d)).collect();
    let pick = choose_weighted(&scores, k, draw.unit_f32());
    let picked_route = route(options[pick]);
    if let Some((side, _)) = door {
        if pick < usable.len() && DIRS[options[pick] as usize].0 == side {
            world.creature_stats.door_followed += 1;
        }
    }
    world.creature_stats.carry_turns += u64::from(route_floor > 0.0);
    if pick == usable.len() {
        let (to, thickness) = crossing.expect("the last option is the crossing only when there is one");
        let due = world.frame + u64::from(thickness) * organism_tick_interval(world, organism, def);
        if let Some(state) = world.organism_mut(organism) {
            state.crossing = Some(organism::Crossing {
                to,
                heading,
                due,
                thickness,
            });
        }
        world.creature_stats.crossings += 1;
        note_outcome(world, DecisionOutcome::Crossing);
        return false;
    }
    commit_step(
        world,
        organism,
        def,
        body,
        &authored,
        &authored_widths,
        heading,
        options[pick],
        push,
    );

    // Did that step take the scout further out? `chooser_step`'s memory.
    let bound = food_trail_of(world).giveup;
    let noreturn = food_trail_of(world).noreturn;
    if let (Some((ax, _)), true) = (away_from, scout_w > 0.0 && !scout_home) {
        let state = world.organism_mut(organism).expect("live: it just stepped");
        let nx = state.chain.first().map_or(hx, |c| c.0);
        let level = (nx - ax).abs() as f32;
        if bound && picked_route >= HUNGRY_ROUTE && level > f32::from(FORAGE_TRIP_MIN) {
            state.scout_lit = true;
        }
        let on_trail = picked_route > 0.0;
        let stranded = noreturn && state.energy < state.scout_e0 - state.energy;
        let dark_past_a_trail = bound && state.scout_lit && !on_trail && !stranded;
        if level > state.scout_best + PATIENCE_PROGRESS && !dark_past_a_trail {
            state.scout_best = level;
            state.scout_patience = (state.scout_patience + PATIENCE_RECOVER).min(1.0);
        } else {
            state.scout_patience *= decay;
            if state.scout_patience < SCOUT_GIVE_UP {
                state.scout_home = true;
                state.scout_dark = bound && !on_trail;
            }
        }
    }
    // Did that step close on the pull's target? `chooser_step`'s memory.
    if let Some(((ax, ay), _)) = pull {
        let state = world.organism_mut(organism).expect("live: it just stepped");
        let (nx, ny) = state.chain.first().copied().unwrap_or((hx, hy));
        let (vx, vy) = ((ax - nx) as f32, (ay - ny) as f32);
        let dist = (vx * vx + vy * vy).sqrt();
        if dist < state.home_best - PATIENCE_PROGRESS {
            state.home_best = dist;
            state.home_best_at = (nx, ny);
            state.home_away = 0;
            state.home_patience = (state.home_patience + PATIENCE_RECOVER).min(1.0);
        } else {
            state.home_patience *= decay;
            let (bx, by) = state.home_best_at;
            let away = (nx - bx).abs().max((ny - by).abs()).clamp(0, u16::MAX as i32) as u16;
            state.home_away = state.home_away.max(away);
            if state.home_away >= EXCURSION_CELLS && away <= 1 {
                state.home_patience = 1.0;
                state.home_away = 0;
            }
        }
    }
    true
}

/// **The door reader** (`door_read`), on the walk's own want rather than
/// `outward_want`'s: an empty ant going out, in a door box, reads which side
/// the food trail is on. The same stale gate and the same reach-6 sensors.
fn walk_door_read(
    world: &World,
    organism: OrganismId,
    def: &CreatureDef,
    (hx, hy): (i32, i32),
    want: f32,
) -> Option<(i32, f32)> {
    let site = door_site(world, hx, hy)?;
    let st = world.organism(organism)?;
    if st.spoil.is_some() || want <= 0.0 {
        return None;
    }
    let window = match food_trail_of(world).window {
        w if w > 0.0 => w,
        _ => return_window(),
    };
    let (last, window) = if forage_drive_of(world).need == ForageNeed::Met {
        (
            return_met_frame(st),
            window * walk_gain(&traits_of(world, organism, def), organism::TRAIT_RETURN_MEMORY),
        )
    } else {
        (world.nest_last_return.get(site).copied().unwrap_or(0), window)
    };
    if last == 0 || world.frame.saturating_sub(last) as f32 > window {
        return None;
    }
    let so = def.sensor_offset;
    let b_e = f32::from(world.pheromone_at(Channel::B, hx + so, hy));
    let b_w = f32::from(world.pheromone_at(Channel::B, hx - so, hy));
    let g = (b_e - b_w) / (b_e + b_w + TRAIL_HALF);
    if g == 0.0 {
        return None;
    }
    Some((if g > 0.0 { 1 } else { -1 }, food_trail_of(world).gain * want * g.abs()))
}

// --- the walk: after ------------------------------------------------------

/// **After the step**: the way out's progress, which is what escape reads,
/// and the trace row.
fn walk_after(world: &mut World, organism: OrganismId, moved: bool) {
    let Some(mut mind) = mind_of(world, organism) else {
        return;
    };
    let head = world
        .organism(organism)
        .and_then(|s| s.chain.first().copied())
        .unwrap_or(mind.at);
    if matches!(mind.drive, Drive::Out | Drive::Escape) {
        let (kind, _) = out_target(world, organism, head, &mind);
        let score = out_score(world, head, kind, &mind);
        if kind != mind.out_kind {
            mind.out_kind = kind;
            mind.out_best = score;
            mind.stall = 0;
        } else if score < mind.out_best {
            mind.out_best = score;
            // **Ground it cut itself is not the way getting anywhere**: an
            // escaping ant that stepped into its own cut stays escaping, so
            // it puts the pellet down and cuts on. Reset here, it waited out
            // the whole stall again after every cut, and a 30%-budget ant in
            // the guard's mound pocket cut three cells in 530 frames and
            // starved with its jaws full, two rows short of the air (traced
            // 2026-10-07, `trace_one_guard_row`).
            if mind.drive != Drive::Escape {
                mind.stall = 0;
            }
        } else {
            mind.stall = mind.stall.saturating_add(1);
        }
    } else {
        mind.out_kind = OUT_NONE;
        mind.out_best = i32::MAX;
        mind.stall = 0;
    }
    if let Some(n) = world.needs.as_ref() {
        if n.trace_every > 0 && organism.is_multiple_of(n.trace_every) {
            let st = world.organism(organism);
            let row = WalkRow {
                frame: world.frame,
                id: organism,
                at: mind.at,
                to: head,
                drive: mind.drive,
                job: mind.job,
                energy: st.map_or(f32::NAN, |s| s.energy),
                crop: st.and_then(|s| s.crop).map_or(0.0, |c| c.worth()),
                hunger: mind.hunger,
                hold: mind.hold,
                forage: mind.forage,
                threshold: FORAGE_THRESHOLD * mind.jitter,
                depth: depth_of(world, head).unwrap_or(i32::MIN),
                pref: mind.pref,
                p_move: mind.p_move,
                moved,
                stall: mind.stall,
                target: walk_target(world, organism, head, &mind),
            };
            world.needs.as_mut().expect("walking").rows.push(row);
        }
    }
    keep_mind(world, organism, mind);
}

/// Where the drive's pull aimed, for the trace: the same targets
/// `walk_step` pulls to, read again after the step.
fn walk_target(world: &World, organism: OrganismId, head: (i32, i32), mind: &Mind) -> Option<(i32, i32)> {
    let st = world.organism(organism)?;
    match mind.drive {
        Drive::Eat => mind.food,
        Drive::Out | Drive::Escape => out_target(world, organism, head, mind).1,
        Drive::Carry | Drive::Home => Some(home_target(world, st)),
        Drive::Haul => world.organism(organism).and_then(|s| spoil_haul_target(world, s, head)),
        Drive::Dig => st.dig_return,
        Drive::Seek | Drive::Forage | Drive::Rest => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;

    // --- the no-veto guard ----------------------------------------------------
    //
    // The design's *Measurement and acceptance* and the rule audit's §5 (2026-10-06):
    // an ant at 30% of its budget, in every place, holding every load, in either
    // role, must within a bounded number of frames have eaten, reached the open
    // air, or -- where nothing joins its pocket to the air -- be digging; and if it
    // let go of a load, the way out must still be open afterwards. **It asserts
    // what happened in the scene, never the sign of a weight**: the history's
    // vetoes were each a rule that let a weight stay positive while a gate
    // further down took the act away.

    /// The founding ground's top row: everything at or below it is ground.
    const GROUND: i32 = 40;

    /// How long a row has, in frames. An ant ticks about once in six frames
    /// and pays about a quarter of a joule a tick, so at 30% of the budget it
    /// has about 1,400 frames to live: a row that has met nothing by then has
    /// starved, and the bound only stops a row the ant survives.
    const BOUND: u64 = 3_000;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Place {
        /// Just inside the door, at the top of the founding shaft.
        Door,
        /// The founding chamber at the shaft's foot, 6-7 rows down.
        Chamber,
        /// A room 20-21 rows down, at the end of a shaft below the chamber.
        Deep,
        /// A tunnel through the spoil mound, open to the air at its west end.
        MoundTunnel,
        /// A closed pocket in the mound that nothing joins.
        MoundPocket,
        /// A closed pocket in the ground, 30 rows down, that nothing joins.
        Buried,
    }

    impl Place {
        const ALL: [Place; 6] = [
            Place::Door,
            Place::Chamber,
            Place::Deep,
            Place::MoundTunnel,
            Place::MoundPocket,
            Place::Buried,
        ];

        /// Where the ant is put down.
        fn at(self) -> (i32, i32) {
            match self {
                Place::Door => (61, 41),
                Place::Chamber => (58, 47),
                Place::Deep => (60, 61),
                Place::MoundTunnel => (40, 37),
                Place::MoundPocket => (49, 33),
                Place::Buried => (21, 71),
            }
        }

        /// Nothing joins it to the air, so digging is the need met.
        fn sealed(self) -> bool {
            matches!(self, Place::MoundPocket | Place::Buried)
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Load {
        Nothing,
        /// A pellet of dug soil in the jaws.
        Pellet,
        /// A cell of food picked up at home for the store, in the jaws.
        Food,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Role {
        Forager,
        /// Nest-bound for life, as one ant in four is today (`is_nest_bound`).
        NestWorker,
    }

    /// One row of the guard: what the ant did, and when.
    #[derive(Clone, Debug)]
    struct Row {
        place: Place,
        load: Load,
        role: Role,
        /// The first frame its crop held food or its energy rose.
        ate: Option<u64>,
        /// The first frame its head stood above the ground with nothing overhead.
        out: Option<u64>,
        /// The first frame it cut a cell, and how many it cut.
        dug: Option<u64>,
        digs: u64,
        died: Option<u64>,
        /// It let go of the load it started with.
        let_go: bool,
        /// An open way joined it to the open air at the start, and at the end.
        way_before: bool,
        way_after: bool,
        head: (i32, i32),
        /// Under the walk, what it did: decisions by drive and the rest, and
        /// every decision when the walk was built tracing.
        counts: Option<WalkCounts>,
        trace: Vec<WalkRow>,
    }

    /// Cells a sealed ant must cut for its digging to count: one cut is a
    /// roll that came up, three is an ant working at the wall.
    const DIGGING: u64 = 3;

    impl Row {
        /// **When its need was met**, if before it died. Eating always counts.
        /// The open air counts unless it holds food, which a starving ant eats
        /// rather than carries (the design's hunger: *eats what it holds*).
        /// Digging counts only where nothing joined it to the air.
        fn met(&self) -> Option<u64> {
            let out = self.out.filter(|_| self.load != Load::Food);
            let dug = self.dug.filter(|_| self.place.sealed() && self.digs >= DIGGING);
            [self.ate, out, dug]
                .into_iter()
                .flatten()
                .min()
                .filter(|&f| self.died.is_none_or(|d| f < d))
        }

        /// Met, and no load it let go of shut a way that was open.
        fn green(&self) -> bool {
            self.met().is_some() && !(self.let_go && self.way_before && !self.way_after)
        }
    }

    /// **The guard's bed**: `mound_world`'s ground and mound (`creature.rs`'s
    /// tests), with the rest of a nest under them and food on the surface.
    /// Loose soil from row 40 over a stone floor at 92; a nest site at column
    /// 60 whose founding cut is a shaft on columns 60-61 down to row 45 and a
    /// chamber on rows 46-47; a second shaft from the chamber's floor down to
    /// a room on rows 60-61; a closed pocket in the ground on rows 70-71; a
    /// mound of packed soil on columns 30-55 over rows 26-39 with a tunnel on
    /// rows 35-37 open to the air at column 29 and a closed pocket on rows
    /// 31-33; and a heap of player food 25 columns east of the door.
    ///
    /// **Every open cell in the ground is lined with packed soil**, as the
    /// founding cut lines its own and a digging ant lines the gallery it cuts
    /// (`soil.ron`'s `packs_into`): a hole in loose soil is gone in five frames
    /// (`packedsoil.ron`), and the first draft of this bed lost its shaft,
    /// chamber and the mound pocket's one-row roof that way, burying the ant
    /// it was testing. The mound is fourteen rows tall so that no roof over
    /// its holes is a single row: a packed cell with air beneath it holds on
    /// three contacts, and a one-row roof has two. **Every room is two rows
    /// tall**, as the founding chamber is: an ant on the floor of a taller one
    /// can step only along the floor until it finds a wall (its usable
    /// headings were east and west alone), so a shaft in the middle of a
    /// three-row room's ceiling is out of reach, and the first draft's deep
    /// room starved every empty ant in it for that reason, which is the
    /// room's shape and not a rule.
    fn guard_bed() -> World {
        let mut w = World::new(Rect::new(0, 0, 119, 99));
        w.brood = Some(false);
        w.bud_at_nest = Some(false);
        let soil = w.materials.id_of("soil").expect("soil material");
        let packed = w.materials.id_of("packedsoil").expect("packed soil material");
        let food = w.materials.id_of("provisions").expect("provisions material");
        for x in 0..=119 {
            for y in GROUND..=99 {
                let stone = y >= 92 || x == 0 || x == 119;
                w.set(
                    x,
                    y,
                    if stone {
                        Cell::new(material::STONE, 0)
                    } else {
                        Cell::new(soil, 0)
                    },
                );
            }
        }
        w.register_nest_site(60, 38, 2);
        let cut = crate::sim::world::ShaftFootprint {
            x0: 60,
            x1: 61,
            top: 40,
            bottom: 45,
            mouth_bottom: 41,
            chamber_x0: 56,
            chamber_x1: 65,
            chamber_top: 46,
            chamber_bottom: 47,
            side: None,
        };
        w.nest_sites[0].shaft = Some(cut);
        let open: Vec<(i32, i32)> = (40..=59)
            .flat_map(|y| [(60, y), (61, y)])
            .chain((46..=47).flat_map(|y| (56..=65).map(move |x| (x, y))))
            .chain((60..=61).flat_map(|y| (54..=67).map(move |x| (x, y))))
            .chain((70..=71).flat_map(|y| (18..=24).map(move |x| (x, y))))
            .collect();
        for &(x, y) in &open {
            w.set(x, y, Cell::EMPTY);
        }
        for &(x, y) in &open {
            for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                if w.get(x + dx, y + dy).material == soil {
                    w.set(x + dx, y + dy, Cell::new(packed, 0));
                }
            }
        }
        for y in 26..GROUND {
            for x in 30..=55 {
                let tunnel = (35..=37).contains(&y) && x <= 45;
                let pocket = (31..=33).contains(&y) && (46..=52).contains(&x);
                w.set(
                    x,
                    y,
                    if tunnel || pocket {
                        Cell::EMPTY
                    } else {
                        Cell::new(packed, 0)
                    },
                );
            }
        }
        for y in 37..GROUND {
            for x in 84..=90 {
                w.set(x, y, Cell::new(food, 0));
            }
        }
        w.step_nest_dug();
        step_nest_rest(&mut w);
        w
    }

    /// **The guard's bed stands with nobody in it.** The first draft's did not:
    /// its shaft, chamber and the mound pocket's roof fell in within five
    /// frames and buried the ant each row was testing, so 22 of 36 rows read
    /// red for the ground, not for any rule. Every cell the bed opens must
    /// still be open after `BOUND` frames, and every place must be where an
    /// ant can be put down. Watched red with the packed lining taken out.
    #[test]
    fn the_guard_bed_stands() {
        let mut w = guard_bed();
        let open: Vec<(i32, i32)> = (0..=119)
            .flat_map(|x| (GROUND..=91).map(move |y| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == material::EMPTY)
            .collect();
        let mound: Vec<(i32, i32)> = (30..=55)
            .flat_map(|x| (26..GROUND).map(move |y| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == material::EMPTY)
            .collect();
        // Shaft 40, chamber 16 beside it, deep room 28, buried pocket 14; the
        // mound's tunnel 48 and pocket 21.
        assert_eq!(
            (open.len(), mound.len()),
            (98, 69),
            "test setup: the bed did not open the cells it names"
        );
        for _ in 0..BOUND {
            crate::sim::update::step(&mut w);
            w.step_active_sites();
        }
        let filled: Vec<(i32, i32)> = open
            .iter()
            .chain(&mound)
            .copied()
            .filter(|&(x, y)| w.get(x, y).material != material::EMPTY)
            .collect();
        assert!(
            filled.is_empty(),
            "{} of the bed's open cells filled in {BOUND} frames, first {:?}",
            filled.len(),
            filled.first()
        );
        for place in Place::ALL {
            let mut w = guard_bed();
            let (x, y) = place.at();
            assert!(
                plant_creature_seed(&mut w, x, y, "ant").is_some(),
                "no ant fits at {place:?} {:?}",
                place.at()
            );
            assert_eq!(
                open_to_the_air(&w, (x, y), w.get(x, y).organism_id()),
                !place.sealed(),
                "{place:?} is not as joined to the air as it is named"
            );
        }
    }

    /// One row: the bed, one ant at `place` at 30% of its budget holding
    /// `load` in `role`, run for [`BOUND`] frames under `walk`, with `setup`
    /// applied to the world first (a switch the arm runs under).
    fn guard_row(place: Place, load: Load, role: Role, walk: Option<NeedsMode>, setup: fn(&mut World)) -> Row {
        let mut w = guard_bed();
        setup(&mut w);
        let (x, y) = place.at();
        let site = plant_creature_seed(&mut w, x, y, "ant").expect("test setup: the ant does not fit");
        w.schedule_active_site(site);
        let a = w.get(x, y).organism_id();
        assert_ne!(a, 0, "test setup: no ant at {:?}", place.at());
        let soil = w.materials.id_of("soil").expect("soil material");
        let food = w.materials.id_of("provisions").expect("provisions material");
        let start = w
            .species
            .get(w.organism(a).expect("live").species)
            .creature
            .as_ref()
            .expect("a creature")
            .start_energy;
        let st = w.organism_mut(a).expect("live");
        st.energy = 0.3 * start;
        st.nest_bound_until = if role == Role::NestWorker { u64::MAX } else { 0 };
        st.spoil = match load {
            Load::Nothing => None,
            Load::Pellet => Some(crate::sim::organism::Spoil {
                cell: Cell::new(soil, 0),
                store: false,
            }),
            Load::Food => Some(crate::sim::organism::Spoil {
                cell: Cell::new(food, 0),
                store: true,
            }),
        };
        // A `setup` that built its own walk (the vetoed control) keeps it.
        if let Some(mode) = walk {
            w.needs = Some(Box::new(NeedsWalk::new(mode, w.frame)));
        }
        let digs = w.creature_stats.digs;
        let way_before = open_to_the_air(&w, (x, y), a);
        let mut row = Row {
            place,
            load,
            role,
            ate: None,
            out: None,
            dug: None,
            digs: 0,
            died: None,
            let_go: false,
            way_before,
            way_after: way_before,
            head: (x, y),
            counts: None,
            trace: Vec::new(),
        };
        let mut energy = 0.3 * start;
        for f in 0..BOUND {
            crate::sim::update::step(&mut w);
            w.step_active_sites();
            let Some(st) = w.organism(a) else {
                row.died = Some(f);
                break;
            };
            row.head = st.chain[0];
            if row.ate.is_none() && (st.crop.is_some() || st.energy > energy + 1.0) {
                row.ate = Some(f);
            }
            energy = st.energy;
            let (hx, hy) = row.head;
            if row.out.is_none() && hy < GROUND && !under_cover(&w, hx, hy) {
                row.out = Some(f);
            }
            row.digs = w.creature_stats.digs - digs;
            if row.dug.is_none() && row.digs > 0 {
                row.dug = Some(f);
            }
            row.let_go |= load != Load::Nothing && st.spoil.is_none();
        }
        row.way_after = open_to_the_air(&w, row.head, a);
        row.counts = w.needs.as_ref().filter(|n| n.mode == NeedsMode::Walk).map(|n| n.counts);
        row.trace = w
            .needs
            .as_mut()
            .map(|n| std::mem::take(&mut n.rows))
            .unwrap_or_default();
        row
    }

    /// Whether a 4-connected walk over empty cells (and the ant's own body)
    /// joins `from` to a cell above the ground with nothing overhead.
    fn open_to_the_air(w: &World, from: (i32, i32), a: OrganismId) -> bool {
        let passable = |x: i32, y: i32| {
            (0..=119).contains(&x) && (0..=99).contains(&y) && {
                let c = w.get(x, y);
                c.material == material::EMPTY || c.organism_id() == a
            }
        };
        let mut seen = std::collections::HashSet::from([from]);
        let mut todo = vec![from];
        while let Some((x, y)) = todo.pop() {
            if y < GROUND && !under_cover(w, x, y) {
                return true;
            }
            for n in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                if passable(n.0, n.1) && seen.insert(n) {
                    todo.push(n);
                }
            }
        }
        false
    }

    /// Every row of the guard under `walk`: six places, three loads, two roles.
    fn guard(walk: Option<NeedsMode>, setup: fn(&mut World)) -> Vec<Row> {
        Place::ALL
            .into_iter()
            .flat_map(|p| {
                [Load::Nothing, Load::Pellet, Load::Food]
                    .into_iter()
                    .map(move |l| (p, l))
            })
            .flat_map(|(p, l)| [Role::Forager, Role::NestWorker].into_iter().map(move |r| (p, l, r)))
            .map(|(p, l, r)| guard_row(p, l, r, walk, setup))
            .collect()
    }

    fn print_guard(rows: &[Row]) {
        let at = |f: Option<u64>| f.map_or("-".to_string(), |f| f.to_string());
        println!(
            "{:<12} {:<10} {:<11} {:>5} {:>5} {:>5} {:>4} {:>5} {:>7} {:>9} {:<10} verdict",
            "place", "load", "role", "ate", "out", "dug", "cuts", "died", "let go", "way b/a", "head"
        );
        for r in rows {
            println!(
                "{:<12} {:<10} {:<11} {:>5} {:>5} {:>5} {:>4} {:>5} {:>7} {:>9} {:<10} {}",
                format!("{:?}", r.place),
                format!("{:?}", r.load),
                format!("{:?}", r.role),
                at(r.ate),
                at(r.out),
                at(r.dug),
                r.digs,
                at(r.died),
                r.let_go,
                format!("{}/{}", u8::from(r.way_before), u8::from(r.way_after)),
                format!("{:?}", r.head),
                if r.green() { "green" } else { "RED" }
            );
            if let Some(c) = r.counts {
                let drives: Vec<String> = Drive::ALL
                    .iter()
                    .zip(c.decisions)
                    .filter(|&(_, n)| n > 0)
                    .map(|(d, n)| format!("{} {n}", d.label()))
                    .collect();
                println!(
                    "    walk: {} | ate held {} pellets down {} escape cuts {} packs {}",
                    drives.join(", "),
                    c.ate_held,
                    c.pellets_down,
                    c.escape_cuts,
                    c.escape_packs
                );
            }
        }
        println!(
            "{} of {} rows green",
            rows.iter().filter(|r| r.green()).count(),
            rows.len()
        );
    }

    /// **The guard on today's ant**, as a report: which places, loads and
    /// roles leave a hungry ant with no way to meet its need. Run with
    /// `cargo test --release --lib needs::tests::the_guard_on_todays_ant -- --ignored --nocapture`
    /// (about 2 s, debug or release).
    ///
    /// **Read 2026-10-06 on this branch (main 3ba1e7bd and switches that are
    /// off): 20 of 36 rows green, the same for foragers and nest workers but
    /// in two rows.** The
    /// 16 red, in four groups:
    /// - **Holding food, it starves** (door, chamber, deep room; 6 rows): it
    ///   puts its store load down, walks out and dies without eating.
    /// - **In the mound's tunnel, 11 cells from the air** (4 rows): nothing
    ///   pulls it out of the mound (`MOUND_OUT`'s `way` is off), and its walk
    ///   paces the tunnel's floor until it starves.
    /// - **In a pocket in the ground, lean, it never cuts** (4 rows): the lean
    ///   rule takes its dig roll, and `MOUND_OUT=dig` lets only the mound's
    ///   pockets off it.
    /// - **A forager in the mound's pocket stops after one cut** (2 rows); a
    ///   nest worker there cuts five.
    ///
    /// Under `NEEDS_FIRST` with every part on, 26 of 36: the buried pocket
    /// digs and the chamber's carrier eats its load
    /// ([`the_guard_under_needs_first`]).
    #[test]
    #[ignore = "a report, not a gate: prints the guard's table for today's ant"]
    fn the_guard_on_todays_ant() {
        print_guard(&guard(None, |_| {}));
    }

    /// **...and under `NEEDS_FIRST` with every part on**, the switch built to
    /// waive the blocks a hungry or shut-in ant meets: the rows it turns
    /// green are the guard seeing a block lifted.
    #[test]
    #[ignore = "a report, not a gate: prints the guard's table under NEEDS_FIRST"]
    fn the_guard_under_needs_first() {
        print_guard(&guard(None, |w| w.needs_first = Some(NeedsFirst::ON)));
    }

    /// **The guard under the walk: every row green**, the gate slice 1 has to
    /// pass before it runs a colony. `cargo test --release --lib
    /// needs::tests::the_guard_under_the_walk -- --nocapture` prints the table,
    /// with each row's decisions by drive under it.
    ///
    /// **Read 2026-10-07: 36 of 36** (today's ant 20, `NEEDS_FIRST` 26). Held
    /// food is eaten within 5 frames in every place; the deep room and the
    /// mound's tunnel walk out; both pockets cut. The mound's pocket gets out
    /// and eats (frames 545-647) only since the escape stopped waiting out
    /// its stall after every cut; before, it was green on three cuts and
    /// starved two rows short of the air. The buried pocket, 30 rows down at
    /// 60 J, still starves 10 rows short of the ground: a cut and a step cost
    /// it about 1.6 J, so the climb is past its budget whatever the rule.
    #[test]
    fn the_guard_under_the_walk() {
        let rows = guard(Some(NeedsMode::Walk), |_| {});
        print_guard(&rows);
        let red: Vec<String> = rows
            .iter()
            .filter(|r| !r.green())
            .map(|r| format!("{:?}/{:?}/{:?}", r.place, r.load, r.role))
            .collect();
        assert!(
            red.is_empty(),
            "{} of {} rows red under the walk: {red:?}",
            red.len(),
            rows.len()
        );
    }

    /// **...and its positive control: the same walk with its needs vetoed**,
    /// so every ant does its job or idles whatever its hunger. The guard must
    /// see it: rows go red that the walk turns green.
    #[test]
    fn the_guard_sees_a_vetoed_need() {
        let rows = guard(None, |w| {
            let mut n = NeedsWalk::new(NeedsMode::Walk, w.frame);
            n.veto_needs = true;
            w.needs = Some(Box::new(n));
        });
        print_guard(&rows);
        let red = rows.iter().filter(|r| !r.green()).count();
        assert!(
            red >= VETO_RED,
            "only {red} of {} rows red with the needs vetoed",
            rows.len()
        );
    }

    /// Rows the vetoed control must leave red. **Read 2026-10-07: 9 of 36**
    /// (holding food at the door, starving: 2; idle in the deep room, never
    /// leaving: 2; buried, never cutting: 4; a forager in the mound's pocket
    /// stopping after two cuts: 1), so the bar has headroom under that, not
    /// on it.
    const VETO_RED: usize = 5;

    /// **One row of the guard under the walk, every decision printed**: the
    /// per-ant trace for a row that needs understanding. Name the row in
    /// `NEEDS_GUARD_ROW` as `place,load,role` (`MoundPocket,Nothing,Forager`).
    #[test]
    #[ignore = "a tool, not a gate: prints one guard row's every decision"]
    fn trace_one_guard_row() {
        let want = std::env::var("NEEDS_GUARD_ROW").unwrap_or_else(|_| "MoundPocket,Nothing,Forager".to_string());
        let parts: Vec<&str> = want.split(',').map(str::trim).collect();
        let place = *Place::ALL
            .iter()
            .find(|p| format!("{p:?}") == parts[0])
            .expect("a place");
        let load = *[Load::Nothing, Load::Pellet, Load::Food]
            .iter()
            .find(|l| format!("{l:?}") == parts[1])
            .expect("a load");
        let role = *[Role::Forager, Role::NestWorker]
            .iter()
            .find(|r| format!("{r:?}") == parts[2])
            .expect("a role");
        let row = guard_row(place, load, role, None, |w| {
            let mut n = NeedsWalk::new(NeedsMode::Walk, w.frame);
            n.trace_every = 1;
            w.needs = Some(Box::new(n));
        });
        print_guard(std::slice::from_ref(&row));
        println!(
            "frame  at        to        drive   job     energy  crop   hunger hold  depth p_move moved stall target"
        );
        for t in &row.trace {
            println!(
                "{:>6} {:<9} {:<9} {:<7} {:<7} {:>6.1} {:>6.1} {:>6.3} {:>5.2} {:>5} {:>6.3} {:>5} {:>5} {:?}",
                t.frame,
                format!("{:?}", t.at),
                format!("{:?}", t.to),
                t.drive.label(),
                t.job.label(),
                t.energy,
                t.crop,
                t.hunger,
                t.hold,
                t.depth,
                t.p_move,
                t.moved,
                t.stall,
                t.target
            );
        }
    }
}
