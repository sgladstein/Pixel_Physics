# What a bigger world costs, and how long the field takes to go quiet

*Round 7, sizing the 4x world. **This file replaces an earlier version whose
headline claim was wrong**; the correction is kept in full at the bottom,
because how it went wrong is more useful than the number it got wrong.*

## The measurement

`examples/scale_probe.rs`. Generated world, no player, no input. It steps
until the field goes quiet — quiet being `QUIET_RUN` consecutive frames
whose `field::step` costs under `QUIET_MS` — then measures at rest.

| size | cells | place | structural | gen | peak RSS | quiet at | settled sweep | settled field |
|---|---|---|---|---|---|---|---|---|
| 2048×640 | 1.31 M | 384 ms | 202 ms | **586 ms** | 37 MiB | frame 4501 | 0.00 ms | 0.23 / 1.34 ms |
| 4096×1280 | 5.24 M | 1503 | 903 | 2405 | 138 MiB | — | — | — |
| 6144×1920 | 11.8 M | 3415 | 2244 | 5658 | 305 MiB | — | — | — |
| 8192×2560 | 21.0 M | 6488 | 5325 | **11 813 ms** | **539 MiB** | frame 4501 | 0.07 ms | **4.87 / 10.72 ms** |

Settled figures are mean / worst over 60 frames with **zero awake chunks**.

## What it says

**Chunk sleeping works completely.** A settled 4x world — sixteen times the
cells — sweeps in 0.07 ms. Scale costs the falling-sand simulation nothing.

**The field has two separate costs, and they need separate fixes.**

1. **A transient of ~4500 frames — about 75 seconds** — during which the
   field costs 8 ms/frame at the shipped size and up to 42 ms at 4x. Pressure
   churns (max frame-to-frame swings of 2–27 units, ~3900 of 20480 field
   cells above the settle epsilon) and then decays cleanly and exponentially:
   past frame 4200 the max drift falls 0.0084 → 0.00001 over the next 1400
   frames. **The time to quiet is the same at 1x and 4x** — it is a time
   constant of the solver, not a size effect.
2. **A steady state that is not free at 4x**: 4.87 ms mean, 10.72 ms worst.
   That is the sky. `sky_light_amplitude` is quantised (see below), so the
   full five-pass solve runs on the ~760 frames in 3600 where the sky
   actually steps, and at 4x each of those costs ~20 ms.

**Generation and memory are the other two blockers**, and are unrelated to
the above: 11.8 s and 539 MiB peak at 4x, with
`structural::compute_world_distances` at 45% of generation and holding a
second full-grid mirror plus a distance array while it runs — so it is both
the largest time cost and the reason peak memory is roughly double the
steady grid.

## Changes made while measuring

**Kept: `sky_light_amplitude` is quantised** to `SKY_LIGHT_STEP = 0.01` —
0.26% of the 0.2..4.0 range, under one step of the 8-bit colour it is drawn
through, and above the 0.005 settle epsilon so a step registers.

The old code ran the day/night clock on an accident, and said so: the sun
moves less than `SETTLE_EPSILON_LIGHT` per frame, so the `amplitude_changed`
flag written to drive it "is essentially never true", and the sky advanced
only because the pass wrote a slightly different value every frame and its
tiles therefore never converged. Quantising makes the amplitude piecewise
constant, so the flag fires on the frames the sky actually moves and the
clock runs on the mechanism intended for it.

Quantised on the 0..1 daylight *fraction*, not the amplitude, so both
endpoints stay bit-exact and
`sky_light_amplitude_cycles_between_the_night_floor_and_max_light` keeps its
`assert_eq!`s. The first attempt quantised the amplitude and moved the night
floor to 0.19999999; weakening that assertion to fit would have been the
wrong way round.

**Reverted: subsetting `apply_sky_to` by column.** Gating each column on
`sky_drifted` measured 8.26 → 6.55 ms and was measuring a bug. A tile in the
solve set is rebuilt from a fresh `FieldTile`, so a skipped column does not
keep stale light — it **loses** it. Measured: mid-air cells going `2.43 ->
0.0` and staying there, because the fresh tile also takes
`sky_amplitude = amplitude` and so reports no drift to ask for a repair.
Corrected to "solved OR drifted" it covered the whole world anyway and cost
8.26 → 9.06 ms.

**It is now worth retrying, for a reason that did not hold then.** Both
measurements above were taken *inside the transient*, when the solve set is
most of the world and there is nothing to subset to. The steady-state cost
is a different regime: on a sky-step frame the only thing that needs solving
is the sky-lit tiles. Retry it against the settled numbers, not the
transient ones.

## Next, in order

1. **The steady-state sky cost** (4.87 ms mean at 4x) — retry the column
   subset against settled numbers, with the light-erasure trap above in mind.
2. **Generation** (11.8 s) — `compute_world_distances` first: 45% of the time
   and the whole of the memory spike.
3. **The transient** (~75 s at 8–42 ms/frame). Lowest priority of the three:
   it is paid once at load, it does not grow with world size, and it may be
   legitimate physics settling rather than a defect. Establish which before
   touching the solver.

---

## The correction, kept because the failure is instructive

The first version of this file led with: *"the field never settles, and it
costs a third of the world every frame"*, and attributed it to a pressure
channel that never converges. **That is wrong.** Pressure converges cleanly
at about frame 4500.

Three separate instruments each gave a confident wrong answer, and each was
wrong in the same way — **it could be satisfied without the world being
quiet**:

- **A fixed frame count.** The probe ran 400 frames and called what followed
  "settled". Everything downstream inherited a transient measurement as a
  steady-state one. The solve set was then observed to be flat at 37% of
  tiles "from frame 650 to 3000" — true, and stopping at 3000 when
  convergence lands at 4500 is how a decaying transient reads as a permanent
  one.
- **`world.fields_settled()`.** Latched true at frame 47 while the field was
  demonstrably still churning: it is a verdict over the tiles *actually
  solved* that frame, so an empty or lucky solve set reads as everything
  being settled.
- **A lattice of `field_at_bilinear` probe points.** Reported quiet at frame
  0. Points spread evenly over a world mostly land in solid rock, where the
  pressure never moves at all — a metric that cannot distinguish a still
  world from a blind sampler.

What settled it was keying on the **cost** rather than on any notion of
stillness: the two regimes are 40x apart (42 ms/frame converging against
0.01 ms converged), which no vacuous reading can fake. And keying the
diagnostic to `world.frame` rather than to a call counter, since the field
is stepped intermittently and the two diverge by thousands of frames.

The rule this earns, which is the sibling of "ask what a metric counts when
nothing is wrong": **ask what a metric says when the thing it measures has
not happened yet.** All three instruments answered "quiet" for a world that
was not quiet, and none of them could have said so.
