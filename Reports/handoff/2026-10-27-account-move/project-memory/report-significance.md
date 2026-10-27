---
name: report-significance
description: Owner wants every quoted difference to carry its variability: paired counts, a sign-test p-value, and a multiple-comparison caveat
metadata:
  type: feedback
  modified: 2026-10-27T02:52:32.319Z
---

Owner (Scotttt, 2026-10-27 02:51Z, Teacher lane): "Do you take into account normal variability in these comparisons? Is 111 significantly different than 180 in a statistical sense?" I had called a 180 -> 111 open-nest drop a trade-off. Paired over 16 worlds it was 13/16, p = 0.02: borderline with 7 measures compared. I had to correct the claim.

**Why:** single medians mislead because outcomes here have wide spread. The owner reads the conclusion, not the table.

**How to apply:**
- Quote every A-vs-B difference as paired counts ("lower in 13 of 16 worlds") plus a two-sided sign-test p.
- When several measures are compared at once, say which ones survive a multiple-comparison correction (Bonferroni: p < 0.05 / k).
- Never call a measure a trade-off or a cost unless it passes that bar.
- Also explain how selection weighs things (epsilon-lexicase, no magnitudes, about 10 cases) when it matters to the argument.

Related: [[batch-design-rules]] rule 6 (name the deciding measure; 5 measures at p<.05 gives ~1 in 5 false wins), [[gate-step2-result]].
