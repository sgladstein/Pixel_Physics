# Lane: zoom-out resolution (round 32)

*Closed 2026-09-13. Two PRs, both open, coordinator owns the merge.*

Distinct from `evolution-lab-zoom-out.md`, which is round 31's lane about how
*far* out the view may go. This one is about how many pixels the view is drawn
with once it gets there, and it deliberately reopens none of that.

## The two PRs, and the order

| | |
|---|---|
| **#385** `claude/zoom-out-resolution-measure` | The price. Instrument + report, no production change. |
| **#389** `claude/zoom-out-resolution` | The thing. **Contains #385**, merged in, identical content. |

**Either order works and there is nothing to reconcile** — whichever lands
first, the other's copy of those files is the same. #389 alone is sufficient;
#385 exists because the measurement was worth landing on its own while the
implementation was still being written.

## What it does

`App::pixel_budget` (`+` while zoomed out, x1 / x2 / x4, **default x1**) grows
the render buffer at zoom-out so more of the view reaches a pixel of its own.
The renderer draws into a fixed 512x320 back-buffer that the window magnifies,
so at the widest rung 94% of the cells in view could not reach a pixel at all.

`ZoomOutFilter::Coverage` (round 31) already stopped things *disappearing*. What
it could not stop is a one-cell stem being **drawn four cells wide**. That is
what this fixes.

## The numbers, if you are quoting them

Measured on the shipped path (`App::update` + `App::draw`, HUD included),
`zoomout_pixels game=app`, median of 11 at `RAYON_NUM_THREADS=4`:
**x2 costs 1.29x the whole frame, x4 costs 1.66x** — for four and sixteen times
the cells respectively. Sub-linear because `pixels x stride²` is constant, so
cell reads do not move and only per-pixel colour work does.

Quote the app figure, not the pre-build one (1.31-1.37x / 1.79-2.07x). Both are
in `Reports/zoom-out-resolution-2026-09-13.md` and the gap between them is
itself the point.

## What the coordinator should know

- **The default is off on purpose, and the verdicts decide it, not me.** Three
  cards on `board=zoom`, all pending: lab `20260913T083914900Z-764956`, outdoor
  `20260913T083948135Z-0f4767`, the real game `20260913T100843436Z-de27a0`.
  The lab plainly gains; **outdoor rock arguably loses its grain to smoothness**,
  which cuts against a house style that is chunky on purpose. If the owner likes
  it, flipping the default is a one-line change to `App::pixel_budget`'s
  initialiser plus the README and wiki lines that say it is off.
- **`bin/lab.rs` does not have this yet** and the lab is where the win is
  clearest. It has its own draw path and its own HUD (~35 call sites in
  `src/lab/ui.rs`), so it is a real piece of work rather than a switch — and
  `src/lab/ui.rs` is another lane's file, which is why this lane did not take
  it. That is the obvious follow-up.
- **`src/app.rs` is the contested file here, not `src/render.rs`** — 85 HUD call
  sites were rewritten to go through `render::Hud`. Mechanical, but anyone
  holding a large `app.rs` diff will feel it. Landed the same day it was written
  for that reason.
- **`PIXEL_PHYSICS_ZOOM_OUT=<rung>,<budget>`** starts the app zoomed out. Any
  later session needing to screenshot the real app at zoom-out wants this; the
  sandbox has no keyboard and without it the check has to go through a harness
  that reimplements the app.

## The one that cost the afternoon

The feature was correct in twelve guards and **panicked on the first frame of
the real app** — `index out of bounds: the len is 655360 but the index is
700428`, in a HUD blend a hundred lines from its cause. `main.rs` sized the
buffer from `App::viewport()` and `App::draw` pushed the budget at the renderer
afterwards, so the two disagreed for one frame after any change. Every test
passed through it because every test applied the budget before drawing.

Two sources of truth for one derived quantity, and the tests all happened to
observe them in the agreeing order. Fixed by deriving `viewport()` from the
authoritative pair so there is no ordering at all. Recorded here because the
*shape* recurs: a value pushed from an owner into a consumer, read back from the
consumer by a third party.
