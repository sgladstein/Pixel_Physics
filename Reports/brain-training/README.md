# Brain-training notes (snapshot from the project's shared folder)

These files are **verbatim copies** of working notes from the project's shared
folder (`brain-training/` there), committed 2026-10-19 so they live in the repo
and not only on the mount. The subfolders mirror the shared folder's paths.
The shared-folder copies are the live ones and may move on; a later change
should re-copy rather than edit here by hand, unless the note has moved into
the repo for good.

Links inside these notes to other `brain-training/` files (probes, logs,
reviews, pictures) point at the shared folder and are not in the repo.

| File | What it is | Shared-folder mtime (UTC) |
|---|---|---|
| [design/general-brain-design-rev3.md](design/general-brain-design-rev3.md) | **The design of record for the stage 2 brain** (rev 3): one general option-scoring brain for the ant, and the ladder trail → forage (scarce food) → dig → brood | 2026-10-17 18:57 |
| [stage1/README.txt](stage1/README.txt) | Stage 1 results: an evolved target scorer (dig cell, spoil site) on the old brain; rung table and training-run setup. The old small brain has since been dropped | 2026-10-17 08:55 |
| [ladder/trail-rung-controls.md](ladder/trail-rung-controls.md) | Trail rung: rest/blank/clone floors and the signed-progress score that puts blind walkers below 0 | 2026-10-17 16:25 |
| [ladder/scarce-forage-arena.md](ladder/scarce-forage-arena.md) | A scarce-food forage arena that ranks brains the way the lab colony does | 2026-10-17 17:55 |
| [ladder/dig-rung-design.md](ladder/dig-rung-design.md) | Dig rung design, rev 2.9 (step 4b adopted 2026-10-19 23:27): goal, the 12 baby steps, per-pellet scoring, controls | 2026-10-20 00:13 |
| [ladder/README.md](ladder/README.md) | The ladder research lane's index and numbered headlines (links into its probes point at the shared folder) | 2026-10-19 22:37 |
| [ladder/cheats-seen.md](ladder/cheats-seen.md) | How brains have gamed each rung's score so far, and the grader fix for each | 2026-10-17 16:27 |
| [ladder/pictures/s111-where-it-cuts-2026-10-19.png](ladder/pictures/s111-where-it-cuts-2026-10-19.png) | Where seed 111's gen-226 digger cuts: top row, shaft walls, shaft foot (the case for step 4b) | 2026-10-19 22:36 |
| [stage2/dig/README.md](stage2/dig/README.md) | Dig rung results, steps 1-4: runs, controls, scores, what each round overturned | 2026-10-19 |
| [stage2/dig/s111-gen126-vs-start.gif](stage2/dig/s111-gen126-vs-start.gif), [s111-gens0-250-curves.png](stage2/dig/s111-gens0-250-curves.png) | Seed 111 against its start in motion, and its training curves over all three legs | 2026-10-19 22:19 |
| [stage2/dig/runs/](stage2/dig/runs/) | Seed 111's Google legs (gens 70-150, 150-250) as bundles: `state.txt` (exact resume), csvs, logs, hold-best | 2026-10-19 22:19 |
| [cloud-compute/gcp/](cloud-compute/gcp/) | The Google Cloud launcher (`gcp_train_job.py`; the key comes from the environment, never a file) and its timing notes | 2026-10-19 |
| [tools/](tools/) | The GIF maker (`mkgif.py`) and the scratch frame-dump patch it reads (`gif-frames.patch`, never for `main`) | 2026-10-19 |
| [cloud-compute/cost-research-2026-10-18.md](cloud-compute/cost-research-2026-10-18.md) | What external compute for training costs (GitHub Actions, Modal, AWS, GCP), what was measured, and the recommendation | 2026-10-19 01:44 |
