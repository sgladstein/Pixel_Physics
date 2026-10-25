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
    /// A follower within this many cells (Chebyshev) of the leader's head is
    /// in contact.
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
}

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
        if let Some(cell) = patch.and_then(|p| nearest_food(world, (p.x, head(world, id).map_or(0, |h| h.1)), p.r)) {
            plant_lesson(world, id, cell, lesson);
        }
    }
    unforce(world, id);
    if p.cascade {
        unforce(world, p.leader);
    }
}

/// Hand the lesson to the remembered-place head (keeper seat). A memory of
/// the patch with strength `s`.
fn plant_lesson(world: &mut World, id: OrganismId, cell: (i32, i32), s: f32) {
    let frame = world.frame;
    if let Some(st) = world.organism_mut(id) {
        st.last_food = Some((cell.0, cell.1, frame));
    }
    let _ = s;
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
                    if (east && fat.0 >= end) || (!east && fat.0 <= end) || !stepped && frame.saturating_sub(since) % 30 == 29 {
                        east = !east;
                        leg = leg.saturating_add(1);
                    }
                    p.phase = Phase::Lost { since, centre, leg, east };
                    dir = Some((if east { 1 } else { -1 }, 0));
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
    for (leader, wait) in leaders {
        let target = world.organism(leader).and_then(|s| s.last_food).map(|(x, y, _)| (x, y));
        let at = head(world, leader);
        let dir = if wait { None } else { at.zip(target).and_then(|(a, b)| toward(a, b)) };
        force(world, leader, dir);
    }
}
