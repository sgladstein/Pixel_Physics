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

## The rung-2 re-fits on rung 4 (`rung2_refits_on_rung4.txt`)

The re-fits from `../refit/` (selected on juvenile growth alone) are bigger
here too, and **they cost the plant beside them**. End shoot p50 of a
neighbour, beside the shipped founder -> beside its re-fit:

| founder | tree neighbour | shrub neighbour | herb neighbour |
|---|---|---|---|
| tree | 1,346 -> 585 | 200 -> 102 | 118 -> 126 (flowered 7 -> 6 of 8) |
| shrub | 1,789 -> 1,224 | 367 -> 21 | 246 -> 186 (flowered 8 -> 5 of 8) |
| herb | 1,862 -> 1,429 | 312 -> 166 | 225 -> 226 |
| creeper | 1,568 -> 1,483 | 496 -> 267 | 230 -> 242 |

So rung 2 bred plants that win by crowding: the re-fit shrub takes a
shipped shrub beside it from 367 cells to 21, and costs a herb three of its
eight flowerings. This is what rung 4 charges for.

## Training on rung 4 from the shipped species (2026-10-22)

`plantrain control=X rung=hold stations=bright pop=16 gens=12 layouts=2
holdout=4 final=6 genes=8 sigma=0.9 seed=1`, beside the other three roles,
with `select=random` beside each (tree, herb and creeper on runner threads
with `/mnt/project-files/plant-community/runners/hold.sh`; shrub here).
Final set, 18 beds, champion minus shipped:

| role | score selection | random selection |
|---|---|---|
| shrub | +0.73 +/-0.38, ahead 14, behind 4 | +0.27 +/-0.21 |
| tree | +0.02 +/-0.05 | +0.06 +/-0.13 |
| herb | -0.02 +/-0.17 | -0.08 +/-0.21 |
| creeper | -0.04 +/-0.14 | -0.15 +/-0.15 |

**Only the shrub moved.** Its champion sets fruit within the 40,000 frames
(own score 3.0 -> 4.0 beside every role; the shipped shrub flowers and
stops) at about twice the shipped size, and costs its neighbours less than
the rung-2 re-fit did: a shrub beside it 367 -> 183 cells (the re-fit: 21),
a herb beside it flowered 7 of 8 (the re-fit: 5). `shrub-beside-herb.png`:
shipped, rung-2 re-fit and rung-4 champion, each beside a herb, frame 40,000.

Tree, herb and creeper are flat against shipped. The ladder's rule is that
a rung starts from the last rung's champion, and these started from the
shipped species, so the next runs start rung 4 from the rung-2 re-fits.

## Training on rung 4 from the rung-2 re-fits (2026-10-22)

The same runs, each starting from its role's rung-2 re-fit (`../refit/`)
instead of the shipped species (tree, herb and creeper on runner threads
with `/mnt/project-files/plant-community/runners/hold_from.sh`; shrub
here). Data in `from-refit/<role>/`. Final set, 18 beds, against the shipped
species as control:

| role | start (re-fit) | score champion | random champion | control | score champion - control |
|---|---|---|---|---|---|
| tree | 2.36 | 2.33 | 2.24 | 2.17 | +0.16 +/-0.19 |
| herb | 3.90 | 4.01 | 4.03 | 3.94 | +0.06 +/-0.11 |
| creeper | 2.42 | 2.39 | 2.27 | 2.31 | +0.09 +/-0.03, ahead 18 of 18 |
| shrub | 3.86 | 3.71 | 3.76 | 2.87 | +0.84 +/-0.23 |

**Nothing improved on its start.** Every re-fit already beats shipped on
rung 4, and twelve more generations of selection left each champion at or
just below the file it started from -- the random-selection champion lands
in the same place, so the score is not steering. The tree, herb and creeper
starts all score within 0.1 of the champions.

**Why: the neighbour term is too weak to push back.** The re-fits win by
crowding (the table above), and the champions keep doing it. End shoot p50
of the plant beside, from `from-refit/*/check.txt`:

| founder | shrub beside it | herb beside it |
|---|---|---|
| shipped tree | 200 | 118 (flowered 7 of 8) |
| tree re-fit | 102 | 126 (6) |
| tree, rung-4 random champion | 66 | 92 (3 of 8, 2 dead) |
| shipped shrub | 367 | 246 (8) |
| shrub re-fit | 21 | 186 (5) |
| shrub, rung-4 score champion | 182 | 219 (5) |

On the log size scale a founder that doubles itself gains about as much as
it loses for halving its neighbour, so crowding is close to free, and a
champion that takes a herb from seven flowerings to three still scores
level with the shipped tree. The plan's rule for a role is that *the others
do not pay for it*; the rung-4 score does not yet say that. Two ways on:
a floor, where a candidate that costs any neighbour more than some fraction
of its control size scores zero; or move straight to the mixed community
bed, which judges the whole set at once.

(`own score` in `check.txt` is the old bed score, which pays every
established non-flowering plant 3.00 whatever its size -- the trainer's
`sized_score` is what selection used.)

## The neighbour floor, and rung 4 again from the re-fits (2026-10-22)

`hold_score` is now multiplied by `min(1, kept / 0.5)`, where `kept` is the
neighbour's end shoot against its size beside the control
(`score::NEIGHBOUR_FLOOR`). It stays graded: a neighbour kept at a quarter
of its size halves the score. Before training, the floored score on the
plants already in hand (`floor_probe.txt`, 12 layouts beside every role;
shipped -> rung-2 re-fit -> rung-4 champion from shipped): tree 2.07 ->
1.94 -> 2.09, shrub 2.76 -> 3.00 -> 3.37, herb 3.57 -> 3.36 -> 3.58,
creeper 2.23 -> 2.13 -> 2.27. The crowding re-fits now score at or below
shipped, except the shrub. Shipped beds under the floor run 3-8 of 48 a
role: the noise the floor adds to the control itself.

Rung 4 again from each re-fit, with the floor (`floor/<role>/`;
`/mnt/project-files/plant-community/runners/hold_floor.sh`). Final set, 18
beds, against shipped:

| role | start | score champion | random champion | control |
|---|---|---|---|---|
| shrub | 3.23 | **3.43** (+0.72 +/-0.23, ahead 14 of 15) | 3.13 | 2.71 |
| herb | 3.42 | 3.39 (-0.34 +/-0.39) | 3.16 | 3.73 |
| creeper | 2.10 | 2.09 (+0.07 +/-0.06) | 2.12 | 2.01 |
| tree | 1.78 | 1.85 (-0.06 +/-0.17) | 2.00 | 1.91 |

**The floor steers, where the difference alone did not.** The shrub and herb
score champions both grew smaller than their starts and stopped smothering
the plant beside them (`floor/*/check.txt`): a shrub beside the trained herb
ends at 312 cells (beside its re-fit start, 118; beside the shipped herb,
302), and the random-selection herb, which kept its size, leaves it 81. The
trained shrub ends 381-622 cells against the start's 516-742. The tree did
not move: no child of the re-fit tree both kept its size and spared its
neighbour. The herb and tree champions are level with or behind shipped on
this score, so the re-fit was a poor place for them to start.
