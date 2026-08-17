# Authoring a plant species that is not a tree

Everything on this branch was learned by building *one* species. This file
is the part that generalises — what a second, third or unfamiliar plant form
will need, and which of the levers are real. Written because the owner's
stated intent is trees, vines, bushes and forms with no earthly parallel, all
on this platform, and because most of what follows was expensive to find and
is invisible in the code.

Read `design-philosophy.md` first for what belongs in data at all. This is
the empirical layer under it.

## 1. The levers that actually move a silhouette, ranked by measurement

From two 1,024-genome studies (each individual draws its own multiplier on a
trait, so one run measures a response curve). Mean plant size across
quintiles of the drawn value, low draw to high:

| lever | quintile means (cells) | verdict |
|---|---|---|
| `plastochron` | 2459 → 634 | **strongest**, 3.9x, inverted |
| `branch_chance` | 808 → 2018 | 2.5x |
| `turgor_per_cell` | 1477 → 1176 | 1.3x on size, **r = −0.74 on height** |
| `pipe_ratio` | 1537 → 1247 | weak on size, real on girth (stem 16.1 → 12.7) |
| `light_weight` | flat | **inert** |
| `upward_weight` | flat | **inert** |

**Design the plant with the first three.** Leaf spacing sets how much foliage
a length of shoot carries, which sets income, which sets everything. Branch
chance sets how many shoots exist. Turgor sets how tall it can get, and it is
the only lever that reads almost purely as *height* rather than as size —
which makes it the one to reach for when a form should be low and spreading
rather than simply small.

**Two levers do nothing, and one of them is a bug in disguise.**
`upward_weight` is simply inert. `light_weight` is inert *because the light
field is nearly uniform above the canopy* now that `field.rs` casts per
column — phototropism has no gradient to steer by, so it steers by noise. A
shade-loving understorey plant will need that fixed before it can work at
all.

## 2. Every global bound becomes a visible artifact

Written three times on this branch, each time by a different door:

- the **world ceiling** — trees pinned at row 0 and spread sideways;
- the **turgor ceiling** — trees pinned at their derived height and fused
  into one canopy slab 136 cells across;
- the **hard cutoff** at that ceiling — each crown a flat horizontal plate,
  because every lineage ran at full speed to one row and stopped there.

The fixes generalise: **make the bound graded, and make it per-individual.**
`turgor_taper` (extension rate ∝ `P − Y`, per Lockhart, rather than a step)
turned plates into rounded crowns. `genotype_variance` spread the arrival
times so the bound is not one line across the stand. Any new bound a species
introduces will need both, and the failure looks the same every time: a
straight edge somewhere in the render.

## 3. A stand is not a plant, and a light-optimal stand is a slab

At any height bound, the light-optimal shape of a *stand* is a flat canopy
one cell thick: `ambient_light_above` is per-column sky visibility, so income
grows with lit width and nothing bounds width. Growth follows light, so
growth finds the slab. **Only lateral crowding opposes it**, and it has to be
strong — `crowding_weight` had to go from 0.5 to 6.0 (measured: thickest
fused run 136 → 39 cells at 57-cell spacing).

Raising upward bias and branching along the trunk both made it *worse*, by
pushing more growth into the ceiling. If a new form fuses with its
neighbours, crowding is the lever; nothing else tried has worked.

The term is deliberately **owner-blind** (`candidate_crowding` ignores
`organism_id`), because it models far-red shade avoidance and a phytochrome
cannot tell whose leaf reflected the light. One rule therefore keeps a plant
from merging with itself *and* with its neighbour — which matters far more in
2D, where a crown has `~R²` to branch into instead of `~R³`, so the same
branching is `R` times denser and neighbouring structures merge readily.

## 4. Structure comes from tiers, not from tuning

Fourteen tips drew a whip because they were fourteen copies of one rule. The
fix was `ByOrder<T>` — per-branch-order parameter lists that pad with their
last value, so:

- **one value** is a species with no tiers (moss, a mat, a vine);
- **two values** is a shrub — one trunk tier and everything above it alike;
- **four values** is a tree.

A bare bole falls out of `plastochron: [12, 5, 2, 2]` alone: the trunk tier
leafs rarely, twigs leaf constantly, and no rule ever has to decide which
cells are "trunk" — which a cell cannot know locally, and which four
successive support models failed trying to infer from shape.

**Order is position in the plant, not age**, and that distinction has already
bitten. A seedling is order 0 and sees only the trunk tier, so a leafless
trunk tier starves it before it can ever branch. Narrowing `branch_chance`'s
variance from ±60% to ±35% took establishment failure from 35% to six trees
in eight, because *branching is how a small plant builds leaf area*. Any
species whose juvenile form differs from its adult form needs an age axis
that does not exist yet.

## 5. Extension makes structure; thickening only makes mass

`SecondaryThicken` cannot make structure — it can only fatten what extension
already drew. A long stretch of this project oscillated between "thin whip"
and "big blob" because every knob being turned was a thickening knob, and
thickening moves the plant along a single axis. **If a new form looks wrong
in shape, the lever is in `Grow`, not in `SecondaryThicken`.**

Girth follows Shinozaki's pipe model: cross-section ∝ the foliage above.
Measure that cross-section as *this stem's own contiguous run of woody
cells*, never the row total (a branched plant has more than one run on 53% of
its rows, so a limb suppresses the trunk beside it) and never the immediate
neighbours (a run's growing end always reads 2 and spreads without limit).
Leaves are foliage, not xylem, and must not count as cross-section.

## 6. Nothing creates frontier except the bud bank

A tip that fails for `ORGANISM_STALE_LIMIT` ticks retires permanently. Before
`DormantBud`, once every lineage had retired growth was over for good — zero
active sites by frame 16,000 with the cell count flat from there. **Any
species that should keep growing needs buds.**

Two properties are load-bearing and neither is tuning:

- Buds are deposited by **extension only**, never by thickening — so a blob
  generates no new growth potential however large it gets. That asymmetry is
  what the earlier, reverted bud-break design lacked.
- The break decision is **whole-plant**, in `break_buds`, never local. Every
  local "am I idle" signal saturates simultaneously when a plant stops
  growing (carbon fills to the clamp, crowding decays, conductance relaxes to
  basal), so a local rule fires on every mature cell at once and budding
  becomes proportional to volume. The gate is
  `supportable = ⌊intercepted light / cost⌋`.

Bud *siting* matters as much as bud timing: picking the brightest bud builds
a flat cap, because the brightest bud is always on top of whatever the plant
has already built. Light **per unit crowding** moves flushes to buds with
room.

## 7. Measurement discipline specific to plants

- **One plant is one draw, not a result.** Twelve individuals from one genome
  span 31 to 153 cells. Three separate unit tests on this branch asserted on
  a single grown tree, and all three broke the moment genotypes varied. Any
  test that plants one plant and asserts on what it grew is measuring a
  sample from a wide distribution.
- **Sample the population, not the parameter.** Because every individual
  draws its own genotype, *one* run of N plants is a response curve over N
  genomes — including interactions, which one-knob-at-a-time sweeping cannot
  see. Every global sweep tried on this branch came back monotone, meaning no
  interior optimum exists for a search to find.
- **Set jitter width per trait, from what it moves.** A flat ±15% left three
  of four traits measuring |r| ≤ 0.05 — they were not inert, they were
  under-sampled. Watch integer quantization especially: ±15% on a plastochron
  base of 2 rounds back to 2 across the entire range, so that dimension was
  dead exactly where the crown is built.
- **Salts are positional.** Retire a dead trait by setting its width to 0.0,
  never by removing its slot — renumbering silently rewrites every genome
  ever measured.
- **Edge plants are privileged.** In a row of N, the first and last average
  2x the interior, because they have a free side. Drop them before quoting
  any per-plant statistic.
- **Failure to establish is mostly competition, not genotype.** 35% of a
  1,016-plant stand finished under 300 cells, with failures falling on a
  regular pitch (median gap 2). The stand self-thins. That is a real forestry
  result, not a defect to fix.

## 8. Cost

Frames are the entire cost and scale linearly. Plant count is nearly free
until the cores saturate — 8 and 32 plants both ran 30,000 frames in ~101 s
in a fixed-size world — but packing more plants into the same width changes
the spacing, and spacing is exactly what crown shyness decides. Widening the
world with the count costs proportionally. Scout at 10,000 frames; confirm at
30,000.
