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
use pixel_physics::druid::hud::{self, Action};
use pixel_physics::druid::{Druid, TICKS_PER_SECOND};
use pixel_physics::lab::stats::Stats;
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// The simulation advances at a fixed rate regardless of frame rate, for the
/// reason `main.rs` gives: every CA rule is "one cell per step" rather than a
/// velocity, so a variable timestep changes behaviour with the frame rate.
/// The rate itself is `druid::TICKS_PER_SECOND`, in the lib, because the
/// game prices things per second and the loop is not the only reader.
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
    /// `G` — laying scent, held rather than tapped.
    laying: bool,
    /// A jump press seen since the last frame's input assembly. Separate from
    /// `held.jump` so a press and release faster than one frame still jumps.
    jump_pressed: bool,
    /// `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES` — the same hook the other two
    /// binaries carry, and the only way to see a rendered window on a
    /// headless box, since this build's swapchain is invisible to OS capture.
    screenshot_countdown: Option<u32>,
    /// **`N` — restart, counted in drawn frames.** Owner's playtest,
    /// 2026-09-14: *"there should be a restart option."*
    ///
    /// Not acted on the same frame the key arrives. `Druid::new` generates
    /// 2560x960 and grows it -- about a minute of wall clock -- on this
    /// thread, with nothing else able to run while it does, so a restart
    /// that fires immediately blocks the window the instant the key is
    /// pressed and the message this counts down to would never be drawn at
    /// all: the freeze and the notice would race, and the freeze always
    /// wins. `Some(1)` at the keypress buys one drawn-and-presented frame
    /// with `Druid::note`'s message on screen before `Handler::
    /// perform_restart` actually blocks -- see `Handler::request_restart`.
    restart_countdown: Option<u32>,
    /// See `PIXEL_PHYSICS_DRUID_CENSUS`.
    census_after: Option<u64>,
    /// See `PIXEL_PHYSICS_DRUID_ABSORB_AT`.
    absorb_at: Option<u64>,
    /// See `PIXEL_PHYSICS_DRUID_SMALL`.
    small_at: Option<u64>,
    /// `PIXEL_PHYSICS_DRUID_FOUND_AT` — the tick to commit the open offer on.
    found_at: Option<u64>,
    /// See `PIXEL_PHYSICS_DRUID_GIF`.
    gif: Option<GifCapture>,
    /// See `PIXEL_PHYSICS_DRUID_WALK`.
    walk: (u64, u64),
    /// See `PIXEL_PHYSICS_DRUID_LAY`.
    lay: (u64, u64),
    /// **The button bar's own state — not on `Druid`.** `src/druid/mod.rs`
    /// is a different lane's file for the length of this program (see
    /// `Reports/lanes/druid-screen.md`), so the cursor, the press-armed
    /// action and last frame's laid-out bar live here instead. See
    /// `hud`'s button-bar section doc for the full reasoning.
    ///
    /// Where the cursor is, in framebuffer pixels — `None` once it has left
    /// the window. `pixels.window_pos_to_pixel` is the only conversion this
    /// needs: `Druid` never zooms or resizes its buffer away from the
    /// window (`hud::Interface::draw`'s own comment on `Hud::new(w, h, 1)`),
    /// unlike `src/bin/lab.rs`'s `to_logical` divide.
    cursor: Option<(i32, i32)>,
    /// Last frame's laid-out bar — retained so a click arriving between
    /// frames is tested against the bar the player was actually looking at,
    /// the same reason `lab::ui::Ui::bar` is retained.
    bar: hud::Bar,
    /// The action a press armed, so a button fires on release over *itself*
    /// rather than on press — see `Handler::act`'s call sites below.
    bar_pressed: Option<Action>,
    /// **The biosphere page (item 1 of the playtest)** — also not on
    /// `Druid`, for the same reason the bar's own state is not.
    stats: Stats,
    /// Last frame's `(Stats::rect, cursor)`, `None` when the page was
    /// closed — the repaint decision for the one piece of the interface
    /// `Druid::draw`'s own `ui_changed` does not reach. See `Handler::frame`.
    last_stats_state: Option<StatsState>,
    result: Result<(), Box<dyn std::error::Error>>,
}

/// A stats-page rectangle and the cursor at the time it was drawn — see
/// `Handler::last_stats_state`.
type StatsState = ((i32, i32, i32, i32), Option<(i32, i32)>);

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
        // `PIXEL_PHYSICS_DRUID_LOOK=onehue|unchanged` -- start in a look
        // rather than reaching it with a key, because a headless screenshot
        // cannot press one and the whole point of the selector is to be
        // compared.
        //
        // **Both spellings, since one hue became the default (2026-09-14).**
        // This hook could only ever *select* `onehue`, which was enough while
        // the default was the other one and is now a hook that cannot reach
        // half its own selector -- the comparison the selector exists for
        // would have had no way to render the baseline.
        if let Ok(v) = std::env::var("PIXEL_PHYSICS_DRUID_LOOK") {
            use pixel_physics::render::HeldLook;
            match v.to_ascii_lowercase().as_str() {
                "onehue" => game.renderer.held_look = HeldLook::OneHue,
                "unchanged" | "plain" => game.renderer.held_look = HeldLook::Unchanged,
                other => println!("druid: PIXEL_PHYSICS_DRUID_LOOK={other:?} is not a look -- onehue or unchanged"),
            }
        }
        // `PIXEL_PHYSICS_DRUID_FOUND=1` -- found a colony at startup, for the
        // same reason as the look above: a headless screenshot cannot press
        // `C`, and "did the founding place anybody" is a question with a
        // number rather than a picture.
        // `PIXEL_PHYSICS_DRUID_KEYS=0` -- start with the legend hidden. It
        // is twenty rows tall and covers the lower half of a 512x320 frame,
        // which is exactly where the ground is; a headless run cannot press
        // `/`.
        if std::env::var("PIXEL_PHYSICS_DRUID_KEYS").is_ok_and(|v| v == "0") {
            game.show_keys = false;
        }
        if std::env::var("PIXEL_PHYSICS_DRUID_FOUND").is_ok_and(|v| v != "0") {
            game.found_colony();
        }
        // `PIXEL_PHYSICS_DRUID_ZOOM=N` -- start at that zoom rung, because a
        // headless run cannot press `=`. Needed to render the shrink at all:
        // at 2x3 she is six pixels at zoom 1, so a contact sheet of the
        // feature at the default rung is a picture of the ground with
        // nothing in it, which reads as "the feature does nothing".
        if let Some(n) = std::env::var("PIXEL_PHYSICS_DRUID_ZOOM").ok().and_then(|v| v.trim().parse::<i32>().ok()) {
            // **Press until the rung is reached, not `n` times.** The first
            // version did `n - 1` presses and never left the default rung,
            // because `adjust_zoom` walks the zoom-*out* stride back to 1
            // before it starts raising `zoom` -- so three presses bought
            // three stride steps and no magnification, and the render came
            // back with a two-pixel gnome that read as "the shrink is
            // invisible" rather than "the hook did nothing". A knob nobody
            // can see the value of is a knob nobody can tell is disconnected,
            // so it echoes what it reached.
            let want = n.max(1);
            for _ in 0..64 {
                if game.renderer.zoom >= want && game.renderer.zoom_out_stride <= 1 {
                    break;
                }
                game.renderer.adjust_zoom(1);
            }
            println!("druid: zoom {} (stride {}), asked for {want}", game.renderer.zoom, game.renderer.zoom_out_stride);
        }
        // `PIXEL_PHYSICS_DRUID_MENU=1` -- open the options menu at startup,
        // and `=<n>` to put the cursor on the nth row. Same shape and same
        // reason as every hook here: a headless screenshot cannot press `M`,
        // and a menu is exactly the thing a still image *can* settle.
        if let Ok(v) = std::env::var("PIXEL_PHYSICS_DRUID_MENU") {
            if v != "0" {
                game.toggle_menu();
                if let (Ok(n), Some(m)) = (v.parse::<i32>(), game.menu.as_mut()) {
                    m.step(n);
                }
            }
        }
        // `PIXEL_PHYSICS_DRUID_OFFER=1` -- open the founding screen at
        // startup, and `=<n>` to put the cursor on the nth lineage. The
        // screen is the one part of this game a still image *can* settle, so
        // it is the one that most needs to be reachable without a keyboard.
        if let Ok(v) = std::env::var("PIXEL_PHYSICS_DRUID_OFFER") {
            if v != "0" {
                game.toggle_founding();
                if let (Ok(n), Some(offer)) = (v.parse::<i32>(), game.offer.as_mut()) {
                    offer.step_pick(n);
                }
            }
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
        // `PIXEL_PHYSICS_DRUID_FOUND_AT=<tick>` -- commit whatever the offer is
        // showing, at that player tick. Same shape and same reason as
        // `ABSORB_AT` above: the founding throws a flow, and a flow is a
        // claim about several frames that a screenshot scheduled by hand
        // will miss. Pair it with `PIXEL_PHYSICS_DRUID_OFFER` to choose
        // which lineage.
        let found_at: Option<u64> = std::env::var("PIXEL_PHYSICS_DRUID_FOUND_AT").ok().and_then(|v| v.parse().ok());
        // `PIXEL_PHYSICS_DRUID_SMALL=<tick>` -- press `R` at that player tick.
        // The control arm for the shrink, so a before and an after can be
        // rendered off one binary.
        //
        // **A tick rather than a flag, and that distinction cost a render.**
        // The first version shrank her in `Druid::new` and was refused every
        // time, with the on-screen refusal the only thing that differed
        // between the two arms. `spawn_point` returns a *surface* cell and
        // `Player::at_scaled` CENTRES the body on it, so at construction her
        // feet are seven rows inside the ground -- `try_resize` anchors on
        // the feet, so the target rect was buried and declining it was
        // correct. She has to have landed first. `CLAUDE.md`'s *a scene that
        // contradicts the code will look like a bug in the code*, caught by
        // looking at the render rather than by any test.
        let small_at: Option<u64> = std::env::var("PIXEL_PHYSICS_DRUID_SMALL").ok().and_then(|v| v.trim().parse().ok());
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
        // `PIXEL_PHYSICS_DRUID_LAY=a,b` -- hold `G` between those player
        // ticks, the same shape as `WALK` above and normally paired with it:
        // a trail is a route walked while holding a key, so a headless run
        // needs both halves or it lays one dot and calls it a trail.
        let lay: (u64, u64) = std::env::var("PIXEL_PHYSICS_DRUID_LAY")
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
            laying: false,
            jump_pressed: false,
            screenshot_countdown: std::env::var("PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES").ok().and_then(|v| v.parse().ok()),
            restart_countdown: None,
            census_after,
            absorb_at,
            small_at,
            found_at,
            gif,
            walk,
            lay,
            cursor: None,
            bar: hud::Bar::default(),
            bar_pressed: None,
            stats: Stats::new(),
            last_stats_state: None,
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

        // **`restart_countdown` reaching 0 is the frame that actually pays
        // for it.** The frame that set it to `Some(1)` (`request_restart`)
        // already drew and presented the "regenerating" message on the
        // *old* game before this one runs -- see that field's own doc.
        // Everything below this block, for the rest of this `frame` call,
        // runs against the freshly generated `self.game`.
        if let Some(n) = self.restart_countdown {
            if n == 0 {
                self.restart_countdown = None;
                self.perform_restart();
            } else {
                self.restart_countdown = Some(n - 1);
            }
        }

        // Held state copied fresh; the jump press ORs in, so a press made on
        // a frame that ran zero ticks survives until a tick consumes it.
        self.game.player_input.left = self.held.left;
        self.game.player_input.right = self.held.right || (self.game.ticks >= self.walk.0 && self.game.ticks < self.walk.1);
        self.game.player_input.jump_held = self.held.jump;
        self.game.player_input.down = self.held.down;
        self.game.player_input.grab = self.held.grab;
        self.game.player_input.jump_pressed |= std::mem::take(&mut self.jump_pressed);

        if let Some(n) = self.found_at {
            if self.game.ticks >= n {
                self.found_at = None;
                if self.game.offer.is_none() {
                    self.game.toggle_founding();
                }
                // **The founding schedules its own screenshot**, for the
                // reason the pull below already learned the hard way: the
                // countdown counts *drawn frames* and this counts *player
                // ticks*, and on a software rasteriser one drawn frame is
                // worth several ticks. Scheduling both by hand renders the
                // moment after the flow has finished and reads as the flow
                // not existing.
                if self.game.commit_founding() > 0 {
                    let catch = std::env::var("PIXEL_PHYSICS_DRUID_CATCH").ok().and_then(|v| v.parse().ok());
                    self.screenshot_countdown = Some(catch.unwrap_or(1));
                }
            }
        }

        if let Some(n) = self.small_at {
            if self.game.ticks >= n {
                self.small_at = None;
                // **Loud when it refuses.** A hook that quietly did nothing
                // reads as "the feature is not wired", which is the
                // disconnected-knob trap by name -- and is exactly how the
                // buried-feet bug above presented.
                if self.game.toggle_small() {
                    println!("druid: small at tick {n} -- {:?}", self.game.world.player.as_ref().map(|p| (p.w, p.h)));
                    // **The shrink schedules its own shutter**, the same way
                    // the pull below does and for the same reason: the two
                    // clocks do not line up. `screenshot_countdown` counts
                    // *drawn frames* and this counts *player ticks*, and on a
                    // software rasteriser one drawn frame is worth several
                    // ticks. Scheduled by hand it fired at tick 36 against a
                    // shrink at 40, so the render came back showing her at
                    // full size with the button unlatched -- a picture of the
                    // feature not working, taken four ticks too early.
                    // Overridden by `PIXEL_PHYSICS_DRUID_CATCH` like the pull.
                    let catch = std::env::var("PIXEL_PHYSICS_DRUID_CATCH").ok().and_then(|v| v.parse().ok());
                    self.screenshot_countdown = Some(catch.unwrap_or(2));
                } else {
                    println!("druid: PIXEL_PHYSICS_DRUID_SMALL={n} was refused -- no room where she stands at that tick");
                }
            }
        }

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
            // **Inside the tick loop, and it was outside it.** `lay_trail` is
            // priced per second and divides by `TICKS_PER_SECOND`, so calling
            // it once per *frame* charged a fast machine less than a slow one
            // for the same walk -- and laid one mark per frame instead of one
            // per cell, which is the whole reason `TICK` exists. Caught by a
            // counter and not by the picture: a headless run walked 239 cells
            // and reported `trail 1 marks`.
            if self.laying || (self.game.ticks >= self.lay.0 && self.game.ticks < self.lay.1) {
                self.game.lay_trail();
            }
            self.game.update();
            // **Per tick, not per drawn frame** — `lab::mod::Lab`'s own
            // call site samples the same way (`self.stats.observe`'s own
            // doc, which is stale about *why* but not about *when*: the
            // gate inside is keyed on `World::frame`, so calling it more
            // often than the interval only costs two integer comparisons).
            // A catch-up frame can run several ticks; observing once at the
            // end would silently thin the sample rate under load exactly
            // when the box is busiest.
            self.stats.observe(&self.game.world);
            self.accumulator -= TICK;
            ticks += 1;
        }
        // Drop the backlog rather than carrying a debt into the next frame.
        if self.accumulator >= TICK {
            self.accumulator = Duration::ZERO;
        }

        // **Laid out every frame, regardless of whether it is drawn.** Pure
        // and cheap (a few dozen `hud::text_width` calls), and keeping it
        // live even behind a modal means `self.bar` never goes stale for the
        // click that arrives the instant the modal closes.
        self.bar = hud::bar_layout(&self.game, self.stats.showing());
        let show_overlay = self.game.menu.is_none() && self.game.offer.is_none();

        // **The bar needs no repaint decision of its own — the biosphere
        // page does, and that split is the measured reason.** The bar's
        // outer plate is a fixed rectangle, unconditionally fully repainted
        // every drawn frame regardless of hover or which widgets it holds
        // (`hud::draw_bar` fills the whole strip before painting a single
        // widget), so nothing under it can ever go stale — see `hud`'s
        // button-bar section doc. Measured at 54 us/frame
        // (`PIXEL_PHYSICS_DRUID_BENCH_BAR=1`), so drawing it every frame
        // costs nothing worth avoiding.
        //
        // **The biosphere page is not that shape**, and folding it into the
        // same "always paint, never force a repaint" rule would be a real
        // bug rather than merely a missed optimisation: `Stats::rect`'s
        // bottom edge tracks its content (`stats.rs`'s own doc on why it is
        // not a fixed box), so a frame where the page gets *shorter* —
        // fewer generation buckets, a species that stops standing anywhere
        // — leaves old page pixels below the new, smaller bottom edge with
        // nothing repainting them, exactly the smear `hud::Interface`'s own
        // `last_ui` comparison exists to prevent. So this is that same
        // comparison, aimed at the one piece of screen `Interface` does not
        // own: force a full world repaint on any frame the page's own
        // rectangle changed shape or came in or out of the game entirely.
        // Measured 2026-09-14: the page's own paint is **1.11 ms/frame**
        // (`PIXEL_PHYSICS_DRUID_BENCH_BAR=1`, same run) against 0.055 ms for
        // the bar, which is exactly why it is not simply drawn unconditionally
        // like the bar is — that cost only belongs on the handful of frames
        // a world repaint would have happened anyway, not on every one of
        // sixty a second while the page sits open and idle.
        //
        // **The cursor rides along in the same comparison, for the same
        // reason.** `Stats::draw_at_floor` also draws a hover-note popup
        // wherever the cursor sits over an explainable row
        // (`stats.rs`'s own `draw_note`), and that popup's position is not
        // a function of `Stats::rect` at all — it moves and vanishes with
        // the cursor alone. The same smear the rect comparison guards
        // against applies to it: a note drawn last frame and gone (or moved)
        // this frame leaves its old pixels unrepainted unless something
        // forces the world underneath it fresh. Comparing the cursor
        // position whenever the page is open is the coarse, honest fix —
        // costlier than tracking the note's own rectangle would be, but
        // that rectangle is not exposed and re-deriving it a second time
        // here would be the very side-table duplication this module's own
        // doc warns against.
        let stats_state = show_overlay.then(|| (self.stats.rect(&self.game.world, hud::bar_top()), self.cursor));
        let stats_shape_changed = stats_state != self.last_stats_state;
        self.last_stats_state = stats_state;

        let render_error = match &mut self.pixels {
            Some(pixels) => {
                self.game.draw(pixels.frame_mut(), (WIDTH, HEIGHT), stats_shape_changed);
                // **Hidden while a modal owns the screen.** The options menu
                // and the founding screen are centred over the world and
                // (for the founding screen especially) can reach close to
                // the bar's row — and both already own the keyboard
                // exclusively while open, so the mouse should agree rather
                // than let a click reach a button the player cannot see is
                // live. See `Handler::mouse_button` for the matching input
                // guard.
                if show_overlay {
                    hud::draw_bar(&self.bar, pixels.frame_mut(), (WIDTH, HEIGHT), self.cursor, self.bar_pressed);
                    self.stats.draw_at_floor(
                        pixel_physics::render::Hud::new(WIDTH, HEIGHT, 1),
                        pixels.frame_mut(),
                        &self.game.world,
                        self.cursor,
                        hud::bar_top(),
                    );
                }
                // `PIXEL_PHYSICS_DRUID_BENCH_BAR=1` -- print what the bar and
                // the biosphere page each cost to paint, once, ten ticks in.
                // The instrument the repaint decision above is measured
                // against: `examples/ascii.rs` does not reach this game, and
                // a paint call is otherwise timed only by the frame it sits
                // inside, which mixes it with everything else the frame did.
                if self.game.ticks == 10 && std::env::var("PIXEL_PHYSICS_DRUID_BENCH_BAR").is_ok() {
                    let n = 2000u32;
                    let mut scratch = pixels.frame().to_vec();
                    let t0 = std::time::Instant::now();
                    for _ in 0..n {
                        hud::draw_bar(&self.bar, &mut scratch, (WIDTH, HEIGHT), self.cursor, self.bar_pressed);
                    }
                    let bar_ns = t0.elapsed().as_nanos() as f64 / n as f64;
                    let t1 = std::time::Instant::now();
                    for _ in 0..n {
                        self.stats.draw_at_floor(pixel_physics::render::Hud::new(WIDTH, HEIGHT, 1), &mut scratch, &self.game.world, self.cursor, hud::bar_top());
                    }
                    let stats_ns = t1.elapsed().as_nanos() as f64 / n as f64;
                    eprintln!("druid: bar paint {:.3} us/frame, stats paint {:.3} us/frame, combined {:.3} us/frame", bar_ns / 1000.0, stats_ns / 1000.0, (bar_ns + stats_ns) / 1000.0);
                }
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
        // **While the founding screen is up it owns every binding.** Handled
        // before the main list rather than by adding a guard to each arm:
        // eighteen arms each remembering to check is eighteen chances to
        // forget, and the one that forgets is a key that quietly still works
        // behind a modal screen.
        // **The options menu owns the keyboard while it is up**, and it is
        // checked before the founding screen so exactly one modal can ever be
        // taking input.
        if let Some(m) = self.game.menu.as_mut() {
            match code {
                KeyCode::KeyW | KeyCode::KeyA => m.step(-1),
                KeyCode::KeyS | KeyCode::KeyD => m.step(1),
                KeyCode::Space => {
                    let setting = m.current();
                    setting.advance(&mut self.game);
                }
                KeyCode::KeyM | KeyCode::KeyX | KeyCode::Escape => self.game.menu = None,
                _ => {}
            }
            return;
        }
        if let Some(offer) = self.game.offer.as_mut() {
            match code {
                KeyCode::KeyA => offer.step_pick(-1),
                KeyCode::KeyD => offer.step_pick(1),
                // **Three dials, and the body is the one the owner asked
                // for**: *"more flexibility, especially on body shape and
                // movement."* `Q`/`E` is the radius dial outside this screen,
                // so it is the natural "cycle the thing you are sizing" here.
                KeyCode::KeyQ => offer.step_body(-1),
                KeyCode::KeyE => offer.step_body(1),
                KeyCode::KeyZ => offer.step_founders(-1),
                KeyCode::KeyV => offer.step_founders(1),
                KeyCode::KeyC => {
                    self.game.commit_founding();
                }
                // **Escape closes the screen rather than the game.** Quitting
                // out of a modal is the classic way to lose a session to one
                // keystroke, and `X` -- lift, elsewhere -- is the natural
                // "put this down" here too.
                KeyCode::KeyX | KeyCode::Escape => self.game.offer = None,
                _ => {}
            }
            return;
        }
        match code {
            KeyCode::Escape => event_loop.exit(),
            // Every arm below that used to act on `self.game` directly now
            // goes through `Handler::act` — see its own doc for why that is
            // the single dispatch point rather than `Druid::act` alone.
            KeyCode::KeyP => self.act(Action::TogglePause),
            // The key list, and the only way back to it once it is off.
            // `F1` as well as `/` because `/` is a different physical key on
            // a non-US layout and this is the one binding a lost player needs.
            // Not a bar button: it is the legend's own visibility switch, and
            // a button for "show me more buttons" is not one of the *main*
            // actions the owner asked to see.
            KeyCode::Slash | KeyCode::F1 => self.game.show_keys = !self.game.show_keys,
            // **How held ground is drawn.** A selector rather than a
            // decision, because this is precisely the question no amount of
            // argument settles -- see `render::HeldLook`.
            KeyCode::KeyL => self.act(Action::CycleLook),
            // **The options menu.** Settings rather than verbs -- see
            // `druid::menu` for why they are not more keys.
            KeyCode::KeyM => self.act(Action::ToggleOptions),
            // **Found a colony where you are standing.** It has to be inside
            // running time to tick at all, and the circle you carry is at
            // your feet -- see `Druid::found_colony`.
            KeyCode::KeyC => self.act(Action::FoundColony),
            // **The verb the whole game is built on.** A seed sown on held
            // ground lies there until a circle reaches it -- see
            // `Druid::plant_seed`.
            KeyCode::KeyT => self.act(Action::SowSeed),
            // **Draw the colony's charge.** The verb the economy is built on
            // -- see `Druid::absorb`.
            KeyCode::KeyF => self.act(Action::Absorb),
            // **Which seed `T` sows.** Moved off `TAB`, which now opens the
            // biosphere page below -- matching the key the lab already uses
            // for its own, so a player who has touched both games gets the
            // same reflex.
            KeyCode::KeyK => self.act(Action::CycleSeedKind),
            // **The biosphere page (item 1 of the playtest)** -- births,
            // deaths, population. Not routed through `Druid::act`: the page
            // lives on `Handler`, not on `Druid` -- see `Handler::act`.
            KeyCode::Tab => self.act(Action::ToggleStats),
            // **Which plane `G` writes to.** Its own key rather than a second
            // press of `G`, which is the lab's idiom for the same verb --
            // `G` is *held* here rather than armed, so a second press cannot
            // mean anything different from the first.
            KeyCode::KeyI => self.act(Action::CycleScent),
            // **Small enough to go underground.** `R` for the shape she
            // takes rather than for a word -- the free keys left were `R`,
            // `N` and `B`, and this is the only verb among them that is
            // about *her* rather than about the world. See
            // `Druid::toggle_small`: the geometry decides whether it works,
            // and growing back can be refused.
            KeyCode::KeyR => self.act(Action::ToggleSmall),
            // **Zoom, and it is part of the shrink rather than a nicety.**
            // At her own size she is 7x14 pixels at play zoom; at 2x3 she is
            // a six-pixel blob, and so is the gallery she is standing in.
            // `Renderer::adjust_zoom` and the five magnify looks were built
            // and reviewed for the lab already -- this game simply never
            // bound them, so this is a binding and not a feature, and it
            // touches no line of `render.rs`.
            //
            // `Equal` and `Minus` rather than `+`/`-`, because the unshifted
            // key is what a player actually presses and the legend says
            // `- =` for the same reason.
            KeyCode::Equal => self.act(Action::Zoom(1)),
            KeyCode::Minus => self.act(Action::Zoom(-1)),
            // The economy's verb: a circle that runs while you are elsewhere.
            KeyCode::Space => self.act(Action::PlaceCircle),
            KeyCode::KeyX => self.act(Action::LiftCircle),
            // No note for these two: the radius is on the readout and the
            // preview ring at his feet resizes as he presses them, so a
            // message would be a third copy of a fact already on screen
            // twice. **Continuous dials, kept on the keyboard rather than
            // given a button**: a button pressed dozens of times to walk a
            // ladder is not a control (`lab::ui::STOCK_LADDER`'s own doc
            // makes the same call for its stocking dial). The speed dial.
            // `Z`/`V` rather than more letters near the movement keys, and
            // both are free.
            KeyCode::KeyZ => self.game.speed = (self.game.speed - 1).max(pixel_physics::druid::SPEED_MIN),
            KeyCode::KeyV => self.game.speed = (self.game.speed + 1).min(pixel_physics::druid::SPEED_MAX),
            // **One dial, both bubbles.** Owner's playtest, 2026-09-14:
            // *"there should just be one bubble control size for the druid
            // and placeable bubbles."* `Q`/`E` used to walk the placed
            // circle alone, with `[`/`]` walking the carried one on its own
            // scale — see `Druid::set_bubble_radius` for the one rule that
            // now drives both from these two keys.
            KeyCode::KeyQ => self.game.set_bubble_radius(self.game.place_radius - 10),
            KeyCode::KeyE => self.game.set_bubble_radius(self.game.place_radius + 10),
            // **Unlimited power, for playtesting.** The economy's numbers are
            // first guesses and nobody has played this, so being able to take
            // them out of the way is what makes the mechanics judgeable at
            // all -- the owner's own lab ruling, applied here.
            KeyCode::KeyU => self.act(Action::ToggleUnlimited),
            // **Release the world, or hold it again.** The single most useful
            // key for judging this game: the look the owner picked has no
            // colour tell, so whether "held" reads at all is a question you
            // answer by flipping it and watching, not by looking at a still.
            KeyCode::KeyH => self.act(Action::ToggleHeld),
            // **Restart.** `N` for "new world" -- the free letters left were
            // `B`, `J`, `N`, `O`, `Y`, and this is the only one of them that
            // reads as the verb. See `Handler::request_restart`.
            KeyCode::KeyN => self.request_restart(),
            _ => {}
        }
    }

    /// **Arm a restart.** Does not restart on this frame -- see
    /// `restart_countdown`'s own doc for why the block has to be deferred a
    /// frame behind the message that announces it.
    ///
    /// A second press while one is already pending is a no-op rather than a
    /// restart of the restart: `Druid::note` would just overwrite the same
    /// message with itself, and there is nothing else pending state could
    /// mean here.
    fn request_restart(&mut self) {
        if self.restart_countdown.is_some() {
            return;
        }
        self.game.note("restarting -- generating a new world, about a minute");
        self.restart_countdown = Some(1);
    }

    /// **The block `request_restart` warned the player about.** Everything
    /// here is state that belongs to the *run*, not to the window -- a
    /// stale accumulator would burn its backlog as catch-up ticks against a
    /// world that was never running while it built up, held movement keys
    /// would walk the new player off whatever he spawns standing on (the
    /// same reason `Handler::act` clears them before a modal opens), and
    /// the biosphere page's history is a chronicle of the *old* population,
    /// which would draw as a graph of a species that no longer exists.
    ///
    /// **Whether the biosphere page was open survives; what it was showing
    /// does not** -- `Stats::showing` is read before the replacement and
    /// restored after, `Stats::new`'s own history starts empty either way.
    fn perform_restart(&mut self) {
        let stats_open = self.stats.showing();
        self.game = Druid::new();
        self.game.note("world restarted");
        self.accumulator = Duration::ZERO;
        self.held = HeldKeys::default();
        self.laying = false;
        self.jump_pressed = false;
        self.stats = Stats::new();
        if !stats_open {
            self.stats.toggle();
        }
        self.last_stats_state = None;
        self.bar_pressed = None;
    }

    /// **The single dispatch point this binary's controls actually route
    /// through** — both `key` above and the bar's click handling in
    /// `window_event` call this and nothing else.
    ///
    /// `hud::Druid::act` (`src/druid/hud.rs`) is the pure game-state half
    /// of it and was meant to be the *whole* of it, matching
    /// `lab::mod::Lab::act`'s own doc: *"the single place a control turns
    /// into a change… there is no second copy of what SPACE does."* It
    /// cannot be, here, for a reason worth recording rather than quietly
    /// working around: `src/druid/mod.rs` is Lane B's file for the length
    /// of this program, so `Druid` cannot gain the fields two of these
    /// actions need — clearing this event loop's own held-movement keys
    /// before a modal steals the keyboard (`ToggleOptions`, `FoundColony`),
    /// and the biosphere page itself (`ToggleStats`), which lives on
    /// `Handler` for the same reason the bar's cursor does. So this
    /// function is the actual single dispatch point, and `Druid::act` is
    /// what it calls for everything that does not need those two.
    fn act(&mut self, action: Action) {
        match action {
            Action::ToggleOptions | Action::FoundColony => {
                // **Stop walking on the way in.** Otherwise a key held at
                // the moment a modal opens stays held -- the release goes to
                // the modal, which does not track it -- and he walks off
                // whatever he was standing on.
                self.held = HeldKeys::default();
                self.laying = false;
                self.game.act(action);
            }
            Action::ToggleStats => self.stats.toggle(),
            _ => self.game.act(action),
        }
    }

    /// **The bar's press/release protocol** — `lab::ui::Ui::press`/`Ui::
    /// release` in miniature. A button fires on *release over the same
    /// button a press armed*, which is what lets a press be taken back by
    /// sliding off it before letting go — the behaviour every other button
    /// in the world has, and cheap here since [`hud::Bar::hit`] is a linear
    /// scan over a dozen rectangles.
    ///
    /// **Suppressed entirely while a modal owns the screen**, matching the
    /// guard in `Handler::frame` that stops the bar from being *painted*
    /// then: the options menu and the founding screen already own the
    /// keyboard exclusively while open, and a click reaching a button
    /// nobody can see would be the mouse disagreeing with the keyboard
    /// about who is in charge.
    fn mouse_button(&mut self, pressed: bool) {
        if self.game.menu.is_some() || self.game.offer.is_some() {
            self.bar_pressed = None;
            return;
        }
        let Some((x, y)) = self.cursor else {
            self.bar_pressed = None;
            return;
        };
        if pressed {
            self.bar_pressed = self.bar.hit(x, y);
        } else if let Some(action) = self.bar_pressed.take() {
            if self.bar.hit(x, y) == Some(action) {
                self.act(action);
            }
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
                    // **The founding screen takes the whole keyboard.** A/D
                    // choose a lineage there, and the same press reaching the
                    // walk would have the gnome stroll off the colony site
                    // while you read about it. Held state is cleared on the
                    // way in (`key`), so he stops rather than keeping the
                    // direction he was going.
                    if self.game.offer.is_some() || self.game.menu.is_some() {
                        self.laying = false;
                        if pressed && !event.repeat {
                            self.key(code, event_loop);
                        }
                        return;
                    }
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
                        // **Held, not tapped**, and so it lives here beside
                        // the walk rather than in `key`: the gesture is
                        // walking a route while holding it down, and a tap
                        // would put one dot on the ground.
                        KeyCode::KeyG => self.laying = pressed,
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
            // **The bar's mouse plumbing.** The druid read no mouse at all
            // before item 3 of the 2026-09-14 playtest — `window_event` had
            // only the four arms above. **The mapping is 1:1 and needs no
            // `to_logical` divide**, unlike `src/bin/lab.rs`'s own cursor
            // handling: the druid never touches `zoom` or a `pixel_budget`,
            // `hud::Interface::draw` hardcodes `Hud::new(w, h, 1)`, and the
            // window is built at exactly `(WIDTH, HEIGHT)` above — so
            // `window_pos_to_pixel` is the whole conversion.
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor =
                    self.pixels.as_ref().and_then(|p| p.window_pos_to_pixel((position.x as f32, position.y as f32)).ok()).map(|(x, y)| (x as i32, y as i32));
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor = None;
                self.bar_pressed = None;
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                self.mouse_button(state == ElementState::Pressed);
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
    // **Where the animals actually are**, which is the only thing that can
    // answer whether a laid trail was followed. A picture cannot: an ant is
    // two cells at this zoom, and "the colony drifted east" and "the colony
    // milled about" look identical on a contact sheet. Reported against the
    // trail's own far end rather than in absolute cells, because the number
    // that matters is *did they close on where he pointed*.
    let mut n = 0usize;
    let (mut sx, mut sy) = (0i64, 0i64);
    for id in w.live_organism_ids() {
        let Some(state) = w.organism(id) else { continue };
        if w.species.get(state.species).creature.is_none() {
            continue;
        }
        let Some((x, y)) = state.cells.keys().next().copied() else { continue };
        n += 1;
        sx += x as i64;
        sy += y as i64;
    }
    if n > 0 {
        let (mx, my) = ((sx / n as i64) as i32, (sy / n as i64) as i32);
        // **Furthest reached, not just the mean**, and the mean is the trap
        // that made the first trail A/B unreadable: a colony lives at its
        // nest, so the mean *is* the nest whatever the animals do, and two
        // arms came back identical to the digit while saying nothing. How far
        // the furthest one got, and how many are near a named point, can tell
        // milling from stillness.
        let east = w
            .live_organism_ids()
            .into_iter()
            .filter_map(|id| w.organism(id))
            .filter(|st| w.species.get(st.species).creature.is_some())
            .filter_map(|st| st.cells.keys().map(|&(x, _)| x).max())
            .max()
            .unwrap_or(0);
        print!("  {n} animals, mean at {mx},{my}, furthest east {east}");
        // `PIXEL_PHYSICS_DRUID_MARK=x,y` -- a fixed reference both arms of a
        // paired run can be counted against. The trail head cannot serve: the
        // arm with no trail has none, so the two arms would be measured with
        // different rulers.
        if let Some((rx, ry)) = std::env::var("PIXEL_PHYSICS_DRUID_MARK").ok().and_then(|v| {
            let (a, b) = v.split_once(',')?;
            Some((a.trim().parse::<i32>().ok()?, b.trim().parse::<i32>().ok()?))
        }) {
            let near = w
                .live_organism_ids()
                .into_iter()
                .filter_map(|id| w.organism(id))
                .filter(|st| w.species.get(st.species).creature.is_some())
                .filter(|st| st.cells.keys().next().is_some_and(|&(x, y)| (x - rx).abs() < 40 && (y - ry).abs() < 40))
                .count();
            print!(" — {near} within 40 of the mark {rx},{ry}");
        }
        if let Some(&(tx, ty)) = game.trail.back() {
            let d = (((tx - mx) as f32).powi(2) + ((ty - my) as f32).powi(2)).sqrt();
            let near = w
                .live_organism_ids()
                .into_iter()
                .filter_map(|id| w.organism(id))
                .filter(|s| w.species.get(s.species).creature.is_some())
                .filter(|s| s.cells.keys().next().is_some_and(|&(x, y)| (x - tx).abs() < 40 && (y - ty).abs() < 40))
                .count();
            print!(" — trail head {tx},{ty}, mean is {d:.0} cells off it, {near} animals within 40");
        }
        println!(" (trail {} marks)", game.trail.len());
    }
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
