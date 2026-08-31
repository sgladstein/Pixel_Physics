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

## One number that did NOT reproduce — do not quote either version yet

`instruments.md` records *"a wider box is **cheaper** at fixed founders — world
size is not a term"*. Measured here at fixed founders, `arms=lab`, 1,000 frames,
`RAYON_NUM_THREADS=2`:

| width | mean | solved/f | stand |
|---|---|---|---|
| 256 | 3.79 ms | 13.2 | 820 |
| 512 | 6.39 ms | 26.2 | 821 |
| 1024 | 6.40 ms | 49.9 | 735 |

So **not cheaper — sublinear, then flat**: 1.68x for the first doubling and
1.00x for the second, while the solve set doubles at every step.

**This is reported rather than acted on, and the reason is `CLAUDE.md`'s own
tidiness tell.** 6.386 against 6.395 is a 0.14% agreement on a box with four
cores, other agents on it, and worst frames of 10.9 / 60.6 / 38.8 ms in the
three arms. A clean result on a chaotic quantity is evidence of an artifact
before it is evidence of an effect. **Neither the recorded claim nor this one
should be quoted to a player until somebody runs it on a quiet box** — which is
why no width price went into the parameters panel.

## Still open

- **`REBUILD` for on-record rows.** A run whose world was dropped for the memory
  budget is listed with its census and its verbs drawn dead. The spec
  reproduces it exactly; the button that does so is not built.
- **`compartments` as a verb rather than a rebuild knob.** A partition is a
  stone column, so "drop a wall here" is a brush. It would need
  `LabBox::extra_walls` so a hand-placed wall survives the rebuild every other
  bed knob triggers — `partition_columns()` stays the one place wall positions
  are decided.
- **The width question above.**
