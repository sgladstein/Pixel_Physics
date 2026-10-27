---
name: colsel-judging
description: How to judge a colony-selection (colsel) run honestly: CG drops extinct colonies, pair against the founder, nest as well as alive, adaptive-ramp signals per bed
metadata:
  type: feedback
  modified: 2026-10-24T15:14:32.616Z
---

Learned by the plant-food lane, 2026-10-24 (report: Reports/brain-training/plant-food/plant-food-weaning-2026-10-24.md §5).

- **colsel's common garden (cg.csv) silently skips colonies that died out in the training round** (no carry). Its medians are survivors only. Count missing colonies as 0 alive. plantwean4's "68-71 alive" was 62-65.5 that way.
- **Pair against the founder in the same CG worlds (seed c*1000+99), with the same CG args and the same cap/dist.** Twice a CG median alone looked like progress and wasn't: plantwean4's selected colonies were worse than untrained std_pool in 10 of 16 pairs.
- **Judge nest and crater as well as alive** (owner: food is not the only goal). Selecting at a resource cliff once cost two thirds of the nest.
- **Reproduce one VM CG row locally before trusting a local control.** The carry plus the CG args gave exact matches every time on main.
- **Adaptive ramps (pilecap, distramp, spiderramp) must step on a signal the bed does not hold fixed.** End-alive is pinned by in-run births. On a breeding bed, starved tracks the resource cut, not skill. Colonies dying out per round (`pilegate=extinct:LOW:HIGH`) separated the caps cleanly.

**Why:** these mistakes produced confident wrong reads (progress that was survivorship, a ramp that bounced for 23 rounds). **How to apply:** every colsel batch judgement, any lane. Related: [[plant-food-lane]], [[teacher-lane]], [[inrun-selection-traps]].
