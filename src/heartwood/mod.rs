//! **Heartwood: your deck is your tree's genes.** `cargo run --release --bin heartwood`
//!
//! Probe 8 of the 2026-11-07 brainstorm (`/mnt/project-files/game-design/
//! new-game-brainstorm-2026-11-07/prototypes/8-heartwood.md`). Two trees, one
//! valley, one sun. Your tree grows only where you play a card: you click one
//! of its growing tips and play a card from your hand, and the card is added
//! to that tip's **queue**. A tip works through its queue in order. Win by
//! having more of the valley's lit leaves than the rival tree.
//!
//! **The question it answers**: does playing cards onto tips feel like
//! steering a living tree, or like a slow way of drawing one -- and can a
//! player say, before a card, what the tree will look like after it?
//!
//! **The cards are a wand, and that is the whole design.** Each tip carries a
//! queue of cards, played one after another. *Fork* splits the tip, and each
//! half takes **a copy of the rest of the queue** -- so `Fork, Fork, Lean,
//! Leaf Out` on one tip is four limbs leaning and leafing, the Noita
//! multicast. *Bud* takes the card after it off the queue and holds it on the
//! stem where the tip was standing; if the branch above that point is ever
//! lost (cut, or broken under its own weight) the held card grows from there.
//!
//! **What is the engine's and what is the game's.** The tree is the shipped
//! `tree` species on the shipped plant engine. The game adds three things,
//! all through `sim::tip_cards` and the world clock, all off in every other
//! binary: a card-grown tree's shoot tips wait for a card instead of growing
//! on their own; a carried card replaces a tip's steering; and plants run
//! [`SPEEDUP`] times faster. Every price, every leaf, the wood thickening
//! under its leaves, the upkeep and the break of a heavy crown are the
//! engine's own, which is what keeps a card from being a paintbrush.
//!
//! **Scoring.** Light share is your lit leaf light over both trees' -- the
//! same per-cell light (`plant::ambient_light_above`) the leaves photosynthesise
//! on, summed over each tree's leaf cells. It is on screen at all times, and
//! banked at the end of each of [`SEASONS`] seasons. The last season's share
//! gives the stars, at 40 / 55 / 70 %.
//!
//! **Sugar** is what your leaves earn, in the same light: the purse fills at
//! [`SUGAR_PER_LIGHT`] per unit of lit leaf light per second, and every card
//! has a price. A tree with no card does not grow, so doing nothing loses.

pub mod hud;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::sim::cell::{Cell, OrganismId};
use crate::sim::explosion::Blasts;
use crate::sim::material;
use crate::sim::organism::{self, CellType};
use crate::sim::particle::ParticleSystem;
use crate::sim::player;
use crate::sim::tip_cards::{Steer, TipCards};
use crate::sim::world::World;

/// World size. `common.md`'s probe size: big enough for two crowns, small
/// enough to read on a phone at 3x.
pub const W: i32 = 256;
pub const H: i32 = 160;
/// First soil row.
pub const GROUND_Y: i32 = 118;
const SOIL_DEPTH: i32 = 32;
/// The rock shelf under the rival: where it starts, and how much soil is on it.
pub const SHELF_FROM: i32 = 136;
/// Soil over the shelf: thin, so the rival's roots are shallow.
pub const SHELF_SOIL: i32 = 8;
/// **Clear weather, always.** Live weather put snow where my seed goes on
/// seed 3 and the planting failed; and a probe of light share should not
/// also be a probe of cloud.
pub const WEATHER: Option<crate::sim::weather::Weather> = Some(crate::sim::weather::Weather::CLEAR);
/// Where the two seeds go in.
pub const MY_X: i32 = 80;
pub const RIVAL_X: i32 = 176;
/// How much faster plants run here than in the lab's outdoor baseline. 45
/// frames a tick becomes 7: a 16-cell card takes about two seconds.
pub const SPEEDUP: u32 = 6;
/// Frames the rival grows before you get your seed. It is a tree two
/// "seasons" older than yours, as the design asks.
pub const RIVAL_HEAD_START: u64 = 600;
/// Frames a tree with no tip waits before it sprouts again.
const RESPROUT_AFTER: u64 = 60;
/// Frames per season, and how many there are.
pub const SEASON_FRAMES: u64 = 2700;
pub const SEASONS: usize = 3;
/// Hand size, and how often a card is drawn.
pub const HAND: usize = 5;
pub const DRAW_EVERY: u64 = 200;
/// Grow visits in a row a card's tip may fail to step before the card is
/// called off as blocked. A visit comes every `45 / SPEEDUP` ~ 8 frames on a
/// small tree and up to five times less often on a big one, so this is about
/// one to five seconds.
pub const STALL_LIMIT: u16 = 8;
/// Frames between a carded tip's steps: a 40-cell card lands in under a
/// second and a half. The playtest's verdict on 3-16 cells over two seconds
/// was that "no single play reads".
pub const CARD_STEP_FRAMES: u64 = 2;
/// The valley every game is played in (`Heartwood::new`). Seed 1's rival
/// was mid-sized in the 12-seed table: 1,568 leaves at year end, untouched.
pub const VALLEY_SEED: u64 = 1;
/// Share of the species' turgor ceiling the outline treats as reach.
pub const REACH_SHARE: f32 = 0.9;
/// **The sun is low, morning from the left and evening from the right**
/// (rise 2 in run 3, about 34 degrees), and a leaf's light is the mean of
/// the two. So a crown shades whatever stands beside it, down-sun: you shade
/// the rival in the morning and it shades you in the evening.
///
/// Measured with one sun only (plan, key-masher, random, top tip; seeds
/// 1-6): from the left, behind you, any tree shades the rival and the plan
/// only edged the key-masher (67.5% vs 60.8% final light, 4-2); from the
/// right, behind the rival, its crown left every player 0% of the light and
/// no sugar to play with.
pub const SUNS: [(i32, i32); 2] = [(-3, -2), (3, -2)];
/// How much of the sun a leaf lets through; wood, soil and stone stop it.
pub const LEAF_PASSES: f32 = 0.8;
/// Sugar a second for each cell of wood above ground: the purse pays the
/// tree's upkeep, so a big bare tree runs dry.
pub const UPKEEP_PER_WOOD: f32 = 0.002;
/// How much nearer the roots, in cells of path, a bud released by a stuck tip
/// must be: far enough back that the new shoot has reach to spend.
pub const BUD_DROP: u16 = 20;
/// `wood`'s breaking strength in this valley, against the shipped 21,000.
/// Measured over six seeds with nobody playing: at 21,000 the rival tree --
/// growing at six times speed, while big plants' upkeep (and so their
/// thickening) runs at the size cadence -- snapped at the base and was lost
/// whole on 2 of 6 (2,056 cells in one snap on seed 2); at 40,000 it stood
/// on all six, and 60,000 changed nothing further. A crown that is heavy
/// enough still breaks.
pub const WOOD_STRENGTH: f32 = 40000.0;
/// Sugar earned per second per unit of lit leaf light. At 0.02 sugar piled
/// up past 400 unspent and every card was free; at 0.008 a good tree runs
/// short now and then, and a player chooses (plan bot: 70-93 cards a year).
pub const SUGAR_PER_LIGHT: f32 = 0.026;
pub const START_SUGAR: f32 = 12.0;
/// Prunes you hold, and how long one takes to come back.
pub const PRUNES: u32 = 2;
pub const PRUNE_BACK: u64 = 1200;
/// Stars, on the final season's light share.
pub const STARS: [f32; 3] = [0.40, 0.55, 0.70];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CardKind {
    GrowUp,
    Lean,
    Fork,
    LeafOut,
    Bud,
}

impl CardKind {
    pub const ALL: [CardKind; 5] = [CardKind::GrowUp, CardKind::Lean, CardKind::Fork, CardKind::LeafOut, CardKind::Bud];

    pub fn name(self) -> &'static str {
        match self {
            CardKind::GrowUp => "GROW UP",
            CardKind::Lean => "LEAN TO SUN",
            CardKind::Fork => "FORK",
            CardKind::LeafOut => "LEAF OUT",
            CardKind::Bud => "BUD",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            CardKind::GrowUp => "UP",
            CardKind::Lean => "LEAN",
            CardKind::Fork => "FORK",
            CardKind::LeafOut => "LEAF",
            CardKind::Bud => "BUD",
        }
    }

    pub fn cost(self) -> f32 {
        match self {
            CardKind::GrowUp => 6.0,
            CardKind::Lean => 6.0,
            CardKind::Fork => 8.0,
            CardKind::LeafOut => 8.0,
            CardKind::Bud => 2.0,
        }
    }

    /// Steps the card grows.
    pub fn cells(self) -> u8 {
        match self {
            CardKind::GrowUp => 40,
            CardKind::Lean => 36,
            CardKind::Fork => 8,
            CardKind::LeafOut => 14,
            CardKind::Bud => 0,
        }
    }

    /// What it will do, in the words on screen.
    pub fn sentence(self) -> &'static str {
        match self {
            CardKind::GrowUp => "40 CELLS STRAIGHT UP",
            CardKind::Lean => "36 CELLS CURVING TOWARD THE SUN",
            CardKind::Fork => "8 CELLS, THEN SPLITS IN TWO. EVERY CARD AFTER IT GOES TO BOTH",
            CardKind::LeafOut => "14 CELLS WITH LEAVES AT EVERY STEP",
            CardKind::Bud => "KEEPS THE NEXT CARD HERE. IT GROWS IF THIS BRANCH IS LOST",
        }
    }
}

/// The deck: 14 cards, 5 kinds. The design's 14 with its three unbuilt kinds
/// (Thicken, Deep Root, When Shaded) folded into the five that are.
pub const DECK: [CardKind; 16] = [
    CardKind::Fork,
    CardKind::Fork,
    CardKind::Fork,
    CardKind::GrowUp,
    CardKind::GrowUp,
    CardKind::GrowUp,
    CardKind::GrowUp,
    CardKind::Lean,
    CardKind::Lean,
    CardKind::Lean,
    CardKind::LeafOut,
    CardKind::LeafOut,
    CardKind::LeafOut,
    CardKind::LeafOut,
    CardKind::LeafOut,
    CardKind::Bud,
];

/// A bud the player set: a card held on the stem until the branch above it
/// is lost.
#[derive(Clone, Debug)]
pub struct BudMark {
    /// The stem cell it sits on.
    pub at: (i32, i32),
    /// The tip that grew on from here; the bud fires when neither it nor
    /// anything forked from it is left.
    pub above: u32,
    pub held: VecDeque<CardKind>,
    pub fired: bool,
}

/// What happened, for the screen and for the harness.
#[derive(Clone, Debug, Default)]
pub struct Tally {
    pub cards_played: u32,
    /// Cards called off because their tip could not move.
    pub blocked: u32,
    /// Cards the outline showed red, refused before they were paid for.
    pub refused: u32,
    pub refused_why: BTreeMap<&'static str, u32>,
    /// Buds that broke lower down because a tip above was stuck.
    pub buds_released: u32,
    /// Per card run to the end: (cells between where the outline said it
    /// would end and where it did, cells the outline said it would travel).
    pub predictions: Vec<(f32, f32)>,
    /// ...by reason, in the words the screen uses.
    pub blocked_why: BTreeMap<&'static str, u32>,
    pub sugar_spent: f32,
    pub sugar_earned: f32,
    pub upkeep_paid: f32,
    /// Branches that came off: one per tick in which the tree lost a run of
    /// cells (cut or broken), not one per cell.
    pub limbs_lost: u32,
    pub cells_lost: u32,
    pub prunes: u32,
    pub buds_fired: u32,
    pub forks: u32,
    /// Light share banked at the end of each season.
    pub seasons: Vec<f32>,
}

/// A message for the screen, with the frame it was said on.
#[derive(Clone, Debug)]
pub struct Note {
    pub text: String,
    pub frame: u64,
}

pub struct Heartwood {
    pub world: World,
    pub particles: ParticleSystem,
    pub blasts: Blasts,
    pub tuning: player::Tuning,
    pub seed: u64,
    pub me: OrganismId,
    pub rival: OrganismId,
    /// Game frames since your seed went in (the rival's head start is not
    /// counted).
    pub t: u64,
    pub hand: Vec<CardKind>,
    pub deck: Vec<CardKind>,
    pub sugar: f32,
    /// Selected tips, by tag.
    pub selected: BTreeSet<u32>,
    /// Each tip's queue of cards still to play.
    pub queues: BTreeMap<u32, VecDeque<CardKind>>,
    /// Which tip each forked tip came from.
    pub parent: BTreeMap<u32, u32>,
    /// The card each tip is growing now.
    pub doing: BTreeMap<u32, CardKind>,
    /// Tips whose last card was called off, and why, until a card moves them.
    pub blocked: BTreeMap<u32, &'static str>,
    /// Outline start and end for each card being grown, for the prediction test.
    pub predicted: BTreeMap<u32, Outline>,
    pub buds: Vec<BudMark>,
    pub prunes: u32,
    prune_clock: u64,
    pub tally: Tally,
    pub notes: Vec<Note>,
    /// Lit leaf light, mine and the rival's, as of the last count.
    pub light: (f32, f32),
    pub over: bool,
    rng: u64,
    last_cells: usize,
    /// Tips seen last tick, by tag, for the lost-tip test.
    live_tags: BTreeSet<u32>,
    /// Frames your tree has gone with no growing tip at all.
    tipless: u64,
    /// What your tree's leaves earn and its standing tissue costs, per
    /// plant tick, smoothed: the starving readout.
    pub earns: f32,
    pub costs: f32,
    /// Your wood cells above ground, for upkeep.
    pub wood: usize,
    /// Leaves of either tree the low sun does not reach, for the screen.
    pub dark: Vec<(i32, i32)>,
}

impl Heartwood {
    /// A fresh valley. Deterministic in `seed`.
    pub fn new(seed: u64) -> Self {
        let mut world = World::new(crate::sim::chunk::Rect::new(0, 0, W - 1, H - 1));
        // **The same valley every game** (playtest 2026-11-08): with the
        // world seeded per game the rival ranged from 211 to 2,900 leaves
        // and the same plan scored 6% on one valley and 60% on another --
        // the seed mattered more than the player. The world is now always
        // `VALLEY_SEED`; `seed` shuffles only the deck.
        world.seed = VALLEY_SEED;
        // Noon, held: a valley game, not a day/night one (the design's moving
        // sun is left out -- light here falls straight down a column, so the
        // sun's path could not move a shadow anyway).
        world.clock.set_rates(0, |c| {
            c.sky_hold = Some(0);
            c.growth_speedup = SPEEDUP;
        });
        world.tip_cards = Some(Box::new(TipCards { speedup: SPEEDUP, step_frames: CARD_STEP_FRAMES, ..Default::default() }));
        // **No bending**, as in the lab box: a card-grown stem is one cell
        // thick when it is new, and with bending on every Lean sagged into a
        // wobbling S that the next card could not be aimed from. Breaking
        // under load stays on -- that is the design's planned break -- and
        // bending off also skips building the stress field for the lean
        // (28% of the plant pass, `World::plant_bending`'s doc).
        world.plant_bending = false;
        // **Big plants tick less often** (`World::plant_size_cadence`, the
        // lab's own setting). Measured on seed 1, idle, three seasons: mean
        // tick 3.33 -> 0.55 ms, p99 33 -> 17 ms. At six times the plant
        // clock a full-grown rival's economy pass was landing every seventh
        // frame.
        world.plant_size_cadence = true;
        // **A still, clear valley.** Weather is a lottery the probe does not
        // ask about: on seed 3 the sky snows from frame 0 and buried the
        // seed bed, and a gust breaks a crown whatever was played on it.
        world.weather_override = WEATHER;
        build_valley(&mut world);
        // **No seedlings** (the design's *Drop Seed* is left out): set the
        // tree's seed-setting size out of reach, in this world's own copy of
        // the species registry. Without it the rival had sown the valley
        // with 170-odd seedlings before your seed went in.
        if let Some(wood) = world.materials.id_of("wood") {
            world.materials.get_mut(wood).strength = WOOD_STRENGTH;
        }
        let sp = world.species.id_of("tree").expect("the tree species is compiled in");
        assert!(
            world.species.set_param(sp, CellType::MatureBody, organism::ParamId::SeedMaturity, 0, 1.0e9),
            "the tree's Reproduce behaviour moved"
        );
        let mut game = Heartwood {
            world,
            particles: ParticleSystem::new(),
            blasts: Blasts::new(),
            tuning: player::Tuning::default(),
            seed,
            me: 0,
            rival: 0,
            t: 0,
            hand: Vec::new(),
            deck: Vec::new(),
            sugar: START_SUGAR,
            selected: BTreeSet::new(),
            queues: BTreeMap::new(),
            parent: BTreeMap::new(),
            doing: BTreeMap::new(),
            blocked: BTreeMap::new(),
            predicted: BTreeMap::new(),
            buds: Vec::new(),
            prunes: PRUNES,
            prune_clock: 0,
            tally: Tally::default(),
            notes: Vec::new(),
            light: (0.0, 0.0),
            over: false,
            rng: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
            last_cells: 0,
            live_tags: BTreeSet::new(),
            tipless: 0,
            earns: 0.0,
            costs: 0.0,
            wood: 0,
            dark: Vec::new(),
        };
        game.rival = plant(&mut game.world, RIVAL_X);
        for _ in 0..RIVAL_HEAD_START {
            game.step_world();
        }
        game.me = plant(&mut game.world, MY_X);
        if let Some(t) = game.world.tip_cards.as_deref_mut() {
            t.organisms.push(game.me);
        }
        for _ in 0..HAND {
            game.draw_card();
        }
        game.say("CLICK A TIP OF YOUR TREE, THEN PLAY A CARD");
        game
    }

    fn step_world(&mut self) {
        crate::sim::frame::step(&mut self.world, &mut self.particles, &mut self.blasts, player::PlayerInput::default(), &self.tuning);
    }

    fn rand(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn draw_card(&mut self) {
        if self.hand.len() >= HAND {
            return;
        }
        if self.deck.is_empty() {
            self.deck = DECK.to_vec();
            // Fisher-Yates.
            for i in (1..self.deck.len()).rev() {
                let j = (self.rand() % (i as u64 + 1)) as usize;
                self.deck.swap(i, j);
            }
        }
        if let Some(c) = self.deck.pop() {
            self.hand.push(c);
        }
    }

    pub fn say(&mut self, text: &str) {
        self.notes.push(Note { text: text.to_string(), frame: self.t });
        if self.notes.len() > 4 {
            self.notes.remove(0);
        }
    }

    /// The season you are in, from 0; `SEASONS` once the game is over.
    pub fn season(&self) -> usize {
        ((self.t / SEASON_FRAMES) as usize).min(SEASONS)
    }

    /// Every tip of your tree, by tag, with where it is now.
    pub fn tips(&self) -> Vec<(u32, (i32, i32))> {
        self.world.tip_cards.as_deref().map_or(Vec::new(), |t| t.steer.iter().map(|(&p, s)| (s.tag, p)).collect())
    }

    pub fn tip_at(&self, tag: u32) -> Option<(i32, i32)> {
        self.world.tip_cards.as_deref()?.steer.iter().find(|(_, s)| s.tag == tag).map(|(&p, _)| p)
    }

    /// Whether a tip has nothing left to do: no card growing, nothing queued.
    pub fn idle(&self, tag: u32) -> bool {
        !self.doing.contains_key(&tag) && self.queues.get(&tag).is_none_or(|q| q.is_empty())
    }

    /// The tip nearest `(x, y)` within `reach` cells.
    pub fn nearest_tip(&self, (x, y): (i32, i32), reach: i32) -> Option<u32> {
        self.tips()
            .into_iter()
            .map(|(tag, (tx, ty))| (tag, (tx - x).pow(2) + (ty - y).pow(2)))
            .filter(|&(_, d)| d <= reach * reach)
            .min_by_key(|&(tag, d)| (d, tag))
            .map(|(tag, _)| tag)
    }

    /// Select one tip (or add it, with `add`).
    /// Pick one tip. **One card, one tip** (playtest 2026-11-08): picking
    /// several and playing one card onto all of them beat a plan 9-3 (51.8%
    /// mean light against 42.0%) and made Fork, the one card meant to
    /// multicast, pointless. Fork is the only way one card reaches two tips.
    pub fn select(&mut self, tag: u32) {
        self.selected.clear();
        self.selected.insert(tag);
    }

    /// Play hand card `i` onto the picked tip. Returns whether it played.
    /// A card the outline shows red is refused before it is paid for.
    pub fn play(&mut self, i: usize) -> bool {
        if self.over {
            return false;
        }
        let Some(&card) = self.hand.get(i) else { return false };
        let Some(tag) = self.selected.iter().copied().find(|&t| self.tip_at(t).is_some()) else {
            self.say("FIRST CLICK A TIP OF YOUR TREE");
            return false;
        };
        if let Verdict::No(why) = self.verdict(card, tag) {
            self.tally.refused += 1;
            *self.tally.refused_why.entry(why).or_default() += 1;
            self.say(&format!("CANNOT PLAY {}: {}", card.name(), why));
            return false;
        }
        let price = card.cost();
        self.sugar -= price;
        self.tally.sugar_spent += price;
        self.tally.cards_played += 1;
        self.hand.remove(i);
        self.queues.entry(tag).or_default().push_back(card);
        self.say(&format!("{}: {}", card.name(), card.sentence()));
        true
    }

    /// **What the outline says before you pay**, for `card` on tip `tag`:
    /// red with a reason, amber when it will stop short, or fine. Judged
    /// against the tip's own state now, so a card already queued on the tip
    /// is not counted (the outline is drawn from where the tip stands).
    pub fn verdict(&self, card: CardKind, tag: u32) -> Verdict {
        if self.sugar + 1e-4 < card.cost() {
            return Verdict::No("NOT ENOUGH SUGAR");
        }
        if card == CardKind::Bud {
            return Verdict::Fine;
        }
        let Some(at) = self.tip_at(tag) else { return Verdict::No("THAT TIP IS GONE") };
        // Water: the engine's turgor bound is a path length from the collar
        // (`plant::authored_height_ceiling_for`), and a tip past it cannot
        // step at all.
        let path = self.world.organism_cell(at.0, at.1).map_or(0, |c| c.path_len) as f32;
        let reach = self.reach() - path;
        if reach < 3.0 {
            return Verdict::No("TOO TALL FOR WATER TO REACH");
        }
        // Room: a card's tip takes its least-bad open cell, so it is walled
        // in only when no neighbour is open at all -- the engine's `Boxed`.
        // (Reading the outline's first cells instead refused most cards: a
        // tip sits in its own leaf spray, and steps out of it sideways.)
        let open = |dx: i32, dy: i32| self.world.in_bounds(at.0 + dx, at.1 + dy) && self.world.get(at.0 + dx, at.1 + dy).is_empty();
        if !(-1..=1).any(|dy| (-1..=1).any(|dx| (dx, dy) != (0, 0) && open(dx, dy))) {
            return Verdict::No("NO ROOM TO GROW");
        }
        if reach < card.cells() as f32 {
            return Verdict::Short(reach as u32);
        }
        Verdict::Fine
    }

    /// Cells of path from the collar this tree can lift water to: the
    /// species' authored turgor ceiling, less a little for the taper the
    /// engine starts rolling against near it.
    fn reach(&self) -> f32 {
        let sp = self.world.organism(self.me).map(|s| s.species);
        sp.map_or(0.0, |sp| crate::sim::plant::authored_height_ceiling_for(&self.world, sp) * REACH_SHARE)
    }

    /// Cut your own tree at `(x, y)`: every cell of it within two cells goes,
    /// and what that leaves unsupported falls as the engine drops any cut
    /// limb. Returns whether anything was cut.
    pub fn prune(&mut self, (x, y): (i32, i32)) -> bool {
        if self.prunes == 0 || self.over {
            if self.prunes == 0 {
                self.say(&format!("NO PRUNES LEFT. ONE COMES BACK EVERY {} S", PRUNE_BACK / 60));
            }
            return false;
        }
        let mut cut = 0;
        for dy in -2..=2 {
            for dx in -2..=2 {
                if dx * dx + dy * dy > 5 {
                    continue;
                }
                let (cx, cy) = (x + dx, y + dy);
                if cy >= GROUND_Y {
                    continue;
                }
                if self.world.get(cx, cy).organism_id() == self.me {
                    self.world.set(cx, cy, Cell::EMPTY);
                    self.world.schedule_structural_check_around(cx, cy);
                    cut += 1;
                }
            }
        }
        if cut == 0 {
            return false;
        }
        self.world.record_disturbance(x, y, 3);
        self.prunes -= 1;
        if self.prune_clock == 0 {
            self.prune_clock = self.t + PRUNE_BACK;
        }
        self.tally.prunes += 1;
        self.say("PRUNED");
        true
    }

    /// One game tick.
    pub fn tick(&mut self) {
        if self.over {
            // Play on: the world keeps living, only the score has stopped.
            self.step_world();
            self.sync_tips();
            return;
        }
        self.step_world();
        self.t += 1;
        self.sync_tips();
        self.feed_tips();
        self.fund_cards();
        if self.t.is_multiple_of(10) {
            let (mine, mine_dark, wood) = sun_on(&self.world, self.me);
            let (theirs, theirs_dark, _) = sun_on(&self.world, self.rival);
            self.light = (mine, theirs);
            self.wood = wood;
            self.dark = mine_dark;
            self.dark.extend(theirs_dark);
        }
        if let Some(st) = self.world.organism(self.me) {
            self.earns += (st.income - self.earns) * 0.01;
            self.costs += (st.maintenance - self.costs) * 0.01;
        }
        let earned = self.light.0 * SUGAR_PER_LIGHT / 60.0;
        let upkeep = self.wood as f32 * UPKEEP_PER_WOOD / 60.0;
        self.sugar = (self.sugar + earned - upkeep).max(0.0);
        self.tally.sugar_earned += earned;
        self.tally.upkeep_paid += upkeep;
        if self.t.is_multiple_of(DRAW_EVERY) {
            self.draw_card();
        }
        if self.prune_clock != 0 && self.t >= self.prune_clock {
            self.prunes = (self.prunes + 1).min(PRUNES);
            self.prune_clock = if self.prunes < PRUNES { self.t + PRUNE_BACK } else { 0 };
        }
        self.count_losses();
        self.resprout();
        if self.t.is_multiple_of(SEASON_FRAMES) {
            let share = self.share();
            self.tally.seasons.push(share);
            let n = self.tally.seasons.len();
            if n >= SEASONS {
                self.over = true;
                let stars = self.stars();
                self.say(&format!("THE YEAR IS OVER: {} STARS. N FOR A NEW VALLEY", stars));
            } else {
                self.say(&format!("SEASON {} OVER: YOU HAD {}% OF THE LIGHT", n, (share * 100.0).round() as i32));
            }
        }
    }

    /// Your share of the lit leaf light now.
    /// Sugar a second, after upkeep: what the purse shows.
    pub fn net_rate(&self) -> f32 {
        self.light.0 * SUGAR_PER_LIGHT - self.wood as f32 * UPKEEP_PER_WOOD
    }

    /// **The card pays for its cells.** A carded tip steps every
    /// `CARD_STEP_FRAMES`, far faster than the tree's own economy refills
    /// a tip, so the sugar spent on the card is what funds it: each frame a
    /// tip with a card running is topped up to a full cell of carbon.
    fn fund_cards(&mut self) {
        let Some(t) = self.world.tip_cards.as_deref() else { return };
        let carded: Vec<(i32, i32)> = t.steer.iter().filter(|(_, s)| s.cells_left > 0).map(|(&p, _)| p).collect();
        for (x, y) in carded {
            if let Some(c) = self.world.organism_cell_mut(x, y) {
                c.carbon = c.carbon.max(organism::RESOURCE_SCALE);
            }
        }
    }

    pub fn share(&self) -> f32 {
        let (a, b) = self.light;
        if a + b <= 0.0 {
            0.0
        } else {
            a / (a + b)
        }
    }

    /// Whether your tree is spending more on its wood than its leaves earn
    /// -- the engine's starvation, which sheds tissue from the tips inward.
    pub fn starving(&self) -> bool {
        self.costs > self.earns * 1.05 && self.t > 120
    }

    pub fn stars(&self) -> usize {
        let last = self.tally.seasons.last().copied().unwrap_or(0.0);
        STARS.iter().filter(|&&s| last >= s).count()
    }

    /// Give every new tip of your tree a tag; forget tips that are gone, and
    /// fire any bud whose branch they were.
    fn sync_tips(&mut self) {
        let me = self.me;
        let mut found: Vec<(i32, i32)> = Vec::new();
        if let Some(state) = self.world.organism(me) {
            for &(x, y) in state.cells.keys() {
                let c = self.world.get(x, y);
                if c.organism_id() == me && organism::cell_type(c.aux()) == Some(CellType::GrowingTip) {
                    found.push((x, y));
                }
            }
        }
        let Some(t) = self.world.tip_cards.as_deref_mut() else { return };
        // Forks the engine carried out: the new half inherits the rest of
        // the queue -- the multicast.
        let forks = std::mem::take(&mut t.forks);
        let found_set: BTreeSet<(i32, i32)> = found.iter().copied().collect();
        t.steer.retain(|p, _| found_set.contains(p));
        for p in &found {
            if !t.steer.contains_key(p) {
                let tag = t.new_tag();
                t.steer.insert(*p, Steer { tag, cells_left: 0, aim: (0.0, -1.0), aim_weight: 1.0, keep_weight: 0.3, leaf_every: false, fork: false, stalls: 0, why: 0 });
            }
        }
        // **A card that cannot move is called off, and says why.** A tip
        // walled in by its own leaves, past the height water can be lifted,
        // or too poor for one more cell would otherwise carry its card for
        // the rest of the year. The unspent share of the price comes back.
        let mut blocked: Vec<(u32, u8, u8, (i32, i32))> = Vec::new();
        for (&at, s) in t.steer.iter_mut() {
            if s.cells_left > 0 && s.stalls >= STALL_LIMIT {
                blocked.push((s.tag, s.why, s.cells_left, at));
                s.cells_left = 0;
                s.stalls = 0;
            }
        }
        let moving: Vec<(u32, u8)> = t.steer.values().filter(|s| s.cells_left > 0 && s.stalls == 0).map(|s| (s.tag, s.cells_left)).collect();
        let live: BTreeSet<u32> = t.steer.values().map(|s| s.tag).collect();
        let finished: Vec<(u32, (i32, i32))> = t.steer.iter().filter(|(_, s)| s.cells_left == 0).map(|(&at, s)| (s.tag, at)).collect();
        for f in forks {
            self.tally.forks += 1;
            self.parent.insert(f.child_tag, f.parent_tag);
            let q = self.queues.get(&f.parent_tag).cloned().unwrap_or_default();
            self.queues.insert(f.child_tag, q);
            if let Some(&card) = self.doing.get(&f.parent_tag) {
                self.doing.insert(f.child_tag, card);
            }
        }
        let mut released: Vec<(u32, (i32, i32))> = Vec::new();
        for (tag, why, left, at) in blocked {
            released.push((tag, at));
            if let Some(card) = self.doing.get(&tag).copied() {
                let refund = card.cost() * left as f32 / card.cells().max(1) as f32;
                self.sugar += refund;
                self.tally.blocked += 1;
                *self.tally.blocked_why.entry(blocked_reason(why)).or_default() += 1;
                self.blocked.insert(tag, blocked_reason(why));
                self.say(&format!("{} BLOCKED: {}", card.short(), blocked_reason(why)));
            }
        }
        for (tag, at) in finished {
            self.doing.remove(&tag);
            // **The prediction test**: where the outline said the card would
            // end, against where the tip actually stands.
            if let Some((from, to)) = self.predicted.remove(&tag) {
                if !released.iter().any(|r| r.0 == tag) {
                    let d = |a: (i32, i32), b: (i32, i32)| (((a.0 - b.0).pow(2) + (a.1 - b.1).pow(2)) as f32).sqrt();
                    self.tally.predictions.push((d(at, to), d(from, to)));
                }
            }
        }
        // A tip that has stepped under a card is no longer blocked.
        for (tag, left) in moving {
            if self.doing.get(&tag).is_some_and(|c| left < c.cells()) {
                self.blocked.remove(&tag);
            }
        }
        let lost: Vec<u32> = self.live_tags.difference(&live).copied().collect();
        self.live_tags = live;
        for tag in lost {
            self.blocked.remove(&tag);
            self.selected.remove(&tag);
            self.queues.remove(&tag);
            self.doing.remove(&tag);
        }
        // **A tip at the top of its water stops, and a bud lower down
        // breaks** -- the same release as a stuck card, but before a card is
        // wasted on it. A carded tip is held by the card hook and never ages
        // out on its own, so without this a tip that reached the turgor
        // bound sat there for the rest of the year refusing every card
        // (2026-11-08: 213-485 refusals a year for the key-masher and the
        // plan, against 4-12 cards played).
        let reach = self.reach();
        let topped: Vec<(u32, (i32, i32))> = self
            .world
            .tip_cards
            .as_deref()
            .map(|t| {
                t.steer
                    .iter()
                    .filter(|(_, s)| s.cells_left == 0)
                    .filter(|(p, _)| self.world.organism_cell(p.0, p.1).is_some_and(|c| c.path_len as f32 + 3.0 > reach))
                    .map(|(&p, s)| (s.tag, p))
                    .collect()
            })
            .unwrap_or_default();
        let released: Vec<(u32, (i32, i32))> = released
            .into_iter()
            .chain(topped.into_iter().filter(|(tag, _)| self.queues.get(tag).is_none_or(|q| q.is_empty())))
            .collect();
        for (tag, at) in released {
            self.release_bud_below(tag, at);
        }
        self.check_buds();
        if self.selected.is_empty() {
            // Always have something selected, so a first card never
            // misses: the highest idle tip.
            if let Some(tag) = self.tips().into_iter().filter(|&(tag, _)| self.idle(tag)).min_by_key(|&(tag, (_, y))| (y, tag)).map(|(t, _)| t) {
                self.selected.insert(tag);
            }
        }
    }

    /// Hand each waiting tip the next card in its queue.
    fn feed_tips(&mut self) {
        let tips = self.tips();
        for (tag, (x, y)) in tips {
            if self.doing.contains_key(&tag) {
                continue;
            }
            let Some(q) = self.queues.get_mut(&tag) else { continue };
            let Some(card) = q.pop_front() else { continue };
            if card == CardKind::Bud {
                // The bud holds the card after it.
                let held: VecDeque<CardKind> = q.pop_front().into_iter().collect();
                let below = stem_below(&self.world, self.me, (x, y));
                self.buds.push(BudMark { at: below, above: tag, held, fired: false });
                self.say("A BUD IS SET ON THIS BRANCH");
                continue;
            }
            let heading = self.world.organism_cell(x, y).map_or((0.0, -1.0), |c| c.heading);
            let heading = if heading == (0.0, 0.0) { (0.0, -1.0) } else { heading };
            if matches!(card, CardKind::GrowUp | CardKind::Lean | CardKind::LeafOut) {
                if let Some(&end) = self.outline(card, tag).last() {
                    self.predicted.insert(tag, ((x, y), end));
                }
            }
            let steer = steer_for(&self.world, card, tag, (x, y), heading, self.rival_side());
            if let Some(t) = self.world.tip_cards.as_deref_mut() {
                t.steer.insert((x, y), steer);
            }
            self.doing.insert(tag, card);
        }
    }

    /// +1 if the rival stands to your right.
    fn rival_side(&self) -> f32 {
        if RIVAL_X >= MY_X {
            1.0
        } else {
            -1.0
        }
    }

    /// Whether `tag` or anything forked from it is still growing.
    fn lineage_alive(&self, tag: u32) -> bool {
        self.live_tags.iter().any(|&t| {
            let mut cur = t;
            loop {
                if cur == tag {
                    return true;
                }
                match self.parent.get(&cur) {
                    Some(&p) => cur = p,
                    None => return false,
                }
            }
        })
    }

    fn check_buds(&mut self) {
        let me = self.me;
        let mut fire: Vec<usize> = Vec::new();
        for (i, b) in self.buds.iter().enumerate() {
            if b.fired || self.lineage_alive(b.above) {
                continue;
            }
            fire.push(i);
        }
        for i in fire {
            let at = self.buds[i].at;
            self.buds[i].fired = true;
            let c = self.world.get(at.0, at.1);
            if c.organism_id() != me {
                // The stub went too.
                self.say("A BUD WAS LOST WITH ITS BRANCH");
                continue;
            }
            // Wake the stem cell as a growing tip, as the engine's own bud
            // break does, and give it the held card.
            let held = self.buds[i].held.clone();
            self.wake_tip(at, held);
            self.tally.buds_fired += 1;
            self.say("THE BUD WAKES: A NEW SHOOT FROM THE STUB");
        }
    }

    /// **A tree that has lost every tip sprouts again from the top of what
    /// is left** after a second, so a broken trunk is a setback rather than
    /// the end of the game. The engine's own regrowth is from dormant buds;
    /// this wakes the highest one standing (or, failing that, the highest
    /// stem cell) the way a bud break does. A planned Bud does the same
    /// thing sooner, wherever you put it, with a card waiting.
    fn resprout(&mut self) {
        if !self.tips().is_empty() {
            self.tipless = 0;
            return;
        }
        self.tipless += 1;
        if self.tipless < RESPROUT_AFTER {
            return;
        }
        let me = self.me;
        let Some(state) = self.world.organism(me) else { return };
        let mut best: Option<(bool, i32, (i32, i32))> = None;
        for &(x, y) in state.cells.keys() {
            if y >= GROUND_Y {
                continue;
            }
            let c = self.world.get(x, y);
            if c.organism_id() != me {
                continue;
            }
            let ty = organism::cell_type(c.aux());
            if !matches!(ty, Some(CellType::DormantBud | CellType::MatureBody)) {
                continue;
            }
            let key = (ty == Some(CellType::DormantBud), -y, (x, y));
            if best.is_none_or(|b| (key.0, key.1) > (b.0, b.1)) {
                best = Some(key);
            }
        }
        let Some((_, _, at)) = best else { return };
        self.wake_tip(at, VecDeque::new());
        self.tipless = 0;
        self.say("YOUR TREE SPROUTS AGAIN FROM WHAT IS LEFT");
    }

    /// **A stuck tip stops, and a bud lower down breaks** -- apical
    /// dominance released, which is what a real tree does when its leader
    /// can go no further. Without it a card-grown tree ran out of tips that
    /// could move by mid-year: on seed 3 every one of its eight tips was past
    /// the reach water can be lifted or walled in by leaves from frame ~2,100,
    /// the hand sat full, roots doubled on the unspent income, and at 5,300
    /// the trunk base starved and the crown fell whole. The new tip is the
    /// best-lit stem cell with open air beside it, at least `BUD_DROP` cells
    /// of path nearer the roots than the stuck one.
    fn release_bud_below(&mut self, tag: u32, at: (i32, i32)) {
        let me = self.me;
        let reach = self.world.organism_cell(at.0, at.1).map_or(0, |c| c.path_len);
        let Some(state) = self.world.organism(me) else { return };
        let mut best: Option<(f32, (i32, i32))> = None;
        for &(x, y) in state.cells.keys() {
            if y >= GROUND_Y - 4 {
                continue;
            }
            let c = self.world.get(x, y);
            if c.organism_id() != me || !matches!(organism::cell_type(c.aux()), Some(CellType::DormantBud | CellType::MatureBody)) {
                continue;
            }
            if self.world.organism_cell(x, y).is_none_or(|o| o.path_len + BUD_DROP > reach) {
                continue;
            }
            let open = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0)].iter().filter(|&&(dx, dy)| self.world.get(x + dx, y + dy).is_empty()).count();
            if open < 3 {
                continue;
            }
            let light = crate::sim::plant::ambient_light_above(&self.world, x, y - 1);
            if best.is_none_or(|b| light > b.0) {
                best = Some((light, (x, y)));
            }
        }
        // Retire the stuck tip to wood either way: it cannot move.
        let c = self.world.get(at.0, at.1);
        if c.organism_id() == me && organism::cell_type(c.aux()) == Some(CellType::GrowingTip) {
            self.world.set(at.0, at.1, c.with_aux(organism::pack_cell_type(CellType::MatureBody)));
        }
        let was_selected = self.selected.remove(&tag);
        if let Some(t) = self.world.tip_cards.as_deref_mut() {
            t.steer.remove(&at);
        }
        let Some((_, bud)) = best else { return };
        let keep = self.selected.clone();
        self.wake_tip(bud, VecDeque::new());
        self.tally.buds_released += 1;
        if !was_selected {
            self.selected = keep;
        }
    }

    /// Turn a standing stem cell of your tree into a growing tip with
    /// `queue` waiting on it, as the engine's bud break does.
    fn wake_tip(&mut self, at: (i32, i32), queue: VecDeque<CardKind>) {
        let me = self.me;
        let c = self.world.get(at.0, at.1);
        self.world.set(at.0, at.1, c.with_aux(organism::pack_cell_type(CellType::GrowingTip)));
        let due = self.world.organism_due(crate::sim::plant::ORGANISM_TICK_INTERVAL);
        self.world.schedule_active_site(crate::sim::scheduler::ActiveSite {
            x: at.0,
            y: at.1,
            kind: crate::sim::scheduler::ActiveKind::Organism { organism: me, stale_ticks: 0, plastochron: 0 },
            next_frame: due,
        });
        if let Some(t) = self.world.tip_cards.as_deref_mut() {
            let tag = t.new_tag();
            t.steer.insert(at, Steer { tag, cells_left: 0, aim: (0.0, -1.0), aim_weight: 1.0, keep_weight: 0.3, leaf_every: false, fork: false, stalls: 0, why: 0 });
            self.live_tags.insert(tag);
            self.queues.insert(tag, queue);
            self.selected.clear();
            self.selected.insert(tag);
        }
    }

    fn count_losses(&mut self) {
        let n = self.world.organism(self.me).map_or(0, |s| s.cells.len());
        if self.last_cells > n + 6 {
            self.tally.limbs_lost += 1;
            self.tally.cells_lost += (self.last_cells - n) as u32;
            self.say("A BRANCH CAME DOWN");
        }
        self.last_cells = n;
    }

    /// Where the card in hand slot `i` would take the first selected tip:
    /// the dotted outline. A sketch of the engine's own steering (heading
    /// momentum toward the card's aim), not a replay of it.
    pub fn preview(&self, i: usize) -> Vec<(i32, i32)> {
        let Some(&card) = self.hand.get(i) else { return Vec::new() };
        let Some(&tag) = self.selected.iter().next() else { return Vec::new() };
        self.outline(card, tag)
    }

    /// The verdict on hand card `i` for the picked tip, as the outline shows it.
    pub fn preview_verdict(&self, i: usize) -> Option<Verdict> {
        let &card = self.hand.get(i)?;
        let &tag = self.selected.iter().next()?;
        Some(self.verdict(card, tag))
    }

    /// The dotted outline of `card` played on tip `tag`, from where it stands.
    fn outline(&self, card: CardKind, tag: u32) -> Vec<(i32, i32)> {
        let Some((x, y)) = self.tip_at(tag) else { return Vec::new() };
        let heading = self.world.organism_cell(x, y).map_or((0.0, -1.0), |c| c.heading);
        let heading = if heading == (0.0, 0.0) { (0.0, -1.0) } else { heading };
        let s = steer_for(&self.world, card, tag, (x, y), heading, self.rival_side());
        let mut out = Vec::new();
        let paths: Vec<(f32, f32)> = if card == CardKind::Fork {
            let side = (-heading.1, heading.0);
            vec![unit((heading.0 + side.0, heading.1 + side.1)), unit((heading.0 - side.0, heading.1 - side.1))]
        } else {
            vec![s.aim]
        };
        for aim in paths {
            let (mut px, mut py) = (x as f32, y as f32);
            let mut h = heading;
            for _ in 0..card.cells().max(1) {
                let want = unit((h.0 * s.keep_weight + aim.0 * s.aim_weight, h.1 * s.keep_weight + aim.1 * s.aim_weight));
                h = unit((h.0 * 0.85 + want.0 * 0.15, h.1 * 0.85 + want.1 * 0.15));
                if card == CardKind::Fork {
                    h = aim;
                }
                px += h.0;
                py += h.1;
                out.push((px.round() as i32, py.round() as i32));
            }
        }
        out
    }

    /// Which way Lean to Sun would go from the first selected tip: -1 left,
    /// +1 right.
    pub fn lean_side(&self) -> Option<f32> {
        let &tag = self.selected.iter().next()?;
        let at = self.tip_at(tag)?;
        Some(brighter_side(&self.world, at, self.rival_side()))
    }
}

/// The steering a card asks of a tip.
/// What the outline says about a card before it is played.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verdict {
    Fine,
    /// It will play, but water reaches only this many more cells.
    Short(u32),
    /// It will not play, and why, in the screen's words.
    No(&'static str),
}

/// A card's outline as played: where the tip stood, and where the outline ended.
pub type Outline = ((i32, i32), (i32, i32));

/// Plain words for why a card's tip could not move (`plant::GrowWhy as u8`).
fn blocked_reason(why: u8) -> &'static str {
    use why_code as w;
    match why {
        w::TOO_POOR | w::ORGAN => "TOO LITTLE SUGAR IN THE TIP",
        w::HEIGHT | w::TAPER => "TOO TALL FOR WATER TO REACH",
        w::BOXED | w::NO_DIRECTION => "NO ROOM TO GROW",
        w::HARD => "GROUND TOO HARD",
        _ => "CANNOT GROW HERE",
    }
}

/// `plant::GrowWhy` discriminants, named once.
mod why_code {
    use crate::sim::plant::GrowWhy;
    pub const ORGAN: u8 = GrowWhy::OrganUnaffordable as u8;
    pub const TOO_POOR: u8 = GrowWhy::TooPoor as u8;
    pub const HEIGHT: u8 = GrowWhy::HeightLimit as u8;
    pub const TAPER: u8 = GrowWhy::TaperRoll as u8;
    pub const BOXED: u8 = GrowWhy::Boxed as u8;
    pub const HARD: u8 = GrowWhy::GroundTooHard as u8;
    pub const NO_DIRECTION: u8 = GrowWhy::NoGoodDirection as u8;
}

fn steer_for(world: &World, card: CardKind, tag: u32, at: (i32, i32), heading: (f32, f32), toward_rival: f32) -> Steer {
    let base = Steer { tag, cells_left: card.cells(), aim: (0.0, -1.0), aim_weight: 1.0, keep_weight: 0.3, leaf_every: false, fork: false, stalls: 0, why: 0 };
    match card {
        CardKind::GrowUp => base,
        CardKind::Lean => {
            let side = brighter_side(world, at, toward_rival);
            Steer { aim: unit((0.85 * side, -0.53)), ..base }
        }
        CardKind::Fork => Steer { aim: heading, aim_weight: 0.5, keep_weight: 1.0, fork: true, ..base },
        CardKind::LeafOut => Steer { aim: heading, aim_weight: 0.6, keep_weight: 1.0, leaf_every: true, ..base },
        CardKind::Bud => Steer { cells_left: 0, ..base },
    }
}

/// -1 or +1: which side of `at` is brighter, by the light leaves there would
/// get, a dozen cells out and a few up. Ties go toward the rival, because
/// shading it is the point.
pub fn brighter_side(world: &World, (x, y): (i32, i32), toward_rival: f32) -> f32 {
    // The game's low sun, not the engine's overhead light: Lean goes out of
    // shadow, toward the side the sun actually reaches.
    let sample = |dx: i32| -> f32 { (0..4).map(|k| sun_at(world, (x + dx, y - 2 - k * 2))).sum() };
    let (l, r) = (sample(-12), sample(12));
    if (l - r).abs() < 0.05 {
        toward_rival
    } else if r > l {
        1.0
    } else {
        -1.0
    }
}

fn unit((x, y): (f32, f32)) -> (f32, f32) {
    let n = (x * x + y * y).sqrt();
    if n > f32::EPSILON {
        (x / n, y / n)
    } else {
        (0.0, -1.0)
    }
}

/// The stem cell a tip at `at` is standing on: its own-organism neighbour
/// furthest down.
fn stem_below(world: &World, me: OrganismId, (x, y): (i32, i32)) -> (i32, i32) {
    let mut best = (x, y);
    for (dx, dy) in [(0, 1), (-1, 1), (1, 1), (-1, 0), (1, 0)] {
        let (nx, ny) = (x + dx, y + dy);
        let c = world.get(nx, ny);
        if c.organism_id() == me && organism::cell_type(c.aux()) != Some(CellType::Leaf) {
            best = (nx, ny);
            break;
        }
    }
    best
}

/// Sum of the light on one tree's leaf cells, in noon-equivalent units --
/// the light they photosynthesise on.
pub fn lit_leaf_light(world: &World, id: OrganismId) -> f32 {
    let Some(state) = world.organism(id) else { return 0.0 };
    let mut sum = 0.0;
    for &(x, y) in state.cells.keys() {
        let c = world.get(x, y);
        if c.organism_id() == id && organism::cell_type(c.aux()) == Some(CellType::Leaf) {
            sum += crate::sim::plant::ambient_light_above(world, x, y) / crate::sim::field::MAX_LIGHT;
        }
    }
    sum
}

/// **The low sun** on one tree: the sum over its leaves of the light that
/// reaches each, averaged over the two `SUNS` (1 in open sky, `LEAF_PASSES`
/// for each leaf in the way, nothing through wood or ground), the leaves left
/// under 0.3, and its wood above ground.
///
/// **Game-side, and only for the score and the purse.** The engine's own
/// light falls straight down a column (`plant::ambient_light_above`), so on
/// it two crowns standing side by side could not shade each other and
/// "shade the rival out" was not something a player could do (playtest
/// 2026-11-08). The trees still grow by the engine's light; what you earn
/// and score by is this.
pub fn sun_on(world: &World, id: OrganismId) -> (f32, Vec<(i32, i32)>, usize) {
    let Some(state) = world.organism(id) else { return (0.0, Vec::new(), 0) };
    let mut sum = 0.0;
    let mut dark = Vec::new();
    let mut wood = 0;
    for &(x, y) in state.cells.keys() {
        if y >= GROUND_Y {
            continue;
        }
        let c = world.get(x, y);
        if c.organism_id() != id {
            continue;
        }
        if organism::cell_type(c.aux()) != Some(CellType::Leaf) {
            wood += 1;
            continue;
        }
        let light = sun_at(world, (x, y));
        if light < 0.3 {
            dark.push((x, y));
        }
        sum += light;
    }
    (sum, dark, wood)
}

/// The low suns reaching cell `(x, y)` (`SUNS`, averaged): 1 in open sky,
/// `LEAF_PASSES` for each leaf in the way, nothing through anything else.
/// The cell itself is not counted.
pub fn sun_at(world: &World, at: (i32, i32)) -> f32 {
    SUNS.iter().map(|&from| sun_from(world, at, from)).sum::<f32>() / SUNS.len() as f32
}

/// One sun's light reaching `(x, y)` along `from`.
fn sun_from(world: &World, (x, y): (i32, i32), (sx, sy): (i32, i32)) -> f32 {
    // March toward the sun one run-step at a time, sampling the cells a line
    // of slope sy/sx passes through.
    let mut light = 1.0f32;
    let steps = sx.abs().max(sy.abs());
    let (mut fx, mut fy) = (x as f32 + 0.5, y as f32 + 0.5);
    let (dx, dy) = (sx as f32 / steps as f32, sy as f32 / steps as f32);
    let mut last = (x, y);
    loop {
        fx += dx;
        fy += dy;
        let p = (fx.floor() as i32, fy.floor() as i32);
        if p.1 < 0 || p.0 < 0 || p.0 >= W {
            break;
        }
        if p == last {
            continue;
        }
        last = p;
        let o = world.get(p.0, p.1);
        if o.is_empty() {
            continue;
        }
        if o.organism_id() != 0 && organism::cell_type(o.aux()) == Some(CellType::Leaf) {
            light *= LEAF_PASSES;
        } else {
            return 0.0;
        }
        if light < 0.02 {
            return 0.0;
        }
    }
    light
}

/// Leaf cells on one tree, lit or not.
pub fn leaf_cells(world: &World, id: OrganismId) -> usize {
    let Some(state) = world.organism(id) else { return 0 };
    state
        .cells
        .keys()
        .filter(|&&(x, y)| {
            let c = world.get(x, y);
            c.organism_id() == id && organism::cell_type(c.aux()) == Some(CellType::Leaf)
        })
        .count()
}

/// Soil over stone over bedrock, flat, at field capacity -- except the
/// rival's side, where a rock shelf comes up to within [`SHELF_SOIL`] rows of
/// the surface. The design's thin-soil shelf, and the rival's handicap: a
/// tree on shallow ground roots less, drinks less and grows smaller, by the
/// engine's own rules rather than by a number on the rival.
fn build_valley(w: &mut World) {
    let soil = w.materials.id_of("soil").expect("soil is compiled in");
    for x in 0..W {
        let depth = if x >= SHELF_FROM { SHELF_SOIL } else { SOIL_DEPTH };
        for y in (GROUND_Y + depth)..(GROUND_Y + SOIL_DEPTH) {
            w.set(x, y, Cell::new(material::STONE, 0));
        }
        for y in GROUND_Y..(GROUND_Y + depth) {
            w.set(x, y, Cell::new(soil, (crate::sim::rng::jitter(x, y) * 255.0) as u8).with_aux(material::SOIL_FIELD_CAPACITY));
        }
        for y in (GROUND_Y + SOIL_DEPTH)..(GROUND_Y + SOIL_DEPTH + 4) {
            w.set(x, y, Cell::new(material::STONE, 0));
        }
        for y in (GROUND_Y + SOIL_DEPTH + 4)..H {
            w.set(x, y, Cell::new(material::BEDROCK, 0));
        }
    }
}

/// Plant a `tree` seed on the surface at column `x`; its organism id.
fn plant(w: &mut World, x: i32) -> OrganismId {
    let y = GROUND_Y - 1;
    let ok = w.plant_tree_species(x, y, "tree");
    assert!(ok, "could not plant at ({x}, {y}): cell {:?} species {:?}", w.get(x, y), w.species.id_of("tree"));
    w.get(x, y).organism_id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tree_with_no_cards_does_not_grow_its_shoot() {
        let mut g = Heartwood::new(1);
        for _ in 0..1500 {
            g.tick();
        }
        let shoot = g.world.organism(g.me).map_or(0, |s| s.shoot_cells);
        let rival = g.world.organism(g.rival).map_or(0, |s| s.shoot_cells);
        assert!(shoot <= 3, "an uncarded tree grew {shoot} shoot cells");
        assert!(rival > 50, "the rival should grow on its own: {rival}");
    }

    #[test]
    fn a_grow_up_card_grows_the_tip_upward() {
        let mut g = Heartwood::new(1);
        for _ in 0..60 {
            g.tick();
        }
        let tag = *g.selected.iter().next().expect("a tip is selected once the seed is up");
        let (_, y0) = g.tip_at(tag).unwrap();
        g.hand[0] = CardKind::GrowUp;
        assert!(g.play(0));
        for _ in 0..200 {
            g.tick();
        }
        let (_, y1) = g.tip_at(tag).expect("the tip is still there");
        assert!(y0 - y1 >= 8, "Grow Up moved the tip {} rows", y0 - y1);
    }

    /// The red outline is honest: a card the purse cannot pay for is refused
    /// with its reason before anything is charged, and one that can is
    /// charged once, for one tip.
    #[test]
    fn a_card_is_refused_before_paying_and_charged_once() {
        let mut g = Heartwood::new(1);
        for _ in 0..60 {
            g.tick();
        }
        let tag = *g.selected.iter().next().expect("a tip is selected once the seed is up");
        g.hand[0] = CardKind::Fork;
        g.sugar = CardKind::Fork.cost() - 1.0;
        assert!(matches!(g.verdict(CardKind::Fork, tag), Verdict::No("NOT ENOUGH SUGAR")));
        let before = g.sugar;
        assert!(!g.play(0), "a card the purse cannot pay for was played");
        assert_eq!(g.sugar, before, "a refused card was charged");
        assert_eq!(g.tally.refused, 1);
        g.sugar = 100.0;
        assert!(g.play(0));
        assert_eq!(g.sugar, 100.0 - CardKind::Fork.cost(), "one card is charged once");
    }
}
