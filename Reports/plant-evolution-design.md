# From trees to plants: materials, ecology, evolution, speciation

**Status: partly implemented** — all nine §8 calls were signed off
2026-08-19, and litter, decay, grass and the creeper have since landed; §4a's
register holds the probe verdicts and the do-not-retry notes. *Design and
comparison, 2026-08-18. No implementation* was the standing at the time of
writing and is kept for the date it carries. **Corrected 2026-08-27.** This
document does three things the owner asked for in one sitting: it answers
the plant-materials brief (how this engine grows a plant that is not a
tree), it compares the creature evolution plan against the plant genome
work (where they overlap, where they differ, where they must converge),
and it stages a path toward plant ecology, evolution and speciation. It
ends with numbered calls; nothing below decides them.

**Audited against:** `plant-substrate-v2` at `7b57f8f` (this branch — the
water economy and the signed-off genome re-map are both merged here);
`Reports/creature-evolution-plan.md`, which is **uncommitted** in
`.claude/worktrees/creatures` (branch `creatures-m18`) and cited as such;
`Reports/creature-direction.md` §13 at master `bb20167`. Every code claim
below was checked by reading the file at a line, not carried from a brief;
lines will drift — re-read the function, not the diff.

**Required reading before implementing anything here:** `CLAUDE.md`;
`Reports/design-philosophy.md` §3 (one substrate, many species as data —
the committed direction this document extends); `Reports/plant-genome-design.md`
(the slot map is FINAL); `Reports/creature-evolution-plan.md` §6 (the
dead-end register — several of its entries bind plant-side proposals too);
`Reports/population-dynamics-research.md` (its §12 hands two findings
directly to this work).

---

## 0. The one-page finding

**The creature programme and the plant programme converge on a single
artifact, and neither document says so because each was written on its own
branch: a herbaceous ground layer.**

Three sessions of creature work ended with the same diagnosis in different
costumes — the binding quantity is the fraction of animals that find food
(`creature-direction.md` §13o), every preset ends a run with thousands of
leaves and 0–11 of them within three cells of the ground (§13n), and moss
alone is worth a third of the entire foraging advantage (+0.187 → +0.247).
The creature plan's S4 (litter) and S7 (a second larder) are attempts to
manufacture a ground-level food layer out of what trees shed.

Meanwhile the plant-materials brief asks what a non-tree plant even is,
and the answer that survives §4's tests below is: **grass** — a short
`Grow` species on cheap herbaceous materials that photosynthesizes at the
surface, roots shallowly, holds banks, burns fast, and litters. That is
not just the next plant milestone; it is the creature ecology's missing
food base, the fuel bed for a fire regime, and the first species pair
(grass vs tree) whose competition axes — light above, water below —
already exist and are already measured.

So the recommendation underneath everything else here: **treat the ground
layer as one piece of work serving both programmes**, land its shared
artifacts (species-declared materials, a litter material) once, and let
the creature stages and the plant stages both consume them.

---

## 1. The two evolution programmes, compared

### 1a. What already converged — without either side planning it

Both kingdoms now sit on the organism substrate
(`Reports/organism-substrate-design.md`): generational handles, species as
`.ron` data over a shared behaviour library, per-organism sidecar state,
cell-typed CA-native bodies. That unification is done and neither plan
re-litigates it.

Less obviously, **the two genome designs arrived independently at the same
contract**, which is worth stating because `creature-direction.md` D3
promised "one shared mechanism, implemented creature-first, designed so
plants adopt it later without rework" — and events overtook it: the plant
genome shipped first, signed off, slots positional forever
(`plant-genome-design.md` §9). What both sides now hold, separately
implemented:

| Contract term | Plants (shipped) | Creatures (planned) |
|---|---|---|
| Fixed-length positional slots, append-only, never renumbered | `genotype_draws: [f32; 9]` + `alleles: [u8; 6]` (organism.rs:1122–1142, "positional forever") | reserved-dimension weight blocks, 584 floats (creature plan S2) |
| Per-slot mutation width, not one global width | `genotype_variance` per species per slot — the flat-width failure is measured (a third of the genome dead at ±15%) | E6: `width = floor + rel·|w|`, retiring §7a's ±4.0 clamp — the ±30 homing gate is the measured failure |
| A dead slot is evidence about its *condition* first | slot 1 measured exactly zero and was starvation, not dead code (§8a/§8c) | the granular-divisor rule, cited in S1's falsifier |
| Mutation draws keyed on identity, never on position | **divergent — see §1c** | `rng::stream(seed, child_handle, …)` (direction §7a rule 3) |

The honest reading: **D3's "one mechanism" is satisfied at the contract
level and will never be satisfied at the code level**, because the two
genomes store different kinds of thing (nine developmental scalars and six
strategy loci on one side; a brain's weight matrix on the other) and each
representation is already load-bearing. Unifying the code now would be a
refactor with no pixel attached. §8 asks the owner to ratify that reading
rather than leave D3 half-true on the books.

### 1b. Where the programmes genuinely differ

| | Plants | Creatures |
|---|---|---|
| **What a gene changes** | development and physiology: rates, thresholds, costs (branch chance, plastochron, stomatal point, wood density) | behaviour: connection weights; later anatomy (S8) |
| **Discrete strategy loci** | yes, by design — jump mutation at 0.03/seed is what lets a morph persist as a *cluster*; drift would smear it (organism.rs allele doc) | none — continuous weights only; clusters are expected from selection plus asexual isolation |
| **Reproduction and heredity** | **live today**: `Reproduce` sets seeds carrying the parent's draws ±0.08 jitter and alleles at 0.03 re-roll; evolution is *on* for plants and has been since the re-map | S6, not yet built; heredity is one line when it lands (clone the parent's genome) |
| **Selection unit and clock** | individual, via seed set and establishment; `seed_maturity` 600–700, but the real clock is canopy gaps (see §5c) | individual via budding (E5); ~900-tick life, a hundred generations in minutes — chosen precisely so evolution is *watchable* |
| **Fitness instruments** | paired stands, the 2×2 crossover, megastudy, allele census in the probe | survival + `ants fed`, behaviour-space coverage, reciprocal transplant (S7) |
| **Phenotype readout** | **live**: foliage band = leaf economy, bark band = wood density, legible at play zoom (§8b) | planned: palette lerp on `gut_bias` (S5) |
| **Measured divergence evidence** | the leaf-economy sign flip: acquisitive +21% cells / +32% seed on wet, conservative +43% retained foliage on dry, and the greedy stand drinks its bed dry (§8d) | none yet — S5's two-humped survival curve is the target |

Two asymmetries worth holding onto:

- **Plants are ahead on machinery, behind on population dynamics.**
  Fifteen live loci, heredity running, colour as readout, one measured
  two-niche crossover — but no organism reclamation (a dead plant's slot
  leaks; `free_organism`'s free side is never called for plants, and
  §13m left plant liveness as a known open gap), an immortal seed bank
  (seeds never decay — `plant-genome-design.md` §4.8), and adults that
  never die of anything but violence. A stand that closes canopy is a
  population with the generations turned off.
- **Creatures are behind on machinery, ahead on the selection loop.**
  Nothing is heritable yet, but the plan's `reproduce_threshold >
  start_energy` move makes doing-nothing an extinction instead of a
  strategy, and the generation clock is fast enough to watch. The plant
  side has no equivalent statement of "what forces turnover" — §5c below
  is the attempt.

### 1c. The one real contract divergence: founding-draw keying

Creature law (direction §2c, §7a rule 3, gotcha P-21): every
fitness-relevant draw is keyed on the organism handle, never on position —
position-keyed draws make location a hidden inherited variable and
manufacture spurious selection results. The direction doc explicitly says
the plant genotype's coordinate keying "is on the list of things the
shared-genome work will fix for plants later."

The plant code, read today, defends the opposite choice on purpose
(plant.rs:547–558; organism.rs:1027–1038): founding draws are keyed on
`(world seed, germination coordinate)` so a genotype survives planting
order, slot reuse (a 4-bit generation wraps at 16) and save/load; the
known cost — two individuals germinating at the same cell in a long-lived
world draw the same genotype — is recorded as deliberately not fixed
(plant.rs:564–567).

**The tension is narrower than it looks, and it should be measured before
it is re-litigated.** Inherited genomes short-circuit the positional draw
entirely (plant.rs:575–577) — a lineage's genetics flow parent to child
and position never re-enters. The confound P-21 warns about therefore
binds only *founders*: a favourable spot re-sows the same founding
genotype every time worldgen or reseeding plants there, which could read
as "this allele wins here" when the truth is "this coordinate always
draws this allele." Whether that can actually contaminate the §6
experiments depends on what fraction of a long run's population are
founders versus descendants — a number the generation counter proposed in
§7 (P0) produces for free. §8 puts the call to the owner with that
measurement as the gate, rather than re-keying on principle.

### 1d. Lessons each side has already paid for that bind the other

These are transfers, stated so neither side re-buys them:

- **The ecology is the binding constraint** — creature §13f/k/m/n/o,
  plant slot-1 starvation and the wet-scene-only leaf economy. Every
  divergence failure below should be suspected of being a niche-supply
  failure before a genome failure.
- **The horizon artifact** (§13o): any outcome variable that is a
  function of run length flatters one strategy shape. Plant studies are
  not immune — establishment fraction and allele frequency both drift
  with horizon while a stand is still filling. Fixed-horizon plant
  studies should state what phase of stand development they sample.
- **A metric here has measured the spawn layout three times** (§13n) —
  plant establishment studies must report `placed` alongside outcomes.
- **The multi-task fitness warning** (`PLAN.md` evolution notes): with a
  single binding task, selection collapses the population onto one
  morphology. The plant genome was built to this spec — every locus names
  a trade — but the *scenes* decide whether both sides of a trade exist
  (the conservative allele bought nothing until the dry scene existed).
  Speciation work is mostly scene work.
- **The clonal drift band** (creature S6 pre-flight 2): at small
  population sizes, neutral drift alone fixes a lineage on ~2N
  generations. Plant stands are 8–16 individuals in every current
  harness. **No allele-frequency result at stand scale means anything
  until the drift band is published for that N** — this transfers to
  plants verbatim and none of the plant studies to date has stated it.

---

## 2. Where the programmes must converge — the trophic seam

Four artifacts are about to be built twice unless coordinated:

1. **Litter.** Creature S4 writes a `litter` material (Powder, light,
   flammable, its own food value) and has abscission emit it. Plant
   abscission today deletes leaves outright — `Cell::EMPTY` at
   plant.rs:3510, :3541 and :3740, with a comment conceding the gap
   ("a leaf overwritten by wood is the honest minimum", plant.rs:4093).
   Dead grass (§3) wants the same material or a sibling. One material,
   one abscission edit, two consumers. Whoever lands it owns the
   settled-forest frame-cost measurement S4 already specifies (falling
   powder defeats the dirty-rect skip exactly where it earns its keep).
2. **Food value as material data.** Creature S3 (`Material::food_energy`,
   E3) prices *plant tissue*: leaf, litter, seed, moss. Those numbers are
   bounded by plant-side regrowth rates — S3's own sessile-grazer test
   (c) makes moss's value a function of moss's regrowth, and the same
   arithmetic will bind grass. The values should be set with the plant
   economy in view, not authored creature-side and discovered wrong.
3. **Herbivory as plant damage.** §13m already paid for the seam once
   (one bite killed the tree). When creatures eat at scale, grazing
   becomes a plant mortality term — which §5c wants *anyway* as a
   turnover engine. The plant side should state what a grazed cell does
   (regrow? scar? trigger nothing?) before S5 makes herbivores common.
4. **One persistence harness.** `population-dynamics-research.md` §12,
   verbatim: if both a plant ecology and a creature ecology are built,
   they share one persistence-testing harness — ensemble over ≥20 seeds,
   persistence not population as the metric, order-statistic gates. The
   plant megastudy and the creature `creature_space` sweeps are halfway
   to being that harness from opposite ends.

And one warning imported whole: **grass is an enrichment event.**
Rosenzweig's paradox (pop-dynamics §5) says every improvement to the prey
food base destabilizes the predator–prey system that feeds on it; the
report's acceptance test 9e (double prey food, persistence must hold) will
fire the day a grass layer lands under an ant economy. Expect it; do not
let the failure be blamed on whatever shipped most recently.

---

## 3. The materials question: how a plant stops being wood

### 3a. What is actually there (verified by reading)

A `Grow` species' materials are decided in code at exactly three seeding
sites, and everywhere else growth **copies the parent's material** —
deliberately, per the load-bearing comment at plant.rs:3845–3851 ("no
cell-type-to-material table anywhere"):

| Site | Line | Material |
|---|---|---|
| `germinate()` — shoot | plant.rs:3800 | `id_of("wood")` |
| `germinate()` — companion root | plant.rs:3852 | `id_of("rootwood")` |
| `Grow` arm — leaf cluster | plant.rs:2195 | `id_of("leaf")` |
| `Reproduce`/`set_seed` | plant.rs:745 | `id_of("seed")` (hard requirement) |
| `plant_moss_seed` | plant.rs:4174 | `id_of("moss")` — the separate `Divide` planting path: live `GrowingTip` placed directly, no `Seed`, no `Germinate`, no companion root |

Dead tissue needs no table either: `structural::break_free`
(structural.rs:570–590) reads the per-material `breaks_into`, and wood,
rootwood and leaf all declare `"deadwood"`.

Consequence, measured in the materials brief at 8 plants / 30,000 frames /
worldseed=1: tree, shrub and conifer are three number-sets on one material
vocabulary — 15,887 / 27,048 / 41,399 cells, heights 80 / 158 / 189, and
essentially one composition. The species `.ron`s confirm it structurally:
all three declare the same six cell types with identical economy blocks
(deliberately — "a species that silently sat out the economy would look
like a morphology result"), differing in architecture numbers only. Moss
escapes by being a different *behaviour* on a different planting path, not
by being a different material strategy.

### 3b. Grow or Divide? — answered from what each expresses

Read from organism.rs (the `Divide`/`Grow` split doc, and the field
inventory):

- **`Divide`** is a 4-neighbour mat: uniform candidate, moisture-keyed
  chance, optional shade sensitivity. It has **no** tips, no branching,
  no roots, no leaves, no turgor ceiling, no thickening, no seeds — and
  critically **no genome expression**: `genotype_variance` is a `Grow`
  field, moss's is all zeroes, and a `Divide`-only species has no
  consumer for a single continuous slot. Moss also carries no
  `Photosynthesize`, so it sits outside the carbon and water economies
  entirely — which is exactly why creature-plan E3 had to reject
  carbon-derived food value.
- **`Grow`** is the whole plant model: directed tips, branching,
  `ByOrder` tiers, plastochron leaves, roots with penetration and
  tropism, turgor height ceiling, bud banks, juvenile stage, genotype
  variance — every heritable trait in the signed-off map is a `Grow`
  consumer or a whole-plant pass over a `Grow` organism.

**So the fork resolves cleanly: any plant that should participate in the
economy, the genome, or selection is a `Grow` species. `Divide` is the
mat primitive — right for moss and future crusts/algae, and a `Divide`
species is currently, by construction, outside evolution.** A grass is a
`Grow` species with different numbers *and different materials* — which
is the part §3c has to supply, because today it cannot not have wood.

The engine's own comments already anticipate the degenerate settings a
herbaceous `Grow` species needs: `plastochron: 0` is "a real value" for a
species whose photosynthetic surface is its shoot (the cactus sketch,
organism.rs:294–298); `turgor_per_cell: 0.0` legitimately disables the
height gate ("a moss mat or a vine", organism.rs:566–570);
`heading_inertia` near 0 "is the difference between a habit that reads as
woody and one that reads as a creeper" (organism.rs:313–316). The knobs
were built general; only the materials are welded.

### 3c. What a species should declare: three materials, defaulted

Proposal: `SpeciesDef` gains three optional material names —

```ron
shoot_material: "wood",      // default
root_material:  "rootwood",  // default
leaf_material:  "leaf",      // default
```

— read at exactly the three seeding sites in §3a. Defaults mean every
shipped `.ron` is untouched and byte-identical in behaviour. Propagation
by parent-copy is preserved untouched, so this is *not* the
cell-type-to-material table the original comment forbids: it is the three
seeding constants that comment already relies on, moved from code to
data. Seed material stays shared (`"seed"`) until a dispersal axis exists
(§5d); dead tissue stays per-material via `breaks_into`, so "what is dead
grass" is answered where "what is dead wood" already is — in the
material file.

This is the entire engine change the plant-materials question needs. The
rest is materials and species data.

### 3d. Which per-material flags carry the meaning

The herbaceous material set, argued from flags that already exist and
already do things (values illustrative, set at landing):

- **`grassblade`** (Plant): the shoot *and* the photosynthetic surface —
  low density (~0.3), `flammability` high (~0.7) with a short burn — this
  is the fuel-bed property that makes grassfire a fast surface regime
  distinct from a canopy fire; `breaks_into: "litter"`;
  `max_cantilever_reach` mostly moot at blade scale (a 5–10 cell plant
  barely engages the cantilever rule) — the honest statement is that
  "flop vs break" is not expressible, bending does not exist, and the
  graded outcome this engine *can* give is break-into-litter.
- **`grassroot`** (Plant): **`reinforces_powder: true`** — this is the
  flag with the ecological payload. Rootwood already holds slopes
  (rootwood.ron declares it; `update_powder` gates on it), so grass
  binding a bank is emergent, not written — and §8c's measured fibrous
  mat (root cells with 3+ root neighbours 10% → 55% when branching is
  affordable) is exactly the architecture a sod wants, which is why the
  slot-1 economy repair (already with the owner) stops being a
  tree-genetics detail and becomes the grass milestone's prerequisite.
- **`litter`** (Powder): shared with creature S4 — light, flammable,
  weathers to soil on the decay schedule, carries `food_energy` when S3
  lands. The plant side's abscission writes it instead of `Cell::EMPTY`.

### 3e. The genome on a non-woody plant — mostly already correct

Checked, because the brief asked: `LOCUS_WOOD_DENSITY` reads as *tissue
density* with no wood anywhere in the mechanism — it scales `Grow.cost`
and `max_cantilever_reach` through `organism::wood_density`, all four
call sites take it from the alleles, and the two degradation paths hold:
`bark_band_for_density` returns 0 for `count == 0` (organism.rs:1270–1276),
and the no-finite-span sentinel is read off the raw material, never the
scaled span (structural.rs:415–426), so a material opted out of the
cantilever rule stays opted out at every allele. A grass declares its own
variance vector and simply zeroes what it does not use — the slot map
renumbers nothing, exactly as designed. The one locus that needs species
thought is `LOCUS_LEAF_ECONOMY`: on a species whose shoot is its leaf,
the rate/transpiration pair applies to the blade material's
`Photosynthesize` entry, and the band consumer works unchanged.

---

## 4. Which plants are worth having

Each candidate against the project's tests — does it change what a cell
*does*; which pixels move; what does it trade against — and against the
niche axes the engine can already stage (wet/dry via `soil=N`,
light/shade via canopy, substrate via penetration resistance, and fire).

1. **Grass — first, and the recommendation.** Cells do differently: burns
   fast, litters, binds powder, photosynthesizes at the shoot. Pixels: a
   green ground layer where today there is bare soil — the largest single
   silhouette change available for the cost, plus fire fronts and held
   banks. Trades: shallow water draw versus the trees' deep draw (slots 5
   and 6 finally get their second species), fire risk versus growth rate.
   And it is the creature economy's ground-level food (§0).
2. **Cactus/succulent — second.** The dry niche already means something
   (the §8d crossover), `stomatal_reserve` is already a species scalar,
   and the photosynthetic-shoot pattern is anticipated in the code. One
   real seam: water capacity is root-mass-keyed (`water_capacity_of` ∝
   `root_cells`), and a succulent's whole point is *shoot* storage — it
   needs a species term in that function. One function, one field; but it
   is a mechanism change, so it is second, not first.
3. **Reed/wetland — deferred.** The niche exists (the moss census shows
   wetland is where ground flora thrives) and anoxia tolerance is already
   per-species, but the trade is thin — what does anoxia tolerance cost?
   Until that has an answer, it fails test 2 the way slots 1 and 5 died.
4. **Vine — deferred, and it is an owner-level aesthetic call.** The
   mechanism gap is real (a vine wants surface-attraction, a new `Grow`
   term; `heading_inertia: 0` alone gives a creeper, not a climber), and
   a vine deliberately violates the "you should never see a plant growing
   on another plant" outcome the wiki holds up as the most visible rule —
   the epiphyte-guard history says that line was fought for.

---

## 4a. The Grow body plan: the expressible envelope

§4 picked plants. This section maps the space they were picked *from*,
because the owner's question — can the forms themselves emerge, and how
do we set this up to make a world worth exploring — is a question about
that space, not about any one species.

### What the machinery actually is

A `Grow` organism is four coupled things, and every plant form is a
corner of their joint parameter space:

1. **A scored random walk.** Tips score every open 8-neighbour by a
   weighted blend — continuation, light, wind, upward bias, tropism
   reference — and *sample* from the distribution. The noise is
   load-bearing: it is why twelve identical genomes span 31–153 cells,
   and why no two trees match.
2. **A tier grammar.** Every architectural weight is a `ByOrder` array —
   re-parameterized per branch order. This is the engine's answer to an
   L-system's productions, and it is the shape language: a conifer is
   "orthotropic order 0, plagiotropic above"; a shrub is "sympodial with
   basitonic bud release." Two entries in a table, not two mechanisms.
3. **An economy.** Carbon from foliage against light, water from roots
   against soil, a turgor height ceiling, pipe-model thickening, bud
   banks, seed provisioning. The economy is what makes a form *earned* —
   a shape that cannot pay for itself sheds and shrinks until it can,
   which is why habit reads as alive rather than drawn.
4. **A material body.** Tissue physics — density, flammability, span,
   `breaks_into`, `reinforces_powder` — decide what the form *does* to
   the world around it and what the world does back.

### The corners, with their distance from here

| Form | What expresses it | Status |
|---|---|---|
| broadleaf / conifer / shrub | shipped `.ron`s | **shipped** |
| tussock grass | `plastochron: 0` (photosynthetic shoot — anticipated at organism.rs:294–298), `turgor_source` ~0.1, herbaceous materials | one `.ron` after call 1 |
| creeper / scrambler | `heading_inertia` ~0 ("the difference between woody and creeper", organism.rs:313–316), low upward weight | **values only** — expressible today, never staged |
| weeping habit | negative `upward_weight` on later orders | verify the scorer accepts a signed weight at landing; if it clamps, this is a one-line range widening, not a mechanism |
| prostrate mat / cushion | `Plagiotropic` at order 0, tiny internode, low turgor | small code change — the tropism tier currently applies at order > 0 only |
| columnar succulent | `plastochron: 0` + shoot water storage | one seam: `water_capacity_of` ∝ `root_cells` needs a species term (§4.2) |
| reed | per-species anoxia tolerance + the waterline | parameter exists; the trade is thin (§4.3) |
| liana / climber | surface attraction | **the one true mechanism gap** in the envelope |
| epiphyte | — | deliberately banned (the wiki's most visible rule) |

The striking thing about this table is how short its right-hand column's
tail is: of nine recognisable plant forms, six are values of knobs that
already exist, one is a seam, one is a mechanism, one is banned.

**But — corrected by the owner, and the correction is right — that is a
reachability claim on paper, and what has actually been *seen* is three
slightly different trees of similar vibe.** The appearance phase already
proved that architecture numbers alone do not read (sympody, tropism and
acrotony all fired and moved nothing anyone could see), and shrub /
broadleaf / conifer were authored close together on the axes that
matter. At pixel scale there are about **four axes a form difference can
actually read on**, and the shipped species differ meaningfully on none
of them:

1. **Size class** — order of magnitude, not percent. Height 80 vs 189
   is one class; a 6-cell tussock against a 150-row tree is a different
   kind of object.
2. **Material and palette** — green blade vs brown wood; what it burns
   like, breaks into, does to the soil under it.
3. **Form class** — upright crown vs mound vs prostrate mat vs hanging.
   Broadleaf and conifer are both "upright with a crown"; a cushion or
   a weeping habit is a different silhouette *category*.
4. **Consequence** — carries fire, holds a bank, litters, feeds
   something.

Grass differs on all four, which is the real reason it leads §4.
Creeper, weeping and prostrate each claim a new **form class**; the
claim is untested, and the burden sits on measurement: **the envelope is
not "wide" until at least two of the form probes below read as new
categories by eye.** Any probe that comes back looking like another
small tree goes to the dead-end register before it costs a milestone —
that is one filmstrip against one `.ron`, the cheapest experiment this
document proposes (staged as P1a in §7).

### The register, filled in: WP-C's three probes — OWNER-JUDGED (2026-08-21)

All three built and rendered — one grove sheet each at noon (frames 10,800 /
18,000 / 25,200), against a `tree` sheet re-rendered in the same batch as the
paired control — and **all three judged by the owner through the review
queue**, the two closest calls posted blind because the session had a stake in
the answer. The verdicts, verbatim:

| Card | Verdict | Rating |
|---|---|---|
| Form probes (gallery) | *"Tree and weeping look the same. creeper and prostrate look the same"* | — |
| `tree` vs `weeping` (**blind**) | *"same plant"* | — |
| `creeper` vs `prostrate` (**blind**) | *"Not that different"* | 2 |

| Probe | Cost | Outcome |
|---|---|---|
| **creeper** | values only, no code | **Kept.** Reads as its own group, apart from the tree group — the strongest claim the verdict supports. |
| **prostrate** | 4-line `plant.rs` change | **RETIRED**, file and code both. Not separable from creeper, which needed no code. |
| **weeping** | one `.ron` line, no code | **RETIRED.** *"same plant"* as `tree` — a within-class variation. |

**Tally: at most one, against a bar of two. The envelope claim is not
validated, and the owner's original pushback stands.** Note the tally is
generous to itself: the gallery verdict groups {tree, weeping} against
{creeper, prostrate} and so *implies* creeper is a different kind of thing
from a tree, but it never says so outright. Nothing here certifies a second
new class, and the one probe that survives does so on an inference.

**The three probes accidentally formed a controlled comparison, and that is
the part worth keeping.** Sort them by which axis each one moved:

- creeper and prostrate both cut `turgor_source` to a 5–8 row budget (an
  **axis-1 size-class** move) and both landed in the same group. They differ
  from each other *only* on form knobs — `heading_inertia` 0.05 vs 0.1, and
  order-0 `Plagiotropic` vs not — and the owner could not tell them apart.
- weeping kept `tree`'s `turgor_source: 1.0` and moved **only** a form knob
  (`upward_weight` negative on orders >= 1). It stayed in the tree group.

So across three probes, **every group change came from the size budget and
none came from an architectural knob.** This is the appearance phase's finding
arriving again from a new direction (CLAUDE.md, "ask which *pixels* a lever
moves"): sympody, tropism and acrotony all fired and moved nothing visible,
because they change *which cell gets a label*. Negative `upward_weight` and
order-0 `Plagiotropic` are levers of exactly that kind, and they behaved
exactly that way. `turgor_source` is not — it sets how much plant there is.

**Consequence for the corner table above: a corner named by an architectural
knob should be assumed not to read until shown otherwise; a corner named by a
size or material change should be assumed to read.** Grass leads §4 because it
moves size *and* material *and* consequence — and its own card came back
*"looks different from trees"* at 4/5, which is the first thing in this
programme to clear the bar by eye.

**What was withdrawn, and what would change the answer.** The order-0 tropism
change let a species' founding axis be plagiotropic from its species file
(never from an allele — no point mutation may lay a trunk over). It was
behaviour-free for everything shipped, proven not assumed: the `tree` grove
sheet was byte-identical across it, md5
`35f6147408e8ff75ce865b38697961fc`. It went because the only form it unlocked
was one pure values already reached. Reviving it needs a sheet showing
order-0 plagiotropy doing something `heading_inertia` and a low turgor budget
cannot — not the reachability argument, which was sound and still lost.

`weeping` is worth one line of memory: negative `upward_weight` **is**
expressible today, was never clamped anywhere in `src/sim`, and does make
laterals hang. It simply does not change what kind of thing the plant is at
this scale. If WP-E makes foliage a mass rather than a garnish, that is the
event that would justify re-authoring it — the droop had nothing to drape.

### Within a form class: cone against dome, and the two channels the engine lacks

The four axes above are the *between*-class story. The genome session's
handover (`Reports/plant-work-split.md` §1, from the owner's own
playtest reading — "they still look pretty similar; look at a real
conifer versus an oak") diagnoses the *within*-class one: a spruce is a
cone of solid needle plumes on one unbroken leader, tiers longer the
older they are; an oak is a dome of foliage shell over codominant limbs.
Both shipped species are an irregular tangle of brown sticks with green
speckle, and no setting of the existing knobs fixes that, because two
channels are missing and one is welded:

- **Foliage as mass, not garnish.** Leaf fraction is 26–31% and renders
  as dots strung on branches, so the wood skeleton — generated by the
  same rules for every species — sets every outline, and they converge.
  This is the sympody-phase root cause surviving at 28% leaf what it was
  first diagnosed at 5%.
- **An age channel.** A conifer's taper is laterals longer the older
  they are; an oak's codominant split is an age/size-triggered event.
  Every branch parameter here keys on `ByOrder`, and `tree.ron`'s own
  comment concedes the gap: order is position in the plant, not age.
  Without age there is no length grading, so no cone — and no dome.
- **Species materials** — cause (c) in the handover, which is this
  document's call 1, arrived at independently by both sessions.

The two analyses compose rather than compete: materials and the form
probes buy *between*-class diversity (grass is not a small tree);
foliage mass and age buy *within*-class fidelity (a conifer is not an
oak). Call 9 sequences them.

### The world this makes — the payoff, stated as the ethos demands

What is unique about this engine's version of plant life, if the envelope
is wired up as below, is that **the landscape becomes legible** — every
vista is information, because appearance and form derive from strategy
and history rather than decoration:

- **You can read the ground by the plants.** Pale-thrifty foliage on the
  ridges, dark-acquisitive in the wet valleys — already true and
  measured (§8d's crossover). Pale pioneer bark ringing a burn scar,
  dark dense-wood stands where nothing has disturbed in an age. Grass
  where fire and grazing recur, forest where water runs deep. Biomes are
  *consequences* of one species pool meeting different ground — no biome
  is authored, so their boundaries move when the player moves the water
  or starts the fire.
- **Succession is the spectacle and disturbance is the player's verb.**
  Burn, cut, dam, dig — all exist. A burn scar → ash → soil → grass →
  pioneers → shaded out by dense wood is a story the player *causes*
  and then watches, and every frame of it is the economy working, not a
  script.
- **Breeding is emergent play, at zero code.** Seeds carry genomes;
  heredity is live. A player who collects seeds from an individual they
  like and plants them elsewhere is doing artificial selection — the
  oldest human game with plants, and it falls out of machinery that
  already ships. Nobody has to build "domestication."
- **Long-run drama.** Leave a world running; the allele census drifts,
  the bands shift, the proportions change. The world you come back to
  is measurably not the one you left, and the difference is visible
  before it is numerical.

### Setup principles — how to wire the envelope for this

1. **Every form dimension reads the genome or the environment; no dead
   authored numbers.** Each `ByOrder` entry should eventually be
   genome-scalable the way branch chance and plastochron already are —
   a knob evolution and habitat cannot touch is a knob the world cannot
   vary, and the appearance phase proved what invariant composition
   looks like.
2. **Signed and zero-inclusive ranges over new mechanisms.** Weeping,
   prostrate and blade forms are all *boundary values* of existing
   knobs. Widen a range before writing a term.
3. **The legibility law.** Every strategy axis gets a visible face — a
   band or a silhouette — per `plant-appearance-design.md` §7. A form
   that emerges but moves no pixels has not emerged for the player.
4. **Niches before forms, disturbance before niches** (§5c). The
   envelope supplies the *possibility* of forms; only the world supplies
   the reason for more than one.
5. **The connectedness rule (load-bearing for §6a):** author every new
   species as a point in the *shared* space — same slots, different
   values — rather than through species-only mechanisms. Two islands
   cannot be walked between; two points in one space can.
6. **One new mechanism at a time**, only when a corner is provably
   unreachable by values (surface attraction, shoot storage), and each
   arrives with its trade already named or it dies the slot-1/slot-5
   death.

---

## 4b. What the side-view orientation costs and buys

Asked by the owner directly: how limited is this by the 2D side-scroller
orientation — are forest fires or rivers even possible? Answered per
phenomenon, because the orientation's costs are real but they are not
where they first appear to be.

**Forest fires: essentially already built; what is missing is the
regime, and grass is its unlock.** Fire spread, per-material flammability
and burn times, wood → ash, and the full recovery loop (ash decays to
soil when damp, soil reseeds — the wiki's "a burnt patch grows back on
its own") are shipped; `emergent-world-architecture.md` calls fire the
engine's one closed loop and names the open half as fire-as-*regime* —
recurrence, natural firebreaks, patchy mosaics. A herbaceous fuel bed
makes grassfire and crown fire two visibly different phenomena, and the
wind field (already wired for canopy lean) is the obvious driver for
spread asymmetry. Side view is *good* for fire: flames climbing a slope,
jumping a canopy gap, racing the surface while the wet valley stops
them — all legible in cross-section.

**Rivers: the cross-section of a watershed is reachable; the map of one
is not.** Side view cannot give plan-view river phenomena — meanders,
braiding, a network seen from above. What it can give is the hydrology
this engine's parts already point at: rain (weather exists) on ridges,
infiltration, a water table, springs where the table meets a slope face,
streams cascading downhill, waterfalls off ledges, lakes filling basins,
floods when a dam breaks — and the evaporation → rain matter cycle the
architecture doc names as one of the two worth closing. M10's wider
worlds give long profiles: source in the hills, cascade, valley lake.
Two costs stated per house rules: a *continuously flowing* river keeps
every chunk along its length awake, which collides head-on with the
frame budget — the liquid heightfield bodies sitting test-only in
`liquid.rs` exist for exactly "large water, cheap," and something like
their promotion is probably the price of a standing river; and moving
water as a *force* (carrying seeds, cutting banks) is unbuilt, though
erosion is on the architecture doc's list for the
deposit–diffuse–decay–follow primitive.

**The reframe: side view trades away the map plane and owns the vertical
axis — and the vertical axis is where plant ecology and hydrology
actually live.** Light from above, water from below, soil depth, roots
against the water table, canopy strata, undergrowth shade, cliff faces,
caves and aquifers. No top-down game can show a root system drinking a
bed dry beneath a tree whose crown is shading out a seedling — this
engine already does, measured (§8d). The orientation genuinely hurts the
*creature* side (the §13b sensor-geometry finding: surface-walking broke
the open-2D trail-following triad); plants are the kingdom it favours.

---

## 5. Plant ecology: what exists, what is missing

### 5a. Already present and measured — more than either plan credits

- **Light competition:** shading economics, overtopping, self-pruning,
  the canopy seed-death rule — the wiki page's whole "healthy stand"
  section is competition made visible.
- **Water competition:** the §8d finding that an acquisitive stand
  leaves its bed at wilting point (median soil 187 vs 620) is
  *resource-depletion competition*, unplanned, already emergent — one
  stand changing the environment the next generation establishes into.
- **Establishment as the selection filter:** most seedlings die (5/16
  and 4/16 leafless at 45k frames in the study; the wiki: "most
  seedlings are lost in the first moments"), and seed endowment plumbing
  makes the filter heritable-adjacent already.
- **A closed matter cycle:** burn → ash → soil (damp) → reseed. The
  burnt patch regrows. This is the disturbance loop §5c leans on.
- **Root–soil coupling:** reinforced banks, root displacement,
  hydrotropism.

### 5b. The population-dynamics constraints, applied to plants

The report was written for creatures; three of its findings bind here:

- **Space is the stabilizer and plants get it maximally** — sessile
  individuals are the zero-mobility limit, so plant ecologies sit on the
  stable side of the mobility threshold by construction. The leading
  indicator transfers: per-chunk density variation (9d). A plant world
  where every chunk looks the same is on borrowed time.
- **Ensembles, not runs** (§8): a parameter set with a 30% extinction
  rate looks fine three times. Plant persistence claims get the same
  ≥20-seed, order-statistic treatment — and the megastudy is most of
  that harness already.
- **Enrichment destabilizes** (§5): every plant improvement enriches
  whatever eats plants. Standing, not actionable until the kingdoms
  couple.

### 5c. The missing piece: turnover

**Evolution needs generations, and a closed canopy has none.** Nothing in
the engine kills a healthy adult plant: no senescence, no disease, no
storm mortality beyond load events, and (until creatures eat at scale) no
herbivory. Selection therefore acts only at establishment, and once a
stand closes, the population freezes — the founding cohort *is* the
population for the rest of the run.

Two ways to buy turnover, and they are not equivalent here:

- **Authored mortality** (a lifespan, a senescence clock) — cheap,
  reliable, and exactly the kind of outcome-authoring
  `design-philosophy.md` §2b warns on. A lifespan is defensible as
  biology, but it is a knob that decides the succession rate rather than
  letting anything decide it.
- **Emergent mortality** — fire regimes through the grass layer, load
  and burial, drought years (a weather cycle the moisture system can
  already stage), herbivory when it arrives. Gap dynamics: a disturbance
  opens canopy, the establishment filter runs again in the gap, and the
  pioneer/dense wood-density trade finally has both sides live — cheap
  wood wins the race into gaps (+46% mass, +52% seed, measured), dense
  wood survives what makes gaps.

The recommendation is the emergent route, because it is the one where the
wood-density locus stops saturating: with no disturbance, nothing ever
selects *for* dense wood at stand scale, and §8's own rule says a locus
with only one live side stops being variation. But this is a real design
call (§8, call 6) — the emergent route makes succession rate a property
of the fire/weather tuning, which is harder to steer than a lifespan.

### 5d. Dispersal, named honestly

The genome design deferred seed strategy partly because "the engine has
no dispersal axis — a seed's physics are identical whatever the allele."
That is also an *ecology* gap: recruitment is wherever a seed rolls,
which is next to the parent. Local dispersal plus spatial niches is what
makes divergence *parapatric* — it helps speciation (neighbourhoods breed
true) — but it also means a lineage cannot reach a distant niche. The
cheap future lever is per-species seed material (density, wind
interaction — the wind field is already wired for canopy lean), which
would also hand the deferred `LOCUS_SEED_STRATEGY` its second axis.
Recorded, not scheduled.

---

## 6. Speciation: what it can mean here, and how to know it happened

**There is no sexual reproduction anywhere in either plan, and that
decides what "species" means.** With clonal inheritance there is no gene
flow, so there is nothing to isolate — the creature plan's dead-end
register already caught a proposal for lineage-tag mating rules on
exactly this ground ("asexual budding *is* the isolation"). A "species"
in this engine is therefore not a reproductive community; it is a
**persistent, self-maintaining cluster in genotype space with a niche it
holds** — a strategy that stays distinct because selection keeps it
distinct, not because mating boundaries do.

That definition is operational, and every term of it is measurable with
instruments one side or the other has already built:

1. **Cluster:** multimodal allele frequencies / draw distributions in
   the standing population — the probe's allele census, read against the
   published clonal drift band for that N (§1d), because at stand-scale
   populations one lineage at share 1.0 may be the *null*.
2. **Niche fidelity:** the modes associate with habitat — allele
   frequency by wet/dry, by depth-of-soil, by canopy/gap, with spatial
   separation controls at equal genotypes (the creature S5 control:
   identical genomes must read separation ≈ 0).
3. **The instrument that tells divergence from drift: the reciprocal
   transplant** (creature S7, adopted whole). Take the extreme genotypes
   by the axis under test; establish each in both habitats; the 2×2 must
   be asymmetric — home minus away positive for both — in ≥6 of 8 seeds,
   with the single-habitat control producing a symmetric 2×2. A bimodal
   histogram with a symmetric transplant is drift wearing divergence's
   clothes, and it is precisely what a histogram alone reports as
   success.
4. **Persistence:** the clusters hold across generations and across the
   ensemble, not within one founding cohort.

**What plants already have toward this bar, that creatures do not:** the
raw material is proven — the leaf-economy crossover *is* criterion 2's
content at n=1 seed, measured on pinned monocultures; heredity is live;
discrete loci make clusters representable by construction rather than
hoping a continuous axis goes bimodal. **What plants lack:** generations
(§5c), population plumbing (P0 below), and any experiment yet run on an
*evolving* population rather than pinned monocultures. The first real
speciation experiment is cheap to state: seed a mixed-allele stand across
a wet/dry world, run long enough for descendant establishment to dominate
founders, and read criteria 1–3. Whether "long enough" is even reachable
inside current run lengths is unknown — which is why P0's generation
counter comes first ("did it fire" before "did it work": the count of
*inherited-genome establishments* per run is the plant equivalent of
births-per-generation, and if it reads ~0 at 30k frames, every evolution
claim at that horizon is about founders).

Speciation is reachable here in the same qualified sense the creature
plan's §7 grants open-endedness: **the encoding is not the blocker; the
world is.** The genome can already represent two grasses or two trees
that live differently. What decides whether they *stay* two things is
niche supply (both sides of each trade live in the world at once) and
turnover (selection keeps running after canopy closure). Both are ecology
work, and both are the same ecology work the creature programme needs.

---

## 6a. Can the forms themselves emerge? — the framework assessment

Asked by the owner directly: authoring grass and cactus looks like
prespecifying the outcome — is a framework where those things *emerge*
just not possible? The answer, mirroring the shape of the creature
plan's §7 (which was asked, and answered, the same question about
open-endedness): **partially now, mostly later, and the sequencing is
the whole trick.** Three facts and one rule.

**Fact 1 — today's genome is a neighbourhood, not a morphospace.** Every
locus is a bounded multiplier on the species file's authored numbers
(turgor ±18%, branch ±50%, density ×0.75–1.35). Evolution can make a
tree shorter, denser, thriftier, deeper-rooted; it cannot walk a tree to
a grass, because grass-ness sits outside every variance fence, some of
its axes have no slot at all (reproduction rate foremost), and the
materials are welded in code until call 1 lands. But §4a's table shows
the space itself is connected: most of what distinguishes the forms is
*already a continuous axis the genome touches* — height, leaf spacing,
root architecture, tissue density, water strategy. The fences and the
missing slots are the blockers, and neither is architectural.

**Fact 2 — the body plan itself stays caged, on purpose.** Which cell
types exist, which behaviours they carry, `Grow` versus `Divide` — that
is the discrete *structure* of the developmental program, and evolving
structure is the NEAT-shaped thing the creature side deliberately caged
(D4: variable-length genomes, speciation machinery, illegible results —
"topology is what got caged, not the brain"). Forms **within** the Grow
body plan can emerge; the plan itself is authored. Reopening that cage
is a legitimate future call, but it was made twice with paid-for
reasons, and nothing in §4a's table needs it — the envelope's
unreachable corners are one seam and one mechanism, not a topology.

**Fact 3 — an open morphospace without niches produces one weed.** The
multi-task fitness warning (`PLAN.md`'s evolution notes) and
`population-dynamics-research.md` §6 say the same thing from two fields:
with one binding task, selection collapses everything onto one optimum.
Dry ground where thrift wins, disturbance that rewards short-and-fast,
shade that rewards tall-and-patient — those must exist *and be verified
to support life* before evolution can be asked to discover the forms
that fill them. This is why authored species come first and why doing so
is not a betrayal of emergence: **an authored grass is an ancestor and a
niche probe, not a product** (the plant-side E2). If hand-tuned grass
cannot persist on dry disturbed ground, evolved grass never had a
chance — and the authored species tells you in one run, where an
evolutionary run failing is ambiguous between morphospace, niche,
horizon and drift. The authored species de-confounds the framework.

**The rule that keeps emergence reachable: connectedness (§4a principle
5).** Every new species is authored as a point in the shared slot space
— the same loci at different values, materials as data, mechanisms only
for provably unreachable corners. Then the staged widenings (missing
axes get appended slots; variance fences widen where measurement says
the lethal zone allows; `ByOrder` entries become genome-scalable) each
enlarge the *walkable* space without renumbering anything, and the
species files decay from definitions into founding points.

**The bar, and it is falsifiable: delete `grass.ron` and see if grass
comes back.** Run an evolving tree-ancestor population in a world with
fire, drought and grazing, at the horizons P0's generation counter
certifies as multi-generational; if short, herbaceous, fast-reproducing
forms emerge in the disturbed niches without anyone authoring them, the
framework is real. If not, the failure localizes to whichever ingredient
the stages have not landed — which is precisely what the creature plan's
§7 assessment does for open-endedness, and what a
straight-to-emergence build could never diagnose. Nothing is lost by
starting authored: the ecology work (turnover, niches, generations) is
identical on both routes, and it is the long pole on either.

---

## 7. Staging

Ordered so every stage is independently shippable and judged by eye, per
both plans' discipline. Plant-side stages P; creature stages S referenced
where they interlock.

- **P0 — population plumbing (prerequisite, not a milestone).** Plant
  `free_organism` (the free side exists and nothing calls it for plants;
  a BFS-from-roots liveness check is the known missing piece — §13m and
  issue #8 both name it), seed decay (the immortal seed bank), and a
  **generation counter** in the probe: founders vs inherited
  establishments per run. A grass layer multiplies organism count and
  seed count; landing it on a leaking substrate turns both leaks into
  ceilings. The 4095-organism ceiling is the number to watch.
- **P1 — species-declared materials + the herbaceous material set +
  litter.** §3c's three fields (defaults keep every tree byte-identical);
  `grassblade`/`grassroot`/`litter` materials; abscission writes litter
  instead of `Cell::EMPTY`. **Coordinate litter with creature S4 — one
  material, landed once.** Costs stated: the settled-forest frame
  measurement is part of this stage, not a follow-up.
- **P1a — form probes (cheap, can run alongside P1).** Creeper, weeping
  and prostrate as one `.ron` + one filmstrip each, on existing knobs
  (§4a's table — weeping needs the scorer's sign checked first,
  prostrate the order-0 tropism change). Judged by eye against the
  four-axes bar: does it read as a new form *class* or as another small
  tree? Two successes validate the envelope claim; failures go to the
  dead-end register at filmstrip cost instead of milestone cost.
- **P2 — grass.** One new `.ron` on the P1 materials; blade
  photosynthesis, shallow branched roots (riding the slot-1 economy
  repair), bank-holding, fire-carrying. Judged by eye: ground cover
  where there was bare soil, a bank that holds, a fire that runs the
  surface. Counted: surface-edible cells within three rows of ground
  (the creature metric, today 0–11), paired slope-loss with and without
  sod, burn-front speed on grass vs canopy.
- **P3 — the divergence instruments.** Generation-aware long runs;
  allele-frequency trajectories with the drift band published first;
  the plant reciprocal transplant; the shared persistence harness
  (pop-dynamics §8/§12) serving both kingdoms.
- **P4 — turnover and succession.** The §5c call executed: gap dynamics
  measured (what opens canopy, at what rate, at what seed sweep), the
  wood-density locus read across a disturbance gradient — the first
  test of whether both sides of a strategy trade can stay alive in one
  world, which is criterion 0 for speciation.
- **P5 — the speciation experiment** (§6), run against the operational
  bar, on evolving populations, ensemble-gated. Interlocks with S3–S5
  arriving from the creature side: grazing pressure becomes a plant
  selection axis, grass value becomes a creature niche axis, and the
  enrichment guard (9e) runs at every coupling step.

Not staged, recorded: dispersal (§5d), moss overhaul (§8 call 4), vine
(§4.4), the sublinear allometry temptation (banned creature-side for
making the outcome variable heritable; the same trap exists for any
plant "efficiency with size" term).

---

## 8. NUMBERED CALLS FOR THE OWNER

Where this document cites something as "settled", "signed off" or
"deliberately not done", that is provenance — a record of who decided and
on what evidence — not a fence. Any call below may reopen the decision
underneath it, and the owner has said directly that nothing documented is
beyond discussion.

1. **Species-declared materials (§3c).** Three optional fields —
   `shoot_material`, `root_material`, `leaf_material` — defaulted to
   wood/rootwood/leaf, read at the three seeding sites, propagation
   by parent-copy untouched. This is the entire engine change that lets
   a plant not be wood. Recommended: yes.
2. **The first non-tree plant is grass — authored as an ancestor and a
   niche probe, not a product (§4, §6a).** It ships as a point in the
   shared slot space (connectedness rule), it verifies that the
   disturbed-ground niche can support life at all, and it is the
   creature economy's ground layer (§0) — three jobs, one `.ron`.
   Cactus second (one mechanism seam: shoot water storage), reed and
   vine deferred with reasons.
3. **Litter is one material, landed once (§2.1).** Plant abscission
   stops deleting leaves; creature S4 consumes the same material; the
   two branches coordinate on who lands it and when. Needs an owner
   call mainly because it sequences two live branches.
4. **Moss stays as-is for now.** It is outside the economy and the
   genome by construction (§3b), creature S3 prices it as static
   material data precisely because of that, and an overhaul (giving it
   `Photosynthesize`, a cost, genome expression) is real work with no
   current consumer. Recommended: defer until either the creature
   grazing tests demand a regrowth cooldown or a crust/mat species is
   actually wanted.
5. **Ratify the genome contract as the shared law, and retire D3's
   "one mechanism" reading (§1a).** The contract (positional-forever,
   per-slot widths, append-only, identity-keyed mutation) is satisfied
   twice over; the code stays parallel because the contents differ.
   Alternative: pursue literal code unification — a refactor with no
   pixel attached.
6. **Turnover comes from disturbance, not a lifespan (§5c).** Emergent
   mortality (fire through the grass layer, load, drought, herbivory)
   over an authored senescence clock. This decides P4's shape and is
   the precondition for the wood-density trade — and for any selection
   after canopy closure. The honest cost: succession rate becomes a
   property of fire/weather tuning rather than a knob.
7. **Founding-draw keying (§1c).** The documented rationale for
   position-keyed founders has partly expired: it dates from when the
   genotype was *regenerated* from its key, and draws are now stored on
   `OrganismState` (organism.rs:1027's own "why stored at all" note), so
   a future save format that persists organisms no longer needs the
   position key to reconstruct anything — what remains is same-seed
   founder determinism and the same-cell-resows-same-genotype quirk.
   Recommendation: before the §6 experiments run, either re-key founders
   to the organism handle (P-21's law, one keying change, no slot map
   impact), or keep position keying and let P0's generation counter
   show whether founders even matter at those horizons. My lean is
   re-key — the confound is cheap to remove and expensive to argue
   about in every future study — but it changes which individuals a
   given world seed produces, so it invalidates cross-comparison with
   every stand measured so far, which is why it is a call and not an
   edit.
8. **Adopt the connectedness rule and the emergence bar as standing law
   (§4a principle 5, §6a).** Every future plant species is authored
   through the shared slots at different values — species-only
   mechanisms need the same justification a new material does — and
   "delete `grass.ron` and see if grass comes back" is the framework's
   own long-run test, run when P0–P4 make it meaningful. This
   constrains how every species file is written from here on, which is
   why it is an owner call and not a convention a session adopts.
9. **The silhouette-channel order (answers `plant-work-split.md` §5.2):
   materials → foliage mass → age.** Materials first because it is not
   really competing — three fields, no effect on any shipped tree, and
   both programmes' ground layer waits on it; but stated honestly, it
   buys grass, not a better conifer. **Foliage mass second**: it is the
   twice-diagnosed root cause (5% leaf then, 28% now, same conclusion),
   it moves pixels in *every* species including the herbaceous ones,
   and its cost is known and composes — foliage carries transpiration,
   so more mass re-tunes the water economy and re-baselines the §8d
   crossover, which is the already-measured trade rather than a new
   one. Two flags for whoever builds it: `shade_death` bought bole
   legibility by cutting foliage 11,179 → 2,336 once before, and
   `tree.ron`'s sweep comment says 0.03 "<-- here" while the shipped
   value is 0.003 — deliberate retune or typo, check before sweeping.
   **Age third**, because without foliage mass a perfectly tapered
   skeleton is a cone of sticks, and because its consumer needs design
   before its storage: the storage is trivial (a birth-frame stamp in
   the `OrganismCell` sidecar, off the hot path), but *which object
   age grades* — a cell, a lateral, a tier — is exactly the
   which-object-does-this-rule-evaluate question this repo has paid
   for twice, and length-grading laterals means an age term in
   allocation, which touches the vein-polarity machinery. Propose it
   as its own short design note when its turn comes, not as a rider.

---

## 9. Provenance

Synthesized from: `Reports/creature-evolution-plan.md` (uncommitted,
`creatures-m18`) and `creature-direction.md` §0–§13; the signed-off
`plant-genome-design.md` with its §8 measurements;
`plant-appearance-design.md` §7; `plant-substrate-v2-design.md`;
`organism-substrate-design.md`; `population-dynamics-research.md`;
`design-philosophy.md`; `PLAN.md`'s evolution notes and M16/M18
milestones; `wiki/plants.md`; and a code audit of `plant.rs`,
`organism.rs`, `structural.rs`, the material and species `.ron`s at
`7b57f8f`. Amended after rebase onto `f5e653e` against the genome
session's handover `plant-work-split.md`, whose ownership split this
document accepts as amended in that file's own §6 and whose open
silhouette question call 9 answers. The three-way convergence in §0 (grass = plant milestone =
creature food base = fire fuel bed) is this document's claim, checked
against the creature plan's own numbers rather than adopted from any one
source.
