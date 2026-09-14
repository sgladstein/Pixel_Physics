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

/// Energy in flight.
///
/// **Once "brighter than the charged mark, because it is the event and the
/// mark is only the promise".** Item 2 of the 2026-09-14 playtest removed
/// the mark — *"no indication of how much life energy the ants have...
/// you should just see how much energy you get by how many particles
/// come"* — so the event, this colour, is now the whole reading rather than
/// half of a pair. See [`mote_count_for_amount`].
const FLOW: [u8; 4] = [255, 250, 225, 255];
/// The halo around a mote, and what the landing ring fades to.
/// The scent the druid lays — a cold green, so it reads as something put
/// down rather than as the warm energy of the drain.
///
/// **Channel A only.** The two planes get different colours because they are
/// different instructions and one of them is currently inert: a player who
/// cannot see which plane he is drawing on cannot see why nothing happened.
const SCENT: [u8; 4] = [120, 255, 180, 255];
/// The food route, in the magenta `render.rs`'s own pheromone overlay uses for
/// it — and deliberately a *colder* pair than channel A's, because nothing can
/// read it yet (`open-bugs-handoff.md` §Z7).
const SCENT_B: [u8; 4] = [230, 150, 255, 255];
const SCENT_B_FAINT: [u8; 4] = [120, 70, 140, 255];
/// **How far off the walked route the field is sampled**, in cells.
///
/// Four, because that is roughly how far `pheromone::DIFFUSE` carries a mark
/// before it rounds to nothing — far enough that the readout shows a cloud
/// rather than a line, and near enough that a long trail is a few thousand
/// samples rather than the whole screen.
const SCENT_HALO: i32 = 4;

/// **How many brightness levels a mark can take.** See `Interface::scent`
/// for why this is banded at all rather than continuous — it is the
/// dirty-rect skip, not the palette.
const SCENT_BANDS: u8 = 8;

/// ...and the faintest a mark gets before it is dropped.
///
/// **Not a fade toward the panel colour**, which is what the first version
/// did and why the first headless shot showed no trail at all: a fresh mark
/// is `DEPOSIT` = 40 of 255, so it blended 37% of the way from navy to green
/// and came out a dark muted green that is indistinguishable from the
/// dead-root texture it lies on. A mark is either there or it is not; how
/// strong it is varies the *green*, never how much ground shows through.
const SCENT_FAINT: [u8; 4] = [46, 150, 96, 255];

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
///
/// **The third field is whether the button bar also carries this key** — see
/// `bar_specs`. A key with a button is still listed here (this array is the
/// canonical "every key does something discoverable" guard and stays
/// complete regardless of what else reaches it), but [`legend_size`] and
/// [`Interface::draw`]'s legend loop skip it: the bar's own caption is that
/// key's on-screen home now, and repeating it in the corner would be the
/// same fact drawn twice while doing nothing for the keys that still need a
/// legend row. That is also the owner's own complaint about item 3, read
/// straight: *"I don't want to have to remember all these shortcuts and the
/// menu isn't even a menu, it is a shortcut list"* — a shorter list, for
/// what has no button.
pub const KEYS: &[(&str, &str, bool)] = &[
    ("A D", "WALK", false),
    ("W", "JUMP / SWIM UP", false),
    ("S", "DUCK / SWIM DOWN", false),
    ("SHIFT", "HOLD ON IN A TREE", false),
    ("SPACE", "PLACE A CIRCLE OF TIME", true),
    ("X", "LIFT THE NEAREST CIRCLE", true),
    ("Q E", "ITS RADIUS", false),
    ("[ ]", "HOW FAR YOUR OWN CIRCLE REACHES", false),
    ("Z V", "HOW FAST TIME RUNS IN THEM", false),
    ("G", "LAY A SCENT TRAIL AS YOU WALK", false),
    ("I", "WHICH SCENT - HOME, OR FOOD", true),
    // `T` still sows; `K` now cycles which seed. `TAB` moved to the
    // biosphere page below, matching the key the lab already uses for its
    // own — a player who has touched both games gets the same reflex.
    ("K", "WHICH SEED", true),
    ("T", "SOW A SEED WHERE YOU STAND", true),
    ("C", "FOUND A COLONY - OPENS AN OFFER", true),
    ("F", "DRAW THE CHARGE OUT OF THEM", true),
    ("M", "OPTIONS", true),
    ("H", "HOLD OR RELEASE THE WORLD", true),
    ("L", "HOW HELD GROUND IS DRAWN", true),
    ("U", "UNLIMITED POWER (PLAYTEST)", true),
    ("P", "PAUSE", true),
    ("TAB", "PLANTS, ANIMALS, BIRTHS - THE BIOSPHERE PAGE", true),
    ("/ F1", "HIDE THESE KEYS", false),
    ("ESC", "QUIT", false),
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
    /// How wide the circle the player carries is.
    pub carried_radius: i32,
    pub held: bool,
    pub paused: bool,
    pub look: &'static str,
    pub seed_kind: String,
    pub sown: usize,
    /// Charge standing within reach, and how many animals hold it.
    ///
    /// **No longer drawn.** Item 2 of the 2026-09-14 playtest: *"no
    /// indication of how much life energy the ants have."* This was the
    /// second of the two indications the owner's words covered — the first
    /// being the chevron over a charged animal's head, in [`Interface::draw`]
    /// until this change. The fields stay: `Druid::readout` (`src/druid/
    /// mod.rs`, a different lane's file for the length of this program)
    /// still builds them, and a struct literal missing a field the
    /// constructor still writes would not compile. What changed is only
    /// that [`Readout::lines`] no longer reads them.
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
        lines.push((format!("YOUR CIRCLE R{}", self.carried_radius), TEXT));
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

/// **The options menu**, flattened the moment it is built — same reason as
/// [`Founding`]: the comparison against last frame is what tells the
/// dirty-rect skip the corner owes a repaint.
#[derive(Clone, PartialEq, Debug)]
struct Options {
    /// Label and current value, per row.
    rows: Vec<(String, String)>,
    row: usize,
    note: String,
}

/// **The founding screen**, flattened to text the moment it is built.
///
/// Rows first, then a detail pane for the one you are on: three candidates at
/// this width cannot each carry a blurb and six trait lines side by side, and
/// the version that tried read as a wall. A list you scan plus a pane for the
/// one under the cursor is the same information in a third of the pixels.
#[derive(Clone, PartialEq, Debug)]
struct Founding {
    rows: Vec<Row>,
    picked: usize,
    /// The chosen body, and what choosing it buys — the dial the owner asked
    /// for. Above the lineages, because it applies to all three of them.
    body: String,
    blurb: String,
    dial: String,
}

/// One lineage's row: what it is, what the roll gave it, what it costs, and
/// whether the pool can pay for it.
#[derive(Clone, PartialEq, Debug)]
struct Row {
    name: String,
    /// Each rolled word with how far from neutral it is, 0..1.
    words: Vec<(String, f32)>,
    cost: String,
    afford: bool,
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
    motes: Vec<Mote>,
    /// Screen position and 0..1 age of each arrival bloom.
    landings: Vec<(i32, i32, f32)>,
    /// Which plane the marks below are on, so the colour can say so.
    scent_b: bool,
    /// Scent he has laid, in screen pixels, with how strong it still is —
    /// **quantised to [`SCENT_BANDS`] levels, and that is not cosmetic.**
    ///
    /// This value is compared against last frame's to decide whether the
    /// corner owes a repaint. A raw 0..1 strength changes on *every* frame,
    /// because the plane is decaying every pass — so a trail lying on settled
    /// ground would force a full repaint for ever and quietly cost the
    /// dirty-rect skip its whole job. Banding means the comparison moves only
    /// when a mark visibly changes, which is seconds apart.
    scent: Vec<(i32, i32, u8)>,
    /// The founding screen, while it is open.
    founding: Option<Founding>,
    /// The options menu, while it is open.
    options: Option<Options>,
}

impl Interface {
    pub fn build(game: &Druid, viewport: (u32, u32)) -> Self {
        let readout = game.readout();
        Interface {
            power_bar: (!readout.unlimited).then(|| (readout.power / readout.power_full).clamp(0.0, 1.0)),
            status: readout.lines(),
            keys: game.show_keys,
            rings: rings(game, viewport),
            motes: motes(game),
            landings: landings(game),
            scent_b: game.scent == crate::sim::pheromone::Channel::B,
            scent: scent(game),
            founding: founding(game),
            options: options(game),
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

        // **What he has told them.** Under the marks and motes, over the
        // rings: it is ground he has written on, not an event.
        let (faint, full) = if self.scent_b { (SCENT_B_FAINT, SCENT_B) } else { (SCENT_FAINT, SCENT) };
        for (x, y, band) in &self.scent {
            hc.put(frame, *x, *y, lerp(faint, full, *band as f32 / (SCENT_BANDS - 1) as f32));
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
        // **Aura first, cores after**, in two passes, so no core is painted
        // over by the next mote's halo.
        //
        // **`put`, not a blend, and that is not a shortcut.** A blend into a
        // region the renderer skipped compounds frame on frame until it is
        // flat opaque — the defect the panel fill already paid for, and
        // `drawing_the_same_interface_twice_changes_nothing` is the guard.
        // So the aura is a dim *colour*, drawn once, rather than alpha.
        for m in &self.motes {
            let halo = lerp(PANEL, FLOW_FAINT, 0.35 + m.bright * 0.45);
            for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
                hc.put(frame, m.x + dx, m.y + dy, halo);
            }
            if m.bright > 0.6 {
                let corner = lerp(PANEL, FLOW_FAINT, 0.2 + m.bright * 0.25);
                for (dx, dy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    hc.put(frame, m.x + dx, m.y + dy, corner);
                }
            }
        }
        for m in &self.motes {
            hc.put(frame, m.x, m.y, lerp(FLOW_FAINT, FLOW, m.bright));
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

        // **The options menu, drawn before the founding screen** so that if
        // both were somehow open only one is seen -- and this one is the one
        // whose keys `key` routes first.
        if let Some(o) = &self.options {
            let h = PAD * 2 + LINE * (o.rows.len() as i32 + 4) + 6;
            let left = (viewport.0 as i32 - MENU_W) / 2;
            let top = (viewport.1 as i32 - h) / 2;
            panel(hc, frame, viewport, (left, top, MENU_W, h));
            hc.text(frame, left + PAD, top + PAD, "OPTIONS", ACCENT);
            for (i, (label, value)) in o.rows.iter().enumerate() {
                let y = top + PAD + (i as i32 + 1) * LINE + 4;
                let on = i == o.row;
                if on {
                    hc.text(frame, left + PAD, y, ">", ACCENT);
                }
                hc.text(frame, left + PAD + 8, y, label, if on { TEXT } else { DIM });
                // Green for on and grey for off, so the state of the whole
                // menu reads before a word of it does.
                let tint = if value == "OFF" { DIM } else { GOOD };
                hc.text(frame, left + MENU_VALUE, y, value, tint);
            }
            let foot = top + PAD + (o.rows.len() as i32 + 1) * LINE + 8;
            hc.text(frame, left + PAD + 8, foot, &o.note, DIM);
            hc.text(frame, left + PAD + 8, foot + LINE, MENU_KEYS, KEYCAP);
            return;
        }

        // **The founding screen goes over everything and takes the keys with
        // it.** Drawn last so nothing crosses it, and it returns before the
        // legend below: while the screen is up, `KEYS` is a list of bindings
        // that are not live, and a legend that lies is worse than none.
        if let Some(f) = &self.founding {
            let h = PAD * 2 + LINE * (f.rows.len() as i32 + 5) + 6;
            let left = (viewport.0 as i32 - OFFER_W) / 2;
            let top = (viewport.1 as i32 - h) / 2;
            panel(hc, frame, viewport, (left, top, OFFER_W, h));
            hc.text(frame, left + PAD, top + PAD, "FOUND A COLONY", ACCENT);
            // **The body first, because it applies to all three lineages**
            // and because it is the dial the first version did not have.
            hc.text(frame, left + PAD + 8, top + PAD + LINE + 2, &f.body, TEXT);
            hc.text(frame, left + PAD + 8, top + PAD + LINE * 2 + 2, &f.blurb, DIM);
            for (i, row) in f.rows.iter().enumerate() {
                let y = top + PAD + (i as i32 + 3) * LINE + 6;
                let on = i == f.picked;
                // **The cursor is a character, not a highlight.** A filled
                // row behind text is one more thing to get right against the
                // panel fill, and at 5x7 an arrow reads from further away.
                if on {
                    hc.text(frame, left + PAD, y, ">", ACCENT);
                }
                hc.text(frame, left + PAD + 8, y, &row.name, if on { TEXT } else { DIM });
                let mut x = left + OFFER_COL;
                for (word, strength) in &row.words {
                    let w = hud::text_width(word);
                    // Stop before the cost column rather than drawing under
                    // it; a lineage with six loud traits is rare and reads
                    // fine truncated.
                    if x + w > left + OFFER_W - 40 {
                        break;
                    }
                    // Strength as brightness, so a strong roll is visibly a
                    // strong roll before a word of it is read.
                    let tint = if on { lerp(DIM, ACCENT, *strength) } else { lerp(EDGE, DIM, *strength) };
                    hc.text(frame, x, y, word, tint);
                    x += w + 6;
                }
                let cw = hud::text_width(&row.cost);
                hc.text(frame, left + OFFER_W - PAD - cw, y, &row.cost, if row.afford { GOOD } else { WARN });
            }
            let foot = top + PAD + (f.rows.len() as i32 + 3) * LINE + 10;
            hc.text(frame, left + PAD + 8, foot, &f.dial, TEXT);
            hc.text(frame, left + PAD + 8, foot + LINE, FOUNDING_KEYS, KEYCAP);
            return;
        }

        if !self.keys {
            return;
        }
        let (keys_w, keys_h) = legend_size();
        // **Above the bar, not above the screen edge.** The bar is always
        // on screen (see the button-bar section below) and the legend is a
        // panel like any other here — it must not sit under an opaque strip
        // that is itself repainted every frame.
        let top = viewport.1 as i32 - MARGIN - BAR_HEIGHT - keys_h;
        panel(hc, frame, viewport, (MARGIN, top, keys_w, keys_h));
        for (i, (key, what)) in legend_rows().enumerate() {
            let y = top + PAD + i as i32 * LINE;
            hc.text(frame, MARGIN + PAD, y, key, KEYCAP);
            hc.text(frame, MARGIN + PAD + KEY_COL, y, what, TEXT);
        }
    }
}

/// **The scent trail, read back off the plane rather than remembered.**
///
/// The brightness of every mark is `pheromone_at` at that cell *now*, so a
/// mark fades exactly as its scent does and is dropped when the scent is
/// gone. Remembering how bright it was when it was laid would draw a trail
/// standing over ground that no longer smells of anything — a picture of the
/// gesture instead of a picture of the world, which is the failure mode this
/// repo's overlay rule is about.
///
/// Only the cells he laid are sampled, not the screen: a per-pixel read of
/// the plane every frame is sweep-scale work for a readout.
fn scent(game: &Druid) -> Vec<(i32, i32, u8)> {
    // **A disc around each mark, not the mark.** Owner, on the first version:
    // *"It should look more like what pheromones look like in the pheromone
    // overlay. More diffuse. Not a single sharp line."* — and he is right
    // about the world as well as about the picture. The plane *does* diffuse
    // (`pheromone::DIFFUSE`), so sampling only the cells he walked drew the
    // spine of a cloud and threw the cloud away. `render.rs`'s own overlay
    // samples every cell in view, which is the honest thing and is
    // sweep-scale work for a readout; a disc per mark reaches the same field
    // over the only region that can be non-zero.
    //
    // Deduped by *screen* cell rather than world cell, because that is what
    // gets drawn and the discs overlap heavily along a walked route.
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for &(mx, my) in &game.trail {
        for dy in -SCENT_HALO..=SCENT_HALO {
            for dx in -SCENT_HALO..=SCENT_HALO {
                if dx * dx + dy * dy > SCENT_HALO * SCENT_HALO {
                    continue;
                }
                let (x, y) = (mx + dx, my + dy);
                let v = game.world.pheromone_at(game.scent, x, y);
                if v == 0 {
                    continue;
                }
                let Some((sx, sy)) = game.renderer.world_to_screen(x, y) else { continue };
                if !seen.insert((sx, sy)) {
                    continue;
                }
                out.push((sx, sy, (v as u16 * SCENT_BANDS as u16 / 256) as u8));
            }
        }
    }
    out
}

fn options(game: &Druid) -> Option<Options> {
    let m = game.menu.as_ref()?;
    Some(Options {
        rows: super::menu::SETTINGS.iter().map(|s| (s.label().to_string(), s.value(game))).collect(),
        row: m.row,
        note: m.current().note().to_string(),
    })
}

/// What the options menu binds, drawn along its bottom.
const MENU_KEYS: &str = "W S CHOOSE    SPACE CHANGE    M OR X CLOSE";
/// Where a row's value is right-aligned to.
const MENU_W: i32 = 404;
/// How wide the menu's own name column runs before the value.
const MENU_VALUE: i32 = 250;

/// **Flatten the offer into what the screen draws.**
///
/// Done here rather than at draw time for the reason every other field on
/// `Interface` is: the comparison against last frame is what tells the
/// dirty-rect skip that the corner owes a repaint, and a screen built out of
/// a live `&Druid` at draw time cannot be compared with anything.
fn founding(game: &Druid) -> Option<Founding> {
    let offer = game.offer.as_ref()?;
    let rows = offer
        .candidates()
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let cost = offer.cost_of(i);
            Row {
                // **Numbered, not named.** The name is the body's and the
                // body is now one choice above, shared by all three -- a
                // column repeating it three times said nothing and cost the
                // width the trait words needed.
                name: format!("LINE {}", i + 1),
                words: c.lines().into_iter().map(|l| (l.word.to_string(), l.strength)).collect(),
                cost: format!("{cost:.0}"),
                afford: game.unlimited || game.power >= cost,
            }
        })
        .collect();
    Some(Founding {
        rows,
        picked: offer.picked,
        body: format!("BODY  {}", offer.stock().name),
        blurb: offer.stock().blurb.to_string(),
        dial: format!("FOUNDERS {}    COST {:.0}    POWER {:.0}", offer.founders, offer.cost(), game.power),
    })
}

/// What the founding screen binds, drawn along its bottom. Not in [`KEYS`]:
/// these are live only while the screen is up, and the legend is a list of
/// what works *now*.
const FOUNDING_KEYS: &str = "A D LINE   Q E BODY   Z V HOW MANY   C FOUND   X LEAVE";

/// Where the words start, past the widest stock name.
const OFFER_COL: i32 = 52;
/// Where the cost is right-aligned to.
const OFFER_W: i32 = 340;

fn lerp(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let t = t.clamp(0.0, 1.0);
    let mix = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round().clamp(0.0, 255.0) as u8;
    [mix(0), mix(1), mix(2), 255]
}

/// **How many particles one draw's amount buys.**
///
/// Owner, item 2 of the 2026-09-14 playtest: *"no indication of how much
/// life energy the ants have. You just get less or none if you try to
/// absorb too soon. You should just see how much energy you get by how
/// many particles come."* Before this the count was a flat 26 for every
/// draw regardless of `Draw::amount`, so an absorb taken the instant a
/// reserve opened and one taken at a full reserve looked identical — the
/// number the whole complaint is about was computed and then thrown away
/// before it ever reached the screen.
///
/// **A line, not a step function** — `CLAUDE.md`'s first law, *an outcome is
/// a distribution, not a binary*: a reserve just above zero reads as a thin
/// trickle, a full `super::RESERVE_CAP` (40) reads as a real pull, and every
/// amount between is visibly between.
///
/// **The floor is not decorative.** Every `Draw` that reaches this function
/// already cleared `taken > 0.0` in `super::Druid::absorb` or `placed > 0`
/// in `super::Druid::commit_founding`, so `amount` is never zero here —
/// "nothing charged" already reads as *no draw at all*, which `absorb`'s
/// own refusal message says out loud. What the floor guards against is a
/// small positive amount rounding to one or two motes, which at this
/// game's zoom is indistinguishable from a rendering glitch rather than
/// from "a little".
///
/// **The founding path is the same formula on purpose, and it is the
/// reading that needed checking rather than assuming.** A founding's
/// `commit_founding` divides `paid` by `placed`, and
/// `founding::Candidate::cost`'s own `PER_FOUNDER` is 12 — so a founding's
/// per-stream `amount` sits in roughly the same 12..40 range `absorb`'s
/// per-animal reserve does (reserve cap 40), rather than the much smaller
/// number the caution above worried about. One formula reads honestly in
/// both directions; see the lane report for the measured comparison, and
/// `PIXEL_PHYSICS_DRUID_GIF`/`_ABSORB_AT`/`_FOUND_AT` for how to reproduce
/// it.
fn mote_count_for_amount(amount: f32) -> i32 {
    // At the old fixed count (26), a full-reserve absorb (40) and a typical
    // founding stream (12..40) both land close to this slope's output, so
    // the flow at those amounts reads about the same as it did before this
    // change — what moves is everything *below* a full reserve.
    /// Thinnest a real draw is ever allowed to look — enough motes that a
    /// small pull still reads as *several particles*, per the module doc's
    /// "not a couple big orbs".
    const FLOOR: f32 = 6.0;
    const PER_UNIT: f32 = 0.6;
    (FLOOR + amount.max(0.0) * PER_UNIT).round() as i32
}

/// **The flow: dozens of small particles streaming from each animal to the
/// player.**
///
/// Owner: *"It should look like individual particles of energy flowing. Not a
/// couple big orbs."* So the stream's weight comes from **count and spread**
/// rather than from the size of any one mote — [`mote_count_for_amount`] of
/// them per draw, each on its own phase, each wandering sideways off the
/// line by its own amount.
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
    let Some(player) = &game.world.player else {
        return Vec::new();
    };
    let (px, py) = player.center();
    let mut out = Vec::new();
    for d in &game.draws {
        let per_draw = mote_count_for_amount(d.amount);
        let head = d.age as f32 / super::DRAW_FRAMES as f32;
        // The two ends, in flow order. Everything below is written in terms
        // of "origin" and "destination" rather than "animal" and "player", so
        // an outward founding is the same arithmetic with the ends swapped
        // and needs no second generator.
        let ((ox, oy), (tx, ty)) = if d.outward { ((px, py), d.from) } else { (d.from, (px, py)) };
        let (dx, dy) = ((tx - ox) as f32, (ty - oy) as f32);
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        // Perpendicular to the path, for the wander.
        let (nx, ny) = (-dy / len, dx / len);
        for i in 0..per_draw {
            let seed = i as f32;
            // Spread over the journey, and let the leaders run ahead of the
            // head so the stream has a ragged front rather than a wall.
            let t = head * 1.3 - (seed / per_draw as f32) * 0.62 - (seed * 0.37).fract() * 0.06;
            if !(0.0..=1.0).contains(&t) {
                continue;
            }
            let ease = t * t * (3.0 - 2.0 * t);
            // Narrows to nothing at the destination: the stream converges
            // on wherever it is going.
            let spread = (1.0 - ease) * 7.0;
            let wander = ((t * 7.0 + seed * 2.399).sin() + (seed * 1.7).sin()) * 0.5 * spread;
            // **The bow decays before it arrives**, rather than being
            // symmetric about the midpoint. Owner: *"it seems to go to the
            // top of the gnome instead of the middle."* `center()` is the
            // middle and always was — what read as the top was a symmetric
            // arc lifting the whole stream, so the last thing the eye
            // followed was still 11 cells high when the flow ended. The
            // `(1 - t)` factor peaks the bow at about a third of the way
            // along and flattens the approach into the body.
            let arc = (t * std::f32::consts::PI).sin() * (1.0 - t) * 14.0;
            let wx = ox as f32 + dx * ease + nx * wander;
            let wy = oy as f32 + dy * ease + ny * wander - arc;
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
    game.draws
        .iter()
        .filter_map(|d| {
            let head = d.age as f32 / super::DRAW_FRAMES as f32;
            if head < 1.0 - BLOOM {
                return None;
            }
            // **The bloom belongs at the far end, not on the player.** An
            // outward founding that punctuated itself back at the caster
            // would read as the ground paying *him*, which is the opposite
            // of what just happened.
            let (ex, ey) = if d.outward { d.from } else { (px, py) };
            let (sx, sy) = game.renderer.world_to_screen(ex, ey)?;
            Some((sx, sy, ((head - (1.0 - BLOOM)) / BLOOM).clamp(0.0, 1.0)))
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

/// The keys that still need a legend row — everything in [`KEYS`] the bar
/// does not already caption. See [`KEYS`]'s own doc for why the array
/// itself stays complete while this view is shorter.
fn legend_rows() -> impl Iterator<Item = (&'static str, &'static str)> {
    KEYS.iter().filter(|(_, _, has_button)| !has_button).map(|(key, what, _)| (*key, *what))
}

/// The legend panel's own size, derived from its rows rather than written
/// down — a hardcoded box is what lets a row run off the bottom of it, which
/// `App::help_columns`'s doc records happening to the line that said which
/// key closed the page.
fn legend_size() -> (i32, i32) {
    let rows: Vec<(&str, &str)> = legend_rows().collect();
    let widest = rows.iter().map(|(_, what)| hud::text_width(what)).max().unwrap_or(0);
    (PAD * 2 + KEY_COL + widest, PAD * 2 + rows.len() as i32 * LINE - 2)
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

// -------------------------------------------------------------- button bar
//
// **Owner's ask, 2026-09-14, item 3 of the playtest**: *"We need an actual
// GUI. We can keep it minimalistic, but buttons for the main actions (with
// subtle hotkey always visible). I don't want to have to remember all these
// shortcuts and the menu isn't even a menu, it is a shortcut list."*
//
// **Ported from `lab::ui`, not invented** — that module already solved this
// (a retained, measured, self-fitting button bar), and only the smallest
// honest subset comes across: no icons, no hover-note popup, no pages. A
// face two lines tall — the verb, and under it, dimmer, the key that also
// does it — is the whole ask, and it is also exactly `lab::ui::Widget`'s
// shape.
//
// **Retained state lives on `bin/druid.rs`'s `Handler`, not on [`Druid`].**
// `src/druid/mod.rs` is a different lane's file for the length of this
// program (see `Reports/lanes/druid-screen.md`), so this module cannot add
// a field to [`Druid`] to hold a cursor position or a pressed button across
// frames. The cursor, the press-armed action and the laid-out [`Bar`]
// itself are therefore fields on `Handler`, and [`Druid::act`] below is
// reached from there rather than from a method a stateful `Ui` owns.
// Functionally this is the same shape the lab uses — a retained bar tested
// against the player's last look, one dispatch point every control routes
// through — only which struct is holding the pen differs.
//
// **Nothing here is folded into [`Interface`]'s repaint comparison, and
// that is a considered choice rather than an oversight.** The bar occupies
// a fixed rectangle that is always on screen, always fully repainted (`put`,
// never `blend`) and never toggled or resized, so the world underneath it
// never needs the `ui_changed` force-repaint the rest of this module exists
// to trigger — that mechanism is for a footprint that can change or
// disappear, and the bar's footprint does neither. A hover highlight is
// therefore *not* the hazard it is on the lab's bar (`lab::ui::Ui::is_dirty`
// returns `true` outright while the cursor is in the window, because a
// hover "leaves no footprint the dirty-rect skip knows to erase" there):
// here the bar's own rectangle is unconditionally repainted every drawn
// frame regardless of hover, by [`draw_bar`] being called every frame from
// `Handler::frame` — so a hover highlight changing is just one more thing
// that fixed repaint draws differently, never a stale pixel left behind.
// See the lane report for the measured per-frame cost of that fixed
// repaint.

/// How tall the bar is, in logical pixels — two rows, the same shape
/// `lab::ui::BAR_HEIGHT` uses and the same total (3 top + 24 + 2 gap + 24 +
/// 3 bottom = 56), because it is a proven fit for this font at this width
/// rather than a coincidence.
pub const BAR_HEIGHT: i32 = BTN_TOP * 2 + BTN_HEIGHT * 2 + BTN_ROW_GAP;
const BTN_TOP: i32 = 3;
const BTN_HEIGHT: i32 = 24;
const BTN_ROW_GAP: i32 = 2;
/// Horizontal padding inside a button, and the gap between two buttons —
/// tried loosest first; see [`bar_layout`].
const BTN_SPACINGS: [(i32, i32); 3] = [(2, 2), (2, 1), (1, 1)];

const BAR_BG: [u8; 4] = [20, 22, 27, 255];
const BAR_EDGE: [u8; 4] = [70, 90, 115, 255];
const FACE: [u8; 4] = [43, 47, 56, 255];
const FACE_HOVER: [u8; 4] = [68, 75, 89, 255];
const FACE_DOWN: [u8; 4] = [25, 27, 33, 255];
const FACE_ON: [u8; 4] = [44, 92, 68, 255];
const FACE_ON_HOVER: [u8; 4] = [62, 122, 90, 255];
const BTN_EDGE: [u8; 4] = [78, 85, 99, 255];
const BTN_EDGE_ON: [u8; 4] = [120, 198, 148, 255];
const LABEL: [u8; 4] = [226, 230, 236, 255];
const LABEL_ON: [u8; 4] = [234, 255, 240, 255];
const SUB: [u8; 4] = [124, 131, 145, 255];
const SUB_ON: [u8; 4] = [156, 202, 176, 255];

/// **The first screen row the bar covers.** Everything above it is the
/// world and the rest of the HUD — `lab::ui::bar_top`'s own doc.
pub fn bar_top() -> i32 {
    crate::app::HEIGHT as i32 - BAR_HEIGHT
}

fn bar_row_y(row: usize) -> i32 {
    bar_top() + BTN_TOP + row as i32 * (BTN_HEIGHT + BTN_ROW_GAP)
}

/// A rectangle in framebuffer pixels. `lab::ui::Rect` in miniature, copied
/// rather than shared: `lab` and `druid` do not depend on each other, by
/// the same rule that keeps this a separate binary at all (`Druid`'s own
/// module doc), and a `pub` bridge between them for one struct is the wrong
/// direction to reach for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Rect {
    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
    fn right(&self) -> i32 {
        self.x + self.w
    }
    fn bottom(&self) -> i32 {
        self.y + self.h
    }
}

/// **What a bar button, or a key, does — the one verb set both reach.**
///
/// `Druid::act` is the single place a control turns into a change, mirroring
/// `lab::mod::Lab::act`'s own doc: *"the single place a control turns into a
/// change… there is no second copy of what SPACE does."* `Handler::act` in
/// `src/bin/druid.rs` is the actual outer dispatch point both the key
/// handler and the bar's click handler call — see that function's doc for
/// why the split.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    TogglePause,
    ToggleHeld,
    PlaceCircle,
    LiftCircle,
    FoundColony,
    Absorb,
    SowSeed,
    CycleSeedKind,
    CycleLook,
    CycleScent,
    ToggleOptions,
    ToggleUnlimited,
    /// The biosphere page (item 1 of the playtest). Not handled by
    /// [`Druid::act`] below — the page itself lives on `Handler`, for the
    /// same reason the bar's own state does.
    ToggleStats,
}

/// One button on the bar.
struct Widget {
    rect: Rect,
    line1: String,
    /// **The subtle hotkey the owner asked for**, drawn dimmer than
    /// `line1` — see [`paint_widget`].
    line2: String,
    action: Action,
    latched: bool,
}

/// The whole bar, laid out. Produced by [`bar_layout`] and retained by
/// `Handler` so that a click landing between two frames is tested against
/// the bar the player was actually looking at — `lab::ui`'s own module doc
/// names this as the reason its bar is retained rather than rebuilt at
/// click time.
#[derive(Default)]
pub struct Bar {
    widgets: Vec<Widget>,
}

impl Bar {
    /// The action under `(x, y)`, if any.
    pub fn hit(&self, x: i32, y: i32) -> Option<Action> {
        self.widgets.iter().find(|w| w.rect.contains(x, y)).map(|w| w.action)
    }

    fn fits(&self) -> bool {
        self.widgets.iter().all(|w| w.rect.x >= MARGIN && w.rect.right() <= crate::app::WIDTH as i32 - MARGIN)
    }
}

fn cell_width(label_px: i32, sub: &str, pad: i32) -> i32 {
    label_px.max(hud::text_width(sub)) + pad * 2
}

struct Spec {
    row: usize,
    width: i32,
    line1: String,
    line2: &'static str,
    action: Action,
    latched: bool,
}

fn btn(row: usize, label: String, sub: &'static str, action: Action, latched: bool, pad: i32) -> Spec {
    Spec { row, width: cell_width(hud::text_width(&label), sub, pad), line1: label, line2: sub, action, latched }
}

/// **What the bar needs to know in order to lay itself out and say which
/// of its toggles are on** — `lab::ui::BarState` in miniature, and built for
/// the same reason [`Readout`] is split out of [`Interface`]: `Druid::new`
/// generates and grows a world, which is a minute of wall clock, and
/// [`bar_layout`]'s own guards need to build a bar without paying that on
/// every run. A snapshot of plain values rather than a borrow of [`Druid`]
/// also sidesteps needing a live game at all in a test.
struct BarState {
    paused: bool,
    held: bool,
    offer_open: bool,
    menu_open: bool,
    unlimited: bool,
    stats_open: bool,
    seed_kind: String,
    look: String,
    scent: &'static str,
}

fn bar_state(game: &Druid, stats_open: bool) -> BarState {
    BarState {
        paused: game.paused,
        held: game.world.held,
        offer_open: game.offer.is_some(),
        menu_open: game.menu.is_some(),
        unlimited: game.unlimited,
        stats_open,
        seed_kind: game.seed_kind_name().to_uppercase(),
        look: game.renderer.held_look.label().to_uppercase(),
        scent: match game.scent {
            crate::sim::pheromone::Channel::A => "HOME",
            _ => "FOOD",
        },
    }
}

/// **The main actions, and what is left on the keyboard.**
///
/// Not every bound key is here — the owner asked for buttons on the *main*
/// actions, not twenty-three of them. Left as keyboard-only: the three
/// continuous dials (`Q`/`E` place radius, `[`/`]` carried-circle reach,
/// `Z`/`V` speed) — a button pressed fifty-one times is not a control
/// (`lab::ui`'s own `STOCK_LADDER` doc makes the same call) — and the four
/// held movement keys (`A`/`D`/`W`/`S`/`SHIFT`/`G`), which a press-and-
/// release button cannot express at all.
///
/// **Two rows widen dynamically: seed kind, held-ground look and scent
/// plane double as their own readout**, the same idiom `lab::ui`'s species
/// chip uses — the current value is the label, so there is nothing to
/// desync between a chip and a side table naming what it shows.
fn bar_specs(state: &BarState, pad: i32) -> Vec<Spec> {
    vec![
        btn(0, if state.paused { "RESUME".to_string() } else { "PAUSE".to_string() }, "P", Action::TogglePause, state.paused, pad),
        btn(0, if state.held { "RUN".to_string() } else { "HOLD".to_string() }, "H", Action::ToggleHeld, false, pad),
        btn(0, "PLACE".to_string(), "SPACE", Action::PlaceCircle, false, pad),
        btn(0, "LIFT".to_string(), "X", Action::LiftCircle, false, pad),
        btn(0, "FOUND".to_string(), "C", Action::FoundColony, state.offer_open, pad),
        btn(0, "ABSORB".to_string(), "F", Action::Absorb, false, pad),
        btn(1, "SOW".to_string(), "T", Action::SowSeed, false, pad),
        btn(1, state.seed_kind.clone(), "K", Action::CycleSeedKind, false, pad),
        btn(1, state.look.clone(), "L", Action::CycleLook, false, pad),
        btn(1, state.scent.to_string(), "I", Action::CycleScent, false, pad),
        btn(1, "OPTIONS".to_string(), "M", Action::ToggleOptions, state.menu_open, pad),
        btn(1, "UNLIMITED".to_string(), "U", Action::ToggleUnlimited, state.unlimited, pad),
        btn(1, "STATS".to_string(), "TAB", Action::ToggleStats, state.stats_open, pad),
    ]
}

fn lay_out(state: &BarState, pad: i32, gap: i32) -> Bar {
    let specs = bar_specs(state, pad);
    let mut widgets = Vec::with_capacity(specs.len());
    for row in 0..2 {
        let mut x = MARGIN;
        for spec in specs.iter().filter(|s| s.row == row) {
            let rect = Rect { x, y: bar_row_y(row), w: spec.width, h: BTN_HEIGHT };
            x = rect.right() + gap;
            widgets.push(Widget { rect, line1: spec.line1.clone(), line2: spec.line2.to_string(), action: spec.action, latched: spec.latched });
        }
    }
    Bar { widgets }
}

/// Lay the whole bar out. Pure: same state in, same rectangles out.
///
/// **Widths are measured, never written down** — every label goes through
/// `hud::text_width`, so renaming a button cannot silently leave its face
/// narrower than its own text. **Three spacings, tried loosest first**,
/// copied from `lab::ui::layout`'s own discipline: a bar sized for today's
/// button count is one renamed label away from overflowing, and closing the
/// gaps between buttons is far more readable than losing the last one off
/// the screen.
pub fn bar_layout(game: &Druid, stats_open: bool) -> Bar {
    let state = bar_state(game, stats_open);
    layout_for(&state)
}

fn layout_for(state: &BarState) -> Bar {
    for (pad, gap) in BTN_SPACINGS {
        let bar = lay_out(state, pad, gap);
        if bar.fits() {
            return bar;
        }
    }
    let (pad, gap) = BTN_SPACINGS[BTN_SPACINGS.len() - 1];
    lay_out(state, pad, gap)
}

fn fill_rect(hc: Hud, frame: &mut [u8], r: Rect, colour: [u8; 4]) {
    for y in r.y..r.bottom() {
        for x in r.x..r.right() {
            hc.put(frame, x, y, colour);
        }
    }
}

fn outline_rect(hc: Hud, frame: &mut [u8], r: Rect, colour: [u8; 4]) {
    for x in r.x..r.right() {
        hc.put(frame, x, r.y, colour);
        hc.put(frame, x, r.bottom() - 1, colour);
    }
    for y in r.y..r.bottom() {
        hc.put(frame, r.x, y, colour);
        hc.put(frame, r.right() - 1, y, colour);
    }
}

fn paint_widget(hc: Hud, frame: &mut [u8], w: &Widget, hover: bool, down: bool) {
    let (face, edge, label, sub) = match (w.latched, hover, down) {
        (_, _, true) => (FACE_DOWN, BTN_EDGE_ON, LABEL, SUB),
        (true, true, _) => (FACE_ON_HOVER, BTN_EDGE_ON, LABEL_ON, SUB_ON),
        (true, false, _) => (FACE_ON, BTN_EDGE_ON, LABEL_ON, SUB_ON),
        (false, true, _) => (FACE_HOVER, BTN_EDGE, LABEL, LABEL),
        (false, false, _) => (FACE, BTN_EDGE, LABEL, SUB),
    };
    fill_rect(hc, frame, w.rect, face);
    outline_rect(hc, frame, w.rect, edge);
    let tx = w.rect.x + (w.rect.w - hud::text_width(&w.line1)) / 2;
    hc.text(frame, tx, w.rect.y + 4, &w.line1, label);
    let sx = w.rect.x + (w.rect.w - hud::text_width(&w.line2)) / 2;
    hc.text(frame, sx, w.rect.y + 4 + LINE, &w.line2, sub);
}

/// **Paint the bar. Called every drawn frame, unconditionally** — see the
/// section doc above for why that is what keeps a hover highlight safe
/// without folding the bar into [`Interface`]'s repaint comparison.
pub fn draw_bar(bar: &Bar, frame: &mut [u8], viewport: (u32, u32), cursor: Option<(i32, i32)>, pressed: Option<Action>) {
    let hc = Hud::new(viewport.0, viewport.1, 1);
    let plate = Rect { x: 0, y: bar_top(), w: viewport.0 as i32, h: BAR_HEIGHT };
    fill_rect(hc, frame, plate, BAR_BG);
    for x in 0..viewport.0 as i32 {
        hc.put(frame, x, bar_top(), BAR_EDGE);
    }
    for w in &bar.widgets {
        let hover = cursor.is_some_and(|(x, y)| w.rect.contains(x, y));
        let down = hover && pressed == Some(w.action);
        paint_widget(hc, frame, w, hover, down);
    }
}

impl Druid {
    /// **The single place a control turns into a change** — the game-state
    /// half of it. See [`Action`]'s own doc for the split with
    /// `Handler::act`, which is the actual single dispatch point in the
    /// running game: it wraps this for the two concerns that belong to the
    /// event loop rather than to the game (clearing held movement keys
    /// before a modal opens, and the biosphere page).
    pub fn act(&mut self, action: Action) {
        match action {
            Action::TogglePause => self.paused = !self.paused,
            Action::ToggleHeld => {
                self.world.held = !self.world.held;
                let hold = if self.world.held { crate::sim::clock::SkyPin::Noon.hold() } else { None };
                self.world.set_sky_hold(hold);
                self.note(if self.world.held { "the world is held" } else { "the world is running" });
            }
            Action::PlaceCircle => {
                self.place_quickening();
            }
            Action::LiftCircle => {
                self.lift_quickening();
            }
            Action::FoundColony => self.toggle_founding(),
            Action::Absorb => {
                self.absorb();
            }
            Action::SowSeed => {
                self.plant_seed();
            }
            Action::CycleSeedKind => self.cycle_seed_kind(),
            Action::CycleLook => {
                self.renderer.cycle_held_look();
                let look = self.renderer.held_look.label();
                self.note(format!("held ground drawn: {look}"));
            }
            Action::CycleScent => self.cycle_scent(),
            Action::ToggleOptions => self.toggle_menu(),
            Action::ToggleUnlimited => {
                self.unlimited = !self.unlimited;
                let state = if self.unlimited { "on" } else { "off" };
                self.note(format!("unlimited power {state}"));
            }
            // Handled by `Handler::act` — see this enum's own doc.
            Action::ToggleStats => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // The offer's own types, for the two screen guards below. Scoped to the
    // tests because nothing outside them names it: the screen is built from
    // a `&Druid` and flattened to strings before it reaches this module.
    use crate::druid::founding;

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
        let listed: Vec<&str> = KEYS.iter().flat_map(|(keys, _, _)| keys.split_whitespace()).collect();

        let mut bound: Vec<String> = Vec::new();
        for (i, _) in source.match_indices("KeyCode::") {
            let name: String = source[i + "KeyCode::".len()..].chars().take_while(|c| c.is_alphanumeric()).collect();
            let shown = match name.as_str() {
                "Escape" => "ESC".to_string(),
                "ShiftLeft" | "ShiftRight" => "SHIFT".to_string(),
                "Space" => "SPACE".to_string(),
                "Slash" => "/".to_string(),
                // The punctuation keys whose `KeyCode` name is a word. Every
                // one of these has to be spelled out or the guard fires on a
                // legend that is perfectly correct -- which it has now done
                // twice, once for `TAB` and once for these.
                "BracketLeft" => "[".to_string(),
                "BracketRight" => "]".to_string(),
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
        for (key, what, _) in KEYS {
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
                carried_radius: 28,
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

    /// **The widest founding screen the offer can produce**, for the two
    /// guards below. Built by hand rather than rolled: a random offer is a
    /// sample, and what these guards need is the worst case.
    fn widest_offer() -> Founding {
        let longest = founding::STOCKS.iter().max_by_key(|s| s.blurb.len()).unwrap();
        Founding {
            rows: (0..founding::OFFERED)
                .map(|i| Row {
                    name: format!("LINE {}", i + 1),
                    // Every rolled slot loud at once -- rare, and the case the
                    // row has to survive.
                    words: founding::Candidate { deltas: [1.0; crate::sim::organism::CREATURE_TRAITS] }
                        .lines()
                        .into_iter()
                        .map(|l| (l.word.to_string(), l.strength))
                        .collect(),
                    cost: "9999".to_string(),
                    afford: false,
                })
                .collect(),
            picked: 0,
            body: format!("BODY  {}", founding::STOCKS.iter().max_by_key(|s| s.name.len()).unwrap().name),
            blurb: longest.blurb.to_string(),
            dial: format!("FOUNDERS {}    COST 9999    POWER 9999", founding::FOUNDERS_MAX),
        }
    }

    /// The widest options menu, for the two guards below — the longest label
    /// against the longest note, which are not the same row.
    fn widest_menu() -> Options {
        use crate::druid::menu::SETTINGS;
        Options {
            rows: SETTINGS.iter().map(|s| (s.label().to_string(), "UNCHANGED".to_string())).collect(),
            row: 0,
            note: SETTINGS.iter().map(|s| s.note()).max_by_key(|n| n.len()).unwrap().to_string(),
        }
    }

    /// **The options menu fits, and every character of it has a glyph.**
    /// Sibling of the founding guard below and for the same reason: its
    /// strings never pass through `Readout`.
    #[test]
    fn the_options_menu_fits_and_has_glyphs_for_everything() {
        let (w, h) = (crate::app::WIDTH as i32, crate::app::HEIGHT as i32);
        let o = widest_menu();
        let panel_h = PAD * 2 + LINE * (o.rows.len() as i32 + 4) + 6;
        assert!(MENU_W <= w, "the options menu is {MENU_W} wide in a {w}-wide window");
        assert!(panel_h <= h, "the options menu is {panel_h} tall in a {h}-tall window");
        // A label must stop before the value column, and the note before the
        // right edge.
        for (label, _) in &o.rows {
            let lw = hud::text_width(label);
            assert!(PAD + 8 + lw <= MENU_VALUE, "{label:?} is {lw} wide and runs into the value column at {MENU_VALUE}");
        }
        let inner = MENU_W - PAD * 2 - 8;
        let mut checked = 0;
        for text in [o.note.as_str(), MENU_KEYS] {
            let tw = hud::text_width(text);
            assert!(tw <= inner, "{text:?} is {tw} wide inside {inner}");
            for c in text.chars() {
                assert!(hud::has_glyph(c), "the options menu draws {c:?} in {text:?}");
                checked += 1;
            }
        }
        assert!(checked > 60, "only {checked} characters swept; this guard would pass on nothing");
    }

    /// **The founding screen fits too, and every character of it has a
    /// glyph.** Its strings never pass through `Readout`, so the sweep above
    /// cannot see them, and a missing glyph draws as a blank gap rather than
    /// as an error.
    #[test]
    fn the_founding_screen_fits_and_has_glyphs_for_everything() {
        let (w, h) = (crate::app::WIDTH as i32, crate::app::HEIGHT as i32);
        let f = widest_offer();
        let panel_h = PAD * 2 + LINE * (f.rows.len() as i32 + 5) + 6;
        assert!(OFFER_W <= w, "the founding screen is {OFFER_W} wide in a {w}-wide window");
        assert!(panel_h <= h, "the founding screen is {panel_h} tall in a {h}-tall window");
        // The three text rows under the list are drawn full width, so they
        // are the ones that can run off the right edge.
        let inner = OFFER_W - PAD * 2 - 8;
        let mut checked = 0;
        for text in [f.body.as_str(), f.blurb.as_str(), f.dial.as_str(), FOUNDING_KEYS] {
            let tw = hud::text_width(text);
            assert!(tw <= inner, "{text:?} is {tw} wide inside {inner}");
            for c in text.chars() {
                assert!(hud::has_glyph(c), "the founding screen draws {c:?} in {text:?}, which the font renders as a blank gap");
                checked += 1;
            }
        }
        assert!(checked > 100, "only {checked} characters swept; this guard would pass on nothing");
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
        // **Above the bar, not above the screen edge** — see `Interface::draw`'s
        // own comment on why the legend's bottom anchor moved.
        assert!(MARGIN * 2 + BAR_HEIGHT + keys_h <= h, "the key legend is {keys_h} tall and the bar is {BAR_HEIGHT} tall, together too much for a {h}-tall window");

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
            carried_radius: 96,
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
        assert!(
            MARGIN + status_h < h - MARGIN - BAR_HEIGHT - keys_h,
            "the readout ({status_h}) and the key legend ({keys_h}, above a {BAR_HEIGHT}-tall bar) overlap in a {h}-tall window"
        );
    }

    /// **The button bar itself fits, and no two of its buttons overlap** —
    /// `lab::ui`'s own `the_bar_fits_the_screen_and_no_two_widgets_overlap`,
    /// which this is a direct port of. Swept over every combination of
    /// modal/latched state `bar_specs` reads, since a widened label (a long
    /// seed kind name, `UNCHANGED` for the held-ground look) is exactly the
    /// renamed-button failure `lab::ui`'s own doc on `layout` warns about.
    /// The widest bar the game can actually show — a long seed kind name and
    /// a long held-look label, every latch on at once. Built by hand rather
    /// than from a real `Druid`: `Druid::new` generates and grows a world,
    /// which is a minute of wall clock (see [`BarState`]'s own doc), and a
    /// guard that costs a minute is a guard nobody runs.
    fn widest_bar_state() -> BarState {
        BarState {
            paused: true,
            held: true,
            offer_open: true,
            menu_open: true,
            unlimited: true,
            stats_open: true,
            seed_kind: "SCRAMBLER".to_string(),
            look: "UNCHANGED".to_string(),
            scent: "FOOD",
        }
    }

    #[test]
    fn the_bar_fits_the_screen_and_no_two_widgets_overlap() {
        for state in [BarState { paused: false, held: false, offer_open: false, menu_open: false, unlimited: false, stats_open: false, ..widest_bar_state() }, widest_bar_state()] {
            let bar = layout_for(&state);
            assert!(bar.fits(), "the bar does not fit a {}-wide window even at its tightest spacing", crate::app::WIDTH);
            for (i, a) in bar.widgets.iter().enumerate() {
                assert!(a.rect.w >= hud::text_width(&a.line2), "{:?}'s face ({}) is narrower than its own caption {:?}", a.line1, a.rect.w, a.line2);
                for b in &bar.widgets[i + 1..] {
                    let overlap = a.rect.x < b.rect.right() && b.rect.x < a.rect.right() && a.rect.y < b.rect.bottom() && b.rect.y < a.rect.bottom();
                    assert!(!overlap, "{:?} and {:?} overlap", a.line1, b.line1);
                }
            }
        }
    }

    /// **Every button shows the key that also fires it** — `lab::ui`'s own
    /// `every_speed_chip_shows_a_key`, which is exactly the promise item 3
    /// of the playtest asked for: *"buttons for the main actions (with
    /// subtle hotkey always visible)"*. Checked against `Handler::act`'s
    /// key table would need `bin/druid.rs`'s source, which
    /// `the_legend_names_every_key_the_binary_binds` already reads for the
    /// legend; this guard is the narrower, cheaper claim that a caption is
    /// never blank.
    #[test]
    fn every_button_shows_its_key() {
        let bar = layout_for(&widest_bar_state());
        assert!(bar.widgets.len() >= 10, "only {} widgets on the bar — bar_specs moved and this guard is now blind", bar.widgets.len());
        for w in &bar.widgets {
            assert!(!w.line2.is_empty(), "{:?} has no caption at all", w.line1);
        }
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
            motes: vec![Mote { x: 160, y: 130, bright: 0.5 }],
            landings: vec![(200, 150, 0.3)],
            scent_b: false,
            scent: vec![(140, 152, 7), (141, 152, 4), (142, 153, 1)],
            // **The screen is in the idempotence guard, not beside it.** It
            // draws the biggest panel in the game, and a panel is exactly the
            // shape that compounded last time.
            founding: Some(widest_offer()),
            options: Some(widest_menu()),
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
            motes: Vec::new(),
            landings: Vec::new(),
            scent_b: false,
            scent: Vec::new(),
            founding: None,
            options: None,
        };
        let b = a.clone();
        assert_eq!(a, b, "an unchanged interface must compare equal, or the render skip never fires at all");

        a.status = vec![("POWER 599".to_string(), TEXT)];
        assert_ne!(a, b, "a changed digit must force the repaint");

        let mut c = b.clone();
        c.keys = false;
        assert_ne!(c, b, "hiding the legend must force the repaint, or it stays on screen after the key");

        // **Moving the cursor on the founding screen must force one too.**
        // Without the screen in this comparison the offer would draw once and
        // then sit there, and every keypress after that would change nothing
        // on a settled world -- which reads exactly like the keys being dead.
        let mut e = b.clone();
        e.founding = Some(widest_offer());
        assert_ne!(e, b, "opening the founding screen must force the repaint");
        let mut moved = e.clone();
        if let Some(f) = moved.founding.as_mut() {
            f.picked = 1;
        }
        assert_ne!(moved, e, "moving the cursor must force the repaint, or the keys look dead on settled ground");

        let mut om = b.clone();
        om.options = Some(widest_menu());
        assert_ne!(om, b, "opening the options menu must force the repaint");
        let mut om2 = om.clone();
        if let Some(o) = om2.options.as_mut() {
            o.row = 1;
        }
        assert_ne!(om2, om, "moving the menu cursor must force the repaint");

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
    }
}
