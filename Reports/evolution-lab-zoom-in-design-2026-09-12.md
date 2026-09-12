# Zooming in: what the 64 pixels of a cell should say

*Design lane T, evolution-lab round thirty, 2026-09-12. A report and one
instrument (`examples/zoomin.rs`); nothing in `src/` changes here. The owner's
brief: "explore making zooming in look better, smoothing or upsample the
resolution, or brainstorm better options." The card that puts the options in
front of the owner is `20260912T042033946Z-e6bcfa` on board `render`.*

**The short version.** Smoothing is the wrong answer here and the report says
why with a picture rather than an argument. The right frame is that at 8x a
cell owns 64 pixels and spends them saying one thing; the engine already
carries per-cell state that the 1:1 render can only *encode* — water fill as a
dimming, cracks as a whole-cell darken — and at magnification that state can
be *drawn*. That is more information and more crisp, not less, and it costs
about a nanosecond a pixel. Behind it, the one candidate that answers "big
sharp squares" directly is a material-aware sub-cell texture, and it belongs
as a mode in lane S's runtime filter selector, not as a replacement for
anything.

## 1. What zoom-in does today, and what is wrong with it

`Renderer::zoom` runs 1–8 (`MAX_ZOOM`) and is documented as a
nearest-neighbour magnify: `draw` maps every screen pixel to a cell with
`div_euclid(zoom)`, asks `cell_colour` for that cell, and passes the pixel's
offset inside its block as `sub`. **Exactly one thing reads `sub`** — the
crack strip (`CRACK_EDGE_DARKEN`), which draws a fissure along one edge of the
block instead of darkening the whole cell. Everything else is per cell, so at
8x every cell is a flat 8×8 square of one colour.

Rendered before theorising, with `labshot` on the shipped lab bed at 6,000
frames (`zoom=2/4/8 look=…`), and with the instrument on the plant scene:

| zoom | what the eye reads |
|---|---|
| **2x** | Indistinguishable in kind from play scale. `main.rs` already opens the window at twice the framebuffer, so "1x" is a 2×2 physical block per cell and 2x is 4×4; nothing about the picture changes but its size. Nothing to fix. |
| **4x** | Silhouettes stair-step — a diagonal twig is a run of offset squares. Soil begins to read as a mosaic of flat tiles rather than as grain. A part-full water row at the surface draws as a thin dark line. |
| **8x** | Every cell is a legible square. Soil is a checkerboard of flat brown tiles — the same thing the owner called "big sharp squares that look like giant white gray pixels" on the deep-rock card of 2026-08-21, at eight times the size. The plant is a staircase. The water surface is a black line one block tall (`CLAUDE.md`'s *dark or torn rows* metric trap, now a visible artifact rather than a measurement trap). Nothing inside a block says anything the block's colour did not already say. |

One cost fact that shapes everything below, from `draw`'s own contract: the
dirty-rect skip recomputes pixels for whole *touched chunks*. At 8x the
viewport is 64×40 cells, so **a single 64-cell chunk is larger than the
screen** — any touched cell on screen means a full 163,840-pixel redraw. At
8x the render skip is all-or-nothing, the frame is pixel-bound, and a
per-*cell* cost (the chunk fetch, the material lookup) is paid 64x less often
than at 1x while a per-*pixel* cost is paid exactly as often. So the number to
price a magnify option by is nanoseconds per output pixel, and that number does
not grow with zoom.

## 2. The option space

All rendered by `examples/zoomin.rs` on one scene: `common::PlantScene` at
6,000 frames with a stone basin let into the soil surface and filled with
water, settled 400 frames, sky pinned at noon. One 64×40-cell crop holds air,
plant (leaf and wood), soil, stone and a pool whose top row is part-full — 39
cells of it, printed so the level-line arm's work is countable. Every arm
starts from the shipped 1:1 frame and decides only *which* of those colours
each sub-pixel gets, so no arm can invent a palette the engine would not draw.

Costs are the arm's own reconstruction per output pixel, best of 9,
single-threaded, on this container, over a full 512×320 frame. **They are
upper bounds** on what the same rule adds to `cell_colour` (which already pays
the cell fetch the instrument repeats) and **they moved between runs** —
`nearest` read 5.0, 5.9 and 8.0 ns/px in three runs while other lanes built
— so read them as ranks relative to `nearest` inside one run, not as absolute
figures. Nothing here changes how *often* a redraw happens: every arm is a
pure function of the cell, its 3×3 neighbourhood and the pixel offset, so the
settled-world skip is untouched by all of them.

| | what the player sees at 8x | extra ns/px | what it breaks | mode in lane S's selector? |
|---|---|---|---|---|
| **A. nearest** (today) | flat squares | 0 | — | the default |
| **B. state drawn** (`drawn`) | a part-full water cell shows its fill as a *level line* — the bottom `fill` of the block in the undimmed colour, the rest in the air above. The dark surface line becomes a waterline with sky over it. | **~1**; fires on part-full *surface* cells only (39 of 2,560 in the crop) | nothing at 1x: with `zoom == 1` the level collapses to the same whole-cell dimming, byte-identical to today, the way the crack strip collapses to `CRACK_DARKEN`. Needs the colour of the cell above for the empty part of the block — one neighbour read on a surface cell. | **not a mode** — it is the crack strip's own shape (`sub`-aware per-cell drawing) and should be on whenever `zoom > 1` |
| **C. material texture** (`stamp`) | a mass is drawn at 8x the way it looks at 1x: each block is a field of `zoom/4`-pixel sub-grains, each picking its own palette entry by a world-keyed hash and scaled by the lighting the 1:1 render already put on the cell. Wood gets vertical bark strips keyed on the column; a leaf cell facing air on two sides of a corner gives that corner to the air, so lone leaves are lobes and a canopy edge is scalloped. Hard edges, exact palette, no blur. | **~24** on every redrawn pixel (`stamp+drawn` 29 against `nearest` 5, same run) — at 8x on this box ~4 ms single-threaded for a full redraw; `draw` is parallel over rows | the palette spread of every material was tuned for 1x, so a wide palette (stone) reads speckly at 8x and a narrow one (log, deliberately 18 units) stays flat — which is the same trade `dead-ends.md`'s `log.ron` entry records, now re-run at magnification. Nothing at 1x: sub-grain = cell. | **yes** — `flat` / `textured`, the mirror of lane S's `stride` / `coverage` / `average` on the minify side, with the sub-grain size a dial |
| **D. chamfered edges** (`contour`) | the staircase on a diagonal becomes a 45° edge: at each corner of a block, if the two orthogonal neighbours across it share a class (air / liquid / powder / solid / plant / creature) that is not the cell's own, the corner triangle is theirs. Convex corners are cut; the concave notch of a staircase is filled from the air side by the same rule. Chosen from what the cells *are*, never from colour — the whole difference from hqx/xBR. | **~17** (741 corners cut in the crop) | reads eight neighbours, so a neighbour across a chunk border can change without dirtying this chunk and leave a stale triangle until the chunk redraws (`FOAM_BLEND`'s doc records why the shipped path avoids neighbour reads; at 8x the chunk covers the screen so it is moot, at 2x it is real). And a *taste* cost: a lone leaf cell becomes an octagon and the notch rule speckles roots and canopies with diamonds — `notch=fill` badly, `notch=deep` (fill only where the diagonal cell agrees too) less, `notch=cut` (never fill) not at all but then a diagonal twig stays a staircase. | **yes**, as a third mode, with the notch rule a dial |
| **E. smoothing** (`smooth`) | bilinear interpolation between cell centres of the shipped colours — the first thing "upsample it" suggests | **~30**, four reads per pixel, the most expensive arm | destroys the per-cell grain that makes soil read as soil, softens every edge, and reads as a blurred photograph of the 1x picture. This is precisely the definition the owner rejected for zoom-*out* ("crisp"), arriving from the other side. | no — rejected below |
| **F. brightness noise** (`texture`) | as A plus a per-pixel brightness jitter keyed on world position, 8–15% | ~12 | at 8% invisible, at 15% reads as compression noise laid over the squares; adds no information. C is the version of "texture past some zoom" that carries meaning. | no — rejected below |
| **G. pixel-art upscalers** (hqx / xBR / EPX) | not rendered | — | already in `dead-ends.md`: they infer shape from colour, and the deliberate per-cell shade jitter means adjacent pixels rarely match. Backwards here for as long as palettes carry grain. | no |
| **H. field reconstruction** (`examples/subpixel.rs`) | plant tissue as a thresholded kernel field: smooth tapered strokes and lobes. Not re-rendered | 82 ns/px gated (`subpixel-rendering-2026-08-29.md` §11) | the owner has seen three rounds of it: "3d-ish", then "smooth circular shape/edges look fake", then "different but not clearly better" (card `20260829T090050407Z-b3bfd3`). D is its hard-edged, six-times-cheaper cousin and is the form of that idea worth a fourth look, if any. | it could be a mode; not proposed |

### What the sheet shows, in words the card cannot

- **B** changes one thing and it is the right one: the dark line across the
  top of the pool becomes a waterline. Subtle at contact-sheet size, obvious
  at the full-screen viewer, and it is the only arm that makes the 8x picture
  say something the 1x picture could not.
- **C** is the largest change and the only one under which soil at 8x looks
  like soil rather than a tile floor. The trunk reads as bark. It is also the
  arm most likely to divide opinion, because it changes the *character* of
  magnification from "the pixels, bigger" to "the material, closer".
- **D** does what it says on silhouettes and does something nobody asked for
  on single cells. It is worth a mode because the notch dial may find a
  setting the sheet did not, and because it costs nothing when off.
- **E** is the clearest picture on the sheet, in the wrong direction: the
  grain is gone and the tree is fog.

## 3. Recommendation, ranked

1. **Build B first: draw per-cell state at sub-cell resolution, starting
   with the liquid level.** It is the crack strip's own mechanism
   (`cell_colour` already takes `sub` for exactly this), it fires only on
   part-full surface cells, it is byte-identical at 1x, and it turns a known
   trap into a legible surface. Then the same shape for the other encoded
   state: a burning cell's flame as a strip at the top of the block flickering
   on the bucket `FLAME_FLICKER_PERIOD` already provides, rather than a whole
   tinted square; and, with the field overlay on, the `aux` support distance
   as a mark in the block rather than a tint over it. Each is a per-cell
   rule with a counter the way `filmstrip` prints `crumbled to grit`. *Not*
   a selector mode: there is nothing to select between.
2. **Then C, as a magnify filter mode in lane S's selector**, default
   `flat` (today) so nothing changes for anyone who does not press the key,
   `textured` behind it, the sub-grain size a dial (`zoom/4` is what the
   sheet used). Owner ruling: *for "does this look right", ship a runtime
   selector rather than choosing.* Its cost is the one to quote: ~24 ns/px
   over every redrawn pixel, which at 8x is every pixel of every frame with
   anything moving on screen.
3. **D as a third mode, `chamfered`, with the notch rule a dial**, because it
   is cheap when off and the sheet cannot settle whether a setting exists
   that fixes the twig without octagonising the leaf.
4. **Not E, not F.** Filed in `dead-ends.md` with the condition each
   rejection rests on.

The order is by *ratio of legibility gained to frame cost*, and B wins that
by two orders of magnitude before taste enters. The thing the card asks the
owner is whether C or D is a direction at all; B does not need the card.

### What the build lane would do in `src/render.rs`

Said here rather than done, because lane S is in the file.

- **B**: in `cell_colour`'s `is_liquid` branch, when `self.zoom > 1` and the
  cell is not full, compute `level = round(fill * zoom)` and, for
  `sub.1 < zoom - level`, return the colour of the cell above (an air cell's
  colour is the sky gradient at that position, which `cell_colour` computes
  from `(x, y)` alone — so no neighbour *cell* read is needed if the
  surface test is "the cell above is empty"). Below the level, skip the fill
  dimming. At `zoom == 1` the branch is never entered. Guard on the
  dispatch-site `Cell` and `mat.kind`, never on a name.
- **C**: a `MagnifyFilter` enum beside lane S's minify selector, on the same
  key family, held on `Renderer` and folded into `last_look` so a change
  forces the full redraw the way `grain` does. The textured branch keys
  `rng::jitter` on `(x * zoom + sub.0 - sub.0 % g, …)` and indexes the
  material's own palette, then scales the already-lit colour by the luminance
  ratio — the instrument's exact arithmetic — so the sky light, the fill
  dimming and the heat glow are all preserved.
- **D**: needs the 3×3 class neighbourhood; hoist it per cell the way
  `ChunkRun` hoists the chunk, and accept the chunk-border halo or widen
  `touched` by one cell ring at `zoom > 1`.

## 4. What was rejected, and why

Filed in `dead-ends.md` under **rendering**, each with its condition:

- **Bilinear smoothing at magnification** — costs the most of any arm and
  removes the grain and the edge. Holds while palettes carry a deliberate
  per-cell spread and while "crisp" is the owner's word for what a zoom
  should keep. If the art direction ever wanted a painted look, this is the
  cheapest way to get one and would be worth one more card.
- **Per-pixel brightness noise as "texture past some zoom"** — no information
  and no shape; the material-keyed sub-grain (C) is the version of the idea
  that carries meaning. Holds unconditionally: noise cannot say anything a
  hash of position did not already say.
- **The chamfer's `notch=fill` rule** — fills every one-cell hole in a canopy
  and every soil pocket between roots, which reads as diamonds. Recorded as a
  variant rather than as a rejection of D: `deep` and `cut` are the settings
  worth a dial.

## 5. What is not established

- **No owner verdict yet.** The card is posted and this report does not wait
  for it; the ranking above is the author's reading, and this repo's own
  record is that such readings are overturned by the owner's eye about one
  time in three. Read the verdict through `review.py get
  20260912T042033946Z-e6bcfa` before building C or D.
- **The costs are from a loaded container** and are ranks, not figures. The
  build lane should re-measure whichever mode it lands with
  `PIXEL_PHYSICS_DRAW_TIMING=1` on `render_cost`, paired against `flat` in
  the same run, at zoom 8 on a moving scene — at 8x that is the worst case
  by construction, because the skip cannot help.
- **The scene is one scene.** Fire, creatures, gas and the gnome are not in
  the crop; the instrument takes `look=` and `span=` so they can be.
- **B's "cell above" rule is the surface case only.** A part-full cell under
  a full one (mid-flow) keeps the whole-block undimmed colour in the
  instrument; whether that is right is a question for a moving GIF, which is
  the instrument this static sheet is not.
