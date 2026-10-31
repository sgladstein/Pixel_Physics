//! **The demo menu** (owner, 2026-10-31: *"Can we just have a menu in the
//! game to access all these demos?"*).
//!
//! Until this, a seat demo was reached only from the terminal
//! (`-- demo=queen`), and trying another meant quitting and relaunching. Now
//! the lab's DEMOS page (MENU's top row, or `\`) lists every playable seat
//! with one line on what you do in it, starts one on a click, and lists
//! **PLAIN LAB** for the way back.
//!
//! # Coming back is the point
//!
//! A demo replaces the box on screen with its own scenario and changes a
//! handful of things that are *not* the box: the ants' brain (the satchel
//! seat drops to the hand-built ant, the queen and raider hand in pools of
//! their own), the animal colour (the queen seat turns it off), the dial,
//! and any rule a scenario sets, which `Lab::reset` then carries as a dial
//! into every later rebuild. So the first demo started from the plain lab
//! remembers all of that in [`Home`], and leaving puts every one of them
//! back and rebuilds the box the player came from (its spec and scenario,
//! as `REBUILD` would -- what had grown in it since is gone, as it was the
//! moment the demo replaced it).
//!
//! **Switching seats goes through home too**: the settings are put back
//! before the next seat starts, so a seat opened from the menu after
//! another opens exactly as it does from the command line, and never
//! inherits the last seat's brain or rules.
//!
//! **Only a launch from outside a demo remembers.** A seat that restarts
//! itself (`satchel::next_round` calls `start_demo_with` mid-demo) goes
//! around [`Lab::launch_demo`], so it can never overwrite home with a
//! demo's own settings.

use super::{DemoSeat, Lab};
use crate::lab::{batch, params, scenario, scene, time};

/// One line of the DEMOS page.
pub struct Entry {
    /// `None` is the plain lab: no seat, every tool.
    pub seat: Option<DemoSeat>,
    /// The seat's name as the player knows it.
    pub name: &'static str,
    /// What you do there, in one line that fits the page.
    pub line: &'static str,
}

/// **Every row of the DEMOS page, in the order the owner listed them.**
/// The colony-mind seat is not here: it has no scenario and opens on a plain
/// box (its card says "not built yet"), so it would be a row that promises
/// a game and delivers the lab. Add it here when it ships.
///
/// The lines say what the seat *is*, not its numbers: each seat's card (`H`)
/// carries the goal, score and keys, and is owned by the seat's thread, so a
/// rule change there cannot leave this page saying something false.
pub const ENTRIES: [Entry; 7] = [
    Entry { seat: Some(DemoSeat::Queen), name: "QUEEN", line: "FOUND A COLONY, LAY, STORE FOOD FOR WINTER" },
    Entry { seat: Some(DemoSeat::Enemy), name: "RAIDER", line: "A CENTIPEDE: HUNT ANTS, STEAL A LARVA" },
    Entry { seat: Some(DemoSeat::Keeper), name: "KEEPER", line: "LIFT ANTS ONTO FAR FOOD BEFORE IT ROTS" },
    Entry { seat: Some(DemoSeat::Guest), name: "GUEST", line: "A BEETLE IN THE NEST: SMELL RIGHT, BEG" },
    Entry { seat: Some(DemoSeat::Scout), name: "SCOUT", line: "LEAD SISTERS TO FAR FOOD IN A LEAN YEAR" },
    Entry { seat: Some(DemoSeat::Satchel), name: "SATCHEL", line: "PLAY CARDS THAT EACH DEATH SETS OFF" },
    Entry { seat: None, name: "PLAIN LAB", line: "NO GOAL: THE BOX AND EVERY TOOL" },
];

/// **What a demo changes that is not the box**, taken when the first demo
/// starts from the plain lab and put back when the player leaves or
/// switches seats. See the module doc for why each field is here.
#[derive(Clone)]
pub struct Home {
    spec: scene::LabBox,
    scenario: Option<scenario::Scenario>,
    batch_spec: batch::BatchSpec,
    dials: params::Dials,
    new_ant: bool,
    own_pool: bool,
    lab_ant: usize,
    new_ant_mind: Option<Vec<std::sync::Arc<crate::sim::creature::mind::MindGenome>>>,
    creature_colour: crate::render::CreatureColour,
    camera: (i32, i32, i32, i32),
    requested: u32,
}

impl Home {
    fn take(lab: &Lab) -> Home {
        let r = &lab.renderer;
        Home {
            spec: lab.spec.clone(),
            scenario: lab.scenario.clone(),
            batch_spec: lab.batch_spec.clone(),
            dials: params::Dials::from_world(&lab.world),
            new_ant: lab.new_ant,
            own_pool: lab.own_pool,
            lab_ant: lab.lab_ant,
            new_ant_mind: lab.new_ant_mind.clone(),
            creature_colour: r.creature_colour,
            camera: (r.camera_x, r.camera_y, r.zoom, r.zoom_out_stride),
            requested: lab.time.requested,
        }
    }

    /// Everything but the box. The dials go onto the world on screen, so
    /// the rebuild that follows (`reset` reads them off it) carries home's
    /// rules rather than the last seat's.
    fn put_back_settings(&self, lab: &mut Lab) {
        self.dials.apply_to(&mut lab.world);
        lab.new_ant = self.new_ant;
        lab.own_pool = self.own_pool;
        lab.lab_ant = self.lab_ant;
        lab.new_ant_mind = self.new_ant_mind.clone();
        lab.world.minds = None;
        lab.renderer.creature_colour = self.creature_colour;
        if let Some(i) = time::PRESETS.iter().position(|p| *p == self.requested) {
            lab.time.set_preset(i);
        }
        lab.time.pause();
    }
}

/// The plain lab's row tint and the one playing now.
pub fn playing(lab: &Lab) -> Option<DemoSeat> {
    lab.demo.as_ref().map(|d| d.seat)
}

impl Lab {
    /// **A row of the DEMOS page**: start that seat, or (`None`) go back to
    /// the plain lab. Returns the notice line.
    pub fn play_demo(&mut self, seat: Option<DemoSeat>) -> String {
        match seat {
            Some(seat) => self.launch_demo(seat, Default::default()),
            None => self.leave_demo(),
        }
    }

    /// **Start a seat from anywhere**: the command line's `demo=` and the
    /// DEMOS page both come here. Remembers home on the way in from the
    /// plain lab, puts it back first when switching seats, and -- with a
    /// person at the keys ([`Lab::windowed`]) -- arms the landing pause and
    /// opens the seat card, as `bin/lab.rs` did for `demo=` alone. `card=0`
    /// in `args` opens straight onto the seat.
    pub fn launch_demo(&mut self, seat: DemoSeat, args: std::collections::BTreeMap<String, String>) -> String {
        match (&self.demo, &self.demo_home) {
            (None, _) => self.demo_home = Some(Home::take(self)),
            (Some(_), Some(home)) => home.clone().put_back_settings(self),
            (Some(_), None) => {}
        }
        let card = args.get("card").is_none_or(|v| v != "0");
        self.ui.close_panel();
        let msg = self.start_demo_with(seat, args);
        if self.windowed {
            if let Some(d) = self.demo.as_mut() {
                d.window = true;
            }
            if card {
                self.open_seat_card(false);
            }
        }
        msg
    }

    /// **Back to the plain lab**: the seat goes, home's settings come back,
    /// and the box the player came from is rebuilt, paused. With no home
    /// (a harness that armed a seat by hand) the seat goes and the default
    /// bed is built.
    pub fn leave_demo(&mut self) -> String {
        self.ui.close_panel();
        if self.demo.is_none() {
            return "NO DEMO RUNNING -- THIS IS THE PLAIN LAB".to_string();
        }
        self.demo = None;
        self.world.steered = None;
        match self.demo_home.take() {
            Some(home) => {
                home.put_back_settings(self);
                self.spec = home.spec;
                self.scenario = home.scenario;
                self.batch_spec = home.batch_spec;
                let (cx, cy, zoom, stride) = home.camera;
                self.renderer.camera_x = cx;
                self.renderer.camera_y = cy;
                self.renderer.zoom = zoom;
                self.renderer.zoom_out_stride = stride;
            }
            None => {
                self.scenario = None;
                self.spec = scene::LabBox::default();
            }
        }
        self.reset();
        if self.new_ant {
            self.give_new_ant_minds();
        }
        self.log_ant_brain();
        self.time.pause();
        self.view_dirty = true;
        self.touch_chrome();
        format!("PLAIN LAB -- ANT: {} -- SPACE TO RUN, \\ FOR THE DEMOS", self.ant_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Every seat on the page has a scenario that ships**, so no row
    /// promises a game and opens a plain box; and every playable seat is on
    /// the page. The positive control for "ships" is the colony mind, which
    /// has none and must fail the same load.
    #[test]
    fn every_row_opens_a_real_seat_and_every_real_seat_has_a_row() {
        for e in &ENTRIES {
            if let Some(seat) = e.seat {
                assert!(scenario::Scenario::load(&seat.scenario_name()).is_ok(), "{}: no scenario", e.name);
            }
            assert!(e.line.chars().all(crate::hud::has_glyph), "{}: a glyph the HUD cannot draw", e.name);
        }
        assert!(scenario::Scenario::load(&DemoSeat::Mind.scenario_name()).is_err(), "the control seat grew a scenario: list it");
        for seat in DemoSeat::ALL {
            let listed = ENTRIES.iter().any(|e| e.seat == Some(seat));
            assert_eq!(listed, seat != DemoSeat::Mind, "{seat:?}: on the page {listed}");
        }
        assert_eq!(ENTRIES.iter().filter(|e| e.seat.is_none()).count(), 1, "one way back");
    }

    /// **Into a seat, across to another, and back home**, with what a seat
    /// changes put back. The queen seat turns the animal colour off and
    /// hands her colony its own pool; the satchel seat drops to the
    /// hand-built ant. After both, PLAIN LAB must be the box and the brain
    /// the player left. The positive control: in the satchel seat the brain
    /// really is the hand-built one, so "back to standard" is a change the
    /// test can see.
    #[test]
    fn a_trip_through_two_seats_comes_home_unchanged() {
        let bed = scene::LabBox { founders: 0, colonies: 0, ..scene::LabBox::default() };
        let mut lab = Lab::new(bed.clone());
        lab.pick_ant(0);
        let colour = lab.renderer.creature_colour;
        let (ant, seed) = (lab.ant_name(), lab.spec.seed);
        assert!(lab.new_ant, "test setup: the standard pool is a trained ant");

        lab.play_demo(Some(DemoSeat::Queen));
        assert_eq!(playing(&lab), Some(DemoSeat::Queen));
        assert!(lab.scenario.as_ref().is_some_and(|s| s.name == "demo_queen"), "the queen's box did not load");

        lab.play_demo(Some(DemoSeat::Satchel));
        assert_eq!(playing(&lab), Some(DemoSeat::Satchel));
        assert!(lab.scenario.as_ref().is_some_and(|s| s.name == "demo_satchel"));
        assert!(!lab.new_ant, "control: the satchel seat should run the hand-built ant");
        assert!(lab.demo_home.is_some(), "switching seats forgot home");

        let said = lab.play_demo(None);
        assert!(said.starts_with("PLAIN LAB"), "{said}");
        assert_eq!(playing(&lab), None);
        assert!(lab.scenario.is_none(), "the plain lab came back with a seat's scenario");
        assert!(lab.demo_home.is_none());
        assert_eq!(lab.spec.seed, seed);
        assert_eq!(lab.ant_name(), ant, "the seat's brain outlived the seat");
        assert_eq!(lab.renderer.creature_colour, colour, "the queen's colour setting outlived her");
        assert!(lab.world.steered.is_none(), "a body is still bound in the plain lab");
        assert!(!lab.demo_refuses_world(), "the plain lab still refuses its tools");
    }

    /// **A seat restarting itself never rewrites home.** `start_demo_with`
    /// from inside a demo (the satchel's next round) must leave home as
    /// the plain lab it was, so PLAIN LAB still goes back there.
    #[test]
    fn a_seat_restarting_itself_keeps_home() {
        let mut lab = Lab::new(scene::LabBox { founders: 0, colonies: 0, ..scene::LabBox::default() });
        lab.pick_ant(0);
        lab.play_demo(Some(DemoSeat::Satchel));
        lab.start_demo_with(DemoSeat::Satchel, Default::default());
        assert!(!lab.new_ant, "control: the restarted seat runs the hand-built ant");
        lab.play_demo(None);
        assert!(lab.new_ant, "home was overwritten by the seat's own restart");
    }
}
