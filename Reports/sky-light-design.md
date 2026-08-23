# Sky light: the design round for the open-cast-dig case

*Design, nothing built. Commissioned after
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
