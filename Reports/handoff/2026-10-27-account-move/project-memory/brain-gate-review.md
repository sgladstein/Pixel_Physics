---
name: brain-gate-review
description: Brain-design review 2026-10-26: additive scorer units can't be off, so lessons leak into fed ants; clamp gate (layout 6) + reach-aware memory (layout 7) built; copy-judging rules
metadata:
  type: project
  modified: 2026-10-26T18:09:05.508Z
---

Brain-design review thread (2026-10-26, owner asked 16:10Z), reviewing the Teacher lane's PR #932 report. Report: `Reports/brain-training/research/brain-design-review-2026-10-26.md` (+ data folder beside it); page https://claude.ai/artifact/WNBXM8rgXW7M5iovFkXoK5.

**Finding:** a scorer unit adds its saturated output to every option even when it should be "off", so any lesson copied into an ungated unit also moves fed ants (agreement 0.88-0.91 only at hold KL 0.67-0.83; held still it learns nothing, 0.30 from 0.28). A per-unit gate `clamp(1 + A_u·[ctx, mem, 1], 0, 2)` multiplying the unit's feature sum is exactly off at gate 0: hand-laid start + mutation search reached 0.87 at hold 0.04. World check (4 far-moving-food worlds, teacher off): gated unit starved 107 vs home-low instinct 177 vs colony 13 1,107; same weights without the gate killed every founder by frame 8,000. Blind search from zero found half (0.52; 691 starved).
**Corrections:** the PR #932 "plateau" was the fit's step size (lr 0.5 reaches 0.91); its home-low comparison was circular (input non-zero on 98% of labelled vs 0.02% of other decisions); saturation is bred (sharp gene 6.73, past cap 4) but not the cause. `1 + tanh` gate is illegal (no transcendentals); `1 + squash` needs y=9 to shut to 1/10.
**Origin:** rev 3 never designed a multiplicative path; XdG-style gating was named 10-21 but never designed.

**CORRECTION 18:45Z (Teacher lane 16 worlds):** gated come-home lost 6/16 colonies and every far-lab-box founder, starving AT HOME: the teacher's labels assumed a nest store (PP_CARRY_STORE=1) but no input says whether there is food at home, so the gate learned 'low -> home' unconditionally. Rule: a gate can only switch on what the ant can sense. Blind search from zero found the gate lesson on 1 of 3 seeds (0.52/0.30/0.30). Teacher lane proposes a per-ant 'food at home' context input (owner asked); if yes it goes into the same layout-6 bump.
**Owner 18:23Z chose: build the gate as layout 6, on by default.** Built: PR #934 merged b384f43 (2026-10-26 21:16Z). GENOME_LEN 3357 (pool member 3845), gate block at 2797 (mind::gate_row(u), 35 slots), old files gain it at zero, bit-identical guard hash 0x318b65b15e53a2b8; PP_GATE=off freezes gate slots and reproduces old mutation draws. Owner 19:22Z: doesn't want a specific food-at-home input; asks whether this is a general memory gap (memory never sees option features). Owner 22:20Z said yes to memory/gates reading per-verb best: layout 7, PR #935 merged d8387ed (23:36Z). reach[v]=0.25+0.75*(best+1)/2 if a target exists (REACH_FLOOR, so fed ants scoring a full larder "no" still see it), 0 if none; Eat=crop only, larder is PickUp; mind::reach_mem_row(m) reads this decision's reach, reach_gate_row(u) the LAST decision's (MindState::reach; this one is circular). GENOME_LEN 3609 (member 4097). PP_REACH=off = layout-6 draws. homefit/copyreplay can't judge reach rows (state rebuilt per row).

**Plan:** 1) gate layout 6 DONE; 2) come-home in gated unit FAILED without a food-at-home sense, fixed by layout 7 latch; 3) selection with gates open; 4) retire PP_HOMELOW / PP_SOILWAY when matched; 5) dig/nest.

**Rules (also in how-we-test.md §4):**
- Report every teacher copy as (P(label), hold KL); compare arms at matched hold.
- Measure an input's overlap with teacher labels before calling it a sense.
- A "fresh" unit is zeroed in context, feature AND verb rows (+ output).
- Test a clamp gate by mutation search or hand-laid start, not gradient from zero.

Results: [[brain-gate-results]]. Related: [[home-lesson-fit]], [[teacher-lane]], [[homelow-instinct]].
