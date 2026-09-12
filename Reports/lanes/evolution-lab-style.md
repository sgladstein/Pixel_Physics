# Lane V — the look the owner picked, as a mode he can play with

*Round thirty, 2026-09-12. Branch `claude/lab-style-painted-ink-r30`.
Coordinator `session_01NEcvi6sUugSuvmitBeeM4V`.*

## What landed

`MagnifyStyle` on `Renderer`, beside lane S's `ZoomOutFilter` and in the same
shape: a hoisted predicate (`magnifying()`), one match arm where a pixel's
colour is decided, `Shift`+`=` to cycle, and the status line naming the style
whenever it is not the default. Five modes — **cell-art** (the default, and
byte-identical to the build before this), **painted**, **painted+ink**,
**illustrated**, **chamfer** — with the ink weight (`Shift`+`]`), the
occupancy level, the grain amplitude and the chamfer's notch rule
(`Shift`+`[`) as dials.

`examples/magnify.rs` is the lane's instrument (`Reports/instruments.md` has
its row). `examples/labshot.rs` gained `style=`, which is the only way to see
a style on the lab bed — `src/bin/lab.rs` holds the lab's keys and another
lane holds that file.

Card **`20260912T080641206Z-926d32`** on board `render`: A/B frame sequences,
today against painted+ink, 3x, gnome walking, tree alight.

## The number that decided it

A one-cell **diagonal** twig reaching the screen, as a fraction of today's
look, worst of 8 (`magnify mode=twigs`):

| zoom | painted peak / area | painted+ink peak / area |
|---|---|---|
| 2x | 0.62 / 0.87 | 1.27 / 1.58 |
| 3x | 1.00 / 0.92 | 1.27 / 1.37 |
| 4x | 0.77 / 0.90 | 1.27 / 1.41 |
| 8x | 0.89 / 0.90 | 1.27 / 1.40 |

Painted+ink is **more** present than today at every zoom, never less. The
recovery splits at 4x (area, worst): painted 0.903 → class field alone 1.245 →
plus the ink 1.412.

Frame cost, 9 alternating reps inside one run, threads pinned, full redraw at
3x: today 2.77 ms, painted 5.44, painted+ink 6.90, illustrated 5.79, chamfer
3.39 — **+25.2 ns/px** for painted+ink against lane T's 48 ns estimate for
painted alone, because the 3x3 neighbourhood is hoisted per *cell*. Settled
world: **0 pixels recomputed under every style**, so the render skip holds.

## Four things worth carrying out of this lane

- **An odd zoom hides a bilinear loss.** At 3x the middle pixel of a block
  sits exactly on its cell centre, so plain bilinear hands a twig one
  full-strength pixel for free: peak reads 1.00 while a tenth of the twig has
  gone. A gate run only at play scale is blind by construction. Read an even
  zoom too, for any sub-cell reconstruction.
- **A thin-feature gate needs a *diagonal* feature.** A vertical stem is thin
  in one axis only and keeps 87.5% of its peak at 4x; the first version of the
  scene used one and reported that painted loses almost nothing.
- **The chamfer was deleting thin things**, and only the gate said so — 0.03
  of a twig at 2x. Filed in `dead-ends.md`, with the border-band and
  grain-over-sky reverts beside it.
- **`mixed_scene` contains no diagonal anywhere.** The chamfer is correctly
  inert on it, which read as a dead feature until the corners were counted:
  zero. Anything testing an edge rule needs a scene with an edge in it.

## What this lane did not do, and why

- **The water-level line** (`drawn`), which lane T recommends building
  whatever the style verdict. Out of this brief's scope; it is orthogonal to
  every style here and still worth building.
- **The material sub-grain** (`stamp`) — the owner said *"C-bad"*.
- **Smoothing as its own mode** — he judged it *"probably not worth extra
  cost"*. It is reachable as painted with `magnify_grain = 0.0`.
- **A key or a parameters-page row inside the lab.** `src/lab/ui.rs` and
  `src/bin/lab.rs` were off-limits this round. Whoever owns them: the fields
  are `magnify_style`, `magnify_ink`, `magnify_level`, `magnify_grain`,
  `magnify_notch`, and `cycle_magnify_style` / `cycle_magnify_notch` /
  `cycle_magnify_ink` are the steppers.

## Known costs of the look, none of them blocking

- **The crack strip does not draw under a style.** It is the one rule inside
  `cell_colour` that reads `sub`, and the styles take each neighbour's colour
  at `sub = (0, 0)`. A fissure along one edge of a *block* is a per-block rule
  that a curved boundary has to restate rather than inherit.
- **A one-cell hole in a mass becomes a diamond**, because the same occupancy
  bias that fattens a thin mass thins a thin gap. On the card as an explicit
  question. `magnify_level` is the dial; 0.5 is the unbiased contour and costs
  the twig.
- **A neighbour across a chunk border can change without dirtying this
  chunk**, so a silhouette on a seam can hold a stale ink pixel until the
  chunk redraws. Real at zoom 2, moot at zoom 8 where a chunk is most of the
  screen.
- **In the lab** the gain is *continuity* rather than contrast: at 6x the
  one-cell stems read as unbroken vines instead of a chain of offset blocks,
  and the soil keeps its grain. The bed is dark, so the ink subtracts rather
  than adds — it bounds the stem instead of brightening it.
