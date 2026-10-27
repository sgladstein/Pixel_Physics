---
name: training-environment-review
description: Env review 2026-10-27 (PR #941): trained pool dies past 200k on its own bed, can't use windfall, and harness-only rules each cost it in the game
metadata:
  type: project
  modified: 2026-10-27T15:04:54.775Z
---

Thread "Review the ant training environment" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bB1X3bY6CcHJAV4BDfsHUTpC), owner ask 2026-10-27 13:44Z. Report: Reports/brain-training/teacher-training/training-environment-review-2026-10-27.md (PR #941, merged 44410be). Page: https://claude.ai/artifact/SPnFg8uqM8siivwhdkn6d4

Measured on std_pool, e3 garden args, worlds c*1000+99 (16), sign tests:
- **Past the round:** alive colonies 15/16 at 200k, 10/16 at 400k, 0/16 at 600k on the unchanged bed. The hand-built ant keeps 14/16. A 600k twin equals the 200k run exactly up to 200k.
- **Windfall:** training food=mix has no windfall, which is in 9/36 shipped lab boxes and dropped by trees. In two_larders, swapping windfall and corpse for fruit and provisions: lived 3.6-7.3 -> 37-101 (8/8). This explains the Teacher lane's exam die-off.
- **Harness-only rules, one at a time:** lay_cap off starved 284->1,506 (16/0). The game's door ban: nest 857->713 (0/16). oldage=40000: alive 74->61. grants=1: low 24->13. All together: starved 1,232, all extinct by 400k. The hand-built ant reaches 314 alive under the same rules.
- **Physics:** tick=lab (the game's tick) and light rain moved nothing. Every nest hits the bed's 40-row soil floor (lab 96). plants= rain=R never rained in the harness (byte-identical to rain=0), so the plantwean/broodplant beds were dry.
- New inrun switches, off by default: `oldage=N`, `tick=lab labrain=R`.

**Why:** the owner's lab games run 200k-670k frames, with windfall and the game's rules, so 200k gardens on the harness bed overstate trained pools.
**How to apply:** judge pools with a 600k twin and in a windfall box. Before trusting a garden, list harness-only settings. The Teacher lane decides adoption ([[teacher-lane]]). Related: [[far64-lab-playtest]], [[plant-forage-playtest]], [[report-significance]].
