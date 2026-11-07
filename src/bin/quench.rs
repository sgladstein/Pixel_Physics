//! **Quench: aim the roof, not the water.** `cargo run --release --bin quench`
//!
//! The window and event loop for probe 5 of the 2026-11-07 new-game
//! brainstorm. Everything the game does to a world lives in
//! `pixel_physics::quench`; this file is winit and `pixels` and nothing else,
//! the same split `src/bin/druid.rs` uses.
//!
//! Keys (also on screen): `1` Splash, `2` Lid, `3` Dig, left click uses it;
//! `F` turns the steam-drips-under-lids rule on and off; `R` restarts the
//! same round; `N` starts a new seed; `Esc` quits.
//!
//! `PIXEL_PHYSICS_QUENCH_SEED=<n>` picks the seed.

use std::sync::Arc;
use std::time::{Duration, Instant};

use pixel_physics::quench::{Quench, Tool, FRAME_H, FRAME_W, H, SCALE, TICKS_PER_SECOND, W};
use pixel_physics::render::Renderer;
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// Fixed simulation rate, for the reason `main.rs` gives: every CA rule is
/// one cell per step, so a variable step changes behaviour with frame rate.
const TICK: Duration = Duration::from_nanos(1_000_000_000 / TICKS_PER_SECOND);
/// Ceiling on catch-up ticks per drawn frame.
const MAX_TICKS_PER_FRAME: u32 = 4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let seed = std::env::var("PIXEL_PHYSICS_QUENCH_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut handler = Handler::new(seed);
    event_loop.run_app(&mut handler)?;
    handler.result
}

struct Handler {
    game: Quench,
    seed: u64,
    renderer: Renderer,
    world_buf: Vec<u8>,
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    cursor: Option<(i32, i32)>,
    held: bool,
    last_frame: Instant,
    owed: Duration,
    result: Result<(), Box<dyn std::error::Error>>,
}

impl Handler {
    fn new(seed: u64) -> Self {
        println!("quench: seed {seed}. 1 splash, 2 lid, 3 dig, click to use; F drip rule; R restart; N new seed");
        Self {
            game: Quench::new(seed),
            seed,
            renderer: Renderer::new(),
            world_buf: vec![0; (W * H * 4) as usize],
            window: None,
            pixels: None,
            cursor: None,
            held: false,
            last_frame: Instant::now(),
            owed: Duration::ZERO,
            result: Ok(()),
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, msg: String) {
        eprintln!("quench: {msg}");
        self.result = Err(msg.into());
        event_loop.exit();
    }

    fn restart(&mut self, seed: u64) {
        let fast = self.game.fast_drip;
        let tool = self.game.tool;
        self.seed = seed;
        self.game = Quench::new(seed);
        self.game.fast_drip = fast;
        self.game.tool = tool;
        self.renderer = Renderer::new();
        println!("quench: round restarted, seed {seed}");
    }

    fn click(&mut self) {
        if let Some((x, y)) = self.cursor {
            let tool = self.game.tool;
            if self.game.act(tool, x, y) {
                println!("quench: {} at ({x},{y}) t={:.1}s", tool.name(), self.game.seconds());
            }
        }
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("Quench")
            .with_inner_size(LogicalSize::new(FRAME_W, FRAME_H))
            .with_min_inner_size(LogicalSize::new(FRAME_W / 2, FRAME_H / 2));
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(err) => return self.fail(event_loop, format!("window creation failed: {err}")),
        };
        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width, size.height, Arc::clone(&window));
        match Pixels::new(FRAME_W, FRAME_H, surface) {
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
                        KeyCode::Digit1 => self.game.tool = Tool::Splash,
                        KeyCode::Digit2 => self.game.tool = Tool::Lid,
                        KeyCode::Digit3 => self.game.tool = Tool::Dig,
                        KeyCode::KeyF => {
                            self.game.fast_drip = !self.game.fast_drip;
                            println!("quench: steam drips under lids {}", if self.game.fast_drip { "ON" } else { "OFF (shipped engine)" });
                        }
                        KeyCode::KeyR => self.restart(self.seed),
                        KeyCode::KeyN => self.restart(self.seed + 1),
                        KeyCode::Escape => event_loop.exit(),
                        _ => {}
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = self
                    .pixels
                    .as_ref()
                    .and_then(|p| p.window_pos_to_pixel((position.x as f32, position.y as f32)).ok())
                    .map(|(x, y)| (x as i32 / SCALE, y as i32 / SCALE))
                    .filter(|&(x, y)| x < W && y < H);
                // Dig is a drag: holding the button keeps digging.
                if self.held && self.game.tool == Tool::Dig {
                    self.click();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor = None;
                self.held = false;
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                self.held = state == ElementState::Pressed;
                if self.held {
                    self.click();
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                self.owed += now - self.last_frame;
                self.last_frame = now;
                let mut ticks = 0;
                while self.owed >= TICK && ticks < MAX_TICKS_PER_FRAME {
                    self.game.step();
                    self.owed -= TICK;
                    ticks += 1;
                }
                if ticks == MAX_TICKS_PER_FRAME {
                    self.owed = Duration::ZERO;
                }
                let Some(pixels) = self.pixels.as_mut() else { return };
                self.game.draw_frame(&mut self.renderer, &mut self.world_buf, pixels.frame_mut(), self.cursor);
                if let Err(err) = pixels.render() {
                    self.fail(event_loop, format!("render failed: {err}"));
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}
