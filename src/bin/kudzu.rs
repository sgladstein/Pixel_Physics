//! **Kudzu: a vine is eating the village.** `cargo run --release --bin kudzu`
//!
//! A prototype. The game is `pixel_physics::kudzu`; this file is winit and
//! `pixels` and nothing else, the split `src/bin/druid.rs` uses.
//!
//! Hover over the vine with the shears and everything that would fall if
//! you cut there lights up pale. Click to cut. Hold the dig over the roots
//! for three seconds to pull a crown.
//!
//! Keys (also on screen): `1` shears, `2` dig, `R` restart. Headless:
//! `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=N` writes
//! `pixel_physics_kudzu_screenshot.png` to the temp directory after N drawn
//! frames; `KUDZU_HOVER=x,y` fixes the pointer there for it.

use std::sync::Arc;
use std::time::{Duration, Instant};

use pixel_physics::kudzu::hud::{self, FRAME_H, FRAME_W};
use pixel_physics::kudzu::{Kudzu, Level, Tool, HEIGHT, TICKS_PER_SECOND, WIDTH};
use pixel_physics::render::Renderer;
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

const TICK: Duration = Duration::from_nanos(1_000_000_000 / TICKS_PER_SECOND as u64);
/// Ceiling on catch-up ticks per frame, so one stall does not snowball.
const MAX_TICKS_PER_FRAME: u32 = 4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut handler = Handler::new();
    event_loop.run_app(&mut handler)?;
    handler.result
}

struct Handler {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    game: Kudzu,
    level: Level,
    seed: u64,
    renderer: Renderer,
    cells: Vec<u8>,
    last_frame: Instant,
    accumulator: Duration,
    /// The pointer, in world cells, when it is over the world.
    hover: Option<(i32, i32)>,
    held: bool,
    screenshot_countdown: Option<u32>,
    result: Result<(), Box<dyn std::error::Error>>,
}

impl Handler {
    fn new() -> Self {
        let seed = std::env::var("KUDZU_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
        let level = Level::Walls;
        let hover = std::env::var("KUDZU_HOVER").ok().and_then(|v| {
            let (x, y) = v.split_once(',')?;
            Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
        });
        Self {
            window: None,
            pixels: None,
            game: Kudzu::new(level, seed),
            level,
            seed,
            renderer: Renderer::new(),
            cells: vec![0; (WIDTH * HEIGHT * 4) as usize],
            last_frame: Instant::now(),
            accumulator: Duration::ZERO,
            hover,
            held: false,
            screenshot_countdown: std::env::var("PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES").ok().and_then(|v| v.parse().ok()),
            result: Ok(()),
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        self.result = Err(message.into());
        event_loop.exit();
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.accumulator += now.duration_since(self.last_frame);
        self.last_frame = now;
        let mut ticks = 0;
        while self.accumulator >= TICK && ticks < MAX_TICKS_PER_FRAME {
            self.accumulator -= TICK;
            ticks += 1;
            // A held dig counts per simulated frame, so three seconds is
            // three seconds of world time.
            if self.held && self.game.tool == Tool::Dig {
                if let Some((x, y)) = self.hover {
                    self.game.hold_dig(x, y);
                }
            }
            self.game.tick();
        }
        if ticks == MAX_TICKS_PER_FRAME {
            self.accumulator = Duration::ZERO;
        }
        let Some(pixels) = self.pixels.as_mut() else { return };
        hud::draw_frame(&mut self.game, &mut self.renderer, &mut self.cells, pixels.frame_mut(), self.hover);
        if let Some(n) = self.screenshot_countdown {
            if n <= 1 {
                self.screenshot_countdown = None;
                let path = std::env::temp_dir().join("pixel_physics_kudzu_screenshot.png");
                match image::save_buffer(&path, pixels.frame(), FRAME_W, FRAME_H, image::ColorType::Rgba8) {
                    Ok(()) => println!("kudzu: wrote {}", path.display()),
                    Err(e) => eprintln!("kudzu: screenshot failed: {e}"),
                }
            } else {
                self.screenshot_countdown = Some(n - 1);
            }
        }
        if let Err(err) = pixels.render() {
            self.fail(event_loop, format!("render failed: {err}"));
        }
    }

    fn key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Digit1 => self.game.tool = Tool::Snip,
            KeyCode::Digit2 => self.game.tool = Tool::Dig,
            KeyCode::KeyR => {
                let tool = self.game.tool;
                self.game = Kudzu::new(self.level, self.seed);
                self.game.tool = tool;
                self.last_frame = Instant::now();
            }
            _ => {}
        }
    }

    fn click(&mut self) {
        if self.game.over() {
            return;
        }
        if let (Tool::Snip, Some((x, y))) = (self.game.tool, self.hover) {
            self.game.snip(x, y);
        }
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("Kudzu")
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
                    if event.state == ElementState::Pressed && !event.repeat {
                        self.key(code);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.hover = self
                    .pixels
                    .as_ref()
                    .and_then(|p| p.window_pos_to_pixel((position.x as f32, position.y as f32)).ok())
                    .map(|(x, y)| (x as i32 / hud::SCALE as i32, y as i32 / hud::SCALE as i32))
                    .filter(|&(_, y)| y < HEIGHT);
            }
            WindowEvent::CursorLeft { .. } => {
                self.hover = None;
                self.held = false;
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                self.held = state == ElementState::Pressed;
                if self.held {
                    self.click();
                } else {
                    self.game.digging = None;
                }
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
