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

It is disqualified twice — 7–8x the draw, and in the picture the plants very
nearly disappear into a smooth brown band, which is the complaint made worse
rather than fixed. It ships because the owner should get to reject the blur
by eye rather than take a lane's word for it, and because it is the proposal
the next person makes. Recorded in `Reports/dead-ends.md` with the condition
its rejection depends on.

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
