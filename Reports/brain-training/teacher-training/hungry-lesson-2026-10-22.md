# Go out when hungry: the lesson, the founders, and the arena energy fix (2026-10-22)

The "go out when hungry" lane, split from the Teacher lane on 2026-10-21 by the
owner. Its job: teach copy v2 (`measurements/step1-runs/s171-copy/s171-down-v2.txt`)
to leave the nest for food when its tank runs low, without breaking its fetch or
its dig, and hand the Teacher lane founder genomes for in-colony selection. The
Teacher lane kept scoring and selection.

**Outcome in one paragraph.** The cause of in-run colony die-offs was traced
(ants holding a soil pellet at the bottom of the nest chamber when hungry). A
scratch teacher that acts out the lesson lifts clone-colony survival from 1.5 to
15.1 of 24 when obeyed every time. Copies obey it 30-50% of the time; two
founders (h1, h6) were delivered. The training arenas read energy against their
own budget rather than the tank, which forced the lesson into a narrow window;
that was fixed on main by PR #781. A first copy on honest readings (copy 9)
failed. In-colony selection (Teacher lane, 64 seeds x 6 rounds) ranks h1 above
h6, so the next copy should be aimed at in-colony survival.

Everything needed to rerun this is in
[`measurements/hungry-lesson/`](measurements/hungry-lesson/) (file list at the end).

## Why in-run colonies die

- Copy v2's job choice does not respond to its own energy. At every energy level
  it picks walk ~60% and feed ~37%.
- In the last 4k frames before a die-off, 94% of hungry decisions (bank under 5
  tanks) are ants **holding a soil pellet at the bottom of the nest chamber**,
  4-9 rows below the datum.
  - Jaws full means no PickUp.
  - An upward step was offered on all but 38 of 15,052 of these decisions; a
    Drop on only 184.
- Feeding is billed like a cut, and they feed constantly. So tanks drain in
  lockstep and everyone starves within ~1k frames.

## The teacher (scratch, `PP_HUNGRY=lo:hi` in `mind.rs`)

The rule acts for a bank between `lo` and `hi` tanks (bank / start_energy), with
an empty crop:

- **Laden:** drop the pellet off the hole if a Drop is offered. Otherwise carry
  it up, then away.
- **Empty:** take any PickUp. Below ground, step up. On the surface, step further
  from home, up to 40 columns (`PP_HUNGRY_R`).
- **Return** (any energy): beyond 40 columns with an empty crop and empty jaws,
  step nearer home.

Switches: `PP_HUNGRY=1.99:2 PP_HUNGRY_FLOOR=1.05 PP_HUNGRY_AWAY=1
PP_HUNGRY_SOFT=1 PP_HUNGRY_RETURN=1`. `PP_HUNGRY_ACT=q` makes the ant obey the
teacher with probability q. `PP_ENERGY_REAL=J` divides the energy sense by J
instead of `start_energy` (the scratch form of PR #781).

**Ceiling** (teacher acting, clone colonies of copy v2, 30 founders requested,
200k frames, seeds 11-18):

| Teacher obeyed | Survivors of 24 (mean) |
|---|---|
| Never (copy v2 alone) | 1.5 |
| 25% of the time | 7.4 |
| 50% of the time | 5.1 |
| Always | 15.1 |

What did not work:

- Firing at 3+ tanks: founders start at 3 and run out before the heaps fill.
- Heading for the food the teacher can see: ants chased provisions buried in
  mound walls and piled into towers.
- No search radius: ants ran to the bed's walls.

## The arena energy trap, and PR #781

Before PR #781, `budget_to` rewrote `start_energy` to the arena's budget so the
bills kept their size, and the energy sense divided by that same field:

- fetch and trail (budget 1e9) read exactly 1 tank, a flat 0.5, all run;
- the soil arena (2,400 J) read 1 tank falling to 0.25 as the ant dug.

A plain "below 2 tanks" lesson would then fire on every soil-arena decision and
fight the dig, which is why the scratch window starts at 1.05 tanks.

**PR #781** (owner approved; Brain lane's patch, opened from this lane) adds
`CreatureDef::sensed_tank`, which `budget_to` sets to the authored
`start_energy`. Every arena now reads the species' tank. Price on copy v2 with
no retrain, main against the PR on the same build and worlds (it matched the
scratch switch world for world):

- soil full loads (`reliab 8 12000`, 96 worlds): 10.71 -> 10.12 (-0.58 +- 0.32);
- fetch worse side (holdout 96, floor 17.61): 19.83 -> 19.76.

Side effect: a fed arena's ant (1e9 J) now feels full many times over (~1.0), so
the `forager` preset's egg bar fires there. The Teacher lane chose this (option
A) over keeping fed arenas at one tank, because a fed arena is a test
convenience and the ant should read its real tank everywhere.

## Founders (copier `copier_h.py`, hungry + return groups, v2 anchors)

| Genome | Live links | Clone survivors, seeds 11-18 | Soil full loads (v2 10.71) | Fetch worse side (floor 17.61) |
|---|---|---|---|---|
| copy v2 | 614 | 2 0 1 9 0 0 0 0 (1.5) | 10.71 | 19.83 |
| **hungry-h1** | 881 | 7 9 0 23 3 22 4 2 (8.8) | 10.49 (-0.22 +- 0.36) | 18.16 |
| **hungry-h6** | 1,135 | 18 6 2 0 23 22 14 0 (10.6) | 9.59 (-1.11 +- 0.37) | 18.33 |
| h8 (L1 on new links) | 907 | 0 0 1 0 0 1 0 11 (1.6) | 10.58 | 18.33 |
| copy 9 (honest readings) | 1,135 | 24 3 0 0 0 0 0 0 (3.4) | not run | not run |

- h6's dig loss is behaviour, not the link bill: v2 with the same 521 extra slots
  set to 1e-5 scores 10.70.
- Copies obey the teacher 30-50% of the time; the ceiling needs nearly all of it.
- Copy 9 was a plain "below 2 tanks" lesson with v2's kept skills re-taught on
  honest readings (`PP_ENERGY_REAL=200`). Its lesson overlap rose only from 0.24
  to 0.30. Not delivered.
- The Teacher lane's judge (`judge-h1.txt`, `judge-baseline-copy-v2.txt`): h1
  starved 43 of 192 in the hunger probe against v2's 99.

**In-colony ranking (Teacher lane, 64 seeds x 6 rounds, turnover 30k):** h1 lost
20 colonies of 64 against h6's 28, and brought in 6,906 food per seed against
5,291. Clone survival ranks them the other way. For the colony line h1 is the
founder, and any further copy should be aimed at in-colony survival.

## What is left

- A copy that holds the lesson better than 30-50% compliance, judged on in-colony
  survival (the Teacher lane runs a 64-seed cloud batch in ~20 minutes).
- Now that the arenas read real tanks, the 1.05 floor can go: the lesson can be a
  plain threshold. Copy 9 shows that alone is not enough.

## Files in `measurements/hungry-lesson/`

| File | What it is |
|---|---|
| `hungry-h1.raw.txt`, `hungry-h6.raw.txt` | The two delivered founders |
| `judge-h1.txt`, `judge-baseline-copy-v2.txt` | Teacher lane's standard judge on h1 and v2 |
| `scratch-on-b0dc264a.patch` | The whole scratch tree as a diff from main b0dc264a: the Teacher lane's in-run harness and teacher probes (`examples/inrun.rs`, `challenges::inrun`, `creature::cull_organism`; never for main) plus this lane's `PP_HUNGRY*` teacher, the copy logger's hungry label, and `PP_ENERGY_REAL`. The scratch commits are on no remote branch, so this is their only copy in the repo |
| `copier_h.py` | The copier (gradient descent on KL to teacher targets, groups, v2 anchors, `keep_rewrite`). Needs ~9 GB; run one at a time on a 15 GB box |
| `copy-h1.json` ... `copy-h9.json` | Copier configs for h1, h4, h6, h8 and copy 9. Paths point into the session scratchpad that made them |
| `thin.py` | Thins copy logs before training |
| `ev.sh` | Clone-colony judge: `ev.sh NAME GENOME seeds...` (inrun, 30 founders, 200k frames) |
| `comp.py` | Compliance with the teacher, by situation |
