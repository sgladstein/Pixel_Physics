# Where the plant engine's costs actually are, and what "attach costs" should mean (2026-08-27)

**Status: investigation, with one measured correction to a load-bearing
claim in the series. No engine change proposed for landing yet — §8 is a
plan, §9 are the owner's calls.**

Written against `why-changes-cost-so-much-2026-08-27.md` (the diagnosis),
`plant-heritability-survey-design-2026-08-27.md` §4a (the inventory),
`plant-evolvability-handoff-2026-08-27.md` (where the line stands) and
`plant-simulation-research.md` §7 (the literature). The diagnosis is
measured and is not re-derived here. What this adds is **where in the
engine the missing costs actually are**, which turns out not to be where
§4a puts them.

## Contents

| § | |
|---|---|
| 1 | The headline, in four sentences |
| 2 | Two closed economies and one open one |
| 3 | The three conditions a cost must meet — and why two of them get skipped |
| 4 | Why plants differ in size and colour, and the recipe that produced both |
| 5 | The correction: turgor is not a free lever, it is an unrewarded one |
| 6 | Fences and prices — the corrected inventory |
| 7 | The second problem, which is not costs at all: denomination |
| 8 | What to do, in order |
| 9 | Owner calls, surfaced not settled |

## 1. The headline, in four sentences

**The engine's resource economies are not unpriced. They are close to
finished.** Carbon has income bounded by intercepted light, superlinear
maintenance charged on a monotone girth memory, growth funded from
*surplus* rather than income, functional-balance allocation to the limiting
organ, die-back, and starvation death; water has capacity from contacting
root cells, demand from foliage, stomatal closure that throttles income,
and a separate desiccation term that sheds. Both are closed loops with real
equilibria, and this report found no hole in either.

**What is unpriced is the morphology layer sitting on top of them** — the
decisions about *arrangement* rather than *amount*: which way a tip grows,
at what angle it departs, how long it holds a line, where it puts its
leaves, when it stops going up. Those decisions are made by a weighted
score that costs nothing to move.

**And exactly one arrangement axis is priced — vertical position under
foliage — but its reward is switched off in the bed every plant measurement
in this line is taken on.** That is §5, and it is measured both ways: height
costs an order of magnitude in bill-to-income and changes nothing reachable
at the standard 56-cell spacing, and at 20-cell spacing the same two arms
separate cleanly on who survives at all.

**So "attach costs" is the wrong instruction to give this engine, and would
produce the uniformity it is trying to avoid.** The instruction that fits
what is actually here is *attach the missing half*: some levers need a
cost, and at least one prominent lever needs its **benefit** turned back on.

## 2. Two closed economies and one open one

Verified in source rather than taken from the series.

**Carbon.** `allocate_to_frontier` (`plant.rs:4210`) sets
`income = intercepted / L_node x INCOME_PER_NODE`, and the growth pool is
`(income - maintenance).max(0).min(stock)` — net production, not gross,
with the comment recording that charging the bill while leaving the pool
gross was tried and produced *"a bigger tree that is dying"* at
bill/income 2.10. `maintenance_cost` (`:3432`) is
`MAINTENANCE_PER_CELL + MAINTENANCE_PER_NODE x (q_peak / L_node)^1.5` on
shoot tissue, Takenaka's exponent, charged on the *monotone* peak so a
branch that has lost foliage keeps its bill. `STARVATION_DEATH_TICKS`
closes it: 200 consecutive ticks unable to pay the mass term sets
`senescent`, and `rot_remains` carries the plant out at the species
half-life. This is a working equilibrium with a graded death, and it
already satisfies both of `CLAUDE.md`'s ethos laws.

**Water.** `water_capacity_of` (`:541`) is `WATER_SCALE x contact_root_cells`,
so storage is bought with root tissue that touches soil and a walled-in root
buys none. `settle_water` (`:568`) returns the stomatal term that multiplies
every photosynthetic credit and, separately, the desiccation term that
`drought_death` sheds on — deliberately not collapsed into one number,
because keying shedding on the spent side would make the conservative allele
select against itself. Closed loop.

**Morphology.** `Grow`'s direction score (`:1940`) is

```rust
let preference = dot(dir, heading) * continuation_weight
               + dot(dir, photo)   * light_weight
               + dot(dir, wind)    * wind_weight
               + dot(dir, gravity_or_water) * upward_weight;
let score = preference / (1.0 + density * crowding_weight);
```

Nothing in that expression is charged. Every open 8-neighbour costs the
same `Grow.cost` to occupy; a step toward light and a step away from it are
the same price. The score decides *shape*, and shape is free.

## 3. The three conditions a cost must meet — and why two of them get skipped

§4a's criterion 2 ("it has a counterweight") is necessary and it is not
sufficient. A cost creates a real trade-off only when all three hold:

1. **The cost exists** — pushing the lever makes something worse.
2. **The cost and the benefit are paid in the same currency**, so they are
   comparable at the margin. A benefit in carbon against a cost in "a rule
   refuses you" is not a trade, it is a fence.
3. **The environment varies which arm wins.** Otherwise there is one global
   optimum and selection pins every individual to it — which is §4a's own
   failure mode arriving one level up.

Condition 3 is `plant-simulation-research.md` §7b's multi-task result, and
it answers the owner's question about whether equilibrium and diversity are
the same question directly: **they are not.** Conditions 1 and 2 buy
equilibrium — the system settles instead of running away. Only condition 3
buys diversity. A perfectly priced engine in a uniform world produces one
optimal plant, on time, at equilibrium, and the owner has already rejected
that outcome three times.

This matters for sequencing. The costs pass is worth doing because it is
what makes new mechanisms composable (the diagnosis's §2, which holds). But
**it will not on its own produce plants that look different from each
other**, and if it is sold that way it will read as a failure when it
lands.

## 4. Why plants differ in size and colour, and the recipe that produced both

The two things the owner keeps naming as the only differences between
plants are exactly the two things this engine has deliberately given a
price *and* a readout.

- **Size** is the carbon economy's output. It is what the priced loop sets.
- **Colour** is the readout of the only two fully priced heritable axes:
  `LOCUS_LEAF_ECONOMY` sets the foliage band (`plant.rs:963`) and
  `LOCUS_WOOD_DENSITY` sets the bark band (`:964`).

That is not a coincidence and it is not a complaint — it is the recipe
working. `LEAF_RATE_ALLELES` is paired with `LEAF_TRANSPIRATION_ALLELES` at
every consumer *"because a free rate axis would be selection candy with no
bill attached"*; `WOOD_DENSITY_ALLELES` is *"one number for both on
purpose, so tuning cannot quietly turn the trade into a free lunch."* And
the leaf-economy pair has a **measured crossover** — `plant-genome-design.md`
§8d, a full 2x2, where the sign flips: acquisitive wins wet (+21% mass, +32%
seed), conservative wins dry (+43% foliage retained), and the acquisitive
stand drinks its own bed to the wilting point. Neither allele wins
everywhere, so selection cannot saturate it. That is all three conditions
met, once, and it is the template.

**Two honest qualifications on that template, both from the record.** §8d
is one world seed each way and says so; on total dry mass the crossover is
only +4%, inside the between-individual spread, and the claim rests on
foliage retention, the water block and the soil left behind. And the
wood-density pair has conditions 1 and 2 but **condition 3 is unverified**:
cheap wood measured +46% mass and +52% seed, while dense wood's advantage
is a longer loaded cantilever, and nothing on the record shows a scene where
that advantage pays. Until it does, wood density is a trade-off on paper and
a one-way street in practice.

## 5. The correction: turgor is not a free lever, it is an unrewarded one

§4a's first entry under *"Free levers — do NOT make these heritable without
adding a cost first"* is `turgor_source` and `turgor_yield`, on the
reasoning that *"raising the height ceiling costs nothing"*.

**Raising the ceiling costs nothing; the height a plant then builds is
priced, and priced hard.** `accumulate_support` (`:3721`) runs a basipetal
sweep in which every cell below foliage accumulates that foliage into `q`,
and `organism_upkeep` (`:5062`) charges *every* non-frontier shoot cell
`MAINTENANCE_PER_NODE x (q_peak / L_node)^1.5`. A trunk of height H under a
crown of Q nodes therefore pays that term H times over. Height is linear in
the bill, crown is superlinear, and the two multiply.

So the missing half is not the cost. It is the **benefit**. The light field
casts direct sun down each column and `field.rs` states the consequence
outright: *"Open sky now reads the same at any depth, so height carries no
intrinsic reward and a canopy shades what is beneath it because it is in
the way."* Height pays only by escaping a **neighbour's** shade — and the
standard plant bed spaces its plants so that never happens.
`PlantScene::build` sets `spacing = width / (trees + 1)`, which is 56 cells
at the defaults, against a median crown thickness of 11 and a max of 30.
The crowns do not touch. `plant_probe`'s own header says the spacing is held
deliberately: *"packing 32 trees into the default 512 columns puts them 15
cells apart instead of 57, which is a different experiment — crown shyness
is exactly what that spacing decides."*

**Every measurement in the plant-evolvability line is therefore taken on a
bed where competition for light is switched off**, which is the one thing
that makes height, crown spread, phototropism and crowding worth anything.
That is `CLAUDE.md`'s *"check that a guard's inputs actually vary what it
guards"* applied to a whole test bed rather than to one test.

### 5a. Measured: what raising the height ceiling actually costs

`turgor_source` swept low / as-authored / high with everything else held
fixed, rebuilt between arms (`.ron` is `include_str!`; each arm's value was
echoed back from the file before running), 8 trees, 28,800 frames — 8 whole
day/night periods, so the phase is pinned — and **four world seeds per
arm**, `plant_probe`. `tree.ron` restored by a trap and verified byte-clean
afterwards.

`h_max = (turgor_source − turgor_yield) / turgor_per_cell`, so the three
arms are ceilings of 60, 120 and 240 cells of hydraulic path.

| arm | median height | median cells | bill / income | cells shed to starvation | seeds set |
|---|---|---|---|---|---|
| **0.55** (h_max 60) | 85, 87, 90, 91 | 1,740–2,242 | 0.42–0.58 | 28–459 | 23, 26, 29, 48 |
| **1.0** (authored, h_max 120) | 130, 135, 137, 154 | 3,286–5,566 | **1.15–1.48** | **4,656–6,943** | 21, 24, 30, 41 |
| ~~1.90~~ (h_max 240) | — | — | — | — | — |

**The high arm is discarded.** All four of its runs hit the scene ceiling
(`canopy top row 0`), which `plant_probe` flags itself — *"shape numbers
from this run are void"* — and it is the exact trap
`plant-heritability-survey-design-2026-08-27.md` §4 names. Its economy
numbers ran in the same direction (bill/income 3.4–6.2, 12,000–17,500 cells
shed) and are recorded here only as a direction, not as a data point: a
clipped population's bill is a reading of truncated plants.

**The knob is connected and the cost is enormous.** Across the valid pair,
height, bill-to-income and starvation shedding each separate with **no
overlap between arms over four seeds**: the plant goes from comfortably
solvent at 0.5 to running its stock down at 1.3, and sheds two orders of
magnitude more tissue to starvation. §4a's *"raising the height ceiling
costs nothing"* does not survive contact with the maintenance walk.

**And the fitness proxy does not notice.** Seeds set overlaps completely
(23–48 against 21–41). The mechanism is legible rather than inferred:
`Reproduce` runs on every `MatureBody` cell, so seed rate is `seed_chance x
canopy size`; a taller plant has more cells, which buys back exactly what
the higher bill costs it. **The economy converts the price of height into
tissue turnover rather than into fewer offspring.**

**A limit on how far that second finding can be pushed, and it is the
important one.** Germinations were 8–9 in every arm, against 8 founders —
that is the founders germinating and **essentially nothing recruiting**,
which independently reconfirms the handoff's §2.3 finding that generation
depth is the live constraint for trees, now with a direct counter rather
than the seed-bank inference. So this bed cannot report a fitness
consequence *for any lever*, and "the cost is invisible to selection" is
not a claim it can support. What it does support is narrower and still
decisive: **the cost is real, it is large, and nothing in the reachable
output moves when it is paid.**



### 5b. Measured: turning competition on changes what height *is*

The same two turgor arms, same seeds, same 512-column world, same 28,800
frames — **24 trees instead of 8**, so spacing falls from 56.9 cells to
20.5 and crowns overlap. Density is the only difference, and the world is
byte-identical in width, soil and light, so nothing is traded for the extra
plants except room. No run hit the ceiling.

| arm | established, of 24 sown | median cells / plant | seeds set | bill / income |
|---|---|---|---|---|
| **0.55** | 17, 19, 21, 23 | 825–1,458 | 36, 36, 43, 47 | 0.45–0.56 |
| **1.0** | **11, 13, 13, 16** | 2,100–2,851 | 29, 40, 44, 46 | 0.99–1.53 |

All 24 germinated in every run of both arms, so the difference is not
recruitment — it is **which seedlings got past 20 cells**. The two arms do
not overlap: the tall stand carries roughly two-thirds as many established
plants, each about 2.5x bigger.

**That is competition, and it does not exist in the standard bed.** At 8
trees the same two arms establish 8/8/8/8 and 9/8/8/6 — everybody makes it,
in both arms. At 24 trees, height buys the survivors their neighbours'
share and costs the losers everything. Height stops being a tax and becomes
a **contest with an outcome**: tall is fewer, larger and riskier; short is
more, smaller and safer. That is a strategy trade-off of exactly the kind
§3's condition 3 requires, and it appeared purely from changing the spacing
— no engine change, no new cost.

**And it exposes why seeds-set can never have been the signal.** Total
seeds barely move (39.5 against 42 by median, fully overlapping) in either
bed, at either turgor. The reason is structural: stand seed output is
`seed_chance x total mature cells`, total mass is bounded by intercepted
light, and intercepted light is fixed by the width of the world. **The
stand's output is set by the resource, not by the plants' morphology; what
morphology decides is who gets it.** So a stand total is the wrong
denominator for every fitness question in this line, and a *per-genotype
share against its own competitors* is the right one. That is a measurement
change, not an engine change, and it is a prerequisite for reading any
costs work at all.

## 6. Fences and prices — the corrected inventory

The distinction §4a's binary (free / priced) misses, and the one that says
what to build. A **price** is continuous, paid in a currency the plant is
already spending, and leaves the extreme *reachable but expensive*. A
**fence** is a threshold or a cap: it makes the extreme unreachable, so
every individual sits against it and nothing varies. Fences are why a
population converges even when a cost exists somewhere nearby.

| lever | today | what is missing |
|---|---|---|
| `seed_maturity` | **fence** — hard shoot-cell threshold. Its own doc names the attractor it is holding shut: *"selection for instant reproduction, which is a real evolutionary attractor and a boring one"* | a price on early reproduction (seed provisioning scaled to parent size), so precocity is affordable and *costly*, not forbidden |
| `max_active_tips` | **fence** on top of a price (`supportable`, income over tip cost) | the price is the good half; the cap is what pins every species to its authored integer |
| `MAX_ROOT_FRACTION` | **fence** | root tissue already costs `MAINTENANCE_PER_CELL` and buys water capacity — the price is there, the cap overrides it |
| turgor `h_max` | **fence, with a taper** — the taper is a genuine softening and is why crowns fade out over a band rather than stopping on a line | not the cost (§5); the reward |
| `crowding_weight` | **neither, by design** — the cliff fix made it divide rather than subtract, so it reorders and can no longer kill | this was the right fix for the cliff and it removed the counterweight; what replaces it is open |
| `light_weight` / `upward_weight` / `wind_weight` / `continuation_weight` | **free** — unnormalised weights on an uncharged score | §7 |
| `branch_angle` | **free, and it bypasses the score entirely** (`:2218`) — the angled path rebuilds the candidate set at a flat 1.0 | |
| `plastochron`, `leaf_cluster` | **half-priced** — a leaf costs nothing to *build* (flagged in-code at `:2411`: *"A real leaf is built from carbon and should charge for it"*) but costs to *run*, through transpiration demand and the girth term | the construction cost, which is the one that would make leaf placement a decision |
| `rate` (Photosynthesize) | **free at the authored layer, priced at the genome layer** | see below — this is the sharpest single finding |
| `Grow.cost`, `seed_cost`, `shade_death`, `drought_death`, `stomatal_reserve`, `transpiration` | **priced** | |

**The sharpest single finding, and the smallest thing to fix.** The genome
pairs photosynthetic rate with transpirational demand at every consumer,
and the comment says why. The **species file does not**: `rate: 0.5` and
`transpiration: 0.05` are independent fields of the same `Photosynthesize`
struct (`tree.ron:299`, `:306`), so an author — or a mutation operator that
ever reaches authored parameters — can raise income without raising demand.
The principle the engine already states for alleles is simply not applied
one layer down.

## 7. The second problem, which is not costs at all: denomination

The diagnosis's §2 attributes the retune loop to free parameters
cross-calibrated against each other. That is one of two mechanisms, and the
second is cheaper to fix and has a proven precedent in this repo.

**The direction score has no normalisation.** The four weights are absolute
in the source and *relative* in effect, because the score is used for
weighted sampling over candidates — only the ratios reach behaviour. So
`light_weight` cannot be changed without silently reweighting the other
three, and nothing in the file says so. That is precisely what made the
lateral-phototropism repair a five-species tuning pass rather than a bug
fix (`plant-phototropism-lateral-2026-08-27.md` §3).

**The engine has already solved this once, elsewhere, and nobody
generalised it.** `INCOME_PER_NODE` was re-denominated in `L_node` units so
that *"it survives changes to `MAX_LIGHT`, `leaf_cluster` and the light
model the way its predecessor did not"*, and `MAINTENANCE_PER_NODE` is in
the same currency so the two are directly comparable without a conversion.
That is the medicine for the retune loop, applied to the carbon economy and
not to the score.

So the diagnosis's rule 2 — *batch by shared tuning surface* — is right,
and there is a stronger version available for at least one of those
surfaces: **make the shared budget explicit, and the batch stops being
shared.** Normalising the four weights to a simplex is a mechanical change
with no behaviour change at the current values, and it converts "changing
one weight retunes five species" into "changing one weight moves one
species along one axis".

## 8. What to do, in order

Nothing here is proposed for landing in this session; each step is scoped
against `CLAUDE.md`'s rule that a shared-budget change is a tuning change,
not a fix.

1. **Turn competition on before pricing anything, and fix the
   denominator.** The bed already exists — §5b is `trees=24 width=512` on
   the shipped `plant_probe`, no build required — so this is a *convention*
   rather than a feature: a competitive arm alongside the existing sparse
   one, which stays valid as an isolated-individual instrument with its
   baselines banked. The denominator is the other half and is the part that
   needs work: §5b shows stand seed totals are pinned by world width, so
   fitness has to be read as a genotype's share against its own competitors.
   Without both, every lever whose payoff is competitive is measured with
   its benefit disabled and its scoreboard held constant, and a costs pass
   measured there will read as a pure tax. Prerequisite for steps 2 and 4,
   and the cheapest item on the list.
2. **Couple `rate` and `transpiration` at the authored layer**, the way
   `WOOD_DENSITY_ALLELES` couples strength and price. Smallest diff on the
   list, states a principle the engine already holds, and is a real
   counterweight rather than a tax.
3. **Charge carbon for building a leaf.** The in-code note at `:2411` asks
   for exactly this and defers it to *"the single pass that sets it"*. It
   converts `plastochron` and `leaf_cluster` from half-priced to priced,
   and it makes leaf placement a decision with an opportunity cost.
   Shared-budget: it moves the carbon economy, so it is a tuning pass and
   wants the seed sweep either side.
4. **Convert two fences to prices** — `seed_maturity` and
   `max_active_tips` — once (1) exists to measure them against. Both have a
   price sitting next to them already.
5. **Normalise the direction score** (§7), as a separate no-behaviour-change
   commit, *before* anyone touches `light_weight` again. This is what makes
   the banked phototropism patch affordable: the re-derivation it needs is
   over three degrees of freedom, not five species x four orders.
6. **Only then** the heritability survey, which now has a defensible
   ranking criterion: rank on §3's three conditions, not on effect size.

**Sequencing note against the handoff's §3 drift.** Steps 1-5 are
machinery, not census — they change what the engine can express, which is
the project. Step 6 is characterisation and stays last.

## 9. Owner calls, surfaced not settled

The handoff's §6 carries three (what replaces the withdrawn §6.2; which
clades and in what order; whether the niche table keeps naming species).
This investigation adds three more, all of which change what gets built:

1. **Is diversity supposed to come from the environment or from the
   genome?** §3's condition 3 says a priced engine in a uniform world
   yields one plant. The environment route (heterogeneous light, water and
   disturbance, per Bornhofen) and the genome route (more loci) need
   different work, and the first is largely worldgen and scene rather than
   plant code.
2. **Should reproduction stay fenced?** Removing `seed_maturity`'s
   threshold in favour of a price deliberately opens the ruderal strategy
   its own comment calls "a real evolutionary attractor and a boring one".
   With disturbance in the world that is a legitimate strategy; without
   disturbance it is just the boring optimum. This one is contingent on
   call 1.
3. **Does dense wood ever need to win?** §4's qualification. If the answer
   is yes, something has to break branches in the ordinary run of play,
   which is a structural-line question and not a plant-line one.
