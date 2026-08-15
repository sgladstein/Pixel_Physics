# The tree shape problem, stated plainly

Written because the work kept oscillating between two failure modes and the
oscillation is itself the diagnosis. The owner:

> It seems like we just keep switching back and forth between thin whisp
> and big blob. What is the challenge?

## 1. The two failure modes

Every configuration tried lands in one of exactly two states:

| | mass | structure | growth over time |
|---|---|---|---|
| **Blob** | huge (12,039 wood) | none — a porous lump | fills until it stops |
| **Whip** | tiny (~40 cells) | a thin line with a few kinks | stops at ~frame 5,000 |

Measured, same scene (`grove`, 8 trees, 14,000 frames), varying only
`SecondaryThicken`:

```
thickening unbounded    12,039 wood   253 leaf   105 cells wide   BLOB
thickening bounded       2,817 wood   322 leaf    13 cells wide   WHIP
thickening off           1,623 wood   384 leaf    12 cells wide   WHIP
```

## 2. Why tuning cannot escape it

**Mass and structure are produced by two different mechanisms, and only one
of them is doing anything.**

- **Extension** — `Grow` advancing a `GrowingTip`, adding length and
  branching. This is what makes *structure*: a trunk, limbs, a crown
  outline.
- **Thickening** — `SecondaryThicken` adding girth beside existing cells.
  This makes *mass* and cannot make structure; it can only fatten what
  extension already drew.

Every knob touched so far has been a thickening knob. Thickening moves the
plant along a single axis — how much mass — so tuning it slides between
"too much mass" and "too little mass" **with structure absent at both
ends**. That is the oscillation, and it is a property of which knob is
being turned, not of the values.

## 3. The actual defect: extension is one-shot

A real tree extends for its whole life. Ours extends once and stops.

Traced:

- A `GrowingTip` that fails to find a candidate for `ORGANISM_STALE_LIMIT`
  (4) consecutive ticks **retires permanently** to `MatureBody`.
- `max_active_tips` caps concurrent tips at **14**.
- **Nothing ever creates a new tip.** Once every lineage has retired, the
  organism has no frontier and growth is over for good.

Measured: active sites reach **0** by frame 16,000 and the cell count is
flat from there. The tree is not slowing down; it is finished.

So the plant gets a one-shot budget of roughly 14 tips, each of which
extends a short distance and dies. Whatever that draws is the final
structure, forever. It is a whip because 14 short extensions is a whip.

**The blob is this same defect wearing a disguise.** With extension dead,
the only mechanism still able to add anything is thickening, so all
subsequent growth is girth. Unbounded thickening turns a whip into a lump
without ever adding structure — which is why the blob has no branches in
it, only lobes.

## 4. Why the obvious fix failed before

Bud break (design doc §2e) was built to solve exactly this and was
reverted. Its rule converts a `MatureBody` back to a `GrowingTip` when it
has surplus carbon and low local crowding. It ran away, and the reason
generalises:

**When a tree stops growing, every local signal equalizes at once.** Carbon
fills every cell to `RESOURCE_SCALE` (the transport clamp guarantees it),
crowding decays everywhere within two ticks, and conductance decays to
basal everywhere because there is no flux. So any purely local "am I idle"
test fires on *every* mature cell simultaneously, and budding becomes
proportional to volume.

Capping it to one bud per organism per tick converts exponential growth
into linear growth, which still fills the world. Ratio bounds
(root:shoot, shoot:root) bound *proportions*, not size.

## 5. The question this needs answered

> How does a plant sustain frontier extension over its whole life, bounded,
> in a way that produces **structure** rather than mass?

Constraints any answer has to satisfy:

- **Bounded without an arbitrary cap.** "Stop at N cells" is not an answer;
  the bound should fall out of the mechanism.
- **Local.** The engine is a per-cell CA with a per-organism upkeep pass.
  No whole-tree traversal per cell.
- **Cannot rely on a local idle signal**, per §4 — they all saturate
  together.
- **Per-species.** Trees first, but vines, bushes and unfamiliar forms must
  be reachable points on the same parameter surface.
- **2D.** A crown has `~R³` of volume to branch into in 3D and `~R²` in 2D,
  so the same branching is `R` times denser here and neighbouring
  structures merge readily.

## 6. What is already known to work, and should not be re-litigated

- Polarity/canalization: a real strand hierarchy, 33% of tissue fully
  vascular, strands at the ceiling.
- The pipe model gate, once measured on the **row total** rather than
  per-cell.
- Light, after `LIGHT_DECAY` was corrected — it was attenuating through
  empty air and capping tree height at ~21 rows.
- Crown shyness (owner-blind crowding), for stand-scale spacing.
- Roots, soil moisture, transport, structural collapse.

## 7. Measurement discipline for whatever is tried next

Three metrics on this branch have measured something other than their own
name (`rows >1 cell wide` → the basal slab; vein `p99/p50` → tree shape;
widest run → the root mat). Judge by:

- **the picture**, on `grove` (96 rows of sky) — never on `forest` or the
  default `plant_probe` ground, both 40-row scenes with a ceiling;
- **stem thickness above the base**, not `rows >1 cell wide`;
- **wood:leaf ratio** — a tree is a sparse skeleton carrying a mass of
  foliage. Ours has run at 48:1;
- **whether growth is still happening at frame 20,000**, which is the
  actual subject of this document.
