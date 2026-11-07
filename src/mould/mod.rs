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
//! * **The tools** each act on the model's own levers: the Lamp is a
//!   negative term in every sensor plus a per-cell fade, Oats are a small
//!   food pile (a steady trail deposit that also pays the store), Salt is
//!   ground a walker cannot step on, and Snip kills walkers and clears trail
//!   along a stroke.
//! * **The storm** at the end of a dish cuts the thickest tube (one cut; see
//!   `STORM_CUTS` for why not three), and the score is read before and after.

pub mod play;
pub mod render;

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
    /// sensor reads `trail - light_weight`.
    pub light_weight: f32,
    /// Fraction of trail a fully lit cell loses per step, on top of `keep`:
    /// "the tube under the lamp thins".
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
    /// Chance per step that a walker in full light dies.
    pub burn: f32,
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
            light_weight: 400.0,
            light_fade: 0.25,
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
            burn: 1.0 / 400.0,
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
/// Seconds after the storm at which "still joined" is read.
pub const AFTER_SECONDS: u32 = 2;
/// The thrift star: the network must be no more than this many times the
/// shortest possible length, and join at least `THRIFT_MIN_JOINED` piles (a
/// lone tube is thrifty by default). Set from the probe's first census,
/// where clean single tubes read 1.0-1.3x and branchy networks 1.6-1.9x.
pub const THRIFT_BAR: f32 = 1.5;
pub const THRIFT_MIN_JOINED: u32 = 3;
/// Cells of tube width that make one cell of network length. Calibrated on
/// the probe's straight home-to-pile tubes, which are about 12 cells wide at
/// the `tube` threshold; at 4 a straight tube read as 3.8x its own length.
pub const TUBE_WIDTH: f32 = 12.0;
/// How often (steps) the feeding search and pile census run.
const FED_EVERY: u64 = 6;

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
            Tool::Lamp => "HOLD CLICK: LAMP. THE MOULD FLEES LIGHT",
            Tool::Oats => "CLICK: DROP OATS. THE MOULD WILL REACH FOR THEM",
            Tool::Salt => "DRAG: A SALT LINE THE MOULD CANNOT CROSS",
            Tool::Snip => "DRAG: CUT THE MOULD",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Growing,
    /// The storm struck at this step.
    Storm(u64),
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
    /// Recent snip strokes, for drawing a mark that fades: (x, y, step).
    pub snip_marks: Vec<(f32, f32, u64)>,
    pub counts: Counts,
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

impl Dish {
    pub fn new(seed: u64) -> Self {
        Self::with_params(seed, Params::default())
    }

    pub fn with_params(seed: u64, params: Params) -> Self {
        let mut rng = Rng::new(seed);
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
        let piles = layout(&mut rng)
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
            store: START_STORE,
            lamp: None,
            oats_left: OATS_STOCK,
            salt_left: SALT_STOCK,
            snips_left: SNIP_STOCK,
            stroke: None,
            phase: Phase::Growing,
            before: None,
            score: None,
            after: None,
            cuts: Vec::new(),
            snip_marks: Vec::new(),
            counts: Counts::default(),
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
                    dish.walkers.push(Walker { x, y, heading });
                    placed += 1;
                }
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
        let end = Self::dish_steps();
        if self.t >= end {
            0
        } else {
            ((end - self.t) as u32).div_ceil(STEPS_PER_SECOND)
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
        if *left == 0 {
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
                for (ox, oy) in disc(2.5) {
                    if let Some(i) = cell(x + ox, y + oy) {
                        self.trail[i] = 0.0;
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
        self.deposit_food();
        self.move_walkers();
        self.diffuse();
        if self.t.is_multiple_of(FED_EVERY) {
            self.census();
        }
        self.economy();
        self.expire_oats();
        self.phase_tick();
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
        let r = LAMP_R;
        let x0 = (lx - r).floor().max(0.0) as usize;
        let y0 = (ly - r).floor().max(0.0) as usize;
        let x1 = ((lx + r).ceil() as usize + 1).min(W);
        let y1 = ((ly + r).ceil() as usize + 1).min(H);
        for y in y0..y1 {
            for x in x0..x1 {
                let d = ((x as f32 + 0.5 - lx).powi(2) + (y as f32 + 0.5 - ly).powi(2)).sqrt();
                // A bright flat core and a soft edge, so a sensor reads a
                // slope before the walker is in the light.
                let l = ((r - d) / (r * 0.5)).clamp(0.0, 1.0);
                self.light[idx(x, y)] = l;
            }
        }
        self.light_box = Some((x0, y0, x1, y1));
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
        for w in &self.walkers {
            for p in self.piles.iter_mut() {
                if p.food > 0.0 && (p.x - w.x).powi(2) + (p.y - w.y).powi(2) <= (p.r + 1.0).powi(2) {
                    p.eaters += 1;
                }
            }
        }
        // Breadth-first along tube from every pile the mould is eating.
        self.fed.fill(u16::MAX);
        let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
        for p in &self.piles {
            if !p.reached() {
                continue;
            }
            for (ox, oy) in disc(p.r + 1.0) {
                if let Some(i) = cell(p.x + ox, p.y + oy) {
                    if self.ground[i] == Ground::Agar && self.fed[i] != 0 {
                        self.fed[i] = 0;
                        queue.push_back(i);
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
            let (x, y) = (i % W, i / W);
            let mut visit = |j: usize, fed: &mut Vec<u16>| {
                if self.ground[j] == Ground::Agar && self.trail[j] > tube && fed[j] == u16::MAX {
                    fed[j] = d + 1;
                    queue.push_back(j);
                }
            };
            if x > 0 {
                visit(i - 1, &mut self.fed);
            }
            if x + 1 < W {
                visit(i + 1, &mut self.fed);
            }
            if y > 0 {
                visit(i - W, &mut self.fed);
            }
            if y + 1 < H {
                visit(i + W, &mut self.fed);
            }
        }
    }

    fn economy(&mut self) {
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
        match self.phase {
            Phase::Growing if self.t + 1 >= Self::dish_steps() => {
                self.lamp = None;
                self.stroke = None;
                self.update_light();
                let before = self.measure();
                self.storm();
                self.before = Some(before);
                self.phase = Phase::Storm(self.t);
            }
            // Read the damage once the storm has passed, before the mould has
            // had time to grow a way round the scars; then let it play out.
            Phase::Storm(at) if self.t == at + (AFTER_SECONDS * STEPS_PER_SECOND) as u64 => {
                self.after = Some(self.measure().reach);
            }
            Phase::Storm(at) if self.t >= at + Self::storm_steps() => {
                let mut s = self.before.clone().unwrap_or_default();
                s.after = self.after.unwrap_or(0).min(s.reach);
                let joined_two = s.reach >= 2;
                s.stars = [
                    s.reach == s.piles,
                    s.reach >= THRIFT_MIN_JOINED && s.ratio() <= THRIFT_BAR,
                    joined_two && s.after == s.reach,
                ];
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

    /// The storm strikes the thickest tubes of the best-joined piece,
    /// away from the piles themselves, and leaves a scar nothing crosses.
    fn storm(&mut self) {
        let (label, n) = self.pieces();
        let (best, _) = self.best_piece(&label, n);
        let mut cand: Vec<(f32, usize)> = Vec::new();
        for (i, &l) in label.iter().enumerate() {
            if l == best && best != 0 {
                let (x, y) = ((i % W) as f32 + 0.5, (i / W) as f32 + 0.5);
                let near_pile = self
                    .piles
                    .iter()
                    .any(|p| !p.oats && (p.x - x).powi(2) + (p.y - y).powi(2) <= (p.r + 12.0).powi(2));
                if !near_pile {
                    cand.push((self.trail[i], i));
                }
            }
        }
        cand.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut at: Vec<(f32, f32)> = Vec::new();
        for (_, i) in cand {
            let (x, y) = ((i % W) as f32 + 0.5, (i / W) as f32 + 0.5);
            if at.iter().all(|c| (c.0 - x).powi(2) + (c.1 - y).powi(2) >= 40.0 * 40.0) {
                at.push((x, y));
                if at.len() == STORM_CUTS {
                    break;
                }
            }
        }
        let mut cuts = Vec::new();
        for &(x, y) in &at {
            // The tube's direction here: the long axis of the tube cells
            // within 8 cells. The cut runs across it.
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
            let a = (x - cx * CUT_HALF, y - cy * CUT_HALF);
            let b = (x + cx * CUT_HALF, y + cy * CUT_HALF);
            let steps = (CUT_HALF * 2.0).ceil() as usize;
            for k in 0..=steps {
                let f = k as f32 / steps as f32;
                let (px, py) = (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f);
                for (ox, oy) in disc(1.5) {
                    if let Some(i) = cell(px + ox, py + oy) {
                        if self.ground[i] == Ground::Agar && !self.on_pile(i) {
                            self.ground[i] = Ground::Scar;
                            self.trail[i] = 0.0;
                            self.scent[i] = 0.0;
                        }
                    }
                }
                self.kill_in(px, py, 2.0, Kill::Storm);
            }
            cuts.push((a, b));
        }
        self.cuts = cuts;
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

    /// The positive control for the lamp: light must empty the cells under it
    /// within a second, against the same dish with no lamp.
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
            under(&lit) * 4 < under(&dark),
            "lamp {} vs dark {}",
            under(&lit),
            under(&dark)
        );
    }

    #[test]
    fn a_dish_ends_with_a_score() {
        let mut d = Dish::new(1);
        while d.phase != Phase::Over {
            d.step();
        }
        let s = d.score.clone().unwrap();
        assert_eq!(s.piles, 5);
        assert!(s.reach >= 1);
        assert!(d.cuts.len() <= STORM_CUTS);
    }
}
