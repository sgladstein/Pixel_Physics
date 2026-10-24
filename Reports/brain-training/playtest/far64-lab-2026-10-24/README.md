# The owner's far64 lab playtest — 2026-10-24

**What was played.** The draft PR #865 build (`4e80c202`, branch
`claude/project-thread-8y5ak0-playtest`): the lab's trained ant is farfood2's
best colony (ramp2far c64 r23), far sense `PP_HOMEVEC=100:100`. A hand-built
1024x512 box (no scenario), `ANT BRAIN: NEW` from frame 0, no plants at all
(`plants` 0 in every census row), food painted by hand. 224,802 frames.
Colonies placed at X 104, 480, 896 (F928-1002, ~51 each), then 856 (F25927,
40), 148 (F103346, 35), 786 and 540 (F171774, 31 and 39), 406 (F177639, 28).
Mutation set to 0 at F154138.

**The owner's notes:** *"The ants are surviving better, but still seem to
eventually die. Nests are staying ridiculously small relative to the number of
ants/food/brood in the colony. Sometimes the ants shift from foraging to just
living at the nest."*

**What the Teacher lane expected:** the colony lives with food about 100 columns
out, struggles past 150, dies at 200; nest digging below the morning pool's.

Files: `data/` holds the chronicle, its census CSV and actions CSV exactly as
uploaded. `deltas.py CSV col,col,...` prints per-10k-frame deltas of the
cumulative counters (births, deaths, starved, digs, deliveries, ...).

**Read from the census only.** The food's positions are not in the log (see
*The log bug* below), so food distance per colony is unknown and nothing here
traces individual ants. Every number is all colonies together; the census does
not split by colony.

## 1. "They eventually die": boom and bust, with no brake on laying

| frame | ants | food left (MJ) | born in 10k | starved in 10k | of which afield |
|---|---|---|---|---|---|
| 10k | 109 | 6.42 | 58 | 98 | 90 |
| 20k | 402 | 5.47 | 329 | 27 | 21 |
| 30k | 710 | 3.40 | 472 | 166 | 148 |
| 40k | 241 | 2.38 | 382 | 817 | 697 |
| 90k | 17 | 0.38 | 17 | 55 | 44 |
| 100k (food painted) | 67 | 2.17 | 97 | 46 | 32 |
| 120k | 313 | 0.96 | 389 | 291 | 258 |
| 160k | 28 | 0.28 | 65 | 203 | 137 |
| 170k (food painted) | 16 | 2.51 | 21 | 26 | 19 |
| 200k | 224 | 0.91 | 276 | 195 | 100 |
| 220k | 22 | 0.16 | 79 | 262 | 103 |

- **3,852 of 4,220 deaths (91%) by the last census were starvation**, 2,899
  of them afield, 509 near the nest, 444 under ground. Old age 271, killed 97.
- **Each food drop makes one boom and one crash.** Three drops (before F928,
  ~F90k, ~F165k, plus a smaller one ~F133k) give three cycles: 710 ants then 17;
  313 then 16; 224 then 22.
- **The first crash came with food still in the box.** 30k to 40k: 710 to 241
  ants, 817 starved (697 afield), and 2.4-3.4 MJ of provisions (roughly 2,500-3,500
  cells) was still lying in the box. The colony did not eat the box empty and then
  die. It outgrew what its foragers bring home.
- **Nothing slows laying.** `lays_declined` (a mind that could afford an egg and
  chose not to) is 0 in every row, and `births_denied_no_space` is 0. Larvae are
  almost all hungry in every row (118 of 118 at 70k).
- **Why training did not see this: the size brake is only in the harness.**
  Training runs with breeding on and `lay_cap=48:8` (`examples/inrun.rs`:
  the laying bar doubles for every 8 heads over 48). That brake is in the
  harness, not in the game. The lab has none, so the pool plays uncapped. The
  harness's own comment records what uncapped breeding did in the pilot:
  *"boomed to 150-200 ants, starved ~650 adults a round"*. That is this run.
- **Landing on 1 tank still kills most founders.** 87 of the first 154
  founders died within ~1,500 frames of being placed, without breeding (the
  `LINE ENDED WITH ITS FOUNDER` block at F1463-2507). The F25927 drop lost 30 of
  40 by F27383. This is the thin start `labsurvive grant=` already tests:
  a lab colony lands on 1 tank, training founders get 3.

**Against the expectation.** "Survives better" holds: lines of the first drops
lived to F114k-F130k (GORSE, ACORN, KUDZU, 5-9 generations), and the box was
not empty of ants until fed again. "Eventually dies" is not a far-food death in
this run: it is the colony breeding past its income every time it is fed.

## 2. "Nests stay ridiculously small"

| frame | ants | under ground | dug open (cells) | dug back to soil | mound (cells / rows) | digs per ant per 10k |
|---|---|---|---|---|---|---|
| 10k | 109 | 14 | 118 | 35 | 164 / 5 | — |
| 30k | 710 | 17 | 230 | 129 | 361 / 13 | 0.98 |
| 60k | 156 | 29 | 457 | 202 | 564 / 21 | 1.03 |
| 120k | 313 | 17 | 573 | 295 | 720 / 15 | 1.16 |
| 220k | 22 | 5 | 906 | 513 | 886 / 9 | 1.20 |

- **About one dig per ant per 10,000 frames**, the whole run (0.6-3.2).
  A 7-tall room 20 wide is 140 digs.
- **Every colony together kept ~900 open dug cells in 220k frames**: about one
  7-tall room 130 wide, shared across up to 8 colony sites. 513 of the 1,784
  cells ever dug went back to soil.
- **The brood lives in a pocket by the door.** Median brood depth 5-7 rows;
  `brood_near_door` (within 4 columns of the door) is most of the brood in
  every row (121 of 156 at 150k). Much of it sits in a 40-157 cell room not joined to the biggest open
  room. That biggest room (1,110-1,150 cells, 0 brood in every row) was
  already there at 10k and barely grew, so it is not the ants' work.
- **The spoil goes on top.** `spoil dumped` 285 in the first 10k, and the mound
  above the surface grew to 886 cells and 22 rows high. `spoil kept inside`,
  `held below` and `lifted` are 0 all run.

**Against the expectation.** Known gap, larger than "below the morning
pool". The digging rate is what makes the nest small. The ants do dig, just
about once per 10,000 frames each.

## 3. "Sometimes they shift from foraging to just living at the nest"

| frame | ants | under | mound top | afield | hungry on mound top | mean fill: mound top / afield |
|---|---|---|---|---|---|---|
| 10k | 109 | 14 (13%) | 57 (52%) | 38 | 0 | 4.98 / 3.36 |
| 20k | 402 | 8 (2%) | 236 (59%) | 158 | 4 | 2.95 / 2.86 |
| 30k | 710 | 17 (2%) | 290 (41%) | 402 | 98 | 1.77 / 2.76 |
| 70k | 95 | 9 (9%) | 36 (38%) | 50 | 2 | 4.18 / 4.22 |
| 180k | 77 | 2 (3%) | 40 (52%) | 33 | 1 | 3.21 / 3.36 |
| 190k | 167 | 15 (9%) | 81 (49%) | 70 | 2 | 3.71 / 3.81 |

- **The ants at home are on the mound, not in the nest.** 2-27% of ants are
  under ground at any census; 6-62% stand on top of the spoil mound.
- **Most mound-sitters are fed.** Their mean fill is the same as afield ants',
  and only 0-9 of them are below their start energy, except at 30k (98 of 290),
  the peak just before the crash. So this is mostly fed ants idling on the heap,
  not hungry ants failing to leave. The 10-22 playtest lane saw the same thing in
  the in-run bed: *"the same genome ends up as a heap resident or a
  stay-at-home"*.
- **What the census cannot say:** whether a given ant stopped foraging, or the
  colony's mix of ants changed. Telling those apart needs a per-ant trace on
  this box with the food where the owner put it (the method rule: trace
  individuals, not a population statistic). That is the open next step.

## The log bug: painted food was never written to the chronicle

The actions log holds 32 lines and no `PAINTED FOOD`, yet `fruit_j` jumps by
+1.8 MJ (90k to 100k), +0.44 MJ (130k to 140k) and +2.2 MJ (160k to 170k), each
one right after a `PAUSED`/`RESUMED` pair. The cause is `Lab::release`. It
ended a live stroke with `self.stroke.take()`, which drops the stroke without
calling `finish_stroke`. A left-button release inside the window always goes
there, so the food, soil and dig brushes never logged from play. Only the
right-button eraser and a release outside the window, which go through
`end_stroke`, ever wrote a line. The release path dates from 2026-08-30
(`7578f8f6`, the box's first verbs). The 10-08 fix (`7429327e`) added the line
to `finish_stroke`, which that release skipped, so painted food has never been
logged from a real mouse. No chronicle stored in the repo has a single
`PAINTED FOOD` or `ERASED` line. The 10-08 test closed the stroke with
`end_stroke`, so it never took the path a mouse takes. **Fixed in PR #867, the
same PR as this report**: `release` now calls `finish_stroke`, and the test
closes the food stroke with `release`. PR #869 then logged the eight other
verbs that changed the box silently (alarm, fling, lamps, roster culls, rain,
plant pace, fruit rot); owner, 2026-10-24: *"Player actions should all be in
the data."*

## For training (sent to the Teacher lane)

1. The lab has no `lay_cap`. A pool selected under the harness's size brake has
   never been scored on an uncapped colony. Either put a brake in the game
   (the shipped switch would be `PIXEL_PHYSICS_LAY_BRAKE` or a port of
   `lay_cap`), or score the `+lab` case on starvation after the peak, not only
   on alive/lived.
2. The landing grant (1 tank in the lab, 3 in training) still kills ~56% of
   founders in the first ~1,500 frames.
3. One dig per ant per 10k frames, with the spoil on the mound and the brood in
   a pocket by the door, is the nest gap the owner sees.
4. Mound-sitting, fed and idle, is 30-60% of the colony for most of the run.
