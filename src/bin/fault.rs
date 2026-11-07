//! **Fault: crack the ice wall before the lake breaks it.**
//! `cargo run --release --bin fault` (optionally `-- seed=N`)
//!
//! A prototype (probe 6 of the 2026-11-07 new-game round), not a lab seat.
//! Everything it does to a world lives in `pixel_physics::fault`; this file
//! is winit and `pixels` and nothing else, the split `src/bin/druid.rs` uses.
//!
//! Keys: `1`-`4` pick Tap, Drill, Meltwater, Frost (or click the buttons);
//! click the ice to use the tool (hold for Meltwater); `R` restarts the same
//! wall, `N` deals a new one; `S` toggles the tension shimmer.
//!
//! `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=n` writes the frame buffer to
//! `fault-screenshot.png` after `n` drawn frames and exits -- the window's
//! swapchain is invisible to OS capture on a headless box, and the buffer is
//! what was drawn into it.

use std::sync::Arc;
use std::time::{Duration, Instant};

use pixel_physics::fault::view::{self, Ui, View};
use pixel_physics::fault::{Action, Game, Tool, FPS};
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// Fixed timestep: every rule in the engine is "one cell per step".
const TICK: Duration = Duration::from_nanos(1_000_000_000 / FPS as u64);
/// Ceiling on catch-up steps per drawn frame, so a stall does not snowball.
const MAX_TICKS_PER_FRAME: u32 = 4;
/// Steps between splashes while Meltwater is held.
const POUR_EVERY: u32 = 4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let seed = std::env::args().skip(1).find_map(|a| a.strip_prefix("seed=").and_then(|s| s.parse().ok())).unwrap_or(1);
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut handler = Handler::new(seed);
    event_loop.run_app(&mut handler)?;
    handler.result
}

struct Handler {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    game: Game,
    view: View,
    seed: u64,
    ui: Ui,
    /// The cursor, in world cells; `None` off the world.
    cell: Option<(i32, i32)>,
    /// The cursor, in buffer pixels.
    px: Option<(i32, i32)>,
    pressed: bool,
    pour_clock: u32,
    last_frame: Instant,
    accumulator: Duration,
    screenshot_countdown: Option<u32>,
    result: Result<(), Box<dyn std::error::Error>>,
}

impl Handler {
    fn new(seed: u64) -> Self {
        let screenshot_countdown = std::env::var("PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES").ok().and_then(|s| s.parse().ok());
        Handler {
            window: None,
            pixels: None,
            game: Game::new(seed),
            view: View::new(),
            seed,
            ui: Ui { tool: Tool::Tap, cursor: None, plain: false },
            cell: None,
            px: None,
            pressed: false,
            pour_clock: 0,
            last_frame: Instant::now(),
            accumulator: Duration::ZERO,
            screenshot_countdown,
            result: Ok(()),
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, msg: String) {
        self.result = Err(msg.into());
        event_loop.exit();
    }

    fn restart(&mut self, seed: u64) {
        self.seed = seed;
        self.game = Game::new(seed);
        self.pressed = false;
        self.accumulator = Duration::ZERO;
        self.last_frame = Instant::now();
        if let Some(w) = &self.window {
            w.set_title(&format!("Fault -- wall {seed}"));
        }
    }

    fn use_tool(&mut self) {
        if self.game.over() {
            return;
        }
        if let Some((x, y)) = self.cell {
            self.game.act(Action { tool: self.ui.tool, x, y });
        }
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.accumulator += now - self.last_frame;
        self.last_frame = now;
        let mut ticks = 0;
        while self.accumulator >= TICK && ticks < MAX_TICKS_PER_FRAME {
            self.accumulator -= TICK;
            ticks += 1;
            if !self.game.over() {
                if self.pressed && self.ui.tool == Tool::Meltwater {
                    self.pour_clock += 1;
                    if self.pour_clock.is_multiple_of(POUR_EVERY) {
                        self.use_tool();
                    }
                }
                self.game.step();
            }
        }
        if ticks == MAX_TICKS_PER_FRAME {
            self.accumulator = Duration::ZERO;
        }
        self.ui.cursor = self.cell;
        let Some(pixels) = self.pixels.as_mut() else { return };
        self.view.draw(&self.game, pixels.frame_mut(), self.ui);
        if let Some(n) = self.screenshot_countdown.as_mut() {
            *n = n.saturating_sub(1);
            if *n == 0 {
                let (w, h) = view::size();
                let saved = image::save_buffer("fault-screenshot.png", pixels.frame(), w, h, image::ColorType::Rgba8);
                println!("fault: screenshot -> fault-screenshot.png ({saved:?})");
                event_loop.exit();
            }
        }
        if let Err(err) = pixels.render() {
            self.fail(event_loop, format!("render failed: {err}"));
        }
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let (w, h) = view::size();
        let attributes = Window::default_attributes().with_title(format!("Fault -- wall {}", self.seed)).with_inner_size(LogicalSize::new(w, h)).with_min_inner_size(LogicalSize::new(w / 2, h / 2));
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(err) => return self.fail(event_loop, format!("window creation failed: {err}")),
        };
        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width, size.height, Arc::clone(&window));
        match Pixels::new(w, h, surface) {
            Ok(p) => self.pixels = Some(p),
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
                    let err = self.pixels.as_mut().and_then(|p| p.resize_surface(size.width, size.height).err());
                    if let Some(err) = err {
                        self.fail(event_loop, format!("surface resize failed: {err}"));
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed || event.repeat {
                    return;
                }
                if let PhysicalKey::Code(code) = event.physical_key {
                    match code {
                        KeyCode::Digit1 => self.ui.tool = Tool::Tap,
                        KeyCode::Digit2 => self.ui.tool = Tool::Drill,
                        KeyCode::Digit3 => self.ui.tool = Tool::Meltwater,
                        KeyCode::Digit4 => self.ui.tool = Tool::Frost,
                        KeyCode::KeyR => self.restart(self.seed),
                        KeyCode::KeyN => self.restart(self.seed + 1),
                        KeyCode::KeyS => self.game.shimmer = !self.game.shimmer,
                        KeyCode::Escape => event_loop.exit(),
                        _ => {}
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let px = self.pixels.as_ref().and_then(|p| p.window_pos_to_pixel((position.x as f32, position.y as f32)).ok()).map(|(x, y)| (x as i32, y as i32));
                self.px = px;
                self.cell = px.and_then(|(x, y)| {
                    let (cx, cy) = (x / view::SCALE as i32, y / view::SCALE as i32);
                    (cy < pixel_physics::fault::level::H).then_some((cx, cy))
                });
            }
            WindowEvent::CursorLeft { .. } => {
                self.px = None;
                self.cell = None;
                self.pressed = false;
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                let down = state == ElementState::Pressed;
                if down {
                    if let Some(t) = self.px.and_then(|(x, y)| view::tool_at(&self.game, x, y)) {
                        self.ui.tool = t;
                        return;
                    }
                    if self.game.over() {
                        return;
                    }
                    self.pressed = true;
                    self.pour_clock = 0;
                    self.use_tool();
                } else {
                    self.pressed = false;
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
