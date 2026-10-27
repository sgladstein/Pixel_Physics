# Is the ant training world set up properly? — 2026-10-27

**Asked by the owner** (2026-10-27 13:44Z, Teacher lane thread): *"review our training environment and if that is set up
properly."* The sibling review of scoring and selection is `selection-review` (PR #939); this one covers the worlds
themselves: the `examples/inrun.rs` bed, its round, food, breeding and rules, and the worlds the common garden and the
lab cases judge in. Page with the chart: <https://claude.ai/artifact/SPnFg8uqM8siivwhdkn6d4>.

**Answer.** The physics is right, but the timescale, the food and the rules are not. The bed is built by the lab's own
scenario code, and its physics shortcut measured as harmless. But it leaves out three things the lab game has, and
each one kills trained colonies there while the hand-built ant survives:

1. **Time.** A round is 200,000 frames on fresh soil. The shipped pool is alive in 15 of 16 garden worlds at 200k and
   in **0 of 16 at 600k**, on the same unchanged bed. The hand-built ant keeps 14 of 16.
2. **Food.** Training feeds only provisions and fruit. Windfall, the food in 9 of 36 shipped lab boxes and what the
   lab's trees drop, is never offered. In Two Larders the trained pool makes no deliveries on windfall and corpse. With
   the foods swapped to fruit and provisions it lives (mean headcount 3.6-7.3 → 37-101, 8 of 8 seeds, p = 0.008).
3. **Rules that exist only in the harness.** These are the laying brake (`lay_cap`), the lifted door ban
   (`PP_MIND_DOOR_OPEN=drop`), no old age, and three founding tanks. With all four set the way the game has them, the
   pool starves 1,232 against 284 (16/16) and every colony is extinct by 400k. The hand-built ant under the same rules
   grows to 314 ants.

## Method

- Pool: `assets/minds/std_pool.txt` (nest7 hweanrules colony 13, the current ratchet founder), all 30 members once.
- Bed: the e3 arm's common garden exactly (`scratch/inrun/jobs/elite2.txt`): `dist=25 spread=10 grants=3 window=4000
  sel=off food=mix patches=20000:40:120 setenv=PP_BED_W=448,PP_MIND_DOOR_OPEN=drop`, plus `STANDARD`. Worlds
  `c*1000+99`, c = 1..16, the gardens colony selection reads.
- One change per arm, 200,000 frames. 600,000-frame twins of the base and the all-changes arm. The hand-built ant
  (`nomind=1`) on the base, the all-changes arm and the 600k base. The 600k runs equal the 200k runs exactly at 200k
  (16/16 alive and nest identical), so the bed is deterministic and the long arms extend the short ones.
- Two new `examples/inrun.rs` switches, off by default (unset is the run unchanged): `oldage=N` (the ant's
  `life_half_life`; the bed zeroes it) and `tick=lab [labrain=R]` (step with `lab::tick_world`, the game's own tick,
  with the mister at `Rain::from_index(R)`).
- Lab: `examples/labsurvive.rs`, 60,000 frames after landing, `two_larders` as shipped and three food-swapped copies
  (`training-environment-review-2026-10-27/scen/`), seeds 1-8 for the trained pool, 1-4 for the hand-built ant.
- Every difference is worlds up/down of 16 with a two-sided sign test. With about 70 tests here, only p < 0.001 is
  read as firm.

Files: `training-environment-review-2026-10-27/` holds `summary.csv` (one row per run and 200k block), `series.json`
(median alive and colonies alive every 10k), `ladder-vs-T.txt` (the full table), `lab/results.txt`, and the scripts
(`mkjobs.py` writes the 208 inrun jobs, `run1.sh` runs one, `summ.py` and `stats.py` read them, `lab1.sh` runs a lab
box). About 240 runs on a 4-core cloud machine, about 40 minutes.

## 1. A round is shorter than a colony's life

| frames | trained pool, training bed: colonies alive / median ants / starved in block | hand-built ant, same bed | trained pool, all game rules |
|---|---|---|---|
| 200k | 15/16, 74, 284 | 16/16, 85, 1.5 | 11/16, 16.5, 1,232 |
| 400k | 10/16, 40.5, 250 | 15/16, 86, 0.5 | 0/16 |
| 600k | **0/16**, 0, 97 | 14/16, 83, 2 | 0/16 |

Nothing in the bed changes after 200k. Colony 1099, traced: stored food in the nest 214-374 cells through 260k, the
heap jumping to 55-119 columns out six times in a row (220k-360k), alive 74 → 22 → 0 by 340k with 70 cells still
stored. It died of the known fault (trained ants starve beside a full larder) once a run of far heaps came. A 200k
round holds ten heap moves, so selection rarely meets that run, and the garden (200k) and the lab case (15,000 frames
after landing) never judge past it. The owner's playtests ran 225k and 668k frames.

Nest at 600k is not the problem: the trained pool's open nest grows 494 → 783 while it dies (hand-built 1,205 →
1,866). Crater grows 62 → 333 (hand-built 0).

## 2. One game rule at a time (200k, medians, worlds up/down vs the training bed)

| change | alive | starved | food home | nest | firm effects |
|---|---|---|---|---|---|
| none (training bed) | 74 | 284 | 4,029 | 857 | — |
| `lay_cap=off` (the game has no brake) | 60 (8/8) | **1,506 (16/0)** | 3,374 (4/12) | 554 (2/14, p=.004) | starved, peak 80 → 184 (16/0), crater 62 → 188 (13/3, p=.02) |
| door ban kept (`PP_MIND_DOOR_OPEN` unset, the game's default) | 72 | 254 | **3,525 (1/15)** | **713 (0/16)** | food home, nest; open nest 494 → 340 (2/14, p=.004) |
| `oldage=40000` (the species file's) | 61 (2/14, p=.004) | 228 | 4,075 | 882 | 189 ants die of age |
| `grants=1` (one founding tank) | 73 | 268 | 4,128 | 874 | low point 24 → 13 (1/14, p=.001) |
| `tick=lab` (the game's tick) | 76 | 277 | 3,875 | 888 | none |
| `tick=lab labrain=1` (+ light rain) | 70 | 286 | 4,134 | 890 | none |
| all of the above | 16 (4/11) | **1,232 (16/0)** | **3,218 (1/15)** | **532 (1/15)** | 5 extinct at 200k; peak 170 (16/0) |
| hand-built ant, training bed | 85 | 2 | 1,602 | 1,322 | |
| hand-built ant, all of the above | **314 (16/0)** | 114 | **6,582 (16/0)** | 797 | the game's rules are survivable |

Rows 1-4 are rules that exist in training only, each with a cost in the game. Two of them were added on purpose
(`lay_cap` by the brood lane to keep the bed cheap and stable, the door drop by the Teacher lane), and the far64
playtest already traced the owner's boom and bust to the missing brake (`Reports/brain-training/playtest/far64-lab-2026-10-24/`).
The physics shortcut (rows 5-6) is clean: the harness steps the sweep, active sites, fields and scent, and leaves out
liquid and chunk bodies, particles, blasts and the mister, and none of that moves a bare bed.

## 3. The food menu

| Two Larders, trained pool, 60k frames | mean headcount (`lived`) | deliveries (plain, inflated) | alive at end |
|---|---|---|---|
| as shipped (windfall west, corpse east) | 3.6-7.3 | 0-18 | 0 in 7 of 8 |
| fruit and provisions instead | 37-101 | 1,900-5,300 | 35-141 |
| windfall side swapped, corpse kept | 46-74 | 440-1,270 | 25-116 |
| corpse side swapped, windfall kept | 14-44 | 670-5,100 | 2-56 |
| hand-built ant, as shipped | 131-221 | 7,200-21,500 | 342-444 |

Each food on its own is enough to kill the trained colony that depends on it. This explains the Teacher lane's exam
(`scratch/inrun/exam.py`, 2026-10-27: shipped boxes, trained pools 2-5 mean headcount in two_larders,
scattered_larder, feast_and_famine and the_bank, against the hand-built ant's 4-147). All four feed
windfall. Why windfall fails (smell, bite, or carry) was not traced here: `labsurvive trace=N` is the tool.

## 4. Smaller findings

- **Nests hit the floor.** The bed has 40 rows of soil. Every colony at 200k, trained (deepest open cell 35-39 rows
  down) and hand-built (39 in all 16), has dug to it. The lab box has 96 rows and the owner's boxes 176. Nest-size and
  room-shape cases are selected against a floor the game does not have.
- **A planted bed's rain never falls.** `inrun::step` never calls `rain::tick`, so `plants=… rain=2` is byte-identical
  to `rain=0` (`garden.csv` and `events.csv`, same md5, seed 7, 20k frames). On `tick=lab` the same rain doubles live
  leaf (237 → 472) and gives 7x the fruit (4 → 30) by 20k in that world. The plant-food lane's planted and weaning beds
  (`plantwean*`, `broodplant1`) ran dry. The garden's 6,000 growing frames still use the harness step even under
  `tick=lab`.
- **One colony.** The owner places 5-8 colonies in one box. Training never has a neighbour.
- **Landing.** Training founds 30 ants heaped at the door (`pile=1`) on bare soil at frame 0. A lab colony is 52, spread
  along the ground, usually into a grown box.
- **The lab's TRAINING page is stale.** `lab::training::Settings` is the 2026-10-22 control: no breeding,
  `kpm 0.03 kevents 10`, the 256 bed, food 25 out.

## 5. Recommendations, most important first

1. **Judge past the round.** Run the garden to 600k (cost ~3x a garden; `CG_EVERY=8` keeps it to a few % of a batch),
   or carry nest and ants into the next round (`INHERIT=1`). Report colonies alive at 600k beside the 200k numbers.
2. **Put windfall (and corpse, if ants should eat flesh) in `food=`,** and add a windfall box (`scattered_larder` or
   `feast_and_famine`) to the lab cases.
3. **One set of rules.** For each harness-only rule, either make it game physics or stop training with it: a laying
   brake (the colony's own, `PP_LAYGENE`, or a game rule), the door drop, old age, the founding grant. Tell the
   Teacher lane before any lands, since each changes training results.
4. **Give the bed the lab's soil depth** (96 rows) so nest selection is not capped.
5. **Lengthen the lab case** from 15,000 frames after landing to at least 100,000.
6. Run planted beds on `tick=lab` so that their rain falls. Later, try a second colony on some beds.

These are measured on one pool. The windfall and long-run findings are large enough (0 vs 37-101 headcount; 0/16 vs
14/16 colonies) that a second pool is unlikely to reverse them, and the Teacher lane's exam already saw the same
failure in the instinct and elite1 pools.
