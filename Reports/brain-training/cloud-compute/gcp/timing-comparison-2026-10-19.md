# Google Cloud timing comparison, 2026-10-19 (project pixel-physics-512114, us-central1 spot)
Same job on every machine: brain=mind challenge=trail clones=1 start=blank gens=6 pop=100 layouts=8 sigma=1.0 holdout=12 seed=1,
built from ed4815b3. Every run ended with identical scores (hold-out champion 0.80, start -0.03, blinded -0.06), so the
numbers below are pure speed. Per-run files are in the timing-* folders next to this file.

| machine | CPU | threads | build s | 6 generations (s) | per-generation seconds | thread-seconds |
|---|---|---|---|---|---|---|
| c4d-highcpu-16 | AMD EPYC 9B45 | 16 | 99 | 118 | 16,19,24,8,12,17 | 1,888 |
| c4d-highcpu-32 | AMD EPYC 9B45 | 32 | 93 | 72 | 8,10,13,4,6,9 | 2,304 |
| c4d-highcpu-64 | AMD EPYC 9B45 | 64 | 91 | 48 | 4,5,7,2,3,5 | 3,072 |
| c4-highcpu-32 | Intel Xeon 8581C | 32 | 99 | 80 | 9,11,14,5,7,10 | 2,560 |

- Doubling cores gives 1.6x (16->32) then 1.5x (32->64): more machine, less work per core-second.
- Intel c4 is 11% slower than AMD c4d at the same 32 threads.
- The ~95 s build is a fixed cost that dominates a 6-generation test; for long runs (tens of generations) it vanishes.
- NOT RUN: c4d-highcpu-32 with threads=16. The VM could not be created: insert failed with SERVICE_ACCOUNT_ACCESS_DENIED
  ("The user does not have access to service account train-writer@pixel-physics-512114.iam.gserviceaccount.com. User:
  train-launcher@..."). The launcher (updated 15:14) attaches train-writer to every VM, and train-launcher lacks permission to
  attach it (needs Service Account User on train-writer). Not changed by me. Also: cmd_start printed "started" although the
  insert operation had failed, because the error shows up on the operation only after the wait.
