# Why hA1 ants still starve, after selection too (2026-10-22)

For the Teacher lane. Picture: `why-ants-starve-2026-10-22.png` (same folder).

## Setup

- Compute branch `claude/project-thread-8y5ak0` at 98ad9dc4, pack move on (its default). The only additions are a
  read-only decision log and the home sense written into `trace.csv`: `dlog-trace-anchor-vs-98ad9dc4.patch`.
  - `PP_DLOG=<file>` writes one row per decision: frame, id, x, y, energy, crop, pellet, chosen verb and target,
    options offered per verb, job scores, the 22 context readings.
  - `trace.csv` gains `ax,ay` (the `forage_anchor` the mind reads as home).
  - Determinism check: rerunning hA1 with the trace change gave an identical dead-ant table.
- In-run bed, runl.sh settings, selection off, 200k frames, 24 ants per colony, 16 colonies per arm. Three arms:
  - **hA1**: founder plus its 30 mutants, as in runl.sh. Seeds 1 to 16.
  - **Selected**: round-6 ants (`r5/final`) of shortr-1022's 6x200k arm. Colonies 2, 3, 4, 5, 7, 8, 9, 11, 12,
    13, 14, 16, 18, 19, 20 and 21 (1, 6, 10, 15 and 17 were extinct or nearly). Top 30 by score, as rounds.sh
    picks them, in a fresh world with seed = colony number.
  - **Old ant**: `nomind=1`, seeds 1 to 16.

| Arm | Starved (of 384) | Home deliveries per colony |
|---|---|---|
| hA1 | 82 (5.1 per 24) | 251 |
| Selected | 65 (4.1 per 24) | 3,860 |
| Old ant | 1 | 792 |

These match the common-garden numbers (unselected about 256 home; selected about 3,900 home and about 5 starved).

## Ranked causes (every starved ant, its last 2k frames of decisions plus its life)

| # | Cause | hA1 | Selected |
|---|---|---|---|
| 1 | **Walked out past the food and never came back**, empty, and starved at the box's side wall or in the open field | 41 (50%) | 40 (62%) |
| 2 | **Died at home with a soil pellet in its jaws** that it could not put down, so it could not eat: in a colony famine (hA1), alone (both), or in the first 6k frames of the round (selected) | 27 (33%) | 23 (35%) |
| 3 | Other: pellet out in the food band, or empty near home | 14 (17%) | 2 (3%) |

Selection removed most of the famine bursts (hA1 seeds 3 and 10, 25 deaths). It did **not** touch the roamers.

### 1. Roamers: no way home when empty

- **Where they die.** 35 of 41 (hA1) and 35 of 40 (selected) died at the side walls, about ±124 columns. In
  their final trip they spent about 70% of their decisions within 13 columns of a wall, climbing it or pacing
  along it. The food is at 25±10 columns.
- **They leave fat.** The tank when last inside 40 columns was p50 48 (hA1) and 35 (selected), p90 about 100.
  Then they lived 51k (hA1) or 42k (selected) frames out there, p90 over 100k. Example: hA1 seed 1, ant 4
  reached 85 tanks, walked to the wall at frame 25k, and sat there until it starved at 97k.
- **Their home sense was right.** 81 of the 82 ants that starved past 60 columns (both arms) had `forage_anchor` within 15 columns
  of the nest at death. They
  knew where home was and did not use it.
- **What they chose.** Walk 85 to 99%. Of the chosen walks, homeward and outward steps were equal at every
  tank level (homeward 30% / out 28% at 20+ tanks, 25% / 25% under 2 tanks). There is no homeward pull at all,
  and none appears as hunger rises.
- **The same holds for every ant out there, not only the dead.** Past 60 columns, the net homeward bias of chosen
  walks:
  - empty, both arms: -2% to +2%;
  - holding a pellet: +0 to +1%;
  - holding food: -3% to -23%, so food carried that far travels outward.
- **Inside 40 columns, only food pulls home**, at +6% to +8.5%. An empty ant in the food band is pushed slightly
  outward (selected -8%): that is the "go out when hungry" lesson, which ends at 40 columns.
- **The old ant on the same worlds** made 2,644 trips past 60 columns and 2,626 came back (99%).
  - hA1: 457 trips, 336 back (74%).
  - Selected: 342 trips, 246 back (72%).
  - The old ant's homing pull is crop fill (`home_weighted_pick_why`). It keeps food in the crop, so it is tied to
    home. The new ant empties its crop in the field and then has nothing pulling it back.

### 2. Pellet jam at home: jaws full, nothing to do

- They held the pellet p50 6.6k frames (hA1) or 2.9k frames (selected), p90 90k and 51k, and were often fat when
  they picked it up (p90 61 to 83 tanks).
- **Drop was offered in only 12% (hA1) and 2% (selected) of their last-2k decisions**, and never at all for 4 and
  9 ants. When it was offered, it was almost never taken: 7 times in all for hA1.
- PickUp and Eat were offered about 0% of the time, because the jaws are full. Dig is not offered with a pellet
  unless the ant is walled in.
- So they walk, often up and down the same 3 to 4 rows of a shaft (selected: walks up 2,871, down 2,089).
- **The store held food at every one of these deaths.**
- **Selected start-of-round deaths (15).** These are founders that begin at 3 tanks, dig at once (0 field bites)
  and die holding a pellet by frame 2k to 6k. Colony 9 lost 7 this way.
- This is the jam the foraging lane traced on c1. PP_NEEDDROP (a hungry ant may put its pellet down off open
  ground) is aimed at it, and is still waiting on the owner.

## What this suggests (not built)

1. **A "come home when empty and far" lesson for the founder.**
   - Rule: empty crop, past about 40 columns (or N frames since the nest) → take the step nearest home, more
     strongly as the tank falls.
   - This mirrors the old ant's crop tether and the hungry lesson's "go out to 40 columns".
   - The home sense is already correct, so the lesson only has to make the scorer read `f::HOME`.
   - It targets 50 to 62% of the deaths. Selection cannot find it by itself, because no mutant shows a homeward
     pull to select.
2. **Release the pellet jam:** PP_NEEDDROP, or a hungry "put it down anywhere" Drop. That covers about a third of
   deaths, including the start-of-round ones.
3. Both stack on hA1 and touch neither digging nor the score.

## Files

- `ranked.txt`: cause table and roam stats.
- `dead-hA1.txt` and `dead-selected.txt`: one row per dead ant with its choices and offers.
- `tools/`:
  - `dead.py`, `rank.py`, `roam.py`, `heading.py`, `heading_all.py`, `jam.py`, `pic.py`;
  - run scripts `run.sh`, `runsel.sh`, `runold.sh`. Paths are this session's scratchpad: edit `S=`.
- Selected genomes come from the bucket: `runs/shortr-1022/jobs/hA1r200_K.tgz`, `r5/final`.
