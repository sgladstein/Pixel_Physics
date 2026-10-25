//! **One ant the player drives** — the body of the play-as-an-ant game
//! (`Reports/play-as-ant-slice0-2026-10-11.md`; the design guide and the
//! implementation design are Claude Docs linked from there).
//!
//! The steered body is an ordinary ant. Its genome still runs every decision;
//! what changes is that five places in its tick take the keyboard's answer
//! instead of the brain's or the colony's (`creature.rs`, each hook named
//! `steer::` at its site):
//!
//! 1. **the outputs**, replaced after the look-ahead merge, on the serial
//!    thread, so the parallel and serial drivers stay identical;
//! 2. **`forage_pace`**, which would make an empty forager walk whatever its
//!    `Move` said -- skipped, so she stands unless steered;
//! 3. **`food_trail_lay`**, which lays trail B only for a trip's load --
//!    bypassed, so T lays wherever she walks;
//! 4. **`act`'s rewrites** -- the lean and forager no-dig gates, the
//!    forage drive's feed scaling, `needs_first_act`, the crop holds, the
//!    stranger hold, the spoil holds and lift (Q puts a pellet down beside
//!    her or not at all), and the dig's turns, side-cuts and refusals (G
//!    cuts the cell she faces, if the jaw can take it) -- skipped: the
//!    pressed verb is the verb. Sealed in with a pellet, G packs it behind
//!    her as she cuts on (`pack_behind`, the colony's own escape);
//! 5. **the step**: the usable heading nearest the steer direction, among
//!    headings within 90 degrees of it, ahead of `chooser_step`'s fallbacks
//!    (`step_chain`, `fills_before_walking`). None such, and she stands,
//!    turned to face the steer -- so G digs where she is steered.
//!
//! Plus three things that hold whether or not a keyboard is attached
//! ([`Steered::input`] `None` is "her genome drives her", the state above 4x
//! and the state every slice-0 measurement runs her in):
//!
//! - **no old age** (owner, 2026-10-11: *"My player shouldn't die of old
//!   age"*): the hazard roll is skipped for the steered body. Hunger, a
//!   beetle and a fight still kill;
//! - **no caste**: the nest-worker caste is chosen by id, one ant in four
//!   (`Storeroom::caste`), so [`mark`] clears her `nest_bound_until`;
//! - **the queen rule for her colony only**: the two laying checks pass
//!   `BreedingRegime::Queen` for `colony` and the process regime for every
//!   other colony.
//!
//! **When she dies** (slice 0): `World::steered` stays set with her dead
//! `(id, born_frame)`, so every per-body hook above is false, but her
//! colony **keeps the queen rule** for the rest of the run
//! ([`Steered::colony`] is all `regime_of_colony` reads). The slice-0
//! founding numbers were measured that way. Slice 2's hand-off in
//! `creature_dies` must decide which rule a colony with no player body
//! alive runs.
//!
//! **Off is off.** With `World::steered` at `None` every hook is one
//! branch not taken and the build is the build without this module. Checked
//! by the slice-0 comparison against main in the report: every `deeptrace`
//! output byte-identical, at one thread and at four.
//!
//! **A body with a mind** (the trained ant, the spider, the centipede)
//! returns into `mind::tick` before any of the five hooks; there the keyboard
//! picks the mind's option instead ([`ForcedPick`], [`take_forced`],
//! [`choose`]), and [`force`] lets a module lead any mind body the same way.
//!
//! **Not `World::player`** -- that is the gnome (`sim::player`).

use crate::sim::cell::OrganismId;
use crate::sim::world::World;

/// What the keyboard says this decision. Every field is a level: held or
/// not. A press that must not be lost between decisions (the lab reads keys
/// once a displayed frame and an ant decides every six frames) is latched by
/// the caller with [`Steered::press`], and [`take_input`] clears it once
/// used.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SteerInput {
    /// Screen direction, `x` right and `y` down, each -1, 0 or 1. `None` is
    /// no steer key held: she stands.
    pub dir: Option<(i8, i8)>,
    /// E: take the food in reach.
    pub eat: bool,
    /// Q: put down what the crop or the jaws hold.
    pub drop: bool,
    /// F: offer the crop to a nestmate.
    pub share: bool,
    /// G: dig.
    pub dig: bool,
    /// B: bite the nearest foe.
    pub bite: bool,
    /// L: lay an egg, if she can afford one.
    pub lay: bool,
    /// T held: lay the food trail (channel B).
    pub trail: bool,
}

impl SteerInput {
    /// Both inputs together: the latched presses ride on the held levels.
    fn or(self, other: SteerInput) -> SteerInput {
        SteerInput {
            dir: self.dir.or(other.dir),
            eat: self.eat || other.eat,
            drop: self.drop || other.drop,
            share: self.share || other.share,
            dig: self.dig || other.dig,
            bite: self.bite || other.bite,
            lay: self.lay || other.lay,
            trail: self.trail || other.trail,
        }
    }
}

/// The one body the player drives, and the rules that come with it.
#[derive(Clone, Debug)]
pub struct Steered {
    pub id: OrganismId,
    /// With `id`, the body's identity: a dead ant's id is never matched to
    /// another body by accident.
    pub born_frame: u64,
    /// Her colony, which runs the queen rule.
    pub colony: u32,
    /// `None`: her genome drives her (fast-forward above the steering cap,
    /// and every slice-0 measurement). `Some`: the keyboard does.
    pub input: Option<SteerInput>,
    /// Presses waiting for her next decision.
    pub latched: SteerInput,
    /// The input her last decision used, held levels and latch together
    /// ([`take_input`]); `act` reads it after the latch is spent.
    pub used: SteerInput,
    /// **Every input as it was used**, `(frame, input)`, off (`None`) unless
    /// a caller turns it on. Pushed only when the input differs from the
    /// last row, so a held key is one row. The frame is the frame the input
    /// was *used* on, because the lab's tick loop is capped by wall-clock
    /// time and the frame a key *landed* on depends on the machine. **Meant
    /// for replay; no replay has been shown yet** (slice 1 must show one
    /// exact replay before anything relies on it).
    pub log: Option<Vec<(u64, SteerInput)>>,
}

impl Steered {
    /// Latch a press so it is used at her next decision however many frames
    /// pass before it.
    pub fn press(&mut self, input: SteerInput) {
        self.latched = self.latched.or(input);
    }
}

/// **Make `id` the steered body.** Clears the caste (she is a forager
/// whatever her id), and records her colony for the queen rule. Returns
/// `false`, changing nothing, if `id` is not a live organism.
pub fn mark(world: &mut World, id: OrganismId, input: Option<SteerInput>) -> bool {
    let Some(state) = world.organism_mut(id) else {
        return false;
    };
    state.nest_bound_until = 0;
    let (born_frame, colony) = (state.born_frame, state.colony);
    world.steered = Some(Steered {
        id,
        born_frame,
        colony,
        input,
        latched: SteerInput::default(),
        used: SteerInput::default(),
        log: None,
    });
    true
}

/// **Give the steered body `joules` more in her bank**, booked as
/// `Granted` so the colony's energy ledger still closes. A lone founder is
/// placed with exactly the species' `start_energy` (200 J for the ant) and
/// the lay bar is 1,100 J of body energy, so a raised reserve is how a
/// queen of one reaches her first egg without a long blind forage. Returns
/// the amount granted (0 with nobody steered).
pub fn grant(world: &mut World, joules: f32) -> f32 {
    let Some((id, colony)) = world.steered.as_ref().map(|s| (s.id, s.colony)) else {
        return 0.0;
    };
    if joules <= 0.0 || !is_steered(world, id) {
        return 0.0;
    }
    let Some(state) = world.organism_mut(id) else {
        return 0.0;
    };
    state.energy += joules;
    world.book(colony, crate::sim::world::Account::Granted, f64::from(joules));
    joules
}

/// **Is `organism` the steered body?** `false` with nobody steered, which is
/// the whole of the off path: one `Option` read.
#[inline]
pub fn is_steered(world: &World, organism: OrganismId) -> bool {
    match world.steered.as_ref() {
        None => false,
        Some(s) => s.id == organism && world.organism(organism).is_some_and(|st| st.born_frame == s.born_frame),
    }
}

/// **The keyboard's input for `organism` this decision**, if it is the
/// steered body and a keyboard drives it. Read-only; [`take_input`] is the
/// one that spends the latch.
#[inline]
pub fn input_of(world: &World, organism: OrganismId) -> Option<SteerInput> {
    let s = world.steered.as_ref()?;
    if s.id != organism {
        return None;
    }
    let held = s.input?;
    if !is_steered(world, organism) {
        return None;
    }
    Some(held.or(s.latched))
}

/// **Spend this decision's input**: the held levels plus the latch, which is
/// cleared, and the row logged. Called once per decision of the steered body,
/// at the output hook.
pub fn take_input(world: &mut World, organism: OrganismId) -> Option<SteerInput> {
    let used = input_of(world, organism)?;
    let frame = world.frame;
    let s = world.steered.as_mut().expect("input_of read it");
    s.latched = SteerInput::default();
    s.used = used;
    if let Some(log) = s.log.as_mut() {
        if log.last().is_none_or(|&(_, last)| last != used) {
            log.push((frame, used));
        }
    }
    Some(used)
}

/// **The usable heading the steer direction picks**: of `usable` (indices
/// into `DIRS`), the one whose direction is nearest `dir`, among those within
/// 90 degrees of it (a strictly positive dot product). `None` when none is,
/// which is the flat-ground case for straight up: the two usable headings
/// are level and neither points up, so she stands rather than pick between
/// forward and back. Ties go to the first in `DIRS` order, so the pick is a
/// function of the input alone.
pub fn pick_heading(usable: &[u8], dirs: &[(i32, i32); 8], dir: (i8, i8)) -> Option<u8> {
    let (sx, sy) = (f32::from(dir.0), f32::from(dir.1));
    let len = (sx * sx + sy * sy).sqrt();
    if len == 0.0 {
        return None;
    }
    let mut best: Option<(u8, f32)> = None;
    for &d in usable {
        let (dx, dy) = dirs[d as usize];
        let (dx, dy) = (dx as f32, dy as f32);
        let cos = (dx * sx + dy * sy) / (len * (dx * dx + dy * dy).sqrt());
        if cos > 1e-6 && best.is_none_or(|(_, c)| cos > c + 1e-6) {
            best = Some((d, cos));
        }
    }
    best.map(|(d, _)| d)
}

/// **A decision forced on a body that thinks with a mind** (`mind::tick`),
/// the mind's counterpart of the five hooks above, which all live in
/// `creature_tick`'s hand-built path that a mind returns before reaching
/// (enemy seat, 2026-10-25: a trained lab ant, the spider and the
/// centipede are all minds, so `mark` alone steered none of them).
///
/// The mind still decides first, so its memory and its hidden state keep
/// time; then the pick is replaced by the option this names, from the list
/// the mind was offered -- **only a legal option is ever forced**, so the
/// body can do nothing a mind of its kind could not. Read by [`choose`]:
///
/// - `verb` set: the option of that verb whose target lies nearest `dir`
///   (any target of that verb if `dir` is `None`). None listed and the
///   rule falls through to `dir`. `PickUp` falls back to `Eat`.
/// - `dir` alone: the Walk whose step is nearest `dir` within 90 degrees,
///   else a chain body's TurnAround if its tail lies that way, else (a
///   level hold only) the walk straight up: held into a wall, she climbs.
/// - Neither, or nothing usable that way: Rest. **She stands**, as a
///   steered hand-built ant does with no key held.
///
/// Two writers. The steered body ([`World::steered`]) has its
/// [`SteerInput`] converted ([`ForcedPick::from`]); any other body may be
/// led by a module through [`force`] (the scout seat's tandem follower,
/// the first such). The steered body's keyboard wins over a map entry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ForcedPick {
    pub dir: Option<(i8, i8)>,
    pub verb: Option<crate::sim::creature::options::Verb>,
    /// Spent by the one decision that reads it ([`force`] entries only;
    /// a steered body's input is spent by [`take_input`] as ever).
    pub once: bool,
}

impl From<SteerInput> for ForcedPick {
    /// The keyboard's verbs onto a mind's: B bites, G digs, E takes food,
    /// Q puts down, F feeds. L and T have no mind verb (laying is the
    /// body's, at its bar; a mind lays scent by its own emits) and are not
    /// read here.
    fn from(input: SteerInput) -> ForcedPick {
        use crate::sim::creature::options::Verb;
        let verb = if input.bite {
            Some(Verb::Attack)
        } else if input.dig {
            Some(Verb::Dig)
        } else if input.eat {
            Some(Verb::PickUp)
        } else if input.drop {
            Some(Verb::Drop)
        } else if input.share {
            Some(Verb::Feed)
        } else {
            None
        };
        ForcedPick { dir: input.dir, verb, once: false }
    }
}

/// One body's entry in [`World::mind_force`]: the pick, and the body it
/// was written for, so an id the engine hands to a newborn later is never
/// led by an entry meant for the dead.
#[derive(Clone, Copy, Debug)]
pub struct Forced {
    pub born_frame: u64,
    pub pick: ForcedPick,
}

/// **Lead `id`'s next decisions** (or only the next, with `once`).
/// Returns `false`, changing nothing, if `id` is not a live organism.
/// Takes effect only on a body with a mind; [`unforce`] hands it back.
pub fn force(world: &mut World, id: OrganismId, pick: ForcedPick) -> bool {
    let Some(born_frame) = world.organism(id).map(|s| s.born_frame) else {
        return false;
    };
    world.mind_force.insert(id, Forced { born_frame, pick });
    true
}

/// Hand `id` back to its own mind.
pub fn unforce(world: &mut World, id: OrganismId) {
    world.mind_force.remove(&id);
}

/// **The pick forced on `organism` this decision**, if any, spending what
/// is spent: the steered body's input (and its latch), or a `once` entry.
/// With nobody steered and the map empty, two reads and `None`, which is
/// the whole of the off path.
pub fn take_forced(world: &mut World, organism: OrganismId) -> Option<ForcedPick> {
    if let Some(input) = take_input(world, organism) {
        return Some(ForcedPick::from(input));
    }
    if world.mind_force.is_empty() {
        return None;
    }
    let entry = *world.mind_force.get(&organism)?;
    let alive = world.organism(organism).is_some_and(|s| s.born_frame == entry.born_frame);
    if !alive || entry.pick.once {
        world.mind_force.remove(&organism);
    }
    alive.then_some(entry.pick)
}

/// **Which option a [`ForcedPick`] names**, as an index into `opts`, the
/// list the mind was offered; `head` is the body's head cell. `None` only
/// when the list holds no option the rule can use, not even Rest (every
/// mind's list holds Rest, so in practice never). A pure function of its
/// inputs. Only verbs whose act has landed (`Verb::acts`) are forced.
pub fn choose(opts: &[crate::sim::creature::options::Opt], head: (i32, i32), pick: ForcedPick) -> Option<usize> {
    use crate::sim::creature::options::Verb;
    let toward = |i: usize, dir: (i8, i8)| -> f32 {
        let (dx, dy) = ((opts[i].target.0 - head.0) as f32, (opts[i].target.1 - head.1) as f32);
        let (sx, sy) = (f32::from(dir.0), f32::from(dir.1));
        let len = (dx * dx + dy * dy).sqrt() * (sx * sx + sy * sy).sqrt();
        if len == 0.0 {
            0.0
        } else {
            (dx * sx + dy * sy) / len
        }
    };
    // The option of `verb` nearest `dir`; ties to the first listed, so the
    // pick is a function of the list alone.
    let of_verb = |verb: Verb, dir: Option<(i8, i8)>, need_ahead: bool| -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for i in (0..opts.len()).filter(|&i| opts[i].verb == verb && verb.acts()) {
            let cos = dir.map_or(0.0, |d| toward(i, d));
            if need_ahead && cos <= 1e-6 {
                continue;
            }
            if best.is_none_or(|(_, c)| cos > c + 1e-6) {
                best = Some((i, cos));
            }
        }
        best.map(|(i, _)| i)
    };
    if let Some(verb) = pick.verb {
        let got = of_verb(verb, pick.dir, false).or_else(|| (verb == Verb::PickUp).then(|| of_verb(Verb::Eat, pick.dir, false)).flatten());
        if got.is_some() {
            return got;
        }
    }
    if let Some(dir) = pick.dir {
        if let Some(i) = of_verb(Verb::Walk, Some(dir), true).or_else(|| of_verb(Verb::TurnAround, Some(dir), true)) {
            return Some(i);
        }
        // **Held into a wall, she climbs** (scout seat's ask): a level hold with
        // nothing within 90 degrees takes the walk straight up, before standing.
        // Only up, and only for a level hold: held down on flat ground she must
        // stand (so G digs where she is steered), not wander sideways.
        let square = (dir.1 == 0)
            .then(|| (0..opts.len()).find(|&i| opts[i].verb == Verb::Walk && opts[i].target.0 == head.0 && opts[i].target.1 < head.1))
            .flatten();
        if square.is_some() {
            return square;
        }
    }
    of_verb(Verb::Rest, None, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::sim::creature::DIRS;

    #[test]
    fn up_on_flat_ground_stands_rather_than_pick_between_forward_and_back() {
        // Level ground: only east (0) and west (4) are usable.
        assert_eq!(pick_heading(&[0, 4], &DIRS, (0, -1)), None);
        assert_eq!(pick_heading(&[0, 4], &DIRS, (1, 0)), Some(0));
        assert_eq!(pick_heading(&[0, 4], &DIRS, (-1, 0)), Some(4));
    }

    #[test]
    fn a_slope_is_climbed_by_the_nearest_heading_within_ninety_degrees() {
        // Going east into a rise: north-east (1) and east are usable.
        assert_eq!(pick_heading(&[0, 1, 4], &DIRS, (1, -1)), Some(1));
        assert_eq!(
            pick_heading(&[1, 4], &DIRS, (1, 0)),
            Some(1),
            "45 degrees off is still within 90"
        );
        assert_eq!(pick_heading(&[4], &DIRS, (1, 0)), None, "straight back is not a pick");
    }

    /// **Off is off, in CI**: a lab colony with `World::steered` pointing at
    /// a body that does not exist (a held direction and G, and a colony no
    /// ant is in) runs cell for cell as the same colony with nobody steered.
    /// It catches a future hook that reads `steered` without asking whether
    /// the body is hers. The positive control steers one live ant and must
    /// differ, or the hash is blind. Watched red with `input_of`'s body
    /// checks taken out. **Blind to the old-age gate**: 3,000 frames is too
    /// short for any ant to age out, so `is_steered` answering true for
    /// everyone passes here; the slice-0 `deeptrace` comparison covers it.
    #[test]
    fn a_steer_on_no_living_body_changes_nothing() {
        use crate::lab::scenario::{Placement, Scenario};
        fn run(steer: Option<Option<OrganismId>>) -> (u64, usize) {
            let mut sc = Scenario::load("steady_income").expect("steady_income ships");
            for e in &mut sc.timeline {
                if matches!(e.what, Placement::Colony { .. } | Placement::Heap { .. }) {
                    e.at = 200;
                }
            }
            let mut lab = crate::lab::Lab::new(sc.bed.clone());
            lab.load_scenario(sc);
            for _ in 0..400 {
                lab.tick_for_harness();
            }
            let held = SteerInput {
                dir: Some((1, 0)),
                dig: true,
                ..Default::default()
            };
            match steer {
                None => {}
                Some(None) => {
                    lab.world.steered = Some(Steered {
                        id: OrganismId::MAX,
                        born_frame: 0,
                        colony: u32::MAX,
                        input: Some(held),
                        latched: held,
                        used: held,
                        log: None,
                    })
                }
                Some(Some(_)) => {
                    let mut ids = lab.world.live_organism_ids();
                    ids.sort();
                    let ant = lab.world.species.id_of("ant").expect("ant ships");
                    let id = *ids.iter().find(|&&i| lab.world.organism(i).is_some_and(|s| s.species == ant)).expect("the colony landed");
                    assert!(mark(&mut lab.world, id, Some(held)));
                }
            }
            for _ in 0..2_600 {
                lab.tick_for_harness();
            }
            let w = &lab.world;
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
        let off = run(None);
        assert_eq!(run(Some(None)), off, "a steer on no living body moved the world");
        assert_ne!(run(Some(Some(0))), off, "steering a live ant changed nothing: the hash cannot see a hook");
    }

    /// **The forced pick names a legal option, by direction, verb first.**
    /// A synthetic list round a head at the origin: walks east and north-east,
    /// a turn-around to the west (a chain's tail), a dig to the south, Rest.
    #[test]
    fn a_forced_pick_takes_the_nearest_listed_option_and_stands_otherwise() {
        use crate::sim::creature::options::{Opt, Verb, FEATURES};
        let o = |verb, target| Opt { verb, target, feats: [0.0; FEATURES] };
        let opts = [o(Verb::Walk, (1, 0)), o(Verb::Walk, (1, -1)), o(Verb::TurnAround, (-4, 0)), o(Verb::Dig, (0, 1)), o(Verb::Rest, (0, 0))];
        let f = |dir, verb| ForcedPick { dir, verb, once: false };
        assert_eq!(choose(&opts, (0, 0), f(Some((1, 0)), None)), Some(0));
        assert_eq!(choose(&opts, (0, 0), f(Some((1, -1)), None)), Some(1));
        assert_eq!(choose(&opts, (0, 0), f(Some((0, -1)), None)), Some(1), "north: the north-east walk is within 90 degrees");
        assert_eq!(choose(&opts, (0, 0), f(Some((-1, 0)), None)), Some(2), "west: no walk that way, the chain turns round");
        assert_eq!(choose(&opts, (0, 0), f(Some((0, 1)), None)), Some(4), "south: nothing walks that way, she stands (no sideways wander)");
        let face = [o(Verb::Walk, (0, 1)), o(Verb::Walk, (0, -1)), o(Verb::Rest, (0, 0))];
        assert_eq!(choose(&face, (0, 0), f(Some((1, 0)), None)), Some(1), "held right into a wall, she climbs it");
        assert_eq!(choose(&face, (0, 0), f(Some((0, -1)), None)), Some(1), "held up, the walk up");
        assert_eq!(choose(&opts, (0, 0), f(None, None)), Some(4), "no key: she stands");
        assert_eq!(choose(&opts, (0, 0), f(Some((1, 0)), Some(Verb::Dig))), Some(3), "a pressed verb beats the direction");
        assert_eq!(choose(&opts, (0, 0), f(Some((1, 0)), Some(Verb::Feed))), Some(0), "a verb not on the list falls back to the walk");
        let eat = [o(Verb::Eat, (0, 0)), o(Verb::Rest, (0, 0))];
        assert_eq!(choose(&eat, (0, 0), f(None, Some(Verb::PickUp))), Some(0), "E with nothing to pick up eats from the crop");
    }

    /// The lab's trained colony, one ant in it marked or led, and a hash of
    /// the box after `frames` more ticks. The shared rig of the mind tests.
    fn trained_lab_run(lead: impl Fn(&mut crate::lab::Lab, OrganismId), frames: u32) -> (u64, (i32, i32), (i32, i32)) {
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
        let mut ids = lab.world.live_organism_ids();
        ids.sort();
        let ant = lab.world.species.id_of("ant").expect("ant ships");
        let id = *ids
            .iter()
            .find(|&&i| lab.world.organism(i).is_some_and(|s| s.species == ant) && crate::sim::creature::mind::has(&lab.world, i))
            .expect("the colony landed, thinking with minds");
        let start = lab.world.organism(id).and_then(|s| s.chain.first().copied()).expect("alive");
        lead(&mut lab, id);
        // The last cell her head stood in: led into the box's edge she may
        // drown or be eaten, and a dead body's position is its last one.
        let mut end = start;
        for _ in 0..frames {
            lab.tick_for_harness();
            if let Some(at) = lab.world.organism(id).and_then(|s| s.chain.first().copied()) {
                end = at;
            }
        }
        let w = &lab.world;
        let b = w.bounds().expect("the lab sets bounds");
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let c = w.get(x, y);
                for v in [u64::from(c.material.0), u64::from(c.aux()), u64::from(c.organism_id())] {
                    h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
        }
        (h, start, end)
    }

    /// **A body with a mind obeys the keyboard** (shared/steer-mind.md): a
    /// trained lab ant held to the east walks east, and off is off -- an
    /// entry for a body that is not alive, and an empty map, leave the box
    /// cell for cell as nobody steered. Watched red with `take_forced`'s call
    /// in `mind::tick` taken out (the steered ant then wanders like the rest
    /// and the hash equals the control).
    #[test]
    fn a_trained_ant_is_steered_through_its_mind_and_off_is_off() {
        let east = SteerInput { dir: Some((1, 0)), ..Default::default() };
        let (off, _, _) = trained_lab_run(|_, _| {}, 300);
        let (dead, _, _) = trained_lab_run(
            |lab, _| {
                lab.world.mind_force.insert(OrganismId::MAX, Forced { born_frame: 0, pick: ForcedPick::from(east) });
            },
            300,
        );
        assert_eq!(dead, off, "an entry for no living body moved the world");
        let (on, start, end) = trained_lab_run(
            |lab, id| {
                assert!(mark(&mut lab.world, id, Some(east)));
            },
            300,
        );
        assert_ne!(on, off, "steering a trained ant changed nothing: the mind ignores the keyboard");
        assert!(end.0 >= start.0 + 10, "held east for 300 frames (50 decisions), the ant went from {start:?} to {end:?}");
    }

    /// **A module can lead a mind ant without the keyboard** (`force`).
    /// Led east through the map, the ant goes east; this seed's ant left to
    /// its mind walks west (the control, asserted), so the lead is what moved
    /// her. A first version led her west and stayed green with the hook taken
    /// out: she was going that way anyway.
    #[test]
    fn a_forced_entry_leads_a_mind_ant() {
        let east = ForcedPick { dir: Some((1, 0)), verb: None, once: false };
        let (_, s0, e0) = trained_lab_run(|_, _| {}, 300);
        assert!(e0.0 < s0.0 + 10, "left to her mind she went east anyway ({s0:?} to {e0:?}): the test cannot see the lead");
        let (_, start, end) = trained_lab_run(
            |lab, id| {
                assert!(force(&mut lab.world, id, east));
            },
            300,
        );
        assert!(end.0 >= start.0 + 10, "led east for 300 frames, the ant went from {start:?} to {end:?}");
    }

    /// **The nest raider is steerable** (enemy seat): a centipede with the
    /// lab's trained raider mind, held east, takes its five-cell chain east.
    /// The positive control is the same centipede left to its mind, which
    /// does not go 10 columns east in the same time on this seed.
    #[test]
    fn a_steered_centipede_takes_its_chain_where_it_is_held() {
        use crate::lab::scenario::{Placement, Scenario};
        let run = |steer: bool| -> ((i32, i32), (i32, i32), usize) {
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
            let minds = crate::lab::training::predator_minds("centipede").expect("the raider's lab pool ships");
            let b = lab.world.bounds().expect("bounded");
            let x = b.min_x + 40;
            let id = crate::train::predator::place_predator(&mut lab.world, "centipede", x, None, minds[0].clone(), 1.0).expect("a centipede lands");
            let start = lab.world.organism(id).and_then(|s| s.chain.first().copied()).expect("alive");
            let len = lab.world.organism(id).map_or(0, |s| s.chain.len());
            if steer {
                assert!(mark(&mut lab.world, id, Some(SteerInput { dir: Some((1, 0)), ..Default::default() })));
            }
            let mut end = start;
            for _ in 0..300 {
                lab.tick_for_harness();
                if let Some(at) = lab.world.organism(id).and_then(|s| s.chain.first().copied()) {
                    end = at;
                }
            }
            (start, end, len)
        };
        let (start, end, len) = run(true);
        assert_eq!(len, 5, "the raider is a five-cell chain");
        assert!(end.0 >= start.0 + 10, "held east for 300 frames (60 decisions), the raider went from {start:?} to {end:?}");
        let (s0, e0, _) = run(false);
        assert!(e0.0 < s0.0 + 10, "left to its mind the raider also went 10 east ({s0:?} to {e0:?}): the test cannot see the steer");
    }

    #[test]
    fn a_latched_press_rides_on_the_held_levels_until_it_is_used() {
        let held = SteerInput {
            dir: Some((1, 0)),
            ..Default::default()
        };
        let press = SteerInput {
            lay: true,
            ..Default::default()
        };
        let both = held.or(press);
        assert!(both.lay && both.dir == Some((1, 0)));
    }
}
