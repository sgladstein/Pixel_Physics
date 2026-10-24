# Does fast soil drainage change how plants grow? (2026-10-22)

> **Landed 2026-10-24** by the report sweep (thread *Land unlanded thread reports*): copied verbatim from the project shared folder `plant-perf/`. The drainage rule it checked shipped as PR #782 (`SOIL_DRAINAGE_RATE` 0.25 -> 1.0); the scratch harness it names lived in a container and is gone.

Plant performance thread, for the owner's question in the soil-water review thread:
*"Have the plant Lane consider if this will affect the actual plant growth in any way? If not, let's go for the best performance."*

**Answer: no measurable effect on plant growth.** Every growth difference between the shipped drainage and fast drainage is no larger
than the difference a near-zero change (drain rate 0.25 -> 0.26) makes by chance. One small physical shift: the top 10 rows of soil
sit ~0.025 wetter under fast drainage (0.542 -> 0.569 of saturation, 4 of 4 seeds), and water beside roots is slightly higher.
Plants are, if anything, a hair better watered. Soil-moisture writes fall from ~1,000 a tick to ~70.

## Setup
- Binary: main at 910268b3 (includes #773, rain per column) plus the review lane's two switches ported unchanged:
  `SCRATCH_DRAIN_RATE=1.0` (cell passes its whole surplus above field capacity down in one visit) and
  `SCRATCH_SCHEDULE_INFILTRATION=1` (keeps surface drying armed). Unset = main exactly.
- Arms: **off** (shipped, 0.25), **on** (1.0 + schedule fix), **ctl** (0.26: the noise floor; a bed is chaotic, so any change
  re-rolls every outcome, and this arm says how far that alone moves the numbers).
- Training beds: `plantrain mode=rungs species=tree,shrub,herb,creeper rungs=establish,grow,life stations=bright,dim layouts=12`.
- Community bed: `plant_m0 mode=community mix=roles frames=120000 sample=20000`, seeds 1-4 (ctl on 1-3); dry spells:
  `spells=20000,40000`, seeds 1-2. Numbers below are means over the census rows from 60,000 to 120,000 frames.

## Community bed (tree, shrub, herb, creeper), mister on
| seed | arm | plants_all | leaf_all | tree_leaf | shrub_leaf | herb_leaf | creeper_leaf | herb_rootwet | creeper_rootwet | top_water | sw |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | off | 68 | 6.37e+03 | 5.13e+03 | 393 | 751 | 96.2 | 0.56 | 0.547 | 0.533 | 1.07e+03 |
| 1 | ctl | 65 | 6.18e+03 | 4.84e+03 | 305 | 874 | 158 | 0.573 | 0.582 | 0.546 | 1.06e+03 |
| 1 | on | 72.8 | 6.13e+03 | 4.43e+03 | 560 | 1.03e+03 | 117 | 0.59 | 0.581 | 0.574 | 78 |
| 2 | off | 62.8 | 6.04e+03 | 3.56e+03 | 1.68e+03 | 630 | 166 | 0.601 | 0.495 | 0.555 | 1.01e+03 |
| 2 | ctl | 62.2 | 6.49e+03 | 3.95e+03 | 1.56e+03 | 843 | 136 | 0.597 | 0.532 | 0.568 | 958 |
| 2 | on | 70.5 | 6.78e+03 | 4.09e+03 | 1.72e+03 | 832 | 136 | 0.597 | 0.531 | 0.566 | 70.8 |
| 3 | off | 80.2 | 7.23e+03 | 4.9e+03 | 1.32e+03 | 955 | 59.2 | 0.597 | 0.536 | 0.537 | 999 |
| 3 | ctl | 62.5 | 8.39e+03 | 6.22e+03 | 1.2e+03 | 844 | 120 | 0.583 | 0.53 | 0.554 | 1e+03 |
| 3 | on | 71 | 9.24e+03 | 7.06e+03 | 1.33e+03 | 716 | 123 | 0.582 | 0.571 | 0.567 | 65.8 |
| 4 | off | 55.5 | 7.72e+03 | 5.81e+03 | 518 | 1.23e+03 | 162 | 0.567 | 0.534 | 0.544 | 1.27e+03 |
| 4 | on | 64.8 | 7.06e+03 | 5.43e+03 | 473 | 1e+03 | 156 | 0.604 | 0.572 | 0.569 | 72.5 |

## Community bed with wet/dry spells (20,000 wet, 40,000 dry)
| seed | arm | plants_all | leaf_all | tree_leaf | shrub_leaf | herb_leaf | creeper_leaf | herb_rootwet | creeper_rootwet | top_water | sw |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | off | 50 | 6.78e+03 | 5.55e+03 | 392 | 722 | 113 | 0.503 | 0.467 | 0.427 | 547 |
| 1 | ctl | 44 | 6.75e+03 | 5.62e+03 | 256 | 727 | 151 | 0.475 | 0.471 | 0.421 | 553 |
| 1 | on | 57.8 | 6.5e+03 | 4.61e+03 | 755 | 1.02e+03 | 121 | 0.525 | 0.479 | 0.435 | 46.5 |
| 2 | off | 56 | 5.98e+03 | 3.53e+03 | 1.54e+03 | 745 | 169 | 0.562 | 0.365 | 0.446 | 507 |
| 2 | on | 53.2 | 6.63e+03 | 4.03e+03 | 1.58e+03 | 879 | 130 | 0.555 | 0.442 | 0.449 | 38.4 |

`sw` = soil-moisture writes a tick (the cost the change removes). `top_water` = mean soil water in the top 10 rows.

## Training beds (12 layouts each; p50 end shoot cells; core-seconds a bed)
Raw output below. Summary: establishment counts, germination timing, end-shoot quartiles and growth rate move by the same
amounts between off and on as between off and ctl, in both directions. Largest moves: shrub grow-bright p50 359 (off) / 319 (ctl) / 302 (on);
shrub life-dim p50 369 / 225 / 336; herb life-bright p50 167 / 172 / 204. Cost a bed: establish 0.20 -> 0.12, grow 0.84 -> 0.46,
life 6.18 -> 3.64 core-s (bright).

### off
```
plantrain rungs: species=tree,shrub,herb,creeper rungs=establish,grow,life stations=bright,dim layouts=12 base=3000000 threads=2
  establish bright (width 48 frames 3000 n_est 20 lamp 1 rain Light soil 1): 0.20 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   1700 | end shoot p25    52 p50    62 p75   144 cv 0.71 | rgr/1k p25 0.78 p50 0.83 p75 0.94
    shrub    germ 12/12 est 12/12 est@ p50    900 | end shoot p25   126 p50   163 p75   200 cv 0.31 | rgr/1k p25 0.88 p50 1.05 p75 1.10
    herb     germ 12/12 est 12/12 est@ p50   1600 | end shoot p25    36 p50    44 p75    45 cv 0.12 | rgr/1k p25 0.44 p50 0.49 p75 0.49
    creeper  germ 12/12 est 12/12 est@ p50   1200 | end shoot p25    52 p50    56 p75    63 cv 0.26 | rgr/1k p25 0.50 p50 0.57 p75 0.70
  establish dim (width 48 frames 3000 n_est 20 lamp 0.4 rain Light soil 1): 0.17 core-s a bed
    tree     germ 12/12 est  9/12 est@ p50   2400 | end shoot p25    22 p50    29 p75    41 cv 0.78 | rgr/1k p25 0.48 p50 0.73 p75 1.03
    shrub    germ 12/12 est  7/12 est@ p50   1500 | end shoot p25    17 p50    21 p75    37 cv 0.95 | rgr/1k p25 0.17 p50 0.38 p75 1.06
    herb     germ 12/12 est 12/12 est@ p50   2500 | end shoot p25    22 p50    24 p75    24 cv 0.09 | rgr/1k p25 0.26 p50 0.32 p75 0.36
    creeper  germ 12/12 est  9/12 est@ p50   1300 | end shoot p25    24 p50    47 p75    52 cv 0.67 | rgr/1k p25 0.47 p50 0.53 p75 0.74
  grow bright (width 48 frames 10000 n_est 20 lamp 1 rain Light soil 1): 0.84 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   1750 | end shoot p25   433 p50   538 p75   700 cv 0.30 | rgr/1k p25 0.38 p50 0.40 p75 0.41
    shrub    germ 12/12 est 12/12 est@ p50   1000 | end shoot p25   147 p50   359 p75   405 cv 0.53 | rgr/1k p25 0.22 p50 0.32 p75 0.33
    herb     germ 12/12 est 12/12 est@ p50   1750 | end shoot p25    95 p50   108 p75   116 cv 0.35 | rgr/1k p25 0.20 p50 0.20 p75 0.21
    creeper  germ 12/12 est 12/12 est@ p50   1250 | end shoot p25    71 p50    82 p75   131 cv 0.33 | rgr/1k p25 0.15 p50 0.17 p75 0.21
  grow dim (width 48 frames 10000 n_est 20 lamp 0.4 rain Light soil 1): 0.69 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   2750 | end shoot p25    84 p50   372 p75   477 cv 0.70 | rgr/1k p25 0.21 p50 0.38 p75 0.39
    shrub    germ 12/12 est 11/12 est@ p50   2750 | end shoot p25    58 p50    77 p75    91 cv 0.92 | rgr/1k p25 0.16 p50 0.20 p75 0.28
    herb     germ 12/12 est 12/12 est@ p50   2500 | end shoot p25    60 p50    62 p75    79 cv 0.35 | rgr/1k p25 0.15 p50 0.16 p75 0.18
    creeper  germ 12/12 est 11/12 est@ p50   1500 | end shoot p25    36 p50    65 p75    80 cv 0.59 | rgr/1k p25 0.11 p50 0.13 p75 0.19
  life bright (width 96 frames 40000 n_est 20 lamp 1 rain Light soil 1): 6.18 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   2000 | end shoot p25  1778 p50  2508 p75  2727 cv 0.39 | rgr/1k p25 0.12 p50 0.13 p75 0.13
    shrub    germ 12/12 est 12/12 est@ p50   1250 | end shoot p25   355 p50   625 p75   702 cv 0.42 | rgr/1k p25 0.08 p50 0.09 p75 0.09
    herb     germ 12/12 est 12/12 est@ p50   1750 | end shoot p25   146 p50   167 p75   203 cv 0.48 | rgr/1k p25 0.05 p50 0.06 p75 0.06
    creeper  germ 12/12 est 12/12 est@ p50   1000 | end shoot p25    70 p50   104 p75   117 cv 0.30 | rgr/1k p25 0.03 p50 0.04 p75 0.05
  life dim (width 96 frames 40000 n_est 20 lamp 0.4 rain Light soil 1): 4.92 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   3500 | end shoot p25   273 p50  1286 p75  1598 cv 0.82 | rgr/1k p25 0.07 p50 0.11 p75 0.12
    shrub    germ 12/12 est 11/12 est@ p50   2750 | end shoot p25   187 p50   369 p75   503 cv 0.61 | rgr/1k p25 0.08 p50 0.09 p75 0.09
    herb     germ 12/12 est 12/12 est@ p50   2750 | end shoot p25   174 p50   226 p75   236 cv 0.49 | rgr/1k p25 0.06 p50 0.07 p75 0.07
    creeper  germ 12/12 est 11/12 est@ p50   1750 | end shoot p25    60 p50    75 p75   110 cv 0.53 | rgr/1k p25 0.03 p50 0.03 p75 0.05
```
### ctl (0.26)
```
plantrain rungs: species=tree,shrub,herb,creeper rungs=establish,grow,life stations=bright,dim layouts=12 base=3000000 threads=2
  establish bright (width 48 frames 3000 n_est 20 lamp 1 rain Light soil 1): 0.20 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   1700 | end shoot p25    52 p50    62 p75   144 cv 0.70 | rgr/1k p25 0.78 p50 0.83 p75 0.94
    shrub    germ 12/12 est 12/12 est@ p50    900 | end shoot p25   126 p50   156 p75   200 cv 0.31 | rgr/1k p25 0.88 p50 1.03 p75 1.10
    herb     germ 12/12 est 12/12 est@ p50   1600 | end shoot p25    36 p50    44 p75    45 cv 0.12 | rgr/1k p25 0.44 p50 0.48 p75 0.49
    creeper  germ 12/12 est 12/12 est@ p50   1200 | end shoot p25    52 p50    56 p75    63 cv 0.26 | rgr/1k p25 0.50 p50 0.57 p75 0.70
  establish dim (width 48 frames 3000 n_est 20 lamp 0.4 rain Light soil 1): 0.15 core-s a bed
    tree     germ 12/12 est  9/12 est@ p50   2400 | end shoot p25    22 p50    29 p75    41 cv 0.77 | rgr/1k p25 0.48 p50 0.73 p75 1.03
    shrub    germ 12/12 est  6/12 est@ p50   1500 | end shoot p25    17 p50    22 p75    37 cv 0.98 | rgr/1k p25 0.38 p50 0.56 p75 1.06
    herb     germ 12/12 est 12/12 est@ p50   2500 | end shoot p25    22 p50    24 p75    24 cv 0.09 | rgr/1k p25 0.26 p50 0.32 p75 0.36
    creeper  germ 12/12 est  9/12 est@ p50   1300 | end shoot p25    24 p50    47 p75    52 cv 0.68 | rgr/1k p25 0.47 p50 0.53 p75 0.74
  grow bright (width 48 frames 10000 n_est 20 lamp 1 rain Light soil 1): 0.81 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   1750 | end shoot p25   433 p50   538 p75   700 cv 0.31 | rgr/1k p25 0.38 p50 0.40 p75 0.41
    shrub    germ 12/12 est 12/12 est@ p50   1000 | end shoot p25   142 p50   319 p75   364 cv 0.55 | rgr/1k p25 0.22 p50 0.31 p75 0.32
    herb     germ 12/12 est 12/12 est@ p50   1750 | end shoot p25    95 p50   108 p75   116 cv 0.35 | rgr/1k p25 0.20 p50 0.20 p75 0.21
    creeper  germ 12/12 est 12/12 est@ p50   1250 | end shoot p25    73 p50    82 p75   134 cv 0.32 | rgr/1k p25 0.15 p50 0.17 p75 0.21
  grow dim (width 48 frames 10000 n_est 20 lamp 0.4 rain Light soil 1): 0.68 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   2750 | end shoot p25    84 p50   357 p75   498 cv 0.74 | rgr/1k p25 0.21 p50 0.37 p75 0.39
    shrub    germ 12/12 est 10/12 est@ p50   3000 | end shoot p25    68 p50    86 p75   198 cv 0.78 | rgr/1k p25 0.18 p50 0.28 p75 0.31
    herb     germ 12/12 est 12/12 est@ p50   2500 | end shoot p25    60 p50    62 p75    79 cv 0.35 | rgr/1k p25 0.15 p50 0.16 p75 0.18
    creeper  germ 12/12 est 10/12 est@ p50   1500 | end shoot p25    54 p50    68 p75    88 cv 0.54 | rgr/1k p25 0.13 p50 0.19 p75 0.19
  life bright (width 96 frames 40000 n_est 20 lamp 1 rain Light soil 1): 6.27 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   2000 | end shoot p25  1786 p50  2529 p75  2632 cv 0.44 | rgr/1k p25 0.12 p50 0.13 p75 0.13
    shrub    germ 12/12 est 12/12 est@ p50   1250 | end shoot p25   417 p50   671 p75   715 cv 0.42 | rgr/1k p25 0.08 p50 0.09 p75 0.09
    herb     germ 12/12 est 12/12 est@ p50   1750 | end shoot p25   145 p50   172 p75   192 cv 0.42 | rgr/1k p25 0.05 p50 0.06 p75 0.06
    creeper  germ 12/12 est 12/12 est@ p50   1000 | end shoot p25    74 p50   104 p75   117 cv 0.26 | rgr/1k p25 0.03 p50 0.04 p75 0.05
  life dim (width 96 frames 40000 n_est 20 lamp 0.4 rain Light soil 1): 5.08 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   3500 | end shoot p25   339 p50  1342 p75  1837 cv 0.74 | rgr/1k p25 0.08 p50 0.12 p75 0.12
    shrub    germ 12/12 est 11/12 est@ p50   2750 | end shoot p25   186 p50   225 p75   425 cv 0.69 | rgr/1k p25 0.06 p50 0.07 p75 0.09
    herb     germ 12/12 est 12/12 est@ p50   2750 | end shoot p25   200 p50   221 p75   231 cv 0.35 | rgr/1k p25 0.06 p50 0.06 p75 0.07
    creeper  germ 12/12 est 11/12 est@ p50   1750 | end shoot p25    49 p50    73 p75    78 cv 0.59 | rgr/1k p25 0.03 p50 0.03 p75 0.04
```
### on (1.0)
```
plantrain rungs: species=tree,shrub,herb,creeper rungs=establish,grow,life stations=bright,dim layouts=12 base=3000000 threads=2
  establish bright (width 48 frames 3000 n_est 20 lamp 1 rain Light soil 1): 0.12 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   1700 | end shoot p25    52 p50    61 p75   140 cv 0.70 | rgr/1k p25 0.78 p50 0.83 p75 0.94
    shrub    germ 12/12 est 12/12 est@ p50    900 | end shoot p25   126 p50   170 p75   200 cv 0.30 | rgr/1k p25 0.88 p50 1.07 p75 1.10
    herb     germ 12/12 est 12/12 est@ p50   1600 | end shoot p25    36 p50    44 p75    45 cv 0.15 | rgr/1k p25 0.44 p50 0.48 p75 0.49
    creeper  germ 12/12 est 12/12 est@ p50   1200 | end shoot p25    51 p50    53 p75    61 cv 0.32 | rgr/1k p25 0.45 p50 0.50 p75 0.62
  establish dim (width 48 frames 3000 n_est 20 lamp 0.4 rain Light soil 1): 0.11 core-s a bed
    tree     germ 12/12 est  9/12 est@ p50   2400 | end shoot p25    22 p50    29 p75    41 cv 0.77 | rgr/1k p25 0.48 p50 0.73 p75 1.03
    shrub    germ 12/12 est  6/12 est@ p50   1500 | end shoot p25    11 p50    21 p75    40 cv 1.00 | rgr/1k p25 0.17 p50 0.56 p75 1.00
    herb     germ 12/12 est 12/12 est@ p50   2500 | end shoot p25    22 p50    24 p75    24 cv 0.10 | rgr/1k p25 0.26 p50 0.32 p75 0.36
    creeper  germ 12/12 est  9/12 est@ p50   1300 | end shoot p25    24 p50    45 p75    52 cv 0.67 | rgr/1k p25 0.45 p50 0.53 p75 0.74
  grow bright (width 48 frames 10000 n_est 20 lamp 1 rain Light soil 1): 0.46 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   1750 | end shoot p25   432 p50   565 p75   621 cv 0.29 | rgr/1k p25 0.38 p50 0.40 p75 0.41
    shrub    germ 12/12 est 12/12 est@ p50   1000 | end shoot p25   146 p50   302 p75   328 cv 0.49 | rgr/1k p25 0.22 p50 0.30 p75 0.31
    herb     germ 12/12 est 12/12 est@ p50   1750 | end shoot p25    86 p50   100 p75   108 cv 0.36 | rgr/1k p25 0.19 p50 0.20 p75 0.20
    creeper  germ 12/12 est 12/12 est@ p50   1250 | end shoot p25    70 p50    82 p75   106 cv 0.33 | rgr/1k p25 0.14 p50 0.17 p75 0.20
  grow dim (width 48 frames 10000 n_est 20 lamp 0.4 rain Light soil 1): 0.44 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   2750 | end shoot p25    84 p50   372 p75   508 cv 0.74 | rgr/1k p25 0.21 p50 0.39 p75 0.40
    shrub    germ 12/12 est 11/12 est@ p50   2750 | end shoot p25    36 p50    59 p75   230 cv 0.95 | rgr/1k p25 0.12 p50 0.17 p75 0.32
    herb     germ 12/12 est 12/12 est@ p50   2500 | end shoot p25    59 p50    65 p75    68 cv 0.35 | rgr/1k p25 0.15 p50 0.16 p75 0.18
    creeper  germ 12/12 est 11/12 est@ p50   1500 | end shoot p25    44 p50    64 p75    78 cv 0.46 | rgr/1k p25 0.14 p50 0.18 p75 0.20
  life bright (width 96 frames 40000 n_est 20 lamp 1 rain Light soil 1): 3.64 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   2000 | end shoot p25  1630 p50  2492 p75  2630 cv 0.45 | rgr/1k p25 0.12 p50 0.13 p75 0.13
    shrub    germ 12/12 est 12/12 est@ p50   1250 | end shoot p25   348 p50   605 p75   704 cv 0.54 | rgr/1k p25 0.07 p50 0.09 p75 0.09
    herb     germ 12/12 est 12/12 est@ p50   1750 | end shoot p25   122 p50   204 p75   225 cv 0.47 | rgr/1k p25 0.05 p50 0.06 p75 0.06
    creeper  germ 12/12 est 12/12 est@ p50   1000 | end shoot p25    77 p50   106 p75   119 cv 0.27 | rgr/1k p25 0.03 p50 0.04 p75 0.05
  life dim (width 96 frames 40000 n_est 20 lamp 0.4 rain Light soil 1): 3.01 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   3250 | end shoot p25   430 p50  1212 p75  1742 cv 0.69 | rgr/1k p25 0.09 p50 0.11 p75 0.12
    shrub    germ 12/12 est 12/12 est@ p50   6750 | end shoot p25   165 p50   336 p75   511 cv 0.64 | rgr/1k p25 0.07 p50 0.09 p75 0.09
    herb     germ 12/12 est 12/12 est@ p50   2750 | end shoot p25   181 p50   225 p75   227 cv 0.48 | rgr/1k p25 0.06 p50 0.07 p75 0.07
    creeper  germ 12/12 est 11/12 est@ p50   1750 | end shoot p25    63 p50    77 p75    92 cv 0.51 | rgr/1k p25 0.03 p50 0.04 p75 0.04
```


---

## Addendum (2026-10-22, 01:10Z): three re-checks, same answer

1. **Root-mat site.** The first "on" arm changed the rate only at the main drainage site; the root-mat pull still read 0.25.
   Re-ran "on" with both sites moved (main 910268b3): community plants 77 / 71.8 / 63.2 / 60 (seeds 1-4), top band 0.556-0.566,
   rungs within the control's moves. No change to the conclusion.
2. **Today's main, with #777** (plants close stomata on dry soil, so growth now reads soil water). Branch off origin/main dc79ccd5 + the
   switches at both sites; rungs gain the dry station. Community bed and dry spells below; the dry station's life rung at 12 layouts
   suggested a tree drop (p50 423 -> 240) but at 48 layouts it is noise: tree p50 192 off / 172 ctl / 192 on, herb 226 / 197 / 227,
   creeper 45 / 45 / 55, shrub 0 / 0 / 0 (p75 243 / 253 / 228).
3. **The real commit** (lab world thread, 80295015: SOIL_DRAINAGE_RATE 1.0 + drying schedule armed on "drank") against its main
   65a166ac7, four species' rungs bright/dim: **pass**, every move the size of the 0.26 control and in both directions. (Core-seconds in
   that pair are contaminated by a concurrent job; do not quote them.)

### Community bed on main + #777 (means over 60k-120k frames)
| run | arm | plants_all | leaf_all | tree_leaf | shrub_leaf | herb_leaf | creeper_leaf | herb_rootwet | creeper_rootwet | top_water | sw |
|---|---|---|---|---|---|---|---|---|---|---|---|
| comm 1 | off | 72 | 6.22e+03 | 4.61e+03 | 294 | 1.19e+03 | 127 | 0.584 | 0.591 | 0.544 | 1.1e+03 |
| comm 1 | ctl | 71.2 | 6.38e+03 | 4.72e+03 | 406 | 1.12e+03 | 133 | 0.586 | 0.525 | 0.541 | 1.13e+03 |
| comm 1 | on | 79.8 | 6.04e+03 | 3.98e+03 | 609 | 1.32e+03 | 132 | 0.61 | 0.569 | 0.571 | 58.9 |
| comm 2 | off | 58.8 | 6.55e+03 | 3.85e+03 | 1.86e+03 | 716 | 124 | 0.602 | 0.522 | 0.567 | 994 |
| comm 2 | ctl | 66.8 | 6.61e+03 | 3.72e+03 | 2.09e+03 | 669 | 134 | 0.584 | 0.521 | 0.553 | 998 |
| comm 2 | on | 57 | 7.28e+03 | 4.49e+03 | 1.84e+03 | 832 | 121 | 0.586 | 0.548 | 0.56 | 58.9 |
| comm 3 | off | 75.5 | 7.73e+03 | 5.6e+03 | 1.41e+03 | 552 | 172 | 0.57 | 0.541 | 0.55 | 1.13e+03 |
| comm 3 | ctl | 69.8 | 7.64e+03 | 5.65e+03 | 968 | 887 | 138 | 0.587 | 0.526 | 0.552 | 1.03e+03 |
| comm 3 | on | 69.5 | 7.96e+03 | 6.05e+03 | 873 | 960 | 70.5 | 0.573 | 0.548 | 0.562 | 58.4 |
| comm 4 | off | 66 | 7.4e+03 | 5.46e+03 | 566 | 1.21e+03 | 168 | 0.583 | 0.531 | 0.554 | 1.23e+03 |
| comm 4 | ctl | 52.8 | 7.29e+03 | 5.37e+03 | 698 | 1.08e+03 | 150 | 0.604 | 0.558 | 0.561 | 1.1e+03 |
| comm 4 | on | 47.2 | 7.45e+03 | 5.81e+03 | 621 | 874 | 140 | 0.586 | 0.553 | 0.561 | 71.3 |
| sp 1 | off | 55 | 5.66e+03 | 4.35e+03 | 330 | 855 | 125 | 0.503 | 0.48 | 0.432 | 525 |
| sp 1 | ctl | 60.2 | 5.7e+03 | 3.9e+03 | 372 | 1.3e+03 | 135 | 0.52 | 0.466 | 0.4 | 554 |
| sp 1 | on | 63.8 | 6.07e+03 | 4.14e+03 | 841 | 978 | 115 | 0.517 | 0.48 | 0.441 | 37.2 |
| sp 2 | off | 48.5 | 6.37e+03 | 3.87e+03 | 1.68e+03 | 678 | 133 | 0.526 | 0.379 | 0.439 | 507 |
| sp 2 | ctl | 42.8 | 6.52e+03 | 3.98e+03 | 1.91e+03 | 474 | 160 | 0.532 | 0.424 | 0.445 | 530 |
| sp 2 | on | 52.5 | 6.48e+03 | 3.93e+03 | 1.47e+03 | 928 | 143 | 0.541 | 0.439 | 0.458 | 37.6 |

### Dry station, life rung, 48 layouts (main + #777)
### off
```
  life dry (width 96 frames 40000 n_est 20 lamp 1 rain Off soil 0.25): 0.56 core-s a bed
    tree     germ 48/48 est 29/48 est@ p50   3000 | end shoot p25     0 p50   192 p75   871 cv 1.33 | rgr/1k p25 0.08 p50 0.10 p75 0.12
    shrub    germ 48/48 est 22/48 est@ p50   2500 | end shoot p25     0 p50     0 p75   243 cv 1.46 | rgr/1k p25 0.06 p50 0.07 p75 0.10
    herb     germ 48/48 est 40/48 est@ p50   3000 | end shoot p25     0 p50   226 p75   250 cv 0.69 | rgr/1k p25 0.06 p50 0.07 p75 0.07
    creeper  germ 48/48 est 28/48 est@ p50   2000 | end shoot p25     0 p50    45 p75    80 cv 1.01 | rgr/1k p25 0.03 p50 0.04 p75 0.04
```
### ctl
```
  life dry (width 96 frames 40000 n_est 20 lamp 1 rain Off soil 0.25): 0.59 core-s a bed
    tree     germ 48/48 est 29/48 est@ p50   3000 | end shoot p25     0 p50   172 p75   917 cv 1.38 | rgr/1k p25 0.07 p50 0.10 p75 0.12
    shrub    germ 48/48 est 22/48 est@ p50   2500 | end shoot p25     0 p50     0 p75   253 cv 1.48 | rgr/1k p25 0.06 p50 0.07 p75 0.10
    herb     germ 48/48 est 40/48 est@ p50   3000 | end shoot p25     0 p50   197 p75   257 cv 0.69 | rgr/1k p25 0.06 p50 0.07 p75 0.07
    creeper  germ 48/48 est 28/48 est@ p50   2000 | end shoot p25     0 p50    45 p75    80 cv 1.00 | rgr/1k p25 0.03 p50 0.04 p75 0.04
```
### on
```
  life dry (width 96 frames 40000 n_est 20 lamp 1 rain Off soil 0.25): 0.58 core-s a bed
    tree     germ 48/48 est 29/48 est@ p50   3000 | end shoot p25     0 p50   192 p75  1321 cv 1.34 | rgr/1k p25 0.07 p50 0.11 p75 0.13
    shrub    germ 48/48 est 22/48 est@ p50   2500 | end shoot p25     0 p50     0 p75   228 cv 1.51 | rgr/1k p25 0.06 p50 0.07 p75 0.09
    herb     germ 48/48 est 40/48 est@ p50   3250 | end shoot p25     0 p50   227 p75   255 cv 0.68 | rgr/1k p25 0.06 p50 0.07 p75 0.07
    creeper  germ 48/48 est 28/48 est@ p50   2000 | end shoot p25     0 p50    55 p75    80 cv 0.97 | rgr/1k p25 0.03 p50 0.04 p75 0.04
```

### Real commit: rungs, main 65a166ac7 (base) vs 80295015 (new)
### base
```
plantrain rungs: species=tree,shrub,herb,creeper rungs=establish,grow,life stations=bright,dim layouts=12 base=3000000 threads=2
  establish bright (width 48 frames 3000 n_est 20 lamp 1 rain Light soil 1): 0.19 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   1700 | end shoot p25    52 p50    61 p75   143 cv 0.71 | rgr/1k p25 0.78 p50 0.83 p75 0.94
    shrub    germ 12/12 est 12/12 est@ p50    900 | end shoot p25   126 p50   163 p75   200 cv 0.31 | rgr/1k p25 0.88 p50 1.05 p75 1.10
    herb     germ 12/12 est 12/12 est@ p50   1600 | end shoot p25    36 p50    44 p75    45 cv 0.12 | rgr/1k p25 0.44 p50 0.47 p75 0.49
    creeper  germ 12/12 est 12/12 est@ p50   1200 | end shoot p25    52 p50    56 p75    75 cv 0.27 | rgr/1k p25 0.50 p50 0.57 p75 0.71
  establish dim (width 48 frames 3000 n_est 20 lamp 0.4 rain Light soil 1): 0.15 core-s a bed
    tree     germ 12/12 est  8/12 est@ p50   2400 | end shoot p25    13 p50    29 p75    41 cv 0.81 | rgr/1k p25 0.62 p50 0.97 p75 1.03
    shrub    germ 12/12 est  8/12 est@ p50   1600 | end shoot p25    15 p50    21 p75    37 cv 0.97 | rgr/1k p25 0.16 p50 0.38 p75 0.56
    herb     germ 12/12 est 12/12 est@ p50   2500 | end shoot p25    22 p50    24 p75    24 cv 0.09 | rgr/1k p25 0.26 p50 0.32 p75 0.36
    creeper  germ 12/12 est  9/12 est@ p50   1300 | end shoot p25    29 p50    47 p75    52 cv 0.69 | rgr/1k p25 0.51 p50 0.53 p75 0.74
  grow bright (width 48 frames 10000 n_est 20 lamp 1 rain Light soil 1): 0.87 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   1750 | end shoot p25   468 p50   606 p75   713 cv 0.25 | rgr/1k p25 0.39 p50 0.40 p75 0.41
    shrub    germ 12/12 est 12/12 est@ p50   1000 | end shoot p25   147 p50   359 p75   405 cv 0.51 | rgr/1k p25 0.22 p50 0.32 p75 0.33
    herb     germ 12/12 est 12/12 est@ p50   1750 | end shoot p25    94 p50   108 p75   116 cv 0.35 | rgr/1k p25 0.20 p50 0.20 p75 0.21
    creeper  germ 12/12 est 12/12 est@ p50   1250 | end shoot p25    71 p50    82 p75   126 cv 0.33 | rgr/1k p25 0.15 p50 0.17 p75 0.21
  grow dim (width 48 frames 10000 n_est 20 lamp 0.4 rain Light soil 1): 1.29 core-s a bed
    tree     germ 12/12 est 11/12 est@ p50   2500 | end shoot p25    58 p50   357 p75   477 cv 0.77 | rgr/1k p25 0.21 p50 0.37 p75 0.39
    shrub    germ 12/12 est 10/12 est@ p50   2750 | end shoot p25    56 p50    69 p75    77 cv 1.01 | rgr/1k p25 0.16 p50 0.25 p75 0.28
    herb     germ 12/12 est 12/12 est@ p50   2500 | end shoot p25    60 p50    65 p75    79 cv 0.36 | rgr/1k p25 0.15 p50 0.16 p75 0.18
    creeper  germ 12/12 est 10/12 est@ p50   1500 | end shoot p25    52 p50    62 p75    80 cv 0.55 | rgr/1k p25 0.13 p50 0.18 p75 0.19
  life bright (width 96 frames 40000 n_est 20 lamp 1 rain Light soil 1): 7.30 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   2000 | end shoot p25  1671 p50  2532 p75  2972 cv 0.41 | rgr/1k p25 0.12 p50 0.13 p75 0.13
    shrub    germ 12/12 est 12/12 est@ p50   1250 | end shoot p25   353 p50   583 p75   619 cv 0.43 | rgr/1k p25 0.08 p50 0.09 p75 0.09
    herb     germ 12/12 est 12/12 est@ p50   1750 | end shoot p25   135 p50   191 p75   226 cv 0.42 | rgr/1k p25 0.06 p50 0.06 p75 0.07
    creeper  germ 12/12 est 12/12 est@ p50   1000 | end shoot p25    71 p50   104 p75   117 cv 0.30 | rgr/1k p25 0.03 p50 0.04 p75 0.05
  life dim (width 96 frames 40000 n_est 20 lamp 0.4 rain Light soil 1): 4.57 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   3500 | end shoot p25   273 p50  1286 p75  1598 cv 0.82 | rgr/1k p25 0.07 p50 0.11 p75 0.12
    shrub    germ 12/12 est 11/12 est@ p50   2750 | end shoot p25   187 p50   369 p75   508 cv 0.60 | rgr/1k p25 0.08 p50 0.09 p75 0.09
    herb     germ 12/12 est 12/12 est@ p50   2750 | end shoot p25   195 p50   226 p75   238 cv 0.48 | rgr/1k p25 0.06 p50 0.07 p75 0.07
    creeper  germ 12/12 est 11/12 est@ p50   1750 | end shoot p25    60 p50    75 p75   110 cv 0.53 | rgr/1k p25 0.03 p50 0.03 p75 0.05
```
### new
```
plantrain rungs: species=tree,shrub,herb,creeper rungs=establish,grow,life stations=bright,dim layouts=12 base=3000000 threads=2
  establish bright (width 48 frames 3000 n_est 20 lamp 1 rain Light soil 1): 0.10 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   1700 | end shoot p25    52 p50    61 p75   140 cv 0.70 | rgr/1k p25 0.78 p50 0.83 p75 0.94
    shrub    germ 12/12 est 12/12 est@ p50    900 | end shoot p25   126 p50   170 p75   200 cv 0.30 | rgr/1k p25 0.88 p50 1.07 p75 1.10
    herb     germ 12/12 est 12/12 est@ p50   1600 | end shoot p25    36 p50    44 p75    45 cv 0.15 | rgr/1k p25 0.44 p50 0.48 p75 0.49
    creeper  germ 12/12 est 12/12 est@ p50   1200 | end shoot p25    51 p50    53 p75    61 cv 0.31 | rgr/1k p25 0.45 p50 0.50 p75 0.62
  establish dim (width 48 frames 3000 n_est 20 lamp 0.4 rain Light soil 1): 0.10 core-s a bed
    tree     germ 12/12 est  9/12 est@ p50   2400 | end shoot p25    22 p50    29 p75    41 cv 0.77 | rgr/1k p25 0.48 p50 0.73 p75 1.03
    shrub    germ 12/12 est  6/12 est@ p50   1500 | end shoot p25    10 p50    21 p75    40 cv 0.99 | rgr/1k p25 0.17 p50 0.56 p75 1.00
    herb     germ 12/12 est 12/12 est@ p50   2500 | end shoot p25    22 p50    24 p75    24 cv 0.10 | rgr/1k p25 0.26 p50 0.32 p75 0.36
    creeper  germ 12/12 est  9/12 est@ p50   1300 | end shoot p25    24 p50    45 p75    52 cv 0.65 | rgr/1k p25 0.45 p50 0.53 p75 0.74
  grow bright (width 48 frames 10000 n_est 20 lamp 1 rain Light soil 1): 0.43 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   1750 | end shoot p25   432 p50   565 p75   621 cv 0.29 | rgr/1k p25 0.38 p50 0.40 p75 0.41
    shrub    germ 12/12 est 12/12 est@ p50   1000 | end shoot p25   147 p50   319 p75   364 cv 0.50 | rgr/1k p25 0.22 p50 0.31 p75 0.32
    herb     germ 12/12 est 12/12 est@ p50   1750 | end shoot p25    86 p50   100 p75   110 cv 0.36 | rgr/1k p25 0.19 p50 0.20 p75 0.20
    creeper  germ 12/12 est 12/12 est@ p50   1250 | end shoot p25    70 p50    83 p75   106 cv 0.33 | rgr/1k p25 0.14 p50 0.17 p75 0.20
  grow dim (width 48 frames 10000 n_est 20 lamp 0.4 rain Light soil 1): 0.40 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   2750 | end shoot p25    93 p50   372 p75   508 cv 0.66 | rgr/1k p25 0.29 p50 0.39 p75 0.40
    shrub    germ 12/12 est 12/12 est@ p50   3250 | end shoot p25    50 p50    59 p75   230 cv 0.95 | rgr/1k p25 0.13 p50 0.18 p75 0.29
    herb     germ 12/12 est 12/12 est@ p50   2500 | end shoot p25    59 p50    65 p75    68 cv 0.35 | rgr/1k p25 0.15 p50 0.16 p75 0.18
    creeper  germ 12/12 est 11/12 est@ p50   1500 | end shoot p25    54 p50    64 p75    78 cv 0.51 | rgr/1k p25 0.14 p50 0.16 p75 0.20
  life bright (width 96 frames 40000 n_est 20 lamp 1 rain Light soil 1): 5.65 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   2000 | end shoot p25  1663 p50  2472 p75  2583 cv 0.44 | rgr/1k p25 0.12 p50 0.12 p75 0.13
    shrub    germ 12/12 est 12/12 est@ p50   1250 | end shoot p25   370 p50   684 p75   765 cv 0.44 | rgr/1k p25 0.08 p50 0.09 p75 0.10
    herb     germ 12/12 est 12/12 est@ p50   1750 | end shoot p25   127 p50   191 p75   200 cv 0.45 | rgr/1k p25 0.05 p50 0.06 p75 0.07
    creeper  germ 12/12 est 12/12 est@ p50   1000 | end shoot p25    79 p50   110 p75   120 cv 0.26 | rgr/1k p25 0.04 p50 0.04 p75 0.05
  life dim (width 96 frames 40000 n_est 20 lamp 0.4 rain Light soil 1): 2.73 core-s a bed
    tree     germ 12/12 est 12/12 est@ p50   3250 | end shoot p25   437 p50  1319 p75  1742 cv 0.67 | rgr/1k p25 0.09 p50 0.11 p75 0.12
    shrub    germ 12/12 est 12/12 est@ p50   4250 | end shoot p25   182 p50   306 p75   519 cv 0.64 | rgr/1k p25 0.07 p50 0.09 p75 0.09
    herb     germ 12/12 est 12/12 est@ p50   2750 | end shoot p25   176 p50   219 p75   229 cv 0.50 | rgr/1k p25 0.06 p50 0.06 p75 0.07
    creeper  germ 12/12 est 11/12 est@ p50   1750 | end shoot p25    53 p50    76 p75    80 cv 0.58 | rgr/1k p25 0.03 p50 0.03 p75 0.04
```
