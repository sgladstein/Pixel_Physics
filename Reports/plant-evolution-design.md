# From trees to plants: materials, ecology, evolution, speciation

**Status:** design and comparison, 2026-08-18. No implementation. This
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
2. **The first non-tree plant is grass (§4).** Cactus second (one
   mechanism seam: shoot water storage), reed and vine deferred with
   reasons. Recommended: grass, and treat it as the shared
   ground-layer milestone of §0, not a plants-only feature.
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
`7b57f8f`. The three-way convergence in §0 (grass = plant milestone =
creature food base = fire fuel bed) is this document's claim, checked
against the creature plan's own numbers rather than adopted from any one
source.
