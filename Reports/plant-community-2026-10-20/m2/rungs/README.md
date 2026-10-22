# M2, first look: rungs 1-2 on the four role species, by station (2026-10-21)

`cargo run --release --example plantrain -- mode=rungs species=tree,shrub,herb,creeper rungs=establish,grow stations=bright,dim,dry layouts=12`
(`rungs-bright-dim-dry035.txt`; `dry-025.txt` is the dry station at the
soil it ships with). 12 layouts, all under a lamp, world seeds 3,000,000+.
Rung 1 (establish) is 48 wide and 3,000 frames; rung 2 (grow) 48 wide and
10,000 frames. `n_est` 20 shoot cells.

## What a station does

| rung 2, end shoot p50 (CV) | tree | shrub | herb | creeper |
|---|--:|--:|--:|--:|
| bright | 599 (0.25) | 318 (0.55) | 102 (0.37) | 95 (0.29) |
| dim (lamps at 0.4) | 418 (0.73) | 81 (0.91) | 71 (0.35) | 61 (0.43) |
| dry (soil 0.25, no mister) | 688 (0.33) | 326 (0.57) | 106 (0.35) | 91 (0.35) |

Established by frame 3,000 (rung 1), bright against dim: tree 12 -> 9 of 12,
shrub 12 -> 5, herb 12 -> 12, creeper 12 -> 9.

- **Dim separates the roles, and in the wrong order for the plan.** The
  shrub is meant to be the shade-tolerant understory (§4.2) and is the one
  that suffers most in dim light: 5 of 12 establish, and it ends a quarter
  of its bright size. The herb, meant to be the open-ground colonist, is the
  most shade-proof. That is the role re-fit's first job, and the shade genes
  (A5, parked) are the lever it has.
- **Dry is not a station yet.** A young plant's growth ignores soil water
  above the germination threshold: soil at 1.0, 0.35, 0.25 and 0.15 grew the
  same herb (end shoot p50 116, 110, 106, against 102 under the mister), and
  at 0.12 and below nothing germinated at all. Water is a cliff with no
  middle, so `Station::DRY` stays as the instrument and out of the default
  set until a dry plant grows less.

## What a bed costs (§13's budget, measured)

| | core-s a bed | plan's estimate |
|---|--:|--:|
| rung 1, mister on | 0.66 | ~0.3 |
| rung 1, mister off | 0.04 | |
| rung 2, mister on | 2.4 | ~1.5 |
| rung 2, mister off | 0.13-0.17 | |

**The mister is about 94% of a small bed's cost.** Rung 1's outcomes with
it off (at field capacity) matched it on to within a cell or two, so the
cheapest lever on rungs 1-2 may be to run them without rain; rung 2 differs
more (shrub p50 318 against 362) and needs a paired check before it is used.

## Fixed on the way

- **An unsprouted seed counted as germinated** (`Census::germinated` read
  `shoot > 0`, and a seed is one shoot cell): 12 of 12 "germinated" where 4
  never grew. Now: more than the seed cell.
- **A seed between two lamps never sprouts in a narrow bed**, so the short
  rungs plant under a lamp only and leave light to the station. The plan's
  24-wide pocket could not hold a founder at all; 48 is the narrowest that
  did.

## Does each rung predict the next? (`ladder_herb.txt`, `ladder_shrub.txt`)

`plantrain mode=ladder control=SPECIES genomes=16 layouts=8`: 16 mutated
children (sigma 0.9, up to 8 genes, the shape/leaf/flower/height groups,
`mode=ranking`'s recipe) plus the shipped species, each grown on rung 1 and
rung 2 on two disjoint layout sets, bright and dim, and once on a whole life
(P1's 96-wide bed and score, 40,000 frames). End shoot cells, mean over 8
layouts. Spearman over 17 arms:

| | rung 1 repeat | rung 2 repeat | rung 1 -> 2 | rung 2 -> life | rung 1 -> life |
|---|--:|--:|--:|--:|--:|
| herb, bright | +1.00 | +0.90 | +0.58 | **+0.75** | +0.44 |
| herb, dim | +1.00 | +0.74 | +0.66 | **+0.72** | +0.60 |
| shrub, bright | +0.77 | +0.90 | +0.68 | **+0.78** | +0.73 |
| shrub, dim | +0.51 | +0.62 | +0.63 | **+0.71** | +0.49 |

- **Rung 2 is the filter.** It ranks children the same way on a fresh set
  (0.62-0.90) and predicts a whole life at 0.71-0.78, for about a fifth of
  the cost (2.4 core-s against ~12).
- **Rung 1 is too early for the herb.** Its +1.00 is the tidiness tell, not
  a good instrument: 12 of 17 herb arms read *identical* numbers at frame
  3,000, because almost no shape, leaf, flower or height gene acts that
  early. It ranks nothing it is not handed. For the shrub it does vary
  (repeat 0.77 bright), but it still predicts life worse than rung 2. So
  rung 1 survives only as a cheap death filter (did it establish at all),
  not as a ranking.
- **A shade-tolerant shrub is already in reach.** Child c09 grew 253 and
  174 shoot cells in dim light against the shipped shrub's 116 and 114,
  and 457/428 in bright against 288/312, with a better whole-life score
  (3.45 against 3.18). Random 8-gene steps find it; selection on rung 2's
  dim station should find better.
