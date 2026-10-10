# Why ants still starve inside the nest with the new routes and the backfill on (heap 90, seeds 3 and 4)

Deep trace, 2026-10-07, about 21:00 UTC. The question came from Nest race's rerun (`results.md`, last section): with
WAY_FOOT on top of Nest building's backfill, seeds 3 and 4 still lose 145 and 260 ants inside the nest at 200-300k. Measured
unless marked *inferred*.

## The answer

**They starve beside store crumbs that no ant can eat.** The storeroom and an ant's mouth read "food" two different ways:

- **The store** counts every loose crumb with any food in it (worth above 0 J). That count decides whether the store "can
  feed" (8 cells or more), and every counted crumb is a destination of the store's pull.
- **The mouth** only sees a crumb worth more than about 15 J. For this ant's gut a crumb pays 0.81 of its worth, and the
  mouth's line is 12 J.

Late in both runs the store fills up with crumbs worth 0-15 J (mean 8 J). Those crumbs do two things:

1. **They keep the store reading full.** So the hungry are pulled to the store, and the walk out stays off for any hungry
   ant within smell of it, even when there is nothing there to eat.
2. **They draw the hungry to themselves.** The store's pull leads a hungry ant to the nearest counted crumb, edible or not.
   It stops there, with its mouth seeing nothing.

**Seed 4 is the empty-store case.** From 229k to 262k the store held 0-6 edible cells, often 0 J, while 25-58 inedible
crumbs kept it reading full. 249 of the 252 starvers died with fewer than 8 edible cells in the store (median 2). For all of
their hungry spell, the store read "can feed".

**Seed 3 is the wrong-crumb case.** The edible store never dropped below 8 cells during any starver's last hungry spell.
It fell to 3-10 cells at 235-238k, came back to 14-19 cells (5.8-8.2 kJ) by the 240k burst, and held 59-95 cells (28-45 kJ)
at 255-277k. But 128 of the 144 starvers died with no edible store cell within 10 cells of them and an inedible crumb within
10. Ten cells is the store's smell range, so the only store food they could smell was food they could not eat.

**Across both seeds:** 337 of 396 starvers died with an inedible crumb within 10 cells and no edible store cell within 10
cells.

**It is not the new routes.** Falls were 1.3-1.8% of the starvers' last decisions. None of them tried to leave: the walk
out was 0.7-4.3% of their decisions, against 16.6-18.8% for hungry ants in the nest that were fed.

![store crumbs](pictures/residual-store-crumbs-s3-s4.png)

`pictures/residual-store-crumbs-s3-s4.png` shows the store's lobe on the room's west wall: seed 3 at 200k, seed 3 at 240k,
seed 4 at 237.5k. **Cyan** marks store food an ant's mouth can see, **pink** marks store crumbs too small to see, and a
**white cross** marks where a starver last stood (seed 3 dying at 240-241.5k, seed 4 at 236-239k). At 200k on seed 3 the
lobe is cyan and nobody is dying. At 240k it is mostly pink, with the crosses along it and along the floor. The same
panels at full nest size are in `-whole.png`. Red is ants, cream the larva column under the door, yellow crumbs and food
as drawn by the nest map.

## How it was measured

**The runs.** Build `claude/nest-race-way-foot` 74a2f08a, plus my shut-in probe v4 and a new store-food census:
`tools/probe-v4-foodcensus-74a2f08a.patch`, measuring only. Heap 90 (`foodgap=90`), 300k, `RAYON_NUM_THREADS=1`, the env as
in `results.md`, with `NEEDS_FIRST=on,backfill` (`tools/runres.sh`, `tools/runfood.sh`).

**They match Nest race's runs exactly:**

- Ants at 300k: 647 / 748 (s3, WAY_FOOT off / on) and 722 / 736 (s4).
- Starved 20-300k: 1,040 / 148 and 434 / 269.

**The census changes nothing.** `foodevery=500` writes `storefood.csv`: every loose food cell in the nest region every 500
frames, with its material, worth, what this ant's mouth gets for it (`diet_yield` through the ant's own gut), whether that
clears the mouth's 12 J line, and whether the store counts it (`is_store_cell`). Over every frame the census reruns shared
with the first runs, their stats and death lines are byte-identical.

**When.** Starvation is a late burst. Per 10k frames (`stats.csv`), on the switch-on arm:

| window | s3 starved | s3 store bites | s3 eat pulls | s4 starved | s4 store bites | s4 eat pulls |
|---|---|---|---|---|---|---|
| 200-210k | 2 | 2,443 | 7,200 | 0 | 3,782 | 6,581 |
| 220-230k | 1 | 2,792 | 8,666 | 0 | 1,847 | 10,965 |
| 230-240k | 0 | 1,625 | 18,012 | 62 | 437 | 16,838 |
| 240-250k | 61 | 508 | 13,355 | 59 | 489 | 16,445 |
| 250-260k | 32 | 1,177 | 13,260 | 46 | 296 | 17,393 |
| 260-270k | 28 | 1,122 | 11,304 | 32 | 724 | 6,994 |
| 270-280k | 15 | 1,248 | 3,596 | 43 | 343 | 18,893 |

The signature is that the eat pull fires 2-3 times as often while store bites fall 3-8 times. Hungry ants were sent to the
store and did not eat there.

**Who.** These counts cover every lean spell (an ant under half its grant until it is back at its grant) that began below
the old ground line at 200-300k (`tools/hspells.awk`, `tools/spellfunnel.py`):

| | s3 | s4 |
|---|---|---|
| spells | 1,385 | 1,643 |
| stayed in the nest: fed / starved / other death / still hungry at 300k | 951 / 144 / 21 / 35 | 1,148 / 252 / 38 / 31 |
| left the nest during the spell: all fed | 234 | 174 |

**Starvers against the hungry ants that stayed in and were fed** (s3; s4 in brackets):

- **Where they started:** a median row 221 (202), the room's floor, against 167 (168), the top of the shaft.
- **Who they were:** nest workers, 136 of 144 (219 of 252). Most of the fed ones were newborns (647 of 951).
- **Spell length:** 2,190 (2,462) frames against 140 (195).
- **What pulled them,** as a share of their decisions:
  - store pull 29% (25%) against 12.5% (8%);
  - walk out 4.3% (0.7%) against 18.8% (16.6%);
  - falls 1.8% (1.3%) against 4.9% (6.9%).

**What they sensed.** Starvers had a food cell straight ahead in 142 of 144 spells (251 of 252), on 2,964 (17,471)
decisions. The brain's `FoodAdjacent`, the engine's own "is there food my mouth can take", was on for 7 (31) of them.

**What that food was.** Seed 3 decisions within 4 frames after a census, with the cell ahead looked up:

- 177 of 258 were inedible crumbs (40 counted as store);
- 52 were not loose food, which leaves corpses (*inferred*: the census excludes corpses);
- 22 were edible crumbs.

**The positive control is the engine agreeing with the census:**

- an edible store crumb ahead: `FoodAdjacent` on for 20 of 20;
- an inedible crumb ahead: off for 177 of 184 (the 7 had other food beside the head).

**The store, cell by cell** (`tools/storecensus.py`; full tables in `residual/tables.txt`):

- **Seed 3, 150-210k:** 96-168 store cells, of which 3-13 inedible. Nobody starved.
- **Seed 3, from 214k:** inedible crumbs rise to 30-55. Edible cells fall to 3-10 at 235-238.5k.
  - At the burst (239-241k), 1-5 of the 14-19 edible cells had an ant beside them.
  - The ants that died there died a median 1-3 cells from an inedible crumb and 13-27 from an edible one.
- **Seed 4, from 229.5k:** 0-6 edible cells (0-3,012 J) and 25-58 inedible, until about 262k.
- **The two kinds of crumb** (seed 3, 236-238k): inedible store crumbs were worth 0-15 J, mean 7.9 J (230 cell-samples).
  Edible ones had a mean of 383 J.

## The code where the two readings part (74a2f08a, `src/sim/creature.rs`)

- `loose_food` is "food worth above 0, owned by nobody, not a corpse". `fill_nest_store` builds the store's cells and its
  field's seeds from it, and so does `is_store_cell`.
- `store_can_feed` asks for `STORE_EAT_MIN` = 8 such cells at the last rebuild. It gates both the eat pull
  (`nest_store_pull`) and `store_feeds_here`.
- `store_feeds_here` makes `hungry_out_pull` return nothing: no walk out at the store.
- `adjacent_food_counted`, the mouth and the `FoodAdjacent` sense, keeps only cells whose `diet_yield` is over
  `EAT_YIELD_THRESHOLD` (12 J). `crumbs.ron` has food class -1 and carries its worth per cell. The founder's gut sits at -0.8,
  so a crumb pays 0.81 of its worth and needs about 15 J to be seen.

The earlier edibility check (`sky-meal/laybar-body-results.md`, heap 90 s4 to 134k) found 99% of the store's worth edible,
and so does this census up to about 210k. **By cell count**, which is what the gate and the field read, the inedible share
grows late: to over half the store on both seeds after about 214k.

## The switch-off arm

Seed 3 off loses 504 in the nest at 200-300k, and they are the same kind of ant:

- 459 are nest workers;
- every one stayed in the nest;
- the store pull is 44% of their decisions and the walk out 12.5%.

But its store during the bursts was small (6-14 cells, mostly edible, 0-6 inedible). I have not separated "held by
inedible crumbs" from "a store too small for the crowd" there. So I do not claim the off arm's deaths have the same
cell-level cause.

## Not traced

- ~~**Where the 0-15 J crumbs come from.**~~ **Traced at 22:40** (`../../store-edible-review-deep-trace-2026-10-07.md` §3,
  probe v5). The store's stubs are its own meals run down. A nest worker bites a small store crumb (median 26-28 J), holds
  it to its grant under `meal`, and puts back the last 8-10 J. That happened 189 (s4) and 184 (s3) times at 150-285k,
  fastest at 200-230k. Stubs above the ground line come from fed ants putting down a whole cell chewed to nothing, and
  diggers do not carry those into the store.
- **Why seed 4's edible store ran dry for 30k frames.** Brood rose to 550-610 then. Roughly (same review, §5): new food
  reaching the store was ~100-180 kJ per 10k, against more eaten out of it from 150k on. The flows do not balance the
  census exactly.
- **How the hungry ants that stayed in were fed.** 80% of those spells had no crop gain, so they were not fed by biting.
  Sharing from nestmates is the likely route (*inferred*: adult-to-adult shares are not logged).

## What this means for the WAY_FOOT verdict and the 12-seed bar

- **The residual deaths are not WAY_FOOT's routes.** They come from the storeroom's (`NEST_STORE`) reading of food, which is
  on the stack under both arms.
- **The on arm's bigger store** (about twice the store bites) carries more crumbs, but loses fewer ants than the off arm.
- **For the 12-seed bar:** a `foodevery=` run lets `storecensus.py` count how many of each seed's nest starvers died with
  inedible crumbs and no edible store food within 10 cells. That is the share this defect explains, seed by seed.
- **Who proposes the fix.** Per the standing rule, the fix is Nest race's to propose. It has to make the store count, seed
  and smell only food a mouth can take, or make the mouth and the store agree some other way. I can review it.

## Tools

All are in `tools/`, a few seconds to a few minutes per run.

- `hspells.awk`: every lean spell from `digrows.csv.gz`, with pulls, falls, held items, crop gains, and food ahead against
  food seen.
- `spellfunnel.py`: the funnel above.
- `nestdeaths.py`: one row per starver.
- `storecensus.py`: the store by edibility, with the starvers' nearest cells.
- `deathmap.py`: nest-map pictures with marks, in pure Python.
