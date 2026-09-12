# Lane note — a master menu (round 30, R2)

Owner, 2026-09-12: *"There are lots of hidden menus that can only be accessed
by knowing the F key. There should be a master menu accessible from the main
UI that leads to all the other menus."* Plus: *"The menu could be revamped
too."*

## What changed

`Panel::Menu` (`src/lab/ui.rs`), opened by a `MENU` bar chip and by `F6`. It
lists every page and every view toggle, one `Row::choice` each: destination
left, the key that also reaches it (or the live value, for two controls
with no other readout — see below) right. Every row fires the exact
`Action` its key already fired — no second definition of a control.
Re-derived the page list rather than trusting the brief's own table, which
was already stale (`PlantList`/`AntList`/`Log`/`Scenarios`/`Compare` had
mouse routes, just none from one place).

## The bar: consolidation, not addition

`PIXEL_PHYSICS_BAR_TRACE=1` before this change: row 0 and row 1 both **508
of 508, zero slack**, at the tightest of the three `SPACINGS` — the bar
takes nothing more without removing something first (`Reports/dead-ends.md`,
the `SHELF` entry, already established this for row 0).

**`MENU` replaces `PLANTS`/`ANTS`/`BOX`** on row 1 (three pure-nav chips
folded into one) **and `PARAMS` moves off row 0 into a MENU row** (`P`
still opens it directly) — both pure navigation, unlike the jar chip, which
stayed because it carries live state (`SHELF` entry, same reasoning
reversed).

Measured after: **row 0 and row 1 both fit at 508 of 508 with zero overflow
at every one of the three `SPACINGS`, including the loosest** (`pad=2
gap=2`) — not merely passing, real headroom for the first time since
`SHELF` landed. Button count **29 → 26**
(`every_button_answers_where_it_was_drawn`, updated in the same change).

**A bare-add alternative was built and measured, not argued** — see
`dead-ends.md`: adding `MENU` on top of everything (no removal) also passes
`fits()`, but only at the tightest spacing, both rows at exactly 0 slack —
worse than consolidation, comfortable at every spacing. Rejected on that.

## Two verbs that had no button at all now have one

`Digit9` (write the chronicle) and `KeyF` (`cycle_display_floor`) were direct
calls specifically because — their own comments said so — *"this one has no
button."* Now one exists, so both route through `Lab::act` as `Action::
WriteChronicle`/`Action::CycleDisplayFloor`, the rule those comments
themselves state.

## The display-floor handoff (lane R1) — closed

R1 left `MIN {}HZ` as a temporary second line in the corner
(`TimeControl::readout`), explicitly for this lane to re-home — the one
line R1 could not delete because no other readout of `F`'s setting existed
anywhere. **It now has one, in the row's value slot, not just its label**:
`DISPLAY FLOOR`'s right-aligned field reads `MIN {n}HZ` live off
`TimeControl::display_floor` (mirrored into `Ui`, `ANIMAL COLOUR`/`OVERLAY`'s
own pattern), key named in the note. Corner's second line removed; ticks
alone remain. Owner: *"Much better"* on R1's card, then *"remove Min 10hz
too"* once this row existed — one complaint, closed in two steps.

## The magnify style/ink/level/grain handoff (round 30's style lane, #352)

Judgement call: **a VIEW grouping on MENU, not a PARAMS row** — these are
`Renderer`-only view state, structurally identical to `CycleOverlay`/
`CycleCreatureColour` (not PARAMS rows either), and PARAMS' `Knob`/`write`/
`save` machinery never reaches `Renderer`. `MAGNIFY STYLE` gets key `0`
(the digit row's last free key) plus a MENU row; the other four are MENU
rows only. Style/notch/ink reuse `Renderer::cycle_magnify_style/notch/ink`
(outdoor's `Shift+=`/`Shift+[`/`Shift+]`, so both games step identically).
`cycle_magnify_level`/`cycle_magnify_grain` **didn't exist when this lane
started** — `render.rs` was the style lane's file — and were added here,
`cycle_magnify_ink`'s own shape, once the coordinator lifted that fence.
All five now share one mechanism. Default stays `MagnifyStyle::CellArt`,
byte-identical to a build without the enum.

## What a later lane should not have to re-derive

**`lay_out`'s "508 of 508, zero slack" does not mean the same thing on both
rows.** Slack is redistributed into the gaps *between* groups, so any row
with non-negative slack always reports exactly its own width — only an
*overflow* (`right > 508` at a looser spacing) says a row is out of room.
Row 1 read "508 of 508" at every spacing even before this change and was
never the bottleneck; row 0's overflow at the two looser spacings (`522`,
`512`) was.

## Verification

- `cargo test --lib`: 1,655 passed, 70 ignored, 0 failed (one hard-coded
  button count updated: `every_button_answers_where_it_was_drawn`, 29 → 26).
- `the_bar_fits_the_screen_and_no_two_widgets_overlap` watched red: a
  deliberately oversized `MENU` label overflowed it before the label was
  restored.
- `cargo clippy --all-targets --release --locked -- -D warnings`: clean.
- `cargo run --release --example ascii`: 31 scenes, 0 skipped -- unmoved.
- `bash scripts/acceptance.sh`: all cases met expectations.
- `bash scripts/worldgencheck.sh`, `bash scripts/docscheck.sh`: both clean.
- `examples/labui.rs` broke at runtime (not compile time): every "reach this
  panel"/"leave whatever is open" idiom assumed `PLANTS`/`ANTS`/`BOX`/
  `PARAMS` always had a persistent widget, true until this lane. Two shared
  helpers (`reach`, `leave_open_panel`) fixed every call site at once --
  caught by running the harness, not by any existing guard.
- MENU itself overran its budget by 1px (26 rows, 229px against 228px) and,
  being a generic panel page, was never routed through `fit_rows` -- every
  other such page is short enough nobody had hit this. Fixed both: added
  the call, and dropped the two group-label rows for real margin (211px).
- Review card `20260912T162217714Z-bbc1aa`, board `lab`: before/after of the
  bar at rest, plus the menu open, with the rejected bare-add option's
  numbers in `meta`.
