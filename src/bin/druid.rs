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
//! **The keys are on screen**, listed in the bottom-left corner and hideable
//! with `/`. They live in `druid::hud::KEYS` rather than in this comment, and
//! a guard there reads *this file* and fails if a key is bound here and not
//! named there — because a comment at the top of a source file is precisely
//! where they were the first time somebody tried to play this and reported
//! that the game had no interface at all.

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

/// Frames to wait after a scripted absorb before the screenshot, so the flow
/// is caught in mid-air rather than before it starts or after it lands.
/// `druid::DRAW_FRAMES` is 42 player ticks; a third of the way along shows the
/// stream strung out with its head near the player.
const DRAW_FRAMES_TO_CATCH: u32 = 4;

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

/// **An animation being captured**, in player ticks — see
/// `PIXEL_PHYSICS_DRUID_GIF`.
struct GifCapture {
    /// First tick to capture on.
    start: u64,
    /// Ticks between captures. 1 is every tick, which at 60/s is real time.
    every: u64,
    /// How many frames to take before writing the file and exiting.
    count: usize,
    out: std::path::PathBuf,
    frames: Vec<Vec<u8>>,
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
    /// See `PIXEL_PHYSICS_DRUID_CENSUS`.
    census_after: Option<u64>,
    /// See `PIXEL_PHYSICS_DRUID_ABSORB_AT`.
    absorb_at: Option<u64>,
    /// See `PIXEL_PHYSICS_DRUID_GIF`.
    gif: Option<GifCapture>,
    /// See `PIXEL_PHYSICS_DRUID_WALK`.
    walk: (u64, u64),
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
        // `PIXEL_PHYSICS_DRUID_FOUND=1` -- found a colony at startup, for the
        // same reason as the look above: a headless screenshot cannot press
        // `C`, and "did the founding place anybody" is a question with a
        // number rather than a picture.
        if std::env::var("PIXEL_PHYSICS_DRUID_FOUND").is_ok_and(|v| v != "0") {
            game.found_colony();
        }
        // `PIXEL_PHYSICS_DRUID_CIRCLES=x,y,r,rate;x,y,r,rate` -- place
        // standing quickenings at startup. Third hook of the same shape and
        // for the same reason as the two above: a headless run cannot press
        // `SPACE`, and the speed dial is a claim about what happens over
        // hundreds of frames, which is not a thing a screenshot can settle.
        if let Ok(spec) = std::env::var("PIXEL_PHYSICS_DRUID_CIRCLES") {
            for one in spec.split(';').filter(|s| !s.trim().is_empty()) {
                let n: Vec<&str> = one.split(',').collect();
                match n.as_slice() {
                    [x, y, r, rate] => {
                        let parsed = (x.trim().parse(), y.trim().parse(), r.trim().parse(), rate.trim().parse::<u32>());
                        if let (Ok(x), Ok(y), Ok(r), Ok(rate)) = parsed {
                            game.world.quickenings.push(pixel_physics::sim::world::Quickening::at(x, y, r));
                            // The dial is one number for the whole game now,
                            // so the last entry's speed wins -- see
                            // `Druid::speed`. Kept in the spec's shape so the
                            // measurement scripts still read.
                            game.speed = rate.clamp(pixel_physics::druid::SPEED_MIN, pixel_physics::druid::SPEED_MAX);
                            let woken = game.world.wake_region(x, y, r);
                            println!("druid: circle at {x},{y} r{r}, world speed x{} — woke {woken} sites", game.speed);
                        } else {
                            eprintln!("druid: CIRCLES entry {one:?} is not x,y,r,rate");
                        }
                    }
                    _ => eprintln!("druid: CIRCLES entry {one:?} is not x,y,r,rate"),
                }
            }
        }
        // `PIXEL_PHYSICS_DRUID_UNLIMITED=1` -- the `U` key, for a run with no
        // hands on it. Not a convenience: the first attempt to measure the
        // speed dial reported *no circles at all*, because a rate-8 circle
        // drains 9/s in base cost before a single plant is counted and the
        // economy had closed both of them by frame 1,600. Measuring growth
        // and measuring the price at the same time measures neither.
        if std::env::var("PIXEL_PHYSICS_DRUID_UNLIMITED").is_ok_and(|v| v != "0") {
            game.unlimited = true;
        }
        // `PIXEL_PHYSICS_DRUID_CENSUS=N` -- after N ticks, print living plant
        // tissue inside each standing circle and exit. The instrument for the
        // speed dial: "did it fire" needs a counter, and per-circle is the
        // only granularity that can tell a working dial from a world that
        // simply runs fast everywhere.
        let census_after: Option<u64> = std::env::var("PIXEL_PHYSICS_DRUID_CENSUS").ok().and_then(|v| v.parse().ok());
        // `PIXEL_PHYSICS_DRUID_ABSORB_AT=N` -- press `F` at player tick N.
        // The drawn energy is in flight for `DRAW_FRAMES` and then gone, so
        // catching it needs the press and the screenshot to be scheduled
        // together; a headless run cannot press anything.
        let absorb_at: Option<u64> = std::env::var("PIXEL_PHYSICS_DRUID_ABSORB_AT").ok().and_then(|v| v.parse().ok());
        // `PIXEL_PHYSICS_DRUID_GIF=start,every,count[,out.gif]` -- capture an
        // animation instead of a still.
        //
        // **This exists because a still is the wrong instrument for a flow,
        // and it cost two wrong answers to learn.** The owner judged the
        // drawn-energy effect from single frames twice -- "a couple big orbs"
        // once, and before that a frame with nothing in it at all -- and both
        // readings were fair, because a stream of particles is a thing that
        // *moves* and a photograph of one is a scatter of dots. `filmstrip`
        // has had `gif=1` for exactly this reason for a while; it just cannot
        // drive this game.
        let gif = std::env::var("PIXEL_PHYSICS_DRUID_GIF").ok().and_then(|v| {
            let n: Vec<&str> = v.split(',').collect();
            let (start, every, count) = (n.first()?.trim().parse().ok()?, n.get(1)?.trim().parse().ok()?, n.get(2)?.trim().parse().ok()?);
            let out = n.get(3).map_or_else(|| std::env::temp_dir().join("pixel_physics_druid.gif"), |o| o.trim().into());
            Some(GifCapture { start, every, count, out, frames: Vec::new() })
        });
        // `PIXEL_PHYSICS_DRUID_WALK=N` -- hold `D` for the first N player
        // ticks. A colony is founded at the gnome's feet, so a scripted
        // absorb has about five cells for the stream to cross and the flow
        // reads as a flash; walking him off first is the difference between
        // rendering the feature and rendering a sparkle.
        // A range, not a prefix: the colony has to charge *first*, and it
        // only charges while it is inside running time -- which, when he is
        // standing with it, is his own carried circle. So the script is
        // stand, then step off, then pull.
        let walk: (u64, u64) = std::env::var("PIXEL_PHYSICS_DRUID_WALK")
            .ok()
            .and_then(|v| {
                let (a, b) = v.split_once(',')?;
                Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
            })
            .unwrap_or((0, 0));
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
            census_after,
            absorb_at,
            gif,
            walk,
            result: Ok(()),
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        self.result = Err(message.into());
        event_loop.exit();
    }

    fn title(&self) -> String {
        let g = &self.game;
        // **The pool, and the two rates that move it.** A number on its own
        // cannot say whether you are winning: `power 412` is the same reading
        // whether it is climbing or about to run out, and the rates are the
        // whole question the game asks.
        let purse = if g.unlimited {
            "power UNLIMITED".to_string()
        } else {
            format!("power {:.0} ({:+.1}/s)", g.power, g.income - g.drain)
        };
        format!(
            "The Held World — {:.0} fps — {} — {} — {} circle{} r{} — {} animals — look {}{}",
            self.fps,
            if g.world.held { "HELD" } else { "running" },
            purse,
            g.world.quickenings.len(),
            if g.world.quickenings.len() == 1 { "" } else { "s" },
            g.place_radius,
            // **The count, not just the fact.** A founding that placed nobody
            // and a founding that took look identical on screen at play zoom,
            // which is `CLAUDE.md`'s "did it fire at all needs a counter".
            g.world.live_creature_count(),
            g.renderer.held_look.label(),
            if g.paused { " — PAUSED" } else { "" },
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
        self.game.player_input.right = self.held.right || (self.game.ticks >= self.walk.0 && self.game.ticks < self.walk.1);
        self.game.player_input.jump_held = self.held.jump;
        self.game.player_input.down = self.held.down;
        self.game.player_input.grab = self.held.grab;
        self.game.player_input.jump_pressed |= std::mem::take(&mut self.jump_pressed);

        if let Some(n) = self.absorb_at {
            if self.game.ticks >= n {
                self.absorb_at = None;
                if self.game.absorb() > 0.0 {
                    // **The pull schedules its own screenshot**, because the
                    // two clocks do not line up: `screenshot_countdown` counts
                    // *drawn frames* and this counts *player ticks*, and on a
                    // software rasteriser a drawn frame is worth several
                    // ticks. Scheduling both by hand produced two renders
                    // with no flow in them and a wrong story about why (I
                    // blamed the colony starving; it had 68 charge).
                    // `PIXEL_PHYSICS_DRUID_CATCH=N` picks how far along the
                    // stream is when the shutter opens.
                    let n = std::env::var("PIXEL_PHYSICS_DRUID_CATCH").ok().and_then(|v| v.parse().ok());
                    self.screenshot_countdown = Some(n.unwrap_or(DRAW_FRAMES_TO_CATCH));
                }
            }
        }
        if let Some(n) = self.census_after {
            // Player ticks, not `world.frame` -- see `Druid::ticks`. Keying
            // this on the world's counter is what made the dial's first
            // measurement read a real-time circle at 2 cells against 45.
            if self.game.ticks >= n {
                self.census_after = None;
                census(&self.game);
                event_loop.exit();
                return;
            }
        }

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
                // **Captured after the draw, before the present**, so the
                // interface is in the frame -- the flow this exists to record
                // is drawn by the HUD, not by the world.
                if let Some(g) = &mut self.gif {
                    let t = self.game.ticks;
                    if t >= g.start && g.frames.len() < g.count && (t - g.start).is_multiple_of(g.every) {
                        g.frames.push(pixels.frame().to_vec());
                    }
                    if g.frames.len() >= g.count {
                        let g = self.gif.take().expect("just checked");
                        save_gif(&g);
                        event_loop.exit();
                        return;
                    }
                }
                if let Some(n) = self.screenshot_countdown {
                    if n <= 1 {
                        self.screenshot_countdown = None;
                        println!(
                            "druid: shutter at tick {} — {} draws in flight, {} motes drawn",
                            self.game.ticks,
                            self.game.draws.len(),
                            pixel_physics::druid::hud::mote_count(&self.game)
                        );
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
            KeyCode::KeyP => {
                self.game.paused = !self.game.paused;
            }
            // The key list, and the only way back to it once it is off.
            // `F1` as well as `/` because `/` is a different physical key on
            // a non-US layout and this is the one binding a lost player needs.
            KeyCode::Slash | KeyCode::F1 => self.game.show_keys = !self.game.show_keys,
            // **How held ground is drawn.** A selector rather than a
            // decision, because this is precisely the question no amount of
            // argument settles -- see `render::HeldLook`.
            KeyCode::KeyL => {
                self.game.renderer.cycle_held_look();
                let look = self.game.renderer.held_look.label();
                self.game.note(format!("held ground drawn: {look}"));
            }
            // **Found a colony where you are standing.** It has to be inside
            // running time to tick at all, and the circle you carry is at
            // your feet -- see `Druid::found_colony`.
            KeyCode::KeyC => {
                self.game.found_colony();
            }
            // **The verb the whole game is built on.** A seed sown on held
            // ground lies there until a circle reaches it -- see
            // `Druid::plant_seed`.
            KeyCode::KeyT => {
                self.game.plant_seed();
            }
            // **Draw the colony's charge.** The verb the economy is built on
            // -- see `Druid::absorb`.
            KeyCode::KeyF => {
                self.game.absorb();
            }
            KeyCode::Tab => self.game.cycle_seed_kind(),
            // The economy's verb: a circle that runs while you are elsewhere.
            KeyCode::Space => {
                self.game.place_quickening();
            }
            KeyCode::KeyX => {
                self.game.lift_quickening();
            }
            // No note for these two: the radius is on the readout and the
            // preview ring at his feet resizes as he presses them, so a
            // message would be a third copy of a fact already on screen twice.
            // The speed dial. `Z`/`V` rather than more letters near the
            // movement keys, and both are free.
            KeyCode::KeyZ => self.game.speed = (self.game.speed - 1).max(pixel_physics::druid::SPEED_MIN),
            KeyCode::KeyV => self.game.speed = (self.game.speed + 1).min(pixel_physics::druid::SPEED_MAX),
            // **Your own circle.** `[`/`]` because that is brush size in the
            // sandbox and this is the same gesture: how far your hand reaches.
            KeyCode::BracketLeft => {
                self.game.world.carried_radius =
                    (self.game.world.carried_radius - 8).max(pixel_physics::sim::world::CARRIED_RADIUS)
            }
            KeyCode::BracketRight => {
                self.game.world.carried_radius =
                    (self.game.world.carried_radius + 8).min(pixel_physics::druid::CARRIED_RADIUS_MAX)
            }
            KeyCode::KeyQ => self.game.place_radius = (self.game.place_radius - 10).max(pixel_physics::druid::PLACE_RADIUS_MIN),
            KeyCode::KeyE => self.game.place_radius = (self.game.place_radius + 10).min(pixel_physics::druid::PLACE_RADIUS_MAX),
            // **Unlimited power, for playtesting.** The economy's numbers are
            // first guesses and nobody has played this, so being able to take
            // them out of the way is what makes the mechanics judgeable at
            // all -- the owner's own lab ruling, applied here.
            KeyCode::KeyU => {
                self.game.unlimited = !self.game.unlimited;
                let state = if self.game.unlimited { "on" } else { "off" };
                self.game.note(format!("unlimited power {state}"));
            }
            // **Release the world, or hold it again.** The single most useful
            // key for judging this game: the look the owner picked has no
            // colour tell, so whether "held" reads at all is a question you
            // answer by flipping it and watching, not by looking at a still.
            KeyCode::KeyH => {
                self.game.world.held = !self.game.world.held;
                let hold = if self.game.world.held { pixel_physics::sim::clock::SkyPin::Noon.hold() } else { None };
                self.game.world.set_sky_hold(hold);
                self.game.note(if self.game.world.held { "the world is held" } else { "the world is running" });
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

/// **Living plant tissue inside each standing circle**, and outside all of
/// them, at whatever frame the caller asked for.
///
/// Per circle rather than world-wide, which is the whole point: a world-wide
/// count cannot tell a speed dial that works from one that runs everything
/// fast, and those are exactly the two things to distinguish.
fn census(game: &Druid) {
    let w = &game.world;
    let mut inside = vec![0usize; w.quickenings.len()];
    let mut outside = 0usize;
    for id in w.live_organism_ids() {
        let Some(state) = w.organism(id) else { continue };
        // **Senescent is not living, and in this game that is most of the
        // world.** `Start::Dead` marks every plant senescent, so a census
        // that counted them would report a dead wood as thriving -- the
        // metric answering a different question than the one asked.
        if w.species.get(state.species).creature.is_some() || state.senescent {
            continue;
        }
        let cells = state.cells.len();
        match w.quickenings.iter().position(|q| state.cells.keys().next().is_some_and(|(x, y)| q.contains(*x, *y))) {
            Some(i) => inside[i] += cells,
            None => outside += cells,
        }
    }
    println!("druid census at frame {} (player ticks {}, speed x{}):", w.frame, game.ticks, game.speed);
    for (i, q) in w.quickenings.iter().enumerate() {
        println!("  circle {i} at {},{} r{} : {} living plant cells", q.x, q.y, q.r, inside[i]);
    }
    println!("  outside every circle : {outside} living plant cells");
    let (charge, holders) = game.charge_in_reach();
    println!("  animals {} ({} awake), charge {charge:.0} in {holders} within reach", game.animals, game.animals_awake);
}

/// Write the captured frames out as a looping animation.
///
/// The delay is derived from the capture interval and the fixed 60 ticks a
/// second, so the result plays at the speed the game actually ran — the whole
/// point being to judge motion, which a GIF at an arbitrary rate cannot do.
fn save_gif(g: &GifCapture) {
    let delay_ms = (g.every * 1000 / u64::from(TICKS_PER_SECOND)).max(16);
    let delay = image::Delay::from_saturating_duration(Duration::from_millis(delay_ms));
    let file = match std::fs::File::create(&g.out) {
        Ok(f) => f,
        Err(e) => return eprintln!("gif failed: {e}"),
    };
    let mut encoder = image::codecs::gif::GifEncoder::new(file);
    if let Err(e) = encoder.set_repeat(image::codecs::gif::Repeat::Infinite) {
        return eprintln!("gif failed: {e}");
    }
    for f in &g.frames {
        let Some(buf) = image::RgbaImage::from_raw(WIDTH, HEIGHT, f.clone()) else {
            return eprintln!("gif failed: a frame was not {WIDTH}x{HEIGHT}");
        };
        if let Err(e) = encoder.encode_frame(image::Frame::from_parts(buf, 0, 0, delay)) {
            return eprintln!("gif failed: {e}");
        }
    }
    drop(encoder);
    eprintln!("gif saved ({} frames, {delay_ms}ms apart): {}", g.frames.len(), g.out.display());
}

/// Its own filename, so a druid screenshot and a sandbox one can both exist.
fn save_framebuffer_png(rgba: &[u8], width: u32, height: u32) {
    let path = std::env::temp_dir().join("pixel_physics_druid_screenshot.png");
    match image::save_buffer(&path, rgba, width, height, image::ColorType::Rgba8) {
        Ok(()) => eprintln!("screenshot saved: {}", path.display()),
        Err(e) => eprintln!("screenshot failed: {e}"),
    }
}
