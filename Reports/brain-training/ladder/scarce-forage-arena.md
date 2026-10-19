# A scarce-food forage arena that ranks brains the way the lab does

Ladder research lane, 2026-10-17 ~14:35. For the brain lane (its 13:48 ask, item 2), for the stage 2 forage rung. All numbers are measured unless marked inferred. Probe, jobs, logs and the full table are in `probes/scarce/`.

**Corrected 2026-10-17 ~16:40, after the independent review (`review-stage2-docs.md`, findings 5-7, 11-13, 21, 24).** The numbers all reproduced exactly on main `8e628480`. What changed:
- **`kept` overcounted.** It added every old-age death in the colony, hatchlings included (the rig's `old_age()` does the same). Counting founders alive only, the order still holds on every pair: 25.1 / 10.3 / 0.0 at 40 columns, 25.7 / 3.3 / 0.4 at 80. Count founder ids only.
- **Laying is automatic.** An adult at the nest with an empty crop and a bank over about 1,100 J lays, and a stage 2 brain cannot refuse. So this ranking rewards "be poor at the nest", not "lay within your means". **Do not train on this arena until food caps births (a store brake) or the mind has a Lay verb, and the three reference arms are re-run under that setting.** Done in section 6: with the brake that works, the champion stays last but the game ant and the cheap arm tie.
- **Train at 12k on the 80-column bed, not 40.** At 12k and 40 columns the overbreeder ties the game ant on the worse side (19.2 against 19.3).
- **Which brake (added ~17:35; measured ~18:15, see section 6).** `FOOD_BRAKE` (income over burn) is the right one of the three for a fixed drip, because income here does not grow with foragers. But as shipped it will not bite on this bed. Every brake is off below 30 live adults (`FOOD_BRAKE_SMALL`, full at 60), and the champion peaks at 15-27 adults per layout. Its burn also leaves out eggs and brood: approximated from the 2,000-frame books, the champion reads 1.3-3.6 income over burn through its crash, mostly above the ramp's top of 1.5. So lower the small-colony gate per world, and count brood cost or key on income per adult. Then check `food_brake_held` is above 0 before re-running the arms. `STORE_BRAKE` needs a store that holds a surplus; `LAY_BRAKE`'s holding was its crowding cue.
- **The ranking is one comparison.** Cheap and champion are one genome at two brain prices. The independent comparison is the game ant against that genome, and one of its four pairs is 32.0 against 31.7.
- Several figures below were means quoted beside worse sides; they are now labelled. Section 3 is the colony's books, not a trace of ants.

## Answer

**Use a drip of 5 provisions every 1,000 frames at the heap spot, 12 ants, births on, and score adults alive at the end times founders kept (old age counts as kept), worse side of each mirrored pair.** Corrected: count only founders' old-age deaths as kept (see above); the table's first two rows count every old-age death, and the founders-alive-only rows beneath give the lower bound. At 24,000 frames this ranks the game ant above the cheap-brain champion, and that above the champion, on every paired layout (4 of 4 and 4 of 4, at both 40 and 80 columns). That is the lab's order: about 319, 241 and 153 ants in the lab main test.

| 24k frames, drip 5, births on | Game ant | Cheap-brain champion | Champion | Zero genome |
|---|---|---|---|---|
| Adults × founders kept, worse side, food 40 columns out | **32.5** | 15.1 | 1.6 | 0 |
| The same, food 80 columns out | **34.1** | 7.7 | 1.7 | 0 |
| Adults × founders alive (lower bound), 40 columns | **25.1** | 10.3 | 0.0 | 0 |
| The same, 80 columns | **25.7** | 3.3 | 0.4 | 0 |
| Founders kept (40 columns), mean; worse side 1.0 / 0.5 / 0.1 | 1.0 | 0.6 | 0.2 | 0 |
| Births per layout (40 columns) | 28 | 38 | 43 | 0 |
| Ants starved per layout (40 columns) | 0.4 | 19 | 37 | 11.5 |
| Food cells collected, of 120 (40 columns) | 113 | 120 | 120 | 0 |

Neither score the brain lane proposed reproduces the lab:
- **Founders kept × colony energy with births off** ranks the cheap brain first and the game ant last at every drip level (for example 31,400 against 18,000 at drip 5, 12k frames, means; worse sides 30,372 against 14,941).
- **Ants alive with births on at 12k frames** puts the cheap brain above the game ant at 40 columns (26.0 against 22.4 at drip 5, means; worse sides 24.5 against 20.0), and at every richer drip. The overbreeders have not starved yet by then.

**Food brought home never ranks the game ant first.** All three collect nearly all the food, and the champion collects the most or ties. That is the abundant-heap score's mistake again, so "scarce" alone does not fix it. The score has to read survival.

## 1. The bed

`probes/scarce/ladderscarce.rs`, on `cf9b4350` plus this lane's throwaway probe patches; not for merge.
- **The bed:** the rig's forage bed, 256 wide, with the nest at column 128 on east layouts and 128 from the other wall on west ones.
- **The food:** a scenario-timeline `Heap` of provisions at the heap spot every `dripevery` frames. It needs `tick_timeline` after each step, as before. The brain lane's timeline tick in the rig's `frame()` does the same job.
- **Births:** `World::births_paused` turns them off.
- **What it prints every 2,000 frames:** adults, founders alive and kept, deaths by cause, bank, crop, brood and its bank, food dripped and collected, births, and the colony's energy books.
- **The arms:**
  - the game ant;
  - the champion (`rig-2026-10-17e/forage-game-big-s1/best.txt`);
  - the same champion with its brain priced like the game ant's (`PROBE_SYN_SCALE=0.125`, the "cheap" arm);
  - the zero genome.
- **Layouts:** 8 hold-out layouts (seeds 60000-60003, mirrored).
- **Run time:** 12k frames on 8 layouts is about 25 s single-threaded for the game ant, so a 24k layout is roughly 6-7 s.

## 2. The grid (48 runs, plus 16 at 24k frames or 24 ants)

| Drip per 1,000 frames | Births | What ranks game > cheap > champion on paired layouts |
|---|---|---|
| 5 | on, 12k | founders kept (4/4, 4/4 at 40 columns), and kept × energy. Adults × kept ties game and cheap (means 21.8 against 21.4; worse sides 19.2 against 19.3, game ahead 2/4) and puts the champion far below (mean 3.2, worse side 2.2). On the 80-column bed it already ranks right: 17.7 / 8.3 / 4.9, game > cheap 4/4 |
| 5 | on, **24k** | adults × kept 4/4 and 4/4 at both distances. Corrected: not quite every survival score; at 40 columns adults alone ranks game > cheap 3/4, and kept ranks cheap > champion 3/4 |
| 5, 10, 20 | off | nothing: the game ant is last on every energy score, because it brings home the least |
| 10, 20 | on, 12k | nothing: the cheap brain leads on head count (48.6 against 38.9 at drip 10), and only the champion's founders fall |
| 10, 24 ants (same food per ant as drip 5) | on, 12k | kept × energy (4/4 and 4/4 at 40 columns, 4/4 and 3/4 at 80). Head count puts the cheap brain slightly ahead (59.8 against 55.6) |

So two things make the ranking: **scarce food per ant, and a horizon long enough for overbreeding to starve the colony.** With more food per ant, the colony that lays more is still ahead at the bell.

## 3. The colony's energy books (no lab run, and no ant traced)

Corrected title: these are colony totals, not a trace of individuals. The upkeep table recomputes exactly, but it does not explain the cheap arm's starvation: the cheap arm costs 18% more per adult, takes in 14% more and nets more (44,917 against 40,604 J a layout), yet starves 18.9 ants a layout against 0.4. The extra goes into brood (29,313 against 16,166 J; 38.2 against 28.0 births). To answer why, trace each starved ant's last 2,000 frames (bank at its last nest visit, whether it laid, last meal).

The lab upkeep check was shelved on Scott's word (no lab tests until an ant passes the basic behaviours). The arena's own books answer the same question. Drip 5, 40 columns:

| Joules per adult per frame | Brain | Moving | Base | Total |
|---|---|---|---|---|
| Game ant | 0.005 | 0.033 | 0.027 | **0.065** |
| Cheap-brain champion | 0.005 | 0.046 | 0.026 | 0.077 |
| Champion | **0.037** | 0.049 | 0.028 | **0.114** |

- **The champion costs 1.75x the game ant per adult**: 8x in brain, 1.4x in walking.
- **It also lays half as many again on the same food:** 43 births per layout in 24k frames against 28. It feeds larvae twice as much (33,300 J against 16,200 J per layout; this counts crop feeding and larvae eating, not bank-to-larva nursing or shares).
- **The result:** 37 ants starve per layout, founders and newborns alike, and the colony ends with 16 adults against the game ant's 36.
- **Pricing the brain like the game ant's removes the brain cost but not the walking or the overbreeding.** The cheap arm still loses 19 ants a layout.
- **Inferred:** this is also why the champion's lab colony is half the game ant's. The lab's fixed income is scarce per ant once the colony is large.

## 4. Controls and cheats for the stage 2 forage rung

- **Do-nothing floor.** The zero genome on today's brain does nothing, starves and scores 0. A blank stage 2 brain picks at random, so it is not this control. Build a rest-only brain for the floor, and run the blank brain as the random null (`trail-rung-controls.md` has the same point for trail).
- **Should pass.** The game ant passes on today's brain. A hand-built stage 2 forager is needed before training (cheat E2).
- **Refusing to feed larvae.** In stage 2, feeding a larva is a brain choice. A colony that never breeds can score at most 12; the births-off runs are that colony, and score 10.4-10.6 at 12k frames, against 32-34 for the game ant with births on at 24k. Adults × kept is safe from this. Founders kept alone, or kept × energy, would reward it, which is why neither is the recommended score.
- **Corpse meals (C8).** The score counts heads, not meals, so eating dead nestmates earns nothing directly.
- **Horizon (E3).** Corrected: train at 12k **on the 80-column bed** (game > cheap 4/4 there, 17.7 against 8.3 on the worse side); at 12k and 40 columns the overbreeder ties the game ant. Hold out at 24k on both beds, because only there does a colony that breeds too fast lose clearly.
- **Laying (added).** Laying is automatic (see the correction at the top). Until food caps births or the mind has a Lay verb, a brain can only lay less by staying poor or away from the nest.
- **Founder premium (added, inferred).** One founder lost costs 1/12 of the score, about three newborns at 36 adults, and the mind can read its age. Watch births per founder as a second number.
- **Bell timing (added, measured).** The score is one snapshot at the last frame. The cheap arm's worse side climbs from 12.4 at 20k to 15.1 at 24k while 18.9 ants a layout have starved. Score the mean of the last few snapshots, or the minimum over the last quarter.
- **Switch set (added, measured).** With `NURSE=off` the game ant still beats the champion 4/4 (30.4 against 0.9, worse side), but the champion sits at 0 on three of four pairs, so only the cheap arm would show whether the arena still discriminates. Re-run all three arms under stage 2's exact switches.
- **More arms (added).** Add two or three arms whose lab colony size is known before calling this arena a ranker of brains.
- **Worse side (C4).** The table already uses it; a one-sided forager starves on its bad side.

## 5. What it costs, and what is not checked

- **Cost.** A 24k layout is 4x the rig's current 6k forage layout. At the design's search size (population 100, 8 layouts, 100 generations) that moves forage from about 4 h a seed to about 16 h (inferred, scaled from `design/general-brain-design-rev3.md`). Training at 12k with a 24k hold-out halves that.
- **Not checked:**
  - Food further out than 80 columns. The lab's far-food bed (170-200) is in `reserve-leash-check.md`.
  - Larger colonies (48 or more ants).
  - Whether a drip at one spot lets a colony camp at the spot. No arm here did: births and deaths are at the nest.
  - The random null and the rest-only floor, until the stage 2 act path exists.
  - The zero genome scores 1.0 at 12k (founders still alive), and 0 only by 24k.

## 6. Food caps births: the brake study (2026-10-17 ~17:40-18:15)

For the brain lane's ask of 17:33: make the food brake bite on this bed, try two burn rules, run the positive control first, then rerun the three arms. All measured, on `cf9b4350` plus throwaway env-gated probe patches (`probes/scarce/brake/`, never pushed). 8 layouts, 24k frames (12k read from the same runs), drip 5, 12 ants, births on, every other switch as shipped. **Kept now counts founder ids only** (`death_log`: founders alive plus founders dead of old age). The brake-off rerun reproduces the old champion log line for line (96 of 96 SNAP lines).

**Answer.** Only one rule actually holds eggs: **burn plus brood** (income over burn, where burn also counts eggs at 120 J and every joule fed to larvae), with the small-colony gate at 6. It stops the overbreeding: starved per layout 29 → 6 for the cheap arm and 33 → 13 for the champion. The champion stays last on every pair. **But the gap between the game ant and the cheap arm goes with it.** At 80 columns they tie at every snapshot from 10k to 22k (game ahead on 1-2 of 4 pairs). They separate 4/4 only at the 24k bell (21.9 against 16.4), and at 22k it was 20.7 against 20.9. So the game > cheap order in the brake-off arena was the overbreeding itself. A brain that costs 18% more per adult cannot be told from the game ant on this bed once laying follows food. One that costs 1.75x can.

**The positive control (champion, 80 columns, 24k):**

| Rule | Eggs held (ticks, 8 layouts) | Births per layout | Fired? |
|---|---|---|---|
| Off | 0 | 39.4 | — |
| Gate 6, shipped burn | 763 | 38.8 | barely: income over shipped burn reads 1.3-3.0 while the colony starves |
| Gate 6, fixed need 0.065 J/adult-frame | 0 | 39.4 (identical to off, SNAP for SNAP) | no |
| Gate 6, fixed need 0.10 | 100 | 40.4 | no |
| Gate 6, fixed need 0.15 | 6,210 | 34.8 | partly, and it holds the game ant far harder (70,372 ticks, births 28.8 → 12.1) |
| **Gate 6, burn plus brood, ramp 1.0-1.5** | **34,719** | **25.5** | **yes, on all 8 layouts** |
| Gate 6, burn plus brood, ramp 0.8-1.2 | 11,046 | 29.8 | yes, weaker |

**Why the fixed-need rule fails here.** The champion starves at 15-27 adults, where food per adult is still high (0.17-0.27 J a frame at the bell), so a fixed need never stops it. The game ant grows the biggest colony, so its food per adult falls first and the rule holds it hardest. It brakes the frugal colony and leaves the overbreeders alone.

**The three arms, 80 columns. Adults × founders kept (founder ids), worse side, mean of 4 pairs:**

| Rule | 12k: game / cheap / champion | Pairs game > cheap, cheap > champion | 24k: game / cheap / champion | Pairs | Game ant's births, 24k | Starved per layout, 24k: game / cheap / champion |
|---|---|---|---|---|---|---|
| Off | 17.7 / 8.0 / 4.5 | 4/4, 3/4 | 32.2 / 5.5 / 1.0 | 4/4, 4/4 | 28.8 | 0.6 / 29.0 / 33.0 |
| **Burn plus brood, 1.0-1.5** | **16.4 / 14.9 / 6.1** | **2/4, 4/4** | **21.9 / 16.4 / 8.3** | **4/4, 4/4** | **18.4** | **0.5 / 5.9 / 12.9** |
| Burn plus brood, 0.8-1.2 | 18.1 / 15.6 / 6.4 | 3/4, 3/4 | 30.3 / 11.9 / 2.3 | 4/4, 3/4 | 28.4 | 0.9 / 15.4 / 23.5 |
| Fixed need 0.065 | 18.3 / 12.8 / 4.5 | 3/4, 3/4 | 25.6 / 6.2 / 1.0 | 4/4, 4/4 | 27.2 | 0.5 / 23.8 / 33.0 |
| Fixed need 0.15 | 15.0 / 3.1 / 3.2 | 4/4, 3/4 | 13.8 / 3.3 / 0.7 | 4/4, 2/4 | 12.1 | 1.0 / 25.5 / 30.9 |

At 40 columns, burn plus brood puts the cheap arm ahead of the game ant at 12k (cheap 20.3 against game 17.8, game ahead 1/4) and just behind it at 24k (game 20.4 against cheap 17.8, game ahead 3/4). The champion is last on every pair at both distances. Full table: `probes/scarce/brake/table.txt`.

**What it does to the game ant (burn plus brood, 80 columns, 24k):** births 28.8 → 18.4 a layout (-36%), score 32.2 → 21.9, starved 0.6 → 0.5, founders kept 0.95 → 0.96. Its income over burn sits at 0.92-1.32 at the bell, so it lays at a rate the drip can pay for.

**The rule, exactly as tested** (one colony per world, so the probe used world-wide brood counters; the rig should book them per colony):
- every 256 frames, smoothed over about 3,000 frames (the shipped `step_colony_pace`):
  - income = the colony's `HarvestedPlant + HarvestedCorpse`;
  - burn = `Metabolized + Moved + SynapseTax` **plus** eggs laid × `egg_cost` (120 J), `brood_nursed_j`, `brood_crop_fed_j`, `brood_shared_j` and `brood_ate_j`;
- r = income / burn;
- the laying bar's factor is 1 at r ≥ 1.5, rises linearly to 3 as r falls to 1.0, and is infinite (no egg) at r ≤ 1.0 (the shipped `FOOD_BRAKE` ramp, `FOOD_BRAKE_MAX` 3);
- strength = clamp((live adults − 6) / 6, 0, 1), so the brake is off below 6 adults and full from 12. The shipped gate is 30, with full strength at 60, which this bed never reaches with the costly arms.

**Recommendation.** Use burn plus brood at 1.0-1.5 with the gate at 6. Accept that the arena then ranks the champion last and the cheap arm level with the game ant: the old game > cheap order measured laying, which the stage 2 brain cannot choose. Score the mean of the last few snapshots rather than the bell, because the 24k 4/4 rests on one frame. Before trusting it on a mind ant, check `food_brake_held` is above 0 on a mind arm, since the mind lays through `settle`.

## Files

- `probes/scarce/ladderscarce.rs`: the probe (args: genome, dist, width, layouts, first, frames, seed0, ants, heap0, drip, dripevery, births).
- `probes/scarce/runscarce.sh` with `scarcejobs.txt` and `scarcejobs2.txt`: the grid.
- `probes/scarce/scarcetab.py` and `probes/scarce/table.txt`: the paired table for every setting.
- `probes/scarce/logs/`: one log per run, with a SNAP line every 2,000 frames for each layout.
- `probes/scarce/brake/`: the brake study (section 6). `runbrake*.sh` and `jobs*.txt` are the 48 runs, `braketab.py` and `table.txt` the paired table, `logs/` one log per arm, rule and distance, `ladderscarce-brake.rs` the probe with founder-only kept, and the two `.diff` files the throwaway patches (`PROBE_BRAKE_SMALL`, `PROBE_BRAKE_BURN=brood`, `PROBE_BRAKE_NEED`).
