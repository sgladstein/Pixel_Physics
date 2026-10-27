---
name: enemy-seat-demo
description: Enemy seat demo (play the centipede raider, demo=enemy) 2026-10-25 — PR #909, what it measured, traps for later seat/predator work
metadata:
  type: project
  modified: 2026-10-25T11:22:08.482Z
---

Thread "Build the enemy seat demo" cmsg_01Mei3dwkcdzPxa5Jmuqi2bB6bSGLzkCV4hUfCkREaLmTi. Steering for mind bodies merged as PR #903 (0ea4e4e). Demo PR #909 merged as 6033776 (2026-10-25 11:43Z). Report: Reports/teaching-the-colony-2026-10-25/enemy-seat-demo.md. Owner page (blind A/B videos): https://claude.ai/artifact/Pesqv6DRSuAkWf5vWayg5V. Owner picked (14:40Z) "Turn on brain mutation in demo" on card cmsg_01Mei3dwkcdzPxa5Jmuqi2bBVvF6dLWsZ68SLopJbzdMrY: built in PR #917, merged 7ffd841 (fathers Store::wild_all; seasons card).

- Play: `--bin lab -- demo=enemy` (`oldant` for the hand-built colony). The raider lands 70 columns from the door at frame 26,000.
- 36 runs with examples/enemyhunt.rs (autopilot through Lab::demo_key) on main 648d705:
  - The door is defended: 12 of 12 raids die on the mound, both brains.
  - Stalking lone foragers: the trained colony kills the raider 4 of 6; the hand-built colony 0 of 6.
  - A landing 30 columns out was killed in under 430 frames on 2 seeds, so the landing moved to 70.
- Trap: the hand-built ant's gnaw (Eat at an animal cell) wounds with NO ENCOUNTER_BITE row. Count bites taken from the `gnawed` meter.
- Trap: the brain link price default 0.25 (e7763514) doubled the trained lab colony (~105 to ~240 ants at frame 26k). Re-measure any lab number taken before it.
- The BATTLE panel auto-opens at the first predator kill and covers a top-left HUD.

**Why:** later seat, predator or defence work starts from this. **How to apply:** reuse enemyhunt's loop for any steered-body harness, and don't re-derive "the door is defended". Related: [[nest-raider-lane]] [[predator-lane]] [[queen-seat-demo]] [[colony-mind-seat]] [[merge-desk-owns-pr-watching]]
- Birth mutation finding (2026-10-25): every egg mutated at the TRAINING page's rate (kpm 0.03, kevents 10) makes lab colonies go extinct, 3 of 6 within 175k frames with no raider, because the lab culls no bad mind before it lays. A tenth of the rate (pm 0.003, 1 event) lives like the unmutated colony, so that is the demo default. Applies to any plan for in-lab evolution.
- Owner playtest 2026-10-25 19:06Z: forced spit annoying; combat has no strategy (walk at pile, spam bite, die near nest). Fix PR #920: Steered::bites_when_full (player bites past the centipede's sated rule; fullbite=0 restores). Skulk autopilot kills 26 -> 345 over 6 seeds: the sated rule WAS the balance. Card cmsg_01Mei3dwkcdzPxa5Jmuqi2bBRZEWBC5gXbpaiEfqXWv6uA: hit and run (recommended; sprint + digest heals/regrows tail) / venom / brood heist / all three.
- Hit and run built (PR #922 merged ff80fb6): Steered::sprint + SPRINT_PACE 2.5 (Shift, wind 240/600 frames), CreatureDef::regrows (centipede; regrow_tail, PIXEL_PHYSICS_REGROW=off). Door deaths 5/6 -> 4/6, skulk 2/6 -> 1/6; regrowth fired 3x in 24 runs. Trap: autopilot resting 40 cols from the door sits on the foragers' road to the fruit. Next if owner wants: venom, brood heist.
- 2026-10-26: main 8301720 has a new std_pool (PR #924). Every enemy-demo number above (kills, deaths, mutation rates) was on the old pool; re-measure before quoting.
- (from MEMORY 10-27) PR #909 6033776, mutation #917 7ffd841, no-spit #920, hit-and-run #922 ff80fb6; venom/brood heist offered; training-rate birth mutation kills lab colonies; door defended 12/12; gnaw bites missing from encounter_log. Owner 10-25 19:06Z: forced spit annoying, combat a pile.
