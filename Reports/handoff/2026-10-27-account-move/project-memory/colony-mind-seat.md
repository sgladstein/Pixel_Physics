---
name: colony-mind-seat
description: 2026-10-25 colony mind seat demo verdict (no standalone demo on trained ants), PR #905, and the two measurement traps every player-verb test must avoid
metadata:
  type: project
  modified: 2026-10-25T09:44:27.042Z
---

Thread "Build the colony mind seat demo" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBEgF6GcJFQCizK4Vq4jzPFk), one of six seat demos (board /mnt/project-files/seat-demos/BOARD.md). Verdict 10:10Z: **no standalone demo on today's trained ants**. PR #905 merged 8329b37 (2026-10-25 10:12Z): Reports/teaching-the-colony-2026-10-25/colony-mind-verdict.md + colony-mind/ notes; examples/mindprobe.rs (paired verb probe vs unplayed twin), scripts/mindprobe_cmp.py, scripts/mindstake.py; off hooks Minds::closed_stomach, World::scale_pheromone. Owner decision card posted (recommended: fold close-the-stomach into keeper, favour into queen's who-lays).

Measured (16 paired seeds, steady_income): close the stomach works (adult feeds -95%, hunger spread x2, 16/16) but no payoff; louder food trail at the door = noise on trip_deliveries; alarm and warmth move no ant; famine bed kills 14/16 colonies whatever the policy.

**Rules for any player-verb test (why: both bit this seat):**
1. Check claims against the TRAINED ant (mind.rs/options.rs). The lab runs minds by default; a mind ant returns at creature.rs:7998-8022 before act() and the hand-built walk, so forage drive, met stamp, RECRUIT, MEET_WAY, store rules, nurse scent are dead there.
2. Score food home with creature_stats.trip_deliveries, never MindCounts::delivered or plain deliveries (crumbs re-handled in the nest, 2.4-5.8x income). Sanity check: can the control's number exceed what physically entered?

Related: [[teaching-the-colony-plan]], [[land-before-resolve]].
