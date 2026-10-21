# Design: a reserved test set and a progress probe for the trainer

*2026-10-21, brain lane, at the owner's request ("what is the correct way to do this?"). Status: **built**, after an independent review (one BLOCKER, three MAJOR, three MINOR; all taken -- see the end).*

## Why

The trainer uses one fixed set of hold-out worlds (`holdout_layouts`, seeds 50,000+, 24 in every dig run) for two jobs:
picking a run's best brain (`hold-best.txt`, the `top` genome) and deciding when to stop (`flat`, 50 generations after
the last hold-out gain). Whatever it picks there is flattered there. Measured 2026-10-21 on the second 8-row round
(8 rows, 12,000 frames), the four arms' hold-best brains read **13.2-13.7 full loads on those 24 worlds and 10.8-12.3 on
72 worlds nothing was ever picked on**; the start, b181, itself a hold-best on the same 24, reads 11.42 against 11.08.
So the trainer's own curve cannot measure progress: it showed every arm "below its start" while three of four were
ahead on never-used worlds, and its stop rule ended one arm at generation 55 while it was +0.85 ahead there. The
brain lane's own judging had the same flaw: its "96 fresh worlds" (`holdout=96`) are `holdout_layouts(96)`, whose
first 24 are the trainer's hold-out, and its 24 sealed worlds have been used to choose between brains.

The standard answer is three sets with one job each: **training** worlds choose parents (new each generation, as
now); **validation** worlds choose the run's best brain and when to stop (always flattered for what they choose, so a
selection tool, never a progress measure); **test** worlds are never used to choose anything, and every progress
claim and comparison is made on them.

## What

1. **`train::test_layouts(n)`**: seeds `30_000 + k / 2`, mirrored pairs like every other set. That range is clear
   of training (run seed 0: 1,000 up to 25,500 for its first 49,000 layouts; run seeds 1+: 101,000+), hold-out
   (50,000+), sealed (70,000+) and calibration (90,000+) for any `n` up to 40,000. A test asserts the ranges are
   disjoint at the sizes used.
2. **A progress probe in `train()`** (`Settings::probe`, `probe=N`, default 10; 0 is off): every N generations, and
   at the last one, the run's current hold-best genome (`top_genome`; the start before the first hold-out) is scored
   on `test_layouts(Settings::probe_worlds)` (default 96) with `score_rows`, which draws nothing from the training
   stream, and written to `test.csv` (`gen,who,fit,top_gen,parts...`). The start is scored once and written as
   `who=start` (re-scored once on a resume). **Nothing reads `test.csv`**: not the ranking, not `top`, not `flat`.
   On by default (house rule: features default on unless a measured harm): at the dig settings it costs 96 worlds
   every 10 generations against about 2,840 a generation, about 0.3%.
3. **Validation size**: no code. Launch dig runs with `holdout=48` from now on (the noise behind the early stop;
   about 1-2% more a generation). Noted in the handoff's run arguments.
4. **`test=1` in `examples/train.rs`** beside `sealed=1`: `score=` / `noise=1` runs on `test_layouts` so manual
   judging uses the reserved set. Judging from now on: test worlds only; sealed is retired to validation use.

## Guards

- **The probe changes nothing a run does**: a short `train()` on the bowl arena with `probe=1` and `probe=0` writes
  identical `progress.csv`, `holdout.csv`, `state.txt` and champions (positive control: `test.csv` exists with a row
  per generation in the first, and not in the second).
- **The probe reads the hold-best, not the generation's champion**: on an arena whose hold-out and training disagree,
  `test.csv`'s `top_gen` equals `state.txt`'s `top_gen` every row.
- **Disjoint ranges**: `test_layouts(40_000)` shares no seed with training (run seeds 0 to 9, 200 generations of 96),
  `holdout_layouts`, `sealed_layouts`, `calibration_layouts` at 10,000 each.
- **Resume**: a run resumed partway writes the same `test.csv` as one that never stopped (rows of generations before
  the resume kept, the start re-scored once and identical).

## Review and what changed (2026-10-21)

- **BLOCKER, fixed:** a resume of a run saved before the probe (no `test.csv`) would have panicked in `rows_before`.
  `test.csv` is opened by `rows_kept(.., missing_ok)`, which reads a missing file as empty; the others stay strict.
  Guard `a_run_from_before_the_probe_resumes_with_it_on`, red with `missing_ok` ignored.
- **MAJOR, fixed:** the start is written once per run (only when no `start` row survives a resume); probe rows are
  written before the state is saved, as `holdout.csv`'s are.
- **MAJOR, fixed:** in a gated suite the floor depends on how many units are scored, so 96 test worlds veto
  differently from 24 hold-out worlds. `test.csv` carries `unfloored` (the product with no veto) beside `fit`, and the
  `_under` flags in its parts.
- **MAJOR, fixed:** `holdout=48` alone would also have moved the suite's calibration (references and floors), since
  calibration used `holdout`'s size. **`calib=N`** sets calibration apart (default: `holdout`, as before; written into
  the arena string when it differs), so the hold-out can grow with the floors where they were. `test=N` is a world
  count of its own; `sealed=1` with `test=N` is refused.
- **MINOR, taken:** the range check is a runtime assert in `train()` (`assert_training_clear_of_test`; run seed 0
  reaches 30,000 after 58,000 layouts); the probe also scores **the generation's champion** (`who=champion`), which
  neither selection chose, so a population moving ahead while the hold-out stalls shows; the hold-best's score is
  cached while it stands; the guards zero the clock, check the layouts are the test range, and check the hold-best
  row's `top_gen` and fit against the state with the hold-best from before the first probe (a probe of every
  generation was blind to "probes the champion": at the generation the hold-best changes they are the same brain --
  found by putting the fault back, fixed by probing every 4).

**Launch arguments from now on (dig runs):** `holdout=48 calib=24` (validation grows; floors as calibrated before),
probe at its default (every 10 generations, 96 test worlds). Judge by `test.csv` and by `score=` / `noise=1` with
`test=96`, never by the hold-out.
