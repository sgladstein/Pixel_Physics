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
use crate::sim::organism::SpeciesId;
use crate::sim::world::{self, World};

use super::params;
use super::plainspeak;
use super::roster;
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
/// Which batch dial a typed number is going into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypedField {
    Copies,
    Frames,
}

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
    /// Move the stocking count one stop along [`STOCK_LADDER`], `-1` or `+1`.
    /// Shares the brush's cells; see `layout`.
    Stock(i32),
    /// Cycle the false-colour view of the invisible channels.
    CycleOverlay,
    /// Cycle which colour every animal wears in the box -- see
    /// `render::CreatureColour`. The ANTS page's chart and legend group and
    /// tint by the same mode, through `Ui::set_creature_colour`'s mirror.
    CycleCreatureColour,
    Panel(Panel),
    Stats,
    Help,
    Reset,
    /// Show one page of the parameters panel — an index into
    /// [`params::GROUPS`].
    ParamGroup(usize),
    /// Scroll the parameters panel by one page, `-1` or `+1`.
    ParamScroll(i32),
    /// Scroll the rack page by one page, `-1` or `+1`.
    ///
    /// **The field existed and nothing moved it.** `rack_scroll` was written,
    /// clamped and honoured by the renderer from the day the page landed, so
    /// every guard over it passed -- and with no key, click or `Action` bound
    /// to it a rack of a hundred showed rows 1-12 and could reach no further.
    /// The owner found it by asking what a hundred copies would look like.
    /// `CLAUDE.md`'s standing warning about a channel with a writer and no
    /// reader, in its other direction: a reader nothing writes.
    RackScroll(i32),
    /// Collapse the rack to one row per swept setting, and back.
    RackGroup,
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
    /// Throw away every row one batch produced.
    ///
    /// **`CLEAR` and one-row `CLOSE` were the only two, and neither is what
    /// you want after a fifty-copy run.** Owner, 2026-09-01: *"you should be
    /// able to delete individual experiments or whole batches. Right now the
    /// only option is delete everything."*
    ChamberCloseBatch(u32),
    /// Run one row on for the number of ticks the TICKS dial holds.
    ChamberExtend(usize),
    /// Run every row of one batch on by the same amount.
    ChamberExtendBatch(u32),
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
    /// Start typing a number straight into one of the batch dials.
    ///
    /// **Because the dials cannot be driven to their own limits.** COPIES
    /// steps by one to 200 and TICKS by a thousand to 200,000 -- two hundred
    /// clicks each, which is not a control anybody would use to set up a long
    /// experiment. Owner, 2026-09-01: *"you should be able to type as it takes
    /// too long clicking + if you want to run a really long experiment"*.
    BatchType(TypedField),
    BatchCopies(i32),
    /// **Pin one individual from the roster** -- an index into the rows as
    /// they are currently sorted and filtered, which is what was on screen
    /// when the click landed.
    ///
    /// The index is resolved to an `Individual` *inside the same draw* and
    /// never stored. `Reports/dead-ends.md` records the shape: any selection
    /// stored as a position into a list a neighbouring verb rewrites has this
    /// bug, and a roster is rewritten by every birth, every death and every
    /// click on a column heading.
    RosterSelect(usize),
    /// Scroll the roster by one page, `-1` or `+1`.
    RosterScroll(i32),
    /// Sort the roster on one column; clicking the sorted column reverses it.
    RosterSort(usize),
    /// Cycle the roster's filter: everything, only what is in trouble, only
    /// the pinned individual's founding line.
    RosterFilter,
    /// Keep the camera on the pinned individual. The other half of the
    /// marker: a marker says *where it is*, following says *watch this one*,
    /// and at play zoom an ant is two dark cells.
    RosterFollow,
    /// Let the pinned individual go.
    RosterRelease,
    /// Spare the pinned individual, or stop sparing it. The set is what
    /// `RosterCullRest` keeps.
    RosterSpare,
    /// Cull the pinned individual.
    RosterCull,
    /// Cull every individual in this table that is not spared.
    RosterCullRest,
    /// Hold the pinned individual as the thing to compare against, or --
    /// when one is already held and the pin has moved -- open the
    /// comparison. One chip rather than two, because the state it is in
    /// is written on its own face and in the notice it leaves.
    RosterCompare,
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
    /// **Put food on the ground.** Paints `windfall` — the fruit a herb drops
    /// — which falls, piles at its own angle of repose and rots back into
    /// soil, so a heap you paint is food that behaves like food rather than a
    /// permanent fixture.
    ///
    /// **This is the box's control arm as much as it is a verb.** `wiki/
    /// ants.md` records the two arms plainly: put food on the ground beside a
    /// nest and a colony breeds **thirteen generations deep**; leave the same
    /// colony to forage the sealed bed and it picks food up sixteen hundred
    /// times and brings it home four. **The one intervention that separates
    /// those two runs was the one thing the lab could not do**, so no
    /// measurement in this bed could tell "the foraging is broken" from "the
    /// economy is broken". Now it can.
    ///
    /// **Off the bar, like `Wall` and `Release`, and this was measured rather
    /// than assumed.** `PIXEL_PHYSICS_BAR_TRACE` on the shipped layout reports
    /// **row 0 slack 0 and row 1 slack 0** — both rows sit at exactly 508 of
    /// 508 — so there is no seventh tool cell at any spacing `layout` tries.
    /// `Reports/dead-ends.md` carries three earlier attempts at fitting one.
    /// The key is in `HELP`, marked `(NO BUTTON)`, which is the pattern `K`
    /// already set.
    Food,
}

/// Every tool **that has a cell on the bar**, in bar order. One list, so the
/// row, the key table and the tests cannot disagree about what exists.
///
/// [`Tool::Release`] is deliberately not in it: its verb moved to the page
/// that already knows what it means (`PLACE`, on the rack), which is the
/// owner's own ruling; see that variant.
///
/// **[`Tool::Wall`] is here now, and the history is worth keeping because it
/// is a measurement rather than a preference.** The bar was measured *full*
/// when the wall verb landed -- 1 px of slack on row 0 and 0 on row 1 at the
/// tightest of the three spacings `layout` tries, with
/// `the_bar_fits_the_screen_and_no_two_widgets_overlap` refusing a ninth cell
/// immediately. So the verb shipped reachable only by its key, and whether it
/// earned a cell was left to the owner rather than forced -- squeezing a
/// control in is how the overlapping columns on the rack page happened.
/// Dropping `KEEP` and `FREE` freed two cells and the owner called it
/// (2026-09-01), so the wall became the seventh.
///
/// **The bar is full again at seven, and that is measured rather than
/// assumed.** The obvious reading of "dropping two cells made room for one"
/// is that a spare cell is left over; it is not. Putting an eighth back
/// fails `the_bar_fits_the_screen_and_no_two_widgets_overlap` exactly as a
/// ninth did before -- the freed width did not all go to the tool row, and
/// `WALL` is not the width of the `KEEP` it replaced. So the rule that
/// applied at eight applies unchanged at seven: **run the fit guard before
/// assuming the next lab control has anywhere to live**, and expect it to
/// say no.
pub const TOOLS: [Tool; 7] =
    [Tool::Look, Tool::Plant, Tool::Colony, Tool::Cull, Tool::Soil, Tool::Water, Tool::Wall];

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
            Tool::Food => "FOOD",
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
            // **`E`, and the bottom-row run had nothing left to give.** The
            // run is `Z X C V B N`; its next two keys are already spoken for
            // — `M` keeps the inspected individual and `,` places a jar — and
            // `F` and `G` are the display rate and the shelf. So this joins
            // `K` off the run, for the reason `K` states: a tool with no bar
            // cell is not in the row the positional rule is about.
            Tool::Food => "E",
        }
    }
    /// **Whether this tool puts animals in the box.** The two that do share a
    /// species chip and a count, because they are one decision asked twice --
    /// *which animal, and how many* -- and the second differs only in whether
    /// the genome comes off the shelf or out of the species table.
    pub fn is_stocking(self) -> bool {
        matches!(self, Tool::Colony | Tool::Release)
    }

    /// Whether this tool paints continuously while the button is held. The
    /// verbs are one-shot — a drag that founded a colony per pixel would empty
    /// the organism table in one gesture.
    pub fn is_brush(self) -> bool {
        matches!(self, Tool::Soil | Tool::Water | Tool::Food)
    }
    fn note(self) -> &'static str {
        match self {
            Tool::Look => "POINT AT A CELL AND READ IT. CLICK TO PIN THE CELL PAGE OPEN; CLICK IT AGAIN TO PUT IT AWAY. WHAT IS UNDER THE POINTER IS ALWAYS READ OUT TOP RIGHT, TOOL OR NO TOOL.",
            Tool::Plant => "PUT ONE SEED IN THE SOIL WHERE YOU CLICK. THE CHIP TO THE RIGHT SAYS WHICH SPECIES AND WHAT IT COSTS TO GROW ONE. A SEED NEEDS BARE SOIL WITH ROOM ABOVE IT.",
            Tool::Colony => "PUT ANIMALS IN THE BOX. THE CHIP TO THE RIGHT SAYS WHICH ANIMAL -- ANT, BEETLE, WORM -- AND THE STOCK DIAL BESIDE IT SAYS HOW MANY. AT 1 IT IS ONE ANIMAL WHERE YOU CLICK, WITH NO NEST. ABOVE 1 IT IS A COLONY AT THE SURFACE UNDER THE CLICK, ARRIVING WITH A PATCH OF NEST TO WALK HOME TO -- WITHOUT ONE THERE IS NO GRADIENT AND NOBODY FORAGES.",
            Tool::Cull => "KILL THE ORGANISM YOU CLICK. IT IS MARKED SENESCENT, NOT DELETED, SO IT ROTS DOWN OVER ITS SPECIES HALF-LIFE AND FEEDS WHATEVER IS STILL ALIVE. THIS IS THE SELECTION LEVER: WHAT YOU CULL DOES NOT BREED.",
            Tool::Soil => "PAINT SOIL, AT FIELD CAPACITY -- DAMP ENOUGH FOR A ROOT, NOT SO WET IT SLUMPS. IT WILL NOT PAINT OVER STONE OR OVER A LIVING PLANT.",
            Tool::Water => "PAINT WATER, FULL. IT RUNS, IT SOAKS INTO SOIL, AND TOO MUCH OF IT DROWNS ROOTS -- WHICH IS AN EXPERIMENT, NOT A MISTAKE.",
            Tool::Wall => "DROP A WALL FLOOR TO CEILING IN THE COLUMN YOU CLICK, OR CLICK ONE YOU PLACED TO TAKE IT OUT. A WALL IS WHAT MAKES TWO POPULATIONS IN ONE BOX INTO TWO POPULATIONS: THEY CANNOT MIX, SO THEY CAN DRIFT APART. IT CUTS WHATEVER IS IN THE WAY, WHICH IS THE POINT -- A WALL THROUGH A STAND IS A STAND SPLIT IN HALF. IT SURVIVES A REBUILD.",
            Tool::Food => "PUT FOOD ON THE GROUND WHERE YOU PAINT. IT IS WINDFALL -- THE FRUIT A HERB DROPS -- SO IT FALLS, PILES UP AND ROTS BACK INTO THE SOIL RATHER THAN SITTING THERE FOR EVER. A COLONY WITH FOOD BESIDE THE NEST BREEDS HARD; THE SAME COLONY LEFT TO FORAGE THE SEALED BED MOSTLY DOES NOT. THIS IS HOW YOU TELL THOSE TWO APART.",
            Tool::Release => "PUT THE ARMED JAR BACK IN THE BOX WHERE YOU CLICK. TWO DIALS DECIDE WHAT ARRIVES: THE STOCK DIAL ON THE BAR IS HOW MANY, AND THE DRIFT DIAL ON THE SHELF IS HOW FAR EACH ONE HAS MOVED FROM THE JAR. AT 0 BROODS IT IS THAT EXACT INDIVIDUAL AGAIN, SO A COLONY IS A COLONY OF CLONES; AT 1 EACH IS AS DIFFERENT AS ITS OWN CHILD WOULD HAVE BEEN, DRAWN SEPARATELY, SO A COLONY IS A COLONY OF SIBLINGS. OPEN THE SHELF WITH G TO PICK A JAR AND SET THAT DIAL.",
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
    /// **Every plant in the box, one per row.** Draws itself for `Params`'
    /// reason and is a [`Panel`] for the same two.
    ///
    /// **Reached from the PLANTS page rather than from the bar**, which is
    /// not a stylistic choice: the bar was measured full on 2026-08-31 and
    /// again on 2026-09-01 -- row 1 at exactly 508 of 508 px and row 0 with
    /// two pixels spare, at the tightest spacing `layout` will accept. There
    /// is no seventh chip. So the aggregate page becomes the cover of the
    /// roster, and the row that opens it is a `Body::Head`, which already
    /// carries a hit target and so cost no new painter code.
    PlantList,
    /// Every animal in the box, one per row. `PlantList`'s twin.
    ///
    /// **Two tables rather than one page with a kingdom switch**, because the
    /// columns differ: a plant has seeds, a water status and a shoot, an
    /// animal has a bank and a crop. A shared table would read `--` down half
    /// of every column, which is the haystack `specimen_sections` splits the
    /// two kingdoms to avoid.
    AntList,
    /// **What happened while you were not looking.** `World::run_log`, newest
    /// first.
    ///
    /// The one page in the lab whose subject is *time* rather than a standing
    /// quantity: every other page answers "what is the box like now", and at
    /// 1024x a player crosses tens of thousands of frames between two glances
    /// at it. A count that moved from 15 to 64 does not say a lineage died.
    ///
    /// **Narrative only. Never count anything off it** -- the log is bounded
    /// (`world::RUN_LOG_CAP`) and drops its oldest lines, so counting rows
    /// would give an undercount wearing the shape of an answer. Every number
    /// in this interface comes from `LifeCounters` and `CreatureStats`, which
    /// are unbounded.
    Log,
    /// **Two individuals, side by side, with what differs marked.**
    ///
    /// The question a selection experiment is actually asking -- *why did
    /// this one do better than that one* -- and nothing on this interface
    /// could answer it. Every other page is about one individual or about the
    /// whole box; a difference between two is neither, and reading it by
    /// flipping between two cell pages means holding thirty numbers in your
    /// head and comparing them from memory.
    ///
    /// **Reached from the roster's own footer, not from the bar**, for
    /// `PlantList`'s reason: the bar has been measured full twice and there
    /// is no seventh chip. The roster is also where both individuals get
    /// chosen, so the verb sits where its operands do.
    Compare,
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
            Panel::PlantList => "EVERY PLANT",
            Panel::AntList => "EVERY ANIMAL",
            Panel::Log => "WHAT HAPPENED",
            Panel::Compare => "SIDE BY SIDE",
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
    /// **How many animals a stocking click puts down**, for the two verbs
    /// that stock the box -- `COLONY` and a jar release. Shares the brush's
    /// three cells on the bar, because a brush radius means nothing to either
    /// of them and a stocking count means nothing to a brush; see `layout`.
    pub stock: i32,
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

/// What the stocking dial says it is for. Three notes rather than one,
/// because the two arrows do different things and a shared note would make
/// the player read a sentence about `+` while hovering `-`.
const STOCK_NOTE: &str = "HOW MANY ANIMALS ONE CLICK PUTS DOWN. AT 1 IT IS A SINGLE ANIMAL WHERE YOU CLICK, WITH NO NEST -- THIS IS HOW YOU ADD ONE BEETLE. ABOVE 1 IT IS A COLONY: A PATCH OF NEST TO WALK HOME TO AND THE ANIMALS SPREAD ALONG THE GROUND EITHER SIDE OF THE CLICK, WHICH IS WHAT THEY NEED TO FORAGE AT ALL. IT GOVERNS BOTH STOCKING VERBS -- COLONY AND A JAR RELEASE.";
const STOCK_DOWN_NOTE: &str = "FEWER. THE LADDER RUNS 1, 2, 4, 8, 16, 32, 52, 104 -- 52 IS THE SHIPPED COLONY, AND BELOW ABOUT FIFTY A COLONY LOOKS BROKEN EVEN WHEN THE CODE IS RIGHT. 1 IS ONE ANIMAL AND NO NEST.";
const STOCK_UP_NOTE: &str = "MORE. THE LADDER RUNS 1, 2, 4, 8, 16, 32, 52, 104. EVERY ANIMAL IS A LIVING THING THE BOX HAS TO FEED, SO A HUNDRED OF THEM IS AN EXPERIMENT ABOUT CROWDING WHETHER YOU MEANT IT AS ONE OR NOT.";

/// The stops the stocking dial climbs.
///
/// **A ladder rather than a step of one**, because the two ends are 1 and a
/// full colony and a dial you have to click fifty-one times is not a control
/// anybody uses -- the same finding the batch dials' typed entry came from.
/// 52 is `creature::COLONY_ANTS`, the shipped colony and Grasse's threshold in
/// practice; it is the default, so a player who never touches this dial gets
/// exactly the colony the tool has always placed.
const STOCK_LADDER: [i32; 8] = [1, 2, 4, 8, 16, 32, 52, 104];

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
        // **`.`, which is the key `bin/lab.rs` actually binds.** It read `;`
        // and had since the chip landed; `;` is the brood dial's step-down.
        // A printed caption that is not the key is worse than none, because
        // the player who tries it changes something else.
        width: cell_width(species_px, ".", pad),
        line1: state.species.to_string(),
        line2: ".".to_string(),
        action: Some(Action::NextSpecies),
        latched: state.tool == Tool::Plant,
        icon: None,
        ratio: None,
        note: state.species_note.to_string(),
    };

    // **One dial, two meanings, decided by the armed tool.** A brush radius
    // is meaningless to `COLONY` and a stocking count is meaningless to the
    // soil brush, so the three cells say whichever the armed tool can use.
    //
    // Context-sensitive rather than a fourth group because **the bar is
    // full** -- measured, with `the_bar_fits_the_screen_and_no_two_widgets_
    // overlap` refusing an eighth tool cell -- and because these three cells
    // were dead weight under exactly the tools that needed a dial of their
    // own. Both cells are sized to the wider of the two faces, so the row
    // does not shift under the cursor when the tool changes.
    let stocking = state.tool.is_stocking();
    let step_px = cell_width(hud::text_width("W"), "]", pad);
    let dial_px = hud::text_width("R64").max(hud::text_width("104"));
    let dial_w = cell_width(dial_px, "SIZE", pad).max(cell_width(dial_px, "STOCK", pad));
    let narrower = Spec {
        width: step_px,
        ..if stocking {
            button("-", "[", Action::Stock(-1), false, STOCK_DOWN_NOTE, pad)
        } else {
            button("-", "[", Action::Brush(-1), false, "A NARROWER BRUSH. THE RADIUS IS IN CELLS, SO R1 IS A THREE-CELL DAB AND R16 IS A SPADEFUL.", pad)
        }
    };
    let wider = Spec {
        width: step_px,
        ..if stocking {
            button("+", "]", Action::Stock(1), false, STOCK_UP_NOTE, pad)
        } else {
            button("+", "]", Action::Brush(1), false, "A WIDER BRUSH. THE RADIUS IS IN CELLS, AND THE COST OF A STROKE GOES UP WITH ITS AREA, NOT ITS LENGTH.", pad)
        }
    };
    let size = Spec {
        // Sized to `R64` and `104`, the widest either can say. See the
        // readout.
        width: dial_w,
        line1: if stocking { state.stock.to_string() } else { format!("R{}", state.brush) },
        line2: if stocking { "STOCK" } else { "SIZE" }.to_string(),
        action: None,
        latched: false,
        icon: None,
        ratio: None,
        note: if stocking { STOCK_NOTE } else { "HOW WIDE THE SOIL AND WATER BRUSHES ARE, IN CELLS." }.to_string(),
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

/// Height of `Body::Lines`' plotted area, in pixels -- about the owner's ask
/// of "40px tall", and shared between `Row::height` and `draw_lines` so the
/// two cannot disagree about how tall the chart actually is.
const CHART_H: i32 = 40;

/// What one row of an info panel is.
enum Body {
    /// A named quantity: label on the left, value on the right.
    Value { label: String, value: String, tint: [u8; 4] },
    /// A population over the last few dozen samples.
    Spark { label: String, series: Vec<u32>, tint: [u8; 4] },
    /// **Several named populations on one shared y-axis.** See `draw_lines`
    /// for why this is not just `Spark` called several times: `Spark`
    /// normalises each series to *its own* peak, which is right for one
    /// quantity and wrong the moment two are compared -- a colony of 4 and
    /// one of 40 would each fill their strip top to bottom and read as a
    /// tie. Built for the ANTS page's per-group chart, whose whole reason to
    /// exist is telling two fighting groups apart by more than a colour.
    /// One axis, several tints.
    Lines { caption: String, series: Vec<(Vec<u32>, [u8; 4])> },
    /// **A row that reads like `Value` and clicks like `Head`.** Label on
    /// the left, the current setting on the right, and the whole row is a
    /// hit target through the same `taps` mechanism `Head` already uses in
    /// `paint_page` -- no new painter code, the same trap `Head`'s own doc
    /// records ("built, thrown away, and could never fire") already has its
    /// fix in one place. Written for the ANTS page's colour-mode toggle,
    /// which needed a clickable row rather than a foldable group.
    Choice { label: String, value: String, action: Action },
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
    fn lines(caption: impl Into<String>, series: Vec<(Vec<u32>, [u8; 4])>, note: impl Into<String>) -> Self {
        Self { body: Body::Lines { caption: caption.into(), series }, note: note.into() }
    }
    fn choice(label: impl Into<String>, value: impl Into<String>, action: Action, note: impl Into<String>) -> Self {
        Self { body: Body::Choice { label: label.into(), value: value.into(), action }, note: note.into() }
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
            // Same footprint as `Value` -- it draws like one and only the
            // tap target under it differs.
            Body::Choice { .. } => LINE,
            // A rule above the label and a pixel of air under it, so a shut
            // group reads as a lid rather than as another value row.
            Body::Head { .. } => LINE + 4,
            // 22, not the strip's own 14: the strip's caption sits *under* the
            // bars, and a row measuring only the bars lets it overprint the
            // next row. Lane A's `Generations` row records the same trap.
            Body::Spark { .. } => 22,
            // `CHART_H` plus the same +10 margin `Spark` carries above, and
            // for the identical reason: the caption is drawn a couple of
            // pixels under the chart, not inside it, so a row measuring only
            // the chart lets the caption overprint the legend row below it.
            Body::Lines { .. } => CHART_H + 10,
            Body::Gap => 4,
        }
    }
    fn width(&self) -> i32 {
        match &self.body {
            Body::Value { label, value, .. } => {
                hud::text_width(label) + 12 + hud::text_width(value)
            }
            Body::Choice { label, value, .. } => hud::text_width(label) + 12 + hud::text_width(value),
            Body::Spark { label, .. } => hud::text_width(label).max(96),
            Body::Lines { caption, .. } => hud::text_width(caption).max(96),
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

/// **No longer `Copy`.** The per-group census below is a `Vec`, and a `Vec`
/// cannot derive it -- the only two call sites that ever relied on `Sample`
/// being copied out of the ring took it by reference already (`series` and
/// `delta` both hold `fn(&Sample) -> _`), so nothing downstream needed the
/// bit this gave up. Checked directly rather than assumed: `grep -n
/// samples src/lab/ui.rs` turns up no `.copied()` on a `Sample`.
#[derive(Clone, Default)]
struct Sample {
    /// **The simulated frame it was taken at**, which this did not carry.
    ///
    /// Without it a sample is a value with no position, so nothing
    /// downstream can say whether the series is evenly spaced -- and while
    /// `observe` was called once per *drawn* frame it was not. `stats::
    /// Sample` has always carried one, which is why the same defect there
    /// only cost resolution rather than the axis. Anything that wants to
    /// plot these against time, or check the spacing, reads this.
    frame: u64,
    plants: u32,
    ants: u32,
    germinations: u64,
    /// **One entry per live colony at this sample** -- `(species, colony,
    /// alive)`, copied straight out of `World::live_creature_groups` rather
    /// than kept as a reference, for the same reason the rest of `Sample`
    /// is a copy: this ring outlives the tick it was read on. Built for the
    /// ANTS page's per-group chart, which the owner asked for after
    /// watching an ant colony and a beetle group fight with only the
    /// combined `ants` total above to read it by -- `groups` is what lets
    /// `History::group_series` answer "how many of *this one*" per sample.
    /// Empty on every sample taken before the first colony was founded, and
    /// `group_series` reads that as zero rather than as a gap -- a group
    /// that has not been founded yet and one that died out must look the
    /// same on the chart: the floor.
    groups: Vec<(SpeciesId, u32, u32)>,
}

/// **One sample of one watched individual.**
///
/// Distinct from `Sample` above, which is the whole box's population: this is
/// about a single organism and is sampled far more often, because what it is
/// for is a *path*. A trail sampled at the population strip's 120 frames
/// would be nine dots across a run.
#[derive(Clone, Copy, Default, Debug)]
struct Track {
    /// The simulated frame it was taken at. Carried for the same reason
    /// `Sample::frame` now is -- a value with no position cannot be checked
    /// for even spacing, and the population strip spent a while unevenly
    /// spaced without anything being able to say so.
    frame: u64,
    /// Where it was: a creature's head, a plant's collar. `roster::anchor_of`,
    /// which is the same point the roster's marker aims at, so the trail ends
    /// where the marker sits rather than a few cells off it.
    at: (i32, i32),
    /// A creature's bank; a plant's water status.
    energy: f32,
    cells: u16,
}

/// **How often the watched individual is sampled**, in simulated frames.
///
/// An ant runs on `CreatureDef::tick_interval`, which is 6 for the shipped
/// one, so this is every second move. Fine enough that the trail is a path
/// rather than a scatter, coarse enough that `WATCH_SAMPLES` covers a useful
/// stretch of run.
const WATCH_EVERY: u64 = 12;

/// **How many samples the watch ring holds.** At `WATCH_EVERY` this is 1,536
/// simulated frames of history -- long enough to show a foraging excursion
/// out and back, which is the shape the trail exists to show.
const WATCH_SAMPLES: usize = 128;

/// **The trail and the per-individual series, for whoever is pinned.**
///
/// Per-box state, and it is on `Chamber` for exactly the reason `History` is:
/// a trail is a set of world coordinates, so one left shared would draw
/// chamber A's path across chamber B's bed. That is the bleed the chamber
/// table warns about, one level further down.
///
/// **Keyed on the individual, not on the pin.** The ring clears itself when
/// `who` changes, so it can never show one animal's path under another's
/// name -- including the case a shared ring would get wrong silently, where a
/// slot is re-used and the new occupant inherits the old one's trail.
#[derive(Clone, Debug, Default)]
pub struct Watch {
    who: Option<roster::Individual>,
    samples: VecDeque<Track>,
    next_at: u64,
}

impl Watch {
    /// Sample `who`, if there is one and it is due.
    fn observe(&mut self, world: &World, who: Option<roster::Individual>) {
        if self.who != who {
            self.who = who;
            self.samples.clear();
            self.next_at = 0;
        }
        let Some(who) = who else { return };
        // A rebuild puts the frame counter back, and a trail carried across it
        // would draw a path between two different worlds. Same guard
        // `History::observe` carries, and it is needed here for a stronger
        // reason: those are counts, these are coordinates.
        if self.samples.back().is_some_and(|t| world.frame < t.frame) {
            self.samples.clear();
            self.next_at = 0;
        }
        if world.frame < self.next_at && !self.samples.is_empty() {
            return;
        }
        // **A dead individual stops adding to its trail and keeps it.** It
        // resolves to nothing once `free_organism` has run, and the path it
        // left is the most interesting thing about it -- where it got to
        // before it starved is exactly what the graveyard row cannot say.
        let Some(state) = who.resolve(world) else { return };
        let Some(at) = roster::anchor_of(state) else { return };
        self.samples.push_back(Track {
            frame: world.frame,
            at,
            energy: state.energy,
            cells: state.cells.len().min(u16::MAX as usize) as u16,
        });
        while self.samples.len() > WATCH_SAMPLES {
            self.samples.pop_front();
        }
        self.next_at = world.frame + WATCH_EVERY;
    }

    /// The path, oldest first.
    fn path(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.samples.iter().map(|t| t.at)
    }

    /// One channel as a series the sparkline painter can take, **scaled to
    /// its own range rather than to zero.**
    ///
    /// `draw_spark` normalises against the series' peak, which is right for
    /// the population strip it was written for -- a count of zero means
    /// extinction and belongs on the floor. It is wrong for a per-individual
    /// channel, and measurably so: the shipped ant's bank runs 264 to 349
    /// over a full ring, so every bar lands between 76% and 100% of the peak
    /// and the strip draws as a solid block. The sawtooth of an animal
    /// spending and refilling -- the one thing the row exists to show, and
    /// what its own explanation promises -- is invisible.
    ///
    /// This is the same failure `CLAUDE.md` records for the canopy-density
    /// overlay, where a magnitude-scaled blend produced a sheet that read as
    /// blank and the obvious conclusion was "the mechanism is dead". The
    /// remedy there was a full ramp over the real range, and it is the remedy
    /// here.
    ///
    /// **The caption carries the absolute range, so the floor is never
    /// implied to be zero.** Without that this would be the more dangerous
    /// error of the two: a bank hovering at 95% of full and one swinging from
    /// empty would draw identically.
    ///
    /// A channel that does not vary at all draws flat rather than dividing by
    /// zero -- an ant's body is 2 cells for its whole life, and a flat line is
    /// the true answer for it.
    fn series(&self, pick: fn(&Track) -> f32) -> Vec<u32> {
        let (lo, hi) = self.range(pick);
        let span = hi - lo;
        if span <= f32::EPSILON {
            return self.samples.iter().map(|_| 1u32).collect();
        }
        self.samples.iter().map(|t| (((pick(t) - lo) / span) * 100.0).clamp(0.0, 100.0) as u32 + 1).collect()
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Whether the ring is about the organism in `id`.
    ///
    /// **Gated on this and not merely on "something is pinned"**, because the
    /// cell page follows the cursor and the pin does not: click a second ant
    /// while the first is pinned and an ungated page draws the *pinned* one's
    /// series under the *clicked* one's numbers. Every row on that page is
    /// about one individual or the page is worse than having none.
    fn about(&self, id: u16) -> bool {
        self.who.is_some_and(|w| w.id == id) && !self.samples.is_empty()
    }

    /// The lowest and highest a channel reached across the ring.
    ///
    /// The caption needs both, because `draw_spark` normalises against the
    /// series' own peak -- so a bank that never left 95.4..95.9 draws exactly
    /// like one that swung from empty to full. Without the range printed, the
    /// shape is unreadable in the one case a player most wants to trust it.
    fn range(&self, pick: fn(&Track) -> f32) -> (f32, f32) {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for t in &self.samples {
            let v = pick(t);
            lo = lo.min(v);
            hi = hi.max(v);
        }
        if lo > hi { (0.0, 0.0) } else { (lo, hi) }
    }

    /// How many simulated frames the trail spans.
    fn span(&self) -> u64 {
        match (self.samples.front(), self.samples.back()) {
            (Some(a), Some(b)) => b.frame.saturating_sub(a.frame),
            _ => 0,
        }
    }
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
            frame: world.frame,
            plants: orgs.saturating_sub(ants),
            ants,
            germinations: world.germinations,
            // Read straight off the engine's own partition rather than
            // re-deriving one here -- `World::live_creature_groups`' own doc
            // is explicit that the world owns the split and a page only
            // draws it.
            groups: world.live_creature_groups().into_iter().map(|g| (g.species, g.colony, g.alive)).collect(),
        });
        while self.samples.len() > HISTORY {
            self.samples.pop_front();
        }
        self.next_at = world.frame + SAMPLE_EVERY;
    }

    fn series(&self, pick: fn(&Sample) -> u32) -> Vec<u32> {
        self.samples.iter().map(pick).collect()
    }

    /// **One group's population across the ring, `0` where a sample carries
    /// no row for it.**
    ///
    /// `series` above picks a field every sample always has; a group can be
    /// absent from a sample because it had not been founded yet or because
    /// it has since died out, and both must draw the same way -- the floor,
    /// not a series that quietly gets shorter. `CLAUDE.md`'s first law for
    /// an outcome: a dying colony is a line falling to zero, and the graph
    /// this feeds is exactly the case that law was written against ("a pool
    /// that is either full or empty ... has the same defect the rubble
    /// did").
    ///
    /// `matches` rather than a fixed `(species, colony)` pair, because the
    /// two grouping modes need different equalities: colony mode wants
    /// exactly one `(species, colony)`, species mode wants every colony of
    /// one species summed into a single line. A closure expresses both; a
    /// tuple could only express the first.
    /// **Every group any sample in the ring still holds**, in placement
    /// order and without repeats -- the list that keeps a wiped-out group on
    /// the ANTS page until its last sample has decimated away.
    fn remembered_groups(&self) -> Vec<(SpeciesId, u32)> {
        let mut out: Vec<(SpeciesId, u32)> = Vec::new();
        for s in &self.samples {
            for &(sp, co, _) in &s.groups {
                if !out.contains(&(sp, co)) {
                    out.push((sp, co));
                }
            }
        }
        out.sort_by_key(|&(sp, co)| (co, sp.0));
        out
    }

    fn group_series(&self, matches: impl Fn(SpeciesId, u32) -> bool) -> Vec<u32> {
        self.samples
            .iter()
            .map(|s| s.groups.iter().filter(|&&(sp, co, _)| matches(sp, co)).map(|&(_, _, alive)| alive).sum())
            .collect()
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

    /// **How many simulated frames the last delta actually spans.**
    ///
    /// The `CHANGE` row's explanation used to say "120 SIMULATED FRAMES
    /// APART" as a literal, and while `observe` was called once per drawn
    /// frame that was simply untrue -- the real gap was whatever a batch
    /// happened to be, drifting around 140 and set by how fast the machine
    /// was. Reading it off the samples means the page states a measured fact
    /// about the run in front of it rather than a property it assumes, and a
    /// cadence that comes adrift again shows up in the tooltip instead of
    /// being invisible. It is also the only reader `Sample::frame` has, which
    /// is the point: a field with a writer and no reader is dead weight.
    fn gap(&self) -> Option<u64> {
        let n = self.samples.len();
        if n < 2 {
            return None;
        }
        Some(self.samples[n - 1].frame.saturating_sub(self.samples[n - 2].frame))
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

/// `render::group_colour`'s `[f32; 3]` (0..255 per channel, to match the
/// renderer's own cell-colour maths) into the `[u8; 4]` tint every row body
/// on this page actually wants. One conversion, so a chart line and the
/// legend swatch beside it can never round differently and drift apart.
/// How many groups the ANTS page charts and lists. See `capped_group_rows`.
const GROUP_ROWS: usize = 12;

/// **What has happened to a group's dead**, folded for the legend: killed,
/// starved, and the killers by name. `World::group_deaths` is per
/// `(species, colony)`; species mode sums every colony of the species, so
/// the legend's deaths agree with the line it sits under in either mode.
fn group_losses(world: &World, species: SpeciesId, colony: Option<u32>) -> (u64, u64, Vec<(String, u64)>) {
    let mut killed = 0u64;
    let mut starved = 0u64;
    let mut killers: Vec<(String, u64)> = Vec::new();
    for d in &world.group_deaths {
        if d.species != species || colony.is_some_and(|c| c != d.colony) {
            continue;
        }
        killed += d.by_cause[crate::sim::organism::DeathCause::Killed.index()];
        starved += d.by_cause[crate::sim::organism::DeathCause::Starved.index()]
            + d.by_cause[crate::sim::organism::DeathCause::StarvedInFlight.index()];
        for (asp, acol, n) in &d.killed_by {
            let name = world.species.get(*asp).name.to_uppercase();
            let who = if colony.is_some() { format!("{name} {acol}") } else { name };
            match killers.iter_mut().find(|(k, _)| *k == who) {
                Some((_, m)) => *m += n,
                None => killers.push((who, *n)),
            }
        }
    }
    (killed, starved, killers)
}

fn tint_of(rgb: [f32; 3]) -> [u8; 4] {
    [rgb[0].round().clamp(0.0, 255.0) as u8, rgb[1].round().clamp(0.0, 255.0) as u8, rgb[2].round().clamp(0.0, 255.0) as u8, 255]
}

/// **Which of a world's live groups the ANTS page draws, and how many it had
/// to leave out.**
///
/// Colony mode's row is `World::live_creature_groups`'s own row, unchanged.
/// Species mode folds every colony of a species into the first row that
/// species claimed, because the chart draws one line per row and "one line
/// per species" is the whole point of that mode -- two colonies of the same
/// species must merge into one line, not draw a second one at the bottom.
///
/// **Capped at [`GROUP_ROWS`]** -- a page-height limit, not a palette one.
/// The cap was the palette's eight until the owner's ruling that *"every
/// line should have its own colour even when there are twenty"* made
/// `render::group_palette` generate a fresh hue per group; what bounds the
/// legend now is the screen, and twelve rows under a chart is what fits
/// beside the world. Past the cap the twelve *with the most animals alive*
/// are kept, tie-broken on `(species, colony)` so the choice cannot depend
/// on hash-map or float iteration order -- `CLAUDE.md`'s standing warning
/// about an unstable sort whose tie order is not a pure function of the
/// comparator. The kept rows are re-sorted back to placement order before
/// being returned, so capping never reshuffles the ones that do fit.
///
/// **A group that has died out stays on the page while the ring remembers
/// it**, at `alive 0`. The first draft listed only live groups, and the
/// beetles that had killed fifteen ants and then starved were simply gone
/// from the legend -- the one line the owner most wants to watch fall to
/// the floor ("one almost getting wiped out") vanished at the moment it
/// reached it. `remembered` is every group any sample in the ring holds; a
/// group leaves the page only when its last sample decimates away.
fn capped_group_rows(world: &World, colony_mode: bool, remembered: &[(SpeciesId, u32)]) -> (Vec<(SpeciesId, u32, u32)>, usize) {
    let mut rows: Vec<(SpeciesId, u32, u32)> = Vec::new();
    let mut add = |species: SpeciesId, colony: u32, alive: u32| {
        if colony_mode {
            match rows.iter_mut().find(|(sp, co, _)| *sp == species && *co == colony) {
                Some((_, _, a)) => *a += alive,
                None => rows.push((species, colony, alive)),
            }
        } else {
            match rows.iter_mut().find(|(sp, _, _)| *sp == species) {
                Some((_, _, a)) => *a += alive,
                None => rows.push((species, 0, alive)),
            }
        }
    };
    for g in world.live_creature_groups() {
        add(g.species, g.colony, g.alive);
    }
    for &(species, colony) in remembered {
        add(species, colony, 0);
    }
    // Placement order, whichever list a row arrived from: `(colony, species)`
    // is `live_creature_groups`' own order, and in species mode every colony
    // is 0 so it collapses to species order.
    rows.sort_by_key(|&(sp, co, _)| (co, sp.0));
    let cap = GROUP_ROWS;
    if rows.len() <= cap {
        return (rows, 0);
    }
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by(|&a, &b| rows[b].2.cmp(&rows[a].2).then(rows[a].0.0.cmp(&rows[b].0.0)).then(rows[a].1.cmp(&rows[b].1)));
    let mut keep: Vec<usize> = order.into_iter().take(cap).collect();
    keep.sort_unstable();
    let dropped = rows.len() - keep.len();
    (keep.into_iter().map(|i| rows[i]).collect(), dropped)
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
    /// **How many animals a stocking click puts down** — one stop per
    /// stocking verb, indexed by [`Ui::stock_slot`].
    ///
    /// An index into [`STOCK_LADDER`] rather than the number itself, so the
    /// dial cannot land between stops and the ladder is the only place the
    /// stops are written down.
    ///
    /// **Two values behind one control, because the two verbs want opposite
    /// defaults and both defaults are right.** `COLONY` founds fifty-two --
    /// below about fifty a colony looks broken even when the code is right --
    /// and a *release* is a specimen you kept on purpose, where fifty-two
    /// copies of your one good forager is a box you did not ask for. The cell
    /// shows whichever the armed tool uses, which is the same rule that makes
    /// it a brush radius under a brush.
    stock: [usize; 2],
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
    /// **Which individual that cell belonged to when it was clicked**, so the
    /// page can follow it when it walks off the cell.
    ///
    /// The inspector was purely positional, and for a plant that is the right
    /// model -- it is a *cell* page and a plant's cells stay put. For an
    /// animal it is not: an ant crosses a cell in a handful of ticks, so the
    /// page emptied the moment the box was running. Owner, from play: *"the
    /// selection is currently positional in the lab, but the creature moves
    /// and is immediately unselected, unless time is paused."*
    ///
    /// The handle rather than a copy of the state, because the page is
    /// deliberately live -- `inspect_rows` re-reads the world every frame so a
    /// clicked ant's energy falls while you watch, and a snapshot would freeze
    /// exactly the numbers the page exists to show.
    inspect_organism: Option<u16>,
    pub(crate) history: History,
    /// The pinned individual's trail and per-individual series.
    /// Per-box, and swapped with the chamber like `history`.
    pub(crate) watch: Watch,
    /// **The other individual**, for `Panel::Compare`.
    ///
    /// Deliberately not a second pin: only one individual is marked in the
    /// box and followed at a time, because two markers on a 512-wide bed is
    /// two things to find rather than one. This is a *held* identity, and
    /// like the pin it is `(organism_id, born_frame)` -- never a row index,
    /// which reorders under a sort, and never a cell, which the individual
    /// walks off.
    held: Option<roster::Individual>,
    /// **The individuals a cull-the-rest will keep.**
    ///
    /// A set rather than a second pin, because the question it answers is
    /// *"keep these"* and there is no reason that is one. Identities, like
    /// everything else that names an individual here -- a row index reorders
    /// under a sort and a cell is somewhere the animal walks off.
    ///
    /// **Not pruned when its members die.** A spared individual that starves
    /// is simply no longer in the table, and a set that quietly forgot it
    /// would make `SPARE` look like it had failed. `CULL REST` resolves
    /// against the world each time it runs, so a dead entry costs nothing.
    spared: Vec<roster::Individual>,
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
    /// Whether the rack is showing one row per setting instead of one per run.
    rack_grouped: bool,
    /// The dial being typed into and the digits so far, if any.
    typing: Option<(TypedField, String)>,
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
    /// **The individual the interface is holding on to**, across frames and
    /// across the thing moving.
    ///
    /// An identity rather than an index or a cell, and both of those were the
    /// obvious wrong answers. An index into the roster names a different
    /// animal the moment anything is born, dies or the sort changes. A cell
    /// -- which is what `inspect` is, deliberately -- names whatever is
    /// standing there now, so a walking ant leaves its own page behind. This
    /// is the third thing: `(handle, born_frame)`, checked both halves, and a
    /// pin that stops resolving is a **death** rather than a lookup failure.
    ///
    /// While it holds, `inspect` is re-pointed at the individual's own cell
    /// every frame, so the cell page follows the animal instead of the ground
    /// and every verb already on that page keeps working unchanged.
    pinned: Option<roster::Individual>,
    /// Whether the camera is chasing the pin.
    following: bool,
    /// The roster page's own clickable rectangles, for `params_bar`'s reason.
    roster_bar: Bar,
    roster_box: Option<Rect>,
    /// **How each roster is being read, one entry per kingdom.**
    ///
    /// Per kingdom rather than shared, and that is a correctness matter
    /// rather than a convenience. A sort is stored as a **column index**, and
    /// column 1 is SEED on the plants table and BANK on the animals' -- so
    /// one shared index means "sort on the second column, whatever that is
    /// here", which is not a thing anybody asked for. Caught by the harness
    /// on its first run: sorting the plants list and then opening the animals
    /// list drew the animals in a sort nobody had chosen, and a click on the
    /// third row pinned ant 41 where ant 11 was expected. Scroll and filter
    /// go with it for the ordinary reason -- coming back to a list you had
    /// scrolled and finding it at the top is a list that forgot what you were
    /// doing.
    roster_view: [RosterView; 2],
    /// **The generic pages' own clickable rectangles.**
    ///
    /// `paint_page` has always collected `Body::Head` rows into a `taps` vec
    /// and the one caller that draws a `Panel` through it passed
    /// `&mut Vec::new()` -- so every heading on the PLANTS, ANTS and BOX pages
    /// was a hit target that was built, thrown away, and could never fire.
    /// Nothing noticed because those pages had no clickable headings yet; the
    /// roster's way in is the first, and it would have shipped as a row that
    /// looks like a button and does nothing. `CLAUDE.md`'s standing check --
    /// a channel needs a writer and a reader, and the compiler checks
    /// neither -- in its "read and never written" direction.
    panel_bar: Bar,
    /// **Mirrors `Renderer::creature_colour`** -- see that enum's own doc in
    /// `render.rs`. `Ui` does not hold the renderer, so this is how the
    /// ANTS page's chart and legend learn which mode to group and colour
    /// their lines by: `Lab` pushes the mode across whenever it changes
    /// (`Lab::new`, right after the renderer is built, and the
    /// `CycleCreatureColour` handler), rather than the page reaching through
    /// a shared handle it does not have. There are exactly two writers, both
    /// in `Lab`, which is what keeps a second source of truth from drifting
    /// out of step with the first.
    ///
    /// **`Option` rather than a bare `CreatureColour`**, because `Ui`
    /// derives `Default` and `render::CreatureColour` does not implement
    /// it -- and `render.rs` is out of scope for this change. `None` is not
    /// a third mode; it is the instant between `Ui::new` and `Lab::new`
    /// finishing, before either writer has run. `creature_colour()` below is
    /// the only reader and it treats `None` as `Off`, which is the enum's
    /// own "nothing has customised this yet" state.
    creature_colour: Option<render::CreatureColour>,
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

/// Every species that can be *stocked* — the animals, in a stable order.
///
/// [`plantable_species`]' mirror image, filtered the same way and for the same
/// reason: the species table is the list, and a hand-kept roster of animal
/// names is the side table that goes stale the first time `assets/species/`
/// gains a file. There were three animals in the table and exactly one of them
/// -- the ant -- could be put in the box by hand, which is the gap this
/// closes. Owner: *"I need to be able to add a beetle manually."*
pub fn stockable_species(world: &World) -> Vec<crate::sim::organism::SpeciesId> {
    use crate::sim::organism::SpeciesId;
    let mut ids: Vec<SpeciesId> = (0..world.species.len() as u16)
        .map(SpeciesId)
        .filter(|id| world.species.get(*id).creature.is_some())
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
            // **`WORDS`, and this reverses an argued decision.** It used to
            // be `STATE` -- the group that moves while the box runs, and what
            // a player opened the page to watch -- against `Default`'s own 0,
            // which was then `LIFE`, three rows that never change.
            //
            // `WORDS` is now 0, and it answers the question the page is most
            // often opened with: *what kind of thing is this*. The old
            // reasoning still holds for the second page you open and every
            // one after, and it does not have to be paid for -- the field is
            // sticky, so one click on `STATE` puts every later page there and
            // leaves it there. Defaulting to the summary costs a returning
            // player one click and costs a new one nothing.
            specimen_section: 0,
            // The shipped colony for `COLONY` and a single individual for a
            // release, so both verbs do for an untouched dial exactly what
            // they have always done.
            stock: [
                STOCK_LADDER
                    .iter()
                    .position(|n| *n == crate::sim::creature::COLONY_ANTS)
                    .expect("the ladder carries the shipped colony size"),
                0,
            ],
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
            || self.roster_box.is_some_and(|r| r.contains(x, y))
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
    ///
    /// **Clicking anything alive latches the individual too**, and
    /// [`Ui::follow_inspected`] then keeps the page on it as it moves. The
    /// put-it-away click is therefore *the same animal* rather than the same
    /// cell: an ant is two cells and a beetle is four, and having to click the
    /// exact cell you first hit -- on a body that has since walked and turned
    /// around -- is a toggle nobody can aim.
    pub fn inspect(&mut self, world: &World, cell: (i32, i32)) {
        let organism = match world.get(cell.0, cell.1).organism_id() {
            0 => None,
            id if world.organism(id).is_some() => Some(id),
            _ => None,
        };
        let same = self.inspect == Some(cell) || (organism.is_some() && organism == self.inspect_organism);
        if same {
            self.inspect = None;
            self.inspect_organism = None;
        } else {
            self.inspect = Some(cell);
            self.inspect_organism = organism;
        }
    }

    /// **Keep the cell page on the individual it was opened on.**
    ///
    /// Called once per drawn frame, before the page is painted, and it does
    /// nothing at all in the common case: while the tracked cell still belongs
    /// to the tracked individual the page stays exactly where it was put. That
    /// is what keeps a *plant* page honest -- a plant does not move, and
    /// silently sliding the reticle to some other cell of the same tree would
    /// change the material and temperature rows the player is reading.
    ///
    /// It moves only when the cell stops being that individual's: an ant that
    /// walked, a leaf that was shed. Then it takes the individual's cell
    /// nearest the one it lost, so a two-cell ant's page follows the end of it
    /// that stayed closest rather than jumping across the body.
    ///
    /// When the individual is gone the latch is dropped and **the cell is
    /// kept**, which is the honest reading of a death: the page goes on
    /// reporting that position, where a corpse now lies.
    pub fn follow_inspected(&mut self, world: &World) {
        let (Some(at), Some(id)) = (self.inspect, self.inspect_organism) else { return };
        // **Liveness before position**, and that order is the whole of the
        // death case: `free_organism` clears the slot and leaves the id
        // written in the cells, so a dead animal's cell still reads back its
        // handle. Tested position-first, the page would go on following a
        // corpse for ever and report it as alive.
        let Some(state) = world.organism(id) else {
            self.inspect_organism = None;
            return;
        };
        if world.get(at.0, at.1).organism_id() == id {
            return;
        }
        // **Ties broken on the coordinate, never on iteration order.** The
        // cell list is a `HashMap`, so "the first nearest cell" is not a
        // defined thing and a page that picked one would be a different page
        // on two runs of the same box.
        let nearest = state.cells.keys().min_by_key(|(x, y)| {
            let (dx, dy) = ((x - at.0) as i64, (y - at.1) as i64);
            (dx * dx + dy * dy, *y, *x)
        });
        match nearest {
            Some(&cell) => self.inspect = Some(cell),
            // Alive with no cells is not a state the engine produces; if it
            // ever does, stop following rather than pointing at nothing.
            None => self.inspect_organism = None,
        }
    }

    /// The individual the cell page is following, if it is following one.
    pub fn inspected_organism(&self) -> Option<u16> {
        self.inspect_organism
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
        // **The watch ring is fed from the pin rather than from a separate
        // selection.** A second way to choose an individual is a second thing
        // that can disagree with the marker on screen, and the pin is already
        // the thing that means "this one" everywhere else on this interface.
        self.watch.observe(world, self.pinned);
    }

    /// **Told by `Lab` whenever the animals' colour mode changes.** See
    /// `creature_colour`'s own doc for why the mirror exists and who the two
    /// callers are.
    pub fn set_creature_colour(&mut self, mode: render::CreatureColour) {
        self.creature_colour = Some(mode);
    }

    /// The mirrored mode, read as `Off` for the instant before `Lab` has set
    /// it. `pub` for the same reason `tool()`/`stock()`/`broods()` are --
    /// a test that wants to know what a mirrored dial currently reads should
    /// not need a private-field workaround to ask.
    pub fn creature_colour(&self) -> render::CreatureColour {
        self.creature_colour.unwrap_or(render::CreatureColour::Off)
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
            .chain(self.roster_bar.widgets.iter())
            .chain(self.panel_bar.widgets.iter())
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
            .or_else(|| self.roster_bar.hit(x, y))
            .or_else(|| self.panel_bar.hit(x, y))
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

    // ------------------------------------------------------------- roster

    /// The individual the interface is holding on to, if any.
    pub fn pinned(&self) -> Option<roster::Individual> {
        self.pinned
    }

    /// Hold on to one individual. Pinning the one already pinned lets it go,
    /// which is `inspect`'s own gesture and the one a player already knows
    /// from this interface.
    pub fn pin(&mut self, who: roster::Individual) -> bool {
        if self.pinned == Some(who) {
            self.pinned = None;
            self.following = false;
            false
        } else {
            self.pinned = Some(who);
            true
        }
    }

    /// Let the pinned individual go, and stop chasing it.
    pub fn release_pin(&mut self) {
        self.pinned = None;
        self.following = false;
    }

    /// How many samples the watch ring holds — the count a contact sheet
    /// must print beside a trail. An image says *what* and *where*; only a
    /// number says the ring fired at all, and a trail of one dot and a trail
    /// that was never sampled look identical.
    pub fn watch_len(&self) -> usize {
        self.watch.len()
    }

    /// How many simulated frames the trail spans.
    pub fn watch_span(&self) -> u64 {
        self.watch.span()
    }

    /// Whether `who` is on the spare list.
    pub fn is_spared(&self, who: roster::Individual) -> bool {
        self.spared.contains(&who)
    }

    /// How many individuals are spared.
    pub fn spared_count(&self) -> usize {
        self.spared.len()
    }

    /// Who is spared. For a harness or a guard that has to check the *right*
    /// ones lived: a population count cannot, because the box breeds while
    /// the culled bodies rot and a birth looks exactly like a survival.
    pub fn spared_list(&self) -> &[roster::Individual] {
        &self.spared
    }

    /// Spare the pinned individual, or stop sparing it. Returns what to say.
    pub fn toggle_spared(&mut self) -> String {
        let Some(who) = self.pinned else {
            return "PIN ONE FIRST".to_string();
        };
        match self.spared.iter().position(|s| *s == who) {
            Some(i) => {
                self.spared.remove(i);
                format!("NO LONGER SPARED -- {} KEPT", self.spared.len())
            }
            None => {
                self.spared.push(who);
                format!("SPARED -- {} KEPT", self.spared.len())
            }
        }
    }

    /// Forget every spared individual.
    pub fn clear_spared(&mut self) {
        self.spared.clear();
    }

    /// **Who a cull-the-rest would take**, resolved against the world now.
    ///
    /// Returned rather than acted on, so `Lab` owns the killing and this owns
    /// the selection -- the same split every other verb on this page uses.
    /// Computed from the *kingdom*, not from the visible rows: culling only
    /// what a filter happens to be showing would make the same button mean
    /// different things depending on a chip pressed two clicks ago, and a
    /// destructive verb is the last place for that.
    ///
    /// **Already-rotting rows are not targets**, and that is what makes the
    /// count on the button honest. A cull is graded, so the culled stay in
    /// the table saying ROTTING for thousands of frames; counting them left
    /// the face reading `CULL REST 13` immediately after a press that had
    /// taken all thirteen, so a button whose whole job is to say what it will
    /// do was promising thirteen deaths and delivering none.
    pub fn cull_rest_targets(&self, world: &World, kingdom: roster::Kingdom) -> Vec<roster::Individual> {
        roster::rows(world, kingdom, roster::SortKey::Slot, false, roster::Filter::All)
            .into_iter()
            .filter(|r| !matches!(r.state, roster::RowState::Senescent))
            .map(|r| r.who)
            .filter(|w| !self.spared.contains(w))
            .collect()
    }

    /// The individual held for comparison, if any.
    pub fn held(&self) -> Option<roster::Individual> {
        self.held
    }

    /// **The `VS` gesture.** Returns what to say about it.
    ///
    /// Hold the pin when nothing is held, or when the held one *is* the pin
    /// (so pressing it twice on one row is a no-op that reads as one); open
    /// the comparison once the pin has moved to somebody else. Returning the
    /// notice rather than setting it keeps `Ui` out of the business of
    /// phrasing, which is `Lab::act`'s throughout this interface.
    pub fn compare_or_hold(&mut self) -> (String, bool) {
        let Some(pin) = self.pinned else {
            return ("PIN ONE FIRST".to_string(), false);
        };
        if self.held == Some(pin) {
            self.held = None;
            return ("LET THE FIRST ONE GO".to_string(), false);
        }
        match self.held {
            None => {
                self.held = Some(pin);
                ("HELD. NOW PIN ANOTHER AND PRESS VS".to_string(), false)
            }
            Some(_) => ("SIDE BY SIDE".to_string(), true),
        }
    }

    /// Forget the held individual. Called when the pin is released, because a
    /// half-set comparison outliving the thing that set it is a chip that
    /// says VS and does nothing recognisable.
    pub fn clear_held(&mut self) {
        self.held = None;
    }

    pub fn following(&self) -> bool {
        self.following
    }

    /// Chase the pin, or stop. Refused with no pin: following nothing would
    /// latch a button that does nothing, which is worse than a button that
    /// says no.
    pub fn toggle_following(&mut self) -> bool {
        self.following = self.pinned.is_some() && !self.following;
        self.following
    }

    /// **Which roster is open**, or `None` when neither is.
    ///
    /// Every roster mutator below goes through this rather than taking a
    /// kingdom argument: the page that is open is the page the click landed
    /// on, and a caller passing the other one would be a second source of
    /// truth about which table is on screen.
    pub fn roster_kingdom(&self) -> Option<roster::Kingdom> {
        match self.panel {
            Some(Panel::PlantList) => Some(roster::Kingdom::Plants),
            Some(Panel::AntList) => Some(roster::Kingdom::Creatures),
            _ => None,
        }
    }

    fn view(&self, kingdom: roster::Kingdom) -> RosterView {
        self.roster_view[view_slot(kingdom)]
    }

    fn view_mut(&mut self, kingdom: roster::Kingdom) -> &mut RosterView {
        &mut self.roster_view[view_slot(kingdom)]
    }

    /// Which column the open roster is sorted on, and which way.
    pub fn roster_sort(&self) -> Option<(usize, bool)> {
        self.roster_kingdom().and_then(|k| self.view(k).sort)
    }

    /// What the open roster is filtered to.
    pub fn roster_filter(&self) -> roster::Filter {
        self.roster_kingdom().map(|k| self.view(k).filter).unwrap_or_default()
    }

    /// The sort the roster page is *actually* drawn with, resolved through the
    /// kingdom's own column list.
    ///
    /// **The one accessor `Lab` rebuilds the row list through**, so a click
    /// cannot be resolved against a different order than the one on screen.
    /// The alternative -- `Lab` re-deriving the key from the column index --
    /// is the second copy of the arithmetic this module exists to avoid.
    pub fn roster_sort_key(&self, kingdom: roster::Kingdom) -> (roster::SortKey, bool) {
        match self.view(kingdom).sort {
            Some((c, desc)) => {
                let cols = roster_cols(kingdom);
                (cols[c.min(cols.len() - 1)].2, desc)
            }
            None => (roster::SortKey::Slot, false),
        }
    }

    /// Stop chasing, without letting go. What a death does: the pin is worth
    /// keeping (the page has something to say about it) and the chase is not.
    pub fn stop_following(&mut self) {
        self.following = false;
    }

    /// Point the cell page at a world cell **without the toggle**.
    ///
    /// `inspect` toggles, which is right for a click on the ground and wrong
    /// for this: the pin re-points the page every frame, and a toggle would
    /// make the page flicker on and off at sixty hertz the moment an animal
    /// stood still. Same distinction, and the same reason, as `arm_tool`
    /// against `set_tool`.
    ///
    /// **It takes the individual too, and that is not decoration.** The cell
    /// page keeps a second field, `inspect_organism`, and `follow_inspected`
    /// re-points the page at *that* organism's nearest cell once per frame.
    /// Setting only the cell left the latch on whoever was clicked in the
    /// world last, so `follow_inspected` dragged the page straight back one
    /// frame later: pinning an ant from the roster put the marker and the
    /// FOLLOW camera on the ant and left the page reading a plant across the
    /// bed, for as long as the pin was held. Found by cropping a contact
    /// sheet -- the row was highlighted, the pin resolved, and the page was
    /// simply somebody else's.
    pub fn inspect_at(&mut self, cell: (i32, i32), organism: u16) {
        self.inspect = Some(cell);
        self.inspect_organism = Some(organism);
    }

    /// Sort the open roster on a column, or reverse it if it is already the
    /// one.
    ///
    /// **Descending first**, for `sort_chambers`' reason: every column here
    /// is a "how much", and the row worth looking at in a bed of a hundred is
    /// the extreme one, so one click puts it at the top rather than at the
    /// bottom.
    pub fn sort_roster(&mut self, col: usize) {
        let Some(k) = self.roster_kingdom() else { return };
        let v = self.view_mut(k);
        v.sort = match v.sort {
            Some((c, desc)) if c == col => Some((c, !desc)),
            _ => Some((col, true)),
        };
        // A sort moves rows under the cursor, so the window goes back to the
        // top: staying at row 30 of a list that has just been reordered shows
        // you a window you did not choose.
        v.scroll = 0;
    }

    /// Move the open roster's window by one page. Clamped in `paint_roster`
    /// against the list as it actually is, which matters more here than
    /// anywhere else on this interface: a roster changes length between two
    /// frames with nobody touching anything.
    pub fn scroll_roster(&mut self, direction: i32) {
        let Some(k) = self.roster_kingdom() else { return };
        let page = ROSTER_ROWS.max(1);
        let v = self.view_mut(k);
        v.scroll = (v.scroll as i32 + direction * page as i32).max(0) as usize;
    }

    pub fn roster_scroll(&self) -> usize {
        self.roster_kingdom().map(|k| self.view(k).scroll).unwrap_or(0)
    }

    /// Cycle what the open roster shows. `line` is the pinned individual's
    /// founding line, or `None` when nothing is pinned -- in which case the
    /// LINE state is skipped rather than shown empty, because a filter on a
    /// line nobody chose keeps nothing and reads as a broken page.
    pub fn cycle_roster_filter(&mut self, line: Option<u32>) -> String {
        let Some(k) = self.roster_kingdom() else { return String::new() };
        let v = self.view_mut(k);
        // ALL -> IN TROUBLE -> DEAD -> (LINE n) -> ALL. The graveyard sits
        // after the living states and before the lineage cut, so the cycle
        // reads as "everything, then what is going wrong, then what already
        // did" -- and a player who only wants the living never passes through
        // two thousand dead rows to get back.
        v.filter = match (v.filter, line) {
            (roster::Filter::All, _) => roster::Filter::Trouble,
            (roster::Filter::Trouble, _) => roster::Filter::Dead,
            (roster::Filter::Dead, Some(l)) => roster::Filter::Lineage(l),
            (roster::Filter::Dead, None) => roster::Filter::All,
            (roster::Filter::Lineage(_), _) => roster::Filter::All,
        };
        v.scroll = 0;
        format!("SHOWING {}", v.filter.label())
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

    /// Move the rack window by one page. Clamped in `paint_rack` against the
    /// list as it actually is, the way the parameters page does it -- closing
    /// a chamber or landing a batch row changes the length under the scroll.
    pub fn scroll_rack(&mut self, direction: i32) {
        let page = RACK_ROWS.max(1);
        self.rack_scroll = (self.rack_scroll as i32 + direction * page as i32).max(0) as usize;
    }

    pub fn rack_scroll(&self) -> usize {
        self.rack_scroll
    }

    /// Collapse the rack to one row per swept setting, or expand it again.
    pub fn toggle_rack_grouping(&mut self) {
        self.rack_grouped = !self.rack_grouped;
        // The list changes length underneath, so the window goes back to the
        // top for `sort_chambers`' reason.
        self.rack_scroll = 0;
    }

    pub fn rack_grouped(&self) -> bool {
        self.rack_grouped
    }

    /// Start typing into `field`, from empty.
    ///
    /// **Empty rather than pre-filled with the current value.** Typing a
    /// fresh number is the common case and starting from `9000` means
    /// clearing it first; the old value is still on screen until the new one
    /// is committed, and `Escape` puts it back.
    pub fn begin_typing(&mut self, field: TypedField) {
        self.typing = Some((field, String::new()));
    }

    pub fn typing(&self) -> Option<(TypedField, &str)> {
        self.typing.as_ref().map(|(f, b)| (*f, b.as_str()))
    }

    /// Take one digit. Anything else is ignored, and the buffer is capped at
    /// the width of the largest number either dial accepts.
    pub fn type_digit(&mut self, c: char) {
        if let Some((_, buf)) = &mut self.typing {
            if c.is_ascii_digit() && buf.len() < 7 {
                buf.push(c);
            }
        }
    }

    pub fn type_backspace(&mut self) {
        if let Some((_, buf)) = &mut self.typing {
            buf.pop();
        }
    }

    /// Finish, and hand back what was typed. An empty buffer commits nothing,
    /// so pressing enter without typing leaves the dial where it was.
    pub fn commit_typing(&mut self) -> Option<(TypedField, u64)> {
        let (field, buf) = self.typing.take()?;
        buf.parse::<u64>().ok().map(|v| (field, v))
    }

    pub fn cancel_typing(&mut self) {
        self.typing = None;
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

    /// Which of the two stocking counts the armed tool reads. See
    /// [`Ui::stock`]'s field.
    fn stock_slot(&self) -> usize {
        usize::from(self.tool == Tool::Release)
    }

    /// How many animals a stocking click puts down, for the armed verb.
    pub fn stock(&self) -> i32 {
        STOCK_LADDER[self.stock[self.stock_slot()].min(STOCK_LADDER.len() - 1)]
    }

    /// Move the stocking dial one stop. Clamped at both ends rather than
    /// wrapped: a dial that jumped from one animal to a hundred and four
    /// because you clicked once too often is a dial that empties a box.
    pub fn adjust_stock(&mut self, delta: i32) {
        let last = STOCK_LADDER.len() as i32 - 1;
        let slot = self.stock_slot();
        self.stock[slot] = (self.stock[slot] as i32 + delta).clamp(0, last) as usize;
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
    /// **Two individuals, paired by row label, with what differs marked.**
    ///
    /// Built out of `params::specimen_rows` rather than out of a second copy
    /// of that arithmetic, so a number can never read one way here and
    /// another way on the cell page -- which would be the worst possible
    /// defect on a page whose entire job is comparison.
    ///
    /// **Paired by label, and the union rather than the intersection.** Two
    /// animals produce the same rows, but a plant against an animal does not,
    /// and the rows that exist for only one of them are exactly the ones
    /// worth seeing (`SEEDS SET` against `DELIVERED` says which kingdom you
    /// picked by mistake). A missing side reads `--`, which is a fact about
    /// that individual rather than a gap in the page.
    ///
    /// **Rows whose label carries a frame number never pair, by design.**
    /// `specimen_rows` stamps a few life events into the label itself
    /// (`F636 FIRST FED`), so two individuals that both fed produce two rows
    /// rather than one, each with `--` on the other side. That reads
    /// correctly -- it says when *each* of them first fed, which is the fact
    /// -- and it does inflate the differing count by one per such event.
    ///
    /// **The difference is marked, not computed.** A numeric delta was the
    /// obvious build and is wrong here: half these rows are not numbers
    /// (`ORIGIN FOUNDER`, `CROP EMPTY`, `STARVING NO`), and a page that
    /// subtracts where it can and does not where it cannot is a page whose
    /// blank cells mean two different things. Same-or-different is defined
    /// for every row.
    fn compare_rows(&self, world: &World) -> Vec<Row> {
        let (Some(a), Some(b)) = (self.held, self.pinned) else {
            return vec![Row::value(
                "NOTHING TO COMPARE",
                "--",
                FAINT,
                "PIN AN INDIVIDUAL, PRESS HOLD, THEN PIN A SECOND ONE AND PRESS VS. THIS PAGE NEEDS TWO.".to_string(),
            )];
        };
        let name = |who: roster::Individual| -> String {
            match who.resolve(world) {
                Some(st) => format!("{} {}", param_label(&world.species.get(st.species).name), who.id),
                // **Dead is a legitimate side of a comparison and is the
                // interesting one.** "Why did this one die and that one not"
                // is the question; a page that refused to show a corpse would
                // refuse exactly when it is most wanted. The graveyard keeps
                // the record, so the rows below still have something to read.
                None => format!("#{} (DIED)", who.id),
            }
        };
        // **Back to the list it was opened from, not to a fixed page.** Both
        // individuals were chosen there, so it is where a player who wants a
        // different pair is going; `paint_roster` learned the same lesson the
        // hard way, that a page a click cannot leave is a page a player
        // cannot leave.
        let home = match self.pinned.and_then(|w| w.resolve(world)) {
            Some(st) if world.species.get(st.species).creature.is_some() => Panel::AntList,
            _ => Panel::PlantList,
        };
        let mut rows = vec![
            Row::head("BACK TO THE LIST", false, 0, Action::Panel(home), "RETURN TO THE ROSTER BOTH OF THESE WERE PICKED FROM."),
            Row::value(
                "HELD",
                name(a),
                TITLE,
                "THE ONE YOU PRESSED HOLD ON. IT IS NOT MARKED IN THE BOX -- ONLY THE PIN IS, BECAUSE TWO MARKERS ON ONE BED IS TWO THINGS TO FIND RATHER THAN ONE.".to_string(),
            ),
            Row::value(
                "PINNED",
                name(b),
                TITLE,
                "THE ONE CURRENTLY PINNED, MARKED IN THE BOX. PIN A DIFFERENT ROW AND COME BACK AND THIS SIDE CHANGES -- THE HELD ONE STAYS PUT.".to_string(),
            ),
            Row::gap(),
        ];

        let left = params::specimen_rows(world, a.id);
        let right = params::specimen_rows(world, b.id);
        let find = |rs: &[(String, String, String)], label: &str| -> Option<(String, String)> {
            rs.iter().find(|(l, _, _)| l == label).map(|(_, v, n)| (v.clone(), n.clone()))
        };
        let mut seen: Vec<String> = Vec::new();
        let mut differing = 0usize;
        let (mut unlike, mut alike): (Vec<Row>, Vec<Row>) = (Vec::new(), Vec::new());
        for (label, _, _) in left.iter().chain(right.iter()) {
            if seen.iter().any(|s| s == label) {
                continue;
            }
            seen.push(label.clone());
            let lv = find(&left, label);
            let rv = find(&right, label);
            let note = lv.as_ref().or(rv.as_ref()).map(|(_, n)| n.clone()).unwrap_or_default();
            let (lt, rt) = (
                lv.map(|(v, _)| v).unwrap_or_else(|| "--".to_string()),
                rv.map(|(v, _)| v).unwrap_or_else(|| "--".to_string()),
            );
            let same = lt == rt;
            if !same {
                differing += 1;
            }
            let row = Row::value(label, format!("{lt}  |  {rt}"), if same { FAINT } else { VALUE }, note);
            if same { alike.push(row) } else { unlike.push(row) }
        }
        // **What differs comes first, and that is the page's whole argument.**
        // Kept in specimen order the two shipped ants put fourteen identical
        // plain-speech lines above the nine rows that actually differed, so
        // the answer to "why is this one doing better" was below the fold on
        // a page opened to ask exactly that. `fit_rows` trims from the
        // bottom, so ordering is also what decides which rows survive a page
        // that overruns -- the identical ones are the right ones to lose.
        let alike_count = alike.len();
        rows.extend(unlike);
        rows.extend(alike);
        // **The count, and it is not decoration.** A page of forty rows all
        // dimmed reads identically to a page that failed to load, and this
        // interface has already paid for that once -- a collapse read as
        // "chunks are working" off a picture whose body count was zero all
        // run. Two clones of one genome legitimately differ nowhere, and the
        // number is what says so out loud.
        rows.insert(
            3,
            Row::value(
                "DIFFER",
                format!("{differing} OF {}", differing + alike_count),
                if differing == 0 { FAINT } else { VALUE },
                "HOW MANY ROWS BELOW ARE NOT IDENTICAL. THEY ARE LISTED FIRST, BRIGHT, WITH THE MATCHING ONES DIMMED UNDER THEM -- SO WHAT YOU OPENED THIS PAGE TO SEE IS AT THE TOP OF IT. ZERO IS A REAL ANSWER AND NOT A BROKEN PAGE: TWO COPIES OF ONE JAR, PLACED THE SAME MOMENT, DIFFER NOWHERE UNTIL THE BOX HAS DONE SOMETHING TO THEM.".to_string(),
            ),
        );
        rows
    }

    fn panel_rows(&self, panel: Panel, world: &World, spec: &LabBox, fps: f32) -> Vec<Row> {
        let orgs = world.live_organism_count();
        let ants = world.live_creature_count();
        let plants = orgs.saturating_sub(ants);
        match panel {
            // **Draws itself** -- see `paint_params`. Its rows carry buttons
            // and a range, so they are not `Row`s; `draw` branches away before
            // this is called, and the arm is here so that a page added to
            // `Panel` cannot be silently left out of both.
            Panel::Params | Panel::Shelf | Panel::Chambers | Panel::PlantList | Panel::AntList => Vec::new(),
            Panel::Compare => self.compare_rows(world),
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
                        format!(
                            "HOW THE STANDING COUNT MOVED ACROSS THE LAST TWO SAMPLES, {} SIMULATED FRAMES APART. A STILL PICTURE CANNOT SHOW WHETHER A BOX FULL OF GREEN IS BREEDING OR DYING; THIS CAN.",
                            match self.history.gap() {
                                Some(g) => g.to_string(),
                                None => "--".to_string(),
                            }
                        ),
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
                    // **The way in to the roster**, and the reason it is a
                    // heading rather than a chip: `Body::Head` already pushes
                    // a full-width invisible hit target, so this cost no new
                    // painter code -- and the bar has no room for a seventh
                    // button, measured twice (see `Panel::PlantList`).
                    Row::head(
                        "LIST EVERY PLANT",
                        false,
                        plants,
                        Action::Panel(Panel::PlantList),
                        "OPEN THE ROSTER: ONE ROW PER PLANT, SORTABLE, AND CLICKING A ROW PUTS A MARKER ROUND THAT PLANT IN THE BOX AND ITS NUMBERS ON SCREEN. THIS PAGE IS THE STAND; THAT ONE IS THE INDIVIDUALS IN IT.",
                    ),
                ]
            }
            Panel::Ants => {
                let (d, tint) = delta_text(self.history.delta(|s| s.ants as i64));
                let (allocated, live) = world.organism_slot_usage();
                // **Which grouping the chart and legend read off the world.**
                // Follows the renderer's own creature-colour mode, not a
                // separate switch on this page -- the owner's rule: graph by
                // colony only when the box is painting colonies, otherwise by
                // species, so the graph and the animals in the box are always
                // answering the same question. `Off` falls to `Species` here,
                // same as `render::group_colour` does for the colour itself --
                // an animal wearing its own material still belongs to a
                // species, even though it is not drawing that species' colour
                // on its body.
                let colony_mode = self.creature_colour() == render::CreatureColour::Colony;
                let paint_mode =
                    if colony_mode { render::CreatureColour::Colony } else { render::CreatureColour::Species };
                let remembered = self.history.remembered_groups();
                let (shown, dropped) = capped_group_rows(world, colony_mode, &remembered);
                let series: Vec<(Vec<u32>, [u8; 4])> = shown
                    .iter()
                    .map(|&(species, colony, _)| {
                        let s = if colony_mode {
                            self.history.group_series(move |sp, co| sp == species && co == colony)
                        } else {
                            self.history.group_series(move |sp, _| sp == species)
                        };
                        (s, tint_of(render::group_colour(paint_mode, species, colony).unwrap_or(render::GROUP_NONE)))
                    })
                    .collect();
                let peak = series.iter().flat_map(|(s, _)| s.iter().copied()).max().unwrap_or(0);
                // **One legend row per shown group**, built while `series` is
                // still borrowed rather than after `Row::lines` below moves
                // it -- the two need the same per-group series (one for the
                // line, one for its own delta) and this is the one pass that
                // reads it for both.
                let legend: Vec<Row> = shown
                    .iter()
                    .zip(series.iter())
                    .map(|(&(species, colony, alive), (s, group_tint))| {
                        let gd = (s.len() >= 2).then(|| s[s.len() - 1] as i64 - s[s.len() - 2] as i64);
                        let (gd_text, _) = delta_text(gd);
                        let name = world.species.get(species).name.to_uppercase();
                        let label = if colony_mode { format!("{name} {colony}") } else { name };
                        // **What the dead died of, on the row and not only
                        // in the note**, because *"one almost got wiped
                        // out, then came back"* is a line on the chart and
                        // whether it was eaten or starved is the whole
                        // question. Kills are on the face whenever there
                        // are any; the starved count and the killers by
                        // name are in the note, which is where a number
                        // that does not fit on the row goes on this page.
                        let (killed, starved, killers) = group_losses(world, species, colony_mode.then_some(colony));
                        let face = if killed > 0 { format!("{alive}  {gd_text}  K{killed}") } else { format!("{alive}  {gd_text}") };
                        let mut note = format!(
                            "THIS GROUP'S ANIMALS ALIVE NOW, HOW THAT COUNT MOVED SINCE THE LAST SAMPLE, AND K FOR HOW MANY OF ITS DEAD WERE KILLED BY ANOTHER ANIMAL. TINTED TO MATCH ITS LINE ABOVE AND THE ANIMALS WEARING IT IN THE BOX. DEAD SO FAR: {killed} KILLED, {starved} STARVED."
                        );
                        if !killers.is_empty() {
                            let who: Vec<String> = killers.iter().map(|(k, n)| format!("{k} X{n}")).collect();
                            note.push_str(&format!(" KILLED BY {}.", who.join(", ")));
                        }
                        Row::value(label, face, *group_tint, note)
                    })
                    .collect();
                let mut rows = vec![
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
                    // **The toggle the chart and the animals in the box both
                    // answer to.** A row rather than a bar chip -- the bar is
                    // full, measured twice (`CLAUDE.md`), and `Body::Head`
                    // already proved a page row can carry a click for free.
                    Row::choice(
                        "ANIMALS WEAR",
                        self.creature_colour().label(),
                        Action::CycleCreatureColour,
                        "WHAT COLOUR EVERY ANIMAL WEARS IN THE BOX, AND WHAT THE CHART AND LEGEND BELOW GROUP AND COLOUR BY. CLICK TO CYCLE: OWN COLOUR, BY SPECIES, BY COLONY. THE CHART GROUPS BY COLONY ONLY IN THAT LAST MODE -- OTHERWISE IT SUMS EVERY COLONY OF A SPECIES INTO ONE LINE.",
                    ),
                    // **Replaces the single TREND spark.** One line per
                    // group on one shared axis, so two groups fighting can be
                    // read off directly -- the gap the owner reported after
                    // watching an ant colony and a beetle group fight with
                    // only the combined total to graph it by.
                    Row::lines(
                        format!("GROUPS  PEAK {peak}"),
                        series,
                        "EACH LINE IS ONE GROUP'S POPULATION, ALL ON ONE SCALE -- UNLIKE A SEPARATE STRIP PER GROUP, TWO GROUPS ARE COMPARABLE DIRECTLY RATHER THAN EACH FILLING ITS OWN AXIS. A GROUP THAT DIES OUT DRAWS FALLING TO THE FLOOR, NOT AS A SHORTER LINE. SAME COLOUR AS THE ANIMALS WEARING IT IN THE BOX.",
                    ),
                ];
                rows.extend(legend);
                if dropped > 0 {
                    rows.push(Row::value(
                        "MORE GROUPS",
                        format!("+{dropped}"),
                        FAINT,
                        "MORE GROUPS THAN THIS PAGE HAS ROOM TO GRAPH. THE TWELVE WITH THE MOST ANIMALS ALIVE ARE DRAWN; THE REST ARE STILL COUNTED IN ALIVE ABOVE AND STILL WEAR THEIR OWN COLOUR IN THE BOX.",
                    ));
                }
                rows.push(Row::gap());
                rows.push(Row::head(
                    "LIST EVERY ANIMAL",
                    false,
                    ants,
                    Action::Panel(Panel::AntList),
                    "OPEN THE ROSTER: ONE ROW PER ANIMAL, SORTABLE, AND CLICKING A ROW PUTS A MARKER ROUND THAT ANIMAL AND ITS NUMBERS ON SCREEN. AN ANT IS TWO DARK CELLS AT PLAY ZOOM AND YOU FIND IT ONLY BECAUSE IT MOVES, WHICH IS THE WHOLE REASON THIS LIST EXISTS.",
                ));
                rows
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
                // The way in to the log, on the same mechanism as the two
                // rosters: a `Body::Head` already carries a hit target, and
                // the bar has no room for a seventh chip.
                Row::head(
                    "WHAT HAPPENED",
                    false,
                    world.run_log.len(),
                    Action::Panel(Panel::Log),
                    "OPEN THE RUN LOG: BIRTHS, DEATHS AND FIRSTS, NEWEST FIRST. EVERY OTHER PAGE SAYS WHAT THE BOX IS LIKE NOW; AT 1024X YOU CROSS TENS OF THOUSANDS OF FRAMES BETWEEN TWO GLANCES AT IT, AND A COUNT THAT MOVED DOES NOT SAY WHOSE LINE ENDED.",
                ),
            ],
            Panel::Log => self.log_rows(world),
        }
    }

    /// The run log as rows, newest first.
    ///
    /// **Bounded twice, and both bounds are stated on the page.** The log
    /// itself drops its oldest lines past `world::RUN_LOG_CAP`, and this page
    /// shows only the newest `LOG_ROWS` of what survives. Neither is allowed
    /// to read as *nothing happened*: the last row says how many lines are
    /// off the bottom and how many are gone for good. A trimmed history that
    /// looks like a quiet one is the same failure as a zero body count read
    /// as "chunks are working".
    fn log_rows(&self, world: &World) -> Vec<Row> {
        let mut rows = vec![Row::head(
            "BACK TO THE BOX",
            false,
            0,
            Action::Panel(Panel::Box),
            "RETURN TO THE BOX PAGE.",
        )];
        let shown: Vec<&world::LogEvent> = world.run_log.recent().take(LOG_ROWS).collect();
        if shown.is_empty() {
            rows.push(Row::value(
                "NOTHING YET",
                "--".to_string(),
                FAINT,
                "NO BIRTH, DEATH OR FIRST HAS HAPPENED SINCE THE BOX WAS BUILT. THIS IS AN EMPTY LOG, NOT A TRIMMED ONE -- THE ROW BELOW SAYS WHICH.",
            ));
        }
        for e in shown.iter() {
            let who = param_label(&world.species.get(e.species).name);
            let (what, tint, note) = match e.kind {
                world::LogKind::Born => (
                    format!("{who} {} BORN", e.id),
                    GOOD,
                    format!(
                        "A SEED GERMINATED, OR AN ANIMAL BUDDED FROM {}. THE NUMBER IS THE SLOT IT HOLDS, WHICH IS REUSED AFTER 16 TURNS -- THE ROSTER PINS BY SLOT AND BIRTH FRAME TOGETHER FOR THAT REASON.",
                        if e.other == 0 { "NOTHING".to_string() } else { format!("PARENT {}", e.other) }
                    ),
                ),
                world::LogKind::Died => (
                    format!("{who} {} {}", e.id, cause_of(e.other)),
                    POOR,
                    "IT LEFT THE WORLD, AND WHAT KILLED IT. FELLED OR LOST IS THE ONE THAT IS NOT A DEATH THE ENGINE INTENDED: THE PLANT WAS PULLED APART BY THE SUPPORT CHECK WHILE STILL ALIVE.".to_string(),
                ),
                world::LogKind::FirstFeed => (
                    format!("{who} {} FIRST FED", e.id),
                    VALUE,
                    "THE FIRST TIME THIS ANIMAL EVER PICKED FOOD UP. ONE PER LIFE -- EVERY LATER MOUTHFUL IS COUNTED ON ITS OWN PAGE, NOT HERE, BECAUSE A LOG OF EVERY BITE WOULD DROWN EVERYTHING WORTH READING.".to_string(),
                ),
                world::LogKind::FirstSeed => (
                    format!("{who} {} FIRST SEED", e.id),
                    VALUE,
                    "THE FIRST SEED THIS PLANT EVER SET -- THE MOMENT IT STOPPED BEING A COST TO THE BOX AND STARTED BEING A PARENT. LATER SEEDS ARE COUNTED ON ITS OWN PAGE: THE SHIPPED BED SETS 3,099 OF THEM IN 90,000 FRAMES AND A LOG OF ALL OF THEM WOULD BE 90% SEED-SET.".to_string(),
                ),
                world::LogKind::LineEnded => (
                    format!("LINE {} ENDED", e.other),
                    POOR,
                    "THE LAST LIVING MEMBER OF A LINEAGE LEFT THE WORLD. THIS IS THE EVENT NO STANDING COUNT CAN SHOW: THE POPULATION NUMBER CAN HOLD PERFECTLY STEADY WHILE THE BOX QUIETLY LOSES EVERY DESCENDANT OF ONE FOUNDER.".to_string(),
                ),
            };
            rows.push(Row::value(format!("F{}", e.frame), what, tint, note));
        }
        let below = world.run_log.len().saturating_sub(shown.len());
        let dropped = world.run_log.dropped();
        rows.push(Row::value(
            "OLDER",
            if dropped == 0 { format!("{below} MORE") } else { format!("{below} MORE, {dropped} LOST") },
            FAINT,
            "HOW MANY LINES ARE OFF THE BOTTOM OF THIS PAGE, AND HOW MANY HAVE AGED OUT OF THE LOG FOR GOOD. THE SECOND NUMBER IS WHY NOTHING IN THE LAB IS EVER COUNTED OFF THIS PAGE -- THE COUNTS COME FROM EACH INDIVIDUAL'S OWN TOTALS, WHICH ARE NEVER TRIMMED.",
        ));
        rows
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
    /// **The pinned individual's own series** — two sparklines for the group
    /// that moves.
    ///
    /// Empty unless the pin is on the organism whose page this is, so a page
    /// never carries somebody else's history. Empty is the right answer and
    /// not a failure: nothing is pinned yet, or the ring has not filled, and
    /// in both cases the page is simply the page it always was.
    fn watch_rows(&self, id: u16) -> Vec<Row> {
        if !self.watch.about(id) {
            return Vec::new();
        }
        let span = self.watch.span();
        let (lo, hi) = self.watch.range(|t| t.energy);
        let (clo, chi) = self.watch.range(|t| t.cells as f32);
        let mut out = Vec::new();
        out.push(Row::spark(
            format!("BANK {lo:.0}-{hi:.0}"),
            self.watch.series(|t| t.energy),
            if hi > 0.0 && lo < hi * 0.5 { FAIR } else { GOOD },
            format!("THIS ONE'S OWN ENERGY OVER THE LAST {} SIMULATED FRAMES, OLDEST ON THE LEFT, ONE SAMPLE EVERY {WATCH_EVERY}. THE BARS ARE SCALED TO ITS OWN PEAK, SO THE CAPTION CARRIES THE RANGE -- A FLAT FULL STRIP AT 95-96 AND ONE AT 0-200 DRAW THE SAME SHAPE. AN ANIMAL WHOSE SAWTOOTH STOPS CLIMBING BACK IS ONE THAT HAS STOPPED FINDING FOOD.", span),
        ));
        out.push(Row::spark(
            format!("BODY {clo:.0}-{chi:.0} CELLS"),
            self.watch.series(|t| t.cells as f32),
            VALUE,
            "HOW MANY CELLS IT HAS HELD OVER THE SAME WINDOW. FOR A PLANT THIS IS GROWTH, AND A STEP DOWN IS SOMETHING EATING IT OR A BRANCH COMING OFF; FOR AN ANIMAL IT IS FLAT UNLESS SOMETHING HAS BITTEN A PIECE OUT OF IT.".to_string(),
        ));
        out
    }

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
        // **The chosen group opens whether or not it fits, and that is a
        // change.** The rule used to be `cost <= room` for every group
        // including the chosen one, which reads as "serve it first" and is
        // not: a group *larger than the whole budget* could never satisfy it,
        // so clicking its heading did nothing at all, for ever. Nothing had
        // hit it while the biggest group was eleven rows against a
        // fourteen-row budget; `WORDS` is eighteen and hit it immediately.
        //
        // A group that overruns is trimmed by `fit_rows` at the end of this
        // function, which prints how many rows it dropped -- so an oversized
        // group is shown short and says so, rather than being silently
        // unopenable. A heading that does nothing when clicked is the worse
        // of the two by a long way.
        // **The watched individual's own series, which belong in `STATE` and
        // nowhere else.** `STATE` is defined as the group that moves while
        // the box runs, and a series is precisely the record of it moving.
        // The alternative -- a fifth group of its own -- was measured and does
        // not fit: giving the run-log timeline its own heading cost ~15px and
        // put the page over the screen, and that is the same ~15px again.
        let mut sparks = self.watch_rows(cell.organism_id());
        let state_at = sections.iter().position(|(l, _, _)| *l == "STATE");
        let spark_h: i32 = sparks.iter().map(Row::height).sum();

        open[chosen] = true;
        room -= sections[chosen].2.len() as i32 * LINE;
        if state_at == Some(chosen) {
            room -= spark_h;
        }
        for i in 0..sections.len() {
            let cost = sections[i].2.len() as i32 * LINE
                + if state_at == Some(i) { spark_h } else { 0 };
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
                if state_at == Some(i) {
                    // Moved rather than copied: `Row` is not `Clone` and does
                    // not need to be -- there is one `STATE` group, so these
                    // rows have exactly one destination.
                    rows.append(&mut sparks);
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
            Body::Lines { caption, series } => {
                draw_lines(frame, Rect { x: left, y, w: right - left, h: CHART_H }, series);
                text(frame, left, y + CHART_H + 2, caption, FAINT);
            }
            Body::Choice { label, value, action } => {
                text(frame, left, y, label, FAINT);
                text(frame, right - hud::text_width(value), y, value, GOOD);
                // The same invisible full-width hit target `Head` pushes
                // below -- a `Choice` is a `Head` that draws as a value row
                // instead of a heading, and the click mechanism does not
                // care which one drew it.
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

/// **Several groups' populations on one shared y-axis** -- the peak over
/// *every* series drawn, never each series' own peak the way `draw_spark`
/// normalises above. `draw_spark` is right for one quantity; it is wrong the
/// moment two are compared side by side, because each strip fills to its own
/// top regardless of the other's size -- a colony of 4 and one of 40 would
/// draw identically tall. Built for the ANTS page's per-group chart, whose
/// whole reason to exist is telling two fighting groups apart by more than a
/// colour.
///
/// Handles the two degenerate inputs without dividing by zero: no groups at
/// all (`series` empty, before anything has been founded) and no samples yet
/// in an existing group's series both print a caption instead of a chart,
/// and a shared peak of zero (every series flat at the floor) draws every
/// point on the baseline rather than panicking on a `0.0 / 0.0`.
fn draw_lines(frame: &mut [u8], area: Rect, series: &[(Vec<u32>, [u8; 4])]) {
    fill(frame, area, [24, 27, 33, 255]);
    if series.is_empty() {
        text(frame, area.x + 2, area.y + 2, "NO GROUPS YET", FAINT);
        return;
    }
    let len = series.iter().map(|(s, _)| s.len()).max().unwrap_or(0);
    if len == 0 {
        text(frame, area.x + 2, area.y + 2, "NO SAMPLES YET", FAINT);
        return;
    }
    let peak = series.iter().flat_map(|(s, _)| s.iter().copied()).max().unwrap_or(0);
    for (s, tint) in series {
        let mut prev: Option<(i32, i32)> = None;
        for (i, &v) in s.iter().enumerate() {
            // Oldest sample at the left edge, newest at the right -- same
            // convention `draw_spark`'s `bar_w` walk uses, just interpolated
            // across the strip's own width instead of stepped in bar-sized
            // columns, since several series drawn as opaque bars would hide
            // each other where they overlap and a line does not.
            let x = area.x + if len > 1 { (i as i32 * (area.w - 1)) / (len as i32 - 1) } else { 0 };
            let h = if peak == 0 { 0 } else { ((v as f32 / peak as f32) * (area.h - 1) as f32).round() as i32 };
            let y = area.bottom() - 1 - h;
            match prev {
                Some((px, py)) => draw_segment(frame, px, py, x, y, *tint),
                None => render::put(frame, W, H, x, y, *tint),
            }
            prev = Some((x, y));
        }
    }
}

/// A one-pixel-wide line between two points, for `draw_lines` above. Nothing
/// else in this module draws anything but rectangles and single pixels, so
/// there was no line primitive to reuse -- this is a plain integer
/// Bresenham, with `render::put`'s own bounds check doing the clipping
/// rather than a second one here.
fn draw_segment(frame: &mut [u8], x0: i32, y0: i32, x1: i32, y1: i32, colour: [u8; 4]) {
    let (mut x, mut y) = (x0, y0);
    let dx = (x1 - x0).abs();
    let sx: i32 = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy: i32 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        render::put(frame, W, H, x, y, colour);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
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

/// How many roster rows a page will draw at most. The ceiling; the floor is
/// whatever is left after the header, the verbs and the pager, and
/// `paint_roster` computes it.
const ROSTER_ROWS: usize = 14;

/// How one roster is being read: where it is scrolled to, what it is sorted
/// on, and what it is filtered to. One per kingdom -- see `Ui::roster_view`.
#[derive(Clone, Copy, Default)]
struct RosterView {
    scroll: usize,
    /// `None` is registry order, which carries no opinion.
    sort: Option<(usize, bool)>,
    filter: roster::Filter,
}

/// Which slot of `Ui::roster_view` a kingdom reads.
fn view_slot(kingdom: roster::Kingdom) -> usize {
    match kingdom {
        roster::Kingdom::Plants => 0,
        roster::Kingdom::Creatures => 1,
    }
}


/// **The roster's columns: a heading, the widest value it can hold, and what
/// sorting on it means.**
///
/// The third term is what stops a table and its sort drifting apart. The rack
/// keys its sort on a bare column index in a `match`, which works and means
/// an inserted column silently sorts on its neighbour; carrying the key on
/// the column makes an insert a compile-time move instead.
///
/// The second string is the **widest value the column can ever hold**, not a
/// typical one. A column sized to what it usually shows is a column that
/// breaks the first time a run gets big, which is the run you most want to
/// read. Measured through `hud::text_width` and never counted by hand -- the
/// rack's offsets were hand-counted first and overlapped by eight pixels.
type RosterCol = (&'static str, &'static str, roster::SortKey);

const PLANT_COLS: [RosterCol; 8] = [
    ("SPECIES", "SPECIES--", roster::SortKey::Species),
    ("CELLS", "CELLS", roster::SortKey::Cells),
    ("SEED", "SEED", roster::SortKey::Score),
    ("WATER", "WATER", roster::SortKey::Energy),
    ("AGE", "999.9K", roster::SortKey::Age),
    ("GEN", "GEN", roster::SortKey::Generation),
    ("LINE", "LINE9", roster::SortKey::Lineage),
    ("STATE", "STARVING", roster::SortKey::State),
];

const ANT_COLS: [RosterCol; 8] = [
    ("SPECIES", "SPECIES--", roster::SortKey::Species),
    ("BANK", "BANK9", roster::SortKey::Energy),
    ("CROP", "CROP", roster::SortKey::Carrying),
    // **`YOUNG` replaced `BODY`, and that is a judgement about what a
    // hundred-row table is for.** Body size is the same two cells on every
    // ant in the box, so the column carried no information; young is the
    // fitness figure, it is the one you sort a colony on to find what is
    // actually breeding, and it did not exist until the life record.
    ("YOUNG", "YOUNG", roster::SortKey::Score),
    ("AGE", "999.9K", roster::SortKey::Age),
    ("GEN", "GEN", roster::SortKey::Generation),
    ("LINE", "LINE9", roster::SortKey::Lineage),
    ("STATE", "STARVING", roster::SortKey::State),
];

fn roster_cols(kingdom: roster::Kingdom) -> &'static [RosterCol; 8] {
    match kingdom {
        roster::Kingdom::Plants => &PLANT_COLS,
        roster::Kingdom::Creatures => &ANT_COLS,
    }
}

/// Where each roster column starts, relative to the page's left margin.
/// Derived once and read by both the header and the rows, so the two cannot
/// drift -- a header that no longer sits over its column is worse than none.
fn roster_col_x(kingdom: roster::Kingdom) -> [i32; 8] {
    let mut out = [0i32; 8];
    // The row number sits first, outside the table.
    let mut x = hud::text_width("999") + RACK_GAP;
    for (i, (_, widest, _)) in roster_cols(kingdom).iter().enumerate() {
        out[i] = x;
        x += hud::text_width(widest) + RACK_GAP;
    }
    out
}

/// The page's width: the sum of its own columns, so it is exactly as wide as
/// it has to be and no wider.
fn roster_page_width(kingdom: roster::Kingdom) -> i32 {
    let col = roster_col_x(kingdom);
    let last = roster_cols(kingdom).last().expect("eight columns");
    (col[7] + hud::text_width(last.1) + PAGE_PAD * 2).min(W as i32 - MARGIN * 2)
}

/// **The widest the cell page can ever be drawn.**
///
/// Not a measurement of the page as it is -- a bound on every page it can
/// become. The roster's header has to stay clear of it and is painted
/// *first*, so anything read off the cell page's own rectangle is a frame
/// stale, and the one frame that matters is the frame the WORDS group opens
/// and the page suddenly widens.
///
/// The bound holds because `plainspeak` caps every phrase at
/// [`plainspeak::PHRASE_COLUMNS`] and guards it, in a test whose message
/// already says why: a wider one *"will widen the cell page and slide it
/// over the roster"*. Built through `Row::width` and `page_rect`'s own
/// arithmetic rather than written down, so it cannot drift from them.
fn widest_cell_page() -> i32 {
    let phrase = "W".repeat(plainspeak::PHRASE_COLUMNS);
    Row::value(&phrase, "", VALUE, "").width().max(120) + PAGE_PAD * 2
}

/// **What a spared row's number carries in front of it.**
///
/// A const rather than an inline `format!("*{}", ..)` so the glyph guard can
/// reach it: as a literal buried in `paint_roster` it was drawable only by
/// luck, and it was not -- the 5x7 set had no `*` and the mark shipped as a
/// blank gap. `every_string_the_bar_can_draw_is_drawable` could not have
/// caught that, because nothing it walks contained the character.
pub(crate) const SPARED_MARK: &str = "*";

/// **Every fixed string the roster page draws, in one list.**
///
/// Named rather than inlined so `every_string_the_bar_can_draw_is_drawable`
/// can reach them: this page paints itself, so nothing in `panel_rows` covers
/// it, and `hud::draw_text` renders a character outside its 5x7 set as a
/// **silent blank**. That trap has shipped three times here.
const ROSTER_LITERALS: [&str; 5] = [
    "NOTHING ALIVE IN THIS BOX YET",
    "NOTHING MATCHES THIS FILTER",
    "THIS ONE HAS DIED",
    "CLICK A ROW TO PIN ONE",
    // The graveyard's own empty state. "NOTHING MATCHES THIS FILTER" is true
    // and useless here: an empty graveyard is a box where nothing has died
    // yet, which is a fact about the run and not a filter the player should
    // go and undo.
    "NOTHING HAS DIED YET",
];

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
const RACK_COLS: [(&str, &str); 7] = [
    ("SEED", "SEED 999999"),
    // The swept setting. Dashes outside a sweep rather than a hidden column:
    // a table whose shape changes with its contents is one you cannot learn,
    // and the column that is empty today is the one a sweep fills tomorrow.
    ("SET", "99999"),
    ("FRAME", "9999999"),
    ("PLT", "9999"),
    ("ANI", "9999"),
    ("GEN", "99/99"),
    ("SOWN", "999999"),
];

/// **What the batch line says while copies are in flight.**
///
/// Hoisted out of `paint_rack` because it is drawn with `text` rather than as
/// a widget, so nothing could read it back: the one picture that should have
/// shown it had a thumbnail over it, and a line no test can see and no sheet
/// reliably shows is a line that rots.
///
/// **Ticks lead, runs follow.** Fifty copies of 9,000 ticks report `0/50` for
/// the whole of the first minute while 200,000 ticks have already been
/// simulated, so a runs-only readout is indistinguishable from a batch that
/// has not started -- the owner asked for the tick figure for exactly that
/// reason. The run count stays beside it because that is the number saying
/// how many rows can already be compared.
fn batch_progress_line(p: &super::batch::Progress, left_note: &str) -> String {
    let pct = (p.ticks * 100).checked_div(p.ticks_planned).unwrap_or(0);
    format!(
        "{}% -- {}/{} TICKS  {}/{} DONE  {}M{:02}S  {}  {} HELD",
        pct,
        p.ticks,
        p.ticks_planned,
        p.finished + p.failed,
        p.total,
        p.elapsed.as_secs() / 60,
        p.elapsed.as_secs() % 60,
        left_note,
        p.held
    )
}

/// **One setting's runs, reduced to the order statistic.**
///
/// A sweep of `k` settings by `r` replicates is `k * r` rows, and the
/// comparison it exists to make is between *settings*, not between runs. But
/// the median alone is not the answer: outcomes here are chaotic in the seed
/// — twelve copies of one chamber differing in nothing at all spread 2.42x on
/// plants and 3.12x on animals — so a difference of two medians inside that
/// is not a result. Both are carried, and the page draws them on two lines,
/// because `CLAUDE.md`'s rule for exactly this table is **show the spread,
/// never a mean**.
struct RackGroup {
    setting: f32,
    runs: usize,
    /// One `Spread` per numeric column, in `RACK_COLS` order from `FRAME`.
    /// `None` for a column no run in the group has measured yet.
    cols: [Option<super::stats::Spread>; 5],
}

/// Group finished rows by their swept setting. Rows with no setting, and rows
/// still running or never censused, are not data points and are left out.
fn rack_groups(chambers: &[super::ChamberSummary]) -> Vec<RackGroup> {
    let mut keys: Vec<f32> = Vec::new();
    for ch in chambers {
        if let (Some(v), Some(_)) = (ch.setting, ch.census.as_ref()) {
            if !keys.iter().any(|k| (k - v).abs() < f32::EPSILON) {
                keys.push(v);
            }
        }
    }
    keys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    keys.into_iter()
        .map(|v| {
            let mine: Vec<&super::ChamberSummary> = chambers
                .iter()
                .filter(|c| c.setting.is_some_and(|s| (s - v).abs() < f32::EPSILON) && c.census.is_some())
                .collect();
            let pull = |f: &dyn Fn(&super::stats::Census) -> f32| -> Option<super::stats::Spread> {
                let mut vals: Vec<f32> =
                    mine.iter().filter_map(|c| c.census.as_ref()).map(f).collect();
                super::stats::Spread::of(&mut vals)
            };
            RackGroup {
                setting: v,
                runs: mine.len(),
                cols: [
                    pull(&|c| c.frame as f32),
                    pull(&|c| c.plants as f32),
                    pull(&|c| c.animals as f32),
                    pull(&|c| c.plant_generation as f32),
                    pull(&|c| c.seeds_borne as f32),
                ],
            }
        })
        .collect()
}

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
const RACK_LITERALS: [&str; 8] = [
    "CLICK A ROW FOR ITS PICTURE",
    "-- THE BOX YOU ARE IN",
    "-- KEPT AS NUMBERS ONLY, THE WORLD WAS NOT HELD",
    "HERE",
    "RECORD",
    "REBUILDING",
    "RUNNING",
    "NO SWEEP TO GROUP -- EVERY ROW IS ITS OWN SETTING",
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
/// **Rows of the log this page draws.** The roster's own page height, for the
/// same reason: it is what fits above the bar with a header and a footer.
const LOG_ROWS: usize = 14;

/// The cause index a `Died` line carries, as words.
///
/// `LogEvent::other` is a `u16` because a birth puts a parent handle there;
/// on a death it is `DeathCause::index()`. An index this does not recognise
/// prints as `DIED` rather than as a wrong cause -- a log that confidently
/// names the wrong killer is worse than one that admits it does not know.
fn cause_of(index: u16) -> &'static str {
    match crate::sim::organism::DEATH_CAUSE_LIST.get(index as usize) {
        Some(c) => c.label(),
        None => "DIED",
    }
}

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
        // **How many rows fit is computed, not assumed.** `RACK_ROWS` is the
        // ceiling; the floor is whatever is left after the picture, the
        // verbs, the pager and the batch controls have taken their share. A
        // fixed row count made the panel 327 px tall on a 320 px screen once
        // a picture and a running batch were both up -- the panel clamped at
        // `MARGIN` and everything past the fold, ENTER included, fell off the
        // bottom behind the bar. The table is the index and the picture is
        // the detail, so the table is what gives way.
        let picture_h = if thumb.is_some() { RACK_THUMB_H + 4 } else { 0 };
        let batch_h = 16 + if thumb_batch_running { 11 } else { 0 };
        let verbs_h = if self.rack_selected.is_some() { 14 } else { 0 };
        // The pager depends on `shown` and `shown` on the pager, so budget
        // for it whenever the rack could overflow one screen and drop it
        // below.
        let fixed = PAGE_HEADER + RACK_HEAD + verbs_h + picture_h + batch_h + PARAM_TABS + PAGE_PAD + 13;
        let room = ((bar_top() - 4 - MARGIN - fixed) / RACK_ROW).max(1) as usize;
        let shown = chambers.len().min(RACK_ROWS).min(room);
        self.rack_scroll = self.rack_scroll.min(chambers.len().saturating_sub(shown));

        let w = rack_page_width();
        // **Every row this page can draw is counted here, or it is drawn
        // outside its own panel.** The pager and the verbs were each added
        // without a term, and the verbs went under the picture, so selecting
        // a row pushed ENTER out of the box and behind the bar -- present to
        // `widget_rect`, invisible to a player, which is how the page shipped
        // with a working ENTER nobody could press.
        let pager_h = if chambers.len() > shown { 13 } else { 0 };
        let h = PAGE_HEADER
            + RACK_HEAD
            + RACK_ROW * shown.max(1) as i32
            + pager_h
            + verbs_h
            + picture_h
            + batch_h
            + PARAM_TABS
            + PAGE_PAD;
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
        // GROUP, beside CLEAR. A hundred rows of a sweep are `k` settings
        // wearing `r` labels each, and the comparison the sweep exists for is
        // between the settings.
        let group_w = cell_width(hud::text_width("GROUP"), "", PAD);
        widgets.push(Widget {
            rect: Rect { x: right - new_w - clear_w - group_w - 8, y: rect.y + 3, w: group_w, h: 11 },
            line1: "GROUP".into(),
            line2: String::new(),
            action: Some(Action::RackGroup),
            latched: self.rack_grouped,
            icon: None,
            ratio: None,
            note: "COLLAPSE THE RACK TO ONE ROW PER SWEPT SETTING: THE MEDIAN ON TOP AND THE LOW-TO-HIGH RANGE UNDER IT. READ THE RANGE FIRST -- TWELVE COPIES OF ONE BOX DIFFERING IN NOTHING AT ALL ALREADY SPREAD 2.4X ON PLANTS AND 3.1X ON ANIMALS, SO TWO SETTINGS CLOSER TOGETHER THAN THAT HAVE NOT BEEN TOLD APART.".into(),
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
                    // Seed and setting are known before a run has any
                    // census, so they sort in flight; the rest cannot, and a
                    // row with nothing measured yet sorts below every row
                    // that has something.
                    match c {
                        0 => return r.seed as i64,
                        1 => return r.setting.map_or(i64::MIN, |v| v as i64),
                        2 => return r.frame as i64,
                        _ => {}
                    }
                    let Some(n) = r.census.as_ref() else { return -1 };
                    match c {
                        3 => n.plants as i64,
                        4 => n.animals as i64,
                        5 => n.plant_generation as i64,
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

        // ---- grouped: one setting per pair of lines, medians over ranges.
        let groups = if self.rack_grouped { rack_groups(chambers) } else { Vec::new() };
        if self.rack_grouped {
            if groups.is_empty() {
                text(frame, left, y + 2, RACK_LITERALS[7], FAINT);
                y += RACK_ROW;
            }
            for g in groups.iter().skip(self.rack_scroll).take(RACK_ROWS / 2) {
                text(frame, left, y + 2, &format!("{:.0}", g.setting), VALUE);
                text(frame, left + col[0], y + 2, &format!("{} RUNS", g.runs), FAINT);
                text(frame, left + col[1], y + 2, &format!("{:.0}", g.setting), VALUE);
                for (i, sp) in g.cols.iter().enumerate() {
                    let x = left + col[2 + i];
                    match sp {
                        Some(sp) => text(frame, x, y + 2, &format!("{:.0}", sp.mid), if i == 1 { GOOD } else { FAINT }),
                        None => text(frame, x, y + 2, "-", FAINT),
                    }
                }
                y += RACK_ROW;
                // **The range, under its own median.** A median with no
                // spread beside it invites exactly the comparison this
                // engine cannot support: two settings differing by less than
                // the noise between two copies of one chamber.
                for (i, sp) in g.cols.iter().enumerate() {
                    if let Some(sp) = sp {
                        text(frame, left + col[2 + i], y + 2, &format!("{:.0}-{:.0}", sp.low, sp.high), FAINT);
                    }
                }
                y += RACK_ROW;
            }
        }

        // ---- the rows.
        for ch in order.iter().skip(self.rack_scroll).take(shown) {
            if self.rack_grouped {
                break;
            }
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
            match ch.setting {
                Some(v) => text(frame, left + col[1], y + 2, &format!("{v:.0}"), VALUE),
                None => text(frame, left + col[1], y + 2, "-", FAINT),
            }
            // The counter that says whether it is frozen -- and, on a copy
            // still in flight, the one that says it is moving at all.
            text(frame, left + col[2], y + 2, &format!("{}", ch.frame), if ch.active || ch.running.is_some() { SUB_ON } else { FAINT });

            match &ch.census {
                Some(c) => {
                    text(frame, left + col[3], y + 2, &format!("{}", c.plants), GOOD);
                    text(frame, left + col[4], y + 2, &format!("{}", c.animals), FAIR);
                    text(frame, left + col[5], y + 2, &format!("{}/{}", c.plant_generation, c.animal_generation), FAINT);
                    text(frame, left + col[6], y + 2, &format!("{}", c.seeds_borne), FAINT);
                }
                // Never looked at, which is not the same as empty. One dash
                // per column, on the column, rather than one run of text
                // guessed into position.
                None => {
                    for x in col.iter().skip(3) {
                        text(frame, left + x, y + 2, "-", FAINT);
                    }
                }
            }
            if ch.running.is_some() {
                text(frame, right - hud::text_width(RACK_LITERALS[6]), y + 2, RACK_LITERALS[6], VALUE);
            } else if ch.active {
                text(frame, right - hud::text_width("HERE"), y + 2, RACK_LITERALS[3], SUB_ON);
            } else if ch.rebuilding {
                text(frame, right - hud::text_width(RACK_LITERALS[5]), y + 2, RACK_LITERALS[5], SUB_ON);
            } else if ch.on_record {
                text(frame, right - hud::text_width(RACK_LITERALS[4]), y + 2, RACK_LITERALS[4], FAINT);
            }
            y += RACK_ROW;
        }

        // ---- the pager, only when there is something off the page.
        //
        // Same idiom as the parameters page, deliberately: a rack of a
        // hundred and a page of four hundred knobs are the same problem, and
        // two different scroll gestures in one interface is one to learn
        // twice. The count is spelled out rather than implied by the bar,
        // because "13-24 OF 100" is the sentence that tells you there are
        // eighty-eight rows you have not looked at.
        if chambers.len() > shown {
            let step_w = cell_width(hud::text_width("<"), "", PAD);
            let up = Rect { x: left, y: y + 2, w: step_w, h: 10 };
            let down = Rect { x: left + step_w + 2, y: y + 2, w: step_w, h: 10 };
            let last = (self.rack_scroll + shown).min(chambers.len());
            text(
                frame,
                left + step_w * 2 + 8,
                y + 3,
                &format!("{}-{} OF {}", self.rack_scroll + 1, last, chambers.len()),
                FAINT,
            );
            for (r, label, dir) in [(up, "<", -1), (down, ">", 1)] {
                widgets.push(Widget {
                    rect: r,
                    line1: label.into(),
                    line2: String::new(),
                    action: Some(Action::RackScroll(dir)),
                    latched: false,
                    icon: None,
                    ratio: None,
                    note: "SCROLL THE RACK. SORTING A COLUMN JUMPS BACK TO THE TOP, SO THE FASTEST WAY TO THE RUN YOU WANT IS USUALLY TO SORT ON IT RATHER THAN TO PAGE THROUGH.".into(),
                });
            }
            y += 13;
        }

        // ---- the verbs on the highlighted row: FIRST, under the table.
        //
        // **They used to be last, and that made them unreachable.** Drawn
        // after the picture and the batch dials, a selected row pushed ENTER
        // clean out of the panel -- so the page had a working ENTER button
        // that no player could see, and the owner reported there was `no way
        // to enter the others`. The picture is the thing worth clipping when
        // the page runs long; the only verb that walks into a chamber is not.
        if let Some(i) = self.rack_selected {
            let vy = y + 3;
            let mut vx = left;
            let here = chambers.get(i).is_some_and(|c| c.active);
            // An on-record row's world was dropped for the memory budget, so
            // there is nothing to walk into. Its verbs are drawn dead rather
            // than hidden, and the row itself says which it is.
            let on_record = chambers.get(i).is_some_and(|c| c.on_record);
            let rebuilding = chambers.get(i).is_some_and(|c| c.rebuilding);
            let batch_of = chambers.get(i).and_then(|c| c.batch);
            for (label, action, on, why) in [
                ("ENTER", Action::Chamber(i), !here && !on_record,
                 "PUT THIS CHAMBER ON SCREEN. THE ONE YOU LEAVE IS HELD EXACTLY WHERE IT IS -- IT RESUMES ON THE TICK IT STOPPED AT, NOT FROM THE START."),
                ("CLOSE", Action::ChamberClose(i), !here && !on_record,
                 "THROW THIS CHAMBER AWAY. THE BOX YOU ARE IN CANNOT BE CLOSED: STEP INTO ANOTHER ONE FIRST, SO THAT CLOSING NEVER ALSO MOVES YOU SOMEWHERE YOU DID NOT ASK TO GO."),
                // Only ever on an on-record row: a chamber that still has its
                // world has nothing to rebuild.
                // Only on a row a batch produced: a chamber you made
                // yourself has no batch to close.
                // **The amount comes from the TICKS dial**, which is already
                // on this page and already types. A second number for "how
                // much more" would be a second thing to set and a second
                // place for the two to disagree.
                ("MORE", Action::ChamberExtend(i), !here,
                 "RUN THIS ROW ON BY THE TICKS SHOWN BELOW, IN THE BACKGROUND. IT CARRIES ON FROM WHERE IT STOPPED RATHER THAN STARTING AGAIN, SO EXTENDING A 9,000-TICK COPY BY 20,000 COSTS TWENTY THOUSAND AND NOT TWENTY-NINE. A ROW KEPT AS NUMBERS ONLY IS REBUILT FROM ITS RECIPE AND RUN THE WHOLE WAY, WHICH REACHES THE SAME PLACE AND TAKES LONGER."),
                ("MORE B", Action::ChamberExtendBatch(batch_of.unwrap_or(0)), batch_of.is_some(),
                 "RUN EVERY ROW OF THIS BATCH ON BY THE TICKS SHOWN BELOW. THE BOX YOU ARE IN IS LEFT ALONE -- IT IS LIVE, AND THE SPEED DIAL ALREADY RUNS IT."),
                ("CLOSE B", Action::ChamberCloseBatch(batch_of.unwrap_or(0)), batch_of.is_some(),
                 "THROW AWAY EVERY ROW THIS BATCH PRODUCED, CHAMBERS AND RECORDS ALIKE. THE BOX YOU ARE IN IS KEPT EVEN IF IT CAME FROM THIS BATCH, SO CLOSING ONE NEVER ALSO MOVES YOU SOMEWHERE YOU DID NOT ASK TO GO."),
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
        // **Advance past the verbs.** They used to be the last thing drawn
        // and so never had to; moved above the picture, a block that does not
        // move `y` gets the picture painted straight over it -- which it was,
        // across the REBUILD label, on the first render after the move.
        y += 14;

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
        let typing = self.typing.clone();
        let mut dial = |label: &str, value: String, typed: TypedField, minus: Action, plus: Action, note: &'static str, x: i32, w: &mut Vec<Widget>| -> i32 {
            let _ = &typing;
            let step = cell_width(hud::text_width("W"), "", PAD);
            text(frame, x, by + 2, label, FAINT);
            let mut cx = x + hud::text_width(label) + 4;
            for (face, action) in [("-", minus), ("+", plus)] {
                if face == "+" {
                    // The value sits between the two faces, in a **fixed**
                    // column: a value that gains a digit would otherwise
                    // shove its own `+` sideways under the cursor, which is
                    // the failure the parameters page already sizes against.
                    //
                    // It is also the click target for typing one in. Two
                    // hundred clicks to reach either dial's ceiling is not a
                    // control, so the number itself is a button.
                    let live = self.typing.as_ref().filter(|(f, _)| *f == typed);
                    match live {
                        Some((_, buf)) => text(frame, cx + 3, by + 2, &format!("{buf}_"), SUB_ON),
                        None => text(frame, cx + 3, by + 2, &value, VALUE),
                    }
                    w.push(Widget {
                        rect: Rect { x: cx + 1, y: by, w: BATCH_VALUE_W, h: 11 },
                        line1: String::new(),
                        line2: String::new(),
                        action: Some(Action::BatchType(typed)),
                        latched: live.is_some(),
                        icon: None,
                        ratio: None,
                        note: "CLICK THE NUMBER AND TYPE ONE IN. ENTER COMMITS IT, ESCAPE PUTS THE OLD ONE BACK. THE FACES EITHER SIDE STEP IT, WHICH IS FINE FOR A NUDGE AND TWO HUNDRED CLICKS FOR A LONG RUN.".into(),
                    });
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
        bx = dial("COPIES", format!("{}", state.batch.copies), TypedField::Copies, Action::BatchCopies(-1), Action::BatchCopies(1),
            "HOW MANY COPIES OF THIS BOX TO RUN. EACH ONE GETS ITS OWN SEED, WHICH IS WHAT MAKES THEM DIFFERENT WORLDS RATHER THAN THE SAME WORLD N TIMES -- MEASURED, THE SEED ALONE MOVES THE FINAL CENSUS BY 2.4 TO 3.1 TIMES.", bx, &mut widgets);
        // The return is the next free x, unused after the last dial — kept as
        // a return rather than dropped so a third dial slots in beside these
        // two without re-deriving the arithmetic.
        let _ = dial("TICKS", format!("{}", state.batch.frames), TypedField::Frames, Action::BatchFrames(-1), Action::BatchFrames(1),
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

        if let Some(p) = &state.batch.progress {
            // **The counter beside the work.** A rack of rows filling in
            // looks the same whether four copies are running or none are, so
            // the count and the clock are on screen rather than inferred.
            let left_note = match p.remaining() {
                Some(d) => format!("{}M{:02}S LEFT", d.as_secs() / 60, d.as_secs() % 60),
                None => "ESTIMATING".to_string(),
            };
            // **Ticks, not just runs.** Fifty copies of 9,000 ticks report
            // `0/50` for the whole of the first minute while 200,000 ticks
            // have actually been simulated, so runs-done reads as a stalled
            // batch for as long as the first copy takes. The percentage is
            // over ticks; the run count stays beside it because that is what
            // says how many rows you can already compare.
            let line = batch_progress_line(p, &left_note);
            text(frame, left, y, &line, SUB_ON);
            if p.failed > 0 {
                text(frame, left, y + 9, &format!("{} FAILED TO BUILD", p.failed), POOR);
            }
            // No `y` advance: the batch line is the last thing this page
            // draws now that the row verbs have moved above the picture.
            // Adding one back means something follows it -- and that
            // something needs a term in `h` above, or it lands outside the
            // panel exactly as ENTER did.
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

    /// **The roster page: one row per living thing, and a click pins one.**
    ///
    /// Draws itself for `paint_params`' reason -- its rows carry verbs, so
    /// they are not `Row`s -- and follows `paint_rack`'s shape throughout,
    /// deliberately: a rack of a hundred chambers and a bed of a hundred
    /// plants are the same problem, and two different table gestures in one
    /// interface is one to learn twice.
    ///
    /// **The row list is built first and the panel sized from it**, never
    /// painted with a running cursor. `Reports/dead-ends.md` rejected the
    /// running-cursor panel once its contents became variable, and the
    /// condition it recorded was *the row list being variable* -- which a
    /// roster is more than anything else on screen: it changes length between
    /// two frames with nobody touching anything.
    fn paint_roster(&mut self, frame: &mut [u8], world: &World, kingdom: roster::Kingdom) -> Option<(String, Rect, i32)> {
        let mut widgets: Vec<Widget> = Vec::new();
        let mut note: Option<(String, Rect, i32)> = None;

        let cols = roster_cols(kingdom);
        let view = self.view(kingdom);
        let (sort_key, desc) = self.roster_sort_key(kingdom);
        let rows = roster::rows(world, kingdom, sort_key, desc, view.filter);
        let total = rows.len();

        // How many rows fit is computed rather than assumed, for the reason
        // `paint_rack` records: a fixed count made that page 327 px tall on a
        // 320 px screen and everything past the fold fell behind the bar.
        let verbs_h = if self.pinned.is_some() { 14 } else { 0 };
        let fixed = PAGE_HEADER + RACK_HEAD + verbs_h + PAGE_PAD + 13 + 13;
        let room = ((bar_top() - 4 - MARGIN - fixed) / RACK_ROW).max(1) as usize;
        let shown = total.min(ROSTER_ROWS).min(room);
        self.view_mut(kingdom).scroll = view.scroll.min(total.saturating_sub(shown));
        let scroll = self.view(kingdom).scroll;

        let pager_h = if total > shown { 13 } else { 0 };
        let h = PAGE_HEADER + RACK_HEAD + RACK_ROW * shown.max(1) as i32 + pager_h + verbs_h + 13 + PAGE_PAD;
        let w = roster_page_width(kingdom);
        let bottom = bar_top() - 4;
        let rect = Rect { x: MARGIN, y: (bottom - h).max(MARGIN), w, h };
        self.roster_box = Some(rect);

        fill(frame, rect, PANEL_BG);
        outline(frame, rect, PANEL_EDGE);
        let left = rect.x + PAGE_PAD;
        let right = rect.right() - PAGE_PAD;
        let title = if kingdom == roster::Kingdom::Plants { "EVERY PLANT" } else { "EVERY ANIMAL" };
        text(frame, left, rect.y + 6, title, TITLE);

        // BACK, in the header, to the page this one was opened from.
        //
        // **Not a nicety: without it the roster is a dead end.** Every other
        // page here is closed by pressing the bar chip that opened it, and
        // the roster has no chip -- that is the whole reason it hangs off the
        // PLANTS page. So the only way out was the bar's own PLANTS button,
        // which is one row up and reads as "open the page I am already
        // inside". Found by the harness, which got stuck here on its second
        // pass: `widget_rect` could not aim a click at a page it could not
        // leave, which is exactly the player's problem in miniature.
        let cover = if kingdom == roster::Kingdom::Plants { Panel::Plants } else { Panel::Ants };
        // **The header buttons stop short of where the cell page can reach.**
        // The two pages are open together by design -- pinning a row is how
        // the cell page gets pointed at an individual -- and the cell page is
        // painted after this one, sized to its own widest row and slid left
        // when it will not fit beside this one. A wide one reaches into this
        // page's header and simply paints over whatever is there. It has been
        // eating the right-hand end of BACK for as long as both pages have
        // existed; adding a destructive verb to this row is what made it
        // worth fixing, because a CULL REST hidden under another panel is a
        // different order of problem from a clipped `K`. The *table* keeps
        // its full width -- its last column is STATE and its values are
        // short -- so only the buttons move.
        //
        // **The bound is the widest the cell page can ever be, not the width
        // it happens to have.** `inspect_box` is last frame's rectangle, and
        // one frame is exactly the error: opening the WORDS group is the
        // event that widens the page, so the frame that clips is the frame
        // last frame's rectangle knows nothing about. That is the version
        // that shipped and was photographed.
        let bar_right = if self.inspect.is_some() {
            right.min(W as i32 - MARGIN - widest_cell_page() - 3)
        } else {
            right
        };
        let bw = cell_width(hud::text_width("BACK"), "", PAD) + 4;
        widgets.push(Widget {
            rect: Rect { x: bar_right - bw, y: rect.y + 3, w: bw, h: 11 },
            line1: "BACK".into(),
            line2: String::new(),
            action: Some(Action::Panel(cover)),
            latched: false,
            icon: None,
            ratio: None,
            note: "BACK TO THE COUNTS THIS LIST HANGS OFF. THAT PAGE IS THE POPULATION; THIS ONE IS THE INDIVIDUALS IN IT.".into(),
        });

        // FILTER, beside it. One chip cycling three states rather than three
        // chips: the page is already at its width, and the state it is in is
        // written on the chip's own face, which a set of latches is not.
        let filter_label = view.filter.label();
        let fw = cell_width(hud::text_width(&filter_label), "", PAD) + 4;
        widgets.push(Widget {
            rect: Rect { x: bar_right - bw - fw - 4, y: rect.y + 3, w: fw, h: 11 },
            line1: filter_label,
            line2: String::new(),
            action: Some(Action::RosterFilter),
            latched: view.filter != roster::Filter::All,
            icon: None,
            ratio: None,
            note: "WHAT THE LIST IS SHOWING. IN TROUBLE KEEPS ONLY THE ONES STARVING, ROTTING OR HUNGRY, WHICH IS THE QUESTION A LIST OF A HUNDRED IS OPENED TO ASK. LINE KEEPS ONE FOUNDING LINE, AND IT NEEDS A ROW PINNED FIRST BECAUSE OTHERWISE THERE IS NO LINE TO MEAN.".into(),
        });

        // **CULL REST, on the table's own bar and not on the pinned row.**
        // It is a verb about the *whole list*, and it has to work with
        // nothing pinned -- the gesture is walk the list sparing the few you
        // want, then press this once. A verb that needed a pin would make
        // "keep these, kill the rest" require an arbitrary survivor to be
        // selected as well.
        //
        // **The count is on the face of it.** `CULL REST 48` says what the
        // press will do before it is pressed, which for the one destructive
        // verb on this page is the difference between a button and a trap.
        // It is also the only feedback available at the moment of pressing:
        // a cull is graded, so nothing vanishes and an unlabelled button
        // would read as having done nothing.
        let doomed = self.cull_rest_targets(world, kingdom).len();
        let kept = self.spared.len();
        let cull_label = format!("CULL REST {doomed}");
        let cw = cell_width(hud::text_width(&cull_label), "", PAD) + 4;
        widgets.push(Widget {
            rect: Rect { x: bar_right - bw - fw - cw - 8, y: rect.y + 3, w: cw, h: 11 },
            line1: cull_label,
            line2: String::new(),
            action: Some(Action::RosterCullRest),
            latched: kept > 0,
            icon: None,
            ratio: None,
            note: format!(
                "KILL EVERY {} IN THE BOX EXCEPT THE {kept} YOU HAVE SPARED. THE NUMBER ON THE BUTTON IS HOW MANY THAT IS RIGHT NOW. IT IGNORES THE FILTER ON PURPOSE -- CULLING ONLY WHAT A FILTER HAPPENS TO BE SHOWING WOULD MAKE ONE BUTTON MEAN DIFFERENT THINGS DEPENDING ON A CHIP PRESSED TWO CLICKS AGO. USE SPARE ON A PINNED ROW TO BUILD THE KEEP LIST.",
                if kingdom == roster::Kingdom::Plants { "PLANT" } else { "ANIMAL" }
            ),
        });

        for x in rect.x + 1..rect.right() - 1 {
            render::put(frame, W, H, x, rect.y + PAGE_HEADER - 4, DIVIDER);
        }

        // ---- the column header. Measured through `hud::text_width` against
        // the widest value each column can ever hold, never counted by hand:
        // the rack's own columns were hand-counted first and overlapped by
        // eight pixels, reported from play.
        let mut y = rect.y + PAGE_HEADER;
        let col = roster_col_x(kingdom);
        for (i, (head, widest, _)) in cols.iter().enumerate() {
            let sorted_on = view.sort.map(|(c, _)| c) == Some(i);
            let label = match view.sort {
                Some((c, d)) if c == i => format!("{head}{}", if d { "\\" } else { "/" }),
                _ => (*head).to_string(),
            };
            text(frame, left + col[i], y, &label, if sorted_on { TITLE } else { FAINT });
            // The whole column is the target: a three-character heading is
            // not something a person can reliably hit.
            widgets.push(Widget {
                rect: Rect { x: left + col[i] - 2, y: y - 2, w: hud::text_width(widest) + 4, h: RACK_HEAD },
                line1: String::new(),
                line2: String::new(),
                action: Some(Action::RosterSort(i)),
                latched: false,
                icon: None,
                ratio: None,
                note: String::new(),
            });
        }
        y += RACK_HEAD;
        for x in rect.x + 1..rect.right() - 1 {
            render::put(frame, W, H, x, y - 2, DIVIDER);
        }

        // ---- the rows.
        if total == 0 {
            // An empty state that says which empty it is. A box with no
            // plants and a filter hiding all of them look identical
            // otherwise, and the second one is a mistake the player made two
            // clicks ago.
            let why = match view.filter {
                roster::Filter::All => ROSTER_LITERALS[0],
                roster::Filter::Dead => ROSTER_LITERALS[4],
                _ => ROSTER_LITERALS[1],
            };
            text(frame, left, y + 2, why, FAINT);
            y += RACK_ROW;
        }
        for (n, r) in rows.iter().enumerate().skip(scroll).take(shown) {
            let selected = self.pinned == Some(r.who);
            let band = Rect { x: rect.x + 1, y, w: rect.w - 2, h: RACK_ROW };
            // Before the text, for `paint_rack`'s reason: a hover fill
            // painted over the row erases the line you are pointing at, which
            // looks exactly like a row with no data in it.
            let hovered = self.cursor.is_some_and(|(cx, cy)| band.contains(cx, cy));
            if selected {
                fill(frame, band, FACE_ON);
            } else if hovered {
                fill(frame, band, FACE_HOVER);
            }
            // The whole row is the button, and it is aimed with the position
            // in the *sorted* list -- which `Lab::act` resolves to an identity
            // in the same frame rather than storing.
            widgets.push(Widget {
                rect: band,
                line1: String::new(),
                line2: String::new(),
                action: Some(Action::RosterSelect(n)),
                latched: false,
                icon: None,
                ratio: None,
                note: String::new(),
            });

            // **A spared row says so in its number.** The keep list is built
            // by walking the table pressing SPARE, so the one thing it has to
            // be possible to see at a glance is which rows are already in it
            // -- otherwise building a list of six out of a hundred means
            // remembering which six. `*` rather than a colour, because the
            // number column is already tinted for selection and two meanings
            // on one channel is how a readout stops being read.
            let spared = self.is_spared(r.who);
            let tint = if selected {
                TITLE
            } else if spared {
                GOOD
            } else {
                FAINT
            };
            let label = if spared { format!("{SPARED_MARK}{}", n + 1) } else { format!("{}", n + 1) };
            text(frame, left, y + 2, &label, tint);
            let species = param_label(&world.species.get(r.species).name);
            let state_tint = match r.state {
                roster::RowState::Senescent | roster::RowState::Starving => POOR,
                roster::RowState::Hungry => FAIR,
                // `Far` is neutral, not a warning: it says an animal is deep
                // in an excursion, which is what a forager is for. See
                // `roster::RowState::Far` for why it stopped being `LOST`.
                roster::RowState::Far => VALUE,
                roster::RowState::Carrying => GOOD,
                // Dimmer than POOR, which is a warning about something still
                // alive. Nothing can be done about this row.
                roster::RowState::Dead(_) => FAINT,
                roster::RowState::Ok => FAINT,
            };
            // **A grave's age is the life it had, not the time since it was
            // born.** See `RosterRow::died_frame`.
            let age = r.died_frame.unwrap_or(world.frame).saturating_sub(r.born_frame);
            let values: [(String, [u8; 4]); 8] = if kingdom == roster::Kingdom::Plants {
                [
                    (species, VALUE),
                    (format!("{}", r.cells), GOOD),
                    (format!("{}", r.score), if r.score > 0 { GOOD } else { FAINT }),
                    (format!("{:.2}", r.energy), if r.energy < 0.5 { FAIR } else { FAINT }),
                    (compact(age as f64), FAINT),
                    (format!("{}", r.generation), FAINT),
                    (format!("{}", r.lineage), FAINT),
                    (r.state.label().to_string(), state_tint),
                ]
            } else {
                [
                    (species, VALUE),
                    (format!("{:.0}", r.energy), if r.state == roster::RowState::Hungry { FAIR } else { GOOD }),
                    (format!("{}", r.carrying), if r.carrying > 0 { GOOD } else { FAINT }),
                    (format!("{}", r.score), if r.score > 0 { GOOD } else { FAINT }),
                    (compact(age as f64), FAINT),
                    (format!("{}", r.generation), FAINT),
                    (format!("{}", r.lineage), FAINT),
                    (r.state.label().to_string(), state_tint),
                ]
            };
            for (i, (v, t)) in values.iter().enumerate() {
                text(frame, left + col[i], y + 2, v, *t);
            }
            y += RACK_ROW;
        }

        // ---- the pager, only when there is something off the page. Spelled
        // out rather than implied by a bar, because "13-24 OF 100" is the
        // sentence that says there are seventy-six you have not looked at.
        if total > shown {
            let step_w = cell_width(hud::text_width("<"), "", PAD);
            let up = Rect { x: left, y: y + 2, w: step_w, h: 10 };
            let down = Rect { x: left + step_w + 2, y: y + 2, w: step_w, h: 10 };
            let last = (scroll + shown).min(total);
            text(frame, left + step_w * 2 + 8, y + 3, &format!("{}-{} OF {}", scroll + 1, last, total), FAINT);
            for (r, label, dir) in [(up, "<", -1), (down, ">", 1)] {
                widgets.push(Widget {
                    rect: r,
                    line1: label.into(),
                    line2: String::new(),
                    action: Some(Action::RosterScroll(dir)),
                    latched: false,
                    icon: None,
                    ratio: None,
                    note: "SCROLL THE LIST. SORTING A COLUMN JUMPS BACK TO THE TOP, SO THE FASTEST WAY TO THE ONE YOU WANT IS USUALLY TO SORT ON IT RATHER THAN TO PAGE THROUGH.".into(),
                });
            }
            y += 13;
        }

        // ---- the verbs on the pinned row.
        //
        // Under the table for `paint_rack`'s reason, which it learned the
        // hard way: drawn last, after everything else the page can hold, a
        // selected row pushed its own verbs out of the panel and behind the
        // bar -- present to `widget_rect`, invisible to a player.
        let vy = y + 3;
        if let Some(who) = self.pinned {
            let mut vx = left;
            let alive = who.alive(world);
            for (label, action, on, why) in [
                ("FOLLOW", Action::RosterFollow, alive,
                 "KEEP THE CAMERA ON THIS ONE WHILE IT MOVES. AN ANT IS TWO DARK CELLS AT PLAY ZOOM AND YOU FIND IT ONLY BECAUSE IT MOVES, SO A MARKER ALONE IS HALF AN ANSWER: THIS IS THE OTHER HALF."),
                ("LINE", Action::RosterFilter, alive,
                 "SHOW ONLY THIS ONE'S FOUNDING LINE. TWO INDIVIDUALS WITH THE SAME LINE SHARE AN ANCESTOR IN THIS BOX; TWO WITH DIFFERENT ONES DO NOT, SO THIS IS HOW YOU WATCH ONE LINE TAKE THE BOX OVER."),
                (if self.is_spared(who) { "SPARED" } else { "SPARE" }, Action::RosterSpare, true,
                 "KEEP THIS ONE OUT OF A CULL-THE-REST. PRESS IT AGAIN TO STOP SPARING IT. THE SET SURVIVES CHANGING THE PIN, SO THE WAY TO CLEAR A BOX DOWN TO THE FEW YOU WANT IS TO WALK THE LIST SPARING THEM AND THEN PRESS CULL REST ONCE."),
                ("CULL", Action::RosterCull, alive,
                 "KILL THIS ONE. IT IS THE SAME GRADED DEATH THE BRUSH DEALS -- IT KEEPS ITS CELLS UNTIL THEY ROT AND ITS ROW SAYS ROTTING WHILE THEY DO, SO NOTHING VANISHES OFF THE SCREEN THE INSTANT YOU PRESS IT."),
                (if self.held.is_some() { "VS" } else { "HOLD" }, Action::RosterCompare, true,
                 "HOLD THIS ONE, THEN PIN ANOTHER AND PRESS IT AGAIN TO PUT THE TWO SIDE BY SIDE. THE QUESTION A SELECTION EXPERIMENT IS ACTUALLY ASKING IS WHY ONE DID BETTER THAN ANOTHER, AND READING THAT OFF TWO CELL PAGES MEANS COMPARING THIRTY NUMBERS FROM MEMORY."),
                ("RELEASE", Action::RosterRelease, true,
                 "LET GO OF THIS ONE. THE MARKER COMES OFF THE BOX, THE PAGE STOPS FOLLOWING IT, AND ANY HELD COMPARISON IS FORGOTTEN."),
            ] {
                let bw = cell_width(hud::text_width(label), "", PAD) + 6;
                if on {
                    widgets.push(Widget {
                        rect: Rect { x: vx, y: vy, w: bw, h: 11 },
                        line1: label.into(),
                        line2: String::new(),
                        action: Some(action),
                        latched: label == "FOLLOW" && self.following,
                        icon: None,
                        ratio: None,
                        note: why.into(),
                    });
                } else {
                    // Drawn dead rather than hidden, for the rack's reason: a
                    // verb that vanishes when it does not apply teaches
                    // nothing, and one that is visibly unavailable says why
                    // when you hover it.
                    text(frame, vx + PAD, vy + 2, label, SUB);
                }
                vx += bw + 4;
            }
            // **The pin's own death, said out loud.** The page would
            // otherwise simply stop having numbers on it, which reads as the
            // interface having broken rather than as the animal having died.
            if !alive {
                text(frame, vx + 4, vy + 2, ROSTER_LITERALS[2], POOR);
            }
        } else {
            text(frame, left, vy + 2, ROSTER_LITERALS[3], FAINT);
        }

        // Retained first, then painted from what was retained -- the house
        // idiom, so an invisible hit target cannot be drawn as a blank chip.
        for wid in widgets.iter().filter(|w| !w.line1.is_empty()) {
            let hover = self.cursor.is_some_and(|(x, y)| wid.rect.contains(x, y));
            let down = hover && self.pressed.is_some() && self.pressed == wid.action;
            paint_widget(frame, wid, hover, down);
        }
        self.roster_bar = Bar { widgets, dividers: Vec::new() };
        if let Some(wid) = self.roster_bar.hovered(self.cursor) {
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
                "PUT THE ARMED JAR BACK IN THE BOX. THIS CLOSES THE RACK AND ARMS THE PLACING -- THE NEXT CLICK IN THE BOX IS WHERE IT GOES. HOW MANY GO IN IS THE STOCK DIAL ON THE BAR: AT 1 IT IS ONE, ABOVE 1 IT IS A COLONY WITH A NEST, LAID OUT THE WAY A FOUNDED ONE IS. HOW FAR EACH HAS DRIFTED IS THE DIAL ON THE LEFT -- AT 0 BROODS EVERY ONE IS THAT EXACT INDIVIDUAL, SO A COLONY IS A COLONY OF CLONES; ABOVE 0 EACH DRIFTS SEPARATELY, SO IT IS A COLONY OF SIBLINGS AND THE SPREAD IS REAL. A PLANT ARRIVES AS A SEED THAT STILL HAS TO GERMINATE; AN ANIMAL ARRIVES ALIVE.",
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
pub(crate) fn wrap_words(text: &str, columns: usize) -> Vec<String> {
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
        // Before anything is painted, so the reticle, the page and the rows
        // under it are all reading one position rather than three.
        self.follow_inspected(world);

        // The inspected cell, marked in the world. The verb has to leave a
        // mark: a panel that names a cell without showing which cell it is
        // makes the player find it again by counting pixels.
        // **The whole body of the pinned individual, before its cell marker.**
        //
        // A single-cell reticle is the right answer for a cell you clicked and
        // the wrong one for an individual you picked off a list: it says
        // *here* when the question was *which one*, and on a tree it marks one
        // cell of two thousand. The box is drawn under the reticle so the two
        // read as one mark rather than as two things.
        //
        // **A full replace on a fixed colour, never a blend into the cells
        // underneath.** A magnitude-scaled blend was tried elsewhere in this
        // engine and produced a sheet that read as blank -- the ramp was red,
        // the wood was brown, and a mid-range value moved one colour byte from
        // 139 to 155. The obvious reading, "the mechanism is dead", would have
        // sent a fix at working code.
        //
        // And it is **static**, which is the whole point: the design guide's
        // own measurement is that an ant is findable only because it moves,
        // and a dead one has stopped. A marker that needed motion would fail
        // on exactly the individual you most want to find.
        // **Where it has been**, under the marker that says where it is.
        //
        // Drawn first so the marker sits on top: the trail is context and the
        // mark is the answer, and a bright dot buried under a path reads as
        // one more sample. Oldest is dimmest, so the direction of travel is
        // legible without an arrowhead -- an arrow at this scale is three
        // pixels and reads as noise.
        //
        // **Nothing here needs the individual to still exist.** The ring is
        // the record; a starved ant's last excursion is exactly the thing its
        // graveyard row cannot tell you, and it stays on screen while it is
        // pinned. That is the whole reason the trail is kept on the ring
        // rather than recomputed from the world every frame.
        if self.pinned.is_some() {
            let n = self.watch.len();
            for (i, (wx, wy)) in self.watch.path().enumerate() {
                let (x0, y0, x1, y1, _) = renderer.world_rect_to_screen(wx, wy, wx, wy);
                let (cx, cy) = ((x0 + x1) / 2, (y0 + y1) / 2);
                if cy >= bar_top() {
                    continue;
                }
                // Fades from a third of full to full across the ring. Not to
                // zero: the oldest sample is where the excursion *started*,
                // which is half of what makes a path a path.
                let t = if n > 1 { i as f32 / (n - 1) as f32 } else { 1.0 };
                let k = 0.33 + 0.67 * t;
                let tint = [
                    (MARKER[0] as f32 * k) as u8,
                    (MARKER[1] as f32 * k) as u8,
                    (MARKER[2] as f32 * k) as u8,
                    255,
                ];
                render::put(frame, W, H, cx, cy, tint);
            }
        }

        if let Some(who) = self.pinned {
            if let Some(state) = who.resolve(world) {
                let mut b = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
                for &(x, y) in state.cells.keys() {
                    b.0 = b.0.min(x);
                    b.1 = b.1.min(y);
                    b.2 = b.2.max(x);
                    b.3 = b.3.max(y);
                }
                // A body of two cells is already covered by the reticle, and a
                // box drawn round it is the reticle again one pixel out. The
                // bar is four cells, which is where a box starts saying
                // something the reticle does not.
                let big = b.0 <= b.2 && (b.2 - b.0 >= 3 || b.3 - b.1 >= 3);
                if big {
                    let (x0, y0, x1, y1, _) = renderer.world_rect_to_screen(b.0, b.1, b.2, b.3);
                    let (x0, y0) = (x0 - 2, y0 - 2);
                    let (x1, y1) = (x1 + 2, y1 + 2);
                    if y0 < bar_top() {
                        // Corner ticks rather than a closed rectangle. A box
                        // round a tree is a box round most of the screen, and
                        // a closed one reads as a panel border; corners read
                        // as a bracket and leave the plant visible inside it.
                        let box_rect = Rect { x: x0, y: y0, w: x1 - x0 + 1, h: (y1.min(bar_top() - 1)) - y0 + 1 };
                        let arm = 4;
                        for i in 0..arm {
                            for (cx, cy) in [
                                (box_rect.x + i, box_rect.y),
                                (box_rect.x, box_rect.y + i),
                                (box_rect.right() - 1 - i, box_rect.y),
                                (box_rect.right() - 1, box_rect.y + i),
                                (box_rect.x + i, box_rect.bottom() - 1),
                                (box_rect.x, box_rect.bottom() - 1 - i),
                                (box_rect.right() - 1 - i, box_rect.bottom() - 1),
                                (box_rect.right() - 1, box_rect.bottom() - 1 - i),
                            ] {
                                if cy < bar_top() {
                                    render::put(frame, W, H, cx, cy, MARKER);
                                }
                            }
                        }
                    }
                }
            }
        }

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
            self.panel_bar = Bar::default();
            self.shelf_box = None;
            self.shelf_bar = Bar::default();
            self.roster_box = None;
            self.roster_bar = Bar::default();
        } else if self.panel == Some(Panel::Chambers) {
            // Its own painter, for `Params`' and `Shelf`'s reason: a row here
            // is a chamber with two verbs attached, not a label.
            if let Some((body, avoid, y)) = self.paint_rack(frame, state.chambers, state.rack_thumb, state) {
                note = Some((body, avoid, y, Note::BesidePage));
            }
            self.panel_box = None;
            self.panel_bar = Bar::default();
            self.params_box = None;
            self.params_bar = Bar::default();
            self.shelf_box = None;
            self.shelf_bar = Bar::default();
            self.roster_box = None;
            self.roster_bar = Bar::default();
        } else if self.panel == Some(Panel::Shelf) {
            // The same deal, and the same reason: a jar row is a verb.
            if let Some((body, avoid, y)) = self.paint_shelf(frame) {
                note = Some((body, avoid, y, Note::BesidePage));
            }
            self.panel_box = None;
            self.panel_bar = Bar::default();
            self.params_box = None;
            self.params_bar = Bar::default();
            self.rack_box = None;
            self.rack_bar = Bar::default();
            self.roster_box = None;
            self.roster_bar = Bar::default();
        } else if matches!(self.panel, Some(Panel::PlantList) | Some(Panel::AntList)) {
            // Its own painter, for the same reason as the three above: a row
            // here is an individual with three verbs attached, not a label.
            let kingdom = if self.panel == Some(Panel::PlantList) {
                roster::Kingdom::Plants
            } else {
                roster::Kingdom::Creatures
            };
            if let Some((body, avoid, y)) = self.paint_roster(frame, world, kingdom) {
                note = Some((body, avoid, y, Note::BesidePage));
            }
            self.panel_box = None;
            self.panel_bar = Bar::default();
            self.params_box = None;
            self.params_bar = Bar::default();
            self.shelf_box = None;
            self.shelf_bar = Bar::default();
            self.rack_box = None;
            self.rack_bar = Bar::default();
        } else if let Some(panel) = self.panel {
            self.params_box = None;
            self.params_bar = Bar::default();
            self.shelf_box = None;
            self.shelf_bar = Bar::default();
            self.rack_box = None;
            self.rack_bar = Bar::default();
            self.roster_box = None;
            self.roster_bar = Bar::default();
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
            let mut taps: Vec<Widget> = Vec::new();
            if let Some((text, y)) = paint_page(frame, rect, panel.title(), &rows, self.cursor, &mut taps) {
                note = Some((text, rect, y, Note::BesidePage));
            }
            self.panel_bar = Bar { widgets: taps, dividers: Vec::new() };
        } else {
            self.panel_box = None;
            self.panel_bar = Bar::default();
            self.params_box = None;
            self.params_bar = Bar::default();
            self.shelf_box = None;
            self.shelf_bar = Bar::default();
            self.roster_box = None;
            self.roster_bar = Bar::default();
        }

        // **Not beside `Panel::Compare`, which is the one page that already
        // holds it.** Every other page answers a different question from the
        // cell page, so the two sit side by side. Compare's left column *is*
        // the pinned individual's cell page, row for row, and a pin forces
        // `inspect` open -- so leaving both up draws the same numbers twice,
        // takes the width that made the comparison legible, and puts the
        // duplicate on the side the reader is scanning toward.
        if let Some(at) = self.inspect.filter(|_| self.panel != Some(Panel::Compare)) {
            let rows = self.inspect_rows(world, at);
            // Beside the open page rather than under it, so opening a page
            // does not hide the cell you are inspecting.
            let anchor = self.panel_box.or(self.params_box).or(self.shelf_box).or(self.roster_box).map_or(MARGIN, |r| r.right() + 6);
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
            paint_hover_cell(frame, world, (wx, wy), self.inspect_box);
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
///
/// **It docks clear of whatever is already parked there.** `avoid` is the
/// cell page's rectangle, when one is open. Both want the
/// top-left -- the readout at a fixed `HOVER_TOP`, the page anchored at
/// `MARGIN` when no panel is open -- and until the roster landed they were
/// rarely both up, because opening the cell page took a deliberate click on
/// the world. A pin opens it as a matter of course, so the collision went
/// from occasional to constant: the sheet showed `EMPTY 172.85 / 20C / DRY /
/// NO ORGANISM` painted over the page's own AT and MATERIAL rows, which reads
/// as one panel contradicting itself.
///
/// **The transient one moves.** The readout follows the cursor and is gone
/// the moment it leaves; the page is pinned and is what the player is
/// reading.
fn paint_hover_cell(frame: &mut [u8], world: &World, (x, y): (i32, i32), avoid: Option<Rect>) {
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
    let h = lines.len() as i32 * LINE + 8;
    let mut r = Rect { x: MARGIN, y: HOVER_TOP, w, h };
    if let Some(page) = avoid {
        if page.contains(r.x, r.y) || page.contains(r.right(), r.bottom()) || (r.x < page.right() && r.right() > page.x && r.y < page.bottom() && r.bottom() > page.y) {
            // Beside it if there is room, under it if there is not, and back
            // where it started if neither -- a readout squeezed off the
            // screen is worse than one overlapping something.
            let beside = page.right() + 6;
            r.x = if beside + w <= W as i32 - MARGIN { beside } else { r.x };
            if r.x == MARGIN {
                let under = page.bottom() + 6;
                r.y = if under + h <= bar_top() - MARGIN { under } else { r.y };
            }
        }
    }
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
            // The widest face the dial can show, so the fit guard below is
            // measured against the worst case rather than the default.
            stock: 104,
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
                setting: None,
                batch: None,
                running: None,
                index: i,
                active: i == 0,
                label: format!("{}", i + 1),
                seed: i as u64 + 1,
                frame: 0,
                census: None,
            })
            .collect()
    }

    /// **The batch line reports ticks, not just runs.**
    ///
    /// The owner's ask, and the reason it is not "N of M done": fifty copies
    /// of 9,000 ticks sit at `0/50` for the whole of the first minute while a
    /// fifth of the work is already done, which reads as a batch that has not
    /// started. Asserted rather than photographed -- the line is drawn with
    /// `text` and the one contact sheet that should have shown it had a
    /// thumbnail over it.
    #[test]
    fn the_batch_line_leads_with_ticks() {
        let p = super::super::batch::Progress {
            total: 50,
            finished: 0,
            failed: 0,
            held: 0,
            elapsed: std::time::Duration::from_secs(65),
            cancelled: false,
            ticks: 180_000,
            ticks_planned: 450_000,
            live: Vec::new(),
        };
        let line = batch_progress_line(&p, "3M20S LEFT");

        // The case the whole readout exists for: no run has landed, and the
        // line must still show real progress.
        assert!(line.starts_with("40% -- 180000/450000 TICKS"), "ticks must lead: {line}");
        assert!(line.contains("0/50 DONE"), "and the run count stays beside them: {line}");
        assert!(line.contains("1M05S"), "elapsed is mm ss, zero-padded: {line}");

        // Every glyph of it must actually draw -- the 5x7 set has no `*`,
        // `#` or `~`, and an undrawable character renders as a silent blank.
        for c in line.chars() {
            assert!(crate::hud::has_glyph(c), "the batch line cannot draw {c:?}: {line}");
        }

        // A batch asked for zero ticks must not divide by zero.
        let empty = super::super::batch::Progress { ticks_planned: 0, ..p };
        assert!(batch_progress_line(&empty, "ESTIMATING").starts_with("0% --"));
    }

    /// **Everything the rack page draws stays on the screen.**
    ///
    /// The page grows: a pager row, a thumbnail, two dials, a RUN button and
    /// a progress line, stacked under a table whose height is fixed. Each
    /// addition has been safe on its own and nothing checked the sum, which
    /// is the bar's own failure mode one panel over -- and a control pushed
    /// off the bottom is still returned by `widget_rect`, so a harness keeps
    /// clicking it and every test stays green while no player can reach it.
    #[test]
    fn the_rack_page_stays_on_the_screen() {
        let st = state(false, 1);
        let mut buf = vec![0u8; (W * H * 4) as usize];
        // The tallest case: more rows than fit (so the pager draws), a
        // picture showing, and a batch running under it.
        let (tw, th) = (W / RACK_THUMB_SHRINK, H / RACK_THUMB_SHRINK);
        let thumb = super::super::Thumb { w: tw, h: th, frame: 0, rgba: vec![0; (tw * th * 4) as usize] };
        let mut running = state(true, 1);
        running.batch.progress = Some(super::super::batch::Progress {
            total: 50,
            finished: 3,
            failed: 0,
            held: 3,
            elapsed: std::time::Duration::from_secs(90),
            cancelled: false,
            ticks: 412_000,
            ticks_planned: 900_000,
            live: Vec::new(),
        });
        for (label, st) in [("at rest", &st), ("with a batch running", &running)] {
            let mut page = Ui::new();
            // **With a row picked**, which is the tallest the page ever gets
            // and the case the verbs live in: ENTER, CLOSE and REBUILD are
            // only drawn on a highlighted row, under the picture of it. A
            // guard that never selects one never sees them, and the owner
            // reported exactly that -- "there is currently no way to enter
            // the others" -- against a page that draws an ENTER button.
            page.select_chamber(20);
            page.paint_rack(&mut buf, &rack(40), Some(&thumb), st);
            // **Against the panel, not the screen.** The first version of
            // this guard checked `H` and passed while ENTER was being drawn
            // below the panel, behind the bar -- on the screen by arithmetic
            // and invisible in fact. That is the bug the owner reported.
            let panel = page.rack_box.expect("paint_rack records the panel it drew");
            assert!(
                panel.y >= 0 && panel.bottom() <= H as i32,
                "{label}: the rack panel itself is {} px tall on a {H} px screen ({}..{}) -- it clamps, and \
                 whatever is drawn past the fold falls off the bottom behind the bar",
                panel.h,
                panel.y,
                panel.bottom()
            );
            for w in &page.rack_bar.widgets {
                assert!(
                    w.rect.y >= panel.y && w.rect.bottom() <= panel.bottom(),
                    "{label}: a rack control is drawn outside the panel -- {:?} at y {}..{}, panel {}..{}. \
                     Off the bottom it sits behind the bar: visible to `widget_rect`, invisible to a player.",
                    w.line1,
                    w.rect.y,
                    w.rect.bottom(),
                    panel.y,
                    panel.bottom()
                );
                assert!(
                    w.rect.x >= 0 && w.rect.right() <= W as i32,
                    "{label}: a rack control runs off the screen horizontally -- {:?} ends at {} of {W}",
                    w.line1,
                    w.rect.right()
                );
            }
        }
    }

    /// **A number can be typed into a batch dial, and it lands where the
    /// faces would have landed it.**
    ///
    /// The dials step COPIES by one to 200 and TICKS by a thousand to
    /// 200,000 -- two hundred clicks to either ceiling, which is why the
    /// owner asked for typing. The clamp is the half worth guarding: it lives
    /// on `Lab::commit_typed_batch` rather than in the keyboard handler
    /// precisely so a typed 900,000 and two hundred clicks reach the same
    /// number, and two clamps for one dial is how they drift apart.
    #[test]
    fn a_batch_dial_takes_a_typed_number() {
        let mut page = Ui::new();
        assert!(page.typing().is_none(), "nothing is being typed into by default");

        page.begin_typing(TypedField::Frames);
        for c in "45000".chars() {
            page.type_digit(c);
        }
        assert_eq!(page.typing(), Some((TypedField::Frames, "45000")));
        assert_eq!(page.commit_typing(), Some((TypedField::Frames, 45_000)));
        assert!(page.typing().is_none(), "committing closes the editor");

        // Non-digits are ignored rather than accepted and then failing to
        // parse -- the buffer is what is drawn on screen, so junk in it is
        // junk a player is looking at.
        page.begin_typing(TypedField::Copies);
        for c in "1a2.".chars() {
            page.type_digit(c);
        }
        assert_eq!(page.typing(), Some((TypedField::Copies, "12")));
        page.type_backspace();
        assert_eq!(page.typing(), Some((TypedField::Copies, "1")));

        // Escape leaves the dial alone, and an empty commit is not a zero.
        page.cancel_typing();
        assert!(page.typing().is_none());
        page.begin_typing(TypedField::Copies);
        assert_eq!(page.commit_typing(), None, "enter on an empty buffer must not commit 0");
    }

    /// **A rack bigger than the page can be paged through.**
    ///
    /// The bug this is named for shipped, and it shipped *green*:
    /// `rack_scroll` was written, clamped and honoured by the renderer from
    /// the day the page landed, so it looked wired from every angle except
    /// the one that mattered -- nothing ever moved it. A rack of a hundred
    /// showed rows 1-12 and no key, click or `Action` could reach row 13.
    /// The owner found it by asking what a hundred copies would look like.
    ///
    /// So this asserts the two halves separately: the control **exists** (the
    /// half that was missing) and it **changes which rows are drawn** (the
    /// half that would pass on a control wired to nothing).
    #[test]
    fn a_rack_taller_than_the_page_can_be_scrolled() {
        const N: usize = 30;
        let chambers = rack(N);
        let mut page = Ui::new();
        let st = state(false, 1);
        let mut buf = vec![0u8; (W * H * 4) as usize];

        let drawn = |page: &Ui| -> Vec<usize> {
            page.rack_bar
                .widgets
                .iter()
                .filter(|w| w.line1.is_empty())
                .filter_map(|w| match w.action {
                    Some(Action::ChamberSelect(i)) => Some(i),
                    _ => None,
                })
                .collect()
        };

        page.paint_rack(&mut buf, &chambers, None, &st);
        let first = drawn(&page);
        assert_eq!(first.len(), RACK_ROWS, "the page shows its window, not the whole rack");
        assert_eq!(first[0], 0, "and it starts at the top");

        // The control must be on the page at all. Without this the rest of
        // the test passes on a `scroll_rack` no player can call.
        assert!(
            page.rack_bar.widgets.iter().any(|w| w.action == Some(Action::RackScroll(1))),
            "the rack has more rows than fit and drew no way to scroll -- the exact bug this guards"
        );

        page.scroll_rack(1);
        page.paint_rack(&mut buf, &chambers, None, &st);
        let second = drawn(&page);
        assert_eq!(second[0], RACK_ROWS, "one page forward starts where the last one ended");
        assert!(
            second.iter().all(|i| !first.contains(i)),
            "paging forward redrew rows the first page had already shown: {first:?} then {second:?}"
        );

        // Clamped at the far end: paging past the last row must still leave a
        // full window of real rows rather than an empty page.
        for _ in 0..10 {
            page.scroll_rack(1);
        }
        page.paint_rack(&mut buf, &chambers, None, &st);
        let last = drawn(&page);
        assert_eq!(last.len(), RACK_ROWS, "scrolled off the end and drew a short page");
        assert_eq!(*last.last().expect("rows"), N - 1, "the end of the list is reachable");

        // Back at the top, and a sort returns there too -- a window held
        // across a reorder shows you rows you did not choose.
        page.scroll_rack(-20);
        assert_eq!(page.rack_scroll(), 0, "scrolling back must not go negative");
        page.scroll_rack(1);
        page.sort_chambers(2);
        assert_eq!(page.rack_scroll(), 0, "a sort must jump back to the top");

        // A rack that fits draws no pager: a control that does nothing is
        // worse than no control.
        let mut small = Ui::new();
        small.paint_rack(&mut buf, &rack(4), None, &st);
        assert!(
            !small.rack_bar.widgets.iter().any(|w| w.action == Some(Action::RackScroll(1))),
            "a rack that fits on one page still drew a pager"
        );
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

        // Column 3 is PLT -- SEED, SET, FRAME, then the census columns.
        // Descending puts the *largest* first, which is chamber 0 here, so
        // sort ascending to actually reverse the list.
        const PLT: usize = 3;
        assert_eq!(RACK_COLS[PLT].0, "PLT", "the column indices moved and this test is now sorting on something else");
        page.sort_chambers(PLT);
        page.sort_chambers(PLT);
        assert_eq!(page.rack_sort, Some((PLT, false)), "a second click on one column reverses it");
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

    /// **The roster's header buttons are never under the cell page.**
    ///
    /// Both pages are open together by design and the cell page is painted
    /// second, sized to its own widest row and slid left when it will not fit
    /// beside this one -- so a wide one silently overpaints this page's
    /// header. It had been eating the right-hand end of BACK since both pages
    /// existed, found by cropping a contact sheet rather than by anything
    /// here, and the row now also carries CULL REST.
    ///
    /// **Asserted against the bound, not against a rendered page.** The first
    /// version of this guard read `inspect_box` -- last frame's rectangle --
    /// and passed for the broken build, because the frame that clips is the
    /// frame the WORDS group opens and the page has not been that wide
    /// before. Blind, in the way `CLAUDE.md` means it.
    #[test]
    fn the_roster_header_stays_clear_of_the_cell_page() {
        let world = world();
        let edge = W as i32 - MARGIN - widest_cell_page();
        // 177 is what the widest cell page actually measured in
        // `labui-roster--plants-pinned.png`, WORDS open, read off the pixels;
        // BACK ended at 339 there and the page began at 331.
        assert!(widest_cell_page() >= 177, "the bound is under the widest page ever photographed");
        for open in [false, true] {
            for kingdom in [roster::Kingdom::Plants, roster::Kingdom::Creatures] {
                let mut ui = Ui::new();
                ui.panel = Some(if kingdom == roster::Kingdom::Plants {
                    Panel::PlantList
                } else {
                    Panel::AntList
                });
                if open {
                    ui.inspect = Some((10, 10));
                }
                let mut buf = vec![0u8; (W * H * 4) as usize];
                let _ = ui.paint_roster(&mut buf, &world, kingdom);
                let mut seen = 0;
                for wid in &ui.roster_bar.widgets {
                    let Some(action) = wid.action else { continue };
                    if !matches!(action, Action::Panel(_) | Action::RosterFilter | Action::RosterCullRest) {
                        continue;
                    }
                    seen += 1;
                    if open {
                        assert!(
                            wid.rect.right() <= edge,
                            "{kingdom:?}: {action:?} ends at {} and the cell page can reach {edge}",
                            wid.rect.right()
                        );
                    }
                    // ...and still on the page rather than shoved off its left
                    // side, which is the way a clamp fails quietly.
                    assert!(wid.rect.x > MARGIN, "{kingdom:?}: {action:?} was pushed off the page");
                }
                assert_eq!(seen, 3, "{kingdom:?}: the header should carry CULL REST, the filter and BACK");
            }
        }
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
            for literal in ROSTER_LITERALS {
                check(literal, "roster literal");
            }
            // The keep mark, which is drawn in front of a spared row's number
            // and is the one string on this page that is neither a widget
            // face nor an empty state. It shipped blank.
            check(SPARED_MARK, "roster keep mark");
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
                    // **Markdown emphasis, asserted directly.** This used to
                    // ride on `*` having no glyph; the roster's keep mark
                    // gave the font one, so the note that found the bed's
                    // `**bold**` would now draw asterisks instead of gaps --
                    // visible, but still not what the page means to say.
                    assert!(!p.note.contains("**"), "markdown emphasis in a hover note: {:?}", p.note);
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
                    Body::Choice { label, value, .. } => {
                        check(label, "choice row");
                        check(value, "choice value");
                    }
                    Body::Spark { label, .. } => check(label, "strip caption"),
                    Body::Lines { caption, .. } => check(caption, "chart caption"),
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
        let w = world();
        let mut ui = Ui::new();
        ui.inspect(&w, (10, 20));
        assert_eq!(ui.inspecting(), Some((10, 20)));
        ui.inspect(&w, (11, 20));
        assert_eq!(ui.inspecting(), Some((11, 20)));
        ui.inspect(&w, (11, 20));
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


    // ------------------------------------------------------------- roster

    /// A world with `plants` plants and `ants` ants, each a single cell, so a
    /// roster test can say exactly how many rows it expects.
    fn peopled(plants: usize, ants: usize) -> World {
        let mut world = world();
        for (name, n) in [("tree", plants), ("ant", ants)] {
            let species = world.species.id_of(name).unwrap_or_else(|| panic!("{name} must be a loaded species"));
            let shoot = world.species.get(species).shoot_material.clone();
            let material = world.materials.id_of(&shoot).unwrap_or_else(|| panic!("{shoot} must be loaded"));
            for i in 0..n {
                let id = world.push_organism(species).expect("organism slots free");
                let (x, y) = (2 + (i as i32 % 20) * 3, if name == "ant" { 40 } else { 10 });
                world.set(x, y, crate::sim::cell::Cell::new(material, 0).with_organism_id(id));
                if name == "ant" {
                    world.organism_mut(id).expect("just made").chain = vec![(x, y)];
                }
            }
        }
        world
    }

    /// **The roster lists every live individual of its kingdom and no other.**
    ///
    /// The plainest thing it does, and the one a cached list would break: a
    /// roster is rebuilt every frame precisely because the population moves
    /// underneath it.
    #[test]
    fn a_roster_lists_every_live_organism_of_its_kingdom_and_no_other() {
        let mut world = peopled(5, 7);
        let plants = roster::rows(&world, roster::Kingdom::Plants, roster::SortKey::Slot, false, roster::Filter::All);
        let ants = roster::rows(&world, roster::Kingdom::Creatures, roster::SortKey::Slot, false, roster::Filter::All);
        assert_eq!(plants.len(), 5, "the plants table must hold every plant and nothing else");
        assert_eq!(ants.len(), 7, "the animals table must hold every animal and nothing else");
        assert_eq!(
            plants.len() + ants.len(),
            world.live_organism_count(),
            "the two tables together are the whole registry -- a row in neither is an individual you cannot reach"
        );
        assert_eq!(ants.len(), world.live_creature_count(), "the animals table is the creature count");

        // A death takes its row with it, which is the half a cached list
        // fails. Freed through the engine's own seam rather than by editing
        // the list, so this is a statement about the world and not about the
        // table.
        let doomed = ants[0].who;
        world.free_organism(doomed.id);
        let after = roster::rows(&world, roster::Kingdom::Creatures, roster::SortKey::Slot, false, roster::Filter::All);
        assert_eq!(after.len(), 6, "a freed organism kept its row");
        assert!(!after.iter().any(|r| r.who == doomed), "the dead individual is still listed");
        assert!(!doomed.alive(&world), "and it does not resolve any more");
    }

    /// **An identity survives its slot being handed to somebody else.**
    ///
    /// The guard `born_frame` exists for, and the reason a bare handle is not
    /// an identity: `encode_organism_id` gives the slot index 12 bits and the
    /// generation 4, so a handle comes back after 16 turns of one slot. A pin
    /// keyed on the handle alone would silently follow whatever animal landed
    /// in the recycled slot -- which is a different creature wearing the
    /// number of the one you were watching.
    #[test]
    fn an_individual_survives_slot_reuse_as_an_identity() {
        let mut world = peopled(0, 1);
        let species = world.species.id_of("ant").expect("ant loaded");
        let first = roster::rows(&world, roster::Kingdom::Creatures, roster::SortKey::Slot, false, roster::Filter::All)[0].who;

        // Turn the slot over until the four generation bits wrap and the
        // handle comes back. Sixteen reuses is the whole cycle, so this is
        // bounded and it is the real mechanism rather than a simulated one.
        // **Free the handle you have, not the one you started with.** A
        // freed slot comes back with its generation bumped, so the second
        // turn of the loop holds a *different* handle -- and `free_organism`
        // checks the generation, so freeing the original silently does
        // nothing and the next push takes a fresh slot instead. The first
        // version of this loop did exactly that and never collided.
        let mut current = first.id;
        let mut collided = None;
        for _ in 0..64 {
            world.free_organism(current);
            world.frame += 1;
            current = world.push_organism(species).expect("the slot was just freed");
            if current == first.id {
                collided = Some(current);
                break;
            }
        }
        let id = collided.expect("sixteen reuses of one slot must bring the handle back");
        let born = world.organism(id).expect("just made").born_frame;

        // The positive control: the halves the identity is made of really do
        // collide, or the assertion below is about nothing.
        assert_eq!(id, first.id, "the handle came back -- that is the situation being guarded");
        assert_ne!(born, first.born_frame, "and the frame did not, which is what tells them apart");

        assert!(
            first.resolve(&world).is_none(),
            "the original individual resolved to the stranger now holding its slot -- the exact bug born_frame exists to stop"
        );
        let now = roster::Individual { id, born_frame: born };
        assert!(now.alive(&world), "the new occupant is findable by its own identity");
    }

    /// **A pin is not disturbed by the list moving underneath it.**
    ///
    /// Sorting reorders every row; the pin must still name the individual it
    /// named. `Reports/dead-ends.md` records the general shape -- a selection
    /// stored as a position into a list a neighbouring verb rewrites -- and a
    /// sort is that verb.
    #[test]
    fn a_pin_survives_a_sort_and_a_filter() {
        let mut world = peopled(0, 9);
        // Spread the banks so a sort on energy is a real reorder rather than
        // a no-op over equal values -- otherwise this passes on a comparator
        // that does nothing.
        // Scrambled rather than monotone: energies handed out in registry
        // order make a descending sort the identity permutation, and the
        // control below would then fail for a comparator that works.
        for (i, id) in world.live_organism_ids().into_iter().enumerate() {
            world.organism_mut(id).expect("live").energy = [40.0, 10.0, 90.0, 20.0, 70.0, 30.0, 80.0, 50.0, 60.0][i];
        }
        let by_slot = roster::rows(&world, roster::Kingdom::Creatures, roster::SortKey::Slot, false, roster::Filter::All);
        let by_bank = roster::rows(&world, roster::Kingdom::Creatures, roster::SortKey::Energy, true, roster::Filter::All);
        assert_eq!(by_slot.len(), by_bank.len(), "a sort must not lose rows");
        assert_ne!(
            by_slot.iter().map(|r| r.who.id).collect::<Vec<_>>(),
            by_bank.iter().map(|r| r.who.id).collect::<Vec<_>>(),
            "the sort changed nothing, so this test is not testing a reorder"
        );
        assert!(by_bank.windows(2).all(|w| w[0].energy >= w[1].energy), "descending sort is not descending");

        let mut ui = Ui::new();
        ui.panel = Some(Panel::AntList);
        let pinned = by_slot[3].who;
        ui.pin(pinned);
        ui.sort_roster(1);
        assert_eq!(ui.pinned(), Some(pinned), "a sort moved the pin");
        ui.cycle_roster_filter(Some(0));
        assert_eq!(ui.pinned(), Some(pinned), "a filter moved the pin");
        assert!(pinned.alive(&world), "and the individual it names is still there");
    }

    /// **The two tables keep their own sort, scroll and filter.**
    ///
    /// A sort is stored as a *column index*, and column 1 is SEED on the
    /// plants table and BANK on the animals'. Shared, one index means "the
    /// second column, whatever that is here", which is nobody's request --
    /// and the harness caught it doing real damage: sorting the plants list
    /// and then opening the animals list drew the animals in an order nobody
    /// had chosen, so a click on the third row pinned ant 41 where ant 11 was
    /// expected.
    #[test]
    fn each_roster_keeps_its_own_sort_and_filter() {
        let mut ui = Ui::new();
        ui.panel = Some(Panel::PlantList);
        ui.sort_roster(2);
        ui.cycle_roster_filter(None);
        let plant_sort = ui.roster_sort();
        let plant_filter = ui.roster_filter();
        assert_eq!(plant_sort, Some((2, true)), "the plants table did not take the sort");

        ui.panel = Some(Panel::AntList);
        assert_eq!(ui.roster_sort(), None, "the animals table inherited the plants table's sort");
        assert_eq!(ui.roster_filter(), roster::Filter::All, "the animals table inherited the plants table's filter");
        ui.sort_roster(5);

        ui.panel = Some(Panel::PlantList);
        assert_eq!(ui.roster_sort(), plant_sort, "the plants table lost its sort to the animals table");
        assert_eq!(ui.roster_filter(), plant_filter, "the plants table lost its filter");

        // And the two column lists really do disagree about what index 1
        // means, or the whole hazard above is theoretical.
        assert_ne!(
            PLANT_COLS[1].0, ANT_COLS[1].0,
            "the two tables now agree on column 1, so this guard is guarding nothing -- re-derive it"
        );
    }

    /// **A roster taller than the page can be scrolled, and the two halves
    /// are asserted separately.**
    ///
    /// The rack shipped `rack_scroll` written, clamped and honoured by the
    /// renderer with **nothing bound to move it** -- a rack of a hundred
    /// showed rows 1-12 for ever and every guard over it passed. So: the
    /// control **exists**, and it **changes which rows are drawn**.
    #[test]
    fn a_roster_taller_than_the_page_can_be_scrolled() {
        let world = peopled(0, 40);
        let mut ui = Ui::new();
        ui.panel = Some(Panel::AntList);
        let mut buf = vec![0u8; (W * H * 4) as usize];

        let drawn = |ui: &Ui| -> Vec<usize> {
            ui.roster_bar
                .widgets
                .iter()
                .filter(|w| w.line1.is_empty())
                .filter_map(|w| match w.action {
                    Some(Action::RosterSelect(i)) => Some(i),
                    _ => None,
                })
                .collect()
        };

        ui.paint_roster(&mut buf, &world, roster::Kingdom::Creatures);
        let first = drawn(&ui);
        assert!(!first.is_empty() && first.len() < 40, "the page must show a window of a 40-row list, not all of it or none");
        assert_eq!(first[0], 0, "and it starts at the top");
        assert!(
            ui.roster_bar.widgets.iter().any(|w| w.action == Some(Action::RosterScroll(1))),
            "the roster has more rows than fit and drew no way to scroll -- the exact bug this guards"
        );

        ui.scroll_roster(1);
        ui.paint_roster(&mut buf, &world, roster::Kingdom::Creatures);
        let second = drawn(&ui);
        assert!(
            second.iter().all(|i| !first.contains(i)),
            "paging forward redrew rows the first page had shown: {first:?} then {second:?}"
        );

        // Clamped at the far end, and the end is reachable.
        for _ in 0..10 {
            ui.scroll_roster(1);
        }
        ui.paint_roster(&mut buf, &world, roster::Kingdom::Creatures);
        let last = drawn(&ui);
        assert_eq!(*last.last().expect("rows"), 39, "the end of the list is not reachable");
        assert_eq!(last.len(), first.len(), "scrolled off the end and drew a short page");

        ui.scroll_roster(-20);
        assert_eq!(ui.roster_scroll(), 0, "scrolling back must not go negative");
        ui.scroll_roster(1);
        ui.sort_roster(1);
        assert_eq!(ui.roster_scroll(), 0, "a sort must jump back to the top");

        // A roster that fits draws no pager: a control that does nothing is
        // worse than no control.
        let small = peopled(0, 3);
        let mut tiny = Ui::new();
        tiny.panel = Some(Panel::AntList);
        tiny.paint_roster(&mut buf, &small, roster::Kingdom::Creatures);
        assert!(
            !tiny.roster_bar.widgets.iter().any(|w| w.action == Some(Action::RosterScroll(1))),
            "a roster that fits on one page still drew a pager"
        );
    }

    /// **The roster page stays on the screen, at every length it can be.**
    ///
    /// Its height is a sum of terms and every one of them was added by
    /// somebody: the rack's pager and its verbs were each added without a
    /// term, which put that panel 327 px tall on a 320 px screen and dropped
    /// its own ENTER button behind the bar.
    #[test]
    fn the_roster_page_stays_on_the_screen() {
        let mut buf = vec![0u8; (W * H * 4) as usize];
        for n in [0usize, 1, 5, 14, 15, 40, 200] {
            let world = peopled(0, n.min(60));
            for pinned in [false, true] {
                let mut ui = Ui::new();
                ui.panel = Some(Panel::AntList);
                if pinned {
                    let rows = roster::rows(&world, roster::Kingdom::Creatures, roster::SortKey::Slot, false, roster::Filter::All);
                    if let Some(r) = rows.first() {
                        ui.pin(r.who);
                    }
                }
                ui.paint_roster(&mut buf, &world, roster::Kingdom::Creatures);
                let rect = ui.roster_box.expect("the page was drawn");
                assert!(rect.y >= MARGIN, "n={n} pinned={pinned}: the page starts at y={} -- off the top", rect.y);
                assert!(rect.bottom() <= bar_top(), "n={n} pinned={pinned}: the page runs into the bar");
                assert!(rect.x >= 0 && rect.right() <= W as i32, "n={n} pinned={pinned}: the page runs off the side");
                // **Every widget inside its own panel.** The assertion the
                // rack was missing: a page can be the right height and still
                // draw its verbs below itself.
                for wid in &ui.roster_bar.widgets {
                    assert!(
                        wid.rect.y >= rect.y && wid.rect.bottom() <= rect.bottom(),
                        "n={n} pinned={pinned}: a {:?} widget at y={} is outside the panel {}..{}",
                        wid.action,
                        wid.rect.y,
                        rect.y,
                        rect.bottom()
                    );
                }
            }
        }
    }

    /// **A roster with no way out is a page a player is stuck on.**
    ///
    /// No bar chip carries `Action::Panel(PlantList)` -- that is the whole
    /// reason the roster hangs off the PLANTS page -- so "press whatever
    /// opened it" cannot close this one. The harness found it twice by
    /// panicking; this is the same finding as a test.
    #[test]
    fn a_roster_can_be_left_by_a_button_on_it() {
        let world = peopled(2, 2);
        let mut buf = vec![0u8; (W * H * 4) as usize];
        for (panel, kingdom, back_to) in [
            (Panel::PlantList, roster::Kingdom::Plants, Panel::Plants),
            (Panel::AntList, roster::Kingdom::Creatures, Panel::Ants),
        ] {
            let mut ui = Ui::new();
            ui.panel = Some(panel);
            ui.paint_roster(&mut buf, &world, kingdom);
            assert!(
                ui.widget_rect(Action::Panel(back_to)).is_some(),
                "{panel:?} drew no way back to {back_to:?}"
            );
            // And the way out is not the page's own action, which is what a
            // caller would reach for and which is not on screen.
            assert!(
                ui.widget_rect(Action::Panel(panel)).is_none(),
                "{panel:?} is reachable from itself, so this guard is not testing what it says"
            );
        }
    }

    /// **The sort keys are pinned to their columns.**
    ///
    /// The rack keys its sort on a bare column index in a `match`, so
    /// inserting a column there silently sorts on its neighbour; carrying the
    /// key on the column makes an insert a compile-time move. This asserts
    /// the pairing anyway, so a *reordering* -- which the compiler cannot see
    /// -- breaks loudly. PR 3 inserts an offspring column into both tables.
    #[test]
    fn the_roster_columns_and_their_sort_keys_agree() {
        assert_eq!(PLANT_COLS[0].0, "SPECIES");
        assert_eq!(PLANT_COLS[2].0, "SEED");
        assert_eq!(PLANT_COLS[2].2, roster::SortKey::Score, "the SEED column must sort on what it shows");
        assert_eq!(ANT_COLS[1].0, "BANK");
        assert_eq!(ANT_COLS[3].0, "YOUNG");
        assert_eq!(ANT_COLS[3].2, roster::SortKey::Score, "the YOUNG column must sort on what it shows");
        assert_eq!(ANT_COLS[1].2, roster::SortKey::Energy, "the BANK column must sort on what it shows");
        assert_eq!(PLANT_COLS[7].0, "STATE");
        assert_eq!(ANT_COLS[7].0, "STATE");
        for cols in [&PLANT_COLS, &ANT_COLS] {
            for (head, widest, _) in cols {
                assert!(
                    hud::text_width(head) <= hud::text_width(widest),
                    "column {head:?} is wider than the widest value it claims to hold ({widest:?}) -- the header will overlap its neighbour"
                );
            }
        }
    }

    /// **The roster's order is total**, so no sort implementation ever has a
    /// tie to break.
    ///
    /// This started as "sort the same list eight times and check the answer
    /// does not move", and that guard was **blind**: it stayed green with the
    /// tie-break deleted and the sort switched to `sort_unstable_by`, because
    /// a sort is deterministic within one build whatever its comparator says.
    /// The hazard `CLAUDE.md` records is that ipnsort picks its small-sort
    /// strategy from the *element type*, so two sorts asking the comparator
    /// identical questions can still order equal elements differently -- and
    /// nothing inside one build can see that.
    ///
    /// So this asserts the property that makes the hazard harmless instead:
    /// `roster::compare` returns `Equal` only for a row against itself. With
    /// no ties there is nothing for an implementation to choose.
    #[test]
    fn a_roster_sort_is_total_over_ties() {
        use std::cmp::Ordering;
        let world = peopled(4, 12);
        for (kingdom, n) in [(roster::Kingdom::Plants, 4), (roster::Kingdom::Creatures, 12)] {
            let rows = roster::rows(&world, kingdom, roster::SortKey::Slot, false, roster::Filter::All);
            assert_eq!(rows.len(), n, "the positive control: {n} rows to compare");
            for key in [
                roster::SortKey::Slot,
                roster::SortKey::Species,
                roster::SortKey::Cells,
                roster::SortKey::Energy,
                roster::SortKey::Carrying,
                roster::SortKey::Score,
                roster::SortKey::Generation,
                roster::SortKey::Lineage,
                roster::SortKey::Age,
                roster::SortKey::State,
            ] {
                for desc in [false, true] {
                    for a in &rows {
                        assert_eq!(
                            roster::compare(a, a, key, desc),
                            Ordering::Equal,
                            "{key:?}: a row does not compare equal to itself"
                        );
                        for b in &rows {
                            if a.who == b.who {
                                continue;
                            }
                            assert_ne!(
                                roster::compare(a, b, key, desc),
                                Ordering::Equal,
                                "{key:?} desc={desc}: two distinct rows are tied, so the sort has a choice to make and the toolchain makes it"
                            );
                        }
                    }
                }
            }
        }
        // The positive control for the control: these rows really are tied on
        // the underlying quantity, or "no ties" is true for an uninteresting
        // reason.
        let ants = roster::rows(&world, roster::Kingdom::Creatures, roster::SortKey::Slot, false, roster::Filter::All);
        assert!(
            ants.iter().all(|r| r.generation == ants[0].generation),
            "every ant here should share a generation -- otherwise the tie-break is not being exercised"
        );
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
            // **Give the animal the brain its species ships with.**
            // `push_organism` allocates the slot; `place_creature` is what
            // normally fills the genome in, and this test does not go through
            // it. Without this the ant's `WORDS` group reads `NO BRAIN YET`
            // and is seven rows instead of fourteen -- so the page fits, the
            // fold never fires, and the guard is measuring a specimen no
            // player will ever open a page on.
            let species_genome = world.species.get(species).genome.clone();
            world.organism_mut(id).expect("just pushed").genome = species_genome;
            let shoot = world.species.get(species).shoot_material.clone();
            let material = world.materials.id_of(&shoot).unwrap_or_else(|| panic!("{name}'s shoot material {shoot} must be loaded"));
            world.set(10, 10, crate::sim::cell::Cell::new(material, 0).with_organism_id(id));
            let ui = Ui::new();

            // The positive control, and it is the whole reason either arm
            // means anything: laid out flat this page has to be too tall, or
            // a fold that did nothing would pass.
            let sections = params::specimen_sections(&world, id);
            assert_eq!(sections.len(), 4, "{name}: every kingdom gets the same four groups");
            assert_eq!(sections[0].0, "WORDS", "{name}: the summary leads, and `Ui::new` defaults to index 0");
            assert!(!sections[0].2.is_empty(), "{name}: the summary group is empty, so the page would draw a heading over nothing");
            let flat = 5 * LINE + 4 + sections.len() as i32 * (LINE + 4) + sections.iter().map(|(_, _, r)| r.len() as i32 * LINE).sum::<i32>();
            // **Both kingdoms overflow now, where only the plant did.** The
            // `WORDS` group added eight to fourteen rows, so the fold is
            // load-bearing on an ant's page too -- and this stays an equality
            // against a literal rather than becoming `assert!(over)`, so that
            // a future change making either page fit flat fails here and says
            // the fold has stopped being tested, instead of passing quietly.
            let over = flat > page_content_budget();
            assert!(over, "{name}: flat page is {flat}px against a {}px budget -- it fits without folding, so the fold is not being tested at all", page_content_budget());

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
            // **Both kingdoms fold now**, where only the plant did before:
            // the `WORDS` group adds eight to fourteen rows and neither page
            // holds all four groups at once. What matters is not how many are
            // shut but that the page fits *because* it folded rather than
            // because rows were trimmed, which the assertion above checks.
            let shut = rows.iter().filter(|row| matches!(row.body, Body::Head { open: false, .. })).count();
            assert!(shut >= 1, "{name}: nothing folded, so the fold is not being tested on this kingdom");
            assert!(shut < sections.len(), "{name}: every group is shut, so the page shows no rows at all");

            // **A click opens what it says, whichever group it is.** The half
            // the fit assertions cannot reach: a rule that served the groups
            // strictly from the top would fit the page perfectly and ignore
            // the player, and every assertion above would still be green.
            for chosen in 0..sections.len() {
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
                assert_eq!(heads.len(), sections.len(), "{name}: the page lost a group heading");
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

    /// **A still world does not sample itself repeatedly** — the cadence
    /// gate, on its own.
    ///
    /// This is only half the claim, and for a while it was the whole guard
    /// under the name of the other half. See
    /// `the_series_is_sampled_on_simulated_time_not_on_draws` below for why
    /// driving `observe` in a loop cannot say anything about the call site,
    /// which is where the defect was.
    #[test]
    fn a_still_world_yields_one_sample() {
        let mut history = History::default();
        let world = world();
        for _ in 0..500 {
            history.observe(&world);
        }
        assert_eq!(history.samples.len(), 1, "a still world sampled itself repeatedly");
        assert!(history.delta(|s| s.plants as i64).is_none(), "one sample is not a delta");
    }

    /// **The population series is sampled on simulated frames, never on
    /// drawn ones**, driven through a real `Lab` at the top of the ladder.
    ///
    /// **The guard that carried this name for a while could not fail.** It
    /// called `History::observe` five hundred times against a world whose
    /// `frame` never moved, and asserted one sample came out. That exercises
    /// the cadence gate — which was never wrong — and says nothing about the
    /// call site, which was: both `observe` calls sat in `Lab::advance`,
    /// after the tick loop, so at 256x the gate was offered one chance per
    /// 256 simulated frames and could not fire at its own 120-frame interval
    /// however short that interval was. The strip's x-axis was the speed
    /// dial, and it stayed green through all of it.
    ///
    /// So this drives `advance` rather than `observe`, at a multiplier where
    /// a batch is longer than `SAMPLE_EVERY` — the only regime in which the
    /// two placements differ at all. **The assertion is the spacing, not the
    /// count**, because how many ticks a batch actually runs depends on
    /// `Plan::budget` and therefore on the machine: `CLAUDE.md`'s rule that a
    /// wall-clock-dependent assertion is a flake generator. Spacing is exact
    /// either way, since `tick` advances `World::frame` by one.
    #[test]
    fn the_series_is_sampled_on_simulated_time_not_on_draws() {
        let mut lab = crate::lab::Lab::new(crate::lab::scene::LabBox {
            width: 256,
            height: 192,
            ground_y: 96,
            soil_depth: 48,
            founders: 2,
            colonies: 0,
            ..crate::lab::scene::LabBox::default()
        });
        lab.time.requested = 256;
        lab.time.phase = crate::lab::time::Phase::Running;
        // Enough batches that a per-batch sampler and a per-tick one cannot
        // agree by luck, and enough frames for several intervals to elapse.
        for _ in 0..40 {
            lab.advance(std::time::Duration::from_millis(16));
        }
        let frames: Vec<u64> = lab.ui.history.samples.iter().map(|s| s.frame).collect();
        assert!(
            frames.len() >= 3,
            "only {} samples over {} simulated frames -- the series is not being fed",
            frames.len(),
            lab.world.frame
        );
        for pair in frames.windows(2) {
            assert_eq!(
                pair[1] - pair[0],
                SAMPLE_EVERY,
                "samples {} and {} are {} simulated frames apart, not {SAMPLE_EVERY} -- the cadence is following the draw loop, not the world. Frames: {frames:?}",
                pair[0],
                pair[1],
                pair[1] - pair[0]
            );
        }
    }

    /// **The comparison pairs by label, counts what differs, and puts those
    /// rows first.**
    ///
    /// The ordering is the claim worth guarding, not the pairing: kept in
    /// specimen order, the two shipped ants put fourteen identical
    /// plain-speech lines above the nine rows that actually differed, so a
    /// page opened to ask "why is this one doing better" answered below the
    /// fold. `fit_rows` trims from the bottom, so this is also what decides
    /// which rows survive an overrun.
    #[test]
    fn the_comparison_leads_with_what_differs() {
        let mut lab = crate::lab::Lab::new(crate::lab::scene::LabBox {
            colonies: 1,
            founders: 2,
            ..crate::lab::scene::LabBox::default()
        });
        for _ in 0..2000 {
            lab.tick_for_harness();
        }
        let live = roster::rows(
            &lab.world,
            roster::Kingdom::Creatures,
            roster::SortKey::Slot,
            false,
            roster::Filter::All,
        );
        assert!(live.len() >= 2, "need two animals to compare, got {}", live.len());
        let (a, b) = (live[0].who, live[1].who);

        // **Nothing held: the page says so rather than drawing blank.**
        lab.ui.pin(b);
        let none = lab.ui.compare_rows(&lab.world);
        assert!(
            none.iter().any(|r| matches!(&r.body, Body::Value { label, .. } if label == "NOTHING TO COMPARE")),
            "a half-set comparison drew something other than its own empty state"
        );

        // **The positive control, and it is an individual against itself.**
        // Every row must match, so a `DIFFER` that is not zero here means the
        // pairing is wrong rather than the individuals being different --
        // which no comparison of two *different* animals could tell apart.
        lab.ui.held = Some(b);
        let mirror = lab.ui.compare_rows(&lab.world);
        let differ_of = |rows: &[Row]| -> (usize, usize) {
            for r in rows {
                if let Body::Value { label, value, .. } = &r.body {
                    if label == "DIFFER" {
                        let mut it = value.split(" OF ");
                        let d = it.next().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
                        let t = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                        return (d, t);
                    }
                }
            }
            panic!("the comparison page has no DIFFER row at all, in {} rows", rows.len())
        };
        let (d, total) = differ_of(&mirror);
        assert_eq!(d, 0, "an individual differs from itself in {d} of {total} rows");
        assert!(total > 10, "only {total} rows were paired at all; the page is not reading the specimen");

        // **Two different animals differ somewhere**, or the page is
        // reporting sameness it cannot have established.
        lab.ui.held = Some(a);
        let real = lab.ui.compare_rows(&lab.world);
        let (d, total) = differ_of(&real);
        assert!(d > 0, "two different animals compared identical across all {total} rows");

        // ...and those rows lead. Every bright row before every dim one.
        let tints: Vec<[u8; 4]> = real
            .iter()
            .skip_while(|r| !matches!(&r.body, Body::Value { label, .. } if label != "DIFFER" && label != "HELD" && label != "PINNED"))
            .filter_map(|r| match &r.body {
                Body::Value { tint, .. } => Some(*tint),
                _ => None,
            })
            .collect();
        let last_bright = tints.iter().rposition(|t| *t == VALUE);
        let first_dim = tints.iter().position(|t| *t == FAINT);
        if let (Some(bright), Some(dim)) = (last_bright, first_dim) {
            assert!(bright < dim, "a matching row ({dim}) sorts above a differing one ({bright})");
        }
    }

    /// **The watch ring is about one individual, samples on simulated time,
    /// and keeps what it saw after that individual dies.**
    ///
    /// Three claims in one guard because they share an expensive fixture (a
    /// bed with a live colony, run far enough to fill a ring), and because
    /// the third is only meaningful given the first two.
    #[test]
    fn the_watch_ring_follows_one_individual_and_outlives_it() {
        let mut lab = crate::lab::Lab::new(crate::lab::scene::LabBox {
            colonies: 1,
            founders: 2,
            ..crate::lab::scene::LabBox::default()
        });
        for _ in 0..400 {
            lab.tick_for_harness();
        }
        let live = roster::rows(
            &lab.world,
            roster::Kingdom::Creatures,
            roster::SortKey::Slot,
            false,
            roster::Filter::All,
        );
        assert!(live.len() >= 2, "the bed has no colony to watch: {} animals", live.len());
        let (a, b) = (live[0].who, live[1].who);

        // **Nothing is sampled without a pin.** The ring is fed from the pin
        // and from nothing else, so an unpinned box must leave it empty --
        // otherwise it is quietly walking every organism every tick.
        for _ in 0..200 {
            lab.tick_for_harness();
        }
        assert!(lab.ui.watch.is_empty(), "the ring sampled with nothing pinned");

        lab.ui.pin(a);
        for _ in 0..(WATCH_EVERY as usize * 6) {
            lab.tick_for_harness();
        }
        let frames: Vec<u64> = lab.ui.watch.samples.iter().map(|t| t.frame).collect();
        assert!(frames.len() >= 4, "only {} samples: the ring is not being fed", frames.len());
        for pair in frames.windows(2) {
            assert_eq!(
                pair[1] - pair[0],
                WATCH_EVERY,
                "samples are {} simulated frames apart, not {WATCH_EVERY}: {frames:?}",
                pair[1] - pair[0]
            );
        }
        assert!(lab.ui.watch.about(a.id), "the ring does not know who it is about");
        assert!(!lab.ui.watch.about(b.id), "the ring answers for an individual it is not watching");

        // **Re-pinning clears it.** A ring carried across a change of subject
        // draws one animal's path under another's name, which is worse than
        // drawing nothing: it is a wrong answer that looks like a right one.
        lab.ui.release_pin();
        lab.ui.pin(b);
        lab.tick_for_harness();
        assert!(
            lab.ui.watch.samples.iter().all(|t| t.frame >= frames[frames.len() - 1]),
            "the ring kept the previous individual's samples after the pin moved"
        );

        // **And it survives the death of its subject**, which is the whole
        // reason the path lives on the ring rather than being recomputed from
        // the world. Where an ant got to before it starved is exactly what
        // its graveyard row cannot say.
        lab.ui.release_pin();
        lab.ui.pin(a);
        for _ in 0..(WATCH_EVERY as usize * 8) {
            lab.tick_for_harness();
        }
        let held = lab.ui.watch.len();
        assert!(held > 0, "nothing to lose before the death");
        lab.world.mark_organism_senescent(a.id);
        lab.world.free_organism(a.id);
        assert!(!a.alive(&lab.world), "the cull did not take");
        for _ in 0..(WATCH_EVERY as usize * 4) {
            lab.tick_for_harness();
        }
        assert_eq!(
            lab.ui.watch.len(),
            held,
            "the trail changed after its subject died -- it should stop growing and lose nothing"
        );
        assert!(lab.ui.watch.about(a.id), "the ring stopped answering for a dead individual it still holds");
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

    /// **Two founded colonies produce two group series, and a group that
    /// dies out reads 0 rather than dropping out of its own line.**
    ///
    /// The scene is `creature.rs`'s own
    /// `a_founding_is_one_colony_and_the_next_founding_is_another` (a stone
    /// floor, two `found_colony_of` calls) -- proven there to actually hold
    /// two separate colonies, so this borrows it rather than re-deriving a
    /// scene that might not. What is new here is reading it through
    /// `History::group_series`, which is what the ANTS page's chart and
    /// legend actually call.
    #[test]
    fn two_founded_colonies_are_two_group_series() {
        let mut world = World::new(WorldRect::new(0, 0, 199, 199));
        for x in 10..190 {
            world.set(x, 101, crate::sim::cell::Cell::new(crate::sim::material::STONE, 0));
        }
        let first_n = world.found_colony_of(50, 100, "ant", 4);
        let second_n = world.found_colony_of(150, 100, "ant", 4);
        assert!(first_n >= 2 && second_n >= 2, "the scene must actually hold two colonies: placed {first_n} and {second_n}");

        let mut history = History::default();
        history.observe(&world);

        let groups = world.live_creature_groups();
        assert_eq!(groups.len(), 2, "two foundings are two groups: {groups:?}");
        let (sp0, co0, alive0) = (groups[0].species, groups[0].colony, groups[0].alive);
        let (sp1, co1, alive1) = (groups[1].species, groups[1].colony, groups[1].alive);

        let series0 = history.group_series(move |sp, co| sp == sp0 && co == co0);
        let series1 = history.group_series(move |sp, co| sp == sp1 && co == co1);
        assert_eq!(series0.last().copied(), Some(alive0), "the first colony's series must end at its own alive count");
        assert_eq!(series1.last().copied(), Some(alive1), "the second colony's series must end at its own alive count");

        // Free every animal in the first colony, so it is missing from the
        // next sample -- and `group_series` must read that sample as 0, not
        // as one entry shorter than the surviving colony's.
        let doomed: Vec<u16> =
            world.live_organism_ids().into_iter().filter(|&id| world.organism(id).is_some_and(|s| s.colony == co0)).collect();
        assert!(!doomed.is_empty(), "nothing to free -- this test would prove nothing");
        for id in doomed {
            world.free_organism(id);
        }
        world.frame += SAMPLE_EVERY;
        history.observe(&world);

        let series0_after = history.group_series(move |sp, co| sp == sp0 && co == co0);
        let series1_after = history.group_series(move |sp, co| sp == sp1 && co == co1);
        assert_eq!(series0_after.last().copied(), Some(0), "a colony that died out must draw a floor, not stop -- it read {series0_after:?}");
        assert_eq!(series1_after.last().copied(), Some(alive1), "the surviving colony's own count should not have moved");
        assert_eq!(series0_after.len(), series1_after.len(), "every group's series must carry one entry per sample in the ring");
    }

    /// **The ANTS page's legend names both founded colonies, each in its own
    /// tint, in placement order.** The proof that `Panel::Ants` is actually
    /// wired to `capped_group_rows` and `render::group_colour` rather than
    /// merely compiling against them -- same scene as
    /// `two_founded_colonies_are_two_group_series` above.
    #[test]
    fn the_ants_legend_names_both_colonies_in_placement_order() {
        let mut world = World::new(WorldRect::new(0, 0, 199, 199));
        for x in 10..190 {
            world.set(x, 101, crate::sim::cell::Cell::new(crate::sim::material::STONE, 0));
        }
        let first_n = world.found_colony_of(50, 100, "ant", 4);
        let second_n = world.found_colony_of(150, 100, "ant", 4);
        assert!(first_n >= 2 && second_n >= 2, "the scene must actually hold two colonies: placed {first_n} and {second_n}");
        let groups = world.live_creature_groups();
        assert_eq!(groups.len(), 2, "the scene must hold exactly two colonies: {groups:?}");

        let mut ui = Ui::new();
        // Colony mode -- species mode would sum both colonies (same species,
        // "ant") into one row, and this test is about telling two colonies of
        // *one* species apart, which is the harder of the two grouping modes.
        ui.set_creature_colour(render::CreatureColour::Colony);
        ui.observe(&world);

        let spec = LabBox::default();
        let rows = ui.panel_rows(Panel::Ants, &world, &spec, 60.0);
        let labels: Vec<&str> = rows
            .iter()
            .filter_map(|row| match &row.body {
                Body::Value { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .collect();
        let legend: Vec<(&str, [u8; 4])> = rows
            .iter()
            .filter_map(|row| match &row.body {
                Body::Value { label, tint, .. } if label.starts_with("ANT ") => Some((label.as_str(), *tint)),
                _ => None,
            })
            .collect();
        assert_eq!(legend.len(), 2, "the legend must carry one row per founded colony -- page labels were {labels:?}");
        assert_ne!(legend[0].0, legend[1].0, "two different colonies must not share a label");
        assert_ne!(legend[0].1, legend[1].1, "two different colonies must not share a tint");
        // Placement order: `live_creature_groups` sorts by colony, and the
        // colony founded first (at x=50) claimed the lower colony number.
        assert!(
            legend[0].0.contains(&groups[0].colony.to_string()) && legend[1].0.contains(&groups[1].colony.to_string()),
            "the legend must list colonies in placement order: {legend:?} against {groups:?}"
        );

        // **A colony that dies out stays on the page at 0** while the ring
        // remembers it -- the beetles that vanished from the first draft's
        // legend the moment they starved. Free every animal of the first
        // colony, sample again, and the legend still carries both rows with
        // the dead one reading 0 on its face.
        let doomed: Vec<u16> = world
            .live_organism_ids()
            .into_iter()
            .filter(|&id| world.organism(id).is_some_and(|s| s.colony == groups[0].colony))
            .collect();
        for id in doomed {
            world.free_organism(id);
        }
        world.frame += SAMPLE_EVERY;
        ui.observe(&world);
        let rows = ui.panel_rows(Panel::Ants, &world, &spec, 60.0);
        let faces: Vec<(String, String)> = rows
            .iter()
            .filter_map(|row| match &row.body {
                Body::Value { label, value, .. } if label.starts_with("ANT ") => Some((label.clone(), value.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(faces.len(), 2, "the wiped-out colony must stay listed: {faces:?}");
        assert!(faces[0].1.starts_with('0'), "the dead colony's face must read 0 alive: {faces:?}");
        assert_eq!(faces[0].0, legend[0].0, "and it keeps its place in the order: {faces:?}");
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
