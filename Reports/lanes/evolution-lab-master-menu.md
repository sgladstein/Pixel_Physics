# Lane note — a master menu (round 30, R2)

Owner, 2026-09-12: *"There are lots of hidden menus that can only be accessed
by knowing the F key. There should be a master menu accessible from the main
UI that leads to all the other menus."* Plus: *"The menu could be revamped
too."*

## What changed

`Panel::Menu` (`src/lab/ui.rs`), opened by a `MENU` bar chip and by `F6`. It
lists every page in the lab and every view toggle, one `Row::choice` each:
the destination on the left, the key that also reaches it (or the live
value, for the two controls with no other readout — see below) on the
right. Every row fires the exact `Action` its key already fired — nothing
here is a second definition of a control.

**Re-derived page list** (the brief's own table was stale —
`PlantList`/`AntList`/`Log`/`Scenarios`/`Compare` already had mouse routes,
just none from one place): `Plants`(F1), `Ants`(F2), `Box`(F3), `Params`(P),
`Shelf`(G), `Chambers`(F4), `History`(F5), plus `PlantList`/`AntList`/
`Log`/`Scenarios`/`Compare` — a row on some other page each, no key.

## The bar: consolidation, not addition

`PIXEL_PHYSICS_BAR_TRACE=1` before this change: row 0 and row 1 both **508
of 508, zero slack**, at the tightest of the three `SPACINGS` — the bar
takes nothing more without removing something first (`Reports/dead-ends.md`,
the `SHELF` entry, already established this for row 0).

**`MENU` replaces `PLANTS`/`ANTS`/`BOX`** on row 1 (three pure-navigation
chips folded into one) **and `PARAMS` moves off row 0 into a MENU row** (`P`
still opens it directly). Both were pure navigation — no chip here ever
carried live state the way the jar chip does, which is why the jar chip
stayed and these didn't (`SHELF` entry, same reasoning, other direction).

Measured after: **row 0 and row 1 both fit at 508 of 508 with zero overflow
at every one of the three `SPACINGS`, including the loosest** (`pad=2
gap=2`) — not merely passing, real headroom for the first time since
`SHELF` landed. Button count **29 → 26**
(`every_button_answers_where_it_was_drawn`, updated in the same change).

**A bare-add alternative was built and measured, not argued** — see
`dead-ends.md`: adding `MENU` on top of everything else (no removal) also
passes `fits()`, but only at the single tightest spacing, both rows at
exactly 0 slack — worse than consolidation, comfortable at every spacing
tried. Rejected on that measurement.

## Two verbs that had no button at all now have one

`Digit9` (write the chronicle) and `KeyF` (`cycle_display_floor`) were direct
calls to `Lab`/`TimeControl` methods specifically because — their own source
comments said so — *"this one has no button."* Now one exists (`SAVE
CHRONICLE NOW`, `DISPLAY FLOOR` on the MENU page), so both keys route through
`Lab::act` as `Action::WriteChronicle`/`Action::CycleDisplayFloor`, matching
the rule the comments themselves stated: `Lab::act` dispatches a verb a
button also draws.

## The display-floor handoff (lane R1)

R1 left `MIN {}HZ` as a temporary second line in the top-left corner
(`TimeControl::readout`), explicitly for this lane to re-home — it was the
one line R1 could not delete because no other readout of `F`'s setting
existed anywhere. It now has one: the MENU page's `DISPLAY FLOOR` row shows
the live value (`MIN {}HZ`) and its note names the key. The corner's second
line is removed (`time.rs`); ticks alone remain, per the owner's other
2026-09-12 instruction.

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
*overflow* (`right > 508` at a looser spacing) says a row is actually out of
room. Row 1 read "508 of 508" at every spacing even before this change and
was never the bottleneck; row 0's overflow at the two looser spacings
(`522`, `512`) was. Read the overflow column, not "fits", to find which row
has room.

## Verification

- `cargo test --lib`: 1,655 passed, 70 ignored, 0 failed (one hard-coded
  button count updated: `every_button_answers_where_it_was_drawn`, 29 → 26).
- `the_bar_fits_the_screen_and_no_two_widgets_overlap` watched red: a
  deliberately oversized `MENU` label overflowed it before the label was
  restored.
- `cargo clippy`, `cargo run --release --example ascii`, `acceptance.sh`,
  `worldgencheck.sh`, `docscheck.sh`: see PR body / commit for final numbers.
- Review cards: see PR body.
