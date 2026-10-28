# PF4 on far food (2026-10-28)

Report: `Reports/brain-training/evolvable-pheromones-2026-10-25.md` §9.14. Batch: `.github/train-jobs/pf4w.batch`
(runner results branch `batch-results/pf4w`, full folders as one tgz per arm there).

| File | What |
|---|---|
| `free/`, `deaf/` | each arm's `gens.csv` (column `heaphome` = deliveries whose trip began at the heap), `listen.csv`, and the g39 genomes |
| `garden.json`, `garden.log` | the pre-registered judge (`garden.py ... SCORE=heaphome PATCHES=20000:100:250 DEAFALSO=A,B`, `PP_BED_W=448`) |
| `h2h.py`, `h2h.json` | each arm's best (free c00 hearing and deafened, deaf c12) on worlds 5,000,500-515, per world |
| `gif2.py` | the page's animation (world 5000502, laden ants marked) |
