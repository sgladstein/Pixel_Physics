# Copy test data (2026-10-28)

The report is [`../brain-copy-test-2026-10-28.md`](../brain-copy-test-2026-10-28.md).

- `boxes.csv`: every exam run, one `labsurvive` `RESULT` line per arm, box and seed (seeds 11-18, `after=60000`,
  one tank). Made by `scratch/copytest/boxes.py`; compare arms with `scratch/copytest/cmp.py boxes.csv A B`.
  Rows written before `trip_deliveries` was added to `RESULT` leave it empty.
- `landing-trace.txt`: the landing table. Per arm and bill setting, four `labsurvive scenario=feast_and_famine
  seed=11..14 after=1500 grant=0 trace=250 [tax=0]` runs, summarised as the mean over seeds of the median ant.
  The raw `TRACE` files (30k lines) were not kept.
- `fits.json`: each fit's arguments, start and final scores (held-out P(label), by verb), and live links. Curves
  are dropped. Log names are `lab/<box>-<seed>.log` (shadow labels) and `dag/<copy>/<box>-<seed>.log` (relabelled).
- `self-agreement.json`: the addendum's table, the hand-built ant's agreement with itself per move (mean over the six
  held-out runs, total ticks), from `PP_SELFAGREE=20` copy logs.
- `c3-pruned.raw.txt`: the best-living copy (c3 at `prox=0.2`), a layout-8 genome. Run it with
  `labsurvive founders=c3-pruned.raw.txt@c3:1`.
- `g1-twins.raw.txt`: the generalist copy for the Teacher lane (2026-10-28), a layout-8 genome. Fitted (`copyfit.py`
  `switches=1 plain_k=1 prox=0.2`, 6,000 steps, from std_pool member 0) on the hand-built ant's shadow labels in the
  four selection twins only: `lab_famine_scatter`, `lab_famine_trickle`, `lab_herbs`, `lab_far_mix` (branch
  `claude/project-thread-8hfe09` @4379f253), seeds 1-4 for training, 5-6 held out. The six exam boxes were never
  seen. Held-out P(label) 0.360 (walk 0.255, turn round 0.130, rest 0.505, dig 0.093, pick up 0.383, put down
  0.295) on 45,000 decisions; 690 billed links.
