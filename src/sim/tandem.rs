//! **Tandem runs: one ant leads another to food by touch** (the lone scout
//! seat's demo, `demo=scout`; plan `Reports/teaching-the-colony-2026-10-25/
//! scout-demo/plan-v2.md` §3).
//!
//! A leader (the player, or in arm A a taught ant) taps a nestmate; if she
//! accepts, she follows **one cell behind, in the leader's footsteps**,
//! forced through the mind's own option list (`World::mind_force`, the
//! enemy seat's steer-for-minds slot) so her body, her bills and her falls
//! stay the game's. At landmarks on the way (a change of slope, a tunnel
//! mouth, the patch's edge, and now and then for no reason) she stops to
//! learn the place, and drops back a cell first so the leader can see it
//! coming. A leader who walks on through a stop loses her: she sweeps the
//! ground left and right around the last place she touched the leader, and
//! is released to her own mind if she does not find her.
//!
//! **The outcome is graded, not pass/fail** (CLAUDE.md law 1): a pupil
//! brought all the way to the patch is planted with a full memory of it
//! (`recall`, the keeper seat's head); one lost on the way keeps a weaker
//! memory in proportion to the landmarks she stopped at, so a broken run
//! still half-teaches, as broken tandem runs do in *Temnothorax*.
//!
//! **What is scripted, said plainly.** Following is a hard instinct once a
//! tap is accepted (a state-entered body rule, like a crossing: any ant can
//! be tapped and it ends on arrival or a break; not a caste). The accept
//! chance, the pause clock and the cascade chance `c` are hand-set knobs for
//! a demo, printed on the help card unless the arms are sealed. None of it
//! is a heritable gain yet; the seat design's step 5 (lead as a rule every
//! ant has, gated on far-food deliveries beating no-tandem on paired seeds)
//! is what could ever ship on by default.
//!
//! **Off is off**: `World::tandem` is `None` unless a demo sets it, and the
//! only hook is one `Option` read in `lab::tick_world`.

use std::collections::{BTreeMap, VecDeque};

use crate::sim::cell::OrganismId;
use crate::sim::rng;
use crate::sim::world::World;

/// Streams for this module's draws, keyed with the organism and the frame.
const RNG_SLOT_ACCEPT: u64 = 0x7a4d_0001;
const RNG_SLOT_PAUSE: u64 = 0x7a4d_0002;
const RNG_SLOT_CASCADE: u64 = 0x7a4d_0003;
const RNG_SLOT_PAUSE_LEN: u64 = 0x7a4d_0004;

/// Which arm of the demo is running (plan v2 §3.8-3.10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arm {
    /// The scout: tap, follow by touch, pauses, breaks, and a cascade at `c`.
    Scout,
    /// The owner's rejected design given equal terms: a call gathers every
    /// eligible sister within reach into a column that never pauses or breaks.
    Column,
    /// The escort: the scout without the cascade.
    Escort,
}

impl Arm {
    pub fn letter(self) -> char {
        match self {
            Arm::Scout => 'A',
            Arm::Column => 'B',
            Arm::Escort => 'C',
        }
    }
}

/// Every hand-set number in one place, printed on the seat's help card.
#[derive(Clone, Debug)]
pub struct Knobs {
    /// A follower whose head is within this many cells (Chebyshev) of any
    /// cell of the leader's body is in contact ([`contact_gap`]).
    pub contact: i32,
    /// Past this gap for `break_frames` frames, contact is broken.
    pub break_gap: i32,
    pub break_frames: u64,
    /// At or under this gap the follower stands, so she sits behind the
    /// leader and never on top of her (review: stacking on 30% of frames).
    pub rest_gap: i32,
    /// A tap reaches nestmates within this many cells.
    pub tap_reach: i32,
    /// Arm B's call reaches this far.
    pub call_reach: i32,
    /// At most this many pairs at once, colony-wide.
    pub max_pairs: usize,
    /// The geometric pause clock: chance per follower step of a stop where
    /// there is no landmark (mean 1/p steps).
    pub pause_chance: f32,
    /// The tell: frames she lags before a stop.
    pub tell_frames: u64,
    /// A stop lasts between these, longer for a slow sister.
    pub pause_min: u64,
    pub pause_max: u64,
    /// She picks up again after a stop at or under this gap; between this
    /// and `break_gap` she picks up but has not learned the landmark.
    pub resume_learned: i32,
    /// The ground sweep: legs of these lengths each way, for this long.
    pub sweep_legs: [i32; 3],
    pub sweep_frames: u64,
    /// Landmarks are at least this many leader steps apart, so a rough
    /// surface is not a stop every few cells (first headless run: 24 stops
    /// in 100 columns at no spacing).
    pub landmark_spacing: u32,
    /// Arrival: within this many cells of a patch food cell.
    pub arrive_reach: i32,
    /// A lost pupil keeps this share of a full lesson, times her landmark
    /// fraction.
    pub lost_lesson: f32,
    /// How long a released pupil is watched for "broken, still arrived".
    pub watch_frames: u64,
    /// Arm B's column gives up after this long.
    pub column_timeout: u64,
    /// The mind's energy sense (`c::ENERGY`, `e/(1+e)`): under `weak` she is
    /// too weak to go; from `weak` to `full` the accept chance falls from
    /// `accept_hi` to `accept_lo`; over `full`, `accept_full`.
    pub weak: f32,
    pub full: f32,
    pub accept_hi: f32,
    pub accept_lo: f32,
    pub accept_full: f32,
    /// The cascade: chance per trip home from the patch that a taught ant
    /// taps a sister of her own (arm A only).
    pub cascade_c: f32,
    /// **A cascade leader works her way out of a box** (off by default; the
    /// scout seat's lean season sets it, round 2, 2026-10-30). Traced in the
    /// round-1 season: every cascade run started at the nest, where the
    /// pupil's teacher had just put her food down, and none got out of the
    /// mound -- one leader stood at x 133 for 6,000 frames with her pupil in
    /// contact and the pupil starved while led; another 9,000 frames. Forced
    /// one heading at the target, a body boxed by the mound, the spoil and the
    /// provisions has nowhere to go (the headless player was boxed the same
    /// way and needed the same cure). On: when she has come no nearer her
    /// target for [`UNSTICK_FRAMES`], she tries the upward headings in turn,
    /// then digs.
    pub cascade_unstick: bool,
    /// A cascade run not arrived after this many frames is given up (the
    /// pupil released with what she learned); 0 is never (the default).
    pub cascade_timeout: u64,
}

/// Frames without getting nearer before a cascade leader tries another
/// heading ([`Knobs::cascade_unstick`]); the headless player's own number.
pub const UNSTICK_FRAMES: u64 = 40;

impl Default for Knobs {
    fn default() -> Self {
        Knobs {
            contact: 2,
            break_gap: 4,
            break_frames: 45,
            rest_gap: 1,
            tap_reach: 3,
            call_reach: 6,
            max_pairs: 6,
            pause_chance: 1.0 / 50.0,
            tell_frames: 12,
            pause_min: 60,
            pause_max: 90,
            resume_learned: 2,
            sweep_legs: [8, 16, 32],
            sweep_frames: 360,
            landmark_spacing: 24,
            arrive_reach: 6,
            lost_lesson: 0.6,
            watch_frames: 6_000,
            column_timeout: 3_000,
            weak: 0.4,
            full: 0.8,
            accept_hi: 0.9,
            accept_lo: 0.4,
            accept_full: 0.15,
            cascade_c: 0.25,
            cascade_unstick: false,
            cascade_timeout: 0,
        }
    }
}

/// The food a tandem leads to: provisions within `r` columns of `x`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Patch {
    pub x: i32,
    pub r: i32,
}

impl Patch {
    pub fn holds_column(&self, x: i32) -> bool {
        (x - self.x).abs() <= self.r
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Following,
    /// She lags a cell before a stop, until `until`; then `Pausing`.
    Tell { until: u64, pause_for: u64, landmark: bool },
    /// She stands, learning the place, until `until`.
    Pausing { until: u64, landmark: bool },
    /// Contact lost: she sweeps the ground around `centre`.
    Lost { since: u64, centre: (i32, i32), leg: u8, east: bool },
}

#[derive(Clone, Debug)]
pub struct Pair {
    pub leader: OrganismId,
    pub follower: OrganismId,
    pub phase: Phase,
    /// The leader's recent head cells, oldest first; the follower steps
    /// toward the oldest she has not reached (breadcrumb following).
    pub crumbs: VecDeque<(i32, i32)>,
    /// Landmark flags on crumbs, as cells.
    pub landmarks: Vec<(i32, i32)>,
    /// Landmarks she stopped at with the leader close (learned), and every
    /// landmark she reached at all.
    pub learned: u8,
    pub met: u8,
    pub since: u64,
    pub last_contact: (i32, i32),
    /// Frames the gap has been over `break_gap`.
    pub over_for: u64,
    /// Her head last tick, to count steps.
    pub last_at: (i32, i32),
    /// Frames the leader's head has not moved.
    pub leader_still: u64,
    /// Frames since the follower's head last moved (a step is one decision
    /// in five or six, so "did not step this tick" says nothing alone).
    pub follower_still: u64,
    /// Leader steps since the last landmark.
    pub since_landmark: u32,
    /// Arm B: a column member, no pauses or breaks.
    pub column: bool,
    /// A taught ant leading by the cascade (her target is her memory).
    pub cascade: bool,
}

/// A tap's answer, shown on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TapResult {
    Accepted(OrganismId),
    Refused(OrganismId, Refusal),
    NoOne,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    TooWeak,
    WellFed,
    NotNow,
    Busy,
}

impl Refusal {
    pub fn words(self) -> &'static str {
        match self {
            Refusal::TooWeak => "too weak to go: feed her first",
            Refusal::WellFed => "well fed",
            Refusal::NotNow => "not now",
            Refusal::Busy => "too many runs at once",
        }
    }
}

/// A moment the overlay shows for a while: a refusal shake, a break, an
/// arrival.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Refused(Refusal),
    Broke,
    Arrived,
    Recontact,
    Landmark,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub taps: u32,
    pub refusals: u32,
    pub started: u32,
    pub done: u32,
    pub broken: u32,
    pub broken_still_arrived: u32,
    pub recontacts: u32,
    pub landmarks_learned: u32,
    pub starved_while_led: u32,
    pub cascade_runs: u32,
    /// Trips home from the patch by anyone in the teaching tree.
    pub tree_trips: u32,
    /// Trips home from the patch by any ant (the patch's income).
    pub patch_trips: u32,
}

#[derive(Clone, Debug)]
pub struct Released {
    pub id: OrganismId,
    pub at: u64,
    pub arrived: bool,
}

/// The whole module's state, on `World::tandem`.
#[derive(Clone, Debug)]
pub struct Tandems {
    pub arm: Arm,
    pub knobs: Knobs,
    pub patch: Option<Patch>,
    pub pairs: Vec<Pair>,
    /// Pupil -> teacher, for the teaching tree.
    pub tree: BTreeMap<OrganismId, OrganismId>,
    pub released: Vec<Released>,
    pub counts: Counts,
    /// Recent marks for the overlay, `(organism, mark, frame)`.
    pub marks: Vec<(OrganismId, Mark, u64)>,
    /// Each ant's `trip_deliveries` as last seen, to key the cascade and the
    /// patch's income on the first delivery of a trip (review 5).
    trips_seen: BTreeMap<OrganismId, u32>,
    /// The patch's income: frames of trips home from it, for a per-minute bar.
    pub income: VecDeque<u64>,
    /// Lesson planted on arrival, by pupil, 0..1 (also what recall gets).
    pub lessons: BTreeMap<OrganismId, f32>,
    /// Each cascade leader's nearest approach to her target and the frame
    /// she made it ([`Knobs::cascade_unstick`]).
    progress: BTreeMap<OrganismId, (i32, u64)>,
}

impl Tandems {
    pub fn new(arm: Arm, patch: Option<Patch>) -> Self {
        let mut knobs = Knobs::default();
        if arm != Arm::Scout {
            knobs.cascade_c = 0.0;
        }
        Tandems {
            arm,
            knobs,
            patch,
            pairs: Vec::new(),
            tree: BTreeMap::new(),
            released: Vec::new(),
            counts: Counts::default(),
            marks: Vec::new(),
            trips_seen: BTreeMap::new(),
            income: VecDeque::new(),
            lessons: BTreeMap::new(),
            progress: BTreeMap::new(),
        }
    }

    pub fn pair_of(&self, id: OrganismId) -> Option<&Pair> {
        self.pairs.iter().find(|p| p.follower == id)
    }

    pub fn leads(&self, id: OrganismId) -> bool {
        self.pairs.iter().any(|p| p.leader == id)
    }

    fn mark(&mut self, id: OrganismId, m: Mark, frame: u64) {
        self.marks.push((id, m, frame));
        if self.marks.len() > 64 {
            self.marks.remove(0);
        }
    }

    /// Food trips home from the patch in the last `frames` frames.
    pub fn income_over(&self, now: u64, frames: u64) -> usize {
        self.income.iter().filter(|&&f| now.saturating_sub(f) <= frames).count()
    }
}

fn head(world: &World, id: OrganismId) -> Option<(i32, i32)> {
    world.organism(id).and_then(|s| s.chain.first().copied())
}

fn gap(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs().max((a.1 - b.1).abs())
}

/// **How far the follower's head is from her leader's body** -- any cell of
/// it, not her head. A follower touches her leader's gaster, not her face:
/// measured head to head, a pupil pressed against the tail of a leader facing
/// away reads as three cells off, and the first played run deadlocked on
/// exactly that (the pupil "closing" a gap she could not close, the leader
/// held until she did, 2,000 frames). `None` if either is dead.
pub fn contact_gap(world: &World, follower: OrganismId, leader: OrganismId) -> Option<i32> {
    let f = head(world, follower)?;
    world.organism(leader)?.chain.iter().map(|&c| gap(f, c)).min()
}

fn toward(from: (i32, i32), to: (i32, i32)) -> Option<(i8, i8)> {
    let d = ((to.0 - from.0).signum() as i8, (to.1 - from.1).signum() as i8);
    (d != (0, 0)).then_some(d)
}

/// **The mind's own energy sense** for `id`, `e/(1+e)` with `e` the bank in
/// tanks (`mind::context_at`): 0.5 is one full tank, 0.8 four.
pub fn energy_sense(world: &World, id: OrganismId) -> Option<f32> {
    let st = world.organism(id)?;
    let def = world.species.creature_shared(st.species)?;
    let tank = def.sensed_tank.unwrap_or(def.start_energy).max(f32::EPSILON);
    let e = (st.energy / tank).max(0.0);
    Some(e / (1.0 + e))
}

/// The accept chance at energy sense `s` (plan v2 §3.7). `None` is "too
/// weak to go", a refusal the player can fix by feeding her.
pub fn accept_chance(k: &Knobs, s: f32) -> Option<f32> {
    if s < k.weak {
        return None;
    }
    if s >= k.full {
        return Some(k.accept_full);
    }
    let t = (s - k.weak) / (k.full - k.weak);
    Some(k.accept_hi + (k.accept_lo - k.accept_hi) * t)
}

/// **Who a tap can reach**: live same-colony adults (not brood) within
/// `reach` cells of the leader, not already in a pair, not carrying, nearest
/// first. No nest-bound filter (review 9: it would make one sister in four
/// untappable by id, a hand-made caste in the player's hands).
pub fn candidates(world: &World, leader: OrganismId, reach: i32) -> Vec<OrganismId> {
    let Some(t) = world.tandem.as_ref() else { return Vec::new() };
    let Some(at) = head(world, leader) else { return Vec::new() };
    let Some(lst) = world.organism(leader) else { return Vec::new() };
    let (species, colony) = (lst.species, lst.colony);
    let mut out: Vec<(i32, OrganismId)> = world
        .live_organism_ids()
        .into_iter()
        .filter(|&id| id != leader)
        .filter_map(|id| {
            let st = world.organism(id)?;
            if st.species != species || st.colony != colony || st.spoil.is_some() || st.crop.is_some_and(|c| c.cells > 0) {
                return None;
            }
            if t.pairs.iter().any(|p| p.follower == id || p.leader == id) || crate::sim::steer::is_steered(world, id) {
                return None;
            }
            let h = *st.chain.first()?;
            let g = gap(h, at);
            (g <= reach).then_some((g, id))
        })
        .collect();
    out.sort();
    out.into_iter().map(|(_, id)| id).collect()
}

/// **Tap the nearest sister** within reach and ask her to follow `leader`.
/// Arm B calls instead: every eligible sister within the call reach falls in.
pub fn tap(world: &mut World, leader: OrganismId) -> TapResult {
    let Some(arm) = world.tandem.as_ref().map(|t| t.arm) else { return TapResult::NoOne };
    if arm == Arm::Column {
        return call(world, leader);
    }
    let reach = world.tandem.as_ref().map_or(3, |t| t.knobs.tap_reach);
    let Some(&id) = candidates(world, leader, reach).first() else { return TapResult::NoOne };
    let frame = world.frame;
    let seed = world.seed;
    let s = energy_sense(world, id).unwrap_or(0.0);
    let t = world.tandem.as_mut().expect("read above");
    t.counts.taps += 1;
    let refuse = |t: &mut Tandems, r: Refusal| {
        t.counts.refusals += 1;
        t.mark(id, Mark::Refused(r), frame);
        TapResult::Refused(id, r)
    };
    if t.pairs.len() >= t.knobs.max_pairs {
        return refuse(t, Refusal::Busy);
    }
    let Some(p) = accept_chance(&t.knobs, s) else { return refuse(t, Refusal::TooWeak) };
    if !rng::stream(seed, id as u64, frame, RNG_SLOT_ACCEPT).chance(p) {
        return refuse(t, if s >= t.knobs.full { Refusal::WellFed } else { Refusal::NotNow });
    }
    start_pair(world, leader, id, false, false);
    TapResult::Accepted(id)
}

/// Arm B's call: the nearest eligible sister and every other one within the
/// call reach join a column, under the same hunger bands and the same cap.
fn call(world: &mut World, leader: OrganismId) -> TapResult {
    let reach = world.tandem.as_ref().map_or(6, |t| t.knobs.call_reach);
    let frame = world.frame;
    let seed = world.seed;
    let mut first = TapResult::NoOne;
    for id in candidates(world, leader, reach) {
        let s = energy_sense(world, id).unwrap_or(0.0);
        let t = world.tandem.as_mut().expect("checked");
        if t.pairs.len() >= t.knobs.max_pairs {
            break;
        }
        t.counts.taps += 1;
        let ok = accept_chance(&t.knobs, s).is_some_and(|p| rng::stream(seed, id as u64, frame, RNG_SLOT_ACCEPT).chance(p));
        if ok {
            start_pair(world, leader, id, true, false);
            if first == TapResult::NoOne {
                first = TapResult::Accepted(id);
            }
        } else {
            t.counts.refusals += 1;
            let r = if s < t.knobs.weak { Refusal::TooWeak } else { Refusal::WellFed };
            t.mark(id, Mark::Refused(r), frame);
            if first == TapResult::NoOne {
                first = TapResult::Refused(id, r);
            }
        }
    }
    first
}

fn start_pair(world: &mut World, leader: OrganismId, follower: OrganismId, column: bool, cascade: bool) {
    let frame = world.frame;
    let lat = head(world, leader).unwrap_or((0, 0));
    let fat = head(world, follower).unwrap_or(lat);
    let t = world.tandem.as_mut().expect("caller checked");
    t.counts.started += 1;
    if cascade {
        t.counts.cascade_runs += 1;
    }
    let mut crumbs = VecDeque::new();
    crumbs.push_back(lat);
    t.pairs.push(Pair {
        leader,
        follower,
        phase: Phase::Following,
        crumbs,
        landmarks: Vec::new(),
        learned: 0,
        met: 0,
        since: frame,
        last_contact: lat,
        over_for: 0,
        last_at: fat,
        leader_still: 0,
        follower_still: 0,
        since_landmark: 0,
        column,
        cascade,
    });
}

/// The follower's forced step this tick, or `None` to stand. One adapter,
/// so the slot it writes to is named in one place.
fn force(world: &mut World, id: OrganismId, dir: Option<(i8, i8)>) {
    crate::sim::steer::force(world, id, crate::sim::steer::ForcedPick { dir, verb: None, once: false });
}

fn unforce(world: &mut World, id: OrganismId) {
    crate::sim::steer::unforce(world, id);
}

/// Is `(x, y)` a landmark on the leader's path, given the crumbs before it?
/// A change of slope (two or more rows over the last three crumbs after a
/// level stretch, or the reverse), a tunnel mouth (roofed on one side and
/// open on the other), or the patch's edge.
fn is_landmark(world: &World, crumbs: &VecDeque<(i32, i32)>, at: (i32, i32), patch: Option<Patch>) -> bool {
    let n = crumbs.len();
    if n >= 6 {
        let a = crumbs[n - 6];
        let b = crumbs[n - 3];
        let rise_old = (b.1 - a.1).abs();
        let rise_new = (at.1 - b.1).abs();
        if (rise_old <= 1 && rise_new >= 3) || (rise_old >= 3 && rise_new <= 1) {
            return true;
        }
    }
    if let Some(&prev) = crumbs.back() {
        let roofed = |p: (i32, i32)| !(1..=3).all(|dy| world.is_empty(p.0, p.1 - dy));
        if roofed(prev) != roofed(at) {
            return true;
        }
        if let Some(p) = patch {
            if p.holds_column(at.0) != p.holds_column(prev.0) {
                return true;
            }
        }
    }
    false
}

fn release(world: &mut World, i: usize, arrived: bool) {
    let frame = world.frame;
    let t = world.tandem.as_mut().expect("caller checked");
    let p = t.pairs.swap_remove(i);
    t.released.push(Released { id: p.follower, at: frame, arrived });
    let id = p.follower;
    let lesson = if arrived {
        1.0
    } else {
        let met = f32::from(p.met.max(1));
        t.knobs.lost_lesson * f32::from(p.learned) / met
    };
    if arrived {
        t.counts.done += 1;
        t.tree.insert(id, p.leader);
        t.mark(id, Mark::Arrived, frame);
    } else {
        t.counts.broken += 1;
        t.mark(id, Mark::Broke, frame);
        if lesson > 0.0 {
            t.tree.insert(id, p.leader);
        }
    }
    let patch = t.patch;
    if lesson > 0.0 {
        t.lessons.insert(id, lesson);
        // The heap's nearest food cell, or its middle at her height once it has been eaten down: arrival no longer
        // needs food in reach (the leader's stop says "here"), so neither may the lesson.
        let y = head(world, id).map_or(0, |h| h.1);
        if let Some(cell) = patch.map(|p| nearest_food(world, (p.x, y), p.r).unwrap_or((p.x, y))) {
            plant_lesson(world, id, cell, lesson);
        }
    }
    unforce(world, id);
    if p.cascade {
        unforce(world, p.leader);
    }
}

/// Hand the lesson to the remembered-place head (`creature::recall`, the
/// keeper seat's shared piece): her memory of the patch, learned with
/// strength `s`. With the head off (`World::recall` `None`) the memory is
/// still written and today's trained ant ignores it (its pool weight on the
/// memory is zeroed), which is the honest arm.
fn plant_lesson(world: &mut World, id: OrganismId, cell: (i32, i32), s: f32) {
    crate::sim::creature::recall::plant_with(world, id, cell, s);
}

/// The nearest provisions cell to `near` within `r` columns of it.
fn nearest_food(world: &World, near: (i32, i32), r: i32) -> Option<(i32, i32)> {
    let food = world.materials.id_of("provisions")?;
    let mut best: Option<(i32, (i32, i32))> = None;
    for x in near.0 - r..=near.0 + r {
        for y in near.1 - 40..=near.1 + 40 {
            if world.in_bounds(x, y) && world.get(x, y).material == food {
                let d = gap((x, y), near);
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, (x, y)));
                }
            }
        }
    }
    best.map(|(_, c)| c)
}

fn food_within(world: &World, at: (i32, i32), r: i32) -> bool {
    let Some(food) = world.materials.id_of("provisions") else { return false };
    for dx in -r..=r {
        for dy in -r..=r {
            let (x, y) = (at.0 + dx, at.1 + dy);
            if world.in_bounds(x, y) && world.get(x, y).material == food {
                return true;
            }
        }
    }
    false
}

/// **One tick of every pair**, before the world steps, so the forced picks
/// are fresh for this tick's decisions.
pub fn tick(world: &mut World) {
    let Some(t) = world.tandem.as_ref() else { return };
    if t.pairs.is_empty() && t.released.is_empty() && t.patch.is_none() {
        return;
    }
    let frame = world.frame;
    let seed = world.seed;
    let k = t.knobs.clone();
    let patch = t.patch;
    let mut i = 0;
    while i < world.tandem.as_ref().map_or(0, |t| t.pairs.len()) {
        let p = world.tandem.as_ref().expect("checked").pairs[i].clone();
        let (Some(lat), Some(fat)) = (head(world, p.leader), head(world, p.follower)) else {
            // One of them died. A follower dead while led counts against the run.
            let dead_follower = fat_dead(world, p.follower);
            let t = world.tandem.as_mut().expect("checked");
            t.pairs.swap_remove(i);
            if dead_follower {
                t.counts.starved_while_led += 1;
            } else {
                unforce(world, p.follower);
            }
            if p.cascade {
                unforce(world, p.leader);
            }
            continue;
        };
        let mut p = p;
        // The leader's footsteps.
        if p.crumbs.back() == Some(&lat) {
            p.leader_still += 1;
        } else {
            p.leader_still = 0;
        }
        if p.crumbs.back() != Some(&lat) {
            p.since_landmark = p.since_landmark.saturating_add(1);
            let lm = !p.column && p.since_landmark >= k.landmark_spacing && is_landmark(world, &p.crumbs, lat, patch);
            p.crumbs.push_back(lat);
            if lm {
                p.landmarks.push(lat);
                p.since_landmark = 0;
            }
            while p.crumbs.len() > 48 {
                p.crumbs.pop_front();
            }
        }
        let g = contact_gap(world, p.follower, p.leader).unwrap_or_else(|| gap(fat, lat));
        let stepped = fat != p.last_at;
        p.follower_still = if stepped { 0 } else { p.follower_still + 1 };
        p.last_at = fat;
        // Arrival: in the patch, and either food in reach or her leader stopped here for a second. The second
        // clause is the leader's word that this is the place: the heap is eaten down between drops, and in the
        // first played run a pupil stood in the patch with her leader for 1,800 frames, never "arriving",
        // because the food in reach had gone home in other ants' crops.
        if patch.is_some_and(|pt| pt.holds_column(fat.0)) && (food_within(world, fat, k.arrive_reach) || p.leader_still > 60) {
            world.tandem.as_mut().expect("checked").pairs[i] = p;
            release(world, i, true);
            continue;
        }
        // A cascade run that has not arrived in time is given up (the seat's knob; 0 is never).
        if p.cascade && k.cascade_timeout > 0 && frame.saturating_sub(p.since) > k.cascade_timeout {
            world.tandem.as_mut().expect("checked").pairs[i] = p;
            release(world, i, false);
            continue;
        }
        // Arm B's column gives up after its timeout.
        if p.column && frame.saturating_sub(p.since) > k.column_timeout {
            world.tandem.as_mut().expect("checked").pairs[i] = p;
            release(world, i, false);
            continue;
        }
        let mut dir: Option<(i8, i8)> = None;
        match p.phase {
            Phase::Following => {
                if g <= k.contact {
                    p.last_contact = lat;
                }
                // Drop crumbs she has reached; step toward the oldest left.
                while p.crumbs.len() > 1 && p.crumbs.front().is_some_and(|&c| gap(c, fat) <= 1) {
                    let c = p.crumbs.pop_front().expect("len > 1");
                    if !p.column && p.landmarks.contains(&c) {
                        p.landmarks.retain(|&l| l != c);
                        p.met = p.met.saturating_add(1);
                        let pause_for = pause_len(&k, seed, p.follower, frame);
                        p.phase = Phase::Tell { until: frame + k.tell_frames, pause_for, landmark: true };
                    }
                }
                if matches!(p.phase, Phase::Following) && !p.column && stepped && rng::stream(seed, p.follower as u64, frame, RNG_SLOT_PAUSE).chance(k.pause_chance) {
                    let pause_for = pause_len(&k, seed, p.follower, frame);
                    p.phase = Phase::Tell { until: frame + k.tell_frames, pause_for, landmark: false };
                }
                if matches!(p.phase, Phase::Following) {
                    if g <= k.rest_gap && p.leader_still > 30 {
                        // **Give way.** A leader who has been trying to move for half a second with her pupil
                        // pressed against her is usually blocked by that pupil (first headless run: the pupil
                        // stood in the one tunnel cell above her leader, and both starved in place). Step away.
                        dir = toward(lat, fat).or(Some((0, -1)));
                    } else if g <= k.rest_gap {
                        dir = None;
                    } else {
                        let target = p.crumbs.front().copied().unwrap_or(lat);
                        dir = toward(fat, target).or_else(|| toward(fat, lat));
                    }
                    if g > k.break_gap && !p.column {
                        p.over_for += 1;
                        if p.over_for > k.break_frames {
                            p.phase = Phase::Lost { since: frame, centre: p.last_contact, leg: 0, east: fat.0 <= p.last_contact.0 };
                        }
                    } else {
                        p.over_for = 0;
                    }
                }
            }
            Phase::Tell { until, pause_for, landmark } => {
                dir = None;
                if frame >= until {
                    p.phase = Phase::Pausing { until: frame + pause_for, landmark };
                }
            }
            Phase::Pausing { until, landmark } => {
                dir = None;
                if frame >= until {
                    if g <= k.resume_learned {
                        if landmark {
                            p.learned = p.learned.saturating_add(1);
                            let t = world.tandem.as_mut().expect("checked");
                            t.counts.landmarks_learned += 1;
                            t.mark(p.follower, Mark::Landmark, frame);
                        }
                        p.phase = Phase::Following;
                    } else if g <= k.break_gap {
                        p.phase = Phase::Following;
                    } else {
                        p.phase = Phase::Lost { since: frame, centre: fat, leg: 0, east: lat.0 >= fat.0 };
                    }
                    p.over_for = 0;
                }
            }
            Phase::Lost { since, centre, leg, east } => {
                if g <= k.contact {
                    let t = world.tandem.as_mut().expect("checked");
                    t.counts.recontacts += 1;
                    t.mark(p.follower, Mark::Recontact, frame);
                    p.phase = Phase::Following;
                    p.crumbs.clear();
                    p.crumbs.push_back(lat);
                    p.over_for = 0;
                } else if frame.saturating_sub(since) > k.sweep_frames {
                    world.tandem.as_mut().expect("checked").pairs[i] = p;
                    release(world, i, false);
                    continue;
                } else {
                    let len = k.sweep_legs[usize::from(leg).min(k.sweep_legs.len() - 1)];
                    let end = centre.0 + if east { len } else { -len };
                    let (mut leg, mut east) = (leg, east);
                    // Blocked: she tries to climb for a while before turning back. The first cut reversed on any
                    // tick she had not stepped on, which is five ticks in six for a walking ant, so she paced a
                    // four-cell stretch for the whole search, leg counter racing, never reaching a leader eight
                    // cells off (caught by the sweep test after a merge moved the box).
                    let blocked = p.follower_still > 60;
                    if (east && fat.0 >= end) || (!east && fat.0 <= end) || blocked {
                        east = !east;
                        leg = leg.saturating_add(1);
                        p.follower_still = 0;
                    }
                    p.phase = Phase::Lost { since, centre, leg, east };
                    let dx = if east { 1 } else { -1 };
                    dir = Some((dx, if p.follower_still > 20 { -1 } else { 0 }));
                }
            }
        }
        let follower = p.follower;
        world.tandem.as_mut().expect("checked").pairs[i] = p;
        force(world, follower, dir);
        i += 1;
    }
    watch_trips(world);
}

fn fat_dead(world: &World, id: OrganismId) -> bool {
    head(world, id).is_none()
}

fn pause_len(k: &Knobs, seed: u64, id: OrganismId, frame: u64) -> u64 {
    let span = k.pause_max.saturating_sub(k.pause_min).max(1);
    k.pause_min + rng::stream(seed, id as u64, frame, RNG_SLOT_PAUSE_LEN).below(span as u32 + 1) as u64
}

/// The patch's income, "broken, still arrived", and the cascade: every
/// ant's first delivery of a trip whose memory lies in the patch (review 5:
/// never per cell put down, never on spoil).
fn watch_trips(world: &mut World) {
    let Some(patch) = world.tandem.as_ref().and_then(|t| t.patch) else { return };
    let frame = world.frame;
    // Once a second is plenty for a trip counter.
    if !frame.is_multiple_of(30) {
        return;
    }
    let seed = world.seed;
    let mut fired: Vec<OrganismId> = Vec::new();
    // **She was taught the patch, not a cell.** Her own pick-up anywhere in the heap moves her memory to that
    // cell; the recall head keeps a lesson only within `recall::SAME_PLACE` (6) of where it was planted, and the
    // heap is 20 columns wide, so in the first four-seed sweep most taught ants lost the lesson on their first
    // trip (the head tilted 17 decisions in a whole seed-1 run). A trip home from the patch renews it where she
    // last picked up, at the strength she was taught.
    let mut renew: Vec<(OrganismId, f32)> = Vec::new();
    let ids = world.live_organism_ids();
    for id in ids {
        let Some(st) = world.organism(id) else { continue };
        let trips = st.life.trip_deliveries;
        let from_patch = st.last_food.is_some_and(|(x, _, _)| patch.holds_column(x));
        let t = world.tandem.as_mut().expect("checked");
        let seen = t.trips_seen.insert(id, trips);
        let Some(seen) = seen else { continue };
        if trips <= seen || !from_patch {
            continue;
        }
        t.income.push_back(frame);
        t.counts.patch_trips += 1;
        if t.tree.contains_key(&id) {
            t.counts.tree_trips += 1;
            renew.push((id, t.lessons.get(&id).copied().unwrap_or(1.0)));
            if let Some(r) = t.released.iter_mut().find(|r| r.id == id && !r.arrived) {
                r.arrived = true;
                t.counts.broken_still_arrived += 1;
            }
            if t.knobs.cascade_c > 0.0 && rng::stream(seed, id as u64, frame, RNG_SLOT_CASCADE).chance(t.knobs.cascade_c) {
                fired.push(id);
            }
        }
    }
    let t = world.tandem.as_mut().expect("checked");
    while t.income.front().is_some_and(|&f| frame.saturating_sub(f) > 3_600) {
        t.income.pop_front();
    }
    let watch = t.knobs.watch_frames;
    t.released.retain(|r| frame.saturating_sub(r.at) <= watch);
    for (id, s) in renew {
        if let Some((x, y, _)) = world.organism(id).and_then(|st| st.last_food) {
            plant_lesson(world, id, (x, y), s);
        }
    }
    for id in fired {
        cascade_tap(world, id);
    }
}

/// A taught ant home from the patch taps a sister by the player's own rule
/// and leads her toward the place she remembers (arm A's scripted part).
fn cascade_tap(world: &mut World, leader: OrganismId) {
    let Some(t) = world.tandem.as_ref() else { return };
    if t.pairs.len() >= t.knobs.max_pairs || t.leads(leader) {
        return;
    }
    let reach = t.knobs.tap_reach;
    let k = t.knobs.clone();
    let frame = world.frame;
    let seed = world.seed;
    let Some(&id) = candidates(world, leader, reach).first() else { return };
    let s = energy_sense(world, id).unwrap_or(0.0);
    let Some(p) = accept_chance(&k, s) else { return };
    if !rng::stream(seed, id as u64, frame, RNG_SLOT_ACCEPT).chance(p) {
        return;
    }
    start_pair(world, leader, id, false, true);
}

/// **Cascade leaders walk to what they remember**: each tick, a leader who
/// is not the player and leads a pair is forced toward her last-food cell,
/// waiting while her pupil stops. Called from [`tick`]'s caller after it.
pub fn steer_cascade_leaders(world: &mut World) {
    let Some(t) = world.tandem.as_ref() else { return };
    let leaders: Vec<(OrganismId, bool)> = t
        .pairs
        .iter()
        .filter(|p| p.cascade)
        .map(|p| (p.leader, !matches!(p.phase, Phase::Following)))
        .collect();
    let unstick = t.knobs.cascade_unstick;
    let frame = world.frame;
    if let Some(t) = world.tandem.as_mut() {
        t.progress.retain(|id, _| leaders.iter().any(|(l, _)| l == id));
    }
    for (leader, wait) in leaders {
        let target = world.organism(leader).and_then(|s| s.last_food).map(|(x, y, _)| (x, y));
        let at = head(world, leader);
        let dir = if wait { None } else { at.zip(target).and_then(|(a, b)| toward(a, b)) };
        if !unstick {
            force(world, leader, dir);
            continue;
        }
        // **Out of the box**: progress is getting nearer the target, not merely moving (a chain that turns
        // round in place moves its head every decision and gets nowhere). Standing for her pupil is not stuck.
        let dist = at.zip(target).map_or(0, |(a, b)| gap(a, b));
        let t = world.tandem.as_mut().expect("read above");
        let e = t.progress.entry(leader).or_insert((dist, frame));
        if wait || dist < e.0 {
            *e = (dist, frame);
        }
        let stuck = frame.saturating_sub(e.1);
        let mut pick = crate::sim::steer::ForcedPick { dir, verb: None, once: false };
        if let Some(d) = dir.filter(|_| stuck > UNSTICK_FRAMES) {
            let alts = [(d.0, -1), (0, -1), (-d.0, -1), (-d.0, 0), (d.0, 1)];
            pick.dir = Some(alts[((stuck / UNSTICK_FRAMES - 1) as usize) % alts.len()]);
            if stuck > 5 * UNSTICK_FRAMES {
                pick.verb = Some(crate::sim::creature::options::Verb::Dig);
            }
        }
        crate::sim::steer::force(world, leader, pick);
    }
}

#[cfg(test)]
mod tests {
    //! On the demo's own box with the trained ant (`demo_scout.ron`,
    //! `set_new_ant(true)`): a claim checked on the hand-built ant does not
    //! transfer to a mind ant (`how-the-ant-works` s16).
    use super::*;
    use crate::lab::Lab;
    use crate::sim::steer::{self, ForcedPick};

    const PATCH: Patch = Patch { x: 248, r: 10 };

    fn box_at(frames: u64) -> Lab {
        let sc = crate::lab::scenario::Scenario::load("demo_scout").expect("demo_scout.ron ships");
        let mut lab = Lab::new(sc.bed.clone());
        lab.load_scenario(sc);
        lab.set_new_ant(true);
        while lab.world.frame < frames {
            lab.tick_for_harness();
        }
        lab
    }

    fn arm(lab: &mut Lab, arm: Arm) {
        let mut t = Tandems::new(arm, Some(PATCH));
        t.knobs.pause_chance = 0.0;
        lab.world.tandem = Some(Box::new(t));
    }

    /// A minded ant on the surface and the nearest sister she can reach, the
    /// first such pair in id order.
    fn pair_up(world: &World, reach: i32) -> Option<(OrganismId, OrganismId)> {
        let mut ids = world.live_organism_ids();
        ids.sort_unstable();
        ids.into_iter().filter(|&id| crate::sim::creature::mind::has(world, id)).find_map(|id| {
            let h = head(world, id)?;
            let top = world.room_surface_at(h.0)?;
            if h.1 > top + 1 {
                return None;
            }
            candidates(world, id, reach).first().map(|&f| (id, f))
        })
    }

    fn lead(world: &mut World, id: OrganismId, dir: Option<(i8, i8)>) {
        steer::force(world, id, ForcedPick { dir, verb: None, once: false });
    }

    fn failed(world: &World) -> u64 {
        world.minds.as_ref().map_or(0, |m| m.counts.failed)
    }

    /// Every cell's material and owner, and every live body's head.
    fn digest(world: &World) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |v: u64| h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3);
        let b = world.bounds().expect("the box sets bounds");
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let c = world.get(x, y);
                mix(u64::from(c.material.0));
                mix(u64::from(c.organism_id()));
            }
        }
        for id in world.live_organism_ids() {
            if let Some((x, y)) = head(world, id) {
                mix(u64::from(id) ^ ((x as u64) << 20) ^ ((y as u64) << 40));
            }
        }
        h
    }

    /// **Off is off**: no tandem state and an armed module with no pair leave
    /// the box byte-identical; a live pair does not (the positive control,
    /// without which "identical" could mean the module never ran at all).
    #[test]
    fn an_armed_module_with_no_pair_changes_nothing() {
        let run = |mode: u8| {
            let mut lab = box_at(600);
            if mode >= 1 {
                arm(&mut lab, Arm::Scout);
            }
            if mode == 2 {
                let (l, f) = pair_up(&lab.world, 6).expect("a leader with a sister in reach");
                start_pair(&mut lab.world, l, f, false, false);
            }
            for _ in 0..300 {
                lab.tick_for_harness();
            }
            digest(&lab.world)
        };
        let (off, idle, live) = (run(0), run(1), run(2));
        assert_eq!(off, idle, "an armed module with no pair moved the box");
        assert_ne!(off, live, "a live pair changed nothing: the digest is blind or the module never ran");
    }

    /// **She follows by touch, behind and not on top**: a leader walked east
    /// for 900 frames is followed, the pupil in contact (gap <= break) on
    /// nearly every Following frame, at rest only beside the body, and every
    /// pick the pair forced was a pick the body could act on.
    #[test]
    fn a_pupil_follows_her_leader_by_touch() {
        let mut lab = box_at(600);
        arm(&mut lab, Arm::Scout);
        let (l, f) = pair_up(&lab.world, 6).expect("a leader with a sister in reach");
        start_pair(&mut lab.world, l, f, false, false);
        let (l0, f0) = (head(&lab.world, l).unwrap(), head(&lab.world, f).unwrap());
        let failed0 = failed(&lab.world);
        let (mut near, mut following, mut gaps) = (0u32, 0u32, [0u32; 8]);
        for _ in 0..900 {
            // The seat's own leash (`lab::demo::scout::frame`): the leader is held while her pupil lags three
            // cells or more, or stops. Unheld, a quicker leader outwalks a slower pupil in 150 frames.
            let hold = lab.world.tandem.as_deref().and_then(|t| t.pair_of(f)).is_some_and(|p| {
                !matches!(p.phase, Phase::Following) || contact_gap(&lab.world, f, l).is_none_or(|g| g >= 3)
            });
            lead(&mut lab.world, l, (!hold).then_some((1, 0)));
            lab.tick_for_harness();
            let t = lab.world.tandem.as_deref().unwrap();
            let Some(p) = t.pair_of(f) else { break };
            if matches!(p.phase, Phase::Following) {
                let g = contact_gap(&lab.world, f, l).expect("both alive");
                following += 1;
                gaps[(g as usize).min(7)] += 1;
                near += u32::from(g <= t.knobs.break_gap);
            }
        }
        let (l1, f1) = (head(&lab.world, l).unwrap_or(l0), head(&lab.world, f).unwrap_or(f0));
        assert!(l1.0 - l0.0 >= 15, "the leader did not get anywhere ({l0:?} -> {l1:?}); the test needs a walk");
        assert!(l1.0 - f1.0 <= 6, "the pupil did not keep up: leader {l0:?}->{l1:?}, pupil {f0:?}->{f1:?}, gaps {gaps:?}");
        assert!(following > 300, "the pair left Following early ({following} frames), gaps {gaps:?}");
        assert!(near * 10 >= following * 9, "contact on only {near} of {following} Following frames, gaps {gaps:?}");
        assert_eq!(failed(&lab.world), failed0, "a forced pick the body could not act on");
    }

    fn paused_pair(leader_walks: bool) -> Tandems {
        let mut lab = box_at(600);
        arm(&mut lab, Arm::Scout);
        let (l, f) = pair_up(&lab.world, 6).expect("a leader with a sister in reach");
        start_pair(&mut lab.world, l, f, false, false);
        let frame = lab.world.frame;
        let t = lab.world.tandem.as_mut().unwrap();
        t.pairs[0].met = 1;
        t.pairs[0].phase = Phase::Tell { until: frame + t.knobs.tell_frames, pause_for: 75, landmark: true };
        for _ in 0..240 {
            lead(&mut lab.world, l, leader_walks.then_some((1, 0)));
            lab.tick_for_harness();
        }
        *lab.world.tandem.take().unwrap()
    }

    /// **A stop is the leader's to keep**: standing through a pupil's stop at
    /// a landmark, she learns it and follows on; walking on through it, the
    /// run breaks. The same pair, the same stop, only the leader differs.
    #[test]
    fn walking_through_a_stop_breaks_the_run_and_waiting_keeps_it() {
        let kept = paused_pair(false);
        let p = kept.pairs.first().expect("a waited-for pupil is still in her run");
        assert!(matches!(p.phase, Phase::Following), "after the stop she should follow again, is {:?}", p.phase);
        assert_eq!((p.learned, kept.counts.landmarks_learned), (1, 1), "the landmark was not learned");
        let broke = paused_pair(true);
        let lost = broke.pairs.first().is_none_or(|p| matches!(p.phase, Phase::Lost { .. }));
        assert!(lost, "a pupil left behind at her stop is not searching or released: {:?}", broke.pairs.first().map(|p| p.phase));
        assert_eq!(broke.counts.landmarks_learned, 0);
    }

    /// **The search finds a leader who waits**: a pupil lost a few cells
    /// short sweeps the ground, first the way she was going (as `tick` sets
    /// it), and touches her again.
    #[test]
    fn a_lost_pupil_sweeps_and_finds_a_leader_who_waits() {
        let mut lab = box_at(600);
        arm(&mut lab, Arm::Scout);
        let (l, f) = pair_up(&lab.world, 6).expect("a leader with a sister in reach");
        start_pair(&mut lab.world, l, f, false, false);
        // The pupil stops for good; the leader walks eight cells on, then stands.
        let frame = lab.world.frame;
        lab.world.tandem.as_mut().unwrap().pairs[0].phase = Phase::Pausing { until: frame + 100_000, landmark: false };
        let l0 = head(&lab.world, l).unwrap();
        while head(&lab.world, l).is_some_and(|h| h.0 < l0.0 + 8) && lab.world.frame < frame + 900 {
            lead(&mut lab.world, l, Some((1, 0)));
            lab.tick_for_harness();
        }
        let fat = head(&lab.world, f).expect("alive");
        let now = lab.world.frame;
        lab.world.tandem.as_mut().unwrap().pairs[0].phase = Phase::Lost { since: now, centre: fat, leg: 0, east: true };
        let sweep = lab.world.tandem.as_ref().unwrap().knobs.sweep_frames;
        for _ in 0..sweep + 30 {
            lead(&mut lab.world, l, None);
            lab.tick_for_harness();
        }
        let t = lab.world.tandem.as_deref().unwrap();
        assert!(t.counts.recontacts >= 1, "the sweep never touched a standing leader {} cells off", head(&lab.world, l).map_or(0, |h| h.0 - fat.0));
    }

    /// **Refusals are graded by hunger**: too weak refuses outright, and from
    /// there the willing share only falls as she fills.
    #[test]
    fn the_accept_chance_falls_as_she_fills() {
        let k = Knobs::default();
        assert_eq!(accept_chance(&k, k.weak - 0.01), None);
        let mut last = f32::INFINITY;
        for i in 0..=100 {
            let s = k.weak + (1.0 - k.weak) * i as f32 / 100.0;
            let p = accept_chance(&k, s).expect("not weak");
            assert!(p <= last + 1e-6, "accept chance rose from {last} to {p} at sense {s}");
            assert!((0.0..=1.0).contains(&p));
            last = p;
        }
        assert!(accept_chance(&k, k.weak).unwrap() > accept_chance(&k, 0.99).unwrap());
    }

    /// **Arm B's column is capped and gives up**: a call fills at most
    /// `max_pairs`, and a column that never gets there is released on its
    /// timeout, not kept for ever.
    #[test]
    fn the_column_is_capped_and_times_out() {
        let mut lab = box_at(600);
        arm(&mut lab, Arm::Column);
        {
            let k = &mut lab.world.tandem.as_mut().unwrap().knobs;
            (k.weak, k.accept_hi, k.accept_lo, k.accept_full, k.call_reach, k.max_pairs, k.column_timeout) = (0.0, 1.0, 1.0, 1.0, 40, 3, 300);
        }
        let (l, _) = pair_up(&lab.world, 40).expect("a leader with sisters in reach");
        assert!(matches!(tap(&mut lab.world, l), TapResult::Accepted(_)));
        let t = lab.world.tandem.as_deref().unwrap();
        assert_eq!(t.pairs.len(), 3, "the call ignored its cap (or found too few sisters for the test)");
        assert!(t.pairs.iter().all(|p| p.column));
        for _ in 0..360 {
            lead(&mut lab.world, l, None);
            lab.tick_for_harness();
        }
        let t = lab.world.tandem.as_deref().unwrap();
        assert!(t.pairs.is_empty(), "the column outlived its timeout");
        assert_eq!(t.counts.started, 3);
    }

    /// **The cascade is per trip home, from the patch, by a taught ant**:
    /// one delivery is one roll; a delivery remembered elsewhere, or by an
    /// ant nobody taught, is none.
    #[test]
    fn the_cascade_rolls_once_per_trip_home_from_the_patch() {
        let mut lab = box_at(600);
        arm(&mut lab, Arm::Scout);
        while !lab.world.frame.is_multiple_of(30) {
            lab.tick_for_harness();
        }
        let (taught, sister) = pair_up(&lab.world, 3).expect("an ant with a sister in tap reach");
        let untaught = lab
            .world
            .live_organism_ids()
            .into_iter()
            .filter(|&i| i != taught && i != sister && crate::sim::creature::mind::has(&lab.world, i))
            .min()
            .expect("a third ant");
        {
            let t = lab.world.tandem.as_mut().unwrap();
            t.knobs.cascade_c = 1.0;
            (t.knobs.weak, t.knobs.accept_hi, t.knobs.accept_lo, t.knobs.accept_full) = (0.0, 1.0, 1.0, 1.0);
            t.tree.insert(taught, 0);
        }
        let frame = lab.world.frame;
        for (id, x) in [(taught, PATCH.x), (untaught, PATCH.x)] {
            lab.world.organism_mut(id).unwrap().last_food = Some((x, 150, frame));
        }
        watch_trips(&mut lab.world); // first sight: nothing to compare with
        lab.world.organism_mut(taught).unwrap().life.trip_deliveries += 1;
        lab.world.organism_mut(untaught).unwrap().life.trip_deliveries += 1;
        watch_trips(&mut lab.world);
        let c = lab.world.tandem.as_deref().unwrap().counts;
        assert_eq!((c.patch_trips, c.tree_trips, c.cascade_runs), (2, 1, 1), "{c:?}");
        // Remembered outside the patch: a trip, but not the patch's.
        lab.world.organism_mut(taught).unwrap().last_food = Some((PATCH.x - 100, 150, frame));
        lab.world.organism_mut(taught).unwrap().life.trip_deliveries += 1;
        watch_trips(&mut lab.world);
        let c2 = lab.world.tandem.as_deref().unwrap().counts;
        assert_eq!((c2.patch_trips, c2.tree_trips, c2.cascade_runs), (2, 1, 1), "{c2:?}");
    }

    /// **Same seed, same run**: the module's draws are keyed streams, not a
    /// shared generator.
    #[test]
    fn a_run_repeats_exactly() {
        let run = || {
            let mut lab = box_at(600);
            let mut t = Tandems::new(Arm::Scout, Some(PATCH));
            t.knobs.pause_chance = 1.0 / 20.0;
            lab.world.tandem = Some(Box::new(t));
            let (l, f) = pair_up(&lab.world, 6).expect("pair");
            start_pair(&mut lab.world, l, f, false, false);
            for _ in 0..600 {
                lead(&mut lab.world, l, Some((1, 0)));
                lab.tick_for_harness();
            }
            (format!("{:?}", lab.world.tandem.as_ref().unwrap().counts), head(&lab.world, f), digest(&lab.world))
        };
        assert_eq!(run(), run());
    }
}
