# Food 90 cells from the nest instead of 30: what changes

Written 2026-10-06 by Laying, for Scott's question (19:35): *"I didn't realize the food heap was so close to our
nest. How much does it change things if it is 90 cells away, similar to our previous tests during the foraging loop
development?"*

## Answer

**Moving the heap from 30 to 90 cells away halves the colony and multiplies starving by 5-20. Deep nest time stays
near zero and gets lower: about 0.5 of ~270 ants live deeper than 10 rows, against about 3 of ~560 now.** The near
heap is not what keeps ants out of the deep nest. With the food far away, even fewer ants come into the nest at all.

- **Colony:** a mean of 246-308 ants over 100-300k, against 537-573. At 300k: 103-248, against 560-616. No colony died
  out.
- **Starved** (20-300k): 105-207, against 8-42.
- **Trip food** (forage trips delivered, 100-300k): 2,260-2,814, against 7,347-10,128. That is about a third.
- **Ants in the dug nest:** about 3, against 25-34.
- **Ants deeper than 10 rows:** 0.4-0.5 of 246-308 ants (door column 0.3-0.5, off it 0.0-0.1), against 2.5-3.0 of
  537-573 (door column 1.5-1.7, off it 1.0-1.3). Both are near zero.
- **Where the extra deaths are** (traced, ant by ant): about half are lost foragers. They walked west, away from the
  food, scouting with no pull and off any trail, never came back past the door, and starved out west, mostly at the
  far wall. On seeds 1 and 3 that is 197 of 359 starvers (55%); on seeds 2 and 4 it is 137 of 290 (47%). Most of the
  rest held a soil pellet in the mound (20-26%) or died in the nest. None died between the nest and the heap, and none
  at the heap.
- **Why the colonies fall** (seeds 1 and 3, measured): old age outruns births. Seed 1 went from 467 ants at 140k to
  121 at 200k with 501 old-age deaths, 228 births and 73 starved. Births fall because trip food falls (376 -> 115 ->
  120 per 20k).
- **Why less food comes home:**
  - Measured: round trips go on as often as with the heap at 30 (1,000-1,300 per 20k frames), but only 10-40%
    bring food, against 50-90%.
  - Traced on 12 ants an arm (seed 1, 100-105k): 7 of 12 fed ants on the mound top stood still on nearly every
    decision. They held only a few hundred joules, and at that level the brain's move output was negative. With the
    heap at 30, 11 of 12 such ants held thousands of joules and walked.
  - The mound top holds 36-41% of the colony, against 20-22%.
- **The foraging loop's own test still passes at 90 on today's game** (measured, 24 seeds): 23 of 4,833 ants starved.
  So no switch since then broke long trips there. What that test never had is the empty west: its nest sat about 48
  columns from a wall, while this box has 250 empty columns west of the door. That this is the difference is
  inferred until the wall test below is read.

**On the stack** (hunger-first + rest pull + homing + brood bearing), far food is just as bad, and two of four colonies
collapse:

- **Ants deeper than 10 rows:** 2.6-3.2 of 253-329 ants, against 8.0-11.4 of 510-568 with the heap at 30.
- **Starved:** 96-290, against 2-6.
- **Collapses:** seeds 2 and 4 fell to 9 ants at 300k.
- **Where they died:** hunger-first all but removes the mound pellet deaths (1-15 per seed). The west death stays and
  grows: 79-154 per seed died more than 60 columns west of the door.

## Where the heap sits, then and now

- **Now:** the goal box (`assets/lab_scenarios/nest_goal.ron`) drops one heap of provisions at x 286, at frame 6,000.
  The colony is placed at x 256, so the heap is 30 cells east of the door. `examples/deeptrace.rs` keeps that spot
  topped up to 120 cells. In the app the player tops it up by hand.
- **The foraging-loop tests:** the colony bed (`examples/trailfollow.rs`, `gap=`) put the food 90 cells from the
  nest by default and also tested 140 (`Reports/lanes/foraging-loop.md`, *Standing owner rulings* and *Baseline*).
  Scott remembered it right.

## The setting

`deeptrace foodgap=<cells>` moves the scenario's provisions heap and the spot `top_up` refills together, so no
stale heap is left at 30. Unset means the scenario's own 30. It is measuring only: the game and the app are
unchanged. It is a local commit on main f20864652 (`lay/food-gap` e654f0d03, not pushed); the patch is
`deeptrace-foodgap.patch` in this folder.

- **Identity:** with `foodgap` unset, seed 1 to 30k is byte-identical to the baseline 3ba1e7bd5 s1 (`stats.csv`,
  `colony.csv` and the 30k map). Main f20864652 is the same game as 3ba1e7bd5 (PR 646 changed only tools and docs).
- **Positive control:** with `foodgap=90` the FOUNDED line reads `food_x=346`. The 10k map shows the surface food at
  columns 336-363 and none at 286.

## Commands

Shipped game, every `PIXEL_PHYSICS_*` unset, `RAYON_NUM_THREADS=1`:
`deeptrace scenario=nest_goal seed=S frames=300000 founder=evolved ants=0 mapevery=1000 hungry=1 foodgap=90`.
The heap-at-30 arm is the shared baseline `/mnt/project-files/deep-trace/baseline/3ba1e7bd5/s1..s4`, the same command
without `foodgap`. Scored with `deep-trace/tools/scorecard.py --from 100000 --to 300000`.

## Scorecard, seeds 1-4, 100-300k (heap at 90 / heap at 30)

| | s1 | s2 | s3 | s4 |
|---|---|---|---|---|
| ants deeper than 10 rows, of all (door column / off it) | 0.5 of 246 (0.4 / 0.1) / 2.9 of 566 (1.7 / 1.2) | 0.4 of 283 (0.3 / 0.1) / 2.5 of 573 (1.5 / 1.0) | 0.5 of 258 (0.5 / 0.0) / 3.0 of 537 (1.7 / 1.3) | 0.5 of 308 (0.5 / 0.1) / 2.7 of 560 (1.7 / 1.0) |
| ants in the dug nest (share of ant-time) | 3 (1.4%) / 25 (4.5%) | 3 (1.0%) / 28 (4.8%) | 3 (1.1%) / 34 (6.4%) | 3 (1.1%) / 25 (4.5%) |
| ants at 100k / 200k / 300k | 337 / 121 / 103 vs 538 / 613 / 573 | 402 / 322 / 138 vs 548 / 560 / 616 | 439 / 140 / 248 vs 462 / 551 / 560 | 323 / 376 / 175 vs 526 / 564 / 587 |
| mean ants (lowest) | 246 (91 at 288k) / 566 | 283 (136 at 299k) / 573 | 258 (127 at 207k) / 537 | 308 (172 at 299k) / 560 |
| starved 20-300k | 207 / 42 | 105 / 8 | 158 / 15 | 190 / 12 |
| larvae starved | 445 / 848 | 395 / 963 | 455 / 680 | 360 / 858 |
| births / eggs | 1,116 / 1,563 vs 2,703 / 3,613 | 1,223 / 1,609 vs 2,684 / 3,634 | 1,129 / 1,605 vs 2,630 / 3,444 | 1,468 / 1,875 vs 2,613 / 3,638 |
| trip food (forage trips delivered) | 2,260 / 7,556 | 2,282 / 7,589 | 2,373 / 10,128 | 2,814 / 7,347 |
| nest / mound tunnels / mound top / heap / open ground | 1/43/36/0/18% vs 4/40/20/6/30% | 1/38/41/1/20% vs 5/40/22/6/27% | 1/38/41/0/19% vs 6/41/20/6/27% | 1/39/38/1/21% vs 4/43/20/7/26% |
| door shut (maps of 295; longest) | 16; 6 / 23; 8 | 40; 7 / 6; 2 | 23; 5 / 7; 3 | 29; 6 / 5; 1 |

Fewer larvae starve with far food only because there are fewer larvae: half as many eggs are laid.

**At 150k** (posted in the thread at 19:55; means over 100-150k): ants deeper than 10 rows were 0.5-1.5 of 291-414,
against 2.0-2.7 of 481-560. Starved by 150k: 24-83, against 0-23. Trip food: 458-1,041, against 1,765-2,718.

Nest pictures: `nests-s1-4.png`. Rows alternate heap at 30 / heap at 90 for seeds 1, 2, 3, 4; columns are 100k,
200k and 300k; each map is cropped to 60 columns either side of the door. The heap shows as green on the right in
the near-heap rows only, because at 90 it falls outside the crop. With far food the brood pile is thinner and the
room smaller. It is still one room.

## Where the extra deaths are (traced)

Reader `starvex.py`: the death line's place, for every ant that starved after 20k.

| | starved | west of the mound (x < 216) | nest and mound (x 216-296) | between mound and heap, or at it |
|---|---|---|---|---|
| heap at 90, s1 / s2 / s3 / s4 | 203 / 102 / 156 / 188 | 94 / 52 / 91 / 65 | 109 / 50 / 65 / 123 | 0 / 0 / 0 / 0 |
| heap at 30, s1 / s2 / s3 / s4 | 42 / 8 / 14 / 11 | 8 / 2 / 2 / 3 | 33 / 6 / 10 / 8 | 1 / 0 / 2 / 0 |

**The west starvers** (`weststarve.py`, `westfate.py`, ant by ant):

- **Who they were:** mostly ants that had foraged before (39-84 per seed). Their median lifetime deliveries were
  1-3.
- **Where they died:** at a median x of 37-53. The box's west wall is at x 4, and the heap is at x 346.
- **When they left:** last seen within 40 columns of the nest a median of 2,800-3,500 frames before death. They had
  about 200-225 J then, just at the grant.
- **How they walked:** in their last 3,000 frames, every scored decision carried the scout's pull out at full weight
  (2.00). On 65-71% of them the step chosen pointed away from home (median cosine to home -0.71). The scout had given
  up and turned for home on only 13-18%. They stepped west on 36-41% of decisions and east on 9-13%. There was no
  food trail under them on 87-98%. They carried nothing.

So the scout's walk out carries them away from home. On the west side there is no food and no trail to turn them,
and most never come back.

- **The same walk with the heap at 30:** more ants go hungry west of the mound (1,293 on seed 1, against 839 with
  the heap at 90). Only 12 of them starve. Most die of old age, 707 of them without coming back. On seeds 1-2, every
  hungry west decision carried the scout's pull on 62-72%, against 99% with far food. The far-food ants out west
  are also younger (median age 22-25k frames, against 31-34k) and leaner (0.81-0.89 of the grant, against 0.99).

**The mound starvers** (`starvewhere.py`, all four seeds):

- **Sealed in:** 48-94 per seed were trapped at their last sample: behind the shut door, in a closed pocket, or
  encased. 38-80 of them held a pellet when first found trapped. This is the known mound trap (findings.md,
  `CARRY_HOME` seed 2), now with more ants in it.
- **Off the map:** 53-95 starvers per seed. These are the west starvers.

## Why less food comes home (traced on 12 ants an arm)

Seed 1. I took 12 fed, empty ants standing on the mound top at 100k in each arm and logged every decision to 105k
(`ants=all only=…`, a world identical to the 300k run through 104k; reader `focal.py`, `idlewhy.py`).

- **Heap at 90:**
  - 7 of 12 stood still on 87-98% of their decisions. Their median move output was -0.51 to -0.80, and their
    median chance to step was 0.
  - Those 7 held 219-515 J at 100k, except one at 984 J that fell to 333 J. Four of them were nest workers.
  - 2 more stood still on about two thirds of their decisions.
  - The 3 that walked held 1,097-1,373 J and had their forage drive up (0.8-1.0). They delivered 2-5 loads in
    5,000 frames.
- **Heap at 30:** 11 of 12 walked on 59-73% of decisions, with median move output +0.05 to +0.75. They held
  1,600-17,800 J, except a nest worker at 246 J that ate its way to 1,044 J. The one that stood still held 618 J.

Over all focal decisions, by energy:

| | 200-400 J | 400-700 J | 1,000 J and up |
|---|---|---|---|
| heap at 90 | 7,284 decisions: median move output -0.75; chance to step 0 on 48% | 2,699: -0.74; 0 on 27% | 805: +0.75; never 0 |
| heap at 30 | 472 (mostly one nest worker): +0.60; never 0 | 1,004: -0.03; 0 on 57% | 9,744: +0.73; never 0 |

With the heap at 90, 83% of the focal decisions were made at 200-700 J. With the heap at 30, 81% were made at
1,000 J or more.

The still decisions also read a full crowd (crowding 1.00, against 0.83 for the steppers), carrying nothing, and
home not ahead.

So with far food, most ants live at a few hundred joules, and at that level a crowded, empty ant's brain keeps it
still on the mound top. What sets the move output from energy and crowding is the evolved brain's weights, which I
have not traced. This is 12 ants on one seed. It agrees with the scorecard's mound-top share on all four seeds, but
that link is inferred.

## Why seeds 1 and 3 fall (measured, `stats.csv`, per 20k frames)

| seed 1, heap at 90 | ants at end | births | old age | starved | trip food | round trips |
|---|---|---|---|---|---|---|
| 120-140k | 467 | 237 | 166 | 2 | 351 | 1,093 |
| 140-160k | 383 | 125 | 199 | 10 | 376 | 1,323 |
| 160-180k | 213 | 52 | 199 | 23 | 115 | 1,070 |
| 180-200k | 121 | 51 | 103 | 40 | 120 | 575 |

Seed 3 is the same shape: 439 ants at 100k fall to 140 at 200k, with 2-3x as many old-age deaths as births.

With the heap at 30, seed 1 runs 250-300 births against 250-280 old-age deaths in every window, with 567-1,165 trip
food. A boom's cohort ages out together, and with far food there is not enough food coming home to replace it.
Round trips (out at least 8 cells and back) stay as frequent as with the heap at 30, but most come back empty.

## Every starver sorted (seeds 1-4 and the stack at 90; `lostfit.py`, ant by ant, on its last hunger)

The pattern Redesign and Nest building traced (`needs-ant/depth-test/depth-test-2026-10-06.md`) is a **lost forager**.
Here that means one that died more than 6 columns west of the door, held no pellet for most of its last hunger, and
never came back east past the door's column in it.

| run | starved | lost forager | ...of them >60 columns west | pellet holder in the mound | in the nest | other |
|---|---|---|---|---|---|---|
| shipped, seed 1 | 203 | 110 (54%) | 94 | 41 | 25 | 27 |
| shipped, seed 2 | 102 | 49 (48%) | 46 | 26 | 12 | 15 |
| shipped, seed 3 | 156 | 87 (56%) | 84 | 33 | 21 | 15 |
| shipped, seed 4 | 188 | 88 (47%) | 58 | 48 | 11 | 41 |
| stack, seed 1 | 184 | 91 (49%) | 89 | 1 | 8 | 84 (57 >60 west) |
| stack, seed 2 | 286 | 106 (37%) | 88 | 10 | 1 | 169 (66 >60 west) |
| stack, seed 3 | 93 | 81 (87%) | 77 | 4 | 3 | 5 |
| stack, seed 4 | 182 | 108 (59%) | 95 | 15 | 0 | 59 (31 >60 west) |

- **Lost foragers, what they did and sensed** (seeds 1 and 3):
  - No pull acted on 95-98% of their last-hunger decisions.
  - The scouting walk was on 66-72%; the scout had given up on 25-27%.
  - They were off any trail on 95-96%.
  - 76-77% of their steps went west.
  - Median age at death was 26-28k frames.
  - Redesign's `starvetrace.py` on the same runs: the last hunger began a median 24 columns west of the door, at
    0.99 of the grant. 19 of 150 (seed 1) and 19 of 114 (seed 3) crossed the door's column during it.
- **Pellet holders:** spoil haul was the pull on 68-75% of their last-hunger decisions. This is the mound trap
  (findings.md). Hunger-first all but removes it on the stack.
- **Other** (seeds 1 and 3, 42): 36 went hungry in the mound's tunnels within 20 columns of the door without a pellet
  most of the time, and 6 died out west after crossing the door. On the stack, "other" is mostly ants that crossed the
  door and then went west again: 154 of its 317 died more than 60 columns west.

Per-ant rows: `scores/starvers-g90-s1.csv`, `scores/starvers-g90-s3.csv`; the sort: `scores/lostfit-*.txt`;
Redesign's trace: `scores/starvetrace-g90-s1.txt`, `-s3.txt`.

## The foraging loop's own test at 90, on today's game (measured)

This is bed B1 from `Reports/lanes/foraging-loop.md` (`trailfollow mode=gap gate=shipped frames=24000 ants=20 near=10
food=400 refill=400 arms=self gaps=90`, with the bed's env), 24 seeds, on main f20864652:

- Food taken: a median of 920 cells per seed.
- Starved: 23 of 4,833 ants that lived (0.5%), with a median of 0 per seed.
- Born: 4,353.

The closest saved log from 09-30 is the mute-off arm (`Reports/data/food-trail-mute-2026-09-30.tar.gz`): taken 319 per
seed, 30 starved of 780, 300 born. So long trips on that bed work at least as well today. Log:
`scores/bed-b1-gap90-main-f20864652.log`.

What the bed has that this box lacks:

- **A near west wall.** Its founders sit at x 30-68 and the nest at x 48, so a scout walking west meets the wall
  within about 48 columns. Here the nest is 252 columns from the west wall.
- **Scale and time.** It runs 20 founders for 24,000 frames, against 300-500 ants for 300,000.
- **No dug nest or mound.**

**Inferred, not yet measured:** the west room is what turns the scouting walk into a death. The scouting walk itself
(gain 2) has been on since 09-26, before the loop's baseline, so no later switch brought it in. The wall test below
measures this.

## Wall test: a wall 60 cells west of the door (measured)

`deeptrace westwall=60` puts a full-height wall at x 196. Shipped game (main f20864652 + foodgap + westwall, the
`westwall=` arg uncommitted in Laying's worktree), heap at 90 and at 30, seeds 1-4 to 300k, against the same runs
without the wall (`scores/w-100-300.txt`).

| 100-300k, wall / no wall | s1 | s2 | s3 | s4 |
|---|---|---|---|---|
| **heap at 90:** ants at 300k | 291 / 103 | 181 / 138 | 355 / 248 | 247 / 175 |
| mean ants | 316 / 246 | 244 / 283 | 306 / 258 | 288 / 308 |
| starved 20-300k | 130 / 207 | **267 / 105** | 92 / 158 | 50 / 190 |
| trip food | 2,919 / 2,260 | 2,248 / 2,282 | 2,953 / 2,373 | 2,244 / 2,814 |
| ants deeper than 10 rows (door column / off it) | 0.4 of 316 (0.4 / 0.0) / 0.5 of 246 | 0.2 of 244 (0.2 / 0.0) / 0.4 of 283 | 0.5 of 306 (0.4 / 0.1) / 0.5 of 258 | 0.5 of 288 (0.4 / 0.1) / 0.5 of 308 |
| **heap at 30:** ants at 300k | 525 / 573 | 594 / 616 | 564 / 560 | 570 / 587 |
| starved 20-300k | 32 / 42 | 10 / 8 | 9 / 15 | 3 / 12 |
| ants deeper than 10 rows | 2.2 of 533 / 2.9 of 566 | 2.6 of 553 / 2.5 of 573 | 2.9 of 605 / 3.0 of 537 | 2.9 of 563 / 2.7 of 560 |

**With the heap at 90 the wall helps on three seeds of four and not on seed 2.** On seeds 1, 3 and 4 fewer starved
(130 / 92 / 50 against 207 / 158 / 190) and the colony was bigger at 300k on all four. On seed 2, 267 starved against
105, and they died **at the wall**: 105 of them 50-60 columns west of the door. Sorted ant by ant (`lostfit.py`), 145
of seed 2's starvers were lost foragers, and on their last hunger they had **already given up on 95% of their scouting
decisions, and were on a trail on 76%**, stepping west and east evenly. **Inferred from those shares, not traced per
ant:** they laid a trail out to the wall, gave up on it, and the trail held them there. A give-up made on a trail
is not "spent" (`scout_dark`), so its pull home is scaled down by the trail and the away term still pushes it outward.
Deep time is near zero with or without the wall; with the heap at 30 the wall changes little.

**So the empty west room is part of the deaths, not all of it.** A short room stops the long walk, but a given-up ant
on a trail can still die at its end. The way home is now its own lane (WAY_HOME); Laying's first build and its
results are in `/mnt/project-files/way-home/lost-home-from-laying.md`.

## Stack (Nest race's arm) at 30 and at 90

NEEDS_FIRST=on, NEST_REST=workers, CARRY_HOME=on, BROOD_BEARING=1.9, rebuilt on claude/pack-behind 4ca0631c + the
brood-bearing commit + foodgap (`stack-food-on-4ca0631c.patch`). **Identity:** with the heap at 30 it reproduces Nest
race's stack exactly on seeds 1-3 (11.4 of 510, 9.2 of 529, 8.0 of 522 ants deeper than 10 rows; their
`inside/stack/deep-who-s1-3.md`).

| 100-300k, heap at 90 / heap at 30 | s1 | s2 | s3 | s4 |
|---|---|---|---|---|
| ants deeper than 10 rows (door column / off it) | 3.2 of 329 (2.9 / 0.2) / 11.4 of 510 (8.2 / 3.1) | 2.8 of 253 (2.6 / 0.2) / 9.2 of 529 (7.2 / 2.0) | 2.8 of 325 (2.7 / 0.1) / 8.0 of 522 (6.0 / 2.0) | 2.6 of 280 (2.3 / 0.2) / 10.9 of 568 (7.7 / 3.2) |
| ants in the dug nest | 17 / 65 | 14 / 53 | 14 / 54 | 11 / 68 |
| ants at 100k / 200k / 300k | 402 / 342 / 191 vs 509 / 534 / 502 | 296 / 309 / **9** vs 651 / 584 / 447 | 325 / 163 / 394 vs 577 / 597 / 545 | 429 / 342 / **9** vs 543 / 584 / 541 |
| starved 20-300k | 186 / 2 | 290 / 2 | 96 / 4 | 185 / 6 |
| trip food | 5,203 / 14,738 | 3,947 / 12,432 | 5,593 / 13,180 | 4,373 / 21,125 |
| nest / mound tunnels / mound top / heap / open ground | 5/49/31/0/15% vs 13/42/21/5/20% | 5/51/27/0/17% vs 10/43/24/5/18% | 4/47/30/0/18% vs 10/40/20/4/25% | 4/45/33/0/18% vs 12/44/16/5/22% |

At 150k (means over 100-150k): ants deeper than 10 rows were 2.7-3.3 of 305-367, against 5.9-10.6 of 494-582.
Starved by 150k: 17-96, against 0-2.

## What this means for the nest work (inferred)

- The near heap does not explain the empty deep nest. Moving it away lowers deep time further, because the colony is
  hungrier and smaller.
- 30 cells is an easy world: food is reachable from the mound itself. Lane results measured at 30 may not hold where
  food is far. The foraging loop's rulings were made at 90, and `HUNGRY_HOME` was found to help there (it is off; the
  foraging-loop note says it helps at 90 and kills at 140).
- The west scouts are the "lost foragers" pattern Deep trace and Redesign saw (homing for hungry unladen ants, in the
  backlog waiting for Scott's yes). Far food makes it the biggest single cause of death.

## Files (this folder)

- `food-distance-2026-10-06.md` (this file); `deeptrace-foodgap.patch`. **Parked 2026-10-06 21:50** (coordinator: every
  lane onto ants not living in the nest).
- `scores/`: the scorecards (`g90-100-300.txt`, `base-100-300.txt`, `g90-100-150.txt`, `base-100-150.txt`,
  `w-100-300.txt` for the wall test), `g90-starvewhere.txt` and `lostfit-w90.txt`.
- `tools/`: `starvex.py`, `weststarve.py`, `westfate.py`, `westall.py`, `mtop.py`, `focal.py`, `idlewhy.py`, `run.sh`,
  `runstack.sh`, `runfocal.sh`.
- `nests-s1-4.png`.
- Raw runs stay in the Laying container (`scratchpad/food/runs/`): g90-s1..4, st30-s1..4, st90-s1..4, foc90-s1,
  foc30-s1, w90-s1..4, w30-s1..4.
