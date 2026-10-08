# The stack, one switch at a time, on steady food: the nest store starves the foragers (2026-10-08)

**Step A of the playtest plan, its last open question.** With `edible` on,
the owner's playtest stack holds a steady colony on renewable food, but
starves about 7x main's adults on the surface
([`../appetite-sweep-2026-10-08/README.md`](../appetite-sweep-2026-10-08/README.md) §3).
This takes each of the six switches out in turn to find which one does it,
then traces the starvers it adds and tests the store's reach. Also the first per-switch evidence for the
stack's default flip (plan step D), which so far had only the whole bundle
against main. Measured unless marked *inferred*.

## Setup

`steady_income` (40 cells of provisions every 1,000 frames, a colony of
220-300), 200k frames, mutation off, evolved founder, `food=0`, `hungry=1`,
seeds 1-4, binary `deeptrace` at main `6ffda5db` plus the appetite row (the
appetite sweep's). **all** is the owner's playtest line plus `edible`
(`NEEDS_FIRST=on,backfill CARRY_HOME=on DOOR_COLUMN=on LAY_BAR=body
NEST_STORE=on,pick=20,jaws,sky,meal,smell=10,edible WAY_FOOT=on`); each
**minus X** is that line with X unset (off); **main** is no switch at all.
Scripts: [`loo.sh`](loo.sh), [`looq.sh`](looq.sh); readers
[`looread.py`](looread.py) (the table), [`surfstarve.py`](surfstarve.py) and
[`lastspell.py`](lastspell.py) (the trace). Run data is not in the repo.

## Each switch out

Per seed: colony min / mean after 100k; adults starved (on the surface /
in the nest); larvae starved; eggs.

| arm | seed 1 | seed 2 | seed 3 | seed 4 |
|---|---|---|---|---|
| all | 221/269, 424 (380/1), 142, 2,038 | 225/268, 387 (341/9), 121, 1,948 | 244/272, 191 (171/3), 6, 1,677 | 243/275, 198 (195/1), 131, 1,880 |
| main | 263/295, 67 (45/2), 215, 1,931 | 213/285, 68 (28/6), 197, 1,927 | 231/277, 98 (62/2), 179, 1,911 | 215/267, 53 (15/6), 247, 1,895 |
| minus `NEEDS_FIRST` | 181/245, 413 (100/188), 103, 1,976 | 222/259, 267 (163/57), 263, 2,042 | 235/266, 344 (131/188), 143, 1,824 | 238/256, 298 (199/36), 301, 2,094 |
| minus `CARRY_HOME` | 203/248, 603 (587/2), 89, 2,041 | 186/228, 496 (473/1), 205, 2,077 | 184/221, 447 (431/3), 236, 2,008 | 159/225, 503 (486/0), 183, 2,173 |
| minus `DOOR_COLUMN` | 191/269, 404 (320/0), 5, 1,795 | 175/225, 249 (197/0), 131, 1,840 | 206/260, 329 (253/0), 5, 1,705 | 175/270, 198 (147/2), 6, 1,640 |
| minus `LAY_BAR` | 198/262, 481 (403/3), 76, 2,135 | 219/253, 375 (269/2), 160, 2,053 | 232/265, 235 (217/6), 127, 1,996 | 194/252, 515 (348/87), 102, 2,194 |
| **minus `NEST_STORE`** | **227/274, 114 (109/0)**, 261, 2,119 | **272/303, 186 (173/0)**, 237, 2,146 | **256/286, 112 (101/2)**, 210, 2,066 | **250/285, 92 (87/0)**, 255, 2,061 |
| minus `WAY_FOOT` | 196/245, 327 (212/108), 70, 1,909 | 162/240, 337 (170/163), 72, 1,894 | 218/255, 309 (184/103), 24, 1,808 | 217/245, 360 (309/36), 149, 2,024 |

- **Taking the store out cuts surface starvation on every seed** (380/341/171/195
  -> 109/173/101/87) and raises the colony's floor and mean on every seed
  (min 221-244 -> 227-272, mean 268-275 -> 274-303). Larvae starve more
  without it (6-142 -> 210-261, main's 179-247): the store feeds the brood.
- **`CARRY_HOME` is protective**: without it surface starvation rises on every
  seed (431-587).
- **`NEEDS_FIRST` and `WAY_FOOT` protect the nest**: without either, adults
  starve inside the nest again (36-188 a run against 0-9).
- `DOOR_COLUMN` and `LAY_BAR` move nothing consistently.
- Even without the store, surface starvation (87-173) stays above main's
  (15-62): something else adds a little, not traced.

## Who the store's starvers are

Every ant that starved on the surface, its last hungry spell (rows of
`hungry.csv` no more than 40 frames apart, ending at its death):

| seed | arm | surface starvers | where its last spell began (median, columns from the door; the heap is at +30) | began > 100 columns out | bit in the last 2,000 frames |
|---|---|---|---|---|---|
| 1 | all | 380 | **+32** (65 in the heap zone) | 88 | 1 |
| 1 | minus `NEST_STORE` | 109 | -72 | 39 | 0 |
| 2 | all | 341 | **+61** | 139 | 0 |
| 2 | minus `NEST_STORE` | 173 | -96 | 79 | 0 |
| 3 | all | 171 | **+36** | 11 | 0 |
| 3 | minus `NEST_STORE` | 101 | -78 | 43 | 0 |
| 4 | all | 195 | **+53** | 28 | 0 |
| 4 | minus `NEST_STORE` | 87 | -87 | 36 | 0 |

In their last 2,000 frames every one of them walked with no pull at all
(`none` 60-62% of decisions, `not scored` 38-40%), empty, on the surface, and
died at the box's edges, 120-260 columns out (seed 1).

- **Without the store, the few that starve are explorers lost on the far side**
  (their spells begin a median 72-96 columns west, away from the food).
- **With it, the extra ones go hungry on the food's own side**, at or past the
  heap, and walk off. The heap had been eaten (each 40-cell drop is gone within
  1,000 frames in every arm: the ground maps at each drop show 37-40 cells),
  and the food had gone into the nest store, but **the store's pull never
  reached them**: `nest store` is 0% of their pulls. The playtest line sets
  the store's `smell=10` (the pull acts only within 10 steps of the store), and
  the heap is 30 columns from the door.
- `smell` was built (2026-10-06) against the crumb trap: a store holding only
  crumbs pulled every hungry ant in to starve beside it. `edible` fixes the
  crumb trap itself, so `smell=10` may now only be cutting the store off from
  the foragers it should feed. *Inferred; under test below.*

## Tested: the store with a wider reach, and with none -- not the fix

The same line with the store's `smell=40`, and with no `smell` part (the old
reach: every hungry empty ant on the way in), seeds 1-4, 200k; same columns
as the first table (colony min/mean; adults starved, surface/nest; larvae
starved):

| arm | seed 1 | seed 2 | seed 3 | seed 4 |
|---|---|---|---|---|
| all (`smell=10`) | 221/269, 424 (380/1), 142 | 225/268, 387 (341/9), 121 | 244/272, 191 (171/3), 6 | 243/275, 198 (195/1), 131 |
| `smell=40` | 211/254, 243 (172/26), 74 | 238/259, 264 (225/1), 282 | 225/251, 134 (99/1), 68 | 212/245, 313 (210/2), 162 |
| no `smell` | 173/229, 358 (216/54), 224 | 200/239, 265 (154/42), 277 | 233/249, 218 (131/77), 50 | 227/254, 252 (180/43), 186 |
| no store | **227/274, 114 (109/0), 261** | **272/303, 186 (173/0), 237** | **256/286, 112 (101/2), 210** | **250/285, 92 (87/0), 255** |

- **A wider reach trades one starvation for another.** At `smell=40` surface
  starvation falls on 3 of 4 seeds, but the colony's mean is lower on all 4
  (245-259 against 268-275). With no limit the nest starves again (42-77
  adults inside a run, the crumb-trap shape even with `edible`) and the
  colony is smaller on all 4.
- **No store is the best arm on every seed** for the colony (mean 274-303)
  and for adults starved (92-186).
- So on renewable food the store, in the playtest's form, costs the colony
  whatever its reach: it moves food deep into the nest, which feeds the brood
  (larvae starved 6-162 with it, 210-261 without) and starves the foragers
  who are not near it.

## What this means for the stack and the default flip (plan step D)

- **`NEST_STORE` should not be in a default flip on this evidence**, and it
  is the one part of the stack that hurts on renewable food. The 12-seed
  heap-90 comparison (the bundle against main, store included) was one-sided
  for the bundle, so the store's cost there is not known: *a heap-90 run
  without the store is what decides whether the flip is the bundle minus the
  store*.
- `CARRY_HOME`, `NEEDS_FIRST` and `WAY_FOOT` each protect on steady food.
  `DOOR_COLUMN` and `LAY_BAR` move nothing consistently here.
- Feeding the brood is what the store does well. A store that feeds
  foragers too would need its food where foragers are (near the door, or a
  pull that reaches the heap's side), not a wider smell: a design question
  for the owner, not a retune.
