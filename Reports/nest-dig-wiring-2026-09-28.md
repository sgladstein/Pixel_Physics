# Digging concentrates under the nest: two weights on the ant's dig

*Measurement and a default change, 2026-09-28. `engine` / `lab`. Lane note:
[`lanes/nest-mouth.md`](lanes/nest-mouth.md). Follows
[`nest-spoil-footing-2026-09-27.md`](nest-spoil-footing-2026-09-27.md),
whose §7 left the lane on "the digging is scattered".*

## 0. The answer

**The colony digs one concentrated nest instead of a scrape across the
whole box, and the change is two weights in the ant's genome.**

- **Before:** the ant dug whatever ground it faced, wherever it was, at a small
  baseline rate (`(Bias, Dig, 0.15)`).
- **Now:** that baseline is below zero, so an ant on open ground away from
  home does not dig at all. `(SurfaceCurvature, Dig, -1.0)` gives digging back
  where the ground encloses the ant: a tunnel face, a shaft, a pit.
- **At home, nothing changed:** hidden units 5/6 still carry the dig urge to
  about 0.8.

In `digbox` (40 ants, no food, 12 seeds, frame 12,000), against the shipped
genome:

| | shipped | new | seeds better |
|---|---:|---:|---:|
| openings to the surface | 27.5 | **10** | 12 of 12 |
| roofed share of the dug room | 0.58 | **0.78** | 11 of 12 |
| 90th-percentile depth, rows | 7 | **10.5** | 8 of 12 |
| middle-half width, columns | 73.5 | **35.5** | 12 of 12 |
| largest connected piece | 0.16 | **0.33** | 12 of 12 |
| cells dug | 127 | 122 | same digging |
| cuts that built (funnel) | 3.4% | **6.7%** | |

**For the first time the colony's dig beats random digging.** Against the
random walkers from the door:
- roofed share 0.86 and depth 1.00, where it was 0.02 and 1.00;
- mouths 0.39, where it was 0.00;
- combined panel 0.18, where it was 0.00.

It is still some way from the spec, one mouth and chambers, but the digging
is now in one place.

## 1. How the lane got here, including a correction

The strategy given to the owner on 2026-09-27 proposed a **dig marker**, a
digging scent at the face with a saturating response (Toffin 2009). That is
the one mechanism the lane's own biology research lists under *do not
attempt*. `nest-biology-digging-signals-2026-09-19.md` §3.1 reports it was
tested directly in *Acromyrmex lundi*: a fresh face against one an hour old,
no difference. The correction was sent to the owner before anything was
built.

The report's ranked list (§10) put three cues within reach:

- **Fresh spoil draws digging** (rank 4). This was censused first, at the
  decision, not over the box (the funnel's new line, `c4157a8a`):
  - fresh spoil lies within 2 cells of the target on **37% of decisions to
    dig** on the shipped ant, and 71% with `SPOIL_FOOTING=ground`;
  - the control with spoil destroyed on digging reads exactly 0.

  So the cue discriminates. But a rule reading it would stall every tunnel,
  because the face is underground and the pile is at the lip. The biology
  splits the two: piles draw where digging *starts*, and inside the burrow
  the face does the rest (§2.2).
- **Surface curvature** (rank 3). This is already a live sense on the ant,
  already wired to `DropSpoil`. It reads negative in a pit, a shaft or a
  tunnel, and positive on a rise.
- **Digging happens where an ant meets a face** (§2.2). In the engine the
  opposite was true: the baseline fired on open ground.

The cheapest test of the last two together needed no code: `digbox wire=`
against the shipped genome. It did the concentrating on its own, so the
gate switch planned for "outside, dig only beside spoil" was not built.

## 2. The four arms, and the grid

`digbox`, 40 ants, energy 1,000, `w=200 soil=60`, 12 seeds, frame 12,000,
medians; seeds more nest-like against the shipped genome in brackets.

| arm | cells dug | mouths | roofed | depth90 | width (iqr) | largest |
|---|---:|---:|---:|---:|---:|---:|
| shipped | 127 | 27.5 | 0.58 | 7 | 73.5 | 0.16 |
| curvature only (`-1.0`) | 258 | 28.5 (2/10) | 0.74 (11/1) | 15.5 (12/0) | 79 | 0.15 |
| less digging only (bias `-0.3`) | 40 | 10 (12/0) | 0.52 (4/8) | 1.5 (0/12) | 27.5 | 0.31 |
| **both** | 122 | **10 (12/0)** | **0.78 (11/1)** | **10.5 (8/4)** | **35.5 (12/0)** | **0.33 (12/0)** |

**The two weights do different jobs:**
- The bias sets how few mouths there are. Alone it stops the scrape, but the
  colony barely digs (40 cells) and stays shallow.
- The curvature sets how deep. Alone it doubles the room and the depth, but
  on a scrape as wide as before.

Together they give the concentration and the depth for the same digging.

**Not a knife-edge** (curvature -0.5/-1/-2 x bias -0.15/-0.3/-0.5, 12 seeds
each):

| curvature / bias | dug | mouths | roofed | depth90 | built |
|---|---:|---:|---:|---:|---:|
| -0.5 / -0.15 | 100 | 10.5 | 0.75 | 10 | 5.5% |
| -0.5 / -0.3 | 68 | 8.5 | 0.76 | 10 | 3.3% |
| -0.5 / -0.5 | 43 | 10 | 0.55 | 1 | 2.0% |
| -1 / -0.15 | 136 | 12.5 | 0.77 | 11 | 6.2% |
| **-1 / -0.3** | 122 | 10 | 0.78 | 10.5 | 6.7% |
| -1 / -0.5 | 100 | 8.5 | 0.77 | 12.5 | 5.1% |
| -2 / -0.15 | 175 | 16 | 0.79 | 14 | 6.7% |
| -2 / -0.3 | 162 | 12.5 | 0.82 | 18.5 | 6.5% |
| -2 / -0.5 | 163 | 11.5 | 0.83 | 16 | 7.5% |

- Every cell keeps mouths at 8.5–16, against 27.5.
- Every cell but the starved corner keeps the roofed share at 0.75–0.83.
- The shipped point is the middle of the grid, and all four of its
  neighbours stay at 15 mouths or fewer and roofed 0.7 or more.
- A deeper nest is one step away: -2 / -0.3 digs to 18.5 rows at 12.5 mouths.

**The default reproduces the measured arm line for line** (seeds 1–3), since
`wire=` sets the same slots the genome file now carries.

## 3. What it looks like

Seed 1, tinted: tunnel wall cyan, spoil orange, ants magenta, the nest
white. Stops at 3,000, 6,000, 9,000 and 12,000 frames, shipped and new side
by side; the picture was sent to the owner on 2026-09-28.

- **Shipped:** a lined crust scraped across the whole view. Shafts and two
  long diagonal tunnels run off the sides.
- **New:** ground away from the nest untouched to the edges. The digging sits
  under the nest strip: lined galleries, several shafts with the central one
  past the bottom of the view, a sloping tunnel and a side gallery. Spoil is
  heaped on the surface over the nest.

## 4. The colony bed and the lab

Run to tell the foraging lane what moved, not to veto. The owner ruled on
2026-09-27 that colony numbers do not block a step toward the nest.

BED-LAB-PENDING

## 5. What is still missing, and what is next

- **One mouth.** Ten remain, along the nest strip. The biology has no
  measured mechanism for how one entrance arises (§4 of the digging-signals
  report), so this has to come out of concentration rather than out of a
  rule. The candidates, now that digging is concentrated:
  - a narrower home: the `NEST_SITE_COLS` dead end's condition, "after
    something concentrates digging", is met;
  - the founding shaft as the one hole (`NEST_SHAFT`, door + dug mouth);
  - fresh spoil as the cue for where new digging starts.
- **Chambers.** The research ranks contents, brood or a granary, as what
  turns a tunnel into a chamber (§6), and the dig box has none.
- **The footing switch and drop-away**, re-scored on the new dig. Both made
  the old dig stick more, and both opened more holes, because the digging was
  scattered.

## 6. Predictions, written before each batch

| # | arm | prediction | result | right? |
|---|---|---|---|---|
| 43 | fresh-spoil census, shipped | fresh spoil beside 5–30% of decisions | 37% | wrong: higher |
| 44 | + footing | 10–50% | 71% | wrong: higher |
| 45 | spoil destroyed (control) | exactly 0 | 0 | right |
| 46 | every arm | cuts see more spoil than decisions | yes, but for footing + drop-away | mostly right |
| 47 | curvature only | mouths within ±3 | 28.5 | right |
| 48 | bias only | mouths ≤ 22, fewer digs | 10; 40 cells | right |
| 49 | both | as 48, slightly deeper | mouths 10, depth 10.5, roofed 0.78 | better than predicted |
| 50 | grid | mouths fall as the bias falls; curvature alone does not move them | monotone at -1 and -2, not at -0.5 | mostly right |
| 51 | grid | depth rises with the curvature; bias -0.5 starves the digging | depth right; starved only at -0.5 / -0.5 | partly right |
| 52 | grid | no knife-edge around -1 / -0.3 | all four neighbours ≤ 15 mouths, roofed ≥ 0.7 | right |
| 53 | colony bed | starved within ±25 | BED-53 | BED-53R |
| 54 | lab | births no worse than 4 / 8 either way; ≤ 1 extra colony lost | LAB-54 | LAB-54R |
