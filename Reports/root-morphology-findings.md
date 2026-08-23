# Root morphology: why taproot-vs-fibrous is inexpressible today

Written after two review verdicts on root architecture, both negative, the
second with direction attached. This is a findings note, not a proposal:
it records what was measured, what the engine structurally cannot express,
and what is reachable now. No mechanism is designed here.

## The verdicts

1. On slot 1 (root branch chance) low vs high draw: *"clearly different but
   not as much as I would like. It just looks like more vs less roots
   instead of fully different morphology."*
2. On the order-hierarchy before/after: *"These also don't look clearly
   different. You are taking a structure that is already chaotic and
   variable between the plants and making slight changes that are not
   clear."*

With the target stated, and an explicit constraint on how to reach it:
*"I am not asking you to hardcode or specifically design these types of
roots, but create a system where these types of morphologies can develop
or evolve naturally (and should have different effects on the plant or
develop to fill an ecological niche)."*

The botany given: **taproots** — one dominant central root, swelling into
conical (carrot), fusiform (radish), napiform (turnip); **fibrous and
adventitious** — dense shallow networks, modifying into fasciculated
(dahlia), nodulose (tip-swollen), moniliform (beaded). Fibrous systems
also vary in *scale*: shallow fibrous (turf grass) spreading horizontally
to catch light rain, against deep fibrous (prairie grass) driving dense
networks feet down to survive drought and stabilise deep soil. Plus
environmental adaptations — buttress roots in shallow soil, aerial roots
taking humidity from air.

## Three reasons the sheets could not have shown it

### 1. Roots cannot thicken. At all. (`plant.rs`, `thicken`'s `can_widen`)

```rust
let can_widen = axis.iter().any(|&(dx, dy)| {
    let n = world.get(x + dx, y + dy);
    n.material == material::EMPTY
        || (n.organism_id() == organism_id && organism::cell_type(n.aux()) == Some(CellType::Leaf))
});
if !can_widen { return; }
```

A root cell is buried in **soil** — a `Powder`, neither `EMPTY` nor a
`Leaf` — so this returns early on every root cell in the world. **Every
root is one cell wide, permanently.**

`SecondaryThicken(pipe_ratio: 5.5)` sits on the `MatureBody` entry, and
that entry serves rootwood cells as well as wood ones, so the behaviour
*reads* live on roots and does nothing. Same shape as the `ByOrder`
finding one commit earlier: machinery present, gated unreachable.

**Consequence:** the entire taproot family is inexpressible. Conical,
fusiform and napiform are *thickness* shapes — a carrot is a carrot
because it is fat. A one-cell-wide dominant root is visually identical to
any other one-cell-wide strand, so no amount of tuning the existing knobs
can produce one.

### 2. Nothing makes one root dominant over its siblings

`allocate_to_frontier` shares carbon across root tips on a weight that
does not distinguish a primary axis from a third-order lateral. There is
no root analogue of apical dominance — no mechanism by which a leading tip
suppresses or out-competes its siblings.

A democratic frontier produces a **fibrous** system by construction. That
is not a tuning position; it is the only position reachable. Taproot is
not at one end of a current axis — it is off the map.

### 3. The baseline is chaotic, so a distribution shift is invisible

The owner's second verdict names this exactly. Between-plant spread inside
a single stand is enormous (one measured stand: root cells min 90, median
438, max 1,435). Both comparisons posted were **stand against stand**, so
a shift in the median sat inside the within-stand variance and could not
read as a difference.

**Method consequence for whoever picks this up:** stop comparing stands
for a morphology question. Compare single plants at high zoom, or render
N plants per treatment as a grid so the *distribution* is the thing shown,
and pair it with the discriminating number. A morphology claim is about
shape, and a median is not a shape.

## What IS reachable now, and is the near-term win

The **shallow-fibrous against deep-fibrous** distinction — turf grass
against prairie grass — has its hooks already:

- genotype slot 5 (root tropism gain) steers depth against spread, and is
  live and measured;
- the water economy already differentiates surface moisture from a deep
  table, so the two strategies have genuinely different payoffs;
- the owner's framing asks for exactly this coupling — *"different effects
  on the plant or develop to fill an ecological niche"*. Shallow catches
  light rain; deep survives drought. That is a real trade the engine can
  already price.

This does not need new mechanism. It needs the comparison run and rendered
so the two ends are visible, on the presentation method above rather than
stand-against-stand.

## What a system-not-preset answer would need

Recorded as constraints on any future design, per the owner's explicit
"do not hardcode these types":

- **Thickening driven by a quantity, not a species preset.** Whatever lets
  a root widen has to be fed by something the plant *has* — stored carbon,
  downstream demand — so that a storage organ is a consequence of the
  economy rather than an authored shape. The `can_widen` soil gate has to
  be answered first: a root displacing soil to thicken is a different
  physical claim from a branch widening into air, and `displace_soil_water`
  already exists as prior art for a root taking a soil cell.
- **Dominance as a mechanism with a genome handle**, not a flag. If a
  leading tip can suppress siblings, then "how strongly" is a slot, and
  taproot against fibrous becomes *where an individual sits on that axis*
  — which is what makes it evolvable and selectable rather than authored.
- **Both must pay.** A taproot that costs nothing extra saturates and
  stops being variation, the same failure §4.7 records for penetration
  force.

Whether either is worth building, and in what order relative to the
silhouette work (`plant-evolution-design.md` call 9: materials → foliage
mass → age), is an owner call and is not made here.
