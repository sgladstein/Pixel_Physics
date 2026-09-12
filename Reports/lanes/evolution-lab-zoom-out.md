# Round 30, lane S — the widest zoom-out stopped dropping the thin things

*Owner, from play, 2026-09-12: "when I zoom out all the way, instead of
looking crisp, it looks like pixels of plants and other foreground things are
disappearing."* They were. `src/render.rs` is shared, so this was both games.

## What was wrong

The zoom-out was a **point sample**: `screen_to_world` maps a screen pixel to
one world cell and `ChunkRun::colour` drew that cell. At `MAX_ZOOM_OUT_STRIDE`
(4) that is one cell in sixteen reaching the screen, so a one-cell-wide stem
had three chances in four of falling between sampled columns and vanishing.

`MAX_ZOOM_OUT_STRIDE`'s own doc already stated the goal it was missing — the
cap exists so the view stays *"the same kind of picture, zoomed out" rather
than aliasing into noise*. Point sampling **is** that aliasing.

**The dropout had two passes, not one**, which the brief did not have.
`draw_particles` and the chunk-body pass are separate passes routing through
`world_to_screen`, which returned `None` for any position off the stride
lattice — so a grain in flight vanished 15 times in 16 independently of the
terrain behind it. Both are fixed in one change.

## What shipped

A runtime selector, `ZoomOutFilter`, on **`Shift`+`-`** (every letter on the
keyboard was already bound; hanging it off the zoom-out key is where you
reach for it anyway). Named on the status line whenever `zoom_out_stride > 1`
and silent otherwise — at stride 1 the filter cannot move a pixel, and at
stride > 1 it decides every pixel, so a zoomed-out screenshot that does not
name it cannot be reproduced.

- **`Coverage`** (the new default) — the most *salient* cell of the block, by
  `zoom_out_salience`: creature > plant > liquid > bulk ground > gas > air.
  Every pixel stays one real cell's own colour at its own position, so the
  picture is crisp rather than blurred.
- **`Stride`** — today's point sampling, kept as the control.
- **`Average`** — the area mean. Shipped to be rejected by eye, not by
  argument; see below.

**Salience is stated as data, never inferred from the palette** — it keys on
`MaterialKind`, the field the sweep itself dispatches on, so a material added
tomorrow gets a rank from its kind rather than from somebody remembering to
list it.

**`Solid` and `Powder` tie on purpose, and the tie is the thing that bounds
the risk.** Ties resolve to the block's top-left cell, which is exactly the
cell `Stride` drew — so a block entirely inside terrain, or entirely inside
sky, renders byte-identical to before, and **only blocks that genuinely mix
kinds move**. A world made only of stone and sand renders identically under
both filters, asserted.

`MAX_ZOOM_OUT_STRIDE` and `max_zoom_out_stride` are untouched. This changes
what a pixel *is*, not how many there are.

## The numbers

Lab bed 2048x1280, 48 founders, 4,000 frames, stride 4, the real 512x320
viewport, 8,890 living cells standing. Screen pixels on which a plant or an
ant reached the screen, measured by rendering the same world twice per filter
— once as it stands, once with every plant and creature cell erased:

| filter | live pixels | screen columns with life |
|---|---|---|
| `Stride` (before) | 548 | 189 / 512 |
| `Coverage` (after) | **1,638** | **323 / 512** |
| `Average` | 1,637 | 322 / 512 |

Two thirds of the pixels carrying the stand were being thrown away, and a
third of the columns that hold a plant drew as bare sky. `Stride`'s own count
is non-zero, so the probe proves its own sensitivity in the same run.

## The inverse artifact, which is the half a survival count cannot see

The coordinator asked for it before the card went out, and it is the right
question: a max filter's characteristic failure is not dropping thin things
but **over**-drawing them, and a stand that is a tenth plant rendering as a
hedge would be a worse lie than the dropout. So every kind is censused against
the area it actually occupies in the viewport. 1.00x is areally honest.

| lab, stride 4 | cells | true area, px | `Stride` | `Coverage` | `Average` |
|---|---|---|---|---|---|
| plant | 8,516 | 532 | 536 (**1.01x**) | 1,467 (2.76x) | 1,520 (2.86x) |
| creature | 374 | 23.4 | 12 (0.51x) | 171 (7.32x) | 169 (7.23x) |
| liquid | 328 | 20.5 | 10 (0.49x) | 168 (8.20x) | 203 (9.90x) |

| outdoor, stride 4 | cells | true area, px | `Stride` | `Coverage` | `Average` |
|---|---|---|---|---|---|
| plant | 648 | 40.5 | 32 (0.79x) | 166 (4.10x) | 166 (4.10x) |
| liquid (the sea) | 23,487 | 1,468 | 1,448 (**0.99x**) | 1,689 (1.15x) | 1,689 (1.15x) |

**Three findings, and the first two reframe the question.**

**The over-report is a property of the zoom, not of the filter.** `Coverage`
and `Average` agree to within a few percent on every kind (2.76 against 2.86,
7.32 against 7.23, 4.10 against 4.10, 1.15 against 1.15). Once a pixel stands
for sixteen cells, a block holding one stem either draws the stem at a whole
pixel's worth of ink or draws nothing; there is no third answer at this
resolution. So the coordinator's proposed **coverage-threshold** mode would
not buy areal honesty *and* survival — it is a dial back toward dropping
things, which is the bug. Not built, and this is why.

**The exaggeration is confined to exactly the things that were vanishing.**
The sea, at 23,487 cells, draws at 1.15x; plants at 2.76x; ants at 7.32x. Big
bodies are untouched and thin scattered ones are inflated, which is the
trade stated in one line. And the ratio is the wrong unit for how alarming it
is: an ant at 7.32x is **171 pixels of 163,840**, against 12 today — four
dots on the screen where there is currently almost never one.

**`Stride` is areally honest on plants and halves the rare clustered kinds.**
1.01x on plant says the old filter was never losing *ink*; it was putting it
in the wrong columns, which is why the headline is **134 of 512 columns losing
their plant entirely** rather than any pixel total. The 0.49–0.51x on ants and
water is clustering, not bias: a few hundred cells in a handful of clumps
either fall on the lattice together or miss together.

**The instrument's positive control is in the table.** The sea reading
**0.99x** under `Stride` is the case whose answer is known — a body 400 cells
across cannot be lost by any filter — so the ratio is shown to read 1.00 when
nothing is wrong, in the same run as the readings that do not.

**One measurement was wrong first, and the tell was that control.** The census
originally counted cells over the whole world against a viewport-sized
denominator, which on the 8192x2560 outdoor world put 145,170 sea cells
against a 2,621,440-cell viewport and reported the sea drawn at **0.16x**.
That is not a rendering finding, it is two numbers about two different
rectangles. `CLAUDE.md`'s *ask what your number counts* — and what caught it
was noticing a filter apparently losing five sixths of an ocean.

## The owner's verdict

Card `20260912T041559012Z-1b7300`, blind three-way, answered 2026-09-12:
*"A is best, B look blurry; C is worst"*. Decoded through `blind_was`
`[1, 2, 0]` — displayed A was `Coverage`, B was `Average`, C was `Stride`:

- **`Coverage` best** — the shipped default, confirmed.
- **`Average` blurry** — as the picture predicted.
- **`Stride` worst** — the current build is the worst of the three, which is
  the owner confirming the defect independently of the counts.

**One caveat on the middle finding, recorded rather than buried.** The card
was answered against its first text, which carried a steer of mine against the
blend (*"which I expect you to dislike"*). The coordinator had already ruled
that steer out on the owner's own *"I am open to different visual styles"*,
and the corrected, neutral text was pushed afterwards. So *"A is best"* and
*"C is worst"* stand clean, and *"B look blurry"* was given with a nudge in
front of it. If the visual-style question is reopened it wants a fresh,
unsteered card — and it is a bigger question than this PR.

**Frame cost.** The dirty-rect skip survives intact — `Coverage` is a pure
function of cell data, not an animated grain, so a settled world stays
skipped: **0 pixels recomputed and 0.09–0.10 ms under all three filters**.
The cost is confined to full redraws, i.e. while the camera is moving:

| filter | full redraw, lab / outdoor |
|---|---|
| `Stride` | 2.21 / 2.44 ms |
| `Coverage` | 4.65 / 5.42 ms |
| `Average` | 16.24 / 20.75 ms |

Arms interleaved inside one run, medians, `RAYON_NUM_THREADS` pinned. The
same lab arm read 2.05 and 5.91 ms in an earlier run, so **no cross-run
number here is worth anything** and none is quoted.

## Why `Average` ships and is not the default

It costs 3–4x `Coverage`'s draw and repaints 86% of the screen, and it is
areally indistinguishable from `Coverage` — it recovers the same life on the
same pixels and differs only in whether those pixels carry a real cell's
colour or a blend. That made it purely a question of taste, which is the
owner's to settle and not a lane's; he settled it *"B look blurry"*. It stays
as a live arm rather than being deleted, because it is the proposal the next
person makes and because the underlying taste question — *"I don't know if I
love the pixel aesthetic"* — is still open and larger than this change.
Recorded in `Reports/dead-ends.md` with the condition its rejection depends
on.

## The instrument

`examples/zoomfilter.rs` — three filters side by side on one world in one
binary, with the census, the paired timings and the settled-world arm printed
under each tile. Nothing already in `examples/` could answer this: `viewshot`
has a `stride=` but no filter and no census, and `labzoom` is one row per
*zoom step*, which is the settled argument this deliberately does not reopen.

**Two controls failed first, for one reason worth carrying.**
`Renderer::draw` is what builds `self.sky` and `self.daylight`, and
`cell_colour` reads both — so a hand-written comparison loop on a fresh
`Renderer` compares a lit frame against an unlit one and fails for the
*lighting* rather than for the sampling. Prime with one real `draw` first.

## Known limitation, deliberately not fixed here

**Picking still resolves to the block's anchor, not to the cell you can see.**
`screen_to_world` and `world_to_screen` remain exact inverses (asserted at
every stride under all three filters), and the anchor is the only cell a total
mapping can name — a pixel stands for sixteen cells and one of them has to
win. Under `Coverage` the cell *drawn* may be a different member of that
block, so at stride 4 a click can paint a neighbour of the thing under the
cursor. That was true before this change for fifteen cells in sixteen; it is
now noticeable because the sixteenth is visible. Fixing it means the inspector
and the brush asking the renderer which cell it drew, which is a second
change.

*Freshness: 2026-09-12.*
