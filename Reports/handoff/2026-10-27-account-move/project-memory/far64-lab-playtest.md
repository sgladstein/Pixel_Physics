---
name: far64-lab-playtest
description: Owner's 2026-10-24 far64 lab playtest analysis: boom-bust because lab has no lay_cap, landing deaths, 1 dig/ant/10k, mound-sitters; painted-food log bug
metadata:
  type: project
  modified: 2026-10-24T16:39:32.089Z
---

Thread "Analyze far64 lab playtest" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBYLmKNFFNFew8RC7jD2KLaX), 2026-10-24. Owner played PR #865 build (4e80c202), hand-built 1024x512 box, no plants, food painted by hand, 7 colony drops, 225k frames. Report: Reports/brain-training/playtest/far64-lab-2026-10-24/README.md (branch claude/project-thread-3r7600).

- Death is boom-bust: each food drop -> 710/313/224 ants -> crash; 91% starvation, mostly afield; first crash with 2.4 MJ still in box. lays_declined 0 all run.
- **Training's size brake lay_cap=48:8 lives only in examples/inrun.rs, not the game.** Lab colonies are uncapped. Judge pools in the lab on starvation after peak.
- 56% of landed founders starve within ~1,500 frames (lab lands on 1 tank, training on 3).
- Nest: ~1 dig per ant per 10k frames; ~900 open cells total; brood in pocket by door; spoil on mound (22 rows).
- 2-27% of ants underground; 30-60% sit FED on the spoil mound (idle, not hungry).
- Bug: Lab::release did stroke.take() without finish_stroke, so PAINTED FOOD/SOIL/DUG never logged from a real mouse: release() dropped strokes since 2026-08-30 and the line was added 10-08 in finish_stroke, which release skipped. Fixed in PR #867 (merged). Before the fix a food drop = fruit_j jump after PAUSED/RESUMED.
- Owner rule 2026-10-24 17:31Z: "Player actions should all be in the data." PR #869 added logging for alarm, fling, lamps, roster culls, rain, plant pace, fruit rot. Any new box-changing verb must call world.log_player_action.

**Why:** next playtest or Teacher-lane session shouldn't re-derive these. **How to apply:** compare lab runs vs training knowing the cap difference; see [[teacher-lane]], [[playtest-lane]], [[lab-playtest-lane]].
