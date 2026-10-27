# Training speed: lane hand-off note (written 2026-10-27 ~18:55Z, main 09e338c)

Thread "Training speed" (one of the four performance lanes, started 2026-10-21). Facts only.

## What this lane landed (all on main)

- **#775** (merged 10-22): scent-trail pass. Exact inline rounding plus skipping quiet rows. The fetch arena went 2.88 s → 1.37 s and the soil arena 4.07 s → 2.46 s (8 worlds, 1 thread). The in-run colony is 19% faster. Results are bit-identical: NOISE rows, the lab_cost world and field hash, and in-run births/stock/sel all match.
- **#793** (merged 10-22): `target-cpu=x86-64-v3` build flag in `gcp_train_job.py`, plus a note to resume with the same flags. A further 14-19% faster, bit-identical.
- The teacher lane carried the same flag and `RAYON_NUM_THREADS=1` into `scratch/cloud/inrun_cloud.py`. Both are on main (lines 69 and 84).
- I checked the creatures lane's #803 on the in-run colony: identical results, 18% faster.

## Branches: one line each, land or why not

- `claude/project-thread-087uht` (this lane's own branch, head 921fbfdc): **delete.** It is fully contained in main. There are 0 commits beyond main.
- `claude/project-thread-8y5ak0` (compute branch): **land via #948, Teacher lane's PR, already with the merge desk.** It holds 8 commits beyond main and #948 contains exactly those 8 (7286595c → f1536023). Nothing older is missing: `git rev-list origin/main..branch` lists only these 8. Do not push to it from here.
- `claude/project-thread-8y5ak0-teacher-probes` (5 commits, 10-21 to 10-22, Teacher lane's): **don't land.** Every commit is labelled "scratch ... never for main" by its author. It holds the pre-move probe harness. The in-run harness that replaced it is on main (`examples/inrun.rs`). Delete it once the Teacher lane agrees nothing in it is still referenced.
- `claude/project-thread-8y5ak0-playtest` (1 commit, 10-24, "far64 pool as the lab's trained ant, far sense 100:100"): **don't land.** It replaces `assets/minds/std_pool.txt` and changes one line of `options.rs`. That would change the lab's ant, and the owner holds "trained by default in lab" until a pool survives the far-food box (see memory `far64-lab-playtest`). It was a playtest build only. Delete it once the playtest is no longer needed.

## Open to-dos

- None that block anything. The Teacher lane offered a 48-core before/after timing for v3 + #775 + RAYON=1 on its cloud launcher, and it never arrived. It is only a confirmation: all three changes are on main and bit-identical.

## Unsaved files

- None of value. My local check branches (`perfpick`, `pick803`, `pr772`, `pr803`, `stack`) were cherry-picks of other lanes' commits for timing, and all of that is on main (#772, #787, #803). The bench scripts in `/tmp/tr` were throwaway; the shared benchmark recipe is in memory `training-speed-lane`.
