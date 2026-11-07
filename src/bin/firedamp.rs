//! **Firedamp.** `cargo run --release --bin firedamp` (from the repository
//! root). A probe of the new-game brainstorm of 2026-11-07: a mine filling
//! with gas you herd with fans and doors and burn with matches and fuses.
//! Everything it does to the mine lives in `pixel_physics::firedamp`; this
//! file is the window, the mouse and the speaker.
//!
//! - `1` fan, `2` door, `3` match, `4` fuse (or click the buttons in the bar)
//! - left button: use the tool; on a fan, drag to aim it; on a door, slam or
//!   open it
//! - right button: pick a fan, door or fuse back up
//! - `Space` pause, `R` same mine again, `N` a new mine, `Esc` quit
//!
//! `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=N` writes the window to
//! `<temp>/pixel_physics_firedamp.png` after N drawn frames, the same hook
//! the other binaries carry. `FIREDAMP_SEED=N` picks the mine.

use std::sync::Arc;
use std::time::{Duration, Instant};

use pixel_physics::audio::{self, GrainKind, SoundCmd};
use pixel_physics::firedamp::paint::{self, FH, FW, S};
use pixel_physics::firedamp::{gas, Event, Game, Tool, TICKS_PER_SECOND};
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

const TICK: Duration = Duration::from_nanos(1_000_000_000 / TICKS_PER_SECOND);
const MAX_TICKS_PER_FRAME: u32 = 5;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let seed = std::env::var("FIREDAMP_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut h = Handler {
        window: None,
        pixels: None,
        game: Game::new(seed),
        accumulator: Duration::ZERO,
        last: Instant::now(),
        cursor: None,
        screen_cursor: None,
        paused: false,
        sound: audio::Output::open(seed, audio::Style::default()).ok(),
        shot: std::env::var("PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES").ok().and_then(|v| v.parse().ok()),
        down: false,
    };
    event_loop.run_app(&mut h)?;
    Ok(())
}

struct Handler {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    game: Game,
    accumulator: Duration,
    last: Instant,
    /// The pointer in mine cells.
    cursor: Option<(i32, i32)>,
    /// The pointer in screen pixels, for the bar.
    screen_cursor: Option<(i32, i32)>,
    paused: bool,
    sound: Option<audio::Output>,
    shot: Option<u32>,
    down: bool,
}

impl Handler {
    fn key(&mut self, code: KeyCode, event_loop: &ActiveEventLoop) {
        match code {
            KeyCode::Digit1 => self.game.tool = Tool::Fan,
            KeyCode::Digit2 => self.game.tool = Tool::Door,
            KeyCode::Digit3 => self.game.tool = Tool::Match,
            KeyCode::Digit4 => self.game.tool = Tool::Fuse,
            KeyCode::Space => self.paused = !self.paused,
            KeyCode::KeyR => self.game = Game::new(self.game.seed),
            KeyCode::KeyN => self.game = Game::new(self.game.seed + 1),
            KeyCode::Escape => event_loop.exit(),
            _ => {}
        }
    }

    fn sounds(&mut self) {
        let events: Vec<Event> = std::mem::take(&mut self.game.events);
        let Some(out) = &mut self.sound else { return };
        let pan = |x: i32| (x as f32 / gas::W as f32) * 2.0 - 1.0;
        let mut cmds = Vec::new();
        for e in events {
            match e {
                Event::Fizzle { x, thick, .. } => {
                    let gain = if thick { 0.6 } else { 0.3 };
                    cmds.push(SoundCmd::Burst { kind: GrainKind::Hiss, rate: 40.0, secs: 0.35, pan: pan(x), spread: 0.1, gain, muffle: 0.2, delay: 0.0 });
                }
                Event::Lit { x, .. } => {
                    cmds.push(SoundCmd::Burst { kind: GrainKind::Crackle, rate: 30.0, secs: 0.8, pan: pan(x), spread: 0.4, gain: 0.5, muffle: 0.1, delay: 0.0 });
                }
                Event::Bang { x, gas: g, .. } => {
                    // The whump grows with the gas that was in the pocket.
                    let gain = (0.35 + g / 60.0).min(1.0);
                    cmds.push(SoundCmd::Grain { kind: GrainKind::Thud, pan: pan(x), gain, muffle: 0.3, delay: 0.0 });
                    cmds.push(SoundCmd::Burst { kind: GrainKind::Thud, rate: 8.0 + g / 6.0, secs: 0.3 + g / 150.0, pan: pan(x), spread: 0.5, gain: gain * 0.7, muffle: 0.5, delay: 0.03 });
                    cmds.push(SoundCmd::Burst { kind: GrainKind::Grit, rate: 50.0, secs: 1.2, pan: pan(x), spread: 0.6, gain: 0.4, muffle: 0.3, delay: 0.15 });
                }
                Event::FaceBlown { x, .. } => {
                    cmds.push(SoundCmd::Grain { kind: GrainKind::Thud, pan: pan(x), gain: 1.0, muffle: 0.2, delay: 0.0 });
                    cmds.push(SoundCmd::Burst { kind: GrainKind::Grit, rate: 70.0, secs: 1.6, pan: pan(x), spread: 0.6, gain: 0.6, muffle: 0.2, delay: 0.1 });
                }
                Event::Ore { x, .. } => {
                    cmds.push(SoundCmd::Burst { kind: GrainKind::Knock, rate: 25.0, secs: 0.6, pan: pan(x), spread: 0.4, gain: 0.35, muffle: 0.1, delay: 0.2 });
                }
                Event::Cart { x, .. } => cmds.push(SoundCmd::Grain { kind: GrainKind::Knock, pan: pan(x), gain: 0.12, muffle: 0.4, delay: 0.0 }),
                Event::Fan { x, .. } => cmds.push(SoundCmd::Grain { kind: GrainKind::Whoosh, pan: pan(x), gain: 0.5, muffle: 0.0, delay: 0.0 }),
                Event::Door { x, closed, .. } => cmds.push(SoundCmd::Grain {
                    kind: if closed { GrainKind::Thud } else { GrainKind::Knock },
                    pan: pan(x),
                    gain: 0.45,
                    muffle: 0.1,
                    delay: 0.0,
                }),
            }
        }
        let burning = self.game.gas.burning_count();
        if burning > 0 {
            cmds.push(SoundCmd::Grain { kind: GrainKind::Crackle, pan: 0.0, gain: ((burning as f32).sqrt() / 12.0).min(0.6), muffle: 0.1, delay: 0.0 });
        }
        out.send(cmds);
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = now - self.last;
        self.last = now;
        if !self.paused {
            self.accumulator += dt;
            let mut n = 0;
            while self.accumulator >= TICK && n < MAX_TICKS_PER_FRAME {
                self.game.step();
                self.accumulator -= TICK;
                n += 1;
            }
            if n == MAX_TICKS_PER_FRAME {
                self.accumulator = Duration::ZERO;
            }
        }
        self.sounds();
        let Some(pixels) = &mut self.pixels else { return };
        paint::paint(&self.game, pixels.frame_mut(), self.cursor);
        if self.paused {
            paint::text(pixels.frame_mut(), FW as i32 / 2 - 36, 200, "PAUSED", [255, 255, 255, 255]);
        }
        if let Some(n) = self.shot {
            if n <= 1 {
                self.shot = None;
                let path = std::env::temp_dir().join("pixel_physics_firedamp.png");
                match image::save_buffer(&path, pixels.frame(), FW as u32, FH as u32, image::ColorType::Rgba8) {
                    Ok(()) => eprintln!("firedamp screenshot saved: {}", path.display()),
                    Err(e) => eprintln!("firedamp screenshot failed: {e}"),
                }
            } else {
                self.shot = Some(n - 1);
            }
        }
        if let Err(e) = pixels.render() {
            eprintln!("render failed: {e}");
            event_loop.exit();
        }
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("Firedamp")
            .with_inner_size(LogicalSize::new(FW as u32, FH as u32))
            .with_min_inner_size(LogicalSize::new(FW as u32 / 2, FH as u32 / 2));
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("window creation failed: {e}");
                event_loop.exit();
                return;
            }
        };
        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width, size.height, Arc::clone(&window));
        match Pixels::new(FW as u32, FH as u32, surface) {
            Ok(p) => self.pixels = Some(p),
            Err(e) => {
                eprintln!("pixels init failed: {e}");
                event_loop.exit();
                return;
            }
        }
        self.window = Some(window);
        self.last = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(p) = &mut self.pixels {
                    if p.resize_surface(size.width, size.height).is_err() {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let px = self.pixels.as_ref().and_then(|p| p.window_pos_to_pixel(position.into()).ok());
                self.screen_cursor = px.map(|(x, y)| (x as i32, y as i32));
                self.cursor = px.and_then(|(x, y)| {
                    let (cx, cy) = ((x / S) as i32, (y / S) as i32);
                    gas::inside(cx, cy).then_some((cx, cy))
                });
                if self.down {
                    if let Some((x, y)) = self.cursor {
                        self.game.drag(x, y);
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor = None;
                self.screen_cursor = None;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                match (button, pressed) {
                    (MouseButton::Left, true) => {
                        if let Some(t) = self.screen_cursor.and_then(|(x, y)| paint::button_at(x, y)) {
                            self.game.tool = t;
                        } else if let Some((x, y)) = self.cursor {
                            self.down = true;
                            self.game.press(x, y);
                        }
                    }
                    (MouseButton::Left, false) => {
                        if self.down {
                            self.down = false;
                            match self.cursor {
                                Some((x, y)) => self.game.release(x, y),
                                None => self.game.aiming = None,
                            }
                        }
                    }
                    (MouseButton::Right, true) => {
                        if let Some((x, y)) = self.cursor {
                            self.game.pick_up(x, y);
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && !event.repeat {
                    if let PhysicalKey::Code(code) = event.physical_key {
                        self.key(code, event_loop);
                    }
                }
            }
            WindowEvent::RedrawRequested => self.frame(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}
