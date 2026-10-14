# why-no-emergence-2026-10-14 — the §5 measurement

Scripts behind [`../why-no-emergence-2026-10-14.md`](../why-no-emergence-2026-10-14.md) §5: is an ant's
energy reserve about one foraging round trip, and how do ants die when a placement rule is on?

- `run.sh ARM SEED OUT` — one `deeptrace` run of the dry goal box (`nest_goal`, evolved founder, mutation off,
  300,000 frames, `ants=60 hungry=1 mapevery=1000`). Arms: `h30` (the box's food, 30 columns east), `h90`
  (`foodgap=90`), `nurse` (`PIXEL_PHYSICS_NURSE_STAY=on`), `rest` (`PIXEL_PHYSICS_NEST_REST=workers`). Seeds 1-4
  were run; the data regenerates deterministically on the same build and is not kept.
- `margin.py` — burn rate (J a frame with an empty crop and nothing digesting), every focal decision's reserve in
  frames of living, focal foraging trips (home zone -> heap zone -> home with food) and the reserve each left with,
  deaths by cause.
- `starvers.py` — each starved adult's last hunger spell: stuck (moved on under 10% of decisions), held (over half
  of it under a pull that does not lead to food), or free.
- `stuckway.py` — for starvers that died in the nest (exact cause `STARVED`, window `LO`-`HI`): depth, nearest
  approach to the door, moves.
- `shaftcensus.py` — the door shaft (7 columns x 16 rows under the door, the ground row included): heads, occupied
  cells, share at the stacking cap, share of the colony and of below-ground ants, who is carrying food or lean;
  `BINS=1` adds a 20k-frame timeline of shaft, colony, below-ground ants, starvers, births and food. Adapted from
  the adversarial review's scripts.

Known limits: four seeds; `nurse` and `rest` at food 30 only; the `NURSE_STAY` die-offs fit a jam, a population
boom and food diverted to brood alike, and no run here separates them (the report's §5.3 names the run that would).
