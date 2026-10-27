---
name: batch-design-rules
description: How to size and design colony-selection batches (training compute audit 2026-10-24, owner asked that the launching lane follow it)
metadata:
  type: feedback
  modified: 2026-10-24T15:00:24.528Z
---

Source: the training compute audit thread (2026-10-24), which analysed all 52 batches. Page: https://claude.ai/artifact/FXPApab9z3BFE36NJ6Wvwn. Data: /mnt/project-files/training-efficiency/. The owner wanted these rules to reach the Teacher lane.

1. The common garden is 30% of the CPU at CG_EVERY=2, but decisions only read 1-2 gardens. Use CG_EVERY=8, or at least 4; it saves about 22%.
2. 7% of colony-rounds were byte-identical reruns. Treat a control as a cache keyed by build + founder pool + args + seeds, and reuse it or resume its shared prefix. Don't rerun it.
3. Controls ARE needed. Judging an idea against its own first garden got the call wrong on 9 of 18 measures. One shared control per ratchet turn can serve several ideas.
4. 32 colonies keep 99% of large-effect calls and 85% of medium ones. Test 32 vs 64 once before making it the standard.
5. Rounds: rule changes show at the first garden; learned behaviour settles by r23-31; erosion needs 48. Screen at 32 colonies x 16 rounds, then run finalists at 64 x 48.
6. Name the deciding measure before launching. Five measures at p<.05 gives about 1 in 5 false wins. Count extinct colonies as 0 in garden medians.

**Why:** the owner asked that these become the launching lane's practice (2026-10-24 14:59Z). **How to apply:** set CG_EVERY and the batch size in every new jobs line, and write the deciding measure into the commit and memory before launch. See [[far-food-1024]], [[ratchet-founder]].

- **In-lab evolution (enemy seat, PR #917, 2026-10-25):** training's child mutation rate (kpm 0.03, ksigma 1.0, kevents 10) on every lab egg = 3/6 colonies extinct by 150k vs 0/6 off (the lab culls no bad mind before it lays). A tenth (pm 0.003, 1 event) stayed within the off arm's spread. Use the low rate if in-lab evolution is ever turned on.
- **Emit rows (pheromone lane, PR #919 §9, 2026-10-25):** a zeroed emit row opened to standard mutation never rebuilds a state-dependent signal (0/2, 50 gens); seeded N(0,0.05) rebuilt a laden-only trail in ~40 gens, hearing colonies beat deaf 4/4. Keep the alarm emit row frozen on the ant line; if an emit row is opened, start it dense and run a deaf arm.
