//! **The seat demos' common launcher** (2026-10-25).
//!
//! The owner asked for one playable demo per *seat* -- each a different
//! answer to "who is the player, and how do they teach the colony?"
//! (`Reports/teaching-the-colony-2026-10-25.md` and its folder of seat
//! designs). Six threads build them at once, so they share one way in:
//!
//! ```text
//! cargo run --release --bin lab -- demo=queen      (or PIXEL_PHYSICS_LAB_DEMO=queen)
//! ```
//!
//! `demo=<seat>` does two things and nothing else:
//!
//! 1. loads the lab scenario `assets/lab_scenarios/demo_<seat>.ron` through
//!    the ordinary `scenario=` path, so a demo is a saved box with a
//!    question written on it like any other, and gets the lab's clock,
//!    pages, overlays and screenshot hooks for free;
//! 2. sets [`Lab::demo`], which this module reads to route keys to a body the
//!    player drives (`sim::steer`) and to let each seat's own file add its
//!    verbs.
//!
//! **Off is off.** With no `demo=`, `Lab::demo` is `None` and every hook
//! below is one branch not taken: the lab is the lab.
//!
//! **Each seat owns its own file** (`demo/<seat>.rs`) and its own scenario;
//! the only shared edit is one arm in each `match` below. The board that
//! records who owns which shared piece is the project's
//! `seat-demos/shared/launcher.md`.
//!
//! # The body key layer
//!
//! With a body marked, [`Lab::demo_key`] reads keys before the lab does
//! (`bin/lab.rs` maps winit's codes to [`DemoKey`], so this module never
//! sees winit -- the same seam the rest of `lab` keeps):
//!
//! - **arrows steer** (the speed dial moves to `PageUp`/`PageDown`; the
//!   digits still pick a preset); `WASD` stays the pan;
//! - slice 0's verbs keep their letters: `E` eat, `Q` drop, `F` share,
//!   `G` dig, `B` bite, `L` lay, `T` (held) trail;
//! - **`Y U P J K R` are the seat's own keys** (only one demo runs at a time,
//!   so seats may reuse them); `R` never rebuilds the box in a demo;
//! - `H` the seat card (`card`: goal, score, how you lose, every key),
//!   `Tab` camera-follow on and off.
//!
//! Above [`STEER_CAP`] the keyboard lets go and her genome drives her, the
//! state slice 0 measured her in: a 64x body cannot be steered by a hand.

pub mod card;
pub mod keeper;
pub mod guest;
pub mod enemy;
pub mod queen;
pub mod scout;

use crate::sim::cell::OrganismId;
use crate::sim::steer::{self, SteerInput, SteerRole};

use super::Lab;

/// The speed multiple above which the keyboard lets go of the body and her
/// genome drives her (`Steered::input` is `None`). Slice 0's "above 4x".
pub const STEER_CAP: u32 = 4;

/// Which seat's demo is running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemoSeat {
    Queen,
    Keeper,
    Guest,
    Mind,
    Enemy,
    Scout,
}

impl DemoSeat {
    pub const ALL: [DemoSeat; 6] = [DemoSeat::Queen, DemoSeat::Keeper, DemoSeat::Guest, DemoSeat::Mind, DemoSeat::Enemy, DemoSeat::Scout];

    /// The name typed after `demo=`, and the stem of its scenario.
    pub fn name(self) -> &'static str {
        match self {
            DemoSeat::Queen => "queen",
            DemoSeat::Keeper => "keeper",
            DemoSeat::Guest => "guest",
            DemoSeat::Mind => "mind",
            DemoSeat::Enemy => "enemy",
            DemoSeat::Scout => "scout",
        }
    }

    pub fn parse(s: &str) -> Option<DemoSeat> {
        let s = s.trim().to_ascii_lowercase();
        DemoSeat::ALL.into_iter().find(|d| d.name() == s || (s == "colony-mind" && *d == DemoSeat::Mind))
    }

    /// `demo_<seat>`: the scenario file this demo opens on.
    pub fn scenario_name(self) -> String {
        format!("demo_{}", self.name())
    }

    /// The body the player wears in this seat, if any: the queen seat and the
    /// scout seat drive an ant; the others bring their own (or none).
    pub fn body(self) -> Option<SteerRole> {
        match self {
            DemoSeat::Queen => Some(SteerRole::Queen),
            DemoSeat::Scout => Some(SteerRole::Worker),
            // The guest seat's body is the rove it lands itself
            // (`guest::pick_body`), never an ant.
            DemoSeat::Guest => Some(SteerRole::Worker),
            // The enemy seat's centipede (`enemy`): no queen rule, and it is
            // bound only once it lands, after the warm-up.
            DemoSeat::Enemy => Some(SteerRole::Worker),
            _ => None,
        }
    }
}

/// A key as the demo layer sees it: the few the layer reads, already mapped
/// from winit by `bin/lab.rs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemoKey {
    Left,
    Right,
    Up,
    Down,
    Tab,
    /// Shift, held: the enemy seat's sprint.
    Sprint,
    /// An upper-case ASCII letter.
    Letter(char),
}

/// The keys held for the body, as levels.
#[derive(Clone, Copy, Debug, Default)]
pub struct Held {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    trail: bool,
}

impl Held {
    fn dir(&self) -> Option<(i8, i8)> {
        let x = self.right as i8 - self.left as i8;
        let y = self.down as i8 - self.up as i8;
        (x != 0 || y != 0).then_some((x, y))
    }
}

/// What the pointer did over the world, for a seat that claims it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseAct {
    /// A button went down (`erase`: the right one).
    Press { erase: bool },
    /// Moved with the button still down, after a press the seat claimed.
    Drag,
    /// The button came up, after a press the seat claimed.
    Release,
}

/// The running demo: the seat, what is held, and the seat's own state.
///
/// **Each seat keeps its own state in its own field** (`queen`, and one line
/// per seat that needs one), so no seat's code reaches into another's or into
/// `Lab`'s own fields.
#[derive(Clone, Debug)]
pub struct Demo {
    pub seat: DemoSeat,
    pub held: Held,
    /// Camera follows the body (`Tab`).
    pub follow: bool,
    /// Every `key=value` on the command line, for the seat to read its own
    /// (`arm=`, `sealed=` ...). Unknown keys are the seat's business; the
    /// lab ignores what it does not read, as it always has.
    pub args: std::collections::BTreeMap<String, String>,
    /// A press over the world went to the seat, so its drag and release do too.
    pub mouse_held: bool,
    /// The queen seat's own state (None in every other seat).
    pub queen: Option<queen::QueenDemo>,
    /// The keeper seat's own state (None in every other seat).
    pub keeper: Option<keeper::KeeperDemo>,
    /// The scout seat's own state (set by `start_demo_with`, which has the args).
    pub scout: Option<scout::ScoutDemo>,
    /// The guest seat's own state (None in every other seat).
    pub guest: Option<guest::GuestDemo>,
    /// The enemy seat's own state.
    pub enemy: Option<enemy::EnemyDemo>,
    /// The seat card is up ([`card`]). Opened by the window at start
    /// (`Lab::open_seat_card`), never by a headless harness, whose keys must
    /// reach the seat rather than close a card nobody can see.
    pub card: bool,
    /// `H` paused a running clock to show the card, so closing it resumes.
    pub card_resume: bool,
    /// The clock's multiple last frame, so the card can tell a seat's
    /// warm-up ending (the dial dropping to play speed) while it is up.
    pub card_seen_multiple: u32,
}

impl Demo {
    pub fn new(seat: DemoSeat) -> Demo {
        Demo {
            seat,
            held: Held::default(),
            follow: true,
            args: Default::default(),
            mouse_held: false,
            queen: (seat == DemoSeat::Queen).then(queen::QueenDemo::default),
            keeper: (seat == DemoSeat::Keeper).then(keeper::KeeperDemo::default),
            scout: None,
            guest: (seat == DemoSeat::Guest).then(guest::GuestDemo::default),
            enemy: (seat == DemoSeat::Enemy).then(enemy::EnemyDemo::default),
            card: false,
            card_resume: false,
            card_seen_multiple: 0,
        }
    }
}

/// **Drawn over the world**, in world cells on the box's own buffer, after the
/// lab's own marks (`Lab::paint`). A seat that draws here must also say so in
/// [`marks_world`], or the dirty-rect skip leaves last frame's marks behind.
pub fn draw_world(lab: &Lab, frame_buf: &mut [u8]) {
    let Some(demo) = lab.demo.as_ref() else { return };
    match demo.seat {
        DemoSeat::Queen => queen::draw_world(lab, frame_buf),
        DemoSeat::Keeper => keeper::draw_world(lab, frame_buf),
        DemoSeat::Scout => scout::draw_world(lab, frame_buf),
        DemoSeat::Guest => guest::draw_world(lab, frame_buf),
        DemoSeat::Enemy => enemy::draw_world(lab, frame_buf),
        _ => {}
    }
}

/// Whether the seat drew anything over the world this frame ([`draw_world`]).
pub fn marks_world(lab: &Lab) -> bool {
    match lab.demo.as_ref().map(|d| d.seat) {
        Some(DemoSeat::Queen) => queen::marks_world(lab),
        Some(DemoSeat::Keeper) => keeper::marks_world(lab),
        Some(DemoSeat::Scout) => scout::marks_world(lab),
        Some(DemoSeat::Guest) => guest::marks_world(lab),
        Some(DemoSeat::Enemy) => enemy::marks_world(lab),
        _ => false,
    }
}

/// **The seat's panel**, on the interface canvas (the 2x layer when there is
/// one), after the clock. `hc.text` is the lab's own text.
pub fn draw_hud(lab: &Lab, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    let Some(demo) = lab.demo.as_ref() else { return };
    match demo.seat {
        DemoSeat::Queen => queen::draw_hud(lab, hc, frame_buf),
        DemoSeat::Keeper => keeper::draw_hud(lab, hc, frame_buf),
        DemoSeat::Scout => scout::draw_hud(lab, hc, frame_buf),
        DemoSeat::Guest => guest::draw_hud(lab, hc, frame_buf),
        DemoSeat::Enemy => enemy::draw_hud(lab, hc, frame_buf),
        _ => {}
    }
}

/// **The seat card and its reminder** ([`card`]), drawn after the lab's own
/// chrome (`Lab::paint`) so the card covers the cell readout and the seat's
/// panel rather than sitting under them.
pub fn draw_card(lab: &Lab, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    card::draw_reminder(lab, hc, frame_buf);
    card::draw(lab, hc, frame_buf);
}

/// **Whether a seat routes a body letter itself** (`E Q F G B L T`) rather
/// than handing it to the steered body. A seat that does adds its arm here
/// and routes the letter in `demo_key` beside `queen::seat_key`.
///
/// The guest seat takes `B` (kill a larva) and `E` (eat a corpse): the
/// rove's own bite and eat would kill a worker, which is not a beggar's verb.
pub fn seat_takes(seat: DemoSeat, c: char) -> bool {
    match seat {
        DemoSeat::Guest => guest::takes(c),
        _ => false,
    }
}

/// The seat's name as the player knows it (the enemy seat is the raider).
pub fn seat_title(seat: DemoSeat) -> &'static str {
    match seat {
        DemoSeat::Queen => "QUEEN",
        DemoSeat::Keeper => "KEEPER",
        DemoSeat::Guest => "GUEST",
        DemoSeat::Mind => "COLONY MIND",
        DemoSeat::Enemy => "RAIDER",
        DemoSeat::Scout => "SCOUT",
    }
}

/// The one-line key list each seat used to show on `H` and at start. The
/// ticker clips it at about 72 characters, so the window now shows the seat
/// card ([`card`]) instead; this stays for the log line a headless run
/// prints.
pub fn help_line(seat: DemoSeat) -> String {
    let body = "ARROWS STEER -- E EAT, Q DROP, F FEED, G DIG, B BITE, L LAY, T TRAIL -- TAB FOLLOW -- PGUP/PGDN SPEED";
    match seat {
        DemoSeat::Queen => format!("QUEEN DEMO: {body} -- {}", queen::SEAT_KEYS),
        DemoSeat::Keeper => format!("KEEPER DEMO: {} -- H HELP -- WASD PAN, THE LAB'S TOOLS WORK WHEN THE HAND IS NONE", keeper::SEAT_KEYS),
        DemoSeat::Scout => format!("SCOUT DEMO: {body} -- {}", scout::SEAT_KEYS),
        DemoSeat::Guest => format!("GUEST DEMO: ARROWS MOVE -- {} -- TAB FOLLOW -- PGUP/PGDN SPEED", guest::SEAT_KEYS),
        DemoSeat::Enemy => format!(
            "ENEMY DEMO: YOU ARE A CENTIPEDE -- ARROWS CREEP, SHIFT SPRINT, B BITE, G CUT/SHOVE, E EAT, Q SPIT OR DROP -- TAB FOLLOW -- PGUP/PGDN SPEED -- {} -- THE COLONY FIGHTS BACK BUT DOES NOT LEARN YOU",
            enemy::SEAT_KEYS
        ),
        s if s.body().is_some() => format!("{} DEMO: {body}", s.name().to_uppercase()),
        s => format!("{} DEMO", s.name().to_uppercase()),
    }
}

impl Lab {
    /// **Start a seat's demo**: load its scenario (if one ships) and arm the
    /// layer. Returns the message for the notice line.
    pub fn start_demo(&mut self, seat: DemoSeat) -> String {
        self.start_demo_with(seat, Default::default())
    }

    /// [`Lab::start_demo`] with the command line's `key=value` pairs, set
    /// before the scenario loads so a seat can read them while it builds.
    pub fn start_demo_with(&mut self, seat: DemoSeat, args: std::collections::BTreeMap<String, String>) -> String {
        let mut demo = Demo::new(seat);
        if seat == DemoSeat::Keeper {
            demo.keeper = Some(keeper::KeeperDemo::from_args(&args));
        }
        demo.args = args;
        self.demo = Some(demo);
        // `seed=N` on the command line reseeds the demo's box.
        let seed = self.demo.as_ref().and_then(|d| d.args.get("seed")).and_then(|v| v.parse::<u64>().ok());
        // The ticker clips at about 72 characters: the window's line is the
        // seat and the two keys that matter, and the card says the rest. The
        // scenario's own load line went to the log only.
        let msg = match super::scenario::Scenario::load(&seat.scenario_name()) {
            Ok(mut s) => {
                if let Some(sd) = seed {
                    s.bed.seed = sd;
                }
                let loaded = self.load_scenario(s);
                eprintln!("demo {}: {loaded} -- {}", seat.name(), help_line(seat));
                format!("{} DEMO -- SPACE TO PLAY, H FOR THE GOAL AND KEYS", seat_title(seat))
            }
            Err(e) => format!("NO SCENARIO {} YET ({e}) -- OPENING ON THE SAVED BOX", seat.scenario_name().to_uppercase()),
        };
        if seat == DemoSeat::Keeper {
            keeper::on_start(self);
        }
        if seat == DemoSeat::Scout {
            let seed = self.spec.seed;
            if let Some(d) = self.demo.as_mut() {
                d.scout = Some(scout::ScoutDemo::from_args(&d.args, seed));
            }
        }
        self.demo_bind_body();
        // The lab's welcome window would cover the box: a demo opens on its
        // own seat card instead (`H` shows it again).
        self.show_help = false;
        msg
    }

    /// **Put the seat card up.** With `pause`, a running clock stops under
    /// it and resumes when it closes (`H` mid-game). Without, the clock is
    /// left alone: at start a demo is either paused already or in a seat's
    /// warm-up (the raider, the scout, the guest run the colony at the top
    /// of the dial before the player lands), and the warm-up is exactly
    /// when there is time to read. If the warm-up ends with the card still
    /// up, [`Lab::demo_frame`] pauses then, so the game waits for the reader.
    pub fn open_seat_card(&mut self, pause: bool) {
        let multiple = self.time.multiple();
        let Some(d) = self.demo.as_mut() else { return };
        d.card = true;
        d.card_seen_multiple = multiple;
        d.card_resume = pause && multiple > 0;
        if d.card_resume {
            self.time.pause();
        }
        self.touch_chrome();
    }

    /// **`Space` while the card is up means "play"**: the card closes and a
    /// paused clock runs. A running one (a warm-up) is left running, where
    /// the lab's own `Space` would have paused it. `false` when no card is
    /// up and `Space` is the lab's.
    pub fn demo_space(&mut self) -> bool {
        if !self.demo.as_ref().is_some_and(|d| d.card) {
            return false;
        }
        if let Some(d) = self.demo.as_mut() {
            d.card_resume = false;
        }
        self.close_seat_card();
        if self.time.multiple() == 0 {
            self.time.toggle_phase();
        }
        true
    }

    /// Take the seat card down; resume the clock if opening it paused it.
    pub fn close_seat_card(&mut self) {
        let Some(d) = self.demo.as_mut() else { return };
        if !d.card {
            return;
        }
        d.card = false;
        if std::mem::take(&mut d.card_resume) && self.time.multiple() == 0 {
            self.time.toggle_phase();
        }
        self.touch_chrome();
    }

    /// Mark the seat's body if it wants one and none is marked. Called after
    /// every rebuild and each frame, because a scenario may land its colony
    /// on a timeline rather than at frame 0.
    pub fn demo_bind_body(&mut self) {
        let Some(demo) = self.demo.as_ref() else { return };
        let Some(role) = demo.seat.body() else { return };
        if steer::steered_id(&self.world).is_some() {
            return;
        }
        // A dead body stays dead: slice 0 keeps `World::steered` set with her
        // dead id so her colony keeps its rule, and a demo shows the death
        // rather than quietly handing the player a new ant.
        if self.world.steered.is_some() {
            return;
        }
        let pick = match demo.seat {
            DemoSeat::Queen => queen::pick_body(&self.world),
            // No body until the warm-up hands one over (`scout::HANDOVER`). A layer armed without the seat's
            // state (no `start_demo_with`) binds at once, as every other seat does.
            DemoSeat::Scout if demo.scout.as_ref().is_some_and(|s| !s.started) => None,
            DemoSeat::Scout => scout::pick_body(&self.world).or_else(|| oldest_ant(&self.world)),
            DemoSeat::Guest => guest::pick_body(&self.world),
            DemoSeat::Enemy => return,
            _ => oldest_ant(&self.world),
        };
        if let Some(id) = pick {
            steer::mark_with(&mut self.world, id, Some(SteerInput::default()), role);
            if self.world.steered.is_some() {
                if let Some(s) = self.world.steered.as_mut() {
                    s.log = Some(Vec::new());
                }
            }
        }
    }

    /// **Before each simulated tick** (`Lab::tick`), so a seat's verbs act
    /// per tick at any speed, and a headless harness runs the same sequence
    /// as the window.
    pub fn demo_before_tick(&mut self) {
        match self.demo.as_ref().map(|d| d.seat) {
            Some(DemoSeat::Queen) => queen::before_tick(self),
            Some(DemoSeat::Keeper) => keeper::before_tick(self),
            Some(DemoSeat::Scout) => scout::before_tick(self),
            Some(DemoSeat::Guest) => guest::before_tick(self),
            Some(DemoSeat::Enemy) => enemy::before_tick(self),
            _ => {}
        }
    }

    /// **After each simulated tick**: counters, notices.
    pub fn demo_after_tick(&mut self) {
        match self.demo.as_ref().map(|d| d.seat) {
            Some(DemoSeat::Queen) => queen::after_tick(self),
            Some(DemoSeat::Keeper) => keeper::after_tick(self),
            Some(DemoSeat::Scout) => scout::after_tick(self),
            Some(DemoSeat::Guest) => guest::after_tick(self),
            Some(DemoSeat::Enemy) => enemy::after_tick(self),
            _ => {}
        }
    }

    /// **The pointer over the world**, offered to the seat before the lab's
    /// own tool. `true` when the seat took it: the press's drag and release
    /// then go to the seat too.
    pub fn demo_mouse(&mut self, cell: (i32, i32), act: MouseAct) -> bool {
        let Some(seat) = self.demo.as_ref().map(|d| d.seat) else { return false };
        let took = match seat {
            DemoSeat::Queen => queen::mouse(self, cell, act),
            DemoSeat::Keeper => keeper::mouse(self, cell, act),
            DemoSeat::Enemy => enemy::mouse(self, cell, act),
            _ => false,
        };
        if let Some(d) = self.demo.as_mut() {
            d.mouse_held = match act {
                MouseAct::Press { .. } => took,
                MouseAct::Drag => d.mouse_held,
                MouseAct::Release => false,
            };
        }
        took
    }

    /// **A key for the demo layer.** `true` when the layer used it, and the
    /// lab must not also act on it.
    pub fn demo_key(&mut self, key: DemoKey, pressed: bool) -> bool {
        let Some(seat) = self.demo.as_ref().map(|d| d.seat) else { return false };
        // The card first: `H` toggles it, and any other press only closes it
        // (a key that also steered or bit would act on a world the player
        // has not looked at yet). Releases pass, so a key held across the
        // card is not left stuck down.
        if pressed {
            let open = self.demo.as_ref().is_some_and(|d| d.card);
            if open {
                self.close_seat_card();
                return true;
            }
            if key == DemoKey::Letter('H') {
                self.open_seat_card(true);
                return true;
            }
        }
        // The seat's own keys first, so a seat can claim one even with no
        // body (the keeper, the colony mind) -- and a body letter it routes
        // by its own rules (`seat_takes`: the guest seat's B and E).
        if let DemoKey::Letter(c) = key {
            if matches!(c, 'Y' | 'U' | 'P' | 'J' | 'K' | 'R' | 'H') || seat_takes(seat, c) {
                if pressed {
                    if seat == DemoSeat::Queen {
                        queen::seat_key(self, c);
                    } else if seat == DemoSeat::Keeper {
                        keeper::seat_key(self, c);
                    } else if seat == DemoSeat::Scout {
                        scout::seat_key(self, c);
                    } else if seat == DemoSeat::Guest {
                        guest::seat_key(self, c);
                    } else if seat == DemoSeat::Enemy {
                        enemy::seat_key(self, c);
                    }
                }
                return true;
            }
        }
        if key == DemoKey::Sprint {
            if seat != DemoSeat::Enemy {
                return false;
            }
            enemy::sprint_key(self, pressed);
            return true;
        }
        if seat.body().is_none() {
            return false;
        }
        let demo = self.demo.as_mut().expect("read above");
        match key {
            DemoKey::Left => demo.held.left = pressed,
            DemoKey::Right => demo.held.right = pressed,
            DemoKey::Up => demo.held.up = pressed,
            DemoKey::Down => demo.held.down = pressed,
            DemoKey::Tab => {
                if pressed {
                    demo.follow = !demo.follow;
                }
            }
            DemoKey::Letter('T') => demo.held.trail = pressed,
            DemoKey::Sprint => return false,
            DemoKey::Letter(c @ ('E' | 'Q' | 'F' | 'G' | 'B' | 'L')) => {
                if pressed {
                    let press = SteerInput {
                        eat: c == 'E',
                        drop: c == 'Q',
                        share: c == 'F',
                        dig: c == 'G',
                        bite: c == 'B',
                        lay: c == 'L',
                        ..SteerInput::default()
                    };
                    if let Some(s) = self.world.steered.as_mut() {
                        s.press(press);
                    }
                }
            }
            DemoKey::Letter(_) => return false,
        }
        true
    }

    /// **Once a displayed frame**: hand the held keys to the body (or let go
    /// above [`STEER_CAP`]), follow her with the camera, and run the seat's
    /// own per-frame work.
    pub fn demo_frame(&mut self) {
        let Some(demo) = self.demo.as_ref() else { return };
        let (seat, held, follow) = (demo.seat, demo.held, demo.follow);
        // A seat's warm-up ending under the card (the dial dropping from the
        // top to play speed) waits for the reader: pause, and resume when
        // the card closes.
        if demo.card {
            let (was, now) = (demo.card_seen_multiple, self.time.multiple());
            if was > STEER_CAP && (1..=STEER_CAP).contains(&now) {
                self.time.pause();
                if let Some(d) = self.demo.as_mut() {
                    d.card_resume = true;
                }
                self.touch_chrome();
            }
            if let Some(d) = self.demo.as_mut() {
                d.card_seen_multiple = self.time.multiple();
            }
        }
        self.demo_bind_body();
        if seat.body().is_some() {
            let steering = self.time.multiple() <= STEER_CAP;
            if let Some(s) = self.world.steered.as_mut() {
                s.input = steering.then_some(SteerInput {
                    dir: held.dir(),
                    trail: held.trail,
                    ..SteerInput::default()
                });
            }
            if follow {
                if let Some(at) = steer::steered_id(&self.world).and_then(|id| self.world.organism(id)).and_then(super::roster::anchor_of) {
                    let bounds = self.world.bounds();
                    self.renderer.follow(at, (super::WIDTH, super::HEIGHT), bounds);
                }
            }
        }
        match seat {
            DemoSeat::Queen => queen::frame(self),
            DemoSeat::Keeper => keeper::frame(self),
            DemoSeat::Scout => scout::frame(self),
            DemoSeat::Guest => guest::frame(self),
            DemoSeat::Enemy => enemy::frame(self),
            _ => {}
        }
    }
}

/// The ant alive longest (lowest born frame, then lowest id): a default body
/// for a seat that has not said which ant it wants.
pub fn oldest_ant(world: &crate::sim::world::World) -> Option<OrganismId> {
    let ant = world.species.id_of("ant")?;
    let mut best: Option<(u64, OrganismId)> = None;
    for id in world.live_organism_ids() {
        let Some(s) = world.organism(id) else { continue };
        if s.species != ant {
            continue;
        }
        if best.is_none_or(|b| (s.born_frame, id) < b) {
            best = Some((s.born_frame, id));
        }
    }
    best.map(|(_, id)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_seat_parses_back_from_its_own_name() {
        for s in DemoSeat::ALL {
            assert_eq!(DemoSeat::parse(s.name()), Some(s));
        }
        assert_eq!(DemoSeat::parse("colony-mind"), Some(DemoSeat::Mind));
        assert_eq!(DemoSeat::parse("nobody"), None);
    }

    /// **The layer end to end, without a window**: a scout-seat demo on a
    /// box whose colony lands early binds a worker body once it lands, an
    /// arrow held through [`Lab::demo_key`] reaches her as a steer at her
    /// next decision, `R` is the seat's (the box is not rebuilt), and the
    /// rule her colony runs is not the queen's. The positive control for
    /// "the key reached her" is `used`, which only `take_input` writes.
    #[test]
    fn an_arrow_through_the_layer_steers_the_bound_body() {
        use crate::lab::scenario::{Placement, Scenario};
        let mut sc = Scenario::load("steady_income").expect("steady_income ships");
        for e in &mut sc.timeline {
            if matches!(e.what, Placement::Colony { .. } | Placement::Heap { .. }) {
                e.at = 200;
            }
        }
        let mut lab = Lab::new(sc.bed.clone());
        lab.load_scenario(sc);
        lab.demo = Some(Demo::new(DemoSeat::Scout));
        lab.demo_frame();
        assert!(steer::steered_id(&lab.world).is_none(), "bound a body before the colony landed");
        for _ in 0..300 {
            lab.tick_for_harness();
        }
        lab.demo_frame();
        let id = steer::steered_id(&lab.world).expect("no body bound after the colony landed");
        assert_eq!(lab.world.steered.as_ref().map(|s| s.role), Some(SteerRole::Worker));
        assert!(lab.demo_key(DemoKey::Right, true), "the layer did not take an arrow");
        assert!(lab.demo_key(DemoKey::Letter('R'), true), "R reached the lab in a demo");
        assert!(!lab.demo_key(DemoKey::Letter('Z'), true), "the layer swallowed a lab tool key");
        let frame = lab.world.frame;
        for _ in 0..30 {
            lab.demo_frame();
            lab.tick_for_harness();
        }
        assert!(lab.world.frame > frame, "R rebuilt the box");
        assert_eq!(steer::steered_id(&lab.world), Some(id));
        assert_eq!(lab.world.steered.as_ref().map(|s| s.used.dir), Some(Some((1, 0))), "the arrow never reached her");
    }
}
