# needs-ant/tools — reviewer scripts (2026-10-06)

Written by the adversarial reviewers of the needs-and-jobs ant design. Read-only over the shared baseline
(/mnt/project-files/deep-trace/baseline/3ba1e7bd5). Run with `python3 -I`.

- nestwhere.py BASE SEEDS FROM TO — where today's "time in the dug nest" (zone nest = head below the old
  ground line) sits: depth bands under the ground line, door column top 10 rows, room vs tunnel, sealed,
  holding soil. e.g. `python3 -I nestwhere.py /mnt/project-files/deep-trace/baseline/3ba1e7bd5 1,2,3,4 50000 300000`
- roomshape.py BASE — the scorecard's room rule (7 of 9 neighbours open) on the 12 baseline maps: rooms of 30+ cells,
  their row and column span.
- cover.py — where `under_cover` (any soil within 20 rows straight up) is true for ant cells: mound vs dug nest.
- larva_reach.py — share of dug-nest cells within the shipped larva sense's 6-cell box of any brood cell.
- where.py / throat.py DIR — nest-air level (sealed walls) by place; need a directory holding a copy of
  ../gradient-check/gradcheck.py as their argument.
