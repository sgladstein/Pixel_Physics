---
name: dig-lesson-oldant
description: Old-ant dig teacher (2026-10-22/23): copies into shipped weights fail; ACT=1 crutch in colsel13; Dig's own feature head dh7 gives nest 13->25 at no food cost, handed to Teacher lane
metadata:
  type: project
  modified: 2026-10-23T04:00:14.912Z
---

Dig-scoring thread. Files: /mnt/project-files/nest-score/dig-lesson/README.md (rounds 1-3), head/ (dh7.head, patch vs compute 05eff7dc, copier_dh.py).
- PP_OLDLAB=1: world clone, old ant's one tick = label. ACT=1 cut calls act; =2 underground program; =3 empty jaws only; =4 cell only when mind digs.
- hcs garden (PP_HOMEVEC=40:30, cg.sh, 32 seeds sel-off): nest 13, crater 115, food 318, starved 1. ACT=1: nest 38.5, food 376 (Teacher lane runs it as colsel13 crutch, patch landed 05eff7dc). ACT=3: nest 60.5, food 307. ACT=4: nest 16 -> which cell is worth ~nothing; gain is WHEN to dig.
- Copies into shipped weights fail (hA1 full: starve; hcs verb-only: nest lost).
- Dig head (PP_DIGFEAT/PP_DIGHEAD=file/PP_DIGHEAD_GATE=1): 8 units over 45 feats + 12 new dig senses; rank (times hcs's weight) + "when" into Dig job, gated to empty-jaw dig states. dh7: nest 25, crater 100, food 305, starved 2. Weights in side file (not evolvable yet).
- Traps: head alone choosing cells cut hanging bites -> ants buried, sank to floor, starved (drop ROOF_DROP labels). Adding head before squash flattens (hcs Dig scores 0.97-0.99). Ungated "when" cost food.
See [[room-shape-measure]], [[teacher-lane]], [[nest-cuts-rule]].
- 04:30Z 10-23: PP_DIGHEAD_EVOLVE=1 (patch head/dighead-evolve-vs-1a98de26.patch): head weights appended to genome (3279), crossed whole, mutated at PP_DIGHEAD_EVOLVE_F=0.25 (full rate hurt: 5/24 adults). Handed to Teacher lane. Reviewer found crater = mind ants skip old ant's roof (top 6 rows) + room body rules; with them on crater 0 (colsel14 testing). If made permanent, head labels need re-recording with rules on (offered).
- 05:35Z 10-23 Round 4 (c5e87f68, PP_BODYRULES=both, its own counter, 32 seeds): hcs nest 171.5 crater 8.5 food 569.5; +dh7 nest 191.5 food 518; +dh8 (labels re-recorded with rules) nest 190.5 food 474. Rules do the work; head adds ~20 cells, costs food. Rec: plain hcs+rules, head at most as evolvable start. Sent to Teacher lane. Disk allowance filled once (raw label logs ~250MB each): delete raw logs after filt.py.
