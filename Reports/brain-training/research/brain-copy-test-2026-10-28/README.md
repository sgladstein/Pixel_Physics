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
- `c3-pruned.raw.txt`: the best-living copy (c3 at `prox=0.2`), a layout-8 genome. Run it with
  `labsurvive founders=c3-pruned.raw.txt@c3:1`.
