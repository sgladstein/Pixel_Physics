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

## 2026-10-25: the brain's link price, not training, sets the far-food range

The old-ant thread traced the far-food gap to the brain's per-link energy bill (the standard pool bills ~1,900 links
a tick; one tank lasts ~1,000 frames against the hand-built ant's ~3,700) and built `PIXEL_PHYSICS_LINK_PRICE`.
**linkprice1** (32 x 24 from laygene2 colony 8, the scalegate setup, the 448 bed with the ramp allowed to 230, lab
case at 150 and 200 columns; built on main with the lab plant pool briefly on, so lab numbers sit low on food in
every arm alike) trained at price 1 (control), 0.25 and 0. Deciding measure named before launch: the lab case at
r23, paired against price 1 (20 of 32), garden nest not lower.

- **Price 1 never left 100 columns** (median starved 39-53 a round). Prices 0.25 and 0 ramped to 120-125 and
  bounced there, with colonies dying out in-round at the edge (0.25, r23: 17 of 32).
- **Lab case r23:** price 0 ahead in 22 of 32 pairs (median 1,894 vs 1,028), 0.25 in 14. Garden r23 (extinct as 0):
  nest 280 / 0 / 249, starved 39 / - / 2, alive 81 / - / 88.5. Price 0 passed its lab bar and failed the nest bar.
- **Lab boxes** (grant 1, seeds 1/2, alive at 15k, plant pool off):

| Price | Colonies | Fruit 150 | Provisions 150 | Fruit 200 |
|---|---|---|---|---|
| 0 | standard pool | 173/205 | 364/323 | 163/146 |
| 0 | best four price-0 colonies | 190-246 | 316-360 | 129-188 |
| 1 | standard pool | 114/123 | 200/178 | 20/39 |
| 1 | the same four | 95-130 | 134-234 | 0-9 |

**Overturned:** "far food needs a better-trained ant". The untrained pool at price 0 outlives the hand-built ant at
200 columns (114-132), and training at price 0 adds little to it. The lever is the world's price, the owner's call.

Found on the way: `+scale` scored a colony that had died out at the cap (no ants, so leftover food alone set its
need); fixed to 0. Harmless to donors (a dead colony carries nothing), but it skewed the case's column.

## 2026-10-25: the nest erodes inside the round, not from colony selection

**erosion1** (32 x 24 from colony 8, price 1): the standard lex arm beside `plain` (no colony selection; each colony
keeps only the in-run loop). Measure named before launch: garden median nest, r7 -> r23.

- lex: 446 -> 394 -> 280 (identical to linkprice1's price-1 arm, same seeds: the harness is deterministic).
- plain: 185 -> 84 -> 46, with late cuts 120 -> 65 and 26 of 32 colonies left by r23.

**Overturned:** "colony selection wears the nest down". Without it the nest erodes faster; colony selection is
what slows it. The erosion is inside the round, where children are bred from the best-scoring ants, and the
standard in-run score pays a delivery 7 times (`w_deliver=7`) what it pays a nest cut (`w_dig` 1). Next:
**erosion2** raises `w_dig` to 3 and 7 against a fresh control.

**erosion2** (the same setup, `w_dig` 1 / 3 / 7): paying a nest cut more inside the round does not stop the erosion.
Garden median nest r7 -> r15 -> r23: control 446 -> 394 -> 280, `w_dig=3` 467 -> 318 -> 204, `w_dig=7` 491 -> 487 ->
184. The lab case fell (`w_dig=7` ahead of the control in 1 of 32 pairs). The control matched erosion1's exactly on
a later main. Carried ants' live links stayed at ~1,950 in every arm (the standard pool's member 0: 1,947), so the
brain is not being pruned. **Next, erosion3:** the same control against food held 25 columns out (`dist=25`, no
ramp). Every batch that eroded trained on far food (100+ columns); colsel10 at 25 columns grew its nest r31 -> r47.

**erosion3** (the same control against `near`: food held 25 columns out, no ramp): far food is not what erodes the
nest. Garden median nest r7 -> r15 -> r23: control 446 -> 394 -> 280, `near` 276 -> 176 -> 156 (each arm's garden at
its own training distance, 100 and 25). `near` had the smaller nest in 27 of 32 pairs at r23 and lost the lab case
(ahead in 10 of 32); it kept more ants alive in its own easier garden (86.5 against 81). **Overturned:** "the far-food
ramp trades digging away". Every arm of erosion1-3 erodes, whatever the food distance, the dig pay or the colony
selection, so the next suspect is the in-run loop itself. One candidate found since: `pack_behind`'s cut ahead and
`escape_cut` never reached `cut_log`, so the in-run dig credit missed them (fixed with the 0.25 link price, PR #899).
**price25a**'s control (r7 -> r15 garden nest, on the fixed build) is the first read of whether that mattered.

## 2026-10-25: at the 0.25 price the ramp hits a wall in the bed, not in the ants

**price25a** (32 x 16 from colony 8 at the new 0.25 link price, the erosion3 setup): control, `rho`
(`PIXEL_PHYSICS_B_RHO=0.25`, the pheromone lane's faster-fading B trail) and `caste` (`PP_MIND_CASTELESS=1`).

**The ramp is a cliff at 121 columns, and it is the bed's.** In every arm the food distance went 100 -> 125 in five
rounds, then alternated: every round at 120 lost a median 3-6 ants and kept all 32 colonies, and every round at 125
lost ~230-350 and only 12-25 colonies came through. The bed's west wall is 128 columns from the door, and a west heap
closer than 8 cells to it is moved east (`inrun.rs`), so past 120 **both heaps sit on the east side**. Five more
columns are not the difficulty; losing the west heap is. widebed1 and linkprice1b's "stalled at 120-125" was the
same wall. With `cg_every=8` every garden (r7, r15) fell on a 125 round, so the gardens drop the colonies that round
killed, and the control's lab case read 404 at r15 against 1,766 at r14.

Paired only where both colonies survived the garden round:

| r15, vs control | nest bigger | food home (`home8`) more | starved fewer | crater smaller |
|---|---|---|---|---|
| `rho` (n = 11) | 3 (233 vs 395) | 10 (15,406 vs 10,905) | 7 | 8 (7 vs 19) |
| `caste` (n = 14) | 5 (267 vs 392) | 12 (12,754 vs 10,290) | 3 (40 vs 13) | 11 (4.5 vs 18.5) |

Lab case at r14 (the last clean round): `rho` ahead in 16 of 32 pairs, `caste` in 12. Both arms bring more food home
and leave smaller craters, and both dig smaller nests. That is the opposite of the untrained screen for `rho`
(nest 432 -> 528). `caste` also starves more, as the touch-feeding finding predicted. Neither is carried: the samples
are small and the cliff confounds them. **Next, price25b:** the same three arms, 24 rounds, ramp capped at 120
(`distramp=100:5:10:120`), so every round keeps food on both sides. The control's r7 -> r23 garden is also the first
erosion read on the `cut_log`-fixed build: here the survivors' nest was 510 -> 395 by r15, against 446 -> 394 at
price 1, so the fix has not stopped it.

## 2026-10-25: with food on both sides, the nest stops eroding; colony 10 becomes the standard ant

**price25b** (price25a's three arms, 32 x 24, ramp capped at 120 so both heaps stay on the bed). Every colony of
every arm lived through every round. Garden medians (food 25 out), paired over 32 colonies:

| | r7 nest | r15 nest | r23 nest | r23 food home | r23 starved | lab case r23 ahead |
|---|---|---|---|---|---|---|
| control | 458 | 454 | **501** | 12,230 | 8.5 | (base) |
| `rho` (B fade 0.25) | 440 | 476 | 214 (bigger 2/32) | 15,988 (28/32) | 22.5 (fewer 8/32) | 20/32 |
| `caste` (`PP_MIND_CASTELESS=1`) | 509 | 297 | 278 (1/32) | 14,549 (26/32) | 46.5 (2/32) | 27/32 |

**Overturned: "the nest erodes inside the round whatever we do."** At price 1 every control fell from ~446 to ~280
by r23 (erosion1-3); this one held at ~500. The change from erosion3's control is the 0.25 link price and the
`cut_log` fix together (both in #899); which of the two did it is not separated. Both arms trade the nest for food
home under selection, and both starve more, so neither is carried. `rho` is recorded as failed under selection
by the pheromone lane; `caste` wins the lab case clearly (27/32) and may come back weaned.

**The ratchet: colony 10 of the control is the new `std_pool`.** Four of the control's best r23 colonies against
colony 8 (the old `std_pool`) on 8 fresh gardens with food 100 out (the colsel garden is 25 out; its repro of
colony 10 at its own seed matched the VM row exactly):

| | nest (bigger) | crater (smaller) | starved | food home (higher) |
|---|---|---|---|---|
| colony 8 | 344 | 6.5 | 13.5 | 12,524 |
| c10 | 462 (8/8) | 2 (8/8) | 10 | 11,872 (1/8) |
| c21 | 394 (7/8) | 8 (3/8) | 8.5 | 10,402 (0/8) |
| c22 | 430 (7/8) | 5 (5/8) | 8.5 | 11,515 (3/8) |
| c24 | 430 (7/8) | 5.5 (5/8) | 6.5 | 11,556 (3/8) |

Lab box, price 0.25, seeds 1-4, mean headcount after landing, c10 vs colony 8: fruit 150 95.9 vs 98.6, fruit 200
77.9 vs 74.5, provisions 150 152.8 vs 153.7. Level in the lab, a third more nest, about 5% less food home.
`founders=std` on the new file reproduces the carry's fruit 200 runs exactly. **Next, door1:** control against
`PP_MIND_DOOR_OPEN=drop` and `=clear` from the new pool (the owner asked for both crutches off the trained ant).

## 2026-10-25: the door's two crutches under selection; a plants-only lab case

**door1** (32 x 24 from the new `std_pool`, colony 10; price25b's setup): control against `PP_MIND_DOOR_OPEN=drop`
(an ant may put a pellet down in the door column) and `=clear` (the world stops deleting soil there). Every colony
lived through every round. Garden medians (food 25 out), paired over 32:

| | r7 / r15 / r23 nest | r23 food home (higher) | r23 starved | r23 alive | lab case ahead r7 / r15 / r23 |
|---|---|---|---|---|---|
| control | 404 / 390 / 383 | 16,412 | 6 | 92 | (base) |
| `drop` | 419 / 341 / 377 (r23 bigger 18/32) | 15,469 (15/32) | 8.5 | 92 | 15 / 23 / 18 |
| `clear` | 440 / 423 / 409 (17/32) | 6,073 (2/32) | 12.5 | 87 | 21 / 29 / 25 |

**Lifting the put-down ban costs nothing under selection**: every measure is even with the control. **Stopping the
world's clear of the door column halves the food carried home**, and 24 rounds of selection did not win it back
(7,188 -> 5,403 -> 6,073); it does lift the lab case (median 2,181 against 1,865 at r23), where 15,000 frames is
too short for a plugged door to cost. The clear stays a crutch the trained ant needs; the ban could go.

**A plants-only lab case** (`scratch/inrun/labscen/lab_herbs.ron`, the owner's herb-bed opening
`herb_first_colony_near` as is: herbs at the nest grown 35,000 frames, then 52 ants, no food heaps). labsurvive,
after=15000, seeds 1-4, grant=1, mean headcount: the hand-built ant 68.1 / 69.3 / 67.8 / 61.5, the standard pool
10.0 / 21.8 / 11.3 / 14.7 (default grant, seed 1: 55.9 against 10.8). It is the playtest's complaint as a number
(the trained ant bites a plant at the same rate starving or fat), and no case in training has asked for it. A run
costs ~28 s against ~5 s for a far-food box. **Next, herbs1:** control against `lab_herbs` added to the lab case,
alone and with `drop`.

**herbs1** (the control against `lab_herbs` added to the summed lab case, alone and with `drop`): the herb bed did not
move. Each arm's 32 r23 carries in `lab_herbs` (labsurvive, seed = colony, after=15000, mean headcount): control
median 16.9, `herbs` 21.0 (ahead in 15 of 32), `herbsdrop` 17.5 (16 of 32); in `lab_far200_fruit` 64.9, 63.2, 61.9.
Gardens: `herbs` r23 nest 326 vs 383 (bigger 17/32) and starved 23 vs 6; `herbsdrop` even. The control matched
door1's to the digit (deterministic). **Why nothing moved:** ~17 ants on herbs is a sliver of a sum the far-food boxes
(~65 each) set. **Next, herbs2:** `labcase=lab_herbs` (new colsel arg), the herb bed as a lexicase case of its own.

**herbs2** (`labcase=lab_herbs`, the herb bed as a lexicase case of its own; alone as `hcase` and with `drop` as
`hcasedrop`; the control reran door1's to the digit): **the herb bed still did not move.** The in-round herb case
stayed flat (median 188 -> 174 tenths, r0 -> r23), and the r23 carries in `lab_herbs` (seed = colony) sit where the
control's do: median 17.3 and 17.4 against 16.9, ahead in 16 and 15 of 32. Two ways of asking failed the same way,
so the approach is wrong, not the weighting: whatever the hand-built ant does on herbs (~65 ants against ~17) is
not one mutation away in this pool, or this case cannot see it. Not tried yet: tracing why the trained founders
starve beside herbs (the playtest found the bite rate flat in hunger) before building a third case.

What the arms did win: the far-food box. `lab_far200_fruit` medians 71.6 (`hcase`, ahead 19/32) and 71.6
(`hcasedrop`, 24/32) against 64.9; `hcasedrop`'s garden nest at r23 480 against 383 (bigger 23/32) with food home
even (13/32). Four of `hcasedrop`'s best r23 colonies against colony 10 (both under `drop`), 8 fresh gardens with
food 100 out: every one brings more food home (c1 12,875, c12 13,720, c14 13,806, c17 12,709 against 10,785; higher
in 7-8 of 8) but digs a smaller or a cratered nest (c1 274, c12 239, c14 400 against 418; c17 479 but crater 9
against 3.5, worse in 8/8). No clean win, so colony 10 stays the standard ant.

**Next, fw1:** the standard pool carries 0 on every last-food weight (the keeper and scout seats both found the
memory reading into nothing), so selection never tried it. `fwseed05` / `fwseed15` draw each member's 16 `f::FOOD_WAY`
weights N(0, 0.5) / N(0, 1.5) and change nothing else; run on herbs2's build so `hcasedrop` is the control.

**fw1** (`fwseed05` / `fwseed15`, control `hcasedrop`, same build and garden worlds): **failed.** r23 gardens:
starved 26.5 and 29 against 10 (more in 26/32 and 24/31 paired by colony), `fwseed05`'s food home 10,442 against
15,824 (lower 27/32), `fwseed15`'s nest 305 against 480 (smaller 20/31); `lab_herbs` medians 148 and 112 tenths
against 178. A random last-food memory is a cost the pool pays before selection can find a use for it. Not carried.

**The nest, owner's ranking (2026-10-25 15:05Z): too small, then a crater for a mouth, then one room.** First
looked at, not measured: colony 8 and colony 10 against the hand-built ant on the same 4 garden worlds (food 100
out, 200k frames; pictures https://claude.ai/artifact/7CykXk4N8PiKzo1BEomvVz). The trained nest is one pit packed
with stored food: open cells below ground 70-150 against 980-1,250, stored food 430-630 against 25-65 (stock.csv
`nest_food` 398 against 48), and the open room stays flat all game while the store grows. `nestcount`'s `nest`
counts the food as room, which is why the earlier size numbers (316-545 against 1,116-1,384) undersold the gap.

**nest1** (from `std_pool`; `heaps=20:120` draws each food heap's distance per colony-round, owner: "randomly moving
the distance from the food"; `heapsgate` adds `scalegate=50`; `heapsgatewean` adds the old ant's roof and room rules,
`PP_BODYRULES=both`, weaned over rounds 6-20 and off in every garden and lab case, owner asked 15:15Z). r23 gardens
(median nest / food home / starved / rooms score), `hcasedrop` as the reference:

| arm | nest | food home | starved | rooms | open room, r23 training maps | stored food |
|---|---|---|---|---|---|---|
| heaps | 116 | 21,908 | 4.5 | 0 | 75 | 342 |
| heapsgate | 424 | 21,037 | 6 | 19 | 88 | 544 |
| heapsgatewean | 464 | 15,964 | 8.5 | 18.5 | 97 | 666 |
| hcasedrop | 480 | 15,824 | 10 | 3 | 110 | 491 |

Random distances alone shrink the nest by three quarters (smaller than `hcasedrop` in 31/32) while food home rises
(29/32). The gate buys it back (bigger than `heaps` in 31/32) and raises the rooms score (17/3), but **what it grows
is stored food**: open room stays 75-110 cells in every arm. The weaned rules came out level with the gate alone on
nest (16/16 against `hcasedrop`) and cost food. **Next, nest2:** `nestcount` now reports `nest_open` (open cells of
the flooded nest: hand-built ant 1,025, colonies 8 and 10 3-16, nest1's colonies 2-64), and `scaleopen=1` scores the
gate's case on it with `nestneed=6,3,1`. Arms `opengate` / `opengatewean`, controls nest1's gated arms (cached).

**The rooms score counts stored food as space** (replay https://claude.ai/artifact/1MgsAwA6bprQ1o73HaCs6B). nest1
`heapsgate` colony 9 (r23 garden, rooms score 85, the arm's best) is a food pile with 17 open nest cells; the
hand-built ant on the same world scores 36 on 732. `roomshape.OPEN_ONLY` / colsel `shapeopen=1` reads rooms from the
cells that are not stored food: colony 9 -> 0, colonies 8 and 10 0-1, hand-built ant 19-100.

**nest2** (`scaleopen=1 nestneed=6,3,1`: the size gate on open nest cells; controls nest1's gated arms, cached).
Open nest cells in the training maps, median r7 / r15 / r23 (pictures https://claude.ai/artifact/7PtyjnZKCrGeCNpUAJusqt):

| arm | open r7 | r15 | r23 | garden nest r23 | food home | crater |
|---|---|---|---|---|---|---|
| opengate | 4 | 6 | 6 | 76 (smaller 31/32) | 21,668 | 8 |
| heapsgate (control) | 7 | 6 | 15 | 424 | 21,037 | 7 |
| opengatewean (rules r6-r20) | 42 | 168 | 50 (more 24/8) | 666 (bigger 24/5) | 12,880 (lower 20/9) | 13 |
| heapsgatewean (control) | 38 | 28 | 26 | 464 | 15,964 | 6 |

The open-room score alone fails; with the old rules on, open room at r15 is six times the control's and loses two
thirds as the rules come off. On world 9099 the trained colony brings home 8x the hand-built ant's food (27,165
deliveries against 3,323) and burns most of it; its store only grows (18 -> 472 cells) while the hand-built ant's
stays 5-41 and ends at 6. **nest3** (the same arm weaned 8:40 over 48 rounds) was stopped 25 minutes in: the owner
(19:09Z) ruled the garden's food far too close, and asked for it farther and always moving, and for replays after most
runs.

**nest4** (`patches=20000:40:120` in training and garden: one heap 40-120 columns out on a random side, moving every
20,000 frames; 48 rounds; replays https://claude.ai/artifact/8SCJ54UxXuWroAWGgtShy6). **No arm improved.** Garden
starved per run (median, r15-r47) 220-276 for the no-rules control, 144-244 with the rules weaned 8:40, 110-211 with
that and `shapeopen=1`, against the hand-built ant's 11 on world 21099; 5, 9 and 15 of 32 colonies dead at r47. The
rules arms dig open nests (training open median up to 291 and 308) and die out as the rules go. The best survivors
hoard: the rooms arm's colony 21 keeps 83 ants with 40 starved in a 640-cell nest of which 87 cells are open and 655
cells of food lie in it. **Why:** colony 10 on `patches=20000:20:40` and `20000:30:60` (worlds 1099, 2099) died out
in 3 of 4 runs, starving 258-522 ants beside 175-315 cells of stored food that it never ate (store 350 -> 317 while
the colony died, frames 140k-180k): the moves kill the trained ant, not the distance. The brood lane found the same on
2026-10-23 (starving ants at home touching crumbs chose Walk 200 of 214 times). **Next, nest5:** the garden keeps the
far moving food; in training the heap first moves every 200,000 frames, and `tramp=200000:50:20000` shrinks that by
x0.8 each round the median colony starves 50 or fewer. `trampcrutch` adds `PP_HOMEEAT=1.0`, weaned r8-r28.

**nest5** (the move ramp, 32 rounds): **failed before it started.** The median colony starved 99-285 ants a round
with the heap never moving, so `tramp` never shortened the interval, and the garden on far moving food showed no gain
over nest4. `trampcrutch` (`PP_HOMEEAT` weaned r8-r28) was no better. A hunger probe (`PP_HUNGRYPROBE=1`, ants under
a quarter tank every 250 frames; replays https://claude.ai/artifact/HYDeKaY32fzLUdudCkFG3i) put 338 of 438 sightings
of colony 7's hungry ants in the outer 16 columns of the bed, the east wall included, 300 columns from the door.

## 2026-10-25 night: trained ants cannot come home; a teacher that turns them for home in time

**Why they die at the walls** (nest5 colony 7 replayed on world 21099, `trace=100`; the brain's own choices logged at
the walls with `PP_COPY_LOG`). 72 of 106 starved ants reached a wall while fat (median 13 tanks) and stayed there a
median 36,000 frames. At the wall the walk scores sit at +-0.99 whatever the home sense reads (`f::HOME` +-0.78,
`f::LAST_FOOD` +-1.34 between the step away and the step into the wall): of 1,469 decisions, 504 stepped away, 536
into the wall and 403 along it. The ant has no "go back"; at 300 columns a random walk home outlasts any tank. With
food 25 columns out this never cost anything, which is why nothing selected against it. The starvation trace of
2026-10-22 found the same of the hand-built ant (`/mnt/project-files/starve-trace/`): 50-62% of its starvation deaths
walked out fat and starved at the walls.

**The ceiling** (`PP_HOMEFAR`, the come-home teacher, acting on every decision it labels, `PP_CARRY=1`, with
`PP_CARRY_HUNGRY=2 PP_CARRY_HUNGRYONLY=1 PP_CARRY_OUT=0 PP_CARRY_STORE=1`; colony 10's carry, `patches=20000:40:120`,
worlds 21099-24099, 200,000 frames). Each step was found by tracing the dead of the step before:

| teacher | starved per world | alive at end |
|---|---|---|
| none | 217 / 381 / 476 / 145 | 23 / 0 / 37 / 0 |
| home under 4 tanks (`PP_HOMEFAR_HUNGRY=1`, R 40) | 24 each: founders start at 3 tanks, so never leave | 0 |
| home under 2 tanks, R 40 | 125 / 138 / 303 / 217 | 70 / 75 / 61 / 62 |
| empty and past 130 columns (the old rule) | 119 / 274 / 396 / 354 | 70 / 0 / 36 / 23 |
| home under 1 + 0.012 x columns tanks (`PP_HOMEFAR_FUEL`), R 1 | 104 / 144 / 184 / 51 | 70 / 65 / 35 / 71 |
| + a hungry ant in a pit climbs or cuts out | 91 / 145 / 144 / 82 | 59 / 66 / 68 / 64 |
| + a hungry pellet holder puts it down and goes home | 50 / 108 / 123 / 129 | 66 / 68 / 72 / 65 |
| + turn round rather than step into its own tail | **14 / 24 / 31 / 25** | **76 / 71 / 73 / 75** |

The hand-built ant on world 21099: 11 starved, 80 alive. What each row fixed: at a flat 2 tanks an ant 300 columns
out turned with ~165 columns of fuel and died on the way; fuel 0.025 was too timid (founders turned back before any
heap, all 24 died); the older hungry rule walked pellet holders *away* from home; ants sat in two-cell pits dug where
a heap had been, the home side packed; and one stood on one cell 2,500 frames because the only nearer step was its own
tail. The nest stays tiny with the teacher on (open 2-14 cells): this fixes feeding, not digging. Films:
https://claude.ai/artifact/Y2ERy6mXkeZYxZ7sg5KDYY.

**nest6** (the nest on far food that holds still, `heaps=40:120`) was stopped an hour in for **nest7**: the same three
nest4 arms' shape with the teacher weaned instead (`wean=8:40` now also rewrites `PP_CARRY`, the teacher's share; the
garden runs it at 0 and the lab strips it): `hctl` (no teacher), `hwean`, `hweanrules` (+ the old roof and room rules
on the same schedule). Judged with the teacher off.

**nest7** (48 rounds, far moving food; replays https://claude.ai/artifact/Q7w3bV8voybduVpourMBLF). Garden medians at
r47, missing colonies counted as dead:

| arm | alive | starved | nest | crater | colonies dead |
|---|---|---|---|---|---|
| `hctl` | 58 | 241 | 93 | 12 | 12 |
| `hwean` (teacher weaned r8-r40) | 0 | 218 | 108 | 15 | 19 |
| `hweanrules` (+ old rules weaned) | 66.5 | 289 | 617 | 138 | 4 |

**The weaned teacher did not stick**: once it was off, colonies starved as the control's did and more died out.
Selection has nothing to pick between: the walk scores are saturated (above), so a small change in how the brain
weighs home changes almost no choice. The habit has to be written into the weights (a copy), not selected for.
**The rules arm grew nests round after round** (garden nest 199 at r7 to 617 at r47), most of them cratered. Its
colony 13 is the exception: world 13099, 85 alive, 162 starved, nest 947 cells of which 386 open, crater 17, open
rooms score 29 (hand-built ant there: 86, 1, 1,085 open, crater 1, 80).

## 2026-10-26: colony 13 becomes the standard ant

Paired against the previous pool (price25b colony 10) on 8 fresh worlds (1007-8007), food far and moving,
everything off:

| | alive | starved | food home | nest | open | crater | rooms (open) |
|---|---|---|---|---|---|---|---|
| colony 13, median | 71.5 | 240 | 4,112 | 821 | 398 | 23 | 32.5 |
| previous pool, median | 10 | 428 | 5,790 | 194 | 16.5 | 29 | 0 |
| colony 13 higher in | 7/8 | 2/8 | 2/8 | 8/8 | 8/8 | 5/8 | 8/8 |

Lab boxes (labsurvive after=15000, seeds 1-4, mean headcount): far mix 102 vs 128 (worse), fruit 200 69 vs 62, herbs
12.8 vs 15.9. It is the first trained pool with a real open nest and rooms with every crutch off, and it keeps
colonies alive on far moving food; it carries less food home and is a fifth worse in the far-mix box.
`assets/minds/std_pool.txt` is now its carry (`mkpool.py`). **Next, nest8:** from this pool, no scaffold against the
old rules weaned again r8-r40, with and without `shapeopen=1`.

## 2026-10-26: copying the come-home teacher into the weights fails, and why the brain is hard to train by degrees

**The copy** (`examples/homefit.rs`): label colony 13's own runs with the come-home teacher (`PP_CARRY=0`, labels
only), then fit the labels into the top genome on the game's own `decide`, with every other choice held to the
original (KL on unlabelled rows), and add the delta to all 30 pool members. 8 test worlds, everything off, against
colony 13 (starved 240, food home 4,112):

| fit | what moved | agreement with the teacher (offline) | starved | food home | colonies dead |
|---|---|---|---|---|---|
| 1 | far-from-home links, light hold | 0.28 -> 0.63 | 184 | 35% lower | 2 |
| 2 | same, 3x hold | 0.42 | 254.5 | about the same | 0 |
| 3 | + one unit gated on energy | 0.45 | 204.5 | 3,811 (lower 7/8) | 1 |
| 4 | fit 3 refit on its own runs | 0.41 | 290.5 | 3,556 | 0 |
| 5 | that unit's whole 80 weights | 0.40 | 260.5 | 3,890 | 0 |

**Why none worked, traced** (world 1007, `trace=100`): the fitted ant agrees with the teacher 57-66% of the time on
colony 13's situations but only 33-38% on its own (copy-log field 7, the pick, against field 15, the label), and
hungry ants far out still drift *away* from home: -0.06 to -0.29 columns per 100 frames, against +2.25 with the
teacher acting (the positive control). Deaths stay 80+ columns out.

**The cause is the scorer, not the lesson.** An option's score is `squash(sharp x raw)`, and `sharp = 1 + g[SHARP]`
is capped at 4. The hand-built forager seed set `g[SHARP] = 3.0`, and every genome since (colony 13's 60: 5.3-6.7)
sits past the cap, so the gene is neutral to mutation. In colony 13's logged decisions 80% of walk scores are past
+-0.95, and in 72% of decisions every walk is a flat "no", drawn among almost at random. A graded leaning cannot
move such a pick, so selection and a weight fit both see only flips from "no" to "yes". Recomputed from the logged
scores, pinned walks: 80% at 4x, 62% at 2x, 31% at 1x, 4% at 0.5x. Colony 13 at a lower ceiling, 8 worlds: 2x
starved 257, open nest 350 (vs 240, 398); 1x starved 296, open 178. It lives; its constants were set at 4x.
`PP_SHARP_MAX=x` lowers the ceiling (default 4; byte-identical to editing the gene, checked).

## 2026-10-26: nest batch 8, nothing beats colony 13

From colony 13, 32 colonies x 48 rounds, far moving food: `gctl` (control), `grules` (old roof and room rules
weaned r8-r40), `grulesrooms` (same, open-room shape score). Garden at r47 (medians): control 68 alive, 311 starved,
nest 446, crater 93; rules 63, 294, **175**, 83; rules+rooms 72, 348, 579, 52. The four best by open room, 8 worlds
against colony 13:

| colony | alive | starved | food home | open nest | crater |
|---|---|---|---|---|---|
| colony 13 | 71.5 | 240 | 4,112 | 398 | 23 |
| rules+rooms c19 | 72 | 362 (8/8 worse) | 2,634 | 525 (6/8 better) | 21 |
| rules+rooms c10 | 56 | 355 | 2,339 | 477 | 65 |
| control c22 | 60 | 299 | 2,492 | 392 | 88 |
| control c11 | 45 | 378 | 2,617 | 362 | 106 |

Colony 13 stays the founder. Replays (c19, colony 13, hand-built, world 19099):
https://claude.ai/artifact/6R8cj4dx6dizbYCydqPmqi. **Next, nest9** (`jobs/nest9.txt`): control, `PP_SHARP_MAX=2`
and `PP_SHARP_MAX=1` from colony 13, to see whether selection can re-tune with graded choices visible.

## 2026-10-26: the sharpness batch (nest9) gains nothing; the copy at 1x stalls too

nest9 from colony 13, 48 rounds: `sctl` (byte-identical garden to nest8's `gctl`: same seeds, same build, so
selection is deterministic), `PP_SHARP_MAX=2`, `PP_SHARP_MAX=1`. Garden r47: control 68 alive, 311 starved, nest 446,
crater 93, open-room score 0.63; 2x 61, 332, 364, 128, 0.26, food home 4,282 against 3,184; 1x 55, 268, 382, 143,
0.15. The four colonies that starved least on their own world (75-109) were ordinary over 8 worlds (starved
224-318 against 240, open nest 171-250 against 398). Replays: https://claude.ai/artifact/WZWLZEvuNivcHrHQ6TF2XK.
Copying the come-home teacher at 1x (fit 6, labels relogged at 1x): agreement 0.30 -> 0.46, the plateau of fits
1-5, starved 253 against 296 at 1x, food home -22%, 3 of 8 colonies thinned below 15. **Pinned scores were not
the binding limit.**

## 2026-10-26: a "home, when low" sense; the lesson sticks and starvation falls ~5x

`PP_HOMELOW=k` writes, on every Walk option, +1 for a step towards home's column, -1 away, 0 along, times
`clamp(1 - e / (1 + k x columns from home), 0, 1)`, e in sensed tanks: the teacher's fuel gate as a graded reading,
0 for a fed ant. Fitting only its 16 unit weights (`homefit feat=18`) to the teacher's labels on colony 13's own
runs: agreement 0.28 -> 0.79 at hold KL 0.0006, against 0.40-0.63 at KL ~0.2 for every fit on existing links.

**Two traps on the way.** (1) The slot is `f::BUILDING`, which is `f::FOOD_WAY`, the last-food memory, on by
default: the first fits' weights read the food way wherever the sense was not written, and that pool delivered
nothing in 30k frames with the sense off. `PP_HOMELOW` now asserts `PP_LASTFOOD=off`. (2) Ungated, both far lab
boxes died (52 of 52 starved, every seed): ants land there at 0.4-0.9 tanks, so every one read "low" at once and
walked home to an empty nest. `PP_HOMELOW_STORE=1` reads 0 while no nest store holds food.

Clean fit, `PP_LASTFOOD=off PP_HOMELOW=0.006 PP_HOMELOW_STORE=1`, teacher off, against colony 13 as shipped:

| | alive | starved | food home | open nest | crater | rooms (open) | died out |
|---|---|---|---|---|---|---|---|
| colony 13, worlds 1007-8007 | 71.5 | 240 | 4,112 | 398 | 23 | 32.5 | 0 |
| sense, worlds 1007-8007 | 78 | 43.5 (8/8 lower) | 4,890 | 378 | 6.5 | 31.5 | 0 |
| colony 13, worlds 9007-16007 | 70.5 | 247 | 3,874 | 359 | 45 | 33 | 2 |
| sense, worlds 9007-16007 | 79 (8/8) | 44 | 5,039 (8/8) | 470 | 40 | 39.5 | 0 |

Colony 13 with `PP_LASTFOOD=off` alone starved 297.5, so the gain is the sense. k=0.012 lost 2 of 8 colonies
(founders turned home before reaching far food); 0.006 none. Lab (`labsurvive after=15000`, 4 seeds, mean alive):
far mix 110 vs 102, fruit 70 vs 69, herbs 18 vs 13. World 13099 (colony 13's own garden world) died out with the
sense, not yet traced. Pool: `assets/minds/homelow_pool.txt`. Replays:
https://claude.ai/artifact/JA6kxgzwxmxy4EoaEqir5B. **Next, nest10** (`jobs/nest10.txt`): nest training from that
pool with the sense on.

## 2026-10-26: nest batch 10, the come-home habit survives selection

nest10 from `homelow_pool` with the sense on (`PP_LASTFOOD=off PP_HOMELOW=0.006 PP_HOMELOW_STORE=1`), 48 rounds:
`hlctl` (control), `hlshape` (+`shapeopen=1`), `hlrulesrooms` (old rules weaned + open-room shape). Garden r47
(medians): control 80 alive, **41 starved** (nest8's control from colony 13: 311), nest 757, crater 38; shape 78, 56,
683, 45; rules+rooms 79, 72, 437, **183**. The habit held in every round (garden starved 32-77 in the two plain arms).
The four best colonies by open room, 16 worlds against the starting pool:

| colony | alive | starved | food home | open nest | crater | rooms (open) |
|---|---|---|---|---|---|---|
| homelow_pool | 79 | 43.5 | 4,944 | 392 | 9 | 32 |
| hlctl c10 | 77.5 | 41 | 5,071 | 439 | 32 | 40 |
| hlctl c2 | 84.5 | 41 | 5,043 | 452 | 44 | 32 |
| hlshape c31 | 78 | 58.5 | 4,166 | 523 (9/16 higher) | 32 | 30.5 |
| hlshape c18 | 78 | 55 | 4,310 | 462 | 67 | 55.8 |

None beats the start clearly, so it stays the founder. Selection keeps the habit but does not grow the nest:
48 rounds moved open room by at most a third, and the hand-built ant keeps 1,000-1,400. The dig thread's finding
(the old ant's labels: *which* cell is worth ~nothing, the gain is *when* to dig) suggests the same cure as the come-
home lesson, a "nest too small for the colony" sense read by the Dig job; it needs a slot (no free option feature
or context input remains), so it waits on the layout decision. Replays:
https://claude.ai/artifact/8bfwRq2tQWYEiZnBmS6ZS5.

## 2026-10-26: why the trained nest is small, and the "way out" sense for soil haulers

**Where the room goes** (homelow_pool with the sense on, 8 worlds; the nest flooded from the door): the ant cuts
~1,000 nest cells, and crumbs (half-eaten fruit put down, `assets/materials/crumbs.ron`) fill a median ~616 of them;
open ~381. The hand-built ant (world 13007): 1,438 open, 9 crumbs. The `spoil_in` count (~360, "soil let go inside")
is mostly not soil left in the nest: every release is the mind's own Drop by a fed ant, and about two thirds are at
row 0 of the door column, where the world clears soil (`PP_MIND_DOOR_OPEN=drop` keeps the clear).

**Choosing to dig is not the bottleneck.** Underground with empty jaws and a cut in reach, the ant picks Dig 2% of the
time (25 of 1,504 logged decisions): the Dig job's sum is about -6.3 (bias -5.65, hidden units -4.23, a negative
weight on the best cut's score). Two hand-set pushes on that job both failed:

| push (8 worlds vs homelow_pool) | open nest | cells cut | crater | starved |
|---|---|---|---|---|
| none | 378 | 1,046 | 6.5 | 43.5 |
| Dig job's depth weight 1.65 -> 5 / 9 / 14 | 364 / 404 / 263 | ~1,080 | 37-40 | 38-56 |
| + `k` x food cells in reach, k = 4 / 8 / 12 (not landed) | 282 / 159 / 158 | ~1,050 | 44-96 | 40-61 |

At k=8 the ant underground picked Dig at 30% of its chances, not 2%, and the cuts still rose only 7%: **hauling
is the bottleneck**. Half of all underground decisions belong to an ant carrying a soil pellet, and a third of its
steps go deeper (world 13007, ~166 decisions per pellet removed).

**`PP_SOILWAY=1`** (options.rs, scratch, needs `PP_HOMELOW`) writes home-low's slot on a pellet carrier's Walk
options inside a nest: +1 for a step nearer the door along `NestWay`, -1 for a step deeper, +1 for a step out. It
reuses the 16 weights fitted for home-low (both mean "this step heads home, and it matters now"); nothing was refitted.

| 16 worlds, far moving food (medians) | open nest | crumbs | cut | crater | starved | alive | food home |
|---|---|---|---|---|---|---|---|
| homelow_pool, worlds 1-8 | 378 | 616 | 1,046 | 6.5 | 43.5 | 78 | 4,890 |
| + way out, worlds 1-8 | 644 | 486 | 1,273 | 60 | 30.5 | 79 | 3,918 |
| homelow_pool, worlds 9-16 | 470 | 554 | 1,077 | 40 | 44 | 79 | 5,039 |
| + way out, worlds 9-16 | 546 | 474 | 1,180 | 123 | 43.5 | 78.5 | 3,976 |

The open nest was bigger in 12 of 16 worlds and the cuts more in 15 of 16, but the crater was bigger in 13 of 16.
Shallow cuts (0-7 rows) did not rise. The one cavern widens at the bottom (cuts 16+ rows deep: 1,051 against
843 per world) and its top slumps into a funnel. Replays: https://claude.ai/artifact/CdtFxitcZ9yau316uAV2hP.

**nest11** (3feb590a): colony selection from homelow_pool with the way-out sense on, `swctl` and `swshape` (the
analogues of nest10's `hlctl`/`hlshape`), 48 rounds. More colonies died out in training than in nest10 (7-26 of 32
alive per round in swctl, against 22-32). Garden r47: crater 103-173 against nest10's 38-45, nest no bigger. The
four best colonies, 8 worlds against the starting pool with the sense on, were all worse. Open nest 231-582
against 644, and cuts 630-1,000 against 1,273. Selection did not trade the crater away; it bred digging out.

## 2026-10-26/27: the gated brain (layouts 6 and 7), a hand-laid "home had food" memory, and selection losing it

Background: [`brain-cannot-combine-senses-2026-10-26.md`](../research/brain-cannot-combine-senses-2026-10-26.md) and
its review [`brain-design-review-2026-10-26.md`](../research/brain-design-review-2026-10-26.md). Every row below is 16
worlds (seeds s*1000+7), far moving food (`patches=20000:40:120`, `dist=25 grants=3 food=mix`, `PP_BED_W=448`,
`PP_MIND_DOOR_OPEN=drop`), teacher off, medians; "lab" is `labsurvive` over `scratch/inrun/labscen`, `grant=1`, 3 seeds.

**1. The review's gated come-home unit, alone, loses colonies.** Unit 8 refit from plain senses with a clamp gate (the
review's data folder, scratch `PP_GATE_FILE`, `PP_LASTFOOD=off`): 6 of 16 colonies extinct (home-low instinct 0,
colony 13 1). Where they lived, starvation was 2-19 against the instinct's 19-74. In the lab's far boxes every founder
starved (52/52, 0 deliveries); in the herb box it did much better (alive 67-121 against 19-77). Traced over the first
25k frames: every gated ant that starved died within 4 columns of home (41/41, 73/74, 53/57, 58/61), and hungry gated
ants headed home at +7 to +9 columns per 100 frames, to a nest with no store yet. The instinct avoids it only through
`PP_HOMELOW_STORE=1`. The ant had no input saying whether home holds food. The review's 4-world pass came from its
harness (no founding grant, store filled early): its own `world.sh` keeps 2007, 4007 and 8007 alive.

**2. Selection uses the gates (layout 6, `gate6`, 8d6fe613).** Colony selection from std_pool, gates trainable
(`g6open`) against frozen (`PP_GATE=off`), 32 colonies x 48 rounds, nest11's cases without instincts. Garden r47, open vs
frozen: alive 66 / 67.5, starved 304 / 311, food home 5,069 / 3,184, nest 133 / 446, crater 35 / 93. Every final
open-arm genome carries ~30 gate links. Zeroing the gate slots of the two best colonies (16 worlds each) cut food home
4,611 -> 3,052 (lower in 13/16) and 12,979 -> 2,630 (16/16), so the gates carry real foraging. But the rank-1 colony
(c11) is a no-dig colony: nest 21, open 1.5. Its gates cut, it digs again (nest 302) and loses 6 colonies against 2.

**3. A hand-laid "home had food" memory (layout 7).** Memory never sees option features, so food at home is invisible
unless acted on; layout 7 (PR #935) gives memory and the gates each verb's reach. Built by hand
(`scratchpad latch/mklatch_hb.py`, recorded in `scratch/inrun/genomes/latch_pool.txt`'s header) on colony 13's three
least-used memory units (8, 11, 9), rate 0:
- P = at the nest AND a PickUp target (Eat's only target is the crop, so the larder is PickUp's reach);
- N = at the nest AND no PickUp target AND energy below ~0.37;
- L = a flip-flop (self-weight 10, +15 from P, -15 from N, bias -2 so it starts "empty");
- unit 8's gate gains +3·L and its bias -2.7.

In world 2007, L was on in 81% of nest decisions with food in reach and 12% without; food in the field never set it.

| arm (16 worlds) | lost | starved | food home | open nest | crater | rooms |
|---|---|---|---|---|---|---|
| home-low pool + instinct | 0 | 43.5 | 4,944 | 392 | 9 | 32 |
| gate alone (G7) | 4 | 10.5 | 4,773 | 494 | 34 | 33 |
| latch, reset whenever at the nest without food in reach | 0 | 222 | 4,585 | 517 | 103 | 67 |
| latch, reset only when hungry, starts unset | 0 | 20.5 | 4,832 | 615 | 107 | 84 |
| **latch, reset only when hungry, starts "empty"** | **0** | **45.5** | **4,865** | **532** | **88** | **76** |
| latch units hijacked but unread | 9 | 52 | 172 | 68 | 39 | 8 |
| colony 13 | 1 | 273 | 3,895 | 447 | 60 | 36 |

The first latch reset itself at the door on every trip, so every hungry far ant read "empty". The unset start let the
gate open in the lab's far boxes (46-52 of 52 starved). Starting "empty" fixed that: far_mix alive 203-223 (instinct
198-229), far200_fruit 106-131 (111-138), herbs 15-34 (instinct 19-77). **The last row matches the instinct with no
instinct input.** The crater is worse, and the hijacked units cost colony 13 something real (row 6).

**4. Selection loses it (`latch1`, f0212d87).** From latch_pool (no instinct) against homelow_pool + instinct, same
cases, 48 rounds. Garden starvation went 46 -> 265 -> 179 -> 231 -> 298 -> 311 across r7-r47 in the latch arm, while the
instinct arm held 31-97. Nest fell in both arms (672 -> 143; 806 -> 539). The latch weights themselves mostly survived:
the hunger threshold drifted (-60 -> -40), and by itself that costs little (starved 57 against 45.5). Unit 8's feature
and gate rows moved (L1 14.5 and 8). **Putting unit 8 and the latch units back into the best evolved colony (c32)
cut starvation 291.5 -> 29.5, lower in 16/16 worlds.** So the evolved remainder can still use the habit; selection
lost it, and the instinct arm keeps it only because an input built from the answer cannot drift.

Taken together: the brain can now *represent* "come home when low, if home has food" from plain senses, and that
matches the instinct. Mutation-only colony selection did not *keep* it. Each of the last three nest batches also bred
the nest smaller. Whatever trains the next pool has to protect a taught habit, or select on it, before it can refine it.

## 2026-10-27: why selection lost the habit -- in-run mutation outruns it (`drift1`)

The owner asked why selection would drop a good habit at all. In `latch1`, selection did prefer colonies that starved
less: the half it kept starved a median 144 against 179 for the half it replaced, and the rank correlation between
starvation and rank was positive in all four recent arms (+0.23 to +0.64). Colony starvation also repeats across worlds:
13 evolved colonies on two fresh worlds, rank agreement +0.89. Yet garden starvation rose round after round in every
arm, the instinct arm too (35 -> 82). No ramp was on. `drift1` (a88231d9) ran the same latch pool and the same cases
three more ways:

| arm, from latch_pool | garden starved r7 -> r47 | garden nest r7 -> r47 | colonies in the r47 garden |
|---|---|---|---|
| lexicase, in-run mutation `kpm=0.013` (the default; `latch1`) | 46 -> 311 | 672 -> 143 | 13 of 32 |
| no colony selection (`plain`), default mutation | 174 (r7), 339 (r15), then extinct | 236 (r7), 223 (r15) | 3 by r15 |
| lexicase, mutation `kpm=0.003` | 51 -> 28 | 702 -> 700 | 32 |
| lexicase, no mutation (`kpm=0 kevents=0`) | 37 -> 37 | 890 -> 890 | 32 |
| home-low instinct, lexicase, default mutation (`latch1`) | 31 -> 71 | 806 -> 539 | 25 |

Mann-Whitney over the r47 gardens: starvation at `kpm=0.003` against the default, p = 6e-7; nest, p = 9e-6; its own
r7 against r47, p = 0.003 (it *improved*); against the instinct arm at r47, p = 0.0004; against no mutation, p = 0.2
(not distinguishable). **At the default rate a child carries ~26 changed weights (pm 0.013 over ~2,000 live links, sigma
1.0) plus 4 structural events, and every birth in a 200k-frame round mutates. That load outruns what colony
selection can repair, so every trait it does not re-select every round decays.** This is general: it bites the
instinct arm and the gate arms as well. It is not specific to the come-home habit. At a quarter of the rate the
same selection kept the habit, kept the nest, and lowered starvation further. The lab already had this
measured for in-lab evolution (enemy seat, PR #917: training's rate killed 3 of 6 lab colonies, a tenth stayed
within the off arm's spread).

## 2026-10-27: a lower rate is not enough; keeping the best half unchanged is (`self1`, `elite1`)

**`self1`** (88da3dc6): std_pool (no hand-laid habit), 32 colonies x 96 rounds, seed 31, gates and reach open. At a
quarter of the mutation rate, nothing improved. Starvation went 292 -> 281 (p = 0.8), and both food home (4,168 -> 2,720)
and the nest (663 -> 428) drifted down (p ~ 5e-6 each). At the default rate, food home rose 4,106 -> 7,852 (p = 1e-5),
paid for with the nest: 596 -> 129 (p = 9e-9), late cuts 328 -> 71. The home-low + instinct pool at the low rate lost
ground as well: starvation 46 -> 112 (p = 3e-5). drift1's latch pool held at the low rate only because it started at
the answer.

**`elite1`** (07c50df9) adds colsel's `elite=1`: a colony that kept its place last round runs with `kpm=0 kevents=0`.
Only the children that replace the bottom half (and round 0) mutate, so the kept half is carried unchanged. This is
the (mu + lambda) elitism a genetic algorithm uses against drift. Same seed, same pools, same cases as `self1`.

| arm (garden, round 7 -> 95) | starved | food home | nest | crater | late cuts |
|---|---|---|---|---|---|
| std_pool, kpm 0.003 | 292 -> 281 | 4,168 -> 2,720 | 663 -> 428 | 103 -> 120 | 326 -> 262 |
| std_pool, kpm 0.003, **elite** | 330 -> 281 (p = 2e-4) | 4,168 -> 5,714 (p = 1e-4) | 824 -> 776 (p = 0.4) | 76 -> 8 (p = 1e-4) | 372 -> 404 |
| std_pool, default kpm | 244 -> 207 | 4,106 -> 7,852 | 596 -> 129 | 89 -> 14 | 328 -> 71 |
| std_pool, default kpm, **elite** | 296 -> 219 (p = 6e-4) | 3,763 -> 4,242 (p = 0.1) | 626 -> 746 (p = 0.03) | 70 -> 31 (p = 0.008) | 316 -> 292 |
| home-low + instinct, kpm 0.003 | 46 -> 112 | 4,814 -> 6,142 | 769 -> 556 | 29 -> 25 | 376 -> 296 |
| home-low + instinct, kpm 0.003, **elite** | 38 -> 86 (p = 1e-7) | 4,945 -> 8,402 (p = 4e-9) | 870 -> 743 (p = 2e-4) | 22 -> 11 | 413 -> 352 |

Medians over the garden's colonies (26-32 of 32). Each p is a Mann-Whitney test of the arm's own r7 against its r95.
With four measures per arm the Bonferroni bar is 0.0125. At round 95, elite against non-elite at the low rate gives:

- food home 5,714 vs 2,720, p = 2e-8;
- nest 776 vs 428, p = 4e-6;
- crater 8 vs 120, p = 6e-8;
- starvation equal (281 vs 281, p = 0.6).

**With the kept half protected, selection now moves the std_pool arms the right way on every case or holds them**:
less starvation, more food home, the crater nearly gone, the nest kept. Without that protection, the same cases drifted
down at the low rate and traded the nest away at the default rate. It is still slow, and it has not found come-home in
96 rounds: std_pool starvation is 219-281 against the instinct pool's 38-86.

The instinct arm is the exception: its starvation rose even with elitism (38 -> 86) while food home climbed 70%. Kept
colonies cannot drift, so that rise is selection choosing it: a colony's rank is summed over ~10 cases, and children
with more food home outranked kept colonies that starved fewer. That is a trade in the cases, not drift.

## 2026-10-27 (afternoon): more rounds stall; the pool is one family; a held-out exam; why the game's boxes kill trained ants

Data for this section: [`colony-selection-log-2026-10-24/data-1027/`](colony-selection-log-2026-10-24/data-1027/) (garden
`cg.csv` per arm of drift1, self1, elite1 and elite2; exam tables). Reviews it draws on, all on main: the selection review
([`selection-review-2026-10-27.md`](../research/selection-review-2026-10-27.md), PR #939), the "home had food" landscape
([`latch-landscape-2026-10-27.md`](../research/latch-landscape-2026-10-27.md), PR #940), the environment review (PR #941)
and the write-gated memory (PR #942, layout 8).

**`elite2`** (elite1's three arms carried to 192 rounds, resumed at r96). Garden r95 -> r191, Mann-Whitney:

- std_pool kpm 0.003 + elite: flat. Starved 281 -> 257 (p = 0.8), food home 5,714 -> 6,466 (p = 0.04, not past 0.05/6).
- std_pool default kpm + elite: food home rose 4,242 -> 5,416 (p = 7e-4), and starved rose with it, 219 -> 340 (p = 4e-5).
- home-low + instinct + elite: flat (starved 86 -> 72, p = 0.4).

Under this selection, progress stops around round 50. The selection review explains why. One round's colony rank
barely repeats on a second world (0.05-0.09), and a mutated child is worse than its unmutated donor on lab survival
in 63-69% of pairs.

**The pool is one family** (`scratch/inrun/lineage.py`). In every lexicase arm of drift1, self1 and elite1, all 32
colonies descend from **one** round-0 colony by round 7-15. At elite1's round 95, all 930 carried ants in an arm
share one std_pool member as their family root (p16, p06, p04 by arm). Two colonies' best genomes differ in 22-68 of
~2,000 live weights; two ants of one colony differ in 7-16. std_pool itself is nearly a clone: two of its members
differ in a median **35** weights. The ratchet ("the best colony becomes the next founder") restarts every batch from
one colony. This sits against the owner's "pool, not one brain" rule.

**A held-out exam** (`scratch/inrun/exam.py`, `exam_cmp.py`). Nothing selection saw: garden worlds 500001-500016, and
8 of the game's own boxes from `assets/lab_scenarios`, as shipped (no founding grant, 60,000 frames after landing,
2 seeds). The 16-world harness this lane used until now (seeds s*1000+7) was colony s's round-7 training world.

- Held-out garden, against std_pool, 16 paired worlds, sign test:
  - elite1's e3 winner (c32): deliveries 5,807 vs 4,040 (15/16, p = 5e-4); starved 252 vs 262 (n.s.); open nest
    361 vs 512 (3/16 better, p = 0.02).
  - e13 winner (c7): deliveries 5,177 vs 4,040 (14/16, p = 0.004).
  - home-low + instinct pool: starved 43.5 vs 262 (16/16, p = 3e-5).
  - ehl3 winner (c5) against that instinct pool: deliveries 8,719 vs 4,629 (16/16), but starved 75 vs 43.5 (2/16
    better, p = 0.004), open nest 223 vs 428 and rooms 9.5 vs 69 (1/16, p = 5e-4).
- Game boxes: mean ants alive over the run:

  | brain | two_larders | scattered_larder | feast_and_famine | played_bed | herb_first_colony | crowded_bench |
  |---|---|---|---|---|---|---|
  | hand-built ant | 147 | 37 | 34 | 162 | 113 | 419 |
  | std_pool | 4 | 5 | 4 | 127 | 62 | 181 |
  | trained winners and instinct pool | 4 | 4-5 | 4 | 105-140 | 48-106 | 84-188 |

  far_larder and the_bank starve every brain (2-4). Every trained brain is below the hand-built ant on lived, alive
  and deliveries over the 16 box-seeds (p <= 0.004).

**Why the boxes kill trained ants.** The environment review found that 9 of the 36 shipped boxes feed on windfall or
corpse, which training never offered. Swapping those foods in two_larders took the trained pool from 3.6-7.3 ants
lived to 37-101 (8/8 seeds). This lane traced which part matters (`PP_MIND_LOG`, two_larders with windfall in both
heaps against fruit and provisions at the same spots, seed 1, 2,500 frames):

- **Windfall** reads as food (`f::FOOD`). A PickUp option existed in 104 of 34,031 decisions with windfall, against
  1,120 of 35,894 with fruit. When there was one, the ants took it at a similar rate (32% vs 24%).
- Windfall **rots**: `decays_into` soil, chance 0.15 dry and 0.7 damp per 200-frame tick, so a heap lasts about
  1,300 frames dry. The trained ants hug the walls and the divider and arrive after it has gone. The lesson is a
  fast search for short-lived food, not a windfall sense.
- **Corpse** is never offered to a mind ant. PickUp needs `diet_yield` above the threshold at the ant's own gut, and
  corpse's `food_class` +1 against a gut near -1 gives quality about 0. The hand-built ant's 1,557 "deliveries" in an
  all-corpse box are unexplained: possibly separate forage code, or a gut that evolved in that box. Its put-down count
  runs 4-12x real (`how-we-test.md`).

**Also from the environment review:** the std_pool garden has 15/16 colonies alive at 200k but 0/16 at 600k on the
same bed (hand-built ant 14/16), and training's harness-only rules (lay cap, the door ban, no old age, grants) each
make the game harder than training. Lifting all of them at once kills every colony by 400k.

**Built for the next batches** (all off by default; at bc8485fb on the lane branch):

- colsel `refound_mut=PM:SIGMA:EVENTS` with `carry=random`: mutation happens once, when a colony is refounded.
  `sel=` on the job line now reaches inrun.
- One `seed_of()` for every world seed. The old sums collided from round 99.
- `donor600=FRAMES[:Q]`: donors must also hold up in one long run.
- inrun `food=all`: each moving heap draws provisions, fruit, windfall or corpse.

**Still running at the time of writing:** `clean1` (jobs/clean1.txt, 32 x 48, seed 31). Its arms:

- k3 (control);
- kdec (colony selection only);
- kgoal (one case per goal, gates 25);
- kboth (kdec and kgoal together);
- ktone (in-colony selection kept, mutation only at refounding);
- kev1 (kevents=1).

Round-23 garden: no arm clearly lower on starvation than k3 (kev1 248 vs 276, p = 0.04). kgoal and kboth lost
nest (564 and 471 vs 872, p < 1e-3), as expected once nest has one vote instead of six.

## 2026-10-27 (evening): moving where mutation and selection happen does not lower starvation (`clean1`)

**Question.** The selection review (PR #939) blamed three things for the stall: the in-colony ant score fighting the
colony cases, child colonies damaged by mutation, and nest holding most of the lexicase vote. `clean1` tested a fix for
each against the current setup, from std_pool, 32 colonies x 48 rounds, seed 31 (`scratch/inrun/jobs/clean1.txt`, built
at bc8485fb). **Deciding measure, named before the run:** garden starvation at round 47 against k3 (Mann-Whitney over
colonies), with nest as the guard. Then the held-out exam (`exam.py`) of each arm's rank-0 colony at round 47.

| arm | what changes against k3 |
|---|---|
| k3 | control: `elite=1 kpm=0.003` (the elite1 setup) |
| kev1 | one structural event per birth instead of four (`kevents=1`, the review's §5.1) |
| ktone | in-colony selection kept; mutation only once, when a colony is refounded (`refound_mut`) |
| kdec | colony selection only: no in-colony mutation or selection, random carry, `refound_mut` |
| kgoal | one case per goal (`lex+crater+scale+lab`), lab and scale gates at 25 |
| kboth | kdec and kgoal together |

**Training garden** (median over colonies with a carry; 26-29 of 32 per arm at round 47; data `data-1027/clean1/`):

| arm | starved r47 | vs k3 | nest r47 | vs k3 | starved r7 -> r47 | food home r7 -> r47 |
|---|---|---|---|---|---|---|
| k3 | 292 | | 814 | | 252 -> 292 (p = 0.8) | 4,347 -> 4,873 (p = 0.008) |
| kev1 | 227 | p = 0.18 | 812 | p = 0.63 | 274 -> 227 (p = 0.05) | 4,346 -> 5,178 (p = 7e-4) |
| ktone | 325 | p = 0.08 | 766 | p = 0.72 | 286 -> 325 (p = 0.04) | 4,224 -> 5,370 (p = 0.001) |
| kdec | 260 | p = 0.59 | **403** | **p = 5e-5** | 300 -> 260 (p = 0.3) | 4,412 -> 4,082 (p = 0.7) |
| kgoal | 264 | p = 0.36 | 740 | p = 0.25 | 310 -> 264 (p = 0.005) | 4,066 -> 4,587 (p = 0.04) |
| kboth | 266 | p = 0.97 | **594** | **p = 4e-4** | 244 -> 266 (p = 0.3) | 4,697 -> 5,222 (p = 0.08) |

No arm is lower than k3 on starvation at round 47. kev1 is the only arm that improved on its own start in both
starvation and food with the nest held, but against k3 it is not significant. At round 23 kgoal and kboth had lost
nest (564 and 471 vs 872, p < 1e-3); by round 47 kgoal had recovered it and kboth had not.

**Held-out exam** (16 garden worlds 500001+, sign test paired by world; Bonferroni bar 0.007 over 7 measures; data
`data-1027/exam/c1*`), each arm's rank-0 colony against k3's (c23):

- kev1 (c6): starved 286 vs 267 (8/16), deliveries 5,470 vs 5,198 (9/16), nest 839 vs 964 (5/16). No difference.
- ktone (c22): deliveries 6,318 vs 5,198 (13/16, p = 0.02), starved 314 vs 267 (3/16 better, p = 0.02), nest 762 vs
  964 (2/16, p = 0.004).
- **kdec (c5): deliveries 6,575 vs 5,198 (15/16, p = 5e-4), but nest 290 vs 964 (0/16, p = 3e-5); against std_pool,
  open nest 27 vs 512 and rooms 0 vs 47 (0/16, p = 3e-5), alive 60 vs 71 (1/15, p = 0.001).** Colony selection alone
  sold the digging for food.
- kgoal (c5): starved 226 vs 267 (12/16, p = 0.08), deliveries 4,762 vs 5,198 (3/16, p = 0.02).
- kboth (c17): nest 588 vs 964 (2/16, p = 0.004); against std_pool, open nest 106 vs 512 (0/16).
- k3 itself against std_pool: deliveries 5,198 vs 4,040 (13/16, p = 0.02); nothing else moved.
- The game's lab boxes: unchanged. Every clean1 winner lives 4-5 in two_larders, scattered_larder and
  feast_and_famine, like std_pool, and no arm differs from k3 on lived (best kev1 11/14, p = 0.06).

Only kdec's and kboth's nest losses, and kdec's deliveries gain, clear the Bonferroni bar.

**Lineage** (`lineage.py`): every arm, random carry included, is down to one round-0 colony by round 15-31, as in elite1.

**What it says.**

1. Where mutation happens is not what holds starvation up. Moving it to refounding (ktone, kdec) or making each birth
   gentler (kev1) left starvation where it was in 48 rounds.
2. Taking the in-colony score out entirely (kdec) is worse than useless here. The in-colony score was carrying the
   digging: without it the carried ants are random, and colony selection on its own let the nest go.
3. Rebalancing the cases (kgoal) moved weight from nest to the other goals, as designed. On held-out worlds that came
   out as slightly less starvation and fewer deliveries, neither significant.

Together with elite2's stall, this points away from the selection mechanics and toward what the colony is offered and
judged on: the environment review's windfall gap and 200k horizon, and a brain that has no stepwise path to "come home
when low" (PR #940, write gate PR #942). **Recommendation:** keep k3's setup (`elite=1 kpm=0.003`) as the training
default; do not adopt `refound_mut`/`carry=random`. `kevents=1` is harmless and may be kept. Next batches are env1
(windfall in training, the long-run donor gate) and write1 (`PP_WRITE=on`), as planned.

## 2026-10-27 (night): new account; training defaults; more than one family saved; the G1 baseline exam

**Defaults (plan A1, the owner did not object).** colsel `elite=1` is now on unless a job says `elite=0`, and inrun's
`STANDARD` mutation rate is `kpm=0.003` (was 0.013). Every batch since elite1 passed both by hand, so those batches
are unchanged; a job file from before elite1 that relied on the old defaults now needs `elite=0 kpm=0.013` to repeat.

**Pools for diversity** (`scratch/inrun/mkpool.py` from the bucket's `small/` copies; each file's header carries its
held-out garden numbers): `assets/minds/elite1_e3c32_pool.txt`, `elite1_e13c7_pool.txt`, `elite1_ehl3c5_pool.txt`
(needs the home-low instinct it was trained with) and `clean1_k3c23_pool.txt` (clean1's k3 rank 0 at r47). All four
are layout 7 (genome 4097); std_pool is layout 5.

**The launcher works from this account.** `vmproof-1027` (c4d-highcpu-8, ~5 minutes) started, built, ran an exam line,
uploaded and pulled. A short SHA cannot be fetched by the VM (`g1base-1027` died on it); `runvm.sh` now expands it.
`inrun_cloud.py` takes `OUT exam P BOXES SEEDS AFTER FOUNDERS [ENV]` lines, and `exam.py` takes `AFTER=`.

**G1 baseline (`g1base2-1027`, launched 22:05Z, built at 4e4807a7 = main c67f37e1 + this lane's commits; the exam
binary, labsurvive, is main's).** The bar G1 is judged on, measured before anything is trained against it. Brains:
the hand-built ant, std_pool, and the four saved pools above except ehl3c5 (which needs an instinct). Boxes: the six
where the hand-built ant lives (two_larders, scattered_larder, feast_and_famine, played_bed, herb_first_colony,
crowded_bench), seeds 1-8, as shipped, **600,000 frames after landing**. **Deciding measures, named before the run:**
ants alive at the end and starved, paired by box and seed against the hand-built ant, sign test per box. Results and
the bar follow in the next section.

## 2026-10-28 (00:30Z): windfall and carrion in training -- `foodwild`, and the `env1` batch

**Corpse belongs in.** The authored ant's gut is neutral (`assets/species/ant.ron` traits slot 0 = 0.0), so the
hand-built ant digests corpse (quality 0.25) as well as plants (0.25). The trained ants' gut is a body gene open in
training (`body=default`, [body genes](body-genes-2026-10-22.md)) and drifted toward the plant end (-1), where corpse
is worth nothing: that is why a mind ant is never offered corpse at its gut, and the hand-built ant's corpse
deliveries need no separate forage code (inferred from the code, not traced in a run). Offering corpse in training
gives selection a reason to keep the gut where both foods pay.

**`food=all` as built is not a training bed.** 4 held-out worlds (500001-500004), 200k, garden bed, sel=off (data
`data-1027/foodwild/sweep.txt`):

| arm | std_pool colonies alive at 200k | hand-built ant colonies alive |
|---|---|---|
| `food=mix` (today's training) | 4 / 4 (64-80 ants) | 4 / 4 (79-99) |
| `food=all` (every heap uniform over provisions, fruit, windfall, corpse) | **0 / 4** | **1 / 4** (75; 0, 0, 4) |
| `foodwild=0.5` (half the heaps windfall or corpse) | 0 / 4 | 2 / 4 |
| `foodwild=0.25` | **3 / 4** (64-100) | **4 / 4** (77-91) |

With `patches=` there is one heap at a time and it jumps every 20,000 frames, so a heap the colony cannot use is a
20,000-frame famine. Every colony lost under `food=all` had a wild first or second heap (traced from `patches.csv`):
30 founders on 3 tanks cannot wait for the next. `foodwild=P[:KINDS]` (new, inrun) makes each jump after the first
windfall or corpse with chance P, from the same stream, so place and timing match `food=mix` on the seed; the founding
heap is never wild. At 0.25 the hand-built ant loses nothing and std_pool loses one world in four: hard, with spread.

**A selection box for the new foods:** `scratch/inrun/labscen/lab_far_wild.ron` is `lab_far_mix` with windfall west
and corpse east. Seed 1, 15k after landing: hand-built 80 alive (lived 51.8), std_pool 75 (44.5); on `lab_far_mix`
175 and 220. The exam's boxes stay held out.

**The G1 baseline exam was preempted.** `g1base2-1027` lost its spot VM at ~22:30Z with 14 of 30 box lines done, and
the watcher died with this session's worker, so nobody noticed for two hours. What came back: every trained pool
(std_pool, e3c32, e13c7, k3c23) is extinct by 600k in two_larders, scattered_larder and feast_and_famine in 8/8 seeds,
mean headcount 0.3-0.5 in all but four runs (7.5-16.8), so they die within a few thousand frames. The hand-built ant
at 600k: feast_and_famine 0/8 alive (lived median 8.1), scattered_larder 3/8 alive (lived 14.9). At 60k it lives in
8/8 of each (copy-test lane, seeds 11-18). So G1's "boxes where the hand-built ant lives" must be re-based at 600k, and
`lived` (mean headcount, graded) is the measure that can still separate brains there. Fixes so it cannot happen again:
`exam.py LAB_SEEDS=A-B` (one VM line per seed), and a relaunched batch skips every job line an earlier launch in the
chain finished (`runvm.sh` passes the chain; `inrun_cloud.py` checks the bucket).

**`env1`** (`scratch/inrun/jobs/env1.txt`): founder `clean1_k3c23_pool.txt` (the ratchet's latest product of the
default setup; in the three boxes above every candidate founder dies alike), 32 colonies x 24 rounds, garden every 8,
`elite=1 kpm=0.003`, 16 cores per arm:

| arm | change against `c` |
|---|---|
| c | control: today's training (`food=mix`, lab cases lab_far_mix + lab_far200_fruit) |
| w | `foodwild=0.25` (windfall and corpse) and the lab_far_wild case |
| wf | `foodwild=0.25:windfall` (windfall only) and the lab_far_wild case: whether corpse matters |
| w6 | w plus `donor600=600000` (donors must also hold up over one 600k run) |

**Deciding measure, named before launch:** the held-out exam of each arm's rank-0 colony at r23 against c's, `lived`
paired over the 6 boxes x 8 seeds at 600k (48 pairs), two-sided sign test, Bonferroni over the three comparisons
(p < 0.0167). Guard: garden r23 open nest not lower (extinct colonies counted as 0). Food home is not a deciding
measure: the pheromones lane found it double-counts a drop-and-repick loop at the nest edge (2026-10-28), and in
colsel it is one case of ten. Run `selaudit.py` on the first 16 rounds.

## 2026-10-28 (01:20Z): `env1` training gardens -- carrion is a famine for every brain, windfall is not

`env1-1028` (built at 17f03045, launched 00:24Z, done 01:19Z: 24 rounds x 4 arms in 55 minutes on 64 cores).
Data: `data-1027/env1/` (cg.csv, colsel.csv and `selaudit.py` output per arm).

| arm | colonies extinct per training round (of 32) | garden r23: colonies with a carry, alive (extinct = 0), starved |
|---|---|---|
| c (control) | 1-9, median 4 | 28, 61, 259 |
| w (windfall + corpse, 25% of heaps) | **15-24, median 21** | 16, **0**, 244 |
| wf (windfall only) | 0-7, median 3 | 29, 59.5, 251 |
| w6 (w + donor600) | **17-29, median 25** | 3, **0**, 146 |

**Correction to the 00:30Z section: corpse is not a learnable food on this bed, for any brain.** One held-out world
(500001, 60k, `foodwild=1:KIND` so every heap after the first is that food): corpse heaps, hand-built ant 7
deliveries after 20k and extinct, std_pool 80 and extinct; windfall heaps, hand-built 559 and 80 alive, std_pool
1,208 and 65 alive. The gut argument above was inferred and is wrong as the explanation: the bed's mind ants carry
the species' authored body (the `body=` switch is not on main), and the hand-built ant fails on corpse here too. Why
corpse feeds the hand-built ant in the game's boxes and no one on this bed is open (hand-off to-do 5). **Corpse comes
out of training** until it is.

**Windfall alone changes nothing in the garden** (wf against c at r23: alive 59.5 vs 61, starved 251 vs 259). Its
selection audit looks like elite1's (ranksum retest 0.05, children 54% of the top half, children worse than their
donor on lab 233 of 365 pairs). Whether it changed the game boxes is the deciding measure: the exam batch `x1`
(rank-0 colonies at r23: c c9, w c17, wf c28, w6 c22; plus the copy-test lane's pruned copy
`scratch/inrun/genomes/copytest_c3_pruned.raw.txt` from PR #970, and the 16 lost baseline box sets), 6 boxes x 8
seeds at 600k, one VM line per seed.

## 2026-10-28 (03:10Z): exam `x1` at 600k -- every pool trained since std_pool is worse in the game; the 600k donor check is the one change that helped

`x1-1028` (VM, built at 7630b2e8): the four env1 rank-0 colonies at r23, the copy-test lane's pruned copy of the
hand-built ant (`c3p`, ~804 billed links), and the 16 baseline box sets `g1base2-1027` lost to preemption; with
`g1base2`'s 14 finished lines that is 10 brains x 6 boxes x 8 seeds at `after=600000`, `lived` = mean headcount
over the 600k. Data: `data-1027/x1/lab600k.csv`; `python3 exam_cmp2.py` in that folder reprints every line below.
64 cores for about 1 h 10 min (g1base2 + x1 together about 110 core-hours).

**Deciding measure (named at 00:30Z), env1 arms against their control `c`, lived paired over 48 box-seeds,
Bonferroni p < 0.0167:**

| arm | higher / lower than c | p | passes |
|---|---|---|---|
| w (windfall + corpse) | 14 / 18 | 0.6 | no |
| wf (windfall only) | 17 / 13 | 0.58 | no |
| w6 (w + `donor600=600000`) | **27 / 6** | **0.0003** | **yes** |

w6 against w (the same arm without the donor check): 30 / 1, p = 3e-8. Starved does not separate any env1 arm. So
the windfall in training did nothing for the game boxes; the 600k check on donors did.

**The ratchet line has gone backwards in the game boxes** (lived, 48 pairs against std_pool): clean1 k3c23 6 / 30
(p = 7e-5), elite1 e3c32 6 / 26 (p = 5e-4), elite1 e13c7 10 / 24 (p = 0.024), env1 c 5 / 32 (p = 7e-6). Only w6 is
level with std_pool (20 / 13, p = 0.3). Every one of those pools won its own training round on the bed: the bed's
score and the game boxes have come apart, which is why the next founder is std_pool, not k3c23.

**Lived per box, median over 8 seeds** (alive at 600k in brackets):

| box | hand-built | std_pool | k3c23 | env1 w6 | c3p (pruned copy) |
|---|---|---|---|---|---|
| two_larders | 172.2 (4/8) | 0.4 (0) | 0.4 (0) | 0.4 (0) | 17.8 (5/8) |
| scattered_larder | 14.9 (3/8) | 0.5 (0) | 0.4 (0) | 0.5 (0) | 0.7 (0) |
| feast_and_famine | 8.1 (0/8) | 0.4 (0) | 0.3 (0) | 0.4 (0) | 1.1 (0) |
| played_bed | 246.8 (6/8) | 154.4 (8/8) | 151.1 (8/8) | 182.3 (8/8) | 3.4 (1/8) |
| herb_first_colony | 306.8 (8/8) | 144.1 (8/8) | 98.8 (8/8) | 214.0 (8/8) | 10.4 (4/8) |
| crowded_bench | 644.0 (8/8) | 198.4 (8/8) | 166.4 (7/8) | 243.0 (8/8) | 24.5 (5/8) |

The hand-built ant beats std_pool on 45 of 48 (p = 1e-10). c3p splits clean by box: better than std_pool in
scattered_larder and feast_and_famine (8 / 0 each, p = 0.0078) and two_larders (7 / 1, p = 0.07), worse in the
three plant boxes (0 / 8 each).

**The three windfall boxes are not a training-score problem.** Every trained pool's colony is gone a few thousand
frames after landing there (lived 0.3-0.5 over 600k with 52-104 ants is about 2,000-6,000 frames), with 0-19
deliveries and no births; the hand-built ant delivers 1,500-226,000. A founding grant does not rescue it (std_pool,
seed 1, 60k: two_larders 0 deliveries with or without 3 tanks; scattered_larder 7 and 6). These boxes are sized to
the hand-built ant's idle burn (food 92% of 52 ants x 0.10 x 6,000), and with no food a tank lasts the shipped pool
~1,000 frames against the hand-built ant's ~3,700 (old-ant thread, 2026-10-25, `labsurvive` row of
`Reports/instruments.md`). Being traced: whether it is the brain bill alone (`tax=`) or the colony never reaching
the windfall at all (two_larders' heap is 60 columns from the door and gets no delivery).

**Proposed G1 bar** (to the owner, 2026-10-28): per box, the trained pool's lived median at least 70% of the
hand-built ant's and the paired sign test against it not significant (p >= 0.05), in 5 of 6 boxes, 8 seeds, 600k.
Interim milestone: beat std_pool on the same 48 pairs. One exam of one brain is 48 runs, about 6 core-hours.

### env2 (launched 03:13Z): the ratchet restarted from std_pool, with the donor check

| arm | change |
|---|---|
| s_c | control: the env1 `c` setup, founded from `assets/minds/std_pool.txt` |
| s_d6 | `donor600=600000` |
| s_d6hp | s_d6 plus `homepick=12` (the Nest lane's re-carry switch, PR #967: a cell picked up within 12 columns of the door is not paid again) |
| s_d6nr | s_d6 plus `nest_r=0` (dig pay everywhere, no 20-column band; the Nest lane's garden screen: a bigger nest, no extinctions) |

32 colonies x 24 rounds, P=16 each. **Deciding measure, named before launch:** the exam of each arm's rank-0 colony
at r23, lived paired over the 6 boxes x 8 seeds at 600k: s_d6 against s_c, s_d6hp and s_d6nr against s_d6
(Bonferroni over 3, p < 0.0167), and each against std_pool for the ratchet. Guard: garden open nest not lower.
Jobs: `scratch/inrun/jobs/env2.txt`.

### Famine twins for selection (03:35Z)

Two selection cases in `scratch/inrun/labscen`, twins of the exam's famine boxes, which stay held out:
`lab_famine_scatter` (scattered_larder's food budget, colony at x 400, shallower soil, scatter over 30-450) and
`lab_famine_trickle` (feast_and_famine's 74% of idle burn for one colony of 52 at x 256: 2 windfall cells every
1,000 frames at x 290, 24 every 12,000 at x 60). Positive control, seed 2, colsel's own 15k after landing:

| case | hand-built ant: lived, alive | std_pool: lived, alive |
|---|---|---|
| lab_famine_scatter | 51.4, 48 | 16.6, 2 |
| lab_famine_trickle | 33.3, 23 | 16.5, 3 |

So the cases separate the brain that holds out in a famine from one that does not, inside colsel's 15k budget. They
go into the next batch's `labscens=`, and the copy-test lane fits its generalist copy on them (not on the exam
boxes; c3 was fitted on the exam's own famine boxes at seeds 1-4, so its wins there are not held out at box level).

### env2's exam measure, amended before it runs (03:40Z)

The watchdog lane (2026-10-28): the lab case's 15k headcount does not predict survival (env1 c's 28 r23 carries to
300k on their own lab seeds: Spearman -0.14 against alive at 300k; c9, the rank 0, dead in both boxes), and
headcount rises with a hatching boom that starves (trained colonies starve a median 67% of what they hatch there;
x1 at 600k: 75-93%, the hand-built ant 22%). So `lived` alone can reward the wrong colony. **env2's exam reports
three paired measures over the 48 box-seeds, and an arm passes only if lived passes and neither of the others is
significantly worse:** lived; alive at 600k; starved / (founders + births). Same comparisons, Bonferroni over 3.
Also found: the env1, clean1 and elite1 job lines dropped `shapeopen=1` (on since nest3, gone after nest11); the
watchdog finds either shape version noise (rho ~0), so it comes back for consistency, not as a lever.

## 2026-10-28 (04:55Z): `env2` training gardens -- all four arms alike on the bed; exam `x2` launched

`env2-1028` (built at bbccb08c, launched 03:13Z, done 04:49Z: 24 rounds x 4 arms in 1 h 36 min on 64 cores, the
donor600 check about 40 min over env1's 55). Data: `data-1027/env2/` (cg.csv, colsel.csv, donor600.csv, and
`selaudit.py` for s_c and s_d6).

| arm | colonies extinct per round (of 32) | garden r23: colonies with a carry, alive median, starved median, nest median | rank 0 at r23 |
|---|---|---|---|
| s_c | 0-4, median 2 | 28, 72, 276.5, 872 | c31 |
| s_d6 | 0-5, median 1 | 31, 69, 279, 787 | c5 |
| s_d6hp | 0-8, median 3 | 31, 74, 265, 968 | c13 |
| s_d6nr | 0-4, median 1.5 | 30, 72, 285.5, 859 | c32 |

The bed cannot tell the arms apart, as with env1. s_d6's selection audit: ranksum retest 0.04 [-0.10, 0.19], so
which colony is ranked first in a round is close to a coin toss between rounds; only home8 (0.26), scale (0.32) and
lab (0.15) repeat at all. Saved the four rank-0 carries as `scratch/inrun/genomes/env2_{cc31,d6c5,d6hpc13,d6nrc32}_pool.txt`.

**Exam `x2`** (`scratch/inrun/jobs/x2.txt`, 336 lines): the four env2 carries, env1's w6c22, std_pool and the
hand-built ant, 6 boxes x 8 seeds at 600k, every brain on the same build so the controls are paired with the
judged. Measures as named at 03:40Z (lived, alive at 600k, starved / (founders + births)).

## 2026-10-28 (05:55Z): exam `x2` -- env2 made std_pool worse; the donor check did not replicate; homepick hurt

`x2-1028` (built 4d1f9fcf, 04:53-05:52Z, 336 lines, about 64 core-hours). Data: `data-1027/x2/lab600k.csv`;
`python3 x2an.py` in that folder reprints every line. std_pool's and the hand-built ant's 48 rows came out
**identical to x1's** on a later build, so the exam is deterministic across these builds and x1's other rows stay
comparable.

| comparison (48 box-seeds) | lived | alive at 600k | starved share (lower is better) |
|---|---|---|---|
| s_d6 vs s_c (the donor check) | 11 / 23, p = 0.058 | **6 / 19, p = 0.015** | 26 / 9, p = 0.006 (worse) |
| s_d6hp vs s_d6 (homepick=12) | **5 / 26, p = 0.0002** | 11 / 12 | 14 / 21 |
| s_d6nr vs s_d6 (nest_r=0) | 9 / 21, p = 0.043 | 11 / 14 | 19 / 18 |
| s_c vs std_pool | 8 / 21, p = 0.024 | 9 / 15 | 22 / 14 |
| s_d6 vs std_pool | 10 / 22, p = 0.05 | 5 / 20, p = 0.004 | 30 / 7, p = 0.0002 (worse) |
| env1 w6c22 vs std_pool | 20 / 13, p = 0.3 | 12 / 12 | **10 / 26, p = 0.011 (better)** |

Bonferroni bar p < 0.0167. **No env2 arm passes against its control, and every env2 colony is worse than its own
founder in the game boxes.** The donor check, which passed in env1 (27 / 6 from the k3c23 founder), does not repeat
from std_pool: s_d6 is lower on every measure, significantly so on alive. homepick=12 costs heavily (lived 5 / 26),
in the plant boxes (herb_first_colony lived median 50 against s_d6's 81, crowded_bench 100 against 156), where it
also had 3 of 8 colonies dead. The windfall boxes: every brain but the hand-built ant 0.4-0.5 again.

**Reading.** The ratchet has now gone backwards on its last five founders' worth of batches: 24 rounds of today's
selection from any founder lands below that founder in the game. With selection close to noise (s_d6 ranksum
retest 0.04) and no training case predicting 600k survival (watchdog: 311 donors, every case within +-0.25),
what 24 rounds do is drift, and the drift is downhill for the game. env1's w6 is the one pool level with std_pool,
and it starves a smaller share (p = 0.011, a guard measure, so not a pass); it does not become the lab ant (B1
needs a pool that beats std_pool). Lab default unchanged.

### env3 (launched 05:56Z): select on what the game asks, not on the bed

Founder std_pool, 32 colonies x 24 rounds, all with donor600 (as s_d6, the reference), `shapeopen=1` back, and the
famine twins as their own lab cases (`labcase=lab_herbs,lab_famine_scatter,lab_famine_trickle`):

| arm | P | lab case scored on |
|---|---|---|
| t_fam | 16 | mean headcount over 15k (today's) |
| t_fama | 24 | alive at 100k (`labscore=alive:100000`) |
| t_famak | 24 | alive at 100k less 0.1 x starved (`labscore=alive:100000:0.1`; 0.1 picked after looking at 28 carries, so untested) |

A 100k lab run costs about 15x a 15k one (lab_far_mix, std_pool: 108 s against 7 s), hence the wider P on the two
alive arms. **Deciding measure, named before launch:** the exam of each arm's rank-0 colony at r23, paired over the
48 box-seeds at 600k: t_fama and t_famak against t_fam on lived, alive at 600k and starved share (an arm passes on
lived with neither other measure significantly worse; Bonferroni over 2), and each against std_pool. The same exam
carries the copy-test lane's generalist copy `g1-twins` (fitted on the four twins only).

## 2026-10-28 (09:35Z): `env3` gardens -- at 100k the famine twins score every colony 0; exam `x3` launched

`env3-1028` (built 57c3bc22, 05:56-09:26Z, 3 h 30 min on 64 cores). Data: `data-1027/env3/`.

| arm | extinct per round (median of 32) | garden r23: carries, alive, starved, nest (medians) | lab_herbs case r0 -> r23 (median) | famine twins r0 -> r23 | rank 0 r23 |
|---|---|---|---|---|---|
| t_fam (15k headcount) | 2 | 27, 70, 270, 541 | 134 -> 193.5 | scatter 177 -> 174, trickle 148.5 -> 158 | c7 |
| t_fama (alive at 100k) | 1 | 32, 66, 302, 704 | 400 -> 830 | 0 -> 0, 0 -> 0 | c9 |
| t_famak (alive100k - 0.1 x starved) | 1 | 32, 63, 249.5, 556 | 93 -> 295.5 | -51 -> -51.5, -52 -> -52 | c32 |

**The famine twins carried no signal in the 100k arms**: every colony is dead there by 100k, so the case is a
constant (0, or minus the founding colony's starved). In t_fam, at 15k, they did not move in 24 rounds. Selection
did move the herb case in both 100k arms (median doubled and tripled). Whether any of it reached the game: exam `x3`
(`scratch/inrun/jobs/x3.txt`, 336 lines), the three r23 rank-0 carries, the copy-test lane's generalist copy
`g1-twins` (fitted on the four twins only, PR #975) and its specialist c3, std_pool and the hand-built ant, 6 boxes
x 8 seeds at 600k, measures as named at 05:55Z.

## 2026-10-28 (10:40Z): exam `x3` -- alive-at-100k selection made it worse; famine cases at 15k held level with std_pool; the generalist copy is still a specialist

`x3-1028` (built b5860d0b, 09:29-10:31Z, 336 lines). Data: `data-1027/x3/lab600k.csv`, `python3 x3an.py`.
std_pool's rows match x1's again; the hand-built ant's played_bed median moved 246.8 -> 242.6 (main carried plant
changes by then), so this exam's pairs, all on one build, are the ones to read.

| comparison (48 box-seeds) | lived | alive at 600k | starved share |
|---|---|---|---|
| t_fama (alive at 100k) vs t_fam | **7 / 21, p = 0.013 (worse)** | 10 / 14 | 21 / 16 |
| t_famak (alive100k - 0.1 x starved) vs t_fam | 12 / 17, p = 0.46 | 9 / 15 | 16 / 20 |
| t_fam vs std_pool | 20 / 14, p = 0.39 | 12 / 12 | 23 / 14 |
| t_fama vs std_pool | 8 / 22, p = 0.016 | 9 / 15 | 22 / 16 |
| g1-twins vs std_pool | 23 / 25 | **6 / 24, p = 0.0014 (worse)** | 21 / 26 |
| g1-twins vs c3 | 20 / 24 | 11 / 10 | 25 / 23 |

**No pool beats std_pool, and scoring the lab case on alive at 100k made the winner worse**, not better: the famine
twins are dead by 100k for every colony, so that case went flat, and what selection did move -- the herb case,
doubled -- did not carry to the game's herb box (t_fama herb_first_colony lived median 126 against t_fam's 188).
t_fam is the first env from std_pool not to land below it (env2's s_d6, the same setup without the famine cases
and shapeopen: 10 / 22); it is level, not a pass. The generalist copy g1-twins, fitted on the twins only, keeps the
copy's split on held-out boxes: two_larders alive in 6 of 8 (std_pool 0), the plant boxes near zero (played_bed
lived 1.7 against 154).

### Exam `x4` (launched 10:45Z): a mixed pool

The copies live where std_pool dies and die where it lives. A colony's pool can hold both: `mixh` is g1-twins x 30
plus std_pool's 30 members (half the eggs carry each), `mixq` g1-twins x 10 plus the 30 (a quarter). Same exam, 96
lines (`scratch/inrun/jobs/x4.txt`), on x3's build so x3's std_pool and hand-built rows pair with it. std_pool's
members as raw genomes: `scratch/inrun/genomes/std_pool_raw/`. **Deciding measure, named before launch:** lived,
paired against std_pool over the 48 box-seeds, Bonferroni over the two mixes; alive at 600k and starved share as
guards. A pass makes it the candidate lab ant (B1), with a side-by-side picture first.

## 2026-10-28 (11:00Z): exam `x4` -- a mixed pool lands between its parts, level with std_pool

`x4-1028` (built f6e0ee5c, the x3 engine plus records only; 10:37-10:53Z, 96 lines). std_pool, hand-built and
g1-twins rows copied from x3 into `data-1027/x4/lab600k.csv`; `python3 x4an.py`.

| comparison (48 box-seeds) | lived | alive at 600k | starved share |
|---|---|---|---|
| mixh (half g1-twins) vs std_pool | 25 / 20, p = 0.55 | 9 / 18, p = 0.12 | 23 / 23 |
| mixq (a quarter) vs std_pool | 21 / 21 | 7 / 17, p = 0.064 | 20 / 21 |
| mixh vs g1-twins | 26 / 18 | **25 / 6, p = 0.0009** | 21 / 27 |

Neither mix passes. Half the eggs on the copy buys two_larders (alive 3 of 8, lived median 6.9 against std_pool's
0.4) and costs the plant boxes (crowded_bench 117 against 198, herb_first_colony 92 against 144). The two famine
boxes that kill everyone fast (scattered_larder, feast_and_famine) do not move for any pool.

**Where the day leaves G1.** Four batches and four exams: no trained, copied or mixed pool beats std_pool, and
the hand-built ant is far ahead of all of them (lived median 200 against 58; 45 of 48 pairs). Selection on the bed
drifts downhill (env1, env2), a game-like lab score did not fix it (env3), and the copies are specialists (x3, x4).
Next: the copy-test lane is fitting one copy on both kinds of label -- the hand-built ant's moves in the famine
twins, std_pool's own in the plant twins -- due about 13:30Z; it gets the exam, and if it is level or better it
founds a batch with the famine cases (t_fam's setup, the one env that did not land below its founder).

### env4 (launched after #977 merges): t_fam's setup, one world per round, and from env1's w6

While the copy-test lane fits its two-label copy, the VM runs t_fam's setup (the one batch that held level with its
founder) two ways, 32 colonies x 24 rounds, P=32 each:

| arm | change against t_fam |
|---|---|
| f_ow | `oneworld=1` (Selection lane: every colony plays the same world each round, so a round ranks colonies, not worlds; ranksum retest in env2/env3 was -0.04 to 0.10) |
| f_w6 | founded from `scratch/inrun/genomes/env1_w6c22_pool.txt` (level with std_pool on lived, a smaller starved share, p = 0.011) |

**Deciding measure, named before launch:** the exam of each rank-0 r23 colony, lived paired over the 48 box-seeds
at 600k against t_fam's c7 and against std_pool (Bonferroni over 2 per arm), alive at 600k and starved share as
guards. Pass against std_pool -> candidate lab ant (B1), picture first.

**Same VM, exam `x5`**: the copy-test lane's two-label copies, both fitted from std_pool member 0 on the hand-built
ant's moves in the famine twins and std_pool's own in lab_herbs and lab_far_mix (exam boxes never seen):
`copytest_g2.raw.txt` (1,137 billed links) and `copytest_g3.raw.txt` (plant labels weighted x3, 981 links). 96
exam lines beside env4's two colsel lines (P=20 each), all in `scratch/inrun/jobs/env4.txt`. src/, assets/ and
examples/ are unchanged since x3's build (b5860d0b), so x3's std_pool and hand-built rows pair with them. Deciding
measure: lived against std_pool over the 48 box-seeds (Bonferroni over 2), alive and starved share as guards.

**f_ow is void** (12:50Z): under `oneworld=1` inrun seeded the births' stream from the world's seed, so colonies
sharing a carry in one world ran identically (Selection lane, selow1: 1-2 distinct outcomes a round). PR #979 adds
`sseed=` and has colsel pass each colony its own. f_ow ran before it and is not examined; it is rerun after #979.

## 2026-10-28 (13:20Z): env4 and exam x5 in; exam x6 launched

**x5** (the copy-test lane's two-label copies, 96 lines, 11:30-11:50Z; std_pool and hand-built rows from x3, engine
unchanged since b5860d0b): `data-1027/x5/lab600k.csv`, `python3 x5an.py`.

| comparison (48 box-seeds) | lived | alive at 600k | starved share |
|---|---|---|---|
| g2 vs std_pool | 9 / 21, p = 0.043 | **4 / 20, p = 0.0015 (worse)** | 21 / 19 |
| g3 (plant labels x3) vs std_pool | 15 / 19 | 9 / 16 | 17 / 25 |

The plant labels brought std_pool's walk back and took g1's famine walk with it: two_larders alive in 0 and 1 of 8
(g1 6 of 8). g3 beats std_pool in crowded_bench (lived median 232 against 198) and loses played_bed (95 against 154)
and herb_first_colony (76 against 144).

**env4** (built 4532a298; spot preemption at 11:53Z with the 96 exam lines done, the two colsel lines rerun whole on
a resume VM, 11:53-13:11Z). Data: `data-1027/env4/`. f_w6: extinct per round median 2, garden r23 alive 71.5,
starved 236.5 (t_fam's 70 / 270); rank 0 at r23 c31, saved as `scratch/inrun/genomes/env4_w6c31_pool.txt`. f_ow is
void (above).

**x6** (`scratch/inrun/jobs/x6.txt`, 144 lines): f_w6's c31, the copy-test lane's g4 (`copytest_g4.raw.txt`, famine
labels x2, 983 links) and the Selection lane's 2-island winner (`selisl1_isl2c6_pool.txt`, k3 line + islands=2:4,
round 47). Same engine as x3, so x3's std_pool and hand-built rows pair with all three. Deciding measure: lived
against std_pool (Bonferroni over 3); alive and starved share as guards; f_w6 also against its founder env1 w6c22
(x1/x2 rows, same engine).

## 2026-10-28 (13:55Z): exam `x6` -- env4's winner, copy g4 and the islands winner all land below or level with std_pool

`x6-1028` (built 0e2f00f7, 13:15-13:35Z, 144 lines; std_pool and hand-built rows from x3, env1 w6 from x2, all the
same engine). Data: `data-1027/x6/lab600k.csv`, `python3 x6an.py`.

| comparison (48 box-seeds) | lived | alive at 600k | starved share |
|---|---|---|---|
| env4 f_w6 c31 vs std_pool | **6 / 25, p = 0.0009 (worse)** | 6 / 18, p = 0.023 | 12 / 25, p = 0.047 (fewer starve) |
| env4 f_w6 c31 vs its founder env1 w6c22 | **8 / 23, p = 0.011 (worse)** | 6 / 18 | 23 / 19 |
| copy g4 vs std_pool | 12 / 21 | 8 / 17 | 20 / 19 |
| selisl1 isl2 c6 r47 vs std_pool | **6 / 25, p = 0.0009 (worse)** | 5 / 19, p = 0.0066 | 29 / 8, p = 0.00075 (more starve) |

The islands winner was level with std_pool on the Selection lane's own check (garden 200k and the lab case at 15k);
at 600k in the game boxes it is clearly below. g4 keeps g1's two_larders colony alive in 1 of 8 (g1 6 of 8) and
beats std_pool in crowded_bench (lived median 294 against 198), below it elsewhere.

**Six exams, one pattern: 24-48 rounds of selection lower the game score of whatever founds them** (k3c23, env1 c,
env2's four arms, env3's alive arm, f_w6 against w6c22, isl2), and the one batch that held level (t_fam) did not
repeat from another founder (f_w6, same setup). The bed's judges and the 600k bed check choose on something the
game's boxes do not reward.

### env5 (launched 14:00Z): a long lab-box check between a candidate and the next generation

colsel `donorlab=FRAMES[:Q[:A,B]]` (new, off by default): after the gates, each donor candidate's carry lands in
lab_herbs and lab_far_mix (selection twins, not exam boxes) on its own seeds and runs FRAMES after landing; the
top half by (alive at the end, mean headcount) stay donors. Smoke-tested on 4 colonies x 2 rounds. t_fam's setup
otherwise (its r23 c7, already examined in x3, is the control: the default path is unchanged, so a rerun would be
identical):

| arm | P | change against t_fam |
|---|---|---|
| e_dl | 21 | + `donorlab=200000` |
| e_dlo | 21 | `donorlab=200000` in place of donor600 |
| e_dl4 | 22 | `donorlab=400000` in place of donor600 |

**Deciding measure, named before launch:** each arm's r23 rank-0 colony, lived paired over the 48 box-seeds at 600k
against std_pool and against t_fam c7 (Bonferroni over 3 arms), alive and starved share as guards.

**Exam baseline moves at 36e25d65** (2026-10-28 14:57Z, PR #973: shrubs resprout from the base, default on). It
changes the plant exam boxes (played_bed, herb_first_colony, crowded_bench), so the std_pool and hand-built rows of
x1-x6 (all built before it) pair only with brains examined on a build before it. Every exam on a main at or after
36e25d65 reruns std_pool and the hand-built ant in the same batch.

## 2026-10-28 (17:15Z): env5 in -- the long twin run did not climb inside training; exam `x7` queued

env5 ran 24 rounds in 3 h 16 min (13:49-17:05Z). Rank-0 colonies at r23: e_dl c28, e_dlo c28 (different genomes),
e_dl4 c31, written as `scratch/inrun/genomes/env5_{dlc28,dloc28,dl4c31}_pool.txt`.

What `donorlab` saw (OUT/donorlab.csv: each donor candidate's carry, alive at the end summed over lab_herbs +
lab_far_mix, mean over that round's candidates):

| arm | r0 | r5 | r11 | r17 | r23 |
|---|---|---|---|---|---|
| e_dl (200k) | 377 | 72 | 202 | 213 | 296 |
| e_dlo (200k) | 368 | 264 | 148 | 210 | 316 |
| e_dl4 (400k) | 227 | 84 | 94 | 49 | 74 |

So keeping the better half of the candidates did not lift the long twin survival of the next generation: all three
arms end at or below round 0, and the 400k arm falls by two thirds. The gates and the round's other cases drive the
pool, and `donorlab` only chooses among what they pass. This is training-side; the exam decides.

**Exam x7** (288 runs, launches on the VM after the watchdog's twinrank-1028, ref 677b51a7, which has #973): the
three env5 winners, t_fam c7 (`env3_famc7_pool.txt`, rerun here because x3 was pre-#973), std_pool and the hand-built
ant, 6 boxes x 8 seeds at 600k. Deciding measure as named at launch above.

**twinrank-1028** (for the watchdog lane, VM 17:10Z, ref 0704e089, pre-#973 to match the carries): the 32 t_fama
r23 carries plus old and std, lab_herbs + lab_far_mix at 200k and herb_first_colony + played_bed at 600k, seeds 1-3.
Asks whether the twins rank colonies as the exam does (the watchdog's Spearman, fail below 0.3 or p >= .05).
`exam.py` now takes `PIXEL_PHYSICS_LAB_SCENARIOS` from the job env (default unchanged) so a job can run the twins.

## 2026-10-28 (19:20Z): exam `x7` -- the long lab-twin check picks no better than chance; nothing beats std_pool

x7-1028 (288 runs, ref 677b51a7, after #973; 18:21-19:14Z), `data-1027/x7/lab600k.csv`, `x7an.py`. Paired over the 48
box-seeds at 600k, up/down (Bonferroni over the 3 env5 arms: p < 0.017):

| against std_pool | lived | alive at the end | starved share |
|---|---|---|---|
| e_dl c28 (donor600 + donorlab 200k) | 18 / 20, p 0.87 | 17 / 7, p 0.064 | more starve 25 / 11, p 0.029 (not after Bonferroni) |
| e_dlo c28 (donorlab 200k in place of donor600) | **4 / 26, p 6e-5** | **4 / 21, p 0.0009** | 23 / 16 |
| e_dl4 c31 (donorlab 400k in place of donor600) | **7 / 25, p 0.002** | 10 / 16 | 23 / 14 |
| t_fam c7 (control, rerun on this build) | 13 / 21, p 0.23 | 14 / 11 | more starve 25 / 12, p 0.047 |
| hand-built ant (std vs old) | std 4 / 44, p 1.5e-9 | std 10 / 25, p 0.017 | std more 48 / 0 |

Against t_fam c7, the arm env5 changes: e_dl 14 / 16 (level), e_dlo 5 / 23 (p 0.0009), e_dl4 10 / 23 (p 0.035).
Per box, every trained brain still dies in two_larders, scattered_larder and feast_and_famine (lived 0.3-0.5, at most
1 of 8 seeds alive); e_dlo also collapses in herb_first_colony (lived median 15, 3/8 alive) and crowded_bench (18,
3/8). Median lived over 48: std 70, t_fam c7 58, e_dl 50, e_dl4 60, e_dlo 0.7; hand-built 184.

**Reading.** Adding the long twin check on top of donor600 changes nothing; putting it in place of donor600 makes the
game worse. That fits the watchdog's twinrank verdict (18:25Z): at 200k, lab_herbs does not agree with itself seed to
seed (Spearman lived 0.00; lab_far_mix +0.20) and the twins rank the 32 t_fama carries against the exam boxes at
+0.06, so `donorlab` with one seed per box is close to a random pick among the gated candidates, while donor600 (the
garden at 600k) carries some signal. The old-vs-std control does show on the twins (lived 325 vs 146), so they see
large differences, not sibling ones. On this build std_pool and t_fam c7 are level (13 / 21), so t_fam's famine
cases neither helped nor hurt; no trained pool beats std_pool, and the lab default stays std_pool (B1 waits for one
that does).

**Next, launched 19:18Z: twinself-1028** (816 runs, ref 3c0ebbc6). Can any non-exam lab box steer selection at 600k?
Five labscen boxes (lab_herbs, lab_far_mix, lab_mid_fruit, lab_far_wild, lab_far200_fruit) at 600k with the 32
t_fama carries, 3 seeds, beside herb_first_colony, played_bed and crowded_bench rerun on this build. Deciding measure
(before launch): a box can steer if its seed-to-seed Spearman of lived over the 32 colonies (mean of the 3 seed pairs)
is >= 0.3, and its 3-seed lived sum ranks the exam boxes' 9-run sum at Spearman >= 0.3 with permutation p < 0.01
(0.05 / 5 boxes). The famine twins are left out: every trained colony is dead there, so they cannot rank siblings.

## 2026-10-28 (21:30Z): twinself -- no lab twin can steer at 600k either; the exam's plant boxes can

twinself-1028 (816 runs, ref 3c0ebbc6, 19:18-21:15Z), `data-1027/twinself/twinself.csv`, `twinself_an.py`. The 32
t_fama r23 carries, 3 seeds. "self" is the box's seed-to-seed Spearman of lived over the 32 colonies (mean of the 3
seed pairs); "vs exam" is its 3-seed lived sum against the exam plant boxes' sum (for an exam box, the other two):

| box | self | vs exam | perm p | dead / 96 | median lived | hand-built | std_pool |
|---|---|---|---|---|---|---|---|
| lab_herbs | -0.00 | +0.11 | 0.28 | 73 | 3 | 93 | 74 |
| lab_far_mix | +0.22 | -0.18 | 0.84 | 82 | 103 | 489 | 119 |
| lab_mid_fruit | -0.09 | -0.19 | 0.85 | 90 | 79 | 301 | 97 |
| lab_far_wild | +0.08 | +0.09 | 0.30 | 93 | 49 | 306 | 52 |
| lab_far200_fruit | +0.27 | -0.01 | 0.52 | 91 | 84 | 311 | 110 |
| herb_first_colony (exam) | +0.47 | +0.62 | <0.0001 | 3 | 142 | 278 | 134 |
| played_bed (exam) | +0.16 | +0.41 | 0.010 | 1 | 143 | 292 | 172 |
| crowded_bench (exam) | +0.24 | +0.64 | <0.0001 | 1 | 204 | 649 | 211 |

(hand-built and std_pool: 3-seed mean lived.) All five twins summed against the exam: -0.08. **Verdict against the
measure named at launch: no twin steers** (none reaches self >= 0.3; none ranks the exam). The three exam plant boxes
rank colonies alike (+0.41 to +0.64), so sibling colonies do differ in a way the game sees.

**The dead column is the finding.** At 600k most trained colonies are dead in every twin (73-93 of 96 runs, where
`dead` is 0 ants at the end), against 1-3 of 96 in the exam plant boxes. lab_herbs differs from herb_first_colony
only in where its two herbs stand (x 240/270, at the nest, against 128/384), yet 73 against 3 dead. The twin runs
also carry colsel's lab env (`PP_MIND_DOOR_OPEN=drop`, which every colsel job since env1 sets; `PP_BED_W` only
widens the in-run bed). So either the box or the door rule kills them. **twinenv-1028** (204 runs, launched 21:30Z)
separates the two: herb_first_colony with `door=drop`, lab_herbs without, same founders and seeds.

## 2026-10-28 (22:10Z): twinenv -- the box kills, not the door rule; twingame launched

twinenv-1028 (204 runs, ref e9ed27cd, same engine as twinsself), `data-1027/twinenv/`, `twinenv_an.py`, paired per
colony-seed (32 x 3) against twinself's rows:

| arm | dead / 96 | median lived | hand-built | std_pool |
|---|---|---|---|---|
| herb_first_colony, plain (twinself) | 3 | 142 | 278 | 134 |
| herb_first_colony, `door=drop` | 2 | 137 | 278 | 134 |
| lab_herbs, plain | 75 | 1.1 | 93 | 33 |
| lab_herbs, lab env (twinself) | 73 | 2.5 | 93 | 74 |

door=drop against plain: herb_first_colony 46/50 (p 0.76), lab_herbs 17/15 (p 0.86): **the door rule does nothing
measurable at 600k**, so the watchdog's plugged-door lead is closed for these boxes. With the rule held, lab_herbs
against herb_first_colony is 1/95 (p 2e-27): moving the two herbs from x 128/384 to 240/270, at the nest, kills 3 in
4 trained colonies and costs the hand-built ant two thirds of its headcount. The twins are harsher boxes, not
broken ones: they kill what the exam boxes keep, so they cannot rank siblings that all die in them.

**twingame-1028** (714 runs, launched 22:10Z): seven game boxes that are not among the six exam boxes
(played_bed_scrambler, played_bed_understory, played_bed_windfall_reach, steady_income, the_bank, far_larder,
boom_bust), same 34 founders, 3 seeds, 600k, plain env. Deciding measure as twinself's (self >= 0.3 and vs exam >= 0.3
at p < 0.01, now 0.05 / 7), against twinself's exam rows (same engine). A box that passes is a candidate steering
case; the three played_bed variants are close kin of an exam box, so a pass there is weaker evidence of a general
signal than one in steady_income, the_bank, far_larder or boom_bust.

## 2026-10-28 (23:45Z): twingame -- two played_bed variants rank colonies as the exam does; env6 launched on them

twingame-1028 (714 runs, ref f948010c, 22:12-23:22Z), `data-1027/twingame/`, `twingame_an.py`, against twinself's
exam rows (same engine):

| box | self | vs exam | perm p | dead / 96 | median lived | hand-built | std_pool |
|---|---|---|---|---|---|---|---|
| played_bed_scrambler | +0.00 | **+0.52** | 0.0010 | 1 | 143 | 271 | 145 |
| played_bed_understory | +0.27 | **+0.52** | 0.0015 | 0 | 171 | 261 | 153 |
| played_bed_windfall_reach | +0.04 | +0.13 | 0.25 | 2 | 160 | 226 | 155 |
| steady_income | +0.01 | -0.15 | 0.80 | 18 | 133 | 277 | 155 |
| the_bank | +0.21 | +0.03 | 0.43 | 96 | 0.2 | 0.4 | 0.2 |
| far_larder | +0.00 | +0.00 | 1.0 | 96 | 0.2 | 0.4 | 0.2 |
| boom_bust | +0.16 | -0.04 | 0.59 | 96 | 13 | 20 | 9 |

**Against the measure as named, no box passes**: none reaches self >= 0.3. Two pass the exam half at p < 0.05/7.
the_bank and far_larder kill even the hand-built ant (lived 0.4) under labsurvive's plain landing, so they say
nothing here. After the named measure (exploratory, so weaker): scrambler + understory summed over their 6 runs rank
the exam at **+0.67** (p < 0.0001); with 2 seeds each +0.61, 1 seed each +0.45. The variants are played_bed with one
thing added, so the kinship control ranks them against herb_first_colony + crowded_bench only: +0.59 (p 0.0002),
where played_bed itself scores +0.41. The self bar was too strict for a summed check: the exam's own played_bed
agrees with itself at +0.16 and still ranks the other two at +0.41.

**env6** (launched 23:45Z): t_fam's setup from std_pool on this build, three arms, P 21 each:

| arm | change |
|---|---|
| g_ctl | none (t_fam with donor600: the control, rerun on this engine) |
| g_dl | + `donorlab=600000:50:played_bed_scrambler,played_bed_understory,played_bed_scrambler,played_bed_understory:lived` |
| g_dlo | the same donorlab in place of donor600 |

colsel's `donorlab` gains a fourth field, `lived`, that ranks on mean headcount first (the quantity the rank checks
validated) and runs a box named twice on two seeds. Smoke (4 colonies, 1 round, 3k): c1 had more alive at the end
(145 vs 128) and lower lived (138.2 vs 140.8), and `:lived` kept c2 where the default would keep c1.
**Deciding measure, named before launch:** exam x8 (6 boxes x 8 seeds at 600k), each arm's r23 rank-0 colony, lived
paired over the 48 box-seeds against g_ctl's and against std_pool (Bonferroni over 2 arms), alive and starved share
as guards. The donorlab log's own lived by round says whether the filter lifts the next generation (it did not in env5).

## 2026-10-29 (11:30Z): exam `x9` -- the hand-written dig head (H3) lives in the game as well as std_pool

H3 (the Nest lane's poolH3: std_pool with a hand-written dig head, g[2797:3279] only; Scott: seed training with it only
if colonies live) through the six exam boxes x 8 seeds at 600k, run in this lane's container on bb0d7570 (same engine
as x7: std_pool played_bed seed 1 rerun beside it gave the identical row). `data-1027/x9/lab600k.csv` (H3 rows + x7's
std, old, famc7), `x9an.py`. Paired over 48 box-seeds:

| H3 against | lived | alive at the end | starved share |
|---|---|---|---|
| std_pool | 19 / 14, p 0.49 | 13 / 13 | 21 / 15, p 0.41 |
| hand-built ant | 2 / 46 | 8 / 27 | 48 / 0 more |

Per box (lived median): played_bed 178 vs std 155, crowded_bench 229 vs 203, herb_first_colony 126 vs 139; all three
famine boxes 0.4-0.5 like every trained brain. Median lived over 48: H3 71, std 70. **H3 passes Scott's gate**: the
colonies live as well as std_pool's in the game, with the tunnels the Nest lane measured on the bed. The founders are
in `scratch/inrun/genomes/handdig_h3/` (30 raws, each once). H3+eat (the chambered-nests thread's eat seed on H3,
`handdig_h3eat/`) gets the same exam on the VM after env6, beside env6's own exam (x8).

## 2026-10-29 (14:45Z): exam `x8` -- the played-bed filter did not help; x10 -- H3 + eat level too; env7 selects on survival alone

**x8** (144 runs, ref 00d18fe3; std, old, famc7 from x7, same engine), `data-1027/x8/`, `x8an.py`. env6's r23 rank-0
colonies, lived paired over 48 box-seeds (Bonferroni over the 2 filter arms: p < 0.025):

| against | g_dl c3 (+ filter) | g_dlo c8 (filter for donor600) | g_ctl c9 (control) |
|---|---|---|---|
| g_ctl | 14 / 19, p 0.49 | 11 / 18, p 0.26 | -- |
| std_pool | 11 / 22, p 0.08 | 9 / 22, p 0.029 | 13 / 17, p 0.58 |

Alive at the end: g_dlo vs std 6 / 19 (p 0.015); g_dl vs g_ctl 15 / 11. Starved share: g_dlo vs std 26 / 12 more
(p 0.034). Per box, the filter arms are worst in the played bed itself (lived median g_dl 87, g_dlo 128, g_ctl 169,
std 155) and herb_first_colony (87, 102, 129, 139); all trained brains die in the three famine boxes, so the watchdog's
store question (does g_dlo lose where the store is the buffer?) cannot be read there: every arm is at 0.4-0.5.
**Verdict: the filter does not lift the game; neither arm beats its control or std_pool.** With 2-5 donor candidates
a round it chose one or two of a handful the bed's cases had already picked, and its own score fell over the run
(g_dl 648 -> 608, g_dlo 654 -> 511): the ruler was right, the thing it was ruling was drifting down regardless.

**x10**: H3 + eat (`handdig_h3eat/`, the chambered-nests thread's eat seed on H3; container, bb0d7570), against x9's
rows: vs std_pool lived 15 / 14 (p 1), alive 17 / 8 (p 0.11), starved share 13 / 23 (p 0.13); vs H3 lived 16 / 12.
Level with both. The next batch seeds from H3 (the tie rule named before x10, and H3 + eat is already the
chambered-nests thread's runner screen `chambers-sel1`).

**env7** (launched 14:45Z): selection on the ruler alone. colsel `lex+lab` with the bed's four cases weighted 0
(`weights=home8:0,starved:0,nestcuts_late:0,nest:0`), so a colony's whole score is its mean headcount in
played_bed_scrambler + played_bed_understory at 600k, 2 seeds each (`labscens=` the two boxes twice, `labafter=600000`,
new). No labgate, scalegate, labcase or donor600. Bed rounds of 200k still breed the next generation, as always.

| arm | founder |
|---|---|
| L_std | std_pool |
| L_h3 | H3 (`handdig_h3_pool.txt`, the 30 raws once each in file order) |

32 colonies x 16 rounds, P 31 each. Two colsel changes ride along: `labafter=` (LAB_AFTER from a jobs line) and the
lab case runs one job per colony x box instead of one per colony (same seeds, same rounding). Smoke (4 colonies, H3,
labafter=3000): with the new map the round-0 lab column was identical to the old loop's (1370, 1306, 1363, 1427), and
rank followed the lab case alone (c4: lowest home8, rank 0).
**Deciding measure, named before launch:** exam x11, each arm's r15 rank-0 colony, lived paired over the 48
box-seeds against its own founder (std_pool; H3 from x9), Bonferroni over 2 arms, alive and starved share as guards,
plus the played-bed twins' lived by round in colsel.csv (does the selected score climb, unlike env5/env6).

## 2026-10-30 (08:40Z): exam `x11` -- the first trained pools that beat std_pool in the game; x12 -- the selected tunnel line is level

x11 + x12 (144 runs, ref 63621e6b, same engine as x7; std, old from x7, H3 from x9, H3+eat from x10),
`data-1027/x11/lab600k.csv`, `x11an.py`. env7's r15 rank-0 colonies (selection on mean headcount in
played_bed_scrambler + played_bed_understory at 600k alone, 16 rounds) and the chambered-nests thread's sel1 c9
(Scott's lab-ant candidate). Paired over 48 box-seeds, up / down, Bonferroni over env7's 2 arms (p < 0.025):

| | lived | alive at the end | starved share |
|---|---|---|---|
| **L_h3 c12 vs its founder H3** | **25 / 8, p 0.0046** | **20 / 5, p 0.004** | fewer 4 / 33, p 1e-6 |
| L_h3 c12 vs std_pool | 26 / 8, p 0.003 | 23 / 2, p 2e-5 | fewer 8 / 30, p 0.0005 |
| **L_std c23 vs its founder std_pool** | 23 / 14, p 0.19 | **20 / 5, p 0.004** | fewer 12 / 26, p 0.034 |
| sel1 c9 vs std_pool | 16 / 18 | 17 / 8 | 16 / 21 |
| sel1 c9 vs H3+eat as built | 14 / 17 | 9 / 15 | 23 / 15 |

Lived median per plant box (std_pool / hand-built / L_std / L_h3): played_bed 155 / 238 / **304** / 237;
herb_first_colony 139 / 238 / **318** / 261; crowded_bench 203 / 678 / 376 / **552**. In the three famine boxes every
brain still dies (lived 0.3-0.5), so 10-15 of the 48 pairs tie and the sign tests run on the plant boxes.
**Verdict against the measure named at launch:** L_h3 beats its founder on lived (p 0.0046, passes 0.025); L_std
does not on lived (p 0.19) but does on alive at the end (p 0.004). Both beat std_pool on the guards. This is the
first batch since std_pool whose winner beats it in the game, after ten exams of level-or-worse.

Caveats, before calling it: (1) the two selection boxes are played_bed variants, so the played_bed gain is the
nearest kin of what was selected; herb_first_colony and crowded_bench are not played beds and gained as much.
(2) The Nest lane (08:05Z) ran both pools on its garden worlds: dug 100-200 cells against 800-1,000 for the
untrained pools, no tunnels in 15 of 16 runs, L_h3 extinct in 4 of 8 garden worlds, with H3's dig head intact --
selection on the lab boxes alone bred digging out through the core. Scott's rule is dig and forage all game, so
this is a trade until a picture says otherwise. (3) On the training bed L_h3's colonies died out more each round
(12 of 32 alive at r14) because nothing scored the bed. (4) sel1 c9 is level with both std_pool and its own founder:
16 rounds of the standard score kept the tunnel line, it did not improve it.

## 2026-10-30 (09:00Z): env8 -- keep env7's survival score, add a floor on the open nest

**Question:** can a batch keep L_h3's survival gain and get the digging back? env8 starts both arms from L_h3 c12
(`genomes/env7_h3c12_pool.txt`, the ratchet), with env7's job line unchanged (lab case alone, the two played-bed
boxes at 600k, 2 seeds each, bed cases weighted 0).

| arm | change from env7 |
|---|---|
| N_ctl | none (16 more rounds of env7: does more of the same keep climbing or keep losing the nest?) |
| N_gate | `nestgate=50` (new): after the lab ranking picks the better half, a donor must also have an open nest (`nest_open`) at or above the donors' median on its 200k training bed |

Why a size floor and not `scalegate=`: the watchdog's env6 audit found nest / need pays a colony for storing less;
the Nest lane (10-30 08:31Z) asked for the gate on size, `nest_open`, the shape measure that repeats world to world.
Smoke (4 colonies, 2 rounds, 20k, labafter=3000, from L_h3 c12): round 1's gate read c2=7 c1=7 open cells,
equal to `nestcount.count` on the same run folders, and kept both (a tie at the median); on distinct values the same
arithmetic keeps the upper half (3/40/12/25 -> c2, c4). At 20k every nest is 4-9 cells, so the smoke shows the
plumbing, not the pressure.

**Deciding measure, named before launch:** exam x13, each arm's r15 rank-0 colony, against L_h3 c12 and std_pool
(rows reused from x11, same engine unless a merge moves it, then rerun all three). Two conditions, both needed for N_gate to win:
(1) lived paired over the 48 exam box-seeds not worse than L_h3 c12 (a sign test with N_gate behind at p < 0.05 fails it);
(2) `nest_open` at 200k in the Nest lane's garden worlds 61-68 bigger than L_h3 c12's in at least 6 of 8
(one-sided sign test p 0.145 alone, so also report the medians), with the chambers judge's `tunnels` and pictures beside it, not scored.
N_ctl is judged on (1) against L_h3 c12 only, to say whether more rounds alone still pay.

**Correction and baseline (Nest lane, 10-30 09:20Z).** The x11 caveat's "100-200 cells" was nest.csv's `dug`; the
gate's own measure, `nest_open` at 200k in garden worlds 61-68 (`scratch/nest/poolrounds.sh OUT SEED 1 200000 POOLDIR
nest_r=0`, main 678a4fb4), is starker: std_pool 421 659 608 417 215 668 534 615; H3 573 558 462 399 668 269 536 758;
L_std c23 21 48 11 24 68 36 37 53; L_h3 c12 23 15 11 21 12 11 15 4. So condition (2) against c12 is a low bar. The
pass bar stays as named; x13 also reports N_gate against H3 world by world (the level wanted back), unscored.

**Watchdog audit of env7 (10-30 09:33Z, page https://claude.ai/artifact/5PhfJen9ESuBszGLjPxBVr).** Not gamed: in x11's
plant boxes both pools bring home 2-3x std_pool's food (`trip_deliveries`) and starve fewer per hatchling (45-70% vs
75-79%). But no nest: at ref 63621e6b (x11 rows reproduced exactly) open nest averaged over 100k-600k, seed 1, was L_h3
2 (played_bed) / 4 (crowded_bench), L_std 21 / 193, std_pool 216 / 181, hand-built 87 (played_bed). Training drove it:
on the bed, nest r0 -> r15 828 -> 154 (L_std) and 836 -> 68 (L_h3). Watchdog's advice: not L_h3 as the lab ant;
env8's gate is the right shape; x13 reports `nest_open` beside lived.

## 2026-10-31 (00:30Z): env8 done -- the nest gate did not grow the training-bed nest more than the control; exam x13 queued

env8 (ac9d6b64; lost at r3 to a preemption 11:05Z, resumed as env8b-1030, done 00:09Z). Median over 32 colonies by
round, training-bed `nest` (cells) and lab case (summed tenths):

| round | N_ctl nest / lab | N_gate nest / lab |
|---|---|---|
| 0 | 76 / 8,466 | 76 / 8,466 |
| 4 | 107 / 10,159 | 96 / 7,539 |
| 8 | 167 / 9,955 | 139 / 9,994 |
| 15 | 188 / 10,565 | 129 / 9,979 |

Both arms' bed nests grew from L_h3 c12's 76, the ungated control more; the gate read 4-100 open cells per donor in
rounds 13-15 and kept the upper half each time, so it fired, but it did not steer. Colonies alive at r15: 20 / 32 (ctl),
28 / 32 (gate). Rank-0 at r15: N_ctl c12 (lab 14,441, nest 203), N_gate c10 (lab 13,726, nest 128), pools
`genomes/env8_ctlc12_pool.txt`, `env8_gatec10_pool.txt`. The bed is not the verdict: x13 is.

**x13** (`jobs/x13.txt`, 240 exam runs on main after #990 garden, #993, #994 pip food): N_ctl c12, N_gate c10, L_h3 c12,
H3 and std_pool, 6 boxes x 8 seeds at 600k, all rerun on the one build (the engine moved since x11), plus the plants
lane's rungs 5-6 stage-0 run as a `sh` job. Deciding measure as named at env8's launch; garden `nest_open` from the
Nest lane on worlds 61-68.

**env8 garden nest (Nest lane, 10-31 00:39Z, worlds 61-68, 200k, main a4a15fe0; page https://claude.ai/artifact/Jnzf4Raro16z7wddNsQFq5#env8).**
`nest_open` median (range): N_gate c10 40 (13-85), N_ctl c12 30 (8-69), L_h3 c12 14 (4-23), H3 547 (269-758), std_pool 571
(215-668). Condition (2) passes: N_gate > c12 in 8 / 8 (p 0.008), but so does N_ctl (8 / 8), and N_gate vs N_ctl is 4 / 4 --
the nest came back a little from more rounds, not from the gate. Tunnels 1 / 8 each env8 arm, 0 / 8 c12, 8 / 8 founders;
lines extinct N_gate 1 / 8, N_ctl 2 / 8, c12 4 / 8. Reading: a median gate over donors that all dig ~1/15 of the founders
cannot pull the line back; a next try needs an absolute floor near the founders' level or `nest_open` as a scored case.

## 2026-10-31 (01:50Z): exam `x13` -- env8's gated winner beats std_pool and passes its named measure, but so does the control

x13 (240 runs, ref f245c3a9, after #990 garden / #993 / #994; every brain rerun on it), `data-1027/x13/lab600k.csv`,
`x13an.py`. Paired over 48 box-seeds, up / down:

| | lived | alive at the end | starved share (fewer) |
|---|---|---|---|
| **N_gate c10 vs L_h3 c12** | 22 / 11, p 0.08 | 14 / 11 | 24 / 12 |
| N_ctl c12 vs L_h3 c12 | 19 / 16 | 12 / 12 | 13 / 24 |
| N_gate vs N_ctl | 16 / 16 | 12 / 13 | 27 / 11 |
| **N_gate vs std_pool** | **27 / 11, p 0.014** | **24 / 1** | 32 / 5 |
| N_ctl vs std_pool | 25 / 16, p 0.21 | 24 / 0 | 29 / 9 |
| L_h3 c12 vs std_pool | 26 / 9, p 0.006 | 23 / 1 | 30 / 7 |
| L_h3 c12 vs H3 | 25 / 9, p 0.009 | 23 / 2 | 29 / 7 |

Lived median per plant box (std / H3 / L_h3 / N_ctl / N_gate): played_bed 210 / 186 / 340 / 372 / 370; herb_first_colony
151 / 211 / 346 / 479 / 385; crowded_bench 259 / 246 / 575 / 554 / 643. Famine boxes: all five die (lived 0.3-0.5).
**x11 replicates on the new build**: L_h3 c12 vs std 26 / 9 (x11: 26 / 8), so garden + pip food did not move it.
**Verdict against the measure named at env8's launch:** N_gate passes both conditions -- (1) lived not behind L_h3 c12
(22 / 11 ahead), (2) garden `nest_open` above c12 in 8 / 8. But N_ctl passes both too, and N_gate vs N_ctl is level on
lived and on the garden nest (4 / 4): the gain is more rounds, not the gate. N_gate's nest (median 40 open cells) is
still a fifteenth of the founders' (547-571). Best lab-ant candidate so far on survival: N_gate c10 (vs std_pool lived
p 0.014, passes Bonferroni 0.025 over 2 arms; alive 24 / 1); the nest trade-off is unchanged and is Scott's call.

## 2026-10-31 (02:05Z): env9 -- restart from the diggers, survival score, an absolute floor on the open nest

env7's selection (lab case alone) took the founders' training-bed open nest from ~490 to under 100 in 15 rounds, and
env8's median gate could not pull it back. env9 restarts from the two founders that dig (H3, std_pool) with env7's job
line plus `nestfloor=300` (new): after the lab ranking picks the better half, a donor needs a training-bed `nest_open`
of at least 300 (the founders' round-0 median 488-490, p25 379-389); if fewer than 2 clear it, the 2 largest nests
breed. Arms F_h3 (H3), F_std (std_pool), 32 colonies x 16 rounds, on main after #990/#993/#994.

**Deciding measure, named before launch:** exam x14, each arm's r15 rank-0 colony, with N_gate c10 and std_pool rerun
on the same build. A winner needs both: (1) garden `nest_open` (Nest lane, worlds 61-68, 200k) median >= 250 and above
N_gate c10 in >= 6 of 8 worlds; (2) exam lived ahead of std_pool, sign test p < 0.025 (Bonferroni over 2 arms). Lived
against N_gate c10 is reported, not scored: what the floor costs in survival is the trade Scott decides.
Smoke (H3, 4 colonies x 2 rounds at 20k, nestfloor=8): read 23-30 open cells and kept both donors each round; the
fallback branch, on values 3/40/12/25 under a floor of 300, keeps the two largest (c2, c4); 350/40/320/25 keeps c1, c3.

## 2026-10-31 (02:55Z): the lay brake in the lab -- Scott's card, measured in x14

Scott (chambered-nests thread card, 10-31 02:44Z) picked "Test it in the lab (Teacher measures)": paired lab exams with
and without the training bed's brake on laying (`labsurvive laycap=48:8`, PR #992), shipped on in the lab game if not
measured harmful. exam.py gains `LAB_ARGS=` in a job's ENV (';'-separated extra labsurvive args).

x14 will carry it: std_pool (today's lab ant) and N_gate c10 (the best survival candidate), each with and without
`laycap=48:8`, 6 boxes x 8 seeds at 600k, on the same build as env9's arms.
**Harm, named before launch:** for either brain, brake vs no brake with fewer colonies alive at the end (sign test
p < 0.05, two-sided, no correction -- the stricter reading for a harm test), or any box where the unbraked colony is
alive at the end in >= 6 of 8 seeds and the braked one in <= 2. Lived (mean headcount) is reported but not a harm test:
a brake on laying lowers the headcount by design. Starved share and deliveries reported. Pictures: played_bed seeds 1-2,
std_pool with and without, `nestrows=88` (the full 80-row bed).
