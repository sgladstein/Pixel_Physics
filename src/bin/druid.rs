//! **The held world.** `cargo run --release --bin druid`
//!
//! The third game's window and event loop. Everything it does to a world
//! lives in `pixel_physics::druid`; this file is winit and `pixels` and
//! nothing else, the same split `src/bin/lab.rs` uses.
//!
//! **It takes `lab.rs`'s file shape and `main.rs`'s frame body**, and the
//! second half of that is not a preference. The lab's catch-up is a *speed
//! dial* driven off a wall clock, which is right for a box you are fast-
//! forwarding and wrong for a game somebody is walking around in;
//! `main.rs`'s fixed-timestep accumulator is the one that belongs here.
//!
//! Keys: `A`/`D` walk, `W` jump, `S` down, `Shift` hold on to a tree,
//! `P` pause, `H` release or re-hold the world, `L` how held ground is drawn,
//! `Esc` quit.

use std::sync::Arc;
use std::time::{Duration, Instant};

use pixel_physics::app::{HEIGHT, WIDTH};
use pixel_physics::druid::Druid;
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// The simulation advances at a fixed rate regardless of frame rate, for the
/// reason `main.rs` gives: every CA rule is "one cell per step" rather than a
/// velocity, so a variable timestep changes behaviour with the frame rate.
const TICKS_PER_SECOND: u32 = 60;
const TICK: Duration = Duration::from_nanos(1_000_000_000 / TICKS_PER_SECOND as u64);
/// Ceiling on catch-up ticks per frame — without it a stall makes the next
/// frame simulate the whole missing interval and stall further.
const MAX_TICKS_PER_FRAME: u32 = 5;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    // `Poll` plus the `request_redraw` in `about_to_wait`: without both, a
    // frame only arrives when an input event does, and a world nobody is
    // touching stops drawing.
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut handler = Handler::new();
    event_loop.run_app(&mut handler)?;
    handler.result
}

struct Handler {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    game: Druid,
    last_frame: Instant,
    last_title: Instant,
    accumulator: Duration,
    fps: f32,
    held: HeldKeys,
    /// A jump press seen since the last frame's input assembly. Separate from
    /// `held.jump` so a press and release faster than one frame still jumps.
    jump_pressed: bool,
    /// `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES` — the same hook the other two
    /// binaries carry, and the only way to see a rendered window on a
    /// headless box, since this build's swapchain is invisible to OS capture.
    screenshot_countdown: Option<u32>,
    result: Result<(), Box<dyn std::error::Error>>,
}

#[derive(Default)]
struct HeldKeys {
    left: bool,
    right: bool,
    jump: bool,
    down: bool,
    grab: bool,
}

impl Handler {
    fn new() -> Self {
        let now = Instant::now();
        let mut game = Druid::new();
        // `PIXEL_PHYSICS_DRUID_LOOK=onehue` -- start in a look rather than
        // reaching it with a key, because a headless screenshot cannot press
        // one and the whole point of the selector is to be compared.
        if std::env::var("PIXEL_PHYSICS_DRUID_LOOK").is_ok_and(|v| v.eq_ignore_ascii_case("onehue")) {
            game.renderer.held_look = pixel_physics::render::HeldLook::OneHue;
        }
        Self {
            window: None,
            pixels: None,
            game,
            last_frame: now,
            last_title: now,
            accumulator: Duration::ZERO,
            fps: 0.0,
            held: HeldKeys::default(),
            jump_pressed: false,
            screenshot_countdown: std::env::var("PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES").ok().and_then(|v| v.parse().ok()),
            result: Ok(()),
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        self.result = Err(message.into());
        event_loop.exit();
    }

    fn title(&self) -> String {
        let q = self.game.world.quickenings.len();
        format!(
            "The Held World — {:.0} fps — {} — look {} — {} standing quickening{}{}",
            self.fps,
            if self.game.world.held { "HELD" } else { "running" },
            // Named on screen, per `CLAUDE.md`'s rule for a runtime selector:
            // an option nobody can see the value of is one nobody can tell is
            // disconnected.
            self.game.renderer.held_look.label(),
            q,
            if q == 1 { "" } else { "s" },
            if self.game.paused { " — PAUSED" } else { "" },
        )
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame);
        self.last_frame = now;

        let instant_fps = 1.0 / elapsed.as_secs_f32().max(1e-6);
        self.fps = if self.fps == 0.0 { instant_fps } else { self.fps * 0.9 + instant_fps * 0.1 };

        // Held state copied fresh; the jump press ORs in, so a press made on
        // a frame that ran zero ticks survives until a tick consumes it.
        self.game.player_input.left = self.held.left;
        self.game.player_input.right = self.held.right;
        self.game.player_input.jump_held = self.held.jump;
        self.game.player_input.down = self.held.down;
        self.game.player_input.grab = self.held.grab;
        self.game.player_input.jump_pressed |= std::mem::take(&mut self.jump_pressed);

        self.accumulator += elapsed;
        let mut ticks = 0;
        while self.accumulator >= TICK && ticks < MAX_TICKS_PER_FRAME {
            self.game.update();
            self.accumulator -= TICK;
            ticks += 1;
        }
        // Drop the backlog rather than carrying a debt into the next frame.
        if self.accumulator >= TICK {
            self.accumulator = Duration::ZERO;
        }

        let render_error = match &mut self.pixels {
            Some(pixels) => {
                self.game.draw(pixels.frame_mut(), (WIDTH, HEIGHT), false);
                if let Some(n) = self.screenshot_countdown {
                    if n <= 1 {
                        self.screenshot_countdown = None;
                        save_framebuffer_png(pixels.frame(), WIDTH, HEIGHT);
                    } else {
                        self.screenshot_countdown = Some(n - 1);
                    }
                }
                pixels.render().err()
            }
            None => None,
        };
        if let Some(err) = render_error {
            self.fail(event_loop, format!("render failed: {err}"));
            return;
        }

        // Retitling every frame flickers and wastes work.
        if now.duration_since(self.last_title) >= Duration::from_millis(250) {
            self.last_title = now;
            let title = self.title();
            if let Some(window) = &self.window {
                window.set_title(&title);
            }
        }
    }

    fn key(&mut self, code: KeyCode, event_loop: &ActiveEventLoop) {
        match code {
            KeyCode::Escape => event_loop.exit(),
            KeyCode::KeyP => self.game.paused = !self.game.paused,
            // **How held ground is drawn.** A selector rather than a
            // decision, because this is precisely the question no amount of
            // argument settles -- see `render::HeldLook`.
            KeyCode::KeyL => self.game.renderer.cycle_held_look(),
            // **Release the world, or hold it again.** The single most useful
            // key for judging this game: the look the owner picked has no
            // colour tell, so whether "held" reads at all is a question you
            // answer by flipping it and watching, not by looking at a still.
            KeyCode::KeyH => {
                self.game.world.held = !self.game.world.held;
                let hold = if self.game.world.held { pixel_physics::sim::clock::SkyPin::Noon.hold() } else { None };
                self.game.world.set_sky_hold(hold);
            }
            _ => {}
        }
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Fires again after a suspend on mobile; only build once.
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("The Held World")
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
        // Reset after startup: generating and growing a world takes a while,
        // and without this the first frame owes the accumulator all of it.
        self.last_frame = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                // A minimised window reports zero, which the surface rejects.
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
                    match code {
                        KeyCode::KeyA => self.held.left = pressed,
                        KeyCode::KeyD => self.held.right = pressed,
                        KeyCode::KeyW => {
                            self.held.jump = pressed;
                            if pressed && !event.repeat {
                                self.jump_pressed = true;
                            }
                        }
                        KeyCode::KeyS => self.held.down = pressed,
                        // Either shift, so it does not matter which hand is
                        // on the movement keys.
                        KeyCode::ShiftLeft | KeyCode::ShiftRight => self.held.grab = pressed,
                        _ => {}
                    }
                    if pressed && !event.repeat {
                        self.key(code, event_loop);
                    }
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

/// Its own filename, so a druid screenshot and a sandbox one can both exist.
fn save_framebuffer_png(rgba: &[u8], width: u32, height: u32) {
    let path = std::env::temp_dir().join("pixel_physics_druid_screenshot.png");
    match image::save_buffer(&path, rgba, width, height, image::ColorType::Rgba8) {
        Ok(()) => eprintln!("screenshot saved: {}", path.display()),
        Err(e) => eprintln!("screenshot failed: {e}"),
    }
}
