# Test-world scores, 2026-10-21 (brain lane)

Raw `examples/train.rs ... score=<genome> noise=1` outputs, saved so the next session judges against the same files
rather than re-running them. Commands: `Reports/handoff/brain-lane-handoff.md` section 4. Judge with
`Reports/brain-training/tools/judge.py`, e.g.
`python3 -I Reports/brain-training/tools/judge.py s91-fetch.txt s91-trail.txt s171:s171-soil.txt:s171-fetch.txt:s171-trail.txt copy2:copy2-soil.txt:copy2-fetch.txt:copy2-trail.txt`
(run from this folder).

- Top level, **96 test worlds** (`test=96`, 8 rows, 12,000 frames; fetch `dist=10,30 budget=1200`; trail default):
  `s91` (fetch/trail reference), `s171` (`training/starts/s171-8-g79.txt`), `copy2` (the teacher lane's copy v2),
  `b181` (`training/starts/b181-8-g138.txt`), round 2's four hold-bests `cur191`, `cur192` (=
  `training/starts/r2-cur192-g113.txt`), `small191`, `small192` (`../round2/`). These are the handoff's section-0 table.
- `ablation/`: s171 intact and with its depth links / roofed links zeroed (the zeroed genomes are here), test worlds.
- `crowd/`: trail at 3/6/12/24 ants (`clones=1 ants=N test=48`), and the 1- and 12-ant soil runs whose decision logs
  (15 MB, 157 MB) were not kept; the 12-ant Feed share (13%) came from those logs.
- `mutation/`: the offline mutation test. `mut.csv` (arms A-D, children of s171 by `scratch-analysis/2026-10-21/mutants.rs`
  on branch `scratch/brain-instruments`, scored on reliab's two 48-world sets), `mut2.csv` (arms E-J), `mut3.csv`
  (arms K, L: the research lane's prune-only-small-links arm A'); `picked.txt` the children truncation kept;
  `mean-*.txt` the averaged children, `meanrec.csv` their reliab scores, `test-*.txt` the averaged children and
  parent on 96 test worlds.

  Arm settings (pm / sigma / prune, grow 0.1 throughout; 50 children each unless the CSV says otherwise): A 0.03/0.3/0.3
  (**current**), B 0.01/0.1/0.3, C 0.03/0.3/0, D 0.01/0.1/0, E 0.02/0.2/0, F 0.03/0.1/0, G 0.01/0.3/0, H 0.02/0.2/0.1,
  I 0.03/0.3/0.1, J 0.01/0.1/0.1; K and L are A with every parent link above |w| 1 (K) or 5 (L) kept from pruning
  (`mutants-for-main-after-pr752.rs`'s tau). Results: handoff log, 2026-10-21.
