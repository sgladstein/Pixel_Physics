//! **Seep** -- `cargo run --release --bin seep`. Probe 4 of the 2026-11-07
//! new-game brainstorm: a mine leaking gas, and a slime mould that eats it.
//!
//! Window and input only; the game is `pixel_physics::seep`. Keys are on
//! screen in the tool bar: 1-5 pick a tool (Blow, Spark, Damp, Cut, Fuse),
//! the left mouse button uses it, R restarts, N tries the next seed, Space
//! pauses, H shows the help card. The card (goal, keys, stars) is up when
//! the game opens and the mine waits under it; any key or click starts.
//!
//! `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=N` saves `seep-screenshot.png`
//! after N drawn frames and exits, the same hook the other binaries carry.

use std::sync::Arc;
use std::time::{Duration, Instant};

use pixel_physics::seep::draw::{self, FH, FW, SCALE};
use pixel_physics::seep::{Act, Game, Tool, TICKS_PER_SECOND};
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

const TICK: Duration = Duration::from_nanos(1_000_000_000 / TICKS_PER_SECOND as u64);
const MAX_TICKS_PER_FRAME: u32 = 4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let seed = std::env::var("SEEP_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut h = Handler {
        window: None,
        pixels: None,
        game: Game::new(seed),
        seed,
        tool: Tool::Blow,
        cursor: None,
        last_cursor: None,
        down: false,
        drag_dir: (1.0, 0.0),
        paused: false,
        // The screenshot hook wants the mine, not the card.
        show_help: std::env::var("PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES").is_err() || std::env::var("SEEP_HELP").is_ok(),
        last: Instant::now(),
        acc: Duration::ZERO,
        frames: 0,
        shot_after: std::env::var("PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES").ok().and_then(|s| s.parse().ok()),
        result: Ok(()),
    };
    event_loop.run_app(&mut h)?;
    h.result
}

struct Handler {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    game: Game,
    seed: u64,
    tool: Tool,
    /// Cursor in cells, or in the bar (y >= H).
    cursor: Option<(f32, f32)>,
    last_cursor: Option<(f32, f32)>,
    down: bool,
    drag_dir: (f32, f32),
    paused: bool,
    /// The help card is over the mine, and the mine is waiting.
    show_help: bool,
    last: Instant,
    acc: Duration,
    frames: u32,
    shot_after: Option<u32>,
    result: Result<(), Box<dyn std::error::Error>>,
}

impl Handler {
    fn fail(&mut self, el: &ActiveEventLoop, msg: String) {
        self.result = Err(msg.into());
        el.exit();
    }

    /// What the held button does this tick.
    fn acts(&mut self) -> Vec<Act> {
        let Some((cx, cy)) = self.cursor else { return Vec::new() };
        if !self.down || cy >= pixel_physics::seep::gas::H as f32 {
            return Vec::new();
        }
        let (x, y) = (cx as i32, cy as i32);
        if let Some((lx, ly)) = self.last_cursor {
            let (dx, dy) = (cx - lx, cy - ly);
            if dx * dx + dy * dy > 0.04 {
                let l = (dx * dx + dy * dy).sqrt();
                self.drag_dir = (dx / l, dy / l);
            }
        }
        self.last_cursor = Some((cx, cy));
        match self.tool {
            Tool::Blow => vec![Act::Blow { x: cx, y: cy, dx: self.drag_dir.0, dy: self.drag_dir.1 }],
            Tool::Damp => vec![Act::Damp { x, y }],
            Tool::Cut => vec![Act::Cut { x, y }],
            // Taps: one per press.
            Tool::Spark => {
                self.down = false;
                vec![Act::Spark { x, y }]
            }
            Tool::Fuse => {
                self.down = false;
                vec![Act::Fuse { x, y }]
            }
        }
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Seep: keep the gas off the lamps")
            .with_inner_size(LogicalSize::new(FW as u32 * 2, FH as u32 * 2))
            .with_min_inner_size(LogicalSize::new(FW as u32, FH as u32));
        let window = match el.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => return self.fail(el, format!("window creation failed: {e}")),
        };
        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width, size.height, Arc::clone(&window));
        match Pixels::new(FW as u32, FH as u32, surface) {
            Ok(p) => self.pixels = Some(p),
            Err(e) => return self.fail(el, format!("pixels init failed: {e}")),
        }
        self.window = Some(window);
        self.last = Instant::now();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    if let Some(Err(e)) = self.pixels.as_mut().map(|p| p.resize_surface(size.width, size.height)) {
                        self.fail(el, format!("surface resize failed: {e}"));
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                if let PhysicalKey::Code(code) = event.physical_key {
                    // With the card up, any key but Escape just starts.
                    if self.show_help && code != KeyCode::Escape {
                        self.show_help = false;
                        return;
                    }
                    match code {
                        KeyCode::KeyH => self.show_help = true,
                        KeyCode::Digit1 => self.tool = Tool::Blow,
                        KeyCode::Digit2 => self.tool = Tool::Spark,
                        KeyCode::Digit3 => self.tool = Tool::Damp,
                        KeyCode::Digit4 => self.tool = Tool::Cut,
                        KeyCode::Digit5 => self.tool = Tool::Fuse,
                        KeyCode::KeyR => self.game = Game::new(self.seed),
                        KeyCode::KeyN => {
                            self.seed += 1;
                            self.game = Game::new(self.seed);
                        }
                        KeyCode::Space => self.paused = !self.paused,
                        KeyCode::Escape => el.exit(),
                        _ => {}
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = self
                    .pixels
                    .as_ref()
                    .and_then(|p| p.window_pos_to_pixel((position.x as f32, position.y as f32)).ok())
                    .map(|(x, y)| (x as f32 / SCALE as f32, y as f32 / SCALE as f32));
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor = None;
                self.down = false;
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                if self.show_help {
                    if state == ElementState::Pressed {
                        self.show_help = false;
                    }
                    self.down = false;
                    return;
                }
                self.down = state == ElementState::Pressed;
                self.last_cursor = None;
                // A press on the bar picks a tool.
                if self.down {
                    if let Some((cx, cy)) = self.cursor {
                        let h = pixel_physics::seep::gas::H as f32;
                        if cy >= h {
                            let k = ((cx * SCALE as f32 - 4.0) / 70.0) as usize;
                            if let Some(t) = Tool::ALL.get(k) {
                                self.tool = *t;
                            }
                            self.down = false;
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                self.acc += now - self.last;
                self.last = now;
                let mut n = 0;
                while self.acc >= TICK && n < MAX_TICKS_PER_FRAME {
                    self.acc -= TICK;
                    n += 1;
                    if !self.paused && !self.show_help && !self.game.over() {
                        let acts = self.acts();
                        self.game.step(&acts);
                    }
                }
                if n == MAX_TICKS_PER_FRAME {
                    self.acc = Duration::ZERO;
                }
                let cursor = self.cursor.map(|(x, y)| (x as i32, y as i32));
                let (game, tool) = (&self.game, self.tool);
                if let Some(p) = self.pixels.as_mut() {
                    draw::render(game, p.frame_mut(), cursor, tool);
                    if self.show_help {
                        draw::help(p.frame_mut());
                    }
                    if let Some(after) = self.shot_after {
                        if self.frames >= after {
                            let _ = image::save_buffer("seep-screenshot.png", p.frame(), FW as u32, FH as u32, image::ColorType::Rgba8);
                            el.exit();
                        }
                    }
                    if let Err(e) = p.render() {
                        return self.fail(el, format!("render failed: {e}"));
                    }
                }
                self.frames += 1;
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}
