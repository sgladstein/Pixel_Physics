# M2: the four roles re-fit on rung 2 (2026-10-21)

Plan §6.3 step 1: re-fit each role species on rungs 1-3 in its own station
set. Done here on rung 2 (juvenile growth, 48 wide, 10,000 frames), scored
by `train_plant::score::grow_score` against the shipped species' own size
in the same station. Every run: pop 16, 16 generations, 6 beds a child,
8 genes a mutation, sigma 0.9, a `select=random` run beside it as the null.
Herb, creeper, tree and the shrub's second seed ran on runner threads
(`/mnt/project-files/plant-community/runners/run.sh`, pinned to e45cd5a17
plus this branch's trainer).

| role (stations) | final set, score selection | random selection |
|---|---|---|
| shrub (bright, dim), seed 1 | +0.32 +/-0.08, ahead 16/16 | -0.00 +/-0.15 |
| shrub (bright, dim), seed 2 | +0.50 +/-0.18, ahead 16/16 | -0.17 +/-0.24 |
| tree (bright) | +0.27 +/-0.07, ahead 8/8 | -0.12 +/-0.21 |
| herb (bright) | +0.50 +/-0.08, ahead 8/8 | +0.04 +/-0.02 |
| creeper (bright) | +0.20 +/-0.05, ahead 8/8 | +0.05 +/-0.05 |

Selection works on every role; the random null does not move.

**Over a whole life** (96 wide, 40,000 frames, 12 layouts, base 7,000,000,
rain per column -- `refit_rain_per_column.txt`), median end shoot cells,
shipped -> re-fit:

| | bright | dim lamp (0.4) |
|---|---|---|
| tree | 2,452 -> 5,613 | 1,105 -> 4,978 |
| shrub seed 1 / seed 2 | 744 -> 909 / 872 | 398 -> 771 / 892 |
| herb | 199 -> 1,096 | 201 -> 848 |
| creeper | 95 -> 160 | 74 -> 132 |

Picture: `four-roles-refit-f40000.png` (left of each pair shipped, right
re-fit, three layouts, frame 40,000; tree and herb under the full lamp,
shrub and creeper under the dim one).

**What it says.** The shipped species are badly under-fit to the engine as
it now is: even the random null beat the shipped shrub under the dim lamp
on the first run. And selection on juvenile growth alone makes **every role
bigger everywhere, shade included** -- the tree re-fit grows almost as
large under the dim lamp as under the full one, and the herb, meant as the
short-lived colonist, ends five times its shipped size. Nothing on rung 2
charges for that, so the role trade-offs of plan §4.2 (canopy needs bright
light; the colonist loses any long contest) are not in these files. They
are starting points for rung 4 (each role beside the others), not species
to ship.

## The mister was drenching the training beds

The lab mister placed a rate's cells per drop whatever the box's width
(found by the plant performance thread), so the 48-wide rung-2 bed got
10.7 times the lab's rain per column and the 96-wide life bed 5.3 times.
`lab::rain::RATE_WIDTH` now makes a rate per column; a 512-wide box is
bit-identical. Effect on the training beds (`shipped_rain_per_column.txt`
against `../rungs/`): a rung-2 bed 2.3 -> 1.1 core-s, a life bed 14 -> 7
core-s; the shipped species' sizes barely move (shrub life bright 777 ->
744, tree 2,404 -> 2,452, herb 208 -> 199), which fits the finding that
growth ignores soil water above the germination cliff (`../rungs/`). The
re-fits above, trained in the wet beds, hold their gains in the lab's rain
(`*/check_wet.txt` against `refit_rain_per_column.txt`).

## Cheat watch: one free lever (plan §6.2)

`GrowingTip.photosynthesize_rate` -- what a growing shoot tip fixes on its
own -- rose in all five runs: tree 0.5 -> 2.26, shrub 0.5 -> 1.28 and
0.5 -> 2.25, creeper 0.5 -> 1.54, herb 0.55 -> 0.94. One-gene sweeps
(`plantrain mode=sweep gene=GrowingTip.photosynthesize_rate`, 12 layouts,
base 7,000,000, whole life, peak shoot p50):

| | shipped | 1.0 | 1.5 | 2.25 |
|---|---|---|---|---|
| tree, bright | 2,452 | 3,664 | 3,587 | 3,735 |
| tree, dim | 1,108 | 1,676 | 2,460 | 3,149 |
| shrub, bright | 744 | 745 | 830 | 810 |
| shrub, dim | 398 | 503 | 711 | 593 |

And back the other way, the re-fit with this one gene returned to 0.5:
tree 5,695 -> 4,826 bright, 5,097 -> 2,551 dim; shrub 909 -> 814 bright,
771 -> 594 dim. So it is most of the re-fits' shade gain. Its price is
transpiration (`rate * TRANSPIRATION_PER_RATE`), and a young plant's growth
ignores soil water above the germination cliff (`../rungs/`), so the price
is never paid. Per the plan this is an engine pricing gap, not something to
train around; it went to the soil-water lane (2026-10-21) with the sweep
above as a check for its fix. Retrain once water grades growth.
