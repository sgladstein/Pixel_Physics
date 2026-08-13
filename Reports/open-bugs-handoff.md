# Open bugs handoff: chunk-seam cliffs, and sand-into-water displacement

Written at the end of the session that landed `eb8d427`, `e7c6ebd`,
`c759836`, `abffff2`. Both bugs below are **open**. Everything here was
measured, not reasoned — where something is a guess it says so.

Read this before touching either area; three hypotheses in that session
were wrong until measured, and two reproductions failed because they
measured the wrong quantity on a scene that didn't exercise the real path.

---

## 1. Chunk-seam cliffs (PRIORITY)

### Symptom, from live play

Sand (and any material) dropped as a blob spreads horizontally when it
hits the ground. Where the spreading front meets a **vertical chunk
boundary**, it stops there and slows dramatically. The pile holds a sharp
vertical face exactly on the chunk line — confirmed by the owner with the
F1 chunk-grid overlay on, the faces line up perfectly with the green
gridlines. It does resolve, but takes ~25 seconds, and looks badly
unnatural until it does.

Depends on the initial state: small piles, or piles aligned so the spread
never crosses a seam, do not show it.

### Reproduction (works — use this, do not invent another)

```
640 x 383 world, stone floor along the bottom row.
Three brush-sized circles dropped from height:
    paint_circle(150, 90, 46, SAND)
    paint_circle(300, 90, 46, SAND)
    paint_circle(450, 90, 28, SAND)
Step. At frames 400 / 800 / 1500, build the sand surface height per
column, then find adjacent-column height jumps >= 6.
```

Measured result (both `update::step` and `parallel::step`, near-identical):

| frame | cliffs >= 6 | of which within 1 cell of a seam |
|---|---|---|
| 400 | 3 | **3** — at x=127, 193, 255 (17, 36, 37 cells tall) |
| 800 | 2 | **2** — at x=193, 255 |
| 1500 | 0 | 0 |

The key observation: at frame 150 there are ~12 cliffs scattered
anywhere. By frame 400 **the only ones left are the seam-aligned ones**.
Seams are not *creating* cliffs — they are the only place cliffs fail to
relax.

**Two earlier reproductions failed and are recorded so they aren't
retried:**

- Measuring the *spreading front* (leftmost/rightmost sand extent) shows
  nothing — the front crosses seams smoothly. A vertical face persists
  *behind* an advancing front, so the front is the wrong quantity.
- A single contiguous block (rather than dropped circles) produces tall
  cliffs but **none of them land on seams**. The falling-blob impact
  matters.

### Ruled out (measured, not assumed)

- **Not the parallel checkerboard.** Serial and parallel show it
  identically. It is in shared rule code.
- **Not the row-order inversion.** An earlier investigation correctly
  found that `parallel::step` runs its four passes in index order so
  even-`cy` chunk rows always sweep before the odd row beneath them,
  inverting the bottom-to-top invariant at half of all *horizontal*
  seams. That is a real latent issue, but the cliffs here are on
  **vertical** seams (chunk columns), so it is not this.
- **Not `sweep_region()` returning `None`.** See the next section — this
  looked extremely promising and turned out to change nothing.

### The `sweep_region() == None` dead end (and a real, separate finding)

While hunting this, a genuine oddity turned up: chunks that are
`!is_settled()` (so they count as awake, and keep the world from ever
sleeping) but whose `sweep_region()` returns `None`, so they are **never
actually swept**. Measured on the reproduction above: 0 such chunks at
frames 100 and 200, then **3 at frame 400 and 3 at frame 600** — they
persist, re-marked by ongoing neighbour activity every frame.

Cause: `World::touch_neighbours` wakes a neighbour by calling
`chunk.mark_dirty(x, y)` with the position of the *write*, which by
construction lies outside that neighbour. `Chunk::sweep_region` expands
`dirty` by the chunk's own `reach` and intersects with its bounds, so if
`reach` is too small to drag that out-of-bounds point back inside, the
intersection is empty. `parallel.rs`'s `run_pass` already documents this
`None` case as reachable and works around it (putting the chunk back
unswept), which stops it being *lost* but not *stalled*.

**Tried: clamping `(x, y)` into the chunk's own bounds inside
`Chunk::mark_dirty`.** Result: eliminated the never-swept chunks
completely (0 at every checkpoint) and **changed the cliffs not at all** —
byte-identical cliff positions and heights at frame 400. It also failed
`sim::world::tests::neighbour_waking_stops_at_the_neighbours_own_reach`,
which documents that `None` as a *deliberate* issue #3 optimization ("an
empty neighbour chunk no longer pays for a wide, pointless sweep just
because something moved far away in a chunk next door"). **Reverted.**

**But the awake-but-never-swept state is still worth fixing separately.**
It is a standing CPU cost and plausibly part of the owner-visible
"chunks 40/40 awake" while nothing moves. The right fix is probably to
let such a chunk drop back to *settled* (it provably has nothing to
sweep) rather than clamping, which preserves the optimization. Not
attempted.

### Leading hypothesis for the actual cliffs — UNTESTED

The **two-angle repose model**. A settled pile is allowed to hold the
steeper `max_stability_angle` until something flips it into `flowing`;
`FLAG_FLOWING` is set only by `CellSurface::move_cell`. If cells at a
seam specifically never earn that flag, they hold a vertical face and
relax only when something else happens to disturb them.

This fits every observed property: the shape (vertical, far steeper than
repose), the seam alignment, both drivers, and the slow eventual
resolution. **It has not been tested.** Start by instrumenting whether
seam-adjacent cells ever get `flowing()` set, compared with cells a few
columns away.

Second candidate, also untested: `roll_along_slope`'s sideways scan
behaving differently for a cell whose scan crosses a chunk boundary
(stale reads on the far side, or the scan terminating at the boundary).

---

## 2. Sand-into-water displacement (SECOND PRIORITY)

### Original bug and what was actually wrong

Dropping a dense blob (sand) into water made the water appear on top of
the blob almost immediately and spray sideways out of it.

Root cause, **measured**: rows are swept bottom-to-top. When sand at
`(x, y)` displaces water at `(x, y+1)`, the water lands at `(x, y)`. The
sweep then proceeds *upward* to row `y-1`, whose sand cell displaces that
same water parcel again — and again, once per sand row. On a walled
50-row sand block resting on water, the highest water row went from 150
to **100 in a single frame**.

An early claim in that session that this was "one cell per frame" was
**wrong** — the owner caught it. Do not trust that reading.

### What was fixed

- `c759836`: `move_cell` marked the displaced cell `with_moved(false)`
  unconditionally. `revisited` describes the *mover*; the displaced cell
  travels the opposite way, so when the mover goes down the displaced
  cell goes **up**, into rows not yet swept, and must be flagged. Changed
  to `with_moved(!revisited)`. **Necessary but not sufficient** — on its
  own it changed nothing observable.
- `abffff2`: the actual fix. `try_move` never consulted the
  *destination's* moved flag before swapping, so an already-moved cell
  could be displaced again immediately. Added a `dst.moved()` refusal.
  After: the water rises **exactly one row per frame**. Regression test
  `sim::update::displacement::*` under both drivers. Worst frame on the
  ascii stress scene also improved (9.7ms -> 8.0ms parallel), since the
  refusal removes redundant swap work.

The test scene is **walled deliberately**. An earlier version measured
identically with and without the fix because `find_lateral_descent`
carried the surface water away before the displacement path ever fired.

### NEW owner observation after the fix — the fix is not finished

The eruption is gone, but the replacement behaviour is also wrong:

1. Sand falls partially into the water.
2. Water propagates up **the same column**, producing a **striped
   effect** — one row sand, one row water, alternating.
3. While that propagates, the **top of the sand pile freezes / stalls in
   mid-air**.
4. Then the water comes out of the top and everything settles.

### Analysis of the new behaviour (reasoned from the fix — verify it)

The striping is a direct consequence of the `dst.moved()` refusal, and
follows from it exactly:

- Row `y`: sand displaces water; water lands at `y`, flagged moved.
- Row `y-1`: sand tries to move into `(x, y)`, sees `dst.moved()`, is
  **refused**, and stays put.
- Result within one frame: sand at `y-1`, water at `y`, sand at `y+1` —
  the stripe.
- Next frame the flag is cleared, the pair swaps, and the pattern ripples
  upward one row per frame.

The **mid-air stall** is the same mechanism seen from the sand's side:
the sand column cannot descend because the cell below it is flagged, and
nothing else moves it, so it hangs unsupported. That reads as floating
sand and is arguably a worse artifact than the original eruption.

**The underlying design gap:** displacement is a straight vertical swap.
Water displaced by a sinking blob has nowhere to go but *through* it.
Physically it should flow **around** the blob. Nothing in the engine
expresses that.

### Options for whoever picks this up (none tried)

1. **Sideways-preferring displacement.** When a dense cell displaces a
   lighter one, place the lighter cell in a nearby free-or-lighter cell
   to the side (up-left / up-right) in preference to the vacated cell
   directly above. Physically the right answer and kills both the
   striping and the stall. Touches shared `try_move` — highest risk,
   highest value.
2. **Let the refused mover fall anyway** when it has no support, so the
   column does not hang in mid-air. Fixes the worst visual (floating
   sand) without addressing the striping.
3. **Revert `abffff2` and accept the eruption.** Recorded only for
   completeness — the owner rated the eruption as clearly wrong, so this
   is a last resort.

Worth deciding explicitly whether the current state is better or worse
than before `abffff2`. The eruption was fast and wrong; the stripe is
slow and wrong, and adds floating sand.

---

## Standing methodology note

Every fix in this area that was judged by test output alone failed to
change what the owner saw. The two that worked were both driven by a
reproduction built from the owner's own description of the *initial
state*, and verified by measuring the exact quantity being complained
about (surface slope, cliff position vs seam, rows risen per frame).
Build the reproduction first, confirm it reproduces, and only then write
the fix.
