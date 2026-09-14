//! **The options menu** — `M`.
//!
//! Owner's ask, 2026-09-14: *"We need a menu with options. The first I would
//! add is the ability to turn off plant destruction or breaking due to stress
//! (which should be off by default)."*
//!
//! **Every row here is a switch the engine already has.** Nothing in this
//! module adds behaviour; it exposes settings that existed and were reachable
//! only from a key nobody could guess, or from the lab's parameters panel, or
//! from nowhere at all. That is deliberate — a menu is discoverability, and a
//! menu row that is also a new mechanic is two changes wearing one commit.
//!
//! **Why a menu rather than more keys.** There are twenty bindings already and
//! the legend fills the bottom-left corner. A setting is a thing you change
//! once and then forget; a verb is a thing you press. Keeping them apart is
//! what stops the legend growing until it is the screen.

use super::Druid;

/// One row of the menu.
///
/// An enum with a `match` per question rather than a table of closures: the
/// rows read and write scattered corners of `Druid` and `World`, and closures
/// over `&mut Druid` in a `const` table do not exist in this language.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Setting {
    /// `World::plant_load_failure` — whether a living plant can be pulled
    /// apart by its own load.
    PlantBreak,
    /// `World::plant_bending` — whether it leans under load and wind.
    PlantBend,
    /// [`Druid::unlimited`].
    Unlimited,
    /// [`Druid::show_keys`].
    Keys,
    /// `Renderer::held_look` — a cycle rather than a toggle.
    HeldLook,
    /// `World::held` itself.
    Held,
    /// [`Druid::scent`] — which plane `G` writes to.
    Scent,
}

pub const SETTINGS: &[Setting] = &[Setting::PlantBreak, Setting::PlantBend, Setting::Scent, Setting::Unlimited, Setting::Keys, Setting::HeldLook, Setting::Held];

impl Setting {
    pub fn label(self) -> &'static str {
        match self {
            Setting::PlantBreak => "PLANTS BREAK UNDER STRESS",
            Setting::PlantBend => "PLANTS BEND UNDER LOAD",
            Setting::Unlimited => "UNLIMITED POWER",
            Setting::Keys => "SHOW THE KEY LIST",
            Setting::HeldLook => "HOW HELD GROUND IS DRAWN",
            Setting::Held => "TIME IS HELD",
            Setting::Scent => "SCENT TRAIL WRITES",
        }
    }

    /// One line saying what turning it off buys or costs, because a switch
    /// whose consequence is invisible is a switch nobody touches twice.
    ///
    /// **Kept short enough to fit**, which is not a matter of taste: the
    /// first draft ran 395 pixels inside a 314-pixel panel and the guard
    /// `the_options_menu_fits_and_has_glyphs_for_everything` caught it. The
    /// panel widened to 404 as well, so there is room, but a note that grows
    /// past the width will go red rather than off the edge.
    pub fn note(self) -> &'static str {
        match self {
            Setting::PlantBreak => "OFF - NOTHING LIVING IS TORN APART BY ITS WEIGHT",
            Setting::PlantBend => "OFF - A STEM STANDS WHERE IT GREW, WIND OR NOT",
            Setting::Unlimited => "ON - NOTHING CHARGED AND NOTHING COLLECTED",
            Setting::Keys => "THE LIST IN THE BOTTOM CORNER",
            Setting::HeldLook => "UNCHANGED, OR ONE COLD HUE OUTSIDE YOUR CIRCLES",
            Setting::Held => "OFF - THE WHOLE WORLD RUNS. A CONTROL, NOT THE GAME",
            // The one row whose note is a warning rather than a trade.
            Setting::Scent => "FOOD IS A REAL ROUTE THAT NO ANT CAN READ YET",
        }
    }

    pub fn value(self, game: &Druid) -> String {
        let on = |b: bool| if b { "ON".to_string() } else { "OFF".to_string() };
        match self {
            Setting::PlantBreak => on(game.world.plant_load_failure),
            Setting::PlantBend => on(game.world.plant_bending),
            Setting::Unlimited => on(game.unlimited),
            Setting::Keys => on(game.show_keys),
            Setting::HeldLook => game.renderer.held_look.label().to_uppercase(),
            Setting::Held => on(game.world.held),
            Setting::Scent => match game.scent {
                crate::sim::pheromone::Channel::A => "HOME".to_string(),
                _ => "FOOD".to_string(),
            },
        }
    }

    /// **Advance the row.** A toggle for five of them and a cycle for the
    /// look, which is why this is one verb rather than a `bool` the caller
    /// flips: the menu should not have to know which rows are two-valued.
    pub fn advance(self, game: &mut Druid) {
        match self {
            Setting::PlantBreak => {
                game.world.plant_load_failure = !game.world.plant_load_failure;
                // **The reproduction behind this call: `CLAUDE.md`'s "reproduce
                // before you fix."** A bare field write here left an already-
                // settled beam standing after the switch went back on --
                // `structural::tests::flipping_the_load_failure_switch_the_way_
                // the_menu_does_does_not_retroactively_recheck_a_settled_beam`
                // reproduces it directly. See `schedule_structural_recheck_of_
                // all_living_plants`'s own doc for why a toggle alone never
                // reached those cells.
                game.world.schedule_structural_recheck_of_all_living_plants();
            }
            Setting::PlantBend => game.world.plant_bending = !game.world.plant_bending,
            Setting::Unlimited => game.unlimited = !game.unlimited,
            Setting::Keys => game.show_keys = !game.show_keys,
            Setting::HeldLook => game.renderer.cycle_held_look(),
            Setting::Held => game.world.held = !game.world.held,
            Setting::Scent => game.cycle_scent(),
        }
    }
}

/// Where the cursor is. `None` closed — one piece of state rather than an
/// `open: bool` beside a cursor that can disagree with it, the same shape
/// [`super::founding::Offer`] uses.
#[derive(Default)]
pub struct Menu {
    pub row: usize,
}

impl Menu {
    pub fn step(&mut self, delta: i32) {
        let n = SETTINGS.len() as i32;
        self.row = (((self.row as i32 + delta) % n + n) % n) as usize;
    }

    pub fn current(&self) -> Setting {
        SETTINGS[self.row.min(SETTINGS.len() - 1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every label, value and note has a glyph in the 5x7 font. The menu's
    /// strings never pass through `Readout`, so `hud`'s own sweep cannot see
    /// them, and a missing glyph draws as a blank gap rather than an error.
    #[test]
    fn every_word_the_menu_draws_has_a_glyph() {
        let mut checked = 0;
        for s in SETTINGS {
            for text in [s.label(), s.note()] {
                for c in text.chars() {
                    assert!(crate::hud::has_glyph(c), "no glyph for {c:?} in {text:?}");
                    checked += 1;
                }
            }
        }
        assert!(checked > 200, "only {checked} characters swept; this guard would pass on nothing");
    }

    /// The cursor wraps in both directions and never indexes off the end —
    /// the failure a modulo on a `usize` makes at row 0.
    #[test]
    fn the_cursor_wraps_both_ways() {
        let mut m = Menu::default();
        m.step(-1);
        assert_eq!(m.row, SETTINGS.len() - 1);
        m.step(1);
        assert_eq!(m.row, 0);
        for _ in 0..SETTINGS.len() * 3 {
            m.step(1);
            assert!(m.row < SETTINGS.len());
        }
    }

    /// **Every row names a distinct setting.** A duplicated arm would give
    /// two rows that move one switch, which reads as one of them being dead.
    #[test]
    fn no_two_rows_are_the_same_setting() {
        for (i, a) in SETTINGS.iter().enumerate() {
            for b in &SETTINGS[i + 1..] {
                assert_ne!(a, b, "{a:?} appears twice");
                assert_ne!(a.label(), b.label(), "{a:?} and {b:?} share a label");
            }
        }
    }
}
