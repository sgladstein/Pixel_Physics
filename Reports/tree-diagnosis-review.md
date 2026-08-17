# Review: the tree-shape diagnosis does not hold up

Adversarial review of `Reports/tree-shape-problem-statement.md`,
`tree-architecture-research.md`, and the commits around them. Measured
independently, with its own probe, against the same HEAD.

## 1. "Extension is one-shot" is wrong — the binding constraint is the world's top edge

Time series at `ground=96`, 8 trees:

```
frame   cells  tips  canopyTop  sites
 5000    1576    18         35     29
 8000    2861    17          6     31
 9000    3063     4          0      4
11000    3131     0          0      0
20000    3142     0          0      0
```

- **"Stops at ~frame 5,000" is false by 2x.** At 5,000 the stand is at 1,576
  cells and it *doubles* afterward.
- **Tips collapse only when `canopyTop` reaches 0.** Tip count is healthy
  (10–18) for 9,000 frames with no downward trend until the canopy touches
  row 0. Per-tree shoot heights `89, 92, 10, 4, 42, 96, 90, 89` in a
  **96-row sky** — five of eight trees pinned against the world boundary.

**The control settles it.** Same scene, ground at 200:

```
frame   cells  tips  canopyTop
22000   12855    30          7
24000   14294     4          0
30000   14459     0          0
```

**14,459 cells against 3,142 — 4.6x more tissue, and growth runs to frame
24,000 instead of 11,000, purely from adding sky.** Same signature: five
trees pinned at the ceiling.

**It also challenges the sub-critical branching diagnosis**
(`tree-extension-audit.md`). Mean offspring 0.9944 was measured at
`ground=96`, *inside* the ceiling. A tip at row 0 has no upward candidate,
so the boundary **manufactures** tip deaths and drags the ratio under 1. If
sub-criticality were intrinsic, tree size would not scale with available
sky — it scales 4.6x. And at `ground=200` the tip count was at its **run
maximum (30) at frame 22,000**, then fell to 4 by 24,000 as `canopyTop` hit
0 — a cliff at the boundary, not the smooth decay a genuinely sub-critical
process produces.

**Decisive missing measurement: re-run offspring-per-tip at `ground=200`.**
If it rises to ≥1, "sub-critical branching" is the ceiling in disguise.

**This is the third recurrence of a documented failure.**
`tree-architecture-research.md` §6 records it for `forest` at 40 rows.
`grove` at 96 did not fix it — it moved it.

## 2. The whip/blob dichotomy rests on two non-comparable harnesses

| | `plant_probe ground=96` | `filmstrip grove` |
|---|---|---|
| trees | `trees=N`, default 1 | **hard-coded 3** |
| spacing at 8 trees | **56 cells** | **140 cells** |
| seed placement | at `ground-1`, resting | dropped **25 rows** |
| soil depth | 30 rows | 34 rows |

**"Measured, same scene (`grove`, 8 trees, 14,000 frames)" is a run that
cannot exist** — `filmstrip` has no `trees=` argument. Those numbers are
`plant_probe trees=8`. So the headline table is a 56-cell-spacing, 8-tree
scene while the pictures are a 140-cell-spacing, 3-tree scene. That matters
because `38ef499` (crown shyness) is a *spacing* mechanism: tuned at one
stand density, eyeballed at another.

The real grove: **1,701 cells, 3 trees, growth ends ~frame 12,000 with
`canopyTop` at 2** — ceiling-limited too.

**"Whip: tiny (~40 cells)" is wrong by ~12x.** Measured median is **495
cells/tree** (`[408, 711, 44, 15, 160, 793, 513, 495]`). The "~40" is the
*leaf* median. **A fourth metric on this branch reporting something other
than its name.**

**Summed totals hide a bimodal ensemble.** Per-tree sizes span **15 to 793,
a 53x spread**, with 2–3 of 8 trees failing to establish at all. `height`
also conflates canopy and roots (`max_y - min_y` over all cells), which is
why it reads 97 in a 96-row sky.

## 3. The `thicken()` row-total gate has a real and common failure case

`row_width = cells_in_row` counts **every cell the organism has on that
row, regardless of whether they form one stem.**

```
rows occupied by some organism:                          564
rows where the organism has MORE THAN ONE separate run:  297 (53%)
rows where cells_in_row >= 2x the longest real run:      159 (28%)
```

Concrete rows where the gate returns the wrong verdict at `pipe_ratio: 10`:

```
org  y   cells_in_row  longest_run  runs  leaves_above   gate   correct
 2  95         10            5        2         86       8.60    17.20
 2  80          9            3        4         83       9.22    27.67
 2  81          9            4        3         85       9.44    21.25
 6  43         10            5        3         58       5.80    11.60
```

In every one the true cross-section says *thicken* and the row total says
*don't*. **A limb elsewhere on the same row silently suppresses thickening
in the trunk.** Worst observed: 23 cells across 9 separate runs, longest
run 7 — the gate reads a cross-section of 23 for a 7-cell stem.

Two more defects in the same expression:

- **Leaves are counted as stem cross-section.** 10% of cells are `Leaf`,
  each inflating its own row's denominator. Foliage is not xylem.
- **`leaf_count` counts `Leaf | GrowingTip` while `row_width` counts those
  *plus* everything else** — the same cell appears on both sides.

The commit's justification ("the row total cannot under-read") is correct
but answers the wrong objection. It fixed over-reading in a *blob* and
introduced systematic over-reading in a *branched* tree — which is the
shape being aimed for. Per `CLAUDE.md`, a fix that trades one artifact for
another needs a test that catches the trade, and none exists.

## 4. The light fix is unsound, and its justification is contradicted by the repo's own instrument

Measured converged (36,000-frame warm-up), at both phase extremes:

```
noon      y=0 3.0151  y=96 0.6887  y=200 0.3917  y=288 0.2806
midnight  y=0 0.2000  y=96 0.8056  y=200 0.3920  y=288 0.2816
  -> NEVER crosses 0.1 in 300 rows of open air, at either phase
```

**`Germinate`'s 0.1 gate is now unreachable anywhere with sky access.** It
has degraded into a binary "am I sealed in rock" test. Confirmed
behaviourally: 8/8 trees germinated 200 rows down.

**The repo contradicts itself about the old constant.** `LIGHT_DECAY`'s doc
says 0.997 crossed 0.1 "roughly 20 world cells below open sky";
`print_light_versus_depth`'s doc says of the *same* constant "0.16 at depth
128 — below the 0.1 gate by depth ~145". **20 vs 145.** An analytic
solution of the diffusion-with-absorption steady state gives ~132 rows at
noon, agreeing with 145.

**So the premise that motivated the change — "air is attenuating sunlight,
a tree cannot be planted more than a couple of field rows from open sky" —
was measured at a low light phase or on an unconverged field.** At 0.997,
light already reached ~130–145 rows, covering `forest` (40) and `grove`
(96) with room to spare.

Consequently `GROVE_GROUND_Y = 96` is justified by a number that does not
exist, and its own doc violates it — citing a "hard ceiling" of 75 and then
setting 96. **There is no light reason the ground cannot sit at 250.**

### Unconsidered consequences, measured

**Day/night is erased at depth, and the gradient inverts.**

```
y      min      max    swing
0    0.2000   3.0151   2.8151
96   0.6640   0.8271   0.1631   (5.8% of the surface swing)
200  0.3706   0.3818   0.0112   (0.4%)
```

Relaxation time at 0.9997 (~3,300 steps) is comparable to
`DAY_NIGHT_PERIOD_FRAMES` (3,600), so **the deep field never converges — it
is a permanently lagging DC average.** For ~45% of every cycle the profile
*inverts* near the surface: at midnight `y=0` reads 0.2000 while `y=48`
reads 0.9251 — **4.6x brighter 48 rows down.** So `phototropism_dir` points
**downward** across the top ~70 rows for nearly half of every day, and
`light_weight: 0.4` is the second-largest term in the `Grow` blend.

**Caves are now lit.** With a `FIELD_SCALE`-aligned 24-wide shaft:

```
tunnel light at y=196: x=300:0.394  x=352:0.289  x=400:0.216  x=448:0.177
sealed rock (400,100): 0.0000
```

156 rows underground and 150 cells into a tunnel reads **0.18 — above the
germination gate.** Moss's `shade_factor` there reads ~0.91 instead of
~1.0, weakening its shade preference wherever a cave mouth is ≥8 cells.

**Photosynthesis income was multiplied, not just extended.** `MAX_LIGHT` is
4.0 and `Photosynthesize { rate: 0.5 }` gives up to **2.0 carbon/tick
against a `Grow` cost of 0.2**. Every mature cell pins at the 4.0 cap.

**This undermines the problem statement's §4.** §4 treats
"every local signal equalizes at once" as an inherent property of the
substrate, and it is the stated reason both bud break and
`tree-architecture-research.md` §1's self-pruning were set aside. **It is
substantially a consequence of this tuning change.** Under a light field
that actually attenuates, shaded interior cells would not saturate and a
carbon-balance test *would* discriminate. **§4 has not been established.**

**No test catches any of this.** All 385 pass.
`open_sky_reads_brighter_than_a_directly_blocked_cell` probes one field row
down and says so in its own comment; `print_light_versus_depth` is
`#[ignore]`d, runs 4,000 frames from cold, and asserts nothing. This is
`CLAUDE.md`'s "a superseded mechanism's tests keep passing while testing
nothing".

## 5. Not measured, and should have been

1. **Offspring-per-tip with headroom** (`ground=200`) — the one measurement
   that discriminates "sub-critical branching" from "ceiling artifact".
2. **What light a tip actually reads.** `ambient_light_above` samples
   `field_at(x, y-8)` and `rebuild_blocked` blocks a whole 8x8 field block
   if *any* cell in it is plant — so a tip with its own canopy in the block
   above reads ~0. Shading is binary at 8-cell granularity, and nobody has
   measured the distribution of light *as tips see it*.
3. **An A/B of `LIGHT_DECAY` on tree outcomes.** Changed on a depth
   argument; no tree-shape measurement was taken across it.
4. **Anything at a controlled light phase.** Every plant run samples an
   arbitrary phase of a 3,600-frame oscillator whose deep-field lag exceeds
   the period.
5. **Per-tree distributions rather than stand sums.** The 53x spread and
   the 2–3-in-8 establishment failures are invisible in every quoted
   number.
6. **Whether `crowding_weight` does anything** — `max canopy 0.000` at end
   of run, independently confirmed by the audit.

## What survives

- **The two-mechanism framing** (extension makes structure, thickening
  makes mass) is sound and is the most useful thing in the document.
- **Thickening knobs cannot produce structure** — correct.
- **The §7 measurement discipline is right**, and its own fourth item
  ("whether growth is still happening at frame 20,000") is exactly the
  question that, asked with more sky, overturns §3.
- **The `thicken()` change was a genuine improvement** over the two local
  probes it replaced (12,039 → 2,817 wood is real). It is the *residual*
  over-read on branched rows that is wrong, not the direction.
