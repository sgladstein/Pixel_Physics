//! **Founding a colony — the one act in this game you do not get back.**
//!
//! `C` used to drop twelve stock ants and print how many landed. That is a
//! keypress, not an act: nothing about the colony was yours, every founding
//! was the same founding, and the brief asked for the opposite — *"part
//! random, part user design, some user input"*, with *"each attempt rare and
//! costly"*.
//!
//! So `C` opens an offer. **Three dials, and the split between what you
//! choose and what the world rolls is the owner's**, 2026-09-14: the first
//! version rolled the body too, and the verdict was *"more flexibility,
//! especially on body shape and movement."*
//!
//! - **The body you choose** (`Q`/`E`). Six stocks, and the choice is not
//!   cosmetic: it settles how the animal *moves* — the hopper is the only
//!   shipped consumer of `BrainOutput::Impulse` and actually jumps, the
//!   segmented body bends where a rigid one cannot, a two-cell body dies
//!   whole where a six-cell one loses a tail and lives.
//! - **The lineage the world rolls** (`A`/`D`, three on offer). Traits only,
//!   including two that are movement — how fast it lives and how tightly it
//!   turns.
//! - **How many founders** (`Z`/`V`).
//!
//! Then you spend the pool to put them in the ground. **Walking away costs nothing and leaves the same three
//! standing** — it is committing that rerolls them, which is the whole of what
//! makes the choice a choice. A free reroll would turn "rare and costly" into
//! a slot machine you play until you win.
//!
//! **Everything here is a number the engine already reads**, which is why this
//! module is arithmetic and no engine change went with it. The stock is
//! `found_colony_of`'s species parameter; the roll is `CreatureDef::traits`
//! with deltas on six slots, handed to `creature::release_creature_specimen`
//! as an `Origin::Stock`, which stamps them verbatim and adds the colony's
//! own scent offset on top.
//!
//! **The brain genome is passed through untouched, deliberately.**
//! `organism.rs` keeps traits outside the genome on purpose — *"a gut is not a
//! synapse"* — and `ant.ron`'s hidden layer is where the whole homing circuit
//! lives (`PheroAAlong`/`PheroBAlong` into `Move` at ±6.0, gated on
//! `Carrying`). Rolling the wiring would hand the player colonies that cannot
//! forage, which is a worse game *and* a bug that takes an evening to
//! recognise, because a colony that random-walks looks exactly like a colony
//! that is merely unlucky.
//!
//! **Every number below is a first guess.** Nothing here has been played with
//! the economy on; they are set so that a default founding is affordable
//! twice and a heavy one hurts, and that is all the authority they have.

use crate::sim::organism::{TRAIT_ARMOUR, TRAIT_CROP_CAPACITY, TRAIT_CURVATURE_RADIUS, TRAIT_DIG_FORCE, TRAIT_GUT_BIAS, TRAIT_PACE, TRAIT_SIGHT_RANGE, CREATURE_TRAITS};
use crate::sim::rng::Rng;

/// How many lineages are on offer at once.
pub const OFFERED: usize = 3;

/// Founder-count bounds, and where the dial starts.
pub const FOUNDERS_MIN: i32 = 4;
pub const FOUNDERS_MAX: i32 = 24;
pub const FOUNDERS_DEFAULT: i32 = 12;

/// Pool spent per founder before the stock and the roll are priced in.
///
/// Set against `POWER_START` (600) so a default founding — the common ant,
/// twelve of them, a neutral roll — costs 144, a shade under a quarter of a
/// full pool. Heavy stock at a strong roll runs past 500 and is meant to: at
/// that price the founder dial is the thing you reach for, which is the
/// decision this screen exists to offer.
const PER_FOUNDER: f32 = 12.0;

/// **A stock the offer can draw from.**
///
/// Six of the eleven species with a `creature` block, and the five that are
/// missing were each excluded for a measured reason rather than by taste:
/// `ancestor` and `flitter` declare no nest, so `found_colony_of` places
/// animals with no home gradient and nobody forages; `beetle` carries **zero**
/// pheromone wires and no `Attack` — it run-and-tumbles at random, which
/// `dead-ends.md` already records as the thing that made beetles useless as
/// predation pressure; and `ant_block`/`ant_block_shaded` are render fixtures.
/// `a_stock_is_a_species_that_can_actually_keep_house` holds that line, with
/// `beetle` as its positive control.
pub struct Stock {
    /// The species name, as `found_colony_of` and the registry spell it.
    pub species: &'static str,
    /// What it is called on screen.
    pub name: &'static str,
    /// One line on what you are choosing, in the world's words rather than
    /// the file's.
    pub blurb: &'static str,
    /// Cells in the body plan. Prices the stock, and it is the honest
    /// quantity to price on: a nine-cell animal is nine cells of the world
    /// spent, and takes a bite in the middle without dying — which a two-cell
    /// body cannot do at all.
    ///
    /// **"digs a wider corridor" stood here and was false.** Corrected
    /// 2026-09-14 against `Reports/creature-articulated-body-2026-09-09.md`,
    /// which measured the *opposite sign*: a one-cell width increase took
    /// roofed void from `4, 6, 4` to **0 at every window from frame 1,000
    /// on**. The mechanism is two facts that only look contradictory: the dig
    /// verb is **body-blind** — `act` targets one cell ahead of the head and
    /// never reads `def.body` — while the *step* is body-aware and its
    /// refusal is terminal, since an ant that cannot fit through the hole it
    /// just cut re-rolls its heading (`tumble`) and never faces that soil
    /// again. So a wider body does not widen a gallery; it stops one being
    /// dug. `colony_stations`' own doc says the same thing from the surface
    /// side — wide bodies get *spread out* because they **gridlock**.
    pub cells: i32,
}

pub const STOCKS: &[Stock] = &[
    Stock { species: "ant", name: "COMMON ANT", blurb: "TWO CELLS. CHEAP, QUICK, DIES WHOLE.", cells: 2 },
    Stock { species: "hopper", name: "HOPPER", blurb: "IT JUMPS. GETS WHERE WALKING DOES NOT.", cells: 3 },
    Stock { species: "ant_long", name: "LONG ANT", blurb: "SIX CELLS. LOSES A TAIL AND LIVES.", cells: 6 },
    Stock { species: "longant", name: "SEGMENTED ANT", blurb: "SEVEN CELLS, JOINTED. BENDS ROUND CORNERS.", cells: 7 },
    Stock { species: "ant_wide", name: "BROAD ANT", blurb: "FIVE ACROSS, TWO DEEP. JAMS IN A NARROW GAP.", cells: 9 },
    Stock { species: "chitin_pale", name: "PALE CHITIN", blurb: "PLATED FROM BIRTH. TWICE AN ANT TO BITE THROUGH.", cells: 9 },
];

/// **What a founding rolls**, and the words each end of it gets.
///
/// Six slots of `CREATURE_TRAITS`, chosen because each is legible in one
/// phrase and each is genuinely read by the engine — a line here that no
/// consumer reads would be `dead-ends.md`'s channel-with-no-reader wearing a
/// player-facing label, which is the worst version of it, because the player
/// would be choosing on it.
///
/// The four words are the bands, outward from neutral. Neutral itself gets no
/// line: a lineage that is unremarkable in a slot should read as unremarkable
/// rather than as a weak version of something.
struct Rolled {
    slot: usize,
    /// Strong low, mild low, mild high, strong high.
    words: [&'static str; 4],
    /// What one unit of the *high* direction adds to the price. Zero where
    /// neither end is an advantage.
    premium: f32,
}

const ROLLED: &[Rolled] = &[
    // Diet. Neither end is better — a grazer and a scavenger are different
    // colonies, not a worse and a better one — so it is priced at zero.
    Rolled { slot: TRAIT_GUT_BIAS, words: ["EATS ONLY GREEN", "PREFERS GREEN", "PREFERS CARRION", "EATS ONLY CARRION"], premium: 0.0 },
    // How fast it lives. Fast is more work done and more food burnt; it is
    // priced, but lightly, because it is a real trade rather than a gift.
    Rolled { slot: TRAIT_PACE, words: ["SLOW-LIVING", "UNHURRIED", "BRISK", "BURNS FAST"], premium: 0.15 },
    // How tightly it turns — the second movement trait, and neither end is
    // better: a tight turner works a small patch over and a wide one covers
    // ground, so it is priced at zero like diet.
    Rolled { slot: TRAIT_CURVATURE_RADIUS, words: ["TURNS ON THE SPOT", "TURNS TIGHT", "TURNS WIDE", "RANGES WIDE"], premium: 0.0 },
    Rolled { slot: TRAIT_DIG_FORCE, words: ["SOFT JAWS", "WEAK JAWS", "STRONG JAWS", "SHEARING JAWS"], premium: 0.30 },
    Rolled { slot: TRAIT_ARMOUR, words: ["THIN SHELLED", "LIGHTLY PLATED", "WELL PLATED", "ARMOURED"], premium: 0.30 },
    Rolled { slot: TRAIT_CROP_CAPACITY, words: ["SHALLOW CROP", "SMALL CROP", "DEEP CROP", "GREAT CROP"], premium: 0.25 },
    // The dearest, and the rarest thing on the list. Every shipped ant is
    // blind; an eye costs about 4% of a life and buys the only sense the
    // colony has past the cells a head touches.
    Rolled { slot: TRAIT_SIGHT_RANGE, words: ["BLIND", "DIM SIGHTED", "KEEN EYED", "FAR SIGHTED"], premium: 0.50 },
];

/// The band a rolled value falls in: `None` is neutral and draws no line.
///
/// Bands rather than a printed number, because the number is a
/// `CREATURE_TRAITS` coordinate and means nothing to anyone holding the
/// keyboard.
fn band(v: f32) -> Option<usize> {
    match v {
        v if v < -0.45 => Some(0),
        v if v < -0.15 => Some(1),
        v if v <= 0.15 => None,
        v if v <= 0.45 => Some(2),
        _ => Some(3),
    }
}

/// One line of a lineage's character, ready to draw.
pub struct Line {
    pub word: &'static str,
    /// How far from neutral, 0..1 — the screen leans on this for colour, so
    /// a strong roll is visibly a strong roll before anything is read.
    pub strength: f32,
}

/// **A lineage on offer**: how this bloodline differs from its stock.
///
/// **It carries no body.** The body is the player's dial, not the roll's —
/// owner's ruling, and it means the same three lineages can be put into any
/// of the six stocks, which is what "more flexibility on body shape" asks
/// for. It also keeps this type pure — no `World`, no species registry — so
/// the screen is testable as arithmetic.
///
/// Deltas rather than absolutes, and that is the honest form: the stock's own
/// character is in its blurb, and what the roll says is *how this lineage
/// departs from whatever body you put it in*.
#[derive(Clone)]
pub struct Candidate {
    pub deltas: [f32; CREATURE_TRAITS],
}

impl Candidate {

    /// **Roll one lineage**, deterministically.
    ///
    /// Keyed on the world seed, which attempt this is, and which of the three
    /// it is, so the same world offers the same foundings in the same order —
    /// determinism is required here (`PLAN.md`, same-build replay), and an
    /// offer that reshuffled on a redraw would also be unreadable.
    ///
    /// **The draw is triangular, not uniform, and that is the ethos rather
    /// than a flourish.** *An outcome is a distribution, not a binary*: two
    /// uniforms summed put most slots near neutral and make an extreme
    /// genuinely uncommon, so a lineage usually has one or two things to say
    /// about itself and occasionally has something worth paying for. A
    /// uniform draw gives every candidate six loud traits and none of them
    /// mean anything.
    pub fn roll(seed: u64, attempt: u32, index: usize) -> Self {
        // A separate stream per candidate: one shared generator would couple
        // each candidate to how many draws the ones before it happened to
        // take, so adding a seventh rolled slot would silently reshuffle
        // candidate 2 as well as candidate 0. `rng.rs`'s own doc names this.
        let mut rng = Rng::new(
            seed ^ (attempt as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (index as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9),
        );
        let mut deltas = [0.0f32; CREATURE_TRAITS];
        for r in ROLLED {
            deltas[r.slot] = (rng.unit_f32() + rng.unit_f32() - 1.0).clamp(-1.0, 1.0);
        }
        Candidate { deltas }
    }

    /// The lines this lineage draws, neutral slots omitted.
    pub fn lines(&self) -> Vec<Line> {
        ROLLED
            .iter()
            .filter_map(|r| {
                let v = self.deltas[r.slot];
                band(v).map(|b| Line { word: r.words[b], strength: v.abs().min(1.0) })
            })
            .collect()
    }

    /// **What this founding costs**, in pool, for `founders` of them.
    ///
    /// Three terms, and each is something the player can see on the screen
    /// that charges for it: how many, how much animal each one is, and how
    /// much the roll gave you. A cost with a term nobody can point at is a
    /// number that reads as arbitrary however carefully it was derived.
    pub fn cost(&self, stock: usize, founders: i32) -> f32 {
        let body = 1.0 + (STOCKS[stock.min(STOCKS.len() - 1)].cells - 2).max(0) as f32 * 0.25;
        // Only the *high* side is charged for, and only where a high side is
        // an advantage. A thin-shelled, blind, slow lineage is not a discount
        // you can farm — it is simply cheap, which it should be.
        let premium: f32 = ROLLED.iter().map(|r| self.deltas[r.slot].max(0.0) * r.premium).sum();
        founders.max(0) as f32 * PER_FOUNDER * body * (1.0 + premium)
    }
}

/// **One line of the screen the cursor can sit on.**
///
/// The founding menu is a vertical list, and that is the whole of what makes
/// it an arrow-key menu. Owner playtest, 2026-09-14: *"the found menu needs
/// to be way improved. it should be fully controlled by arrow keys and/or
/// wasd and/or mouse."* The bindings it had were six **letters**, one per
/// dial and none of them related — `A`/`D` a lineage, `Q`/`E` a body,
/// `Z`/`V` a count — which is a shortcut list wearing a panel, and is the
/// owner's own words for it elsewhere in the same playtest: *"the menu isn't
/// even a menu, it is a shortcut list."*
///
/// A list has an up and a down, so a cursor is all it takes for every device
/// to reach every dial: arrows and `WASD` move it, left and right work the
/// dial it is on, the mouse names a row directly. Nothing here is a second
/// copy of the key handler's opinion — [`Offer::adjust`] and
/// [`Offer::activate`] are the only two verbs, and the keyboard and the
/// mouse both call them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Row {
    /// The stock, which settles how the animal moves.
    Body,
    /// One of the three lineages on offer. Landing on it picks it — a cursor
    /// that sat on a row without choosing it would be two selections on one
    /// list.
    Lineage(usize),
    /// How many founders to spend on.
    Founders,
    /// Commit. A row rather than only a letter, because a menu you can drive
    /// with the arrows has to have somewhere for the arrows to *arrive*.
    Found,
    /// Walk away. Costs nothing and leaves the same three standing.
    Leave,
}

/// Every row, in the order the screen draws them. The cursor is an index
/// into this, so the list and the order are stated once.
pub const ROWS: &[Row] = &[Row::Body, Row::Lineage(0), Row::Lineage(1), Row::Lineage(2), Row::Founders, Row::Found, Row::Leave];

/// What [`Offer::activate`] asks the caller to do — the two things this
/// module cannot do for itself, because they need the world.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Activate {
    Commit,
    Close,
    /// Handled here; the caller has nothing to do.
    Done,
}

/// **What the player set last time, kept across closes.**
///
/// Owner playtest, 2026-09-14: *"When I change things they should stay as the
/// default next time I open the menu."* They did not, and the module doc
/// above claimed they did — `toggle_founding` built a fresh [`Offer`] on
/// every open, so the body, the count and the pick all snapped back. **The
/// same line lost the reroll**: `commit_founding` called [`Offer::reroll`]
/// and *then* dropped the offer, so the fresh three it drew went out with it
/// and the next open served attempt 0 again. "Committing is what costs you
/// the other two" was the design and had never once happened in the game.
/// Both are one bug — the screen's state had nowhere to live between opens —
/// and this is that somewhere.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Memory {
    pub body: usize,
    pub founders: i32,
    pub picked: usize,
    /// Which row the cursor was on. Kept for the same reason as the rest: a
    /// menu that reopens with the cursor somewhere you did not leave it is a
    /// menu you have to re-read.
    pub row: usize,
    /// Which draw is on the table. Bumped by a commit and by nothing else.
    pub attempt: u32,
}

impl Default for Memory {
    fn default() -> Self {
        Memory { body: 0, founders: FOUNDERS_DEFAULT, picked: 0, row: 0, attempt: 0 }
    }
}

/// **The screen's state**: what is on offer, what is picked, how many.
pub struct Offer {
    attempt: u32,
    seed: u64,
    candidates: Vec<Candidate>,
    pub picked: usize,
    /// **The body, and it is chosen rather than rolled.** Index into
    /// [`STOCKS`]; see the module doc for the owner's ruling behind the
    /// split.
    pub body: usize,
    pub founders: i32,
    /// Where the cursor is, as an index into [`ROWS`].
    pub row: usize,
    /// The row the pointer is over, or `None` when it is elsewhere. Held here
    /// rather than on the window handler so that it rides the interface's own
    /// frame-to-frame comparison: this game skips the repaint of a settled
    /// world, and a highlight that moved without the comparison noticing
    /// would simply not be drawn.
    pub hover: Option<Row>,
}

impl Offer {
    pub fn new(seed: u64) -> Self {
        Self::resumed(seed, Memory::default())
    }

    /// **Open the screen on what the player left it at.** See [`Memory`] for
    /// the playtest this exists for.
    ///
    /// Every field is clamped on the way in rather than trusted: the stock
    /// list and the founder bounds are both things a later change can
    /// shorten, and a remembered index past the end of a shortened list is a
    /// panic on the one screen the player cannot avoid.
    pub fn resumed(seed: u64, memory: Memory) -> Self {
        let mut offer = Offer {
            attempt: memory.attempt,
            seed,
            candidates: Vec::new(),
            picked: memory.picked.min(OFFERED - 1),
            body: memory.body.min(STOCKS.len() - 1),
            founders: memory.founders.clamp(FOUNDERS_MIN, FOUNDERS_MAX),
            row: memory.row.min(ROWS.len() - 1),
            hover: None,
        };
        offer.draw_candidates();
        offer
    }

    /// What to hand [`Offer::resumed`] next time.
    pub fn memory(&self) -> Memory {
        Memory { body: self.body, founders: self.founders, picked: self.picked, row: self.row, attempt: self.attempt }
    }

    /// Roll the three on the table for the current attempt. **Does not bump
    /// the attempt**, so an offer rebuilt from its own memory is the same
    /// offer rather than the next one.
    fn draw_candidates(&mut self) {
        self.candidates = (0..OFFERED).map(|i| Candidate::roll(self.seed, self.attempt, i)).collect();
    }

    /// Draw a fresh three. Called on commit and never on a refusal, so a
    /// founding you could not afford leaves the offer exactly as it was.
    pub fn reroll(&mut self) {
        self.attempt = self.attempt.wrapping_add(1);
        self.draw_candidates();
        self.picked = 0;
        // ...and the cursor comes back to the first lineage if it was on one
        // of the others, so the highlight and the pick cannot disagree.
        if matches!(ROWS.get(self.row), Some(Row::Lineage(_))) {
            self.row = 1;
        }
        // The body is not rerolled: it is the player's standing choice, and
        // resetting it every founding would make the dial feel like it had
        // not been set.
    }

    /// The row the cursor is on.
    pub fn row(&self) -> Row {
        ROWS[self.row.min(ROWS.len() - 1)]
    }

    /// Move the cursor, wrapping — same reason [`Offer::step_pick`] wraps.
    pub fn step_row(&mut self, delta: i32) {
        let n = ROWS.len() as i32;
        self.row = (((self.row as i32 + delta) % n + n) % n) as usize;
        self.sync_pick();
    }

    /// Put the cursor on a named row. What a click does, and what a hover
    /// does not.
    pub fn go_to(&mut self, row: Row) {
        if let Some(i) = ROWS.iter().position(|r| *r == row) {
            self.row = i;
            self.sync_pick();
        }
    }

    /// **Landing on a lineage row picks it.** The alternative is a cursor and
    /// a selection that can point at different rows, which is two highlights
    /// on one list and reads as the keys not working.
    fn sync_pick(&mut self) {
        if let Row::Lineage(i) = self.row() {
            self.picked = i.min(self.candidates.len().saturating_sub(1));
        }
    }

    /// **Work the dial the cursor is on.** Left and right, from any device.
    pub fn adjust(&mut self, delta: i32) {
        match self.row() {
            Row::Body => self.step_body(delta),
            // On a lineage row the dial *is* the list, so left and right walk
            // it rather than doing nothing — a direction that is dead on four
            // of seven rows reads as the menu being half-wired.
            Row::Lineage(_) => {
                self.step_pick(delta);
                self.row = 1 + self.picked;
            }
            Row::Founders => self.step_founders(delta),
            Row::Found | Row::Leave => {}
        }
    }

    /// **Choose the row the cursor is on**, which is what `ENTER` and a click
    /// both mean. The two outcomes the screen cannot carry out itself come
    /// back as [`Activate`].
    pub fn activate(&mut self) -> Activate {
        match self.row() {
            Row::Found => Activate::Commit,
            Row::Leave => Activate::Close,
            // On a dial, choosing is stepping it forward — the same thing
            // `SPACE` means on the options menu, so the two screens do not
            // disagree about what the confirm key does.
            _ => {
                self.adjust(1);
                Activate::Done
            }
        }
    }

    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    pub fn picked(&self) -> &Candidate {
        &self.candidates[self.picked.min(self.candidates.len() - 1)]
    }

    /// Move the selection, wrapping — three items on one row, and a selection
    /// that stops dead at the end is a worse keyboard than one that comes
    /// round.
    pub fn step_pick(&mut self, delta: i32) {
        let n = self.candidates.len() as i32;
        self.picked = (((self.picked as i32 + delta) % n + n) % n) as usize;
    }

    pub fn step_founders(&mut self, delta: i32) {
        self.founders = (self.founders + delta).clamp(FOUNDERS_MIN, FOUNDERS_MAX);
    }

    /// Cycle the body, wrapping — same reason as `step_pick`.
    pub fn step_body(&mut self, delta: i32) {
        let n = STOCKS.len() as i32;
        self.body = (((self.body as i32 + delta) % n + n) % n) as usize;
    }

    pub fn stock(&self) -> &'static Stock {
        &STOCKS[self.body.min(STOCKS.len() - 1)]
    }

    pub fn cost(&self) -> f32 {
        self.picked().cost(self.body, self.founders)
    }

    /// What the chosen body costs each of the three lineages, for the screen.
    pub fn cost_of(&self, index: usize) -> f32 {
        self.candidates[index.min(self.candidates.len() - 1)].cost(self.body, self.founders)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Same world, same offer. Required rather than nice: `PLAN.md` asks for
    /// same-build deterministic replay, and an offer that reshuffled between
    /// two draws of the same screen would be unreadable besides.
    #[test]
    fn the_same_world_offers_the_same_foundings() {
        let a = Offer::new(4242);
        let b = Offer::new(4242);
        for (x, y) in a.candidates().iter().zip(b.candidates()) {
            assert_eq!(x.deltas, y.deltas);
        }
        // ...and a different world offers something else, or the seed is not
        // reaching the draw at all -- which is exactly what the sweep gotcha
        // warns reads as a clean result.
        let other = Offer::new(4243);
        let same = a.candidates().iter().zip(other.candidates()).all(|(x, y)| x.deltas == y.deltas);
        assert!(!same, "two different seeds produced the identical offer; the seed is not connected");
    }

    /// Committing is what costs you the other two. If a reroll returned the
    /// same three, walking away and committing would be the same move.
    #[test]
    fn committing_draws_a_fresh_three() {
        let mut offer = Offer::new(7);
        let before: Vec<_> = offer.candidates().iter().map(|c| c.deltas).collect();
        offer.reroll();
        let after: Vec<_> = offer.candidates().iter().map(|c| c.deltas).collect();
        assert_ne!(before, after);
    }

    #[test]
    fn a_founding_costs_more_for_more_founders_and_for_heavier_stock() {
        let neutral = Candidate { deltas: [0.0; CREATURE_TRAITS] };
        let (light, heavy) = (0, STOCKS.len() - 1);
        assert!(neutral.cost(light, 24) > neutral.cost(light, 4));
        assert_eq!(neutral.cost(light, 0), 0.0);
        // The common ant is two cells; pale chitin is nine -- and the same
        // lineage in the heavier body costs more, which is the body dial
        // doing something.
        assert!(STOCKS[heavy].cells > STOCKS[light].cells);
        assert!(neutral.cost(heavy, 12) > neutral.cost(light, 12));
        // And a strong roll is dearer than a neutral one, in the same body.
        let mut gifted = neutral.clone();
        gifted.deltas[TRAIT_SIGHT_RANGE] = 1.0;
        assert!(gifted.cost(light, 12) > neutral.cost(light, 12));
        // ...while a *bad* roll is not a discount to farm.
        let mut poor = neutral.clone();
        poor.deltas[TRAIT_SIGHT_RANGE] = -1.0;
        assert_eq!(poor.cost(light, 12), neutral.cost(light, 12));
    }

    /// **The ethos as an assertion**: *an outcome is a distribution, not a
    /// binary*. A lineage should usually have one or two things to say about
    /// itself, so most slots must land neutral and an extreme must still
    /// happen.
    ///
    /// It fails in both directions, which is the point — a constant roll
    /// gives 0% strong, and a uniform one gives ~55%, and neither is a middle.
    #[test]
    fn a_roll_has_a_middle() {
        let mut neutral = 0;
        let mut strong = 0;
        let mut total = 0;
        for attempt in 0..200u32 {
            for i in 0..OFFERED {
                let c = Candidate::roll(99, attempt, i);
                for r in ROLLED {
                    total += 1;
                    match band(c.deltas[r.slot]) {
                        None => neutral += 1,
                        Some(0) | Some(3) => strong += 1,
                        _ => {}
                    }
                }
            }
        }
        let neutral_share = neutral as f32 / total as f32;
        let strong_share = strong as f32 / total as f32;
        // Printed, not just gated: the shares are quoted in the README and in
        // this change's commit message, and a number nobody can see the value
        // of is a number somebody eventually writes down from memory.
        // `cargo test --lib --release a_roll_has_a_middle -- --nocapture`.
        println!("roll over {total} slots: {:.1}% neutral, {:.1}% strong", neutral_share * 100.0, strong_share * 100.0);
        assert!((0.10..0.35).contains(&neutral_share), "neutral share {neutral_share:.3} -- the draw has lost its middle");
        assert!((0.10..0.40).contains(&strong_share), "strong share {strong_share:.3} -- extremes are either impossible or ordinary");
    }

    /// **What the player set is what the screen opens on.** Owner playtest,
    /// 2026-09-14: *"When I change things they should stay as the default
    /// next time I open the menu."*
    ///
    /// **The control is that the default does not already satisfy it**: an
    /// offer built from `Memory::default()` has body 0, the default founder
    /// count and pick 0, so a `resumed` that quietly ignored its argument
    /// would pass a guard written against a default-shaped memory. Every
    /// field here is set to something the default is not.
    #[test]
    fn the_screen_opens_on_what_was_left_set() {
        let mut offer = Offer::new(11);
        offer.body = 3;
        offer.founders = FOUNDERS_MAX;
        offer.step_row(1);
        offer.adjust(1);
        let memory = offer.memory();
        assert_ne!(memory, Memory::default(), "this guard is testing a memory the default already matches and would pass on a `resumed` that ignored it");

        let back = Offer::resumed(11, memory);
        assert_eq!(back.memory(), memory, "the screen came back on something other than what it was left on");
        // ...and the three on the table are the same three, because walking
        // away is not what costs you them.
        let before: Vec<_> = offer.candidates().iter().map(|c| c.deltas).collect();
        let after: Vec<_> = back.candidates().iter().map(|c| c.deltas).collect();
        assert_eq!(before, after, "closing the screen rerolled the offer -- walking away must cost nothing");
    }

    /// **Committing is what costs you the other two, and it has to survive
    /// the close.**
    ///
    /// It did not: `commit_founding` called `reroll` and then dropped the
    /// offer, so the fresh three went out with it and the next open served
    /// attempt 0 again. The design was in the module doc from the first day
    /// and had never once happened in the game.
    #[test]
    fn a_commit_is_still_a_commit_after_the_screen_closes() {
        let mut offer = Offer::new(11);
        let before: Vec<_> = offer.candidates().iter().map(|c| c.deltas).collect();
        offer.reroll();
        // The close: everything the game keeps, and nothing else.
        let reopened = Offer::resumed(11, offer.memory());
        let after: Vec<_> = reopened.candidates().iter().map(|c| c.deltas).collect();
        assert_ne!(before, after, "the reroll did not survive the close -- committing costs you nothing and the choice is not a choice");
    }

    /// **Every row is reachable and every row does something**, which is the
    /// whole of what "fully controlled by arrow keys" means.
    ///
    /// The shape of the check is `CLAUDE.md`'s *check that a planned step can
    /// demonstrate itself*: a cursor that moves over a row whose left and
    /// right do nothing is a menu that is half-wired, and it looks identical
    /// to one that works until you are on that row.
    #[test]
    fn every_row_of_the_menu_is_reachable_and_does_something() {
        let mut offer = Offer::new(5);
        // Reachable: `ROWS.len()` presses of down come back where they
        // started, having visited every row exactly once.
        let mut seen = Vec::new();
        for _ in 0..ROWS.len() {
            seen.push(offer.row());
            offer.step_row(1);
        }
        assert_eq!(seen, ROWS.to_vec(), "down did not walk the list in order");
        assert_eq!(offer.row(), ROWS[0], "the cursor must wrap -- a list that stops dead at the end is a worse keyboard");

        // Does something: each dial row's `adjust` moves its own dial and
        // nothing else's.
        let state = |o: &Offer| (o.body, o.founders, o.picked);
        offer.go_to(Row::Body);
        let was = state(&offer);
        offer.adjust(1);
        assert_ne!(offer.body, was.0, "left and right on BODY moved nothing");
        assert_eq!((offer.founders, offer.picked), (was.1, was.2), "BODY moved a dial that was not its own");

        offer.go_to(Row::Founders);
        let was = state(&offer);
        offer.adjust(-1);
        assert_ne!(offer.founders, was.1, "left and right on FOUNDERS moved nothing");
        assert_eq!((offer.body, offer.picked), (was.0, was.2), "FOUNDERS moved a dial that was not its own");

        // Landing on a lineage row picks it -- one highlight, not two.
        for i in 0..OFFERED {
            offer.go_to(Row::Lineage(i));
            assert_eq!(offer.picked, i, "the cursor sat on lineage {i} while lineage {} was picked", offer.picked);
        }
        // ...and left and right walk the list rather than being dead on it.
        offer.go_to(Row::Lineage(0));
        offer.adjust(1);
        assert_eq!(offer.row(), Row::Lineage(1), "left and right are dead on a lineage row");
        assert_eq!(offer.picked, 1, "the cursor and the pick parted company");

        // The two buttons are the only rows where choosing is not stepping.
        offer.go_to(Row::Found);
        assert_eq!(offer.activate(), Activate::Commit);
        offer.go_to(Row::Leave);
        assert_eq!(offer.activate(), Activate::Close);
        offer.go_to(Row::Body);
        assert_eq!(offer.activate(), Activate::Done, "ENTER on a dial must work it, the same as SPACE does on the options menu");
    }

    /// **A remembered index past the end of a shortened list must not
    /// panic.** [`STOCKS`] and [`FOUNDERS_MAX`] are both things a later
    /// change can make smaller, and the screen is the one the player cannot
    /// avoid.
    #[test]
    fn a_stale_memory_is_clamped_rather_than_trusted() {
        let wild = Memory { body: 999, founders: 9_999, picked: 999, row: 999, attempt: 3 };
        let offer = Offer::resumed(2, wild);
        assert!(offer.body < STOCKS.len());
        assert!(offer.picked < OFFERED);
        assert!(offer.row < ROWS.len());
        assert!((FOUNDERS_MIN..=FOUNDERS_MAX).contains(&offer.founders));
        // The dials still answer, which is the thing a clamp is for.
        assert!(offer.cost() > 0.0);
        let _ = offer.stock();
    }

    /// Every stock draws a word for every band, and every character of it has
    /// a glyph in the 5x7 font. The sibling of `hud`'s own glyph sweep, kept
    /// here because these strings never pass through `Readout`.
    #[test]
    fn every_word_the_offer_can_draw_has_a_glyph() {
        let mut checked = 0;
        for s in STOCKS {
            for text in [s.name, s.blurb] {
                for ch in text.chars() {
                    assert!(crate::hud::has_glyph(ch), "no glyph for {ch:?} in {text:?}");
                    checked += 1;
                }
            }
        }
        for r in ROLLED {
            for word in r.words {
                for ch in word.chars() {
                    assert!(crate::hud::has_glyph(ch), "no glyph for {ch:?} in {word:?}");
                    checked += 1;
                }
            }
        }
        assert!(checked > 200, "only {checked} characters swept; this guard would pass on nothing");
    }

    /// **Every stock can read the plane `G` arms, and this is the guard the
    /// original bug walked straight past.**
    ///
    /// `Druid::scent` ships at `Channel::default()`, which is `A` — the
    /// engine moved that default from `B` on 2026-09-09 because
    /// `open-bugs-handoff.md` §Z7 measured that **nothing can read B**. This
    /// game then named `B` explicitly, threw the default away, and shipped a
    /// verb that deposited a correct gradient no animal could act on. Two
    /// independent harnesses had already reported the exact tie (1,903 =
    /// 1,903 and 595 = 595) and a third — this game's own paired run —
    /// reproduced it before anyone connected the two.
    ///
    /// So the guard is not "is the default A"; that is a tautology over one
    /// line. It is **can the animals the player can actually found read it**:
    /// the repaired homing pair is `(Carrying, 0, 45.5)`, and the pre-fix
    /// weight it replaced was `75.0`, which saturated the unit and flattened
    /// the signal to about 0.3% of a step.
    ///
    /// **The positive control is the old number**, asserted absent: if
    /// `75.0` ever comes back on unit 0 of a stock, that stock is deaf again
    /// and this goes red.
    #[test]
    fn every_stock_can_read_the_plane_the_trail_arms() {
        // The armed plane is the engine's own opinion about which one is
        // readable, inherited rather than named -- that is the whole fix.
        assert_eq!(crate::sim::pheromone::Channel::default(), crate::sim::pheromone::Channel::A);
        let gate = regex_lite_carrying_zero;
        for s in STOCKS {
            let text = std::fs::read_to_string(format!("{}/{}.ron", crate::sim::organism::ASSET_DIR, s.species)).unwrap_or_default();
            let w = gate(&text).unwrap_or_else(|| panic!("{} authors no `(Carrying, 0, _)` homing gate at all", s.species));
            assert!(
                (w - 45.5).abs() < 0.01,
                "{} gates its homing unit at {w}, not the repaired 45.5 -- at 75.0 the unit saturates and the ant reads a laid channel-A trail at ~0.3% of a step (open-bugs-handoff.md SSZ7)",
                s.species
            );
        }
    }

    /// The weight on `(Carrying, 0, w)`, parsed out of a species file without
    /// pulling in a regex crate for one line.
    fn regex_lite_carrying_zero(text: &str) -> Option<f32> {
        text.lines().find_map(|l| {
            let l: String = l.chars().filter(|c| !c.is_whitespace()).collect();
            let rest = l.strip_prefix("(Carrying,0,")?;
            rest.split(')').next()?.parse().ok()
        })
    }

    /// **A stock has to be a species that can actually keep house**, and the
    /// three ways it can fail to be one have each already happened to
    /// somebody.
    ///
    /// `found_colony_of` returns 0 — silently, and a silent no-op is
    /// indistinguishable from a broken feature — when the species is not in
    /// the registry, or declares a nest whose material is missing. And a
    /// species with no nest at all gets animals on the ground with no home
    /// gradient, so nobody forages. The third is subtler and is the one that
    /// would be found last: a species can have a nest and still not wire the
    /// trail-following circuit, in which case the colony run-and-tumbles at
    /// random and looks merely unlucky for as long as you care to watch it.
    ///
    /// **`beetle` is the positive control** — a real species, with a nest,
    /// and zero pheromone wires. It is deliberately not in `STOCKS`, and this
    /// guard asserts it would be caught, so the check is known to
    /// discriminate rather than merely to pass.
    #[test]
    fn a_stock_is_a_species_that_can_actually_keep_house() {
        let forages = |name: &str| -> (bool, bool) {
            let text = std::fs::read_to_string(format!("{}/{name}.ron", crate::sim::organism::ASSET_DIR)).unwrap_or_default();
            let nest = text.lines().any(|l| l.trim_start().starts_with("nest:") && !l.contains("nest:\"\"") && !l.contains("nest: \"\""));
            let trail = text.contains("PheroAAlong") && text.contains("PheroBAlong");
            (nest, trail)
        };
        for s in STOCKS {
            let (nest, trail) = forages(s.species);
            assert!(nest, "{} declares no nest -- its colony would have no home gradient", s.species);
            assert!(trail, "{} does not wire the trail circuit -- its colony would walk at random", s.species);
        }
        // The control: the check has to be able to say no.
        let (nest, trail) = forages("beetle");
        assert!(nest, "beetle should still have a nest; this control is testing the wrong thing");
        assert!(!trail, "beetle now wires the trail circuit -- this control is blind and the guard above proves nothing");
    }
}
