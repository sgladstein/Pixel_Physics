//! **The option list: everything an ant could legally do this decision, each
//! described by the same features** -- stage 2, step 5 of the brain plan
//! (`brain-training/design/general-brain-design-rev3.md`, *How it decides*;
//! stage 2 chosen by the owner 2026-10-17).
//!
//! The full brain picks a job, then that job's target, by scoring every
//! (verb, target) pair with one shared scorer. This builds the pairs and
//! their features: each of the eight steps the body can take, turning
//! around, resting, each cell the jaws can cut, each item in reach to pick
//! up or eat, each free cell to put a load down in, each nestmate or larva
//! to feed and each stranger to bite. A typical ant has 10-25.
//!
//! **A mind chooses from this list** (`mind::tick`); the game ant never
//! reads it. For the game ant it is built only as a shadow, so the game is
//! unchanged with it on: that exists to time it at lab scale -- the design
//! estimated that building the options would cost more than the network
//! itself, and the whole new ant has to fit in 1.3x the game ant's frame
//! (`needscost arms=off,shadow`). The shadow is off unless
//! [`World::option_shadow`] is set.
//!
//! **Each verb does one thing** (fix plan step 1, 2026-10-18; the actions
//! audit, `brain-training/ladder/actions-audit.md`): Pick up takes food into
//! the crop, Eat eats from the crop, Dig cuts ground, Put down puts down
//! what the jaws or the crop hold. What the body forbids on purpose, so no
//! rung trains it, is listed at [`Verb`].
//!
//! **Features that need something the engine does not have yet read 0**:
//! the building scent (it comes with its emit) and the way back to the last
//! food (a new body memory). They keep their columns so the cost and the
//! layout are the full list's.
//!
//! **No transcendental anywhere a decision reads** (`brain.rs`): squashes
//! are `x / (1 + |x|)`, and the home direction's cosine is a ratio of
//! integers' square roots, which IEEE rounds exactly.
//!
//! [`World::option_shadow`]: crate::sim::World::option_shadow

use super::{
    body_has_foothold, jaw_can_cut, kin_footing, landing_after_step_into, parting_enabled, segment_authored_widths,
    spoil_site_open, stacker_of, under_cover_by, BodyShape, CellMemo, CellRead, CreatureDef, DIRS,
};
use super::{crop_to_feed, diet_yield, gut_of, organism_crop_capacity, EAT_YIELD_THRESHOLD};
use super::{flipped_body, landing_is_placeable_through_tissue};
use crate::sim::cell::OrganismId;
use crate::sim::material::{self, Known, MaterialId, MaterialKind};
use crate::sim::pheromone::{self, Channel};
use crate::sim::World;
use std::cell::RefCell;

/// What the body can do, the design's "must" verbs for the first ant.
///
/// **What the body forbids on purpose**, so no rung tries to train it (fix
/// plan step 1): these are physics or the game's settled rules, not choices
/// a mind is meant to learn, and an option the body forbids is never listed.
///
/// - **Soil goes down only where it will stay**: on ground, with three
///   empty rows above it, never in the door column (`spoil_site_open` with
///   the footing test). So an ant holding soil in a two-row tunnel carries
///   it until it reaches headroom.
/// - **One thing in the jaws, one food in the crop.** Jaws holding soil
///   cannot pick food up, and a crop holding one food takes no other.
/// - **Jaws holding soil feed nobody from the crop** (`crop_to_feed`); the
///   ant may still eat from its own crop, and still share from its bank.
/// - **A dead ant is not picked up** until carrying one is built, and is
///   not cut: Dig is for ground.
/// - **A live seed is not cut** (`jaw_can_cut`).
/// - **Feed is offered only when it would give something**: a nestmate
///   poorer than the ant, or a hungry larva that the crop or the bank can
///   feed.
/// - **Laying is the body's**, at its energy bar; and **the crop digests on
///   its own** whatever the ant chooses (Eat only hurries it).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verb {
    /// One of the eight headings the body can enter.
    Walk,
    /// A long body reverses rather than stepping into its own neck.
    TurnAround,
    Rest,
    /// A cell of ground in reach the jaws can cut, with the jaws free: not
    /// food, a dead ant or a live seed. Until fix plan step 1 the jaws could
    /// also cut food or a dead ant and carry it undigested, so putting it
    /// down counted no delivery; and Dig needed an empty crop, a rule rather
    /// than physics (the jaws cut, the crop is inside). **With a pellet in
    /// the jaws, only when walled in**: the pack move ([`pack_on`]).
    Dig,
    /// Food in reach into the crop, with the jaws free: room for one more
    /// cell, the crop's one material, and a bite worth taking to this gut.
    /// Food only (fix plan step 1): a dead ant waits for a carry of its own.
    PickUp,
    /// A load put down beside the head: a pellet where the spoil rule would
    /// leave it, a crop cell in any free cell. The jaws first: what goes
    /// down is what the jaws hold, else the crop's food.
    Drop,
    /// **The crop's own, now** (target: the head): finish the cell being
    /// digested. Until fix plan step 1 it also swallowed food in reach,
    /// which made carrying and eating one choice (trained ants ate 73 of
    /// 75 cells they picked up, `actions-audit.md`).
    Eat,
    /// A nestmate poorer than the ant, from its bank; or a hungry larva,
    /// from the crop if the crop holds food a larva can take, else from the
    /// bank, poorer than the ant (energy runs downhill, as a share does).
    /// Listed only when it would give something.
    Feed,
    /// A stranger in reach. **No act yet** (fights are parked): never
    /// picked ([`Verb::acts`](super::mind)).
    Attack,
}

pub const VERBS: usize = 9;

impl Verb {
    pub const ALL: [Verb; VERBS] = [
        Verb::Walk,
        Verb::TurnAround,
        Verb::Rest,
        Verb::Dig,
        Verb::PickUp,
        Verb::Drop,
        Verb::Eat,
        Verb::Feed,
        Verb::Attack,
    ];

    pub fn name(self) -> &'static str {
        [
            "walk",
            "turn_around",
            "rest",
            "dig",
            "pick_up",
            "drop",
            "eat",
            "feed",
            "attack",
        ][self as usize]
    }
}

/// Feature indices, in [`feature_names`] order.
pub mod f {
    /// What is in the target cell, one of ten (one-hot): these nine and
    /// [`LINED`], which came later and so sits at the end.
    pub const AIR: usize = 0;
    pub const SOIL: usize = 1;
    /// **Placed spoil only**: a pellet some ant put down. Until PR 2 it was
    /// spoil *or* packed soil, so the ant's own gallery wall (every
    /// neighbour of a cut is packed, `line_burrow`) read as a heap
    /// (`dig-rung-design.md` R5); the wall is [`LINED`] now.
    pub const SPOIL: usize = 2;
    pub const FOOD: usize = 3;
    pub const BROOD: usize = 4;
    pub const CORPSE: usize = 5;
    pub const STONE: usize = 6;
    pub const WATER: usize = 7;
    pub const ANT: usize = 8;
    /// Scent A, B and alarm there, each as strength, how much stronger
    /// than here, and how much stronger than here two cells further that
    /// way: `SCENT + 3 * channel + k`. The two differences are gained
    /// (`SCENT_GAIN`).
    pub const SCENT: usize = 9;
    /// Missing: comes with the building-scent emit. **Holds [`FOOD_WAY`]
    /// until then** (on by default, [`last_food`]).
    pub const BUILDING: usize = 18;
    /// **How much the option points back to where this ant last picked up
    /// food** ([`last_food`]): the step's cosine to that spot, times how far
    /// it is on a log scale, times how fresh the memory is. Shares
    /// [`BUILDING`]'s slot, which nothing wrote: a new slot would have moved
    /// every later weight of every saved genome and every tool that indexes
    /// them, and [`LAST_FOOD`], the slot the design named for it, has carried
    /// the far-from-home sense since 2026-10-22. The building scent takes a
    /// new slot of its own when it comes.
    pub const FOOD_WAY: usize = BUILDING;
    /// The body can stand there; it would fall; it climbs.
    pub const STAND: usize = 19;
    pub const FALL: usize = 20;
    pub const CLIMB: usize = 21;
    /// **The hole**: a cell some ant cut (`World::dug_cells`) or inside a
    /// founding cut, whatever is in it now -- so a pellet put down there,
    /// or ground fallen in, sits in the hole (PR 2; the step-3 misses,
    /// `dig-rung-design.md` 9). Declared as "the jaws can cut it" and never
    /// written until then. And how firm the cell is.
    pub const CUT: usize = 22;
    pub const HARD: usize = 23;
    /// Soil wetness there, by level.
    pub const WET: usize = 24;
    /// Up (+) or down (-) from the head; rows below the ground as built
    /// (`mind::depth_at`, the context's own depth); open cells beneath it,
    /// a cliff's height.
    pub const UP: usize = 25;
    pub const DEPTH: usize = 26;
    pub const DROP_BELOW: usize = 27;
    /// Open cells above it, in body heights; ground overhead.
    pub const HEADROOM: usize = 28;
    pub const ROOFED: usize = 29;
    /// Who is there: nestmate, stranger, laden, hungry, larva, the larva's
    /// hunger, dead.
    pub const NESTMATE: usize = 30;
    pub const STRANGER: usize = 31;
    pub const LADEN: usize = 32;
    pub const HUNGRY: usize = 33;
    pub const LARVA: usize = 34;
    pub const LARVA_HUNGER: usize = 35;
    pub const DEAD: usize = 36;
    /// Animals beside it; open cells around it per animal.
    pub const CROWD: usize = 37;
    pub const ROOM: usize = 38;
    /// How much the option points home, by the body's home vector.
    pub const HOME: usize = 39;
    /// Named for the way back to the last food, and **carries the
    /// far-from-home sense instead** ([`super::home_vec`], on by default);
    /// the last-food memory is [`FOOD_WAY`].
    pub const LAST_FOOD: usize = 40;
    /// Warmer (+) or cooler (-) there than here.
    pub const WARMTH: usize = 41;
    /// The option undoes the last step.
    pub const BACK: usize = 42;
    /// **Worked wall**: packed soil, the lining a cut leaves round itself.
    /// The tenth kind of thing in the cell, split out of [`SPOIL`] (PR 2).
    pub const LINED: usize = 43;
    /// **The bite is hanging** (Dig targets only; `creature::bite_is_hanging`):
    /// a grain touching it rests on it or on air with two or fewer other
    /// grains holding it, or a pellet rests on it -- a touch reading since
    /// 2026-10-21. Until then the crumb rule read one cut ahead (PR 2;
    /// ladder lane, `dig-rung-design.md` 9), which it matches on every Dig
    /// option of s171's soil-arena runs; the index and its trained weights
    /// carry over unchanged.
    pub const ROOF_DROP: usize = 44;
    /// The one-hot kinds of thing, in order.
    pub const KINDS: [usize; 10] = [AIR, SOIL, SPOIL, FOOD, BROOD, CORPSE, STONE, WATER, ANT, LINED];
}

/// Features per option.
pub const FEATURES: usize = 45;

/// The names of the features, for readouts.
pub fn feature_names() -> [&'static str; FEATURES] {
    [
        "air",
        "soil",
        "spoil",
        "food",
        "brood",
        "corpse",
        "stone",
        "water",
        "ant",
        "a",
        "a_diff",
        "a_ahead",
        "b",
        "b_diff",
        "b_ahead",
        "alarm",
        "alarm_diff",
        "alarm_ahead",
        "building",
        "stand",
        "fall",
        "climb",
        "cut",
        "hard",
        "wet",
        "up",
        "depth",
        "drop_below",
        "headroom",
        "roofed",
        "nestmate",
        "stranger",
        "laden",
        "hungry",
        "larva",
        "larva_hunger",
        "dead",
        "crowd",
        "room",
        "home",
        "last_food",
        "warmth",
        "back",
        "lined",
        "roof_drop",
    ]
}

/// One thing the ant could do: a verb and the cell it acts on.
#[derive(Clone, Copy, Debug)]
pub struct Opt {
    pub verb: Verb,
    pub target: (i32, i32),
    pub feats: [f32; FEATURES],
}

/// The materials the features name, looked up once a decision rather than
/// per cell (`.claude/rules/measuring-the-world.md`, hot-path work).
struct Mats {
    spoil: Option<MaterialId>,
    packed: Option<MaterialId>,
    corpse: Option<MaterialId>,
}

/// What every option of one decision shares: where the head is, what is
/// sensed there, and the body's own memory.
struct Here {
    head: (i32, i32),
    neck: Option<(i32, i32)>,
    colony: u32,
    scent: [f32; 3],
    warmth: f32,
    home: Option<(i32, i32)>,
    /// Where this ant last picked up food, and how fresh that memory is,
    /// `T / (T + age)` ([`last_food`]); `None` with the memory off or
    /// before its first pick-up.
    food: Option<((i32, i32), f32)>,
    mats: Mats,
}

fn squash(x: f32) -> f32 {
    x / (1.0 + x.abs())
}

fn scent(world: &World, ch: Channel, x: i32, y: i32) -> f32 {
    mind_scent(world, ch, x, y)
}

/// **Every scent a mind ant reads goes through here** (head, options, dig head), so the deaf and shift switches
/// below reach all of them at once. Unset, it is `pheromone_at` scaled to 0..1, exactly as before.
pub fn mind_scent(world: &World, ch: Channel, x: i32, y: i32) -> f32 {
    if scent_deaf(ch) {
        return 0.0;
    }
    let dx = scent_shift(ch);
    world.pheromone_at(ch, x + dx, y) as f32 / pheromone::Scent::MAX as f32
}

/// SCRATCH (evolvable-pheromones lane, 2026-10-25): `PP_SCENT_SHIFT=A:40,B:40` makes a mind ant read a plane
/// `dx` columns away from where it stands. **The scrambled listening test the review asked for** (H3): unlike
/// [`scent_deaf`], the ant still reads a real plane with real trails on it -- the same kind of numbers it was
/// trained on -- but they no longer describe where it is. Off the plane's edge it reads 0.
pub fn scent_shift(ch: Channel) -> i32 {
    static V: std::sync::OnceLock<[i32; 3]> = std::sync::OnceLock::new();
    V.get_or_init(|| {
        let mut d = [0; 3];
        for item in std::env::var("PP_SCENT_SHIFT").unwrap_or_default().split(',').map(str::trim).filter(|n| !n.is_empty()) {
            let (name, dx) = item.split_once(':').expect("PP_SCENT_SHIFT=CH:DX,...");
            let dx: i32 = dx.parse().expect("PP_SCENT_SHIFT dx");
            match name.to_ascii_lowercase().as_str() {
                "a" => d[Channel::A as usize] = dx,
                "b" => d[Channel::B as usize] = dx,
                "alarm" => d[Channel::Alarm as usize] = dx,
                other => panic!("PP_SCENT_SHIFT: unknown channel {other} (A, B, alarm)"),
            }
        }
        d
    })[ch as usize]
}

/// SCRATCH (evolvable-pheromones lane, 2026-10-25): `PP_SCENT_DEAF=A,B,alarm` (any subset) makes a mind ant read
/// those planes as 0 **everywhere it reads them**: the head inputs and their changes (`mind.rs`), every option's
/// strength and two differences ([`f::SCENT`]), and the dig head's channel-A feature (`targets.rs`).
/// `PP_SCENT_SILENT=` (same list) stops a mind ant *depositing* on those planes while still billing the emit, so
/// the energy is unchanged and only the information goes. **Why both, and why not `PP_SCENT_BLIND`**: the old
/// switch zeroes the six head inputs only, leaving the nine per-option features that steer each step, and
/// because the B emitter reads B it also changes what is laid -- a partial, confounded blinding (adversarial
/// review, `/mnt/project-files/evolvable-pheromones/adversarial-review-2026-10-25.md` H3). Deaf asks "does
/// anyone *read* this channel"; silent asks "does anything *written* on it matter to the colony". Unset, every
/// read and deposit is untouched (bit-identical).
pub fn scent_deaf(ch: Channel) -> bool {
    static V: std::sync::OnceLock<[bool; 3]> = std::sync::OnceLock::new();
    V.get_or_init(|| channel_list("PP_SCENT_DEAF"))[ch as usize]
}

/// `PP_SCENT_SILENT`; see [`scent_deaf`].
pub fn scent_silent(ch: Channel) -> bool {
    static V: std::sync::OnceLock<[bool; 3]> = std::sync::OnceLock::new();
    V.get_or_init(|| channel_list("PP_SCENT_SILENT"))[ch as usize]
}

fn channel_list(var: &str) -> [bool; 3] {
    let mut on = [false; 3];
    for name in std::env::var(var).unwrap_or_default().split(',').map(str::trim).filter(|n| !n.is_empty()) {
        match name.to_ascii_lowercase().as_str() {
            "a" => on[Channel::A as usize] = true,
            "b" => on[Channel::B as usize] = true,
            "alarm" => on[Channel::Alarm as usize] = true,
            "all" => on = [true; 3],
            other => panic!("{var}: unknown channel {other} (A, B, alarm, all)"),
        }
    }
    on
}

const CHANNELS: [Channel; 3] = [Channel::A, Channel::B, Channel::Alarm];

/// **The gain on a scent difference, before the squash.** Along a trail
/// the scent changes by about 0.005-0.008 of full scale per cell and
/// 0.016-0.023 over three, so raw differences would need weights of 50-150
/// to move a pick, three times the rig's bound (ladder lane,
/// `trail-rung-controls.md`, measured on the trail arena's hold-out). At 32
/// a cell's slope reads about 0.15, three cells' about 0.4, and the edge of
/// a trail, a drop of 0.1-0.3, 0.75-0.9: the whole range in use.
const SCENT_GAIN: f32 = 32.0;

/// How far up and down the head-room and cliff reads look, in cells: two
/// body heights of a three-cell ant.
const REACH: i32 = 6;

/// The window [`build_with`] remembers cells in, round the head: the
/// targets lie one cell out (the tail a few), and each reads two cells
/// round itself, [`REACH`] rows below and `under_cover`'s 20 above. A read
/// outside it is still exact, only not remembered.
const MEMO_SIDE: i32 = 6;
const MEMO_UP: i32 = 24;
const MEMO_DOWN: i32 = REACH + 4;

/// **A mind walled in with a pellet can pack its way out**, as the game's
/// ant does (`NEEDS_FIRST`'s `pack`) and the player's ant does (G): it cuts a
/// cell beside its head, steps in, and packs the pellet into the cell its tail
/// leaves (`pack_behind`). Without it a mind holding soil in a pocket no
/// bigger than its body has no move but stepping into itself, turning round or
/// resting: its jaws are full, so no cut, and there is no room for a put-down.
/// Owner's ruling, 2026-10-22, on the foraging-loop thread's measurement: in
/// the in-run bed with the carry-home teacher acting, 25 of 55 starved ants
/// died walled in holding a pellet; with the move, starvation went 55 -> 25
/// of 192 (8 colonies, 200k frames). On by default;
/// `PIXEL_PHYSICS_MIND_PACK=off` is the body before. Minds only: the game's
/// ant has its own pack in `NEEDS_FIRST`.
pub fn pack_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("PIXEL_PHYSICS_MIND_PACK").map_or(true, |v| v != "off"))
}

/// **A mind goes round a trunk** (plant-food lane, 2026-10-25; the owner, asked whether the trained ant should get the
/// old ant's trunk move: *"Yes. Do it"*). A Walk into living wood is offered when the game's own `trunk_crossing`
/// finds the far side, and taking it starts the same `organism::Crossing` the game's ant makes (`mind::walk`): the
/// body waits thickness x interval frames, comes out on the far side, and pays the steps it replaced. Before it, wood
/// was a wall to a mind (`is_partable` refuses woody tissue and the Walk landings filter on it), so a trained ant had
/// to climb over a stem: on the played bed (`examples/treewalk`, 4 seeds x 20k frames) it spent 5.2-8.3% of its time
/// stuck beside wood against 0.3-2.3% for the game's ant. **On by default**; `PIXEL_PHYSICS_MIND_CROSS=off` is the body
/// before, and `CROSS_TRUNK=0` turns crossing off for every animal. A bed with no living wood (the training pile bed)
/// offers no new option, so it is unchanged.
pub fn cross_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| match std::env::var("PIXEL_PHYSICS_MIND_CROSS").as_deref() {
        Ok("on") | Err(_) => super::crossing_enabled(),
        Ok("off") => false,
        Ok(other) => panic!("PIXEL_PHYSICS_MIND_CROSS={other:?}: use on or off"),
    })
}

/// **A mind can bite** (lone predator thread, 2026-10-23): `PIXEL_PHYSICS_MIND_ATTACK=on` lands the Attack act
/// (`mind::attack`), the game's own bite on a stranger in reach, and lets a mind pick up a cell of corpse its gut can
/// use. **Why**: a predator that hunts by a mind needs a verb that kills and a mouth for what it killed; until now
/// Attack was listed and never picked, and a dead ant was never picked up (no carry of one is built, and an ant
/// eating its colony's dead was not what the rule was about). **Built off, because it changes results**: a
/// mind ant's genome can carry a live Attack switch, and in a box with two colonies or a predator it would start
/// biting. Off, nothing is read or written: Attack is never picked and a corpse never offered, as before.
///
/// **On by default since 2026-10-24** (owner: "Everything should be on by default"); `=off` is the game before.
/// What the default costs, measured: on the colony bed with no spider the bite has no stranger to act on, and the
/// blank verb it enters as (`mind::enter_bite`) cost nothing past the mutant draw's own noise (stage two, 12 seeds);
/// a genome that holds Attack banned still never bites outside `inrun`, which is the only caller of `enter_bite`.
/// **Not every trained pool holds it banned** (seat reviews, 2026-10-25): `assets/minds/std_pool.txt`, the lab's
/// trained ant, sets the Attack switch (slot 8) to 1 in member 0 and no other member's delta touches it, so all 30
/// members bite a stranger in reach (`mind::attack`). Read a pool's slot 8 before assuming its ants are peaceable.
pub fn attack_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| match std::env::var("PIXEL_PHYSICS_MIND_ATTACK").as_deref() {
        Ok("on") | Err(_) => true,
        Ok("off") => false,
        Ok(other) => panic!("PIXEL_PHYSICS_MIND_ATTACK={other:?}: use on or off"),
    })
}

/// **A mind may pick up a corpse** ([`attack_on`]'s mouth for what it killed): on with the bite, unless
/// `PP_MIND_CORPSE=off` (SCRATCH, lone predator thread, 2026-10-24), which keeps the bite and the old refusal of a
/// dead ant -- built to ask whether a colony's own dead in its mouths, rather than the bite, is what a run with
/// the switch on and no predator changes.
pub fn corpse_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    attack_on() && *ON.get_or_init(|| std::env::var("PP_MIND_CORPSE").map_or(true, |v| v != "off"))
}

/// **A mind with eyes sees its prey** (lone predator thread, 2026-10-24; owner: "It seems realistic that spiders
/// should use sight"): `PIXEL_PHYSICS_MIND_SIGHT=on` lets a mind whose body has an eye (`CreatureDef::sight_range`
/// above 0 -- the spider's 64, never the ant's 0) cast the game's own eye (`creature::sighted`: rays all round,
/// stopped by soil and rock) once a decision, and read the nearest prey it reached **on its home senses**: the
/// context's home seen, distance and its change, and each option's step-toward-home, through [`sensed_home`].
///
/// **Why the home senses, not new ones**: a new input would move every weight after it in every genome on disk,
/// the ants' included. A spider has no nest; its home was the cell it landed on, a point that says nothing about
/// where the ants are. So for a sighted mind "home" becomes "the prey I can see" -- no prey in sight reads as no
/// home -- and an eyeless mind (every ant) runs exactly as before. The eye is billed as every eye is,
/// `sight_fraction x start_energy` a cell read (`mind::tick`).
///
/// **On by default** (owner, 2026-10-24: "Everything should be on by default"): measured, spiders trained with it
/// from the hand-built hunter killed 80.6 ants a judging against 40.9 without (19 of 24 paired judgings won, 4
/// lost; 4 seeds x 6 rounds, judged on 6 fresh beds). **The cost is to spiders trained blind**: the stage-one pools'
/// home weights were learnt on their landing cell, and retrained with sight they killed 63 against 87 -- use a
/// pool trained sighted (`predator/pools/sight-h2`), or `=off` for a blind one. An eyeless mind (every ant) casts
/// nothing either way; `=off` casts no ray for anyone.
pub fn sight_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| match std::env::var("PIXEL_PHYSICS_MIND_SIGHT").as_deref() {
        Ok("on") | Err(_) => true,
        Ok("off") => false,
        Ok(other) => panic!("PIXEL_PHYSICS_MIND_SIGHT={other:?}: use on or off"),
    })
}

/// How many ticks a sighted mind cast its eye, and on how many it saw prey ([`sight_on`]; counted once a tick, where
/// the eye is billed): the count that says whether the sense fired at all.
pub static SIGHT_CASTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static SIGHT_SEEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// **Where a mind's home senses point** ([`sight_on`]): the nearest prey a sighted mind can see, or for any other
/// mind its forage anchor (none in a home-blind world). Returns the eye's cell reads too, 0 when no eye was cast.
pub fn sensed_home(world: &World, organism: OrganismId, def: &CreatureDef, st: &crate::sim::organism::OrganismState) -> (Option<(i32, i32)>, u64) {
    if sight_on() && def.sight_range > 0 {
        if let Some(&(hx, hy)) = st.chain.first() {
            let (seen, reads) = super::sighted(world, hx, hy, organism, def);
            if reads > 0 {
                return (seen.prey.map(|s| (s.x, s.y)), reads);
            }
        }
    }
    ((!world.home_blind).then_some(st.forage_anchor), 0)
}

/// **Whether an option feels how warm its target is** (`f::WARMTH`): on by default; `PIXEL_PHYSICS_MIND_WARMTH=off`
/// leaves the feature at 0 and reads no temperature. SCRATCH port of the creature-performance thread's switch
/// (mind-warmth-switch.patch, written against main) for the in-run gate check, 2026-10-22.
pub fn warmth_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| match std::env::var("PIXEL_PHYSICS_MIND_WARMTH").as_deref() {
        Ok("on") | Ok("") | Err(_) => true,
        Ok("off") => false,
        Ok(other) => panic!("PIXEL_PHYSICS_MIND_WARMTH={other:?}: use on or off"),
    })
}

/// **A default-on switch** (owner, 2026-10-24): on unless the variable is set to `0` or `off`. The standard training
/// path's senses and verbs read this, so the lab, training and every harness run them without a `setenv=`; the old
/// behaviour is one `=off` away.
pub fn switch_on(name: &str) -> bool {
    std::env::var(name).map_or(true, |v| switch_on_value(&v))
}

/// The value half of [`switch_on`]: anything but `0` or `off` is on.
pub fn switch_on_value(v: &str) -> bool {
    !matches!(v.trim(), "0" | "off")
}

/// SCRATCH (far-from-home sense thread, 2026-10-22): `PP_HOMEVEC=R:W`, a home vector that stays sharp
/// far out. **Why**: the body's home vector reaches a mind only squashed -- `f::HOME` is `cos * squash(d / 32)` and
/// `c::HOME_DIST` is `squash(d / 32)`, which read 0.44 at 25 columns (the food band), 0.65 at 60 and 0.76 at 100.
/// Two lessons that need "far from home" (come home when empty and far, starvation trace; stay on the trail outward,
/// maze thread) copied well onto hA1 and still leaked into foraging in the bed, and both write-ups name this ramp.
/// With it on: every option gets `f::LAST_FOOD` (unwritten until now) = the step's cosine to home times
/// [`far_ramp`], and the context's `c::D_SCENT + 2` (d_alarm, 0 on every bed decision: nothing lays alarm) reads the
/// ramp itself. Both are exactly 0 within `R` of home. Off: nothing is read or written, byte-identical.
/// A genome with live links on either slot must have them zeroed before the switch means anything (hA1 has 8 and 16).
///
/// **On by default at `40:30`** (owner, 2026-10-24: "if we have decided these are the correct path forward they should
/// be default on"): every colony-selection batch since colsel18 ran with it, and the standard line's founder `hcs` and
/// everything selected from it was trained with it. `PP_HOMEVEC=off` (or `0`) is the old mind, byte-identical.
///
/// **`PP_HOMEVEC=log:S`** (teacher lane, 2026-10-24; owner: "why does the sense need to be binarized at all"): the same
/// two slots read `ln(1 + d / S)` instead of the ramp -- graded from the door, never capped (S = 40: 0.49 at 25
/// columns, 1.25 at 100, 2.14 at 300), so a mind can tell 70 columns from 150. **Why**: the ramp saturates at `R + W`
/// (70 at the default), and every pool selected with food 25-45 columns out learned "far reads 1 -> turn for home":
/// traced on farfood1's best colony with food 60 out, empty ants stepped homeward 29-31% of the time within 60
/// columns and 60-75% past 70. In a lab box with food 100 columns out it lived only with the ramp moved out
/// (80:60, 100:100 or off: 28-134 alive against 0-2). The dead zone within `R` was a guard for copying the come-home
/// lesson onto a forager without it leaking into foraging near home; colony selection does not copy. Encoded as `R` =
/// NaN, read by [`far_ramp`].
///
/// **The default since farfood3b (teacher lane, 2026-10-24 19:00Z) is `log:40`.** Owner: "I prefer the graded option
/// unless it is significantly worse and cannot be trained to be better." Measured on one founder (far64z, its far
/// links dropped), 32 colonies x 24 rounds each, food ramped from 80 columns: graded against no far sense at all, the
/// lab case at round 23 was 1,183 vs 1,292 (11 of 32 colonies higher, n.s.), and the common garden starved 43.5 vs 103.
/// In lab boxes the two arms' best colonies were level (fruit 150 columns out: 95-117 alive graded, 101-142 none;
/// hand-built ant 127-159). `40:30` stays reachable for an old pool: `PP_HOMEVEC=40:30`.
pub fn home_vec() -> Option<(f32, f32)> {
    static V: std::sync::OnceLock<Option<(f32, f32)>> = std::sync::OnceLock::new();
    *V.get_or_init(|| {
        let v = std::env::var("PP_HOMEVEC").unwrap_or_else(|_| "log:40".to_string());
        if !switch_on_value(&v) {
            return None;
        }
        if let Some(s) = v.strip_prefix("log:") {
            let s: f32 = s.parse().expect("PP_HOMEVEC=log:S");
            assert!(s > 0.0, "PP_HOMEVEC=log:S, S > 0");
            return Some((f32::NAN, s));
        }
        let p: Vec<f32> = v.split(':').map(|x| x.parse().expect("PP_HOMEVEC=R:W")).collect();
        assert!(p.len() == 2 && p[1] > 0.0, "PP_HOMEVEC=R:W, W > 0");
        Some((p[0], p[1]))
    })
}

/// SCRATCH (far-from-home sense thread, 2026-10-22): `PP_HOMEDIR=1`, home's direction without its
/// distance: every option gets `f::BUILDING` (unwritten; only the scratch `PP_BOTTOM` borrows it, on Dig options) =
/// the step's cosine to home, unsquashed. **Why**: `f::HOME` = `cos * squash(d / 32)` is weakest near home -- 0.24 at
/// 10 columns -- which is where the maze thread's trail rule 2 ("stay on the trail, step outward") has to know which
/// way is out; its copies dithered (back-steps on 59% of sideways steps against 45%). A genome's live links on the
/// slot must be zeroed first (hA1 has 11). Unset: nothing is written, byte-identical.
pub fn home_dir() -> bool {
    static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *V.get_or_init(|| {
        let on = std::env::var("PP_HOMEDIR").is_ok_and(|v| v == "1");
        assert!(!(on && std::env::var("PP_BOTTOM").is_ok()), "PP_HOMEDIR and PP_BOTTOM both write f::BUILDING");
        on
    })
}

/// **The last-food memory** (`PP_LASTFOOD`; Creatures research thread, 2026-10-25, for the teacher lane): each ant
/// remembers where it last picked up food away from home (`OrganismState::last_food`, set by `mind::pick_up`), and
/// every option reads [`f::FOOD_WAY`] = the step's cosine to that spot x [`far_ramp`]`(d, log:40)` x `T / (T + age)`,
/// `age` the frames since the pick-up. Returns `T`, how many frames the memory takes to fade to half (default
/// 4,000: about two to four foraging trips in the lab box); `PP_LASTFOOD=off` (or `0`) writes nothing and the
/// mind is byte-identical to before; `PP_LASTFOOD=T` sets the fade.
///
/// **Why** (`Reports/brain-training/research/creatures-1996-lessons-2026-10-25.md`, idea 1): a trained ant's
/// weights are fixed at birth, so nothing it knows says where food is in *this* world, and in the owner's 1024-wide
/// lab box with fruit 150-200 columns out the trained pool walks at random and starves (`far-food-1024`). Real ants
/// remember a feeder; the rev 3 design lists a last-food vector as a *must* body memory, and nothing kept one.
///
/// **It enters at weight 0**: a genome file saved before it (layout 4 or older, `mind::LAYOUT`) has every scorer
/// unit's weight on the slot zeroed as it is read (`mind::enter_last_food`): the slot read 0 until now, so those
/// weights moved no choice, and a pool selected before the memory must not act on it at random weights. Their only
/// effect was the link bill, which is the one way it changes such a founder's ants. A file saved by a build with
/// the memory keeps what evolution put there.
///
/// **On by default** (owner, 2026-09-27 and the project rule: features ship on unless there is a measured harm): the
/// feature is a sense at weight 0, so a pool moves only as selection wires it. Panics with `PP_HOMEDIR=1` or
/// `PP_BOTTOM`, the scratch switches that wrote the same unwritten slot -- set `PP_LASTFOOD=off` to run them.
pub fn last_food() -> Option<f32> {
    static V: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    *V.get_or_init(|| {
        let v = std::env::var("PP_LASTFOOD").unwrap_or_default();
        if !v.is_empty() && !switch_on_value(&v) {
            return None;
        }
        assert!(
            !(std::env::var("PP_HOMEDIR").is_ok_and(|v| v == "1") || std::env::var("PP_BOTTOM").is_ok()),
            "PP_HOMEDIR and PP_BOTTOM write f::BUILDING, which holds the last-food memory: set PP_LASTFOOD=off to run them"
        );
        match v.trim() {
            "" | "on" | "1" => Some(LAST_FOOD_FADE),
            t => {
                let t: f32 = t.parse().unwrap_or_else(|_| panic!("PP_LASTFOOD={t:?}: off, on, or a fade in frames"));
                assert!(t > 0.0, "PP_LASTFOOD=T, T > 0");
                Some(t)
            }
        }
    })
}

/// [`last_food`]'s default fade: the memory reads half as strong this many frames after the pick-up.
pub const LAST_FOOD_FADE: f32 = 4000.0;

/// [`last_food`]'s distance scale, the far sense's own (`PP_HOMEVEC=log:40`): 0.49 at 25 columns, 1.25 at 100,
/// 2.14 at 300, never capped (owner, 2026-10-24: graded senses, no cap).
pub const LAST_FOOD_SCALE: f32 = 40.0;

/// SCRATCH (sense-cap audit, 2026-10-24): `PP_HUNGRY_GRADED`, [`f::HUNGRY`] graded ([`describe`]). **On by default
/// since laygene2** (teacher lane, 2026-10-24; owner: "the default should be on unless there is uncertainty"): the
/// colonies selected with it on (laygene2's lgkeep arm) are the standard pool, and the previous pool, never trained
/// with it, lived as well or better with it on in lab boxes (fruit 150 columns out 108-115 alive against 94-104).
/// `PP_HUNGRY_GRADED=off` is the one-bit feature.
fn hungry_graded() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| switch_on("PP_HUNGRY_GRADED"))
}

/// [`home_vec`]'s distance: 0 within `r` of home, rising linearly to 1 at `r + w`, 1 beyond; or, for `log:S` (`r` NaN),
/// `ln(1 + d / w)`, graded and uncapped.
pub fn far_ramp(d: f32, r: f32, w: f32) -> f32 {
    if r.is_nan() {
        return (1.0 + d.max(0.0) / w).ln();
    }
    ((d - r) / w).clamp(0.0, 1.0)
}

fn eat_below() -> Option<f32> {
    static V: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    *V.get_or_init(|| std::env::var("PP_EAT_BELOW").ok().and_then(|v| v.parse().ok()))
}

/// SCRATCH (foraging-loop thread's switch, ported by the teacher lane 2026-10-22): `PP_NEEDDROP=1`, the old ant's
/// hungry put-down for a mind. Unset: no change.
pub fn need_drop_on() -> bool {
    static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *V.get_or_init(|| std::env::var("PP_NEEDDROP").is_ok_and(|v| v == "1"))
}

/// **Every option `organism` has right now**, appended to `out` (cleared
/// first). A pure read of the world: no draw, no write.
pub fn build(world: &World, organism: OrganismId, def: &CreatureDef, out: &mut Vec<Opt>) {
    build_with(world, organism, def, true, out);
}

/// [`build`], reading [`f::ROOF_DROP`] only when `roof` is set. **Why**:
/// it was the dearest feature on the list as a forecast (`creature::cut_drops_roof`,
/// about 1-3% of a soil hold-out run and 4-6% of a scarce one, paired
/// timings with it on and off, PR 2), it sits only on Dig options, and a
/// mind whose Dig is switched off never scores one (`mind::decide`). So
/// the mind passes its Dig switch, and its picks are the same either way.
pub fn build_with(world: &World, organism: OrganismId, def: &CreatureDef, roof: bool, out: &mut Vec<Opt>) {
    build_remembering(world, organism, def, roof, out, true);
    home_eat_crutch(world, organism, def, out);
}

/// SCRATCH (brood lane, 2026-10-23; not for merging): `PP_HOMEEAT=H` -- a weaned crutch, the crater's pattern. An ant at
/// home below `H` grants (`start_energy`) with a pick-up in reach is offered only its pick-ups, so it eats what it is
/// standing beside. Why: with the training bed's real laying on (`inrun brood=lay`), colsel20's evolved colonies die
/// with 106-188 kJ of crumbs in the nest, and starving ants at home touching crumbs, crop empty, jaws free, chose Walk
/// 200 of 214 times (`PP_HUNGRYPROBE`): nothing in the bed had ever let an ant go hungry. `PP_HOMEEAT_P=p` (default 1)
/// applies it per ant per 1,000-frame window with probability p (a pure hash), so colsel's `wean` can take it away.
fn home_eat_crutch(world: &World, organism: OrganismId, def: &CreatureDef, out: &mut Vec<Opt>) {
    static H: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    static P: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    let Some(h) = *H.get_or_init(|| std::env::var("PP_HOMEEAT").ok().and_then(|v| v.parse().ok())) else { return };
    let p = *P.get_or_init(|| std::env::var("PP_HOMEEAT_P").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0));
    if !out.iter().any(|o| o.verb == Verb::PickUp) {
        return;
    }
    let Some(st) = world.organism(organism) else { return };
    let Some(&(x, y)) = st.chain.first() else { return };
    // "At home" is inside the nest's bowl -- within 12 columns of a nest site, at or below its door -- not the body's
    // `food_at_home`, which is the door's ring only: the crumbs lie in the bowl (first run: the crutch never fired).
    let in_nest = world.nest_sites.iter().any(|n| (x - n.x).abs() <= 12 && y >= n.surface);
    if st.energy >= h * def.start_energy || !in_nest {
        return;
    }
    if p < 1.0 && super::rng::stream(world.seed, organism as u64, world.frame / 1000, 0xB00D_0001).unit_f32() >= p {
        return;
    }
    out.retain(|o| o.verb == Verb::PickUp);
    HOME_EAT_FORCED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// How many decisions [`home_eat_crutch`] narrowed (brood lane scratch; read by `inrun`).
pub static HOME_EAT_FORCED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// [`build_with`], with its cell window ([`MEMO_SIDE`]) or, `remember`
/// false, with none -- every read straight from the world, as before the
/// window: the reference the window is held to in tests.
fn build_remembering(world: &World, organism: OrganismId, def: &CreatureDef, roof: bool, out: &mut Vec<Opt>, remember: bool) {
    out.clear();
    let Some(st) = world.organism(organism) else { return };
    let Some(&head) = st.chain.first() else { return };
    let here = Here {
        head,
        neck: st.chain.get(1).copied(),
        colony: st.colony,
        scent: CHANNELS.map(|ch| scent(world, ch, head.0, head.1)),
        warmth: if warmth_on() { world.field_at_bilinear(head.0 as f32, head.1 as f32).temperature } else { 0.0 },
        home: sensed_home(world, organism, def, st).0,
        food: last_food().and_then(|t| {
            st.last_food.map(|(x, y, at)| ((x, y), t / (t + world.frame.saturating_sub(at) as f32)))
        }),
        mats: Mats {
            spoil: world.materials.known(Known::Spoil),
            packed: world.materials.known(Known::PackedSoil),
            corpse: world.materials.known(Known::Corpse),
        },
    };
    // **Every cell the options' features read, read once** (`CellMemo`):
    // the targets are the head's ring and its tail, and each reads a 5x5
    // round itself, `REACH` rows down and `under_cover`'s 20 up, so
    // neighbouring targets read most of the same cells. Same cells, same
    // values: a pure read of an unchanging world (performance lane,
    // 2026-10-21).
    let mut memo = if remember {
        CellMemo::new(world, (head.0 - MEMO_SIDE, head.1 - MEMO_UP), (head.0 + MEMO_SIDE, head.1 + MEMO_DOWN))
    } else {
        CellMemo::new(world, head, (head.0 - 1, head.1 - 1))
    };
    // **Each target described once**: several verbs can aim at one cell
    // (rest and eat at the head, dig and pick-up at one soil cell), and
    // `describe` is a pure read of an unchanging world, so a second option
    // at a target copies the first's features before its own extras. Off
    // with `remember`, which is how `the_option_window_changes_no_feature`
    // holds both to the plain reads.
    // The list is this thread's, kept between builds (`DESCRIBED`).
    let mut described = DESCRIBED.take();
    described.clear();
    // The targets' temperatures, the field's corners read once for the few
    // blocks they fall in (`field::BilinearSampler`). Off with `remember`.
    let mut warm = remember.then(|| world.field_bilinear_sampler());
    let mut push = |verb: Verb, target: (i32, i32), extra: &dyn Fn(&mut [f32; FEATURES])| {
        let mut feats = match described.iter().find(|(t, _)| remember && *t == target) {
            Some(&(_, feats)) => feats,
            None => {
                let feats = describe(world, &mut memo, warm.as_mut(), &here, target);
                described.push((target, feats));
                feats
            }
        };
        extra(&mut feats);
        out.push(Opt { verb, target, feats });
    };

    // Walk: the eight landings, each legal if the body fits there, with the
    // foothold as a feature rather than a filter (`usable_headings` filters
    // it), so a step off an edge is an option the brain can refuse.
    let groups = &st.segment_groups;
    let authored = segment_authored_widths(def, st.fates);
    let parting = parting_enabled();
    let stacker = stacker_of(world, organism);
    let kin = kin_footing(world, organism, def);
    // SCRATCH (`PP_NEEDDROP=1`): a hungry ant (`needs_hungry`, as `NEEDS_FIRST` reads it) holding soil may put it down
    // where the game ant's need would (`need_drop_site`: any footing beside the head, not in a doorway, not closing a
    // way). The starvation trace (2026-10-22): a third of hA1's starved ants die at home holding a pellet by a stocked store.
    let need_site = if need_drop_on() && st.spoil.is_some_and(|s| !s.store) && super::needs_hungry(world, st, def, head) {
        super::need_drop_site(world, head)
    } else {
        None
    };
    // One buffer for all eight landings (`landing_after_step_into`).
    let mut landing = Vec::with_capacity(st.chain.len());
    // SCRATCH (foraging-loop thread): `PP_DROPWHY=1` counts, for a pellet holder under 1 tank, whether any cell
    // beside the head is empty, whether any is an on-ground spoil site, and whether the game ant's need site exists.
    static DROPWHY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *DROPWHY.get_or_init(|| std::env::var("PP_DROPWHY").is_ok()) && st.spoil.is_some() && st.energy < def.start_energy {
        use std::sync::atomic::Ordering::Relaxed;
        let ring: Vec<(i32, i32)> = DIRS.iter().map(|&(dx, dy)| (head.0 + dx, head.1 + dy)).collect();
        let empty = ring.iter().filter(|p| world.is_empty(p.0, p.1)).count();
        let open = ring.iter().filter(|p| spoil_site_open(world, p.0, p.1, true)).count();
        let ants = ring.iter().filter(|p| { let o = world.get(p.0, p.1).organism_id(); o != 0 && o != organism }).count();
        let c = &super::mind::scratch_log::DROPWHY;
        c[0].fetch_add(1, Relaxed);
        if empty > 0 { c[1].fetch_add(1, Relaxed); }
        if open > 0 { c[2].fetch_add(1, Relaxed); }
        if super::need_drop_site(world, head).is_some() { c[3].fetch_add(1, Relaxed); }
        if ants >= 3 { c[4].fetch_add(1, Relaxed); }
        if empty == 0 && ants >= 1 { c[5].fetch_add(1, Relaxed); }
    }
    for d in 0..8u8 {
        let (dx, dy) = DIRS[d as usize];
        let t = (head.0 + dx, head.1 + dy);
        // `from` and `to` both `d`, as `usable_headings` asks it: the same
        // predicate the walk and the tumble use.
        landing_after_step_into(
            world,
            def,
            BodyShape {
                chain: &st.chain,
                groups,
                authored: &authored,
            },
            t,
            (d, d),
            parting,
            &mut landing,
        );
        if !landing_is_placeable_through_tissue(world, &st.chain, &landing, parting, stacker) {
            // A trunk in the way: the step is the crossing (`cross_on`), footed by the trunk it goes round.
            // `trunk_crossing` is `None` at once unless `t` itself is living wood.
            let body = BodyShape {
                chain: &st.chain,
                groups,
                authored: &authored,
            };
            if cross_on() && super::trunk_crossing(world, def, body, d, parting, stacker).is_some() {
                push(Verb::Walk, t, &|feats| {
                    feats[f::STAND] = 1.0;
                    feats[f::CLIMB] = f32::from(u8::from(dy < 0));
                });
            }
            continue;
        }
        let footed = body_has_foothold(world, def, &landing, t, kin);
        push(Verb::Walk, t, &|feats| {
            feats[f::STAND] = f32::from(u8::from(footed));
            feats[f::FALL] = f32::from(u8::from(!footed));
            feats[f::CLIMB] = f32::from(u8::from(footed && dy < 0));
        });
    }
    // Turning around is the walk's own flip (`flipped_body`): legal when
    // the reversed body lays out without crossing itself.
    if st.chain.len() > 1 && flipped_body(world, &st.chain, groups, &authored, parting).is_some() {
        let tail = *st.chain.last().expect("len > 1");
        push(Verb::TurnAround, tail, &|_| {});
    }
    push(Verb::Rest, head, &|_| {});

    let jaws_free = st.spoil.is_none();
    let crop = st.crop;
    let crop_food = crop.is_some();
    // **What the pick-up allows** (`act`'s ingest): room for one more cell
    // at the crop's own unit, the crop's one material, and a bite worth
    // taking to this gut. Only legal options are listed, so a mind never
    // picks a pick-up the body would refuse.
    let gut = gut_of(world, organism, def);
    let room = crop.is_none_or(|c| c.worth() + c.unit <= organism_crop_capacity(world, organism, def));
    let feedable = crop_to_feed(world, st);
    // **The pack move** ([`pack_on`]): walled in with a pellet in the jaws,
    // every cut the jaws could make is offered, and `mind::dig` packs the
    // pellet into the cell the tail leaves.
    let pack = !jaws_free && pack_on() && st.spoil.is_some_and(|s| !s.store) && super::walled_in(world, organism, head);
    for &(dx, dy) in DIRS.iter() {
        let t = (head.0 + dx, head.1 + dy);
        let cell = world.get(t.0, t.1);
        let food = world.materials.get(cell.material).food_energy > 0.0 && (cell.organism_id() == 0 || super::mind::scratch_log::grazeable(world, cell));
        if (jaws_free || pack) && diggable(world, def, organism, cell) && !review_body_rules_refuse(world, t) {
            push(Verb::Dig, t, &|feats| {
                if roof {
                    feats[f::ROOF_DROP] = f32::from(u8::from(super::bite_is_hanging(world, t)));
                }
            });
        }
        let corpse = here.mats.corpse.is_some_and(|c| cell.material == c);
        if jaws_free
            && food
            && (!corpse || corpse_on())
            && room
            && crop.is_none_or(|c| c.material == cell.material)
            && diet_yield(world, cell, gut.bias) > EAT_YIELD_THRESHOLD
        {
            push(Verb::PickUp, t, &|_| {});
        }
        // A pellet goes down where it will stay (the spoil rule); a crop
        // cell in any free cell, as the food drop's first rule puts one.
        // **On ground, always** (`SPOIL_FOOTING=ground`'s test), though the
        // game ships that switch off: off, "two of the three cells below
        // are filled" passes on the carrier's own back, on a nestmate and
        // over a hole between two diagonals, and those pellets turn loose
        // and fall (64% of them, `dead-ends.md` 1206). Where a mind's soil
        // goes is its choice only if it stays where it was put. These
        // options serve minds alone, so the game is untouched (review of
        // the dig rung design, finding 6).
        let drop = if st.spoil.is_some() {
            spoil_site_open(world, t.0, t.1, true) || need_site == Some(t)
        } else {
            crop.is_some_and(|c| c.cells > 0) && world.is_empty(t.0, t.1)
        };
        if drop {
            push(Verb::Drop, t, &|_| {});
        }
        let other = cell.organism_id();
        if other != organism {
            if let Some(o) = animal(world, other) {
                if o.colony == here.colony {
                    // Downhill for a bank, and a larva only while it is a
                    // larva and short of its target: an egg or a pupa is not
                    // fed (`kin_deficit`). **Only when it would give
                    // something** (fix plan step 1): the crop route gives
                    // what the larva keeps of the cell still to chew
                    // (`brood::crop_feed_from`), so nothing for a food its
                    // gut cannot use or a cell chewed through, and then the
                    // bank must be richer. Until then such a Feed was listed,
                    // picked and did nothing (`MindCounts::failed`).
                    let poorer = o.energy < st.energy;
                    let crop_gives = feedable.is_some_and(|c| {
                        let (quality, overhead) = super::yield_of(world, other, def, c.material);
                        quality * (1.0 - overhead) > 0.0 && c.unit - c.digesting > 0.0
                    });
                    // SCRATCH (foraging-loop thread): `PP_NOADULTFEED=1` offers Feed to larvae only, to test whether
                    // adult-to-adult sharing (24% of h1's decisions, each billed as jaw work) is what drains a home
                    // crowd in lockstep.
                    let fed = match o.brood {
                        None => poorer && std::env::var("PP_NOADULTFEED").is_err() && !stomach_closed(world, head),
                        Some(b) => {
                            b.stage == crate::sim::organism::BroodStage::Larva
                                && o.energy < b.target
                                && (crop_gives || poorer)
                        }
                    };
                    if fed {
                        push(Verb::Feed, t, &|_| {});
                    }
                } else {
                    push(Verb::Attack, t, &|_| {});
                }
            }
        }
    }
    // SCRATCH (foraging-loop thread): `PP_EAT_BELOW=N` offers Eat only while the bank holds under N tanks
    // (start_energy), so a full ant cannot choose to swallow its load. Unset: no change.
    let eat_ok = eat_below().is_none_or(|n| world.organism(organism).is_some_and(|s| s.energy < n * def.start_energy));
    if crop_food && eat_ok {
        push(Verb::Eat, head, &|_| {});
    }
    // SCRATCH (teacher lane, 2026-10-21): `PP_BOTTOM=<reach>` writes a candidate "bottom of the hole"
    // sense onto every Dig option, borrowing the unwritten `f::BUILDING` slot: 1 when no open cell of the cavity the
    // head stands in (flooded through open cells within Chebyshev `reach` of the head) lies lower than the target.
    if let Some(r) = bottom_reach() {
        let low = super::mind::scratch_log::cavity_low(world, organism, head, r);
        for o in out.iter_mut().filter(|o| o.verb == Verb::Dig) {
            o.feats[f::BUILDING] = f32::from(u8::from(o.target.1 >= low));
        }
    }
    DESCRIBED.set(described);
}

/// **Is the shared stomach closed where this head stands?** The colony mind's verb (`Minds::closed_stomach`); false in
/// any world without minds or without a closed disc, so the shipped lab never pays more than the `Option` read.
fn stomach_closed(world: &World, (hx, hy): (i32, i32)) -> bool {
    world.minds.as_ref().is_some_and(|m| m.closed_stomach.iter().any(|&(x, y, r)| (hx - x).pow(2) + (hy - y).pow(2) <= r * r))
}

fn bottom_reach() -> Option<i32> {
    static R: std::sync::OnceLock<Option<i32>> = std::sync::OnceLock::new();
    *R.get_or_init(|| std::env::var("PP_BOTTOM").ok().and_then(|v| v.parse().ok()))
}

thread_local! {
    /// [`build_remembering`]'s list of targets described, kept between
    /// builds so a decision does not allocate it (ant cost review A2).
    static DESCRIBED: std::cell::Cell<Vec<Described>> = const { std::cell::Cell::new(Vec::new()) };
}

/// A target and the features [`describe`] gave it.
type Described = ((i32, i32), [f32; FEATURES]);

/// **Ground a mind's jaws can cut**: what `jaw_can_cut` allows, less food
/// and dead ants. A corpse is food too (`food_energy` 120), so the one test
/// keeps both out. **Why**: `jaw_can_cut` takes any loose cell no harder
/// than the jaw, so a mind's Dig could cut a food cell or a dead ant and
/// carry it in the jaws undigested, and its put-down counted no delivery
/// (actions audit, confirmed by the review's probe): a second way to carry
/// food. The game ant's own cut is untouched.
///
/// **Unless the species digs what it cannot eat** (`CreatureDef::digs_inedible_food`, the centipede, nest raider
/// thread, 2026-10-25): then food this gut would not take ([`diet_yield`] at or under the eating bar) is ground
/// like any other, so a carnivore can shove through a colony's store of crumbs. Never a cell of a living
/// organism, and never food it could eat: that is still the pick-up's.
pub(super) fn diggable(world: &World, def: &CreatureDef, organism: OrganismId, cell: crate::sim::cell::Cell) -> bool {
    let not_food = world.materials.get(cell.material).food_energy <= 0.0
        || (def.digs_inedible_food && cell.organism_id() == 0 && diet_yield(world, cell, gut_of(world, organism, def).bias) <= EAT_YIELD_THRESHOLD);
    not_food && jaw_can_cut(world, def, organism, cell)
}

/// **Whether `(x, y)` is part of a hole**: dug at or below the ground as
/// built ([`World::dug_cells`], which holds every cut `cut_cell` makes and
/// the founding cut, and `World::room_surface_at`).
///
/// **The datum is what keeps a heap out.** `dug_cells` also holds every
/// pellet an ant cut back out of a spoil heap, and a heap stands on the
/// surface, above the ground as built; the soil score's hole
/// (`train::challenges::soil_hole`) counts only cuts of ground that was
/// there at the start, which is the same set by this test. With no datum
/// (no box) every cut counts.
pub(crate) fn in_the_hole(world: &World, x: i32, y: i32) -> bool {
    world.dug_cells.contains(&(x, y)) && world.room_surface_at(x).is_none_or(|top| y >= top)
}

/// The animal standing in a cell, if any: a plant is an organism too.
fn animal(world: &World, id: OrganismId) -> Option<&crate::sim::organism::OrganismState> {
    (id != 0)
        .then(|| world.organism(id))
        .flatten()
        .filter(|o| world.species.get(o.species).creature.is_some())
}

/// The features every option reads at its target, verb-specific ones (the
/// walk's footing) left for the caller to set.
type Sampler<'w> = crate::sim::field::BilinearSampler<'w, crate::sim::fxhash::ChunkMap<crate::sim::field::FieldTile>>;

fn describe(world: &World, memo: &mut impl CellRead, warm: Option<&mut Sampler<'_>>, here: &Here, (x, y): (i32, i32)) -> [f32; FEATURES] {
    let mut v = [0.0f32; FEATURES];
    let cell = memo.cell(x, y);
    let kind = world.materials.kind(cell.material);
    let m = world.materials.get(cell.material);
    let who = animal(world, cell.organism_id());
    let class = if who.is_some_and(|o| o.brood.is_some()) {
        f::BROOD
    } else if who.is_some() {
        f::ANT
    } else if cell.material == material::EMPTY || kind == MaterialKind::Gas {
        f::AIR
    } else if here.mats.corpse.is_some_and(|c| cell.material == c) {
        f::CORPSE
    } else if m.food_energy > 0.0 {
        f::FOOD
    } else if here.mats.spoil.is_some_and(|s| cell.material == s) {
        f::SPOIL
    } else if here.mats.packed.is_some_and(|p| cell.material == p) {
        f::LINED
    } else if kind == MaterialKind::Liquid {
        f::WATER
    } else if kind == MaterialKind::Powder {
        f::SOIL
    } else {
        f::STONE
    };
    v[class] = 1.0;
    v[f::CUT] = f32::from(u8::from(in_the_hole(world, x, y)));

    let (dx, dy) = ((x - here.head.0).signum(), (y - here.head.1).signum());
    for (c, ch) in CHANNELS.into_iter().enumerate() {
        let s = scent(world, ch, x, y);
        v[f::SCENT + 3 * c] = s;
        v[f::SCENT + 3 * c + 1] = squash(SCENT_GAIN * (s - here.scent[c]));
        v[f::SCENT + 3 * c + 2] = squash(SCENT_GAIN * (scent(world, ch, x + 2 * dx, y + 2 * dy) - here.scent[c]));
    }

    // Footing for a non-walk target: something filled under it.
    let below = memo.cell(x, y + 1).material != material::EMPTY;
    v[f::STAND] = f32::from(u8::from(below));
    if kind == MaterialKind::Powder || kind == MaterialKind::Solid {
        v[f::HARD] = squash(m.penetration_resistance);
    }
    // Wetness by level: a powder's own water, else what it stands on.
    let wet_of = |c: crate::sim::Cell| {
        (world.materials.kind(c.material) == MaterialKind::Powder)
            .then(|| f32::from(c.aux()) / f32::from(material::SOIL_SATURATED))
    };
    v[f::WET] = wet_of(cell)
        .or_else(|| wet_of(memo.cell(x, y + 1)))
        .unwrap_or(0.0)
        .min(1.0);

    v[f::UP] = (here.head.1 - y).signum() as f32;
    v[f::DEPTH] = super::mind::depth_at(world, x, y);
    v[f::DROP_BELOW] = (1..=REACH)
        .take_while(|&d| memo.cell(x, y + d).material == material::EMPTY)
        .count() as f32
        / REACH as f32;
    v[f::HEADROOM] = (1..=REACH)
        .take_while(|&d| memo.cell(x, y - d).material == material::EMPTY)
        .count() as f32
        / REACH as f32;
    // Remembering each cell's cover answer in the memo, so targets sharing
    // a column skip the repeat, measured no better (mind colony 6.963G ->
    // 6.979G, callgrind, 2026-10-22): the scan stops at the first ground
    // overhead, and an empty sky row is one compare.
    v[f::ROOFED] = f32::from(u8::from(under_cover_by(world, memo, x, y)));

    if let Some(o) = who {
        let mine = o.colony == here.colony;
        v[f::NESTMATE] = f32::from(u8::from(mine));
        v[f::STRANGER] = f32::from(u8::from(!mine));
        v[f::LADEN] = f32::from(u8::from(o.crop.is_some() || o.spoil.is_some()));
        let start = world
            .species
            .get(o.species)
            .creature
            .as_ref()
            .map_or(1.0, |d| d.start_energy)
            .max(1.0);
        let hunger = (1.0 - o.energy / start).clamp(0.0, 1.0);
        if o.brood.is_some() {
            v[f::LARVA] = 1.0;
            v[f::LARVA_HUNGER] = hunger;
        } else {
            // SCRATCH (sense-cap audit, 2026-10-24; off by default): `PP_HUNGRY_GRADED=1` reads a nestmate's hunger
            // as the ant reads its own bank -- `1 / (1 + e)`, `e` its bank in the species' sensed tanks (`c::ENERGY`
            // is `e / (1 + e)`) -- instead of "under half a start grant", which is 1 or 0 with nothing between and 0
            // for any nestmate above one grant. On the training bed it fired on no option at all (every nestmate
            // banked 7-60 tanks); in a lab box on 2.3-12% of options.
            v[f::HUNGRY] = if hungry_graded() {
                let tank = world.species.get(o.species).creature.as_ref().map_or(1.0, |d| d.sensed_tank.unwrap_or(d.start_energy)).max(1.0);
                1.0 / (1.0 + o.energy.max(0.0) / tank)
            } else {
                f32::from(u8::from(hunger > 0.5))
            };
        }
    }
    v[f::DEAD] = f32::from(u8::from(here.mats.corpse.is_some_and(|c| cell.material == c)));

    let (mut animals, mut open, mut near) = (0, 0, 0);
    for ny in -2..=2 {
        for nx in -2..=2 {
            if nx == 0 && ny == 0 {
                continue;
            }
            let c = memo.cell(x + nx, y + ny);
            let animal = c.organism_id() != 0;
            if animal && nx.abs() <= 1 && ny.abs() <= 1 {
                animals += 1;
            }
            near += i32::from(animal);
            open += i32::from(c.material == material::EMPTY);
        }
    }
    v[f::CROWD] = animals as f32 / 8.0;
    v[f::ROOM] = open as f32 / 24.0 / (1 + near) as f32;

    // How much this option points home: the cosine between the step and
    // the home vector, times how far home is, squashed. 0 at home, or blind.
    if let Some((ax, ay)) = here.home {
        let (hx, hy) = (ax - here.head.0, ay - here.head.1);
        let (sx, sy) = (x - here.head.0, y - here.head.1);
        let (h2, s2) = ((hx * hx + hy * hy) as f32, (sx * sx + sy * sy) as f32);
        if h2 > 0.0 && s2 > 0.0 {
            let cos = (hx * sx + hy * sy) as f32 / (h2.sqrt() * s2.sqrt());
            v[f::HOME] = cos * squash(h2.sqrt() / 32.0);
            // SCRATCH ([`home_vec`]): the same cosine, times how far past the food band home is, on the unwritten
            // `f::LAST_FOOD` slot. Exactly 0 inside `R` of home, so a weight on it cannot move a choice there.
            if let Some((r, w)) = home_vec() {
                v[f::LAST_FOOD] = cos * far_ramp(h2.sqrt(), r, w);
            }
            // SCRATCH ([`home_dir`]): the cosine alone, at every distance, on the unwritten `f::BUILDING` slot.
            if home_dir() {
                v[f::BUILDING] = cos;
            }
        }
    }
    // The way back to the last food ([`last_food`]), on the same terms as
    // home's: 0 standing on the spot, or before any pick-up.
    if let Some(((fx, fy), fresh)) = here.food {
        let (gx, gy) = (fx - here.head.0, fy - here.head.1);
        let (sx, sy) = (x - here.head.0, y - here.head.1);
        let (g2, s2) = ((gx * gx + gy * gy) as f32, (sx * sx + sy * sy) as f32);
        if g2 > 0.0 && s2 > 0.0 {
            let cos = (gx * sx + gy * sy) as f32 / (g2.sqrt() * s2.sqrt());
            v[f::FOOD_WAY] = cos * far_ramp(g2.sqrt(), f32::NAN, LAST_FOOD_SCALE) * fresh;
        }
    }
    if warmth_on() {
        let warmth = match warm {
            Some(warm) => warm.temperature_at(x as f32, y as f32),
            None => world.field_at_bilinear(x as f32, y as f32).temperature,
        };
        v[f::WARMTH] = squash(warmth - here.warmth);
    }
    v[f::BACK] = f32::from(u8::from(here.neck == Some((x, y))));
    v
}

thread_local! {
    static SCRATCH: RefCell<Vec<Opt>> = const { RefCell::new(Vec::new()) };
}

/// **The shadow**: build `organism`'s options and throw them away, counting
/// what was built ([`World::creature_stats`]: decisions, options per verb,
/// and a sum of every feature so nothing is optimised out). Called once per
/// decision from `creature_tick` when [`World::option_shadow`] is set.
///
/// [`World::creature_stats`]: crate::sim::World::creature_stats
/// [`World::option_shadow`]: crate::sim::World::option_shadow
pub fn shadow(world: &mut World, organism: OrganismId, def: &CreatureDef) {
    SCRATCH.with(|s| {
        let mut opts = s.borrow_mut();
        build(world, organism, def, &mut opts);
        let st = &mut world.creature_stats;
        st.option_decisions += 1;
        st.options_built += opts.len() as u64;
        for o in opts.iter() {
            st.options_by_verb[o.verb as usize] += 1;
            st.option_feature_sum += o.feats.iter().map(|&v| f64::from(v)).sum::<f64>();
        }
    });
}

/// SCRATCH (crater review, 2026-10-23; off unless set): `PP_BODYRULES=roof|rooms|both` offers a mind ant only
/// the cuts the shipped ant's own dig refusals allow: `roof` = `dig_roof_of` / `under_roof` (no cut within 6 rows
/// under the founding surface outside the door's columns), `rooms` = `shaft_room_refuses` (the room geometry stopgap).
/// No waiver for a shut-in ant (the shipped ant waives all but Floor when escaping).
/// **Never a default and never permanent** (owner, 2026-10-23: "No permanent body rule from the old ant!"): a training
/// scaffold only, weaned to nothing by `scratch/inrun/colsel.py wean=R0:R1`, and every ant is judged with it off.
fn review_body_rules_refuse(world: &World, t: (i32, i32)) -> bool {
    static V: std::sync::OnceLock<(bool, bool)> = std::sync::OnceLock::new();
    let (roof, rooms) = *V.get_or_init(|| match std::env::var("PP_BODYRULES").as_deref() {
        Ok("roof") => (true, false),
        Ok("rooms") => (false, true),
        Ok("both") => (true, true),
        _ => (false, false),
    });
    // PP_BODYRULES_P=p (Teacher lane, 2026-10-23; owner: "No permanent body rule from the old ant!"): a scaffold that
    // is weaned. Each ground cell is protected with probability p, drawn once per cell and world (a pure hash), so at
    // p < 1 a fraction of the roof and room cells are free to cut and the brain has to spare the rest itself. Unset = 1.
    static P: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    let p = *P.get_or_init(|| std::env::var("PP_BODYRULES_P").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0));
    if (roof || rooms) && p < 1.0 && super::rng::stream(world.seed, t.0 as u64, t.1 as u64, 0xB0D7_0001).unit_f32() >= p {
        return false;
    }
    (roof && super::dig_roof_of(world).is_some_and(|rows| super::under_roof(world, t, rows)))
        || (rooms && super::shaft_room_refuses(world, t).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::creature::{targets, usable_headings};
    use crate::sim::Cell;

    /// `PP_HOMEVEC=log:S` is graded from the door and never caps, where the ramp reads 1 at every distance past
    /// `R + W` -- the property the log form exists for (70 and 150 columns must read differently).
    #[test]
    fn the_log_far_sense_keeps_rising_where_the_ramp_caps() {
        assert_eq!(far_ramp(70.0, 40.0, 30.0), far_ramp(150.0, 40.0, 30.0));
        assert_eq!(far_ramp(0.0, f32::NAN, 40.0), 0.0);
        let (a, b, c) = (far_ramp(25.0, f32::NAN, 40.0), far_ramp(70.0, f32::NAN, 40.0), far_ramp(150.0, f32::NAN, 40.0));
        assert!(0.0 < a && a < b && b < c, "{a} {b} {c}");
        assert!((far_ramp(100.0, f32::NAN, 40.0) - 3.5f32.ln()).abs() < 1e-6);
    }

    fn colony_box() -> World {
        crate::lab::scene::LabBox {
            founders: 4,
            colonies: 1,
            colony_ants: 12,
            width: 256,
            height: 128,
            ground_y: 64,
            soil_depth: 40,
            ..crate::lab::scene::LabBox::default()
        }
        .build()
    }

    fn step(w: &mut World) {
        crate::sim::update::step(w);
        w.step_active_sites();
        w.step_fields();
        w.step_pheromones();
    }

    /// **The option list's cell window changes no feature** (`CellMemo` in
    /// [`build_with`]): every live ant of a digging colony, at several
    /// points of its run, gets the same options with the same features, bit
    /// for bit, as the list built reading every cell straight from the
    /// world. The count is the control that it compared real lists.
    #[test]
    fn the_option_window_changes_no_feature() {
        let mut w = colony_box();
        let (mut compared, mut shared) = (0, 0);
        for round in 0..6 {
            for _ in 0..500 {
                step(&mut w);
            }
            for id in w.live_organism_ids() {
                let Some(st) = w.organism(id) else { continue };
                let Some(def) = w.species.get(st.species).creature.clone() else { continue };
                let (mut a, mut b) = (Vec::new(), Vec::new());
                build_remembering(&w, id, &def, true, &mut a, true);
                build_remembering(&w, id, &def, true, &mut b, false);
                assert_eq!(a.len(), b.len(), "round {round}, ant {id}");
                for (i, (x, y)) in a.iter().zip(&b).enumerate() {
                    assert_eq!((x.verb, x.target), (y.verb, y.target), "round {round}, ant {id}");
                    // A target an earlier option shares: its features came
                    // from the remembered description, not a fresh one.
                    shared += usize::from(a[..i].iter().any(|o| o.target == x.target));
                    let same = x.feats.iter().zip(&y.feats).all(|(p, q)| p.to_bits() == q.to_bits());
                    assert!(same, "round {round}, ant {id}, {:?} at {:?}: {:?} vs {:?}", x.verb, x.target, x.feats, y.feats);
                    compared += 1;
                }
            }
        }
        assert!(compared > 400 && shared > 50, "only {compared} options compared, {shared} at a shared target");
    }

    /// **The shadow leaves the game unchanged**: one box run twice from the
    /// same seed, the shadow on in one, is the same world cell for cell and
    /// ant for ant after 3,000 frames -- and the shadow did fire, at every
    /// decision of the run, with every verb a digging colony meets. Watched
    /// red with one draw from the world's stream added to `shadow`.
    #[test]
    fn the_shadow_leaves_the_game_unchanged() {
        let (mut off, mut on) = (colony_box(), colony_box());
        on.option_shadow = true;
        for _ in 0..3000 {
            step(&mut off);
            step(&mut on);
        }
        let b = off.bounds().expect("a lab box is bounded");
        for y in b.min_y..b.max_y {
            for x in b.min_x..b.max_x {
                let (a, b) = (off.get(x, y), on.get(x, y));
                assert!(
                    a.material == b.material && a.aux() == b.aux() && a.organism_id() == b.organism_id(),
                    "({x},{y}) differs: {a:?} against {b:?}"
                );
            }
        }
        let (ids_off, ids_on) = (off.live_organism_ids(), on.live_organism_ids());
        assert_eq!(ids_off, ids_on, "different organisms alive");
        for id in ids_off {
            let (a, b) = (off.organism(id).expect("live"), on.organism(id).expect("live"));
            assert_eq!(
                (&a.chain, a.heading, a.energy.to_bits()),
                (&b.chain, b.heading, b.energy.to_bits()),
                "organism {id} differs"
            );
        }
        let st = on.creature_stats;
        assert_eq!(
            off.creature_stats.option_decisions, 0,
            "the shadow ran with the switch off"
        );
        assert!(
            st.option_decisions > 1000,
            "only {} decisions built",
            st.option_decisions
        );
        assert_eq!(
            st.options_by_verb[Verb::Rest as usize],
            st.option_decisions,
            "rest is offered at every decision"
        );
        for v in [Verb::Walk, Verb::TurnAround, Verb::Dig, Verb::Drop] {
            assert!(
                st.options_by_verb[v as usize] > 0,
                "a digging colony never had a {} option",
                v.name()
            );
        }
        let per = st.options_built as f64 / st.option_decisions as f64;
        assert!((3.0..40.0).contains(&per), "{per:.1} options a decision");
    }

    /// **An option's hole is the soil score's hole** (`f::CUT`,
    /// [`in_the_hole`]): over 3,000 frames of a digging colony, every option
    /// at every 25th frame reads `f::CUT` exactly when its cell was open in
    /// the founding cut at frame 0, or was ground at frame 0 and has been cut
    /// since -- `train::challenges::soil_hole`'s definition, rebuilt here from
    /// the cut log and a frame-0 snapshot rather than from the code under
    /// test. And the colony's own wall reads as wall, not as a heap.
    #[test]
    fn the_hole_and_the_wall_read_as_the_soil_score_reads_them() {
        let mut w = colony_box();
        w.cut_log = Some(Vec::new());
        let sp = w.species.id_of("ant").expect("ant");
        let def = w.species.get(sp).creature.clone().expect("creature");
        let packed = w.materials.id_of("packedsoil").expect("packedsoil");
        let b = w.bounds().expect("a box");
        let mut ground0 = std::collections::HashSet::new();
        let mut founding0 = std::collections::HashSet::new();
        for x in b.min_x..=b.max_x {
            let Some(top) = w.room_surface_at(x) else { continue };
            for y in top..=b.max_y {
                let c = w.get(x, y);
                if c.material == material::EMPTY || w.materials.kind(c.material) == MaterialKind::Creature {
                    founding0.insert((x, y));
                } else {
                    ground0.insert((x, y));
                }
            }
        }
        assert!(founding0.len() > 10, "a founding cut of {} cells", founding0.len());
        let mut opts = Vec::new();
        let (mut hole, mut wall, mut n) = (0, 0, 0);
        for f in 0..3000 {
            step(&mut w);
            if f % 25 != 0 {
                continue;
            }
            let cut: std::collections::HashSet<(i32, i32)> = w
                .cut_log
                .iter()
                .flatten()
                .map(|c| (c.0, c.1))
                .filter(|c| ground0.contains(c))
                .collect();
            for id in w.live_organism_ids() {
                if !w.organism(id).is_some_and(|s| s.species == sp && s.brood.is_none()) {
                    continue;
                }
                build(&w, id, &def, &mut opts);
                for o in &opts {
                    let want = founding0.contains(&o.target) || cut.contains(&o.target);
                    assert_eq!(
                        o.feats[f::CUT] == 1.0,
                        want,
                        "frame {f} ant {id}: {:?} at {:?}",
                        o.verb,
                        o.target
                    );
                    hole += usize::from(want);
                    let lined = w.get(o.target.0, o.target.1).material == packed;
                    assert_eq!(o.feats[f::LINED] == 1.0, lined, "frame {f}: wall at {:?}", o.target);
                    assert!(
                        !(lined && o.feats[f::SPOIL] == 1.0),
                        "frame {f}: wall read as spoil at {:?}",
                        o.target
                    );
                    wall += usize::from(lined);
                    n += 1;
                }
            }
        }
        assert!(
            hole > 100 && wall > 100,
            "{hole} hole options and {wall} wall options of {n}"
        );
        assert!(
            w.cut_log.as_ref().is_some_and(|l| l.len() > 20),
            "the colony dug {:?}",
            w.cut_log.as_ref().map(Vec::len)
        );
        // A pellet cut back out of a heap is not the hole: the colony above
        // never did that, so put the case in by hand -- a cut a row above
        // the ground as built, and one a row below it.
        let x = b.min_x + 20;
        let top = w.room_surface_at(x).expect("a datum");
        w.dug_cells.insert((x, top - 1));
        w.dug_cells.insert((x, top + 1));
        assert!(!in_the_hole(&w, x, top - 1), "a cut out of a heap above the ground");
        assert!(in_the_hole(&w, x, top + 1), "a cut into the ground");
    }

    /// **The options are the body's own**: at every decision sampled from a
    /// working colony, rest is offered once at the head; the footed steps
    /// are exactly the walk's own (`usable_headings`); the digs are exactly
    /// the cells the jaws can cut (`targets::dig_candidates`) and only with
    /// empty jaws and crop; a load goes down only from a carrier and only
    /// on real ground (`spoil_site_open` with the footing rule: the minds'
    /// own, stricter than the game's spoil branch, whose footing switch
    /// ships off); one colony has nobody to bite;
    /// each target is one kind of thing; every feature is in -1..1. Then food
    /// is put beside a head, and a pick-up and a meal appear on it: the
    /// positive control for the reach scan.
    #[test]
    fn the_options_are_the_bodys_own() {
        let mut w = colony_box();
        let sp = w.species.id_of("ant").expect("ant");
        let def = w.species.get(sp).creature.clone().expect("creature");
        let corpse = w.materials.id_of("corpse");
        let mut opts = Vec::new();
        let (mut decisions, mut carrying, mut crossable) = (0, 0, 0);
        for f in 0..3000 {
            step(&mut w);
            if f % 25 != 0 {
                continue;
            }
            for id in w.live_organism_ids() {
                let Some(st) = w.organism(id).filter(|s| s.species == sp && s.brood.is_none()) else {
                    continue;
                };
                let Some(&head) = st.chain.first() else { continue };
                let (jaws, crop) = (st.spoil.is_some(), st.crop.is_some());
                build(&w, id, &def, &mut opts);
                decisions += 1;
                carrying += usize::from(jaws || crop);
                let of =
                    |v: Verb| -> Vec<(i32, i32)> { opts.iter().filter(|o| o.verb == v).map(|o| o.target).collect() };
                assert_eq!(of(Verb::Rest), vec![head], "frame {f} ant {id}");
                // A Walk into living wood is the trunk crossing (`cross_on`), which the game's walk takes only when
                // blocked, so it is held apart from the steps: each one must be a crossing the body could make.
                let wood = |t: &(i32, i32)| {
                    let c = w.get(t.0, t.1);
                    super::super::is_living_tissue(&w, c) && w.materials.get(c.material).woody
                };
                type Cells = Vec<(i32, i32)>;
                let (crossings, footed): (Cells, Cells) = opts
                    .iter()
                    .filter(|o| o.verb == Verb::Walk && o.feats[f::STAND] == 1.0)
                    .map(|o| o.target)
                    .partition(wood);
                crossable += crossings.len();
                for t in &crossings {
                    let d = DIRS.iter().position(|&(dx, dy)| (head.0 + dx, head.1 + dy) == *t).expect("a step") as u8;
                    let authored = segment_authored_widths(&def, st.fates);
                    let body = BodyShape {
                        chain: &st.chain,
                        groups: &st.segment_groups,
                        authored: &authored,
                    };
                    assert!(cross_on(), "frame {f} ant {id}: a walk into wood with the crossing off");
                    assert!(
                        super::super::trunk_crossing(&w, &def, body, d, parting_enabled(), stacker_of(&w, id)).is_some(),
                        "frame {f} ant {id}: a walk into wood at {t:?} with no far side"
                    );
                }
                let usable: Vec<(i32, i32)> = usable_headings(&w, id, &def)
                    .iter()
                    .map(|&d| (head.0 + DIRS[d as usize].0, head.1 + DIRS[d as usize].1))
                    .collect();
                assert_eq!(
                    footed, usable,
                    "frame {f} ant {id}: footed steps against the walk's own"
                );
                // The body's own cut, less food and dead ants, with free
                // jaws: a full crop no longer stops it (fix plan step 1).
                // With soil in the jaws, only a walled-in ant cuts: the pack move.
                let walled = jaws && pack_on() && st.spoil.is_some_and(|s| !s.store) && super::super::walled_in(&w, id, head);
                let digs: Vec<(i32, i32)> = if jaws && !walled {
                    Vec::new()
                } else {
                    targets::dig_candidates(&w, &def, id, head)
                        .into_iter()
                        .filter(|t| w.materials.get(w.get(t.0, t.1).material).food_energy <= 0.0)
                        .collect()
                };
                assert_eq!(of(Verb::Dig), digs, "frame {f} ant {id}: digs");
                for t in of(Verb::PickUp) {
                    let cell = w.get(t.0, t.1);
                    assert!(!jaws, "frame {f} ant {id}: a pick-up with soil in the jaws");
                    assert!(
                        w.materials.get(cell.material).food_energy > 0.0 && Some(cell.material) != corpse,
                        "frame {f} ant {id}: a pick-up of {} at {t:?}",
                        w.materials.get(cell.material).name
                    );
                }
                assert!(of(Verb::Eat).iter().all(|&t| t == head) && (of(Verb::Eat).len() == usize::from(crop)), "frame {f} ant {id}: eat is the crop's own");
                for t in of(Verb::Drop) {
                    assert!(jaws || crop, "frame {f} ant {id}: a drop with nothing to drop");
                    if jaws {
                        assert!(
                            spoil_site_open(&w, t.0, t.1, true),
                            "frame {f} ant {id}: pellet down at {t:?}"
                        );
                    } else {
                        assert!(w.is_empty(t.0, t.1), "frame {f} ant {id}: crop cell down at {t:?}");
                    }
                }
                assert!(
                    of(Verb::Attack).is_empty(),
                    "frame {f} ant {id}: a stranger in a one-colony box"
                );
                for o in &opts {
                    assert_eq!(
                        f::KINDS.iter().filter(|&&k| o.feats[k] == 1.0).count(),
                        1,
                        "frame {f} ant {id}: {:?} at {:?} is not one kind of thing",
                        o.verb,
                        o.target
                    );
                    for (k, &v) in o.feats.iter().enumerate() {
                        // The graded far sense (`PP_HOMEVEC=log:S`, the default since farfood3b) is uncapped by
                        // the owner's choice, so `LAST_FOOD` = cosine x ln(1 + d / S) passes 1 past ~70 columns
                        // (1.25 at 100); 4.6 at 4,096 columns, so 6 bounds any world.
                        let lim = if k == f::LAST_FOOD { 6.0 } else { 1.0 };
                        assert!(
                            v.is_finite() && (-lim..=lim).contains(&v),
                            "frame {f} ant {id}: {} = {v} on {:?}",
                            feature_names()[k],
                            o.verb
                        );
                    }
                }
            }
        }
        assert!(
            decisions > 1000 && carrying > 0 && crossable > 0,
            "{decisions} decisions, {carrying} carrying, {crossable} trunk crossings offered"
        );

        // The control: food beside a free-jawed head -- on whichever side
        // of it is open. **Either side, not both**, since 2026-10-21: the
        // food goes on one cell, and asking for both open made the control
        // a bet on where twelve ants happen to stand at frame 3,000. Soil
        // water passing through roots (`LabBox::soil_through_roots`, on in
        // this box) reshuffled the colony and lost that bet with nothing
        // about the options changed -- every per-decision assertion above
        // still held.
        let food = w.materials.id_of("provisions").expect("provisions");
        let (id, t) = w
            .live_organism_ids()
            .into_iter()
            .filter_map(|id| {
                w.organism(id)
                    .filter(|s| s.species == sp && s.brood.is_none() && s.spoil.is_none() && s.crop.is_none())
                    .and_then(|s| s.chain.first().map(|&h| (id, h)))
            })
            .find_map(|(id, (x, y))| {
                [(x + 1, y), (x - 1, y)].into_iter().find(|&(tx, ty)| w.get(tx, ty).material == material::EMPTY).map(|t| (id, t))
            })
            .expect("a free-jawed, empty-cropped ant with open ground beside its head");
        build(&w, id, &def, &mut opts);
        assert!(
            !opts.iter().any(|o| o.target == t && o.verb == Verb::PickUp),
            "food offered before it was put down"
        );
        w.set(t.0, t.1, Cell::new(food, 0));
        build(&w, id, &def, &mut opts);
        let o = opts
            .iter()
            .find(|o| o.target == t && o.verb == Verb::PickUp)
            .expect("no pick-up on the food");
        assert_eq!(o.feats[f::FOOD], 1.0, "the food reads as food");
        // One verb each (fix plan step 1): the food is picked up, never
        // eaten where it lies nor cut into the jaws.
        for v in [Verb::Eat, Verb::Dig] {
            assert!(
                !opts.iter().any(|o| o.target == t && o.verb == v),
                "{} offered on food",
                v.name()
            );
        }
    }
}
