//! **Play as one ant: the lab's queen mode** -- slice 1 of the game
//! (`Reports/play-as-ant-slice1-2026-10-11.md`; slice 0 built the body,
//! `sim::steer`). You are a founding queen; every other ant runs the
//! shipped rules.
//!
//! **On when the box was opened from a scenario with a
//! [`super::scenario::Placement::Queen`]** (`queen_start.ron`), and only
//! then: [`QueenMode::owns`] is set by `Lab::reset` from the scenario, not
//! from `World::steered`. A harness that marks a body itself (`queenprobe`,
//! the needs walk's tests) and sets her input directly keeps her input: the
//! mode's keys would otherwise overwrite it every tick with "nothing
//! pressed". Every other box pays one `bool` read per tick and per frame.
//!
//! What it adds to the lab, and nothing more:
//!
//! - **the keys** ([`Key`]): `WASD` steer, `E` eat, `Q` drop, `F` share, `G`
//!   dig, `B` bite, `L` lay, `T` lay the food trail. Each is a held level, and
//!   each press is also latched (`Steered::press`) so a tap shorter than her
//!   six-frame decision still lands once. While the mode is on these keys
//!   are hers, not the lab's (pan, food tool, fling, and so on); every other
//!   key and the mouse are the lab's as before;
//! - **the steering cap** ([`STEER_CAP`]): at 4x and below the keys drive
//!   her; above it her genome does (`Steered::input = None`), and the panel
//!   says so;
//! - **the camera**, pinned to her head and two zoom rungs in on her first
//!   frame;
//! - **the panel**, top right: her bank against her egg bar, her crop, what
//!   her jaws hold, her colony's brood and workers against the goal of
//!   twenty, which keys are down, and whether she is steered;
//! - **three chronicle lines**: when she landed, when the goal was met, and
//!   when she died, so a playtest's actions log carries them.
//!
//! **Not here yet, by the design's build order**: time dropping to 1x on
//! danger, touching a worker to read her, hints, and passing the player to
//! a worker when the queen dies (slice 2); the rival and the result card
//! (slice 3). When she dies the mode stays on with nobody to steer, and the
//! panel says so.

use crate::render;
use crate::sim::cell::OrganismId;
use crate::sim::organism::BroodStage;
use crate::sim::steer::SteerInput;
use crate::sim::world::World;

use super::{HEIGHT, WIDTH};

/// The fastest dial setting the keys still drive her at (the design guide's
/// decision, 2026-10-11: steer up to 4x, tune by play). Above it her genome
/// drives her.
pub const STEER_CAP: u32 = 4;

/// The first goal: twenty workers besides herself (owner, 2026-10-11).
pub const GOAL_WORKERS: usize = 20;

/// How often the colony census behind the panel and the goal is taken, in
/// frames. A census walks every organism, so it is not done per tick.
const CENSUS_EVERY: u64 = 60;

/// One of the queen's keys, as the binary maps them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Eat,
    Drop,
    Share,
    Dig,
    Bite,
    Lay,
    Trail,
}

/// Her colony as the panel shows it: adults besides her, and brood by stage.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Census {
    pub workers: usize,
    pub eggs: usize,
    pub larvae: usize,
    pub pupae: usize,
}

/// The mode's own state. `Default` is off and empty; [`super::Lab::reset`]
/// puts it back to that for every new box.
#[derive(Clone, Debug, Default)]
pub struct QueenMode {
    /// **Whether this box is a queen-mode box**: opened from a scenario that
    /// places a queen. Everything below is inert while it is `false`.
    pub owns: bool,
    /// The keys down now.
    held: Held,
    /// The frame her body was first seen, and the body.
    pub landed: Option<(u64, OrganismId)>,
    /// The frame the goal was first met.
    pub goal_met: Option<u64>,
    /// The frame she was first found dead.
    pub died: Option<u64>,
    /// The last census, and the frame it was taken on.
    pub census: Census,
    census_frame: Option<u64>,
    /// Whether the camera has been brought in on her yet.
    zoomed: bool,
    /// What the panel said last frame: a change forces a full redraw, or
    /// the old text stays painted where the dirty-rect skip did not repaint.
    last_lines: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Held {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    eat: bool,
    drop: bool,
    share: bool,
    dig: bool,
    bite: bool,
    lay: bool,
    trail: bool,
}

impl Held {
    fn set(&mut self, key: Key, down: bool) {
        match key {
            Key::Up => self.up = down,
            Key::Down => self.down = down,
            Key::Left => self.left = down,
            Key::Right => self.right = down,
            Key::Eat => self.eat = down,
            Key::Drop => self.drop = down,
            Key::Share => self.share = down,
            Key::Dig => self.dig = down,
            Key::Bite => self.bite = down,
            Key::Lay => self.lay = down,
            Key::Trail => self.trail = down,
        }
    }

    fn input(self) -> SteerInput {
        let dx = i8::from(self.right) - i8::from(self.left);
        let dy = i8::from(self.down) - i8::from(self.up);
        SteerInput {
            dir: (dx, dy).ne(&(0, 0)).then_some((dx, dy)),
            eat: self.eat,
            drop: self.drop,
            share: self.share,
            dig: self.dig,
            bite: self.bite,
            lay: self.lay,
            trail: self.trail,
        }
    }
}

/// What one key alone asks for: the latch for a single press.
fn press_of(key: Key) -> SteerInput {
    let mut held = Held::default();
    held.set(key, true);
    held.input()
}

impl QueenMode {
    /// The mode for a box opened from `scenario`: on if it places a queen.
    pub fn for_scenario(scenario: Option<&super::scenario::Scenario>) -> Self {
        QueenMode {
            owns: scenario.is_some_and(|s| s.places_queen()),
            ..QueenMode::default()
        }
    }

    /// **Is the box in queen mode?** A queen scenario, with her body marked.
    pub fn on(&self, world: &World) -> bool {
        self.owns && world.steered.is_some()
    }

    /// A key went down or up. Down also latches the press, so a tap between
    /// two of her decisions is used at the next one. Does nothing with
    /// nobody steered.
    pub fn key(&mut self, world: &mut World, key: Key, down: bool) {
        if !self.owns {
            return;
        }
        self.held.set(key, down);
        if down {
            if let Some(s) = world.steered.as_mut() {
                s.press(press_of(key));
            }
        }
    }

    /// **Hand the keys to her for this tick**, or to her genome above the
    /// cap. Called once a tick before the step. One `bool` read outside
    /// queen mode.
    pub fn feed(&self, world: &mut World, requested: u32) {
        if !self.owns {
            return;
        }
        let Some(s) = world.steered.as_mut() else { return };
        if requested <= STEER_CAP {
            s.input = Some(self.held.input());
        } else {
            s.input = None;
            s.latched = SteerInput::default();
        }
    }

    /// **After each tick**: note her landing, the goal and her death, and
    /// keep the census fresh. One `bool` read outside queen mode.
    pub fn observe(&mut self, world: &mut World) {
        if !self.owns {
            return;
        }
        let Some((id, colony)) = world.steered.as_ref().map(|s| (s.id, s.colony)) else {
            return;
        };
        let alive = crate::sim::steer::is_steered(world, id);
        if self.landed.is_none() && alive {
            self.landed = Some((world.frame, id));
            world.log_player_action(format!("QUEEN LANDED (ANT {id}, COLONY {colony})"));
        }
        if self.died.is_none() && self.landed.is_some() && !alive {
            self.died = Some(world.frame);
            world.log_player_action("YOUR QUEEN DIED".to_string());
        }
        if self.census_frame.is_none_or(|f| world.frame >= f + CENSUS_EVERY) {
            self.census = census(world, colony, id);
            self.census_frame = Some(world.frame);
            if self.goal_met.is_none() && self.census.workers >= GOAL_WORKERS {
                self.goal_met = Some(world.frame);
                world.log_player_action(format!("GOAL MET: {GOAL_WORKERS} WORKERS"));
            }
        }
    }

    /// **Keep the camera on her**: two rungs in on the first frame she is
    /// seen, then the renderer's dead-zone follow (the one `Lab::follow_pin`
    /// uses, for its reason: a strictly centred view repaints every frame).
    /// Returns `true` on that first frame, so the caller can clear the
    /// screen for her (the biosphere page opens on every fresh lab, and it
    /// covers the panel).
    pub fn camera(&mut self, renderer: &mut render::Renderer, world: &World) -> bool {
        if !self.owns {
            return false;
        }
        let Some(head) = head_of(world) else { return false };
        let bounds = world.bounds();
        let first = !self.zoomed;
        if first {
            self.zoomed = true;
            // One rung a call, whatever the delta (`Renderer::adjust_zoom`):
            // two rungs from the lab's opening view is x3, which keeps the
            // food spot 60 columns off in view.
            for _ in 0..2 {
                renderer.zoom_within(1, (WIDTH, HEIGHT), bounds);
            }
        }
        renderer.follow(head, (WIDTH, HEIGHT), bounds);
        first
    }

    /// The panel's lines, and whether they changed since the last frame
    /// (so the caller can force a full redraw). Empty with nobody steered.
    pub fn lines(&mut self, world: &World, requested: u32) -> (Vec<String>, bool) {
        let lines = if self.on(world) {
            panel_lines(self, world, requested)
        } else {
            Vec::new()
        };
        let changed = lines != self.last_lines;
        if changed {
            self.last_lines.clone_from(&lines);
        }
        (lines, changed)
    }

    /// Draw the panel, top right, over a dark backing so it reads on sky and
    /// soil alike.
    pub fn draw(&self, hc: render::Hud, frame: &mut [u8], lines: &[String]) {
        if lines.is_empty() {
            return;
        }
        let w = lines.iter().map(|l| crate::hud::text_width(l)).max().unwrap_or(0) + 8;
        let h = lines.len() as i32 * LINE + 6;
        let x0 = WIDTH as i32 - w - 4;
        let y0 = 4;
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                hc.blend(frame, x, y, [12, 10, 16, 255], 0.78);
            }
        }
        for (i, line) in lines.iter().enumerate() {
            let colour = if i == 0 {
                TITLE
            } else if line.starts_with("KEYS") {
                KEYS
            } else {
                TEXT
            };
            hc.text(frame, x0 + 4, y0 + 4 + i as i32 * LINE, line, colour);
        }
    }
}

const LINE: i32 = 9;
const TITLE: [u8; 4] = [255, 30, 230, 255];
const TEXT: [u8; 4] = [225, 225, 230, 255];
const KEYS: [u8; 4] = [150, 150, 165, 255];

/// Her head, while she lives.
fn head_of(world: &World) -> Option<(i32, i32)> {
    let s = world.steered.as_ref()?;
    if !crate::sim::steer::is_steered(world, s.id) {
        return None;
    }
    world.organism(s.id)?.chain.first().copied()
}

/// Her colony: adults of her species besides her, and its brood by stage.
pub fn census(world: &World, colony: u32, queen: OrganismId) -> Census {
    let species = world.organism(queen).map(|s| s.species);
    let mut c = Census::default();
    for id in world.live_organism_ids() {
        if id != queen
            && world
                .organism(id)
                .is_some_and(|s| s.colony == colony && Some(s.species) == species)
        {
            c.workers += 1;
        }
    }
    for id in world.live_brood_ids() {
        let Some(s) = world.organism(id) else { continue };
        if s.colony != colony {
            continue;
        }
        match s.brood.map(|b| b.stage) {
            Some(BroodStage::Egg) => c.eggs += 1,
            Some(BroodStage::Larva) => c.larvae += 1,
            Some(BroodStage::Pupa) => c.pupae += 1,
            None => {}
        }
    }
    c
}

fn panel_lines(mode: &QueenMode, world: &World, requested: u32) -> Vec<String> {
    let mut out = vec!["YOUR QUEEN".to_string()];
    let s = world.steered.as_ref().expect("on() checked");
    let body = crate::sim::steer::is_steered(world, s.id)
        .then(|| world.organism(s.id))
        .flatten();
    match body {
        Some(state) => {
            let def = world.species.get(state.species).creature.as_ref();
            let bar = def.and_then(|d| crate::sim::creature::egg_bar_of(world, d, state));
            out.push(match bar {
                Some(bar) if state.energy >= bar => format!("BANK {:.0} J  EGG READY: PRESS L AT HOME", state.energy),
                Some(bar) => format!("BANK {:.0} J  NEXT EGG AT {bar:.0} J", state.energy),
                None => format!("BANK {:.0} J", state.energy),
            });
            let capacity = def.map_or(0.0, |d| d.crop_capacity);
            out.push(match state.crop {
                Some(c) if c.cells > 0 => {
                    let pct = if capacity > 0.0 {
                        100.0 * c.worth() / capacity
                    } else {
                        0.0
                    };
                    format!("CROP {} BITES ({pct:.0}% FULL)", c.cells)
                }
                _ => "CROP EMPTY".to_string(),
            });
            out.push(match state.spoil {
                Some(sp) if sp.store => "JAWS: FOOD FOR THE STORE".to_string(),
                Some(_) => "JAWS: A PELLET OF SOIL (Q DROPS IT)".to_string(),
                None => "JAWS: EMPTY".to_string(),
            });
        }
        None => out.push(match mode.died {
            Some(f) => format!("DIED AT FRAME {f}: NOBODY TO STEER"),
            None => "NOT LANDED YET".to_string(),
        }),
    }
    let c = mode.census;
    out.push(format!("EGGS {}  LARVAE {}  PUPAE {}", c.eggs, c.larvae, c.pupae));
    out.push(match mode.goal_met {
        Some(f) => format!("WORKERS {}  GOAL OF {GOAL_WORKERS} MET AT FRAME {f}", c.workers),
        None => format!("WORKERS {} OF {GOAL_WORKERS}", c.workers),
    });
    out.push(if body.is_none() {
        "THE COLONY RUNS ON ITS OWN".to_string()
    } else if requested <= STEER_CAP {
        format!("STEERING AT {requested}X")
    } else {
        format!("ABOVE {STEER_CAP}X HER GENOME DRIVES HER")
    });
    let k = mode.held;
    let mark = |on: bool, name: &str| if on { format!("[{name}]") } else { name.to_string() };
    out.push(format!(
        "KEYS {} {} {} {}",
        mark(k.up || k.down || k.left || k.right, "WASD STEER"),
        mark(k.eat, "E EAT"),
        mark(k.drop, "Q DROP"),
        mark(k.share, "F SHARE"),
    ));
    out.push(format!(
        "KEYS {} {} {} {}",
        mark(k.dig, "G DIG"),
        mark(k.bite, "B BITE"),
        mark(k.lay, "L LAY"),
        mark(k.trail, "T TRAIL"),
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::scenario::{Placement, Scenario};
    use crate::lab::Lab;

    /// The shipped scenario, read from the file the binary loads but without
    /// `Scenario::load`'s directory lookup, which other tests point at
    /// scratch directories while this runs.
    fn queen_lab() -> Lab {
        let sc: Scenario =
            ron::from_str(include_str!("../../assets/lab_scenarios/queen_start.ron")).expect("queen_start.ron parses");
        let mut lab = Lab::new(sc.bed.clone());
        lab.load_scenario(sc);
        lab
    }

    fn head(lab: &Lab) -> (i32, i32) {
        head_of(&lab.world).expect("she lives")
    }

    fn run(lab: &mut Lab, frames: u32) {
        for _ in 0..frames {
            lab.tick_for_harness();
        }
    }

    /// **The scenario puts one queen in the box, marked and granted, and
    /// the box is in queen mode.** Her bank is the ant's start plus the
    /// 900 J grant, booked so the ledger closes; she is the only ant.
    #[test]
    fn queen_start_lands_one_steered_queen_with_her_grant() {
        let mut lab = queen_lab();
        assert!(lab.queen_mode(), "a Queen placement must put the box in queen mode");
        let s = lab.world.steered.clone().expect("steered");
        let state = lab.world.organism(s.id).expect("she lives");
        let ant = lab.world.species.id_of("ant").unwrap();
        let start = lab.world.species.get(ant).creature.as_ref().unwrap().start_energy;
        assert!(
            (state.energy - (start + 900.0)).abs() < 1.0,
            "bank {} vs start {start} + 900",
            state.energy
        );
        assert_eq!(state.colony, s.colony);
        let ants = lab
            .world
            .live_organism_ids()
            .into_iter()
            .filter(|&id| lab.world.organism(id).is_some_and(|o| o.species == ant))
            .count();
        assert_eq!(ants, 1, "a colony of one");
        run(&mut lab, 1);
        assert_eq!(
            lab.queen.landed.map(|(_, id)| id),
            Some(s.id),
            "her landing is noted on the first tick"
        );
    }

    /// **A box without the placement is not in queen mode, and the hooks do
    /// nothing in it.** The off path: `feed` and `observe` read one `Option`.
    #[test]
    fn a_box_without_a_queen_is_not_in_queen_mode() {
        let mut lab = Lab::new(super::super::scene::LabBox::default());
        assert!(!lab.queen_mode());
        lab.queen_key(Key::Right, true);
        run(&mut lab, 30);
        assert!(lab.world.steered.is_none());
        assert_eq!(lab.queen.landed, None);
    }

    /// **A harness that steers an ant itself keeps her.** Outside a queen
    /// scenario the mode must not hand her "nothing pressed" every tick:
    /// that broke the needs walk's steered test and would have frozen
    /// `queenprobe`'s scripted player, which both set `input` directly.
    #[test]
    fn a_harness_steered_ant_outside_queen_mode_keeps_its_input() {
        let mut lab = Lab::new(super::super::scene::LabBox::default());
        let spec = lab.spec.clone();
        let colony = [Placement::Colony { species: "ant".into(), x: 256, count: 1 }];
        crate::lab::scenario::apply_placements(&mut lab.world, &colony, &spec);
        let ant = lab.world.species.id_of("ant").unwrap();
        let id = lab.world.live_organism_ids().into_iter().find(|&i| lab.world.organism(i).is_some_and(|s| s.species == ant)).expect("an ant");
        let held = SteerInput { dir: Some((1, 0)), ..Default::default() };
        assert!(crate::sim::steer::mark(&mut lab.world, id, Some(held)));
        assert!(!lab.queen_mode(), "a hand-marked ant is not queen mode");
        run(&mut lab, 10);
        assert_eq!(lab.world.steered.as_ref().unwrap().input, Some(held));
    }

    /// **The keys drive her at 4x and below, and her genome above.** Held
    /// keys arrive as levels each tick; above the cap the input is `None`
    /// and the latch is dropped, so nothing pressed at 16x fires on the way
    /// back down. Then the feel: holding W and G she cuts her way out of the
    /// founding chamber, so her head moves.
    #[test]
    fn the_keys_drive_her_at_4x_and_her_genome_above() {
        let mut lab = queen_lab();
        lab.time.requested = 1;
        lab.queen_key(Key::Up, true);
        lab.queen_key(Key::Dig, true);
        run(&mut lab, 1);
        let input = lab.world.steered.as_ref().unwrap().input;
        assert_eq!(input.map(|i| (i.dir, i.dig)), Some((Some((0, -1)), true)));
        let from = head(&lab);
        run(&mut lab, 900);
        assert_ne!(head(&lab), from, "W and G held for 900 frames, and she never moved");
        lab.time.requested = 16;
        lab.queen_key(Key::Lay, true);
        run(&mut lab, 1);
        let s = lab.world.steered.as_ref().unwrap();
        assert_eq!(s.input, None, "above {STEER_CAP}x her genome drives her");
        assert_eq!(
            s.latched,
            SteerInput::default(),
            "a press above the cap is not saved for later"
        );
        lab.time.requested = 4;
        run(&mut lab, 1);
        assert!(
            lab.world.steered.as_ref().unwrap().input.is_some(),
            "at {STEER_CAP}x the keys are hers again"
        );
    }

    /// **A tap between two of her decisions is still used once.** The press
    /// is latched on the key-down, the key comes up before she decides, and
    /// her next decision uses it and clears the latch.
    #[test]
    fn a_tap_shorter_than_a_decision_is_used_once() {
        let mut lab = queen_lab();
        lab.time.requested = 1;
        run(&mut lab, 3);
        lab.queen_key(Key::Lay, true);
        lab.queen_key(Key::Lay, false);
        assert!(lab.world.steered.as_ref().unwrap().latched.lay, "the press is latched");
        let mut used = false;
        for _ in 0..12 {
            run(&mut lab, 1);
            used |= lab.world.steered.as_ref().unwrap().used.lay;
        }
        assert!(used, "her next decision used the tap");
        assert!(!lab.world.steered.as_ref().unwrap().latched.lay, "and spent it");
    }

    /// **The panel tells the player what they need**: her bank against the
    /// egg bar, her colony against the goal, and who is driving.
    #[test]
    fn the_panel_names_her_bank_her_colony_and_who_drives() {
        let mut lab = queen_lab();
        run(&mut lab, 2);
        let (lines, changed) = lab.queen.lines(&lab.world, 1);
        assert!(changed, "the first panel is a change");
        let text = lines.join("\n");
        for want in [
            "YOUR QUEEN",
            "BANK",
            "J",
            "CROP",
            "WORKERS 0 OF 20",
            "STEERING AT 1X",
            "WASD STEER",
        ] {
            assert!(text.contains(want), "panel lacks {want:?}:\n{text}");
        }
        let (_, again) = lab.queen.lines(&lab.world, 1);
        assert!(!again, "an unchanged panel does not force a redraw");
        let (lines, _) = lab.queen.lines(&lab.world, 64);
        assert!(lines.join(" ").contains("HER GENOME DRIVES HER"));
    }

    /// **The food spot stops at its cap.** Fired again and again with no one
    /// eating, a `Larder` holds at `cap` cells where a `Heap` would grow for
    /// ever and bury a lone queen.
    #[test]
    fn a_larder_tops_up_to_its_cap_and_no_further() {
        let mut lab = queen_lab();
        let food = lab.world.materials.id_of("provisions").unwrap();
        let count = |w: &World| {
            let mut n = 0;
            for x in 316 - crate::lab::scenario::LARDER_REACH..=316 + crate::lab::scenario::LARDER_REACH {
                for y in 0..320 {
                    n += usize::from(w.get(x, y).material == food);
                }
            }
            n
        };
        let p = [Placement::Larder {
            material: "provisions".into(),
            x: 316,
            cells: 20,
            cap: 60,
        }];
        for _ in 0..10 {
            crate::lab::scenario::apply_placements(&mut lab.world, &p, &lab.spec);
        }
        assert_eq!(count(&lab.world), 60, "ten top-ups of 20 against a cap of 60");
    }

    #[test]
    fn held_keys_make_the_steer_and_a_press_alone_names_its_verb() {
        let mut h = Held::default();
        assert_eq!(h.input(), SteerInput::default());
        h.set(Key::Right, true);
        h.set(Key::Up, true);
        assert_eq!(h.input().dir, Some((1, -1)));
        h.set(Key::Left, true);
        assert_eq!(h.input().dir, Some((0, -1)), "left and right cancel");
        assert_eq!(
            press_of(Key::Lay),
            SteerInput {
                lay: true,
                ..Default::default()
            }
        );
        assert_eq!(press_of(Key::Down).dir, Some((0, 1)));
    }
}
