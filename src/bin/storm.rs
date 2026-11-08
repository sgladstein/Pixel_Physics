//! **Storm Glass** -- lightning that leaves a mark. `cargo run --release --bin storm`
//!
//! The window and event loop only. Everything the game does lives in
//! `pixel_physics::storm`, the same split `lab.rs` and `druid.rs` use, so
//! the headless probe (`examples/stormprobe.rs`) plays the exact game this
//! window shows.
//!
//! Keys (also on screen):
//! - hold **Space**: call lightning above the pointer; release to strike
//! - **click**: pour water at the pointer -- it puts out fire
//! - **right click**: drive an iron rod into the ground below the pointer
//!   (open ground under open sky only; anywhere else it says why and keeps
//!   the rod)
//! - **H**: show how to play again (it shows for the first ten seconds)
//! - **R**: start the storm again (a new seed)
//! - **Esc**: quit
//!
//! `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=N` writes the Nth drawn frame to
//! `pixel_physics_storm_screenshot.png` in the temp directory, the same hook
//! the other binaries carry.

use std::sync::Arc;
use std::time::{Duration, Instant};

use pixel_physics::storm::{draw::SCALE, Input, Storm, TICKS_PER_SECOND};
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
    let mut h = Handler::new();
    event_loop.run_app(&mut h)?;
    h.result
}

struct Handler {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    game: Storm,
    seed: u64,
    last: Instant,
    acc: Duration,
    hold: bool,
    splash: bool,
    rod: bool,
    help: bool,
    pointer: (i32, i32),
    shot_after: Option<u32>,
    drawn: u32,
    result: Result<(), Box<dyn std::error::Error>>,
}

impl Handler {
    fn new() -> Self {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_secs() % 100_000);
        Handler {
            window: None,
            pixels: None,
            game: Storm::new(seed),
            seed,
            last: Instant::now(),
            acc: Duration::ZERO,
            hold: false,
            splash: false,
            rod: false,
            help: false,
            pointer: (128, 30),
            shot_after: std::env::var("PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES").ok().and_then(|v| v.parse().ok()),
            drawn: 0,
            result: Ok(()),
        }
    }

    fn fail(&mut self, el: &ActiveEventLoop, msg: String) {
        self.result = Err(msg.into());
        el.exit();
    }

    fn frame(&mut self) {
        let now = Instant::now();
        self.acc += now - self.last;
        self.last = now;
        let mut n = 0;
        while self.acc >= TICK && n < MAX_TICKS_PER_FRAME {
            let input = Input { at: self.pointer, hold: self.hold, splash: self.splash, rod: self.rod, help: self.help };
            // Edges are consumed by the first tick that sees them.
            self.splash = false;
            self.rod = false;
            self.help = false;
            self.game.update(input);
            self.acc -= TICK;
            n += 1;
        }
        if n == MAX_TICKS_PER_FRAME {
            self.acc = Duration::ZERO;
        }
        let Some(p) = self.pixels.as_mut() else { return };
        self.game.draw(p.frame_mut());
        self.drawn += 1;
        if self.shot_after == Some(self.drawn) {
            let (w, h) = Storm::frame_size();
            let path = std::env::temp_dir().join("pixel_physics_storm_screenshot.png");
            if let Some(img) = image::RgbaImage::from_raw(w, h, p.frame().to_vec()) {
                let _ = img.save(&path);
                eprintln!("storm: screenshot written to {}", path.display());
            }
        }
        if let Err(e) = p.render() {
            eprintln!("storm: render failed: {e}");
        }
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let (w, h) = Storm::frame_size();
        let attrs = Window::default_attributes()
            .with_title("Storm Glass")
            .with_inner_size(LogicalSize::new(w, h))
            .with_min_inner_size(LogicalSize::new(w / 2, h / 2));
        let window = match el.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => return self.fail(el, format!("window creation failed: {e}")),
        };
        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width, size.height, Arc::clone(&window));
        match Pixels::new(w, h, surface) {
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
                    let err = self.pixels.as_mut().and_then(|p| p.resize_surface(size.width, size.height).err());
                    if let Some(e) = err {
                        self.fail(el, format!("surface resize failed: {e}"));
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(p) = &self.pixels {
                    let (x, y) = p.window_pos_to_pixel((position.x as f32, position.y as f32)).unwrap_or_else(|pos| p.clamp_pixel_pos(pos));
                    self.pointer = (x as i32 / SCALE, y as i32 / SCALE);
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button, .. } => match button {
                MouseButton::Left => self.splash = true,
                MouseButton::Right => self.rod = true,
                _ => {}
            },
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    let down = event.state == ElementState::Pressed;
                    match code {
                        KeyCode::Space => self.hold = down,
                        KeyCode::KeyR if down && !event.repeat => {
                            self.seed += 1;
                            self.game = Storm::new(self.seed);
                            self.hold = false;
                        }
                        KeyCode::KeyH if down && !event.repeat => self.help = true,
                        KeyCode::Escape if down => el.exit(),
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => self.frame(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}
