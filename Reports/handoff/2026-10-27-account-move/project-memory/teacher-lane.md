---
name: teacher-lane
description: Teacher lane (copy-then-select for the ant brain), thread started 2026-10-21 13:49Z: state, tools setup, and findings that changed the plan
metadata:
  type: project
  modified: 2026-10-21T14:30:07.413Z
---

Took over from the outside teacher session (session_0155kwr6ZPrC8bXE2JopsdJ4). Handoff: Reports/handoff/teacher-lane-handoff-2026-10-21.md (PR #759, branch ccr-439ee31b-pn36yq until merged). Branch for this lane: claude/project-thread-8y5ak0.

- **Owns the one ant (owner, 2026-10-22 05:34Z):** the Teacher lane owns the single ant brain. Foraging food loop, Dig and nest scoring, the maze thread and playtesting deliver lessons, teachers, score rules and findings to stack onto the Teacher lane's CURRENT founder (now hA1, scratch/inrun/genomes/hA1.raw.txt on the compute branch, since 2026-10-22 10:55Z; see [[inrun-batches-day2]]); they may run only small test trainings, never long divergent ants.
- **Overnight go (owner, 2026-10-22 05:41Z):** the nest-zone rule may become the default score if it shrinks the crater without clearly cutting food home; long selections allowed, as many rounds as wanted; owner 05:55Z: all 64 cores (use `--type c4d-highcpu-64`). Founder is c1 since 05:38Z (64x6: home food 2,319 vs h1 1,431/colony-round, p=0.001; starves more than h1 with births off).
- **Digging is core (owner 13:15Z 2026-10-22, relayed by dig lane):** ants must keep digging a large multi-chamber nest AND keep foraging all game; this is the lane's core job. Selection erodes digging (late-round ants re-bail spoil, new rooms only cut early); round length is NOT the lever (refuted). Dig lane writing room-credit patch (pay cutter of cells that end up in a chamber/niche); crater-refusal copy into founder not started. Files: /mnt/project-files/nest-score/digloss/.
- **PR #761** (spoil roll on by default + DecisionOutcome::Packed trace fix) handed to the merge desk thread. Lanes never run the full suite; the merge desk merges (owner rule, 2026-10-21).
- **Walk home (owner, 2026-10-21):** one brain for every ant (no digger/forager castes; ideally ants self-select roles from environmental signals, a future concern). Plan: leave the empty walk home to selection now; teach it later in a world with both food and a dig, conditioned on a sensed signal, built as the colony tier's first arena.
- **Scratch tools rebased onto main** at /home/user/tp (branch scratch/teacher-probes, never for main): Soil needs `step_near: 0.0`, `train::mutate` takes a `keep` slice now. Reproduces s171 9.47 and copy v2 10.71 on reliab's 96 worlds exactly. Added a 15th Q field to the copy log: cavity-bottom reading per Dig option (reach 2,3,4,6).
- **Findings (2026-10-21, report /mnt/project-files/teacher-lane/dig-on-copy-v2-2026-10-21.md):** copying "dig at the foot" into copy v2 collapses (5.2-7.5 vs 10.7) unless the ant has a body-level "bottom of my hole is in reach" sense the job stage reads directly (scratch: PP_BOTTOM=3 PP_BOTTOM_CTX=1 on borrowed c::CROWD); with it, walls x100 copy = 11.14, +0.32 +- 0.43 (n.s.). Cause: depth is squash(rows/16), row 7 vs 8 differ 0.03, and the job stage sees options only via one saturated best score. Teacher acting on v2: walk home +6.0, dig +3.5 (floor-limited), down +1.2, all three +11.0, so walk home is the biggest piece left. My earlier "6.5 wasted foot visits" count was an artefact (short steps near the foot).
- **Saved for later threads:** scratch patch against main 7ff76eda, analysis scripts and genomes (v2c, v2-C3) in /mnt/project-files/teacher-lane/.

**Why:** a later thread should not redo the sense probe or the tool rebase.
**How to apply:** don't redo the dig-copy arms; the open question is the empty surface trip (walk home). Judge any genome with reliab's 96 worlds, then the trainer's fetch floor (worse side). See [[ant-brain-research-lane]].

**In-run selection** moved to [[inrun-selection]]. The full log of rulings and runs up to 21:00Z is /mnt/project-files/teacher-lane/teacher-lane-memory-full-2026-10-21.md.
