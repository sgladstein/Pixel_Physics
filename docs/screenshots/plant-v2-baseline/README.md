# Plant substrate v2 — the before-picture

Shot at the start of the plant-substrate-v2 work, against `master` at
`a39da4e`, **before any behaviour change**. Every sheet is the same run of
the same scene, rendered through a different channel:

```
cargo run --release --example filmstrip -- scene=tree start=0 every=1500 count=6 cols=3 zoom=3 crop=140,10,110,40 channel=<off|celltype|resource|canopy>
```

Tiles run left→right, top→bottom: frames 0, 1500, 3000, 4500, 6000, 7500.

The scene deliberately mirrors the geometry of
`../tree-rewrite-live-verification/`, the shots the owner's "still a tiny
tree, one cell thick, ~18 cells, no leaves, no roots" report was made
against, so this is comparable to a picture someone already has an opinion
about rather than to a fresh scene nobody does.

| file | channel |
|---|---|
| `tree-material.png` | ordinary material colour — what a player sees |
| `tree-celltype.png` | `CellType`: yellow `Seed`, green `GrowingTip`, purple `MatureBody`, pink `Leaf`, orange `RootTip` |
| `tree-resource.png` | the `Grow` energy budget, `0..RESOURCE_SCALE` |
| `tree-canopy.png` | canopy density, the crowding signal `Grow` scores against |

## What they show

**Every symptom in the report reproduces.** One cell thick everywhere;
about 18 cells total; no leaves; no roots. Growth stops completely by
frame ~3000 — tiles 3, 4 and 5 are pixel-identical.

**`tree-celltype.png` is the one worth looking at first.** It shows the
size ceiling directly, which no test asserts and no metric here measures:
at frame 1500 there are two green `GrowingTip` cells, at 3000 there is
one, and from 4500 onward there are **none** — every lineage has retired
to purple `MatureBody` and nothing in the engine can create a new frontier.
`PLAN.md` diagnosed this in prose ("no mechanism, epicormic budding or
anything like it, for a mature tree to issue a new shoot later"); this is
the picture of it.

It also shows, at a glance, that **no `Leaf` and no `RootTip` cell is ever
produced** — pink and orange appear nowhere in any tile, at any frame.

**`tree-resource.png` rules out the obvious wrong explanation.** The tree
is still holding a mid-range resource level in the frames where it has
stopped growing entirely. Growth here is **frontier-limited, not
income-limited** — so tuning the resource economy cannot lift this ceiling,
and `SecondaryThicken` never firing is not a starvation symptom either.

## Numbers, from `examples/plant_probe.rs`

An image says *what* and *where*; these say *how much*. Both were needed —
see the caution below.

```
after 400 frames: 13 active sites, 0 awake chunks
chunks were awake on 91/400 frames (22.8%)

max resource 1.961 / 4.0
max canopy   1.600 / 4.0
canopy decay ladder, newest cell to oldest: 1.600  0.800  0.533  0.267  0.000
one quantization step of canopy density is 0.267 (4 bits, 15 steps)
```

Two findings:

- **Canopy density works.** The deposit lands and decays exactly as
  designed. It is *not* inert. But the entire live range of the signal is
  about six quantization steps, and `CANOPY_DENSITY_DECAY_PER_TICK`'s own
  doc records that `0.5` was chosen to clear the quantization half-step —
  so the constant is set by the storage, not by the behaviour. That is the
  concrete version of `plant-substrate-v2-design.md` §3a's argument for
  moving these scalars off `Cell::aux`.
- **Transport is starved, not runaway.** `organism::diffuse_resource` is
  dispatched from the CA sweep, and the sweep skips settled chunks — so the
  *diffuse* step of deposit→diffuse→decay→follow ran on under a quarter of
  frames here, and stops entirely once the tree settles, while decay runs
  once per organism tick regardless. This is invisible today because
  `tree.ron` puts `Photosynthesize` on `GrowingTip`, so every tip funds
  itself. It stops being invisible the moment `Photosynthesize` moves to
  `Leaf` only, which makes moving diffusion to a per-organism pass
  (Decision 2) a correctness prerequisite rather than a storage tidy-up.

## A caution, recorded because it happened here

The canopy sheet was **misread twice** before the probe settled it.

The overlay first copied `apply_field_overlay`'s magnitude-scaled blend, so
canopy density was drawn as red blended into wood's own brown: a mid-range
reading moved one colour byte from 139 to 155 and the sheet looked blank.
The obvious reading — "the deposit isn't there" — would have sent a fix at
a mechanism that was working. The scalar channels now *replace* the cell
colour on a fixed dark→bright ramp instead, so the readout does not depend
on the material it is drawn over.

Even after that fix, the corrected sheet was read as "everything at the
ramp floor," and that was wrong too: 1.600/4.0 draws at roughly half
brightness, which is genuinely hard to separate from the floor on a
one-cell-wide twig. Only the probe settled it.

Both mistakes are the same one `CLAUDE.md` already names — reaching for an
image to answer *how much*. The sheets are the right tool for "there are no
leaves and no roots, and the tips are gone by frame 4500." They are the
wrong tool for "what is the value," and `plant_probe` exists for that half.
