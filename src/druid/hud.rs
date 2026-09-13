//! **The druid game's interface** — the readout, the key legend, and the
//! rings that say where time is running.
//!
//! Written because the game shipped without one. The owner's first playtest,
//! 2026-09-13: *"I just tried to playtest and there is no GUI at all? I have
//! no idea how to control anything other than run and jump."* Every key
//! existed — in a doc comment at the top of `src/bin/druid.rs` and in the OS
//! window title, neither of which is on screen. **A verb the player cannot
//! find is a verb the game does not have**, which is `CLAUDE.md`'s second law
//! wearing a different hat: *there must be a verb, and it must deliver
//! something*.
//!
//! Three things, in the order they were missed:
//!
//! 1. **The keys**, on screen and **on by default**. Fourteen rows against
//!    the sandbox's sixty, so they fit in a corner and need no discovery step
//!    at all — the sandbox can afford to hide its help behind `/` because it
//!    has a mouse, a palette and a toolbelt to find things with, and this
//!    game has a gnome and nothing else. `/` hides them once they are known.
//! 2. **The circles.** Under `HeldLook::Unchanged` — the owner's pick and the
//!    default — held ground draws *exactly* as running ground does, so a
//!    placed quickening has no appearance whatsoever: you could place one,
//!    walk away, and never find it again. A thin ring is the interface answer
//!    and it is deliberately plain, because it is **not the rim** — the rim is
//!    arrested matter, hanging rain and a leaf fixed mid-fall, and it is its
//!    own piece of work (`Reports/held-world-game-concept-2026-09-13.md`).
//! 3. **What just happened.** `Druid::found_colony` printed `REFUSED - no
//!    ground` to a terminal nobody is looking at, so a founding that placed
//!    nobody and one that placed twelve were the same event on screen. That
//!    is `CLAUDE.md`'s *"did it fire at all needs a counter"* pointed at the
//!    player rather than at the harness.
//!
//! **The dirty-rect trap, which is why [`Interface`] is a value rather than a
//! pile of draw calls.** Everything here paints over terrain the renderer
//! believes is settled, and the renderer tracks no footprint for it — so when
//! the readout *changes* (a digit, a vanished message, a moved ring) the old
//! pixels stay burned into the ground with no error anywhere. `App::draw`
//! solves this by forcing a full redraw whenever any overlay is up, and the
//! lab by forcing one every frame; neither is right for a game whose whole
//! premise is a world that is standing still, where the render skip is doing
//! its best work exactly when nothing is happening. So the interface is built
//! *before* the world is drawn, compared with last frame's, and only a
//! difference forces the repaint.

use super::Druid;
use crate::hud;
use crate::render::Hud;

/// **Panel fill, and it is opaque on purpose.**
///
/// The sandbox and the lab both draw translucent panels, and both of them
/// force a full redraw every frame an overlay is up — which is what makes a
/// blend safe there. Here the whole design is *not* forcing that redraw, and
/// a blend into a region the renderer skipped compounds: measured on the
/// first headless shot, four frames of `blend(.., 0.72)` over unchanged sky
/// left the panel at **(9, 12, 20)**, i.e. flat opaque, against the
/// **(30, 44, 63)** a single pass gives. Worse than merely being darker than
/// intended, it *oscillates*: a chunk updating under the panel repaints that
/// strip back to one-pass translucency, so the panel would visibly flicker
/// between two looks as the world moved beneath it.
///
/// So the fill is written rather than blended. Idempotent, stable, and it
/// makes the whole question go away — and every other thing this module draws
/// (text, rings) is already a `put` for the same reason.
const PANEL: [u8; 4] = [14, 18, 26, 255];
const EDGE: [u8; 4] = [70, 90, 115, 255];
const TEXT: [u8; 4] = [225, 228, 235, 255];
/// Keys sit a shade under their own descriptions, so the eye runs down the
/// verbs and only crosses to the key it needs — `App::draw_help`'s rule.
const KEYCAP: [u8; 4] = [150, 158, 175, 255];
const DIM: [u8; 4] = [140, 148, 165, 255];
const ACCENT: [u8; 4] = [120, 200, 255, 255];
const WARN: [u8; 4] = [240, 170, 90, 255];
const GOOD: [u8; 4] = [140, 220, 150, 255];

/// The circle the player carries: warm, because it is *him*.
const RING_CARRIED: [u8; 4] = [255, 214, 140, 255];
/// A circle standing on its own in the world, paid for.
const RING_STANDING: [u8; 4] = [150, 220, 255, 255];
/// What the next `SPACE` would place. Faint: it is not there yet.
const RING_PREVIEW: [u8; 4] = [96, 116, 140, 255];

/// One row of text. 7-pixel glyphs and two of air, the same step
/// `App::HELP_LINE` and `lab::stats::LINE` use.
const LINE: i32 = hud::GLYPH_HEIGHT + 2;
const PAD: i32 = 4;
const MARGIN: i32 = 4;
/// Pixels from a legend row's left edge to its description. `SHIFT` and
/// `SPACE` are the widest keys at five glyphs, so 34 clears both with air.
const KEY_COL: i32 = 34;

/// **Every key the game binds, and what it does in the world's own words.**
///
/// `WALK` rather than `MOVE LEFT/RIGHT`, `FOUND A COLONY AT YOUR FEET` rather
/// than `found_colony_of` — `CLAUDE.md`'s rule for writing to the owner
/// applies on screen too, and doubly, because this list is the only place the
/// game says what it *is*.
///
/// `the_legend_names_every_key_the_binary_binds` reads
/// `src/bin/druid.rs` and fails if a key is bound and not listed here, which
/// is exactly the failure that produced this module.
pub const KEYS: &[(&str, &str)] = &[
    ("A D", "WALK"),
    ("W", "JUMP / SWIM UP"),
    ("S", "DUCK / SWIM DOWN"),
    ("SHIFT", "HOLD ON IN A TREE"),
    ("SPACE", "PLACE A CIRCLE OF TIME"),
    ("X", "LIFT THE NEAREST CIRCLE"),
    ("Q E", "ITS RADIUS"),
    ("T", "SOW A SEED WHERE YOU STAND"),
    ("TAB", "WHICH SEED"),
    ("C", "FOUND A COLONY AT YOUR FEET"),
    ("H", "HOLD OR RELEASE THE WORLD"),
    ("L", "HOW HELD GROUND IS DRAWN"),
    ("U", "UNLIMITED POWER (PLAYTEST)"),
    ("P", "PAUSE"),
    ("/ F1", "HIDE THESE KEYS"),
    ("ESC", "QUIT"),
];

/// **The state the readout draws, as plain numbers.**
///
/// Split out from [`Interface`] so the text can be tested without building a
/// world — `Druid::new` generates and then grows one, which is a minute of
/// wall clock, and a guard that costs a minute is a guard nobody runs. The
/// font-coverage check below sweeps this over a spread of values instead.
pub struct Readout {
    pub power: f32,
    pub income: f32,
    pub drain: f32,
    pub unlimited: bool,
    pub animals: usize,
    pub animals_awake: usize,
    pub circles: usize,
    pub radius: i32,
    pub held: bool,
    pub paused: bool,
    pub look: &'static str,
    pub seed_kind: String,
    pub sown: usize,
    pub message: Option<String>,
}

impl Readout {
    /// The corner block, one line per tuple.
    ///
    /// **The rates are on it, not just the pool.** `POWER 412` is the same
    /// reading whether it is climbing or thirty seconds from closing your
    /// wood, and which of those is true is the only question the game asks.
    pub fn lines(&self) -> Vec<(String, [u8; 4])> {
        let mut lines = Vec::new();
        if self.unlimited {
            lines.push(("POWER UNLIMITED".to_string(), ACCENT));
        } else {
            let net = self.income - self.drain;
            lines.push((format!("POWER {:.0}  {net:+.1}/S", self.power), if net < 0.0 { WARN } else { GOOD }));
            lines.push((format!("IN {:.1}/S   OUT {:.1}/S", self.income, self.drain), DIM));
        }
        // **Awake, not just alive.** An animal in held ground is scenery: it
        // pays nothing and does nothing, and it looks identical to a working
        // one at play zoom. The gap between the two numbers is the whole
        // reason to walk somewhere.
        lines.push((format!("ANIMALS {}   AWAKE {}", self.animals, self.animals_awake), TEXT));
        lines.push((format!("CIRCLES {}   NEXT R{}", self.circles, self.radius), TEXT));
        // **What T would sow, and how many have gone in.** A seed dropped on
        // held ground is invisible until time reaches it, so without the
        // count a working key and a broken one look the same.
        lines.push((format!("SEED {}   SOWN {}", self.seed_kind.to_uppercase(), self.sown), TEXT));
        lines.push((
            format!("WORLD {}   LOOK {}", if self.held { "HELD" } else { "RUNNING" }, self.look.to_uppercase()),
            if self.held { TEXT } else { WARN },
        ));
        if self.paused {
            lines.push(("PAUSED".to_string(), WARN));
        }
        if let Some(message) = &self.message {
            lines.push((message.to_uppercase(), ACCENT));
        }
        lines
    }
}

/// A circle of running time, in screen pixels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Ring {
    cx: i32,
    cy: i32,
    r: i32,
    colour: [u8; 4],
}

/// **One frame of interface, as a value that can be compared with the last
/// one.** See the module doc for why that comparison exists.
#[derive(Clone, PartialEq, Debug)]
pub struct Interface {
    status: Vec<(String, [u8; 4])>,
    keys: bool,
    rings: Vec<Ring>,
}

impl Interface {
    pub fn build(game: &Druid, viewport: (u32, u32)) -> Self {
        Interface { status: game.readout().lines(), keys: game.show_keys, rings: rings(game, viewport) }
    }

    pub fn draw(&self, frame: &mut [u8], viewport: (u32, u32)) {
        // **Scale 1, stated rather than asked for.** `render::Hud` exists
        // because the sandbox grows its framebuffer past the logical
        // 512x320 at zoom-out; this game never does — it hands
        // `Renderer::draw` exactly `(WIDTH, HEIGHT)` — so a logical pixel and
        // a buffer pixel are the same thing here. Reading
        // `Renderer::pixel_scale` instead would size the HUD against a budget
        // the buffer is not honouring.
        let hc = Hud::new(viewport.0, viewport.1, 1);

        // Rings under the panels, so text is never crossed by one.
        for ring in &self.rings {
            hc.circle(frame, ring.cx, ring.cy, ring.r, ring.colour);
        }

        let status_w = self.status.iter().map(|(t, _)| hud::text_width(t)).max().unwrap_or(0) + PAD * 2;
        let status_h = self.status.len() as i32 * LINE + PAD * 2 - 2;
        panel(hc, frame, viewport, (MARGIN, MARGIN, status_w, status_h));
        for (i, (text, colour)) in self.status.iter().enumerate() {
            hc.text(frame, MARGIN + PAD, MARGIN + PAD + i as i32 * LINE, text, *colour);
        }

        if !self.keys {
            return;
        }
        let (keys_w, keys_h) = legend_size();
        let top = viewport.1 as i32 - MARGIN - keys_h;
        panel(hc, frame, viewport, (MARGIN, top, keys_w, keys_h));
        for (i, (key, what)) in KEYS.iter().enumerate() {
            let y = top + PAD + i as i32 * LINE;
            hc.text(frame, MARGIN + PAD, y, key, KEYCAP);
            hc.text(frame, MARGIN + PAD + KEY_COL, y, what, TEXT);
        }
    }
}

/// The legend panel's own size, derived from its rows rather than written
/// down — a hardcoded box is what lets a row run off the bottom of it, which
/// `App::help_columns`'s doc records happening to the line that said which
/// key closed the page.
fn legend_size() -> (i32, i32) {
    let widest = KEYS.iter().map(|(_, what)| hud::text_width(what)).max().unwrap_or(0);
    (PAD * 2 + KEY_COL + widest, PAD * 2 + KEYS.len() as i32 * LINE - 2)
}

fn panel(hc: Hud, frame: &mut [u8], viewport: (u32, u32), (left, top, w, h): (i32, i32, i32, i32)) {
    let (right, bottom) = ((left + w).min(viewport.0 as i32), (top + h).min(viewport.1 as i32));
    for y in top..bottom {
        for x in left..right {
            hc.put(frame, x, y, PANEL);
        }
    }
    for x in left..right {
        hc.put(frame, x, top, EDGE);
        hc.put(frame, x, bottom - 1, EDGE);
    }
    for y in top..bottom {
        hc.put(frame, left, y, EDGE);
        hc.put(frame, right - 1, y, EDGE);
    }
}

/// **Where time is running, in screen pixels.**
///
/// The radius comes from converting a point on the rim rather than from
/// scaling the world radius by a zoom factor this module would have to know
/// about: `Renderer::world_to_screen` already owns that arithmetic in three
/// branches (zoom, stride, 1:1), and a second copy of it here is the side
/// table that goes stale.
fn rings(game: &Druid, _viewport: (u32, u32)) -> Vec<Ring> {
    let renderer = &game.renderer;
    let on_screen = |x: i32, y: i32, r: i32, colour: [u8; 4]| -> Option<Ring> {
        let (cx, cy) = renderer.world_to_screen(x, y)?;
        let (ex, _) = renderer.world_to_screen(x + r, y)?;
        Some(Ring { cx, cy, r: ex - cx, colour })
    };
    let mut rings = Vec::new();
    for q in &game.world.quickenings {
        rings.extend(on_screen(q.x, q.y, q.r, RING_STANDING));
    }
    if let Some(q) = game.world.carried {
        rings.extend(on_screen(q.x, q.y, q.r, RING_CARRIED));
    }
    // What `SPACE` would place, at his feet. Drawn whatever the world is
    // doing: a circle placed in a *running* world is inert, and seeing the
    // ring next to a readout that says `WORLD RUNNING` is how that reads as a
    // state rather than as a broken key.
    if let Some(player) = &game.world.player {
        let (px, py) = player.center();
        rings.extend(on_screen(px, py, game.place_radius, RING_PREVIEW));
    }
    rings
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Every key the binary binds is named on screen.**
    ///
    /// This is the guard for the bug that produced the module: `H`, `L`, `C`,
    /// `SPACE`, `X`, `Q`, `E` and `U` were all bound, all working, and all
    /// invisible — documented only in a source comment and the window title.
    /// Reading the binary rather than a hand-kept list is the point; a list
    /// maintained beside the match statement is the copy that goes stale.
    #[test]
    fn the_legend_names_every_key_the_binary_binds() {
        let source = include_str!("../bin/druid.rs");
        let listed: Vec<&str> = KEYS.iter().flat_map(|(keys, _)| keys.split_whitespace()).collect();

        let mut bound: Vec<String> = Vec::new();
        for (i, _) in source.match_indices("KeyCode::") {
            let name: String = source[i + "KeyCode::".len()..].chars().take_while(|c| c.is_alphanumeric()).collect();
            let shown = match name.as_str() {
                "Escape" => "ESC".to_string(),
                "ShiftLeft" | "ShiftRight" => "SHIFT".to_string(),
                "Space" => "SPACE".to_string(),
                "Slash" => "/".to_string(),
                // `KeyA` .. `KeyZ` and the function keys read straight
                // across; anything else added later shows up as itself and
                // fails loudly rather than being silently skipped.
                // Upper-cased, because every legend entry is: the font is
                // uppercase-only and `hud::draw_text` upper-cases anyway.
                // Without this, binding `KeyCode::Tab` against a legend row
                // reading `TAB` failed here -- which is the guard working,
                // but on a spelling rather than on a missing row.
                other => other.strip_prefix("Key").unwrap_or(other).to_uppercase(),
            };
            if !bound.contains(&shown) {
                bound.push(shown);
            }
        }
        assert!(bound.len() >= 12, "the parse found only {bound:?} — `src/bin/druid.rs` moved and this guard is now blind");
        for key in &bound {
            assert!(listed.contains(&key.as_str()), "`{key}` is bound in src/bin/druid.rs but the on-screen legend does not name it: {listed:?}");
        }
    }

    /// The font has no lowercase and draws an unknown character as a blank
    /// gap rather than complaining — `hud.rs`'s own comments record that
    /// omission shipping **four** separate times, every one of them found by
    /// looking at the rendered page. This is the cheap version of looking.
    #[test]
    fn every_character_the_interface_draws_has_a_glyph() {
        for (key, what) in KEYS {
            for text in [key, what] {
                for c in text.chars() {
                    assert!(hud::has_glyph(c), "the legend draws {c:?} in {text:?}, which the font renders as a blank gap");
                }
            }
        }
        // The readout is formatted, so sweep values that produce every branch
        // and every sign rather than one tidy case.
        for (power, income, drain, unlimited, paused) in [
            (0.0, 0.0, 0.0, false, false),
            (612.4, 7.25, 1.5, false, true),
            (0.0, 0.0, 99.9, false, false),
            (600.0, 0.0, 0.0, true, false),
        ] {
            let readout = Readout {
                power,
                income,
                drain,
                unlimited,
                animals: 12,
                animals_awake: 7,
                circles: 3,
                radius: 60,
                held: true,
                paused,
                look: "one hue",
                seed_kind: "conifer".to_string(),
                sown: 3,
                message: Some("no ground here - nothing founded".to_string()),
            };
            for (text, _) in readout.lines() {
                for c in text.chars() {
                    assert!(hud::has_glyph(c), "the readout draws {c:?} in {text:?}, which the font renders as a blank gap");
                }
            }
        }
    }

    /// Both panels fit the screen they are drawn on. The failure this guards
    /// is `App::help_columns`'s: a page that ran three lines past its own
    /// panel, so the last thing it drew off-screen was the row saying which
    /// key closed it.
    #[test]
    fn both_panels_fit_inside_the_viewport() {
        let (w, h) = (crate::app::WIDTH as i32, crate::app::HEIGHT as i32);
        let (keys_w, keys_h) = legend_size();
        assert!(MARGIN + keys_w <= w, "the key legend is {keys_w} wide in a {w}-wide window");
        assert!(MARGIN * 2 + keys_h <= h, "the key legend is {keys_h} tall in a {h}-tall window");

        // The readout sits at the top and the legend at the bottom; they must
        // not meet in the middle at the readout's tallest (message showing,
        // paused, and the economy visible).
        let readout = Readout {
            power: 1234.5,
            income: 12.5,
            drain: 30.0,
            unlimited: false,
            animals: 4321,
            animals_awake: 4321,
            circles: 12,
            radius: 240,
            held: true,
            paused: true,
            look: "unchanged",
            seed_kind: "scrambler".to_string(),
            sown: 999,
            message: Some("out of power - a standing circle closed".to_string()),
        };
        let lines = readout.lines();
        let status_h = lines.len() as i32 * LINE + PAD * 2 - 2;
        let status_w = lines.iter().map(|(t, _)| hud::text_width(t)).max().unwrap_or(0) + PAD * 2;
        assert!(MARGIN + status_w <= w, "the readout is {status_w} wide in a {w}-wide window");
        assert!(MARGIN + status_h < h - MARGIN - keys_h, "the readout ({status_h}) and the key legend ({keys_h}) overlap in a {h}-tall window");
    }

    /// **Drawing the interface twice must change nothing the second time.**
    ///
    /// The guard for the defect the first headless shot caught: the panel
    /// fill was a `blend`, and a blend into a region the renderer skipped
    /// compounds frame on frame until it is flat opaque — see [`PANEL`]. The
    /// property that makes this module safe to draw over a world that is not
    /// being repainted is *idempotence*, so that is what is asserted, rather
    /// than any particular colour: swap `put` back for `blend` and this goes
    /// red on the first pixel of the fill.
    #[test]
    fn drawing_the_same_interface_twice_changes_nothing() {
        let (w, h) = (crate::app::WIDTH, crate::app::HEIGHT);
        // A bright ground, so a compounding blend has somewhere to darken
        // from — over black it would be idempotent by accident.
        let mut frame = vec![0u8; (w * h * 4) as usize];
        for px in frame.chunks_exact_mut(4) {
            px.copy_from_slice(&[120, 170, 230, 255]);
        }
        let ui = Interface {
            status: vec![("POWER 600  +0.0/S".to_string(), TEXT), ("WORLD HELD".to_string(), TEXT)],
            keys: true,
            rings: vec![Ring { cx: 200, cy: 150, r: 28, colour: RING_CARRIED }],
        };
        ui.draw(&mut frame, (w, h));
        let once = frame.clone();
        ui.draw(&mut frame, (w, h));
        let differing = frame.chunks_exact(4).zip(once.chunks_exact(4)).filter(|(a, b)| a != b).count();
        assert_eq!(differing, 0, "{differing} pixels changed on the second identical draw — the interface is not idempotent, so it compounds over a skipped frame");
    }

    /// **Put the fault back**: an interface that has changed must compare
    /// unequal, because that comparison is the only thing standing between
    /// the readout and a smear burned into settled ground.
    #[test]
    fn a_changed_readout_compares_unequal() {
        let mut a = Interface { status: vec![("POWER 600".to_string(), TEXT)], keys: true, rings: Vec::new() };
        let b = a.clone();
        assert_eq!(a, b, "an unchanged interface must compare equal, or the render skip never fires at all");

        a.status = vec![("POWER 599".to_string(), TEXT)];
        assert_ne!(a, b, "a changed digit must force the repaint");

        let mut c = b.clone();
        c.keys = false;
        assert_ne!(c, b, "hiding the legend must force the repaint, or it stays on screen after the key");

        let mut d = b.clone();
        d.rings = vec![Ring { cx: 10, cy: 10, r: 28, colour: RING_CARRIED }];
        assert_ne!(d, b, "a ring appearing must force the repaint");
        let mut e = d.clone();
        e.rings[0].cx += 1;
        assert_ne!(e, d, "a ring that moved one pixel must force the repaint -- this is the smear");
    }
}
