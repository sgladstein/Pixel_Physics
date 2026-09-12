# Zooming in: what the 64 pixels of a cell should say

*Design lane T, evolution-lab round thirty, 2026-09-12. A report and one
instrument (`examples/zoomin.rs`); nothing in `src/` changes here. The owner's
brief: "explore making zooming in look better, smoothing or upsample the
resolution, or brainstorm better options." The card that puts the options in
front of the owner is `20260912T042033946Z-e6bcfa` on board `render`; the
styles card that followed the widened remit is `20260912T044316700Z-4abaa9`.*

**The remit widened mid-lane, and it changes the answer.** The brief said
*this is a pixel game and a proposal that blurs cell edges may fail the taste
that produced this request*. The coordinator withdrew that with the owner's
own words: *"I am open to different visual styles. I don't know if I love the
pixel aesthetic, even given the pixel simulation."* And the zoom-out "crisp"
was re-read from its source — *"instead of looking crisp, it looks like
pixels of plants and other foreground things are disappearing"* — so it names
**definition, the absence of things vanishing**, not blockiness. §0 carries
what that does to the rest; §2 and §3 were written before it and are kept as
the *filter* half of the answer, with the *style* half in §2b and the ranking
redone in §3.

**The short version.** Two questions, one entry point. As a *magnifier*, the
best thing to build is nearly free: at 8x a cell owns 64 pixels and spends
them saying one thing, while the engine already carries per-cell state the
1:1 render can only encode — draw it (a part-full water cell's fill as a level
line, ~1 ns/px). As a *style*, the simulation being cellular does not oblige
the renderer to look cellular: the same per-cell data supports a soft look, an
illustrated look with curved silhouettes and ink, a lit look and a textured
cell-art look, all rendered here at 8x and at play scale, all keeping the
render skip, at 30–48 ns/px. The card puts six of them in front of the owner;
he decides by eye. **The style question deserves its own round** — a look is
judged at play scale, in motion, with the gnome and the ants in it, and this
lane's sheet is a still of one crop — and §6 says what that round does first.

## 0. What the widened remit changes

- **Smoothing is no longer rejected on taste.** It stays the most expensive
  filter on the sheet and it still removes the per-cell grain; what it is not,
  any more, is disqualified for being soft. Its `dead-ends.md` entry is
  amended the same day: the cost half stands, the taste half is withdrawn,
  and the card is the re-test.
- **"Crisp" is a constraint on every style, not a vote for blocks.** Read
  as *nothing disappears*, it cuts against the soft look at play scale — at
  3x, bilinear thins a one-cell twig into haze, which is the zoom-out
  complaint arriving from the other side — and it cuts *for* the illustrated
  look, whose ink line makes every twig bolder than today. The sheet at 3x
  (§2b) is where that is visible.
- **The option space is styles, not filters**, and the way to get there
  without touching a simulation rule is to decide each sub-pixel's *class*
  from a field over the cells and its colour from the cells of that class.
  Everything in §2b is built that way.

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

## 2b. The styles — what the per-cell data supports without a simulation change

Same instrument, same scene, same crop at 8x and a full viewport at 3x (play
scale); card `20260912T044316700Z-4abaa9` on board `render`. A style here is a rule for
*which class a sub-pixel belongs to* — air, liquid, powder, solid, plant,
creature — computed as a bilinear field of class occupancy between cell
centres, with the colour taken from the nearest contributing cell of the
winning class. So a boundary is a curve through the lattice, an interior keeps
its grain, and no colour is invented (the `subpixel` report's §10a snap, done
with a 2×2 bilinear instead of a 5×5 kernel and six times cheaper). A mass
wins where its occupancy clears `level` (0.35 on the sheet; 0.5 is the
unbiased contour and shrinks a one-cell twig to a diamond — the bias is what
keeps thin things from disappearing).

| look | features | what the player sees | ns/px (8x / 3x) | what it costs beyond the pixel |
|---|---|---|---|---|
| **cell-art** (today) | `nearest` | flat squares; at play scale, the game as it is | 6 / 6 | — |
| **soft** | `smooth` | bilinear over the shipped colours. At 8x a blurred photograph of the 1x picture; at 3x the stand loses definition — thin twigs fade | 36 / 36 | the grain; and at play scale, thin things |
| **illustrated** | `iso+outline` | curved silhouettes, flat interiors with their grain, an ink line where a mass meets air. At 8x a cartoon; at 3x a bold-outlined illustration in which every twig is *more* present than today | 33 / 35 | two passes (class field, then colour); the ink dilates thin things by a pixel, which is also why nothing disappears |
| **painted** | `smooth+texture` | soft, with a fine grain laid back over it so a mass reads as material rather than fog | 48 / 45 | the same as soft plus a hash per pixel; still loses thin things |
| **lit** | `iso+lit+outline` | the illustrated look with edge shading from the field's own slope: leaves become beads, the pool gets a bevel | 42 / 43 | reads as rounded volume — the "3d-ish" the owner rejected for plants twice in the subpixel rounds, offered again because the remit is now open, not because that verdict moved |
| **textured cell-art** | `stamp+drawn` | the pixel look, kept, with each mass drawn at 8x the way it looked at 1x (sub-grain, bark, leaf lobes) and the water level drawn | 32 / 28 | the palette spread of every material was tuned at 1x |

Two things the 3x sheet says that the 8x sheet cannot:

- **Soft fails "crisp" at play scale.** The one-cell twigs that make a
  stand read as a stand go to haze under bilinear, which is the *disappearing*
  the owner complained of, produced by a different mechanism.
- **Illustrated passes it with margin**, and is the one look on the sheet
  that is plausibly a *different game* rather than the same game filtered:
  bold outlines, flat fills, exact palette. It is also the cheapest of the
  non-pixel looks.

The two columns come from one quiet pass (`reps=3`, both zooms back to back); the labels burned into the sheets are from a loaded run where `nearest` read 10, which is the rank-not-figure caveat of §2 made visible.

All six keep the dirty-rect skip: every one is a pure function of the cell,
its 3×3 neighbourhood and the pixel offset, so nothing forces a redraw on a
settled world. The style features read neighbours across chunk borders, the
same halo caveat as the chamfer in §2. And at 48 ns/px the most expensive look
is ~8 ms single-threaded for a full 512×320 redraw on this container, against
~5 ms for the whole 1:1 draw of the same scene — so on the owner's machine,
where `render_cost` records the full redraw at 12 ms for the shipped world, a
style is a real fraction of the frame on every frame something moves, and the
draw being parallel over rows is what makes it affordable.

## 3. Recommendation, ranked

Two lists, because the widened remit made them two questions.

**As a magnifier — build now, whatever the style verdict:**

1. **Draw per-cell state at sub-cell resolution, starting with the liquid
   level.** It is the crack strip's own mechanism (`cell_colour` already
   takes `sub` for exactly this), it fires only on part-full surface cells,
   it is byte-identical at 1x, and it turns a known trap into a legible
   surface. Then the same shape for a burning cell's flame (a strip at the
   top of the block on the bucket `FLAME_FLICKER_PERIOD` already provides)
   and, with the overlay on, the `aux` support distance as a mark rather
   than a tint. It survives every style below unchanged, because it decides
   what a block *says*, not how its edge is drawn.

**As a style — the owner's call, from the card, and then its own round:**

2. **Illustrated** (`iso+outline`) is the author's pick if one has to be
   named: it is the only look on the sheet that answers "crisp" *better*
   than today at play scale, it is the cheapest non-pixel look, it is exact
   palette by construction, and it is the flat-and-cartoony direction the
   owner asked for in the subpixel rounds without the rounded shading he
   rejected there.
3. **Textured cell-art** (`stamp+drawn`) if the verdict is that the pixel
   look stays: it answers "big sharp squares" at magnification and changes
   nothing at 1x.
4. **Soft / painted** only if the owner wants them despite the play-scale
   loss of thin things — and then with the `level`-style bias ported to the
   colour field so twigs are dilated before they are blurred.
5. **Lit** is on the card to be voted down deliberately rather than by
   default; the record says it loses.

Whichever wins ships as a **runtime selector** in lane S's shape — a
`MagnifyFilter`/`Look` enum on `Renderer`, default `cell-art`, folded into
`last_look` so a change forces a full redraw — with `level`, the outline
width and the sub-grain size as dials. Owner ruling: *stop balancing, start
exposing.*

The chamfer (§2, D) drops out of the ranking: `iso` does what it did on
silhouettes without octagonising single cells, at twice the cost and with no
notch rule to tune. It stays in the instrument as the cheaper hard-edged
fallback.

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
- **Illustrated / any `iso` look**: two passes per redrawn row — the class
  field (four cell reads per output pixel, a 2×2 that only changes at cell
  boundaries, so hoistable per cell the way the chunk is) then the colour,
  with the ink test reading the field `ow` pixels away. The field is a pure
  function of the 2×2 cells, so the skip's identity argument holds per
  chunk with the same one-cell halo as D. At `zoom == 1` the field is the
  cell grid and the look collapses to today's picture plus the outline —
  which is the one thing that would change the 1x game, and is exactly what
  the owner is being asked to judge on the 3x sheet.

## 4. What was rejected, and why

Filed in `dead-ends.md` under **rendering**, each with its condition:

- **Bilinear smoothing at magnification** — filed first as a rejection on
  cost and on taste; **the taste half was withdrawn the same day** when the
  remit widened (§0), and the entry now records the cost and the play-scale
  finding only: it is the most expensive filter, it removes the grain, and
  at 3x it thins one-cell twigs into haze. The card is the re-test; if the
  owner picks soft, the twig loss is the thing to fix before it ships.
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
- **The styles are one still each.** A look is judged at play scale in
  motion, with the gnome, the ants, fire and weather in it, and none of
  those are on the sheet. The 3x panes are the nearest this lane gets.
- **B's "cell above" rule is the surface case only.** A part-full cell under
  a full one (mid-flow) keeps the whole-block undimmed colour in the
  instrument; whether that is right is a question for a moving GIF, which is
  the instrument this static sheet is not.

## 6. The style question deserves its own round — and what it does first

The honest answer to the coordinator's last line is yes. A visual style is a
whole-game decision and this lane can only put a still of one crop in front
of the owner; the record here says stills have twice got a rejection where a
GIF got a diagnosis, and that creatures are seen only by moving. So, in
order, the round that follows the verdict:

1. **Take the owner's pick from the card and render it at play scale in
   motion** — `filmstrip gif=1` over a scene with the gnome walking, water
   pouring and a fire, through the same rule, before a line of `render.rs`
   changes. If it does not hold up moving, no filter setting will save it.
2. **Land it as a selector mode, default off**, beside lane S's minify
   selector, with `level`, outline width and sub-grain size as dials on the
   parameters page — the owner tunes the look in the game, which is the
   game.
3. **Only then re-derive what the look breaks**: the palette spreads were
   set for cell-art at 1x; the gnome sprite and the life marks are drawn
   *after* the world and would sit un-styled on a styled ground; the crack
   strip and the fill level are per-block rules that need restating on a
   curved boundary. Each is a known cost, none is a reason to wait.
