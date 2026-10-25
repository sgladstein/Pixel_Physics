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

## 2026-10-24 evening: graded far sense, nest scaling, and a new founder

**plantwean5** (std_pool on the planted bed, pile cap stepped on extinctions, 64 × 32): never weaned below a pile of
20; every step to 15 put 36-45 of 64 colonies out. At cap 20 the common garden got *worse* over the run: alive
(extinct as 0) 69.5 at r3 → 59 at r31, nest 160 → 141.5, extinct 13 → 17. Selection on the planted bed did not teach
the pool to live on plants. Not carried forward.

**farfood3b** (one founder, far64z; 32 colonies × 24 rounds; common garden every 8 rounds; food ramped from 80
columns). The deciding measures were named before launch.

| Arm | Food ramp reached | Lab case r23 | CG r23 nest | CG r23 starved | CG r23 nest ÷ needed |
|---|---|---|---|---|---|
| logsense (`PP_HOMEVEC=log:40`) | 100 | 1,183 | 127 | 43.5 | 0.115 |
| nosense (`PP_HOMEVEC=off`) | 110 | 1,292 | 181.5 | 103 | 0.168 |
| logscale (log:40 + `+scale`) | 90 | 869 | 318 | 24 | 0.225 |

- **Sense:** no far sense beat graded on the lab case in 21 of 32 pairs, which is not significant. In lab boxes the
  two arms' best colonies were level. Graded stays, per the owner's ruling, and is now the default (`log:40`).
- **Nest scaling:** it did what it was asked. Nest ÷ needed was higher in 30 of 32 pairs, nest 318 vs 127 (29/3),
  and the garden starved fewer. But the lab case fell in 30 of 32 pairs, and in lab boxes its colonies did worse at
  150-200 columns. Adding a case thins the pressure on the others. Not the founder; to be retried with the lab case
  weighted, or once laying is learned.

**Lab boxes, best balanced colonies** (alive at 15k frames after landing, 1 tank, seeds 1/2):

| Colony | Fruit 100 | Provisions 100 | Fruit 150 | Provisions 150 | Fruit 200 |
|---|---|---|---|---|---|
| logsense c12 | 117/109 | 193/190 | 110/95 | 189/232 | 31/43 |
| logsense c22 | 131/140 | 269/279 | 106/117 | 241/257 | 54/53 |
| nosense c4 | 128/136 | 265/205 | 102/101 | 226/278 | 58/71 |
| logscale c13 | 120/93 | 161/166 | 57/66 | 123/106 | 3/2 |
| far64 (the playtest build) | 113/118 | 182/221 | 17/36 | | |
| hand-built ant | 169/194 | 219/219 | 127/159 | 124/173 | 114/132 |

**Ratchet turn:** std_pool = logsense colony 12. Its lab survival is close to the best, and its nest is 409 cells
with crater 0 (c22's is 214), against the arm's median of 127. The lab now opens on the trained ant (`-- oldant` for
the hand-built one). Food 150 columns out is now survived at about the hand-built ant's level; 200 is still short.

## 2026-10-24 night: learned laying finds no brake; the cap stays

**laygene2** (32 colonies x 24 rounds, from logsense c12, `lex+spoil+crater+shape+lab`, food ramped from 100
columns, scent soft ceiling and graded hunger on in every arm). The owner asked whether laying should balance
through training rather than by rule. Three arms: `lg` (laying gene on, the training cap weaned away by
`capwean`), `lgctl` (laying gene off, cap kept), `lgkeep` (laying gene on, cap kept).

- **It did not balance.** With the cap weaned, colonies grow to 160-200 ants and starve about 850-1,050 ants a
  round; the garden nest halves (144 against 317 with the cap kept). On the lab case `lg` beat `lgctl` in only 13
  of 19 pairs, and no colony learned to lay less when food ran short. **Overturned:** "the genome will find the
  brake if we let it". The lay gate's senses are local only (owner, 17:13Z: no colony-wide store sense), and with
  those the selection never found one. The training cap stays; the laying gene stays on underneath it.
- **lgkeep, best balanced colonies, garden round 23** (nest / nest-to-needed / starved / crater): c9 481/.39/65/19,
  c6 665/.42/93/8, c8 536/.37/47/2, c17 570/.43/60/9. The current pool on the same garden: 409/.31/53/0.

**Lab boxes** (as above, both sense fixes on for every row):

| Colony | Fruit 100 | Provisions 100 | Fruit 150 | Provisions 150 | Fruit 200 |
|---|---|---|---|---|---|
| lgkeep c9 | 112/96 | 219/215 | 109/102 | 204/205 | 47/30 |
| lgkeep c6 | 39/106 | 201/196 | 94/82 | 201/207 | 24/15 |
| lgkeep c8 | 114/111 | 233/212 | 111/129 | 187/196 | 9/37 |
| lgkeep c17 | 121/118 | 211/250 | 105/91 | 175/185 | 19/27 |
| logsense c12 (current pool) | 153/173 | 232/217 | 115/108 | 173/194 | 23/19 |

The current pool with the two fixes off: fruit 100 132/109, fruit 150 104/94, provisions 150 142/188, fruit 200
37/44. So the fixes cost the trained ant nothing in the box and help it at 100 and provisions 150.

**Defaults:** `PP_SCENT_SOFTCAP` and `PP_HUNGRY_GRADED` are now on by default (owner, 17:35Z: on unless there is
uncertainty); `=0` gives the old behaviour. The soft ceiling is a world switch, so the hand-built ant's scent
changes too; its tests stay green.

**Ratchet turn:** std_pool = lgkeep colony 8: lab survival level with the current pool, nest 536 cells against 409,
crater 2, fewest starved of the four. Fruit 200 columns out is still the hand-built ant's (114/132 against 9/37).

## 2026-10-24 late: a wider bed does not buy range; a lab floor under the nest case works

**widebed1** (32 colonies x 24 rounds, from laygene2 colony 8, the shipped pool; `cg_every=8`). Three arms:
`ctl` (the standard cases, food ramp to the bed's 110-column limit), `wide` (the bed widened to 448 columns with
`PP_BED_W`, ramp allowed to 230), `scalegate` (`+scale` with `labgate=50`: every donor must clear the round's median
lab case). Deciding measures named before launch: wide vs ctl on the lab case, r23, paired (20 of 32); scalegate vs
ctl on garden nest-to-needed, r23, paired (20 of 32), lab case not worse.

- **wide failed.** Given room, the ramp still stalled at 120-125 columns, and the lab case lost 27 of 32 pairs.
  **Overturned:** "the bed's width is what stops the ramp". The limit is in the ant (a later finding by the old-ant
  thread put most of it on the brain's per-link energy bill).
- **scalegate beat ctl but just missed its bar.** Garden r23 (extinct as 0): alive 87 vs 74.5 (31 of 32 pairs),
  starved 46 vs 111.5, nest 220 vs 165, nest-to-needed ahead in 19 of 32 (bar 20), lab case ahead in 23 of 32.
  The floor did what farfood3b's `+scale` arm could not: nest and lab together.
- **Not carried.** No scalegate colony matched the founder's own garden (best nest 370 vs colony 8's 536).
- **The nest erodes over rounds in every arm.** Garden median nest, r7 -> r23: ctl 251 -> 165, scalegate 321 -> 220,
  wide 313 -> 196.5; laygene2's lgkeep 528 -> 317. Colony 8's 536 was partly its one world: its descendants
  averaged about 250 by r7. A founder picked on one garden world is picked partly on luck; pick on several.
