//! **Growing tips that wait for a card** -- the plant-engine hook behind the
//! Heartwood probe (`cargo run --release --bin heartwood`).
//!
//! Heartwood's pitch is *your deck is your tree's genes*: the player taps a
//! growing tip and plays a card on it, and the card decides where that tip's
//! next few cells go. This module is the only thing the shipped plant engine
//! knows about that. It holds, per world:
//!
//! - which organisms are **card-grown** (`organisms`). A shoot tip of one of
//!   these never grows on its own: with no card it *holds* -- neither grows
//!   nor ages out -- which is what makes *doing nothing* lose;
//! - the **card each tip is carrying** (`steer`), keyed by the tip's cell and
//!   carried forward by `plant.rs` as the tip advances.
//!
//! **Off everywhere but that game.** `World::tip_cards` is `None` in every
//! world any other binary builds, and every hook in `plant.rs` is then a
//! single `Option` read that answers [`Card::Free`] -- the engine's own
//! growth, bit for bit. Roots are never gated: a card-grown tree still
//! drinks and anchors by itself.
//!
//! **What a card overrides, and what it leaves alone.** A carried card
//! replaces the four steering weights (`[continuation, light, wind, up]`,
//! `plant.rs`'s `Behavior::Grow` arm) with two -- *keep going* and *aim* --
//! and replaces the tier's up-reference with the card's own aim. Everything
//! that makes a tree a tree is still the engine's: what a step costs, the
//! turgor height bound, heading momentum and stem stiffness (so a turn is a
//! curve, not a corner), leaves placed at nodes and charged for, wood that
//! thickens under its leaves, upkeep, and the structural break of a heavy
//! crown on a thin stem. The card says *where*; the tree still decides
//! whether it can afford to.
//!
//! Keyed by cell rather than by a stored id because a growing tip *is* a
//! cell: each step creates the child and retires the parent, so the key moves
//! with the tip exactly once per step ([`moved`]). `tag` is the game's stable
//! name for one tip across those moves.

use std::collections::BTreeMap;

use super::cell::OrganismId;
use super::world::World;

/// What one carried card asks of the tip, for its next `cells_left` steps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Steer {
    /// The game's name for this tip. Survives every step; a fork's lateral
    /// gets a fresh one (see [`TipCards::forks`]).
    pub tag: u32,
    /// Steps still to grow under this card. 0 is a tip that has finished its
    /// card and is waiting for the next one -- it keeps its entry so the game
    /// can still find it by `tag`.
    pub cells_left: u8,
    /// The direction this card pulls toward, unit length. Takes the place
    /// of the tier's up-reference in the engine's score.
    pub aim: (f32, f32),
    /// Share of the steering budget given to `aim`.
    pub aim_weight: f32,
    /// Share given to carrying straight on along the tip's own heading.
    pub keep_weight: f32,
    /// Every step is a node, so every step puts out a leaf spray (the
    /// species' own `leaf_cluster`, charged as the engine charges it).
    pub leaf_every: bool,
    /// On the next step, split: the engine builds its own lateral as well
    /// as the continuation, and both carry the rest of this card, aimed
    /// apart. Cleared by that step.
    pub fork: bool,
    /// Visits in a row this card's tip ran `Grow` and did not step, and the
    /// last reason (`plant::GrowWhy as u8`). Written by [`stalled`], cleared
    /// by [`moved`]; the game reads them to call a card blocked.
    pub stalls: u16,
    pub why: u8,
}

/// The hook's answer for one shoot tip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Card {
    /// Not a card-grown organism, or not a shoot tip, or no table: the
    /// engine's own growth.
    Free,
    /// Card-grown, carrying no card (or one already used up): wait.
    Hold,
    /// Carrying a card with steps left.
    Steer(Steer),
}

/// A fork the engine carried out, for the game to read and clear.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fork {
    pub parent_tag: u32,
    pub child_tag: u32,
}

#[derive(Clone, Debug, Default)]
pub struct TipCards {
    /// Organisms whose shoot tips grow only on a card.
    pub organisms: Vec<OrganismId>,
    /// The card each tip is carrying, by the tip's current cell.
    pub steer: BTreeMap<(i32, i32), Steer>,
    /// Next fresh tag; the game hands these out too, through [`TipCards::new_tag`].
    pub next_tag: u32,
    /// Forks carried out since the game last drained this.
    pub forks: Vec<Fork>,
    /// Steps grown under a card, all organisms -- the did-it-fire counter.
    pub steered_steps: u64,
    /// Tip visits that held for want of a card.
    pub held_visits: u64,
    /// **How many times faster card-grown worlds run the plant clock.**
    /// Copied into `Clock::growth_speedup` by the game; kept here too so a
    /// harness can echo it.
    pub speedup: u32,
}

impl TipCards {
    pub fn new_tag(&mut self) -> u32 {
        self.next_tag += 1;
        self.next_tag
    }
}

/// The hook `plant.rs` asks at the top of a shoot tip's `Grow`.
#[inline]
pub fn card_for(world: &World, organism_id: OrganismId, x: i32, y: i32) -> Card {
    let Some(t) = world.tip_cards.as_deref() else { return Card::Free };
    if !t.organisms.contains(&organism_id) {
        return Card::Free;
    }
    match t.steer.get(&(x, y)) {
        Some(s) if s.cells_left > 0 => Card::Steer(*s),
        _ => Card::Hold,
    }
}

/// Count one held visit. Observation only.
#[inline]
pub fn note_hold(world: &mut World) {
    if let Some(t) = world.tip_cards.as_deref_mut() {
        t.held_visits += 1;
    }
}

/// The tip at `from` grew its continuation into `to`: carry its card along,
/// one step spent. A fork flag survives a step that could not afford or find
/// room for its lateral, so the split comes on a later step of the same card;
/// [`forked`] is what spends it.
pub fn moved(world: &mut World, from: (i32, i32), to: (i32, i32)) {
    let Some(t) = world.tip_cards.as_deref_mut() else { return };
    if let Some(mut s) = t.steer.remove(&from) {
        s.cells_left = s.cells_left.saturating_sub(1);
        s.stalls = 0;
        t.steer.insert(to, s);
        t.steered_steps += 1;
    }
}

/// A forced fork happened: the continuation is at `primary`, the lateral at
/// `lateral`, and the axis they left was heading `heading`. The lateral gets
/// the rest of the card under a new tag, aimed along the way it left; the
/// continuation is re-aimed as its mirror image about the old heading, so the
/// pair open into a V rather than one carrying on and one peeling off.
pub fn forked(world: &mut World, primary: (i32, i32), lateral: (i32, i32), origin: (i32, i32), heading: (f32, f32)) {
    let Some(t) = world.tip_cards.as_deref_mut() else { return };
    let Some(mut p) = t.steer.get(&primary).copied() else { return };
    let away = unit(((lateral.0 - origin.0) as f32, (lateral.1 - origin.1) as f32));
    let h = unit(heading);
    // Reflect `away` about `h`: 2(a.h)h - a.
    let d = away.0 * h.0 + away.1 * h.1;
    let mirror = unit((2.0 * d * h.0 - away.0, 2.0 * d * h.1 - away.1));
    p.fork = false;
    let mut l = p;
    l.tag = t.new_tag();
    l.aim = away;
    p.aim = mirror;
    t.steer.insert(primary, p);
    t.steer.insert(lateral, l);
    t.forks.push(Fork { parent_tag: p.tag, child_tag: l.tag });
}

/// The card-carrying tip at `at` ran `Grow` and did not step, for `why`
/// (`plant::GrowWhy as u8`). Observation only: nothing in the engine reads it.
pub fn stalled(world: &mut World, at: (i32, i32), why: u8) {
    let Some(t) = world.tip_cards.as_deref_mut() else { return };
    if let Some(s) = t.steer.get_mut(&at) {
        s.stalls = s.stalls.saturating_add(1);
        s.why = why;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_table_means_free_growth() {
        let w = World::new(crate::sim::chunk::Rect::new(0, 0, 15, 15));
        assert_eq!(card_for(&w, 1, 3, 3), Card::Free);
    }

    #[test]
    fn a_gated_tip_holds_until_a_card_arrives_and_the_card_moves_with_it() {
        let mut w = World::new(crate::sim::chunk::Rect::new(0, 0, 15, 15));
        w.tip_cards = Some(Box::new(TipCards { organisms: vec![7], ..Default::default() }));
        assert_eq!(card_for(&w, 7, 3, 3), Card::Hold);
        assert_eq!(card_for(&w, 8, 3, 3), Card::Free, "another organism is never gated");
        let s = Steer { tag: 1, cells_left: 2, aim: (0.0, -1.0), aim_weight: 1.0, keep_weight: 0.3, leaf_every: false, fork: true, stalls: 0, why: 0 };
        w.tip_cards.as_mut().unwrap().steer.insert((3, 3), s);
        assert_eq!(card_for(&w, 7, 3, 3), Card::Steer(s));
        moved(&mut w, (3, 3), (3, 2));
        assert_eq!(card_for(&w, 7, 3, 3), Card::Hold);
        let Card::Steer(after) = card_for(&w, 7, 3, 2) else { panic!("the card did not follow the tip") };
        assert_eq!((after.cells_left, after.fork, after.tag), (1, true, 1), "a fork not yet carried out waits for a later step");
        moved(&mut w, (3, 2), (3, 1));
        assert_eq!(card_for(&w, 7, 3, 1), Card::Hold, "a spent card leaves the tip waiting");
        assert!(w.tip_cards.as_ref().unwrap().steer.contains_key(&(3, 1)), "...but still findable by its tag");
    }

    #[test]
    fn a_fork_opens_into_a_v_about_the_old_heading() {
        let mut w = World::new(crate::sim::chunk::Rect::new(0, 0, 15, 15));
        let s = Steer { tag: 1, cells_left: 3, aim: (0.0, -1.0), aim_weight: 1.0, keep_weight: 0.3, leaf_every: false, fork: false, stalls: 0, why: 0 };
        w.tip_cards = Some(Box::new(TipCards { organisms: vec![7], next_tag: 1, ..Default::default() }));
        w.tip_cards.as_mut().unwrap().steer.insert((5, 4), s);
        // Heading straight up from (5,5); continuation went to (5,4), lateral up-right to (6,4).
        forked(&mut w, (5, 4), (6, 4), (5, 5), (0.0, -1.0));
        let t = w.tip_cards.as_ref().unwrap();
        let (p, l) = (t.steer[&(5, 4)], t.steer[&(6, 4)]);
        assert!(l.aim.0 > 0.5 && p.aim.0 < -0.5, "the pair must open both ways: {:?} {:?}", p.aim, l.aim);
        assert_ne!(p.tag, l.tag);
        assert!(!p.fork && !l.fork, "a fork is spent once it happens");
        assert_eq!(t.forks, vec![Fork { parent_tag: 1, child_tag: l.tag }]);
    }
}
