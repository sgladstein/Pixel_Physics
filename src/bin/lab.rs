//! **The evolution lab.** `cargo run --release --bin lab`
//!
//! A second game against the same engine, not a fork of it — the structure
//! `src/lib.rs` was written for and `Reports/evolution-lab-design-guide-
//! 2026-08-30.md` §7a argues for at length. Everything with behaviour lives
//! in `pixel_physics::lab` and `pixel_physics::sim`; this file knows winit and
//! pixels exist and nothing else does, which is the same seam `src/main.rs`
//! keeps.
//!
//! **The frame loop is the game's core dial and it is not `main.rs`'s.** The
//! sandbox runs a fixed timestep with a hard `MAX_TICKS_PER_FRAME = 5`
//! catch-up ceiling, because there a backlog means the machine stalled. Here a
//! backlog is the *point*: Running asks for many ticks per displayed frame
//! deliberately. So the ceiling moves from a tick count to a wall clock, and
//! it lives in `lab::time` beside the readout, so what is on screen and what
//! the loop did cannot disagree.
//!
//! Keys:
//!
//! | | |
//! |---|---|
//! | `Space` | Tending ⇄ Running |
//! | `Up` / `Down` | the speed dial, through the presets |
//! | `1`-`6` | jump straight to a preset |
//! | `F1` / `F2` / `F3` | the plants, ants and box pages |
//! | `Tab` | the stats page |
//! | `WASD` / drag | pan; `-` / `=` zoom |
//! | `R` | rebuild the box |
//! | `Esc` | quit |
//!
//! **And none of them is the only way in.** `lab::ui` draws every one of the
//! above as a button along the bottom of the screen with its key printed
//! under it, so the lab can be driven with the mouse alone — owner request,
//! 2026-08-30, *"It shouldn't all be keyboard shortcuts."* The keys are
//! unchanged; the bar is additive. Both routes go through `Lab::act`, so
//! there is one definition of what each control does.
//!
//! `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=N` dumps the framebuffer to
//! `%TEMP%/pixel_physics_lab.png` after N rendered frames and exits nothing —
//! the hook that makes this binary checkable on a headless box, where
//! `labshot` can only render the world and never the interface drawn over
//! it.

use std::sync::Arc;
use std::time::{Duration, Instant};

use pixel_physics::lab::time::Phase;
use pixel_physics::lab::ui::{Action, Panel};
use pixel_physics::lab::{scene::LabBox, Lab, HEIGHT, WIDTH};
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

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
    lab: Lab,
    last_frame: Instant,
    last_title: Instant,
    fps: f32,
    cursor: Option<(i32, i32)>,
    held: Held,
    /// Real rendered frames left before a one-shot framebuffer dump, from
    /// `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES`. The same hook `main.rs`
    /// carries, and for the same reason: this build's swapchain is not
    /// visible to OS screen capture, so getting a look at an actual rendered
    /// window means dumping the framebuffer the app already holds. It is also
    /// what makes the lab verifiable at all on a headless box -- `labshot`
    /// renders the *world*, and only this renders the world **plus the HUD,
    /// the key page and the stats panel**, which is most of what this binary
    /// is. Cleared once fired, so it never repeats.
    screenshot_countdown: Option<u32>,
    result: Result<(), Box<dyn std::error::Error>>,
}

#[derive(Default)]
struct Held {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

impl Handler {
    fn new() -> Self {
        Self {
            window: None,
            pixels: None,
            lab: Lab::new(LabBox::default()),
            last_frame: Instant::now(),
            last_title: Instant::now(),
            fps: 0.0,
            cursor: None,
            held: Held::default(),
            screenshot_countdown: std::env::var("PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES")
                .ok()
                .and_then(|v| v.parse().ok()),
            result: Ok(()),
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        self.result = Err(message.into());
        event_loop.exit();
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame);
        self.last_frame = now;
        let dt = elapsed.as_secs_f32().max(1e-6);
        self.fps += (1.0 / dt - self.fps) * 0.1;

        // The view pans on the same keys the sandbox scrolls with. Read from
        // held state rather than edges, and applied here rather than in
        // `advance`, because the camera is view state: it belongs on the
        // rendered frame, not inside the catch-up loop where it would run
        // once per tick and scroll at the speed of the dial.
        let dir = (
            self.held.right as i32 - self.held.left as i32,
            self.held.down as i32 - self.held.up as i32,
        );
        if dir != (0, 0) {
            let bounds = self.lab.world.bounds();
            self.lab.renderer.pan(dir, dt.min(0.1), (WIDTH, HEIGHT), bounds);
        }

        self.lab.advance(elapsed);

        let render_error = match &mut self.pixels {
            Some(pixels) => {
                self.lab.draw(pixels.frame_mut(), self.fps);
                if let Some(n) = self.screenshot_countdown {
                    if n <= 1 {
                        self.screenshot_countdown = None;
                        let path = std::env::temp_dir().join("pixel_physics_lab.png");
                        match image::save_buffer(
                            &path,
                            pixels.frame(),
                            WIDTH,
                            HEIGHT,
                            image::ColorType::Rgba8,
                        ) {
                            Ok(()) => eprintln!("lab screenshot saved: {}", path.display()),
                            Err(e) => eprintln!("lab screenshot failed: {e}"),
                        }
                    } else {
                        self.screenshot_countdown = Some(n - 1);
                    }
                }
                pixels.render().err().map(|e| format!("render failed: {e}"))
            }
            None => None,
        };
        if let Some(message) = render_error {
            return self.fail(event_loop, message);
        }

        if now.duration_since(self.last_title) >= Duration::from_millis(250) {
            self.last_title = now;
            if let Some(window) = &self.window {
                window.set_title(&format!(
                    "Evolution Lab — {} — {:.0} fps",
                    match self.lab.time.phase {
                        Phase::Tending => "tending".to_string(),
                        Phase::Running => format!("running {:.1}x", self.lab.time.achieved.max(0.0)),
                    },
                    self.fps
                ));
            }
        }
    }

    fn key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, pressed: bool) {
        // Held state first, and only `WASD` — the arrow keys drive the dial,
        // so a pan reading of them would scroll the view every time the
        // player changed speed.
        match code {
            KeyCode::KeyA => self.held.left = pressed,
            KeyCode::KeyD => self.held.right = pressed,
            KeyCode::KeyW => self.held.up = pressed,
            KeyCode::KeyS => self.held.down = pressed,
            _ => {}
        }
        if !pressed {
            return;
        }
        // Any key dismisses the opening key list, so it is never in the way —
        // except `?`, which toggles it, and `Escape`, which quits.
        if self.lab.show_help && code != KeyCode::Escape {
            self.lab.show_help = false;
            if code == KeyCode::Slash {
                return;
            }
        }
        match code {
            KeyCode::Escape => event_loop.exit(),
            KeyCode::Slash => self.lab.show_help = !self.lab.show_help,
            KeyCode::Space => self.lab.time.toggle_phase(),
            KeyCode::ArrowUp => self.lab.time.faster(),
            KeyCode::ArrowDown => self.lab.time.slower(),
            KeyCode::Digit1 => self.lab.time.set_preset(0),
            KeyCode::Digit2 => self.lab.time.set_preset(1),
            KeyCode::Digit3 => self.lab.time.set_preset(2),
            KeyCode::Digit4 => self.lab.time.set_preset(3),
            KeyCode::Digit5 => self.lab.time.set_preset(4),
            KeyCode::Digit6 => self.lab.time.set_preset(5),
            KeyCode::F1 => self.lab.act(Action::Panel(Panel::Plants)),
            KeyCode::F2 => self.lab.act(Action::Panel(Panel::Ants)),
            KeyCode::F3 => self.lab.act(Action::Panel(Panel::Box)),
            KeyCode::Tab => self.lab.stats.toggle(),
            KeyCode::KeyR => self.lab.reset(),
            KeyCode::Minus => self.lab.renderer.adjust_zoom(-1),
            KeyCode::Equal => self.lab.renderer.adjust_zoom(1),
            _ => {}
        }
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("Evolution Lab")
            .with_inner_size(LogicalSize::new(WIDTH * 2, HEIGHT * 2))
            .with_min_inner_size(LogicalSize::new(WIDTH, HEIGHT));
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(err) => return self.fail(event_loop, format!("window creation failed: {err}")),
        };
        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width, size.height, Arc::clone(&window));
        match Pixels::new(WIDTH, HEIGHT, surface) {
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
                if let Some(pixels) = &mut self.pixels {
                    if pixels.resize_surface(size.width, size.height).is_err() {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = self
                    .pixels
                    .as_ref()
                    .and_then(|p| p.window_pos_to_pixel(position.into()).ok())
                    .map(|(x, y)| (x as i32, y as i32));
                self.lab.set_cursor(self.cursor);
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor = None;
                self.lab.set_cursor(None);
            }
            // The one event this binary did not handle at all until the bar
            // existed. Press and release are kept apart deliberately: a button
            // fires on release over itself, so a press can be taken back by
            // sliding off it, and `REBUILD` cannot throw the box away on the
            // way past.
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    match (state == ElementState::Pressed, self.cursor) {
                        (true, Some((x, y))) => self.lab.press(x, y),
                        (false, Some((x, y))) => self.lab.release(x, y),
                        // Released with the pointer outside the window: the
                        // gesture is abandoned, not aimed at whatever it was
                        // last over.
                        (false, None) => self.lab.ui.cancel_press(),
                        (true, None) => {}
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let up = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y > 0.0,
                    MouseScrollDelta::PixelDelta(p) => p.y > 0.0,
                };
                self.lab.renderer.adjust_zoom(if up { 1 } else { -1 });
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    self.key(event_loop, code, event.state == ElementState::Pressed);
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
