//! **The evolution lab — the biosphere the second game is played in.**
//!
//! A sealed box of soil under a grow light, with plants and creatures living
//! in it, that the player can run at speed and read the state of. The design
//! of record is `Reports/evolution-lab-design-guide-2026-08-30.md`; this
//! module is its Gate 1 (*one hand-built box that runs plants and creatures
//! together*) and Gate 3 (*the two-phase loop, end to end*).
//!
//! **It is a second game, not a second engine.** Everything below the scene
//! is the shipped simulation, reached through `sim::frame::step` — the same
//! tick `src/main.rs` runs, in the same order, with nothing skipped. The
//! lab's frame time comes from what is *not in the box* rather than from what
//! is not in the binary: measured (feasibility §3c), a bed with no rock pays
//! 0.028 ms for the structural scheduler against 3.389 ms outdoors, blasts
//! 0.000, particles 0.000, the gnome 0.001. So there is nothing to strip and
//! stripping is what would make this a fork.
//!
//! Four files, and each is owned by one concern so that concurrent work on
//! them does not collide (`CLAUDE.md`, *working alongside another session*):
//!
//! | file | owns |
//! |---|---|
//! | `scene` | the box: geometry, soil, light, partitions, founders |
//! | `time`  | Tending vs Running, the speed dial, and what it actually achieved |
//! | `stats` | the census, and the page that draws it |
//! | this file | `Lab` — the state the three above are wired into, and the frame |
//!
//! **The viewport is `app::WIDTH` x `app::HEIGHT`**, deliberately the same
//! framebuffer the sandbox uses, so `render::Renderer` and `hud` need no
//! second set of geometry and a screenshot from either game is comparable
//! with one from the other.

pub mod scene;
pub mod stats;
pub mod time;

use crate::render::Renderer;
use crate::sim::explosion::Blasts;
use crate::sim::frame;
use crate::sim::particle::ParticleSystem;
use crate::sim::player;
use crate::sim::world::World;

pub use crate::app::{HEIGHT, WIDTH};

/// The whole lab: a world, the systems that live beside the cell grid, a view
/// on it, and the two things the player drives — time and what is on screen.
pub struct Lab {
    pub world: World,
    pub particles: ParticleSystem,
    pub blasts: Blasts,
    pub renderer: Renderer,
    /// Present so `frame::step` gets the same arguments the sandbox gives it.
    /// The lab never summons a gnome, and `player::step` is a no-op without
    /// one — measured at 0.001 ms/frame, which is the whole reason this is a
    /// dormant field rather than a phase the lab skips.
    player_tuning: player::Tuning,
    /// The speed dial and the two-phase loop. See `time`.
    pub time: time::TimeControl,
    /// The census and its page. See `stats`.
    pub stats: stats::Stats,
    /// What the box was built from, kept so a reset rebuilds the same lab and
    /// so the stats page can say what bed these numbers are from.
    pub spec: scene::LabBox,
    /// The key list. On by default on a fresh lab and dismissed by any key,
    /// because there is no other way to discover a control here — the sandbox
    /// grew its bindings a key at a time in front of somebody who already knew
    /// them, and the lab has no such person.
    pub show_help: bool,
}

impl Lab {
    /// Build the box `spec` describes and start it in Tending at 1x.
    pub fn new(spec: scene::LabBox) -> Self {
        let world = spec.build();
        Self {
            world,
            particles: ParticleSystem::new(),
            blasts: Blasts::new(),
            renderer: Renderer::new(),
            player_tuning: player::Tuning::default(),
            time: time::TimeControl::new(),
            stats: stats::Stats::new(),
            spec,
            // Down only when something explicitly asks for it. The one caller
            // is a headless capture wanting to photograph what is *under* the
            // key list, which is otherwise unreachable without a keypress and
            // so unphotographable on a box with no keyboard.
            show_help: std::env::var("PIXEL_PHYSICS_LAB_HELP").as_deref() != Ok("0"),
        }
    }

    /// Rebuild the box from the same spec, keeping the view and the dial.
    pub fn reset(&mut self) {
        self.world = self.spec.build();
        self.particles = ParticleSystem::new();
        self.blasts = Blasts::new();
        self.stats = stats::Stats::new();
    }

    /// One simulated tick — the shipped sequence, nothing skipped.
    ///
    /// Private on purpose: everything outside goes through `advance`, because
    /// how many of these run per displayed frame is `time`'s decision and
    /// splitting that across two callers is how a speed dial stops being
    /// honest about what it achieved.
    fn tick(&mut self) {
        frame::step(
            &mut self.world,
            &mut self.particles,
            &mut self.blasts,
            player::PlayerInput::default(),
            &self.player_tuning,
        );
    }

    /// Run whatever this displayed frame's share of simulated time is, and
    /// report what actually happened.
    ///
    /// `elapsed` is real time since the last call. Everything about how that
    /// becomes a tick count — the requested multiplier, the wall-clock
    /// ceiling that keeps the window answering, whether the phase is Tending
    /// or Running — is `time::TimeControl`'s, so that the readout on screen
    /// and the loop that produced it cannot disagree.
    pub fn advance(&mut self, elapsed: std::time::Duration) -> time::Advance {
        let plan = self.time.plan(elapsed);
        let started = std::time::Instant::now();
        let mut ran = 0u32;
        while ran < plan.ticks {
            self.tick();
            ran += 1;
            if started.elapsed() >= plan.budget {
                break;
            }
        }
        let advance = self.time.record(ran, started.elapsed());
        self.stats.observe(&self.world);
        advance
    }

    /// Draw the world, then whatever the lab is showing over it.
    pub fn draw(&mut self, frame_buf: &mut [u8], cursor: Option<(i32, i32)>) {
        // Anything drawn over the terrain has no footprint tracked between
        // frames, so the dirty-rect skip cannot know to erase last frame's.
        // Same rule, and the same reasoning, as `App::draw`'s.
        let force_full =
            cursor.is_some() || self.stats.showing() || self.time.hud_is_dirty() || self.show_help;
        let touched = self.world.take_touched_chunks();
        self.renderer.draw(
            &self.world,
            &self.particles,
            &touched,
            frame_buf,
            (WIDTH, HEIGHT),
            force_full,
        );
        self.time.draw(frame_buf, &self.world);
        // **One page at a time.** The key list and the biosphere page are both
        // full-height overlays, and drawn together they interleave into
        // something neither of them is -- caught by looking at a capture of
        // the real window, not by any test, since both draw exactly what they
        // were asked to. The key list wins because it is transient: it is up
        // on a fresh lab and gone on the next keypress, where the page is
        // where you live.
        //
        // `draw_at` rather than `draw`: the hover explanation is the half of
        // the page that makes it readable cold, and `Lab::draw` is the only
        // place holding the cursor.
        if self.show_help {
            draw_help(frame_buf);
        } else {
            self.stats.draw_at(frame_buf, &self.world, cursor);
        }
    }
}

/// The key list, drawn over a dimmed screen.
///
/// Uppercase and punctuation-light on purpose: `hud`'s font is a hand-authored
/// 5x7 bitmap with a deliberately small glyph set, and a character it does not
/// have draws as a silent blank rather than as anything you would notice. That
/// gap has shipped three times in this repo, so every line here is checked
/// against `hud::has_glyph` by `every_help_line_is_drawable`.
const HELP: [&str; 10] = [
    "THE EVOLUTION LAB",
    "",
    "SPACE    TENDING / RUNNING",
    "UP DOWN  SPEED",
    "1-6      SPEED PRESET",
    "TAB      STATS",
    "F        DISPLAY RATE",
    "WASD     PAN     - =  ZOOM",
    "R        REBUILD THE BOX",
    "?        THIS PAGE",
];

fn draw_help(frame: &mut [u8]) {
    let w = HELP.iter().map(|l| crate::hud::text_width(l)).max().unwrap_or(0);
    let (bw, bh) = (w + 24, HELP.len() as i32 * 10 + 20);
    let (x0, y0) = ((WIDTH as i32 - bw) / 2, (HEIGHT as i32 - bh) / 2);
    for y in y0..(y0 + bh) {
        for x in x0..(x0 + bw) {
            if x < 0 || y < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
                continue;
            }
            let i = ((y as u32 * WIDTH + x as u32) * 4) as usize;
            // Dim what is behind rather than covering it, so the box stays
            // visible under the page and the page reads as an overlay.
            for c in 0..3 {
                frame[i + c] /= 4;
            }
        }
    }
    for (i, line) in HELP.iter().enumerate() {
        let colour = if i == 0 { [235, 235, 200, 255] } else { [200, 200, 200, 255] };
        crate::hud::draw_text(frame, WIDTH, HEIGHT, x0 + 12, y0 + 12 + i as i32 * 10, line, colour);
    }
}

#[cfg(test)]
mod tests {
    /// **The font cannot draw everything, and what it cannot draw it draws as
    /// nothing.** `[`/`]`, then `_`/`<`/`>`, then `;`/`'` have each shipped
    /// blank in this engine's UI for as long as they were bound. This is the
    /// cheap standing check, not a discipline.
    #[test]
    fn every_help_line_is_drawable() {
        for line in super::HELP {
            for c in line.chars() {
                assert!(crate::hud::has_glyph(c), "the HUD font has no glyph for {c:?} in {line:?}");
            }
        }
    }
}
