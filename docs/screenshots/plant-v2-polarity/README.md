# Polarity — the picture gate for Decision 6

Shot after `12739bc` ("Give the carbon channel polarity, and let a strand
canalize"), on the `forest` scene, which is the same scene the earlier
plant sheets use:

```
cargo run --release --example filmstrip -- scene=forest start=8000 every=1 count=1 cols=1 zoom=14 crop=64,4,44,44 channel=<off|celltype|vein>
cargo run --release --example filmstrip -- scene=forest start=1200 every=1700 count=4 cols=4 zoom=8 crop=64,4,44,44 channel=vein
```

| file | channel |
|---|---|
| `tree-material.png` | ordinary material colour — what a player sees |
| `tree-celltype.png` | `CellType`: purple `MatureBody`, pink `Leaf`, orange `RootTip` |
| `tree-vein.png` | **`VeinConductance`** — the max of a cell's four per-face carbon efflux conductances, `CONDUCTANCE_MIN..CONDUCTANCE_MAX` (1..30) |
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
system and lower stem are brightest, brightness falls steadily toward the
tips, and the lateral branch on the left is almost dark. Nothing in the
code computes subtree size or identifies a trunk; the conductance field
carries that information because it is what the flux wrote into it. This is
Shinozaki's pipe model arrived at from transport rather than asserted.

**It develops over time.** `vein-time-series.png` is the load-bearing
sheet: at frame 1200 the whole plant is dim and roughly uniform, and by
6300 the root junction and lower stem have pulled clearly ahead. A single
frame cannot distinguish "canalized" from "was always like that".

Paired numbers, from `plant_probe -- trees=24 frames=8000`, because the
sheet says *what and where* and only a number says *how much*:

```
vein conductance (max face per cell), 1..30:
  min 1.00  p50 1.45  p90 27.77  p99 29.98  max 30.00
  1775/4906 cells still at the basal floor (36%) -- undifferentiated tissue
  strand contrast, p99/p50: 20.72x (ceiling 30x)
```

The median cell is essentially unpolarized while the top decile is
saturated. That split is the mechanism working.

## Two things not to misread

**The tree is a whip, and that is not polarity's doing.** It was a whip
before this pass too, and the trunk-thickening deficit is `thicken()`
growing sideways along open ground rather than around the stem — recorded
as known-open in `PLAN.md` and left for the economy pass. Do not read the
silhouette here as a verdict on Decision 6.

**Shot on `forest`, not `tree`, deliberately.** Canalization is about
choosing between *competing* paths. The `tree` scene currently draws a
single one-cell-wide whip, which has no competing paths at all, so its
conductance sheet shows one uniform strand and tells you nothing. If this
sheet is ever re-shot, keep it on a scene whose trees actually branch.
