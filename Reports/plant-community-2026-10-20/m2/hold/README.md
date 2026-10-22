# M2: rung 4, holding ground beside another role (2026-10-22)

Plan §6.2 rung 4: a candidate's whole life beside one shipped plant of each
other role in turn. Built in `train_plant::arena` (`Rung::Hold`,
`BedSpec::neighbour`, `Neighbour`) and scored by `score::hold_score`;
`plantrain rung=hold` trains on it, `plantrain mode=rungs rungs=hold`
measures it.

**The bed.** 56 wide, 40,000 frames, one lamp at column 28, the founder and
the neighbour 12 columns apart, mirrored about the lamp (the layout's seed
picks the founder's side). Two things were measured on the way and fixed:

- At 64 wide (the plan's) there are two lamps, at 18 and 46; a pair about
  the one nearer the middle put one plant 6 columns from the wall, and the
  plant there failed on every layout that put the founder on that side (tree
  peak 1 cell on 2 of 4, herb 4-107 cells against 180-247 on the other).
- Placed `gap` from a founder at the usual layout offsets, the neighbour
  sometimes stood between lamps and never established.

**The score.** The founder's own life, less whatever it cost its neighbour
below what that neighbour manages beside the shipped species of the
founder's role -- one step of the neighbour's life lost costs one step of the
founder's; a neighbour that does better earns nothing. The life is
`score::sized_score`: `bed_score`, except that an established plant that has
not flowered is paid its end size against the control's on rung 2's log
scale. **`bed_score` alone is flat for a perennial here**: the tree and the
creeper never flower in 40,000 frames, so every one of them read 2.999 --
55 cells and 1,900 alike.

**The shipped pairs** (`shipped_pairs.txt`, 8 layouts, base 7,000,000,
bright, main dc79ccd5 with soil water grading growth). Every founder and
every neighbour established on 8 of 8. End shoot p50 of the founder:

| founder \ beside | tree | shrub | herb | creeper |
|---|---|---|---|---|
| tree | 1,813 | 2,095 | 2,227 | 2,349 |
| shrub | 187 | 270 | 306 | 319 |
| herb | 154 | 181 | 213 | 190 |
| creeper | 103 | 109 | 107 | 111 |

The tree suppresses everything beside it (the shrub to 60% of its size
beside the creeper, the herb to 80%) and is barely touched itself; the
creeper is untouched by anyone. A herb neighbour flowers 7 of 8 beside a
tree and 8 of 8 beside anything else. That is the rung's signal: a role that
crowds its neighbour has something to lose here.

Also fixed on the way: the trainer's `seedbound` teeth arm cut only the
shoot tip's `Grew` rule, and the shrub's tip also grows on `Node`, so a
"seedbound" shrub scored the control's own score on 2 of 3 rung-2 beds and
the teeth check refused every shrub run on rung 4. It now cuts every rule
that grows a tip (rung 2: seedbound shrub 1.18 against control 2.30).
