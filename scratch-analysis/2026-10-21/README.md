# Scratch from 2026-10-21 (brain lane) -- not for main

Saved at the hand-off so nothing lives only in a container. Paths inside the scripts point at the old session's
scratch folder; change them.

- `mutants.rs` (examples/, builds on this branch): children of one genome by `train::mutate` at chosen pm/sigma/prune/grow.
  `mutants-for-main-after-pr752.rs`: the same for current main (mutate takes a keep mask; optional tau keeps |w|>tau).
- `mutsize.py`: the offline mutation-size test over reliab's CSV (children's spread; what one truncation step gains).
- `averaged-child.sh`: the averaged-child test, verbatim.
- `teacher-ceiling-and-reliab.patch`: the hand teacher (`PP_TEACH`) and reliab tweaks used for the 20.5-20.9 ceiling.
- On this branch, uncommitted until now: `PP_NO_ROOFDROP=1` (blank the roof flag; counted at exit), `PP_PLAIN_BED=1`
  (plain box: flat, 40 rows, no lens or step), `examples/bedfeat.rs` (each hold-out layout's bed features, CSV).
- `queue.sh`: the depth/roofed ablation, trail-per-ant at 3-24 ants, and the 12-ant decision trace, as run.
- `sc.py`, `top2genome.py` (state.txt's top genome -> genome file), `lineage_plot.py`, `analyse.py` (rank agreement),
  `judge-sealed-and-fresh-OLD.py` (the pre-test-set judge; use `Reports/brain-training/tools/judge.py`).
- `watch.py` (poll the VMs; exits on a change), `waitci.sh` (wait for every check run on a commit; `gh api`).
