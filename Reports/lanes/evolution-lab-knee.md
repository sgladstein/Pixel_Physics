# Evolution lab — the knee lane (round 34, lane A)

Owner of `examples/antcost.rs` and
[`../evolution-lab-knee-2026-09-14.md`](../evolution-lab-knee-2026-09-14.md).
Brief: *find out what changes at the knee; do not fix anything.* Nothing under
`src/` was touched, by instruction — the point was to measure the trunk rather
than a change.

---

## 2026-09-14 — there is no knee, and half an ant is not in the creature pass

**One line fits 0 to 793 ants**, on two bed widths and two seeds, once bed age
and plant load are held still: `cost ≈ 925 + 3.94·ants` at 512x512 with
residuals inside ±78 µs. The chord that runs straight through the 400–600
threshold reads **4.34 µs/ant** against **3.78** for the chord below it. A
knee would put those at 0.5 and 2.7.

**The cheap regime is reproducible as an artifact.** The same harness on the
bed round 32 used — planted, arms at their natural ages — gives **1.04 µs/ant
from 108 to 230 ants and 5.22 from 230 to 542**, a 5.0x step. The plant column
says what happened: ants eat, a plant costs 2.0–6.4 µs/tick against an ant's
~4, and the step sits exactly where the plants bottom out. In a planted bed
the marginal cost of an ant is **negative** over part of the range — 36 to 122
ants takes the frame from 2,554 µs to 2,250.

**The redirect for round 35.** Measured with `SCHED_PASS` rather than by
subtraction: the creature phase is **45%** of the frame's growth from 0 to 428
ants and the cost of one decision is **flat** (6.1–9.5 µs over a 17x range in
population). The other 55% is the CA sweep over the **29.4 cells per ant per
frame** the ant leaves dirty. Two terms fit to ±60 µs:
`cost ≈ 452 + 1.89·ants + 0.0615·swept`, and that 1.89 matches the directly
timed creature phase at 1.87. Round 33's parallel read phase measured its own
hit rate at 0.35; it is the same share from the other side.

**Struck off by measurement, not argument:** the active-site scheduler
(`lag/tick` 0.000 and `lag max` 0 in every arm to 793 ants, heap depth 1,400
against caps of 2,000+256, ticks per ant per frame flat at 0.195–0.198);
density (4x bed width at fixed count does not lower the per-ant cost); jamming
(`blk%` 7–15% with no trend); contention (everything at
`RAYON_NUM_THREADS=1`, `par=off`); the moisture pass as a per-ant term
(`sw seen` flat in ant count, 15,553 against 15,820).

## The four rules any later population sweep inherits

Each cost most of this session to find and each silently changes the answer.

1. **Age-match every arm** (`age=N`, new). `antcost`'s stocking loop makes bed
   age a function of ant count — frame 4,440 against 1,000 — and an empty bed
   costs 429.8 / 924.3 / 712.2 µs at ages 1,000 / 5,000 / 9,000 with nothing
   else changed. That 2.15x is the water cycle on the frame clock.
2. **Never subtract the `ants=0` arm.** Its soil-water state was never
   disturbed by a founding; in three runs it costs *more* than beds with 4–20
   ants in them, so a per-ant figure built from it can come out negative.
3. **`founders=0` for any cost question about animals**, or the ant term is
   measured against a subtraction that moves the other way.
4. **`[sched]`'s `total` is the scheduler phase, not the frame** — 0.80 ms
   beside a 2.75 ms frame at 428 ants, with the creature row ~100% of it.

## Reach of the harness

**~900 ants is the ceiling and width does not lift it**: 892 on a 2048-wide
bed, 916 on 4096, at `rounds=250 settle=20`. The loss is starvation, so a bed
with standing food is what reaches three thousand.

## What `antcost` gained

`age=`, `widths=`, `heights=`, `plants=` (herb founders as an arm axis),
`swept=1`, and the `sites/f`, `lag/tick`, `lag max`, `plants`, `pcells`
columns. Fits are now grouped per `(bed, founders, par)` — a fit pooled across
beds fits a line through two backgrounds and calls the difference an ant.

## Handover

Branch `claude/evolution-lab-knee`, **PR #407**, opened against `main` merged
through #406. The coordinator owns the merge; read the head SHA off the PR
rather than off this line, which is committed before the push that carries it.

**Nothing under `src/` was touched**, so there is no behavioural change to
review — the diff is `examples/antcost.rs`, the report, this note, and the
index lines. Gates on the branch: clippy clean, `--lib` 1,723 passed / 0
failed, `--test worldgen --test determinism` 44 passed / 0 failed, `docscheck`
clean, `deadendindex --touching` silent.
