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
- `errand-ceiling.json`: addendum 2's numbers (the ceiling shown the walk's memory and errand, and in the heading-pick
  arms the walk's own scores), from `PP_DECLOG=1 PP_SHADOW_STATE=1` copy logs.
- `g1-twin-check.txt`: the raw `RESULT` rows of the twin-seed check (g1 vs c3 vs std_pool, held-out seeds 7-12, 60k).
- `g2-mixed.raw.txt`, `g3-mixed-plant3.raw.txt`, `g4-mixed-famine2.raw.txt` (Teacher lane's ask, 10-28): copies fitted
  from std_pool member 0 on two label sources at once -- the hand-built ant's shadow labels in the two famine twins,
  and std_pool's own picks while it played `lab_herbs` and `lab_far_mix` (`PP_COPY_SAMPLE=200`, relabelled with
  `scratch/copytest/selflabel.py`), seeds 1-4. g3 lists the plant logs three times, g4 the famine logs twice (the fit
  samples equally per listed file). Billed links 1,137 / 981 / 983. `g2-g4-twin-check.txt`: their `RESULT` rows on
  held-out twin seeds 7-12 at 60k (std rerun identical to `g1-twin-check.txt`). Twin check, mean colony size,
  far_mix / herbs / famine_scatter: std 238 / 50 / 4.4; g2 226 / 67 / 5.5; g3 237 / 55 / 4.9; g4 267 / 62 / 5.1;
  g1 11 / 31 / 8.3. g4 beats std 23/1 there. At 600k on the exam boxes (Teacher x5) g2 and g3 brought std_pool's
  plant walk back but lost g1's two_larders survival (alive 0 and 1 of 8 vs g1's 6 of 8).
