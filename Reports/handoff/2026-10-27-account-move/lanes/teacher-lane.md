# Teacher lane: hand-off (2026-10-27, ~18:50Z)

The Teacher lane owns "the one ant line": colony selection (`scratch/inrun/colsel.py`) over the in-run training harness
(`examples/inrun.rs`), run on the Google Cloud spot VM. Its record of every batch is
`Reports/brain-training/teacher-training/colony-selection-log-2026-10-24.md` (read the last five sections for today).

## State of the work, in one paragraph

**Why selection lost a good habit:** in-run mutation at the default `kpm=0.013` outran colony selection (drift1, PR #937).

**Elitism fixes the drift** (colsel `elite=1`, PR #938): kept colonies don't mutate. But progress still stalls by
round ~50 (elite2).

**The selection review** (PR #939) found three reasons:
- the in-colony ant score works against the colony cases;
- one round's colony rank is mostly noise (0.05-0.09 repeat);
- the pool collapses to one founder colony by round 7-15.

**"Come home when low, if home has food"** has no step-by-step path for mutation (PR #940). The write-gated memory
(layout 8, PR #942, `PP_WRITE=on`, off by default) gives it one, but that is untested under selection.

**The environment review** (PR #941):
- training never offers windfall or corpse;
- judging stops at 200k frames, and every std_pool colony is dead by 600k;
- harness-only rules make training easier than the game.

**This lane's held-out exam:** trained brains die out in 3 of the game's 8 boxes, where the hand-built ant thrives.
Windfall rots before slow trained ants reach it; corpse is never offered to a mind ant at its gut.

## clean1 result (done 19:12Z; logged in PR #955, merged 0cecdde)

No arm starves less than k3 (`elite=1 kpm=0.003`) at round 47. Colony selection alone (kdec) sold the nest for food
(open nest 27 vs 512 held out). No arm changes the game's lab boxes. Every arm collapses to one founder colony.
Recommendation: keep `elite=1 kpm=0.003` as the training default; do not adopt `refound_mut`/`carry=random`. The VM is
deleted; the full job tgzs stay in bucket `runs/clean1/`.

## Open to-dos (in order)

1. clean1 is judged and logged (above, PR #955).
2. **env1:** windfall in training (`food=all`, a new inrun option) and a windfall box inside `labgate`.
   - `donor600=` donor gate: 400k-600k, or every other round. Lockstep rounds make it ~3x wall time.
   - Check repeatability with `scratch/inrun/selaudit.py` on the first 16 rounds before gating on it.
   - Lift the harness-only rules one at a time: lay_cap, door ban `PP_MIND_DOOR_OPEN`, `oldage=`, grants. All at
     once kills every colony.
3. **write1:** the std_pool ratchet with `PP_WRITE=on` vs off (layout 8, PR #942), same seeds, on clean1's best setup.
   Judge on traced far-colony starvation and on whether winners carry live write rows (`describe()` prints "write N").
   This is the test for making the write gate default-on.
4. **Keep diversity:** the ratchet (best colony becomes the next founder) restarts every batch from one colony, and
   std_pool members differ in only ~35 weights. Seed the next pool from several colonies or arms, and report lineage
   (`scratch/inrun/lineage.py`, `selaudit.py`) every batch.
5. **Corpse:** a mind ant at gut about -1 is never offered corpse (`diet_yield` threshold). The hand-built ant's corpse
   "deliveries" are unexplained (maybe separate forage code, or an evolved gut). Decide whether corpse belongs in
   `food=all` before env1.
6. **Stored food is counted in cells, and most stored cells are crumbs** (pheromones lane, PR #944; `how-we-test.md`
   §4). By frame 100k almost every larder cell is a part-eaten crumb, and kJ per cell fell from 0.55 to 0.38 over 150
   generations in one evolved line. In colsel the cell count (`stock.csv` nest_food + door_food) enters only the scale
   case's *need* (`nest_scale`, colsel.py ~line 390). There, more crumbs make a nest look too small, which penalises
   crumbing rather than rewarding it. A crumb does fill a cell, so cells may be the right unit for space. Decide that
   before switching the case to `larder.csv` *_kj, and check every other lane's score that counts stored cells as food.
7. **Can the brain copy the hand-built ant in the game's boxes?** (owner asked for this 2026-10-27 21:01Z, after
   clean1). Give it its own thread in the new account. It tells apart the two remaining explanations for why trained
   brains die in two_larders, scattered_larder and feast_and_famine (4-5 lived vs the hand-built ant's 34-147):
   - if the brain can copy the hand-built ant there and then survives, the brain is capable, and the problem is what
     training offers and selects for (env1, write1);
   - if it can't copy it, the brain's design is the bottleneck.

   **Method:**
   - Label the hand-built ant's verb choices in those boxes (`PP_MIND_LOG` gives each decision's options and features)
     and fit a pool to them with the teacher/copy tools (`homefit`/`copyreplay`, Teacher-lane lessons).
   - Report each copy as (P(label), hold KL) per `how-we-test.md` §4.
   - Then run the copied pool in the same boxes with the exam (`scratch/inrun/exam.py`, `BOXES=two_larders,...`), paired
     against std_pool and the hand-built ant.
   - Try layout 8 (`PP_WRITE=on`) as well as the default. Note the hand-built ant uses separate forage code for corpse,
     so leave corpse boxes out or label them separately.
8. **Owner item 3, a standing practice:** a replay of each batch's winner plus traces of the ants that starved.

## Owner decisions still pending

- **Write the "judge a part by its own job" rules into `Reports/how-we-test.md`?** These were proposed 2026-10-26 19:22Z
  after the owner's worry that short-term food/survival verdicts kill right-but-early steps. The rules:
  - separate "does the part do its job" (its own metric, traced) from "is the colony better today";
  - side effects become new issues, not failures;
  - default-off-but-kept;
  - a training budget before a survival verdict.

  Recommendation: yes. No answer yet.
- **Change inrun's default `kpm`, or make colsel `elite=1` the default?** These affect every lane. After clean1 the
  recommendation is yes, to both: `elite=1` plus `kpm=0.003`.

## Pools and founders

- `assets/minds/std_pool.txt`: the standard pool (nest7 hweanrules colony 13), layout 5/6 era. It is nearly a clone.
- `assets/minds/homelow_pool.txt`: needs `PP_HOMELOW=0.006,PP_HOMELOW_STORE=1` (the "home, when low" instinct), plus
  `PP_LASTFOOD=off`. On held-out worlds it starves 43.5 vs std_pool's 262.
- `scratch/inrun/genomes/latch_pool.txt`: layout 7, with the hand-laid "home had food" latch. Rebuild it with
  `scratch/inrun/latch/mklatch_hb.py`.
- No trained pool beats the hand-built ant in the game's lab boxes. elite1's winners (e3 c32, e13 c7, ehl3 c5 at r95)
  were not saved as pools; they are only in the `runs/elite1-r1` bucket (`small/` and `jobs/`).

## Files and data

- In the repo (PR #948): the log's `data-1027/` (garden and exam tables for drift1, self1, elite1, elite2), `exam.py`,
  `exam_cmp.py`, `lineage.py`, and the job files.
- Commit 9b5d471b on the lane branch (pushed once #948 merges) adds `scratch/cloud/runvm.sh`, `countjobs.py` and
  `scratch/inrun/latch/mklatch_hb.py`.
- `/mnt/project-files/selection-review/`: colsel.csv per arm, per-ant r95 files for elite1, exam CSVs. The useful
  summary tables are now in the repo. The per-ant r95 files (110 MB) are not, and need not be.
- The bucket keeps full job tgzs for every batch under `runs/<name>/`. Batch names today: drift1, self1, elite1,
  elite1-r1, elite2, clean1.

## Branches

`claude/project-thread-8y5ak0` is the compute branch. Everything to bc8485fb plus the log is in **PR #948** (merge desk).
Commit `9b5d471b` (tools) is local to the lane worktree and is pushed after #948 merges. A copy survives the session in
`/mnt/project-files/handoff/lanes/teacher-tools/`: the commit as a patch (`git am` it onto main) and the three files
under `files/`. The master hand-off carried these three files into PR #953, so commit 9b5d471b itself is dropped.

## Working practices not written in the repo

- **Quote differences with paired counts and a sign-test p, and say which survive Bonferroni** (owner, 2026-10-27
  02:51Z). Never call something a trade-off unless it passes. Lexicase has no magnitudes, so explain weighting when it
  matters.
- **Name the deciding measure before the batch.** Run a held-out exam, not training worlds: the old 16-world harness
  (seeds s*1000+7) was a training world.
- **The VM:**
  - Google Cloud has only 64 cores, so one VM at a time.
  - Spot instances get preempted; runvm.sh relaunches with `--resume`.
  - `inrun_cloud` prefixes names with `pixel-train-`.
  - Background waits die at 2 h, so re-attach a watcher.
- **Disk:** pulls of long batches fill the session disk. Pull, keep `cg.csv`/`colsel.csv`, and delete the colony folders.
- **Merge desk:** lanes open a PR, send the merge desk the head SHA, hold pushes until it reports the merge, and never
  watch CI.
