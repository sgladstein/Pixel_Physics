---
name: colsel-batches-1022
description: Colony-selection batches colsel3-colsel7 of 2026-10-22 evening (kept-nest rule, spoil/crater/shape cases, harder food): results and state
metadata:
  type: project
---

- **17:20Z re-judge (colsel-1022 r9 pools, 32 colonies, CG):** old ant nest 302 / kept cuts 252 / late 46 / spoil_in 0; lex 24 / 17 / 1 / 570; plain 3 / 0 / 0; gate 1 / 0 / 0. Lex beats plain on kept cuts 18/2 paired; 9/32 lex colonies nest >=100. "Lex keeps digging" was mostly crater. colsel2-1022 (raw-cuts rule) deleted; colsel3-1022 (DIG=nest, arms lex / lex+lesson c1:0.1 / +inherit_nest=1, 64x16, jobs colsel3.txt, 8b06da19) started 17:18Z, waiter scratchpad csvm3.sh pulls to scratchpad/colsel3.
- **colsel3-1022 (DIG=nest, 16 rounds) WORKS:** best arm lex+lesson+inherit_nest: CG nest 225 / kept 178 / late 48 / starved 0 / food 1,224 (old 302/252/46/0/716); 59/64 nests >=100. Replay updated (world 5099, colony 5 vs old ant).
- **colsel4 (same arm, 32 rounds):** nest peaks r21 (257) then falls to 147 at r31; spoil_in 437 -> 1,030 (treadmill returns); food 1,582. Its +spoil arm was a no-op: lexicase used range(4). Fixed 22016765 (all cases read; +spoil/+crater/+rooms MODE suffixes).
- Owner 18:25Z: next = stop the crater (top layer near the nest) + get multiple rooms; asked whether to stop paying past a room size (I advised against: punishes digging; reward branching at colony level instead). Dig thread asked (18:30Z) for a GRADED colony-level "on the way to multiple rooms" measure from end maps (/mnt/project-files/teacher-lane/nestmaps-1022/). colsel5 replaced by colsel6-1022 (19:12Z, 529ff42a): +spoil / +spoil+crater / +spoil+crater+shape (roomshape.py from the dig thread: 2nd room or side tunnel scores, main room + crater 0; old ant median 20, selected 3-6). Waiter csvm6.sh -> scratchpad/colsel6.
- **colsel6 (20:05Z):** +spoil WORKS (spoil left 188 -> 18, 63/63; nest holds ~160 to r31) -> standard. +crater and +shape as colony cases FAIL (barely move own number, cost nest; shape median 2 all rounds): no room-making variation to select. Asked dig thread for an old-ant-underground-dig DAgger lesson onto hA1 (behaviour source first). Table: /mnt/project-files/teacher-lane/nestmaps-1022/colsel6.md.
- **colsel7-1022 (20:15Z, b9c7de7d):** harder/moving food on the +spoil arm: dist=45, dist=65, dists=45,25,65 per colony-round (colsel `dists=`). Old-ant baselines at 45/65 running locally (scratchpad cgnd.txt). Waiter csvm7.sh.
- New lanes requested from coordinator 20:00Z (owner offered capacity): A far-from-home sense (+ re-copy come-home & trail rule 2), B land harness on main (merge main, re-thread hooks, bit-identity).
- Old ant CG (32 worlds) by food distance 25/45/65: home 716/400/302, starved 0/0/0, nest 302/613/679, kept late 46/106/96, crater ~0, mean tank 213/32/7 (scratchpad cgnd.txt). Farther food: old ant digs MORE, eats less.

**Why:** next session must not rerun these arms. **How to apply:** +spoil is standard; crater/shape need a behaviour source (dig lesson). See [[nest-cuts-rule]], [[teacher-pictures]], [[teacher-plan-1022]].
- **20:55Z NEST RULE FIX (d3392d97):** old nestcount counted door markers/crumbs above as roof and the door's 3-wide column down craters as nest -> selected "nest" numbers in colsel3-7 are inflated (round-31 maps, fixed: old 286/crater 2; ctl 147/262; +spoil 96/345; +spoil+crater 54/368). Now crater = no ground between datum and cell AND datum gap > 5 (stamp NN..N). Same rule in roomshape.nest_cells and inrun inherit_nest. All batches before colsel8 selected on the faulty rule. Pictures: /mnt/project-files/teacher-lane/nest-vs-crater-*.png.
- **colsel7 (old rule):** food 45/65/moving: CG home 3,089/2,643/1,808 vs old ant 400/302/400 (32/32), starved 2-3; r31 kept nest (fixed rule) 23/6/21 vs 96 at food 25 -> farther food = pits. Picture nests-by-food-distance.png.
Later batches: [[colsel-batches-1022b]].
