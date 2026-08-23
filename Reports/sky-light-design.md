# Sky light: the design round for the open-cast-dig case

**Status: shipped. Propagated /4 is the default**, with `F12` cycling to
DEPTH → /2 → /1 for comparison, and
`skylight=off|4|2|1` on `viewshot` and `filmstrip`. Two of this report's own
claims were wrong and are corrected in *What building it changed* at the
bottom — read that before trusting the cost or accuracy figures above it.

*Design. Commissioned after
[`dark-bands-diagnosis.md`](dark-bands-diagnosis.md) shipped the per-cell
genesis map and left one case open: a pit you dig in daylight still draws as
cave, because those cells really were rock.
[`prior-art-underground-lighting.md`](prior-art-underground-lighting.md) said
the answer is propagation rather than a better boolean. This measures that
claim on the geometries that decide it, before anyone writes it into the
engine.*

Instrument: `examples/sky_light_probe.rs`. It builds one test world holding
every case at once — a 1-wide shaft, an 8-wide shaft, a 64-wide open pit, a
horizontal tunnel into a cliff, an overhanging lip, a sealed worldgen
chamber, and flat ground as the control — cuts the dug ones **after** the
surface freezes (a pit cut before it is a valley and proves nothing), and
reports three candidates side by side.

## Finding 1: the existing light channel cannot drive this, three times over

The obvious move is to draw `field.rs`'s light channel, which already does
Beer-Lambert attenuation through occluders and is already computed. Measured,
it fails on its own terms — all values normalised against the channel's own
open-sky reading at the same frame, which cancels the 20:1 day/night
oscillation exactly:

| sample | field channel | wanted |
|---|---|---|
| 8-wide shaft, 12 cells down | **1.00** | lit |
| 8-wide shaft, 48 cells down | **1.00** | dark |
| 8-wide shaft, 100 cells down | **1.00** | dark |
| 1-wide shaft, 12 cells down | 0.16 | dark |
| 64-wide pit, rim → floor | 1.00 → **1.00** | a gradient |
| tunnel, mouth → 5 cells in | 0.14 → **0.00** | a soft falloff |

Three separate disqualifications, none of them tuning:

1. **It hands a dug shaft free daylight.** `apply_sky` casts sun straight
   *down* each column and attenuates only through occluders, on the correct
   principle that clear air does not attenuate sunlight. The air above a dug
   shaft is clear, so the shaft is lit to the bottom. That is precisely the
   bug the owner remembers from before dug stone became background — *"if you
   dug a tall skinny shaft all the way down, it looked like sunny sky all the
   way down"*.
2. **It has a width threshold at exactly `FIELD_SCALE`.** A 1-wide shaft dims
   (its block is 7/8 solid, so Beer-Lambert bites) and a block-aligned 8-wide
   shaft does not dim at all. That is the same shape of defect as the 12-vs-13
   cell threshold `dead-ends.md` §977 records, arriving through a different
   door — and widening a shaft is what mining *is*.
3. **It cuts rather than fades at a cave mouth.** 0.14 at the mouth to 0.00
   five cells in, because the column cast is dead under 30 rows of rock and
   only the diffusion bounce reaches inside. `CAVE_FADE_DEPTH` exists to stop
   exactly that hard edge.

None of this is a fault in the channel. It answers "how much occluder is
directly above me", which is the right question for a leaf and the wrong one
for a cave.

## Finding 2: seeded propagation is right on every geometry

The model: sky light is seeded **only where the cell is outdoors**
(`World::is_outdoors` — the per-cell genesis map that shipped, which is the
first bit of Terraria's wall layer), and then spreads by distance with a
per-cell decay, `0.91` through air and `0.56` through material, taking the max
over all paths. Terraria's own constants, from the decompiled `Lighting.cs`.

**The difference from the existing channel is entirely in the seeding**, and
that is the finding worth carrying: both models propagate, and only one
refuses to hand a dug shaft free daylight. A shaft is not outdoors, so nothing
is seeded in it and its light has to walk down from the mouth.

| sample | exact solve | wanted |
|---|---|---|
| open sky | 1.000 | bright |
| sealed chamber | 0.000 | dark |
| 1-wide shaft, 1 / 12 / 48 down | 0.828 / 0.293 / 0.010 | lit, dark, dark |
| 8-wide shaft, 1 / 24 / 100 down | 0.828 / 0.095 / 0.000 | lit, dark, dark |
| 64-wide pit, rim → floor | 0.828 → 0.023 | a gradient |
| tunnel, 0 / 12 / 60 cells in | 0.910 / 0.293 / 0.003 | a soft falloff |
| under the overhanging lip | 1.000 | bright |

Right on all seven, including the two the shipped fix already gets right and
which any replacement must not break. Rendered, it is tapering wedges down
each shaft, a gradient in the pit and a bright wedge into the tunnel — a
picture of holes rather than of black rectangles.

The `0.91` figure is worth noting for a different reason: it reaches a tenth
of full brightness in **24 cells**, and `CAVE_FADE_DEPTH` is 24, set by eye
here years apart from Terraria and independently.

## Finding 3: the cost is all in the resolution, and 8 is too coarse

Terraria recomputes its screen light map every frame, so "Terraria does it
per frame" looks like a licence. It is not: **a Terraria tile is sixteen
screen pixels**, so a Terraria screen is roughly 120x70 = 8,400 cells, while
this engine's cell *is* the pixel and a 512x320 viewport is 163,840. Same
algorithm, two orders of magnitude more of it.

Measured on one viewport (512x320), sweeping the block size:

| block size | blocks | sweep | 1-wide shaft top | tunnel mouth | pit rim |
|---|---|---|---|---|---|
| exact Dijkstra | 163,840 | 4.85 ms | 0.828 | 0.910 | 0.828 |
| 1 (per pixel) | 163,840 | 4.41 ms | 0.828 | 0.910 | 0.828 |
| 2 | 40,960 | 0.55 ms | 0.400 | 0.871 | 0.793 |
| **4** | **10,240** | **0.13 ms** | **0.245** | **0.770** | **0.725** |
| 8 (`FIELD_SCALE`) | 2,560 | 0.03 ms | 0.069 | 0.242 | 0.424 |

Two things fall out of that table:

- **The four-sweep approximation is free of charge.** At block size 1 it
  matches the exact solve to three decimals at every sample; its worst
  disagreement anywhere in the world is 0.29, at one cell on a shaft wall.
  The accuracy that matters is lost to *resolution*, not to the algorithm.
- **`FIELD_SCALE` = 8 is too coarse and would have shipped looking broken.**
  At 8 the 1-wide shaft reads 0.069 and vanishes from the render entirely,
  and the tunnel mouth reads 0.242 — so it fixes the pit and leaves shafts
  and tunnels looking exactly as they do today. Block size **4** keeps all
  three structures, at 0.13 ms per viewport per frame: **0.8% of a 16.7 ms
  frame budget**, against a full redraw that already costs ~11.5 ms.

10,240 blocks at size 4 is within spitting distance of Terraria's ~8,400-tile
screen map. The prior art turns out to match on *grid size*, not only on
algorithm — which is the strongest evidence available that this is the right
scale to work at.

The block grid itself costs 2.4–3.3 ms to build from scratch, and that number
must **not** be charged to this approach: `FieldTile` already carries
occupancy and `rebuild_blocked` maintains it every field step. Timed
separately in the probe for exactly that reason.

## The options

**Option 1 — leave it.** The pit stays black. Costs nothing, changes nothing,
and keeps a known limitation the README now states plainly.

**Option 2 — per-frame propagation over the viewport at block size 4.**
~0.13 ms/frame, reusing the occupancy the field already maintains, sampled
bilinearly per pixel the way `glow_at` already does. No stored state, no
invalidation, always correct — recompute and the answer is right by
construction. The known cost is that it is *approximate* near one-cell
features (a 1-wide shaft reads 0.245 where the truth is 0.828, so it will be
dimmer than it should be), and that the per-frame recompute is real work on a
settled screen where the dirty-rect skip otherwise does nothing — the
animated-grain lesson, and it needs measuring against a settled world before
it is believed.

**Option 3 — per-pixel field, computed once and maintained incrementally.**
Exactly right everywhere (the first table), ~45 ms once at genesis against
worldgen's own ~325 ms, one byte per cell = 1.3 MB at 2048x640. Per-frame cost
only where cells changed, bounded by the 24-cell decay radius. The risk is the
invalidation: light *decreases* when a cell is filled, which needs a local
re-solve rather than a local relax, and a bug there leaves stale bright
patches that nothing will ever correct. That is the same class of state bug as
the stateful skyline §985 records.

**Option 4 — Option 2 now, Option 3 later if the approximation shows.** The
two share the model, the constants and the seeding; they differ only in where
the answer is stored. Nothing built for 2 has to be unbuilt to get to 3, in
the same way the per-cell map is on the way to a full wall layer.

**Recommended: 4, starting with 2, behind a runtime selector.** `CLAUDE.md`'s
rule for a judge-by-eye question is to ship the selector rather than choose —
five grain modes behind one key settled in minutes what argument could not.
The same key would give: off (today), coarse-4, coarse-2, per-pixel, with the
active one named on screen and its cost stated.

## What this would also fix for free

Both remaining artifacts in `dark-bands-diagnosis.md` are depth artifacts, and
a sky-light field replaces depth as the input to both ramps:

- the pit, which is the case this was commissioned for;
- **rock under a suspended object**, still over-darkened because the decision
  is per cell while `light_datum` is per column. Under a propagated field that
  rock is one cell from open air and reads bright, with no column datum
  involved.

## What must be re-checked before believing any of it

- The oscillation. Any *decision* taken on this channel has to go through
  `field::noon_equivalent_light`; the probe sidesteps it by normalising
  against open sky at the same frame, which a renderer cannot do.
- The settled-world cost, not the moving-world cost.
- Whether block size 4 reads as blocks. Noita hit exactly this and solved it
  by blurring rather than refining; bilinear sampling is that blur, and §0c
  is this engine's own record of what it looks like when it is missing.

*Freshness: 2026-08-23.*

## What building it changed

Two claims above did not survive implementation. Both are left standing in
place rather than quietly edited, because the *way* they failed is the
transferable part.

**1. "The four-sweep approximation is free of charge" was measured at sample
points and is false between them.** The probe compared the sweeps against an
exact Dijkstra at the cells the table names and found three-decimal agreement,
with a worst disagreement of 0.29 "at one cell on a shaft wall". Rendered, that
0.29 is not one cell: it is a **comb of vertical stripes down the inside of
every pit**. The sweeps are separable, so light can travel sideways then down
but never down then sideways — and a pit, where light enters at the rim and
must spread once it is inside, is exactly the shape that needs the second
order. Fixed by running the four sweeps **twice**, which costs 2x and clears
it.

This is `CLAUDE.md`'s own rule arriving from an unusual direction: an image
says *what and where*, a metric says *how much*, and the mistake here was
using a metric to answer a question about *shape*. A handful of sampled cells
cannot see a comb.

**2. "The block-grid scan must not be charged to this approach" was written
here and then ignored in the implementation.** The report says the engine
already maintains occupancy, so the 2.4–3.3 ms scan is not a real cost. The
first working version then built the grid with a `World::get` — a `HashMap`
lookup — per cell, over a viewport-plus-margin region of 286,720 cells, and
measured **+7.5 ms on a 13.2 ms redraw**: fifty times what this report
predicted.

`CHUNK_SIZE` is 64 and blocks are 1, 2 or 4 aligned to world multiples of
their own size, so a block can never straddle a chunk and **one lookup covers
a whole block**. That took it to +2.3 ms. It is the same lesson the fake-AO
note already records as the prerequisite for reviving that experiment, and
the same one `ChunkView` applied to the sweep — which is to say the engine had
written it down twice and the implementation still paid it.

### The cost, as built

Split, measured (`PIXEL_PHYSICS_SKY_LIGHT_TIMING=1`), at block size 4 over a
640x448 region of 18,193 blocks:

| | |
|---|---|
| block grid build | 1.7 ms |
| two rounds of four sweeps | 0.5 ms |
| **on a frame where the world or camera changed** | **+2.3 ms of a 13.2 ms redraw** |
| **on a settled, still frame** | **0** — one rebuild in eight frames, then none |

The settled figure is the one that mattered and it is answered: the rebuild is
gated on `camera_moved || scale_changed || !touched.is_empty()`, so the
dirty-rect skip keeps its winnings on exactly the state it exists for. The
+2.3 ms is paid only on frames that were already repainting everything.

### The unexpected result: finer is worse

The design above assumed block size traded cost against accuracy in one
direction, and recommended 4 as the cheapest acceptable point. Rendered, **4
is also the cleanest**. At /2 and /1 the separable-sweep striping is still
faintly visible even after the second round, because there is less bilinear
blur to hide it; at /4 it is gone. Noita's own answer to its 32x32 fog was to
blur rather than refine, and this is the same result reached from the other
end — which is a stronger reason to keep /4 than the cost argument this report
originally made for it.

*Prototype freshness: 2026-08-23.*

## Option 3 measured, because it had only ever been costed

The prototype shipped as Option 2 and Option 3 stayed a paragraph — "~45 ms
once at genesis, per-frame cost only where cells changed" — with its risk
named and never tested. Asked directly whether it had been tested, the answer
was no: what had been measured was per-pixel **recomputed from scratch** (4.4
ms four-sweep, 4.9 ms exact), which is a different thing from stored and
maintained. `examples/sky_light_probe.rs` now tests the real one.

**It works, and the hard direction works too.** Four edits — two removals
(light only rises, which a relax handles) and two additions (light falls,
which a max-propagation cannot undo locally), the last of them plugging the
mouth of a lit shaft so that a hundred rows below were lit *through* the cell
being filled. Each checked against a **full recompute**, not against a
cheaper version of itself:

| edit | cells re-solved | ms | worst error vs a full solve |
|---|---|---|---|
| dig a 12-wide room off the pit floor | 4,856 | 0.63 | 0.0005 |
| dig into the cliff behind the tunnel | 4,687 | 0.75 | 0.0005 |
| fill a block mid-pit (light drops) | 5,054 | 0.88 | 0.0005 |
| **plug the 1-wide shaft at its mouth** | 2,544 | 0.80 | 0.0029 |

The mechanism that makes this tractable is a **bounded local re-solve** rather
than the classic light-removal BFS: zero everything within `INFLUENCE` of the
edit, treat the ring outside as fixed sources, re-solve the interior. Its
correctness argument is one sentence — a cell's light is at most `0.91^d` of
any source `d` cells away *along the path*, and a path is never shorter than
the straight line, so a Euclidean box of that radius contains every cell whose
value could move by a representable amount.

**Swept rather than asserted**, because a theory predicting "just under the
threshold" and a measurement landing just under it are the same number twice:

| influence radius | worst error | against 1/255 = 0.0039 |
|---|---|---|
| 30 | 0.0381 | **visible, ten times over** |
| 59 (the theoretical minimum) | 0.0029 | just under |
| 90 | 0.0007 | comfortable |

The radius controls the error, so this is a real bound and not a lucky run —
and 59 clears by only 25%, so a shipping version wants ~90, which costs 2.3x
the box area.

### The result that reverses the intuition

Incremental is supposed to be the cheap one. Measured, it is not, and the
reason is that its cost scales with **how many separate places changed**,
while Option 2's does not:

| | cost on a changed frame |
|---|---|
| Option 2 (coarse /4, whole viewport) | **2.3 ms, flat** — independent of how much changed |
| Option 3 (per-pixel, incremental, r=59) | **0.7 ms per edit site** |
| Option 3 at the safer r=90 | **~1.6 ms per edit site** |

So Option 3 is cheaper only while fewer than about **three** separate places
changed in a frame, and about **one and a half** at the radius it should
actually use. A falling-sand world does not oblige: sand slides, water sloshes
and debris settles in many places at once, and a busy scene here runs 16 of 40
chunks awake on a collapse and 40 of 40 while terrain settles. Boxes would
have to be merged into a few bounding rects to compete, and a merged box is a
bigger box.

Option 2 pays a fixed 2.3 ms however violent the frame. Option 3's bill is
unbounded in exactly the situations the engine is most interesting.

### Where Option 3 still wins, and what it would cost to be safe

- **Panning over a still world.** Option 2 recomputes on every camera move;
  Option 3 does not, because its field is world-space and complete.
- **Anything off-screen that needs a correct answer** — creature sight,
  plant siting, a minimap. Option 2 only ever knows about the viewport.

Against that: 1.3 MB at 2048x640, a ~45 ms genesis solve, and one correctness
obligation that Option 2 does not have at all — **every** cell change must be
caught, including ones the CA makes on its own. Miss a falling grain and the
stale bright patch it leaves is permanent, because nothing ever recomputes it.
That is the same class of bug as the stateful skyline in `dead-ends.md` §985,
and it is the reason to keep Option 2 as the default even now that Option 3 is
known to work.

*Option 3 freshness: 2026-08-23.*

## The verdicts, and what they settled

Three cards, answered independently, and they agree:

- **Design round** (`20260823T005213673Z-68e25a`), per-pixel vs /4 vs /8 —
  *"2 looks good"*, which was /4.
- **Prototype** (`20260823T020127421Z-823b63`), the four modes in the app —
  *"2 looks best"*, which was /4 again, over per-pixel and /2.
- **Blind A/B against a stored per-pixel field**
  (`20260823T030324126Z-77658b`) — *"They look a little different but not
  clear which is better"*, with no pane chosen.

That last one is the one that decides the architecture. **A look nobody can
pick between does not justify option 3's machinery** — 1.3 MB, a ~45 ms
genesis solve, a per-edit-site bill that is unbounded in a busy frame, and one
obligation option 2 does not have at all: catch *every* cell change including
the CA's own, or the stale bright patch is permanent. Option 3 is measured,
works, and is not worth building. It stays recorded here in case the two cases
it uniquely serves ever come up — panning cost, and off-screen queries like
creature sight or a minimap.

So /4 became the default, and the old depth fade moved to `F12` as the
comparison.

## Two bugs the flip surfaced, both real

Turning it on by default ran it through guards the prototype had never
reached.

**1. Daylight came in around the side of the world.** The propagation region
is the viewport plus a margin, so it routinely extends past the world's own
bounds — and cells out there were neither solid nor outdoors, so the code
skipped them and left them counting as perfectly transmissive air. Sky seeded
*above* the world then ran down the columns *beside* it at the air rate and
re-entered from the side, lighting sealed rock near the left and right edges.
Measured at **0.0149 in a sealed chamber against the edge, where the same
depth mid-world reads 0.0** — faint, and above the 1/255 floor.

Nothing caught it: all four `underground-definition.md` guards build their
geometry far from the edge, and reverting the fix left every one of them
green. Now guarded by
`daylight_does_not_leak_around_the_edge_of_the_world`, and worth noting that
the *absolute* threshold in that test would have missed it too — 0.0149
slipped under a 0.02 bar. Only the **paired** comparison against mid-world
rock caught it, which is exactly what `CLAUDE.md` says a paired comparison is
for.

**2. The no-grid fallback was backwards.** `sky_light_at` answers `1.0` when
it has no grid, which is the right default for a caller asking about open sky
and exactly wrong at the one place that reads it: control only reaches the
ramp when `under_sky` has already established the cell is *inside the
ground*, so "no information" there has to mean dark, not lit. Reachable from
any `cell_colour` call without a preceding `draw` — a handful of tests — and
it lit a sealed room to 486 where full dark is 93. Caught by
`the_dark_under_a_roof_fades_in_with_depth_rather_than_cutting`, one of the
four guards `underground-definition.md` left behind, doing precisely the job
it was written for four milestones ago.

*Shipped freshness: 2026-08-23.*
