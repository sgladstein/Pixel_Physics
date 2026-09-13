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

/// **An animal with something worth taking.** Warm gold, the same family as
/// the carried circle, because both are *yours*: the mark says "this is
/// energy that belongs to you and is waiting".
const CHARGED: [u8; 4] = [255, 226, 120, 255];
/// ...and the same at a low charge, so the mark reads as filling rather than
/// switching on. Interpolated toward [`CHARGED`] by how full the animal is.
const CHARGED_LOW: [u8; 4] = [150, 120, 60, 255];
/// Energy in flight. Brighter than the charged mark, because it is the event
/// and the mark is only the promise.
const FLOW: [u8; 4] = [255, 250, 225, 255];
/// The halo around a mote, and what the landing ring fades to.
const FLOW_FAINT: [u8; 4] = [190, 150, 70, 255];

/// One row of text. 7-pixel glyphs and two of air, the same step
/// `App::HELP_LINE` and `lab::stats::LINE` use.
const LINE: i32 = hud::GLYPH_HEIGHT + 2;
const PAD: i32 = 4;
const MARGIN: i32 = 4;
/// Pixels from a legend row's left edge to its description. `SHIFT` and
/// `SPACE` are the widest keys at five glyphs, so 34 clears both with air.
const KEY_COL: i32 = 34;
/// The power bar's box, in logical pixels. As wide as the panel's widest row
/// so it reads as the panel's own gauge rather than as a widget in it.
const BAR_W: i32 = 150;
const BAR_H: i32 = 5;

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
    ("Z V", "HOW FAST TIME RUNS IN THEM"),
    ("T", "SOW A SEED WHERE YOU STAND"),
    ("TAB", "WHICH SEED"),
    ("C", "FOUND A COLONY AT YOUR FEET"),
    ("F", "DRAW THE CHARGE OUT OF THEM"),
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
    pub rate: u32,
    pub held: bool,
    pub paused: bool,
    pub look: &'static str,
    pub seed_kind: String,
    pub sown: usize,
    /// Charge standing within reach, and how many animals hold it.
    pub charge: (f32, usize),
    pub reserve_cap: f32,
    /// What a full pool looks like, for the bar. Power can exceed it; the bar
    /// clamps rather than rescaling, because a gauge whose full mark moves is
    /// not a gauge.
    pub power_full: f32,
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
        // The bar is drawn, not written -- this row reserves its line so the
        // panel sizes itself around it, and `draw` paints over the blanks.
        lines.push((String::new(), TEXT));
        // **Awake, not just alive.** An animal in held ground is scenery: it
        // pays nothing and does nothing, and it looks identical to a working
        // one at play zoom. The gap between the two numbers is the whole
        // reason to walk somewhere.
        lines.push((format!("ANIMALS {}   AWAKE {}", self.animals, self.animals_awake), TEXT));
        lines.push((format!("CIRCLES {}   NEXT R{}   SPEED X{}", self.circles, self.radius, self.rate), TEXT));
        // **What T would sow, and how many have gone in.** A seed dropped on
        // held ground is invisible until time reaches it, so without the
        // count a working key and a broken one look the same.
        lines.push((format!("SEED {}   SOWN {}", self.seed_kind.to_uppercase(), self.sown), TEXT));
        // **What `F` would give you, and from how many.** The number is the
        // whole decision: walk to the colony now, or leave it charging.
        let (charge, holders) = self.charge;
        lines.push((
            format!("CHARGE {charge:.0} IN {holders} NEAR YOU"),
            if charge >= self.reserve_cap { GOOD } else { DIM },
        ));
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

/// A charged animal, in screen pixels, with how full it is.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Mark {
    x: i32,
    y: i32,
    fullness: f32,
}

/// One mote of drawn energy, in screen pixels, on its way to the player.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Mote {
    x: i32,
    y: i32,
    /// 0..1, how hot this particular mote burns. Varies per mote and rises as
    /// it nears the player, so the stream shimmers instead of marching.
    bright: f32,
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
    /// 0..1 of the pool, or `None` while power is unlimited.
    power_bar: Option<f32>,
    keys: bool,
    rings: Vec<Ring>,
    marks: Vec<Mark>,
    motes: Vec<Mote>,
    /// Screen position and 0..1 age of each arrival bloom.
    landings: Vec<(i32, i32, f32)>,
}

impl Interface {
    pub fn build(game: &Druid, viewport: (u32, u32)) -> Self {
        let readout = game.readout();
        Interface {
            power_bar: (!readout.unlimited).then(|| (readout.power / readout.power_full).clamp(0.0, 1.0)),
            status: readout.lines(),
            keys: game.show_keys,
            rings: rings(game, viewport),
            marks: marks(game),
            motes: motes(game),
            landings: landings(game),
        }
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

        // **The promise: a mark over an animal holding charge.** A chevron
        // rather than a dot, and it grows with how full the animal is, so a
        // colony reads as *filling* across a glance instead of switching on
        // one ant at a time. Drawn over the head, not on the body: an ant is
        // two cells and a dot on it is an ant of a different colour.
        for m in &self.marks {
            let tint = lerp(CHARGED_LOW, CHARGED, m.fullness);
            let arms = 2 + (m.fullness * 4.0) as i32;
            for i in 0..=arms {
                for w in 0..2 {
                    hc.put(frame, m.x - i, m.y - 5 - i + w, tint);
                    hc.put(frame, m.x + i, m.y - 5 - i + w, tint);
                }
            }
        }

        // **The event: energy on its way in.** Brighter and larger as it
        // arrives, so the flow reads as gathering rather than as a line of
        // dots — the last few pixels before it lands are the loudest thing on
        // screen, which is where the satisfaction has to be.
        // **Many small particles, not a few orbs.** Owner, on the first
        // attempt: *"It should look like individual particles of energy
        // flowing. Not a couple big orbs."* So each mote is one or two
        // pixels and there are dozens of them, each on its own phase with its
        // own lateral wander — the flow is in the *count* and the spread, not
        // in the size of any one of them.
        for m in &self.motes {
            hc.put(frame, m.x, m.y, lerp(FLOW_FAINT, FLOW, m.bright));
            // A second pixel only on the brightest, so the stream has grain
            // rather than being uniform dust.
            if m.bright > 0.78 {
                hc.put(frame, m.x, m.y - 1, FLOW);
            }
        }

        // **It lands on you.** A ring that blooms outward from the player as
        // the head of each stream arrives, so the energy visibly *enters*
        // rather than merely stopping. Without it the motes vanish at his
        // feet and the moment has no punctuation.
        for (cx, cy, t) in &self.landings {
            let r = (2.0 + t * 14.0).round() as i32;
            hc.circle(frame, *cx, *cy, r, lerp(FLOW, FLOW_FAINT, *t));
            if *t < 0.4 {
                hc.circle(frame, *cx, *cy, r / 2, FLOW);
            }
        }

        let status_w = self.status.iter().map(|(t, _)| hud::text_width(t)).max().unwrap_or(0).max(BAR_W) + PAD * 2;
        let status_h = self.status.len() as i32 * LINE + PAD * 2 - 2;
        panel(hc, frame, viewport, (MARGIN, MARGIN, status_w, status_h));
        for (i, (text, colour)) in self.status.iter().enumerate() {
            let y = MARGIN + PAD + i as i32 * LINE;
            if text.is_empty() {
                // **The pool as a bar you watch move.** Owner: *"the power
                // should be a bar that you can see drain and fill."* A number
                // tells you where you are; a bar tells you which way you are
                // going without reading anything, which is what you want
                // while you are looking at the world instead of the corner.
                if let Some(fill) = self.power_bar {
                    let lit = (BAR_W as f32 * fill).round() as i32;
                    for x in 0..BAR_W {
                        let on = x < lit;
                        let c = if on {
                            // Green with room, amber as it runs down.
                            lerp(WARN, GOOD, (fill * 2.0).min(1.0))
                        } else {
                            EDGE
                        };
                        for dy in 0..BAR_H {
                            hc.put(frame, MARGIN + PAD + x, y + dy, c);
                        }
                    }
                }
                continue;
            }
            hc.text(frame, MARGIN + PAD, y, text, *colour);
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

fn lerp(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let t = t.clamp(0.0, 1.0);
    let mix = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round().clamp(0.0, 255.0) as u8;
    [mix(0), mix(1), mix(2), 255]
}

/// **Which animals are worth walking to**, in screen pixels.
fn marks(game: &Druid) -> Vec<Mark> {
    game.charged_animals()
        .into_iter()
        .filter_map(|((x, y), fullness)| {
            let (sx, sy) = game.renderer.world_to_screen(x, y)?;
            Some(Mark { x: sx, y: sy, fullness })
        })
        .collect()
}

/// **The flow: dozens of small particles streaming from each animal to the
/// player.**
///
/// Owner: *"It should look like individual particles of energy flowing. Not a
/// couple big orbs."* So the stream's weight comes from **count and spread**
/// rather than from the size of any one mote — [`PER_DRAW`] of them, each on
/// its own phase, each wandering sideways off the line by its own amount.
///
/// Three things make it read as energy rather than as a dotted line, and all
/// three are cheap:
///
/// - **Staggered phases**, so particles are strung along the whole journey at
///   once instead of arriving as a pulse.
/// - **A lateral wander** perpendicular to the path, unique per mote and
///   *narrowing* as it nears the player — the stream converges on him, which
///   is what makes it look drawn in rather than merely travelling.
/// - **A bow** on the path, because a straight line between two points on
///   level ground reads as a wire.
///
/// Everything is derived from the mote's index, so it is deterministic and
/// stable frame to frame — a particle wanders along a fixed curve rather than
/// jittering, which is the difference between a flow and static.
fn motes(game: &Druid) -> Vec<Mote> {
    /// Enough that the stream has body at any distance. One animal's draw is
    /// a thread; a colony's is a river, which is the point.
    const PER_DRAW: i32 = 26;
    let Some(player) = &game.world.player else {
        return Vec::new();
    };
    let (px, py) = player.center();
    let mut out = Vec::new();
    for d in &game.draws {
        let head = d.age as f32 / super::DRAW_FRAMES as f32;
        let (dx, dy) = ((px - d.from.0) as f32, (py - d.from.1) as f32);
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        // Perpendicular to the path, for the wander.
        let (nx, ny) = (-dy / len, dx / len);
        for i in 0..PER_DRAW {
            let seed = i as f32;
            // Spread over the journey, and let the leaders run ahead of the
            // head so the stream has a ragged front rather than a wall.
            let t = head * 1.3 - (seed / PER_DRAW as f32) * 0.62 - (seed * 0.37).fract() * 0.06;
            if !(0.0..=1.0).contains(&t) {
                continue;
            }
            let ease = t * t * (3.0 - 2.0 * t);
            // Narrows to nothing at the player: the stream converges on him.
            let spread = (1.0 - ease) * 7.0;
            let wander = ((t * 7.0 + seed * 2.399).sin() + (seed * 1.7).sin()) * 0.5 * spread;
            let arc = (t * std::f32::consts::PI).sin() * 11.0;
            let wx = d.from.0 as f32 + dx * ease + nx * wander;
            let wy = d.from.1 as f32 + dy * ease + ny * wander - arc;
            if let Some((sx, sy)) = game.renderer.world_to_screen(wx.round() as i32, wy.round() as i32) {
                // Brightest as it lands, with a per-mote offset so the stream
                // shimmers rather than fading uniformly.
                let bright = (0.35 + 0.65 * ease + (seed * 3.1).sin() * 0.18).clamp(0.0, 1.0);
                out.push(Mote { x: sx, y: sy, bright });
            }
        }
    }
    out
}

/// **Where energy is arriving, and how far through its bloom.**
///
/// One per draw whose head has reached the player, over the last stretch of
/// its life — so several animals drained together give several overlapping
/// blooms rather than one, which is what makes a big pull feel bigger.
fn landings(game: &Druid) -> Vec<(i32, i32, f32)> {
    const BLOOM: f32 = 0.28;
    let Some(player) = &game.world.player else {
        return Vec::new();
    };
    let (px, py) = player.center();
    let Some((sx, sy)) = game.renderer.world_to_screen(px, py) else {
        return Vec::new();
    };
    game.draws
        .iter()
        .filter_map(|d| {
            let head = d.age as f32 / super::DRAW_FRAMES as f32;
            (head >= 1.0 - BLOOM).then(|| (sx, sy, ((head - (1.0 - BLOOM)) / BLOOM).clamp(0.0, 1.0)))
        })
        .collect()
}

/// **How many motes the flow is drawing right now**, for a headless run that
/// has to tell "too faint to see" from "not there at all".
///
/// A pixel test cannot answer it: the first attempt filtered for warm bright
/// pixels and counted the **dead-root texture**, whose bounding box was the
/// whole frame. Asking the thing that draws them is the only honest reading.
pub fn mote_count(game: &Druid) -> usize {
    motes(game).len()
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
/// **What a circle looks like at speed.**
///
/// Owner: *"there should be different visuals for bubbles of faster speeds."*
/// The ring runs from its cool standing blue toward a hot white as the dial
/// climbs, and above real time it gains a second ring inside it — so speed
/// reads two ways at once, by colour at a glance and by *count* when you are
/// looking straight at it. Colour alone is the single-channel readout this
/// repo keeps learning not to rely on.
fn speed_tint(base: [u8; 4], speed: u32) -> [u8; 4] {
    let t = ((speed.max(1) - 1) as f32 / (super::SPEED_MAX - 1).max(1) as f32).clamp(0.0, 1.0);
    lerp(base, FLOW, t)
}

fn rings(game: &Druid, _viewport: (u32, u32)) -> Vec<Ring> {
    let renderer = &game.renderer;
    let on_screen = |x: i32, y: i32, r: i32, colour: [u8; 4]| -> Option<Ring> {
        let (cx, cy) = renderer.world_to_screen(x, y)?;
        let (ex, _) = renderer.world_to_screen(x + r, y)?;
        Some(Ring { cx, cy, r: ex - cx, colour })
    };
    let mut rings = Vec::new();
    let tint = speed_tint(RING_STANDING, game.speed);
    for q in &game.world.quickenings {
        rings.extend(on_screen(q.x, q.y, q.r, tint));
        // One extra ring inside per two steps of the dial, so a fast circle
        // is visibly *thicker* and not merely a different colour.
        for i in 1..=(game.speed.saturating_sub(1) / 2) as i32 {
            rings.extend(on_screen(q.x, q.y, q.r - i * 3, tint));
        }
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
                rate: 4,
                held: true,
                paused,
                look: "one hue",
                seed_kind: "conifer".to_string(),
                sown: 3,
                charge: (123.0, 7),
                reserve_cap: 40.0,
                power_full: 600.0,
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
            rate: 8,
            held: true,
            paused: true,
            look: "unchanged",
            seed_kind: "scrambler".to_string(),
            sown: 999,
            charge: (4321.0, 210),
            reserve_cap: 40.0,
            power_full: 600.0,
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
            power_bar: Some(0.62),
            status: vec![("POWER 600  +0.0/S".to_string(), TEXT), (String::new(), TEXT), ("WORLD HELD".to_string(), TEXT)],
            keys: true,
            rings: vec![Ring { cx: 200, cy: 150, r: 28, colour: RING_CARRIED }],
            marks: vec![Mark { x: 120, y: 140, fullness: 0.8 }],
            motes: vec![Mote { x: 160, y: 130, bright: 0.5 }],
            landings: vec![(200, 150, 0.3)],
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
        let mut a = Interface {
            power_bar: Some(0.5),
            status: vec![("POWER 600".to_string(), TEXT)],
            keys: true,
            rings: Vec::new(),
            marks: Vec::new(),
            motes: Vec::new(),
            landings: Vec::new(),
        };
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

        // The two that move every frame while they exist, and would smear
        // worst: a mote is drawn over settled ground and travels.
        let mut f = b.clone();
        f.motes = vec![Mote { x: 10, y: 10, bright: 0.1 }];
        assert_ne!(f, b, "energy in flight must force the repaint");
        let mut g = f.clone();
        g.motes[0].bright += 0.05;
        assert_ne!(g, f, "a mote that only brightened must still force the repaint");
        let mut h = b.clone();
        h.marks = vec![Mark { x: 10, y: 10, fullness: 0.5 }];
        assert_ne!(h, b, "a charged animal appearing must force the repaint");
    }
}
