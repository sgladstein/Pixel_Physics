//! **Assessment before commitment** — what an animal reads off an opponent
//! in the instant before it decides whether to bite it.
//!
//! # Why this exists
//!
//! The engine has had a working fight since 2026-09-06 and it has exactly
//! one shape: `BrainOutput::Attack` rolls, [`creature::nearest_foe`] returns
//! the first non-kin body cell in the ring, and a bite lands. The animal
//! commits with **no information about the thing it is committing to** — the
//! damage ratio `(bite / armour)²` is computed *after* the decision, in one
//! direction only (mine on theirs, never theirs on mine), and the local odds
//! are not read at all.
//!
//! That makes an encounter a coin with one face. `CLAUDE.md`'s first law —
//! *an outcome is a distribution, not a binary* — says that is the same
//! defect the old rubble had: a fight that is either a massacre or nothing.
//! And the behavioural-ecology literature says the same thing from the other
//! end, which is the part worth writing down here rather than only in the
//! report:
//!
//! * **Escalation is negatively related to asymmetry.** Across the assessment
//!   models (Maynard Smith & Price's hawk–dove, Parker's assessment, Enquist
//!   & Leimar's sequential assessment) the contested quantity is *resource
//!   holding potential*, and the measured regularity is that the more
//!   lopsided the pair, the shorter and cheaper the contest. Most encounters
//!   never escalate at all.
//! * **In ants the graded channel is the assessment, not the damage.**
//!   Czaczkes et al. 2024 on *Lasius niger*: overt aggression does **not**
//!   vary with relatedness or spatial distance, while antennation and jerking
//!   do. Hölldobler's *Myrmecocystus mimicus* tournaments put hundreds of
//!   ants in display and almost none in a fight, and the border moves toward
//!   whichever colony is outnumbered.
//!
//! So the middle a colony fight is missing is not a finer damage curve. It is
//! **the encounter that does not become a fight** — and, crucially, the one
//! that still *tells the colony something*, which is what makes a border a
//! border instead of a coincidence.
//!
//! `Reports/animal-conflict-research-2026-09-14.md` is the sourcing, the
//! mapping onto every lever this engine has, and what it deliberately does
//! not model.
//!
//! # What this module is, and what it is careful not to be
//!
//! Pure arithmetic over numbers the caller already has. It owns no state, it
//! reads no world, and it decides nothing on its own — the caller rolls
//! against what it returns. That keeps it testable without a bed, which
//! `examples/trailfollow.rs`'s `mode=arith` half already established as the
//! cheap way to settle a question about a weighted sum.
//!
//! **It is a capacity, never an exemption.** `dead-ends.md` :272, :276, :403
//! record four successive support models that died by giving one side an
//! exempt state, and the held-world survey (§10a) names that rejection as
//! binding on anything in this area. [`COMMIT_FLOOR`] is why this obeys it:
//! the commitment probability is bounded **below** by a non-zero floor at
//! every asymmetry, so an ant facing a beetle is *reluctant* and never
//! *forbidden*. There is no configuration of this module under which an
//! animal cannot be attacked.

/// **The floor on commitment: how willing the most hopelessly outmatched
/// animal in the world still is.**
///
/// This constant is the whole of this module's compliance with
/// `dead-ends.md` :272/:276/:403 — *protection as an exemption rather than a
/// capacity multiplier*. A defender that is very hard to hurt makes an
/// attacker very reluctant; it never makes it unable. At 0.05 a maximally
/// outmatched attacker still commits on one encounter in twenty, which over
/// a colony's worth of encounters is a steady trickle rather than a wall —
/// and it is exactly the trickle that lets the swarm rule
/// (`OrganismState::gnawed`, damage banked on the *victim*) bring down what
/// no single mouth can.
///
/// **Not zero, and the difference is not cosmetic.** At zero this would be a
/// rule about who may be attacked, which is the shape that has failed here
/// four times. At 0.05 it is a rule about who is *worth* attacking, which is
/// the shape the literature actually describes.
pub const COMMIT_FLOOR: f32 = 0.05;

/// **Default slope of the assessment, in units of "how sharply does an
/// asymmetry change my mind".**
///
/// Set from the geometry rather than from a measurement, and that is stated
/// rather than hidden: at `boldness = 4` a total asymmetry of +0.5 (a
/// comfortable advantage) reads ~0.88 and -0.5 reads ~0.16, which puts the
/// interesting range of the logistic across the range the engine's own
/// quantities actually span. `CLAUDE.md` asks for bars to be set from
/// measurement with headroom; this is not a bar, it is the shape of a dial,
/// and the standing direction for dials is *expose rather than balance*
/// (`Reports/lanes/evolution-lab-coordinator.md`). `PIXEL_PHYSICS_CONTEST_
/// BOLDNESS` moves it without a rebuild.
pub const BOLDNESS_DEFAULT: f32 = 4.0;

/// **Default weight on the numerical term against the strength term.**
///
/// 1.0: being outnumbered two-to-one in contact weighs the same as facing a
/// plate your jaw makes a third of the progress on. That parity is a
/// starting point and is meant to be moved — it is the single most
/// interesting dial here, because the empirical literature disagrees with
/// itself about exactly this. Lanchester's square law predicts numbers
/// should dominate for animals that fight all-against-all, and mature
/// *Myrmecocystus* colonies with more workers do win tournaments; but the
/// one quantitative test on a non-human animal — fire ant (*Solenopsis
/// invicta*) mortality across numerical ratios — came out approximately
/// **linear**, not square. So "numbers matter more than individual quality"
/// is well supported and "numbers matter quadratically" is not, and 1.0 is
/// the honest place to start an owner who wants to find out.
pub const NUMBERS_WEIGHT_DEFAULT: f32 = 1.0;

/// **What a display writes into the alarm plane**, against
/// `pheromone::ALARM_DEPOSIT`'s 240 for a bite.
///
/// **A sixth of a wound, and the ratio is the design.** An alarm is written
/// by one event against a plane that is otherwise zero
/// (`pheromone::ALARM_DEPOSIT`'s own doc), and a bite is meant to saturate
/// it — *"a swarm on one animal should read as loud"*. A display must not:
/// it happens on encounters that are common by construction, at a border
/// where two colonies are in constant contact, and a display as loud as a
/// wound would peg the plane flat and destroy the only distal signal a
/// colony has that somebody is actually being killed.
///
/// It is also the reason the display is not billed in joules. `cry_alarm`'s
/// own doc records what happened the last time a cost landed on the wrong
/// side of a fight — `armour_fraction`'s toothless plate — and the size of
/// this verb has not been measured. A deposit that decays at `ALARM_RHO`
/// within about a second and a half of play is a *small, self-clearing*
/// price paid in signal rather than in energy; if measurement says it wants
/// an energy account too, that is a change to make with a number in hand.
pub const DISPLAY_DEPOSIT: u8 = 40;

/// **What one closure of this jaw takes off a cell of that armour**, 0..=1.
///
/// The engine's own curve, lifted here so that the fight's copy and the
/// assessment's copy cannot drift apart. `creature.rs` carries the same
/// three lines inline at the gnaw site and its comment says why that matters
/// — *"a second copy of `(bite/armour)^2` is how the fight and the meal come
/// to disagree about how hard a beetle is"*. The attack site now calls this;
/// the gnaw site is left alone deliberately, because it reaches the number
/// through `adjacent_food_counted`'s own scan and pulling that apart is a
/// wider change than this one is allowed to be.
///
/// **Clamped, and the clamp is load-bearing.** `held-world-game-concept-
/// 2026-09-13.md` §10a records a whole design argument built on the
/// unclamped reading: damage cannot exceed 1.0, and 1.0 is *one cell*, not
/// a kill. The curve saturates rather than blowing up, and there is a
/// standing test asserting it is quadratic in the ratio with a continuity
/// check at `bite == armour`.
#[inline]
pub fn bite_progress(bite: f32, armour: f32) -> f32 {
    let ratio = if armour <= 0.0 { 1.0 } else { (bite / armour).clamp(0.0, 1.0) };
    ratio * ratio
}

/// **The local odds, as one signed number in -1..=1.**
///
/// `+1` is "everyone touching me is mine", `-1` is "everyone touching me is
/// a stranger", `0` is an even contact or an empty one. Derived from the
/// same body-ring walk `nearest_foe` already makes, so it costs nothing and
/// is read at exactly the scale a melee is fought at.
///
/// **`kin` includes the animal doing the looking.** A side of one is still a
/// side, and this argument is the count of animals on *my* side against the
/// count on theirs. Getting that wrong is not a rounding error: without it a
/// fair duel reads as 0 against 1, i.e. maximally outnumbered, for **both**
/// contestants at once.
///
/// **Animals, not cells, and it was cells for one afternoon.** A shipped
/// guard caught it — a lone attacker facing one two-celled defender read as
/// outnumbered two to one, and the median time to breach a maximally
/// armoured ant went from 18 frames to 126. `BrainInput::Crowding` counts
/// *cells* in r=2 and is right to, because it is asking how full the
/// neighbourhood is; this is asking how many opponents there are, and those
/// are different questions that happen to be computed from the same walk.
///
/// **Zero when nothing is touching**, which is the same value an even
/// contact gives — deliberately, because both mean *the numbers tell me
/// nothing*, and manufacturing a distinction there would be a reading the
/// data does not support.
#[inline]
pub fn numbers(kin: u32, foes: u32) -> f32 {
    let total = kin + foes;
    if total == 0 {
        return 0.0;
    }
    (kin as f32 - foes as f32) / total as f32
}

/// What an animal has read off the encounter it is standing in.
#[derive(Clone, Copy, Debug, Default)]
pub struct Assessment {
    /// Progress **one of my bites** makes on the cell I would strike, 0..=1.
    pub mine: f32,
    /// Progress **one of its bites** makes on the cell it would strike back
    /// at, 0..=1. Zero for anything that cannot bite at all, which is the
    /// case that has to keep working: a plant-eating rival is not a threat
    /// and an assessment that read it as one would stop colonies fighting
    /// over food, which is the whole thing being built.
    pub theirs: f32,
    /// [`numbers`], -1..=1.
    pub numbers: f32,
}

impl Assessment {
    /// **The total asymmetry this animal reads**, before the slope is
    /// applied. Positive favours me.
    ///
    /// A plain weighted sum of two terms on the same signed scale, and it is
    /// worth saying out loud that this is a *sum* rather than a product:
    /// `CLAUDE.md`'s standing warning about weighted sums is that changing
    /// what one term can express reallocates the whole thing, so the two
    /// terms here are deliberately given the same -1..=1 codomain and one
    /// explicit weight between them, rather than one being a bounded
    /// fraction and the other unbounded.
    #[inline]
    pub fn asymmetry(&self, numbers_weight: f32) -> f32 {
        (self.mine - self.theirs) + numbers_weight * self.numbers
    }
}

/// **How likely this animal is to commit**, in `COMMIT_FLOOR..=1`.
///
/// A logistic on the asymmetry, floored. Three properties are the point, and
/// each is asserted by a test in this module:
///
/// 1. **At parity it is a coin.** Two identical animals with even numbers
///    read ~0.5, which is the war of attrition and is what makes an evenly
///    matched border grind rather than resolve.
/// 2. **It is monotone in the asymmetry**, so a bigger advantage is never
///    less inviting — the property a sum of terms can lose the moment
///    anybody adds a third one.
/// 3. **It never reaches zero and never exceeds one.** The floor is the
///    capacity-not-exemption rule; the ceiling is what makes "assessment
///    off" and "assessment at a hopeless advantage" the same behaviour,
///    which is how the A/B stays interpretable.
#[inline]
pub fn commitment(a: Assessment, boldness: f32, numbers_weight: f32) -> f32 {
    let x = boldness * a.asymmetry(numbers_weight);
    // `exp` rather than `brain::squash`: that one is a different curve with
    // a different codomain, and round 33 records what happens when a
    // squash's negative half is clamped away (every degree of "would rather
    // not" landing on exactly 0.0, and 18-22% of a bed going quiet). This
    // wants the full sigmoid, both halves live.
    let logistic = 1.0 / (1.0 + (-x).exp());
    COMMIT_FLOOR + (1.0 - COMMIT_FLOOR) * logistic
}

/// **Is assessment on at all?**
///
/// **On by default** — the standing owner rule for behaviours is *ship
/// everything on*. The switch exists so the A/B runs from **one binary**,
/// which is `CLAUDE.md`'s stale-binary gotcha made avoidable:
/// `PIXEL_PHYSICS_CONTEST=off` puts the old unconditional bite and the new
/// assessed one in the same build, with no recompile sitting between the
/// arms to become the thing that actually changed.
///
/// Off is **exactly** the previous behaviour rather than an approximation of
/// it: the caller substitutes a commitment of 1.0, so every encounter that
/// used to bite still bites, on the same RNG draw in the same order.
pub fn enabled() -> bool {
    use std::sync::OnceLock;
    static ON: std::sync::OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("PIXEL_PHYSICS_CONTEST").map(|v| v != "off" && v != "0").unwrap_or(true))
}

/// [`BOLDNESS_DEFAULT`], overridable by `PIXEL_PHYSICS_CONTEST_BOLDNESS`.
///
/// **Clamped at zero from below rather than rejected**, because a negative
/// slope is a coherent thing to ask for by accident and an incoherent thing
/// to simulate: it would make an animal keener the worse its odds, which is
/// not a strategy any literature describes. Zero is meaningful and allowed —
/// it reads every encounter as parity, i.e. assessment that gathers no
/// information, which is the honest null arm for "does *reading* the
/// opponent matter, as against merely hesitating".
pub fn boldness() -> f32 {
    use std::sync::OnceLock;
    static V: OnceLock<f32> = OnceLock::new();
    *V.get_or_init(|| env_f32("PIXEL_PHYSICS_CONTEST_BOLDNESS", BOLDNESS_DEFAULT).max(0.0))
}

/// [`NUMBERS_WEIGHT_DEFAULT`], overridable by `PIXEL_PHYSICS_CONTEST_NUMBERS`.
///
/// Clamped at zero from below for the reason [`boldness`] gives. Zero is the
/// arm that asks whether the *numerical* half is what produces a border, as
/// against the strength half — and those are separable here in a way they
/// are not in a real ant, which is most of what a simulation is for.
pub fn numbers_weight() -> f32 {
    use std::sync::OnceLock;
    static V: OnceLock<f32> = OnceLock::new();
    *V.get_or_init(|| env_f32("PIXEL_PHYSICS_CONTEST_NUMBERS", NUMBERS_WEIGHT_DEFAULT).max(0.0))
}

/// [`DISPLAY_DEPOSIT`], overridable by `PIXEL_PHYSICS_CONTEST_DISPLAY`.
///
/// Saturating rather than wrapping on a value past 255, so a fat-fingered
/// dial reads as "as loud as this plane goes" instead of as silence — which
/// is the failure mode that looks exactly like the mechanism being dead.
pub fn display_deposit() -> u8 {
    use std::sync::OnceLock;
    static V: OnceLock<u8> = OnceLock::new();
    *V.get_or_init(|| match std::env::var("PIXEL_PHYSICS_CONTEST_DISPLAY") {
        Ok(s) => s.parse::<f32>().map(|v| v.clamp(0.0, 255.0) as u8).unwrap_or(DISPLAY_DEPOSIT),
        Err(_) => DISPLAY_DEPOSIT,
    })
}

fn env_f32(key: &str, fallback: f32) -> f32 {
    std::env::var(key).ok().and_then(|s| s.parse::<f32>().ok()).filter(|v| v.is_finite()).unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The floor is the capacity-not-exemption rule, so it gets the test
    /// that names it. Swept rather than spot-checked, because the claim is
    /// about the *whole* domain: there must be no asymmetry, at any dial
    /// setting, at which an animal cannot attack.
    #[test]
    fn commitment_never_reaches_zero_at_any_asymmetry_or_dial() {
        for &boldness in &[0.0f32, 1.0, 4.0, 20.0, 1e6] {
            for &w in &[0.0f32, 1.0, 8.0] {
                for step in 0..=40 {
                    let t = step as f32 / 20.0 - 1.0;
                    let a = Assessment { mine: 0.0f32.max(t), theirs: 0.0f32.max(-t), numbers: t };
                    let c = commitment(a, boldness, w);
                    assert!(c >= COMMIT_FLOOR, "commitment {c} fell under the floor at t={t} boldness={boldness} w={w}");
                    assert!(c <= 1.0, "commitment {c} exceeded 1 at t={t} boldness={boldness} w={w}");
                }
            }
        }
    }

    /// An even match is a coin. This is the property that gives the mechanic
    /// its middle: without it an assessment is just a second threshold.
    #[test]
    fn an_even_match_is_close_to_a_coin() {
        let a = Assessment { mine: 0.4, theirs: 0.4, numbers: 0.0 };
        let c = commitment(a, BOLDNESS_DEFAULT, NUMBERS_WEIGHT_DEFAULT);
        assert!((c - 0.525).abs() < 0.01, "an even match read {c}, which is not a coin");
    }

    /// Monotone in the asymmetry, which a weighted sum can lose silently the
    /// moment a term changes sign convention.
    #[test]
    fn commitment_rises_with_advantage() {
        let mut last = -1.0;
        for step in 0..=20 {
            let t = step as f32 / 10.0 - 1.0;
            let c = commitment(Assessment { mine: 0.0f32.max(t), theirs: 0.0f32.max(-t), numbers: 0.0 }, BOLDNESS_DEFAULT, NUMBERS_WEIGHT_DEFAULT);
            assert!(c > last, "commitment fell from {last} to {c} as the advantage rose to {t}");
            last = c;
        }
    }

    /// **Being outnumbered has to move it on its own**, with the strength
    /// terms held identical — otherwise the numerical half is decoration and
    /// every result about colony size is really about armour. The positive
    /// control for the term, in the form `CLAUDE.md` asks for: construct the
    /// case whose answer is known to be non-zero and check the instrument
    /// reports it.
    #[test]
    fn numbers_move_commitment_with_strength_held_fixed() {
        let even = Assessment { mine: 0.4, theirs: 0.4, numbers: 0.0 };
        let outnumbered = Assessment { numbers: -1.0, ..even };
        let supported = Assessment { numbers: 1.0, ..even };
        let (lo, mid, hi) = (
            commitment(outnumbered, BOLDNESS_DEFAULT, NUMBERS_WEIGHT_DEFAULT),
            commitment(even, BOLDNESS_DEFAULT, NUMBERS_WEIGHT_DEFAULT),
            commitment(supported, BOLDNESS_DEFAULT, NUMBERS_WEIGHT_DEFAULT),
        );
        assert!(lo < mid && mid < hi, "numbers did not order commitment: {lo} / {mid} / {hi}");
        // ...and by a margin that a roll can actually express. A term that
        // moves the third decimal is a term nobody will ever see fire.
        assert!(hi - lo > 0.5, "the numerical term spans only {} of the commitment range", hi - lo);
    }

    /// `numbers_weight = 0` must remove the term completely rather than
    /// merely shrinking it — that is what makes it a usable null arm.
    #[test]
    fn a_zero_numbers_weight_is_a_true_null() {
        let even = Assessment { mine: 0.4, theirs: 0.4, numbers: 0.0 };
        let outnumbered = Assessment { numbers: -1.0, ..even };
        assert_eq!(commitment(even, BOLDNESS_DEFAULT, 0.0), commitment(outnumbered, BOLDNESS_DEFAULT, 0.0));
    }

    /// The engine's own damage curve, asserted here so that a change to it
    /// breaks something on this side too. The numbers are the ones
    /// `creature.rs`'s existing test names: half the plate makes a quarter
    /// of the progress, and the curve is continuous at `bite == armour`.
    #[test]
    fn bite_progress_is_the_engines_own_quadratic() {
        assert!((bite_progress(0.5, 1.0) - 0.25).abs() < 1e-6);
        assert!((bite_progress(1.0, 1.0) - 1.0).abs() < 1e-6);
        // Saturating, not blowing up -- the correction §10a records.
        assert!((bite_progress(2.0, 1.0) - 1.0).abs() < 1e-6);
        // Armour of zero is not a division by zero and not an exemption.
        assert!((bite_progress(1.0, 0.0) - 1.0).abs() < 1e-6);
    }

    /// An empty ring and an even one both read zero, and the lopsided cases
    /// saturate. Cheap, and it is the term every colony-size claim rests on.
    #[test]
    fn numbers_reads_the_ring() {
        assert_eq!(numbers(0, 0), 0.0);
        assert_eq!(numbers(2, 2), 0.0);
        assert_eq!(numbers(0, 3), -1.0);
        assert_eq!(numbers(3, 0), 1.0);
        assert!((numbers(2, 1) - 1.0 / 3.0).abs() < 1e-6);
    }

    /// **A fair duel is parity**, and it is the case the caller gets wrong
    /// by forgetting to count itself. Asserted here rather than only at the
    /// call site because the `+ 1` is this function's contract, not the
    /// caller's convenience: a one-against-one that reads as -1 makes both
    /// animals withdraw from each other for ever, which is how this arrived
    /// (median breach 18 frames -> 126, caught by a shipped guard).
    #[test]
    fn a_duel_between_two_animals_is_parity() {
        assert_eq!(numbers(1, 1), 0.0);
        let even = Assessment { mine: 0.6, theirs: 0.6, numbers: numbers(1, 1) };
        assert!((commitment(even, BOLDNESS_DEFAULT, NUMBERS_WEIGHT_DEFAULT) - 0.525).abs() < 0.01);
        // ...and two of mine against one of theirs is an advantage, while
        // one of mine against two of theirs is not.
        assert!(numbers(2, 1) > 0.0 && numbers(1, 2) < 0.0);
    }
}
