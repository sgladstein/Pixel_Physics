---
name: keeper-seat-demo
description: Keeper (gardener) seat demo 2026-10-25: demo=keeper, carry is a real graded verb on trained ants, recall head = labelled fake, meal copy neutral; PRs #907 + follow-up
metadata:
  type: project
  modified: 2026-10-25T12:48:27.061Z
---

Thread "Build the keeper seat demo" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bB6MX7UvF4jg8EL1hHuUi6hs), branch claude/project-thread-20o5zg.

- PR #907 merged 041c9da: creature::recall (recall head, PIXEL_PHYSICS_RECALL_GAIN, off = byte-identical), creature::set_down / treat / is_home_for, examples/keeperlook.rs. Scout seat uses recall::plant_with / knows.
- PR #914 merged 1372548 (demo): src/lab/demo/keeper.rs, assets/lab_scenarios/demo_keeper.ron, examples/keeperdemo.rs, Reports/keeper-seat-demo-2026-10-25.md. Artifact: https://claude.ai/artifact/617nmHWDrw4ZKVrgPqLEsn
- Findings (12 paired seeds, pool #906): carrying ants to far food with listening OFF raises station visits +52 (10 carried) / +105 (~17), colony 120 -> 146 -> 168 ants; scripted listening (gain 6) adds +12..+89; meal copy neutral (copy 0 = 0.5). Untaught colonies find a station 280 cols out by ~11.5k frames.
- Traps: set_down refuses stacked ants (use creature::can_carry; a scripted hand asked 10, lifted 6 -> false "carry does nothing"); a lab box's first non-empty row is the LID (use colony_surface from enclosure ceiling); lab landing kills ~30/52 founders by frame 3k (demo warms up 6k frames).
- Taught ants are laden 67-70% of the time; tilt acts only when empty; Walk is 97-98% of empty decisions, so the trained ant's slow laden walk home is the limit, not the verb choice.

**Why:** a later thread on player verbs, in-life learning or the keeper seat should start from these numbers. **How to apply:** see [[teaching-the-colony-plan]], [[colony-mind-seat]], [[creatures-1996-lessons]].
