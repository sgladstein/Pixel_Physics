# Colony selection, 2026-10-22 to 2026-10-24: what each batch overturned

Teacher lane. This picks up where
[`inrun-selection/README.md`](../inrun-selection/README.md) (snapshot of 2026-10-22) stops. It covers about 30
batches on the 64-core VM and the rulings they led to. It records **what each batch overturned**, so that no later
session runs the same arm again. Pictures are in [`colony-selection-log-2026-10-24/`](colony-selection-log-2026-10-24/).
The raw `colsel.csv` and `cg.csv` for every batch are in the training bucket under the batch name (`colselN-10DD`,
`farfoodN-1024` and so on).

Abbreviations used throughout:
- **CG** is the common garden: fresh worlds, selection off, judged on the round's carried pool.
- **rN** is round N.
- **nest** and **crater** are cells counted by `nestcount.py` from the round-end map.
- **paired** means colony against the same-seed colony in the other arm.

## In plain words

- **The trained ant digs a real nest now.** It is a roofed chamber with side galleries and almost no crater. The
  crater was never a pay problem. Selected brains had no rule protecting the roof, and the old ant's *body* had one
  ([`crater-review-2026-10-23.md`](crater-review-2026-10-23.md)). We trained with those body rules on, then weaned
  them off. The colonies kept roofing after the rules were gone: crater 2.5-4, nest 325 → 504 (stdcheck).
- **Every batch now starts from the last batch's best colony** (owner's ratchet rule, 2026-10-24).
  [`assets/minds/std_pool.txt`](../../../assets/minds/std_pool.txt) is the current founder. Before this rule, every
  batch restarted from the same old brain, so the line never improved between batches.
- **Breeding is on in training.** It is stable at 70-90 ants with about 2 starved per round, and it does not empty
  the food-packed nest. The packed nest is the colony's working larder.
- **Trained colonies died in the owner's lab box because training only ever put food 25 columns away.** The
  far-from-home sense also turned ants back past about 70 columns. A food-distance ramp plus a softer far sense
  brings survival at 100 columns level with the old ant, from zero. 150-200 columns is still short. farfood3 is
  testing a graded (log) far sense against none.

## How to read a batch name

Batches are `colselN-10DD`, `brood1`, `plantweanN`, `stdcheck`, `farfoodN`. Arms are rows in
`scratch/inrun/jobs/<batch>.txt`. Each row is: `NAME colsel P MODE COLONIES ROUNDS FRAMES FOUNDER [args]`.

## 2026-10-22: the nest rule, and the old ant's setup becomes standard

| Batch | Arms | Result | What it overturned |
|---|---|---|---|
| colsel3/4 | lex + lesson c1 + inherited nest, 16 / 32 rounds | 59/64 nests ≥ 100 at r15; nest peaks r21 then erodes 257 → 147; spoil let go 437 → 1,030 | "Longer runs keep digging" is false: digging erodes after r21. Its `+spoil` arm was a no-op (lexicase read only 4 cases; fixed `22016765`) |
| colsel6 | +spoil / +spoil+crater / +spoil+crater+shape | +spoil: spoil left 188 → 18 (63/63), nest holds about 160 to r31 | **+spoil became standard.** The crater and room-shape cases barely moved their own numbers and cost nest: there was no room-making variation to select |
| colsel7 | food at 45, 65, moving (45/25/65) | home food rose (3,089 / 2,643 / 1,808 vs old ant 400 / 302 / 400) but kept nest fell to 23 / 6 / 21 (vs 96 at food 25) | Farther food on that founder made pits, not nests ([`nests-by-food-distance.png`](colony-selection-log-2026-10-24/nests-by-food-distance.png)). The old ant at farther food digs *more* (nest 302 / 613 / 679) |
| nest-rule fix `d3392d97` | (meter) | old nestcount counted door markers as roof and the door column as nest | **Selected "nest" numbers in colsel3-7 were inflated.** Crater is now "no ground between the datum and the cell, and datum gap > 5" ([`nest-vs-crater-highlighted.png`](colony-selection-log-2026-10-24/nest-vs-crater-highlighted.png)) |
| colsel8 | hA1 vs hcs founder | CG r31 food / starved / nest / crater: hA1 1,850 / 3 / 6 / 74; **hcs 2,008 / 0 / 50 / 224** | hcs became the founder ([`nests-new-founder.png`](colony-selection-log-2026-10-24/nests-new-founder.png)) |
| colsel9b | +cut_join=32, ±crater case | cut_join alone: food down on 60/64; cut_join+crater: 2,019 / 0 / 69 / 180 | Owner chose the standard: hcs + `PP_HOMEVEC=40:30`, lex+spoil+crater+inherit, lesson c1:0.1, inherit_nest=1, cut_join=32 ([`nests-cut-join.png`](colony-selection-log-2026-10-24/nests-cut-join.png)) |

## 2026-10-23 night: the crater cannot be paid away

| Batch | Arms | Result (CG r47) | What it overturned |
|---|---|---|---|
| colsel10 | standard, 48 rounds, food 25 / moving / 45 | nest / crater: 111 / 207 at 25; at 45 nest 3; moving nest collapses after r23 | The standard digs a wide open shaft with nest along its sides ([`nests-colsel10.png`](colony-selection-log-2026-10-24/nests-colsel10.png)) |
| colsel11b | `cut_roof=1` (no pay for unroofing) | nest 111 → 2.5, crater 207 → 73, food 2,016 → 1,604 | **A no-pay rule makes colonies stop digging rather than dig under a roof** ([`nests-colsel11.png`](colony-selection-log-2026-10-24/nests-colsel11.png)) |
| colsel12 | roof charge, + top-3-row charge, moving | nest 46.5 / 4 / 52, crater 204 / 124 / 185 | **Pay rules don't change which cell is cut.** Every arm had the same shape ([`nests-colsel12.png`](colony-selection-log-2026-10-24/nests-colsel12.png)). Settled by the independent review: the old ant's body refuses roof and room-breaking cuts, and the mind ant had neither rule |
| colsel13 | old ant's cut choice as crutch | preempted, not rerun | Deprioritised by the review |
| colsel14 | body rules `PP_BODYRULES=roof\|rooms\|both`, dig head | **VOID**: the meter fix made inheritance copy stored food round to round, and colonies re-dropped it as deliveries | Any change to what `read_nestmap` keeps changes what is inherited. Check the new world's start, not only the meter |
| colsel15 | rerun of 14 with the fix | roof: food 4,465, nest 143, crater 1; both: nest 189, crater 0 | **Body rules end the crater.** The inherited worlds show a flat scraped band (traced next) |
| colsel16/17 | roof rule ± inheritance | **no inheritance: nest 794 vs 143 (58/4 paired)**, crater 1; moving 604 | Inheritance opened the soil crust under the surface within 50 frames, with no ant cutting. Fixed by the soil-loss lane (PR #834, `inherit_link=20`). One huge roofed bowl, no rooms ([`gardens-colsel17.png`](colony-selection-log-2026-10-24/gardens-colsel17.png)) |
| colsel18/19 | both rules, no inheritance, + rooms case | nest 413-437, crater 1-6, rooms 4-12; flat room + galleries both sides | Old-ant-like plan with rules on ([`gardens-colsel19.png`](colony-selection-log-2026-10-24/gardens-colsel19.png)) |
| colsel20 | **weaned** 8:32 (rules fade, judged rules-off), light dig head | light head 322 / 5 / 0 starved, 63/64 nests ≥ 100; no-head control 71; light beats control 59/5 paired | **The roof habit survives weaning.** A general light sense (`PP_DIGLIGHT`) beats exact soil counts ([`gardens-colsel20.png`](colony-selection-log-2026-10-24/gardens-colsel20.png)). But the nest is a bowl packed with stored food |

## 2026-10-23 day: the food-packed bowl

| Batch | Arms | Result | What it overturned |
|---|---|---|---|
| colsel21 | colsel20 + rooms case | nest 208 at 25 (peak 419 r35); rooms only with moving food, small nests | Room scores were fooled by food shapes |
| colsel22 | + `store=4` (delivery pay falls as the store fills) | larder 36 vs 452, but **crater 310 vs 10** | **Store pay brings the crater back.** Dropped; larder left to brood and plants |
| colsel23 | inheritance with lined carve, link=20 | nest inh 102 vs ctl 208 (20/38 paired), rooms 4.5 vs 0 | Inheritance works mechanically now but still costs nest ([`nests-colsel23.png`](colony-selection-log-2026-10-24/nests-colsel23.png)) |
| brood1 | `brood=lay lay_cap=48:8` ± eat-at-home crutch | 70-90 ants, 61-64/64 alive, starved about 2/round; crutch crater 200-371 | **Breeding is stable; the crutch is not needed.** Breeding became the default (owner, 2026-10-24, PR #845) ([`nests-brood1.png`](colony-selection-log-2026-10-24/nests-brood1.png)) |
| broodplant1 | brood + plant bed vs brood alone | 64/64 alive both; nest 259 vs 186; stored food 297 vs 260 | Plants don't empty the bowl either ([`nests-broodplant1.png`](colony-selection-log-2026-10-24/nests-broodplant1.png)). Brood lane: the packed bowl *is* the larder, and rotting it kills 4/4 |

## 2026-10-24: the ratchet, and why trained colonies die in the lab

**stdcheck** (from the new `std_pool`, 64 × 16). Rules off from round 0 holds: crater 2.5-4, nest 325 → 504, starved
2-6. It beats the weaned arm on 41/64 paired nests. **Ratchet turn 1:** std_pool = stdoff colony 27 r15 (CG nest 530,
crater 3, starved 6, rooms 44), PR #851 ([`nests-stdcheck.png`](colony-selection-log-2026-10-24/nests-stdcheck.png)).
No old-ant body rule is needed once a pool roofs rules-off.

**The owner's lab playtest (07:13Z):** trained colonies starve within 5,000 frames of landing beside 8.9 MJ of fruit.
The old ant lives.

Traced with `examples/labsurvive.rs`, on a steady bed with food heaps at ±d columns:

| Measurement | Result |
|---|---|
| Trained pool, food d ≤ 50 | survives |
| Trained pool, food d ≥ 100 | all starve (fruit and provisions alike) |
| Old ant | survives to 240 |
| Start tanks, fruit vs provisions | barely matter |
| Training bed, food at 60 | 0 alive, even with 3 tanks |
| Training bed, food at 25 | 58-65 alive |

**Cause one, food distance.** Every batch behind std_pool fed at 25 columns. A trace (`PP_MIND_LOG`) shows empty ants
random-walking (41% steps out, 35% home) and none passing 49 columns. Pheromone trails *are* used: blinding them
halves food home. But a trail cannot lead to food nobody has reached.

**Cause two, the far-from-home sense.** `PP_HOMEVEC=40:30` saturates at 70 columns. Past that, an ant's "far from
home" input is maxed and the trained brain turns home: 60-75% of steps go homeward past 70 columns. At 100 columns
the 40:30 sense leaves 0-2 alive in the lab. 80:60, 100:100 or the sense switched off give 28-134.

**farfood1** (distance ramp `distramp=25:10:10`, lab case in lexicase, 64 × 24). The ramp settled at 45; 55 is a
cliff for this pool. Lab median 76 → 124. The best colony lives at fruit 25-50 (97-125 alive vs std 33-61); 100+ is
still 0 ([`lab-survival-farfood1.png`](colony-selection-log-2026-10-24/lab-survival-farfood1.png)).

**farfood2** (ramp from 45, `ramp2` with 40:30 vs `ramp2far` with 100:100):

| Arm | Food ramp | Lab case | CG nest | CG starved |
|---|---|---|---|---|
| ramp2far (100:100) | 45 → 80 | 386 → 730 | 134 → 224 | 56 → 32 |
| ramp2 (40:30) | to 70 | 208 → 403 | about 96 | rose to 110 |

Best colony far64 (ramp2far c64 r23), in the lab:
- At 100 columns it roughly matches the old ant: fruit 141-149, provisions 211-224, old ant 177-238.
- At 150 columns: 11-93.
- At 200 columns: 0.

The same brain with its far-sense links dropped (`far64z`), untrained on `PP_HOMEVEC=log:40`, does better:
fruit at 150 → 69-91, and 7-46 at 200
([`lab-survival-farfood2.png`](colony-selection-log-2026-10-24/lab-survival-farfood2.png)).

**Owner ruling (14:04Z):** prefer the graded far sense (`PP_HOMEVEC=log:S`, PR #856, ln(1 + d/S), no cap, no dead
zone) unless it is significantly worse and can't be trained better. A brain selected on a capped sense collapses on
the log sense (0-16 alive). So the old links are dropped and the pool reselected.

**farfood3** (running from 2026-10-24 ~17:20Z): far64z founder, `distramp=80:5:10 food=mix`, `logsense`
(`log:40`) vs `nosense` (`off`), 64 × 24.
- **Deciding measure, named before results:** the lab case at r23, graded vs none, paired by colony seed.
- **Secondary:** CG nest not lower.

The lab default change ("trained ant by default", held patch) ships once a pool survives the box and digs well.

## Standing rules this period produced

- **Judge digging rules-off** in the common garden. Judge feeding by tank, stored nest food and starved, never home
  deliveries alone: re-drops made home8 count about 11k of 13k deliveries as churn.
- **No pay rule for the crater or rooms.** Pay changes how much ants dig, never which cell.
- **Ratchet:** every batch starts from the latest best pool.
- **Breeding, `PP_HOMEVEC`, the dig head + light, `cut_join`, the crater and rooms cases, and `PP_MIND_GRAZE` are
  standard defaults** (PR #849). Experimental switches stay off.
- **Batch design** (training compute audit, 2026-10-24; being added to `Reports/how-we-test.md` by that thread):
  - CG every 8 rounds.
  - A shared control per ratchet turn, reused rather than rerun.
  - Screen at 32 × 16 before 64 × 48.
  - Name the deciding measure before launch.
  - Extinct colonies count as 0.
