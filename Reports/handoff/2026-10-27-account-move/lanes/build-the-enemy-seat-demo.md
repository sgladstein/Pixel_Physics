# Lane: Build the enemy seat demo (`demo=enemy`, play the centipede raider)

Written 2026-10-27 for the master hand-off. All code and reports are on main; nothing is parked.

## What landed (all merged to main)
- #903 (0ea4e4e): steering for mind bodies (`sim::steer::take_forced` / `choose`).
- #909 (6033776): the demo itself (`src/lab/demo/enemy.rs`, `assets/lab_scenarios/demo_enemy.ron`, autopilot `examples/enemyhunt.rs`).
- #917 (7ffd841): birth mutation in the demo (`fathers::Store::wild_all`, seasons card). The default is a tenth of the training rate, because the full rate drove 3 of 6 lab colonies extinct with no raider.
- #920 (92a8b65): the owner's playtest fix. The player's raider bites with a full gut (`Steered::bites_when_full`; `fullbite=0` restores the old rule). The centipede's sated rule had been the whole balance: a stalker's kills went from 26 to 345 over 6 seeds.
- #922 (ff80fb6): hit and run.
  - Shift sprints: `Steered::sprint`, `SPRINT_PACE` 2.5, a wind meter of 240 frames that takes 600 to recover.
  - Centipedes regrow their tail: `CreatureDef::regrows`, `creature::regrow_tail`, turned off by `PIXEL_PHYSICS_REGROW=off`. It is an engine rule, so wild centipedes in other boxes regrow too.
  - The HUD counts cells lost, not bites.
- Everything is written up in `Reports/teaching-the-colony-2026-10-25/enemy-seat-demo.md`: the build, the 36-run table, mutation, the playtest fix, and hit and run. That folder also has the review, plan, plan review and plan v2. The index line is in `Reports/README.md`.

## Pending owner decision
- Owner's message (2026-10-25 19:06Z): the forced spit was annoying (fixed), and the combat has no strategy ("walk toward the pile, spam bite, die near the nest").
- The decision card "Which combat mechanic should the enemy demo get first?" is still unanswered. Its options:
  1. Hit and run. **Recommended, and built** (#922).
  2. Venom: a bite numbs an ant for a few seconds, and venom refills slowly.
  3. Brood heist: the goal becomes carrying eggs and larvae out of the nest, with the queen as the boss; tunnels are one ant wide, so fights inside are one at a time.
  4. All three, in that order.
- Recommendation: the owner plays the hit-and-run build first, then venom next.
- The owner has not yet played the hand-built-vs-trained comparison either.

## Open to-dos
- An owner playtest of hit and run is needed. Nothing else is in progress.
- Re-measure before quoting any number. Main got a new standard pool (PR #924, 8301720) after every number in the report was taken.
- Known limits of the autopilot (`enemyhunt`):
  - It never kills anything at the door.
  - Regrowth fired only 3 times in 24 runs: a raider hurt at the door hasn't eaten, and a stalker rarely loses a cell.
  - Trap: a raider resting about 40 columns from the door sits on the foragers' road to the fruit and gets bitten there. That is why the autopilot now rests only with no ant within 10 cells.

## Files outside the repo
- The owner's picture page, "The Raider Demo" (https://claude.ai/artifact/Pesqv6DRSuAkWf5vWayg5V), will not move to the new account.
  - Its videos and cards were rendered from `enemyhunt gif=... card=...`. The commands and every number are in the report, so they can be re-rendered.
  - Copies were in this session's scratchpad (`raider/`, `hitrun/`). Those are ephemeral, not in the shared folder. They are pictures only and need not be kept.
- The raw A/B outputs (the RESULT lines) are summarized in the report tables. They can be regenerated with the commands in the report.
- Status board: `/mnt/project-files/seat-demos/status/enemy.md`, superseded by this note.

## Branches / jobs
- Branch `claude/project-thread-1ekeut` is fully merged (head 6066fa2, which is in main). No unmerged work.
- No jobs are running on the Google Cloud VM or anywhere else.

## Lessons worth carrying
- A seat's player body can be exempted from *decision* rules (the sated rule), but not from body physics. Regrowth was made a species rule, not a player perk.
- Count bites taken from the `gnawed` meter. The hand-built ant's gnaw writes no `ENCOUNTER_BITE` row.
- Score loads home with `trip_deliveries`.
- The brain link price default of 0.25 (e7763514) doubled the trained lab colony at frame 26k. Numbers taken before it are stale.
