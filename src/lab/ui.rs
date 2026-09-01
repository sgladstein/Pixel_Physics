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

use super::params;
use super::scene::LabBox;
use super::{HEIGHT as H, WIDTH as W};

// --------------------------------------------------------------- geometry

/// How many rows of the screen the bar owns, measured up from the bottom.
///
/// A button holds two 7-pixel rows (its label and its keyboard shortcut) plus
/// padding, which is 24, and the bar needs an edge and a margin around that:
/// one row of buttons is 30.
///
/// **Two rows, and the second one is not free.** 26 more rows of 512 is 13,312
/// pixels painted over terrain that was already painted, and a lab redraw
/// measures a flat 19.2-19.9 ns/px with essentially no fixed cost to amortise
/// (the field lane, 2026-08-30) — so this costs about **0.26 ms of a drawn
/// frame**, every drawn frame, and it does not get cheaper as the bar settles
/// because chrome has no dirty-rect skip. It is spent because the alternative
/// is worse: at one row the seven-stop ladder, six pages and six tools do not
/// fit at any spacing, and a bar that does not fit loses its last control off
/// the right edge where only a screenshot ever finds it.
pub const BAR_HEIGHT: i32 = 56;
const BTN_TOP: i32 = 3;
const BTN_HEIGHT: i32 = 24;
/// Between the two rows of buttons.
const ROW_GAP: i32 = 2;
/// How many rows of buttons the bar has. Row 0 is the tools — what a click on
/// the world does — and row 1 is the transport, the speed ladder and the
/// pages. **The tools sit against the world and the transport against the
/// screen edge**, which is the order they are reached in: you pick a tool and
/// then use it up there, and you set the clock once and leave it.
const ROWS: usize = 2;
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
/// a click there is the active tool rather than a press.
pub fn bar_top() -> i32 {
    H as i32 - BAR_HEIGHT
}

/// The top of one row of buttons.
fn row_y(row: usize) -> i32 {
    bar_top() + BTN_TOP + row as i32 * (BTN_HEIGHT + ROW_GAP)
}

// ------------------------------------------------------------------ tabs

// **The rack's tab strip sits against the top edge of the bar**, and the
// reasoning that put it there is worth keeping because it is measured.
//
// **The bar cannot simply take another cell.** Measured with the bar's own
// `PIXEL_PHYSICS_BAR_TRACE` on the shipped layout: row 0 has **1 pixel**
// spare and row 1 has **0**, at the tightest of the three spacings `layout`
// tries — the bar is already running compressed to fit what is on it, so
// five tabs at ~34 px do not go in it at any spacing. A tab is therefore a
// strip *beside* the bar, and the only question was which edge it eats.
//
// **Three placements were built and rendered rather than argued** —
// `CLAUDE.md`: *for "does this look right", ship a runtime selector rather
// than choosing* — and the owner chose this one by eye, 2026-08-31. The
// other two are in `Reports/dead-ends.md` with the defect that sank the
// nearest rival: a strip at the screen's top edge covers the ceiling and the
// grow-light fixtures, which is where you look to see what is lit, and it
// paints over the first line of the `PAUSED` readout.
//
// This edge is the one with least in it: the bed's stone floor already sits
// flush with the bar by `scene.rs`'s own arithmetic.

/// Height of the tab strip, including its rule.
///
/// One glyph plus two pixels either side plus the rule. A tab is a number, so
/// this is as short as a legible strip gets.
pub const TAB_H: i32 = hud::GLYPH_HEIGHT + 5;

/// How many tabs the strip shows before it stops.
///
/// The owner's call: *"numbered tabs on the bar (for your top 5)"*, with the
/// whole rack behind its own page. A strip that grew with the rack would be
/// the control that breaks at the fiftieth chamber, which is exactly the size
/// a batch produces.
pub const TABS_SHOWN: usize = 5;

/// The strip's top row. Always drawn.
///
/// **It was hidden below two chambers and that was a circular bug**, reported
/// by the owner as *"how do i access the rack, I don't see it in the menu"*.
/// The reasoning was that a facility with one box has no switching to offer —
/// true, and irrelevant, because the strip does not only *switch* between
/// chambers, it carries `ALL`, which is the only way to reach the page where
/// chambers are **made**. So the way to get a second chamber was inside the
/// page you could only open once you already had two, and the lab opens on
/// one.
///
/// `F4` was the sole remaining route and was not on the key list either. Both
/// halves are fixed; this is the half that matters, because a control nobody
/// can find is a control that does not exist.
fn tab_strip_y(_chambers: usize) -> i32 {
    bar_top() - TAB_H
}

/// Lay the tabs out, left to right, and say where each one is.
///
/// Widths come from the label rather than a fixed cell, so a two-digit
/// chamber is not clipped — a rack reaches double figures the first time a
/// batch finishes.
fn lay_out_tabs(chambers: &[super::ChamberSummary], y: i32) -> Bar {
    let mut widgets = Vec::new();
    let mut x = MARGIN;
    for ch in chambers.iter().take(TABS_SHOWN) {
        let w = hud::text_width(&ch.label) + 8;
        widgets.push(Widget {
            rect: Rect { x, y: y + 1, w, h: TAB_H - 2 },
            line1: ch.label.clone(),
            line2: String::new(),
            action: Some(Action::Chamber(ch.index)),
            latched: ch.active,
            icon: None,
            ratio: None,
            note: String::new(),
        });
        x += w + GAP;
    }
    // **The way into the whole rack, and it lives here because the bar has
    // nowhere to put it** — 0-1 px of slack, as above. The strip has room to
    // spare at five tabs, so the page that holds the other forty-five opens
    // from the strip that admits it is only showing five.
    //
    // It carries the count of what is hidden rather than a bare label: a
    // strip that silently stopped at five would tell you your rack is five
    // chambers long. And it is a *verb* rather than the `+N` label it
    // replaced — `CLAUDE.md`'s second law, a control that only informs is a
    // control the player is a spectator of.
    let hidden = chambers.len().saturating_sub(TABS_SHOWN);
    let label = if hidden > 0 { format!("ALL +{hidden}") } else { "ALL".to_string() };
    let w = hud::text_width(&label) + 8;
    widgets.push(Widget {
        rect: Rect { x, y: y + 1, w, h: TAB_H - 2 },
        line1: label,
        line2: String::new(),
        action: Some(Action::Panel(Panel::Chambers)),
        latched: false,
        icon: None,
        ratio: None,
        note: "EVERY CHAMBER, WITH WHAT IS ALIVE IN IT AND HOW DEEP ITS GENERATIONS GOT. THE TABS REACH THE FIRST FIVE; THIS IS HOW A RACK OF FIFTY IS READ.".into(),
    });
    Bar { widgets, dividers: Vec::new() }
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
/// The ring under the cursor that says what the active tool will cover.
/// Bright and cool, so it reads against soil, water and the lit sky alike.
const TOOL_RING: [u8; 4] = [150, 226, 255, 255];

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

/// The same call under a name that does not collide with a local `text`
/// binding. `draw` already binds `text` from `paint_page`'s return.
fn text_at(frame: &mut [u8], x: i32, y: i32, s: &str, colour: [u8; 4]) {
    text(frame, x, y, s, colour);
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
    /// Choose what a left-click on the world does. See [`Tool`].
    Tool(Tool),
    /// Cycle which species the planting tool puts in.
    NextSpecies,
    /// Widen or narrow the brush, by `+1` or `-1` steps.
    Brush(i32),
    /// Cycle the false-colour view of the invisible channels.
    CycleOverlay,
    Panel(Panel),
    Stats,
    Help,
    Reset,
    /// Show one page of the parameters panel — an index into
    /// [`params::GROUPS`].
    ParamGroup(usize),
    /// Scroll the parameters panel by one page, `-1` or `+1`.
    ParamScroll(i32),
    /// Move one parameter by one of its own steps. The index is into the
    /// **current page's** list, which is what was on screen when the click
    /// landed; the sign is which way.
    ParamAdjust(usize, i32),
    /// Highlight one parameter row, so `SAVE` knows which one it means.
    ParamSelect(usize),
    /// Write the highlighted parameter back to its asset file.
    ParamSave,
    /// **Show one group of the cell page's specimen rows**, closing whichever
    /// was open. The index is into `params::specimen_sections`' own order,
    /// which is fixed and the same for every species.
    SpecimenSection(usize),
    /// **Put the individual the cell page is open on into a jar.** The
    /// button that replaced the `KEEP` tool: the page is already pointed at
    /// one organism, so the second click the tool needed was a click at
    /// something the interface already knew.
    KeepInspected,
    /// **Arm the armed jar for placing**, and get the rack out of the way so
    /// there is a box to place it in. The button that replaced the `FREE`
    /// tool.
    ShelfPlace,
    /// Arm one jar on the shelf — an index into the loaded rack.
    ShelfSelect(usize),
    /// Turn the brood dial by `-1` or `+1`.
    Broods(i32),
    /// Put a drifted copy of the armed jar on the shelf, at the current
    /// dial. The shelf's own breeding verb: it keeps the variant rather
    /// than only releasing it.
    ShelfDrift,
    /// Take the armed jar off the shelf for good.
    ShelfDiscard,
    /// Write the armed jar out as a full species `.ron` — the way out of
    /// the lab and into the game. See `sim::species_export`.
    ShelfPromote,
    /// Re-read the shelf directory, for a jar added or removed outside the
    /// running game.
    ShelfReload,
    /// Put one chamber of the rack on screen — an index into
    /// `Lab::chamber_summaries`.
    Chamber(usize),
    /// Highlight one row of the rack page, which is also what asks for its
    /// picture. Separate from [`Action::Chamber`] because looking at a
    /// chamber and walking into one are different decisions: a rack is read
    /// by comparing rows, and a click that switched on contact would run the
    /// box you were only inspecting.
    ChamberSelect(usize),
    /// Add a chamber: the box on screen again, at the next unused seed.
    ChamberAdd,
    /// Close the highlighted chamber. Refused for the one on screen.
    ChamberClose(usize),
    /// Re-run an on-record row back into a world. See
    /// `Lab::rebuild_record`.
    ChamberRebuild(usize),
    /// Throw away every chamber and record except the one on screen.
    ChamberClear,
    /// Sort the rack on one column. Clicking the column already sorted
    /// reverses it; there is no third state, because a click that cycled
    /// through *unsorted* would take two more clicks to get back to the
    /// order you wanted.
    ChamberSort(usize),
    /// Run a rack of copies of the box on screen, headless, in the
    /// background.
    BatchRun,
    /// Ask a running rack to stop. Copies already finished are kept.
    BatchStop,
    /// How many copies the next rack runs, by `-1` or `+1` steps of its own.
    BatchCopies(i32),
    /// How many ticks each copy runs for, same.
    BatchFrames(i32),
}

/// **What a left-click on the world does.**
///
/// The design guide's Gate 4 verbs, plus the two the owner asked for
/// separately. `Reports/evolution-lab-design-guide-2026-08-30.md` names `cull`
/// and `partition` as *"the only two with no engine support and the two the
/// premise most depends on"*, and puts **selection only** in the opening — so
/// the opening's whole lever is `cull`, and until now it did not exist.
///
/// **The keys are positional, not mnemonic, and that is deliberate.** The
/// obvious initials are gone: `S` is pan-down, `W` is pan-up, `P`... would be
/// free but `C`/`S`/`W` are not, so a mnemonic set would be four near-misses
/// and two collisions. `Z X C V B N` is one unbroken run of the keyboard's
/// bottom row in **the same left-to-right order the buttons appear in**, which
/// is a thing you can learn by looking at the bar once. Every button prints
/// its own key under it either way — the bar is how you find a control and the
/// key is how you use it once you have.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tool {
    /// Point at a cell and read it. The default, and the only one that changes
    /// nothing.
    #[default]
    Look,
    /// Put a seed of the selected species in the soil.
    Plant,
    /// Release a colony of founders.
    Colony,
    /// Kill the organism under the cursor — marked senescent, so it rots down
    /// rather than vanishing.
    Cull,
    /// Paint soil, at field capacity.
    Soil,
    /// Paint water, full.
    Water,
    /// **Put the armed jar back in the box**, as itself or drifted by the
    /// shelf's brood dial.
    ///
    /// **The one tool that is not on the bar**, and it is deliberate. The
    /// pair this used to belong to — `KEEP` and `FREE` — cost two of the
    /// eight tool buttons to say what two page buttons say better, which is
    /// the owner's reading of it: *"I feel like we don't need the keep and
    /// free buttons... this will save some menu space."* Keeping is now a
    /// button on the cell page, where you are already looking at the
    /// individual you want, and placing is armed by `PLACE` on the shelf,
    /// where you are already looking at the jar. What is left over is the
    /// *aiming*, which genuinely is a world click and genuinely is a mode,
    /// so the mode survives and only its button is gone. The jar chip on the
    /// bar latches while it is armed, so it is still visible.
    Release,
    /// Drop a wall, floor to ceiling, in the column you click. Click a wall
    /// you placed to take it out again.
    Wall,
}

/// Every tool **that has a cell on the bar**, in bar order. One list, so the
/// row, the key table and the tests cannot disagree about what exists.
///
/// **Two of them are deliberately not in it, for opposite reasons.**
/// [`Tool::Release`] came off because its verb moved to the page that already
/// knows what it means (`PLACE`, on the rack), which is the owner's own
/// ruling; see that variant. [`Tool::Wall`] was never on it, because the bar
/// was measured **full** when the wall verb landed -- 1 px of slack on row 0
/// and 0 on row 1 at the tightest of the three spacings `layout` tries, with
/// `the_bar_fits_the_screen_and_no_two_widgets_overlap` saying so immediately
/// when a ninth cell was tried.
///
/// **That constraint has since gone, and giving the wall a face is still the
/// owner's call rather than this file's.** Dropping `KEEP` and `FREE` freed
/// two cells, so there is now room; but which verb earns a bar cell, and
/// whether the wall's key moves onto the run when it gets one, are
/// proportions the owner chose by eye. Flagged rather than forced -- squeezing
/// a control in is how the overlapping columns on the rack page happened.
pub const TOOLS: [Tool; 6] = [Tool::Look, Tool::Plant, Tool::Colony, Tool::Cull, Tool::Soil, Tool::Water];

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Look => "LOOK",
            Tool::Plant => "PLANT",
            Tool::Colony => "COLONY",
            Tool::Cull => "CULL",
            Tool::Soil => "SOIL",
            Tool::Water => "WATER",
            Tool::Wall => "WALL",
            // Never drawn on the bar -- this is what the notice says while it
            // is armed, so it is the verb rather than the old `FREE`: what it
            // does now is put the jar you picked *somewhere*.
            Tool::Release => "PLACE",
        }
    }
    /// The key printed under a tool's button. Never called for
    /// [`Tool::Release`], which has no button; its key is bound directly in
    /// `bin/lab.rs` to the action that arms it.
    fn key(self) -> &'static str {
        match self {
            Tool::Look => "Z",
            Tool::Plant => "X",
            Tool::Colony => "C",
            Tool::Cull => "V",
            Tool::Soil => "B",
            Tool::Water => "N",
            Tool::Release => ",",
            // **Off the run, because it is off the bar.** The positional rule
            // is "one unbroken run in *bar* order", and neither of these has
            // a bar cell — see `TOOLS` for why each. Taking the next key in
            // the run would have moved `NextSpecies` off `.` for a control
            // that is not in the row the rule is about, and the wall's `K` is
            // a key players have already been given.
            Tool::Wall => "K",
        }
    }
    /// Whether this tool paints continuously while the button is held. The
    /// verbs are one-shot — a drag that founded a colony per pixel would empty
    /// the organism table in one gesture.
    pub fn is_brush(self) -> bool {
        matches!(self, Tool::Soil | Tool::Water)
    }
    fn note(self) -> &'static str {
        match self {
            Tool::Look => "POINT AT A CELL AND READ IT. CLICK TO PIN THE CELL PAGE OPEN; CLICK IT AGAIN TO PUT IT AWAY. WHAT IS UNDER THE POINTER IS ALWAYS READ OUT TOP RIGHT, TOOL OR NO TOOL.",
            Tool::Plant => "PUT ONE SEED IN THE SOIL WHERE YOU CLICK. THE CHIP TO THE RIGHT SAYS WHICH SPECIES AND WHAT IT COSTS TO GROW ONE. A SEED NEEDS BARE SOIL WITH ROOM ABOVE IT.",
            Tool::Colony => "RELEASE A COLONY OF FOUNDERS AT THE SURFACE UNDER THE CLICK. THEY ARRIVE WITH A PATCH OF NEST TO WALK HOME TO -- WITHOUT ONE THERE IS NO GRADIENT AND NOBODY FORAGES.",
            Tool::Cull => "KILL THE ORGANISM YOU CLICK. IT IS MARKED SENESCENT, NOT DELETED, SO IT ROTS DOWN OVER ITS SPECIES HALF-LIFE AND FEEDS WHATEVER IS STILL ALIVE. THIS IS THE SELECTION LEVER: WHAT YOU CULL DOES NOT BREED.",
            Tool::Soil => "PAINT SOIL, AT FIELD CAPACITY -- DAMP ENOUGH FOR A ROOT, NOT SO WET IT SLUMPS. IT WILL NOT PAINT OVER STONE OR OVER A LIVING PLANT.",
            Tool::Water => "PAINT WATER, FULL. IT RUNS, IT SOAKS INTO SOIL, AND TOO MUCH OF IT DROWNS ROOTS -- WHICH IS AN EXPERIMENT, NOT A MISTAKE.",
            Tool::Wall => "DROP A WALL FLOOR TO CEILING IN THE COLUMN YOU CLICK, OR CLICK ONE YOU PLACED TO TAKE IT OUT. A WALL IS WHAT MAKES TWO POPULATIONS IN ONE BOX INTO TWO POPULATIONS: THEY CANNOT MIX, SO THEY CAN DRIFT APART. IT CUTS WHATEVER IS IN THE WAY, WHICH IS THE POINT -- A WALL THROUGH A STAND IS A STAND SPLIT IN HALF. IT SURVIVES A REBUILD.",
            Tool::Release => "PUT THE ARMED JAR BACK IN THE BOX WHERE YOU CLICK. AT 0 BROODS IT IS THAT EXACT INDIVIDUAL AGAIN; AT 1 IT IS AS DIFFERENT AS ITS OWN CHILD WOULD HAVE BEEN, AND SO ON UP. OPEN THE SHELF WITH G TO PICK A JAR AND SET THE DIAL.",
        }
    }
}

/// Which info panel a button opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Plants,
    Ants,
    Box,
    /// **The numbers behind the verbs.** Drawn by [`Ui::paint_params`] rather
    /// than through [`Ui::panel_rows`], because its rows are not labels: each
    /// one carries two buttons and a range, and the page carries a tab strip,
    /// a pager and a save. It is a [`Panel`] all the same so that it obeys the
    /// one-page-at-a-time rule in `Lab::act` and latches its own button, which
    /// are the two things a second mechanism would have had to reimplement.
    Params,
    /// **The rack of kept genetics.** Draws itself for `Params`' reason —
    /// its rows are jars with a verb attached, not labels — and is a
    /// [`Panel`] for the same two: the one-page-at-a-time rule and the
    /// latch on the bar.
    Shelf,
    /// **The rack: every chamber, with the numbers you would compare them
    /// by.** Draws itself for `Params`' and `Shelf`'s reason — a row is a
    /// chamber with two verbs attached, not a label — and the tabs above the
    /// bar only reach the first five, so this is the page a rack of fifty is
    /// read through.
    Chambers,
}

impl Panel {
    fn title(self) -> &'static str {
        match self {
            Panel::Plants => "PLANTS",
            Panel::Ants => "ANTS",
            Panel::Box => "THE BOX",
            Panel::Params => "PARAMETERS",
            Panel::Shelf => "THE SHELF",
            Panel::Chambers => "THE RACK",
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
    /// `(x, row)` — a group separator, and which row's band it is drawn in.
    /// The row is carried rather than derived, because a divider drawn the
    /// full height of a two-row bar reads as a column rule across controls
    /// that have nothing to do with each other.
    dividers: Vec<(i32, usize)>,
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
pub struct BarState<'a> {
    pub running: bool,
    pub requested: u32,
    pub achieved: f32,
    pub presets: &'static [u32],
    pub panel: Option<Panel>,
    pub stats: bool,
    pub help: bool,
    /// What a left-click on the world does.
    pub tool: Tool,
    /// The species the planting tool will put in, and one line about it.
    /// Borrowed from the world's species table rather than copied, so a chip
    /// naming a species cannot name one that is not loaded.
    pub species: &'a str,
    pub species_note: &'a str,
    /// Brush radius in cells, for the two painting tools.
    pub brush: i32,
    /// The active false-colour channel, already named.
    pub overlay: &'static str,
    /// **What `RELEASE` will put in**: the armed jar's name, or how many
    /// jars are on the shelf when none is armed. Owned by the caller for the
    /// species chip's reason — a chip naming a jar that is not on the rack
    /// would be the stale side table that argument is about.
    pub jar: &'a str,
    pub jar_note: &'a str,
    /// One row per chamber, in rack order. Borrowed for the species chip's
    /// reason: a tab strip that named a chamber from its own copy would be
    /// the stale side table that argument is about.
    pub chambers: &'a [super::ChamberSummary],
    /// How many copies and how many ticks the next rack will use, and how a
    /// running one is getting on. `None` when nothing is running.
    pub batch: super::BatchBar,
    /// The highlighted chamber's picture, if one has been taken.
    ///
    /// Rendered by `Lab` rather than here, and only when a row is clicked:
    /// `Renderer::draw` needs `&mut` and this page has it borrowed shared,
    /// but more to the point a picture per frame of a box that is *frozen*
    /// would be the same picture, repainted sixty times a second.
    pub rack_thumb: Option<&'a super::Thumb>,
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
    /// Owned rather than `&'static str`, because the species chip's note is
    /// the species' own line read out of the loaded table — and a chip that
    /// named the species while explaining a *different* one would be the
    /// stale-side-table failure this field exists to avoid.
    note: String,
}

fn button(
    label: &str,
    sub: &str,
    action: Action,
    latched: bool,
    note: &str,
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
        note: note.to_string(),
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
/// The middle rung is not decoration: at seven stops the bar misses the
/// natural spacing by four pixels, and closing the *gaps between buttons* to
/// one pixel while keeping two pixels of padding inside each face is far more
/// readable than the reverse.
const SPACINGS: [(i32, i32); 3] = [(PAD, GAP), (PAD, 1), (1, 1)];
/// The smallest gap between two groups that still reads as a gap. A bar packed
/// tighter than this is one row of undifferentiated buttons.
const MIN_SEPARATOR: i32 = 6;

/// Lay the whole bar out. Pure: same state in, same rectangles out.
///
/// **Widths are measured, never written down.** Every label goes through
/// `hud::text_width`, so renaming a button cannot silently leave its face
/// narrower than its own text — which is the failure a hand-tuned pixel table
/// produces and only a screenshot catches.
pub fn layout(state: &BarState<'_>) -> Bar {
    for (pad, gap) in SPACINGS {
        let bar = lay_out(state, pad, gap);
        if std::env::var("PIXEL_PHYSICS_BAR_TRACE").is_ok() {
            // **Per row, because the bar has two and only one of them is
            // tight.** This printed the last widget's right edge alone, which
            // is row 1's — so a row 0 that had overflowed would have been
            // invisible here, and where to put a new button is exactly the
            // question this trace is read to answer.
            let limit = W as i32 - MARGIN;
            for ri in 0..ROWS {
                let y = row_y(ri);
                let right = bar.widgets.iter().filter(|w| w.rect.y == y).map(|w| w.rect.right()).max().unwrap_or(0);
                eprintln!("bar trace: pad={pad} gap={gap} row={ri} right={right} of {limit} slack={}", limit - right);
            }
            eprintln!("bar trace: pad={pad} gap={gap} fits={}", bar.fits());
        }
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

fn lay_out(state: &BarState<'_>, pad: i32, gap: i32) -> Bar {
    // Group 1 — transport. The phase button's face is sized to the *wider* of
    // its two captions so that pressing it does not shove the rest of the bar
    // sideways; a control that moves when you use it is a control you miss on
    // the second press.
    // `STOP` rather than `PAUSE`, and the four characters are not the reason
    // — though they are why the bar still fits an eight-stop ladder. The rule
    // this bar keeps is *verb on the button, state on the readout*, and `STOP`
    // is the verb the press performs where `PAUSE` is halfway to being the
    // state the readout beside it already owns.
    let phase_label = if state.running { "STOP" } else { "RUN" };
    let phase_icon = if state.running { Icon::Pause } else { Icon::Play };
    let phase_px = ICON_W
        + ICON_GAP
        + hud::text_width("STOP").max(hud::text_width("RUN"));
    // **The caption says what the press will produce, not what is true now.**
    // This is the single easiest thing in the whole bar to get backwards: on a
    // stopped box the button reads `RUN` because clicking it starts the run,
    // and the readout two cells to its right says `PAUSED`, which is the
    // state. Verb on the button, state on the readout.
    let phase = Spec {
        width: cell_width(phase_px, "SPACE", pad),
        line1: phase_label.to_string(),
        line2: "SPACE".to_string(),
        action: Some(Action::TogglePhase),
        latched: state.running,
        icon: Some(phase_icon),
        ratio: None,
        note: "STOP THE BOX DEAD, OR START IT AGAIN. PAUSED, NOTHING TICKS AT ALL -- THAT IS WHEN YOU PLANT, CULL AND DIG. RUNNING GOES AT WHATEVER THE DIAL ASKS FOR, AND 1X IS REAL TIME.".to_string(),
    };
    // `DN`/`UP` rather than `DOWN`/`UP`: the caption was the wider of the two
    // lines and so set the face width, and two arrow buttons 27 pixels wide
    // cost more of a 512-pixel bar than the extra two letters are worth. Side
    // by side under `<<` and `>>` they read as the arrow keys, and the key
    // page spells them out in full.
    let step_width = cell_width(hud::text_width("<<"), "DN", pad)
        .max(cell_width(hud::text_width(">>"), "UP", pad));
    let slower = Spec {
        width: step_width,
        ..button("<<", "DN", Action::Slower, false, "ONE STOP DOWN THE SPEED LADDER.", pad)
    };
    let faster = Spec {
        width: step_width,
        ..button(">>", "UP", Action::Faster, false, "ONE STOP UP THE SPEED LADDER. ASKING FOR SPEED FROM A STOPPED BOX ALSO STARTS THE RUN.", pad)
    };
    // **The achieved figure is only shown while it means something**, and
    // stopped it means nothing at all: no tick runs, so the ratio is zero by
    // construction rather than by measurement. What the cell says instead is
    // the *state*, in the one word the owner's complaint asked for -- `PAUSED`
    // over `NOT TICKING`, graded at 0.0 so it draws in the alarm colour with
    // an empty strip under it. Three channels saying one thing, because
    // "it isn't pausing anything" was a complaint about legibility as much as
    // about behaviour.
    let (line1, line2, ratio) = if state.running {
        (
            format!("ASK {}X", state.requested),
            // One decimal below 100x and none above, so the line is never
            // more than nine characters and the face never has to grow for a
            // fast box.
            if state.achieved < 100.0 {
                format!("GOT {:.1}X", state.achieved.max(0.0))
            } else {
                format!("GOT {:.0}X", state.achieved)
            },
            state.achieved.max(0.0) / state.requested.max(1) as f32,
        )
    } else {
        ("PAUSED".to_string(), "NO TICKS".to_string(), 0.0)
    };
    let readout = Spec {
        // Sized to the widest thing it can ever say, not to what it says now:
        // a readout that changes width as the number changes shoves the bar
        // sideways once a second.
        width: hud::text_width("GOT 99.9X").max(hud::text_width("REAL TIME")) + pad * 2 + 2,
        line1,
        line2,
        action: None,
        latched: false,
        icon: None,
        ratio: Some(ratio),
        note: "WHAT THE DIAL WAS ASKED FOR, AGAINST WHAT THE BOX ACTUALLY MANAGED, AND THE STRIP IS THE SECOND OVER THE FIRST. A BED THAT HAS GROWN COSTS WHAT IT COSTS AND THE DIAL IS ONLY A REQUEST. STOPPED, IT SAYS PAUSED -- NO TICK RUNS, SO THERE IS NOTHING TO REPORT.".to_string(),
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
                note: "RUN AT THIS MULTIPLE OF REAL TIME, STARTING THE BOX IF IT IS STOPPED. 1X IS REAL TIME. THE TOP OF THE LADDER IS DELIBERATELY PAST WHAT ANY BOX CAN DO -- THAT IS HOW THE ACHIEVED READOUT EARNS ITS KEEP.".to_string(),
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
            "RESET",
            "R",
            Action::Reset,
            false,
            "TEAR THE BOX DOWN AND BUILD THE SAME ONE AGAIN FROM ITS SPEC. THE VIEW AND THE DIAL ARE KEPT; EVERYTHING LIVING IN IT IS NOT.",
            pad,
        ),
    ];

    // ---- row 0, the tools. What a click on the world does, and what it
    // does it with. Against the world rather than against the screen edge,
    // because this is the row you reach for while you are working up there.
    let tools: Vec<Spec> = TOOLS
        .iter()
        .map(|t| button(t.label(), t.key(), Action::Tool(*t), state.tool == *t, t.note(), pad))
        .collect();

    // The species chip. **Not decoration**: the design guide is explicit that
    // planting has to show what you are about to plant *"or planting is a slot
    // machine"*, and the face is that. Sized to the widest species the table
    // can hold rather than to the current one, so cycling it does not shove
    // the row sideways.
    let species_px = species_face_px().max(hud::text_width(state.species));
    let species = Spec {
        width: cell_width(species_px, ";", pad),
        line1: state.species.to_string(),
        line2: ";".to_string(),
        action: Some(Action::NextSpecies),
        latched: state.tool == Tool::Plant,
        icon: None,
        ratio: None,
        note: state.species_note.to_string(),
    };

    // The brush, for the two painting tools: narrower, the radius, wider.
    let step_px = cell_width(hud::text_width("W"), "]", pad);
    let narrower = Spec {
        width: step_px,
        ..button("-", "[", Action::Brush(-1), false, "A NARROWER BRUSH. THE RADIUS IS IN CELLS, SO R1 IS A THREE-CELL DAB AND R16 IS A SPADEFUL.", pad)
    };
    let wider = Spec {
        width: step_px,
        ..button("+", "]", Action::Brush(1), false, "A WIDER BRUSH. THE RADIUS IS IN CELLS, AND THE COST OF A STROKE GOES UP WITH ITS AREA, NOT ITS LENGTH.", pad)
    };
    let size = Spec {
        // Sized to `R64`, the widest it can say. See the readout.
        width: cell_width(hud::text_width("R64"), "SIZE", pad),
        line1: format!("R{}", state.brush),
        line2: "SIZE".to_string(),
        action: None,
        latched: false,
        icon: None,
        ratio: None,
        note: "HOW WIDE THE SOIL AND WATER BRUSHES ARE, IN CELLS.".to_string(),
    };

    // The overlay. **Its face is the channel**, because an unlabelled
    // false-colour view is unreadable — `CLAUDE.md` records a canopy-density
    // sheet that read as blank and was misdiagnosed as dead code.
    let overlay = Spec {
        width: cell_width(overlay_face_px(), "O", pad),
        line1: state.overlay.to_string(),
        line2: "O".to_string(),
        action: Some(Action::CycleOverlay),
        latched: state.overlay != "OFF",
        icon: None,
        ratio: None,
        note: "FALSE-COLOUR THE INVISIBLE CHANNELS: PRESSURE, TEMPERATURE, LIGHT, AIR HUMIDITY, AND THE TWO PHEROMONES. AIR HUMIDITY IS THE AIR, NOT THE GROUND -- SOIL WATER IS AN ORGANISM OVERLAY AND IT IS THE ONE A ROOT DRINKS. PHEROMONE IS THE ONE TO WATCH -- IT IS AT FULL CELL RESOLUTION AND IT IS THE COLONY'S OWN MAP OF ITSELF, SO YOU SEE THE TRAIL BEFORE YOU SEE THE ANT.".to_string(),
    };

    // **The numbers behind all of the above.** On row 0 rather than with the
    // pages on row 1, and that is a measurement rather than a preference: at
    // the seven-stop ladder row 1 fits at exactly its own width, so one more
    // button there loses `REBUILD` off the right edge (`PIXEL_PHYSICS_BAR_
    // TRACE=1` prints both rows). Row 0 has room, and it is not a bad home —
    // the overlay beside it is not a tool either, and row 0 has become "what
    // you are working with and how you see it".
    let params = button(
        "PARAMS",
        "P",
        Action::Panel(Panel::Params),
        state.panel == Some(Panel::Params),
        "THE NUMBERS BEHIND THE VERBS: WHAT SOIL COSTS TO DIG, HOW MUCH SHOOT A PLANT NEEDS BEFORE IT SETS SEED, HOW HARD AN ANT CAN DIG, HOW BRIGHT THE LAMPS ARE. GROUPED IN FOUR PAGES, EACH ROW WITH ITS OWN RANGE, AND EVERY ROW EXPLAINS ITSELF ON HOVER.",
        pad,
    );

    // **The jar chip: what `RELEASE` is about to put in, and the door to the
    // rack.** The species chip's pattern and the species chip's argument —
    // the design guide says planting has to show what you are about to plant
    // *"or planting is a slot machine"*, and that is true with more force
    // here, because two jars are the same two dark cells on screen and differ
    // only in the genome nobody can see. A bare `SHELF` page button was tried
    // first and is worse on both counts: it says nothing about what is armed,
    // and it costs the same width.
    //
    // Its face is the armed jar's name, or the dial when nothing is armed, so
    // the chip is never blank and never lies about what a click will do.
    let jar = Spec {
        width: cell_width(jar_face_px().max(hud::text_width(state.jar)), "G", pad),
        line1: state.jar.to_string(),
        line2: "G".to_string(),
        action: Some(Action::Panel(Panel::Shelf)),
        latched: state.tool == Tool::Release || state.panel == Some(Panel::Shelf),
        icon: None,
        ratio: None,
        note: state.jar_note.to_string(),
    };

    let rows: [Vec<Vec<Spec>>; ROWS] = [
        vec![tools, vec![species, jar], vec![narrower, size, wider], vec![overlay, params]],
        vec![vec![phase, slower, faster, readout], presets, panels.into_iter().collect()],
    ];

    let mut widgets = Vec::new();
    let mut dividers = Vec::new();
    for (ri, groups) in rows.into_iter().enumerate() {
        // Slack goes into the gaps *between* groups rather than at one end, so
        // the groups read as groups and the row stays centred if a label is
        // ever renamed. Per row, because the two rows have different content
        // and a shared separator would make the shorter one look padded.
        let content: i32 = groups
            .iter()
            .map(|g| g.iter().map(|s| s.width).sum::<i32>() + gap * (g.len() as i32 - 1).max(0))
            .sum();
        let gaps = (groups.len() as i32 - 1).max(1);
        let slack = W as i32 - MARGIN * 2 - content;
        let separator = (slack / gaps).max(MIN_SEPARATOR);
        let mut x = MARGIN;
        let y = row_y(ri);
        for (gi, group) in groups.into_iter().enumerate() {
            if gi > 0 {
                dividers.push((x + separator / 2, ri));
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
                    note: spec.note.clone(),
                });
                x += spec.width;
            }
        }
    }
    Bar { widgets, dividers }
}

/// Width of the widest species name the chip has to hold, so cycling the
/// species does not resize the chip and shove the row along under the cursor.
/// Measured off the shipped table rather than written down: a species added to
/// `assets/species/` must not silently overrun its own chip.
fn species_face_px() -> i32 {
    ["SCRAMBLER", "CONIFER", "CREEPER", "SHRUB", "GRASS", "HERB", "MOSS", "TREE"]
        .iter()
        .map(|n| hud::text_width(n))
        .max()
        .unwrap_or(0)
}

/// Width of the widest overlay name. Same reasoning as the species chip's, and
/// the same failure it prevents: a control that changes width when you use it
/// is a control you miss on the second press.
/// Width of the widest thing the jar chip has to hold.
///
/// **A jar name can be 64 characters and the chip is not sized to that.** It
/// is sized to what fits the bar, and a longer name is drawn clipped rather
/// than allowed to shove the row along — the species chip's rule (*"a control
/// that changes width when you use it is a control you miss on the second
/// press"*), applied to a string the player types instead of one the asset
/// table fixes. `NO JARS` and `SHELF (N)` are the two idle faces, so the
/// measured maximum is over those and a representative name.
fn jar_face_px() -> i32 {
    ["NO JARS", "12 JARS", "1 JAR"].iter().map(|n| hud::text_width(n)).max().unwrap_or(0)
}

/// The longest jar name the chip can show, in characters.
///
/// **A jar name may be 64 characters and this is seven**, which is the width
/// of the chip's own idle face (`12 JARS`) and is set by the bar rather than
/// by taste — see [`Tool::Release`] for the measurement. Longer names are
/// clipped rather than allowed to widen the chip: a control that changes
/// width when you use it is a control you miss on the second press, and row 0
/// has no width to give.
///
/// **Nothing is lost by the clip**, because the chip is not where a jar is
/// identified: the rack behind it prints every name in full and the chip's
/// own hover line names the armed one. Seven characters covers the default
/// names outright — a jar is named after its species and numbered up, so
/// `HERB`, `ANT_2` and `CONIFER` all fit whole.
const JAR_FACE_CHARS: usize = 7;

fn overlay_face_px() -> i32 {
    ["OFF", "PRESSURE", "TEMPERATURE", "LIGHT", "AIR HUMIDITY", "PHEROMONE A", "PHEROMONE B"]
        .iter()
        .map(|n| hud::text_width(n))
        .max()
        .unwrap_or(0)
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
    // **A bar button is two stacked lines and a page chip is one**, so the
    // bar's fixed inset would put a chip's only line 4 pixels down a 9-pixel
    // face and hang two rows of it over whatever is under the button. Keyed on
    // the height rather than on `line2` being empty, which is the same test
    // today and would stop being one the moment a bar readout lost its
    // caption: every widget `layout` produces is `BTN_HEIGHT` tall.
    let ty = if r.h < BTN_HEIGHT { r.y + (r.h - hud::GLYPH_HEIGHT) / 2 } else { r.y + 4 };
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
    /// **A named group of rows, and whether it is showing.** Clickable: the
    /// action opens this group and closes whichever was open. `hidden` is how
    /// many rows are behind it while it is shut, which is the whole reason a
    /// fold is honest -- a group that says `+ GENOME 11` has not lost
    /// anything, where a page that quietly stopped at the bottom of the
    /// screen had.
    Head { label: String, open: bool, hidden: usize, action: Action },
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
    fn head(label: &str, open: bool, hidden: usize, action: Action, note: impl Into<String>) -> Self {
        Self {
            body: Body::Head { label: label.into(), open, hidden, action },
            note: note.into(),
        }
    }
    fn height(&self) -> i32 {
        match self.body {
            Body::Value { .. } => LINE,
            // A rule above the label and a pixel of air under it, so a shut
            // group reads as a lid rather than as another value row.
            Body::Head { .. } => LINE + 4,
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
            Body::Head { label, hidden, .. } => hud::text_width(label) + 12 + hud::text_width(&hidden.to_string()) + 8,
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

/// The bar's short population strip for **one** chamber.
///
/// `pub` and parked in `Chamber` rather than kept on `Ui`, because a strip
/// that follows the viewer instead of the box draws chamber B's population
/// over chamber A's bed. `observe` below already recognises this failure in
/// its narrow form — a rebuild putting the frame counter back — and parking
/// is the same fix generalised to a switch, where the frame moves *forward*
/// and the reset it relies on therefore never fires.
///
/// Fields stay private: the strip is drawn by `Ui` and read by nobody else.
#[derive(Default)]
pub struct History {
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
    /// What a left-click on the world does. See [`Tool`].
    tool: Tool,
    /// Which plantable species the planting tool puts in — an **index into
    /// the world's own plantable list**, not a name, so a chip can never name
    /// a species that is not loaded. Wrapped at use rather than clamped, so
    /// an asset set with fewer species cannot leave it dangling.
    species: usize,
    /// Radius of the soil and water brushes, in cells.
    brush: i32,
    /// **What the last verb did, and when it said so.**
    ///
    /// `CLAUDE.md`'s second law: *if an event produces no visible consequence
    /// it is not finished regardless of what the simulation believes*. Three
    /// of the tools produce nothing you can see on a **stopped** box — a
    /// culled plant is marked senescent and rots at its species half-life,
    /// which needs ticks; a seed is one cell; a colony's ants are two dark
    /// cells each. Stopped is exactly when you use them. So every verb says
    /// what it did, with the count in it.
    ///
    /// Real time, not frames: a frame-based timer on a paused box never
    /// expires, which is the one state this line exists for.
    notice: Option<(String, std::time::Instant)>,
    /// The world cell the player last clicked, re-read every frame.
    inspect: Option<(i32, i32)>,
    pub(crate) history: History,
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
    /// Which page of the parameters panel is showing — an index into
    /// `params::GROUPS`, wrapped at use so a shorter list can never leave it
    /// dangling (`species`' reason).
    param_group: usize,
    /// First visible row of that page. Pages are short enough that most never
    /// scroll; the two that do are clamped in `paint_params` against the list
    /// as it actually is, not against a remembered length.
    param_scroll: usize,
    /// The row `SAVE` means. Set by adjusting a row or by clicking its name,
    /// so the gesture is *turn the knob, then keep it* rather than a selection
    /// mode you have to be in first.
    param_selected: Option<usize>,
    /// The parameters page's own clickable rectangles, retained for the bar's
    /// reason: a click arrives between frames, so it is tested against the
    /// page the player was looking at. A second `Bar` rather than a second
    /// kind of list, so `hit`, `hovered` and `widget_rect` are the ones
    /// already written.
    params_bar: Bar,
    params_box: Option<Rect>,
    /// **The rack, as it was last read off disk.**
    ///
    /// Held here rather than re-read every frame: the shelf is a directory,
    /// and a `read_dir` per frame is a syscall storm to answer a question
    /// whose answer only changes when the player presses a button. The
    /// buttons that change it reload it; `RELOAD` is on the page for a jar
    /// added from outside the running game.
    shelf: Vec<crate::sim::specimen::Specimen>,
    /// Jar files in the directory that would not parse. Counted rather than
    /// hidden — a shelf that quietly shows four of five jars is worse than
    /// one that says so.
    shelf_skipped: usize,
    /// Which jar the `RELEASE` tool means. An index into `shelf`, cleared
    /// whenever the rack is reloaded, because the row a stored index names
    /// is not the jar it named before.
    shelf_selected: Option<usize>,
    /// **How far to drift a release**, counted in broods: 0 is the same
    /// individual, 1 is as different as its own child would have been. See
    /// `sim::specimen::drift`.
    broods: u32,
    /// The shelf page's own clickable rectangles, for `params_bar`'s reason.
    shelf_bar: Bar,
    /// The rack's tab strip, laid out last frame and retained for the same
    /// reason `bar` is: a click landing between two frames is tested against
    /// the strip the player was actually looking at.
    tabs: Bar,
    /// The rack page's own buttons, retained for `params_bar`'s reason.
    rack_bar: Bar,
    /// Where the rack page was drawn, so a click on it does not also reach
    /// the world behind it.
    rack_box: Option<Rect>,
    /// Which row is highlighted, and therefore which chamber's picture the
    /// page is showing. An index into the rack, **re-clamped every draw**:
    /// closing a chamber renumbers the ones after it, and a selection held
    /// across that would highlight a different box than the one you picked.
    rack_selected: Option<usize>,
    /// First visible row. A batch makes fifty chambers and the page shows
    /// twelve.
    rack_scroll: usize,
    /// Which column the rack is sorted on, and whether it is descending.
    ///
    /// `None` is rack order — the order chambers were made, which is the only
    /// order that carries no opinion. A batch of fifty is read by sorting it;
    /// a rack of three is read as it stands.
    rack_sort: Option<(usize, bool)>,
    shelf_box: Option<Rect>,
    /// The cell page's own clickable rectangles — today the one `KEEP`
    /// button, which is only there while the page is pointed at something
    /// alive. A `Bar` for `params_bar`'s reason.
    inspect_bar: Bar,
    /// **Which specimen group the cell page is showing, while it is folded.**
    /// Kept on the interface rather than per cell, so clicking from one plant
    /// to the next leaves you looking at the same group -- which is what you
    /// want when the reason you are clicking around is to compare them.
    /// Index 1 (`STATE`) is the default: it is the block that moves while the
    /// box runs, and it is what a player opened the page to watch.
    specimen_section: usize,
}

/// Every species that can be planted, in a stable order.
///
/// **Filtered on `creature.is_none()`, not on a written-down list.** The
/// species table holds ants, worms and beetles beside the plants, and a
/// hand-kept list of plant names is the side table that goes stale the first
/// time `assets/species/` gains a file. Sorted by name so the cycle order is
/// the same on every run and in every harness.
pub fn plantable_species(world: &World) -> Vec<crate::sim::organism::SpeciesId> {
    use crate::sim::organism::SpeciesId;
    let mut ids: Vec<SpeciesId> = (0..world.species.len() as u16)
        .map(SpeciesId)
        .filter(|id| world.species.get(*id).creature.is_none())
        .collect();
    ids.sort_by(|a, b| world.species.get(*a).name.cmp(&world.species.get(*b).name));
    ids
}

/// Where the hover readout's top edge sits.
///
/// Clear of the **tallest** thing `time::draw` can put in the left column —
/// six lines at `4 + 10i`, so 66 — rather than of whatever it is showing now.
/// A readout that jumped 30 pixels every time the box was stopped would be
/// harder to read than one sitting slightly low.
const HOVER_TOP: i32 = 68;

/// How long a verb's notice stays up. Long enough to read one line cold,
/// short enough that it is gone before you have finished the next click.
const NOTICE_SECONDS: f32 = 3.0;

impl Ui {
    pub fn new() -> Self {
        Self {
            // A radius of 6, the sandbox's own default: one click is a
            // shovelful rather than a grain, which is what makes the first
            // stroke read as a verb.
            brush: 6,
            // `STATE`, not `LIFE`: the group that moves while the box runs is
            // what a player opened the page to watch, and `Default`'s own 0
            // would land them on three rows that never change.
            specimen_section: 1,
            ..Self::default()
        }
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
            // The strip stands over the world, so without this a click on a
            // tab would switch chamber *and* drop the inspector on whatever
            // cell was behind it.
            || self.tabs.widgets.iter().any(|wid| wid.rect.contains(x, y))
            || self.panel_box.is_some_and(|r| r.contains(x, y))
            || self.params_box.is_some_and(|r| r.contains(x, y))
            || self.shelf_box.is_some_and(|r| r.contains(x, y))
            || self.rack_box.is_some_and(|r| r.contains(x, y))
            || self.inspect_box.is_some_and(|r| r.contains(x, y))
    }

    /// Arm a press. Fires nothing: a button that acted on press could not be
    /// taken back, and a mis-click on `REBUILD` would already have thrown the
    /// box away by the time you noticed.
    pub fn press(&mut self, x: i32, y: i32) {
        self.pressed = self.hit(x, y);
        self.press_inside = self.covers(x, y);
    }

    /// Release, and say what it meant.
    pub fn release(&mut self, x: i32, y: i32) -> Release {
        let armed = self.pressed.take();
        let inside = std::mem::take(&mut self.press_inside);
        if let Some(action) = armed {
            // Still over the button it armed, or the gesture was taken back.
            return if self.hit(x, y) == Some(action) {
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

    /// Where the cell page was drawn last frame — the retained rectangle, so a
    /// harness hovering one of its rows is hovering a row that exists.
    pub fn inspect_rect(&self) -> Option<Rect> {
        self.inspect_box
    }

    /// Show one group of the cell page's specimen rows.
    ///
    /// **Not a toggle**, unlike `set_tool`: the page shows exactly one group
    /// while it is folded, so clicking the open heading again would leave the
    /// page with no group on it at all -- which reads as the page having
    /// broken rather than as having been put away. Clicking the *cell* again
    /// is how the page closes, and that has not changed.
    pub fn show_specimen_section(&mut self, i: usize) {
        self.specimen_section = i;
    }

    /// Which group the cell page is showing.
    pub fn specimen_section(&self) -> usize {
        self.specimen_section
    }

    pub fn toggle_panel(&mut self, panel: Panel) {
        self.panel = if self.panel == Some(panel) { None } else { Some(panel) };
    }

    /// Put whatever page is open away. For a button *on* a page whose verb
    /// needs the box back — `PLACE` is the only one so far.
    pub fn close_panel(&mut self) {
        self.panel = None;
    }

    pub fn tool(&self) -> Tool {
        self.tool
    }

    /// Choose a tool, or put it away by choosing it twice — which lands you
    /// back on `LOOK`, the one tool that changes nothing. A brush you cannot
    /// switch off is a brush that paints the next time you meant to point.
    pub fn set_tool(&mut self, tool: Tool) {
        self.tool = if self.tool == tool { Tool::Look } else { tool };
    }

    /// **Arm a tool, without the toggle.** For a control that *picks
    /// something the tool will use* rather than pressing the tool itself --
    /// the species chip, a jar on the shelf.
    ///
    /// These call sites all used `set_tool`, and that was a bug the owner
    /// reported from play: *"i will click plant, then change the type from
    /// grass to herb and suddenly plant is unselected and now the mouse is
    /// on look."* Choosing a species while `PLANT` is already armed re-pressed
    /// `PLANT`, which is exactly what `set_tool` is written to read as "put it
    /// away". A toggle is right for a button that *is* the tool and wrong for
    /// a chip beside it: picking herb is never a request to stop planting.
    pub fn arm_tool(&mut self, tool: Tool) {
        self.tool = tool;
    }

    pub fn brush(&self) -> i32 {
        self.brush
    }

    /// The radius is clamped to the same 1..=64 the sandbox's brush uses, and
    /// for the same reason: below 1 a brush paints nothing and above 64 one
    /// click is most of a 512-wide bed.
    pub fn adjust_brush(&mut self, delta: i32) {
        self.brush = (self.brush + delta).clamp(1, 64);
    }

    /// Which of `world`'s plantable species is selected, and its name.
    pub fn species_of(&self, world: &World) -> Option<crate::sim::organism::SpeciesId> {
        let plantable = plantable_species(world);
        plantable.get(self.species % plantable.len().max(1)).copied()
    }

    pub fn next_species(&mut self) {
        self.species = self.species.wrapping_add(1);
    }

    /// Say what a verb just did. Displaces whatever the last one said.
    pub fn say(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), std::time::Instant::now()));
    }

    /// Whether anything this module draws needs the frame fully repainted.
    /// Always true today, and stated rather than assumed: a hover highlight
    /// leaves no footprint the dirty-rect skip knows to erase.
    /// **What the last verb said, if it is still saying it.**
    ///
    /// The notice is the lab's whole answer to `CLAUDE.md`'s second law for
    /// verbs whose effect is one or two cells — a kept jar is a file, a
    /// freed ant is two dark pixels, and a refusal is nothing at all. A test
    /// that asserts a verb *worked* without reading this cannot tell a
    /// refusal from a success.
    pub fn notice_text(&self) -> Option<String> {
        self.notice.as_ref().map(|(t, _)| t.clone())
    }

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
        self.bar
            .widgets
            .iter()
            .chain(self.params_bar.widgets.iter())
            .chain(self.shelf_bar.widgets.iter())
            // The strip, the rack page and the cell page belong here for
            // this accessor's whole reason: it is what a harness aims a
            // synthetic click with, so a control missing from it is a control
            // no test and no contact sheet can press. `labui` found this by
            // panicking on `ALL` the first time it was asked to open the rack.
            .chain(self.tabs.widgets.iter())
            .chain(self.rack_bar.widgets.iter())
            .chain(self.inspect_bar.widgets.iter())
            .find(|wid| wid.action == Some(action))
            .map(|wid| wid.rect)
    }

    /// The action under `(x, y)` on the interface as it was last drawn — the
    /// bar first, then the parameters page. They never overlap (a page opens
    /// above `bar_top`), so the order is documentation rather than a rule: the
    /// bar is painted over everything and must win any seam.
    pub fn hit(&self, x: i32, y: i32) -> Option<Action> {
        self.bar
            .hit(x, y)
            .or_else(|| self.params_bar.hit(x, y))
            .or_else(|| self.shelf_bar.hit(x, y))
            .or_else(|| self.rack_bar.hit(x, y))
            .or_else(|| self.tabs.hit(x, y))
            .or_else(|| self.inspect_bar.hit(x, y))
    }

    /// Sort the rack on a column, or reverse it if it is already the one.
    ///
    /// **Descending first.** Every column here is a "how much" — plants,
    /// animals, generations, seeds — and the row worth looking at after a
    /// batch is the biggest one, so one click puts it at the top rather than
    /// at the bottom of fifty.
    pub fn sort_chambers(&mut self, col: usize) {
        self.rack_sort = match self.rack_sort {
            Some((c, desc)) if c == col => Some((c, !desc)),
            _ => Some((col, true)),
        };
        // A sort moves rows under the cursor, so the scroll goes back to the
        // top: staying at row 30 of a list that has just been reordered shows
        // you a window you did not choose.
        self.rack_scroll = 0;
    }

    /// Highlight one row of the rack page.
    pub fn select_chamber(&mut self, i: usize) {
        self.rack_selected = Some(i);
    }

    /// Which row of the rack page is highlighted.
    pub fn selected_chamber(&self) -> Option<usize> {
        self.rack_selected
    }

    /// Which parameters page is showing, and where its list is scrolled to.
    pub fn param_group(&self) -> params::Group {
        params::GROUPS[self.param_group % params::GROUPS.len()]
    }

    /// Show one page. Resets the scroll and the selection, because both are
    /// indices into the page you just left.
    pub fn set_param_group(&mut self, index: usize) {
        self.param_group = index % params::GROUPS.len();
        self.param_scroll = 0;
        self.param_selected = None;
    }

    /// Which line the visible window of the parameters page starts at.
    /// Clamped in `paint_params` against the list as it actually is, so this
    /// is a read of what was drawn rather than of what was asked for.
    pub fn param_scroll(&self) -> usize {
        self.param_scroll
    }

    pub fn scroll_params(&mut self, direction: i32) {
        let page = PARAM_ROWS.saturating_sub(1).max(1);
        self.param_scroll = (self.param_scroll as i32 + direction * page as i32).max(0) as usize;
    }

    pub fn select_param(&mut self, index: usize) {
        self.param_selected = Some(index);
    }

    // ------------------------------------------------------------- the shelf

    /// Re-read the shelf directory, **keeping whatever was armed armed**.
    ///
    /// By *name*, and the distinction is not pedantic — it was a bug. The
    /// first version cleared the selection, on the true observation that an
    /// index is not a name: a rack that gains or loses a jar renumbers every
    /// row after it, and a stored index would then arm a different animal
    /// than the one the player highlighted. But `DRIFT` writes a jar and
    /// reloads, so it disarmed the very jar it had just bred from — and the
    /// next `FREE` said `NO JAR ARMED`, which is a verb doing nothing in
    /// response to a button that had visibly worked a moment earlier.
    ///
    /// Re-finding the name fixes both: the selection survives any change to
    /// the rack that keeps the jar, and is dropped exactly when the jar is
    /// gone, which is the only case where clearing is right. **Found by the
    /// contact sheet's own counter** (`examples/labui.rs`), not by a test:
    /// the tile showed a bed with a plant in it either way and only
    /// `organisms 89 -> 89` said the release had refused.
    pub fn reload_shelf(&mut self) {
        let armed = self.armed_jar().map(|j| j.name.clone());
        let (jars, skipped) = crate::sim::specimen::load();
        self.shelf = jars;
        self.shelf_skipped = skipped.len();
        self.shelf_selected = armed.and_then(|name| self.shelf.iter().position(|j| j.name == name));
    }

    pub fn shelf(&self) -> &[crate::sim::specimen::Specimen] {
        &self.shelf
    }

    /// The armed jar, if one is.
    pub fn armed_jar(&self) -> Option<&crate::sim::specimen::Specimen> {
        self.shelf_selected.and_then(|i| self.shelf.get(i))
    }

    pub fn select_jar(&mut self, index: usize) {
        self.shelf_selected = (index < self.shelf.len()).then_some(index);
    }

    /// **What the jar chip says**, in the two states it has: the armed
    /// jar's name, or how full the rack is when nothing is armed.
    ///
    /// Never blank. A chip that goes empty when nothing is armed reads as a
    /// control that has stopped working, and this one is also the door to
    /// the page that would explain it.
    pub fn jar_face(&self) -> String {
        match self.armed_jar() {
            Some(jar) => jar.name.to_uppercase().chars().take(JAR_FACE_CHARS).collect(),
            None if self.shelf.is_empty() => "NO JARS".to_string(),
            // Singular at one. A chip that says `1 JARS` is a chip nobody
            // proof-read, and this one is on screen whenever the shelf is
            // not empty and nothing is armed.
            None if self.shelf.len() == 1 => "1 JAR".to_string(),
            None => format!("{} JARS", self.shelf.len()),
        }
    }

    /// The chip's hover line: what is armed, how far a release will drift
    /// it, and — when nothing is armed — how to arm one.
    pub fn jar_chip_note(&self) -> String {
        match self.armed_jar() {
            Some(jar) => format!(
                "{} {} -- {} OF SPECIES {}, TAKEN AT GENERATION {}. THE DIAL IS AT {}. CLICK HERE TO OPEN THE RACK AND PICK ANOTHER.",
                if self.tool == Tool::Release { "CLICK IN THE BOX TO PLACE" } else { "PLACE (,) WILL PUT BACK" },
                jar.name.to_uppercase(),
                jar.genetics.kingdom(),
                jar.species.to_uppercase(),
                jar.taken.generation,
                self.brood_label()
            ),
            None if self.shelf.is_empty() => {
                "THE SHELF IS EMPTY. POINT AT A PLANT OR AN ANT WITH LOOK (Z) AND PRESS KEEP ON THE CELL PAGE TO PUT ITS GENETICS IN A JAR -- IT IS A COPY, SO THE ONE YOU CLICKED GOES ON LIVING. CLICK HERE TO OPEN THE RACK.".to_string()
            }
            None => format!(
                "{} JAR(S) ON THE SHELF AND NONE ARMED. CLICK HERE TO OPEN THE RACK AND PICK ONE; PLACE THEN PUTS IT BACK IN THE BOX, AS ITSELF OR DRIFTED BY A NUMBER OF BROODS.",
                self.shelf.len()
            ),
        }
    }

    pub fn broods(&self) -> u32 {
        self.broods
    }

    /// Move the dial, clamped to `0..=MAX_BROODS`.
    pub fn adjust_broods(&mut self, delta: i32) {
        self.broods = (self.broods as i32 + delta).clamp(0, MAX_BROODS as i32) as u32;
    }

    /// How the dial reads on the page and in a notice: `CLONE` at zero,
    /// because *"0 BROODS"* is the one setting a player will look at and
    /// misread as "off".
    pub fn brood_label(&self) -> String {
        match self.broods {
            0 => "CLONE".to_string(),
            1 => "1 BROOD".to_string(),
            n => format!("{n} BROODS"),
        }
    }

    pub fn selected_param(&self) -> Option<usize> {
        self.param_selected
    }

    /// Every parameter on the page currently showing, in row order — the list
    /// an [`Action::ParamAdjust`] index refers to.
    ///
    /// Rebuilt from the world rather than retained, `App::tunables_list`'s
    /// tradeoff: a few dozen entries off registries already in memory, against
    /// a stored list that would have to be kept in step with a species reload
    /// and with the bar's own species chip.
    pub fn page_params(&self, world: &World, spec: &LabBox) -> Vec<params::Param> {
        let group = self.param_group();
        params::registry(world, spec, self.species_of(world))
            .into_iter()
            .filter(|p| p.group == group)
            .collect()
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
            // **Draws itself** -- see `paint_params`. Its rows carry buttons
            // and a range, so they are not `Row`s; `draw` branches away before
            // this is called, and the arm is here so that a page added to
            // `Panel` cannot be silently left out of both.
            Panel::Params | Panel::Shelf | Panel::Chambers => Vec::new(),
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

    /// **The cell page's one button**, or `None` when the cell it is open on
    /// holds nothing alive.
    ///
    /// Sited in the page header beside the title, which is the only band on
    /// the page that is not a row: the rows are re-read from the world every
    /// frame and change height as an ant walks out from under the marker, so
    /// a button among them would move while the player was reaching for it.
    /// The shelf's `RELOAD` sits in its header for the same reason and this
    /// matches it deliberately — one place on a page is where its verbs are.
    fn keep_button(&self, world: &World, at: (i32, i32), rect: Rect) -> Option<Widget> {
        let id = world.get(at.0, at.1).organism_id();
        let state = world.organism(id)?;
        let species = world.species.get(state.species).name.to_uppercase();
        let w = cell_width(hud::text_width("KEEP"), "", PAD);
        Some(Widget {
            rect: Rect { x: rect.right() - PAGE_PAD - w, y: rect.y + 3, w, h: 11 },
            line1: "KEEP".into(),
            line2: String::new(),
            action: Some(Action::KeepInspected),
            latched: false,
            icon: None,
            ratio: None,
            note: format!(
                "PUT THIS {species}'S GENETICS IN A JAR ON THE SHELF (G). IT IS A COPY -- THE ONE ON THE PAGE GOES ON LIVING, AND KEEPING IT IS NOT A CHOICE WEIGHED AGAINST LETTING IT BREED. A JAR HOLDS WHAT THIS INDIVIDUAL WOULD HAVE PASSED TO ITS OWN OFFSPRING AND NOTHING ELSE, SO IT IS SMALL AND IT SURVIVES A REBUILD OF THE BOX."
            ),
        })
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

    /// What the inspector says about the cell the player clicked, **and about
    /// the individual that owns it**.
    ///
    /// The five cell rows are always five, present or absent, so the page does
    /// not resize under the cursor as the cell changes underneath it. Under
    /// them, when the cell belongs to something alive, is the specimen —
    /// `params::specimen_rows`, which is the other half of the parameters
    /// panel: those pages are a *species*' numbers and reach every member,
    /// this is the one you clicked and every line of it differs between two
    /// individuals of the same species.
    ///
    /// **So the page does change height, and only in one way**: it is longer
    /// while it is pointed at something alive, and the two kingdoms are
    /// different lengths. That is a function of what is in the cell rather
    /// than of the cursor, and the inspector is pinned by a click and not by
    /// hover, so it moves when the world moves — an ant walking out from under
    /// it — which is the honest reading of "what is here now".
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
        let mut rows = vec![
            Row::value("AT", format!("{x},{y}"), FAINT, "THE CELL YOU CLICKED, IN WORLD COORDINATES. CLICK IT AGAIN TO PUT THE INSPECTOR AWAY."),
            Row::value("MATERIAL", material, VALUE, "WHAT IS IN THE CELL RIGHT NOW. RE-READ EVERY FRAME, SO IT CHANGES UNDER YOU WHILE THE BOX RUNS."),
            Row::value("TEMPERATURE", format!("{}C", cell.temperature()), FAINT, "THIS CELL'S OWN TEMPERATURE IN DEGREES, NOT THE BOX AVERAGE. HEAT MOVES CELL TO CELL, SO TWO CELLS SIDE BY SIDE CAN DISAGREE AND THE DIFFERENCE IS WHAT DRIVES IT."),
            Row::value("ORGANISM", species, if organism.is_some() { GOOD } else { FAINT }, "THE SPECIES OF THE LIVING THING THIS CELL BELONGS TO, IF ANY. AN ANT IS TWO CELLS AND A TREE IS THOUSANDS; EITHER WAY THE CELL KNOWS WHICH ORGANISM OWNS IT."),
            Row::value("ENERGY", energy, VALUE, "THAT ORGANISM'S WHOLE-BODY ENERGY, NOT THIS CELL'S SHARE. WATCH IT WHILE THE BOX RUNS: A FORAGING ANT CLIMBS AND A STARVING ONE DOES NOT, AND A PLANT IN GOOD LIGHT BANKS CARBON FASTER THAN ITS BODY SPENDS IT."),
        ];
        let sections = params::specimen_sections(world, cell.organism_id());
        if sections.is_empty() {
            return rows;
        }
        rows.push(Row::gap());

        // **Folded only when it has to be**, which is the rule the whole
        // mechanism rests on. An ant's eight specimen rows fit beside the
        // cell block with room to spare, so its page is exactly what it
        // always was -- three headings and everything under them. A plant's
        // twenty-five do not, so its page shows one group at a time. The
        // player does not have to know which case they are in: what is on
        // screen is the most the screen can hold.
        //
        // Measured 2026-09-01, and this is the defect it fixes: the plant
        // page came to 302px on a 320px screen and `page_rect` had no clamp,
        // so it was drawn at y = -42 and the title, AT, MATERIAL,
        // TEMPERATURE and ORGANISM were simply off the top of the screen. It
        // did not look truncated -- it looked like a page that started at
        // ENERGY.
        let head_h = Row::head("", false, 0, Action::SpecimenSection(0), "").height();
        let mut room = page_content_budget()
            - rows.iter().map(Row::height).sum::<i32>()
            - sections.len() as i32 * head_h;
        let mut open = vec![false; sections.len()];
        // **The group you last clicked gets the room first, then the rest fill
        // from the top with whatever still fits.** Not an accordion, which was
        // the first build and showed strictly less: on the plant page the
        // three-row `LIFE` block fits alongside either of the two eleven-row
        // blocks with 14px to spare, so closing it bought nothing and cost the
        // player the individual's identity while they were reading its
        // numbers. Two consequences worth naming, because they are what makes
        // this predictable rather than merely clever: a click always opens
        // what it says (the chosen group is served before anything else, so it
        // can never be crowded out by a group the player did not ask for), and
        // a page whose groups all fit -- an ant's -- never folds at all, so
        // that page is exactly what it always was.
        let chosen = self.specimen_section.min(sections.len() - 1);
        for i in std::iter::once(chosen).chain(0..sections.len()) {
            let cost = sections[i].2.len() as i32 * LINE;
            if !open[i] && cost <= room {
                open[i] = true;
                room -= cost;
            }
        }
        for (i, (label, note, section)) in sections.iter().enumerate() {
            let open = open[i];
            rows.push(Row::head(
                label,
                open,
                if open { 0 } else { section.len() },
                Action::SpecimenSection(i),
                if open {
                    (*note).to_string()
                } else {
                    format!("{note} CLICK TO SHOW ITS {} ROWS -- THE PAGE HOLDS ONE GROUP AT A TIME BECAUSE ALL OF THEM AT ONCE DO NOT FIT ON THE SCREEN.", section.len())
                },
            ));
            if open {
                for (label, value, note) in section {
                    rows.push(Row::value(label, value, VALUE, note));
                }
            }
        }
        fit_rows(rows, page_content_budget())
    }
}

/// The tallest a page's rows may be before the page runs off the top.
///
/// Derived from the geometry rather than written down: a page is drawn
/// upward from just above the bar, so what it may not exceed is the gap
/// between the bar and the top margin, less its own header and padding.
fn page_content_budget() -> i32 {
    bar_top() - 4 - MARGIN - PAGE_HEADER - PAGE_PAD
}

/// **Guarantee the rows fit, and say so when they did not.**
///
/// The backstop under the fold above, and it exists because the fold is a
/// judgement about *today's* row counts: a species with a wider genome, or one
/// more line added to `STATE`, would overflow a single open group and the page
/// would go back to being drawn off the top of the screen. This cannot -- it
/// drops whatever will not fit and puts the count in its place.
///
/// `CLAUDE.md`'s cap rule is the reason it is written this way: a cap must
/// bound work rather than gate whether something happens, and the test is
/// whether exhausting it produces an *answer*. Trimming to the budget and
/// printing `+7 MORE` produces no answer at all, which is the safe half; a
/// silent stop at the bottom of the screen would have produced "this page has
/// fourteen rows on it", which is not true.
fn fit_rows(mut rows: Vec<Row>, budget: i32) -> Vec<Row> {
    let marker = Row::value("...", "+0 MORE", FAINT, "");
    if rows.iter().map(Row::height).sum::<i32>() <= budget {
        return rows;
    }
    let room = budget - marker.height();
    let mut used = 0;
    let keep = rows
        .iter()
        .take_while(|row| {
            used += row.height();
            used <= room
        })
        .count();
    let dropped = rows.len() - keep;
    rows.truncate(keep);
    rows.push(Row::value(
        "...",
        format!("+{dropped} MORE"),
        FAINT,
        "MORE ROWS THAN THE SCREEN HOLDS, SO THE REST ARE NOT DRAWN. THIS IS A BACKSTOP RATHER THAN A FEATURE -- IF YOU ARE SEEING IT, A GROUP HAS GROWN PAST WHAT ONE PAGE CAN SHOW AND WANTS SPLITTING.",
    ));
    rows
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
    // **Clamped at the top, and it must never be what saves the page.** A
    // page taller than the screen used to be given a negative `y` and drawn
    // with its title and first rows off the top edge -- measured at
    // y = -42 on the cell page of a plant, 2026-09-01. The clamp stops that
    // looking like a page that starts at its fifth row, but it trades one
    // silent loss for another: it now runs off the *bottom* instead. What
    // actually keeps the page whole is `fit_rows`, which trims to
    // `page_content_budget` and prints the count of what it dropped.
    Rect { x, y: (bottom - h).max(MARGIN), w, h }
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
    taps: &mut Vec<Widget>,
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
            Body::Head { label, open, hidden, action } => {
                for x in rect.x + 1..rect.right() - 1 {
                    render::put(frame, W, H, x, y + 1, DIVIDER);
                }
                // `-` open, `+` shut, then the count of what is behind it.
                // The sign is on the left of the label rather than the right
                // because that column is where the eye already is, and a row
                // of headings has to be scannable without reading any of it.
                let sign = if *open { "-" } else { "+" };
                text(frame, left, y + 4, sign, if *open { VALUE } else { GOOD });
                text(frame, left + 8, y + 4, label, if *open { VALUE } else { FAINT });
                if !*open {
                    text(frame, right - hud::text_width(&hidden.to_string()), y + 4, &hidden.to_string(), GOOD);
                }
                // An invisible hit target, exactly the row it was drawn at.
                // Empty `line1` is the house idiom for "clickable, not
                // painted" -- `paint_widget`'s own caller skips it, so the
                // heading above is the only thing on screen.
                taps.push(Widget {
                    rect: Rect { x: rect.x + 1, y, w: rect.w - 2, h: row.height() },
                    line1: String::new(),
                    line2: String::new(),
                    action: Some(*action),
                    latched: false,
                    icon: None,
                    ratio: None,
                    note: String::new(),
                });
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

// ------------------------------------------------------- the parameters page
//
// **A page, not another button group.** The bar's transport row fits at
// exactly its own width at seven speed stops, so the panel had to be
// something you open; and what it has to show — a name, a live figure, a way
// to move it, and the range it may move through — is four columns, which is a
// page shape and not a button shape.

/// One parameter row, tall enough for a 7-pixel label with a pixel of air
/// either side and a 2-pixel fill track under it.
const PARAM_ROW: i32 = 12;
/// A `SOIL` / `WATER` / `PACKEDSOIL` subheader, drawn wherever the category
/// changes. The sandbox's own panel does this and it is the whole reason a
/// row can print `PENETRATION RESISTANCE` rather than
/// `SOIL PENETRATION RESISTANCE` and still be unambiguous.
const PARAM_HEAD: i32 = 10;
/// The tab strip, and the pager under the rows.
const PARAM_TABS: i32 = 14;
/// **What one jar says about itself on hover.**
///
/// Everything a player needs to decide whether to release this one and not
/// the one under it, in the order they would ask: what it is, where it came
/// from, and whether they bred it themselves.
fn jar_note(jar: &crate::sim::specimen::Specimen) -> String {
    let mut s = format!(
        "{} -- A {} OF SPECIES {}, TAKEN AT FRAME {} FROM GENERATION {} OF LINEAGE {}.",
        jar.name.to_uppercase(),
        jar.genetics.kingdom(),
        jar.species.to_uppercase(),
        jar.taken.frame,
        jar.taken.generation,
        jar.taken.lineage
    );
    match &jar.taken.from_jar {
        Some((parent, 0)) => s.push_str(&format!(" AN EXACT COPY OF {}.", parent.to_uppercase())),
        Some((parent, n)) => s.push_str(&format!(" DRIFTED {n} BROODS FROM {}.", parent.to_uppercase())),
        None => s.push_str(" TAKEN STRAIGHT OUT OF THE BOX."),
    }
    s
}

/// **How far the brood dial goes.**
///
/// Eight, and it is a legibility bound rather than a modelling one: nothing
/// stops the drift loop running longer, but past a handful of broods a
/// specimen is no longer *that animal, varied* — it is a walk away from it,
/// and the shelf's whole claim is that you can see the relationship. A
/// player who wants more releases the drifted jar and keeps drifting it,
/// which is the same arithmetic with the intermediate steps kept.
const MAX_BROODS: u32 = 8;

/// One jar row on the shelf page. `PARAM_ROW`'s height, deliberately: the
/// two pages sit in the same place and a row that was a different height
/// would read as a different kind of thing.
const SHELF_ROW: i32 = 12;

/// How many jars are on screen at once. Ten rather than `PARAM_ROWS`' 13 —
/// this page carries a dial strip the parameters page does not, and the two
/// have to end at the same height above the bar.
const SHELF_ROWS: usize = 10;

/// The shelf page's width: the widest jar name a player can type without the
/// row wrapping, beside the species and generation columns. Measured through
/// `hud::text_width` for `param_page_width`'s reason.
fn shelf_page_width() -> i32 {
    let rows = PAGE_PAD * 2 + hud::text_width("A_LONG_ENOUGH_JAR_NAME") + SHELF_RIGHT;
    // **...and wide enough that the dial strip does not touch the verbs.**
    // Measured rather than eyeballed, `param_page_width`'s rule: the rows
    // alone gave a page on which `+` and `COPY` were flush against each
    // other and read as one control, which a rendered tile showed and no
    // layout assertion could. The terms are the strip left to right — the
    // `DRIFT` label, both dial steps, the widest value it can hold, the
    // three verb faces — plus a gap the strip must keep in the middle.
    let step = cell_width(hud::text_width("W"), "", PAD);
    let dial = hud::text_width("DRIFT") + 6 + step + 4 + hud::text_width("8 BROODS") + 6 + step;
    let verbs: i32 = ["PLACE", "COPY", "DISCARD", "PROMOTE"].iter().map(|v| cell_width(hud::text_width(v), "", PAD) + 2).sum();
    rows.max(PAGE_PAD * 2 + dial + SHELF_STRIP_GAP + verbs)
}

/// The gap the dial strip keeps between its `+` and the first verb.
const SHELF_STRIP_GAP: i32 = 10;

/// How much of a jar row the right-hand columns own: the species name and
/// the generation, plus a gap so a long name does not run into them.
const SHELF_RIGHT: i32 = 96;

/// The rack page's geometry.
///
/// **Twelve rows and one picture.** The same frame-cost argument as
/// `PARAM_ROWS` below: this page has no dirty-rect skip, so every row is
/// repainted on every drawn frame it is open. Twelve rows plus a 128x80
/// picture is about 39,000 pixels, well under the parameters page, and it is
/// only paid while somebody has deliberately opened it. Past twelve the page
/// scrolls, which is the case a batch makes rather than an unusual one.
const RACK_ROWS: usize = 12;
const RACK_ROW: i32 = 11;
/// The column header's own band.
const RACK_HEAD: i32 = 11;
/// The fixed column a batch dial's value sits in, wide enough for five
/// digits — `TICKS` reaches 45,000. Fixed rather than fitted so a value that
/// gains a digit cannot shove its own `+` button sideways under the cursor.
const BATCH_VALUE_W: i32 = 34;

/// **The rack page's columns: a heading and the widest thing it can hold.**
///
/// The offsets were hand-counted at first and they overlapped — reported as
/// *"seed and batch text overlap in the menu"*. `SEED 9999` is 53 px and the
/// FRAME column started 62 px in from a seed column that started at 16, so
/// the two collided by 8 px, and by more once a batch pushed a seed to five
/// digits. `layout` and `param_page_width` both already say why: **measure
/// through `hud::text_width`, never count by hand.** This is that rule
/// applied to a table.
///
/// The second string is the **widest value the column can ever hold**, not a
/// typical one. A column sized to what it usually shows is a column that
/// breaks the first time a run gets big, which is the run you most want to
/// read.
const RACK_COLS: [(&str, &str); 6] = [
    ("SEED", "SEED 999999"),
    ("FRAME", "9999999"),
    ("PLT", "9999"),
    ("ANI", "9999"),
    ("GEN", "99/99"),
    ("SOWN", "999999"),
];

/// Gap between columns.
const RACK_GAP: i32 = 6;

/// Where each column starts, relative to the page's left margin.
///
/// Derived once and read by both the header and the rows, so the two cannot
/// drift apart — which is its own bug class: a header that no longer sits
/// over its column is worse than no header.
fn rack_col_x() -> [i32; RACK_COLS.len()] {
    let mut out = [0i32; RACK_COLS.len()];
    // The row number sits first, outside the table.
    let mut x = hud::text_width("99") + RACK_GAP;
    for (i, (_, widest)) in RACK_COLS.iter().enumerate() {
        out[i] = x;
        x += hud::text_width(widest) + RACK_GAP;
    }
    out
}

/// **Every fixed string the rack page draws, in one list.**
///
/// Named rather than inlined so `every_string_the_bar_can_draw_is_drawable`
/// can reach them: this page paints itself, so nothing in `panel_rows`
/// covers it, and `hud::draw_text` renders a character outside its 5x7 set as
/// a **silent blank**. The column header read `  SEED` in its first contact
/// sheet because it began `  # SEED` and `#` has no glyph.
const RACK_LITERALS: [&str; 6] = [
    "CLICK A ROW FOR ITS PICTURE",
    "-- THE BOX YOU ARE IN",
    "-- KEPT AS NUMBERS ONLY, THE WORLD WAS NOT HELD",
    "HERE",
    "RECORD",
    "REBUILDING",
];
/// The picture is the world at a quarter scale in each axis.
const RACK_THUMB_SHRINK: u32 = 4;
const RACK_THUMB_H: i32 = (H / RACK_THUMB_SHRINK) as i32;

/// Wide enough for the widest row this page can produce, measured through
/// `hud::text_width` rather than counted by hand — `layout`'s rule, for
/// `layout`'s reason. The picture sets the floor: a page narrower than its own
/// thumbnail would clip it.
fn rack_page_width() -> i32 {
    let col = rack_col_x();
    let last = col[RACK_COLS.len() - 1] + hud::text_width(RACK_COLS[RACK_COLS.len() - 1].1);
    // Plus room for the right-hand state marker, which is drawn from the
    // right margin inward and would otherwise sit on the last column.
    let row = last + RACK_GAP + hud::text_width("REBUILDING");
    (PAGE_PAD * 2 + row).max((W / RACK_THUMB_SHRINK) as i32 + PAGE_PAD * 2)
}

/// Draw a chamber's picture at `(x, y)`.
///
/// Clipped rather than assumed to fit: the page is placed against the bar and
/// a short window can push it off the top, and a blit that ran past the
/// framebuffer would be a panic in a page nobody opens on a small screen.
fn blit(frame: &mut [u8], x: i32, y: i32, thumb: &super::Thumb) {
    for ty in 0..thumb.h as i32 {
        for tx in 0..thumb.w as i32 {
            let i = ((ty * thumb.w as i32 + tx) * 4) as usize;
            let px = [thumb.rgba[i], thumb.rgba[i + 1], thumb.rgba[i + 2], 255];
            render::put(frame, W, H, x + tx, y + ty, px);
        }
    }
}

/// How many rows are on screen at once.
///
/// **Thirteen, and it is a frame-cost number as much as a legibility one.**
/// A lab redraw measures a flat 19.2-19.9 ns/px (the field lane, 2026-08-30)
/// and this page has no dirty-rect skip, so every row is paid for on every
/// drawn frame it is open. At thirteen rows the page is 295x210, which is
/// 61,950 pixels and about **1.2 ms of a drawn frame** — five times what the
/// bar's second row costs, and unlike the bar it is only paid while somebody
/// has deliberately opened it. Thirteen rows shows three of the four pages
/// whole; `PLANT` is fourteen lines and pays one press of the pager.
const PARAM_ROWS: usize = 13;
/// The page's width. Wide enough for the widest field name this registry can
/// produce (`REPRODUCTIVE ALLOCATION`, 23 characters) beside the four right-
/// hand columns, and measured through `hud::text_width` rather than counted by
/// hand — `layout`'s rule, for `layout`'s reason.
fn param_page_width() -> i32 {
    PAGE_PAD * 2 + hud::text_width("REPRODUCTIVE ALLOCATION") + PARAM_RIGHT
}
/// How much of a row the four right-hand columns own: `[-]`, the value, `[+]`,
/// and the range, plus a gap so the longest name does not run into the first
/// of them. **Measured against the widest thing each column can hold** rather
/// than eyeballed: this was 118 for one contact sheet, which put
/// `REPRODUCTIVE ALLOCATION` through the `-` face and drew `10000.000` over
/// it. Nothing failed; a sheet showed it.
const PARAM_RIGHT: i32 = 142;
/// x of the `-` face, the value's right edge and the `+` face — all relative
/// to the row's right margin, in one place so the three cannot drift apart.
///
/// **The value has a fixed column and is right-aligned in it**, rather than
/// the `-` sitting three pixels from whatever the text happens to be. Both
/// read the same on a still frame and only the fixed column survives being
/// used: a value that gains a digit would otherwise shove its own `-` button
/// sideways under the cursor, which is the failure `lay_out` sizes the phase
/// button and the species chip against.
const PARAM_MINUS: i32 = 134;
const PARAM_VALUE_RIGHT: i32 = 75;
const PARAM_PLUS: i32 = 72;
/// A `-` or `+` face.
const PARAM_STEP_W: i32 = 9;

/// How many rows the whole page is, subheaders included — what the visible
/// window is measured against, since a subheader takes a slot on screen.
fn param_lines(list: &[params::Param]) -> Vec<Line> {
    let mut out = Vec::new();
    let mut last: Option<String> = None;
    for (i, p) in list.iter().enumerate() {
        if last.as_deref() != Some(p.tunable.category.as_str()) {
            last = Some(p.tunable.category.clone());
            out.push(Line::Head(p.tunable.category.to_uppercase()));
        }
        out.push(Line::Row(i));
    }
    out
}

/// One drawn line of the parameters page.
enum Line {
    /// A category subheader — a material name, or the species the page is
    /// about.
    Head(String),
    /// Index into the page's parameter list.
    Row(usize),
}

impl Line {
    fn height(&self) -> i32 {
        match self {
            Line::Head(_) => PARAM_HEAD,
            Line::Row(_) => PARAM_ROW,
        }
    }
}

/// A field name as a player reads it: `penetration_resistance` is
/// `PENETRATION RESISTANCE`.
///
/// **Every character goes through `hud::has_glyph` first.** `draw_text`
/// renders anything outside its 5x7 set as a silent blank, and that trap has
/// shipped three times here; a name arriving from an asset file is exactly the
/// path a test over hardcoded strings would not cover, so an unknown character
/// becomes a visible `?` rather than a gap in the middle of a word.
fn param_label(name: &str) -> String {
    name.chars()
        .map(|c| if c == '_' { ' ' } else { c })
        .map(|c| if hud::has_glyph(c) { c.to_ascii_uppercase() } else { '?' })
        .collect()
}

impl Ui {
    /// Draw the parameters page and return the note under the cursor.
    ///
    /// Builds `params_bar` as it paints — one pass, hover found with the same
    /// `y` the row was drawn at. `paint_page`'s rule and `stats.rs`'s: a
    /// second pass over the same arithmetic is how a control and the thing it
    /// activates come to disagree.
    /// **The rack.** Draws itself for `paint_params`' reason — its rows
    /// carry a verb rather than a label — and builds `shelf_bar` as it
    /// paints, so the rectangles a click is tested against are the ones
    /// that were drawn.
    ///
    /// **The dial is above the rack, not beside a row.** It applies to
    /// whichever jar is armed, and a per-row dial would be eight copies of
    /// one number that can disagree with each other — and would say, wrongly,
    /// that drift is a property of the specimen rather than of the release.
    /// **The rack: one row per chamber, and the numbers you compare them by.**
    ///
    /// The tabs above the bar reach the first five. This is the page a rack of
    /// fifty is read through, so the columns are chosen for *comparing* rather
    /// than for describing: what is alive, how deep the generations got, and
    /// how much has been born. `stats::Census` already carries every one of
    /// them, so a row is a read rather than a walk of a frozen box.
    ///
    /// **`FRAME` is a column and not a debug aid.** A frozen chamber and a
    /// running one look identical in any picture of a rack, so the number that
    /// separates them is on every row — `CLAUDE.md`, *"did it fire at all"
    /// needs a counter, not a picture*.
    ///
    /// **A chamber that has never been stepped shows dashes, not zeroes.** Its
    /// census has never run, and a row of zeroes would say *nothing lives
    /// here* where the truth is *nobody has looked yet* — which is the
    /// difference between a dead box and a fresh one, and the whole question a
    /// batch is read to answer.
    fn paint_rack(&mut self, frame: &mut [u8], chambers: &[super::ChamberSummary], thumb: Option<&super::Thumb>, state: &BarState) -> Option<(String, Rect, i32)> {
        let mut widgets: Vec<Widget> = Vec::new();
        let thumb_batch_running = state.batch.progress.is_some();
        let mut note: Option<(String, Rect, i32)> = None;

        // Re-clamped here rather than trusted: closing a chamber renumbers
        // the ones after it, and a selection held across that would highlight
        // a different box than the one that was picked.
        self.rack_selected = self.rack_selected.filter(|&i| i < chambers.len());
        let shown = chambers.len().min(RACK_ROWS);
        self.rack_scroll = self.rack_scroll.min(chambers.len().saturating_sub(shown));

        let w = rack_page_width();
        let picture_h = if thumb.is_some() { RACK_THUMB_H + 4 } else { 0 };
        let batch_h = 16 + if thumb_batch_running { 11 } else { 0 };
        let h = PAGE_HEADER + RACK_HEAD + RACK_ROW * shown.max(1) as i32 + picture_h + batch_h + PARAM_TABS + PAGE_PAD;
        let bottom = bar_top() - 4;
        let rect = Rect { x: MARGIN, y: (bottom - h).max(MARGIN), w, h };
        self.rack_box = Some(rect);

        fill(frame, rect, PANEL_BG);
        outline(frame, rect, PANEL_EDGE);
        let left = rect.x + PAGE_PAD;
        let right = rect.right() - PAGE_PAD;
        text(frame, left, rect.y + 6, "THE RACK", TITLE);

        // NEW, in the header. The rack's own verb, and it reseeds: at the same
        // seed every draw in the engine is a pure function of `(world.seed,
        // identity, position)`, so an unseeded copy is a bit-identical box and
        // a rack of them is one sample wearing many labels.
        let new_w = cell_width(hud::text_width("NEW"), "", PAD);
        widgets.push(Widget {
            rect: Rect { x: right - new_w, y: rect.y + 3, w: new_w, h: 11 },
            line1: "NEW".into(),
            line2: String::new(),
            action: Some(Action::ChamberAdd),
            latched: false,
            icon: None,
            ratio: None,
            note: "ANOTHER CHAMBER: THIS BOX'S RECIPE AGAIN, AT THE NEXT UNUSED SEED. THE SEED IS WHAT MAKES IT A REPLICATE RATHER THAN A COPY -- AT THE SAME SEED IT WOULD BE THE SAME WORLD, CELL FOR CELL.".into(),
        });
        // CLEAR, beside NEW. A rack of fifty is made by one click and has to
        // be unmade by one too -- clearing it a row at a time is fifty clicks
        // and a verb nobody would use.
        let clear_w = cell_width(hud::text_width("CLEAR"), "", PAD);
        widgets.push(Widget {
            rect: Rect { x: right - new_w - clear_w - 4, y: rect.y + 3, w: clear_w, h: 11 },
            line1: "CLEAR".into(),
            line2: String::new(),
            action: Some(Action::ChamberClear),
            latched: false,
            icon: None,
            ratio: None,
            note: "THROW AWAY EVERY CHAMBER AND EVERY RECORD EXCEPT THE BOX YOU ARE IN. THE ONE ON SCREEN IS KEPT, SO CLEARING NEVER ALSO MOVES YOU SOMEWHERE YOU DID NOT ASK TO GO.".into(),
        });
        for x in rect.x + 1..rect.right() - 1 {
            render::put(frame, W, H, x, rect.y + PAGE_HEADER - 4, DIVIDER);
        }

        // ---- the column header.
        //
        // **Not decoration.** The row `864 6 52 0/0 0` is unreadable without
        // it — which is the whole failure of a comparison page, since a
        // column you cannot name is a column you cannot compare on. Caught by
        // looking at the first render rather than by any test.
        let mut y = rect.y + PAGE_HEADER;
        let col = rack_col_x();
        for (i, (head, widest)) in RACK_COLS.iter().enumerate() {
            let sorted_on = self.rack_sort.map(|(c, _)| c) == Some(i);
            let tint = if sorted_on {
                TITLE
            } else {
                match i {
                    2 => GOOD,
                    3 => FAIR,
                    _ => FAINT,
                }
            };
            // The arrow is drawn *after* the heading rather than in place of
            // a character of it, so a sorted column's name stays readable.
            let label = match self.rack_sort {
                Some((c, desc)) if c == i => format!("{head}{}", if desc { "\\" } else { "/" }),
                _ => (*head).to_string(),
            };
            text(frame, left + col[i], y, &label, tint);
            // The whole column width is the target: a three-character heading
            // is not something a person can reliably hit.
            widgets.push(Widget {
                rect: Rect { x: left + col[i] - 2, y: y - 2, w: hud::text_width(widest) + 4, h: RACK_HEAD },
                line1: String::new(),
                line2: String::new(),
                action: Some(Action::ChamberSort(i)),
                latched: false,
                icon: None,
                ratio: None,
                note: String::new(),
            });
        }
        y += RACK_HEAD;

        // **Sorted into a display order, never reordered in place.** A row
        // carries its own `index`, and every verb below is aimed with that
        // rather than with the position it happens to be drawn at — sorting a
        // list and then acting on screen positions is how a click ends up
        // opening the wrong chamber.
        let mut order: Vec<&super::ChamberSummary> = chambers.iter().collect();
        if let Some((c, desc)) = self.rack_sort {
            order.sort_by(|a, b| {
                let key = |r: &super::ChamberSummary| -> i64 {
                    let Some(n) = r.census.as_ref() else { return -1 };
                    match c {
                        0 => r.seed as i64,
                        1 => r.frame as i64,
                        2 => n.plants as i64,
                        3 => n.animals as i64,
                        4 => n.plant_generation as i64,
                        _ => n.seeds_borne as i64,
                    }
                };
                // Tie broken on `index`, so the order is total and a redraw
                // cannot shuffle equal rows under the cursor.
                let ord = key(a).cmp(&key(b)).then(a.index.cmp(&b.index));
                if desc { ord.reverse() } else { ord }
            });
        }
        for x in rect.x + 1..rect.right() - 1 {
            render::put(frame, W, H, x, y - 2, DIVIDER);
        }

        // ---- the rows.
        for ch in order.iter().skip(self.rack_scroll).take(shown) {
            let row = ch.index;
            let selected = self.rack_selected == Some(row);
            let band = Rect { x: rect.x + 1, y, w: rect.w - 2, h: RACK_ROW };
            // **Before the text, not after.** A hover fill painted over the
            // row it highlights erases the line you are pointing at, which
            // looks exactly like a row that has no data.
            let hovered = self.cursor.is_some_and(|(cx, cy)| band.contains(cx, cy));
            if selected {
                fill(frame, band, FACE_ON);
            } else if hovered {
                fill(frame, band, FACE_HOVER);
            }
            // The whole row is the button. A four-pixel-wide number is not a
            // click target, and a row that only responds on its label is a row
            // players think is broken.
            widgets.push(Widget {
                rect: band,
                line1: String::new(),
                line2: String::new(),
                action: Some(Action::ChamberSelect(row)),
                latched: false,
                icon: None,
                ratio: None,
                note: String::new(),
            });

            let tint = if ch.active { SUB_ON } else if selected { TITLE } else { FAINT };
            text(frame, left, y + 2, &ch.label, tint);
            text(frame, left + col[0], y + 2, &format!("SEED {}", ch.seed), FAINT);
            // The counter that says whether it is frozen.
            text(frame, left + col[1], y + 2, &format!("{}", ch.frame), if ch.active { SUB_ON } else { FAINT });

            match &ch.census {
                Some(c) => {
                    text(frame, left + col[2], y + 2, &format!("{}", c.plants), GOOD);
                    text(frame, left + col[3], y + 2, &format!("{}", c.animals), FAIR);
                    text(frame, left + col[4], y + 2, &format!("{}/{}", c.plant_generation, c.animal_generation), FAINT);
                    text(frame, left + col[5], y + 2, &format!("{}", c.seeds_borne), FAINT);
                }
                // Never looked at, which is not the same as empty. One dash
                // per column, on the column, rather than one run of text
                // guessed into position.
                None => {
                    for x in col.iter().skip(2) {
                        text(frame, left + x, y + 2, "-", FAINT);
                    }
                }
            }
            if ch.active {
                text(frame, right - hud::text_width("HERE"), y + 2, RACK_LITERALS[3], SUB_ON);
            } else if ch.rebuilding {
                text(frame, right - hud::text_width(RACK_LITERALS[5]), y + 2, RACK_LITERALS[5], SUB_ON);
            } else if ch.on_record {
                text(frame, right - hud::text_width(RACK_LITERALS[4]), y + 2, RACK_LITERALS[4], FAINT);
            }
            y += RACK_ROW;
        }

        // ---- the picture of whichever row is highlighted.
        if let Some(t) = thumb {
            let ty = y + 2;
            blit(frame, rect.x + (rect.w - t.w as i32) / 2, ty, t);
            y = ty + RACK_THUMB_H;
        }

        // ---- the rack's own verb: run copies of this box, headless.
        //
        // **Two dials and a button, not a hidden default.** The owner's
        // standing direction for the lab is *"give me the tools, data, access
        // to the parameters and I do that testing myself"* — a batch size
        // and a run length chosen in the source are two decisions taken away
        // from the person the feature is for.
        let by = y + 3;
        let mut dial = |label: &str, value: String, minus: Action, plus: Action, note: &'static str, x: i32, w: &mut Vec<Widget>| -> i32 {
            let step = cell_width(hud::text_width("W"), "", PAD);
            text(frame, x, by + 2, label, FAINT);
            let mut cx = x + hud::text_width(label) + 4;
            for (face, action) in [("-", minus), ("+", plus)] {
                if face == "+" {
                    // The value sits between the two faces, in a **fixed**
                    // column: a value that gains a digit would otherwise
                    // shove its own `+` sideways under the cursor, which is
                    // the failure the parameters page already sizes against.
                    text(frame, cx + 3, by + 2, &value, VALUE);
                    cx += BATCH_VALUE_W;
                }
                w.push(Widget {
                    rect: Rect { x: cx, y: by, w: step, h: 11 },
                    line1: face.into(),
                    line2: String::new(),
                    action: Some(action),
                    latched: false,
                    icon: None,
                    ratio: None,
                    note: note.into(),
                });
                cx += step + 2;
            }
            cx
        };
        let mut bx = left;
        bx = dial("COPIES", format!("{}", state.batch.copies), Action::BatchCopies(-1), Action::BatchCopies(1),
            "HOW MANY COPIES OF THIS BOX TO RUN. EACH ONE GETS ITS OWN SEED, WHICH IS WHAT MAKES THEM DIFFERENT WORLDS RATHER THAN THE SAME WORLD N TIMES -- MEASURED, THE SEED ALONE MOVES THE FINAL CENSUS BY 2.4 TO 3.1 TIMES.", bx, &mut widgets);
        // The return is the next free x, unused after the last dial — kept as
        // a return rather than dropped so a third dial slots in beside these
        // two without re-deriving the arithmetic.
        let _ = dial("TICKS", format!("{}", state.batch.frames), Action::BatchFrames(-1), Action::BatchFrames(1),
            "HOW LONG EACH COPY RUNS, IN SIMULATED TICKS. THE FIRST INHERITED PLANT APPEARS AROUND 1,800 AND THE FOURTH GENERATION AROUND 10,000. RUNNING IT HEADLESS IS EXACT -- IT IS THE SAME SIMULATION YOU WOULD HAVE WATCHED, NOT AN APPROXIMATION.", bx + 6, &mut widgets);

        let running = state.batch.progress.is_some();
        let (face, action, why) = if running {
            ("STOP", Action::BatchStop, "STOP THE RACK. COPIES THAT HAVE ALREADY FINISHED ARE KEPT -- STOPPING LOSES ONLY THE ONES STILL IN FLIGHT.")
        } else {
            ("RUN", Action::BatchRun, "RUN THE COPIES NOW, IN THE BACKGROUND. THE BOX ON SCREEN KEEPS WORKING WHILE THEY GO, AND EACH ONE APPEARS IN THIS LIST AS IT LANDS.")
        };
        let bw = cell_width(hud::text_width(face), "", PAD) + 8;
        widgets.push(Widget {
            rect: Rect { x: right - bw, y: by, w: bw, h: 11 },
            line1: face.into(),
            line2: String::new(),
            action: Some(action),
            latched: running,
            icon: None,
            ratio: None,
            note: why.into(),
        });
        y = by + 13;

        if let Some(p) = state.batch.progress {
            // **The counter beside the work.** A rack of rows filling in
            // looks the same whether four copies are running or none are, so
            // the count and the clock are on screen rather than inferred.
            let left_note = match p.remaining() {
                Some(d) => format!("{}M{:02}S LEFT", d.as_secs() / 60, d.as_secs() % 60),
                None => "ESTIMATING".to_string(),
            };
            let line = format!(
                "{}/{} DONE  {}M{:02}S  {}  {} HELD",
                p.finished + p.failed,
                p.total,
                p.elapsed.as_secs() / 60,
                p.elapsed.as_secs() % 60,
                left_note,
                p.held
            );
            text(frame, left, y, &line, SUB_ON);
            if p.failed > 0 {
                text(frame, left, y + 9, &format!("{} FAILED TO BUILD", p.failed), POOR);
                y += 9;
            }
            y += 11;
        }

        // ---- the two verbs on the highlighted row.
        if let Some(i) = self.rack_selected {
            let vy = y + 3;
            let mut vx = left;
            let here = chambers.get(i).is_some_and(|c| c.active);
            // An on-record row's world was dropped for the memory budget, so
            // there is nothing to walk into. Its verbs are drawn dead rather
            // than hidden, and the row itself says which it is.
            let on_record = chambers.get(i).is_some_and(|c| c.on_record);
            let rebuilding = chambers.get(i).is_some_and(|c| c.rebuilding);
            for (label, action, on, why) in [
                ("ENTER", Action::Chamber(i), !here && !on_record,
                 "PUT THIS CHAMBER ON SCREEN. THE ONE YOU LEAVE IS HELD EXACTLY WHERE IT IS -- IT RESUMES ON THE TICK IT STOPPED AT, NOT FROM THE START."),
                ("CLOSE", Action::ChamberClose(i), !here && !on_record,
                 "THROW THIS CHAMBER AWAY. THE BOX YOU ARE IN CANNOT BE CLOSED: STEP INTO ANOTHER ONE FIRST, SO THAT CLOSING NEVER ALSO MOVES YOU SOMEWHERE YOU DID NOT ASK TO GO."),
                // Only ever on an on-record row: a chamber that still has its
                // world has nothing to rebuild.
                ("REBUILD", Action::ChamberRebuild(i), on_record && !rebuilding,
                 "RUN THIS ROW AGAIN AND KEEP THE WORLD THIS TIME. ITS NUMBERS WERE KEPT BUT ITS WORLD WAS NOT, AND THE RECIPE PLUS ITS SEED REPRODUCES THE RUN EXACTLY -- SO WHAT COMES BACK IS THE SAME BOX, NOT A SIMILAR ONE. IT RUNS IN THE BACKGROUND."),
            ] {
                let bw = cell_width(hud::text_width(label), "", PAD) + 6;
                if on {
                    widgets.push(Widget {
                        rect: Rect { x: vx, y: vy, w: bw, h: 11 },
                        line1: label.into(),
                        line2: String::new(),
                        action: Some(action),
                        latched: false,
                        icon: None,
                        ratio: None,
                        note: why.into(),
                    });
                } else {
                    // Drawn dead rather than hidden. A verb that vanishes when
                    // it does not apply teaches nothing; one that is visibly
                    // unavailable says why when you hover it.
                    text(frame, vx + PAD, vy + 2, label, SUB);
                }
                vx += bw + 4;
            }
            if here {
                text(frame, vx + 4, vy + 2, RACK_LITERALS[1], FAINT);
            } else if rebuilding {
                text(frame, vx + 4, vy + 2, RACK_LITERALS[5], SUB_ON);
            } else if on_record {
                text(frame, vx + 4, vy + 2, RACK_LITERALS[2], FAINT);
            }
        } else {
            text(frame, left, y + 5, RACK_LITERALS[0], FAINT);
        }

        // Only the real buttons get a face. A row is a band, not a button:
        // painting it as one would put a bevel round every line of the table,
        // and its highlight is already drawn above, under its own text.
        for wid in widgets.iter().filter(|w| !w.line1.is_empty()) {
            let hover = self.cursor.is_some_and(|(x, y)| wid.rect.contains(x, y));
            let down = hover && self.pressed.is_some() && self.pressed == wid.action;
            paint_widget(frame, wid, hover, down);
        }
        self.rack_bar = Bar { widgets, dividers: Vec::new() };
        if let Some(wid) = self.rack_bar.hovered(self.cursor) {
            if !wid.note.is_empty() {
                note = Some((wid.note.clone(), wid.rect, wid.rect.y - 4));
            }
        }
        note
    }

    fn paint_shelf(&mut self, frame: &mut [u8]) -> Option<(String, Rect, i32)> {
        let mut widgets: Vec<Widget> = Vec::new();
        let mut note: Option<(String, Rect, i32)> = None;

        let count = self.shelf.len();
        let shown = count.min(SHELF_ROWS);
        // **A tall enough page when the rack is empty**, so the first thing
        // a player sees is the sentence that tells them how to fill it
        // rather than an empty box that reads as broken.
        let rows_h = SHELF_ROW * shown.max(1) as i32;
        let w = shelf_page_width();
        let h = PAGE_HEADER + PARAM_TABS + rows_h + PARAM_TABS + PAGE_PAD;
        let bottom = bar_top() - 4;
        let rect = Rect { x: MARGIN, y: (bottom - h).max(MARGIN), w, h };
        self.shelf_box = Some(rect);

        fill(frame, rect, PANEL_BG);
        outline(frame, rect, PANEL_EDGE);
        let left = rect.x + PAGE_PAD;
        let right = rect.right() - PAGE_PAD;
        text(frame, left, rect.y + 6, "THE SHELF", TITLE);

        // RELOAD, in the header. The rack is read off a directory, so it can
        // change without the game touching it.
        let reload_w = cell_width(hud::text_width("RELOAD"), "", PAD);
        widgets.push(Widget {
            rect: Rect { x: right - reload_w, y: rect.y + 3, w: reload_w, h: 11 },
            line1: "RELOAD".into(),
            line2: String::new(),
            action: Some(Action::ShelfReload),
            latched: false,
            icon: None,
            ratio: None,
            note: "RE-READ THE SHELF DIRECTORY. JARS ARE FILES IN ASSETS/SHELF, SO YOU CAN COPY ONE IN FROM ANOTHER RUN OR ANOTHER MACHINE AND IT WILL BE HERE.".into(),
        });
        for x in rect.x + 1..rect.right() - 1 {
            render::put(frame, W, H, x, rect.y + PAGE_HEADER - 4, DIVIDER);
        }

        // ---- the dial strip, and the three verbs that act on the armed jar.
        let ty = rect.y + PAGE_HEADER;
        let step_w = cell_width(hud::text_width("W"), "", PAD);
        let dial = self.brood_label();
        text(frame, left, ty + 2, "DRIFT", SUB_ON);
        let dial_x = left + hud::text_width("DRIFT") + 6;
        for (dx, label, sign) in [(0, "-", -1), (step_w + 2 + hud::text_width("8 BROODS") + 6, "+", 1)] {
            widgets.push(Widget {
                rect: Rect { x: dial_x + dx, y: ty, w: step_w, h: 11 },
                line1: label.into(),
                line2: String::new(),
                action: Some(Action::Broods(sign)),
                latched: false,
                icon: None,
                ratio: None,
                note: "HOW FAR A RELEASE DRIFTS FROM THE JAR, COUNTED IN BROODS. ZERO IS THAT EXACT INDIVIDUAL AGAIN. ONE IS AS DIFFERENT AS ITS OWN CHILD WOULD HAVE BEEN -- IT IS THE SAME MUTATION THE ENGINE APPLIES AT A BIRTH, APPLIED ONCE PER BROOD. NOTHING HERE IS A SEPARATE RATE YOU HAVE TO CALIBRATE.".into(),
            });
        }
        text(frame, dial_x + step_w + 4, ty + 2, &dial, if self.broods == 0 { VALUE } else { EDGE_ON });

        let armed = self.armed_jar().map(|j| (j.name.clone(), j.species.clone(), j.genetics.kingdom()));
        let mut vx = right;
        for (label, action, note_text) in [
            (
                "PROMOTE",
                Action::ShelfPromote,
                "WRITE THE ARMED JAR OUT AS A WHOLE SPECIES FILE IN ASSETS/SPECIES, SO THE ANIMAL BECOMES ONE OF THE GAME'S OWN AND CAN BE PLANTED BY NAME. THIS IS THE WAY OUT OF THE LAB. CREATURES ONLY SO FAR, AND IT ALSO NEEDS A MATERIAL OF THE SAME NAME BEFORE IT WILL HATCH -- WHAT A NEW CREATURE LOOKS LIKE IS A THING TO DRAW, NOT TO GENERATE.",
            ),
            (
                "DISCARD",
                Action::ShelfDiscard,
                "TAKE THE ARMED JAR OFF THE SHELF FOR GOOD. NOTHING ELSE IN THE LAB DELETES A JAR -- KEEPING ONE NEVER OVERWRITES ANOTHER -- SO THIS IS THE ONLY WAY A SPECIMEN IS LOST.",
            ),
            // **`COPY`, not `DRIFT`.** The dial to its left is already
            // labelled `DRIFT`, and a rendered page with the same word as a
            // noun on one side and a verb on the other read as one control
            // that had been drawn twice. Caught by looking at the sheet;
            // nothing in the layout could have said so.
            (
                "COPY",
                Action::ShelfDrift,
                "PUT A COPY OF THE ARMED JAR ON THE SHELF, DRIFTED BY THE DIAL ON THE LEFT. THE ORIGINAL STAYS ARMED, SO YOU CAN MAKE SEVERAL SIBLINGS FROM ONE PARENT. THIS IS HOW A LINE IS BRED WITHOUT EVER RELEASING IT -- THE NEW JAR RECORDS WHICH ONE IT CAME FROM AND HOW FAR.",
            ),
            // **Last in the list is leftmost on screen**, because the strip
            // is laid out right to left -- so this reads first, which is what
            // it deserves: the other three are what you do *to* a jar and
            // this is the one that puts it back in the world.
            //
            // It arms rather than places, and that is not a compromise: a jar
            // has to go *somewhere*, and the page it is on is covering most
            // of the box. So the button closes the rack and hands the next
            // world click to the specimen -- which is the `FREE` tool's
            // aiming, kept, with its button on the bar given up.
            (
                "PLACE",
                Action::ShelfPlace,
                "PUT THE ARMED JAR BACK IN THE BOX. THIS CLOSES THE RACK AND ARMS THE PLACING -- THE NEXT CLICK IN THE BOX IS WHERE IT GOES. AT 0 BROODS IT IS THAT EXACT INDIVIDUAL AGAIN; AT 1 IT IS AS DIFFERENT AS ITS OWN CHILD WOULD HAVE BEEN. A PLANT ARRIVES AS A SEED THAT STILL HAS TO GERMINATE; AN ANIMAL ARRIVES ALIVE.",
            ),
        ] {
            let bw = cell_width(hud::text_width(label), "", PAD);
            vx -= bw;
            widgets.push(Widget {
                rect: Rect { x: vx, y: ty, w: bw, h: 11 },
                line1: label.into(),
                line2: String::new(),
                action: Some(action),
                latched: false,
                icon: None,
                ratio: None,
                note: match &armed {
                    Some((name, ..)) => format!("{note_text} ARMED: {}.", name.to_uppercase()),
                    None => format!("{note_text} NOTHING IS ARMED YET -- CLICK A JAR BELOW."),
                },
            });
            vx -= 2;
        }

        // ---- the rack.
        let mut y = rect.y + PAGE_HEADER + PARAM_TABS;
        if count == 0 {
            // **The empty state carries the instruction.** A player who
            // opens this page first has no way to guess that the shelf is
            // filled by a tool on the bar rather than by a button here.
            text(frame, left, y + 2, "NOTHING KEPT YET.", FAINT);
            text(frame, left, y + 12, "CLICK A PLANT OR AN ANT WITH LOOK", FAINT);
            text(frame, left, y + 22, "(Z), THEN PRESS KEEP ON THAT PAGE.", FAINT);
            y += SHELF_ROW * shown.max(1) as i32;
        }
        for (i, jar) in self.shelf.iter().take(SHELF_ROWS).enumerate() {
            let hovered = self.cursor.is_some_and(|(cx, cy)| (rect.x..rect.right()).contains(&cx) && (y..y + SHELF_ROW).contains(&cy));
            let selected = self.shelf_selected == Some(i);
            if hovered || selected {
                fill(
                    frame,
                    Rect { x: rect.x + 1, y, w: rect.w - 2, h: SHELF_ROW },
                    if selected { [40, 52, 44, 255] } else { [34, 40, 52, 255] },
                );
            }
            let name = jar.name.to_uppercase();
            text(frame, left, y + 1, &name, if selected { LABEL_ON } else { LABEL });
            let species = jar.species.to_uppercase();
            text(frame, right - SHELF_RIGHT + 4, y + 1, &species, if selected { SUB_ON } else { FAINT });
            // **Generation, because it is the number that says whether a jar
            // is a founder you kept or a descendant you selected for** — the
            // one thing about a specimen that a name cannot carry and that
            // the player chose.
            let gen = format!("G{}", jar.taken.generation);
            text(frame, right - hud::text_width(&gen), y + 1, &gen, FAINT);
            let note_text = jar_note(jar);
            if hovered {
                note = Some((note_text.clone(), rect, y));
            }
            widgets.push(Widget {
                rect: Rect { x: rect.x + 1, y, w: rect.w - 2, h: SHELF_ROW },
                line1: String::new(),
                line2: String::new(),
                action: Some(Action::ShelfSelect(i)),
                latched: false,
                icon: None,
                ratio: None,
                note: note_text,
            });
            y += SHELF_ROW;
        }

        // ---- the footer: how much of the rack is showing, and what was
        // unreadable. Both are counts rather than silence, for the reason
        // every verb on this bar says what it did.
        let mut footer = match count {
            0 => String::new(),
            n if n > SHELF_ROWS => format!("{SHELF_ROWS} OF {n} -- THE REST ARE IN THE DIRECTORY"),
            n => format!("{n} KEPT"),
        };
        if self.shelf_skipped > 0 {
            footer.push_str(&format!("  {} UNREADABLE", self.shelf_skipped));
        }
        if !footer.is_empty() {
            text(frame, left, y + 3, &footer, FAINT);
        }

        self.shelf_bar = Bar { widgets, dividers: Vec::new() };
        for wid in &self.shelf_bar.widgets {
            // The invisible row strip is a hit target and nothing else --
            // painting it would put a second highlight over the row's own.
            if wid.line1.is_empty() {
                continue;
            }
            let hover = self.cursor.is_some_and(|(x, y)| wid.rect.contains(x, y));
            let down = hover && self.pressed.is_some() && self.pressed == wid.action;
            paint_widget(frame, wid, hover, down);
        }
        if let Some(wid) = self.shelf_bar.hovered(self.cursor).filter(|w| !w.note.is_empty()) {
            note = Some((wid.note.clone(), rect, wid.rect.y));
        }
        note
    }

    fn paint_params(&mut self, frame: &mut [u8], world: &World, spec: &LabBox) -> Option<(String, Rect, i32)> {
        let list = self.page_params(world, spec);
        let lines = param_lines(&list);
        let mut widgets: Vec<Widget> = Vec::new();
        let mut note: Option<(String, Rect, i32)> = None;

        // Clamp the scroll against the list as it is now. A page whose content
        // changed under a stored offset would otherwise open blank, which
        // reads as "this page has nothing on it".
        let max_scroll = lines.len().saturating_sub(PARAM_ROWS);
        if self.param_scroll > max_scroll {
            self.param_scroll = max_scroll;
        }
        let first = self.param_scroll;
        let shown: Vec<&Line> = lines.iter().skip(first).take(PARAM_ROWS).collect();
        let paged = lines.len() > PARAM_ROWS;

        let w = param_page_width();
        let content: i32 = shown.iter().map(|l| l.height()).sum();
        let h = PAGE_HEADER + PARAM_TABS + content + if paged { PARAM_TABS } else { 0 } + PAGE_PAD;
        let bottom = bar_top() - 4;
        let rect = Rect { x: MARGIN, y: (bottom - h).max(MARGIN), w, h };
        self.params_box = Some(rect);

        fill(frame, rect, PANEL_BG);
        outline(frame, rect, PANEL_EDGE);
        let left = rect.x + PAGE_PAD;
        let right = rect.right() - PAGE_PAD;
        text(frame, left, rect.y + 6, "PARAMETERS", TITLE);

        // SAVE, in the header, acting on whatever row was last touched. Verb
        // on the button and the object named beside it, so a press cannot mean
        // something the screen did not say.
        let save_w = cell_width(hud::text_width("SAVE"), "", PAD);
        let save = Rect { x: right - save_w, y: rect.y + 3, w: save_w, h: 11 };
        let armed = self.param_selected.and_then(|i| list.get(i));
        widgets.push(Widget {
            rect: save,
            line1: "SAVE".into(),
            line2: String::new(),
            action: Some(Action::ParamSave),
            latched: false,
            icon: None,
            ratio: None,
            note: match armed {
                Some(p) => format!(
                    "WRITE {} = {} BACK INTO ITS ASSET FILE, SO IT SURVIVES THE NEXT RUN. THE EDIT REPLACES THAT ONE FIELD AND NOTHING ELSE -- COMMENTS ARE KEPT -- AND THE FILE IS PARSED BEFORE ANYTHING IS WRITTEN, SO A BAD EDIT IS REPORTED RATHER THAN SAVED.",
                    param_label(&p.tunable.name),
                    p.display()
                ),
                None => "WRITE A PARAMETER BACK INTO ITS ASSET FILE. NOTHING IS PICKED YET -- MOVE A ROW WITH ITS - OR + BUTTON, OR CLICK ITS NAME, AND THIS WILL SAVE THAT ONE.".to_string(),
            },
        });

        for x in rect.x + 1..rect.right() - 1 {
            render::put(frame, W, H, x, rect.y + PAGE_HEADER - 4, DIVIDER);
        }

        // The tab strip.
        let mut tx = left;
        let ty = rect.y + PAGE_HEADER;
        for (i, group) in params::GROUPS.iter().enumerate() {
            let tw = cell_width(hud::text_width(group.label()), "", PAD);
            widgets.push(Widget {
                rect: Rect { x: tx, y: ty, w: tw, h: 11 },
                line1: group.label().into(),
                line2: String::new(),
                action: Some(Action::ParamGroup(i)),
                latched: i == self.param_group,
                icon: None,
                ratio: None,
                note: group.note().into(),
            });
            tx += tw + 2;
        }

        // The rows.
        let mut y = rect.y + PAGE_HEADER + PARAM_TABS;
        for line in &shown {
            match line {
                Line::Head(name) => {
                    text(frame, left, y + 1, &param_label(name), SUB_ON);
                }
                Line::Row(i) => {
                    let Some(p) = list.get(*i) else { continue };
                    let hovered = self.cursor.is_some_and(|(cx, cy)| {
                        (rect.x..rect.right()).contains(&cx) && (y..y + PARAM_ROW).contains(&cy)
                    });
                    let selected = self.param_selected == Some(*i);
                    if hovered || selected {
                        let tint = if selected { [40, 52, 44, 255] } else { [34, 40, 52, 255] };
                        fill(frame, Rect { x: rect.x + 1, y, w: rect.w - 2, h: PARAM_ROW }, tint);
                    }
                    if hovered {
                        note = Some((p.note.clone(), rect, y));
                    }
                    // **The fill track under the name, not a separate column.**
                    // It says where in its own range this number sits, which
                    // is the question a bare figure cannot answer without
                    // reading the range column too — and it costs two pixel
                    // rows rather than forty of width.
                    let track = Rect { x: left, y: y + PARAM_ROW - 3, w: right - PARAM_RIGHT - left, h: 2 };
                    if p.writable() {
                        fill(frame, track, [30, 34, 42, 255]);
                        let filled = (track.w as f32 * p.fraction()).round() as i32;
                        if filled > 0 {
                            fill(frame, Rect { w: filled, ..track }, if selected { EDGE_ON } else { [70, 96, 122, 255] });
                        }
                    }
                    text(frame, left, y + 1, &param_label(&p.tunable.name), if selected { LABEL_ON } else { LABEL });

                    if p.writable() {
                        let minus = Rect { x: right - PARAM_MINUS, y: y + 1, w: PARAM_STEP_W, h: 9 };
                        let plus = Rect { x: right - PARAM_PLUS, y: y + 1, w: PARAM_STEP_W, h: 9 };
                        let value = p.display();
                        text(frame, right - PARAM_VALUE_RIGHT - hud::text_width(&value), y + 1, &value, VALUE);
                        let range = p.range();
                        text(frame, right - hud::text_width(&range), y + 1, &range, FAINT);
                        for (r, label, sign) in [(minus, "-", -1), (plus, "+", 1)] {
                            widgets.push(Widget {
                                rect: r,
                                line1: label.into(),
                                line2: String::new(),
                                action: Some(Action::ParamAdjust(*i, sign)),
                                latched: false,
                                icon: None,
                                ratio: None,
                                // Empty: the row's own note is the
                                // explanation, and a note on the button would
                                // replace it the moment you reached for the
                                // thing it was explaining.
                                note: String::new(),
                            });
                        }
                        // The name half of the row selects, so `SAVE` can be
                        // aimed without moving the value first.
                        widgets.push(Widget {
                            rect: Rect { x: rect.x + 1, y, w: right - PARAM_RIGHT - rect.x - 2, h: PARAM_ROW },
                            line1: String::new(),
                            line2: String::new(),
                            action: Some(Action::ParamSelect(*i)),
                            latched: false,
                            icon: None,
                            ratio: None,
                            note: p.note.clone(),
                        });
                    } else {
                        let value = p.display();
                        text(frame, right - hud::text_width(&value), y + 1, &value, FAINT);
                    }
                }
            }
            y += line.height();
        }

        // The pager, only when there is something off the page.
        if paged {
            let step_w = cell_width(hud::text_width("<"), "", PAD);
            let up = Rect { x: left, y: y + 2, w: step_w, h: 10 };
            let down = Rect { x: left + step_w + 2, y: y + 2, w: step_w, h: 10 };
            let last = (first + shown.len()).min(lines.len());
            text(frame, left + step_w * 2 + 8, y + 3, &format!("{}-{} OF {}", first + 1, last, lines.len()), FAINT);
            for (r, label, dir) in [(up, "<", -1), (down, ">", 1)] {
                widgets.push(Widget {
                    rect: r,
                    line1: label.into(),
                    line2: String::new(),
                    action: Some(Action::ParamScroll(dir)),
                    latched: false,
                    icon: None,
                    ratio: None,
                    note: "SCROLL THIS PAGE. THE PAGES ARE SHORT ON PURPOSE -- A PANEL WITH FOUR HUNDRED ROWS IN IT IS NOT ACCESS, IT IS A HAYSTACK.".into(),
                });
            }
        }

        // **Retain first, then paint from what was retained.** The bar does
        // this the same way round for the same reason: the rectangles a click
        // is tested against have to be the rectangles that were drawn, and a
        // paint loop over a list that is then thrown away is the second copy
        // of the layout this module exists to avoid.
        self.params_bar = Bar { widgets, dividers: Vec::new() };
        for wid in &self.params_bar.widgets {
            // The invisible select strip is a hit target and nothing else --
            // painting it would put a second highlight over the row's own.
            if wid.line1.is_empty() {
                continue;
            }
            let hover = self.cursor.is_some_and(|(x, y)| wid.rect.contains(x, y));
            let down = hover && self.pressed.is_some() && self.pressed == wid.action;
            paint_widget(frame, wid, hover, down);
        }
        // A chip's note wins over the row's: the cursor can only be over one
        // of them, and the chip is the smaller, more specific target.
        if let Some(wid) = self.params_bar.hovered(self.cursor).filter(|w| !w.note.is_empty()) {
            note = Some((wid.note.clone(), rect, wid.rect.y));
        }
        note
    }
}

/// Narrowest a hover explanation may be squeezed to. Below about fifteen
/// columns a sentence becomes a vertical strip of words, which is unreadable
/// in a different way from being covered up.
const NOTE_MIN_WIDTH: i32 = 96;

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
    // **Sized to the gap it is going into, not to a constant.** At a fixed
    // 210 the parameters page — the widest thing that opens here — left 209
    // pixels beside it, so the box missed by *one pixel*, fell through to the
    // left edge, and was drawn straight over the page it was explaining. That
    // is precisely the failure this function exists to avoid, arriving through
    // its own fallback; a contact sheet showed it and no test could have.
    let gap = match place {
        Note::AboveBar => W as i32 - MARGIN * 2,
        Note::BesidePage => (W as i32 - MARGIN - (avoid.right() + 6)).max(avoid.x - 6 - MARGIN),
    };
    let width = 210.min(gap).min(W as i32 - MARGIN * 2).max(NOTE_MIN_WIDTH);
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

        // **The brush, and what it will cover, before you press.** A radius
        // you can only discover by painting is a radius you discover by
        // ruining something. Drawn in world space through
        // `world_rect_to_screen`, so it is the right size at every zoom rather
        // than the right number of *screen* pixels at one of them.
        if let Some((cx, cy)) = self.cursor.filter(|&(_, y)| y < bar_top()) {
            let (wx, wy) = renderer.screen_to_world(cx, cy);
            let r = if self.tool.is_brush() { self.brush } else { 1 };
            let (x0, _, x1, _, _) = renderer.world_rect_to_screen(wx - r, wy - r, wx + r, wy + r);
            let screen_r = ((x1 - x0) / 2).max(2);
            if self.tool != Tool::Look {
                render::draw_circle_outline(frame, W, H, cx, cy, screen_r, TOOL_RING);
            }
        }

        let mut note: Option<(String, Rect, i32, Note)> = None;

        if self.panel == Some(Panel::Params) {
            // Its own painter: the rows carry buttons, so they are not `Row`s
            // and the page is not a `paint_page`.
            if let Some((body, avoid, y)) = self.paint_params(frame, world, spec) {
                note = Some((body, avoid, y, Note::BesidePage));
            }
            self.panel_box = None;
            self.shelf_box = None;
            self.shelf_bar = Bar::default();
        } else if self.panel == Some(Panel::Chambers) {
            // Its own painter, for `Params`' and `Shelf`'s reason: a row here
            // is a chamber with two verbs attached, not a label.
            if let Some((body, avoid, y)) = self.paint_rack(frame, state.chambers, state.rack_thumb, state) {
                note = Some((body, avoid, y, Note::BesidePage));
            }
            self.panel_box = None;
            self.params_box = None;
            self.params_bar = Bar::default();
            self.shelf_box = None;
            self.shelf_bar = Bar::default();
        } else if self.panel == Some(Panel::Shelf) {
            // The same deal, and the same reason: a jar row is a verb.
            if let Some((body, avoid, y)) = self.paint_shelf(frame) {
                note = Some((body, avoid, y, Note::BesidePage));
            }
            self.panel_box = None;
            self.params_box = None;
            self.params_bar = Bar::default();
            self.rack_box = None;
            self.rack_bar = Bar::default();
        } else if let Some(panel) = self.panel {
            self.params_box = None;
            self.params_bar = Bar::default();
            self.shelf_box = None;
            self.shelf_bar = Bar::default();
            self.rack_box = None;
            self.rack_bar = Bar::default();
            let rows = self.panel_rows(panel, world, spec, fps);
            // Anchored under the button that opened it, which is only
            // available because `Lab::act` closes the biosphere page when one
            // of these opens — see the note there. Both pages drawn at once
            // interleave into something neither of them is, and a contact
            // sheet with both open was the only thing that showed it.
            let anchor = self
                .bar
                .widgets
                .iter()
                .find(|wid| wid.action == Some(Action::Panel(panel)))
                .map_or(MARGIN, |wid| wid.rect.x);
            let rect = page_rect(&rows, anchor, bar_top() - 4);
            self.panel_box = Some(rect);
            if let Some((text, y)) = paint_page(frame, rect, panel.title(), &rows, self.cursor, &mut Vec::new()) {
                note = Some((text, rect, y, Note::BesidePage));
            }
        } else {
            self.panel_box = None;
            self.params_box = None;
            self.params_bar = Bar::default();
            self.shelf_box = None;
            self.shelf_bar = Bar::default();
        }

        if let Some(at) = self.inspect {
            let rows = self.inspect_rows(world, at);
            // Beside the open page rather than under it, so opening a page
            // does not hide the cell you are inspecting.
            let anchor = self.panel_box.or(self.params_box).or(self.shelf_box).map_or(MARGIN, |r| r.right() + 6);
            let rect = page_rect(&rows, anchor, bar_top() - 4);
            self.inspect_box = Some(rect);
            // The group headings are hit targets, and they are collected by
            // the paint loop rather than measured again afterwards: a second
            // pass over the same arithmetic is how a label and the thing it
            // clicks come to disagree about which row they are (this is
            // `paint_page`'s own hover rule, and the reason it is stated
            // there).
            let mut taps: Vec<Widget> = Vec::new();
            if let Some((text, y)) = paint_page(frame, rect, "CELL", &rows, self.cursor, &mut taps) {
                note = Some((text, rect, y, Note::BesidePage));
            }
            // **`KEEP`, in the header, and only while there is something to
            // keep.** This is the `KEEP` tool's whole job, moved to where the
            // player already is: the page is open *on* one individual, so the
            // click that used to aim the tool is a click the interface had
            // already been given. A button that is absent over bare soil
            // rather than present and refusing, because the page is re-read
            // every frame and a permanently-greyed control on a page that
            // changes under you reads as broken.
            if let Some(button) = self.keep_button(world, at, rect) {
                let hover = self.cursor.is_some_and(|(x, y)| button.rect.contains(x, y));
                let down = hover && self.pressed == Some(Action::KeepInspected);
                if hover && !button.note.is_empty() {
                    note = Some((button.note.clone(), rect, rect.y, Note::BesidePage));
                }
                paint_widget(frame, &button, hover, down);
                taps.push(button);
            }
            self.inspect_bar = Bar { widgets: taps, dividers: Vec::new() };
        } else {
            self.inspect_box = None;
            self.inspect_bar = Bar::default();
        }

        // The bar itself, over everything a page drew, because a page opens
        // *above* it and the two must not fight over the seam.
        let bar = Rect { x: 0, y: bar_top(), w: W as i32, h: BAR_HEIGHT };
        fill(frame, bar, BAR_BG);
        for x in 0..W as i32 {
            render::put(frame, W, H, x, bar.y, BAR_EDGE);
        }
        for (dx, row) in &self.bar.dividers {
            // Inside that row's own band. A rule the full height of a two-row
            // bar separates controls that have nothing to do with each other.
            let top = row_y(*row) + 2;
            for y in top..top + BTN_HEIGHT - 4 {
                render::put(frame, W, H, *dx, y, DIVIDER);
            }
        }
        for wid in &self.bar.widgets {
            let hover = self.cursor.is_some_and(|(x, y)| wid.rect.contains(x, y));
            let down = hover && self.pressed.is_some() && self.pressed == wid.action;
            paint_widget(frame, wid, hover, down);
        }

        // **The rack's tabs.** Laid out here rather than in `layout` because
        // the strip is not part of the bar — the bar has 0-1 pixels spare and
        // cannot take another cell at any spacing (see `tab_strip_y`). Drawn after
        // the bar so that `AboveBar` sits against its top edge rather than
        // under it.
        self.tabs = lay_out_tabs(state.chambers, tab_strip_y(state.chambers.len()));
        {
            let y = tab_strip_y(state.chambers.len());
            fill(frame, Rect { x: 0, y, w: W as i32, h: TAB_H }, BAR_BG);
            // The rule along the strip's own top, so the strip and the bar
            // read as one panel rather than two stacked ones.
            for x in 0..W as i32 {
                render::put(frame, W, H, x, y, BAR_EDGE);
            }
            for wid in &self.tabs.widgets {
                let hover = self.cursor.is_some_and(|(x, y)| wid.rect.contains(x, y));
                let down = hover && self.pressed.is_some() && self.pressed == wid.action;
                paint_widget(frame, wid, hover, down);
            }
        }
        // A bar button explains itself too, and its note wins over a page's:
        // the cursor can only be over one of them, and the bar is on top.
        if let Some(wid) = self.bar.hovered(self.cursor) {
            if !wid.note.is_empty() {
                note = Some((wid.note.clone(), wid.rect, wid.rect.y - 4, Note::AboveBar));
            }
        }

        // **What is under the pointer, always, with no mode to turn on.**
        // Owner request, 2026-08-30: *"the info option that tells me what
        // material, temp, etc that my mouse is hovering over."* Docked rather
        // than following the cursor, for `draw_note`'s reason — a box under
        // the pointer covers the thing it is describing.
        //
        // **Left column, under the clock, and it was top right first.** The
        // biosphere page is a full-height *right-hand* column and `Lab::draw`
        // paints it after this, so a top-right box spends its life half
        // hidden — which a contact sheet showed and no test could have. The
        // left column is free below the clock.
        if let Some((cx, cy)) = self.cursor.filter(|&(x, y)| y < bar_top() && !self.covers(x, y)) {
            let (wx, wy) = renderer.screen_to_world(cx, cy);
            paint_hover_cell(frame, world, (wx, wy));
        }

        // The last verb's notice, over everything, just above the bar.
        if let Some((text, at)) = &self.notice {
            if at.elapsed().as_secs_f32() < NOTICE_SECONDS {
                let w = hud::text_width(text) + 12;
                let r = Rect {
                    x: ((W as i32 - w) / 2).max(MARGIN),
                    y: bar_top() - 16,
                    w: w.min(W as i32 - MARGIN * 2),
                    h: 13,
                };
                fill(frame, r, NOTE_BG);
                outline(frame, r, MARKER);
                text_at(frame, r.x + 6, r.y + 3, text, MARKER);
            }
        }

        if let Some((body, avoid, y, place)) = note {
            draw_note(frame, &body, avoid, y, place);
        }
    }
}

/// **What is in the cell under the pointer**, docked top right.
///
/// Four lines, always four, present or absent — a readout that changes height
/// as the cursor moves is one you cannot read while moving. Every line names
/// the accessor it came from:
///
/// - the material's `display` name and the cell's world coordinates;
/// - `Cell::temperature`, this cell's own, not the coarse field's;
/// - **fill for a liquid, held water for a powder** — the two conventions
///   point opposite ways (`CLAUDE.md`: on a `Liquid` `aux == 0` is *full*, on
///   a `Powder` it is *dry*), so this goes through `update::liquid_fill` for
///   the first rather than reading `aux` and getting it backwards;
/// - the organism owning the cell, and its whole-body energy.
fn paint_hover_cell(frame: &mut [u8], world: &World, (x, y): (i32, i32)) {
    use crate::sim::material::MaterialKind;
    let cell = world.get(x, y);
    let def = world.materials.get(cell.material);
    let wet = match world.materials.kind(cell.material) {
        MaterialKind::Liquid => {
            let fill = crate::sim::update::liquid_fill(cell);
            format!("FILL {}%", fill as u32 * 100 / crate::sim::material::LIQUID_FULL.max(1) as u32)
        }
        _ if def.water_capacity > 0 => format!(
            "WATER {}%",
            cell.aux().min(def.water_capacity) as u32 * 100 / def.water_capacity.max(1) as u32
        ),
        _ => "DRY".to_string(),
    };
    let organism = world.organism(cell.organism_id());
    let life = match organism {
        Some(state) => format!(
            "{} E{:.0}",
            world.species.get(state.species).name.to_uppercase(),
            state.energy
        ),
        None => "NO ORGANISM".to_string(),
    };
    let lines = [
        format!("{} {},{}", def.display.to_uppercase(), x, y),
        format!("{}C{}", cell.temperature(), if cell.is_burning() { " BURNING" } else { "" }),
        wet,
        life,
    ];
    let w = lines.iter().map(|l| hud::text_width(l)).max().unwrap_or(0) + 12;
    let r = Rect { x: MARGIN, y: HOVER_TOP, w, h: lines.len() as i32 * LINE + 8 };
    fill(frame, r, READOUT_BG);
    outline(frame, r, PANEL_EDGE);
    for (i, line) in lines.iter().enumerate() {
        let tint = if i == 0 { VALUE } else if organism.is_some() && i == 3 { GOOD } else { FAINT };
        text_at(frame, r.x + 6, r.y + 4 + i as i32 * LINE, line, tint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect as WorldRect;

    fn state(running: bool, requested: u32) -> BarState<'static> {
        BarState {
            running,
            requested,
            achieved: 6.4,
            presets: &super::super::time::PRESETS,
            panel: None,
            stats: true,
            help: false,
            tool: Tool::Look,
            species: "HERB",
            species_note: "WHICH PLANT THE PLANTING TOOL PUTS IN.",
            brush: 6,
            overlay: "OFF",
            jar: "NO JARS",
            jar_note: "THE RACK OF KEPT GENETICS: EVERY JAR IS ONE INDIVIDUAL'S GENOME.",
            // A rack of one, which is what the lab opens with and what the
            // bar's own fit test must be measured against: the tab strip is
            // not drawn below two chambers, so it can never be the thing that
            // makes the bar not fit.
            chambers: &[],
            rack_thumb: None,
            batch: super::super::BatchBar { copies: 8, frames: 9_000, progress: None },
        }
    }

    fn world() -> World {
        World::new(WorldRect::new(0, 0, 63, 63))
    }

    fn rack(n: usize) -> Vec<super::super::ChamberSummary> {
        (0..n)
            .map(|i| super::super::ChamberSummary {
                on_record: false,
                rebuilding: false,
                index: i,
                active: i == 0,
                label: format!("{}", i + 1),
                seed: i as u64 + 1,
                frame: 0,
                census: None,
            })
            .collect()
    }

    /// **A sort reorders what is drawn and must not re-aim what is clicked.**
    ///
    /// The bug this is named for was one line from shipping: the row loop
    /// aimed its verbs with the loop's own position, which equals the
    /// chamber's index right up until a sort moves a row. After that, clicking
    /// the top row opens whichever chamber happens to be *first in the rack*
    /// — so the more useful the sort, the more wrong the click.
    ///
    /// Built so the two orders genuinely differ: seeds ascend with the index
    /// and plant counts descend against it, so sorting on plants reverses the
    /// list. A fixture where they agreed would pass with the bug in place.
    #[test]
    fn sorting_the_rack_reorders_the_rows_without_re_aiming_the_verbs() {
        const N: usize = 5;
        let mut chambers = rack(N);
        for (i, ch) in chambers.iter_mut().enumerate() {
            // Descending against the index, so the sorted order is reversed.
            ch.census = Some(super::super::stats::Census { plants: (N - i) * 10, ..Default::default() });
        }

        let mut page = Ui::new();
        let st = state(false, 1);
        let mut buf = vec![0u8; (W * H * 4) as usize];

        // Column 2 is PLT. Descending puts the *largest* first, which is
        // chamber 0 here, so sort ascending to actually reverse the list.
        page.sort_chambers(2);
        page.sort_chambers(2);
        assert_eq!(page.rack_sort, Some((2, false)), "a second click on one column reverses it");
        page.paint_rack(&mut buf, &chambers, None, &st);

        // The rows are bands with no face; their action names the chamber.
        let aimed: Vec<usize> = page
            .rack_bar
            .widgets
            .iter()
            .filter(|w| w.line1.is_empty())
            .filter_map(|w| match w.action {
                Some(Action::ChamberSelect(i)) => Some(i),
                _ => None,
            })
            .collect();
        assert_eq!(
            aimed,
            vec![4, 3, 2, 1, 0],
            "ascending on PLT must draw the smallest first AND aim each row at its own chamber; \
             {aimed:?} in rack order would mean the verbs are aimed at screen positions"
        );
    }

    /// **A rack of one draws no strip, and a rack of two does.**
    ///
    /// Both halves, at every style. The first alone is green for a strip that
    /// never draws at all — which is the failure that matters, because the lab
    /// opens on one chamber and a strip that is broken there looks exactly
    /// like a strip that is correctly absent.
    #[test]
    fn a_rack_of_one_draws_no_tabs_and_a_rack_of_two_does() {
        // **The way into the rack is reachable at every rack size, and one is
        // the size that matters.** This replaces a guard that asserted the
        // opposite — that no strip is drawn below two chambers — which was
        // the bug rather than the behaviour: `ALL` lives in the strip and is
        // the only route to the page where chambers are *made*, so hiding it
        // at one chamber made a second chamber unreachable. The lab opens on
        // one. Reported by the owner as "I don't see it in the menu".
        for n in [1usize, 2, 5, 50] {
            let bar = lay_out_tabs(&rack(n), tab_strip_y(n));
            let all = bar.widgets.iter().find(|w| w.action == Some(Action::Panel(Panel::Chambers)));
            assert!(all.is_some(), "a rack of {n} has no way into the rack page");
            assert!(
                bar.widgets.iter().any(|w| w.action == Some(Action::Chamber(0))),
                "a rack of {n} draws no tab for its first chamber"
            );
        }
    }

    /// **The strip never overlaps the bar, and never leaves the screen.**
    ///
    /// A strip that ran under the bar would be invisible and still take the
    /// clicks aimed at the bar's top row — a control stealing another
    /// control's presses, which is the sort of thing only a screenshot ever
    /// finds.
    #[test]
    fn the_tab_strip_clears_the_bar_and_the_screen() {
        let y = tab_strip_y(5);
        assert!(y + TAB_H <= bar_top(), "the strip ran into the bar: {y} + {TAB_H} > {}", bar_top());
        assert!(y >= 0, "the strip ran off the top of the screen");
    }

    /// **A tab is clickable where it is drawn.**
    ///
    /// Aimed through the retained layout rather than at where a tab "ought to
    /// be", for `widget_rect`'s reason: a test that computes the rectangle
    /// itself is a test of its own arithmetic. The last tab is checked as well
    /// as the first, because an off-by-one in the advance only shows at the
    /// end of the strip.
    #[test]
    fn every_tab_is_clickable_where_it_is_drawn() {
        let chambers = rack(5);
        let y = tab_strip_y(chambers.len());
        let bar = lay_out_tabs(&chambers, y);
        assert_eq!(bar.widgets.len(), 6, "five chambers, five tabs, and the way into the rack");
        for (i, wid) in bar.widgets.iter().take(5).enumerate() {
            let (cx, cy) = (wid.rect.x + wid.rect.w / 2, wid.rect.y + wid.rect.h / 2);
            assert_eq!(bar.hit(cx, cy), Some(Action::Chamber(i)), "tab {i} is not clickable at its own centre");
            assert!(wid.rect.right() <= W as i32 - MARGIN, "tab {i} ran off the right edge");
        }
        assert!(bar.widgets[0].latched, "the chamber on screen is not marked as such");
    }

    /// **A rack longer than the strip says so.**
    ///
    /// The owner's call is five tabs with the rest behind the menu, and a
    /// batch produces fifty — so the case that matters is the one where the
    /// strip is *not* the whole rack. A strip that silently stopped at five
    /// would tell you your rack is five chambers long.
    #[test]
    fn a_rack_past_five_shows_what_it_is_not_showing() {
        let chambers = rack(12);
        let y = tab_strip_y(chambers.len());
        let bar = lay_out_tabs(&chambers, y);
        assert_eq!(bar.widgets.len(), TABS_SHOWN + 1, "five tabs and the way into the rest");
        let all = bar.widgets.last().expect("the ALL button");
        assert_eq!(all.line1, "ALL +7", "it must count the chambers the strip is not showing");
        assert_eq!(all.action, Some(Action::Panel(Panel::Chambers)), "it is a verb, not a label -- and not a switch to chamber 7");

        // And at five or fewer it is still there, still a verb, with nothing
        // to count: the rack page is where a chamber is closed, so a rack of
        // three must be able to reach it.
        let small = lay_out_tabs(&rack(3), tab_strip_y(3));
        let all = small.widgets.last().expect("the ALL button");
        assert_eq!(all.line1, "ALL");
        assert_eq!(all.action, Some(Action::Panel(Panel::Chambers)));
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
        // **The rack page's own strings**, including its column header —
        // which is how `#` was caught: it has no 5x7 glyph, `draw_text`
        // renders it as a silent blank, and the header read `  SEED` in the
        // first contact sheet. `CLAUDE.md` records that trap as having
        // shipped three times, and this page had no guard over it because it
        // paints itself rather than going through `panel_rows`.
        {
            let mut page = Ui::new();
            page.select_chamber(0);
            let mut buf = vec![0u8; (W * H * 4) as usize];
            let world = world();
            let spec = LabBox::default();
            let st = state(false, 1);
            let _ = (&world, &spec);
            page.paint_rack(&mut buf, &rack(7), None, &st);
            for wid in &page.rack_bar.widgets {
                check(&wid.line1, "rack button");
                check(&wid.note, "rack explanation");
            }
            for wid in &lay_out_tabs(&rack(7), tab_strip_y(7)).widgets {
                check(&wid.line1, "tab face");
                check(&wid.note, "tab explanation");
            }
            check(Panel::Chambers.title(), "rack title");
            // The header and the never-censused row, neither of which is a
            // widget and so neither of which the loops above reach.
            for literal in RACK_LITERALS {
                check(literal, "rack literal");
            }
        }

        // **The parameters page's own rows, and this was a blind spot.**
        //
        // Its 42 knobs each carry a hover note and **none of them was
        // covered**: proven by injecting `~#~` into one and watching this
        // test stay green. `CLAUDE.md` is explicit that a guard which does
        // not go red for the fault it is named for is not weak but blind, and
        // is to be replaced rather than argued with. The page is reached
        // through `params::registry` rather than `panel_rows`, which is how
        // it slipped past.
        //
        // It found a live defect immediately: the bed notes had been given
        // markdown emphasis, and the 5x7 set has **no `*`** -- it would have
        // drawn as gaps mid-sentence.
        {
            let w = world();
            let sp = LabBox::default();
            // With a plant selected and without, because the plant rows are
            // only built when one is — half the page is otherwise unreached.
            // The same accessor the page itself uses, so the guard covers
            // the rows a player actually sees rather than a second guess at
            // which species is selected.
            for plant in [None, Ui::new().species_of(&w)] {
                for p in params::registry(&w, &sp, plant) {
                    check(&param_label(&p.tunable.name), "parameter name");
                    check(&p.tunable.category.to_uppercase(), "parameter category");
                    check(&p.note, "parameter explanation");
                    if let Some(shown) = &p.shown {
                        check(shown, "parameter value");
                    }
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
                    Body::Head { label, .. } => check(label, "group heading"),
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
                assert!(
                    bar.fits(),
                    "the bar does not fit the screen at {} stops",
                    super::super::time::PRESETS.len()
                );
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
                // **A rectangle intersection, not an x-range one.** The
                // x-range form was correct for a one-row bar and is a false
                // alarm for two: every tool on the top row shares its columns
                // with the transport under it, which is the layout rather than
                // a fault. It is also *weaker* than it looks in the other
                // direction — it could not have caught two rows overlapping.
                for (i, a) in bar.widgets.iter().enumerate() {
                    for b in &bar.widgets[i + 1..] {
                        let apart = a.rect.right() <= b.rect.x
                            || b.rect.right() <= a.rect.x
                            || a.rect.bottom() <= b.rect.y
                            || b.rect.bottom() <= a.rect.y;
                        assert!(apart, "{:?} and {:?} overlap", a.line1, b.line1);
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
        // Row 0: one chip per tool, the species chip, two brush steps, the
        // overlay, the parameters page and the shelf. Row 1: three transport
        // buttons (the readout is not one), one chip per stop on the ladder,
        // and six pages. Written as the sum rather than as a literal so that
        // growing either list does not have to come here.
        assert_eq!(
            buttons,
            TOOLS.len() + 1 + 2 + 1 + 1 + 1 + 3 + super::super::time::PRESETS.len() + 6,
            "the bar carried {buttons} pressable buttons"
        );
        // Nothing above the bar is pressable — that belongs to the world.
        assert_eq!(bar.hit(10, bar_top() - 1), None);
    }

    /// **The single easiest thing on this bar to get backwards.** The face
    /// names the state the press will *produce*, not the state that is true —
    /// so it reads `RUN` while the box is stopped, beside a readout that says
    /// `PAUSED`. Verb on the button, state on the readout.
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
        assert_eq!(face(&running), "STOP", "a running box must offer to stop");
        // And the readout beside it names the state, so the two together are
        // unambiguous rather than each being half a sentence.
        // **The transport readout, not just "a widget with no action".** The
        // brush's `R6` chip is a readout too now, and it lays out first, so
        // the old predicate started answering for the wrong cell the day the
        // tools row arrived. The ratio strip is what makes this one unique.
        let readout = |bar: &Bar| {
            bar.widgets
                .iter()
                .find(|w| w.action.is_none() && w.ratio.is_some())
                .map(|w| w.line1.clone())
                .unwrap()
        };
        assert_eq!(readout(&tending), "PAUSED");
        assert!(readout(&running).starts_with("ASK"), "{}", readout(&running));
    }

    /// The achieved figure is the dial's honesty mechanism and it is also
    /// meaningless on a stopped box, where no tick runs at all. Shown while
    /// running, replaced by the state while paused — asserted both ways, since
    /// a readout that never printed it at all would pass the first half.
    #[test]
    fn the_achieved_figure_is_shown_running_and_withheld_while_paused() {
        // Keyed on the ratio strip, for the reason the sibling test records:
        // the brush size chip is also an actionless readout.
        let line2 = |running, requested| {
            layout(&state(running, requested))
                .widgets
                .iter()
                .find(|w| w.action.is_none() && w.ratio.is_some())
                .map(|w| w.line2.clone())
                .unwrap()
        };
        assert_eq!(line2(true, 64), "GOT 6.4X");
        assert_eq!(line2(false, 1), "NO TICKS");
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

    /// **The cell page fits on the screen, for something alive of each
    /// kingdom** -- and the plant arm proves the fold is what makes it fit.
    ///
    /// This is the guard that was missing, and its absence is
    /// `CLAUDE.md`'s *"check that a guard's inputs actually vary what it
    /// guards"* in its purest form. `a_page_sits_above_the_bar_and_inside_
    /// the_screen` covers `panel_rows` and only `panel_rows`; both
    /// `inspect_rows` sweeps aim at a bare cell of an **empty** test world,
    /// so they saw five rows where a plant has thirty. Every one of the three
    /// was green while the plant page was being drawn at y = -42 with its
    /// title and first four rows off the top of the screen.
    #[test]
    fn the_cell_page_fits_on_the_screen_for_a_plant_and_for_an_ant() {
        for (name, kingdom) in [("tree", "plant"), ("ant", "animal")] {
            let mut world = world();
            let species = world.species.id_of(name).unwrap_or_else(|| panic!("{name} must be a loaded species"));
            let id = world.push_organism(species).expect("a fresh world has organism slots free");
            let shoot = world.species.get(species).shoot_material.clone();
            let material = world.materials.id_of(&shoot).unwrap_or_else(|| panic!("{name}'s shoot material {shoot} must be loaded"));
            world.set(10, 10, crate::sim::cell::Cell::new(material, 0).with_organism_id(id));
            let ui = Ui::new();

            // The positive control, and it is the whole reason the plant arm
            // means anything: laid out flat this page has to be too tall, or
            // a fold that did nothing would pass.
            let sections = params::specimen_sections(&world, id);
            assert_eq!(sections.len(), 3, "{name}: every kingdom gets the same three groups");
            let flat = 5 * LINE + 4 + sections.len() as i32 * (LINE + 4) + sections.iter().map(|(_, _, r)| r.len() as i32 * LINE).sum::<i32>();
            let over = flat > page_content_budget();
            assert_eq!(over, kingdom == "plant", "{name}: flat page is {flat}px against a {}px budget -- the fold is being tested against the wrong kingdom", page_content_budget());

            let rows = ui.inspect_rows(&world, (10, 10));
            for anchor in [0, 200, W as i32 - 10] {
                let r = page_rect(&rows, anchor, bar_top() - 4);
                assert!(r.y >= MARGIN, "the {kingdom} cell page starts at y={} -- off the top of the screen", r.y);
                assert!(r.bottom() <= bar_top(), "the {kingdom} cell page runs into the bar");
                assert!(r.x >= 0 && r.right() <= W as i32, "the {kingdom} cell page runs off the side");
            }

            // **The fold is what fits it, not the backstop.** `fit_rows` will
            // trim anything to size, so a page that fits only because rows
            // were dropped is a page that has quietly stopped saying things.
            assert!(
                !rows.iter().any(|row| matches!(&row.body, Body::Value { value, .. } if value.ends_with("MORE"))),
                "the {kingdom} cell page fits only because rows were dropped"
            );
            let shut = rows.iter().filter(|row| matches!(row.body, Body::Head { open: false, .. })).count();
            assert_eq!(shut, if kingdom == "plant" { 1 } else { 0 }, "{name}: wrong number of groups folded away");

            // **A click opens what it says, whichever group it is.** The half
            // the fit assertions cannot reach: a rule that served the groups
            // strictly from the top would fit the page perfectly and ignore
            // the player, and every assertion above would still be green.
            for chosen in 0..3 {
                let mut ui = Ui::new();
                ui.show_specimen_section(chosen);
                let heads: Vec<bool> = ui
                    .inspect_rows(&world, (10, 10))
                    .iter()
                    .filter_map(|row| match row.body {
                        Body::Head { open, .. } => Some(open),
                        _ => None,
                    })
                    .collect();
                assert_eq!(heads.len(), 3, "{name}: the page lost a group heading");
                assert!(heads[chosen], "{name}: clicking group {chosen} did not open it");
            }
        }
    }

    /// **`fit_rows` drops rather than overflows, and says how much it
    /// dropped.** The backstop the guard above insists must stay unused, put
    /// under load on its own so that "unused" is a measurement rather than an
    /// assumption about a code path nobody has run.
    #[test]
    fn a_page_with_more_rows_than_the_screen_holds_is_trimmed_and_says_so() {
        let rows: Vec<Row> = (0..200).map(|i| Row::value(format!("ROW {i}"), "1", VALUE, "N")).collect();
        let budget = page_content_budget();
        let fitted = fit_rows(rows, budget);
        assert!(fitted.iter().map(Row::height).sum::<i32>() <= budget, "fit_rows returned rows taller than the budget it was given");
        let Some(Body::Value { label, value, .. }) = fitted.last().map(|r| &r.body) else {
            panic!("the trimmed page has no marker row")
        };
        assert_eq!(label, "...");
        let dropped: usize = value.trim_start_matches('+').trim_end_matches(" MORE").parse().expect("the marker names a count");
        assert_eq!(fitted.len() - 1 + dropped, 200, "the marker's count does not account for every row that went");
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

    /// **The ladder grew a seventh stop the same day this bar was written and
    /// pushed `REBUILD` off the right edge.** The bar now tightens rather than
    /// overflowing, and this is the guard over that — asserted for a ladder
    /// one stop longer than the real one, so the next stop is caught here
    /// rather than in a screenshot.
    #[test]
    fn the_bar_still_fits_a_longer_speed_ladder() {
        const LONGER: [u32; 8] = [1, 2, 4, 8, 16, 64, 256, 1024];
        let mut s = state(true, 64);
        s.presets = &LONGER;
        let bar = layout(&s);
        assert!(bar.fits(), "an eight-stop ladder does not fit");
        // ...and the sensitivity half: a ladder long enough that no spacing
        // can hold it must *report* that, rather than quietly dropping a
        // control or silently overlapping two.
        const ABSURD: [u32; 20] = [1; 20];
        let mut s = state(true, 1);
        s.presets = &ABSURD;
        assert!(!layout(&s).fits(), "a twenty-stop ladder claimed to fit");
    }

    /// Every stop on the ladder has a key printed under it. The caption used
    /// to come from a hand-written list of six; the seventh stop arrived and
    /// that chip had no caption and no key at all.
    #[test]
    fn every_speed_chip_shows_a_key() {
        let bar = layout(&state(false, 1));
        for wid in &bar.widgets {
            if let Some(Action::Preset(i)) = wid.action {
                assert_eq!(wid.line2, (i + 1).to_string(), "{:?} names the wrong key", wid.line1);
            }
        }
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
