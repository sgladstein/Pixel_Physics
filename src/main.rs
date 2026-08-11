//! Window, input and the frame loop.
//!
//! This is the only module that knows winit and pixels exist. Everything with
//! behaviour lives in `app` and `sim`, which is what keeps those testable
//! without a display.

use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use pixel_physics::app::{App, HEIGHT, WIDTH};
use pixel_physics::sim::material::ASSET_DIR;
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// The simulation advances at a fixed rate regardless of frame rate. A variable
/// timestep would make cellular automaton rules behave differently on different
/// machines, since every rule is "one cell per step" rather than a velocity.
const TICKS_PER_SECOND: u32 = 60;
const TICK: Duration = Duration::from_nanos(1_000_000_000 / TICKS_PER_SECOND as u64);
/// Ceiling on catch-up ticks per frame. Without it, a stall makes the next
/// frame try to simulate the whole missing interval, which stalls it further.
const MAX_TICKS_PER_FRAME: u32 = 5;

/// Editors write a file in several operations, so a single save produces a
/// burst of notifications. Reloading once per burst avoids parsing the file
/// while it is still half-written.
const RELOAD_DEBOUNCE: Duration = Duration::from_millis(200);

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
    app: App,

    /// Leftover simulation time not yet consumed by a tick.
    accumulator: Duration,
    last_frame: Instant,
    fps: f32,
    last_title_update: Instant,

    /// Kept alive for as long as we want notifications; dropping it stops them.
    _watcher: Option<RecommendedWatcher>,
    material_events: Option<Receiver<()>>,
    /// When a change was last noticed, for debouncing a burst of writes.
    pending_reload: Option<Instant>,

    /// Counts down real rendered frames until a one-shot framebuffer PNG
    /// dump, set from `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES`; `None` means
    /// not requested, and it is cleared to `None` again once fired so it
    /// never repeats. Exists because this build's DXGI/wgpu swapchain is not
    /// visible to Windows screen capture — neither BitBlt/CopyFromScreen nor
    /// PrintWindow(PW_RENDERFULLCONTENT) could see the client area, both
    /// capturing solid black while the window chrome captured fine — so
    /// getting a look at an actual rendered scene means dumping the
    /// framebuffer the app already holds in memory, with no OS capture API
    /// involved. `scripts/screenshot.ps1` still works for confirming the
    /// window exists, is titled correctly, and reports sane fps/state
    /// through its title bar; it just cannot see the canvas.
    screenshot_countdown: Option<u32>,

    /// Cursor position in framebuffer pixels, `None` while outside the window.
    cursor: Option<(i32, i32)>,
    /// Previous painted position, so a drag paints a continuous stroke.
    last_paint: Option<(i32, i32)>,
    painting: bool,
    erasing: bool,

    result: Result<(), Box<dyn std::error::Error>>,
}

impl Handler {
    fn new() -> Self {
        let (watcher, material_events) = watch_materials();
        Self {
            window: None,
            pixels: None,
            app: App::new(),
            accumulator: Duration::ZERO,
            last_frame: Instant::now(),
            fps: 0.0,
            last_title_update: Instant::now(),
            _watcher: watcher,
            material_events,
            pending_reload: None,
            screenshot_countdown: std::env::var("PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES")
                .ok()
                .and_then(|s| s.parse().ok()),
            cursor: None,
            last_paint: None,
            painting: false,
            erasing: false,
            result: Ok(()),
        }
    }

    /// Reload materials shortly after the files stop changing.
    ///
    /// Failing to watch is not fatal: the engine still runs, and F5 reloads by
    /// hand. That matters because the assets directory is resolved relative to
    /// the working directory, which a shipped binary may not share.
    fn poll_material_changes(&mut self) {
        if let Some(rx) = &self.material_events {
            // Drain the burst; the timer restarts on each one, so the reload
            // happens once the writes have stopped.
            let mut saw_change = false;
            while rx.try_recv().is_ok() {
                saw_change = true;
            }
            if saw_change {
                self.pending_reload = Some(Instant::now());
            }
        }

        if let Some(at) = self.pending_reload {
            if at.elapsed() >= RELOAD_DEBOUNCE {
                self.pending_reload = None;
                self.app.reload_materials();
            }
        }
    }

    /// Stop the loop, reporting `err` from `main`.
    fn fail(&mut self, event_loop: &ActiveEventLoop, err: impl Into<Box<dyn std::error::Error>>) {
        self.result = Err(err.into());
        event_loop.exit();
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame);
        self.last_frame = now;

        // Exponential smoothing, or the reading is too noisy to read.
        let instant_fps = 1.0 / elapsed.as_secs_f32().max(1e-6);
        self.fps = if self.fps == 0.0 {
            instant_fps
        } else {
            self.fps * 0.9 + instant_fps * 0.1
        };

        self.accumulator += elapsed;
        let mut ticks = 0;
        while self.accumulator >= TICK && ticks < MAX_TICKS_PER_FRAME {
            self.app.update();
            self.accumulator -= TICK;
            ticks += 1;
        }
        // Drop any remaining backlog rather than carrying a debt forward.
        if self.accumulator >= TICK {
            self.accumulator = Duration::ZERO;
        }

        // The error is captured and handled after the borrow of `self.pixels`
        // ends, since `fail` needs all of `self`.
        let render_error = match &mut self.pixels {
            Some(pixels) => {
                self.app.draw(pixels.frame_mut());
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

        // Retitling every frame is wasted work and makes the title flicker.
        if now.duration_since(self.last_title_update) >= Duration::from_millis(250) {
            self.last_title_update = now;
            if let Some(window) = &self.window {
                window.set_title(&self.app.status(self.fps));
            }
        }
    }

    fn paint_now(&mut self) {
        let Some(pos) = self.cursor else { return };
        if !self.painting && !self.erasing {
            return;
        }
        let erase = self.erasing;
        match self.last_paint {
            Some(prev) => self.app.paint_stroke(prev, pos, erase),
            None => self.app.paint(pos.0, pos.1, erase),
        }
        self.last_paint = Some(pos);
    }

    fn key(&mut self, code: KeyCode, event_loop: &ActiveEventLoop) {
        match code {
            KeyCode::Escape => event_loop.exit(),
            KeyCode::Space => self.app.paused = !self.app.paused,
            // Single-step while paused, for inspecting a rule frame by frame.
            KeyCode::Period => self.app.step_once = true,
            KeyCode::KeyR => self.app.reset(),
            KeyCode::F1 => self.app.toggle_overlay(),
            KeyCode::F5 => self.app.reload_materials(),
            KeyCode::KeyF => {
                if let Some((x, y)) = self.cursor {
                    self.app.ignite(x, y);
                }
            }
            KeyCode::KeyP => {
                if let Some((x, y)) = self.cursor {
                    self.app.spawn_burst(x, y);
                }
            }
            KeyCode::KeyX => {
                if let Some((x, y)) = self.cursor {
                    self.app.explode(x, y);
                }
            }
            KeyCode::KeyT => {
                if let Some((x, y)) = self.cursor {
                    self.app.plant_tree(x, y);
                }
            }
            KeyCode::KeyM => {
                if let Some((x, y)) = self.cursor {
                    self.app.plant_moss(x, y);
                }
            }
            KeyCode::KeyW => {
                if let Some((x, y)) = self.cursor {
                    self.app.plant_worm(x, y);
                }
            }
            KeyCode::BracketLeft => self.app.adjust_brush(-2),
            KeyCode::BracketRight => self.app.adjust_brush(2),
            KeyCode::KeyQ => self.app.cycle_material(-1),
            KeyCode::KeyE => self.app.cycle_material(1),
            KeyCode::Digit1 => self.app.select_material(1),
            KeyCode::Digit2 => self.app.select_material(2),
            KeyCode::Digit3 => self.app.select_material(3),
            KeyCode::Digit4 => self.app.select_material(4),
            KeyCode::Digit5 => self.app.select_material(5),
            KeyCode::Digit6 => self.app.select_material(6),
            KeyCode::Digit7 => self.app.select_material(7),
            KeyCode::Digit8 => self.app.select_material(8),
            KeyCode::Digit9 => self.app.select_material(9),
            _ => {}
        }
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // `resumed` fires again after suspend on mobile; only build once.
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("Pixel Physics")
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

        window.set_title(&self.app.status(0.0));
        self.window = Some(window);
        self.last_frame = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                // A minimised window reports a zero size, which the surface
                // rejects.
                if size.width > 0 && size.height > 0 {
                    let resize_error = self
                        .pixels
                        .as_mut()
                        .and_then(|p| p.resize_surface(size.width, size.height).err());
                    if let Some(err) = resize_error {
                        self.fail(event_loop, format!("surface resize failed: {err}"));
                    }
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && !event.repeat {
                    if let PhysicalKey::Code(code) = event.physical_key {
                        self.key(code, event_loop);
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                // Map window coordinates onto the framebuffer, which is scaled
                // and letterboxed inside the window.
                self.cursor = self
                    .pixels
                    .as_ref()
                    .and_then(|p| {
                        p.window_pos_to_pixel((position.x as f32, position.y as f32))
                            .ok()
                    })
                    .map(|(x, y)| (x as i32, y as i32));
                self.paint_now();
            }

            WindowEvent::CursorLeft { .. } => {
                self.cursor = None;
                self.last_paint = None;
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => self.painting = pressed,
                    MouseButton::Right => self.erasing = pressed,
                    _ => {}
                }
                if pressed {
                    // Start a fresh stroke rather than joining the last one.
                    self.last_paint = None;
                    self.paint_now();
                } else if !self.painting && !self.erasing {
                    self.last_paint = None;
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                if scroll != 0.0 {
                    self.app.adjust_brush(if scroll > 0.0 { 2 } else { -2 });
                }
            }

            WindowEvent::RedrawRequested => self.frame(event_loop),

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.poll_material_changes();

        // Painting while the mouse is held but stationary: no CursorMoved
        // arrives, so the brush would only apply on movement.
        if self.painting || self.erasing {
            self.paint_now();
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

/// Watch the material directory, reducing every event to a bare "something
/// changed" — the reload re-reads the whole directory anyway, so which file it
/// was does not matter.
fn watch_materials() -> (Option<RecommendedWatcher>, Option<Receiver<()>>) {
    let (tx, rx) = channel();
    let mut watcher = match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if res.is_ok() {
            // The receiver is gone only during shutdown.
            let _ = tx.send(());
        }
    }) {
        Ok(w) => w,
        Err(_) => return (None, None),
    };

    match watcher.watch(Path::new(ASSET_DIR), RecursiveMode::NonRecursive) {
        Ok(()) => (Some(watcher), Some(rx)),
        // No assets directory beside the binary. The compiled-in materials are
        // already loaded, and F5 still works if one appears later.
        Err(_) => (None, None),
    }
}

/// Dumps the framebuffer `pixels` already holds in memory straight to a PNG.
/// See the doc comment on `Handler::screenshot_countdown` for why this exists
/// rather than an external screen-capture tool.
///
/// Usage: run with `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=<n>` set, and after
/// `n` real rendered frames a PNG lands at
/// `%TEMP%\pixel_physics_screenshot.png`. Combine with whatever brush/paint/
/// ignite calls are needed to set up the scene worth looking at first — there
/// is no built-in scene-scripting hook, so that part is still a manual edit
/// here for now.
fn save_framebuffer_png(rgba: &[u8], width: u32, height: u32) {
    let path = std::env::temp_dir().join("pixel_physics_screenshot.png");
    match image::save_buffer(&path, rgba, width, height, image::ColorType::Rgba8) {
        Ok(()) => eprintln!("screenshot saved: {}", path.display()),
        Err(e) => eprintln!("screenshot failed: {e}"),
    }
}
