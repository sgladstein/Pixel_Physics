# Closing the loop: what a plant and an animal could be to each other

*Design lane, 2026-09-10. No engine code — this is a priced specification, a
build order, and one owner ruling put to the queue. It answers
`evolution-lab-what-is-missing-2026-09-09.md` §4 ("the ecology has no
relationships") with mechanisms rather than a list, and it is scoped by
`evolution-lab-direction-2026-09-09.md`, which is the direction of record and
whose rulings are named where this depends on them. Every number below marked
**(measured)** was taken on this branch on 2026-09-10 at
`RAYON_NUM_THREADS=1`; every number marked **OWED** has not been taken and
names the run that would take it.*

**Status: proposals and prices only. Nothing is built, nothing is decided.**
Two review cards carry the two decisions that are not a lane's to make:
`20260910T032635178Z-6dfed9` (pollen as gene flow — §4) and
`20260910T032808971Z-cba50c` (which form of seed dispersal — §2).

---

## 0. The answer

**The box has two halves of one relationship built and unjoined, and the join
costs one field and two one-call hooks.** A flower is a terminal organ with a
colour and no function; a fruit falls, an ant carries it home, and the seed
inside is destroyed at the moment of pickup. Joining them — the seed survives
the animal — makes the colony the plants' distribution network and the plants
the colony's renewable larder, which is the ecological answer to *"over a long
enough time everything dies out"*.

The headline to build toward is **THE COLONY THAT GARDENS SURVIVES**: a ring
of herbs around a nest, flowers turning to fruit after visits, fruit carried
home in a line, a bed whose planting map is a record of where the colony
foraged.

**Three measurements taken for this document change what should be built.**

1. **The fruit pipeline is budget-limited, not animal-limited.** A fruit was
   ripe and the plant could not afford to fill it **18,867 times in 40,000
   frames**, against **56 fruit that dropped**; with no colony in the box the
   refusals *double*. So a pollination bonus on the ripening **clock** buys
   nothing — the bonus has to go on the **price**. This is not what
   `what-is-missing` §4 assumed and it is the load-bearing correction here.
2. **Nothing has ever eaten a flower.** The best mouthful any ant swallowed in
   40,000 frames is **960 — a fruit**, not the 1,440 flower, reproducing
   `dead-ends.md` line 1636 at a different bed. **That is what makes B
   cheap**: the 1,440 calibrates a printed ceiling and a doc comment, not any
   measured behaviour.
3. **A fallen fruit does not lie around waiting to be found** — mean residence
   on the floor **293 frames with a colony, 68 with none**. The colony is not
   the main sink, and nothing in the repo can say whether the other exit is
   rot or germination. *Which exit a fruit takes* is the whole of §2, so **the
   first deliverable is an exit census, not a mechanism.**

### The priced changes

| # | change | files | new lines | constants to re-derive | what the player sees | decided by |
|---|---|---|---|---|---|---|
| **A0** | **Windfall exit census** — split "a windfall left the world" into germinated / rotted / eaten / carried | `examples/windfall_probe.rs`, three counters on `World` | ~60 | none | nothing | the three exits sum to creations; a `colonies=0` arm shows *eaten* fall to zero |
| **A1** | **The seed survives the mouth** — a bitten windfall leaves a `pip` where it was bitten instead of a hole, at a species-set chance | `assets/materials/pip.ron` (new), `src/sim/plant.rs`, `assets/species/herb.ron`+`scrambler.ron`, **one hook** at `creature.rs:5001` | ~120 + 40 asset | `herb.reproductive_allocation` **0.28** (§2.5) | a fruit eaten leaves a pale seed behind, which sprouts | plants established from pips > 0; ant-fed bed's stand vs `colonies=0` |
| **A2** | **The seed rides home** — the crop carries one seed, deposited with the cell it is dropped with | `src/sim/organism.rs` (`Crop`, one field), **two hooks** at `creature.rs:5001` and `:5137` | ~90 | as A1, plus `herb.seed_half_life` **14,000** checked against transit (§2.5) | a line of ants carrying fruit; herbs coming up around the nest | plants established within 32 cells of a nest, paired against `colonies=0` |
| **B1** | **Nectar** — a flower an animal feeds at pays a small meal out of the plant's reproductive pocket and **survives** | `src/sim/plant.rs`, `assets/materials/flower.ron`, **one hook** at `creature.rs:4954` | ~110 | `flower.food_energy` **1,440** (§3.2), `EAT_YIELD_THRESHOLD` **12.0** | ants working a flower head instead of stripping it | flower standing stock; ant intake; *organs built* must not fall |
| **B2** | **Pollination as a discount** — a visited organ's `Ripen` cost is graded down, so a visited plant actually sets fruit | `src/sim/plant.rs`, `assets/species/*.ron` (one field), one `u8` on the organ cell | ~70 | `herb` `Ripen(cost:)` **0.3 / 0.35**, `reproductive_allocation` **0.28** | flowers turning to fruit where the ants go, and not where they don't | fruit dropped, split visited/unvisited; `organ_ripening_blocked` |
| **C-brush** | **A BRUSH tool** — carry pollen from flower to flower by hand; the seed is a cross | `src/lab/ui.rs`, `src/sim/organism.rs` (`cross`), `src/sim/plant.rs` | ~200 | none | a plant that is visibly both its parents | **owner ruling, card `…6dfed9`** |
| **C-animal** | **Animals carry the pollen**, default off | one hook in the same site as B1 | ~40 on top of C-brush | the clustering instruments (§4.3) | a bed that hybridises itself | **owner ruling, card `…6dfed9`** |
| **D** | **Palatability from foliage tone** — the leaf's price, already heritable and already visible as colour, is read by the gut | `src/sim/creature.rs` (`diet_yield`), `assets/materials/leaf.ron` | ~40 | `EAT_YIELD_THRESHOLD` **12.0**, `diet_yield`'s whole worked table, `LOCUS_ALLELES[0]` **2** | two kingdoms drifting apart in colour | a paired sweep: leaf-tone allele frequency with and without grazers |
| **E** | **A nectar-feeder up the stems** | `assets/species/*.ron` after articulated bodies land | one `.ron` | — | an animal that lives where the ants cannot | sketch only |

**Build order: A0 → A1 → A2 → B1 → B2 → (C on the ruling) → D → E.** Plant
side and instruments first; every creature-side change is one call at a named
site, added after `claude/creature-evolution-engine-67lhjp` lands. §7 has the
briefs.

---

## 1. What the box actually does today

**Measured 2026-09-10, `RAYON_NUM_THREADS=1`, release, this branch at
`origin/main` = `09da43d7`. One world seed — see the seeding defect in §8.1,
which is why it is one.**

`labshot scenario=played_bed frames=6000,20000,40000` — the mix the owner
plants (grass, herb, shrub; **four of the thirteen plants are herb**, the only
shipped species that fruits), colony landing at frame 6,000:

| frame | plants | ants | carrying food | standing flower | standing fruit | seeds |
|---|---|---|---|---|---|---|
| 6,000 | 617 | 33 | 0 | **60** | 8 | 746 |
| 20,000 | 715 | 25 | 23 | **59** | 2 | 1,325 |
| 40,000 | 543 | 54 | 41 | **66** | 3 | 1,912 |

**Sixty flowers stand in the box at every stop and no animal has any reason to
approach one.** Standing fruit is 2–8. So the *pollination surface* is an
order of magnitude larger than the fruiting surface — which is why B is worth
more than its price suggests, and why B's failure mode is not "too few
flowers".

`windfall_probe frames=40000 seed=1` on the harness bed (8 herb founders, one
colony of 52), against its own `colonies=0` control:

| | with the colony | no colony |
|---|---|---|
| organs built (flower + fruit set) | 200 | — |
| **ripening refused for want of budget** | **18,867** | **36,886** |
| windfalls created | 56 | 39 |
| mean standing flower / fruit / windfall | 18.3 / 16.7 / 0.61 | 42.0 / 15.9 / 0.22 |
| mean time a windfall stands on the floor | **293 frames** | **68 frames** |
| plants alive at the end | 217 | 496 |
| pickups / deliveries / drops | 2,327 / 62 / 1,876 | — |
| best mouthful ever offered / swallowed | 292 / **960** | — |
| food standing within reach of a nest cell, at the end | **2** | — |

Four readings, each of which changes a design decision:

- **The colony halves the standing flower count (42.0 → 18.3) and halves the
  stand (496 → 217).** It is a grazer of flowering plants already; it simply
  gets nothing durable back and gives nothing back.
- **Ripening refusals are the bottleneck at both stages** — `plant.rs:5110`'s
  `Ripen` arm gates *both* flower→fruit and fruit→windfall on
  `reproductive_budget >= cost`, and with mean standing fruit of 16.7 and an
  organism cadence of ~45 frames, 18,867 refusals over 40,000 frames is very
  nearly *every standing organ, every tick*. The pipeline is starved of
  budget, not of time and not of pollinators.
- **`deliveries 62` against `pickups 2,327` is 2.7%.** §Z6's *"4 of 1,651"*
  was the pre-crop economy; the round trip is better than that and still thin,
  and every one of those 62 arrivals is a seed destroyed at the door.
- **The 1,440 flower has never been eaten**, at this bed and this length,
  reproducing `dead-ends.md` line 1636 exactly. `best offer 292` is the
  gut-filtered view; `best bite 960` is a fruit at face value.

---

## 2. A — gut and midden dispersal

### 2.1 Why the seed dies, to the line

`act` has no "eat here" path and "carry" path. It has **one bite**, at
`creature.rs:4954`, which clears the cell (`:5001`) and puts a whole cell into
`OrganismState::crop`; digestion (`creature_tick`, `:2967–3030`) and putting
it down (`:5100–5156`) are what differ afterwards.

A windfall is not an ordinary cell. Per `windfall.ron` and `plant::drop_organ`
(`plant.rs:2040`), **a windfall cell is an organism-owned `CellType::Seed`
wearing the species' `windfall_material`** — a live child organism with
alleles, a fate table, a lineage and an endowment. The bite destroys it twice:
`world.set(.., Cell::EMPTY)` takes its only cell and `reconcile_chain` frees
the slot. What goes into the crop is
`Crop { material, shade, unit, cells, digesting }` (`organism.rs:4249`) —
**five scalars with no identity in them** — and what comes back out at `:5137`
is `Carried::into_cell`, a bare `Cell::new(material, shade)` with organism id
0. That is the whole of "the seed does not survive the trip", and it is not a
bug: no field exists that could carry it.

### 2.2 The two forms, and which first

**A1 — the seed survives the mouth (dropped where the fruit was eaten).** At
the bite site, if the bitten cell is a `CellType::Seed`, the plant side is
asked whether the seed survives; if it does, the cell is converted to a `pip`
in place instead of being cleared, and the organism is *not* reconciled. The
ant gets its 960 J; the pip stands where the fruit lay and germinates on the
ordinary `Behavior::Germinate` path (`plant.rs:5006`).

**A2 — the seed rides home.** `Crop` gains one field, `passenger:
Option<SeedPassenger>`, filled at the same bite site and consumed at the drop
site (`:5137`), where the dropped cell becomes a live pip organism instead of
a bare material cell.

**Build A1 first and land A2 immediately behind it, as one arc.** A1 alone is
not dispersal — a fruit lies where it fell and the ant walked to it, so the
seed moves by one cell — but it is the whole of the plant side, it is testable
without a single creature-side line, and it is the version that survives the
creature lane's window. A2 is then one field and one extra call. **The owner's
fork is on card `20260910T032808971Z-cba50c`**; if the answer is "on the spot
is enough", A2 is dropped and the plant side is unchanged.

### 2.3 `pip`, and why a new material rather than reusing `seed`

The deposited object cannot be a `seed` cell, and the reason is arithmetic. A
windfall is worth 960 and a `seed` is worth 480 (`seed.ron`), so if eating a
fruit left a seed standing, one fruit would feed **1,440 J of ants and produce
no plant** — a one-directional pump, exactly the shape `Crop::unit`'s own
`min` guard exists to stop, and exactly the shape evolution finds.

`pip` at **`food_energy: 40.0`** closes it, checkably, before anything is
built. `diet_yield` (`creature.rs:3468`) multiplies face value by a gut-match
factor — 0.25 at the shipped neutral gut, ~1.0 at a full plant specialist —
against `EAT_YIELD_THRESHOLD` **12.0** (`:3517`):

- neutral gut: 40 × 0.25 = **10 < 12** — the shipped ant cannot see a pip at
  all. The pump cannot run.
- plant specialist (`gut_bias −1.0`): 40 × ~1.0 = **40 > 12** — a granivore
  can. Seed predation exists, and it is a niche difference expressed in one
  number rather than in a rule.

That is the material's justification under `flower.ron`'s own test — *a
material is warranted when its physics genuinely differ on numbers that
already exist*: a pip differs from a seed on food value (40 against 480), on
rot, and on colour, and the last matters most, because `flower.ron` records
five label changes in a row that fired, counted and read as nothing.

**The honest cost:** the world gains 40 J of standing plant food per fruit
eaten, because the ant is credited the full 960. Charging the pip against the
meal (960 → 920) was considered and rejected — `best_bite`'s 960 is what
`stamp_probe`'s birth arithmetic and `dead-ends.md`'s gut re-test condition
(*"at a 960-point fruit … 12 of 12 seeds"*) both rest on, and moving it
re-derives a creature constant to buy conservation in a quantity nothing
conserves: plant matter is outside `creature::standing_meat`, which counts
only materials with `worth_in_aux`.

### 2.4 The outcome must be a distribution

`CLAUDE.md`'s first law. **`seed_gut_survival`, a species field on the plant**
(`herb.ron`, `scrambler.ron`), default **0.6**, clamped to [0,1], rolled once
at the bite. Some fruit eaten destroys the seed; some passes. It is a plant
trait, not an animal one, because it is the seed coat that survives a gut.
Two properties, both checked rather than assumed:

- **It is a parameter the player can turn** — the owner's standing direction
  applied to an ecology rather than to an economy.
- **It is evolvable from the day it lands**, unlike `seed_launch`.
  `dead-ends.md` line 1708 records that a parameter no species authors is
  reachable only within `PARAM_REACH × 1.0 = 4` of zero, which put
  `seed_launch`'s useful range out of reach. A probability's range is
  [0,1] ⊂ [−4,4], so the fallback is *wider* than the range and that entry
  does not bite here. Say so in the field's own comment, or the next session
  will re-derive it.

The graded germination bonus after passage is **deliberately positional, not
physiological, in the first build**: a seed deposited at the nest is in dug,
tamped, un-shaded soil away from its parent, which is a real bonus the world
already computes. A scarification bonus on the child's `endowment` mints
carbon and wants its own measurement — a second knob, not a first one.

### 2.5 What the herb's economy re-derives

`CLAUDE.md`: *a mechanism at inherited constants is a regression*, and *a term
in a weighted sum is not an independent knob*.

- **`reproductive_allocation: 0.28` (herb.ron:221) is the one most likely to
  be found compensating.** It, `seed_cost: 0.18` and `seed_maturity: 60` were
  calibrated in a world where the colony is a **pure seed sink**: every fruit
  an ant took was an offspring deleted. A returns a fraction of those to the
  bank, so the herb's *realised* output per unit of budget rises with no gene
  changing. **The test is the paired `colonies=0` control and the failure it
  looks for is the stand running away**: if the ant-fed bed holds *more*
  plants at 120,000 frames than the ant-free bed, the colony is now a net
  source and 0.28 is over-set. Budget re-deriving it as part of A.
- **`Ripen(cost: 0.3)` on `Flower` and `0.35` on `Fruit` (herb.ron:252, :258)
  do not move for A** — nothing about A changes what an organ costs. They move
  for B2 (§3.3), and the two changes must not be measured in one run.
- **`seed_half_life: 14,000` (herb.ron:85) does not need re-deriving and does
  need checking.** A passenger's decay clock should keep running during
  carriage — a free, graded transit cost — but 14,000 frames against a carry
  of hundreds means transit costs approximately nothing, so the half-life is
  not what throttles A. If the median frames from pickup to deposit ever
  approaches four figures it becomes a live knob and the field's meaning has
  changed. **OWED**; nothing prints it today.
- **`fruit`'s 960 does not move** (§2.3), which is the point of `pip`.

### 2.6 What the player sees, and the counter that says it fired

**Sees:** a fruit an ant bites leaves a pale seed lying where the fruit was
instead of a hole (A1); an ant walking home with a fruit puts it down at the
nest and a plant comes up out of the midden (A2). At play zoom, over a
session, **a stand of herbs appears around the nest and along the route the
colony works** — which no still can show, so the card is a frame sequence or a
`filmstrip gif=1`, per the *movement, not stills* ruling.

**Counters, both halves** — `CLAUDE.md`'s "pair every *it fired* counter with
an effect counter from the far side of the call". It fired: `seeds_spilled` (a
bite met a seed and the roll passed) and `seeds_delivered` (a passenger was
put down; A2 only). It worked: **`plants_from_pip`**, plus `pips_rotted` and
`pips_eaten` so the three exits sum to spills. A null on `plants_from_pip`
with a healthy `seeds_spilled` is the mechanism firing into ground where
nothing germinates — a scene finding, not a code one.

### 2.7 The measurement that decides it

`labforage scenario=played_bed seed=1,2,3 frames=120000`,
`RAYON_NUM_THREADS=1`, paired against the identical binary ablated by
`seed_gut_survival: 0.0` — **an ablation switch that is a species value costs
nothing and keeps the two arms bit-comparable**, the control shape
`method-worked-cases-2026-09-05.md` recommends over adding a metric. Ships if,
over three or more seeds:

1. **plants established within 32 cells of a nest** is higher on a majority of
   seeds — the headline made into a number, and the one column a still cannot
   fake;
2. **the stand does not run away** — total plants at 120,000 frames inside the
   `colonies=0` band, or `reproductive_allocation` is re-derived in the same
   change;
3. **the colony is not worse off** — `deliveries`, `eats` and animals alive
   unchanged or better. A dispersal mechanism that costs the colony food has
   taken the mutualism apart from the other end.

Read `plants` and `plants_from_pip` at two consecutive stops before believing
either; `CLAUDE.md`'s settling rule applies to a stand as much as a collapse.

---

## 3. B — nectar and pollination

### 3.1 The trap it must not walk into

`what-is-missing` §4, from `dead-ends.md`: *any pollination **requirement** is
a new failure mode for reproduction in a box whose animals already die out.*
So **a visit is a graded bonus and never a gate**: a plant with no pollinator
fruits exactly as it does today, there is no arm in which a flower can fail to
set for want of an animal, and no default-off switch is needed because there
is nothing to switch off.

### 3.2 Nectar: what the 1,440 actually calibrates

Checked site by site, `flower.ron`'s `food_energy: 1440.0` reaches four places
and **only two of them move**:

- **`best_bite`'s doc** (`creature.rs:~3877`) uses it as the worked example
  for *best, not first* — *"a leaf pays 480 and a flower pays 1,440"*. The
  **example** must be rewritten; the **rule** is untouched, since fruit 960
  against leaf 480 still carries it.
- **`creature_probe`'s printed reachability ceiling** is 1,540 at
  `gut_bias −1.0` on the strength of the flower, and `dead-ends.md` line 1636
  records that as *the* trap. The ceiling falls and the trap goes with it —
  a repair, not a cost.
- **`stamp_probe`'s 580** does not read the flower at all (a flower up a stem
  is standing but not reachable), and **`dead-ends.md`'s gut re-test
  condition** rests on fruit at 960. Neither moves.
- **`EAT_YIELD_THRESHOLD = 12.0` moves.** Nectar is the first sub-100 J meal
  in the box and 12.0 is the line deciding whether anything can see it — and
  the constant's own comment already asks for this: *"a threshold set from an
  argument is exactly the shape this project has been bitten by"*.

So the summary is the opposite of the intuition: **the 1,440 calibrates one
ceiling already recorded as misleading and one doc comment.** No measured
behaviour rests on it, because nothing has eaten one (§1). That is what makes
B cheap.

**The design.** A flower an animal feeds at pays **`nectar_yield`, default
120 J** — a quarter of a leaf — **out of the plant's `reproductive_budget`,
and survives.** Refill is the flower's own per-cell `ripeness` clock, so a
just-drained flower pays less until it comes round: that is the grading.
`flower.food_energy` **stays at 1,440** for an animal that bites the cell off
— destroying a flower is still a huge meal and still ends the flower. Nectar
is the alternative, not the replacement.

Two things this buys that are not obvious. It is the **first renewable food in
the box** — every other calorie is a whole cell that leaves the world when
eaten — so it is the first resource a colony could hold a territory around
rather than mine out. And charging it to `reproductive_budget` puts nectar in
**the same pocket as fruit and loose seed**, which is already how `Ripen` and
`Reproduce` compete: a plant that feeds animals sets fewer seeds, with no new
accounting anywhere.

### 3.3 Pollination: the bonus goes on the price, not the clock

`organ_ripening_blocked` fired **18,867 times in 40,000 frames** against 56
drops, and the block is `budget < cost` (`plant.rs:5171`, `:5191`). A bonus
that adds ripeness does nothing to a fruit that has sat ripe for ten thousand
frames waiting to be afforded.

**So a visit discounts the organ's ripening cost.** One saturating `u8` of
visits on the organ cell; effective cost
`cost × (1 − pollination_discount × visits/VISIT_SAT)`, with
`pollination_discount` a species field around 0.5. A well-visited flower sets
fruit at half price; an unvisited one sets fruit exactly as today. **That is
"sooner, fuller" expressed against the constraint that is actually binding**,
graded three ways: by visits, by the plant's budget, and by the species value.

**The shared-budget trap applies and must be priced before the work starts**
(`CLAUDE.md`, the `phototropism_dir` case). `reproductive_budget` funds loose
seed, flower construction and ripening alike, so a cheaper ripening
**reallocates the pocket toward organs and away from loose seed** even though
no constant's meaning changed — and `seed_cost: 0.18`,
`reproductive_allocation: 0.28` and `seed_maturity: 60` were all calibrated
with organs at full price. Name them in the brief and budget re-deriving them;
if that is unaffordable the change is not scoped, it is merely started.

The two halves of B pull against each other by design — nectar *costs* the
budget, the discount *saves* it — so **the net must be measured, not argued**.
The prediction to falsify: visited plants fruit more than today, unvisited
plants slightly less, total fruit up. If the total falls, `nectar_yield` is
over-set or the discount under-set, and the sweep is over those two.

### 3.4 The hook, and what the player sees

One call at the swallow block `creature.rs:4954`, before the bite lands: if
the best offer is a `Flower` cell, ask the plant side for nectar; if it pays,
credit it, mark the visit, and the flower stands.

**Sees:** ants working a flower head and leaving it standing instead of
stripping it; and over a session, **fruit on the plants near the colony's
routes and not on the ones it never reaches** — a pattern in the bed the
player made by placing a nest, visible with no overlay.

**Counters:** `flower_visits` and `nectar_paid` (it fired); `organs_built` and
`fruit_dropped` **split visited/unvisited** (the effect).
`organ_ripening_blocked` must fall for visited plants and hold for unvisited
ones — a fall in both is the discount leaking.

**Decided by:** `labforage scenario=played_bed`, 3+ seeds, 120,000 frames,
ablated by `nectar_yield: 0.0`. Ships if fruit dropped rises without the
colony's intake falling, and if the visited/unvisited split is real rather
than a proximity artefact — **the control for that is a compartment with
flowers and no colony in the same run**, not a second seed.

---

## 4. C — pollen as gene flow

**This is an owner ruling, not a lane decision, and it is on the queue as
`20260910T032635178Z-6dfed9`.** Both readings are given honestly below; the
recommendation follows; nothing is built until the card comes back.

### 4.1 Reading one: the animals carry it

A visited flower's seed carries genome slots from the last plant that animal
visited. Mechanically it is small: a `pollen: Option<PollenPayload>` on the
animal, written at the nectar hook, read at fruit-set, and a uniform crossover
over the two parents' allele arrays.

**What it buys** is *the breeding fantasy realised through the ecology* rather
than through a menu. Two lines planted in one bed with a colony between them
start producing plants that are neither parent, with no player action;
isolating a compartment becomes an evolutionary event with a visible
consequence; the player gets a lever with two sides, because moving the nest
changes which plants cross. It is the strongest single answer to *"the box has
no relationships"* — the only proposal here in which an animal changes what a
plant **is** rather than where it goes.

**What it costs is not a tuning cost.**

- **It contradicts the direction of record.**
  `evolution-lab-direction-2026-09-09.md` §3: *"Mating in the world — hard
  dead end (asexual budding **is** the isolation). Only the shelf verb."*
  Only the owner can move that.
- **It costs the species concept, and worse, the instrument.**
  `plant-evolution-design.md` §6: with clonal inheritance there is no gene
  flow, so a "species" here is defined operationally as *a persistent,
  self-maintaining cluster in genotype space with a niche it holds* — and the
  three instruments built on that definition (allele-frequency multimodality
  against the clonal drift band, niche fidelity, the reciprocal transplant)
  are the lab's **only** way to say two lines have separated. Gene flow across
  a shared bed is precisely what dissolves clusters, so every allele census
  taken to date becomes uninterpretable in a bed where pollen moves.
- **It pre-empts the shelf's `CROSS`.**
  `evolution-lab-genetics-2026-08-31.md` §6.1 designed crossover across one
  shared scaffold *so that* `CROSS` is representable on the shelf — the
  player's scissors — and Arc C2 states it as *"World stays asexual; player
  gets the scissors."*

### 4.2 Reading two: the player carries it — the BRUSH

**The hand version is the same mechanism.** A `BRUSH` tool on the lab's bar:
click a flower and the brush loads that plant's genome; click another and that
flower's fruit sets a seed that is a uniform crossover of the two. It is
`evolution-lab-direction` B6 (*"hands on the colony"*) extended from the
colony to the bed, and `CLAUDE.md`'s second law exactly: **a verb, and what it
produces is a plant that is visibly both its parents.**

**What it buys.** `CROSS` realised physically in the bed at a fraction of a
shelf UI's cost, done where the plants are. Gene flow happens only where the
player puts it, so **an isolated compartment stays an isolate and the player
becomes the isolating mechanism** — a better game than a bed that hybridises
itself, because it makes isolation a thing you *do* — and the clustering
instruments survive intact, since a bed with no brush strokes is still clonal.

**What it costs.** It is not a relationship: the colony gets nothing from it,
and neither does the headline. And it is the more expensive half in lines — a
tool, a cursor state, a bar slot and a page row, ~200 against ~40. **The
animal half is the cheap one once the brush exists.**

### 4.3 Recommendation

**Build the brush first; build the animal half second, and only if the owner
reverses the standing dead end.** Three reasons, in order of weight:

1. **The brush builds the animal half's engine.** `PollenPayload` and
   `organism::cross(a, b)` are 100% shared, so the animal version is then one
   hook and one field. Nothing is wasted either way and the irreversible
   decision is deferred to the cheap moment.
2. **The cost of reading one is an instrument, and instruments are what this
   lab is** (*"give me the tools, data, access"*). Trading the only
   operational definition of plant speciation for a behaviour is a trade only
   the owner can price.
3. **Default-off is not enough on its own, and that is the honest part.** It
   still splits every future measurement into two worlds, and `CLAUDE.md`'s
   record is that ship-it-off is how the trail circuit stayed inert for four
   rounds. If the owner wants animals carrying pollen it should ship **on**,
   with the clustering reports re-run against it — a real cost, and the reason
   this is a card.

**One live defect either version must handle first.** Petal colour is drawn
per individual from `ORGAN_BAND_STREAM` (`plant.rs:2302`, `:2711`) and **is
not heritable** — `organism.rs:5104–5120` says so and calls giving it a locus
*"a genome change"*. So a hybrid will not look like a blend of its parents,
however convincingly it is one, and **a crossing verb whose product looks
random is the sixth label change in a row that fires, counts and reads as
nothing** (`flower.ron`'s own recorded failure mode). Either C ships with a
`LOCUS_FLOWER_COLOUR` — a genome change, priced separately, belonging with the
heritability survey — or the hybrid is made legible another way and the card
says which. **This is the largest hidden cost in C and it is not in the line
counts above.**

---

## 5. D — herbivory defence, price only

**The proposal.** `diet_yield` (`creature.rs:3468`) prices a mouthful from
`MaterialDef::food_class` and the animal's `gut_bias`. Leaf is one material
for every plant in the world, so every leaf tastes the same. But
`Cell::shade` **already carries the individual's foliage band**, and the
foliage band is derived from `LOCUS_LEAF_ECONOMY` (`organism.rs:6073`) — a
real, heritable, two-allele gene whose alleles are *acquisitive*
(`LEAF_RATE_ALLELES` 1.2, transpiration 1.5) and *conservative* (0.85, 0.7).
The wiki already states it in the player's words: **foliage tone is the
leaf's price.**

So the coupling is free of new state: **`diet_yield` scales palatability by
the leaf's own tone**, cheap acquisitive leaves worth more to a mouth,
expensive conservative leaves worth less. That is the real leaf-economics
prediction, not an arbitrary correlation — long-lived leaves are tougher — and
it is two kingdoms evolving against each other in a channel the player can
already see on screen.

**Price.** ~15 lines in `diet_yield` for the term, ~10 more for
`MaterialDef::palatability_from_shade` plus a line of `leaf.ron`. The hot-path
guard is **free**: `CLAUDE.md` says put the opt-in on `Material` and test it
at the call site that already holds the `Cell`, and `best_bite` already holds
it and already calls `diet_yield` eight times a creature tick — a `Vec` index,
no `World::get`.

**Constants that must be re-derived, and one that is the real cost.**
`EAT_YIELD_THRESHOLD = 12.0` moves, because a leaf stops being one number and
the bar deciding what is food at all now has two answers per leaf; and
`diet_yield`'s worked table (`:3505–3509`) is the doc every later session
reasons from and has to be rewritten. **The real cost is
`LOCUS_ALLELES[0] = 2`**: two alleles is a *binary* defence axis — thriving or
gone, the defect the ethos names — and a graded one needs three or more, which
`organism.rs:6094` records was deliberately cut *down* from six to remove a
mutation bias (uniform draw over six against a consumer clamped to two bands).
Widening it re-opens that bias, and every species' `foliage_bands (first,
count)` needs a third band with it.

**Decided by:** a paired sweep, never a single run — leaf-tone allele
frequency in the standing population with grazers and with `colonies=0`, 3+
seeds, 120,000 frames. The claim is *selection*, so the null is drift, and
`selection_arena`'s standing finding applies: for plants a null is a statement
about the world rather than about the genome. **A positive control is
mandatory and cheap** — set the coupling absurdly strong and confirm the
allele frequency moves at all before believing the shipped strength.

**Sequence:** after B, not before. D's premise is that a mouth choosing
between plants matters, and today the mouth's best choice is a fruit it can
rarely reach.

---

## 6. E — a niche by height, sketch only

`food_height` measured **95.8% of all food worth five or more cells up** on
`wetland` and 87.6% on `rolling`, and §1 measured organs standing to **53
rows** while an ant's head reached **38**. Every food cell is nonetheless
worth the same to every animal at every height, which is why the box has no
niches — the direction says so when it explains what a queen does *not* fix.

**The sketch:** a nectar-feeding hopper that lives up the stems. The jump
exists and is wired (`hopper.ron`, 2,551 launches in its first 3,000 frames).
B1 gives it a food that is only up there and that **renews** — the difference
between a niche and a one-off meal, since an animal cannot hold a territory
around a resource that leaves the world when eaten. It would be the first
animal in the box whose living is made where another cannot reach.

**Not before** B1 lands, articulated bodies land
(`claude/creature-evolution-engine-67lhjp` owns `hopper.ron`), and the creature
line sets the jump rate — a bias of 2.0 is a hop on two ticks in three and it
kills the animal inside a session. **Cost once those hold: one `.ron` file**,
which is the point of writing it down now.

---

## 7. Build order and briefs

**Order.** *(the `labshot` seed fix, §8.1, three lines, whoever is first
here)* → A0 → A1 → A2 → B1 → B2 → C (on the ruling) → D → E. Plant side and
instruments first; every creature-side change is one call at a named site,
added after `claude/creature-evolution-engine-67lhjp` lands. Each brief below
is under 400 words and is meant to be handed to a Sonnet lane as-is.

**Every brief carries the same cost fork, and it is not decoration:** *build
it, or write the finding and stop; never a half-built fix.* A half-finished
`plant.rs` on `main` costs every concurrent session.

**Every brief carries the same visibility rule:** animals are judged on a
moving sequence — `filmstrip gif=1`, or a frame sequence on the card — never a
still. Owner's ruling, 2026-09-09.

### Brief A0 — the exit census (do this first, it is not optional)

*Owns:* `examples/windfall_probe.rs`, three counters on `World`.
*Question it answers:* a windfall leaves the floor in 68 frames with no animal
in the box. Does it germinate, rot, or get eaten? The whole of A is about
which exit a fruit takes, and nothing in the repo can currently split them.
*Build:* count the three exits at their sites (`plant::germinate`,
`decay.rs`'s windfall arm, `creature.rs`'s bite) and print them beside
`fruit_dropped`. *The gate:* the three exits plus standing stock must sum to
creations — a residual is a fourth exit nobody knew about and is itself the
finding. *Positive control:* `colonies=0` must take *eaten* to exactly zero
while the other two move; if it does not, the counter is in the wrong place.
*Measurement:* `windfall_probe frames=40000` with and without a colony, one
seed is enough for a conservation check. *Card:* none — nothing visible.
*Cost fork:* if the residual will not close in a day, write the residual down
and stop; a census that does not conserve is worse than none.

### Brief A1 — the seed survives the mouth

*Owns:* `assets/materials/pip.ron` (new), `src/sim/plant.rs`,
`assets/species/herb.ron`, `assets/species/scrambler.ron`.
**Does not touch `creature.rs`** — write the hook as a one-line patch in the
report and hand it on.
*Build:* `plant::seed_survives_bite(world, x, y, rng) -> bool` — if the cell
is a `CellType::Seed` wearing a windfall material and `rng.chance(species
.seed_gut_survival)`, convert it in place to `pip` and keep the organism;
otherwise return false and let the caller clear it. `pip.ron`:
`food_energy: 40.0` (§2.3 — check the arithmetic against
`EAT_YIELD_THRESHOLD` in the file's own comment), a distinct pale colour band,
faster rot than `seed`, `insubstantial`, `falls_through_organisms`.
`seed_gut_survival: 0.6` on both fruiting species with the
`param_scale`-fallback note from §2.4.
*Constants to re-derive:* `herb.reproductive_allocation` **0.28** — measure
the ant-fed stand against `colonies=0` and re-derive if the stand runs away.
*Counters:* `seeds_spilled` and `plants_from_pip` (both — one is "it fired",
one is the effect).
*Measurement that decides:* `labforage scenario=played_bed`, 3+ seeds,
120,000 frames, `RAYON_NUM_THREADS=1`, ablated by `seed_gut_survival: 0.0`.
Ships on established plants up, stand not running away, colony intake
unharmed.
*Card:* a frame sequence of one fruit being bitten and the pip sprouting, with
`plants_from_pip` in `meta`.
*Cost fork:* if `plants_from_pip` is zero with `seeds_spilled` healthy, that
is a finding about germination conditions — write it and stop.

### Brief A2 — the seed rides home

*Owns:* `src/sim/organism.rs` (`Crop`, one field). *Two hooks handed on:*
`creature.rs:5001` (fill the passenger) and `:5137` (put it down as a live
pip instead of a bare cell).
*Build:* `Crop.passenger: Option<SeedPassenger>` — one seed per crop, taken
from the first `CellType::Seed` bitten, popped by the first cell dropped, so
an ant carrying three fruit delivers one live seed. That asymmetry is the
graded outcome, not a limitation.
*Constants:* as A1, plus **check** `herb.seed_half_life` **14,000** against
the measured median frames from pickup to deposit — print it, do not assume
it.
*Counters:* `seeds_delivered`; and the headline number, **plants established
within 32 cells of a nest**.
*Measurement:* as A1 plus that column, paired against `colonies=0`.
*Card:* a `filmstrip gif=1` of the carry and the sprouting, with the delivery
count in `meta`.
*Cost fork:* if the creature lane has not landed, stop at A1 and say so.

### Brief B1 — nectar

*Owns:* `src/sim/plant.rs`, `assets/materials/flower.ron`,
`assets/species/*.ron`. *One hook handed on:* `creature.rs:4954`.
*Build:* `plant::nectar_offer(world, x, y) -> f32` — pays `nectar_yield`
(default 120) out of `OrganismState::reproductive_budget`, gated on the
flower's own `ripeness` clock so a drained flower pays less until it comes
round; the flower is **not** removed. Records the visit for B2.
*Constants to re-derive:* `EAT_YIELD_THRESHOLD` **12.0** — nectar is the first
sub-100 J meal and the threshold's own comment already asks for this;
`best_bite`'s doc example; `creature_probe`'s printed ceiling.
`flower.food_energy` **1,440 stays** — biting the flower off still ends it.
*Counters:* `flower_visits`, `nectar_paid`, and `organs_built` (the effect —
it must not fall).
*Measurement:* `labforage scenario=played_bed`, 3+ seeds, 120,000 frames,
ablated by `nectar_yield: 0.0`.
*Card:* a gif of ants working a flower head that is still standing afterwards,
with `flower_visits` in `meta`.
*Cost fork:* if standing flowers collapse, the yield is over-set — halve it
once, then write the finding and stop.

### Brief B2 — pollination as a discount

*Owns:* `src/sim/plant.rs`, `assets/species/*.ron`, one `u8` on the organ
cell. No creature-side change at all — B1's hook already records the visit.
*Build:* saturating visit count on the organ cell; effective `Ripen` cost
`× (1 − pollination_discount × visits/VISIT_SAT)`.
*Read first:* §3.3, and `CLAUDE.md`'s shared-budget rule. **Name
`seed_cost` 0.18, `reproductive_allocation` 0.28 and `seed_maturity` 60 in
your first commit message as the constants this reallocates, and budget
re-deriving them.** A correct mechanism at inherited constants is a
regression.
*Counters:* `organ_ripening_blocked` and `fruit_dropped`, **each split
visited/unvisited**. A fall in blocked for both arms is the discount leaking.
*Measurement:* as B1. **Do not measure B1 and B2 in one run** — they pull on
the same budget in opposite directions and a single run cannot separate them.
*Control:* a compartment with flowers and no colony, in the same run, is what
tells a pollination effect from a proximity artefact.
*Card:* the bed at 120,000 frames with fruit clustered on the colony's side,
`fruit dropped near / far` in `meta`.
*Cost fork:* if total fruit falls, report the two-knob sweep and stop.

### Brief C — the brush (only on the owner's ruling, card `…6dfed9`)

*Owns:* `src/lab/ui.rs`, `src/sim/organism.rs` (`cross`), `src/sim/plant.rs`.
*Read first:* §4.3's last paragraph. **Petal colour is not heritable**, so a
hybrid will not look like one; decide how the cross is made legible *before*
building it, or this is the sixth invisible lever.
*Build:* a bar tool that loads a pollen payload from one flower and applies a
uniform crossover at the other's fruit-set.
*Counter:* crosses made, and hybrids established.
*Card:* the two parents and the child, side by side, at play zoom.
*Cost fork:* if legibility cannot be solved inside the lane, write that
finding — it is more valuable than the tool.

### Brief D — palatability from foliage tone

*Owns:* `src/sim/creature.rs` (`diet_yield`), `assets/materials/leaf.ron`.
Sequence after B. *Read first:* §5, particularly `LOCUS_ALLELES[0] = 2` — the
axis is **binary** until that widens, and the ethos says an outcome is a
distribution.
*Measurement:* leaf-tone allele frequency with grazers against `colonies=0`,
3+ seeds, 120,000 frames, **with a deliberately absurd coupling run first as
the positive control**.
*Card:* the stand's colour with and without grazers.
*Cost fork:* if the allele will not move at absurd strength, the coupling is
not reaching the mouth — say so and stop.

---

## 8. What this contradicts, and what is owed

### 8.1 A defect found while measuring: `labshot` cannot seed a scenario

**`labshot scenario=<name> seed=N` silently ignores `seed=`.**
`examples/labshot.rs:134` reads `let spec = match &scenario { Some(s) =>
s.bed.clone(), … }`, so in scenario mode every command-line bed knob — the
world seed included — is discarded, and the echoed parameter line prints the
scenario's own `seed=1` truthfully, so it does not read as ignored.
**Confirmed by running seeds 1, 2 and 3 on `played_bed` and getting
digit-identical output**, which is `CLAUDE.md`'s own tell.

`examples/labforage.rs:300–312` **fixed exactly this defect on itself** and
its comment records the same tell (*"caught by three seeds returning a
byte-identical sample row … one command away from turning an eighteen-run
sweep into three runs reported six times"*). `labshot` never got the fix. The
repair is the same three lines, applied to the `Scenario` before `build`, not
to the local `spec`.

**Consequences.** Every contact sheet ever taken of `played_bed` is seed 1,
including §1's. And **every "3+ seeds on the played bed" measurement in this
document must be run through `labforage`, not `labshot`, until this is
fixed** — which is why it is a prerequisite in the build order rather than a
footnote. `Reports/instruments.md`'s `labshot` row says `seed=` "landed
2026-09-08 and it was missing rather than defaulted"; that is true of the
flag-built bed and false of scenario mode, and the row should say so.

### 8.2 Two written claims this document's measurements move

- **`flower.ron`: nectar is *"the richest thing a plant offers a walking
  animal … the whole reason a real animal crosses a meadow rather than eating
  the nearest hedge"*.** True of the table and false of the world: 40,000
  frames, best mouthful swallowed **960**. The comment should carry the
  measurement, because the next session to reason from it will over-value the
  flower exactly as `creature_probe` did.
- **`what-is-missing` §4 frames pollination as *"a flower that only sets seed
  if an animal has touched it"* and prices the danger as a reproduction
  requirement.** The requirement is the wrong danger; the measured danger is
  that a *clock* bonus of any size is inert, because the pipeline is blocked
  on budget 18,867 times against 56 drops. The mechanism has to act on the
  price.

### 8.3 What is OWED

Every one of these is a run nobody has made. Marked so that no later session
quotes them as measured.

1. **All 120,000-frame played-bed figures**, 3+ seeds, for A, B and D — none
   exists, and §8.1 says why they cannot come from `labshot` yet.
2. **The three windfall exits** (A0). Until then, "a fallen fruit is gone in
   68 frames" is a residence time with no cause attached.
3. **Median frames from pickup to deposit**, which is what decides whether
   `seed_half_life` is a live knob for A2 (§2.5).
4. **`stamp_probe` at a `pip`-bearing bed** — the arithmetic in §2.3 says the
   birth ceiling does not move because the ant is credited the full 960, and
   it is arithmetic, not a measurement.
5. **The concurrent measurement lane's census**
   (`Reports/lanes/evolution-lab-ecology-measure.md` on
   `origin/claude/lab-ecology-measure-r26`) **did not exist when this was
   written** — checked by `git fetch origin` and a branch listing on
   2026-09-10. Every number here is this lane's own. Where that lane's census
   disagrees, it wins: it is the one measuring the loop as it stands.

---

## 9. `dead-ends.md`, grepped before proposing anything

Per `CLAUDE.md`, the mechanism rather than the area. Verbatim result:

| grep | hits | what it says |
|---|---|---|
| `pollinat` | **0** | no pollination mechanism has ever been tried |
| `nectar` | **0** | nor a renewable flower meal |
| `midden`, `endozoo`, `frugivor` | **0** | nor gut dispersal in any form |
| `disperse\|dispersal` | 3, none about seeds | advection back-tracing and gust dispersal; unrelated |
| `windfall` | **1**, line 1636 | *fruit can be **reached** is not the same as being sown* — every flower and fruit stands 22–40 rows up and standing windfall never exceeds 1 cell in 90,000 frames. **Reproduced here** at a different bed and length: mean standing windfall **0.61**, peak 7 |
| `seed_launch` | **2**, lines 601 and 1708 | *a dispersal lever priced per seed is a good trade only for a species that is not already seed-limited* (`tree` fails, **`herb` measured +38% distant plants**); and the `param_scale` all-zero-corpus trap |

**Both `seed_launch` entries bear directly on this design and neither blocks
it.** Line 601's trap is a *price per seed* — A charges the plant nothing at
all (the endowment was already paid by `Ripen`), so there is no output to
halve; and its own contrast names **herb**, the species this design runs on,
as the one with seed to spare. Line 1708's trap is a parameter with an
all-zero corpus, which bounds evolution to ±4 units; `seed_gut_survival` is a
probability whose useful range is inside that bound, so it is evolvable from
the day it lands (§2.4). Line 911 — *ripening is charged to the reproductive
budget because an organ is a donor in `allocate_to_frontier` and can never pay
from its own carbon* — is why B's nectar payment must come from
`reproductive_budget` and not from the flower cell, and why B2 acts on the
cost rather than on the cell.

---

## 10. The owner rulings this depends on

Named so that a later session can see what would fall if one were reversed.

- **"Give me the tools, data, access to the parameters that need to be
  tweaked … that is the game"** (round three, standing). Every mechanism here
  lands as a species field the player can turn, not as a tuned constant.
- **Ship new behaviours as default** (round twenty). None of A, B or D is
  proposed behind an off switch; only C-animal is, and only because §4.3 says
  a default-off gene flow is itself a cost.
- **Trophallaxis is a brain output the genome can evolve, never a rule**
  (2026-09-09). The same principle is why B1's hook offers nectar and lets the
  existing feed arbitration decide, rather than authoring a "visit flowers"
  behaviour.
- **Movement, not stills, is how animals are seen** (2026-09-09). Every card
  in §7 is a sequence or a gif.
- **An omnivore should be viable** (card `20260823T104411499Z-963f8d`, and the
  guard `a_starved_nestmates_corpse_is_still_dinner` enforces it). §2.3's
  40 J pip is chosen so that the shipped neutral gut cannot see it — the
  mechanism must not push the ant toward the plant specialism the owner
  vetoed.
- **A fruit carries the seed** (2026-08-23), which is why a windfall is a
  `CellType::Seed` at all — and therefore why A is one field rather than a
  dispersal subsystem.
- **Fruit is charged to the reproductive account, construction to the acting
  cell** (2026-08-27/29). §3 sits entirely on the first half of that split.

**Two rulings this document asks for and does not assume:** which form of seed
dispersal (card `20260910T032808971Z-cba50c`), and whether animals may carry
pollen (card `20260910T032635178Z-6dfed9`).
