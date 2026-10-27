---
name: brain-gate-results
description: First results on brain layouts 6/7 (gates, reach-aware memory): selection uses the gates for foraging; hand-laid home-had-food latch matches the instinct (Teacher lane, 2026-10-27)
metadata:
  type: project
  modified: 2026-10-27T00:21:31.865Z
---

Teacher lane results, 2026-10-27 00:21Z (its scratchpad latch/, gate6/, g6c/). Layout background: [[brain-gate-review]].

**gate6 selection** (std_pool, gates open vs PP_GATE=off, 48 rounds, CG at r47): alive 66 vs 67.5, starved 304 vs 311, food home 5,069 vs 3,184, nest 133 vs 446, crater 35 vs 93. Final open genomes carry ~30 gate links each. Ablation (gate slots zeroed, 16 worlds) drops food home 4,611->3,052 (13/16) and 12,979->2,630 (16/16): selection really put foraging into the gates. But the rank-1 colony 11 does not dig (nest 21, open 1.5), so food-only wins can hide a lost nest.

**Hand-laid "home had food" latch** (layout 7, mklatch_hb.py): memory units 8/11/9 at rate 0; P = AND(at_nest, reach[PickUp]); N = AND(at_nest, no pickup, energy < ~0.37); L = flip-flop (self 10, +-15 from P/N, bias -2 so it starts empty); unit 8's gate +3*L, bias -2.7.
- v1 reset on any at-nest decision with no PickUp in reach -> reset at the door every trip, starved 222. Lesson: a reset must not fire at the door.
- v3, 16 worlds: 0 colonies lost, starved 45.5 vs instinct 43.5, food home equal, open nest 532 vs 392, crater 88 vs 9 (worse). Lab far boxes equal to the instinct.
- Hijacking those memory units without reading them lost 9/16 colonies: colony 13 was using them.

Next: selection from the latch pool (no instinct) vs homelow + instinct, on the VM (latch1, f0212d87).

## Detail moved from MEMORY (10-27)
- Layout 6 gate PR #934 merged b384f43 (PP_GATE=off old draws); layout 7 reach PR #935 merged d8387ed (PP_REACH=off; floor 0.25, owner yes 22:20Z). 4 worlds: starved 107 gate vs 177 home-low vs 1,107 c13. CORRECTION 18:51Z: 16 worlds gated come-home lost 6 colonies, all starved in far lab boxes (no "food at home" sense); blind search found it 1/3. Judge copies as (agreement, hold KL).
