# Two digging modes below ground, on the new stack: results (Nest building, 2026-10-07 21:5x; census 22:2x)

**Verdict (22:2x): INCONCLUSIVE, blocked upstream. Keep it off, and do not file it as a failure.**
- **99% of the modes' nest starvers died in the store's crumb trap** that Deep trace found (store counts crumbs too small
  to eat, so it reads "can feed" and keeps the hungry at it). That trap is on in both arms; the modes walk into it from
  about 80k, the base only late. Section "The crumb trap" below.
- **Rerun the pair on Nest race's crumb fix** (NEST_STORE `edible`) when it lands. The coordinator will say when.

**First reading (21:5x), still the measurements:**
- **The good part:** the modes change the nest's shape, and on 3 of 4 seeds they double the ant-time in the dug nest.
- **The cost:** the extra time is hungry ants. Deep ants starve in the thousands.
  - Starved: 2,597-5,280, against 9-269.
  - Nearly all of those were last seen deep in the nest.
  - The store underground is a fifth to two thirds of the size.
- **Chambers:** the chamber count rises from 1 to 2-5. In the pictures it is one central room ringed by a maze of short
  stubs (read from the pictures, not traced).

## The crumb trap (22:2x, measured)
**The question** (coordinator, 21:48): did the modes starvers die while the store read "can feed" with almost nothing
edible, or near crumbs too small to eat?

**Runs.** The modes arm again, seeds 1-4 to 300k, with Deep trace's measuring-only store-food census
(`nest-race/store-arms/way-foot/tools/probe-v4-foodcensus-74a2f08a.patch`, `foodevery=500`) and `hungry=1`; each
ant's last decision row kept ([`census/runfood.sh`](census/runfood.sh)). **Every stats row is byte-identical to the
first modes runs** (296 of 296 on each seed), so the census and logs changed nothing.

**Every ant that starved in the nest, 20-300k, at its last decision** ([`census/crumbcheck.py`](census/crumbcheck.py),
[`census/crumbcheck-modes.txt`](census/crumbcheck-modes.txt)):

| | seed 1 | seed 2 | seed 3 | seed 4 | all |
|---|---|---|---|---|---|
| starved in the nest | 4,378 | 5,200 | 4,325 | 2,545 | 16,448 |
| **empty store**: store reads "can feed" (8+ counted cells), fewer than 8 edible | 4,319 | 4,685 | 3,741 | 2,115 | 14,860 |
| **wrong crumb**: otherwise, an inedible crumb within 10 cells and no edible store cell within 10 | 48 | 466 | 541 | 377 | 1,432 |
| edible store food within 10 cells | 0 | 14 | 37 | 37 | 88 |
| none of those | 11 | 35 | 6 | 16 | 68 |
| median edible store cells at the death | 1 | 0 | 0 | 4 | |
| median distance to the nearest inedible crumb | 2 | 2 | 3 | 2 | |

- **So 16,292 of 16,448 (99%) died in one of Deep trace's two cases.** Its own headline count (inedible crumb within 10,
  no edible store cell within 10) is 4,047 / 4,950 / 4,101 / 2,369.
- **The store sits in the trap for most of the run** ([`census/storestate.py`](census/storestate.py),
  [`census/storestate-modes.txt`](census/storestate-modes.txt)). From 80k, 77-100% of census frames read "can feed" with
  under 8 edible cells, on a median 0-2 edible cells (seeds 1-3).
- **Seed 4 is the control inside the run.** At 160-200k its store recovered to a median 22 edible cells and was in the
  trap 8% of the time, and starving fell to 9 in 160-180k (from 109-394 per 20k either side). When the trap lifts, the
  dying stops.
- **The base** (Deep trace's census of the same stack, seeds 3 and 4): the store kept 8+ edible cells until about
  214k / 229k, which is when its late nest deaths began.

**What this leaves open (inferred, not traced):**
- **Why the modes' store runs dry of edible food at about 60-80k**, against about 220k on the base. More mouths deep
  in a smaller store is the obvious reading; supply against draw is not counted.
- **My two earlier leads** (widening drawn to loose food; the maze cutting ants off) may still be why the store is small,
  but they are not why the ants die. The ants die beside the store, reading it as full.

## What ran
- **Code:** branch `claude/nest-building-dig-modes-below`, on Nest race's `74a2f08a` (WAY_FOOT plus the backfill). It
  is the 10-05 re-port of the two modes (`nest-race/lane3/dig-modes-retest.patch`: `DIG_MODES`, `DIG_TIP`,
  `MODES_WHERE=below`, plus the off long-tunnel switches), all off by default.
  - deeptrace's `stats.csv` gains two counters: `modes_refused` (cuts refused as not a tip) and `modes_widened`
    (rolls turned to the wall beside contents).
- **Base env:** `NEEDS_FIRST=on,backfill`, `CARRY_HOME=on`, `DOOR_COLUMN=on`, `LAY_BAR=body`,
  `NEST_STORE=on,pick=20,jaws,sky,meal,smell=10`, `WAY_FOOT=on`, `RAYON_NUM_THREADS=1`.
- **Modes arm:** the base plus `DIG_MODES=on DIG_TIP=on MODES_WHERE=below`.
- **Runs:** `scenario=nest_goal founder=evolved ants=0 dig=1 mapevery=1000 foodgap=90`, seeds 1-4, to 300k. Script:
  [`run.sh`](run.sh).
  - **Two departures from the brief, both logging only:**
    - `hungry=0`, for disk. Shown logging-only on build 6 (identical stats).
    - The per-decision dig record (`digrows.csv.gz`, about 250 MB a run) went to /dev/null. This base has no
      `digfrom=`, and the first try filled the disk.
  - **Off = base, measured:** every new read is behind `dig_modes().is_some()`, and Nest race checked (21:47) that this
    base reproduces its own WAY_FOOT-on runs exactly: nest time 11.9/11.2/12.2/13.1%, starved 24/9/148/269, mean ants
    703/702/734/688.
- **The modes fire:** at 30k on seed 1 there were 46,589 refused cuts and 2,272 widened. At 300k the engine's digs were
  4,235-11,673, against 41,948-59,803 in the base.

## Measured, base / modes (seeds 1 / 2 / 3 / 4, 100-300k unless said; [`scorecard.txt`](scorecard.txt))

| | seed 1 | seed 2 | seed 3 | seed 4 |
|---|---|---|---|---|
| ant-time in the dug nest | 11.9% / 20.0% | 11.2% / 21.8% | 12.2% / 23.4% | 13.1% / 14.7% |
| ants deeper than 10 rows (of all) | 51 of 703 / 60 of 520 | 47 of 702 / 74 of 477 | 58 of 734 / 82 of 493 | 58 of 688 / 58 of 593 |
| ...of those, fed | 50.5 / **8.8** | 45.9 / **8.0** | 54.2 / **10.2** | 52.5 / **17.4** |
| mean ants | 703 / 520 | 702 / 477 | 734 / 493 | 688 / 593 |
| starved 20-300k | 24 / **4,428** | 9 / **5,280** | 148 / **4,385** | 269 / **2,597** |
| births | 3,427 / 5,340 | 3,385 / 5,831 | 3,523 / 5,578 | 3,451 / 4,525 |
| trip deliveries | 12,272 / 16,577 | 12,351 / 16,526 | 14,022 / 17,169 | 13,355 / 15,124 |
| stored food underground at 300k (cells) | 188 / 35 | 127 / 42 | 130 / 33 | 56 / 37 |
| corpses underground at 300k | 0 / 23 | 3 / 8 | 0 / 11 | 2 / 7 |
| biggest room at 300k (cells) | 3,940 / 997 | 4,396 / 651 | 4,076 / 977 | 3,392 / 1,672 |
| chambers at 100k / 200k / 300k (owner's rule) | 1/1/1 / 1/1/2 | 1/1/1 / 4/4/4 | 1/1/1 / 4/4/5 | 1/1/1 / 5/2/2 |
| door shut (maps) | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |

**Where the starvers were** (inferred from `colony.csv`: ants last seen under 60 J that vanished between 20k and
299k; the counts track `starved` within about 5%):
- **Modes:** 3,815 / 5,011 / 4,169 / 2,510 were last seen more than 10 rows down in the dug nest. Another 11-431 were
  higher in the nest, and 1-3 were in the mound.
- **Base:** 4-256 deep.

## Pictures ([`nest-s1.png`](nest-s1.png) to [`nest-s4.png`](nest-s4.png); base above modes, 50k to 300k)
- **Base:** one room that keeps growing, about 70 x 90 by 300k, with the brood column under the door and the store
  along its walls.
- **Modes:**
  - The nest stops growing at about 100-150k. It is a squarish block under the door, about a quarter the size (half on seed 4).
  - Inside it is a central room round the brood column, ringed by a maze of thin soil walls and stubs.
  - Little stored food, and corpses on the floor.
  - The extra "chambers" the counter finds sit in that maze. This is read from the pictures, not traced; a chamber
    counts only where a picture shows a passage between rooms.

## Not traced, and the lead (21:5x; answered at 22:2x by the crumb trap above)
- **Why deep ants starve under the modes.** Two readings, neither traced:
  - The store is where the widening goes. Loose food counts as "contents" (`is_contents`), so diggers are drawn to cut
    the walls beside the store (code-read).
  - The maze splits the deep nest so hungry ants can't reach the food. The door is never shut, so this would be
    inside the nest.
  - **Trace before any fix:** follow the deep starvers' last 3,000 frames on one seed: where they were, which pull
    they were on, and their walk distance to the nearest stored food.
- **More births and fewer larvae starved under the modes**, then the young adults starve deep. This is the same
  shape as the 10-05 retest's "young ants starve together" (inferred).
- Four seeds, one heap distance.
- **Nest race reviewed it (21:47): agrees, keep it off.** Its "two crowds" retest ends the same way (deep ants stop being
  fed, store bites 43-52k -> 3-5k, colony shrinks), which suggests both rules break the store's access (inferred).
