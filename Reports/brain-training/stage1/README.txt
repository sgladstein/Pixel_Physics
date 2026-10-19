Stage 1: an evolved target scorer (dig cell, spoil site) on today's brain (frozen, game walk).
Binary: brain-training/bin/stage1-f3b9b354/train (patch beside it; local commit on top of PR 708 cf9b4350).
Mount is noexec: copy the binary to local disk, chmod +x.

Rungs (free= hands that hand filter to the scorer too), hand-built scorer, 8 hold-outs (rung-hand-*.log):
  rung            hand-built  legal  scorer-blind  hand rules
  none (cell only)   75.2      73.0     73.0         76.0   <- only ~140 of ~5,500 dig decisions had a choice
  holds              70.8      71.0     37.2         76.0
  cue                71.5      72.0     67.2         76.0
  face               70.2      75.2     72.5         76.0
  company           107.0     102.5     92.8         76.0   <- the company rule costs room at 12 ants
  all four           86.8      77.0     50.2         76.0
12 hold-outs (noise-*.log): hand rules 77.2 sd 5.0; hand-built (rules kept) 78.3 sd 5.4;
  hand-built all four 89.8 sd 10.8; blank all four 64.9 sd 10.9.
One arena 2.2 s with or without the scorer (cost=1, 3 alternating repeats).
Pictures: stage1-rungs-sheet.png (last frame of the first hold-out bed, nest crop x5).

Training runs (runs/): rung "all four", 3 seeds from the hand-built scorer, 3 from blank,
pop 48, gens 30, layouts 8, sigma 0.3, hold-out 12 every 5 gens. Only the scorer evolves.
Decision rows: champion vs hand (hand rules), legal (same none-rate, even cell), scorer-blind, start.
