//! **Mould: herd a slime mould with a lamp.** `cargo run --release --bin mould`
//!
//! A probe of its own, not a lab seat: everything it does to a dish lives in
//! `pixel_physics::mould`; this file is winit and `pixels` and nothing else,
//! the split `src/bin/lab.rs` and `src/bin/druid.rs` use.
//!
//! The keys are on screen (the bottom line), and so is what the next click
//! will do. It opens on the lesson (`mould::lesson`), a dish that teaches one
//! tool at a time; N goes on to dish 1. `cargo run --release --bin mould -- 3`
//! starts on dish seed 3, and `-- 0` is the lesson. Every dish opens behind
//! the briefing card; H shows it again.
//!
//! **It opens in the game's look** (`mould::look::Style::GAME`, the owner's
//! pick from the interface round: B's dark culture with C's tool tiles and
//! storm timeline). **F2 cycles the looks**: that one, the look before it,
//! then the three candidates, named only by a letter in the bottom-right
//! corner as they were judged. `-- 0 B` opens the lesson in look B.
//!
//! **The Wanderer** (`mould::wander`, prototype 2026-11-08): `-- wander`
//! (or `-- wander 3` for dish seed 3), or W from any dish. No clock and no
//! stars: lead her to the crack on the right and press G. N goes on to the
//! next dish with what arrived; R tries this dish again with what came in.
//!
//! Headless: `PIXEL_PHYSICS_MOULD_SCREENSHOT_AFTER_FRAMES=N` writes
//! `pixel_physics_mould_screenshot.png` to the temp directory after N drawn
//! frames, the same hook the other binaries have.

use std::sync::Arc;
use std::time::{Duration, Instant};

use pixel_physics::mould::lesson::{Lesson, Step};
use pixel_physics::mould::look::Style;
use pixel_physics::mould::render::{self, Banner, View};
use pixel_physics::mould::wander::{Outcome, FIRST_WALKERS};
use pixel_physics::mould::{Dish, Tool, H, STEPS_PER_FRAME, W};
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
    // Seed 0 is the lesson, and the game opens on it. `wander [seed]` opens
    // the Wanderer instead.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let wander = args.first().is_some_and(|a| a == "wander");
    let seed_arg = if wander { args.get(1) } else { args.first() };
    let seed = seed_arg
        .and_then(|s| s.parse().ok())
        .unwrap_or(if wander { 1 } else { 0 });
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let journey = wander.then_some(Journey {
        stretch: 1,
        carry: FIRST_WALKERS,
    });
    let mut handler = Handler::new(seed, journey);
    // `-- 0 B`: open in an interface look (`mould::look`), by name or letter.
    if let Some(style) = args.last().and_then(|s| Style::parse(s)) {
        handler.view.style = style;
    }
    event_loop.run_app(&mut handler)?;
    handler.result
}

/// Where a Wanderer run is: which dish of the journey, and how many walkers
/// it starts with.
#[derive(Clone, Copy, Debug)]
struct Journey {
    stretch: u32,
    carry: u32,
}

/// Seed 0 is the lesson; any other seed is a scored dish, or a stretch of
/// the Wanderer's journey.
fn new_dish(seed: u64, journey: Option<Journey>) -> (Dish, Option<Lesson>) {
    if let Some(j) = journey {
        return (Dish::wanderer(seed.max(1), j.stretch, j.carry), None);
    }
    if seed == 0 {
        let (d, l) = Lesson::dish();
        (d, Some(l))
    } else {
        (Dish::new(seed), None)
    }
}

fn title(seed: u64, journey: Option<Journey>) -> String {
    if let Some(j) = journey {
        return format!("Mould - Mabel, dish {}", j.stretch);
    }
    if seed == 0 {
        "Mould - lesson".into()
    } else {
        format!("Mould - dish {seed}")
    }
}

struct Handler {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    dish: Dish,
    seed: u64,
    lesson: Option<Lesson>,
    /// The Wanderer's journey, when that is what is being played.
    journey: Option<Journey>,
    view: View,
    /// Cursor in dish cells.
    cursor: Option<(f32, f32)>,
    pressed: bool,
    last_frame: Instant,
    accumulator: Duration,
    frames: u64,
    screenshot_after: Option<u64>,
    result: Result<(), Box<dyn std::error::Error>>,
}

impl Handler {
    fn new(seed: u64, journey: Option<Journey>) -> Self {
        let screenshot_after = std::env::var("PIXEL_PHYSICS_MOULD_SCREENSHOT_AFTER_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok());
        let (dish, lesson) = new_dish(seed, journey);
        Handler {
            window: None,
            pixels: None,
            dish,
            seed,
            lesson,
            journey,
            view: View {
                brief: true,
                style: Style::GAME,
                ..View::default()
            },
            cursor: None,
            pressed: false,
            last_frame: Instant::now(),
            accumulator: Duration::ZERO,
            frames: 0,
            screenshot_after,
            result: Ok(()),
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        self.result = Err(message.into());
        event_loop.exit();
    }

    fn restart(&mut self, seed: u64) {
        self.seed = seed;
        (self.dish, self.lesson) = new_dish(seed, self.journey);
        self.pressed = false;
        self.view.brief = true;
        // Speed and pause belong to one dish: a fast dish's successor
        // started at 4x.
        self.view.fast = false;
        self.view.paused = false;
        if let Some(w) = &self.window {
            w.set_title(&title(seed, self.journey));
        }
    }

    /// N in the Wanderer: on to the next dish with what arrived, or, after
    /// she died, a new journey from the first dish.
    fn next_stretch(&mut self) {
        let Some(j) = self.journey else { return };
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
        self.restart(self.seed + 1);
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
        self.pressed = true;
        let Some((x, y)) = self.cursor else { return };
        match (down, self.view.tool) {
            (true, Tool::Oats) => {
                self.dish.place_oats(x, y);
            }
            (true, t @ (Tool::Salt | Tool::Snip)) => {
                self.dish.begin_stroke(t, x, y);
            }
            _ => {}
        }
        self.hold();
    }

    fn key(&mut self, code: KeyCode, event_loop: &ActiveEventLoop) {
        if self.view.brief && code != KeyCode::Escape && code != KeyCode::F2 {
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
            KeyCode::Digit1 => self.view.tool = Tool::Lamp,
            KeyCode::Digit2 => self.view.tool = Tool::Oats,
            KeyCode::Digit3 => self.view.tool = Tool::Salt,
            KeyCode::Digit4 => self.view.tool = Tool::Snip,
            KeyCode::Space => self.view.fast = !self.view.fast,
            KeyCode::KeyP => self.view.paused = !self.view.paused,
            KeyCode::KeyR => self.restart(self.seed),
            KeyCode::KeyN if self.journey.is_some() => self.next_stretch(),
            KeyCode::KeyN => self.restart(self.seed + 1),
            KeyCode::F2 => self.view.style = self.view.style.next(),
            // Go: only while fed mould is in the crack (the dish refuses
            // otherwise, and the hint line says why).
            KeyCode::KeyG => {
                self.dish.go();
            }
            // W: start a Wanderer journey from wherever you are.
            KeyCode::KeyW => {
                self.journey = Some(Journey {
                    stretch: 1,
                    carry: FIRST_WALKERS,
                });
                self.restart(self.seed.max(1));
            }
            KeyCode::Escape => event_loop.exit(),
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
            if !self.view.paused && !self.view.brief {
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
        let Some(pixels) = self.pixels.as_mut() else { return };
        self.view.cursor = self.cursor;
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
    let (fw, fh) = render::frame_size(SCALE);
    if width < fw || height < fh {
        ScalingMode::Fill
    } else {
        ScalingMode::PixelPerfect
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let (w, h) = render::frame_size(SCALE);
        let attributes = Window::default_attributes()
            .with_title(title(self.seed, self.journey))
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
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if event.state == ElementState::Pressed && !event.repeat {
                        self.key(code, event_loop);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = self
                    .pixels
                    .as_ref()
                    .and_then(|p| p.window_pos_to_pixel((position.x as f32, position.y as f32)).ok())
                    .map(|(x, y)| (x as f32 / SCALE as f32, y as f32 / SCALE as f32))
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
