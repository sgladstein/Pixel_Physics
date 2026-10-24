# Where ant-training time goes, and what made it faster (Training speed thread, 2026-10-21/22)

> **Landed 2026-10-24** by the report sweep (thread *Land unlanded thread reports*), from the thread's notes in
> project memory. **Not re-measured, and pre-dates colony selection's current bed**: re-profile on `main` before
> trusting the split. The code is on `main` (PR #775 pheromone speed-up, PR #793 the v3 build flag in the launcher).

**Where the time goes (2026-10-21).** About 98% of a training run is the world tick; the harness is 1-2%. The cores
were already full (trainer job pool; in-run `xargs -P 4`), and `RAYON_NUM_THREADS` 1 vs 4 differed by under 4%.
Before #775: the fetch and soil arenas spent 57-60% in the pheromone pass and 18-21% in `end_step`, the mind ~5%; the
trail arena spent 45% in the mind; an in-run colony 43% mind, 28% pheromone, 14% `end_step`.

**What shipped, all bit-identical.**

- **PR #775** (pheromone: exact inline rounding, skip quiet rows, a vectorised inner loop): fetch 2.88 -> 1.37 s,
  soil 4.07 -> 2.46 s (8 worlds, 1 thread); in-run -19%.
- **`target-cpu=x86-64-v3`** for training builds only: another -19% fetch, -14% soil, -23% in-run total, identical
  noise rows, `lab_cost` hash and in-run files. In `gcp_train_job.py` (PR #793) and the Teacher lane's
  `inrun_cloud.py`; **resume a run with the same flags it started with**. The shipped game keeps the portable build
  (that is the owner's call).
- Creatures' PR #803 (scorer across units), checked here on the in-run colony: identical, -18%.

**Benchmark.** Brain lane judge, s171-8-g79, start `dig-4b4-s161-g340-hold-best`, `test=8 threads=1 noise=1`,
`RAYON_NUM_THREADS=1`, arenas fetch/trail/soil; the hash is the 8 NOISE rows per arena against
`Reports/brain-training/stage2/dig/8/test96-scores/s171-*`.

**Traps.** In-run `events.csv` and `final/index.csv` row order differs run to run (a std `HashMap` in `inrun.rs`):
compare sorted; `births`/`stock`/`sel.csv` are stable. `perf` works in the container after
`apt-get install linux-tools-6.8.0-31-generic`, run from `/usr/lib/linux-tools-6.8.0-31/perf` directly.
