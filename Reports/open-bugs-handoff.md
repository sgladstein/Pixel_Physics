# Open bugs handoff: sand-into-water displacement

Rewritten after the session that landed `15b2e51`, `38a8799`, `0717eec`.
Everything here was measured, not reasoned — where something is a guess it
says so.

**The chunk-seam bugs that used to be §1 of this document are fixed**, for
both powders and liquids. What they turned out to be is recorded below,
because the wrong answer was recorded here confidently for a whole session
and the method that eventually found the right one is worth reusing.

---

## 0. Closed: chunk-seam cliffs and terracing (was PRIORITY)

### What it was

Sand dropped as a blob held a sharp vertical face exactly on a vertical
chunk boundary for ~25 seconds. Water spreading across seams held flat
plateaus with sharp risers on the gridlines, each riser fringed with
ragged one-cell-tall horizontal films (reported separately as "banding on
moving water" — same bug, seen close up).

### The recorded leading hypothesis was wrong

This document previously said the cause was the two-angle repose model
never flipping seam-adjacent cells into `flowing`, so they held
`max_stability_angle`. **Measured: false.** Seam-adjacent cells *do* get
`flowing()` set — instrumented directly, they carry it while a control
column two cells away does not. Do not revisit this.

### Actual cause

Both drivers sweep **chunk by chunk**, so every cell in a chunk is updated
before any cell in the chunk to its right. A free face landing on a
boundary became a one-column conveyor: the seam column shed exactly one
cell per frame off its bottom while the whole column above slumped down
one to refill it, and the chunk to its left — already swept that frame —
could not widen the face.

Instrumented in the seam column: **33 straight-down slumps against 0.9
sideways escapes per frame**, where a single-region sweep of the identical
state gave 9 escapes.

### What found it

`update::step_monolithic` — sweep the whole world as one region, ignoring
chunk decomposition. Kept as `#[cfg(test)]` and as a live test
(`chunking_the_sweep_does_not_change_where_a_pile_settles`). It answers
the one question three wrong hypotheses had all been guessing at: *is this
coming from the movement rules, or from how the sweep is cut up?* From
byte-identical starting state it produced zero seam cliffs, and relaxed an
already-dammed frame-400 state ~10x faster.

### The fix

`FLAG_UNDERCUT` (`cell.rs`): a hole opened by a move with a horizontal
component may not be slumped straight down into for the one frame the flag
survives, so the cell above has to find its own sideways escape — which,
on a face, is the avalanche that was missing. Straight-down moves
deliberately do not set it, because a column falling through air *must*
descend as a unit.

Set and read for `Powder` and `Liquid` only. It is read back from a cell
its writer does not own, so leaving it ungated let a gas rising past a
sand pile stall the sand for a frame.

### Also fixed: chunks awake but never swept

`Chunk::is_settled` now answers from `sweep_region` rather than `dirty`, so
a chunk whose dirty mark cannot expand back into its own bounds counts as
settled instead of sitting awake forever. 3 such chunks → 0. The
alternative (clamping `mark_dirty`) stays reverted; it discards issue #3's
optimization and fails
`neighbour_waking_stops_at_the_neighbours_own_reach`.

---

## 1. Sand-into-water displacement (the only open bug here)

### History

Dropping a dense blob into water made the water appear on top of the blob
almost immediately and spray sideways out of it. Root cause, measured:
rows are swept bottom-to-top, so as the sweep works upward each successive
sand cell displaces the *same* water parcel again, and it crossed the
whole height of the blob in one frame.

- `c759836`: `move_cell` marked the displaced cell `with_moved(!revisited)`
  rather than unconditionally `false`. Necessary but not sufficient.
- `abffff2`: `try_move` gained a `dst.moved()` refusal, so an
  already-moved cell cannot be displaced again in the same frame.

### The explicit better/worse call this document asked for — decided

Measured on a walled pool with a 22-radius sand blob dropped in, worst
value over 400 frames under the parallel driver:

| metric | before `abffff2` | now |
|---|---|---|
| water rise | **29 rows/frame** | **1 row/frame** |
| sand/water/sand stripes | 41 | **1379** |
| sand cells with air beneath | 86 | 115 |

**Decision: keep `abffff2`. Option 3 (revert) is closed.** Water crossing
29 rows in a single frame is a gross physics violation and was rated
clearly wrong from live play; one row per frame is correct. The striping
it traded for is genuinely much worse than this document previously
described — not "one row sand, one row water" rippling upward, but the
whole blob dissolving into a persistent checkerboard, still ~1370 stripe
sites at frame 80.

### Option 1 was tried and does not work — measured

"Sideways-preferring displacement" (place the displaced lighter cell in a
free-or-lighter cell up-left/up-right in preference to the vacated cell
directly above), implemented as a proper 3-cycle so mass is conserved:

| metric | now | with option 1 |
|---|---|---|
| stripes | 1379 | 1370 |
| sand in air | 115 | 115 |
| water rise | 1 row/frame | **2 rows/frame** |

No effect on the striping, no effect on the stall, and it *regressed* the
one metric `abffff2` exists to protect. **Reverted, not committed.**

The reason it cannot work as specified: inside a pool there is no
free-or-lighter cell beside the mover. The blob's own cells are denser and
the surrounding water is the same material, so the sideways path only ever
opens where the blob is already at a free surface — which is exactly where
striping was never the problem.

### Why this is harder than the remaining options suggest

The striping is not a bug in the refusal; it follows from two things that
are each individually correct:

1. Displaced material may move at most one row per frame (`abffff2`).
2. Displacement is a straight vertical swap, so water under a sinking body
   has nowhere to go but *through* it.

Given both, a 44-cell-tall blob **must** take ~44 frames to pass a water
parcel, and the column must alternate while it does. No local tweak to
`try_move` changes that; the two premises jointly imply the stripe.

So the real options are:

1. **Option 2 from the old list** — let a refused mover fall anyway when
   it has no support. Contained, and addresses only the floating-sand
   visual (115 cells), not the striping. Worth doing on its own merits.
2. **Move a coherent body as a body**, rather than cell-by-cell swaps —
   the `rigid.rs` direction. This is the only thing that actually removes
   the premise. Large.
3. **Accept it and reduce blob density in play.** Loose material stripes
   far less because it does not present a solid cross-section.

Not attempted. Do not attempt another local `try_move` tweak without a new
idea about premise 2 — the sideways-preferring one is now measured not to
work, and is the obvious one.

---

## Standing methodology note

Still the most valuable thing in this document, and it earned another
entry this session.

Every fix in this area judged by test output alone has failed to change
what the owner saw. The ones that worked were driven by a reproduction
built from the owner's own description of the *initial state*, confirmed
to reproduce the complained-about quantity **before** any fix was written,
and verified with a live `cargo run` afterwards (the framebuffer capture
hook in `main.rs` makes this cheap: set
`PIXEL_PHYSICS_CAPTURE_SEQUENCE=<start>,<interval>,<count>` and edit the
scene into `build_terrain`).

**New this session: the obvious metric was wrong twice, in the same way.**
For water, measuring the topmost water cell per column showed chunk seams
at worst 1.7x the interior roughness — which reads as "no seam effect at
all" — while measuring the same scene by *volume* showed 9.0x and
climbing. A `Liquid` cell holds a continuous fill, `render.rs` dims it
toward black by that fill, and the near-empty film cells fringing a riser
are nearly invisible on screen but count at full height in a
topmost-cell metric, smoothing over precisely the riser being complained
about. Three reproductions failed before the metric was changed to volume.

When a reproduction "doesn't reproduce", suspect the metric before
suspecting the scene.
