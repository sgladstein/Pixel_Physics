---
name: creature-perf-lane
description: Creature performance thread (2026-10-21/22): benchmark, PRs #774-#805, rule-change trials (warmth, egg retry), tools and file ownership
metadata:
  type: project
  modified: 2026-10-22T09:07:39.926Z
---

Thread "Creature performance" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBArGt3ZWisrsCqBTXsz4QZH). Owner: focus on the NEW ant (mind.rs, options.rs). Speed-ups must be bit-identical; rule changes need Scotttt's OK with numbers (owner 2026-10-22: explore every ant-cost-review item, /mnt/project-files/ant-cost-review/ant-cost-review-2026-10-22.md).

- **Scene:** `RAYON_NUM_THREADS=1 needscost seed=1 at=40000 handat=40000 window=600 arms=off,mind mind=/mnt/project-files/perf/genomes/v2-C3.txt`, callgrind `--instr-atstart=no cg=1`.
- **Merged exact PRs:** #774, #785, #788, #792, #798 (WayCells), #801 (egg walk scans chunks, -4.0%), #803 (scorer: all units side by side in lanes, each unit's own order; 8 lanes if <=8 live units; mind -5.8%, in-run -18% per training-speed lane). **#805** (one adjacent_nest a tick via context_at, FxHashMap minds, CellMemo buffer pool, DESCRIBED thread-local; mind -2.9%) merged 2026-10-22 09:25Z as 4f884a7.
- **Mind colony 600 frames:** 9.87G (pre-#785) -> ~5.91G after #805.
- **Rule trials (switches off by default, local commit 5d41fbe5 in worktree /home/user/pp-trial, never for main):** PIXEL_PHYSICS_MIND_WARMTH=off (patch /mnt/project-files/creature-perf/mind-warmth-switch.patch; Teacher lane runs it in a 64-seed cloud batch, `setenv=` on inrun) and PIXEL_PHYSICS_EGG_RETRY=24 (patch egg-retry-switch.patch; new OrganismState::egg_walk_after). In-run bed has egg laying OFF, so egg retry is invisible there. Labbench 24 seeds (labforage, game ant; out dirs scratchpad lb-eggretry, lb-eggretry2): no gated metric moved (births higher 11/24, all p>=0.15). Warmth-off cloud gate (64 seeds): food same, colonies lost 17 vs 9 (p 0.15): recommended leave off. Egg walk is still ~11% of mind frame after #805; the wait would cut ~9%. Scotttt approved shipping the wait ON (13:17Z): PR #810 merged as 5f4497a (brood::EGG_RETRY_FRAMES=24, PIXEL_PHYSICS_EGG_RETRY=off restores). Warmth "head only" = warmth-off trial + a new head input needing retraining, so nothing more to trial.
- **Scorer B3 turned out exact** (across units, not across features); no rule change needed.
- **PR #813 (exact, -2.1% mind):** SpeciesRegistry::creature_shared (Arc memo), landing_after_step_into (one buffer for 8 walk landings), fall check copies body only off-ground, MaterialRegistry::known(Known::Spoil/PackedSoil/Corpse), BilinearSampler::temperature_at. Merged 2026-10-22 15:14Z as 255b13e. Thread resolved.
- **After #810+#813:** mind colony 600 frames 5.86G -> 5.20G. Egg walk (pile_site) 4.1%, under_cover_by (settle) 8%, regroup_by_scent 2.5%.
- **Left (not done, recommended stop):** per-chunk home count (~4% now; home definition is complex). Regroup cadence and the review's "not recommended" items stay off.
- **Tools:** scratchpad verify2.sh (pp-base vs working tree: needscost hashes s1/s2, train soil/scarce, callgrind). In-run hash check: training-speed lane runs it (teacher branch + cherry-picks). labbench.py X X --new-env K=V for one-binary env A/B.
- **Dead end:** memoising under_cover per cell (no gain).

**Why:** a later thread should not redo these or re-ask ownership.
**How to apply:** check PR states first; rule-change numbers go to Scotttt before landing; tell the Teacher lane before anything result-changing lands.
