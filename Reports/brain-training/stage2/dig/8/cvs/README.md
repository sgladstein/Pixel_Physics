# Copy-then-select against selection alone (2026-10-21/22)

The last arena run of the brain lane. Four arms, 8-row shaft, 12,000-frame soil worlds, racing, `keepref=` s91,
ref b0dc264: `cvs-copy-201/202` start from the teacher lane's copy v2 (its revived links never pruned),
`cvs-sel-201/202` from `training/starts/s171-8-g79.txt`. Results and reading: `Reports/handoff/brain-lane-handoff.md`,
log entry 2026-10-22 01:55. Picture: `cvs-result.png`.

- `*-hold-best.txt`: each arm's delivered brain. `*-train.log`, `*-progress.csv`: the run's own output (the
  snapshot keeps only the tail of `train.log`; copy-201's starts at generation ~100).
- `test96/`: `score=<hold-best> test=96 noise=1` outputs for each arm: `-soil` (8 rows, 12,000 frames),
  `-fetch` (`dist=10,30`), `-fetch30` (`dist=30,30`), `-trail`. Judge with `Reports/brain-training/tools/judge.py`
  against `../test96-scores/s91-*.txt`.
