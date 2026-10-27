---
name: ant-cost-review-lane
description: Fable "Ant cost review" lane (2026-10-22, done): where the new ant's frame goes by rule vs code, the ranked list, the measured negatives, scratch branch
metadata:
  type: project
  modified: 2026-10-22T04:58:08.176Z
---

Thread "Ant cost review" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBAAHhS8ttkmDt5sCeGPfa4s), 2026-10-22 04:18-05:10Z, at Scotttt's request. Report: /mnt/project-files/ant-cost-review/ant-cost-review-2026-10-22.md, and in the repo as PR #822 (docs only, Reports/ant-cost-review-2026-10-22/ + index line, branch claude/project-thread-oo9r0d-docs, with the merge desk 2026-10-22 15:50Z). Scratch branch `claude/project-thread-oo9r0d` (0efe4017): `needscost census=1` counters + `SCRATCH_EGG_RETRY`/`SCRATCH_EGG_AT_NEST`/`SCRATCH_FROM` switches; never for main.

- **Main finding:** the new ant's cost is mostly the rule at its design rate (decide every 6 frames, ~10.7 options x 45 features, v2-C3 reads every feature and verb). Option list repeats 0.04% of decisions -> no cache. Rest 0.4% -> no rest hold. Nothing unread.
- **Egg walk is code, not rule:** 65-89% of ticks are above the egg bar, 70-80% of walks find no home in reach yet fill 121 cells first; 12.7% of frame. Fix: per-chunk home-cell count (bit-identical, ~-9%). Handed to the Creature lane with allocation churn (~6%), adjacent_nest twice a tick, roofed column scan.
- **Rule changes offered to the owner:** egg-walk retry wait 24 frames (-10.3% instr, births moved +48-60% inside 4.5x seed spread; recommended only after the exact fix); warmth head-only (-5%; warmth is 0 on 84-89% of options); dense scorer sum order (-4-6%, not bit-identical); tick cadence named, not recommended. Ruled out by count: egg walk only beside home (27-61% of eggs laid 2-4 steps from home).
- Behavioural note: v2-C3 picks Feed on 34% of decisions, walk 64%.

**Why:** a later session should not re-count these or re-propose the dead levers.
**How to apply:** see the report's sections A (lane) and B (owner); [[creature-perf-lane]], [[teacher-lane]].
