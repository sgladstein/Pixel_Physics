# Which authored parameters should become heritable — a survey design (2026-08-27)

**Status: method, not results.** Written so the question *"what should be
heritable?"* stops being answered by argument. Two proposals for it have
already been refuted (`plant-morphology-evolvability-2026-08-26.md` §5a,
§5b), and a third — review B's organ-allocation budget
(`plant-evolvability-three-reviews-2026-08-27.md`) — is currently unfalsified
rather than supported. Four opinions, no measurement.

## 1. Why a survey rather than another design round

A plant's genome is 10 continuous slots plus 6 discrete loci. Everything else
that shapes a plant is an **authored per-species constant**: roughly 37
scalars on `Behavior::Grow` plus the `SpeciesDef` top-level fields
(materials, palette bands, half-lives, `stomatal_reserve`). So there are on
the order of **25-30 parameters that set phenotype and are not heritable at
all**, and that set is the candidate pool.

**The method already exists in this repo and has never been pointed here.**
`plant-species-authoring.md`'s lever table ran exactly this experiment for
the *heritable* slots and produced hard numbers — `plastochron` 3.9x on
cells, `branch_chance` 2.5x, `turgor_per_cell` 1.3x on size with r = −0.74 on
height, `light_weight` and `upward_weight` **inert across 1,024 genomes**.
That table is why nobody wastes time on `upward_weight` any more. Nothing
equivalent exists for the authored-only parameters.

## 2. The two criteria, and the second is the one that gets forgotten

A parameter earns heritability only if **both** hold:

1. **It moves the phenotype** — measurable as an effect on a scale-free
   descriptor, and confirmed by the owner's eye.
2. **It has a counterweight.** `allocate_to_frontier` states the failure in
   its own comment: *"a quantity with a cost and no counterweight has exactly
   one optimum — the minimum — which a working economy will find and hold
   every plant at. The visible result is one root morphology everywhere."*

Criterion 2 is why `LEAF_RATE_ALLELES` is paired with
`LEAF_TRANSPIRATION_ALLELES` at every consumer, and why
`WOOD_DENSITY_ALLELES` scales strength and price together — *"one number for
both on purpose, so tuning cannot quietly turn the trade into a free lunch."*

**A free lever made heritable produces uniformity, not diversity.** Selection
finds its optimum and pins every individual there, which is the opposite of
what this programme wants. So the survey ranks on criterion 1 and the
trade-off pass filters on criterion 2; a parameter passing only the first is
a worse candidate than one passing both less strongly.

## 3. The survey

**Arms.** Per parameter, three `.ron` variants at low / as-authored / high,
everything else held fixed. Rebuild between every point — `.ron` is
`include_str!`, and identical output across settings means the knob was never
connected (this has produced whole invalid sweeps here). Run
`cargo build --release --examples` with `set -o pipefail` and read
`${PIPESTATUS[0]}`.

**Measurement.** `plant_probe`'s existing scale-free descriptors: crown
profile, foliage centre, foliage share, slenderness, root:shoot.

**Size is excluded from the ranking and printed as a diagnostic.** Every
discriminating number in the record so far is a magnitude, and the owner's
verdict three separate times is *"the biggest differences are still size and
color"*. A survey that ranks on size is guaranteed a positive result and
answers nothing.

**Unit of comparison.** Distributions over >= 30 individuals per arm, never
stand medians — within-stand spread runs 90 / 438 / 1,435 root cells and 31 -
153 cells for identical genomes, so a median is not a shape
(`root-morphology-findings.md` states this as an explicit method
consequence).

**Controls, both required.**
- *Sensitivity*: an arm known to move the descriptors must move them. `grass`
  against `tree` is the pair the owner has already rated as visibly different
  (4/5); if the descriptor set cannot separate those, it is blind and
  nothing else it reports counts.
- *Specificity*: an all-identical `tree.ron` arm. Its within-arm spread **is**
  the noise floor, measured rather than assumed. Any parameter whose three
  arms overlap by more than that floor did not move anything.

**Acceptance.** The top few parameters go to the review queue as **blind**
cards with the descriptor numbers in `meta`. The ranking picks what to
render; the owner's eye is the verdict. Bar: beat `prostrate`'s 2/5, and
prefer `grass`'s 4/5.

## 4. Traps specific to this survey

- **Founder monomorphism does not apply here** and it is worth saying so:
  this survey varies *authored species constants*, not alleles, so
  `plant.rs:718-719`'s hardcoded `LOCUS_BRANCH_ANGLE = 1` /
  `LOCUS_INTERNODE = 1` does not gate it. That freeze blocks testing those
  two *loci*; it does not block testing the `branch_angle` and `internode`
  constants they scale.
- **A parameter with no consumer, or a consumer gated off for the species
  under test.** This class has burned the project repeatedly — `light_weight`
  is inert because the light field has no gradient, not because the lever is
  dead; grass's whole carbon readout is zero because it owns no
  `CellType::Leaf`. Establish that each parameter's consumer is *reached* for
  the species being varied before believing a null.
- **Designed oscillators.** Day/night is 3,600 frames and first rain lands at
  14,400 at the default seed. Every arm shares `start_frame` and runs a
  3,600 multiple, or weather phase is in the numbers.
- **The `canopy_top` ceiling void.** A high-turgor arm can reach row 0, at
  which point every shape number from it is an artifact of the ceiling.
  Discard those samples and **report how many** — clipped plants converge,
  so silently keeping them reads as "this parameter does nothing".
- **Do not touch any sort on the plant path while this runs.**
  `allocate_to_frontier`'s tie order is element-type-dependent and silently
  changes how every plant grows.

## 4a. The candidate list, from a parameter inventory (2026-08-27)

A read-only inventory traced all ~45 authored per-species parameters to their
consumers. Full table in the commit that added this section; the three claims
below were re-verified in source by the coordinating session before being
recorded.

**The headline is not which parameter is biggest. It is that most of the big
ones are free.**

### Free levers — do NOT make these heritable without adding a cost first

`turgor_source` and `turgor_yield` (raising the height ceiling costs
nothing), `plastochron` (leaves cost no carbon — flagged in-code at
`plant.rs:2411-2416` as a known gap), `heading_inertia`, `juvenile_size`,
`juvenile_plastochron`, `seed_maturity` (lowering it is free), the
`Photosynthesize` `rate` scalar, `branch_angle` on the scoring side
(`plant.rs:2218-2224` bypasses scoring entirely for the wide path), and
`seed_half_life` / `remains_half_life`.

`crowding_weight` is the sharpest case: its counterweight was **deliberately
removed**. Since the cliff fix (`plant.rs:1897-1919`) crowding *divides*
rather than subtracts, so a fully crowded tip still takes its least-bad
direction and high values no longer kill tips.

**Why this matters more than the effect-size ranking.** Per §2, a heritable
parameter with no counterweight has one optimum and selection pins every
individual there. So making these genes produces **uniformity**, which is the
exact failure this programme exists to avoid. **The machinery package must
ship costs alongside the loci, not merely the loci** — and that is a
conclusion about the design, reached from the parameter surface rather than
from an opinion.

### Priced levers — the defensible candidates

`leaf_cluster` is the strongest: it is the single largest contributor to
visible foliage mass, and it is *partially* priced — `l_node()`
(`plant.rs:3151`) normalises per-node policy income in both the bud-break
budget and the frontier pool, while the per-cell carbon credit is **not**
normalised, and self-shading via `ambient_light_above` bites. Also priced:
`Grow.cost` (already heritable through wood density, and traded against
structural strength), `max_active_tips` (capped by `supportable` and splits
one carbon pool), `shade_death`/`drought_death` (cubed, and shed cells become
litter rather than nothing), `stomatal_reserve` (closing cuts the carbon
credit), `transpiration` (it *is* the counterweight to `rate` and
`leaf_cluster`), and `seed_cost` (debited from the parent, so dearer seeds
means fewer).

### The special case: materials

`shoot_material` / `root_material` / `leaf_material` have no counterweight in
the plant economy — **and are the largest measured screen effect in the whole
record**, since `grass` scored 4/5 (*"looks different from trees"*) and
material is what it moved. They may be self-pricing through physics rather
than through the economy: a material carries `density`,
`max_cantilever_reach`, `flammability` and `breaks_into`, so "everyone
converges on the best material" is only a risk if one material dominates on
all of those at once. **Unverified, and worth checking before designing
around it.**

### Three verified findings that change existing records

1. **`light_weight` is not "inert because the light field is flat".**
   `organism::phototropism_dir` (`organism.rs:3422-3430`) returns **only
   `(0,-1)` or `(0,0)`** — it has no lateral term at all, so it is a
   conditional duplicate of `upward_weight` and structurally cannot steer
   sideways. That is a different and more fixable diagnosis than the one
   `plant-species-authoring.md` records.
2. **Bud break is entirely dead for `grass`.** `break_buds` sums
   `intercepted` over `CellType::Leaf` only (`plant.rs:4066`), and grass
   declares no `Leaf` type, so `supportable` is 0 on every call and
   `grass.ron`'s `acrotony` and BudBreak `cost` can never reach the screen.
   Same root cause as the §5b carbon-book bug.
3. **`leaf_cluster` never runs for grass at all** — `leaf_due` requires
   `plastochron_interval > 0` and `grass.ron` is `plastochron: [0,0]`.

**Consequence for the survey: it must be run per species, not once.** A
parameter measured inert on grass may be live on tree and vice versa, and
three of the largest levers are already known to be dead for one of them.

### Also found

`Behavior::Transpire` has live consumers (`plant.rs:2572`, `4972`) and **no
shipped species declares it**. Genotype slot 9 is drawn at width 0.5-0.7 in
every shoot vector and has no consumer — already stated at
`organism.rs:1925-1933`, confirmed here.

## 5. What the survey cannot answer

It ranks parameters by whether varying them moves a plant. It does **not**
say whether the resulting forms are *new archetypes* — a flower, a fruit, a
determinate axis — because those cell types and behaviours do not exist, and
no setting of an existing parameter creates one. The reach report's bill of
materials is unchanged by any result here
(`plant-evolvability-handoff-2026-08-27.md` §3 records this distinction and
why the session previously blurred it).

What it does buy: if the machinery work goes ahead, this says which of the
existing knobs deserve to be genome slots alongside the new organs, and which
should stay authored — with evidence instead of a fourth opinion.
