# Would "loose soil lets air through" give ants a usable nest-air signal?

2026-10-06. Offline Python only: no engine changes, no builds. Every number is labelled
**MEASURED** (computed here, on the shared baseline maps `deep-trace/baseline/3ba1e7bd5`) or
**INFERRED** (a reading of measured numbers, or an assumption). Medians [range] are over the
maps where the gas gets out: 7 maps for R0, all 11 for every other rule, unless a row says
otherwise.

## Bottom line

- **Mound porosity (R1) does not change the nest's slope. It only lowers the nest's level.**
  (MEASURED, `p1e_shift.py`.) Under R1 the nest field equals R0's field minus a constant: the
  constant is 739-4,623 source units, and its spread across the nest is 0.00-0.06% of R0's nest
  span. Reading R0's field with that constant subtracted gives the same "reads and points out"
  share as the solved R1 field (at most 2.3 points apart on any map). Every readability gain
  below comes from the relative sensor's denominator getting smaller.
- **Readable, and pointing out, rises with conductance:** 8.5% at R0, then 16.0 / 21.6 / 27.4 /
  43.0% at eps 0.02 / 0.05 / 0.1 / 0.3 (MEASURED).
- **Following the signal still fails.**
  - With a 1-cell step and a 1% threshold, 94-99.6% of nest cells (medians over maps) never
    take a step. The share
    reaching the sky is 0.2-3.4% (MEASURED).
  - An ant stepping on its own 6-cell sensor reaches the sky from 0.0% of nest cells, under
    every rule and threshold (MEASURED, extra).
- **The exact slope leads out of the nest, but not always to the sky.** On 3 to 6 of the 11 maps
  (depending on eps), almost every nest cell's exact descent ends at one covered pocket in the
  mound, 13-15 walking steps short of the sky (MEASURED).
- **Letting the ground leak as well (R2) makes things worse.** Readability does not improve at
  eps up to 0.1. 59-83% of exact descents end at local minima inside the nest, and the slope's
  shape changes in the harmful direction (MEASURED).
- **The level becomes steadier with R1.** At eps 0.1 there are no refresh-to-refresh jumps over
  25% (R0 had 19 of 43), but the level is still mostly a head-count of ants beyond the sensor's
  reach: 82-89% of a room cell's level comes from more than 6 cells away (MEASURED).
- **A gas stepped at the trail planes' rate would never settle in game time.** The nest takes
  1.2-2.2M frames under R1 and 4.5-5.1M under R0 to come within 10% of the solved field. That is
  4-17x the whole 300k-frame run (MEASURED operator; the rate is ASSUMED).

## Setup (what was solved)

- **Maps:** the 11 maps with a way out for the walk (`expH.MAPS`).
  - Sources are ants 1 and brood 0.25, averaged over 11 maps at f-5,000..f+5,000 (`helpers.srcs`).
  - The sink is open sky with nothing over it (`helpers.masks_v 'orig'`), held at 0.
  - The steady state of div(k grad c) + s = 0 is solved directly (sparse LU).
- **Conductance:** each cell has a conductance k: open 1, porous eps, packed eps/20, sealed 0. A
  pair of cells passes w x min(k_i, k_j), with w = 1 across a face and 0.5 across a corner in the
  8-neighbour runs. This is expH's rule exactly; the solver is checked against
  `expH.solve_p` and `helpers.solve_v` below.
- **R0:** all solid sealed.
- **R1:** 's' above the ground line passes eps.
- **R2:** loose spoil passes eps and every other solid passes eps/20.
  - **INFERRED classification:** the map writes every Powder material as 's'
    (`examples/deeptrace.rs` `write_map`). So natural `soil`, gallery-wall `packedsoil` and
    dumped `spoil` cannot be told apart. Below the ground line there are 107,218 's' cells and
    zero '#' cells.
  - So "loose spoil" is taken to be all 's' above the ground line, plus any 's' at or below it
    that was open in an earlier snapshot of the same seed (dug, then refilled). That finds only
    **0-6 cells per map**, so R2 is in effect R1 plus packed ground at eps/20.
  - R2all (a bracket, not requested): every 's' cell passes eps.
- **The sensor** is helpers.reads: a sample 6 cells ahead (4 on a diagonal), which must be open
  and hold a value; contrast is |f-c|/(f+c).
  - "Points out" means the sample is lower and fewer walking steps from the sky (8-neighbour
    BFS).
  - The followers are expF part 2, vectorised: steepest 1-cell walkable descent, stepping when
    (a-b)/(a+b) >= threshold.

## Controls (MEASURED; `p0_controls.out`, `p0b_specificity.out`)

| Control | Result |
|---|---|
| R0 reproduces the check | `deps/expH.py` rerun as-is: 7/11 maps open, span 6.0% [2.9-8.8], readable 9.1% [3.3-16.3], readable and out 8.5% [2.2-14.5]. Same through this pipeline. **Gate passed.** |
| Solver = reviewer's solvers | max relative difference 0.0 against `solve_p` (eps 0, 1e-6, 0.02, 0.05, 0.1, 0.3) and `solve_v` (diagonal 0 and 0.5), identical NaN pattern, all 11 maps. Not blind: eps 0.1 against 0.1001 differs by 2.6e-4 |
| Pipeline = reviewer's numbers | expF (2) followers exact (99.8/0.5, 99.8/0.7, 99.6/0.0, 99.5/0.0); expF (3) line-of-sight counts exact (54/174, 38/161, 40/122, 33/59); expB census rows exact with its own random picks; expB stability lines exact (table 4) |
| Toy porous lid: sealed 10x10 room, only exit a 3-row lid at eps 0.1, source 1 per cell | Solved equals analytic in every cell: lid 100/200/300, room top 400, floor 445. Highest at the floor in every column, strictly falling toward the lid, span 10.1%, steepest descent points to the lid in 100% of room cells |
| Eigen control (control.py's shaft) | lowest eigenvalue 1.504095e-3 = 4 sin^2(pi/162), relative error 2.5e-14. tau_1 = 95,739 frames; 10% settle ~223k frames, against the stepped ~222k |
| **Specificity: R1 at eps 1e-6 against R0** | **Does not match to rounding, and the cause is physical.** On the 7 R0-open maps the nest level moves by 0.6-18% (s2@100k worst), and readable moves by up to 0.9 points. The cause is gas made by ants in mound pockets that R0 calls sealed (3.9-83.9 source units, against 460-644 connected). At any eps > 0 that gas must leave through the spoil, partly via the nest. With those sources zeroed, 1e-6 matches R0 to <=3.6e-4 relative and every metric to 0.01 points. So eps -> 0+ is a singular limit: **R0 quietly deletes the gas of ants in sealed mound pockets.** On the 4 R0-sealed maps, 1e-6 opens the nest at a flat ~1e7 (span 0.001%, follower 0%) |

## Table 1: readability (items 1-2; MEASURED; `p1_summary.txt`)

| Rule | eps | Gas gets out | Room span | Reads >=1% | Reads and points out | Reads through soil | Reads, points out, clear line (extra) |
|---|---|---|---|---|---|---|---|
| R0 | 0 | 7/11 | 6.0% [2.9-8.8] | 9.1% [3.3-16.3] | 8.5% [2.2-14.5] | 32.8% [23.6-63.9] | 5.5% [1.0-10.7] |
| R0, 8-nbr (extra) | 0 | 11/11 | 13.3% [7.1-21.2] | 14.3% [9.8-24.9] | 13.2% [9.5-23.8] | 20.9% [12.3-32.0] | 11.5% [7.6-21.5] |
| R1 | 0.02 | 11/11 | 12.6% [8.9-17.3] | 16.8% [8.7-23.1] | 16.0% [8.0-22.5] | 21.1% [16.3-29.9] | 13.7% [5.6-18.6] |
| R1 | 0.05 | 11/11 | 19.3% [14.0-22.6] | 22.3% [11.9-30.9] | 21.6% [11.1-29.2] | 13.9% [11.1-20.9] | 20.1% [9.1-26.9] |
| R1 | 0.1 | 11/11 | 24.2% [18.8-27.2] | 28.2% [15.6-39.2] | 27.4% [14.9-37.9] | 10.1% [8.6-15.0] | 26.0% [13.3-36.3] |
| R1 | 0.3 | 11/11 | 33.2% [28.4-35.1] | 46.4% [27.5-61.7] | 43.0% [26.5-57.1] | 6.9% [5.0-8.1] | 41.5% [25.2-55.0] |
| R1, 8-nbr | 0.1 | 11/11 | 25.7% [22.7-29.4] | 28.9% [19.8-43.2] | 28.2% [19.0-42.0] | 9.1% [8.1-11.2] | 26.8% [17.8-40.6] |
| R2 | 0.02 | 11/11 | 13.6% [8.5-18.9] | 16.0% [8.4-24.0] | 15.2% [7.7-22.0] | 22.8% [16.7-31.4] | 12.9% [5.3-18.1] |
| R2 | 0.05 | 11/11 | 18.4% [13.0-21.7] | 21.1% [10.9-30.4] | 20.3% [10.1-28.9] | 15.9% [13.1-23.1] | 18.4% [8.1-25.7] |
| R2 | 0.1 | 11/11 | 22.9% [17.1-25.5] | 31.0% [14.0-42.5] | 27.3% [13.2-37.7] | 13.3% [9.9-17.5] | 24.7% [11.3-35.8] |
| R2 | 0.3 | 11/11 | 30.5% [25.0-32.0] | 78.5% [67.2-84.3] | 50.4% [40.7-59.7] | 6.6% [4.7-9.7] | 47.9% [38.6-57.1] |
| R2all (bracket) | 0.02 / 0.05 / 0.1 / 0.3 | 11/11 | 11.7 / 21.0 / 29.4 / 41.7% | 25.1 / 57.7 / 75.1 / 99.6% | 9.9 / 19.7 / 30.6 / 43.5% | 25.9 / 11.3 / 6.8 / 4.9% | 6.0 / 17.2 / 28.7 / 41.2% |

What each column means:
- *Gas gets out*: maps where any nest cell is connected to the sink.
- *Room span*: (max-min)/max of the level over walk-reachable nest cells.
- *Reads >=1%*: share of nest cells where some heading's 6-cell contrast is at least 1%.
- *Reads and points out*: that heading's sample is lower and closer to the sky by walking.
- *Reads through soil*: per map, the share of >=1% (cell, heading) reads whose straight line to
  the sample crosses a soil cell (expF part 3). R0's reads pooled: 285/823.
- *Clear line*: "reads and points out", using only reads with an open line of sight.
- *Sealed nest cells*: 0.2% [0.0-0.6] for R0 and R1, 0.0% for R2 and every 8-neighbour run.
- *Paired over the 7 R0-open maps*: "reads and points out" goes up on 7/7 maps, by a median of
  +7.5 / +20.2 / +36.2 points at R1 0.02 / 0.1 / 0.3.

## Table 2: following the slope (item 3; MEASURED; `p1_summary.txt`, `p1b_terminals.txt`, `p1c_sensor_follower.txt`)

| Rule | eps | 1-cell follower reaches sky at 0.5% / 1% / 2% | Exact descent reaches sky | Exact descent ends in one covered mound pocket | Exact descent ends inside the nest | 1% follower never moves | 6-cell sensor follower reaches sky (extra) |
|---|---|---|---|---|---|---|---|
| R0 | 0 | 0.9 [0-2.9] / 0.4 [0-0.7] / 0.0 [0-0.2] | 99.4% [98.6-99.8] | 0.0% | 0.0% | 99.6% | 0.0% (max 2.9%) |
| R0, 8-nbr (extra) | 0 | 3.9 [2.2-7.8] / 1.0 [0-4.1] / 0.0 [0-0.8] | 99.7% | 0.0% | 0.0% | 99.0% | 0.0% (max 4.6%) |
| R1 | 0.02 | 4.0 [0-6.9] / 0.2 [0-2.2] / 0.0 [0-0.7] | 99.2% [0.0-99.8] | 3 of 11 maps | 0.0% | 98.3% | 0.0% (max 0.0%) |
| R1 | 0.05 | 4.8 [0-8.5] / 0.3 [0-4.9] / 0.0 [0-1.1] | 98.6% [0.0-99.4] | 5 of 11 maps | 0.0% | 96.1% | 0.0% (max 0.0%) |
| R1 | 0.1 | 5.8 [0-10.5] / 3.4 [0-6.2] / 0.0 [0-1.6] | 98.6% [0.0-99.4] | 5 of 11 maps | 0.0% | 95.5% | 0.0% (max 0.0%) |
| R1 | 0.3 | 0.4 [0-14.8] / 0.4 [0-8.0] / 0.2 [0-4.7] | 0.4% [0.0-99.4] | 6 of 11 maps | 0.0% | 94.3% | 0.0% (max 0.0%) |
| R1, 8-nbr | 0.1 | 6.0 [0-11.8] / 3.5 [0-6.5] / 0.9 [0-3.5] | 99.5% [0.0-99.7] | 4 of 11 maps | 0.0% | 95.2% | 0.0% (max 0.0%) |
| R2 | 0.02 | 4.0 [0-6.9] / 0.0 [0-2.2] / 0.0 [0-0.7] | 40.6% [0.0-52.8] | 0.4% [0-35.1] | 58.6% [46.5-73.1] | 98.2% | 0.0% (max 0.0%) |
| R2 | 0.05 | 4.5 [0-8.1] / 0.3 [0-4.7] / 0.0 [0-1.1] | 29.6% [0.0-43.9] | 0.5% [0-36.7] | 69.3% [54.8-77.9] | 96.4% | 0.0% (max 0.0%) |
| R2 | 0.1 | 4.9 [0-9.7] / 3.1 [0-6.0] / 0.0 [0-1.4] | 22.5% [0.0-36.9] | 0.5% [0-31.7] | 74.5% [61.7-81.9] | 95.6% | 0.0% (max 0.0%) |
| R2 | 0.3 | 5.5 [0-13.0] / 3.5 [0-7.1] / 0.3 [0-2.9] | 12.3% [0.0-24.6] | 0.5% [0-22.3] | 82.9% [73.6-88.7] | 95.1% | 0.0% (max 0.0%) |

What each column means:
- *1-cell follower*: share of all walk-reachable nest cells whose steepest 1-cell walkable
  descent reaches open sky when each step needs (a-b)/(a+b) >= threshold. expF counted only
  cells with a value; the two counts differ only on R0's few sealed cells.
- *Exact descent*: the same follower with any drop accepted, i.e. where the slope leads.
- *Covered mound pocket*:
  - On R1 this is bimodal by map: on the affected maps 99%+ of nest cells end at one covered
    open mound cell. That cell is a median 13-15 walking steps (range 6-31) from the sky,
    where the gas leaves through the spoil roof.
  - Affected maps at eps 0.02: s1@200k, s3@295k, s4@295k. From 0.05: also s1@295k and s3@100k.
    At 0.3: also s3@200k.
- *Inside the nest*: R2's exact descent ends at 6-8 local minima per map. They are a median 43
  walking steps from the sky, against 33 at the median starting cell, so the slope leads
  further in, toward shallow ceilings.
- *Never moves*: share of nest cells where no 1-cell drop clears 1%.
- *6-cell sensor follower* (extra): step one cell toward the walkable heading whose 6-cell
  sample is lowest, if that contrast clears the threshold. The cell reported is the 0.5% one;
  1% and 2% are also 0.0%. Positive control: the shaft is reached from 100% at 0.5% and 1%.
  Under R1 these followers climb to 3-6 rows under the ground line and stop there.
- *Extra, p1d*: the exact steepest descent over all neighbours points into soil for 0.0% of
  nest cells under R0 and R1, and for 20.6-23.6% under R2.

## Table 3: census and depth (items 4-5; MEASURED; `p1_summary.txt`)

| Rule | eps | Door level / top level | Share of a room cell's level from sources within 6 cells | ...within 20 cells | Share of the top level made by ants above ground | Spearman(level, rows below ground) | Spearman(level, walking steps out) |
|---|---|---|---|---|---|---|---|
| R0 | 0 | 94.2% [91.7-97.7] | 11.1% [9.1-19.6] | 84.0% | 39.3% [31.6-65.2] | 0.95 [0.92-0.97] | 0.88 [0.77-0.89] |
| R0, 8-nbr (extra) | 0 | 87.9% [81.1-93.3] | 13.5% [6.6-19.0] | 78.0% | 35.5% [22.8-53.4] | 0.95 | 0.86 |
| R1 | 0.02 | 87.8% [85.2-92.3] | 13.3% [6.9-20.2] | 85.5% | 32.9% [25.7-44.0] | 0.95 [0.92-0.97] | 0.86 [0.77-0.89] |
| R1 | 0.05 | 82.9% [80.2-87.9] | 14.7% [7.4-22.6] | 86.5% | 28.5% [20.1-35.8] | 0.95 | 0.86 |
| R1 | 0.1 | 79.4% [75.3-83.7] | 15.9% [7.9-24.5] | 89.0% | 22.5% [16.1-29.5] | 0.95 | 0.86 |
| R1 | 0.3 | 71.8% [66.5-75.3] | 17.9% [8.7-27.4] | 89.2% | 14.8% [10.7-20.5] | 0.95 | 0.86 |
| R1, 8-nbr | 0.1 | 76.0% [72.8-79.7] | 16.0% [7.9-24.8] | 89.0% | 22.3% [14.8-27.3] | 0.95 | 0.86 |
| R2 | 0.02 | 88.0% [85.6-92.6] | 13.3% [6.9-20.1] | 85.3% | 33.3% [26.0-44.4] | 0.96 [0.94-0.98] | 0.79 [0.63-0.86] |
| R2 | 0.05 | 83.9% [81.2-88.7] | 14.6% [7.3-22.3] | 85.9% | 28.8% [20.8-36.7] | 0.97 [0.95-0.99] | 0.75 [0.57-0.84] |
| R2 | 0.1 | 81.0% [77.1-85.1] | 15.8% [7.8-24.1] | 88.3% | 24.0% [17.1-31.1] | 0.97 [0.96-0.99] | 0.73 [0.52-0.82] |
| R2 | 0.3 | 73.7% [69.3-78.0] | 18.0% [8.6-26.7] | 88.6% | 17.0% [12.8-23.7] | 0.97 [0.95-0.98] | 0.66 [0.48-0.76] |

What each column means:
- *Door/top*: the level at the nest cell nearest the sky by walking, over the highest nest
  level.
- *Within 6 / 20 cells*: per map, the median over 25 random room cells of the share of that
  cell's level supplied by sources within that chessboard distance (Green's-function rows, as
  in expB).
  - The 25 cells are picked per map from a fixed seed, so the same cells are used for every
    rule.
  - With expB's own picks, R0 gives 8.1-16.8% (median 12.3%). The 25-cell sample moves this
    number by a few points.
- *Ants above ground*: share of the top cell's level supplied by sources at or above the ground
  line (superposition).
- *Paired over the 7 R0-open maps*:
  - The within-6 share rises on 7/7 maps, by +1.3 / +4.8 / +6.9 points at R1 0.02 / 0.1 / 0.3.
  - Door/top falls on 7/7, by -7.8 / -16.3 / -24.0 points.
- *Depth*: Spearman's rank correlation over nest cells. It is identical to 4 decimals for R0 and
  every R1 eps (s1@200k: 0.9494), because R1 only shifts the nest by a constant (bottom line).
  The level ranks with depth just as well under the sealed rule, but over a 6% span.

## Table 4: stability between refreshes (item 6; MEASURED; `p2_stability.txt`)

Seed 1, 101 maps at 150k-250k, one every 1,000 frames, sources from that map only.

| Rule | Gas-sealed maps | Nest median level, min / median / max | Change per refresh, \|dlog\| median / p90 / max | Refreshes jumping >25% | Descent direction changes per refresh (extra) |
|---|---|---|---|---|---|
| R0 | 43/101 | 1,569 / 2,984 / 6,814 (x4.3) | 0.173 / 0.720 / 0.972 | 19 of 43 | 8% of cells |
| R1 eps 0.02 | 0/101 | 1,080 / 1,441 / 2,236 (x2.1) | 0.092 / 0.228 / 0.433 | 11 of 100 | 10% |
| R1 eps 0.1 | 0/101 | 590 / 756 / 1,005 (x1.7) | 0.056 / 0.143 / 0.195 | 0 of 100 | 10% |
| R0, 8-nbr (extra; = expB line 2) | 0/101 | 434 / 673 / 1,490 (x3.4) | 0.147 / 0.450 / 0.601 | 39 of 100 | 10% |
| R1 eps 0.1, 8-nbr (extra) | 0/101 | 271 / 348 / 470 (x1.7) | 0.052 / 0.140 / 0.193 | 0 of 100 | 10% |

What each column means:
- *|dlog|* is |ln(level_t / level_t-1)| between consecutive maps that are both open; 0.056 is
  a 6% change and 0.720 is 105%.
- *Jumps* counts pairs where both maps are open.
- *Direction changes*: share of nest cells, present in both maps, whose exact 1-cell descent
  direction differs. The "reads and points out" status flips for 0.9-2.1% of cells.

## Table 5: settling if stepped physically (item 7)

**ASSUMED rate:** nest air would diffuse like the trail planes in `src/sim/pheromone.rs`: a 3x3
blend at `DIFFUSE` = 0.25 per pass, one pass per 12 frames. That is a continuum diffusivity
D = 0.25/36 cells^2 per frame, the constant `gradcheck.settle_frames` used, validated on the
shaft. Every cell, open or porous, holds gas at unit capacity (also ASSUMED). Frames scale as
1/D. The operator is MEASURED (eigsh, shift-invert at 0, 60 slowest modes).

| Map | Rule | tau_1 = 1/(D lambda_1), frames | Where the slowest mode lives | Nest within 10% of the solved field, from empty |
|---|---|---|---|---|
| s1@200k | R0 | 1,960,833 | 95% nest | 4,524,863 (explicit stepping: 4,525,200) |
| s1@200k | R1 0.02 / 0.05 / 0.1 / 0.3 | 950,226 / 755,529 / 651,998 / 537,615 | 96-99% nest | 2,200,024 / 1,747,094 / 1,505,939 (stepping: 1,506,000) / 1,239,614 |
| s1@200k | R1 0.1, 8-nbr (D/2) | 619,195 | 99% nest | 1,429,642 |
| s1@200k | R2 0.02 / 0.05 / 0.1 / 0.3 | 184,034,575 / 74,780,965 / 38,162,576 / 13,422,927 | 100% packed ground | 4,790,400 (stepping; the mode estimate was rejected) / 4,465,181 / 4,375,327 (stepping: 4,376,400) / 4,052,698 |
| s3@200k | R0 | 2,202,227 | 91% nest | 5,091,676 |
| s3@200k | R1 0.02 / 0.05 / 0.1 / 0.3 | 967,957 / 747,903 / 632,084 / 505,080 | 95-99% nest | 2,246,777 / 1,732,222 / 1,461,095 / 1,164,635 |
| s3@200k | R1 0.1, 8-nbr (D/2) | 618,141 | 98% nest | 1,428,554 |
| s3@200k | R2 0.02 / 0.05 / 0.1 / 0.3 | 188,449,772 / 76,518,427 / 38,996,489 / 13,659,989 | 100% packed ground | (unreliable, ~3.8M) / 4,349,421 / 4,130,471 / 3,677,259 |

What each column means:
- *tau_1*: the e-folding time of the slowest mode.
- *Where it lives*: the share of that mode's squared amplitude on nest cells, or on soil.
- *Nest within 10%*: computed from the 60 slowest modes, with the rest bounded. Checked by
  explicit stepping where shown.
  - The R2 0.02 estimate from modes was rejected: its bound left 14% unexplained.
  - Under R2 0.3 the first mode with at least 10% of its weight in the nest is mode #22,
    tau 386,160 frames (s1). Under R2 at 0.02-0.1, none of the 60 slowest modes reaches 10%.
- The original check's "sealed walls did not settle within 1,500,000 frames" was its cap; the
  value is 4.5M.

## What this means for "should loose soil pass air, and at what conductance" (INFERRED)

1. **Mound porosity is a lever on the denominator, not on the slope.**
   - Within the nest, R1 at any eps is R0 minus a constant. So "loose soil passes air" buys
     readability only because the sensor reads a ratio.
   - The same readings come from R0's sealed field with that constant subtracted. That
     constant is the door's level, which is mostly set by the mound's ants and its few sky
     openings.
   - If readability is the goal, the cheaper equivalent is to read the slope against a lower
     reference: a smaller baseline, or an absolute-difference threshold. Treating the mound as
     outside (the reviewer's 74%) is the limit of the same lever.
2. **If a porosity rule ships anyway, restrict it to mound spoil (R1). eps 0.05-0.1 is the
   defensible range:**
   - every map opens (11/11);
   - reads and points out 21.6-27.4%;
   - no refresh jumps over 25% at 0.1;
   - the share of a room cell's level from ants above ground halves (39 -> 22.5% at 0.1).
   - Going to 0.3 raises readability to 43% but sends the exact slope into mound pockets on 6
     of 11 maps.
   - 0.02 leaves 11 of 100 refreshes jumping more than 25%.
3. **Do not let the packed ground leak (R2).** Even at eps/20 = 0.001 it:
   - points the exact slope at shallow ceilings (59-83% of descents end inside the nest);
   - gives 5.5-7.8 points less "reads and points out" than R1's pure level shift on every one
     of the 7 R0-open maps (`p1e_shift.txt`);
   - parks the slowest mode in the ground (13-188M frames).
4. **No conductance makes nest air something an ant can follow out with a threshold sensor,**
   and none makes it a local signal: 82-89% of a room cell's level comes from more than 6
   cells away, even at 0.3.
5. **No conductance makes it something a stepped gas could settle to.** Making it settle within
   ~10k frames would take a diffusion 100-500x faster than the trail planes: tens of explicit
   passes per frame, or a direct solve each refresh, which is the global computation F2
   already flagged. INFERRED from the 1.2-5.1M frames in table 5.

## Could not establish

- **Which cells are loose.** The map does not say whether an 's' cell is `soil`, `packedsoil` or
  `spoil`, so R2's "loose below the ground line" is a proxy.
  - The engine's own words cut the other way. `assets/materials/soil.ron` calls natural soil
    "loose tilth". `spoil.ron` calls the dumped pellet "tamped ground that has been placed".
  - So a literal "loose soil passes air" on engine materials could mean the ground leaks and the
    mound does not: closer to the R2all bracket, the worst case for direction.
  - I could not check (no git) whether this baseline's mound is `spoil` or `soil`.
- **How much gas a porous cell should hold.** Unit capacity is assumed. A lower air-filled
  porosity would speed R2's ground modes up; their size was not swept.
- **The real nest-air pass rate.** No implementation exists; table 5 uses the trail planes'
  constants.
- **Whether ants would actually behave differently.** Everything here is a steady-state field
  read by idealised followers. No kinesis or stochastic walk was run.
- **How the stability splits between ants moving and geometry changing.** Not separated, as in
  the review.

## Reproduce

```
D=/mnt/project-files/needs-ant/porosity
python3 -I $D/deps/expH.py $D/deps                 # gate: R0 7/11, span 6.0% [2.9-8.8], readable 9.1% [3.3-16.3]  (6 s)
python3 -I $D/p0_controls.py $D/deps > $D/p0_controls.out          # controls (a)-(e)
python3 -I $D/p0b_specificity.py $D/deps > $D/p0b_specificity.out  # why eps 1e-6 != R0
python3 -I $D/p1_rules.py $D/deps                  # items 1-5 -> p1_summary.txt, p1_permap.tsv (165 rows = 11 maps x 15 variants; ~1 min)
python3 -I $D/p1b_terminals.py $D/deps             # where descents end -> p1b_terminals.txt
python3 -I $D/p1c_sensor_follower.py $D/deps       # extra -> p1c_sensor_follower.txt
python3 -I $D/p1d_wallward.py $D/deps              # extra -> p1d_wallward.txt
python3 -I $D/p1e_shift.py $D/deps                 # level vs slope -> p1e_shift.txt
python3 -I $D/p2_stability.py $D/deps              # item 6 -> p2_stability.txt
python3 -I $D/p3_settle.py $D/deps                 # item 7 -> p3_settle.txt
python3 -I $D/p3b_settle_check.py $D/deps          # item 7 stepping checks (~1 min)
python3 -I $D/p4_render.py $D/deps                 # porosity_s1_f200000.png
```

- `deps/` holds verbatim copies of the reviewer's `gradcheck.py`, `helpers.py`, `expH.py`,
  `expB.py` and `expF.py`, with hashes in `deps/SHA256SUMS`.
- `poro_lib.py` holds the shared code: the generalised solver, followers, census and metrics.
- Outputs are written beside the scripts. numpy 2.5.3, scipy 1.18.1, Pillow 12.3.0.

## Picture

`porosity_s1_f200000.png`: seed 1 at 200k, with R0 and R1 eps 0.1 side by side.

- **Top row:** nest air over nest cells, on the nest's own range. Light brown marks spoil that
  passes gas. A cyan square marks the cell where 5 or more nest cells' exact descent ends when
  that is not the sky; under R1 that is one mound cell, for 838 of 845 nest cells.
- **Bottom row:** green where the 1% sensor reads and points out (10.1% under R0, 30.3% under
  R1), grey where the nest is flat to it, red where the slope runs into soil (0% in both).
- On their own ranges the two top panels look nearly identical. That is the bottom line in
  picture form: the shape is the same, and only the level moved.
