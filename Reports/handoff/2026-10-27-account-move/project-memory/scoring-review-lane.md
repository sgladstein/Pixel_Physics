---
name: scoring-review-lane
description: "In-colony scoring review" Opus thread (2026-10-22, closed ~15:20Z): ant score is a food score; adaptive prices inert; colony-level selection tested (3x food, starvation /3, digging to 0); handed to Teacher lane
metadata:
  type: project
  modified: 2026-10-22T15:11:07.561Z
---

Thread cmsg_01Mei3dwkcdzPxa5Jmuqi2bBAjUJa5aCELHGG1cHuiEGV2, asked by Scotttt 2026-10-22 14:40Z. Doc: https://claude.ai/artifact/S2h242Q6ebWYKjywSEBsUt . Hand-off + scripts + data: /mnt/project-files/scoring-review/ (for-teacher-lane-2026-10-22.md, colsel.py, cg.py, picks.py, pick-log-vs-ee38ae5b.patch).

- Measured (12 colonies, 1 standard hA1 round, 1,302 births): what separates parents is food 87%, home meals 10%, digging 3%.
  Dig price 2.9->2.1 (does not floor), clear price pinned at cap 3 from 75k, tank food price ~1.0: all adaptive prices inert.
  z-rank parents agree with plain score on 27% (271/1,022).
- Owner asked 14:51Z about weighting a job up when the colony is bad at it. Answer given: don't build adaptive ant-level
  weights (weight can't buy a job ants barely differ in); put need in senses and at colony level.
- Colony selection prototype (12 colonies x 6 rounds, rank-sum of home food, late cuts, starved, nest score; bottom half
  refounded from top half), common garden 12 fixed worlds: home 12,324 vs plain 4,131, starved 1.7 vs 5.5, all 12 alive vs 10,
  but cuts 0 in 12/12 (plain 132). Cause: nest score is negative for any crater-digger and ~+1 for no digging, so it
  rewards not digging. Next (Teacher lane's): digging as a gate or lexicase case at colony level, 64 colonies.
- Free births (3 tanks) are only ~4% of food flow: low priority.

**Why:** colony selection is the strongest lever measured so far; the next session must not reuse a summed colony score.
**How to apply:** see [[inrun-selection]], [[dig-loss]], [[colony-training-review-lane]], [[teacher-lane]].
