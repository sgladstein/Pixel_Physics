//! **The keeper seat's demo** (`demo=keeper`): the player is a hand over the
//! formicarium, not an ant. The design is
//! `Reports/teaching-the-colony-2026-10-25/keeper.md`; the plan this file is
//! built to is the keeper thread's `plan-v2` (landing with
//! `Reports/keeper-seat-demo-2026-10-25.md`).
//!
//! **What the box holds** (`assets/lab_scenarios/demo_keeper.ron` plus this
//! file):
//!
//! - the colony at the west end, landed at the first tick, with a founding
//!   heap under it;
//! - a **far station** of provisions near the east end, kept topped up so it
//!   never runs dry ([`STATION_LEVEL`]), the place the player teaches;
//! - a **ration inside the nest** ([`KeeperDemo::ration`] cells per 1,000
//!   frames, onto dug home cells): enough that the colony is not starved out
//!   before the player has done anything, below what it eats, so foraging
//!   pressure stays. A pick-up at home never writes `last_food`
//!   (`mind::pick_up`), so the ration cannot overwrite a lesson.
//!
//! **The recall head** (`creature::recall`) is set on the world here, after
//! every rebuild, at the demo's gain and copy; [`KeeperDemo::listening`]
//! switches the gain to 0, the honest arm, without touching anything else.
//!
//! The counters are the plan's: station bites, station deliveries (a cell
//! taken at the station and put down at home, `trip_deliveries` -- never
//! `MindCounts::delivered`, how-we-test §4), the frame of the first one, and
//! how many ants know the station.
//!
//! **The player's verbs** are the seat keys (`Y U J K`, the launcher's), so
//! nothing in the lab's own bar or key map moves:
//!
//! - `Y` **CARRY**: press near an ant (it snaps to the nearest liftable one
//!   within `PICK_R`), drag -- the ant rides the pointer -- and release to
//!   set it down (`creature::set_down`). Set down within [`LESSON_R`] of
//!   food that is not at home, it remembers that food as a lesson
//!   (`recall::plant`). Under stakes it spends a charge and tires the ant;
//! - `U` **TREAT**: click an ant to put a mouthful of provisions in its crop
//!   (`creature::treat`, booked as granted food). Away from home it also
//!   remembers the place it was treated. Under stakes it is bought from the
//!   pantry;
//! - `J` **RINGS**: off, who knows a taught place (a ring per taught or fed
//!   ant, by generation), or traffic (where heads have been lately);
//! - `K` **CARD**: the verdict card, the round so far (it comes up by
//!   itself when the round ends).
//!
//! `P` (listening) was a key until 10-30; it is swallowed now, and
//! `listening=0` on the command line is the honest arm. **On the hand-built
//! ant** (`-- oldant`) the recall head is inert -- only a mind reads
//! `last_food` (`options::f::FOOD_WAY`) -- so a carry there teaches nothing
//! remembered: it puts a forager on the food, which finds it, eats and lays
//! its trail home. The verb still works (see the measurements below).
//!
//! **What the measurement says this can and cannot show** (build step 4,
//! `examples/keeperdemo.rs`, 12 paired seeds, the 24,000 frames after the
//! demo opens, the pool of PR #906):
//!
//! - **Carrying is a real verb on today's ant, graded by how many you
//!   carry.** Listening *off*, against nobody carried: station visits +52
//!   (10 carried, 10 of 12 seeds up) and +105 (~17 carried, 10 of 12), food
//!   home +15 (9 of 12), and a bigger colony (median 120 ants, 146, 168).
//!   Three carried ants are inside the noise. A carried ant takes food at the
//!   station, walks it home, and walks back on its own trail.
//! - **The scripted listening adds to that**, unevenly: visits +36 (3
//!   carried, 9 of 12 up), +12 (10, 6 of 12), +89 (~17, 9 of 12); food home
//!   +4 to +14. No loss in digging or survival in any arm.
//! - **The meal copy moves nothing measurable**: copy 0 and copy 0.5 gave
//!   the same traffic, and the share that "knows" never rises past the ants
//!   taught by hand, because a fed place is weak and the next stray bite puts
//!   it to sleep. It stays on (neutral), counted on the HUD.
//!
//! An earlier pass read "carrying alone changes nothing": its scripted hand
//! asked for 10 ants and lifted about 6, because the nearest ants to the nest
//! were stacked on one another and [`creature::set_down`] refuses a stack.
//! The hand now picks the nearest ant it can lift (`creature::can_carry`).
//!
//! ## Stakes: windfalls, a limited hand, a round and a verdict (2026-10-30)
//!
//! The 10-30 review (`/mnt/project-files/seat-demos/review-2026-10-30/
//! keeper/review.md`) found no loop: one burst of carries in the first
//! minute, then watching a colony that thrived alone on a free station, with
//! a hidden price (13 ants carried left the colony +42, -24 and -35 ants
//! against the same seed idle, and the HUD never said so). **On by default;
//! `stakes=0` is the sandbox above, unchanged**, for the paired comparison.
//!
//! - **Windfalls** ([`Patch`]): no station and no top-up. A heap of
//!   provisions lands at a random far column every [`Rules::patch_every`]
//!   frames (the first as the demo opens) and **rots** [`Rules::patch_life`]
//!   frames later: what is left turns to ash, a grey heap the ants cannot eat
//!   -- the mark a patch nobody reached leaves behind. The column sequence
//!   comes from the box's seed, so the same seed drops the same windfalls.
//! - **The nest ration is cut** to [`Rules::ration`] cells a 1,000 frames
//!   once the round opens (the warm-up keeps the sandbox ration, so the
//!   opening box is the sandbox's).
//! - **The hand has charges** ([`Rules::charges`], one back every
//!   [`Rules::recharge`] frames): a carry spends one. A carried ant is also
//!   a forager or digger taken off its route, which the paired runs show is
//!   a real price.
//! - **Treats are bought from the pantry**: every windfall cell the colony
//!   saves puts one in; a treat costs [`Rules::treat_cost`].
//! - **Score: food saved**, the windfall cells ants took before they
//!   rotted (eaten there or carried off): cells gone from a windfall, but
//!   never more than the bites ants standing at it took ([`Patch::taken`];
//!   until 10-31 any cell gone counted, so an erased windfall read
//!   "STRIPPED -- ALL 60 CELLS SAVED"; on idle runs the two agree to within
//!   a few cells, 274 against 274 median). **Not cells home** (`trip_deliveries`), which the brief
//!   asked for and which is still counted and shown beside it: measured on
//!   4 paired seeds, a good keeper stripped 7 of 8 windfalls against idle's
//!   1 and still brought *fewer* cells home (19 against 24, seed 1), because
//!   a taught ant eats at the windfall; and the hand-built ant (`-- oldant`)
//!   books almost no `trip_deliveries` at all (0 on every idle seed), so a
//!   score on it would read 0 on the ant the fun hunt plays.
//! - **The lift tires the ant** ([`Rules::lift_cost`], a share of its
//!   energy): set down on food it eats and recovers; set down on empty
//!   ground it may not make it home. Without it, carrying ants out to bare
//!   ground *helped* (careless above idle in 3 of 4 seeds): the hand moved
//!   foragers into the field for free.
//! - **The round** is [`Rules::round`] frames from the opening; **it is lost**
//!   if the colony falls under [`Rules::lose_at`] ants. Either way the
//!   verdict card comes up and the clock stops.
//! - **The verdict** compares the colony with **the same seed left alone**: a
//!   twin box built and run on a background thread by the window
//!   ([`spawn_twin`]), from the same scenario and seed, with nobody's hand on
//!   it. Its opening is fingerprinted against the player's ([`fingerprint`]),
//!   and a twin that does not match is labelled as such on the card rather
//!   than trusted.
//! - **Graded on food saved against a band of three twins** ([`grade`],
//!   2026-10-31): the exact twin and two whose dice were knocked for a
//!   second at the opening ([`perturb`]). The grade had been ants at the end
//!   against the one exact twin, and the 10-31 review showed that a coin
//!   toss on the trained ant; this file's sweep agrees from the other side:
//!   a knocked idle box with no hand at all, graded that old way against the
//!   exact twin, was MASTER 4 and HEAVY 4 of 12. Ants are the consequence
//!   line on the card, never the grade. The band costs no second core: the
//!   three twins run one after another on the one twin thread, ~8.5 s each
//!   (0.30 ms a frame, 30,000 frames), so ~26 s of one core in a ~7 minute
//!   round at 1x, and the verdict waits for the last.
//! - **What the grade can and cannot tell apart** (trained ant, seeds 25-36,
//!   paired): good against careless 7 better, 0 worse, 5 the same (sign
//!   p = 0.016); against the random hand 4 better, 0 worse, 8 the same
//!   (p = 0.125): on today's trained ant a carry to *anywhere* in the far
//!   field puts a forager where windfalls fall, and food saved itself only
//!   separates aim from that 8 to 3 (p = 0.23). The verdict does not invent
//!   a gap the box does not have.
//! - **The hand rests when the round is over** (stakes on): carries and
//!   treats are refused on a box run on past its card.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::sim::cell::{Cell, OrganismId};
use crate::sim::creature::{self, recall};
use crate::sim::material;
use crate::sim::organism::DeathCause;
use crate::sim::world::World;

use super::super::Lab;

/// The keeper seat's own keys, as the help line prints them. **`P`
/// (listening) is not listed** (10-30 review: a developer A/B switch whose
/// "scripted" broke the fiction); `listening=0` on the command line is the
/// honest arm.
pub const SEAT_KEYS: &str = "Y CARRY, U TREAT, J RINGS, K CARD";

/// Food within this many cells of where an ant is set down (or treated) is
/// what it remembers. The station heap spreads ~10 columns either side.
pub const LESSON_R: i32 = 12;
/// A press picks the ant whose head is nearest, within this many cells.
/// **10, was 6** (10-30 review: 7 of 10 presses on the nest mouth missed a
/// 1-3 px ant). A press snaps to the nearest liftable ant this close.
const PICK_R: i32 = 10;
/// Frames the box runs before the player gets it (`warmup=`): the landing's
/// die-off (about 30 of 52 founders by frame 3,000, on every landing tried,
/// `nest_goal`'s own included) happens before the demo opens, and the nest
/// has a first chamber for the ration.
pub const DEFAULT_WARMUP: u64 = 6000;
/// The traffic overlay's memory: its counts halve every this many frames.
const HEAT_HALF: u64 = 2000;

/// Provisions the station is topped back up to, in cells.
pub const STATION_LEVEL: usize = 80;
/// Columns either side of the station counted as "at the station", for
/// bites and for "knows".
pub const STATION_R: i32 = 20;
/// "Knows the station" is strength x freshness at or above this (plan §5).
pub const KNOWS_PHI: f32 = 0.25;
/// Frames between the counters' scans: one mind decision interval.
const SCAN_EVERY: u64 = 6;
/// Frames between station top-ups.
const TOP_UP_EVERY: u64 = 60;

/// **Gain 6, copy 0.5**, from build step 4. Gain: the dose's knee on the unit
/// box (furthest columns toward a spot 60 out in 1,500 frames: g4 37, g6
/// 124, g8 113), and on the demo box 4, 6 and 8 were alike within the seed
/// spread with 6 the best median. Copy: 0, 0.25, 0.5 and 0.75 gave the same
/// station traffic (10 ants carried, 12 seeds); 0.5 is kept, on by default
/// as neutral, so the HUD has copies to count.
pub const DEFAULT_GAIN: f32 = 6.0;
pub const DEFAULT_COPY: f32 = 0.5;

/// Columns either side of a windfall's landing column counted as "at" it:
/// for its cells left, for "taken at a windfall", and for its rot. A
/// 60-cell heap piles ~8 columns either side and slumps a little wider.
pub const PATCH_R: i32 = 14;
/// Rows above and below a windfall's landing surface its cells are looked
/// for in (a heap stands ~8 rows; ants drop stray cells in their tunnels).
const PATCH_UP: i32 = 30;
const PATCH_DOWN: i32 = 12;
/// Windfalls land between these columns: far from the nest (column 107),
/// short of the east wall.
pub const PATCH_MIN_X: i32 = 220;
pub const PATCH_MAX_X: i32 = 490;
/// [`Rules::patch_life`] on the hand-built ant (`-- oldant`): see
/// [`on_start`].
pub const OLDANT_PATCH_LIFE: u64 = 2400;
/// The twin's read-out step, in frames.
const TWIN_EVERY: u64 = 100;
/// Idle twins in the verdict's band ([`TwinLog`]): run one after another
/// on the one twin thread, so the band costs time on that core, never a
/// second core -- see the module doc for the measured cost.
pub const DEFAULT_TWINS: u32 = 3;
/// Frames a windfall's end ("STRIPPED", "ROTTED") stays written over it.
const PATCH_NOTE: u64 = 1500;

/// **The round's rules under stakes**, each a demo argument of the same name
/// (`patch_every=3000` ...) so a sweep can move one without a rebuild.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rules {
    /// Frames between windfalls; the first lands as the demo opens.
    pub patch_every: u64,
    /// Frames a windfall lies before what is left of it rots.
    pub patch_life: u64,
    /// Provisions cells in a windfall.
    pub patch_cells: i32,
    /// The hand's charges (and how many it opens with).
    pub charges: u32,
    /// Frames for one charge to come back.
    pub recharge: u64,
    /// Frames from the opening to the verdict.
    pub round: u64,
    /// The round is lost when fewer ants than this are alive.
    pub lose_at: usize,
    /// Nest ration cells a 1,000 frames once the round opens (the warm-up
    /// keeps [`KeeperDemo::ration`]).
    pub ration: u32,
    /// Pantry cells a treat costs.
    pub treat_cost: u32,
    /// Share of a carried ant's energy the lift takes.
    pub lift_cost: f32,
}

impl Default for Rules {
    fn default() -> Self {
        Self { patch_every: 3000, patch_life: 4500, patch_cells: 60, charges: 3, recharge: 1000, round: 24_000, lose_at: 10, ration: 0, treat_cost: 10, lift_cost: 0.5 }
    }
}

/// How a windfall ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchEnd {
    /// The colony took every cell before it rotted.
    Stripped,
    /// It rotted with `left` cells uneaten (now ash).
    Rotted { left: u32 },
}

/// **One windfall**: where it landed, when it rots, and what the colony made
/// of it.
#[derive(Clone, Copy, Debug)]
pub struct Patch {
    pub x: i32,
    /// The surface it landed on.
    pub y: i32,
    pub landed: u64,
    pub rots: u64,
    /// Cells placed.
    pub size: u32,
    /// Cells still there at the last scan.
    pub left: u32,
    /// **Cells ants took** (bit off and swallowed or carried), the score:
    /// at each scan, the cells gone from the windfall's rectangle, but never
    /// more than the bites ants standing at it took since the last scan
    /// (`life.bites`, one per cell a mouth removes, on both brains). Cells
    /// that leave any other way -- erased, slumped out of the count -- are
    /// not saved (10-31 review: an erase arm scored "STRIPPED -- ALL 60
    /// CELLS SAVED" on windfalls no ant touched).
    pub taken: u32,
    /// Cells taken here and put down at home.
    pub home: u32,
    /// Ants the hand set down within reach of it.
    pub taught: u32,
    pub ended: Option<(u64, PatchEnd)>,
}

impl Patch {
    pub fn live(&self) -> bool {
        self.ended.is_none()
    }
}

/// Why the round ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    /// The clock ran out.
    Time,
    /// The colony fell under [`Rules::lose_at`] ants.
    Lost,
}

/// **The round's result, frozen when it ends**, so ticks that run on after
/// it (the box does not stop for a harness) cannot move the card.
#[derive(Clone, Copy, Debug)]
pub struct RoundEnd {
    pub frame: u64,
    pub why: Ending,
    pub ants: usize,
    pub saved: u64,
    pub home: u64,
}

/// One read-out of the idle twin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TwinPoint {
    pub frame: u64,
    pub ants: usize,
    pub saved: u64,
    pub home: u64,
}

/// **What the twin thread has written so far.** Shared with the thread
/// behind a mutex; read on the HUD, a lock a displayed frame.
///
/// **The twins are a band, not one run** (10-31 review): one thread runs
/// [`KeeperDemo::twins`] idle boxes one after another. The first is the
/// player's box exactly, left alone (its read-outs are the HUD's live
/// line); each after it is the same opening with the world's random stream
/// knocked once ([`perturb`]), so the band is what "nobody's hand" scatters
/// to on this seed.
#[derive(Debug, Default)]
pub struct TwinLog {
    /// The exact twin's [`fingerprint`] at its opening (every knocked twin's
    /// is checked against it).
    pub opening: Option<u64>,
    /// The exact twin's read-outs.
    pub points: Vec<TwinPoint>,
    /// The exact twin reached the round's end (or lost).
    pub exact_done: bool,
    /// Each twin's last read-out, the exact one first.
    pub finals: Vec<TwinPoint>,
    /// Every twin has finished.
    pub done: bool,
    /// The exact twin's own round ended this way, if it did.
    pub ended: Option<RoundEnd>,
    pub failed: Option<String>,
}

/// Which verb a press over the world is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tool {
    /// The lab's own tool takes the pointer.
    #[default]
    None,
    Carry,
    Treat,
}

/// What is drawn over the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Overlay {
    #[default]
    Off,
    Knows,
    Traffic,
}

/// What the demo has counted since the box was built.
#[derive(Clone, Copy, Debug, Default)]
pub struct KeeperCounts {
    /// Station food cells gone between scans (bites, and any slump off the
    /// counted columns, which the wide count keeps small).
    pub station_bites: u64,
    /// Cells taken at the station and put down at home.
    pub station_deliveries: u64,
    pub first_station_delivery: Option<u64>,
    /// Provisions cells put into the nest as the ration.
    pub ration_given: u64,
    /// Station cells added by the top-up.
    pub station_given: u64,
    pub carried: u64,
    pub treated: u64,
    /// An ant's head coming into the station's columns
    /// from outside them: a visit. `taught_arrivals` are the visits by ants
    /// the player taught or that were fed the place (a provenance record,
    /// awake or dormant).
    pub arrivals: u64,
    pub taught_arrivals: u64,
    /// **The score under stakes**: windfall cells taken at a windfall and
    /// put down at home.
    pub patch_home: u64,
    /// **The score under stakes**: windfall cells the colony took before
    /// they rotted, eaten there or carried off.
    pub saved: u64,
    pub patches: u64,
    pub stripped: u64,
    pub rotted: u64,
    /// Cells that rotted uneaten.
    pub rotted_cells: u64,
    /// Carries the hand set down within reach of a live windfall.
    pub carried_to_food: u64,
    /// Presses refused for want of a charge, or treats for want of pantry.
    pub refused: u64,
    /// The fewest ants alive at any scan since the opening.
    pub min_ants: usize,
}

/// The keeper seat's running state.
#[derive(Clone, Debug)]
pub struct KeeperDemo {
    pub gain: f32,
    pub copy: f32,
    /// False: gain 0, today's ants (the honest arm).
    pub listening: bool,
    /// The station's column.
    pub station_x: i32,
    /// Ration cells a 1,000 frames, onto home cells in the nest.
    pub ration: u32,
    pub counts: KeeperCounts,
    /// Ants seen taking food at the station, with their `trip_deliveries` at
    /// that moment: a rise after it is a station delivery.
    haul: BTreeMap<OrganismId, u32>,
    /// Station cells at the end of the last scan (after any top-up).
    station_cells: usize,
    last_scan: u64,
    /// Ration cells placed and not yet seen eaten.
    ration_cells: Vec<(i32, i32)>,
    /// Ants whose head was at the station at the last scan.
    at_station: std::collections::BTreeSet<OrganismId>,
    pub tool: Tool,
    pub overlay: Overlay,
    /// The ant in the hand (CARRY, between press and release).
    pub held: Option<OrganismId>,
    /// Frames to run before the player gets the box.
    pub warmup: u64,
    /// Ant-head counts per cell, halved every [`HEAT_HALF`] frames: the
    /// traffic overlay. Empty until the overlay is first shown.
    heat: Vec<u16>,
    /// **Stakes on** (the default): windfalls, charges, the round and the
    /// verdict. `stakes=0` is the sandbox.
    pub stakes: bool,
    pub rules: Rules,
    /// Run the idle twin for the verdict (`twin=0` turns it off; a harness
    /// starts it itself with [`start_twin`]).
    pub twin_on: bool,
    /// How many idle twins make the band (`twins=`, at least 1): the exact
    /// one, then knocked ones ([`TwinLog`]).
    pub twins: u32,
    /// The warm-up is running: no round yet.
    warming: bool,
    /// The frame the round opened on, and the ants alive then.
    pub opened: Option<u64>,
    pub ants_at_open: usize,
    /// [`fingerprint`] of the box at the opening, before any hand.
    pub opening_print: Option<u64>,
    pub patches: Vec<Patch>,
    rng: u64,
    next_patch: u64,
    pub charges: u32,
    recharge_at: u64,
    /// Treats on hand: windfall cells saved, less treats bought.
    pub pantry: u64,
    treats_spent: u64,
    pub ended: Option<RoundEnd>,
    /// The verdict card is up (`K`; it comes up by itself at the end).
    pub card: bool,
    /// Where the pointer is while an ant is in the hand.
    pub hand_at: Option<(i32, i32)>,
    /// The idle twin, once started.
    pub twin: Option<Arc<Mutex<TwinLog>>>,
    /// Ants seen taking food at a windfall: their `trip_deliveries` then,
    /// and which windfall (an index into `patches`).
    patch_haul: BTreeMap<OrganismId, (u32, usize)>,
    /// Every ant's `life.bites` at the last windfall scan: a rise by an ant
    /// standing at a windfall is cells its mouth took there ([`Patch::taken`]).
    bites: BTreeMap<OrganismId, u32>,
}

impl Default for KeeperDemo {
    fn default() -> Self {
        Self {
            gain: DEFAULT_GAIN,
            copy: DEFAULT_COPY,
            listening: true,
            station_x: 390,
            ration: 3,
            counts: KeeperCounts::default(),
            haul: BTreeMap::new(),
            station_cells: 0,
            last_scan: 0,
            ration_cells: Vec::new(),
            at_station: Default::default(),
            tool: Tool::None,
            overlay: Overlay::Off,
            held: None,
            warmup: DEFAULT_WARMUP,
            heat: Vec::new(),
            stakes: true,
            rules: Rules::default(),
            twin_on: true,
            twins: DEFAULT_TWINS,
            warming: false,
            opened: None,
            ants_at_open: 0,
            opening_print: None,
            patches: Vec::new(),
            rng: 0,
            next_patch: 0,
            charges: Rules::default().charges,
            recharge_at: 0,
            pantry: 0,
            treats_spent: 0,
            ended: None,
            card: false,
            hand_at: None,
            twin: None,
            patch_haul: BTreeMap::new(),
            bites: BTreeMap::new(),
        }
    }
}

impl KeeperDemo {
    /// **The same settings, every running count cleared**: what a rebuilt
    /// box (`Lab::reset`) starts from.
    fn fresh(&self) -> Self {
        Self {
            gain: self.gain,
            copy: self.copy,
            listening: self.listening,
            station_x: self.station_x,
            ration: self.ration,
            tool: self.tool,
            overlay: self.overlay,
            warmup: self.warmup,
            stakes: self.stakes,
            rules: self.rules,
            twin_on: self.twin_on,
            twins: self.twins,
            charges: self.rules.charges,
            ..KeeperDemo::default()
        }
    }

    /// The demo's settings from the command line: `gain=`, `copy=`,
    /// `station=`, `ration=`, `listening=0`, `stakes=0`, `twin=0`, and each
    /// of [`Rules`]' fields by name.
    pub fn from_args(args: &BTreeMap<String, String>) -> Self {
        let mut k = Self::default();
        let num = |key: &str| args.get(key).map(|v| v.parse::<f32>().unwrap_or_else(|_| panic!("demo=keeper: {key}={v:?} is not a number")));
        let int = |key: &str| args.get(key).map(|v| v.parse::<u64>().unwrap_or_else(|_| panic!("demo=keeper: {key}={v:?} is not a whole number")));
        if let Some(s) = int("stakes") {
            k.stakes = s != 0;
        }
        if let Some(t) = int("twin") {
            k.twin_on = t != 0;
        }
        if let Some(t) = int("twins") {
            k.twins = (t as u32).max(1);
        }
        let r = &mut k.rules;
        r.patch_every = int("patch_every").unwrap_or(r.patch_every).max(1);
        r.patch_life = int("patch_life").unwrap_or(r.patch_life);
        r.patch_cells = int("patch_cells").map_or(r.patch_cells, |v| v as i32);
        r.charges = int("charges").map_or(r.charges, |v| v as u32);
        r.recharge = int("recharge").unwrap_or(r.recharge).max(1);
        r.round = int("round").unwrap_or(r.round);
        r.lose_at = int("lose_at").map_or(r.lose_at, |v| v as usize);
        r.ration = int("stakes_ration").map_or(r.ration, |v| v as u32);
        r.treat_cost = int("treat_cost").map_or(r.treat_cost, |v| v as u32);
        r.lift_cost = num("lift_cost").unwrap_or(r.lift_cost).clamp(0.0, 1.0);
        k.charges = k.rules.charges;
        if let Some(g) = num("gain") {
            k.gain = g;
        }
        if let Some(c) = num("copy") {
            k.copy = c;
        }
        if let Some(x) = num("station") {
            k.station_x = x as i32;
        }
        if let Some(r) = num("ration") {
            k.ration = r as u32;
        }
        if let Some(l) = num("listening") {
            k.listening = l != 0.0;
        }
        if let Some(w) = num("warmup") {
            k.warmup = w as u64;
        }
        k
    }

    /// The gain the head runs at now.
    pub fn live_gain(&self) -> f32 {
        if self.listening {
            self.gain
        } else {
            0.0
        }
    }
}

/// The station as a place an ant can remember: its column at the surface.
pub fn station_place(world: &World, k: &KeeperDemo) -> (i32, i32) {
    (k.station_x, surface_under(world, k.station_x))
}

/// The ground (or the top of the heap) in column `x`, looked for from under
/// the lid: the first non-empty row from the top is the box's own ceiling.
pub fn surface_under(world: &World, x: i32) -> i32 {
    let top = world.enclosure().map_or(0, |e| e.ceiling_y + 1);
    creature::colony_surface(world, x, top).unwrap_or(super::super::HEIGHT as i32 / 2)
}

/// Provisions cells in the station's counted columns.
fn station_food(world: &World, k: &KeeperDemo) -> usize {
    let Some(food) = world.materials.id_of("provisions") else { return 0 };
    let (w, h) = (super::super::WIDTH as i32, super::super::HEIGHT as i32);
    let mut n = 0;
    for x in (k.station_x - STATION_R).max(0)..=(k.station_x + STATION_R).min(w - 1) {
        for y in 0..h {
            if world.get(x, y).material == food {
                n += 1;
            }
        }
    }
    n
}

fn keeper(lab: &Lab) -> Option<&KeeperDemo> {
    lab.demo.as_ref()?.keeper.as_ref()
}

fn keeper_mut(lab: &mut Lab) -> Option<&mut KeeperDemo> {
    lab.demo.as_mut()?.keeper.as_mut()
}

/// **Put the recall head on the world** if it is missing (a fresh box, or a
/// rebuild: `Lab::reset` builds a new `World`), and keep its gain at the
/// demo's live gain.
fn ensure_recall(lab: &mut Lab) {
    let Some((gain, copy)) = keeper(lab).map(|k| (k.live_gain(), k.copy)) else { return };
    match lab.world.recall.as_mut() {
        Some(r) => {
            r.gain = gain;
            r.copy = copy;
        }
        None => lab.world.recall = Some(recall::Recall::new(gain, copy)),
    }
}

/// **After the demo's scenario loads** (the launcher has already applied
/// `seed=`): run the warm-up with nobody watching.
pub fn on_start(lab: &mut Lab) {
    let warmup = keeper(lab).map_or(0, |k| k.warmup);
    // **The hand-built ant's windfalls rot sooner** (`-- oldant`, the fun
    // hunt's ant): it forages far better (65 ants at the opening against
    // ~25 trained, and idle it took 378-436 of 480 windfall cells at the
    // trained life of 4,500 frames, 4 seeds), so at that life there is
    // little left for a hand to save. At 2,400 the idle colony takes
    // 299-377 and a good keeper beat idle in ants 4 of 4 (+5 to +74).
    // A `patch_life=` on the command line wins.
    let given = lab.demo.as_ref().is_some_and(|d| d.args.contains_key("patch_life"));
    let new_ant = lab.new_ant;
    if let Some(k) = keeper_mut(lab) {
        k.warming = true;
        if !new_ant && !given {
            k.rules.patch_life = OLDANT_PATCH_LIFE;
        }
    }
    for _ in 0..warmup {
        lab.tick_for_harness();
    }
    let seed = lab.spec.seed;
    let print = fingerprint(&lab.world);
    let frame = lab.world.frame;
    let alive = live_ants(&lab.world);
    if let Some(k) = keeper_mut(lab) {
        k.warming = false;
        k.opening_print = Some(print);
        open_round(k, frame, alive, seed);
    }
}

/// **Open the round** at `frame`: the windfall sequence is seeded from the
/// box's seed, so the same seed drops the same windfalls in the same order
/// (the twin's and the player's box alike).
fn open_round(k: &mut KeeperDemo, frame: u64, alive: usize, seed: u64) {
    k.opened = Some(frame);
    k.ants_at_open = alive;
    k.counts.min_ants = alive;
    k.rng = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x6B65_6570_6572;
    k.next_patch = frame;
    k.charges = k.rules.charges;
    k.recharge_at = frame + k.rules.recharge;
}

/// splitmix64: the windfall columns. The demo's own stream, so it never
/// draws from (and never shifts) the world's.
fn next_rand(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn live_ants(world: &World) -> usize {
    let Some(ant) = world.species.id_of("ant") else { return 0 };
    world.live_organism_ids().into_iter().filter(|&id| world.organism(id).is_some_and(|s| s.species == ant)).count()
}

/// **The box's fingerprint**: every cell's material and every live ant's
/// head, FNV-1a. Two boxes built from the same scenario and seed agree on
/// it at the opening; a twin that does not is not the player's box left
/// alone, and the card says so instead of trusting it.
pub fn fingerprint(world: &World) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |v: u64| {
        h ^= v;
        h = h.wrapping_mul(0x0100_0000_01b3);
    };
    let (w, ht) = (super::super::WIDTH as i32, super::super::HEIGHT as i32);
    for y in 0..ht {
        for x in 0..w {
            eat(u64::from(world.get(x, y).material.0));
        }
    }
    for id in world.live_organism_ids() {
        if let Some(&(x, y)) = world.organism(id).and_then(|s| s.chain.first()) {
            eat(((x as u64) << 32) ^ (y as u32 as u64));
        }
    }
    eat(world.frame);
    h
}

/// One of the seat keys (`Y U P J K R`) was pressed.
pub fn seat_key(lab: &mut Lab, key: char) {
    let Some(k) = keeper_mut(lab) else { return };
    let msg = match key {
        'Y' => {
            k.tool = if k.tool == Tool::Carry { Tool::None } else { Tool::Carry };
            k.held = None;
            k.hand_at = None;
            if k.tool == Tool::Carry { "CARRY: PRESS ON AN ANT, DRAG IT TO FOOD, LET GO" } else { "CARRY OFF" }.to_string()
        }
        'U' => {
            k.tool = if k.tool == Tool::Treat { Tool::None } else { Tool::Treat };
            k.held = None;
            k.hand_at = None;
            if k.stakes {
                format!("TREAT: CLICK AN ANT TO FEED IT -- COSTS {} FROM THE PANTRY", k.rules.treat_cost)
            } else {
                "TREAT: CLICK AN ANT TO FEED IT A MOUTHFUL -- AWAY FROM HOME IT REMEMBERS WHERE".to_string()
            }
        }
        // `P` (listening) is no longer a player's key: `listening=0` on the
        // command line. Swallowed, so it does not fall through to the lab.
        'P' => return,
        'K' => {
            k.card = !k.card;
            if k.card { "THE CARD: YOUR COLONY AGAINST THE SAME SEED LEFT ALONE (K CLOSES)" } else { "CARD CLOSED" }.to_string()
        }
        'J' => {
            k.overlay = match k.overlay {
                Overlay::Off => Overlay::Knows,
                Overlay::Knows => Overlay::Traffic,
                Overlay::Traffic => Overlay::Off,
            };
            match k.overlay {
                Overlay::Off => "OVERLAY OFF",
                Overlay::Knows => "OVERLAY: WHO KNOWS A TAUGHT PLACE -- WHITE BY YOUR HAND, AMBER FED ONCE, RED FED TWICE OR MORE, GREY ASLEEP",
                Overlay::Traffic => "OVERLAY: TRAFFIC -- WHERE HEADS HAVE BEEN IN THE LAST FEW THOUSAND FRAMES",
            }
            .to_string()
        }
        _ => return,
    };
    ensure_recall(lab);
    lab.ui.say(msg);
}

/// **Once a displayed frame** (the window only; a harness never calls it):
/// start the idle twin the first time, so the verdict has its comparison.
pub fn frame(lab: &mut Lab) {
    let want = keeper(lab).is_some_and(|k| k.stakes && k.twin_on && k.twin.is_none() && k.opened.is_some());
    if want {
        start_twin(lab);
    }
}

/// **Start the idle twin** for this box: the same scenario, seed and
/// arguments, built and run on its own thread with nobody's hand on it, to
/// the end of the round. Its read-outs land in [`KeeperDemo::twin`].
pub fn start_twin(lab: &mut Lab) {
    let Some(demo) = lab.demo.as_ref() else { return };
    let mut args = demo.args.clone();
    args.insert("seed".to_string(), lab.spec.seed.to_string());
    // The twin thinks with the same brain as the player's box (`-- oldant`).
    args.insert("twin_oldant".to_string(), if lab.new_ant { "0" } else { "1" }.to_string());
    let Some(k) = keeper(lab) else { return };
    let until = k.opened.unwrap_or(k.warmup) + k.rules.round;
    let log = spawn_twin(args, until, k.twins);
    if let Some(k) = keeper_mut(lab) {
        k.twin = Some(log);
    }
}

/// [`start_twin`]'s thread. Public so a harness can check the twin against
/// an idle arm it ran itself (they must agree to the ant).
pub fn spawn_twin(mut args: BTreeMap<String, String>, until: u64, runs: u32) -> Arc<Mutex<TwinLog>> {
    let log = Arc::new(Mutex::new(TwinLog::default()));
    let out = log.clone();
    args.insert("twin".to_string(), "0".to_string());
    let spawned = std::thread::Builder::new().name("keeper-twin".to_string()).spawn(move || {
        let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            for n in 0..runs.max(1) {
                if !run_twin(&args, until, u64::from(n), &out) {
                    break;
                }
            }
        }));
        if let Ok(mut l) = out.lock() {
            l.done = true;
            l.exact_done = true;
        }
        if let Err(e) = ran {
            let why = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
            if let Ok(mut l) = out.lock() {
                l.failed = Some(why);
                l.done = true;
            }
        }
    });
    if let Err(e) = spawned {
        if let Ok(mut l) = log.lock() {
            l.failed = Some(e.to_string());
            l.done = true;
        }
    }
    log
}

/// Frames a knocked twin runs on a changed seed ([`perturb`]).
const KNOCK_FRAMES: u64 = 60;

/// **Knock the box's random draws** (`n` > 0) without touching a cell or
/// an ant: from the opening exactly, [`KNOCK_FRAMES`] ticks run with the
/// world's seed changed -- the seed every ant's per-frame choice is drawn
/// from (`rng::stream(world.seed, ..)`) -- and then it is put back, so the
/// box is the same box and only one second of its dice differ. `n` = 0
/// does nothing. Public so a harness can run a knocked idle arm itself.
///
/// **Not `world.rng`**: knocking that stream instead (tried first, 10-31)
/// left 45 of 48 knocked twins (seeds 25-36, both ants) saving exactly what
/// the exact one saved -- the ants never draw from it, so the "band" was
/// one run three times.
pub fn perturb(lab: &mut Lab, n: u64) {
    if n == 0 {
        return;
    }
    let seed = lab.world.seed;
    lab.world.seed = seed ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for _ in 0..KNOCK_FRAMES {
        lab.tick_for_harness();
    }
    lab.world.seed = seed;
}

/// One twin, the `n`th: built, knocked by [`perturb`] after its opening, and
/// run to `until`. The exact twin (`n` = 0) writes every read-out; each
/// twin writes its last. False if it could not run (the rest are skipped).
fn run_twin(args: &BTreeMap<String, String>, until: u64, n: u64, out: &Arc<Mutex<TwinLog>>) -> bool {
    let sc = match super::super::scenario::Scenario::load("demo_keeper") {
        Ok(s) => s,
        Err(e) => {
            if let Ok(mut l) = out.lock() {
                l.failed = Some(e);
            }
            return false;
        }
    };
    let mut lab = Lab::new(sc.bed.clone());
    lab.set_new_ant(args.get("twin_oldant").is_none_or(|v| v != "1"));
    lab.start_demo_with(super::DemoSeat::Keeper, args.clone());
    let print = keeper(&lab).and_then(|k| k.opening_print);
    if let Ok(mut l) = out.lock() {
        if n == 0 {
            l.opening = print;
        } else if l.opening != print {
            l.failed = Some(format!("KNOCKED TWIN {n} OPENED ON ANOTHER BOX"));
            return false;
        }
    }
    perturb(&mut lab, n);
    let mut last = None;
    loop {
        let f = lab.world.frame;
        let k = keeper(&lab);
        let ended = k.and_then(|k| k.ended);
        if f.is_multiple_of(TWIN_EVERY) || f >= until || ended.is_some() {
            let point = TwinPoint { frame: f, ants: live_ants(&lab.world), saved: k.map_or(0, |k| k.counts.saved), home: k.map_or(0, |k| k.counts.patch_home) };
            last = Some(point);
            if n == 0 {
                if let Ok(mut l) = out.lock() {
                    l.points.push(point);
                    l.ended = ended;
                }
            }
        }
        if f >= until || ended.is_some() {
            break;
        }
        lab.tick_for_harness();
    }
    if let Ok(mut l) = out.lock() {
        if n == 0 {
            l.exact_done = true;
        }
        l.finals.extend(last);
    }
    true
}

/// What the twin says, for the HUD and the card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwinRead {
    /// No twin (`twin=0`, `stakes=0`, or not started yet).
    Off,
    /// The twin has not reached this frame yet.
    Behind { at: u64 },
    /// The twin's opening did not match this box's.
    Mismatch,
    Failed,
    /// The twin at (or just before) the frame asked for.
    At(TwinPoint),
}

/// **The twin at `frame`**: its last read-out at or before `frame`, once it
/// has got that far. A twin whose round ended earlier (lost) holds its last
/// point.
pub fn twin_at(k: &KeeperDemo, frame: u64) -> TwinRead {
    let Some(t) = k.twin.as_ref() else { return TwinRead::Off };
    let Ok(l) = t.lock() else { return TwinRead::Failed };
    if l.failed.is_some() {
        return TwinRead::Failed;
    }
    if let (Some(a), Some(b)) = (l.opening, k.opening_print) {
        if a != b {
            return TwinRead::Mismatch;
        }
    }
    let last = l.points.last().copied();
    match last {
        Some(p) if p.frame >= frame || l.exact_done => {
            let at = l.points.iter().rev().find(|p| p.frame <= frame).copied().unwrap_or(p);
            TwinRead::At(at)
        }
        _ => TwinRead::Behind { at: last.map_or(0, |p| p.frame) },
    }
}

/// Before each simulated tick.
pub fn before_tick(lab: &mut Lab) {
    ensure_recall(lab);
}

/// After each simulated tick: the counters, the station top-up (sandbox) or
/// the windfalls, charges and the round (stakes), the ration.
pub fn after_tick(lab: &mut Lab) {
    let frame = lab.world.frame;
    let Some(mut k) = lab.demo.as_mut().and_then(|d| d.keeper.take()) else { return };
    // **A rebuilt box starts the counts again** (`Lab::reset` makes a new
    // `World` whose frame restarts): yesterday's ids mean other ants now.
    // The round opens again on the rebuilt box's first tick (no warm-up).
    if frame < k.last_scan {
        k = k.fresh();
        open_round(&mut k, frame, live_ants(&lab.world), lab.spec.seed);
    }
    let playing = k.stakes && !k.warming && k.opened.is_some();
    if frame.is_multiple_of(SCAN_EVERY) {
        let since = k.last_scan;
        scan(&mut lab.world, &mut k, frame);
        if playing {
            scan_patches(lab, &mut k, frame, since);
        }
    }
    if !k.stakes && frame.is_multiple_of(TOP_UP_EVERY) {
        top_up_station(lab, &mut k);
    }
    let ration = if playing { k.rules.ration } else { k.ration };
    if frame.is_multiple_of(1000) && ration > 0 {
        give_ration(&mut lab.world, &mut k, ration);
    }
    if playing && k.ended.is_none() {
        play(lab, &mut k, frame);
    }
    if frame.is_multiple_of(600) {
        recall::prune(&mut lab.world);
    }
    if !k.heat.is_empty() && frame.is_multiple_of(HEAT_HALF) {
        for h in &mut k.heat {
            *h /= 2;
        }
    }
    if let Some(d) = lab.demo.as_mut() {
        d.keeper = Some(k);
    }
}

/// The counters' scan: station bites since the last scan, ants seen taking
/// food at the station, and the station deliveries they then make.
fn scan(world: &mut World, k: &mut KeeperDemo, frame: u64) {
    let now = station_food(world, k);
    k.counts.station_bites += k.station_cells.saturating_sub(now) as u64;
    k.station_cells = now;
    let Some(ant) = world.species.id_of("ant") else { return };
    let since = k.last_scan;
    k.last_scan = frame;
    let (w, h) = (super::super::WIDTH as i32, super::super::HEIGHT as i32);
    if k.overlay == Overlay::Traffic && k.heat.is_empty() {
        k.heat = vec![0; (w * h) as usize];
    }
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        if st.species != ant {
            continue;
        }
        if let (false, Some(&(x, y))) = (k.heat.is_empty(), st.chain.first()) {
            if (0..w).contains(&x) && (0..h).contains(&y) {
                let c = &mut k.heat[(y * w + x) as usize];
                *c = c.saturating_add(1);
            }
        }
        let trips = st.life.trip_deliveries;
        let here = st.chain.first().is_some_and(|h| (h.0 - k.station_x).abs() <= STATION_R);
        if here && k.at_station.insert(id) {
            k.counts.arrivals += 1;
            if world.recall.as_ref().is_some_and(|r| r.provenance.contains_key(&id)) {
                k.counts.taught_arrivals += 1;
            }
        } else if !here {
            k.at_station.remove(&id);
        }
        let at_station = st.last_food.is_some_and(|(x, _, _)| (x - k.station_x).abs() <= STATION_R);
        let fresh = st.last_food.is_some_and(|(_, _, f)| f > since || since == 0);
        if let Some(&before) = k.haul.get(&id) {
            if trips > before {
                k.counts.station_deliveries += u64::from(trips - before);
                k.counts.first_station_delivery.get_or_insert(frame);
                k.haul.remove(&id);
            } else if !at_station {
                k.haul.remove(&id);
            }
        } else if at_station && fresh && st.crop.is_some() {
            k.haul.insert(id, trips);
        }
    }
    k.haul.retain(|&id, _| world.organism(id).is_some());
    k.at_station.retain(|&id| world.organism(id).is_some());
}

/// Top the station back up to [`STATION_LEVEL`].
fn top_up_station(lab: &mut Lab, k: &mut KeeperDemo) {
    let have = station_food(&lab.world, k);
    if have >= STATION_LEVEL {
        return;
    }
    let want = (STATION_LEVEL - have) as i32;
    let placed = super::super::scenario::apply_placements(
        &mut lab.world,
        &[super::super::scenario::Placement::Heap { material: "provisions".into(), x: k.station_x, cells: want }],
        &lab.spec,
    );
    k.counts.station_given += placed.cells as u64;
    k.station_cells = station_food(&lab.world, k);
}

/// Seconds at 1x, for the notices and the HUD.
fn secs(frames: u64) -> u64 {
    frames.div_ceil(60)
}

/// **The round under stakes, one tick**: a charge back, a windfall due, a
/// windfall rotting, and the round's end.
fn play(lab: &mut Lab, k: &mut KeeperDemo, frame: u64) {
    if k.charges >= k.rules.charges {
        k.recharge_at = frame + k.rules.recharge;
    } else if frame >= k.recharge_at {
        k.charges += 1;
        k.recharge_at = frame + k.rules.recharge;
    }
    if frame >= k.next_patch {
        k.next_patch = frame + k.rules.patch_every;
        drop_patch(lab, k, frame);
    }
    for i in 0..k.patches.len() {
        if k.patches[i].live() && frame >= k.patches[i].rots {
            rot_patch(lab, k, i, frame);
        }
    }
    if frame.is_multiple_of(SCAN_EVERY) {
        let alive = live_ants(&lab.world);
        k.counts.min_ants = k.counts.min_ants.min(alive);
        let opened = k.opened.unwrap_or(frame);
        let why = if alive < k.rules.lose_at {
            Some(Ending::Lost)
        } else if frame >= opened + k.rules.round {
            Some(Ending::Time)
        } else {
            None
        };
        if let Some(why) = why {
            k.ended = Some(RoundEnd { frame, why, ants: alive, saved: k.counts.saved, home: k.counts.patch_home });
            k.card = true;
            k.held = None;
            k.hand_at = None;
            lab.time.pause();
            lab.ui.say(match why {
                Ending::Lost => format!("THE COLONY IS LOST: {alive} ANTS LEFT -- THE CARD IS UP (K CLOSES)"),
                Ending::Time => "THE ROUND IS OVER -- THE CARD IS UP (K CLOSES)".to_string(),
            });
        }
    }
}

/// **Drop a windfall** at a seeded random far column, clear of every live
/// one, and say where.
fn drop_patch(lab: &mut Lab, k: &mut KeeperDemo, frame: u64) {
    let span = (PATCH_MAX_X - PATCH_MIN_X + 1) as u64;
    let mut x = PATCH_MIN_X;
    for _ in 0..16 {
        x = PATCH_MIN_X + (next_rand(&mut k.rng) % span) as i32;
        if k.patches.iter().all(|p| !p.live() || (p.x - x).abs() > 2 * PATCH_R + 6) {
            break;
        }
    }
    let y = surface_under(&lab.world, x);
    let placed = super::super::scenario::apply_placements(
        &mut lab.world,
        &[super::super::scenario::Placement::Heap { material: "provisions".into(), x, cells: k.rules.patch_cells }],
        &lab.spec,
    );
    if placed.cells == 0 {
        return;
    }
    let size = placed.cells as u32;
    k.patches.push(Patch { x, y, landed: frame, rots: frame + k.rules.patch_life, size, left: size, taken: 0, home: 0, taught: 0, ended: None });
    k.counts.patches += 1;
    lab.ui.say(format!("A WINDFALL LANDED AT {x} -- IT ROTS IN {}S. CARRY ANTS TO IT", secs(k.rules.patch_life)));
}

/// A windfall's cells: (x0, x1, y0, y1), inclusive.
fn patch_rect(p: &Patch) -> (i32, i32, i32, i32) {
    let (w, h) = (super::super::WIDTH as i32, super::super::HEIGHT as i32);
    ((p.x - PATCH_R).max(0), (p.x + PATCH_R).min(w - 1), (p.y - PATCH_UP).max(0), (p.y + PATCH_DOWN).min(h - 1))
}

fn patch_food(world: &World, p: &Patch) -> u32 {
    let Some(food) = world.materials.id_of("provisions") else { return 0 };
    let (x0, x1, y0, y1) = patch_rect(p);
    let mut n = 0;
    for y in y0..=y1 {
        for x in x0..=x1 {
            if world.get(x, y).material == food {
                n += 1;
            }
        }
    }
    n
}

/// **A windfall rots**: what is left of it turns to ash, a grey heap the
/// ants cannot eat, where everyone can see the food that was lost.
fn rot_patch(lab: &mut Lab, k: &mut KeeperDemo, i: usize, frame: u64) {
    let world = &mut lab.world;
    let (Some(food), Some(ash)) = (world.materials.id_of("provisions"), world.materials.id_of("ash")) else { return };
    let (x0, x1, y0, y1) = patch_rect(&k.patches[i]);
    let mut left = 0u32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            if world.get(x, y).material == food {
                world.set(x, y, Cell::new(ash, 0));
                left += 1;
            }
        }
    }
    let p = &mut k.patches[i];
    p.left = 0;
    p.ended = Some((frame, PatchEnd::Rotted { left }));
    k.counts.rotted += 1;
    k.counts.rotted_cells += u64::from(left);
    lab.ui.say(format!("THE WINDFALL AT {} ROTTED: {left} CELLS LOST, {} HOME", p.x, p.home));
}

/// Columns and rows past a windfall's rectangle an ant's head may stand
/// while its jaws take a cell at the edge (a bite is a neighbouring cell,
/// and the head moves a little in the [`SCAN_EVERY`] frames between scans).
/// Windfalls land at least `2 * PATCH_R + 6` apart, so no head is near two.
const BITE_REACH: i32 = 3;

/// **The windfalls' scan**: cells left on each, the cells ants took there
/// ([`Patch::taken`], the score), a windfall eaten to nothing stripped, and
/// the windfall cells ants carry home -- by `trip_deliveries` exactly as the
/// station's count is.
fn scan_patches(lab: &mut Lab, k: &mut KeeperDemo, frame: u64, since: u64) {
    let Some(ant) = lab.world.species.id_of("ant") else { return };
    // Bites since the last scan by ants standing at each live windfall.
    let mut bit = vec![0u32; k.patches.len()];
    for id in lab.world.live_organism_ids() {
        let Some(st) = lab.world.organism(id) else { continue };
        if st.species != ant {
            continue;
        }
        let now = st.life.bites;
        let before = k.bites.insert(id, now).unwrap_or(now);
        let d = now.saturating_sub(before);
        if d == 0 {
            continue;
        }
        let Some(&(hx, hy)) = st.chain.first() else { continue };
        if let Some(i) = k.patches.iter().rposition(|p| {
            let (x0, x1, y0, y1) = patch_rect(p);
            p.live() && (x0 - BITE_REACH..=x1 + BITE_REACH).contains(&hx) && (y0 - BITE_REACH..=y1 + BITE_REACH).contains(&hy)
        }) {
            bit[i] += d;
        }
    }
    k.bites.retain(|&id, _| lab.world.organism(id).is_some());
    for (i, &bites) in bit.iter().enumerate() {
        if !k.patches[i].live() {
            continue;
        }
        let left = patch_food(&lab.world, &k.patches[i]);
        let p = &mut k.patches[i];
        let gone = p.left.saturating_sub(left);
        p.taken = (p.taken + gone.min(bites)).min(p.size.saturating_sub(left));
        p.left = left;
        if left == 0 {
            p.ended = Some((frame, PatchEnd::Stripped));
            k.counts.stripped += 1;
            let (x, taken) = (p.x, p.taken);
            lab.ui.say(format!("THE WINDFALL AT {x} IS STRIPPED BARE -- {taken} CELLS SAVED BY ANTS"));
        }
    }
    k.counts.saved = k.patches.iter().map(|p| u64::from(p.taken)).sum();
    k.pantry = k.counts.saved.saturating_sub(k.treats_spent);
    let world = &lab.world;
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        if st.species != ant {
            continue;
        }
        let trips = st.life.trip_deliveries;
        let at = st.last_food.and_then(|(x, _, _)| k.patches.iter().rposition(|p| (x - p.x).abs() <= PATCH_R));
        let fresh = st.last_food.is_some_and(|(_, _, f)| f > since);
        if let Some(&(before, i)) = k.patch_haul.get(&id) {
            if trips > before {
                let d = trips - before;
                k.patches[i].home += d;
                k.counts.patch_home += u64::from(d);
                k.patch_haul.remove(&id);
            } else if at != Some(i) {
                k.patch_haul.remove(&id);
            }
        } else if let (Some(i), true, true) = (at, fresh, st.crop.is_some()) {
            k.patch_haul.insert(id, (trips, i));
        }
    }
    k.patch_haul.retain(|&id, _| world.organism(id).is_some());
}

/// The hand is limited now: stakes on, the round open and not over.
fn limited(k: &KeeperDemo) -> bool {
    k.stakes && k.opened.is_some() && k.ended.is_none() && !k.warming
}

/// **The hand rests once the round is over** (stakes on). Before 10-31 an
/// ended round made [`limited`] false, so a box run on past its card had
/// free carries and treats with no pantry check, on a score already frozen.
const ROUND_OVER: &str = "THE ROUND IS OVER -- THE HAND RESTS (STAKES=0 IS THE SANDBOX)";

fn resting(k: &KeeperDemo) -> bool {
    k.stakes && k.ended.is_some()
}

/// What a carry did: the cell the ant landed on, and the food it was
/// taught, if any.
pub type Carried = ((i32, i32), Option<(i32, i32)>);

/// **The hand's CARRY**, the player's and a harness's one path: set `ant`
/// down at `to`, spend a charge, and teach it the food there if any. Under
/// stakes with no charge left it refuses, and says when one comes back.
/// Returns where it landed and what it was taught.
pub fn hand_carry(lab: &mut Lab, ant: OrganismId, to: (i32, i32)) -> Result<Carried, String> {
    let frame = lab.world.frame;
    let Some(k) = keeper_mut(lab) else { return Err("NO KEEPER DEMO".to_string()) };
    if resting(k) {
        k.counts.refused += 1;
        return Err(ROUND_OVER.to_string());
    }
    let limit = limited(k);
    if limit && k.charges == 0 {
        k.counts.refused += 1;
        return Err(format!("NO CHARGES LEFT -- ONE COMES BACK IN {}S", secs(k.recharge_at.saturating_sub(frame))));
    }
    let at = creature::set_down(&mut lab.world, ant, to).map_err(|why| format!("CANNOT SET IT DOWN THERE: {}", why.to_uppercase()))?;
    if limit {
        let cost = keeper(lab).map_or(0.0, |k| k.rules.lift_cost);
        if let Some(st) = lab.world.organism_mut(ant) {
            st.energy *= 1.0 - cost;
        }
    }
    let taught = teach_near(lab, ant, at);
    lab.world.log_player_action(format!("KEEPER CARRIED ANT {ant:?} TO {},{}{}", at.0, at.1, if taught.is_some() { ", TAUGHT" } else { "" }));
    if let Some(k) = keeper_mut(lab) {
        k.counts.carried += 1;
        if limit {
            k.charges -= 1;
        }
        if let Some(f) = taught {
            if let Some(i) = k.patches.iter().rposition(|p| p.live() && (f.0 - p.x).abs() <= PATCH_R) {
                k.patches[i].taught += 1;
                k.counts.carried_to_food += 1;
            }
        }
    }
    Ok((at, taught))
}

/// **The ration**: up to `ration` provisions cells onto empty home cells
/// with ground under them, deepest first (inside the chamber, not on the
/// mound), never more than four rations standing. No home dug yet: nothing.
fn give_ration(world: &mut World, k: &mut KeeperDemo, ration: u32) {
    let Some(food) = world.materials.id_of("provisions") else { return };
    k.ration_cells.retain(|&(x, y)| world.get(x, y).material == food);
    let room = (4 * ration as usize).saturating_sub(k.ration_cells.len()).min(ration as usize);
    if room == 0 {
        return;
    }
    let Some(site) = world.nest_sites.first().copied() else { return };
    let Some(ant) = super::oldest_ant(world) else { return };
    let (w, h) = (super::super::WIDTH as i32, super::super::HEIGHT as i32);
    let mut spots = Vec::new();
    for x in (site.x - 60).max(1)..=(site.x + 60).min(w - 2) {
        for y in (site.surface - 4).max(1)..=(site.surface + 80).min(h - 2) {
            if world.get(x, y).material != material::EMPTY || world.get(x, y + 1).material == material::EMPTY || world.get(x, y + 1).material == food {
                continue;
            }
            if creature::is_home_for(world, ant, (x, y)) {
                spots.push((x, y));
            }
        }
    }
    // Deepest first, then nearest the site's column: inside the chamber.
    spots.sort_by_key(|&(x, y)| (-y, (x - site.x).abs()));
    for (x, y) in spots.into_iter().take(room) {
        world.set(x, y, Cell::new(food, 0));
        k.ration_cells.push((x, y));
        k.counts.ration_given += 1;
    }
}

/// **The read-out a harness and the HUD share**: ants alive, ants that know
/// the station (strength x freshness >= [`KNOWS_PHI`]), and starved deaths.
pub fn census(lab: &Lab) -> (usize, usize, u64) {
    let Some(k) = keeper(lab) else { return (0, 0, 0) };
    let place = station_place(&lab.world, k);
    let Some(ant) = lab.world.species.id_of("ant") else { return (0, 0, 0) };
    let mut alive = 0;
    let mut knows = 0;
    for id in lab.world.live_organism_ids() {
        if lab.world.organism(id).is_none_or(|s| s.species != ant) {
            continue;
        }
        alive += 1;
        if recall::knows(&lab.world, id, place, STATION_R).is_some_and(|v| v >= KNOWS_PHI) {
            knows += 1;
        }
    }
    (alive, knows, lab.world.deaths_by_cause[DeathCause::Starved as usize])
}

/// The demo's counts, for a harness.
pub fn counts(lab: &Lab) -> Option<KeeperCounts> {
    keeper(lab).map(|k| k.counts)
}

/// **Teach**: carry `ant` to the station and set it down there, and plant
/// the place in its memory. Counted as a carry.
pub fn carry_to_station(lab: &mut Lab, ant: OrganismId) -> Result<(i32, i32), &'static str> {
    let Some(k) = keeper(lab) else { return Err("no keeper demo") };
    let place = station_place(&lab.world, k);
    ensure_recall(lab);
    let at = creature::set_down(&mut lab.world, ant, (place.0 - 6, place.1 - 2))?;
    recall::plant(&mut lab.world, ant, place);
    if let Some(k) = keeper_mut(lab) {
        k.counts.carried += 1;
    }
    Ok(at)
}

/// The ant whose head is nearest `cell`, within [`PICK_R`]: one the hand can
/// lift if there is one (`creature::can_carry`), else the nearest, so a
/// refusal names why.
fn ant_at(world: &World, cell: (i32, i32)) -> Option<OrganismId> {
    ant_near(world, cell, true).or_else(|| ant_near(world, cell, false))
}

fn ant_near(world: &World, cell: (i32, i32), liftable: bool) -> Option<OrganismId> {
    let ant = world.species.id_of("ant")?;
    let mut best: Option<(i32, OrganismId)> = None;
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        if st.species != ant || (liftable && !creature::can_carry(world, id)) {
            continue;
        }
        let Some(&(x, y)) = st.chain.first() else { continue };
        let d = (x - cell.0).abs().max((y - cell.1).abs());
        if d <= PICK_R && best.is_none_or(|b| (d, id) < b) {
            best = Some((d, id));
        }
    }
    best.map(|(_, id)| id)
}

/// The edible cell nearest `at` within [`LESSON_R`], if any.
fn food_near(world: &World, at: (i32, i32)) -> Option<(i32, i32)> {
    let mut best: Option<(i32, (i32, i32))> = None;
    for dy in -LESSON_R..=LESSON_R {
        for dx in -LESSON_R..=LESSON_R {
            let c = (at.0 + dx, at.1 + dy);
            if !world.in_bounds(c.0, c.1) {
                continue;
            }
            let cell = world.get(c.0, c.1);
            if cell.material == material::EMPTY || cell.organism_id() != 0 || creature::food_value(world, cell) <= 0.0 {
                continue;
            }
            let d = dx.abs().max(dy.abs());
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, c));
            }
        }
    }
    best.map(|(_, c)| c)
}

/// **Teach `ant` the food near `at`**, unless that food is at home (a lesson
/// at home teaches "here is the nest", and a mind never writes one there
/// itself). Returns where it was taught.
fn teach_near(lab: &mut Lab, ant: OrganismId, at: (i32, i32)) -> Option<(i32, i32)> {
    let food = food_near(&lab.world, at)?;
    if creature::is_home_for(&lab.world, ant, food) {
        return None;
    }
    ensure_recall(lab);
    recall::plant(&mut lab.world, ant, food).then_some(food)
}

/// The pointer over the world; `true` if the keeper seat took it.
pub fn mouse(lab: &mut Lab, cell: (i32, i32), act: super::MouseAct) -> bool {
    let Some(tool) = keeper(lab).map(|k| k.tool) else { return false };
    if tool != Tool::None && matches!(act, super::MouseAct::Press { .. }) && keeper(lab).is_some_and(resting) {
        lab.ui.say(ROUND_OVER.to_string());
        return true;
    }
    match (tool, act) {
        (Tool::None, _) => false,
        (Tool::Carry, super::MouseAct::Press { .. }) => {
            let frame = lab.world.frame;
            if let Some(k) = keeper_mut(lab).filter(|k| limited(k) && k.charges == 0) {
                k.counts.refused += 1;
                let wait = secs(k.recharge_at.saturating_sub(frame));
                lab.ui.say(format!("NO CHARGES LEFT -- ONE COMES BACK IN {wait}S"));
                return true;
            }
            let ant = ant_at(&lab.world, cell);
            if let Some(k) = keeper_mut(lab) {
                k.held = ant;
                k.hand_at = ant.map(|_| cell);
            }
            lab.ui.say(if ant.is_some() { "IN THE HAND -- DRAG IT TO A WINDFALL AND LET GO".to_string() } else { format!("CARRY: NO ANT WITHIN {PICK_R} CELLS OF THE HAND") });
            true
        }
        (Tool::Carry, super::MouseAct::Drag) => {
            if let Some(k) = keeper_mut(lab).filter(|k| k.held.is_some()) {
                k.hand_at = Some(cell);
            }
            true
        }
        (Tool::Carry, super::MouseAct::Release) => {
            let Some(ant) = keeper_mut(lab).and_then(|k| {
                k.hand_at = None;
                k.held.take()
            }) else {
                return true;
            };
            let msg = match hand_carry(lab, ant, cell) {
                Ok((_, Some(f))) => {
                    let k = keeper(lab);
                    let charges = k.map_or(0, |k| k.charges);
                    match k.and_then(|k| k.patches.iter().rev().find(|p| p.live() && (f.0 - p.x).abs() <= PATCH_R)) {
                        Some(p) => format!("SET DOWN -- IT KNOWS THE WINDFALL AT {} ({charges} CHARGES LEFT)", p.x),
                        None => format!("SET DOWN -- IT REMEMBERS THE FOOD AT {}, {}", f.0, f.1),
                    }
                }
                Ok((_, None)) => "SET DOWN AWAY FROM FOOD -- TIRED BY THE LIFT, NOTHING TO EAT".to_string(),
                Err(why) => why,
            };
            lab.ui.say(msg);
            true
        }
        (Tool::Treat, super::MouseAct::Press { .. }) => {
            if let Some(k) = keeper_mut(lab).filter(|k| limited(k) && k.pantry < u64::from(k.rules.treat_cost)) {
                k.counts.refused += 1;
                lab.ui.say("PANTRY EMPTY -- BRING WINDFALL FOOD HOME TO EARN TREATS".to_string());
                return true;
            }
            let msg = match ant_at(&lab.world, cell) {
                None => format!("TREAT: NO ANT WITHIN {PICK_R} CELLS OF THE HAND"),
                Some(ant) => {
                    let Some(food) = lab.world.materials.id_of("provisions") else { return true };
                    let worth = creature::food_value(&lab.world, Cell::new(food, 0));
                    if creature::treat(&mut lab.world, ant, food, worth) {
                        lab.world.log_player_action(format!("KEEPER TREATED ANT {ant:?}"));
                        if let Some(k) = keeper_mut(lab) {
                            k.counts.treated += 1;
                            if limited(k) {
                                k.pantry -= u64::from(k.rules.treat_cost);
                                k.treats_spent += u64::from(k.rules.treat_cost);
                            }
                        }
                        let at = lab.world.organism(ant).and_then(|s| s.chain.first().copied()).unwrap_or(cell);
                        let taught = (!creature::is_home_for(&lab.world, ant, at)).then(|| {
                            ensure_recall(lab);
                            recall::plant(&mut lab.world, ant, at)
                        });
                        if taught == Some(true) {
                            "TREATED -- IT REMEMBERS WHERE".to_string()
                        } else {
                            "TREATED AT HOME -- A MEAL, BUT NOTHING TO REMEMBER".to_string()
                        }
                    } else {
                        "TREAT REFUSED: ITS CROP IS FULL OR HOLDS ANOTHER FOOD".to_string()
                    }
                }
            };
            lab.ui.say(msg);
            true
        }
        (Tool::Treat, _) => true,
    }
}

/// Fixed colours (a full replace, never a blend: CLAUDE.md's debug-readout
/// rule): by generation, white for the player's own hand.
const GEN_COLOURS: [[u8; 4]; 3] = [[255, 255, 255, 255], [255, 176, 32, 255], [230, 40, 40, 255]];
const ASLEEP: [u8; 4] = [110, 110, 110, 255];
/// The traffic ramp, dark to bright; a cell's count picks the step.
const HEAT_RAMP: [[u8; 4]; 5] = [[40, 40, 140, 255], [40, 120, 200, 255], [40, 200, 160, 255], [230, 230, 60, 255], [255, 255, 255, 255]];

fn put_world(lab: &Lab, frame_buf: &mut [u8], (x, y): (i32, i32), colour: [u8; 4]) {
    let hc = super::super::ui::hud_canvas(&lab.renderer);
    if let Some((sx, sy)) = lab.renderer.world_to_logical(x, y) {
        hc.put(frame_buf, sx, sy, colour);
    }
}

/// A square ring of half-width `r` around `at`.
fn ring(lab: &Lab, frame_buf: &mut [u8], at: (i32, i32), r: i32, colour: [u8; 4]) {
    for d in -r..=r {
        put_world(lab, frame_buf, (at.0 + d, at.1 - r), colour);
        put_world(lab, frame_buf, (at.0 + d, at.1 + r), colour);
        put_world(lab, frame_buf, (at.0 - r, at.1 + d), colour);
        put_world(lab, frame_buf, (at.0 + r, at.1 + d), colour);
    }
}

/// Marks over the world: the overlay, the ant in the hand, the station.
pub fn draw_world(lab: &Lab, frame_buf: &mut [u8]) {
    let Some(k) = keeper(lab) else { return };
    match k.overlay {
        Overlay::Off => {}
        Overlay::Knows => {
            if let Some(r) = lab.world.recall.as_ref() {
                for (&id, p) in &r.provenance {
                    let Some(&head) = lab.world.organism(id).and_then(|s| s.chain.first()) else { continue };
                    let colour = if recall::lesson(&lab.world, id).is_some() { GEN_COLOURS[(p.generation as usize).min(2)] } else { ASLEEP };
                    ring(lab, frame_buf, head, 2, colour);
                }
            }
        }
        Overlay::Traffic => {
            let w = super::super::WIDTH as i32;
            for (i, &c) in k.heat.iter().enumerate() {
                if c == 0 {
                    continue;
                }
                let step = ((c as f32).log2() as usize / 2).min(HEAT_RAMP.len() - 1);
                put_world(lab, frame_buf, (i as i32 % w, i as i32 / w), HEAT_RAMP[step]);
            }
        }
    }
    if k.overlay != Overlay::Off && !k.stakes {
        let st = station_place(&lab.world, k);
        ring(lab, frame_buf, st, STATION_R, [80, 220, 80, 255]);
    }
    if k.stakes {
        draw_patches(lab, k, frame_buf);
    }
    if let Some((id, st)) = k.held.and_then(|id| lab.world.organism(id).map(|s| (id, s))) {
        let Some(&head) = st.chain.first() else { return };
        // **The ant rides the hand** (10-30 review: dragging did nothing and
        // the release teleported it). The real ant stays put until the
        // release (`set_down` is the one move); its body is drawn at the
        // pointer, ringed in cyan, with a dim ring where it was picked up.
        // The ring under the pointer turns green over food it would learn.
        ring(lab, frame_buf, head, 3, [40, 110, 130, 255]);
        let at = k.hand_at.unwrap_or(head);
        let body = [255, 230, 90, 255];
        for &(x, y) in &st.chain {
            let (dx, dy) = (x - head.0, y - head.1);
            for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                put_world(lab, frame_buf, (at.0 + 2 * dx + ox, at.1 + 2 * dy + oy), body);
            }
        }
        let learns = food_near(&lab.world, at).is_some_and(|f| !creature::is_home_for(&lab.world, id, f));
        ring(lab, frame_buf, at, 5, if learns { [80, 240, 100, 255] } else { [80, 220, 255, 255] });
    }
}

/// Time-left colours for a windfall: fresh, half gone, about to rot.
const PATCH_FRESH: [u8; 4] = [90, 230, 110, 255];
const PATCH_HALF: [u8; 4] = [240, 200, 60, 255];
const PATCH_LATE: [u8; 4] = [240, 80, 50, 255];
const PATCH_ROT: [u8; 4] = [140, 140, 140, 255];

/// **The windfalls on the box**: a ring around each live one coloured by
/// how long it has left, a bar that empties as it ages, and the seconds and
/// cells home written over it; a stripped or rotted one keeps its verdict
/// written over it for a while.
fn draw_patches(lab: &Lab, k: &KeeperDemo, frame_buf: &mut [u8]) {
    let hc = super::super::ui::hud_canvas(&lab.renderer);
    let frame = lab.world.frame;
    let text = |x: i32, y: i32, s: &str, colour: [u8; 4], frame_buf: &mut [u8]| {
        if let Some((sx, sy)) = lab.renderer.world_to_logical(x, y) {
            hc.text(frame_buf, sx - crate::hud::text_width(s) / 2, sy, s, colour);
        }
    };
    for p in &k.patches {
        let top = p.y - 22;
        match p.ended {
            None => {
                let life = p.rots.saturating_sub(p.landed).max(1);
                let left = p.rots.saturating_sub(frame);
                let frac = left as f32 / life as f32;
                let colour = if frac > 0.5 {
                    PATCH_FRESH
                } else if frac > 0.2 {
                    PATCH_HALF
                } else {
                    PATCH_LATE
                };
                ring(lab, frame_buf, (p.x, p.y - 6), 11, colour);
                let w = (2 * PATCH_R) as f32;
                let fill = (w * frac).round() as i32;
                for dx in 0..=(2 * PATCH_R) {
                    let c = if dx <= fill { colour } else { PATCH_ROT };
                    put_world(lab, frame_buf, (p.x - PATCH_R + dx, top), c);
                    put_world(lab, frame_buf, (p.x - PATCH_R + dx, top + 1), c);
                }
                // Seconds to the rot, and the cells ants already saved from it.
                let taken = p.taken;
                let label = if taken > 0 { format!("{}S  +{taken}", secs(left)) } else { format!("{}S", secs(left)) };
                text(p.x, top - 9, &label, colour, frame_buf);
            }
            Some((at, end)) if frame.saturating_sub(at) < PATCH_NOTE => {
                let (label, colour) = match end {
                    PatchEnd::Stripped => (format!("STRIPPED +{}", p.taken), [255, 255, 255, 255]),
                    PatchEnd::Rotted { left } => (format!("ROTTED -{left}"), PATCH_ROT),
                };
                text(p.x, top - 9, &label, colour, frame_buf);
            }
            Some(_) => {}
        }
    }
}

/// Whether [`draw_world`] drew anything this frame.
pub fn marks_world(lab: &Lab) -> bool {
    keeper(lab).is_some_and(|k| {
        let f = lab.world.frame;
        k.overlay != Overlay::Off || k.held.is_some() || (k.stakes && k.patches.iter().any(|p| p.ended.is_none_or(|(at, _)| f.saturating_sub(at) < PATCH_NOTE)))
    })
}

/// **What the rings show**, counted the way they are drawn: ants with a
/// taught or fed record whose lesson is awake (white, amber, red), and those
/// whose record is asleep (grey). The 10-30 review found the HUD's "KNOW IT
/// 8" beside ~60 rings: it counted strength x freshness >= 0.25 at the
/// station, the rings any record. The HUD now prints this.
pub fn ring_counts(world: &World) -> (usize, usize) {
    let Some(r) = world.recall.as_ref() else { return (0, 0) };
    let mut awake = 0;
    let mut asleep = 0;
    for &id in r.provenance.keys() {
        if world.organism(id).is_none() {
            continue;
        }
        if recall::lesson(world, id).is_some() {
            awake += 1;
        } else {
            asleep += 1;
        }
    }
    (awake, asleep)
}

const WHITE: [u8; 4] = [235, 235, 235, 255];
const DIM: [u8; 4] = [160, 160, 160, 255];
const GOOD: [u8; 4] = [110, 235, 120, 255];
const BAD: [u8; 4] = [255, 110, 80, 255];
const WARN: [u8; 4] = [255, 200, 90, 255];

/// The keeper's panel, under the frame counter.
pub fn draw_hud(lab: &Lab, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    let Some(k) = keeper(lab) else { return };
    if k.stakes {
        draw_stakes_hud(lab, k, hc, frame_buf);
        return;
    }
    let (alive, _, starved) = census(lab);
    let (awake, asleep) = ring_counts(&lab.world);
    let c = k.counts;
    let r = lab.world.recall.as_ref().map(|r| r.counts).unwrap_or_default();
    let first = c.first_station_delivery.map_or("NOT YET".to_string(), |f| format!("FRAME {f}"));
    // Short lines: the panel is 512 logical pixels wide, ~80 glyphs.
    let mut lines = vec![
        (format!("KEEPER (SANDBOX)  Y CARRY  U TREAT  J RINGS  -- HAND: {}", tool_name(k)), WHITE),
        (format!("STATION: {} VISITS ({} TAUGHT OR FED) -- {} CELLS HOME -- FIRST {first}", c.arrivals, c.taught_arrivals, c.station_deliveries), [200, 200, 200, 255]),
        (format!("RINGS: {awake} AWAKE, {asleep} ASLEEP, OF {alive} -- CARRIED {}, TREATED {} -- PASSED ON {}", c.carried, c.treated, r.copies), [200, 200, 200, 255]),
        (format!("RATION {} CELLS -- STARVED {starved}", c.ration_given), [150, 150, 150, 255]),
    ];
    if !k.listening {
        lines.push(("LISTENING OFF (LISTENING=0): TODAY'S ANTS".to_string(), WARN));
    }
    for (i, (line, colour)) in lines.iter().enumerate() {
        hc.text(frame_buf, 4, 16 + 10 * i as i32, line, *colour);
    }
}

fn tool_name(k: &KeeperDemo) -> &'static str {
    match k.tool {
        Tool::None => "NONE",
        Tool::Carry => "CARRY",
        Tool::Treat => "TREAT",
    }
}

/// **The game panel** (stakes): what the hand has, the score, the colony,
/// and the same seed left alone -- four lines, the nest left clear.
fn draw_stakes_hud(lab: &Lab, k: &KeeperDemo, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    let frame = lab.world.frame;
    // **After the end the panel shows the round's result, frozen** (as the
    // card does): a box resumed after a loss runs on, and its live counts
    // beside a "lost" card read as a contradiction.
    let alive = k.ended.map_or_else(|| live_ants(&lab.world), |e| e.ants);
    let (saved, home) = k.ended.map_or((k.counts.saved, k.counts.patch_home), |e| (e.saved, e.home));
    let end = k.ended.map_or(frame, |e| e.frame);
    let left = k.opened.map_or(k.rules.round, |o| (o + k.rules.round).saturating_sub(end));
    let x0 = 4;
    let mut y = 16;
    hc.text(frame_buf, x0, y, &format!("KEEPER  Y CARRY  U TREAT  J RINGS  K CARD  -- HAND: {}", tool_name(k)), WHITE);
    y += 10;
    // The charges as pips: filled is a carry in the hand.
    let label = "CHARGES";
    hc.text(frame_buf, x0, y, label, if k.charges == 0 { WARN } else { WHITE });
    let mut x = x0 + crate::hud::text_width(label) + 5;
    for i in 0..k.rules.charges {
        let full = i < k.charges;
        hc.fill_rect(frame_buf, x, y, 7, 7, if full { [90, 220, 255, 255] } else { [60, 60, 70, 255] });
        x += 10;
    }
    let next = if k.charges < k.rules.charges && k.ended.is_none() { format!("NEXT {}S", secs(k.recharge_at.saturating_sub(frame))) } else { String::new() };
    let rest = format!("{next:<9}  PANTRY {} (A TREAT IS {})   {}:{:02} LEFT", k.pantry, k.rules.treat_cost, secs(left) / 60, secs(left) % 60);
    hc.text(frame_buf, x + 4, y, &rest, WHITE);
    y += 10;
    let danger = alive < k.rules.lose_at + 5;
    hc.text(
        frame_buf,
        x0,
        y,
        &format!("FOOD SAVED {saved} (HOME {home})   ANTS {alive} ({} AT START, LOST UNDER {})", k.ants_at_open, k.rules.lose_at),
        if danger { BAD } else { WHITE },
    );
    y += 10;
    let (twin, colour) = twin_line(k, end, saved);
    hc.text(frame_buf, x0, y, &twin, colour);
    if k.overlay == Overlay::Knows {
        y += 10;
        let (awake, asleep) = ring_counts(&lab.world);
        hc.text(frame_buf, x0, y, &format!("RINGS: {awake} AWAKE (WHITE BY HAND, AMBER/RED FED), {asleep} ASLEEP (GREY)"), DIM);
    }
    if k.card {
        draw_card(lab, hc, frame_buf);
    }
}

/// The HUD's twin line: the same seed left alone, now.
fn twin_line(k: &KeeperDemo, frame: u64, saved: u64) -> (String, [u8; 4]) {
    match twin_at(k, frame) {
        // **Food saved leads** (the verdict's own number); ants follow as
        // the consequence, uncoloured: one difference of two chaotic runs.
        TwinRead::At(t) => {
            let d = saved as i64 - t.saved as i64;
            let colour = match d.signum() {
                1 => GOOD,
                -1 => BAD,
                _ => WHITE,
            };
            (format!("LEFT ALONE: {} SAVED, {} ANTS -- YOUR HAND {} SAVED", t.saved, t.ants, signed(d, "CELL")), colour)
        }
        // Before its first read-out the twin is in its own warm-up.
        TwinRead::Behind { at: 0 } => ("LEFT ALONE: THE TWIN BOX IS STILL WARMING UP".to_string(), DIM),
        TwinRead::Behind { at } => (format!("LEFT ALONE: THE TWIN BOX IS STILL AT FRAME {at}"), DIM),
        TwinRead::Off => ("LEFT ALONE: NOT RUN -- COMPARING WITH THE START".to_string(), DIM),
        TwinRead::Mismatch => ("LEFT ALONE: THE TWIN DID NOT MATCH YOUR BOX -- NOT SHOWN".to_string(), WARN),
        TwinRead::Failed => ("LEFT ALONE: THE TWIN FAILED -- NOT SHOWN".to_string(), WARN),
    }
}

fn signed(d: i64, unit: &str) -> String {
    let s = if d.abs() == 1 { "" } else { "S" };
    if d >= 0 {
        format!("+{d} {unit}{s}")
    } else {
        format!("{d} {unit}{s}")
    }
}

/// **The verdict's grade.** `Lost` is the colony falling under
/// [`Rules::lose_at`]; the rest place the food your ants saved against the
/// band of idle twins ([`TwinLog`]): inside it, give or take
/// [`BAND_SLACK`], they barely noticed you.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    Lost,
    Heavy,
    Clumsy,
    Barely,
    Good,
    Master,
}

impl Grade {
    pub fn words(self) -> &'static str {
        match self {
            Grade::Lost => "THE COLONY IS LOST",
            Grade::Heavy => "A HEAVY HAND",
            Grade::Clumsy => "A CLUMSY KEEPER",
            Grade::Barely => "THEY BARELY NOTICED YOU",
            Grade::Good => "A GOOD KEEPER",
            Grade::Master => "A MASTER KEEPER",
        }
    }

    fn colour(self) -> [u8; 4] {
        match self {
            Grade::Lost | Grade::Heavy => BAD,
            Grade::Clumsy => WARN,
            Grade::Barely => WHITE,
            Grade::Good | Grade::Master => GOOD,
        }
    }
}

/// Cells past either edge of the twins' band still read as "barely
/// noticed": the band is three samples of a wide scatter, so its edges are
/// soft. **The thresholds are from the 10-31 sweep** (trained ant, seeds
/// 25-36; four more knocked idle runs a seed, outside the band, as the
/// no-hand control, n = 48): idle's own food saved spreads a median 149
/// cells across three twins, so one knocked run lands above all three a
/// quarter of the time (12 of 48 called GOOD -- the floor for a band of
/// three, by exchangeability) and the slack barely moves that (15: 12,
/// 40: 11); a master bar of 100 over the top is reached by 1 of 48. Five
/// twins took the no-hand GOOD to 0 of 24 but the scripted good keeper's
/// GOOD-or-better from 8 to 4 of 12, and good-vs-careless from p = 0.016 to
/// 0.125: three it is.
pub const BAND_SLACK: u64 = 25;
/// Cells over the band's top for a master keeper (over it by less, but by
/// more than [`BAND_SLACK`], is a good one). 80 gave the careless hand a
/// MASTER (1 of 12), 100 none, 160 the good keeper none.
pub const MASTER_OVER: u64 = 100;
/// Cells under the band's bottom for a heavy hand (under it by less, but by
/// more than [`BAND_SLACK`], is a clumsy one). The mirror of
/// [`MASTER_OVER`]: no scripted arm fell under the band at all.
pub const HEAVY_UNDER: u64 = 100;

/// **Grade a round**: food your ants saved, against the lowest and highest
/// any idle twin saved on this seed. **Not ants at the end** (10-31 review,
/// fresh seeds): on the trained ant, ants against one idle twin put the
/// scripted good keeper above idle 7 of 12 (p = 0.77) and called a careless
/// hand MASTER 5 of 12; idle's own ants spread 31-88. Food saved is what a
/// hand aims at, and it tracked aim on the review's seeds 13-24 (good beat
/// a random hand 12 of 12); on seeds 25-36 it beat careless 10 of 12 and
/// random 8 of 12 (see the module doc).
pub fn grade(saved: u64, band: (u64, u64), lost: bool) -> Grade {
    let (lo, hi) = band;
    if lost {
        Grade::Lost
    } else if saved >= hi + MASTER_OVER {
        Grade::Master
    } else if saved > hi + BAND_SLACK {
        Grade::Good
    } else if saved + BAND_SLACK >= lo {
        Grade::Barely
    } else if saved + HEAVY_UNDER > lo {
        Grade::Clumsy
    } else {
        Grade::Heavy
    }
}

/// The twins' band of food saved, once every twin has finished (and the
/// exact one matched this box's opening).
pub fn twin_band(k: &KeeperDemo) -> Option<(u64, u64)> {
    let t = k.twin.as_ref()?;
    let l = t.lock().ok()?;
    if !l.done || l.failed.is_some() || l.finals.is_empty() || l.opening != k.opening_print {
        return None;
    }
    let lo = l.finals.iter().map(|p| p.saved).min()?;
    let hi = l.finals.iter().map(|p| p.saved).max()?;
    Some((lo, hi))
}

/// (twins finished, twins asked for), for the card's "waits" line.
fn twins_run(k: &KeeperDemo) -> (usize, u32) {
    let n = k.twin.as_ref().and_then(|t| t.lock().ok().map(|l| l.finals.len())).unwrap_or(0);
    (n, k.twins)
}

/// **The verdict card**: the round against the same seed left alone. At the
/// end it is the result; before it (`K`), the round so far. **Graded on
/// food your ants saved against the idle twins' band** ([`grade`]); ants
/// are its consequence, shown, never graded.
pub fn card_lines(lab: &Lab) -> Vec<(String, [u8; 4])> {
    let Some(k) = keeper(lab) else { return Vec::new() };
    let c = k.counts;
    let (frame, ants, saved) = match k.ended {
        Some(e) => (e.frame, e.ants, e.saved),
        None => (lab.world.frame, live_ants(&lab.world), c.saved),
    };
    let head = match k.ended.map(|e| e.why) {
        Some(Ending::Lost) => "-- THE COLONY IS LOST --",
        Some(Ending::Time) => "-- THE ROUND IS OVER --",
        None => "-- THE ROUND SO FAR --",
    };
    let mut out = vec![(format!("{head}   (K CLOSES)"), WARN)];
    let total: u64 = k.patches.iter().map(|p| u64::from(p.size)).sum();
    out.push((format!("YOUR ANTS SAVED {saved} OF {total} CELLS: {} STRIPPED, {} ROTTED", c.stripped, c.rotted), WHITE));
    let read = twin_at(k, frame);
    let band = k.ended.and_then(|_| twin_band(k));
    match (read, band) {
        (TwinRead::At(_), Some((lo, hi))) => {
            let range = if lo == hi { format!("{lo}") } else { format!("{lo} TO {hi}") };
            out.push((format!("THE SAME SEED LEFT ALONE ({} TRIES): {range} SAVED", k.twins), WHITE));
            let (line, colour) = if saved > hi + BAND_SLACK {
                (format!("YOUR HAND SAVED {} MORE THAN THE BEST OF THEM", saved - hi), GOOD)
            } else if saved + BAND_SLACK < lo {
                (format!("YOUR HAND SAVED {} FEWER THAN THE WORST OF THEM", lo - saved), BAD)
            } else {
                ("WITHIN WHAT THE BOX DOES ALONE".to_string(), WHITE)
            };
            out.push((line, colour));
        }
        (TwinRead::At(t), None) => {
            out.push((format!("THE SAME SEED LEFT ALONE: {} SAVED BY NOW", t.saved), WHITE));
            if k.ended.is_some() {
                let (n, of) = twins_run(k);
                out.push((format!("THE TWIN BOXES ARE STILL RUNNING ({n} OF {of}) -- THE VERDICT WAITS"), DIM));
            } else {
                out.push((format!("YOUR HAND {} SAVED SO FAR", signed(saved as i64 - t.saved as i64, "CELL")), DIM));
            }
        }
        (TwinRead::Behind { at }, _) => out.push((format!("THE TWIN BOX IS STILL RUNNING (FRAME {at}) -- THE VERDICT WAITS"), DIM)),
        (why, _) => {
            let reason = match why {
                TwinRead::Mismatch => "THE TWIN DID NOT MATCH YOUR BOX",
                TwinRead::Failed => "THE TWIN FAILED",
                _ => "NO TWIN RAN",
            };
            out.push((format!("{reason}: NOTHING TO COMPARE WITH"), DIM));
        }
    }
    // The consequence: ants, against the exact twin when there is one.
    let when = if k.ended.is_some() { "AT THE END" } else { "NOW" };
    let alone = match read {
        TwinRead::At(t) => format!(" ({} LEFT ALONE)", t.ants),
        _ => String::new(),
    };
    out.push((format!("ANTS {ants} {when}{alone}, {} AT THE START, {} AT THE WORST", k.ants_at_open, c.min_ants.min(ants)), WHITE));
    out.push((format!("CARRIED {} ANTS ({} TO A WINDFALL), {} TREATS, {} REFUSED", c.carried, c.carried_to_food, c.treated, c.refused), DIM));
    if let Some(e) = k.ended {
        let lost = e.why == Ending::Lost;
        match (band, lost) {
            (Some(b), _) => {
                let g = grade(saved, b, lost);
                out.push((format!("VERDICT: {}", g.words()), g.colour()));
            }
            (None, true) => out.push((format!("VERDICT: {}", Grade::Lost.words()), Grade::Lost.colour())),
            (None, false) if matches!(read, TwinRead::At(_) | TwinRead::Behind { .. }) => {}
            (None, false) => out.push(("VERDICT: NONE -- NO TWIN TO COMPARE WITH".to_string(), DIM)),
        }
    }
    out
}

/// The card, on the right of the box: the nest is at the west end.
fn draw_card(lab: &Lab, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    let lines = card_lines(lab);
    let wpx = lines.iter().map(|(l, _)| crate::hud::text_width(l)).max().unwrap_or(0) + 12;
    let x0 = (super::super::WIDTH as i32 - wpx - 2).max(8);
    let y0 = 72;
    hc.fill_rect(frame_buf, x0 - 6, y0 - 6, wpx, 10 * lines.len() as i32 + 10, [10, 10, 14, 235]);
    for (i, (line, colour)) in lines.iter().enumerate() {
        hc.text(frame_buf, x0, y0 + 10 * i as i32, line, *colour);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo(args: &[(&str, &str)]) -> Lab {
        let sc = crate::lab::scenario::Scenario::load("demo_keeper").expect("demo_keeper ships");
        let mut lab = Lab::new(sc.bed.clone());
        lab.set_new_ant(true);
        let kv = args.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        lab.start_demo_with(super::super::DemoSeat::Keeper, kv);
        lab
    }

    /// **The hand end to end, and a rebuild after it.** CARRY through
    /// [`mouse`] (press on an ant, release beside the station) sets the ant
    /// down there, teaches it the station's food, and counts the carry; the
    /// recall head is on the world at the demo's gain. Then `Lab::reset`
    /// builds a new `World`: the head must come back on the first tick and
    /// the counts start again, or every carry after a rebuild teaches
    /// nothing. Positive controls: the lesson is checked to be *absent*
    /// before the carry, and the head absent straight after the reset.
    #[test]
    fn the_hand_carries_and_teaches_and_survives_a_rebuild() {
        // The sandbox (`stakes=0`): the station this test carries to.
        let mut lab = demo(&[("warmup", "300"), ("stakes", "0")]);
        assert!(lab.world.minds.is_some(), "the demo must run the trained ant");
        assert_eq!(lab.world.recall.as_ref().map(|r| r.gain), Some(DEFAULT_GAIN));
        let (ant, head) = lab
            .world
            .live_organism_ids()
            .into_iter()
            .filter(|&id| creature::can_carry(&lab.world, id))
            .find_map(|id| Some((id, *lab.world.organism(id)?.chain.first()?)))
            .expect("an ant to lift");
        seat_key(&mut lab, 'Y');
        assert!(mouse(&mut lab, head, super::super::MouseAct::Press { erase: false }));
        // The hand takes the nearest liftable head, which in a pile may be a
        // neighbour's: teach whichever it took.
        let ant = keeper(&lab).and_then(|k| k.held).unwrap_or_else(|| panic!("the press beside ant {ant:?} picked nothing"));
        assert!(recall::lesson(&lab.world, ant).is_none(), "taught before the hand touched it");
        let place = station_place(&lab.world, keeper(&lab).expect("keeper"));
        assert!(mouse(&mut lab, (place.0 - 6, place.1 - 2), super::super::MouseAct::Release));
        let at = *lab.world.organism(ant).and_then(|s| s.chain.first()).expect("still alive");
        assert!((at.0 - place.0).abs() <= LESSON_R, "set down at {at:?}, the station is at {place:?}");
        assert!(recall::lesson(&lab.world, ant).is_some(), "set down beside food and taught nothing");
        assert_eq!(counts(&lab).map(|c| c.carried), Some(1));
        lab.reset();
        assert!(lab.world.recall.is_none(), "a rebuilt world kept the old head: this test is not testing the re-apply");
        lab.tick_for_harness();
        assert!(lab.world.recall.is_some(), "the recall head did not come back after a rebuild");
        assert_eq!(counts(&lab).map(|c| c.carried), Some(0), "the counts outlived the box they counted");
    }

    /// **The stakes end to end**: a windfall lands as the round opens; a
    /// carry to it spends a charge and teaches it; with the charges gone the
    /// hand refuses (the positive control that the budget is live); the
    /// windfall rots to ash with no provisions left on it; the round ends
    /// with the card up. And the sandbox (`stakes=0`) drops no windfall, so
    /// `stakes=0` is still today's box.
    #[test]
    fn windfalls_charges_rot_and_the_round_ends() {
        let mut lab = demo(&[("warmup", "300"), ("patch_life", "240"), ("round", "900"), ("charges", "2"), ("recharge", "100000")]);
        let opened = keeper(&lab).and_then(|k| k.opened).expect("the round opened after the warm-up");
        assert_eq!(opened, lab.world.frame);
        lab.tick_for_harness();
        let p = *keeper(&lab).and_then(|k| k.patches.first()).expect("a windfall lands as the round opens");
        assert!(p.size > 0 && (PATCH_MIN_X..=PATCH_MAX_X).contains(&p.x), "{p:?}");
        assert_eq!(patch_food(&lab.world, &p), p.size, "the windfall's cells are not where it says");
        let ants: Vec<OrganismId> = lab.world.live_organism_ids().into_iter().filter(|&id| creature::can_carry(&lab.world, id)).take(3).collect();
        assert_eq!(ants.len(), 3, "three ants to lift");
        for &id in &ants[..2] {
            let (_, taught) = hand_carry(&mut lab, id, (p.x - 3, p.y - 2)).expect("a charged hand carries");
            assert!(taught.is_some(), "set down on the windfall and taught nothing");
        }
        let k = keeper(&lab).expect("keeper");
        assert_eq!((k.charges, k.counts.carried_to_food, k.patches[0].taught), (0, 2, 2));
        assert!(hand_carry(&mut lab, ants[2], (p.x - 3, p.y - 2)).is_err(), "an empty hand still carried");
        assert_eq!(counts(&lab).map(|c| (c.carried, c.refused)), Some((2, 1)));
        while keeper(&lab).is_some_and(|k| k.patches[0].live()) && lab.world.frame <= p.rots + 1 {
            lab.tick_for_harness();
        }
        let p = keeper(&lab).expect("keeper").patches[0];
        assert!(p.ended.is_some(), "the windfall outlived its life: {p:?}");
        assert_eq!(patch_food(&lab.world, &p), 0, "food left on a rotted windfall");
        while keeper(&lab).is_some_and(|k| k.ended.is_none()) && lab.world.frame <= opened + 912 {
            lab.tick_for_harness();
        }
        let k = keeper(&lab).expect("keeper");
        assert!(k.ended.is_some() && k.card, "the round did not end with the card up");
        assert!(card_lines(&lab).iter().any(|(l, _)| l.starts_with("VERDICT")), "the card has no verdict");
        // The hand rests after the round: no free carries on a frozen score.
        let carried = counts(&lab).map(|c| c.carried);
        assert!(hand_carry(&mut lab, ants[2], (p.x - 3, p.y - 2)).is_err(), "the hand carried after the round was over");
        assert_eq!(counts(&lab).map(|c| c.carried), carried);

        let mut sandbox = demo(&[("warmup", "300"), ("stakes", "0")]);
        for _ in 0..60 {
            sandbox.tick_for_harness();
        }
        assert!(keeper(&sandbox).is_some_and(|k| k.patches.is_empty() && k.counts.station_given > 0), "stakes=0 is not the sandbox");
    }

    /// **Food saved counts only what ants took** (10-31 review: an erase arm
    /// read "STRIPPED -- ALL 60 CELLS SAVED" on windfalls no ant reached).
    /// Half a fresh windfall's cells are emptied by no mouth -- as the lab's
    /// erase did -- and the scan must book none of them saved. Positive
    /// control: the cells left *did* fall, so the scan saw the removal.
    #[test]
    fn food_saved_counts_only_what_ants_took() {
        let mut lab = demo(&[("warmup", "300")]);
        lab.tick_for_harness();
        let p = *keeper(&lab).and_then(|k| k.patches.first()).expect("a windfall lands as the round opens");
        let food = lab.world.materials.id_of("provisions").expect("provisions");
        let (x0, x1, y0, y1) = patch_rect(&p);
        let mut emptied = 0;
        for y in y0..=y1 {
            for x in x0..=x1 {
                if emptied < p.size / 2 && lab.world.get(x, y).material == food {
                    lab.world.set(x, y, Cell::EMPTY);
                    emptied += 1;
                }
            }
        }
        for _ in 0..2 * SCAN_EVERY {
            lab.tick_for_harness();
        }
        let q = keeper(&lab).expect("keeper").patches[0];
        assert!(q.left + emptied <= p.size + 2, "the scan did not see the cells go: {q:?} after emptying {emptied}");
        assert_eq!((q.taken, counts(&lab).map(|c| c.saved)), (0, Some(0)), "cells no ant took were booked saved: {q:?}");
    }

    /// **The grade is food saved against the twins' band**, every word
    /// reachable, and a lost colony is lost whatever it saved.
    #[test]
    fn the_grade_reads_food_saved_against_the_band() {
        let band = (200, 260);
        assert_eq!(grade(260 + MASTER_OVER, band, false), Grade::Master);
        assert_eq!(grade(260 + BAND_SLACK + 1, band, false), Grade::Good);
        assert_eq!(grade(260 + BAND_SLACK, band, false), Grade::Barely);
        assert_eq!(grade(200 - BAND_SLACK, band, false), Grade::Barely);
        assert_eq!(grade(200 - BAND_SLACK - 1, band, false), Grade::Clumsy);
        assert_eq!(grade(200 - HEAVY_UNDER, band, false), Grade::Heavy);
        assert_eq!(grade(999, band, true), Grade::Lost);
    }
}
