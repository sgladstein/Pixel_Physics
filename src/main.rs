//! Window, input and the frame loop.
//!
//! This is the only module that knows winit and pixels exist. Everything with
//! behaviour lives in `app` and `sim`, which is what keeps those testable
//! without a display.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use pixel_physics::app::{App, HEIGHT, WIDTH};
use pixel_physics::sim::material::ASSET_DIR;
use pixel_physics::sim::organism;
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

    /// A multi-frame capture in progress, set from
    /// `PIXEL_PHYSICS_CAPTURE_SEQUENCE=<start_frame>,<interval_frames>,<count>`.
    /// Complements `screenshot_countdown`'s single-moment dump for the case a
    /// screenshot can't show: a change that only reads correctly across many
    /// frames (a settling water column, an explosion's debris trajectory).
    /// `None` once the capture completes; never re-arms.
    capture_sequence: Option<CaptureSequence>,

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
            capture_sequence: CaptureSequence::from_env(),
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
                self.app.draw(pixels.frame_mut(), self.cursor);
                if let Some(n) = self.screenshot_countdown {
                    if n <= 1 {
                        self.screenshot_countdown = None;
                        save_framebuffer_png(pixels.frame(), WIDTH, HEIGHT);
                    } else {
                        self.screenshot_countdown = Some(n - 1);
                    }
                }
                if let Some(seq) = &mut self.capture_sequence {
                    seq.tick(pixels.frame());
                }
                pixels.render().err()
            }
            None => None,
        };
        if let Some(err) = render_error {
            self.fail(event_loop, format!("render failed: {err}"));
            return;
        }
        // Outside the `tick` call above, not because of the `pixels` borrow
        // (that ends before this point regardless) but because `.take()`
        // needs `&mut self.capture_sequence` free of the `&mut seq` borrow
        // `tick` was just called through -- both borrow the same field.
        if self.capture_sequence.as_ref().is_some_and(CaptureSequence::is_complete) {
            self.capture_sequence.take().unwrap().finish();
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
            // Contextual: closes the tunables panel first (the one overlay
            // with real "editing" semantics, where a close-without-saving
            // distinction matters -- see `App::toggle_tunables`'s own
            // doc), quits only once nothing is open to close instead.
            KeyCode::Escape => {
                if self.app.show_tunables {
                    self.app.show_tunables = false;
                } else if self.app.has_pin() {
                    self.app.clear_pin();
                } else {
                    event_loop.exit();
                }
            }
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
            KeyCode::KeyB => self.app.toggle_build_background(),
            KeyCode::KeyC => {
                if let Some((x, y)) = self.cursor {
                    self.app.strike(x, y);
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
            KeyCode::Equal => self.app.renderer.adjust_zoom(1),
            KeyCode::Minus => self.app.renderer.adjust_zoom(-1),
            KeyCode::KeyV => self.app.renderer.cycle_field_overlay(),
            KeyCode::KeyG => self.app.renderer.cycle_grain(),
            // The A/B key. Deliberately reassigned as the question changes --
            // see `App::toggle_experiment`.
            KeyCode::KeyK => self.app.toggle_experiment(),
            KeyCode::PageUp | KeyCode::PageDown if self.app.show_tunables => self.app.tunables_cycle_group(),
            KeyCode::KeyS if self.app.show_tunables => self.app.save_tunable(),
            KeyCode::KeyI => self.app.toggle_hover_inspector(),
            KeyCode::KeyN => self.app.toggle_stress_view(),
            KeyCode::KeyZ => self.app.cycle_tool(),
            KeyCode::Tab => self.app.toggle_palette(),
            KeyCode::Slash => self.app.toggle_help(),
            KeyCode::KeyO => self.app.toggle_tunables(),
            // Arrow keys and Enter only drive the tunables panel -- they
            // have no other binding to steal. Left/right additionally drive
            // a *pinned* tunable with the panel closed, which is the whole
            // point of pinning: sweep a value with the world visible rather
            // than behind a panel covering most of it (`App::pinned`).
            KeyCode::ArrowUp if self.app.show_tunables => self.app.tunables_move(-1),
            KeyCode::ArrowDown if self.app.show_tunables => self.app.tunables_move(1),
            KeyCode::ArrowLeft if self.app.show_tunables => self.app.tunables_adjust(-1),
            KeyCode::ArrowRight if self.app.show_tunables => self.app.tunables_adjust(1),
            KeyCode::ArrowLeft => self.app.adjust_pinned(-1),
            KeyCode::ArrowRight => self.app.adjust_pinned(1),
            // Enter *pins and closes*, rather than saving -- saving moved to
            // `S`. Pin is the far more frequent action (every value judged by
            // eye needs it; saving happens once at the end), and "take this
            // one with me" is the better meaning for the key that dismisses
            // a dialog.
            KeyCode::Enter if self.app.show_tunables => self.app.pin_selected(),
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
                let erase = button == MouseButton::Right;
                if pressed {
                    // Start a fresh stroke rather than joining the last one.
                    self.last_paint = None;
                    // The drag-out tools commit on *release*, so a press
                    // only records where the gesture began; `paint_now`
                    // would lay a blob at the corner.
                    if self.app.tool == pixel_physics::app::Tool::Brush {
                        self.paint_now();
                    } else if let Some((x, y)) = self.cursor {
                        self.app.begin_drag(x, y);
                    }
                } else {
                    if let Some((x, y)) = self.cursor {
                        self.app.end_drag(x, y, erase);
                    }
                    if !self.painting && !self.erasing {
                        self.last_paint = None;
                    }
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

/// Watch the material and species directories, reducing every event to a
/// bare "something changed" — the reload re-reads each whole directory
/// anyway, so which file it was, or which of the two directories, does not
/// matter. One `notify` watcher covers both paths and one `Receiver` fires
/// for either; `App::reload_materials` (despite the name — kept to avoid
/// churning every call site for a rename that doesn't fix anything) already
/// reloads species alongside materials on every call, so a single "reload
/// everything" signal is all either watched directory needs to produce.
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

    let materials_watched = watcher.watch(Path::new(ASSET_DIR), RecursiveMode::NonRecursive).is_ok();
    // Species reload was designed to hot-reload the same way materials do
    // (`Reports/organism-substrate-design.md`) but the watcher never
    // actually covered its directory — caught by independent review of the
    // first version of this section: `SpeciesRegistry::reload` existed and
    // was tested, but no live-editing path ever called it. Watched
    // independently of whether materials succeeded above; a missing species
    // directory is no more fatal than a missing materials one (the
    // compiled-in set already loaded, per `SpeciesRegistry::builtin`).
    let species_watched = watcher.watch(Path::new(organism::ASSET_DIR), RecursiveMode::NonRecursive).is_ok();

    if materials_watched || species_watched {
        (Some(watcher), Some(rx))
    } else {
        // Neither assets directory exists beside the binary. The compiled-in
        // materials and species are already loaded, and F5 still works if
        // either directory appears later.
        (None, None)
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

/// A multi-frame companion to `save_framebuffer_png` for behaviour that only
/// reads correctly across time — a water column settling, explosion debris
/// scattering. One screenshot cannot show either; a strip of them can, and
/// so can the assembled GIF `finish` writes alongside them.
///
/// Usage: run with `PIXEL_PHYSICS_CAPTURE_SEQUENCE=<start_frame>,
/// <interval_frames>,<count>` set (e.g. `0,15,40` — start immediately, one
/// frame every 15 ticks, 40 captures). Numbered PNGs and one `sequence.gif`
/// land in a fresh timestamped folder under `%TEMP%`. Scene setup is still a
/// manual edit here, same limitation `save_framebuffer_png` already has.
struct CaptureSequence {
    /// Frames remaining before the next capture. Counts down to 0, at which
    /// point a capture fires and this resets to `interval - 1` (see `tick`'s
    /// own comment for why it's `- 1` and not `interval` bare) — or, for the
    /// very first capture, counts down from the requested start delay.
    countdown: u32,
    interval: u32,
    remaining: u32,
    dir: PathBuf,
    /// Raw RGBA frames, kept in memory rather than re-read from disk so the
    /// GIF assembly in `finish` doesn't need a second decode pass.
    frames: Vec<Vec<u8>>,
}

impl CaptureSequence {
    fn from_env() -> Option<Self> {
        let spec = std::env::var("PIXEL_PHYSICS_CAPTURE_SEQUENCE").ok()?;
        let mut parts = spec.split(',').map(str::trim);
        let start: u32 = parts.next()?.parse().ok()?;
        let interval: u32 = parts.next()?.parse().ok()?;
        let count: u32 = parts.next()?.parse().ok()?;
        if count == 0 {
            return None;
        }
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Some(Self {
            countdown: start,
            interval: interval.max(1),
            remaining: count,
            dir: std::env::temp_dir().join(format!("pixel_physics_capture_{timestamp}")),
            frames: Vec::with_capacity(count as usize),
        })
    }

    /// Called once per rendered frame. Captures `rgba` when the countdown
    /// reaches zero, then resets it for the next capture.
    fn tick(&mut self, rgba: &[u8]) {
        if self.remaining == 0 {
            return;
        }
        if self.countdown == 0 {
            self.frames.push(rgba.to_vec());
            self.remaining -= 1;
            // `interval - 1`, not `interval`: this call already accounts for
            // one tick (the capture itself), so counting down from `interval`
            // would space captures `interval + 1` ticks apart instead of
            // `interval` -- caught by a test expecting interval=1 to capture
            // every tick, which instead captured every other one. Saturating,
            // not bare subtraction: `from_env` is the only production path
            // and already floors `interval` at 1, but a test-constructed
            // `CaptureSequence` isn't bound by that, and this must not panic
            // (or silently wrap in release) just because a caller built one
            // with `interval: 0`.
            self.countdown = self.interval.saturating_sub(1);
        } else {
            self.countdown -= 1;
        }
    }

    fn is_complete(&self) -> bool {
        self.remaining == 0
    }

    /// Writes every captured frame as a numbered PNG, then assembles the
    /// same sequence into one animated GIF alongside them. Both
    /// deliberately: the GIF is one shareable artifact for a human; the
    /// individual PNGs exist so an agent can `Read` several of them
    /// directly, which is the actual point — seeing change over time
    /// without needing GIF-decoding support.
    fn finish(self) {
        if let Err(e) = std::fs::create_dir_all(&self.dir) {
            eprintln!("capture sequence: failed to create {}: {e}", self.dir.display());
            return;
        }
        // Real playback speed and a loop, not the `image::Frame::new`
        // defaults (0ms delay, no NETSCAPE loop extension) -- a GIF that
        // plays once, instantly, and stops on the last frame defeats the
        // point of "one shareable artifact for a human" this exists for.
        // 60 ticks/second is `main.rs`'s own fixed simulation rate
        // (`TICKS_PER_SECOND`), so `interval` ticks between captures maps
        // directly to real elapsed time.
        let delay_ms = (self.interval as u64 * 1000) / 60;
        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(delay_ms));
        let mut gif_frames = Vec::with_capacity(self.frames.len());
        for (i, rgba) in self.frames.iter().enumerate() {
            let path = self.dir.join(format!("frame_{i:04}.png"));
            if let Err(e) = image::save_buffer(&path, rgba, WIDTH, HEIGHT, image::ColorType::Rgba8) {
                eprintln!("capture sequence: failed to save {}: {e}", path.display());
            }
            if let Some(buf) = image::RgbaImage::from_raw(WIDTH, HEIGHT, rgba.clone()) {
                gif_frames.push(image::Frame::from_parts(buf, 0, 0, delay));
            }
        }
        let gif_path = self.dir.join("sequence.gif");
        match std::fs::File::create(&gif_path) {
            Ok(file) => {
                let mut encoder = image::codecs::gif::GifEncoder::new(file);
                if let Err(e) = encoder.set_repeat(image::codecs::gif::Repeat::Infinite) {
                    eprintln!("capture sequence: gif set_repeat failed: {e}");
                }
                if let Err(e) = encoder.encode_frames(gif_frames) {
                    eprintln!("capture sequence: gif encode failed: {e}");
                }
            }
            Err(e) => eprintln!("capture sequence: failed to create {}: {e}", gif_path.display()),
        }
        eprintln!("capture sequence saved: {} ({} frames)", self.dir.display(), self.frames.len());
    }
}

#[cfg(test)]
mod capture_sequence_tests {
    use super::CaptureSequence;
    use std::sync::Mutex;

    /// `cargo test` runs tests in parallel within one process, and env vars
    /// are process-global — without this, the `from_env` tests below would
    /// race each other's `set_var`/`remove_var` calls. Guards only the tests
    /// that touch `PIXEL_PHYSICS_CAPTURE_SEQUENCE`, not the pure `tick`
    /// tests above them, which don't need it.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn seq(countdown: u32, interval: u32, remaining: u32) -> CaptureSequence {
        CaptureSequence {
            countdown,
            interval,
            remaining,
            dir: std::env::temp_dir(),
            frames: Vec::new(),
        }
    }

    #[test]
    fn captures_immediately_when_start_countdown_is_zero() {
        let mut s = seq(0, 15, 3);
        s.tick(&[1, 2, 3, 4]);
        assert_eq!(s.frames.len(), 1, "a zero start delay should capture on the first tick");
        assert_eq!(s.remaining, 2);
        assert_eq!(s.countdown, 14, "countdown resets to interval-1, so captures land exactly `interval` ticks apart");
    }

    #[test]
    fn waits_out_the_start_delay_before_the_first_capture() {
        let mut s = seq(2, 15, 3);
        s.tick(&[0]);
        s.tick(&[0]);
        assert!(s.frames.is_empty(), "should not capture before the start delay elapses");
        s.tick(&[9]);
        assert_eq!(s.frames.len(), 1);
        assert_eq!(s.frames[0], vec![9]);
    }

    #[test]
    fn captures_land_exactly_interval_ticks_apart() {
        // The regression this session's own review caught: an earlier
        // version reset the countdown to `interval` rather than
        // `interval - 1`, spacing captures `interval + 1` ticks apart.
        let mut s = seq(0, 4, 3);
        let mut capture_ticks = Vec::new();
        for tick in 0..20 {
            let before = s.frames.len();
            s.tick(&[0]);
            if s.frames.len() > before {
                capture_ticks.push(tick);
            }
            if s.is_complete() {
                break;
            }
        }
        assert_eq!(capture_ticks, vec![0, 4, 8], "captures should land exactly 4 ticks apart, not 5");
    }

    #[test]
    fn stops_capturing_once_remaining_hits_zero() {
        let mut s = seq(0, 1, 2);
        s.tick(&[1]);
        s.tick(&[2]);
        assert!(s.is_complete());
        // A further tick must not capture a third frame or underflow `remaining`.
        s.tick(&[3]);
        assert_eq!(s.frames.len(), 2);
    }

    #[test]
    fn is_complete_only_after_every_requested_frame_is_captured() {
        let mut s = seq(0, 5, 2);
        assert!(!s.is_complete());
        s.tick(&[0]);
        assert!(!s.is_complete(), "one of two captured is not complete");
        for _ in 0..5 {
            s.tick(&[0]);
        }
        assert!(s.is_complete());
    }

    #[test]
    fn from_env_parses_start_interval_count() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: serialized by ENV_LOCK against every other test in this module.
        unsafe { std::env::set_var("PIXEL_PHYSICS_CAPTURE_SEQUENCE", "5,15,40") };
        let s = CaptureSequence::from_env().expect("valid spec should parse");
        assert_eq!(s.countdown, 5);
        assert_eq!(s.interval, 15);
        assert_eq!(s.remaining, 40);
        unsafe { std::env::remove_var("PIXEL_PHYSICS_CAPTURE_SEQUENCE") };
    }

    #[test]
    fn from_env_is_none_when_unset_or_malformed() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: serialized by ENV_LOCK against every other test in this module.
        unsafe { std::env::remove_var("PIXEL_PHYSICS_CAPTURE_SEQUENCE") };
        assert!(CaptureSequence::from_env().is_none());

        unsafe { std::env::set_var("PIXEL_PHYSICS_CAPTURE_SEQUENCE", "not,a,spec") };
        assert!(CaptureSequence::from_env().is_none());
        unsafe { std::env::remove_var("PIXEL_PHYSICS_CAPTURE_SEQUENCE") };

        // A zero count would spin forever "complete" without ever capturing.
        unsafe { std::env::set_var("PIXEL_PHYSICS_CAPTURE_SEQUENCE", "0,15,0") };
        assert!(CaptureSequence::from_env().is_none());
        unsafe { std::env::remove_var("PIXEL_PHYSICS_CAPTURE_SEQUENCE") };
    }

    #[test]
    fn from_env_floors_a_zero_interval_at_one() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // A zero interval would mean "capture every frame forever" is fine,
        // but the countdown-reset arithmetic (`countdown = interval`) must
        // not get stuck re-triggering the same frame twice in the same tick.
        // SAFETY: serialized by ENV_LOCK against every other test in this module.
        unsafe { std::env::set_var("PIXEL_PHYSICS_CAPTURE_SEQUENCE", "0,0,3") };
        let s = CaptureSequence::from_env().expect("valid spec should parse");
        assert_eq!(s.interval, 1);
        unsafe { std::env::remove_var("PIXEL_PHYSICS_CAPTURE_SEQUENCE") };
    }
}
