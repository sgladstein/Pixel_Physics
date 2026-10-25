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

/// **Which body the player wears** (seat demos, 2026-10-25,
/// `Reports/teaching-the-colony-2026-10-25/` and the shared board's
/// `steered-ant.md`). The queen seat and the lone-scout seat both drive an
/// ordinary ant; they differ only in the breeding rule her colony runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SteerRole {
    /// The founding queen: her colony runs the queen rule (slice 0's only
    /// role, and still what [`mark`] gives).
    #[default]
    Queen,
    /// One worker among many (the scout seat): her colony keeps its own
    /// breeding rule. Still no old age -- the owner's ruling is about the
    /// player's body, not the queen's (*"My player shouldn't die of old
    /// age"*, 2026-10-11).
    Worker,
}

/// The one body the player drives, and the rules that come with it.
#[derive(Clone, Debug)]
pub struct Steered {
    pub id: OrganismId,
    /// Queen or worker: only `regime_of_colony` reads it.
    pub role: SteerRole,
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
    mark_with(world, id, input, SteerRole::Queen)
}

/// [`mark`] with the role named: [`SteerRole::Worker`] leaves her colony's
/// breeding rule alone.
pub fn mark_with(world: &mut World, id: OrganismId, input: Option<SteerInput>, role: SteerRole) -> bool {
    let Some(state) = world.organism_mut(id) else {
        return false;
    };
    state.nest_bound_until = 0;
    let (born_frame, colony) = (state.born_frame, state.colony);
    world.steered = Some(Steered {
        id,
        role,
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

/// **The steered body, if she is alive**: what a seat's own module reads to
/// find her each tick (`world.organism(id)` for where she is).
pub fn steered_id(world: &World) -> Option<OrganismId> {
    let s = world.steered.as_ref()?;
    is_steered(world, s.id).then_some(s.id)
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
                        role: SteerRole::Queen,
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

    /// **A trained (mind) ant marked as the player's body obeys the keys**
    /// (seat demos, 2026-10-25). The lab opens on the trained pool, and
    /// `mind::tick` reads no steer hook, so without the hand-off in
    /// `creature_tick` the keys did nothing to her: `used` stayed at its
    /// default because `take_input` was never reached. Watched red with the
    /// `input_of(..).is_none()` term taken out of that branch.
    #[test]
    fn a_keyboard_drives_a_mind_ant() {
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
        let id = *ids
            .iter()
            .find(|&&i| crate::sim::creature::mind::has(&lab.world, i))
            .expect("test setup: no ant has a mind");
        let held = SteerInput {
            dir: Some((1, 0)),
            ..Default::default()
        };
        assert!(mark_with(&mut lab.world, id, Some(held), SteerRole::Worker));
        assert_eq!(steered_id(&lab.world), Some(id));
        for _ in 0..600 {
            lab.tick_for_harness();
        }
        // `used` first: with the hand-off taken out she does not take the
        // keys, and (measured) also dies inside the 600 frames.
        assert_eq!(lab.world.steered.as_ref().map(|s| s.used), Some(held), "the mind ant never took the keys");
        assert!(lab.world.organism(id).is_some(), "test setup: the steered ant died");
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
