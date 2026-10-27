# Training jobs on GitHub Actions

Two kinds of file, two workflows:

- **`<name>.job`**: one run of the arena trainer (`examples/train`), by `.github/workflows/train.yml`. Below.
- **`<batch>.batch`**: colony-selection (`colsel.py`), in-run (`rounds.sh`) and emergence (`emerge.py`) job lines,
  the ones the Google VM runs, by `.github/workflows/batch.yml`. See [Batches](#batches) at the end.

Push a `<name>.job` file in this folder and `.github/workflows/train.yml` runs
it on a free hosted Linux runner (public repo: 4 vCPU, 6 h per job, 20 jobs at
once across the whole repo). **Push job files to a `train/<anything>` branch**,
not `claude/**`: a push to `claude/**` also starts the full CI matrix on the same pool.

## A job file

One line of `train` arguments, `#` lines are comments:

```
# fetch rung, seed 2
brain=mind challenge=suite arenas=fetch,trail gens=100 pop=100 layouts=8 seed=2 start=training/starts/mind-s2.txt
```

- Every `.job` added or changed in the **head commit** of a push to `train/**` (or `claude/**`) becomes one run. One job per file; the name is the file name.
- `every=` must be 1 (the default): hold-out and `curves.png` every generation from gen 0 is the owner's rule; other values are refused.
- `ref=<branch or commit>` is the workflow's, not train's: it builds the trainer from that ref instead of the pushed commit (so a lane trains on its own branch's code while the job file and `start=` genomes ride on the pushed commit).
- `threads=` defaults to the runner's core count; `out=` is set by the workflow.
- A `start=` file must be **in the repo** (the `/mnt/project-files` mount is not on the
  runner). Absolute paths, `..`, `out=`, and non `key=value` words are refused by
  `scripts/train_plan.py` (`--selftest` shows each refusal).
- The run uses the commit the push is on, so the job file is the record of what ran.

## Results

- `train-results/<name>` branch: a single force-pushed snapshot every 10 minutes and at
  the end (log, `STATUS.txt` with commit, args, CPU and times, csvs, `curves.png`,
  champions, every picture). Read it with
  `git fetch origin train-results/<name> && git checkout FETCH_HEAD -- .` in a scratch dir.
- The run's **artifact** `train-<name>` has the full folder (every picture), kept 14 days.
- These branches are data, not source: never merge them into `main`.

## Limits worth knowing

- No run survives the 6 h limit, but a run can be carried on: push the same job file again with `resume=1` and a higher `gens=` (the total, not the extra) and nothing else changed. **Pin `ref=` to a commit sha** for a run you will resume: a resume built from code other than the earlier leg's (its `built from:` commit, compared on `src/`, `examples/`, `assets/` and the build files) is refused before anything is published, since a pay change that keeps the arena line would otherwise resume silently. The workflow unpacks `train-results/<name>` as the out folder and the trainer goes on from its `state.txt` exactly as if it had never stopped (`Settings::resume`; a different arena, start, hold-out or parts is refused, other settings only warn). It may be pushed while the first leg is still running: `concurrency` queues it behind that leg. Only one leg can wait: a third push of the name cancels the waiting one. A job pushed again **without** `resume=1` starts afresh and replaces the results branch. A hosted runner fits about 24 generations of the step-4 dig suite (soil, fetch, trail; pop 100, 16 layouts) into one 6 h leg. 100 generations of the
  fetch+trail suite took ~3.6 h at 4 threads on the runner pool.
- The 20-job pool is shared with PR CI, and `max-parallel: 10` is per workflow run, not for the repo: two pushes can hold all 20 slots. If CI starts queueing, lower it or wait.
- One run per job name at a time (`concurrency`): the same name pushed from two branches queues instead of overwriting the other's results branch.
- `STATUS.txt` ends with `finished:` and `exit code:`; if the exit code line says `none`, the run was cancelled, timed out or lost its runner. No `finished` line means it is still running.
- The hosted "4-core" runner is 2 physical cores: threads=2 is 1.98x faster than 1, threads=4 only 1.18x faster than 2 (measured on a 6-generation probe; scores are identical at every thread count). The runner CPU also varies by run (an EPYC 9V45 did a generation in ~17 s where an EPYC 7763 took ~44 s on the same kind of job).
- Past about a dozen generations per hour you want more cores than this: see `brain-training/cloud-compute/cost-research-2026-10-18.md` for Modal timings.

## Batches

A `.batch` file runs the VM's job lines (`scratch/cloud/inrun_cloud.py`) on hosted runners instead, so a lane can
screen a colony-selection idea without the one Google VM. The format, and every refusal, is in `scripts/batch.py`
(`python3 scripts/batch.py --selftest`):

```
# elite2's e13 arm, screened at 32 colonies x 16 rounds
ref=0123456789abcdef0123456789abcdef01234567
e13s colsel 31 lex+spoil+crater+shape+scale+lab 32 16 200000 assets/minds/std_pool.txt lesson=... elite=1
```

- **A line is a VM jobs line**: `OUT colsel P MODE NCOL ROUNDS FRAMES GENOME args`, `OUT SEED ROUNDS FRAMES SEL
  GENOME args` (rounds.sh), or `OUT emerge ARM N GENS FRAMES FOUNDER opts` (`scratch/pheromones/emerge.py`). Words
  after OUT like `PP_GATE=off` are that line's environment. Or `use=scratch/inrun/jobs/X.txt [only=a,b]` runs lines of a
  VM jobs file as they are. colsel's `P` becomes the runner's cores.
- **Runners.** Each colsel or emerge line gets a runner; rounds.sh lines go four to a runner. One `.batch` per push.
  Push to a `train/<anything>` branch, as for `.job` files.
- **`ref=`** builds the code from that branch or commit, resolved to one sha for every leg. It must be at or after
  the commit that added `batch.yml` (colsel's `DEADLINE` and rounds.sh's `.done` markers are what make legs work).
  Paths in the lines are read in that tree, as on the VM.
- **Legs.** A runner stops at 6 h, so a batch runs in up to 8 legs of ~5 h 20 min (`leg_minutes=N` for shorter
  ones). colsel stops between rounds before the leg ends and the next leg resumes from its `state.json`, which
  gives the same rows as an unbroken run (checked 2026-10-27: 4 colonies x 4 rounds paused after round 1 and resumed on
  a fresh folder, colsel.csv and cg.csv byte-identical to an unbroken run at a different P). emerge resumes from its
  last generation; rounds.sh skips rounds it marked done. A line with no progress in a whole leg is marked failed.
- **Results: the branch `batch-results/<batch>`**, one force-pushed commit after every leg: `STATUS.md` (every
  line: done, paused, failed), each line's `colsel.csv`, `cg.csv`, `gens.csv`, a log tail and, for rounds.sh lines that
  ran in one leg, `armsum.txt`; and `<runner>.tgz`, the whole folder but event logs and the every-4k maps (the
  VM's small copy, and what the next leg carries on from). Read it with
  `git fetch origin batch-results/<batch> && git archive FETCH_HEAD | tar -x -C DEST`.
- **`resume=1`** in a later push of the same batch carries on from that branch (more legs, or lines that failed).
  Without it a push starts afresh and replaces the branch.
- The full folders, event logs included, are artifacts `full-batch-<batch>-leg<k>-<runner>`, kept 14 days.
- **Compute.** A hosted runner is 4 vCPUs on 2 physical cores. Screening size is 32 colonies x 16 rounds
  (`Reports/how-we-test.md` §7); a 64 x 48 confirmation belongs on the VM.
