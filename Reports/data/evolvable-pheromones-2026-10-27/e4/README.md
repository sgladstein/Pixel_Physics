# E4 on far food: six repeats (2026-10-28)

Report: `Reports/brain-training/evolvable-pheromones-2026-10-25.md` §9.15. Batches: `.github/train-jobs/e4a.batch`
(R=1-3) and `e4b.batch` (R=4-6), pf4w's lines at `SEED=100+R`, ref 597a9943. Full run folders (every generation's
genomes) are one tgz per run on the runner results branches `batch-results/e4a` and `batch-results/e4b`.

| File | What |
|---|---|
| `e4_{free,deaf}_R/` | each run's `gens.csv` (column `heaphome` = deliveries whose trip began at the heap), `listen.csv`, and its garden-best g039 genome |
| `garden_a.json`, `garden_a.log` | the pre-registered judge (`garden.py ... SCORE=heaphome PATCHES=20000:100:250 DEAFALSO=A,B`, `PP_BED_W=448`, a local build of 597a9943) over R=1-3, plus pf4w as a control on the build: it reproduced pf4w's garden exactly (819 / 527, the same criterion medians) |
| `garden_b.json`, `garden_b.log` | the same over R=4-6 |
| `combine.py`, `verdict.txt` | garden.py's verdict rule over all six pairs (per-run rows depend only on the run, so halves combine), plus the second pre-registered question (free higher in 6/6) |

The page's animation is `../pf4w/gif2.py` on repeat 2's bests (free `c04`, deaf `c02`), world 5000502.
