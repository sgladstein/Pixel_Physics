---
name: selection-review-1027
description: Selection review (2026-10-27, PR #939): colony survival ~luck (rank-sum retest 0.05-0.09), nest cases 63-71% of vote, child rounds hurt lab/nest, pool is one lineage; selaudit.py
metadata:
  type: project
  modified: 2026-10-27T14:07:17.058Z
---

Thread "Review ant and colony selection scoring" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bB7LMJpudpMHq2DKMPwgyc2o), asked by Scotttt 2026-10-27 13:43Z. Report Reports/brain-training/research/selection-review-2026-10-27.md (PR #939), pictures https://claude.ai/artifact/S4yqktKEjHwFW8YJgyLyWG, data /mnt/project-files/selection-review/ (elite1/self1/drift1 colsel.csv + elite1 r95 per-ant).

Measured from elite1 (32x96):
- Survival order (10-case rank-sum) retest for same unmutated colony across 2 worlds: 0.05-0.09 = no-mutation control. Children take 46-49% of kept half; kept tenure median 1 round.
- Per-case retest: food 0.3-0.48, nest/scale/lab 0.2-0.43; shape, spoil, herb box ~0; starved 0.21 (std) vs 0.02 (instinct arm). Herb box rank is 0.62 world seed. World explains only 0-0.2 of bed rankings.
- Effective weights: 6 nest cases 63-71% of survival vote; labgate+scalegate leave ~4 donors/round; lexicase barely acts.
- Child vs own donor same round: worse on lab 63-69% of 1,520 pairs (control 54%), nest 57-60%; food better in 2/3 arms. Food vs nest between colonies -0.40.
- In-run breeding: 24 lines -> 2-3 per round; every r95 ant descends from one ant 7-22 rounds back.
- Seed overlaps: garden seed at r99; lab seeds collide with colony c+1 from r100.

Follow-up: colsel `donor600=FRAMES[:Q]` (a long-run survival gate on donors, off by default) is in Teacher lane commit bc8485fb. Rounds run in lockstep, so a 600k run costs about 3x a round's wall time even though it adds only about 37% compute; env1 may use 400k or run it every other round.

**Why:** "no weights" was wrong in practice; selection was judged by outcomes only.
**How to apply:** audit every colsel arm with `python3 scratch/inrun/selaudit.py OUT/` (rule in how-we-test §7). State effective weights when a result depends on them. Ranked fixes handed to Teacher lane: cut child-round damage (kevents=1; mutate once at refounding + sel=random), one case per goal with explicit weights, more donors/families; more worlds only after. Running mean for kept colonies not recommended (toy model). Related: [[mutation-load]], [[colsel-judging]], [[scoring-review-lane]], [[report-significance]].
