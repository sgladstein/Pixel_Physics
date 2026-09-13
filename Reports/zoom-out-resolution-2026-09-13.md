# What it costs to give the zoomed-out view its own pixels

*2026-09-13. Round 32, the zoom lane. Measurement only — no production code
changes with this report; the number is what decides whether the change is
worth building, and it had not been taken.*

## The question, and why it is a good one

The owner, 2026-09-13:

> *"why my screen resolution can solve all of the pixels, why cannot there just
> be more pixels when you zoom out?"*

He is right, and the constraint is not the monitor. The renderer draws into a
**fixed 512x320 back-buffer** (`WIDTH`/`HEIGHT`, `src/app.rs:202-203`), which
`Pixels::new(WIDTH, HEIGHT, surface)` (`src/main.rs:776`) then upscales to the
window. The screen's real pixels **magnify** that image; they do not carry more
of it.

So at `MAX_ZOOM_OUT_STRIDE = 4` (`src/render.rs:1925`) the view spans
**2048x1280 cells through a 512x320 buffer** — sixteen world cells per buffer
pixel, fifteen of which must be discarded. That discarding is the whole of
[§Z11](open-bugs-handoff.md) (closed 2026-09-12): `Stride` drew the block's
top-left cell and dropped the other fifteen, so a one-cell stem had three
chances in four of vanishing. `ZoomOutFilter::Coverage` answered *which of the
sixteen wins*. **Growing the buffer removes the question instead of answering
it.**

## The instrument

`examples/zoomout_pixels.rs`. Every arm draws the **same 2048x1280-cell span**
from the same camera and varies only how that span is divided between buffer
resolution and stride:

| arm | buffer | stride | cells reaching a pixel | discarded |
|---|---|---|---|---|
| ships | 512x320 | 4 | 163,840 | 2,457,600 (94%) |
| x2 | 1024x640 | 2 | 655,360 | 1,966,080 (75%) |
| x4 | 2048x1280 | 1 | 2,621,440 | 0 |

**Nothing already in `examples/` answers this**, and the two near misses are
worth naming because both look like they would. `subpixel_cost` grows the buffer
at `zoom > 1` — magnifying, so one cell is read once and painted `zoom²` times
and cell reads *fall* per pixel. `render_cost`'s `viewport_scaling` grows the
buffer at stride 1, so a bigger frame shows **more world** and the extra it
measures is content (its own note says the extra is cheap underground stone).
This holds the span fixed and trades stride against resolution, which has a cost
shape neither has: **`pixels x stride²` is constant across the arms, so total
cell reads are equal** and anything the bigger buffer costs is per-*pixel* work.

The span is asserted, not assumed, and the camera is **read back** from the
renderer rather than taken as requested — `set_camera` clamps to the world, and
the first run of this harness cropped 245 rows underground while its label said
"skyline". Arms are interleaved round-robin inside one run, because two
byte-identical runs have disagreed 2.42x on this box.

## The numbers

Median of 5–9, `RAYON_NUM_THREADS=4`, three separate runs agreeing. **Whole
frame** is `frame::step` + `Renderer::draw`, because a render-phase figure alone
is half a number.

**Outdoor**, 8192x2560 settled 600 frames, camera on the skyline:

| buffer | stride | draw ms | whole frame ms | vs ships | settled ms | skip repainted |
|---|---|---|---|---|---|---|
| 512x320 | 4 | 10.1–11.0 | 27.0–31.7 | 1.00x | 10.0–10.8 | 163,840 px |
| 1024x640 | 2 | 19.1–20.8 | 36.0–41.4 | **1.31–1.34x** | 19.3–20.2 | 655,360 px |
| 2048x1280 | 1 | 36.2–39.8 | 55.9–57.8 | **1.79–2.07x** | 35.5–38.8 | 2,621,440 px |

**Lab**, 2048x1280 bed, 48 founders, 3,000 frames:

| buffer | stride | draw ms | whole frame ms | vs ships | settled ms | skip repainted |
|---|---|---|---|---|---|---|
| 512x320 | 4 | 9.4–9.7 | 22.7–24.7 | 1.00x | 0.185–0.199 | 0 px |
| 1024x640 | 2 | 17.9–18.3 | 31.0–33.7 | **1.31–1.37x** | 0.186–0.214 | 0 px |
| 2048x1280 | 1 | 25.6–28.0 | 38.6–43.4 | **1.70–1.79x** | 0.184–0.205 | 0 px |

### Three findings, in order of how much they change the decision

**1. Sixteen times the pixels costs 1.8–2.1x the frame, not sixteen.** The
hypothesis in the round-32 brief — that the true cost is far below the pixel
ratio — holds, and by a wide margin. But **not for the reason the brief
proposed**. It is not the dirty-rect skip (see 3). It is that cell reads are
constant by construction and per-pixel colour work is a minority of the frame:
the *draw* rises 3.5x for 16x the pixels, and the simulation half — 13–21 ms,
untouched by any of this — dilutes that again.

**2. Half the benefit costs a third of the price.** The x2 arm carries **four
times** the cells for **1.31–1.37x** the frame. Going the rest of the way to one
cell per pixel buys 4x more again for another ~0.5x of frame. Whatever ships,
the x2 rung is the one that survives a cost argument.

**3. Outdoors the dirty-rect skip does not fire at all, and that is the number
to be suspicious of.** The `skip repainted` column is a tell placed in the
harness deliberately: with nothing touched, a settled screen should repaint ~0
pixels. The **lab does** — 0 px, 0.19 ms, and **flat across a 16x pixel range**
(1.05x), so at rest the change is genuinely free there. The **outdoor world
repaints every pixel in every arm**, so its "settled" column is a full redraw
wearing the name of a settled one. The sky is always moving outdoors, so at the
widest zoom-out the outdoor screen is never at rest and the full cost above is
paid every frame. `CLAUDE.md`'s warning — a change free in every moving scene
and ruinous at rest — is inverted here: it is the *moving* number that governs,
and the settled one that flatters.

## What the pictures say, which the numbers cannot

Two review cards, both `board=zoom`, posted 2026-09-13, verdicts pending:

- **lab**, `20260913T083914900Z-764956`
- **outdoor**, `20260913T083948135Z-0f4767`

Each shows the same patch of world at the same apparent size, the coarse arm
magnified back up exactly as the monitor already does it. They were posted
separately because **they do not obviously agree**, which is the finding:

- On the **lab bed** the finer buffer plainly wins. `Coverage` keeps every plant
  visible — §Z11 is genuinely closed — but it renders each one as 4x4 blocks, so
  a one-cell stem is drawn **four cells wide**. The dropout became an
  *exaggeration*. At one cell per pixel the stems get their real shape back.
- On **outdoor rock** it is not obvious at all. The per-cell shade jitter that
  gives stone its grain is drawn at 4x4 blocks today; at one cell per pixel that
  grain becomes fine enough to read as **smooth** rather than as texture. The
  finer arm is more correct and arguably less characterful, and the house style
  is chunky.

That second reading is why the change is worth costing before building and not
worth shipping on a default. It is a judgement only the owner can make.

## What to build, given the above

Sized from the numbers rather than from the brief:

1. **Grow the buffer only at zoom-out**, never at zoom 1 — normal play must pay
   nothing and must keep the chunky look, which is not in question.
2. **A runtime selector, defaulting to today's behaviour**, per `CLAUDE.md`'s
   *ship a runtime selector rather than choosing*: `off` / `x2` / `full`, named
   on screen with its cost. The verdicts above decide the default, not this
   report.
3. **Cap the buffer by the window's physical pixels.** This is the owner's own
   framing — *"my screen resolution can solve all of the pixels"* — and it is
   also the cost control: a buffer larger than the window is paying for pixels
   the display cannot show, and the GPU would only discard them again, which
   would reintroduce §Z11's dropout below the salience rule where nothing can
   answer it.
4. Zoom level and render stride have to **separate**. Today `zoom_out_stride` is
   both. Span must stay `512 x level` while the renderer samples at
   `level / scale`, so `scale` must divide `level` — at level 3 only 1 and 3 are
   expressible.

The one cost not in the tables: the HUD is drawn at fixed pixel coordinates
through 82 call sites in `src/app.rs`, so a grown buffer needs those to scale or
the text lands in a corner at a quarter size. That is the bulk of the
implementation work, and none of it is in the render path.
