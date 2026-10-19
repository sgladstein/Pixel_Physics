# How much of a plant can a trainer reach? The plant engine, reviewed for training

*2026-10-19. Step 2 of 5 of the plant-trainer line: brainstorm
([`plant-training-brainstorm-2026-10-19.md`](plant-training-brainstorm-2026-10-19.md))
→ **this review** → plan
([`plant-trainer-plan-2026-10-19.md`](plant-trainer-plan-2026-10-19.md)) →
adversarial review → build. Written from `src/sim/plant.rs` (14,750 lines of
engine before its tests), `src/sim/organism.rs`, `assets/species/herb.ron`
and the plant reports, and checked against a probe built for it
(`examples/herb_morphs.rs`, §4). The owner's questions it answers: **how much
of a plant's look, growth pattern and visible behaviour is controllable by
evolution, how much is hardcoded, and how flexible the design is for new,
emergent forms.***

## 1. The answer, first

- **A trainer that edits species files (owner ruling 1) reaches most of what a
  plant is.** Height, branching, leaf amount, flower size and timing, root
  habit, seed economy and life history are all species-file numbers. herb
  alone authors **101 values** across ten behaviours, a fate table and twelve
  life-history and look fields. In the game itself, a plant can inherit only
  a much narrower slice of that, because mutation rates are low and the
  steps are small.
- **The silhouette range is wide and the genome sets it** (§4, measured).
  Sixteen random herb variants × three world seeds grew a thin stalk, leafy
  bushes, a round crown, a sprawling flowering mat and a tiny ground cover.
  Across them the variant explained **76% of the spread in plant size and
  66% in leaf share** (48–49% of height and width). For comparison, the
  shipped game's small per-individual jitter explains 0–5% of herb size
  (`clone_variance`).
- **The ceiling on looks is hardcoded geometry, not missing genes.** Every
  variant is still a one-cell stick with leaves sprayed as speckle. Where a
  leaf goes, what shape a leaf cluster or a flower head takes, how thick a
  stem may be, and which hues exist are all fixed in code or in shared
  material palettes. **A trainer can arrange the existing parts but cannot
  change what the parts look like.** The plan has to move the most visible
  of these into the species file before "improved looks" is reachable
  (§6).
- **Two findings change how the trainer must be built.**
  - **Outdoors, the shipped herb never flowers.** On three seeds over
    20,000 frames, its leader stopped at 42–47 rows, short of the ninth node
    where the flower is. The stops were the height its water reaches and
    finding no direction worth growing; the side shoots stopped at the tip
    cap. Its home is the lab under lamps, so **the trainer must score herbs
    in the lab's light regime**.
  - **The weather is a bigger source of noise than development.** One world
    seed stunted most variants to 12–20 rows whatever their genome.

## 2. How a plant is built (the substrate in one page)

A plant is a set of cells tagged with an organism id, with roles (`Seed`,
`GrowingTip`, `MatureBody`, `Leaf`, `RootTip`, `DormantBud`, `Flower`,
`Fruit`). Every 45 frames (`ORGANISM_TICK_INTERVAL`), each living cell runs
its role's **behaviours** from the species file (`Grow`, `Photosynthesize`,
`Absorb`, `Transpire`, `Reproduce`, `SecondaryThicken`, `BudBreak`, `Ripen`,
`Germinate`, `Divide`, `StructuralAnchor`).

- **A tip grows by a weighted draw over its eight neighbours.** Each open cell
  scores `continuation·heading + light·light-direction + wind·wind +
  up·(gravity or water)`, the four weights summed to one, then divided by
  `1 + crowding·density`. The tip picks one by weighted chance (`Grow` in
  `organism_tick`). Every silhouette comes out of repeating that a few
  thousand times.
- **What a cell becomes when it grows, makes a node, stalls or flushes** is
  the **fate table**: the production rule. It is data (`Fate { when, becomes,
  child, lateral, after_metamers }`), with five events (`Grew`, `Node`,
  `Stale`, `Flush`, `Ripe`) and first-match-wins lookup. Before it existed
  these were four hardcoded writes (`builtin_fate`'s doc).
- **Whole-plant decisions run once a tick per plant**: carbon is shared out to
  the growing tips (`allocate_to_frontier`), a bud may break
  (`break_buds`), roots may re-initiate (`break_root_tips`), grass may tiller
  (`break_tillers`), flowers may rebloom (`process_rebloom`), and upkeep
  charges maintenance, sheds starved tissue and dies back (`organism_upkeep`).
- **The economy is the engine's**: income per node, maintenance with a
  size exponent, construction multiples for wood, leaf and organ, a water
  book with transport along real paths, nutrients. These are about 73
  numeric constants in `plant.rs`, and they bind every species alike.

## 3. The layers that decide a plant

| layer | what it holds | heritable in play? | reachable by the trainer? |
|---|---|---|---|
| **Species file** (`assets/species/*.ron`, `SpeciesDef`) | every behaviour's parameters, the fate table, life-history half-lives, nectar, seed traits, palette band ranges, which materials the plant is built of | no: authored, fixed per species | **yes, all of it**: this *is* the trained object (ruling 1). `SpeciesDef` serialises (`species_export::to_ron`) and `SpeciesRegistry::register_ron` loads a variant at runtime through the shipped parser |
| **Genotype draws** (10 per `Grow` block, `genotype_variance`) | a per-individual multiplier `1 ± variance` on slots such as branch chance, plastochron, root reach and head size | yes: drawn at germination, jittered at birth (`MUTATION_SIGMA` 0.08) | the variances are species-file numbers |
| **Discrete loci** (7: leaf economy, branch angle, internode, sympody, tropism, wood density, flower colour) | small allele tables (e.g. branch angle ×0.4/1.0/1.6) | yes: 0.03 per birth, 0.25 founder variant chance | the founding allele follows the species file; **the allele tables are hardcoded** |
| **`ParamGenome`** | up to 8 overrides of any of 45 `ParamId`s, reaching 4× the largest authored value | **mutation shipped at 0** (`PARAM_MUTATION_CHANCE`, "the dial and not the finding") | not needed: the trainer writes the values outright |
| **`FateGenome`** | the plant's own copy of the fate table | yes: 0.30 per birth, four operators | the fate table is in the species file |
| **Defence** | one scalar, priced in growth | yes | as a founding value |
| **Developmental key** (`World::developmental_key`) | whether growth draws are keyed on the world or on the plant's own frame | world setting, not species | **no**: it is a world setting. §5.4 |
| **Materials** (`assets/materials/*.ron`) | palettes (8 bands of 4 tones for leaf, wood and flower; 6 for fruit), density, strength, burn | no | **only by choosing among existing ones**: a species names its shoot, root, leaf, flower, fruit and seed materials |
| **Engine constants** | ~73 in `plant.rs`: economy, thresholds, caps, tick intervals | no | **no**: they are physics and bind every species |
| **Engine logic** | the tip walk, leaf and organ placement, allocation, die-back, bending, breakage, regrowth rules | no | **no** |
| **Switches** | 35 `PIXEL_PHYSICS_*` names in `plant.rs` (about 30 behaviour, the rest traces) | no | no (world-wide) |

## 4. What the probe showed

`cargo run --release --example herb_morphs -- n=16 seeds=3 frames=20000
solo=1`. Sixteen variants (column 0 is the shipped herb), each grown alone
on a 96-wide `PlantScene` bed on three world seeds. Each variant moves five
of the shoot's look levers by a log factor (σ 0.9) through
`ParamGenome::apply`, plus `after_metamers`. The founder is drawn on its
own; its seedlings are left out of picture and census. Sheet and census:
[`plant-trainer-2026-10-19/herb-morphs-16x3.png`](plant-trainer-2026-10-19/herb-morphs-16x3.png),
[`.log`](plant-trainer-2026-10-19/herb-morphs-16x3.log). The whole run takes
under a minute on this container.

**4.1 The genome sets the silhouette, at these step sizes.** Share of the
spread between plants that is between variants rather than within one
(1 − mean within-variant variance / total variance, 16 × 3):

| | above-ground cells | height | width | leaf share |
|---|---|---|---|---|
| shipped development key | **0.76** | 0.48 | 0.49 | **0.66** |
| `dev=0` (siblings develop alike) | 0.62 | 0.38 | 0.58 | 0.68 |

`dev=0` does not raise it here because the three rows are different
**worlds**, so they differ in weather, not only in growth draws. The forms
reached: a thin stalk (0), leafy bushes (5, 8), a round crown (11), a
sprawling mat that flowers (7, `after_metamers` 3), a ground cover nine rows
high (9, branch angle 96°) and a tall whip (14). **Read a column, not a
tile**: most hold their form down the column.

**4.2 The bed's weather is the biggest noise.** World seed 2 held the shipped
herb, variant 3, variant 10 and variant 12 to 12–17 rows and 42–60 cells,
whatever the genome. The other seeds grew the same genomes to 30–51 rows. The
cause was not traced. A trainer that does not control the sky will spend
most of its signal on it.

**4.3 Outdoors, a herb is not a herb.** No variant except the one with
`after_metamers` 3 held a flower. The shipped herb on all three seeds:

| seed | high | why its shoot tips stopped (commonest three) |
|---|---|---|
| 1 | 42 | tip cap 64, no good direction 14, height limit 9 |
| 2 | 17 | tip cap 5, no room 1 |
| 3 | 47 | height limit 19, tip cap 15, no good direction 14 |

It is at full height by frame 4,000, nine metamers need about 45 rows, and
no tip retired as *flowered*. `herb.ron` records this exact pairing being
fixed by hand (turgor 0.32 → 0.46, "9 axes terminated across 32 established
plants" before). Where that was measured is not recorded in the comment, but
the herb is the lab's plant and the lab counted standing flowers (81 at
6,000 frames, pollinator design §1.2). **So the arena is a lab bed, not a
`PlantScene`.** It is also a ready-made known-answer task for the trainer
(brainstorm A1): make the outdoor herb flower.

**4.4 The root cap decides root size.** On 34 of 48 beds, root cells match
above-ground cells to within 6. That is `MAX_ROOT_FRACTION = 0.5` binding,
which the wiki already notes ("nearly every grass and shrub root tip stops
at its share"). Its doc says 0.5 was "chosen so the rule bites only on
runaway growth". On the herb it bites on ordinary plants. **Root to shoot
ratio is a strategy, and today it is one hardcoded number for every
species.**

**4.5 Texture barely moves.** Leaf share ran 24–70%, but every plant is drawn
the same way: a one-cell brown-olive line with single green cells scattered
along it. That is `plant-appearance-design.md`'s diagnosis for trees, and
the reason five placement levers (weeping, prostrate, sympody, tropism,
acrotony) have each come back from the owner as "same plant" or "not that
different" (`plant.rs`, `FOUNDER_VARIANT_CHANCE`'s doc). The variants here
read differently because they change **size, branching and composition
together**. That is a lever the trainer can pull, where the in-game
allele steps could not.

## 5. Every visible thing a plant does, and who controls it

S = species file (trainer reaches it). H = heritable in play. C = hardcoded
in code or shared data.

| visible aspect | S: what the species file sets | H | C: what is fixed |
|---|---|---|---|
| **height** | `turgor_source/yield/per_cell/taper`, `upward_weight` | genotype slot | the turgor model; `THIRST_PER_EXCESS_HEIGHT` |
| **stem path** (straight or wandering) | `heading_inertia`, `continuation_weight`, `stem_stiffness` | — | 8-neighbour walk; the drawn-heading mode (`K`, world-wide) |
| **branching amount** | `branch_chance`, `max_active_tips`, `branch_priming`, `juvenile_*`, `BudBreak.acrotony` | slot 0 | **the tip cap is a hard count**; which bud breaks (`break_buds`) beyond acrotony |
| **branch angle, tiers** | `branch_angle` per order, `tropism` per order, `sympodial` | 3-allele locus | the allele tables; orthotropic/plagiotropic are the only two habits |
| **node spacing** | `plastochron`, `internode` | slot 2, locus | — |
| **leaf amount** | `leaf_cluster`, `Photosynthesize` rates, `shade_death`, `drought_death` | locus (leaf economy) | `LEAF_CONSTRUCTION_MULTIPLE`; leaves only at nodes |
| **leaf arrangement and shape** | `leaf_spread` (blob ↔ line) only | — | **first leaf a random empty cell behind the tip; the rest a breadth-first spray that never touches the node or sits ahead of it.** No phyllotaxis (alternate, opposite, whorled), no leaf shape, no petiole |
| **crown lift, thinning** | `shade_death`, `drought_death` | — | die-back order and `MAX_DIEBACK_FRACTION` |
| **stem thickness** | `SecondaryThicken.pipe_ratio` | — | the pipe-model `thicken`; a stem is one cell until thickened |
| **colour** | which bands of each material's palette (`foliage/bark/flower/fruit_bands`), and which materials | foliage band (via leaf economy), flower band (locus); **fruit colour has no locus** | **the palettes themselves** (8 bands of 4 tones in `leaf`, `wood`, `flower`; 6 in `fruit`); the dry-tip browning readout |
| **flowers and fruit** | fate rules (`becomes: Flower`, `after_metamers`), `organ_cluster`, `Ripen` rates and costs, `rebloom_after`, nectar | `FateGenome`; slot 9 (head size ±70%) | **head shape is a random breadth-first blob**; organs only where a fate puts them |
| **roots** | the root `Grow` block, `Absorb`, `penetration_force` | slots 1, 5, 8 | **`MAX_ROOT_FRACTION` 0.5 for every species** (binds, §4.4); `MIZ_THRESHOLD`; hydrotropism overriding gravity outright (W6's link 3) |
| **seeds and spread** | `seed_cost`, `reproductive_allocation`, `seed_maturity`, `seed_launch` (priced by reach), `seed_gut_survival`, `seed_provision_fraction` | `ParamGenome` (off) | how a fruit falls; the launch arc; the midden rules |
| **germination** | `light_threshold`, `soil_water_threshold` | — | the seed-bank drying rules |
| **lifespan** | `annual_half_life`, `life_half_life`, `remains_half_life`, `seed_half_life` | — | the annual rule's gate (switch `ANNUAL`) |
| **regrowth** | only indirectly: grass tillers apply to "a species with an economy and no leaf stage" | — | **tillering, regrowing grazed leaf from store, resprouting and root re-initiation are rules with world-wide switches**; a species can opt in only by its structure |
| **defence against grazing** | founding value | yes | its price curve |
| **bending and breaking** | `stem_stiffness` per order | wood density locus | wood strength (material), `stress_field`, bend and break algorithms |
| **death and remains** | half-lives | — | `rot_remains`, litter and burial rules |
| **growth speed** | costs and rates | — | the 45-frame tick, `ORGANISM_STALE_LIMIT` |

**Tally:** of 20 visible aspects, the species file has at least one dial on
19 (all but regrowth), though for leaf arrangement that dial is a single
number. In-game heredity reaches 10. **Eight carry a
hardcoded part that sets what the player actually sees**: leaf arrangement,
head shape, stem thickness rule, palettes, root share, tip cap, regrowth
rules, branch habits.

## 6. How flexible is it for new, emergent forms?

**Where it is open:**

- **Form is emergent, not drawn.** Local rules repeated thousands of times
  give the silhouettes in §4. Nothing in the code knows what a bush is, and
  bushes, crowns and mats appear anyway.
- **The fate table can express new body plans.** Determinate against
  indeterminate, a flower ending each axis or each node, a growth form
  nobody authored. Measured on 2026-09-03: the fate genome alone made a herb
  lineage launch shoots off its roots in the shipped game, 4 events in 3
  runs (`plant-engine-rethink-2026-09-03.md` §4a).
- **Mechanics key on structure, not names.** No growth rule tests a species
  name (the only two name lookups in `plant.rs`'s engine are the planting
  helpers `plant_moss_seed` and `plant_tree`), which is why a trainer can
  change a plant's kind as well as its numbers. Grass regrowth applies to "a species with an economy and no
  leaf stage", for example.
- **The plumbing exists.** `register_ron`, `PlantScene::species_ron`,
  `species_export::to_ron`, `ParamGenome::apply`, `clamp_param` and
  `param_scale` already do the variant → world → species-file loop. The
  probe needed no engine change.

**Where it is walled.** Each item is a place where the trainer would find
nothing, however long it ran:

1. **Leaf placement and shape** (§5): the largest look lever, all in code.
2. **Flower and fruit head shape**: a random blob.
3. **Stem width**: one cell unless the pipe model thickens it.
4. **Hues**: only what the shared palettes hold.
5. **Root share**: 0.5 for everything.
6. **The tip's decision**: four fixed features. A tip cannot steer away
   from its own other branches, toward open sky rather than toward light, or
   by how loaded its branch is.
7. **Whole-plant policies**: which bud breaks, how carbon is split between
   tips, what dies back first, when to regrow. All rules, none of them
   strategies a species can choose.
8. **Clonal spread** is reachable and inert: 144 shoots launched off roots
   came to nothing (`plant-engine-rethink` §4b, 2026-09-03). That predates
   W6's "a cell that changes role changes tissue" fix (2026-09-08), so it
   needs re-measuring before it is called walled.
9. **Developmental coherence** is a world setting. Whether a species grows
   as a coherent form or a mosaic of per-cell dice is not the species' to
   choose.

**The law the economy already enforces**, and that every un-walling must
keep
(`plant-heritability-survey-design-2026-08-27.md` §2, quoted in `plant.rs`'s
`launch_price` doc): *"a free lever made heritable produces uniformity, not
diversity"*. A new species-file field with a benefit and no price gives the
trainer exactly one answer, and every species finds it. Each field moved out
of the code needs its cost moved with it.

## 7. What this hands the plan

1. **Day one needs no plant "mind".** The species file is a large, working
   search space. Train it first (brainstorm rungs A–D).
2. **"Improved looks" needs un-walling, in order of what it shows on
   screen**: leaf arrangement and cluster shape, then head shape, then stem
   width, then a species-owned root share. Each becomes a species-file field
   whose default reproduces today **bit-identically**, with a price where it
   buys anything. The house pattern for this is
   `leaf_spread`'s zero guard (`plant.rs`: the extra draw is skipped at 0 so
   "every plant in the world" does not change).
3. **Hues** are a separate question: new palettes are new material data,
   not a species field. Either the trainer chooses among existing palettes
   and bands, or the owner rules that a species may carry its own.
4. **The arena is a lab bed, with the sky pinned per bed.** Herbs do not
   flower in `PlantScene`. Weather left live puts most of the noise into the
   score.
5. **Developmental key.** Train under the key the game ships, or decide
   first whether a species should own its key. Its measured effect on herb
   size heritability is 0.034 → 0.650 (`plant-engine-rethink` §6.8). It is
   the owner's call, because it would change every plant in both games.
6. **Known-answer tasks are available for the trainer's own positive
   controls**: the outdoor flowering failure (§4.3), the hand-tuned head size
   (`organ_cluster` 9 → 12 by the owner's eye), the germination floor that
   sits above wilting (`plant-water-scarcity` §3).
