---
name: training-speed-lane
description: Training-speed performance thread (started 2026-10-21 21:47Z): where training time goes, the shared benchmark, PR #775, tooling setup
metadata:
  type: project
  modified: 2026-10-21T23:18:13.123Z
---

Thread cmsg_01Mei3dwkcdzPxa5Jmuqi2bB6AchEzy6PMTn6jEH7uZ5gk, branch claude/project-thread-087uht. One of four perf threads (lab world cmsg_..6YrKEBA1AimzzHSS3rvLcq owns end_step/world.rs, PR #772; creatures cmsg_..ArGt3ZWisrsCqBTXsz4QZH owns mind/options/creature.rs/brood; plants cmsg_..6FQGYAmpZjUVJAvTCKje3M). This thread owns src/sim/pheromone.rs (agreed with creatures).

- **Where training time goes (perf, 2026-10-21):** ~98% world tick, harness 1-2%. Cores already full (trainer job pool; in-run xargs -P 4); RAYON 1 vs 4 within 4%. Before #775: fetch/soil arenas pheromone 57-60%, end_step 18-21%, mind ~5%; trail arena mind 45%; in-run colony mind 43%, pheromone 28%, end_step 14%.
- **Shared TRAINING benchmark:** brain lane judge, s171-8-g79, start dig-4b4-s161-g340-hold-best, test=8 threads=1 noise=1, RAYON_NUM_THREADS=1, fetch/trail/soil; hash = 8 NOISE rows per arena vs Reports/brain-training/stage2/dig/8/test96-scores/s171-*.
- **#775 merged** by 2026-10-22 04:40Z. **PR #793** (v3 flag in gcp_train_job.py + resume note) merged 2026-10-22 04:59Z. Creatures PR #803 (scorer across units) checked by this thread on the in-run colony: identical, -18%. pheromone.rs handed to the maze thread (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBW3jdokQaExBzjV5E2Mdhh1) for a rock-blocks-spread rule (owner's call; told it to keep an open-cell mask, not world reads, and extend the tiled-pass guard).
- **PR #775** (pheromone: exact inline round + quiet-row skip), with merge desk 23:2xZ: fetch 2.88->1.46 s, soil 4.07->2.65, in-run -19%, all bit-identical.
- **PR #775 head ceccaec7** adds a vectorised inner loop: fetch 2876->1368 ms, soil 4066->2464 (8 worlds, 1 thread).
- **target-cpu=x86-64-v3** build: another -19% fetch, -14% soil, -23% in-run total; bit-identical (NOISE rows, lab_cost hash, in-run files). Proposed to brain lane for gcp_train_job.py's cargo build line only (not the shipped game: portability is the owner's call). Brain lane (wrapping up; owner moved training to the teacher lane's in-colony methods, 2026-10-22) asked this thread to open the launcher PR itself after #775 lands: one build line + header note 'resume with the same flags you started with'. Teacher lane applied v3 + #775 + RAYON_NUM_THREADS=1 to its 48-core launcher (scratch/cloud/inrun_cloud.py, branch claude/project-thread-8y5ak0); 48-core before/after pending. Teacher lane now owns the trainer.
- **In-run events.csv / final/index.csv row order differs run to run** (std HashMap RandomState in inrun.rs); compare sorted. births/stock/sel.csv are stable.
- **perf works**: apt-get install linux-tools-6.8.0-31-generic, run /usr/lib/linux-tools-6.8.0-31/perf directly.
- Owner rules: up to 4 Haiku runners total across perf lanes (ask coordinator); don't watch CI after handing a PR to the merge desk.

**Why:** a later thread should not re-profile from scratch or re-negotiate file ownership.
**How to apply:** re-profile on main after #772/#775 land; see [[brain-lane]], [[inrun-selection]].
