---
name: maze-lane
description: Pheromone trail maze thread (2026-10-22): maze arena, who follows a laid trail, trainability, and the walls-block-the-spread diffusion finding
metadata:
  type: project
  modified: 2026-10-22T15:15:00.000Z
---

Thread cmsg_01Mei3dwkcdzPxa5Jmuqi2bBW3jdokQaExBzjV5E2Mdhh1, branch claude/project-thread-wijq9s. Owner asked (2026-10-22 04:04Z): can an ant follow a pheromone trail through a maze, build training into the test, may test/propose trail diffusion changes.

**Built:** `challenge=maze` in the train rig (src/train/challenges/maze.rs; forks=, ramp=, spread=, blend=, walls=through|sink|reflect), `examples/mazefilm.rs` (stacked GIF per genome + brain-free "trail as laid" readability line). Scratch in /tmp/claude-0/maze (genomes h1/s171/v2 from compute branch scratch/inrun/genomes/).

**Maze results (early, h1/s171 era):** follower 0.69, h1 0.01; flat trail (ramp=0) every brain fails; through-rock spread degrades the maze trail, rock-blocks-spread keeps it. The mind does read per-option scent diffs (options.rs SCENT_GAIN 32).
**Delivered to Teacher lane (2026-10-22 ~06:00Z), founder c1:** c1 maze 0.02, trail 0.47. Lesson = graft in c1's empty unit 15 (B_diff/B_diff2 +16, out +20): maze 0.77, trail 0.92. File /mnt/project-files/maze/lessons/c1-trail-graft-u15.txt.
**Walls switch:** PIXEL_PHYSICS_TRAIL_WALLS=1 (off by default; stone/soil block trail spread). Off = bit-identical to main. Switch DROPPED from PR #804 (card's recommended option, 17:30Z); kept in commit 3501f682 + dead-ends. Colony (labbench played_bed 12 seeds 120k, 2026-10-22 14:10Z): walls on mildly worse (intake 2.21M->2.02M 5/7, net food home lower 10/12, underground time 29->37%); walls + A_RHO 0.003 clearly worse (intake -34% 2/10, ant-frames -32% 1/11). Decision card posted: drop switch (rec.) / land off / hold.
**Open-air maze (owner ask):** challenge=openmaze, lattice of bars in air with GAP=2 at nodes (a full lattice = sealed rooms in side view, ants trapped). Worse side 8 test pairs: follower 0.32, c1 0.02, c1+graft 0.18, blind 0.003. laden=1 starts ants carrying fruit. Also PR #804.
**Colony verdicts (Teacher lane 2026-10-22):** plain c1 graft = clear no (49 colonies died vs 9, ants walk trails instead of foraging). Gated graft (unit 15 gated on c1's mem1 laden flag; file lessons/c1-trail-graft-laden-gated.txt, "c1gl"): food home 1,914 vs 2,373 per colony-round (lower 41/64 seeds, p=0.03), lost 13 vs 9: not adopted. Trail-following lessons so far all cost foraging. One free unit cannot be neutral in both states (squash offset). Option feature f::LADEN/HUNGRY describe the ant AT THE TARGET cell, not self: never gate on them.

**Why trail following hurts (traced 2026-10-22):** B is a crowd scent (peaks at nest); climb-B lessons keep ants home. Full record: Reports/brain-training/trail-following/ and [[trail-teacher]].

**Landed:** PR #804 MERGED 17:47Z (f873f9c): openmaze, laden starts, Reports/brain-training/trail-following/ (report + genomes + patch + scripts). See [[trail-teacher]].

**Owner rule 2026-10-22 05:34Z:** the Teacher lane owns the one ant; this lane delivers the maze arena, trail findings and lessons to it, keeps maze training short, no separate ant.

**Why:** don't rebuild the arena or re-run the sweep.
**How to apply:** pheromone.rs is held by the Training speed thread; asked them 2026-10-22 before editing. Any diffusion change goes to the owner with evidence first (one physics for every world). See [[teacher-lane]], [[inrun-selection]].
