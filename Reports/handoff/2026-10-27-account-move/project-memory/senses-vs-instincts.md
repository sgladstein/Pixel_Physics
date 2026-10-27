---
name: senses-vs-instincts
description: Owner 2026-10-26 15:59Z questioned PP_HOMELOW/PP_SOILWAY as "behaviours built as senses"; I agreed and offered instincts vs plain senses plus a gating brain
metadata:
  type: feedback
---

Owner (Scotttt, 2026-10-26 15:59Z, Teacher lane): "you keep suggesting and implementing new senses that to me don't sound like actual senses but specific behaviors that you want and are building as a sense?"

My answer: yes. PP_HOMELOW (toward home x how hungry for the distance, 0 when fed) and PP_SOILWAY (toward the door only while carrying soil) pre-combine real senses into the answer, so the brain only learns to follow it. SOILWAY reused home-low's weights, so it is close to a hand-written instinct. I did this because the scorer cannot build "A AND B" (its scores are saturated). I withdrew the brain-slots card. I offered two options:
1. Keep them, labelled as instincts.
2. Plain senses (e.g. the way to the door through the tunnels, always on) plus a brain change that lets a unit gate one input by another, so training learns the combination. I recommended this one.

**Why:** the owner's line is learned behaviour, not built-in rules ([[no-oldant-body-rules]]).

**How to apply:**
- Don't propose a new "sense" that is gated to one situation or carries the desired direction. Call such a thing an instinct, and ask first.
- Prefer always-on raw senses plus architecture that can combine them.

See [[nest-dig-job]], [[home-lesson-fit]].

**Follow-up (owner 16:03Z):** "if our brain is bad at combining two senses, we have a fundamental issue in how it's designed." I agreed and gave three reasons:
- The step scorer is additive: context is the same for every option in a decision, so it can only matter through squash saturation.
- Hidden units feed only the job head, never the step scorer.
- Zero weights never come alive, and training is mutation-only on a few dozen colonies.

My proposal: context-gated scorer units, where a gate from context multiplies a unit's feature weights. Test it first: the new brain must learn come-home from plain senses, both by fit and by selection, with no instinct inputs.

I offered to write a design and test plan first. Awaiting the owner's answer.

**16:10Z owner:** write a report, then start a new thread to review the original brain design and the later work, adversarially review the report, and plan.

**Done:**
- Report `Reports/brain-training/research/brain-cannot-combine-senses-2026-10-26.md`, PR #932, head b9f402b0, with the merge desk.
- Measured with scratchpad ns/sat.py: median unit slope 0.023, 75% of sums past slope 0.1, Walk scores past ±0.95 in 76%, hunger slope 0.022 vs fed 0.023.
- Asked the coordinator for the review thread (Opus), with the owner's message attached.
- Correction: in-run neutral_mutate DOES add links (4 events per child, half of them adds), so a given dead link appears in ~1 of 1,250 children. Not "never".
