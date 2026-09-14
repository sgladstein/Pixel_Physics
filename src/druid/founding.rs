//! **Founding a colony — the one act in this game you do not get back.**
//!
//! `C` used to drop twelve stock ants and print how many landed. That is a
//! keypress, not an act: nothing about the colony was yours, every founding
//! was the same founding, and the brief asked for the opposite — *"part
//! random, part user design, some user input"*, with *"each attempt rare and
//! costly"*.
//!
//! So `C` opens an offer. Three lineages are drawn from the world's own seed;
//! you pick one, pick how many founders to pay for, and spend the pool to put
//! them in the ground. **Walking away costs nothing and leaves the same three
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

use crate::sim::organism::{TRAIT_ARMOUR, TRAIT_CROP_CAPACITY, TRAIT_DIG_FORCE, TRAIT_GUT_BIAS, TRAIT_PACE, TRAIT_SIGHT_RANGE, CREATURE_TRAITS};
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
    /// spent, digs a wider corridor, and takes a bite in the middle without
    /// dying — which a two-cell body cannot do at all.
    pub cells: i32,
}

pub const STOCKS: &[Stock] = &[
    Stock { species: "ant", name: "COMMON ANT", blurb: "TWO CELLS. CHEAP, QUICK, DIES WHOLE.", cells: 2 },
    Stock { species: "hopper", name: "HOPPER", blurb: "IT JUMPS. GETS WHERE WALKING DOES NOT.", cells: 3 },
    Stock { species: "ant_long", name: "LONG ANT", blurb: "SIX CELLS. LOSES A TAIL AND LIVES.", cells: 6 },
    Stock { species: "longant", name: "SEGMENTED ANT", blurb: "SEVEN CELLS, JOINTED. BENDS ROUND CORNERS.", cells: 7 },
    Stock { species: "ant_wide", name: "BROAD ANT", blurb: "NINE CELLS. DIGS A ROOM YOU CAN STAND IN.", cells: 9 },
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

/// **A lineage on offer**: a stock, and how this one differs from it.
///
/// Deltas rather than absolutes, and that is the honest form: the stock's own
/// character is in its blurb, and what the roll says is *how this lineage
/// departs from its stock*. It also keeps this whole type pure — no `World`,
/// no species registry — so the screen is testable as arithmetic.
#[derive(Clone)]
pub struct Candidate {
    pub stock: usize,
    pub deltas: [f32; CREATURE_TRAITS],
}

impl Candidate {
    pub fn stock(&self) -> &'static Stock {
        &STOCKS[self.stock]
    }

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
        let stock = rng.below(STOCKS.len() as u32) as usize;
        let mut deltas = [0.0f32; CREATURE_TRAITS];
        for r in ROLLED {
            deltas[r.slot] = (rng.unit_f32() + rng.unit_f32() - 1.0).clamp(-1.0, 1.0);
        }
        Candidate { stock, deltas }
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
    pub fn cost(&self, founders: i32) -> f32 {
        let body = 1.0 + (self.stock().cells - 2).max(0) as f32 * 0.25;
        // Only the *high* side is charged for, and only where a high side is
        // an advantage. A thin-shelled, blind, slow lineage is not a discount
        // you can farm — it is simply cheap, which it should be.
        let premium: f32 = ROLLED.iter().map(|r| self.deltas[r.slot].max(0.0) * r.premium).sum();
        founders.max(0) as f32 * PER_FOUNDER * body * (1.0 + premium)
    }
}

/// **The screen's state**: what is on offer, what is picked, how many.
pub struct Offer {
    attempt: u32,
    seed: u64,
    candidates: Vec<Candidate>,
    pub picked: usize,
    pub founders: i32,
}

impl Offer {
    pub fn new(seed: u64) -> Self {
        let mut offer = Offer { attempt: 0, seed, candidates: Vec::new(), picked: 0, founders: FOUNDERS_DEFAULT };
        offer.reroll();
        offer
    }

    /// Draw a fresh three. Called on commit and never on a refusal, so a
    /// founding you could not afford leaves the offer exactly as it was.
    pub fn reroll(&mut self) {
        self.candidates = (0..OFFERED).map(|i| Candidate::roll(self.seed, self.attempt, i)).collect();
        self.attempt = self.attempt.wrapping_add(1);
        self.picked = 0;
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

    pub fn cost(&self) -> f32 {
        self.picked().cost(self.founders)
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
            assert_eq!(x.stock, y.stock);
            assert_eq!(x.deltas, y.deltas);
        }
        // ...and a different world offers something else, or the seed is not
        // reaching the draw at all -- which is exactly what the sweep gotcha
        // warns reads as a clean result.
        let other = Offer::new(4243);
        let same = a.candidates().iter().zip(other.candidates()).all(|(x, y)| x.stock == y.stock && x.deltas == y.deltas);
        assert!(!same, "two different seeds produced the identical offer; the seed is not connected");
    }

    /// Committing is what costs you the other two. If a reroll returned the
    /// same three, walking away and committing would be the same move.
    #[test]
    fn committing_draws_a_fresh_three() {
        let mut offer = Offer::new(7);
        let before: Vec<_> = offer.candidates().iter().map(|c| (c.stock, c.deltas)).collect();
        offer.reroll();
        let after: Vec<_> = offer.candidates().iter().map(|c| (c.stock, c.deltas)).collect();
        assert_ne!(before, after);
    }

    #[test]
    fn a_founding_costs_more_for_more_founders_and_for_heavier_stock() {
        let neutral = Candidate { stock: 0, deltas: [0.0; CREATURE_TRAITS] };
        assert!(neutral.cost(24) > neutral.cost(4));
        assert_eq!(neutral.cost(0), 0.0);
        // The common ant is two cells; pale chitin is nine.
        let heavy = Candidate { stock: STOCKS.len() - 1, deltas: [0.0; CREATURE_TRAITS] };
        assert!(heavy.stock().cells > neutral.stock().cells);
        assert!(heavy.cost(12) > neutral.cost(12));
        // And a strong roll is dearer than a neutral one, on the same stock.
        let mut gifted = neutral.clone();
        gifted.deltas[TRAIT_SIGHT_RANGE] = 1.0;
        assert!(gifted.cost(12) > neutral.cost(12));
        // ...while a *bad* roll is not a discount to farm.
        let mut poor = neutral.clone();
        poor.deltas[TRAIT_SIGHT_RANGE] = -1.0;
        assert_eq!(poor.cost(12), neutral.cost(12));
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
