# Where the ants' decisions go, and what freezes them

*2026-09-22. Result: step 1 of
[`ant-movement-plan-2026-09-22.md`](ant-movement-plan-2026-09-22.md), and, in
§10 (2026-09-23), step 2: the drop, cone and homeward counters. Both trace
today's code and change no behaviour. The mechanism it measures is described in
[`how-the-ant-works.md`](how-the-ant-works.md).*

## 0. The answer, stated once

- **On the foraging bed, ants spend most of their decisions frozen.** Their
  chance of stepping is **exactly zero** on 70% of empty-ant decisions and 83%
  of laden ones, at gap 90, median over 24 runs.
- **Empty ants are frozen by trail B.** At those decisions the trail-B term
  in the stepping sum is **−3.13**, against +0.10 when they move. No other
  term comes close, and the decomposition's residual is 0.000.
- **They are standing on local peaks of trail B.** 79% of their long-stall
  time is spent where **every heading they faced read downhill** on trail B,
  typically 3–5 different headings. The stalls sit a median 24 cells east of
  the nest's centre, where earlier work found the trail's hotspot.
- **The reading causes the freeze, and the colony's own trail makes it
  worse.** Empty ants step on:
  - **11.5%** of decisions as shipped;
  - **19.1%** with their own trail muted (higher in 23 of 24 runs);
  - **38.1%** with the trail-B reader off (24 of 24), the same as on a bed
    with no trail at all.
- **The same reading is what gets them to food.**
  - With the reader off, ants reaching the food fall 558 → 105 and second
    trips 87 → 0.
  - With only their own trail muted, second trips rise **87 → 183** (20 runs
    better, 3 worse), reproducing the 2026-09-21 result on today's code.
  - Without the hand-laid ramp the colony hardly forages: **10 of 1,441
    ants** reached food in 72 runs.
- **Laden ants are frozen by trail A's throttle and by facing away from
  home**: trail A −2.12 against −0.63 when moving; `HomeAligned` −0.21
  against +2.08. When a laden ant tumbles, the homeward re-roll:
  - is refused on 52% of tumbles because the ant is **on its anchor**;
  - loses its fill-scaled roll on 24%;
  - fires on 18%. Of those firings, 75% pick a heading toward home, 16%
    perpendicular, and 9% away.

**What this changes: one mechanism is doing two opposite jobs.** The
forward-difference throttle on `Move` is how an empty ant follows a trail
(step while the scent ahead is stronger) and how it gets trapped on the
trail's peaks (at a peak, every direction is weaker, so it never steps).
Stage 2 of the plan reads trail B as **presence per usable heading**, and a
bad reading means **turn more** rather than **stop**. That design has no
"downhill in every direction" state, so it should remove the trap by
construction; the stage-2 scenes have to show it does.
Stage 1 changes nothing about it. §8 draws the consequences.

## 1. What was built, and how it was checked

- **The engine records its own decisions.** Every walking decision pushes a
  `creature::DecisionRow` while `World::decision_log` is on (off by
  default). The row carries:
  - the head and heading, before and after;
  - the usable-heading mask, and the setting class that follows from it;
  - the leg, fill and anchor;
  - the wired inputs, the raw `Move`, `p_move` and `Turn`;
  - both rolls, and the branch taken;
  - the homeward re-roll's gate and cosine.

  `CreatureStats::decision_census` counts the same decisions by leg ×
  setting × outcome. `how-the-ant-works.md` §15 has the details.
- **No behaviour change, checked three ways**, each test watched going red
  on its planted fault:

  | Test | What it compares | Fault planted to watch it go red |
  |---|---|---|
  | `the_decision_trace_changes_nothing_it_watches` | a colony bed with the log on and off: grid, both planes, every creature, every counter | one extra RNG draw in the trace |
  | `the_setting_class_reads_the_ground_the_ant_stands_on` | flat ground reads exactly E+W, a sealed pocket 0–1 headings, the inside of a stone ring "open" | the foothold test removed |
  | `every_traced_decision_agrees_with_the_counters_and_the_positions` | rows against the census, against `moves`/`falls`/`reversals`/`crossings`/`tumbles`/`tumbles_homeward`, and against where the head went | idle decisions mislabelled as tumbles |

- **Every harness run checks itself.** `trailfollow decisioncsv` asserts at
  the end of each run that the census equals its own recount of the drained
  rows, and that the rows equal the engine's per-verb counters. **All 504
  runs below passed** (seven arms × 72).
- **Refactor.** `tumble`'s usable-heading test is now `usable_headings`,
  shared with the setting class so the two cannot drift. `home_weighted_pick`
  became `home_weighted_pick_why`, with the same body, the draw still taken
  last, and a test-only wrapper for the existing guard.
- **Tools.** `trailfollow` gains:
  - `decisioncsv`, `decisionnorows`, `dtag=` and `decisiondir=`;
  - `breadoff`: trail B read by nobody, still laid by everybody.

  `scripts/decisioncensus.py` reads the rows. Its `--selftest` passes,
  covering a known stall, known refusals, and a Move sum decomposed with
  zero residual.

## 2. The bed and the arms

- **The bed:** `trailfollow mode=gap`, 20 ants, 24,000 frames, food 400
  refilled every 400 frames, the hand-laid ramp until frame 6,000.
  `RAYON_NUM_THREADS=2` pinned. 24 seeds at each of **gaps 90, 140 and
  200**, the owner's standing instruction. Everything is paired within
  (gap, seed).
- **"Levers"** are the bed corrections of `ant-forage-bed-and-gates-2026-09-21.md`
  §2: `PIXEL_PHYSICS_COLONY_SPACING=2`, `PIXEL_PHYSICS_STACK_DEPTH=4`,
  `layfrom=founders`.

| Arm | Ramp | Ants lay B | Ants read B | Levers |
|---|---|---|---|---|
| A `hand` | yes | yes | yes | yes |
| B `hand breadoff` | yes | yes | **no** | yes |
| C `hmute` | yes | **no** | yes | yes |
| D `hand`, shipped | yes | yes | yes | **no** |
| E `self` | **no** | yes | yes | yes |
| F `self breadoff` | no | yes | **no** | yes |
| G `mute` | no | **no** | yes (nothing to read) | yes |

## 3. Where decisions go

Gap 90, arm A, 24 runs, **1,559,448 decisions**. Pooled shares first, with the
median over runs in brackets. The setting is the number of usable headings.

| Leg | Setting | Share of the leg's decisions | Stepped | Tumbled after a failed roll | Idle (both rolls failed) |
|---|---|---|---|---|---|
| empty | pocket (0–1) | 7.6% [7.3%] | 3.6% | 44.5% | 44.6% |
| empty | corridor (2) | 12.3% [11.4%] | 17.2% | 40.5% | 40.7% |
| empty | junction (3–5) | 54.0% [54.7%] | 13.2% | 42.3% | 42.3% |
| empty | open (6–8) | 26.1% [24.3%] | 10.3% | 43.6% | 43.7% |
| laden | pocket | 9.7% [9.0%] | 0.8% | 47.2% | 47.4% |
| laden | corridor | 20.7% [19.3%] | 7.5% | 45.7% | 46.2% |
| laden | junction | 57.4% [57.8%] | 8.6% | 45.4% | 45.0% |
| laden | open | 12.2% [11.0%] | 12.7% | 42.6% | 42.4% |

Three things this corrects:

- **Most decisions are not made in a corridor.** 55–58% of both legs'
  decisions are made where 3–5 headings are usable: burrows, corners, the
  nest. Flat ground is 11–21%.
- **"Open" on this bed is a crowd, not a canopy.** With stacking at the
  shipped depth of 1 (arm D), the open share falls from 24% to **2.8%**
  (empty) and from 11% to **3.1%** (laden). With stacking on, nestmates are
  enterable and count as footing, so a knot of ants reads as open ground.
- **S0's prediction was for a fed ant on bare ground.** On the bed, empty
  ants are hungrier, and where there is no trail they step on about **38%**
  of decisions (arms E and F, every gap). The jitter S0 predicts is a claim
  about a pinned, fed ant, and S0 still has to test it.

## 4. The freeze

**How often the chance of stepping is exactly zero**, median over runs (arm A):

| Gap | Empty | Laden | Laden decisions, pooled |
|---|---|---|---|
| 90 | **69.5%** (range 40–78%) | **83.2%** | 410,021 |
| 140 | 26.2% | 65.0% | 82,460 |
| 200 | 22.5% | 45.9% (7 runs with laden ants) | 6,039 |

The freeze tracks how much the colony forages. At gap 90 the colony is fed by
the ramp and laden traffic lays trail B; at 200 almost nobody carries food, so
almost nothing is laid.

**The stepping sum, term by term** (gap 90, arm A, founder weights):

| Term | Empty, frozen | Empty, moving | Laden, frozen | Laden, moving |
|---|---|---|---|---|
| bias | +2.000 | +2.000 | +2.000 | +2.000 |
| energy | −1.517 | −1.107 | −1.633 | −1.237 |
| home_aligned | 0 | 0 | −0.205 | **+2.078** |
| stillness | +0.757 | +0.040 | +0.448 | +0.117 |
| kin_need | +0.155 | +0.297 | +0.079 | +0.226 |
| food_adjacent | −0.054 | −0.004 | −0.342 | −0.025 |
| crowding | −0.210 | −0.108 | −0.196 | −0.108 |
| trail A (units 0–1) | −0.002 | −0.003 | **−2.124** | −0.628 |
| trail B (units 2–3) | **−3.130** | +0.103 | −0.006 | −0.005 |
| residual | −0.000 | −0.000 | +0.000 | +0.000 |
| **sum** | **−2.000** | **+1.218** | **−1.979** | **+2.418** |

- **Empty ants: trail B is almost the whole difference**, −3.23 of −3.22.
  Stillness is doing its designed job (+0.76 on frozen ants), but it tops
  out at +1.5, so it can never outweigh a trail-B reading of −3.
- **Laden ants: two terms.** Trail A (−1.50 of the difference) and facing
  away from home (−2.28).
- The residual is zero because every ant on this bed carries the founder
  wiring for these terms, so the decomposition is exact.

## 5. The peak trap, ant by ant

For every long stall of an empty ant (10+ consecutive decisions without
relocating, on one leg), the mean trail-B reading was taken for each distinct
heading the ant faced during it. Gap 90, arm A: **9,790 stalls holding
721,196 decisions.**

| During the stall | Stalls | Share of stalled decisions |
|---|---|---|
| **every heading it faced read downhill** | 5,049 | **78.7%** |
| some heading read uphill | 3,134 | 12.5% |
| flat (0) on some heading | 1,427 | 6.5% |
| only one heading seen | 180 | 2.3% |

- **Distinct headings faced per stall:** 2 → 1,292; 3 → 1,894; 4 → 2,238;
  5 → 1,736; 6 → 1,231; 7 → 774; 8 → 445.
- **Where:** a median 24 cells east of the nest centre (p10 19, p90 67).

**An ant that has tumbled through four or five headings and found every one
weaker than where it stands is on a local peak.** The reader then forbids
every step, and only the peak fading or an outside push can free it. That is
the "trapping" candidate `pheromone-trail-direction-2026-09-16.md` named and
nobody had separated from the "direction" one. Here it is measured, and it
accounts for four-fifths of the stalled time.

## 6. The homeward re-roll

Every tumble, by the gate that decided it (gap 90, arm A):

| Leg | Tumbles | On anchor | Roll failed | **Fired** | No crop | No usable heading |
|---|---|---|---|---|---|---|
| laden | 186,285 | **52.5%** | 24.0% | **17.9%** | 5.2% | 0.4% |
| empty | 465,439 | 1.9% | 0.1% | 0.0% | 97.5% | 0.4% |

- **Of the laden firings:** toward home 75.2%, perpendicular 15.7%, away 9.1%.
- **Why "no crop" appears on laden rows, and firings on empty ones:** the
  leg is taken from what the brain saw, before `act`. An ant that dropped
  its load this tick is still "laden" when it tumbles, and one that just
  picked up is still "empty".

What this says:
- **The re-roll works when it is asked.** It fires on every tumble that
  passes its gates, at chance `fill`.
- **But it is asked in the wrong place half the time.** A laden ant on its
  anchor is at the nest, which the bed report's §10 finds is where drops
  fail.
- **The 9% "away" firings are the obstruction case the plan predicted**
  (§2a, S2, S3): the best usable heading points away from home, so the ant
  faces away and its throttle reads negative.

## 7. The trail-B arms

**How often empty ants step** (median over runs, paired within seed; higher /
lower against arm A in brackets):

| Gap | A `hand` | C `hmute` | B `breadoff` | D shipped | E `self` | F `self breadoff` | G `mute` |
|---|---|---|---|---|---|---|---|
| 90 | 11.5% | **19.1%** (23/1) | **38.1%** (24/0) | 14.9% (20/4) | 38.1% | 38.4% | 38.8% |
| 140 | 34.8% | 35.7% | 38.6% | 33.7% | 39.0% | 38.9% | 38.5% |
| 200 | 37.7% | 37.9% | 38.6% | 37.9% | 39.1% | 38.9% | 38.5% |

**The loop**, all three gaps, from `scripts/funnelpair.py`:

| Stage | A `hand` | C `hmute` | B `breadoff` | E `self` | F `self breadoff` | G `mute` |
|---|---|---|---|---|---|---|
| ants that lived | 1,499 | 1,579 | 1,532 | 1,441 | 1,445 | 1,446 |
| reached the food | 558 | 624 | **105** | **10** | 19 | 15 |
| got back holding it | 386 | 422 | 11 | 10 | 15 | 12 |
| went back out | 243 | 350 | 7 | 1 | 9 | 7 |
| **reached the food a second time** | **87** | **183** | **0** | **0** | 1 | 0 |

What the arms establish:

- **Reading trail B is what gets ants to food on this bed** (A against B).
  With no reader, the hand-laid ramp is invisible to them.
- **The same reading freezes them**, most where the most trail has been laid
  (gap 90, A against B).
- **The colony's own trail adds to the freeze and costs second trips**
  (A against C). Removing it leaves the ramp readable, and raises empty-ant
  stepping from 11.5% to 19.1%.
- **Without the ramp, the colony's own trail neither helps nor freezes
  anyone** (E, F and G agree to within a point at every gap): almost
  nobody finds food, so almost nothing is laid.

### 7a. A setup confound, declared

The plan (§7) designed the reader-off arm to separate *reading* harm from
*laying* harm. **On the hand-ramp bed it cannot.** Switching off the reader
also hides the hand-laid ramp, so arm B's collapse says the reader is
needed, not that laying is harmless. The no-ramp arms E, F and G are the
clean version of the question, and they show there is too little trail for
it to matter. So the question "does the colony's own laying hurt, read the
way it is read today" is answered by A against C: yes.

## 8. What this means for the plan

- **Stage 2's design aims at exactly what froze the colony here.** A
  forward-difference throttle cannot follow a trail without also being
  trapped at its peaks, because both are the same arithmetic. Reading
  presence per usable heading, and turning rather than freezing, has no
  state where every direction reads worse and the only answer is to stand
  still. That is a design claim, and S4, S5 and the bed have to show it.
- **Stage 1 still comes first**, for the reason the plan gives: it changes
  exploration and homing, and doing both stages at once would confound
  them. But stage 1 is judged on scenes without trails, so on this bed it
  **cannot** show the freeze going away. The bed will look frozen until
  stage 2.
- **The drop census (step 4) should start at the anchor.** 52.5% of laden
  tumbles happen on it, and the bed report already puts failed drops there.
- **Frozen, not jittering.** On the bed, the thing to fix is freezing. S0
  still tests the jitter prediction on bare ground, with energy pinned.

## 9. What this does not settle

- **One bed.** It is a flat surface with burrows and no plants, so the
  canopy and plant-climbing settings are untested, as the plan says.
- **Stacking inflates the "open" class.** Conclusions about open ground need
  the shipped stack depth or a real canopy (S5).
- **Muting and `breadoff` remove weights, and so synapse tax.** It is one or
  two synapses per ant, too small to move a loop count, but not zero.
- **The trace covered the move stage only.** A drop that rolled and found no
  space (C2) was not in it. Step 2 added it; see §10.

## 10. Step 2: the drop, the cone and the homeward re-roll, counted

*2026-09-23.*

**The answer.**

- **Half the drops a laden ant wins at the nest go nowhere.** At gap 90 the
  median run has **52%** of its won drop rolls find no empty neighbour
  (p10 33%, p90 72%; 57% pooled). The roll is spent and the tick does
  nothing. The cells in the way:

  | Filling the eight neighbours | Share |
  |---|---|
  | nestmates | 41% |
  | `packedsoil` (burrow lining) | 23% |
  | nest material | 14% |
  | loose soil | 8% |
  | food already put down | 8% |
  | the ant's own body | 5% |
  | spoil | 1% |

- **It is a crowding effect where the colony is busiest.** At gap 140 only
  15 of 24 runs deliver anything and the median no-room share is 0% (30%
  pooled, from a few crowded runs). At gap 200 two runs deliver at all.
- **The cone mostly goes straight.** Straight is taken on 97% of corridor
  steps and 75–79% at junctions and in the open. At junctions usually only
  one side is open (68% of steps). **In side view its turns are mostly
  climbs.** At junctions, side-steps go up, level and down:

  | Leg | Up | Level | Down |
  |---|---|---|---|
  | empty | 7,213 | 6,306 | 2,822 |
  | laden | 2,131 | 1,689 | 753 |

  Laden ants also turn right twice as often as left (15.9% against 6.8%).
  They mostly face west, toward home, where right is up.
- **`Turn` is too small to steer anything on this bed.** Over the 72 runs its
  largest value is 0.031, and it reaches 0.001 on 0.38% of gap-90 decisions
  and almost never at the other gaps.

**What was built.** Three counters, always on, and the same facts in the
trace row:

- **C1**: `homeward_why` counts every call to the homeward re-roll by the
  gate that decided it, and `homeward_aim` every firing by where it pointed.
- **C2**: `drop_census` counts every drop roll: lost, placed, delivered, or
  no room. The row adds the roll, its probability, `free8`, and the eight
  neighbours by material, own body or other organism.
- **C3**: `cone_picks` counts which forward candidate each step took;
  `turn_requests` and `turn_discarded` count steps with nonzero `Turn`, and
  those whose requested side had been zeroed. The row adds the three scores
  and the pick.

`how-the-ant-works.md` §5, §6 and §15 describe them.

**How it was checked.** Each test was watched going red on its planted fault.

| Test | Known answer | Fault planted |
|---|---|---|
| `every_traced_decision_agrees_with_the_counters_and_the_positions`, extended | counters = rows = `drops`, `deliveries`, `tumbles_homeward`; every pick is the heading the head moved along | delivered and placed swapped; the pick off by one |
| `the_homeward_re_roll_aims_along_a_known_floor_at_the_rate_its_fill_sets` | a laden ant on a bare floor, anchor 40 cells west: every firing at cosine exactly 1, firings = the sum of fill within binomial noise | argmax turned into argmin |
| `a_drop_with_nowhere_to_go_is_counted_and_one_with_room_is_delivered` | a pocket sealed by nest material: every won roll no room, nothing dropped; an open nest floor: every won roll delivered | no room booked as placed |
| `the_cone_discards_a_turn_with_nowhere_to_go_and_follows_one_with_somewhere` | bare floor facing east: scores 0 / 1.6 / 0 and a small `Turn` either way discarded; inside a stone ring: 1.4 / 1.6 / 0.6, none discarded, the side asked for wins | the side candidates' footing bonus removed |

**On the bed**, arm A of §2 (24 seeds × gaps 90, 140, 200):

- **All 72 runs passed** the end-of-run reconciliation.
- **The step-1 columns of seed 1 at gap 90 are byte-identical** to the step-1
  trace, 69,837 lines. So step 2 changed no behaviour on the bed.
- **The homeward counter reproduces §6 exactly** on the same runs, as it
  must.

**Reading the drop numbers.**

- **Nothing is ever "placed" away from the nest.** `Drop` reads exactly 0
  off the nest for a full crop (`how-the-ant-works.md` §5), so every won roll
  is at the nest.
- **"On the anchor" (99.6% of no-room rolls) says nothing new.** A drop can
  only win at the nest, and at the nest the anchor is re-set to the head.
- **Against plan §8's hypotheses**, at gap 90:
  - nestmates (H4) are the largest share;
  - burrow lining (H1) is second;
  - nest material (H2) is third;
  - earlier drops (H3) are fourth.

  None is the whole story. Step 4 still has to say when each blocking cell
  got there, and whether the blocking nestmates are themselves laden and
  waiting.

**A correction, and how it happened.** The plan's first step-1 revision said
`Turn` was exactly 0 on all 69,836 decisions of one run. The CSV printed it to
four decimals, so a nonzero value below 0.00005 read as 0.0000, and a parse
called it zero. Printed in full, that run's `Turn` is nonzero on 8,196
decisions, with a largest value of 0.00000008. The conclusion stands (`Turn`
cannot steer on this bed); the claim did not. The CSV now prints `Turn` in
full. Step 1's exact-zero `p_move` figures were checked against the same
trap: at most 2 of 48,172 could be a tiny positive rounded down.

**For the plan.** Step 4 has its instrument, and gap 90 is where to use it.
The chooser should be judged in absolute directions (up, level, down), since
left and right mean different things at different headings. Its
per-usable-heading design already scores that way.

## Data

In `Reports/data/`:

- `decisions-seed1-gap90-2026-09-22.csv.gz` — every decision of one run of
  arm A (69,836 rows). Read it with `python3 scripts/decisioncensus.py`
  after gunzipping. **Replaced 2026-09-23** by the same run with step 2's
  columns. Every step-1 column is unchanged except `Turn`, which is now
  printed in full.
- `decision-census-step2-A-levers-hand-2026-09-23.log` — step 2's 72 runs,
  with the C1–C3 lines per run, and `decision-census-step2-rows-2026-09-23.txt`,
  `decisioncensus.py` read over all 72 runs' rows, one block per gap.
- `decision-census-{A-levers-hand,B-levers-hand-breadoff,C-levers-hmute,D-shipped-hand,E-levers-self,F-levers-self-breadoff,G-levers-mute}-2026-09-22.log`
  — each arm's 72 runs, with a census block and a loop funnel per run.
  `python3 scripts/funnelpair.py <a> <b>` pairs the funnels.
