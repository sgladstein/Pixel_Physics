//! **Ants that judge a visitor by its smell: the inspection ladder** (guest
//! seat demo, 2026-10-25; design `Reports/teaching-the-colony-2026-10-25/
//! seat-guest.md`, plan and reviews in the seat-demos folder, `guest/`).
//!
//! **Why it is needed.** A trained ant sorts every other animal by colony
//! *number* (`options::describe`'s NESTMATE/STRANGER), offers Feed only to
//! its own colony and Attack to anything else, and never reads the
//! three-number scent every animal carries. Step 0 measured it
//! (`examples/guestprobe.rs`, `played_bed`, 3 seeds): a beetle of another
//! colony standing beside the door died in 49-879 frames to the mind's
//! Attack, and its scent near or far from the nest's changed **nothing**,
//! byte for byte. Given the host's number instead it was never bitten. So a
//! visitor could only be family or a target, with no middle to earn.
//!
//! **What this does.** For a *registered* visitor ([`Visitors`], `None` in
//! every world but the guest demo), an ant that touches it rolls one rung of
//! a ladder on how far the visitor's scent is from its own, judged by **the
//! ant's own tolerance** (`tolerance_radius`, the radius the mouth and the
//! kin sense already use): ignore, antennate, display, nip. The rung is
//! carried out through the ant's own mind, never around it: a held ant's
//! option list is cut to the one `Rest` (or one Walk away) by
//! [`force_options`], so its bills, fall, memory and reschedule run as for
//! any decision (review of plan v1, finding 1: an early return skipped the
//! body). The mind's Attack is never offered on a visitor (`options.rs`), so
//! the ladder's nip is the only harm and it is graded.
//!
//! **Hand-made behaviour, said plainly.** This is a body rule, as nursing by
//! touch is, not something the colony evolved. Some ants are reliably
//! pickier than others, but that is a per-ant die ([`pick`]), not a gene:
//! tolerance barely varies in a colony and trained ants do not mutate in the
//! lab. Whether any of it ever runs outside the demo is the owner's call.
//!
//! **Off is off.** With `World::visitors` at `None` every hook is one
//! `Option` read (`creature_tick`, `mind::tick`, `options::build_with`,
//! `lab::tick_world`).

use super::{cry_alarm, crop_take, expressed_traits, kill_detail, kill_side, note_food, reconcile_chain, scent_of, tolerance_radius, DIRS};
use crate::sim::cell::{Cell, OrganismId};
use crate::sim::material::Known;
use crate::sim::organism::{DeathCause, SpeciesId, TRAIT_SCENT_A, TRAIT_SCENT_B, TRAIT_SCENT_C};
use crate::sim::pheromone::{Channel, ALARM_DEPOSIT};
use crate::sim::world::Account;
use crate::sim::{rng, World};
use std::collections::VecDeque;

use super::options::{Opt, Verb};

const SCENT_SLOTS: [usize; 3] = [TRAIT_SCENT_A, TRAIT_SCENT_B, TRAIT_SCENT_C];

/// Frames per second at the lab's 1x, which every time below is stated in.
const FPS: f32 = 60.0;
/// A new contact episode starts on the first touch after this many frames
/// without one (review finding 3: one roll per episode, not one per frame of
/// a long touch).
const EPISODE_GAP: u64 = 120;
/// An ant rolls at most once per this many frames, whatever touches it.
const COOLDOWN: u64 = 360;
/// How long each rung holds the ant on the visitor.
const HOLD_ANTENNATE: u64 = 90;
const HOLD_DISPLAY: u64 = 60;
/// After the display's hold, the ant backs off for this long (one Walk away).
const AWAY: u64 = 18;
const HOLD_NIP: u64 = 36;
/// One nip's share of a cell: three inside the engine's heal window take one
/// (`creature::wound_heal_frames`, 0.25 per 750 frames).
const NIP: f32 = 0.34;
/// Below this effective gap a nip is impossible: a good disguise is never
/// killed by chance (finding 3). The floors keep antennate and display
/// possible at any gap, so the middle is always there.
const NIP_BELOW: f32 = 0.8;
const FLOOR: f32 = 0.03;
const CEILING: f32 = 0.97;
/// The ladder's three steps, as centres of logistic curves on the effective
/// gap `g'`, and their slope. Starting values, tuned by playing (plan step 6).
const CENTRE_ANTENNATE: f32 = 0.5;
const CENTRE_DISPLAY: f32 = 0.9;
const CENTRE_NIP: f32 = 1.3;
const SLOPE: f32 = 6.0;
/// The alarm at the spot adds at most this much to `g'` (finding 3: a nip's
/// own alarm must not snowball into a death spiral).
const ALARM_TERM_MAX: f32 = 0.25;
/// A hungry ant is a little pickier.
const HUNGER_TERM: f32 = 0.2;
/// A refused beg makes the roll that follows pickier.
const BEG_PENALTY: f32 = 0.3;
/// Rub: the guest's scent moves this share of the gap per press; the rubbed
/// ant's moves a quarter as far (finding 9: three rubbed workers must not be
/// regrouped into a colony of their own).
const RUB_STEP: f32 = 0.10;
const RUB_ANT_SHARE: f32 = 0.25;
/// One rub (or one beg) per ant per this many frames.
const RUB_COOLDOWN: u64 = 120;
/// Soak and fade, as time constants in seconds at 1x (finding 8).
const SOAK_TAU_S: f32 = 20.0;
const FADE_TAU_S: f32 = 60.0;
/// Soak is refused while the guest is further than this from the nest's
/// odour, in units of the colony's tolerance.
const SOAK_REFUSE_G: f32 = 2.0;
/// Columns either side of a nest's door, and rows above its surface, that
/// count as in the nest for the soak.
const SOAK_REACH: i32 = 16;
/// Rows below the door's surface that count as in the nest.
const SOAK_DEPTH: i32 = 60;
/// A donor gives from its bank down to this share of its species' grant,
/// never below (finding 6).
const DONOR_FLOOR: f32 = 0.7;
/// The share of the donor's spare bank one accepted beg moves, and the most
/// one beg can move: a droplet, not a meal. A grown trained colony's adults
/// hold 3-5 grants each (`guestprobe` DONORS, `steady_income`, 6 seeds), so
/// uncapped one beg moved ~175 J and ten begs filled the guest for the rest
/// of the run.
const BEG_SHARE: f32 = 0.25;
const BEG_MAX: f32 = 30.0;
/// Events kept for the ledger.
const EVENTS_KEPT: usize = 64;

/// **Beg wariness** (guest seat stakes, 2026-10-30; [`Wariness`]): what one
/// beg adds to the colony's wariness of the guest, and to the asked ant's
/// own, in gap units. The 10-30 review measured a disguised beggar fed on
/// 76-92% of begs with no cost to asking again, so begging was a toggle.
/// Starting values; tuned by the seat's paired scripts (`guestgif
/// script=good|spam`), whose numbers are in the seat's README.
pub const WARY_BEG_COLONY: f32 = 0.06;
pub const WARY_BEG_ANT: f32 = 0.25;
/// Time constants the wariness fades on, in seconds at 1x.
const WARY_COLONY_TAU_S: f32 = 20.0;
const WARY_ANT_TAU_S: f32 = 30.0;
/// The most either term can add to the gap.
const WARY_COLONY_MAX: f32 = 0.6;
const WARY_ANT_MAX: f32 = 1.0;
/// The share of the wariness a ladder roll reads (a beg's acceptance reads
/// all of it). At the full term a disguised spammer lost all three roves in
/// 45-110 s on four seeds (18-27 nips each), which read as a trap rather
/// than as greed catching up with you.
const WARY_ROLL_SHARE: f32 = 0.5;
/// A rub takes this much off the colony's wariness, and halves the rubbed
/// ant's own: grooming is how a guest buys trust back.
pub const WARY_RUB_CALM: f32 = 0.08;

const RNG_SLOT_LADDER: u64 = 50;
const RNG_SLOT_PICK: u64 = 51;
const RNG_SLOT_BEG: u64 = 52;

/// One rung of the ladder, or what a beg came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rung {
    Ignore,
    Antennate,
    Display,
    Nip,
    /// A beg answered.
    Fed,
    /// A beg the ant had nothing to give to (no roll).
    Empty,
    /// The ant was rubbed.
    Rubbed,
}

/// What an ant is doing about a visitor, and when it may next decide to.
#[derive(Clone, Copy, Debug)]
pub struct Mark {
    pub rung: Rung,
    /// The frame the rung was rolled (or the beg answered): what a HUD reads.
    pub at: u64,
    pub visitor: OrganismId,
    /// Held on the visitor (a forced Rest) until this frame.
    pub hold_until: u64,
    /// Then backing off (a forced Walk away) until this frame.
    pub away_until: u64,
    /// No new roll before this frame.
    pub cooldown_until: u64,
    /// The last frame it touched a visitor: an episode is touches with no gap
    /// over [`EPISODE_GAP`].
    pub last_touch: u64,
    /// No rub or beg on this ant before this frame.
    pub rub_until: u64,
}

/// A registered visitor: the guest's body, and the scent it fades back to.
#[derive(Clone, Copy, Debug)]
pub struct Visitor {
    pub id: OrganismId,
    /// Set on landing; [`tick`] relaxes the scent toward it.
    pub home_scent: [f32; 3],
    /// The visitor's gap to the host colony, as last measured (for a HUD's
    /// tint): its scent against the nearest seeded nest odour, in units of
    /// the mean tolerance the ladder has met.
    pub gap: f32,
}

/// The guest's keys this tick, set by the seat and spent by [`tick`].
#[derive(Clone, Copy, Debug, Default)]
pub struct GuestInput {
    pub rub: bool,
    pub soak: bool,
    pub beg: bool,
    pub kill: bool,
    pub eat: bool,
}

/// Counters, colony-wide for the visit (the "did it fire" numbers).
#[derive(Clone, Copy, Debug, Default)]
pub struct VisitCounts {
    pub episodes: u64,
    pub rolls: u64,
    pub ignored: u64,
    pub antennated: u64,
    pub displayed: u64,
    pub nipped: u64,
    pub cells_lost: u64,
    pub deaths: u64,
    pub rubs: u64,
    pub soak_frames: u64,
    pub soak_refused: u64,
    pub begs: u64,
    pub fed: u64,
    pub fed_j: f64,
    pub refused: u64,
    pub empty: u64,
    pub larvae_killed: u64,
    pub corpses_eaten: u64,
    pub eaten_j: f64,
    pub rejected_attacks: u64,
}

/// One line of the ledger.
#[derive(Clone, Copy, Debug)]
pub struct VisitEvent {
    pub frame: u64,
    pub rung: Rung,
    pub ant: OrganismId,
    pub joules: f32,
}

/// **How wary the colony is of the guest's begging** (guest seat stakes,
/// 2026-10-30). `Visitors::wary`, `None` unless the seat's stakes are on, so
/// `stakes=0` and every test written before it run the ladder unchanged.
///
/// Every beg adds [`WARY_BEG_COLONY`] to a colony-wide term and
/// [`WARY_BEG_ANT`] to the asked ant's own; both fade, and a rub calms them
/// ([`WARY_RUB_CALM`]). The sum is added to the gap the beg's acceptance and
/// every ladder roll read, so greed is graded: the same ant asked again soon
/// refuses, a colony asked too often starts displaying, and a refused beg's
/// roll lands on the nip side once the sum passes the nip line. Stored with
/// the frame it was last written and faded lazily on read, so an idle guest
/// costs nothing per tick.
#[derive(Clone, Debug, Default)]
pub struct Wariness {
    colony: f32,
    colony_at: u64,
    ants: crate::sim::fxhash::FxHashMap<OrganismId, (f32, u64)>,
}

fn faded(v: f32, at: u64, now: u64, tau_s: f32) -> f32 {
    v * (-(now.saturating_sub(at) as f32) / (FPS * tau_s)).exp()
}

impl Wariness {
    /// The colony-wide term now.
    pub fn colony(&self, now: u64) -> f32 {
        faded(self.colony, self.colony_at, now, WARY_COLONY_TAU_S)
    }

    /// One ant's own term now.
    pub fn ant(&self, ant: OrganismId, now: u64) -> f32 {
        self.ants.get(&ant).map_or(0.0, |&(v, at)| faded(v, at, now, WARY_ANT_TAU_S))
    }

    /// The whole addition to `ant`'s gap now.
    pub fn term(&self, ant: OrganismId, now: u64) -> f32 {
        self.colony(now) + self.ant(ant, now)
    }

    /// Add `colony` to the colony's term and `own` to `ant`'s (either may be
    /// negative), each kept in `[0, max]`.
    pub fn add(&mut self, now: u64, ant: Option<OrganismId>, colony: f32, own: f32) {
        self.colony = (self.colony(now) + colony).clamp(0.0, WARY_COLONY_MAX);
        self.colony_at = now;
        if let Some(a) = ant {
            let v = (self.ant(a, now) + own).clamp(0.0, WARY_ANT_MAX);
            self.ants.insert(a, (v, now));
        }
    }

    /// A rub of `ant`: the colony calms by [`WARY_RUB_CALM`], the ant halves.
    pub fn calm(&mut self, now: u64, ant: OrganismId) {
        let own = self.ant(ant, now);
        self.add(now, Some(ant), -WARY_RUB_CALM, -0.5 * own);
    }

    fn prune(&mut self, now: u64) {
        self.ants.retain(|_, &mut (v, at)| faded(v, at, now, WARY_ANT_TAU_S) > 0.01);
    }
}

/// The wariness term for `ant`, 0 with the stakes off.
fn wary_term(world: &World, ant: OrganismId) -> f32 {
    world.visitors.as_ref().and_then(|v| v.wary.as_ref()).map_or(0.0, |w| w.term(ant, world.frame))
}

/// **The visitors an ant colony judges by smell**, and everything the ladder
/// keeps. `World::visitors`; `None` everywhere but the guest demo.
#[derive(Clone, Debug)]
pub struct Visitors {
    pub list: Vec<Visitor>,
    /// The species that judges (the ant). Other species pass by.
    pub host: SpeciesId,
    pub marks: crate::sim::fxhash::FxHashMap<OrganismId, Mark>,
    pub counts: VisitCounts,
    pub events: VecDeque<VisitEvent>,
    pub input: GuestInput,
    /// Running mean of the tolerance radii the ladder has read, for the
    /// visitor's [`Visitor::gap`] when no ant is touching it.
    pub mean_radius: f32,
    /// The guest seat's beg wariness ([`Wariness`]): `None` (the default)
    /// is the ladder as it was before the seat had stakes.
    pub wary: Option<Wariness>,
}

impl Visitors {
    pub fn new(host: SpeciesId) -> Self {
        Self {
            list: Vec::new(),
            host,
            marks: Default::default(),
            counts: VisitCounts::default(),
            events: VecDeque::new(),
            input: GuestInput::default(),
            mean_radius: 1.0,
            wary: None,
        }
    }

    fn note(&mut self, frame: u64, rung: Rung, ant: OrganismId, joules: f32) {
        if self.events.len() == EVENTS_KEPT {
            self.events.pop_front();
        }
        self.events.push_back(VisitEvent { frame, rung, ant, joules });
    }
}

/// **Register `id` as a visitor** whose scent fades back toward `home`, and
/// set its scent there now. Creates `World::visitors` for `host` if needed.
pub fn register(world: &mut World, host: SpeciesId, id: OrganismId, home: [f32; 3]) {
    for (slot, v) in SCENT_SLOTS.iter().zip(home) {
        world.set_organism_trait(id, *slot, v.clamp(-1.0, 1.0));
    }
    let v = world.visitors.get_or_insert_with(|| Visitors::new(host));
    v.list.retain(|x| x.id != id);
    v.list.push(Visitor { id, home_scent: home.map(|s| s.clamp(-1.0, 1.0)), gap: f32::NAN });
}

/// **A scent `dist` from `nest`**, toward the farthest corner of the scent
/// cube from it, so the clamp to `[-1, 1]` takes as little as possible off
/// the distance asked for (review finding 10). Returns the point and the
/// distance it actually lies at.
pub fn scent_away_from(nest: [f32; 3], dist: f32) -> ([f32; 3], f32) {
    let corner = nest.map(|s| if s >= 0.0 { -1.0 } else { 1.0 });
    let d = [corner[0] - nest[0], corner[1] - nest[1], corner[2] - nest[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
    let p = [0, 1, 2].map(|k| (nest[k] + d[k] / len * dist).clamp(-1.0, 1.0));
    (p, distance(&p, &nest))
}

fn distance(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    super::scent_distance_sq(a, b).sqrt()
}

/// Is `id` a registered visitor? `false` with the ladder off.
#[inline]
pub fn is_visitor(world: &World, id: OrganismId) -> bool {
    world.visitors.as_ref().is_some_and(|v| v.list.iter().any(|x| x.id == id))
}

/// The visitor's own scent (raw: the ladder writes it raw).
fn visitor_scent(world: &World, id: OrganismId) -> Option<[f32; 3]> {
    world.organism(id).map(|s| scent_of(&s.traits))
}

/// **The ant's stable pickiness**, a per-id die in `[-0.2, 0.2]`: some ants
/// are reliably stricter. Keyed on the id alone, so it never re-phases.
pub fn pick(world: &World, ant: OrganismId) -> f32 {
    rng::stream(world.seed, ant as u64, 0, RNG_SLOT_PICK).unit_f32() * 0.4 - 0.2
}

/// **The gap the ant reads**: the visitor's scent against the ant's own,
/// over the ant's own tolerance radius, both as the ant's body expresses them
/// (`is_living_kin_id` reads them so). The host judges.
pub fn gap(world: &World, ant: OrganismId, visitor: [f32; 3]) -> Option<(f32, f32)> {
    let s = world.organism(ant)?;
    let t = expressed_traits(s, world.plasticity, world.trait_reach);
    let r = tolerance_radius(&t).max(1e-3);
    Some((distance(&visitor, &scent_of(&t)) / r, r))
}

fn logistic(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// **The ladder's odds at effective gap `g`**: the chances of at least an
/// antennation, at least a display, and a nip, in that order, each no larger
/// than the one before. `scale` multiplies every non-ignore chance (a rubbed
/// ant rolls at half odds). The first two never reach 0 or 1; a nip is 0
/// below [`NIP_BELOW`].
pub fn odds(g: f32, scale: f32) -> (f32, f32, f32) {
    let p1 = logistic(SLOPE * (g - CENTRE_ANTENNATE)).clamp(FLOOR, CEILING) * scale;
    let p2 = (logistic(SLOPE * (g - CENTRE_DISPLAY)).clamp(FLOOR, CEILING) * scale).min(p1);
    let p3 = if g < NIP_BELOW { 0.0 } else { (logistic(SLOPE * (g - CENTRE_NIP)).min(CEILING) * scale).min(p2) };
    (p1, p2, p3)
}

/// **An ant touching a visitor rolls the ladder** (`creature_tick`, before the
/// mind decides). Marks the ant; never returns early, never skips a body
/// tick. A nip lands now; the hold is carried out by [`force_options`].
pub fn meet(world: &mut World, ant: OrganismId, (x, y): (i32, i32)) {
    let Some(v) = world.visitors.as_ref() else { return };
    if world.organism(ant).is_none_or(|s| s.species != v.host || s.brood.is_some()) {
        return;
    }
    // The contact: a visitor cell in the ant's head 8-neighbourhood.
    let mut touch: Option<(OrganismId, (i32, i32))> = None;
    for vis in &v.list {
        let Some(st) = world.organism(vis.id) else { continue };
        // Prefer the tail: a nip takes the head last.
        for &(cx, cy) in st.chain.iter().rev() {
            if (cx - x).abs() <= 1 && (cy - y).abs() <= 1 {
                touch = Some((vis.id, (cx, cy)));
                break;
            }
        }
        if touch.is_some() {
            break;
        }
    }
    let Some((visitor, _)) = touch else { return };
    // **The bite lands on the rear cell**, whichever cell was touched: the
    // head is taken last, so a nipped guest loses its body a cell at a time
    // (three nips a cell) and dies only when the head goes -- a graded
    // death, not the first unlucky nip on the head (seen in the first GIF:
    // a two-cell rove landing on a crowded mound lost its head to three
    // nips inside a second).
    let Some(cell) = world.organism(visitor).and_then(|s| s.chain.last().copied()) else { return };
    let frame = world.frame;
    let mark = v.marks.get(&ant).copied();
    let new_episode = mark.is_none_or(|m| frame.saturating_sub(m.last_touch) > EPISODE_GAP);
    let cooled = mark.is_none_or(|m| frame >= m.cooldown_until);
    if let Some(m) = world.visitors.as_mut().and_then(|v| v.marks.get_mut(&ant)) {
        m.last_touch = frame;
    }
    if new_episode {
        if let Some(v) = world.visitors.as_mut() {
            v.counts.episodes += 1;
        }
    }
    if !(new_episode && cooled) {
        if mark.is_none() {
            // First touch with no roll due cannot happen (no mark means
            // cooled); kept for clarity.
        }
        return;
    }
    roll(world, ant, visitor, cell, (x, y), 1.0, 0.0);
}

/// **One roll of the ladder** for `ant` against `visitor`, whose cell `cell`
/// is beside the ant's head at `head`. `scale` multiplies the non-ignore odds;
/// `penalty` is added to the gap (a refused beg).
fn roll(world: &mut World, ant: OrganismId, visitor: OrganismId, cell: (i32, i32), head: (i32, i32), scale: f32, penalty: f32) -> Rung {
    let frame = world.frame;
    let Some(vs) = visitor_scent(world, visitor) else { return Rung::Ignore };
    let Some((g, radius)) = gap(world, ant, vs) else { return Rung::Ignore };
    let alarm = f32::from(world.pheromone_at(Channel::Alarm, head.0, head.1)) / f32::from(ALARM_DEPOSIT);
    let hunger = world
        .organism(ant)
        .zip(world.species.creature_shared(world.organism(ant).map_or(SpeciesId(0), |s| s.species)))
        .map_or(0.0, |(s, d)| (1.0 - s.energy / d.start_energy.max(1.0)).clamp(0.0, 1.0));
    let g_eff = g + (0.5 * alarm).min(ALARM_TERM_MAX) + pick(world, ant) + HUNGER_TERM * hunger + penalty + WARY_ROLL_SHARE * wary_term(world, ant);
    let (p1, p2, p3) = odds(g_eff, scale);
    let u = rng::stream(world.seed, ant as u64, frame, RNG_SLOT_LADDER).unit_f32();
    let rung = if u < p3 {
        Rung::Nip
    } else if u < p2 {
        Rung::Display
    } else if u < p1 {
        Rung::Antennate
    } else {
        Rung::Ignore
    };
    let (hold, away) = match rung {
        Rung::Antennate => (HOLD_ANTENNATE, 0),
        Rung::Display => (HOLD_DISPLAY, AWAY),
        Rung::Nip => (HOLD_NIP, 0),
        _ => (0, 0),
    };
    if let Some(v) = world.visitors.as_mut() {
        v.mean_radius += 0.02 * (radius - v.mean_radius);
        v.counts.rolls += 1;
        match rung {
            Rung::Ignore => v.counts.ignored += 1,
            Rung::Antennate => v.counts.antennated += 1,
            Rung::Display => v.counts.displayed += 1,
            Rung::Nip => v.counts.nipped += 1,
            _ => {}
        }
        let old = v.marks.get(&ant).copied();
        v.marks.insert(
            ant,
            Mark {
                rung,
                at: frame,
                visitor,
                hold_until: frame + hold,
                away_until: frame + hold + away,
                cooldown_until: frame + COOLDOWN,
                last_touch: frame,
                rub_until: old.map_or(0, |m| m.rub_until),
            },
        );
        if rung != Rung::Ignore {
            v.note(frame, rung, ant, 0.0);
        }
    }
    match rung {
        Rung::Display => {
            // The engine's display: a quiet mark at the displayer's own cell
            // (`act`'s withdrawal), not the 240 a wound cries.
            world.deposit_pheromone(Channel::Alarm, head.0, head.1, crate::sim::contest::display_deposit());
            world.note_encounter(crate::sim::world::ENCOUNTER_DISPLAY, ant, visitor);
        }
        Rung::Nip => {
            nip(world, ant, visitor, cell);
        }
        _ => {}
    }
    rung
}

/// **The nip**: the done-branch of the game's bite (`mind::attack`), on one
/// visitor cell, with the ownership checked first (finding 2: a chain can be
/// stale until its owner reconciles). Three nips inside the heal window take
/// a cell; the head is the last.
fn nip(world: &mut World, ant: OrganismId, visitor: OrganismId, (tx, ty): (i32, i32)) {
    if world.get(tx, ty).organism_id() != visitor {
        return;
    }
    world.note_encounter(crate::sim::world::ENCOUNTER_BITE, ant, visitor);
    cry_alarm(world, tx, ty);
    let done = world.organism(visitor).is_some_and(|st| st.gnawed + NIP >= 1.0);
    if let Some(st) = world.organism_mut(visitor) {
        st.gnawed = if done { 0.0 } else { st.gnawed + NIP };
    }
    // The guest flinches: it turns to face away from the biter.
    let away = world.organism(ant).and_then(|s| s.chain.first().copied()).map(|(ax, ay)| ((tx - ax).signum() as i8, (ty - ay).signum() as i8));
    if let Some(dir) = away.filter(|d| *d != (0, 0)) {
        if let Some(h) = crate::sim::steer::pick_heading(&[0, 1, 2, 3, 4, 5, 6, 7], &DIRS, dir) {
            if let Some(st) = world.organism_mut(visitor) {
                st.heading = h;
            }
        }
    }
    if !done {
        return;
    }
    let victim_group = world.organism(visitor).map(|s| (s.species, s.colony, s.energy));
    let victim_side = kill_side(world, visitor);
    world.set(tx, ty, Cell::EMPTY);
    if let Some(v) = world.visitors.as_mut() {
        v.counts.cells_lost += 1;
    }
    if !reconcile_chain(world, visitor) {
        world.creature_stats.attack_kills += 1;
        if let (Some(vg), Some(me)) = (victim_group, world.organism(ant).map(|s| (s.species, s.colony))) {
            let detail = kill_detail(world, crate::sim::world::KILL_VERB_BITE, ant, visitor, victim_side);
            world.tally_kill_detailed((vg.0, vg.1), me, vg.2, detail);
        }
        if let Some(v) = world.visitors.as_mut() {
            v.counts.deaths += 1;
            v.note(world.frame, Rung::Nip, ant, 0.0);
        }
    }
}

/// **A held ant's options, cut to the one it must take** (`mind::tick`, after
/// the option list is built and before the mind scores it): the `Rest` while
/// it holds on the visitor, then one Walk away while it backs off. Nothing
/// else is touched, so the mind's draw, bills, memory and reschedule run as
/// for any decision. Unchanged when the ant is not held, or when the option
/// it needs is not on the list.
pub fn force_options(world: &World, ant: OrganismId, opts: &mut Vec<Opt>) {
    let Some(v) = world.visitors.as_ref() else { return };
    let Some(m) = v.marks.get(&ant) else { return };
    let frame = world.frame;
    if frame < m.hold_until {
        if let Some(rest) = opts.iter().copied().find(|o| o.verb == Verb::Rest) {
            opts.clear();
            opts.push(rest);
        }
    } else if frame < m.away_until {
        let Some(&(vx, vy)) = world.organism(m.visitor).and_then(|s| s.chain.first()) else { return };
        let far = opts
            .iter()
            .copied()
            .filter(|o| o.verb == Verb::Walk)
            .max_by_key(|o| (o.target.0 - vx).pow(2) + (o.target.1 - vy).pow(2));
        if let Some(walk) = far {
            opts.clear();
            opts.push(walk);
        }
    }
}

/// An ant beside the guest: its id, its cell, and the guest's cell it touches.
pub type Beside = (OrganismId, (i32, i32), (i32, i32));

/// The adult ants beside any cell of `guest`, nearest-first by ring order,
/// without repeats.
pub fn ants_beside(world: &World, host: SpeciesId, guest: OrganismId) -> Vec<Beside> {
    let Some(st) = world.organism(guest) else { return Vec::new() };
    let mut out: Vec<Beside> = Vec::new();
    for &(gx, gy) in &st.chain {
        for (dx, dy) in crate::sim::structural::NEIGHBOURS_8 {
            let (x, y) = (gx + dx, gy + dy);
            let o = world.get(x, y).organism_id();
            if o == 0 || o == guest || out.iter().any(|e| e.0 == o) {
                continue;
            }
            if world.organism(o).is_some_and(|s| s.species == host && s.brood.is_none()) {
                out.push((o, (x, y), (gx, gy)));
            }
        }
    }
    out
}

/// **One world tick of the visit** (`lab::tick_world`): fade, the soak, and the
/// guest's pressed verbs, then the dead ants' marks pruned. One `Option` read
/// with the ladder off.
pub fn tick(world: &mut World) {
    let Some(v) = world.visitors.as_ref() else { return };
    let input = v.input;
    let host = v.host;
    let ids: Vec<Visitor> = v.list.clone();
    for vis in ids {
        if world.organism(vis.id).is_none() {
            continue;
        }
        let Some(mine) = visitor_scent(world, vis.id) else { continue };
        let head = world.organism(vis.id).and_then(|s| s.chain.first().copied());
        // The nest odour nearest the guest, for the soak and the tint. From
        // the site's `surface`, never its `y`: `y` is the founding gesture's
        // row, which on a tall sky is far over the ground.
        let nest = head.and_then(|(hx, hy)| {
            world.nest_sites.iter().filter(|n| n.seeded).min_by_key(|n| (n.x - hx).pow(2) + (n.surface - hy).pow(2)).map(|n| (n.scent, n.x, n.surface))
        });
        let mean_r = world.visitors.as_ref().map_or(1.0, |v| v.mean_radius).max(1e-3);
        // **Fade always, soak on top**: both pull every frame, so a guest
        // soaking without rest settles where the two balance -- a quarter
        // of its own gap from the nest at the default time constants -- and
        // rubbing particular ants is what closes the rest. Soak alone never
        // makes you family.
        let k_fade = 1.0 - (-1.0 / (FPS * FADE_TAU_S)).exp();
        let mut next = [0, 1, 2].map(|i| mine[i] + k_fade * (vis.home_scent[i] - mine[i]));
        if input.soak {
            if let (Some((ns, nx, ny)), Some((hx, hy))) = (nest, head) {
                // **In the nest**: within [`SOAK_REACH`] columns of the door,
                // from the mound's top down into the galleries, or beside
                // brood. Six cells round the door's surface cell missed a
                // guest standing on a grown mound (first GIF: soak on, no
                // frame soaked in a minute beside the door).
                let by_nest = (hx - nx).abs() <= SOAK_REACH && (-SOAK_REACH..=SOAK_DEPTH).contains(&(hy - ny));
                let by_brood = brood_beside(world, vis.id).is_some();
                if by_nest || by_brood {
                    if distance(&mine, &ns) / mean_r <= SOAK_REFUSE_G {
                        let k_soak = 1.0 - (-1.0 / (FPS * SOAK_TAU_S)).exp();
                        next = [0, 1, 2].map(|i| next[i] + k_soak * (ns[i] - next[i]));
                        if let Some(v) = world.visitors.as_mut() {
                            v.counts.soak_frames += 1;
                        }
                    } else if let Some(v) = world.visitors.as_mut() {
                        v.counts.soak_refused += 1;
                    }
                }
            }
        }
        for (slot, val) in SCENT_SLOTS.iter().zip(next) {
            if let Some(s) = world.organism_mut(vis.id) {
                s.traits[*slot] = val.clamp(-1.0, 1.0);
            }
        }
        let gap_now = nest.map_or(f32::NAN, |(ns, _, _)| distance(&next, &ns) / mean_r);
        if let Some(v) = world.visitors.as_mut() {
            if let Some(x) = v.list.iter_mut().find(|x| x.id == vis.id) {
                x.gap = gap_now;
            }
        }
        if input.rub {
            rub(world, host, vis.id);
        }
        if input.beg {
            beg(world, host, vis.id);
        }
        if input.kill {
            kill_larva(world, vis.id);
        }
        if input.eat {
            eat_corpse(world, vis.id);
        }
    }
    if let Some(v) = world.visitors.as_mut() {
        v.input.rub = false;
        v.input.beg = false;
        v.input.kill = false;
        v.input.eat = false;
    }
    if world.frame.is_multiple_of(600) {
        let dead: Vec<OrganismId> = world.visitors.as_ref().map_or(Vec::new(), |v| v.marks.keys().copied().filter(|&id| world.organism(id).is_none()).collect());
        let now = world.frame;
        if let Some(v) = world.visitors.as_mut() {
            for id in dead {
                v.marks.remove(&id);
            }
            if let Some(w) = v.wary.as_mut() {
                w.prune(now);
            }
        }
    }
}

/// **Rub**: groom the first ant beside you that is not freshly rubbed. Your
/// scent moves [`RUB_STEP`] of the gap toward hers, hers a quarter as far
/// toward yours, and she rolls the ladder at half odds (being groomed calms,
/// but a hostile ant can still nip). Returns whether an ant was rubbed.
pub fn rub(world: &mut World, host: SpeciesId, guest: OrganismId) -> bool {
    let frame = world.frame;
    let ready = |w: &World, a: OrganismId| w.visitors.as_ref().is_none_or(|v| v.marks.get(&a).is_none_or(|m| frame >= m.rub_until));
    let Some((ant, at, gcell)) = ants_beside(world, host, guest).into_iter().find(|e| ready(world, e.0)) else { return false };
    let (Some(mine), Some(hers)) = (visitor_scent(world, guest), world.organism(ant).map(|s| scent_of(&s.traits))) else { return false };
    for (i, slot) in SCENT_SLOTS.iter().enumerate() {
        let d = hers[i] - mine[i];
        if let Some(s) = world.organism_mut(guest) {
            s.traits[*slot] = (mine[i] + RUB_STEP * d).clamp(-1.0, 1.0);
        }
        if let Some(s) = world.organism_mut(ant) {
            s.traits[*slot] = (hers[i] - RUB_STEP * RUB_ANT_SHARE * d).clamp(-1.0, 1.0);
        }
    }
    let rung = roll(world, ant, guest, gcell, at, 0.5, 0.0);
    if let Some(v) = world.visitors.as_mut() {
        if let Some(w) = v.wary.as_mut() {
            w.calm(frame, ant);
        }
        v.counts.rubs += 1;
        if let Some(m) = v.marks.get_mut(&ant) {
            m.rub_until = frame + RUB_COOLDOWN;
            if rung == Rung::Ignore {
                m.rung = Rung::Rubbed;
                m.at = frame;
            }
        }
    }
    true
}

/// **Beg**: ask the first ant beside you that you have not just asked. With
/// nothing to spare (bank at or under [`DONOR_FLOOR`] of its grant) it shows
/// a grey `-` and nothing is rolled; else it accepts with chance `1.2 - g'`
/// and gives [`BEG_SHARE`] of its spare bank, booked `SharedOut` on its colony
/// and `SharedIn` on yours; refused, it rolls the ladder with
/// [`BEG_PENALTY`]. Returns the joules given. With the seat's stakes on,
/// `g'` also carries the colony's [`Wariness`], and every ask adds to it.
pub fn beg(world: &mut World, host: SpeciesId, guest: OrganismId) -> f32 {
    let frame = world.frame;
    let ready = |w: &World, a: OrganismId| w.visitors.as_ref().is_none_or(|v| v.marks.get(&a).is_none_or(|m| frame >= m.rub_until));
    let Some((ant, at, gcell)) = ants_beside(world, host, guest).into_iter().find(|e| ready(world, e.0)) else { return 0.0 };
    if let Some(v) = world.visitors.as_mut() {
        v.counts.begs += 1;
    }
    let grant = world.organism(ant).and_then(|s| world.species.creature_shared(s.species)).map_or(1.0, |d| d.start_energy);
    let bank = world.organism(ant).map_or(0.0, |s| s.energy);
    let spare = bank - DONOR_FLOOR * grant;
    let set_mark = |w: &mut World, rung: Rung| {
        if let Some(v) = w.visitors.as_mut() {
            let m = v.marks.entry(ant).or_insert(Mark {
                rung,
                at: frame,
                visitor: guest,
                hold_until: 0,
                away_until: 0,
                cooldown_until: 0,
                last_touch: frame,
                rub_until: 0,
            });
            m.rung = rung;
            m.at = frame;
            m.rub_until = frame + RUB_COOLDOWN;
        }
    };
    if spare <= 0.0 {
        set_mark(world, Rung::Empty);
        if let Some(v) = world.visitors.as_mut() {
            v.counts.empty += 1;
            v.note(frame, Rung::Empty, ant, 0.0);
        }
        return 0.0;
    }
    let Some(vs) = visitor_scent(world, guest) else { return 0.0 };
    let Some((g, _)) = gap(world, ant, vs) else { return 0.0 };
    // The wariness this ask meets is read before the ask adds its own.
    let g_eff = g + pick(world, ant) + wary_term(world, ant);
    let p = (1.2 - g_eff).clamp(0.0, 1.0);
    if let Some(w) = world.visitors.as_mut().and_then(|v| v.wary.as_mut()) {
        w.add(frame, Some(ant), WARY_BEG_COLONY, WARY_BEG_ANT);
    }
    if rng::stream(world.seed, ant as u64, frame, RNG_SLOT_BEG).unit_f32() < p {
        let amount = (BEG_SHARE * spare).min(BEG_MAX);
        if let Some(s) = world.organism_mut(ant) {
            s.energy -= amount;
        }
        if let Some(s) = world.organism_mut(guest) {
            s.energy += amount;
        }
        let (give, take) = (world.colony_of(ant), world.colony_of(guest));
        world.book(give, Account::SharedOut, f64::from(amount));
        world.book(take, Account::SharedIn, f64::from(amount));
        set_mark(world, Rung::Fed);
        if let Some(v) = world.visitors.as_mut() {
            v.counts.fed += 1;
            v.counts.fed_j += f64::from(amount);
            v.note(frame, Rung::Fed, ant, amount);
        }
        amount
    } else {
        if let Some(v) = world.visitors.as_mut() {
            v.counts.refused += 1;
        }
        roll(world, ant, guest, gcell, at, 1.0, BEG_PENALTY);
        if let Some(m) = world.visitors.as_mut().and_then(|v| v.marks.get_mut(&ant)) {
            m.rub_until = frame + RUB_COOLDOWN;
        }
        0.0
    }
}

/// A larva beside any cell of `guest`: its id and cell.
pub fn brood_beside(world: &World, guest: OrganismId) -> Option<(OrganismId, (i32, i32))> {
    let st = world.organism(guest)?;
    for &(gx, gy) in &st.chain {
        for (dx, dy) in crate::sim::structural::NEIGHBOURS_8 {
            let (x, y) = (gx + dx, gy + dy);
            let o = world.get(x, y).organism_id();
            if o != 0 && o != guest && world.organism(o).and_then(|s| s.brood).is_some_and(|b| b.stage == crate::sim::organism::BroodStage::Larva) {
                return Some((o, (x, y)));
            }
        }
    }
    None
}

/// **Kill a larva beside you** (`brood::larva_eaten`): it becomes a corpse
/// holding its bank, with an alarm cried at it. You gain nothing until you
/// eat the corpse ([`eat_corpse`]): one ledger path, two visible beats.
pub fn kill_larva(world: &mut World, guest: OrganismId) -> bool {
    let Some((larva, at)) = brood_beside(world, guest) else { return false };
    if !crate::sim::brood::larva_eaten(world, larva, at) {
        return false;
    }
    let frame = world.frame;
    if let Some(v) = world.visitors.as_mut() {
        v.counts.larvae_killed += 1;
        v.note(frame, Rung::Nip, larva, 0.0);
    }
    true
}

/// **Eat a corpse cell beside you** into the crop, as a mind's pick-up takes
/// food (`mind::pick_up`): never a living cell, so the guest cannot kill a
/// worker with its mouth (review finding 4). Returns the worth taken.
pub fn eat_corpse(world: &mut World, guest: OrganismId) -> f32 {
    let Some(corpse) = world.materials.known(Known::Corpse) else { return 0.0 };
    let Some(st) = world.organism(guest) else { return 0.0 };
    let crop = st.crop;
    let chain = st.chain.clone();
    let cap = world.species.creature_shared(st.species).map_or(0.0, |d| super::organism_crop_capacity(world, guest, &d));
    for &(gx, gy) in &chain {
        for (dx, dy) in crate::sim::structural::NEIGHBOURS_8 {
            let (x, y) = (gx + dx, gy + dy);
            let c = world.get(x, y);
            if c.material != corpse || c.organism_id() != 0 {
                continue;
            }
            let worth = super::food_value(world, c);
            if worth <= 0.0 || crop.is_some_and(|k| k.material != corpse || k.worth() + k.unit > cap) {
                continue;
            }
            world.loss_context = Some(DeathCause::Eaten);
            world.set(x, y, Cell::EMPTY);
            world.loss_context = None;
            crop_take(world, guest, corpse, c.shade, worth, None);
            note_food(world, guest, (x, y), super::FOOD_SWALLOW, worth, false);
            let frame = world.frame;
            if let Some(v) = world.visitors.as_mut() {
                v.counts.corpses_eaten += 1;
                v.counts.eaten_j += f64::from(worth);
                v.note(frame, Rung::Fed, 0, worth);
            }
            return worth;
        }
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_odds_are_graded_and_never_certain() {
        let mut last = (0.0, 0.0, 0.0);
        for i in 0..=40 {
            let g = i as f32 * 0.1;
            let (p1, p2, p3) = odds(g, 1.0);
            assert!((FLOOR..=CEILING).contains(&p1), "antennate at g={g}: {p1}");
            assert!(p2 >= FLOOR.min(p1) && p2 <= p1, "display at g={g}: {p2} vs {p1}");
            assert!(p3 <= p2 && p3 < 1.0, "nip at g={g}: {p3}");
            assert!(p1 >= last.0 - 1e-6 && p2 >= last.1 - 1e-6 && p3 >= last.2 - 1e-6, "odds fell as the gap grew at g={g}");
            last = (p1, p2, p3);
        }
    }

    /// **A good disguise is never nipped by chance** (review finding 3). Watched
    /// red with the `NIP_BELOW` test taken out of `odds`.
    #[test]
    fn no_nip_below_the_line_and_some_above_it() {
        assert_eq!(odds(NIP_BELOW - 0.01, 1.0).2, 0.0);
        assert!(odds(NIP_BELOW + 0.6, 1.0).2 > 0.2);
        assert!(odds(0.0, 1.0).0 > 0.0, "the middle must exist even for a perfect disguise");
    }

    #[test]
    fn a_rubbed_ant_rolls_at_half_odds() {
        let (a, b, c) = odds(1.5, 1.0);
        let (x, y, z) = odds(1.5, 0.5);
        assert!((x - a * 0.5).abs() < 1e-6 && y <= b * 0.5 + 1e-6 && z <= c * 0.5 + 1e-6);
    }

    #[test]
    fn a_landing_scent_lies_where_it_was_asked_unless_the_cube_stops_it() {
        let nest = [0.47, -0.94, -0.92];
        let (p, d) = scent_away_from(nest, 2.0);
        assert!((d - 2.0).abs() < 0.05, "asked 2.0, got {d} at {p:?}");
        assert!(p.iter().all(|v| (-1.0..=1.0).contains(v)));
    }

    use crate::sim::creature::mind::{blank, give, MindGenome};
    use crate::sim::scheduler::ActiveKind;
    use std::sync::Arc;

    /// A rove and an ant side by side on the ground of a small box, the rove
    /// touching the ant: (world, rove, ant, ant species).
    fn rove_beside_ant() -> (World, OrganismId, OrganismId, SpeciesId) {
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
            Some(ActiveKind::Creature { organism }) => organism,
            other => panic!("no {name} placed: {other:?}"),
        };
        // The ant first, then the rove just ahead of her head, so the rove
        // is in her mouth's reach (the bite targets the head's ring).
        let ant = place(&mut w, 60, "ant");
        let body = w.organism(ant).expect("ant").chain.clone();
        let head = body[0];
        let dir = body.get(1).map_or(1, |b| (head.0 - b.0).signum()).max(-1);
        let dir = if dir == 0 { 1 } else { dir };
        let rove = (1..=4)
            .find_map(|k| match crate::sim::creature::plant_creature_seed(&mut w, head.0 + dir * k, gy - 1, "rove").map(|s| s.kind) {
                Some(ActiveKind::Creature { organism }) => Some(organism),
                _ => None,
            })
            .expect("no rove placed ahead of the ant");
        let host = w.species.id_of("ant").expect("ant ships");
        let rcells = w.organism(rove).expect("rove").chain.clone();
        assert!(
            rcells.iter().any(|&(x, y)| (x - head.0).abs() <= 1 && (y - head.1).abs() <= 1),
            "test setup: the rove {rcells:?} is not at the ant's head {head:?} (body {body:?})"
        );
        assert!(ants_beside(&w, host, rove).iter().any(|e| e.0 == ant), "test setup: the ant is not beside the rove");
        assert_ne!(w.colony_of(rove), w.colony_of(ant), "test setup: the rove is the ant's nestmate");
        (w, rove, ant, host)
    }

    /// **A registered visitor is never offered as a bite** (`options`, plan
    /// v2 finding 4): the ladder's nip is its only harm, so the harm stays
    /// graded. The positive control is the same rove unregistered: a stranger
    /// of another colony, offered as an Attack. Watched red with the
    /// `is_visitor` test taken out of `options::build_remembering`.
    #[test]
    fn a_visitor_is_never_offered_as_a_bite_but_a_stranger_is() {
        if !crate::sim::creature::options::attack_on() {
            return;
        }
        let (mut w, rove, ant, host) = rove_beside_ant();
        give(&mut w, ant, Arc::new(MindGenome::new(blank(&[Verb::Walk, Verb::Attack]))));
        let def = w.species.get(host).creature.clone().expect("creature");
        let bites_on_rove = |w: &World| {
            let mut opts = Vec::new();
            crate::sim::creature::options::build_with(w, ant, &def, false, &mut opts);
            opts.iter().filter(|o| o.verb == Verb::Attack && w.get(o.target.0, o.target.1).organism_id() == rove).count()
        };
        if bites_on_rove(&w) == 0 {
            let mut opts = Vec::new();
            crate::sim::creature::options::build_with(&w, ant, &def, false, &mut opts);
            let list: Vec<_> = opts.iter().map(|o| (o.verb, o.target, w.get(o.target.0, o.target.1).organism_id())).collect();
            panic!(
                "an unregistered stranger beside the ant was not offered as a bite: the test cannot see the gate. ant {:?}, rove {:?}, options {list:?}",
                w.organism(ant).map(|s| s.chain.clone()),
                w.organism(rove).map(|s| s.chain.clone())
            );
        }
        let home = scent_of(&w.organism(rove).expect("rove").traits);
        register(&mut w, host, rove, home);
        assert_eq!(bites_on_rove(&w), 0, "a registered visitor was offered as a bite");
    }

    /// **A beg moves joules from one colony's books to the other's, the same
    /// on both sides**, capped at [`BEG_MAX`]; an ant with nothing to spare
    /// gives nothing and shows `Empty`. Smelling of the ant's own nest, the
    /// rove is all but sure to be fed, so a few tries must feed it.
    #[test]
    fn a_beg_is_booked_out_of_one_colony_and_into_the_other() {
        let (mut w, rove, ant, host) = rove_beside_ant();
        let home = scent_of(&w.organism(ant).expect("ant").traits);
        register(&mut w, host, rove, home);
        let grant = w.species.creature_shared(host).expect("creature").start_energy;
        let (give_c, take_c) = (w.colony_of(ant), w.colony_of(rove));
        let mut fed = 0.0;
        for _ in 0..6 {
            w.organism_mut(ant).expect("ant").energy = 2.0 * grant;
            let (e_ant, e_rove) = (w.organism(ant).expect("ant").energy, w.organism(rove).expect("rove").energy);
            let (out0, in0) = (w.colony_books(give_c).get(Account::SharedOut), w.colony_books(take_c).get(Account::SharedIn));
            let got = beg(&mut w, host, rove);
            let (out1, in1) = (w.colony_books(give_c).get(Account::SharedOut), w.colony_books(take_c).get(Account::SharedIn));
            assert!((out1 - out0 - f64::from(got)).abs() < 1e-3 && (in1 - in0 - f64::from(got)).abs() < 1e-3, "booked out {} and in {} for {got}", out1 - out0, in1 - in0);
            assert!((e_ant - w.organism(ant).expect("ant").energy - got).abs() < 1e-3 && (w.organism(rove).expect("rove").energy - e_rove - got).abs() < 1e-3);
            assert!(got <= BEG_MAX);
            fed += got;
            w.frame += RUB_COOLDOWN;
        }
        assert!(fed > 0.0, "an ant smelling its own nest on the beggar never fed it in six begs");
        w.organism_mut(ant).expect("ant").energy = 0.5 * grant;
        let empty = w.visitors.as_ref().expect("on").counts.empty;
        assert_eq!(beg(&mut w, host, rove), 0.0, "an ant under the floor gave");
        assert_eq!(w.visitors.as_ref().expect("on").counts.empty, empty + 1);
    }

    /// **Begging the same ant again and again pays less with the stakes on,
    /// and a rub buys some of it back** ([`Wariness`]). The rove smells of
    /// the ant's own nest and the ant is kept rich, so without wariness every
    /// beg is all but sure to pay: that arm is the positive control, and the
    /// wary arm must be fed less, refused more, and nipped at least once by
    /// a refusal's roll. Watched red with the `wary_term` taken out of
    /// `beg`'s acceptance.
    #[test]
    fn a_wary_colony_feeds_a_repeat_beggar_less_and_a_rub_calms_it() {
        let run = |wary: bool| {
            let (mut w, rove, ant, host) = rove_beside_ant();
            let home = scent_of(&w.organism(ant).expect("ant").traits);
            register(&mut w, host, rove, home);
            if wary {
                w.visitors.as_mut().expect("on").wary = Some(Wariness::default());
            }
            let grant = w.species.creature_shared(host).expect("creature").start_energy;
            for _ in 0..12 {
                w.organism_mut(ant).expect("ant").energy = 3.0 * grant;
                if w.organism(rove).is_none() {
                    break;
                }
                beg(&mut w, host, rove);
                w.frame += RUB_COOLDOWN;
            }
            let c = w.visitors.as_ref().expect("on").counts;
            (c.fed, c.refused, c.nipped + c.displayed + c.antennated, w, rove, ant, host)
        };
        let (fed_off, refused_off, ..) = run(false);
        let (fed_on, refused_on, rolled_on, mut w, rove, ant, host) = run(true);
        assert!(fed_off >= 11, "test setup: a rove smelling of the nest was refused without wariness ({fed_off} fed, {refused_off} refused)");
        assert!(fed_on + 4 <= fed_off && refused_on >= 4, "wariness did not cut a repeat beggar's gifts: {fed_on} fed / {refused_on} refused, against {fed_off} without");
        assert!(rolled_on > 0, "a wary colony's refusals rolled nothing");
        let now = w.frame;
        let before = w.visitors.as_ref().and_then(|v| v.wary.as_ref()).map(|x| x.term(ant, now)).expect("on");
        if w.organism(rove).is_some() {
            w.frame += RUB_COOLDOWN;
            let now = w.frame;
            let faded_only = w.visitors.as_ref().and_then(|v| v.wary.as_ref()).map(|x| x.term(ant, now)).expect("on");
            assert!(rub(&mut w, host, rove), "no ant rubbed");
            let after = w.visitors.as_ref().and_then(|v| v.wary.as_ref()).map(|x| x.term(ant, now)).expect("on");
            assert!(after < faded_only - 0.05 && faded_only <= before, "a rub did not calm: {before} -> faded {faded_only} -> rubbed {after}");
        }
    }

    /// **A rub moves the guest's scent four times as far as the ant's**
    /// ([`RUB_STEP`], [`RUB_ANT_SHARE`]), each toward the other.
    #[test]
    fn a_rub_moves_the_guest_four_times_as_far_as_the_ant() {
        let (mut w, rove, ant, host) = rove_beside_ant();
        let hers0 = scent_of(&w.organism(ant).expect("ant").traits);
        let (home, _) = scent_away_from(hers0, 0.8);
        register(&mut w, host, rove, home);
        let mine0 = scent_of(&w.organism(rove).expect("rove").traits);
        assert!(rub(&mut w, host, rove), "no ant rubbed");
        let mine1 = scent_of(&w.organism(rove).expect("rove").traits);
        let hers1 = scent_of(&w.organism(ant).expect("ant").traits);
        let (dm, dh) = (distance(&mine0, &mine1), distance(&hers0, &hers1));
        let d0 = distance(&mine0, &hers0);
        assert!((dm - RUB_STEP * d0).abs() < 1e-3, "the guest moved {dm} of {d0}");
        assert!((dh - RUB_STEP * RUB_ANT_SHARE * d0).abs() < 1e-3, "the ant moved {dh} of {d0}");
        assert!(distance(&mine1, &hers1) < d0, "the rub pushed them apart");
        assert!(!rub(&mut w, host, rove), "a freshly rubbed ant was rubbed again inside the cooldown");
    }

    /// The trained colony of `steady_income`, landed at frame 200 and run to
    /// 400, with a rove placed at the nest door smelling `dist` from the
    /// nest's odour. (lab, rove, host, that scent)
    fn trained_colony_with_a_rove(dist: f32) -> (crate::lab::Lab, OrganismId, SpeciesId, [f32; 3]) {
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
        let host = lab.world.species.id_of("ant").expect("ant ships");
        let nest = lab.world.nest_sites.first().cloned().expect("the colony landed");
        let mut rove = None;
        'search: for off in [4, 2, 6, -4, -2, -6, 8, -8] {
            for lift in 0..24 {
                if let Some(s) = crate::sim::creature::plant_creature_seed_in(&mut lab.world, nest.x + off, nest.surface - 2 - lift, "rove", None) {
                    if let ActiveKind::Creature { organism } = s.kind {
                        lab.world.schedule_active_site(s);
                        rove = Some(organism);
                        break 'search;
                    }
                }
            }
        }
        let rove = rove.expect("the rove fits at the door");
        let (scent, _) = scent_away_from(nest.scent, dist);
        for (slot, v) in SCENT_SLOTS.iter().zip(scent) {
            lab.world.set_organism_trait(rove, *slot, v);
        }
        (lab, rove, host, scent)
    }

    fn cell_hash(w: &World) -> (u64, usize) {
        let b = w.bounds().expect("the lab sets bounds");
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let c = w.get(x, y);
                for v in [u64::from(c.material.0), u64::from(c.shade), u64::from(c.aux()), u64::from(c.organism_id())] {
                    h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
        }
        (h, w.live_organism_ids().len())
    }

    /// **Off is off, in CI**: a trained colony with a rove at its door runs
    /// cell for cell the same with no ladder, an empty one, and one naming a
    /// body that does not exist. It catches a hook that acts without asking
    /// whether the body is a registered, living visitor. The positive control
    /// registers the rove itself and must differ, or the hash is blind.
    #[test]
    fn a_ladder_with_no_living_visitor_changes_nothing() {
        fn run(arm: u8) -> ((u64, usize), u64) {
            let (mut lab, rove, host, scent) = trained_colony_with_a_rove(2.0);
            match arm {
                0 => {}
                1 => lab.world.visitors = Some(Visitors::new(host)),
                2 => {
                    let mut v = Visitors::new(host);
                    v.list.push(Visitor { id: OrganismId::MAX, home_scent: scent, gap: f32::NAN });
                    lab.world.visitors = Some(v);
                }
                _ => register(&mut lab.world, host, rove, scent),
            }
            for _ in 0..2_600 {
                lab.tick_for_harness();
            }
            let rolls = lab.world.visitors.as_ref().map_or(0, |v| v.counts.rolls);
            (cell_hash(&lab.world), rolls)
        }
        let off = run(0);
        assert_eq!(run(1).0, off.0, "an empty ladder moved the world");
        assert_eq!(run(2).0, off.0, "a ladder naming no living body moved the world");
        let on = run(3);
        assert!(on.1 > 0, "the registered rove at the door was never met: the positive control did not fire");
        assert_ne!(on.0, off.0, "a met visitor changed nothing: the hash cannot see the ladder");
    }

    /// **Soak refuses a guest too far off** ([`SOAK_REFUSE_G`]) and takes one
    /// close enough, at the same door. Watched red with the refusal taken out.
    #[test]
    fn a_guest_too_far_off_cannot_soak_and_one_close_enough_can() {
        for (dist, takes) in [(3.0, false), (1.0, true)] {
            let (mut lab, rove, host, scent) = trained_colony_with_a_rove(dist);
            register(&mut lab.world, host, rove, scent);
            for _ in 0..60 {
                lab.world.visitors.as_mut().expect("on").input.soak = true;
                tick(&mut lab.world);
                lab.world.frame += 1;
            }
            let c = lab.world.visitors.as_ref().expect("on").counts;
            assert_eq!((c.soak_frames > 0, c.soak_refused > 0), (takes, !takes), "dist {dist}: {c:?}");
        }
    }
}
