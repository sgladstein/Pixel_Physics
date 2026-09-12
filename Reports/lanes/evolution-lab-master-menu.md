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

**Re-derived page list** (the brief's table was stale — `PlantList`/
`AntList`/`Log`/`Scenarios`/`Compare` already had mouse routes, just none
from one place): `Plants`(F1)/`Ants`(F2)/`Box`(F3)/`Params`(P)/`Shelf`(G)/
`Chambers`(F4)/`History`(F5), plus those five with a row elsewhere, no key.

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
calls specifically because — their own source comments said so — *"this one
has no button."* Now one exists (`SAVE CHRONICLE NOW`, `DISPLAY FLOOR` on
the MENU page), so both route through `Lab::act` as `Action::WriteChronicle`/
`Action::CycleDisplayFloor`, the rule those comments themselves state:
`Lab::act` dispatches a verb a button also draws.

## The display-floor handoff (lane R1) — closed

R1 left `MIN {}HZ` as a temporary second line in the top-left corner
(`TimeControl::readout`), explicitly for this lane to re-home — it was the
one line R1 could not delete because no other readout of `F`'s setting
existed anywhere. **It now has one, and the value is in the row's value
slot, not just its label**: `DISPLAY FLOOR`'s right-aligned field reads
`MIN {n}HZ` live off `TimeControl::display_floor` (mirrored into `Ui` the
same way `ANIMAL COLOUR`/`OVERLAY` mirror `Renderer` state), with the key
named in the note instead. The corner's second line is removed (`time.rs`);
ticks alone remain. Owner's verdict on R1's card: *"Much better"*; on the
follow-up once this row existed: *"remove Min 10hz too"* — the same
complaint, closed in two steps by two lanes.

## The magnify style/ink/level/grain handoff (round 30's style lane, #352)

Judgement call, as the coordinator's handoff invited: **a VIEW grouping on
the MENU page, not a PARAMS row.** These are `Renderer`-only view settings —
structurally identical to `CycleOverlay`/`CycleCreatureColour`, which are
not PARAMS rows either — not simulation truth. PARAMS' `Knob`/`write`/`save`
machinery only reaches `World`/`LabBox`, and reaches `Renderer` nowhere; a
value nobody has seen in the lab yet did not earn extending that machinery.

- `MAGNIFY STYLE` — key `0` (the digit row's own last free key, `Digit8`/
  `Digit9`'s own precedent) plus a MENU row. Reuses `Renderer::
  cycle_magnify_style`, the outdoor game's `Shift+=` sequence, so both games
  step through the identical order.
- `MAGNIFY NOTCH`, `MAGNIFY INK` — MENU rows only, reusing `Renderer::
  cycle_magnify_notch`/`cycle_magnify_ink` (outdoor's `Shift+[`/`Shift+]`).
- `MAGNIFY LEVEL`, `MAGNIFY GRAIN` — MENU rows only, reusing `Renderer::
  cycle_magnify_level`/`cycle_magnify_grain`. **Added to `render.rs` in this
  lane**, `cycle_magnify_ink`'s own shape — neither existed when the style
  lane still owned that file; the coordinator lifted the fence once #352
  merged and closed it, and the addition was small and symmetric enough to
  take rather than ship a three-plus-two split. All five share one mechanism.

Default stays `MagnifyStyle::CellArt` (byte-identical to a build without the
enum) — nothing changes unless the row or key is used.

## What a later lane should not have to re-derive

**`lay_out`'s "508 of 508, zero slack" does not mean the same thing on both
rows.** Slack is redistributed into the gaps *between* groups, so any row
with non-negative slack always reports exactly its own width — only an
*overflow* (`right > 508` at a looser spacing) says a row is out of room.
Row 1 read "508 of 508" at every spacing even before this change and was
never the bottleneck; row 0's overflow at the two looser spacings (`522`,
`512`) was. Read the overflow column, not "fits", to find which row has
room.

## Verification

- `cargo test --lib`: 1,655 passed, 70 ignored, 0 failed (one hard-coded
  button count updated: `every_button_answers_where_it_was_drawn`, 29 → 26).
- `the_bar_fits_the_screen_and_no_two_widgets_overlap` watched red: a
  deliberately oversized `MENU` label overflowed it before the label was
  restored.
- `cargo clippy`, `cargo run --release --example ascii`, `acceptance.sh`,
  `worldgencheck.sh`, `docscheck.sh`: see PR body / commit for final numbers.
- Review cards: see PR body.
