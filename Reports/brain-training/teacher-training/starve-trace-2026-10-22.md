# Why hA1 ants still starve, after selection too, and a come-home lesson that did not take (2026-10-22)

**Status: finding of record; no genome recommended.** Thread "Why selected ants starve" (project chat, 2026-10-22). Everything referenced here is in [`starve-trace-2026-10-22/`](starve-trace-2026-10-22/). The patches are scratch, env-gated (off unless the variable is set) and are **not applied to `src/`**: they apply to the compute branch `claude/project-thread-8y5ak0` (98ad9dc4) and to 9d19121c with the foraging lane's patch respectively.

For the Teacher lane. Picture: [`why-ants-starve-2026-10-22.png`](starve-trace-2026-10-22/why-ants-starve-2026-10-22.png).

## Setup

- Compute branch `claude/project-thread-8y5ak0` at 98ad9dc4, pack move on (its default). The only additions are a
  read-only decision log and the home sense written into `trace.csv`: [`dlog-trace-anchor-vs-98ad9dc4.patch`](starve-trace-2026-10-22/dlog-trace-anchor-vs-98ad9dc4.patch).
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
2. **Release the pellet jam.** PP_NEEDDROP is not it: the foraging lane tested it on hA1 at 32 seeds, with no change (4.0 vs 4.3 died). The Teacher lane has put a body-side fix to the owner. That covers about a third of
   deaths, including the start-of-round ones.
3. Both stack on hA1 and touch neither digging nor the score.

## The come-home lesson (14:00 to 15:46Z)

**No genome is recommended.** The rule works when the teacher acts, but all three copies (hm1, hm2, hm3) do worse than hA1 in the bed. The owner chose "neither yet" on the Teacher lane's card (15:40Z, no sense bit and no body tether for now). Attempt 2 below had already been running when that came in. It is recorded so that nobody re-runs it.

### The rule

- Switch: `PP_HOMEFAR=R` in `carry_label`, file [`come-home/homefar-farctx-vs-9d19121c-with-shadow-rim.patch`](starve-trace-2026-10-22/come-home/homefar-farctx-vs-9d19121c-with-shadow-rim.patch), which carries both switches. It applies to
  9d19121c, with the foraging lane's shadow-rim patch already in it.
- When: the ant has no food in its crop, no pellet in its jaws, and is more than R columns from the founding hole.
- What: take the Walk that brings it nearest home's column. With none (a wall face), take the step down.
- Ceiling: PP_CARRY=1 makes the teacher act. Labels only: PP_CARRY=0 with PP_COPY_LOG.

### Results (bed, runl.sh settings, pack on, selection off, seeds 1-32; `results-32-seeds.txt`)

| Arm | Starved per 24 | Starved past 60 cols | Collapsed (12+ dead) | Home food | New cuts | Time past 60 cols |
|---|---|---|---|---|---|---|
| hA1 (base) | 4.31 | 2.69 | 2 / 32 | 269 | 320 | 15.9% |
| Teacher acting, R=45 (16 seeds) | 3.44 | 0.81 | 1 / 16 | 289 | 268 | 2.3% |
| Copy hm1 (R=45) | **19.97** | 0.25 | **32 / 32** | 82 | 173 | 0.4% |
| Copy hm2 (R=60, plus food-band anchor) | **8.59** | 1.34 | **11 / 32** | 235 | 247 | 3.1% |

- **The ceiling is good.** Roamer deaths fall by 70%, food goes up, and digging falls somewhat (320 to 268 new cuts).
  Total deaths go 5.12 to 3.44 at 16 seeds, but paired by seed that is 5 fewer and 6 more, so it is noise-level.
- **hm1 leaked.**
  - Labels came from hA1's own states (6 colonies, seeds 41-46) plus soil, fetch and trail keep logs. Config is
    hm1.json, the same as hA1.json apart from the start genome and the logs.
  - The lesson took (overlap 0.48 to 0.95), but every empty ant learned to stay home. Time in the food band fell
    from 19k to 5k trace rows per colony, and the colonies starved beside the nest.
- **hm2 still leaked.** Its config adds `none_lookalike: [0.25, 0.62]` to the copier (in `copier_c.py` here). That
  puts unlabelled empty ants whose home distance reads inside the food band into an anchor group, "noneL", at
  weight 20. The lesson took only weakly (overlap 0.49 to 0.63). Roaming halved, but deaths doubled.

### Why I think it leaks (a guess, not measured)

- The trigger has to be read from `c::HOME_DIST` = squash(dist/32). That reads 0.44 at 25 columns, 0.56 at 40,
  0.65 at 60 and 0.76 at 100: a shallow ramp.
- The scorer's walk feature `f::HOME` also scales with the same squash. So "far" and "in the food band" are barely
  separable, and the copy moves the whole empty-ant policy.
- This is the same shape as the dig-copy finding (depth rows 7 and 8 differ by 0.03 and the copy collapsed until a
  sharp "bottom in reach" sense existed).

### Options for whoever picks this up

1. **A sharper sense.** For example, a context bit "more than 60 columns from home", on a borrowed slot as the
   PP_BOTTOM_CTX test did. Then re-copy. This is the likeliest fix.
2. **Ship the rule as a body or game-side tether instead of a lesson.** This would be the owner's call: it is a rule,
   not learned. The ceiling numbers above are what it would buy.
3. **Stronger anchors for the copier.** A noneL group of hundreds of thousands of rows rather than 7k sampled ones,
   with the full log kept for empty ants, plus a cut of the carry weight.


### Teacher lane, 15:15Z
Read. I put the choice (sharp far-from-home sense vs body tether vs neither) to the owner on a card, recommending the sense. Building and testing the sense bit in scratch (env-gated, then a re-copy at 32 seeds) is fine to start now; I will forward the answer by file here.

### Teacher lane, 15:40Z
Owner chose "Neither yet" on the come-home card (no far-from-home sense, no body tether for now). Please stop any work on the sense bit; roamers wait.

### Attempt 2, the sharp sense (15:05 to 15:46Z): also not recommended
- **The switch:** `PP_FAR_CTX=60` puts "more than 60 columns from home" on context slot 9 (d_alarm), which reads 0 on
  every bed decision. It is in `homefar-farctx-vs-9d19121c-with-shadow-rim.patch`, which supersedes the first patch.
- **The control caught a problem.** hA1 has 16 live links reading slot 9, so with only the sense on, starvation rose
  from 5.1 to 8.7 per 24 (16 seeds).
- **The fix:** `hA1z.raw.txt` is hA1 with those 16 links zeroed. With the sense on it matches hA1 at 32 seeds (3.75
  vs 4.31 starved, paired 17 fewer and 12 more). It is not bit-identical: the bed's mutants and the brain tax both
  follow the live-link count.
- **hm3:** copied from hA1z with the sense as a context input (`hm3.json`). The copy fit well: lesson overlap 0.43
  to 0.95, everything else 0.96 to 0.93.
- **But in the bed it is still worse than hA1:**

| Arm | Starved per 24 | Starved past 60 cols | Collapsed | Home food | New cuts | Time past 60 cols |
|---|---|---|---|---|---|---|
| hA1 | 4.31 | 2.69 | 2 / 32 | 269 | 320 | 15.9% |
| hA1z with the sense (control) | 3.75 | 2.19 | 1 / 32 | 247 | 308 | 12.8% |
| hm3 | 6.81 | 2.66 | 5 / 32 | 273 | 297 | 5.1% |

  Time spent far out fell by two thirds, but roamer deaths did not fall at all, and total deaths rose.
- **What this suggests (not traced):** the ants that still die out there are not the ones the rule reaches. They may
  hold pellets or food, or sit on wall faces where no step goes homeward or down. The extra deaths are somewhere else.
- **Next step for whoever picks this up:** trace hm3's dead ants with `tools/dead.py` and `rank.py` before any
  further copy.


## Files (all in `starve-trace-2026-10-22/`)

- **The trace:**
  - `ranked.txt`: cause table and roam stats.
  - `dead-hA1.txt`, `dead-selected.txt`: one row per dead ant, with its choices and the options it was offered.
  - `tools/`: `dead.py`, `rank.py`, `roam.py`, `heading.py`, `heading_all.py`, `jam.py`, `cls.py`, `pic.py`.
  - Run scripts `tools/run.sh` (hA1), `tools/runsel.sh` (selected colonies) and `tools/runold.sh` (old ant). Their
    paths point at that session's scratchpad: edit `S=`.
  - Selected genomes: bucket `runs/shortr-1022/jobs/hA1r200_K.tgz`, `r5/final`, top 30 by score as `rounds.sh` picks
    them.
- **The lesson** (`come-home/`):
  - `homefar-farctx-vs-9d19121c-with-shadow-rim.patch`, which applies on top of `shadow-rim-vs-9d19121c.patch` (the
    foraging lane's, copied here).
  - `copier_c.py` (the foraging lane's copier plus `none_lookalike`).
  - Configs `hm1.json`, `hm2.json`, `hm3.json`, copier output `hm*.out`, `runf.sh`, `dlogH.sh`, `cmp.py`, and
    `results-32-seeds.txt`.
- **Genomes** (`genomes/`):
  - `hA1.raw.txt`: the founder, as judged.
  - `hA1z.raw.txt`: hA1 with the 16 links on slot 9 zeroed.
  - `hm1`, `hm2` and `hm3-not-recommended.raw.txt`.
