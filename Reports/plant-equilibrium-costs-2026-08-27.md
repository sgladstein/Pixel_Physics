# Does every lever in the plant engine have a cost and a benefit? (2026-08-27)

**Status: audit, with two measured results. No engine change proposed for
landing — §9 is a plan, §10 are the owner's calls.**

The question this answers is the owner's, stated directly: *make the plant
system one that finds its own equilibrium, so that adding a feature does not
require retuning everything else.* Written against
`why-changes-cost-so-much-2026-08-27.md` (the diagnosis, measured and not
re-derived here), `plant-heritability-survey-design-2026-08-27.md` §4a (the
inventory this corrects in two places), and
`plant-evolvability-handoff-2026-08-27.md`.

## Contents

| § | |
|---|---|
| 1 | The answer, and the mechanism in one sentence |
| 2 | What a lever needs, and the four verdicts |
| 3 | The audit — every authored parameter, by verdict |
| 4 | Why *construction is free and upkeep is pooled* is the whole retune loop |
| 5 | Measured: turgor is the "cost, no benefit" row |
| 6 | Fences — where the tuner has already compensated by hand |
| 7 | The second mechanism, which is not costs at all: denomination |
| 8 | The instrumental half: what this does and does not buy for diversity |
| 9 | What to do, in order |
| 10 | Owner calls — three answered, and what they change |

## 1. The answer, and the mechanism in one sentence

**No — but the gap is not where §4a puts it, and it is narrower and more
systematic than "most levers are free".**

The two resource economies are close to finished. Carbon has income bounded
by intercepted light, superlinear maintenance charged on a monotone girth
memory, growth funded from *surplus* rather than income, functional-balance
allocation to the limiting organ, die-back, and starvation death. Water has
capacity bought with contacting root tissue, demand driven by foliage,
stomatal closure that throttles income, and a separate desiccation term that
sheds. This audit found no hole in either.

The gap is one structural asymmetry, and it is the mechanism of the retune
loop:

> **The engine prices tissue *upkeep* and almost never prices tissue
> *construction*.** So a morphology lever's cost is not charged where the
> decision is made; it is deferred into one pooled number — the maintenance
> bill — which every other constant is calibrated against. Move any
> morphology lever and that pool moves, so every constant that reads it has
> to move too.

That is why the retune is *global* rather than local. It is not that the
levers are individually unpriced; it is that they all bill to the same
account, after the fact.

**Three of the four carbon debits in the whole plant engine are
construction charges, and the two largest tissue producers are not among
them.** `Grow.cost`, `BudBreak.cost`, `seed_cost` and a root re-initiation
step are the complete list (`write_carbon` sites at `plant.rs:4012`,
`:4180`, `:4984`, `:5310`, plus the per-step decrement in `Grow`). Leaves
are free to build — flagged in-code at `:2411`: *"A real leaf is built from
carbon and should charge for it; adding that price now would change the
economy this phase is explicitly not tuning."* Secondary thickening is free
to build — `thicken()` (`:5871`) spends nothing, and
`WOOD_DENSITY_ALLELES`' own doc says so: *"Secondary thickening pays no
carbon today, so the price binds on extension only."* Both then pay
`MAINTENANCE_PER_CELL` for ever.

## 2. What a lever needs, and the four verdicts

A lever is **self-limiting** when pushing it up buys something *and* costs
something, in a currency the plant is already spending, so there is an
interior optimum the economy can find on its own. Anything else needs a
human to decide where it stops.

| verdict | what it does | what holds it today | what it costs you at feature time |
|---|---|---|---|
| **benefit, no cost** | runs away | a hand-placed **fence** (§6) | the fence has to be re-placed every time anything near it changes |
| **cost, no benefit** | pure tax — nobody would push it, so it looks settled | nothing; it just sits | the day the benefit is switched on, every constant calibrated against "this is a tax" moves at once |
| **neither** | inert | nothing | nothing, until something else makes it live — then it is a surprise |
| **both** | finds its own level | itself | the economy absorbs the change |

The second row is the one this project keeps being bitten by and has no
name for. It is exactly the shape of the phototropism withdrawal:
`light_weight` was calibrated against a lever whose codomain was
`{(0,-1),(0,0)}` — a lever with a cost in scoring budget and no lateral
benefit, because none was reachable — and giving it a real benefit moved
five species' weights at once
(`plant-phototropism-lateral-2026-08-27.md`). §5 shows `turgor_source` is
the same shape, measured.

## 3. The audit — every authored parameter, by verdict

Every field of `Behavior` and every `SpeciesDef` scalar, traced to its
consumer in source. `ByOrder` fields are one entry.

### Both — self-limiting today

| lever | buys | costs |
|---|---|---|
| `Grow.cost` | — (it *is* the price of extension) | carbon per step; scaled with strength by `WOOD_DENSITY_ALLELES` so tuning cannot make it a free lunch |
| `seed_cost` | provisioning for the seedling (`state.endowment`) | carbon, debited from the setting cell |
| `BudBreak.cost` | a new tip — new frontier | carbon from the plant's richest cell, out of the same pool `Grow` draws on, so flushing competes with extending |
| `transpiration` | nothing on its own — it **is** the counterweight to `rate` and `leaf_cluster` | water demand, which closes stomata and throttles income |
| `stomatal_reserve` | drought survival — closure protects the stock | closure cuts the carbon credit directly (`water_status` multiplies every credit) |
| `shade_death`, `drought_death` | a lower bill — shed foliage stops costing | lost income, and shed cells become litter |
| `penetration_force` | access to harder ground | priced **on use**: `penetration_cost_mult` charges a root more carbon per step in resistant powder |
| `Divide.cost` | — (moss's price of dividing) | carbon |

### Benefit, no cost — the runaway class

| lever | buys | what should cost, and does not |
|---|---|---|
| `pipe_ratio` (`SecondaryThicken`) | stem girth — structural strength via `max_cantilever_reach`, and the visible bole | **construction is free**; `thicken()` spends no carbon. Upkeep is charged, pooled, after the fact |
| `plastochron`, `leaf_cluster` | foliage, which is income | **construction is free** (`:2411`). Priced only through pooled upkeep and transpiration demand |
| `rate` (`Photosynthesize`) | carbon income, directly | nothing at the authored layer. The **genome** pairs it with `transpiration` at every consumer *"because a free rate axis would be selection candy with no bill attached"*; `tree.ron` leaves `rate: 0.5` and `transpiration: 0.05` as independent fields of the same struct |
| `Absorb.rate` | faster water uptake | nothing — but it **saturates** against `water_capacity_of`, which is bought with root tissue. Self-limiting by ceiling rather than by price |
| `branch_chance`, `branch_priming` | more axes | each branch is one extra `Grow.cost`, i.e. branching and extending are priced *identically*. Nothing prices the *choice* |
| `seed_chance` | offspring | `seed_cost` per seed, which is real but tiny (0.3) against a stand output pinned by total light (§5b) |
| `acrotony`, `thickening_survival` | where and whether buds survive | nothing |
| `seed_half_life`, `remains_half_life` | a longer-lived seed bank / slower rot | nothing |
| `shoot_material`, `root_material`, `leaf_material` | density, cantilever reach, flammability, `breaks_into` | nothing in the **economy**. May be self-pricing through physics — a material trades on four axes at once — but that is unverified and worth checking before designing around it |

### Cost, no benefit — the deferred-retune class

| lever | costs | benefit that is switched off |
|---|---|---|
| `turgor_source`, `turgor_yield` | height, charged once per trunk cell as `(q_peak/L_node)^1.5` — **measured at 0.5 → 1.3 bill-to-income and a 200x rise in starvation shedding, §5a** | escaping a neighbour's shade. `field.rs`: *"open sky reads the same at any depth, so height carries no intrinsic reward."* The standard bed spaces plants 56 cells apart against a median crown thickness of 11, so no neighbour is ever in the way — **§5b turns it back on and the arms separate** |
| `light_weight` | scoring budget | lateral steering, until the banked `phototropism_dir` repair lands |

### Neither — free but inert, or purely structural

`continuation_weight`, `upward_weight`, `wind_weight`, `heading_inertia`,
`internode`, `branch_angle` (which additionally **bypasses the score
entirely** on the angled path, `:2218`, rebuilding the candidate set at a
flat 1.0), `sympodial`, `tropism`, `juvenile_size`,
`juvenile_plastochron`, `juvenile_branch`, `turgor_taper`,
`damp_chance`/`dry_chance`/`shade_sensitive`, `Germinate`'s two
thresholds, the palette bands.

These decide *shape* and cost nothing, because every open 8-neighbour costs
the same `Grow.cost`: a step toward light and a step away are the same
price. They are the four weights of one unnormalised sum (§7), which is a
different problem from a missing cost and has a different fix.

`crowding_weight` is its own case and is deliberately in none of these
rows: the cliff fix made it **divide** rather than subtract, so it reorders
and can no longer kill. That was the right fix for a real arithmetic cliff
(median tree 2,620 cells at 12.0 against 26 at 20.0) and it removed the
counterweight; what replaces it is open.

## 4. Why "construction is free, upkeep is pooled" is the whole retune loop

Take the leaf. Placing one costs nothing, so `plastochron` and
`leaf_cluster` can be moved freely at authoring time. The leaf then earns
income and adds to `q_peak` for every cell below it, so it raises the
maintenance bill of the whole path back to the collar, superlinearly. The
bill is what `allocate_to_frontier` subtracts to get the growth pool, what
`break_buds` divides to get `supportable`, and what
`STARVATION_DEATH_TICKS` watches.

So changing one leaf-placement number moves the growth pool, the tip cap
and the death rule at once — and **none of those three is where the change
was made**. That is the global retune, and it is a property of *where the
charge lands*, not of how many parameters are free.

Charging construction fixes the locality without adding a single new
concept: the decision that creates the tissue pays for it, at the point it
is made, out of the pool it was going to draw from anyway. The pooled
upkeep then stops being the only signal and becomes one of two.

**This is a shared-budget change and must be scoped as one.** Per
`CLAUDE.md`, a change that reallocates a shared budget is a tuning change
however small its diff: `INCOME_PER_NODE`, `MAINTENANCE_PER_NODE`,
`Grow.cost` and `supportable` are all calibrated against today's free
construction, and re-deriving them is part of the work, not scope creep.

## 5. Measured: turgor is the "cost, no benefit" row

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
magnitude more tissue to starvation. the survey's §4a *"raising the height ceiling
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
which independently reconfirms the handoff's §2 finding that generation
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
§8's condition 3 requires, and it appeared purely from changing the spacing
— no engine change, no new cost.

**And it exposes why seeds-set can never have been the signal.** Total
seeds barely move (39.5 against 42 by median, fully overlapping) in either
bed, at either turgor, because stand output is `seed_chance x total mature
cells`, total mass is bounded by intercepted light, and that is fixed by
the width of the world. **The stand's output is set by the resource;
morphology decides who gets it.** §9 step 1 carries the consequence: until
the readout is a genotype's share against its own competitors, a costs pass
measured on stand totals reports nothing whichever way it goes.

## 6. Fences — where the tuner has already compensated by hand

A **price** is continuous, paid in a currency the plant is already
spending, and leaves the extreme reachable but expensive. A **fence** is a
threshold or a cap: it makes the extreme unreachable, so every individual
sits against it and nothing varies. Every fence in the list below is a
place where a benefit had no cost and a human placed the stop by hand —
which is the retune loop's other half, since a hand-placed stop has to be
re-placed whenever anything near it moves.

| fence | holds shut | the price already sitting next to it |
|---|---|---|
| `seed_maturity` (600 shoot cells) | precocious reproduction. Its own doc names what it is holding: *"the world fills with dynasties of two-cell plants that never pay the cost of growing up — selection for instant reproduction, which is a real evolutionary attractor and a boring one"* | `seed_cost`, which could scale with parent size so precocity is affordable *and* expensive rather than forbidden |
| `max_active_tips` (14 shoot, 10 root) | unbounded frontier | `supportable` in `break_buds` — income over what one tip costs — which is a genuine price. The cap sits on top of it and is what pins every species to its authored integer |
| `MAX_ROOT_FRACTION` | all-root plants | root cells already pay `MAINTENANCE_PER_CELL` and buy water capacity |
| turgor `h_max` | unbounded height | §5. This one has a genuine softening — `turgor_taper` makes the last stretch stochastic, which is why crowns fade over a band instead of stopping on a line. It is the only fence in the engine with a graded edge, and it is worth copying |
| `STARVATION_DEATH_TICKS` | instant death on one bad tick | correctly a hysteresis rather than a fence — measured against the mass term alone, because a mature plant is in deficit on the full bill essentially all the time |

Not every fence should become a price. A cap that bounds *work* is
legitimate and `max_active_tips` partly is one. The test is `CLAUDE.md`'s:
does exhausting it produce an **answer**, or merely less work? All four of
the first four produce an answer.

## 7. The second mechanism, which is not costs at all: denomination

The diagnosis attributes the retune loop to free parameters
cross-calibrated against each other. That is one mechanism; here is a
second, cheaper to fix, with a proven precedent in this repo.

**The direction score has no normalisation.**

```rust
let preference = dot(dir, heading) * continuation_weight
               + dot(dir, photo)   * light_weight
               + dot(dir, wind)    * wind_weight
               + dot(dir, gravity_or_water) * upward_weight;
let score = preference / (1.0 + density * crowding_weight);
```

The four weights are absolute in the source and **relative in effect**,
because the score is used for weighted *sampling* over candidates — only
the ratios reach behaviour. So `light_weight` cannot be moved without
silently reweighting the other three, and nothing in the file says so. It
is a four-number simplex with three degrees of freedom, written as four
independent scalars per species per branch order.

**The engine already solved exactly this, elsewhere, and nobody
generalised it.** `INCOME_PER_NODE` was re-denominated in `L_node` units so
that *"it survives changes to `MAX_LIGHT`, `leaf_cluster` and the light
model the way its predecessor did not"*, and `MAINTENANCE_PER_NODE` sits in
the same currency so the two are directly comparable without a conversion.
That is the medicine for the retune loop, applied to the carbon economy and
not to the score.

So the diagnosis's *batch by shared tuning surface* is right, and there is
a stronger version available for at least this surface: **make the shared
budget explicit and the batch stops being shared.** Normalising the four
weights is a mechanical change with no behaviour change at the current
values, and it turns "changing one weight retunes five species times four
orders" into "changing one weight moves one species along one axis". It is
also what makes the banked phototropism patch affordable.

## 8. The instrumental half: what this does and does not buy for diversity

Stated because the costs work is instrumental and should not be sold as
something it is not.

A cost creates a real trade-off only when three things hold: the cost
exists; it is paid in the same currency as the benefit, so the two are
comparable at the margin; and **the environment varies which arm wins**.
The third is `plant-simulation-research.md` §7b's multi-task result, and it
means equilibrium and diversity are *not* the same question. Conditions 1
and 2 buy equilibrium — the system settles instead of running away, which
is what makes the next feature affordable. Only condition 3 buys diversity.

The engine has one axis with all three, measured: `LOCUS_LEAF_ECONOMY`
pairs `LEAF_RATE_ALLELES` with `LEAF_TRANSPIRATION_ALLELES` at every
consumer, and `plant-genome-design.md` §8d records a full 2x2 where the
sign flips — acquisitive wins wet (+21% mass, +32% seed), conservative wins
dry (+43% foliage retained), and the acquisitive stand drinks its own bed
to the wilting point. Neither allele wins everywhere, so selection cannot
saturate it. Recorded honestly in its own report as one world seed each
way, with the dry margin on *total mass* only +4% and the claim resting on
three other quantities that agree.

`LOCUS_WOOD_DENSITY` has conditions 1 and 2 and **condition 3 is
unverified**: cheap wood measured +46% mass and +52% seed, dense wood's
advantage is a longer loaded cantilever, and nothing on record shows a
scene where that pays.

**So: a costs pass will make features composable. It will not, on its own,
produce plants that look different from each other**, and if it is sold
that way it will read as a failure when it lands.

## 9. What to do, in order

Nothing here is proposed for landing in this session.

1. **Fix the denominator, and use the competitive bed.** §5b's bed is
   `trees=24 width=512` on the shipped `plant_probe` — no build, no
   feature. But §5b also shows stand seed totals are pinned by world
   width, so they can never have been a fitness or equilibrium signal:
   output is `seed_chance x total mature cells`, total mass is bounded by
   intercepted light, and that is fixed by the width of the world. **The
   stand's output is set by the resource; morphology decides who gets it.**
   Until the readout is a genotype's share against its own competitors, a
   costs pass measured on stand totals will report nothing whichever way it
   goes. This is the prerequisite for reading steps 2–4 and it is the
   cheapest item on the list.
2. **Couple `rate` and `transpiration` at the authored layer**, the way
   `WOOD_DENSITY_ALLELES` couples strength and price. Smallest diff on the
   list, states a principle the engine already holds one layer up, and is a
   real counterweight rather than a tax.
3. **Charge carbon for construction** — settled by the owner as
   *charge at the decision* (§10a), so this is now three ordered pieces
   rather than one:
   - **3a. Leaves.** `plant.rs:2411` asks for exactly this and defers it to
     *"the single pass that sets it"*. Per-tissue coefficient, leaf dearest
     (§10a.1). This is the piece that makes leaf payback time exist, and
     with it, leaf *placement* a decision.
   - **3b. Thickening.** `thicken()` spends nothing today. Same charge,
     lower coefficient.
   - **3c. Only then, stop billing interior wood.** Heartwood is dead
     tissue and should not pay `MAINTENANCE_PER_CELL`. **Strictly after
     3a/3b** — that constant exists to stop blob interiors standing for
     ever, and removing it before construction is charged resurrects the
     blob (§10a.3).

   Shared-budget throughout: re-deriving `INCOME_PER_NODE`,
   `MAINTENANCE_PER_NODE`, `Grow.cost` and `supportable` is part of the
   work, gated on a seed sweep either side.

4. **Route reproduction through the surplus pool** (§10b option C), which
   is the same fix as 3a one account over: seeds should draw from
   `(income − maintenance)` like growth, not from a mature cell's
   effectively bottomless stock. Makes `seed_maturity` redundant rather
   than requiring a decision about it, and does not depend on competition
   being live. `max_active_tips` and `MAX_ROOT_FRACTION` can be revisited
   as fences afterwards, with `turgor_taper` as the in-repo model for
   giving a stop a graded edge.

5. **Normalise the direction score** (§7) as a separate no-behaviour-change
   commit, *before* anyone touches `light_weight` again.
6. **Only then** the heritability survey, ranked on §2's four verdicts
   rather than on effect size — a lever in the "benefit, no cost" row is
   disqualified from heritability until step 3 or a bespoke counterweight
   reaches it, because a free lever made heritable produces uniformity.

## 10. Owner calls — three answered 2026-08-27, and what they change

### 10a. Construction charging: settled — charge at the decision

**Owner's ruling: charge at the decision.** *"I don't care about keeping
numbers the same now. I care about the long term architecture being
correct."* The alternative — attributing the pooled bill back to the
decision that caused it — is withdrawn.

**The biology agrees, and says the engine is missing a standard term.**
Plant physiology splits respiration two ways (the McCree–de Wit–Penning de
Vries–Thornley paradigm, reviewed in Amthor, *Annals of Botany* 86:1–20,
2000):

```
R = growth respiration      -- proportional to NEW tissue, paid at construction
  + maintenance respiration -- proportional to EXISTING tissue, paid per tick
```

This engine has the second and not the first. Building a gram of tissue
costs roughly 1.25–1.5 g of glucose; the surplus over what is incorporated
*is* growth respiration and it is paid when the tissue is made, not
amortised. So charge-at-the-decision is not merely the simpler option, it
is the missing half of the textbook model.

Three consequences beyond "charge for leaves", all of which change §9 step 3:

1. **Construction cost is per tissue, and leaves are dearest.** Roughly
   1.4–1.6 g glucose/g for leaf against 1.2–1.4 for stem and root (Penning
   de Vries et al. 1974; Poorter 1994), because of protein and lipid
   content. A per-tissue coefficient is the accurate model, not one flat
   number.
2. **What construction cost buys is payback time, and that is why it
   matters here.** A leaf's payback time is construction cost over net
   carbon gain, and a leaf must outlive it or it is a strict loss — the
   spine of the leaf economics spectrum this engine already half-models
   through `LEAF_RATE_ALLELES`/`LEAF_TRANSPIRATION_ALLELES`. **Payback time
   is currently undefined here, because a leaf costs nothing to make**, so
   nothing makes it a mistake to put a leaf in your own shade. Charging
   construction is what turns leaf *placement* into a decision, and is the
   route by which self-pruning could fall out instead of being a shed rule
   bolted on.
3. **Wood is inverted, and fixing it is a package with a strict order.** In
   biology wood is expensive to build and nearly free to keep: heartwood is
   dead and does not respire, and only sapwood's living parenchyma does,
   scaling with the crown it supplies — Shinozaki's pipe model, which
   `pipe_ratio` and `q_peak` already implement. This engine has it the
   other way round: `thicken()` builds for free and every wood cell then
   pays `MAINTENANCE_PER_CELL` for ever, so a large old tree is punished
   for bulk that in reality costs it nothing.

   **But the second half cannot be done first.** `MAINTENANCE_PER_CELL`
   exists precisely so that *"abandoned wood and blob interiors — which
   have `q_peak ≈ 0` — would be free to keep standing for ever, and the
   die-back would have nothing to remove."* Making heartwood free today
   resurrects the blob. Charge construction first — that stops the blob at
   the front end — and only then is it safe to stop billing interior wood
   rent it does not owe.

**Deliberately not modelled:** real construction cost is frequently
nitrogen-limited rather than carbon-limited, especially for leaves. A third
currency is not worth it; carbon and water already give the Liebig minimum
that makes the leaf-economy crossover work.

### 10b. Reproduction: the options, and a finding that reframes them

**Verified while writing this, and it changes the question: reproduction
and growth do not compete at all today.** `Behavior::Reproduce`
(`plant.rs:4975`) reads the carbon of the *cell it runs on* and debits
`seed_cost` from that. Mature cells sit pinned at `RESOURCE_SCALE` — the
engine's own diagnostic says so, *"the trunk pinned at the `RESOURCE_SCALE`
cap. The plant was never out of carbon."* Meanwhile `allocate_to_frontier`
distributes the surplus pool **only to frontier cells**. Two separate
accounts: a mature plant reproduces out of a stock that is effectively
bottomless, and `seed_cost: 0.3` is not a price in any meaningful sense.

That is *why* the fence has to exist. The three options, with that in view:

| | what it is | what it costs you |
|---|---|---|
| **A — keep the fence** (today) | `seed_maturity: 600` — zero seeds below 600 shoot cells, full rate above | every species sits at its authored integer, the integer is arbitrary, and "reproduce early and cheap" is not a strategy that exists in the game at all. There can be no weed |
| **B — price it** | drop the threshold, make a seed expensive relative to a small plant's budget: the size-versus-fecundity trade, r/K, Grime's ruderal↔competitor axis | **only bites if being small is punished.** With no competition and no disturbance, reproducing at two cells is strictly better and everything becomes a weed — the same shape as turgor in §5, a price whose counterweight is switched off |
| **C — route it through the surplus pool** | seeds draw from the same `(income − maintenance)` pool growth draws on, instead of from a mature cell's stock | nothing obvious. Reproducing *is* not growing, automatically |

**C is the recommendation.** It is what biology does — reproduction is
allocated from surplus after maintenance, and the cost of reproduction
(reproducing trees measurably grow less that season) is among the
better-documented allocation trade-offs. It makes `seed_maturity`
*redundant* rather than requiring a decision about it: a two-cell plant has
almost no surplus, so it can barely reproduce, with no rule saying it may
not. Unlike B it does not depend on competition being live, because it is
an internal allocation constraint rather than an external one. And it is
the smallest diff of the three — **the same fix as the leaf: charge the
decision against the account it should have been drawing on.**

### 10c. Wood density: answered, and the real question restated

**Owner's answer: yes in theory, not now.** Disturbance and branch-breaking
are partly implemented and will be fleshed out later; that is a
structural-line concern and not a blocker here. The "does dense wood ever
need to win" framing is withdrawn.

**The question he actually wants carried forward** is whether wood density
survives a genome rebuilt from general behaviours rather than named levers
— i.e. whether someone would be right to argue the lever is not worth its
cost, or is too tree-specific.

Recorded assessment: **density is not tree-specific and should not stay a
named locus.** The strength-versus-cost trade applies to any structural
tissue — a grass culm, a moss stem — and it already half-lives in the right
place: `Material::density` and `Material::max_cantilever_reach` are on the
material, and `WOOD_DENSITY_ALLELES` only scales them per individual. Under
a behaviour-general genome that survives as *"this tissue is dense"* rather
than *"this tree has dense wood"* — a relocation, not a cut. The one thing
that would make it genuinely not worth its cost is if nothing ever loads
structural tissue, which is the half the owner says is coming.

### 10d. Still open

From `plant-evolvability-handoff-2026-08-27.md` §6, unchanged by this
audit: what replaces the withdrawn §6.2 (what *should* be heritable);
which clades and in what order; and whether the niche table keeps naming
species.
