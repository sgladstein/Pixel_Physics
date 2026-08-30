# Lane H — motion decoys

**Branch** `claude/creature-lane-h-motion-decoys`, cut from `origin/main`
(`6d5cbcf`), `origin/main` merged in at `83bd4c4`.

**Cost fork: built the probe and answered the question.**

## The answer, for the coordinator and for Lane D

**Motion does not move the size at which a creature becomes findable — it
removes the size axis.** In ordinary weather a body that moves has **0–2**
competitors at *every* size from 1 to 16 cells, against 141 for a still
two-cell body and 15 for a still nine-cell one. A walking two-cell ant is
already better off than a stationary sixteen-cell one, on four seeds and in
every sky measured, including a storm.

**This qualifies `creature-appearance-design.md` rather than demolishing it,
and Lane D should read the split before changing course.** The static ladder
is correct and still governs a population that is not small: **22–42% of ants
never moved once across a ~384-frame horizon**, and only 25–41% register any
motion in a given 8-frame gap. For a resting ant, extent is still the only
lever and the report's numbers stand unchanged. For a walking one, nine cells
buys nothing measurable while costing 4.5x the body energy per hatch, ~a
third of the placement sites and an 8–10x blocked-movement rate.

**The lever that follows is behavioural, and it is the one §7 of that report
says evolution can actually reach**: `individual_as_species` copies the body
and the palette is keyed by species name, but `genome` and `traits` — the
brain — are the individual's own. *How often an animal moves* is brain-side.
Not measured here; flagged because it is the first appearance-adjacent lever
the E5 question can be answered "yes" for.

## What landed

- `examples/motion_look.rs` — the instrument. Reuses `creature_look`'s
  `luma`, `SURROUND`, pinned daylight and window geometry verbatim, so its
  static column *is* that report's `decoys`; both counts come out of one
  loop, so `moving <= still` is true by construction. `mode=probe` is the
  size ladder plus four controls, `mode=live` is real ants, `weather=` pins
  the sky, `out=x.png` writes a review-queue frame sequence.
  **`examples/creature_look.rs` was read and not edited** (Lane D), and
  nothing under `src/` or `assets/` was touched.
- `Reports/creature-motion-decoys-2026-08-30.md` — the report, with its line
  in `Reports/README.md` in the same commit.

## Controls, because a clean answer is the tell

Three are asserts that abort the run: a **frozen pair** changes 0 pixels (so
nothing here is render-side grain animation), the counter fires on all
**157,752** windows at contrast 0, and at motion 0 the moving column equals
the still column exactly. The fourth is target-side: a probe painted and
moved one cell is flagged MOVING at 1, 2 and 9 cells while the same probe
held still is not. One negative control fails in one condition — in a
**blizzard** a 1-cell still probe reads MOVING, because snow lands on its
only pixel; at 2 cells and up it holds.

The first result was tidy (moving decoys 0, ambient 0) and was not trusted
until the weather arms produced 2,870–6,316 moving pixels and a blizzard
produced 13 moving decoys at 2 cells. The world these reports measure on is
genuinely at rest; it is not a broken instrument.

## Review card

Posted as a **frame sequence** (the skill's own measured note: a GIF showed
the owner one static frame where a sequence played), with the still and the
moving decoy counts in `meta`. Fire-and-forget; collect with
`review.py inbox`.

## Re-taken after main was merged in

Another session merged `main` into this branch at `9323e2e`, bringing **#142**
(ants starve, a birth grant), #145, #146 and #149. My three files were not
touched by it. Every figure in the report was **re-taken on the merged head
and is identical** — ladder, ambient motion, all four live seeds, the
never-moved fractions.

Identical output across a merge is the stale-binary tell, so it was checked
rather than assumed: rebuilt after the merge (binary 05:43:34 against
`creature.rs` 05:39:03) and the binary carries `birth_grant`, which exists
only in the merged `ant.ron`. It is unmoved because this harness runs 600
frames of founders — nothing hatches and nothing starves in that window, so
#142's ledger changes cannot reach it. Recorded in the report as §7a,
including that a colony run long enough to starve is where the never-moved
fraction would be expected to move.

## Landing

- **Head SHA** `c997c58` — *The decoy field is static, and an ant is not:
  motion measured*.
- **PR** [#150](https://github.com/sgladstein/Pixel_Physics/pull/150),
  opened from this lane.
- **Review card** `20260830T045018630Z-abf50b`, board `creatures` — one
  frozen frame against 20 stepping ones, still and moving decoy counts in
  `meta`. Fire-and-forget; collect with `python3 scripts/review.py inbox`.
- `cargo +1.98.0 clippy --all-targets -- -D warnings` clean (three findings
  the container's 1.94.1 accepted), `bash scripts/docscheck.sh` clean.
  `origin/main` merged in at `83bd4c4` and the headline numbers re-measured
  identically on the merged tree.
