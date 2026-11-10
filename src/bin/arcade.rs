//! **Arcade: every prototype from one menu.** `cargo run --release --bin arcade`
//!
//! A small window listing the game prototypes of the 2026-11-07 new-game
//! brainstorm: seven of the eight, since Mould moved to its own repository,
//! `sgladstein/Slime-Mold`, on 2026-11-10. Pick one and it opens in its own
//! window; close it and the list is still here. Picking another while one is
//! open closes the open one first, so the list works as a switcher.
//!
//! This is option A of the two Scott was offered on 2026-11-08 ("a launcher
//! that opens each game", rather than one binary hosting every game in one
//! window). It touches nothing in the games: each still runs as its own
//! binary, so its keys, its Esc-to-quit and its probes are exactly as they
//! were. The cost of that choice is that each switch closes one window and
//! opens another.
//!
//! **How a game is started.** From a source checkout (the usual case: the
//! repository this was built from still exists) it runs
//! `cargo run --release --bin <game>` in that checkout, so a game is always
//! the current code: cargo rebuilds it if it is stale and is a no-op if it
//! is not. The first launch of each game compiles it (about a minute; the
//! build output is in the terminal). `cargo build --release --bins` up front
//! makes every switch instant. Only if cargo cannot be started does it fall
//! back to the game's executable next to this one, which may be stale.
//!
//! Keys (also on screen): **1-7** or **click** opens a game, **Up/Down** and
//! **Enter** do the same from the keyboard, **K** closes the open game,
//! **Esc** quits the menu (an open game stays open).
//!
//! Adding a prototype is one row in `GAMES`.
//!
//! `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=N` writes the Nth drawn frame to
//! `pixel_physics_arcade_screenshot.png` in the temp directory and exits,
//! the same hook the other binaries carry.

use std::path::Path;
use std::process::{Child, Command};
use std::sync::Arc;

use pixel_physics::hud::{draw_text_scaled, text_width, GLYPH_HEIGHT};
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// One prototype: its binary name, the name on the list, and one line on
/// what you do in it. The text is drawn in the 5x7 dot font, which has no
/// apostrophe or `>`; keep a line to 36 characters so it fits the frame.
struct Game {
    bin: &'static str,
    name: &'static str,
    line: &'static str,
}

const GAMES: &[Game] = &[
    Game { bin: "storm", name: "Storm Glass", line: "call lightning, it leaves glass" },
    Game { bin: "firedamp", name: "Firedamp", line: "herd mine gas, then burn it" },
    Game { bin: "seep", name: "Seep", line: "leaking gas, a mould that eats it" },
    Game { bin: "quench", name: "Quench", line: "aim the roof, not the water" },
    Game { bin: "fault", name: "Fault", line: "crack the ice wall before the lake" },
    Game { bin: "kudzu", name: "Kudzu", line: "a vine is eating the village" },
    Game { bin: "heartwood", name: "Heartwood", line: "grow a tree, shade out a rival" },
];

/// The frame buffer, in pixels; the window shows it at `ZOOM`.
const FW: u32 = 336;
const FH: u32 = 196;
const ZOOM: u32 = 3;

const TOP: i32 = 34;
const ROW: i32 = 14;
const NAME_X: i32 = 30;
const LINE_X: i32 = NAME_X + 13 * 6;

const BG: [u8; 4] = [18, 20, 26, 255];
const HOVER: [u8; 4] = [40, 46, 60, 255];
const OPEN: [u8; 4] = [34, 70, 48, 255];
const TITLE: [u8; 4] = [255, 214, 120, 255];
const NAME: [u8; 4] = [235, 238, 245, 255];
const DIM: [u8; 4] = [140, 148, 165, 255];
const NOTE: [u8; 4] = [170, 210, 255, 255];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut handler = Handler::new();
    event_loop.run_app(&mut handler)?;
    handler.result
}

/// The game that is open, and whether we asked it to close (so its exit is
/// not reported as a failure).
struct Running {
    game: usize,
    child: Child,
    closing: bool,
}

struct Handler {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    /// The row under the pointer or picked with the arrows.
    hover: Option<usize>,
    running: Option<Running>,
    status: String,
    shot_after: Option<u32>,
    drawn: u32,
    result: Result<(), Box<dyn std::error::Error>>,
}

impl Handler {
    fn new() -> Self {
        println!("arcade: 1-{} or click opens a game; K closes it; Esc quits the menu", GAMES.len());
        Handler {
            window: None,
            pixels: None,
            hover: None,
            running: None,
            status: "Pick a game.".into(),
            shot_after: std::env::var("PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES").ok().and_then(|v| v.parse().ok()),
            drawn: 0,
            result: Ok(()),
        }
    }

    fn fail(&mut self, el: &ActiveEventLoop, msg: String) {
        self.result = Err(msg.into());
        el.exit();
    }

    fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    /// Close the open game, if any. `kill` on a `cargo run` child reaches the
    /// game itself on macOS and Linux, because cargo replaces its own process
    /// with the program it runs; mid-build it stops the build.
    fn close(&mut self) {
        if let Some(r) = self.running.as_mut() {
            r.closing = true;
            let _ = r.child.kill();
            let _ = r.child.wait();
            self.status = format!("Closed {}.", GAMES[r.game].name);
        }
        self.running = None;
    }

    fn open(&mut self, game: usize) {
        if self.running.as_ref().is_some_and(|r| r.game == game) {
            self.status = format!("{} is already open.", GAMES[game].name);
            return;
        }
        self.close();
        match launch(GAMES[game].bin) {
            Ok(child) => {
                println!("arcade: opening {}", GAMES[game].bin);
                self.running = Some(Running { game, child, closing: false });
                self.status = format!("Opening {}. First time builds it, about a minute.", GAMES[game].name);
            }
            Err(e) => self.status = format!("Could not start {}: {e}", GAMES[game].bin),
        }
    }

    /// Notice a game that has closed itself (its window shut, or Esc).
    fn poll(&mut self) {
        let Some(r) = self.running.as_mut() else { return };
        match r.child.try_wait() {
            Ok(None) => {}
            Ok(Some(status)) => {
                let name = GAMES[r.game].name;
                // A signal (no exit code) is someone closing it from outside,
                // not the game failing.
                self.status = if status.success() || r.closing || status.code().is_none() {
                    format!("{name} closed. Pick another.")
                } else {
                    format!("{name} stopped with an error; see the terminal.")
                };
                self.running = None;
                self.redraw();
            }
            Err(e) => {
                self.status = format!("Lost track of {}: {e}", GAMES[r.game].name);
                self.running = None;
            }
        }
    }

    fn frame(&mut self, el: &ActiveEventLoop) {
        let open = self.running.as_ref().map(|r| r.game);
        let Some(p) = self.pixels.as_mut() else { return };
        let buf = p.frame_mut();
        draw(buf, self.hover, open, &self.status);
        self.drawn += 1;
        if self.shot_after == Some(self.drawn) {
            let path = std::env::temp_dir().join("pixel_physics_arcade_screenshot.png");
            match image::save_buffer(&path, p.frame(), FW, FH, image::ColorType::Rgba8) {
                Ok(()) => eprintln!("arcade: screenshot written to {}", path.display()),
                Err(e) => eprintln!("arcade: screenshot failed: {e}"),
            }
            el.exit();
        }
        if let Err(e) = p.render() {
            self.fail(el, format!("render failed: {e}"));
        }
    }
}

/// Start `bin`: `cargo run --release --bin bin` in the checkout this was
/// built from, or failing that the executable beside this one.
fn launch(bin: &str) -> std::io::Result<Child> {
    let checkout = Path::new(env!("CARGO_MANIFEST_DIR"));
    if checkout.join("Cargo.toml").is_file() {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        if let Ok(child) = Command::new(cargo).current_dir(checkout).args(["run", "--release", "--bin", bin]).spawn() {
            return Ok(child);
        }
    }
    let exe = std::env::current_exe()?.with_file_name(format!("{bin}{}", std::env::consts::EXE_SUFFIX));
    Command::new(exe).spawn()
}

/// Which row the buffer point `(x, y)` is on.
fn row_at(x: i32, y: i32) -> Option<usize> {
    if !(4..FW as i32 - 4).contains(&x) || y < TOP - 4 {
        return None;
    }
    let i = ((y - (TOP - 4)) / ROW) as usize;
    (i < GAMES.len()).then_some(i)
}

fn text(buf: &mut [u8], x: i32, y: i32, s: &str, c: [u8; 4]) {
    draw_text_scaled(buf, (FW, FH), 1, (x, y), s, c);
}

fn fill(buf: &mut [u8], x0: i32, y0: i32, w: i32, h: i32, c: [u8; 4]) {
    for y in y0.max(0)..(y0 + h).min(FH as i32) {
        for x in x0.max(0)..(x0 + w).min(FW as i32) {
            let i = (y as usize * FW as usize + x as usize) * 4;
            buf[i..i + 4].copy_from_slice(&c);
        }
    }
}

fn draw(buf: &mut [u8], hover: Option<usize>, open: Option<usize>, status: &str) {
    for px in buf.chunks_exact_mut(4) {
        px.copy_from_slice(&BG);
    }
    let title = "PROTOTYPES";
    text(buf, (FW as i32 - text_width(title)) / 2, 10, title, TITLE);
    for (i, g) in GAMES.iter().enumerate() {
        let y = TOP + i as i32 * ROW;
        if open == Some(i) {
            fill(buf, 4, y - 4, FW as i32 - 8, ROW - 1, OPEN);
        } else if hover == Some(i) {
            fill(buf, 4, y - 4, FW as i32 - 8, ROW - 1, HOVER);
        }
        text(buf, 12, y, &format!("{}", i + 1), TITLE);
        text(buf, NAME_X, y, g.name, NAME);
        text(buf, LINE_X, y, g.line, DIM);
        if open == Some(i) {
            text(buf, FW as i32 - 10 - text_width("OPEN"), y, "OPEN", NOTE);
        }
    }
    let foot = TOP + GAMES.len() as i32 * ROW + 4;
    // Wrap the status onto two lines at a word if it does not fit one.
    let max = ((FW as i32 - 24) / 6) as usize;
    let (a, b) = match status.char_indices().rfind(|&(i, c)| c == ' ' && i <= max) {
        Some((i, _)) if status.len() > max => (&status[..i], &status[i + 1..]),
        _ => (status, ""),
    };
    text(buf, 12, foot, a, NOTE);
    text(buf, 12, foot + GLYPH_HEIGHT + 3, b, NOTE);
    text(buf, 12, FH as i32 - 12, "1-7 OR CLICK: OPEN   K: CLOSE IT   ESC: QUIT MENU", DIM);
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("Arcade -- prototypes")
            .with_inner_size(LogicalSize::new(FW * ZOOM, FH * ZOOM))
            .with_min_inner_size(LogicalSize::new(FW, FH));
        let window = match el.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(e) => return self.fail(el, format!("window creation failed: {e}")),
        };
        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width, size.height, Arc::clone(&window));
        match Pixels::new(FW, FH, surface) {
            Ok(p) => self.pixels = Some(p),
            Err(e) => return self.fail(el, format!("pixels init failed: {e}")),
        }
        self.window = Some(window);
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
                    let hover = p.window_pos_to_pixel((position.x as f32, position.y as f32)).ok().and_then(|(x, y)| row_at(x as i32, y as i32));
                    if hover != self.hover {
                        self.hover = hover;
                        self.redraw();
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.hover = None;
                self.redraw();
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                if let Some(i) = self.hover {
                    self.open(i);
                    self.redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else { return };
                if event.state != ElementState::Pressed || event.repeat {
                    return;
                }
                let digit = match code {
                    KeyCode::Digit1 | KeyCode::Numpad1 => Some(0),
                    KeyCode::Digit2 | KeyCode::Numpad2 => Some(1),
                    KeyCode::Digit3 | KeyCode::Numpad3 => Some(2),
                    KeyCode::Digit4 | KeyCode::Numpad4 => Some(3),
                    KeyCode::Digit5 | KeyCode::Numpad5 => Some(4),
                    KeyCode::Digit6 | KeyCode::Numpad6 => Some(5),
                    KeyCode::Digit7 | KeyCode::Numpad7 => Some(6),
                    KeyCode::Digit8 | KeyCode::Numpad8 => Some(7),
                    KeyCode::Digit9 | KeyCode::Numpad9 => Some(8),
                    _ => None,
                };
                if let Some(d) = digit.filter(|&d| d < GAMES.len()) {
                    self.open(d);
                    self.redraw();
                    return;
                }
                match code {
                    KeyCode::ArrowDown => self.hover = Some(self.hover.map_or(0, |h| (h + 1) % GAMES.len())),
                    KeyCode::ArrowUp => self.hover = Some(self.hover.map_or(GAMES.len() - 1, |h| (h + GAMES.len() - 1) % GAMES.len())),
                    KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                        if let Some(i) = self.hover {
                            self.open(i);
                        }
                    }
                    KeyCode::KeyK => self.close(),
                    KeyCode::Escape => el.exit(),
                    _ => {}
                }
                self.redraw();
            }
            WindowEvent::RedrawRequested => self.frame(el),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        // The menu only redraws on input, but while a game is open it wakes
        // a few times a second to notice the game closing itself.
        self.poll();
        if self.running.is_some() {
            el.set_control_flow(ControlFlow::WaitUntil(std::time::Instant::now() + std::time::Duration::from_millis(250)));
        } else {
            el.set_control_flow(ControlFlow::Wait);
        }
        if self.shot_after.is_some() {
            self.redraw();
        }
    }
}
