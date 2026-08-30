//! **The lab's control panel: the button bar along the bottom of the screen.**
//!
//! Owner request, 2026-08-30: *"We need more of a GUI. There should be buttons
//! at the bottom of the screen to play ants, plants, pop up info panels,
//! control the speed... It shouldn't all be keyboard shortcuts."*
//!
//! Nothing is *removed* by this file. Every key `bin/lab.rs` bound still
//! works and every button names its own key under its label, because the bar
//! is how you find a control and the key is how you use it once you have. A
//! bar that replaced the keys would trade one undiscoverable interface for a
//! slower one.
//!
//! **The layout function is the only place the geometry exists.** [`layout`]
//! returns a [`Bar`] — a list of [`Widget`]s each carrying its own rectangle —
//! and that one list is painted, hit-tested for clicks, and hit-tested for
//! hover. `CLAUDE.md` calls out two copies of a layout as how a button and the
//! thing it activates come to disagree, and lane A's `stats.rs` reaches the
//! same shape from the other side (build the whole row list *before* painting
//! anything, so self-measurement, hover and the paint all read one list).
//!
//! **The bar is retained, and it has to be.** A click arrives between frames,
//! so `release` hit-tests the `Bar` the *last* painted frame produced — which
//! is the bar the player was looking at when they pressed. Re-deriving the
//! rectangles at click time would be the second copy this module exists to
//! avoid.
//!
//! **Every label that names a quantity explains itself on hover.** Owner
//! ruling on the sandbox's colony panel, 2026-08-30: *"the user should be able
//! to mouse hover over some of the words and get an explanation of what it
//! means and this could also be a way to access more details data."* So a
//! [`Row`] carries its note as a field rather than in a side table — lane A's
//! `stats.rs` idiom, copied deliberately so the two agree at the merge instead
//! of diverging — and so does every [`Widget`] on the bar.
//!
//! **The panels carry deltas, not just counts.** The owner's verdict on a
//! still frame of a breeding colony against a sterile one was *"no, not
//! without motion at least"*: an ant is two dark cells and a population that
//! is climbing looks exactly like one that is collapsing. A number a picture
//! cannot show is the readout's whole job, so each population row prints its
//! change since the last sample and draws the last few dozen samples as a
//! strip. Samples are taken on `World::frame`, never per displayed frame: one
//! displayed frame is 1 tick at 1x and up to 256 at the top of the ladder, so
//! a per-call sample would make the x-axis the speed dial rather than time.

use std::collections::VecDeque;

use crate::hud;
use crate::render;
use crate::sim::world::World;

use super::scene::LabBox;
use super::{HEIGHT as H, WIDTH as W};

// --------------------------------------------------------------- geometry

/// How many rows of the screen the bar owns, measured up from the bottom.
///
/// 30 is not arbitrary: a button holds two 7-pixel rows (its label and its
/// keyboard shortcut) plus padding, which is 24, and the bar needs an edge
/// and a margin around that.
pub const BAR_HEIGHT: i32 = 30;
const BTN_TOP: i32 = 3;
const BTN_HEIGHT: i32 = 24;
/// Horizontal padding inside a button, per side.
const PAD: i32 = 2;
/// Between two buttons of the same group.
const GAP: i32 = 2;
/// Between the bar's contents and the screen edge.
const MARGIN: i32 = 4;
/// Baseline-to-baseline for stacked text.
const LINE: i32 = hud::GLYPH_HEIGHT + 2;
const ICON_W: i32 = 6;
const ICON_GAP: i32 = 3;

/// The first screen row the bar covers. Everything above it is the world, and
/// a click there inspects rather than pressing anything.
pub fn bar_top() -> i32 {
    H as i32 - BAR_HEIGHT
}

// ---------------------------------------------------------------- palette
//
// Opaque, not a blend. The bar is an instrument panel bolted over the world,
// and a translucent one reads as a tint on the box rather than as chrome —
// which is the difference between "the lab has controls" and "the lab has a
// smudge along the bottom".

const BAR_BG: [u8; 4] = [20, 22, 27, 255];
const BAR_EDGE: [u8; 4] = [74, 82, 96, 255];
const DIVIDER: [u8; 4] = [48, 53, 63, 255];
const FACE: [u8; 4] = [43, 47, 56, 255];
const FACE_HOVER: [u8; 4] = [68, 75, 89, 255];
const FACE_DOWN: [u8; 4] = [25, 27, 33, 255];
const FACE_ON: [u8; 4] = [44, 92, 68, 255];
const FACE_ON_HOVER: [u8; 4] = [62, 122, 90, 255];
const EDGE: [u8; 4] = [78, 85, 99, 255];
const EDGE_ON: [u8; 4] = [120, 198, 148, 255];
const LABEL: [u8; 4] = [226, 230, 236, 255];
const LABEL_ON: [u8; 4] = [234, 255, 240, 255];
const SUB: [u8; 4] = [124, 131, 145, 255];
const SUB_ON: [u8; 4] = [156, 202, 176, 255];
const READOUT_BG: [u8; 4] = [14, 16, 20, 255];
const PANEL_BG: [u8; 4] = [17, 19, 24, 255];
const PANEL_EDGE: [u8; 4] = [88, 98, 114, 255];
const TITLE: [u8; 4] = [238, 238, 210, 255];
const VALUE: [u8; 4] = [228, 234, 240, 255];
const FAINT: [u8; 4] = [132, 139, 153, 255];
const NOTE_BG: [u8; 4] = [16, 20, 30, 255];
const GOOD: [u8; 4] = [112, 208, 132, 255];
const FAIR: [u8; 4] = [224, 192, 92, 255];
const POOR: [u8; 4] = [224, 112, 92, 255];
const MARKER: [u8; 4] = [255, 214, 92, 255];

/// Colour for "the box achieved this much of what was asked". Graded rather
/// than a pass/fail tint: `CLAUDE.md`'s first law is that an outcome is a
/// distribution, and a dial that only knows "keeping up" and "not keeping up"
/// throws away the whole middle the readout exists to show.
fn grade(ratio: f32) -> [u8; 4] {
    if ratio >= 0.8 {
        GOOD
    } else if ratio >= 0.4 {
        FAIR
    } else {
        POOR
    }
}

// ------------------------------------------------------------------ rects

/// A rectangle in framebuffer pixels: `x..x+w` by `y..y+h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
    fn right(&self) -> i32 {
        self.x + self.w
    }
    fn bottom(&self) -> i32 {
        self.y + self.h
    }
}

/// The first fill-a-rectangle primitive in this engine — there was none, and
/// every panel until now dimmed what was behind it instead. Opaque on purpose;
/// [`render::blend`] is still the right call for anything that wants the world
/// to show through.
fn fill(frame: &mut [u8], r: Rect, colour: [u8; 4]) {
    for y in r.y..r.bottom() {
        for x in r.x..r.right() {
            render::put(frame, W, H, x, y, colour);
        }
    }
}

fn outline(frame: &mut [u8], r: Rect, colour: [u8; 4]) {
    for x in r.x..r.right() {
        render::put(frame, W, H, x, r.y, colour);
        render::put(frame, W, H, x, r.bottom() - 1, colour);
    }
    for y in r.y..r.bottom() {
        render::put(frame, W, H, r.x, y, colour);
        render::put(frame, W, H, r.right() - 1, y, colour);
    }
}

fn text(frame: &mut [u8], x: i32, y: i32, s: &str, colour: [u8; 4]) {
    hud::draw_text(frame, W, H, x, y, s, colour);
}

// ------------------------------------------------------------------ icons

/// The two transport glyphs, drawn as pixels rather than added to
/// `hud`'s font.
///
/// A play triangle and a pause bar are *interface*, not typography: they never
/// appear inside a string, they want to be centred against a label rather than
/// advanced past like a character, and putting them in the font would mean
/// picking two ASCII codepoints to stand for them and then explaining that
/// choice forever. `hud.rs` is also a file three other lines touch, and this
/// needed nothing from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Icon {
    Play,
    Pause,
}

fn draw_icon(frame: &mut [u8], icon: Icon, x: i32, y: i32, colour: [u8; 4]) {
    match icon {
        Icon::Play => {
            for row in 0..7 {
                let width = 4 - (row - 3i32).abs();
                for col in 0..width {
                    render::put(frame, W, H, x + col, y + row, colour);
                }
            }
        }
        Icon::Pause => {
            for row in 0..7 {
                for col in [0, 1, 4, 5] {
                    render::put(frame, W, H, x + col, y + row, colour);
                }
            }
        }
    }
}

// ----------------------------------------------------------------- actions

/// What pressing a widget does. Owned by the UI; [`super::Lab::act`] is the
/// one place that turns one of these into a change to the lab, so a button
/// and its keyboard shortcut cannot drift apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    TogglePhase,
    Slower,
    Faster,
    Preset(usize),
    Panel(Panel),
    Stats,
    Help,
    Reset,
}

/// Which info panel a button opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Plants,
    Ants,
    Box,
}

impl Panel {
    fn title(self) -> &'static str {
        match self {
            Panel::Plants => "PLANTS",
            Panel::Ants => "ANTS",
            Panel::Box => "THE BOX",
        }
    }
}

// ----------------------------------------------------------------- widgets

/// One thing drawn on the bar. A button, or — when `action` is `None` — a
/// readout that is drawn and never pressed.
pub struct Widget {
    pub rect: Rect,
    line1: String,
    line2: String,
    pub action: Option<Action>,
    latched: bool,
    icon: Option<Icon>,
    /// Achieved-against-requested, drawn as a fill strip. Readout only.
    ratio: Option<f32>,
    /// What this control does, in a sentence, shown while the cursor is over
    /// it. Carried on the widget rather than in a lookup keyed by action, for
    /// the reason lane A's `Row::note` is: a table beside the thing it
    /// describes is a table that goes stale.
    note: String,
}

/// The whole bar, laid out. Produced by [`layout`] and retained by [`Ui`] so
/// that a click landing between two frames is tested against the bar the
/// player was actually looking at.
#[derive(Default)]
pub struct Bar {
    pub widgets: Vec<Widget>,
    dividers: Vec<i32>,
}

impl Bar {
    /// The action under `(x, y)`, or `None` — including for a readout, which
    /// has a rectangle and no verb.
    pub fn hit(&self, x: i32, y: i32) -> Option<Action> {
        self.widgets.iter().find(|wid| wid.rect.contains(x, y)).and_then(|wid| wid.action)
    }

    /// Whether every widget landed inside the screen. The separator is forced
    /// to `MIN_SEPARATOR` before this is asked, so a bar that only fits by
    /// closing the gaps between its groups reports that it does not.
    fn fits(&self) -> bool {
        self.widgets.iter().all(|wid| wid.rect.x >= MARGIN && wid.rect.right() <= W as i32 - MARGIN)
    }

    fn hovered(&self, at: Option<(i32, i32)>) -> Option<&Widget> {
        let (x, y) = at?;
        self.widgets.iter().find(|wid| wid.rect.contains(x, y))
    }
}

/// Everything the bar needs to know about the lab in order to lay itself out
/// and to say which of its toggles are on.
///
/// A snapshot rather than a borrow of `Lab`, so that `Lab::draw` can hand the
/// UI its own state and the world at the same time without the borrow checker
/// having an opinion about it.
pub struct BarState {
    pub running: bool,
    pub requested: u32,
    pub achieved: f32,
    pub presets: &'static [u32],
    pub panel: Option<Panel>,
    pub stats: bool,
    pub help: bool,
}

/// Width of a cell whose label is `label_px` wide and whose shortcut caption
/// is `sub`. The caption is often the wider of the two (`SPACE` against
/// `RUN`), and a button narrower than its own caption is the sort of thing
/// that only shows up in a screenshot.
fn cell_width(label_px: i32, sub: &str, pad: i32) -> i32 {
    label_px.max(hud::text_width(sub)) + pad * 2
}

struct Spec {
    width: i32,
    line1: String,
    line2: String,
    action: Option<Action>,
    latched: bool,
    icon: Option<Icon>,
    ratio: Option<f32>,
    note: &'static str,
}

fn button(
    label: &str,
    sub: &'static str,
    action: Action,
    latched: bool,
    note: &'static str,
    pad: i32,
) -> Spec {
    Spec {
        width: cell_width(hud::text_width(label), sub, pad),
        line1: label.to_string(),
        line2: sub.to_string(),
        action: Some(action),
        latched,
        icon: None,
        ratio: None,
        note,
    }
}

/// The spacings the bar will try, loosest first.
///
/// **A bar that does not fit loses its last button off the right edge, and
/// only a screenshot ever shows it.** That is not hypothetical: this bar was
/// built against a six-stop speed ladder, the ladder grew a seventh stop
/// (`1024X`) the same day, and `REBUILD` was immediately half off the screen.
/// So the natural spacing is an attempt rather than an assumption, and a bar
/// too wide for the screen tightens instead of overflowing.
const SPACINGS: [(i32, i32); 2] = [(PAD, GAP), (1, 1)];
/// The smallest gap between two groups that still reads as a gap. A bar packed
/// tighter than this is one row of undifferentiated buttons.
const MIN_SEPARATOR: i32 = 8;

/// Lay the whole bar out. Pure: same state in, same rectangles out.
///
/// **Widths are measured, never written down.** Every label goes through
/// `hud::text_width`, so renaming a button cannot silently leave its face
/// narrower than its own text — which is the failure a hand-tuned pixel table
/// produces and only a screenshot catches.
pub fn layout(state: &BarState) -> Bar {
    for (pad, gap) in SPACINGS {
        let bar = lay_out(state, pad, gap);
        if bar.fits() {
            return bar;
        }
    }
    // Nothing fit. Draw the tightest rather than nothing, and let
    // `the_bar_fits_the_screen_and_no_two_widgets_overlap` be the thing that
    // says so — a bar that silently dropped a control would be worse than one
    // that visibly does not fit.
    let (pad, gap) = SPACINGS[SPACINGS.len() - 1];
    lay_out(state, pad, gap)
}

fn lay_out(state: &BarState, pad: i32, gap: i32) -> Bar {
    // Group 1 — transport. The phase button's face is sized to the *wider* of
    // its two captions so that pressing it does not shove the rest of the bar
    // sideways; a control that moves when you use it is a control you miss on
    // the second press.
    let phase_label = if state.running { "TEND" } else { "RUN" };
    let phase_icon = if state.running { Icon::Pause } else { Icon::Play };
    let phase_px = ICON_W
        + ICON_GAP
        + hud::text_width("TEND").max(hud::text_width("RUN"));
    // **The caption says what the press will produce, not what is true now.**
    // This is the single easiest thing in the whole bar to get backwards: in
    // Tending the button reads `RUN` because clicking it starts the run, and
    // the readout two cells to its right says `TENDING`, which is the state.
    // Verb on the button, state on the readout.
    let phase = Spec {
        width: cell_width(phase_px, "SPACE", pad),
        line1: phase_label.to_string(),
        line2: "SPACE".to_string(),
        action: Some(Action::TogglePhase),
        latched: state.running,
        icon: Some(phase_icon),
        ratio: None,
        note: "START OR STOP THE EXPERIMENT. TENDING RUNS THE BOX AT REAL TIME SO YOU CAN WORK IN IT; RUNNING FAST-FORWARDS IT AT THE SPEED THE DIAL ASKS FOR.",
    };
    let step_width = cell_width(hud::text_width("<<"), "DOWN", pad)
        .max(cell_width(hud::text_width(">>"), "UP", pad));
    let slower = Spec {
        width: step_width,
        ..button("<<", "DOWN", Action::Slower, false, "ONE STOP DOWN THE SPEED LADDER.", pad)
    };
    let faster = Spec {
        width: step_width,
        ..button(">>", "UP", Action::Faster, false, "ONE STOP UP THE SPEED LADDER. ASKING FOR SPEED FROM A STOPPED BOX ALSO STARTS THE RUN.", pad)
    };
    // **The achieved figure is only shown while it means something.** In
    // Running it is ticks-per-wall-second over the requested multiple, which
    // is exactly the honesty mechanism the dial was built around. In Tending
    // the loop asks for one tick and finishes it in microseconds, so the same
    // arithmetic reports a large number that says nothing about whether the
    // window is keeping up -- and printing `GOT 5.5X` beside `ASK 1X` reads as
    // the box running five times faster than asked. `lab::time`'s own readout
    // hides it in Tending for this reason; this one does the same rather than
    // quietly disagreeing with it. Whether the *window* keeps up is the frames
    // per second on the box page, which is a different measurement.
    let (line1, line2, ratio) = if state.running {
        (
            format!("ASK {}X", state.requested),
            format!("GOT {:.1}X", state.achieved.max(0.0)),
            state.achieved.max(0.0) / state.requested.max(1) as f32,
        )
    } else {
        ("TENDING".to_string(), "REAL TIME".to_string(), 1.0)
    };
    let readout = Spec {
        // Sized to the widest thing it can ever say, not to what it says now:
        // a readout that changes width as the number changes shoves the bar
        // sideways once a second.
        width: hud::text_width("GOT 999.9X").max(hud::text_width("REAL TIME")) + pad * 2 + 2,
        line1,
        line2,
        action: None,
        latched: false,
        icon: None,
        ratio: Some(ratio),
        note: "WHAT THE DIAL WAS ASKED FOR, AGAINST WHAT THE BOX ACTUALLY MANAGED, AND THE STRIP IS THE SECOND OVER THE FIRST. A BED THAT HAS GROWN COSTS WHAT IT COSTS AND THE DIAL IS ONLY A REQUEST. IN TENDING THERE IS NOTHING TO REPORT -- TENDING IS REAL TIME BY DEFINITION.",
    };

    // Group 2 — the speed ladder, one chip per stop.
    // The caption is derived from the position, not from a list that has to
    // be kept the same length as the ladder: the ladder gained a seventh stop
    // and a hand-written list of six would have left that chip captionless.
    let presets: Vec<Spec> = state
        .presets
        .iter()
        .enumerate()
        .map(|(i, mult)| {
            let label = format!("{mult}X");
            let key = if i < 9 { (i + 1).to_string() } else { String::new() };
            Spec {
                width: cell_width(hud::text_width(&label), &key, pad),
                line1: label,
                line2: key,
                action: Some(Action::Preset(i)),
                latched: state.requested == *mult,
                icon: None,
                ratio: None,
                note: "JUMP STRAIGHT TO THIS MULTIPLE OF REAL TIME. THE TOP OF THE LADDER IS DELIBERATELY PAST WHAT ANY BOX CAN DO -- THAT IS HOW THE ACHIEVED READOUT EARNS ITS KEEP.",
            }
        })
        .collect();

    // Group 3 — the pages.
    let panels = [
        button(
            "PLANTS",
            "F1",
            Action::Panel(Panel::Plants),
            state.panel == Some(Panel::Plants),
            "THE FLORA: HOW MANY ARE STANDING, HOW MANY HAVE EVER GERMINATED, AND WHETHER THE STAND IS CLIMBING OR DYING BACK.",
            pad,
        ),
        button(
            "ANTS",
            "F2",
            Action::Panel(Panel::Ants),
            state.panel == Some(Panel::Ants),
            "THE COLONY: HOW MANY ANIMALS ARE ALIVE, WHAT THE POPULATION IS DOING, AND HOW CLOSE THE BOX IS TO ITS ORGANISM CEILING.",
            pad,
        ),
        button(
            "BOX",
            "F3",
            Action::Panel(Panel::Box),
            state.panel == Some(Panel::Box),
            "THE BED ITSELF: HOW LONG IT HAS RUN, HOW BIG IT IS, AND WHAT IT IS COSTING TO SIMULATE.",
            pad,
        ),
        button(
            "STATS",
            "TAB",
            Action::Stats,
            state.stats,
            "THE CENSUS LINE ACROSS THE TOP OF THE SCREEN.",
            pad,
        ),
        button(
            "HELP",
            "?",
            Action::Help,
            state.help,
            "THE KEY LIST. EVERY BUTTON ON THIS BAR ALSO HAS A KEY, AND THE KEY IS PRINTED UNDER THE BUTTON.",
            pad,
        ),
        button(
            "REBUILD",
            "R",
            Action::Reset,
            false,
            "TEAR THE BOX DOWN AND BUILD THE SAME ONE AGAIN FROM ITS SPEC. THE VIEW AND THE DIAL ARE KEPT; EVERYTHING LIVING IN IT IS NOT.",
            pad,
        ),
    ];

    let groups: [Vec<Spec>; 3] = [
        vec![phase, slower, faster, readout],
        presets,
        panels.into_iter().collect(),
    ];

    // Slack goes into the gaps *between* groups rather than at one end, so the
    // three groups read as three groups and the bar stays centred if a label
    // is ever renamed. Computed here, in the same pass, for the same reason
    // everything else is.
    let content: i32 = groups
        .iter()
        .map(|g| {
            g.iter().map(|s| s.width).sum::<i32>() + gap * (g.len() as i32 - 1).max(0)
        })
        .sum();
    let slack = W as i32 - MARGIN * 2 - content;
    let separator = (slack / 2).max(MIN_SEPARATOR);

    let mut widgets = Vec::new();
    let mut dividers = Vec::new();
    let mut x = MARGIN;
    let y = bar_top() + BTN_TOP;
    for (gi, group) in groups.into_iter().enumerate() {
        if gi > 0 {
            dividers.push(x + separator / 2);
            x += separator;
        }
        for (wi, spec) in group.iter().enumerate() {
            if wi > 0 {
                x += gap;
            }
            widgets.push(Widget {
                rect: Rect { x, y, w: spec.width, h: BTN_HEIGHT },
                line1: spec.line1.clone(),
                line2: spec.line2.clone(),
                action: spec.action,
                latched: spec.latched,
                icon: spec.icon,
                ratio: spec.ratio,
                note: spec.note.to_string(),
            });
            x += spec.width;
        }
    }
    Bar { widgets, dividers }
}

fn paint_widget(frame: &mut [u8], wid: &Widget, hover: bool, down: bool) {
    let r = wid.rect;
    let (face, edge, label, sub) = match (wid.action.is_some(), wid.latched, hover, down) {
        (false, ..) => (READOUT_BG, EDGE, LABEL, FAINT),
        (_, _, _, true) => (FACE_DOWN, EDGE_ON, LABEL, SUB),
        (_, true, true, _) => (FACE_ON_HOVER, EDGE_ON, LABEL_ON, SUB_ON),
        (_, true, false, _) => (FACE_ON, EDGE_ON, LABEL_ON, SUB_ON),
        (_, false, true, _) => (FACE_HOVER, EDGE, LABEL, LABEL),
        _ => (FACE, EDGE, LABEL, SUB),
    };
    fill(frame, r, face);
    outline(frame, r, edge);

    let icon_px = wid.icon.map_or(0, |_| ICON_W + ICON_GAP);
    let text_px = icon_px + hud::text_width(&wid.line1);
    let tx = r.x + (r.w - text_px) / 2;
    let ty = r.y + 4;
    if let Some(icon) = wid.icon {
        draw_icon(frame, icon, tx, ty, label);
    }
    text(frame, tx + icon_px, ty, &wid.line1, label);

    let sy = r.y + 4 + LINE + 2;
    match wid.ratio {
        // The readout's second line is the achieved figure, and under it a
        // strip of the same length as the shortfall. Two channels for one
        // number: the digits for what it is, the strip for how far short.
        Some(ratio) => {
            let sx = r.x + (r.w - hud::text_width(&wid.line2)) / 2;
            text(frame, sx, sy - 1, &wid.line2, grade(ratio));
            let track = Rect { x: r.x + 3, y: r.bottom() - 4, w: r.w - 6, h: 2 };
            fill(frame, track, [38, 40, 46, 255]);
            let filled = (track.w as f32 * ratio.clamp(0.0, 1.0)).round() as i32;
            if filled > 0 {
                fill(frame, Rect { w: filled, ..track }, grade(ratio));
            }
        }
        None => {
            let sx = r.x + (r.w - hud::text_width(&wid.line2)) / 2;
            text(frame, sx, sy, &wid.line2, sub);
        }
    }
}

// ------------------------------------------------------------------- rows

/// What one row of an info panel is.
enum Body {
    /// A named quantity: label on the left, value on the right.
    Value { label: String, value: String, tint: [u8; 4] },
    /// A population over the last few dozen samples.
    Spark { label: String, series: Vec<u32>, tint: [u8; 4] },
    Gap,
}

/// One drawn row, and what it means.
///
/// The note is a field rather than a side table for lane A's reason, which is
/// the reason the colony panel gives too: the page is dense and every row is
/// squeezed to fit 5x7 glyphs, so the note both says what the row *means* and
/// carries the detail that did not fit on the line.
struct Row {
    body: Body,
    note: String,
}

impl Row {
    fn value(
        label: impl Into<String>,
        value: impl Into<String>,
        tint: [u8; 4],
        note: impl Into<String>,
    ) -> Self {
        Self {
            body: Body::Value { label: label.into(), value: value.into(), tint },
            note: note.into(),
        }
    }
    fn spark(
        label: impl Into<String>,
        series: Vec<u32>,
        tint: [u8; 4],
        note: impl Into<String>,
    ) -> Self {
        Self { body: Body::Spark { label: label.into(), series, tint }, note: note.into() }
    }
    fn gap() -> Self {
        Self { body: Body::Gap, note: String::new() }
    }
    fn height(&self) -> i32 {
        match self.body {
            Body::Value { .. } => LINE,
            // 22, not the strip's own 14: the strip's caption sits *under* the
            // bars, and a row measuring only the bars lets it overprint the
            // next row. Lane A's `Generations` row records the same trap.
            Body::Spark { .. } => 22,
            Body::Gap => 4,
        }
    }
    fn width(&self) -> i32 {
        match &self.body {
            Body::Value { label, value, .. } => {
                hud::text_width(label) + 12 + hud::text_width(value)
            }
            Body::Spark { label, .. } => hud::text_width(label).max(96),
            Body::Gap => 0,
        }
    }
}

// ---------------------------------------------------------------- history

/// Simulated frames between two population samples.
///
/// **Sampled on `World::frame`, never per displayed frame.** One displayed
/// frame is one tick at 1x and up to 256 at the top of the ladder, so a
/// per-call sample would put the speed dial on the x-axis instead of time and
/// the same run would draw a different shape depending on how fast you watched
/// it.
const SAMPLE_EVERY: u64 = 120;
const HISTORY: usize = 56;

#[derive(Clone, Copy, Default)]
struct Sample {
    plants: u32,
    ants: u32,
    germinations: u64,
}

#[derive(Default)]
struct History {
    samples: VecDeque<Sample>,
    last_frame: u64,
    next_at: u64,
}

impl History {
    fn observe(&mut self, world: &World) {
        // A rebuild puts the frame counter back to zero, and a series that
        // carried the old box across the reset would draw a population crash
        // that never happened.
        if world.frame < self.last_frame {
            self.samples.clear();
            self.next_at = 0;
        }
        self.last_frame = world.frame;
        if world.frame < self.next_at && !self.samples.is_empty() {
            return;
        }
        let orgs = world.live_organism_count() as u32;
        let ants = world.live_creature_count() as u32;
        self.samples.push_back(Sample {
            plants: orgs.saturating_sub(ants),
            ants,
            germinations: world.germinations,
        });
        while self.samples.len() > HISTORY {
            self.samples.pop_front();
        }
        self.next_at = world.frame + SAMPLE_EVERY;
    }

    fn series(&self, pick: fn(&Sample) -> u32) -> Vec<u32> {
        self.samples.iter().map(pick).collect()
    }

    /// The change in `pick` across the last two samples, or `None` when there
    /// is only one — which is honest, and better than printing `+0` for a box
    /// nobody has watched for long enough yet.
    fn delta(&self, pick: fn(&Sample) -> i64) -> Option<i64> {
        let n = self.samples.len();
        if n < 2 {
            return None;
        }
        Some(pick(&self.samples[n - 1]) - pick(&self.samples[n - 2]))
    }
}

/// `+3`, `-1`, `0` — and the colour says which way without reading it.
fn delta_text(d: Option<i64>) -> (String, [u8; 4]) {
    match d {
        None => ("--".to_string(), FAINT),
        Some(d) if d > 0 => (format!("+{d}"), GOOD),
        Some(d) if d < 0 => (format!("{d}"), POOR),
        Some(_) => ("0".to_string(), FAINT),
    }
}

/// Big numbers, short. The font is 5x7 and a panel is 150 pixels wide.
fn compact(v: f64) -> String {
    let a = v.abs();
    if a >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if a >= 1_000.0 {
        format!("{:.1}K", v / 1_000.0)
    } else {
        format!("{v:.0}")
    }
}

// --------------------------------------------------------------------- ui

/// What a mouse release turned out to mean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    /// A button was pressed and released over itself.
    Fired(Action),
    /// The gesture belonged to the interface and produced no verb — a press
    /// taken back by sliding off the button, or a click on a readout.
    Consumed,
    /// The gesture belonged to the world.
    World,
}

/// The bar, the pages it opens, and what the mouse is doing.
#[derive(Default)]
pub struct Ui {
    /// Where the cursor is, in framebuffer pixels. `None` when it has left.
    cursor: Option<(i32, i32)>,
    /// The action a press armed. A button fires on *release over the same
    /// button*, which is what lets a press be taken back by sliding off it —
    /// the behaviour every other button in the world has, and cheap here.
    pressed: Option<Action>,
    /// Whether the press landed on the interface at all.
    ///
    /// Needed as its own flag because `pressed` is `None` for two very
    /// different presses — one on the world, and one on the readout, which has
    /// a rectangle and no verb. Without this, dragging off a button and
    /// releasing would inspect whatever cell the release happened to be over.
    press_inside: bool,
    /// The page currently open above the bar, if any.
    pub panel: Option<Panel>,
    /// The world cell the player last clicked, re-read every frame.
    inspect: Option<(i32, i32)>,
    history: History,
    /// Last frame's layout. See the module doc: a click arrives between
    /// frames, so this is the bar the player was looking at.
    bar: Bar,
    /// Where the open page was drawn last frame, and where the inspector was.
    ///
    /// Retained for the same reason the bar is, and it is the same rule: a
    /// click must be tested against what was on screen. Sizing a page depends
    /// on what it currently has to say, so re-deriving these at click time
    /// would need the world and would be the second copy of the layout this
    /// module exists to avoid.
    panel_box: Option<Rect>,
    inspect_box: Option<Rect>,
}

impl Ui {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_cursor(&mut self, at: Option<(i32, i32)>) {
        self.cursor = at;
        if at.is_none() {
            self.cancel_press();
        }
    }

    pub fn cursor(&self) -> Option<(i32, i32)> {
        self.cursor
    }

    /// Whether `(x, y)` belongs to the interface rather than to the world —
    /// the bar, or a page open over it. A click here must not also inspect the
    /// cell behind it: an inspector that fires through its own panel moves the
    /// marker every time you try to read the panel.
    pub fn covers(&self, x: i32, y: i32) -> bool {
        y >= bar_top()
            || self.panel_box.is_some_and(|r| r.contains(x, y))
            || self.inspect_box.is_some_and(|r| r.contains(x, y))
    }

    /// Arm a press. Fires nothing: a button that acted on press could not be
    /// taken back, and a mis-click on `REBUILD` would already have thrown the
    /// box away by the time you noticed.
    pub fn press(&mut self, x: i32, y: i32) {
        self.pressed = self.bar.hit(x, y);
        self.press_inside = self.covers(x, y);
    }

    /// Release, and say what it meant.
    pub fn release(&mut self, x: i32, y: i32) -> Release {
        let armed = self.pressed.take();
        let inside = std::mem::take(&mut self.press_inside);
        if let Some(action) = armed {
            // Still over the button it armed, or the gesture was taken back.
            return if self.bar.hit(x, y) == Some(action) {
                Release::Fired(action)
            } else {
                Release::Consumed
            };
        }
        if inside || self.covers(x, y) {
            Release::Consumed
        } else {
            Release::World
        }
    }

    pub fn cancel_press(&mut self) {
        self.pressed = None;
        self.press_inside = false;
    }

    /// Point the inspector at a world cell, or put it away when the same cell
    /// is clicked twice. The panel re-reads the cell every frame rather than
    /// snapshotting it, so a clicked ant's energy falls while you watch.
    pub fn inspect(&mut self, cell: (i32, i32)) {
        self.inspect = if self.inspect == Some(cell) { None } else { Some(cell) };
    }

    pub fn inspecting(&self) -> Option<(i32, i32)> {
        self.inspect
    }

    pub fn toggle_panel(&mut self, panel: Panel) {
        self.panel = if self.panel == Some(panel) { None } else { Some(panel) };
    }

    /// Whether anything this module draws needs the frame fully repainted.
    /// Always true today, and stated rather than assumed: a hover highlight
    /// leaves no footprint the dirty-rect skip knows to erase.
    pub fn is_dirty(&self) -> bool {
        true
    }

    pub fn observe(&mut self, world: &World) {
        self.history.observe(world);
    }

    /// Where the button for `action` was drawn last frame.
    ///
    /// **Reads the retained layout, never a second copy of the arithmetic.**
    /// This is the accessor a harness or a test aims a synthetic click with,
    /// so a test that clicks "where `REBUILD` ought to be" is impossible to
    /// write: it can only click where `REBUILD` actually is.
    pub fn widget_rect(&self, action: Action) -> Option<Rect> {
        self.bar.widgets.iter().find(|wid| wid.action == Some(action)).map(|wid| wid.rect)
    }

    /// The action under `(x, y)` on the bar as it was last drawn.
    pub fn hit(&self, x: i32, y: i32) -> Option<Action> {
        self.bar.hit(x, y)
    }
}

// ------------------------------------------------------------ page content
//
// **Only numbers that are actually read out of the world appear here.** Every
// line below names the accessor it came from, because a page of plausible
// figures is worse than no page: it is the exact failure `CLAUDE.md` records
// when a collapse was read as "chunks are working" from a picture whose body
// count was zero all run.

impl Ui {
    fn panel_rows(&self, panel: Panel, world: &World, spec: &LabBox, fps: f32) -> Vec<Row> {
        let orgs = world.live_organism_count();
        let ants = world.live_creature_count();
        let plants = orgs.saturating_sub(ants);
        match panel {
            Panel::Plants => {
                let (d, tint) = delta_text(self.history.delta(|s| s.plants as i64));
                let (gd, gtint) = delta_text(self.history.delta(|s| s.germinations as i64));
                let series = self.history.series(|s| s.plants);
                let peak = series.iter().copied().max().unwrap_or(0);
                vec![
                    Row::value(
                        "STANDING",
                        plants.to_string(),
                        VALUE,
                        "LIVE ORGANISMS THAT ARE NOT ANIMALS -- EVERY PLANT CURRENTLY ALIVE IN THE BOX, ROOTED OR SEEDLING. A PLANT MARKED SENESCENT IS STILL COUNTED UNTIL ITS REMAINS ROT AWAY, SO A DYING STAND FALLS OFF GRADUALLY RATHER THAN VANISHING.",
                    ),
                    Row::value(
                        "CHANGE",
                        d,
                        tint,
                        "HOW THE STANDING COUNT MOVED ACROSS THE LAST TWO SAMPLES, 120 SIMULATED FRAMES APART. A STILL PICTURE CANNOT SHOW WHETHER A BOX FULL OF GREEN IS BREEDING OR DYING; THIS CAN.",
                    ),
                    Row::value(
                        "GERMINATED",
                        world.germinations.to_string(),
                        VALUE,
                        "EVERY SEED THAT HAS EVER GERMINATED IN THIS BOX SINCE IT WAS BUILT. IT ONLY CLIMBS, SO IT IS THE HONEST TEST OF WHETHER REPRODUCTION IS HAPPENING AT ALL -- A STAND THAT LOOKS HEALTHY AND NEVER MOVES THIS NUMBER IS NOT REPRODUCING.",
                    ),
                    Row::value(
                        "NEW SEEDLINGS",
                        gd,
                        gtint,
                        "GERMINATIONS SINCE THE LAST SAMPLE. THIS IS THE BIRTH RATE; THE ROW ABOVE IS THE RUNNING TOTAL.",
                    ),
                    Row::value(
                        "SEEDS SET",
                        self.seeds_set(world).to_string(),
                        VALUE,
                        "SEEDS SET BY PLANTS THAT ARE STILL ALIVE. SEEDS SET BY A PLANT THAT HAS SINCE DIED ARE NOT COUNTED, SO THIS FALLS WHEN A BEARER DIES.",
                    ),
                    Row::gap(),
                    Row::spark(
                        format!("TREND  PEAK {peak}"),
                        series,
                        GOOD,
                        "THE STANDING PLANT COUNT OVER THE LAST 56 SAMPLES, OLDEST ON THE LEFT. ONE SAMPLE EVERY 120 SIMULATED FRAMES, SO THE SHAPE IS THE SAME WHATEVER SPEED YOU WATCHED IT AT.",
                    ),
                ]
            }
            Panel::Ants => {
                let (d, tint) = delta_text(self.history.delta(|s| s.ants as i64));
                let (allocated, live) = world.organism_slot_usage();
                let series = self.history.series(|s| s.ants);
                let peak = series.iter().copied().max().unwrap_or(0);
                vec![
                    Row::value(
                        "ALIVE",
                        ants.to_string(),
                        VALUE,
                        "ANIMALS ALIVE IN THE BOX -- ORGANISMS WHOSE SPECIES CARRIES A CREATURE DEFINITION. AT PLAY ZOOM AN ANT IS TWO DARK CELLS AND YOU FIND IT ONLY BECAUSE IT MOVES, WHICH IS WHY THIS NUMBER IS HERE.",
                    ),
                    Row::value(
                        "CHANGE",
                        d,
                        tint,
                        "HOW THE COLONY MOVED ACROSS THE LAST TWO SAMPLES. A COLONY THAT BREEDS AND ONE THAT CANNOT LOOK IDENTICAL IN A STILL FRAME.",
                    ),
                    Row::value(
                        "PLANTS",
                        plants.to_string(),
                        FAINT,
                        "WHAT THE COLONY IS LIVING OFF. SHOWN HERE BECAUSE AN ANT COUNT WITH NO FORAGE COUNT BESIDE IT CANNOT SAY WHY IT IS FALLING.",
                    ),
                    Row::value(
                        "ALL ENERGY",
                        compact(world.live_creature_energy()),
                        VALUE,
                        "TOTAL ENERGY HELD BY EVERY LIVE ORGANISM, PLANTS INCLUDED -- NOT THE ANTS ALONE. IT IS THE LEFT-HAND SIDE OF THE ENERGY LEDGER, AND IT IS LABELLED THIS WAY BECAUSE CALLING IT ANT ENERGY WOULD BE WRONG.",
                    ),
                    Row::value(
                        "SLOTS",
                        format!("{live}/{allocated}"),
                        if allocated >= 4000 { POOR } else { FAINT },
                        "LIVE ORGANISMS AGAINST ORGANISM SLOTS EVER ALLOCATED. THE SECOND NUMBER IS THE HIGH-WATER MARK OF CONCURRENT LIFE AND IT NEVER FALLS. THE CEILING IS 4095, AND A BIRTH REFUSED AT THE CEILING IS A BIRTH THAT DID NOT HAPPEN.",
                    ),
                    Row::gap(),
                    Row::spark(
                        format!("TREND  PEAK {peak}"),
                        series,
                        FAIR,
                        "THE ANIMAL COUNT OVER THE LAST 56 SAMPLES, OLDEST ON THE LEFT. ONE SAMPLE EVERY 120 SIMULATED FRAMES.",
                    ),
                ]
            }
            Panel::Box => vec![
                Row::value(
                    "FRAME",
                    world.frame.to_string(),
                    VALUE,
                    "SIMULATED TICKS SINCE THE BOX WAS BUILT. SIXTY TICKS IS ONE SIMULATED SECOND, AT EVERY SPEED -- THE DIAL MULTIPLIES TICKS AND NEVER CHANGES WHAT A TICK IS, SO A FAST-FORWARDED RUN IS THE SAME SIMULATION AND NOT AN APPROXIMATION.",
                ),
                Row::value(
                    "BED",
                    format!("{}X{}", spec.width, spec.height),
                    FAINT,
                    "THE BOX IN CELLS. ONE CELL IS ONE SCREEN PIXEL AT ZOOM 1.",
                ),
                Row::value(
                    "SOIL",
                    format!("{} DEEP", spec.soil_depth),
                    FAINT,
                    "ROWS OF SOIL UNDER THE SURFACE. PLANTS ROOT INTO IT AND ANIMALS DIG IN IT, AND DEPTH IS PAID FOR IN FRAME TIME -- 40 ROWS TO 240 COSTS ABOUT TWICE THE FRAME.",
                ),
                Row::value(
                    "COMPARTMENTS",
                    spec.compartments.to_string(),
                    FAINT,
                    "SEALED WALLS FLOOR TO CEILING. THEY BUY EVOLUTIONARY ISOLATION AND THEY ALSO BUY SPEED, BECAUSE A WALLED BED SETTLES INTO SEPARATE QUIET REGIONS.",
                ),
                Row::value(
                    "ACTIVE SITES",
                    world.active_site_count().to_string(),
                    VALUE,
                    "CELLS THE SCHEDULER STILL HAS SOMETHING TO DO ABOUT. IT IS THE SHARPEST READING OF WHETHER THE BOX IS BUSY OR SETTLED, AND IT DRIVES WHAT THE SPEED DIAL CAN ACHIEVE.",
                ),
                Row::value(
                    "AWAKE CHUNKS",
                    format!("{}/{}", world.active_chunk_count(), world.chunk_count()),
                    VALUE,
                    "CHUNKS THAT WILL BE SWEPT THIS TICK, AGAINST EVERY CHUNK IN THE BOX. A SETTLED BOX SWEEPS ALMOST NOTHING, WHICH IS WHY IT RUNS FAST.",
                ),
                Row::value(
                    "SEED",
                    spec.seed.to_string(),
                    FAINT,
                    "THE NUMBER THIS BOX WAS BUILT FROM. THE SAME SEED AND THE SAME BUILD REBUILD THE SAME BOX EXACTLY.",
                ),
                Row::value(
                    "DISPLAY",
                    format!("{fps:.0} FPS"),
                    if fps >= 30.0 { GOOD } else { FAIR },
                    "DRAWN FRAMES PER SECOND. THIS IS THE WINDOW, NOT THE SIMULATION -- THE SPEED READOUT ON THE BAR IS THE SIMULATION.",
                ),
            ],
        }
    }

    /// Seeds set by plants that are still alive.
    ///
    /// Walks the live organism list rather than reading a world-level counter,
    /// because there is not one: `seeds_set` lives on `OrganismState` and dies
    /// with its bearer. Only computed while the plants page is open, which is
    /// what keeps an O(organisms) walk off every frame.
    fn seeds_set(&self, world: &World) -> u32 {
        world
            .live_organism_ids()
            .into_iter()
            .filter_map(|id| world.organism_state(id))
            .map(|state| state.seeds_set)
            .sum()
    }

    /// What the inspector says about the cell the player clicked.
    ///
    /// Always five rows, present or absent, so the page does not resize under
    /// the cursor as the cell changes underneath it — and so the rectangle the
    /// click test uses is a function of nothing but whether the inspector is
    /// open.
    fn inspect_rows(&self, world: &World, at: (i32, i32)) -> Vec<Row> {
        let (x, y) = at;
        let cell = world.get(x, y);
        let material = world.materials.get(cell.material).display.clone();
        let organism = world.organism(cell.organism_id());
        let (species, energy) = match organism {
            Some(state) => (
                world.species.get(state.species).name.to_uppercase(),
                format!("{:.1}", state.energy),
            ),
            None => ("NONE".to_string(), "--".to_string()),
        };
        vec![
            Row::value("AT", format!("{x},{y}"), FAINT, "THE CELL YOU CLICKED, IN WORLD COORDINATES. CLICK IT AGAIN TO PUT THE INSPECTOR AWAY."),
            Row::value("MATERIAL", material, VALUE, "WHAT IS IN THE CELL RIGHT NOW. RE-READ EVERY FRAME, SO IT CHANGES UNDER YOU WHILE THE BOX RUNS."),
            Row::value("TEMPERATURE", format!("{}C", cell.temperature()), FAINT, "THIS CELL'S OWN TEMPERATURE IN DEGREES, NOT THE BOX AVERAGE. HEAT MOVES CELL TO CELL, SO TWO CELLS SIDE BY SIDE CAN DISAGREE AND THE DIFFERENCE IS WHAT DRIVES IT."),
            Row::value("ORGANISM", species, if organism.is_some() { GOOD } else { FAINT }, "THE SPECIES OF THE LIVING THING THIS CELL BELONGS TO, IF ANY. AN ANT IS TWO CELLS AND A TREE IS THOUSANDS; EITHER WAY THE CELL KNOWS WHICH ORGANISM OWNS IT."),
            Row::value("ENERGY", energy, VALUE, "THAT ORGANISM'S WHOLE-BODY ENERGY, NOT THIS CELL'S SHARE. WATCH IT WHILE THE BOX RUNS: A FORAGING ANT CLIMBS AND A STARVING ONE DOES NOT."),
        ]
    }
}

// ------------------------------------------------------------- page paint

const PAGE_PAD: i32 = 8;
const PAGE_HEADER: i32 = 20;

fn page_rect(rows: &[Row], anchor_x: i32, bottom: i32) -> Rect {
    let content: i32 = rows.iter().map(Row::height).sum();
    let inner = rows.iter().map(Row::width).max().unwrap_or(0).max(120);
    let w = inner + PAGE_PAD * 2;
    let h = PAGE_HEADER + content + PAGE_PAD;
    let x = anchor_x.min(W as i32 - MARGIN - w).max(MARGIN);
    Rect { x, y: bottom - h, w, h }
}

/// Draw one page and return the note under the cursor, if any.
///
/// Hover is found **inside** the paint loop, with `y` advancing by the row's
/// own height — lane A's idiom, and for its reason: a second pass over the
/// same arithmetic is how a label and its explanation come to disagree about
/// which row they belong to.
fn paint_page(
    frame: &mut [u8],
    rect: Rect,
    title: &str,
    rows: &[Row],
    cursor: Option<(i32, i32)>,
) -> Option<(String, i32)> {
    fill(frame, rect, PANEL_BG);
    outline(frame, rect, PANEL_EDGE);
    text(frame, rect.x + PAGE_PAD, rect.y + 6, title, TITLE);
    for x in rect.x + 1..rect.right() - 1 {
        render::put(frame, W, H, x, rect.y + PAGE_HEADER - 4, DIVIDER);
    }

    let left = rect.x + PAGE_PAD;
    let right = rect.right() - PAGE_PAD;
    let mut y = rect.y + PAGE_HEADER;
    let mut hovered = None;
    for row in rows {
        if let Some((cx, cy)) = cursor {
            if !row.note.is_empty()
                && (rect.x..rect.right()).contains(&cx)
                && (y..y + row.height()).contains(&cy)
            {
                hovered = Some((row.note.clone(), y));
                // A hovered row lights up, so the note is visibly *about*
                // something rather than floating beside the page.
                fill(frame, Rect { x: rect.x + 1, y, w: rect.w - 2, h: row.height() }, [34, 40, 52, 255]);
            }
        }
        match &row.body {
            Body::Gap => {}
            Body::Value { label, value, tint } => {
                text(frame, left, y, label, FAINT);
                text(frame, right - hud::text_width(value), y, value, *tint);
            }
            Body::Spark { label, series, tint } => {
                draw_spark(frame, Rect { x: left, y, w: right - left, h: 12 }, series, *tint);
                text(frame, left, y + 14, label, FAINT);
            }
        }
        y += row.height();
    }
    hovered
}

/// A population over time, oldest on the left.
///
/// Scaled to the series' own peak rather than to a fixed axis: the two
/// kingdoms differ by two orders of magnitude in this box, and one shared
/// axis would draw the colony as a flat line on the floor whatever it did.
fn draw_spark(frame: &mut [u8], area: Rect, series: &[u32], tint: [u8; 4]) {
    fill(frame, area, [24, 27, 33, 255]);
    let peak = series.iter().copied().max().unwrap_or(0);
    if series.is_empty() {
        text(frame, area.x + 2, area.y + 2, "NO SAMPLES YET", FAINT);
        return;
    }
    let bar_w = (area.w / series.len() as i32).max(1);
    for (i, v) in series.iter().enumerate() {
        let x = area.x + i as i32 * bar_w;
        if x >= area.right() {
            break;
        }
        // A live population of zero still draws one row, so "everything died"
        // is a flat line on the floor rather than an empty panel that reads as
        // "nothing was sampled".
        let h = if peak == 0 {
            1
        } else {
            ((*v as f32 / peak as f32) * (area.h - 1) as f32).round() as i32 + 1
        };
        let w = bar_w.min(area.right() - x);
        fill(frame, Rect { x, y: area.bottom() - h, w, h }, tint);
        // The top of each bar, brighter. A series that barely varies fills the
        // strip almost solid and reads as "no information"; the profile line
        // is what makes a *flat* population look flat rather than look full.
        fill(frame, Rect { x, y: area.bottom() - h, w, h: 1 }, [244, 248, 252, 255]);
    }
}

/// Where a hover explanation goes relative to what it explains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Note {
    /// Clear of the bar entirely — for a note about a bar button.
    AboveBar,
    /// To one side of an open page — for a note about one of its rows.
    BesidePage,
}

/// The hover explanation.
///
/// Placed **beside** what it explains, never under the cursor: a box that
/// follows the pointer covers the row it is describing, so you read the
/// explanation having lost the thing it was about. Lane A's `stats.rs` reached
/// this the same way and opens its note to the left for the same reason.
fn draw_note(frame: &mut [u8], note: &str, avoid: Rect, row_y: i32, place: Note) {
    let width = 210.min(W as i32 - MARGIN * 2);
    let columns = ((width - 12) / (hud::GLYPH_WIDTH + 1)).max(8) as usize;
    let lines = wrap_words(note, columns);
    let height = lines.len() as i32 * LINE + 10;
    let (x, y) = match place {
        // A bar button's note lifts clear of the whole bar rather than sitting
        // beside the button. Beside would put it over the bar's other buttons,
        // which is precisely the "covers the thing it explains" failure this
        // function exists to avoid — the bar is one row, so *up* is the only
        // direction that clears it.
        Note::AboveBar => (
            avoid.x.min(W as i32 - MARGIN - width).max(MARGIN),
            (bar_top() - 6 - height).max(MARGIN),
        ),
        // A page's note opens to the side, and never over the bar.
        Note::BesidePage => {
            let x = if avoid.right() + 6 + width <= W as i32 - MARGIN {
                avoid.right() + 6
            } else if avoid.x - 6 - width >= MARGIN {
                avoid.x - 6 - width
            } else {
                MARGIN
            };
            (x, (row_y - 6).min(bar_top() - MARGIN - height).max(MARGIN))
        }
    };
    let r = Rect { x, y, w: width, h: height };
    fill(frame, r, NOTE_BG);
    outline(frame, r, PANEL_EDGE);
    for (i, line) in lines.iter().enumerate() {
        text(frame, x + 6, y + 5 + i as i32 * LINE, line, [232, 236, 242, 255]);
    }
}

/// Break `text` into lines of at most `columns` characters, on spaces. A word
/// longer than the column count overruns rather than being split — most of
/// them are numbers, and a number cut in half reads as two numbers.
fn wrap_words(text: &str, columns: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + word.chars().count() <= columns => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

// ----------------------------------------------------------------- drawing

impl Ui {
    /// Paint the whole interface: the marker on an inspected cell, whichever
    /// pages are open, the bar, and — over everything — the hover note.
    ///
    /// Lays the bar out **once** and keeps it, so the rectangles a click is
    /// tested against are the rectangles that were drawn.
    pub fn draw(
        &mut self,
        frame: &mut [u8],
        world: &World,
        spec: &LabBox,
        state: &BarState,
        renderer: &crate::render::Renderer,
        fps: f32,
    ) {
        self.bar = layout(state);

        // The inspected cell, marked in the world. The verb has to leave a
        // mark: a panel that names a cell without showing which cell it is
        // makes the player find it again by counting pixels.
        if let Some((wx, wy)) = self.inspect {
            let (x0, y0, x1, y1, _) = renderer.world_rect_to_screen(wx, wy, wx, wy);
            let (x0, y0) = (x0 - 3, y0 - 3);
            let (x1, y1) = (x1 + 3, y1 + 3);
            if y1 < bar_top() {
                let ring = Rect { x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 };
                outline(frame, ring, MARKER);
                // Four ticks outside the ring. A bare 7x7 outline is a smudge
                // at this scale -- and the cell it marks is often an ant, which
                // is two dark cells you can only find because they move. The
                // reticle is what makes a *stopped* box's inspected cell
                // findable at all.
                for t in 2..5 {
                    let (cx, cy) = (ring.x + ring.w / 2, ring.y + ring.h / 2);
                    render::put(frame, W, H, cx, ring.y - t, MARKER);
                    render::put(frame, W, H, cx, ring.bottom() - 1 + t, MARKER);
                    render::put(frame, W, H, ring.x - t, cy, MARKER);
                    render::put(frame, W, H, ring.right() - 1 + t, cy, MARKER);
                }
            }
        }

        let mut note: Option<(String, Rect, i32, Note)> = None;

        if let Some(panel) = self.panel {
            let rows = self.panel_rows(panel, world, spec, fps);
            // **Bottom-left, not under the button that opened it.** Anchoring
            // a page to its own button is the better affordance and it is not
            // available here: `lab::stats` draws its biosphere page down the
            // whole right-hand column, the page buttons are the bar's
            // right-hand group, and a page opening under its own button lands
            // on top of it. Caught by looking at a contact sheet with both
            // open, which is the only thing that would have shown it.
            let rect = page_rect(&rows, MARGIN, bar_top() - 4);
            self.panel_box = Some(rect);
            if let Some((text, y)) = paint_page(frame, rect, panel.title(), &rows, self.cursor) {
                note = Some((text, rect, y, Note::BesidePage));
            }
        } else {
            self.panel_box = None;
        }

        if let Some(at) = self.inspect {
            let rows = self.inspect_rows(world, at);
            // Beside the open page rather than under it, so opening a page
            // does not hide the cell you are inspecting.
            let anchor = self.panel_box.map_or(MARGIN, |r| r.right() + 6);
            let rect = page_rect(&rows, anchor, bar_top() - 4);
            self.inspect_box = Some(rect);
            if let Some((text, y)) = paint_page(frame, rect, "CELL", &rows, self.cursor) {
                note = Some((text, rect, y, Note::BesidePage));
            }
        } else {
            self.inspect_box = None;
        }

        // The bar itself, over everything a page drew, because a page opens
        // *above* it and the two must not fight over the seam.
        let bar = Rect { x: 0, y: bar_top(), w: W as i32, h: BAR_HEIGHT };
        fill(frame, bar, BAR_BG);
        for x in 0..W as i32 {
            render::put(frame, W, H, x, bar.y, BAR_EDGE);
        }
        for dx in &self.bar.dividers {
            for y in bar.y + 5..bar.bottom() - 5 {
                render::put(frame, W, H, *dx, y, DIVIDER);
            }
        }
        for wid in &self.bar.widgets {
            let hover = self.cursor.is_some_and(|(x, y)| wid.rect.contains(x, y));
            let down = hover && self.pressed.is_some() && self.pressed == wid.action;
            paint_widget(frame, wid, hover, down);
        }
        // A bar button explains itself too, and its note wins over a page's:
        // the cursor can only be over one of them, and the bar is on top.
        if let Some(wid) = self.bar.hovered(self.cursor) {
            if !wid.note.is_empty() {
                note = Some((wid.note.clone(), wid.rect, wid.rect.y - 4, Note::AboveBar));
            }
        }

        if let Some((body, avoid, y, place)) = note {
            draw_note(frame, &body, avoid, y, place);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect as WorldRect;

    fn state(running: bool, requested: u32) -> BarState {
        BarState {
            running,
            requested,
            achieved: 6.4,
            presets: &super::super::time::PRESETS,
            panel: None,
            stats: true,
            help: false,
        }
    }

    fn world() -> World {
        World::new(WorldRect::new(0, 0, 63, 63))
    }

    /// **The font cannot draw everything, and what it cannot draw it draws as
    /// nothing.** `[`/`]`, then `_`/`<`/`>`, then `;`/`'` have each shipped
    /// blank in this engine's UI for as long as they were bound — three
    /// separate times, each found by looking at a rendered image rather than
    /// by any test. This is `lab::every_help_line_is_drawable` extended over
    /// every string this module can put on the screen: button faces, the keys
    /// printed under them, every hover explanation, every page row and the
    /// inspector.
    #[test]
    fn every_string_the_bar_can_draw_is_drawable() {
        let mut checked = 0;
        let mut check = |s: &str, whence: &str| {
            for c in s.chars() {
                assert!(crate::hud::has_glyph(c), "no glyph for {c:?} in {s:?} ({whence})");
            }
            checked += 1;
        };
        for running in [false, true] {
            for requested in super::super::time::PRESETS {
                let bar = layout(&state(running, requested));
                for wid in &bar.widgets {
                    check(&wid.line1, "button face");
                    check(&wid.line2, "shortcut caption");
                    check(&wid.note, "hover explanation");
                }
            }
        }
        let (world, spec, ui) = (world(), LabBox::default(), Ui::new());
        for panel in [Panel::Plants, Panel::Ants, Panel::Box] {
            check(panel.title(), "page title");
            for row in ui.panel_rows(panel, &world, &spec, 60.0) {
                match &row.body {
                    Body::Value { label, value, .. } => {
                        check(label, "page row");
                        check(value, "page value");
                    }
                    Body::Spark { label, .. } => check(label, "strip caption"),
                    Body::Gap => {}
                }
                check(&row.note, "row explanation");
            }
        }
        for row in ui.inspect_rows(&world, (4, 4)) {
            if let Body::Value { label, value, .. } = &row.body {
                check(label, "inspector row");
                check(value, "inspector value");
            }
            check(&row.note, "inspector explanation");
        }
        assert!(checked > 200, "the sweep only reached {checked} strings");
    }

    /// A bar wider than the screen loses its last button off the right edge,
    /// and a screenshot is the only thing that would ever show it. Widths come
    /// from `hud::text_width`, so renaming a button is exactly the change that
    /// would do it.
    #[test]
    fn the_bar_fits_the_screen_and_no_two_widgets_overlap() {
        for running in [false, true] {
            for requested in super::super::time::PRESETS {
                let bar = layout(&state(running, requested));
                for wid in &bar.widgets {
                    assert!(wid.rect.x >= 0, "{:?} starts off the left edge", wid.line1);
                    assert!(
                        wid.rect.right() <= W as i32,
                        "{:?} runs off the right edge at {} of {W}",
                        wid.line1,
                        wid.rect.right()
                    );
                    assert!(
                        wid.rect.bottom() <= H as i32 && wid.rect.y >= bar_top(),
                        "{:?} is outside the bar",
                        wid.line1
                    );
                    // A face narrower than its own caption clips the caption.
                    assert!(
                        wid.rect.w >= hud::text_width(&wid.line2),
                        "{:?} is narrower than its own shortcut caption",
                        wid.line1
                    );
                }
                for (i, a) in bar.widgets.iter().enumerate() {
                    for b in &bar.widgets[i + 1..] {
                        assert!(
                            a.rect.right() <= b.rect.x || b.rect.right() <= a.rect.x,
                            "{:?} and {:?} overlap",
                            a.line1,
                            b.line1
                        );
                    }
                }
            }
        }
    }

    /// **The positive control on the hit test.** Every button is clicked at
    /// the middle of the rectangle it was *laid out* at, and must answer with
    /// its own action. It cannot pass against a hand-written pixel table,
    /// because the coordinates come from the layout itself — which is the only
    /// way to guard "the button and the thing it activates agree" without
    /// writing the second copy of the arithmetic that guard exists to catch.
    #[test]
    fn every_button_answers_where_it_was_drawn() {
        let bar = layout(&state(false, 1));
        let mut buttons = 0;
        for wid in &bar.widgets {
            let (cx, cy) = (wid.rect.x + wid.rect.w / 2, wid.rect.y + wid.rect.h / 2);
            assert_eq!(bar.hit(cx, cy), wid.action, "{:?} answered for another widget", wid.line1);
            // ...and every corner of it, since a rectangle off by one in
            // either direction still passes a centre test.
            for (x, y) in [
                (wid.rect.x, wid.rect.y),
                (wid.rect.right() - 1, wid.rect.y),
                (wid.rect.x, wid.rect.bottom() - 1),
                (wid.rect.right() - 1, wid.rect.bottom() - 1),
            ] {
                assert_eq!(bar.hit(x, y), wid.action, "{:?} missed its own corner", wid.line1);
            }
            buttons += usize::from(wid.action.is_some());
        }
        assert_eq!(buttons, 15, "the bar should carry 15 pressable buttons");
        // Nothing above the bar is pressable — that belongs to the world.
        assert_eq!(bar.hit(10, bar_top() - 1), None);
    }

    /// **The single easiest thing on this bar to get backwards.** The face
    /// names the state the press will *produce*, not the state that is true —
    /// so it reads `RUN` while the box is tending, beside a readout that says
    /// `TENDING`. Verb on the button, state on the readout.
    ///
    /// Both directions are asserted, because a button stuck on one caption
    /// would satisfy either half alone.
    #[test]
    fn the_phase_button_names_what_the_press_will_produce() {
        let tending = layout(&state(false, 1));
        let running = layout(&state(true, 64));
        let face = |bar: &Bar| {
            bar.widgets
                .iter()
                .find(|w| w.action == Some(Action::TogglePhase))
                .map(|w| w.line1.clone())
                .expect("the bar has a phase button")
        };
        assert_eq!(face(&tending), "RUN", "a stopped box must offer to run");
        assert_eq!(face(&running), "TEND", "a running box must offer to stop");
        // And the readout beside it names the state, so the two together are
        // unambiguous rather than each being half a sentence.
        let readout = |bar: &Bar| {
            bar.widgets.iter().find(|w| w.action.is_none()).map(|w| w.line1.clone()).unwrap()
        };
        assert_eq!(readout(&tending), "TENDING");
        assert!(readout(&running).starts_with("ASK"), "{}", readout(&running));
    }

    /// The achieved figure is the dial's honesty mechanism and it is also
    /// meaningless in Tending, where one tick finishes in microseconds and the
    /// same arithmetic reports a large multiple of real time. Shown in
    /// Running, withheld in Tending — asserted both ways, since a readout that
    /// never printed it at all would pass the first half.
    #[test]
    fn the_achieved_figure_is_shown_running_and_withheld_while_tending() {
        let line2 = |running, requested| {
            layout(&state(running, requested))
                .widgets
                .iter()
                .find(|w| w.action.is_none())
                .map(|w| w.line2.clone())
                .unwrap()
        };
        assert_eq!(line2(true, 64), "GOT 6.4X");
        assert_eq!(line2(false, 1), "REAL TIME");
    }

    /// The latch is the only thing on the bar saying which preset is live, and
    /// exactly one of them must claim it.
    #[test]
    fn exactly_one_speed_chip_is_latched() {
        for (i, requested) in super::super::time::PRESETS.iter().enumerate() {
            let bar = layout(&state(*requested > 1, *requested));
            let latched: Vec<usize> = bar
                .widgets
                .iter()
                .enumerate()
                .filter(|(_, w)| w.latched && matches!(w.action, Some(Action::Preset(_))))
                .map(|(j, _)| j)
                .collect();
            assert_eq!(latched.len(), 1, "{requested}x latched {} chips", latched.len());
            assert_eq!(bar.widgets[latched[0]].line1, format!("{requested}X"));
            assert_eq!(bar.widgets[latched[0]].action, Some(Action::Preset(i)));
        }
    }

    fn armed(state: &BarState) -> Ui {
        Ui { bar: layout(state), ..Ui::default() }
    }

    /// A button fires on release **over itself**, so a press can be taken back
    /// by sliding off it. `REBUILD` is the reason: a control that threw the box
    /// away on the way past would be one mis-click from losing a run.
    #[test]
    fn a_press_slid_off_its_button_fires_nothing() {
        let s = state(false, 1);
        let mut ui = armed(&s);
        let reset = ui.widget_rect(Action::Reset).expect("REBUILD is on the bar");
        let elsewhere = ui.widget_rect(Action::Stats).expect("STATS is on the bar");
        ui.press(reset.x + 2, reset.y + 2);
        assert_eq!(ui.release(elsewhere.x + 2, elsewhere.y + 2), Release::Consumed);

        // The positive control: the same press released where it started does
        // fire, so the test above is about the slide and not about the bar
        // being dead.
        let mut ui = armed(&s);
        ui.press(reset.x + 2, reset.y + 2);
        assert_eq!(ui.release(reset.x + 2, reset.y + 2), Release::Fired(Action::Reset));
    }

    /// A click on the interface must not also reach the world behind it —
    /// including a press that began on the bar and ended over the box, and one
    /// that began on an open page.
    #[test]
    fn the_interface_does_not_leak_clicks_into_the_world() {
        let s = state(false, 1);
        let mut ui = armed(&s);
        // Straight at the world.
        ui.press(20, 20);
        assert_eq!(ui.release(20, 20), Release::World);

        // Begun on the bar's own background, between two buttons.
        let mut ui = armed(&s);
        ui.press(1, bar_top() + 1);
        assert_eq!(ui.release(20, 20), Release::Consumed);

        // Begun on an open page. The page's rectangle is retained from the
        // last paint, which is what a real click is tested against.
        let mut ui = armed(&s);
        ui.panel_box = Some(Rect { x: 100, y: 100, w: 80, h: 40 });
        ui.press(120, 120);
        assert_eq!(ui.release(20, 20), Release::Consumed);
        assert!(ui.covers(120, 120));
        assert!(!ui.covers(20, 20));
    }

    /// The inspector is a toggle on the cell, not an accumulator: clicking the
    /// same cell twice puts it away, and clicking a different one moves it.
    #[test]
    fn the_inspector_toggles_on_the_cell_it_is_pointed_at() {
        let mut ui = Ui::new();
        ui.inspect((10, 20));
        assert_eq!(ui.inspecting(), Some((10, 20)));
        ui.inspect((11, 20));
        assert_eq!(ui.inspecting(), Some((11, 20)));
        ui.inspect((11, 20));
        assert_eq!(ui.inspecting(), None);
    }

    /// Every page row that names a quantity carries its own explanation. The
    /// owner asked for this directly, and a row that quietly lost its note
    /// would look identical on screen until somebody hovered it.
    #[test]
    fn every_page_row_explains_itself() {
        let (world, spec, ui) = (world(), LabBox::default(), Ui::new());
        for panel in [Panel::Plants, Panel::Ants, Panel::Box] {
            for row in ui.panel_rows(panel, &world, &spec, 60.0) {
                if matches!(row.body, Body::Gap) {
                    continue;
                }
                assert!(row.note.len() > 30, "{panel:?} has a row with no real explanation");
            }
        }
        for row in ui.inspect_rows(&world, (0, 0)) {
            assert!(row.note.len() > 30, "an inspector row has no real explanation");
        }
        // ...and so does every button, since the bar is where a new player
        // looks first.
        for wid in layout(&state(false, 1)).widgets {
            assert!(wid.note.len() > 30, "{:?} has no explanation", wid.line1);
        }
    }

    /// A page is sized to what it has to say, and has to stay clear of the bar
    /// it opens above.
    #[test]
    fn a_page_sits_above_the_bar_and_inside_the_screen() {
        let (world, spec, ui) = (world(), LabBox::default(), Ui::new());
        for panel in [Panel::Plants, Panel::Ants, Panel::Box] {
            let rows = ui.panel_rows(panel, &world, &spec, 60.0);
            for anchor in [0, 200, W as i32 - 10] {
                let r = page_rect(&rows, anchor, bar_top() - 4);
                assert!(r.x >= 0 && r.right() <= W as i32, "{panel:?} page runs off the screen");
                assert!(r.bottom() <= bar_top(), "{panel:?} page overlaps the bar");
                assert!(r.y >= 0, "{panel:?} page runs off the top");
            }
        }
    }

    /// The population series is sampled on simulated frames, never on drawn
    /// ones: at the top of the ladder one drawn frame is 256 ticks, and a
    /// per-call sample would make the strip's x-axis the speed dial.
    #[test]
    fn the_series_is_sampled_on_simulated_time_not_on_draws() {
        let mut history = History::default();
        let world = world();
        for _ in 0..500 {
            history.observe(&world);
        }
        assert_eq!(history.samples.len(), 1, "a still world sampled itself repeatedly");
        assert!(history.delta(|s| s.plants as i64).is_none(), "one sample is not a delta");
    }

    /// A rebuild puts the frame counter back to zero, and a series carried
    /// across it would draw a population crash that never happened.
    #[test]
    fn a_rebuild_clears_the_series() {
        let mut history = History::default();
        let mut world = world();
        for _ in 0..4 {
            history.observe(&world);
            world.frame += SAMPLE_EVERY;
        }
        assert_eq!(history.samples.len(), 4);
        world.frame = 0;
        history.observe(&world);
        assert_eq!(history.samples.len(), 1, "the old box survived the rebuild");
    }

    #[test]
    fn a_note_wraps_rather_than_running_off_the_screen() {
        let lines = wrap_words("THE FLORA HOW MANY ARE STANDING AND WHETHER IT IS CLIMBING", 12);
        assert!(lines.len() > 3);
        for line in &lines {
            assert!(line.chars().count() <= 12, "{line:?} did not wrap");
        }
    }
}
