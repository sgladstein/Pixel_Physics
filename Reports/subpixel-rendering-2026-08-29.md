# The framebuffer does not have to be the cell grid

Written against the owner's brief, 2026-08-29: *"it is hard to get [plants] to
look good with our pixel graphics... I don't want to increase the simulation
resolution but is there any way to get past this. Upsampling or a more
aggressive filter."*

The answer is yes, and the reason it is yes is smaller than it sounds: **the
extra pixels are already on the screen and are currently spent on
replication.** This report is the measurement, the prototype, and the two
things the prototype got wrong on the way — both of which are the same class of
bug and would have been shipped as features.

**Not to be confused with the other resolution work.** `worldgen-resolution`
(PR #112, landed the same day) makes the *simulation* cell resolution
changeable — `WorldgenParams::scaled(k)`, so the same seed at twice the cells
is the same world. This changes the **render** lattice and leaves the
simulation's alone; the two are complementary and neither supersedes the
other. If you arrived here grepping for "resolution", check which one you
want: more cells, or more pixels per cell.

## 1. Why plants specifically, and not everything

The complaint singles out plants, and that is not taste — it is structural.

Everything else in this world is a **mass**. Rock, soil, water and sand are
drawn as large contiguous regions, and a square lattice renders a mass
perfectly well: the per-cell shade jitter reads as *grain*, which is why
`wood.ron` and `soil.ron` carry a spread of four tones at all.

A plant is a **line**. `plant-appearance-design.md` §1 measured the stand as
~90% `MatureBody` with foliage a one-cell skin at ~5%, and by eye a crown is
one- to three-cell strands. A one-pixel line on a square lattice with no
sub-pixel coverage is a staircase — no taper, no curve, no continuity — and a
lone leaf cell is a hard square. So the same renderer that flatters rock is the
worst possible renderer for a twig, and no amount of per-cell work fixes it,
because **the cell has no interior to draw into**.

That is also why Phase 2's three architectural levers moved nothing
(`plant-appearance-design.md` §5): they change *which cell gets a label*. This
is one level below that — it changes what a cell *is drawn as*.

## 2. The headroom is already paid for

`main.rs` opens the window at `LogicalSize::new(WIDTH * 2, HEIGHT * 2)` against
a `Pixels::new(WIDTH, HEIGHT, ..)` framebuffer. So **every world cell already
occupies at least a 2x2 block of physical screen pixels and all four are
byte-identical**, and more than that on a HiDPI display. The GPU is
nearest-neighbour-replicating a 512x320 buffer into a 1024x640 window.

Rendering into a 1024x640 buffer at 2 pixels per cell shows the **same world
region** at the **same window size** with four times the information. Nothing
about the simulation, the play scale, or how much world is on screen changes.

`Renderer` needs no new concept for this either: `zoom` already means "screen
pixels per cell", `screen_to_world` and `sub_cell` already do the mapping, and
`cell_colour` already takes the sub-cell offset — with exactly one thing
reading it today (the crack strip, `CRACK_EDGE_DARKEN`). The machinery is built
and idle at 1:1.

## 3. What it costs — measured, and it is not what it looks like

`examples/subpixel_cost.rs`. Same world, same camera, same visible region
(asserted, not assumed — a supersample that quietly shows a different amount of
world would look exactly like a result), drawn into progressively finer output
lattices. Best-of-10, mirroring `render_cost`'s own `best_of` so the two are
comparable.

| px/cell | buffer | pixels | ms | vs 1x |
|---|---|---|---|---|
| 1 | 512x320 | 163,840 | 59.78 | 1.00x |
| 2 | 1024x640 | 655,360 | 67.30 | **1.13x** |
| 3 | 1536x960 | 1,474,560 | 78.63 | **1.32x** |

**Four times the pixels costs 13% more; nine times costs 32% more.** That is a
tidy result, which `CLAUDE.md` says to distrust — so here is the mechanism that
makes it true rather than an artifact. `render_cost` on the same box reports a
59.86 ms full redraw of the generated world, and separately reports that
pushing all 163,840 pixels through one branch of `cell_colour` costs 2.3 ms
(stone) to 6.8 ms (sky). **The per-pixel work is therefore under 10% of the
redraw**; the rest is per-*draw* setup — the sky-light grid, the horizon
rebuild, the glow-tile scan over every field tile — and a finer output lattice
does not repeat any of it. The measured increment, 18.9 ms for 1.31M extra
pixels, is 14 ns/px, which is `render_cost`'s own all-stone figure to the
digit. The number is explained, monotone across three points, and consistent
with an independently-measured constant.

Two honest caveats. **This box is slow**: `render_cost`'s doc records 12.07 ms
for the same full redraw on the owner's machine against 59.86 ms here, so the
absolute numbers do not transfer and only the ratios should be quoted. And the
fixed/variable split is what sets the ratio, so on a machine where the fixed
part is proportionally smaller the penalty would be proportionally larger.

**The dirty-rect skip is untouched.** This changes what a full redraw costs, not
how often one happens.

## 4. Why not hqx / xBR / a post-hoc filter

The obvious reading of "upsample it" is a pixel-art scaler run over the
finished frame. Those infer shape from **colour** — which neighbouring pixels
happen to match — and this world is their worst case: the deliberate per-cell
shade jitter means adjacent pixels of the same material rarely match, so the
filter has nothing to latch onto. It would smear the grain, which is the one
thing that is working, and leave the plants alone, which is the thing that is
not. Backwards on both counts.

The renderer knows something a filter cannot recover: **what each cell is**.
Reconstructing from material and organism cell type rather than from RGB is
what makes this work at all.

## 5. The prototype: `examples/subpixel.rs`

Plant tissue is resampled as a continuous scalar field — a compact SPH-style
kernel `(1 - d²/r²)²` summed over the 5x5 neighbourhood — and thresholded with
an antialiased edge. Colours come from the shipped `Renderer` at 1:1, so
nothing here can invent a colour the engine would not have drawn; the
background behind an eroded plant pixel comes from a second pass of the same
renderer over the same world with plant cells emptied, so the sky gradient,
skyline and ground stay exactly right rather than being inpainted.

Four things came out of it, in the order they were found.

### 5a. Shape and colour are separate questions

The first pass weighted the colour by the same kernel that decides the shape —
a 5x5 blur of the palette. The silhouette came out smooth and the whole plant
came out soft-focus, and **the soft focus is what reads as "worse" even where
the silhouette is plainly better**. Shape wants a wide smooth kernel; colour
does not have to use it. `colour_blend` separates them.

### 5b. Foliage and wood are two layers, not one field

With one field over both, the nearest-colour rule renders the brown/green
boundary as a hard square mosaic *inside* a smooth outline — visibly worse than
the staircase it replaced. Splitting them is also the physically true reading:
a leaf hangs in front of the twig it grows off, so the twig should pass
**behind** the crown rather than tile with it.

### 5c. The field gives a thickness and a normal for free — and this is the
part worth more than the smoothing

`cov` sits at the threshold exactly on the outline and climbs with how much
tissue is stacked around the point, so `cov - level` is an **ambient occlusion**
term, already computed. And `grad(cov)` is analytic for a sum of kernels and
points into the mass, so its negation is an outward **surface normal** at
sub-cell resolution.

A crown lit uniformly across its whole area is a large part of why foliage here
reads as confetti rather than as volume, and **no per-cell colour work can fix
it, because a cell has no interior and a one-pixel branch has no side facing
anywhere**. These two terms give a canopy a dark interior and a lit rim, and
round every lobe and every branch, off numbers the reconstruction already had.

### 5d. Both of those terms quilted the interior into squares, twice, for two
different reasons

This is the finding worth carrying, because both bugs are the *same* bug in
different costumes: **an operation that is well-defined at a boundary was
applied in an interior where its input is lattice noise.**

- **Normalising the gradient.** Deep inside a mass the kernel gradients from
  the surrounding cells cancel, so `grad` is lattice-scale residue — and
  dividing it by its own tiny length turns that residue into a full-strength
  unit normal. Fixed by not normalising: the raw gradient is already the right
  shape, large at the rim where the surface genuinely faces somewhere and
  vanishing in the interior where it genuinely does not.
- **The raw kernel sum.** A kernel sum over a *regular* lattice ripples at the
  lattice period unless the kernel is far wider than the spacing, so deep
  inside a solid mass — where the answer should be a flat "completely full" —
  the sum oscillates between cell centres and cell corners. Read as a thickness,
  that ripple is amplified straight into a grid of squares. Fixed by dividing
  by what the kernel sums to when every cell in range is tissue
  (`partition_table`), which carries the same ripple so it cancels — and which
  depends only on where the pixel sits inside its cell, so it is a
  `scale x scale` table computed once, not per-pixel work.

Both were found the same way and by nothing else: **render with the terms off,
and the interior is smooth**. A guard over "does the reconstruction fire" would
have been green through both.

### 5e. The foliage radius reaches the one thing the appearance report said no
lever could

`plant-appearance-design.md` §2.1 is titled *"Foliage is 5% of the plant"* and
says of it: *"No architectural lever can reach this."* That is true of every
lever *inside the simulation* — the 5% was bought deliberately, `shade_death:
0.03` traded crown volume for bole legibility, and §5a's `leaf_cluster` sweep
paid 20% of the stand's mass to move foliage share from 7% to 11%.

**A render-side radius reaches it directly.** `leaf_r` is how much of the
silhouette one leaf cell paints, and raising it from 1.3 to 1.75 fills the
crowns out visibly on the identical stand — same 7,470 leaf cells, same 7,387
wood cells, same economy, same light field, no simulation change of any kind.
Foliage share as a *cell count* is unchanged; foliage share as **screen area**,
which is the quantity the complaint was ever about, is not.

This is worth stating carefully, because it is a knob that can be turned too
far: it makes foliage cover more area, it does not make there be more foliage,
and a crown of five leaves drawn fat is still a crown of five leaves. It also
cannot deposit canopy density — the optical cost §5a priced — so the plant's
own self-shading is untouched by it. What it buys is that the leaves the plant
does have stop being invisible against the twig they hang on.

## 6. Terrain wants the silhouette and not the interior

`arm=all` puts rock and soil through the same reconstruction. With colour
blending on, **the soil grain is destroyed** — the bed becomes a flat brown
field, and the grain is the entire reason soil reads as soil. With
`colour_blend: 0` the grain returns intact while the silhouette is still
smoothed.

So the rule is class-dependent, and it is short:

| | silhouette | interior colour |
|---|---|---|
| plants, roots, thin structures | reconstructed | blended |
| rock, soil, sand — masses | reconstructed | **per-cell, untouched** |

Roots, which are thin structures inside a mass, come out visibly better under
the same treatment as branches — which is the rule doing its job rather than a
coincidence.

## 7. What is not established

- **The reconstruction's own cost is still not shippable**, though one of the
  three optimisations is now measured rather than asserted — see §11.
- **The look is under revision, not settled.** See §9 — the first tuning was
  rejected and the direction it was rejected *toward* is recorded there.
  Nothing should reach the shipped renderer before that lands.
- **Nothing was changed in `src/`.** This is an instrument and a measurement.
- The parameters (`wood_r 1.1`, `leaf_r 1.3`, `level 0.30`, `band 0.10`,
  `blend 0.75`, `ao 0.45`, `shade 0.40`) were set **by eye on one stand**, not
  swept, and outcomes here have enormous spread. They are a demonstration, not
  a calibration.

## 8. The adjacent lever this turned up, not pursued

`TreeDepth::Weave` already assigns each tree a binary depth relative to the
gnome, stable for its life, and `TreeDepth::Haze` already dims the layer
behind. That is an existing, shipped, per-tree depth bit that nothing else
reads. Applying atmospheric perspective to it — the back layer slightly paler
and lower-contrast — would separate a stand into two planes for free, and a
stand reading as one flat hedge is the other half of the complaint
`plant-appearance-design.md` §5a was chasing with `leaf_cluster`. Render-only,
no simulation cost, not built.


## 9. The owner's verdict, and what it rules out

Card `20260829T080553450Z-675375`, board `plants`, blind A/B of the shipped
1:1 render against the reconstruction, answered within five minutes. Decoded
through `blind_was: [1, 0]`, so the displayed "Option A" is the **stored B**,
the reconstruction — the owner identified the new arm correctly:

> *"Option A is obviously new. I don't like how it looks but maybe we just need
> to tweak it more, not give up. The edges between color or material look
> weird, kinda 3d-ish. Could it be more flat or cartoony"*

**This is a verdict on §5c, not on the reconstruction.** The ambient-occlusion
and surface-normal terms are precisely what put a lit side and a shaded side on
every lobe, and "reads as rounded volume" is what they are *for* — so the
complaint lands exactly on the mechanism this report was most pleased with, and
lands on nothing else. The silhouette work, the layer split and the resolution
decoupling are all untouched by it.

Worth stating plainly, because the pull to defend the clever part is strong:
**deriving a normal and a thickness from the field is a real capability and it
is the wrong art direction for this game.** Both can be true. The capability
stays available for anything that does want volume (a boulder, a body of
water); it is off for plants.

The direction asked for — *flat or cartoony* — has an implementation on the
same field and it needs no new state. `cov` is an occupancy fraction, so a
**drawn edge** is just the band immediately above the threshold: darken there,
fill flat inside, no edge detection and no second pass (`outline` /
`outline_width` in the prototype). Flat is `ao: 0`, `shade: 0`,
`colour_blend: 1.0`, a narrower `band`. Card posted as the follow-up; the
answer to *"is the problem the smoothing itself or how far it goes"* is the
thing still genuinely open.

**And note what the round trip cost**: one card, five minutes, and it
overturned the part of this work that no test, no counter and no amount of
reasoning here would have questioned — the shading fired, the numbers were
right, and it was wrong anyway. That is the fourth entry in this repo's tally
of models overturned only by the owner's eye.


## 10. Second round: flat is right, and the metaball's own tell

Card `20260829T085116684Z-232cd7` — the flat arm against the rejected rounded
one, **not blinded**, so the prose reads directly and `choice_label` agrees
with it:

> *"2 is better than 1. Not saying it is better than the current
> implementation. Continue working on 2. Even flatter, there are still 3dish
> looking shading. Also could the edges look more rough, the smooth circular
> shape/edges look fake"*

Three things in that, and the middle sentence is the one to keep in view:
**the bar is the shipped 1:1 render, not the previous attempt.** A/B against
your own last iteration measures progress and can run for ever without
clearing the thing it started from.

### 10a. What was still shading, after the shading was removed

`ao` and `shade` were already off, so the remaining gradient was the
**kernel-weighted colour mean** — smooth wherever the contributing cells
differ, and a smooth gradient across a shape is exactly what reads as a lit
curved surface. It was not modelled as shading and it looked like shading,
which is the whole of the note.

**Quantising the RGB channels independently was the first fix and it wrecks
hue**: at five steps per channel the browns went maroon and the darker tones
went to flat black, because a uniform ladder in RGB does not respect where a
palette's colours actually sit. Recorded in `dead-ends.md`.

What works is snapping each fill to the nearest colour **that actually occurs
in the cells under it**. It can only ever emit a colour the engine already
drew, so the palette is exact by construction, and — the part that matters —
the region boundaries follow the *smooth field* rather than the cell grid.
That is what separates it from `colour_blend: 0.0`, which also emits exact
palette colours but lays them out in a nearest-cell-centre partition, and on a
lattice that partition is squares. Same colours, same flatness, and one of them
is the mosaic of §5b.

### 10b. "The smooth circular shape/edges look fake" is a correct reading of
the mechanism

This is the sharpest note of the three, because it names an artifact of the
method rather than a tuning error. **A sum of radial kernels thresholded at a
constant level can only produce circular arcs and smooth joins between them.**
An isolated cell is a disc; a clump is a run of fused discs. It is legible, it
is organic-ish, and it is unmistakably synthetic once seen — soap bubbles.

The fix is not more geometry. **The threshold does not have to be a constant.**
Perturbing it with coherent value noise makes the same field cut a ragged
outline at no extra kernel work — one noise sample per pixel, shared by every
layer so a branch and the leaf in front of it are cut by the same wander and
their seam does not read as a third object. Keyed to **world** position, never
screen position, for the reason `rng::jitter` is: a threshold keyed to the
screen crawls over the terrain whenever the camera moves.

Foliage is the case that wants it most — a leaf edge is serrated and a canopy
edge is a thousand leaf tips, and neither is an arc.

Posted as card `20260829T090127…` (blind, board `plants`) against the shipped
1:1 render, which is the comparison the second sentence of the verdict asked
for.


## 11. One optimisation measured, two still asserted

The first draft of §7 said the 139 ns/px figure was "an unoptimised upper
bound" and named three optimisations, **none of them measured** — which is a
promissory note, and this repo's own rule is that a projection is not a
measurement.

The first of the three is now real. **Only cells with tissue inside the
kernel's reach can change**, and everything else is background whose answer is
already known. Dilating the plant classes by the scan's own two cells and
gating on that:

| | ms (serial, 1536x960) | ns/px | cells scanned |
|---|---|---|---|
| gate off | 254.87 | 172.8 | 163,840 (100%) |
| **gate on** | **121.65** | **82.5** | 29,944 (**18.3%**) |

**2.1x, and the two frames are byte-identical** (`md5 0e37cb30…` both ways).
That control is the point, not a formality: `CLAUDE.md`'s §*A cost that
vanishes may be work that vanished* is exactly the failure available here — a
gate that quietly excludes real work is a missing feature that times like a
speedup. `gate=false` is a switch on the harness rather than a constant, so
the pairing can be re-run on one command whenever the parameters move.

Note the speedup is 2.1x where the cell count falls 5.5x, because a gated-out
pixel still pays its loop iteration and its background sample, and building the
mask costs a 5x5 scan per cell. That gap is the honest shape of the result and
is why it was worth measuring rather than deriving.

**The two that remain are still unmeasured, and should not be quoted as
numbers**: the draw this would live inside is already parallel over rows, and
the per-cell 5x5 class scan is currently redone by every sub-pixel of every
layer — at `scale` 3 that is nine sub-pixels x two layers x 25 cells = 450 cell
tests per cell, where hoisting the contributing list would make it 25 once plus
a handful of kernel evaluations each. Both are standard and both look large.
Neither has been run.

For scale: `render_cost` puts the renderer's own per-pixel colour work at
~14 ns/px on this box, so at 82.5 ns/px the reconstruction is still about six
times the cost of drawing the pixel it decorates. That is the number the
remaining work has to move, and **it is a reason to get the verdict on the
look first** — optimising a picture nobody has approved is the mistake
`CLAUDE.md` calls "check that a planned step can demonstrate itself".


## 12. Third round: not clearly better, and what that costs the approach

Card `20260829T090050407Z-b3bfd3` — the flat, rough-edged, exact-palette
reconstruction against the shipped 1:1 render, which is the comparison §10's
verdict asked for. Answered with no pane selected:

> *"It is different but not clearly better"*

**That is the bar not cleared.** Three rounds now read, in order: rejected as
3-D-ish; better than the rejected one but *"not saying it is better than the
current implementation"*; and different but not clearly better. The trend is
flat, and `CLAUDE.md` is explicit about what a trend like that means —
*"two fixes failing the same way means the approach is wrong, not the
tuning."* A fourth tuning pass is not the move.

### 12a. But one rider was constant across all three, and that is the other rule

Its sibling: *"when every setting of a sweep fails the same way, suspect the
sweep"* — before condemning an approach, run the control that isolates it.

Every arm put in front of the owner held `leaf_r` at 1.3, so **foliage painted
roughly its own cell area in all three**. Every image was therefore the same
brown-twig-dominated tree with differently-drawn twigs: rounded twigs, flat
twigs, ragged flat twigs. The composition — a stand that is ~90% wood by cell
count and reads as brown wire with green speckle — never varied, and it is
precisely what `plant-appearance-design.md` §1 identifies as the thing that
sets the silhouette.

That is the rider, and §5e already showed the lever that moves it. Combining
them was never put in front of anyone. Posted as the isolating control, with
the question framed so that a fourth *no* is still information: **is it the
drawing, or is it the tree?**

### 12b. What a second "no" would establish

Worth writing down before the answer arrives, so it is not reasoned backwards
from afterwards. If a foliage-dominant crown also reads as not clearly better,
then the honest conclusion is that **rendering is not what is wrong with the
plants** — the sub-cell reconstruction is a real capability that does not
address this complaint, and the work to do is on what the growth model
produces rather than on how it is drawn.

The architectural finding in §2 and §3 survives either verdict, because it is
not about plants: the pixels are already on the screen and cost 13% more to
spend. What would fall is only the claim that spending them on *this* fixes
the plants.


## 13. Stop tuning: three approaches that differ in *what* they change

The control of §12a came back **"I need to see a timelapse of it growing"** —
a redirect rather than a verdict, and a fair one: a still cannot answer whether
a stand looks right, and the review skill says the same in its own words. Asked
at the same time for three approaches to compare.

`examples/plantlook.rs` is that. It is deliberately **not three tunings** of
the reconstruction — §12 established that trend is flat — so the three differ
in what they change, and each is a distinct, falsifiable bet about why plants
look wrong:

| arm | shape primitive | the bet |
|---|---|---|
| `shipped` | one cell, one square | the control |
| `masses` | **a whole crown region** | the primitive is too *small*: a tree's silhouette is a few overlapping foliage masses, and drawing hundreds of little ones is why it reads as speckle |
| `stamps` | **an authored leaf clump** | the simulation should say *where* foliage is and **art** should say what it looks like — which is how 2D games actually draw trees |
| `tone` | **unchanged** | it is not shape at all: wood and leaf sit at the same value, every tree is lit identically, and a stand has no depth |

**`tone` is the one to read carefully.** It changes no pixel's shape — every
cell is still exactly its own square, drawn at 1:1 and magnified like the
control — so if the stand reads better under it, four rounds of silhouette work
were aimed at the wrong quantity, and the cheapest arm is the answer. That is a
result worth having either way, which is why it is in the set.

All four share one simulation per capture, so at any scrubber position the
panes are the same stand at the same instant. Growth runs 155 plant cells to
14,857 across eight frames.

### 13a. Two arms were wrong on the first render, and looking is what caught it

Neither would have been caught by a counter: both *fired*, on every cell they
were supposed to, and produced a plausible picture.

- **`masses` fused the entire stand into one green hedge with no tree in it.**
  The radius was `1.5 * sqrt(n)` — set by eye, and 4.4x too wide at a full
  bucket. The fix is not taste but arithmetic: a blob standing in for `n` cells
  should *cover* about `n` cells, so `r = sqrt(n/pi)`. Area is what a count
  buys, and writing the radius as a free constant hid that.
- **`tone` striped trees mid-canopy** instead of layering the stand. Its
  near/far bit was derived by walking left along connected cells to find a
  "trunk column" — which only follows a horizontal *run*, a few cells wide
  inside a crown, so the bit flipped several times within one tree. The engine
  already knows which plant a cell belongs to (`Cell::organism_id`); the
  doc comment justifying the walk had talked itself out of using it.

Both are the same mistake in different costumes: **a quantity was derived from
local geometry when the engine already held the real answer.** That is the
sibling of §5d's pair, one level up — there the lattice was read where it had
nothing to say, here the geometry was.

### 13b. The refactor was proved, not assumed

Clippy found four real problems after the frames were rendered (a five-element
tuple return, two eight-argument functions, a range loop), and fixing them
touched every drawing path. A behaviour-preserving refactor that silently is
not one would have invalidated the posted card. Re-rendered and compared: **all
four arms byte-identical to the frames the owner is looking at.**


## 14. What shipped: `Shift+G`, foliage stamps

The four-arm card came back:

> *"Your timelapse is too fast. Deciding between shipped and stamps. Can you
> make it a toggleable option in the game. Dont use the a/b test button
> though"*

So `stamps` is the contender and it is now in the shipped renderer as
`FoliageMode`, default `Cells`, cycled with **`Shift+G`** — beside `G`'s
grain, because both answer *what does the texture of this material look like
when one cell is one pixel*. Not on `K`: that key cycles `plant::StemMode`,
which changes how plants **grow**, and this changes only how they are drawn.

This is `CLAUDE.md`'s own convention rather than a shortcut: *for "does this
look right", ship a runtime selector rather than choosing.* Five grain modes
behind one key settled in minutes what argument and stills could not, and four
review rounds here say the same is needed.

### 14a. One code path at every zoom

`cell_colour` already receives the pixel's sub-cell offset. The stamp is
evaluated at `x + (sub.0 + 0.5)/zoom` — so at 1:1 that is the cell centre and
the clump is sampled coarsely, and zoomed in the same clump resolves finely,
with no second implementation and no framebuffer change. The §2 supersampling
would make it finer still at 1:1, but it is **not required** for this and the
two are independent.

The stamp test sits **before** the empty-cell early return, deliberately: a
crown fills out because a leaf cell paints past its own square, so a test
placed after it would only ever repaint cells that were already foliage and
would read as a dead key.

### 14b. It costs 2.7x on a full redraw, and that is the honest number

Paired and alternating on `scene=grove`, which is dense foliage across the
whole viewport and so the worst case rather than the typical one:

| | worst full-screen draw |
|---|---|
| `cells` | 4.32 ms, 4.18 ms |
| `stamps` | 11.30 ms, 11.21 ms |

**+7 ms, about 2.7x.** It reproduces across alternating runs, so it is not the
box moving. Only full redraws pay it — the dirty-rect skip is untouched — but
a camera move forces a full redraw every frame, so walking through a wood is
exactly when it lands.

Most of that is the neighbourhood scan, and it is already much cheaper than
the naive version: `World::get` is a `HashMap` fetch, so a 5x5 scan is 25
SipHashes per pixel (~295 ns/px against `render_cost`'s 11.8 ns per hashed
read, some 48 ms a frame). Caching the chunk across the neighbourhood makes it
**one** hash per pixel in the ordinary case, since a 5-cell neighbourhood
crosses a 64-cell chunk boundary only at the edge.

The obvious next step is the gate §11 measured for the reconstruction — a
per-draw "foliage within reach" mask, so pixels that cannot be covered are not
scanned at all. It is **not built and not measured**, and default-off is what
makes that acceptable for now.

### 14c. What it does *not* change, and the one real consequence

**It cannot touch the simulation.** This module resolves colours at draw time
from a material id and a shade index and the simulation never writes a colour,
so a mode here cannot reach growth, collision, fire or foraging. The guard
proves the containment from the other side too: with no foliage anywhere,
`Cells` and `Stamps` must be **pixel-identical**.

What it *does* change is that a leaf cell paints outside its own square, so
the **drawn** crown is wider than the **physical** one — and everything
physical still reads cells. `MaterialDef::climbable`, `fall_drag`, fire spread
and an ant's footing all stop at a boundary the picture no longer shows, so
the gnome can fall through a painted edge. At `LEAF_STAMP_SPREAD` 2.2 that
overhang is about half a cell on each side. It is a real design consequence
rather than a bug, and it is the argument for a selector over a silent
default; the spread is the knob to pull down if it starts reading as one.

### 14d. The guard was watched going red, twice

Written after the code, so `CLAUDE.md`'s exemption does not apply and the
control was run. Both halves fail for their own fault and pass when it is
removed:

| fault put back | which half fired |
|---|---|
| `Stamps` made a no-op | *"an unchanged frame is what a dead key looks like"* |
| the scan no longer restricted to leaves | *"the scan may not touch rock or sky"* |

The second half is the one that would not have existed on instinct:
`leaf_stamp_at` runs on **every** pixel in `Stamps` mode, so a bug in its
neighbourhood scan repaints rock and sky as well, and a test that only looked
at foliage could not see it.
