//! **Heartwood.** `cargo run --release --bin heartwood` (optionally `-- seed=N`)
//!
//! Probe 8 of the 2026-11-07 new-game brainstorm: two trees in one valley,
//! and yours grows only where you play a card. Everything the game does to a
//! world lives in `pixel_physics::heartwood`; this file is winit and `pixels`
//! and nothing else, the split `src/bin/druid.rs` uses.
//!
//! **Fixed timestep**, for the reason `main.rs` gives: every CA rule is one
//! cell per step, so a variable step changes behaviour with the frame rate.
//!
//! Headless capture, the same hook the other binaries carry:
//! `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=N PIXEL_PHYSICS_SCREENSHOT_PATH=x.png`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use pixel_physics::heartwood::hud::{self, Screen, FRAME_H, FRAME_W};
use pixel_physics::heartwood::Heartwood;
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

const TICK: Duration = Duration::from_nanos(1_000_000_000 / 60);
const MAX_TICKS_PER_FRAME: u32 = 4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let seed = std::env::args().find_map(|a| a.strip_prefix("seed=").and_then(|v| v.parse().ok())).unwrap_or(1);
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut handler = Handler {
        window: None,
        pixels: None,
        game: Heartwood::new(seed),
        screen: Screen::new(),
        seed,
        last_frame: Instant::now(),
        accumulator: Duration::ZERO,
        cursor: None,
        screenshot_countdown: std::env::var("PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES").ok().and_then(|v| v.parse().ok()),
        result: Ok(()),
    };
    event_loop.run_app(&mut handler)?;
    handler.result
}

struct Handler {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    game: Heartwood,
    screen: Screen,
    seed: u64,
    last_frame: Instant,
    accumulator: Duration,
    /// The pointer, in frame pixels.
    cursor: Option<(i32, i32)>,
    screenshot_countdown: Option<u32>,
    result: Result<(), Box<dyn std::error::Error>>,
}

impl Handler {
    fn fail(&mut self, event_loop: &ActiveEventLoop, msg: String) {
        self.result = Err(msg.into());
        event_loop.exit();
    }

    fn new_valley(&mut self) {
        self.seed += 1;
        self.game = Heartwood::new(self.seed);
        self.screen = Screen::new();
        self.last_frame = Instant::now();
        self.accumulator = Duration::ZERO;
    }

    fn click(&mut self, button: MouseButton) {
        let Some(p) = self.cursor else { return };
        if button == MouseButton::Left {
            if let Some(i) = hud::card_under(&self.game, p) {
                self.game.play(i);
                return;
            }
        }
        let Some(cell) = hud::to_cell(p) else { return };
        match button {
            MouseButton::Left => {
                if let Some(tag) = self.game.nearest_tip(cell, 8) {
                    self.game.select(tag);
                }
            }
            MouseButton::Right => {
                self.game.prune(cell);
            }
            _ => {}
        }
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.accumulator += now - self.last_frame;
        self.last_frame = now;
        let mut ticks = 0;
        while self.accumulator >= TICK && ticks < MAX_TICKS_PER_FRAME {
            self.game.tick();
            self.accumulator -= TICK;
            ticks += 1;
        }
        if ticks == MAX_TICKS_PER_FRAME {
            self.accumulator = Duration::ZERO;
        }
        let hover = self.cursor.and_then(|p| hud::card_under(&self.game, p));
        let Some(pixels) = self.pixels.as_mut() else { return };
        self.screen.draw(&mut self.game, pixels.frame_mut(), hover);
        if let Some(n) = self.screenshot_countdown.as_mut() {
            if *n == 0 {
                let path = std::env::var("PIXEL_PHYSICS_SCREENSHOT_PATH").unwrap_or_else(|_| "heartwood.png".to_string());
                let frame = pixels.frame().to_vec();
                if let Err(e) = image::save_buffer(&path, &frame, FRAME_W, FRAME_H, image::ColorType::Rgba8) {
                    eprintln!("heartwood: screenshot failed: {e}");
                } else {
                    println!("heartwood: wrote {path}");
                }
                event_loop.exit();
                return;
            }
            *n -= 1;
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
        let attributes = Window::default_attributes()
            .with_title("Heartwood")
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
                if let PhysicalKey::Code(code) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;
                    if !pressed || event.repeat {
                        return;
                    }
                    match code {
                        KeyCode::Digit1 => {
                            self.game.play(0);
                        }
                        KeyCode::Digit2 => {
                            self.game.play(1);
                        }
                        KeyCode::Digit3 => {
                            self.game.play(2);
                        }
                        KeyCode::Digit4 => {
                            self.game.play(3);
                        }
                        KeyCode::Digit5 => {
                            self.game.play(4);
                        }
                        KeyCode::KeyN => self.new_valley(),
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
                    .map(|(x, y)| (x as i32, y as i32));
            }
            WindowEvent::CursorLeft { .. } => self.cursor = None,
            WindowEvent::MouseInput { state: ElementState::Pressed, button, .. } => self.click(button),
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
