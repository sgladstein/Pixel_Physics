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

### The shipped path, measured after it was built

The three tables above drive `Renderer::draw` directly, which is the right
instrument for pricing something that does not exist yet and is one call short
of the frame the player waits for. `zoomout_pixels game=app` closes that gap:
`App::update` + `App::draw` at each budget, HUD included, buffer sized exactly
as `main.rs` sizes it. Median of 11, `RAYON_NUM_THREADS=4`.

| budget | buffer | update ms | draw ms | whole frame | vs ships |
|---|---|---|---|---|---|
| x1 | 512x320 | 13.31 | 35.03 | 48.34 | 1.00x |
| x2 | 1024x640 | 13.40 | 48.79 | 62.19 | **1.29x** |
| x4 | 2048x1280 | 14.00 | 66.28 | 80.28 | **1.66x** |

**It comes in slightly cheaper than the pre-build estimate** (1.31-1.37x and
1.79-2.07x), and for the reason that should be expected rather than a happy
one: `App::draw` carries fixed cost the subsystem harness does not — the HUD,
the particles, the camera follow — so the growing part is a smaller share of a
bigger frame. The absolute figures are not comparable between the two tables
and the ratios are; this is the same shape as *an isolated harness overstates
what the app will see*, arriving at 1.66x where the isolated arm said 1.79x.

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

**Built, and it is these numbers that were used to size it.** Shipped
2026-09-13 as `App::pixel_budget` (`+` while zoomed out, x1 / x2 / x4, default
x1), with `Renderer::pixel_scale` spending it, `render::Hud` keeping the HUD at
its logical size, and `App::pixel_scale_cap` bounding it by the window. See
README's *Zoom-out resolution status*.

**What the live check caught, and no test did.** Run in the real app under
xvfb, it panicked at the widest rung: `index out of bounds: the len is 655360
but the index is 700428`, in a HUD blend a hundred lines from its cause.
`main.rs` sized the buffer from `App::viewport()` and `App::draw` pushed the
budget at the renderer *afterwards*, so for exactly one frame after any change
to the choice or the cap the two disagreed — a 512x320 buffer with the renderer
believing it was 1024x640. Every test passed through it, because every test
applied the budget before drawing. The fix is one derivation
(`Renderer::pixel_scale_for`, called by `viewport` with the authoritative pair)
so there is no ordering to get right, plus an assert in `draw` that the frame is
the size the caller claims. This is *verify live before declaring done* earning
its place again: the feature was correct in twelve guards and crashed on the
first frame of the real thing.

The one cost not in the tables: the HUD is drawn at fixed pixel coordinates
through 82 call sites in `src/app.rs`, so a grown buffer needs those to scale or
the text lands in a corner at a quarter size. That is the bulk of the
implementation work, and none of it is in the render path.


---

# Part two: the lab, and what the speed dial does to the answer

*Added 2026-09-13, after the sandbox half landed (#389) and the owner asked:
**"So will everything be consistent between the games once PR 392 ships? Does
this impact performance in the lab?"** The honest answer was no and no, and
this is the work that closes both.*

## Why the lab needed it more than the sandbox

`src/lab/mod.rs` re-exports the sandbox's `WIDTH`/`HEIGHT` and drives the same
`Renderer`, so **the lab had exactly the same 15-in-16 discard at the widest
rung** — and none of the fix. #392 flipping the sandbox default to x2 made the
games *diverge*, in the wrong direction: the lab is where the owner said the
finer buffer was best (card `20260913T083914900Z-764956`, *"C is best"* — x4,
decoded through `blind_was: [1, 0, 2]`), and it was the one that could not do it.

## The finding that matters, and it is the opposite of what was expected

The round-33 brief reasoned that the lab draws once per many ticks at the top of
the speed dial, so a render cost would be amortised away. **Measured, it is not
— and the direction reverses.**

`examples/labzoom_cost.rs` drives the real loop (`Lab::advance`, then `Lab::draw`
only when the advance says to draw) and reports **`achieved`: simulated seconds
per real second, render included** — the number on the dial, which is what the
player feels. Frame milliseconds are the wrong unit for a box you run fast and
glance at.

**A 1024x640 bed, 8 founders, one colony** (rung 2, so x2 and x4 both spend
scale 2):

| dial | x1 | x2 | x4 |
|---|---|---|---|
| 1 | 1.0x | **1.00x** | **1.00x** |
| 16 | 5.4x | 0.47x | 0.45x |
| 256 | 4.4x | 0.57x | 0.53x |
| 1024 | 3.5x | 0.65x | 0.64x |

**At dial 1 the bigger buffer is free. At fast-forward it costs a third to a
half of the achieved rate.** The reason is that `TimeControl` gives each pass a
wall-clock budget and the render comes out of it: at dial 1 the box has budget
to spare and the render fits in the slack, while at fast-forward every
millisecond spent drawing is a millisecond not spent ticking. The render is not
amortised by the dial — **it competes with it**, so it costs most exactly where
the brief expected it to cost least.

**A 2048x1280 bed, 48 founders, four colonies** (rung 4, so the budgets differ):
x2 reads **0.73-0.78x** and x4 **0.40-0.44x**, at every dial — this bed cannot
reach 1x real time at all, so the dial never gets to skip a draw and
`ticks/draw` is 1.0 throughout.

## The control, and it is the reason this ships on by default

**The shipped 512x320 bed cannot zoom out at all**, so the budget buys nothing
and costs nothing there: measured **1.00x** achieved at every budget and every
dial, and the harness prints *"the budget buys nothing at this rung"* beside it.
`max_zoom_out_stride` derives the cap from the world's own bounds, and a box no
bigger than the viewport has nothing to pull back from — so the rung is 1, no
power of two divides it, and the buffer never grows. `the_shipped_bed_spends_no_
pixel_budget` is the guard.

That is also the qualification the round-32 brief's claim needed. It said a
typical box would show *whole* at one cell per pixel; the true statement is
**boxes up to 2048x1280** do, because that is the widest span the ladder
reaches. `MAX_BOX` is 4096, so the top of the range does not.

## *"Even when pixels are off screen in the lab, they are being simulated, why
## does zooming out and making them visible affect performance?"*

The owner, 2026-09-13, and it is the sharpest question anyone has asked about
this. The answer in one line: **the simulation half does not move at all** —
those cells were always being stepped, visible or not — and what grows is the
*drawing*.

`Renderer::draw` does its per-**output-pixel** work once per buffer pixel: the
material colour, the depth shade, the per-cell grain, the sky lighting. At
512x320 that is 164k pixels of it; at 2048x1280 it is 2.6M. On top of that a
larger buffer is uploaded to the GPU each frame.

**And this is exactly why the whole-frame figure is 1.66x rather than 16x**, a
ratio that otherwise looks too good:

- the **simulation** is untouched — `frame::step` reads the same 13-14 ms at
  every budget in the sandbox measurement above, and the lab's tick cost does
  not move either;
- **cell reads are constant by construction** — `pixels x stride²` is the same
  product at every budget, which is the whole design of `zoomout_pixels`, so a
  bigger buffer re-reads no world at all;
- so the only quantity that grows is **per-pixel colour work**, which is a
  minority of a frame that also contains the simulation, the HUD and the
  particles.

The lab's answer has one extra term, and it is the one in the table above: the
render competes with the *tick budget*, so on a box at fast-forward the cost
shows up as fewer simulated seconds per real second rather than as a slower
frame. Same work, different unit.

## The default, and it is the owner's pick

**x4** (`lab::DEFAULT_PIXEL_BUDGET`), which is what he chose — **and the same
default as the sandbox, because consistency between the two games is a stated
requirement**:

> *"I am not sure what questions that I answered that suggests zoom should be
> different between the games, but that doesn't seem like what I want."*
> — 2026-09-13, on being told the app would default to x2 and the lab to x4.

**He is right, and the near-miss is worth recording because it is a flaw in the
cards rather than in his eye.** The two offered **different menus**: the lab
card was x1/x2/x4 and the real-game card was two panes, x1 against x2, so **x4
was never on offer in the game**. Reading *"x2 in the game, x4 in the lab"* off
that pair reads the construction of the cards and calls it a preference. Given
the full range, he picked the finest in both. **A comparison can only return a
verdict about the options it contains** — which is the review-queue form of
*ask what your number counts*, and it cost a wrong default in one game.

**What he has actually seen is x2, and that distinction matters.** The lab-bar
card (`20260913T170211133Z-af41c9`, *"I think this is fine. If there are other
better looking options, we can explore them"*) was rendered at the **1024x640
default window**, where `pixel_scale_cap` resolves a request of x4 down to x2 —
`blind_was: [1, 0]` puts the x2 arm in front of him as pane A. So: **the default
asks for x4, the cap gives x2 at that window, and x4 arrives only if the window
grows.** Saying "the lab ships at x4" as though his eye has backed it would be a
claim about a picture nobody has looked at.

**The same qualification applies to the consistency argument, in both
directions.** Both games now *request* the same budget; what either of them
*displays* depends on the window it is in. That is the honest position and it is
the right one — the alternative is two games that disagree by design — but a
sentence implying they are pixel-identical would be wrong.

**And a refused request now says so on screen.** The budget can be refused two
ways — the window is too small, or the zoom rung cannot divide it — and both
were silent, so a player who set x4 and saw x2 had no way to know which, or that
anything had been refused. A line under the clock names the request, what it
resolved to, and which of the two refused it. It is drawn only when the request
is *not* met. That is `time::PRESETS`' own principle applied to a second dial:
*"the dial is a request, and a request the machine cannot meet is how the
readout earns its keep."* Not on the bar, because row 0 measures 508 of 508
pixels at its tightest spacing and a widget there would overflow it.

The numbers above are why x4 is defensible rather than merely obedient:

- on the **shipped bed it is a no-op**, measured;
- on any bed that only reaches **rung 2, x4 and x2 are the same thing**, because
  the scale must divide the rung;
- it is **free at dial 1**, which is when you are actually looking at the box;
- it is **one key** (`+`) to step down, and the cost is named on the way past.

What it does cost is a **large bed at fast-forward: roughly half the achieved
rate**. That is stated here, in the README, and in the PR rather than left for
someone to find.

## Two bugs, both the same shape, and the second was found by a number that
## could not be true

The sandbox half shipped a panic caused by **two sources of truth for one
derived quantity** — `viewport()` read the renderer's pushed copy while `draw`
pushed it afterwards. The lab half found the same shape again, and this time it
was silent:

`zoom_within` derived the scale from `self.pixel_scale()` while its `viewport`
argument came from the caller's authoritative pair. For as long as a rung change
was in flight the two described different frames, and the visible effect was
that **a bed reaching rung 4 at budget 1 stopped at rung 2 at budget 2**. The
tell was a measurement that could not be true: *the bigger buffer ran faster*
(1.09-1.24x). It was not faster — it was showing half as much world.

**The fix was to delete the ambiguity rather than order the operations.** The
camera is about *cells*, so it now takes the **logical** viewport and multiplies
by the ladder stride; nothing in `follow`, `pan`, `set_camera` or `zoom_within`
touches `pixel_scale` at all. `visible_span`'s contract changed with it (logical
viewport, ladder stride) — the same number as before whenever buffer and scale
agree, and still right when they do not. `draw`, the one caller that genuinely
holds a buffer, divides by the scale on the way in.

**Its guard had to be written twice.** The obvious version, asserting on
`Renderer` that the rung is budget-independent, **passed with the fault
reinstated** — blind, because the fault needs a caller whose viewport is derived
from the budget. The one that works is at `Lab` level and pushes the budget at
the renderer first, because that is the order the app runs in; it fails with
*"budget x2 reached rung 3, not 4"*. `CLAUDE.md`'s rule earned its place twice
over here: the first guard was replaced, not widened.

## The other half of the lab, which is not in the renderer

The bar is interactive, which the sandbox's HUD is not. The cursor arrives in
**buffer** pixels and the bar is laid out in **logical** ones, so at x4 every
button would be four screens off. The conversion happens once, at the window
boundary (`Lab::to_logical`), and everything downstream — hit tests, hover
explanations, `press`/`drag`, and the world lookups through the new
`Renderer::logical_to_world` — works in logical pixels. The rack thumbnails are
pinned to budget 1 for their own off-screen draw, since they are not the window.
