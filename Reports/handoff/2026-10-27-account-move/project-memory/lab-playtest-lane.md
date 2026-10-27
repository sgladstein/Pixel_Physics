---
name: lab-playtest-lane
description: Lab COLONY TRAINING page + ANT BRAIN (new ant h1) switch, PR #799 (2026-10-22): how to run, what matches the harness, known gaps.
metadata:
  type: project
  modified: 2026-10-22T05:27:06.537Z
---

Thread "Lab playtest" (2026-10-22) built two things in the lab game, PR #799, handed to the merge desk.
- ANT BRAIN switch (MENU, or ANTS page): every ant in a lab box gets founder genome h1 (assets/minds/h1.raw.txt). `lab -- newant`. A box birth inherits h1 UNMUTATED: the box plays the brain, it does not evolve it.
- COLONY TRAINING page (MENU, or `lab -- training`): in-run selection ported from the compute branch's examples/inrun.rs (src/lab/training.rs), 4 views (status, settings, best ants, what happened), defaults = ctl.txt + kid_grants 3.
- examples/labtrain.rs runs it headless. With step=harness, births.csv/sel.csv are byte-identical to inrun.rs (seeds 1-3, incl. 3-round handover and random arm). The page itself uses the game tick, so it is NOT bit-identical.
- The door heap fits 24 of 30 founders, in the harness too (told the Teacher lane [[teacher-lane]], [[inrun-selection]]).
- Screenshots: /mnt/project-files/lab-playtest/. Headless lab shots: PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=8 + PIXEL_PHYSICS_LAB_TICKS/CURSOR/CLICK under xvfb + lavapipe.

**Why:** owner asked to playtest the new ant and have a UI for colony evolution training.
**How to apply:** training-page changes go in src/lab/training.rs; re-prove with labtrain step=harness against inrun.rs if the harness changes.
