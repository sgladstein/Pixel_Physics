# Brood lane scratch (2026-10-23/24)

Tools and inputs behind `Reports/brood-in-training-2026-10-24.md` and `Reports/brood-plan-2026-10-23.md`. Moved here
from the project's shared folder on 2026-10-27, when the project moved to an account without it.

- `garden.sh` -- one evolved colony on the garden bed (sel=off, 200k frames, breeding at the inrun defaults).
  `CARRYDIR=scratch/inrun/brood/carry` points it at the carries below; `DIST=` moves the food.
- `separation.py` -- how close brood sits to stored food in a run's nest snapshots, against an open-cell control.
- `carry/` -- four late colsel20 carries (Teacher lane, cs20light round 47: c3, c27, c43, c59), the colonies every
  garden number in the report was run on. Their own README gives the environment their genomes need.
- `pilot/` -- the first pilot's scripts as they ran (hcs round 0, 9 arms x 3 seeds): `run.sh` (one arm),
  `batch.sh`/`batch2.sh` (the arms), `summ.py` (the table), `curves.py` (ants and store over the round),
  `sheet.py` (nest contact sheets, crumbs labelled). **They hard-code the session's paths** (`/home/user/brood-wt`,
  a scratchpad under `/tmp/claude-0/...`); edit `cd` and `O=` before reuse. The pilot's raw runs were not kept;
  the numbers are in the plan report.

Results (colsel `cg.csv`/`colsel.csv` per arm) and pictures are in `Reports/brood-in-training-2026-10-24/`.
