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
