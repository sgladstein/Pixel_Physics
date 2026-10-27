---
name: inrun-selection
description: Teacher lane's in-run selection pilot (2026-10-21): owner rulings, harness location, results, and the traps found
metadata:
  type: project
  modified: 2026-10-21T20:29:09.397Z
---

Owner's idea: select ants inside one living colony. Design doc https://claude.ai/artifact/DT1eU7tWsJTyaUCLk67pSV.

**Owner rulings (all 2026-10-21):**
- Pilot: no old age, egg laying off, births only from the score. Ants can starve; food plentiful. Founders piled at door.
- Score: need-priced, pays chains (cutter + carrier). Pays food into nest (depth now; later food left in a chamber, not deepest point), meals at home after work. Score is never "settled": VM gate = weights file read at round boundaries, every change logged, quick checks rerun.
- Adaptive weights later: a stagnant metric gains weight, capped at 4x base (luck must not dominate).
- Selection: target turnover, rank-based bar; z<-3 culled at once; nobody below median breeds; graded kids; parents stay, each kid earned on record since last one, lifetime cap 4.
- Don't force brain size; the link cost should set it. Current cost (synapse_fraction 2.2e-6) bills 700 links 0.31/tick vs idle 0.10: too high. Any species-file change: check the Brain lane first.
- Later: mutation-rate sweep for in-run; findings into design doc -> adversarial review -> long VM runs (VM needs owner's go).

**Harness:** scratch worktree /home/user/tp (branch scratch/teacher-probes, never for main), examples/inrun.rs + `pub mod inrun` in challenges.rs + `creature::cull_organism`. Batches: rounds2.sh (3x200k fresh worlds, top 30 carried); judge.sh GENOME is the standard check.

**Traps already paid for:** see [[inrun-selection-traps]].

**Results (fresh-test survivors of 24):** v2 clones 3, start mix 11.5, random 16-18, score 20-22.

**Die-off cause:** foragers NOT culled; ants gorge to ~17 tanks, stop going out, home sharing drains all tanks in lockstep, all starve within ~1k frames. A-D (p12) unreadable: 3-7 of 8 extinct per arm. Owner chose hall refill (`hall=30000 dead_w=0.5`, built; alone a treadmill). Cull calibrated on clones (z sd 0.97, honest); reliability plateaus ~0.3 after 8k (relia.py) 16-seed fp batch: famprior+turn15k food 10,783 vs 7,373 (z30), starved 3,989 vs 4,591, extinct 13 vs 8 of 16 (unclear while die-offs rule); rerun with lesson founders.
**Later same evening:** tank-level food price (`fprice=tank`) and hungry-departure pay (`w_hungry=10`) built; each barely moved die-offs. Hunger probe (`starve_at=20000:1.5`, scratch probe.py): hungry ants leave home LESS (235 vs 303 departures in 6k), no mutant responds to hunger, so nothing to select. Lesson thread "Go out when hungry lesson" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBJyToRD1JeniEn9bKapCoan) builds the habit; genomes to /mnt/project-files/teacher-lane/hungry-lesson/. `progress=1` (food paid half en route, half on arrival, clawed back if eaten): +54% food, -20% starved, extinctions unchanged; now default in judge.sh. Owner asked for food-store tracking: stock.csv every 1k frames (nest provisions 8 -> 0 by 20k, starvation ~12k later; heaps ~130 throughout; corpses counted separately, they once fooled it). Lesson h1 judged (22:25Z): probe starved 43 vs v2 99 of 192, clone survivors 6.2 vs 1.2, in-run healthy 5 vs 4 of 8. Lesson thread: 94% of hungry ants hold a soil pellet at the chamber bottom. Design doc updated with findings, rules as built, die-off chart, pre-VM list (rev ~40). nest.csv every 25k (lab::nestscore from PR #770 copied into scratch, HARVESTER): most colonies negative, best 26. Final-nest sheet: pictures/final-nests-2026-10-21.png (sheet.py). Latest patch + judge.sh: scratch-patches/latest/. Dig thread: "Dig and nest scoring" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bB133PNrbYZfrtNXv6NHyBi1).
**Why:** the next thread should not rediscover the traps or re-ask the rulings.
**How to apply:** start from this note and the doc; see [[teacher-lane]], [[brain-lane]], [[ant-brain-research-lane]].
