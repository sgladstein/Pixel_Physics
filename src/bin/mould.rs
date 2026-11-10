//! **Mould: herd a slime mould with a lamp.** `cargo run --release --bin mould`
//!
//! A probe of its own, not a lab seat: everything it does to a dish lives in
//! `pixel_physics::mould`; this file is winit and `pixels` and nothing else,
//! the split `src/bin/lab.rs` and `src/bin/druid.rs` use.
//!
//! The keys are on screen (the bottom line), and so is what the next click
//! will do. It opens on the lesson (`mould::lesson`), a dish that teaches one
//! tool at a time; N goes on to the first challenge dish (`mould::level`).
//! M opens the dish picker: the lesson, the challenge dishes (each opens on
//! a star from the one before) and the free seeded dishes, where N deals
//! another. Best stars are kept in `mould-progress.txt` beside the
//! executable. `cargo run --release --bin mould -- 3` starts on free dish
//! seed 3, `-- 0` is the lesson and `-- d2` challenge dish 2. Every dish
//! opens behind the briefing card; H shows it again.
//!
//! **It opens in the game's look** (`mould::look::Style::GAME`, the owner's
//! pick from the interface round: B's dark culture with C's tool tiles and
//! storm timeline). **F2 cycles the looks**: that one, the look before it,
//! then the three candidates, named only by a letter in the bottom-right
//! corner as they were judged. `-- 0 B` opens the lesson in look B.
//!
//! **The looks pass** (2026-11-10), every one on by default and each with a
//! key to compare it against the drawing before, named in the title bar:
//! F3 how food in the tube is drawn (the food ramp, then grey, embers,
//! dried slime), F4 the walkers (grains, glints, off), F5 the smoothing of
//! the rim, tubes, salt and cuts, F6 the stain where mould has been, F7
//! the mould drawn straight from the simulation or as tubes.
//!
//! **The Wanderer** (`mould::wander`, prototype 2026-11-08): `-- wander`
//! (or `-- wander 3` for dish seed 3), W from any dish, or THE WANDERER on
//! the picker. No clock and no stars: lead her to the crack on the right and
//! press G. N goes on to the next dish with what arrived; R tries this dish
//! again with what came in.
//!
//! Headless: `PIXEL_PHYSICS_MOULD_SCREENSHOT_AFTER_FRAMES=N` writes
//! `pixel_physics_mould_screenshot.png` to the temp directory after N drawn
//! frames, the same hook the other binaries have.

use std::sync::Arc;
use std::time::{Duration, Instant};

use pixel_physics::mould::lesson::{Lesson, Step};
use pixel_physics::mould::level::{Progress, DISHES, ENTRIES, FREE, WANDER};
use pixel_physics::mould::look::{self, Style};
use pixel_physics::mould::render::{self, Banner, Menu, QuitArm, View};
use pixel_physics::mould::wander::{Outcome, FIRST_WALKERS};
use pixel_physics::mould::{tune, Dish, Params, Phase, Tool, H, STEPS_PER_FRAME, W};
use pixels::{Pixels, ScalingMode, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// Buffer pixels per dish cell. 4 makes a 1024-wide window.
const SCALE: u32 = 4;
/// One drawn frame of simulation: `STEPS_PER_FRAME` steps, sixty times a
/// second, whatever the display's rate.
const TICK: Duration = Duration::from_nanos(1_000_000_000 / 60);
const MAX_TICKS_PER_FRAME: u32 = 4;
/// Space runs the dish this many times faster: the brief's "wait" button.
const FAST: u32 = 4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The lesson by default; a number is a free dish seed (0 the lesson),
    // `d1`.. a challenge dish, `wander [seed]` the Wanderer.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = args.first().cloned().unwrap_or_default();
    let dish = arg
        .strip_prefix('d')
        .and_then(|n| n.parse::<usize>().ok())
        .filter(|k| (1..=DISHES.len()).contains(k));
    let entry = match (dish, arg.parse::<u64>()) {
        _ if arg == "wander" => Entry::Wander(args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1).max(1)),
        (Some(k), _) => Entry::Dish(k - 1),
        (None, Ok(seed)) if seed > 0 => Entry::Free(seed),
        _ => Entry::Lesson,
    };
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut handler = Handler::new(entry);
    // `-- 0 B`: open in an interface look (`mould::look`), by name or letter.
    if let Some(style) = args.last().and_then(|s| Style::parse(s)) {
        handler.view.style = style;
    }
    event_loop.run_app(&mut handler)?;
    handler.result
}

/// What is being played: the lesson, a challenge dish, a seeded free dish,
/// or a stretch of the Wanderer's journey (on that dish seed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    Lesson,
    Dish(usize),
    Free(u64),
    Wander(u64),
}

impl Entry {
    /// Its card on the picker.
    fn index(self) -> usize {
        match self {
            Entry::Lesson => 0,
            Entry::Dish(k) => k + 1,
            Entry::Free(_) => FREE,
            Entry::Wander(_) => WANDER,
        }
    }
    fn from_index(e: usize, free: u64) -> Entry {
        match e {
            0 => Entry::Lesson,
            FREE => Entry::Free(free),
            _ if e >= WANDER => Entry::Wander(1),
            _ => Entry::Dish(e - 1),
        }
    }
}

/// Where a Wanderer run is: which dish of the journey, and how many walkers
/// it starts with.
#[derive(Clone, Copy, Debug)]
struct Journey {
    stretch: u32,
    carry: u32,
}

impl Journey {
    const FIRST: Journey = Journey {
        stretch: 1,
        carry: FIRST_WALKERS,
    };
}

/// The first tool a dish offers, in bar order.
fn first_tool(d: &Dish) -> Tool {
    [Tool::Lamp, Tool::Oats, Tool::Salt, Tool::Snip]
        .into_iter()
        .find(|&t| d.has_tool(t))
        .unwrap_or(Tool::Lamp)
}

/// The mould's own randomness on a challenge dish: fixed, so a retry is the
/// same dish until the hand changes it.
const LEVEL_SEED: u64 = 1;

fn new_dish(entry: Entry, journey: Option<Journey>) -> (Dish, Option<Lesson>) {
    match entry {
        Entry::Wander(seed) => {
            let j = journey.unwrap_or(Journey::FIRST);
            (Dish::wanderer(seed.max(1), j.stretch, j.carry), None)
        }
        Entry::Lesson => {
            let (d, l) = Lesson::dish();
            (d, Some(l))
        }
        Entry::Dish(k) => (Dish::from_level(&DISHES[k], LEVEL_SEED), None),
        Entry::Free(seed) => (Dish::new(seed), None),
    }
}

fn title(entry: Entry, journey: Option<Journey>) -> String {
    match entry {
        Entry::Wander(_) => format!("Mould - Mabel, dish {}", journey.map_or(1, |j| j.stretch)),
        Entry::Lesson => "Mould - lesson".into(),
        Entry::Dish(k) => format!("Mould - {}", DISHES[k].name.to_lowercase()),
        Entry::Free(seed) => format!("Mould - free dish {seed}"),
    }
}

/// `mould-progress.txt` beside the executable.
fn progress_path() -> Option<std::path::PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join("mould-progress.txt"))
}

/// Best stars so far: a missing or unreadable file is a fresh start.
fn load_progress() -> Progress {
    progress_path()
        .and_then(|p| std::fs::read(p).ok())
        .map(|b| Progress::parse(&String::from_utf8_lossy(&b)))
        .unwrap_or_default()
}

fn save_progress(p: &Progress) {
    if let Some(path) = progress_path() {
        if let Err(err) = std::fs::write(&path, p.text()) {
            eprintln!("could not save {}: {err}", path.display());
        }
    }
}

struct Handler {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    dish: Dish,
    entry: Entry,
    /// The free dish N deals next from, kept across visits to the picker.
    free_seed: u64,
    progress: Progress,
    /// This dish's result is recorded (once per dish).
    recorded: bool,
    lesson: Option<Lesson>,
    /// The Wanderer's journey, when that is what is being played.
    journey: Option<Journey>,
    view: View,
    /// Cursor in dish cells, on the dish only.
    cursor: Option<(f32, f32)>,
    /// Cursor in frame pixels, anywhere on the frame: the bar's tool tiles
    /// are clicked through this (`render::press_at`).
    pointer: Option<(f32, f32)>,
    pressed: bool,
    last_frame: Instant,
    accumulator: Duration,
    frames: u64,
    screenshot_after: Option<u64>,
    /// Escape was pressed once on a dish, at this time: a second within
    /// `QuitArm::WINDOW` quits.
    quit: QuitArm,
    /// The tuning panel's values (T, `mould::tune`), applied to every dish
    /// opened and saved to `tune::TUNE_FILE`.
    tuned: Params,
    shift: bool,
    result: Result<(), Box<dyn std::error::Error>>,
}

impl Handler {
    fn new(entry: Entry) -> Self {
        let screenshot_after = std::env::var("PIXEL_PHYSICS_MOULD_SCREENSHOT_AFTER_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok());
        let journey = matches!(entry, Entry::Wander(_)).then_some(Journey::FIRST);
        let (mut dish, lesson) = new_dish(entry, journey);
        let mut tuned = Params::default();
        if let Ok(text) = std::fs::read_to_string(tune::TUNE_FILE) {
            for bad in tune::from_text(&text, &mut tuned) {
                eprintln!("{}: not understood: {bad}", tune::TUNE_FILE);
            }
        }
        tune::apply(&tuned, &mut dish.params);
        Handler {
            window: None,
            pixels: None,
            dish,
            entry,
            free_seed: match entry {
                Entry::Free(s) | Entry::Wander(s) => s,
                _ => 1,
            },
            progress: load_progress(),
            recorded: false,
            lesson,
            journey,
            view: View {
                brief: true,
                style: Style::GAME,
                ..View::default()
            },
            cursor: None,
            pointer: None,
            pressed: false,
            last_frame: Instant::now(),
            accumulator: Duration::ZERO,
            frames: 0,
            screenshot_after,
            quit: QuitArm::default(),
            tuned,
            shift: false,
            result: Ok(()),
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        self.result = Err(message.into());
        event_loop.exit();
    }

    /// Start `entry`. A Wanderer entry carries on the journey in hand (R
    /// retries a stretch with what came in) or starts one; anything else
    /// ends it.
    fn restart(&mut self, entry: Entry) {
        self.entry = entry;
        match entry {
            Entry::Free(s) => {
                self.free_seed = s;
                self.journey = None;
            }
            Entry::Wander(_) => {
                self.journey.get_or_insert(Journey::FIRST);
            }
            _ => self.journey = None,
        }
        (self.dish, self.lesson) = new_dish(entry, self.journey);
        tune::apply(&self.tuned, &mut self.dish.params);
        self.recorded = false;
        self.pressed = false;
        self.view.brief = true;
        self.view.menu = None;
        // Speed and pause belong to one dish: a fast dish's successor
        // started at 4x.
        self.view.fast = false;
        self.view.paused = false;
        // The first tool this dish offers: one carried over from a dish that
        // had it (snip, from free play) was selected but not on the bar,
        // and a click did nothing (review 1, finding 6).
        self.view.tool = first_tool(&self.dish);
        if let Some(w) = &self.window {
            w.set_title(&title(entry, self.journey));
        }
    }

    /// W, or THE WANDERER on the picker: a new journey from its first dish,
    /// on the free dish seed in hand.
    fn start_journey(&mut self) {
        self.journey = Some(Journey::FIRST);
        let seed = match self.entry {
            Entry::Free(s) | Entry::Wander(s) => s,
            _ => self.free_seed,
        };
        self.restart(Entry::Wander(seed.max(1)));
    }

    /// N: the next dish in the run if it is open, the next free dish on
    /// free play, and otherwise the picker, which says what opens it.
    fn next(&mut self) {
        let e = self.entry.index() + 1;
        match self.entry {
            Entry::Free(s) => self.restart(Entry::Free(s + 1)),
            _ if self.progress.unlocked(e) => self.restart(Entry::from_index(e, self.free_seed)),
            _ => self.open_menu(),
        }
    }

    fn open_menu(&mut self) {
        self.let_go();
        self.view.menu = Some(Menu {
            progress: self.progress,
            cursor: self.entry.index(),
        });
    }

    /// Pick a picker entry: played if open, otherwise only highlighted.
    fn pick(&mut self, e: usize) {
        if e >= ENTRIES {
            return;
        }
        if e == WANDER {
            self.start_journey();
        } else if self.progress.unlocked(e) {
            self.restart(Entry::from_index(e, self.free_seed));
        } else if let Some(m) = self.view.menu.as_mut() {
            m.cursor = e;
        }
    }

    /// Keep the best result: a challenge dish's stars when it ends, and the
    /// lesson once it is finished.
    fn record(&mut self) {
        if self.recorded {
            return;
        }
        let stars = match (self.entry, &self.lesson) {
            (Entry::Lesson, Some(l)) if l.step == Step::Done => 1,
            (Entry::Dish(_), _) if self.dish.phase == Phase::Over => {
                self.dish.card.as_ref().map_or(0, |c| c.stars)
            }
            _ => return,
        };
        self.recorded = true;
        if self.progress.record(self.entry.index(), stars) {
            save_progress(&self.progress);
        }
    }

    /// N in the Wanderer: on to the next dish with what arrived, or, after
    /// she died, a new journey from the first dish.
    fn next_stretch(&mut self) {
        let (Some(j), Entry::Wander(seed)) = (self.journey, self.entry) else { return };
        let arrived = match self.dish.wander_state().and_then(|w| w.outcome) {
            Some(Outcome::Arrived { n, .. }) => Some(n),
            Some(Outcome::Lost { .. }) => None,
            // Still on this dish: N does nothing until she has left or died.
            None => return,
        };
        self.journey = Some(match arrived {
            Some(n) => Journey {
                stretch: j.stretch + 1,
                carry: n,
            },
            None => Journey {
                stretch: 1,
                carry: FIRST_WALKERS,
            },
        });
        self.restart(Entry::Wander(seed + 1));
    }

    /// What holding the mouse does, every frame it is held.
    fn hold(&mut self) {
        let lamp = match (self.pressed, self.view.tool, self.cursor) {
            (true, Tool::Lamp, Some(c)) => Some(c),
            _ => None,
        };
        self.dish.set_lamp(lamp);
    }

    /// Let go of everything: no button held, no stroke drawing, lamp off.
    /// Used whenever the hand may have left without a release reaching us.
    fn let_go(&mut self) {
        self.pressed = false;
        self.dish.end_stroke();
        self.hold();
    }

    fn mouse(&mut self, down: bool) {
        // A click on the picker picks the card under it.
        if self.view.menu.is_some() {
            if down {
                if let Some((x, y)) = self.cursor {
                    let (px, py) = ((x * SCALE as f32) as i32, (y * SCALE as f32) as i32);
                    let hit = render::menu_cards(render::frame_size(SCALE))
                        .iter()
                        .position(|&(cx, cy, w, h)| px >= cx && px < cx + w && py >= cy && py < cy + h);
                    if let Some(e) = hit {
                        self.pick(e);
                    }
                }
            }
            self.let_go();
            return;
        }
        // A click on the briefing card only puts it away.
        if self.view.brief {
            if down {
                self.view.brief = false;
            }
            self.let_go();
            return;
        }
        // A release ends the stroke wherever it lands. It used to return
        // first when the cursor was over the HUD strip or outside the
        // window, leaving the stroke live for the next press to continue.
        if !down {
            self.let_go();
            return;
        }
        // Only a press on the dish holds the button; one on a tool tile
        // selects the tool (review 2, finding M1: the tiles did nothing,
        // and a press on the bar slid up onto the dish lit the lamp).
        let Some((px, py)) = self.pointer else { return };
        let (x, y) = match render::press_at(&self.dish, &self.view, SCALE, px, py) {
            render::Press::Dish(x, y) => (x, y),
            render::Press::Tile(t) => {
                if self.dish.bar_tools().contains(&t) {
                    self.view.tool = t;
                }
                return self.let_go();
            }
            render::Press::Off => return,
        };
        self.pressed = true;
        match self.view.tool {
            Tool::Oats => {
                self.dish.place_oats(x, y);
            }
            t @ (Tool::Salt | Tool::Snip) => {
                self.dish.begin_stroke(t, x, y);
            }
            Tool::Water => {
                self.dish.place_water(x, y);
            }
            Tool::Lamp => {}
        }
        self.hold();
    }

    fn key(&mut self, code: KeyCode, event_loop: &ActiveEventLoop) {
        // Escape quits only twice running on a dish: once closing the picker
        // made the next Escape quit the game outright (review 1, finding 9).
        let armed = self.quit.armed(Instant::now());
        self.quit.clear();
        if let Some(w) = &self.window {
            w.set_title(&title(self.entry, self.journey));
        }
        // The tuning panel (T): the arrows pick and change a row, Enter
        // flips a switch, Backspace resets it; the dish keeps running.
        if code == KeyCode::KeyT {
            self.view.tune = match self.view.tune {
                Some(_) => None,
                None => Some(0),
            };
            return;
        }
        if let Some(row) = self.view.tune {
            let n = tune::KNOBS.len();
            let steps = if self.shift { 5 } else { 1 };
            let changed = match code {
                KeyCode::ArrowUp => {
                    self.view.tune = Some((row + n - 1) % n);
                    false
                }
                KeyCode::ArrowDown => {
                    self.view.tune = Some((row + 1) % n);
                    false
                }
                KeyCode::ArrowLeft => {
                    tune::nudge(&mut self.tuned, row, -steps);
                    true
                }
                KeyCode::ArrowRight | KeyCode::Enter => {
                    tune::nudge(&mut self.tuned, row, steps);
                    true
                }
                KeyCode::Backspace => {
                    tune::reset(&mut self.tuned, row);
                    true
                }
                _ => false,
            };
            if changed {
                tune::apply(&self.tuned, &mut self.dish.params);
                if let Err(e) = std::fs::write(tune::TUNE_FILE, tune::to_text(&self.tuned)) {
                    eprintln!("could not save {}: {e}", tune::TUNE_FILE);
                }
            }
            if matches!(code, KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::Enter | KeyCode::Backspace) {
                return;
            }
        }
        // The picker: 1-6 picks, the arrows and Enter choose, M or Escape
        // goes back to the dish.
        if let Some(m) = self.view.menu {
            let digit = [
                KeyCode::Digit1,
                KeyCode::Digit2,
                KeyCode::Digit3,
                KeyCode::Digit4,
                KeyCode::Digit5,
                KeyCode::Digit6,
                KeyCode::Digit7,
                KeyCode::Digit8,
                KeyCode::Digit9,
            ]
            .iter()
            .position(|&k| k == code);
            match code {
                KeyCode::KeyM | KeyCode::Escape => self.view.menu = None,
                KeyCode::ArrowUp => self.view.menu = Some(Menu { cursor: m.cursor.saturating_sub(1), ..m }),
                KeyCode::ArrowDown => self.view.menu = Some(Menu { cursor: (m.cursor + 1).min(ENTRIES - 1), ..m }),
                KeyCode::Enter | KeyCode::Space => self.pick(m.cursor),
                _ => {
                    if let Some(e) = digit {
                        self.pick(e);
                    }
                }
            }
            return;
        }
        if code == KeyCode::KeyM {
            self.open_menu();
            return;
        }
        if self.view.brief && code != KeyCode::Escape && code != KeyCode::F2 && code != KeyCode::F3 && code != KeyCode::F4 && code != KeyCode::F5 && code != KeyCode::F6 && code != KeyCode::F7 {
            self.view.brief = false;
            self.let_go();
            return;
        }
        match code {
            // The card hides the dish, so a held lamp or stroke ends with it.
            KeyCode::KeyH => {
                self.view.brief = true;
                self.let_go();
            }
            // Only the tools on this dish's bar.
            // A dish with water (`Params::wet`) keys its bar by position
            // (`Dish::bar_tools`: Keep It Alive is 1 lamp, 2 oats, 3
            // water); every other dish keeps the fixed four.
            KeyCode::Digit1 | KeyCode::Digit2 | KeyCode::Digit3 | KeyCode::Digit4 | KeyCode::Digit5 => {
                let k = match code {
                    KeyCode::Digit1 => 0,
                    KeyCode::Digit2 => 1,
                    KeyCode::Digit3 => 2,
                    KeyCode::Digit4 => 3,
                    _ => 4,
                };
                let t = if self.dish.params.wet {
                    self.dish.bar_tools().get(k).copied()
                } else {
                    Tool::ALL.get(k).copied()
                };
                if let Some(t) = t.filter(|t| self.dish.bar_tools().contains(t)) {
                    self.view.tool = t;
                }
            }
            KeyCode::Space => self.view.fast = !self.view.fast,
            KeyCode::KeyP => self.view.paused = !self.view.paused,
            KeyCode::KeyR => self.restart(self.entry),
            KeyCode::KeyN if self.journey.is_some() => self.next_stretch(),
            KeyCode::KeyN => self.next(),
            KeyCode::F2 => self.view.style = self.view.style.next(),
            // How starving mould is drawn (`look::STARVE_LOOK`): grey,
            // embers, dried slime, the food ramp (default). Render-only;
            // named in the title.
            KeyCode::F3 => {
                use std::sync::atomic::Ordering::Relaxed;
                let n = (look::STARVE_LOOK.load(Relaxed) + 1) % look::STARVE_LOOKS.len() as u8;
                look::STARVE_LOOK.store(n, Relaxed);
                if let Some(w) = &self.window {
                    w.set_title(&format!("Mould: starving mould drawn as {} (F3)", look::STARVE_LOOKS[n as usize]));
                }
            }
            // How the walkers are drawn (`look::WALKER_LOOK`): off, grains,
            // glints. Render-only; named in the title.
            KeyCode::F4 => {
                use std::sync::atomic::Ordering::Relaxed;
                let n = (look::WALKER_LOOK.load(Relaxed) + 1) % look::WALKER_LOOKS.len() as u8;
                look::WALKER_LOOK.store(n, Relaxed);
                if let Some(w) = &self.window {
                    w.set_title(&format!("Mould: walkers drawn as {} (F4)", look::WALKER_LOOKS[n as usize]));
                }
            }
            // The anti-pixel pass (`look::SMOOTH`), on or off. Render-only.
            KeyCode::F5 => {
                use std::sync::atomic::Ordering::Relaxed;
                let on = !look::SMOOTH.load(Relaxed);
                look::SMOOTH.store(on, Relaxed);
                if let Some(w) = &self.window {
                    w.set_title(&format!("Mould: smoothing {} (F5)", if on { "on" } else { "off" }));
                }
            }
            // The stain where mould has been (`look::STAIN`), on or off.
            KeyCode::F6 => {
                use std::sync::atomic::Ordering::Relaxed;
                let on = !look::STAIN.load(Relaxed);
                look::STAIN.store(on, Relaxed);
                if let Some(w) = &self.window {
                    w.set_title(&format!("Mould: stain {} (F6)", if on { "on" } else { "off" }));
                }
            }
            KeyCode::F7 => {
                use std::sync::atomic::Ordering::Relaxed;
                let on = !look::FIELD.load(Relaxed);
                look::FIELD.store(on, Relaxed);
                if let Some(w) = &self.window {
                    w.set_title(&format!("Mould: {} (F7)", if on { "drawn from the simulation" } else { "tube look" }));
                }
            }
            // Go: only while fed mould is in the crack (the dish refuses
            // otherwise, and the hint line says why).
            KeyCode::KeyG => {
                self.dish.go();
            }
            // W: start a Wanderer journey from wherever you are.
            KeyCode::KeyW => self.start_journey(),
            KeyCode::Escape if armed => event_loop.exit(),
            KeyCode::Escape => {
                self.quit.press(Instant::now());
                if let Some(w) = &self.window {
                    w.set_title("Mould - press Esc again to quit");
                }
            }
            _ => {}
        }
        self.hold();
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.accumulator += now - self.last_frame;
        self.last_frame = now;
        let mut ticks = 0;
        while self.accumulator >= TICK && ticks < MAX_TICKS_PER_FRAME {
            self.accumulator -= TICK;
            ticks += 1;
            if !self.view.paused && !self.view.brief && self.view.menu.is_none() {
                let n = if self.view.fast {
                    STEPS_PER_FRAME * FAST
                } else {
                    STEPS_PER_FRAME
                };
                for _ in 0..n {
                    self.dish.step();
                }
            }
        }
        if ticks == MAX_TICKS_PER_FRAME {
            self.accumulator = Duration::ZERO;
        }
        if let Some(l) = self.lesson.as_mut() {
            l.update(&mut self.dish);
            self.view.banner = Some(Banner {
                number: l.step.number(),
                of: Step::COUNT,
                lines: l.step.lines(),
                fresh: l.fresh(&self.dish),
            });
        } else {
            self.view.banner = None;
        }
        self.record();
        if let Some(m) = self.view.menu.as_mut() {
            m.progress = self.progress;
        }
        // The 2 s ran out with no second Escape: the title stops saying so
        // (review 2, finding L3; it waited for the next key).
        if self.quit.lapsed(now) {
            if let Some(w) = &self.window {
                w.set_title(&title(self.entry, self.journey));
            }
        }
        let Some(pixels) = self.pixels.as_mut() else { return };
        self.view.cursor = self.cursor;
        self.view.quit_armed = self.quit.armed(now);
        render::draw(&self.dish, &self.view, SCALE, pixels.frame_mut());
        if let Err(err) = pixels.render() {
            return self.fail(event_loop, format!("render failed: {err}"));
        }
        self.frames += 1;
        if self.screenshot_after == Some(self.frames) {
            let (w, h) = render::frame_size(SCALE);
            let path = std::env::temp_dir().join("pixel_physics_mould_screenshot.png");
            let img = image::RgbaImage::from_raw(w, h, pixels.frame().to_vec());
            match img.map(|i| i.save(&path)) {
                Some(Ok(())) => println!("screenshot written to {}", path.display()),
                other => eprintln!("screenshot failed: {other:?}"),
            }
        }
    }
}

/// Whole-pixel scaling when the frame fits the window, shrink-to-fit when it
/// does not. Pixel-perfect never scales below 1:1, so a window shorter than
/// the frame (726 px: a 768-px laptop) showed it cropped, HUD and all.
fn scaling_for(width: u32, height: u32) -> ScalingMode {
    if render::frame_fits(width, height, SCALE) {
        ScalingMode::PixelPerfect
    } else {
        ScalingMode::Fill
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let (w, h) = render::frame_size(SCALE);
        let attributes = Window::default_attributes()
            .with_title(title(self.entry, self.journey))
            .with_inner_size(LogicalSize::new(w, h))
            .with_min_inner_size(LogicalSize::new(W as u32 * 2, H as u32 * 2));
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(err) => return self.fail(event_loop, format!("window creation failed: {err}")),
        };
        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width, size.height, Arc::clone(&window));
        match Pixels::new(w, h, surface) {
            Ok(mut p) => {
                p.set_scaling_mode(scaling_for(size.width, size.height));
                self.pixels = Some(p);
            }
            Err(err) => return self.fail(event_loop, format!("pixels init failed: {err}")),
        }
        self.window = Some(window);
        self.last_frame = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    // Pixel-perfect never scales below 1:1, so a window
                    // shorter than the frame (726 px: a 768-px laptop) showed
                    // it cropped, HUD and all. Shrink to fit instead, and keep
                    // crisp whole-pixel scaling whenever the frame fits.
                    if let Some(p) = self.pixels.as_mut() {
                        p.set_scaling_mode(scaling_for(size.width, size.height));
                    }
                    let err = self
                        .pixels
                        .as_mut()
                        .and_then(|p| p.resize_surface(size.width, size.height).err());
                    if let Some(err) = err {
                        self.fail(event_loop, format!("surface resize failed: {err}"));
                    }
                }
            }
            WindowEvent::ModifiersChanged(m) => self.shift = m.state().shift_key(),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    // Held arrows repeat in the tuning panel only.
                    let repeat_ok = self.view.tune.is_some()
                        && matches!(code, KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown);
                    if event.state == ElementState::Pressed && (!event.repeat || repeat_ok) {
                        self.key(code, event_loop);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer = self
                    .pixels
                    .as_ref()
                    .and_then(|p| p.window_pos_to_pixel((position.x as f32, position.y as f32)).ok())
                    .map(|(x, y)| (x as f32, y as f32));
                self.cursor = self
                    .pointer
                    .map(|(x, y)| (x / SCALE as f32, y / SCALE as f32))
                    .filter(|&(_, y)| y < H as f32);
                if let Some((x, y)) = self.cursor {
                    if self.pressed && self.dish.stroke_active() {
                        self.dish.drag_stroke(x, y);
                    }
                }
                self.hold();
            }
            // Alt-tab with the button held: the release goes elsewhere.
            WindowEvent::Focused(false) => self.let_go(),
            WindowEvent::CursorLeft { .. } => {
                self.cursor = None;
                self.pointer = None;
                self.hold();
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                self.mouse(state == ElementState::Pressed);
            }
            WindowEvent::RedrawRequested => self.frame(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
