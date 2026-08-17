# Polarity — the picture gate for Decision 6

Shot on the `forest` scene, the same scene the earlier plant sheets use.

**Re-shot after the economy pass**, so these show polarity as tuned and
after the `thicken()` fix, not as first landed. Two changes are visible in
them:

- **Canalization contrast 30:1 → 10:1.** At 30:1 this tree was a bare whip;
  it now carries a long lateral branch and roughly twice the foliage.
  Establishment across the variant ensemble went 56% → 73%, biomass +29%.
  See `organism::VEIN_GAIN`.
- **`thicken()` measures the trunk's real cross-section.** The base is now
  a tapering trunk rather than a slab spreading along the ground, and that
  change roughly *doubled* establishment on its own (baseline 5/12 → 11/12)
  because an unbounded trunk had been eating the seedling's own leaves. See
  `plant::stem_width`.

Read the vein sheet with the pipe model in mind: the thick lower trunk and
the lateral branch are saturated because they carry the whole canopy's
flux, and the thin upper twigs are dark because they carry almost none.
Brightness tracks *what a cell carries*, not how alive it is.

```
cargo run --release --example filmstrip -- scene=forest start=8000 every=1 count=1 cols=1 zoom=14 crop=64,4,44,44 channel=<off|celltype|vein>
cargo run --release --example filmstrip -- scene=forest start=1200 every=1700 count=4 cols=4 zoom=8 crop=64,4,44,44 channel=vein
```

| file | channel |
|---|---|
| `tree-material.png` | ordinary material colour — what a player sees |
| `tree-celltype.png` | `CellType`: purple `MatureBody`, pink `Leaf`, orange `RootTip` |
| `tree-vein.png` | **`VeinConductance`** — the max of a cell's four per-face carbon efflux conductances, `CONDUCTANCE_MIN..CONDUCTANCE_MAX` (1..10) |
| `vein-time-series.png` | the same channel at frames 1200 / 2900 / 4600 / 6300 |

`channel=vein` is new in this pass, and `B` cycles to it in the app.

## Why a picture at all, when §10 says not to

`Reports/plant-substrate-v2-design.md` §10 says **"do not screenshot-verify
tree shape at this step"**, and it is right about tree *shape*: with
`Photosynthesize` still on `GrowingTip`, every tip funds itself, transport
barely matters, and the visible silhouette proves nothing either way.

It is wrong as a whole. Canalization either produces a **strand hierarchy**
or it does not, and no unit test answers that — the Y-junction test proves
two faces can diverge, not that a tree grows a trunk. So the conductance
channel was built and shot, and the sheets below are the gate.

## What they show

**A real hierarchy, not a uniform lift.** In `tree-vein.png` the root
ball, the lower trunk and the long lateral branch are saturated, while the
thin upper twigs are dark. Nothing in the code computes subtree size or
identifies a trunk; the conductance field carries that information because
it is what the flux wrote into it. This is Shinozaki's pipe model arrived
at from transport rather than asserted.

Note the lateral branch reads *bright*, which is not an error: it carries a
real share of the canopy, so it carries real flux. Brightness is load, not
importance.

**It develops over time.** `vein-time-series.png` is the load-bearing
sheet: at frame 1200 the whole plant is dim and roughly uniform, and by
6300 the root junction and lower stem have pulled clearly ahead. A single
frame cannot distinguish "canalized" from "was always like that".

Paired numbers, from `plant_probe -- trees=24 frames=8000`, because the
sheet says *what and where* and only a number says *how much*:

```
vein conductance (max face per cell), 1..10:
  min 1.00  p50 4.25  p90 9.88  p99 9.99  max 10.00
  differentiation: 35% undifferentiated / 40% partial / 25% vascular
  strand strength, p99 vs the basal floor: 9.99x of a possible 10x
```

A quarter of the tissue is fully vascular, a third has never carried
anything, and strands reach the ceiling. That split is the mechanism
working.

**A metric that had to be replaced, and it is the interesting part.** This
originally reported `p99/p50` as a "strand contrast", and that number
*fell* from 6.1x to 2.4x across a change that made canalization plainly
better (the `thicken()` fix). Nothing about the strands had changed — p99
sat at the ceiling throughout. A thicker trunk simply means more cells
legitimately carry flux, which lifts the median, so a ratio against the
median was measuring how much undifferentiated tissue happened to be lying
around: a fact about tree *shape*, not about the mechanism. It now reports
the shape of the distribution instead. `CLAUDE.md`'s "ask what a metric
counts when nothing is wrong", for the second time on this branch.

## Two things not to misread

**Do not read the silhouette as a verdict on Decision 6.** Tree shape at
this step is set by the economy and by `thicken()`, not by polarity — §10's
point, and still true. Earlier versions of these sheets showed a bare whip
for exactly that reason; the `thicken()` fix is what changed it, not
anything in the transport rule.

The one place polarity *does* now reach shape is `stem_width`, which takes
its cross-section axis from `supply_direction` — the first consumer of the
conductance field outside transport.

**Shot on `forest`, not `tree`, deliberately.** Canalization is about
choosing between *competing* paths. The `tree` scene currently draws a
single one-cell-wide whip, which has no competing paths at all, so its
conductance sheet shows one uniform strand and tells you nothing. If this
sheet is ever re-shot, keep it on a scene whose trees actually branch.
