---
name: teacher-plan-1022
description: Teacher lane's re-plan of 2026-10-22 16:00Z (owner asked for a whole to-do review): phases, bar, decisions, doc link
metadata:
  type: project
  modified: 2026-10-22T15:57:35.299Z
---

Owner (15:48Z 2026-10-22) asked to stop jumping between tasks and make a detailed plan. Plan doc: https://claude.ai/artifact/PWTEN1zxjrbSsFvqyv2dmz (Claude Docs id b645c8ef-2847-4244-93af-b5e6ae9c6b39).

- **Bar (common garden, per 24 ants, vs old ant):** food >= old ant (735), starved <= 1, late new-ground cuts >= half old ant's and not falling over rounds, >= 2 rooms kept all game, training extinction < 10%.
- **Phases:** 0 pick founder from switches-1022 + nob-1022 (+ maze trail-teacher copy); 1 add rooms + 2nd-half cuts to cg.sh, move common gardens off local 4 cores (VM or GitHub Actions train.yml); 2 colony selection with dig GATE + lexicase over colonies, 64 colonies, 10 rounds, arms gate+lexicase / gate+rank-sum / plain; 3 inherited nests (round N starts in round N-1's world; needs world save/load); 4 starvation only if still >1/24 (births from store; roamers need owner); 5 main-merge of compute branch, harness to main off by default, selected POOL into lab newant (PR #799), GIF vs old ant.
- Decisions put to owner (going ahead unless told otherwise): the bar; colony selection as main method; inherited nests; two 32-core VMs + GitHub runners for judges.
- Compute: spot quota is 64 CPUs project-wide; for in-run batches 1x64 = 2x32 in throughput (no serial part), 2x32 gives parallel batches.
- Snapshot PR #829 (docs only) holds genomes/tools/design doc; backup branch claude/project-thread-8y5ak0-teacher-probes.

**Why:** a later session should follow this order instead of re-deriving it. **How to apply:** check the doc for owner comments before starting a phase; see [[teacher-lane]], [[inrun-batches-day2]].

**Progress 16:40Z:** phase 1 done (scratchpad cgtools/cgx.py reads nest.csv: old ant late cuts 69, dug 387, 1 chamber + 2 niches, pit 0; selected hA1 36 / 34 / 0+0 / pit 74). Phase 2 running: colsel-1022 (scratch/inrun/colsel.py at 0b9edcd7, 64 colonies x 10 rounds, plain/gate/lex, GATE=20 late cuts or a room, CG every 2 rounds into cg.csv). Phase 3 built: inrun writes nestmap.txt, `inherit=PATH`; colsel MODE+inherit; batch colselinh-1022 queued after (228d1c5c). Waiter: scratchpad csvm.sh. Replay updated (HFEskdEuRrzvSsSEYU7LeK): hA1veto_3 vs old ant world 3099; 4 of 261 selected colonies beat the old ant on food, starved and cuts.
- Owner 16:26Z: later phases in order = harder/moving food -> births + egg laying paid from the nest store (extra food must pay for something; selected ants gorge: 770 tanks vs old 246) -> integrated plant world with finite food. Judge now also reads nest store + mean tank (cgx.py).
- Maze thread 16:27Z: trail teacher (empty ants climb laden-only trail) FAILED acting and copied (tA1 died 19/24; B peak runs away). hA1-noB sel-off 32 seeds: home 374 vs 271 (28/4) but died 4.4 vs 2.8. Mine nob-1022: sel-off food 367 vs 260 (56/64), with selection n.s.; CG digging pending.
- **switches-1022 CG (32 seeds, vs hA1r200 paired):** lesson=c1:0.1 keeps digging (cuts 242 vs 130, 15/5 p=.041; late 58 vs 36; dug 100 vs 34; starved 3.8 vs 4.9) at lower median food (1,246 vs 2,692, paired n.s.); lesson3 = food arm (3,605, 16/6 p=.05, cuts 121); cross (cuts 5) and grow_band (cuts 28) kill digging; lex, mutmid, r4 nothing clear. In-run: only grow raised food. Founder stays hA1; next colsel batch should add lesson=c1:0.1.
- **colsel-1022 RESULT (16:30Z):** CG after r9, medians per 24: plain home 4,397 / starved 5.0 / cuts 3 / late 0; gate+rank 7,464 / 3.2 / 0 / 0 (gate loophole: rooms>0 passed 47/64 non-diggers; fixed 260c9c17 = late cuts only); **lex 2,121 / 2.2 / 220 / 44, rooms 1.0, dug 43, pit 56** (vs plain paired: cuts 34/8, starved fewer 36/14, food 28/28). Lexicase over colonies = the method. Pulled to scratchpad/colsel. Next: colselinh-1022 running (inherit arms), colsel2-1022 queued (lex, lex+lesson c1:0.1, lex+lesson+inherit, 16 rounds; waiter csvm2.sh).
