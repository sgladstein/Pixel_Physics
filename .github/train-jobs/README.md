# Training jobs on GitHub Actions

Push a `<name>.job` file in this folder and `.github/workflows/train.yml` runs
it on a free hosted Linux runner (public repo: 4 vCPU, 6 h per job, 20 jobs at
once across the whole repo, at most 10 of them training).

## A job file

One line of `train` arguments, `#` lines are comments:

```
# fetch rung, seed 2
brain=mind challenge=suite arenas=fetch,trail gens=100 pop=100 layouts=8 seed=2 start=training/starts/mind-s2.txt
```

- Every `.job` added or changed in the **head commit** of a push to `claude/**` or
  `train/**` becomes one run. One job per file; the name is the file name.
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

- No run survives the 6 h limit; there is no resume yet. 100 generations of the
  fetch+trail suite took ~3.6 h at 4 threads on the runner pool.
- The 20-job pool is shared with PR CI. If CI starts queueing, lower `max-parallel`.
- The first three jobs here are a thread-scaling timing probe, not training.
