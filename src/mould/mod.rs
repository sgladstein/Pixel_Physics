//! **Mould: a slime mould you herd with a lamp.** `cargo run --release --bin mould`
//!
//! A probe, not a game (brief: `/mnt/project-files/game-design/new-game-
//! brainstorm-2026-11-07/prototypes/2-mould.md`). The one question it exists
//! to answer is whether herding a slime mould *feels alive or feels like
//! watching*. Everything here is the least that lets a player feel the verb.
//!
//! # The model
//!
//! Jeff Jones's agent model of *Physarum* (2010): a few thousand walkers on a
//! trail field. Each walker senses the trail at three points ahead of it
//! (straight on, and `SENSOR_ANGLE` either side, `SENSOR_OFFSET` cells out),
//! turns toward the strongest, steps one cell, and lays trail where it lands.
//! The trail spreads (a 3x3 mean) and fades. That is the whole of it, and it
//! is enough to draw thick tubes, pull in dead ends and find short paths with
//! no network maths at all.
//!
//! **A private field, not `sim::pheromone`.** The engine's fine trail plane
//! was sized against a Jones follower, but it is world-sized, chunk-rect
//! bound, `u16`, and stepped on the world's interval; a dish is a bounded
//! 256x160 box of floats that nothing else reads. Keeping the field here
//! means this probe touches **no shared engine file at all**, so it cannot
//! move a lab, outdoor or held-world result, and the brief's next step
//! (the mould half of "Seep", which eats gas) can lift this module whole.
//!
//! # What the game adds to Jones
//!
//! * **The food store is the population.** One shared store pays for every
//!   new walker and for every walker's upkeep. Walkers on a food pile refill
//!   it. When the store is empty, walkers die, the ones farthest from food
//!   first.
//! * **Food only flows through tubes.** Every few steps a breadth-first search
//!   runs from each food pile the mould is touching, out along cells whose
//!   trail is thick enough to count as tube. A walker more than `FEED_REACH`
//!   cells of tube from food is *unfed*: it cannot divide, it slowly dies,
//!   and its tube is drawn pale. So the mould can only stretch so far from
//!   food, and a cut tube starves what is beyond it, visibly, over seconds:
//!   a graded death, never a disappearance.
//! * **The tools** each act on the model's own levers: the Lamp pushes
//!   (it carries trail and walkers outward from its centre, so a tube
//!   bends aside rather than being erased; see `push_light`), Oats are a small
//!   food pile (a steady trail deposit that also pays the store), Salt is
//!   ground a walker cannot step on, and Snip kills walkers and clears trail
//!   along a stroke.
//! * **The storm** at the end of a dish cuts one tube: the one whose loss
//!   costs the most food (see `aim_storm`; one cut, see `STORM_CUTS` for why
//!   not three). Its line is shown as a shadow `STORM_WARNING_SECONDS`
//!   ahead, and toughness is read after the full watch, once regrowth has
//!   had its chance.
//!
//! # Dishes
//!
//! The window opens on the lesson (`lesson.rs`). After it come the
//! hand-made challenge dishes (`level.rs`: Behind the Wall, Beat the Ruler,
//! Running Dry), each with its own food, salt, tools, clock and win rule and
//! no storm, and the seeded free dishes above. M in the window is the dish
//! picker; a dish opens on a star from the one before.

pub mod lesson;
pub mod level;
pub mod organ;
pub mod play;
pub mod render;
pub mod wander;

use std::f32::consts::TAU;

/// The dish, in cells. The brief's "size that reads on a phone".
pub const W: usize = 256;
pub const H: usize = 160;

/// Simulation steps per second of play. The window runs
/// `STEPS_PER_FRAME` steps per drawn frame at 60 frames a second.
pub const STEPS_PER_FRAME: u32 = 2;
pub const STEPS_PER_SECOND: u32 = 60 * STEPS_PER_FRAME;

/// Steps from the start of a dish to the storm, and from the storm to the
/// final score.
pub const DISH_SECONDS: u32 = 90;
pub const STORM_WATCH_SECONDS: u32 = 8;

// ---------------------------------------------------------------- tunables

/// Every number the mould's behaviour hangs off, in one place, so the probe
/// harness can sweep them without a rebuild. The defaults are the shipped
/// ones; see each field.
#[derive(Clone, Debug)]
pub struct Params {
    /// Jones's sensor angle and rotation angle, radians.
    pub sensor_angle: f32,
    pub rotate_angle: f32,
    /// How far ahead the three sensors sit, in cells.
    pub sensor_offset: f32,
    /// Trail one walker lays per step where it lands.
    pub deposit: f32,
    /// Fraction of trail kept per step, after the spread.
    pub keep: f32,
    /// Scent a food cell gives off per step. Scent is a second field beside
    /// the trail, summed with it in every sensor: kept apart so that the
    /// halo round a pile is neither drawn as tube nor counted as one.
    pub food_deposit: f32,
    /// Fraction of scent kept per step. Higher than `keep`, so food is felt
    /// from beyond a walker's own sensors.
    pub scent_keep: f32,
    /// How strongly light outweighs trail in a sensor. At 1.0 light, a
    /// sensor reads `trail - light_weight`. **0 since the 2026-11-08
    /// playtest** (was 400): the lamp now moves the mould (`push`) instead of
    /// repelling it, and every sensor weight tried on top of the push made a
    /// lamp held over a tube part it more often (weight 15: 4 of 5 trunks
    /// parted, against 2 of 5 at 0).
    pub light_weight: f32,
    /// Fraction of trail a fully lit cell loses per step, on top of `keep`.
    /// 0 since the 2026-11-08 playtest (was 0.25): the lamp must not erase.
    pub light_fade: f32,
    /// Trail above which a cell counts as tube -- for feeding, for the score
    /// and for drawing it fed or pale.
    pub tube: f32,
    /// Cells of tube from food beyond which a walker is unfed.
    pub feed_reach: u16,
    /// Store spent per walker per step.
    pub upkeep: f32,
    /// Store gained per walker standing on food, per step.
    pub eat: f32,
    /// Most walkers one pile can feed at once. This is what makes food
    /// joined the same thing as mould alive: one pile feeds
    /// `table * eat / upkeep` walkers and no more.
    pub table: u32,
    /// Fraction of its scent a pile keeps giving off once the mould is on
    /// it. Without this every walker is pulled back onto the first pile and
    /// the mould never explores.
    pub fed_scent: f32,
    /// Chance per step that a fed walker tries to divide.
    pub divide: f32,
    /// Most walkers a 5x5 window may already hold for a division to happen
    /// there (the walker itself included) -- Jones's growth window, which is
    /// what keeps growth at the edges of the mould.
    pub crowd: u32,
    /// Chance per step that an unfed walker dies.
    pub starve: f32,
    /// Chance per step that a walker dies while the store is empty, scaled
    /// up with its distance from food.
    pub hungry: f32,
    /// Chance per step that a walker in full light dies. 0 since the
    /// 2026-11-08 playtest (was 1/400), for the same reason as `light_fade`.
    pub burn: f32,
    /// How far the lamp's light reaches, in cells. Full brightness inside
    /// half of it, falling to nothing at the edge.
    pub lamp_reach: f32,
    /// Light falls off all the way from the centre, instead of a flat core.
    pub lamp_soft: bool,
    /// How hard the light shoves: the fraction of trail in fully lit cells
    /// carried one cell outward per step, and the chance per step that a
    /// fully lit walker is stepped one cell outward. 0 is the sensor-only
    /// lamp, which repels but cannot move a tube (see `push_light`).
    pub push: f32,
    /// **Each piece of mould eats only the food it is joined to** (owner's
    /// card, 2026-11-08: "Yes, each piece eats its own food"). Off, one
    /// shared store paid for every walker on the dish, so a piece cut away
    /// from most of the food was fed by food it no longer touched, and a cut
    /// cost nothing. On, every walker carries its share of its piece's food
    /// (`Walker::food`), a piece is the tube joined along the feeding search
    /// (`Dish::piece_of`), and when a piece's food runs out its walkers go
    /// hungry, farthest from food first -- the same rule the shared store
    /// used, per piece. Food is never made or lost by a split or a join:
    /// shares travel with their walkers.
    pub pieces: bool,
    /// Seconds a snip stays open: the cut cells are wound, which the mould
    /// cannot cross, then heal back to agar. 0 is the old snip, which only
    /// wiped trail and was healed by the walkers in about 0.5 s, so nothing
    /// beyond it ever went grey (toy-feel brainstorm, 2026-11-08: 0 grey
    /// tube pixels at 30, 60 and 82 s of a whole scripted game).
    pub wound: f32,
    /// **How near fed mould a stray can start eating food**, in cells. A
    /// stray is a walker on unfed ground: off the leash it can still walk
    /// (a cell a step, about 5 s to live), and a pile it stands on becomes
    /// a source of the feeding search like any other. Infinite (the seeded
    /// dishes, which were balanced with it): a stray anywhere colonises any
    /// food. Finite (every hand-made dish, `level.rs`): food more than this
    /// far from fed mould can only be started on by fed mould, so the mould
    /// reaches far food by growing tube to it, and the leash means what the
    /// card says. With no limit, strays broke two dishes' premise: in Beat
    /// the Ruler they reached both far foods and doing nothing joined all
    /// three on 2 of 2 seeds; on Behind the Wall a random hand's stray oat
    /// across the dish became a beachhead and it scored 3 stars. Shut out
    /// entirely (0), an oat even a little past the leash could never be
    /// eaten, so the relays every dish needs died: Running Dry's 86-cell
    /// hop failed with three oats lying round its grey front. 24 is an oat
    /// laid one step ahead (the hands lay them 22 out).
    pub stray_reach: f32,
    /// **Seconds mould still counts as fed for the stray rule** after its
    /// food ran out (`Dish::fed_within`), so oats laid on mould that has
    /// only just gone grey can be eaten. Read only with a finite
    /// `stray_reach`, and only once no cell on the dish is fed. Swept over 8
    /// seeds of Running Dry (`mouldprobe level=3 stray_grace=G`), every hand
    /// scores exactly as under the old rule at 1, 2 and 3 s, and at 5 s a
    /// random hand's oat on its dying mould took 3 stars (seed 4). 2 is the
    /// middle of the range that changes nothing a hand scores and still
    /// rescues: oats laid as the last pile runs dry are eaten. (The random
    /// hand's 3 stars on seed 7 is the old rule's too: 0 s scores the same.)
    pub stray_grace: f32,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            sensor_angle: 45f32.to_radians(),
            rotate_angle: 45f32.to_radians(),
            sensor_offset: 9.0,
            deposit: 5.0,
            keep: 0.90,
            food_deposit: 20.0,
            scent_keep: 0.97,
            light_weight: 0.0,
            light_fade: 0.0,
            tube: 1.0,
            feed_reach: 70,
            upkeep: 0.004,
            eat: 0.03,
            table: 40,
            fed_scent: 0.15,
            divide: 0.03,
            crowd: 7,
            starve: 1.0 / 600.0,
            hungry: 1.0 / 900.0,
            burn: 0.0,
            lamp_reach: LAMP_R,
            lamp_soft: false,
            push: 0.9,
            pieces: true,
            wound: 4.0,
            stray_reach: f32::INFINITY,
            stray_grace: 2.0,
        }
    }
}

/// Food in each of the dish's piles, and in the mould's store at the start.
pub const PILE_FOOD: f32 = 20_000.0;
pub const START_STORE: f32 = 1_500.0;
/// Lamp radius in cells: about a fingertip at phone size.
pub const LAMP_R: f32 = 16.0;
/// Oats: how many per dish, how much food each holds, how long it lasts.
pub const OATS_STOCK: u32 = 5;
pub const OATS_FOOD: f32 = 400.0;
pub const OATS_R: f32 = 3.0;
pub const OATS_SECONDS: u32 = 35;
/// **Oats come back**: one more every this many seconds, up to the stock.
/// The 2026-11-08 playtest found a fresh player had spent all five by about
/// 20 s and then had nothing to decide for a minute ("28 of 35 runs joined
/// the same food at 30 s as at 75 s").
pub const OATS_REGROW_SECONDS: u32 = 15;
/// **Food that turns up partway**, for the same reason: the farthest pile
/// appears at the first time, and a sixth pile somewhere new at the second.
pub const LATE_FOOD_SECONDS: [u32; 2] = [30, 55];
/// The storm's warning: a shadow line over the tube it will strike, this many
/// seconds ahead, so a player can lead the mould off it or build a bypass.
pub const STORM_WARNING_SECONDS: u32 = 5;
/// Salt: one line per dish, this many cells long.
pub const SALT_STOCK: u32 = 1;
pub const SALT_LEN: f32 = 60.0;
/// Snip: this many strokes, each up to this long.
pub const SNIP_STOCK: u32 = 3;
pub const SNIP_LEN: f32 = 30.0;
/// The storm's cuts: how many, and how far each scar runs either side of
/// the tube it crosses. A disc-shaped cut was tried first and the mould
/// simply flowed round it within the storm's eight seconds (every run scored
/// the toughness star, tree or not), so a cut is a line across the tube.
///
/// **One cut, not the brief's three, and the reason is measured.** Over 5
/// seeds x {do nothing, random hand, simple plan, plan with 10 oats that
/// spends the spares on loops}, the toughness star ("nothing lost") was
/// earned in **0 of 20** runs at three cuts and 0 of 20 at two: three cuts
/// split even a looped five-pile network. At one cut it was earned 4 of 20
/// (never by doing nothing), which is a star a player can aim for. It is
/// also the synthesis review's graft: one strike that picks the trunk, so a
/// tree loses a region and a loop shrugs it off.
pub const STORM_CUTS: usize = 1;
pub const CUT_HALF: f32 = 20.0;
/// The thrift star: the network must be no more than this many times the
/// shortest possible length, and join at least `THRIFT_MIN_JOINED` piles (a
/// lone tube is thrifty by default). Set from the probe's first census,
/// where clean single tubes read 1.0-1.3x and branchy networks 1.6-1.9x.
pub const THRIFT_BAR: f32 = 1.15;
pub const THRIFT_MIN_JOINED: u32 = 3;
/// Cells of tube width that make one cell of network length. Calibrated on
/// the probe's straight home-to-pile tubes, which are about 12 cells wide at
/// the `tube` threshold; at 4 a straight tube read as 3.8x its own length.
pub const TUBE_WIDTH: f32 = 12.0;
/// Drawing only (`Dish::looks_fed`): how far beside fed tube, and for how
/// long after, a tube cell still shows fed.
pub const FED_LOOK_REACH: usize = 1;
pub const FED_LOOK_SECONDS: f32 = 0.5;
/// How often (steps) the feeding search and pile census run.
const FED_EVERY: u64 = 6;
/// A join between two piles flashes only if they were not joined within
/// this long: a thin tube flickering across the tube threshold would
/// otherwise snap every census.
pub const SNAP_FORGET_SECONDS: f32 = 2.0;

/// A pile's identity across censuses: its position. Piles never move, and
/// oats are refused on a cell that is not open agar.
fn pile_key(p: &Pile) -> u64 {
    ((p.x.to_bits() as u64) << 32) | p.y.to_bits() as u64
}

/// A straight stroke across the dish, end to end, in cells: a storm cut,
/// a salt line, a snip.
pub type Segment = ((f32, f32), (f32, f32));

// --------------------------------------------------------------------- rng

/// splitmix64. Own, so the module stays self-contained and a seed means the
/// same dish in every build.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// Uniform in 0..n.
    pub fn below(&mut self, n: u32) -> u32 {
        (((self.next_u64() >> 32) * n as u64) >> 32) as u32
    }
}

// ------------------------------------------------------------------- world

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ground {
    /// Open dish.
    Agar,
    /// Outside the dish.
    Rim,
    /// Player's salt line.
    Salt,
    /// Where the storm struck.
    Scar,
    /// A snip, open for `Params::wound` seconds, then agar again.
    Wound,
}

#[derive(Clone, Debug)]
pub struct Pile {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub food: f32,
    pub full: f32,
    /// Player's oats, rather than one of the dish's food piles. Oats lure and
    /// pay, but the score only counts the dish's piles.
    pub oats: bool,
    /// Step at which oats go off. Dish piles never do.
    pub expires: u64,
    /// Walkers on it at the last census.
    pub eaters: u32,
}

impl Pile {
    pub fn reached(&self) -> bool {
        self.eaters > 0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Walker {
    pub x: f32,
    pub y: f32,
    pub heading: f32,
    /// This walker's share of its piece's food (`Params::pieces`). Unused
    /// with the shared store.
    pub food: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Lamp,
    Oats,
    Salt,
    Snip,
}

impl Tool {
    pub const ALL: [Tool; 4] = [Tool::Lamp, Tool::Oats, Tool::Salt, Tool::Snip];
    pub fn name(self) -> &'static str {
        match self {
            Tool::Lamp => "LAMP",
            Tool::Oats => "OATS",
            Tool::Salt => "SALT",
            Tool::Snip => "SNIP",
        }
    }
    /// What pressing the mouse will do, in world words, for the hint line.
    pub fn hint(self) -> &'static str {
        match self {
            Tool::Lamp => "HOLD THE MOUSE: LIGHT PUSHES THE MOULD",
            Tool::Oats => "CLICK: OATS. THE MOULD GROWS TOWARD THEM",
            Tool::Salt => "DRAG: A SALT WALL THE MOULD CAN'T CROSS",
            Tool::Snip => "DRAG ACROSS A TUBE TO CUT IT",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Growing,
    /// The storm struck at this step.
    Storm(u64),
    /// The Wanderer pressed Go at this step and is pouring through the
    /// crack (`wander.rs`).
    Crossing(u64),
    Over,
}

/// What a stroke tool has left in the current stroke.
#[derive(Clone, Copy, Debug)]
struct Stroke {
    tool: Tool,
    last: (f32, f32),
    left: f32,
}

/// The score of a dish, read once before the storm and once after.
#[derive(Clone, Debug, Default)]
pub struct Score {
    /// Dish piles in the best-joined piece of mould, before the storm.
    pub reach: u32,
    pub piles: u32,
    /// Network length in cells (tube cells / `TUBE_WIDTH`), all tube counted.
    pub length: f32,
    /// Shortest possible network joining the piles that are joined.
    pub shortest: f32,
    /// Dish piles still joined after the storm.
    pub after: u32,
    pub stars: [bool; 3],
}

impl Score {
    pub fn star_count(&self) -> u32 {
        self.stars.iter().filter(|s| **s).count() as u32
    }
    /// Length over shortest: 1.0 is perfect, bigger is wasteful.
    pub fn ratio(&self) -> f32 {
        if self.shortest > 0.0 {
            self.length / self.shortest
        } else {
            0.0
        }
    }
}

/// Things that happened, counted, so a picture can carry its numbers.
#[derive(Clone, Debug, Default)]
pub struct Counts {
    pub births: u64,
    pub starved: u64,
    pub hungry: u64,
    pub burned: u64,
    pub snipped: u64,
    pub stormed: u64,
    pub oats_placed: u32,
    pub snips_used: u32,
    pub salt_cells: u32,
    pub lamp_steps: u64,
    /// Joins flashed (`Dish::snaps`).
    pub snaps: u32,
    /// Wound cells opened by snips.
    pub wound_cells: u32,
}

#[derive(Clone)]
pub struct Dish {
    pub params: Params,
    pub seed: u64,
    pub t: u64,
    pub ground: Vec<Ground>,
    pub trail: Vec<f32>,
    /// Food scent: laid by piles and oats, never by walkers.
    pub scent: Vec<f32>,
    scratch: Vec<f32>,
    pub light: Vec<f32>,
    light_box: Option<(usize, usize, usize, usize)>,
    occ: Vec<u8>,
    pub walkers: Vec<Walker>,
    order: Vec<u32>,
    pub piles: Vec<Pile>,
    /// Tube distance to the nearest food the mould is touching; `u16::MAX`
    /// where no tube reaches.
    pub fed: Vec<u16>,
    /// The step at which each cell was last on, or beside, fed tube
    /// (`u64::MAX`: never). Drawing only: see `Dish::looks_fed`.
    pub fed_at: Vec<u64>,
    pub store: f32,
    pub lamp: Option<(f32, f32)>,
    pub oats_left: u32,
    pub salt_left: u32,
    pub snips_left: u32,
    stroke: Option<Stroke>,
    pub phase: Phase,
    pub before: Option<Score>,
    pub score: Option<Score>,
    /// Food joined just after the storm.
    after: Option<u32>,
    /// Each storm cut, as the two ends of its scar.
    pub cuts: Vec<((f32, f32), (f32, f32))>,
    /// Food piles still to come, with the step each appears at.
    pub pending: Vec<(u64, Pile)>,
    /// Where the storm will strike, once it has been aimed.
    pub storm_line: Option<((f32, f32), (f32, f32))>,
    /// Piles that have turned up, with the step, for the arrival flash.
    pub arrived: Vec<(f32, f32, u64)>,
    /// The step the storm strikes at. `Dish::dish_steps()` in a normal dish;
    /// the lesson (`lesson.rs`) holds it at `u64::MAX` until its last step.
    pub storm_at: u64,
    /// Steps before `storm_at` that the storm is aimed and its line shown:
    /// `STORM_WARNING_SECONDS` in a dish; the lesson's whole storm step.
    pub storm_warning: u64,
    /// Recent snip strokes, for drawing a mark that fades: (x, y, step).
    pub snip_marks: Vec<(f32, f32, u64)>,
    /// Which pile (index at the last census) each fed cell's food comes
    /// from; meaningless where `fed` is `u16::MAX`.
    fed_src: Vec<u16>,
    /// The piece each census pile belongs to, by census pile index.
    piece_of_src: Vec<u16>,
    /// Each census pile's identity (`pile_key`), by census pile index, so a
    /// pile added or gone since the census is never confused for another.
    census_keys: Vec<u64>,
    /// Pieces at the last census.
    pub piece_count: usize,
    /// When each pair of piles (by `pile_key`, smaller first) was last
    /// joined into one piece: what keeps a tube that flickers across the
    /// tube threshold from snapping every census.
    last_joined: std::collections::BTreeMap<(u64, u64), u64>,
    /// The piles being eaten at the last census (by `pile_key`) and the
    /// piece each was in: a join flashes only between two of these that
    /// were in different pieces (`join_pieces`).
    eaten_pieces: Vec<(u64, u16)>,
    /// Joins: where two pieces met and became one, and the step. Drawn as a
    /// flash; the owner's favourite moment, "little bridges where there
    /// wasn't even food", made an event the game can see and count.
    pub snaps: Vec<(f32, f32, u64)>,
    /// Open wounds: cell and the step it heals.
    wounds: Vec<(usize, u64)>,
    /// The step each wound cell heals (`u64::MAX` where none), for drawing.
    pub wound_until: Vec<u64>,
    pub counts: Counts,
    /// The hand-made dish this is (`level.rs`), or `None` for a seeded free
    /// dish and the lesson.
    pub level: Option<&'static level::Level>,
    /// Most oats in hand, and seconds per oat coming back (0: never). The
    /// free dish's are `OATS_STOCK` and `OATS_REGROW_SECONDS`; a level sets
    /// its own.
    pub oats_stock: u32,
    pub oats_regrow: u32,
    /// The score card's words once the dish is over: the amount and the
    /// one line of why (`level::Card`).
    pub card: Option<level::Card>,
    /// Beat the Ruler's live number, refreshed twice a second: the
    /// shortest join through the tube over the ruler (`Dish::best_join`).
    pub join_now: Option<f32>,
    /// The Wanderer mode (`wander.rs`): an exit on the right and a Go key in
    /// place of the clock, the storm and the stars. `None` in a scored dish
    /// and the lesson.
    pub wander: Option<wander::Wander>,
    rng: Rng,
}

#[inline]
fn idx(x: usize, y: usize) -> usize {
    y * W + x
}

#[inline]
fn cell(x: f32, y: f32) -> Option<usize> {
    if x < 0.0 || y < 0.0 {
        return None;
    }
    let (ix, iy) = (x as usize, y as usize);
    if ix >= W || iy >= H {
        None
    } else {
        Some(idx(ix, iy))
    }
}

/// The dish's food piles, home first, jittered by seed so paired seeds are
/// different dishes of the same shape.
fn layout(rng: &mut Rng) -> Vec<(f32, f32, f32)> {
    let base: [(f32, f32, f32); 5] = [
        (40.0, 82.0, 7.0),   // home: where the mould starts
        (96.0, 36.0, 6.0),   // near
        (118.0, 124.0, 6.0), // near, the other way
        (188.0, 60.0, 6.0),  // far
        (220.0, 118.0, 6.0), // farthest
    ];
    base.iter()
        .enumerate()
        .map(|(i, &(x, y, r))| {
            if i == 0 {
                (x, y, r)
            } else {
                let j = 8.0;
                (x + (rng.f32() - 0.5) * 2.0 * j, y + (rng.f32() - 0.5) * 2.0 * j, r)
            }
        })
        .collect()
}

/// The empty dish: agar inside an ellipse a few cells in from the edge, rim
/// outside it.
fn agar_ground() -> Vec<Ground> {
    let mut ground = vec![Ground::Rim; W * H];
    let (cx, cy, rx, ry) = (
        W as f32 / 2.0,
        H as f32 / 2.0,
        W as f32 / 2.0 - 3.0,
        H as f32 / 2.0 - 3.0,
    );
    for y in 0..H {
        for x in 0..W {
            let dx = (x as f32 + 0.5 - cx) / rx;
            let dy = (y as f32 + 0.5 - cy) / ry;
            if dx * dx + dy * dy <= 1.0 {
                ground[idx(x, y)] = Ground::Agar;
            }
        }
    }
    ground
}

impl Dish {
    pub fn new(seed: u64) -> Self {
        Self::with_params(seed, Params::default())
    }

    pub fn with_params(seed: u64, params: Params) -> Self {
        Self::build(seed, params, false)
    }

    /// `wander`: keep the sixth pile's late spot off the exit's face (the
    /// spot at (232, 80) sits in the crack), so reaching the way on always
    /// needs a bridge. A scored dish draws the same numbers either way.
    fn build(seed: u64, params: Params, wander: bool) -> Self {
        let mut rng = Rng::new(seed);
        let ground = agar_ground();
        let mut piles = layout(&mut rng)
            .into_iter()
            .map(|(x, y, r)| Pile {
                x,
                y,
                r,
                food: PILE_FOOD,
                full: PILE_FOOD,
                oats: false,
                expires: u64::MAX,
                eaters: 0,
            })
            .collect::<Vec<_>>();
        // Late food: the farthest pile waits, and a sixth goes wherever is
        // far from all the others.
        let s = STEPS_PER_SECOND as u64;
        let mut pending = Vec::new();
        let far = piles.remove(4);
        pending.push((LATE_FOOD_SECONDS[0] as u64 * s, far.clone()));
        let spots = [
            (150.0f32, 24.0f32),
            (160.0, 140.0),
            (70.0, 138.0),
            (232.0, 80.0),
            (60.0, 30.0),
        ];
        let mut order: Vec<usize> = (0..spots.len()).collect();
        for k in (1..order.len()).rev() {
            let j = rng.below(k as u32 + 1) as usize;
            order.swap(k, j);
        }
        let taken: Vec<(f32, f32)> = piles.iter().chain(std::iter::once(&far)).map(|p| (p.x, p.y)).collect();
        if let Some(&k) = order
            .iter()
            .find(|&&k| {
                taken.iter().all(|t| (t.0 - spots[k].0).hypot(t.1 - spots[k].1) >= 45.0)
                    && !(wander && spots[k].0 > 215.0)
            })
        {
            let (x, y) = spots[k];
            pending.push((LATE_FOOD_SECONDS[1] as u64 * s, Pile { x, y, ..far.clone() }));
        }
        Self::assemble(seed, params, rng, ground, piles, pending)
    }

    /// The dish around a ground and its food: the fields, the inoculum on
    /// the first pile, and the first census. Shared by the seeded dishes
    /// and `Dish::from_level`.
    fn assemble(
        seed: u64,
        params: Params,
        rng: Rng,
        ground: Vec<Ground>,
        piles: Vec<Pile>,
        pending: Vec<(u64, Pile)>,
    ) -> Self {
        let mut dish = Dish {
            params,
            seed,
            t: 0,
            ground,
            trail: vec![0.0; W * H],
            scent: vec![0.0; W * H],
            scratch: vec![0.0; W * H],
            light: vec![0.0; W * H],
            light_box: None,
            occ: vec![0; W * H],
            walkers: Vec::new(),
            order: Vec::new(),
            piles,
            fed: vec![u16::MAX; W * H],
            fed_at: vec![u64::MAX; W * H],
            store: START_STORE,
            lamp: None,
            oats_left: OATS_STOCK,
            pending,
            storm_line: None,
            arrived: Vec::new(),
            storm_at: Self::dish_steps(),
            storm_warning: (STORM_WARNING_SECONDS * STEPS_PER_SECOND) as u64,
            salt_left: SALT_STOCK,
            snips_left: SNIP_STOCK,
            stroke: None,
            phase: Phase::Growing,
            before: None,
            score: None,
            after: None,
            cuts: Vec::new(),
            snip_marks: Vec::new(),
            fed_src: vec![0; W * H],
            piece_of_src: Vec::new(),
            census_keys: Vec::new(),
            eaten_pieces: Vec::new(),
            piece_count: 0,
            last_joined: std::collections::BTreeMap::new(),
            snaps: Vec::new(),
            wounds: Vec::new(),
            wound_until: vec![u64::MAX; W * H],
            counts: Counts::default(),
            level: None,
            oats_stock: OATS_STOCK,
            oats_regrow: OATS_REGROW_SECONDS,
            card: None,
            join_now: None,
            wander: None,
            rng,
        };
        // The inoculum: a blob on the home pile.
        let (hx, hy) = (dish.piles[0].x, dish.piles[0].y);
        let mut placed = 0;
        let mut tries = 0;
        while placed < 400 && tries < 20_000 {
            tries += 1;
            let a = dish.rng.f32() * TAU;
            let d = dish.rng.f32().sqrt() * 12.0;
            let (x, y) = (hx + a.cos() * d, hy + a.sin() * d);
            if let Some(i) = cell(x, y) {
                if dish.ground[i] == Ground::Agar && dish.occ[i] == 0 {
                    dish.occ[i] = 1;
                    let heading = dish.rng.f32() * TAU;
                    dish.walkers.push(Walker { x, y, heading, food: 0.0 });
                    placed += 1;
                }
            }
        }
        if dish.params.pieces {
            let share = START_STORE / dish.walkers.len().max(1) as f32;
            for w in dish.walkers.iter_mut() {
                w.food = share;
            }
        }
        dish.census();
        dish
    }

    pub fn dish_steps() -> u64 {
        (DISH_SECONDS * STEPS_PER_SECOND) as u64
    }

    pub fn storm_steps() -> u64 {
        (STORM_WATCH_SECONDS * STEPS_PER_SECOND) as u64
    }

    /// Seconds until the storm, or 0 once it has come.
    pub fn seconds_left(&self) -> u32 {
        let end = self.storm_at;
        if self.t >= end {
            0
        } else {
            ((end - self.t).min(u32::MAX as u64) as u32).div_ceil(STEPS_PER_SECOND)
        }
    }

    pub fn dish_piles(&self) -> impl Iterator<Item = &Pile> {
        self.piles.iter().filter(|p| !p.oats)
    }

    // ----------------------------------------------------------- the tools

    pub fn set_lamp(&mut self, at: Option<(f32, f32)>) {
        self.lamp = at;
    }

    /// Drop oats. False if there are none left or the spot is not open dish.
    pub fn place_oats(&mut self, x: f32, y: f32) -> bool {
        if self.oats_left == 0 || self.phase != Phase::Growing {
            return false;
        }
        match cell(x, y) {
            Some(i) if self.ground[i] == Ground::Agar => {}
            _ => return false,
        }
        self.oats_left -= 1;
        self.counts.oats_placed += 1;
        let expires = self.t + (OATS_SECONDS * STEPS_PER_SECOND) as u64;
        self.piles.push(Pile {
            x,
            y,
            r: OATS_R,
            food: OATS_FOOD,
            full: OATS_FOOD,
            oats: true,
            expires,
            eaters: 0,
        });
        true
    }

    /// Start a salt or snip stroke. False if that tool is spent.
    pub fn begin_stroke(&mut self, tool: Tool, x: f32, y: f32) -> bool {
        let (left, len) = match tool {
            Tool::Salt => (&mut self.salt_left, SALT_LEN),
            Tool::Snip => (&mut self.snips_left, SNIP_LEN),
            _ => return false,
        };
        // Strokes are for the growing dish only: a snip during the storm
        // watch cut the player's own network and cost toughness, while oats
        // were already refused there (bug check, 2026-11-08).
        if *left == 0 || self.phase != Phase::Growing {
            return false;
        }
        *left -= 1;
        if tool == Tool::Snip {
            self.counts.snips_used += 1;
        }
        self.stroke = Some(Stroke {
            tool,
            last: (x, y),
            left: len,
        });
        self.stroke_dab(tool, x, y);
        true
    }

    pub fn drag_stroke(&mut self, x: f32, y: f32) {
        let Some(mut s) = self.stroke else { return };
        let (dx, dy) = (x - s.last.0, y - s.last.1);
        let d = (dx * dx + dy * dy).sqrt();
        if d < 0.5 {
            return;
        }
        let go = d.min(s.left);
        let n = go.ceil() as usize;
        for k in 1..=n {
            let f = k as f32 / n as f32 * go / d;
            self.stroke_dab(s.tool, s.last.0 + dx * f, s.last.1 + dy * f);
        }
        s.left -= go;
        s.last = (s.last.0 + dx * go / d, s.last.1 + dy * go / d);
        self.stroke = if s.left > 0.0 { Some(s) } else { None };
    }

    pub fn end_stroke(&mut self) {
        self.stroke = None;
    }

    pub fn stroke_active(&self) -> bool {
        self.stroke.is_some()
    }

    fn stroke_dab(&mut self, tool: Tool, x: f32, y: f32) {
        match tool {
            Tool::Salt => {
                // Three cells wide: thin things are drawn 2-3 cells wide.
                for (ox, oy) in disc(1.5) {
                    if let Some(i) = cell(x + ox, y + oy) {
                        if self.ground[i] == Ground::Agar && !self.on_pile(i) {
                            self.ground[i] = Ground::Salt;
                            self.trail[i] = 0.0;
                            self.scent[i] = 0.0;
                            self.counts.salt_cells += 1;
                        }
                    }
                }
                self.kill_in(x, y, 1.5, Kill::Snip);
            }
            Tool::Snip => {
                let heal = self.t + (self.params.wound * STEPS_PER_SECOND as f32) as u64;
                for (ox, oy) in disc(2.5) {
                    if let Some(i) = cell(x + ox, y + oy) {
                        self.trail[i] = 0.0;
                        if self.params.wound > 0.0 && self.ground[i] == Ground::Agar && !self.on_pile(i) {
                            self.ground[i] = Ground::Wound;
                            self.wounds.push((i, heal));
                            self.wound_until[i] = heal;
                            self.counts.wound_cells += 1;
                        }
                    }
                }
                self.kill_in(x, y, 2.5, Kill::Snip);
                self.snip_marks.push((x, y, self.t));
            }
            _ => {}
        }
    }

    fn on_pile(&self, i: usize) -> bool {
        let (x, y) = ((i % W) as f32 + 0.5, (i / W) as f32 + 0.5);
        self.piles
            .iter()
            .any(|p| (p.x - x).powi(2) + (p.y - y).powi(2) <= p.r * p.r)
    }

    fn kill_in(&mut self, x: f32, y: f32, r: f32, why: Kill) {
        let r2 = r * r;
        let mut k = 0;
        while k < self.walkers.len() {
            let w = self.walkers[k];
            if (w.x - x).powi(2) + (w.y - y).powi(2) <= r2 {
                self.remove_walker(k, why);
            } else {
                k += 1;
            }
        }
    }

    fn remove_walker(&mut self, k: usize, why: Kill) {
        let w = self.walkers.swap_remove(k);
        if let Some(i) = cell(w.x, w.y) {
            self.occ[i] = 0;
        }
        match why {
            Kill::Starved => self.counts.starved += 1,
            Kill::Hungry => self.counts.hungry += 1,
            Kill::Burned => self.counts.burned += 1,
            Kill::Snip => self.counts.snipped += 1,
            Kill::Storm => self.counts.stormed += 1,
        }
    }

    // ------------------------------------------------------------- the step

    pub fn step(&mut self) {
        self.update_light();
        self.push_light();
        self.deposit_food();
        self.move_walkers();
        self.flow_out();
        self.diffuse();
        if self.t.is_multiple_of(FED_EVERY) {
            // Once the Wanderer has gone, who is fed is settled: the part
            // that left is fed, everything left behind is not (`wander.rs`).
            if self.wander_frozen() {
                self.stamp_fed();
            } else {
                self.census();
                self.wander_census();
            }
        }
        self.economy();
        self.heal_wounds();
        self.expire_oats();
        self.phase_tick();
        self.wander_tick();
        if self.lamp.is_some() {
            self.counts.lamp_steps += 1;
        }
        self.t += 1;
    }

    fn update_light(&mut self) {
        if let Some((x0, y0, x1, y1)) = self.light_box.take() {
            for y in y0..y1 {
                for x in x0..x1 {
                    self.light[idx(x, y)] = 0.0;
                }
            }
        }
        let Some((lx, ly)) = self.lamp else { return };
        let r = self.params.lamp_reach;
        let x0 = (lx - r).floor().max(0.0) as usize;
        let y0 = (ly - r).floor().max(0.0) as usize;
        let x1 = ((lx + r).ceil() as usize + 1).min(W);
        let y1 = ((ly + r).ceil() as usize + 1).min(H);
        for y in y0..y1 {
            for x in x0..x1 {
                let d = ((x as f32 + 0.5 - lx).powi(2) + (y as f32 + 0.5 - ly).powi(2)).sqrt();
                // A bright flat core and a soft edge, so a sensor reads a
                // slope before the walker is in the light.
                let l = if self.params.lamp_soft {
                    // No flat core: brightest at the centre and sloping all
                    // the way out, so every sensor in reach reads a push.
                    ((r - d) / r).clamp(0.0, 1.0).powi(2)
                } else {
                    ((r - d) / (r * 0.5)).clamp(0.0, 1.0)
                };
                self.light[idx(x, y)] = l;
            }
        }
        self.light_box = Some((x0, y0, x1, y1));
    }

    /// **The lamp shoves the mould, rather than only repelling it.**
    ///
    /// The playtest of 2026-11-08 found the lamp "an eraser, not a herding
    /// tool": the plan's lamp logic alone scored 1 star over 5 dishes, worse
    /// than doing nothing. A sensor-only lamp cannot move a tube: walkers
    /// leave the light, the trail behind them fades, and the tube parts.
    /// Measured on the trunk at 25 s with the lamp 8 cells beside it, every
    /// sensor-only setting tried (weights 20 to 400, flat and sloped light,
    /// reach 16 to 28, with and without the fade and the burn) lost a join
    /// on most seeds, and the harder it pushed the more it broke.
    ///
    /// So the light also carries what is under it: a share of each lit
    /// cell's trail moves one cell outward, away from the lamp, and lit
    /// walkers are stepped outward too. The tube is moved as a whole, like a
    /// rope pushed with a finger, and stays joined while it bends.
    fn push_light(&mut self) {
        let push = self.params.push;
        let (Some((lx, ly)), Some((x0, y0, x1, y1))) = (self.lamp, self.light_box) else {
            return;
        };
        if push <= 0.0 {
            return;
        }
        // Trail: outermost rings first, so a share moved this step is not
        // moved again further out in the same step.
        let mut cells: Vec<(f32, usize, usize)> = Vec::new();
        for y in y0..y1 {
            for x in x0..x1 {
                let i = idx(x, y);
                if self.light[i] > 0.0 && self.ground[i] == Ground::Agar {
                    let d = (x as f32 + 0.5 - lx).hypot(y as f32 + 0.5 - ly);
                    cells.push((d, x, y));
                }
            }
        }
        cells.sort_by(|a, b| b.0.total_cmp(&a.0));
        for &(d, x, y) in &cells {
            let i = idx(x, y);
            let (ux, uy) = if d > 0.01 {
                ((x as f32 + 0.5 - lx) / d, (y as f32 + 0.5 - ly) / d)
            } else {
                (1.0, 0.0)
            };
            let (tx, ty) = ((x as f32 + 0.5 + ux * 1.5).floor(), (y as f32 + 0.5 + uy * 1.5).floor());
            if let Some(j) = cell(tx, ty) {
                if j != i && self.ground[j] == Ground::Agar {
                    let moved = self.trail[i] * push * self.light[i];
                    self.trail[i] -= moved;
                    self.trail[j] += moved;
                }
            }
        }
        // Walkers.
        for k in 0..self.walkers.len() {
            let w = self.walkers[k];
            let Some(i) = cell(w.x, w.y) else { continue };
            let l = self.light[i];
            if l <= 0.0 || self.rng.f32() >= push * l {
                continue;
            }
            let d = (w.x - lx).hypot(w.y - ly).max(0.01);
            let (nx, ny) = (w.x + (w.x - lx) / d, w.y + (w.y - ly) / d);
            if let Some(j) = cell(nx, ny) {
                if j != i && self.ground[j] == Ground::Agar && self.occ[j] == 0 {
                    self.occ[i] = 0;
                    self.occ[j] = 1;
                    self.walkers[k].x = nx;
                    self.walkers[k].y = ny;
                }
            }
        }
    }

    fn deposit_food(&mut self) {
        let fd = self.params.food_deposit;
        for p in &self.piles {
            if p.food <= 0.0 {
                continue;
            }
            // Oats smell as strong as a pile, which is what makes a lure out
            // of five of them; a pile running low smells less.
            let mut strength = fd * (0.3 + 0.7 * (p.food / p.full).min(1.0));
            if p.reached() {
                strength *= self.params.fed_scent;
            }
            for (ox, oy) in disc(p.r) {
                if let Some(i) = cell(p.x + ox, p.y + oy) {
                    if self.ground[i] == Ground::Agar {
                        self.scent[i] += strength;
                    }
                }
            }
        }
    }

    #[inline]
    fn sense(&self, x: f32, y: f32) -> f32 {
        match cell(x, y) {
            Some(i) if self.ground[i] == Ground::Agar => {
                self.trail[i] + self.scent[i] - self.params.light_weight * self.light[i]
            }
            _ => -1.0e6,
        }
    }

    fn move_walkers(&mut self) {
        let n = self.walkers.len();
        self.order.clear();
        self.order.extend(0..n as u32);
        for k in (1..n).rev() {
            let j = self.rng.below(k as u32 + 1) as usize;
            self.order.swap(k, j);
        }
        let (sa, ra, so, dep) = (
            self.params.sensor_angle,
            self.params.rotate_angle,
            self.params.sensor_offset,
            self.params.deposit,
        );
        for k in 0..n {
            let wi = self.order[k] as usize;
            let mut w = self.walkers[wi];
            let h = w.heading;
            let f = self.sense(w.x + h.cos() * so, w.y + h.sin() * so);
            let fl = self.sense(w.x + (h - sa).cos() * so, w.y + (h - sa).sin() * so);
            let fr = self.sense(w.x + (h + sa).cos() * so, w.y + (h + sa).sin() * so);
            let mut h = h;
            if f > fl && f > fr {
                // straight on
            } else if f < fl && f < fr {
                h += if self.rng.next_u64() & 1 == 0 { ra } else { -ra };
            } else if fl < fr {
                h += ra;
            } else if fr < fl {
                h -= ra;
            }
            let (nx, ny) = (w.x + h.cos(), w.y + h.sin());
            let here = cell(w.x, w.y).expect("walker inside the dish");
            match cell(nx, ny) {
                Some(there) if there == here => {
                    w.x = nx;
                    w.y = ny;
                    w.heading = h;
                    self.trail[there] += dep;
                }
                Some(there) if self.ground[there] == Ground::Agar && self.occ[there] == 0 => {
                    self.occ[here] = 0;
                    self.occ[there] = 1;
                    w.x = nx;
                    w.y = ny;
                    w.heading = h;
                    self.trail[there] += dep;
                }
                _ => {
                    // Blocked: Jones's rule, stay put and pick a new heading.
                    w.heading = self.rng.f32() * TAU;
                }
            }
            self.walkers[wi] = w;
        }
    }

    fn diffuse(&mut self) {
        let (keep, fade, scent_keep) = (self.params.keep, self.params.light_fade, self.params.scent_keep);
        spread(
            &self.ground,
            &self.trail,
            &mut self.scratch,
            keep,
            Some((&self.light, fade)),
        );
        std::mem::swap(&mut self.trail, &mut self.scratch);
        spread(&self.ground, &self.scent, &mut self.scratch, scent_keep, None);
        std::mem::swap(&mut self.scent, &mut self.scratch);
    }

    /// Which piles the mould is on, and how far every tube cell is from food.
    fn census(&mut self) {
        for p in self.piles.iter_mut() {
            p.eaters = 0;
        }
        // Read before the search below refills it: last census's answer.
        let limit = self.params.stray_reach;
        let strays_ok: Vec<bool> = if limit.is_finite() && self.t > 0 {
            let none_fed = self.fed.iter().all(|&f| f == u16::MAX);
            self.piles.iter().map(|p| self.fed_within(p.x, p.y, p.r + limit, none_fed)).collect()
        } else {
            vec![true; self.piles.len()]
        };
        for w in &self.walkers {
            let stray = cell(w.x, w.y).is_some_and(|i| self.fed[i] == u16::MAX);
            for (p, &ok) in self.piles.iter_mut().zip(&strays_ok) {
                if stray && !ok {
                    continue;
                }
                if p.food > 0.0 && (p.x - w.x).powi(2) + (p.y - w.y).powi(2) <= (p.r + 1.0).powi(2) {
                    p.eaters += 1;
                }
            }
        }
        // Breadth-first along tube from every pile the mould is eating. Each
        // fed cell also remembers which pile its food comes from
        // (`fed_src`); where two piles' searches meet, those piles are one
        // piece (`Params::pieces`), and the cell where they met is where a
        // join is drawn (`snaps`). The distances, and so everything the
        // shared-store game reads, are exactly what they were before the
        // sources were tracked.
        self.fed.fill(u16::MAX);
        let n = self.piles.len();
        let mut parent: Vec<usize> = (0..n).collect();
        // (pile a, pile b, cell where they met), in the order found.
        let mut meets: Vec<(usize, usize, usize)> = Vec::new();
        let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
        for (k, p) in self.piles.iter().enumerate() {
            if !p.reached() {
                continue;
            }
            for (ox, oy) in disc(p.r + 1.0) {
                if let Some(i) = cell(p.x + ox, p.y + oy) {
                    if self.ground[i] == Ground::Agar {
                        if self.fed[i] != 0 {
                            self.fed[i] = 0;
                            self.fed_src[i] = k as u16;
                            queue.push_back(i);
                        } else if self.fed_src[i] as usize != k {
                            // Two piles close enough to share a cell.
                            meets.push((self.fed_src[i] as usize, k, i));
                        }
                    }
                }
            }
        }
        let tube = self.params.tube;
        let reach = self.params.feed_reach;
        while let Some(i) = queue.pop_front() {
            let d = self.fed[i];
            if d >= reach {
                continue;
            }
            let src = self.fed_src[i];
            let (x, y) = (i % W, i / W);
            let mut visit = |j: usize, fed: &mut Vec<u16>, fed_src: &mut Vec<u16>| {
                if self.ground[j] == Ground::Agar && self.trail[j] > tube {
                    if fed[j] == u16::MAX {
                        fed[j] = d + 1;
                        fed_src[j] = src;
                        queue.push_back(j);
                    } else if fed_src[j] != src {
                        meets.push((src as usize, fed_src[j] as usize, j));
                    }
                }
            };
            if x > 0 {
                visit(i - 1, &mut self.fed, &mut self.fed_src);
            }
            if x + 1 < W {
                visit(i + 1, &mut self.fed, &mut self.fed_src);
            }
            if y > 0 {
                visit(i - W, &mut self.fed, &mut self.fed_src);
            }
            if y + 1 < H {
                visit(i + W, &mut self.fed, &mut self.fed_src);
            }
        }
        self.join_pieces(&mut parent, &meets);
        self.stamp_fed();
    }

    /// What the drawing reads (`looks_fed`): fed tube and the cells beside
    /// it, stamped with the time.
    fn stamp_fed(&mut self) {
        let t = self.t;
        let r = FED_LOOK_REACH as isize;
        for i in 0..W * H {
            if self.fed[i] == u16::MAX {
                continue;
            }
            let (x, y) = ((i % W) as isize, (i / W) as isize);
            for dy in -r..=r {
                for dx in -r..=r {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < W && (ny as usize) < H {
                        self.fed_at[ny as usize * W + nx as usize] = t;
                    }
                }
            }
        }
    }

    /// Whether any cell within `r` of a point is fed, or, when no cell on
    /// the dish is fed (`none_fed`), was within the last
    /// `Params::stray_grace` seconds: the stray rule's test
    /// (`Params::stray_reach`). Fed now only, it locked a dish whose food
    /// had all run out (review 1, finding 1): once no cell anywhere was fed
    /// every walker was a stray and every pile refused them, so on Running
    /// Dry oats dropped right on 950 just-starved walkers were never eaten
    /// and the dish ran 85 s it could not be won. With the grace, mould
    /// that has only just gone grey can be rescued by oats laid on it, and
    /// a stray far from any recent network still cannot colonise. Longer is
    /// not kinder: at 5 s (an unfed walker's mean life, the first choice)
    /// a random hand's oat landed on its dying mould and took 3 stars.
    ///
    /// **Only while nothing is fed**, which is exactly the lock. Applied
    /// whenever, it let strays start food near a tube that had dipped for
    /// a moment, and Beat the Ruler's intended hand on seed 7 joined round
    /// the side (1.10x, 1 star) at every grace from 1 to 3 s, against 0.88x
    /// and 3 stars at 0.
    fn fed_within(&self, x: f32, y: f32, r: f32, none_fed: bool) -> bool {
        let grace = if none_fed {
            (self.params.stray_grace.max(0.0) * STEPS_PER_SECOND as f32) as u64
        } else {
            0
        };
        let t = self.t;
        disc(r).any(|(ox, oy)| {
            cell(x + ox, y + oy).is_some_and(|i| {
                self.fed[i] != u16::MAX || (self.fed_at[i] != u64::MAX && t.saturating_sub(self.fed_at[i]) <= grace)
            })
        })
    }

    /// Union the piles whose searches met into pieces, flash every join
    /// between two pieces that were both eating, apart, at the last census
    /// (and not joined a moment ago), and remember the pieces.
    ///
    /// **Both piles eaten last census, in different pieces then** (review
    /// 1, finding 2). Any first meeting used to flash, so an oat laid on
    /// fed tube and eaten by the network already there "joined" it, drawn
    /// halfway between the oat and the pile: 3 oats on fed tube made 6
    /// flashes on free dish 1, none at an oat. A pile newly reached never
    /// flashes on its first census now; a bridge between two pieces that
    /// were each feeding, oats a stray started included, still does.
    fn join_pieces(&mut self, parent: &mut [usize], meets: &[(usize, usize, usize)]) {
        fn root(parent: &mut [usize], mut a: usize) -> usize {
            while parent[a] != a {
                parent[a] = parent[parent[a]];
                a = parent[a];
            }
            a
        }
        let t = self.t;
        let forget = (SNAP_FORGET_SECONDS * STEPS_PER_SECOND as f32) as u64;
        let keys: Vec<u64> = self.piles.iter().map(pile_key).collect();
        for &(a, b, at) in meets {
            let (ra, rb) = (root(parent, a), root(parent, b));
            if ra == rb {
                continue;
            }
            parent[ra.max(rb)] = ra.min(rb);
            let pair = (keys[a].min(keys[b]), keys[a].max(keys[b]));
            let recent = self.last_joined.get(&pair).is_some_and(|&was| t.saturating_sub(was) <= forget);
            let was = |k: u64| self.eaten_pieces.iter().find(|e| e.0 == k).map(|e| e.1);
            let apart = matches!((was(keys[a]), was(keys[b])), (Some(pa), Some(pb)) if pa != pb);
            // Never at the census that builds the dish (nothing was eaten
            // before it): the inoculum lying over two piles is not a join
            // anyone made.
            if !recent && apart {
                let (x, y) = ((at % W) as f32 + 0.5, (at / W) as f32 + 0.5);
                self.snaps.push((x, y, t));
                self.counts.snaps += 1;
            }
        }
        // Pieces, numbered in pile order, and the pairs now joined.
        let n = self.piles.len();
        let mut number = vec![u16::MAX; n];
        self.piece_of_src = vec![0; n];
        let mut count = 0u16;
        for k in 0..n {
            let r = root(parent, k);
            if number[r] == u16::MAX {
                number[r] = count;
                count += 1;
            }
            self.piece_of_src[k] = number[r];
        }
        for a in 0..n {
            if !self.piles[a].reached() {
                continue;
            }
            for b in a + 1..n {
                if self.piles[b].reached() && self.piece_of_src[a] == self.piece_of_src[b] {
                    let pair = (keys[a].min(keys[b]), keys[a].max(keys[b]));
                    self.last_joined.insert(pair, t);
                }
            }
        }
        self.piece_count = count as usize;
        self.eaten_pieces = (0..n)
            .filter(|&k| self.piles[k].reached())
            .map(|k| (keys[k], self.piece_of_src[k]))
            .collect();
        self.census_keys = keys;
        let keep = 2 * STEPS_PER_SECOND as u64;
        self.snaps.retain(|&(_, _, at)| t.saturating_sub(at) <= keep);
    }

    /// The piece a cell's mould belongs to, by the last census: `None` where
    /// it is not fed.
    pub fn piece_of(&self, i: usize) -> Option<usize> {
        if self.fed[i] == u16::MAX {
            return None;
        }
        self.piece_of_src.get(self.fed_src[i] as usize).map(|&p| p as usize)
    }

    fn heal_wounds(&mut self) {
        if self.wounds.is_empty() {
            return;
        }
        let t = self.t;
        let (ground, until) = (&mut self.ground, &mut self.wound_until);
        self.wounds.retain(|&(i, heal)| {
            if heal > t {
                return true;
            }
            // A later snip over the same cell keeps it open longer.
            if until[i] <= t && ground[i] == Ground::Wound {
                ground[i] = Ground::Agar;
                until[i] = u64::MAX;
            }
            false
        });
    }

    /// **Whether a drawn tube cell shows fed (yellow) or starving (grey).**
    /// Owner's playtest, 2026-11-08: "sometimes things were flashing gray".
    /// The feeding search only walks cells over the tube threshold, but a
    /// tube is drawn from half that, so the soft edge of every fed tube
    /// was drawn grey, and edge cells whose trail wobbled across the
    /// threshold switched yellow-grey-yellow every few frames: about 500
    /// cells a second flashing on a working network. A cell now shows fed
    /// if it, or a cell within `FED_LOOK_REACH`, was fed in the last
    /// `FED_LOOK_SECONDS`, so a starving branch still goes grey, a fraction
    /// of a second later. The simulation still reads `fed` alone.
    pub fn looks_fed(&self, i: usize) -> bool {
        let at = self.fed_at[i];
        at != u64::MAX && self.t.saturating_sub(at) <= (FED_LOOK_SECONDS * STEPS_PER_SECOND as f32) as u64
    }

    /// The economy with `Params::pieces`: the shared-store rules below,
    /// applied to each piece's own food. A piece's food is the sum of its
    /// walkers' shares; it earns what its piles are eaten for, pays upkeep
    /// per walker, goes hungry when it runs dry, and pays for its own
    /// births; then what is left is shared out again. An unfed walker (on
    /// tube no food reaches) lives on its own share until it starves.
    fn economy_by_piece(&mut self) {
        let pieces = self.piece_count;
        let upkeep = self.params.upkeep;
        let mut food = vec![0.0f32; pieces];
        // Eating, credited to the eating pile's piece. A pile added since
        // the census has no eaters yet; one gone since is simply absent.
        let eat = self.params.eat;
        let table = self.params.table;
        for p in self.piles.iter_mut() {
            if p.eaters > 0 && p.food > 0.0 {
                let key = pile_key(p);
                let Some(k) = self.census_keys.iter().position(|&c| c == key) else { continue };
                let e = (p.eaters.min(table) as f32 * eat).min(p.food);
                p.food -= e;
                food[self.piece_of_src[k] as usize] += e;
            }
        }
        // Each walker's piece, its share pooled, and upkeep.
        let mut piece_of: Vec<Option<usize>> = Vec::with_capacity(self.walkers.len());
        let mut size = vec![0u32; pieces];
        for k in 0..self.walkers.len() {
            let w = self.walkers[k];
            let i = cell(w.x, w.y).expect("walker inside the dish");
            let g = self.piece_of(i);
            match g {
                Some(g) => {
                    food[g] += w.food - upkeep;
                    size[g] += 1;
                }
                None => self.walkers[k].food = (w.food - upkeep).max(0.0),
            }
            piece_of.push(g);
        }
        let empty: Vec<bool> = food.iter().map(|&f| f <= 0.0).collect();
        for f in food.iter_mut() {
            *f = f.max(0.0);
        }
        // Deaths: the shared-store rules, with "the store is empty" read
        // per piece (or per walker, for an unfed one).
        let reach = self.params.feed_reach as f32;
        let mut k = 0;
        while k < self.walkers.len() {
            let w = self.walkers[k];
            let i = cell(w.x, w.y).expect("walker inside the dish");
            let d = self.fed[i];
            let g = piece_of[k];
            let fed = g.is_some();
            let dry = match g {
                Some(g) => empty[g],
                None => w.food <= 0.0,
            };
            let roll = self.rng.f32();
            let mut why = None;
            let l = self.light[i];
            if l > 0.0 && roll < self.params.burn * l {
                why = Some(Kill::Burned);
            } else if !fed && roll < self.params.starve {
                why = Some(Kill::Starved);
            } else if dry {
                let far = if fed { d as f32 / reach } else { 1.5 };
                if roll < self.params.hungry * (0.25 + far) {
                    why = Some(Kill::Hungry);
                }
            }
            if let Some(why) = why {
                // `remove_walker` swaps the last walker into k: keep the
                // piece list in step.
                self.remove_walker(k, why);
                piece_of.swap_remove(k);
                if let Some(g) = g {
                    size[g] -= 1;
                }
            } else {
                k += 1;
            }
        }
        // Births, each paid for by its parent's piece.
        let n = self.walkers.len();
        let crowd = self.params.crowd;
        for k in 0..n {
            if self.rng.f32() >= self.params.divide {
                continue;
            }
            let Some(g) = piece_of[k] else { continue };
            if food[g] < 1.0 {
                continue;
            }
            let w = self.walkers[k];
            let (wx, wy) = (w.x as i32, w.y as i32);
            if self.light[idx(wx as usize, wy as usize)] > 0.0 {
                continue;
            }
            let mut count = 0;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let (x, y) = (wx + dx, wy + dy);
                    if x >= 0
                        && y >= 0
                        && (x as usize) < W
                        && (y as usize) < H
                        && self.occ[idx(x as usize, y as usize)] != 0
                    {
                        count += 1;
                    }
                }
            }
            if count > crowd {
                continue;
            }
            let dir = self.rng.below(8) as i32;
            let (dx, dy) = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)][dir as usize];
            let (x, y) = (wx + dx, wy + dy);
            if x < 0 || y < 0 || x as usize >= W || y as usize >= H {
                continue;
            }
            let j = idx(x as usize, y as usize);
            if self.ground[j] != Ground::Agar || self.occ[j] != 0 {
                continue;
            }
            self.occ[j] = 1;
            let heading = self.rng.f32() * TAU;
            self.walkers.push(Walker {
                x: x as f32 + 0.5,
                y: y as f32 + 0.5,
                heading,
                food: 0.0,
            });
            piece_of.push(Some(g));
            size[g] += 1;
            food[g] -= 1.0;
            self.counts.births += 1;
        }
        // Share each piece's food out again; the store is the total, for
        // the HUD and anything else that reads it.
        let mut total = 0.0;
        for (k, w) in self.walkers.iter_mut().enumerate() {
            if let Some(g) = piece_of[k] {
                w.food = food[g] / size[g].max(1) as f32;
            }
            total += w.food;
        }
        self.store = total;
    }

    fn economy(&mut self) {
        if self.params.pieces {
            return self.economy_by_piece();
        }
        // Eating.
        let eat = self.params.eat;
        for p in self.piles.iter_mut() {
            if p.eaters > 0 && p.food > 0.0 {
                let e = (p.eaters.min(self.params.table) as f32 * eat).min(p.food);
                p.food -= e;
                self.store += e;
            }
        }
        // Upkeep.
        self.store -= self.walkers.len() as f32 * self.params.upkeep;
        let empty = self.store <= 0.0;
        if empty {
            self.store = 0.0;
        }
        // Deaths, then births.
        let reach = self.params.feed_reach as f32;
        let mut k = 0;
        while k < self.walkers.len() {
            let w = self.walkers[k];
            let i = cell(w.x, w.y).expect("walker inside the dish");
            let d = self.fed[i];
            let fed = d != u16::MAX;
            let roll = self.rng.f32();
            let mut why = None;
            let l = self.light[i];
            if l > 0.0 && roll < self.params.burn * l {
                why = Some(Kill::Burned);
            } else if !fed && roll < self.params.starve {
                why = Some(Kill::Starved);
            } else if empty {
                let far = if fed { d as f32 / reach } else { 1.5 };
                if roll < self.params.hungry * (0.25 + far) {
                    why = Some(Kill::Hungry);
                }
            }
            if let Some(why) = why {
                self.remove_walker(k, why);
            } else {
                k += 1;
            }
        }
        if self.store < 1.0 {
            return;
        }
        let n = self.walkers.len();
        let crowd = self.params.crowd;
        for k in 0..n {
            if self.store < 1.0 {
                break;
            }
            if self.rng.f32() >= self.params.divide {
                continue;
            }
            let w = self.walkers[k];
            let (wx, wy) = (w.x as i32, w.y as i32);
            if self.fed[idx(wx as usize, wy as usize)] == u16::MAX {
                continue;
            }
            if self.light[idx(wx as usize, wy as usize)] > 0.0 {
                continue;
            }
            let mut count = 0;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let (x, y) = (wx + dx, wy + dy);
                    if x >= 0
                        && y >= 0
                        && (x as usize) < W
                        && (y as usize) < H
                        && self.occ[idx(x as usize, y as usize)] != 0
                    {
                        count += 1;
                    }
                }
            }
            if count > crowd {
                continue;
            }
            let dir = self.rng.below(8) as i32;
            let (dx, dy) = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)][dir as usize];
            let (x, y) = (wx + dx, wy + dy);
            if x < 0 || y < 0 || x as usize >= W || y as usize >= H {
                continue;
            }
            let j = idx(x as usize, y as usize);
            if self.ground[j] != Ground::Agar || self.occ[j] != 0 {
                continue;
            }
            self.occ[j] = 1;
            let heading = self.rng.f32() * TAU;
            self.walkers.push(Walker {
                x: x as f32 + 0.5,
                y: y as f32 + 0.5,
                heading,
                food: 0.0,
            });
            self.store -= 1.0;
            self.counts.births += 1;
        }
    }

    fn expire_oats(&mut self) {
        let t = self.t;
        self.piles.retain(|p| !(p.oats && (t >= p.expires || p.food <= 0.0)));
    }

    fn phase_tick(&mut self) {
        let sps = STEPS_PER_SECOND as u64;
        if self.phase == Phase::Growing {
            // Oats come back, and late food turns up.
            if self.t > 0
                && self.oats_regrow > 0
                && self.t.is_multiple_of(self.oats_regrow as u64 * sps)
                && self.oats_left < self.oats_stock
            {
                self.oats_left += 1;
            }
            let t = self.t;
            let mut k = 0;
            while k < self.pending.len() {
                if self.pending[k].0 <= t {
                    let (_, p) = self.pending.remove(k);
                    self.arrived.push((p.x, p.y, t));
                    self.piles.insert(self.dish_piles().count(), p);
                } else {
                    k += 1;
                }
            }
            // A hand-made dish ends by its own rule, with no storm.
            if let Some(level) = self.level {
                return self.level_tick(level);
            }
            if self.storm_line.is_none() && self.t + 1 >= self.storm_at.saturating_sub(self.storm_warning) {
                self.storm_line = self.aim_storm();
            }
        }
        match self.phase {
            Phase::Growing if self.t + 1 >= self.storm_at => {
                self.lamp = None;
                self.stroke = None;
                self.update_light();
                let before = self.measure();
                self.strike();
                self.before = Some(before);
                self.phase = Phase::Storm(self.t);
            }
            // Toughness is read after the whole watch, so a tube that grew
            // back round the scar, or a bypass built in time, counts.
            Phase::Storm(at) if self.t >= at + Self::storm_steps() => {
                let mut s = self.before.clone().unwrap_or_default();
                s.after = self.measure().reach.min(s.reach);
                self.after = Some(s.after);
                // Toughness, like thrift, needs a network worth the name:
                // with each piece eating its own food, two piles left alone
                // regrow to each other round the scar within the watch, and
                // a do-nothing hand took the star on 2 of 5 seeds (0 of 5
                // with the shared store). Three piles, as for thrift.
                //
                // **And both count only once all the food is joined**: the
                // stars are a ladder, as on the hand-made dishes (review 1,
                // finding 3). Below it, doing nothing took the thrift star
                // on 2 of 8 seeds (4 of 6 piles, a short network because it
                // was a small one), and a random hand took thrift and
                // toughness with 5 of 6 joined.
                let networked = s.reach >= THRIFT_MIN_JOINED && s.reach == s.piles;
                s.stars = [
                    s.reach == s.piles,
                    networked && s.ratio() <= THRIFT_BAR,
                    networked && s.after == s.reach,
                ];
                self.card = Some(level::free_card(&s));
                self.score = Some(s);
                self.phase = Phase::Over;
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ the score

    /// Connected pieces of tube: a label per cell (0 = none) and the count.
    pub fn pieces(&self) -> (Vec<u32>, u32) {
        let mut label = vec![0u32; W * H];
        let mut next = 0;
        let tube = self.params.tube;
        let mut stack = Vec::new();
        for s in 0..W * H {
            if label[s] != 0 || self.ground[s] != Ground::Agar || self.trail[s] <= tube {
                continue;
            }
            next += 1;
            label[s] = next;
            stack.push(s);
            while let Some(i) = stack.pop() {
                let (x, y) = (i % W, i / W);
                let mut nb = [usize::MAX; 4];
                if x > 0 {
                    nb[0] = i - 1;
                }
                if x + 1 < W {
                    nb[1] = i + 1;
                }
                if y > 0 {
                    nb[2] = i - W;
                }
                if y + 1 < H {
                    nb[3] = i + W;
                }
                for j in nb {
                    if j != usize::MAX && label[j] == 0 && self.ground[j] == Ground::Agar && self.trail[j] > tube {
                        label[j] = next;
                        stack.push(j);
                    }
                }
            }
        }
        (label, next)
    }

    /// The dish piles each piece touches, and the piece joining the most.
    fn best_piece(&self, label: &[u32], n: u32) -> (u32, Vec<usize>) {
        let mut best = (0u32, Vec::new());
        let piles: Vec<&Pile> = self.dish_piles().collect();
        let mut touched: Vec<Vec<usize>> = vec![Vec::new(); n as usize + 1];
        for (pi, p) in piles.iter().enumerate() {
            let mut seen = std::collections::BTreeSet::new();
            for (ox, oy) in disc(p.r + 1.0) {
                if let Some(i) = cell(p.x + ox, p.y + oy) {
                    if label[i] != 0 {
                        seen.insert(label[i]);
                    }
                }
            }
            for l in seen {
                touched[l as usize].push(pi);
            }
        }
        for (l, t) in touched.into_iter().enumerate() {
            if t.len() > best.1.len() {
                best = (l as u32, t);
            }
        }
        best
    }

    /// Reach, length and shortest, right now.
    pub fn measure(&self) -> Score {
        let (label, n) = self.pieces();
        let (_, joined) = self.best_piece(&label, n);
        let piles: Vec<&Pile> = self.dish_piles().collect();
        // Tube on the piles themselves is not network: leave it out.
        let mut on_pile = vec![false; W * H];
        for p in &self.piles {
            for (ox, oy) in disc(p.r + 2.0) {
                if let Some(i) = cell(p.x + ox, p.y + oy) {
                    on_pile[i] = true;
                }
            }
        }
        let tube_cells = label.iter().zip(&on_pile).filter(|(l, p)| **l != 0 && !**p).count() as f32;
        // Prim's tree over the joined piles' centres, edge = gap between rims.
        let pts: Vec<(f32, f32, f32)> = joined.iter().map(|&i| (piles[i].x, piles[i].y, piles[i].r)).collect();
        let mut shortest = 0.0;
        if pts.len() >= 2 {
            let mut inside = vec![false; pts.len()];
            let mut best = vec![f32::INFINITY; pts.len()];
            best[0] = 0.0;
            for _ in 0..pts.len() {
                let mut u = usize::MAX;
                for v in 0..pts.len() {
                    if !inside[v] && (u == usize::MAX || best[v] < best[u]) {
                        u = v;
                    }
                }
                inside[u] = true;
                shortest += best[u];
                for v in 0..pts.len() {
                    if !inside[v] {
                        let d = ((pts[u].0 - pts[v].0).powi(2) + (pts[u].1 - pts[v].1).powi(2)).sqrt()
                            - pts[u].2
                            - pts[v].2;
                        best[v] = best[v].min(d.max(1.0));
                    }
                }
            }
        }
        Score {
            reach: joined.len() as u32,
            piles: piles.len() as u32,
            length: tube_cells / TUBE_WIDTH,
            shortest,
            ..Score::default()
        }
    }

    /// **Where the storm will strike: the tube whose loss costs the most
    /// food**, thickest first among equals. Until the 2026-11-08 playtest it
    /// struck the thickest tube, which cost a pile in 30 of 35 runs whatever
    /// the player did, and sometimes landed on a spur out to stray oats and
    /// cost nothing -- so a random hand earned toughness by luck. Aiming at
    /// the worst cut makes the star mean "this network has no single weak
    /// tube", and the warning gives a player the five seconds to fix it.
    fn aim_storm(&self) -> Option<((f32, f32), (f32, f32))> {
        let (label, n) = self.pieces();
        let (best, _) = self.best_piece(&label, n);
        if best == 0 {
            return None;
        }
        let mut cand: Vec<(f32, usize)> = Vec::new();
        for (i, &l) in label.iter().enumerate() {
            let (x, y) = (i % W, i / W);
            if l != best || x % 3 != 0 || y % 3 != 0 {
                continue;
            }
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let near_pile = self.piles.iter().any(|p| (p.x - fx).hypot(p.y - fy) <= p.r + 12.0);
            if !near_pile {
                cand.push((self.trail[i], i));
            }
        }
        let cells: Vec<usize> = (0..W * H).filter(|&i| label[i] == best).collect();
        let mut comp = vec![0u32; W * H];
        let mut pick: Option<(u32, f32, Segment)> = None;
        for (trail, i) in cand {
            let (x, y) = ((i % W) as f32 + 0.5, (i / W) as f32 + 0.5);
            let line = self.cut_across(&label, best, (x, y));
            let left = self.joined_without(&label, best, &cells, &mut comp, line);
            let better = match pick {
                None => true,
                Some((l, t, _)) => left < l || (left == l && trail > t),
            };
            if better {
                pick = Some((left, trail, line));
            }
        }
        pick.map(|p| p.2)
    }

    /// The scar line across the tube at `at`: the long axis of the piece's
    /// cells within 8 cells is the tube's direction, and the cut runs across.
    fn cut_across(&self, label: &[u32], best: u32, (x, y): (f32, f32)) -> ((f32, f32), (f32, f32)) {
        let (mut sxx, mut syy, mut sxy) = (0.0f32, 0.0f32, 0.0f32);
        for (ox, oy) in disc(8.0) {
            if let Some(i) = cell(x + ox, y + oy) {
                if label[i] == best {
                    sxx += ox * ox;
                    syy += oy * oy;
                    sxy += ox * oy;
                }
            }
        }
        let along = 0.5 * (2.0 * sxy).atan2(sxx - syy);
        let (cx, cy) = (-along.sin(), along.cos());
        (
            (x - cx * CUT_HALF, y - cy * CUT_HALF),
            (x + cx * CUT_HALF, y + cy * CUT_HALF),
        )
    }

    /// The cells a scar along `line` covers.
    fn line_cells(&self, (a, b): ((f32, f32), (f32, f32))) -> Vec<usize> {
        let steps = (CUT_HALF * 2.0).ceil() as usize;
        let mut out = Vec::new();
        for k in 0..=steps {
            let f = k as f32 / steps as f32;
            let (px, py) = (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f);
            for (ox, oy) in disc(1.5) {
                if let Some(i) = cell(px + ox, py + oy) {
                    if self.ground[i] == Ground::Agar && !self.on_pile(i) {
                        out.push(i);
                    }
                }
            }
        }
        out
    }

    /// Most dish piles one piece of `best` would still join with `line` cut.
    ///
    /// Floods only the piece's own cells (`cells`), reusing `comp`, which
    /// must be all zero on entry and is left all zero. The first version
    /// flooded the whole dish per candidate, and the aim -- several hundred
    /// candidates, once per dish -- made the warning frame 36 ms against a
    /// 13 ms worst before the fix: a visible hitch at the moment the player
    /// most needs a smooth frame.
    fn joined_without(
        &self,
        label: &[u32],
        best: u32,
        cells: &[usize],
        comp: &mut [u32],
        line: ((f32, f32), (f32, f32)),
    ) -> u32 {
        const CUT: u32 = u32::MAX;
        let cut = self.line_cells(line);
        for &i in &cut {
            comp[i] = CUT;
        }
        let mut next = 0;
        let mut stack = Vec::new();
        for &s in cells {
            if comp[s] != 0 {
                continue;
            }
            next += 1;
            comp[s] = next;
            stack.push(s);
            while let Some(i) = stack.pop() {
                let (x, y) = (i % W, i / W);
                for (ok, j) in [
                    (x > 0, i.wrapping_sub(1)),
                    (x + 1 < W, i + 1),
                    (y > 0, i.wrapping_sub(W)),
                    (y + 1 < H, i + W),
                ] {
                    if ok && label[j] == best && comp[j] == 0 {
                        comp[j] = next;
                        stack.push(j);
                    }
                }
            }
        }
        // As `best_piece`: the piece touching the most piles; cut cells are
        // no piece.
        let joined = {
            let lab = &*comp;
            let masked = |i: usize| if lab[i] == CUT { 0 } else { lab[i] };
            let piles: Vec<&Pile> = self.dish_piles().collect();
            let mut touched: Vec<u32> = vec![0; next as usize + 1];
            for p in &piles {
                let mut seen: Vec<u32> = Vec::new();
                for (ox, oy) in disc(p.r + 1.0) {
                    if let Some(i) = cell(p.x + ox, p.y + oy) {
                        let l = masked(i);
                        if l != 0 && !seen.contains(&l) {
                            seen.push(l);
                        }
                    }
                }
                for l in seen {
                    touched[l as usize] += 1;
                }
            }
            touched.into_iter().max().unwrap_or(0)
        };
        for &i in cells.iter().chain(cut.iter()) {
            comp[i] = 0;
        }
        joined
    }

    /// The storm strikes along the line it was aimed at, wherever the mould
    /// is by then, and leaves a scar nothing crosses.
    fn strike(&mut self) {
        let Some(line) = self.storm_line else { return };
        let cells = self.line_cells(line);
        for &i in &cells {
            self.ground[i] = Ground::Scar;
            self.trail[i] = 0.0;
            self.scent[i] = 0.0;
        }
        for i in cells {
            self.kill_in((i % W) as f32 + 0.5, (i / W) as f32 + 0.5, 0.8, Kill::Storm);
        }
        self.cuts = vec![line];
    }
}

#[derive(Clone, Copy)]
enum Kill {
    Starved,
    Hungry,
    Burned,
    Snip,
    Storm,
}

/// One step of a field: a 3x3 mean over open dish (walls neither soak up nor
/// leak), then the fade, and the lamp's extra fade where one is given.
fn spread(ground: &[Ground], from: &[f32], to: &mut [f32], keep: f32, light: Option<(&[f32], f32)>) {
    for y in 1..H - 1 {
        for x in 1..W - 1 {
            let i = idx(x, y);
            if ground[i] != Ground::Agar {
                to[i] = 0.0;
                continue;
            }
            let mut sum = 0.0;
            let mut n = 0.0;
            for r in [i - W, i, i + W] {
                for j in [r - 1, r, r + 1] {
                    if ground[j] == Ground::Agar {
                        sum += from[j];
                        n += 1.0;
                    }
                }
            }
            let mut v = sum / n * keep;
            if let Some((l, fade)) = light {
                if l[i] > 0.0 {
                    v *= 1.0 - fade * l[i];
                }
            }
            to[i] = v;
        }
    }
}

/// Integer offsets inside a disc of radius `r`, centred on the cell.
pub fn disc(r: f32) -> impl Iterator<Item = (f32, f32)> {
    let n = r.ceil() as i32;
    (-n..=n)
        .flat_map(move |dy| (-n..=n).map(move |dx| (dx, dy)))
        .filter(move |(dx, dy)| ((dx * dx + dy * dy) as f32) <= r * r)
        .map(|(dx, dy)| (dx as f32, dy as f32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_dish() {
        let mut a = Dish::new(7);
        let mut b = Dish::new(7);
        for _ in 0..600 {
            a.step();
            b.step();
        }
        assert_eq!(a.walkers.len(), b.walkers.len());
        assert_eq!(a.trail, b.trail);
    }

    #[test]
    fn walkers_never_share_a_cell_or_stand_on_salt() {
        let mut d = Dish::new(3);
        d.begin_stroke(Tool::Salt, 60.0, 60.0);
        d.drag_stroke(60.0, 110.0);
        d.end_stroke();
        for _ in 0..1200 {
            d.step();
        }
        let mut seen = vec![false; W * H];
        for w in &d.walkers {
            let i = cell(w.x, w.y).unwrap();
            assert!(!seen[i], "two walkers in one cell");
            seen[i] = true;
            assert_eq!(d.ground[i], Ground::Agar);
        }
    }

    /// The positive control for the lamp: light must push the mould out from
    /// under it within a second, against the same dish with no lamp -- and,
    /// since the 2026-11-08 playtest, push it rather than kill it: the mould
    /// it moved is still alive a second later. (The old lamp burned walkers
    /// and faded trail; this test asked for a quarter then. The push lamp
    /// leaves about a third, at the same halving the probe's recoil
    /// measure uses, so the bar is a half.)
    #[test]
    fn the_lamp_clears_the_mould_under_it() {
        let mut lit = Dish::new(11);
        let mut dark = Dish::new(11);
        for _ in 0..240 {
            lit.step();
            dark.step();
        }
        let (hx, hy) = (lit.piles[0].x + 10.0, lit.piles[0].y);
        lit.set_lamp(Some((hx, hy)));
        for _ in 0..STEPS_PER_SECOND {
            lit.step();
            dark.step();
        }
        let under = |d: &Dish| {
            d.walkers
                .iter()
                .filter(|w| (w.x - hx).powi(2) + (w.y - hy).powi(2) < (LAMP_R * 0.5).powi(2))
                .count()
        };
        assert!(
            under(&dark) > 20,
            "control: the mould should be there without the lamp ({})",
            under(&dark)
        );
        assert!(
            under(&lit) * 2 < under(&dark),
            "lamp {} vs dark {}",
            under(&lit),
            under(&dark)
        );
        assert!(
            lit.walkers.len() * 10 >= dark.walkers.len() * 9,
            "the lamp pushes, it does not kill: {} walkers lit vs {} dark",
            lit.walkers.len(),
            dark.walkers.len()
        );
    }

    /// The lesson holds the storm back until its last step, however long
    /// the player takes; a real dish's storm still comes on time.
    #[test]
    fn the_lesson_holds_the_storm_until_its_last_step() {
        let (mut d, mut l) = lesson::Lesson::dish();
        for _ in 0..Dish::dish_steps() + STEPS_PER_SECOND as u64 {
            d.step();
            l.update(&mut d);
        }
        assert_eq!(
            l.step,
            lesson::Step::Lamp,
            "nothing was done, so the lesson stays on step 1"
        );
        assert_eq!(d.phase, Phase::Growing, "no storm during the lesson");
        assert!(d.storm_line.is_none());
    }

    #[test]
    fn a_slow_lesson_does_not_starve() {
        // Bug check 2026-11-08: with no clock and piles still eaten, a lesson
        // left alone had 0 walkers from 150 s on and could never pass Join.
        let (mut d, mut l) = lesson::Lesson::dish();
        let start = d.walkers.len();
        for _ in 0..200 * STEPS_PER_SECOND as u64 {
            d.step();
            l.update(&mut d);
        }
        assert!(
            d.walkers.len() * 2 >= start,
            "{} of {start} walkers left after 200 s idle",
            d.walkers.len()
        );
    }

    #[test]
    fn a_lesson_step_hands_back_a_spent_tool() {
        let (mut d, mut l) = lesson::Lesson::dish();
        d.step();
        while d.salt_left > 0 {
            assert!(d.begin_stroke(Tool::Salt, 20.0, 20.0));
            d.end_stroke();
        }
        l.step = lesson::Step::Salt;
        l.update(&mut d);
        assert_eq!(d.salt_left, 1, "the salt step gives one back");
        assert!(d.begin_stroke(Tool::Salt, 30.0, 30.0));
    }

    #[test]
    fn a_dish_ends_with_a_score() {
        let mut d = Dish::new(1);
        while d.phase != Phase::Over {
            d.step();
        }
        let s = d.score.clone().unwrap();
        assert_eq!(s.piles, 6, "five piles plus the one that appears at 0:55");
        assert!(s.reach >= 1);
        assert!(d.cuts.len() <= STORM_CUTS);
    }

    // ------------------------------------------------------------ review 1
    //
    // One per game finding of `game-design/.../review-1.md` that a test can
    // reach cheaply; each was run red with its fix reverted. The window-only
    // findings (5 the organ's release, 6 the tool carried over, 9 Escape)
    // have no test: they live in `src/bin/`.

    const SPS: u64 = STEPS_PER_SECOND as u64;

    /// Finding 1: on Running Dry, once no cell is fed, oats dropped right on
    /// the starving mould must still be eaten (the stray rule's grace).
    #[test]
    fn just_starved_mould_can_be_rescued_by_oats() {
        let mut d = Dish::from_level(&level::DISHES[2], 1);
        while d.t < 20 * SPS || d.fed.iter().any(|&f| f != u16::MAX) {
            d.step();
            assert!(d.phase != Phase::Over, "the dish ended before it ran dry");
        }
        let n = d.walkers.len() as f32;
        assert!(n > 100.0, "mould left to rescue: {n}");
        let (cx, cy) = (
            d.walkers.iter().map(|w| w.x).sum::<f32>() / n,
            d.walkers.iter().map(|w| w.y).sum::<f32>() / n,
        );
        let w = *d
            .walkers
            .iter()
            .min_by(|a, b| (a.x - cx).hypot(a.y - cy).total_cmp(&(b.x - cx).hypot(b.y - cy)))
            .unwrap();
        d.oats_left = d.oats_left.max(2);
        assert!(d.place_oats(w.x, w.y) && d.place_oats(w.x + 3.0, w.y));
        let mut eaten = false;
        for _ in 0..8 * SPS {
            d.step();
            eaten |= d.piles.iter().any(|p| p.oats && p.reached());
        }
        assert!(eaten, "oats on just-starved mould were never eaten");
    }

    /// Lay tube by hand: a thick line of trail between two points.
    fn lay(d: &mut Dish, a: (f32, f32), b: (f32, f32), r: f32, v: f32) {
        let n = ((b.0 - a.0).hypot(b.1 - a.1).ceil() as usize * 2).max(1);
        for k in 0..=n {
            let f = k as f32 / n as f32;
            let (x, y) = (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f);
            for (ox, oy) in disc(r) {
                if let Some(i) = cell(x + ox, y + oy) {
                    if d.ground[i] == Ground::Agar {
                        d.trail[i] = d.trail[i].max(v);
                    }
                }
            }
        }
    }

    /// Finding 2, its first half: an oat eaten by the network already there
    /// is not a join. Three oats on fed tube, far from any pile: no flash.
    #[test]
    fn an_oat_on_fed_tube_is_not_a_join() {
        let mut base = Dish::new(1);
        for _ in 0..25 * SPS {
            base.step();
        }
        let mut spots = Vec::new();
        for i in (0..W * H).step_by(7) {
            let f = base.fed[i];
            if f == u16::MAX || !(25..=50).contains(&f) {
                continue;
            }
            let (x, y) = ((i % W) as f32 + 0.5, (i / W) as f32 + 0.5);
            if base.piles.iter().all(|p| (p.x - x).hypot(p.y - y) > p.r + 15.0)
                && spots.iter().all(|&(a, b): &(f32, f32)| (a - x).hypot(b - y) > 20.0)
            {
                spots.push((x, y));
            }
        }
        assert!(spots.len() >= 3, "fed tube to lay oats on: {}", spots.len());
        let mut flashed = 0;
        for &(x, y) in spots.iter().take(3) {
            let mut b = base.clone();
            let before = b.counts.snaps;
            b.oats_left = 5;
            assert!(b.place_oats(x, y));
            for _ in 0..2 * SPS {
                b.step();
            }
            flashed += b.counts.snaps - before;
        }
        assert_eq!(flashed, 0, "oats on fed tube flashed joins");
    }

    /// Finding 2, its second half: a real bridge between two piles that
    /// were each being eaten, apart, still flashes.
    #[test]
    fn a_bridge_between_two_fed_pieces_flashes() {
        let mut d = Dish::new(1);
        d.walkers.clear();
        d.occ.fill(0);
        d.trail.fill(0.0);
        // Two piles, each with a blob of mould of its own: two pieces.
        let (a, b) = ((d.piles[0].x, d.piles[0].y), (d.piles[1].x, d.piles[1].y));
        let (ra, rb) = (d.piles[0].r, d.piles[1].r);
        for c in [a, b] {
            for (ox, oy) in disc(3.0) {
                if let Some(i) = cell(c.0 + ox, c.1 + oy) {
                    d.occ[i] = 1;
                    d.walkers.push(Walker { x: c.0 + ox, y: c.1 + oy, heading: 0.0, food: 1.0 });
                }
            }
        }
        for _ in 0..FED_EVERY {
            d.step();
        }
        assert!(d.piles[0].reached() && d.piles[1].reached());
        assert_ne!(d.piece_of_src[0], d.piece_of_src[1], "two pieces before the bridge");
        let before = d.counts.snaps;
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        let (ux, uy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
        let from = (a.0 + ux * ra, a.1 + uy * ra);
        let to = (b.0 - ux * rb, b.1 - uy * rb);
        lay(&mut d, from, to, 2.5, 50.0);
        for _ in 0..FED_EVERY {
            d.step();
        }
        assert_eq!(d.piece_of_src[0], d.piece_of_src[1], "the bridge joined them");
        assert!(d.counts.snaps > before, "a bridge between two fed pieces did not flash");
    }

    /// Finding 3: on a free dish the shape stars are a ladder above "all
    /// joined". Doing nothing on seed 1 joined 4 of 6 and took the thrift
    /// star.
    #[test]
    fn shape_stars_need_all_the_food_joined() {
        let mut d = Dish::new(1);
        while d.phase != Phase::Over {
            d.step();
        }
        let s = d.score.clone().unwrap();
        assert!(
            s.reach == s.piles || !(s.stars[1] || s.stars[2]),
            "a shape star with {} of {} joined: {:?}",
            s.reach,
            s.piles,
            s.stars
        );
    }

    /// Finding 4: a snip on the Mould Organ heals, as on every other dish.
    #[test]
    fn an_organ_snip_heals() {
        let wounds = |d: &Dish| d.ground.iter().filter(|g| **g == Ground::Wound).count();
        let mut d = organ::new_dish(1);
        for _ in 0..2 * SPS {
            organ::step(&mut d);
        }
        d.snips_left = 3;
        assert!(d.begin_stroke(Tool::Snip, 100.0, 120.0));
        d.drag_stroke(130.0, 120.0);
        d.end_stroke();
        assert!(wounds(&d) > 0);
        for _ in 0..10 * SPS {
            organ::step(&mut d);
        }
        assert_eq!(wounds(&d), 0, "the organ's snip never healed");
    }

    /// Finding 7: a challenge dish with no mould left ends then, saying so.
    #[test]
    fn a_dish_with_no_mould_left_ends_and_says_so() {
        let mut d = Dish::from_level(&level::DISHES[0], 1);
        for _ in 0..2 * SPS {
            d.step();
        }
        while !d.walkers.is_empty() {
            d.remove_walker(0, Kill::Snip);
        }
        let t0 = d.t;
        while d.phase != Phase::Over {
            d.step();
            assert!(d.t < t0 + SPS, "a dead dish ran on toward the bell");
        }
        let c = d.card.clone().unwrap();
        assert_eq!(c.stars, 0);
        assert!(c.amount.starts_with("THE MOULD DIED"), "{}", c.amount);
    }

    /// Finding 8: Beat the Ruler's join agrees with the food-joined count:
    /// a one-cell break across a spoke is a break.
    #[test]
    fn the_ruler_join_does_not_hop_a_break() {
        let mut d = Dish::from_level(&level::DISHES[1], 1);
        d.walkers.clear();
        d.trail.fill(0.0);
        let piles: Vec<(f32, f32)> = d.dish_piles().map(|p| (p.x, p.y)).collect();
        let m = level::DISHES[1].meeting_point().0;
        for &p in &piles {
            lay(&mut d, m, p, 1.5, 10.0);
        }
        assert!(d.best_join().is_some(), "whole spokes join");
        let (a, b) = (m, piles[1]);
        let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        let l = (b.0 - a.0).hypot(b.1 - a.1);
        let (nx, ny) = (-(b.1 - a.1) / l, (b.0 - a.0) / l);
        for k in -8..=8 {
            if let Some(i) = cell(mid.0 + nx * k as f32 * 0.5, mid.1 + ny * k as f32 * 0.5) {
                d.trail[i] = 0.0;
            }
        }
        assert!(d.measure().reach < 3, "the break splits the network");
        assert_eq!(d.best_join(), None, "the ruler hopped a break the food count sees");
    }
}
