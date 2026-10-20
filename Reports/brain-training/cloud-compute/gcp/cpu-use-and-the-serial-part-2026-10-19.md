# Why the cores are not fully busy (Google timing runs, 2026-10-19)

Source: the four timing runs in `timing-comparison-2026-10-19.md` (same 6-generation trail job; per-generation seconds are
the trainer's own `secs` column, which times only the population scoring, `score_all`). Code read: `src/train/mod.rs`
`train()` (main at bc5c6f511).

| machine | wall for 6 gens | sum of per-gen secs | wall minus per-gen = "the rest" |
|---|---|---|---|
| c4d-highcpu-16 | 118 s | 96 s | 22 s |
| c4d-highcpu-32 | 72 s | 50 s | 22 s |
| c4d-highcpu-64 | 48 s | 26 s | 22 s |
| c4-highcpu-32 (Intel) | 80 s | 56 s | 24 s |

**The rest is 22 s whatever the machine**, so it does not shrink with cores: at 64 vCPU it is 46% of the run. It is the
per-generation hold-out (plus the calibration before generation 0, which this table cannot separate).

Why (from the code): after each generation `train()` loops over `controls(...)` = champion, start, blinded, and for each calls
`score_clones`, which runs the 12 hold-out layouts as 12 jobs. So (1) at most 12 of 32/64 vCPUs can be busy, and (2) the three
controls run one after another, so the time is three single-arena lengths per generation (~3.7 s here) however many cores there are.
The population scoring itself (pop 100 x 8 layouts = 800 jobs, shared by all threads) is not the problem.

Fixes for the brain lane (results unchanged, only faster; hold-out numbers stay identical):
1. **Do not re-score `start` every generation.** Its genome and the hold-out layouts never change, so its hold-out score is the same
   number every generation: compute once, reuse. Removes one of the three sequential batches (a third of the hold-out time).
2. **Run champion and blinded (and later the cached start if wanted) as one batch of jobs** (2 x 12 = 24 jobs) so the batch takes
   one arena length, not two. Needs the controls' jobs flattened into one `std::thread::scope` pool, like `score_all`.
3. (Bigger, optional) run the hold-out of generation g in the same pool as the population scoring of generation g+1.
Estimate for this job: hold-out 22 s -> ~7 s, i.e. 6 generations on 64 vCPU 48 s -> ~33 s, on 32 vCPU 72 s -> ~57 s.

CPU load is now recorded by the launcher (every 2 s, in every snapshot as `cpu.csv`; `fetch` prints a summary), so the
real utilisation of the next long run will be measured, not inferred.
