# The rack lane — several chambers, and copies you can compare

*Branch `claude/evolution-lab-multiple-chambers-qw1hjz`, PR #193. Owner brief
2026-08-31: multiple test chambers switched between with only the selected one
running; 10-100 copies run headless then compared; and an editable box with a
known price.*

## The finding that reframes the premise

**The engine is deterministic by requirement, so copies made without touching
`LabBox::seed` are bit-identical** — a hundred of them are one sample wearing a
hundred labels. Vary the seed and the problem inverts.

Measured (`labbatch`, shipped bed, 9,000 ticks a copy):

| arm | plants | plant cells | animals | seeds sown |
|---|---|---|---|---|
| 4 copies at one seed | 1.00x | 1.00x | 1.00x | 1.00x |
| 12 copies at 12 seeds | **2.42x** | **2.62x** | **3.12x** | **2.53x** |

**That spread is the bar, not just the good news.** Two chambers differing by 2x
means nothing on its own, because two chambers differing by *nothing at all*
already do. Everything downstream reads an order statistic, and a sweep pairs
its settings on the seed — replicate `j` takes `seed0 + j` regardless of
setting, so setting A's replicate 3 and B's replicate 3 are the same world apart
from the knob.

## What each round overturned

**The determinism gate did not cover organisms.** `tests/determinism.rs` is
sand, water, oil, a fire and a blast; every genome draw in the engine was
outside the only test asserting reproducibility. Now covered, and note the
guard that does *not* work: `organisms_born` counts the founders too, so it is
positive on a bed where nothing ever bred. `plant_generation > 0` is the one
that proves a birth.

**Three placements for the tab strip, one verdict.** Shipped behind an env
switch rather than argued; owner chose *above the bar*. The durable half is in
`dead-ends.md`: the bar has **1 px of slack on row 0 and 0 on row 1**, so any
future bar control is a strip beside it, never a cell in it.

**`every_string_the_bar_can_draw_is_drawable` was blind to the parameters
page.** 42 knobs, each with a hover note, **none covered** — proven by injecting
`~#~` and watching it stay green. It reaches the page through
`params::registry`, not `panel_rows`. Extended, and it found a live defect the
same minute: bed notes had been given markdown emphasis and the 5x7 set has
**no `*`**.

**`widget_rect` could not see the tab strip or the rack page.** It is documented
as what a harness aims a synthetic click with, so a control missing from it is
one no test and no contact sheet can press. `labui` found it by panicking.

## Costs, measured on this build

| knob | 512x320 bed, paired arms | note |
|---|---|---|
| **soil depth 40 -> 160 rows** | 3.04 -> 5.69 ms, **1.87x** | identical stand (820 cells both). The cost is `ca_sweep` (1.19 -> 3.29), i.e. the soil water cycle, **not** the field |
| lamps | free | eight cost what one costs |
| a frozen chamber | **zero** | a `World` owns no threads; only memory |
| a chamber in memory | ~2.5 MB at 512x320, ~10.5 MB at 1024x640 | `Cell` is 12 B and `Pheromones` eagerly allocates `4*w*h` more |
| batch throughput | **8.4 s per 9,000-tick chamber** amortised, 4 cores | `creature_space mode=threads` measured 3.27x on 4 |

## The width question, settled on a quiet box (2026-09-01)

Both earlier readings were wrong, and they were wrong in opposite directions.
`instruments.md` said *"a wider box is **cheaper** at fixed founders"*; this
note said *"sublinear, then flat"* (1.68x, then **1.00x**) and refused to quote
it because 6.386 against 6.395 is exactly the tidiness `CLAUDE.md` says to
distrust. It was right to refuse: the flat second doubling was the loaded box
and does not survive a quiet one.

Re-measured with the container freshly booted at 0.03 load per core, three legs,
`arms=lab`, 6,000 frames, `RAYON_NUM_THREADS=4`, median of **three alternating
rounds** so a slow patch of machine hits every arm rather than one:

| leg | 256 | 512 | 1024 | |
|---|---|---|---|---|
| **empty** (`trees=0`) | 0.001 ms | 0.001 ms | 0.001 ms | **1.00x over 4x width**, `solved/f` 0.0 |
| **stocked** (`trees=16`) | 3.575 ms | 5.365 ms | 6.444 ms | 1.50x, then 1.20x |
| its stand | 5,411 cells | 8,861 | 10,483 | **1.94x more plant off the same 16 founders** |
| **per 1,000 plant cells** | 0.661 ms | 0.605 | 0.615 | flat |

**So: width itself is free, and what it costs is what grows in it.** The
absolute frame gets *more* expensive with width, not less — but only because
room fills with life. `PlantScene` spaces founders `width / (trees + 1)` apart
(15 cells at 256, 60 at 1024), so **a narrow box is a crowded box**, and
"at fixed founders" is not "at a fixed stand". `LabBox::spread` divides its span
the same way, so the confound belongs to the phrase rather than to one harness.
That is what the original "cheaper" was seeing, one normalisation short.

**The positive control is what makes those nulls results rather than
blindness**: soil 40 -> 240 at one width moved **1.43x**, against a run-to-run
spread of 1.6-5% on the arms themselves. Worst frames ran 7.7-59.4 ms against
means of 3.5-6.6, and `mean x frames` exceeds the worst by ~3 orders of
magnitude, so **no worst-frame figure here is pinned by its aggregate** — read
the means.

## What that cross-check turned up instead: a colony eats its own neighbourhood

Running the same width axis on the **real** `LabBox` (sealed, with its ceiling)
rather than `PlantScene` produced no cost curve at all, because the three widths
are not three prices — **they are three different biospheres**, and the 256-wide
one ends *dead*. Which is a scene error wearing a result, until you ask what
killed it.

`colonies=0` against `colonies=1`, six seeds, **paired on the seed** (replicate
`j` takes `seed0 + j` whatever the setting, so both arms are the same world
apart from the colony), 6,000 ticks, `founders=8`:

| width | plants, no colony | with one colony | stand left standing | seeds down | the colony itself |
|---|---|---|---|---|---|
| 256 | 76 (55-104) | **1** (0-15) | **1%** | **6 of 6** | 14 ants |
| 512 | 163 (127-229) | 66 (47-108) | 41% | 6 of 6 | 20 ants |
| 1024 | 50 (28-72) | 50 (23-75) | **98%** | 4 of 6 | **2 ants** |

**Monotone across a 4x width range, and the endpoints are far outside the
2.42x seed-noise bar this lane measured** — 1% against 98% is not a sample from
it. This is the finding another lane relayed (*"the colony crashes because it
eats its own neighbourhood, not because the box is poor"*, founder survival
monotone in distance from the nest) **reproduced as the spatial result they said
a rack could get and one bed could not** — their axis was distance within one
bed, this one is how much bed there is per colony.

**A narrow bed feeds the ants and kills the plants; a wide one does the
reverse** — at 1024 the plants are untouched and the colony falls to two
animals because it cannot reach them. **512, the shipped width, is the only one
of the three where both persist.** That is a coexistence window rather than a
tuned default, and it is not claimed to be the best one: three widths is not a
sweep of width, and nothing here says 512 is where the window is widest.

Written into the `colonies` hover note, because a player whose stocked bed
empties needs it at the moment they are setting the box up.

## The box you can resize, and the wall's button (2026-09-01)

Both closed by the owner, and one of them was a judgement I had got wrong.

**`width` and `height` are rows on the parameters page now.** I had recorded
this as *"a feature nobody has asked for"* and declined it — wrong on the
facts: resizing the chamber is squarely inside the owner's original third
idea, *"evaluate if we can modify our test chamber and how much it will
impact performance"*. Having spent a session measuring exactly that axis and
concluding **width is free**, leaving no way to use the finding was the gap,
not the restraint. Owner asked directly; built.

- **`128..4096` per side, step 64**, so every width this lane measured lands
  on the grid. The ceiling is a **memory** decision (the owner's): a chamber
  costs `w * h * 16 B` — 12 for the grid, 4 for the two pheromone planes — so
  512x320 is 2.5 MB and 4096x4096 is ~268 MB, and a rack holds one each.
- **Both size rows print what the current box actually costs**, computed
  through `batch::BatchSpec::world_bytes` rather than a literal, so the
  figure is this box's. Nothing in the engine refuses a box too large to
  hold, so the number has to be on screen *before* `REBUILD` — the only
  moment it can still be reconsidered.
- **`ground_y` rides the height**, by ratio so it is idempotent under a
  sweep. This is `lab_resolution`'s trap closed before it could be met: left
  at 160 in a 640-row box the soil sits in the top quarter and 390 rows are
  void. That was a harness-only footgun; a knob makes it the player's first
  press. `ground_y` stays its own row for anyone who wants to override it.

**`Tool::Wall` has a bar cell.** `KEEP` and `FREE` came off the bar, the
owner called it, and the wall became the seventh tool.

**But the bar is full again at seven, which is measured and not what the
arithmetic suggests.** "Two cells freed, one spent" reads as a cell left
over. There is none: putting an *eighth* back fails
`the_bar_fits_the_screen_and_no_two_widgets_overlap` exactly as a ninth did
before — the freed width did not all go to the tool row, and `WALL` is not
the width of the `KEEP` it replaced. With the wall on, `PIXEL_PHYSICS_BAR_TRACE`
reports the natural spacing **overflowing row 0 by 8 px**, and the layout
falling back to its tightest rung to fit at **506 of 508** on row 0 and
**508 of 508** on row 1. So the standing rule is unchanged: run the fit guard
before assuming the next lab control has anywhere to live, and expect a no.

I nearly shipped the opposite claim — the doc comment said "7 of the 8 that
once fitted" until putting an eighth back proved it false.

## Still open

- **Where the coexistence window actually is.** Three widths at one founder
  count found it; locating it wants width and `founders` swept together, paired
  on the seed, which the rack now runs unaided — and `width` being a
  `write_bed` field means that sweep needs no new code.
- **Nothing bounds a rack's total memory.** One chamber prints its own cost;
  the rack does not sum them, and at 4096 a handful of chambers is a
  gigabyte. The batch has a byte budget and drops worlds to on-record rows;
  the *rack* has no equivalent.

## IN FLIGHT — handoff, 2026-09-01

**Branch `claude/evolution-lab-multiple-chambers-qw1hjz`, pushed, no PR yet.**
The owner asked what a hundred copies would look like on the rack page. That
question found one shipped bug and three gaps.

### Done

- **The rack could not be scrolled at all.** `rack_scroll` was written,
  clamped and honoured by the renderer from the day the page landed, with
  **nothing bound to move it** — no key, no click, no `Action`. A rack of a
  hundred showed rows 1-12 for ever, and every guard over it passed. Fixed
  with a pager mirroring the parameters page. `a_rack_taller_than_the_page_
  can_be_scrolled` asserts both halves separately — the control *exists*, and
  it *changes which rows are drawn* — and was proven red against the original
  no-op.
- **A `SET` column**, so a sweep's rows say which setting they ran at.
  `Chamber`, `OnRecord` and `ChamberSummary` carry `setting: Option<f32>`,
  threaded from `RunResult.setting` through `adopt_chamber`. **Column indices
  shifted**: PLT is now 3, not 2. The sort guard asserts
  `RACK_COLS[3].0 == "PLT"` so the next insert breaks loudly.
- **Tick progress, both readings.** `Shared.ticks` (aggregate) and
  `Shared.live` (per-run), published on the cancel check's existing modulo so
  the hot path pays nothing extra. The bar leads with ticks —
  `40% -- 180000/450000 TICKS  0/50 DONE ...` — hoisted into
  `batch_progress_line` so it is asserted rather than photographed.
- **In-flight copies are rack rows**, marked `RUNNING`, appended *after*
  `on_record` so `rebuild_record`'s index arithmetic is untouched.
- **`GROUP`** collapses a sweep to one row per setting: median on top,
  low-high underneath. Never a mean — `rack_groups` + `stats::Spread`.
- Guards: `the_rack_page_stays_on_the_screen` (the page grew a pager and
  nothing checked the sum) and `the_batch_line_leads_with_ticks`.

### Next, in the owner's priority order

1. **Enter an experiment from the rack.** Owner, 2026-09-01: *"You can only
   enter them from the tabs in the main menu, but that only works for the 1st
   four. There is currently no way to enter the others."* A row click is
   `Action::ChamberSelect` (highlight + still); check whether any ENTER verb
   reaches `Action::Chamber(i)` from the page at all. **Land this first — it
   is what makes the rack usable.**
2. **Type a number into the copies/ticks dials.** Clicking `+` to reach
   200,000 ticks is unusable. Look for existing text entry on the parameters
   page (`field_text` and the `saving_refuses_*` guards suggest editing
   machinery) and reuse it rather than building a second one.
3. Cosmetic: the pager steps by `RACK_ROWS` while a grouped row is two lines,
   so a grouped page scrolls half a screen.

### Gates at handoff

`cargo test --release --lib lab::` **114 passed / 0 failed**. The full suite
and clippy have NOT been re-run since `GROUP` landed — run both before opening
a PR.
